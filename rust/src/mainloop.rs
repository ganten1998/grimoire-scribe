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
    let Ok(path) = std::ffi::CString::new(HEARTBEAT) else { return };
    // SAFETY: open/write/close on a NUL-terminated path we own and the fd open returns; write reads
    // one byte of a static.
    unsafe {
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            if libc::write(fd, b"1".as_ptr() as *const libc::c_void, 1) < 0 { /* best effort */ }
            libc::close(fd);
        }
    }
}

#[cfg(feature = "kindle")]
/// fgets(line, cap, f) on a /proc file: the next line with its '\n', or the next cap - 1 bytes of a
/// longer one; None at the end or on an error.
fn fgets(f: &mut std::io::BufReader<std::fs::File>, cap: usize) -> Option<Vec<u8>> {
    use std::io::BufRead;
    let mut line = Vec::new();
    while line.len() + 1 < cap {
        let buf = match f.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        if buf.is_empty() {
            break;
        }
        let room = &buf[..buf.len().min(cap - 1 - line.len())];
        let (n, nl) = match room.iter().position(|&c| c == b'\n') {
            Some(i) => (i + 1, true),
            None => (room.len(), false),
        };
        line.extend_from_slice(&room[..n]);
        f.consume(n);
        if nl {
            break;
        }
    }
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

#[cfg(feature = "kindle")]
/// isspace() in the C locale: space, \t, \n, \v, \f, \r.
fn isspace(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}

#[cfg(feature = "kindle")]
// The value of the first "<key> : value" line of a /proc file, trimmed ("?" if absent).
fn proc_value(file: &str, key: &str, out: &mut String, cap: usize) {
    strlcpy_q(out, "?", cap);
    let Ok(f) = std::fs::File::open(file) else { return };
    let mut f = std::io::BufReader::new(f);
    let kl = key.len();
    while let Some(raw) = fgets(&mut f, 256) {
        // (as the C's string functions see the line: up to its first NUL)
        let line = &raw[..raw.iter().position(|&c| c == 0).unwrap_or(raw.len())];
        let at = |i: usize| -> u8 { line.get(i).copied().unwrap_or(0) };
        if !line.starts_with(key.as_bytes()) || (at(kl) != b':' && !isspace(at(kl))) {
            continue;
        }
        let Some(colon) = line.iter().position(|&c| c == b':') else { continue };
        let mut v = colon + 1;
        while isspace(at(v)) {
            v += 1;
        }
        let rest = &line[v.min(line.len())..];
        let end = rest.iter().position(|&c| c == b'\r' || c == b'\n').unwrap_or(rest.len());
        // (/proc's text is ASCII; out is a String)
        strlcpy_q(out, &String::from_utf8_lossy(&rest[..end]), cap);
        break;
    }
}

#[cfg(feature = "kindle")]
// What the device is, at the top of grimoire.log, so a tester's log answers the questions about
// their Kindle. Nothing that identifies the unit or a network: people attach this log to public
// issues, so /proc/usid gives only its first 6 characters (the model code, not the serial number).
fn log_device() {
    use std::io::Read;
    let mut code = String::from("?");
    if let Ok(f) = std::fs::File::open("/proc/usid") {
        // fread(code, 1, 6, f)
        let mut b = Vec::with_capacity(6);
        let _ = f.take(6).read_to_end(&mut b);
        code = b.iter().map(|&c| if c.is_ascii_alphanumeric() { c as char } else { '?' }).collect();
    }
    let (mut total, mut avail, mut part, mut hw) = (String::new(), String::new(), String::new(), String::new());
    proc_value("/proc/meminfo", "MemTotal", &mut total, 32);
    proc_value("/proc/meminfo", "MemAvailable", &mut avail, 32);
    proc_value("/proc/cpuinfo", "CPU part", &mut part, 32);
    proc_value("/proc/cpuinfo", "Hardware", &mut hw, 64);
    qlog!("device: model code %s (the first 6 characters of /proc/usid)", if !code.is_empty() { code.as_str() } else { "?" });
    qlog!("device: memory %s total, %s available; CPU part %s, hardware %s", &total, &avail, &part, &hw);
}

#[cfg(feature = "kindle")]
// The loop's watchdog: a phase that held the thread over 50 ms (beyond the wait it is allowed) gets
// a line. Outside the work pump's reach (common.h: a popen, a fork, an fsync, code that does not pump)
// nothing reads the pens meanwhile, and the kernel keeps only 50-175 ms of them
// (docs/INK-PERFORMANCE.md, Reader stalls).
fn phase_done(name: &str, since: &mut u32, wait_ms: u32) {
    let t = now_ms();
    let took = t.wrapping_sub(*since);
    if took > wait_ms.wrapping_add(50) {
        qlog!("main: %s took %u ms", name, took);
    }
    *since = t;
}

// PHASE(name, since, wait_ms): the watchdog's line on the Kindle, nothing elsewhere.
#[cfg(feature = "kindle")]
macro_rules! phase {
    ($name:expr, $since:expr, $wait_ms:expr) => {
        phase_done($name, $since, $wait_ms)
    };
}
#[cfg(not(feature = "kindle"))]
macro_rules! phase {
    ($name:expr, $since:expr, $wait_ms:expr) => {{
        let _ = $since;
    }};
}

#[cfg(feature = "kindle")]
// What the watchdog calls an event (InType's order) and a screen (ScreenId's): a slow batch names its
// slowest event, which tells a page turn from a sheet opening. Never a key's value or a place.
fn event_name(t: i32) -> &'static str {
    const K: [&str; 25] = [
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
        "gesture",
    ];
    const _: () = assert!(K.len() == InType::Gesture as usize + 1, "an event name for each InType");
    if t >= 0 && t <= InType::Gesture as i32 {
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
// (which and where are read only by the Kindle's watchdog line, batch_done)
#[cfg_attr(not(feature = "kindle"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default)]
struct Slowest {
    ms: u32,
    type_: i32,
    scr: i32,
    n: u32,
}

fn event_timed(a: &mut App, ev: &InEvent, slow: &mut Slowest) {
    let t0 = now_ms();
    let scr = a.scr as i32;
    app_event(a, ev);
    let took = now_ms().wrapping_sub(t0);
    // (the C's `!slow->n++ || ...`: the count goes up before the new slowest takes it)
    let first = slow.n == 0;
    slow.n = slow.n.wrapping_add(1);
    if first || took > slow.ms {
        *slow = Slowest { ms: took, type_: ev.type_ as i32, scr, n: slow.n };
    }
}

#[cfg(feature = "kindle")]
fn batch_done(since: &mut u32, slow: &Slowest) {
    let t = now_ms();
    let took = t.wrapping_sub(*since);
    if took > 50 {
        qlog!(
            "main: the event batch took %u ms (slowest: %s on the %s screen, %u ms, of %u events)",
            took,
            event_name(slow.type_),
            screen_name(slow.scr),
            slow.ms,
            slow.n
        );
    }
    *since = t;
}

// BATCH_DONE(since, slow): the same.
#[cfg(feature = "kindle")]
macro_rules! batch_done {
    ($since:expr, $slow:expr) => {
        batch_done($since, $slow)
    };
}
#[cfg(not(feature = "kindle"))]
macro_rules! batch_done {
    ($since:expr, $slow:expr) => {{
        let _ = ($since, $slow);
    }};
}

#[cfg(feature = "sim")]
// The fit check's books: the texts and rooms listed so far (FNV-1a hashes, 512 at most) and how
// many cuts were listed.
#[derive(Default)]
struct Fit {
    cuts: i32,
    seen: Vec<u32>,
}

#[cfg(feature = "sim")]
thread_local! {
    static FIT: std::cell::RefCell<Fit> = std::cell::RefCell::new(Fit::default());
}

#[cfg(feature = "sim")]
// The fit check (make l10n-check runs every screen with GRIMOIRE_CHECK_FIT=1): each text cut
// short to fit its place is listed once (per text and room), and the run ends with status 3.
fn fit_report(text: &str, need: i32, have: i32) {
    // (as the C's string functions see the text: up to a NUL)
    let t = &text[..text.find('\0').unwrap_or(text.len())];
    let mut h: u32 = 2166136261u32 ^ have as u32; // FNV-1a of the room and the text
    for &p in t.as_bytes() {
        h = (h ^ p as u32).wrapping_mul(16777619);
    }
    let listed = FIT.with(|f| {
        let mut f = f.borrow_mut();
        if f.seen.contains(&h) {
            return false;
        }
        if f.seen.len() < 512 {
            f.seen.push(h);
        }
        f.cuts += 1;
        true
    });
    if !listed {
        return;
    }
    use std::io::Write;
    let _ = std::io::stderr()
        .write_all(cformat!("fit[%s]: \"%s\" needs %d px, has %d\n", i18n_current(), t, need, have).as_bytes());
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

// The main loop's poll: in the headless build the runner's, which is handed the App for its expect
// checks (sim_sdl::input_poll_app; the C's app_probe finds it through g_probe_app).
#[cfg(feature = "sim")]
fn poll(ev: &mut InEvent, timeout_ms: i32, a: &mut App) -> i32 {
    crate::sim_sdl::input_poll_app(ev, timeout_ms, a)
}
#[cfg(not(feature = "sim"))]
fn poll(ev: &mut InEvent, timeout_ms: i32, a: &mut App) -> i32 {
    let _ = a;
    input_poll(ev, timeout_ms)
}

// Everything after the command line: the data root, the language, the display and input, the App,
// and the loop (main.c's main from getenv("GRIMOIRE_ROOT") on). In the headless build the loop's
// poll is sim_sdl::input_poll_app, which the App is handed for the runner's expect checks.
fn run() -> i32 {
    #[cfg(feature = "kindle")]
    const DEFAULT_ROOT: &str = "/mnt/us/grimoire";
    #[cfg(not(feature = "kindle"))]
    const DEFAULT_ROOT: &str = "./grimoire-data";
    // (store_init takes a str: a root in bytes that are not UTF-8 reaches it with U+FFFD)
    let root: String = match std::env::var_os("GRIMOIRE_ROOT") {
        Some(r) if !r.is_empty() => r.to_string_lossy().into_owned(),
        _ => DEFAULT_ROOT.to_string(),
    };
    qlog!("Grimoire %s starting, data in %s", GRIMOIRE_VERSION, &root);
    #[cfg(feature = "kindle")]
    log_device();
    if store_init(&root) != 0 {
        return 1;
    }
    // The language, before anything is drawn or said: Settings > Language (settings.ini), and
    // for "auto" (the default) the Kindle's own (i18n.h says where that is found).
    i18n_detect();
    let mut lang = String::new();
    settings_language(&mut lang, 16);
    i18n_apply(&lang);
    #[cfg(feature = "sim")]
    {
        let check_fit = std::env::var_os("GRIMOIRE_CHECK_FIT");
        if check_fit.is_some_and(|v| v == "1") {
            font_set_overflow_hook(Some(fit_report));
        }
    }

    let (mut w, mut h, mut dpi) = (0i32, 0i32, 0f32);
    if display_open(&mut w, &mut h, &mut dpi) != 0 {
        qlog!("display_open failed");
        return 1;
    }
    qlog!("display %dx%d @ %.0f dpi", w, h, dpi);
    // The front light's levels, once: asking takes 95-191 ms on the Kindle, nothing can read the pens
    // meanwhile, and they are not open yet (light.h).
    light_refresh();
    if input_open(w, h, dpi, None, None) != 0 {
        qlog!("input_open failed");
        display_close();
        return 1;
    }
    // (the C's static App: one per process, owned here; PORTING.md, The App)
    let mut app: Box<App> = Box::default();
    if app_init(&mut app, w, h, dpi) != 0 {
        qlog!("app_init failed");
        input_close();
        display_close();
        return 1;
    }
    app_flush(&mut app);
    qlog!("first frame presented");
    // The Kindle UI may repaint once more after we start (the scriptlet "book" closing, a
    // late home-screen refresh). Check the panel still shows us; if not, repaint everything.
    let started = now_ms();
    let mut last_guard = started;
    let (mut guards, mut guard_streak, mut startup_repaints, mut status) = (0i32, 0i32, 0i32, 0i32);
    let mut guard_on = true;
    #[cfg(feature = "kindle")]
    heartbeat();
    #[cfg(feature = "kindle")]
    let mut last_beat = now_ms();

    while app.running {
        // A panel that took none of the first five refreshes is not Grimoire's to drive (a device
        // FBInk does not know, say): leave, so the launcher gives the Kindle back its screen, pen
        // and touch, instead of a device that looks frozen. (app_shutdown below saves.)
        if display_broken() {
            qlog!("display: the first frame and the next 4 refreshes all failed; leaving so the Kindle UI comes back");
            status = 1;
            break;
        }
        let mut ev = InEvent::default();
        let mut phase = now_ms();
        // Batch nearby samples, but keep showing ink during a backlog. A costly brush or
        // a continuously readable device must not postpone refreshes and timers until the
        // queue empties. Leave remaining events queued, in order, for the next iteration.
        // (input_drained: the batch ended because nothing more was queued, not at its 64 events or
        // 8 ms: the prefetch and the Links bar's hold timer wait for a quiet moment.)
        if poll(&mut ev, 30, &mut app) != 0 {
            phase!("reading input", &mut phase, 30);
            let batch_at = now_ms();
            let mut count = 0u32;
            let mut slow = Slowest::default();
            loop {
                event_timed(&mut app, &ev, &mut slow);
                count += 1;
                if !(app.running && count < 64 && now_ms().wrapping_sub(batch_at) < 8) {
                    app.input_drained = false;
                    break;
                }
                if poll(&mut ev, 0, &mut app) == 0 {
                    app.input_drained = true;
                    break;
                }
            }
            batch_done!(&mut phase, &slow);
        } else {
            app.input_drained = true;
            phase!("reading input", &mut phase, 30);
        }
        app_tick(&mut app, now_ms());
        phase!("app_tick", &mut phase, 0);
        app_flush(&mut app);
        phase!("app_flush", &mut phase, 0);
        let t = now_ms();
        let every: u32 = if t.wrapping_sub(started) < 15000 { 700 } else { 1500 };
        if t.wrapping_sub(last_guard) >= every && !app.asleep && guard_on {
            last_guard = t;
            // The Kindle's status bar strip (clock, battery) lives at the top of the panel: check
            // that band densely, and repaint just it (no flash) when something drew there.
            let skip = if app.toast_visible { app.toast_rect } else { RECT_EMPTY };
            let top = Rect { x: 0, y: 0, w: app.w, h: ui_px(150.0) };
            let mut where_ = RECT_EMPTY;
            let before = guards;
            if display_find_overwrite(&app.screen, skip, &mut where_) {
                guards += 1;
                // Most of the screen: a clean full refresh. A patch: repaint just it, no flash.
                let most = where_.w as i64 * where_.h as i64 > app.w as i64 * app.h as i64 * 2 / 5;
                qlog!("screen guard: repainting %s (#%d)", if most { "everything" } else { "the patch" }, guards);
                if most {
                    app_invalidate_all(&mut app, RefreshMode::Full);
                } else {
                    app_invalidate(&mut app, where_, RefreshMode::Ui);
                }
                app_flush(&mut app);
            } else if display_band_overwritten(&app.screen, top, skip) {
                guards += 1;
                qlog!("screen guard: repainting the top band (#%d)", guards);
                app_invalidate(&mut app, top, RefreshMode::Ui);
                app_flush(&mut app);
            }
            // A repaint puts the panel right, so a mismatch at every check means the framebuffer
            // does not read back as drawn (a panel nobody here has tested): rather than repaint
            // forever, the guard stops.
            guard_streak = if guards > before { guard_streak + 1 } else { 0 };
            if guard_streak >= 10 {
                guard_on = false;
                qlog!("screen guard: off after 10 repaints in a row (the framebuffer does not read back as drawn)");
            }
            phase!("the screen guard", &mut phase, 0);
        }
        // Two unconditional repaints shortly after launch catch the Kindle UI's last frame (the
        // home screen strip that could land on top of Grimoire's first frame).
        if !app.asleep
            && ((startup_repaints == 0 && t.wrapping_sub(started) > 1200)
                || (startup_repaints == 1 && t.wrapping_sub(started) > 3500))
        {
            startup_repaints += 1;
            app_invalidate_all(&mut app, RefreshMode::Ui);
            app_flush(&mut app);
        }
        #[cfg(feature = "kindle")]
        if now_ms().wrapping_sub(last_beat) > 2000 {
            heartbeat();
            last_beat = now_ms();
        }
    }
    qlog!("exiting");
    app_shutdown(&mut app);
    display_wait();
    input_close();
    display_close(); // (a headless run whose expect checks failed exits here, with status 3)
    #[cfg(feature = "sim")]
    {
        let cuts = FIT.with(|f| f.borrow().cuts);
        if cuts != 0 {
            use std::io::Write;
            let _ = std::io::stderr()
                .write_all(cformat!("fit: %d text%s cut short to fit\n", cuts, if cuts == 1 { "" } else { "s" }).as_bytes());
            return 3;
        }
    }
    status
}

#[cfg(all(test, feature = "sim"))]
mod tests {
    use super::*;

    #[test]
    fn the_fit_check_lists_a_text_once_per_room() {
        FIT.with(|f| *f.borrow_mut() = Fit::default());
        fit_report("Settings", 300, 200);
        fit_report("Settings", 310, 200);
        fit_report("Settings", 300, 180);
        fit_report("Library", 300, 200);
        assert_eq!(FIT.with(|f| f.borrow().cuts), 3);
        // past 512 listed, a cut counts every time it comes
        for i in 0..600 {
            fit_report("x", 10, i);
        }
        fit_report("x", 10, 599);
        assert_eq!(FIT.with(|f| (f.borrow().cuts, f.borrow().seen.len())), (604, 512));
    }

}
