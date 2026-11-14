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

// ---- the scan's first look at a dropped PDF ----

fn set_mtime(path: &str, secs: i64) {
    let c = std::ffi::CString::new(path).unwrap();
    let tv = [libc::timeval { tv_sec: secs as libc::time_t, tv_usec: 0 }, libc::timeval { tv_sec: secs as libc::time_t, tv_usec: 0 }];
    // SAFETY: utimes reads the path and the two timevals we own.
    assert_eq!(unsafe { libc::utimes(c.as_ptr(), tv.as_ptr()) }, 0);
}

fn marks(e: &Env) -> Vec<String> {
    std::fs::read_dir(format!("{}/import/.marks", e.root)).map(|r| r.flatten().filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect()).unwrap_or_default()
}

// The scan looks at the first and last KiB of a steady PDF before it queues it. The look opens the file the way
// the import itself does (no link followed) and reads only the file that was listed. A link put at the name after
// the listing is not a file: nothing is read through it, nothing is queued, and no sooner look is asked for.
#[test]
fn a_link_put_in_after_the_listing_is_not_read() {
    let e = setup("look-link");
    e.put("grimoire/import/Book.pdf", &pdffix::planner(2));
    // (a whole PDF of the listed file's length: a look through the link would find nothing amiss)
    e.put("documents/Other.pdf", &pdffix::planner(2));
    let found = list_pdfs();
    assert_eq!(found.files.len(), 1);
    let at = format!("{}/import/Book.pdf", e.root);
    std::fs::remove_file(&at).unwrap();
    std::os::unix::fs::symlink(format!("{}/documents/Other.pdf", e.storage), &at).unwrap();
    assert_eq!(lock_take(), Lock::Fresh);
    let q = scan_queue(&found, DPI, false);
    let (left, pending) = (marks(&e), pending_count());
    let sooner = scan_due(now_ms().wrapping_add(60_000), false);
    let said = drain();
    lock_release_if_idle();
    assert_eq!(q, 0, "the scan queued what the link names: {said:?}");
    assert!(left.is_empty() && pending == 0, "{left:?}, {pending} pending");
    assert!(!sooner, "what is no file asks for no sooner look");
    assert!(e.notebooks().is_empty());
    assert!(e.there("documents/Other.pdf"));
}

/// import/Book.pdf listed, then another PDF of its length and time renamed over it: the listing the scan is given
/// names a file that is no longer the one at that name.
fn listed_then_swapped(e: &Env) -> Found {
    let a = pdffix::planner(2);
    // (the same PDF with another digit on its first page: one length, other bytes)
    let mut b = a.clone();
    let digit = b.windows(8).position(|w| w == b"(Page 1)").expect("page 1's text") + 6;
    b[digit] = b'7';
    assert!(a.len() == b.len() && a != b);
    let (at, tmp) = (format!("{}/import/Book.pdf", e.root), format!("{}/import/B.tmp", e.root));
    std::fs::write(&at, &a).unwrap();
    // written before the listing (under a name the scan does not list), so it is a file of its own number
    std::fs::write(&tmp, &b).unwrap();
    set_mtime(&at, 1_700_000_000);
    set_mtime(&tmp, 1_700_000_000);
    let found = list_pdfs();
    assert_eq!(found.files.len(), 1);
    std::fs::rename(&tmp, &at).unwrap();
    found
}

// Another file put at the name after the listing (same length, same time) is not the file that was listed: this
// scan leaves it and asks for a look 5 s on, and the scan that lists the folder again takes it.
#[test]
fn another_file_put_in_after_the_listing_waits_for_the_next_look() {
    let e = setup("look-swap");
    let found = listed_then_swapped(&e);
    assert_eq!(lock_take(), Lock::Fresh);
    let q1 = scan_queue(&found, DPI, false);
    let again = scan_due(now_ms().wrapping_add(WAIT_RESCAN_MS + 1), false);
    let said1 = drain();
    // the next look lists the folder again
    let _ = lock_take();
    let q2 = scan_queue(&list_pdfs(), DPI, false);
    let said2 = drain();
    lock_release_if_idle();
    assert_eq!(q1, 0, "the scan took a file that is not the one it listed: {said1:?}");
    assert!(again, "a look is due 5 s on");
    assert_eq!(q2, 1, "the next listing's file is taken: {said2:?}");
    assert_eq!(e.notebooks(), ["Book"]);
}

// A file that keeps differing from its listing (the same listing given again and again here; on a device, a
// storage that named one file differently by its name and by an open file) is not left for ever: unchanged for
// a minute, it is imported as it is, as a PDF without its end marker is. The clock is the test's own.
#[test]
fn a_file_that_differs_from_its_listing_is_taken_after_a_minute_unchanged() {
    let e = setup("look-minute");
    let found = listed_then_swapped(&e);
    assert_eq!(lock_take(), Lock::Fresh);
    let t0 = 5_000_000u32;
    let mut queued = Vec::new();
    for dt in [0u32, 30_000, 61_000] {
        grimoire::util::sim_clock_set(Some(t0 + dt));
        queued.push(scan_queue(&found, DPI, false));
    }
    grimoire::util::sim_clock_set(None);
    let said = drain();
    lock_release_if_idle();
    assert_eq!(queued, vec![0, 0, 1], "{said:?}");
    assert_eq!(e.notebooks(), ["Book"]);
}

// A FIFO put at the name after the listing is not opened to be read: not a byte is taken from it. (Neither run can
// block: the FIFO has a writer, this test, and holds more than a look asks for.)
#[test]
fn a_fifo_put_in_after_the_listing_is_not_read() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let e = setup("look-fifo");
    let pdf = pdffix::planner(4);
    assert!(pdf.len() >= 2048, "{}", pdf.len());
    e.put("grimoire/import/Book.pdf", &pdf);
    let found = list_pdfs();
    assert_eq!(found.files.len(), 1);
    let at = format!("{}/import/Book.pdf", e.root);
    std::fs::remove_file(&at).unwrap();
    let c = std::ffi::CString::new(at.as_str()).unwrap();
    // SAFETY: mkfifo and open read the NUL-terminated path we own; the descriptor is this test's alone.
    let fifo = unsafe {
        assert_eq!(libc::mkfifo(c.as_ptr(), 0o600), 0);
        let fd = libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK);
        assert!(fd >= 0);
        OwnedFd::from_raw_fd(fd)
    };
    let filled = vec![b'x'; 2048];
    // SAFETY: write reads the buffer we own; read below writes into one we own.
    assert_eq!(unsafe { libc::write(fifo.as_raw_fd(), filled.as_ptr() as *const libc::c_void, filled.len()) }, 2048);
    assert_eq!(lock_take(), Lock::Fresh);
    let q = scan_queue(&found, DPI, false);
    let sooner = scan_due(now_ms().wrapping_add(60_000), false);
    let said = drain();
    lock_release_if_idle();
    let mut buf = vec![0u8; 4096];
    let left = unsafe { libc::read(fifo.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    assert_eq!(q, 0, "{said:?}");
    assert_eq!(left, 2048, "the scan read from the FIFO");
    assert!(!sooner, "what is no file asks for no sooner look");
    assert!(marks(&e).is_empty() && e.notebooks().is_empty());
}
