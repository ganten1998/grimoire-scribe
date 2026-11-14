//! Reads of the data folder take plain files only (hardening round 2026-11). A file of Grimoire's is read
//! when the entry at its name is a plain file: a link there is not followed, a folder or a FIFO reads as a
//! file that is not there, and no read waits. Folders on the way to a file may be links (the data root,
//! notebooks/, a platform's or a test's folder). Each test has a data root of its own.

mod testlib;

use grimoire::prelude::*;
use std::os::unix::fs::{symlink, FileTypeExt};
use std::path::{Path, PathBuf};
use testlib::never_waits;

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
    let link = root.join("link.ini");
    symlink(&outside, &link).unwrap();
    assert_eq!(read_file(text(&link)), None, "a link to a file");
    let ghost = root.join("ghost.ini");
    symlink(t.join("nowhere"), &ghost).unwrap();
    assert_eq!(read_file(text(&ghost)), None, "a dangling link");
    let folder = root.join("folder.ini");
    std::fs::create_dir(&folder).unwrap();
    assert_eq!(read_file(text(&folder)), None, "a folder");
    let plain = root.join("plain.ini");
    std::fs::write(&plain, b"name=Plain\n").unwrap();
    assert_eq!(read_file(text(&plain)).as_deref(), Some(&b"name=Plain\n"[..]));
    // the same file through a folder that is a link
    let way = t.join("way");
    symlink(&root, &way).unwrap();
    assert_eq!(read_file(text(&way.join("plain.ini"))).as_deref(), Some(&b"name=Plain\n"[..]));
    // a second name of the file
    let hard = root.join("hard.ini");
    std::fs::hard_link(&plain, &hard).unwrap();
    assert_eq!(read_file(text(&hard)).as_deref(), Some(&b"name=Plain\n"[..]));
    // the system's own files: links followed, as before
    assert_eq!(read_system_file(text(&link)).as_deref(), Some(&b"marker=outside-only\n"[..]));
    assert_eq!(std::fs::read(&outside).unwrap(), b"marker=outside-only\n");
    let _ = std::fs::remove_dir_all(&t);
}

// No read of the data folder waits: a FIFO at a file's name reads as a file that is not there, at once.
// (never_waits: a call that did wait is let go after 30 s, and the test fails.)
#[test]
fn a_storage_read_never_waits_on_a_fifo() {
    let t = top("fifo");
    let root = t.join("root");
    store_at(&root);
    let pipe = root.join("pipe.png");
    fifo_at(&pipe);
    assert_eq!(never_waits(&pipe, || read_file(text(&pipe))), None);
    // settings.ini: the language is read from it before the app starts
    let settings = root.join("settings.ini");
    fifo_at(&settings);
    let mut lang = String::from("?");
    never_waits(&settings, || settings_language(&mut lang, 16));
    assert_eq!(lang, "");
    std::fs::remove_file(&settings).unwrap();
    // a notebook with no page size in its notebook.ini takes it from its page files: the listing reads them
    let dir = root.join("notebooks/nb-fifo");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("notebook.ini"), b"# Grimoire notebook\nname=Pipe\nfolder=\npaper=4\ncreated=5\nmodified=5\nlast_page=0\nnext_page_id=2\npages=1\n").unwrap();
    let page = dir.join("p1.qpg");
    fifo_at(&page);
    let mut list = Vec::new();
    assert_eq!(never_waits(&page, || store_list(&mut list)), 0);
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].name.as_str(), list[0].page_w, list[0].page_h), ("Pipe", 600, 800), "listed at the device's size");
    // the page itself: a blank page, and the entry is set aside as a damaged page is
    let nb = list[0].clone();
    let mut out = None;
    assert_eq!(never_waits(&page, || store_load_page(&nb, 0, 600, 800, &mut out)), 0);
    assert!(out.is_some_and(|p| p.strokes.is_empty()), "a blank page");
    assert!(std::fs::symlink_metadata(&page).is_err(), "nothing is left at the page's name");
    let aside = named(&dir, "p1.qpg.damaged-");
    assert_eq!(aside.len(), 1, "{aside:?}");
    assert!(std::fs::symlink_metadata(&aside[0]).unwrap().file_type().is_fifo(), "the entry itself was set aside");
    assert_eq!(store_take_corrupt_count(), 1);
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
    for (index, path) in [(0, &p1), (1, &p2)] {
        let page = load(&nb, index);
        assert!(page.strokes.is_empty(), "page {} opens blank", index + 1);
        assert!(std::fs::symlink_metadata(path).is_err(), "nothing is left at page {}'s name", index + 1);
    }
    let (a1, a2) = (named(&dir, &format!("p{}.qpg.damaged-", nb.page_ids[0])), named(&dir, &format!("p{}.qpg.damaged-", nb.page_ids[1])));
    assert_eq!((a1.len(), a2.len()), (1, 1), "one entry set aside for each: {a1:?} {a2:?}");
    assert!(std::fs::symlink_metadata(&a1[0]).unwrap().is_dir(), "the folder, as it was");
    assert!(is_link(&a2[0]), "the link, as it was");
    assert_eq!(store_take_corrupt_count(), 2, "each is counted, as a damaged page is");
    // each page then saves, as a plain file of its own
    for (index, path) in [(0, &p1), (1, &p2)] {
        let mut page = page_new(600, 800, PAPER_BLANK);
        page_add(&mut page, wave(90 + index as u32, 300.0));
        assert_eq!(store_save_page(&nb, index, &page), 0);
        assert!(is_plain(path));
        assert_eq!(load(&nb, index).strokes.len(), 1);
    }
    assert!(std::fs::read(&outside).unwrap() == good, "the file outside is as it was");
    // the third page, untouched, has its ink
    assert_eq!(load(&nb, 2).strokes.len(), 3);
    assert_eq!(store_take_corrupt_count(), 0);
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
    let mut lang = String::from("?");
    settings_language(&mut lang, 16);
    assert_eq!(lang, "");
    let mut s = Settings::default();
    assert_eq!(settings_load(&mut s), 1);
    assert_eq!(settings_save(&s), 0);
    assert!(is_plain(&ini));
    assert!(!holds(&std::fs::read(&ini).unwrap(), b"outside-only"), "nothing of the file outside is saved back");
    assert!(is_link(&aside) && std::fs::read_link(&aside).unwrap() == outside, "the entry is kept aside, as it was");
    assert_eq!(std::fs::read(&outside).unwrap(), text_outside);
    // (what was saved reads)
    assert_eq!(settings_load(&mut s), 0);
    // a second round, with another link: the first one kept stays
    let second = t.join("second.ini");
    std::fs::write(&second, b"marker=second-only\n").unwrap();
    std::fs::remove_file(&ini).unwrap();
    symlink(&second, &ini).unwrap();
    assert_eq!(settings_load(&mut s), 1);
    assert_eq!(settings_save(&s), 0);
    assert!(is_plain(&ini) && !holds(&std::fs::read(&ini).unwrap(), b"-only"));
    assert!(is_link(&aside) && std::fs::read_link(&aside).unwrap() == outside, "settings.ini.unread is still the first");
    assert_eq!(std::fs::read(&second).unwrap(), b"marker=second-only\n");
    assert_eq!(std::fs::read(&outside).unwrap(), text_outside);
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
    let mut got = Settings::default();
    assert_eq!(settings_load(&mut got), 0, "exactly 1 MiB reads");
    assert_eq!(got.mirror_pdf, !s.mirror_pdf);
    let long = padded(lines.as_bytes(), MIB + 1);
    std::fs::write(&ini, &long).unwrap();
    assert_eq!(settings_load(&mut got), 1, "one byte more does not");
    assert_eq!(got.mirror_pdf, s.mirror_pdf, "the defaults are used");
    assert_eq!(settings_save(&got), 0);
    assert!(std::fs::read(&aside).unwrap() == long, "the long file is kept, byte for byte");
    assert!(is_plain(&ini) && std::fs::metadata(&ini).unwrap().len() < 100_000);
    // notebook.ini
    let nb = book("Long", 1, PAPER_BLANK);
    let nbini = nb_dir(&root, &nb).join("notebook.ini");
    let wrote = std::fs::read(&nbini).unwrap();
    std::fs::write(&nbini, padded(&wrote, MIB)).unwrap();
    let mut info = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut info), 0, "exactly 1 MiB reads");
    assert_eq!(info.name.as_str(), "Long");
    let long = padded(&wrote, MIB + 1);
    std::fs::write(&nbini, &long).unwrap();
    assert_eq!(store_load_info(nb.id.as_str(), &mut info), -1, "one byte more does not");
    let mut list = Vec::new();
    assert_eq!(store_list(&mut list), 0);
    assert!(list.is_empty());
    assert!(std::fs::read(&nbini).unwrap() == long, "the file is as it was");
    let _ = std::fs::remove_dir_all(&t);
}
