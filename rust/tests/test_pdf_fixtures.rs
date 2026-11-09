// test_pdf_fixtures.rs: the PDFs the scenarios copy (tests/fixtures/pdf/) are what tests/pdffix makes now (as the
// .mo check does for the catalogs). `make pdf-fixtures` writes them (GRIMOIRE_WRITE_PDF_FIXTURES=<dir>).
mod pdffix;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/pdf")
}

#[test]
fn committed_pdf_fixtures_are_what_the_generator_makes() {
    if let Ok(out) = std::env::var("GRIMOIRE_WRITE_PDF_FIXTURES") {
        std::fs::create_dir_all(&out).unwrap();
        for (name, bytes) in pdffix::scenario_files() {
            std::fs::write(std::path::Path::new(&out).join(name), bytes).unwrap();
        }
        return;
    }
    for (name, bytes) in pdffix::scenario_files() {
        let on_disk = std::fs::read(dir().join(name)).unwrap_or_else(|_| panic!("tests/fixtures/pdf/{name} is missing: make pdf-fixtures"));
        assert!(on_disk == bytes, "tests/fixtures/pdf/{name} differs from what tests/pdffix makes: make pdf-fixtures");
    }
}

#[test]
fn the_generator_is_deterministic_and_plain() {
    assert_eq!(pdffix::planner(3), pdffix::planner(3));
    assert!(pdffix::planner(2).starts_with(b"%PDF-1.4"));
    assert!(pdffix::planner(2).ends_with(b"%%EOF\n"));
    assert!(!String::from_utf8_lossy(&pdffix::planner(1)).contains('\u{2014}'));
}
