//! light_kindle.c: front light through lipc (com.lab126.powerd), as the stock UI and KOReader do.

use crate::prelude::*;
use std::cell::RefCell;
use std::ffi::CString;

// ---- light.h (light.rs forwards here on the Kindle) ----

fn get_prop(prop: &str) -> i32 {
    let cmd = cformat!("lipc-get-prop -i com.lab126.powerd %s 2>/dev/null", prop);
    let Ok(ccmd) = CString::new(cmd) else { return -1 };
    // SAFETY: popen gets two NUL-terminated strings; the stream it returns is read with fgetc and
    // closed with pclose below, and nothing else holds it.
    unsafe {
        let p = libc::popen(ccmd.as_ptr(), c"r".as_ptr());
        if p.is_null() {
            return -1;
        }
        let v = scan_int(p);
        libc::pclose(p);
        v
    }
}

/// fscanf(p, "%d", &v) == 1 ? v : -1: white space, an optional sign, the digits.
///
/// # Safety
/// p is an open stream.
unsafe fn scan_int(p: *mut libc::FILE) -> i32 {
    let mut c = libc::fgetc(p);
    while c != libc::EOF && (c == b' ' as i32 || (9..=13).contains(&c)) {
        c = libc::fgetc(p);
    }
    let mut neg = false;
    if c == b'-' as i32 || c == b'+' as i32 {
        neg = c == b'-' as i32;
        c = libc::fgetc(p);
    }
    if !(b'0' as i32..=b'9' as i32).contains(&c) {
        return -1;
    }
    let mut v: i32 = 0;
    while (b'0' as i32..=b'9' as i32).contains(&c) {
        v = v.wrapping_mul(10).wrapping_add(c - b'0' as i32);
        c = libc::fgetc(p);
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// The environment, for posix_spawn (the libc crate has no `environ` on musl or macOS).
#[cfg(target_vendor = "apple")]
fn environ() -> *const *mut libc::c_char {
    // SAFETY: _NSGetEnviron returns a pointer to the process's environ variable.
    unsafe { *libc::_NSGetEnviron() as *const *mut libc::c_char }
}

/// The environment, for posix_spawn (the libc crate has no `environ` on musl or macOS).
#[cfg(not(target_vendor = "apple"))]
fn environ() -> *const *mut libc::c_char {
    extern "C" {
        static environ: *const *mut libc::c_char;
    }
    // SAFETY: environ is the C library's own variable, valid for the process's life.
    unsafe { environ }
}

/// The shells run_bg started and has not reaped yet. Each is reaped by its pid as it ends (never
/// waitpid(-1): the background worker and Live view reap their own children).
const BG_SHELLS: usize = 8;

/// The levels as last read or set. Each read is two synchronous lipc-get-prop runs (95-191 ms on the
/// Scribe 2, at every Display sheet open and Live view start until 2.0.0-beta.5), and while they run
/// nothing reads the pens (no work pump can run inside a popen). Nothing else changes the light while
/// Grimoire runs (the Kindle UI and KOReader are paused, and the power service gives the same level
/// back after its screensaver), so it is read once, before the pens are opened (main.c).
struct State {
    shells: [libc::pid_t; BG_SHELLS],
    intensity: i32,
    warmth: i32,
    /// the battery as the power service last said, and when it was last asked (now_ms, 0 never):
    /// sys_battery's fallback
    battery: Battery,
    battery_at: u32,
    /// the reading under way, if one is (battery_ask_start)
    ask: BatteryAsk,
}

/// A battery reading asked of the power service in the background: the shell's pid (0 for none),
/// the read end of what it prints, that so far, and when it was started.
#[derive(Clone, Copy)]
struct BatteryAsk {
    pid: libc::pid_t,
    fd: i32,
    out: [u8; 64],
    len: usize,
    at: u32,
}

const NO_ASK: BatteryAsk = BatteryAsk { pid: 0, fd: -1, out: [0; 64], len: 0, at: 0 };

thread_local! {
    static S: RefCell<State> = const {
        RefCell::new(State {
            shells: [0; BG_SHELLS],
            intensity: -1,
            warmth: -1,
            battery: Battery { percent: -1, charging: false },
            battery_at: 0,
            ask: NO_ASK,
        })
    };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

fn reap_shells() {
    with(|s| {
        for i in 0..BG_SHELLS {
            // SAFETY: waitpid on a pid we spawned, status not wanted.
            if s.shells[i] > 0 && unsafe { libc::waitpid(s.shells[i], std::ptr::null_mut(), libc::WNOHANG) } != 0 {
                s.shells[i] = 0; // ended (or not ours now)
            }
        }
    })
}

/// Runs a shell command without waiting for it. Writing to the power service takes tens of ms; doing
/// that on the input path made the first touches laggy. posix_spawn starts it without copying
/// Grimoire (musl's shares the memory until the exec): the double fork used before copied the page
/// tables of a 60-100 MB process twice, on every light slider step (each 200 ms) and once a minute
/// in the middle of a stroke (sys_activity). A shell that ended is reaped at the next call, so at
/// most a few wait as zombies. When it cannot spawn, or eight are still running, the double fork.
fn run_bg(cmd: &str) {
    reap_shells();
    let Ok(ccmd) = CString::new(cmd) else { return };
    for i in 0..BG_SHELLS {
        if with(|s| s.shells[i]) > 0 {
            continue;
        }
        let argv: [*mut libc::c_char; 4] =
            [c"sh".as_ptr() as *mut _, c"-c".as_ptr() as *mut _, ccmd.as_ptr() as *mut _, std::ptr::null_mut()];
        let mut pid: libc::pid_t = 0;
        // SAFETY: posix_spawn gets NUL-terminated strings, a NULL-terminated argv (the strings
        // outlive the call) and the process's own environment; pid is written on success.
        if unsafe { libc::posix_spawn(&mut pid, c"/bin/sh".as_ptr(), std::ptr::null(), std::ptr::null(), argv.as_ptr(), environ()) } == 0 {
            with(|s| s.shells[i] = pid);
            return;
        }
        break;
    }
    // SAFETY: fork has no preconditions. The child forks once more and execs or _exits at once; the
    // grandchild execs /bin/sh with strings made before the fork. The intermediate child is reaped
    // below.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        qlog!("sys: cannot fork for: %s", cmd);
        return;
    }
    if pid == 0 {
        // SAFETY: in the child: fork, execl with NUL-terminated strings and a NULL-terminated list,
        // _exit on every path.
        unsafe {
            if libc::fork() == 0 {
                libc::execl(c"/bin/sh".as_ptr(), c"sh".as_ptr(), c"-c".as_ptr(), ccmd.as_ptr(), std::ptr::null::<libc::c_char>());
                libc::_exit(127);
            }
            libc::_exit(0);
        }
    }
    // SAFETY: waitpid on the child just forked.
    unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) }; // the intermediate child exits at once
}

fn set_prop(prop: &str, v: i32) {
    let cmd = cformat!("lipc-set-prop -i com.lab126.powerd %s %d >/dev/null 2>&1", prop, clampi(v, 0, LIGHT_MAX));
    run_bg(&cmd);
}

pub fn light_available() -> bool {
    // SAFETY: access reads the NUL-terminated paths.
    unsafe { libc::access(c"/usr/bin/lipc-set-prop".as_ptr(), libc::X_OK) == 0 || libc::access(c"/bin/lipc-set-prop".as_ptr(), libc::X_OK) == 0 }
}

pub fn light_refresh() {
    let intensity = get_prop("flIntensity");
    let warmth = get_prop("currentAmberLevel");
    with(|s| {
        s.intensity = intensity;
        s.warmth = warmth;
    })
}

pub fn light_get(intensity: &mut i32, warmth: &mut i32) {
    if with(|s| s.intensity) < 0 {
        light_refresh(); // never read, or the power service did not answer then
    }
    with(|s| {
        *intensity = s.intensity;
        *warmth = s.warmth;
    })
}

pub fn light_set_intensity(level: i32) {
    set_prop("flIntensity", level);
    with(|s| s.intensity = clampi(level, 0, LIGHT_MAX))
}

pub fn light_set_warmth(level: i32) {
    set_prop("currentAmberLevel", level);
    with(|s| s.warmth = clampi(level, 0, LIGHT_MAX))
}

pub fn light_warmth() -> i32 {
    with(|s| s.warmth)
}

pub fn sys_request_sleep() {
    qlog!("sleep: asking the power service to sleep (preventScreenSaver 0, then powerButton 1)");
    // One shell, in this order: the power service ignores the button while preventScreenSaver is
    // 1, and two shells of their own could reach it either way round.
    run_bg(concat!(
        "lipc-set-prop -i com.lab126.powerd preventScreenSaver 0 >/dev/null 2>&1;",
        " lipc-set-prop -i com.lab126.powerd powerButton 1 >/dev/null 2>&1"
    ));
}

/// Runs a shell command and keeps the first line it prints (trimmed), waiting at most timeout_ms:
/// a wedged lipc must not hold up Grimoire. The line (cut to cap - 1 bytes) with a non-empty line, else None.
fn run_line(cmd: &str, cap: usize, timeout_ms: i32) -> Option<Vec<u8>> {
    let mut p = [0i32; 2];
    let ccmd = CString::new(cmd).ok()?;
    // SAFETY: pipe writes the two descriptors into our array.
    if cap == 0 || unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: fork has no preconditions; the child sets up its descriptors and execs (or _exits).
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        // SAFETY: closing the two descriptors pipe just made.
        unsafe {
            libc::close(p[0]);
            libc::close(p[1]);
        }
        return None;
    }
    if pid == 0 {
        // SAFETY: in the child: setpgid/dup2/close on its own descriptors, execl with
        // NUL-terminated strings and a NULL-terminated list, _exit if it fails.
        unsafe {
            libc::setpgid(0, 0); // its own group, so a kill also reaches what the shell started
            libc::dup2(p[1], 1);
            libc::close(p[0]);
            libc::close(p[1]);
            libc::execl(c"/bin/sh".as_ptr(), c"sh".as_ptr(), c"-c".as_ptr(), ccmd.as_ptr(), std::ptr::null::<libc::c_char>());
            libc::_exit(127);
        }
    }
    // SAFETY: setpgid on the child just forked, close of the write end the parent does not use.
    unsafe {
        libc::setpgid(pid, pid); // here too, so the group exists even if the child has not run yet
        libc::close(p[1]);
    }
    let mut buf = [0u8; 128];
    let mut len = 0usize;
    let until = now_ms().wrapping_add(timeout_ms as u32);
    loop {
        let left = until.wrapping_sub(now_ms()) as i32;
        if left <= 0 {
            break;
        }
        let mut pf = libc::pollfd { fd: p[0], events: libc::POLLIN, revents: 0 };
        // SAFETY: poll reads and writes the one pollfd we pass with its count.
        let r = unsafe { libc::poll(&mut pf, 1, left) };
        if r < 0 && errno() == libc::EINTR {
            continue;
        }
        if r <= 0 {
            break;
        }
        // SAFETY: read into buf[len..127]: at most sizeof buf - 1 - len bytes, inside the array.
        let n = unsafe { libc::read(p[0], buf[len..].as_mut_ptr() as *mut libc::c_void, buf.len() - 1 - len) };
        if n <= 0 {
            break; // end of output: the command is done
        }
        len += n as usize;
        if buf[..len].contains(&b'\n') || len >= buf.len() - 1 {
            break;
        }
    }
    // SAFETY: close of the read end we own; waitpid/kill on the child just forked (its group is its
    // pid), the second kill for the case setpgid did not run yet.
    unsafe {
        libc::close(p[0]);
        if libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) == 0 {
            // still running (too slow, or more to say): stop it
            libc::kill(-pid, libc::SIGKILL);
            libc::kill(pid, libc::SIGKILL);
            libc::waitpid(pid, std::ptr::null_mut(), 0);
        }
    }
    // (The buffer is a C string from here: it ends at a NUL byte.)
    let text = &buf[..buf[..len].iter().position(|&c| c == 0).unwrap_or(len)];
    let isspace = |c: u8| c == b' ' || (9..=13).contains(&c);
    let start = text.iter().position(|&c| !isspace(c)).unwrap_or(text.len());
    let s = &text[start..];
    let mut n = s.iter().position(|&c| c == b'\r' || c == b'\n').unwrap_or(s.len());
    while n > 0 && isspace(s[n - 1]) {
        n -= 1;
    }
    if n == 0 {
        return None;
    }
    if n >= cap {
        n = cap - 1;
    }
    Some(s[..n].to_vec())
}

pub fn sys_powerd_state(out: &mut String, cap: usize) -> i32 {
    match run_line("lipc-get-prop -s com.lab126.powerd state 2>/dev/null", cap, 1500) {
        Some(line) => {
            // (The service's words are ASCII.)
            *out = String::from_utf8_lossy(&line).into_owned();
            0
        }
        None => -1,
    }
}

pub fn sys_prevent_sleep(prevent: bool) {
    let cmd = cformat!("lipc-set-prop -i com.lab126.powerd preventScreenSaver %d >/dev/null 2>&1", if prevent { 1 } else { 0 });
    run_bg(&cmd);
    qlog!("sleep: preventScreenSaver %d", prevent);
}

pub fn sys_activity() {
    // Same as KOReader: resets the power service's idle (screensaver) timer.
    run_bg("lipc-set-prop -i com.lab126.powerd touchScreenSaverTimeout 1 >/dev/null 2>&1");
}

pub fn sys_kindle_ui(run: bool) -> bool {
    let stopped = std::env::var_os("GRIMOIRE_AWESOME_STOPPED");
    let pillow = std::env::var_os("GRIMOIRE_PILLOW_DISABLED");
    let wm = stopped.is_some_and(|v| v == "yes");
    let pi = pillow.is_some_and(|v| v == "yes");
    if !wm && !pi {
        return true;
    }
    // Synchronous at the lock boundary only: the UI must finish pausing before we draw again.
    let cmd = cformat!(
        "%s%s echo ready",
        if run {
            if wm { "killall -CONT awesome 2>/dev/null || exit 1; " } else { "" }
        } else if pi {
            "lipc-set-prop com.lab126.pillow disableEnablePillow disable >/dev/null 2>&1 || exit 1; "
        } else {
            ""
        },
        if run {
            if pi { "lipc-set-prop com.lab126.pillow disableEnablePillow enable >/dev/null 2>&1 || exit 1; " } else { "" }
        } else if wm {
            "killall -STOP awesome 2>/dev/null || exit 1; "
        } else {
            ""
        }
    );
    run_line(&cmd, 32, 3000).is_some_and(|answer| answer == b"ready")
}

/// The kernel's power supplies, one directory each.
const POWER_SUPPLY_DIR: &str = "/sys/class/power_supply";
/// lipc is asked for the battery at most this often.
const BATTERY_LIPC_MS: u32 = 60000;
/// A reading that has not ended in this long is stopped, and the battery is unknown until the next
/// (a lipc run took 95-191 ms on the Scribe 2, State says; the synchronous reading this replaces
/// gave each of its two 1 s).
const BATTERY_ASK_MS: u32 = 2000;
/// What the shell asks: the percent, then whether it charges, each on a line of its own even when
/// lipc says nothing (an empty line then).
const BATTERY_ASK: &std::ffi::CStr = c"echo \"$(lipc-get-prop -i com.lab126.powerd battLevel 2>/dev/null)\"; \
echo \"$(lipc-get-prop -i com.lab126.powerd isCharging 2>/dev/null)\"";

/// Starts a reading in the background: a shell (posix_spawn, as run_bg: Grimoire is not copied)
/// whose output is a pipe, in a process group of its own, so that stopping it stops lipc too.
/// Nothing waits for it: battery_ask_end looks at it each time sys_battery is asked.
fn battery_ask_start() {
    let mut p = [0i32; 2];
    // SAFETY: pipe writes the two descriptors into our array.
    if unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
        return;
    }
    let argv: [*mut libc::c_char; 4] =
        [c"sh".as_ptr() as *mut _, c"-c".as_ptr() as *mut _, BATTERY_ASK.as_ptr() as *mut _, std::ptr::null_mut()];
    let mut pid: libc::pid_t = 0;
    // SAFETY: the file actions and attributes are initialized before use and destroyed after;
    // posix_spawn gets NUL-terminated strings, a NULL-terminated argv (static strings) and the
    // process's own environment, and writes pid on success. The child's stdout becomes the pipe's
    // write end, and both of the pipe's descriptors are closed in it.
    let rc = unsafe {
        let mut fa: libc::posix_spawn_file_actions_t = std::mem::zeroed();
        let mut attr: libc::posix_spawnattr_t = std::mem::zeroed();
        libc::posix_spawn_file_actions_init(&mut fa);
        libc::posix_spawnattr_init(&mut attr);
        libc::posix_spawn_file_actions_adddup2(&mut fa, p[1], 1);
        libc::posix_spawn_file_actions_addclose(&mut fa, p[0]);
        libc::posix_spawn_file_actions_addclose(&mut fa, p[1]);
        libc::posix_spawnattr_setflags(&mut attr, libc::POSIX_SPAWN_SETPGROUP as libc::c_short);
        libc::posix_spawnattr_setpgroup(&mut attr, 0);
        let rc = libc::posix_spawn(&mut pid, c"/bin/sh".as_ptr(), &fa, &attr, argv.as_ptr(), environ());
        libc::posix_spawn_file_actions_destroy(&mut fa);
        libc::posix_spawnattr_destroy(&mut attr);
        rc
    };
    // SAFETY: closing the write end, which only the child uses; the read end, kept, reads without
    // blocking and is not handed to anything run later.
    unsafe {
        libc::close(p[1]);
        if rc != 0 {
            libc::close(p[0]);
            return;
        }
        libc::fcntl(p[0], libc::F_SETFL, libc::fcntl(p[0], libc::F_GETFL, 0) | libc::O_NONBLOCK);
        libc::fcntl(p[0], libc::F_SETFD, libc::FD_CLOEXEC);
    }
    with(|s| s.ask = BatteryAsk { pid, fd: p[0], at: now_ms(), ..NO_ASK });
}

/// What the reading printed so far, read without waiting.
fn battery_ask_read(s: &mut State) {
    while s.ask.len < s.ask.out.len() {
        let len = s.ask.len;
        // SAFETY: read into out[len..], at most its remaining length, from the pipe's read end.
        let n = unsafe { libc::read(s.ask.fd, s.ask.out[len..].as_mut_ptr() as *mut libc::c_void, s.ask.out.len() - len) };
        if n <= 0 {
            break;
        }
        s.ask.len += n as usize;
    }
}

/// The reading under way, looked at without waiting: once it has ended, what it says (the percent
/// -1 when lipc gave none, or when it took longer than BATTERY_ASK_MS and was stopped). None while it
/// runs, or when none is under way.
fn battery_ask_end() -> Option<Battery> {
    let ask = with(|s| {
        if s.ask.pid > 0 {
            battery_ask_read(s);
        }
        s.ask
    });
    if ask.pid <= 0 {
        return None;
    }
    // SAFETY: waitpid on the shell battery_ask_start spawned, by its pid, without waiting.
    let ended = unsafe { libc::waitpid(ask.pid, std::ptr::null_mut(), libc::WNOHANG) } != 0; // (or not ours any more)
    let late = !ended && now_ms().wrapping_sub(ask.at) >= BATTERY_ASK_MS;
    if !ended && !late {
        return None;
    }
    if late {
        // SAFETY: SIGKILL to the shell's group and to the shell (its group is its pid), then the wait
        // for it, which a killed process ends at once.
        unsafe {
            libc::kill(-ask.pid, libc::SIGKILL);
            libc::kill(ask.pid, libc::SIGKILL);
            libc::waitpid(ask.pid, std::ptr::null_mut(), 0);
        }
    }
    let out = with(|s| {
        battery_ask_read(s); // (what it printed before it ended)
        // SAFETY: closing the read end this reading owns.
        unsafe { libc::close(s.ask.fd) };
        let out = s.ask.out[..s.ask.len].to_vec();
        s.ask = NO_ASK;
        out
    });
    if late {
        return Some(Battery::default());
    }
    let mut lines = out.split(|&c| c == b'\n');
    let percent = lines.next().and_then(battery_parse_number).filter(|p| (0..=100).contains(p)).unwrap_or(-1);
    let charging = percent >= 0 && lines.next().and_then(battery_parse_number).is_some_and(|c| c != 0);
    Some(Battery { percent, charging })
}

pub fn sys_battery() -> Battery {
    // The kernel first: a few small reads, nothing to wait for.
    if let Some(b) = battery_from_sysfs(POWER_SUPPLY_DIR) {
        return b;
    }
    // Else the power service, asked in the background (each lipc run takes tens of ms or more, and
    // nothing reads the pens while the main thread waits): what it last said, until a new reading
    // ends. One is started at most once a minute, and never while another is under way.
    if let Some(b) = battery_ask_end() {
        with(|s| s.battery = b);
    }
    let due = with(|s| s.ask.pid <= 0 && (s.battery_at == 0 || now_ms().wrapping_sub(s.battery_at) >= BATTERY_LIPC_MS));
    if due {
        with(|s| s.battery_at = now_ms() | 1);
        battery_ask_start();
    }
    with(|s| s.battery)
}

pub fn sys_wall_clock() -> i64 {
    wall_time()
}
