// The file picker (src/browser.rs, 2.1.0-beta.2, lane L8) against a hostile tree, through its own rows, probes
// and walk: links to /etc, to "..", to a PDF, to Grimoire's notebooks; a FIFO; a name with ':'; a name that is not
// UTF-8 (where the file system takes one); a path of 599 bytes and names that would make 600. A link is never
// listed, entered or opened, Grimoire's notebooks, PDFs and trash are never offered, a long path is counted.
#![cfg(feature = "sim")]
use grimoire::browser::{Browser, PickSpec};
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
    assert!(b.open_pick(PickSpec::import()));
    b
}

// All of it in one test: the storage root is the process's.
#[test]
fn the_picker_over_a_hostile_tree() {
    ui_init(300.0);
    i18n_set("en");
    let root = tree("hostile");
    let mut b = open();
    // the root: Import, Documents, Downloads, All storage, and (2.2) the way to the Files card while it is not shown
    assert_eq!(names(&b), ["Import", "Documents", "Downloads", "All storage", "Show Files in Library"]);
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

// A path with a line break in a name cannot be handed to an import (its mark and state keep a path on one line),
// so such a PDF or folder is not a row; a tab is a name's own character, and its rows are there.
#[test]
fn names_with_a_line_break_are_not_rows() {
    ui_init(300.0);
    i18n_set("en");
    let root = plain_tree("line-breaks");
    let mut made = Vec::new();
    for (name, folder) in [("a\nb.pdf", false), ("Sc\nans", true), ("t\tx.pdf", false), ("Sc\tans", true), ("plain.pdf", false)] {
        let p = format!("{root}/documents/{name}");
        let ok = if folder { std::fs::create_dir_all(&p).is_ok() } else { std::fs::write(&p, b"%PDF-1.4\n").is_ok() };
        if ok {
            made.push(name);
        } else {
            eprintln!("  skipped {name:?}: this file system refuses the name");
        }
    }
    let mut b = open();
    assert!(b.go(sp("documents")));
    let rows: Vec<String> = b.rows.iter().map(|r| r.name.clone()).collect();
    for name in ["t\tx.pdf", "Sc\tans", "plain.pdf"] {
        assert_eq!(rows.iter().any(|r| r == name), made.contains(&name), "{name:?}: {rows:?}");
    }
    for name in ["a\nb.pdf", "Sc\nans"] {
        assert!(!rows.iter().any(|r| r == name), "{name:?} is a row: {rows:?}");
    }
}

// Files' Covers view (2.2, issue #145): what reaches the worker is the screen's files alone. A folder of 25 pictures
// and a PDF, nine tiles a screen (a Scribe upright): the first screen's nine are asked for, each its own job, and only
// they get a preview; a turn to the next screen asks for its nine and takes the first screen's still queued out of the
// queue (the jobs held back here by GRIMOIRE_BG_DELAY_MS: the one running runs on). The list asks for nothing, nor
// does the picker's pick mode with the same setting, nor Files' root. A folder past 5,000 entries keeps the cap,
// and only its screen is asked for.
#[test]
fn covers_asks_for_the_screens_files_only() {
    use grimoire::bgwork_previews::*;
    use grimoire::browser::FilesPrefs;
    let root = plain_tree("covers");
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    let dir = format!("{root}/documents/Many");
    std::fs::create_dir_all(&dir).unwrap();
    let png = format!("{dir}/p00.png");
    let px: Vec<u8> = (0..40 * 30).map(|i| (i % 200) as u8).collect();
    assert_eq!(bgimage_write_png(&png, &BgPicture { px, w: 40, h: 30, ch: 1 }), 0);
    for i in 1..25 {
        std::fs::copy(&png, format!("{dir}/p{i:02}.png")).unwrap();
    }
    std::fs::write(format!("{dir}/zz.pdf"), b"%PDF-1.4\n").unwrap();
    let cache = format!("{root}/grimoire/.cache/files");
    let infos = || std::fs::read_dir(&cache).map(|rd| rd.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".info")).count()).unwrap_or(0);
    let probe = |b: &Browser, k: &str| {
        let mut o = String::new();
        assert!(b.probe(2480, k, &mut o), "{k}");
        o
    };
    let settle = || {
        let t0 = grimoire::util::real_now_ms();
        while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 60_000 {
            bg_poll();
            sleep_ms(5);
        }
        assert!(!bg_busy());
    };
    let ask = |b: &mut Browser, n: usize| {
        for _ in 0..200 {
            b.work(2480, now_ms());
            if probe(b, "asked") == n.to_string() {
                return;
            }
        }
        panic!("asked {} of {n}", probe(b, "asked"));
    };
    let card = FilesPrefs { card: true, ..Default::default() };
    // the list: nothing asked
    let mut b = Browser::default();
    assert!(b.open(PickSpec::place(), card));
    assert!(b.go(sp("documents/Many")));
    for _ in 0..20 {
        b.work(2480, now_ms());
    }
    assert_eq!(probe(&b, "view"), "list");
    assert_eq!(probe(&b, "asked"), "0");
    // pick mode with Covers set: a list, nothing asked; Files' root neither
    let mut b = Browser::default();
    assert!(b.open(PickSpec::import(), card));
    b.set_view(1);
    assert!(b.go(sp("documents/Many")));
    for _ in 0..20 {
        b.work(2480, now_ms());
    }
    assert_eq!(probe(&b, "asked"), "0");
    let mut b = Browser::default();
    assert!(b.open(PickSpec::place(), card));
    b.set_view(1);
    for _ in 0..20 {
        b.work(2480, now_ms());
    }
    assert_eq!(probe(&b, "asked"), "0");
    assert_eq!(infos(), 0);
    // Covers: the first screen's nine, made by the worker
    assert!(b.go(sp("documents/Many")));
    assert_eq!(probe(&b, "per_page"), "9");
    assert_eq!(probe(&b, "rows"), "26");
    ask(&mut b, 9);
    settle();
    assert_eq!(infos(), 9);
    // held back: the second screen's nine queued behind a slow first; a turn to the third takes the queued ones back
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "400");
    b.turn(true, 2480);
    ask(&mut b, 18);
    assert_eq!(probe(&b, "making"), "1");
    b.turn(true, 2480);
    // (the second screen's eight queued ones are no longer asked for; the third's eight are)
    ask(&mut b, 18);
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    settle();
    // nine of the first screen, one of the second (the one that was running), the third's seven pictures and the
    // PDF's mark (no PDF at all: refused)
    assert_eq!(infos(), 9 + 1 + 8);
    let zz = FsStorage::open(&root, &format!("{root}/grimoire")).unwrap().stat(&sp("documents/Many/zz.pdf"), false).unwrap();
    assert!(matches!(preview_cached(&sp("documents/Many/zz.pdf"), &zz), Cached::NoPreview));
    assert_eq!(probe(&b, "making"), "0");
    // past 5,000 entries: the cap holds, and the screen alone is asked for (empty files: each marked, no worker)
    let big = format!("{root}/documents/Big");
    std::fs::create_dir_all(&big).unwrap();
    for i in 0..5010 {
        std::fs::write(format!("{big}/b{i:04}.png"), b"").unwrap();
    }
    assert!(b.go(sp("documents/Big")));
    assert_eq!(probe(&b, "capped"), "1");
    assert_eq!(probe(&b, "rows"), "5000");
    ask(&mut b, 9);
    settle();
    assert_eq!(infos(), 18 + 9);
    b.close();
}
