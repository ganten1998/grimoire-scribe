// test_picture_jobs.rs: pictures from outside Grimoire in the background jobs (2.1.0-beta.2 fix round: M12, M13,
// and the seam of lanes L7 and L8). Every picture goes through a real worker, a fresh exec of the headless
// runner (GRIMOIRE_WORK_EXE), as in the app.
// - a picture picked in place has its header read by the worker, never by the job (a pin);
// - one worker death is no verdict: a fresh worker tries the picture again, and a folder whose workers died
//   counts one try of its three, never refused for good (M12);
// - the UI thread never starts a worker: a template or photo whose preview is not made yet cannot be picked
//   until the previews job has made it (M13).
#![cfg(feature = "sim")]
use grimoire::fsview::StoragePath;
use grimoire::picwork::{in_fallback, PicOut, PicPool};
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
static LOCK: Mutex<()> = Mutex::new(());

struct Env {
    root: String,
    storage: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn setup(name: &str) -> Env {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in ["GRIMOIRE_PIC_TEST_CRASH", "GRIMOIRE_PIC_TEST_HANG", "GRIMOIRE_PIC_TEST_TRAP", "GRIMOIRE_PIC_WATCHDOG_MS", "GRIMOIRE_WORK_FAIL_STEP", "GRIMOIRE_SANDBOX", "GRIMOIRE_BG_DELAY_MS"] {
        std::env::remove_var(h);
    }
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let storage = base.join(format!("grimoire-picture-jobs-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::fs::create_dir_all(storage.join("documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, PW, PH);
    ui_reset();
    Env { root: root.to_str().unwrap().to_string(), storage: storage.to_str().unwrap().to_string(), _g: g }
}

fn pictures(dir: &str, n: usize) {
    std::fs::create_dir_all(dir).unwrap();
    for k in 1..=n {
        let p = BgPicture { px: vec![(40 * k) as u8; 150 * 200], w: 150, h: 200, ch: 1 };
        assert_eq!(bgimage_write_png(&format!("{dir}/page-{k}.png"), &p), 0);
    }
}

/// Runs the queued background jobs to their end, as app.rs's poll_jobs does.
fn drain() -> Vec<String> {
    let mut said = Vec::new();
    let t0 = std::time::Instant::now();
    loop {
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::ImportFolder as i32 {
            let mut pages = 0;
            let got = import_finished(r > 0, &mut pages);
            lock_release_if_idle();
            said.push(format!("folder import: {got} ({pages} pages)"));
        }
        if !bg_busy() || t0.elapsed().as_secs() > 120 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    said
}

/// Runs `f` with stderr (the log, also the forked jobs') going to a file; the log's text.
fn with_log<R>(path: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    let r = f();
    // SAFETY: dup2 and close on the descriptor saved above.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    let text = std::fs::read_to_string(path).unwrap_or_default();
    eprint!("{text}");
    (r, text)
}

// ---- 1. The header of a picture picked in place is read by the worker, never by the job ----
//
// bgwork_template.rs:207-217 (check_file_in): lane L8's in-place pictures are read through the walk, whole, and
// their header check goes to the picture pool as every other picture's does. Before the merge of L7 and L8 this
// branch called bgimage_check (stb_image's header parser) in the job, as root. What tells the two apart: with
// every picture worker dying on its FIRST request (GRIMOIRE_PIC_TEST_CRASH=1), a header check done by the worker
// ends the import at check_pages, and the log says the worker ended "on its header"; a header parsed in the job
// would pass, and the worker would die only later, on the decode ("a picture's worker ended (...)" without "on
// its header", from build_notebook). No notebook either way.
#[test]
fn a_picked_pictures_header_is_checked_by_the_worker() {
    let e = setup("pick-header");
    pictures(&format!("{}/documents/Scans", e.storage), 1);
    std::env::set_var("GRIMOIRE_PIC_TEST_CRASH", "1");
    let sp = StoragePath::new(b"documents/Scans").unwrap();
    let log = format!("{}/captured.log", e.storage);
    let (said, text) = with_log(&log, || {
        let q = grimoire::bgwork_template::import_pick_queue(&sp, None, None, 300.0);
        assert!(matches!(q, Queued::Started(_)), "{q:?}");
        drain()
    });
    std::env::remove_var("GRIMOIRE_PIC_TEST_CRASH");
    let mut all = Vec::new();
    store_list(&mut all);
    assert!(all.is_empty(), "{said:?}");
    assert!(text.contains("a picture's worker ended on its header"), "the header check of a picked picture did not go to the worker: {text}");
    assert!(text.contains("page 1 of 1: the worker ended; nothing imported"), "{text}");
    assert!(!text.contains("pictures decode in the writer"), "no fallback: the sandbox works: {text}");
    // and with a worker that lives, the same folder imports (the pick is read in place)
    let q = grimoire::bgwork_template::import_pick_queue(&sp, None, None, 300.0);
    assert!(matches!(q, Queued::Started(_)), "{q:?}");
    let said = drain();
    store_list(&mut all);
    assert_eq!(all.len(), 1, "{said:?}");
    assert!(std::path::Path::new(&format!("{}/documents/Scans/page-1.png", e.storage)).exists());
}

// ---- 2. One worker death is no verdict: a folder of good pictures is never refused for it (M12) ----
//
// A worker's death is not always its picture's fault: the worker is the first process the kernel takes when
// memory runs out anywhere (oom_score_adj 1000), one RLIMIT_CPU is shared by the 32 pictures a worker serves and
// the signal is blamed on whichever picture is in hand, and the watchdog is one number until the Scribe is
// measured. A death is tried again on a fresh worker (picwork.rs), and a folder whose workers died twice in a
// row on a picture counts one try of its three (IMPORT_RETRY), as 2.0 counted what might pass another time; only
// a picture at fault refuses the folder for good. (0c26d912 refused the folder at the first death: 99 tries.)
// Here every worker of the first run dies on its first request; nothing else is wrong.
#[test]
fn worker_deaths_never_refuse_a_good_folder() {
    let e = setup("refused");
    let src = format!("{}/import/Scans", e.root);
    pictures(&src, 2);
    // every worker of this run dies on its first request (as when the kernel takes them); nothing else is wrong
    std::env::set_var("GRIMOIRE_PIC_TEST_CRASH", "1");
    let rc = bg_import_folder(&src, 300.0);
    std::env::remove_var("GRIMOIRE_PIC_TEST_CRASH");
    assert_ne!(rc, 0, "the import failed whole");
    let refused = grimoire::bgwork_template::folder_failed(&src);
    // what the next scan does with the folder, its pictures as good as they were
    let queued = import_scan(300.0);
    let said = drain();
    let mut all = Vec::new();
    store_list(&mut all);
    let attempts = std::fs::read_to_string(format!("{}/import/.attempts", e.root)).unwrap_or_default();
    eprintln!("  after the worker deaths: refused {refused}; the next scan queued {queued} ({said:?}); notebooks {}; .attempts: {attempts:?}", all.len());
    assert!(
        !refused && queued == 1 && all.len() == 1,
        "worker deaths, and the folder was refused until its pictures change: the scan queued {queued} import, {} notebook(s) were made; .attempts says {attempts:?}",
        all.len()
    );
}

// A picture whose worker dies once is decoded by the fresh worker that tries it again; one that two workers in a
// row die on is Dead (a template or photo is then marked bad: test_picwork.rs).
#[test]
fn one_death_is_tried_again_on_a_fresh_worker() {
    let e = setup("retry");
    let dir = format!("{}/pics", e.storage);
    pictures(&dir, 1);
    let good = std::fs::read(format!("{dir}/page-1.png")).unwrap();
    let mut pool = PicPool::new();
    assert!(matches!(pool.decode(&good, false, 300, 400), PicOut::Pic { .. }), "worker 1 serves one");
    // a fresh pool's workers die on their second request: worker 1's second picture dies, worker 2 decodes it
    std::env::set_var("GRIMOIRE_PIC_TEST_CRASH", "2");
    let mut pool = PicPool::new();
    assert!(matches!(pool.decode(&good, false, 300, 400), PicOut::Pic { .. }));
    assert!(matches!(pool.decode(&good, false, 300, 400), PicOut::Pic { .. }), "a death, then the fresh worker's answer");
    let mut w = 0;
    let mut h = 0;
    let mut why: &'static str = "";
    assert_eq!(bgimage_check(&good, good.len(), &mut w, &mut h, &mut why), 0);
    assert!(matches!(pool.check(&good, good.len()), PicOut::Info { .. }), "the header too");
    // every worker dies on its first request: two deaths in a row, Dead
    std::env::set_var("GRIMOIRE_PIC_TEST_CRASH", "1");
    let mut pool = PicPool::new();
    assert!(matches!(pool.decode(&good, false, 300, 400), PicOut::Dead(_)));
    assert!(matches!(pool.check(&good, good.len()), PicOut::Dead(_)));
    std::env::remove_var("GRIMOIRE_PIC_TEST_CRASH");
    assert!(matches!(pool.decode(&good, false, 300, 400), PicOut::Pic { .. }), "and the next picture has a fresh worker");
    assert!(!in_fallback());
}

// ---- 3. The UI thread never starts a worker: a template or photo with no preview yet waits for it (M13) ----
//
// DESIGN.md 3.7's process model is UI -> bgjob child (writer) -> worker, and 4.4 keeps "all long work in the
// bgjob child and its worker" (the launcher ends Grimoire after 15 s without a heartbeat). The Templates sheet
// offers every tile that is not bad, ready or not, and a tap goes to apply_template -> template_install ->
// install_picture on the UI thread. 0c26d912 checked a file of at most 4 MB with no preview yet right there:
// the UI process forked and exec'd a worker, waited for its handshake and its answer (the picture watchdog: 60 s
// on the Kindle), set SIGPIPE to ignored for itself and every later child, and set the process-wide fallback
// flag when no worker could be had. Now such a file is "its preview is not made yet", as a file over 4 MB was:
// the previews job's worker says whether it is a picture, and then it can be picked. Here the caller is this
// test process, as the UI is in the app; a worker would hang, its watchdog 3 s.
#[test]
fn picking_a_template_without_a_preview_never_starts_a_worker_in_the_caller() {
    let e = setup("ui-writer");
    std::fs::create_dir_all(format!("{}/templates", e.root)).unwrap();
    let path = format!("{}/templates/New.png", e.root);
    let p = BgPicture { px: vec![200u8; 150 * 200], w: 150, h: 200, ch: 1 };
    assert_eq!(bgimage_write_png(&path, &p), 0);
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Planner", "", PAPER_BLANK, &mut nb), 0);
    // the app's own disposition (mainloop.rs:336), so a change is seen
    // SAFETY: setting and reading a signal's disposition.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    std::env::set_var("GRIMOIRE_PIC_TEST_HANG", "1");
    std::env::set_var("GRIMOIRE_PIC_WATCHDOG_MS", "3000");
    let mut k = PAPER_BLANK;
    let mut why: &'static str = "";
    let t0 = std::time::Instant::now();
    let rc = template_install(&nb, &path, &mut k, &mut why);
    let waited = t0.elapsed().as_millis();
    std::env::remove_var("GRIMOIRE_PIC_TEST_HANG");
    std::env::remove_var("GRIMOIRE_PIC_WATCHDOG_MS");
    // SAFETY: as above.
    let now_ignored = unsafe {
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGPIPE, std::ptr::null(), &mut old);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
        old.sa_sigaction == libc::SIG_IGN
    };
    eprintln!("  template_install: rc {rc} ({why}) after {waited} ms in the calling process; SIGPIPE now ignored there: {now_ignored}");
    assert!(rc != 0 && why.contains("preview"), "a template with no preview yet waits for it: {rc} {why}");
    assert!(
        waited < 500 && !now_ignored,
        "the caller of template_install (the UI thread in the app) waited {waited} ms for a picture worker of its own (the whole watchdog: 60 s on \
         the Kindle, where the launcher ends Grimoire after 15 s), and its SIGPIPE disposition changed: {now_ignored}"
    );
    // once the previews job has made its preview (its worker checked the file), it is picked at once
    let mut t: Vec<Template> = Vec::new();
    assert_eq!(template_scan(&mut t, 300.0), 0);
    template_list_free(&mut t);
    let _ = drain();
    let t0 = std::time::Instant::now();
    assert_eq!(template_install(&nb, &path, &mut k, &mut why), 0, "{why}");
    assert!(paper_is_image(k) && t0.elapsed().as_millis() < 500);
    let _ = drain();
}
