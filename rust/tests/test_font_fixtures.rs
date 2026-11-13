// test_font_fixtures.rs: the fonts the scenarios copy (tests/fixtures/fonts/) are what tests/fontfix makes now (as
// test_pdf_fixtures.rs checks the PDFs). `make font-fixtures` writes them (GRIMOIRE_WRITE_FONT_FIXTURES=<dir>).
mod fontfix;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/fonts")
}

#[test]
fn committed_font_fixtures_are_what_the_generator_makes() {
    if let Ok(out) = std::env::var("GRIMOIRE_WRITE_FONT_FIXTURES") {
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        for (name, bytes) in fontfix::scenario_files() {
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
    let mut want: Vec<String> = fontfix::scenario_files().iter().map(|f| f.0.to_string()).collect();
    want.sort();
    assert_eq!(have, want, "tests/fixtures/fonts holds what the generator makes, no more: make font-fixtures");
    for (name, bytes) in fontfix::scenario_files() {
        let on_disk = std::fs::read(dir().join(name)).unwrap_or_else(|_| panic!("tests/fixtures/fonts/{name} is missing: make font-fixtures"));
        assert!(on_disk == bytes, "tests/fixtures/fonts/{name} differs from what tests/fontfix makes: make font-fixtures");
    }
}

#[test]
fn the_generator_is_deterministic_and_small() {
    assert_eq!(fontfix::ttf(&fontfix::test_font("Regular", 400)), fontfix::ttf(&fontfix::test_font("Regular", 400)));
    let all: usize = fontfix::scenario_files().iter().map(|f| f.1.len()).sum();
    assert!(all < 400 << 10, "the fixtures weigh {all} bytes");
    assert!(fontfix::ttf(&fontfix::test_font("Regular", 400)).starts_with(&[0, 1, 0, 0]));
    assert!(fontfix::otf(&fontfix::test_font("Regular", 400), 0, false).starts_with(b"OTTO"));
}
