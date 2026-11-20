// test_sec_font_verdict.rs (hardening round 2026-11): where what the background check found of a font is kept.
//
// The rule: where the build has a folder of its own outside the data root (the Kindle's is a constant; any other
// build is given one with GRIMOIRE_STATE, as these tests do), a verdict is read only from Grimoire's own folder,
// by descriptor, and only while every folder on the way is this user's and no one else can write it. The cache
// (grimoire/fonts/.cache) keeps the previews and the marks; a verdict counts only with its companion there, so
// deleting the cache still has every font checked again. A build that names no such folder keeps its verdicts in
// the cache as before, with no test of owners or modes.
//
// A font renamed in Files keeps its verdict, renamed to the new name's key (issue #144).
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
    // (each case is a fresh process to the move aside, which a process tries once: the app forks each job)
    userfonts_aside_reset();
    userfonts_aside_seam(None);
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

/// A folder for the parent of Grimoire's own folder, as /var/local is on the Kindle, with nothing in it;
/// GRIMOIRE_STATE names `<parent>/grimoire` from now on. The parent and that path.
fn own_parent(name: &str) -> (String, String) {
    let p = format!("{}/fontverdict-{}-{name}.parent", tl_tmp(), std::process::id());
    remove_any(&p);
    std::fs::create_dir(&p).unwrap();
    chmod(&p, 0o755);
    let state = format!("{p}/grimoire");
    std::env::set_var("GRIMOIRE_STATE", &state);
    (p, state)
}

/// An entry as it stands, looked at without following it, and what it holds where that is read without opening a
/// pipe: its inode, its mode (with its kind), a link's target, a file's bytes, a folder's names with each one's
/// bytes. None when nothing is there.
type Entry = (u64, u32, Option<std::path::PathBuf>, Vec<(String, Vec<u8>)>);
fn entry(p: &str) -> Option<Entry> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::symlink_metadata(p).ok()?;
    let t = m.file_type();
    let held = if t.is_dir() {
        names(p).into_iter().map(|n| (n.clone(), std::fs::read(format!("{p}/{n}")).unwrap_or_default())).collect()
    } else if t.is_file() {
        vec![(String::new(), std::fs::read(p).unwrap_or_default())]
    } else {
        Vec::new()
    };
    Some((m.ino(), m.mode(), t.is_symlink().then(|| std::fs::read_link(p).unwrap()), held))
}

/// The names in a parent of entries moved aside there.
fn asides(parent: &str) -> Vec<String> {
    names(parent).into_iter().filter(|n| n.starts_with("grimoire.not-own-")).collect()
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

/// The job in this process, as the background child runs it: one font a job, until none is left (5: more). The
/// first is given leave to make Grimoire's own folder, or make it afresh, as the first check of a run is
/// (USERFONTS_AFRESH); the ones after it are not, as the app's next jobs are not.
fn job() -> i32 {
    let mut arg = USERFONTS_AFRESH.to_string();
    loop {
        let rc = bg_font_check(&arg, 300.0);
        if rc != 5 {
            return rc;
        }
        arg = "next".to_string();
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

/// The text 2.1 wrote for the same font: its first line and its hash line were other ones, and it had no
/// `skip=` line for a font that skips no glyph.
fn verdict_text_2_1(font: &[u8]) -> String {
    let now = verdict_text(font);
    let rest: Vec<&str> = now.splitn(3, '\n').collect();
    format!("grimoire-font 1\nhash={:016x}\n{}", fnv(font), rest[2].strip_suffix("skip=\n").unwrap_or(rest[2]))
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

/// Runs `f` with stderr (the log) going to a file; the log's text.
fn with_log<R>(path: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    let r = f();
    let _ = std::io::stderr().flush();
    // SAFETY: dup2 and close on the descriptor saved above.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    let text = std::fs::read_to_string(path).unwrap_or_default();
    eprint!("{text}");
    (r, text)
}

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
    // Files' card names the family from that verdict, for the file's size and time, and only while the
    // verdict's companion is in the cache
    use std::os::unix::fs::MetadataExt;
    let md = std::fs::metadata(format!("{r}/fonts/Mine.ttf")).unwrap();
    check!(userfont_family("Mine.ttf", md.len(), md.mtime()).as_deref() == Some("Grimoire Test"));
    check!(userfont_family("Mine.ttf", md.len() + 1, md.mtime()).is_none());
    let preview = format!("{cache}/{key:016x}.pgm");
    let kept = std::fs::read(&preview).unwrap();
    std::fs::remove_file(&preview).unwrap();
    check!(userfont_family("Mine.ttf", md.len(), md.mtime()).is_none());
    std::fs::write(&preview, &kept).unwrap();
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

// A folder below Grimoire's own that is not Grimoire's own keeps nothing and gives nothing: one that others can
// write, one that is a link. The genuine verdict is behind each of them, and is not read; the job ends before any
// worker starts (6), moves nothing aside, and leaves no mark and no file anywhere. A verdict file that is a link is
// not read either, and the next check puts a file of its own in the link's place. (What stands at the name of
// Grimoire's own folder itself and is not its own is moved aside: `what_is_not_grimoires_own_is_moved_aside`.)
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
        // (nothing is moved aside below Grimoire's own folder, nor beside it)
        let aside = format!("fontverdict-{}-notown.state.not-own-", std::process::id());
        check!(!names(&tl_tmp()).iter().any(|n| n.starts_with(&aside)));
        check!(!everything(&state, &real).iter().any(|n| n.contains(".not-own-")));
    };
    chmod(&fonts, 0o777);
    refused("its fonts folder can be written by anyone");
    chmod(&fonts, 0o700);
    chmod(&tag, 0o770);
    refused("the data root's folder can be written by its group");
    chmod(&tag, 0o700);
    std::fs::rename(&fonts, format!("{tag}/fonts.real")).unwrap();
    std::os::unix::fs::symlink(format!("{tag}/fonts.real"), &fonts).unwrap();
    check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
    check!(wait_job() == 6);
    check!(bg_font_check("", 300.0) == 6);
    check!(names(&cache) == before && names(&format!("{tag}/fonts.real")) == vec![format!("{key:016x}.font")]);
    std::fs::remove_file(&fonts).unwrap();
    std::fs::rename(format!("{tag}/fonts.real"), &fonts).unwrap();
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

// Grimoire's own folder is there, is this user's and has room, and takes no write (its file system went
// read-only, say): nothing is checked. The job finds out before any worker starts and before any mark is written
// (6), and says why; nothing is left in the cache or in the folder, and a font checked before still loads. Once
// the folder takes a write again the font is checked.
fn a_folder_that_takes_no_write_checks_nothing() {
    clean_env();
    // SAFETY: geteuid takes no arguments.
    if unsafe { libc::geteuid() } == 0 {
        // (root writes into a folder whatever its mode says: this case needs a user who cannot)
        eprintln!("    not run as root: no folder refuses root a write by its mode");
        return;
    }
    let r = root("nowrite");
    let state = own_folder("nowrite");
    put(&r, "A.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    let mut other = test_font("Regular", 400);
    other.family = "Another".into();
    put(&r, "B.ttf", &ttf(&other));
    let cache = format!("{r}/fonts/.cache");
    let fonts = format!("{}/fonts", tag_dir(&state, &r));
    let (cache_before, fonts_before) = (names(&cache), names(&fonts));
    chmod(&fonts, 0o500);
    // (no worker could be had either: the job must end before it asks for one, which would be a 4)
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    let (rc, text) = with_log(&format!("{r}/nowrite.log"), || bg_font_check("", 300.0));
    check!(rc == 6);
    check!(text.contains("fonts: what the check finds cannot be kept (it takes no write); nothing is marked"));
    check!(!text.contains("passed") && !text.contains("could not be written") && !text.contains("no worker"));
    check!(names(&cache) == cache_before && names(&fonts) == fonts_before);
    check!(loads("A.ttf") == Ok("overprint"));
    check!(userfonts_try_load("B.ttf", "") == Err(LoadErr::Checking));
    check!(wait_job() == 6);
    check!(names(&cache) == cache_before && names(&fonts) == fonts_before);
    // with every font checked the job writes nothing there and asks nothing of the folder
    std::fs::remove_file(format!("{r}/fonts/B.ttf")).unwrap();
    check!(job() == 0);
    put(&r, "B.ttf", &ttf(&other));
    // the folder takes a write again: the font is checked, and nothing of the probe is left
    chmod(&fonts, 0o700);
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    check!(loads("B.ttf") == Ok("overprint"));
    check!(names(&fonts).len() == 2 && names(&fonts).iter().all(|n| n.ends_with(".font")));
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
    // (an empty name is no name: the cache, as with none)
    std::env::set_var("GRIMOIRE_STATE", "");
    check!(loads("Mine.ttf") == Ok("overprint"));
    std::env::remove_var("GRIMOIRE_STATE");
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    for dir in [cache, format!("{r}/fonts"), r.clone()] {
        chmod(&dir, 0o755);
    }
}

// A name that is set and is no absolute path asked for a folder of Grimoire's own and names none: it is a folder
// that cannot be made, never the cache. A verdict the cache holds (written while no folder was named) is not
// read under such a name, the job ends before any worker starts (6) and says why, nothing is marked, and
// nothing is made under the working directory.
fn a_name_that_is_no_absolute_path_is_not_the_cache() {
    use std::os::unix::ffi::OsStrExt;
    clean_env();
    let r = root("noname");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 0);
    check!(loads("Mine.ttf") == Ok("overprint"));
    let cache = format!("{r}/fonts/.cache");
    let before = names(&cache);
    check!(with_ext(&cache, ".font").len() == 1 && before.len() == 2);
    let relative = format!("fontverdict-{}-relative", std::process::id());
    let nested = format!("{relative}/state");
    let not_text = std::ffi::OsStr::from_bytes(b"/fontverdict-\xff-state").to_os_string();
    for name in [std::ffi::OsString::from(&relative), std::ffi::OsString::from(&nested), std::ffi::OsString::from("."), not_text] {
        eprintln!("    {name:?}");
        std::env::set_var("GRIMOIRE_STATE", &name);
        check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
        check!(wait_job() == 6);
        let (rc, text) = with_log(&format!("{r}/noname.log"), || bg_font_check("", 300.0));
        check!(rc == 6);
        check!(text.contains("fonts: what the check finds cannot be kept (it cannot be made); nothing is marked"));
        check!(names(&cache) == before);
        check!(std::fs::symlink_metadata(&relative).is_err());
    }
    // with no name the cache's verdict is read as before
    std::env::remove_var("GRIMOIRE_STATE");
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    check!(loads("Mine.ttf") == Ok("overprint"));
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
}

// The app, when its own folder cannot be used (here the data root's folder in it is a file, which is not moved
// aside: only what stands at the name of Grimoire's own folder itself is): it says so once, checks no font in
// that session and marks none; Terminess draws, and the Font sheet's row says the font cannot be read.
fn the_app_says_once_that_nothing_can_be_kept() {
    clean_env();
    let r = root("app");
    let state = own_folder("app");
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o700);
    let tag = tag_dir(&state, &r);
    std::fs::write(&tag, b"a file, not a folder").unwrap();
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::write(format!("{r}/settings.ini"), OWN).unwrap();
    let script = "wait 100\nwaitbg\nwait 300\nexpect font is terminess\nexpect settings.font is file:Mine.ttf\ntap 1535 95\nwait 50\ntap library settings.cat.1\nwait 50\ntap library font\nwait 300\n\
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
    check!(!out.contains("moved aside"));
    check!(names(&format!("{r}/fonts/.cache")).is_empty());
    check!(std::fs::read(&tag).is_ok_and(|b| b == b"a file, not a folder"));
    check!(names(&state).len() == 1);
    remove_any(&state);
}

// A font renamed in Files keeps what its check found (issue #144): by the SHA-256 in its verdict the renamed file is
// the same file, so it needs no new check. With no worker to be had, the font under its new name is ready and
// loads (its bytes hash to the verdict's), where the verdicts are in Grimoire's own folder and where they are in
// the cache; so does the font of a renamed folder. The verdict under the new name's key is the very text the check
// wrote, the old key's is gone, and so for the preview. A file renamed with nothing carried, as over USB, has no
// verdict under its new name and waits for a check: the carry is what spares it.
fn a_renamed_font_keeps_its_verdict() {
    for own in [true, false] {
        clean_env();
        let r = root(if own { "renamed" } else { "renamed-cache" });
        let state = own.then(|| own_folder("renamed"));
        put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
        std::fs::create_dir_all(format!("{r}/fonts/Fam")).unwrap();
        put(&r, "Fam/In.ttf", &ttf(&test_font("Bold", 700)));
        check!(job() == 0);
        let cache = format!("{r}/fonts/.cache");
        let verdicts = state.as_ref().map_or(cache.clone(), |s| format!("{}/fonts", tag_dir(s, &r)));
        let was = key_of("Mine.ttf");
        let text = std::fs::read(format!("{verdicts}/{was:016x}.font")).unwrap_or_default();
        check!(!text.is_empty());
        std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
        // renamed in Files: the storage's rename, then what the check found goes with the file
        std::fs::rename(format!("{r}/fonts/Mine.ttf"), format!("{r}/fonts/Yours.ttf")).unwrap();
        userfonts_renamed("Mine.ttf", "Yours.ttf");
        check!(loads("Yours.ttf") == Ok("overprint"));
        check!(state_of("Yours.ttf") == "ready");
        let now = key_of("Yours.ttf");
        check!(now != was);
        check!(std::fs::read(format!("{verdicts}/{now:016x}.font")).is_ok_and(|t| t == text));
        check!(!std::path::Path::new(&format!("{verdicts}/{was:016x}.font")).exists());
        check!(std::path::Path::new(&format!("{cache}/{now:016x}.pgm")).exists());
        check!(!std::path::Path::new(&format!("{cache}/{was:016x}.pgm")).exists());
        // the app's next start on this data root, Settings naming the new path: the font draws on its first screen
        // and no check is run (there is no worker to be had)
        std::fs::write(format!("{r}/settings.ini"), OWN.replace("Mine.ttf", "Yours.ttf")).unwrap();
        let script = "wait 100\nexpect font is file:Yours.ttf\nexpect settings.font is file:Yours.ttf\nexpect font.own.drawn is 1\nquit\n";
        let mut env = vec![("GRIMOIRE_WORK_EXE", NO_WORKER)];
        if let Some(s) = &state {
            env.push(("GRIMOIRE_STATE", s.as_str()));
        }
        let (code, out) = start(&r, script, &env);
        check!(code == Some(0));
        check!(!out.contains("bg: font check of"));
        if code != Some(0) {
            eprintln!("{out}");
        }
        // a folder renamed: its font too
        std::fs::rename(format!("{r}/fonts/Fam"), format!("{r}/fonts/Family")).unwrap();
        userfonts_renamed("Fam", "Family");
        check!(loads("Family/In.ttf").is_ok());
        check!(state_of("Family/In.ttf") == "ready");
        check!(with_ext(&verdicts, ".font").len() == 2);
        // out of its folder, too
        std::fs::rename(format!("{r}/fonts/Family/In.ttf"), format!("{r}/fonts/Swap.ttf")).unwrap();
        userfonts_renamed("Family/In.ttf", "Swap.ttf");
        check!(loads("Swap.ttf").is_ok());
        // the hash still ties a verdict to its bytes: a verdict that ends up under the key of other bytes (another
        // font of the same size and time put at the old name, then "renamed") loads nothing
        let other = ttf(&test_font("Bolt", 700));
        let swap = std::fs::metadata(format!("{r}/fonts/Swap.ttf")).unwrap();
        check!(other.len() as u64 == swap.len());
        std::fs::write(format!("{r}/fonts/Forged.ttf"), &other).unwrap();
        std::fs::File::options().write(true).open(format!("{r}/fonts/Forged.ttf")).and_then(|f| f.set_modified(swap.modified().unwrap())).unwrap();
        std::fs::remove_file(format!("{r}/fonts/Swap.ttf")).unwrap();
        userfonts_renamed("Swap.ttf", "Forged.ttf");
        check!(loads("Forged.ttf") == Err(LoadErr::Checking));
        // renamed with nothing carried, as over USB: no verdict under the new name, and it waits for a check
        std::fs::rename(format!("{r}/fonts/Yours.ttf"), format!("{r}/fonts/Usb.ttf")).unwrap();
        check!(loads("Usb.ttf") == Err(LoadErr::Checking));
        std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
        check!(job() == 0);
        // (checked again, it loads; the Bold file of its family, renamed beside it above, is found by its family)
        check!(loads("Usb.ttf") == Ok("a Bold file"));
    }
}

/// The log line of an entry moved aside from the name of Grimoire's own folder, as far as it names what it was.
fn moved_line(what: &str, mode: u32) -> String {
    // SAFETY: geteuid takes no arguments.
    let uid = unsafe { libc::geteuid() };
    format!("fonts: the entry at the name of Grimoire's own folder was not its own ({what}, owner {uid}, mode {:04o}): it is moved aside as it was", mode & 0o7777)
}

const MOVED: &str = "it is moved aside as it was, and the folder is made afresh; every font is checked once more";

// What stands at the name of Grimoire's own folder and is not Grimoire's own (issue #201) is moved aside in its
// parent, and the folder is made afresh: a folder its group or others can write, a link to a folder of this
// user's, a link to nothing, a file, a pipe. Each is moved as it was, under a name of Grimoire's own
// (`grimoire.not-own-<seconds>-<pid>`): the same inode and mode, what it holds unchanged, a link still a link to
// the same place and never followed (the folder it names, outside the parent, as it was), a pipe never opened.
// The log says once what it was (its kind, owner and mode), never a path. The fresh folder is 0700, the font's
// verdict is kept in it and the font loads; a second job moves nothing. Nothing else in the parent changes.
fn what_is_not_grimoires_own_is_moved_aside() {
    use std::os::unix::fs::MetadataExt;
    let font = ttf(&test_font("Regular", 400));
    // SAFETY: geteuid takes no arguments.
    let uid = unsafe { libc::geteuid() };
    for (kind, what) in [("group", "a folder"), ("others", "a folder"), ("link", "a link"), ("dangling", "a link"), ("file", "a file"), ("pipe", "a pipe")] {
        eprintln!("    {kind}");
        clean_env();
        let r = root(&format!("aside-{kind}"));
        let (parent, state) = own_parent(&format!("aside-{kind}"));
        std::fs::write(format!("{parent}/beside"), b"left alone").unwrap();
        let outside = format!("{}/fontverdict-{}-aside-{kind}.outside", tl_tmp(), std::process::id());
        remove_any(&outside);
        std::fs::create_dir(&outside).unwrap();
        chmod(&outside, 0o700);
        std::fs::write(format!("{outside}/keep.me"), b"outside").unwrap();
        match kind {
            "group" | "others" => {
                std::fs::create_dir(&state).unwrap();
                std::fs::write(format!("{state}/keep.me"), b"not Grimoire's").unwrap();
                chmod(&state, if kind == "group" { 0o775 } else { 0o757 });
            }
            "link" => std::os::unix::fs::symlink(&outside, &state).unwrap(),
            "dangling" => std::os::unix::fs::symlink(format!("{outside}/nothing"), &state).unwrap(),
            "file" => std::fs::write(&state, b"a file where the folder was").unwrap(),
            _ => {
                let c = std::ffi::CString::new(state.as_str()).unwrap();
                // SAFETY: mkfifo reads the NUL-terminated path.
                check!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) } == 0);
            }
        }
        let Some(was) = entry(&state) else { continue };
        let times = |p: &str| std::fs::symlink_metadata(p).map(|m| (m.mtime(), m.mtime_nsec())).ok();
        let outside_was = (entry(&outside), times(&outside), times(&format!("{outside}/keep.me")));
        put(&r, "Mine.ttf", &font);
        let (rc, text) = with_log(&format!("{r}/aside.log"), job);
        check!(rc == 0);
        check!(text.matches(&moved_line(what, was.1)).count() == 1 && text.matches(MOVED).count() == 1);
        check!(!text.contains(&parent));
        // the parent: the fresh folder, the entry under its new name, and what was beside it, as it was
        let moved = asides(&parent);
        check!(moved.len() == 1 && names(&parent).len() == 3);
        let Some(aside) = moved.first() else { continue };
        let tail: Vec<&str> = aside["grimoire.not-own-".len()..].split('-').collect();
        check!(tail.len() == 2 && tail[0].parse::<i64>().is_ok_and(|s| s > 1_700_000_000) && tail[1] == std::process::id().to_string());
        check!(entry(&format!("{parent}/{aside}")).as_ref() == Some(&was));
        check!(std::fs::read(format!("{parent}/beside")).is_ok_and(|b| b == b"left alone"));
        check!((entry(&outside), times(&outside), times(&format!("{outside}/keep.me"))) == outside_was);
        // the fresh folder: Grimoire's own, with the font's verdict in it, and the font loads
        check!(std::fs::symlink_metadata(&state).is_ok_and(|m| m.is_dir() && m.mode() & 0o7777 == 0o700 && m.uid() == uid));
        check!(with_ext(&format!("{}/fonts", tag_dir(&state, &r)), ".font").len() == 1);
        check!(loads("Mine.ttf") == Ok("overprint"));
        // a second job, in this process and in one forked as the app forks it: nothing is moved again
        check!(job() == 0);
        bg_queue(BgKind::FontCheck, Some(""), Some("again"), 300.0, false);
        check!(wait_job() == 0);
        check!(asides(&parent) == moved && names(&parent).len() == 3);
        check!(entry(&format!("{parent}/{aside}")).as_ref() == Some(&was));
        remove_any(&parent);
        remove_any(&outside);
    }
}

// At most once in a process too, even with leave (USERFONTS_AFRESH): after a move aside, what stands at the name
// and is not Grimoire's own again (the fresh folder renamed away and a folder others can write put in its place) is
// not moved by that process. Nothing is kept (6, as for any folder that is not Grimoire's own), and the entry stays
// as it is. A fresh process given leave moves it, and checks the font once more; but never onto a name that is
// taken: with an empty folder at every name it could take in the next seconds (one a plain rename of a folder would
// replace), it gives up and they are as they were. (The app gives that leave to the first check of a run alone:
// `a_run_makes_its_own_folder_afresh_once`.)
fn moved_aside_at_most_once_a_process() {
    clean_env();
    let r = root("aside-once");
    let (parent, state) = own_parent("aside-once");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o777);
    check!(job() == 0);
    check!(asides(&parent).len() == 1);
    std::fs::rename(&state, format!("{parent}/renamed")).unwrap();
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o777);
    let was = entry(&state);
    let cache = format!("{r}/fonts/.cache");
    let cache_before = names(&cache);
    let (rc, text) = with_log(&format!("{r}/once.log"), || bg_font_check(USERFONTS_AFRESH, 300.0));
    check!(rc == 6);
    check!(text.contains("fonts: what the check finds cannot be kept (it is not Grimoire's own); nothing is marked"));
    check!(!text.contains("moved aside"));
    check!(asides(&parent).len() == 1 && names(&parent).len() == 3 && entry(&state) == was);
    check!(names(&cache) == cache_before);
    // the next process, with every name it could move to taken
    let first: i64 = asides(&parent).first().and_then(|n| n["grimoire.not-own-".len()..].split('-').next()?.parse().ok()).unwrap_or(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let decoys: Vec<String> = (now - 1..now + 30).map(|s| format!("{parent}/grimoire.not-own-{s}-{}", std::process::id())).filter(|p| !std::path::Path::new(p).exists()).collect();
    for d in &decoys {
        std::fs::create_dir(d).unwrap();
    }
    let all_before: Vec<Option<Entry>> = names(&parent).iter().map(|n| entry(&format!("{parent}/{n}"))).collect();
    userfonts_aside_reset();
    let (rc, text) = with_log(&format!("{r}/once.log"), || bg_font_check(USERFONTS_AFRESH, 300.0));
    check!(rc == 6 && text.contains(") and cannot be moved aside (File exists)") && !text.contains("moved aside as it was"));
    check!(names(&parent).iter().map(|n| entry(&format!("{parent}/{n}"))).collect::<Vec<_>>() == all_before);
    for d in &decoys {
        std::fs::remove_dir(d).unwrap();
    }
    // the next process: moved (in a later second than the first move, whose name it would take)
    while std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64 <= first {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    userfonts_aside_reset();
    let (rc, text) = with_log(&format!("{r}/once.log"), job);
    check!(rc == 0 && text.matches(MOVED).count() == 1);
    check!(asides(&parent).len() == 2 && names(&parent).len() == 4);
    check!(text.contains("fonts: file 1 of 1 passed"));
    check!(loads("Mine.ttf") == Ok("overprint"));
    remove_any(&parent);
}

// An entry that comes to the name between the move aside and the fresh folder's make (another writer of the parent,
// at that moment) is not moved: the job gives up (6, as for any folder that is not Grimoire's own), says why and
// marks nothing, and the entry stays at the name as it was put there, beside the one entry moved aside.
fn an_entry_back_at_once_is_not_moved_again() {
    clean_env();
    let r = root("aside-race");
    let (parent, state) = own_parent("aside-race");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o775);
    let again = state.clone();
    userfonts_aside_seam(Some(Box::new(move || {
        std::fs::create_dir(&again).unwrap();
        std::fs::set_permissions(&again, std::fs::Permissions::from_mode(0o777)).unwrap();
    })));
    let (rc, text) = with_log(&format!("{r}/race.log"), || bg_font_check(USERFONTS_AFRESH, 300.0));
    userfonts_aside_seam(None);
    check!(rc == 6);
    check!(text.matches(MOVED).count() == 1);
    check!(text.contains("fonts: the folder made afresh is not Grimoire's own either (it is not Grimoire's own); nothing more is moved aside"));
    check!(text.contains("fonts: what the check finds cannot be kept (it is not Grimoire's own); nothing is marked"));
    check!(asides(&parent).len() == 1 && names(&parent).len() == 2);
    check!(mode_of(&state) == 0o777 && names(&state).is_empty());
    check!(names(&format!("{r}/fonts/.cache")).is_empty());
    remove_any(&parent);
}

// A parent that takes no write: the entry cannot be moved aside. The job gives up (6, as for any folder that is not
// Grimoire's own), says why, and the parent is as it was. With nothing at the name the folder cannot be made, as
// before this change. (Root writes into a folder whatever its mode says: not run as root.)
fn a_parent_that_takes_no_write_moves_nothing() {
    clean_env();
    // SAFETY: geteuid takes no arguments.
    let uid = unsafe { libc::geteuid() };
    if uid == 0 {
        eprintln!("    not run as root: no folder refuses root a write by its mode");
        return;
    }
    let r = root("aside-nowrite");
    let (parent, state) = own_parent("aside-nowrite");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o775);
    let (was, before) = (entry(&state), names(&parent));
    chmod(&parent, 0o555);
    let (rc, text) = with_log(&format!("{r}/nowrite.log"), || bg_font_check(USERFONTS_AFRESH, 300.0));
    chmod(&parent, 0o755);
    check!(rc == 6);
    check!(text.contains(&format!("fonts: the entry at the name of Grimoire's own folder is not its own (a folder, owner {uid}, mode 0775) and cannot be moved aside (")));
    check!(text.contains("fonts: what the check finds cannot be kept (it is not Grimoire's own); nothing is marked"));
    check!(!text.contains(MOVED));
    check!(names(&parent) == before && entry(&state) == was);
    remove_any(&state);
    userfonts_aside_reset();
    chmod(&parent, 0o555);
    let (rc, text) = with_log(&format!("{r}/nowrite.log"), || bg_font_check(USERFONTS_AFRESH, 300.0));
    chmod(&parent, 0o755);
    check!(rc == 6);
    check!(text.contains("fonts: what the check finds cannot be kept (it cannot be made); nothing is marked"));
    check!(names(&parent).is_empty());
    remove_any(&parent);
}

// The app, started with a folder others can write at the name of its own folder: the job it forks moves that aside,
// checks the font in the fresh folder, and the font draws. The log says so once and never names a path.
fn the_app_moves_aside_what_is_not_its_own() {
    clean_env();
    let r = root("app-aside");
    let (parent, state) = own_parent("app-aside");
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o775);
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::write(format!("{r}/settings.ini"), OWN).unwrap();
    let script = "wait 100\nwaitbg\nwait 300\nexpect font is file:Mine.ttf\nexpect settings.font is file:Mine.ttf\nquit\n";
    let (code, out) = start(&r, script, &[("GRIMOIRE_STATE", &state), ("GRIMOIRE_WORK_EXE", BIN)]);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    check!(out.matches(&moved_line("a folder", 0o775)).count() == 1);
    check!(!out.contains(&parent));
    check!(asides(&parent).len() == 1 && names(&parent).len() == 2);
    check!(mode_of(&state) == 0o700);
    check!(with_ext(&format!("{}/fonts", tag_dir(&state, &r)), ".font").len() == 1);
    remove_any(&parent);
}

// At most once a run: the UI gives the first check of a run alone leave to make Grimoire's own folder, or make it
// afresh (USERFONTS_AFRESH). The run's first look queues a check (forked, as the app forks it) that moves aside the
// folder others can write at the name and checks the font in the fresh folder. With such a folder at the name again,
// and then with nothing there, the run's later checks keep nothing (6): they move nothing, make nothing, mark nothing,
// and say why. The next run (the UI's memory starts afresh) moves it aside once more and checks the font again.
fn a_run_makes_its_own_folder_afresh_once() {
    clean_env();
    let r = root("aside-run");
    let (parent, state) = own_parent("aside-run");
    put(&r, "Mine.ttf", &ttf(&test_font("Regular", 400)));
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o775);
    let log = format!("{r}/run.log");
    let (rc, text) = with_log(&log, || {
        let _ = userfonts_scan(300.0);
        wait_job()
    });
    check!(rc == 0 && text.matches(MOVED).count() == 1 && text.contains("fonts: file 1 of 1 passed"));
    check!(asides(&parent).len() == 1 && mode_of(&state) == 0o700);
    // (a later check of the run, queued by a load that finds no verdict)
    let later = || {
        check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
        wait_job()
    };
    std::fs::rename(&state, format!("{parent}/away")).unwrap();
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o775);
    let (was, cache_before) = (entry(&state), names(&format!("{r}/fonts/.cache")));
    let (rc, text) = with_log(&log, &later);
    check!(rc == 6 && !text.contains("moved aside"));
    check!(text.contains("fonts: what the check finds cannot be kept (it is not Grimoire's own); nothing is marked"));
    check!(asides(&parent).len() == 1 && entry(&state) == was);
    remove_any(&state);
    let (rc, text) = with_log(&log, &later);
    check!(rc == 6 && text.contains("fonts: what the check finds cannot be kept (it is not there); nothing is marked"));
    check!(std::fs::symlink_metadata(&state).is_err() && asides(&parent).len() == 1);
    check!(names(&format!("{r}/fonts/.cache")) == cache_before);
    // the next run
    remove_any(&state);
    std::fs::create_dir(&state).unwrap();
    chmod(&state, 0o775);
    userfonts_aside_reset();
    let (rc, text) = with_log(&log, &later);
    check!(rc == 0 && text.matches(MOVED).count() == 1 && text.contains("fonts: file 1 of 1 passed"));
    check!(asides(&parent).len() == 2 && mode_of(&state) == 0o700);
    check!(loads("Mine.ttf") == Ok("overprint"));
    remove_any(&parent);
}

/// A writer of the parent that is never Grimoire, racing the app (the review's helper of issue #201): whenever a
/// folder of Grimoire's own (0700) stands at the name it renames it away and, when `plant`, puts a folder others can
/// write in its place. It runs until `stop`; how many folders it took.
fn racer(parent: String, plant: bool, stop: std::sync::Arc<std::sync::atomic::AtomicBool>) -> std::thread::JoinHandle<u32> {
    std::thread::spawn(move || {
        let name = format!("{parent}/grimoire");
        let mut taken = 0;
        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            let own = std::fs::symlink_metadata(&name).is_ok_and(|m| m.is_dir() && m.permissions().mode() & 0o777 == 0o700);
            if own && std::fs::rename(&name, format!("{parent}/taken-{taken}")).is_ok() {
                taken += 1;
                let p = format!("{parent}/planting");
                if plant && std::fs::create_dir(&p).is_ok() {
                    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o775));
                    if std::fs::rename(&p, &name).is_err() {
                        let _ = std::fs::remove_dir(&p);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_micros(500));
        }
        taken
    })
}

// The app against a writer of the parent that takes Grimoire's folder away after every check, putting a folder others
// can write in its place, or leaving nothing. Three fonts, so that a run goes on from check to check (one font a
// check). One move aside a run at most, and three checks at most: once the run's first check is done, a check that
// finds the folder gone or not Grimoire's own keeps nothing, and the session checks no more fonts (said once). The
// next start, with no such writer, moves aside the folder put there once, and the font draws.
fn a_writer_of_the_parent_costs_one_move_a_run() {
    for plant in [true, false] {
        clean_env();
        let what = if plant { "plant" } else { "away" };
        let r = root(&format!("app-race-{what}"));
        let (parent, state) = own_parent(&format!("app-race-{what}"));
        let font = ttf(&test_font("Regular", 400));
        for n in ["Mine.ttf", "Two.ttf", "Three.ttf"] {
            put(&r, n, &font);
        }
        std::fs::write(format!("{r}/settings.ini"), OWN).unwrap();
        if plant {
            std::fs::create_dir(&state).unwrap();
            chmod(&state, 0o775);
        }
        let env = [("GRIMOIRE_STATE", state.as_str()), ("GRIMOIRE_WORK_EXE", BIN)];
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let racing = racer(parent.clone(), plant, stop.clone());
        let (code, out) = start(&r, "wait 2500\nquit\n", &env);
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let taken = racing.join().unwrap_or(0);
        let checks = out.lines().filter(|l| l.contains("bg: font check of")).count();
        let (moved, stopped) = (out.matches(MOVED).count(), out.matches("no font is checked until Grimoire starts again").count());
        eprintln!("    {what}: {checks} check(s), {moved} moved aside, {taken} folder(s) taken by the writer, the checks stopped {stopped} time(s)");
        check!(code == Some(0));
        check!(taken >= 1 && (1..=3).contains(&checks) && stopped <= 1);
        check!(moved == usize::from(plant) && asides(&parent).len() == usize::from(plant));
        check!(!out.contains(&parent));
        if plant {
            remove_any(&state);
            std::fs::create_dir(&state).unwrap();
            chmod(&state, 0o775);
            let (code, out) = start(&r, "wait 100\nwaitbg\nwait 300\nexpect font is file:Mine.ttf\nquit\n", &env);
            check!(code == Some(0) && out.matches(MOVED).count() == 1 && asides(&parent).len() == 2);
            if code != Some(0) {
                eprintln!("{out}");
            }
        }
        remove_any(&parent);
    }
}

#[test]
fn test_sec_font_verdict() {
    run!(only_grimoires_own_folder_is_read);
    run!(the_verdict_is_kept_in_grimoires_own_folder);
    run!(checked_once_on_this_device_and_again_elsewhere);
    run!(a_folder_that_is_not_grimoires_own_keeps_nothing);
    run!(what_is_not_grimoires_own_is_moved_aside);
    run!(moved_aside_at_most_once_a_process);
    run!(an_entry_back_at_once_is_not_moved_again);
    run!(a_parent_that_takes_no_write_moves_nothing);
    run!(two_data_roots_keep_their_verdicts_apart);
    run!(an_earlier_versions_verdict_is_left_alone);
    run!(the_fonts_cache_deleted_checks_again);
    run!(too_little_room_checks_nothing);
    run!(a_folder_that_takes_no_write_checks_nothing);
    run!(without_a_folder_of_its_own_the_cache_keeps_verdicts);
    run!(a_name_that_is_no_absolute_path_is_not_the_cache);
    run!(a_renamed_font_keeps_its_verdict);
    run!(the_app_says_once_that_nothing_can_be_kept);
    run!(the_app_moves_aside_what_is_not_its_own);
    run!(a_run_makes_its_own_folder_afresh_once);
    run!(a_writer_of_the_parent_costs_one_move_a_run);
    done();
}
