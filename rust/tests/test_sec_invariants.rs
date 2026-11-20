// test_sec_invariants.rs (hardening round 2026-11): what the documents and the build rules say of the safe loader,
// held against the code and Cargo.lock.
//
// Every test here reads files of the tree as text (include_str!): no process, no environment variable, no clock.
// A "gone" pin is a phrase a file must no longer hold: a sentence that said what the code does not do. It catches
// the old words coming back (a merge settled the wrong way), not the old meaning in new words. A "present" pin is
// a name the code itself has (a constant, a file's name), never prose, so a reworded sentence does not fail here.
// A "told on" pin is a sentence that states a safety rule, held with the code that makes it true (a line, or a
// whole function with its comments aside): it fails when the code changes and when the sentence is reworded or
// goes, so that the two are read again together.
// A phrase is looked for in the file with every run of white space made one space, and in a source file without
// the marks that open its comment lines, so a paragraph or a comment wrapped anew is the same text. Each test
// gathers every pin that fails and prints them together.
//
// A docs-only pull request runs no test: docs/TESTING.md, "Tests that read the documents".
#![cfg(feature = "sim")]

/// A file of the tree: its path from the repository's root (for the messages) and its text.
type File = (&'static str, &'static str);

const DESIGN: File = ("docs/DESIGN.md", include_str!("../../docs/DESIGN.md"));
const FORMAT: File = ("docs/FORMAT.md", include_str!("../../docs/FORMAT.md"));
const KINDLE: File = ("docs/KINDLE.md", include_str!("../../docs/KINDLE.md"));
const TESTING: File = ("docs/TESTING.md", include_str!("../../docs/TESTING.md"));
const BACKLOG: File = ("docs/BACKLOG.md", include_str!("../../docs/BACKLOG.md"));
const RELEASING: File = ("docs/RELEASING.md", include_str!("../../docs/RELEASING.md"));
const PDFWORK: File = ("rust/src/pdfwork.rs", include_str!("../src/pdfwork.rs"));
const FONTCHECK: File = ("rust/src/fontcheck.rs", include_str!("../src/fontcheck.rs"));
const USERFONTS: File = ("rust/src/userfonts.rs", include_str!("../src/userfonts.rs"));
const FSVIEW: File = ("rust/src/fsview.rs", include_str!("../src/fsview.rs"));
const BROWSER: File = ("rust/src/browser.rs", include_str!("../src/browser.rs"));
const SANDBOX: File = ("rust/src/sandbox.rs", include_str!("../src/sandbox.rs"));
const TEMPLATES: File = ("rust/src/bgwork_template.rs", include_str!("../src/bgwork_template.rs"));
const BGWORK_COPY: File = ("rust/src/bgwork_copy.rs", include_str!("../src/bgwork_copy.rs"));
const APP: File = ("rust/src/app.rs", include_str!("../src/app.rs"));
const PREVIEWS: File = ("rust/src/bgwork_previews.rs", include_str!("../src/bgwork_previews.rs"));
const ADDFONT_TESTS: File = ("rust/tests/test_addfont.rs", include_str!("test_addfont.rs"));
const SANDBOX_TESTS: File = ("rust/tests/test_sandbox_linux.rs", include_str!("test_sandbox_linux.rs"));
const LICENSES_TOOL: File = ("tools/crate_licenses.py", include_str!("../../tools/crate_licenses.py"));
const CARGO_TOML: File = ("rust/Cargo.toml", include_str!("../Cargo.toml"));
const CARGO_LOCK: File = ("rust/Cargo.lock", include_str!("../Cargo.lock"));
const DENY_TOML: File = ("rust/deny.toml", include_str!("../deny.toml"));
const HOME: File = ("rust/src/home.rs", include_str!("../src/home.rs"));
const LIBRARY: File = ("rust/src/library.rs", include_str!("../src/library.rs"));
const PDFIMPORT: File = ("rust/src/pdfimport.rs", include_str!("../src/pdfimport.rs"));
const ANNOTATE: File = ("rust/src/annotate.rs", include_str!("../src/annotate.rs"));
const STORE: File = ("rust/src/store.rs", include_str!("../src/store.rs"));
const TPLPDF: File = ("rust/src/bgwork_tplpdf.rs", include_str!("../src/bgwork_tplpdf.rs"));
const BGJOB: File = ("rust/src/bgjob.rs", include_str!("../src/bgjob.rs"));
const TPLSHEET: File = ("rust/src/tplsheet.rs", include_str!("../src/tplsheet.rs"));
const WEBLINKS: File = ("rust/src/weblinks.rs", include_str!("../src/weblinks.rs"));
const DOCLINKS: File = ("rust/src/doclinks.rs", include_str!("../src/doclinks.rs"));
const LINKCARD: File = ("rust/src/linkcard.rs", include_str!("../src/linkcard.rs"));
const EDITOR: File = ("rust/src/editor.rs", include_str!("../src/editor.rs"));
const HTTPD: File = ("rust/src/httpd.rs", include_str!("../src/httpd.rs"));
const UPLOAD: File = ("rust/src/upload.rs", include_str!("../src/upload.rs"));
const SHARE: File = ("rust/src/share.rs", include_str!("../src/share.rs"));
const PAIR: File = ("rust/src/pair.rs", include_str!("../src/pair.rs"));
const UPLOAD_TESTS: File = ("rust/tests/test_share_upload.rs", include_str!("test_share_upload.rs"));

/// A file's text with every run of white space (a line's end too) made one space. In a file that is no document
/// the mark that opens a comment's line is left out first (`//`, `///` and `//!` in Rust, `#` elsewhere), so a
/// comment wrapped anew is the same text, as a document's paragraph is.
fn flat(f: File) -> String {
    let marks: &[&str] = if f.0.ends_with(".md") {
        &[]
    } else if f.0.ends_with(".rs") {
        &["//!", "///", "//"]
    } else {
        &["#"]
    };
    let mut words = Vec::new();
    for line in f.1.lines() {
        let line = line.trim_start();
        let text = marks.iter().find_map(|m| line.strip_prefix(m)).unwrap_or(line);
        words.extend(text.split_whitespace());
    }
    words.join(" ")
}

/// A gone pin: the file no longer holds the phrase.
fn gone(fails: &mut Vec<String>, f: File, phrase: &str) {
    let n = flat(f).matches(phrase).count();
    if n != 0 {
        fails.push(format!("{} still says, {n} time(s): {phrase}", f.0));
    }
}

/// A present pin: the file names what the code has.
fn present(fails: &mut Vec<String>, f: File, name: &str) {
    if !flat(f).contains(name) {
        fails.push(format!("{} does not name {name}", f.0));
    }
}

/// A sentence that stands on another: while `f` says `words`, `on` must still say `base`. It fails the day the
/// base is reworded or goes (what it said has changed), so that the sentence standing on it is read again and
/// follows. A sentence that was itself reworded is no longer held here.
fn stands_on(fails: &mut Vec<String>, f: File, words: &str, on: File, base: &str) {
    if flat(f).contains(words) && !flat(on).contains(base) {
        fails.push(format!("{} says \"{words}\", and {} no longer says \"{base}\": the first is to follow", f.0, on.0));
    }
}

/// A sentence that stands on a line of code and is itself held: `f` must still say `words` (a sentence reworded or
/// gone fails here, so that its pin is reworded with it), and `on` must still say `base`.
fn told_on(fails: &mut Vec<String>, f: File, words: &str, on: File, base: &str) {
    if !flat(f).contains(words) {
        fails.push(format!("{} no longer says \"{words}\": its pin is to follow it", f.0));
    }
    stands_on(fails, f, words, on, base);
}

/// The code of the function whose first line is the first to hold `sig` in a source file: that line to the brace
/// that closes the function, its comment lines left out and every run of white space made one space. None when no
/// line holds `sig`.
fn code_of(f: File, sig: &str) -> Option<String> {
    let (mut depth, mut opened, mut words) = (0i32, false, Vec::new());
    for line in f.1.lines().skip_while(|l| !l.contains(sig)) {
        let text = line.trim_start();
        if text.starts_with("//") {
            continue;
        }
        words.extend(text.split_whitespace());
        for ch in text.chars() {
            match ch {
                '{' => (depth, opened) = (depth + 1, true),
                '}' => depth -= 1,
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return Some(words.join(" "));
        }
    }
    None
}

/// A sentence that stands on a whole function and is itself held: `f` must still say `words`, and the function
/// `sig` opens in `on` must still be `code` to the letter (its comments aside). Any change to that function's code
/// fails here, a line added to it too, so that the sentence is read again against it and its pin follows.
fn told_on_code(fails: &mut Vec<String>, f: File, words: &str, on: File, sig: &str, code: &str) {
    if !flat(f).contains(words) {
        fails.push(format!("{} no longer says \"{words}\": its pin is to follow it", f.0));
    }
    match code_of(on, sig) {
        None => fails.push(format!("{} has no line with \"{sig}\"", on.0)),
        Some(now) if now != code => fails.push(format!(
            "{} says \"{words}\", and the code of {}'s \"{sig}\" is no longer what it stood on: the first is to follow\n    now: {now}",
            f.0, on.0
        )),
        Some(_) => {}
    }
}

/// A name the code has with its figure beside it: the file names `name`, and `figure` stands within 200 bytes
/// of the first place it does (bytes, not characters: the files hold characters of more than one byte), as a
/// whole number ("20 ms" is not found in "120 ms"). A changed number fails here until the sentence follows.
fn beside(fails: &mut Vec<String>, f: File, name: &str, figure: &str) {
    let text = flat(f);
    let Some(at) = text.find(name) else {
        fails.push(format!("{} does not name {name}", f.0));
        return;
    };
    let b = text.as_bytes();
    let (from, to) = (at.saturating_sub(200), (at + name.len() + 200).min(b.len()));
    if !(from..to.saturating_sub(figure.len() - 1)).any(|i| b[i..].starts_with(figure.as_bytes()) && (i == 0 || !b[i - 1].is_ascii_digit())) {
        fails.push(format!("{} names {name} without its figure, \"{figure}\", within 200 bytes of it", f.0));
    }
}

/// The end of a test: every failed pin, or none.
fn none_failed(fails: &[String], what: &str) {
    assert!(fails.is_empty(), "{what}: {} failed\n  {}\n", fails.len(), fails.join("\n  "));
}

// The limit on record lines is counted for each worker (`Session::rec_bytes`, set to 0 at every start, after the
// handshake's two lines), and a job is many workers. The documents and comments said "a job". What this test
// holds of that: those old wordings are gone, and DESIGN.md names the constant beside its size, so a changed
// number fails here until the sentence follows it. It does not hold the new wording: a sentence that said "a job"
// again in other words would pass. The rule itself is held against the code by
// rust/tests/test_sec_records_cap.rs: the cap ends a worker, and the next starts afresh.
#[test]
fn the_docs_name_the_record_cap_with_its_size_and_not_as_a_jobs() {
    let mut fails = Vec::new();
    gone(&mut fails, DESIGN, "4 MiB a job");
    gone(&mut fails, PDFWORK, "4 MiB a job");
    gone(&mut fails, PDFWORK, "The records of one job");
    gone(&mut fails, PDFWORK, "the job's records");
    // (the list under that heading begins with the handshake's two lines, which are read before the count starts)
    gone(&mut fails, PDFWORK, "4 MiB a worker besides the pixels)");
    // (the font check counts lines, never bytes: its comment claimed the PDF worker's cap)
    gone(&mut fails, FONTCHECK, "4 MiB of records");
    // DESIGN.md names the constant, and its size stands within 200 bytes of the name (bytes, not characters: the
    // file holds characters of more than one byte). One mention with its size is enough: a second mention
    // elsewhere in the document, far from any size, does not fail this.
    let cap = grimoire::pdfwork::RECORDS_MAX_BYTES;
    let (name, size) = ("RECORDS_MAX_BYTES", format!("{} MiB", cap >> 20));
    let design = flat(DESIGN);
    let b = design.as_bytes();
    let mut mentions = design.match_indices(name).map(|(at, _)| at).peekable();
    if mentions.peek().is_none() {
        fails.push(format!("{} does not name {name}", DESIGN.0));
    } else {
        let near = mentions.any(|at| {
            let (from, to) = (at.saturating_sub(200), (at + name.len() + 200).min(b.len()));
            // (the size as a whole number: "4 MiB" is not found in "64 MiB")
            (from..to.saturating_sub(size.len() - 1)).any(|i| b[i..].starts_with(size.as_bytes()) && (i == 0 || !b[i - 1].is_ascii_digit()))
        });
        if cap % (1 << 20) != 0 || !near {
            fails.push(format!("{} names {name} without its size, \"{size}\", within 200 bytes of it", DESIGN.0));
        }
    }
    none_failed(&fails, "the record cap in the documents");
}

// Your own fonts after the font check's hardening (#97): where a check's result (its verdict) is kept, what
// ties it to its file, the `.slow` mark, and the limits. The sentences that described these before that change
// are gone from the files that held them (a merge settled by the older side would bring them back), and the
// documents name what the code has: the companion a refusal leaves, the verdict's number, the glyph limit with
// its figure beside it. The own folder's place is a placeholder until a Scribe's figures are in, and the floors
// of its free room are set from the first of them: while the documents give them, userfonts.rs must still hold
// them, so the day one is changed the sentence is read again and follows.
#[test]
fn the_docs_follow_the_font_checks_verdicts_and_limits() {
    let mut fails = Vec::new();
    // where a verdict is kept: Grimoire's own folder where the build has one, else the cache (`VerdictDir`)
    gone(&mut fails, USERFONTS, "grimoire/fonts/.cache/<key>.font (the verdict");
    gone(&mut fails, USERFONTS, "and its verdict and preview written to the cache");
    gone(&mut fails, FORMAT, "`fonts/.cache/` holds what the background check found");
    stands_on(&mut fails, FORMAT, "in `/var/local/grimoire/<16 hex digits>/fonts/`", USERFONTS, "const STATE_BASE: &str = \"/var/local/grimoire\";");
    stands_on(&mut fails, KINDLE, "`/var/local/grimoire/<16 hex digits>/fonts/<key>.font`", USERFONTS, "const STATE_BASE: &str = \"/var/local/grimoire\";");
    // a verdict counts only with its companion in the cache: a pass with its preview, a refusal with `<key>.no`
    present(&mut fails, FORMAT, "`<key>.no`");
    // what ties a verdict to its file: the file's SHA-256, under the number the code writes in the first line
    gone(&mut fails, FORMAT, "hash=<FNV-1a 64 of the whole file");
    present(&mut fails, FORMAT, &format!("grimoire-font {} hash=<SHA-256 of the whole file", grimoire::fontcheck::VERDICT_NUMBER));
    // the `.slow` mark goes when a verdict is written (bg_font_check)
    gone(&mut fails, FORMAT, "With no verdict beside it the file is checked once more");
    gone(&mut fails, FORMAT, "Otherwise the mark stays, beside the verdict");
    gone(&mut fails, BACKLOG, "mark outlives its second try");
    // the mark is one line under the verdict's number, with how the earlier checks ended and of which bytes
    // (`SlowMark` in userfonts.rs): it is no longer an empty file, and FORMAT.md gives the line with the number
    // the code writes
    gone(&mut fails, FORMAT, "`<key>.slow`, empty");
    present(&mut fails, FORMAT, &format!("grimoire-font {} slow <time | glyphs | time,glyphs> <SHA-256 of the whole font file", grimoire::fontcheck::VERDICT_NUMBER));
    // the second try is told once, for a check that ran out of time and for one that found glyphs too slow,
    // and the two kinds of result do not make a verdict together
    gone(&mut fails, DESIGN, "(`<key>.slow`), and a second is \"Took too long to read.\"");
    gone(&mut fails, DESIGN, "and a second such check is \"Took too long to read.\"");
    gone(&mut fails, FORMAT, "when that check too runs out of time or finds such glyphs");
    // the limits: the glyph limit with its figure, wherever a document gives it; the floors while they stand
    let ms = format!("{} ms", grimoire::sandbox::s1::FONT_GLYPH_CPU_MS);
    for f in [DESIGN, TESTING, BACKLOG] {
        beside(&mut fails, f, "`s1::FONT_GLYPH_CPU_MS`", &ms);
    }
    stands_on(&mut fails, TESTING, "`STATE_FREE_FLOOR`, 4 MB", USERFONTS, "const STATE_FREE_FLOOR: u64 = 4 << 20;");
    stands_on(&mut fails, TESTING, "`STATE_FILES_FLOOR`, 512 files", USERFONTS, "const STATE_FILES_FLOOR: u64 = 512;");
    stands_on(&mut fails, TESTING, "at least 512 inodes free", USERFONTS, "const STATE_FILES_FLOOR: u64 = 512;");
    // the other two placeholders the device checks name with their values: the own folder's place, and the
    // size a draw is timed at (the largest the UI draws)
    stands_on(&mut fails, TESTING, "`STATE_BASE` in `userfonts.rs`, `/var/local/grimoire`", USERFONTS, "const STATE_BASE: &str = \"/var/local/grimoire\";");
    for (f, words) in [(TESTING, "`FONT_TIME_PX` in `fontcheck.rs`, 160 px"), (BACKLOG, "(`FONT_TIME_PX`, 160 px)")] {
        stands_on(&mut fails, f, words, FONTCHECK, "pub const FONT_TIME_PX: f32 = FONT_PX_MAX;");
        stands_on(&mut fails, f, words, FONTCHECK, "pub const FONT_PX_MAX: f32 = 160.0;");
    }
    none_failed(&fails, "your own fonts: verdicts and limits");
}

// Grimoire's own folder on the Kindle (issue #201): what stands at its name and is not Grimoire's own is moved aside
// and the folder made afresh. Each rule sentence of the documents is held with the line of userfonts.rs that makes
// it true: the job alone moves, at the first level only, at most once a run (the UI gives the first check of a run
// alone leave to make the folder, or make it afresh, and a later check makes nothing), to a name of Grimoire's own,
// by a rename that never replaces an entry, looking at the entry without following it. The sentence that says what
// is never done to the entry is held with the whole function that moves it, so that any change to that function has
// the sentence read again. The floors of free room stand on the Scribe's answers and are no longer placeholders.
#[test]
fn the_docs_follow_the_own_folders_move_aside() {
    let mut fails = Vec::new();
    let at_base = "Err(NOT_OWN) if afresh => base_afresh(&parent, name)?,";
    let below = "let tag = own_dir_at(&base, &root_tag(&store_root()), make)?;";
    let name = "format!(\"{name}.not-own-{secs}-{pid}\")";
    let run = "let arg = if with(|st| !std::mem::replace(&mut st.afresh_given, true)) { format!(\"{USERFONTS_AFRESH}{arg}\") } else { arg.to_string() };";
    let made = "let base = match own_dir_at(&parent, name, afresh) {";
    let line = "it is moved aside as it was, and the folder is made afresh; every font is checked once more";
    told_on(&mut fails, DESIGN, "Grimoire never relies on the parent (`/var/local` on the Kindle): when the entry at the own folder's name is not Grimoire's own, the first check of a run moves it aside within the parent", USERFONTS, at_base);
    told_on(&mut fails, TESTING, "Grimoire never relies on /var/local itself: it tests its own folder at every open and moves aside an entry at its name that is not its own", USERFONTS, at_base);
    told_on(&mut fails, KINDLE, "When the entry at `/var/local/grimoire` is not Grimoire's own (a link, a file, or a folder of another user or that others can write), the first check after Grimoire starts moves it aside as it is, never opened", USERFONTS, at_base);
    told_on(&mut fails, DESIGN, "The folders below the own folder are its own, which no one else can write, and are never moved.", USERFONTS, below);
    told_on(&mut fails, DESIGN, "to `grimoire.not-own-<seconds>-<pid>`, and makes the folder afresh, at most once a run", USERFONTS, name);
    told_on(&mut fails, KINDLE, "to `/var/local/grimoire.not-own-<seconds>-<pid>`, and makes the folder afresh", USERFONTS, name);
    // once a run: the UI's leave, from its memory, for the first check of a run alone; a later check makes nothing
    told_on(&mut fails, DESIGN, "gives the first check of a run alone leave to make the own folder or make it afresh, from its own memory, never from a file", USERFONTS, run);
    told_on(&mut fails, DESIGN, "(the own folder itself by the first check of a run alone)", USERFONTS, made);
    told_on(&mut fails, DESIGN, "or, after the first check of a run, it is gone or not Grimoire's own", USERFONTS, made);
    let later = "That is done once a run: a later check of the same run that finds such an entry there, or nothing, keeps nothing, and no font is checked until Grimoire starts again.";
    told_on(&mut fails, KINDLE, later, USERFONTS, run);
    told_on(&mut fails, KINDLE, later, USERFONTS, made);
    told_on(&mut fails, KINDLE, "at most once a run, by its first check, which then goes on in the fresh folder", USERFONTS, run);
    told_on(&mut fails, TESTING, "Once a run: with Grimoire running", USERFONTS, run);
    // under the Kindle's setgid /var/local the folders Grimoire makes are 02700 and of its group: own_stat asks only
    // the write bits of group and others, so they are Grimoire's own
    let own = "(st.st_mode & libc::S_IFMT) == libc::S_IFDIR && st.st_uid == unsafe { libc::geteuid() } && st.st_mode & 0o022 == 0";
    told_on(&mut fails, TESTING, "under a /var/local with the setgid bit, as in step 1, the folders show `drwx--S---` and its group, which they take from it and which has no rights in them", USERFONTS, own);
    told_on(&mut fails, DESIGN, "by a rename that never replaces an entry", USERFONTS, "rename_noreplace(parent.as_raw_fd(), &c, parent.as_raw_fd(), &to)");
    told_on(&mut fails, DESIGN, "and is never followed, opened, read, listed or deleted", USERFONTS, "libc::fstatat(parent.as_raw_fd(), c.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW)");
    told_on_code(&mut fails, DESIGN, "The entry is renamed by descriptor as it stands, by a rename that never replaces an entry, and is never followed, opened, read, listed or deleted; its kind, owner and mode are logged.", USERFONTS, "fn base_afresh(", BASE_AFRESH);
    // the log line the device check and the log's list quote is the one the code writes
    told_on(&mut fails, TESTING, line, USERFONTS, line);
    told_on(&mut fails, KINDLE, line, USERFONTS, line);
    // the floors: set from step 1's answers, no longer placeholders
    told_on(&mut fails, TESTING, "The floors stand on these numbers: `STATE_FREE_FLOOR`, 4 MB, and `STATE_FILES_FLOOR`, 512 files", USERFONTS, "const STATE_FREE_FLOOR: u64 = 4 << 20;");
    told_on(&mut fails, TESTING, "The floors stand on these numbers: `STATE_FREE_FLOOR`, 4 MB, and `STATE_FILES_FLOOR`, 512 files", USERFONTS, "const STATE_FILES_FLOOR: u64 = 512;");
    gone(&mut fails, USERFONTS, "Placeholders until the Scribe's figures are in");
    gone(&mut fails, TESTING, "which step 1 answers");
    gone(&mut fails, TESTING, "must be root's, with no write bit for group or others");
    none_failed(&fails, "Grimoire's own folder: what is not its own is moved aside");
}

/// `base_afresh` in userfonts.rs, its comments aside (`code_of`): what DESIGN.md's sentence on the entry moved aside
/// stands on.
const BASE_AFRESH: &str = r#"fn base_afresh(parent: &OwnedFd, name: &str) -> Result<OwnedFd, &'static str> { let c = CString::new(name).map_err(|_| NOT_OWN)?; let mut st: libc::stat = unsafe { std::mem::zeroed() }; if unsafe { libc::fstatat(parent.as_raw_fd(), c.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } != 0 { return Err(NOT_OWN); } if own_stat(&st) || MOVED_ASIDE.swap(true, std::sync::atomic::Ordering::Relaxed) { return Err(NOT_OWN); } let (kind, owner, mode) = (kind_of(st.st_mode), st.st_uid, (st.st_mode & 0o7777) as u32); let aside = aside_name(name, wall_time(), std::process::id()); let moved = match CString::new(aside.as_str()) { Ok(to) if fsview_name_check(b"", aside.as_bytes(), true).is_ok() => rename_noreplace(parent.as_raw_fd(), &c, parent.as_raw_fd(), &to), _ => Err(libc::ENAMETOOLONG), }; if let Err(e) = moved { qlog!("fonts: the entry at the name of Grimoire's own folder is not its own (%s, owner %u, mode %04o) and cannot be moved aside (%s)", kind, owner, mode, &strerror(e)); return Err(NOT_OWN); } qlog!("fonts: the entry at the name of Grimoire's own folder was not its own (%s, owner %u, mode %04o): it is moved aside as it was, and the folder is made afresh; every font is checked once more", kind, owner, mode); #[cfg(any(test, feature = "sim"))] if let Some(f) = ASIDE_SEAM.lock().unwrap_or_else(|e| e.into_inner()).take() { f(); } own_dir_at(parent, name, true).inspect_err(|why| qlog!("fonts: the folder made afresh is not Grimoire's own either (%s); nothing more is moved aside", *why)) }"#;

// ---- Cargo.lock against deny.toml and the skrifa pin ----

/// A package of Cargo.lock: its name, its version and what it depends on (a name, with a version when the lock
/// holds more than one of that name).
struct Pkg {
    name: String,
    version: String,
    deps: Vec<(String, Option<String>)>,
}

fn unquoted(s: &str) -> String {
    s.trim().trim_end_matches(',').trim_matches('"').to_string()
}

/// Cargo.lock's `[[package]]` blocks. A dependency is `name`, `name version` or `name version (source)`.
fn lock_packages() -> Vec<Pkg> {
    let mut v: Vec<Pkg> = Vec::new();
    let mut in_deps = false;
    for line in CARGO_LOCK.1.lines() {
        let l = line.trim();
        if l == "[[package]]" {
            v.push(Pkg { name: String::new(), version: String::new(), deps: Vec::new() });
            in_deps = false;
            continue;
        }
        let Some(p) = v.last_mut() else { continue };
        if in_deps {
            if l == "]" {
                in_deps = false;
            } else {
                let d = unquoted(l);
                let mut words = d.split(' ');
                let name = words.next().unwrap_or("").to_string();
                p.deps.push((name, words.next().map(str::to_string)));
            }
        } else if let Some(n) = l.strip_prefix("name = ") {
            p.name = unquoted(n);
        } else if let Some(n) = l.strip_prefix("version = ") {
            p.version = unquoted(n);
        } else if l == "dependencies = [" {
            in_deps = true;
        }
    }
    v
}

/// Where the lock holds the package a dependency names: by name and version, or by name alone when the lock has
/// one version of it.
fn find(pkgs: &[Pkg], name: &str, version: Option<&str>) -> Option<usize> {
    let mut hits = pkgs.iter().enumerate().filter(|(_, p)| p.name == name && version.is_none_or(|v| p.version == v)).map(|(i, _)| i);
    let first = hits.next()?;
    hits.next().is_none().then_some(first)
}

/// The `crate = "..."` values of an array of deny.toml's `[bans]` table (`skip` or `deny`), read by hand: the
/// array is the lines from the one that opens it to the first line that is `]`. (`multiple-versions = "deny"`
/// is a value, not the array: the array's line begins with its key.)
fn bans_array(key: &str, fails: &mut Vec<String>) -> Vec<String> {
    let opens = format!("{key} = [");
    let mut out = Vec::new();
    let (mut table, mut inside) = ("", false);
    for line in DENY_TOML.1.lines() {
        let mut l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if !inside {
            if l.starts_with('[') && l.ends_with(']') && !l.contains('=') {
                table = l;
                continue;
            }
            if table != "[bans]" || !l.starts_with(&opens) {
                continue;
            }
            l = l[opens.len()..].trim();
            inside = !l.ends_with(']');
            if l.is_empty() || l == "]" {
                continue;
            }
        } else if l == "]" {
            inside = false;
            continue;
        }
        let before = out.len();
        let mut rest = l;
        while let Some(at) = rest.find("crate = \"") {
            let v = &rest[at + "crate = \"".len()..];
            let Some(end) = v.find('"') else { break };
            out.push(v[..end].to_string());
            rest = &v[end + 1..];
        }
        if out.len() == before {
            fails.push(format!("{}: an entry of `{key}` this test cannot read (it reads `crate = \"...\"`): {l}", DENY_TOML.0));
        }
    }
    out
}

/// The lock, and what deny.toml and this file say of it.
struct Crates {
    pkgs: Vec<Pkg>,
    /// deny.toml's `skip`: (name, version), both built, each with its reason there
    skip: Vec<(String, String)>,
    /// deny.toml's `deny`: names no build may take
    banned: Vec<String>,
}

/// Packages the lock names for a target or a feature no build of Grimoire has, when neither a ban nor a skip is
/// the right word for them: (name, version, why the build never compiles it). Empty: the one such crate, glifo,
/// is banned in deny.toml, which also makes CI's deny job refuse a build that would take it.
const NAMED_NOT_BUILT: &[(&str, &str, &str)] = &[];

fn crates(fails: &mut Vec<String>) -> Crates {
    let pkgs = lock_packages();
    let mut skip = Vec::new();
    for e in bans_array("skip", fails) {
        match e.split_once('@') {
            Some((n, v)) => skip.push((n.to_string(), v.to_string())),
            None => fails.push(format!("{}: the skip of {e} names no version (name@version)", DENY_TOML.0)),
        }
    }
    let banned = bans_array("deny", fails);
    for b in &banned {
        if b.contains(['@', ':', ' ']) {
            fails.push(format!("{}: the ban of {b} names a version; this test reads a ban as the crate's name alone", DENY_TOML.0));
        }
    }
    Crates { pkgs, skip, banned }
}

impl Crates {
    /// Whether the walk stops before this package: a banned name, or one of NAMED_NOT_BUILT.
    fn cut(&self, p: &Pkg) -> bool {
        self.banned.contains(&p.name) || NAMED_NOT_BUILT.iter().any(|(n, v, _)| *n == p.name && *v == p.version)
    }

    /// Every package reached from `from` over the lock's edges, never entering one that is cut.
    fn reachable(&self, from: usize, fails: &mut Vec<String>) -> Vec<bool> {
        let mut seen = vec![false; self.pkgs.len()];
        seen[from] = true;
        let mut todo = vec![from];
        while let Some(i) = todo.pop() {
            for (name, version) in &self.pkgs[i].deps {
                match find(&self.pkgs, name, version.as_deref()) {
                    Some(j) if !seen[j] && !self.cut(&self.pkgs[j]) => {
                        seen[j] = true;
                        todo.push(j);
                    }
                    Some(_) => {}
                    None => fails.push(format!("{}: {} {} depends on {name}, which is not one package of the lock", CARGO_LOCK.0, self.pkgs[i].name, self.pkgs[i].version)),
                }
            }
        }
        seen
    }

    /// The versions of `name` among the packages marked.
    fn versions(&self, name: &str, marked: &[bool]) -> Vec<&str> {
        self.pkgs.iter().enumerate().filter(|(i, p)| marked[*i] && p.name == name).map(|(_, p)| p.version.as_str()).collect()
    }
}

// deny.toml refuses a crate built in two versions unless both are named in `skip` with a reason (CI's deny job).
// Cargo.lock names more than a build compiles (a crate behind a feature nobody asks for), so here the lock is
// walked from grimoire, stopping at what deny.toml bans, and every name still reached in two versions must have
// each of them in `skip`; a skip for anything else would only hide a second version the day it is built.
// What the lock's edges cannot say is what a build really compiles (features, targets): that half is CI's deny
// job, which reads cargo's own graph.
#[test]
fn every_crate_the_lock_names_twice_is_accounted_for() {
    let mut fails = Vec::new();
    let c = crates(&mut fails);
    let all = vec![true; c.pkgs.len()];
    match find(&c.pkgs, "grimoire", None) {
        None => fails.push(format!("{}: no package grimoire (or more than one)", CARGO_LOCK.0)),
        Some(root) => {
            let built = c.reachable(root, &mut fails);
            let mut names: Vec<&str> = c.pkgs.iter().map(|p| p.name.as_str()).collect();
            names.sort();
            names.dedup();
            for name in names {
                let (named, reached) = (c.versions(name, &all), c.versions(name, &built));
                if named.len() < 2 {
                    continue;
                }
                let skipped = |v: &str| c.skip.iter().any(|(n, sv)| n == name && sv == v);
                if reached.len() >= 2 {
                    for v in reached.iter().filter(|v| !skipped(v)) {
                        fails.push(format!(
                            "{name} {v} is one of {} versions of {name} reached from grimoire, and deny.toml does not account for it. \
                             One of: a `skip` of {name}@{v} with its reason, when both are built; a ban (`deny`) of the crate that \
                             brings it, when it must never be built; a line in this file's NAMED_NOT_BUILT, when the lock names it \
                             for a target or a feature no build of Grimoire has",
                            reached.len()
                        ));
                    }
                } else {
                    for v in named.iter().filter(|v| skipped(v)) {
                        fails.push(format!("deny.toml skips {name}@{v}, but fewer than two versions of {name} are reached from grimoire: the skip would only hide a second one"));
                    }
                }
            }
        }
    }
    for (n, v) in &c.skip {
        if find(&c.pkgs, n, Some(v)).is_none() {
            fails.push(format!("deny.toml skips {n}@{v}, which {} does not name", CARGO_LOCK.0));
        }
    }
    for (n, v, _) in NAMED_NOT_BUILT {
        if find(&c.pkgs, n, Some(v)).is_none() {
            fails.push(format!("NAMED_NOT_BUILT lists {n} {v}, which {} does not name", CARGO_LOCK.0));
        }
    }
    none_failed(&fails, "Cargo.lock against deny.toml");
}

/// The version Cargo.toml's `[dependencies]` asks of skrifa, as written (`"=X"`), from the string form
/// (`skrifa = "=X"`) or the table form (`skrifa = { version = "=X", .. }`).
fn skrifa_asked() -> Option<String> {
    let mut table = "";
    for line in CARGO_TOML.1.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            table = l;
            continue;
        }
        let Some(value) = l.strip_prefix("skrifa").map(str::trim_start).and_then(|r| r.strip_prefix('=')) else { continue };
        if table != "[dependencies]" {
            continue;
        }
        let value = value.trim_start();
        let quoted = match value.strip_prefix('"') {
            Some(q) => q,
            None => value[value.find("version")? + "version".len()..].trim_start().strip_prefix('=')?.trim_start().strip_prefix('"')?,
        };
        return Some(quoted[..quoted.find('"')?].to_string());
    }
    None
}

// Your own fonts are read by the skrifa hayro already brings: one font parser in the build, at one version.
// The lock also names an older skrifa (with its read-fonts), which only glifo asks for, and glifo is compiled
// only with a feature of vello_cpu that hayro does not ask for. With glifo banned in deny.toml the walk from
// hayro reaches one skrifa, and Cargo.toml pins that very version.
#[test]
fn the_one_skrifa_is_the_one_hayro_brings() {
    let mut fails = Vec::new();
    let c = crates(&mut fails);
    // (from hayro, not from hayro-interpret: any of the hayro crates may name skrifa)
    match (find(&c.pkgs, "hayro", None), find(&c.pkgs, "grimoire", None)) {
        (Some(hayro), Some(root)) => {
            let from_hayro = c.versions("skrifa", &c.reachable(hayro, &mut fails)).join(", ");
            let from_root = c.versions("skrifa", &c.reachable(root, &mut fails)).join(", ");
            if from_hayro.is_empty() || from_hayro.contains(',') {
                fails.push(format!("the skrifa versions reached from hayro are not exactly one: [{from_hayro}]"));
            } else {
                if from_root != from_hayro {
                    fails.push(format!("grimoire reaches skrifa [{from_root}], hayro reaches {from_hayro}: not one skrifa"));
                }
                match skrifa_asked() {
                    Some(asked) if asked.strip_prefix('=').map(str::trim) == Some(from_hayro.as_str()) => {}
                    asked => fails.push(format!("{}: skrifa is not pinned to the version hayro brings (\"={from_hayro}\"): {asked:?}", CARGO_TOML.0)),
                }
            }
        }
        _ => fails.push(format!("{}: no package hayro, or no package grimoire", CARGO_LOCK.0)),
    }
    // Cargo.toml says why the lock names a second skrifa, and deny.toml no longer has it the wrong way round
    // (glifo is the optional crate; skrifa is its plain dependency).
    present(&mut fails, CARGO_TOML, "glifo");
    gone(&mut fails, DENY_TOML, "glifo's optional dependency");
    none_failed(&fails, "the one skrifa");
}

// Sentences that said what the code does not do, each gone from the file that held it, and one file name the
// code writes that FORMAT.md did not list.
// Files' rename, delete and restore (issue #144): its public text names no label or file of the private plan, and
// says what the screens do now.
#[test]
fn files_rename_delete_and_restore_are_told_as_they_are() {
    let mut fails = Vec::new();
    gone(&mut fails, BACKLOG, "**Not in 2.2** (D31)");
    gone(&mut fails, BACKLOG, "browser.md 17 has it as an optional extra");
    gone(&mut fails, FSVIEW, "browser.md D4");
    // the Trash place has Restore: it is not read only
    gone(&mut fails, DESIGN, "lists `grimoire/trash` read only, through");
    // Delete asks first (the Library's question): it is not one tap
    gone(&mut fails, DESIGN, "**Delete** is one tap on the card");
    gone(&mut fails, TESTING, "its card > Delete (moves it to grimoire/trash): \"Moved to grimoire/trash\"");
    // the rename's words: a name taken (a folder's too), a name too long for the keyboard
    gone(&mut fails, DESIGN, "A file with that name is already here.");
    gone(&mut fails, BACKLOG, "A file with that name is already here.");
    gone(&mut fails, TESTING, "A file with that name is already here.");
    gone(&mut fails, DESIGN, "is not offered to it (\"Couldn't rename file.\")");
    // after a write the rows follow from the listing held: the folder is not read again at one go
    gone(&mut fails, DESIGN, "Renamed: the folder is read again, with its rows' page kept.");
    gone(&mut fails, DESIGN, "\"Moved to grimoire/trash\" and the folder read again.");
    // which is the folder shown, no more: a font renamed, the font in use and a notebook restored read what DESIGN
    // lists (userfonts_renamed, userfonts_apply, store_restored and the Library's reload)
    gone(&mut fails, DESIGN, "After a write nothing is read again.");
    gone(&mut fails, DESIGN, "no folder is listed and no row stat-ed anew");
    gone(&mut fails, BROWSER, "the listing held: nothing is read again");
    // who writes where the font check keeps its results: the job, but for the UI's renames (a .checking mark to
    // .bad, and what the check found of a font renamed in Files, to its new key)
    gone(&mut fails, USERFONTS, "but for the UI's rename of a .checking mark to .bad when that job died; deleting it");
    gone(&mut fails, USERFONTS, "background job alone.");
    gone(&mut fails, USERFONTS, "the UI never writes there");
    // the font in use: not deleted, and a rename takes its setting along
    gone(&mut fails, BACKLOG, "a font in use goes back to Terminess, as it does when its file goes over USB.");
    none_failed(&fails, "Files' rename, delete and restore");
}

// Files' home folder (issue #160): the safety sentences of DESIGN.md and FORMAT.md name what the code has (the value
// read strictly, what it names reached one way only, nothing opened at a home that fails, the refused folders known
// by identity as well as by name), the figure of the value's limit is the constant's, and the sentences the change
// replaced, and a label of the private plan, are gone.
#[test]
fn files_home_is_told_as_it_is() {
    let mut fails = Vec::new();
    // the value's limit, as DESIGN.md and FORMAT.md give it
    let max = grimoire::home::HOME_MAX.to_string();
    beside(&mut fails, DESIGN, "`HOME_MAX`", &max);
    present(&mut fails, FORMAT, &format!("under {max} bytes"));
    // the names the sentences give, and the code that has them
    present(&mut fails, DESIGN, "`Home::parse`");
    present(&mut fails, HOME, "pub fn parse(v: &[u8]) -> Option<Home>");
    present(&mut fails, DESIGN, "`FsStorage::folder`");
    present(&mut fails, FSVIEW, "pub fn folder(&self, p: &StoragePath) -> Result<FsStat, FsErr>");
    present(&mut fails, DESIGN, "`Browser::enter_home`");
    present(&mut fails, BROWSER, "pub fn enter_home(&mut self, p: &StoragePath) -> bool");
    present(&mut fails, DESIGN, "`home_folder_ok`");
    present(&mut fails, HOME, "pub fn home_folder_ok(st: &FsStorage, p: &StoragePath) -> bool");
    // nothing opened at a home that fails: home_files walks to it first
    present(&mut fails, DESIGN, "`home_files` walks to it (`home_folder_there`) before Files is opened");
    present(&mut fails, LIBRARY, "home_folder_there(p)?;");
    // the refused folders by identity as well as by name, and a path part ending in a dot or a space refused
    present(&mut fails, DESIGN, "`FsStorage.refused`");
    present(&mut fails, FSVIEW, "refused: Vec<(u64, u64)>,");
    present(&mut fails, FSVIEW, "self.refused.iter().any(|&o| Some(o) == id)");
    present(&mut fails, DESIGN, "a name ending in a dot or a space");
    present(&mut fails, FSVIEW, "matches!(n.last(), Some(b'.' | b' '))");
    // what the change replaced: the action's old place, Esc at a Library home, a delete that left home set
    gone(&mut fails, DESIGN, "`home` is the Library's top for now.");
    gone(&mut fails, BACKLOG, "Today `home` (the action) goes to the Library's top");
    gone(&mut fails, DESIGN, "At a Library folder that is home Esc asks \"Exit Grimoire?\" too");
    gone(&mut fails, DESIGN, "home folder moved to the trash in Files is gone until Restore puts it back");
    gone(&mut fails, DESIGN, "the owner's D31");
    none_failed(&fails, "Files' home folder");
}

#[test]
fn the_docs_no_longer_say_what_the_code_does_not() {
    let mut fails = Vec::new();
    // picking a template or photo reads none of its bytes on the UI thread (test_picture_jobs.rs holds the code)
    gone(&mut fails, TEMPLATES, "only a small file it has not seen yet is");
    // what a build off the Kindle takes of the sandbox (sandbox.rs: standard_steps, apply_steps, sandboxed_line)
    gone(&mut fails, DESIGN, "Off the Kindle the steps are skipped");
    gone(&mut fails, KINDLE, "anywhere the steps are skipped");
    gone(&mut fails, SANDBOX, "the steps that need root or Linux are skipped");
    // what the UI process reads of a PDF (pdfimport.rs: read_ends; browser.rs: book_key)
    gone(&mut fails, DESIGN, "never reads the PDF beyond the ends of the file");
    // the allow-list as allow_list_for has it, and the calls that answer ENOSYS (build_filter)
    gone(&mut fails, DESIGN, "rt_sigreturn, exit) whose violation traps into a reporter");
    // a panic in the Kindle's build is an abort, not an "E panic" line (Cargo.toml: panic = "abort")
    gone(&mut fails, SANDBOX, "reports \"E panic\"");
    // your own fonts: the second try and its mark, and who writes fonts/.cache (userfonts.rs: bg_font_check,
    // userfonts_job_done)
    gone(&mut fails, FORMAT, "and only the background job writes it.");
    gone(&mut fails, DESIGN, "in all, then \"Took too long to read.\"");
    gone(&mut fails, KINDLE, "took too long to read (T ms)`, or");
    gone(&mut fails, USERFONTS, "Written by the background job only");
    gone(&mut fails, USERFONTS, "(nothing is marked), -1 it could not start");
    present(&mut fails, FORMAT, "`<key>.slow`");
    // where the tests of the sandbox's real steps have run (CI's sandbox job), and where the steps have
    gone(&mut fails, TESTING, "Not run anywhere yet: the sandbox's real steps");
    gone(&mut fails, TESTING, "Not run anywhere yet: a picture in the sandbox's real steps");
    gone(&mut fails, TESTING, "Not run anywhere yet: a font in the sandbox's real steps");
    gone(&mut fails, TESTING, "Not run anywhere yet: those two");
    gone(&mut fails, BACKLOG, "and its sandbox steps have not run anywhere");
    gone(&mut fails, BACKLOG, "placeholders until S1 measures the Scribe");
    gone(&mut fails, SANDBOX_TESTS, "HAVE NOT BEEN RUN");
    gone(&mut fails, SANDBOX_TESTS, "and a forced failed step falls back and says so");
    // the steps that have run on a Scribe are 2.1.0-beta.2's: the narrower system-call rules that came after
    // have a check of their own, which the sandbox round's sections list as not yet run. Two sentences said
    // "the steps have run" without that limit; the two that replace them stand on those sections, so the day
    // the check has run and its sections say so, these fail until they follow.
    gone(&mut fails, SANDBOX_TESTS, "steps themselves have run on a Scribe (sandbox.rs, `s1`).");
    gone(&mut fails, BACKLOG, "the steps have (S1, and the imports, pictures and fonts of 2.1.0-beta.2).");
    stands_on(
        &mut fails,
        SANDBOX_TESTS,
        "wait for their check there (docs/TESTING.md, \"Hardening round (2026-11), sandbox\")",
        TESTING,
        "**Not run on a Scribe, and it must be before a release**",
    );
    stands_on(
        &mut fails,
        BACKLOG,
        "are the entry \"The Scribe check of this round has not run\", above",
        BACKLOG,
        "- **The Scribe check of this round has not run**",
    );
    // five sentences of TESTING.md sent the reader to the sandbox round's section by its place ("at the end",
    // "the round above"), and other rounds' sections have come to stand after it: they name it now, and the
    // name must be a heading of the file. ("of the round above" is held as the whole clause it stood in: the
    // four words alone could be any later sentence's.)
    gone(&mut fails, TESTING, "sandbox\", at the end)");
    gone(&mut fails, TESTING, "sandbox\" at the end");
    gone(&mut fails, TESTING, "the hardening round's section at the end");
    gone(&mut fails, TESTING, "allow-list's of the round above among them");
    stands_on(&mut fails, TESTING, "(\"Hardening round (2026-11), sandbox\", below)", TESTING, "## Hardening round (2026-11), sandbox: ");
    stands_on(&mut fails, TESTING, "see \"Hardening round (2026-11), sandbox\", below.", TESTING, "## Hardening round (2026-11), sandbox: ");
    // the crates: glifo is the optional one (vello_cpu's), and skrifa is glifo's plain dependency
    gone(&mut fails, BACKLOG, "glifo's optional dependency");
    // a PDF whose name never holds the file read is read three times in all, so read again twice (#110); and the
    // bound on its tries holds only while the scan's listing and the open file agree on its size and time
    gone(&mut fails, TESTING, "read again up to three times");
    gone(&mut fails, BACKLOG, "read again up to three times");
    gone(&mut fails, DESIGN, "the try stays counted, so this cannot repeat without end. The record");
    // licenses/rust-crates.txt lists every crate the lock names, not only the ones built in
    gone(&mut fails, LICENSES_TOOL, "the Grimoire binary is built from (cargo's Cargo.lock decides them");
    // the same file gives the two-week rule for crates as the rule of a release and says where an exception is
    // recorded: while the tool writes that, RELEASING.md has the rule and TESTING.md has a release's exception
    stands_on(
        &mut fails,
        LICENSES_TOOL,
        "two weeks old on the day of the tag (docs/RELEASING.md)",
        RELEASING,
        "**Every crate is at least two weeks old.**",
    );
    stands_on(
        &mut fails,
        LICENSES_TOOL,
        "exception made for a release is recorded in docs/TESTING.md",
        TESTING,
        "crate-age rule (two weeks at the tag) was waived",
    );
    // and while TESTING.md's look at the shipped file quotes its first words, the tool writes them
    stands_on(
        &mut fails,
        TESTING,
        "The file begins \"From the Rust crates Cargo.lock names for the Grimoire binary.\"",
        LICENSES_TOOL,
        "\"From the Rust crates Cargo.lock names for the Grimoire binary.",
    );
    none_failed(&fails, "the documents and comments");
}

// Add a font... and Add to fonts (2.2, issue #161): a font file copied into grimoire/fonts by the copy job is no
// font until its check has passed. The sentences that say what keeps a copy from being taken for a font stand on
// the code that does it: the temporary name's form (bgwork_copy.rs) and the scan's refusal of a dot-name
// (userfonts.rs); the look the copy asks for, which sets a time and nothing else; the limit with its figure; the
// Kindle's own fonts folder outside the folders a write may touch (fsview.rs); and what reads a font file as a font
// (nothing before its check has passed it) and what draws its preview in Files' Covers view (the sandboxed worker).
// The day one of those changes, the sentence fails here until it is read again and follows.
#[test]
fn a_font_added_from_a_file_is_told_as_it_is() {
    let mut fails = Vec::new();
    let tmp_head = "const TMP_HEAD: &str = \".grimoire-copy-\";";
    let tmp_tail = "const TMP_TAIL: &str = \".part\";";
    let dot_name = "if name.starts_with('.') { return NameKind::Other; }";
    let words = "a dot-name, which no listing shows and which the fonts folder's scan takes for no font";
    for base in [(BGWORK_COPY, tmp_head), (BGWORK_COPY, tmp_tail), (USERFONTS, dot_name)] {
        stands_on(&mut fails, DESIGN, words, base.0, base.1);
    }
    stands_on(&mut fails, BGWORK_COPY, "a dot-name with the ending `.part`: no listing shows it and the fonts folder's scan takes no such name", USERFONTS, dot_name);
    stands_on(&mut fails, USERFONTS, "a copy still being written has a dot-name ending in `.part`", BGWORK_COPY, tmp_tail);
    // what the copy asks of the font check: a time to look at the folder, nothing more. What follows a copy that went
    // in is copy_told (app.rs), whose one call for a font is userfonts_added, which sets that time and the scale:
    // both held whole, so that a line that applied, loaded or read the font there would fail here.
    const COPY_TOLD: &str = r#"fn copy_told(a: &mut App, to: Option<crate::bgwork_copy::CopyTo>, r: i32) { use crate::bgwork_copy::{copy_failed, copy_refusal, CopyFail, CopyTo}; if r < 0 { let msg = to.map_or_else(|| copy_failed().to_string(), |to| copy_refusal(CopyFail::of_exit(bg_last_exit()), to)); app_toast(a, &msg, 4000); return; } let Some(to) = to.filter(|_| r > 0) else { return }; if to == CopyTo::Fonts { crate::userfonts::userfonts_added(a.dpi); } if crate::library::library_sheet_folder(a) == Some(to) || crate::editor::editor_sheet_folder(a) == Some(to) { return; } let place = match to { CopyTo::Templates => tr("Templates"), CopyTo::Photos => tr("Photos"), CopyTo::Fonts => tr("Fonts"), }; let mut msg = String::new(); csnprintf!(&mut msg, 256, tr("Copied to %s."), place); app_toast(a, &msg, 3000); }"#;
    const USERFONTS_ADDED: &str = r#"pub fn userfonts_added(dpi: f32) { with(|st| { st.dpi = dpi; st.added_ms = Some(now_ms().wrapping_add(added_wait_ms()).max(1)); }); }"#;
    for (f, words) in [
        (DESIGN, "**The copy gives a font nothing.**"),
        (USERFONTS, "the copy gives it nothing (`userfonts_added` only says when the folder is looked at)"),
    ] {
        told_on_code(&mut fails, f, words, APP, "fn copy_told(", COPY_TOLD);
        told_on_code(&mut fails, f, words, USERFONTS, "pub fn userfonts_added(", USERFONTS_ADDED);
    }
    // the limit a font over is refused at, before a byte is copied
    beside(&mut fails, DESIGN, "`FONT_FILE_MAX`", &format!("{} MB", grimoire::fontcheck::FONT_FILE_MAX >> 20));
    // and the longest name a copy may have there, which is what the scan takes of a font's path
    beside(&mut fails, DESIGN, "`USERFONT_REL_MAX`", &format!("{} bytes", grimoire::userfonts::USERFONT_REL_MAX));
    stands_on(&mut fails, DESIGN, "so it is refused as \"Path too long.\"", BGWORK_COPY, "CopyTo::Fonts => crate::userfonts::USERFONT_REL_MAX.min(FS_NAME_MAX),");
    // the Kindle's fonts folder: a write is allowed only inside Grimoire's own folders. A rename or a delete asks
    // write_zone (the allow-list, inside Grimoire's folder only), which asks zone_of (writable only below Grimoire's
    // own folder, never at the storage's root): both held whole. The copy job writes only into Grimoire's folder of
    // its kind, and only where zone_of says writable.
    const ZONE_OF: &str = r#"fn zone_of(grimoire: Option<&StoragePath>, p: &StoragePath, pick: bool) -> FsZone { let comps: Vec<&[u8]> = p.components().collect(); let odd_end = |n: &[u8]| matches!(n.last(), Some(b'.' | b' ')); if comps.iter().any(|n| n.first() == Some(&b'.') || odd_end(n) || (n.len() >= 4 && eq_nocase(&n[n.len() - 4..], b".sdr"))) { return FsZone::Refused; } if comps.first().is_some_and(|n| SYSTEM.iter().any(|s| eq_nocase(n, s))) { return FsZone::Refused; } if let Some(g) = grimoire { let gc: Vec<&[u8]> = g.components().collect(); if !gc.is_empty() && comps.len() > gc.len() && gc.iter().zip(&comps).all(|(a, b)| eq_nocase(a, b)) { let n = comps[gc.len()]; if OWN_REFUSED.iter().any(|s| eq_nocase(n, s)) || (pick && OWN_NOT_PICKED.iter().any(|s| eq_nocase(n, s))) { return FsZone::Refused; } if OWN_WRITABLE.iter().any(|s| eq_nocase(n, s)) { return FsZone::Writable; } if comps.len() == gc.len() + 1 && n.len() >= 4 && eq_nocase(&n[n.len() - 4..], b".ini") { return FsZone::Refused; } } } FsZone::ReadOnly }"#;
    const WRITE_ZONE: &str = r#"fn write_zone(&self, p: &StoragePath) -> Option<&'static str> { let g = self.grimoire.as_ref()?; if zone_of(Some(g), p, false) != FsZone::Writable { return None; } let gl = g.components().count(); let comps: Vec<&[u8]> = p.components().collect(); if comps.len() < gl + 2 || (comps.len() == gl + 2 && eq_nocase(comps[gl], b"import") && eq_nocase(comps[gl + 1], b"done")) { return None; } ["import", "templates", "photos", "fonts"].into_iter().find(|w| eq_nocase(comps[gl], w.as_bytes())) }"#;
    let read_only = "**The Kindle's fonts folder is read only**, as every folder outside Grimoire's own is";
    told_on_code(&mut fails, DESIGN, read_only, FSVIEW, "fn zone_of(", ZONE_OF);
    told_on_code(&mut fails, DESIGN, read_only, FSVIEW, "fn write_zone(", WRITE_ZONE);
    told_on(&mut fails, DESIGN, read_only, BGWORK_COPY, r#"let dest = g.join(to.folder().as_bytes()).ok_or("a path too long")?;"#);
    told_on(&mut fails, DESIGN, read_only, BGWORK_COPY, r#"if st.zone(&dest.join(name).ok_or_else(|| long("a path too long"))?, false) != FsZone::Writable {"#);
    // Nothing in the writer or the UI parses, applies, measures or loads a copied font before its check has passed
    // it: what reads a font file as a font in this process is load_file, which takes only bytes that hash to a
    // passing verdict's, and guarded_face, the one maker of a face from them. A preview of it in Files' Covers view
    // is the sandboxed worker's sample (font_preview, by font_sample's worker). Those sentences are held with that
    // code; the words that said a copied font is "not sampled" and has "no preview" are gone (Covers makes one).
    const LOAD_FILE: &str = r#"fn load_file(st: &FsStorage, rel: &str, dpi: f32, room: usize) -> Result<Loaded, LoadErr> { if !userfont_rel_ok(rel) { return Err(LoadErr::Missing); } let sp = StoragePath::new(rel.as_bytes()).ok_or(LoadErr::Missing)?; let s = st.stat(&sp, false).map_err(|_| LoadErr::Missing)?; let l = Listed { rel: rel.to_string(), size: s.size, mtime: s.mtime, key: key_of(rel, s.size, s.mtime) }; if name_kind(rel.rsplit('/').next().unwrap_or("")) != NameKind::Font || l.size == 0 || l.size > FONT_FILE_MAX as u64 || guard_out(l.key) { return Err(LoadErr::Refused); } let Some(v) = VerdictDir::open(false, false).ok().and_then(|vd| verdict_for(&vd, l.key, l.size)) else { let bad = std::path::Path::new(&cache_file(l.key, "bad")).exists(); if bad { return Err(LoadErr::Refused); } queue_check("", dpi); return Err(LoadErr::Checking); }; let VerdictKind::Passed(facts) = v.what else { return Err(LoadErr::Refused) }; if l.size as usize > room { return Err(LoadErr::Memory); } let data = read_listed(st, &l).ok_or(LoadErr::Missing)?; if font_hash(&data) != v.hash { qlog!("font: a font file is not the one that was checked: checking it again"); queue_check(&format!("recheck:{:016x}", l.key), dpi); return Err(LoadErr::Checking); } Ok(Loaded { data: Rc::new(data), facts: Rc::new(facts), key: l.key }) }"#;
    const GUARDED_FACE: &str = r#"fn guarded_face(l: &Loaded, bold: bool) -> Option<SafeFace> { if !guard_arm_keys(&[l.key]) { qlog!("font: the guard's mark could not be written: your font is not used"); return None; } let face = SafeFace::new(l.data.clone(), l.facts.clone(), bold); if face.is_none() && !bold { guard_disarm_keys(&[l.key]); } face }"#;
    const FONT_PREVIEW: &str = r#"fn font_preview(data: &[u8]) -> Result<Canvas, Fail> { use crate::fontcheck::*; match font_sample(data, PREVIEW_W, PREVIEW_H, mono_ms() + preview_limit_ms(PreviewKind::Font)) { Ok(px) => gray_of(&px, PREVIEW_W, PREVIEW_H, 1), Err(FontOut::Refused(_)) => Err(Fail::Refused), Err(FontOut::Slow | FontOut::SlowGlyphs { .. }) => Err(Fail::Strike(NoPreview::Slow)), Err(FontOut::Dead(_)) => Err(Fail::Strike(NoPreview::Ended)), Err(FontOut::NoWorker(_)) => Err(Fail::Later(NO_WORKER)), Err(_) => Err(Fail::Later("out of memory")), } }"#;
    let covers = "a preview of it in Files' Covers view is made by the sandboxed worker, as for any font in Files (issue #145)";
    let covers_cap = "A preview of it in Files' Covers view is made by the sandboxed worker, as for any font in Files (issue #145)";
    for (f, parses, covers) in [
        (DESIGN, "Nothing in the writer or the UI parses, applies, measures or loads the file before its check has passed it", covers_cap),
        (USERFONTS, "Nothing in the writer or the UI parses, applies, measures or loads it before then", covers),
        (ADDFONT_TESTS, "Until then nothing in the writer or the UI parses, applies, measures or loads it", covers_cap),
    ] {
        told_on_code(&mut fails, f, parses, USERFONTS, "fn load_file(", LOAD_FILE);
        told_on_code(&mut fails, f, parses, USERFONTS, "fn guarded_face(", GUARDED_FACE);
        told_on_code(&mut fails, f, covers, PREVIEWS, "fn font_preview(", FONT_PREVIEW);
        told_on(&mut fails, f, covers, FONTCHECK, "let mut wk = match start_worker_until(data.len(), until) {");
    }
    gone(&mut fails, DESIGN, "not applied, not sampled, not measured, no preview, no tap");
    gone(&mut fails, USERFONTS, "but never applied, sampled or measured before");
    gone(&mut fails, ADDFONT_TESTS, "Until then it is listed and no more: not loaded, not drawn");
    none_failed(&fails, "a font added from a file");
}

// Darker lines in imported PDFs (2.2, issue #173): the rules DESIGN.md and FORMAT.md give, and the code that holds
// each. A gamma on gray pages only, applied by the writer to the page it holds; Off untouched; a fill at its
// notebook's level; every value from a file read strictly.
#[test]
fn darker_lines_are_told_as_they_are() {
    let mut fails = Vec::new();
    // the writer, on the gray page it holds: no new parse, no new decode, the worker as it was
    present(&mut fails, DESIGN, "The writer applies the gamma to the gray page it already holds");
    present(&mut fails, DESIGN, "no new parse of the PDF, no new decode, and the worker and its protocol are unchanged");
    present(&mut fails, PDFIMPORT, "darken(&mut p, state.gamma);");
    // gray pages only, and nothing at Off
    present(&mut fails, DESIGN, "A page kept in colour is left exactly as it is, and a page with gray and colour on it is a colour page");
    present(&mut fails, DESIGN, "At Off nothing is applied");
    present(&mut fails, PDFWORK, "if g == Gamma::Off || p.ch != 1 {");
    // a fill draws at its notebook's own level
    present(&mut fails, DESIGN, "A fill draws the pages it adds at its notebook's own level");
    present(&mut fails, PDFIMPORT, "st.gamma = p.gamma;");
    present(&mut fails, PDFIMPORT, "pub fn gamma_among(kept: &[u8]) -> Gamma {");
    // read strictly: the three values, anything else no gamma, in every file that says a level
    present(&mut fails, FORMAT, "Each of these lines is read strictly: `1.0`, `1.5` or `2.0`, and any other value is no gamma.");
    present(&mut fails, FORMAT, "`kind`, `time` and `gamma` comes once at most");
    present(&mut fails, PDFWORK, "b\"1.5\" => Gamma::Medium, b\"2.0\" => Gamma::Strong, _ => Gamma::Off,");
    present(&mut fails, STORE, "keep_named(\"pdf_gamma\", v, &crate::pdfwork::Gamma::NAMES)");
    present(&mut fails, ANNOTATE, "r.gamma = Some(crate::pdfwork::Gamma::parse(v));");
    present(&mut fails, ANNOTATE, "b\"kind\", b\"time\", b\"gamma\"];");
    // (the mark's, the import state's and source.ini's readers, and a version 1 request's, which drops the line)
    present(&mut fails, PDFIMPORT, "\"gamma\" => m.gamma = Gamma::parse(v.as_bytes()),");
    present(&mut fails, PDFIMPORT, "\"gamma\" => s.gamma = Gamma::parse(v.as_bytes()),");
    present(&mut fails, PDFIMPORT, ".find_map(|line| line.strip_prefix(b\"gamma=\")).map_or(Gamma::Off, Gamma::parse)");
    present(&mut fails, ANNOTATE, "r.file.clear(); r.gamma = None;");
    // what the change took away
    gone(&mut fails, BACKLOG, "A `gamma=` line waits with the darker lines import option");
    none_failed(&fails, "darker lines in imported PDFs");
}

// PDFs as templates (issue #172): the safety sentences of DESIGN.md held with the code that makes them true (only the
// worker parses the PDF, nothing drawn or marked without one, no job queued in a broken session, the page limit whole,
// the cache reached with no link, the floor, nothing kept of a file that changed, never an import, the job's rank,
// nothing read again under a contact), and the page limit's figure beside its name in DESIGN.md and FORMAT.md.
#[test]
fn pdfs_as_templates_are_told_as_they_are() {
    let mut fails = Vec::new();
    let max = grimoire::bgwork_tplpdf::TEMPLATE_PDF_MAX_PAGES.to_string();
    beside(&mut fails, DESIGN, "`TEMPLATE_PDF_MAX_PAGES`", &max);
    present(&mut fails, FORMAT, &format!("n from 1 to {max} in decimal"));
    present(&mut fails, FORMAT, &format!("more than {max} pages"));
    // only the worker parses the PDF, the file the job opened
    told_on(&mut fails, DESIGN, "the writer and the UI never parse PDF syntax", TPLPDF, "let mut s = Session::start(self.file.as_raw_fd(), self.md.len(), &self.limits)?;");
    told_on(&mut fails, DESIGN, "The job opens the PDF once, with no link followed", TPLPDF, "let Ok((file, md)) = open_read_nofollow(src) else {");
    // no worker: nothing drawn, nothing marked; a broken session queues nothing
    let waits = "nothing is drawn and nothing is marked: the PDF's tile waits, as a picture's does";
    told_on(&mut fails, DESIGN, waits, TPLPDF, r#"Err(StartErr::Worker(why)) => return Err(Stop::Later(format!("no worker ({why})"))),"#);
    told_on(&mut fails, DESIGN, waits, TPLPDF, "crate::sandbox::session_set_broken(); Stop::Later(");
    told_on(&mut fails, DESIGN, "A session marked broken queues no PDF's job until Grimoire starts again", TPLPDF, "if crate::sandbox::session_broken() { return; }");
    told_on(
        &mut fails,
        DESIGN,
        "the job's child ends with status 4 (`CODE_SANDBOX`), which bgjob.rs turns into the UI's mark",
        BGJOB,
        "if template_pdf(&job) && code == crate::bgwork_tplpdf::CODE_SANDBOX { crate::sandbox::session_set_broken();",
    );
    // the page limit, whole
    told_on(&mut fails, DESIGN, "is no template as a whole and is never cut to its first pages", TPLPDF, "if n > TEMPLATE_PDF_MAX_PAGES {");
    told_on(&mut fails, DESIGN, "over 16 MB (marked before any worker starts)", TPLPDF, "if size > TEMPLATE_MAX_BYTES as u64 {");
    // the tiles PDFs give in all, and a PDF past them left out whole, with no job
    beside(&mut fails, DESIGN, "`TEMPLATE_PDF_TILES_MAX`", &grimoire::bgwork_tplpdf::TEMPLATE_PDF_TILES_MAX.to_string());
    told_on(&mut fails, DESIGN, "is left out whole, as one tile the sheet cannot use, with no job queued for it", TPLPDF, "if tiles > *left { out.push(whole(true)); return None; }");
    // written only in Grimoire's own folder, reached with no link; the floor before every page
    told_on(&mut fails, DESIGN, "into a cache folder reached with no link on the way", TPLPDF, "if !no_link_below(&store_root(), &cache) || mkdir_p(&cache) != 0 {");
    told_on(&mut fails, DESIGN, "the free-space floor before every page", TPLPDF, "Some(free) if !crate::fsview::fsview_room_for(free, need) =>");
    // ... at each of its two places: before a page's worker is asked, and with the page's real size before it is kept
    told_on(&mut fails, DESIGN, "room for a page before its worker is asked", TPLPDF, "self.room(PAGE_ROOM)?; match self.page(k)? {");
    told_on(&mut fails, DESIGN, "and for its picture and preview before they are written", TPLPDF, "if let Err(e) = self.room(need) {");
    // a file changed while its pages are made: the count is not kept either
    told_on(
        &mut fails,
        DESIGN,
        "The job checks before it keeps the count and before it keeps each page",
        TPLPDF,
        "if !self.same() { return Err(Stop::Changed); } if known != Some(n) {",
    );
    // a file changed while its pages are made: nothing of it after the change is kept
    told_on(
        &mut fails,
        DESIGN,
        "no page of it drawn after the change is kept",
        TPLPDF,
        "(nothing of a file that changed while its worker read it is kept) if !self.same() { return Err(Stop::Changed); } return self.store(k, p);",
    );
    // a page is its picture: drawing or picking it never reads the PDF
    told_on(&mut fails, DESIGN, "Picking a page, drawing it and opening a notebook on it never read the PDF again", TPLPDF, r#"t.path.set(&f("png"));"#);
    told_on(&mut fails, DESIGN, "Picking a page, drawing it and opening a notebook on it never read the PDF again", TEMPLATES, "if let Some(stem) = crate::bgwork_tplpdf::page_stem(path) {");
    // the PDF itself is never a picture: no preview of its own, never installed, never a notebook's picture job
    told_on(&mut fails, DESIGN, "The PDF itself is never a picture", TEMPLATES, "if crate::bgwork_tplpdf::is_pdf_name(&path[slash + 1..]) { return false; }");
    told_on(&mut fails, DESIGN, "a notebook's picture job refuses a PDF, so the picture worker is never handed one", TEMPLATES, "a PDF is no picture (its pages are the templates); nothing made\", id); return -1;");
    // never an import
    told_on(&mut fails, DESIGN, "The import watch reads `grimoire/import` alone", PDFIMPORT, r#"let dir = root_path("import");"#);
    // ... and the template job makes no notebook and queues no import (the user's own Import as notebook aside)
    told_on(&mut fails, DESIGN, "the template job never makes a notebook of a PDF in the templates folder", PDFIMPORT, r#"let dir = root_path("import");"#);
    gone(&mut fails, TPLPDF, "store_create");
    gone(&mut fails, TPLPDF, "pdf_import_queue");
    gone(&mut fails, TPLPDF, "import_pick_queue");
    // removals look for a link again right before they list the cache
    let removals = "looks for a link on the way again right before it lists the cache";
    told_on(&mut fails, DESIGN, removals, TPLPDF, "fn remove_entries(&self) { if !no_link_below(&store_root(), &self.cache) { return; }");
    told_on(&mut fails, DESIGN, removals, TEMPLATES, "fn sweep_cache(cache: &str, keep: &[String]) { if !no_link_below(&store_root(), cache) { return; }");
    // the job's rank, and its slices
    told_on(&mut fails, DESIGN, "It ranks behind imports and font checks, with Files' previews", BGJOB, "if template_pdf(j) { return 4; }");
    stands_on(&mut fails, DESIGN, "It ranks behind imports and font checks, with Files' previews", BGJOB, "BgKind::FilePreviews => 4,");
    told_on(&mut fails, DESIGN, "looked at before a page, never between the tries of one", TPLPDF, "if made + marked >= max || mono_ms().saturating_sub(t0) >= s1::SLICE_WALL_MS as u64 {");
    // the sheet: a PDF's tile it cannot use stays, crossed out (a picture's is left out, as on main)
    told_on(&mut fails, DESIGN, "A PDF's tile the sheet cannot use stays crossed out in its place at every opening", TPLSHEET, "if s.t[i].bad && !pdf_tile(&s.t[i]) { continue; }");
    // the sheet: nothing read again under a contact
    told_on(&mut fails, DESIGN, "Never under a contact: a count seen while a pen or finger is down on the sheet is read after the lift.", TPLSHEET, "if s.press.down { s.pdf_counted = true; }");
    none_failed(&fails, "PDFs as templates");
}

// Web links as a QR card (2.2, issue #174). What keeps versions before 2.2 safe: the web links are in a file of their
// own, which no version before 2.2 opens, and doclinks.ini (which 2.1.0 reads and carries only 16 KiB of lines it does
// not know in) never holds one: its writer writes page targets alone. What an address may be: the rules, held against
// the whole function, checked by the worker, the writer and the reader; a notebook without web links has no file.
// The card draws the stored text and its code and opens nothing. The old words that said web links are left out are
// gone.
#[test]
fn web_links_are_told_as_they_are() {
    let mut fails = Vec::new();
    told_on(&mut fails, FORMAT, "2.1.0 and every version before it never read or write `weblinks.ini`", WEBLINKS, r#"pub const WEBLINKS_FILE: &str = "weblinks.ini";"#);
    told_on(&mut fails, FORMAT, "`doclinks.ini` holds no web link and carries no line for one", DOCLINKS, r#"text.push_str(&format!("link.{}={:08x},p{}\n", l.pid, l.seed, l.target));"#);
    told_on(&mut fails, DESIGN, "In a file of their own the web links are simply absent in 2.1.0 and before, which never open it and never write over it", WEBLINKS, r#"pub const WEBLINKS_FILE: &str = "weblinks.ini";"#);
    const ADDRESS_OK: &str = r#"pub fn weblinks_address_ok(b: &[u8]) -> bool { if b.len() > WEBLINK_ADDRESS_MAX || !b.iter().all(|&c| (0x21..=0x7e).contains(&c) && c != b'\\') { return false; } let rest = if b.len() >= 7 && b[..7].eq_ignore_ascii_case(b"http://") { &b[7..] } else if b.len() >= 8 && b[..8].eq_ignore_ascii_case(b"https://") { &b[8..] } else { return false; }; let end = rest.iter().position(|c| matches!(c, b'/' | b'?' | b'#')).unwrap_or(rest.len()); weblinks_authority(&rest[..end]).is_some() }"#;
    const AUTHORITY: &str = r#"fn weblinks_authority(a: &[u8]) -> Option<(&[u8], Option<u32>)> { let (host, port) = match a.iter().position(|&c| c == b':') { Some(i) => (&a[..i], Some(&a[i + 1..])), None => (a, None), }; if host.is_empty() || !host.iter().all(|&c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')) { return None; } let num = |p: &[u8], max: u32| -> Option<u32> { if p.is_empty() || p.len() > 5 || !p.iter().all(u8::is_ascii_digit) || (p.len() > 1 && p[0] == b'0') { return None; } let v = p.iter().fold(0u32, |v, &c| v * 10 + (c - b'0') as u32); (v <= max).then_some(v) }; let port = match port { None => None, Some(p) => Some(num(p, 65535).filter(|&v| v > 0)?), }; let last = host.strip_suffix(b".").unwrap_or(host).rsplit(|&c| c == b'.').next().unwrap_or(b""); let hex = last.len() >= 2 && last[0] == b'0' && (last[1] | 0x20) == b'x' && last[2..].iter().all(u8::is_ascii_hexdigit); if hex || (!last.is_empty() && last.iter().all(u8::is_ascii_digit)) { let parts: Vec<&[u8]> = host.split(|&c| c == b'.').collect(); if parts.len() != 4 || !parts.iter().all(|p| p.len() <= 3 && num(p, 255).is_some()) { return None; } } Some((host, port)) }"#;
    let rules = "An address is kept only when it starts with `http://` or `https://` (the scheme in either case), holds only printable ASCII other than a space and `\\` (bytes 0x21 to 0x7E: no control byte, no space, nothing outside ASCII), is at most 200 bytes long (under the 213 that the largest QR code Grimoire draws, version 10 at level M, holds) and names a plain host: up to the first `/`, `?` or `#` it is `<host>` or `<host>:<port>`, with no `user@` part, the host of letters, digits, `-`, `_` and `.` only (no `%`, no `[`), a host whose last label is a number only as four numbers from 0 to 255 without leading zeros, and a port from 1 to 65535 without a leading zero.";
    told_on_code(&mut fails, FORMAT, rules, WEBLINKS, "pub fn weblinks_address_ok(", ADDRESS_OK);
    told_on_code(&mut fails, FORMAT, rules, WEBLINKS, "fn weblinks_authority(", AUTHORITY);
    // (the title: the host as a browser writes it, held against the function that makes it)
    const HOST: &str = r#"pub fn weblinks_host(a: &str) -> String { let b = a.as_bytes(); let (rest, own) = if b.len() >= 8 && b[..8].eq_ignore_ascii_case(b"https://") { (&b[8..], 443) } else if b.len() >= 7 && b[..7].eq_ignore_ascii_case(b"http://") { (&b[7..], 80) } else { return a.to_string(); }; let end = rest.iter().position(|c| matches!(c, b'/' | b'?' | b'#')).unwrap_or(rest.len()); match weblinks_authority(&rest[..end]) { Some((host, port)) if weblinks_address_ok(b) => { let mut s = String::from_utf8_lossy(host).to_ascii_lowercase(); if let Some(p) = port.filter(|&p| p != own) { s.push_str(&format!(":{p}")); } s } _ => a.to_string(), } }"#;
    let title = "the card's title shows that host as a browser writes it: in lower case, with its port unless it is the scheme's own (80 for http, 443 for https)";
    told_on_code(&mut fails, FORMAT, title, WEBLINKS, "pub fn weblinks_host(", HOST);
    beside(&mut fails, DESIGN, "WEBLINK_ADDRESS_MAX", &grimoire::weblinks::WEBLINK_ADDRESS_MAX.to_string());
    let checked = "The worker checks the address, the writer checks it again before it is stored, and the reader checks it again.";
    told_on(&mut fails, FORMAT, checked, PDFWORK, "if w < 1 || h < 1 || !crate::weblinks::weblinks_address_ok(t[7].as_bytes()) {");
    told_on(&mut fails, FORMAT, checked, PDFWORK, "if l.src < 1 || l.src > pages || off || !crate::weblinks::weblinks_address_ok(l.address.as_bytes()) {");
    told_on(&mut fails, FORMAT, checked, WEBLINKS, "if !nav_area_valid(area) || !weblinks_address_ok(t) {");
    told_on(&mut fails, FORMAT, "a notebook without web links has no `weblinks.ini`", WEBLINKS, "if !wl.file && n == 0 && wl.unknown.is_empty() {");
    // (a file that does not read, or one with a line skipped, is never written: the writer refuses it)
    let never = "A file over a cap or one that holds a NUL does not read: it shows no web links and is never written over.";
    told_on(&mut fails, FORMAT, never, WEBLINKS, "if !wl.writable || nb.npages() < 0 || nb.npages() > STORE_MAX_PAGES as i32 {");
    told_on(&mut fails, FORMAT, never, WEBLINKS, "WebLinks { writable: false, file: true, ..Default::default() }");
    let skipped = "A line past those bounds (a later version's longer address, say) is skipped and counted in the log, the file's other links are shown, and the file is never written over";
    told_on(&mut fails, FORMAT, skipped, WEBLINKS, "if !carry_line(&mut wl.unknown, line) { wl.skipped += 1; wl.writable = false; }");
    // The card opens nothing: both sentences stand on the whole of linkcard_set_web (its one row, Close, and its
    // title, address and code: a row added fails here), on what the editor does with that row (it closes the card),
    // on the card drawing the code of the stored address, and on weblinks.rs and linkcard.rs making no process and
    // no network call (a call added to either fails here).
    const SET_WEB: &str = r#"pub fn linkcard_set_web(c: &mut LinkCard, pid: u32, seed: u32, address: &str) { let kb = std::mem::take(&mut c.kb); let boxes = std::mem::take(&mut c.boxes); *c = LinkCard { open: true, pid, page: -1, kb, boxes, ..Default::default() }; c.boxes.clear(); c.seeds.push(seed); c.web = address.to_string(); c.title = weblinks_host(address); c.rows[0] = CardRow::Close; c.enabled[0] = true; c.labels[0] = tr("Done").to_string(); c.nrows = 1; }"#;
    for (f, opens_nothing) in [
        (DESIGN, "The Kindle never opens, fetches or runs an address: the card draws the stored text and a code made from it, and nothing else."),
        (FORMAT, "Nothing on the Kindle opens, fetches or runs an address: the card shows the stored text and a code made from it."),
    ] {
        told_on_code(&mut fails, f, opens_nothing, LINKCARD, "pub fn linkcard_set_web(", SET_WEB);
        told_on(&mut fails, f, opens_nothing, EDITOR, "LcResult::Row(CardRow::Close) => card_close(a),");
        told_on(&mut fails, f, opens_nothing, LINKCARD, "c.qr = ui_qr(s, qbox, &web, negative);");
    }
    for f in [WEBLINKS, LINKCARD] {
        for call in ["std::process", "Command::new", "std::net", "TcpStream", "TcpListener", "UdpSocket", "libc::exec", "libc::fork", "posix_spawn", "libc::socket", "libc::connect"] {
            gone(&mut fails, f, call);
        }
    }
    // A template PDF's web links are never read: LINKS is the one command answered with W records, and the template
    // job never sends it (a call added there fails here).
    let templates = "the template job asks the worker for the page count and the pages only (`PROBE`, `R`), never `LINKS`, the one command whose answer holds `W` records, so a template PDF's web links are never read";
    told_on(&mut fails, DESIGN, templates, PDFWORK, r#"["LINKS", w, h] => {"#);
    gone(&mut fails, TPLPDF, ".links(");
    gone(&mut fails, TPLPDF, "LINKS");
    gone(&mut fails, FORMAT, "Web links (URI) are left out and counted in the log");
    gone(&mut fails, BACKLOG, "web links are counted and left out");
    none_failed(&fails, "web links in the documents");
}

// Send a PDF from the Share page (2.2, issue #175): the server takes a PDF from the network, as root, kept only after
// the user's yes on the Kindle. Each sentence of DESIGN.md (and KINDLE.md) that states one of its safety rules is held
// with the code that makes it true: Share alone, the Host, Origin and token checks (whole functions where they are
// functions), each header the rules read there once, no chunked body, one length and exactly that many bytes, a token
// new each time Share opens and gone when it closes, nothing looked at past the first five bytes before the yes, the
// one temporary file made afresh and removed whenever its Part drops (and when its connection closes), two senders
// keeping neither, the quiet time after a send ends (Import too), the send's code (drawn, given in its own reply alone,
// drawn on the Kindle, never logged: the code side and the tests that look at the log and the files), a question that
// ends taken away first, the name never a path (whole functions), never a rename over a file, the question answered
// only once shown and asked again after a turn, X and Esc closing Share, the network table's three rows, the log's
// lines, and the limits with their figures. A changed line, or a reworded sentence, fails here until the two are read
// again together.
#[test]
fn a_pdf_sent_from_the_share_page_is_told_as_it_is() {
    let mut fails = Vec::new();
    const SAME_TOKEN: &str = r#"pub fn same_token(given: &[u8], page: &[u8]) -> bool { if given.len() != page.len() || page.is_empty() { return false; } std::hint::black_box(given.iter().zip(page).fold(0u8, |acc, (x, y)| acc | (x ^ y))) == 0 }"#;
    const SAME_ORIGIN: &str = r#"pub fn same_origin(origin: Option<&[u8]>, host: Option<&[u8]>, site: Option<&[u8]>) -> bool { if site.is_some_and(|s| s.trim_ascii() != b"same-origin") { return false; } let Some(o) = origin.map(|o| o.trim_ascii()) else { return true }; let Some(h) = host.map(|h| h.trim_ascii()).filter(|h| !h.is_empty()) else { return false }; o.len() == 7 + h.len() && o[..7].eq_ignore_ascii_case(b"http://") && o[7..].eq_ignore_ascii_case(h) }"#;
    const HEADER_ONE: &str = r#"fn header_one(head: &[u8], name: &str, cap: usize) -> Result<Option<Vec<u8>>, ()> { match header_all(head, name).as_slice() { [] => Ok(None), [v] if v.len() < cap => Ok(Some(v.to_vec())), _ => Err(()), } }"#;
    const UPLOAD_DROP: &str = r#"fn upload_drop() { with(|s| { s.upload = Upload { serial: s.upload.serial, ..Upload::default() }; s.quiet_from = Some(now_ms()); }); }"#;
    const ASK_SYNC: &str = r#"fn ask_sync(a: &mut App) { let now = httpd_upload_question(); let shown = s(a).ask.as_ref().map(|k| (k.serial, k.rect)); match (now, shown) { (now, Some((serial, old))) if now.as_ref().map(|q| q.0) != Some(serial) => { s(a).ask = None; draw(a); app_invalidate(a, old, RefreshMode::Ui); app_toast(a, tr("Not imported."), 3000); } (Some((serial, name, size, code)), None) => { s(a).ask = Some(Ask { serial, name, size, code, ..Ask::default() }); draw(a); let r = s(a).ask.as_ref().map_or(RECT_EMPTY, |k| k.rect); app_invalidate(a, r, RefreshMode::Ui); } _ => {} } }"#;
    const SHARE_BACK: &str = r#"pub fn share_back(a: &mut App) -> bool { if a.scr != ScreenId::Share { return false; } if s(a).ask.is_some() { ask_answer(a, false); } if s(a).wifi_open { s(a).wifi_open = false; if !s(a).started { start(a); } draw(a); app_invalidate_all(a, RefreshMode::Ui); return true; } let ret = s(a).ret; app_switch( a, if ret == ScreenId::Share { ScreenId::Library } else { ret }, ); true }"#;
    const NEW_TOKEN: &str = r#"pub fn new_token() -> Option<String> { let mut b = [0u8; 16]; crate::util::random_fill(&mut b).then(|| b.iter().map(|x| format!("{x:02x}")).collect()) }"#;
    const NEW_CODE: &str = r#"pub fn new_code() -> Option<String> { loop { let mut b = [0u8; 2]; if !crate::util::random_fill(&mut b) { return None; } let v = u16::from_le_bytes(b); if v < 60_000 { return Some(format!("{:04}", v % 10_000)); } } }"#;
    const BODY_LENGTH: &str = r#"pub fn body_length(v: Option<&[u8]>) -> Option<u64> { let v = v?.trim_ascii(); if v.is_empty() || v.len() > 12 || !v.iter().all(u8::is_ascii_digit) { return None; } std::str::from_utf8(v).ok()?.parse().ok() }"#;
    const PART_CREATE: &str = r#"pub fn create() -> Result<Part, i32> { let root = root_dir()?; let dir = sub_dir(root.as_raw_fd(), ".cache", true)?; let name = part_cname(); unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) }; let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600 as libc::c_uint) }; if fd < 0 { return Err(errno()); } let file = unsafe { std::fs::File::from_raw_fd(fd) }; Ok(Part { dir, file: Some(file), kept: false }) }"#;
    const SUB_DIR: &str = r#"fn sub_dir(dirfd: i32, name: &str, make: bool) -> Result<OwnedFd, i32> { let c = CString::new(name).map_err(|_| libc::EINVAL)?; for _ in 0..2 { let fd = unsafe { libc::openat(dirfd, c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) }; if fd >= 0 { return Ok(unsafe { OwnedFd::from_raw_fd(fd) }); } let e = errno(); if e != libc::ENOENT || !make { return Err(e); } unsafe { libc::mkdirat(dirfd, c.as_ptr(), 0o755) }; } Err(libc::ENOENT) }"#;
    const UPLOAD_NAME: &str = r#"pub fn upload_name(raw: Option<&[u8]>) -> String { raw.and_then(percent_decoded).and_then(|n| cleaned(&n)).unwrap_or_else(|| UPLOAD_FALLBACK.to_string()) }"#;
    const CLEANED: &str = r#"fn cleaned(n: &str) -> Option<String> { let mapped: String = n .chars() .filter(|&c| !unseen(c)) .map(|c| match c { '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{2800}' | '\u{3000}' => ' ', '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\u{fdd0}'..='\u{fdef}' | '\u{fffd}'..='\u{ffff}' => '_', c if c.is_control() || matches!(c as u32, 0xe000..=0xf8ff | 0xf0000..=0x10ffff) || (c as u32 & 0xfffe) == 0xfffe => '_', c => c, }) .collect(); let name = mapped.trim_end_matches(|c| c == ' ' || c == '.'); let pdf = name.len() >= 4 && name.as_bytes()[name.len() - 4..].eq_ignore_ascii_case(b".pdf"); let (stem, ext) = if pdf { name.split_at(name.len() - 4) } else { (name, ".pdf") }; let mut stem = stem.trim_start_matches(|c| c == ' ' || c == '.').to_string(); while stem.len() + ext.len() > FS_NAME_MAX - 3 { stem.pop(); } let name = format!("{stem}{ext}"); (!stem.is_empty() && fsview_name_check(name.as_bytes(), name.as_bytes(), false).is_ok()).then_some(name) }"#;
    const UNSEEN: &str = r#"fn unseen(c: char) -> bool { matches!(c as u32, 0xad | 0x34f | 0x600..=0x605 | 0x61c | 0x6dd | 0x70f | 0x890..=0x891 | 0x8e2 | 0x115f..=0x1160 | 0x17b4..=0x17b5 | 0x180b..=0x180f | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x206f | 0x3164 | 0xfe00..=0xfe0f | 0xfeff | 0xffa0 | 0xfff0..=0xfffb | 0x110bd | 0x110cd | 0x13430..=0x1343f | 0x1bca0..=0x1bca3 | 0x1d173..=0x1d17a | 0xe0000..=0xe0fff) }"#;
    // who may send: Share open, the page's origin, the page's token (new each time Share opens)
    told_on(&mut fails, DESIGN, "the Share screen keeps the server (403", HTTPD, r#"return upload_refuse(c, 403, "share is closed", "Share is closed");"#);
    told_on_code(&mut fails, DESIGN, "Sec-Fetch-Site, when there is one, is `same-origin` (`same_origin`, 403)", UPLOAD, "pub fn same_origin(", SAME_ORIGIN);
    told_on_code(&mut fails, DESIGN, "compared in constant time (`same_token`), and never logged or stored", UPLOAD, "pub fn same_token(", SAME_TOKEN);
    told_on(&mut fails, DESIGN, "compared in constant time (`same_token`), and never logged or stored", HTTPD, "Ok(Some(t)) if same_token(&t, page.as_bytes()) => {}");
    told_on(&mut fails, DESIGN, "new each time Share opens (`httpd_acquire`)", HTTPD, "let token = crate::upload::new_token();");
    // before the yes: the first five bytes, one temporary file made afresh, removed on every way out
    told_on(&mut fails, DESIGN, "Of the body only its first five bytes are looked at before the user's yes", HTTPD, "if !PDF_MAGIC.starts_with(&held) {");
    told_on(&mut fails, DESIGN, "then O_CREAT|O_EXCL|O_NOFOLLOW, mode 0600", UPLOAD, "libc::openat(dir.as_raw_fd(), name.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600 as libc::c_uint)");
    let removed = "The temporary file is removed whenever its `Part` is dropped without `commit`";
    told_on(&mut fails, DESIGN, removed, UPLOAD, "if !self.kept {");
    told_on(&mut fails, DESIGN, removed, HTTPD, "if s.upload.st != UploadState::None && s.upload.client == i {");
    told_on(&mut fails, DESIGN, removed, SHARE, "httpd_upload_discard(UploadEnd::Asleep);");
    told_on(&mut fails, DESIGN, "a second upload while one is under way ends both", HTTPD, "upload_end(UploadEnd::Another);");
    // the name: shown and the file's name in import/, never a path; the file never renamed over another
    told_on_code(&mut fails, DESIGN, "**The name is never a path.**", UPLOAD, "pub fn upload_name(", UPLOAD_NAME);
    told_on_code(&mut fails, DESIGN, "**The name is never a path.**", UPLOAD, "fn cleaned(", CLEANED);
    told_on_code(&mut fails, DESIGN, "what draws as nothing or turns text goes", UPLOAD, "fn unseen(", UNSEEN);
    told_on(&mut fails, DESIGN, "**never over a file** (fsview's `rename_noreplace`", UPLOAD, "match rename_noreplace(self.dir.as_raw_fd(), &part, import.as_raw_fd(), &c) {");
    // the question: answered only once it has been shown
    told_on(&mut fails, DESIGN, "it answers only a tap that comes down after its frame was shown", SHARE, "k.swallow = !seen;");
    // the log: what came and how it ended, never the name or the token
    told_on(&mut fails, DESIGN, "never the name, the token, the send's code or a header", HTTPD, r#"qlog!("httpd: an upload was refused (%s)", why);"#);
    told_on(&mut fails, KINDLE, "Never the file's name, the page's token, the send's code or a header.", HTTPD, r#"qlog!("httpd: an upload was not kept (%s)", why);"#);
    // the limit, with its figure
    beside(&mut fails, DESIGN, "`UPLOAD_MAX`", &format!("{} MB", grimoire::upload::UPLOAD_MAX >> 20));

    // who may send: the Host check, each header the rules read there once and readable, no chunked body, one length
    told_on(&mut fails, DESIGN, "the Host header names the Kindle by address (`host_ok`, 400)", HTTPD, "if !host_ok(head) {");
    let once = "one there twice or too long is refused, never read as absent";
    told_on_code(&mut fails, DESIGN, once, HTTPD, "fn header_one(", HEADER_ONE);
    told_on(&mut fails, DESIGN, once, HTTPD, r#"let Ok(host) = one("Host", 128) else {"#);
    told_on(&mut fails, DESIGN, once, HTTPD, r#"let (Ok(origin), Ok(site)) = (one("Origin", 256), one("Sec-Fetch-Site", 32)) else {"#);
    told_on(&mut fails, DESIGN, once, HTTPD, r#"match one("X-Grimoire-Token", 64) {"#);
    told_on(&mut fails, DESIGN, once, HTTPD, r#"let Ok(length) = one("Content-Length", 32) else {"#);
    told_on(&mut fails, DESIGN, "or with any `Transfer-Encoding` line: no chunked body", HTTPD, r#"if !header_all(head, "Transfer-Encoding").is_empty() {"#);
    told_on_code(&mut fails, DESIGN, "One Content-Length of digits only", UPLOAD, "pub fn body_length(", BODY_LENGTH);
    told_on(&mut fails, DESIGN, "at least five bytes (415)", HTTPD, "if len < PDF_MAGIC.len() as u64 {");
    let exact = "A connection sends exactly the length it said";
    told_on(&mut fails, DESIGN, exact, HTTPD, "if (raw.len() - hl) as u64 > len {");
    told_on(&mut fails, DESIGN, exact, HTTPD, "if r > 0 { return upload_end(UploadEnd::TooLong); }");
    told_on(&mut fails, DESIGN, "Headers are looked for in the head alone, never in the body", HTTPD, "let head = &raw[..hl];");
    told_on(&mut fails, DESIGN, "which the server refuses (OPTIONS answers 405)", HTTPD, r#"return respond_text(405, "GET only");"#);
    told_on(&mut fails, DESIGN, "Room for it above the storage's floor, 100 MB left after it (`fsview_room_for`, 507)", HTTPD, "if !free_bytes().is_some_and(|free| fsview_room_for(free, len)) {");
    // the token: 128 random bits, gone when Share closes
    told_on_code(&mut fails, DESIGN, "The token is 128 random bits", UPLOAD, "pub fn new_token(", NEW_TOKEN);
    told_on(&mut fails, DESIGN, "gone when it closes", HTTPD, "upload_end(UploadEnd::ShareClosed); with(|s| s.token = None);");
    // the quiet time after a send ends, however it ends (Import too), with its figure
    let quiet = "**After an upload ends, however it ends, no new one is taken for `UPLOAD_QUIET_MS`**";
    told_on(&mut fails, DESIGN, quiet, HTTPD, r#"if with(|s| s.quiet_from).is_some_and(|t| ms_since(now_ms(), t) < limit_ms("QUIET", UPLOAD_QUIET_MS) as i32) {"#);
    told_on_code(&mut fails, DESIGN, quiet, HTTPD, "fn upload_drop(", UPLOAD_DROP);
    told_on(&mut fails, DESIGN, quiet, HTTPD, "if s.upload.st != UploadState::None && s.upload.client == i { s.upload = Upload { serial: s.upload.serial, ..Upload::default() }; s.quiet_from = Some(now_ms());");
    told_on(&mut fails, DESIGN, quiet, HTTPD, "let taken = (s.upload.part.take(), std::mem::take(&mut s.upload.name)); s.upload = Upload { serial: s.upload.serial, ..Upload::default() }; s.quiet_from = Some(now_ms()); taken });");
    beside(&mut fails, DESIGN, "`UPLOAD_QUIET_MS`", &format!("{} s", grimoire::upload::UPLOAD_QUIET_MS / 1000));
    gone(&mut fails, DESIGN, "ends any way but Import");
    gone(&mut fails, DESIGN, "before the next send after one not imported");
    // the other time limits, with their figures
    beside(&mut fails, DESIGN, "`UPLOAD_STALL_MS`", &format!("{} s", grimoire::upload::UPLOAD_STALL_MS / 1000));
    beside(&mut fails, DESIGN, "`UPLOAD_BODY_MS`", &format!("{} minutes", grimoire::upload::UPLOAD_BODY_MS / 60_000));
    beside(&mut fails, DESIGN, "`UPLOAD_ASK_MS`", &format!("{} minutes", grimoire::upload::UPLOAD_ASK_MS / 60_000));
    beside(&mut fails, DESIGN, "`UPLOAD_DRAIN_MS`", &format!("{} s", grimoire::upload::UPLOAD_DRAIN_MS / 1000));
    // the send's code: drawn from the system's randomness, in its own reply alone, drawn on the Kindle, never logged
    // (no log line in these files names it) and never written or shown elsewhere (the request test looks)
    told_on_code(&mut fails, DESIGN, "drawn from the system's randomness (`upload::new_code`, each of the 10,000 as likely)", UPLOAD, "pub fn new_code(", NEW_CODE);
    let alone = "It is given in that send's own reply alone, as its first line";
    told_on(&mut fails, DESIGN, alone, HTTPD, r#"buf_printf!(&mut o, "code %s\n", code);"#);
    told_on(&mut fails, DESIGN, alone, HTTPD, "upload_reply_open(c, &code);");
    told_on(&mut fails, DESIGN, "the Kindle shows the same under its question, half again the title's size", SHARE, "font_draw_centered(c, FontFace::Bold, ui_title_size() * 1.5, Rect::new(inner.x, inner.y, inner.w, ch - gap), &code, 0);");
    let never = "**The code is never logged, never written to the storage, never in another connection's answer or in a page, and it is gone with the send.**";
    told_on(&mut fails, DESIGN, never, UPLOAD_TESTS, r#"check!(!httpd.iter().any(|l| whole(format!(" {l} ").as_bytes(), code)));"#);
    told_on(&mut fails, DESIGN, never, UPLOAD_TESTS, r#"check!(!files.iter().any(|b| whole(&[b" ", &b[..], b" "].concat(), code)));"#);
    told_on(&mut fails, DESIGN, never, UPLOAD_TESTS, r#"check!(!t.contains(&format!("code {code}")) && !t.contains(&format!("Code {code}")));"#);
    told_on_code(&mut fails, DESIGN, never, HTTPD, "fn upload_drop(", UPLOAD_DROP);
    for f in [HTTPD, SHARE, UPLOAD] {
        for line in f.1.lines().filter(|l| l.contains("qlog!")) {
            if ["&code", ".code", ", code", "(code", "code_line"].iter().any(|w| line.contains(w)) {
                fails.push(format!("{} logs the send's code: {}", f.0, line.trim()));
            }
        }
    }
    // the question: a turn asks again, a tap lifts on its row, one that ends is taken away first, X and Esc close Share
    told_on(&mut fails, DESIGN, "a turn draws it at its new place and asks again", SHARE, "if rect != was { app_ask(a); }");
    told_on(&mut fails, DESIGN, "and lifts on the row it came down on", SHARE, "if swallow || at.is_none() || at != down {");
    told_on_code(&mut fails, DESIGN, "is taken away first, with the toast \"Not imported.\", and only then can another be drawn", SHARE, "fn ask_sync(", ASK_SYNC);
    let x = "**Share's X, outside the sheet, is its Cancel and closes Share**";
    told_on(&mut fails, DESIGN, x, SHARE, "let on_x = on_x && !rect_contains(k.rect, ev.x as i32, ev.y as i32);");
    told_on(&mut fails, DESIGN, x, SHARE, "if on_x && down.is_none() {");
    told_on(&mut fails, DESIGN, x, SHARE, "ask_answer(a, false); let back = back_screen(a); return app_switch(a, back);");
    told_on_code(&mut fails, DESIGN, "and so is Esc, which closes Share as it does without a question", SHARE, "pub fn share_back(", SHARE_BACK);
    // the temporary file: made afresh in .cache, never through a link, never under a name the scan takes; a crash's goes
    told_on_code(&mut fails, DESIGN, "what is at its name is removed first (the entry, never what it points at)", UPLOAD, "pub fn create(", PART_CREATE);
    told_on_code(&mut fails, DESIGN, "in a `.cache` reached from Grimoire's folder with O_NOFOLLOW", UPLOAD, "fn sub_dir(", SUB_DIR);
    told_on(&mut fails, DESIGN, "never under a name the import's scan takes", UPLOAD, r#"pub const PART_NAME: &str = "share-upload.part";"#);
    told_on(&mut fails, DESIGN, "one a crash left is removed at the next start (`upload_sweep`, from `share_init`)", SHARE, "if crate::upload::upload_sweep() {");
    // the network table: each row's limits and rules
    let pages = "Host must name the Kindle by address; 16 connections, a 4096-byte request, 30 s idle; renders forked, 2 at a time, others queued 2 minutes at most";
    told_on(&mut fails, DESIGN, pages, HTTPD, "if !host_ok(&req) {");
    told_on(&mut fails, DESIGN, pages, HTTPD, "const MAX_CLIENTS: usize = 16;");
    told_on(&mut fails, DESIGN, pages, HTTPD, "req: [u8; 4096],");
    told_on(&mut fails, DESIGN, pages, HTTPD, "if ms_since(now, with(|s| s.cl[c].t0)) > 30000 { return client_close(c); // said nothing (more) for 30 s");
    told_on(&mut fails, DESIGN, pages, HTTPD, "const MAX_CHILDREN: usize = 2;");
    told_on(&mut fails, DESIGN, pages, HTTPD, "if st == ClientState::Queued && ms_since(now, t0) > 120000 {");
    let live = "sessions in an HttpOnly SameSite=Strict cookie; the key POST is bodyless, its event in a custom header, its preflight refused";
    told_on(&mut fails, DESIGN, live, PAIR, r#""%s=%s; HttpOnly; SameSite=Strict; Path=/""#);
    told_on(&mut fails, DESIGN, live, HTTPD, r#"return respond_text(400, "no body allowed");"#);
    told_on(&mut fails, DESIGN, live, HTTPD, r#"let Some(event) = header_get(req, "X-Grimoire-Key", 32) else {"#);
    told_on(&mut fails, DESIGN, live, HTTPD, r#"return respond_text(405, "GET only");"#);
    let send = "one send at a time, 64 MB at most, one Content-Length and each header the rules read once; the body only into one temporary file; the Kindle asks with the send's code, which only the sending page has; nothing kept without Import on the Kindle; 5 s before the next send after one ends";
    told_on(&mut fails, DESIGN, send, HTTPD, "upload_end(UploadEnd::Another);");
    told_on(&mut fails, DESIGN, send, HTTPD, "if len > UPLOAD_MAX {");
    told_on_code(&mut fails, DESIGN, send, HTTPD, "fn header_one(", HEADER_ONE);
    told_on_code(&mut fails, DESIGN, send, UPLOAD, "pub fn create(", PART_CREATE);
    told_on(&mut fails, DESIGN, send, HTTPD, "upload_reply_open(c, &code);");
    told_on(&mut fails, DESIGN, send, HTTPD, "let r = part.map_or(Err(libc::EBADF), |p| p.commit(&name));");
    told_on(&mut fails, DESIGN, send, HTTPD, r#"if with(|s| s.quiet_from).is_some_and(|t| ms_since(now_ms(), t) < limit_ms("QUIET", UPLOAD_QUIET_MS) as i32) {"#);
    beside(&mut fails, DESIGN, "before the next send after one ends", &format!("{} s", grimoire::upload::UPLOAD_QUIET_MS / 1000));
    none_failed(&fails, "a PDF sent from the Share page");
}
