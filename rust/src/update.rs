//! update.h: Settings > Updates (docs/KPM.md, Updates). Grimoire's updates come through KPM, the
//! Kindle's package manager, from Grimoire's own feed: kindle/app/update.sh adds the chosen channel's
//! feed to KPM (and takes the other channel's away: KPM installs the newest Grimoire any feed it knows
//! lists), refreshes KPM's index and lists what it offers. KPM does the downloading (Grimoire has no
//! HTTPS of its own). An update itself cannot run inside Grimoire (KPM's hooks refuse while it runs):
//! Grimoire writes the request and leaves, and the launcher hands it to update.sh, which installs the
//! newest Grimoire with KPM once the launcher is gone, and starts it again.

use crate::prelude::*;
use std::cell::RefCell;
use std::ffi::CString;

pub const UPDATE_FEED_PREVIEW: &str = "https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json";
pub const UPDATE_FEED_STABLE: &str = "https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/stable/manifest.json";
pub const UPDATE_VERSION_LEN: usize = 24;

// ---- pure parts (tests/test_update.c) ----

/// A number from b[*p..] (digits up to max): *p moves past it. False when there is none or it passes max.
fn number(b: &[u8], p: &mut usize, out: &mut u32, max: u32) -> bool {
    let mut s = *p;
    if !(s < b.len() && b[s].is_ascii_digit()) {
        return false;
    }
    let mut v: u64 = 0;
    while s < b.len() && b[s].is_ascii_digit() {
        v = v * 10 + (b[s] - b'0') as u64;
        s += 1;
        if v > max as u64 {
            return false;
        }
    }
    *out = v as u32;
    *p = s;
    true
}

/// A Grimoire version as KPM orders it (tools/kpm-package.py): "2.0.0-beta.5" is 2.0.1005,
/// "2.0.0-rc.1" 2.0.5001, "2.0.0" 2.0.9999, "2.0.1" 2.0.19999. False for anything else.
pub fn update_version_key(version: &str, key: &mut [u32; 3]) -> bool {
    version_key(version.as_bytes(), key)
}

fn version_key(b: &[u8], key: &mut [u32; 3]) -> bool {
    // (A C string ends at its first NUL, and is read up to it.)
    let b = &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())];
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let mut p = 0usize;
    let (mut major, mut minor, mut patch) = (0u32, 0u32, 0u32);
    let mut stage: u32 = 9999;
    if !number(b, &mut p, &mut major, 9999) {
        return false;
    }
    let dot = at(p);
    p += 1;
    if dot != b'.' || !number(b, &mut p, &mut minor, 9999) {
        return false;
    }
    let dot = at(p);
    p += 1;
    if dot != b'.' || !number(b, &mut p, &mut patch, 9999) {
        return false;
    }
    if at(p) == b'-' {
        p += 1;
        let mut n = 0u32;
        if b[p..].starts_with(b"beta.") {
            stage = 1000;
            p += 5;
        } else if b[p..].starts_with(b"rc.") {
            stage = 5000;
            p += 3;
        } else {
            return false;
        }
        if at(p) == b'0' || !number(b, &mut p, &mut n, 999) || n < 1 {
            return false;
        }
        stage += n;
    }
    if at(p) != 0 {
        return false;
    }
    key[0] = major;
    key[1] = minor;
    key[2] = patch * 10000 + stage;
    true
}

/// <0, 0, >0 as version a is older than, the same as, or newer than b; unreadable ones are oldest.
pub fn update_version_cmp(a: &str, b: &str) -> i32 {
    let mut ka = [0u32; 3];
    let mut kb = [0u32; 3];
    let ra = update_version_key(a, &mut ka);
    let rb = update_version_key(b, &mut kb);
    if !ra || !rb {
        return ra as i32 - rb as i32;
    }
    for i in 0..3 {
        if ka[i] != kb[i] {
            return if ka[i] < kb[i] { -1 } else { 1 };
        }
    }
    0
}

/// A prerelease (beta or rc): its updates come from the preview channel unless told otherwise.
pub fn update_is_prerelease(version: &str) -> bool {
    version.contains('-')
}

/// The newest Grimoire in `kpm search grimoire`'s output (each package a line "- grimoire (Grimoire):
/// <description>", the description ending in its version; colour codes and other packages skipped).
/// False when it lists none.
pub fn update_latest(search_output: &str, version: &mut String /* at most UPDATE_VERSION_LEN - 1 bytes */) -> bool {
    latest_of(search_output.as_bytes(), version)
}

// update_latest on bytes: the check's output is whatever KPM printed, and need not be UTF-8.
fn latest_of(out: &[u8], version: &mut String) -> bool {
    version.clear();
    // (A C string ends at its first NUL.)
    let out = &out[..out.iter().position(|&c| c == 0).unwrap_or(out.len())];
    let mut found = false;
    let mut line = 0usize;
    while line < out.len() {
        let end = out[line..].iter().position(|&c| c == b'\n').map(|x| line + x);
        let len = end.map_or(out.len() - line, |e| e - line);
        let l = &out[line..line + len];
        // The line without colour codes (ESC [ ... letter).
        let mut buf: Vec<u8> = Vec::new();
        let mut i = 0usize;
        while i < len && buf.len() + 1 < 512 {
            if l[i] == 0x1b && i + 1 < len && l[i + 1] == b'[' {
                i += 2;
                while i < len && !l[i].is_ascii_alphabetic() {
                    i += 1;
                }
                i += 1;
                continue;
            }
            buf.push(l[i]);
            i += 1;
        }
        while matches!(buf.last(), Some(b'\r') | Some(b' ')) {
            buf.pop();
        }
        if let Some(at) = find(&buf, b"- grimoire (") {
            if find(&buf[at..], b"):").is_some() {
                let v = buf.iter().rposition(|&c| c == b' ').map(|last| &buf[last + 1..]);
                let mut key = [0u32; 3];
                if let Some(v) = v {
                    if let Ok(v) = std::str::from_utf8(v) {
                        if v.len() < UPDATE_VERSION_LEN
                            && update_version_key(v, &mut key)
                            && (!found || update_version_cmp(v, version) > 0)
                        {
                            version.clear();
                            version.push_str(v);
                            found = true;
                        }
                    }
                }
            }
        }
        match end {
            None => break,
            Some(e) => line = e + 1,
        }
    }
    found
}

/// strstr: where needle starts in hay.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---- on the Kindle ----

const OUT_MAX: usize = 64 * 1024;

struct State {
    script: FixedStr<512>,
    request: FixedStr<512>,
    pid: libc::pid_t,
    fd: i32,
    /// the check's output, as far as it came (at most OUT_MAX bytes, then a NUL)
    out: Vec<u8>,
    len: usize,
}

impl Default for State {
    fn default() -> State {
        State { script: FixedStr::default(), request: FixedStr::default(), pid: 0, fd: -1, out: Vec::new(), len: 0 }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

/// script: kindle/app/update.sh (the launcher says where), or NULL: no updates here. request: the file
/// "Update now" writes for the launcher (it says where too; without a launcher, the simulator's, the
/// caller passes one in the data folder).
pub fn update_init(script: Option<&str>, request: Option<&str>) {
    with(|s| {
        s.script.set(script.unwrap_or(""));
        s.request.set(match request {
            Some(r) if !r.is_empty() => r,
            _ => "/tmp/grimoire-update",
        });
    })
}

pub fn update_request_path() -> String {
    with(|s| s.request.as_str().to_string())
}

pub fn update_available() -> bool {
    let script = with(|s| s.script);
    !script.is_empty() && std::fs::metadata(script.as_str()).is_ok_and(|m| m.is_file())
}

pub fn update_checking() -> bool {
    with(|s| s.pid > 0)
}

/// Checks the channel (preview or stable) in a forked child; false while one is under way.
pub fn update_check(preview: bool) -> bool {
    if !update_available() || update_checking() {
        return false;
    }
    let script = with(|s| s.script);
    // (All the child needs is made before the fork.)
    let c_script = CString::new(script.as_bytes()).unwrap_or_default();
    let c_channel = if preview { c"preview" } else { c"stable" };
    let mut p = [0i32; 2];
    // SAFETY: pipe writes the two descriptors into our array.
    if unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
        return false;
    }
    // SAFETY: fork has no preconditions; the child below only sets up its descriptors and signals
    // and execs (or _exits); no borrow of the module's state is held across the fork.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        // SAFETY: closing the two descriptors pipe just made.
        unsafe {
            libc::close(p[0]);
            libc::close(p[1]);
        }
        return false;
    }
    if pid == 0 {
        // SAFETY: in the child: dup2/close on descriptors it owns, sigaction with a zeroed (default)
        // action, execl with NUL-terminated strings and a NULL-terminated list, _exit if it fails.
        unsafe {
            libc::dup2(p[1], 1);
            libc::dup2(p[1], 2);
            for fd in 3..1024 {
                libc::close(fd);
            }
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = libc::SIG_DFL;
            libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGPIPE, &sa, std::ptr::null_mut());
            libc::execl(
                c"/bin/sh".as_ptr(),
                c"sh".as_ptr(),
                c_script.as_ptr(),
                c"check".as_ptr(),
                c_channel.as_ptr(),
                std::ptr::null::<libc::c_char>(),
            );
            libc::_exit(127);
        }
    }
    // SAFETY: closing the write end the parent does not use; fcntl on the read end we own.
    unsafe {
        libc::close(p[1]);
        libc::fcntl(p[0], libc::F_SETFL, libc::fcntl(p[0], libc::F_GETFL, 0) | libc::O_NONBLOCK);
    }
    with(|s| {
        s.pid = pid;
        s.fd = p[0];
        s.len = 0;
        s.out.clear();
        s.out.resize(OUT_MAX + 1, 0);
    });
    qlog!("update: checking the %s channel", if preview { "preview" } else { "stable" });
    true
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UpdateEvent {
    #[default]
    None = 0,  // UPDATE_EV_NONE
    Done = 1,  // UPDATE_EV_DONE
    Failed = 2,  // UPDATE_EV_FAILED
}

fn drain(s: &mut State) {
    while s.len < OUT_MAX {
        // SAFETY: read into the output buffer, which has OUT_MAX + 1 bytes: len + (OUT_MAX - len)
        // stays inside it.
        let r = unsafe { libc::read(s.fd, s.out[s.len..].as_mut_ptr() as *mut libc::c_void, OUT_MAX - s.len) };
        if r > 0 {
            s.len += r as usize;
        } else {
            break;
        }
    }
}

/// Reaps the check: UPDATE_EV_DONE with the newest version it offers in *latest ("" when it offers
/// none), UPDATE_EV_FAILED when the check did not work (no network, no KPM: *why says which).
pub fn update_tick(latest: &mut String /* at most UPDATE_VERSION_LEN - 1 bytes */, why: &mut String, why_cap: usize) -> UpdateEvent {
    // The status once the check has ended; nothing while it runs (or when none was started).
    let status = with(|s| {
        if s.pid <= 0 {
            return None;
        }
        drain(s);
        let mut status = 0;
        // SAFETY: waitpid writes the status into our int.
        if unsafe { libc::waitpid(s.pid, &mut status, libc::WNOHANG) } != s.pid {
            return None;
        }
        drain(s);
        let len = s.len;
        s.out[len] = 0;
        // SAFETY: closing the read end this check owns.
        unsafe { libc::close(s.fd) };
        s.fd = -1;
        s.pid = 0;
        Some(status)
    });
    let Some(status) = status else { return UpdateEvent::None };
    let code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { -1 };
    if code != 0 {
        // update.sh's codes: 3 no KPM, 4 the feed could not be added, 5 the index could not be
        // refreshed (no network, mostly).
        csnprintf!(why, why_cap, "%s", if code == 3 { "no-kpm" } else if code == 4 { "feed" } else if code == 5 { "network" } else { "failed" });
        qlog!("update: the check did not work (%s, status %d)", &*why, code);
        return UpdateEvent::Failed;
    }
    with(|s| latest_of(&s.out[..s.len], latest));
    qlog!("update: the channel offers %s", if !latest.is_empty() { &**latest } else { "no Grimoire" });
    UpdateEvent::Done
}

/// Writes the request ("preview" or "stable") for the launcher. 0, or -1 when it could not.
pub fn update_request(preview: bool) -> i32 {
    let what = if preview { "preview\n" } else { "stable\n" };
    let request = with(|s| s.request);
    write_file_atomic(request.as_str(), what.as_bytes())
}
