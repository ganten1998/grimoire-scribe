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
const BACKLOG: File = ("docs/BACKLOG.md", include_str!("../../docs/BACKLOG.md"));
const PDFWORK: File = ("rust/src/pdfwork.rs", include_str!("../src/pdfwork.rs"));
const FONTCHECK: File = ("rust/src/fontcheck.rs", include_str!("../src/fontcheck.rs"));
const LICENSES_TOOL: File = ("tools/crate_licenses.py", include_str!("../../tools/crate_licenses.py"));
const CARGO_TOML: File = ("rust/Cargo.toml", include_str!("../Cargo.toml"));
const CARGO_LOCK: File = ("rust/Cargo.lock", include_str!("../Cargo.lock"));
const DENY_TOML: File = ("rust/deny.toml", include_str!("../deny.toml"));

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

/// A present pin: the file names what the code has.
fn present(fails: &mut Vec<String>, f: File, name: &str) {
    if !flat(f.1).contains(name) {
        fails.push(format!("{} does not name {name}", f.0));
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

// Sentences that said what the code does not do, each gone from the file that held it.
#[test]
fn the_docs_no_longer_say_what_the_code_does_not() {
    let mut fails = Vec::new();
    // the crates: glifo is the optional one (vello_cpu's), and skrifa is glifo's plain dependency
    gone(&mut fails, BACKLOG, "glifo's optional dependency");
    // licenses/rust-crates.txt lists every crate the lock names, not only the ones built in
    gone(&mut fails, LICENSES_TOOL, "the Grimoire binary is built from (cargo's Cargo.lock decides them");
    none_failed(&fails, "the documents and comments");
}
