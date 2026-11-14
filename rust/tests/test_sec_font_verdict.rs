// test_sec_font_verdict.rs (hardening round 2026-11): where what the background check found of a font is kept.
//
// The rule: where the build has a folder of its own outside the data root (the Kindle's is a constant; any other
// build is given one with GRIMOIRE_STATE, as these tests do), a verdict is read only from Grimoire's own folder,
// by descriptor, and only while every folder on the way is this user's and no one else can write it. The cache
// (grimoire/fonts/.cache) keeps the previews and the marks; a verdict counts only with its companion there, so
// deleting the cache still has every font checked again. A build that names no such folder keeps its verdicts in
// the cache as before, with no test of owners or modes.
//
// Every font goes through a real worker (a fresh exec of the headless runner, GRIMOIRE_WORK_EXE) and the real
// job, in this process or forked by a scan as the app forks it; two cases start the app itself. One #[test], its
// cases in order: they change the environment and the store's root, which are the process's.
#![cfg(feature = "sim")]
mod fontfix;
mod testlib;
use fontfix::*;
use grimoire::fontcheck::*;
use grimoire::prelude::*;
use grimoire::sha256::{hex32, sha256};
use grimoire::userfonts::*;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use testlib::*;

const BIN: &str = env!("CARGO_BIN_EXE_grimoire-headless");
const NO_WORKER: &str = "/nonexistent/grimoire";

const HOOKS: [&str; 13] = [
    "GRIMOIRE_FONT_TEST_CRASH",
    "GRIMOIRE_FONT_TEST_HANG",
    "GRIMOIRE_FONT_TEST_TRAP",
    "GRIMOIRE_FONT_WATCHDOG_MS",
    "GRIMOIRE_FONT_SLICE_MS",
    "GRIMOIRE_FONT_JOB_MS",
    "GRIMOIRE_FONT_MEM_AVAILABLE",
    "GRIMOIRE_FONT_GLYPH_US",
    "GRIMOIRE_FONT_TEST_SLOW",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_STATE",
    "GRIMOIRE_STATE_FREE_BYTES",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
}

/// A fresh data root with an empty fonts folder; the store looks at it from now on.
fn root(name: &str) -> String {
    let d = format!("{}/fontverdict-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(format!("{d}/fonts")).unwrap();
    store_init(&d);
    d
}

/// A path for Grimoire's own folder, outside every data root, with nothing at it yet (Grimoire makes the
/// folder, so its mode is Grimoire's and not this runner's umask's); GRIMOIRE_STATE names it from now on.
fn own_folder(name: &str) -> String {
    let d = format!("{}/fontverdict-{}-{name}.state", tl_tmp(), std::process::id());
    remove_any(&d);
    remove_any(&format!("{d}.real"));
    std::env::set_var("GRIMOIRE_STATE", &d);
    d
}

/// Removes whatever is at a path: a folder with what it holds, a file, or a link (never what it names).
fn remove_any(p: &str) {
    match std::fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => {
            // (a case that stopped half way may have left a folder without its owner's bits)
            let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700));
            let _ = std::fs::remove_dir_all(p);
        }
        Ok(_) => {
            let _ = std::fs::remove_file(p);
        }
        Err(_) => {}
    }
}

fn put(root: &str, rel: &str, bytes: &[u8]) {
    std::fs::write(format!("{root}/fonts/{rel}"), bytes).unwrap();
}

/// The job in this process, as the background child runs it: one font a job, until none is left (5: more).
fn job() -> i32 {
    loop {
        let rc = bg_font_check("", 300.0);
        if rc != 5 {
            return rc;
        }
    }
}

/// Waits for the FontCheck job a scan or a load queued (if one runs): its exit status, or 0 when none ran.
fn wait_job() -> i32 {
    let t0 = std::time::Instant::now();
    let mut code = 0;
    while bg_busy() {
        let mut kind = -1;
        bg_poll_kind(Some(&mut kind));
        if kind == BgKind::FontCheck as i32 {
            code = bg_last_exit();
            if code == 5 {
                bg_queue(BgKind::FontCheck, Some(""), Some("next"), 300.0, false);
            }
        }
        assert!(t0.elapsed().as_secs() < 120, "the font check did not end");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    code
}

/// The list as the Font sheet has it once the check a look queued has ended.
fn listed() -> Vec<UserFont> {
    let mut l = userfonts_scan(300.0);
    wait_job();
    userfonts_refresh(&mut l);
    l
}

fn state_of(rel: &str) -> &'static str {
    let l = listed();
    let mut f = l.iter().find(|f| f.rel == rel).unwrap_or_else(|| panic!("{rel} is not listed")).clone();
    userfont_resolve(&mut f);
    // (a verdict that did not read queues its check from the row: wait for that too)
    wait_job();
    f.state_name()
}

/// A font's key, by the list.
fn key_of(rel: &str) -> u64 {
    listed().iter().find(|f| f.rel == rel).unwrap_or_else(|| panic!("{rel} is not listed")).key
}

/// Whether the font loads as the UI loads it; a load that queued a check has that check's end waited for.
fn loads(rel: &str) -> Result<&'static str, LoadErr> {
    let r = userfonts_try_load(rel, "");
    wait_job();
    r
}

/// FNV-1a 64, as a data root's folder in Grimoire's own is named (the root's path) and as 2.1 hashed a file.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(14695981039346656037u64, |h, &b| (h ^ b as u64).wrapping_mul(1099511628211))
}

/// The folder of a data root in Grimoire's own folder.
fn tag_dir(state: &str, root: &str) -> String {
    format!("{state}/{:016x}", fnv(root.as_bytes()))
}

/// The names in a folder, sorted (none when it is not there).
fn names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn with_ext(dir: &str, ext: &str) -> Vec<String> {
    names(dir).into_iter().filter(|n| n.ends_with(ext)).collect()
}

fn mode_of(p: &str) -> u32 {
    std::fs::symlink_metadata(p).map(|m| m.permissions().mode() & 0o7777).unwrap_or(0)
}

fn chmod(p: &str, mode: u32) {
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// The text of this version's verdict for a font that passes, as the job writes it.
fn verdict_text(font: &[u8]) -> String {
    let facts = facts_in_process(font).expect("the font reads");
    Verdict { hash: font_hash(font), size: font.len() as u64, what: VerdictKind::Passed(facts) }.to_text()
}

/// The text 2.1 wrote for the same font: its first line and its hash line were other ones.
fn verdict_text_2_1(font: &[u8]) -> String {
    let now = verdict_text(font);
    let rest: Vec<&str> = now.splitn(3, '\n').collect();
    format!("grimoire-font 1\nhash={:016x}\n{}", fnv(font), rest[2])
}

/// A small picture file, as a preview is one.
fn pgm() -> Vec<u8> {
    let mut v = b"P5\n8 8\n255\n".to_vec();
    v.extend([255u8; 64]);
    v
}

/// One start of the app on `root` running `script`, with `env` on top of a clean GRIMOIRE_ environment: its
/// exit code and what it printed.
fn start(root: &str, script: &str, env: &[(&str, &str)]) -> (Option<i32>, String) {
    let path = format!("{root}/script.qs");
    std::fs::write(&path, script).unwrap();
    let mut c = Command::new(BIN);
    for (k, _) in std::env::vars() {
        if k.starts_with("GRIMOIRE_") {
            c.env_remove(k);
        }
    }
    c.env("GRIMOIRE_ROOT", root).env("GRIMOIRE_SCRIPT", &path).env("GRIMOIRE_LANG", "en").env("GRIMOIRE_IMPORT_STABLE", "fast");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().unwrap();
    (out.status.code(), format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
}

const OWN: &str = "# Grimoire settings\ntheme=classic\nfont=file:Mine.ttf\nwhatsnew_seen=9.9.9\n";

// A verdict is read only from Grimoire's own folder: a `.font` file in the cache, with its preview beside it,
// is not one, whether it is the text an earlier version wrote there or this version's own text for the very
// file. No check can produce anything here (no worker starts), so the row stays "checking" throughout.
fn only_grimoires_own_folder_is_read() {
    clean_env();
    let r = root("ownonly");
    let _state = own_folder("ownonly");
    let font = ttf(&test_font("Regular", 400));
    put(&r, "Mine.ttf", &font);
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    let key = key_of("Mine.ttf");
    let cache = format!("{r}/fonts/.cache");
    check!(names(&cache).is_empty());
    std::fs::write(format!("{cache}/{key:016x}.pgm"), pgm()).unwrap();
    for text in [verdict_text_2_1(&font), verdict_text(&font)] {
        check!(text.lines().nth(1).is_some_and(|l| l.starts_with("hash=")));
        std::fs::write(format!("{cache}/{key:016x}.font"), &text).unwrap();
        check!(loads("Mine.ttf") == Err(LoadErr::Checking));
        check!(state_of("Mine.ttf") == "checking");
        // (and the file in the cache is as it was put there)
        check!(std::fs::read_to_string(format!("{cache}/{key:016x}.font")).is_ok_and(|t| t == text));
    }
    // (the text is one this version reads: the hash is the file's SHA-256, which is what the job would write)
    check!(Verdict::parse(verdict_text(&font).as_bytes()).is_some_and(|v| v.hash == sha256(&font)));
}

// After a check the cache holds the preview and no verdict; Grimoire's own folder holds the verdict, in a folder
// of this data root, under folders only this user can enter, and names the data root it is of.
fn the_verdict_is_kept_in_grimoires_own_folder() {
    clean_env();
    let r = root("kept");
    let state = own_folder("kept");
    let font = ttf(&test_font("Regular", 400));
    put(&r, "Mine.ttf", &font);
    check!(job() == 0);
    let key = key_of("Mine.ttf");
    let cache = format!("{r}/fonts/.cache");
    check!(names(&cache) == vec![format!("{key:016x}.pgm")]);
    let tag = tag_dir(&state, &r);
    check!(names(&state) == vec![format!("{:016x}", fnv(r.as_bytes()))]);
    check!(names(&tag) == vec!["fonts".to_string(), "root".to_string()]);
    check!(names(&format!("{tag}/fonts")) == vec![format!("{key:016x}.font")]);
    let text = std::fs::read_to_string(format!("{tag}/fonts/{key:016x}.font")).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    check!(lines.first() == Some(&"grimoire-font 2"));
    check!(lines.get(1) == Some(&format!("hash={}", hex32(&sha256(&font))).as_str()));
    check!(lines.get(2) == Some(&format!("size={}", font.len()).as_str()));
    for dir in [state.clone(), tag.clone(), format!("{tag}/fonts")] {
        check!(mode_of(&dir) == 0o700);
    }
    check!(mode_of(&format!("{tag}/fonts/{key:016x}.font")) == 0o600);
    check!(std::fs::read_to_string(format!("{tag}/root")).is_ok_and(|t| t == format!("{r}\n")));
    check!(loads("Mine.ttf") == Ok("overprint"));
    check!(state_of("Mine.ttf") == "ready");
}

// A font is checked once where its verdict is kept: later looks need no worker. The same data root beside
// another own folder (another device) has no verdict there: the font is checked once more, then is ready. And
// the app itself, started on the data root with its own folder, draws with the font on its first screen.
fn checked_once_on_this_device_and_again_elsewhere() {
    clean_env();
    let r = root("once");
    let _first = own_folder("once");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    check!(loads("Mine.ttf") == Ok("overprint"));
    check!(state_of("Mine.ttf") == "ready");
    let second = own_folder("once-elsewhere");
    check!(loads("Mine.ttf") == Err(LoadErr::Checking));
    check!(state_of("Mine.ttf") == "checking");
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    check!(state_of("Mine.ttf") == "ready");
    check!(loads("Mine.ttf") == Ok("overprint"));
    // the app: the same root string (the folder of a data root is named by it), no worker to be had, no wait
    std::fs::write(format!("{r}/settings.ini"), OWN).unwrap();
    let script = "wait 100\nexpect font is file:Mine.ttf\nexpect settings.font is file:Mine.ttf\nexpect font.own.drawn is 1\nquit\n";
    let (code, out) = start(&r, script, &[("GRIMOIRE_STATE", &second), ("GRIMOIRE_WORK_EXE", NO_WORKER)]);
    check!(code == Some(0));
    check!(!out.contains("bg: font check of"));
    if code != Some(0) {
        eprintln!("{out}");
    }
}

// A folder that is not Grimoire's own keeps nothing and gives nothing: one that others can write, one that is a
// link, one that is no folder. The genuine verdict is behind each of them, and is not read; the job ends before
// any worker starts (6) and leaves no mark and no file anywhere. A verdict file that is a link is not read
// either, and the next check puts a file of its own in the link's place.
fn a_folder_that_is_not_grimoires_own_keeps_nothing() {
    clean_env();
    let r = root("notown");
    let state = own_folder("notown");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    check!(loads("Mine.ttf") == Ok("overprint"));
    let key = key_of("Mine.ttf");
    let tag = tag_dir(&state, &r);
    let (fonts, real) = (format!("{tag}/fonts"), format!("{state}.real"));
    let verdict = format!("{fonts}/{key:016x}.font");
    let genuine = std::fs::read(&verdict).unwrap();
    let cache = format!("{r}/fonts/.cache");
    let before = names(&cache);
    // Every file under what is, or was, Grimoire's own folder, and the cache's names: nothing new may appear.
    let everything = |state: &str, real: &str| {
        let mut v = Vec::new();
        for top in [state, real] {
            for a in names(top) {
                v.push(a.clone());
                for b in names(&format!("{top}/{a}")) {
                    v.push(format!("{a}/{b}"));
                    for c in names(&format!("{top}/{a}/{b}")) {
                        v.push(format!("{a}/{b}/{c}"));
                    }
                }
            }
        }
        v.sort();
        v.dedup();
        v
    };
    let whole = everything(&state, &real);
    check!(whole.len() == 4);
    let refused = |what: &str| {
        eprintln!("    {what}");
        check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
        // (the check the load queued, as the app's would: it says what a check finds cannot be kept)
        check!(wait_job() == 6);
        check!(bg_font_check("", 300.0) == 6);
        check!(names(&cache) == before);
        check!(everything(&state, &real) == whole);
        check!(std::fs::read(&verdict).or_else(|_| std::fs::read(format!("{real}/{:016x}/fonts/{key:016x}.font", fnv(r.as_bytes())))).is_ok_and(|b| b == genuine));
    };
    chmod(&fonts, 0o777);
    refused("its fonts folder can be written by anyone");
    chmod(&fonts, 0o700);
    chmod(&tag, 0o770);
    refused("the data root's folder can be written by its group");
    chmod(&tag, 0o700);
    std::fs::rename(&state, &real).unwrap();
    std::os::unix::fs::symlink(&real, &state).unwrap();
    refused("the folder is a link to the genuine one");
    std::fs::remove_file(&state).unwrap();
    std::fs::rename(&real, &state).unwrap();
    std::fs::rename(&fonts, format!("{tag}/fonts.real")).unwrap();
    std::os::unix::fs::symlink(format!("{tag}/fonts.real"), &fonts).unwrap();
    check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
    check!(wait_job() == 6);
    check!(bg_font_check("", 300.0) == 6);
    check!(names(&cache) == before && names(&format!("{tag}/fonts.real")) == vec![format!("{key:016x}.font")]);
    std::fs::remove_file(&fonts).unwrap();
    std::fs::rename(format!("{tag}/fonts.real"), &fonts).unwrap();
    std::fs::rename(&state, &real).unwrap();
    std::fs::write(&state, b"a file where the folder was").unwrap();
    refused("a file is where the folder was");
    std::fs::remove_file(&state).unwrap();
    std::fs::rename(&real, &state).unwrap();
    // whole again: the genuine verdict is read as before
    check!(loads("Mine.ttf") == Ok("overprint"));
    check!(bg_font_check("", 300.0) == 0);
    // The verdict itself a link to a copy of it (kept outside): not read, and never written through.
    let copy = format!("{}/fontverdict-{}-notown.copy", tl_tmp(), std::process::id());
    std::fs::write(&copy, &genuine).unwrap();
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(5000);
    std::fs::File::options().write(true).open(&copy).unwrap().set_modified(old).unwrap();
    let copied_at = std::fs::metadata(&copy).unwrap().modified().unwrap();
    std::fs::remove_file(&verdict).unwrap();
    std::os::unix::fs::symlink(&copy, &verdict).unwrap();
    check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
    // (the check that load queued runs with a worker: the font is checked again)
    check!(wait_job() == 0);
    check!(std::fs::symlink_metadata(&verdict).is_ok_and(|m| m.file_type().is_file()));
    check!(std::fs::read(&verdict).is_ok_and(|b| b == genuine));
    check!(std::fs::metadata(&copy).is_ok_and(|m| m.modified().unwrap() == copied_at && m.len() == genuine.len() as u64));
    check!(names(&fonts) == vec![format!("{key:016x}.font")]);
    check!(loads("Mine.ttf") == Ok("overprint"));
    let _ = std::fs::remove_file(&copy);
}

// Two data roots beside one own folder each have a folder there: a job in one leaves the other's as it was. Once
// a data root is gone, the next job of the other removes the folder that was its.
fn two_data_roots_keep_their_verdicts_apart() {
    clean_env();
    let state = own_folder("two");
    let a = root("two-a");
    put(&a, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    let tag_a = tag_dir(&state, &a);
    let snapshot = |tag: &str| {
        let fonts = names(&format!("{tag}/fonts"));
        let texts: Vec<Vec<u8>> = fonts.iter().map(|n| std::fs::read(format!("{tag}/fonts/{n}")).unwrap_or_default()).collect();
        (names(tag), fonts, texts, std::fs::read(format!("{tag}/root")).unwrap_or_default())
    };
    let a_before = snapshot(&tag_a);
    check!(a_before.1.len() == 1 && a_before.3 == format!("{a}\n").into_bytes());
    let b = root("two-b");
    put(&b, "Other.ttf", &ttf(&test_font("Bold", 700)));
    put(&b, "Blank.ttf", &ttf(&empty_letters()));
    check!(job() == 0);
    let tag_b = tag_dir(&state, &b);
    check!(tag_a != tag_b && names(&state).len() == 2);
    check!(snapshot(&tag_a) == a_before);
    let b_before = snapshot(&tag_b);
    check!(b_before.1.len() == 2 && b_before.3 == format!("{b}\n").into_bytes());
    // a job in the second root while the first is there: both stay
    check!(job() == 0);
    check!(snapshot(&tag_a) == a_before && snapshot(&tag_b) == b_before);
    // the first root goes: its folder goes at the second's next job, and the second's is whole
    std::fs::remove_dir_all(&a).unwrap();
    check!(job() == 0);
    check!(std::fs::symlink_metadata(&tag_a).is_err());
    check!(names(&state) == vec![format!("{:016x}", fnv(b.as_bytes()))]);
    check!(snapshot(&tag_b) == b_before);
    check!(loads("Other.ttf") == Ok("overprint"));
    // (a folder there that is not a data root's, or whose root does not read, is left alone)
    std::fs::create_dir(format!("{state}/notes")).unwrap();
    std::fs::create_dir(format!("{state}/00000000000000aa")).unwrap();
    chmod(&format!("{state}/00000000000000aa"), 0o700);
    std::fs::write(format!("{state}/00000000000000aa/keep.me"), b"x").unwrap();
    std::fs::create_dir(format!("{state}/00000000000000bb")).unwrap();
    chmod(&format!("{state}/00000000000000bb"), 0o700);
    std::fs::write(format!("{state}/00000000000000bb/root"), format!("{a}\n")).unwrap();
    std::fs::write(format!("{state}/00000000000000bb/else"), b"x").unwrap();
    check!(job() == 0);
    check!(names(&state).len() == 4 && names(&format!("{state}/00000000000000bb")).len() == 2);
}

// A `.font` file an earlier version left in the cache beside a listed font is not read and not written: it is
// there byte for byte after the font's check, whose verdict went to Grimoire's own folder.
fn an_earlier_versions_verdict_is_left_alone() {
    clean_env();
    let r = root("earlier");
    let state = own_folder("earlier");
    let font = ttf(&test_font("Regular", 400));
    put(&r, "Mine.ttf", &font);
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    let key = key_of("Mine.ttf");
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    let cache = format!("{r}/fonts/.cache");
    let old = verdict_text_2_1(&font);
    std::fs::write(format!("{cache}/{key:016x}.font"), &old).unwrap();
    std::fs::write(format!("{cache}/{key:016x}.pgm"), pgm()).unwrap();
    check!(state_of("Mine.ttf") == "ready");
    check!(std::fs::read_to_string(format!("{cache}/{key:016x}.font")).is_ok_and(|t| t == old));
    // (the check ran: the preview is the check's now, and the verdict is in the own folder)
    check!(std::fs::read(format!("{cache}/{key:016x}.pgm")).is_ok_and(|b| b != pgm()));
    check!(names(&format!("{}/fonts", tag_dir(&state, &r))) == vec![format!("{key:016x}.font")]);
    check!(job() == 0);
    check!(std::fs::read_to_string(format!("{cache}/{key:016x}.font")).is_ok_and(|t| t == old));
    // it goes with its font file, as everything of a file that is gone does
    std::fs::remove_file(format!("{r}/fonts/Mine.ttf")).unwrap();
    check!(job() == 0);
    check!(names(&cache).is_empty() && names(&format!("{}/fonts", tag_dir(&state, &r))).is_empty());
}

// Deleting grimoire/fonts/.cache has every font checked again, the refused ones too: a verdict counts only with
// its companion in the cache (a pass with its preview, a refusal with its `.no`).
fn the_fonts_cache_deleted_checks_again() {
    clean_env();
    let r = root("cachegone");
    let state = own_folder("cachegone");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    put(&r, "Blank.ttf", &ttf(&empty_letters()));
    check!(job() == 0);
    let cache = format!("{r}/fonts/.cache");
    let fonts = format!("{}/fonts", tag_dir(&state, &r));
    check!(with_ext(&cache, ".pgm").len() == 1 && with_ext(&cache, ".no").len() == 1 && names(&cache).len() == 2);
    check!(with_ext(&fonts, ".font").len() == 2);
    check!(state_of("Mine.ttf") == "ready" && state_of("Blank.ttf") == "unreadable");
    std::fs::remove_dir_all(&cache).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    check!(state_of("Mine.ttf") == "checking" && state_of("Blank.ttf") == "checking");
    check!(loads("Mine.ttf") == Err(LoadErr::Checking));
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    check!(state_of("Mine.ttf") == "ready" && state_of("Blank.ttf") == "unreadable");
    check!(with_ext(&cache, ".pgm").len() == 1 && with_ext(&cache, ".no").len() == 1 && names(&cache).len() == 2);
    check!(with_ext(&fonts, ".font").len() == 2);
    // (one companion alone: only that font is checked again)
    std::fs::remove_file(format!("{cache}/{}", with_ext(&cache, ".no")[0])).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    check!(state_of("Mine.ttf") == "ready" && state_of("Blank.ttf") == "checking");
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    check!(state_of("Blank.ttf") == "unreadable");
}

// With too little room free where the verdicts are kept nothing is checked: the job ends (6) before any mark is
// written, and a font checked before still loads (reading needs no room).
fn too_little_room_checks_nothing() {
    clean_env();
    let r = root("room");
    let state = own_folder("room");
    put(&r, "A.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    let mut other = test_font("Regular", 400);
    other.family = "Another".into();
    put(&r, "B.ttf", &ttf(&other));
    let cache = format!("{r}/fonts/.cache");
    let fonts = format!("{}/fonts", tag_dir(&state, &r));
    let (cache_before, fonts_before) = (names(&cache), names(&fonts));
    std::env::set_var("GRIMOIRE_STATE_FREE_BYTES", "1000");
    check!(bg_font_check("", 300.0) == 6);
    check!(names(&cache) == cache_before && names(&fonts) == fonts_before);
    check!(loads("A.ttf") == Ok("overprint"));
    check!(userfonts_try_load("B.ttf", "") == Err(LoadErr::Checking));
    check!(wait_job() == 6);
    check!(names(&cache) == cache_before && names(&fonts) == fonts_before);
    // with all fonts checked the job needs no room at all
    std::env::remove_var("GRIMOIRE_STATE_FREE_BYTES");
    check!(job() == 0);
    check!(loads("B.ttf") == Ok("overprint"));
    std::env::set_var("GRIMOIRE_STATE_FREE_BYTES", "1000");
    check!(job() == 0);
    std::env::remove_var("GRIMOIRE_STATE_FREE_BYTES");
}

// A build that names no folder of its own keeps its verdicts in the cache, as before, and asks nothing of that
// folder's owner or mode (a platform may keep its data on a file system that has neither).
fn without_a_folder_of_its_own_the_cache_keeps_verdicts() {
    clean_env();
    let r = root("nofolder");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    let key = key_of("Mine.ttf");
    let cache = format!("{r}/fonts/.cache");
    check!(names(&cache) == vec![format!("{key:016x}.font"), format!("{key:016x}.pgm")]);
    check!(std::fs::read_to_string(format!("{cache}/{key:016x}.font")).is_ok_and(|t| t.starts_with("grimoire-font 2\nhash=")));
    check!(loads("Mine.ttf") == Ok("overprint"));
    for dir in [cache.clone(), format!("{r}/fonts"), r.clone()] {
        chmod(&dir, 0o777);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    check!(loads("Mine.ttf") == Ok("overprint"));
    check!(state_of("Mine.ttf") == "ready");
    // (a name that is no absolute path names no folder)
    std::env::set_var("GRIMOIRE_STATE", "not/absolute");
    check!(loads("Mine.ttf") == Ok("overprint"));
    std::env::remove_var("GRIMOIRE_STATE");
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    for dir in [cache, format!("{r}/fonts"), r.clone()] {
        chmod(&dir, 0o755);
    }
}

// The app, when its own folder cannot be used: it says so once, checks no font in that session and marks none;
// Terminess draws, and the Font sheet's row says the font cannot be read.
fn the_app_says_once_that_nothing_can_be_kept() {
    clean_env();
    let r = root("app");
    let state = own_folder("app");
    std::fs::write(&state, b"a file, not a folder").unwrap();
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::write(format!("{r}/settings.ini"), OWN).unwrap();
    let script = "wait 100\nwaitbg\nwait 300\nexpect font is terminess\nexpect settings.font is file:Mine.ttf\ntap 1535 95\nwait 50\ntap library font\nwait 300\n\
expect library.font.state.Mine.ttf is unreadable\nexpect library.font.checking is 0\ntap library font.cancel\nwait 50\nquit\n";
    let (code, out) = start(&r, script, &[("GRIMOIRE_STATE", &state), ("GRIMOIRE_WORK_EXE", BIN)]);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    check!(out.contains("fonts: what the check finds cannot be kept (it is not Grimoire's own); nothing is marked"));
    check!(out.matches("fonts: what the check finds cannot be kept (see the line above): no font is checked until Grimoire starts again").count() == 1);
    check!(out.lines().filter(|l| l.contains("bg: font check of")).count() == 1);
    check!(!out.contains(&state));
    check!(names(&format!("{r}/fonts/.cache")).is_empty());
    check!(std::fs::read(&state).is_ok_and(|b| b == b"a file, not a folder"));
    remove_any(&state);
}

#[test]
fn test_sec_font_verdict() {
    run!(only_grimoires_own_folder_is_read);
    run!(the_verdict_is_kept_in_grimoires_own_folder);
    run!(checked_once_on_this_device_and_again_elsewhere);
    run!(a_folder_that_is_not_grimoires_own_keeps_nothing);
    run!(two_data_roots_keep_their_verdicts_apart);
    run!(an_earlier_versions_verdict_is_left_alone);
    run!(the_fonts_cache_deleted_checks_again);
    run!(too_little_room_checks_nothing);
    run!(without_a_folder_of_its_own_the_cache_keeps_verdicts);
    run!(the_app_says_once_that_nothing_can_be_kept);
    done();
}
