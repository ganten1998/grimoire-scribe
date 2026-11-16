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

// ---- Files' Covers view (2.2, issue #145) ----

/// A folder documents/NAME of `n` small pictures p00.png, p01.png, ... in a fresh storage; the worker is the
/// headless runner, as in the app.
fn covers_tree(name: &str, n: usize) -> String {
    let root = plain_tree(name);
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    for h in ["GRIMOIRE_BG_DELAY_MS", "GRIMOIRE_PIC_TEST_HANG", "GRIMOIRE_PIC_WATCHDOG_MS", "GRIMOIRE_PREVIEW_MS"] {
        std::env::remove_var(h);
    }
    let dir = format!("{root}/documents/Many");
    std::fs::create_dir_all(&dir).unwrap();
    let png = format!("{dir}/p00.png");
    let px: Vec<u8> = (0..40 * 30).map(|i| (i % 200) as u8).collect();
    assert_eq!(bgimage_write_png(&png, &BgPicture { px, w: 40, h: 30, ch: 1 }), 0);
    for i in 1..n {
        std::fs::copy(&png, format!("{dir}/p{i:02}.png")).unwrap();
    }
    // (each ten seconds newer than the one before: by newest they list the other way round)
    for i in 0..n {
        let c = std::ffi::CString::new(format!("{dir}/p{i:02}.png")).unwrap();
        let t = [libc::timeval { tv_sec: 1_700_000_000 + 10 * i as libc::time_t, tv_usec: 0 }; 2];
        // SAFETY: utimes reads the path and the two times we own.
        assert_eq!(unsafe { libc::utimes(c.as_ptr(), t.as_ptr()) }, 0);
    }
    root
}

fn files_probe(b: &Browser, k: &str) -> String {
    let mut o = String::new();
    assert!(b.probe(2480, k, &mut o), "{k}");
    o
}

/// Files open at documents/Many in the Covers view.
fn covers_open() -> Browser {
    let mut b = Browser::default();
    assert!(b.open(PickSpec::place(), grimoire::browser::FilesPrefs { card: true, ..Default::default() }));
    b.set_view(1);
    assert!(b.go(sp("documents/Many")));
    b
}

/// The background jobs run out, the browser working meanwhile (its lookups, its asking).
fn covers_settle(b: &mut Browser) {
    let t0 = grimoire::util::real_now_ms();
    loop {
        bg_poll();
        b.work(2480, now_ms());
        if !bg_busy() && files_probe(b, "making") == "0" {
            break;
        }
        assert!(grimoire::util::real_now_ms().wrapping_sub(t0) < 60_000, "the jobs did not end");
        sleep_ms(5);
    }
    for _ in 0..40 {
        b.work(2480, now_ms());
    }
}

fn cache_infos(root: &str) -> usize {
    std::fs::read_dir(format!("{root}/grimoire/.cache/files")).map(|rd| rd.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".info")).count()).unwrap_or(0)
}

/// A screen to draw the browser on (1860 x 2480, upright).
fn covers_app() -> App {
    assert_eq!(font_init(), 0);
    ui_init(300.0);
    i18n_set("en");
    let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    let (mut w, mut h) = (a.w, a.h);
    grimoire::sim_sdl::display_set_orientation(0, &mut w, &mut h);
    a
}

// What reaches the worker is the screen's files alone. A folder of 25 pictures and a PDF, nine tiles a screen (a
// Scribe upright): the first screen's nine are asked for, each its own job, and only they get a preview; a turn to
// the next screen asks for its nine and takes the first screen's still queued out of the queue (the jobs held back
// here by GRIMOIRE_BG_DELAY_MS: the one running runs on). The list asks for nothing, nor does the picker's pick mode
// with the same setting, nor Files' root. A folder past 5,000 entries keeps the cap, and only its screen is asked
// for.
#[test]
fn covers_asks_for_the_screens_files_only() {
    use grimoire::bgwork_previews::*;
    use grimoire::browser::FilesPrefs;
    let root = covers_tree("covers", 25);
    std::fs::write(format!("{root}/documents/Many/zz.pdf"), b"%PDF-1.4\n").unwrap();
    let probe = files_probe;
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
    assert_eq!(cache_infos(&root), 0);
    // Covers: the first screen's nine, made by the worker
    assert!(b.go(sp("documents/Many")));
    assert_eq!(probe(&b, "per_page"), "9");
    assert_eq!(probe(&b, "rows"), "26");
    ask(&mut b, 9);
    covers_settle(&mut b);
    assert_eq!(cache_infos(&root), 9);
    // held back: the second screen's nine queued behind a slow first; a turn to the third takes the queued ones back
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "400");
    b.turn(true, 2480);
    ask(&mut b, 18);
    assert_eq!(probe(&b, "making"), "1");
    b.turn(true, 2480);
    // (the second screen's eight queued ones are no longer asked for; the third's eight are)
    ask(&mut b, 18);
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    covers_settle(&mut b);
    // nine of the first screen, one of the second (the one that was running), the third's seven pictures and the
    // PDF's mark (no PDF at all: refused)
    assert_eq!(cache_infos(&root), 9 + 1 + 8);
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
    covers_settle(&mut b);
    assert_eq!(cache_infos(&root), 18 + 9);
    b.close();
}

// Leaving takes the queued previews back: the list (the View chip's other word), another folder, and Files closed
// each drop the jobs still queued for the screen, so that only the one that was running is made (it runs on: it is
// held to a preview's own few seconds). Back in Covers, what is missing is asked for again.
#[test]
fn covers_leaving_takes_the_queued_previews_back() {
    let root = covers_tree("leave", 9);
    std::fs::create_dir_all(format!("{root}/documents/Other")).unwrap();
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "300");
    let asked_nine = |b: &mut Browser| {
        for _ in 0..200 {
            b.work(2480, now_ms());
            if files_probe(b, "asked") == "9" {
                return;
            }
        }
        panic!("asked {}", files_probe(b, "asked"));
    };
    let idle = || {
        let t0 = grimoire::util::real_now_ms();
        while bg_busy() {
            bg_poll();
            assert!(grimoire::util::real_now_ms().wrapping_sub(t0) < 60_000);
            sleep_ms(5);
        }
    };
    // nine asked for, one running, eight queued; the list: the eight are gone, the one is made
    let mut b = covers_open();
    asked_nine(&mut b);
    assert_eq!(files_probe(&b, "making"), "1");
    b.set_view(0);
    assert_eq!((files_probe(&b, "view").as_str(), files_probe(&b, "asked").as_str(), files_probe(&b, "making").as_str()), ("list", "0", "0"));
    idle();
    assert_eq!(cache_infos(&root), 1);
    // Covers again: the eight asked for again (the one from the cache); another folder: seven of them gone
    b.set_view(1);
    asked_nine(&mut b);
    assert!(b.go(sp("documents/Other")));
    assert_eq!(files_probe(&b, "asked"), "0");
    idle();
    assert_eq!(cache_infos(&root), 2);
    // back in the folder; Files closed: six gone
    assert!(b.go(sp("documents/Many")));
    asked_nine(&mut b);
    b.close();
    idle();
    assert_eq!(cache_infos(&root), 3);
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    // and with nothing in the way every one of the nine is made
    let mut b = covers_open();
    covers_settle(&mut b);
    assert_eq!((files_probe(&b, "covers").as_str(), cache_infos(&root)), ("ppppppppp", 9));
    b.close();
}

// A preview that left the screen and comes back is read from the cache again: it never stays an icon. `CoverTiles`
// drops a preview's pixels when its file leaves the screen, and looks the file up again when it is back, a slice a
// tick (here one file a step, with no time at all: never the whole screen at once), without a job. Then through the
// browser, each way back: Next and Prev, Sort to another order and back, Show all and back. A file whose job ended
// with nothing to show is not asked for again in the visit, and is at the next.
#[test]
fn covers_reads_a_tile_that_comes_back_from_the_cache_in_slices() {
    use grimoire::bgwork_previews::*;
    let root = covers_tree("back", 21);
    std::fs::write(format!("{root}/documents/Many/notes.txt"), b"no preview").unwrap();
    let st = FsStorage::open(&root, &format!("{root}/grimoire")).unwrap();
    let file = |i: usize| {
        let p = sp(&format!("documents/Many/p{i:02}.png"));
        let s = st.stat(&p, false).unwrap();
        (p, s)
    };
    let screen = |from: usize| (from..from + 9).map(file).collect::<Vec<_>>();
    // the first screen made (its jobs), then the tiles' own model
    let mut b = covers_open();
    covers_settle(&mut b);
    assert_eq!(files_probe(&b, "covers"), "ppppppppp");
    assert_eq!((files_probe(&b, "asked").as_str(), cache_infos(&root)), ("9", 9));
    let mut t = CoverTiles::default();
    let one = screen(0);
    t.screen(&one);
    // (nothing is read when the screen is set: every tile waits to be looked up)
    assert!(one.iter().all(|(p, s)| t.show(p, s) == TileShow::Loading) && t.pending());
    for k in 0..9 {
        assert_eq!(one.iter().filter(|(p, s)| t.show(p, s) == TileShow::Preview).count(), k);
        assert!(t.step(0), "one file a step");
    }
    assert!(one.iter().all(|(p, s)| t.show(p, s) == TileShow::Preview && t.preview(p, s).is_some()) && !t.pending() && !t.making());
    assert!(!t.step(0) && !bg_busy(), "all read: nothing more, and no job was asked for");
    // away (their pixels dropped) and back: read again, in slices, still no job
    t.screen(&[]);
    assert!(one.iter().all(|(p, s)| t.preview(p, s).is_none()));
    t.screen(&one);
    assert!(one.iter().all(|(p, s)| t.show(p, s) == TileShow::Loading));
    assert!(t.step(0) && one.iter().filter(|(p, s)| t.show(p, s) == TileShow::Preview).count() == 1);
    while t.step(20) {}
    assert!(one.iter().all(|(p, s)| t.show(p, s) == TileShow::Preview) && !bg_busy());
    assert_eq!(t.asked(), 9);
    t.forget();
    // the browser: Next (made), Prev (read back), Next (read back), no job after the first time
    b.turn(true, 2480);
    covers_settle(&mut b);
    assert_eq!((files_probe(&b, "covers").as_str(), cache_infos(&root)), ("ppppppppp", 18));
    for next in [false, true, false] {
        b.turn(next, 2480);
        b.work(2480, now_ms());
        for _ in 0..40 {
            b.work(2480, now_ms());
            assert!(!bg_busy(), "a screen that comes back asks for no job");
        }
        assert_eq!(files_probe(&b, "covers"), "ppppppppp");
    }
    assert_eq!((files_probe(&b, "asked").as_str(), cache_infos(&root)), ("18", 18));
    // Sort and Show, by their chips (name, newest, size, name; all, usable): whatever the order shows is previews
    // once its files are made, and back at the first order and the usable files the screen is the one of before,
    // read from the cache with nothing made for it
    let mut a = covers_app();
    b.open = true;
    b.draw(&mut a);
    let mut chip = |b: &mut Browser, name: &str| {
        let (x, y) = b.point(name).unwrap_or_else(|| panic!("{name}"));
        let ev = InEvent { x, y, t_ms: now_ms(), ..Default::default() };
        b.pointer(&mut a, 0, &ev);
        b.pointer(&mut a, 2, &ev);
        b.list_now();
        covers_settle(b);
    };
    // (by newest the last three pictures, never on screen yet, come first: made now; the others were made before)
    chip(&mut b, "sort");
    assert_eq!((files_probe(&b, "sort").as_str(), files_probe(&b, "row.1").as_str()), ("newest", "p20.png"));
    assert_eq!((files_probe(&b, "covers").as_str(), cache_infos(&root)), ("ppppppppp", 21));
    for sort in ["size", "name"] {
        chip(&mut b, "sort");
        assert_eq!(files_probe(&b, "sort"), sort);
        assert_eq!(files_probe(&b, "covers"), "ppppppppp", "sorted by {sort}");
    }
    // (back by name: the first screen of before, every tile its preview again, and nothing made or asked for it)
    let made = cache_infos(&root);
    assert_eq!((files_probe(&b, "row.1").as_str(), made, files_probe(&b, "asked").as_str()), ("p00.png", 21, "21"));
    chip(&mut b, "show");
    assert_eq!(files_probe(&b, "rows"), "22");
    // (the text file sorts first: its tile is its icon, the pictures' are their previews)
    assert_eq!((files_probe(&b, "row.1").as_str(), files_probe(&b, "covers").as_str()), ("notes.txt", "ipppppppp"));
    chip(&mut b, "show");
    assert_eq!(files_probe(&b, "rows"), "21");
    assert_eq!((files_probe(&b, "covers").as_str(), cache_infos(&root)), ("ppppppppp", made));
    canvas_free(&mut a.screen);
    b.close();
    // a file whose job ends with nothing (no worker for it) keeps its icon for the visit: gone and back, no job
    // again; the next visit asks once more
    let root = covers_tree("nothing", 12);
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let mut b = covers_open();
    covers_settle(&mut b);
    assert_eq!((files_probe(&b, "covers").as_str(), cache_infos(&root)), ("iiiiiiiii", 0));
    b.turn(true, 2480);
    covers_settle(&mut b);
    b.turn(false, 2480);
    for _ in 0..40 {
        b.work(2480, now_ms());
        assert!(!bg_busy(), "asked once a visit");
    }
    assert_eq!(files_probe(&b, "covers"), "iiiiiiiii");
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    assert!(b.go(sp("documents/Many")));
    covers_settle(&mut b);
    assert_eq!((files_probe(&b, "covers").as_str(), cache_infos(&root)), ("ppppppppp", 9));
    b.close();
}

// The batching rule, by the tick's own clock: previews that arrive are never drawn one refresh a file. While the
// screen's set is not complete, the tiles that changed wait and are drawn together once COVERS_REFRESH_MS (2 s) have
// passed since the tiles were last drawn; when the set is complete, what is left is drawn at once. Each is one Ui of
// the union of its tiles, as a refresh that fills in by itself (never promoted to the cleaning Full). Here nine
// previews, each held back, cost two refreshes.
#[test]
fn covers_draws_the_tiles_that_changed_together() {
    use grimoire::browser::COVERS_REFRESH_MS;
    let _root = covers_tree("batch", 9);
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "150");
    let mut a = covers_app();
    let mut b = covers_open();
    b.open = true;
    b.list_now();
    b.draw(&mut a);
    let t0 = 50_000u32;
    let mut refreshes: Vec<(u32, String)> = Vec::new();
    // one tick at app time `t`: what it asked to refresh, flushed as the main loop does
    let mut tick = |b: &mut Browser, a: &mut App, t: u32, refreshes: &mut Vec<(u32, String)>| {
        bg_poll();
        b.tick(a, t);
        if !rect_is_empty(a.dirty) {
            assert!(a.dirty_filling && a.dirty_mode == RefreshMode::Ui, "a tile refresh fills in: {:?}", a.dirty_mode);
            refreshes.push((t, files_probe(b, "tiles")));
            app_flush(a);
        }
    };
    // (the tick after a whole draw is where the interval starts)
    tick(&mut b, &mut a, t0, &mut refreshes);
    // the first three previews land with the clock held still inside the interval: nothing is drawn
    let real0 = grimoire::util::real_now_ms();
    while files_probe(&b, "covers").matches('p').count() < 3 {
        tick(&mut b, &mut a, t0 + 100, &mut refreshes);
        assert!(grimoire::util::real_now_ms().wrapping_sub(real0) < 60_000);
        sleep_ms(5);
    }
    tick(&mut b, &mut a, t0 + COVERS_REFRESH_MS - 1, &mut refreshes);
    assert!(refreshes.is_empty(), "{refreshes:?}");
    assert_eq!(files_probe(&b, "tiles"), "lllllllll");
    // the interval is up: the ones that changed, together, in one refresh
    tick(&mut b, &mut a, t0 + COVERS_REFRESH_MS, &mut refreshes);
    assert_eq!(refreshes.len(), 1);
    let first = refreshes[0].1.matches('p').count();
    assert!((3..9).contains(&first) && refreshes[0].1.ends_with('l'), "{refreshes:?}");
    // the rest land inside the next interval: nothing more until the set is complete, then the rest at once
    let t1 = t0 + COVERS_REFRESH_MS + 500;
    while files_probe(&b, "covers") != "ppppppppp" || files_probe(&b, "making") != "0" {
        tick(&mut b, &mut a, t1, &mut refreshes);
        assert!(grimoire::util::real_now_ms().wrapping_sub(real0) < 60_000);
        sleep_ms(5);
    }
    tick(&mut b, &mut a, t1, &mut refreshes);
    assert_eq!(refreshes.len(), 2, "{refreshes:?}");
    assert_eq!(refreshes[1], (t1, "ppppppppp".to_string()));
    // nothing after: the tiles are as drawn
    for dt in [1, COVERS_REFRESH_MS, 10 * COVERS_REFRESH_MS] {
        tick(&mut b, &mut a, t1 + dt, &mut refreshes);
    }
    assert_eq!(refreshes.len(), 2);
    assert_eq!(a.ui_refreshes_since_full, 2, "two counted refreshes for nine previews");
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    b.close();
    canvas_free(&mut a.screen);
}

// Previews that land while an item card is open reach the grid when the card closes: the tick draws no tile under a
// card, and the close refreshes the card's rect together with every tile that changed meanwhile, so none is left an
// hourglass on the panel. And a tile whose preview lands under a press that then slides off is not left inverted.
#[test]
fn covers_tiles_under_a_card_and_under_a_press_come_right() {
    let _root = covers_tree("card", 9);
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "100");
    let mut a = covers_app();
    let mut b = covers_open();
    b.open = true;
    b.list_now();
    b.draw(&mut a);
    app_flush(&mut a);
    // the stats in, the first tile's card opened (a tap on it), while the previews are being made
    let at = |b: &Browser, name: &str| b.point(&format!("item.{name}")).unwrap_or_else(|| panic!("{name}"));
    let tap = |b: &mut Browser, a: &mut App, x: f32, y: f32| {
        let ev = InEvent { x, y, t_ms: now_ms(), ..Default::default() };
        b.pointer(a, 0, &ev);
        b.pointer(a, 2, &ev);
    };
    let (x, y) = at(&b, "p00.png");
    tap(&mut b, &mut a, x, y);
    assert_eq!(files_probe(&b, "card"), "picture");
    app_flush(&mut a);
    let real0 = grimoire::util::real_now_ms();
    let mut t = 10_000u32;
    while files_probe(&b, "covers") != "ppppppppp" || files_probe(&b, "making") != "0" {
        bg_poll();
        t += 5000; // (far past every interval: still nothing is drawn under the card)
        b.tick(&mut a, t);
        assert!(rect_is_empty(a.dirty), "no tile is drawn under a card");
        assert!(grimoire::util::real_now_ms().wrapping_sub(real0) < 60_000);
        sleep_ms(5);
    }
    // closed (Esc): the card's rect and every tile that changed are refreshed together; nothing is left to draw
    let card = b.card_rect();
    assert!(b.back(&mut a));
    assert_eq!(files_probe(&b, "tiles"), "ppppppppp");
    assert!(a.dirty.x <= card.x && a.dirty.y <= card.y, "the card's place is refreshed");
    for i in 0..9 {
        let (x, y) = at(&b, &format!("p{i:02}.png"));
        assert!(rect_contains(a.dirty, x as i32, y as i32), "tile {i} is refreshed with the card's close: {:?}", a.dirty);
    }
    app_flush(&mut a);
    b.tick(&mut a, t + 10_000);
    assert!(rect_is_empty(a.dirty) && b.tiles_due(t + 20_000).is_none());
    b.close();
    // a press on a tile whose preview lands under it, then a slide off: the tile is plain when the finger lifts
    let _root = covers_tree("press", 3);
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "200");
    let mut b = covers_open();
    b.open = true;
    b.list_now();
    b.draw(&mut a);
    app_flush(&mut a);
    let (x, y) = at(&b, "p00.png");
    // (a pixel of the tile beside its preview's box: white, and black while the tile is pressed)
    let beside = |a: &App| a.screen.px[(y as i32 * a.screen.stride + x as i32 - 250) as usize];
    assert_eq!(beside(&a), 255);
    b.pointer(&mut a, 0, &InEvent { x, y, t_ms: now_ms(), ..Default::default() });
    assert_eq!(beside(&a), 0, "pressed");
    // (the app's clock held at the press: no hold opens the card, and the tiles are drawn when their set is complete)
    let t = now_ms();
    let real0 = grimoire::util::real_now_ms();
    while files_probe(&b, "tiles") != "ppp" {
        bg_poll();
        b.tick(&mut a, t);
        app_flush(&mut a);
        assert!(grimoire::util::real_now_ms().wrapping_sub(real0) < 60_000);
        sleep_ms(5);
    }
    assert_eq!(files_probe(&b, "card"), "");
    assert_eq!(beside(&a), 0, "the tile drawn again under the press keeps its pressed look");
    // the finger slides off and lifts over nothing: no tap, and the tile is plain
    b.pointer(&mut a, 1, &InEvent { x: 5.0, y: 5.0, t_ms: now_ms(), ..Default::default() });
    b.pointer(&mut a, 2, &InEvent { x: 5.0, y: 5.0, t_ms: now_ms(), ..Default::default() });
    assert_eq!(files_probe(&b, "card"), "");
    assert_eq!(beside(&a), 255, "not left inverted");
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    b.close();
    canvas_free(&mut a.screen);
}
