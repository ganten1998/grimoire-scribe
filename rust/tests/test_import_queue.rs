// test_import_queue.rs: the UI side of the PDF import (2.1.0-beta.2, DESIGN.md 3.8 and 3.9; the fix round of
// 2026-11-09): what a scan queues and keeps, the import lock, picks that wait, the queue that drops a job, the
// notice. The jobs run as in the app (bg_poll_job, then pdfimport::finished), every page through a real worker.
// Generated PDFs only (tests/pdffix). The writer's side is test_pdfimport.rs.
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::fsview::StoragePath;
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
const DPI: f32 = 300.0;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

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
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_WORK_LEN",
    "GRIMOIRE_BG_DELAY_MS",
    "GRIMOIRE_PIC_TEST_CRASH",
];

struct Env {
    /// Grimoire's own folder (the storage root is its parent)
    root: String,
    storage: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn storage_of(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join(format!("grimoire-import-queue-{}-{name}", std::process::id()))
}

/// This thread's store on the storage `name` (the store, the queue and the import's UI side are per thread).
fn attach(name: &str) -> String {
    let storage = storage_of(name);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::fs::create_dir_all(storage.join("documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(DPI, PW, PH);
    ui_reset();
    root.to_str().unwrap().to_string()
}

fn setup(name: &str) -> Env {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in HOOKS {
        std::env::remove_var(h);
    }
    let _ = std::fs::remove_dir_all(storage_of(name));
    let root = attach(name);
    let storage = storage_of(name).to_str().unwrap().to_string();
    Env { root, storage, _g: g }
}

impl Env {
    fn put(&self, rel: &str, bytes: &[u8]) {
        let p = format!("{}/{rel}", self.storage);
        std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
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
    fn attempts(&self) -> String {
        std::fs::read_to_string(format!("{}/import/.attempts", self.root)).unwrap_or_default()
    }
    fn marks(&self) -> Vec<String> {
        std::fs::read_dir(format!("{}/import/.marks", self.root)).map(|r| r.flatten().filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect()).unwrap_or_default()
    }
    /// Another process (a previous run's detached child) holds import/.lock until the file is dropped.
    fn hold_lock(&self) -> std::fs::File {
        let f = std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(format!("{}/import/.lock", self.root)).unwrap();
        // SAFETY: flock on a descriptor this test owns (another open file description than the module's).
        assert_eq!(unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
        f
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

fn new_id() -> String {
    let mut id = String::new();
    store_make_id(&mut id);
    id
}

fn run_to_end(id: &str, src: &str, most: usize) -> SliceEnd {
    let mut end = SliceEnd::More;
    for _ in 0..most {
        end = run_slice(id, src);
        if end != SliceEnd::More {
            break;
        }
    }
    end
}

// ---- H1: the record of tries ----

// A refused PDF keeps its record across scans: it is queued once, and its row says Failed until it changes
// (DESIGN.md 3.8, FORMAT.md import/.attempts).
#[test]
fn a_scan_keeps_the_record_of_a_pdf_that_was_refused() {
    let e = setup("refused-kept");
    e.put("grimoire/import/Locked.pdf", &pdffix::encrypted_planner(2, "secret", "owner"));
    let src = "grimoire/import/Locked.pdf";
    let md = std::fs::metadata(format!("{}/import/Locked.pdf", e.root)).unwrap();
    let end = run_to_end(&new_id(), src, 5);
    assert!(matches!(end, SliceEnd::Failed(2, 0)), "refused for its password: {end:?}");
    assert!(e.attempts().contains(" 99"), "the refusal is recorded: {:?}", e.attempts());
    for scan in 1..=3 {
        assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0, "scan {scan}");
        assert!(e.attempts().contains(" 99"), "scan {scan}: the record is kept: {:?}", e.attempts());
        assert_eq!(import_status(src, md.len(), md.mtime()), ImportStatus::Failed);
    }
}

#[test]
fn a_refused_pdf_is_queued_and_told_once() {
    let e = setup("refused-once");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    e.put("grimoire/import/Locked.pdf", &pdffix::encrypted_planner(2, "secret", "owner"));
    let mut queued_at = Vec::new();
    let mut told = Vec::new();
    for scan in 1..=6 {
        if grimoire::bgwork_template::import_scan(DPI) > 0 {
            queued_at.push(scan);
        }
        told.extend(drain());
    }
    assert_eq!(queued_at, vec![1], "{told:?}");
    assert_eq!(told, vec!["Failed(Password)".to_string()]);
    assert!(std::path::Path::new(&format!("{}/import/Locked.pdf", e.root)).exists());
}

// A PDF whose every page kills its worker fails whole at each try, and three tries are all it gets.
#[test]
fn a_pdf_of_blank_pages_only_is_tried_three_times() {
    let e = setup("all-blank");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "1");
    e.put("grimoire/import/Planner.pdf", &pdffix::planner(1));
    let mut queued_at = Vec::new();
    let mut told = 0;
    for scan in 1..=7 {
        if grimoire::bgwork_template::import_scan(DPI) > 0 {
            queued_at.push(scan);
        }
        told += drain().len();
    }
    assert_eq!(queued_at, vec![1, 2, 3]);
    assert_eq!(told, 3);
    assert!(e.notebooks().is_empty());
    assert!(std::path::Path::new(&format!("{}/import/Planner.pdf", e.root)).exists());
}

// No room is not the file's fault: nothing is recorded, and the file is taken again once there is room.
#[test]
fn no_room_counts_nothing_and_the_file_goes_once_there_is_room() {
    let e = setup("room");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_PDF_FREE_BYTES", "1000000");
    e.put("grimoire/import/Planner.pdf", &pdffix::planner(2));
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    assert_eq!(drain(), vec!["Failed(Storage)".to_string()]);
    assert_eq!(e.attempts(), "", "no try was counted against the file");
    // the same little room: the scan leaves it (no fork, no toast at every scan)
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    // room again
    std::env::set_var("GRIMOIRE_PDF_FREE_BYTES", "4000000000");
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    drain();
    assert_eq!(e.notebooks(), ["Planner"]);
}

// Two PDFs of one name in import/ and in import/Work/ have a record each.
#[test]
fn a_pdf_and_one_of_its_name_in_a_folder_have_a_record_each() {
    let e = setup("same-name");
    // (the same bytes and the same time: a record keyed by the name alone could not tell them apart)
    let bytes = pdffix::encrypted_planner(1, "secret", "owner");
    e.put("grimoire/import/A.pdf", &bytes);
    e.put("grimoire/import/Work/A.pdf", &bytes);
    for f in ["A.pdf", "Work/A.pdf"] {
        let c = std::ffi::CString::new(format!("{}/import/{f}", e.root)).unwrap();
        let tv = [libc::timeval { tv_sec: 1_700_000_000, tv_usec: 0 }, libc::timeval { tv_sec: 1_700_000_000, tv_usec: 0 }];
        // SAFETY: utimes reads the path and the two timevals we own.
        assert_eq!(unsafe { libc::utimes(c.as_ptr(), tv.as_ptr()) }, 0);
    }
    assert!(matches!(run_to_end(&new_id(), "grimoire/import/A.pdf", 5), SliceEnd::Failed(2, 0)));
    let md = std::fs::metadata(format!("{}/import/Work/A.pdf", e.root)).unwrap();
    assert_eq!(import_status("grimoire/import/Work/A.pdf", md.len(), md.mtime()), ImportStatus::Waiting, "the refusal is the top-level file's only");
}

// ---- H2: the lock and the picks that wait ----

// A pick made while another process holds the lock is queued by the first scan that takes it afresh.
#[test]
fn a_pick_that_waited_for_the_lock_is_taken_up_by_a_later_scan() {
    let e = setup("waiting-pick");
    e.put("documents/Book.pdf", &pdffix::planner(2));
    let other = e.hold_lock();
    let sp = StoragePath::new(b"documents/Book.pdf").unwrap();
    assert_eq!(pdf_import_queue(&sp, Some("Typed name"), None, DPI), Queued::Waiting);
    assert!(e.marks().iter().any(|m| m.contains("state=waiting")));
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0, "the lock is still held elsewhere");
    drop(other);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    drain();
    assert_eq!(e.notebooks(), ["Typed name"]);
    assert!(e.marks().is_empty(), "no mark is left: {:?}", e.marks());
    assert!(std::path::Path::new(&format!("{}/documents/Book.pdf", e.storage)).exists(), "never moved");
}

// A job that is no import never keeps the lock: once the last import has ended, a pick is queued at once,
// whatever else runs.
#[test]
fn the_lock_is_free_once_no_import_is_left_whatever_else_runs() {
    let e = setup("lock-leak");
    e.put("documents/Book.pdf", &pdffix::planner(1));
    assert_eq!(lock_take(), Lock::Fresh);
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "3000");
    bg_queue(BgKind::CompactPages, Some("20261109-101112-ab12"), None, DPI, false);
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    assert_eq!(bg_children(), 1);
    // (the child closes its copy first thing, before its delay: a pick is a person's tap, later than that)
    std::thread::sleep(std::time::Duration::from_millis(300));
    lock_release_if_idle();
    assert_eq!(lock_fd(), -1);
    let sp = StoragePath::new(b"documents/Book.pdf").unwrap();
    let q = pdf_import_queue(&sp, None, None, DPI);
    assert!(matches!(q, Queued::Started(_)), "the compaction's child holds no lock: {q:?}");
    drain();
    assert_eq!(e.notebooks(), ["Book"]);
}

// ---- M4: a folder's pictures and a PDF that cannot be imported ----

#[test]
fn a_folders_pictures_import_when_its_pdf_is_refused() {
    let e = setup("mixed");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    e.put("grimoire/import/Score/Locked.pdf", &pdffix::encrypted_planner(1, "secret", "owner"));
    for k in 1..=2 {
        let p = BgPicture { px: vec![120u8; 150 * 200], w: 150, h: 200, ch: 1 };
        e.put(&format!("grimoire/import/Score/page-{k}.png"), &encode_png(&p).unwrap());
    }
    for _ in 0..3 {
        grimoire::bgwork_template::import_scan(DPI);
        drain();
    }
    assert_eq!(e.notebooks(), ["Score"], "the pictures became a notebook once the PDF beside them was refused");
}

// ---- M5: a pick inside grimoire/import ----

#[test]
fn a_pdf_picked_in_the_import_place_is_filed_and_not_queued_again() {
    let e = setup("pick-in-import");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    e.put("grimoire/import/Planner.pdf", &pdffix::planner(2));
    let sp = StoragePath::new(b"grimoire/import/Planner.pdf").unwrap();
    assert!(matches!(pdf_import_queue(&sp, Some("Mine"), None, DPI), Queued::Started(_)));
    let first = drain();
    assert_eq!(e.notebooks(), ["Mine"], "{first:?}");
    assert!(e.names("grimoire/import").iter().all(|n| n != "Planner.pdf"), "a pick in import/ is a drop: it is filed in done/");
    assert_eq!(e.names("grimoire/import/done").len(), 1);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    assert!(drain().is_empty());
}

#[test]
fn a_picture_folder_picked_in_the_import_place_becomes_one_notebook() {
    let e = setup("folder-in-import");
    let dir = format!("{}/import/Scans", e.root);
    std::fs::create_dir_all(&dir).unwrap();
    for (k, v) in [(1, 40u8), (2, 90u8)] {
        let p = BgPicture { px: vec![v; 150 * 200], w: 150, h: 200, ch: 1 };
        assert_eq!(bgimage_write_png(&format!("{dir}/page-{k}.png"), &p), 0);
    }
    let sp = StoragePath::new(b"grimoire/import/Scans").unwrap();
    let q = grimoire::bgwork_template::import_pick_queue(&sp, None, None, DPI);
    assert!(matches!(q, Queued::Started(_)), "{q:?}");
    // the scan while the pick's job is queued leaves the folder to it
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    drain();
    assert_eq!(e.notebooks(), ["Scans"]);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    drain();
    assert_eq!(e.notebooks(), ["Scans"]);
    assert!(!std::path::Path::new(&dir).exists() && e.names("grimoire/import/done").len() == 1, "filed in done/: {:?}", e.names("grimoire/import/done"));
}

// ---- M7: scans that never stop being due, and files that never hold still ----

#[test]
fn a_rescan_that_finds_nothing_is_not_due_at_every_tick() {
    let e = setup("rescan");
    let f = format!("{}/import/X.pdf", e.root);
    std::fs::write(&f, pdffix::planner(1)).unwrap();
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    let now = now_ms();
    assert!(scan_due(now.wrapping_add(WAIT_RESCAN_MS + 100), false), "a look is due 5 s on");
    std::fs::remove_file(&f).unwrap();
    let mut ran = 0;
    for k in 0..50u32 {
        let t = now.wrapping_add(WAIT_RESCAN_MS + 100 + k * 30);
        if scan_due(t, false) {
            scan_noted(t);
            grimoire::bgwork_template::import_scan(DPI);
            ran += 1;
        }
    }
    assert_eq!(ran, 1, "of 50 ticks 30 ms apart, {ran} ran a scan");
}

#[test]
fn more_than_256_waiting_pdfs_hold_still_in_the_end() {
    let mut seen = Vec::new();
    let mut steady = 0;
    for scan in 0..3u32 {
        for key in 1..=300u64 {
            if time_stable(&mut seen, key, 4096, 1_000, scan * 5_000, 2_000, STABLE_GAP_MS, STABLE_AGE_S) {
                steady += 1;
            }
        }
    }
    assert_eq!(steady, 600, "each of 300 files is steady at the second and third looks");
}

#[test]
fn a_file_dated_ahead_of_the_clock_is_taken_in_the_end() {
    let mut seen = Vec::new();
    let mtime = 1_800_000_000i64;
    let wall0 = mtime - 3600; // the Kindle's clock an hour behind the file's stamp
    let taken = (0..600u32).find(|&k| time_stable(&mut seen, 1, 4096, mtime, k * 1000, wall0 + k as i64, STABLE_GAP_MS, STABLE_AGE_S));
    assert!(taken.is_some_and(|k| k <= 3), "taken at {taken:?} s");
}

// ---- M8: the end of a previous run's detached import ----

#[test]
fn the_notice_of_a_detached_import_goes_when_that_import_is_over() {
    let e = setup("detached-flag");
    e.put("grimoire/import/Planner.pdf", &pdffix::planner(2));
    let other = e.hold_lock();
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    assert!(detached_running());
    drop(other);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    assert!(!detached_running(), "a scan that took the lock knows no other process imports");
}

#[test]
fn the_ui_learns_within_ten_seconds_that_the_detached_import_ended() {
    let e = setup("detached-end");
    e.put("grimoire/import/Next.pdf", &pdffix::planner(2));
    let other = e.hold_lock();
    let now = now_ms();
    scan_noted(now);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0);
    assert!(detached_running());
    drop(other); // the detached child ends
    // the Library stays in front: the app's tick (detached_poll, then a scan when one is due)
    let mut flipped_at = None;
    for k in 1..=60u32 {
        let t = now.wrapping_add(k * 500);
        if detached_poll(t) && flipped_at.is_none() {
            flipped_at = Some(k * 500);
        }
        if scan_due(t, false) {
            scan_noted(t);
            std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
            grimoire::bgwork_template::import_scan(DPI);
        }
    }
    assert!(flipped_at.is_some_and(|ms| ms <= 10_000), "seen at {flipped_at:?} ms");
    assert!(!detached_running());
    drain();
    assert_eq!(e.notebooks(), ["Next"], "the file waiting in import/ was taken");
}

// ---- M9: the notice's page count ----

#[test]
fn an_import_that_goes_on_after_a_restart_shows_its_notice() {
    let e = setup("resume-notice");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    e.put("grimoire/import/Planner.pdf", &pdffix::planner(6));
    // the run before: the probe and one page (another thread: its own store, queue and record)
    std::thread::spawn(|| {
        attach("resume-notice");
        let src = "grimoire/import/Planner.pdf";
        let id = new_id();
        assert!(mark_write_public(&Mark { state: MarkState::Making, id: id.clone(), src: src.into(), ..Default::default() }));
        assert_eq!(run_slice(&id, src), SliceEnd::More);
        assert_eq!(run_slice(&id, src), SliceEnd::More);
    })
    .join()
    .unwrap();
    let _ = &e;
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "120");
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    let mut seen: Vec<Option<(u32, u32)>> = Vec::new();
    let t0 = std::time::Instant::now();
    loop {
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::PdfImport as i32 {
            finished(id.as_str(), r > 0);
        }
        if pending_count() > 0 {
            let p = progress_text(now_ms());
            if seen.last() != Some(&p) {
                seen.push(p);
            }
        }
        if !bg_busy() {
            break;
        }
        assert!(t0.elapsed().as_secs() < 90);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(seen.iter().any(|p| matches!(p, Some((_, 6)))), "the notice had a text: {seen:?}");
}

#[test]
fn two_imports_in_turns_each_show_their_own_count() {
    let e = setup("two-notice");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "120");
    e.put("grimoire/import/A.pdf", &pdffix::planner(3));
    e.put("grimoire/import/B.pdf", &pdffix::planner(7));
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 2);
    let mut seen: Vec<(usize, Option<(u32, u32)>)> = Vec::new();
    let t0 = std::time::Instant::now();
    loop {
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::PdfImport as i32 {
            assert!(matches!(finished(id.as_str(), r > 0), Finished::Imported { .. }));
        }
        if pending_count() > 0 {
            let p = (pending_count(), progress_text(now_ms()));
            if seen.last() != Some(&p) {
                seen.push(p);
            }
        }
        if !bg_busy() {
            break;
        }
        assert!(t0.elapsed().as_secs() < 90);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(seen.iter().filter_map(|(_, p)| *p).all(|(page, n)| page <= n && (n == 3 || n == 7)), "{seen:?}");
    assert!(seen.iter().any(|(n, p)| *n == 1 && matches!(p, Some((_, 7)))), "the last import has its notice: {seen:?}");
}

// ---- M11: a job the full queue drops ----

#[test]
fn an_import_the_full_queue_drops_is_queued_again() {
    let e = setup("dropped");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "150");
    e.put("grimoire/import/A.pdf", &pdffix::planner(1));
    let mut b = pdffix::planner(2);
    b.extend_from_slice(b"\n% another file\n%%EOF\n");
    e.put("grimoire/import/B.pdf", &b);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 2);
    for k in 0..16 {
        bg_mirror_pdf(&format!("20260101-0000{k:02}-aaaa"), DPI, "nowhere", false);
    }
    drain();
    assert!(!bg_busy());
    // the next scan lets go of what the queue dropped, and takes the file again
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    drain();
    assert_eq!(e.notebooks(), ["A", "B"]);
    assert_eq!(pending_count(), 0);
    assert_eq!(lock_fd(), -1, "the lock is let go of");
}

// ---- H9: two copies of one PDF ----

#[test]
fn a_second_copy_says_already_once_the_first_is_imported() {
    let e = setup("twins");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let bytes = pdffix::planner(3);
    e.put("grimoire/import/First.pdf", &bytes);
    e.put("grimoire/import/Second.pdf", &bytes);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 2);
    let mut told = drain();
    // the copy waited (said nothing); once the first is in, the next scan takes it
    assert!(scan_due(now_ms(), false), "a scan is due as soon as the imports are done");
    grimoire::bgwork_template::import_scan(DPI);
    told.extend(drain());
    assert_eq!(e.notebooks().len(), 1, "{told:?}");
    assert!(told.iter().any(|t| t == "Already"), "{told:?}");
    assert_eq!(e.names("grimoire/import/done").len(), 2);
}

// A PDF that never gets its end marker is taken as it is once it has not changed for a minute (it waited as
// "Waiting" for good, with a look every 5 s).
#[test]
fn a_pdf_without_its_end_marker_is_taken_after_a_minute_unchanged() {
    let e = setup("never-whole");
    let mut b = pdffix::planner(2);
    let end = b.windows(5).rposition(|w| w == b"%%EOF").unwrap();
    b.truncate(end);
    e.put("grimoire/import/Cut.pdf", &b);
    let c = std::ffi::CString::new(format!("{}/import/Cut.pdf", e.root)).unwrap();
    let tv = [libc::timeval { tv_sec: 1_700_000_000, tv_usec: 0 }, libc::timeval { tv_sec: 1_700_000_000, tv_usec: 0 }];
    // SAFETY: utimes reads the path and the two timevals we own.
    assert_eq!(unsafe { libc::utimes(c.as_ptr(), tv.as_ptr()) }, 0);
    let t0 = 5_000_000u32;
    let mut queued = Vec::new();
    for dt in [0u32, 3_000, 30_000, 61_000] {
        grimoire::util::sim_clock_set(Some(t0 + dt));
        queued.push(grimoire::bgwork_template::import_scan(DPI));
    }
    grimoire::util::sim_clock_set(None);
    drain();
    assert_eq!(queued, vec![0, 0, 0, 1]);
}
