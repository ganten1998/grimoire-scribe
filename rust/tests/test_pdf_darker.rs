// test_pdf_darker.rs: darker lines in imported PDFs (2.2, issue #173). The darker lines setting (settings.ini
// pdf_gamma: Off, 1.5, 2.0) is the gamma a PDF queued from then on draws its gray pages at; the UI writes the level
// into the import's mark as it queues the file (the setting, or a level of the caller's own: KOReader's request), the
// first slice keeps it in the import's state, every slice draws at it, and the notebook keeps it in its source.ini
// (`gamma=`, a line only when it is not Off). A page kept in colour is never changed. A fill draws the pages it adds
// at its notebook's own gamma, whatever the setting says; the same PDF again with every page is "already imported".
//
// The writer runs in this process (pdfimport::run_slice, every page through a real worker, a fresh exec of the
// headless runner), as in test_pdfimport.rs; the tests of the queue drive the jobs as the app does (bg_poll_job,
// then pdfimport::finished), as test_import_queue.rs does. Generated PDFs only (tests/pdffix).
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::fsview::StoragePath;
use grimoire::pdfimport::*;
use grimoire::pdfwork::Gamma;
use grimoire::prelude::*;
use std::os::fd::AsRawFd;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
const DPI: f32 = 300.0;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

const HOOKS: [&str; 6] = ["GRIMOIRE_PDF_SLICE_PAGES", "GRIMOIRE_PDF_BANDS", "GRIMOIRE_IMPORT_STABLE", "GRIMOIRE_SANDBOX", "GRIMOIRE_BG_DELAY_MS", "GRIMOIRE_FILL_STEP_PAGES"];

struct Env {
    /// Grimoire's own folder (the storage root is its parent)
    root: String,
    storage: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn storage_of(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join(format!("grimoire-pdf-darker-{}-{name}", std::process::id()))
}

/// This thread's store on the storage `name` (the store, the queue and the import's UI side are per thread).
fn attach(name: &str) -> (String, String) {
    let storage = storage_of(name);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::fs::create_dir_all(storage.join("documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(DPI, PW, PH);
    ui_reset();
    (root.to_str().unwrap().to_string(), storage.to_str().unwrap().to_string())
}

fn setup(name: &str) -> Env {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in HOOKS {
        std::env::remove_var(h);
    }
    set_darker(Gamma::Off);
    let _ = std::fs::remove_dir_all(storage_of(name));
    let (root, storage) = attach(name);
    Env { root, storage, _g: g }
}

impl Env {
    /// Moves this thread's store to another storage `name` (made afresh), keeping the test's lock.
    fn switch(&mut self, name: &str) {
        let _ = std::fs::remove_dir_all(storage_of(name));
        let (root, storage) = attach(name);
        self.root = root;
        self.storage = storage;
    }
    /// Puts a PDF in import/ and returns its StoragePath text.
    fn drop(&self, name: &str, bytes: &[u8]) -> String {
        std::fs::write(format!("{}/import/{name}", self.root), bytes).unwrap();
        format!("grimoire/import/{name}")
    }
    fn put(&self, rel: &str, bytes: &[u8]) {
        let p = format!("{}/{rel}", self.storage);
        std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    fn notebooks(&self) -> Vec<NotebookInfo> {
        let mut v = Vec::new();
        store_list(&mut v);
        v
    }
    fn only_notebook(&self) -> String {
        let v = self.notebooks();
        assert_eq!(v.len(), 1, "one notebook");
        v[0].id.as_str().to_string()
    }
    fn source_ini(&self, id: &str) -> String {
        std::fs::read_to_string(format!("{}/notebooks/{id}/source.ini", self.root)).unwrap_or_default()
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

/// The `gamma=` lines of a notebook's source.ini.
fn gamma_lines(ini: &str) -> Vec<&str> {
    ini.lines().filter(|l| l.starts_with("gamma")).collect()
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

/// The pages a fill made (the slices run in this process) put in by the UI as app.rs's tick does, and its PDF filed
/// by the slice the commit queues: what the commit said.
fn put_in() -> Finished {
    ui_reset();
    grimoire::bgwork_template::import_scan(DPI);
    let mut f = Finished::Nothing;
    for _ in 0..100_000 {
        f = fill_tick(true, DPI);
        if f != Finished::Nothing || fill_busy().is_none() {
            break;
        }
    }
    drain();
    f
}

/// A notebook page's picture, as its file holds it: (channels, its pixels decoded by the png crate, the file's bytes).
fn picture(root: &str, id: &str, index: usize) -> (u32, Vec<u8>, Vec<u8>) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let mut pgbox = None;
    assert_eq!(store_load_page(&nb, index as i32, PW, PH, &mut pgbox), 0, "the page file reads");
    let paper = pgbox.as_ref().unwrap().paper;
    assert!(paper_is_document(paper));
    let path = format!("{root}/notebooks/{id}/bg/{:06x}.png", paper_image_id(paper));
    let bytes = std::fs::read(&path).unwrap();
    let dec = png::Decoder::new(std::io::Cursor::new(bytes.clone()));
    let mut r = dec.read_info().unwrap();
    let ch = r.info().color_type.samples() as u32;
    let mut buf = vec![0u8; r.output_buffer_size().unwrap()];
    let f = r.next_frame(&mut buf).unwrap();
    buf.truncate(f.buffer_size());
    assert_eq!((f.width, f.height), (PW as u32, PH as u32));
    (ch, buf, bytes)
}

/// Every page of notebook `id`, in its order.
fn pictures(root: &str, id: &str) -> Vec<(u32, Vec<u8>, Vec<u8>)> {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    (0..nb.npages() as usize).map(|i| picture(root, id, i)).collect()
}

/// `off`'s pixels through `g`'s table.
fn through(off: &[u8], g: Gamma) -> Vec<u8> {
    let t = g.table();
    off.iter().map(|&v| t[v as usize]).collect()
}

/// A planner imported whole, at level `g`, on a storage of its own: its pictures and its source.ini.
fn planner_at(e: &mut Env, name: &str, g: Gamma, bytes: &[u8]) -> (Vec<(u32, Vec<u8>, Vec<u8>)>, String) {
    e.switch(name);
    set_darker(g);
    let (id, ends) = import_all(&e.drop("Planner.pdf", bytes));
    assert!(matches!(last(&ends), SliceEnd::Done { blank: 0, .. }), "{ends:?}");
    (pictures(&e.root, &id), e.source_ini(&id))
}

// Each level, on the same planner: every pixel of a gray page at 1.5 or 2.0 is that pixel at Off through the level's
// table (the gamma goes over the gray page the writer already holds, and nothing else changes); the notebook keeps
// the level in its source.ini, and a notebook made at Off has no such line. The pages and source.ini at Off are the
// import's as before.
#[test]
fn each_level_draws_the_gray_pages_through_its_gamma_and_the_notebook_keeps_it() {
    let mut e = setup("levels");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    let bytes = pdffix::planner(3);
    let (off, off_ini) = planner_at(&mut e, "levels-off", Gamma::Off, &bytes);
    assert!(gamma_lines(&off_ini).is_empty(), "no gamma line at Off:\n{off_ini}");
    assert!(off.iter().all(|p| p.0 == 1), "the planner's pages are gray");
    for (g, line) in [(Gamma::Medium, "gamma=1.5"), (Gamma::Strong, "gamma=2.0")] {
        let (pics, ini) = planner_at(&mut e, &format!("levels-{}", g.name()), g, &bytes);
        assert_eq!(gamma_lines(&ini), [line], "{ini}");
        // the record and the page lines are as at Off: the gamma line rides after the record
        let rest = |t: &str| t.lines().filter(|l| !l.starts_with("gamma=") && !l.starts_with("file=") && !l.starts_with("book=")).map(str::to_string).collect::<Vec<_>>();
        assert_eq!(rest(&ini), rest(&off_ini));
        for (k, (p, o)) in pics.iter().zip(&off).enumerate() {
            assert_eq!(p.0, 1, "page {} stays gray", k + 1);
            assert!(p.1 == through(&o.1, g), "page {}: every pixel is its level at Off through gamma {}", k + 1, g.name());
            assert!(p.1 != o.1, "page {} is darker", k + 1);
        }
    }
    set_darker(Gamma::Off);
}

// A colour page (the second of pdffix::colour: a red box and a pale yellow band on a planner page) is stored as it is,
// its picture's file byte for byte, at every level; the gray pages beside it go darker.
#[test]
fn a_colour_page_is_left_exactly_as_it_is() {
    let mut e = setup("colour");
    let bytes = pdffix::colour();
    let (off, _) = planner_at(&mut e, "colour-off", Gamma::Off, &bytes);
    assert_eq!(off.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 3, 1], "gray, colour, gray");
    let (strong, ini) = planner_at(&mut e, "colour-strong", Gamma::Strong, &bytes);
    assert_eq!(gamma_lines(&ini), ["gamma=2.0"]);
    assert_eq!(strong[1].0, 3);
    assert!(strong[1].2 == off[1].2, "the colour page's picture is the same file");
    for k in [0, 2] {
        assert!(strong[k].1 == through(&off[k].1, Gamma::Strong), "gray page {} is darker", k + 1);
    }
    set_darker(Gamma::Off);
}

/// The mark of the file at `src`, as its file holds it.
fn mark_of(e: &Env, src: &str) -> String {
    let line = format!("src={src}");
    e.marks().into_iter().find(|m| m.lines().any(|l| l == line)).unwrap_or_else(|| panic!("no mark for {src}"))
}

// The level is the one the UI queued the file at, carried by its mark: a pick at the setting, a pick at a level of its
// own (KOReader's request) whatever the setting, and a pick that waits for the lock keeps its level (its own, or the
// setting's as it was then) until a later scan queues it, though the setting changed meanwhile. A file of import/ is
// queued at the setting the scan sees.
#[test]
fn the_mark_carries_the_level_the_file_was_queued_at() {
    let e = setup("marks");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    e.put("documents/One.pdf", &pdffix::planner(1));
    e.put("documents/Two.pdf", &pdffix::planner(2));
    e.put("documents/Three.pdf", &pdffix::planner(3));
    e.put("documents/Five.pdf", &pdffix::planner(5));
    let level = |name: &str| {
        let nb = e.notebooks().into_iter().find(|n| n.name.as_str() == name).unwrap_or_else(|| panic!("{name}"));
        gamma_among(e.source_ini(nb.id.as_str()).as_bytes())
    };
    // the setting
    set_darker(Gamma::Medium);
    let q = pdf_import_queue(&StoragePath::new(b"documents/One.pdf").unwrap(), None, None, DPI);
    assert!(matches!(q, Queued::Started(_)), "{q:?}");
    assert!(e.marks().iter().any(|m| m.lines().any(|l| l == "gamma=1.5")), "{:?}", e.marks());
    drain();
    assert_eq!(level("One"), Gamma::Medium);
    // a level of its own, the setting another
    let q = pdf_import_queue_at(&StoragePath::new(b"documents/Two.pdf").unwrap(), None, None, Some(Gamma::Strong), DPI);
    assert!(matches!(q, Queued::Started(_)), "{q:?}");
    drain();
    assert_eq!(level("Two"), Gamma::Strong);
    // two picks wait for the lock while the setting is Medium: one at KOReader's level Off, one at the setting
    lock_release_if_idle();
    let other = e.hold_lock();
    let q = pdf_import_queue_at(&StoragePath::new(b"documents/Three.pdf").unwrap(), None, None, Some(Gamma::Off), DPI);
    assert_eq!(q, Queued::Waiting);
    let q = pdf_import_queue(&StoragePath::new(b"documents/Five.pdf").unwrap(), None, None, DPI);
    assert_eq!(q, Queued::Waiting);
    let three = mark_of(&e, "documents/Three.pdf");
    assert!(three.contains("state=waiting") && !three.contains("gamma="), "a waiting mark at Off says none: {three}");
    let five = mark_of(&e, "documents/Five.pdf");
    assert!(five.contains("state=waiting") && five.lines().any(|l| l == "gamma=1.5"), "a waiting mark keeps the setting's level: {five}");
    drop(other);
    set_darker(Gamma::Strong);
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 2);
    drain();
    assert_eq!(level("Three"), Gamma::Off, "the level it waited with, not the setting of the scan");
    assert_eq!(level("Five"), Gamma::Medium, "the level it waited with, not the setting of the scan");
    // a file of import/: the scan's setting
    e.drop("Four.pdf", &pdffix::planner(4));
    assert_eq!(grimoire::bgwork_template::import_scan(DPI), 1);
    drain();
    assert_eq!(level("Four"), Gamma::Strong);
    set_darker(Gamma::Off);
}

// A pick that finds a copy of its PDF being imported waits for that copy's notebook (H9): its mark turns back to
// waiting with the level it was queued at. Taken again once the copy's import is gone (here its work folder removed
// before it ended), it makes its own notebook at that level, whatever the setting says.
#[test]
fn a_pick_that_waits_for_a_copy_keeps_its_level() {
    let e = setup("deferred");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let bytes = pdffix::planner(3);
    let first = e.drop("First.pdf", &bytes);
    let mut ida = String::new();
    store_make_id(&mut ida);
    assert_eq!(run_slice(&ida, &first), SliceEnd::More, "the first copy is being imported");
    // the second copy, picked at Strong (its mark as the queue writes it), the setting Off
    let second = "documents/Second.pdf";
    e.put(second, &bytes);
    assert!(mark_write_public(&Mark { state: MarkState::Making, src: second.into(), pick: true, gamma: Gamma::Strong, ..Default::default() }));
    let (_, ends) = import_all(second);
    assert_eq!(ends, [SliceEnd::Deferred]);
    let m = mark_of(&e, second);
    assert!(m.contains("state=waiting") && m.lines().any(|l| l == "gamma=2.0"), "the waiting mark keeps its level: {m}");
    std::fs::remove_dir_all(format!("{}/notebooks/.import-{ida}", e.root)).unwrap();
    let (id, ends) = import_all(second);
    assert!(matches!(last(&ends), SliceEnd::Done { blank: 0, .. }), "{ends:?}");
    assert_eq!(gamma_lines(&e.source_ini(&id)), ["gamma=2.0"], "drawn at the level it waited with");
}

/// Takes the pages of book pages `book` out of notebook `id`'s list (as a delete in the Pages view does).
fn drop_pages(id: &str, book: &[i32]) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let mut m = AnnotateMap::default();
    assert_eq!(annotate_map_load(id, &mut m), 0);
    let keep: Vec<u32> = nb.page_ids.iter().copied().filter(|&pid| !m.e.iter().any(|e| e.pid == pid && book.contains(&e.page))).collect();
    nb.page_ids = keep;
    nb.last_page = 0;
    assert_eq!(store_save_info(&mut nb), 0);
}

// A fill (2.2, #123) draws the pages it adds at its notebook's own gamma, whatever the setting says: a notebook made
// at 1.5 gets its two missing pages at 1.5 while the setting is 2.0, and one made at Off gets them at Off while the
// setting is 1.5, byte for byte the pages a whole import at Off makes. The notebook's gamma line stays as it was.
#[test]
fn a_fill_draws_the_pages_it_adds_at_its_notebooks_gamma() {
    let mut e = setup("fill");
    let bytes = pdffix::tabs();
    let (reference, _) = planner_at(&mut e, "fill-ref", Gamma::Off, &bytes);
    for (made, setting) in [(Gamma::Medium, Gamma::Strong), (Gamma::Off, Gamma::Medium)] {
        e.switch(&format!("fill-{}", made.name()));
        set_darker(made);
        let (id, ends) = import_all(&e.drop("Tabs.pdf", &bytes));
        assert_eq!(last(&ends), &SliceEnd::Done { pages: 4, blank: 0 }, "{ends:?}");
        let before = e.source_ini(&id);
        drop_pages(&id, &[2, 4]);
        set_darker(setting);
        let (_, ends) = import_all(&e.drop("Tabs again.pdf", &bytes));
        assert_eq!(last(&ends), &SliceEnd::Ready { blank: 0 }, "{ends:?}");
        let f = put_in();
        assert!(matches!(f, Finished::Filled { added: 2, blank: 0, .. }), "{f:?}");
        let pics = pictures(&e.root, &id);
        assert_eq!(pics.len(), 4);
        for k in 0..4 {
            assert!(pics[k].1 == through(&reference[k].1, made), "page {} at its notebook's gamma {}", k + 1, made.name());
        }
        assert_eq!(gamma_lines(&e.source_ini(&id)), gamma_lines(&before), "the notebook's gamma line stays as it was");
        assert_eq!(e.only_notebook(), id);
    }
    set_darker(Gamma::Off);
}

// The same PDF again with every page is "PDF already imported." at any level: the notebook is not drawn again.
#[test]
fn the_same_pdf_again_is_already_imported_at_any_level() {
    let mut e = setup("again");
    let bytes = pdffix::planner(2);
    let (off, _) = planner_at(&mut e, "again-a", Gamma::Off, &bytes);
    let id = e.only_notebook();
    set_darker(Gamma::Strong);
    let (_, ends) = import_all(&e.drop("Planner again.pdf", &bytes));
    assert_eq!(last(&ends), &SliceEnd::Already, "{ends:?}");
    assert_eq!(e.only_notebook(), id);
    let now = pictures(&e.root, &id);
    assert!(now.iter().zip(&off).all(|(a, b)| a.2 == b.2), "its pictures are as they were");
    assert!(gamma_lines(&e.source_ini(&id)).is_empty());
    set_darker(Gamma::Off);
}

// A notebook's level is read strictly from its source.ini: only the three known values, the first gamma line, and none
// is Off (a notebook made before 2.2, at Off, or by KOReader).
#[test]
fn a_notebooks_level_reads_only_the_three_known_values() {
    let rec = "file=sha256:5e54918cec7ac9735b8f0e82c1ee75b9f0afef2eadca2998bdb313d4444c75c3:10:0123456789abcdef0123456789abcdef\n";
    assert_eq!(gamma_among(format!("{rec}gamma=1.5\n").as_bytes()), Gamma::Medium);
    assert_eq!(gamma_among(b"gamma=2.0\ngamma=1.5\n"), Gamma::Strong);
    assert_eq!(gamma_among(rec.as_bytes()), Gamma::Off);
    assert_eq!(gamma_among(b""), Gamma::Off);
    for bad in ["gamma=2", "gamma=3.0", "gamma= 1.5", "gamma=1.5 ", "Gamma=1.5", "gamma=", "gamma=1.0"] {
        assert_eq!(gamma_among(format!("{rec}{bad}\n").as_bytes()), Gamma::Off, "{bad}");
    }
    assert_eq!(gamma_line(Gamma::Off), "");
    assert_eq!(gamma_line(Gamma::Medium), "gamma=1.5\n");
    assert_eq!(gamma_line(Gamma::Strong), "gamma=2.0\n");
}

#[test]
fn no_em_dash() {
    assert!(!include_str!("test_pdf_darker.rs").contains('\u{2014}'));
}
