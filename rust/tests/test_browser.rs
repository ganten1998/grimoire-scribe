// The file picker (src/browser.rs, 2.1.0-beta.2, lane L8) against a hostile tree, through its own rows, probes
// and walk: links to /etc, to "..", to a PDF, to Grimoire's notebooks; a FIFO; a name with ':'; a name that is not
// UTF-8 (where the file system takes one); a path of 599 bytes and names that would make 600. A link is never
// listed, entered or opened, Grimoire's notebooks, PDFs and trash are never offered, a long path is counted.
#![cfg(feature = "sim")]
use grimoire::browser::{Browser, PickKind, PickSpec};
use grimoire::fsview::*;
use grimoire::prelude::*;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;

fn sp(s: &str) -> StoragePath {
    StoragePath::new(s.as_bytes()).unwrap()
}

fn tree(name: &str) -> String {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let root = base.join(format!("grimoire-browser-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let g = root.join("grimoire");
    for d in ["documents/Books", "documents/Scans", "Downloads", "system", "koreader"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    for d in ["notebooks/abc", "PDFs/School", "trash", "import"] {
        std::fs::create_dir_all(g.join(d)).unwrap();
    }
    for f in ["documents/a.pdf", "documents/Books/b.pdf", "documents/time 10:30.pdf", "documents/Scans/p1.png", "documents/Scans/p2.png", "grimoire/PDFs/School/x.pdf", "grimoire/trash/t.pdf", "system/s.pdf"] {
        std::fs::write(root.join(f), b"%PDF-1.4\n").unwrap();
    }
    symlink("/etc", root.join("documents/etc")).unwrap();
    symlink("..", root.join("documents/up")).unwrap();
    symlink("a.pdf", root.join("documents/alias.pdf")).unwrap();
    symlink(g.join("notebooks"), root.join("Downloads/nb")).unwrap();
    symlink("/etc/hosts", root.join("documents/Scans/p3.png")).unwrap();
    let fifo = std::ffi::CString::new(root.join("documents/pipe.pdf").as_os_str().as_bytes()).unwrap();
    // SAFETY: a NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    // (macOS's file system takes UTF-8 names only)
    let _ = std::fs::write(root.join(std::ffi::OsStr::from_bytes(b"documents/caf\xe9.pdf")), b"%PDF-1.4\n");
    std::env::set_var("GRIMOIRE_STORAGE", &root);
    assert_eq!(store_init(g.to_str().unwrap()), 0);
    root.to_str().unwrap().to_string()
}

fn names(b: &Browser) -> Vec<String> {
    (1..=b.rows.len()).map(|n| {
        let mut o = String::new();
        assert!(b.probe(2480, &format!("row.{n}"), &mut o));
        o
    }).collect()
}

fn open() -> Browser {
    let mut b = Browser::default();
    assert!(b.open_pick(PickSpec { kinds: &[PickKind::Pdf, PickKind::PictureFolder] }));
    b
}

// All of it in one test: the storage root is the process's.
#[test]
fn the_picker_over_a_hostile_tree() {
    ui_init(300.0);
    i18n_set("en");
    let root = tree("hostile");
    let mut b = open();
    // the root: Import, Documents, Downloads, All storage
    assert_eq!(names(&b), ["Import", "Documents", "Downloads", "All storage"]);
    // documents: the plain folders and PDFs only, in order (links, the FIFO and the odd name are not listed)
    assert!(b.go(sp("documents")));
    let n = names(&b);
    assert_eq!(&n[..2], ["Books", "Scans"], "{n:?}");
    assert!(n.contains(&"a.pdf".to_string()) && n.contains(&"time 10:30.pdf".to_string()), "{n:?}");
    for bad in ["etc", "up", "alias.pdf", "pipe.pdf"] {
        assert!(!n.contains(&bad.to_string()), "{bad} was listed");
    }
    assert!(!n.iter().any(|x| x.contains('\u{fffd}') || x.starts_with("caf")), "an odd name was listed: {n:?}");
    assert!(b.hidden >= 3, "the links are counted: {}", b.hidden);
    // a link is never entered, whatever it points at
    for l in ["documents/etc", "documents/up", "Downloads/nb"] {
        assert!(!b.go(sp(l)), "{l} was entered");
    }
    // Grimoire's notebooks, its PDFs and its trash are refused to a pick (also by their path)
    for l in ["grimoire/notebooks", "grimoire/PDFs", "grimoire/PDFs/School", "grimoire/trash", "system", "koreader"] {
        assert!(!b.go(sp(l)), "{l} was entered");
    }
    // ... and not listed in All storage's grimoire
    assert!(b.go(sp("grimoire")));
    assert_eq!(names(&b), ["import"]);
    // a folder's pictures: counted through the walk, the link among them not
    assert!(b.go(sp("documents")));
    let mut o = String::new();
    assert!(b.probe(2480, "item.Scans", &mut o));
    let _ = o;
    // (the idle tick fills the count in: a folder's `pictures` is read through the walk)
    let mut guard = 0;
    while guard < 20 {
        b.work(2480, 1000 + guard);
        guard += 1;
    }
    let mut c = String::new();
    assert!(b.probe(2480, "pictures.2", &mut c));
    assert_eq!(c, "2", "p3.png is a link and is not counted");
    let _ = root;
}

#[test]
fn a_path_of_599_bytes_lists_and_a_name_that_makes_600_is_counted() {
    ui_init(300.0);
    i18n_set("en");
    let root = tree("long");
    // documents/ + 200 + / + 200 + / + 170 = 9 + 1 + 200 + 1 + 200 + 1 + ... keep the folder at 589 bytes
    let (a, b2, c) = ("a".repeat(200), "b".repeat(200), "c".repeat(177));
    let rel = format!("documents/{a}/{b2}/{c}");
    assert_eq!(rel.len(), 589);
    std::fs::create_dir_all(format!("{root}/{rel}")).unwrap();
    std::fs::write(format!("{root}/{rel}/ok.pdf"), b"%PDF-1.4\n").unwrap();
    // a name that makes the path 600 bytes or more
    std::fs::write(format!("{root}/{rel}/{}.pdf", "z".repeat(10)), b"%PDF-1.4\n").unwrap();
    let mut b = open();
    assert!(b.go(sp(&rel)));
    assert_eq!(b.rows.len(), 1, "only ok.pdf makes a path under 600 bytes: 589 + 1 + 6 = 596");
    assert_eq!(b.too_long, 1);
}

// ---- the fix round of 2026-11-09 (M18 and the Import place's rows): each failed on 0c26d912 ----

fn plain_tree(name: &str) -> String {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let root = base.join(format!("grimoire-browser-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("grimoire/import")).unwrap();
    std::fs::create_dir_all(root.join("documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &root);
    assert_eq!(store_init(root.join("grimoire").to_str().unwrap()), 0);
    grimoire::pdfimport::ui_reset();
    root.to_str().unwrap().to_string()
}

// Back and Esc leave a folder whose parent no longer opens (renamed over USB while the picker is two levels down).
#[test]
fn back_leaves_a_folder_whose_parent_is_gone() {
    ui_init(300.0);
    i18n_set("en");
    let root = plain_tree("up-trap");
    std::fs::create_dir_all(format!("{root}/documents/A/B")).unwrap();
    std::fs::write(format!("{root}/documents/A/B/x.pdf"), b"%PDF-1.4\n").unwrap();
    let mut b = open();
    for p in ["documents", "documents/A", "documents/A/B"] {
        assert!(b.go(sp(p)), "{p}");
    }
    std::fs::rename(format!("{root}/documents/A"), format!("{root}/documents/Z")).unwrap();
    let mut presses = 0;
    while presses < 10 && b.up(now_ms()) {
        presses += 1;
        b.list_now();
    }
    assert!(presses < 10, "ten presses of Back, each one taken, and the picker never left");
    let mut o = String::new();
    assert!(b.probe(2480, "place", &mut o));
    assert_eq!(o, "root");
}

// A row whose file is gone since the listing is not drawn (and the panel refreshed) again at every tick.
#[test]
fn a_row_whose_file_is_gone_is_not_redrawn_at_every_tick() {
    ui_init(300.0);
    i18n_set("en");
    let root = plain_tree("gone-row");
    for f in ["a.pdf", "b.pdf"] {
        std::fs::write(format!("{root}/documents/{f}"), b"%PDF-1.4\n").unwrap();
    }
    let mut b = open();
    assert!(b.go(sp("documents")));
    assert_eq!(b.rows.len(), 2);
    std::fs::remove_file(format!("{root}/documents/a.pdf")).unwrap();
    let redraws = (0..50u32).filter(|k| b.work(2480, 1000 + k * 30).contains(&0)).count();
    assert!(redraws <= 1, "50 ticks: the gone file's row was redrawn {redraws} times");
}

// Only grimoire/import/done and what is in it reads as Imported: a folder named done-old is a folder.
#[test]
fn a_folder_whose_name_starts_with_done_is_not_imported() {
    ui_init(300.0);
    i18n_set("en");
    let root = plain_tree("done-old");
    std::fs::create_dir_all(format!("{root}/grimoire/import/done-old")).unwrap();
    std::fs::write(format!("{root}/grimoire/import/done-old/x.pdf"), b"%PDF-1.4\n").unwrap();
    let mut b = open();
    assert!(b.go(sp("grimoire/import/done-old")));
    for k in 0..10u32 {
        b.work(2480, 1000 + k * 1100);
    }
    let mut o = String::new();
    assert!(b.probe(2480, "status.1", &mut o));
    assert_ne!(o, "imported", "a PDF in import/done-old is not imported");
}
