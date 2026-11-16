// test_sec_invariants.rs (hardening round 2026-11): what the documents and the build rules say of the safe loader,
// held against the code and Cargo.lock.
//
// Every test here reads files of the tree as text (include_str!): no process, no environment variable, no clock.
// A "gone" pin is a phrase a file must no longer hold: a sentence that said what the code does not do. It catches
// the old words coming back (a merge settled the wrong way), not the old meaning in new words. A "present" pin is
// a name the code itself has (a constant, a file's name), never prose, so a reworded sentence does not fail here.
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
const SANDBOX: File = ("rust/src/sandbox.rs", include_str!("../src/sandbox.rs"));
const TEMPLATES: File = ("rust/src/bgwork_template.rs", include_str!("../src/bgwork_template.rs"));
const SANDBOX_TESTS: File = ("rust/tests/test_sandbox_linux.rs", include_str!("test_sandbox_linux.rs"));
const LICENSES_TOOL: File = ("tools/crate_licenses.py", include_str!("../../tools/crate_licenses.py"));
const CARGO_TOML: File = ("rust/Cargo.toml", include_str!("../Cargo.toml"));
const CARGO_LOCK: File = ("rust/Cargo.lock", include_str!("../Cargo.lock"));
const DENY_TOML: File = ("rust/deny.toml", include_str!("../deny.toml"));

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
// its figure beside it. The own folder's place and the floors of its free room are placeholders until a
// Scribe's figures are in: while the documents give them, userfonts.rs must still hold them, so the day one
// is changed the sentence is read again and follows.
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
    none_failed(&fails, "Files' rename, delete and restore");
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
