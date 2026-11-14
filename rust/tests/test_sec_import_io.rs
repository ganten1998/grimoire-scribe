// test_sec_import_io.rs: what the import asks of the storage before it acts on a file (the hardening of 2026-11):
// whether a folder spelled like grimoire/import is that folder, the scan's first look at a dropped PDF, and the
// import lock. The jobs run as in the app (bg_poll_job, then pdfimport::finished), every page through a real
// worker, and every queued job is drained: no test waits on a clock. Generated PDFs only (tests/pdffix).
//
// It runs in CI's unit jobs and in its Linux `sandbox` job (as root, GRIMOIRE_SANDBOX=require, which the setup
// here leaves as it finds it), where the storage is the temporary folder's tmpfs and tells names apart by case.
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::fsview::StoragePath;
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
const DPI: f32 = 300.0;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

// (GRIMOIRE_SANDBOX is not among them: CI's Linux job requires the sandbox, and these tests keep what it set)
const HOOKS: [&str; 14] = [
    "GRIMOIRE_PDF_SLICE_PAGES",
    "GRIMOIRE_PDF_BANDS",
    "GRIMOIRE_PDF_NO_MAP",
    "GRIMOIRE_PDF_FREE_BYTES",
    "GRIMOIRE_PDF_WATCHDOG_MS",
    "GRIMOIRE_PDF_TEST_CRASH_PAGE",
    "GRIMOIRE_PDF_TEST_HANG_PAGE",
    "GRIMOIRE_PDF_TEST_ALLOC_PAGE",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_IMPORT_STABLE",
    "GRIMOIRE_WORK_LEN",
    "GRIMOIRE_BG_DELAY_MS",
    "GRIMOIRE_PIC_TEST_CRASH",
    // (the lock files' folder is the process's: only the lock's own test names one)
    "GRIMOIRE_LOCKS",
];

struct Env {
    /// Grimoire's own folder (the storage root is its parent)
    root: String,
    storage: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn storage_of(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join(format!("grimoire-sec-import-io-{}-{name}", std::process::id()))
}

fn setup(name: &str) -> Env {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in HOOKS {
        std::env::remove_var(h);
    }
    let storage = storage_of(name);
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::fs::create_dir_all(storage.join("documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    // a file is steady at its first look: the tests are about what the scan does with a steady file
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(DPI, PW, PH);
    ui_reset();
    Env { root: root.to_str().unwrap().to_string(), storage: storage.to_str().unwrap().to_string(), _g: g }
}

impl Env {
    fn put(&self, rel: &str, bytes: &[u8]) {
        let p = format!("{}/{rel}", self.storage);
        std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    fn there(&self, rel: &str) -> bool {
        std::fs::symlink_metadata(format!("{}/{rel}", self.storage)).is_ok()
    }
    fn notebooks(&self) -> Vec<String> {
        let mut v = Vec::new();
        store_list(&mut v);
        let mut n: Vec<String> = v.iter().map(|n| n.name.as_str().to_string()).collect();
        n.sort();
        n
    }
    fn names(&self, rel: &str) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(format!("{}/{rel}", self.storage)).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        v.sort();
        v
    }
    /// What Grimoire's own import/done holds.
    fn done(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(format!("{}/import/done", self.root)).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        v.sort();
        v
    }
}

/// Polls the background jobs as app.rs's poll_jobs does, until none is left: what each import said.
fn drain() -> Vec<String> {
    let mut said = Vec::new();
    let t0 = std::time::Instant::now();
    loop {
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::PdfImport as i32 {
            said.push(format!("{:?}", finished(id.as_str(), r > 0)));
        } else if kind == BgKind::ImportFolder as i32 {
            let mut pages = 0;
            let got = import_finished(r > 0, &mut pages);
            lock_release_if_idle();
            said.push(format!("folder {got} ({pages} pages)"));
        }
        if !bg_busy() {
            break;
        }
        assert!(t0.elapsed().as_secs() < 120, "the jobs never ended");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    said
}

/// A test's checks, each tried in turn: one run prints every check that does not hold, and the test fails at its
/// end (so a run on the tree before a change shows all that the change makes true).
#[derive(Default)]
struct Checks(Vec<String>);

impl Checks {
    fn that(&mut self, ok: bool, what: impl std::fmt::Display) {
        if !ok {
            eprintln!("  NOT SO: {what}");
            self.0.push(what.to_string());
        }
    }
    fn done(self) {
        assert!(self.0.is_empty(), "{} check(s) did not hold: {:#?}", self.0.len(), self.0);
    }
}

fn picture(v: u8) -> Vec<u8> {
    encode_png(&BgPicture { px: vec![v; 150 * 200], w: 150, h: 200, ch: 1 }).unwrap()
}

// ---- a folder spelled like grimoire/import ----

// Only a file that lies in grimoire/import moves to import/done. A path through a folder spelled `Grimoire/Import`
// lies there where the storage folds case (the Scribe's does: one folder, two spellings) and does not where the
// storage tells the names apart (two folders). The test reads the host's rule and holds the import to it.
#[test]
fn a_folder_spelled_like_import_is_import_only_when_it_is_that_folder() {
    let e = setup("spelling");
    let mut c = Checks::default();
    // made after grimoire/import: the same folder where case folds, a folder of its own where it does not
    e.put("Grimoire/Import/Book.pdf", &pdffix::planner(2));
    let id = |rel: &str| std::fs::metadata(format!("{}/{rel}", e.storage)).map(|m| (m.dev(), m.ino())).unwrap();
    let folds = id("grimoire/import") == id("Grimoire/Import");
    eprintln!("  this storage {}", if folds { "folds case: one folder" } else { "tells names apart by case: two folders" });
    c.that(in_import("Grimoire/Import/Book.pdf") == folds, format!("in_import of the other spelling is the storage's rule ({folds})"));
    c.that(in_import("grimoire/import/Book.pdf"), "the folder's own spelling is in import");

    // the PDF, picked through that spelling
    let sp = StoragePath::new(b"Grimoire/Import/Book.pdf").unwrap();
    let q = pdf_import_queue(&sp, None, None, DPI);
    drain();
    c.that(matches!(q, Queued::Started(_)), format!("the pick starts: {q:?}"));
    c.that(e.notebooks() == ["Book"], format!("one notebook: {:?}", e.notebooks()));
    let done = e.done();
    if folds {
        c.that(!e.there("Grimoire/Import/Book.pdf") && done.len() == 1, format!("a file of import/ is filed in done/: {done:?}"));
    } else {
        c.that(e.there("Grimoire/Import/Book.pdf"), "the file is still where it was picked");
        c.that(done.is_empty(), format!("nothing moved to grimoire/import/done: {done:?}"));
    }
    // a scan afterwards imports nothing a second time
    grimoire::bgwork_template::import_scan(DPI);
    drain();
    c.that(e.notebooks() == ["Book"], format!("still one notebook after a scan: {:?}", e.notebooks()));

    // a folder of pictures, picked through that spelling
    for (k, v) in [(1, 40u8), (2, 90u8)] {
        e.put(&format!("Grimoire/Import/Scans/page-{k}.png"), &picture(v));
    }
    let sp = StoragePath::new(b"Grimoire/Import/Scans").unwrap();
    let q = grimoire::bgwork_template::import_pick_queue(&sp, None, None, DPI);
    drain();
    grimoire::bgwork_template::import_scan(DPI);
    drain();
    c.that(matches!(q, Queued::Started(_)), format!("the folder's pick starts: {q:?}"));
    c.that(e.notebooks() == ["Book", "Scans"], format!("one notebook more: {:?}", e.notebooks()));
    let done = e.done();
    if folds {
        c.that(!e.there("Grimoire/Import/Scans") && done.len() == 2, format!("the folder is filed in done/: {done:?}"));
    } else {
        c.that(e.names("Grimoire/Import/Scans") == ["page-1.png", "page-2.png"], "the folder is still where it was picked, with its pictures");
        c.that(done.is_empty(), format!("nothing moved to grimoire/import/done: {done:?}"));
    }
    c.done();
}
