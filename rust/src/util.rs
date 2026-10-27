//! common.h: small shared helpers. Implemented in util.c. (In Rust: this module, re-exported as
//! `crate::common` too; fully implemented, as every module needs it from the first wave on.)
//!
//! Also the two pieces the C gets from the language: `FixedStr<N>`, a C `char[N]` field (the same
//! capacity, the same truncation, the same bytes on disk), and C's errno text (`strerror`), which
//! log lines quote.

use std::fmt;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

pub const APP_NAME: &str = "Grimoire";
/// ../VERSION, passed in by build.rs (the C build's -DGRIMOIRE_VERSION).
pub const GRIMOIRE_VERSION: &str = env!("GRIMOIRE_VERSION");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub const RECT_EMPTY: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
}

#[inline]
pub fn rect_is_empty(r: Rect) -> bool {
    r.w <= 0 || r.h <= 0
}

/// empty operands are ignored
pub fn rect_union(a: Rect, b: Rect) -> Rect {
    if rect_is_empty(a) {
        return b;
    }
    if rect_is_empty(b) {
        return a;
    }
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// RECT_EMPTY when disjoint
pub fn rect_intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.w).min(b.x + b.w);
    let y1 = (a.y + a.h).min(b.y + b.h);
    if x1 <= x0 || y1 <= y0 {
        return RECT_EMPTY;
    }
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// grow on all sides (negative shrinks)
pub fn rect_inflate(mut r: Rect, by: i32) -> Rect {
    r.x -= by;
    r.y -= by;
    r.w += 2 * by;
    r.h += 2 * by;
    if r.w < 0 {
        r.w = 0;
    }
    if r.h < 0 {
        r.h = 0;
    }
    r
}

pub fn rect_contains(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h
}

/// an empty inner is contained
pub fn rect_contains_rect(o: Rect, inner: Rect) -> bool {
    if rect_is_empty(inner) {
        return true;
    }
    inner.x >= o.x && inner.y >= o.y && inner.x + inner.w <= o.x + o.w && inner.y + inner.h <= o.y + o.h
}

pub fn rect_overlaps(a: Rect, b: Rect) -> bool {
    !rect_is_empty(rect_intersect(a, b))
}

/// bbox, floor/ceil, padded
pub fn rect_from_points(x0: f32, y0: f32, x1: f32, y1: f32, pad: f32) -> Rect {
    let lx = (if x0 < x1 { x0 } else { x1 }) - pad;
    let hx = (if x0 > x1 { x0 } else { x1 }) + pad;
    let ly = (if y0 < y1 { y0 } else { y1 }) - pad;
    let hy = (if y0 > y1 { y0 } else { y1 }) + pad;
    let ix = lx.floor() as i32;
    let iy = ly.floor() as i32;
    Rect { x: ix, y: iy, w: hx.ceil() as i32 - ix + 1, h: hy.ceil() as i32 - iy + 1 }
}

// The C's comparisons, not f32::clamp or Ord::clamp: those panic when lo > hi, which the C allows
// (it returns lo or hi by the order of its tests), and a NaN v comes back as v from both forms.
#[inline]
pub fn clampf(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}
#[inline]
pub fn clampi(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}
/// `a * b + c` as the C build rounds it (PORTING.md, Floating point). clang's default
/// -ffp-contract=on contracts the expression, and the backend makes one fused multiply-add of it
/// only where it judges the fused instruction faster: on arm64, the host the unit tests and the
/// golden renders run on, it does (fmadd), so there this is `mul_add`. The Kindle's build (zig cc
/// for the Cortex-A7) does not: its ARM backend lowers the contracted form to a separate multiply
/// and add (vmul, vadd; there is no vfma in the C binary), and neither does x86-64 without -mfma.
/// There this is the plain form, which Rust never fuses. For `c - a * b` pass `-a`, for
/// `a * b - c` pass `-c`: negating is exact, so the unfused form is the same number. Of a sum of
/// two products (`a * b + c * d`) clang fuses the left one: `fmla(a, b, c * d)`.
#[inline(always)]
pub fn fmla<T: Contracted>(a: T, b: T, c: T) -> T {
    T::fmla(a, b, c)
}

/// The float types fmla takes: `float` and `double` contract alike.
pub trait Contracted: Copy {
    fn fmla(a: Self, b: Self, c: Self) -> Self;
}
impl Contracted for f32 {
    #[inline(always)]
    fn fmla(a: f32, b: f32, c: f32) -> f32 {
        if cfg!(target_arch = "aarch64") {
            a.mul_add(b, c)
        } else {
            a * b + c
        }
    }
}
impl Contracted for f64 {
    #[inline(always)]
    fn fmla(a: f64, b: f64, c: f64) -> f64 {
        if cfg!(target_arch = "aarch64") {
            a.mul_add(b, c)
        } else {
            a * b + c
        }
    }
}

/// `a + (b - a) * t` is one contracted multiply-add in the C build (common.h's lerpf is a single
/// expression): see fmla.
#[inline]
pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    fmla(b - a, t, a)
}
#[inline]
pub fn mm_to_px(mm: f32, dpi: f32) -> f32 {
    mm * dpi / 25.4
}

/// monotonic milliseconds (CLOCK_MONOTONIC, as the kernel stamps input events: pensync.rs compares
/// the two)
pub fn now_ms() -> u32 {
    let ts = clock(libc::CLOCK_MONOTONIC);
    ((ts.tv_sec as u64).wrapping_mul(1000).wrapping_add(ts.tv_nsec as u64 / 1_000_000)) as u32
}

/// unix seconds
pub fn wall_time() -> i64 {
    // SAFETY: time(NULL) only reads the clock.
    unsafe { libc::time(std::ptr::null_mut()) as i64 }
}

// (libc's time_t is deprecated on 32-bit musl, where it will become 64-bit: the values here are
// milliseconds and today's times, which fit either.)
#[allow(deprecated)]
pub fn sleep_ms(ms: i32) {
    if ms <= 0 {
        return;
    }
    let mut ts = libc::timespec { tv_sec: (ms / 1000) as libc::time_t, tv_nsec: ((ms % 1000) as libc::c_long) * 1_000_000 };
    loop {
        // SAFETY: nanosleep reads ts and writes the remainder back into it; both point at our local.
        let r = unsafe { libc::nanosleep(&ts, &mut ts) };
        if r == -1 && errno() == libc::EINTR {
            continue;
        }
        break;
    }
}

fn clock(id: libc::clockid_t) -> libc::timespec {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: clock_gettime writes one timespec into our local.
    unsafe { libc::clock_gettime(id, &mut ts) };
    ts
}

/// The broken-down local time of a unix time (localtime_r), as the C's dates and log lines take it.
#[allow(deprecated)]
pub fn local_tm(t: i64) -> libc::tm {
    // SAFETY: tm is plain data (all integers and, on some libcs, a const char * that localtime_r
    // sets); zeroed is a valid value, and localtime_r fills it from our local time_t.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        let tt = t as libc::time_t;
        libc::localtime_r(&tt, &mut tm);
        tm
    }
}

/// Logging goes to stderr; on Kindle the launcher redirects it to root/grimoire.log.
/// The line: "HH:MM:SS.mmm " (local time) and the message, cut as the C's 1024-byte line buffer cuts
/// it (1023 bytes), then "\n". Called through `qlog!`, which formats as printf does (cfmt.rs).
pub fn qlog_str(msg: &str) {
    let ts = clock(libc::CLOCK_REALTIME);
    let tm = local_tm(ts.tv_sec as i64);
    let mut line = format!("{:02}:{:02}:{:02}.{:03} ", tm.tm_hour, tm.tm_min, tm.tm_sec, ts.tv_nsec / 1_000_000);
    line.push_str(msg);
    let cut = cut_at_bytes(&line, 1023);
    let mut out = String::with_capacity(cut.len() + 1);
    out.push_str(cut);
    out.push('\n');
    use std::io::Write;
    let _ = std::io::stderr().write_all(out.as_bytes());
}

/// `qlog!("display %dx%d @ %.0f dpi", w, h, dpi)`: the C's qlog, with the C's format string as it is
/// (cfmt.rs formats it as printf would), so every log line stays byte for byte what the C writes.
#[macro_export]
macro_rules! qlog {
    ($fmt:expr $(, $a:expr)* $(,)?) => {
        $crate::util::qlog_str(&$crate::cformat!($fmt $(, $a)*))
    };
}

/// Bounded string copy that always terminates. Returns strlen(dst).
/// (`out` holds at most cap - 1 bytes of src; a cut that would split a UTF-8 character drops that
/// character, where the C would leave its first bytes: see PORTING.md, Strings.)
pub fn strlcpy_q(dst: &mut String, src: &str, cap: usize) -> usize {
    dst.clear();
    if cap == 0 {
        return 0;
    }
    dst.push_str(cut_at_bytes(src, cap - 1));
    dst.len()
}

/// The longest prefix of s of at most max bytes that ends on a character boundary.
pub fn cut_at_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut n = max;
    while n > 0 && !s.is_char_boundary(n) {
        n -= 1;
    }
    &s[..n]
}

/// Format a unix time as "Oct 10, 14:32" (local time) into buf.
pub fn format_time_short(t: i64, buf: &mut String, cap: usize) {
    let tm = local_tm(t);
    // TRANSLATORS: a date and time, "Oct 10, 14:32": the month (as above), the day, then hours and
    // minutes on the 24-hour clock. Reorder them with %1$s, %2$d, %3$02d and %4$02d ("%2$d %1$s,
    // %3$02d:%4$02d" is "10 Oct, 14:32"), using that form for all four.
    let s = crate::cformat!(crate::i18n::c_("date", "%s %d, %02d:%02d"), month_short(&tm), tm.tm_mday, tm.tm_hour, tm.tm_min);
    strlcpy_q(buf, &s, cap);
}

/// The date alone, "Oct 10", as in a new notebook's name (i18n.h's format_date_short).
pub fn format_date_short(t: i64, buf: &mut String, cap: usize) {
    let tm = local_tm(t);
    // TRANSLATORS: a date, "Oct 10", in a new notebook's name ("Notebook Oct 10"): the month (as
    // above), then the day. "%2$d %1$s" is "10 Oct".
    let s = crate::cformat!(crate::i18n::c_("date", "%s %d"), month_short(&tm), tm.tm_mday);
    strlcpy_q(buf, &s, cap);
}

// Dates in the language in use: the month's short name and the order come from the catalog.
fn month_short(tm: &libc::tm) -> &'static str {
    use crate::i18n::nc_;
    // TRANSLATORS: months, as short as the language writes them on a date: "Oct 10, 14:32" on the
    // Library's cards, which are narrow when the Kindle is sideways.
    const MON: [&str; 12] = [
        nc_("month", "Jan"),
        nc_("month", "Feb"),
        nc_("month", "Mar"),
        nc_("month", "Apr"),
        nc_("month", "May"),
        nc_("month", "Jun"),
        nc_("month", "Jul"),
        nc_("month", "Aug"),
        nc_("month", "Sep"),
        nc_("month", "Oct"),
        nc_("month", "Nov"),
        nc_("month", "Dec"),
    ];
    crate::i18n::c_("month", MON[((tm.tm_mon % 12 + 12) % 12) as usize])
}

/// Path helpers (POSIX). mkdir -p; returns 0 on success.
pub fn mkdir_p(path: &str) -> i32 {
    // The C copies into a 1024-byte buffer and refuses a path that fills it.
    if path.len() >= 1023 {
        return -1;
    }
    let b = path.as_bytes();
    for i in 1..b.len() {
        if b[i] == b'/' && mkdir_one(&path[..i]) != 0 {
            return -1;
        }
    }
    mkdir_one(path)
}

fn mkdir_one(p: &str) -> i32 {
    let Ok(c) = std::ffi::CString::new(p) else { return -1 };
    // SAFETY: mkdir reads the NUL-terminated path we own.
    let r = unsafe { libc::mkdir(c.as_ptr(), 0o755) };
    if r != 0 && errno() != libc::EEXIST {
        return -1;
    }
    0
}

#[cfg(feature = "sim")]
/// The simulator's "storage full" (a script command): every write_file_atomic fails, as on a full
/// userstore, so a scenario can see what a failed save does.
pub static SIM_STORAGE_FULL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Atomic file write: path.tmp, fsync, rename. Returns 0 on success. It pumps (work_pump) between
/// its steps, so each wait on the storage is one fsync at most.
pub fn write_file_atomic(path: &str, data: &[u8]) -> i32 {
    let tmp = format!("{path}.tmp");
    #[cfg(feature = "sim")]
    if SIM_STORAGE_FULL.load(Ordering::Relaxed) {
        crate::qlog!("write %s: %s (simulated: storage full)", &tmp, &strerror(libc::ENOSPC));
        return -1;
    }
    let Ok(ctmp) = std::ffi::CString::new(tmp.as_str()) else { return -1 };
    // SAFETY: open/write/fsync/close/rename/unlink on paths and an fd this function owns; the
    // buffers passed are our slices, with their lengths.
    unsafe {
        let fd = libc::open(ctmp.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd < 0 {
            crate::qlog!("write %s: %s", &tmp, &strerror(errno()));
            return -1;
        }
        let mut left = data;
        while !left.is_empty() {
            let w = libc::write(fd, left.as_ptr() as *const libc::c_void, left.len());
            if w < 0 {
                if errno() == libc::EINTR {
                    continue;
                }
                crate::qlog!("write %s: %s", &tmp, &strerror(errno()));
                libc::close(fd);
                libc::unlink(ctmp.as_ptr());
                return -1;
            }
            left = &left[w as usize..];
        }
        // Between the steps the pens are read (work_pump): on the Kindle each fsync waits 15-25 ms,
        // and a page's save (the page, then notebook.ini) took 65-104 ms in one go.
        work_pump();
        // A late error (disk full, I/O) must not let a short file replace a good one.
        if libc::fsync(fd) != 0 {
            crate::qlog!("fsync %s: %s", &tmp, &strerror(errno()));
            libc::close(fd);
            libc::unlink(ctmp.as_ptr());
            return -1;
        }
        work_pump();
        if libc::close(fd) != 0 {
            crate::qlog!("close %s: %s", &tmp, &strerror(errno()));
            libc::unlink(ctmp.as_ptr());
            return -1;
        }
        let Ok(cpath) = std::ffi::CString::new(path) else { return -1 };
        if libc::rename(ctmp.as_ptr(), cpath.as_ptr()) != 0 {
            crate::qlog!("rename %s: %s", path, &strerror(errno()));
            libc::unlink(ctmp.as_ptr());
            return -1;
        }
        work_pump();
        // Best effort: make the rename itself durable.
        if let Ok(cdir) = std::ffi::CString::new(dirname(path)) {
            let dfd = libc::open(cdir.as_ptr(), libc::O_RDONLY);
            if dfd >= 0 {
                libc::fsync(dfd);
                libc::close(dfd);
            }
        }
        work_pump();
    }
    0
}

/// POSIX dirname(3) of a path, as the C calls it on a copy: "." for none, "/" for the root.
pub fn dirname(path: &str) -> String {
    let t = path.trim_end_matches('/');
    if t.is_empty() {
        return if path.starts_with('/') { "/".into() } else { ".".into() };
    }
    match t.rfind('/') {
        None => ".".into(),
        Some(i) => {
            let d = t[..i].trim_end_matches('/');
            if d.is_empty() {
                "/".into()
            } else {
                d.into()
            }
        }
    }
}

// The work pump (common.h): fn and the process that set it.
static PUMP: AtomicUsize = AtomicUsize::new(0);
static PUMP_PID: AtomicI32 = AtomicI32::new(0);

/// Long work on the main thread (a page's render, a full-screen copy, a save) calls work_pump()
/// every few milliseconds of it. On the Kindle the hook reads the two pens (input_evdev.c), whose
/// kernel buffers hold only 50-175 ms: without it a render of 1.2 s lost the pressure and tilt of
/// the raw digitizer, and could cut a stroke (docs/INK-PERFORMANCE.md, Reader stalls). Elsewhere
/// (host tools, tests) no hook is set and it does nothing. Only the process that set the hook runs
/// it: a forked child (an export, a share server render) shares Grimoire's open input devices, and
/// reading them would take Grimoire's pen events, or read a file that came to reuse their number.
pub fn work_pump_set(f: Option<fn()>) {
    PUMP.store(f.map_or(0, |f| f as usize), Ordering::Relaxed);
    // SAFETY: getpid has no preconditions.
    PUMP_PID.store(if f.is_some() { unsafe { libc::getpid() } } else { 0 }, Ordering::Relaxed);
}

pub fn work_pump() {
    let f = PUMP.load(Ordering::Relaxed);
    // SAFETY: getpid has no preconditions.
    if f != 0 && unsafe { libc::getpid() } == PUMP_PID.load(Ordering::Relaxed) {
        // SAFETY: PUMP only ever holds 0 or a `fn()` stored by work_pump_set, cast back to its type.
        let f: fn() = unsafe { std::mem::transmute::<usize, fn()>(f) };
        f();
    }
}

/// Reads a whole file; None on failure. (The C returns what it read when a read fails midway, as
/// fread does; so does this.)
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match f.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    Some(buf)
}

/// errno as the C reads it after a failed call.
pub fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// strerror(e): the C library's text, which log lines quote ("No such file or directory"), never
/// Rust's io::Error text (which adds " (os error 2)").
pub fn strerror(e: i32) -> String {
    // SAFETY: strerror returns a pointer to a NUL-terminated message owned by the C library, valid
    // until the next strerror call on this thread; it is copied out at once.
    unsafe {
        let p = libc::strerror(e);
        if p.is_null() {
            return format!("Unknown error {e}");
        }
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

// ---- FixedStr: a C char[N] field ----

/// A C `char name[N]` struct field: at most N - 1 bytes, NUL-terminated within N, Copy like the C
/// struct it sits in. Setting it cuts as the C's strlcpy_q into that field does (bytes, so what
/// reaches a file is what the C writes); `set_cut` cuts between characters, as i18n_copy does.
/// `as_str` reads the bytes as UTF-8 up to the first invalid sequence (a field the C cut inside a
/// character): see PORTING.md, Strings.
#[derive(Clone, Copy)]
pub struct FixedStr<const N: usize> {
    buf: [u8; N],
}

impl<const N: usize> Default for FixedStr<N> {
    fn default() -> Self {
        FixedStr { buf: [0; N] }
    }
}

impl<const N: usize> FixedStr<N> {
    pub const CAP: usize = N;

    pub fn new() -> Self {
        Self::default()
    }
    // (not std's FromStr: this one cannot fail, it cuts as the C's strlcpy does)
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        let mut f = Self::default();
        f.set(s);
        f
    }
    /// strlcpy_q(field, s, N): the first N - 1 bytes of s. Returns the bytes kept.
    pub fn set(&mut self, s: &str) -> usize {
        self.set_bytes(s.as_bytes())
    }
    pub fn set_bytes(&mut self, s: &[u8]) -> usize {
        self.buf = [0; N];
        if N == 0 {
            return 0;
        }
        let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
        let n = end.min(N - 1);
        self.buf[..n].copy_from_slice(&s[..n]);
        n
    }
    /// i18n_copy(field, s, N): cut between two characters, never inside one.
    pub fn set_cut(&mut self, s: &str) -> usize {
        if N == 0 {
            self.buf = [0; N];
            return 0;
        }
        self.set(cut_at_bytes(s, N - 1))
    }
    pub fn clear(&mut self) {
        self.buf = [0; N];
    }
    pub fn len(&self) -> usize {
        self.buf.iter().position(|&b| b == 0).unwrap_or(N)
    }
    pub fn is_empty(&self) -> bool {
        N == 0 || self.buf[0] == 0
    }
    /// The bytes before the NUL: what the C writes to a file.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len()]
    }
    pub fn as_str(&self) -> &str {
        let b = self.as_bytes();
        match std::str::from_utf8(b) {
            Ok(s) => s,
            Err(e) => std::str::from_utf8(&b[..e.valid_up_to()]).unwrap_or(""),
        }
    }
}

impl<const N: usize> fmt::Display for FixedStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl<const N: usize> fmt::Debug for FixedStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}
impl<const N: usize> PartialEq for FixedStr<N> {
    fn eq(&self, o: &Self) -> bool {
        self.as_bytes() == o.as_bytes()
    }
}
impl<const N: usize> Eq for FixedStr<N> {}
impl<const N: usize> std::hash::Hash for FixedStr<N> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.as_bytes().hash(h)
    }
}
impl<const N: usize> PartialEq<str> for FixedStr<N> {
    fn eq(&self, o: &str) -> bool {
        self.as_bytes() == o.as_bytes()
    }
}
impl<const N: usize> PartialEq<&str> for FixedStr<N> {
    fn eq(&self, o: &&str) -> bool {
        self.as_bytes() == o.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rects() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(rect_union(a, b), Rect::new(0, 0, 15, 15));
        assert_eq!(rect_intersect(a, b), Rect::new(5, 5, 5, 5));
        assert_eq!(rect_intersect(a, Rect::new(20, 20, 1, 1)), RECT_EMPTY);
        assert_eq!(rect_inflate(a, -6), Rect::new(6, 6, 0, 0));
        assert!(rect_contains_rect(a, RECT_EMPTY));
        assert_eq!(rect_from_points(1.5, 2.5, 0.2, 0.7, 1.0), Rect::new(-1, -1, 5, 6));
    }

    #[test]
    fn fixed_str_cuts_like_strlcpy() {
        let mut f = FixedStr::<4>::new();
        assert_eq!(f.set("abcdef"), 3);
        assert_eq!(f, "abc");
        f.set_cut("aé");
        assert_eq!(f.as_bytes(), "aé".as_bytes());
        f.set_cut("abé");
        assert_eq!(f, "ab");
        f.set("abé");
        assert_eq!(f.as_bytes(), b"ab\xc3");
        assert_eq!(f.as_str(), "ab");
    }

    #[test]
    fn dirnames() {
        assert_eq!(dirname("/mnt/us/x.ini"), "/mnt/us");
        assert_eq!(dirname("x.ini"), ".");
        assert_eq!(dirname("/x"), "/");
        assert_eq!(dirname("a//b//"), "a");
    }

    #[test]
    fn contracted_multiply_add_is_the_c_builds() {
        // 0.1 * 0.1 + (-0.01) keeps the product's rounding error only when fused
        let (a, b, c) = (0.1f32, 0.1f32, -0.01f32);
        let (x, y, z) = (0.1f64, 0.1f64, -0.01f64);
        if cfg!(target_arch = "aarch64") {
            assert_eq!(fmla(a, b, c), a.mul_add(b, c));
            assert_ne!(fmla(a, b, c), a * b + c);
            assert_eq!(fmla(x, y, z), x.mul_add(y, z));
            assert_ne!(fmla(x, y, z), x * y + z);
        } else {
            assert_eq!(fmla(a, b, c), a * b + c);
            assert_eq!(fmla(x, y, z), x * y + z);
        }
        assert_eq!(lerpf(2.0, 10.0, 0.25), 4.0);
    }
}
