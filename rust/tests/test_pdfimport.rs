// test_pdfimport.rs: the PDF import end to end (2.1.0-beta.2, DESIGN.md 9 "L5"; areas/pdf.md 7). The tests are
// the writer: they call the slice function pdfimport::run_slice in this process, and every page goes through
// a real worker, a fresh exec of the headless runner (GRIMOIRE_WORK_EXE), as in the app. Generated PDFs only
// (tests/pdffix). The Linux sandbox steps need root and run in the CI job (test_sandbox_linux.rs).
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::fsview::StoragePath;
use grimoire::pdfimport::*;
use grimoire::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

struct Env {
    root: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

const HOOKS: [&str; 13] = [
    "GRIMOIRE_PDF_TEST_WOKE",
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
    let storage = base.join(format!("grimoire-pdfimport-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, PW, PH);
    ui_reset();
    Env { root: root.to_str().unwrap().to_string(), _g: g }
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
    fn work_folders(&self) -> usize {
        std::fs::read_dir(format!("{}/notebooks", self.root))
            .map(|r| r.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(".import-")).count())
            .unwrap_or(0)
    }
    fn done_files(&self) -> usize {
        std::fs::read_dir(format!("{}/import/done", self.root)).map(|r| r.flatten().count()).unwrap_or(0)
    }
}

/// Runs the slices of one import to its end: (the ends, in order).
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
    assert_eq!(r.info().bit_depth, png::BitDepth::Eight);
    let mut buf = vec![0u8; r.output_buffer_size().unwrap()];
    let f = r.next_frame(&mut buf).unwrap();
    buf.truncate(f.buffer_size());
    (f.width, f.height, ch, buf)
}

fn px(p: &(u32, u32, u32, Vec<u8>), x: u32, y: u32) -> Vec<u8> {
    let ch = p.2 as usize;
    let at = (y as usize * p.0 as usize + x as usize) * ch;
    p.3[at..at + ch].to_vec()
}

/// Where the planner's block's centre lands on a 600 x 800 page: the fit of a 420 x 595 page.
fn block_centre() -> (u32, u32) {
    let s = (600.0f64 / 420.0).min(800.0 / 595.0);
    let (rw, rh) = ((420.0 * s) as i32, (595.0 * s) as i32);
    let (ox, oy) = ((600 - rw) / 2, (800 - rh) / 2);
    ((ox as f64 + 210.0 * s) as u32, (oy as f64 + (595.0 - 460.0) * s) as u32)
}

#[test]
fn a_planner_becomes_a_notebook_of_its_pages() {
    let e = setup("planner");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    let src = e.drop("Planner.pdf", &pdffix::planner(7));
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 7, blank: 0 }, "{ends:?}");
    // slice 0 probes, then 4 slices of at most 2 pages, the last committing
    assert_eq!(ends.len(), 5, "{ends:?}");
    assert!(ends[..4].iter().all(|x| *x == SliceEnd::More));
    let nbs = e.notebooks();
    assert_eq!(nbs.len(), 1);
    let nb = &nbs[0];
    assert_eq!(nb.id.as_str(), id);
    assert_eq!(nb.name.as_str(), "Planner");
    assert_eq!(nb.npages(), 7);
    assert_eq!((nb.page_w, nb.page_h), (PW, PH));
    // its book key is KOReader's partial MD5 of the file
    let key = partial_md5_of(&format!("{}/import/done", e.root), &e);
    assert_eq!(nb.source.as_str(), key);
    // pages are fitted and centred: the block of page k is its gray, at the fit's place
    let (cx, cy) = block_centre();
    for k in 1..=7u32 {
        let p = picture_of(&e.root, &id, k);
        assert_eq!((p.0, p.1, p.2), (PW as u32, PH as u32, 1), "page {k} is an 8-bit gray page");
        let g = px(&p, cx, cy)[0] as i32;
        assert!((g - pdffix::planner_gray(k as usize) as i32).abs() <= 2, "page {k}: block {g}, wanted {}", pdffix::planner_gray(k as usize));
        // the margin beside a fitted page is white
        assert_eq!(px(&p, 2, 400), vec![255], "page {k}: the margin is white");
    }
    // the PDF moved to done/, the work folder is gone, no state is left in the notebook
    assert_eq!(e.done_files(), 1);
    assert!(!std::path::Path::new(&format!("{}/import/Planner.pdf", e.root)).exists());
    assert_eq!(e.work_folders(), 0);
    assert!(!std::path::Path::new(&format!("{}/notebooks/{id}/pdf-import.ini", e.root)).exists());
    // source.ini maps every book page to its page
    let src_ini = std::fs::read_to_string(format!("{}/notebooks/{id}/source.ini", e.root)).unwrap();
    assert!(src_ini.contains(&format!("book={key}")) && src_ini.contains("kind=paged") && src_ini.contains("page.7.7=7"), "{src_ini}");
    // a page added after the last is blank (the notebook's own paper)
    assert_eq!(nb.paper, PAPER_BLANK);
}

// the partial MD5 of the PDF now in done/ (only one file there)
fn partial_md5_of(done: &str, _e: &Env) -> String {
    let f = std::fs::read_dir(done).unwrap().flatten().next().unwrap().path();
    let fd = std::fs::File::open(f).unwrap();
    use std::os::fd::AsRawFd;
    grimoire::md5::partial_md5_fd(fd.as_raw_fd())
}

#[test]
fn a_colour_page_is_stored_rgb_and_a_gray_page_gray() {
    let e = setup("colour");
    let src = e.drop("Colour.pdf", &pdffix::colour());
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 3, blank: 0 }, "{ends:?}");
    let chans: Vec<u32> = (1..=3).map(|k| picture_of(&e.root, &id, k).2).collect();
    assert_eq!(chans, vec![1, 3, 1], "gray, colour, gray");
    // the red box of page 2 is red where the fit puts it (user x 60..180, y 100..220)
    let p = picture_of(&e.root, &id, 2);
    let s = (600.0f64 / 420.0).min(800.0 / 595.0);
    let (rw, rh) = ((420.0 * s) as i32, (595.0 * s) as i32);
    let (ox, oy) = ((600 - rw) / 2, (800 - rh) / 2);
    let (cx, cy) = ((ox as f64 + 120.0 * s) as u32, (oy as f64 + (595.0 - 160.0) * s) as u32);
    assert_eq!(px(&p, cx, cy), vec![255, 0, 0]);
    // the pale yellow band is colour too, though it is nearly white
    let (bx, by) = ((ox as f64 + 200.0 * s) as u32, (oy as f64 + (595.0 - 320.0) * s) as u32);
    let band = px(&p, bx, by);
    assert!(band[2] < band[0], "yellow is not gray: {band:?}");
    // the Library thumbnail is page 1's, in gray
    assert!(std::path::Path::new(&format!("{}/notebooks/{id}/thumb.pgm", e.root)).exists());
}

#[test]
fn colour_pages_load_as_gray_on_the_scribe_and_keep_the_memory_budget() {
    // what the UI keeps of a page picture is its luma (bgimage decodes either to a gray Canvas and drops the
    // RGB copy), and a full-size RGB page is within what a page picture drawn on the UI thread may take
    let e = setup("budget");
    let src = e.drop("Colour.pdf", &pdffix::colour());
    let (id, _) = import_all(&src);
    let mut nb = NotebookInfo::default();
    store_load_info(&id, &mut nb);
    let mut pg = None;
    assert_eq!(store_load_page(&nb, 1, PW, PH, &mut pg), 0);
    let pic = paper_image_id(pg.as_ref().unwrap().paper);
    let file = format!("{}/notebooks/{id}/bg/{pic:06x}.png", e.root);
    let data = std::fs::read(&file).unwrap();
    let (mut w, mut h) = (0, 0);
    let mut why = "";
    assert_eq!(bgimage_drawable(&data, &mut w, &mut h, &mut why), 0, "{why}");
    // a Scribe-sized RGB page (1860 x 2480, 13.8 MB decoded) is drawable on the UI thread
    let rgb = BgPicture { px: vec![200u8; 1860 * 2480 * 3], w: 1860, h: 2480, ch: 3 };
    let png = grimoire::pdfimport::encode_png(&rgb).unwrap();
    assert_eq!(bgimage_drawable(&png, &mut w, &mut h, &mut why), 0, "{why}");
    assert_eq!((w, h), (1860, 2480));
    // and what is kept of it is gray: bgimage_luma gives the one-channel canvas
    let mut luma = Canvas::default();
    assert_eq!(bgimage_luma(&rgb, &mut luma), 0);
    assert_eq!(luma.px.len(), 1860 * 2480);
    assert!(luma.px.iter().all(|&v| v == 200));
}

#[test]
fn a_password_refuses_the_pdf_and_an_owner_password_does_not() {
    let e = setup("password");
    let src = e.drop("Locked.pdf", &pdffix::encrypted_planner(2, "secret", "owner"));
    let (_, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Failed(2, 0)), "{ends:?}"); // R_PASSWORD
    assert!(e.notebooks().is_empty());
    assert_eq!(e.work_folders(), 0);
    assert!(std::path::Path::new(&format!("{}/import/Locked.pdf", e.root)).exists(), "the PDF stays where it was");
    // owner password only: opens, every page renders
    let src = e.drop("Open.pdf", &pdffix::encrypted_planner(2, "", "owner"));
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 2, blank: 0 }, "{ends:?}");
    let p = picture_of(&e.root, &id, 1);
    let (cx, cy) = block_centre();
    let g = px(&p, cx, cy)[0] as i32;
    assert!((g - pdffix::planner_gray(1) as i32).abs() <= 2, "{g}");
}

#[test]
fn more_pages_than_a_notebook_holds_are_refused_before_anything_is_drawn() {
    let e = setup("long");
    let src = e.drop("Long.pdf", &pdffix::many(2001));
    let (_, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Failed(3, 2000)), "{ends:?}"); // R_TOO_LONG, 2000
    assert_eq!(ends.len(), 1);
    assert!(e.notebooks().is_empty());
    assert_eq!(e.work_folders(), 0);
    assert!(std::path::Path::new(&format!("{}/import/Long.pdf", e.root)).exists());
    // 2000 exactly is taken
    let src = e.drop("Edge.pdf", &pdffix::many(2000));
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "500");
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 2000, blank: 0 }, "{:?}", &ends[ends.len() - 2..]);
    let mut nb = NotebookInfo::default();
    store_load_info(&id, &mut nb);
    assert_eq!(nb.npages(), 2000);
}

#[test]
fn hostile_files_fail_whole_and_leave_the_file() {
    let e = setup("hostile");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("junk.pdf", pdffix::junk()),
        ("loop.pdf", pdffix::page_tree_loop()),
        ("empty.pdf", b"%PDF-1.4\n%%EOF\n".to_vec()),
        ("zero.pdf", Vec::new()),
    ];
    for (name, bytes) in cases {
        let src = e.drop(name, &bytes);
        let (_, ends) = import_all(&src);
        assert!(matches!(last(&ends), SliceEnd::Failed(..)), "{name}: {ends:?}");
        assert!(e.notebooks().is_empty(), "{name}");
        assert_eq!(e.work_folders(), 0, "{name}");
        assert!(std::path::Path::new(&format!("{}/import/{name}", e.root)).exists(), "{name}: the file stays");
    }
}

#[test]
fn a_page_tree_nested_very_deep_neither_crashes_nor_hangs_the_import() {
    // hayro walks it without recursion: it reads (one page) or refuses, never crashes the writer
    let e = setup("deep");
    let src = e.drop("Deep.pdf", &pdffix::page_tree_deep(5000));
    let (_, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Done { .. } | SliceEnd::Failed(..)), "{ends:?}");
}

#[test]
fn a_truncated_pdf_that_the_renderer_repairs_keeps_what_it_can_read() {
    // hayro repairs a cut file: the pages it finds import (the plan's 2.1 behaviour for repaired files)
    let e = setup("truncated");
    let src = e.drop("Cut.pdf", &pdffix::truncated(8));
    let (id, ends) = import_all(&src);
    match last(&ends) {
        SliceEnd::Done { pages, .. } => {
            let mut nb = NotebookInfo::default();
            store_load_info(&id, &mut nb);
            assert_eq!(nb.npages() as u32, *pages);
            assert!(*pages >= 1 && *pages <= 8);
        }
        SliceEnd::Failed(..) => assert!(e.notebooks().is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_page_that_kills_its_worker_twice_ends_blank_and_the_rest_import() {
    let e = setup("isolation");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "3");
    let src = e.drop("Planner.pdf", &pdffix::planner(6));
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 6, blank: 1 }, "{ends:?}");
    let (cx, cy) = block_centre();
    for k in 1..=6u32 {
        let p = picture_of(&e.root, &id, k);
        let g = px(&p, cx, cy)[0] as i32;
        if k == 3 {
            assert_eq!(g, 255, "page 3 is blank");
            assert!(p.3.iter().all(|&v| v == 255));
        } else {
            assert!((g - pdffix::planner_gray(k as usize) as i32).abs() <= 2, "page {k}: {g}");
        }
    }
    assert_eq!(e.done_files(), 1, "the import went through");
}

#[test]
fn a_pdf_whose_every_page_kills_its_worker_fails_whole() {
    // keeping the good pages (D37) never makes a notebook of nothing but blank pages
    let e = setup("isolation-all");
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "1");
    let src = e.drop("Planner.pdf", &pdffix::planner(1));
    let (_id, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Failed(..)), "{ends:?}");
    assert!(e.notebooks().is_empty());
    assert_eq!(e.work_folders(), 0);
    assert_eq!(e.done_files(), 0, "the PDF stays where it was");
}

#[test]
fn a_page_that_stalls_twice_ends_blank_too() {
    let e = setup("hang");
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "700");
    std::env::set_var("GRIMOIRE_PDF_TEST_HANG_PAGE", "2");
    let src = e.drop("Planner.pdf", &pdffix::planner(3));
    let t = std::time::Instant::now();
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 3, blank: 1 }, "{ends:?}");
    assert!(t.elapsed().as_secs() < 30);
    let p = picture_of(&e.root, &id, 2);
    assert!(p.3.iter().all(|&v| v == 255));
    let p = picture_of(&e.root, &id, 3);
    assert!(p.3.iter().any(|&v| v < 255));
}

#[test]
fn the_state_between_slices_keeps_the_blank_pages_for_a_resume() {
    let e = setup("resume");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "1");
    let src = e.drop("Planner.pdf", &pdffix::planner(5));
    let mut id = String::new();
    store_make_id(&mut id);
    assert_eq!(run_slice(&id, &src), SliceEnd::More); // slice 0
    assert_eq!(run_slice(&id, &src), SliceEnd::More); // pages 1 and 2
    // (the process "dies" here: only the work folder is left)
    let t = std::fs::read(format!("{}/notebooks/.import-{id}/pdf-import.ini", e.root)).unwrap();
    let s = State::parse(&t).unwrap();
    assert_eq!((s.next, s.pages, s.blank.clone()), (3, 5, vec![1]));
    assert!(keep_work_folder(&id), "a resumable work folder is kept");
    // the crash hook goes: the rest of the pages are made, and page 1 stays blank (it was given up on)
    std::env::remove_var("GRIMOIRE_PDF_TEST_CRASH_PAGE");
    let mut end = SliceEnd::More;
    for _ in 0..10 {
        end = run_slice(&id, &src);
        if end != SliceEnd::More {
            break;
        }
    }
    assert_eq!(end, SliceEnd::Done { pages: 5, blank: 1 });
    assert!(picture_of(&e.root, &id, 1).3.iter().all(|&v| v == 255));
}

#[test]
fn a_worker_that_runs_out_of_memory_is_a_failed_page() {
    // the alloc hook asks for more than any address-space cap allows; where the OS does not honour the cap
    // (macOS) it simply allocates and the page renders: either way the import ends with every page made
    let e = setup("alloc");
    std::env::set_var("GRIMOIRE_PDF_TEST_ALLOC_PAGE", "2");
    let src = e.drop("Planner.pdf", &pdffix::planner(3));
    let (_, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Done { pages: 3, .. }), "{ends:?}");
}

#[test]
fn the_same_pdf_twice_is_one_notebook_and_the_second_says_already() {
    let e = setup("duplicates");
    let bytes = pdffix::planner(2);
    let a = e.drop("A.pdf", &bytes);
    let (_, ends) = import_all(&a);
    assert!(matches!(last(&ends), SliceEnd::Done { .. }));
    let b = e.drop("B.pdf", &bytes); // the same bytes under another name
    let (_, ends) = import_all(&b);
    assert_eq!(last(&ends), &SliceEnd::Already, "{ends:?}");
    assert_eq!(e.notebooks().len(), 1);
    assert_eq!(e.done_files(), 2, "the second PDF moved to done/ as well");
    assert!(!std::path::Path::new(&format!("{}/import/B.pdf", e.root)).exists());
    // a different PDF is a new notebook, and one of the same name is numbered
    let c = e.drop("A.pdf", &pdffix::planner(3));
    let (_, ends) = import_all(&c);
    assert!(matches!(last(&ends), SliceEnd::Done { pages: 3, .. }));
    let names: Vec<String> = e.notebooks().iter().map(|n| n.name.as_str().to_string()).collect();
    assert!(names.contains(&"A".to_string()) && names.contains(&"A 2".to_string()), "{names:?}");
}

#[test]
fn a_pdf_picked_anywhere_is_read_in_place_and_never_moved() {
    let e = setup("pick");
    std::fs::create_dir_all(format!("{}/../Documents", e.root)).unwrap();
    std::fs::write(format!("{}/../Documents/Book.pdf", e.root), pdffix::planner(2)).unwrap();
    let src = "Documents/Book.pdf";
    let m = Mark { state: MarkState::Making, id: String::new(), src: src.into(), pick: true, name: "Chosen".into(), folder: "Reading".into(), page: 0, pages: 0, at: 1, ..Default::default() };
    let dir = format!("{}/import/.marks", e.root);
    std::fs::create_dir_all(&dir).unwrap();
    let hash = {
        // the mark's file name is a hash of the source path: written through the module's own writer
        let mut id = String::new();
        store_make_id(&mut id);
        let mm = Mark { id: id.clone(), ..m };
        assert!(mark_write_public(&mm));
        (id, mm)
    };
    let (id, _) = hash;
    let mut end = SliceEnd::More;
    for _ in 0..20 {
        end = run_slice(&id, src);
        if end != SliceEnd::More {
            break;
        }
    }
    assert_eq!(end, SliceEnd::Done { pages: 2, blank: 0 });
    assert!(std::path::Path::new(&format!("{}/../Documents/Book.pdf", e.root)).exists(), "never moved");
    assert_eq!(e.done_files(), 0);
    let nbs = e.notebooks();
    assert_eq!((nbs[0].name.as_str(), nbs[0].folder.as_str()), ("Chosen", "Reading"));
}

#[test]
fn the_files_inside_grimoires_own_folders_are_refused_by_identity() {
    let e = setup("own");
    for dir in ["notebooks", "PDFs", "trash"] {
        std::fs::create_dir_all(format!("{}/{dir}", e.root)).unwrap();
        std::fs::write(format!("{}/{dir}/X.pdf", e.root), pdffix::planner(1)).unwrap();
        let src = format!("grimoire/{dir}/X.pdf");
        let (_, ends) = import_all(&src);
        assert!(matches!(last(&ends), SliceEnd::Failed(..)), "{dir}: {ends:?}");
    }
    assert!(e.notebooks().is_empty());
    // and by name in the picker's entry point
    let sp = StoragePath::new(b"grimoire/PDFs/X.pdf").unwrap();
    assert_eq!(pdf_import_queue(&sp, None, None, 300.0), Queued::Refused("own"));
}

#[test]
fn a_file_that_changes_between_slices_fails_the_job_and_is_tried_again_later() {
    let e = setup("changed");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let src = e.drop("Growing.pdf", &pdffix::planner(4));
    let mut id = String::new();
    store_make_id(&mut id);
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    // the copy grows (or is replaced): size and mtime differ now
    let mut b = pdffix::planner(4);
    b.extend_from_slice(b"\n% more\n%%EOF\n");
    std::fs::write(format!("{}/import/Growing.pdf", e.root), b).unwrap();
    assert!(!keep_work_folder(&id), "a changed source is not resumed");
    assert!(matches!(run_slice(&id, &src), SliceEnd::Failed(..)));
    assert_eq!(e.work_folders(), 0);
    assert!(e.notebooks().is_empty());
}

#[test]
fn no_room_on_the_storage_refuses_the_import_with_the_storage_message() {
    let e = setup("room");
    std::env::set_var("GRIMOIRE_PDF_FREE_BYTES", "1000000"); // 1 MB free: less than the floor
    let src = e.drop("Planner.pdf", &pdffix::planner(2));
    let (_, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Failed(6, 0)), "{ends:?}"); // R_STORAGE
    assert!(e.notebooks().is_empty());
}

#[test]
fn the_private_mapping_or_reading_whole_gives_the_same_pages() {
    let e = setup("mapped");
    let src = e.drop("A.pdf", &pdffix::colour());
    let (a, _) = import_all(&src);
    let one: Vec<Vec<u8>> = (1..=3).map(|k| picture_of(&e.root, &a, k).3).collect();
    let _ = std::fs::remove_dir_all(format!("{}/notebooks/{a}", e.root));
    std::env::set_var("GRIMOIRE_PDF_NO_MAP", "1");
    let src = e.drop("B.pdf", &pdffix::colour());
    let (b, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Done { .. }), "{ends:?}");
    let two: Vec<Vec<u8>> = (1..=3).map(|k| picture_of(&e.root, &b, k).3).collect();
    assert!(one == two, "mapped and read-whole pages differ");
}

#[test]
fn band_mode_makes_the_same_pages_as_whole_page_mode() {
    let e = setup("bands");
    let src = e.drop("A.pdf", &pdffix::colour());
    let (a, _) = import_all(&src);
    let whole: Vec<(u32, Vec<u8>)> = (1..=3).map(|k| { let p = picture_of(&e.root, &a, k); (p.2, p.3) }).collect();
    let _ = std::fs::remove_dir_all(format!("{}/notebooks/{a}", e.root));
    std::env::set_var("GRIMOIRE_PDF_BANDS", "4");
    let src = e.drop("B.pdf", &pdffix::colour());
    let (b, ends) = import_all(&src);
    assert!(matches!(last(&ends), SliceEnd::Done { .. }), "{ends:?}");
    for (k, (ch, w)) in whole.iter().enumerate() {
        let p = picture_of(&e.root, &b, k as u32 + 1);
        assert_eq!(p.2, *ch, "page {}", k + 1);
        // (a band edge may differ by a level or two from the whole render: the interpretation is the same)
        let bad = p.3.iter().zip(w.iter()).filter(|(x, y)| (**x as i32 - **y as i32).abs() > 3).count();
        assert!(bad < 50, "page {}: {bad} samples differ", k + 1);
    }
}

#[test]
fn a_sandbox_step_that_fails_refuses_the_pdf_counts_no_try_and_marks_nothing() {
    let e = setup("sandbox");
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "chroot");
    let src = e.drop("Planner.pdf", &pdffix::planner(2));
    let attempts = format!("{}/import/.attempts", e.root);
    let (_, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Sandbox, "{ends:?}");
    assert!(e.notebooks().is_empty());
    assert_eq!(e.work_folders(), 0);
    assert!(std::path::Path::new(&format!("{}/import/Planner.pdf", e.root)).exists());
    assert!(!std::path::Path::new(&attempts).exists(), "no try was counted");
    assert!(sandbox_broken_in_progress(), "the session is told");
}

#[test]
fn a_worker_of_another_build_is_refused() {
    // the handshake: the first record names the build; the writer's parse of another is a refusal
    use grimoire::pdfwork::*;
    assert_eq!(parse_record(b"B 9.9.9 1"), Ok(Rec::Build("9.9.9 1".into())));
    assert_ne!(Rec::Build("9.9.9 1".into()), Rec::Build(grimoire::sandbox::build_id()));
}

// Where the planned rectangle lands: independent of hayro's own transform, by turning the user-space point.
fn expect_box(w0: f64, h0: f64, rotate: i32, crop: Option<[f64; 4]>, bx: [f64; 4]) {
    let e = setup(&format!("map{rotate}{}", crop.is_some() as u8));
    let src = e.drop("Box.pdf", &pdffix::boxed(w0, h0, rotate, crop, bx));
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 1, blank: 0 }, "{ends:?}");
    let p = picture_of(&e.root, &id, 1);
    let c = crop.unwrap_or([0.0, 0.0, w0, h0]);
    let (cw, ch) = (c[2] - c[0], c[3] - c[1]);
    let (rw0, rh0) = if rotate == 90 || rotate == 270 { (ch, cw) } else { (cw, ch) };
    let s = (PW as f64 / rw0).min(PH as f64 / rh0);
    let (rw, rh) = ((rw0 * s) as i32, (rh0 * s) as i32);
    let (ox, oy) = (((PW - rw) / 2) as f64, ((PH - rh) / 2) as f64);
    // a user-space point to a page pixel
    let map = |x: f64, y: f64| -> (f64, f64) {
        let (u, v) = (x - c[0], c[3] - y); // y down, in the crop box
        let (a, b) = match rotate {
            90 => (ch - v, u),
            180 => (cw - u, ch - v),
            270 => (v, cw - u),
            _ => (u, v),
        };
        (ox + a * s, oy + b * s)
    };
    let corners = [map(bx[0], bx[1]), map(bx[0] + bx[2], bx[1] + bx[3])];
    let (x0, x1) = (corners[0].0.min(corners[1].0), corners[0].0.max(corners[1].0));
    let (y0, y1) = (corners[0].1.min(corners[1].1), corners[0].1.max(corners[1].1));
    let dark = |x: f64, y: f64| px(&p, x as u32, y as u32)[0] < 60;
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    assert!(dark(mx, my), "rot {rotate}: the box's middle is dark");
    assert!(dark(x0 + 3.0, y0 + 3.0) && dark(x1 - 3.0, y1 - 3.0), "rot {rotate}: just inside its corners is dark");
    assert!(!dark(x0 - 4.0, my) && !dark(x1 + 4.0, my) && !dark(mx, y0 - 4.0) && !dark(mx, y1 + 4.0), "rot {rotate}: just outside is white");
}

#[test]
fn a_boxes_place_is_the_fit_for_every_rotation_a_crop_and_a_landscape_page() {
    for rot in [0, 90, 180, 270] {
        expect_box(420.0, 595.0, rot, None, [100.0, 300.0, 80.0, 60.0]);
    }
    // an offset crop box
    expect_box(500.0, 700.0, 0, Some([40.0, 60.0, 440.0, 660.0]), [100.0, 300.0, 80.0, 60.0]);
    expect_box(500.0, 700.0, 90, Some([40.0, 60.0, 440.0, 660.0]), [100.0, 300.0, 80.0, 60.0]);
    // landscape, fitted as it is (letterboxed)
    expect_box(842.0, 595.0, 0, None, [100.0, 300.0, 80.0, 60.0]);
    // A4
    expect_box(595.0, 842.0, 0, None, [100.0, 300.0, 80.0, 60.0]);
}

// ====================================================================================================
// The fix round of 2026-11-09 (the review of 0c26d912): each test failed on that line.
// ====================================================================================================

fn new_id() -> String {
    let mut id = String::new();
    store_make_id(&mut id);
    id
}

fn run_to_end(id: &str, src: &str, most: usize) -> Vec<SliceEnd> {
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

fn attempts_of(e: &Env) -> String {
    std::fs::read_to_string(format!("{}/import/.attempts", e.root)).unwrap_or_default()
}

fn tries_of(e: &Env) -> Vec<i32> {
    attempts_of(e).lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.split_whitespace().nth(2)?.parse().ok()).collect()
}

/// A stand-in worker: it starts and shakes hands, then runs `then` (a shell line), as a worker the writer cannot
/// tell from a real one up to there.
fn stand_in_worker(e: &Env, name: &str, then: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let path = format!("{}/{name}.sh", e.root);
    std::fs::write(&path, format!("#!/bin/sh\nprintf 'B {}\\nS 0\\n' >&4\n{then}\n", grimoire::sandbox::build_id())).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The writer of one slice in a child of its own (as a bgjob child is), in a process group with its workers.
fn fork_slice(id: &str, src: &str) -> i32 {
    // SAFETY: a fork whose child only runs one slice and ends with _exit.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        unsafe { libc::setpgid(0, 0) };
        let _ = run_slice(id, src);
        unsafe { libc::_exit(0) };
    }
    unsafe { libc::setpgid(pid, pid) };
    pid
}

fn kill_group(pid: i32) {
    // SAFETY: SIGKILL to a process group this test made, then waitpid on its leader.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
        libc::kill(pid, libc::SIGKILL);
        let mut st = 0;
        libc::waitpid(pid, &mut st, 0);
    }
}

// ---- H6: a picked file is never moved, whatever its mark ----

fn pick_mark(id: &str, src: &str, name: &str) -> Mark {
    Mark { state: MarkState::Making, id: id.into(), src: src.into(), pick: true, name: name.into(), at: 1, ..Default::default() }
}

fn a_pick_whose_mark_is_lost(name: &str, lose: impl Fn(&str)) {
    let e = setup(name);
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    std::fs::create_dir_all(format!("{}/../documents", e.root)).unwrap();
    let file = format!("{}/../documents/Book.pdf", e.root);
    std::fs::write(&file, pdffix::planner(3)).unwrap();
    let src = "documents/Book.pdf";
    let id = new_id();
    assert!(mark_write_public(&pick_mark(&id, src, "Chosen")));
    assert_eq!(run_slice(&id, src), SliceEnd::More); // the probe
    assert_eq!(run_slice(&id, src), SliceEnd::More); // page 1
    // the Kindle stops here; the mark does not survive (the work folder does)
    lose(&format!("{}/import/.marks", e.root));
    assert!(keep_work_folder(&id));
    let ends = run_to_end(&id, src, 20);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 3, blank: 0 }, "{ends:?}");
    assert!(std::path::Path::new(&file).exists(), "the picked file left documents/");
    assert_eq!(e.done_files(), 0);
    assert_eq!(e.notebooks()[0].name.as_str(), "Chosen", "the name typed is kept in the state too");
}

#[test]
fn a_pick_resumed_without_its_mark_never_moves_the_users_file() {
    a_pick_whose_mark_is_lost("mark-gone", |dir| {
        for m in std::fs::read_dir(dir).unwrap().flatten() {
            std::fs::remove_file(m.path()).unwrap();
        }
    });
}

#[test]
fn a_pick_resumed_with_a_torn_mark_never_moves_the_users_file() {
    a_pick_whose_mark_is_lost("mark-torn", |dir| {
        for m in std::fs::read_dir(dir).unwrap().flatten() {
            std::fs::write(m.path(), b"").unwrap(); // (created, not yet written: what a cut-off write leaves)
        }
    });
}

// A pick inside grimoire/import is a drop whoever chose it: filed in done/ at its commit, with the name typed.
#[test]
fn a_pdf_picked_inside_the_import_folder_is_settled_by_its_own_import() {
    let e = setup("pick-in-import");
    let src = e.drop("Here.pdf", &pdffix::planner(2));
    let id = new_id();
    assert!(mark_write_public(&pick_mark(&id, &src, "Typed")));
    assert_eq!(last(&run_to_end(&id, &src, 10)), &SliceEnd::Done { pages: 2, blank: 0 });
    assert!(!std::path::Path::new(&format!("{}/import/Here.pdf", e.root)).exists());
    assert_eq!(e.done_files(), 1);
    assert_eq!(e.notebooks()[0].name.as_str(), "Typed");
}

// ---- H7: a resume never commits pages whose files are gone ----

fn pages_without_their_picture(root: &str, id: &str) -> Vec<u32> {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let mut gone = Vec::new();
    for i in 0..nb.npages() {
        let mut pg = None;
        assert_eq!(store_load_page(&nb, i, PW, PH, &mut pg), 0);
        let path = format!("{root}/notebooks/{id}/bg/{:06x}.png", paper_image_id(pg.as_ref().unwrap().paper));
        if !std::path::Path::new(&path).exists() {
            gone.push(i as u32 + 1);
        }
    }
    gone
}

#[test]
fn a_slice_cut_off_after_a_strike_keeps_the_pictures_of_its_good_pages() {
    let e = setup("cut-after-strike");
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "1500");
    let src = e.drop("Planner.pdf", &pdffix::planner(6));
    let id = new_id();
    assert_eq!(run_slice(&id, &src), SliceEnd::More); // the probe
    std::env::set_var("GRIMOIRE_PDF_TEST_HANG_PAGE", "4");
    let state_file = format!("{}/notebooks/.import-{id}/pdf-import.ini", e.root);
    let pid = fork_slice(&id, &src);
    let t0 = std::time::Instant::now();
    let mut state = String::new();
    while t0.elapsed().as_secs() < 30 {
        state = std::fs::read_to_string(&state_file).unwrap_or_default();
        if state.contains("strikes=4:1") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    kill_group(pid); // (page 4's second try hangs: the writer ends mid-slice)
    assert!(state.contains("strikes=4:1") && state.contains("next=4"), "the state after the strike: {state}");
    std::env::remove_var("GRIMOIRE_PDF_TEST_HANG_PAGE");
    let ends = run_to_end(&id, &src, 50);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 6, blank: 0 }, "{ends:?}");
    assert_eq!(pages_without_their_picture(&e.root, &id), Vec::<u32>::new());
}

/// An import cut off after two pages, then `cut` done to its work folder (an older build's sweep cut off).
fn resume_after(name: &str, cut: impl Fn(&str)) {
    let e = setup(name);
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    let src = e.drop("Planner.pdf", &pdffix::planner(5));
    let id = new_id();
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    cut(&format!("{}/notebooks/.import-{id}", e.root));
    let ends = run_to_end(&id, &src, 20);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 5, blank: 0 }, "{ends:?}");
    assert_eq!(pages_without_their_picture(&e.root, &id), Vec::<u32>::new(), "every page has its picture");
    assert!(std::path::Path::new(&format!("{}/notebooks/{id}/p1.qpg", e.root)).exists(), "and its page file");
}

#[test]
fn a_resume_after_a_sweep_that_took_the_pictures_and_index_makes_them_again() {
    resume_after("sweep", |work| {
        for f in std::fs::read_dir(format!("{work}/bg")).unwrap().flatten() {
            std::fs::remove_file(f.path()).unwrap();
        }
    });
}

#[test]
fn a_resume_after_a_sweep_that_took_the_pictures_only_makes_them_again() {
    resume_after("sweep-pics", |work| {
        for f in std::fs::read_dir(format!("{work}/bg")).unwrap().flatten() {
            if f.file_name().to_string_lossy().ends_with(".png") {
                std::fs::remove_file(f.path()).unwrap();
            }
        }
    });
}

#[test]
fn a_resume_after_a_sweep_that_took_a_page_file_makes_it_again() {
    resume_after("sweep-page", |work| std::fs::remove_file(format!("{work}/p1.qpg")).unwrap());
}

// ---- H9: a copy waits without holding its job ----

#[test]
fn a_second_copy_does_not_wait_forever_on_a_work_folder_no_job_runs() {
    let e = setup("twins");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let bytes = pdffix::planner(3);
    let a = e.drop("First.pdf", &bytes);
    let b = e.drop("Second.pdf", &bytes);
    let ida = new_id();
    assert_eq!(run_slice(&ida, &a), SliceEnd::More);
    assert_eq!(run_slice(&ida, &a), SliceEnd::More);
    // what the detached child's loop does with the copy's job: it ends at once, counting nothing
    let ends = run_to_end(&new_id(), &b, 15);
    assert!(ends.len() < 15 && *last(&ends) != SliceEnd::More, "{ends:?}");
    assert_eq!(tries_of(&e), vec![1], "only First's try is on record");
    // the first goes on to its end; the copy, taken again, is "already imported"
    assert_eq!(last(&run_to_end(&ida, &a, 20)), &SliceEnd::Done { pages: 3, blank: 0 });
    assert_eq!(last(&run_to_end(&new_id(), &b, 5)), &SliceEnd::Already);
}

// ---- M1: a worker that cannot open the PDF in a later slice ----

#[test]
fn a_worker_that_cannot_open_the_pdf_in_a_later_slice_does_not_blank_the_rest() {
    let e = setup("probe-dies");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    let src = e.drop("Planner.pdf", &pdffix::planner(9));
    let id = new_id();
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    std::env::set_var("GRIMOIRE_WORK_EXE", stand_in_worker(&e, "dies-opening", "exit 0"));
    let ends = run_to_end(&id, &src, 40);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert!(matches!(last(&ends), SliceEnd::Failed(..)), "{ends:?}");
    assert!(e.notebooks().is_empty() && e.done_files() == 0, "no notebook of blank pages, no file filed");
}

#[test]
fn a_file_that_stops_opening_fails_the_import_instead_of_blanking_it() {
    let e = setup("stops-opening");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    let path = format!("{}/import/Planner.pdf", e.root);
    let good = pdffix::planner(5);
    std::fs::write(&path, &good).unwrap();
    let md = std::fs::metadata(&path).unwrap();
    let src = "grimoire/import/Planner.pdf";
    let id = new_id();
    assert_eq!(run_slice(&id, src), SliceEnd::More);
    assert_eq!(run_slice(&id, src), SliceEnd::More);
    // no worker can open the file from here on (no PDF any more, under the same size and mtime)
    let mut junk = vec![b'x'; good.len()];
    junk[..9].copy_from_slice(b"%PDF-1.7\n");
    std::fs::write(&path, &junk).unwrap();
    let c = std::ffi::CString::new(path.clone()).unwrap();
    let tv = [libc::timeval { tv_sec: md.mtime() as _, tv_usec: 0 }, libc::timeval { tv_sec: md.mtime() as _, tv_usec: 0 }];
    // SAFETY: utimes reads the path and the two timevals we own.
    assert_eq!(unsafe { libc::utimes(c.as_ptr(), tv.as_ptr()) }, 0);
    let ends = run_to_end(&id, src, 20);
    assert!(matches!(last(&ends), SliceEnd::Failed(..)), "{ends:?}");
    assert!(e.notebooks().is_empty());
    assert_eq!(e.done_files(), 0);
}

// Read whole (the device refuses the private mapping), every worker of a slice reads the file from its start.
#[test]
fn read_whole_one_worker_death_blanks_only_its_page() {
    let e = setup("readwhole");
    std::env::set_var("GRIMOIRE_PDF_NO_MAP", "1");
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "2");
    let src = e.drop("Whole.pdf", &pdffix::planner(7));
    let ends = run_to_end(&new_id(), &src, 50);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 7, blank: 1 }, "{ends:?}");
}

// ---- M3: SIGPIPE never ends the writer ----

static PIPES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
extern "C" fn on_pipe(_sig: libc::c_int) {
    PIPES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn a_worker_that_ended_by_itself_never_raises_sigpipe_in_the_writer() {
    use grimoire::pdfwork::*;
    use grimoire::sandbox::*;
    use std::os::fd::AsRawFd;
    let e = setup("sigpipe");
    let path = format!("{}/import/locked.pdf", e.root);
    let bytes = pdffix::encrypted_planner(2, "secret", "owner");
    std::fs::write(&path, &bytes).unwrap();
    let len = bytes.len() as u64;
    let limits = Limits { as_bytes: worker_as_bytes(len, false), cpu_secs: 60, mapped: true, file_len: len };
    // SAFETY: a handler that only bumps an atomic (the harness ignores SIGPIPE: this counts what would end the writer).
    unsafe { libc::signal(libc::SIGPIPE, on_pipe as *const () as libc::sighandler_t) };
    PIPES.store(0, std::sync::atomic::Ordering::Relaxed);
    let f = std::fs::File::open(&path).unwrap();
    let mut s = Session::start(f.as_raw_fd(), len, &limits).expect("the worker starts");
    std::thread::sleep(std::time::Duration::from_millis(300)); // (the writer held up: the worker has gone)
    let r = s.probe();
    s.end();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let pipes = PIPES.load(std::sync::atomic::Ordering::Relaxed);
    // SAFETY: back to what the harness had.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
    assert_eq!(pipes, 0, "SIGPIPE reached the writer");
    assert_eq!(r, Err(StartErr::Password), "its last record was read");
}

// ---- M6: a worker that cannot start ----

#[test]
fn a_worker_that_cannot_start_counts_no_try() {
    let e = setup("no-worker-first");
    let src = e.drop("Planner.pdf", &pdffix::planner(2));
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let ends = run_to_end(&new_id(), &src, 5);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert!(matches!(last(&ends), SliceEnd::Failed(..) | SliceEnd::Sandbox), "{ends:?}");
    assert_eq!(attempts_of(&e), "", "a try was counted though no worker ever read the file");
}

#[test]
fn a_worker_that_cannot_start_in_a_later_slice_keeps_the_pages_made() {
    let e = setup("no-worker-later");
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    let src = e.drop("Planner.pdf", &pdffix::planner(9));
    let id = new_id();
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let end = run_slice(&id, &src);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(e.work_folders(), 1, "{end:?}: the work folder, 2 of 9 pages made, is kept");
    assert!(keep_work_folder(&id));
    // the next start goes on from page 3
    let ends = run_to_end(&id, &src, 20);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 9, blank: 0 }, "{ends:?}");
}

// ---- M10: the CPU budget counts the job's own workers ----

#[test]
fn the_cpu_budget_of_a_later_import_starts_with_its_own_probe() {
    let e = setup("cpu");
    let first = e.drop("First.pdf", &pdffix::planner(40));
    assert_eq!(last(&run_to_end(&new_id(), &first, 50)), &SliceEnd::Done { pages: 40, blank: 0 });
    // SAFETY: ru is written by getrusage before it is read.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut ru) };
    let spent = (ru.ru_utime.tv_sec as u64 + ru.ru_stime.tv_sec as u64) * 1000 + (ru.ru_utime.tv_usec as u64 + ru.ru_stime.tv_usec as u64) / 1000;
    let mut other = pdffix::planner(2);
    other.extend_from_slice(b"\n% another file\n%%EOF\n");
    let second = e.drop("Second.pdf", &other);
    let id = new_id();
    assert_eq!(run_slice(&id, &second), SliceEnd::More);
    let s = State::parse(&std::fs::read(format!("{}/notebooks/.import-{id}/pdf-import.ini", e.root)).unwrap()).unwrap();
    assert!(spent > 0);
    assert!(s.cpu_ms < spent, "the second import is charged {} ms, the first's workers used {spent} ms", s.cpu_ms);
}

// ---- M11: a writer that dies in a later slice is counted ----

#[test]
fn a_writer_that_dies_in_a_later_slice_ends_the_import_after_three() {
    let e = setup("uncounted");
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "60000");
    let src = e.drop("Planner.pdf", &pdffix::planner(4));
    let id = new_id();
    assert_eq!(run_slice(&id, &src), SliceEnd::More);
    std::env::set_var("GRIMOIRE_PDF_TEST_HANG_PAGE", "2");
    let bg = format!("{}/notebooks/.import-{id}/bg", e.root);
    let mut rounds = 0;
    while keep_work_folder(&id) && rounds < 6 {
        rounds += 1;
        let pid = fork_slice(&id, &src);
        // (page 1 is made, page 2 hangs: the writer is ended there; a slice that ends the import ends by itself)
        let t0 = std::time::Instant::now();
        while t0.elapsed().as_secs() < 20 && std::fs::read_dir(&bg).map(|d| d.flatten().filter(|f| f.file_name().to_string_lossy().ends_with(".png")).count()).unwrap_or(0) == 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        kill_group(pid);
    }
    std::env::remove_var("GRIMOIRE_PDF_TEST_HANG_PAGE");
    assert!(!keep_work_folder(&id), "{rounds} writers cut off, and the import still goes on at every scan");
    assert!(rounds <= 4, "{rounds} rounds");
    assert_eq!(e.work_folders(), 0);
    assert_eq!(tries_of(&e), vec![1], "one try counted for the whole import");
}

// ---- M22: links=1 ----

#[test]
fn an_imported_pdf_says_its_links_are_in() {
    let e = setup("links-latch");
    let src = e.drop("Planner.pdf", &pdffix::planner(2));
    let (id, ends) = import_all(&src);
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 2, blank: 0 });
    let ini = std::fs::read_to_string(format!("{}/notebooks/{id}/source.ini", e.root)).unwrap();
    assert!(ini.lines().any(|l| l == "links=1"), "KOReader's Open this book would ask for the links again: {ini}");
}

// ---- the Low items ----

#[test]
fn a_typed_name_of_spaces_falls_back_to_the_files_name() {
    i18n_set("en");
    assert_eq!(import_name("   ", "documents/Book.pdf", &[]), "Book");
}

#[test]
fn a_worker_that_says_a_pdf_has_no_pages_is_refused() {
    let e = setup("p0");
    let src = e.drop("Planner.pdf", &pdffix::planner(2));
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "2000");
    std::env::set_var("GRIMOIRE_WORK_EXE", stand_in_worker(&e, "no-pages", "read cmd <&5\nprintf 'P 0\\nK\\n' >&4\nsleep 1"));
    let ends = run_to_end(&new_id(), &src, 5);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert!(matches!(last(&ends), SliceEnd::Failed(..)), "{ends:?}");
    assert!(e.notebooks().is_empty(), "a notebook of no pages");
}

#[test]
fn a_done_mark_is_that_files_and_another_file_of_its_name_imports() {
    let e = setup("done-mark");
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    let src = e.drop("Planner.pdf", &pdffix::planner(2));
    // imported before, its file could not move: marked done with its size and mtime
    assert!(mark_write_public(&Mark { state: MarkState::Done, id: new_id(), src: src.clone(), size: 1, mtime: 2, ..Default::default() }));
    assert_eq!(grimoire::bgwork_template::import_scan(300.0), 1, "another file of that name is a new file");
}

// ---- 3.7: a worker that stalls right after a wake is tried again once, without a strike ----

/// What `f` writes to the log (stderr, where qlog goes in the tests).
fn logged(dir: &str, f: impl FnOnce()) -> String {
    use std::io::Write;
    let path = format!("{dir}/captured.log");
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns (stderr's, and the log file's).
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path.as_str()).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    f();
    let _ = std::io::stderr().flush();
    // SAFETY: stderr back as it was.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    std::fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn a_worker_that_stalls_right_after_a_wake_is_tried_again_without_a_strike() {
    let e = setup("woke");
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "400");
    std::env::set_var("GRIMOIRE_PDF_TEST_HANG_PAGE", "2");
    std::env::set_var("GRIMOIRE_PDF_TEST_WOKE", "1");
    let src = e.drop("Planner.pdf", &pdffix::planner(3));
    let mut ends = Vec::new();
    let log = logged(&e.root, || ends = run_to_end(&new_id(), &src, 20));
    std::env::remove_var("GRIMOIRE_PDF_TEST_WOKE");
    assert_eq!(last(&ends), &SliceEnd::Done { pages: 3, blank: 1 }, "{ends:?}");
    assert_eq!(log.matches("stalled right after a wake; tried again (no strike)").count(), 1, "{log}");
    assert!(log.contains("(strike 1)") && log.contains("(strike 2)"), "two strikes after the one free try: {log}");
}
