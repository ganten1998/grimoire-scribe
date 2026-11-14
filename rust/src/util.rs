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
    #[cfg(feature = "sim")]
    if let Some(at) = sim_clock() { return at; }
    real_now_ms()
}

/// Host time for the runner's waits and background-job deadlines, even with scripted app timers.
pub fn real_now_ms() -> u32 {
    let ts = clock(libc::CLOCK_MONOTONIC);
    ((ts.tv_sec as u64).wrapping_mul(1000).wrapping_add(ts.tv_nsec as u64 / 1_000_000)) as u32
}

// Only the headless runner opts into this clock. Keeping it outside its State avoids borrowing
// that state again when the app or a queued input sample reads now_ms().
#[cfg(feature = "sim")]
static SIM_CLOCK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(feature = "sim")]
pub fn sim_clock() -> Option<u32> {
    let value = SIM_CLOCK.load(Ordering::Relaxed);
    (value != 0).then_some(value as u32)
}
#[cfg(feature = "sim")]
pub fn sim_clock_set(at: Option<u32>) {
    SIM_CLOCK.store(at.map_or(0, |ms| (1u64 << 32) | ms as u64), Ordering::Relaxed);
}

#[cfg(all(test, feature = "sim"))]
#[test]
fn scripted_clock_wraps_and_returns_to_host_time() {
    sim_clock_set(Some(u32::MAX - 9));
    assert_eq!(now_ms(), u32::MAX - 9);
    sim_clock_set(Some(now_ms().wrapping_add(20)));
    assert_eq!(now_ms(), 10);
    sim_clock_set(Some(0));
    assert_eq!(now_ms(), 0);
    sim_clock_set(None);
    assert!(sim_clock().is_none());
    assert!(real_now_ms().wrapping_sub(now_ms()) < 100);
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

// Dates in the language in use: the month's short name and the order come from the catalog.
fn month_short(tm: &libc::tm) -> &'static str {
    use crate::i18n::nc_;
    const MON: [&str; 12] = [
        // TRANSLATORS: months, as short as the language writes them on a date: "Oct 10, 14:32" on the
        // Library's cards, which are narrow when the Kindle is sideways.
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

/// Format a unix time as "Oct 10, 14:32" (local time) into buf.
pub fn format_time_short(t: i64, buf: &mut String, cap: usize) {
    let tm = local_tm(t);
    // TRANSLATORS: a date and time, "Oct 10, 14:32": the month (as above), the day, then hours and
    // minutes on the 24-hour clock. Reorder them with %1$s, %2$d, %3$02d and %4$02d ("%2$d %1$s,
    // %3$02d:%4$02d" is "10 Oct, 14:32"), using that form for all four.
    let s = crate::cformat!(crate::i18n::c_("date", "%s %d, %02d:%02d"), month_short(&tm), tm.tm_mday, tm.tm_hour, tm.tm_min);
    strlcpy_q(buf, &s, cap);
}

/// The 12-hour clock's morning and afternoon markers, looked up when drawn.
static K_AMPM: [&str; 2] = [
    // TRANSLATORS: the marker for the morning on the 12-hour clock (before noon).
    crate::i18n::nc_("date", "AM"),
    // TRANSLATORS: the marker for the afternoon on the 12-hour clock (after noon).
    crate::i18n::nc_("date", "PM"),
];

/// Format a unix time as "Oct 10, 2:32 PM" (local time) into buf: the clock's 12-hour text.
pub fn format_time_short_12h(t: i64, buf: &mut String, cap: usize) {
    let tm = local_tm(t);
    let hour = match tm.tm_hour % 12 {
        0 => 12,
        h => h,
    };
    let ampm = crate::i18n::c_("date", K_AMPM[(tm.tm_hour >= 12) as usize]);
    let s = crate::cformat!(
        // TRANSLATORS: a date and time on the 12-hour clock, "Oct 10, 2:32 PM": the month (as above),
        // the day, the hour (1 to 12), the minutes, then AM or PM. Reorder them with %1$s, %2$d, %3$d,
        // %4$02d and %5$s ("%2$d %1$s, %3$d:%4$02d %5$s" is "10 Oct, 2:32 PM"), using that form for
        // all five.
        crate::i18n::c_("date", "%s %d, %d:%02d %s"),
        month_short(&tm),
        tm.tm_mday,
        hour,
        tm.tm_min,
        ampm
    );
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

/// Path helpers (POSIX). mkdir -p; returns 0 on success. An existing entry, a link to a folder included, is taken
/// as it is (the data root, a platform's folders and a test's folder may sit behind one); code that prunes or
/// empties a folder of Grimoire's checks that folder first.
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

/// Where a lock file is opened. Its natural place is beside what it guards, and that is where it stays unless
/// GRIMOIRE_LOCKS names a folder: a data folder on a file system without advisory locks (flock answers "not
/// implemented" on some FUSE and network mounts) cannot hold one, so whoever starts the app there names a folder
/// that can. The lock is then `<that folder>/<hash>.lock`, the hash being of the natural path, so each natural
/// path keeps one lock file of its own there and every process of the app agrees on it. The folder is made when
/// missing; its lock files are empty and are never pruned. Unset, empty, or a folder that cannot be made: the
/// natural path, unchanged.
pub fn lock_file_path(natural: &str) -> String {
    let Some(dir) = std::env::var_os("GRIMOIRE_LOCKS").and_then(|d| d.into_string().ok()).filter(|d| !d.is_empty()) else {
        return natural.to_string();
    };
    let dir = dir.trim_end_matches('/'); // ("/" alone becomes "": the lock is then "/<hash>.lock")
    if !dir.is_empty() && mkdir_p(dir) != 0 {
        return natural.to_string();
    }
    // FNV-1a, 64 bits
    let mut h: u64 = 14695981039346656037;
    for &b in natural.as_bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    format!("{dir}/{h:016x}.lock")
}

/// Opens `path` to be written from its start (created with mode `mode` when missing), never through a link or onto
/// anything but a file of its own: O_NOFOLLOW, so a planted `<name>.tmp` link can no longer make root truncate and
/// write its target, and O_NONBLOCK, so a FIFO cannot hold the save; then it must be a regular file with one link
/// (a hard link to another file is refused before anything is cut), and only then is it emptied (security review
/// 2026-11). The descriptor (close-on-exec), or -1 with errno.
pub fn open_rewrite_nofollow(path: &std::ffi::CStr, mode: u32) -> i32 {
    // SAFETY: open reads the NUL-terminated path we were given; fstat writes the struct we own; the descriptor is
    // the caller's, or closed here.
    unsafe {
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC, mode as libc::c_uint);
        if fd < 0 {
            return -1;
        }
        let mut st: libc::stat = std::mem::zeroed();
        let own = libc::fstat(fd, &mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFREG && st.st_nlink == 1;
        if !own || libc::ftruncate(fd, 0) != 0 {
            libc::close(fd);
            return -1;
        }
        fd
    }
}

/// A file made afresh for writing: whatever stands at the name (a stale temporary, or anything else) is
/// removed first, the entry itself and never what it may point at, and nothing that was there is opened.
pub fn create_fresh_nofollow(path: &str) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let open = || std::fs::OpenOptions::new().write(true).create_new(true).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC).open(path);
    // (nothing in the way: one call to the storage, as a plain create was)
    let first = match open() {
        Ok(f) => return Ok(f),
        Err(e) => e,
    };
    // (whatever the error was: with an entry at the name it goes and the open is tried once more; a folder
    // there does not go, and that error is the answer)
    if std::fs::symlink_metadata(path).is_err() {
        return Err(first);
    }
    std::fs::remove_file(path)?;
    open()
}

/// Removes a work folder Grimoire was building (`files` first, then the files of its bg/ and its own, then the
/// folders), by descriptor and never through a link: a folder, or a bg/, that is a link is unlinked itself and
/// never entered, so a planted link cannot make root delete the files of another folder (security review
/// 2026-11). A folder left inside (not Grimoire's) stays, and so does the work folder then.
pub fn remove_work_dir(work: &std::path::Path, files: &[&std::ffi::CStr]) {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(work.as_os_str().as_bytes()) else { return };
    // SAFETY: open reads the NUL-terminated path we own; the descriptor is closed below.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        // a link (or no folder at all): the entry itself goes, never what it points at
        if std::fs::symlink_metadata(work).is_ok_and(|m| !m.is_dir()) {
            let _ = std::fs::remove_file(work);
        }
        return;
    }
    for f in files {
        // SAFETY: unlinkat on the descriptor we own and a NUL-terminated name.
        unsafe { libc::unlinkat(fd, f.as_ptr(), 0) };
    }
    // SAFETY: openat on the descriptor we own; the descriptor it returns is closed below.
    let bg = unsafe { libc::openat(fd, c"bg".as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if bg >= 0 {
        unlink_files_at(bg);
        // SAFETY: closing the descriptor opened above, then removing the folder by its name in ours.
        unsafe {
            libc::close(bg);
            libc::unlinkat(fd, c"bg".as_ptr(), libc::AT_REMOVEDIR);
        }
    }
    unlink_files_at(fd);
    // SAFETY: closing the descriptor opened above.
    unsafe { libc::close(fd) };
    let _ = std::fs::remove_dir(work);
}

/// Whether no component of `path` below `root` is a link (a missing one is fine: mkdir_p makes it). A cache that is
/// pruned or written as root must not be reached through a planted link (security review 2026-11). A `path` that
/// is not under `root` is refused.
pub fn no_link_below(root: &str, path: &str) -> bool {
    let Some(rest) = path.strip_prefix(root.trim_end_matches('/')) else { return false };
    let mut at = root.trim_end_matches('/').to_string();
    for part in rest.split('/').filter(|p| !p.is_empty()) {
        at.push('/');
        at.push_str(part);
        match std::fs::symlink_metadata(&at) {
            Ok(m) if m.file_type().is_symlink() => return false,
            Ok(_) => {}
            Err(_) => return true,
        }
    }
    true
}

/// Empties a folder of Grimoire's (its files go, then the folder when nothing is left), by descriptor and never
/// through a link: a folder that is a link is unlinked itself and never entered (security review 2026-11). How
/// many files went.
pub fn empty_dir_nofollow(dir: &std::path::Path) -> i32 {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else { return 0 };
    // SAFETY: open reads the NUL-terminated path we own; the descriptor is closed below.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        if std::fs::symlink_metadata(dir).is_ok_and(|m| !m.is_dir()) {
            let _ = std::fs::remove_file(dir);
        }
        return 0;
    }
    let n = unlink_files_at(fd);
    // SAFETY: closing the descriptor opened above.
    unsafe { libc::close(fd) };
    let _ = std::fs::remove_dir(dir);
    n
}

/// Unlinks every entry of the folder open at `fd` that is not a folder (a link is unlinked, never followed): how
/// many went.
fn unlink_files_at(fd: i32) -> i32 {
    // SAFETY: fdopendir takes a duplicate of our descriptor, which closedir closes; readdir's entries are read
    // before the next readdir call.
    let names = unsafe {
        let d2 = libc::dup(fd);
        if d2 < 0 {
            return 0;
        }
        let d = libc::fdopendir(d2);
        if d.is_null() {
            libc::close(d2);
            return 0;
        }
        let mut names = Vec::new();
        loop {
            let e = libc::readdir(d);
            if e.is_null() {
                break;
            }
            let n = std::ffi::CStr::from_ptr((*e).d_name.as_ptr()).to_owned();
            if n.as_bytes() != b"." && n.as_bytes() != b".." {
                names.push(n);
            }
        }
        libc::closedir(d);
        names
    };
    let mut gone = 0;
    for n in names {
        // SAFETY: unlinkat on the descriptor we own and a NUL-terminated name (a folder fails: EISDIR).
        gone += (unsafe { libc::unlinkat(fd, n.as_ptr(), 0) } == 0) as i32;
    }
    gone
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

#[cfg(any(test, feature = "sim"))]
thread_local! {
    /// A test's injected write failure: the Nth write_file_atomic from now (1 = the next) fails, once; 0 = none.
    /// Per thread, as the store's own state is.
    pub static FAIL_WRITE_AT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Atomic file write: path.tmp, fsync, rename. Returns 0 on success. It pumps (work_pump) between
/// its steps, so each wait on the storage is one fsync at most.
pub fn write_file_atomic(path: &str, data: &[u8]) -> i32 {
    let r = write_file_atomic_in(path, data);
    work_pump_mark(); // (a save's waits are the storage's, not work that should pump)
    r
}

fn write_file_atomic_in(path: &str, data: &[u8]) -> i32 {
    let tmp = format!("{path}.tmp");
    #[cfg(any(test, feature = "sim"))]
    if FAIL_WRITE_AT.with(|c| match c.get() {
        0 => false,
        n => {
            c.set(n - 1);
            n == 1
        }
    }) {
        crate::qlog!("write %s: (injected failure)", &tmp);
        return -1;
    }
    #[cfg(feature = "sim")]
    if SIM_STORAGE_FULL.load(Ordering::Relaxed) {
        crate::qlog!("write %s: %s (simulated: storage full)", &tmp, &strerror(libc::ENOSPC));
        return -1;
    }
    let Ok(ctmp) = std::ffi::CString::new(tmp.as_str()) else { return -1 };
    // SAFETY: open/write/fsync/close/rename/unlink on paths and an fd this function owns; the
    // buffers passed are our slices, with their lengths.
    unsafe {
        let fd = open_rewrite_nofollow(&ctmp, 0o644);
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

thread_local! {
    static PUMP_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Runs f with work_pump doing nothing: work that is no part of what the user waits on and must not
/// read the pens in the middle of it (the renderer's fingerprint, page_render_fingerprint).
pub fn work_pump_off<R>(f: impl FnOnce() -> R) -> R {
    let was = PUMP_OFF.with(|p| p.replace(true));
    let r = f();
    PUMP_OFF.with(|p| p.set(was));
    r
}

pub fn work_pump() {
    if PUMP_OFF.with(|p| p.get()) {
        return;
    }
    let f = PUMP.load(Ordering::Relaxed);
    // SAFETY: getpid has no preconditions.
    if f != 0 && unsafe { libc::getpid() } == PUMP_PID.load(Ordering::Relaxed) {
        // SAFETY: PUMP only ever holds 0 or a `fn()` stored by work_pump_set, cast back to its type.
        let f: fn() = unsafe { std::mem::transmute::<usize, fn()>(f) };
        f();
        let beat = PUMP_BEAT.load(Ordering::Relaxed);
        if beat != 0 {
            // SAFETY: PUMP_BEAT only ever holds 0 or a `fn()` stored by work_pump_beat_set.
            let beat: fn() = unsafe { std::mem::transmute::<usize, fn()>(beat) };
            beat();
        }
    }
}

// The launcher's heartbeat as long work goes on (mainloop.rs sets it on the Kindle): work that pumps
// is making progress, so a page that takes longer to draw than the watchdog waits (a page dense
// enough to take 20 s did, 2.0.0-rc.9) is not a hang. The hook bounds it: only so long after the
// main loop last went round.
static PUMP_BEAT: AtomicUsize = AtomicUsize::new(0);
pub fn work_pump_beat_set(f: Option<fn()>) {
    PUMP_BEAT.store(f.map_or(0, |f| f as usize), Ordering::Relaxed);
}

// The work pump's mark (the runner's): fn and the process that set it.
static PUMP_MARK: AtomicUsize = AtomicUsize::new(0);
static PUMP_MARK_PID: AtomicI32 = AtomicI32::new(0);

/// Work that need not pump ends here: a present (app_flush) and a file save (write_file_atomic). The
/// headless runner times the stretches between input reads and pumps (main.longest_unpumped_ms) and
/// leaves these out; elsewhere no hook is set and it does nothing (only the process that set it runs
/// it, as work_pump).
pub fn work_pump_mark_set(f: Option<fn()>) {
    PUMP_MARK.store(f.map_or(0, |f| f as usize), Ordering::Relaxed);
    // SAFETY: getpid has no preconditions.
    PUMP_MARK_PID.store(if f.is_some() { unsafe { libc::getpid() } } else { 0 }, Ordering::Relaxed);
}

pub fn work_pump_mark() {
    let f = PUMP_MARK.load(Ordering::Relaxed);
    // SAFETY: getpid has no preconditions.
    if f != 0 && unsafe { libc::getpid() } == PUMP_MARK_PID.load(Ordering::Relaxed) {
        // SAFETY: PUMP_MARK only ever holds 0 or a `fn()` stored by work_pump_mark_set, cast back.
        let f: fn() = unsafe { std::mem::transmute::<usize, fn()>(f) };
        f();
    }
}

/// A file of the data folder opened for reading: the entry at the name itself must be a plain file (a link
/// there is not followed, a folder or anything else is refused), and the open never waits. Folders on the way
/// may be links. The file and what fstat says of it. O_NONBLOCK stays set: a caller that hands the descriptor
/// on clears it first.
pub fn open_read_nofollow(path: &str) -> std::io::Result<(std::fs::File, std::fs::Metadata)> {
    use std::os::unix::fs::OpenOptionsExt;
    // (std adds O_CLOEXEC and retries EINTR)
    let f = std::fs::OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY).open(path)?;
    let md = f.metadata()?;
    if !md.is_file() {
        return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
    }
    Ok((f, md))
}

/// A whole file of the data folder, or None: read_file for files with no limit of their own (pages, pictures,
/// previews), read_file_max for the small text files. Plain files only (open_read_nofollow). (The C returns
/// what it read when a read fails midway, as fread does; so do these.)
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let (mut f, _) = open_read_nofollow(path).ok()?;
    let mut buf = Vec::new();
    // (Straight into the buffer, sized from the file's length and grown as needed: a page file is
    // megabytes, read in a handful of calls rather than one per 4 KB with a copy each. EINTR is
    // retried. No memory for it is the C's NULL; after another error buf keeps what was read, as
    // the C's fread leaves it.)
    if let Err(e) = f.read_to_end(&mut buf) {
        if e.kind() == std::io::ErrorKind::OutOfMemory {
            return None;
        }
    }
    Some(buf)
}

/// read_file for a file of at most `cap` bytes: a longer one is None, with nothing of it read.
pub fn read_file_max(path: &str, cap: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    let (f, md) = open_read_nofollow(path).ok()?;
    // (sizes compared as u64, and never cap + 1 as a usize)
    if md.len() > cap as u64 {
        return None;
    }
    let mut buf = Vec::new();
    buf.try_reserve_exact(md.len() as usize).ok()?;
    // (one byte past the limit is asked for: a file that grew since the fstat is told from one that fits)
    if let Err(e) = f.take((cap as u64).saturating_add(1)).read_to_end(&mut buf) {
        if e.kind() == std::io::ErrorKind::OutOfMemory {
            return None;
        }
    }
    if buf.len() as u64 > cap as u64 {
        return None;
    }
    Some(buf)
}

/// A whole file that is not the data folder's (the system's own: /proc, /sys, the Kindle's locale file; a
/// developer's input to the simulator): read as before, links followed.
pub fn read_system_file(path: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    // (Straight into the buffer, sized from the file's length and grown as needed: a page file is
    // megabytes, read in a handful of calls rather than one per 4 KB with a copy each. EINTR is
    // retried. No memory for it is the C's NULL; after another error buf keeps what was read, as
    // the C's fread leaves it.)
    if let Err(e) = f.read_to_end(&mut buf) {
        if e.kind() == std::io::ErrorKind::OutOfMemory {
            return None;
        }
    }
    Some(buf)
}

/// errno as the C reads it after a failed call.
pub fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// errno = v, as the C assigns it: so a failed call's errno can be told from an old one
/// (annotate.rs), or for a caller that reads it after a None (bgimage_read's). Each C library has
/// its own name for the calling thread's slot.
pub fn set_errno(v: i32) {
    // SAFETY: the C library's errno slot belongs to the calling thread and lives as long as it does.
    #[cfg(target_os = "linux")]
    unsafe {
        *libc::__errno_location() = v;
    }
    // SAFETY: the same.
    #[cfg(target_os = "android")]
    unsafe {
        *libc::__errno() = v;
    }
    // SAFETY: the same.
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe {
        *libc::__error() = v;
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos", target_os = "ios", target_os = "freebsd")))]
    let _ = v;
}

/// Random bytes from the system (notebook ids, pairing codes and tokens), the only place that asks
/// for them: /dev/urandom. False when it failed, and buf is then as it was.
pub fn random_fill(buf: &mut [u8]) -> bool {
    use std::io::Read;
    // (std opens it close-on-exec, as the C asks with O_CLOEXEC; read_exact retries EINTR and fails
    // on a short read or an error, as the C's loop does.)
    std::fs::File::open("/dev/urandom").is_ok_and(|mut f| f.read_exact(buf).is_ok())
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
// (byte order: what sorted lists of ids are kept in)
impl<const N: usize> PartialOrd for FixedStr<N> {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl<const N: usize> Ord for FixedStr<N> {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(o.as_bytes())
    }
}
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

    // The system's generator answers, fills the whole buffer, and two draws differ.
    #[test]
    fn random_fill_draws_from_the_system() {
        assert!(random_fill(&mut []));
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        assert!(random_fill(&mut a) && random_fill(&mut b));
        assert_ne!(a, b);
        assert_ne!(a, [0u8; 32]);
        // (every byte position is written: 64 draws leave none of the 32 at zero in all of them)
        let mut seen = [0u8; 32];
        for _ in 0..64 {
            let mut c = [0u8; 32];
            assert!(random_fill(&mut c));
            seen.iter_mut().zip(c).for_each(|(s, v)| *s |= v);
        }
        assert!(seen.iter().all(|&v| v != 0), "{seen:?}");
    }

    // Randomness comes from random_fill alone: no other module opens the system's generator itself.
    #[test]
    fn only_random_fill_reads_the_systems_generator() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let src = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        let mut files = Vec::new();
        walk(src, &mut files);
        assert!(files.len() > 50, "{} sources under {}", files.len(), src.display());
        let mut found = Vec::new();
        for f in &files {
            if f.file_name().is_some_and(|n| n == "util.rs") {
                continue;
            }
            let text = std::fs::read_to_string(f).unwrap();
            for (i, line) in text.lines().enumerate() {
                if line.contains("\"/dev/urandom\"") || line.contains("\"/dev/random\"") {
                    found.push(format!("{}:{}", f.display(), i + 1));
                }
            }
        }
        assert!(found.is_empty(), "use crate::util::random_fill instead: {found:?}");
    }

    // A scratch folder of one test's own (under GRIMOIRE_TEST_TMP when the Makefile sets it).
    fn scratch(name: &str) -> std::path::PathBuf {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let dir = base.join(format!("grimoire-util-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn text(p: &std::path::Path) -> &str {
        p.to_str().unwrap()
    }
    fn fifo_at(p: &std::path::Path) {
        let c = std::ffi::CString::new(text(p)).unwrap();
        // SAFETY: mkfifo reads the NUL-terminated path we own.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
    }

    // The data folder's open takes a plain file at the name and nothing else, and never waits; a folder on the
    // way may be a link.
    #[test]
    fn open_read_nofollow_takes_plain_files_only() {
        use std::os::fd::AsRawFd;
        let d = scratch("open");
        let plain = d.join("plain.ini");
        std::fs::write(&plain, b"twelve bytes").unwrap();
        let (f, md) = open_read_nofollow(text(&plain)).unwrap();
        assert_eq!(md.len(), 12);
        assert!(md.is_file());
        // SAFETY: fcntl reads the flags of the descriptor `f` owns.
        let (fl, fd) = unsafe { (libc::fcntl(f.as_raw_fd(), libc::F_GETFL), libc::fcntl(f.as_raw_fd(), libc::F_GETFD)) };
        assert!(fl >= 0 && fl & libc::O_NONBLOCK != 0, "O_NONBLOCK stays set: {fl:#x}");
        assert!(fd >= 0 && fd & libc::FD_CLOEXEC != 0, "close-on-exec: {fd:#x}");
        drop(f);
        let link = d.join("link.ini");
        std::os::unix::fs::symlink(&plain, &link).unwrap();
        assert!(open_read_nofollow(text(&link)).is_err(), "a link to a plain file");
        let ghost = d.join("ghost.ini");
        std::os::unix::fs::symlink(d.join("nowhere"), &ghost).unwrap();
        assert!(open_read_nofollow(text(&ghost)).is_err(), "a dangling link");
        let folder = d.join("folder");
        std::fs::create_dir(&folder).unwrap();
        let e = open_read_nofollow(text(&folder)).err().expect("a folder");
        assert_eq!(e.raw_os_error(), Some(libc::EINVAL));
        // (a FIFO no one writes to: the answer comes at once, or this test would never end)
        let pipe = d.join("pipe.ini");
        fifo_at(&pipe);
        let e = open_read_nofollow(text(&pipe)).err().expect("a FIFO");
        assert_eq!(e.raw_os_error(), Some(libc::EINVAL));
        assert!(open_read_nofollow(text(&d.join("missing.ini"))).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound));
        // the same plain file through a folder that is a link
        let way = d.join("way");
        std::os::unix::fs::symlink(&d, &way).unwrap();
        let (_, md) = open_read_nofollow(text(&way.join("plain.ini"))).unwrap();
        assert_eq!(md.len(), 12);
        let _ = std::fs::remove_dir_all(&d);
    }

    // The data folder's read gives a file's bytes whatever its length, and the bounded one gives them up to
    // its limit and nothing of a longer file.
    #[test]
    fn read_file_is_the_files_bytes() {
        const MIB: usize = 1 << 20;
        let d = scratch("read");
        let p = d.join("bytes");
        let mut x: u32 = 0x2545F491;
        let all: Vec<u8> = (0..5 * MIB + 321)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x >> 11) as u8
            })
            .collect();
        for n in [0, 1, 8191, 8192, 8193, MIB - 1, MIB, MIB + 1, all.len()] {
            std::fs::write(&p, &all[..n]).unwrap();
            let want = std::fs::read(&p).unwrap();
            assert_eq!(want.len(), n);
            assert!(read_file(text(&p)).as_deref() == Some(&want[..]), "read_file of {n} bytes");
            assert!(read_system_file(text(&p)).as_deref() == Some(&want[..]), "read_system_file of {n} bytes");
            assert!(read_file_max(text(&p), usize::MAX).as_deref() == Some(&want[..]), "read_file_max of {n} bytes, no limit");
            let capped = read_file_max(text(&p), MIB);
            if n <= MIB {
                assert!(capped.as_deref() == Some(&want[..]), "read_file_max of {n} bytes, up to 1 MiB");
            } else {
                assert!(capped.is_none(), "read_file_max of {n} bytes, over 1 MiB");
            }
        }
        // the limit is the file's own length, to the byte
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(read_file_max(text(&p), 3).as_deref(), Some(&b"abc"[..]));
        assert_eq!(read_file_max(text(&p), 2), None);
        assert_eq!(read_file_max(text(&p), 0), None);
        std::fs::write(&p, b"").unwrap();
        assert_eq!(read_file_max(text(&p), 0).as_deref(), Some(&b""[..]));
        assert_eq!(read_file(text(&d.join("missing"))), None);
        assert_eq!(read_file_max(text(&d.join("missing")), MIB), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    // A file made afresh takes the name for itself: what stood there goes, and is never opened or written.
    #[test]
    fn create_fresh_nofollow_never_opens_what_is_there() {
        use std::io::Write;
        use std::os::unix::fs::MetadataExt;
        let d = scratch("fresh");
        let outside = d.join("outside.ini");
        std::fs::write(&outside, b"marker=outside-only\n").unwrap();
        let at = d.join("work.tmp");
        let plain = |p: &std::path::Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_file() && m.nlink() == 1);
        // nothing at the name
        create_fresh_nofollow(text(&at)).unwrap().write_all(b"one").unwrap();
        assert_eq!(std::fs::read(&at).unwrap(), b"one");
        // a stale file of an earlier run: made anew, not written onto
        create_fresh_nofollow(text(&at)).unwrap().write_all(b"2").unwrap();
        assert_eq!(std::fs::read(&at).unwrap(), b"2");
        // a link: the link goes, what it named stays as it was
        std::fs::remove_file(&at).unwrap();
        std::os::unix::fs::symlink(&outside, &at).unwrap();
        create_fresh_nofollow(text(&at)).unwrap().write_all(b"three").unwrap();
        assert!(plain(&at) && std::fs::read(&at).unwrap() == b"three");
        assert_eq!(std::fs::read(&outside).unwrap(), b"marker=outside-only\n");
        // a dangling link: nothing is made where it pointed
        std::fs::remove_file(&at).unwrap();
        std::os::unix::fs::symlink(d.join("nowhere"), &at).unwrap();
        create_fresh_nofollow(text(&at)).unwrap().write_all(b"four").unwrap();
        assert!(plain(&at) && !d.join("nowhere").exists());
        // a second name of another file: that file keeps its bytes
        std::fs::remove_file(&at).unwrap();
        std::fs::hard_link(&outside, &at).unwrap();
        create_fresh_nofollow(text(&at)).unwrap().write_all(b"five").unwrap();
        assert!(plain(&at) && std::fs::read(&at).unwrap() == b"five");
        assert_eq!(std::fs::read(&outside).unwrap(), b"marker=outside-only\n");
        // a FIFO no one reads: replaced by a plain file, at once
        std::fs::remove_file(&at).unwrap();
        fifo_at(&at);
        create_fresh_nofollow(text(&at)).unwrap().write_all(b"six").unwrap();
        assert!(plain(&at) && std::fs::read(&at).unwrap() == b"six");
        // a folder: refused, and it stays
        std::fs::remove_file(&at).unwrap();
        std::fs::create_dir(&at).unwrap();
        std::fs::write(at.join("kept"), b"k").unwrap();
        assert!(create_fresh_nofollow(text(&at)).is_err());
        assert!(at.is_dir() && at.join("kept").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
