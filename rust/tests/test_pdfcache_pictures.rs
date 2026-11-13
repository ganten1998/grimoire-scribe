//! The notebook PDF's cache and the pages' pictures (2.1.0-beta.2 fix round, H4 and the stamp): the cache can
//! only make a PDF faster, never different. A page drawn while its picture was not there (a template's,
//! photo's or KOReader page's picture is made by a background job after the page file names it) is never
//! kept; a record names its picture file as it was, so a picture replaced is drawn again; a picture job's end
//! takes the notebook's records with it; a missing page file is drawn on the notebook's paper of today.
//! One test function: the store root is process-wide.

use grimoire::prelude::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("pdfcache-pictures-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn wave(seed: u32, y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut p);
    let mut s = stroke_new(TOOL_PENCIL, &p, seed);
    for i in 0..30 {
        let q = PenSample { x: 100.0 + 20.0 * i as f32, y: y + 6.0 * (i % 5) as f32, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 7 };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

fn book(name: &str, pages: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_NARROW_NM, &mut nb), 0);
    for i in 1..pages {
        assert_eq!(store_add_page(&mut nb, i as i32, PAPER_NARROW_NM), 0);
    }
    for i in 0..pages {
        let mut p = page_new(800, 1000, PAPER_NARROW_NM);
        page_add(&mut p, wave(i as u32 + 1, 200.0 + 40.0 * i as f32));
        assert_eq!(store_save_page(&nb, i as i32, &p), 0);
    }
    reload(&nb)
}

fn reload(nb: &NotebookInfo) -> NotebookInfo {
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

fn full(nb: &NotebookInfo) -> Vec<u8> {
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf_to(nb, 300.0, &mut BufSink(&mut b), PdfCacheMode::Off, None), 0);
    b.data
}

fn mirror(nb: &NotebookInfo) -> (Vec<u8>, u32) {
    let dir = format!("{}/PDFs", root().display());
    let before = pdf_pages_rendered();
    assert_eq!(export_mirror_pdf(nb, 300.0, &dir), 0);
    let drawn = pdf_pages_rendered() - before;
    let mut path = String::new();
    export_mirror_path(nb, &dir, &mut path, 1000);
    (std::fs::read(&path).unwrap(), drawn)
}

fn records(nb: &NotebookInfo) -> usize {
    std::fs::read_dir(PathBuf::from(pdf_cache_root()).join(nb.id.as_str())).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".z")).count()).unwrap_or(0)
}

fn picture_file(nb: &NotebookInfo, id: u32) -> String {
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    assert_eq!(mkdir_p(&bg), 0);
    let mut f = String::new();
    bgimage_file(&bg, id, &mut f, 800);
    f
}

fn gray_picture(path: &str, w: i32, h: i32, seed: i32) {
    let mut px = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            px[(y * w + x) as usize] = ((x * 7 + y * 3 + seed) % 200) as u8;
        }
    }
    assert_eq!(bgimage_write_png(path, &BgPicture { px, w, h, ch: 1 }), 0);
}

#[test]
fn the_cache_never_keeps_a_page_without_its_picture() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, 800, 1000);
    a_picture_made_after_the_page_was_drawn_reaches_the_pdf();
    a_picture_replaced_is_drawn_again();
    a_picture_jobs_end_takes_the_records();
    a_missing_page_follows_the_notebooks_paper();
    let _ = std::fs::remove_dir_all(&r);
}

/// template_install gives a page its image paper at once and queues the job that makes bg/<id>.png; a mirror
/// that draws the page in between draws it on white paper. That page is the PDF of that run only.
fn a_picture_made_after_the_page_was_drawn_reaches_the_pdf() {
    let mut nb = book("Late picture", 3);
    let paper = paper_image(PAPER_ROLE_TEMPLATE, 0x21);
    let mut p = page_new(800, 1000, paper);
    page_add(&mut p, wave(3, 500.0));
    assert_eq!(store_save_page(&nb, 1, &p), 0);
    nb = reload(&nb);
    let (bare, drawn) = mirror(&nb);
    assert_eq!(drawn, 3, "the first mirror draws every page (page 2 without its picture: it is not there yet)");
    assert_eq!(records(&nb), 2, "the page drawn without its picture is not kept");
    // the background job makes the picture; the UI forgets the failures (app.rs: bgimage_retry_missing)
    gray_picture(&picture_file(&nb, 0x21), 800, 1000, 0);
    bgimage_forget(None);
    bgimage_retry_missing();
    let want = full(&nb);
    assert_ne!(want, bare, "the picture changes what the page looks like");
    let (again, drawn) = mirror(&nb);
    assert_eq!(drawn, 1, "the next mirror draws that page again, and only it");
    assert!(again == want, "the mirror is the full render, the page with its picture");
    // and now it is kept: the share server's download (read only) finds it
    let before = pdf_pages_rendered();
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf(&nb, 300.0, &mut b), 0);
    assert_eq!((b.data == want, pdf_pages_rendered() - before), (true, 0), "the share download is the full render, from the cache");
}

/// A picture file written again under its id (a KOReader page sent again, a picture made again after it
/// went missing) with the same channel count: the record names the file's size and time, so it no longer fits.
fn a_picture_replaced_is_drawn_again() {
    let mut nb = book("Replaced", 2);
    let f = picture_file(&nb, 0x22);
    gray_picture(&f, 800, 1000, 0);
    let paper = paper_image(PAPER_ROLE_DOCUMENT, 0x22);
    let mut p = page_new(800, 1000, paper);
    page_add(&mut p, wave(9, 400.0));
    assert_eq!(store_save_page(&nb, 0, &p), 0);
    nb = reload(&nb);
    let (first, drawn) = mirror(&nb);
    assert_eq!((first == full(&nb), drawn), (true, 2));
    // another gray picture of another size of file, a second later
    std::thread::sleep(std::time::Duration::from_millis(1100));
    gray_picture(&f, 800, 1000, 77);
    bgimage_forget(None);
    let want = full(&nb);
    assert_ne!(want, first);
    let (again, drawn) = mirror(&nb);
    assert_eq!(drawn, 1, "the page on the replaced picture is drawn again");
    assert!(again == want, "and the PDF is the full render");
}

/// The end of a picture job for a notebook (bg_template, in its child) takes the notebook's records with it.
fn a_picture_jobs_end_takes_the_records() {
    let nb = book("Forget", 3);
    mirror(&nb);
    assert_eq!(records(&nb), 3);
    pdf_cache_forget(nb.id.as_str());
    assert_eq!(records(&nb), 0, "a picture job's end: the notebook's records are gone");
    let other = book("Kept", 2);
    mirror(&other);
    pdf_cache_forget(nb.id.as_str());
    assert_eq!(records(&other), 2, "another notebook's stay");
    let (pdf, drawn) = mirror(&nb);
    assert_eq!((pdf == full(&nb), drawn), (true, 3));
}

/// A page whose file is missing (or does not read) is drawn as a blank page of the notebook's paper
/// (store_read_page): the notebook's paper is in the stamp, so a notebook given another paper draws again.
fn a_missing_page_follows_the_notebooks_paper() {
    let mut nb = book("Missing page", 2);
    let file = format!("{}/notebooks/{}/p{}.qpg", root().display(), nb.id.as_str(), nb.page_ids[1]);
    std::fs::remove_file(&file).unwrap();
    let (first, _) = mirror(&nb);
    assert!(first == full(&nb));
    nb.paper = PAPER_BLANK;
    assert_eq!(store_save_info(&mut nb), 0);
    nb = reload(&nb);
    let want = full(&nb);
    assert_ne!(want, first, "the missing page shows the notebook's new paper");
    let (again, drawn) = mirror(&nb);
    assert_eq!(drawn, 2, "the pages are drawn again (the notebook's paper is in every stamp: a damaged page file is drawn on it too)");
    assert!(again == want);
}
