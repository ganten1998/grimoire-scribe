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

// (finding "a folder named Done loops"): under_import, which decides whether a PDF may be moved, takes done/ in any
// case, as the storage compares names; the scan skipped only "done". A PDF in import/Done/ (or in done/ itself on a
// storage that keeps the folder as "Done") was imported, never moved and never counted, and the scan that follows
// a folder holding PDFs took it again at once: "already imported" for ever while the Library is up.
fn done_in_any_case_is_not_scanned() {
    let root = setup("done");
    let imp = format!("{root}/import");
    std::fs::create_dir_all(format!("{imp}/Done")).unwrap();
    std::fs::write(format!("{imp}/Done/Old.pdf"), b"%PDF-1.4\n").unwrap();
    std::fs::write(format!("{imp}/New.pdf"), b"%PDF-1.4\n").unwrap();
    let found = list_pdfs();
    let names: Vec<&str> = found.files.iter().map(|f| f.0.as_str()).collect();
    eprintln!("  listed: {names:?}");
    assert_eq!(names, vec!["New.pdf"], "the scan lists a PDF of import/Done, which no import may ever move");
}

// (one test, its checks in order: the store's root is the process's)
#[test]
fn test_sec_import_names() {
    names_that_end_inside_a_character_do_not_crash_the_scan();
    done_in_any_case_is_not_scanned();
}
