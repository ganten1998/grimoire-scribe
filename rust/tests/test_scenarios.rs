// The headless runner's checks, against the Rust runner (cargo test --features sim): ../Makefile's
// scenarios rule (make rust-scenarios: every tests/scenarios/*.qs, then the runner's self-test,
// tests/selftest/pass.qs and fail.qs) and its l10n-check rule (make rust-l10n-check: every screen
// in every language with the fit check). The scripts and the rules are the C's, shared as they are
// (PORTING.md, The map): the self-test's two runs are checked here as the rule checks them, and the
// scenarios and the languages run through the rules themselves, with the runner cargo built put
// where they look for one. Two more runs check the runner's first words, before any App exists.
//
//   cargo test --features sim --test test_scenarios -- --test-threads=1 --include-ignored
#![cfg(feature = "sim")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_grimoire-headless");

// ../Makefile's SIM_HOOKS: nothing else of the caller's reaches the app's test hooks.
const SIM_HOOKS: [&str; 22] = [
    "GRIMOIRE_HOST",
    "GRIMOIRE_PASSCODE",
    "GRIMOIRE_SIM_ADDRS",
    "GRIMOIRE_HTTPD_DELAY_MS",
    "GRIMOIRE_BG_DELAY_MS",
    "GRIMOIRE_SIM_PRESSURE",
    "GRIMOIRE_BT_API",
    "GRIMOIRE_UPDATE_SCRIPT",
    "GRIMOIRE_UPDATE_REQUEST",
    "FAKE_UPDATE_PREVIEW",
    "FAKE_UPDATE_STABLE",
    "FAKE_UPDATE_FAIL",
    "GRIMOIRE_SIM_STATS",
    "GRIMOIRE_SIM_SIZE",
    "GRIMOIRE_CHECK_FIT",
    "GRIMOIRE_LANG",
    "GRIMOIRE_L10N_DIR",
    "GRIMOIRE_ANNOTATE",
    "GRIMOIRE_SLEEP_MINUTE_MS",
    "GRIMOIRE_ROOT",
    "GRIMOIRE_SCRIPT",
    "GRIMOIRE_SHOTS",
];

fn top() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// A fresh scratch directory (the Makefile's rust-test sets GRIMOIRE_TEST_TMP).
fn scratch(name: &str) -> PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join(format!("test_scenarios-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The runner as the scenarios rule runs it: in English ($(SCEN_LANG)), the hooks unset, from the
/// top of the tree.
fn runner(env: &[(&str, &Path)]) -> Output {
    let mut c = Command::new(BIN);
    c.current_dir(top());
    for h in SIM_HOOKS {
        c.env_remove(h);
    }
    c.env("GRIMOIRE_LANG", "en");
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

/// Its stderr without the log lines' "HH:MM:SS.mmm " stamps, which change from run to run.
fn stderr_unstamped(o: &Output) -> String {
    let text = String::from_utf8_lossy(&o.stderr);
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        let b = line.as_bytes();
        let stamped = b.len() > 13
            && b[2] == b':'
            && b[5] == b':'
            && b[8] == b'.'
            && b[12] == b' '
            && [0, 1, 3, 4, 6, 7, 9, 10, 11].iter().all(|&i| b[i].is_ascii_digit());
        out.push_str(if stamped { &line[13..] } else { line });
    }
    out
}

fn version() -> String {
    std::fs::read_to_string(top().join("VERSION")).unwrap().trim().to_string()
}

// grimoire-headless with no script (display_open): it says what it needs and ends with status 1,
// before any App, as the C runner does.
#[test]
fn runner_needs_a_script() {
    let d = scratch("noscript");
    let root = d.join("root");
    let o = runner(&[("GRIMOIRE_ROOT", &root)]);
    assert_eq!(o.status.code(), Some(1));
    let want = format!(
        "Grimoire {} starting, data in {}\n\
         i18n: language en (auto, from GRIMOIRE_LANG=en)\n\
         grimoire-headless needs GRIMOIRE_SCRIPT=<file> (use grimoire-sim for a window)\n\
         display_open failed\n",
        version(),
        root.display()
    );
    assert_eq!(stderr_unstamped(&o), want);
}

// A script that cannot be opened (input_open): the display closes as it does at the end of a run
// (its refresh counts, no checks), and the status is 1.
#[test]
fn runner_cannot_open_its_script() {
    let d = scratch("badscript");
    let root = d.join("root");
    let script = d.join("no-such-script.qs");
    let o = runner(&[("GRIMOIRE_ROOT", &root), ("GRIMOIRE_SCRIPT", &script)]);
    assert_eq!(o.status.code(), Some(1));
    let want = format!(
        "Grimoire {} starting, data in {}\n\
         i18n: language en (auto, from GRIMOIRE_LANG=en)\n\
         display 1860x2480 @ 300 dpi\n\
         cannot open script {}\n\
         input_open failed\n\
         refreshes: fast 0 (0.0 MP), ui 0 (0.0 MP), full 0 (0.0 MP)\n",
        version(),
        root.display(),
        script.display()
    );
    assert_eq!(stderr_unstamped(&o), want);
}

// The runner checks itself: tests/selftest/pass.qs must exit 0 with its notebook's PDF written.
#[test]
fn selftest_pass() {
    let d = scratch("selftest");
    let root = d.join("pass");
    let o = runner(&[
        ("GRIMOIRE_ROOT", &root),
        ("GRIMOIRE_SCRIPT", &top().join("tests/selftest/pass.qs")),
        ("GRIMOIRE_SHOTS", &d.join("shots")),
    ]);
    assert_eq!(o.status.code(), Some(0), "selftest pass: stderr:\n{}", stderr_unstamped(&o));
    let pdfs = std::fs::read_dir(root.join("PDFs"))
        .map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "pdf")).count())
        .unwrap_or(0);
    assert!(pdfs > 0, "selftest pass: the run ended before its PDF export");
}

// The lines of fail.qs that must fail: every line but the comments, the empty ones, the waits and
// the taps (the rule's awk: !/^[[:space:]]*(#|$)/ && $1 != "wait" && $1 != "tap").
fn lines_meant_to_fail(script: &str) -> Vec<usize> {
    let mut want = Vec::new();
    for (i, line) in script.split('\n').enumerate() {
        let t = line.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '\x0b');
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let first = line.split([' ', '\t']).find(|w| !w.is_empty()).unwrap_or("");
        if first == "wait" || first == "tap" {
            continue;
        }
        want.push(i + 1);
    }
    want
}

// And in fail.qs every line but the comments, waits and taps must be reported as failed, with exit
// status 3.
#[test]
fn selftest_fail() {
    let d = scratch("selftest");
    let script = top().join("tests/selftest/fail.qs");
    let o = runner(&[
        ("GRIMOIRE_ROOT", &d.join("fail")),
        ("GRIMOIRE_SCRIPT", &script),
        ("GRIMOIRE_SHOTS", &d.join("shots")),
    ]);
    let log = String::from_utf8_lossy(&o.stderr).into_owned();
    let got: Vec<usize> = log
        .lines()
        .filter_map(|l| l.strip_prefix("expect failed (line "))
        .filter_map(|l| l.split(')').next()?.parse().ok())
        .collect();
    let want = lines_meant_to_fail(&std::fs::read_to_string(&script).unwrap());
    assert!(
        o.status.code() == Some(3) && got == want,
        "{log}selftest fail: exit {:?} with lines {got:?} failed, wanted exit 3 with lines {want:?}",
        o.status.code()
    );
}

// The rule's own reading of fail.qs, which the runner's line numbers must match.
#[test]
fn selftest_fail_names_the_lines_the_rule_wants() {
    let script = std::fs::read_to_string(top().join("tests/selftest/fail.qs")).unwrap();
    let want = lines_meant_to_fail(&script);
    let first = script.lines().position(|l| l.starts_with("expect screen is editor")).unwrap() + 1;
    assert_eq!(want.first(), Some(&first));
    assert!(!want.iter().any(|&n| script.lines().nth(n - 1).unwrap().starts_with("tap ")));
}

/// make RULE with BUILD at a scratch directory where the runner cargo built stands in for
/// $(BUILD)/host/grimoire-headless (-o: make takes it as up to date), as make rust-scenarios does.
fn make_rule(rule: &str) {
    let build = scratch(&format!("make-{rule}"));
    let host = build.join("host");
    std::fs::create_dir_all(&host).unwrap();
    let bin = host.join("grimoire-headless");
    std::fs::copy(BIN, &bin).unwrap();
    let mut c = Command::new("make");
    c.current_dir(top()).arg(format!("BUILD={}", build.display())).arg("-o").arg(&bin).arg(rule);
    for h in SIM_HOOKS {
        c.env_remove(h);
    }
    let st = c.status().unwrap();
    assert!(st.success(), "make {rule} against the Rust runner failed ({st})");
}

// make rust-scenarios: every tests/scenarios/*.qs (each from a fresh or copied data root, its
// checks, no file left half-written, its PDFs), then the self-test.
#[test]
#[ignore = "needs every screen: app.rs, library.rs, editor.rs, pensettings.rs, calibrate.rs, share.rs, httpd.rs, live.rs, ui.rs, keyboard.rs, pagesview.rs, dispmenu.rs (wave 3)"]
fn scenarios() {
    make_rule("scenarios");
}

// make rust-l10n-check: the catalogs as msgfmt makes them, then y_all_screens.qs in every language
// and style with GRIMOIRE_CHECK_FIT=1, a long data root, hosted by KOReader, and with templates.
#[test]
#[ignore = "needs every screen: app.rs, library.rs, editor.rs, pensettings.rs, calibrate.rs, share.rs, ui.rs, keyboard.rs (wave 3)"]
fn l10n_check() {
    make_rule("l10n-check");
}
