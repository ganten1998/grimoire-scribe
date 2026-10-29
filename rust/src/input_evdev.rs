//! input_evdev.c: Kindle input from evdev.
//!
//! Devices are classified by capability, not by path:
//!   pen   = EV_ABS ABS_X + ABS_PRESSURE and EV_KEY BTN_TOOL_PEN. The Scribe 2 has two:
//!           "stylus-custom" (event6 since firmware 5.20.1, event5 before), a virtual pen that a
//!           pen service on the Kindle feeds from the raw digitizer, scaled to the screen and
//!           aligned with touch, but it only reports position + buttons (BTN_TOOL_PEN, BTN_TOUCH,
//!           ABS_X, ABS_Y: 2 or 3 events a frame). Which process makes it is not known; not
//!           input_manager, whose device is only "kindle-input-pt". And "WacomDigitizer"
//!           (event3), the raw EMR digitizer (0..15624 x 0..20832) with pressure 0..4095 and tilt
//!           in 1/100 degree (seen on device 2026-10-10). Grimoire takes position and buttons
//!           from the scaled pen (grabbed) and pressure + tilt from the raw one (read without a
//!           grab, so the pen service keeps feeding the scaled pen). The scaled pen says when the
//!           pen comes and goes but not where it hovers: Live view's hovering pen is the raw
//!           one's place (input_hover_hook).
//!   touch = EV_ABS ABS_MT_POSITION_X (the capacitive panel, usually /dev/input/touch)
//!   power = EV_KEY KEY_POWER on a device with few keys, never Amazon's every-key proxy
//!           "kindle-input-pt" (firmware 5.20+); the Scribe 2's is "bd71828-pwrkey" (hidkeys_power)
//! Pen and touch are EVIOCGRAB'ed so the (paused) framework never sees them; the power key is
//! only watched, so the system keeps handling it if we die. Its press and release are both read:
//! holding it INPUT_POWER_HOLD_MS (4 s) is Grimoire's guaranteed way out.
//!
//! Pen events follow the Wacom tool protocol (BTN_TOOL_PEN / BTN_TOOL_RUBBER bracket the
//! proximity, BTN_TOUCH marks contact, pressure/tilt arrive as EV_ABS, SYN_REPORT ends a frame),
//! the same model KOReader uses on the Scribe (input.wacom_protocol).
//!
//! The kernel keeps 128 events of the scaled pen for Grimoire: 120-175 ms of writing, and of the raw
//! digitizer about 100 ms, hover too. Nothing on the main thread may hold it longer while the pen is
//! near, or the kernel drops pen events and the stroke loses that stretch, or its pressure and tilt
//! (docs/INK-PERFORMANCE.md, Reader stalls). Long CPU work pumps (input_pump, through common.h's
//! work_pump), which reads the pens as it goes. A wait cannot pump: that is why key_scan never opens
//! a node Grimoire holds (each close waits out an RCU grace period) and never runs while the pen or
//! a finger is in use.

use crate::prelude::*;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::mem::{size_of, size_of_val};
use std::os::unix::ffi::OsStrExt;
use std::sync::atomic::{AtomicI32, Ordering};

use libc::{c_int, c_ulong, c_void};

// ---- linux/input.h ----
//
// The libc crate has none of evdev's structs, numbers or ioctls; these are the kernel's, as
// linux/input.h and linux/input-event-codes.h give them to the C build.

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const EV_MSC: u16 = 0x04;
const EV_MAX: usize = 0x1f;
const SYN_REPORT: u16 = 0;
const SYN_DROPPED: u16 = 3;
const KEY_POWER: u16 = 116;
const BTN_TOOL_PEN: u16 = 0x140;
const BTN_TOOL_RUBBER: u16 = 0x141;
const BTN_TOUCH: u16 = 0x14a;
const BTN_STYLUS: u16 = 0x14b;
const KEY_MAX: usize = 0x2ff;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const ABS_PRESSURE: u16 = 0x18;
const ABS_DISTANCE: u16 = 0x19;
const ABS_TILT_X: u16 = 0x1a;
const ABS_TILT_Y: u16 = 0x1b;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TOOL_TYPE: u16 = 0x37;
const ABS_MT_TRACKING_ID: u16 = 0x39;
const ABS_MT_PRESSURE: u16 = 0x3a;
const ABS_MAX: usize = 0x3f;

/// struct input_event as the kernel writes it to this reader: on the Kindle's 32-bit ARM two
/// unsigned longs of time (input_event_sec, input_event_usec: the time64 C build's layout, and the
/// kernel's own), type, code and value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct InputEvent {
    input_event_sec: c_ulong,
    input_event_usec: c_ulong,
    type_: u16,
    code: u16,
    value: i32,
}

/// struct input_absinfo (EVIOCGABS)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct InputAbsinfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

// linux/ioctl.h's encoding (asm-generic, which ARM uses): the number, the type ('E' for evdev), the
// argument's size and the direction.
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;
const fn ioc(dir: u32, nr: u32, size: usize) -> u32 {
    (dir << 30) | ((size as u32) << 16) | ((b'E' as u32) << 8) | nr
}
/// EVIOCGABS(abs)
const fn eviocgabs_req(abs: u16) -> u32 {
    ioc(IOC_READ, 0x40 + abs as u32, size_of::<InputAbsinfo>())
}
const EVIOCGRAB: u32 = ioc(IOC_WRITE, 0x90, size_of::<c_int>());
const EVIOCSCLOCKID: u32 = ioc(IOC_WRITE, 0xa0, size_of::<c_int>());

// The kernel's own numbers for them, and its structs' sizes (16 bytes an event on the Kindle).
const _: () = assert!(EVIOCGRAB == 0x4004_4590 && EVIOCSCLOCKID == 0x4004_45a0, "evdev ioctls");
const _: () = assert!(eviocgabs_req(ABS_X) == 0x8018_4540, "EVIOCGABS");
const _: () = assert!(size_of::<InputAbsinfo>() == 24, "struct input_absinfo");
const _: () = assert!(size_of::<InputEvent>() == 2 * size_of::<c_ulong>() + 8, "struct input_event");

// pensync.h's copy of the evdev numbers it needs.
const _: () = assert!(PS_EV_SYN == EV_SYN && PS_EV_KEY == EV_KEY && PS_EV_ABS == EV_ABS, "pensync event types");
const _: () = assert!(PS_SYN_REPORT == SYN_REPORT && PS_SYN_DROPPED == SYN_DROPPED, "pensync SYN codes");
const _: () = assert!(
    PS_ABS_X == ABS_X
        && PS_ABS_Y == ABS_Y
        && PS_ABS_PRESSURE == ABS_PRESSURE
        && PS_ABS_TILT_X == ABS_TILT_X
        && PS_ABS_TILT_Y == ABS_TILT_Y,
    "pensync axes"
);
const _: () = assert!(
    PS_BTN_TOOL_PEN == BTN_TOOL_PEN
        && PS_BTN_TOOL_RUBBER == BTN_TOOL_RUBBER
        && PS_BTN_TOUCH == BTN_TOUCH
        && PS_BTN_STYLUS == BTN_STYLUS,
    "pensync buttons"
);

// The C's bitmaps are arrays of unsigned long (NLONGS(bits) of them, which sizes each EVIOCGBIT and
// EVIOCGKEY request); here they are the same bytes, read a bit at a time, which on the Kindle's
// little-endian ARM is the same bit as test_bit's long-wise read.
const BITS_PER_LONG: usize = c_ulong::BITS as usize;
const fn nlongs_bytes(bits: usize) -> usize {
    bits.div_ceil(BITS_PER_LONG) * size_of::<c_ulong>()
}
const EVB_BYTES: usize = nlongs_bytes(EV_MAX + 1);
const KEYB_BYTES: usize = nlongs_bytes(KEY_MAX + 1);
const ABSB_BYTES: usize = nlongs_bytes(ABS_MAX + 1);
const _: () = assert!(cfg!(target_endian = "little"), "evdev bitmaps read as bytes");
fn test_bit(a: &[u8], bit: usize) -> bool {
    (a[bit / 8] >> (bit % 8)) & 1 != 0
}
// Linux's native bitmap byte layout matches the decoder's on our little-endian armhf target.
// (KEY_MAX + 1 <= HID_KEY_BYTES * 8, and the same of the axes)
const _: () = assert!(KEY_MAX < HID_KEY_BYTES * 8, "external key bitmap too small");
const _: () = assert!(ABS_MAX < HID_ABS_BYTES * 8, "external axis bitmap too small");
// (and the bitmaps are hidkeys' arrays, [u8; HID_KEY_BYTES] and [u8; HID_ABS_BYTES], byte for byte)
const _: () = assert!(KEYB_BYTES == HID_KEY_BYTES && ABSB_BYTES == HID_ABS_BYTES, "bitmap sizes");

// ---- the ioctls, each wrapped once ----

/// EVIOCGABS(code) into ai: ioctl's result.
fn eviocgabs(fd: i32, code: u16, ai: &mut InputAbsinfo) -> i32 {
    // SAFETY: EVIOCGABS writes one struct input_absinfo (its size is in the request) into ai, a
    // repr(C) struct of six ints that any bytes make valid; a bad fd only makes it fail.
    unsafe { libc::ioctl(fd, eviocgabs_req(code) as libc::Ioctl, ai as *mut InputAbsinfo,
        ) }
}
/// A read request whose length is the buffer's (EVIOCGNAME, EVIOCGKEY, EVIOCGBIT).
fn evioc_read(fd: i32, nr: u32, buf: &mut [u8]) -> i32 {
    // SAFETY: the request carries buf.len(), and the kernel writes at most that many bytes into the
    // buffer; bytes are bytes.
    unsafe { libc::ioctl(fd, ioc(IOC_READ, nr, buf.len()) as libc::Ioctl, buf.as_mut_ptr(),
        ) }
}
/// EVIOCGNAME(buf.len())
fn eviocgname(fd: i32, buf: &mut [u8]) -> i32 {
    evioc_read(fd, 0x06, buf)
}
/// EVIOCGKEY(buf.len())
fn eviocgkey(fd: i32, buf: &mut [u8]) -> i32 {
    evioc_read(fd, 0x18, buf)
}
/// EVIOCGBIT(ev, buf.len())
fn eviocgbit(fd: i32, ev: u16, buf: &mut [u8]) -> i32 {
    evioc_read(fd, 0x20 + ev as u32, buf)
}
/// EVIOCGMTSLOTS(sizeof slots): slots[0] names the axis, the rest get each slot's value.
fn eviocgmtslots(fd: i32, slots: &mut [i32]) -> i32 {
    // SAFETY: the request carries the slice's size in bytes, and the kernel reads slots[0] and writes
    // at most that many bytes of ints into it.
    unsafe { libc::ioctl(fd, ioc(IOC_READ, 0x0a, size_of_val(slots)) as libc::Ioctl, slots.as_mut_ptr(),
        ) }
}
/// EVIOCGRAB with its argument as the value (1 grab, 0 let go), as the kernel takes it.
fn ioctl_grab(fd: i32, grab: c_int) -> i32 {
    // SAFETY: EVIOCGRAB reads no memory: the kernel takes the argument itself as the flag.
    unsafe { libc::ioctl(fd, EVIOCGRAB as libc::Ioctl, grab) }
}

// ---- libc, each call wrapped once ----

fn read_events(fd: i32, evs: &mut [InputEvent]) -> isize {
    // SAFETY: read writes at most size_of_val(evs) bytes into the slice: repr(C) structs of
    // integers, which any bytes the kernel writes make valid.
    unsafe { libc::read(fd, evs.as_mut_ptr() as *mut c_void, size_of_val(evs)) }
}
fn close_fd(fd: i32) {
    // SAFETY: closing a descriptor this module opened and owns (the caller forgets it after).
    unsafe { libc::close(fd) };
}
fn poll_fds(pf: &mut [libc::pollfd], timeout_ms: i32) -> i32 {
    // SAFETY: poll reads and writes the pf.len() pollfd structs of the slice.
    unsafe { libc::poll(pf.as_mut_ptr(), pf.len() as libc::nfds_t, timeout_ms) }
}
fn pollfd(fd: i32) -> libc::pollfd {
    libc::pollfd { fd, events: libc::POLLIN, revents: 0 ,
    }
}
fn access_ok(path: &CStr) -> bool {
    // SAFETY: access reads the NUL-terminated path.
    unsafe { libc::access(path.as_ptr(), libc::F_OK) == 0 }
}
/// open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC)
fn open_node(path: &str) -> i32 {
    let Ok(c) = CString::new(path) else { return -1 };
    // SAFETY: open reads the NUL-terminated path.
    unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
        ) }
}
fn stat_path(path: &str) -> Option<libc::stat> {
    let c = CString::new(path).ok()?;
    // SAFETY: stat reads the NUL-terminated path and writes one struct stat into our local, plain
    // integers for which zeroes are a valid value.
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        (libc::stat(c.as_ptr(), &mut st) == 0).then_some(st)
    }
}
fn fstat_fd(fd: i32) -> Option<libc::stat> {
    // SAFETY: fstat writes one struct stat into our local, plain integers for which zeroes are a
    // valid value; a bad fd only makes it fail.
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        (libc::fstat(fd, &mut st) == 0).then_some(st)
    }
}
/// A C string's bytes: up to its first NUL.
fn cbytes(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}
/// Bytes from the kernel or a child as text for hidkeys and the log. (Device names are bytes; the
/// C logs them as they are, a String cannot hold an invalid sequence: each becomes U+FFFD.)
fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}
/// x++ on an int counter, wrapping as the C build's does.
fn post_inc(v: &mut i32) -> i32 {
    let o = *v;
    *v = o.wrapping_add(1);
    o
}
/// atoi(3), as musl's: leading white space, an optional sign, then digits.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        neg = s[i] == b'-';
        i += 1;
    }
    // (computed as a negative number, as musl's is, so INT_MIN comes out whole)
    let mut n: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        n = n.wrapping_mul(10).wrapping_sub((s[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        n
    } else {
        n.wrapping_neg()
    }
}

#[derive(Clone, Copy, Debug)]
struct Dev {
    fd: i32,
    ax: InputAbsinfo,
    ay: InputAbsinfo,
    ap: InputAbsinfo,
    atx: InputAbsinfo,
    aty: InputAbsinfo,
    /// amp: ABS_MT_PRESSURE, if any
    amp: InputAbsinfo,
    has_touch_btn: bool,
}

impl Default for Dev {
    fn default() -> Self {
        Dev {
            fd: -1,
            ax: InputAbsinfo::default(),
            ay: InputAbsinfo::default(),
            ap: InputAbsinfo::default(),
            atx: InputAbsinfo::default(),
            aty: InputAbsinfo::default(),
            amp: InputAbsinfo::default(),
            has_touch_btn: false,
        }
    }
}

/// The raw digitizer's newest values (W).
#[derive(Clone, Copy, Debug, Default)]
struct WacomSt {
    p: i32,
    tx: i32,
    ty: i32,
    /// position: for timing the pen service's relay (latency.h), and Live view's hover
    x: i32,
    y: i32,
    /// (placed: x and y read since input was last taken back)
    seen: bool,
    placed: bool,
    /// the raw digitizer also reports the eraser end and the side button
    rubber: bool,
    side: bool,
}

// ---- event queue ----

const QN: usize = 4096;

// ---- pen state ----

/// P
#[derive(Clone, Copy, Debug, Default)]
struct PenSt {
    /// tracked separately: a flip sends PEN 0 / RUBBER 1 in either order
    tool_pen: bool,
    tool_rubber: bool,
    prox: bool,
    eraser: bool,
    touching: bool,
    side: bool,
    was_prox: bool,
    was_touching: bool,
    x: i32,
    y: i32,
    p: i32,
    tx: i32,
    ty: i32,
    dirty: bool,
}

// ---- touch state ----

const MAX_SLOTS: usize = 16;
const _: () = assert!(MAX_SLOTS == TG_MAX); // (mt_column's columns are touchgest's)

/// T: the touch panel's slots as its events leave them. What the contacts mean (one finger's
/// stream, a pinch, a gesture, a palm) is touchgest.rs's, fed one frame per SYN_REPORT.
#[derive(Clone, Copy, Debug)]
struct TouchSt {
    slot: i32,
    id: [i32; MAX_SLOTS],
    x: [i32; MAX_SLOTS],
    y: [i32; MAX_SLOTS],
    /// ABS_MT_TOOL_TYPE: 0 finger, 1 pen, 2 palm (MT_TOOL_PALM)
    tool: [i32; MAX_SLOTS],
    /// ABS_MT_TOUCH_MAJOR and ABS_MT_PRESSURE, 0 where the panel says nothing
    major: [i32; MAX_SLOTS],
    pressure: [i32; MAX_SLOTS],
    /// the kernel's time of the SYN_REPORT being handled
    frame_us: u64,
    /// a SYN_DROPPED came: the rest of its frame is skipped, and its SYN_REPORT asks the panel what it
    /// holds (touch_resync)
    dropped: bool,
}

impl Default for TouchSt {
    fn default() -> Self {
        TouchSt {
            slot: 0,
            id: [-1; MAX_SLOTS],
            x: [0; MAX_SLOTS],
            y: [0; MAX_SLOTS],
            tool: [0; MAX_SLOTS],
            major: [0; MAX_SLOTS],
            pressure: [0; MAX_SLOTS],
            frame_us: 0,
            dropped: false,
        }
    }
}

// ---- the touch log and trace ----

mod touch_log_budget;
use touch_log_budget::{LogDecision, TouchLogBudget};

/// grimoire.log's touch lines: the first TLOG_FRAMES multi-contact frames of each touch session, at
/// most 2000 in a 60-second window; a single finger's TOUCH_MAJOR kept for its first TIP_N frames.
const TLOG_FRAMES: i32 = 300;
const TIP_N: usize = 128;

#[derive(Clone, Copy, Debug)]
struct TouchLog {
    /// multi-contact frames logged this touch session, renewable log window
    frames: i32,
    budget: TouchLogBudget,
    /// a single finger's TOUCH_MAJOR at each frame it reported one (the first TIP_N), how many
    /// frames, the largest
    tip: [i32; TIP_N],
    tip_frames: i32,
    tip_max: i32,
}

impl Default for TouchLog {
    fn default() -> Self {
        TouchLog { frames: 0, budget: TouchLogBudget::default(), tip: [0; TIP_N], tip_frames: 0, tip_max: 0 ,
        }
    }
}

/// The trace file is written TRACE_FLUSH bytes at a time (or at each touch session's end), up to
/// TRACE_MAX bytes in all; its buffer has TRACE_ROOM, so the pen's lines never write.
const TRACE_FLUSH: usize = 4096;
const TRACE_ROOM: usize = 8192;
const TRACE_MAX: u64 = 4 * 1024 * 1024;

/// The opt-in touch trace (input_touch_trace): grimoire/touch-trace.txt while grimoire/touch-trace is
/// there.
#[derive(Debug)]
struct TouchTrace {
    /// O_APPEND, -1 none
    fd: i32,
    /// what waits to be written (TRACE_ROOM reserved: appending never allocates)
    buf: String,
    written: u64,
}

impl Default for TouchTrace {
    fn default() -> Self {
        TouchTrace { fd: -1, buf: String::new(), written: 0 ,
        }
    }
}

// Optional external inputs are watched without exclusive grabs. Native PIN input is discarded
// at the same boundary as pen/touch, and key values and device names are never written to the log.
const KEY_DEVS: usize = 4;
#[derive(Clone, Copy, Debug, Default)]
struct KeyDev {
    fd: i32,
    path: FixedStr<300>,
    /// name: hidkeys_name's, for IN_HID_* and input_hids
    name: FixedStr<41>,
    kind: HidKind,
    /// another app held it exclusively when it came
    held: bool,
    /// a Switch-mode controller (hidkeys_switch)
    switch_layout: bool,
    /// its triggers are decided at its first report (hidkeys_triggers)
    trig_wait: bool,
    hatmask: u32,
    keys: HidKeys,
    /// its analog triggers, read as LT and RT (none: zeroed)
    trig: HidTriggers,
}

const POWERD_BUF: usize = 512;

/// input_evdev.c's file-level statics (PORTING.md, Global state). The flag and the pipe the signal
/// handler touches are the atomics below instead.
#[allow(non_snake_case)]
struct State {
    /// raw pen events written to the log (bring-up)
    raw_logged: i32,
    /// pen frames in contact so far
    contact_frames: i32,
    /// any nonzero pressure ever reported
    pressure_seen: bool,
    /// contact without pressure for a while: use a fixed value
    pressure_missing: bool,
    /// grimoire/tilt-flip: the digitizer's tilt axes are reversed
    tilt_flip: bool,

    pen: Dev,
    touch: Dev,
    /// accelerometer orientation codes (bma4xy_feature on the Scribe 2)
    orient_fd: i32,
    // Power state from the Kindle's power service (the events KOReader uses for its screensaver).
    // A child shell loops `lipc-wait-event` in one-shot mode: each run prints a single line and
    // exits, which flushes it. (The continuous -m mode looked block-buffered into a pipe on device
    // 2026-10-10, but the launcher had preventScreenSaver 1 set that day, which alone stops every
    // sleep; KOReader reads `-m -s 0` through a pipe. An empty /mnt/us/grimoire/watcher-multi runs it
    // that way, for a device test: see powerd_start.)
    powerd_fd: i32,
    powerd_pid: libc::pid_t,
    powerd_buf: [u8; POWERD_BUF],
    powerd_len: usize,
    /// watcher lines that are not events (failures, errors), for the log's sake
    powerd_noise: i32,
    /// logical screen rotation, quarter turns clockwise
    rot: i32,
    /// raw digitizer used only for pressure + tilt (see top)
    wacom: Dev,
    W: WacomSt,
    // Both pens' events are stamped in CLOCK_MONOTONIC for this reader (clock_mono, as input_open opens
    // each node, before any other ioctl): stroke samples carry the kernel's time of their frame
    // (InEvent.k_ms), and the pen's drops say how old their tail is.
    pen_mono: bool,
    wacom_mono: bool,
    // Ink timing (latency.h): both pens' events stamped in CLOCK_MONOTONIC, the raw samples kept to
    // match the relayed ones, and the samples emitted since display_fbink.c last asked.
    ink_mono: bool,
    match_: LatMatch,
    /// kernel time of the relayed pen frame being read
    pen_frame_us: u64,
    ink_newest: u64,
    ink_oldest: u64,
    ink_raw: u64,
    input_grabbed: bool,
    initial_grab_failed: bool,
    pen_resume_block: bool,
    /// last side-button state sent as IN_PEN_BUTTON
    side_reported: bool,
    power_fd: i32,
    /// the press being followed (release, or the 4 s hold)
    pkey: PowerKey,
    /// the power key's events are stamped in now_ms()'s clock (EVIOCSCLOCKID)
    power_mono: bool,
    watch: i32,
    w: i32,
    h: i32,
    pen_cal: InputCalib,
    touch_cal: InputCalib,
    caps: InputCaps,

    // ---- event queue ----
    q: Vec<InEvent>,
    qh: usize,
    qt: usize,
    logged_pen: i32,
    logged_touch: i32,

    // ---- pen state ----
    P: PenSt,
    pen_left_ms: u32,

    // ---- touch state ----
    T: TouchSt,
    /// the recognizer the touch frames go through (touchgest.rs)
    tg: TouchGest,
    /// the touch panel's events are stamped in CLOCK_MONOTONIC for this reader (clock_mono): each
    /// frame is timed by its SYN_REPORT
    touch_mono: bool,
    /// the pen's last contact point (logical px, as emit_pen maps it): touchgest keeps gestures away
    /// from where the pen just wrote
    pen_last: (f32, f32),
    /// last mapped raw hover point and its kernel time, for touchgest's learned-hover rule
    tg_hover: Option<(f32, f32, u32)>,
    /// touch events handled so far (wrapping): input_poll ticks touchgest when none came
    touch_events: u32,
    tlog: TouchLog,
    trace: TouchTrace,
    /// the screen's (input_open), for the trace's header
    dpi: f32,

    // The raw digitizer's own drops: its frames are bigger than the scaled pen's (pressure and tilt
    // too), so its 128 events overflow sooner. Up to the next SYN_REPORT is partial and skipped (no
    // relay timing for it); once its queue is read, wacom_resync asks for pressure, tilt and buttons.
    wacom_dropping: bool,
    wacom_resync: bool,
    wacom_drops: i32,
    // Its frames kept, so each pen frame takes the pressure and tilt of its own moment (pensync.h), not
    // the newest values: a read after a wait takes some 30 frames of each at once. Only when both nodes'
    // events are in CLOCK_MONOTONIC (pen_mono, wacom_mono), which the resync's time is in too.
    raw: PenRawRing,
    // When input_poll last read it (input_open starts it when it is done), and the time before that;
    // whether it has been read since input_open, and whether this read is the first.
    wacom_read_ms: u32,
    wacom_read_gap: u32,
    wacom_read: bool,
    wacom_first_read: bool,
    // Live view's hovering pen (input_hover_hook): the pen service relays the pen coming and going but
    // not where it hovers, so while a hook is set each raw frame of a pen near and not touching says
    // where it is, in the scaled pen's units (pensync.h), as contact frames have shown the raw place to
    // lie on the scaled one. wacom_frame_us: the newest raw frame's kernel time; hover_said: the
    // orientation grimoire.log last named (-2 none yet).
    hover_fn: Option<fn(f32, f32)>,
    hover: PenHover,
    wacom_frame_us: u64,
    hover_said: i32,
    // input_sample_hook's (Live view's): sample_fn sees each pen event as it is queued (and a raw
    // hover, hover_from_raw), sample_batch runs once after each read of the pens. sample_fn runs under
    // this module's borrow, in q_push: by input.h's rule it only stores into memory (app's and live's),
    // so it never reaches input.
    sample_fn: Option<fn(&mut InEvent)>,
    sample_batch: Option<fn()>,

    /// The frame being reported is the first after a pen drop (pen_resync): its sample says so (gap).
    pen_gap: bool,
    /// The pen at the last complete frame read: where a resync after a drop goes on from.
    pen_good: PenState,

    drops: i32,
    /// when the pen was last read (input_open starts it when it is done)
    pen_read_ms: u32,
    /// the pen has been read since input_open
    pen_read: bool,

    keys: [KeyDev; KEY_DEVS],
    keys_enabled: bool,
    keys_init: bool,
    keys_scan: u32,
    /// the keyboard drain reads (g_key_read): its index in keys
    key_read: usize,
    /// Nodes found to be no keyboard or controller, by input_open or a scan: never opened again while
    /// they stay the same node (hidkeys_node).
    key_none: HidNoneCache,

    /// when the pens were last looked at, by input_poll or input_pump
    pens_read_ms: u32,
    /// input_pump's `static bool in`
    pump_in: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            raw_logged: 0,
            contact_frames: 0,
            pressure_seen: false,
            pressure_missing: false,
            tilt_flip: false,
            pen: Dev::default(),
            touch: Dev::default(),
            orient_fd: -1,
            powerd_fd: -1,
            powerd_pid: -1,
            powerd_buf: [0; POWERD_BUF],
            powerd_len: 0,
            powerd_noise: 0,
            rot: 0,
            wacom: Dev::default(),
            W: WacomSt::default(),
            pen_mono: false,
            wacom_mono: false,
            ink_mono: false,
            match_: LatMatch::default(),
            pen_frame_us: 0,
            ink_newest: 0,
            ink_oldest: 0,
            ink_raw: 0,
            input_grabbed: true,
            initial_grab_failed: false,
            pen_resume_block: false,
            side_reported: false,
            power_fd: -1,
            pkey: PowerKey::default(),
            power_mono: false,
            watch: -1,
            w: 0,
            h: 0,
            pen_cal: InputCalib::default(),
            touch_cal: InputCalib::default(),
            caps: InputCaps::default(),
            q: vec![InEvent::default(); QN],
            qh: 0,
            qt: 0,
            logged_pen: 0,
            logged_touch: 0,
            P: PenSt::default(),
            pen_left_ms: 0,
            T: TouchSt::default(),
            tg: TouchGest::default(),
            touch_mono: false,
            pen_last: (0.0, 0.0),
            tg_hover: None,
            touch_events: 0,
            tlog: TouchLog::default(),
            trace: TouchTrace::default(),
            dpi: 0.0,
            wacom_dropping: false,
            wacom_resync: false,
            wacom_drops: 0,
            raw: PenRawRing::default(),
            wacom_read_ms: 0,
            wacom_read_gap: 0,
            wacom_read: false,
            wacom_first_read: false,
            hover_fn: None,
            hover: PenHover::default(),
            wacom_frame_us: 0,
            hover_said: -2,
            sample_fn: None,
            sample_batch: None,
            pen_gap: false,
            pen_good: PenState::default(),
            drops: 0,
            pen_read_ms: 0,
            pen_read: false,
            keys: [KeyDev::default(); KEY_DEVS],
            keys_enabled: false,
            keys_init: false,
            keys_scan: 0,
            key_read: 0,
            key_none: HidNoneCache::default(),
            pens_read_ms: 0,
            pump_in: false,
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// g_quit and g_sigpipe: the signal handler's (it sets the flag and wakes input_poll's poll).
static G_QUIT: AtomicI32 = AtomicI32::new(0);
static G_SIGPIPE: [AtomicI32; 2] = [AtomicI32::new(-1), AtomicI32::new(-1)];

/// What a raw digitizer frame hands Live view (hover_from_raw), called once this module's state is let
/// go, so the hook can reach anything: the hover hook and the place, or, while the sample hook is set,
/// that hook and the hovering pen's event. The other handlers give none.
type Hover = Option<HoverTo>;
enum HoverTo {
    Place(fn(f32, f32), f32, f32),
    Sample(fn(&mut InEvent), InEvent),
}
/// drain's handler of one event.
type Handler = fn(&mut State, &InputEvent) -> Hover;

fn norm(v: i32, a: &InputAbsinfo) -> f32 {
    // (wrapping: the range is the device's, as the C build's int arithmetic wraps)
    let span = a.maximum.wrapping_sub(a.minimum);
    if span > 0 {
        clampf(v.wrapping_sub(a.minimum) as f32 / span as f32, 0.0, 1.0)
    } else {
        0.0
    }
}

// Tilt in degrees. Linux reports ABS_TILT resolution in units per radian; without one, small
// ranges (Wacom's typical -64..63) are taken as degrees, larger ones scaled to +-90.
fn tilt_deg(v: i32, a: &InputAbsinfo) -> f32 {
    if a.maximum == a.minimum {
        return 0.0;
    }
    if a.resolution > 0 {
        return clampf(v as f32 / a.resolution as f32 * 57.29578, -90.0, 90.0);
    }
    let m = if a.minimum.wrapping_abs() > a.maximum.wrapping_abs() { a.minimum.wrapping_abs() } else { a.maximum.wrapping_abs() };
    if m <= 127 {
        return clampf(v as f32, -90.0, 90.0);
    }
    clampf(v as f32 * 90.0 / m as f32, -90.0, 90.0)
}

fn ev_us(ev: &InputEvent) -> u64 {
    (ev.input_event_sec as u64) * 1_000_000 + ev.input_event_usec as u64
}

const PEN_TAIL_MAX: usize = 512; // the kernel keeps 128 events, and this is read in microseconds

fn mono_us() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 ,
    };
    // SAFETY: clock_gettime writes one timespec into our local.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000 + (ts.tv_nsec as u64) / 1000
}

impl State {
    // ---- event queue ----

    fn q_push(&mut self, mut e: InEvent) {
        // PIN touches belong only to Amazon. Do not even log their coordinates.
        if !self.input_grabbed
            && (power_asleep_drops(&e) || e.type_ == InType::PenHover || e.type_ == InType::PenLeave || e.type_ == InType::PenButton)
        {
            return;
        }
        // Device bring-up: the first events of each kind go to the log.
        let t = e.type_ as i32;
        let pen = t >= InType::PenDown as i32 && t <= InType::PenLeave as i32;
        let touch = t >= InType::TouchDown as i32 && t <= InType::TouchUp as i32;
        if (pen && e.type_ != InType::PenHover && post_inc(&mut self.logged_pen) < 120)
            || (touch && post_inc(&mut self.logged_touch) < 120)
        {
            qlog!(
                "input: ev %d at %.0f,%.0f (raw %.3f,%.3f) p %.2f tilt %.0f,%.0f eraser %d side %d",
                t,
                e.x,
                e.y,
                e.nx,
                e.ny,
                e.pressure,
                e.tilt_x,
                e.tilt_y,
                e.eraser,
                e.side_button
            );
        }
        let n = (self.qt + 1) % QN;
        if n == self.qh {
            return; // full: drop (the app drains every loop, so this is pathological)
        }
        if let Some(f) = self.sample_fn {
            if pen {
                f(&mut e);
            }
        }
        self.q[self.qt] = e;
        self.qt = n;
    }
    fn q_pop(&mut self, e: &mut InEvent) -> bool {
        if self.qh == self.qt {
            return false;
        }
        *e = self.q[self.qh];
        self.qh = (self.qh + 1) % QN;
        true
    }

    // Panel <-> logical screen for a device turned `rot` quarter turns clockwise. w x h is the
    // panel; the logical screen is h x w when turned sideways.
    #[allow(non_snake_case)]
    fn panel_to_logical(&self, px: f32, py: f32) -> (f32, f32) {
        let W = (self.w - 1) as f32;
        let H = (self.h - 1) as f32;
        match self.rot & 3 {
            1 => (H - py, px),
            2 => (W - px, H - py),
            3 => (py, W - px),
            _ => (px, py),
        }
    }

    fn map(&self, c: &InputCalib, nx: f32, ny: f32) -> (f32, f32) {
        let mut px = 0.0;
        let mut py = 0.0;
        calib_apply(c, nx, ny, &mut px, &mut py);
        px = clampf(px, 0.0, (self.w - 1) as f32);
        py = clampf(py, 0.0, (self.h - 1) as f32);
        self.panel_to_logical(px, py)
    }

    fn handle_wacom(&mut self, ev: &InputEvent) -> Hover {
        if ev.type_ == EV_SYN && ev.code == SYN_DROPPED {
            self.wacom_dropping = true;
            pensync_raw_clear(&mut self.raw); // what was kept no longer meets what comes after the gap
            if self.wacom_first_read {
                // While Grimoire was starting, as for the pen (pen_resync): no stall, not counted.
                qlog!(
                    concat!(
                        "input: pressure/tilt events dropped by the kernel before Grimoire first read them (not counted; ",
                        "%u ms since input_open)"
                    ),
                    self.wacom_read_gap
                );
                return None;
            }
            self.wacom_drops += 1;
            qlog!(
                "input: pressure/tilt events dropped by the kernel (%d so far; %u ms since it was read before)",
                self.wacom_drops,
                self.wacom_read_gap
            );
            return None;
        }
        if ev.type_ == EV_SYN && ev.code == SYN_REPORT {
            if self.wacom_dropping {
                self.wacom_dropping = false;
                self.wacom_resync = true;
                return None;
            }
            if self.wacom_mono {
                pensync_raw_push(&mut self.raw, ev_us(ev), self.W.p, self.W.tx, self.W.ty);
            }
            if self.ink_mono {
                lat_match_raw(&mut self.match_, ev_us(ev), self.W.x, self.W.y);
            }
            self.wacom_frame_us = ev_us(ev);
            // Learn a usable pen hover for touchgest whenever the input is grabbed. The Live hooks
            // still receive it only when installed.
            let k = pensync_ms(self.wacom_frame_us, self.wacom_mono, now_ms());
            let place = self.hover_from_raw();
            if let Some((x, y)) = place {
                self.tg_hover = Some((x, y, k));
            }
            if self.hover_fn.is_some() || self.sample_fn.is_some() {
                return place.and_then(|(x, y)| match (self.sample_fn, self.hover_fn) {
                    (Some(sample), _) => {
                        let mut e = InEvent { type_: InType::PenHover, x, y, t_ms: now_ms(), eraser: self.W.rubber, side_button: self.W.side, ..Default::default() };
                        e.k_ms = pensync_ms(self.wacom_frame_us, self.wacom_mono, e.t_ms);
                        e.k_us = if self.wacom_mono { self.wacom_frame_us as u32 } else { 0 };
                        Some(HoverTo::Sample(sample, e))
                    }
                    (None, Some(f)) => Some(HoverTo::Place(f, x, y)),
                    (None, None) => None,
                });
            }
            return None;
        }
        if self.wacom_dropping {
            return None;
        }
        if ev.type_ == EV_KEY {
            if ev.code == BTN_TOOL_RUBBER {
                self.W.rubber = ev.value != 0;
            } else if ev.code == BTN_STYLUS {
                self.W.side = ev.value != 0;
            }
            return None;
        }
        if ev.type_ != EV_ABS {
            return None;
        }
        match ev.code {
            ABS_PRESSURE => {
                self.W.p = ev.value;
                self.W.seen = true;
            }
            ABS_X => {
                self.W.x = ev.value;
                self.W.placed = true;
            }
            ABS_Y => {
                self.W.y = ev.value;
                self.W.placed = true;
            }
            ABS_TILT_X => self.W.tx = ev.value,
            ABS_TILT_Y => self.W.ty = ev.value,
            _ => {}
        }
        None
    }

    // After a drop, once its queue is read: what the raw digitizer holds now. (EVIOCGKEY takes queued
    // button events out of the queue, so not before; W only ever keeps the newest values anyway.)
    fn wacom_resync(&mut self) {
        self.wacom_resync = false;
        let fd = self.wacom.fd;
        let mut ai = InputAbsinfo::default();
        if eviocgabs(fd, ABS_PRESSURE, &mut ai) >= 0 {
            self.W.p = ai.value;
            self.W.seen = true;
        }
        if eviocgabs(fd, ABS_X, &mut ai) >= 0 {
            self.W.x = ai.value;
            self.W.placed = true;
        }
        if eviocgabs(fd, ABS_Y, &mut ai) >= 0 {
            self.W.y = ai.value;
            self.W.placed = true;
        }
        if eviocgabs(fd, ABS_TILT_X, &mut ai) >= 0 {
            self.W.tx = ai.value;
        }
        if eviocgabs(fd, ABS_TILT_Y, &mut ai) >= 0 {
            self.W.ty = ai.value;
        }
        let mut keys = [0u8; KEYB_BYTES];
        if eviocgkey(fd, &mut keys) >= 0 {
            self.W.rubber = test_bit(&keys, BTN_TOOL_RUBBER as usize);
            self.W.side = test_bit(&keys, BTN_STYLUS as usize);
        }
        // Pen frames from the lost stretch are older than this, and take these newest values.
        if self.wacom_mono {
            pensync_raw_push(&mut self.raw, mono_us(), self.W.p, self.W.tx, self.W.ty);
        }
        self.wacom_frame_us = if self.wacom_mono { mono_us() } else { 0 };
    }

    // Where the pen hovers, from the raw digitizer's newest frame (pensync.h): the scaled pen's place,
    // normalized as emit_pen's. None while either pen says it touches, before the raw digitizer has said
    // where it is, or while its place is not trusted (pensync_hover_orientation -1).
    fn raw_hover(&self) -> Option<(f32, f32)> {
        let mut sx = 0.0f32;
        let mut sy = 0.0f32;
        if !self.W.placed
            || self.W.p > self.wacom.ap.minimum
            || self.P.touching
            || self.P.was_touching
            || !pensync_hover_place(&self.hover, self.W.x, self.W.y, &mut sx, &mut sy)
        {
            return None;
        }
        Some((norm((sx + 0.5) as i32, &self.pen.ax), norm((sy + 0.5) as i32, &self.pen.ay),
        ))
    }

    // A raw frame read while Live view follows the hover: the pen near (the scaled pen's proximity) and
    // not touching goes to the hook, in screen px as emit_pen maps them. Never while the pen and touch
    // are the Kindle's (the native PIN screen, or the Kindle's own sleep screen). (The place is handed
    // back for drain to give the hook: see Hover.)
    fn hover_from_raw(&self) -> Option<(f32, f32)> {
        if !self.input_grabbed || !self.P.prox {
            return None;
        }
        let (nx, ny) = self.raw_hover()?;
        Some(self.map(&self.pen_cal, nx, ny))
    }

    // A contact frame while Live view follows the hover: the raw digitizer's place against the scaled
    // pen's, to learn how one lies on the other (pensync_hover_learn). Only a raw frame of the same moment
    // counts (within 4 ms, by both nodes' CLOCK_MONOTONIC times): after a wait the newest raw frame is
    // ahead of the pen frames still being read. grimoire.log names the orientation once it is decided,
    // and again only if it changes.
    fn hover_learn(&mut self) {
        if !self.input_grabbed
            || self.wacom.fd < 0 || !self.pen_mono || !self.wacom_mono || !self.W.placed || self.W.p <= self.wacom.ap.minimum {
            return;
        }
        let a = self.wacom_frame_us;
        let b = self.pen_frame_us;
        if a == 0 || a.abs_diff(b) > 4000 {
            return;
        }
        pensync_hover_learn(&mut self.hover, self.W.x, self.W.y, self.P.x, self.P.y);
        let o = pensync_hover_orientation(&self.hover);
        if self.hover.n < PENSYNC_HOVER_FRAMES as u32 || o == self.hover_said {
            return;
        }
        self.hover_said = o;
        if o >= 0 {
            qlog!(
                concat!(
                    "input: Live view's hovering pen comes from the raw digitizer, x %s, y %s (%.1f px from the pen on ",
                    "average over %u contact frames)"
                ),
                if o & 1 != 0 { "reversed" } else { "as is" },
                if o & 2 != 0 { "reversed" } else { "as is" },
                pensync_hover_mean(&self.hover, o) as f64,
                self.hover.n
            );
            return;
        }
        let mut best = pensync_hover_mean(&self.hover, 0);
        for i in 1..4 {
            if pensync_hover_mean(&self.hover, i) < best {
                best = pensync_hover_mean(&self.hover, i);
            }
        }
        qlog!(
            concat!(
                "input: the raw digitizer's place is %.1f px from the pen's at best (%u contact frames): Live view shows ",
                "no hovering pen"
            ),
            best as f64,
            self.hover.n
        );
    }

    fn emit_pen(&mut self, t: InType) {
        let mut e = InEvent { type_: t, ..Default::default() };
        e.nx = norm(self.P.x, &self.pen.ax);
        e.ny = norm(self.P.y, &self.pen.ay);
        (e.x, e.y) = self.map(&self.pen_cal, e.nx, e.ny);
        // The raw digitizer's pressure and tilt of this frame's moment (raw), else its newest.
        let (mut wp, mut wtx, mut wty) = (self.W.p, self.W.tx, self.W.ty);
        let mut raw = PenRaw::default();
        if self.wacom.fd >= 0 && self.pen_mono && self.wacom_mono && pensync_raw_at(&self.raw, self.pen_frame_us, &mut raw) {
            (wp, wtx, wty) = (raw.p, raw.tx, raw.ty);
        }
        let contact = t == InType::PenDown || t == InType::PenMove;
        e.pressure = if t == InType::PenUp || t == InType::PenHover || t == InType::PenLeave { 0.0 } else { norm(self.P.p, &self.pen.ap) };
        if self.wacom.fd >= 0 && self.W.seen && contact {
            e.pressure = norm(wp, &self.wacom.ap);
        }
        if contact {
            if e.pressure > 0.0 || self.W.seen {
                self.pressure_seen = true;
            }
            // (++contact_frames only while no pressure was seen, as the C's && has it)
            if !self.pressure_seen {
                self.contact_frames = self.contact_frames.wrapping_add(1);
                if self.contact_frames > 40 && !self.pressure_missing {
                    self.pressure_missing = true;
                    qlog!("input: pen touches but reports no pressure; using a fixed 0.6 until it does");
                }
            }
            if self.pressure_missing && !self.pressure_seen {
                e.pressure = 0.6;
            }
        }
        if self.wacom.fd >= 0 {
            e.tilt_x = tilt_deg(wtx, &self.wacom.atx);
            e.tilt_y = tilt_deg(wty, &self.wacom.aty);
        } else {
            e.tilt_x = if self.caps.has_tilt { tilt_deg(self.P.tx, &self.pen.atx) } else { 0.0 };
            e.tilt_y = if self.caps.has_tilt { tilt_deg(self.P.ty, &self.pen.aty) } else { 0.0 };
        }
        // Into the screen's frame, like the position (the calligraphy nib turns with the lean).
        // On the owner's Scribe 2, held the way its sensor calls upside down, a right hand reads
        // (-40, -46) raw: leaning toward the screen's bottom right, (+40, +46) here.
        calib_tilt(&self.pen_cal, self.rot, self.tilt_flip, &mut e.tilt_x, &mut e.tilt_y,
        );
        e.eraser = self.P.eraser;
        e.side_button = self.P.side || self.W.side;
        if self.input_grabbed && contact {
            self.hover_learn();
        }
        e.t_ms = now_ms();
        e.k_ms = pensync_ms(self.pen_frame_us, self.pen_mono, e.t_ms);
        // (Live view's identity: this frame covers the raw one it was relayed from, live.rs)
        e.k_us = if self.pen_mono { self.pen_frame_us as u32 } else { 0 };
        e.gap = self.pen_gap;
        if self.ink_mono && contact {
            // A stroke's samples start the oldest afresh: one left from the stroke before (its last
            // sample never got a fast frame of its own) would time this stroke's first frame from it.
            // (Not at the lift: the lift is emitted as it is read, with the stroke's last samples, before
            // the frame that draws them.)
            if t == InType::PenDown {
                self.ink_oldest = 0;
            }
            self.ink_newest = self.pen_frame_us;
            if self.ink_oldest == 0 {
                self.ink_oldest = self.pen_frame_us;
            }
            self.ink_raw = if self.wacom.fd >= 0 { lat_match_pen(&mut self.match_, self.P.x, self.P.y, 3) } else { 0 };
        }
        self.q_push(e);
    }

    /// touchgest's events, in order: read now (t_ms), at the frame's kernel time (k_ms, touchgest's).
    fn tg_push(&mut self, out: &TgOut) {
        let now = now_ms();
        for e in &out.ev[..out.n] {
            let mut e = *e;
            e.t_ms = now;
            self.q_push(e);
        }
    }

    // The pen as its frames report it, for pensync.h.
    fn pen_state(&self) -> PenState {
        let p = &self.P;
        PenState {
            tool_pen: p.tool_pen,
            tool_rubber: p.tool_rubber,
            touching: p.touching,
            side: p.side,
            x: p.x,
            y: p.y,
            p: p.p,
            tx: p.tx,
            ty: p.ty,
        }
    }

    fn pen_frame(&mut self) {
        self.pen_good = self.pen_state();
        // A pen still touching the final PIN key must lift before it can start new ink.
        if self.pen_resume_block {
            if self.P.touching {
                return;
            }
            self.pen_resume_block = false;
            self.P.was_touching = false;
        }
        self.P.prox = self.P.tool_pen || self.P.tool_rubber;
        // The eraser end may only be visible on the raw digitizer (the virtual pen can drop it).
        self.P.eraser = self.P.tool_rubber || self.W.rubber;
        let s = self.pen_state();
        let touching = pensync_contact(&s, self.pen.has_touch_btn, self.pen.ap.minimum);
        if self.trace.fd >= 0 {
            self.trace_pen(touching);
        }
        if ((self.P.prox && !self.P.was_prox) || (touching && !self.P.was_touching)) && touchgest_busy(&self.tg) {
            // The pen arrived (or touched) while a finger, likely the palm, was down: that touch ends
            // with the off-screen cancel, a pinch with PinchEnd, before the pen's own event, and every
            // contact waits to lift. (The pen always wins: a PenDown must never meet a pinch.)
            let mut out = TgOut::default();
            touchgest_cancel(&mut self.tg, pensync_ms(self.pen_frame_us, self.pen_mono, now_ms()), &mut out,
            );
            self.tg_push(&out);
        }
        if touching && !self.P.was_touching {
            self.emit_pen(InType::PenDown);
        } else if touching && self.P.dirty {
            self.emit_pen(InType::PenMove);
        } else if !touching && self.P.was_touching {
            self.emit_pen(InType::PenUp);
        } else if !touching && self.P.prox && self.P.dirty {
            self.emit_pen(InType::PenHover);
        }
        if !self.P.prox && self.P.was_prox {
            self.emit_pen(InType::PenLeave);
            self.pen_left_ms = now_ms();
        }
        if touching {
            // (where it touched, for touchgest; emit_pen's own mapping, read on the touch side)
            self.pen_last = self.map(&self.pen_cal, norm(self.P.x, &self.pen.ax), norm(self.P.y, &self.pen.ay),
            );
        }
        self.P.was_touching = touching;
        self.P.was_prox = self.P.prox;
        self.P.dirty = false;
    }

    // The pen's current buttons and axes straight from the device, into s; which of them it answered
    // (PS_F_*). EVIOCGKEY last: it takes the queued button events out of the queue.
    fn pen_query(&self, s: &mut PenState) -> u32 {
        let fd = self.pen.fd;
        let mut ok = 0u32;
        let mut ai = InputAbsinfo::default();
        if eviocgabs(fd, ABS_X, &mut ai) >= 0 {
            s.x = ai.value;
            ok |= PS_F_X;
        }
        if eviocgabs(fd, ABS_Y, &mut ai) >= 0 {
            s.y = ai.value;
            ok |= PS_F_Y;
        }
        if eviocgabs(fd, ABS_PRESSURE, &mut ai) >= 0 {
            s.p = ai.value;
            ok |= PS_F_P;
        }
        if self.caps.has_tilt && eviocgabs(fd, ABS_TILT_X, &mut ai) >= 0 {
            s.tx = ai.value;
            ok |= PS_F_TX;
        }
        if self.caps.has_tilt && eviocgabs(fd, ABS_TILT_Y, &mut ai) >= 0 {
            s.ty = ai.value;
            ok |= PS_F_TY;
        }
        let mut keys = [0u8; KEYB_BYTES];
        if eviocgkey(fd, &mut keys) >= 0 {
            s.tool_pen = test_bit(&keys, BTN_TOOL_PEN as usize);
            s.tool_rubber = test_bit(&keys, BTN_TOOL_RUBBER as usize);
            s.touching = test_bit(&keys, BTN_TOUCH as usize);
            s.side = test_bit(&keys, BTN_STYLUS as usize);
            ok |= PS_F_KEYS;
        }
        ok
    }

    // The pen as the device holds it now, into P (the native PIN screen handed it back).
    fn pen_read_state(&mut self) {
        let mut s = self.pen_state();
        self.pen_query(&mut s);
        self.P.tool_pen = s.tool_pen;
        self.P.tool_rubber = s.tool_rubber;
        self.P.touching = s.touching;
        self.P.side = s.side;
        self.P.x = s.x;
        self.P.y = s.y;
        self.P.p = s.p;
        self.P.tx = s.tx;
        self.P.ty = s.ty;
        self.P.dirty = true;
    }

    // A pen event as pensync.h takes it: an ABS_MT_PRESSURE (some digitizers only report the MT
    // flavour) scaled into ABS_PRESSURE's range, as handle_pen does.
    fn pen_ev(&self, ev: &InputEvent) -> PenEv {
        let mut e = PenEv { t_us: ev_us(ev), type_: ev.type_, code: ev.code, value: ev.value ,
        };
        if ev.type_ != EV_ABS || ev.code != ABS_MT_PRESSURE {
            return e;
        }
        let (ap, amp) = (&self.pen.ap, &self.pen.amp);
        if amp.maximum <= amp.minimum {
            e.type_ = EV_MSC; // no range to scale it by: nothing to take from it (pensync follows no EV_MSC)
            return e;
        }
        e.code = ABS_PRESSURE;
        e.value = ap.minimum.wrapping_add(
            ((ev.value.wrapping_sub(amp.minimum) as i64 * ap.maximum.wrapping_sub(ap.minimum) as i64)
                / amp.maximum.wrapping_sub(amp.minimum) as i64) as i32,
        );
        e
    }

    // The kernel dropped pen events (its buffer overflowed: Grimoire's thread was held up). rest is
    // what the read held from the SYN_DROPPED on. The rest of the queue is read first, reporting
    // nothing; then the device is asked what it holds now; then pensync_tail says what to report:
    // the tail's frames in order, joined straight from the last good point, never the device's present
    // position ahead of older frames (that drew a jump forward and a spur traced back).
    fn pen_resync(&mut self, rest: &[InputEvent], read_gap: u32, first_read: bool) {
        // (The C's three static arrays; a resync is rare, so they are made for it.)
        let mut raw = vec![InputEvent::default(); PEN_TAIL_MAX];
        let mut ev = vec![PenEv::default(); PEN_TAIL_MAX];
        let mut frames = vec![PenFrame::default(); PEN_TAIL_MAX + 3];
        let mut n = 0usize;
        while n < rest.len() && n < PEN_TAIL_MAX {
            raw[n] = rest[n];
            n += 1;
        }
        while n < PEN_TAIL_MAX {
            let r = read_events(self.pen.fd, &mut raw[n..]);
            if r <= 0 {
                break;
            }
            n += r as usize / size_of::<InputEvent>();
        }
        for i in 0..n {
            ev[i] = self.pen_ev(&raw[i]);
        }
        let mut now_st = self.pen_good;
        let now_ok = self.pen_query(&mut now_st);
        let t = PenTail {
            good: self.pen_good,
            ev: &ev[..n],
            now: now_st,
            now_ok,
            touch_btn: self.pen.has_touch_btn,
            p_min: self.pen.ap.minimum,
        };
        let mut info = PenTailInfo::default();
        let nf = pensync_tail(&t, &mut frames, &mut info);
        // One found at the first read came before Grimoire read the pen at all: a pen hovering at launch
        // fills the queue while the rest of the start runs (each node input_open closes again waits out
        // an RCU grace period). No stroke had begun, and it is not one of the stalls the count is for.
        let mut what = String::new();
        if first_read {
            csnprintf!(&mut what, 112, "before Grimoire first read the pen (not counted): %u ms since input_open", read_gap);
        } else {
            self.drops += 1;
            csnprintf!(&mut what, 112, "(%d so far): %u ms since the pen was read before", self.drops, read_gap);
        }
        // A tail spaced like the pen's samples (about 3 ms) that ends just now: Grimoire was not reading.
        // One bunched at a single time: the pen service sent a burst.
        let now = now_ms();
        qlog!(
            concat!(
                "input: pen events dropped by the kernel %s; tail %d frames, ",
                "kernel times %u..%u ms (now %u%s)%s, partial frame %d events; device now at %d,%d, the tail ends at %d,%d%s"
            ),
            &what,
            info.frames,
            pensync_ms(info.first_us, self.pen_mono, now),
            pensync_ms(info.last_us, self.pen_mono, now),
            now,
            if self.pen_mono { "" } else { ", not CLOCK_MONOTONIC" },
            if info.keys { " with buttons" } else { "" },
            info.skipped,
            t.now.x,
            t.now.y,
            info.x,
            info.y,
            if info.lifted { "; the pen lifted in the lost stretch" } else { "" }
        );
        for f in &frames[..nf as usize] {
            self.P.tool_pen = f.s.tool_pen;
            self.P.tool_rubber = f.s.tool_rubber;
            self.P.touching = f.s.touching;
            self.P.side = f.s.side;
            self.P.x = f.s.x;
            self.P.y = f.s.y;
            self.P.p = f.s.p;
            self.P.tx = f.s.tx;
            self.P.ty = f.s.ty;
            self.P.dirty = f.moved;
            self.pen_frame_us = f.t_us;
            self.pen_gap = f.gap;
            self.pen_frame();
            self.pen_gap = false;
        }
        // An unfinished last frame (only if PEN_TAIL_MAX ran out) goes on as usual.
        for e in &raw[info.used as usize..n] {
            self.handle_pen(e);
        }
    }

    fn handle_pen(&mut self, ev: &InputEvent) {
        if self.raw_logged < 160 && ev.type_ != EV_SYN {
            self.raw_logged += 1;
            qlog!("input: raw pen type %u code %u value %d", ev.type_, ev.code, ev.value);
        }
        match ev.type_ {
            EV_KEY => match ev.code {
                BTN_TOOL_PEN => self.P.tool_pen = ev.value != 0,
                BTN_TOOL_RUBBER => self.P.tool_rubber = ev.value != 0,
                BTN_TOUCH => self.P.touching = ev.value != 0,
                BTN_STYLUS => self.P.side = ev.value != 0,
                _ => {}
            },
            EV_ABS => {
                match ev.code {
                    ABS_X => self.P.x = ev.value,
                    ABS_Y => self.P.y = ev.value,
                    ABS_PRESSURE => self.P.p = ev.value,
                    ABS_MT_PRESSURE => {
                        // some digitizers only report the MT flavour
                        let (ap, amp) = (&self.pen.ap, &self.pen.amp);
                        if amp.maximum > amp.minimum {
                            self.P.p = ap.minimum.wrapping_add(
                                ((ev.value.wrapping_sub(amp.minimum) as i64 * ap.maximum.wrapping_sub(ap.minimum) as i64)
                                    / amp.maximum.wrapping_sub(amp.minimum) as i64) as i32,
                            );
                        }
                    }
                    ABS_TILT_X => self.P.tx = ev.value,
                    ABS_TILT_Y => self.P.ty = ev.value,
                    _ => return,
                }
                self.P.dirty = true;
            }
            // (A SYN_DROPPED never gets here: drain_pen hands it to pen_resync.)
            EV_SYN if ev.code == SYN_REPORT => {
                self.pen_frame_us = ev_us(ev);
                self.pen_frame();
            }
            _ => {}
        }
    }

    // Reads the pen like drain, but a SYN_DROPPED and everything after it go to pen_resync.
    fn drain_pen(&mut self) {
        let mut evs = [InputEvent::default(); 64];
        loop {
            let r = read_events(self.pen.fd, &mut evs);
            if r <= 0 {
                break;
            }
            let now = now_ms();
            let gap = now.wrapping_sub(self.pen_read_ms);
            let first = !self.pen_read;
            self.pen_read_ms = now;
            self.pen_read = true;
            let n = r as usize / size_of::<InputEvent>();
            let mut i = 0usize;
            while i < n && !(evs[i].type_ == EV_SYN && evs[i].code == SYN_DROPPED) {
                self.handle_pen(&evs[i]);
                i += 1;
            }
            if i < n {
                self.pen_resync(&evs[i..n], gap, first);
                continue; // (it read the queue to its end: the next read finds it empty, or what came since)
            }
            if (r as usize) < size_of_val(&evs) {
                break;
            }
        }
    }

    // The panel's slots as it holds them now, after it dropped events: who is down, where, with what
    // tool, size and pressure, and the slot its next events are for. A panel with fewer slots than
    // MAX_SLOTS answers for its own only (mt_column), so a slot it does not have reads as no contact.
    fn touch_resync(&mut self) {
        let fd = self.touch.fd;
        let codes = [ABS_MT_TRACKING_ID, ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_TOOL_TYPE, ABS_MT_TOUCH_MAJOR, ABS_MT_PRESSURE,
        ];
        let mut col = [0i32; MAX_SLOTS];
        for code in codes {
            let empty = if code == ABS_MT_TRACKING_ID { -1 } else { 0 };
            if !mt_column(&mut |buf| eviocgmtslots(fd, buf) >= 0, code as i32, empty, &mut col,
            ) {
                continue;
            }
            match code {
                ABS_MT_TRACKING_ID => self.T.id = col,
                ABS_MT_POSITION_X => self.T.x = col,
                ABS_MT_POSITION_Y => self.T.y = col,
                ABS_MT_TOOL_TYPE => self.T.tool = col,
                ABS_MT_TOUCH_MAJOR => self.T.major = col,
                _ => self.T.pressure = col,
            }
        }
        let mut ai = InputAbsinfo::default();
        if eviocgabs(fd, ABS_MT_SLOT, &mut ai) >= 0 {
            self.T.slot = clampi(ai.value, 0, MAX_SLOTS as i32 - 1);
        }
    }

    // One SYN_REPORT's contacts, as touchgest takes them: logical px through map() (and the normalized
    // place calibration reads), the frame's kernel time; its events go into the queue in order.
    fn touch_frame(&mut self) {
        let now = now_ms();
        let mut f = TgFrame { k_ms: pensync_ms(self.T.frame_us, self.touch_mono, now), ..Default::default() };
        for s in 0..MAX_SLOTS {
            if self.T.id[s] < 0 || f.n >= TG_MAX {
                continue;
            }
            let nx = norm(self.T.x[s], &self.touch.ax);
            let ny = norm(self.T.y[s], &self.touch.ay);
            let (x, y) = self.map(&self.touch_cal, nx, ny);
            let tool = match self.T.tool[s] {
                1 => 1,
                2 => 2,
                _ => 0,
            };
            f.c[f.n] = TgContact { id: self.T.id[s], slot: s as i32, x, y, nx, ny, major: self.T.major[s], tool, pressure: self.T.pressure[s] ,
            };
            f.n += 1;
        }
        let pen = TgPen { prox: self.P.prox, touching: self.P.was_touching, left_ms: self.pen_left_ms, last: self.pen_last ,
            hover: self.tg_hover,
        };
        let mut out = TgOut::default();
        let was_busy = touchgest_busy(&self.tg);
        touchgest_frame(&mut self.tg, &f, &pen, &mut out);
        self.tg_push(&out);
        self.touch_log(&f, &out, was_busy);
    }

    // grimoire.log's touch lines (only while grabbed: touchgest is never fed at the PIN) and the trace.
    fn touch_log(&mut self, f: &TgFrame, out: &TgOut, was_busy: bool) {
        let grabbed = self.input_grabbed;
        let mut line = String::new();
        if f.n >= 2 && self.tlog.frames < TLOG_FRAMES && touchgest_log_frame(f, grabbed, &mut line) {
            self.tlog.frames += 1;
            self.tlog_line(f.k_ms, &line);
        }
        if f.n == 1 && f.c[0].major > 0 {
            let l = &mut self.tlog;
            if (l.tip_frames as usize) < TIP_N {
                l.tip[l.tip_frames as usize] = f.c[0].major;
            }
            l.tip_frames += 1;
            l.tip_max = l.tip_max.max(f.c[0].major);
        }
        if self.trace.fd >= 0 {
            touchgest_trace_frame(f, grabbed, &mut self.trace.buf);
            touchgest_trace_outcome(f.k_ms, out.outcome, grabbed, &mut self.trace.buf);
        }
        let ended = was_busy && !touchgest_busy(&self.tg);
        if ended {
            // The touch session is over: its gaps and how it went, or one finger's size.
            if touchgest_log_session(&self.tg, grabbed, &mut line) {
                self.tlog_line(f.k_ms, &line);
            if touchgest_log_measure(&self.tg, grabbed, &mut line) {
                    self.tlog_line(f.k_ms, &line);
                }
            } else if touchgest_session_contacts(&self.tg) == 1 && self.tlog.tip_frames > 0 {
                let n = (self.tlog.tip_frames as usize).min(TIP_N);
                let mut v = self.tlog.tip;
                v[..n].sort_unstable();
                if touchgest_log_fingertips(v[n / 2], self.tlog.tip_max, self.tlog.tip_frames, grabbed, &mut line,
                ) {
                    self.tlog_line(f.k_ms, &line);
                }
            }
            self.tlog.frames = 0;
            self.tlog.tip_frames = 0;
            self.tlog.tip_max = 0;
        }
        if self.trace.fd >= 0 && (ended || self.trace.buf.len() >= TRACE_FLUSH) {
            self.trace_flush(f.k_ms);
        }
    }

    fn tlog_line(&mut self, k_ms: u32, line: &str) {
        match self.tlog.budget.admit(k_ms) {
            LogDecision::Line => qlog!("%s", line),
            LogDecision::Cap => {
                qlog!("touch: log cap reached (2000 lines/60 s; resumes next window)")}
            LogDecision::Skip => {}}
    }

    // The pen's coming and going for the trace, never where (no write here: the touch side flushes).
    fn trace_pen(&mut self, touching: bool) {
        let what = if self.P.prox && !self.P.was_prox {
            "prox"
        } else if touching && !self.P.was_touching {
            "down"
        } else if !touching && self.P.was_touching {
            "up"
        } else if !self.P.prox && self.P.was_prox {
            "leave"
        } else {
            return;
        };
        if self.trace.buf.len() + 32 <= TRACE_ROOM {
            let k = pensync_ms(self.pen_frame_us, self.pen_mono, now_ms());
            touchgest_trace_pen(k, what, self.input_grabbed, &mut self.trace.buf);
        }
    }

    // What waits goes to the file; at TRACE_MAX the trace ends with one "E <k_ms> cap" line.
    fn trace_flush(&mut self, k_ms: u32) {
        if self.trace.fd < 0 {
            return;
        }
        let capped = self.trace.written + self.trace.buf.len() as u64 >= TRACE_MAX;
        if capped {
            use std::fmt::Write;
            let _ = writeln!(self.trace.buf, "E {k_ms} cap");
        }
        let mut left = self.trace.buf.as_bytes();
        while !left.is_empty() {
            // SAFETY: write reads at most left.len() bytes of our buffer; the fd is the trace's own.
            let w = unsafe { libc::write(self.trace.fd, left.as_ptr() as *const c_void, left.len()) };
            if w < 0 && errno() == libc::EINTR {
                continue;
            }
            if w <= 0 {
                break; // (a full or failed userstore: what did not fit is dropped)
            }
            self.trace.written += w as u64;
            left = &left[w as usize..];
        }
        self.trace.buf.clear();
        if capped {
            close_fd(self.trace.fd);
            self.trace.fd = -1;
        }
    }

    // The trace's header: at each open of the file, and when the screen turns (its places are logical).
    fn trace_header(&mut self) {
        let (w, h) = if self.rot & 1 != 0 { (self.h, self.w) } else { (self.w, self.h) };
        touchgest_trace_header(self.rot, w, h, self.dpi, &mut self.trace.buf);
    }

    fn handle_touch(&mut self, ev: &InputEvent) -> Hover {
        self.touch_events = self.touch_events.wrapping_add(1);
        if ev.type_ == EV_SYN && ev.code == SYN_DROPPED {
            self.T.dropped = true;
            return None;
        }
        if ev.type_ == EV_SYN && ev.code == SYN_REPORT {
            self.T.frame_us = ev_us(ev);
            if self.T.dropped {
                // What the panel holds now, then everything down waits to lift: a lift lost in the
                // drop leaves no contact behind, and nothing half seen becomes a tap.
                self.T.dropped = false;
                self.touch_resync();
                let mut out = TgOut::default();
                touchgest_cancel(&mut self.tg, pensync_ms(self.T.frame_us, self.touch_mono, now_ms()), &mut out,
                );
                self.tg_push(&out);
            }
            self.touch_frame();
            return None;
        }
        if self.T.dropped {
            return None; // the rest of the frame the drop cut: the resync reads what it left
        }
        if ev.type_ == EV_ABS {
            let s = self.T.slot as usize;
            match ev.code {
                ABS_MT_SLOT => self.T.slot = clampi(ev.value, 0, MAX_SLOTS as i32 - 1),
                ABS_MT_TRACKING_ID => {
                    if ev.value >= 0 && self.T.id[s] < 0 {
                        self.T.tool[s] = 0;
                        self.T.major[s] = 0;
                        self.T.pressure[s] = 0;
                    }
                    self.T.id[s] = ev.value;
                }
                ABS_MT_TOOL_TYPE => self.T.tool[s] = ev.value,
                ABS_MT_POSITION_X => self.T.x[s] = ev.value,
                ABS_MT_POSITION_Y => self.T.y[s] = ev.value,
                ABS_MT_TOUCH_MAJOR => self.T.major[s] = ev.value,
                ABS_MT_PRESSURE => self.T.pressure[s] = ev.value,
                _ => {}
            }
        } else if ev.type_ == EV_KEY && ev.code == BTN_TOUCH && ev.value == 0 {
            // Belt and braces: some panels only signal the final lift through BTN_TOUCH.
            self.T.id = [-1; MAX_SLOTS];
        }
        None
    }

    fn handle_orient_fd(&mut self) {
        let mut evs = [InputEvent::default(); 16];
        let r = read_events(self.orient_fd, &mut evs);
        let mut i = 0isize;
        while r > 0 && i < r / size_of::<InputEvent>() as isize {
            let ev = evs[i as usize];
            i += 1;
            if ev.type_ != EV_ABS || ev.code != ABS_PRESSURE {
                continue;
            }
            let t = orient_code_to_turns(ev.value);
            qlog!("input: orientation code %d -> %d", ev.value, t);
            if t < 0 {
                continue;
            }
            let e = InEvent { type_: InType::Orientation, value: t, t_ms: now_ms(), ..Default::default() };
            self.q_push(e);
        }
    }

    fn handle_powerd_fd(&mut self) {
        let len = self.powerd_len;
        let room = POWERD_BUF - 1 - len;
        // SAFETY: read writes at most room bytes into powerd_buf from len on: len + room is 511, inside
        // the buffer, and bytes are bytes.
        let r = unsafe { libc::read(self.powerd_fd, self.powerd_buf[len..].as_mut_ptr() as *mut c_void, room,
            ) };
        if r <= 0 {
            if r == 0 {
                // the watcher died: stop polling it
                close_fd(self.powerd_fd);
                self.powerd_fd = -1;
                qlog!("input: power service watcher ended");
            }
            return;
        }
        self.powerd_len += r as usize;
        self.powerd_buf[self.powerd_len] = 0;
        // (strchr: a newline before the buffer's first NUL)
        while let Some(nl) = cbytes(&self.powerd_buf).iter().position(|&c| c == b'\n') {
            self.powerd_buf[nl] = 0;
            let line = self.powerd_buf[..nl].to_vec();
            let mut e = InEvent::default();
            if line.starts_with(b"goingToScreenSaver") {
                e.type_ = InType::Sleep;
            }
            // Only outOfScreenSaver is a wake. (wakeupFromSuspend is not: the Kindle also resumes
            // briefly on its own timers while the screensaver stays up, and treating that as a wake
            // drew Grimoire over the screensaver and lost track of the real wake that followed.)
            else if line.starts_with(b"outOfScreenSaver") {
                e.type_ = InType::Wake;
            }
            if e.type_ != InType::None {
                qlog!("input: power service: %s", &lossy(&line));
                // (strcspn(line, " "): the first space, or the end)
                let sp = line.iter().position(|&c| c == b' ').unwrap_or(line.len());
                e.value = atoi(&line[sp..]);
                e.t_ms = now_ms();
                self.q_push(e);
            } else if post_inc(&mut self.powerd_noise) < 20 || self.powerd_noise % 100 == 0 {
                // A failed run, or what lipc-wait-event said on the way (its errors come through
                // too). A watcher failing over and over would fill the log: the first 20, then 1 in 100.
                if line.starts_with(WATCH_FAILED.as_bytes()) {
                    qlog!(
                        "input: power service watcher failed, retrying (lipc-wait-event exit status%s; watcher message %d)",
                        &lossy(&line[WATCH_FAILED.len()..]),
                        self.powerd_noise
                    );
                } else {
                    qlog!("input: power service watcher said: %s", &lossy(&line));
                }
            }
            let used = nl + 1;
            self.powerd_buf.copy_within(used..self.powerd_len + 1, 0);
            self.powerd_len -= used;
        }
        if self.powerd_len >= POWERD_BUF - 1 {
            self.powerd_len = 0; // no newline in sight: drop
        }
    }

    fn push_power(&mut self, t: InType, held_ms: i32) {
        if t == InType::None {
            return;
        }
        let e = InEvent { type_: t, value: held_ms, t_ms: now_ms(), ..Default::default() };
        self.q_push(e);
    }

    fn handle_power_fd(&mut self) {
        let mut evs = [InputEvent::default(); 16];
        let r = read_events(self.power_fd, &mut evs);
        let mut i = 0isize;
        while r > 0 && i < r / size_of::<InputEvent>() as isize {
            let ev = evs[i as usize];
            i += 1;
            if ev.type_ == EV_KEY && ev.code == KEY_POWER {
                // Timed by when the kernel saw it, not when it was read: a short press whose release is
                // read late (the loop was busy) must not count as a 4 s hold, nor a real one as short.
                let t = if self.power_mono {
                    (ev.input_event_sec as u64).wrapping_mul(1000).wrapping_add(ev.input_event_usec as u64 / 1000) as u32
                } else {
                    now_ms()
                };
                let mut held = 0;
                let ty = powerkey_event(&mut self.pkey, ev.value, t, &mut held);
                self.push_power(ty, held);
            }
        }
    }

    // The 4 s hold, checked on every poll (they come at least every 30 ms), timeouts included.
    fn power_hold_check(&mut self) {
        let mut held = 0;
        let fd = self.power_fd;
        let t = powerkey_tick(&mut self.pkey, now_ms(), &mut || power_key_down(fd), &mut held,
        );
        if t == InType::PowerUp {
            qlog!("input: the power key is up at the %d ms check: its release was missed", INPUT_POWER_HOLD_MS);
        }
        self.push_power(t, held);
    }

    // ---- setup ----

    fn default_calib(&self) -> (InputCalib, InputCalib) {
        // KOReader applies no axis swap/mirror for the Scribe: the axes map straight onto the
        // upright framebuffer. Swap only if the ranges say the axes are crossed.
        let mut p = InputCalib { valid: false, swap_xy: false, ax: self.w as f32, bx: 0.0, ay: self.h as f32, by: 0.0 ,
        };
        if self.pen.fd >= 0 {
            let (mx, my) = (self.pen.ax.maximum, self.pen.ay.maximum);
            if mx.wrapping_sub(self.h).wrapping_abs() < mx.wrapping_sub(self.w).wrapping_abs()
                && my.wrapping_sub(self.w).wrapping_abs() < my.wrapping_sub(self.h).wrapping_abs()
            {
                p.swap_xy = true;
            }
        }
        let mut t = InputCalib { valid: false, swap_xy: false, ax: self.w as f32, bx: 0.0, ay: self.h as f32, by: 0.0 ,
        };
        if self.touch.fd >= 0 {
            let (mx, my) = (self.touch.ax.maximum, self.touch.ay.maximum);
            let aspect_dev = if my != 0 { mx as f32 / my as f32 } else { 1.0 };
            let aspect_scr = if self.h != 0 { self.w as f32 / self.h as f32 } else { 1.0 };
            if (aspect_dev > 1.0) != (aspect_scr > 1.0) {
                t.swap_xy = true;
            }
        }
        (p, t)
    }

    fn key_push(&mut self, p: &HidPress) {
        let e = InEvent {
            type_: if p.up { InType::KeyUp } else { InType::Key },
            value: p.code,
            key_mods: p.mods as u32,
            gamepad: p.gamepad,
            t_ms: now_ms(),
            ..Default::default()
        };
        self.q_push(e);
    }
    // A resync's releases: what was held before and is not now (a hold ends with them, app.c).
    fn key_ups(&mut self, i: usize, held: Option<&[u8; HID_KEY_BYTES]>, hat_x: i32, hat_y: i32) {
        let mut ups = [HidPress::default(); 16];
        let k = &mut self.keys[i];
        let n = hidkeys_resync(&mut k.keys, k.kind, held, hat_x, hat_y, &mut ups);
        for u in &ups[..n as usize] {
            self.key_push(u);
        }
    }
    fn key_resync(&mut self, i: usize) -> bool {
        let fd = self.keys[i].fd;
        let mut keys = [0u8; HID_KEY_BYTES];
        if eviocgkey(fd, &mut keys) < 0 {
            return false;
        }
        let mut hats = [0i32; 2];
        let mut trig = [i32::MIN; 2];
        self.keys[i].hatmask = 0;
        for (axis, hat) in hats.iter_mut().enumerate() {
            let mut info = InputAbsinfo::default();
            if eviocgabs(fd, ABS_HAT0X + axis as u16, &mut info) >= 0 && info.minimum == -1 && info.maximum == 1 {
                *hat = info.value;
                self.keys[i].hatmask |= 1u32 << axis;
            }
        }
        // The analog triggers as they read now: one pulled is held as a button is (hidkeys_trigger_sync).
        for (j, v) in trig.iter_mut().enumerate() {
            let mut info = InputAbsinfo::default();
            let axis = self.keys[i].trig.axis[j];
            if axis != 0 && eviocgabs(fd, axis as u16, &mut info) >= 0 {
                *v = info.value;
            }
        }
        let k = &self.keys[i];
        hidkeys_trigger_sync(&k.keys, &k.trig, &trig, &mut keys);
        self.key_ups(i, Some(&keys), hats[0], hats[1]);
        true
    }
    // A controller's analog triggers (hidkeys_triggers), from its axes' ranges and where they rest now;
    // until its first report, when that cannot tell (trig_wait), it has none. Axis numbers and ranges go
    // to the log, as attachment paths do: never a press or a name.
    fn key_triggers(&mut self, i: usize, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) {
        let mut info = [HidAxis::default(); HIDKEYS_AXES];
        for (code, a) in info.iter_mut().enumerate() {
            let mut ai = InputAbsinfo::default();
            if test_bit(axes, code) && eviocgabs(self.keys[i].fd, code as u16, &mut ai) >= 0 {
                *a = HidAxis { minimum: ai.minimum, maximum: ai.maximum, value: ai.value ,
                };
            }
        }
        let k = &mut self.keys[i];
        k.trig_wait = !hidkeys_triggers(&mut k.trig, keys, axes, &info);
        for j in 0..2usize {
            let axis = k.trig.axis[j];
            if axis != 0 {
                qlog!(
                    "input: %s at %s is axis %d (%d to %d)",
                    if j != 0 { "RT" } else { "LT" },
                    &k.path,
                    axis,
                    info[axis as usize].minimum,
                    info[axis as usize].maximum
                );
            }
        }
    }
    fn key_announce(&mut self, i: usize, type_: InType) {
        let k = &self.keys[i];
        let mut e = InEvent { type_, gamepad: k.kind == HidKind::Gamepad, value: k.held as i32, t_ms: now_ms(), ..Default::default() };
        e.hid_name.set_bytes(k.name.as_bytes());
        self.q_push(e);
    }
    // A device that went away: unplugged, out of range, or unreadable after a resync.
    fn key_detach(&mut self, i: usize) {
        close_fd(self.keys[i].fd);
        self.keys[i].fd = -1;
        qlog!("input: optional input at %s went away", &self.keys[i].path);
        self.key_announce(i, InType::HidDetach);
    }
    // A node input_open looked at and leaves that is no keyboard or controller either.
    fn key_none_note(&mut self, fd: i32, path: &str, name: &str, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES],
    ) {
        if hidkeys_optional(name, keys, axes) == HidKind::None {
            if let Some(st) = fstat_fd(fd) {
                let n = node_of(&st);
                hidkeys_node_none(&mut self.key_none, path, &n);
            }
        }
    }
    // The device numbers of every node Grimoire holds open.
    fn held_nodes(&self, out: &mut [u64; 5 + KEY_DEVS]) -> usize {
        let cap = out.len();
        let mut fds = [-1i32; 5 + KEY_DEVS];
        fds[..5].copy_from_slice(&[self.pen.fd, self.wacom.fd, self.touch.fd, self.power_fd, self.orient_fd,
        ]);
        let mut nfds = 5;
        let mut n = 0;
        for i in 0..KEY_DEVS {
            if !self.keys_init {
                break;
            }
            fds[nfds] = self.keys[i].fd;
            nfds += 1;
        }
        for &fd in &fds[..nfds] {
            if n >= cap {
                break;
            }
            if fd >= 0 {
                if let Some(st) = fstat_fd(fd) {
                    out[n] = dev_of(&st);
                    n += 1;
                }
            }
        }
        n
    }
    // Devices found by the scan that starts watching are not announced: they did not just come.
    // Every node is stat()ed, and opened only if it may be a keyboard or controller that came: closing
    // an evdev node waits out an RCU grace period (22-96 ms on the Scribe), and opening Grimoire's own
    // seven took the thread 320-370 ms, long enough for the kernel to drop pen events.
    fn key_scan(&mut self, announce: bool) {
        self.keys_scan = now_ms();
        let t0 = mono_us();
        let mut slowest_us = 0u64;
        let mut slowest = String::new();
        let mut nodes = 0;
        let mut opened = 0;
        let mut held = [0u64; 5 + KEY_DEVS];
        let nheld = self.held_nodes(&mut held);
        let Ok(d) = std::fs::read_dir("/dev/input") else { return ;
        };
        for e in d {
            let Ok(e) = e else { break };
            let fname = e.file_name();
            if !fname.as_bytes().starts_with(b"event") {
                continue;
            }
            nodes += 1;
            // (the kernel names its nodes eventN: ASCII)
            let path = cformat!("/dev/input/%s", &*fname.to_string_lossy());
            let mut slot: i32 = -1;
            let mut seen = false;
            for i in 0..KEY_DEVS {
                if self.keys[i].fd < 0 {
                    slot = i as i32;
                } else if self.keys[i].path.as_bytes() == path.as_bytes() {
                    seen = true;
                }
            }
            if seen || slot < 0 {
                continue;
            }
            let Some(st) = stat_path(&path) else { continue };
            if (st.st_mode & libc::S_IFMT) != libc::S_IFCHR {
                continue;
            }
            let node = node_of(&st);
            if hidkeys_node(&self.key_none, &held[..nheld], &path, &node) != HidNodeVerdict::Probe {
                continue;
            }
            let n0 = mono_us();
            let fd = open_node(&path);
            if fd < 0 {
                continue;
            }
            opened += 1;
            let mut keys = [0u8; KEYB_BYTES];
            let mut axes = [0u8; ABSB_BYTES];
            let mut name = [0u8; 64];
            eviocgname(fd, &mut name[..63]);
            eviocgbit(fd, EV_KEY, &mut keys);
            eviocgbit(fd, EV_ABS, &mut axes);
            let name_s = lossy(cbytes(&name));
            let kind = hidkeys_optional(&name_s, &keys, &axes);
            if kind == HidKind::None {
                close_fd(fd);
                hidkeys_node_none(&mut self.key_none, &path, &node);
            } else {
                let slot = slot as usize;
                self.keys[slot] = KeyDev {
                    fd,
                    kind,
                    switch_layout: kind == HidKind::Gamepad && hidkeys_switch(&name_s, &keys),
                    ..Default::default()
                };
                self.keys[slot].path.set(&path);
                let mut shown = String::new();
                hidkeys_name(&mut shown, FixedStr::<41>::CAP, &name);
                self.keys[slot].name.set(&shown);
                if kind == HidKind::Gamepad {
                    self.key_triggers(slot, &keys, &axes);
                }
                // Another app's exclusive grab (a button mapper) would starve Grimoire of its presses: ask
                // for one ourselves, let go at once, and read without one as before.
                let grabbed = ioctl_grab(fd, 1);
                let err = errno();
                self.keys[slot].held = hidkeys_held(grabbed, err);
                if grabbed == 0 {
                    ioctl_grab(fd, 0);
                }
                if !self.key_resync(slot) {
                    close_fd(fd);
                    self.keys[slot].fd = -1;
                } else {
                    qlog!(
                        "input: optional %s attached at %s%s",
                        if kind == HidKind::Gamepad { "controller" } else { "keyboard/remote" },
                        &path,
                        if self.keys[slot].held { ", held by another app" } else { "" }
                    );
                    if announce {
                        self.key_announce(slot, InType::HidAttach);
                    }
                }
            }
            let took = mono_us() - n0;
            if took > slowest_us {
                slowest_us = took;
                strlcpy_q(&mut slowest, &path, 300);
            }
        }
        let took = mono_us() - t0;
        if took > 20000 {
            qlog!(
                "input: key scan took %llu ms over %d nodes (opened %d, slowest %s %llu ms)",
                took / 1000,
                nodes,
                opened,
                if !slowest.is_empty() { slowest.as_str() } else { "none" },
                slowest_us / 1000
            );
        }
    }
    fn shortcuts_enable(&mut self, enabled: bool) {
        if !self.keys_init {
            for k in self.keys.iter_mut() {
                k.fd = -1;
            }
            self.keys_init = true;
        }
        self.keys_enabled = enabled;
        if enabled {
            self.key_scan(false);
        } else {
            for k in self.keys.iter_mut() {
                if k.fd >= 0 {
                    close_fd(k.fd);
                }
                k.fd = -1;
            }
        }
    }
    fn hids(&self, out: &mut [InputHid]) -> i32 {
        let cap = out.len();
        let mut n = 0usize;
        for k in &self.keys {
            if !(self.keys_init && self.keys_enabled && n < cap) {
                break;
            }
            if k.fd < 0 {
                continue;
            }
            out[n] = InputHid { gamepad: k.kind == HidKind::Gamepad, held: k.held, switch_layout: k.switch_layout, ..Default::default() };
            out[n].name.set_bytes(k.name.as_bytes());
            n += 1;
        }
        n as i32
    }
    fn handle_key(&mut self, ev: &InputEvent) -> Hover {
        let i = self.key_read;
        if self.keys[i].fd < 0 {
            return None; // went away earlier in this read (key_detach)
        }
        if !self.input_grabbed {
            self.key_ups(i, None, 0, 0);
            return None;
        }
        if self.keys[i].keys.dropped {
            if ev.type_ == EV_SYN && ev.code == SYN_REPORT && !self.key_resync(i) {
                self.key_detach(i);
            }
            return None;
        }
        // Its first report: where its axes rest now tells which are its triggers (one pulled in it is held,
        // as at attach).
        if self.keys[i].trig_wait && ev.type_ == EV_SYN && ev.code == SYN_REPORT {
            let mut keys = [0u8; KEYB_BYTES];
            let mut axes = [0u8; ABSB_BYTES];
            eviocgbit(self.keys[i].fd, EV_KEY, &mut keys);
            eviocgbit(self.keys[i].fd, EV_ABS, &mut axes);
            self.key_triggers(i, &keys, &axes);
            self.keys[i].trig_wait = false; // (decided now, either way)
            if !self.key_resync(i) {
                self.key_detach(i);
            }
            return None;
        }
        if ev.type_ == EV_ABS
            && (ev.code == ABS_HAT0X || ev.code == ABS_HAT0Y)
            && (self.keys[i].hatmask & (1u32 << (ev.code - ABS_HAT0X))) == 0
        {
            return None;
        }
        let mut out = [HidPress::default(); 2];
        let k = &mut self.keys[i];
        let mut n = if ev.type_ == EV_ABS { hidkeys_trigger_event(&mut k.keys, &k.trig, ev.code as i32, ev.value, &mut out[0]) } else { 0 };
        if n == 0 {
            n = hidkeys_event(&mut k.keys, k.kind, ev.type_ as i32, ev.code as i32, ev.value, &mut out,
            );
        }
        for p in &out[..n as usize] {
            self.key_push(p);
        }
        None
    }

    // The part of input_open before its children and signals: the devices.
    fn open_devices(&mut self, screen_w: i32, screen_h: i32, pen: Option<&InputCalib>, touch: Option<&InputCalib>,
    ) -> i32 {
        self.w = screen_w;
        self.h = screen_h;
        self.tilt_flip = access_ok(c"/mnt/us/grimoire/tilt-flip");
        if self.tilt_flip {
            qlog!("input: tilt-flip: reversing the pen's tilt direction");
        }
        self.P = PenSt::default();
        self.T.id = [-1; MAX_SLOTS];
        let d = match std::fs::read_dir("/dev/input") {
            Ok(d) => d,
            Err(e) => {
                qlog!("input: cannot open /dev/input: %s", &strerror(e.raw_os_error().unwrap_or(0)));
                return -1;
            }
        };
        let mut power_keys = 0;
        let mut power_rank = 0;
        let mut power_path = String::new();
        for e in d {
            let Ok(e) = e else { break };
            let fname = e.file_name();
            if !fname.as_bytes().starts_with(b"event") {
                continue;
            }
            // (the kernel names its nodes eventN: ASCII)
            let path = cformat!("/dev/input/%s", &*fname.to_string_lossy());
            let fd = open_node(&path);
            if fd < 0 {
                qlog!("input: cannot open %s: %s", &path, &strerror(errno()));
                continue;
            }
            // Every node at once: which are kept is known only after the ioctls and the log line below,
            // and by then a hovering pen may have queued events. The clock is this reader's only, so it
            // does nothing to the nodes closed again; orientation never reads event times, and touch
            // times each frame by its SYN_REPORT (touchgest.rs).
            let mono = clock_mono(fd);
            let mut name = [0u8; 64];
            name[0] = b'?';
            eviocgname(fd, &mut name[..63]);
            let mut evb = [0u8; EVB_BYTES];
            let mut keyb = [0u8; KEYB_BYTES];
            let mut absb = [0u8; ABSB_BYTES];
            eviocgbit(fd, 0, &mut evb);
            eviocgbit(fd, EV_KEY, &mut keyb);
            eviocgbit(fd, EV_ABS, &mut absb);
            let name_s = lossy(cbytes(&name));
            let has_abs = test_bit(&evb, EV_ABS as usize);
            let has_key = test_bit(&evb, EV_KEY as usize);
            {
                // (char axes[256]: each piece is what snprintf leaves of it, al moves on by its whole length)
                let mut axes = String::new();
                let mut al = 0usize;
                const NAMES: [(u16, &str); 10] = [
                    (ABS_X, "X"),
                    (ABS_Y, "Y"),
                    (ABS_PRESSURE, "P"),
                    (ABS_DISTANCE, "DIST"),
                    (ABS_TILT_X, "TX"),
                    (ABS_TILT_Y, "TY"),
                    (ABS_MT_POSITION_X, "MTX"),
                    (ABS_MT_POSITION_Y, "MTY"),
                    (ABS_MT_PRESSURE, "MTP"),
                    (ABS_MT_TOOL_TYPE, "MTTOOL"),
                ];
                let mut k = 0;
                while k < NAMES.len() && al < 256 - 40 {
                    let (code, n) = NAMES[k];
                    k += 1;
                    if !test_bit(&absb, code as usize) {
                        continue;
                    }
                    let mut ai = InputAbsinfo::default();
                    eviocgabs(fd, code, &mut ai);
                    let piece = cformat!(" %s[%d..%d r%d]", n, ai.minimum, ai.maximum, ai.resolution);
                    axes.push_str(&piece[..piece.len().min(256 - al - 1)]);
                    al += piece.len();
                }
                qlog!(
                    "input: found %s \"%s\" abs:%s keys:%s%s%s%s%s",
                    &path,
                    &name_s,
                    if !axes.is_empty() { axes.as_str() } else { " none" },
                    if test_bit(&keyb, BTN_TOOL_PEN as usize) { " TOOL_PEN" } else { "" },
                    if test_bit(&keyb, BTN_TOOL_RUBBER as usize) { " RUBBER" } else { "" },
                    if test_bit(&keyb, BTN_TOUCH as usize) { " TOUCH" } else { "" },
                    if test_bit(&keyb, BTN_STYLUS as usize) { " STYLUS" } else { "" },
                    if test_bit(&keyb, KEY_POWER as usize) { " POWER" } else { "" }
                );
            }
            let pen = has_abs
                && has_key
                && test_bit(&absb, ABS_X as usize)
                && test_bit(&absb, ABS_PRESSURE as usize)
                && test_bit(&keyb, BTN_TOOL_PEN as usize);
            let touch = !pen && has_abs && test_bit(&absb, ABS_MT_POSITION_X as usize);
            // The power key is decided by its own rule (hidkeys_power), not by what makes a keyboard.
            let power = if !pen && !touch && has_key { hidkeys_power(&name_s, &keyb, &absb) } else { -1 };
            // Orientation: ABS_PRESSURE with the accelerometer's tiny code range (15..22), not a pen.
            if !pen
                && !touch
                && has_abs
                && test_bit(&absb, ABS_PRESSURE as usize)
                && !test_bit(&absb, ABS_X as usize)
                && self.orient_fd < 0
            {
                let mut ai = InputAbsinfo::default();
                eviocgabs(fd, ABS_PRESSURE, &mut ai);
                if ai.minimum >= 10 && ai.maximum <= 30 {
                    self.orient_fd = fd;
                    self.caps.has_orientation = true;
                    let t = orient_code_to_turns(ai.value);
                    self.caps.orientation = if t < 0 { 0 } else { t };
                    qlog!(
                        "input: orientation sensor %s (%s), current code %d -> %d",
                        &path,
                        &name_s,
                        ai.value,
                        self.caps.orientation
                    );
                    continue;
                }
                qlog!(
                    concat!(
                        "input: %s (%s) has ABS_PRESSURE %d..%d, outside an orientation sensor's 10..30: not used for ",
                        "rotation"
                    ),
                    &path,
                    &name_s,
                    ai.minimum,
                    ai.maximum
                );
            }
            // The scaled pen (axis maxima match the screen) drives the pointer; another pen device,
            // if any, only supplies pressure + tilt.
            let mut scaled = false;
            if pen {
                let mut ax = InputAbsinfo::default();
                let mut ay = InputAbsinfo::default();
                eviocgabs(fd, ABS_X, &mut ax);
                eviocgabs(fd, ABS_Y, &mut ay);
                let near = |v: i32, to: i32| v.wrapping_sub(to).wrapping_abs() <= 2;
                scaled = (near(ax.maximum, self.w) && near(ay.maximum, self.h)) || (near(ax.maximum, self.h) && near(ay.maximum, self.w));
                if !scaled && self.wacom.fd < 0 && test_bit(&absb, ABS_PRESSURE as usize) {
                    self.wacom.fd = fd;
                    self.wacom_mono = mono;
                    eviocgabs(fd, ABS_X, &mut self.wacom.ax);
                    eviocgabs(fd, ABS_Y, &mut self.wacom.ay);
                    eviocgabs(fd, ABS_PRESSURE, &mut self.wacom.ap);
                    eviocgabs(fd, ABS_TILT_X, &mut self.wacom.atx);
                    eviocgabs(fd, ABS_TILT_Y, &mut self.wacom.aty);
                    qlog!(
                        "input: pressure/tilt source %s (%s), p %d..%d tilt %d..%d (not grabbed)",
                        &path,
                        &name_s,
                        self.wacom.ap.minimum,
                        self.wacom.ap.maximum,
                        self.wacom.atx.minimum,
                        self.wacom.atx.maximum
                    );
                    continue;
                }
            }
            if pen && self.pen.fd >= 0 && scaled {
                // A scaled pen found after a raw one: the raw one becomes the pressure source.
                self.key_none_note(fd, &path, &name_s, &keyb, &absb);
                close_fd(fd);
                continue;
            }
            if pen && self.pen.fd < 0 {
                self.pen.fd = fd;
                self.pen_mono = mono;
                eviocgabs(fd, ABS_X, &mut self.pen.ax);
                eviocgabs(fd, ABS_Y, &mut self.pen.ay);
                eviocgabs(fd, ABS_PRESSURE, &mut self.pen.ap);
                if test_bit(&absb, ABS_MT_PRESSURE as usize) {
                    eviocgabs(fd, ABS_MT_PRESSURE, &mut self.pen.amp);
                }
                self.caps.has_tilt = test_bit(&absb, ABS_TILT_X as usize) && test_bit(&absb, ABS_TILT_Y as usize);
                if self.caps.has_tilt {
                    eviocgabs(fd, ABS_TILT_X, &mut self.pen.atx);
                    eviocgabs(fd, ABS_TILT_Y, &mut self.pen.aty);
                }
                self.pen.has_touch_btn = test_bit(&keyb, BTN_TOUCH as usize);
                if ioctl_grab(fd, 1) != 0 {
                    self.initial_grab_failed = true;
                    qlog!("input: could not grab the pen: %s", &strerror(errno()));
                }
                self.caps.has_pen = true;
                self.caps.has_pressure = self.pen.ap.maximum > self.pen.ap.minimum;
                self.caps.pen_name.set_bytes(cbytes(&name));
                qlog!(
                    "input: pen %s (%s) x %d..%d y %d..%d p %d..%d tilt %s %d..%d res %d",
                    &path,
                    &name_s,
                    self.pen.ax.minimum,
                    self.pen.ax.maximum,
                    self.pen.ay.minimum,
                    self.pen.ay.maximum,
                    self.pen.ap.minimum,
                    self.pen.ap.maximum,
                    if self.caps.has_tilt { "yes" } else { "no" },
                    self.pen.atx.minimum,
                    self.pen.atx.maximum,
                    self.pen.atx.resolution
                );
            } else if touch && self.touch.fd < 0 {
                self.touch.fd = fd;
                self.touch_mono = mono;
                eviocgabs(fd, ABS_MT_POSITION_X, &mut self.touch.ax);
                eviocgabs(fd, ABS_MT_POSITION_Y, &mut self.touch.ay);
                if ioctl_grab(fd, 1) != 0 {
                    self.initial_grab_failed = true;
                    qlog!("input: could not grab touch: %s", &strerror(errno()));
                }
                self.caps.has_touch = true;
                self.caps.touch_name.set_bytes(cbytes(&name));
                let mut slots = InputAbsinfo::default();
                if eviocgabs(fd, ABS_MT_SLOT, &mut slots) >= 0 {
                    self.caps.touch_slots = slots.maximum.wrapping_add(1);
                }
                qlog!(
                    "input: touch %s (%s) x %d..%d y %d..%d",
                    &path,
                    &name_s,
                    self.touch.ax.minimum,
                    self.touch.ax.maximum,
                    self.touch.ay.minimum,
                    self.touch.ay.maximum
                );
                // How many contacts it follows and how it sizes them (touchgest's thresholds come
                // from the owner's traces).
                let mut major = InputAbsinfo::default();
                eviocgabs(fd, ABS_MT_TOUCH_MAJOR, &mut major);
                qlog!(
                    "input: touch slots %d..%d, touch_major %d..%d r%d",
                    slots.minimum,
                    slots.maximum,
                    major.minimum,
                    major.maximum,
                    major.resolution
                );
            } else if power >= 0 && (self.power_fd < 0 || power < power_rank) {
                if self.power_fd >= 0 {
                    qlog!("input: %s is another power key with more keys, not watched", &power_path);
                    close_fd(self.power_fd);
                }
                self.power_fd = fd;
                power_rank = power;
                self.caps.has_power = true;
                power_keys += 1;
                strlcpy_q(&mut power_path, &path, 300);
                // Its events in now_ms()'s clock (switched as it was opened), to time a hold exactly.
                self.power_mono = mono;
                qlog!(
                    "input: power key %s (%s)%s",
                    &path,
                    &name_s,
                    if self.power_mono { "" } else { ", timed when read (EVIOCSCLOCKID failed)" }
                );
            } else {
                if power >= 0 {
                    power_keys += 1;
                    qlog!("input: %s (%s) is another power key, not watched", &path, &name_s);
                }
                self.key_none_note(fd, &path, &name_s, &keyb, &absb);
                close_fd(fd);
            }
        }
        // One power key is watched, the one with the fewest keys (the first found of those): if another
        // device is the one that reports presses, Grimoire never sees the button. (The pen, the touch
        // panel, keyboards and Amazon's every-key proxy are never taken for one, so a KEY_POWER they
        // advertise does not count here: hidkeys_power, pinned by tests/test_hidkeys.c.)
        if power_keys > 1 {
            qlog!("input: WARNING %d devices report a power key; watching only %s", power_keys, &power_path);
        }
        if self.power_fd < 0 {
            qlog!("input: WARNING no power key found: no fallbacks, and no 4 s hold to leave");
        }
        if !self.caps.has_orientation {
            qlog!("input: no orientation sensor found");
        }
        if self.pen.fd < 0 && self.wacom.fd >= 0 {
            // Only a raw digitizer: use it for everything (calibration maps its axes).
            self.pen = self.wacom;
            self.pen_mono = self.wacom_mono;
            self.wacom.fd = -1;
            eviocgabs(self.pen.fd, ABS_X, &mut self.pen.ax);
            eviocgabs(self.pen.fd, ABS_Y, &mut self.pen.ay);
            if ioctl_grab(self.pen.fd, 1) != 0 {
                self.initial_grab_failed = true;
            }
            self.caps.has_pen = true;
            self.caps.has_pressure = true;
            self.caps.has_tilt = self.pen.atx.maximum > self.pen.atx.minimum;
            qlog!("input: using the raw digitizer as the pen");
        }
        if self.pen.fd >= 0 && !self.pen_mono {
            qlog!("input: the pen's EVIOCSCLOCKID failed: stroke samples are timed when read");
        }
        // Ink timing (latency.h, with /mnt/us/grimoire/ink-metrics): the pens' events stamped in now_ms()'s
        // clock (set as they were opened). The clock is per reader, so the pen service and the Kindle's
        // other readers keep theirs.
        if self.pen.fd >= 0 && access_ok(c"/mnt/us/grimoire/ink-metrics") {
            self.ink_mono = self.pen_mono && (self.wacom.fd < 0 || self.wacom_mono);
            if !self.ink_mono {
                qlog!("input: EVIOCSCLOCKID failed (no ink timing)");
            }
            let rmin = [self.wacom.ax.minimum, self.wacom.ay.minimum];
            let rmax = [self.wacom.ax.maximum, self.wacom.ay.maximum];
            let smin = [self.pen.ax.minimum, self.pen.ay.minimum];
            let smax = [self.pen.ax.maximum, self.pen.ay.maximum];
            lat_match_init(&mut self.match_, &rmin, &rmax, &smin, &smax);
        }
        if self.wacom.fd >= 0 {
            self.caps.has_pressure = self.wacom.ap.maximum > self.wacom.ap.minimum;
            self.caps.has_tilt = self.wacom.atx.maximum > self.wacom.atx.minimum;
        }
        if self.pen.fd >= 0 && self.wacom.fd >= 0 {
            // (With only a raw digitizer it is the pen, and its own hover frames say where it hovers.)
            let rmin = [self.wacom.ax.minimum, self.wacom.ay.minimum];
            let rmax = [self.wacom.ax.maximum, self.wacom.ay.maximum];
            let smin = [self.pen.ax.minimum, self.pen.ay.minimum];
            let smax = [self.pen.ax.maximum, self.pen.ay.maximum];
            pensync_hover_init(&mut self.hover, &rmin, &rmax, &smin, &smax);
        }
        if self.pen.fd < 0 {
            qlog!("input: WARNING no pen digitizer found");
        }
        if self.touch.fd < 0 {
            qlog!("input: WARNING no touch panel found");
        }
        (self.pen_cal, self.touch_cal) = self.default_calib();
        if let Some(c) = pen {
            if c.valid {
                self.pen_cal = *c;
            }
        }
        if let Some(c) = touch {
            if c.valid {
                self.touch_cal = *c;
            }
        }
        // The pens' read gaps start here: a drop at the first read says how long the start took.
        self.pen_read_ms = now_ms();
        self.wacom_read_ms = self.pen_read_ms;
        0
    }

    // Side button presses/releases, from whichever pen device reports them.
    fn side_check(&mut self) {
        let side = self.P.side || self.W.side;
        if side == self.side_reported {
            return;
        }
        self.side_reported = side;
        let mut b = InEvent { type_: InType::PenButton, side_button: side, ..Default::default() };
        b.nx = norm(self.P.x, &self.pen.ax);
        b.ny = norm(self.P.y, &self.pen.ay);
        (b.x, b.y) = self.map(&self.pen_cal, b.nx, b.ny);
        b.t_ms = now_ms();
        qlog!("input: pen button %s", if side { "down" } else { "up" });
        self.q_push(b);
    }
}

fn drain(fd: i32, f: Handler) {
    let mut evs = [InputEvent::default(); 64];
    loop {
        let r = read_events(fd, &mut evs);
        if r <= 0 {
            break;
        }
        for ev in &evs[..r as usize / size_of::<InputEvent>()] {
            // (Each event under its own borrow of the state: a raw frame's hover hook runs after it.)
            match with(|s| f(s, ev)) {
                Some(HoverTo::Place(hook, x, y)) => hook(x, y),
                Some(HoverTo::Sample(hook, mut e)) => hook(&mut e),
                None => {}
            }
        }
        if (r as usize) < size_of_val(&evs) {
            break;
        }
    }
}

fn orient_code_to_turns(v: i32) -> i32 {
    // KOReader's KindleGyroTransform (kx132.h / bma4xy): 15 portrait up, 16 portrait down,
    // 17 landscape left (device turned clockwise), 18 landscape right (counter-clockwise).
    match v {
        15 => 0,
        17 => 1,
        16 => 2,
        18 => 3,
        _ => -1, // face up/down and other codes: keep the current orientation
    }
}

// The watcher shell. `-s 0` waits with no timeout (KOReader and Amazon's own scripts always pass
// -s). A run that fails prints a marker line (handle_powerd_fd logs it) and the loop tries again
// 2 s later. lipc-wait-event runs in the background so that the TERM trap can stop it: the shell
// gets TERM when Grimoire goes, even on a crash (PR_SET_PDEATHSIG), and takes its child along.
// No more event names in the one-shot loop: each one is one more restart around suspend and
// resume, and an outOfScreenSaver could fall into a restart's gap.
macro_rules! WATCH_EVENTS {
    () => {
        "com.lab126.powerd goingToScreenSaver,outOfScreenSaver"
    };
}
macro_rules! WATCH_FAILED {
    () => {
        "grimoire-watcher-failed"
    };
}
macro_rules! WATCH_TRAP {
    () => {
        "trap 'kill $w 2>/dev/null; exit 0' TERM; "
    };
}
const WATCH_FAILED: &str = WATCH_FAILED!();
const fn cstr(s: &str) -> &CStr {
    match CStr::from_bytes_with_nul(s.as_bytes()) {
        Ok(c) => c,
        Err(_) => panic!("a C string ends with its one NUL"),
    }
}
const K_WATCH_ONE_SHOT: &CStr = cstr(concat!(
    WATCH_TRAP!(),
    "while :; do lipc-wait-event -s 0 ",
    WATCH_EVENTS!(),
    " 2>&1 & w=$!; ",
    "wait $w || { echo \"",
    WATCH_FAILED!(),
    " $?\"; sleep 2; }; done\0"
));
// /mnt/us/grimoire/watcher-multi: one continuous run for every event, KOReader's way (no gap
// between runs), for a device test of whether -m output reaches Grimoire through the pipe.
const K_WATCH_MULTI: &CStr = cstr(concat!(
    WATCH_TRAP!(),
    "while :; do lipc-wait-event -m -s 0 ",
    WATCH_EVENTS!(),
    " 2>&1 & w=$!; ",
    "wait $w; echo \"",
    WATCH_FAILED!(),
    " $?\"; sleep 2; done\0"
));

fn powerd_start() {
    let multi = access_ok(c"/mnt/us/grimoire/watcher-multi");
    let script = if multi { K_WATCH_MULTI } else { K_WATCH_ONE_SHOT };
    let mut p: [c_int; 2] = [0; 2];
    // SAFETY: pipe writes the two descriptors into our array.
    if unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
        return;
    }
    // SAFETY: fork, as PORTING.md's process model has it: the child below only sets its group and
    // death signal, moves descriptors and execs (or _exits) with strings made before the fork; no
    // borrow of this module's state is held across it.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        close_fd(p[0]);
        close_fd(p[1]);
        return;
    }
    if pid == 0 {
        // SAFETY: in the child: setpgid, prctl, dup2 and close on its own descriptors, then execlp
        // with NUL-terminated strings and a NULL-terminated argument list; _exit ends the child
        // without the parent's exit handlers if the exec fails.
        unsafe {
            libc::setpgid(0, 0); // its own group, so the loop and its child can be stopped together
            #[cfg(target_os = "linux")]
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as c_ulong); // and it goes when Grimoire does, even on a crash
            libc::dup2(p[1], 1);
            libc::close(p[0]);
            libc::close(p[1]);
            libc::execlp(c"sh".as_ptr(), c"sh".as_ptr(), c"-c".as_ptr(), script.as_ptr(), std::ptr::null::<libc::c_char>(),
            );
            libc::_exit(127);
        }
    }
    close_fd(p[1]);
    // SAFETY: fcntl on the read end we own.
    unsafe { libc::fcntl(p[0], libc::F_SETFL, libc::O_NONBLOCK) };
    with(|s| {
        s.powerd_fd = p[0];
        s.powerd_pid = pid;
    });
    qlog!(
        "input: watching the power service (pid %d): %s",
        pid,
        if multi { "one continuous lipc-wait-event -m (the watcher-multi switch)" } else { "one-shot lipc-wait-event runs" }
    );
}

// Whether the power key is down right now, from the device itself: 1, 0, or -1 if it cannot tell.
fn power_key_down(power_fd: i32) -> i32 {
    let mut keys = [0u8; KEYB_BYTES];
    if power_fd < 0 || eviocgkey(power_fd, &mut keys) < 0 {
        return -1;
    }
    if test_bit(&keys, KEY_POWER as usize) {
        1
    } else {
        0
    }
}

// ---- setup ----

extern "C" fn on_signal(sig: c_int) {
    let _ = sig;
    G_QUIT.store(1, Ordering::SeqCst);
    let w = G_SIGPIPE[1].load(Ordering::SeqCst);
    if w >= 0 {
        let c: u8 = 1;
        // SAFETY: write is async-signal-safe; it reads the one byte of our local.
        let _ = unsafe { libc::write(w, &c as *const u8 as *const c_void, 1) };
    }
}

// (dev_t and ino_t are 64-bit on the Kindle's musl, as the C casts them; not on every target.)
#[allow(clippy::unnecessary_cast)]
fn dev_of(st: &libc::stat) -> u64 {
    st.st_rdev as u64
}
#[allow(clippy::unnecessary_cast)]
fn node_of(st: &libc::stat) -> HidNode {
    HidNode { rdev: st.st_rdev as u64, ino: st.st_ino as u64, ctime_s: st.st_ctime as i64, ctime_ns: st.st_ctime_nsec as i64 ,
    }
}

// Stamps fd's events in now_ms()'s clock, for this reader only (the pen service and the Kindle's
// other readers keep theirs); false if the driver refuses. Called right after open(), before any
// other ioctl: switched once an event is queued (a hovering pen sends one every 3 ms), the reader's
// queue is thrown away and a SYN_DROPPED queued (Linux 4.9's evdev_set_clk_type), a drop at the
// first read that no stall caused.
fn clock_mono(fd: i32) -> bool {
    let clk: c_int = libc::CLOCK_MONOTONIC;
    // SAFETY: EVIOCSCLOCKID reads one int, our local.
    unsafe { libc::ioctl(fd, EVIOCSCLOCKID as libc::Ioctl, &clk as *const c_int) == 0 }
}

impl State {
    // Live view's raw digitizer map (input_raw_pen): three raw places inside its ranges (so no cut
    // bends the line) taken through what raw_hover and hover_from_raw do to a frame, unrounded, give
    // the affine map; the tilt's turn into the screen's frame comes from calib_tilt the same way.
    fn raw_pen_map(&self, out: &mut LiveRawMap) -> bool {
        // (Only once contact frames have decided how the raw axes lie on the scaled pen's: before that
        // an axis taken as it is might be reversed, and the ring's samples, the scaled pen's own, stay.)
        if self.wacom.fd < 0
            || !self.wacom_mono
            || !self.pen_mono // (the main loop's events then take no kernel time: no identities in common)
            || !self.input_grabbed
            || self.hover.n < PENSYNC_HOVER_FRAMES as u32
            || pensync_hover_orientation(&self.hover) < 0
        {
            return false;
        }
        let h = &self.hover;
        let place = |rx: i32, ry: i32| -> Option<(f32, f32)> {
            let (mut sx, mut sy) = (0.0f32, 0.0f32);
            if !pensync_hover_place(h, rx, ry, &mut sx, &mut sy) {
                return None;
            }
            let normf = |v: f32, a: &InputAbsinfo| {
                let span = a.maximum.wrapping_sub(a.minimum);
                if span > 0 { (v - a.minimum as f32) / span as f32 } else { 0.0 }
            };
            Some(self.map(&self.pen_cal, normf(sx, &self.pen.ax), normf(sy, &self.pen.ay),
            ))
        };
        let (x0, x1) = (h.rmin[0] + (h.rmax[0] - h.rmin[0]) / 4, h.rmin[0] + (h.rmax[0] - h.rmin[0]) * 3 / 4,
        );
        let (y0, y1) = (h.rmin[1] + (h.rmax[1] - h.rmin[1]) / 4, h.rmin[1] + (h.rmax[1] - h.rmin[1]) * 3 / 4,
        );
        if x1 <= x0 || y1 <= y0 {
            return false;
        }
        let (Some(p0), Some(px), Some(py)) = (place(x0, y0), place(x1, y0), place(x0, y1)) else { return false ;
        };
        let (dx, dy) = ((x1 - x0) as f32, (y1 - y0) as f32);
        let (ax, ay) = ((px.0 - p0.0) / dx, (px.1 - p0.1) / dx);
        let (bx, by) = ((py.0 - p0.0) / dy, (py.1 - p0.1) / dy);
        *out = LiveRawMap {
            m: [ax, bx, p0.0 - ax * x0 as f32 - bx * y0 as f32, ay, by, p0.1 - ay * x0 as f32 - by * y0 as f32,
            ],
            rmin: h.rmin,
            rmax: h.rmax,
            pmin: self.wacom.ap.minimum,
            pmax: self.wacom.ap.maximum,
            tilt_min: [self.wacom.atx.minimum, self.wacom.aty.minimum],
            tilt_max: [self.wacom.atx.maximum, self.wacom.aty.maximum],
            tilt_res: [self.wacom.atx.resolution, self.wacom.aty.resolution],
            ..Default::default()
        };
        let (mut t1, mut t2) = (1.0f32, 0.0f32);
        calib_tilt(&self.pen_cal, self.rot, self.tilt_flip, &mut t1, &mut t2);
        let (mut t3, mut t4) = (0.0f32, 1.0f32);
        calib_tilt(&self.pen_cal, self.rot, self.tilt_flip, &mut t3, &mut t4);
        out.tm = [t1, t3, t2, t4];
        out.dev = fstat_fd(self.wacom.fd).map_or(0, |st| st.st_rdev as u64);
        // The node's path, from the descriptor (/proc names what it was opened as).
        let mut path = [0u8; 128];
        let link = cformat!("/proc/self/fd/%d", self.wacom.fd);
        let Ok(c) = CString::new(link) else { return false ;
        };
        // SAFETY: readlink writes at most path.len() - 1 bytes into the local array.
        let n = unsafe { libc::readlink(c.as_ptr(), path.as_mut_ptr() as *mut libc::c_char, path.len() - 1,
            ) };
        if n <= 0 {
            return false;
        }
        out.path.set_bytes(&path[..n as usize]);
        true
    }
}

// ---- input.h (input.rs forwards here on the Kindle) ----

pub fn input_raw_pen(out: &mut LiveRawMap) -> bool {
    with(|s| s.raw_pen_map(out))
}

pub fn input_open(screen_w: i32, screen_h: i32, dpi: f32, pen: Option<&InputCalib>, touch: Option<&InputCalib>,
) -> i32 {
    if with(|s| s.open_devices(screen_w, screen_h, pen, touch)) != 0 {
        return -1;
    }
    with(|s| {
        touchgest_init(&mut s.tg, dpi);
        touchgest_set_slots(&mut s.tg, s.caps.touch_slots.max(0) as usize);
        s.dpi = dpi;
    });

    powerd_start();
    let mut p: [c_int; 2] = [-1; 2];
    // SAFETY: pipe writes the two descriptors into our array.
    if unsafe { libc::pipe(p.as_mut_ptr()) } == 0 {
        // SAFETY: as above.
        unsafe {
            libc::fcntl(p[0], libc::F_SETFL, libc::O_NONBLOCK);
            libc::fcntl(p[1], libc::F_SETFL, libc::O_NONBLOCK);
        }
        G_SIGPIPE[0].store(p[0], Ordering::SeqCst);
        G_SIGPIPE[1].store(p[1], Ordering::SeqCst);
    }
    // SAFETY: sigaction with a zeroed struct (no flags, an empty mask) and on_signal, an extern "C"
    // handler that only stores to atomics and writes one byte (async-signal-safe).
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = on_signal as extern "C" fn(c_int) as libc::sighandler_t;
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
    }
    let ok = with(|s| {
        if s.initial_grab_failed {
            qlog!("input: another app holds an input device; refusing to open an unresponsive UI");
        }
        let ok = !s.initial_grab_failed && (s.pen.fd >= 0 || s.touch.fd >= 0);
        s.pens_read_ms = now_ms();
        ok
    });
    if ok {
        work_pump_set(Some(input_pump));
    }
    if ok {
        0
    } else {
        -1
    }
}
pub fn input_set_calib(pen: Option<&InputCalib>, touch: Option<&InputCalib>) {
    with(|s| {
        if let Some(c) = pen {
            s.pen_cal = *c;
        }
        if let Some(c) = touch {
            s.touch_cal = *c;
        }
    })
}
pub fn input_default_calib(pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
    let (p, t) = with(|s| s.default_calib());
    if let Some(pen) = pen {
        *pen = p;
    }
    if let Some(touch) = touch {
        *touch = t;
    }
}
pub fn input_caps() -> InputCaps {
    with(|s| s.caps)
}

// Reads what the two pens hold, the raw digitizer first so a pen frame finds its sample: wacom and
// pen say which a poll found readable. input_poll's step, and input_pump's.
fn read_pens(wacom: bool, pen: bool) {
    if wacom {
        let (fd, grabbed) = with(|s| {
            let now = now_ms();
            s.wacom_read_gap = now.wrapping_sub(s.wacom_read_ms);
            s.wacom_read_ms = now;
            s.wacom_first_read = !s.wacom_read;
            s.wacom_read = true;
            (s.wacom.fd, s.input_grabbed)
        });
        drain(fd, if grabbed { State::handle_wacom } else { discard_input },
        );
        with(|s| {
            if s.wacom_resync {
                s.wacom_resync();
            }
        });
    }
    if pen {
        let (fd, grabbed) = with(|s| (s.pen.fd, s.input_grabbed));
        if grabbed {
            with(|s| s.drain_pen());
        } else {
            drain(fd, discard_input);
        }
    }
    if let Some(batch) = with(|s| s.sample_batch) {
        batch();
    }
}

// The work pump (common.h), set by input_open: long work on the main thread calls it every few
// milliseconds, and at most every PUMP_MS it reads the two pens as input_poll does, without waiting,
// then the touch panel. The raw digitizer's buffer overflows after about 100 ms of hover (its frames
// are 5-7 events), the scaled pen's after 120-175 ms of writing, and a page's render took 1.2 s; the
// touch panel's would lose a finger's lift. What it reads goes into the queue behind what is there,
// in order, for the main loop. Keys, power and orientation wait for input_poll: they keep their own
// order, and nothing of theirs is lost meanwhile.
const PUMP_MS: u32 = 8;
fn input_pump() {
    let now = now_ms();
    let fds = with(|s| {
        if s.pump_in || now.wrapping_sub(s.pens_read_ms) < PUMP_MS {
            return None;
        }
        s.pump_in = true;
        s.pens_read_ms = now;
        Some((s.wacom.fd, s.pen.fd, s.touch.fd, s.input_grabbed))
    });
    let Some((wacom_fd, pen_fd, touch_fd, grabbed)) = fds else { return ;
    };
    let mut pf = [pollfd(-1); 3];
    let mut n = 0usize;
    let mut iwac: i32 = -1;
    let mut ipen: i32 = -1;
    let mut itouch: i32 = -1;
    if wacom_fd >= 0 {
        iwac = n as i32;
        pf[n] = pollfd(wacom_fd);
        n += 1;
    }
    if pen_fd >= 0 {
        ipen = n as i32;
        pf[n] = pollfd(pen_fd);
        n += 1;
    }
    if touch_fd >= 0 {
        itouch = n as i32;
        pf[n] = pollfd(touch_fd);
        n += 1;
    }
    if n != 0 && poll_fds(&mut pf[..n], 0) > 0 {
        read_pens(
            iwac >= 0 && (pf[iwac as usize].revents & libc::POLLIN) != 0,
            ipen >= 0 && (pf[ipen as usize].revents & libc::POLLIN) != 0,
        );
        // Touch after the pens, as input_poll reads them.
        if itouch >= 0 && (pf[itouch as usize].revents & libc::POLLIN) != 0 {
            drain(touch_fd, if grabbed { State::handle_touch } else { discard_input },
            );
        }
    }
    with(|s| {
        s.side_check();
        s.pump_in = false;
    });
}

pub fn input_poll(ev: &mut InEvent, timeout_ms: i32) -> i32 {
    if G_QUIT.load(Ordering::SeqCst) != 0 {
        *ev = InEvent { type_: InType::Quit, t_ms: now_ms(), ..Default::default() };
        G_QUIT.store(0, Ordering::SeqCst);
        return 1;
    }
    // A queue the main loop takes its time over (each batch runs at most 8 ms, but one event can take
    // longer) still has the pens read every PUMP_MS.
    input_pump();
    if with(|s| s.q_pop(ev)) {
        return 1;
    }
    // Never while the pen or a finger is in use: nothing reads the pen while the scan runs.
    let touch_events = with(|s| s.touch_events);
    let (fds, key_fds) = with(|s| {
        let busy = s.P.prox || s.P.touching || touchgest_busy(&s.tg);
        if s.keys_enabled && hidkeys_scan_due(now_ms(), s.keys_scan, busy, s.pen_left_ms) {
            s.key_scan(true);
        }
        let mut key_fds = [-1i32; KEY_DEVS];
        for (i, k) in s.keys.iter().enumerate() {
            if s.keys_enabled && k.fd >= 0 {
                key_fds[i] = k.fd;
            }
        }
        ([s.powerd_fd, s.orient_fd, s.wacom.fd, s.pen.fd, s.touch.fd, s.power_fd,
            ], key_fds,
        )
    });
    let [powerd_fd, orient_fd, wacom_fd, pen_fd, touch_fd, power_fd] = fds;
    let sig_fd = G_SIGPIPE[0].load(Ordering::SeqCst);
    let watch = with(|s| s.watch);
    let mut pf = [pollfd(-1); 8 + KEY_DEVS];
    let mut ikey = [-1i32; KEY_DEVS];
    let mut n = 0usize;
    let mut add = |fd: i32| -> i32 {
        pf[n] = pollfd(fd);
        n += 1;
        n as i32 - 1
    };
    let ipwd = if powerd_fd >= 0 { add(powerd_fd) } else { -1 };
    let iori = if orient_fd >= 0 { add(orient_fd) } else { -1 };
    let iwac = if wacom_fd >= 0 { add(wacom_fd) } else { -1 };
    let ipen = if pen_fd >= 0 { add(pen_fd) } else { -1 };
    let itouch = if touch_fd >= 0 { add(touch_fd) } else { -1 };
    let ipow = if power_fd >= 0 { add(power_fd) } else { -1 };
    let isig = if sig_fd >= 0 { add(sig_fd) } else { -1 };
    if watch >= 0 {
        add(watch);
    }
    for i in 0..KEY_DEVS {
        if key_fds[i] >= 0 {
            ikey[i] = add(key_fds[i]);
        }
    }
    let r = poll_fds(&mut pf[..n], timeout_ms);
    with(|s| s.pens_read_ms = now_ms());
    let readable = |i: i32, mask: libc::c_short| i >= 0 && (pf[i as usize].revents & mask) != 0;
    if r > 0 {
        read_pens(readable(iwac, libc::POLLIN), readable(ipen, libc::POLLIN));
        if readable(itouch, libc::POLLIN) {
            let grabbed = with(|s| s.input_grabbed);
            drain(touch_fd, if grabbed { State::handle_touch } else { discard_input },
            );
        }
    }
    // Fingers held still give no frames: what only time decides (a tap's last lift, a lone finger's
    // wait for its partner) is decided here, by touchgest's tick.
    with(|s| {
        if s.touch_events == touch_events && s.input_grabbed && touchgest_busy(&s.tg) {
            let mut out = TgOut::default();
            touchgest_tick(&mut s.tg, now_ms(), &mut out);
            s.tg_push(&out);
        }
    });
    if r > 0 {
        if readable(ipow, libc::POLLIN) {
            with(|s| s.handle_power_fd());
        }
        if readable(iori, libc::POLLIN) {
            with(|s| s.handle_orient_fd());
        }
        if readable(ipwd, libc::POLLIN | libc::POLLHUP) {
            with(|s| s.handle_powerd_fd());
        }
        // A simultaneous pen-down or native sleep event takes ownership before external keys.
        for i in 0..KEY_DEVS {
            if ikey[i] >= 0 {
                let re = pf[ikey[i] as usize].revents;
                if (re & libc::POLLIN) != 0 {
                    let fd = with(|s| {
                        s.key_read = i;
                        s.keys[i].fd
                    });
                    drain(fd, State::handle_key);
                }
                with(|s| {
                    if s.keys[i].fd >= 0 && (re & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL)) != 0 {
                        s.key_detach(i);
                    }
                });
            }
        }
        if readable(isig, libc::POLLIN) {
            let mut buf = [0u8; 16];
            // SAFETY: read writes at most 16 bytes into our local buffer.
            while unsafe { libc::read(sig_fd, buf.as_mut_ptr() as *mut c_void, buf.len()) } > 0 {}
        }
    }
    with(|s| s.power_hold_check());
    if G_QUIT.load(Ordering::SeqCst) != 0 {
        *ev = InEvent { type_: InType::Quit, t_ms: now_ms(), ..Default::default() };
        G_QUIT.store(0, Ordering::SeqCst);
        return 1;
    }
    with(|s| {
        s.side_check();
        if s.q_pop(ev) {
            1
        } else {
            0
        }
    })
}

fn discard_input(s: &mut State, ev: &InputEvent) -> Hover {
    let _ = (s, ev);
    None
}

pub fn input_watch_fd(fd: i32) {
    with(|s| s.watch = fd)
}
pub fn input_shortcuts_enable(enabled: bool) {
    with(|s| s.shortcuts_enable(enabled))
}
pub fn input_hids(out: &mut [InputHid]) -> i32 {
    with(|s| s.hids(out))
}
pub fn input_set_orientation(quarter_turns: i32) {
    with(|s| {
        if s.rot != quarter_turns & 3 && s.trace.fd >= 0 {
            s.rot = quarter_turns & 3;
            s.trace_header();
        }
        s.rot = quarter_turns & 3;
    })
}
#[allow(non_snake_case)]
pub fn input_logical_to_panel(lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
    let (w, h, rot) = with(|s| (s.w, s.h, s.rot));
    let W = (w - 1) as f32;
    let H = (h - 1) as f32;
    match rot & 3 {
        1 => {
            *px = ly;
            *py = H - lx;
        }
        2 => {
            *px = W - lx;
            *py = H - ly;
        }
        3 => {
            *px = W - ly;
            *py = lx;
        }
        _ => {
            *px = lx;
            *py = ly;
        }
    }
}
pub fn input_set_grab(grab: bool) -> bool {
    for i in 0..KEY_DEVS {
        let (init, fd) = with(|s| (s.keys_init, s.keys[i].fd));
        if !init {
            break;
        }
        if fd >= 0 {
            drain(fd, discard_input);
            with(|s| {
                s.key_ups(i, None, 0, 0); // (what was held is let go: a hold ends)
                if grab && !s.key_resync(i) {
                    s.key_detach(i);
                }
            });
        }
    }
    // Forget touches that were used to type the PIN, including the final finger lift.
    if grab {
        let (pen_fd, touch_fd, wacom_fd) = with(|s| (s.pen.fd, s.touch.fd, s.wacom.fd));
        if pen_fd >= 0 {
            drain(pen_fd, discard_input);
        }
        if touch_fd >= 0 {
            drain(touch_fd, discard_input);
        }
        if wacom_fd >= 0 {
            drain(wacom_fd, discard_input);
        }
    }
    with(|s| {
        // What the fingers were doing ends here, while their events still reach the app: a finger's
        // stream with the off-screen cancel, a pinch with PinchEnd.
        let mut out = TgOut::default();
        touchgest_cancel(&mut s.tg, now_ms(), &mut out);
        s.tg_push(&out);
        let mut ok = true;
        let flag = if grab { 1 } else { 0 };
        if s.pen.fd >= 0 && ioctl_grab(s.pen.fd, flag) != 0 {
            ok = false;
        }
        if s.touch.fd >= 0 && ioctl_grab(s.touch.fd, flag) != 0 {
            ok = false;
        }
        if !ok && grab {
            if s.pen.fd >= 0 {
                ioctl_grab(s.pen.fd, 0);
            }
            if s.touch.fd >= 0 {
                ioctl_grab(s.touch.fd, 0);
            }
        }
        s.input_grabbed = grab && ok;
        s.tg_hover = None;
        s.P = PenSt::default();
        s.pen_good = PenState::default();
        s.W = WacomSt::default();
        pensync_raw_clear(&mut s.raw);
        s.T = TouchSt::default();
        s.side_reported = false;
        s.pen_resume_block = false;
        if s.input_grabbed {
            if s.pen.fd >= 0 {
                s.pen_read_state();
                s.pen_good = s.pen_state();
                s.pen_resume_block = s.P.touching;
                s.side_reported = s.P.side;
            }
            if s.touch.fd >= 0 {
                // The fingers still down from the PIN wait to lift (the frame after the cancel).
                s.touch_resync();
                s.touch_frame();
            }
        } else {
            // Not fed while the Kindle has the panel: what touchgest held goes now, so nothing
            // counts as down meanwhile (the key scan's busy flag).
            let mut out = TgOut::default();
            touchgest_frame(&mut s.tg, &TgFrame::default(), &TgPen::default(), &mut out);
            s.tg_push(&out);
        }
        qlog!("input: native lock: %s %s", if grab { "grab" } else { "release" }, if ok { "ok" } else { "failed" });
        ok
    })
}
pub fn input_hover_hook(f: Option<fn(f32, f32)>) {
    with(|s| s.hover_fn = f)
}
pub fn input_sample_hook(sample: Option<fn(&mut InEvent)>, batch: Option<fn()>) {
    with(|s| {
        s.sample_fn = sample;
        s.sample_batch = batch;
    })
}
pub fn input_touch_context(v: TgView) {
    with(|s| touchgest_set_view(&mut s.tg, v))
}
pub fn input_touch_trace(path: Option<&str>) {
    with(|s| {
        s.trace_flush(now_ms());
        if s.trace.fd >= 0 {
            close_fd(s.trace.fd);
            s.trace.fd = -1;
        }
        let Some(path) = path else { return };
        let Ok(c) = CString::new(path) else { return };
        // SAFETY: open reads the NUL-terminated path.
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND | libc::O_CLOEXEC, 0o644 as libc::c_uint,
            ) };
        if fd < 0 {
            qlog!("input: touch trace %s: %s", path, &strerror(errno()));
            return;
        }
        let written = fstat_fd(fd).map_or(0, |st| st.st_size as u64);
        if written >= TRACE_MAX {
            close_fd(fd);
            qlog!("input: touch trace %s is full (4 MB): not written", path);
            return;
        }
        s.trace = TouchTrace { fd, buf: String::with_capacity(TRACE_ROOM), written ,
        };
        s.trace_header();
        s.trace_flush(now_ms());
        qlog!("input: touch trace in %s", path);
    })
}
pub fn input_forget_signal_pipe() {
    for p in &G_SIGPIPE {
        p.store(-1, Ordering::SeqCst); // (not closed: the child closes every inherited fd itself)
    }
}
pub fn input_close() {
    input_touch_trace(None);
    work_pump_set(None);
    with(|s| s.hover_fn = None);
    input_sample_hook(None, None);
    input_shortcuts_enable(false);
    with(|s| {
        if s.pen.fd >= 0 {
            ioctl_grab(s.pen.fd, 0);
            close_fd(s.pen.fd);
        }
        if s.touch.fd >= 0 {
            ioctl_grab(s.touch.fd, 0);
            close_fd(s.touch.fd);
        }
        if s.power_fd >= 0 {
            close_fd(s.power_fd);
        }
        if s.wacom.fd >= 0 {
            close_fd(s.wacom.fd);
        }
        s.wacom.fd = -1;
        if s.orient_fd >= 0 {
            close_fd(s.orient_fd);
        }
        s.orient_fd = -1;
        if s.powerd_pid > 0 {
            // SAFETY: kill and waitpid on the watcher this module forked and has not reaped (its group
            // is its own: setpgid in powerd_start).
            unsafe {
                libc::kill(-s.powerd_pid, libc::SIGTERM); // the shell and its current lipc-wait-event
                libc::kill(s.powerd_pid, libc::SIGTERM);
                libc::waitpid(s.powerd_pid, std::ptr::null_mut(), 0);
            }
        }
        if s.powerd_fd >= 0 {
            close_fd(s.powerd_fd);
        }
        s.powerd_fd = -1;
        s.powerd_pid = -1;
        // (Each end is taken out of G_SIGPIPE before it is closed: a signal that comes late finds -1,
        // never a closed descriptor, or one that a later open got the number of.)
        for p in &G_SIGPIPE {
            let fd = p.swap(-1, Ordering::SeqCst);
            if fd >= 0 {
                close_fd(fd);
            }
        }
        s.pen.fd = -1;
        s.touch.fd = -1;
        s.power_fd = -1;
    })
}

// ---- latency.h ----

/// Kindle only (read by display_fbink.rs for each live ink frame): the kernel times of the newest
/// and the oldest pen sample emitted since the last call, and of the raw sample the newest came from
/// (0 when it matched none). False when no pen sample came since the last call, or the pen's events
/// are not in CLOCK_MONOTONIC.
pub fn input_ink_times(newest: &mut u64, oldest: &mut u64, raw: &mut u64) -> bool {
    with(|s| {
        if s.ink_newest == 0 {
            return false;
        }
        *newest = s.ink_newest;
        *oldest = s.ink_oldest;
        *raw = s.ink_raw;
        s.ink_newest = 0;
        s.ink_oldest = 0;
        s.ink_raw = 0;
        true
    })
}
/// Editor-side shape outcomes share the bounded, opt-in touch trace and its grabbed-input gate.
pub fn input_touch_trace_event(line: &str) {
    if line.len() > 256 || line.contains(['\n', '\r']) || !line.starts_with("shape ") { return; }
    with(|s| {
        if !s.input_grabbed || s.trace.fd < 0 { return; }
        let k = now_ms();
        if s.trace.buf.len() + line.len() + 24 > TRACE_ROOM { s.trace_flush(k); }
        use std::fmt::Write;
        let _ = writeln!(s.trace.buf, "E {k} {line}");
        s.trace_flush(k);
    });
}
