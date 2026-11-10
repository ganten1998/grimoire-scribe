// test_sec_import_names.rs (security review 2026-11, finding "a file name crashes the import scan"): the scan of
// grimoire/import (pdfimport::list_pdfs, run by import_scan at every start, at the Library's scans and on a pick)
// asks is_pdf_name of every top-level file and of every entry one folder down. It sliced the name's last four BYTES
// as a str: a name whose last four bytes begin inside a character ("会议记录", "东京", a folder "照片" inside a
// picture folder, "éaé") panicked, and the Kindle build (panic = "abort") ended the UI, at every start, until the
// file was taken away over USB.
#![cfg(feature = "sim")]
use grimoire::pdfimport::*;
use grimoire::prelude::*;

fn setup(name: &str) -> String {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let storage = base.join(format!("grimoire-sec-names-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    root.to_str().unwrap().to_string()
}

#[test]
fn names_that_end_inside_a_character_do_not_crash_the_scan() {
    let root = setup("cjk");
    let imp = format!("{root}/import");
    for n in ["会议记录", "东京", "éaé", "Ok.pdf"] {
        std::fs::write(format!("{imp}/{n}"), b"%PDF-1.4\n").unwrap();
    }
    std::fs::create_dir_all(format!("{imp}/Trip/照片")).unwrap();
    std::fs::write(format!("{imp}/Trip/p1.png"), b"x").unwrap();
    std::fs::write(format!("{imp}/Trip/写真"), b"x").unwrap();
    let found = std::panic::catch_unwind(list_pdfs);
    assert!(found.is_ok(), "the scan of import/ panicked on a file's name (the Kindle build aborts the UI)");
    let found = found.unwrap();
    let names: Vec<&str> = found.files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(names, vec!["Ok.pdf"]);
}
