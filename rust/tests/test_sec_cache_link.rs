// test_sec_cache_link.rs (security review 2026-11, finding "the PDF cache's prune follows a link"): every background
// child sweeps grimoire/.cache/pdf once a day (bgjob.rs -> pdf_cache_sweep_due -> prune_in), and every mirror prunes
// it. prune_in listed the folder by path and removed, recursively (remove_dir_all), every folder in it that is not a
// live notebook's: a .cache/pdf (or .cache) that is a link made root delete every folder of its target, anywhere on
// the device. Planting the link needs a way to make one on /mnt/us (another program on the Kindle, a shell; never MTP).
#![cfg(feature = "sim")]
use grimoire::prelude::*;

#[test]
fn the_cache_sweep_never_prunes_through_a_link() {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let top = base.join(format!("grimoire-sec-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&top);
    let root = top.join("us/grimoire");
    std::fs::create_dir_all(root.join("notebooks")).unwrap();
    std::fs::create_dir_all(root.join(".cache")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", top.join("us"));
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    // the folder the link points at: not Grimoire's, with folders and a file of its own
    let outside = top.join("outside");
    for d in ["etc", "bin", "var/lib"] {
        std::fs::create_dir_all(outside.join(d)).unwrap();
        std::fs::write(outside.join(d).join("f"), b"x").unwrap();
    }
    std::fs::write(outside.join("keep"), b"x").unwrap();
    std::os::unix::fs::symlink(&outside, root.join(".cache/pdf")).unwrap();
    grimoire::pdfcache::pdf_cache_sweep_due();
    let left: Vec<String> = std::fs::read_dir(&outside).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    eprintln!("  left in the linked folder after the sweep: {left:?}");
    for d in ["etc", "bin", "var"] {
        assert!(outside.join(d).is_dir(), "the sweep deleted {d}/ through a linked .cache/pdf");
    }
    assert!(!outside.join(".swept").exists(), "the sweep wrote its stamp through the link");
    let _ = std::fs::remove_dir_all(&top);
}
