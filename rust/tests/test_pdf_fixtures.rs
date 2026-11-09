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
        let _ = std::fs::remove_dir_all(&out);
        for (name, bytes) in pdffix::scenario_files() {
            let p = std::path::Path::new(&out).join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        }
        return;
    }
    // (and nothing else lies there: a stale fixture would be copied into a scenario)
    let mut have = Vec::new();
    fn walk(d: &std::path::Path, base: &std::path::Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            if e.path().is_dir() {
                walk(&e.path(), base, out);
            } else {
                out.push(e.path().strip_prefix(base).unwrap().to_string_lossy().into_owned());
            }
        }
    }
    walk(&dir(), &dir(), &mut have);
    have.sort();
    let mut want: Vec<String> = pdffix::scenario_files().iter().map(|f| f.0.to_string()).collect();
    want.sort();
    assert_eq!(have, want, "tests/fixtures/pdf holds what the generator makes, no more: make pdf-fixtures");
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
