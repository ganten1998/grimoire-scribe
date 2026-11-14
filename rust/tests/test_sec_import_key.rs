// test_sec_import_key.rs: "PDF already imported." means that very file (the hardening of 2026-11). A book's key
// is KOReader's partial MD5: twelve samples of 1 KiB at fixed places, so two files can have one key. A notebook
// made from a PDF keeps the SHA-256 and the size of its file (one line in its source.ini), and a PDF that has a
// notebook's key is compared with that record: the same file is "already imported", another file becomes a
// notebook of its own, and the book's first notebook keeps the key for KOReader. A notebook without a record
// (made by an earlier version, or from KOReader) goes by its key, as it always did.
//
// The tests are the writer (pdfimport::run_slice in this process, every page through a real worker, a fresh
// exec of the headless runner), as test_pdfimport.rs. Generated PDFs only: pdffix::shared_key_set.
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::browser::{Browser, PickKind, PickSpec};
use grimoire::fsview::StoragePath;
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use grimoire::sha256::sha256_of;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

struct Env {
    /// Grimoire's own folder (the storage root is its parent)
    root: String,
    storage: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

const HOOKS: [&str; 14] = [
    "GRIMOIRE_PDF_TEST_WOKE",
    "GRIMOIRE_PDF_JOB_CPU_SECS",
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
];

fn setup(name: &str) -> Env {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in HOOKS {
        std::env::remove_var(h);
    }
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let storage = base.join(format!("grimoire-sec-import-key-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::fs::create_dir_all(storage.join("documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, PW, PH);
    ui_reset();
    Env { root: root.to_str().unwrap().to_string(), storage: storage.to_str().unwrap().to_string(), _g: g }
}

impl Env {
    /// Puts a PDF in import/ and returns its StoragePath text.
    fn drop(&self, name: &str, bytes: &[u8]) -> String {
        std::fs::write(format!("{}/import/{name}", self.root), bytes).unwrap();
        format!("grimoire/import/{name}")
    }
    fn notebooks(&self) -> Vec<NotebookInfo> {
        let mut v = Vec::new();
        store_list(&mut v);
        v
    }
    fn done_files(&self) -> usize {
        std::fs::read_dir(format!("{}/import/done", self.root)).map(|r| r.flatten().count()).unwrap_or(0)
    }
    /// Whether a file of import/done holds exactly these bytes.
    fn filed(&self, bytes: &[u8]) -> bool {
        std::fs::read_dir(format!("{}/import/done", self.root)).is_ok_and(|r| r.flatten().any(|e| std::fs::read(e.path()).is_ok_and(|b| b == bytes)))
    }
    /// The marks in import/.marks, as text.
    fn marks(&self) -> Vec<String> {
        std::fs::read_dir(format!("{}/import/.marks", self.root)).map(|r| r.flatten().filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect()).unwrap_or_default()
    }
    /// The path of import/<name> on the host.
    fn at(&self, name: &str) -> String {
        format!("{}/import/{name}", self.root)
    }
    /// A file of notebook `id` (notebook.ini, source.ini), as text.
    fn ini(&self, id: &str, file: &str) -> String {
        std::fs::read_to_string(format!("{}/notebooks/{id}/{file}", self.root)).unwrap_or_default()
    }
}

/// Runs the slices of one import to its end: (its id, the ends in order).
fn import_all(src: &str) -> (String, Vec<SliceEnd>) {
    let mut id = String::new();
    store_make_id(&mut id);
    let mut ends = Vec::new();
    for _ in 0..400 {
        let e = run_slice(&id, src);
        let more = e == SliceEnd::More;
        ends.push(e);
        if !more {
            break;
        }
    }
    (id, ends)
}

fn last(ends: &[SliceEnd]) -> &SliceEnd {
    ends.last().unwrap()
}

const DONE: SliceEnd = SliceEnd::Done { pages: 2, blank: 0 };

/// A notebook page's picture: (w, h, channels) and its pixels, decoded by the png crate.
fn picture_of(root: &str, id: &str, page: u32) -> (u32, u32, u32, Vec<u8>) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let mut pgbox = None;
    assert_eq!(store_load_page(&nb, page as i32 - 1, PW, PH, &mut pgbox), 0, "the page file reads");
    let paper = pgbox.as_ref().unwrap().paper;
    assert!(paper_is_document(paper));
    let pic = paper_image_id(paper);
    let path = format!("{root}/notebooks/{id}/bg/{pic:06x}.png");
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
    let mut r = dec.read_info().unwrap();
    let ch = r.info().color_type.samples() as u32;
    let mut buf = vec![0u8; r.output_buffer_size().unwrap()];
    let f = r.next_frame(&mut buf).unwrap();
    buf.truncate(f.buffer_size());
    (f.width, f.height, ch, buf)
}

/// The gray of the block of page 2 of notebook `id` (the planner's block, where the fit of a 420 x 595 page on
/// 600 x 800 puts its centre).
fn block_gray(root: &str, id: &str) -> i32 {
    let p = picture_of(root, id, 2);
    let s = (600.0f64 / 420.0).min(800.0 / 595.0);
    let (rw, rh) = ((420.0 * s) as i32, (595.0 * s) as i32);
    let (ox, oy) = ((600 - rw) / 2, (800 - rh) / 2);
    let (cx, cy) = ((ox as f64 + 210.0 * s) as usize, (oy as f64 + (595.0 - 460.0) * s) as usize);
    p.3[(cy * p.0 as usize + cx) * p.2 as usize] as i32
}

// the grays pdffix::shared_key_set draws page 2's block in: 0.25, 0.75 and 0.50 of 255
const GRAY_A: i32 = 64;
const GRAY_B: i32 = 191;
const GRAY_C: i32 = 128;

fn key_of(bytes: &[u8]) -> String {
    grimoire::md5::partial_md5(|buf, off| {
        let off = (off as usize).min(bytes.len());
        let n = buf.len().min(bytes.len() - off);
        buf[..n].copy_from_slice(&bytes[off..off + n]);
        n
    })
}

/// The three PDFs that are equal in every sampled KiB, checked to be what the tests take them for, and their key.
fn the_set() -> (Vec<u8>, Vec<u8>, Vec<u8>, String) {
    let (a, b, c) = pdffix::shared_key_set();
    let key = key_of(&a);
    assert_eq!(key.len(), 32);
    assert!(key_of(&b) == key && key_of(&c) == key, "the three files have one book key");
    assert!(a.len() == b.len() && a != b, "A and B have one length and other bytes");
    assert!(c.len() > a.len(), "C is longer than A");
    assert!(a.len() >= 5 * 1024 && c.len() < 16 * 1024, "every sample is whole, and there is no fourth: {} and {} bytes", a.len(), c.len());
    (a, b, c, key)
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

// Two files of one key and one length: the second is another file, and becomes a notebook of its own. Each file
// dropped again says "already", by its own notebook's record.
#[test]
fn a_pdf_that_only_shares_a_books_key_becomes_a_notebook_of_its_own() {
    let e = setup("own-notebook");
    let (a, b, _, _) = the_set();
    let (id_a, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    let (id_b, ends) = import_all(&e.drop("B.pdf", &b));
    assert_eq!(last(&ends), &DONE, "a file that only shares the book's key was taken for the book: {ends:?}");
    let names: Vec<String> = e.notebooks().iter().map(|n| n.name.as_str().to_string()).collect();
    assert!(names.len() == 2 && names.contains(&"A".to_string()) && names.contains(&"B".to_string()), "{names:?}");
    // each notebook's page 2 is its own file's
    let (ga, gb) = (block_gray(&e.root, &id_a), block_gray(&e.root, &id_b));
    assert!((ga - GRAY_A).abs() <= 2 && (gb - GRAY_B).abs() <= 2, "page 2: {ga} and {gb}, wanted {GRAY_A} and {GRAY_B}");
    assert_eq!(e.done_files(), 2, "both files are filed in import/done");
    // the same two files again, under other names
    for (name, bytes) in [("A again.pdf", &a), ("B again.pdf", &b)] {
        let (_, ends) = import_all(&e.drop(name, bytes));
        assert_eq!(last(&ends), &SliceEnd::Already, "{name}: {ends:?}");
    }
    assert_eq!(e.notebooks().len(), 2);
    assert_eq!(e.done_files(), 4);
}

// One notebook holds a book's bare key (`source=` in notebook.ini, `book=` in source.ini): the first. KOReader's
// route and every earlier build go by those two lines, so they keep finding the book's own notebook; the second
// notebook carries the key in its record alone.
#[test]
fn the_books_first_notebook_keeps_the_key() {
    let e = setup("first-keeps-key");
    let (a, b, _, key) = the_set();
    let (id_a, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    let (id_b, ends) = import_all(&e.drop("B.pdf", &b));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    let has = |text: &str, line: &str| text.lines().any(|l| l == line);
    let (nb_a, src_a) = (e.ini(&id_a, "notebook.ini"), e.ini(&id_a, "source.ini"));
    assert!(has(&nb_a, &format!("source={key}")), "{nb_a}");
    assert!(has(&src_a, &format!("book={key}")), "{src_a}");
    let (nb_b, src_b) = (e.ini(&id_b, "notebook.ini"), e.ini(&id_b, "source.ini"));
    assert!(!nb_b.lines().any(|l| l.starts_with("source=")), "{nb_b}");
    assert!(!src_b.lines().any(|l| l.starts_with("book=")), "{src_b}");
    assert!(has(&src_b, "kind=paged") && has(&src_b, "page.1.2=1") && has(&src_b, "page.2.2=2"), "{src_b}");
    assert!(src_b.lines().any(|l| l.starts_with("file=sha256:") && l.ends_with(&format!(":{key}"))), "its record names the key: {src_b}");
    // KOReader's Open this book in Grimoire, a request by the key for page 1 of 2: the book's first notebook,
    // though the other is the newer
    let dir = format!("{}/{}", e.root, ANNOTATE_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(bgimage_write_png(&format!("{dir}/page.png"), &BgPicture { px: vec![200u8; (PW * PH) as usize], w: PW, h: PH, ch: 1 }), 0);
    let request = format!("version=1\nbook={key}\ntitle=A\npage=1\npages=2\nkind=paged\npicture=page.png\ntime={}\n", wall_time());
    std::fs::write(format!("{dir}/{ANNOTATE_REQUEST}"), request).unwrap();
    let mut r = AnnotateResult::default();
    annotate_take(true, &mut r);
    assert_eq!(r.outcome, AnnotateOutcome::Opened);
    assert_eq!(r.nb.as_str(), id_a, "the request opened {} (the first is {id_a}, the second {id_b})", r.nb.as_str());
    // and both notebooks still hold what they held
    assert_eq!(e.ini(&id_a, "source.ini"), src_a);
    assert_eq!(e.ini(&id_b, "notebook.ini"), nb_b);
}

// The record: one line of the notebook's source.ini, of the file as it was read when the notebook was made. A
// version that does not know the line keeps it when it writes the file again (it parses the text and writes it
// back, as adding a page does).
#[test]
fn a_notebook_keeps_the_record_of_its_file() {
    let e = setup("record");
    let (a, _, _, key) = the_set();
    let (id, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    let text = e.ini(&id, "source.ini");
    let line = format!("file=sha256:{}:{}:{key}", sha256_of(&a), a.len());
    assert!(text.lines().any(|l| l == line), "source.ini has no record of its file:\n{text}\nwanted: {line}");
    assert!(line.len() <= 130);
    // read and written back by the code that knows only the lines of its own
    let mut m = AnnotateMap::default();
    assert_eq!(annotate_map_parse(Some(text.as_bytes()), &mut m), 0);
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(&id, &mut nb), 0);
    let back = String::from_utf8(annotate_map_text(&m, &nb).unwrap()).unwrap();
    assert!(back.lines().any(|l| l == line), "{back}");
    assert_eq!(back, text, "the file is written back as it was");
    // and the module reads its own line
    let rec = file_record_parse(&line).expect("the record reads");
    assert_eq!((rec.sha256, rec.size, rec.key), (sha256_of(&a), a.len() as u64, key));
}

// A notebook without a record (one an earlier version made, or KOReader, or one whose record is of a form this
// version does not read) goes by its key, as before: the first file seen with that key is its book.
#[test]
fn a_notebook_without_a_record_goes_by_its_key() {
    let e = setup("no-record");
    let (a, b, _, _) = the_set();
    let (id, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    let path = format!("{}/notebooks/{id}/source.ini", e.root);
    let with = std::fs::read_to_string(&path).unwrap();
    let without: String = with.lines().filter(|l| !l.starts_with("file=")).map(|l| format!("{l}\n")).collect();
    for (round, text) in [("no record", without.clone()), ("a record of another form", format!("{without}file=md5:0123456789abcdef0123456789abcdef:{}\n", a.len()))] {
        std::fs::write(&path, &text).unwrap();
        let (_, ends) = import_all(&e.drop("B.pdf", &b));
        assert_eq!(last(&ends), &SliceEnd::Already, "{round}: {ends:?}");
        assert_eq!(e.notebooks().len(), 1, "{round}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "{round}: the notebook is given no record later");
    }
    // a notebook an older version saved lost notebook.ini's `source=` line: its source.ini's `book=` holds the key
    let info = format!("{}/notebooks/{id}/notebook.ini", e.root);
    let cut: String = std::fs::read_to_string(&info).unwrap().lines().filter(|l| !l.starts_with("source=")).map(|l| format!("{l}\n")).collect();
    std::fs::write(&info, cut).unwrap();
    std::fs::write(&path, &without).unwrap();
    let (_, ends) = import_all(&e.drop("B.pdf", &b));
    assert_eq!(last(&ends), &SliceEnd::Already, "the key in source.ini alone: {ends:?}");
    assert_eq!(e.notebooks().len(), 1);
}

fn picker_at(place: &str) -> Browser {
    let mut b = Browser::default();
    assert!(b.open_pick(PickSpec { kinds: &[PickKind::Pdf, PickKind::PictureFolder] }));
    assert!(b.go(StoragePath::new(place.as_bytes()).unwrap()));
    b
}

fn probe(b: &Browser, key: &str) -> String {
    let mut o = String::new();
    b.probe(2480, key, &mut o);
    o
}

/// The idle tick until every row's key was read (`keyed.N`), by a count of calls: never a number of ticks chosen
/// in advance, and never a wait. False when the count ran out.
fn work_until_keyed(b: &mut Browser) -> bool {
    for k in 0..400u32 {
        if (1..=b.rows.len()).all(|n| probe(b, &format!("keyed.{n}")) == "1") {
            return true;
        }
        b.work(2480, 1000 + k);
    }
    false
}

/// (the row's tag, its card's rows) for the PDF named `name`.
fn card_of(b: &mut Browser, name: &str) -> (String, String) {
    let i = b.rows.iter().position(|r| r.name == name).unwrap_or_else(|| panic!("{name} is not a row"));
    b.open_card(i);
    (probe(b, &format!("notes.{}", i + 1)), probe(b, "card.rows"))
}

// In Files, "has notes" is a statement about a PDF's key and size against the notebooks' keys and records (the
// picker reads no file whole). A PDF of a record's size offers Import as notebook beside Open its notebook: the
// import then gives the exact answer. A notebook whose record is of another size was made from another file and
// does not count. A notebook without a record counts by its key, as before.
#[test]
fn the_card_of_a_pdf_follows_the_record() {
    ui_init(300.0);
    i18n_set("en");
    let e = setup("card");
    let mut c = Checks::default();
    let (a, b, cc, _) = the_set();
    for (name, bytes) in [("A.pdf", &a), ("B.pdf", &b), ("C.pdf", &cc)] {
        std::fs::write(format!("{}/documents/{name}", e.storage), bytes).unwrap();
    }
    let (id_a, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");

    // only A is imported, with its record
    let mut p = picker_at("documents");
    c.that(work_until_keyed(&mut p), "every row's key is read by the idle tick, and keyed.N says so");
    let (notes, rows) = card_of(&mut p, "B.pdf");
    c.that(notes == "1", "B.pdf (the record's size) has notes");
    c.that(rows == "Open its notebook|Import as notebook|Cancel", format!("B.pdf's card: {rows}"));
    let (notes, rows) = card_of(&mut p, "C.pdf");
    c.that(notes == "0", "C.pdf (another size than the record's) has no tag");
    c.that(rows == "Import as notebook|Cancel", format!("C.pdf's card: {rows}"));
    let (notes, rows) = card_of(&mut p, "A.pdf");
    c.that(notes == "1" && rows == "Open its notebook|Import as notebook|Cancel", format!("A.pdf's card: {rows}"));

    // A's notebook without its record: the key decides, as it did
    let path = format!("{}/notebooks/{id_a}/source.ini", e.root);
    let with = std::fs::read_to_string(&path).unwrap();
    let without: String = with.lines().filter(|l| !l.starts_with("file=")).map(|l| format!("{l}\n")).collect();
    std::fs::write(&path, &without).unwrap();
    let mut p = picker_at("documents");
    work_until_keyed(&mut p);
    let (notes, rows) = card_of(&mut p, "C.pdf");
    c.that(notes == "1" && rows == "Open its notebook|Cancel", format!("without a record, C.pdf goes by the key: {notes}, {rows}"));
    std::fs::write(&path, &with).unwrap();

    // C imported as well (read in place): each file names its own notebook
    let (id_c, ends) = import_all("documents/C.pdf");
    c.that(last(&ends) == &DONE, format!("C.pdf becomes a notebook of its own: {ends:?}"));
    let mut p = picker_at("documents");
    work_until_keyed(&mut p);
    let of = |p: &Browser, name: &str| p.rows.iter().find(|r| r.name == name).and_then(|r| r.notebook.clone());
    c.that(of(&p, "A.pdf").as_deref() == Some(id_a.as_str()), format!("A.pdf names A's notebook: {:?}", of(&p, "A.pdf")));
    c.that(of(&p, "C.pdf").as_deref() == Some(id_c.as_str()), format!("C.pdf names its own notebook: {:?}", of(&p, "C.pdf")));
    if last(&ends) == &DONE {
        let (ga, gc) = (block_gray(&e.root, &id_a), block_gray(&e.root, &id_c));
        c.that((ga - GRAY_A).abs() <= 2 && (gc - GRAY_C).abs() <= 2, format!("each notebook's page 2 is its own file's: {ga} and {gc}"));
    }
    c.done();
}

// ---- the file that is filed is the file that was read ----

fn set_mtime(path: &str, secs: i64) {
    let c = std::ffi::CString::new(path).unwrap();
    let tv = [libc::timeval { tv_sec: secs as libc::time_t, tv_usec: 0 }, libc::timeval { tv_sec: secs as libc::time_t, tv_usec: 0 }];
    // SAFETY: utimes reads the path and the two timevals we own.
    assert_eq!(unsafe { libc::utimes(c.as_ptr(), tv.as_ptr()) }, 0);
}

/// One time for a file and for the file that takes its name (so the two differ in nothing a listing shows).
const T: i64 = 1_700_000_000;

/// `other` takes the name import/<name> as the next import is about to file its PDF: written now, under a name of
/// its own (so it is a file of its own number), given the time `mtime` when one is named, and renamed over the
/// name at that moment (pdfimport's seam for the tests: nothing waits for it).
fn takes_the_name_at_filing(e: &Env, name: &str, other: &[u8], mtime: Option<i64>) {
    let (at, tmp) = (e.at(name), e.at(".incoming"));
    std::fs::write(&tmp, other).unwrap();
    if let Some(t) = mtime {
        set_mtime(&tmp, t);
    }
    sim_before_filing(move || std::fs::rename(&tmp, &at).unwrap());
}

// An import reads its file whole before it files it (to compare it with a notebook's record, or to make the
// record), and a file of import/ is filed by its name. The name is looked at once more just before the move: it
// must still hold the file that was read (that very file, with the size and the time it was opened with). What
// took the name meanwhile is not moved and not marked: it stays in import/, and the next scan imports it as the
// file it is.
#[test]
fn a_file_that_took_the_name_while_the_first_was_read_is_left_for_the_next_scan() {
    let e = setup("name-taken");
    let mut c = Checks::default();
    let (a, b, cc, _) = the_set();
    let (_, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    assert!(e.done_files() == 1 && e.filed(&a));

    // 1. "imported before": A's bytes again, and B (A's key, length and time, other bytes) takes the name
    let src = e.drop("Again.pdf", &a);
    set_mtime(&e.at("Again.pdf"), T);
    takes_the_name_at_filing(&e, "Again.pdf", &b, Some(T));
    let (_, ends) = import_all(&src);
    c.that(last(&ends) == &SliceEnd::Already, format!("1: the file that was read is the book's own: {ends:?}"));
    c.that(std::fs::read(e.at("Again.pdf")).is_ok_and(|x| x == b), "1: the file that took the name is still at it");
    c.that(e.done_files() == 1 && !e.filed(&b), format!("1: it was not filed in import/done ({} file(s) there)", e.done_files()));
    c.that(e.marks().is_empty(), format!("1: no mark says it was imported: {:?}", e.marks()));
    c.that(e.notebooks().len() == 1, "1: one notebook so far");
    // the next scan's import of that name takes the file that is there
    let (id_b, ends) = import_all(&src);
    c.that(last(&ends) == &DONE, format!("1: the file at the name is imported as itself: {ends:?}"));
    if last(&ends) == &DONE {
        let g = block_gray(&e.root, &id_b);
        c.that((g - GRAY_B).abs() <= 2, format!("1: its notebook's page 2 is its own: {g}, wanted {GRAY_B}"));
    }
    c.that(e.filed(&b) && !std::path::Path::new(&e.at("Again.pdf")).exists(), "1: and is filed once it was imported");

    // 2. a first import: C becomes a notebook, and a file of another length takes its name while C is read
    let other = pdffix::planner(3);
    let src = e.drop("Long.pdf", &cc);
    takes_the_name_at_filing(&e, "Long.pdf", &other, None);
    let before = e.done_files();
    let (id_c, ends) = import_all(&src);
    c.that(last(&ends) == &DONE, format!("2: the file that was read became a notebook: {ends:?}"));
    if last(&ends) == &DONE {
        let g = block_gray(&e.root, &id_c);
        c.that((g - GRAY_C).abs() <= 2, format!("2: of its own pages: {g}, wanted {GRAY_C}"));
    }
    c.that(std::fs::read(e.at("Long.pdf")).is_ok_and(|x| x == other), "2: the file that took the name is still at it");
    c.that(e.done_files() == before && !e.filed(&other), "2: it was not filed in import/done");
    c.that(e.marks().is_empty(), format!("2: no mark says it was imported: {:?}", e.marks()));
    let (_, ends) = import_all(&src);
    c.that(last(&ends) == &SliceEnd::Done { pages: 3, blank: 0 }, format!("2: the file at the name is imported as itself: {ends:?}"));
    c.that(e.filed(&other), "2: and is filed once it was imported");

    // 3. the file itself, with more written to it while it was read: no longer the file that was read
    let grown = pdffix::planner(1);
    let src = e.drop("Grown.pdf", &grown);
    let at = e.at("Grown.pdf");
    sim_before_filing(move || {
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(&at).unwrap().write_all(b"\n% more\n").unwrap();
    });
    let before = e.done_files();
    let (_, ends) = import_all(&src);
    c.that(last(&ends) == &SliceEnd::Done { pages: 1, blank: 0 }, format!("3: {ends:?}"));
    c.that(std::fs::read(e.at("Grown.pdf")).is_ok_and(|x| x.len() == grown.len() + 8), "3: the file that changed is still at its name");
    c.that(e.done_files() == before && e.marks().is_empty(), "3: it was not filed, and no mark says it was imported");

    // 4. the name gone while the file was read: nothing to file, and no mark is left for a file that is not there
    let src = e.drop("Gone.pdf", &pdffix::planner(2));
    let at = e.at("Gone.pdf");
    sim_before_filing(move || std::fs::remove_file(&at).unwrap());
    let before = e.done_files();
    let (_, ends) = import_all(&src);
    c.that(last(&ends) == &DONE, format!("4: {ends:?}"));
    c.that(e.done_files() == before && e.marks().is_empty(), format!("4: nothing filed and no mark left: {:?}", e.marks()));
    c.done();
}

// A name that never holds the file that was read is not taken at every scan for ever. Each such end leaves its try
// counted, so the file is tried PDF_TRIES times with those contents, as a file that keeps failing is, and then
// waits until it changes. (Here an equal copy takes the name each time. On a device it would take a storage that
// named one file differently by its name and by an open file: docs/TESTING.md has the check for that.)
#[test]
fn a_name_that_never_holds_the_file_read_is_tried_three_times_and_no_more() {
    let e = setup("name-never");
    let mut c = Checks::default();
    let (a, _, _, _) = the_set();
    let (_, ends) = import_all(&e.drop("A.pdf", &a));
    assert_eq!(last(&ends), &DONE, "{ends:?}");
    let src = e.drop("Same.pdf", &a);
    set_mtime(&e.at("Same.pdf"), T);
    for k in 1..=PDF_TRIES {
        takes_the_name_at_filing(&e, "Same.pdf", &a, Some(T));
        let (_, ends) = import_all(&src);
        c.that(last(&ends) == &SliceEnd::Already, format!("try {k}: {ends:?}"));
        let status = import_status(&src, a.len() as u64, T);
        let want = if k < PDF_TRIES { ImportStatus::Waiting } else { ImportStatus::Failed };
        c.that(status == want, format!("after try {k} of {PDF_TRIES} the row says {status:?}, wanted {want:?}"));
    }
    c.that(std::fs::read(e.at("Same.pdf")).is_ok_and(|x| x == a), "the file is still at its name");
    c.that(e.done_files() == 1 && e.marks().is_empty(), "it was never filed, and no mark says it was imported");
    c.that(e.notebooks().len() == 1, "one notebook");
    // the same name with a file that stays is filed as before, and its record of tries goes with it
    std::fs::write(e.at("Same.pdf"), &a).unwrap();
    let (_, ends) = import_all(&src);
    c.that(last(&ends) == &SliceEnd::Already && e.done_files() == 2, format!("a file that stays at its name is filed: {ends:?}"));
    c.that(!std::fs::read_to_string(format!("{}/import/.attempts", e.root)).is_ok_and(|t| t.lines().any(|l| !l.starts_with('#'))), "no record of tries is left");
    c.done();
}
