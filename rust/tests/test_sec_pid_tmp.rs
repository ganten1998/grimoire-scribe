//! Temporary files named after the process are made afresh (hardening round 2026-11): the mirror's
//! `<name>.pdf.<pid>.tmp` and the caches' `<name>.<pid>` (the previews, the kept drawings, their stamps,
//! thumb.src). What stands at such a name is removed, never opened or written, and nothing there holds a
//! write. The test calls the writers itself, so the process id in the names is its own.

mod testlib;

use grimoire::prelude::*;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use testlib::{never_waits, Checks};

/// A fresh data root for one test, with a file outside it: (the scratch folder, the root, the outside file).
fn setup(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let t = base.join(format!("grimoire-sec-pidtmp-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&t);
    let root = t.join("root");
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, 600, 800);
    let outside = t.join("outside.ini");
    std::fs::write(&outside, OUTSIDE).unwrap();
    (t, root, outside)
}

const OUTSIDE: &[u8] = b"marker=outside-only\n";

fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn is_plain(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_file())
}

fn wave(seed: u32, y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut p);
    let mut s = stroke_new(TOOL_PENCIL, &p, seed);
    for i in 0..30 {
        let q = PenSample { x: 60.0 + 12.0 * i as f32, y: y + 5.0 * (i % 5) as f32, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 7 };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

/// A notebook of two pages with ink.
fn book(name: &str) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_BLANK, &mut nb), 0);
    assert_eq!(store_add_page(&mut nb, 1, PAPER_BLANK), 0);
    for i in 0..2 {
        let mut p = page_new(600, 800, PAPER_BLANK);
        page_add(&mut p, wave(i as u32 + 1, 200.0 + 80.0 * i as f32));
        assert_eq!(store_save_page(&nb, i, &p), 0);
    }
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

/// The names in `dir` that end with `suffix`.
fn ending(dir: &Path, suffix: &str) -> Vec<String> {
    std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(suffix)).collect()).unwrap_or_default()
}

// The mirror's PDF is written into a file of its own, whatever stands at its temporary's name: a link there,
// or a second name of another file.
#[test]
fn a_mirror_is_never_written_through_its_temporary() {
    let (t, root, outside) = setup("mirror");
    let nb = book("Mirror");
    let pdfs = root.join("PDFs");
    std::fs::create_dir_all(&pdfs).unwrap();
    let pdf = pdfs.join("Mirror.pdf");
    let tmp = pdfs.join(format!("Mirror.pdf.{}.tmp", std::process::id()));
    let mut c = Checks::default();
    // a link at the temporary's name
    symlink(&outside, &tmp).unwrap();
    c.that(export_mirror_pdf(&nb, 300.0, text(&pdfs)) == 0, "the mirror is written with a link at its temporary's name");
    c.that(std::fs::read(&outside).unwrap() == OUTSIDE, "the file the link names is as it was");
    c.that(is_plain(&pdf) && std::fs::read(&pdf).is_ok_and(|d| d.starts_with(b"%PDF")), "the PDF is a plain file of its own");
    c.that(ending(&pdfs, ".tmp").is_empty(), "no temporary file is left");
    // a second name of the outside file at the temporary's name
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&pdf);
    std::fs::write(&outside, OUTSIDE).unwrap();
    std::fs::hard_link(&outside, &tmp).unwrap();
    c.that(export_mirror_pdf(&nb, 300.0, text(&pdfs)) == 0, "the mirror is written with another file's second name at its temporary's name");
    c.that(std::fs::read(&outside).unwrap() == OUTSIDE, "that file is as it was");
    c.that(is_plain(&pdf) && std::fs::read(&pdf).is_ok_and(|d| d.starts_with(b"%PDF")), "the PDF is a plain file of its own, again");
    c.that(ending(&pdfs, ".tmp").is_empty(), "no temporary file is left, again");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// A page's preview is written into a file of its own, whatever stands at its temporary's name.
#[test]
fn a_preview_is_never_written_through_its_temporary() {
    let (t, root, outside) = setup("preview");
    let nb = book("Preview");
    let cache = root.join("notebooks").join(nb.id.as_str()).join(".page-previews");
    std::fs::create_dir_all(&cache).unwrap();
    let mine = cache.join(format!("p1.pgm.{}", std::process::id()));
    symlink(&outside, &mine).unwrap();
    let mut page = None;
    assert_eq!(store_load_page(&nb, 0, 600, 800, &mut page), 0);
    let page = page.unwrap();
    let mut canvas = Canvas::default();
    assert_eq!(canvas_init(&mut canvas, page.w, page.h), 0);
    page_render(&mut canvas, &page, Rect::new(0, 0, page.w, page.h), 300.0);
    store_save_page_preview(&nb, nb.page_ids[0], &canvas);
    let mut c = Checks::default();
    c.that(std::fs::read(&outside).unwrap() == OUTSIDE, "the file the link names is as it was");
    c.that(is_plain(&cache.join("p1.pgm")), "the preview is a plain file of its own");
    c.that(store_page_preview_ok(&nb, nb.page_ids[0]), "and it is whole and current");
    c.that(std::fs::symlink_metadata(&mine).is_err(), "no temporary file is left");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// A FIFO at a temporary's name never holds a write: it is replaced by a plain file, at once.
// (never_waits: a write that did wait is let go after 30 s, and the test fails.)
#[test]
fn a_temporary_that_is_a_fifo_never_holds_a_write() {
    let (t, root, _) = setup("fifo");
    let nb = book("Card");
    let id = nb.id.as_str();
    let dir = root.join("notebooks").join(id);
    let mut thumb = Canvas::default();
    assert_eq!(canvas_init(&mut thumb, 60, 80), 0);
    assert_eq!(store_save_thumb(id, &thumb), 0);
    let mine = dir.join(format!("thumb.src.{}", std::process::id()));
    let c = std::ffi::CString::new(text(&mine)).unwrap();
    // SAFETY: mkfifo reads the NUL-terminated path we own.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
    never_waits(&mine, || store_note_thumb_source(id, "x"));
    assert_eq!(store_thumb_source(id).as_deref(), Some("x"));
    assert!(is_plain(&dir.join("thumb.src")));
    assert!(std::fs::symlink_metadata(&mine).is_err(), "no temporary file is left");
    let _ = std::fs::remove_dir_all(&t);
}
