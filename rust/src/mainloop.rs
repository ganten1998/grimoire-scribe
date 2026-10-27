//! main.c: entry point for the Kindle build and the desktop simulator. (In Rust: this module, which
//! src/bin/grimoire.rs and src/bin/grimoire-headless.rs call; a module named main.rs would read as
//! a binary to every Rust tool.)
//!
//! The command line is the C's: --version prints the version, --help (-h) the usage; everything
//! else comes from the environment (GRIMOIRE_ROOT, and the launcher's and the runner's GRIMOIRE_*
//! variables, which the modules that read them document). kindle/app/grimoire.sh takes its own
//! flags (--hosted koreader, --annotate, --book, --stop-framework, --reset-calibration) and passes
//! what Grimoire needs as GRIMOIRE_HOST, GRIMOIRE_ANNOTATE and GRIMOIRE_BOOK.

use crate::prelude::*;

#[cfg(feature = "kindle")]
// The launcher's watchdog kills Grimoire and restores the Kindle UI if this file goes stale,
// so a hang can never leave the device stuck with its UI paused.
const HEARTBEAT: &str = "/tmp/grimoire.alive";

#[cfg(feature = "kindle")]
fn heartbeat() {
    todo!()
}

#[cfg(feature = "kindle")]
// The value of the first "<key> : value" line of a /proc file, trimmed ("?" if absent).
fn proc_value(file: &str, key: &str, out: &mut String, cap: usize) {
    todo!()
}

#[cfg(feature = "kindle")]
// What the device is, at the top of grimoire.log, so a tester's log answers the questions about
// their Kindle. Nothing that identifies the unit or a network: people attach this log to public
// issues, so /proc/usid gives only its first 6 characters (the model code, not the serial number).
fn log_device() {
    todo!()
}

#[cfg(feature = "kindle")]
// The loop's watchdog: a phase that held the thread over 50 ms (beyond the wait it is allowed) gets
// a line. Outside the work pump's reach (common.h: a popen, a fork, an fsync, code that does not pump)
// nothing reads the pens meanwhile, and the kernel keeps only 50-175 ms of them
// (docs/INK-PERFORMANCE.md, Reader stalls).
fn phase_done(name: &str, since: &mut u32, wait_ms: u32) {
    todo!()
}

#[cfg(feature = "kindle")]
// What the watchdog calls an event (InType's order) and a screen (ScreenId's): a slow batch names its
// slowest event, which tells a page turn from a sheet opening. Never a key's value or a place.
fn event_name(t: i32) -> &'static str {
    const K: [&str; 24] = [
        "none",
        "pen down",
        "pen move",
        "pen up",
        "pen hover",
        "pen leave",
        "touch down",
        "touch move",
        "touch up",
        "power",
        "quit",
        "pen button",
        "orientation",
        "pinch begin",
        "pinch move",
        "pinch end",
        "sleep",
        "wake",
        "power up",
        "key",
        "power held",
        "controller or keyboard came",
        "controller or keyboard went",
        "key up",
    ];
    const _: () = assert!(K.len() == InType::KeyUp as usize + 1, "an event name for each InType");
    if t >= 0 && t <= InType::KeyUp as i32 {
        K[t as usize]
    } else {
        "?"
    }
}

#[cfg(feature = "kindle")]
fn screen_name(s: i32) -> &'static str {
    const K: [&str; ScreenId::COUNT] = ["library", "editor", "pen feel", "calibration", "share"];
    if s >= 0 && (s as usize) < ScreenId::COUNT {
        K[s as usize]
    } else {
        "?"
    }
}

/// The event batch's slowest event: how long, which, and the screen it came to.
#[derive(Clone, Copy, Debug, Default)]
struct Slowest {
    ms: u32,
    type_: i32,
    scr: i32,
    n: u32,
}

fn event_timed(a: &mut App, ev: &InEvent, slow: &mut Slowest) {
    todo!()
}

#[cfg(feature = "kindle")]
fn batch_done(since: &mut u32, slow: &Slowest) {
    todo!()
}

#[cfg(feature = "sim")]
// The fit check (make l10n-check runs every screen with GRIMOIRE_CHECK_FIT=1): each text cut
// short to fit its place is listed once (per text and room), and the run ends with status 3.
fn fit_report(text: &str, need: i32, have: i32) {
    todo!()
}

fn usage() {
    #[cfg(feature = "kindle")]
    let root = "/mnt/us/grimoire";
    #[cfg(not(feature = "kindle"))]
    let root = "./grimoire-data";
    print!(
        "{}",
        cformat!(
            "Grimoire %s: a notebook for the Kindle Scribe\n  env GRIMOIRE_ROOT   data directory (default %s)\n  --version        print the version\n",
            GRIMOIRE_VERSION,
            root
        )
    );
}

/// main(): the exit status. args as std::env::args() gives them (args[0] the program).
pub fn main(args: Vec<String>) -> i32 {
    // Rust's runtime ignores SIGPIPE before main; a C program starts with the default action, and
    // Grimoire relies on that: httpd.c ignores it only once the share server starts, and every
    // child forked before then (lipc through the shell, bluetooth.sh, update.sh) inherits what this
    // process has. So the C's starting state first.
    // SAFETY: signal() with SIG_DFL, before any thread or child exists.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    for a in args.iter().skip(1) {
        if a == "--version" {
            println!("{GRIMOIRE_VERSION}");
            return 0;
        }
        if a == "--help" || a == "-h" {
            usage();
            return 0;
        }
    }
    run()
}

// Everything after the command line: the data root, the language, the display and input, the App,
// and the loop (main.c's main from getenv("GRIMOIRE_ROOT") on). In the headless build the loop's
// poll is sim_sdl::input_poll_app, which the App is handed for the runner's expect checks.
fn run() -> i32 {
    todo!()
}
