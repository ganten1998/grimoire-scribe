// New page's batch (owner, 2026-11-03): store_add_pages adds n blank pages in one store operation (one
// notebook.ini write, nothing added on a failure), store_room_for_pages is its limit, and
// store_undo_add_pages takes the batch back while its pages are still the blank pages it wrote.
use grimoire::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn fresh_root(tag: &str) -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("add-pages-{tag}-{}", std::process::id()));
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

fn notebook(name: &str, n: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_MEDIUM, &mut nb), 0);
    while nb.npages() < n as i32 {
        let at = nb.npages();
        assert_eq!(store_add_page(&mut nb, at, PAPER_MEDIUM), 0);
    }
    nb
}

fn ids_on_disk(nb: &NotebookInfo) -> Vec<u32> {
    let mut back = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut back), 0);
    back.page_ids
}

fn page_files(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| f.ends_with(".qpg")).collect();
    v.sort();
    v
}

#[test]
fn a_batch_adds_n_blank_pages_in_one_list_write() {
    let root = fresh_root("batch");
    let mut nb = notebook("Batch", 3); // ids 1..=3
    let dir = dir_of(&nb);
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, 4, PAPER_MEDIUM, &mut undo), 0);
    assert_eq!(nb.page_ids, vec![1, 4, 5, 6, 7, 2, 3], "after the first page, in order, with new ids");
    assert_eq!(nb.next_page_id, 8);
    assert_eq!(ids_on_disk(&nb), nb.page_ids);
    // Each a blank page of the notebook's size on the paper asked for, as New page writes one.
    for i in 1..=4 {
        let mut p = None;
        assert_eq!(store_load_page(&nb, i, 600, 800, &mut p), 0);
        let p = p.unwrap();
        assert!(p.strokes.is_empty());
        assert_eq!(p.paper, PAPER_MEDIUM);
    }
    let u = undo.expect("recorded");
    assert_eq!((u.before.clone(), u.added.clone()), (vec![1, 2, 3], vec![4, 5, 6, 7]));
    assert!(u.applies(&nb) && store_added_pages_untouched(&nb, &u));
    // The end, and an index past it clamped there.
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 99, 2, PAPER_MEDIUM, &mut undo), 0);
    assert_eq!(&nb.page_ids[7..], &[8, 9]);
    assert_eq!(page_files(&dir).len(), 9);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_batch_stays_within_the_notebook_limit() {
    let root = fresh_root("limit");
    let mut nb = notebook("Limit", 1);
    assert_eq!(store_room_for_pages(&nb), STORE_MAX_PAGES as i32 - 1);
    let mut undo = None;
    // None, a negative count, and one more than fits: refused, nothing written.
    for n in [0, -3, STORE_MAX_PAGES as i32] {
        assert_eq!(store_add_pages(&mut nb, 1, n, PAPER_MEDIUM, &mut undo), -1, "{n}");
        assert!(undo.is_none());
        assert_eq!(nb.npages(), 1);
    }
    // Exactly what fits: the notebook is full.
    let room = store_room_for_pages(&nb);
    assert_eq!(store_add_pages(&mut nb, 1, room, PAPER_MEDIUM, &mut undo), 0);
    assert_eq!(nb.npages(), STORE_MAX_PAGES as i32);
    assert_eq!(store_room_for_pages(&nb), 0);
    assert_eq!(ids_on_disk(&nb).len(), STORE_MAX_PAGES);
    let mut again = None;
    assert_eq!(store_add_pages(&mut nb, 1, 1, PAPER_MEDIUM, &mut again), -1);
    assert!(again.is_none());
    // and the whole batch undone at once
    assert_eq!(store_undo_add_pages(&mut nb, &undo.unwrap()), 0);
    assert_eq!(nb.page_ids, vec![1]);
    assert_eq!(page_files(&dir_of(&nb)), vec!["p1.qpg".to_string()]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_failed_batch_adds_no_page() {
    let root = fresh_root("fail");
    let mut nb = notebook("Fail", 2);
    let dir = dir_of(&nb);
    let ini = std::fs::read(format!("{dir}/notebook.ini")).unwrap();
    let files = page_files(&dir);
    let was = nb.clone();
    // notebook.ini cannot be replaced (a directory with something in it where it goes): the page files the
    // batch wrote are taken back, nb is as it was.
    std::fs::rename(format!("{dir}/notebook.ini"), format!("{dir}/ini.keep")).unwrap();
    std::fs::create_dir_all(format!("{dir}/notebook.ini/x")).unwrap();
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, 5, PAPER_MEDIUM, &mut undo), -1);
    assert!(undo.is_none());
    assert_eq!((nb.page_ids.clone(), nb.next_page_id, nb.modified), (was.page_ids.clone(), was.next_page_id, was.modified));
    assert_eq!(page_files(&dir), files, "no page file left behind");
    std::fs::remove_dir_all(format!("{dir}/notebook.ini")).unwrap();
    std::fs::rename(format!("{dir}/ini.keep"), format!("{dir}/notebook.ini")).unwrap();
    // A page file that cannot be written (the folder read-only): nothing either.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    assert_eq!(store_add_pages(&mut nb, 1, 3, PAPER_MEDIUM, &mut undo), -1);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(nb.page_ids, was.page_ids);
    assert_eq!(page_files(&dir), files);
    assert_eq!(std::fs::read(format!("{dir}/notebook.ini")).unwrap(), ini);
    std::fs::remove_dir_all(root).unwrap();
}

thread_local! {
    static SAVES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}
fn count_save() {
    SAVES.with(|c| c.set(c.get() + 1));
}

#[test]
fn a_failed_batch_on_a_picture_paper_unlists_its_pages_in_one_write() {
    let root = fresh_root("fail-image");
    let mut nb = notebook("Fail image", 2);
    let dir = dir_of(&nb);
    let mut bgdir = String::new();
    assert_eq!(bgimage_dir(nb.id.as_str(), &mut bgdir, 700), 0);
    // A picture paper (a template's: it stays on new pages), listed in bg/pages.ini for page 1 already.
    let paper = paper_image(0, 7);
    assert!(paper_is_image(paper) && !paper_is_document(paper));
    assert_eq!(store_note_papers_in(&bgdir, &[1], &[paper]), 0);
    let side = std::fs::read(format!("{bgdir}/pages.ini")).unwrap();
    // notebook.ini cannot be replaced: the batch lists its 40 pages in bg/pages.ini, then takes them off
    // again in one write, not one write a page.
    std::fs::rename(format!("{dir}/notebook.ini"), format!("{dir}/ini.keep")).unwrap();
    std::fs::create_dir_all(format!("{dir}/notebook.ini/x")).unwrap();
    let n = 40;
    let mut undo = None;
    SAVES.with(|c| c.set(0));
    work_pump_mark_set(Some(count_save)); // (each write_file_atomic, done or failed, ends with a mark)
    let rc = store_add_pages(&mut nb, 1, n, paper, &mut undo);
    work_pump_mark_set(None);
    assert_eq!(rc, -1);
    // n page files, bg/pages.ini listing them, notebook.ini (failing), bg/pages.ini unlisting them.
    assert_eq!(SAVES.with(|c| c.get()), n as u32 + 3);
    assert_eq!(std::fs::read(format!("{bgdir}/pages.ini")).unwrap(), side, "bg/pages.ini as it was");
    std::fs::remove_dir_all(format!("{dir}/notebook.ini")).unwrap();
    std::fs::rename(format!("{dir}/ini.keep"), format!("{dir}/notebook.ini")).unwrap();
    assert_eq!(page_files(&dir).len(), 2, "no page file left behind");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_batch_undoes_only_while_its_pages_are_blank_and_newest() {
    let root = fresh_root("undo");
    let mut nb = notebook("Undo", 3); // ids 1..=3
    let dir = dir_of(&nb);
    nb.last_page = 2; // id 3 open
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, 3, PAPER_MEDIUM, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(nb.page_ids, vec![1, 4, 5, 6, 2, 3]);
    nb.last_page = 5;
    assert_eq!(store_undo_add_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, vec![1, 2, 3]);
    assert_eq!(nb.last_page, 2, "last_page follows its page");
    assert_eq!(ids_on_disk(&nb), nb.page_ids);
    assert_eq!(page_files(&dir), vec!["p1.qpg", "p2.qpg", "p3.qpg"]);
    assert!(!u.applies(&nb));
    assert_eq!(store_undo_add_pages(&mut nb, &u), -1, "once");

    // A page of the batch written on: refused, nothing changed.
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 3, 2, PAPER_MEDIUM, &mut undo), 0);
    let u = undo.unwrap();
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(&nb, &mut w, &mut h);
    let mut p = page_new(w, h, PAPER_MEDIUM);
    let mut tp = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut tp);
    let mut s = stroke_new(TOOL_PENCIL, &tp, 7);
    for i in 0..10 {
        let q = PenSample { x: 50.0 + i as f32 * 10.0, y: 100.0, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i * 4 };
        stroke_append(&mut s, &q, 300.0);
    }
    page_add(&mut p, s);
    assert_eq!(store_save_page(&nb, 4, &p), 0);
    assert!(!store_added_pages_untouched(&nb, &u));
    let list = nb.page_ids.clone();
    assert_eq!(store_undo_add_pages(&mut nb, &u), -1);
    assert_eq!(nb.page_ids, list);
    assert_eq!(ids_on_disk(&nb), list);

    // Another change since (a move): no longer the newest, refused.
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, 1, PAPER_MEDIUM, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(store_move_pages(&mut nb, &[0], 3), 0);
    assert!(!u.applies(&nb));
    assert_eq!(store_undo_add_pages(&mut nb, &u), -1);
    // The open page one of the batch: last_page goes to the page before it.
    let mut undo = None;
    let n = nb.npages();
    assert_eq!(store_add_pages(&mut nb, n, 2, PAPER_MEDIUM, &mut undo), 0);
    nb.last_page = n + 1;
    assert_eq!(store_undo_add_pages(&mut nb, &undo.unwrap()), 0);
    assert_eq!(nb.last_page, n - 1);
    std::fs::remove_dir_all(root).unwrap();
}

/// Redo of an undone batch (2.0.0-beta.2, the owner's ruling of 2026-11-03): the same pages again, with the
/// same ids at the same places, each file the blank page the batch wrote, notebook.ini once, last_page
/// following its page; one step, refused after any other change to the pages, with a file in the way that
/// is not that blank page, or when a file cannot be written (nothing left behind, nb as it was).
#[test]
fn an_undone_batch_redoes_the_same_pages() {
    let root = fresh_root("redo");
    let mut nb = notebook("Redo", 3); // ids 1..=3
    let dir = dir_of(&nb);
    nb.last_page = 2; // id 3 open
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, 3, PAPER_WIDE, &mut undo), 0);
    let u = undo.unwrap();
    assert_eq!(u.paper, PAPER_WIDE);
    assert_eq!(store_redo_add_pages(&mut nb, &u), -1, "not undone yet");
    nb.last_page = 5; // (the editor's: id 3 still open)
    assert_eq!(store_undo_add_pages(&mut nb, &u), 0);
    assert_eq!(nb.last_page, 2);
    assert_eq!(page_files(&dir), vec!["p1.qpg", "p2.qpg", "p3.qpg"]);
    assert_eq!(store_redo_add_pages(&mut nb, &u), 0);
    assert_eq!(nb.page_ids, vec![1, 4, 5, 6, 2, 3], "the same ids at the same places");
    assert_eq!(nb.last_page, 5, "last_page follows its page");
    assert!(nb.next_page_id >= 7);
    assert_eq!(ids_on_disk(&nb), nb.page_ids);
    for pid in [4, 5, 6] {
        assert_eq!(std::fs::read(format!("{dir}/p{pid}.qpg")).unwrap(), u.blank, "p{pid} the blank page it was");
    }
    for i in 1..=3 {
        assert_eq!(store_page_paper(&nb, i), PAPER_WIDE);
    }
    assert_eq!(store_redo_add_pages(&mut nb, &u), -1, "once");
    // The Undo takes them back again (still blank).
    assert!(u.applies(&nb) && store_added_pages_untouched(&nb, &u));
    assert_eq!(store_undo_add_pages(&mut nb, &u), 0);

    // A file in the way that is not that blank page: refused, nothing written, nb as it was.
    std::fs::write(format!("{dir}/p5.qpg"), b"not a blank page").unwrap();
    let was = nb.clone();
    assert_eq!(store_redo_add_pages(&mut nb, &u), -1);
    assert_eq!((nb.page_ids.clone(), nb.next_page_id), (was.page_ids.clone(), was.next_page_id));
    assert_eq!(page_files(&dir), vec!["p1.qpg", "p2.qpg", "p3.qpg", "p5.qpg"], "p4 written for it taken back");
    std::fs::remove_file(format!("{dir}/p5.qpg")).unwrap();
    // A file that cannot be written (the folder read-only): refused, nothing left behind.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    assert_eq!(store_redo_add_pages(&mut nb, &u), -1);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(nb.page_ids, was.page_ids);
    assert_eq!(page_files(&dir), vec!["p1.qpg", "p2.qpg", "p3.qpg"]);
    // Another change to the pages since the undo (a page added): refused, that page's file untouched.
    assert_eq!(store_add_page(&mut nb, 1, PAPER_MEDIUM), 0);
    let changed = nb.page_ids.clone();
    assert_eq!(store_redo_add_pages(&mut nb, &u), -1);
    assert_eq!(nb.page_ids, changed);
    assert_eq!(ids_on_disk(&nb), changed);
    std::fs::remove_dir_all(root).unwrap();
}

/// A batch on a picture paper, undone and redone: bg/pages.ini lists its pages again, as the batch did.
#[test]
fn a_redone_batch_on_a_picture_paper_lists_its_pages_again() {
    let root = fresh_root("redo-image");
    let mut nb = notebook("Redo image", 2);
    let mut bgdir = String::new();
    assert_eq!(bgimage_dir(nb.id.as_str(), &mut bgdir, 700), 0);
    let paper = paper_image(0, 7);
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, 3, paper, &mut undo), 0);
    let listed = std::fs::read(format!("{bgdir}/pages.ini")).unwrap();
    let u = undo.unwrap();
    assert_eq!(store_undo_add_pages(&mut nb, &u), 0);
    assert_ne!(std::fs::read(format!("{bgdir}/pages.ini")).unwrap_or_default(), listed);
    assert_eq!(store_redo_add_pages(&mut nb, &u), 0);
    assert_eq!(std::fs::read(format!("{bgdir}/pages.ini")).unwrap(), listed, "listed again as the batch listed them");
    assert_eq!(store_page_paper(&nb, 2), paper);
    std::fs::remove_dir_all(root).unwrap();
}

/// The paper a batch takes from the page it goes after (the owner, 2026-11-03): read from the page file's start,
/// an older version's Blank given its picture back from bg/pages.ini, the notebook's for a missing, damaged or
/// newer page, or an index out of range.
#[test]
fn a_pages_paper_read_without_its_strokes() {
    let root = fresh_root("paper");
    let nb = notebook("Paper", 4);
    let dir = dir_of(&nb);
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(&nb, &mut w, &mut h);
    let mut p = page_new(w, h, PAPER_DOTS);
    let mut tp = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut tp);
    let mut s = stroke_new(TOOL_PENCIL, &tp, 7);
    for i in 0..10 {
        let q = PenSample { x: 50.0 + i as f32 * 10.0, y: 100.0, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i * 4 };
        stroke_append(&mut s, &q, 300.0);
    }
    page_add(&mut p, s);
    assert_eq!(store_save_page(&nb, 1, &p), 0);
    assert_eq!(store_page_paper(&nb, 0), PAPER_MEDIUM);
    assert_eq!(store_page_paper(&nb, 1), PAPER_DOTS);
    // An image paper (a template's), written as a version 2 page.
    let picture = paper_image(0, 7);
    assert_eq!(store_save_page(&nb, 2, &page_new(w, h, picture)), 0);
    assert_eq!(store_page_paper(&nb, 2), picture);
    // An older version's page saved as Blank that bg/pages.ini lists with a picture: the picture.
    let pid = nb.page_ids[3];
    std::fs::write(format!("{dir}/p{pid}.qpg"), [&b"QPG1"[..], &1u16.to_le_bytes(), &4u16.to_le_bytes(), &(w as u32).to_le_bytes(), &(h as u32).to_le_bytes(), &0u32.to_le_bytes()].concat()).unwrap();
    let mut bgdir = String::new();
    assert_eq!(bgimage_dir(nb.id.as_str(), &mut bgdir, 700), 0);
    assert_eq!(store_note_papers_in(&bgdir, &[pid], &[picture]), 0);
    assert_eq!(store_page_paper(&nb, 3), picture);
    // Missing, damaged, newer, or out of range: the notebook's paper.
    std::fs::remove_file(format!("{dir}/p{pid}.qpg")).unwrap();
    assert_eq!(store_page_paper(&nb, 3), nb.paper);
    std::fs::write(format!("{dir}/p{pid}.qpg"), b"QPG").unwrap();
    assert_eq!(store_page_paper(&nb, 3), nb.paper);
    std::fs::write(format!("{dir}/p{pid}.qpg"), [&b"QPG1"[..], &9u16.to_le_bytes(), &2u16.to_le_bytes(), &(w as u32).to_le_bytes(), &(h as u32).to_le_bytes()].concat()).unwrap();
    assert_eq!(store_page_paper(&nb, 3), nb.paper);
    assert_eq!(store_page_paper(&nb, -1), nb.paper);
    assert_eq!(store_page_paper(&nb, 99), nb.paper);
    std::fs::remove_dir_all(root).unwrap();
}
