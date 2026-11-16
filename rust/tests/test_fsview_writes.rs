// The storage layer's writes (src/fsview.rs and src/bgwork_copy.rs, 2.2, issue #117) on a tree of the host's own:
// a rename that never replaces an entry (renameat2's flag where Linux has it, and the fallback forced through a
// test seam so a run anywhere takes it too), a rename that changes only the case, the trash with its record and
// restore for a file, a folder and a notebook (both the name store_delete gives and the record form), a name taken
// on restore, the record read strictly, the busy check, and the copy job with the floor, a write that fails, and a
// source that changes. Every refusal of the allow-list. Nothing on screen calls these yet.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::bgwork_copy::*;
use grimoire::fsview::*;
use grimoire::prelude::*;
use std::os::unix::fs::{symlink, MetadataExt};
use testlib::*;

fn sp(s: &str) -> StoragePath {
    StoragePath::new(s.as_bytes()).unwrap_or_else(|| panic!("{s}"))
}

/// A tree made afresh: the storage root and Grimoire's folder in it (its store set up there), with a few files.
fn tree() -> (String, String) {
    let root = format!("{}/fsw-storage", tl_tmp());
    let _ = std::fs::remove_dir_all(&root);
    let g = format!("{root}/grimoire");
    for d in ["documents", "grimoire/import/Trip", "grimoire/import/done", "grimoire/templates", "grimoire/photos", "grimoire/fonts", "grimoire/PDFs"] {
        std::fs::create_dir_all(format!("{root}/{d}")).unwrap();
    }
    for (f, body) in [
        ("documents/a.pdf", &b"%PDF-1.4 a\n"[..]),
        ("grimoire/templates/a.png", b"template a"),
        ("grimoire/templates/b.png", b"template b"),
        ("grimoire/import/Book.pdf", b"%PDF-1.4 book\n"),
        ("grimoire/import/Trip/p1.png", b"one"),
        ("grimoire/import/Trip/p2.png", b"two"),
        ("grimoire/import/done/Old-20260101-000000.pdf", b"%PDF old"),
        ("grimoire/PDFs/x.pdf", b"%PDF mirror"),
        ("grimoire/settings.ini", b"version=1\n"),
    ] {
        std::fs::write(format!("{root}/{f}"), body).unwrap();
    }
    std::env::set_var("GRIMOIRE_STORAGE", &root);
    store_init(&g);
    (root, g)
}

fn open(root: &str, g: &str) -> FsStorage {
    FsStorage::open(root, g).unwrap()
}

fn stat_of(st: &FsStorage, p: &str) -> FsStat {
    st.stat(&sp(p), false).unwrap()
}

fn read(root: &str, p: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{root}/{p}")).ok()
}

fn names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// The trash as the screens will list it: FsListing over trash_dir, then fsview_trash_entries.
fn trash_list(st: &FsStorage) -> Vec<TrashEntry> {
    let mut l = FsListing::start(st, &st.trash_dir().unwrap(), false).unwrap();
    while !l.step(FS_SLICE_MS) {}
    fsview_trash_entries(&l.entries)
}

// Writes go only to what is inside Grimoire's import, templates, photos and fonts: never the storage's other
// folders, the folder itself, import/done, Grimoire's settings, PDFs, notebooks or trash; never to a name of
// Grimoire's own, a hidden name, or through a link.
fn writes_stay_in_grimoires_folders() {
    let (root, g) = tree();
    symlink("/etc", format!("{root}/grimoire/fonts/etc")).unwrap();
    symlink(format!("{root}/documents/a.pdf"), format!("{root}/grimoire/templates/link.png")).unwrap();
    let st = open(&root, &g);
    for p in [
        "documents/a.pdf",
        "grimoire/templates",
        "grimoire/import",
        "grimoire/import/done",
        "grimoire/settings.ini",
        "grimoire/PDFs/x.pdf",
        "grimoire/trash",
        "grimoire/notebooks",
        "grimoire",
    ] {
        let s = st.stat(&sp(p), false).unwrap_or_default();
        let r = st.rename(&sp(p), &s, b"other");
        let t = st.trash(&sp(p), &s);
        if r != Err(FsWriteErr::Refused) || t != Err(FsWriteErr::Refused) {
            eprintln!("  {p}: rename {r:?}, trash {t:?}");
        }
        check!(r == Err(FsWriteErr::Refused) && t == Err(FsWriteErr::Refused));
    }
    // a link is its own stat and is never written through
    let link = stat_of(&st, "grimoire/templates/link.png");
    check!(st.rename(&sp("grimoire/templates/link.png"), &link, b"l2.png") == Err(FsWriteErr::Refused));
    check!(st.trash(&sp("grimoire/templates/link.png"), &link) == Err(FsWriteErr::Refused));
    check!(read(&root, "documents/a.pdf").is_some());
    check!(matches!(st.trash(&sp("grimoire/fonts/etc/hosts"), &FsStat::default()), Err(FsWriteErr::Walk(FsErr::Link))));
    // the names: Grimoire's own, hidden, another folder, the rules
    let s = stat_of(&st, "grimoire/import/Trip");
    check!(st.rename(&sp("grimoire/import/Trip"), &s, b"done") == Err(FsWriteErr::Refused));
    check!(st.rename(&sp("grimoire/import/Trip"), &s, b"DONE") == Err(FsWriteErr::Refused));
    check!(st.rename(&sp("grimoire/import/Trip"), &s, b".Trip") == Err(FsWriteErr::Name(NameRule::LeadingDot)));
    let a = stat_of(&st, "grimoire/templates/a.png");
    check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"../a.png") == Err(FsWriteErr::Name(NameRule::Char(b'/'))));
    check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"a:b.png") == Err(FsWriteErr::Name(NameRule::Char(b':'))));
    check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"a.jpg") == Err(FsWriteErr::Name(NameRule::Extension)));
    check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"a.png ") == Err(FsWriteErr::Name(NameRule::TrailingSpace)));
    check!(read(&root, "grimoire/templates/a.png").as_deref() == Some(&b"template a"[..]));
    // a Grimoire whose folder is not inside the root writes nothing
    let outside = FsStorage::open(&format!("{root}/documents"), &g).unwrap();
    let d = outside.stat(&sp("a.pdf"), false).unwrap();
    check!(outside.trash(&sp("a.pdf"), &d) == Err(FsWriteErr::Refused));
    check!(outside.trash_dir().is_none());
}

// A rename never replaces an entry (the other file keeps its bytes), acts only on the item listed (a file that
// changed, or went, since: Changed), and renames a file and a folder. Both ways: renameat2's flag where the
// system has it, and the fallback (a look for the name, then renameat) forced.
fn a_rename_never_replaces() {
    for forced in [false, true] {
        fsview_rename_fallback_force(forced);
        let (root, g) = tree();
        let st = open(&root, &g);
        let a = stat_of(&st, "grimoire/templates/a.png");
        check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"b.png") == Err(FsWriteErr::Exists));
        check!(read(&root, "grimoire/templates/a.png").as_deref() == Some(&b"template a"[..]));
        check!(read(&root, "grimoire/templates/b.png").as_deref() == Some(&b"template b"[..]));
        // a hard link to the item under the new name is still another entry: refused, through the temporary name
        std::fs::hard_link(format!("{root}/grimoire/templates/a.png"), format!("{root}/grimoire/templates/same.png")).unwrap();
        let a = stat_of(&st, "grimoire/templates/a.png");
        check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"same.png") == Err(FsWriteErr::Exists));
        check!(names(&format!("{root}/grimoire/templates")) == ["a.png", "b.png", "same.png"]);
        std::fs::remove_file(format!("{root}/grimoire/templates/same.png")).unwrap();
        let a = stat_of(&st, "grimoire/templates/a.png");
        check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"Holiday.png") == Ok(sp("grimoire/templates/Holiday.png")));
        check!(read(&root, "grimoire/templates/Holiday.png").as_deref() == Some(&b"template a"[..]) && read(&root, "grimoire/templates/a.png").is_none());
        // the same inode: one rename, no copy
        check!(std::fs::metadata(format!("{root}/grimoire/templates/Holiday.png")).unwrap().ino() == a.ino);
        // changed since it was listed, or gone: Changed, and nothing moves
        let b = stat_of(&st, "grimoire/templates/b.png");
        std::fs::write(format!("{root}/grimoire/templates/b.png"), b"template b, longer").unwrap();
        check!(st.rename(&sp("grimoire/templates/b.png"), &b, b"c.png") == Err(FsWriteErr::Changed));
        check!(st.trash(&sp("grimoire/templates/b.png"), &b) == Err(FsWriteErr::Changed));
        check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"c.png") == Err(FsWriteErr::Changed));
        check!(read(&root, "grimoire/templates/c.png").is_none());
        // a folder
        let t = stat_of(&st, "grimoire/import/Trip");
        check!(st.rename(&sp("grimoire/import/Trip"), &t, b"Trip 2026") == Ok(sp("grimoire/import/Trip 2026")));
        check!(names(&format!("{root}/grimoire/import/Trip 2026")) == ["p1.png", "p2.png"]);
        // a new path of 600 bytes or more
        let t = stat_of(&st, "grimoire/import/Trip 2026");
        check!(st.rename(&sp("grimoire/import/Trip 2026"), &t, &[b'x'; 250]).is_ok());
        let deep = format!("grimoire/import/{}", "x".repeat(250));
        std::fs::create_dir_all(format!("{root}/{deep}/{}", "y".repeat(250))).unwrap();
        let deeper = format!("{deep}/{}", "y".repeat(250));
        std::fs::write(format!("{root}/{deeper}/f.png"), b"f").unwrap();
        let f = stat_of(&st, &format!("{deeper}/f.png"));
        check!(st.rename(&sp(&format!("{deeper}/f.png")), &f, format!("{}.png", "z".repeat(90)).as_bytes()) == Err(FsWriteErr::TooLong));
    }
    fsview_rename_fallback_force(false);
}

// A rename that changes only the case lands on the new name exactly (through a temporary name, so a storage that
// folds case does not take the new name for the old), leaves no temporary name behind, and keeps the item.
fn a_case_only_rename() {
    for forced in [false, true] {
        fsview_rename_fallback_force(forced);
        let (root, g) = tree();
        let st = open(&root, &g);
        let a = stat_of(&st, "grimoire/templates/a.png");
        check!(st.rename(&sp("grimoire/templates/a.png"), &a, b"A.png") == Ok(sp("grimoire/templates/A.png")));
        check!(names(&format!("{root}/grimoire/templates")) == ["A.png", "b.png"]);
        check!(std::fs::metadata(format!("{root}/grimoire/templates/A.png")).unwrap().ino() == a.ino);
        // a folder, and a case change of the extension
        let t = stat_of(&st, "grimoire/import/Trip");
        check!(st.rename(&sp("grimoire/import/Trip"), &t, b"TRIP").is_ok());
        check!(names(&format!("{root}/grimoire/import")) == ["Book.pdf", "TRIP", "done"]);
        let b = stat_of(&st, "grimoire/templates/b.png");
        check!(st.rename(&sp("grimoire/templates/b.png"), &b, b"b.PNG").is_ok());
        check!(names(&format!("{root}/grimoire/templates")) == ["A.png", "b.PNG"]);
        // the same name: nothing to do
        let b = stat_of(&st, "grimoire/templates/b.PNG");
        check!(st.rename(&sp("grimoire/templates/b.PNG"), &b, b"b.PNG") == Ok(sp("grimoire/templates/b.PNG")));
    }
    fsview_rename_fallback_force(false);
}

// Waits for the background children to end, reaping each (at most 30 s). The last notify result.
fn settle() -> i32 {
    let mut r = 0;
    let t0 = grimoire::util::real_now_ms();
    while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 30_000 {
        let got = bg_poll();
        if got != 0 {
            r = got;
        }
        sleep_ms(5);
    }
    check!(!bg_busy());
    r
}

// What an import holds is never renamed or trashed: the PDF a PDF import works on, a folder of pictures (a scan's,
// by its path under Grimoire's folder), what is inside it, and the folder a held PDF is in; nor is anything put back
// into a held folder. The jobs wait in the queue behind another (an export of no notebook, which fails), so no
// import runs; once they leave it, the items are free again.
fn what_an_import_holds_stays() {
    let (root, g) = tree();
    std::fs::create_dir_all(format!("{root}/grimoire/import/Work")).unwrap();
    std::fs::write(format!("{root}/grimoire/import/Work/A.pdf"), b"%PDF a").unwrap();
    let st = open(&root, &g);
    // a picture of the folder in the trash, to be put back while the folder is held
    let p2 = stat_of(&st, "grimoire/import/Trip/p2.png");
    let back = st.trash(&sp("grimoire/import/Trip/p2.png"), &p2).unwrap();
    let mirror = format!("{root}/mirror");
    bg_queue(BgKind::MirrorPdf, Some("20261115-000000-dead"), Some(&mirror), 300.0, false);
    check!(bg_children() == 1);
    let trip = format!("{g}/import/Trip");
    bg_queue(BgKind::PdfImport, Some("20261115-000000-beef"), Some("grimoire/import/Book.pdf"), 300.0, false);
    bg_queue(BgKind::PdfImport, Some("20261115-000000-f00d"), Some("grimoire/import/Work/A.pdf"), 300.0, false);
    bg_queue(BgKind::ImportFolder, Some("20261115-000000-cafe"), Some(&trip), 300.0, false);
    check!(bg_children() == 1);
    for p in ["grimoire/import/Book.pdf", "grimoire/import/Trip", "grimoire/import/Trip/p1.png", "grimoire/import/Work", "grimoire/import/Work/A.pdf"] {
        let s = stat_of(&st, p);
        check!(st.busy(&sp(p)));
        check!(st.rename(&sp(p), &s, b"Other") == Err(FsWriteErr::Busy));
        check!(st.trash(&sp(p), &s) == Err(FsWriteErr::Busy));
    }
    check!(!st.busy(&sp("grimoire/templates/a.png")) && !st.busy(&sp("grimoire/import/done/Old-20260101-000000.pdf")));
    check!(st.restore(&back) == Err(FsWriteErr::Busy));
    check!(bg_unqueue(BgKind::PdfImport, "20261115-000000-beef", "grimoire/import/Book.pdf"));
    check!(bg_unqueue(BgKind::PdfImport, "20261115-000000-f00d", "grimoire/import/Work/A.pdf"));
    check!(bg_unqueue(BgKind::ImportFolder, "20261115-000000-cafe", &trip));
    settle();
    let s = stat_of(&st, "grimoire/import/Book.pdf");
    check!(!st.busy(&sp("grimoire/import/Book.pdf")));
    check!(st.rename(&sp("grimoire/import/Book.pdf"), &s, b"Book 1.pdf").is_ok());
    check!(names(&format!("{root}/grimoire/import")) == ["Book 1.pdf", "Trip", "Work", "done"]);
    check!(st.restore(&back) == Ok(Restored::File(sp("grimoire/import/Trip/p2.png"))));
}

// A file and a folder to the trash and back: one rename each way (the same inode), the record beside the item
// (where it was, when), the trash listing it by the time in its name, its folder in the trash gone after the
// restore. Both ways of renaming.
fn trash_and_restore_a_file_and_a_folder() {
    for forced in [false, true] {
        fsview_rename_fallback_force(forced);
        let (root, g) = tree();
        let st = open(&root, &g);
        let a = stat_of(&st, "grimoire/templates/a.png");
        let before = wall_time();
        let entry = st.trash(&sp("grimoire/templates/a.png"), &a).unwrap();
        let w = String::from_utf8(entry.clone()).unwrap();
        check!(w.starts_with("files-") && w.len() == "files-20261115-101010-0a9f".len());
        check!(read(&root, "grimoire/templates/a.png").is_none());
        check!(names(&format!("{g}/trash/{w}")) == [".grimoire-trash", "a.png"]);
        check!(std::fs::metadata(format!("{g}/trash/{w}/a.png")).unwrap().ino() == a.ino);
        let rec = st.trash_record(&entry).unwrap();
        check!(rec.from == sp("grimoire/templates/a.png") && rec.time >= before && rec.time <= wall_time());
        let text = read(&g, &format!("trash/{w}/.grimoire-trash")).unwrap();
        check!(text.starts_with(b"# Grimoire") && text.ends_with(format!("\nfrom=grimoire/templates/a.png\ntime={}\n", rec.time).as_bytes()));
        // the folder of pictures too, a moment later
        let t = stat_of(&st, "grimoire/import/Trip");
        let folder = st.trash(&sp("grimoire/import/Trip"), &t).unwrap();
        check!(names(&format!("{g}/trash/{}", String::from_utf8_lossy(&folder))) == [".grimoire-trash", "Trip"]);
        let list = trash_list(&st);
        check!(list.len() == 2 && list.iter().all(|e| e.kind == TrashKind::File));
        check!(list.iter().all(|e| (e.time - rec.time).abs() <= 2));
        // back
        check!(st.restore(&entry) == Ok(Restored::File(sp("grimoire/templates/a.png"))));
        check!(read(&root, "grimoire/templates/a.png").as_deref() == Some(&b"template a"[..]));
        check!(std::fs::metadata(format!("{root}/grimoire/templates/a.png")).unwrap().ino() == a.ino);
        check!(!std::path::Path::new(&format!("{g}/trash/{w}")).exists());
        check!(st.restore(&folder) == Ok(Restored::File(sp("grimoire/import/Trip"))));
        check!(names(&format!("{root}/grimoire/import/Trip")) == ["p1.png", "p2.png"]);
        check!(names(&format!("{g}/trash")).is_empty());
        // an entry restored already is not there to restore
        check!(st.restore(&entry).is_err());
    }
    fsview_rename_fallback_force(false);
}

// A name taken where the item goes back: " 2", then " 3" (before a file's extension, at a folder's end).
fn a_name_taken_on_restore() {
    let (root, g) = tree();
    let st = open(&root, &g);
    let mut entries = Vec::new();
    for _ in 0..3 {
        let a = stat_of(&st, "grimoire/templates/a.png");
        entries.push(st.trash(&sp("grimoire/templates/a.png"), &a).unwrap());
        std::fs::write(format!("{root}/grimoire/templates/a.png"), b"a new a").unwrap();
    }
    check!(st.restore(&entries[0]) == Ok(Restored::File(sp("grimoire/templates/a 2.png"))));
    check!(st.restore(&entries[1]) == Ok(Restored::File(sp("grimoire/templates/a 3.png"))));
    check!(read(&root, "grimoire/templates/a.png").as_deref() == Some(&b"a new a"[..]));
    check!(read(&root, "grimoire/templates/a 2.png").as_deref() == Some(&b"template a"[..]));
    let t = stat_of(&st, "grimoire/import/Trip");
    let e = st.trash(&sp("grimoire/import/Trip"), &t).unwrap();
    std::fs::create_dir(format!("{root}/grimoire/import/Trip")).unwrap();
    check!(st.restore(&e) == Ok(Restored::File(sp("grimoire/import/Trip 2"))));
    check!(names(&format!("{root}/grimoire/import/Trip 2")) == ["p1.png", "p2.png"]);
    let _ = entries;
}

// A restore refused: the folder it came from is gone, the record is missing or does not read (a link, a FIFO,
// too long, a key it does not know), it names a place outside Grimoire's folders for the user's files, or the
// item in the trash is not the one it names. The item stays in the trash each time. A FIFO for a record is never
// waited on.
fn a_restore_refused() {
    let (root, g) = tree();
    std::fs::create_dir_all(format!("{root}/grimoire/import/Work")).unwrap();
    std::fs::write(format!("{root}/grimoire/import/Work/A.pdf"), b"%PDF a").unwrap();
    let st = open(&root, &g);
    let w = stat_of(&st, "grimoire/import/Work/A.pdf");
    let gone = st.trash(&sp("grimoire/import/Work/A.pdf"), &w).unwrap();
    std::fs::remove_dir(format!("{root}/grimoire/import/Work")).unwrap();
    check!(st.restore(&gone) == Err(FsWriteErr::FolderGone));
    let ws = String::from_utf8(gone.clone()).unwrap();
    check!(names(&format!("{g}/trash/{ws}")) == [".grimoire-trash", "A.pdf"]);
    let a = stat_of(&st, "grimoire/templates/a.png");
    let e = st.trash(&sp("grimoire/templates/a.png"), &a).unwrap();
    let es = String::from_utf8(e.clone()).unwrap();
    let rec = format!("{g}/trash/{es}/.grimoire-trash");
    let good = std::fs::read(&rec).unwrap();
    let put = |body: &[u8]| {
        let _ = std::fs::remove_file(&rec);
        std::fs::write(&rec, body).unwrap();
    };
    for bad in [
        &b"from=documents/a.png\ntime=5\n"[..],
        b"from=grimoire/settings.ini\ntime=5\n",
        b"from=grimoire/templates\ntime=5\n",
        b"from=grimoire/import/done\ntime=5\n",
        b"from=grimoire/notebooks/a.png\ntime=5\n",
    ] {
        put(bad);
        check!(st.restore(&e) == Err(FsWriteErr::Refused));
    }
    put(b"from=grimoire/photos/b.png\ntime=5\n");
    check!(st.restore(&e) == Err(FsWriteErr::BadRecord)); // (the trash holds a.png, not b.png)
    for bad in [&b"from=grimoire/templates/a.png\ntime=5\nkind=file\n"[..], b"from=grimoire/templates/a.png\n", &[b'#'; 5000]] {
        put(bad);
        check!(st.restore(&e) == Err(FsWriteErr::BadRecord));
        check!(st.trash_record(&e) == Err(FsWriteErr::BadRecord));
    }
    std::fs::remove_file(&rec).unwrap();
    check!(st.restore(&e) == Err(FsWriteErr::BadRecord));
    std::fs::write(format!("{root}/elsewhere"), &good).unwrap();
    symlink(format!("{root}/elsewhere"), &rec).unwrap();
    check!(st.restore(&e) == Err(FsWriteErr::BadRecord));
    std::fs::remove_file(&rec).unwrap();
    let fifo = std::ffi::CString::new(rec.as_str()).unwrap();
    // SAFETY: a NUL-terminated path.
    check!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) } == 0);
    let t0 = grimoire::util::real_now_ms();
    check!(st.restore(&e) == Err(FsWriteErr::BadRecord));
    check!(grimoire::util::real_now_ms().wrapping_sub(t0) < 1000);
    std::fs::remove_file(&rec).unwrap();
    put(&good);
    check!(names(&format!("{g}/trash/{es}")) == [".grimoire-trash", "a.png"]);
    check!(st.restore(&e) == Ok(Restored::File(sp("grimoire/templates/a.png"))));
    // names the trash does not know, a page, and paths
    for bad in [&b"notes.txt"[..], b"", b"..", b"a/b", b"files-x"] {
        check!(st.restore(bad).is_err());
    }
}

// A notebook goes back to notebooks/<id> from the name store_delete gives it (every version) and from a record whose
// from= names that folder; refused while the id is taken. A page in the trash is listed and not restored.
fn notebooks_and_pages_in_the_trash() {
    for forced in [false, true] {
        fsview_rename_fallback_force(forced);
        let (root, g) = tree();
        store_set_device(300.0, 600, 800);
        let mut nb = NotebookInfo::default();
        check!(store_create("Trip notes", "School", PAPER_BLANK, &mut nb) == 0);
        check!(store_add_page(&mut nb, 1, PAPER_BLANK) == 0);
        let id = nb.id.as_str().to_string();
        check!(store_delete_page(&mut nb, 1) == 0);
        check!(store_delete(&id) == 0);
        let st = open(&root, &g);
        let list = trash_list(&st);
        check!(list.len() == 2);
        let nbe = list.iter().find(|e| e.kind == TrashKind::Notebook).unwrap().clone();
        let page = list.iter().find(|e| e.kind == TrashKind::Page).unwrap().clone();
        check!(nbe.id == id && page.id == id && page.page > 1 && nbe.time > 0);
        check!(st.restore(&page.name) == Err(FsWriteErr::Refused));
        check!(st.trash_record(&nbe.name) == Err(FsWriteErr::Refused));
        // the name store_delete gave it
        check!(st.restore(&nbe.name) == Ok(Restored::Notebook(id.clone())));
        let mut back = NotebookInfo::default();
        check!(store_load_info(&id, &mut back) == 0 && back.name.as_str() == "Trip notes" && back.folder.as_str() == "School");
        check!(trash_list(&st).len() == 1);
        // the record form: the notebook's folder in files-<stamp>-<4 hex>/ beside a record naming notebooks/<id>
        check!(store_delete(&id) == 0);
        let old = trash_list(&st).into_iter().find(|e| e.kind == TrashKind::Notebook).unwrap();
        let w = format!("{g}/trash/files-20261115-101010-0a9f");
        std::fs::create_dir(&w).unwrap();
        std::fs::rename(format!("{g}/trash/{}", String::from_utf8_lossy(&old.name)), format!("{w}/{id}")).unwrap();
        std::fs::write(format!("{w}/.grimoire-trash"), format!("from=grimoire/notebooks/{id}\ntime=1763200000\n")).unwrap();
        // ... while a notebook of that id is there: refused, and it stays
        std::fs::create_dir(format!("{g}/notebooks/{id}")).unwrap();
        check!(st.restore(b"files-20261115-101010-0a9f") == Err(FsWriteErr::Exists));
        check!(names(&w) == [".grimoire-trash".to_string(), id.clone()]);
        std::fs::remove_dir(format!("{g}/notebooks/{id}")).unwrap();
        check!(st.restore(b"files-20261115-101010-0a9f") == Ok(Restored::Notebook(id.clone())));
        check!(store_load_info(&id, &mut back) == 0 && back.name.as_str() == "Trip notes");
        check!(!std::path::Path::new(&w).exists());
        // a record that names notebooks/<id> over an item of another name, or a file: refused
        check!(store_delete(&id) == 0);
        let old = trash_list(&st).into_iter().find(|e| e.kind == TrashKind::Notebook).unwrap();
        std::fs::create_dir(&w).unwrap();
        std::fs::rename(format!("{g}/trash/{}", String::from_utf8_lossy(&old.name)), format!("{w}/{id}")).unwrap();
        std::fs::write(format!("{w}/.grimoire-trash"), "from=grimoire/notebooks/20261115-000000-0000\ntime=1\n").unwrap();
        check!(st.restore(b"files-20261115-101010-0a9f") == Err(FsWriteErr::BadRecord));
        let _ = root;
    }
    fsview_rename_fallback_force(false);
}

// The copy request: refused for a source that is no plain file, has changed, sits in Grimoire's own folders, or
// would leave less than the floor; else queued, and the job puts the bytes in the folder under the source's name
// (" 2" when taken), reported as a job the user waits on.
fn the_copy_request() {
    let (root, g) = tree();
    std::fs::write(format!("{root}/documents/photo.png"), vec![7u8; 200_000]).unwrap();
    let fifo = std::ffi::CString::new(format!("{root}/documents/pipe.png")).unwrap();
    // SAFETY: a NUL-terminated path.
    check!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) } == 0);
    let st = open(&root, &g);
    let p = stat_of(&st, "documents/photo.png");
    check!(copy_request(&st, &sp("documents"), &stat_of(&st, "documents"), CopyTo::Photos) == Err(FsWriteErr::Walk(FsErr::NotFile)));
    check!(copy_request(&st, &sp("documents/pipe.png"), &stat_of(&st, "documents/pipe.png"), CopyTo::Photos) == Err(FsWriteErr::Walk(FsErr::NotFile)));
    check!(copy_request(&st, &sp("grimoire/PDFs/x.pdf"), &stat_of(&st, "grimoire/PDFs/x.pdf"), CopyTo::Photos) == Err(FsWriteErr::Refused));
    check!(copy_request(&st, &sp("grimoire/settings.ini"), &stat_of(&st, "grimoire/settings.ini"), CopyTo::Photos) == Err(FsWriteErr::Refused));
    let mut other = p;
    other.mtime -= 1;
    check!(copy_request(&st, &sp("documents/photo.png"), &other, CopyTo::Photos) == Err(FsWriteErr::Changed));
    copy_seams(Some((100 << 20) + 199_999), None);
    check!(copy_request(&st, &sp("documents/photo.png"), &p, CopyTo::Photos) == Err(FsWriteErr::NoRoom));
    copy_seams(Some((100 << 20) + 200_000), None);
    check!(copy_request(&st, &sp("documents/photo.png"), &p, CopyTo::Photos) == Ok(()));
    check!(settle() == 1);
    copy_seams(None, None);
    check!(read(&root, "grimoire/photos/photo.png") == read(&root, "documents/photo.png"));
    check!(copy_request(&st, &sp("documents/photo.png"), &p, CopyTo::Photos) == Ok(()));
    check!(settle() == 1);
    check!(names(&format!("{g}/photos")) == ["photo 2.png", "photo.png"]);
    check!(copy_request(&st, &sp("documents/a.pdf"), &stat_of(&st, "documents/a.pdf"), CopyTo::Fonts) == Ok(()));
    check!(settle() == 1);
    check!(read(&g, "fonts/a.pdf").as_deref() == Some(&b"%PDF-1.4 a\n"[..]));
    // a source that changes after the request, before the job reads it: the job refuses, nothing is left
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "400");
    let p = stat_of(&st, "documents/photo.png");
    check!(copy_request(&st, &sp("documents/photo.png"), &p, CopyTo::Templates) == Ok(()));
    std::fs::write(format!("{root}/documents/photo.png"), vec![8u8; 200_001]).unwrap();
    check!(settle() == -1);
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    check!(names(&format!("{g}/templates")) == ["a.png", "b.png"]);
}

// The job on its own (as the child runs it): it refuses a source that is no plain file or a link, one that is not
// the file listed, a copy that would go under the floor, and a write that fails (a full storage): nothing is left
// in the folder each time. A temporary file a dead copy left is swept; one of a live process is not.
fn the_copy_job() {
    let (root, g) = tree();
    std::fs::write(format!("{root}/documents/font.ttf"), vec![1u8; 300_000]).unwrap();
    symlink(format!("{root}/documents/font.ttf"), format!("{root}/documents/link.ttf")).unwrap();
    let st = open(&root, &g);
    let f = stat_of(&st, "documents/font.ttf");
    let arg = |s: &FsStat, p: &str| format!("{}.{}.{}.{}:{}", s.dev, s.ino, s.size, s.mtime, p);
    let fonts = format!("{g}/fonts");
    check!(bg_copy_file("fonts", &arg(&stat_of(&st, "documents"), "documents")) == -1);
    check!(bg_copy_file("fonts", &arg(&f, "documents/link.ttf")) == -1);
    check!(bg_copy_file("notebooks", &arg(&f, "documents/font.ttf")) == -1);
    check!(bg_copy_file("fonts", "not a request") == -1);
    let mut moved = f;
    moved.size += 1;
    check!(bg_copy_file("fonts", &arg(&moved, "documents/font.ttf")) == -1);
    check!(names(&fonts).is_empty());
    copy_seams(Some((100 << 20) + 299_999), None);
    check!(bg_copy_file("fonts", &arg(&f, "documents/font.ttf")) == -1);
    check!(names(&fonts).is_empty());
    copy_seams(None, Some(100_000));
    check!(bg_copy_file("fonts", &arg(&f, "documents/font.ttf")) == -1);
    check!(names(&fonts).is_empty());
    copy_seams(None, None);
    // leftovers: a dead process's temporary file goes; a live one's (this process's parent) stays
    // (a process id no system gives: above every pid_max)
    let dead = format!("{fonts}/.grimoire-copy-{}.part", i32::MAX);
    // SAFETY: getppid has no preconditions.
    let live = format!("{fonts}/.grimoire-copy-{}.part", unsafe { libc::getppid() });
    std::fs::write(&dead, b"half").unwrap();
    std::fs::write(&live, b"half").unwrap();
    check!(bg_copy_file("fonts", &arg(&f, "documents/font.ttf")) == 0);
    check!(read(&g, "fonts/font.ttf") == read(&root, "documents/font.ttf"));
    let left = names(&fonts);
    check!(left.len() == 2 && left.contains(&"font.ttf".to_string()) && left.iter().any(|n| format!("{fonts}/{n}") == live));
    std::fs::remove_file(&live).unwrap();
    // the folder made where it is missing
    std::fs::remove_dir_all(format!("{g}/photos")).unwrap();
    check!(bg_copy_file("photos", &arg(&f, "documents/font.ttf")) == 0);
    check!(names(&format!("{g}/photos")) == ["font.ttf"]);
}

#[test]
fn test_fsview_writes() {
    run!(writes_stay_in_grimoires_folders);
    run!(a_rename_never_replaces);
    run!(a_case_only_rename);
    run!(what_an_import_holds_stays);
    run!(trash_and_restore_a_file_and_a_folder);
    run!(a_name_taken_on_restore);
    run!(a_restore_refused);
    run!(notebooks_and_pages_in_the_trash);
    run!(the_copy_request);
    run!(the_copy_job);
    std::env::remove_var("GRIMOIRE_STORAGE");
    let _ = std::fs::remove_dir_all(format!("{}/fsw-storage", tl_tmp()));
    done();
}
