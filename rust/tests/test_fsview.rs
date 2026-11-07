// The storage layer (src/fsview.rs, 2.1 lane L0, read-only) against a hostile tree: links to /etc and to "..",
// a FIFO, a name with ':', a name that is not UTF-8, 599- and 601-byte paths, a folder on another device,
// Grimoire's PDFs cased differently. A listing never enters a link, and its slices keep to their budget.
mod testlib;
use grimoire::fsview::*;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use testlib::*;

fn sp(s: &[u8]) -> StoragePath {
    StoragePath::new(s).unwrap_or_else(|| panic!("{:?}", String::from_utf8_lossy(s)))
}

/// The hostile tree, made afresh: the storage root and Grimoire's folder in it.
fn tree() -> (String, String) {
    let root = format!("{}/fsview-storage", tl_tmp());
    let _ = std::fs::remove_dir_all(&root);
    let g = format!("{root}/grimoire");
    for d in ["documents/Books", "Downloads", "fonts", "system", "koreader", ".active", "Book.sdr"] {
        std::fs::create_dir_all(format!("{root}/{d}")).unwrap();
    }
    for d in ["notebooks/abc", "PDFs/School", "trash", "import/Planner", "templates", "photos", "fonts", "app", ".cache", "bluetooth"] {
        std::fs::create_dir_all(format!("{g}/{d}")).unwrap();
    }
    for f in ["documents/a.pdf", "documents/Books/b.pdf", "documents/time 10:30.pdf", "grimoire/settings.ini", "grimoire/PDFs/School/x.pdf", "grimoire/import/Planner/p1.png"] {
        std::fs::write(format!("{root}/{f}"), b"%PDF-1.4\n").unwrap();
    }
    symlink("/etc", format!("{root}/documents/etc")).unwrap();
    symlink("..", format!("{root}/documents/up")).unwrap();
    symlink("a.pdf", format!("{root}/documents/alias.pdf")).unwrap();
    symlink(format!("{g}/notebooks"), format!("{root}/Downloads/nb")).unwrap();
    let fifo = std::ffi::CString::new(format!("{root}/documents/pipe")).unwrap();
    // SAFETY: a NUL-terminated path.
    check!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) } == 0);
    (root, g)
}

fn names(l: &FsListing) -> Vec<String> {
    let mut v: Vec<String> = l.entries.iter().map(|e| String::from_utf8_lossy(&e.name).into_owned()).collect();
    v.sort();
    v
}

fn list_all(st: &FsStorage, p: &[u8], pick: bool) -> FsListing {
    let mut l = FsListing::start(st, &sp(p), pick).unwrap();
    while !l.step(FS_SLICE_MS) {}
    l
}

// What a StoragePath may be: under 600 bytes, no "..", ".", empty component, leading "/" or NUL.
fn paths_are_checked() {
    let p599 = [b"a".repeat(200), b"b".repeat(200), b"c".repeat(197)].join(&b'/');
    check!(p599.len() == 599 && StoragePath::new(&p599).is_some());
    let p600 = [&p599[..], b"d"].concat();
    check!(p600.len() == 600 && StoragePath::new(&p600).is_none());
    let p601 = [&p599[..], b"/d"].concat();
    check!(p601.len() == 601 && StoragePath::new(&p601).is_none());
    for bad in [&b"/etc"[..], b"../x", b"a/../b", b"a/./b", b"a//b", b"a/", b"..", b".", b"a\0b"] {
        check!(StoragePath::new(bad).is_none());
    }
    check!(StoragePath::new(b"").is_some_and(|p| p.is_root()));
    check!(StoragePath::new(b"documents/time 10:30.pdf").is_some());
    check!(StoragePath::new(b"documents/caf\xe9.pdf").is_some());
    check!(sp(b"documents").join(b"a/b").is_none() && sp(b"documents").join(b"..").is_none());
    check!(sp(b"documents").join(b"a.pdf") == Some(sp(b"documents/a.pdf")));
    check!(sp(b"a/b/c.pdf").split_last().is_some_and(|(f, n)| f == sp(b"a/b") && n == b"c.pdf"));
    check!(StoragePath::root().split_last().is_none());
}

// The zones: the system's folders, dot-entries and *.sdr refused; Grimoire's own refused (PDFs and trash to a
// pick), its import, templates, photos and fonts writable, the rest read-only; names in any case.
fn zones_by_name_in_any_case() {
    let (root, g) = tree();
    let st = FsStorage::open(&root, &g).unwrap();
    for (p, pick, z) in [
        (&b""[..], false, FsZone::ReadOnly),
        (b"documents/a.pdf", false, FsZone::ReadOnly),
        (b"Downloads", true, FsZone::ReadOnly),
        (b"fonts", false, FsZone::ReadOnly),
        (b"system", false, FsZone::Refused),
        (b"KOReader/plugins", false, FsZone::Refused),
        (b".active", false, FsZone::Refused),
        (b"documents/Book.SDR", false, FsZone::Refused),
        (b"grimoire", false, FsZone::ReadOnly),
        (b"grimoire/settings.ini", false, FsZone::Refused),
        (b"grimoire/notebooks/abc", false, FsZone::Refused),
        (b"Grimoire/Notebooks", false, FsZone::Refused),
        (b"grimoire/app", false, FsZone::Refused),
        (b"grimoire/.cache", false, FsZone::Refused),
        (b"grimoire/bluetooth", false, FsZone::Refused),
        (b"grimoire/PDFs/School/x.pdf", false, FsZone::ReadOnly),
        (b"grimoire/PDFs/School/x.pdf", true, FsZone::Refused),
        (b"grimoire/pdfs", true, FsZone::Refused),
        (b"grimoire/Trash", true, FsZone::Refused),
        (b"grimoire/import/Planner", true, FsZone::Writable),
        (b"GRIMOIRE/Fonts", false, FsZone::Writable),
        (b"grimoire/photos", false, FsZone::Writable),
    ] {
        let got = st.zone(&sp(p), pick);
        if got != z {
            eprintln!("  {} (pick {pick}): {got:?}", String::from_utf8_lossy(p));
        }
        check!(got == z);
    }
}

// A link anywhere fails: to /etc, to "..", to a file, to Grimoire's notebooks; a FIFO is no file and never
// waits; a name with ':' opens; the walk never leaves the root's device.
fn the_walk_follows_no_link() {
    let (root, g) = tree();
    let st = FsStorage::open(&root, &g).unwrap();
    check!(st.open_dir(&sp(b"documents"), false).is_ok());
    check!(st.open_dir(&sp(b"documents/etc"), false).err() == Some(FsErr::Link));
    check!(st.open_dir(&sp(b"documents/etc/ssh"), false).err() == Some(FsErr::Link));
    check!(st.open_file(&sp(b"documents/etc/hosts"), false).err() == Some(FsErr::Link));
    check!(st.open_dir(&sp(b"documents/up"), false).err() == Some(FsErr::Link));
    check!(st.open_dir(&sp(b"documents/up/documents"), false).err() == Some(FsErr::Link));
    check!(st.open_file(&sp(b"documents/alias.pdf"), true).err() == Some(FsErr::Link));
    check!(st.open_dir(&sp(b"Downloads/nb/abc"), false).err() == Some(FsErr::Link));
    check!(st.open_file(&sp(b"documents/a.pdf"), true).is_ok());
    check!(st.open_file(&sp(b"documents/Books/b.pdf"), true).is_ok());
    check!(st.open_file(&sp(b"documents/time 10:30.pdf"), true).is_ok());
    let t = real();
    check!(st.open_file(&sp(b"documents/pipe"), true).err() == Some(FsErr::NotFile));
    check!(real().wrapping_sub(t) < 1000);
    check!(st.open_file(&sp(b"documents"), true).err() == Some(FsErr::NotFile));
    check!(st.open_file(&sp(b"documents/none.pdf"), true).err() == Some(FsErr::NotFound));
    check!(st.open_dir(&sp(b"documents/a.pdf"), false).err() == Some(FsErr::NotDir));
    check!(st.open_dir(&sp(b"system"), false).err() == Some(FsErr::Refused));
    check!(st.open_file(&sp(b"grimoire/settings.ini"), false).err() == Some(FsErr::Refused));
    // the stat an action compares with the listing: a link is its own, never followed
    let a = st.stat(&sp(b"documents/a.pdf"), false).unwrap();
    check!(a.size == 9 && a.same(&st.stat(&sp(b"documents/a.pdf"), false).unwrap()));
    check!(!a.same(&st.stat(&sp(b"documents/Books/b.pdf"), false).unwrap()));
    let l = st.stat(&sp(b"documents/etc"), false).unwrap();
    check!((l.mode & libc::S_IFMT as u32) == libc::S_IFLNK as u32);
    // A folder on another device (here the system's /dev, whose st_dev is not the root's): not entered.
    let (d0, d1) = (std::fs::metadata("/").map(|m| std::os::unix::fs::MetadataExt::dev(&m)), std::fs::metadata("/dev").map(|m| std::os::unix::fs::MetadataExt::dev(&m)));
    if let (Ok(d0), Ok(d1)) = (d0, d1) {
        if d0 != d1 {
            let sys = FsStorage::open("/", "/nonexistent-grimoire").unwrap();
            check!(sys.open_dir(&sp(b"dev"), false).err() == Some(FsErr::OtherDevice));
            check!(sys.open_file(&sp(b"dev/null"), false).err() == Some(FsErr::OtherDevice));
        }
    }
}

fn real() -> u32 {
    grimoire::util::real_now_ms()
}

// Grimoire's own folders by identity: its PDFs cased differently are its PDFs on a case-insensitive storage
// (and nothing on a case-sensitive one); a pick never reaches them, nor the notebooks by any case.
fn grimoires_folders_by_identity() {
    let (root, g) = tree();
    let st = FsStorage::open(&root, &g).unwrap();
    check!(st.open_dir(&sp(b"grimoire/PDFs/School"), false).is_ok());
    check!(st.open_dir(&sp(b"grimoire/PDFs/School"), true).err() == Some(FsErr::Refused));
    check!(st.open_file(&sp(b"grimoire/PDFs/School/x.pdf"), true).err() == Some(FsErr::Refused));
    let ci = std::path::Path::new(&format!("{g}/pdfs")).exists();
    let lower = st.open_dir(&sp(b"grimoire/pdfs/School"), true).err();
    check!(lower == Some(if ci { FsErr::Refused } else { FsErr::NotFound }));
    check!(st.open_file(&sp(b"grimoire/pdfs/School/x.pdf"), true).is_err());
    check!(st.open_dir(&sp(b"grimoire/notebooks"), false).err() == Some(FsErr::Refused));
    check!(st.open_dir(&sp(b"Grimoire/NOTEBOOKS/abc"), false).is_err());
    check!(st.open_dir(&sp(b"grimoire/import/Planner"), true).is_ok());
    // by identity alone, whatever the name: a Grimoire whose folder is elsewhere, its notebooks reached here
    let other = FsStorage::open(&format!("{root}/grimoire"), &g).unwrap();
    check!(other.open_dir(&sp(b"notebooks"), false).err() == Some(FsErr::Refused));
    check!(other.open_dir(&sp(b"PDFs"), true).err() == Some(FsErr::Refused));
    check!(other.open_dir(&sp(b"PDFs"), false).is_ok());
}

// A listing: names and kinds only, links and the deny-list's entries left out (counted), a FIFO and odd
// names kept; Grimoire's folder lists only the user's places; stats only for the rows asked, links unfollowed.
fn a_listing_never_enters_a_link() {
    let (root, g) = tree();
    let odd = format!("{root}/documents/caf\u{e9} \u{2014}.pdf");
    std::fs::write(&odd, b"x").unwrap();
    // a name that is not UTF-8 (a storage that refuses one, APFS, has none)
    let raw = std::ffi::OsStr::from_bytes(b"caf\xe9.pdf");
    let non_utf8 = std::fs::write(std::path::Path::new(&format!("{root}/documents")).join(raw), b"x").is_ok();
    let st = FsStorage::open(&root, &g).unwrap();
    let l = list_all(&st, b"", false);
    check!(names(&l) == ["Downloads", "documents", "fonts", "grimoire"]);
    check!(l.hidden == 4 && l.done && !l.capped); // system, koreader, .active, Book.sdr
    let mut l = list_all(&st, b"documents", false);
    let mut want = vec!["Books", "a.pdf", "caf\u{e9} \u{2014}.pdf", "pipe", "time 10:30.pdf"];
    if non_utf8 {
        want.push("caf\u{fffd}.pdf");
    }
    want.sort();
    check!(names(&l) == want);
    check!(l.hidden == 3); // etc, up, alias.pdf: links are never offered
    check!(l.entries.iter().all(|e| e.stat.is_none()));
    let n = l.entries.len();
    check!(l.stat(0..n, FS_SLICE_MS));
    for e in &l.entries {
        let want = match e.name.as_slice() {
            b"Books" => FsKind::Dir,
            b"pipe" => FsKind::Other,
            _ => FsKind::File,
        };
        check!(e.kind == want && e.stat.is_some());
    }
    if non_utf8 {
        let e = l.entries.iter().find(|e| e.name == b"caf\xe9.pdf").unwrap();
        check!(st.open_file(&sp(b"documents").join(&e.name).unwrap(), true).is_ok());
    }
    let l = list_all(&st, b"grimoire", false);
    check!(names(&l) == ["PDFs", "fonts", "import", "photos", "templates", "trash"]);
    let l = list_all(&st, b"grimoire", true);
    check!(names(&l) == ["fonts", "import", "photos", "templates"]);
    check!(FsListing::start(&st, &sp(b"documents/etc"), false).err() == Some(FsErr::Link));
    check!(FsListing::start(&st, &sp(b"documents/up"), false).err() == Some(FsErr::Link));
    check!(FsListing::start(&st, &sp(b"grimoire/notebooks"), false).err() == Some(FsErr::Refused));
}

// Slices keep to their budget (a clock that moves 1 ms a reading: a slice of 20 reads about 20 entries and
// stops), 5,000 entries at most, and stats in slices too.
fn slices_keep_to_their_budget() {
    let (root, g) = tree();
    let big = format!("{root}/Downloads/big");
    std::fs::create_dir_all(&big).unwrap();
    for i in 0..5100 {
        std::fs::write(format!("{big}/f{i:05}.png"), b"").unwrap();
    }
    let st = FsStorage::open(&root, &g).unwrap();
    let mut l = FsListing::start(&st, &sp(b"Downloads/big"), false).unwrap();
    let mut t = 0u32;
    let mut slices = 0;
    loop {
        let start = t;
        let before = l.entries.len();
        let done = l.step_clocked(FS_SLICE_MS, &mut || {
            t += 1;
            t
        });
        slices += 1;
        check!(t - start <= FS_SLICE_MS + 2);
        check!(l.entries.len() - before <= FS_SLICE_MS as usize);
        if done {
            break;
        }
    }
    check!(l.entries.len() == FS_LIST_MAX && l.capped && slices >= FS_LIST_MAX / FS_SLICE_MS as usize);
    // stats: the rows on screen, a slice at a time
    let mut t = 0u32;
    let mut slices = 0;
    while !l.stat_clocked(100..112, 5, &mut || {
        t += 1;
        t
    }) {
        slices += 1;
        check!(slices < 10);
    }
    check!(slices >= 2);
    check!(l.entries[100..112].iter().all(|e| e.stat.is_some()) && l.entries[112].stat.is_none() && l.entries[99].stat.is_none());
    // the real clock: a slice of a real listing is short
    let mut l = FsListing::start(&st, &sp(b"Downloads/big"), false).unwrap();
    let t0 = real();
    l.step(FS_SLICE_MS);
    check!(real().wrapping_sub(t0) < 1000 && !l.entries.is_empty());
    let _ = std::fs::remove_dir_all(&big);
}

// Free space: the floor every big job leaves, the "nearly full" line, and the storage's own number.
fn free_space() {
    let (root, g) = tree();
    let st = FsStorage::open(&root, &g).unwrap();
    check!(st.free_bytes().is_some_and(|f| f > 0));
    let mb = 1u64 << 20;
    check!(fsview_room_for(500 * mb, 400 * mb));
    check!(!fsview_room_for(500 * mb, 401 * mb));
    check!(!fsview_room_for(50 * mb, 60 * mb));
    check!(fsview_nearly_full(199 * mb) && !fsview_nearly_full(200 * mb));
}

// The root: GRIMOIRE_STORAGE off the Kindle (an absolute path, never "/"), none without it.
fn the_root_is_the_hooks() {
    std::env::remove_var("GRIMOIRE_STORAGE");
    check!(fsview_root().is_none());
    for bad in ["/", "//", "relative/path", ""] {
        std::env::set_var("GRIMOIRE_STORAGE", bad);
        check!(fsview_root().is_none());
    }
    std::env::set_var("GRIMOIRE_STORAGE", "/tmp/storage");
    check!(fsview_root().as_deref() == Some("/tmp/storage"));
    std::env::remove_var("GRIMOIRE_STORAGE");
    check!(FS_KINDLE_ROOT == "/mnt/us");
}

#[test]
fn test_fsview() {
    run!(paths_are_checked);
    run!(zones_by_name_in_any_case);
    run!(the_walk_follows_no_link);
    run!(grimoires_folders_by_identity);
    run!(a_listing_never_enters_a_link);
    run!(slices_keep_to_their_budget);
    run!(free_space);
    run!(the_root_is_the_hooks);
    let _ = std::fs::remove_dir_all(format!("{}/fsview-storage", tl_tmp()));
    done();
}
