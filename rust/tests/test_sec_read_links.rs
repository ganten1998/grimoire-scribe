//! Reads of the data folder take plain files only (hardening round 2026-11). A file of Grimoire's is read
//! when the entry at its name is a plain file: a link there is not followed, a folder or a FIFO reads as a
//! file that is not there, and no read waits. Folders on the way to a file may be links (the data root,
//! notebooks/, a platform's or a test's folder). Each test has a data root of its own.

mod testlib;

use grimoire::prelude::*;
use std::os::unix::fs::{symlink, FileTypeExt};
use std::path::{Path, PathBuf};
use testlib::{waits_on, Checks};

/// A fresh scratch folder for one test, named with the process id.
fn top(name: &str) -> PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let t = base.join(format!("grimoire-sec-read-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&t);
    std::fs::create_dir_all(&t).unwrap();
    t
}

fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// The store on a data root of the test's own (made here), with a small device.
fn store_at(root: &Path) {
    assert_eq!(store_init(text(root)), 0);
    store_set_device(300.0, 600, 800);
}

fn fifo_at(p: &Path) {
    let c = std::ffi::CString::new(text(p)).unwrap();
    // SAFETY: mkfifo reads the NUL-terminated path we own.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
}

fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())
}

fn is_plain(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_file())
}

/// The entries of `dir` whose names start with `prefix`.
fn named(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> =
        std::fs::read_dir(dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with(prefix)).map(|e| e.path()).collect();
    v.sort();
    v
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

/// A notebook of `pages` pages with ink: page 1 has one stroke, page 2 two, and so on.
fn book(name: &str, pages: usize, paper: PaperKind) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", paper, &mut nb), 0);
    for i in 1..pages {
        assert_eq!(store_add_page(&mut nb, i as i32, paper), 0);
    }
    for i in 0..pages {
        let mut p = page_new(600, 800, paper);
        for k in 0..=i {
            page_add(&mut p, wave((i * 10 + k) as u32 + 1, 100.0 + 60.0 * k as f32));
        }
        assert_eq!(store_save_page(&nb, i as i32, &p), 0);
    }
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

fn nb_dir(root: &Path, nb: &NotebookInfo) -> PathBuf {
    root.join("notebooks").join(nb.id.as_str())
}

fn load(nb: &NotebookInfo, index: i32) -> Box<Page> {
    let mut out = None;
    assert_eq!(store_load_page(nb, index, 600, 800, &mut out), 0);
    out.expect("a page")
}

// The read itself: the entry at the name must be a plain file. A folder on the way may be a link, a second
// name of a plain file is that file, and the system's own files are read as before.
#[test]
fn a_storage_read_takes_plain_files_only() {
    let t = top("plain");
    let root = t.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let outside = t.join("outside.ini");
    std::fs::write(&outside, b"marker=outside-only\n").unwrap();
    let mut c = Checks::default();
    let link = root.join("link.ini");
    symlink(&outside, &link).unwrap();
    c.that(read_file(text(&link)).is_none(), "a link to a file reads as no file");
    let ghost = root.join("ghost.ini");
    symlink(t.join("nowhere"), &ghost).unwrap();
    c.that(read_file(text(&ghost)).is_none(), "a dangling link reads as no file");
    let folder = root.join("folder.ini");
    std::fs::create_dir(&folder).unwrap();
    c.that(read_file(text(&folder)).is_none(), "a folder reads as no file");
    let plain = root.join("plain.ini");
    std::fs::write(&plain, b"name=Plain\n").unwrap();
    c.that(read_file(text(&plain)).as_deref() == Some(&b"name=Plain\n"[..]), "a plain file reads");
    // the same file through a folder that is a link
    let way = t.join("way");
    symlink(&root, &way).unwrap();
    c.that(read_file(text(&way.join("plain.ini"))).as_deref() == Some(&b"name=Plain\n"[..]), "a plain file behind a linked folder reads");
    // a second name of the file
    let hard = root.join("hard.ini");
    std::fs::hard_link(&plain, &hard).unwrap();
    c.that(read_file(text(&hard)).as_deref() == Some(&b"name=Plain\n"[..]), "a second name of a plain file reads");
    // the system's own files: links followed, as before
    c.that(read_system_file(text(&link)).as_deref() == Some(&b"marker=outside-only\n"[..]), "the system read follows a link");
    c.that(std::fs::read(&outside).unwrap() == b"marker=outside-only\n", "the file outside is as it was");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// No read of the data folder waits: a FIFO at a file's name reads as a file that is not there, at once.
// (waits_on: a call that did wait is let go after 30 s, and its check fails.)
#[test]
fn a_storage_read_never_waits_on_a_fifo() {
    let t = top("fifo");
    let root = t.join("root");
    store_at(&root);
    let mut c = Checks::default();
    let pipe = root.join("pipe.png");
    fifo_at(&pipe);
    let (r, waited) = waits_on(&pipe, || read_file(text(&pipe)));
    c.that(!waited && r.is_none(), "read_file of a FIFO is None, at once");
    let (r, waited) = waits_on(&pipe, || bgimage_read(text(&pipe), 1 << 20));
    c.that(!waited && r.is_none(), "bgimage_read of a FIFO is None, at once");
    // settings.ini: the language is read from it before the app starts
    let settings = root.join("settings.ini");
    fifo_at(&settings);
    let mut lang = String::from("?");
    let (_, waited) = waits_on(&settings, || settings_language(&mut lang, 16));
    c.that(!waited && lang.is_empty(), "settings_language with settings.ini a FIFO is \"\", at once");
    std::fs::remove_file(&settings).unwrap();
    // a notebook with no page size in its notebook.ini takes it from its page files: the listing reads them
    let dir = root.join("notebooks/nb-fifo");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("notebook.ini"), b"# Grimoire notebook\nname=Pipe\nfolder=\npaper=4\ncreated=5\nmodified=5\nlast_page=0\nnext_page_id=2\npages=1\n").unwrap();
    let page = dir.join("p1.qpg");
    fifo_at(&page);
    let mut list = Vec::new();
    let (rc, waited) = waits_on(&page, || store_list(&mut list));
    c.that(!waited && rc == 0, "store_list with a page file a FIFO lists, at once");
    assert_eq!(list.len(), 1);
    c.that((list[0].name.as_str(), list[0].page_w, list[0].page_h) == ("Pipe", 600, 800), "the notebook is listed at the device's size");
    // the page itself: a blank page, and the entry is set aside as a damaged page is
    let nb = list[0].clone();
    let mut out = None;
    let (rc, waited) = waits_on(&page, || store_load_page(&nb, 0, 600, 800, &mut out));
    c.that(!waited && rc == 0, "store_load_page of a FIFO opens, at once");
    c.that(out.is_some_and(|p| p.strokes.is_empty()), "as a blank page");
    c.that(std::fs::symlink_metadata(&page).is_err(), "nothing is left at the page's name");
    let aside = named(&dir, "p1.qpg.damaged-");
    c.that(aside.len() == 1 && std::fs::symlink_metadata(&aside[0]).unwrap().file_type().is_fifo(), "the entry itself was set aside");
    c.that(store_take_corrupt_count() == 1, "and counted as a damaged page is");
    // what a notebook's card was drawn from, read from thumb.src beside it
    let src = dir.join("thumb.src");
    fifo_at(&src);
    let (r, waited) = waits_on(&src, || store_thumb_source("nb-fifo"));
    c.that(!waited && r.is_none(), "store_thumb_source with thumb.src a FIFO is None, at once");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// An entry at a page's name that is no plain file is set aside as a damaged page is (the entry itself, never
// what it may point at), and the page then saves.
#[test]
fn a_page_entry_that_is_no_plain_file_is_set_aside() {
    let t = top("aside");
    let root = t.join("root");
    store_at(&root);
    let nb = book("Three", 3, PAPER_BLANK);
    let dir = nb_dir(&root, &nb);
    let (p1, p2) = (dir.join(format!("p{}.qpg", nb.page_ids[0])), dir.join(format!("p{}.qpg", nb.page_ids[1])));
    // page 1: a folder at its name; page 2: a link to a copy of the good page, kept outside
    let outside = t.join("outside.qpg");
    std::fs::copy(&p2, &outside).unwrap();
    let good = std::fs::read(&outside).unwrap();
    std::fs::remove_file(&p1).unwrap();
    std::fs::create_dir(&p1).unwrap();
    std::fs::remove_file(&p2).unwrap();
    symlink(&outside, &p2).unwrap();
    assert_eq!(store_take_corrupt_count(), 0);
    let mut c = Checks::default();
    for (index, path) in [(0, &p1), (1, &p2)] {
        let page = load(&nb, index);
        c.that(page.strokes.is_empty(), &format!("page {} opens blank", index + 1));
        c.that(std::fs::symlink_metadata(path).is_err(), &format!("nothing is left at page {}'s name", index + 1));
    }
    let (a1, a2) = (named(&dir, &format!("p{}.qpg.damaged-", nb.page_ids[0])), named(&dir, &format!("p{}.qpg.damaged-", nb.page_ids[1])));
    c.that(a1.len() == 1 && std::fs::symlink_metadata(&a1[0]).unwrap().is_dir(), "the folder is set aside, as it was");
    c.that(a2.len() == 1 && is_link(&a2[0]), "the link is set aside, as it was");
    c.that(store_take_corrupt_count() == 2, "each is counted, as a damaged page is");
    // each page then saves, as a plain file of its own
    for (index, path) in [(0, &p1), (1, &p2)] {
        let mut page = page_new(600, 800, PAPER_BLANK);
        page_add(&mut page, wave(90 + index as u32, 300.0));
        c.that(store_save_page(&nb, index, &page) == 0 && is_plain(path), &format!("page {} then saves as a plain file", index + 1));
        c.that(load(&nb, index).strokes.len() == 1, &format!("page {} reads back what was saved", index + 1));
    }
    c.that(std::fs::read(&outside).unwrap() == good, "the file outside is as it was");
    // the third page, untouched, has its ink
    c.that(load(&nb, 2).strokes.len() == 3, "the untouched page has its ink");
    c.that(store_take_corrupt_count() == 0, "and nothing more is counted");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// A guard, which passes before and after the hardening round: a data folder that sits behind a link, with
// notebooks/ and the mirror's folder links too, works as it always did. It pins the promise to linked setups
// (a platform's folders, a user's choice, the tests' own folders): it fails if a read or mkdir_p is ever made to
// refuse a folder on the way that is a link.
#[test]
fn a_data_folder_behind_a_link_still_works() {
    let t = top("linked");
    let (real, shelf, pdfs) = (t.join("real-root"), t.join("shelf"), t.join("pdfs"));
    for d in [&real, &shelf, &pdfs] {
        std::fs::create_dir_all(d).unwrap();
    }
    symlink(&shelf, real.join("notebooks")).unwrap();
    symlink(&pdfs, real.join("PDFs")).unwrap();
    let root = t.join("root");
    symlink(&real, &root).unwrap();
    store_at(&root);
    assert!(is_link(&root) && is_link(&real.join("notebooks")), "store_init left the links as they are");
    // a notebook, and a page of it
    let nb = book("Linked", 1, PAPER_BLANK);
    assert!(is_plain(&shelf.join(nb.id.as_str()).join("notebook.ini")), "the notebook is in the linked folder");
    assert_eq!(load(&nb, 0).strokes.len(), 1);
    // the settings
    let mut s = Settings::default();
    settings_default(&mut s);
    s.mirror_pdf = !s.mirror_pdf;
    let want = s.mirror_pdf;
    assert_eq!(settings_save(&s), 0);
    let mut back = Settings::default();
    assert_eq!(settings_load(&mut back), 0);
    assert_eq!(back.mirror_pdf, want);
    assert!(is_plain(&real.join("settings.ini")));
    // the Library's listing
    let mut list = Vec::new();
    assert_eq!(store_list(&mut list), 0);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name.as_str(), "Linked");
    // a page's preview
    let page = load(&nb, 0);
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, page.w, page.h), 0);
    page_render(&mut c, &page, Rect::new(0, 0, page.w, page.h), 300.0);
    store_save_page_preview(&nb, nb.page_ids[0], &c);
    assert!(store_page_preview_ok(&nb, nb.page_ids[0]));
    assert!(is_plain(&shelf.join(nb.id.as_str()).join(".page-previews/p1.pgm")));
    // the notebook's PDF, into a mirror folder that is a link
    assert_eq!(export_mirror_pdf(&nb, 300.0, text(&root.join("PDFs"))), 0);
    assert!(std::fs::read(pdfs.join("Linked.pdf")).unwrap().starts_with(b"%PDF"));
    let _ = std::fs::remove_dir_all(&t);
}

/// Every plain file under `dir` (links are not followed), with its bytes.
fn plain_files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let ft = e.file_type().unwrap();
        if ft.is_dir() {
            out.extend(plain_files(&e.path()));
        } else if ft.is_file() {
            out.push((e.path(), std::fs::read(e.path()).unwrap()));
        }
    }
    out
}

fn holds(bytes: &[u8], what: &[u8]) -> bool {
    bytes.windows(what.len()).any(|w| w == what)
}

/// `text` (whole lines), then one comment line that makes it `len` bytes long.
fn padded(text: &[u8], len: usize) -> Vec<u8> {
    let mut v = text.to_vec();
    assert!(v.len() + 2 <= len);
    v.push(b'#');
    v.resize(len - 1, b'x');
    v.push(b'\n');
    v
}

const MIB: usize = 1 << 20;

// A notebook.ini that is a link is not read: the notebook is left out of the Library, and nothing in its
// folder is written, moved or removed.
#[test]
fn a_linked_notebook_ini_is_not_read_or_saved_back() {
    let t = top("nbini");
    let root = t.join("root");
    store_at(&root);
    let nb = book("Real", 1, PAPER_BLANK);
    let dir = nb_dir(&root, &nb);
    let ini = dir.join("notebook.ini");
    // (no page size: a notebook.ini that reads without one is written again with it)
    let outside = t.join("outside.ini");
    let text_outside = b"# Grimoire notebook\nname=Outside\nmarker=outside-only\npaper=4\ncreated=5\nmodified=5\nlast_page=0\nnext_page_id=2\npages=1\n";
    std::fs::write(&outside, text_outside).unwrap();
    std::fs::remove_file(&ini).unwrap();
    symlink(&outside, &ini).unwrap();
    let before = plain_files(&dir);
    let mut list = Vec::new();
    assert_eq!(store_list(&mut list), 0);
    assert!(list.is_empty(), "listed: {:?}", list.iter().map(|n| n.name.as_str().to_string()).collect::<Vec<_>>());
    assert_eq!(store_page_totals(), Some((0, 0)));
    let mut info = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut info), -1);
    assert!(is_link(&ini), "the entry is as it was");
    let after = plain_files(&dir);
    assert!(after == before, "the notebook's files are as they were");
    assert!(!after.iter().any(|(_, b)| holds(b, b"outside-only")), "nothing of the file outside is in the notebook's folder");
    assert_eq!(std::fs::read(&outside).unwrap(), text_outside);
    let _ = std::fs::remove_dir_all(&t);
}

// A settings.ini that is a link is not read: the defaults are used, and the entry is kept as
// settings.ini.unread when the settings are next saved. The first one kept stays.
#[test]
fn a_linked_settings_ini_is_not_read_and_is_kept() {
    let t = top("setini");
    let root = t.join("root");
    store_at(&root);
    let (ini, aside) = (root.join("settings.ini"), root.join("settings.ini.unread"));
    let outside = t.join("outside.ini");
    let text_outside = b"# Grimoire settings\nmarker=outside-only\nlanguage=es\n";
    std::fs::write(&outside, text_outside).unwrap();
    symlink(&outside, &ini).unwrap();
    let mut c = Checks::default();
    let mut lang = String::from("?");
    settings_language(&mut lang, 16);
    c.that(lang.is_empty(), "the language is not read from it");
    let mut s = Settings::default();
    c.that(settings_load(&mut s) == 1, "settings_load says the settings were not read");
    c.that(settings_save(&s) == 0 && is_plain(&ini), "the settings then save, as a plain file");
    c.that(!holds(&std::fs::read(&ini).unwrap(), b"outside-only"), "nothing of the file outside is saved back");
    c.that(is_link(&aside) && std::fs::read_link(&aside).is_ok_and(|to| to == outside), "the entry is kept as settings.ini.unread, as it was");
    c.that(std::fs::read(&outside).unwrap() == text_outside, "the file outside is as it was");
    // (what was saved reads)
    c.that(settings_load(&mut s) == 0, "what was saved reads");
    // a second round, with another link: the first one kept stays
    let second = t.join("second.ini");
    std::fs::write(&second, b"marker=second-only\n").unwrap();
    std::fs::remove_file(&ini).unwrap();
    symlink(&second, &ini).unwrap();
    c.that(settings_load(&mut s) == 1, "second round: not read");
    c.that(settings_save(&s) == 0 && is_plain(&ini) && !holds(&std::fs::read(&ini).unwrap(), b"-only"), "second round: saved, with nothing of either file");
    c.that(is_link(&aside) && std::fs::read_link(&aside).is_ok_and(|to| to == outside), "settings.ini.unread is still the first");
    c.that(std::fs::read(&second).unwrap() == b"marker=second-only\n" && std::fs::read(&outside).unwrap() == text_outside, "both files outside are as they were");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// settings.ini and notebook.ini are read up to 1 MiB, to the byte. A longer settings.ini is kept aside at the
// next save; a notebook with a longer notebook.ini is left out of the Library, its file untouched.
#[test]
fn an_ini_over_the_limit_is_not_read() {
    let t = top("limit");
    let root = t.join("root");
    store_at(&root);
    let (ini, aside) = (root.join("settings.ini"), root.join("settings.ini.unread"));
    let mut s = Settings::default();
    settings_default(&mut s);
    let lines = format!("# Grimoire settings\nmirror_pdf={}\n", !s.mirror_pdf as i32);
    std::fs::write(&ini, padded(lines.as_bytes(), MIB)).unwrap();
    let mut c = Checks::default();
    let mut got = Settings::default();
    c.that(settings_load(&mut got) == 0 && got.mirror_pdf != s.mirror_pdf, "a settings.ini of exactly 1 MiB reads");
    let long = padded(lines.as_bytes(), MIB + 1);
    std::fs::write(&ini, &long).unwrap();
    c.that(settings_load(&mut got) == 1 && got.mirror_pdf == s.mirror_pdf, "one byte more is not read: the defaults are used");
    c.that(settings_save(&got) == 0, "the settings then save");
    c.that(std::fs::read(&aside).is_ok_and(|kept| kept == long), "the long file is kept as settings.ini.unread, byte for byte");
    c.that(is_plain(&ini) && std::fs::metadata(&ini).unwrap().len() < 100_000, "and settings.ini is this build's");
    // notebook.ini
    let nb = book("Long", 1, PAPER_BLANK);
    let nbini = nb_dir(&root, &nb).join("notebook.ini");
    let wrote = std::fs::read(&nbini).unwrap();
    std::fs::write(&nbini, padded(&wrote, MIB)).unwrap();
    let mut info = NotebookInfo::default();
    c.that(store_load_info(nb.id.as_str(), &mut info) == 0 && info.name.as_str() == "Long", "a notebook.ini of exactly 1 MiB reads");
    let long = padded(&wrote, MIB + 1);
    std::fs::write(&nbini, &long).unwrap();
    c.that(store_load_info(nb.id.as_str(), &mut info) == -1, "one byte more is not read");
    let mut list = Vec::new();
    c.that(store_list(&mut list) == 0 && list.is_empty(), "and the notebook is left out of the Library");
    c.that(std::fs::read(&nbini).unwrap() == long, "its notebook.ini is as it was");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

/// errno as the C library has it for this thread.
fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

// A picture is read when the entry at its name is a plain file, of at most the size its caller takes.
#[test]
fn a_picture_is_never_read_through_a_link() {
    let t = top("picture");
    let root = t.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let outside = t.join("outside.png");
    let png: Vec<u8> = b"\x89PNG\r\n\x1a\n".iter().copied().chain((0..200u8).cycle().take(1000)).collect();
    std::fs::write(&outside, &png).unwrap();
    let link = root.join("000001.png");
    symlink(&outside, &link).unwrap();
    assert_eq!(bgimage_read(text(&link), 1 << 20), None, "a link to a picture");
    assert!(bgimage_read(text(&outside), 1 << 20).as_deref() == Some(&png[..]), "the file itself");
    assert!(bgimage_read(text(&outside), png.len()).as_deref() == Some(&png[..]), "at its size");
    // one byte over what the caller takes
    assert_eq!(bgimage_read(text(&outside), png.len() - 1), None);
    assert_eq!(errno(), libc::EFBIG);
    // a folder, and nothing at all, read as a missing file
    let folder = root.join("000002.png");
    std::fs::create_dir(&folder).unwrap();
    assert_eq!(bgimage_read(text(&folder), 1 << 20), None);
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(bgimage_read(text(&root.join("000003.png")), 1 << 20), None);
    assert_eq!(errno(), libc::ENOENT);
    assert_eq!(std::fs::read(&outside).unwrap(), png);
    let _ = std::fs::remove_dir_all(&t);
}

/// A file moved out of the data folder to `outside`, with a link to it left at its name.
fn move_out_and_link(file: &Path, outside: &Path) {
    std::fs::rename(file, outside).unwrap();
    symlink(outside, file).unwrap();
}

fn canvas_of(nb: &NotebookInfo, index: i32) -> Canvas {
    let page = load(nb, index);
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, page.w, page.h), 0);
    page_render(&mut c, &page, Rect::new(0, 0, page.w, page.h), 300.0);
    c
}

// The reads of a file's first bytes (a page's paper, a preview's header, a picture's hash and colour type, the
// blank pages a batch added) take a plain file at the name itself, as the whole-file read does.
#[test]
fn header_reads_take_plain_files_only() {
    let t = top("headers");
    let root = t.join("root");
    store_at(&root);
    let mut c = Checks::default();
    let nb = book("Headers", 2, PAPER_BLANK);
    let dir = nb_dir(&root, &nb);
    // a page's paper, read from its file's start
    let mut grid = page_new(600, 800, PAPER_GRID);
    page_add(&mut grid, wave(7, 200.0));
    assert_eq!(store_save_page(&nb, 1, &grid), 0);
    c.that(store_page_paper(&nb, 1) == PAPER_GRID, "a page's paper is read from its file");
    move_out_and_link(&dir.join(format!("p{}.qpg", nb.page_ids[1])), &t.join("outside.qpg"));
    c.that(store_page_paper(&nb, 1) == PAPER_BLANK, "a page file that is a link gives the notebook's paper");
    // a preview's header
    store_save_page_preview(&nb, nb.page_ids[0], &canvas_of(&nb, 0));
    c.that(store_page_preview_ok(&nb, nb.page_ids[0]), "a preview just saved is whole and current");
    move_out_and_link(&dir.join(".page-previews/p1.pgm"), &t.join("outside.pgm"));
    c.that(!store_page_preview_ok(&nb, nb.page_ids[0]), "a preview that is a link is not");
    // a picture's hash
    let picture = t.join("outside.png");
    std::fs::write(&picture, vec![7u8; 5000]).unwrap();
    let (mut hash, mut len) = (0u64, 0usize);
    c.that(bgimage_content_hash_file(text(&picture), &mut hash, &mut len) == 0 && len == 5000, "a plain file is hashed");
    let link = root.join("linked.png");
    symlink(&picture, &link).unwrap();
    c.that(bgimage_content_hash_file(text(&link), &mut hash, &mut len) == -1, "a link is not hashed");
    c.that(bgimage_content_hash_file(text(&dir), &mut hash, &mut len) == -1, "a folder is not hashed");
    // a kept picture's colour type, from its PNG header (26 bytes: the signature, IHDR, the colour type last)
    let (mut bgdir, mut file) = (String::new(), String::new());
    assert_eq!(bgimage_dir(nb.id.as_str(), &mut bgdir, 700), 0);
    std::fs::create_dir_all(&bgdir).unwrap();
    bgimage_file(&bgdir, 1, &mut file, 800);
    let mut head = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    head.extend_from_slice(&600u32.to_be_bytes());
    head.extend_from_slice(&800u32.to_be_bytes());
    head.extend_from_slice(&[8, 2]);
    assert_eq!(head.len(), 26);
    std::fs::write(&file, &head).unwrap();
    let paper = paper_image(PAPER_ROLE_TEMPLATE, 1);
    c.that(bgimage_channels(nb.id.as_str(), paper) == 3, "a colour picture's header is read from its file");
    move_out_and_link(Path::new(&file), &t.join("outside-colour.png"));
    c.that(bgimage_channels(nb.id.as_str(), paper) == 1, "a picture that is a link reads as no colour picture");
    // the blank pages a batch added: untouched only as plain files, and written again only where nothing stands
    let mut nb2 = book("Added", 1, PAPER_BLANK);
    let dir2 = nb_dir(&root, &nb2);
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb2, 1, 2, PAPER_BLANK, &mut undo), 0);
    let u = undo.unwrap();
    let added = dir2.join(format!("p{}.qpg", u.added[0]));
    let blank = t.join("outside-blank.qpg");
    c.that(store_added_pages_untouched(&nb2, &u), "pages just added are untouched");
    move_out_and_link(&added, &blank);
    c.that(!store_added_pages_untouched(&nb2, &u), "an added page that is a link is not counted as untouched");
    std::fs::remove_file(&added).unwrap();
    std::fs::rename(&blank, &added).unwrap();
    assert_eq!(store_undo_add_pages(&mut nb2, &u), 0);
    assert!(std::fs::symlink_metadata(&added).is_err());
    std::fs::write(&blank, &u.blank).unwrap();
    symlink(&blank, &added).unwrap();
    let was = nb2.page_ids.clone();
    c.that(store_redo_add_pages(&mut nb2, &u) == -1 && nb2.page_ids == was, "the redo is refused with a link at an added page's name");
    c.that(is_link(&added) && std::fs::read(&blank).unwrap() == u.blank, "and the link and its file are as they were");
    std::fs::remove_file(&added).unwrap();
    c.that(store_redo_add_pages(&mut nb2, &u) == 0 && is_plain(&added), "with nothing at the name the redo writes the page");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// The sweep of a previews job's leftovers works only in a .page-previews that is a folder of the notebook's own.
#[test]
fn the_preview_sweep_stays_in_its_own_folder() {
    let t = top("sweep");
    let root = t.join("root");
    store_at(&root);
    let pid = 4242;
    let mut c = Checks::default();
    // a real folder: this process id's leftovers go, another's stay
    let nb = book("Own", 1, PAPER_BLANK);
    let cache = nb_dir(&root, &nb).join(".page-previews");
    std::fs::create_dir_all(&cache).unwrap();
    for f in ["x.4242", "x.4242.tmp", "x.777", "p1.pgm"] {
        std::fs::write(cache.join(f), b"x").unwrap();
    }
    store_cleanup_preview_work(nb.id.as_str(), pid);
    let mut left: Vec<String> = std::fs::read_dir(&cache).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    left.sort();
    c.that(left == ["p1.pgm", "x.777"], "in the notebook's own folder the job's leftovers are removed");
    // a .page-previews that is a link: nothing behind it is removed
    let other = book("Linked", 1, PAPER_BLANK);
    let outside = t.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    for f in ["keep.4242", "keep.4242.tmp"] {
        std::fs::write(outside.join(f), b"not Grimoire's").unwrap();
    }
    symlink(&outside, nb_dir(&root, &other).join(".page-previews")).unwrap();
    store_cleanup_preview_work(other.id.as_str(), pid);
    c.that(outside.join("keep.4242").exists() && outside.join("keep.4242.tmp").exists(), "behind a linked .page-previews nothing is removed");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}

// A notebook's picture index (bg/index.ini) is read up to 1 MiB, to the byte: a longer one is there but does
// not read, so nothing is saved over it.
#[test]
fn the_picture_index_has_a_limit() {
    let t = top("index");
    let bg = t.join("bg");
    std::fs::create_dir_all(&bg).unwrap();
    let mut textv = b"# Grimoire background pictures: id=key (docs/FORMAT.md)\n000001=00000000000000aa\n#".to_vec();
    textv.resize(MIB - 1, b'x');
    textv.push(b'\n');
    std::fs::write(bg.join("index.ini"), &textv).unwrap();
    let mut c = Checks::default();
    let mut ix = BgIndex::default();
    c.that(bgindex_load(text(&bg), &mut ix) == 0 && ix.e.len() == 1 && ix.e[0].id == 1 && ix.e[0].key == 0xaa, "an index of exactly 1 MiB reads");
    textv.insert(textv.len() - 1, b'x');
    std::fs::write(bg.join("index.ini"), &textv).unwrap();
    c.that(bgindex_load(text(&bg), &mut ix) == -1 && ix.e.is_empty(), "one byte more is there but does not read");
    c.done();
    let _ = std::fs::remove_dir_all(&t);
}
