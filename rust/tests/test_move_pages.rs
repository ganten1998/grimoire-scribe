// Move to... and Copy to notebook... (docs/beta2/BRIEFS.md, Lane A): store_move_pages,
// store_copy_pages_to, nav_target_across and nav_copy_pages_across.
use grimoire::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn fresh_root(tag: &str) -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("move-pages-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, 600, 800);
    root
}

fn dir_of(nb: &NotebookInfo) -> String {
    let mut d = String::new();
    assert_eq!(store_notebook_dir(nb.id.as_str(), &mut d, 700), 0);
    d
}

fn squiggle(t: ToolKind, y: f32, seed: u32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    let mut s = stroke_new(t, &p, seed);
    for i in 0..20 {
        let f = i as f32;
        let q = PenSample { x: 50.0 + f * 10.0, y: y + (f * 0.4).sin() * 10.0, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i * 4 };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

/// A notebook of n pages, page k (from 1) with k strokes, so each page's bytes are its own.
fn notebook(name: &str, n: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_MEDIUM, &mut nb), 0);
    while nb.npages() < n as i32 {
        let at = nb.npages();
        assert_eq!(store_add_page(&mut nb, at, PAPER_MEDIUM), 0);
    }
    for k in 0..n {
        let (mut w, mut h) = (0, 0);
        store_notebook_page_size(&nb, &mut w, &mut h);
        let mut p = page_new(w, h, PAPER_MEDIUM);
        for j in 0..=k {
            page_add(&mut p, squiggle(TOOL_PENCIL, 100.0 + 40.0 * j as f32, 0x100 + j as u32));
        }
        assert_eq!(store_save_page(&nb, k as i32, &p), 0);
    }
    nb
}

fn ids_on_disk(nb: &NotebookInfo) -> Vec<u32> {
    let mut back = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut back), 0);
    back.page_ids
}

#[test]
fn moving_pages_keeps_their_order_and_writes_only_the_list() {
    let root = fresh_root("move");
    let mut nb = notebook("Move", 6); // ids 1..=6
    let dir = dir_of(&nb);
    let bytes = |pid: u32| std::fs::read(format!("{dir}/p{pid}.qpg")).unwrap();
    let before: Vec<Vec<u8>> = (1..=6).map(bytes).collect();
    let listing = |d: &str| {
        let mut v: Vec<String> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    };
    let files = listing(&dir);

    // One page to the end; then back to the front.
    nb.last_page = 1; // page id 2 open
    assert_eq!(store_move_pages(&mut nb, &[1], 6), 0);
    assert_eq!(nb.page_ids, vec![1, 3, 4, 5, 6, 2]);
    assert_eq!(nb.last_page, 5, "last_page follows the page it named");
    assert_eq!(ids_on_disk(&nb), nb.page_ids);
    assert_eq!(store_move_pages(&mut nb, &[5], 0), 0);
    assert_eq!(nb.page_ids, vec![2, 1, 3, 4, 5, 6]);
    assert_eq!(nb.last_page, 0);

    // Ticked pages apart from each other stand together, in their order, before page `to`.
    assert_eq!(store_move_pages(&mut nb, &[5, 0, 3], 2), 0); // 6, 2, 4 before page 3 (id 3)
    assert_eq!(nb.page_ids, vec![1, 2, 4, 6, 3, 5]);
    // A `to` that is itself moved: before the first page after it that is not.
    assert_eq!(store_move_pages(&mut nb, &[1, 2], 2), 0); // 2, 4 before 6 is the same place
    assert_eq!(nb.page_ids, vec![1, 2, 4, 6, 3, 5]);
    assert_eq!(store_move_pages(&mut nb, &[0, 1], 1), 0); // 1, 2 before 4: unchanged
    assert_eq!(nb.page_ids, vec![1, 2, 4, 6, 3, 5]);
    assert_eq!(store_move_pages(&mut nb, &[2, 4], 6), 0); // 4, 3 at the end
    assert_eq!(nb.page_ids, vec![1, 2, 6, 5, 4, 3]);
    // Every page: nothing to do.
    assert_eq!(store_move_pages(&mut nb, &[0, 1, 2, 3, 4, 5], 3), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 6, 5, 4, 3]);
    // Bad input changes nothing.
    for (ix, to) in [(vec![], 0), (vec![6], 0), (vec![-1], 0), (vec![1, 1], 0), (vec![1], 7), (vec![1], -1)] {
        assert_eq!(store_move_pages(&mut nb, &ix, to), -1, "{ix:?} {to}");
        assert_eq!(nb.page_ids, vec![1, 2, 6, 5, 4, 3]);
    }
    assert_eq!(ids_on_disk(&nb), nb.page_ids);
    // Page files and every other file stay as they were: only notebook.ini changed.
    for pid in 1..=6u32 {
        assert_eq!(bytes(pid), before[pid as usize - 1]);
    }
    assert_eq!(listing(&dir), files);

    // notebook.ini that cannot be written: the move fails, the list in memory and on disk unchanged.
    let ini = std::fs::read(format!("{dir}/notebook.ini")).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let last = nb.last_page;
    assert_eq!(store_move_pages(&mut nb, &[0], 6), -1);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(nb.page_ids, vec![1, 2, 6, 5, 4, 3]);
    assert_eq!(nb.last_page, last);
    assert_eq!(std::fs::read(format!("{dir}/notebook.ini")).unwrap(), ini);
    std::fs::remove_dir_all(root).unwrap();
}

/// Annotated notebooks: Move is allowed. A book page added later goes just before the first page, in the
/// order as it is now, that holds a later book page (annotate_insert_at), whatever the order became.
#[test]
fn a_moved_annotated_notebook_keeps_inserting_by_book_order() {
    let root = fresh_root("annotate");
    let mut nb = notebook("Book", 4); // ids 1..=4
    let dir = dir_of(&nb);
    std::fs::write(format!("{dir}/source.ini"), "book=abc\nkind=paged\npage.10.100=1\npage.20.100=2\npage.30.100=3\n").unwrap();
    let mut m = AnnotateMap::default();
    assert_eq!(annotate_map_load(nb.id.as_str(), &mut m), 0);
    assert_eq!(annotate_insert_at(&nb, &m, 15, 100), 1); // before id 2 (book page 20)
    // Book page 30's page first, the note page (id 4) between 10 and 20.
    assert_eq!(store_move_pages(&mut nb, &[2], 0), 0);
    assert_eq!(store_move_pages(&mut nb, &[3], 2), 0);
    assert_eq!(nb.page_ids, vec![3, 1, 4, 2]);
    assert_eq!(annotate_insert_at(&nb, &m, 15, 100), 0, "id 3 (page 30) is now the first page with a later book page");
    assert_eq!(annotate_insert_at(&nb, &m, 35, 100), 4, "nothing later: the end");
    assert_eq!(annotate_insert_at(&nb, &m, 5, 100), 0);
    // Back to book order: the rule gives the place it gave before.
    assert_eq!(store_move_pages(&mut nb, &[0], 4), 0);
    assert_eq!(nb.page_ids, vec![1, 4, 2, 3]);
    assert_eq!(annotate_insert_at(&nb, &m, 15, 100), 2);
    // source.ini is not written by a move.
    assert!(std::fs::read_to_string(format!("{dir}/source.ini")).unwrap().starts_with("book=abc\nkind=paged\npage.10.100=1"));
    std::fs::remove_dir_all(root).unwrap();
}

/// A page move is one undo step (owner, 2026-11-02): the record puts the exact order back through the same
/// path (notebook.ini alone), last_page following its page, and only while the move is still the newest
/// change to the notebook's pages.
#[test]
fn a_page_move_undoes_to_the_exact_order() {
    let root = fresh_root("undo");
    let mut nb = notebook("Undo", 6); // ids 1..=6
    let dir = dir_of(&nb);
    let files_before: Vec<(String, Vec<u8>)> = {
        let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap()).filter(|e| e.file_name() != "notebook.ini")
            .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap())).collect();
        v.sort();
        v
    };
    // Pages apart from each other: 6, 2 and 4 before page 3, page id 4 open.
    nb.last_page = 3;
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[5, 1, 3], 2, &mut undo), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 4, 6, 3, 5]);
    assert_eq!(nb.last_page, 2);
    let u = undo.clone().expect("a move that changed the order is recorded");
    assert_eq!(u.before, vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(u.after, nb.page_ids);
    assert!(u.applies(&nb));
    assert_eq!(store_undo_move_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 3, 4, 5, 6], "the exact order before");
    assert_eq!(nb.last_page, 3, "last_page follows its page back");
    assert_eq!(ids_on_disk(&nb), nb.page_ids, "through notebook.ini");
    assert!(!u.applies(&nb), "undone once: the order is no longer the move's");
    assert_eq!(store_undo_move_pages(&mut nb, &u), -1);
    assert_eq!(nb.page_ids, vec![1, 2, 3, 4, 5, 6]);
    // Nothing else was written: page files, bg/ and the rest stay byte for byte.
    let files_after: Vec<(String, Vec<u8>)> = {
        let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap()).filter(|e| e.file_name() != "notebook.ini")
            .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap())).collect();
        v.sort();
        v
    };
    assert_eq!(files_after, files_before);

    // A move that changes nothing leaves the last record as it was.
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[0], 6, &mut undo), 0);
    assert_eq!(nb.page_ids, vec![2, 3, 4, 5, 6, 1]);
    let first = undo.clone().unwrap();
    assert_eq!(store_move_pages_undo(&mut nb, &[0, 1], 1, &mut undo), 0); // already there
    assert_eq!(undo.as_ref(), Some(&first));
    // Another move replaces it: undo takes back only the newest.
    assert_eq!(store_move_pages_undo(&mut nb, &[5], 0, &mut undo), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 3, 4, 5, 6]);
    assert!(!first.applies(&nb));
    assert_eq!(store_undo_move_pages(&mut nb, &first), -1);
    let second = undo.clone().unwrap();
    assert_eq!(store_undo_move_pages(&mut nb, &second), 0);
    assert_eq!(nb.page_ids, vec![2, 3, 4, 5, 6, 1]);

    // A change to the pages since (a page added, one deleted, one copied): the record is refused, the
    // notebook stays as it is.
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[5], 0, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(store_add_page(&mut nb, 6, PAPER_MEDIUM), 0);
    let changed = nb.page_ids.clone();
    assert!(!u.applies(&nb));
    assert_eq!(store_undo_move_pages(&mut nb, &u), -1);
    assert_eq!(nb.page_ids, changed);
    assert_eq!(ids_on_disk(&nb), changed);
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[1], 0, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(store_delete_page(&mut nb, 3), 0);
    assert_eq!(store_undo_move_pages(&mut nb, &u), -1);
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[1], 0, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(store_duplicate_page(&mut nb, 0, 1), 0);
    assert_eq!(store_undo_move_pages(&mut nb, &u), -1);
    // Another notebook with the same order: never.
    let mut other = notebook("Other", 2);
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut other, &[0], 2, &mut undo), 0);
    let mut u = undo.unwrap();
    u.notebook = nb.id.as_str().to_string();
    assert_eq!(store_undo_move_pages(&mut other, &u), -1);
    assert_eq!(other.page_ids, vec![2, 1]);

    // notebook.ini that cannot be written: the undo fails, the record still applies (to try again).
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[0], 2, &mut undo), 0);
    let u = undo.unwrap();
    let moved = nb.page_ids.clone();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    assert_eq!(store_undo_move_pages(&mut nb, &u), -1);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(nb.page_ids, moved);
    assert!(u.applies(&nb));
    assert_eq!(store_undo_move_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, u.before);
    std::fs::remove_dir_all(root).unwrap();
}

/// An annotated notebook's move undoes the same way: source.ini, which names pages by id, is never
/// written, and the book-order insertion rule gives what it gave before the move.
#[test]
fn an_annotated_notebook_move_undoes_the_same_way() {
    let root = fresh_root("undo-annotate");
    let mut nb = notebook("Book", 4);
    let dir = dir_of(&nb);
    let source = "book=abc\nkind=paged\npage.10.100=1\npage.20.100=2\npage.30.100=3\n";
    std::fs::write(format!("{dir}/source.ini"), source).unwrap();
    let mut m = AnnotateMap::default();
    assert_eq!(annotate_map_load(nb.id.as_str(), &mut m), 0);
    let at_before = annotate_insert_at(&nb, &m, 15, 100);
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[2], 0, &mut undo), 0); // book page 30's page first
    assert_eq!(nb.page_ids, vec![3, 1, 2, 4]);
    assert_eq!(annotate_insert_at(&nb, &m, 15, 100), 0);
    assert_eq!(store_undo_move_pages(&mut nb, &undo.unwrap()), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 3, 4]);
    assert_eq!(annotate_insert_at(&nb, &m, 15, 100), at_before);
    assert_eq!(std::fs::read_to_string(format!("{dir}/source.ini")).unwrap(), source);
    std::fs::remove_dir_all(root).unwrap();
}

/// store_move_order: the order a drop makes, asked before the move (a drop back in place changes nothing).
#[test]
fn the_order_a_drop_would_make() {
    let ids = [1u32, 2, 3, 4, 5];
    assert_eq!(store_move_order(&ids, &[1], 4), Some(vec![1, 3, 4, 2, 5]));
    assert_eq!(store_move_order(&ids, &[1], 1), Some(ids.to_vec()), "before itself: unchanged");
    assert_eq!(store_move_order(&ids, &[1], 2), Some(ids.to_vec()), "after itself: unchanged");
    assert_eq!(store_move_order(&ids, &[0, 4], 5), Some(vec![2, 3, 4, 1, 5]));
    assert_eq!(store_move_order(&ids, &[3], 0), Some(vec![4, 1, 2, 3, 5]));
    assert_eq!(store_move_order(&ids, &[], 0), None);
    assert_eq!(store_move_order(&ids, &[5], 0), None);
    assert_eq!(store_move_order(&ids, &[1], 6), None);
}

#[test]
fn targets_across_notebooks() {
    assert_eq!(nav_target_across("p12", "SRC", "DST").as_deref(), Some("n:SRC:p12"));
    assert_eq!(nav_target_across("n:DST:p7", "SRC", "DST").as_deref(), Some("p7"));
    assert_eq!(nav_target_across("index", "SRC", "DST"), None);
    assert_eq!(nav_target_across("n:OTHER:p3", "SRC", "DST").as_deref(), Some("n:OTHER:p3"));
    assert_eq!(nav_target_across("n:SRC:p3", "SRC", "DST").as_deref(), Some("n:SRC:p3"));
    assert_eq!(nav_target_across("future-target", "SRC", "DST").as_deref(), Some("future-target"));
    assert_eq!(nav_target_across("p012", "SRC", "DST").as_deref(), Some("p012"), "not a page target: unknown, kept");
    assert_eq!(nav_target_across("p5", "SRC", "SRC").as_deref(), Some("p5"));
    assert_eq!(nav_target_across("index", "SRC", "SRC").as_deref(), Some("index"), "within one notebook a link is copied exactly");
}

#[test]
fn a_broken_link_into_its_own_notebook_stays_broken() {
    let mut dst = NotebookInfo::default();
    dst.id.set("DST");
    dst.page_ids = vec![1, 2, 7];
    // a page dst still has: the plain form
    assert_eq!(nav_target_into("n:DST:p7", "SRC", &dst).as_deref(), Some("p7"));
    // a page dst deleted: the form naming dst, which a save keeps and shows as Page gone
    assert_eq!(nav_target_into("n:DST:p9", "SRC", &dst).as_deref(), Some("n:DST:p9"));
    // everything else as nav_target_across
    assert_eq!(nav_target_into("p12", "SRC", &dst).as_deref(), Some("n:SRC:p12"));
    assert_eq!(nav_target_into("index", "SRC", &dst), None);
    assert_eq!(nav_target_into("index", "DST", &dst).as_deref(), Some("index"));
    assert_eq!(nav_target_into("p9", "DST", &dst).as_deref(), Some("p9"));
}

fn write_picture(nb: &NotebookInfo, id: u32, bytes: &[u8], key: Option<u64>) {
    let mut bg = String::new();
    assert_eq!(bgimage_dir(nb.id.as_str(), &mut bg, 700), 0);
    std::fs::create_dir_all(&bg).unwrap();
    let mut f = String::new();
    bgimage_file(&bg, id, &mut f, 700);
    std::fs::write(&f, bytes).unwrap();
    if let Some(key) = key {
        let mut ix = BgIndex::default();
        assert_eq!(bgindex_load(&bg, &mut ix), 0);
        ix.e.push(BgIndexEntry { id, key });
        assert_eq!(bgindex_save(&bg, &ix), 0);
    }
}

fn picture(nb: &NotebookInfo, id: u32) -> Option<Vec<u8>> {
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    let mut f = String::new();
    bgimage_file(&bg, id, &mut f, 700);
    std::fs::read(f).ok()
}

#[test]
fn copy_to_notebook_carries_pages_names_links_and_pictures() {
    let root = fresh_root("copy");
    let src = notebook("Source", 4); // ids 1..=4
    let mut dst = notebook("Destination", 2); // ids 1..=2
    let sdir = dir_of(&src);
    let ddir = dir_of(&dst);
    let other = notebook("Other", 1);

    // Page 2 on a template picture (id 0x123, as Grimoire gives it), page 3 on a photo whose id the
    // destination's own picture holds already: that copy takes another id.
    write_picture(&src, 0x123, b"PNG-template", Some(0xaaaa_0000_0000_0123));
    write_picture(&src, 0x456, b"PNG-photo", Some(0xbbbb_0000_0000_0456));
    write_picture(&dst, 0x456, b"PNG-someone-else", Some(0xcccc_0000_0000_0456));
    for (index, paper) in [(1, paper_image(PAPER_ROLE_TEMPLATE, 0x123)), (2, paper_image(PAPER_ROLE_DOCUMENT, 0x456))] {
        let mut p = None;
        assert_eq!(store_load_page(&src, index, 600, 800, &mut p), 0);
        let mut p = p.unwrap();
        p.paper = paper;
        assert_eq!(store_save_page(&src, index, &p), 0);
    }
    let mut nav = Nav::default();
    nav_init(&mut nav);
    nav_set_name(&mut nav, 1, "Intro");
    nav_set_name(&mut nav, 3, "Photo page");
    let links: [(u32, u32, &str); 7] = [
        (1, 0x101, "p3"),             // to a page copied with it: the copy's copy
        (1, 0x102, "p4"),             // to a page left behind: back into the source
        (1, 0x103, "index"),          // nowhere to go: left out
        (3, 0x301, &format!("n:{}:p2", dst.id.as_str())), // to the destination: a page of its own now
        (3, 0x302, &format!("n:{}:p1", other.id.as_str())), // elsewhere: as it is
        (3, 0x303, "future-kind"),    // unknown: kept
        (4, 0x401, "p1"),             // a page not copied: not carried
    ];
    for (pid, seed, t) in links {
        // (nav_link_set takes only the targets this version makes; an unknown one comes back as read)
        let l = NavLink { pid, seed, target: FixedStr::from_str(t), ..Default::default() };
        assert_eq!(nav_link_put(&mut nav, &l), 0, "{t}");
    }
    assert_eq!(nav_link_set_area(&mut nav, 1, 0x101, Some(Rect::new(100, 200, 50, 40))), 0);
    assert_eq!(nav_save(src.id.as_str(), &mut nav, &src), 0);
    assert_eq!(nav_save_links(src.id.as_str(), &mut nav, &src), 0);

    let page_bytes = |dir: &str, pid: u32| std::fs::read(format!("{dir}/p{pid}.qpg")).unwrap();
    let src_bytes: Vec<Vec<u8>> = (1..=4).map(|p| page_bytes(&sdir, p)).collect();

    // Pages 3, 1, 2 (ids) in that order, before the destination's page 2.
    let mut out = Vec::new();
    assert_eq!(store_copy_pages_to(&src, &[3, 1, 2], &mut dst, 1, &mut out), 0);
    assert_eq!(out.iter().map(|c| (c.from, c.to)).collect::<Vec<_>>(), vec![(3, 3), (1, 4), (2, 5)]);
    assert_eq!(dst.page_ids, vec![1, 3, 4, 5, 2]);
    assert_eq!(ids_on_disk(&dst), dst.page_ids);
    assert!(dst.next_page_id >= 6);
    // Byte for byte, but for the photo's paper value (bytes 16..20) under its new id.
    assert_eq!(page_bytes(&ddir, 4), src_bytes[0]);
    assert_eq!(page_bytes(&ddir, 5), src_bytes[1]);
    let photo = page_bytes(&ddir, 3);
    assert_eq!(photo.len(), src_bytes[2].len());
    let diff: Vec<usize> = (0..photo.len()).filter(|&i| photo[i] != src_bytes[2][i]).collect();
    assert!(!diff.is_empty() && diff.iter().all(|&i| (16..20).contains(&i)), "{diff:?}");
    let mut p = None;
    assert_eq!(store_load_page(&dst, 1, 600, 800, &mut p), 0);
    let new_photo = p.as_ref().unwrap().paper;
    assert!(paper_is_document(new_photo) && paper_image_id(new_photo) != 0x456);
    assert_eq!(picture(&dst, paper_image_id(new_photo)).as_deref(), Some(&b"PNG-photo"[..]));
    assert_eq!(picture(&dst, 0x456).as_deref(), Some(&b"PNG-someone-else"[..]), "its own picture untouched");
    assert_eq!(picture(&dst, 0x123).as_deref(), Some(&b"PNG-template"[..]), "a free id: the same");
    let pages_ini = std::fs::read_to_string(format!("{ddir}/bg/pages.ini")).unwrap();
    assert!(pages_ini.contains(&format!("p3={}\n", new_photo.0)) && pages_ini.contains(&format!("p5={}\n", paper_image(0, 0x123).0)), "{pages_ini}");
    // The source is as it was.
    for pid in 1..=4u32 {
        assert_eq!(page_bytes(&sdir, pid), src_bytes[pid as usize - 1]);
    }

    // Names and links.
    let mut res = NavCopyResult::default();
    assert_eq!(nav_copy_pages_across(&nav, src.id.as_str(), &out, &dst, 1.0, 1.0, &mut res), 0);
    assert_eq!((res.names, res.links, res.links_dropped), (2, 5, 1));
    let mut got = Nav::default();
    nav_init(&mut got);
    assert_eq!(nav_load(dst.id.as_str(), &mut got), 0);
    assert_eq!(nav_name(&got, 4), "Intro");
    assert_eq!(nav_name(&got, 3), "Photo page");
    let target = |pid: u32, seed: u32| nav_link_get(&got, pid, seed).map(|l| l.target.as_str().to_string());
    assert_eq!(target(4, 0x101).as_deref(), Some("p3"), "page 3 was copied too: its copy, id 3 here");
    assert_eq!(nav_link_get(&got, 4, 0x101).unwrap().area, Rect::new(100, 200, 50, 40));
    assert_eq!(target(4, 0x102), Some(format!("n:{}:p4", src.id.as_str())));
    assert_eq!(target(4, 0x103), None);
    assert_eq!(target(3, 0x301).as_deref(), Some("p2"));
    assert_eq!(target(3, 0x302), Some(format!("n:{}:p1", other.id.as_str())));
    assert_eq!(target(3, 0x303).as_deref(), Some("future-kind"));
    assert_eq!(got.links.len(), 5);

    // Copying the template page again finds its picture there: one file, the same id.
    let mut again = Vec::new();
    let end = dst.npages();
    assert_eq!(store_copy_pages_to(&src, &[2], &mut dst, end, &mut again), 0);
    let mut p = None;
    assert_eq!(store_load_page(&dst, end, 600, 800, &mut p), 0);
    assert_eq!(p.unwrap().paper, paper_image(PAPER_ROLE_TEMPLATE, 0x123));

    // Refusals leave the destination as it was, no file behind.
    let ini = std::fs::read(format!("{ddir}/notebook.ini")).unwrap();
    let files = std::fs::read_dir(&ddir).unwrap().count();
    let mut out = Vec::new();
    assert_eq!(store_copy_pages_to(&src, &[9], &mut dst, 0, &mut out), -1);
    assert_eq!(store_copy_pages_to(&src, &[1, 1], &mut dst, 0, &mut out), -1);
    assert_eq!(store_copy_pages_to(&src, &[1], &mut dst, 99, &mut out), -1);
    let mut same = src.clone();
    assert_eq!(store_copy_pages_to(&src, &[1], &mut same, 0, &mut out), -1);
    // A damaged page refuses the whole copy (the page before it is not left there).
    let good = page_bytes(&sdir, 4);
    std::fs::write(format!("{sdir}/p4.qpg"), b"QPG1junk").unwrap();
    let ids = dst.page_ids.clone();
    assert_eq!(store_copy_pages_to(&src, &[1, 4], &mut dst, 0, &mut out), -1);
    assert!(out.is_empty());
    assert_eq!(dst.page_ids, ids);
    std::fs::write(format!("{sdir}/p4.qpg"), &good).unwrap();
    // notebook.ini cannot be written: the page files written are taken back.
    std::fs::set_permissions(format!("{ddir}/notebook.ini"), std::fs::Permissions::from_mode(0o444)).unwrap();
    std::fs::write(format!("{ddir}/notebook.ini.tmp"), b"").unwrap();
    std::fs::set_permissions(format!("{ddir}/notebook.ini.tmp"), std::fs::Permissions::from_mode(0o444)).unwrap();
    let blocked = store_copy_pages_to(&src, &[1, 4], &mut dst, 0, &mut out);
    std::fs::set_permissions(format!("{ddir}/notebook.ini"), std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::set_permissions(format!("{ddir}/notebook.ini.tmp"), std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::remove_file(format!("{ddir}/notebook.ini.tmp")).unwrap();
    assert_eq!(blocked, -1);
    assert!(out.is_empty());
    assert_eq!(dst.page_ids, ids);
    assert_eq!(std::fs::read(format!("{ddir}/notebook.ini")).unwrap(), ini);
    assert_eq!(std::fs::read_dir(&ddir).unwrap().count(), files);
    assert_eq!(ids_on_disk(&dst), dst.page_ids);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn copy_to_notebook_read_only_missing_and_other_sizes() {
    let root = fresh_root("copy-ro");
    let src = notebook("Source", 3);
    let sdir = dir_of(&src);
    // A notebook of another page size (another Scribe's).
    store_set_device(300.0, 900, 1200);
    let mut dst = notebook("Bigger", 1);
    store_set_device(300.0, 600, 800);
    let ddir = dir_of(&dst);
    assert_eq!((dst.page_w, dst.page_h), (900, 1200));

    // Page 1 read-only (bytes after its strokes that are no chunk), page 2 missing, page 3 ordinary.
    let mut ro = std::fs::read(format!("{sdir}/p1.qpg")).unwrap();
    ro.extend_from_slice(b"\x01\x02\x03");
    std::fs::write(format!("{sdir}/p1.qpg"), &ro).unwrap();
    let mut p = None;
    assert_eq!(store_load_page(&src, 0, 600, 800, &mut p), 0);
    assert!(p.unwrap().readonly);
    std::fs::remove_file(format!("{sdir}/p2.qpg")).unwrap();
    let plain = std::fs::read(format!("{sdir}/p3.qpg")).unwrap();

    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_link_set(&mut nav, 3, 0x7, "p1"), 0);
    assert_eq!(nav_link_set_area(&mut nav, 3, 0x7, Some(Rect::new(100, 100, 60, 30))), 0);

    let mut out = Vec::new();
    assert_eq!(store_copy_pages_to(&src, &[1, 2, 3], &mut dst, 0, &mut out), 0);
    assert_eq!(dst.page_ids, vec![2, 3, 4, 1]);
    assert_eq!(std::fs::read(format!("{ddir}/p2.qpg")).unwrap(), ro, "read-only: byte for byte");
    assert_eq!(std::fs::read(format!("{ddir}/p4.qpg")).unwrap(), plain, "its own size kept in the file");
    // The missing page is an empty page of the destination's size; the others scale as it opens them.
    let mut p = None;
    assert_eq!(store_load_page(&dst, 1, 900, 1200, &mut p), 0);
    let p = p.unwrap();
    assert!(p.strokes.is_empty() && p.w == 900 && p.h == 1200 && p.paper == PAPER_MEDIUM);
    let mut p = None;
    assert_eq!(store_load_page(&dst, 2, 900, 1200, &mut p), 0);
    let p = p.unwrap();
    assert_eq!((p.w, p.h, p.strokes.len()), (900, 1200, 3));
    assert!((p.strokes[0].pts[0].x - 75.0).abs() < 0.01, "scaled by 1.5: {}", p.strokes[0].pts[0].x);

    // The link's area scales with the page.
    let mut res = NavCopyResult::default();
    assert_eq!(nav_copy_pages_across(&nav, src.id.as_str(), &out, &dst, 1.5, 1.5, &mut res), 0);
    let mut got = Nav::default();
    nav_init(&mut got);
    assert_eq!(nav_load(dst.id.as_str(), &mut got), 0);
    let l = nav_link_get(&got, 4, 0x7).unwrap();
    assert_eq!(l.target.as_str(), "p2", "page 1 was copied along: its copy is p2 here");
    assert_eq!(l.area, Rect::new(150, 150, 90, 45));

    // A read-only page whose picture would need another id there is refused, and nothing is left.
    let mut ro2 = std::fs::read(format!("{sdir}/p3.qpg")).unwrap();
    let paper = paper_image(PAPER_ROLE_DOCUMENT, 0x777);
    let mut pg = None;
    assert_eq!(page_deserialize(&ro2, &mut pg), 0);
    let mut pg = pg.unwrap();
    pg.paper = paper;
    assert_eq!(page_serialize(&pg, &mut ro2), 0);
    ro2.extend_from_slice(b"\x09");
    std::fs::write(format!("{sdir}/p3.qpg"), &ro2).unwrap();
    write_picture(&src, 0x777, b"PNG-ours", Some(0x1111_0000_0000_0777));
    write_picture(&dst, 0x777, b"PNG-theirs", Some(0x2222_0000_0000_0777));
    let ids = dst.page_ids.clone();
    let count = std::fs::read_dir(&ddir).unwrap().count();
    let pics = std::fs::read_dir(format!("{ddir}/bg")).unwrap().count();
    assert_eq!(store_copy_pages_to(&src, &[3], &mut dst, 0, &mut out), -1);
    assert_eq!(dst.page_ids, ids);
    assert_eq!(std::fs::read_dir(&ddir).unwrap().count(), count);
    assert_eq!(std::fs::read_dir(format!("{ddir}/bg")).unwrap().count(), pics, "the picture copied for it is taken back");
    std::fs::remove_dir_all(root).unwrap();
}

/// Redo of an undone move (2.0.0-beta.2, the owner's ruling of 2026-11-03): the exact order the move left,
/// through notebook.ini alone, last_page following its page; one step (it applies only to the order the
/// undo left), refused after any other change to the pages, in another notebook, and with nb as it was when
/// notebook.ini cannot be written.
#[test]
fn an_undone_move_redoes_to_the_exact_order() {
    let root = fresh_root("redo");
    let mut nb = notebook("Redo", 6); // ids 1..=6
    let dir = dir_of(&nb);
    nb.last_page = 3; // id 4 open
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[5, 1, 3], 2, &mut undo), 0);
    let u = undo.unwrap();
    // Not undone yet: nothing to redo.
    assert_eq!(store_redo_move_pages(&mut nb, &u), -1);
    assert_eq!(nb.page_ids, vec![1, 2, 4, 6, 3, 5]);
    assert_eq!(store_undo_move_pages(&mut nb, &u), 0);
    assert_eq!(nb.last_page, 3);
    assert_eq!(store_redo_move_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 4, 6, 3, 5], "the exact order the move left");
    assert_eq!(nb.last_page, 2, "last_page follows its page");
    assert_eq!(ids_on_disk(&nb), nb.page_ids);
    assert_eq!(store_redo_move_pages(&mut nb, &u), -1, "once");
    // The same record undoes it again (the Undo gets it back), and redoes again.
    assert!(u.applies(&nb));
    assert_eq!(store_undo_move_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 3, 4, 5, 6]);
    // Another change since the undo (a page added): refused, the notebook as it is.
    assert_eq!(store_add_page(&mut nb, 6, PAPER_MEDIUM), 0);
    let changed = nb.page_ids.clone();
    assert_eq!(store_redo_move_pages(&mut nb, &u), -1);
    assert_eq!(nb.page_ids, changed);
    assert_eq!(ids_on_disk(&nb), changed);
    // Another notebook with the same order: never.
    let mut other = notebook("Other", 2);
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut other, &[0], 2, &mut undo), 0);
    let mut u2 = undo.unwrap();
    assert_eq!(store_undo_move_pages(&mut other, &u2), 0);
    u2.notebook = nb.id.as_str().to_string();
    assert_eq!(store_redo_move_pages(&mut other, &u2), -1);
    assert_eq!(other.page_ids, vec![1, 2]);
    // notebook.ini that cannot be written: refused, nb as it was, the record still good to try again.
    let mut undo = None;
    assert_eq!(store_move_pages_undo(&mut nb, &[0], 3, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(store_undo_move_pages(&mut nb, &u), 0);
    let was = nb.clone();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    assert_eq!(store_redo_move_pages(&mut nb, &u), -1);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!((nb.page_ids.clone(), nb.last_page, nb.modified), (was.page_ids.clone(), was.last_page, was.modified));
    assert_eq!(store_redo_move_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, u.after);
    std::fs::remove_dir_all(root).unwrap();
}
