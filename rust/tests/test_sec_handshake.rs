// test_sec_handshake.rs (hardening round 2026-11): what the three writers take as a worker's sandbox line.
//
// The writers are this process's (pdfwork::Session, picwork::PicPool, fontcheck::check_font); the worker is a
// stand-in, a shell script that says a handshake's lines. Every stand-in makes a file `started` first, and the
// ones that wait for a command make a file `handed` when the writer sends one. Each case runs in a forked child
// (forkcase): the flags a refused worker sets are process-wide, and so is the environment.
#![cfg(feature = "sim")]
mod fontfix;
mod forkcase;
mod pdffix;
mod testlib;
use grimoire::fontcheck::*;
use grimoire::pdfwork::*;
use grimoire::picwork::*;
use grimoire::prelude::*;
use grimoire::sandbox::*;
use testlib::*;

/// What a case left behind.
struct Seen {
    code: i32,
    log: String,
    /// the stand-in ran
    started: bool,
    /// the writer sent it a command
    handed: bool,
}

/// GRIMOIRE_SANDBOX for a case: required, or not.
#[derive(Clone, Copy)]
enum Ask {
    Require,
    Not,
}

/// Runs `f` in a forked child against a stand-in worker whose script goes on with `body` after it made
/// `started` (`@BUILD@` in it is this build's handshake word, `@DIR@` the case's folder). None when the case cannot
/// run here: a Linux host that requires the sandbox and has no tmpfs /tmp refuses a start at the jail, before
/// any worker runs (one line says so, and the case counts as passed).
fn case(name: &str, body: &str, ask: Ask, f: impl FnOnce(&str) -> i32) -> Option<Seen> {
    use std::os::unix::fs::PermissionsExt;
    let dir = format!("{}/sec-handshake-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = format!("{dir}/worker.sh");
    let text = format!("#!/bin/sh\nPATH=/usr/bin:/bin\n: > '@DIR@/started'\n{body}").replace("@BUILD@", &build_id()).replace("@DIR@", &dir);
    std::fs::write(&script, text).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (code, log) = forkcase::in_child(&format!("{dir}/log"), || {
        for h in ["GRIMOIRE_WORK_FAIL_STEP", "GRIMOIRE_PIC_TEST_CRASH", "GRIMOIRE_PIC_TEST_HANG", "GRIMOIRE_PIC_TEST_TRAP"] {
            std::env::remove_var(h);
        }
        for h in ["GRIMOIRE_FONT_TEST_CRASH", "GRIMOIRE_FONT_TEST_HANG", "GRIMOIRE_FONT_TEST_TRAP"] {
            std::env::remove_var(h);
        }
        match ask {
            Ask::Require => std::env::set_var("GRIMOIRE_SANDBOX", "require"),
            Ask::Not => std::env::remove_var("GRIMOIRE_SANDBOX"),
        }
        std::env::set_var("GRIMOIRE_WORK_EXE", &script);
        f(&dir)
    });
    let seen = Seen { code, log, started: std::fs::metadata(format!("{dir}/started")).is_ok(), handed: std::fs::metadata(format!("{dir}/handed")).is_ok() };
    eprintln!("  {name}: exit code {}, the stand-in ran: {}, it was sent a command: {}; the case's log:", seen.code, seen.started, seen.handed);
    for l in seen.log.lines() {
        eprintln!("    {l}");
    }
    let _ = std::fs::remove_dir_all(&dir);
    if cfg!(target_os = "linux") && !seen.started && seen.log.contains("work: no chroot directory") {
        eprintln!("  skipped: this host requires the sandbox and its /tmp is not tmpfs, so no worker starts here");
        return None;
    }
    Some(seen)
}

/// The stand-ins' scripts, after `started`.
const SAYS_S0_THEN_WAITS: &str = "printf 'B %s\\nS 0\\n' '@BUILD@' >&4\nIFS= read -r line <&5 && : > '@DIR@/handed'\n";

/// A PDF session on a two-page planner written into the case's folder.
fn pdf_session(dir: &str) -> Result<Session, StartErr> {
    use std::os::fd::IntoRawFd;
    let bytes = pdffix::planner(2);
    let path = format!("{dir}/a.pdf");
    std::fs::write(&path, &bytes).unwrap();
    let len = bytes.len() as u64;
    // (the descriptor stays open for the child's short life)
    let fd = std::fs::File::open(&path).unwrap().into_raw_fd();
    Session::start(fd, len, &Limits { as_bytes: worker_as_bytes(len, false), cpu_secs: 60, mapped: true, file_len: len })
}

/// A small PNG's bytes.
fn small_png(dir: &str) -> Vec<u8> {
    let path = format!("{dir}/a.png");
    let src = BgPicture { px: (0..50 * 60).map(|i| (i % 250) as u8).collect(), w: 50, h: 60, ch: 1 };
    assert_eq!(bgimage_write_png(&path, &src), 0);
    std::fs::read(&path).unwrap()
}

/// What each writer came to, as a line of the case's log, and 0 when it is the outcome `want` names.
fn pdf_start_is(dir: &str, want: &str) -> i32 {
    let r = pdf_session(dir);
    let got = match &r {
        Ok(s) => format!("Ok(sandboxed: {})", s.sandboxed),
        Err(e) => format!("{e:?}"),
    };
    forkcase::note(&format!("Session::start: {got}"));
    // (a session that started is asked for its pages, so the log and `handed` show how far the writer went)
    if let Ok(mut s) = r {
        forkcase::note(&format!("the probe: {:?}", s.probe()));
    }
    (got != want) as i32
}

fn picture_waits(dir: &str) -> i32 {
    let bytes = small_png(dir);
    let out = PicPool::new().decode(&bytes, false, 300, 400);
    let got = match &out {
        PicOut::Pic { .. } => "Pic".to_string(),
        o => format!("{o:?}"),
    };
    forkcase::note(&format!("PicPool::decode: {got}; in_fallback: {}", in_fallback()));
    !(matches!(out, PicOut::NoWorker(_)) && in_fallback()) as i32
}

fn font_has_no_worker(want: &str) -> i32 {
    let out = check_font(&fontfix::ttf(&fontfix::test_font("Regular", 400)), 44);
    let got = match &out {
        FontOut::Passed { .. } => "Passed".to_string(),
        o => format!("{o:?}"),
    };
    forkcase::note(&format!("check_font: {got}"));
    (got != format!("NoWorker({want:?})")) as i32
}

// R8-4. Where the sandbox is required a worker that says `S 0` is refused before the writer sends it a command
// or a byte of the file.
fn a_pdf_worker_that_is_not_sandboxed_is_refused_where_required() {
    let Some(s) = case("pdf-s0", SAYS_S0_THEN_WAITS, Ask::Require, |dir| pdf_start_is(dir, "Worker(\"the worker is not sandboxed\")")) else {
        return;
    };
    check!(s.code == 0);
    check!(s.started);
    check!(!s.handed);
}

fn a_picture_worker_that_is_not_sandboxed_is_refused_where_required() {
    let Some(s) = case("pic-s0", SAYS_S0_THEN_WAITS, Ask::Require, picture_waits) else {
        return;
    };
    check!(s.code == 0);
    check!(s.started);
    check!(!s.handed);
    check!(s.log.contains("work: pictures wait (the worker is not sandboxed)"));
}

fn a_font_worker_that_is_not_sandboxed_is_refused_where_required() {
    let Some(s) = case("font-s0", SAYS_S0_THEN_WAITS, Ask::Require, |_| font_has_no_worker("the worker is not sandboxed")) else {
        return;
    };
    check!(s.code == 0);
    check!(s.started);
    check!(!s.handed);
}

// (the guard: where the sandbox is not required, the Mac and the simulator, `S 0` is a worker as before)
fn where_the_sandbox_is_not_required_s0_is_taken() {
    let Some(s) = case("pdf-s0-free", SAYS_S0_THEN_WAITS, Ask::Not, |dir| pdf_start_is(dir, "Ok(sandboxed: false)")) else {
        return;
    };
    check!(s.code == 0);
    check!(s.started);
    // (the session started, so the writer went on to its first command)
    check!(s.handed);
}

#[test]
fn test_sec_handshake() {
    run!(a_pdf_worker_that_is_not_sandboxed_is_refused_where_required);
    run!(a_picture_worker_that_is_not_sandboxed_is_refused_where_required);
    run!(a_font_worker_that_is_not_sandboxed_is_refused_where_required);
    run!(where_the_sandbox_is_not_required_s0_is_taken);
    done();
}
