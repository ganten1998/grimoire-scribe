//! pensync.h: the pure parts of reading the pen (input_evdev.c), unit-tested on the host: how a
//! stroke goes on after the kernel dropped pen events, the kernel's sample times in now_ms()'s clock,
//! which of the raw digitizer's samples a pen frame takes its pressure and tilt from, and where the pen
//! hovers (from the raw digitizer: the scaled pen says only that it is near).
//!
//! Each reader of an evdev device has its own buffer in the kernel. Grimoire's for the scaled pen
//! (stylus-custom) holds 128 events, and its frames carry only X and Y (2 or 3 events each), so it
//! holds 42-63 frames: 120-175 ms of writing. When Grimoire's thread stalls longer than that, the
//! kernel throws away everything unread and queues SYN_DROPPED and the newest event. What the next
//! read finds is a partial frame (the rest of the frame that newest event belongs to), then every
//! frame that came after it, in order: the tail. Buttons and axes the tail does not carry stayed as
//! they were, and only the device can say what that is (EVIOCGKEY, EVIOCGABS: what it holds now).
//! EVIOCGKEY also takes the queued button events out of the reader's queue, so it is asked only
//! once the tail has been read.

use crate::prelude::*;

/// Linux evdev numbers (linux/input.h, which the host does not have; input_evdev.c checks them).
// (u16, as input_event's type and code are.)
pub const PS_EV_SYN: u16 = 0x00;
pub const PS_EV_KEY: u16 = 0x01;
pub const PS_EV_ABS: u16 = 0x03;
pub const PS_SYN_REPORT: u16 = 0;
pub const PS_SYN_DROPPED: u16 = 3;
pub const PS_ABS_X: u16 = 0x00;
pub const PS_ABS_Y: u16 = 0x01;
pub const PS_ABS_PRESSURE: u16 = 0x18;
pub const PS_ABS_TILT_X: u16 = 0x1a;
pub const PS_ABS_TILT_Y: u16 = 0x1b;
pub const PS_BTN_TOOL_PEN: u16 = 0x140;
pub const PS_BTN_TOOL_RUBBER: u16 = 0x141;
pub const PS_BTN_TOUCH: u16 = 0x14a;
pub const PS_BTN_STYLUS: u16 = 0x14b;

/// One event as read (input_event's fields; the time in microseconds of the reader's clock).
#[derive(Clone, Copy, Debug, Default)]
pub struct PenEv {
    pub t_us: u64,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

/// The pen's buttons and axes as its frames report them (ABS_MT_PRESSURE already scaled into p).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PenState {
    pub tool_pen: bool,
    pub tool_rubber: bool,
    pub touching: bool,
    pub side: bool,
    pub x: i32,
    pub y: i32,
    pub p: i32,
    pub tx: i32,
    pub ty: i32,
}

/// PenState's fields as bits: which ones a query of the device read.
pub const PS_F_TOOL_PEN: u32 = 1 << 0;
pub const PS_F_TOOL_RUBBER: u32 = 1 << 1;
pub const PS_F_TOUCH: u32 = 1 << 2;
pub const PS_F_SIDE: u32 = 1 << 3;
pub const PS_F_X: u32 = 1 << 4;
pub const PS_F_Y: u32 = 1 << 5;
pub const PS_F_P: u32 = 1 << 6;
pub const PS_F_TX: u32 = 1 << 7;
pub const PS_F_TY: u32 = 1 << 8;
pub const PS_F_KEYS: u32 = PS_F_TOOL_PEN | PS_F_TOOL_RUBBER | PS_F_TOUCH | PS_F_SIDE;
pub const PS_F_ALL: u32 = (1 << 9) - 1;

/// Whether the pen touches the screen: BTN_TOUCH, or for a pen without it, pressure above p_min
/// while it is near (BTN_TOOL_PEN or BTN_TOOL_RUBBER).
pub fn pensync_contact(s: &PenState, touch_btn: bool, p_min: i32) -> bool {
    todo!()
}

/// What a resync starts from. (ev: the C's ev and n.)
#[derive(Clone, Copy, Debug, Default)]
pub struct PenTail<'a> {
    /// the pen at the last complete frame read before the drop
    pub good: PenState,
    /// what was read from the SYN_DROPPED on (it included), in order
    pub ev: &'a [PenEv],
    /// the device's state, asked once all of ev was read
    pub now: PenState,
    /// PS_F_* of now that the device answered (0: the ioctls failed)
    pub now_ok: u32,
    /// pensync_contact's
    pub touch_btn: bool,
    pub p_min: i32,
}

/// A frame to report, as a SYN_REPORT would have it.
#[derive(Clone, Copy, Debug, Default)]
pub struct PenFrame {
    pub s: PenState,
    /// its SYN_REPORT's time
    pub t_us: u64,
    /// it carried an axis: a move (or hover) to report
    pub moved: bool,
    /// the first after the lost stretch: joined straight from the last good point
    pub gap: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PenTailInfo {
    /// complete frames in the tail
    pub frames: i32,
    /// events of the partial frame left out (its SYN_REPORT not counted)
    pub skipped: i32,
    /// ev[used..n) is an unfinished last frame, for the caller to read on as usual
    pub used: i32,
    /// the first and last tail frames' times (0 with no tail)
    pub first_us: u64,
    pub last_us: u64,
    /// the tail carried a button event
    pub keys: bool,
    /// the pen was found lifted: a lift at the last good point went first
    pub lifted: bool,
    /// where the reported frames end
    pub x: i32,
    pub y: i32,
}

/// The frames to report after a SYN_DROPPED, in order, into out (cap n + 3 is always enough); their
/// count. The tail's frames, each as it was: a button before its first event in the tail was the
/// other way (the input core passes only changes), an axis before its first event is taken at that
/// first value (the nearest known), and what the tail never carries is as the device says now (else
/// as it was). Never the device's present position ahead of older frames. The first frame is the
/// gap; if the pen touched at the last good point and the tail shows that stroke ended (no contact
/// at the first frame or just before it, or the other end by then), a lift at the last good point
/// goes before it, and the first frame starts a stroke of its own. A button that changed after the
/// tail was read (its event taken by EVIOCGKEY) comes last, at the last frame's place. With no frame
/// after the partial one, one frame of the device's present state.
pub fn pensync_tail(t: &PenTail, out: &mut [PenFrame], info: &mut PenTailInfo) -> i32 {
    todo!()
}

/// A sample's time for the stroke (InEvent.k_ms): the kernel's time of its frame in now_ms()'s
/// uint32 milliseconds, when the reader's clock is CLOCK_MONOTONIC (mono: EVIOCSCLOCKID took). Else,
/// or for a time of 0, one after now, or one over PENSYNC_K_MAX_AGE_MS before now (none of which a
/// real CLOCK_MONOTONIC stamp can be), now: when it was read.
pub const PENSYNC_K_MAX_AGE_MS: u32 = 10000;
pub fn pensync_ms(t_us: u64, mono: bool, now: u32) -> u32 {
    todo!()
}

/// The pen Grimoire follows (stylus-custom) carries position and buttons only; pressure and tilt come
/// from the raw digitizer (WacomDigitizer), which the Kindle's pen service relays it from, a frame
/// for a frame, about 0.1 ms later. Both are read in turns, the raw one first, and a read takes
/// everything queued: after a wait of 100 ms that is some 30 frames of each. Each pen frame used to
/// take the newest raw values, so such a stretch of a stroke was drawn with one flat pressure and
/// tilt. The raw frames are kept instead (about 0.8 s of them), and each pen frame takes the raw
/// frame of its moment: the newest one stamped at or before it, by the kernel's times (both nodes'
/// readers are switched to CLOCK_MONOTONIC).
pub const PENSYNC_RAW_N: usize = 256;
#[derive(Clone, Copy, Debug, Default)]
pub struct PenRaw {
    /// the raw frame's SYN_REPORT time (or when a resync asked)
    pub t_us: u64,
    pub p: i32,
    pub tx: i32,
    pub ty: i32,
}
#[derive(Clone, Debug)]
pub struct PenRawRing {
    pub r: [PenRaw; PENSYNC_RAW_N],
    pub next: i32,
    pub count: i32,
}
impl Default for PenRawRing {
    fn default() -> Self {
        PenRawRing { r: [PenRaw::default(); PENSYNC_RAW_N], next: 0, count: 0 }
    }
}

/// Forgets every frame kept: the raw digitizer dropped events (what is kept no longer meets what
/// comes next), or the pens went to the native PIN screen and back.
pub fn pensync_raw_clear(g: &mut PenRawRing) {
    todo!()
}
/// A raw frame read (pressure and tilt as the digitizer reports them), at its kernel time. Frames come
/// in time order: a resync after a drop pushes what it asked at the time it asked.
pub fn pensync_raw_push(g: &mut PenRawRing, t_us: u64, p: i32, tx: i32, ty: i32) {
    todo!()
}
/// The raw frame a pen frame of time t_us goes with: the newest kept at or before t_us. False when
/// none is that old (nothing kept, a drop forgot them, or it is older than the oldest kept): the
/// caller takes the raw digitizer's newest values, as before there was a ring.
pub fn pensync_raw_at(g: &PenRawRing, t_us: u64, out: &mut PenRaw) -> bool {
    todo!()
}

/// The pen service relays the pen coming and going, and where it touches, but not where it hovers
/// (proximity only: no X or Y between BTN_TOUCH 0 and BTN_TOOL_PEN 0, in the raw pen log of
/// 2026-10-26). The raw digitizer streams its hover frames, so Live view's hovering pen is taken from
/// it, in the scaled pen's units. The pen service scales each raw axis linearly onto the screen
/// (15624 / 1860 = 20832 / 2480 = 8.4 on the Scribe 2, within 3 px), each maybe reversed. Each axis
/// is taken as it is (as Grimoire's raw-only fallback and KOReader take the raw axes) until contact
/// frames, where both pens report the same moment, say otherwise; and not at all when even the best
/// orientation is far from the scaled pen, since a ring in the wrong place is worse than none.
/// contact frames compared before they decide
pub const PENSYNC_HOVER_FRAMES: i32 = 32;
/// px: the best orientation's mean distance from the scaled pen, at most
pub const PENSYNC_HOVER_TOL: i32 = 8;
/// px: one frame's distance counts at most this (a stray frame weighs little)
pub const PENSYNC_HOVER_CAP: i32 = 64;
#[derive(Clone, Copy, Debug, Default)]
pub struct PenHover {
    /// the raw digitizer's x and y ranges
    pub rmin: [i32; 2],
    pub rmax: [i32; 2],
    /// the scaled pen's
    pub smin: [i32; 2],
    pub smax: [i32; 2],
    /// per orientation (bit 0: x reversed, bit 1: y reversed): distances summed
    pub err: [u32; 4],
    /// contact frames compared (halved with err past 4096, so it keeps up)
    pub n: u32,
}

pub fn pensync_hover_init(h: &mut PenHover, rmin: &[i32; 2], rmax: &[i32; 2], smin: &[i32; 2], smax: &[i32; 2]) {
    todo!()
}
/// A contact frame: the raw digitizer's place (rx, ry) and the scaled pen's (sx, sy) at the same moment.
pub fn pensync_hover_learn(h: &mut PenHover, rx: i32, ry: i32, sx: i32, sy: i32) {
    todo!()
}
/// The orientation in use: 0 (both axes as they are) until PENSYNC_HOVER_FRAMES frames are compared,
/// then the nearest of the four; -1 when its mean distance is over PENSYNC_HOVER_TOL px (no hover).
pub fn pensync_hover_orientation(h: &PenHover) -> i32 {
    todo!()
}
/// Orientation o's mean distance from the scaled pen so far, in px (0 before any frame).
pub fn pensync_hover_mean(h: &PenHover, o: i32) -> f32 {
    todo!()
}
/// A raw place in the scaled pen's units, under the orientation in use; false when there is none.
pub fn pensync_hover_place(h: &PenHover, rx: i32, ry: i32, sx: &mut f32, sy: &mut f32) -> bool {
    todo!()
}
