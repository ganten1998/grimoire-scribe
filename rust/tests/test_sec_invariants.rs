// test_sec_invariants.rs (hardening round 2026-11): what the documents and the build rules say of the safe loader,
// held against the code and Cargo.lock.
//
// Every test here reads files of the tree as text (include_str!): no process, no environment variable, no clock.
// A "gone" pin is a phrase a file must no longer hold: a sentence that said what the code does not do. A
// "present" pin is a name the code itself has (a constant, a file's name), never prose, so a reworded sentence
// does not fail here. A phrase is looked for in the file with every run of white space made one space, so a
// paragraph wrapped anew is the same text. Each test gathers every pin that fails and prints them together.
#![cfg(feature = "sim")]

/// A file of the tree: its path from the repository's root (for the messages) and its text.
type File = (&'static str, &'static str);

const DESIGN: File = ("docs/DESIGN.md", include_str!("../../docs/DESIGN.md"));
const PDFWORK: File = ("rust/src/pdfwork.rs", include_str!("../src/pdfwork.rs"));
const FONTCHECK: File = ("rust/src/fontcheck.rs", include_str!("../src/fontcheck.rs"));

/// The text with every run of white space (a line's end too) made one space.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A gone pin: the file no longer holds the phrase.
fn gone(fails: &mut Vec<String>, f: File, phrase: &str) {
    let n = flat(f.1).matches(phrase).count();
    if n != 0 {
        fails.push(format!("{} still says, {n} time(s): {phrase}", f.0));
    }
}

/// The end of a test: every failed pin, or none.
fn none_failed(fails: &[String], what: &str) {
    assert!(fails.is_empty(), "{what}: {} failed\n  {}\n", fails.len(), fails.join("\n  "));
}

// The limit on record lines is counted for each worker (`Session::rec_bytes`, set to 0 at every start), and a job
// is many workers. The documents and comments said "a job"; they say "a worker" now, and DESIGN.md names the
// constant beside its size, so a changed number fails here until the sentence follows it.
// (rust/tests/test_sec_records_cap.rs holds the rule itself: the cap ends a worker, and the next starts afresh.)
#[test]
fn the_record_cap_is_a_workers_and_the_docs_say_so() {
    let mut fails = Vec::new();
    gone(&mut fails, DESIGN, "4 MiB a job");
    gone(&mut fails, PDFWORK, "4 MiB a job");
    gone(&mut fails, PDFWORK, "The records of one job");
    gone(&mut fails, PDFWORK, "the job's records");
    // (the font check counts lines, never bytes: its comment claimed the PDF worker's cap)
    gone(&mut fails, FONTCHECK, "4 MiB of records");
    // DESIGN.md names the constant, and its size stands within 200 bytes of the name (bytes, not characters: the
    // file holds characters of more than one byte).
    let cap = grimoire::pdfwork::RECORDS_MAX_BYTES;
    let (name, size) = ("RECORDS_MAX_BYTES", format!("{} MiB", cap >> 20));
    let design = flat(DESIGN.1);
    match design.find(name) {
        None => fails.push(format!("{} does not name {name}", DESIGN.0)),
        Some(at) => {
            let b = design.as_bytes();
            let (from, to) = (at.saturating_sub(200), (at + name.len() + 200).min(b.len()));
            // (the size as a whole number: "4 MiB" is not found in "64 MiB")
            let near = (from..to.saturating_sub(size.len() - 1)).any(|i| b[i..].starts_with(size.as_bytes()) && (i == 0 || !b[i - 1].is_ascii_digit()));
            if cap % (1 << 20) != 0 || !near {
                fails.push(format!("{} names {name} without its size, \"{size}\", within 200 bytes of it", DESIGN.0));
            }
        }
    }
    none_failed(&fails, "the record cap in the documents");
}
