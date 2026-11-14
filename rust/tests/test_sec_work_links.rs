// test_sec_work_links.rs (security review 2026-11, finding "the leftover cleanup follows links"): at every start
// (and at the Library's scans) import_scan removes what a cut-off import left in grimoire/notebooks/.import-<id>/
// (bgwork_template::import_leftovers -> remove_work). remove_work read the folder and its bg/ by path, following
// links, and unlinked every entry it found, as root: a `.import-x` that is a link to a folder, or a real work folder
// whose bg/ is one, made the UI delete every file in that folder, anywhere on the device. Planting the link needs a
// way to make one on /mnt/us (another program on the Kindle, a shell; never MTP), which is the precondition.
#![cfg(feature = "sim")]
use grimoire::prelude::*;

fn setup(name: &str) -> (String, std::path::PathBuf) {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let top = base.join(format!("grimoire-sec-links-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&top);
    let storage = top.join("us");
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::fs::create_dir_all(root.join("notebooks")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    (root.to_str().unwrap().to_string(), top)
}

fn victim(top: &std::path::Path, name: &str) -> std::path::PathBuf {
    let v = top.join(name);
    std::fs::create_dir_all(&v).unwrap();
    for f in ["passwd", "settings.conf", "keep.me"] {
        std::fs::write(v.join(f), b"not Grimoire's").unwrap();
    }
    v
}

fn left(v: &std::path::Path) -> usize {
    std::fs::read_dir(v).map(|r| r.count()).unwrap_or(0)
}

fn the_cleanup_of_a_cut_off_import_never_follows_a_link() {
    let (root, top) = setup("cleanup");
    // a work folder that is itself a link, and a real one whose bg/ is a link
    let v1 = victim(&top, "outside1");
    let v2 = victim(&top, "outside2");
    std::os::unix::fs::symlink(&v1, format!("{root}/notebooks/.import-aaaaaaaaaaaaaaaa")).unwrap();
    std::fs::create_dir_all(format!("{root}/notebooks/.import-bbbbbbbbbbbbbbbb")).unwrap();
    std::os::unix::fs::symlink(&v2, format!("{root}/notebooks/.import-bbbbbbbbbbbbbbbb/bg")).unwrap();
    import_scan(300.0);
    let (a, b) = (left(&v1), left(&v2));
    eprintln!("  files left outside the storage: {a} of 3 behind the linked work folder, {b} of 3 behind the linked bg/");
    assert_eq!(a, 3, "the scan deleted files through a linked .import- folder");
    assert_eq!(b, 3, "the scan deleted files through a linked bg/");
    let _ = std::fs::remove_dir_all(&top);
}

// (the same, for KOReader's request folder: annotate_take, at every start, empties grimoire/import/.annotate by its
// path: a .annotate that is a link to a folder made the UI delete every file of that folder)
fn the_start_never_empties_a_folder_through_a_linked_request_folder() {
    let (root, top) = setup("annotate");
    let v = victim(&top, "outside3");
    std::os::unix::fs::symlink(&v, format!("{root}/import/.annotate")).unwrap();
    let mut r = AnnotateResult::default();
    annotate_take(false, &mut r);
    let n = left(&v);
    eprintln!("  files left behind the linked import/.annotate: {n} of 3");
    assert_eq!(n, 3, "the start deleted files through a linked import/.annotate");
    let _ = std::fs::remove_dir_all(&top);
}

// (and the caches the background jobs prune: templates/.cache, whose stale previews the previews job deletes, and
// fonts/.cache, whose entries for files that are gone the font check deletes)
fn the_caches_are_never_pruned_through_a_link() {
    let (root, top) = setup("caches");
    let (v1, v2) = (victim(&top, "outside4"), victim(&top, "outside5"));
    std::fs::create_dir_all(format!("{root}/templates")).unwrap();
    std::os::unix::fs::symlink(&v1, format!("{root}/templates/.cache")).unwrap();
    std::fs::create_dir_all(format!("{root}/fonts")).unwrap();
    std::os::unix::fs::symlink(&v2, format!("{root}/fonts/.cache")).unwrap();
    grimoire::bgwork_template::bg_template("", &format!("{root}/templates"), 300.0);
    grimoire::userfonts::bg_font_check("", 300.0);
    let (a, b) = (left(&v1), left(&v2));
    eprintln!("  files left behind the linked caches: {a} of 3 (templates/.cache), {b} of 3 (fonts/.cache)");
    assert_eq!(a, 3, "the previews job deleted files through a linked templates/.cache");
    assert_eq!(b, 3, "the font check deleted files through a linked fonts/.cache");
    let _ = std::fs::remove_dir_all(&top);
}

/// The files of a folder, by name, with their bytes.
fn files_of(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> =
        std::fs::read_dir(dir).unwrap().flatten().map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap())).collect();
    v.sort();
    v
}

// (hardening round 2026-11: in import/ only a real folder is a folder of pictures. A link there is left as it
// is: the scan takes nothing from it and writes no mark into it.)
fn a_scan_never_takes_a_linked_folder() {
    let (root, top) = setup("scan-linked");
    let outside = top.join("outside-pictures");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("a.png"), b"no picture").unwrap();
    std::os::unix::fs::symlink(&outside, format!("{root}/import/Linked")).unwrap();
    let before = files_of(&outside);
    assert_eq!(import_scan(300.0), 0, "nothing is queued for a linked folder");
    assert_eq!(files_of(&outside), before, "the folder behind the link is as it was (no mark written into it)");
    assert!(std::fs::symlink_metadata(format!("{root}/import/Linked")).unwrap().file_type().is_symlink());
    let _ = std::fs::remove_dir_all(&top);
}

// (and the import job itself, handed such a path: it reads nothing there and makes nothing)
fn a_folder_job_never_reads_a_linked_folder() {
    let (root, top) = setup("job-linked");
    let outside = top.join("outside-pictures");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("a.png"), b"no picture").unwrap();
    let mut id = String::new();
    store_make_id(&mut id);
    std::fs::write(outside.join(".grimoire-import"), format!("{id}\n")).unwrap();
    let link = format!("{root}/import/Linked");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let before = files_of(&outside);
    assert_eq!(bg_import_folder(&link, 300.0), 0, "the job ends with nothing to do");
    assert_eq!(files_of(&outside), before, "the folder behind the link is as it was");
    assert_eq!(left(std::path::Path::new(&format!("{root}/notebooks"))), 0, "no notebook is made");
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink(), "the link is as it was");
    let _ = std::fs::remove_dir_all(&top);
}

// (one test, its checks in order: the store's root is the process's)
#[test]
fn test_sec_work_links() {
    the_cleanup_of_a_cut_off_import_never_follows_a_link();
    the_start_never_empties_a_folder_through_a_linked_request_folder();
    the_caches_are_never_pruned_through_a_link();
    a_scan_never_takes_a_linked_folder();
    a_folder_job_never_reads_a_linked_folder();
}
