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
    if touch_btn {
        return s.touching;
    }
    (s.tool_pen || s.tool_rubber) && s.p > p_min
}

/// The PenState field an event sets (a PS_F_* bit), 0 for one the pen's reader does not follow.
fn field_of(e: &PenEv) -> u32 {
    if e.type_ == PS_EV_KEY {
        return match e.code {
            PS_BTN_TOOL_PEN => PS_F_TOOL_PEN,
            PS_BTN_TOOL_RUBBER => PS_F_TOOL_RUBBER,
            PS_BTN_TOUCH => PS_F_TOUCH,
            PS_BTN_STYLUS => PS_F_SIDE,
            _ => 0,
        };
    }
    if e.type_ == PS_EV_ABS {
        return match e.code {
            PS_ABS_X => PS_F_X,
            PS_ABS_Y => PS_F_Y,
            PS_ABS_PRESSURE => PS_F_P,
            PS_ABS_TILT_X => PS_F_TX,
            PS_ABS_TILT_Y => PS_F_TY,
            _ => 0,
        };
    }
    0
}

fn set_field(s: &mut PenState, f: u32, v: i32) {
    match f {
        PS_F_TOOL_PEN => s.tool_pen = v != 0,
        PS_F_TOOL_RUBBER => s.tool_rubber = v != 0,
        PS_F_TOUCH => s.touching = v != 0,
        PS_F_SIDE => s.side = v != 0,
        PS_F_X => s.x = v,
        PS_F_Y => s.y = v,
        PS_F_P => s.p = v,
        PS_F_TX => s.tx = v,
        PS_F_TY => s.ty = v,
        _ => {}
    }
}

fn get_field(s: &PenState, f: u32) -> i32 {
    match f {
        PS_F_TOOL_PEN => s.tool_pen as i32,
        PS_F_TOOL_RUBBER => s.tool_rubber as i32,
        PS_F_TOUCH => s.touching as i32,
        PS_F_SIDE => s.side as i32,
        PS_F_X => s.x,
        PS_F_Y => s.y,
        PS_F_P => s.p,
        PS_F_TX => s.tx,
        PS_F_TY => s.ty,
        _ => 0,
    }
}

fn is_syn(e: &PenEv, code: u16) -> bool {
    e.type_ == PS_EV_SYN && e.code == code
}

// (cap is out's length: the caller's room.)
fn push(out: &mut [PenFrame], n: &mut usize, f: PenFrame) {
    if *n < out.len() {
        out[*n] = f;
        *n += 1;
    }
}

/// The lift that goes first when the pen touched at the last good point and the tail shows that
/// stroke ended in the lost stretch: no contact at the first frame, or none just before it (pre: a
/// BTN_TOUCH 1 in the first frame means the pen was up until then, as the input core passes only
/// changes), or the other end by then (the eraser came down). The stroke ends where it was last
/// seen, not wherever the tail (or the device now) finds the pen, and the first frame starts anew.
fn lift_first(
    t: &PenTail,
    pre: &PenState,
    first: &PenState,
    t_us: u64,
    out: &mut [PenFrame],
    n: &mut usize,
    in_: &mut PenTailInfo,
) {
    let g = &t.good;
    if !pensync_contact(g, t.touch_btn, t.p_min) {
        return;
    }
    if pensync_contact(pre, t.touch_btn, t.p_min)
        && pensync_contact(first, t.touch_btn, t.p_min)
        && pre.tool_pen == g.tool_pen
        && pre.tool_rubber == g.tool_rubber
    {
        return;
    }
    let mut up = t.good;
    up.touching = false;
    if !t.touch_btn {
        up.p = t.p_min;
    }
    push(out, n, PenFrame { s: up, t_us, ..Default::default() });
    in_.lifted = true;
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
    let mut in_ = PenTailInfo::default();
    let mut n = 0usize;
    let tn = t.ev.len();
    // A second drop in what was read throws away everything before it too: start after the last.
    let mut start = 0usize;
    for i in 0..tn {
        if is_syn(&t.ev[i], PS_SYN_DROPPED) {
            start = i + 1;
        }
    }
    // The partial frame, up to its SYN_REPORT: left out (the rest of it was thrown away).
    let mut k = start;
    while k < tn && !is_syn(&t.ev[k], PS_SYN_REPORT) {
        k += 1;
    }
    in_.skipped = (k - start) as i32;
    let tail = if k < tn { k + 1 } else { tn };
    let mut end = tail;
    for i in tail..tn {
        if is_syn(&t.ev[i], PS_SYN_REPORT) {
            end = i + 1;
        }
    }
    in_.used = (if k < tn { end } else { tn }) as i32;

    // The state before the tail's first frame. What the tail carries is known from it; what it
    // never carries did not change in it, and the device's present state says what that is.
    let mut s = t.good;
    let mut carried = 0u32;
    for i in tail..end {
        let f = field_of(&t.ev[i]);
        if f == 0 || (carried & f) != 0 {
            continue;
        }
        carried |= f;
        set_field(&mut s, f, if (f & PS_F_KEYS) != 0 { (t.ev[i].value == 0) as i32 } else { t.ev[i].value });
    }
    let mut f = 1u32;
    while (f & PS_F_ALL) != 0 {
        if (carried & f) == 0 && (t.now_ok & f) != 0 {
            set_field(&mut s, f, get_field(&t.now, f));
        }
        f <<= 1;
    }
    let pre = s;

    let mut moved = false;
    for i in tail..end {
        let e = &t.ev[i];
        let f = field_of(e);
        if f != 0 {
            set_field(&mut s, f, e.value);
            if (f & PS_F_KEYS) != 0 {
                in_.keys = true;
            } else {
                moved = true;
            }
        }
        if !is_syn(e, PS_SYN_REPORT) {
            continue;
        }
        if in_.frames == 0 {
            lift_first(t, &pre, &s, e.t_us, out, &mut n, &mut in_);
            push(out, &mut n, PenFrame { s, t_us: e.t_us, moved: true, gap: true });
            in_.first_us = e.t_us;
        } else {
            push(out, &mut n, PenFrame { s, t_us: e.t_us, moved, gap: false });
        }
        in_.last_us = e.t_us;
        in_.frames += 1;
        moved = false;
    }

    if in_.frames == 0 {
        // Nothing complete after the partial frame: the pen is as the device says now, at the time
        // of the partial frame's SYN_REPORT (or of the last event read).
        let at = if k < tn {
            t.ev[k].t_us
        } else if tn != 0 {
            t.ev[tn - 1].t_us
        } else {
            0
        };
        lift_first(t, &s, &s, at, out, &mut n, &mut in_);
        push(out, &mut n, PenFrame { s, t_us: at, moved: true, gap: true });
    } else if (t.now_ok & PS_F_KEYS) != 0 {
        // A button the tail carried that the device says has changed since: that event came after
        // the tail was read, and EVIOCGKEY took it out of the queue. It happens here.
        let mut after = s;
        let mut f = PS_F_TOOL_PEN;
        while (f & PS_F_KEYS) != 0 {
            if (t.now_ok & f) != 0 {
                set_field(&mut after, f, get_field(&t.now, f));
            }
            f <<= 1;
        }
        if after.tool_pen != s.tool_pen
            || after.tool_rubber != s.tool_rubber
            || after.touching != s.touching
            || after.side != s.side
        {
            s = after;
            push(out, &mut n, PenFrame { s, t_us: in_.last_us, ..Default::default() });
        }
    }
    in_.x = s.x;
    in_.y = s.y;
    *info = in_;
    n as i32
}

/// A sample's time for the stroke (InEvent.k_ms): the kernel's time of its frame in now_ms()'s
/// uint32 milliseconds, when the reader's clock is CLOCK_MONOTONIC (mono: EVIOCSCLOCKID took). Else,
/// or for a time of 0, one after now, or one over PENSYNC_K_MAX_AGE_MS before now (none of which a
/// real CLOCK_MONOTONIC stamp can be), now: when it was read.
pub const PENSYNC_K_MAX_AGE_MS: u32 = 10000;
pub fn pensync_ms(t_us: u64, mono: bool, now: u32) -> u32 {
    if !mono || t_us == 0 {
        return now;
    }
    let k = (t_us / 1000) as u32; // as now_ms() truncates: sec * 1000 + usec / 1000
    let age = now.wrapping_sub(k);
    if (age as i32) < 0 || age > PENSYNC_K_MAX_AGE_MS {
        return now;
    }
    k
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
    g.next = 0;
    g.count = 0;
}
/// A raw frame read (pressure and tilt as the digitizer reports them), at its kernel time. Frames come
/// in time order: a resync after a drop pushes what it asked at the time it asked.
pub fn pensync_raw_push(g: &mut PenRawRing, t_us: u64, p: i32, tx: i32, ty: i32) {
    g.r[g.next as usize] = PenRaw { t_us, p, tx, ty };
    g.next = (g.next + 1) % PENSYNC_RAW_N as i32;
    if g.count < PENSYNC_RAW_N as i32 {
        g.count += 1;
    }
}
/// The raw frame a pen frame of time t_us goes with: the newest kept at or before t_us. False when
/// none is that old (nothing kept, a drop forgot them, or it is older than the oldest kept): the
/// caller takes the raw digitizer's newest values, as before there was a ring.
pub fn pensync_raw_at(g: &PenRawRing, t_us: u64, out: &mut PenRaw) -> bool {
    // From the newest back: the pen frame being read is nearly always within the last few.
    let n = PENSYNC_RAW_N as i32;
    for i in 1..=g.count {
        let r = &g.r[((g.next - i + n) % n) as usize];
        if r.t_us <= t_us {
            *out = *r;
            return true;
        }
    }
    false
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
    *h = PenHover { rmin: *rmin, rmax: *rmax, smin: *smin, smax: *smax, err: [0; 4], n: 0 };
}

/// A raw coordinate on axis a in the scaled pen's units, reversed or not.
fn hover_axis(h: &PenHover, a: usize, v: i32, reversed: bool) -> f32 {
    let span = h.rmax[a] - h.rmin[a];
    if span <= 0 {
        return h.smin[a] as f32;
    }
    let mut f = ((v - h.rmin[a]) as f32 / span as f32).clamp(0.0, 1.0);
    if reversed {
        f = 1.0 - f;
    }
    // smin + f * (smax - smin) in one expression: clang fuses it (PORTING.md, Floating point).
    f.mul_add((h.smax[a] - h.smin[a]) as f32, h.smin[a] as f32)
}
/// A contact frame: the raw digitizer's place (rx, ry) and the scaled pen's (sx, sy) at the same moment.
pub fn pensync_hover_learn(h: &mut PenHover, rx: i32, ry: i32, sx: i32, sy: i32) {
    for o in 0..4usize {
        let dx = hover_axis(h, 0, rx, (o & 1) != 0) - sx as f32;
        let dy = hover_axis(h, 1, ry, (o & 2) != 0) - sy as f32;
        let d = (if dx < 0.0 { -dx } else { dx }) + (if dy < 0.0 { -dy } else { dy });
        h.err[o] += if d < PENSYNC_HOVER_CAP as f32 { (d + 0.5) as u32 } else { PENSYNC_HOVER_CAP as u32 };
    }
    h.n += 1;
    if h.n >= 4096 {
        for o in 0..4 {
            h.err[o] /= 2;
        }
        h.n /= 2;
    }
}
/// The orientation in use: 0 (both axes as they are) until PENSYNC_HOVER_FRAMES frames are compared,
/// then the nearest of the four; -1 when its mean distance is over PENSYNC_HOVER_TOL px (no hover).
pub fn pensync_hover_orientation(h: &PenHover) -> i32 {
    if h.n < PENSYNC_HOVER_FRAMES as u32 {
        return 0;
    }
    let mut best = 0usize;
    for o in 1..4usize {
        if h.err[o] < h.err[best] {
            best = o;
        }
    }
    if h.err[best] <= PENSYNC_HOVER_TOL as u32 * h.n {
        best as i32
    } else {
        -1
    }
}
/// Orientation o's mean distance from the scaled pen so far, in px (0 before any frame).
pub fn pensync_hover_mean(h: &PenHover, o: i32) -> f32 {
    if h.n != 0 && (0..4).contains(&o) {
        h.err[o as usize] as f32 / h.n as f32
    } else {
        0.0
    }
}
/// A raw place in the scaled pen's units, under the orientation in use; false when there is none.
pub fn pensync_hover_place(h: &PenHover, rx: i32, ry: i32, sx: &mut f32, sy: &mut f32) -> bool {
    let o = pensync_hover_orientation(h);
    if o < 0 || h.rmax[0] <= h.rmin[0] || h.rmax[1] <= h.rmin[1] {
        return false;
    }
    *sx = hover_axis(h, 0, rx, (o & 1) != 0);
    *sy = hover_axis(h, 1, ry, (o & 2) != 0);
    true
}
