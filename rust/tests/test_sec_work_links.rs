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

#[test]
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
