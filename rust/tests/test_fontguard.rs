// test_fontguard.rs: the crash guard of your own fonts (2.1.0-beta.2, DESIGN.md 4.2.8), start by start, with the
// real app: the headless runner started again and again on one data root, as the Kindle's launcher starts
// Grimoire again after it died. A crash during a custom font's first screen is simulated by the runner's hook
// GRIMOIRE_FONT_TEST_DRAW_CRASH, which ends the process at the first glyph a custom font draws.
//
//   start 1   the font is in use, its mark is written, the first screen never ends (the hook)
//   start 2   the mark is found: a strike, and this start is in Terminess
//   start 3   the font is tried once more; the screen never ends again
//   start 4   the second strike: Terminess, and at every start after it
//   then      the file changes (another key): it is checked afresh and is the font again
#![cfg(feature = "sim")]
mod fontfix;
mod testlib;
use fontfix::*;
use grimoire::prelude::*;
use grimoire::userfonts::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use testlib::*;

const BIN: &str = env!("CARGO_BIN_EXE_grimoire-headless");

fn root(name: &str) -> String {
    let d = format!("{}/fontguard-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(format!("{d}/fonts")).unwrap();
    d
}

/// One start of the app on `root`, running `script`: its exit (None: it died by a signal, with which) and what
/// it printed.
fn start(root: &str, script: &str, crash: bool) -> (Option<i32>, Option<i32>, String) {
    let path = format!("{root}/script.qs");
    std::fs::write(&path, script).unwrap();
    let mut c = Command::new(BIN);
    for (k, _) in std::env::vars() {
        if k.starts_with("GRIMOIRE_") {
            c.env_remove(k);
        }
    }
    c.env("GRIMOIRE_ROOT", root).env("GRIMOIRE_SCRIPT", &path).env("GRIMOIRE_LANG", "en").env("GRIMOIRE_IMPORT_STABLE", "fast");
    if crash {
        c.env("GRIMOIRE_FONT_TEST_DRAW_CRASH", "1");
    }
    let out = c.output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.code(), out.status.signal(), text)
}

fn guard(root: &str) -> Vec<GuardEntry> {
    match std::fs::read(format!("{root}/.font-inuse")) {
        Ok(b) => guard_parse(&b).expect("the guard file reads"),
        Err(_) => Vec::new(),
    }
}

/// A data root whose settings ask for fonts/Mine.ttf in Classic, the font checked already.
fn prepared(name: &str) -> String {
    let r = root(name);
    std::fs::write(format!("{r}/fonts/Mine.ttf"), ttf(&test_font("Regular", 400))).unwrap();
    std::fs::write(format!("{r}/settings.ini"), "# Grimoire settings\ntheme=classic\nfont=file:Mine.ttf\nwhatsnew_seen=9.9.9\n").unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    store_init(&r);
    check!(bg_font_check("", 300.0) == 0);
    r
}

const IN_USE: &str = "wait 100\nexpect font is file:Mine.ttf\nexpect settings.font is file:Mine.ttf\nexpect font.own.drawn is 1\nexpect font.guard is\nquit\n";
const TERMINESS: &str = "wait 100\nexpect font is terminess\nexpect settings.font is file:Mine.ttf\nexpect font.own.drawn is 0\nquit\n";

// Two starts that never finish their first screen keep the font out until its file changes.
fn two_strikes_then_terminess_until_the_file_changes() {
    let r = prepared("strikes");
    // A healthy start first: the font draws, and its mark is gone once the first screen is on the panel.
    let (code, _, out) = start(&r, IN_USE, false);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    check!(guard(&r).is_empty());
    // start 1: the first screen never ends
    let (code, signal, _) = start(&r, IN_USE, true);
    check!(code.is_none() && signal == Some(libc::SIGABRT));
    let g = guard(&r);
    check!(g.len() == 1 && g[0].strikes == 0 && g[0].armed);
    let key = g[0].key;
    // start 2: a strike; Terminess for this start, and the setting stays as it is
    let (code, _, out) = start(&r, TERMINESS, true);
    check!(code == Some(0));
    check!(out.contains("did not finish its first screen"));
    check!(guard(&r) == vec![GuardEntry { key, strikes: 1, armed: false }]);
    // start 3: tried once more, and it dies again
    let (code, signal, _) = start(&r, IN_USE, true);
    check!(code.is_none() && signal == Some(libc::SIGABRT));
    check!(guard(&r) == vec![GuardEntry { key, strikes: 1, armed: true }]);
    // start 4: the second strike. No hook any more: the font would draw, and is not asked to
    let (code, _, _) = start(&r, TERMINESS, false);
    check!(code == Some(0));
    check!(guard(&r) == vec![GuardEntry { key, strikes: 2, armed: false }]);
    // every start after: Terminess, and the Font sheet says the font cannot be used
    let sheet = "wait 100\nexpect font is terminess\ntap 1535 95\nwait 50\ntap library font\nwait 50\nexpect library.font.state.Mine.ttf is unreadable\n\
expect library.ui_has.font.file:Mine.ttf is 0\ntap library font.cancel\nwait 50\nquit\n";
    let (code, _, out) = start(&r, sheet, false);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    check!(guard(&r) == vec![GuardEntry { key, strikes: 2, armed: false }]);
    // The file changes (the same font saved again, a later time): another key. It is checked afresh in the
    // background and, the Library in front, is the font again.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
    std::fs::File::options().write(true).open(format!("{r}/fonts/Mine.ttf")).unwrap().set_modified(later).unwrap();
    let again = "wait 100\nwaitbg\nwait 200\nexpect font is file:Mine.ttf\nexpect font.own.drawn is 1\nquit\n";
    let (code, _, out) = start(&r, again, false);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    // (the old file's strikes are still on record, in no one's way; the Font sheet's look at the folder forgets
    // them, and lists the font as one to use)
    check!(guard(&r) == vec![GuardEntry { key, strikes: 2, armed: false }]);
    let sheet = "wait 100\nexpect font is file:Mine.ttf\ntap 1535 95\nwait 50\ntap library font\nwait 50\nexpect library.font.state.Mine.ttf is ready\n\
expect library.font.pick is file:Mine.ttf\nexpect font.guard is\ntap library font.cancel\nwait 50\nquit\n";
    let (code, _, out) = start(&r, sheet, false);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    check!(guard(&r).is_empty());
}

// One start that never finished (a power cut during the first screen) costs one start in Terminess, not the
// font: the start after that draws it, and the mark is gone.
fn one_strike_is_forgiven() {
    let r = prepared("forgiven");
    let (code, _, _) = start(&r, IN_USE, true);
    check!(code.is_none());
    let (code, _, _) = start(&r, TERMINESS, false);
    check!(code == Some(0));
    check!(guard(&r).len() == 1 && guard(&r)[0].strikes == 1);
    let (code, _, out) = start(&r, IN_USE, false);
    check!(code == Some(0));
    if code != Some(0) {
        eprintln!("{out}");
    }
    check!(guard(&r).is_empty());
    check!(!std::path::Path::new(&format!("{r}/.font-inuse")).exists());
}

// Gothic never loads the font: nothing is armed, nothing can strike; back in Classic it is the font.
fn gothic_holds_no_custom_font() {
    let r = prepared("gothic");
    std::fs::write(format!("{r}/settings.ini"), "# Grimoire settings\ntheme=gothic\nfont=file:Mine.ttf\nwhatsnew_seen=9.9.9\n").unwrap();
    let script = "wait 100\nexpect font is terminess\nexpect font.own.bytes is 0\nexpect settings.font is file:Mine.ttf\nquit\n";
    let (code, _, _) = start(&r, script, true);
    check!(code == Some(0));
    check!(guard(&r).is_empty());
}

#[test]
fn test_fontguard() {
    run!(two_strikes_then_terminess_until_the_file_changes);
    run!(one_strike_is_forgiven);
    run!(gothic_holds_no_custom_font);
    done();
}
