// test_pdf_fill.rs: the same PDF again fills its notebook (2.2, issue #123). A PDF whose notebook (by its file
// record, or by the book key where the notebook has none: one an earlier version made, or KOReader) lacks some of the
// book's pages gets those pages added, in book order, with the links that start or end on them and the outline's names
// for them; the pages it had, their ink, names and links stay as they were. A notebook that has every
// page says "PDF already imported."; a PDF whose page count is not the one the notebook's pages name becomes a
// notebook of its own. The writer only makes the pages; the UI process puts them in (pdfimport::fill_tick), with no
// notebook open and nothing else writing it, and a commit cut off is taken to its end, never left half.
//
// The writer runs in this process (pdfimport::run_slice, every page through a real worker, a fresh exec of the
// headless runner), as in test_pdfimport.rs; the tests of the queue drive the jobs as the app does (bg_poll_job,
// then pdfimport::finished), as test_import_queue.rs does, and the UI's commit as app.rs's tick does (fill_tick).
// None waits on a clock. Generated PDFs only (pdffix::tabs: four pages of 465 x 620 points, a tab on each, a link by
// a name on page 1, an outline naming pages 1 and 3).
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
const DPI: f32 = 300.0;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

const HOOKS: [&str; 11] = [
    "GRIMOIRE_PDF_JOB_CPU_SECS",
    "GRIMOIRE_PDF_SLICE_PAGES",
    "GRIMOIRE_PDF_BANDS",
    "GRIMOIRE_PDF_NO_MAP",
    "GRIMOIRE_PDF_FREE_BYTES",
    "GRIMOIRE_PDF_TEST_CRASH_PAGE",
    "GRIMOIRE_IMPORT_STABLE",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_BG_DELAY_MS",
    "GRIMOIRE_FILL_STEP_PAGES",
];

struct Env {
    /// Grimoire's own folder (the storage root is its parent)
    root: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn storage_of(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join(format!("grimoire-pdf-fill-{}-{name}", std::process::id()))
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
    Env { root, _g: g }
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
    fn info(&self, id: &str) -> NotebookInfo {
        let mut nb = NotebookInfo::default();
        assert_eq!(store_load_info(id, &mut nb), 0, "notebook {id} reads");
        nb
    }
    fn ini(&self, id: &str, file: &str) -> String {
        std::fs::read_to_string(format!("{}/notebooks/{id}/{file}", self.root)).unwrap_or_default()
    }
    fn done_files(&self) -> usize {
        std::fs::read_dir(format!("{}/import/done", self.root)).map(|r| r.flatten().count()).unwrap_or(0)
    }
    fn in_import(&self, name: &str) -> bool {
        std::path::Path::new(&format!("{}/import/{name}", self.root)).exists()
    }
    /// The work folders of imports (notebooks/.import-<id>), by name.
    fn work_folders(&self) -> Vec<String> {
        std::fs::read_dir(format!("{}/notebooks", self.root))
            .map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with(".import-")).collect())
            .unwrap_or_default()
    }
    /// A fill's state as its work folder holds it.
    fn fill_state(&self, fid: &str) -> State {
        let t = std::fs::read(format!("{}/notebooks/.import-{fid}/pdf-fill.ini", self.root)).expect("a fill's state");
        State::parse(&t).expect("it reads")
    }
    /// Every file of notebook `id` (its folder and bg/), with its bytes.
    fn files(&self, id: &str) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        let dir = format!("{}/notebooks/{id}", self.root);
        for sub in ["", "bg"] {
            let d = if sub.is_empty() { dir.clone() } else { format!("{dir}/{sub}") };
            for e in std::fs::read_dir(&d).map(|r| r.flatten().collect::<Vec<_>>()).unwrap_or_default() {
                if e.file_type().is_ok_and(|t| t.is_file()) {
                    let name = if sub.is_empty() { e.file_name().to_string_lossy().into_owned() } else { format!("{sub}/{}", e.file_name().to_string_lossy()) };
                    out.insert(name, std::fs::read(e.path()).unwrap());
                }
            }
        }
        out
    }
    /// The page files in notebook `id`'s folder (listed or not).
    fn page_files(&self, id: &str) -> usize {
        self.files(id).keys().filter(|k| k.starts_with('p') && k.ends_with(".qpg")).count()
    }
}

/// Runs the slices of one import to its end: (its id, the ends in order).
fn import_all(src: &str) -> (String, Vec<SliceEnd>) {
    let mut id = String::new();
    store_make_id(&mut id);
    let ends = run_on(&id, src, 400);
    (id, ends)
}

fn run_on(id: &str, src: &str, most: usize) -> Vec<SliceEnd> {
    let mut ends = Vec::new();
    for _ in 0..most {
        let e = run_slice(id, src);
        let more = e == SliceEnd::More;
        ends.push(e);
        if !more {
            break;
        }
    }
    ends
}

fn last(ends: &[SliceEnd]) -> &SliceEnd {
    ends.last().unwrap()
}

/// The UI's ticks (app.rs, the Library in front) until a commit ends, or none begins: what it says.
fn commit_all() -> Finished {
    for _ in 0..100_000 {
        let f = fill_tick(true, DPI);
        if f != Finished::Nothing || fill_busy().is_none() {
            return f;
        }
    }
    panic!("the commit never ended");
}

/// A start of the app as far as the fills go: what the UI remembered is gone, and the scan finds the work folders
/// (a fill whose pages are made is noted for the UI to put in).
fn start_again() {
    ui_reset();
    grimoire::bgwork_template::import_scan(DPI);
}

/// The pages a fill made (its work folder `fid`, the slices run in this process) put in by the UI, and its PDF filed
/// by the slice the commit queues: what the commit said.
fn put_in() -> Finished {
    start_again();
    let f = commit_all();
    drain();
    f
}

fn filled(f: &Finished, n: u32) -> bool {
    matches!(f, Finished::Filled { added, blank: 0, .. } if *added == n)
}

/// The book key of these bytes (KOReader's partial MD5), as the import and KOReader compute it.
fn key_of(bytes: &[u8]) -> String {
    grimoire::md5::partial_md5(|buf, off| {
        let off = (off as usize).min(bytes.len());
        let n = buf.len().min(bytes.len() - off);
        buf[..n].copy_from_slice(&bytes[off..off + n]);
        n
    })
}

/// Which book page each page of notebook `id` holds, in its order (0 for a page that holds none).
fn book_pages(id: &str, nb: &NotebookInfo) -> Vec<i32> {
    let mut m = AnnotateMap::default();
    assert_eq!(annotate_map_load(id, &mut m), 0);
    nb.page_ids.iter().map(|&pid| m.e.iter().find(|e| e.pid == pid).map_or(0, |e| e.page)).collect()
}

/// The document links of notebook `id` whose pages it lists: (its page's book page, its target's book page, area).
fn doc_links(id: &str, nb: &NotebookInfo) -> Vec<(i32, i32, Rect)> {
    let bp = book_pages(id, nb);
    let book = |pid: u32| nb.page_ids.iter().position(|&p| p == pid).map_or(-1, |i| bp[i]);
    let mut dl = doclinks_load(id);
    doclinks_prune(&mut dl, nb);
    let mut v: Vec<(i32, i32, Rect)> = dl.links.iter().map(|l| (book(l.pid), book(l.target), l.area())).collect();
    v.sort_by_key(|x| (x.0, x.1, x.2.x, x.2.y));
    v
}

/// No page has two links to one target whose areas are within 2 px (the fill's rule for "the same link").
fn no_link_twice(links: &[(i32, i32, Rect)]) -> bool {
    let near = |a: Rect, b: Rect| (a.x - b.x).abs() <= 2 && (a.y - b.y).abs() <= 2 && (a.x + a.w - b.x - b.w).abs() <= 2 && (a.y + a.h - b.y - b.h).abs() <= 2;
    links.iter().enumerate().all(|(i, a)| links[i + 1..].iter().all(|b| !(a.0 == b.0 && a.1 == b.1 && near(a.2, b.2))))
}

/// The name of each page of notebook `id`, in its order.
fn names(id: &str, nb: &NotebookInfo) -> Vec<String> {
    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_load_names(id, &mut nav), 0);
    let v = nb.page_ids.iter().map(|&p| nav_name(&nav, p).to_string()).collect();
    nav_free(&mut nav);
    v
}

/// Takes pages out of notebook `id`'s list (as a delete in the Pages view does; the files stay, as they do there).
fn drop_pages(id: &str, book: &[i32]) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let bp = book_pages(id, &nb);
    let keep: Vec<u32> = nb.page_ids.iter().zip(&bp).filter(|(_, b)| !book.contains(b)).map(|(&p, _)| p).collect();
    nb.page_ids = keep;
    nb.last_page = 0;
    assert_eq!(store_save_info(&mut nb), 0);
}

/// Ink on page `index` of notebook `id`: one pencil stroke.
fn ink(id: &str, index: i32) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(&nb, &mut w, &mut h);
    let mut pg = None;
    assert_eq!(store_load_page(&nb, index, w, h, &mut pg), 0);
    let mut p = pg.unwrap();
    let mut tp = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut tp);
    let mut s = stroke_new(TOOL_PENCIL, &tp, 77);
    for i in 0..10 {
        let q = PenSample { x: 50.0 + i as f32 * 10.0, y: 100.0, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i * 4 };
        stroke_append(&mut s, &q, DPI);
    }
    page_add(&mut p, s);
    assert_eq!(store_save_page(&nb, index, &p), 0);
}

/// The user's own link on page `index` of notebook `id` (links.ini), to its first page.
fn user_link(id: &str, index: usize) {
    let nb = {
        let mut nb = NotebookInfo::default();
        assert_eq!(store_load_info(id, &mut nb), 0);
        nb
    };
    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_load(id, &mut nav), 0);
    assert_eq!(nav_link_set(&mut nav, nb.page_ids[index], 0x1234_5678, &format!("p{}", nb.page_ids[nb.page_ids.len() - 1])), 0);
    assert_eq!(nav_save_links(id, &mut nav, &nb), 0);
    nav_free(&mut nav);
}

/// A page's own name (nav.ini), given by the user.
fn name_page(id: &str, index: usize, name: &str) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_load(id, &mut nav), 0);
    assert_eq!(nav_set_name(&mut nav, nb.page_ids[index], name), 0);
    assert_eq!(nav_save(id, &mut nav, &nb), 0);
    nav_free(&mut nav);
}

/// The tabs planner, imported whole: its notebook's id.
fn imported_tabs(e: &Env) -> (String, Vec<u8>) {
    let bytes = pdffix::tabs();
    let (id, ends) = import_all(&e.drop("Tabs.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 4, blank: 0 }, "{ends:?}");
    (id, bytes)
}

/// The files of a notebook that a fill never writes: the pages it had and the user's links.
fn kept(files: &BTreeMap<String, Vec<u8>>, nb: &NotebookInfo) -> BTreeMap<String, Vec<u8>> {
    files
        .iter()
        .filter(|(k, _)| k.as_str() == "links.ini" || nb.page_ids.iter().any(|p| **k == format!("p{p}.qpg")) || (k.starts_with("bg/") && k.ends_with(".png")))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

// ---- the fill ----

// A notebook made from the PDF (so with its file record) that lost two pages: the same PDF again adds them back in
// book order, with their tabs; the pages it kept are byte for byte what they were (their ink too), their names stay
// (the user's name, the outline's), the user's link stays, and no link is there twice.
#[test]
fn a_notebook_with_a_record_is_filled_with_the_pages_it_lacks() {
    let e = setup("record");
    let (id, bytes) = imported_tabs(&e);
    drop_pages(&id, &[2, 4]);
    ink(&id, 0);
    name_page(&id, 1, "Mine");
    user_link(&id, 0);
    let before = e.info(&id);
    assert_eq!(book_pages(&id, &before), [1, 3]);
    let files = e.files(&id);
    let record = e.ini(&id, "source.ini").lines().find(|l| l.starts_with("file=")).map(str::to_string).expect("a record");
    let (fid, ends) = import_all(&e.drop("Tabs again.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    let f = put_in();
    assert!(filled(&f, 2), "{f:?}");
    assert_eq!(e.notebooks().len(), 1, "no notebook of its own");
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4], "the pages in book order");
    assert_eq!(&nb.page_ids[0], &before.page_ids[0]);
    assert_eq!(&nb.page_ids[2], &before.page_ids[1]);
    // the pages it had, its pictures and the user's links: byte for byte
    let now = e.files(&id);
    for (k, v) in kept(&files, &before) {
        assert_eq!(now.get(&k), Some(&v), "{k} is as it was");
    }
    // the names: the user's stays, the outline's stays, the new pages have none (the outline names pages 1 and 3)
    assert_eq!(names(&id, &nb), ["Cover", "", "Mine", ""]);
    // the tabs: one on each page and the named link on page 1, none twice
    let links = doc_links(&id, &nb);
    let pairs: Vec<(i32, i32)> = links.iter().map(|l| (l.0, l.1)).collect();
    assert_eq!(pairs, [(1, 3), (1, 4), (2, 4), (3, 1), (4, 2)], "{links:?}");
    assert!(no_link_twice(&links));
    // the record is the file's still; the PDF is filed; no work folder is left
    assert!(e.ini(&id, "source.ini").lines().any(|l| l == record));
    assert_eq!(e.done_files(), 2);
    assert!(e.work_folders().is_empty(), "{:?}", e.work_folders());
    assert!(!std::path::Path::new(&format!("{}/notebooks/.import-{fid}", e.root)).exists());
    // and the same PDF a third time: every page is there
    let (_, ends) = import_all(&e.drop("Tabs a third time.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Already, "{ends:?}");
}

// A notebook without a record (an earlier version's, or one whose record this version does not read) is filled by
// its key, and is never given a record.
#[test]
fn a_notebook_without_a_record_is_filled_by_its_key() {
    let e = setup("no-record");
    let (id, bytes) = imported_tabs(&e);
    let path = format!("{}/notebooks/{id}/source.ini", e.root);
    let without: String = std::fs::read_to_string(&path).unwrap().lines().filter(|l| !l.starts_with("file=")).map(|l| format!("{l}\n")).collect();
    std::fs::write(&path, &without).unwrap();
    drop_pages(&id, &[1, 2]);
    let thumb = format!("{}/notebooks/{id}/thumb.pgm", e.root);
    let _ = std::fs::remove_file(&thumb);
    let (_, ends) = import_all(&e.drop("Tabs again.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    assert!(filled(&put_in(), 2));
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4]);
    let text = e.ini(&id, "source.ini");
    assert!(!text.lines().any(|l| l.starts_with("file=")), "the notebook was given a record:\n{text}");
    assert!(text.lines().any(|l| l == format!("book={}", key_of(&bytes))), "{text}");
    // the new first page is the book's page 1: its card has its picture again
    assert!(std::path::Path::new(&thumb).exists());
    assert_eq!(names(&id, &nb), ["Cover", "", "Notes", ""]);
    assert!(no_link_twice(&doc_links(&id, &nb)));
}

/// KOReader's "Open this book in Grimoire" for the tabs planner (its key), holding the book pages `held`, with the
/// links its plugin sent (`links`: book pages and rectangles in 600 x 800 pictures). Returns the notebook's id.
fn koreader_book(e: &Env, key: &str, pages: i32, held: &[i32], links: &[(i32, i32, Rect)]) -> String {
    let mut st = koreader_request(e, key, pages, held, links);
    for _ in 0..1000 {
        if st.state != BookState::Running {
            break;
        }
        book_step(150, &mut st);
    }
    assert_eq!(st.state, BookState::Done, "KOReader's book: {st:?}");
    st.nb.as_str().to_string()
}

/// KOReader's request (as koreader_book), started and left running.
fn koreader_request(e: &Env, key: &str, pages: i32, held: &[i32], links: &[(i32, i32, Rect)]) -> BookStatus {
    let dir = format!("{}/{BOOK_DIR}", e.root);
    std::fs::create_dir_all(&dir).unwrap();
    let mut req = format!("version=1\nbook={key}\ntitle=Tabs\npages={pages}\npage={}\nkind=paged\ntime={}\n", held[0], wall_time());
    for &p in held {
        assert_eq!(bgimage_write_png(&format!("{dir}/page-{p}.png"), &BgPicture { px: vec![(180 + p) as u8; (PW * PH) as usize], w: PW, h: PH, ch: 1 }), 0);
        req.push_str(&format!("picture.{p}=page-{p}.png\n"));
    }
    let mut lt = format!("P {PW} {PH}\n");
    for (s, d, r) in links {
        lt.push_str(&format!("L {s} {d} {} {} {} {}\n", r.x, r.y, r.w, r.h));
    }
    std::fs::write(format!("{dir}/links.txt"), lt).unwrap();
    std::fs::write(format!("{dir}/{BOOK_REQUEST}"), req).unwrap();
    let mut st = BookStatus::default();
    book_start(true, &mut st);
    st
}

/// The tabs planner's links as the import maps them (a notebook made of it, then removed): (from, to, area).
fn worker_links(e: &Env) -> (Vec<u8>, Vec<(i32, i32, Rect)>) {
    let (id, bytes) = imported_tabs(e);
    let nb = e.info(&id);
    let links = doc_links(&id, &nb);
    std::fs::remove_dir_all(format!("{}/notebooks/{id}", e.root)).unwrap();
    std::fs::remove_dir_all(format!("{}/import/done", e.root)).unwrap();
    (bytes, links)
}

// A notebook KOReader made holds only the pages KOReader sent (1 and 3 here) and KOReader's links between them, its
// rectangles a pixel off the worker's (two engines). The same PDF dropped in import/ fills it by its key: pages 2
// and 4 come in book order, the links that reach them are added, and the links KOReader sent are not added again.
#[test]
fn a_koreader_notebook_is_filled_and_no_link_is_doubled() {
    let e = setup("koreader");
    let (bytes, wl) = worker_links(&e);
    let key = key_of(&bytes);
    let shift = |r: Rect| Rect { x: r.x + 1, y: r.y + 1, w: r.w, h: r.h };
    let of = |s: i32, d: i32| wl.iter().find(|l| l.0 == s && l.1 == d).map(|l| shift(l.2)).unwrap();
    // KOReader sent pages 1 and 3, and the links of both: 1 to 3, 3 to 1, and 1 to 4 (a page it does not hold, left out)
    let id = koreader_book(&e, &key, 4, &[1, 3], &[(1, 3, of(1, 3)), (3, 1, of(3, 1)), (1, 4, of(1, 4))]);
    let before = e.info(&id);
    assert_eq!(book_pages(&id, &before), [1, 3]);
    let src_before = e.ini(&id, "source.ini");
    assert!(src_before.lines().any(|l| l == "links=1") && !src_before.lines().any(|l| l.starts_with("file=")), "{src_before}");
    let ko = doc_links(&id, &before);
    assert_eq!(ko.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), [(1, 3), (3, 1)], "{ko:?}");
    ink(&id, 1);
    let files = e.files(&id);
    let (_, ends) = import_all(&e.drop("Tabs.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    assert!(filled(&put_in(), 2));
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4]);
    let links = doc_links(&id, &nb);
    assert_eq!(links.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), [(1, 3), (1, 4), (2, 4), (3, 1), (4, 2)], "{links:?}");
    assert!(no_link_twice(&links), "{links:?}");
    // KOReader's links are the ones that stay, as KOReader drew them
    for k in &ko {
        assert!(links.contains(k), "KOReader's {k:?} is kept as it was: {links:?}");
    }
    for (k, v) in kept(&files, &before) {
        assert_eq!(e.files(&id).get(&k), Some(&v), "{k} is as it was");
    }
    // the record-less notebook stays so; the key stays its own
    let text = e.ini(&id, "source.ini");
    assert!(!text.lines().any(|l| l.starts_with("file=")) && text.lines().any(|l| l == format!("book={key}")), "{text}");
    // the outline names only pages a fill adds: KOReader's pages 1 and 3 keep none (its names are for those two)
    assert_eq!(names(&id, &nb), ["", "", "", ""]);
}

// A fill adds only links that start or end on a page it adds, and names only those pages, so deleting them takes
// everything it wrote. KOReader's boxes 3 px off the worker's (one more than the 2 px rule): the links between the
// pages KOReader sent stay KOReader's alone, none doubled. A KOReader notebook of pages 3 and 4 filled with 1 and 2:
// page 1 takes the outline's name, page 3 (KOReader's) does not.
#[test]
fn a_fill_writes_links_and_names_only_for_the_pages_it_adds() {
    let e = setup("added-only");
    let (bytes, wl) = worker_links(&e);
    let key = key_of(&bytes);
    let off = |s: i32, d: i32| wl.iter().find(|l| l.0 == s && l.1 == d).map(|l| Rect { x: l.2.x + 3, y: l.2.y + 3, w: l.2.w, h: l.2.h }).unwrap();
    let id = koreader_book(&e, &key, 4, &[1, 3], &[(1, 3, off(1, 3)), (3, 1, off(3, 1))]);
    let before = e.info(&id);
    let ko = doc_links(&id, &before);
    let src_before = e.ini(&id, "source.ini");
    let (_, ends) = import_all(&e.drop("Tabs.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    assert!(filled(&put_in(), 2));
    let nb = e.info(&id);
    let links = doc_links(&id, &nb);
    assert_eq!(links.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), [(1, 3), (1, 4), (2, 4), (3, 1), (4, 2)], "{links:?}");
    for k in &ko {
        assert!(links.contains(k), "KOReader's {k:?} is kept as it was: {links:?}");
    }
    // the pages the fill added deleted: the notebook's links, names and links= line are what they were
    drop_pages(&id, &[2, 4]);
    let nb = e.info(&id);
    assert_eq!(nb.page_ids, before.page_ids);
    assert_eq!(doc_links(&id, &nb), ko);
    assert_eq!(names(&id, &nb), ["", ""]);
    let links_line = |t: &str| t.lines().find(|l| l.starts_with("links=")).map(str::to_string);
    assert_eq!(links_line(&e.ini(&id, "source.ini")), links_line(&src_before));
    // names: only the pages it adds
    std::fs::remove_dir_all(format!("{}/notebooks/{id}", e.root)).unwrap();
    std::fs::remove_dir_all(format!("{}/import/done", e.root)).unwrap();
    let id = koreader_book(&e, &key, 4, &[3, 4], &[]);
    let (_, ends) = import_all(&e.drop("Tabs again.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    assert!(filled(&put_in(), 2));
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4]);
    assert_eq!(names(&id, &nb), ["Cover", "", "", ""]);
}

// A notebook that has every page of the book: "PDF already imported.", and nothing of it is written.
#[test]
fn a_notebook_with_every_page_says_already() {
    let e = setup("full");
    let (id, bytes) = imported_tabs(&e);
    let files = e.files(&id);
    let (_, ends) = import_all(&e.drop("Tabs again.pdf", &bytes));
    assert_eq!(ends, [SliceEnd::Already]);
    assert_eq!(e.files(&id), files);
    assert_eq!(e.notebooks().len(), 1);
}

// A notebook whose pages name another page count of the book (KOReader's engine counted five pages of this PDF): the
// PDF, which has four, becomes a notebook of its own, and the first notebook is left as it was.
#[test]
fn a_pdf_of_another_page_count_becomes_a_notebook_of_its_own() {
    let e = setup("count");
    let bytes = pdffix::tabs();
    let key = key_of(&bytes);
    let id = koreader_book(&e, &key, 5, &[1, 3], &[]);
    let files = e.files(&id);
    let (fid, ends) = import_all(&e.drop("Tabs.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 4, blank: 0 }, "{ends:?}");
    assert_eq!(e.files(&id), files, "the first notebook is as it was");
    assert_eq!(e.notebooks().len(), 2);
    // the first keeps the bare key; the new one has it in its record alone
    assert!(!e.ini(&fid, "notebook.ini").lines().any(|l| l.starts_with("source=")));
    assert!(e.ini(&fid, "source.ini").lines().any(|l| l.starts_with("file=sha256:") && l.ends_with(&key)));
}

// ---- the writer makes the pages; the UI puts them in ----

// The writer never writes the notebook: its slices, run to their end here or in another thread (as in the child that
// finishes the queue after Grimoire closed: another store, queue and record), end with the pages made and the
// notebook byte for byte as it was, its work folder kept for the UI. Nothing holds the notebook meanwhile. The UI
// then puts the pages in.
#[test]
fn the_writer_never_writes_the_notebook_and_the_ui_puts_the_pages_in() {
    let e = setup("ui-commits");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let (id, bytes) = imported_tabs(&e);
    drop_pages(&id, &[2, 4]);
    let files = e.files(&id);
    let src = e.drop("Tabs again.pdf", &bytes);
    let s2 = src.clone();
    let fid = std::thread::spawn(move || {
        attach("ui-commits");
        let mut fid = String::new();
        store_make_id(&mut fid);
        let ends = run_on(&fid, &s2, 10);
        assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
        // a slice run again (the queue, a start) ends at once, writing nothing
        assert_eq!(run_slice(&fid, &s2), SliceEnd::Ready { blank: 0 });
        fid
    })
    .join()
    .unwrap();
    assert_eq!(e.files(&id), files, "the writer wrote nothing of the notebook");
    // a fill's state has a name an earlier version never reads, and says the pages are made
    let work = format!("{}/notebooks/.import-{fid}", e.root);
    assert!(!std::path::Path::new(&format!("{work}/pdf-import.ini")).exists());
    let st = e.fill_state(&fid);
    assert_eq!((st.stage, st.todo.clone(), st.fill.clone()), (STAGE_READY, vec![2, 4], id.clone()));
    assert!(keep_work_folder(&fid), "the next start goes on with it");
    assert!(!fill_holds(&id) && fill_busy().is_none(), "nothing holds the notebook");
    assert!(e.in_import("Tabs again.pdf"), "the PDF is filed once its pages are in");
    // the UI, as the Library's tick does
    let f = put_in();
    assert_eq!(f, Finished::Filled { nb: id.clone(), added: 2, blank: 0, trimmed: 0 });
    assert_eq!(book_pages(&id, &e.info(&id)), [1, 2, 3, 4]);
    assert!(e.work_folders().is_empty() && !e.in_import("Tabs again.pdf") && e.done_files() == 2);
}

/// Pages 2 and 4 of the tabs planner made for its notebook (pages 1 and 3 kept), not yet put in: the notebook, its
/// fill's work folder and the PDF's name in import/.
fn made_for(e: &Env, name: &str) -> (String, String, String) {
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "4");
    let (id, bytes) = imported_tabs(e);
    drop_pages(&id, &[2, 4]);
    let src = e.drop(name, &bytes);
    let (fid, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    (id, fid, src)
}

// A notebook renamed, moved or deleted while its fill's pages were made is not filled: nothing of it is written (a
// rename and a move stay as the user made them; a deleted notebook stays in the trash, whole, and is never made
// again), the work folder goes, and the PDF stays in import/, where the next scan takes it again as the file it is:
// here, a fill of the renamed notebook.
#[test]
fn a_notebook_renamed_moved_or_deleted_while_its_pages_were_made_is_not_filled() {
    let e = setup("dropped");
    // (the PDFs left in import/ are not taken by the scans of the first part)
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "hold");
    for what in ["rename", "move", "delete"] {
        for nb in e.notebooks() {
            std::fs::remove_dir_all(format!("{}/notebooks/{}", e.root, nb.id.as_str())).unwrap();
        }
        let name = format!("Tabs {what}.pdf");
        let (id, fid, _) = made_for(&e, &name);
        let pre = e.files(&id);
        match what {
            "rename" => assert_eq!(store_rename(&id, "Renamed by the user"), 0),
            "move" => assert_eq!(store_set_folder(&id, "School"), 0),
            _ => assert_eq!(store_delete(&id), 0),
        }
        let files = e.files(&id);
        let f = put_in();
        assert_eq!(f, Finished::Nothing, "{what}");
        assert_eq!(e.files(&id), files, "{what}: nothing of the notebook was written");
        assert!(!std::path::Path::new(&format!("{}/notebooks/.import-{fid}", e.root)).exists(), "{what}");
        assert!(e.in_import(&name), "{what}: the PDF stays for the next scan");
        match what {
            "rename" => assert_eq!(e.info(&id).name.as_str(), "Renamed by the user"),
            "move" => assert_eq!(e.info(&id).folder.as_str(), "School"),
            _ => {
                assert!(!std::path::Path::new(&format!("{}/notebooks/{id}", e.root)).exists(), "a deleted notebook stays deleted");
                let trash: Vec<String> = std::fs::read_dir(format!("{}/trash", e.root)).unwrap().flatten().map(|d| d.file_name().to_string_lossy().into_owned()).collect();
                let t = trash.iter().find(|t| t.starts_with(&id)).expect("in the trash");
                // the notebook in the trash is the one deleted, file for file
                let held: Vec<String> = std::fs::read_dir(format!("{}/trash/{t}", e.root)).unwrap().flatten().filter(|d| d.file_type().is_ok_and(|f| f.is_file())).map(|d| d.file_name().to_string_lossy().into_owned()).collect();
                assert_eq!(held.len(), pre.keys().filter(|k| !k.starts_with("bg/")).count(), "{held:?}");
                assert_eq!(std::fs::read(format!("{}/trash/{t}/notebook.ini", e.root)).ok().as_ref(), pre.get("notebook.ini"));
            }
        }
    }
    // the renamed notebook's PDF, taken again by the scan: a fill of the notebook as it is now
    for nb in e.notebooks() {
        std::fs::remove_dir_all(format!("{}/notebooks/{}", e.root, nb.id.as_str())).unwrap();
    }
    for what in ["rename", "move", "delete"] {
        std::fs::remove_file(format!("{}/import/Tabs {what}.pdf", e.root)).unwrap();
    }
    let (id, _, _) = made_for(&e, "Tabs again.pdf");
    assert_eq!(store_rename(&id, "Renamed by the user"), 0);
    assert_eq!(put_in(), Finished::Nothing);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    drain();
    let f = commit_all();
    drain();
    assert!(filled(&f, 2), "{f:?}");
    let nb = e.info(&id);
    assert_eq!((nb.name.as_str(), book_pages(&id, &nb)), ("Renamed by the user", vec![1, 2, 3, 4]));
}

// The UI's commit takes a few ticks (here a file a tick). Nothing in the app can act between them (its input waits,
// as for KOReader's book job), and no background job on the notebook starts meanwhile; and should anything write the
// notebook between two ticks, the commit keeps what it wrote: each file is read again and written whole in the tick
// that writes it. A rename, a move and a page copied in (Copy to notebook) stay; a notebook deleted is never made again.
#[test]
fn what_writes_the_notebook_between_two_ticks_of_the_commit_stays() {
    let e = setup("between");
    std::env::set_var("GRIMOIRE_FILL_STEP_PAGES", "1");
    let (id, _, _) = made_for(&e, "Tabs again.pdf");
    let mut other = NotebookInfo::default();
    assert_eq!(store_create("Other", "", PAPER_BLANK, &mut other), 0);
    ink(other.id.as_str(), 0);
    start_again();
    assert_eq!(fill_tick(true, DPI), Finished::Nothing);
    assert_eq!(fill_busy(), Some((0, 2)), "the commit is under way");
    assert!(fill_holds(&id) && !fill_holds(other.id.as_str()));
    // a job on the notebook (its PDF mirror) waits for the commit's end
    let mirror = format!("{}/mirror", e.root);
    bg_queue(BgKind::MirrorPdf, Some(&id), Some(&mirror), DPI, false);
    assert_eq!(bg_children(), 0, "no job on the notebook starts while its commit runs");
    assert_eq!(store_rename(&id, "Renamed by the user"), 0);
    assert_eq!(fill_tick(true, DPI), Finished::Nothing);
    assert_eq!(store_set_folder(&id, "School"), 0);
    let mut dst = e.info(&id);
    let mut out = Vec::new();
    assert_eq!(store_copy_pages_to(&other, &[other.page_ids[0]], &mut dst, 0, &mut out), 0);
    let copied = out[0].to;
    let f = commit_all();
    assert!(filled(&f, 2), "{f:?}");
    assert!(!fill_holds(&id) && fill_busy().is_none());
    let nb = e.info(&id);
    assert_eq!((nb.name.as_str(), nb.folder.as_str()), ("Renamed by the user", "School"));
    assert!(nb.page_ids.contains(&copied), "the page copied in is listed");
    assert!(std::path::Path::new(&format!("{}/notebooks/{id}/p{copied}.qpg", e.root)).exists());
    assert_eq!(book_pages(&id, &nb), [0, 1, 2, 3, 4]);
    drain();
    // a notebook deleted between two ticks: the commit ends there, and nothing makes it again
    for nb in e.notebooks() {
        std::fs::remove_dir_all(format!("{}/notebooks/{}", e.root, nb.id.as_str())).unwrap();
    }
    let (id, fid, _) = made_for(&e, "Tabs once more.pdf");
    std::env::set_var("GRIMOIRE_FILL_STEP_PAGES", "1");
    start_again();
    assert_eq!(fill_tick(true, DPI), Finished::Nothing);
    assert!(fill_busy().is_some());
    assert_eq!(store_delete(&id), 0);
    assert_eq!(commit_all(), Finished::Nothing);
    assert!(!std::path::Path::new(&format!("{}/notebooks/{id}", e.root)).exists(), "a deleted notebook stays deleted");
    assert!(!e.notebooks().iter().any(|n| n.id.as_str() == id));
    assert!(!std::path::Path::new(&format!("{}/notebooks/.import-{fid}", e.root)).exists());
}

// A fill whose worker cannot be started (the sandbox or the exec refused) keeps its work folder for the next start,
// as an import does, and holds nothing: its notebook opens (nothing but a commit under way holds one). Once a worker
// can start again the fill goes on and its pages go in.
#[test]
fn a_fill_whose_worker_cannot_start_holds_nothing() {
    let e = setup("no-worker");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let (id, bytes) = imported_tabs(&e);
    drop_pages(&id, &[2, 4]);
    let src = e.drop("Tabs again.pdf", &bytes);
    let mut fid = String::new();
    store_make_id(&mut fid);
    assert_eq!(run_slice(&fid, &src), SliceEnd::More, "the probe");
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire-worker");
    assert_eq!(run_slice(&fid, &src), SliceEnd::Sandbox);
    assert_eq!(e.work_folders().len(), 1, "its pages made are kept for the next start");
    assert!(!fill_holds(&id) && fill_busy().is_none(), "the notebook opens");
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    let ends = run_on(&fid, &src, 10);
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    assert!(filled(&put_in(), 2));
    assert_eq!(book_pages(&id, &e.info(&id)), [1, 2, 3, 4]);
}

// KOReader's "Open this book in Grimoire" while a fill of the book's notebook is under way: KOReader's request wins.
// Its job adds the page it sent as it does on main (nothing waits for the fill), and the fill waits, with no job
// queued and no scan made due at every tick; once KOReader's job has ended (and its notebook is closed: the
// Library in front), the fill adds only what the notebook still lacks, the page KOReader added meanwhile its own.
#[test]
fn koreaders_request_wins_over_a_fill_and_the_fill_adds_only_what_is_still_missing() {
    let e = setup("koreader-meanwhile");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    let bytes = pdffix::tabs();
    let key = key_of(&bytes);
    let id = koreader_book(&e, &key, 4, &[1, 3], &[]);
    e.drop("Tabs.pdf", &bytes);
    // KOReader hands book page 2 over: its job starts
    let mut st = koreader_request(&e, &key, 4, &[2], &[]);
    assert_eq!(st.state, BookState::Running);
    // the fill's slices meanwhile: its pages are made, and it waits for the UI, due for nothing
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    let told = drain();
    assert_eq!(told, ["Nothing"], "the pages are made");
    assert!(!scan_due(now_ms(), false), "no scan is due at once");
    for _ in 0..5 {
        // (app.rs: no commit begins while a book job runs)
        assert_eq!(fill_tick(false, DPI), Finished::Nothing);
        assert!(fill_busy().is_none() && !bg_busy(), "nothing runs or is queued while it waits");
    }
    for _ in 0..100 {
        if st.state != BookState::Running {
            break;
        }
        book_step(150, &mut st);
    }
    assert_eq!((st.state, st.added), (BookState::Done, 1), "KOReader's job ran as without a fill");
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3]);
    let page2 = nb.page_ids[1];
    let f = commit_all();
    drain();
    assert!(filled(&f, 1), "{f:?}");
    let nb = e.info(&id);
    assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4]);
    assert_eq!(nb.page_ids[1], page2, "book page 2 is KOReader's");
}

// ---- stops, cuts and a PDF that goes ----

// A fill stopped in the middle (the run before ended after the probe and one page) goes on at the next start's scan,
// from its work folder, and fills the notebook.
#[test]
fn a_fill_goes_on_after_a_stop_in_the_middle() {
    let e = setup("resume");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let (id, bytes) = imported_tabs(&e);
    drop_pages(&id, &[1, 2, 4]);
    let src = e.drop("Tabs again.pdf", &bytes);
    // the run before: the probe and one page (another thread: its own store, queue and record)
    let s2 = src.clone();
    std::thread::spawn(move || {
        attach("resume");
        let mut fid = String::new();
        store_make_id(&mut fid);
        assert!(mark_write_public(&Mark { state: MarkState::Making, id: fid.clone(), src: s2.clone(), ..Default::default() }));
        assert_eq!(run_slice(&fid, &s2), SliceEnd::More);
        assert_eq!(run_slice(&fid, &s2), SliceEnd::More);
    })
    .join()
    .unwrap();
    assert_eq!(e.work_folders().len(), 1);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    let told = drain();
    assert_eq!(told, ["Nothing"], "its pages are made");
    let f = commit_all();
    drain();
    assert!(filled(&f, 3), "{f:?}");
    assert_eq!(book_pages(&id, &e.info(&id)), [1, 2, 3, 4]);
    assert!(e.work_folders().is_empty());
}

// A fill cancelled before its commit (its PDF went away after a page was made) leaves the notebook as it was.
#[test]
fn a_cancelled_fill_leaves_the_notebook_as_it_was() {
    let e = setup("cancel");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let (id, bytes) = imported_tabs(&e);
    drop_pages(&id, &[2, 3, 4]);
    let files = e.files(&id);
    let src = e.drop("Tabs again.pdf", &bytes);
    let mut fid = String::new();
    store_make_id(&mut fid);
    assert_eq!(run_slice(&fid, &src), SliceEnd::More);
    assert_eq!(run_slice(&fid, &src), SliceEnd::More);
    std::fs::remove_file(format!("{}/import/Tabs again.pdf", e.root)).unwrap();
    assert!(matches!(run_slice(&fid, &src), SliceEnd::Failed(..)));
    assert!(e.work_folders().is_empty());
    assert_eq!(e.files(&id), files, "the notebook is as it was");
}

// A commit cut off after any of its steps (Grimoire killed there) is taken up again at the next start and fills the
// notebook, with the page ids it gave the first time (no page file left over); until its notebook.ini is written
// (step 5) the notebook lists the pages it had, and none of what it shows has changed. The PDF may even be gone by
// then: a commit that has begun goes on from the work folder alone.
#[test]
fn a_commit_cut_off_at_any_step_ends_with_the_notebook_filled() {
    let e = setup("cut");
    let (bytes, wl) = worker_links(&e);
    let key = key_of(&bytes);
    let of = |s: i32, d: i32| wl.iter().find(|l| l.0 == s && l.1 == d).map(|l| l.2).unwrap();
    for step in 1..=7u32 {
        // a fresh notebook from KOReader for each step (pages 1 and 3, its own pictures), the last one gone
        for nb in e.notebooks() {
            std::fs::remove_dir_all(format!("{}/notebooks/{}", e.root, nb.id.as_str())).unwrap();
        }
        let id = koreader_book(&e, &key, 4, &[1, 3], &[(1, 3, of(1, 3)), (3, 1, of(3, 1))]);
        let pristine = e.files(&id);
        let before = e.info(&id);
        let src = e.drop(&format!("Tabs {step}.pdf"), &bytes);
        let (fid, ends) = import_all(&src);
        assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "step {step}");
        sim_fill_cut(step);
        start_again();
        assert_eq!(commit_all(), Finished::Nothing, "step {step}: cut off");
        let ids = e.fill_state(&fid).ids;
        assert_eq!(ids.len(), 2, "step {step}: the ids it gave are kept");
        let nb = e.info(&id);
        if step <= 4 {
            assert_eq!(nb.page_ids, before.page_ids, "step {step}: the notebook lists the pages it had");
            assert_eq!(names(&id, &nb), ["", ""], "step {step}");
            assert_eq!(doc_links(&id, &nb), doc_links(&id, &before), "step {step}");
            for (k, v) in kept(&pristine, &before) {
                assert_eq!(e.files(&id).get(&k), Some(&v), "step {step}: {k}");
            }
        } else {
            assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4], "step {step}");
        }
        // the PDF goes, and the commit is taken up again all the same
        if step % 2 == 1 {
            std::fs::remove_file(format!("{}/import/Tabs {step}.pdf", e.root)).unwrap();
        }
        let f = put_in();
        assert!(filled(&f, 2), "step {step}: {f:?}");
        let nb = e.info(&id);
        assert_eq!(book_pages(&id, &nb), [1, 2, 3, 4], "step {step}");
        assert!(ids.iter().all(|x| nb.page_ids.contains(&x.1)), "step {step}: the same ids");
        assert_eq!(e.page_files(&id), 4, "step {step}: no page file left over");
        let links = doc_links(&id, &nb);
        assert_eq!(links.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), [(1, 3), (1, 4), (2, 4), (3, 1), (4, 2)], "step {step}: {links:?}");
        assert!(no_link_twice(&links), "step {step}: {links:?}");
        assert_eq!(names(&id, &nb), ["", "", "", ""], "step {step}: KOReader's pages keep none");
        assert!(e.work_folders().is_empty(), "step {step}");
        for (k, v) in kept(&pristine, &before) {
            assert_eq!(e.files(&id).get(&k), Some(&v), "step {step}: {k}");
        }
        // every page's picture is there
        for i in 0..4 {
            let mut pg = None;
            assert_eq!(store_load_page(&nb, i, PW, PH, &mut pg), 0, "step {step}: page {i}");
            let paper = pg.unwrap().paper;
            assert!(paper_is_document(paper));
            assert!(std::path::Path::new(&format!("{}/notebooks/{id}/bg/{:06x}.png", e.root, paper_image_id(paper))).exists(), "step {step}: page {i}'s picture");
        }
    }
}

// A commit cut off after its notebook.ini, when the book's page 1 became the notebook's first page, still gives the
// card that page's picture when it is taken up again.
#[test]
fn a_commit_cut_after_its_notebook_ini_still_gives_the_card_its_picture() {
    let e = setup("cut-thumb");
    let (id, bytes) = imported_tabs(&e);
    drop_pages(&id, &[1, 2]);
    let thumb = format!("{}/notebooks/{id}/thumb.pgm", e.root);
    let _ = std::fs::remove_file(&thumb);
    let (_, ends) = import_all(&e.drop("Tabs again.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
    sim_fill_cut(5);
    start_again();
    assert_eq!(commit_all(), Finished::Nothing, "cut off");
    assert_eq!(book_pages(&id, &e.info(&id)), [1, 2, 3, 4], "the pages are listed");
    assert!(!std::path::Path::new(&thumb).exists());
    assert!(filled(&put_in(), 2));
    assert!(std::path::Path::new(&thumb).exists(), "the card's picture is the book's page 1");
}

// A fill whose notebook no longer holds the PDF's key by the time its pages go in (a state or a notebook changed by
// hand) is not committed: nothing of the notebook is written.
#[test]
fn a_notebook_that_no_longer_holds_the_key_is_not_filled() {
    let e = setup("key-gone");
    let (id, fid, _) = made_for(&e, "Tabs again.pdf");
    let key = e.fill_state(&fid).key;
    let ini = format!("{}/notebooks/{id}/notebook.ini", e.root);
    let text = std::fs::read_to_string(&ini).unwrap();
    assert!(text.contains(&format!("source={key}")), "{text}");
    std::fs::write(&ini, text.replace(&format!("source={key}"), "source=00000000000000000000000000000000")).unwrap();
    let path = format!("{}/notebooks/{id}/source.ini", e.root);
    let src: String = std::fs::read_to_string(&path).unwrap().lines().filter(|l| !l.starts_with("file=") && !l.starts_with("book=")).map(|l| format!("{l}\n")).collect();
    std::fs::write(&path, &src).unwrap();
    let files = e.files(&id);
    assert_eq!(put_in(), Finished::Nothing);
    assert_eq!(e.files(&id), files, "nothing of the notebook was written");
    assert!(e.work_folders().is_empty());
}

// A commit cut off three times (Grimoire killed there at each start) ends at the third stop: its work folder goes,
// and the notebook is as the cuts left it (here, before its notebook.ini: as it was).
#[test]
fn a_commit_cut_off_three_times_ends() {
    let e = setup("cut-thrice");
    let (id, _, _) = made_for(&e, "Tabs again.pdf");
    let before = e.info(&id);
    for _ in 0..3 {
        sim_fill_cut(2);
        start_again();
        assert_eq!(commit_all(), Finished::Nothing, "cut off");
    }
    sim_fill_cut(0);
    start_again();
    assert_eq!(commit_all(), Finished::Failed(FailWhy::Generic), "the third stop ends it");
    assert!(e.work_folders().is_empty());
    assert_eq!(e.info(&id).page_ids, before.page_ids);
}

// ---- the queue ----

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
        }
        if !bg_busy() {
            break;
        }
        assert!(t0.elapsed().as_secs() < 120, "the jobs never ended");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    said
}

// The scenario as the owner meets it: a PDF dropped in import/ became a notebook; some of its pages were deleted;
// the same PDF dropped again fills them, and the second copy dropped with it says "PDF already imported." once the
// fill is in (two fills of one notebook never run: the copy waits for the first).
#[test]
fn a_pdf_dropped_again_fills_its_notebook_and_a_second_copy_says_already() {
    let e = setup("queue");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let bytes = pdffix::tabs();
    e.drop("Tabs.pdf", &bytes);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    let told = drain();
    assert!(told.iter().any(|t| t.starts_with("Imported")), "{told:?}");
    let id = e.notebooks()[0].id.as_str().to_string();
    drop_pages(&id, &[2, 3]);
    e.drop("Tabs again.pdf", &bytes);
    e.drop("Tabs copy.pdf", &bytes);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 2);
    let mut told = drain();
    for _ in 0..4 {
        let f = commit_all();
        if f != Finished::Nothing {
            told.push(format!("{f:?}"));
        }
        told.extend(drain());
        grimoire::bgwork_template::import_scan(DPI);
        told.extend(drain());
    }
    assert_eq!(told.iter().filter(|t| t.starts_with("Filled")).count(), 1, "{told:?}");
    assert!(told.iter().any(|t| t.contains(&format!("nb: \"{id}\"")) && t.contains("added: 2")), "{told:?}");
    assert!(told.iter().any(|t| t == "Already"), "{told:?}");
    assert_eq!(e.notebooks().len(), 1);
    assert_eq!(book_pages(&id, &e.info(&id)), [1, 2, 3, 4]);
    assert_eq!(e.done_files(), 3);
}

// ---- the water level keeps a notebook's own links ----

// A fill over the water level trims only links it adds: the links the notebook had all stay, though their page is the
// densest.
#[test]
fn the_water_level_of_a_fill_trims_only_the_links_it_adds() {
    // 14,200 links the notebook has: 1,000 on page 1 (the densest), 33 on each of pages 2 to 401
    let mut had_links = DocLinks::default();
    for p in 1..=401u32 {
        for k in 0..(if p == 1 { 1000u32 } else { 33 }) {
            let area = Rect { x: (k % 40) as i32 * 12, y: (k / 40) as i32 * 12, w: 10, h: 10 };
            assert!(had_links.push(DocLink::new(p, 1000 + k, if p == 1 { 2 } else { 1 }, area).unwrap(), None));
        }
    }
    let had: std::collections::HashSet<(u32, u32)> = had_links.links.iter().map(|l| (l.pid, l.seed)).collect();
    // a fill adds one link on each of pages 2 to 401: 14,600, past the 14,400 of the water level
    let recs: Vec<grimoire::pdflinks::DocRecord> =
        (2..=401u32).map(|p| grimoire::pdflinks::DocRecord { src: p, dst: if p == 3 { 4 } else { 3 }, rect: Rect { x: 500, y: 500, w: 20, h: 20 }, quad: None, index: 100 }).collect();
    let mut dl = had_links.clone();
    let st = pdflinks_fill(&mut dl, &recs, PW, PH, &|p| (1..=401).contains(&p), &|_, _| false);
    assert_eq!(st.added, 400);
    let mut plain = dl.clone();
    let gone = pdflinks_water_keeping(&mut dl, &|l| had.contains(&(l.pid, l.seed)));
    assert_eq!((gone, dl.links.len()), (200, 14_400), "only links the fill added go");
    assert!(had.iter().all(|h| dl.links.iter().any(|l| (l.pid, l.seed) == *h)), "every link the notebook had stays");
    // the plain water level (an import's, a new notebook's) would trim the densest page: the notebook's own links
    assert!(pdflinks_water(&mut plain) > 0);
    assert!(!had.iter().all(|h| plain.links.iter().any(|l| (l.pid, l.seed) == *h)), "the plain level takes links page 1 had");
}

// The state of a fill reads back as it was written, and is refused when its lists do not fit together.
#[test]
fn a_fills_state_reads_back_strictly() {
    let s = State {
        src: "grimoire/import/Tabs.pdf".into(),
        size: 10,
        mtime: 20,
        key: "0123456789abcdef0123456789abcdef".into(),
        pages: 4,
        next: 2,
        pics: vec![0x12],
        fill: "20261115-120000-ab12".into(),
        todo: vec![2, 4],
        page_w: 600,
        page_h: 800,
        sig: 0xfeed,
        ..Default::default()
    };
    let t = s.text();
    assert_eq!(State::parse(t.as_bytes()), Some(s.clone()));
    for (from, to) in [
        ("todo=2,4", "todo=4,2"),
        ("todo=2,4", "todo=2,5"),
        ("todo=2,4", "todo="),
        ("next=2", "next=4"),
        ("fill=20261115-120000-ab12", "fill=../x"),
        // a page size the store does not take, or one side alone
        ("page_w=600", "page_w=50"),
        ("page_h=800", "page_h=60000"),
        ("page_h=800", "page_h=0"),
        ("stage=0", "stage=4"),
        // pages made only once every page of todo is
        ("stage=0", "stage=1"),
    ] {
        assert_eq!(State::parse(t.replace(from, to).as_bytes()), None, "{to}");
    }
    // a commit begun: every page made, each page id given once to a page of todo
    let done = State { next: 3, pics: vec![0x12, 0x13], stage: STAGE_COMMIT, ids: vec![(2, 7), (4, 8)], ..s.clone() };
    let dt = done.text();
    assert_eq!(State::parse(dt.as_bytes()), Some(done.clone()));
    for (from, to) in [("ids=2:7,4:8", "ids=2:7,3:8"), ("ids=2:7,4:8", "ids=2:7,4:7"), ("ids=2:7,4:8", "ids=2:7,2:8"), ("ids=2:7,4:8", "ids=2:0,4:8")] {
        assert_eq!(State::parse(dt.replace(from, to).as_bytes()), None, "{to}");
    }
    assert_eq!(State::parse(State { next: 2, pics: vec![0x12], ..done }.text().as_bytes()), None);
    // an import's state has no fill lines
    let plain = State { fill: String::new(), todo: Vec::new(), page_w: 0, page_h: 0, sig: 0, ..s };
    assert!(!plain.text().contains("fill="));
    assert_eq!(State::parse(plain.text().as_bytes()), Some(plain.clone()));
}
