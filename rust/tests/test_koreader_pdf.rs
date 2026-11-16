// test_koreader_pdf.rs: KOReader hands a PDF over by its path (2.2, issue #142). The plugin's version 2 request
// names the PDF it has open (`file=`) and draws nothing; Grimoire checks the path through the storage walk, as a
// picked file is, and gives the PDF to the PDF import as a pick: the same job, sandboxed worker, book key, file record
// and fill rules. The book job follows that import, holding nothing, and names the notebook it made, filled or found
// whole and the page of it the reader was on; the app opens it there when the Library's cards are still in front, and
// else keeps that page as the one the notebook opens at (book_pdf_keep_page; the scenarios y_koreader_pdf_* run the
// app's side). A notebook KOReader made through version 1 is filled with the pages it lacks; the pages it has stay as
// they are.
//
// The jobs run as the app runs them (bg_poll_job, pdfimport::finished, the UI's fill_tick), each page through a real
// worker (a fresh exec of the headless runner), as in test_pdf_fill.rs; the end of the book's own import goes to the
// book job (book_pdf_end), as app.rs gives it. Generated PDFs only (pdffix).
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use std::os::unix::fs::symlink;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
const DPI: f32 = 300.0;
static LOCK: Mutex<()> = Mutex::new(());
const TITLE: &str = "Tabs Planner 2027";

const HOOKS: [&str; 6] = ["GRIMOIRE_PDF_SLICE_PAGES", "GRIMOIRE_IMPORT_STABLE", "GRIMOIRE_SANDBOX", "GRIMOIRE_BG_DELAY_MS", "GRIMOIRE_FILL_STEP_PAGES", "GRIMOIRE_PDF_FREE_BYTES"];

struct Env {
    /// the storage root, and Grimoire's own folder in it
    storage: String,
    root: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn storage_of(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join(format!("grimoire-koreader-pdf-{}-{name}", std::process::id()))
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
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(DPI, PW, PH);
    ui_reset();
    Env { storage: storage.to_str().unwrap().to_string(), root: root.to_str().unwrap().to_string(), _g: g }
}

impl Env {
    /// A PDF in the storage's documents/ (where KOReader reads books): its absolute path.
    fn book(&self, name: &str, bytes: &[u8]) -> String {
        let path = format!("{}/documents/{name}", self.storage);
        std::fs::write(&path, bytes).unwrap();
        path
    }
    fn info(&self, id: &str) -> NotebookInfo {
        let mut nb = NotebookInfo::default();
        assert_eq!(store_load_info(id, &mut nb), 0, "notebook {id} reads");
        nb
    }
    fn notebooks(&self) -> Vec<NotebookInfo> {
        let mut v = Vec::new();
        store_list(&mut v);
        v
    }
    fn ini(&self, id: &str, file: &str) -> String {
        std::fs::read_to_string(format!("{}/notebooks/{id}/{file}", self.root)).unwrap_or_default()
    }
    fn book_dir(&self) -> String {
        format!("{}/{BOOK_DIR}", self.root)
    }
    fn marks(&self) -> usize {
        std::fs::read_dir(format!("{}/import/.marks", self.root)).map(|r| r.flatten().count()).unwrap_or(0)
    }
}

/// The book key of these bytes (KOReader's partial MD5).
fn key_of(bytes: &[u8]) -> String {
    grimoire::md5::partial_md5(|buf, off| {
        let off = (off as usize).min(bytes.len());
        let n = buf.len().min(bytes.len() - off);
        buf[..n].copy_from_slice(&bytes[off..off + n]);
        n
    })
}

/// KOReader's version 2 request for the PDF at `file` (written now), the reader on `page` of `pages`; started.
fn hand_over(e: &Env, key: &str, file: &str, page: i32, pages: i32) -> BookStatus {
    std::fs::create_dir_all(e.book_dir()).unwrap();
    let req = format!(
        "# Grimoire: a whole book to open, from KOReader's Grimoire plugin (docs/FORMAT.md)\nversion=2\nbook={key}\nfile={file}\ntitle={TITLE}\npages={pages}\npage={page}\nkind=paged\ntime={}\n",
        wall_time()
    );
    std::fs::write(format!("{}/{BOOK_REQUEST}", e.book_dir()), req).unwrap();
    let mut st = BookStatus::default();
    book_start(true, &mut st);
    st
}

/// The app's ticks while the book job runs (app.rs): its step, a fill's commit with the Library in front, and the
/// jobs' ends, the book's own import's to the book job. Then the jobs left (a fill's filing slice).
fn run_book(st: &mut BookStatus) {
    let t0 = std::time::Instant::now();
    while st.state == BookState::Running {
        book_step(150, st);
        let book = book_pdf_src();
        let f = fill_tick(true, DPI);
        if f != Finished::Nothing && book.is_some() && fill_told_src() == book {
            book_pdf_end("", f);
        }
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::PdfImport as i32 {
            let src = queued_src(id.as_str());
            let fin = finished(id.as_str(), r > 0);
            if src.is_some() && src == book_pdf_src() {
                book_pdf_end(id.as_str(), fin);
            }
        }
        assert!(t0.elapsed().as_secs() < 120, "the book job never ended: {st:?}");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while bg_busy() {
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::PdfImport as i32 {
            finished(id.as_str(), r > 0);
        }
        assert!(t0.elapsed().as_secs() < 120, "the jobs never ended");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The book pages a notebook's pages hold, in its page order (source.ini).
fn book_pages(id: &str, nb: &NotebookInfo) -> Vec<i32> {
    let mut m = AnnotateMap::default();
    assert_eq!(annotate_map_load(id, &mut m), 0);
    let v = nb.page_ids.iter().map(|p| m.e.iter().find(|e| e.pid == *p).map_or(0, |e| e.page)).collect();
    annotate_map_free(&mut m);
    v
}

/// KOReader's version 1 "Open this book in Grimoire" (it drew pages `held` of `pages`): the notebook's id.
fn koreader_book(e: &Env, key: &str, pages: i32, held: &[i32]) -> String {
    let dir = e.book_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let mut req = format!("version=1\nbook={key}\ntitle=Tabs\npages={pages}\npage={}\nkind=paged\ntime={}\n", held[0], wall_time());
    for &p in held {
        assert_eq!(bgimage_write_png(&format!("{dir}/page-{p}.png"), &BgPicture { px: vec![(180 + p) as u8; (PW * PH) as usize], w: PW, h: PH, ch: 1 }), 0);
        req.push_str(&format!("picture.{p}=page-{p}.png\n"));
    }
    std::fs::write(format!("{dir}/{BOOK_REQUEST}"), req).unwrap();
    let mut st = BookStatus::default();
    book_start(true, &mut st);
    for _ in 0..1000 {
        if st.state != BookState::Running {
            break;
        }
        book_step(150, &mut st);
    }
    assert_eq!(st.state, BookState::Done, "KOReader's version 1 book: {st:?}");
    assert!(!st.pdf, "a version 1 job is not the PDF import's: the app holds the input for it, as before");
    st.nb.as_str().to_string()
}

fn probe(key: &str) -> String {
    let mut out = String::new();
    assert_eq!(annotate_probe(key, 0, &mut out, 64), 0);
    out
}

// stderr into a file while a start runs, to hold its log to the rule: no file name, title or key
static SAVED: AtomicI32 = AtomicI32::new(-1);
fn log_capture(e: &Env) {
    let path = std::ffi::CString::new(format!("{}/../captured.log", e.root)).unwrap();
    // SAFETY: dup, open and dup2 on descriptor 2 and one this function opens, which it closes again.
    unsafe {
        SAVED.store(libc::dup(2), Ordering::Relaxed);
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
}
fn log_captured(e: &Env) -> String {
    let saved = SAVED.swap(-1, Ordering::Relaxed);
    if saved >= 0 {
        // SAFETY: puts descriptor 2 back from the copy log_capture made, and closes the copy.
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
    let t = std::fs::read_to_string(format!("{}/../captured.log", e.root)).unwrap_or_default();
    eprint!("{t}");
    t
}

// The PDF KOReader has open, handed over by its path with the reader on page 3: the PDF import makes the book's
// notebook (named after its title, keyed by its file, with the record of that file), read in place (it stays in
// documents/), and the notebook opens at book page 3. The request is taken at once; the log names no file, title or
// key.
#[test]
fn a_pdf_handed_over_by_its_path_becomes_a_notebook_open_at_the_readers_page() {
    let e = setup("new");
    let bytes = pdffix::tabs();
    let key = key_of(&bytes);
    let file = e.book("Tabs Planner.pdf", &bytes);
    log_capture(&e);
    let mut st = hand_over(&e, &key, &file, 3, 4);
    let started = (st.state, book_pdf_src(), probe("book_version"), std::path::Path::new(&e.book_dir()).exists());
    run_book(&mut st);
    let log = log_captured(&e);
    assert_eq!(started, (BookState::Running, Some("documents/Tabs Planner.pdf".to_string()), "2".to_string(), false), "the import queued, the request taken");
    assert_eq!((st.state, st.made, st.added, st.blank), (BookState::Done, true, 4, 0), "{st:?}");
    assert!(st.pdf, "a job that holds nothing (app.rs reads this)");
    let id = st.nb.as_str().to_string();
    let nb = e.info(&id);
    assert_eq!(nb.name.as_str(), TITLE);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4]);
    assert_eq!(st.pid, nb.page_ids[2], "the reader's page, book page 3");
    assert_eq!(nb.source.as_str(), key, "keyed by its file");
    let src = e.ini(&id, "source.ini");
    assert!(src.lines().any(|l| l == format!("book={key}")) && src.lines().any(|l| l.starts_with("file=sha256:")), "{src}");
    assert!(std::path::Path::new(&file).exists(), "the PDF stays where KOReader has it");
    assert_eq!(e.marks(), 0, "no mark is left");
    assert_eq!(probe("book"), "done");
    for private in ["Tabs", "documents", &key[..12], "Planner"] {
        assert!(!log.contains(private), "the log names {private:?}:\n{log}");
    }
}

// A colour page of a PDF handed over by its path keeps its colour (the PDF import's own pictures), where KOReader's
// version 1 pages are gray.
#[test]
fn a_pdf_handed_over_by_its_path_keeps_its_colour() {
    let e = setup("colour");
    let bytes = pdffix::colour();
    let file = e.book("Colour.pdf", &bytes);
    let mut st = hand_over(&e, &key_of(&bytes), &file, 2, 3);
    run_book(&mut st);
    assert_eq!((st.state, st.added), (BookState::Done, 3), "{st:?}");
    let id = st.nb.as_str().to_string();
    let nb = e.info(&id);
    assert_eq!(st.pid, nb.page_ids[1]);
    // the PNG's colour type (byte 25 of its header): 2 for RGB, 0 for gray
    let bg = format!("{}/notebooks/{id}/bg", e.root);
    let types: Vec<u8> = std::fs::read_dir(&bg)
        .unwrap()
        .flatten()
        .filter(|f| f.file_name().to_string_lossy().ends_with(".png"))
        .map(|f| std::fs::read(f.path()).unwrap()[25])
        .collect();
    assert!(types.contains(&2) && types.contains(&0), "a colour picture and gray ones: {types:?}");
}

// A notebook KOReader made through version 1 (book pages 1 and 3, drawn gray by KOReader) and then the same PDF by its
// path, the reader on page 2: the import fills it by its key with pages 2 and 4 in book order, the pages KOReader drew
// stay as they were (the same page ids and pictures), and the notebook opens at book page 2. The notebook stays
// without a file record, as a fill leaves one.
#[test]
fn a_version_2_hand_over_fills_a_notebook_koreader_made() {
    let e = setup("fill");
    let bytes = pdffix::tabs();
    let key = key_of(&bytes);
    let id = koreader_book(&e, &key, 4, &[1, 3]);
    let before = e.info(&id);
    assert_eq!(book_pages(&id, &before), [1, 3]);
    let pictures = |nb: &NotebookInfo| -> Vec<Vec<u8>> {
        nb.page_ids
            .iter()
            .map(|p| std::fs::read(format!("{}/notebooks/{id}/p{p}.qpg", e.root)).unwrap_or_default())
            .collect()
    };
    let kept = pictures(&before);
    let file = e.book("Tabs.pdf", &bytes);
    let mut st = hand_over(&e, &key, &file, 2, 4);
    run_book(&mut st);
    assert_eq!((st.state, st.made, st.added), (BookState::Done, false, 2), "{st:?}");
    assert_eq!(st.nb.as_str(), id, "the notebook KOReader made");
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4]);
    assert_eq!(st.pid, nb.page_ids[1], "the reader's page, book page 2");
    assert_eq!((nb.page_ids[0], nb.page_ids[2]), (before.page_ids[0], before.page_ids[1]), "KOReader's pages kept");
    assert_eq!(vec![pictures(&nb)[0].clone(), pictures(&nb)[2].clone()], kept, "and their files");
    let src = e.ini(&id, "source.ini");
    assert!(!src.lines().any(|l| l.starts_with("file=")), "{src}");
    assert_eq!(e.notebooks().len(), 1, "no notebook of its own");
    // Once more: every page is there ("already imported"): it opens at the reader's page, nothing added.
    let mut st = hand_over(&e, &key, &file, 4, 4);
    run_book(&mut st);
    assert_eq!((st.state, st.added, st.nb.as_str()), (BookState::Done, 0, id.as_str()), "{st:?}");
    assert_eq!(st.pid, e.info(&id).page_ids[3], "book page 4");
}

// A path the storage walk refuses fails the book, as a request that does not read does: nothing is queued or marked,
// no notebook is made, the request is gone, and the log says why without the path.
#[test]
fn a_path_the_walk_refuses_fails_the_book() {
    let e = setup("refused");
    let bytes = pdffix::tabs();
    let key = key_of(&bytes);
    let good = e.book("Tabs.pdf", &bytes);
    let s = e.storage.clone();
    symlink(&good, format!("{s}/documents/Alias.pdf")).unwrap();
    symlink(format!("{s}/documents"), format!("{s}/Linked")).unwrap();
    std::fs::create_dir_all(format!("{s}/.hidden")).unwrap();
    std::fs::write(format!("{s}/.hidden/Tabs.pdf"), &bytes).unwrap();
    std::fs::create_dir_all(format!("{}/PDFs", e.root)).unwrap();
    std::fs::write(format!("{}/PDFs/Tabs.pdf", e.root), &bytes).unwrap();
    std::fs::create_dir_all(format!("{s}/documents/Folder.pdf")).unwrap();
    let fifo = std::ffi::CString::new(format!("{s}/documents/pipe.pdf")).unwrap();
    // SAFETY: mkfifo with a NUL-terminated path this test owns.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    let outside = storage_of("refused-outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("Tabs.pdf"), &bytes).unwrap();
    let refused = [
        outside.join("Tabs.pdf").to_str().unwrap().to_string(), // outside the storage
        format!("{s}/documents/Alias.pdf"),                     // a link
        format!("{s}/Linked/Tabs.pdf"),                          // a folder on its way that is a link
        format!("{s}/documents/../documents/Tabs.pdf"),          // ".." in it
        format!("{s}//documents/Tabs.pdf"),                      // an empty component
        format!("{s}/.hidden/Tabs.pdf"),                         // a hidden folder
        format!("{}/PDFs/Tabs.pdf", e.root),                     // Grimoire's own PDF mirror
        format!("{s}/documents/Folder.pdf"),                     // a folder
        format!("{s}/documents/pipe.pdf"),                       // a FIFO (never waited on)
        format!("{s}/documents/Missing.pdf"),                    // not there
    ];
    for file in &refused {
        log_capture(&e);
        let st = hand_over(&e, &key, file, 1, 4);
        let log = log_captured(&e);
        assert_eq!((st.state, st.why, st.too_many), (BookState::Failed, AnnotateWhy::Log, false), "{file}: {st:?}");
        assert!(book_pdf_src().is_none() && pending_count() == 0 && !bg_busy() && e.marks() == 0, "{file}: nothing queued");
        assert!(!std::path::Path::new(&e.book_dir()).exists(), "{file}: the request is gone");
        assert!(log.contains("book: the PDF KOReader named ") && !log.contains("Tabs") && !log.contains("documents"), "{file}:\n{log}");
    }
    assert!(e.notebooks().is_empty());
    // the PDF itself, after all that
    let mut st = hand_over(&e, &key, &good, 1, 4);
    run_book(&mut st);
    assert_eq!((st.state, st.added), (BookState::Done, 4), "{st:?}");
}

// A PDF the import cannot read fails the book as a book fails (the import's lines say why), and nothing is made.
#[test]
fn a_pdf_the_import_refuses_fails_the_book() {
    let e = setup("junk");
    let junk = b"%PDF-1.4\nnot a PDF after all\n%%EOF\n".to_vec();
    let file = e.book("Junk.pdf", &junk);
    let mut st = hand_over(&e, &key_of(&junk), &file, 1, 1);
    assert_eq!(st.state, BookState::Running);
    run_book(&mut st);
    assert_eq!((st.state, st.why), (BookState::Failed, AnnotateWhy::Log), "{st:?}");
    assert!(e.notebooks().is_empty());
    assert!(std::path::Path::new(&file).exists(), "the PDF stays where it was");
}

// What the app needs when a hand-over's import ends with something else in front (app.rs, book_tick): how the import
// ended, once, to say it in the import's own words; and the reader's page kept as the page the notebook opens at, with
// nothing else of the notebook changed (its place among the Library's cards, by its modified time, neither).
#[test]
fn the_import_s_end_is_told_once_and_the_readers_page_is_kept() {
    let e = setup("kept");
    let bytes = pdffix::tabs();
    let file = e.book("Tabs.pdf", &bytes);
    let mut st = hand_over(&e, &key_of(&bytes), &file, 3, 4);
    assert!(st.pdf && book_pdf_told().is_none(), "nothing is told while it runs");
    run_book(&mut st);
    assert_eq!((st.state, st.added), (BookState::Done, 4), "{st:?}");
    let id = st.nb.as_str().to_string();
    assert_eq!(book_pdf_told(), Some((id.clone(), Finished::Imported { pages: 4, blank: 0, trimmed: 0 })));
    assert_eq!(book_pdf_told(), None, "once");
    let before = e.info(&id);
    assert_eq!(before.last_page, 0, "an import's notebook opens at its first page");
    let ini = e.ini(&id, "notebook.ini");
    assert!(book_pdf_keep_page(&id, st.pid));
    let after = e.info(&id);
    assert_eq!(after.last_page, 2, "book page 3");
    assert_eq!((after.modified, after.page_ids.clone(), after.name.as_str(), after.source.as_str()), (before.modified, before.page_ids.clone(), before.name.as_str(), before.source.as_str()));
    assert_eq!(e.ini(&id, "notebook.ini"), ini.replace("last_page=0", "last_page=2"), "that line alone");
    // again: nothing to write; a page the notebook does not hold, or a notebook that is not there: nothing kept
    assert!(book_pdf_keep_page(&id, st.pid));
    assert!(!book_pdf_keep_page(&id, 9999) && !book_pdf_keep_page("20200101-000000-0000", st.pid));
    assert_eq!(e.info(&id).last_page, 2);
    // A hand-over the import refuses tells the import's failure, for the app's toast.
    let junk = b"%PDF-1.4\nnot a PDF after all\n%%EOF\n".to_vec();
    let file = e.book("Junk.pdf", &junk);
    let mut st = hand_over(&e, &key_of(&junk), &file, 1, 1);
    run_book(&mut st);
    assert_eq!(st.state, BookState::Failed);
    assert!(matches!(book_pdf_told(), Some((_, Finished::Failed(FailWhy::Generic)))));
    // A path refused at the start never reached the import: nothing of it to tell.
    let st = hand_over(&e, &key_of(&bytes), &format!("{}/documents/Missing.pdf", e.storage), 1, 4);
    assert_eq!((st.state, st.pdf), (BookState::Failed, true));
    assert_eq!(book_pdf_told(), None);
}

// Another process holds the import lock (an earlier run's import, still going in its detached child): the PDF
// queues behind it as a picked PDF does, its mark waiting, and the status says so (the app's toast: "Another import
// is still going. This one starts after it."). The job follows and holds nothing, however long that import takes;
// once the lock is free the Library's scan queues the PDF, and the job ends on the notebook at the reader's page.
#[test]
fn a_hand_over_behind_another_runs_import_waits_as_a_pick_does() {
    use std::os::fd::AsRawFd;
    let e = setup("waits");
    let bytes = pdffix::tabs();
    let key = key_of(&bytes);
    let file = e.book("Tabs.pdf", &bytes);
    let other = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(format!("{}/import/.lock", e.root)).unwrap();
    // SAFETY: flock on a descriptor this test owns (another open file description than the module's).
    assert_eq!(unsafe { libc::flock(other.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
    log_capture(&e);
    let mut st = hand_over(&e, &key, &file, 3, 4);
    let log = log_captured(&e);
    assert_eq!((st.state, st.pdf, st.waits), (BookState::Running, true, true), "{st:?}");
    assert!(log.contains("book: the PDF KOReader named waits for an import of an earlier run") && log.contains("book: other imports go first"), "{log}");
    assert_eq!((e.marks(), pending_count(), bg_busy()), (1, 0, false), "its mark waits; nothing is queued");
    // (longer than the 3 s after which an import that is nowhere counts as lost: a waiting mark is somewhere)
    let t0 = std::time::Instant::now();
    while t0.elapsed().as_millis() < 3600 {
        book_step(150, &mut st);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(st.state, BookState::Running, "it follows for as long as the other import takes");
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 0, "the lock is still held elsewhere");
    drop(other); // that import ends
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1, "the Library's scan queues the pick that waited");
    run_book(&mut st);
    assert_eq!((st.state, st.made, st.added), (BookState::Done, true, 4), "{st:?}");
    assert_eq!(st.pid, e.info(st.nb.as_str()).page_ids[2], "the reader's page, book page 3");
    assert_eq!(e.marks(), 0);
}

// Imports queued ahead of the hand-over's in this run (a PDF dropped in import/ that the start's scan took): the
// status says it waits too. A hand-over with nothing ahead of it does not.
#[test]
fn a_hand_over_behind_a_queued_import_says_it_waits() {
    let e = setup("queued");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::fs::write(format!("{}/import/Colour.pdf", e.root), pdffix::colour()).unwrap();
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1, "the dropped PDF is queued first");
    let bytes = pdffix::tabs();
    let file = e.book("Tabs.pdf", &bytes);
    let mut st = hand_over(&e, &key_of(&bytes), &file, 2, 4);
    assert_eq!((st.state, st.waits, pending_count()), (BookState::Running, true, 2), "{st:?}");
    run_book(&mut st);
    std::env::remove_var("GRIMOIRE_IMPORT_STABLE");
    assert_eq!((st.state, st.added), (BookState::Done, 4), "{st:?}");
    assert_eq!(e.notebooks().len(), 2, "both PDFs became notebooks");
    // alone: nothing to wait for
    drop(e);
    let e = setup("alone");
    let file = e.book("Tabs.pdf", &bytes);
    let mut st = hand_over(&e, &key_of(&bytes), &file, 2, 4);
    assert_eq!((st.state, st.waits), (BookState::Running, false));
    run_book(&mut st);
    assert_eq!(st.state, BookState::Done);
}
