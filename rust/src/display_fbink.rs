//! display_fbink.c: Kindle e-ink output through FBInk (MTK / Bellatrix3 on the Scribe 2).
//!
//! Pixels go straight into the mmap'ed framebuffer (FBInk's raw-data blitter is compiled out
//! of MINIMAL builds, and a direct copy is the fastest path anyway). FBInk then issues the
//! refresh ioctl with the waveform for the mode:
//!   FAST -> DU   (in pen mode on MTK: the driver's path for handwriting, see below)
//!   UI   -> GL16 (partial, no flash)
//!   FULL -> GC16 + flash
//! "Fast mode" (no automatic REAGL promotion) is enabled for the session like KOReader does on
//! the Scribe, so large partial refreshes keep the waveform we ask for.
//!
//! Orientation, two layers:
//!  1. Device rotation (display_set_orientation): the app draws a logical screen (portrait, or
//!     landscape when the Kindle is turned sideways) that is mapped onto the panel.
//!  2. Framebuffer quirk: on the Scribe 2 the framebuffer's memory can be upside down relative
//!     to the touch/pen panel, depending on the rotation the Kindle UI left it in (the stock UI
//!     renders rotated itself). Seen on device 2026-10-10: at fb rotate 1 every tap landed exactly
//!     180 degrees from the drawn button. Override with an empty /mnt/us/grimoire/screen-flip or
//!     /mnt/us/grimoire/screen-noflip.
//!
//! Colour panels (the Scribe Colorsoft: FBInk's has_color_panel, 32 bpp): the driver reads the
//! framebuffer for an update some time after it was sent (about 23 ms, as KOReader's developers
//! measured it on a Scribe Colorsoft; Grimoire has not run on one), in tiles. Pixels changed again
//! before that show half old, striped. So after a fast
//! refresh that is not live ink (a pressed button, the panel being dragged, a pinch, a slider)
//! Grimoire waits until the driver has read the frame (MXCFB_WAIT_FOR_MDP_COMPLETION), as KOReader
//! does. Live ink (display_live_ink: a pen stroke, erasing included) never waits, so pen reading
//! never stalls: a stroke's frames only add ink the next one sends again, and erasing ends with a
//! grayscale refresh of all it erased, which repaints anything a frame's read caught half done.

use crate::ffi_fbink as ffi;
use crate::prelude::*;
use std::cell::RefCell;
use std::ffi::CStr;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

// MTK pen mode: the driver's path for handwriting. Measured on the owner's Scribe 2 (2026-10-26,
// docs/INK-PERFORMANCE.md): a live ink frame starts on the panel within 1 ms in pen mode, while
// plain DU frames over ink still on the panel queue in the driver, up to 300 ms with quick circles.
// FBInk notes that on Bellatrix3 small pen-mode DU updates skip the MDP (the image stage that also
// rotates) at 90/270 degree rotations, and the Scribe 2 runs at fb rotate 1 or 3: they still land
// in the right place. On by default on grayscale panels; /mnt/us/grimoire/pen-mode's first word
// overrides it: "off" for plain DU, "padded" to pad small updates to 6400 px so the MDP still takes
// them (beta.4's first form), anything else (an empty file too) the direct form. Colour panels,
// where Grimoire has never run, keep plain DU unless the file asks.
// The driver requires that pen-mode updates never collide with standard ones (FBInk's note): so a
// stroke enters pen mode only while no standard update can still be on the panel (the quiet thread
// below follows them), the driver stays in it until a grayscale refresh is due, and that refresh
// first waits for the pen's last update to end.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PenCfg {
    #[default]
    Off = 0, // PEN_OFF
    Padded = 1, // PEN_PADDED
    Direct = 2, // PEN_DIRECT
}
static K_PEN_CFG: [&str; 3] = ["off", "padded", "direct"];

// Ink metrics (latency.h): live ink frames timed from the pen, by how they went out.
const INK_DU: usize = 0;
const INK_PEN_DIRECT: usize = 1;
const INK_PEN_PADDED: usize = 2;
const INK_KINDS: usize = 3;
static K_INK_KIND: [&str; INK_KINDS] = ["DU", "pen mode", "padded pen mode"];

/// display_fbink.c's file-level statics (PORTING.md, Global state): the main thread's. The quiet
/// thread's books are Q, below.
#[allow(non_snake_case)]
struct State {
    g_fd: i32,
    /// FBInkState's fields display_fbink.c reads (the shim keeps g_cfg, the FBInkConfig)
    g_st: ffi::GrimFbState,
    /// the mmap'ed framebuffer, g_fblen bytes (empty while closed: FBInk's close unmaps it)
    g_fb: &'static mut [u8],
    g_fblen: usize,
    /// last logical frame, in physical order: +4.6 MB on the 2024 Scribe
    g_previous: Canvas,
    g_frame_known: bool,
    /// the mapping covers the whole panel at 8 or 32 bpp: the guards can read it
    g_readback: bool,
    g_pen_cfg: PenCfg,
    /// the driver is in pen mode
    g_pen_mode: bool,
    /// cleared if the driver refuses pen mode
    g_pen_ok: bool,
    /// the stroke being drawn asked for the pen path (display_pen_mode)
    g_stroke_pen: bool,
    /// the last update sent in pen mode
    g_pen_marker: u32,
    g_pen_last_ms: u32,
    /// with ink metrics both files are read again, every 2 s at most
    g_pen_checked: u32,
    /// strokes that kept plain DU: a standard update was still on the panel
    g_pen_busy: u32,
    /// (this stroke counted)
    g_pen_busy_noted: bool,
    // /mnt/us/grimoire/latency-ab (a device test, with ink metrics): strokes take turns in blocks of
    // 12 at plain DU, direct pen mode and padded pen mode, so one session times all three (in the
    // same conditions: the same pages, the same hand).
    g_ab: bool,
    g_strokes: u32,
    g_count: [i32; 3],
    g_fail: i32,
    g_ok: i32,
    g_stats_at: u32,
    /// when the last refresh went out (paces the first-frame retries)
    g_last_refresh: u32,
    g_last_submit_us: u64,
    g_copy_us: u64,
    g_submit_us: u64,
    g_copy_max_us: u64,
    g_submit_max_us: u64,
    g_perf_frames: u32,
    g_perf_at: u32,
    g_perf_enabled: bool,
    g_lat_pen: [LatHist; INK_KINDS],
    g_lat_oldest: [LatHist; INK_KINDS],
    g_lat_raw: [LatHist; INK_KINDS],
    g_lat_hop: [LatHist; INK_KINDS],
    g_raw_miss: [u32; INK_KINDS],
    /// refreshes sent; the first few are logged in full (device bring-up)
    g_presents: i32,
    /// framebuffer is 180 degrees from the panel's touch orientation
    g_flip: bool,
    /// device rotation, quarter turns clockwise
    g_rot: i32,
    /// panel (framebuffer view) size
    PW: i32,
    PH: i32,
    /// logical screen size (swapped when turned sideways)
    LW: i32,
    LH: i32,
    /// a colour panel: fast refreshes outside live ink wait for the driver's read
    g_mdp_wait: bool,
    last_fast_wait: bool,
    /// waits logged (the first few, for a tester's log)
    g_mdp_logged: i32,
    /// strokes being drawn (display_live_ink): their fast frames never wait
    g_ink: LiveInk,
    /// Grimoire's colors inverted (user setting), on top of the fb's own flag
    g_invert: bool,
    /// a live (FAST) frame is being copied (see copy_rect)
    g_live: bool,
    /// Logical area pushed with DU (dithered) and not yet repainted in grayscale: the framebuffer
    /// there legitimately differs from the canvas, so the overwrite checks skip it.
    g_live_area: Rect,
    /// display_skip_overlay
    g_overlay: Rect,
    /// display_skip_status
    g_status: Rect,
}

impl State {
    fn new() -> State {
        State {
            g_fd: -1,
            g_st: ffi::GrimFbState::default(),
            g_fb: &mut [],
            g_fblen: 0,
            g_previous: Canvas::default(),
            g_frame_known: false,
            g_readback: false,
            g_pen_cfg: PenCfg::Off,
            g_pen_mode: false,
            g_pen_ok: true,
            g_stroke_pen: false,
            g_pen_marker: 0,
            g_pen_last_ms: 0,
            g_pen_checked: 0,
            g_pen_busy: 0,
            g_pen_busy_noted: false,
            g_ab: false,
            g_strokes: 0,
            g_count: [0; 3],
            g_fail: 0,
            g_ok: 0,
            g_stats_at: 0,
            g_last_refresh: 0,
            g_last_submit_us: 0,
            g_copy_us: 0,
            g_submit_us: 0,
            g_copy_max_us: 0,
            g_submit_max_us: 0,
            g_perf_frames: 0,
            g_perf_at: 0,
            g_perf_enabled: false,
            g_lat_pen: Default::default(),
            g_lat_oldest: Default::default(),
            g_lat_raw: Default::default(),
            g_lat_hop: Default::default(),
            g_raw_miss: [0; INK_KINDS],
            g_presents: 0,
            g_flip: false,
            g_rot: 0,
            PW: 0,
            PH: 0,
            LW: 0,
            LH: 0,
            g_mdp_wait: false,
            last_fast_wait: false,
            g_mdp_logged: 0,
            g_ink: LiveInk::default(),
            g_invert: false,
            g_live: false,
            g_live_area: RECT_EMPTY,
            g_overlay: RECT_EMPTY,
            g_status: RECT_EMPTY,
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::new());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// ---- FBInk, through the shim: safe wrappers with FBInk's names ----

fn fbink_open() -> i32 {
    // SAFETY: opens the framebuffer; no arguments.
    unsafe { ffi::grim_fbink_open() }
}
fn fbink_close(fd: i32) -> i32 {
    // SAFETY: closes FBInk's fd and unmaps its framebuffer; the caller has dropped its slice of it.
    unsafe { ffi::grim_fbink_close(fd) }
}
fn fbink_init(fd: i32) -> i32 {
    // SAFETY: fbink_init on the fd fbink_open gave, with the shim's own config.
    unsafe { ffi::grim_fbink_init(fd) }
}
fn fbink_get_state() -> ffi::GrimFbState {
    let mut st = ffi::GrimFbState::default();
    // SAFETY: the shim fills the GrimFbState it is handed, our local, and nothing else.
    unsafe { ffi::grim_fbink_get_state(&mut st) };
    st
}
fn fbink_refresh(fd: i32, top: u32, left: u32, width: u32, height: u32, wfm: i32, flashing: bool) -> i32 {
    // SAFETY: plain values; FBInk checks the fd and the region itself.
    unsafe { ffi::grim_fbink_refresh(fd, top, left, width, height, wfm, flashing) }
}
fn fbink_get_last_marker() -> u32 {
    // SAFETY: reads FBInk's last marker; no arguments.
    unsafe { ffi::grim_fbink_get_last_marker() }
}
fn fbink_wait_for_complete(fd: i32, marker: u32) -> i32 {
    // SAFETY: plain values. The callers keep the hard rule: never on a marker the quiet thread
    // waits on (display_wait checks its books first; pen_leave's marker went out in pen mode, which
    // the quiet thread never takes on).
    unsafe { ffi::grim_fbink_wait_for_complete(fd, marker) }
}
fn fbink_mtk_toggle_auto_reagl(fd: i32, on: bool) -> i32 {
    // SAFETY: plain values; an ioctl on FBInk's fd.
    unsafe { ffi::grim_fbink_mtk_toggle_auto_reagl(fd, on) }
}
fn fbink_mtk_toggle_pen_mode(fd: i32, on: bool) -> i32 {
    // SAFETY: plain values; an ioctl on FBInk's fd.
    unsafe { ffi::grim_fbink_mtk_toggle_pen_mode(fd, on) }
}

/// access(path, F_OK) == 0
fn exists(path: &CStr) -> bool {
    // SAFETY: path is NUL-terminated; access only reads it.
    unsafe { libc::access(path.as_ptr(), libc::F_OK) == 0 }
}

/// A C char array's text up to its NUL (FBInk's names, the driver's id), for the log. (c_char is
/// u8 on the Kindle's ARM.)
fn c_text(a: &[libc::c_char]) -> String {
    let b: Vec<u8> = a.iter().take_while(|&&c| c != 0).copied().collect();
    String::from_utf8_lossy(&b).into_owned()
}

// Logical screen pixel -> framebuffer pixel (device rotation, then the framebuffer quirk).
fn to_fb(s: &State, lx: i32, ly: i32) -> (i32, i32) {
    let (mut px, mut py) = match s.g_rot {
        1 => (ly, s.PH - 1 - lx),
        2 => (s.PW - 1 - lx, s.PH - 1 - ly),
        3 => (s.PW - 1 - ly, lx),
        _ => (lx, ly),
    };
    if s.g_flip {
        px = s.PW - 1 - px;
        py = s.PH - 1 - py;
    }
    (px, py)
}

// Byte offset of logical pixel (lx, ly) in the framebuffer: one byte per pixel at 8 bpp, four at
// 32 bpp (the Scribe Colorsoft).
fn fb_index(s: &State, lx: i32, ly: i32) -> usize {
    let (fx, fy) = to_fb(s, lx, ly);
    fbcheck_offset(fx, fy, s.g_st.scanline_stride as usize, s.g_st.bpp as i32)
}

fn clock_us() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: clock_gettime writes one timespec into our local.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } != 0 {
        return 0;
    }
    (ts.tv_sec as u64).wrapping_mul(1000000).wrapping_add(ts.tv_nsec as u64 / 1000)
}

// The gray the panel's memory holds at logical (lx, ly), Grimoire's own inversion undone: what the
// canvas has there unless another process drew. Only while g_readback.
fn fb_gray(s: &State, lx: i32, ly: i32) -> u8 {
    let flip = if s.g_st.bpp == 8 { s.g_st.inverted_grayscale != s.g_invert } else { s.g_invert }; // as copy_rect writes
    fbcheck_gray(&s.g_fb[fb_index(s, lx, ly)..], s.g_st.bpp as i32, flip)
}

fn pixel_format_name(f: u32) -> &'static str {
    match f {
        ffi::FBINK_PXFMT_Y4 => "Y4",
        ffi::FBINK_PXFMT_Y8 => "Y8",
        ffi::FBINK_PXFMT_BGR565 => "BGR565",
        ffi::FBINK_PXFMT_RGB565 => "RGB565",
        ffi::FBINK_PXFMT_BGR24 => "BGR24",
        ffi::FBINK_PXFMT_RGB24 => "RGB24",
        ffi::FBINK_PXFMT_BGRA => "BGRA",
        ffi::FBINK_PXFMT_RGBA => "RGBA",
        ffi::FBINK_PXFMT_BGR32 => "BGR32",
        ffi::FBINK_PXFMT_RGB32 => "RGB32",
        _ => "unknown",
    }
}

// Logical rect -> the framebuffer rect it covers.
fn fb_rect(s: &State, r: Rect) -> Rect {
    let (x0, y0) = to_fb(s, r.x, r.y);
    let (x1, y1) = to_fb(s, r.x + r.w - 1, r.y + r.h - 1);
    let lx = if x0 < x1 { x0 } else { x1 };
    let ly = if y0 < y1 { y0 } else { y1 };
    Rect { x: lx, y: ly, w: (x1 - x0).abs() + 1, h: (y1 - y0).abs() + 1 }
}

// <asm-generic/ioctl.h>'s _IOC, which the Kindle's ARM kernel uses: direction (1 write, 2 read) in
// the top 2 bits, then 14 bits of size, the type byte and the number.
const fn ioc(dir: u32, ty: u8, nr: u32, size: usize) -> u32 {
    (dir << 30) | ((size as u32) << 16) | ((ty as u32) << 8) | nr
}

// Not in FBInk's eink/mtk-kindle.h: blocks until the driver has read the framebuffer for the
// update with this marker (KOReader uses it on the Scribe Colorsoft's FW 5.19.6 kernels).
// (_IOW('F', 0x4D, uint32_t))
const MXCFB_WAIT_FOR_MDP_COMPLETION: u32 = ioc(1, b'F', 0x4D, std::mem::size_of::<u32>());

// The driver's waits, on an fd of their own (the thread below), in its marker protocol. Never the
// submission wait, and never two waits on one of a marker's signals. The driver (hwtcon_v2) ends an
// update by signalling each of its markers twice, submitted and then complete, and a marker that
// still has waiters is freed by the last one to leave: a submission waiter woken by the first
// signal can free it before the second, and the kernel panics (the Kindle reboots on its watchdog;
// the latency report's submission wait did this). Each signal wakes one waiter, so a second waiter
// on one sits out the driver's 5 s timeout. (mdp_wait, on colour panels, waits on a signal of its
// own.)
#[allow(non_camel_case_types)]
#[repr(C)]
struct grim_marker_data {
    update_marker: u32,
    collision_test: u32,
}
// (_IOWR('F', 0x2F, struct grim_marker_data))
const GRIM_WAIT_FOR_UPDATE_COMPLETE: u32 = ioc(3, b'F', 0x2F, std::mem::size_of::<grim_marker_data>());
fn wait_done(fd: i32, marker: u32) -> i32 {
    let mut d = grim_marker_data { update_marker: marker, collision_test: 0 };
    // SAFETY: the driver reads and writes the grim_marker_data it is handed, our local, for the
    // length of the call; fd is the quiet thread's own framebuffer fd.
    unsafe { libc::ioctl(fd, GRIM_WAIT_FOR_UPDATE_COMPLETE as libc::Ioctl, &mut d as *mut grim_marker_data) }
}
fn start_thread(f: fn()) -> bool {
    // (The handle is dropped: the thread is detached, as pthread_detach leaves it.)
    std::thread::Builder::new().spawn(f).is_ok()
}

// Standard updates still on the panel, for pen mode's no-collision rule. A thread waits for each
// grayscale one to end, in order (their waveforms differ in length), and for the newest standard DU
// one (DU updates all last as long, so the newest ends last). It only waits and keeps books: no
// allocation, no log, nothing a bgjob fork could find half done.
const QUIET_GRAY: usize = 32;
struct Quiet {
    on: bool,
    fd: i32,
    errors: i32,
    gray: [u32; QUIET_GRAY],
    gray_head: i32,
    gray_n: i32,
    /// the queue was full: unknown until 2 s later
    overflow_ms: u32,
    du_sent: u32,
    du_wait: u32,
    du_done: u32,
    /// the newest marker it took on
    handed: u32,
}
static Q: Mutex<Quiet> = Mutex::new(Quiet {
    on: false,
    fd: -1,
    errors: 0,
    gray: [0; QUIET_GRAY],
    gray_head: 0,
    gray_n: 0,
    overflow_ms: 0,
    du_sent: 0,
    du_wait: 0,
    du_done: 0,
    handed: 0,
});
/// work for the thread
static Q_CV: Condvar = Condvar::new();
/// it finished some (display_wait)
static Q_IDLE: Condvar = Condvar::new();

fn qlock() -> MutexGuard<'static, Quiet> {
    // (Nothing that holds it can panic, and the Kindle build aborts on a panic: a poisoned lock
    // would still hold the books as they are.)
    Q.lock().unwrap_or_else(|e| e.into_inner())
}

fn quiet_main() {
    let mut q = qlock();
    loop {
        while q.gray_n == 0 && q.du_wait == 0 {
            q = Q_CV.wait(q).unwrap_or_else(|e| e.into_inner());
        }
        let gray = q.gray_n > 0;
        let m = if gray { q.gray[q.gray_head as usize] } else { q.du_wait };
        if !gray {
            q.du_wait = 0;
        }
        let fd = q.fd;
        drop(q);
        let rc = wait_done(fd, m);
        if rc < 0 {
            std::thread::sleep(Duration::from_secs(1)); // no word from the driver: give the update its longest time
        }
        q = qlock();
        if rc < 0 {
            q.errors += 1;
            if q.errors >= 3 {
                q.on = false; // pen mode off for the session (logged by the main thread)
            }
        }
        if gray {
            q.gray_head = (q.gray_head + 1) % QUIET_GRAY as i32;
            q.gray_n -= 1;
        } else {
            q.du_done = m;
        }
        Q_IDLE.notify_all();
    }
}

fn quiet_sent(marker: u32, gray: bool) {
    let mut q = qlock();
    if q.fd < 0 {
        return;
    }
    if !gray {
        q.du_sent = marker;
        q.du_wait = marker;
        q.handed = marker;
    } else if (q.gray_n as usize) < QUIET_GRAY {
        let at = (q.gray_head + q.gray_n) as usize % QUIET_GRAY;
        q.gray[at] = marker;
        q.handed = marker;
        q.gray_n += 1;
    } else {
        q.overflow_ms = now_ms() | 1;
    }
    Q_CV.notify_one();
}

// No standard update can still be on the panel.
fn quiet_now() -> bool {
    let q = qlock();
    q.on && q.gray_n == 0 && q.du_done == q.du_sent && (q.overflow_ms == 0 || now_ms().wrapping_sub(q.overflow_ms) > 2000)
}

// A thread with its own fd on the framebuffer (FBInk's stays the main thread's); false if either fails.
// (The C hands it Q's fd and on: they are the only ones it is ever given.)
fn open_watch(f: fn()) -> bool {
    // SAFETY: open() of a NUL-terminated constant path; the fd it gives is ours.
    let fd = unsafe { libc::open(c"/dev/fb0".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
    {
        let mut q = qlock();
        q.fd = fd;
        if fd < 0 {
            return false;
        }
        q.on = true;
    }
    if start_thread(f) {
        return true;
    }
    let mut q = qlock();
    q.on = false;
    // SAFETY: the fd opened above, which nothing else uses (the thread did not start).
    unsafe { libc::close(fd) };
    q.fd = -1;
    false
}

fn lat_reset(s: &mut State) {
    for k in 0..INK_KINDS {
        lat_hist_init(&mut s.g_lat_pen[k], 100);
        lat_hist_init(&mut s.g_lat_oldest[k], 100);
        lat_hist_init(&mut s.g_lat_raw[k], 100);
        lat_hist_init(&mut s.g_lat_hop[k], 100);
        s.g_raw_miss[k] = 0;
    }
}

// The panel's temperature as the driver reads it ("temp:25 temp_zone:8"): the waveforms' lengths
// follow it.
fn panel_temp(out: &mut String, cap: usize) {
    csnprintf!(out, cap, "?");
    let Some(data) = read_file("/proc/hwtcon/temp") else { return };
    // fgets: the first line, newline included, at most cap - 1 bytes; nothing read leaves "?"
    let mut line = &data[..data.len().min(cap.saturating_sub(1))];
    if let Some(nl) = line.iter().position(|&b| b == b'\n') {
        line = &line[..nl + 1];
    }
    if line.is_empty() {
        return;
    }
    // (cut at the first CR or LF, and where the C string would end)
    let end = line.iter().position(|&b| b == b'\r' || b == b'\n' || b == 0).unwrap_or(line.len());
    *out = String::from_utf8_lossy(&line[..end]).into_owned();
}

fn lat_log(s: &mut State) {
    let (mut a, mut b, mut c, mut d) = (String::new(), String::new(), String::new(), String::new());
    for k in 0..INK_KINDS {
        if s.g_lat_pen[k].n == 0 {
            continue;
        }
        lat_hist_format(&s.g_lat_pen[k], &mut a, 96);
        lat_hist_format(&s.g_lat_oldest[k], &mut b, 96);
        lat_hist_format(&s.g_lat_raw[k], &mut c, 96);
        lat_hist_format(&s.g_lat_hop[k], &mut d, 96);
        qlog!(
            concat!(
                "display: ink latency, %s: pen->submit %s; oldest pen->submit %s; raw->submit %s; relay hop %s; ",
                "unmatched %u"
            ),
            K_INK_KIND[k],
            &a,
            &b,
            &c,
            &d,
            s.g_raw_miss[k]
        );
    }
    let mut t = String::new();
    panel_temp(&mut t, 48);
    let quiet_errors = qlock().errors;
    qlog!(
        "display: driver: panel %s; strokes kept on plain DU %u; standard-update waits failed %d%s",
        &t,
        s.g_pen_busy,
        quiet_errors,
        if quiet_errors >= 3 { " (no pen mode)" } else { "" }
    );
    lat_reset(s);
}

pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {
    let (lw, lh) = with(|s| {
        s.g_frame_known = false;
        s.g_rot = quarter_turns & 3;
        s.LW = if (s.g_rot & 1) != 0 { s.PH } else { s.PW };
        s.LH = if (s.g_rot & 1) != 0 { s.PW } else { s.PH };
        qlog!("display: orientation %d quarter turns -> logical %dx%d", s.g_rot, s.LW, s.LH);
        (s.LW, s.LH)
    });
    *w = lw;
    *h = lh;
}

fn set_pen_mode(s: &mut State, on: bool) {
    if !s.g_st.is_mtk || !s.g_pen_ok || on == s.g_pen_mode {
        return;
    }
    let rc = fbink_mtk_toggle_pen_mode(s.g_fd, on);
    if s.g_presents < 400 {
        qlog!("display: pen mode %s -> %d", if on { "on" } else { "off" }, rc);
    }
    if rc < 0 {
        qlog!("display: pen mode %s refused (%d), using plain DU", if on { "on" } else { "off" }, rc);
        s.g_pen_ok = false;
        s.g_pen_mode = false;
        return;
    }
    s.g_pen_mode = on;
}

pub fn display_live_ink(on: bool) {
    with(|s| live_ink_mark(&mut s.g_ink, on, now_ms()));
}

fn read_pen_cfg(s: &State) -> PenCfg {
    let Some(data) = read_file("/mnt/us/grimoire/pen-mode") else {
        return if s.g_st.has_color_panel { PenCfg::Off } else { PenCfg::Direct };
    };
    // fscanf "%15s": white space skipped, then the first word, at most 15 bytes; none leaves ""
    let is_space = |b: u8| b == b' ' || (9..=13).contains(&b);
    let start = data.iter().position(|&b| !is_space(b)).unwrap_or(data.len());
    let word = &data[start..];
    let len = word.iter().take(15).take_while(|&&b| !is_space(b)).count();
    let w = &word[..len];
    if w == b"off" {
        PenCfg::Off
    } else if w == b"padded" {
        PenCfg::Padded
    } else {
        PenCfg::Direct
    }
}

// A stroke of ink starts (on) or ends. Its fast frames take pen mode when it is on and no standard
// update can still be on the panel (display_present); the driver stays in pen mode past the stroke,
// until a grayscale refresh is due (pen_leave).
pub fn display_pen_mode(on: bool) {
    display_live_ink(on);
    with(|s| {
        s.g_stroke_pen = on;
        if !on {
            return;
        }
        s.g_strokes = s.g_strokes.wrapping_add(1);
        s.g_pen_busy_noted = false;
        let mut want = s.g_pen_cfg;
        if s.g_perf_enabled && now_ms().wrapping_sub(s.g_pen_checked) >= 2000 {
            s.g_pen_checked = now_ms();
            let ab = exists(c"/mnt/us/grimoire/latency-ab");
            if ab != s.g_ab {
                qlog!("display: latency-ab %s", if ab { "on: strokes take turns, 12 each" } else { "off" });
            }
            s.g_ab = ab;
            if !s.g_ab {
                want = read_pen_cfg(s);
            }
        }
        if s.g_ab {
            const TURNS: [PenCfg; 3] = [PenCfg::Off, PenCfg::Direct, PenCfg::Padded];
            want = TURNS[((s.g_strokes / 12) % 3) as usize];
        }
        if want != s.g_pen_cfg {
            qlog!("display: pen mode %s (stroke %u)", K_PEN_CFG[want as usize], s.g_strokes);
            s.g_pen_cfg = want;
        }
    })
}

// Before a standard update: out of pen mode, once the panel has finished the pen's last update (a
// standard one over it would collide). Grayscale refreshes come after a pause, so this rarely waits.
fn pen_leave(s: &mut State) {
    if !s.g_pen_mode {
        return;
    }
    if now_ms().wrapping_sub(s.g_pen_last_ms) < 2000 {
        fbink_wait_for_complete(s.g_fd, s.g_pen_marker);
    }
    set_pen_mode(s, false);
}

// After a fast refresh on a colour panel: wait until the driver has read the frame, so the next
// change to those pixels cannot show through half done. A driver that refuses the wait, or takes
// more than a second over it, is not asked again this session (logged once).
fn mdp_wait(s: &mut State) {
    let mut marker = fbink_get_last_marker();
    let t0 = now_ms();
    // SAFETY: the driver reads the u32 marker it is handed, our local, for the length of the call;
    // g_fd is FBInk's open framebuffer fd.
    let rc = unsafe { libc::ioctl(s.g_fd, MXCFB_WAIT_FOR_MDP_COMPLETION as libc::Ioctl, &mut marker as *mut u32) };
    let e = errno();
    let took = now_ms().wrapping_sub(t0);
    if rc < 0 && e == libc::EINTR {
        return;
    }
    if rc < 0 || took > 1000 {
        s.g_mdp_wait = false;
        qlog!(
            concat!(
                "display: MXCFB_WAIT_FOR_MDP_COMPLETION %s after %u ms (marker %u): no longer waiting for the colour ",
                "panel's read"
            ),
            &if rc < 0 { strerror(e) } else { "took too long".to_string() },
            took,
            marker
        );
        return;
    }
    if s.g_mdp_logged < 20 {
        s.g_mdp_logged += 1;
        qlog!("display: waited %u ms for the colour panel's driver to read fast refresh %u", took, marker);
    }
}

pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    with(|s| {
        s.g_fd = fbink_open();
        if s.g_fd < 0 {
            qlog!("display: fbink_open failed: %s", &strerror(errno()));
            return -1;
        }
        // (the shim zeroes its FBInkConfig and sets is_quiet first, as the C's memset does)
        if fbink_init(s.g_fd) < 0 {
            qlog!("display: fbink_init failed");
            fbink_close(s.g_fd);
            s.g_fd = -1;
            return -1;
        }
        s.g_st = fbink_get_state();
        let mut len: libc::size_t = 0;
        // SAFETY: the shim writes the mapping's length into our local and returns its address.
        let p = unsafe { ffi::grim_fbink_get_fb_pointer(s.g_fd, &mut len) };
        s.g_fblen = len;
        s.g_fb = if p.is_null() {
            &mut []
        } else {
            // SAFETY: FBInk mmap'ed g_fblen bytes of the framebuffer, readable and writable, at p,
            // and keeps them mapped until fbink_close, before which display_close (and the failure
            // below) drops this slice. Only the main thread touches it (the quiet thread has its
            // own fd and never the mapping); bytes need no alignment.
            unsafe { std::slice::from_raw_parts_mut(p, len) }
        };
        let st = s.g_st;
        let (name, codename, platform) = (c_text(&st.device_name), c_text(&st.device_codename), c_text(&st.device_platform));
        qlog!(
            "display: %s (%s) %ux%u %ubpp stride %u rota %u dpi %u mtk %d inverted %d",
            &name,
            &codename,
            st.screen_width,
            st.screen_height,
            st.bpp,
            st.scanline_stride,
            st.current_rota,
            st.screen_dpi,
            st.is_mtk,
            st.inverted_grayscale
        );
        qlog!(
            concat!(
                "display: FBInk device id 0x%X, platform %s, pixel format %s (%u), color panel %d, eclipse waveforms %d, ",
                "hw invert %d"
            ),
            st.device_id,
            &platform,
            pixel_format_name(st.pixel_format),
            st.pixel_format,
            st.has_color_panel,
            st.has_eclipse_wfm,
            st.can_hw_invert
        );
        // The driver's own description, untouched, for devices nobody here has seen (channel order,
        // a second buffer, the physical size).
        let mut v = ffi::fb_var_screeninfo::default();
        let mut f = ffi::fb_fix_screeninfo::default();
        // SAFETY: the driver fills the fb_var_screeninfo it is handed, our local, laid out as
        // <linux/fb.h> (its size is checked in ffi_fbink.rs).
        let have_v =
            unsafe { libc::ioctl(s.g_fd, ffi::FBIOGET_VSCREENINFO as libc::Ioctl, &mut v as *mut ffi::fb_var_screeninfo) } == 0;
        if have_v {
            qlog!(
                concat!(
                    "display: vinfo %ux%u virtual %ux%u offset %u,%u bpp %u grayscale %u red %u/%u green %u/%u blue %u/%u ",
                    "transp %u/%u rotate %u size %ux%u mm"
                ),
                v.xres,
                v.yres,
                v.xres_virtual,
                v.yres_virtual,
                v.xoffset,
                v.yoffset,
                v.bits_per_pixel,
                v.grayscale,
                v.red.offset,
                v.red.length,
                v.green.offset,
                v.green.length,
                v.blue.offset,
                v.blue.length,
                v.transp.offset,
                v.transp.length,
                v.rotate,
                v.width,
                v.height
            );
        } else {
            qlog!("display: FBIOGET_VSCREENINFO failed: %s", &strerror(errno()));
        }
        // SAFETY: the same, for the fb_fix_screeninfo.
        if unsafe { libc::ioctl(s.g_fd, ffi::FBIOGET_FSCREENINFO as libc::Ioctl, &mut f as *mut ffi::fb_fix_screeninfo) } == 0 {
            qlog!(
                "display: finfo id \"%.16s\" line_length %u smem_len %u, mapped %zu bytes",
                &c_text(&f.id),
                f.line_length,
                f.smem_len,
                s.g_fblen
            );
        } else {
            qlog!("display: FBIOGET_FSCREENINFO failed: %s", &strerror(errno()));
        }
        if p.is_null() || (st.bpp != 8 && st.bpp != 32) {
            qlog!("display: unsupported framebuffer (bpp %u)", st.bpp);
            s.g_fb = &mut [];
            fbink_close(s.g_fd);
            s.g_fd = -1;
            return -1;
        }
        if st.is_mtk {
            qlog!("display: auto REAGL off -> %d", fbink_mtk_toggle_auto_reagl(s.g_fd, false));
        }
        s.g_flip = st.current_rota == 1; // FB_ROTATE_CW: observed upside down vs. the panel
        if exists(c"/mnt/us/grimoire/screen-flip") {
            s.g_flip = true;
        }
        if exists(c"/mnt/us/grimoire/screen-noflip") {
            s.g_flip = false;
        }
        qlog!("display: fb rotate %u -> %s", st.current_rota, if s.g_flip { "drawing rotated 180 degrees" } else { "drawing as is" });
        s.g_pen_cfg = read_pen_cfg(s);
        s.g_perf_enabled = exists(c"/mnt/us/grimoire/ink-metrics");
        s.g_ab = s.g_perf_enabled && exists(c"/mnt/us/grimoire/latency-ab");
        if st.is_mtk {
            qlog!(
                "display: MTK pen mode %s",
                if s.g_ab {
                    "taking turns with plain DU (latency-ab)"
                } else if s.g_pen_cfg == PenCfg::Off {
                    "off (plain DU for live ink)"
                } else if s.g_pen_cfg == PenCfg::Direct {
                    "on"
                } else {
                    "on, padded"
                }
            );
        }
        if st.is_mtk && (s.g_pen_cfg != PenCfg::Off || s.g_ab || s.g_perf_enabled) {
            // display_wait's deadline on the monotonic clock: a step of the wall clock (the Kindle's
            // time sync) must not end it early. (Rust's Condvar times its waits on CLOCK_MONOTONIC,
            // as the C sets the cond's clock to.)
            if !open_watch(quiet_main) {
                qlog!("display: no thread to follow standard updates (%s): no pen mode", &strerror(errno()));
            }
        }
        if s.g_perf_enabled {
            lat_reset(s);
        }
        s.g_mdp_wait = st.has_color_panel && st.bpp == 32;
        if s.g_mdp_wait {
            qlog!("display: colour panel: fast refreshes other than live ink wait until the driver has read the frame");
        }
        s.PW = st.screen_width as i32;
        s.LW = s.PW;
        s.PH = st.screen_height as i32;
        s.LH = s.PH;
        s.g_frame_known = false;
        if canvas_init(&mut s.g_previous, s.PW, s.PH) != 0 {
            qlog!("display: no memory for the live pixel cache; dithering whole fast rectangles");
        }
        *w = s.PW;
        *h = s.PH;
        // The screen guard reads the panel back; it must stay inside the mapping.
        s.g_readback = s.PW > 0
            && s.PH > 0
            && fbcheck_offset(s.PW - 1, s.PH - 1, st.scanline_stride as usize, st.bpp as i32).wrapping_add(fbcheck_px_bytes(st.bpp as i32))
                <= s.g_fblen;
        if !s.g_readback {
            qlog!("display: the framebuffer mapping (%zu bytes) does not cover the panel: no readback, no screen guard", s.g_fblen);
        }
        // FBInk never reports 0 dpi: for a device it does not know it guesses 167 (fbink.c), which would
        // lay a Scribe's UI out at about half size. Then the driver's physical size decides.
        let mut from = DpiSource::default();
        let d = fbcheck_dpi(
            &name,
            st.screen_dpi,
            if have_v { v.xres } else { 0 },
            if have_v { v.yres } else { 0 },
            if have_v { v.width } else { 0 },
            if have_v { v.height } else { 0 },
            &mut from,
        );
        if from == DpiSource::PanelSize {
            qlog!(
                concat!(
                    "display: FBInk does not know this device (\"%s\", it guesses %u dpi); %ux%u px over %ux%u mm makes ",
                    "%.0f dpi"
                ),
                &name,
                st.screen_dpi,
                v.xres,
                v.yres,
                v.width,
                v.height,
                d
            );
        } else if from == DpiSource::Default {
            qlog!(
                concat!(
                    "display: FBInk does not know this device (\"%s\", it guesses %u dpi) and the driver gives no usable ",
                    "size (%ux%u mm); using %.0f dpi"
                ),
                &name,
                st.screen_dpi,
                v.width,
                v.height,
                d
            );
        }
        *dpi = d;
        0
    })
}

// DU is 1-bit. Changed pixels in live (FAST) frames are ordered-dithered (8x8 Bayer), so a gray shows as the same
// overall darkness in black and white dots: the stroke looks as wide and as dark while drawing
// as after the GL16 settle on pen-up. (Thresholding made light pencil vanish; forcing every inked
// pixel black made live strokes look thick and dark, then "fade" when they settled.) Over gray
// paper the dots are black and the paper itself: fbpack_row only moves a shown pixel toward its
// new value, so no pixel goes white and then black a waveform later.
// (g_live, g_live_area and g_overlay are in State.)
pub fn display_skip_overlay(r: Rect) {
    with(|s| s.g_overlay = r);
}
pub fn display_skip_status(r: Rect) {
    with(|s| s.g_status = r);
}
/// Whether the driver is in pen mode now. It stays so past a stroke, until a grayscale refresh
/// is due (pen_leave), and that refresh waits for the pen's last frame: the clock and battery
/// corner (status.rs) keeps its own refreshes out of that wait.
pub fn display_pen_mode_on() -> bool {
    with(|s| s.g_pen_mode)
}
pub fn display_set_invert(on: bool) {
    with(|s| {
        if on != s.g_invert {
            s.g_frame_known = false;
        }
        s.g_invert = on;
    })
}

// A full-screen copy takes 45-100 ms on the Kindle (fbpack_row, 20-40 ns a pixel): the pens are read
// every 64 rows of it (work_pump, common.h), which a stroke's small frames never reach.
fn copy_rect(s: &Canvas, r: Rect) {
    for y in r.y..r.y + r.h {
        // (one row under the state's borrow; the pump runs outside it)
        if !with(|st| copy_row(st, s, r, y)) {
            return;
        }
        if ((y - r.y) & 63) == 63 {
            work_pump();
        }
    }
}

// copy_rect's row y; false where the C's loop returns (the row would leave the mapping).
fn copy_row(st: &mut State, s: &Canvas, r: Rect, y: i32) -> bool {
    let stride = st.g_st.scanline_stride as usize;
    let px_bytes: usize = if st.g_st.bpp == 8 { 1 } else { 4 };
    let src = &s.px[y as usize * s.stride as usize + r.x as usize..];
    // Along a logical row the framebuffer moves by a constant step (a pixel or a whole
    // scanline, either direction, depending on the rotation).
    let (fx0, fy0) = to_fb(st, r.x, y);
    let (fx1, fy1) = to_fb(st, r.x + 1, y);
    let step = (fy1 - fy0) as isize * stride as isize + (fx1 - fx0) as isize * px_bytes as isize;
    let off = fy0 as isize * stride as isize + fx0 as isize * px_bytes as isize;
    let last = off + step * (r.w - 1) as isize;
    if off < 0 || last < 0 || (if off > last { off } else { last }) as usize + px_bytes > st.g_fblen {
        return false;
    }
    let previous_at = (fy0 as usize).wrapping_mul(st.g_previous.stride as usize).wrapping_add(fx0 as usize);
    let previous_step = (fy1 - fy0) as isize * st.g_previous.stride as isize + (fx1 - fx0) as isize;
    let flip_px = if st.g_st.bpp == 8 { st.g_st.inverted_grayscale != st.g_invert } else { st.g_invert };
    let (bpp, live, known) = (st.g_st.bpp as i32, st.g_live, st.g_frame_known);
    let previous = if st.g_previous.px.is_empty() { None } else { Some(&mut st.g_previous.px[..]) };
    fbpack_row(&mut st.g_fb[..], off as usize, step, previous, previous_at, previous_step, src, r.w, r.x, y, bpp, flip_px, live, known);
    true
}

// Sends the refresh of framebuffer rect r and keeps the books: counts, failures, and whether the
// panel has taken any refresh at all (display_broken). (The C hands it the FBInkConfig for the
// refresh; the shim keeps the config, so here its waveform and flash.)
fn send_refresh(s: &mut State, r: Rect, mode: RefreshMode, wfm: i32, flashing: bool) -> i32 {
    let t0 = if s.g_perf_enabled { clock_us() } else { 0 };
    let ret = fbink_refresh(s.g_fd, r.y as u32, r.x as u32, r.w as u32, r.h as u32, wfm, flashing);
    let e = errno();
    if s.g_perf_enabled {
        s.g_last_submit_us = clock_us().wrapping_sub(t0);
    }
    s.g_count[mode as usize] += 1;
    if ret < 0 {
        s.g_fail += 1;
    } else {
        s.g_ok += 1;
    }
    s.g_last_refresh = now_ms();
    if now_ms().wrapping_sub(s.g_stats_at) > 30000 {
        s.g_stats_at = now_ms();
        qlog!(
            "display: refreshes so far: fast %d, ui %d, full %d, failed %d",
            s.g_count[0],
            s.g_count[1],
            s.g_count[2],
            s.g_fail
        );
    }
    if ret < 0 {
        qlog!("display: refresh %d,%d %dx%d mode %d failed (%d): %s", r.x, r.y, r.w, r.h, mode as i32, ret, &strerror(e));
    }
    ret
}

pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {
    let Some(r) = with(|s| {
        if s.g_fd < 0 {
            return None;
        }
        let mut r = rect_intersect(r, Rect { x: 0, y: 0, w: s.LW, h: s.LH });
        r = rect_intersect(r, Rect { x: 0, y: 0, w: screen.w, h: screen.h });
        if rect_is_empty(r) {
            return None;
        }
        // Pen mode only tolerates DU updates; anything else must leave it first (pen_leave: the only
        // wait, for the pen's last update, which grayscale settles find long done since they run after
        // a pause). No other wait for pending DU updates here: blocking stalls input reading, the
        // kernel's evdev buffer overflows and strokes get cut.
        if mode != RefreshMode::Fast {
            pen_leave(s);
        } else if s.g_stroke_pen && s.g_pen_cfg != PenCfg::Off && !s.g_pen_mode && s.g_pen_ok {
            if quiet_now() {
                set_pen_mode(s, true);
            } else if !s.g_pen_busy_noted {
                s.g_pen_busy = s.g_pen_busy.wrapping_add(1);
                s.g_pen_busy_noted = true;
            }
        }
        Some(r)
    }) else {
        return;
    };
    // The pen samples this frame draws are the ones emitted until now: taken before the copy, since
    // the copy pumps (copy_rect), and what it reads is drawn by the next frame.
    let (mut newest, mut oldest, mut raw) = (0u64, 0u64, 0u64);
    let timed = with(|s| s.g_perf_enabled) && input_ink_times(&mut newest, &mut oldest, &mut raw);
    let t0 = with(|s| {
        s.g_live = mode == RefreshMode::Fast;
        if s.g_perf_enabled {
            clock_us()
        } else {
            0
        }
    });
    copy_rect(screen, r);
    with(|s| {
        let copy_us = if s.g_perf_enabled { clock_us().wrapping_sub(t0) } else { 0 };
        s.g_live = false;
        if mode == RefreshMode::Fast {
            s.g_live_area = rect_union(s.g_live_area, r);
        } else if mode == RefreshMode::Full || rect_contains_rect(r, s.g_live_area) {
            s.g_live_area = RECT_EMPTY;
        }
        let flashing = mode == RefreshMode::Full;
        let wfm = match mode {
            RefreshMode::Fast => ffi::GRIM_WFM_DU,
            RefreshMode::Ui => ffi::GRIM_WFM_GL16,
            RefreshMode::Full => ffi::GRIM_WFM_GC16,
        };
        let whole_frame = rect_contains_rect(r, Rect { x: 0, y: 0, w: s.LW, h: s.LH });
        let mut r = if mode == RefreshMode::Full { Rect { x: 0, y: 0, w: s.PW, h: s.PH } } else { fb_rect(s, r) };
        let padded = s.g_pen_cfg != PenCfg::Direct;
        if mode == RefreshMode::Fast && s.g_pen_mode && padded && (s.g_st.current_rota & 1) != 0 {
            r = fbpack_pen_region(r, s.PW, s.PH);
        }
        let ret = send_refresh(s, r, mode, wfm, flashing);
        let marker = fbink_get_last_marker();
        if ret >= 0 {
            if s.g_pen_mode {
                s.g_pen_marker = marker;
                s.g_pen_last_ms = now_ms();
            } else {
                quiet_sent(marker, mode != RefreshMode::Fast);
            }
        }
        let ink = live_ink_frame(&mut s.g_ink, mode, now_ms());
        // (A present with no ink took the pen samples emitted before it all the same, untimed: a stroke's
        // last ones that never got a fast frame would otherwise wait to be the next stroke's "oldest".)
        if mode == RefreshMode::Fast && s.g_perf_enabled {
            let sent_us = clock_us();
            if ink && ret >= 0 {
                let kind = if !s.g_pen_mode {
                    INK_DU
                } else if padded {
                    INK_PEN_PADDED
                } else {
                    INK_PEN_DIRECT
                };
                if timed {
                    lat_hist_add(&mut s.g_lat_pen[kind], sent_us.wrapping_sub(newest) as i64);
                    lat_hist_add(&mut s.g_lat_oldest[kind], sent_us.wrapping_sub(oldest) as i64);
                    if raw != 0 {
                        lat_hist_add(&mut s.g_lat_raw[kind], sent_us.wrapping_sub(raw) as i64);
                        lat_hist_add(&mut s.g_lat_hop[kind], newest.wrapping_sub(raw) as i64);
                    } else {
                        s.g_raw_miss[kind] = s.g_raw_miss[kind].wrapping_add(1);
                    }
                }
            }
            s.g_copy_us = s.g_copy_us.wrapping_add(copy_us);
            s.g_submit_us = s.g_submit_us.wrapping_add(s.g_last_submit_us);
            if copy_us > s.g_copy_max_us {
                s.g_copy_max_us = copy_us;
            }
            if s.g_last_submit_us > s.g_submit_max_us {
                s.g_submit_max_us = s.g_last_submit_us;
            }
            s.g_perf_frames = s.g_perf_frames.wrapping_add(1);
            if now_ms().wrapping_sub(s.g_perf_at) >= 10000 {
                qlog!(
                    "display: fast timing (%u frames, pen %d): copy avg/max %llu/%llu us; submit avg/max %llu/%llu us",
                    s.g_perf_frames,
                    s.g_pen_mode,
                    s.g_copy_us / s.g_perf_frames as u64,
                    s.g_copy_max_us,
                    s.g_submit_us / s.g_perf_frames as u64,
                    s.g_submit_max_us
                );
                lat_log(s);
                s.g_perf_at = now_ms();
                s.g_perf_frames = 0;
                s.g_copy_us = 0;
                s.g_submit_us = 0;
                s.g_copy_max_us = 0;
                s.g_submit_max_us = 0;
            }
        }
        if ret < 0 {
            s.g_frame_known = false;
        } else if mode != RefreshMode::Fast && whole_frame && !s.g_previous.px.is_empty() {
            s.g_frame_known = true;
        }
        if mode == RefreshMode::Fast { s.last_fast_wait = ret >= 0 && s.g_mdp_wait && !ink; }
        if ret >= 0 && mode == RefreshMode::Fast && s.g_mdp_wait && !ink {
            mdp_wait(s);
        }
        if ret >= 0 && s.g_presents < 40 {
            qlog!(
                "display: refresh #%d %d,%d %dx%d mode %d pen %d -> ok, marker %u, fb[center]=%u",
                s.g_presents,
                r.x,
                r.y,
                r.w,
                r.h,
                mode as i32,
                s.g_pen_mode,
                fbink_get_last_marker(),
                if s.g_readback { fb_gray(s, s.LW / 2, s.LH / 2) as u32 } else { 0u32 }
            );
        }
        if s.g_presents == 0 && s.g_readback {
            if s.g_st.bpp == 32 {
                // The raw bytes too: they show the channel order the driver really uses.
                let (pi, qi) = (fb_index(s, 100, 191), fb_index(s, 100, 600));
                let (p, q) = (&s.g_fb[pi..pi + 4], &s.g_fb[qi..qi + 4]);
                qlog!(
                    concat!(
                        "display: readback after first frame: fb(100,191)=%u (expect ~0, the top rule) fb(100,600)=%u, bytes ",
                        "%02x %02x %02x %02x and %02x %02x %02x %02x (Grimoire writes v v v ff)"
                    ),
                    fb_gray(s, 100, 191),
                    fb_gray(s, 100, 600),
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    q[0],
                    q[1],
                    q[2],
                    q[3]
                );
            } else {
                qlog!(
                    "display: readback after first frame: fb(100,191)=%u (expect ~0, the top rule) fb(100,600)=%u",
                    fb_gray(s, 100, 191),
                    fb_gray(s, 100, 600)
                );
            }
        }
        s.g_presents += 1;
    })
}

pub fn display_broken() -> bool {
    with(|s| {
        if s.g_fd < 0 || s.g_ok > 0 || s.g_presents == 0 {
            return false;
        }
        if fbcheck_broken(s.g_presents, s.g_ok) {
            return true;
        }
        // Nothing has shown yet: refresh the whole panel again once a second (the framebuffer already
        // holds the frame), so a panel that takes nothing is known within seconds.
        if now_ms().wrapping_sub(s.g_last_refresh) < 1000 {
            return false;
        }
        qlog!("display: nothing has shown yet; refreshing the whole panel again (refresh %d of 5)", s.g_presents + 1);
        let whole = Rect { x: 0, y: 0, w: s.PW, h: s.PH };
        send_refresh(s, whole, RefreshMode::Full, ffi::GRIM_WFM_GC16, true);
        s.g_presents += 1;
        fbcheck_broken(s.g_presents, s.g_ok)
    })
}

pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    *where_ = RECT_EMPTY;
    with(|s| {
        if s.g_fd < 0 || !s.g_readback {
            return false;
        }
        let w = if screen.w < s.LW { screen.w } else { s.LW };
        let h = if screen.h < s.LH { screen.h } else { s.LH };
        const STEP: i32 = 24;
        let (mut differ, mut x0, mut y0, mut x1, mut y1) = (0, w, h, 0, 0);
        for y in (STEP / 2..h).step_by(STEP as usize) {
            for x in (STEP / 2..w).step_by(STEP as usize) {
                if rect_contains(skip, x, y) || rect_contains(s.g_live_area, x, y) || rect_contains(s.g_overlay, x, y) || rect_contains(s.g_status, x, y) {
                    continue;
                }
                let want = screen.px[y as usize * screen.stride as usize + x as usize];
                let have = fb_gray(s, x, y);
                if (want as i32 - have as i32).abs() <= 24 {
                    continue;
                }
                differ += 1;
                if x < x0 {
                    x0 = x;
                }
                if y < y0 {
                    y0 = y;
                }
                if x > x1 {
                    x1 = x;
                }
                if y > y1 {
                    y1 = y;
                }
            }
        }
        // A few stray samples are ink still settling; another window's strip is dozens.
        if differ < 8 {
            return false;
        }
        *where_ = rect_intersect(
            Rect { x: x0 - STEP, y: y0 - STEP, w: x1 - x0 + 2 * STEP, h: y1 - y0 + 2 * STEP },
            Rect { x: 0, y: 0, w, h },
        );
        qlog!("display: %d samples differ in %d,%d %dx%d (something drew there)", differ, where_.x, where_.y, where_.w, where_.h);
        true
    })
}

pub fn display_band_overwritten(screen: &Canvas, band: Rect, skip: Rect) -> bool {
    with(|s| {
        if s.g_fd < 0 || !s.g_readback {
            return false;
        }
        let band = rect_intersect(
            band,
            Rect {
                x: 0,
                y: 0,
                w: if screen.w < s.LW { screen.w } else { s.LW },
                h: if screen.h < s.LH { screen.h } else { s.LH },
            },
        );
        let (mut differ, mut total) = (0, 0);
        for y in (band.y + 2..band.y + band.h).step_by(6) {
            for x in (band.x + 2..band.x + band.w).step_by(6) {
                if rect_contains(skip, x, y) || rect_contains(s.g_live_area, x, y) || rect_contains(s.g_overlay, x, y) || rect_contains(s.g_status, x, y) {
                    continue;
                }
                let want = screen.px[y as usize * screen.stride as usize + x as usize];
                let have = fb_gray(s, x, y);
                total += 1;
                if (want as i32 - have as i32).abs() > 24 {
                    differ += 1;
                }
            }
        }
        if total != 0 && differ > total / 100 + 2 {
            qlog!(
                "display: %d of %d samples in band %d,%d %dx%d differ (something drew there)",
                differ,
                total,
                band.x,
                band.y,
                band.w,
                band.h
            );
            return true;
        }
        false
    })
}

// Until the last update has ended (before sleep and exit). One the quiet thread took on is waited
// for in its books, not at the driver: a second waiter on its marker would sit out the driver's 5 s.
pub fn display_wait() {
    let fd = with(|s| s.g_fd);
    if fd < 0 {
        return;
    }
    let mut q = qlock();
    let theirs = q.fd >= 0 && fbink_get_last_marker() == q.handed;
    if theirs {
        let until = Instant::now() + Duration::from_secs(6); // past the driver's own timeout
        while q.gray_n != 0 || q.du_done != q.du_sent {
            let now = Instant::now();
            if now >= until {
                break;
            }
            let (g, t) = Q_IDLE.wait_timeout(q, until - now).unwrap_or_else(|e| e.into_inner());
            q = g;
            if t.timed_out() {
                break;
            }
        }
    }
    drop(q);
    if !theirs {
        fbink_wait_for_complete(fd, ffi::LAST_MARKER);
    }
}

pub fn display_close() {
    with(|s| {
        if s.g_fd < 0 {
            return;
        }
        pen_leave(s);
        if s.g_st.is_mtk {
            fbink_mtk_toggle_auto_reagl(s.g_fd, true);
        }
        // (FBInk's close unmaps the framebuffer: the slice goes first)
        s.g_fb = &mut [];
        fbink_close(s.g_fd);
        s.g_fd = -1;
        canvas_free(&mut s.g_previous);
        s.g_frame_known = false;
    })
}
/// Last Fast frame needed the existing colour-panel driver wait; reads cached state only.
pub fn display_last_fast_waiting() -> bool { with(|s| s.last_fast_wait) }
