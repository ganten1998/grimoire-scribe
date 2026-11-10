//! Pictures in the sandbox (2.1.0-beta.2, lane L7; DESIGN.md 3.7 and 4): the templates, photos and
//! picture-folder pages that come from outside Grimoire are decoded by the safe loader's worker (sandbox.rs,
//! `WorkKind::Picture`), never by the writer. KOReader's own page PNGs and Grimoire's own bg/ pictures are
//! not handled here: the UI still draws those, as before.
//!
//! - the worker side (`worker_run`): takes a command line on fd 5 followed by the picture's bytes, runs the
//!   very same stb_image path the writer ran before (bgimage.rs: header check, decode within the budget,
//!   EXIF turn for a photo, the page fit) and answers on fd 4. It never logs.
//! - the writer side (`PicPool`): starts a worker lazily, sends one picture at a time, validates every
//!   record and the pixel buffer, and keeps one worker for up to `s1::PICTURE_SLICE` pictures (so a
//!   2000-picture folder costs a few dozen execs, not 2000). A worker that dies or stalls on a picture is
//!   replaced by a fresh one, which tries the picture again; a second death in a row fails that picture
//!   alone (`PicOut::Dead`): a template or photo is marked `.bad`, a folder's import counts one try of its
//!   three. A call the filter refuses is final at once. A worker that cannot start, or whose sandbox step
//!   failed, is not the picture's fault. Where the sandbox is required (the Kindle, or GRIMOIRE_SANDBOX=require:
//!   sandbox.rs `sandbox_required`) pictures then fail closed (the owner's ruling, 2026-11-10): nothing is
//!   decoded, in a worker or in the writer, and the picture waits (`PicOut::NoWorker`: a template or photo is not
//!   marked, a folder's import counts no try, and a later job tries again). Elsewhere (the Mac, the simulator,
//!   tests) the pool decodes in the writer, as 2.0 did (D21). Either way the log says so once a process.
//!
//! Commands (writer to worker, one line, then `len` bytes):
//!   `C <file_len> <len>`            the header check of a picture (the first `len` bytes of a file of
//!                                   `file_len` bytes)
//!   `D <flags> <pw> <ph> <len>`     decode, turn upright when flags & 1, fit onto a pw x ph page
//!   `H <hook> <n>`                  a test hook (headless and test builds only: `crash`, `hang`, `trap`; the
//!                                   n-th request of this worker)
//!   `Q`                             end
//!
//! Records (worker to writer, text lines of a fixed grammar):
//!   `B <version> <protocol>`, `S <0|1>`  the handshake, as the PDF worker's
//!   `I <w> <h>`                     the answer to `C`: a picture Grimoire takes, this big
//!   `G <sw> <sh> <w> <h> <ch>`      the answer to `D`, then w * h * ch bytes: the page picture (sw x sh was the
//!                                   source's size), top row first
//!   `E bad <n>`                     the picture is at fault (n: one of `WHYS`)
//!   `E mem`                         the memory it needed was not there
//!   `E sandbox <step>`, `E protocol`, `E sigsys <call>`

use crate::prelude::*;
use crate::sandbox::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// What the worker says when a picture is at fault, as numbers (the writer maps them back to these texts and
/// never logs a worker's own words).
pub const WHYS: [&str; 6] = [
    "a damaged or unsupported picture",
    "not a PNG, JPEG or PNM picture of at most 8192 pixels a side",
    "more than 8192 pixels a side",
    "more than 40 megapixels",
    "too big to decode in the memory Grimoire gives a picture (a 16-bit or progressive one this size)",
    "a damaged picture (it takes more memory to decode than its size says)",
];

fn why_index(why: &str) -> usize {
    WHYS.iter().position(|w| *w == why).unwrap_or(0)
}

/// The most bytes of one picture the worker takes (bgimage.rs: a picture folder's page).
const BYTES_MAX: usize = BG_SOURCE_MAX_BYTES;

/// How long a worker may go without its next record before it is killed: the table's number, or (headless and
/// test builds) GRIMOIRE_PIC_WATCHDOG_MS.
fn watchdog_ms() -> u64 {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_PIC_WATCHDOG_MS").ok().and_then(|v| v.parse::<u64>().ok()).filter(|&v| v > 0) {
        return v;
    }
    s1::PICTURE_WATCHDOG_MS as u64
}

/// The limits of a picture worker (3.7 step 8).
pub fn limits() -> Limits {
    Limits {
        as_bytes: s1::WORKER_IMAGE + s1::PICTURE_AS_MARGIN,
        cpu_secs: s1::PICTURE_CPU_SECS,
        mapped: false,
        file_len: 0,
    }
}

// ---------------------------------------------------------------------------------------------------------
// the worker's side
// ---------------------------------------------------------------------------------------------------------

fn say(s: &str) {
    write_all(FD_OUT, s.as_bytes());
}

fn write_all(fd: i32, mut b: &[u8]) {
    while !b.is_empty() {
        // SAFETY: write from a slice we own.
        let n = unsafe { libc::write(fd, b.as_ptr() as *const libc::c_void, b.len()) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            // the writer is gone: the parent-death signal is on its way
            // SAFETY: _exit ends the process at once.
            unsafe { libc::_exit(6) };
        }
        b = &b[n as usize..];
    }
}

fn read_line_fd(fd: i32) -> Option<String> {
    let mut line = Vec::new();
    loop {
        let mut b = [0u8; 1];
        // SAFETY: read one byte into a buffer we own.
        let n = unsafe { libc::read(fd, b.as_mut_ptr() as *mut libc::c_void, 1) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            return None;
        }
        if b[0] == b'\n' {
            return String::from_utf8(line).ok();
        }
        if line.len() > 200 {
            return None;
        }
        line.push(b[0]);
    }
}

/// `len` bytes from fd 5, or None (the pipe ended, or no memory for them).
fn read_bytes_fd(fd: i32, len: usize) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    v.try_reserve_exact(len).ok()?;
    v.resize(len, 0);
    let mut got = 0;
    while got < len {
        // SAFETY: read into the part of v not yet filled.
        let n = unsafe { libc::read(fd, v[got..].as_mut_ptr() as *mut libc::c_void, len - got) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            return None;
        }
        got += n as usize;
    }
    Some(v)
}

fn number(t: &str) -> Option<usize> {
    if t.is_empty() || t.len() > 9 || !t.bytes().all(|c| c.is_ascii_digit()) || (t.len() > 1 && t.starts_with('0')) {
        return None;
    }
    t.parse().ok()
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "kindle", allow(dead_code))]
struct Hooks {
    crash: usize,
    hang: usize,
    trap: usize,
}

#[cfg(not(feature = "kindle"))]
fn run_hooks(h: &Hooks, n: usize) {
    if h.crash == n {
        std::process::abort();
    }
    if h.hang == n {
        // a busy loop: a sleeping call is not on the allow-list
        let mut x = 0u64;
        loop {
            x = std::hint::black_box(x.wrapping_add(1));
        }
    }
    if h.trap == n {
        // SAFETY: getpid takes no arguments; it is not on the allow-list, so under the filter it never returns.
        unsafe { libc::getpid() };
    }
}
#[cfg(feature = "kindle")]
fn run_hooks(_h: &Hooks, _n: usize) {}

/// The decode one `D` asks for: exactly what the writer did before this lane (bgimage.rs), in the same order.
/// Err is the record to send.
fn decode_one(data: &[u8], orient: bool, pw: i32, ph: i32) -> Result<(BgPicture, i32, i32), String> {
    let mut why: &'static str = "a damaged or unsupported picture";
    let mut p = BgPicture::default();
    match bgimage_decode(data, &mut p, &mut why) {
        0 => {}
        -2 => return Err("E mem\n".to_string()),
        _ => return Err(format!("E bad {}\n", why_index(why))),
    }
    if orient && bgimage_orient(&mut p, bgimage_exif_orientation(data)) != 0 {
        return Err("E mem\n".to_string());
    }
    let (sw, sh) = (p.w, p.h);
    let mut fit = BgPicture::default();
    if bgimage_normalize(&p, pw, ph, &mut fit) != 0 {
        return Err("E mem\n".to_string());
    }
    Ok((fit, sw, sh))
}

/// The worker's loop (after sandbox.rs's steps): one picture a command.
pub fn worker_run(_l: &Limits) -> i32 {
    #[cfg_attr(feature = "kindle", allow(unused_mut))]
    let mut hooks = Hooks::default();
    let mut served = 0usize;
    while let Some(cmd) = read_line_fd(FD_IN) {
        let t: Vec<&str> = cmd.split(' ').collect();
        match t.as_slice() {
            ["C", file_len, len] => {
                let (Some(file_len), Some(len)) = (number(file_len), number(len)) else {
                    say("E protocol\n");
                    return 1;
                };
                if len == 0 || len > BYTES_MAX || file_len < len || file_len > BYTES_MAX {
                    say("E protocol\n");
                    return 1;
                }
                let Some(data) = read_bytes_fd(FD_IN, len) else {
                    return 1;
                };
                served += 1;
                run_hooks(&hooks, served);
                let (mut w, mut h) = (0, 0);
                let mut why: &'static str = "";
                if bgimage_check(&data, file_len, &mut w, &mut h, &mut why) == 0 {
                    say(&format!("I {w} {h}\n"));
                } else {
                    say(&format!("E bad {}\n", why_index(why)));
                }
            }
            ["D", flags, pw, ph, len] => {
                let (Some(flags), Some(pw), Some(ph), Some(len)) = (number(flags), number(pw), number(ph), number(len)) else {
                    say("E protocol\n");
                    return 1;
                };
                if flags > 1 || pw == 0 || ph == 0 || pw > 6000 || ph > 6000 || len == 0 || len > BYTES_MAX {
                    say("E protocol\n");
                    return 1;
                }
                let Some(data) = read_bytes_fd(FD_IN, len) else {
                    return 1;
                };
                served += 1;
                run_hooks(&hooks, served);
                match decode_one(&data, flags == 1, pw as i32, ph as i32) {
                    Ok((fit, sw, sh)) => {
                        drop(data);
                        say(&format!("G {sw} {sh} {} {} {}\n", fit.w, fit.h, fit.ch));
                        write_all(FD_OUT, &fit.px);
                    }
                    Err(rec) => say(&rec),
                }
            }
            #[cfg(not(feature = "kindle"))]
            ["H", hook, n] => {
                let Some(n) = number(n) else {
                    say("E protocol\n");
                    return 1;
                };
                match *hook {
                    "crash" => hooks.crash = n,
                    "hang" => hooks.hang = n,
                    "trap" => hooks.trap = n,
                    _ => {
                        say("E protocol\n");
                        return 1;
                    }
                }
            }
            ["Q"] => return 0,
            _ => {
                say("E protocol\n");
                return 1;
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------------------------------------
// the writer's side
// ---------------------------------------------------------------------------------------------------------

/// What a picture came to.
#[derive(Debug)]
pub enum PicOut {
    /// the header check passed: a picture Grimoire takes, this big
    Info { w: i32, h: i32 },
    /// decoded, upright if asked, and fitted onto the page: `src_w` x `src_h` was the source's size
    Pic { src_w: i32, src_h: i32, fit: BgPicture },
    /// the picture is at fault (damaged, unsupported, too big): trying again gives the same
    Bad(&'static str),
    /// the memory it needed was not there: another time it may be
    Retry(&'static str),
    /// the worker died or stalled on it: it is marked bad
    Dead(&'static str),
    /// no worker can be had and the sandbox is required (the Kindle): nothing was decoded, in a worker or in the
    /// writer. Not the picture's fault: nothing is marked and no try is counted; a later job tries it again.
    NoWorker(&'static str),
}

/// What `PicOut::NoWorker` says (the reason is in the log line `no_worker` writes once a process).
pub const NO_WORKER_WHY: &str = "no sandboxed worker can be had";

static NO_WORKER: AtomicBool = AtomicBool::new(false);
static SANDBOX_LINE: AtomicBool = AtomicBool::new(false);

/// Whether no picture worker can be had in this process (one could not start, or a sandbox step failed). Where
/// the sandbox is required pictures then wait (`PicOut::NoWorker`); elsewhere the writer decodes them itself.
pub fn in_fallback() -> bool {
    NO_WORKER.load(Ordering::Relaxed) || session_broken()
}

fn no_worker(why: &str) {
    if !NO_WORKER.swap(true, Ordering::Relaxed) {
        if sandbox_required() {
            qlog!("work: pictures wait (%s): no sandboxed worker can be had, and the writer decodes none; nothing is marked and no try is counted", why);
        } else {
            qlog!("work: pictures decode in the writer (%s); the sandbox is unavailable", why);
        }
    }
}

/// The writer's pictures: one worker at a time, a fresh one after `s1::PICTURE_SLICE` pictures or a death.
pub struct PicPool {
    w: Option<Worker>,
    served: usize,
}

impl Default for PicPool {
    fn default() -> Self {
        Self::new()
    }
}

enum Start {
    Ready,
    /// no worker can be had: the sandbox or the exec is unavailable
    NoWorker,
}

/// What one request came to (`PicPool::ask`).
enum Asked {
    /// the worker's first record
    Line(Vec<u8>),
    /// no worker can be had, and none was given this picture: the writer may decode it (the sandbox is not
    /// required: D21)
    Local,
    /// no worker can be had, and the sandbox is required: nothing decodes it now, it waits
    Wait,
}

/// No worker can be had for a picture: where the sandbox is required it waits, never decoded here (whether a
/// worker was given it before or not); elsewhere one that no worker was given is decoded in the writer (D21), and
/// one a worker was given is refused, for a worker the picture took over could otherwise hand it to a root process
/// (security review 2026-11).
fn without_worker(handed: bool) -> Result<Asked, &'static str> {
    if sandbox_required() {
        Ok(Asked::Wait)
    } else if handed {
        Err("no worker for a picture a worker was given")
    } else {
        Ok(Asked::Local)
    }
}

impl PicPool {
    pub fn new() -> PicPool {
        PicPool { w: None, served: 0 }
    }

    fn start(&mut self) -> Start {
        if in_fallback() {
            if session_broken() {
                no_worker("an earlier worker's sandbox step failed");
            }
            return Start::NoWorker;
        }
        if self.w.is_some() && self.served < s1::PICTURE_SLICE {
            return Start::Ready;
        }
        self.end();
        // a worker that is gone must never kill the writer with SIGPIPE
        // SAFETY: setting a signal's disposition to ignore.
        unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
        let mut w = match Worker::spawn(WorkKind::Picture, None, &limits()) {
            Ok(w) => w,
            Err(SpawnErr::Sandbox(step)) => {
                session_set_broken();
                no_worker(&format!("step {step}"));
                return Start::NoWorker;
            }
            Err(SpawnErr::Start) => {
                no_worker("the worker did not start");
                return Start::NoWorker;
            }
        };
        let deadline = mono_ms() + 15_000;
        let hello = (|| {
            let line = w.read_line(512, deadline).ok()?;
            if line != format!("B {}", build_id()).as_bytes() {
                return Some(Err("the worker is another build"));
            }
            let line = w.read_line(512, deadline).ok()?;
            let s = String::from_utf8_lossy(&line).into_owned();
            if s == "S 0" || s == "S 1" {
                return Some(Ok(()));
            }
            if let Some(step) = s.strip_prefix("E sandbox ") {
                if step.len() < 32 && step.bytes().all(|c| c.is_ascii_lowercase()) {
                    session_set_broken();
                    return Some(Err("a sandbox step failed"));
                }
            }
            Some(Err("no handshake"))
        })();
        match hello {
            Some(Ok(())) => {}
            Some(Err(why)) => {
                w.kill_and_reap();
                no_worker(why);
                return Start::NoWorker;
            }
            None => {
                w.kill_and_reap();
                no_worker("the worker did not answer");
                return Start::NoWorker;
            }
        }
        if !SANDBOX_LINE.swap(true, Ordering::Relaxed) {
            qlog!("%s", &sandboxed_line());
        }
        #[cfg(not(feature = "kindle"))]
        for (var, hook) in [("GRIMOIRE_PIC_TEST_CRASH", "crash"), ("GRIMOIRE_PIC_TEST_HANG", "hang"), ("GRIMOIRE_PIC_TEST_TRAP", "trap")] {
            if let Some(n) = std::env::var(var).ok().and_then(|v| v.parse::<usize>().ok()).filter(|&n| n > 0) {
                w.send(&format!("H {hook} {n}\n"));
            }
        }
        self.w = Some(w);
        self.served = 0;
        Start::Ready
    }

    fn end(&mut self) {
        if let Some(mut w) = self.w.take() {
            w.send("Q\n");
            w.finish(500);
        }
    }

    /// One request: the command line, the bytes, and the first record back. When no worker can be had nothing was
    /// decoded: `Asked::Wait` where the sandbox is required (the picture waits), else `Asked::Local` when no worker
    /// was ever given this picture (the caller decodes it in the writer, D21). `handed` says an earlier try gave it
    /// to a worker already: a picture a worker was given is never decoded in the writer (`without_worker`).
    fn ask(&mut self, line: &str, data: &[u8], mut handed: bool) -> Result<Asked, &'static str> {
        let mut again = true;
        loop {
            if let Start::NoWorker = self.start() {
                return without_worker(handed);
            }
            let fresh = self.served == 0;
            let Some(w) = self.w.as_mut() else {
                return without_worker(handed);
            };
            self.served += 1;
            let deadline = mono_ms() + watchdog_ms();
            // (once its command went out, the worker may read the picture: from here on it was given it)
            let sent = w.send(line) && {
                handed = true;
                w.send_bytes(data, deadline)
            };
            if !sent {
                // an idle worker that was killed from outside: one try on a fresh one
                let stalled = mono_ms() >= deadline;
                self.drop_worker();
                if !fresh && again && !stalled {
                    again = false;
                    continue;
                }
                return Err(if stalled { "the worker stalled" } else { "the worker ended" });
            }
            match w.read_line(512, deadline) {
                Ok(l) => return Ok(Asked::Line(l)),
                Err(ReadErr::Timeout) => {
                    self.drop_worker();
                    return Err("the worker stalled");
                }
                Err(_) => {
                    self.drop_worker();
                    return Err("the worker ended");
                }
            }
        }
    }

    fn drop_worker(&mut self) {
        if let Some(mut w) = self.w.take() {
            w.kill_and_reap();
        }
        self.served = 0;
    }

    /// The header check of a picture (bgimage_check): `head` is the first bytes of a file of `file_len` bytes.
    /// A worker that ends or stalls on it is not taken as the picture's fault at once: a fresh worker tries it
    /// again, and only a second death in a row is `Dead` (the kernel takes a worker first when memory runs
    /// out anywhere, and one CPU limit is shared by the pictures a worker serves).
    pub fn check(&mut self, head: &[u8], file_len: usize) -> PicOut {
        if head.is_empty() {
            return PicOut::Bad(WHYS[1]);
        }
        if head.len() > BYTES_MAX || file_len < head.len() || file_len > BYTES_MAX {
            return PicOut::Bad("too big a file");
        }
        let cmd = format!("C {} {}\n", file_len, head.len());
        let mut r = self.ask(&cmd, head, false);
        if let Err(why) = r {
            qlog!("work: a picture's worker ended on its header (%s); a fresh worker tries it again", why);
            r = self.ask(&cmd, head, true);
        }
        let line = match r {
            Ok(Asked::Line(l)) => l,
            Ok(Asked::Local) => return local_check(head, file_len),
            Ok(Asked::Wait) => return PicOut::NoWorker(NO_WORKER_WHY),
            Err(why) => {
                qlog!("work: a picture's worker ended on its header (%s)", why);
                return PicOut::Dead(why);
            }
        };
        match parse_info(&line) {
            Reply::Info(w, h) => PicOut::Info { w, h },
            Reply::Bad(why) => PicOut::Bad(why),
            Reply::Mem => PicOut::Retry("out of memory"),
            // A sandbox step can only fail before the handshake's S line: after the picture's bytes it is a worker
            // that lies (one the picture took over), never a reason to read the picture in the writer.
            Reply::Sandbox => self.late_sandbox(),
            Reply::Sigsys(n) => self.sigsys(n),
            _ => {
                self.drop_worker();
                PicOut::Dead("the worker broke the protocol")
            }
        }
    }

    fn late_sandbox(&mut self) -> PicOut {
        self.drop_worker();
        qlog!("work: a picture's worker claimed a failed sandbox step after it was given the picture: refused");
        PicOut::Dead("the worker broke the protocol")
    }

    fn sigsys(&mut self, n: i32) -> PicOut {
        self.drop_worker();
        qlog!("work: a picture's worker made call %d, which the sandbox does not allow", n);
        PicOut::Dead("the worker made a call the sandbox does not allow")
    }

    /// Decodes a picture (bgimage_decode), turned upright when `orient`, fitted onto a `pw` x `ph` page
    /// (bgimage_normalize). As `check`: a worker that ends or stalls on it gets a fresh one for a second try,
    /// and only a second death in a row is `Dead`.
    pub fn decode(&mut self, data: &[u8], orient: bool, pw: i32, ph: i32) -> PicOut {
        if data.is_empty() || data.len() > BYTES_MAX || !(1..=6000).contains(&pw) || !(1..=6000).contains(&ph) {
            return PicOut::Bad("a damaged or unsupported picture");
        }
        match self.decode_once(data, orient, pw, ph, false) {
            Ok(out) => out,
            Err(why) => {
                qlog!("work: a picture's worker ended (%s); a fresh worker tries it again", why);
                self.decode_once(data, orient, pw, ph, true).unwrap_or_else(|why| {
                    qlog!("work: a picture's worker ended (%s)", why);
                    PicOut::Dead(why)
                })
            }
        }
    }

    /// One try of `decode`: Err when the worker ended or stalled on the picture (gone, a fresh one comes next).
    /// `handed`: an earlier try gave the picture to a worker (it is then never decoded in the writer).
    fn decode_once(&mut self, data: &[u8], orient: bool, pw: i32, ph: i32, handed: bool) -> Result<PicOut, &'static str> {
        let r = self.ask(&format!("D {} {} {} {}\n", orient as u8, pw, ph, data.len()), data, handed);
        let line = match r {
            Ok(Asked::Line(l)) => l,
            Ok(Asked::Local) => return Ok(local_decode(data, orient, pw, ph)),
            Ok(Asked::Wait) => return Ok(PicOut::NoWorker(NO_WORKER_WHY)),
            Err(why) => return Err(why),
        };
        self.decoded(&line, pw, ph)
    }

    /// The worker's answer to a `D`, and the pixels after it.
    fn decoded(&mut self, line: &[u8], pw: i32, ph: i32) -> Result<PicOut, &'static str> {
        Ok(match parse_info(line) {
            Reply::Pic { sw, sh, w, h, ch } => {
                if w != pw || h != ph || !(ch == 1 || ch == 3) || sw < 1 || sh < 1 || sw > 8192 || sh > 8192 || (sw as i64) * (sh as i64) > BG_MAX_PIXELS {
                    self.drop_worker();
                    return Ok(PicOut::Dead("the worker broke the protocol"));
                }
                let n = w as usize * h as usize * ch as usize;
                let mut px: Vec<u8> = Vec::new();
                if px.try_reserve_exact(n).is_err() {
                    self.drop_worker();
                    return Ok(PicOut::Retry("out of memory"));
                }
                px.resize(n, 0);
                let deadline = mono_ms() + watchdog_ms();
                let got = self.w.as_mut().map(|w| w.read_exact(&mut px, deadline));
                if !matches!(got, Some(Ok(()))) {
                    self.drop_worker();
                    qlog!("work: a picture's worker ended while it sent the pixels");
                    return Err("the worker ended");
                }
                PicOut::Pic { src_w: sw, src_h: sh, fit: BgPicture { px, w, h, ch } }
            }
            Reply::Bad(why) => PicOut::Bad(why),
            Reply::Mem => PicOut::Retry("out of memory"),
            Reply::Sandbox => self.late_sandbox(),
            Reply::Sigsys(n) => self.sigsys(n),
            _ => {
                self.drop_worker();
                PicOut::Dead("the worker broke the protocol")
            }
        })
    }
}

impl Drop for PicPool {
    fn drop(&mut self) {
        self.end();
    }
}

enum Reply {
    Info(i32, i32),
    Pic { sw: i32, sh: i32, w: i32, h: i32, ch: i32 },
    Bad(&'static str),
    Mem,
    Sandbox,
    /// the seccomp filter ended the worker: the call it made (never a picture's fault, and never a reason to
    /// decode the picture in the writer: the picture is refused)
    Sigsys(i32),
    Other,
}

/// One record line, strictly: ASCII, single spaces, decimal numbers without sign or leading zero.
fn parse_info(line: &[u8]) -> Reply {
    if !line.iter().all(|&c| (0x20..0x7f).contains(&c)) {
        return Reply::Other;
    }
    let Ok(s) = std::str::from_utf8(line) else { return Reply::Other };
    let t: Vec<&str> = s.split(' ').collect();
    let num = |i: usize| t.get(i).and_then(|x| number(x)).filter(|&n| n <= i32::MAX as usize).map(|n| n as i32);
    match (t[0], t.len()) {
        ("I", 3) => match (num(1), num(2)) {
            (Some(w), Some(h)) => Reply::Info(w, h),
            _ => Reply::Other,
        },
        ("G", 6) => match (num(1), num(2), num(3), num(4), num(5)) {
            (Some(sw), Some(sh), Some(w), Some(h), Some(ch)) => Reply::Pic { sw, sh, w, h, ch },
            _ => Reply::Other,
        },
        ("E", 3) if t[1] == "bad" => match num(2).and_then(|n| WHYS.get(n as usize)) {
            Some(w) => Reply::Bad(w),
            None => Reply::Other,
        },
        ("E", 2) if t[1] == "mem" => Reply::Mem,
        ("E", 3) if t[1] == "sandbox" => Reply::Sandbox,
        ("E", 3) if t[1] == "sigsys" => match num(2) {
            Some(n) => Reply::Sigsys(n),
            None => Reply::Other,
        },
        _ => Reply::Other,
    }
}

/// The writer's own header check: what 2.0 did, for when no worker can be had and the sandbox is not required.
fn local_check(head: &[u8], file_len: usize) -> PicOut {
    if sandbox_required() {
        return PicOut::NoWorker(NO_WORKER_WHY); // (never reached: `without_worker`; a last guard)
    }
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = "";
    if bgimage_check(head, file_len, &mut w, &mut h, &mut why) == 0 {
        PicOut::Info { w, h }
    } else {
        PicOut::Bad(why)
    }
}

/// The writer's own decode: what 2.0 did, for when no worker can be had and the sandbox is not required.
fn local_decode(data: &[u8], orient: bool, pw: i32, ph: i32) -> PicOut {
    if sandbox_required() {
        return PicOut::NoWorker(NO_WORKER_WHY); // (never reached: `without_worker`; a last guard)
    }
    match decode_one(data, orient, pw, ph) {
        Ok((fit, sw, sh)) => PicOut::Pic { src_w: sw, src_h: sh, fit },
        Err(rec) => match parse_info(rec.trim_end().as_bytes()) {
            Reply::Bad(why) => PicOut::Bad(why),
            _ => PicOut::Retry("out of memory"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn why_table_is_what_bgimage_says() {
        // every refusal text bgimage can give a picture is in the table, or the worker would say "damaged"
        let mut why: &'static str = "";
        let (mut w, mut h) = (0, 0);
        assert_eq!(bgimage_check(b"nope", 4, &mut w, &mut h, &mut why), -1);
        assert_eq!(why_index(why), 1);
        for (i, t) in WHYS.iter().enumerate() {
            assert_eq!(why_index(t), i);
        }
    }

    #[test]
    fn replies_parse_strictly() {
        assert!(matches!(parse_info(b"I 10 20"), Reply::Info(10, 20)));
        assert!(matches!(parse_info(b"G 5 6 7 8 1"), Reply::Pic { sw: 5, sh: 6, w: 7, h: 8, ch: 1 }));
        assert!(matches!(parse_info(b"E bad 3"), Reply::Bad(_)));
        assert!(matches!(parse_info(b"E bad 6"), Reply::Other));
        assert!(matches!(parse_info(b"E mem"), Reply::Mem));
        assert!(matches!(parse_info(b"I 010 20"), Reply::Other));
        assert!(matches!(parse_info(b"I 10  20"), Reply::Other));
        assert!(matches!(parse_info(b"I -1 20"), Reply::Other));
        assert!(matches!(parse_info(b"I 99999999999 20"), Reply::Other));
        assert!(matches!(parse_info(b"E sigsys 20"), Reply::Sigsys(20)));
        assert!(matches!(parse_info(b"X"), Reply::Other));
    }
}
