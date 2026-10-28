//! touchgest: the touch recognizer, pure and unit-tested on the host like pensync.rs. Both readers
//! of the touch panel hand it contact frames (every contact down at one SYN_REPORT, in logical screen
//! px): input_evdev.rs on the Kindle, and the headless runner's `finger`, `frame`, `swipe` and `tapn`
//! lines (sim_sdl.rs). It answers with the events the app has always had (TouchDown, TouchMove and
//! TouchUp for one finger, PinchBegin, PinchMove and PinchEnd for two), plus InType::Gesture.
//!
//! It keeps no App and does no I/O: its state lives in the reader's state, it never calls out, it
//! never allocates, and a frame costs O(1) (at most TG_MAX contacts). Every time it reads is the
//! frame's (TgFrame.k_ms, the kernel's time of the SYN_REPORT in now_ms()'s clock), never the clock:
//! after a stall the read times bunch up, the kernel's keep the real spacing. Times are u32 compared
//! with wrapping_sub (the PORTING timing rule). Places are classified in integer px rounded from
//! map()'s f32, and the events carry map()'s f32 unchanged, so one finger's events are what the
//! reader made of them before this module existed. The numbers marked prov. are provisional: the
//! owner's touch traces (touch-trace.txt) will set them.
//!
//! One finger is the stream it always was: TouchDown at once, TouchMove when the place changes,
//! TouchUp at the lift, nothing added in between. While the pen is near, or for a second after it
//! leaves, a finger is the writing hand resting on the screen and gives nothing, and a finger that
//! was already down ends with the off-screen cancel (TOUCH_F_CANCEL, x = y = -10000), never a lift
//! in its place, which could follow a link or press a button under the palm.
//!
//! A second finger starts a two-finger session: the first finger's stream ends with the cancel, and
//! PinchBegin (value 1) names both fingers by the order they landed in. Within 150 ms of the first it
//! may become a swipe or a tap; up to 400 ms, a zoom or a pan only; later, it is a palm or a second
//! hand, as it always was, and every contact is suppressed until all have lifted. Nothing moves until
//! the session commits (the spread changes by a tenth, or the midpoint travels 70 px at 300 dpi, the
//! editor's own dead zone). It is classified once, there: a zoom when the spread changed most (and
//! a pinch may zoom), else a drag along its main axis, which pans on an axis the page scrolls on
//! (per axis, never per direction: a drag at a scroll limit pans nowhere and never turns a page), is
//! refused more than 30 degrees off its axis when nothing scrolls, and is otherwise a swipe. Each
//! frame from the commit brings a PinchMove whose value is the class (1 zoom, 2 pan, 3 swipe).
//!
//! A gesture fires at the first lift, never later (no double taps, so nothing waits): a swipe that
//! went far enough, straight, both fingers the same way, or a tap whose fingers all lifted quickly
//! without moving (its decision waits only for the last of them). Its events are PinchEnd then
//! Gesture, in one call. A refusal is PinchEnd alone (value 1 when only the pen refused it, so the
//! editor can say so). After either, every contact waits to lift, so a late finger never leaks a tap.
//!
//! The pen always wins: a session the pen is near (touching, hovering, or gone under 300 ms) never
//! fires, and one it left 300 ms to 1 s ago fires only far from where it last touched. A contact that
//! rests (a grip thumb, a writing palm: still for 300 ms before the gesture's first finger, or flagged
//! a palm) is left out of the count, as long as it is far from the gesture's fingers; if it moves, the
//! gesture is refused.

use crate::prelude::*;


/// Contacts followed at once (input_evdev.rs's MAX_SLOTS).
pub const TG_MAX: usize = 16;

/// One contact of a frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct TgContact {
    /// tracking id (the runner: the finger's ID)
    pub id: i32,
    /// evdev slot (the runner: the finger's ID)
    pub slot: i32,
    /// logical screen px after map() (the runner: make_ev's)
    pub x: f32,
    pub y: f32,
    /// device-normalized 0..1 before calibration: calibrate.rs reads ev.nx and ev.ny
    pub nx: f32,
    pub ny: f32,
    /// ABS_MT_TOUCH_MAJOR, 0 unknown
    pub major: i32,
    /// 0 finger, 1 pen, 2 palm (MT_TOOL_PALM)
    pub tool: u8,
    /// ABS_MT_PRESSURE, 0 unknown
    pub pressure: i32,
}

/// Every contact down at one frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct TgFrame {
    pub k_ms: u32,
    pub n: usize,
    pub c: [TgContact; TG_MAX],
}

/// One column of the panel's slots, as EVIOCGMTSLOTS answers it for one ABS_MT code (`ask` is that
/// ioctl: it gets a buffer whose first int is the code and returns whether the kernel took the
/// request). The kernel writes only as many ints as the panel has slots (the smaller of that and the
/// buffer) and leaves the rest of the buffer as it was, so the buffer goes in filled with `empty`: for
/// the tracking id column that is -1 (no contact; a 0 there is a valid id, a finger that never lifts),
/// for the others 0. False, and `col` untouched, when the ioctl failed.
pub fn mt_column(ask: &mut dyn FnMut(&mut [i32; TG_MAX + 1]) -> bool, code: i32, empty: i32, col: &mut [i32; TG_MAX]) -> bool {
    let mut buf = [empty; TG_MAX + 1];
    buf[0] = code;
    if !ask(&mut buf) {
        return false;
    }
    col.copy_from_slice(&buf[1..]);
    true
}

/// The pen, as the touch side sees it.
#[derive(Clone, Copy, Debug, Default)]
pub struct TgPen {
    /// pen or eraser in proximity
    pub prox: bool,
    pub touching: bool,
    /// when it last left proximity (now_ms()'s clock; State.pen_left_ms on the Kindle)
    pub left_ms: u32,
    /// its last contact point, logical px
    pub last: (f32, f32),
}

/// What the screen in front allows, pushed by the editor (input_touch_context).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TgView {
    /// the page can scroll on that axis now
    pub scrolls_v: bool,
    pub scrolls_h: bool,
    /// settings.pinch_zoom, and no sheet in front that refuses pinches
    pub zoom_ok: bool,
    /// settings.gestures
    pub gestures_on: bool,
    /// phase 3's recording; always false now
    pub record: bool,
    pub keyboard_open: bool,
}

/// What one call made: its events in order (the caller queues them as they are), and what it
/// decided, for the log, the trace and the probes.
#[derive(Clone, Copy, Debug, Default)]
pub struct TgOut {
    pub n: usize,
    pub ev: [InEvent; 8],
    pub outcome: TgOutcome,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TgOutcome {
    #[default]
    None,
    Fired(i32),
    Rejected(TgReject),
}

/// Why a session was refused (touch.reject names them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TgReject {
    Pen,
    Apart,
    Straight,
    Short,
    Direction,
    Hand,
    Spread,
    Late,
    Three,
    Keyboard,
    Off,
    Slow,
}

/// The gestures (InEvent.value of an InType::Gesture). The three-finger ones are named for the
/// files and never fired yet.
pub const G_2_UP: i32 = 1;
pub const G_2_DOWN: i32 = 2;
pub const G_2_LEFT: i32 = 3;
pub const G_2_RIGHT: i32 = 4;
pub const G_2_TAP: i32 = 5;
pub const G_3_UP: i32 = 6;
pub const G_3_DOWN: i32 = 7;
pub const G_3_LEFT: i32 = 8;
pub const G_3_RIGHT: i32 = 9;
pub const G_3_TAP: i32 = 10;


// ---- the rules' numbers ----

/// A second finger this soon after the first makes a session that may swipe or tap (prov.).
const LAND_MS: u32 = 150;
/// Up to this, a later second finger zooms or pans only; later still, a second contact is a palm or
/// a second hand, as it always was.
const PALM_MS: u32 = 400;
/// A finger this soon after the pen was near is the writing hand.
const PEN_WINDOW_MS: u32 = 1000;
/// A session the pen left less than this before is the pen's hand: it never fires (prov.).
const PEN_GONE_MS: u32 = 300;
/// A kernel time this far before the pen's leave still counts as the pen being near: a touch frame
/// read after the leave can be older than it (PENSYNC_K_MAX_AGE_MS bounds how old k_ms gets).
const PEN_BEHIND_MS: u32 = 10000;
/// A resting contact was still (within REST_MM of where it landed) for this long before the
/// gesture's first finger (prov.). REST_MM is also the move that fixes the finger count.
const REST_MS: u32 = 300;
const REST_MM: f32 = 3.0;
/// The commit: the spread changes by this much, or the midpoint travels COMMIT_PX at 300 dpi.
const COMMIT_SPREAD: f32 = 0.10;
const COMMIT_PX: f32 = 70.0;
/// A commit is a zoom when the spread changed at least this much of the midpoint's travel.
const ZOOM_RATIO: f32 = 0.6;
/// tan 30 degrees: a drag farther off its axis is not straight.
const TAN30: f32 = 0.577_350_3;
/// A swipe goes SWIPE_MM along its axis, each finger at least FINGER_MM within the cosine COS_MIN of
/// the midpoint's way, the farthest finger at most FINGER_RATIO times the nearest, the spread
/// changed by at most SPREAD_MAX, all within SWIPE_MS of the first landing (prov.).
const SWIPE_MM: f32 = 20.0;
const FINGER_MM: f32 = 12.0;
const COS_MIN: f32 = 0.8;
const FINGER_RATIO: f32 = 2.0;
const SPREAD_MAX: f32 = 0.25;
const SWIPE_MS: u32 = 1000;
/// A tap: every finger lifts within TAP_LIFT_MS of the others, TAP_MS from the first landing to
/// the last lift, none moving more than TAP_MM (prov.).
const TAP_LIFT_MS: u32 = 150;
const TAP_MS: u32 = 300;
const TAP_MM: f32 = 5.0;
/// A gesture's fingers are at least APART_MIN_MM and at most APART_MAX_MM apart (prov.).
const APART_MIN_MM: f32 = 8.0;
const APART_MAX_MM: f32 = 70.0;
/// A resting contact, and the pen's last point once it left 300 ms to 1 s ago, keep this far from
/// every finger of the gesture (prov.).
const FAR_MM: f32 = 40.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Ph {
    /// nothing down (or only contacts the panel calls a pen: never fingers)
    #[default]
    Idle,
    /// one finger followed (one_on: its TouchDown went out; else the pen window holds it, for
    /// LAND_MS in case a second finger makes a gesture of it)
    One,
    /// a lone finger beside resting contacts, quiet, waiting PALM_MS for a second one
    Wait,
    /// two fingers: PinchBegin went out
    Session,
    /// an uncommitted session's first finger lifted: a tap, if the other lifts in time
    TapWait,
    /// every contact down waits to lift (while none is locked, a new pair beside resting ones may
    /// still make a gesture)
    Suppressed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Class {
    /// not committed yet
    #[default]
    Pending,
    Zoom,
    Pan,
    Swipe,
}

/// How a touch session (first contact down to all lifted) went, for grimoire.log.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Result_ {
    #[default]
    None,
    Fired(i32),
    Rejected(TgReject),
    Zoom,
    Pan,
    Palm,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Default)]
struct Track {
    on: bool,
    /// in this frame
    seen: bool,
    /// landed in this frame
    fresh: bool,
    id: i32,
    slot: i32,
    down_k: u32,
    /// where it landed, where it is, and where it was as the session began or it started resting
    /// (integer px)
    x0: i32,
    y0: i32,
    ix: i32,
    iy: i32,
    sx: i32,
    sy: i32,
    /// where it was as the session began (map()'s f32), for the gesture's start midpoint
    fx: f32,
    fy: f32,
    /// the newest contact as given
    c: TgContact,
    /// the farthest it has been from where it landed, px
    far: f32,
    /// TOUCH_MAJOR passed T_palm at some frame
    broad: bool,
    /// ever reported as a pen (tool 1): never a finger
    pen: bool,
    /// ever reported as a palm (tool 2)
    palm: bool,
    /// left out of the count beside a gesture's fingers
    resting: bool,
    /// waits to lift: no gesture forms while it is down
    locked: bool,
    /// its place in the touch session's record (sess), TG_MAX past its room
    si: usize,
}

/// The pixel sizes of the rules, from millimetres at the screen's dpi (touchgest_init).
#[derive(Clone, Copy, Debug)]
struct Thr {
    commit: f32,
    rest: f32,
    tap: f32,
    apart_min: f32,
    apart_max: f32,
    finger: f32,
    swipe: f32,
    far: f32,
}

impl Thr {
    fn at(dpi: f32) -> Thr {
        // (ui_px's scale: 300 dpi is 1)
        let scale = if dpi > 50.0 { dpi / 300.0 } else { 1.0 };
        Thr {
            commit: (COMMIT_PX * scale).round_ties_even(),
            rest: mm_to_px(REST_MM, dpi),
            tap: mm_to_px(TAP_MM, dpi),
            apart_min: mm_to_px(APART_MIN_MM, dpi),
            apart_max: mm_to_px(APART_MAX_MM, dpi),
            finger: mm_to_px(FINGER_MM, dpi),
            swipe: mm_to_px(SWIPE_MM, dpi),
            far: mm_to_px(FAR_MM, dpi),
        }
    }
}

/// The recognizer's state (the reader keeps it: input_evdev's State, the runner's).
#[derive(Clone, Copy, Debug)]
pub struct TouchGest {
    t: [Track; TG_MAX],
    ph: Ph,
    /// One: the finger followed, and whether its TouchDown went out; Wait: the lone finger
    one: usize,
    one_on: bool,
    /// the last TouchDown or TouchMove, whose place the TouchUp repeats
    last: InEvent,
    /// Session: the pair by landing order, the first one's landing, the first lift
    p: [usize; 2],
    first_k: u32,
    lift_k: u32,
    class: Class,
    /// a zoom or pan the pen forbids: it moves nothing
    inert: bool,
    /// the commit's axis was horizontal
    axis_h: bool,
    /// the second finger landed after LAND_MS: a zoom or a pan only
    slow: bool,
    /// landed in the pen window: never a zoom or a pan
    in_window: bool,
    /// the pen touched, was near or left under PEN_GONE_MS before, at some frame: never fires
    blocked: bool,
    /// left PEN_GONE_MS to PEN_WINDOW_MS before, with a finger within FAR_MM of its last point
    pen_close: bool,
    /// the finger count is fixed: a third finger now is late
    fixed: bool,
    /// spread and midpoint as the session began (integer px)
    d0: f32,
    mx0: f32,
    my0: f32,
    /// PinchBegin went out and its PinchEnd has not
    begun: bool,
    /// a cancel came: every contact of the next frame waits to lift (the panel's state after a drop,
    /// or the fingers still down when the Kindle hands the panel back)
    hold: bool,
    view: TgView,
    palm_major: i32,
    thr: Thr,
    last_gesture: i32,
    last_reject: Option<TgReject>,
    result: Result_,
    /// the touch session's record (first contact down to all lifted), for grimoire.log: each
    /// contact's landing and lift, in the order they landed
    sess: [(u32, u32); TG_MAX],
    sess_n: usize,
}

impl Default for TouchGest {
    fn default() -> Self {
        TouchGest {
            t: [Track::default(); TG_MAX],
            ph: Ph::Idle,
            one: 0,
            one_on: false,
            last: InEvent::default(),
            p: [0; 2],
            first_k: 0,
            lift_k: 0,
            class: Class::Pending,
            inert: false,
            axis_h: false,
            slow: false,
            in_window: false,
            blocked: false,
            pen_close: false,
            fixed: false,
            d0: 0.0,
            mx0: 0.0,
            my0: 0.0,
            begun: false,
            hold: false,
            view: TgView::default(),
            palm_major: i32::MAX,
            thr: Thr::at(300.0),
            last_gesture: 0,
            last_reject: None,
            result: Result_::None,
            sess: [(0, 0); TG_MAX],
            sess_n: 0,
        }
    }
}

fn ipx(v: f32) -> i32 {
    v.round() as i32
}

fn dist(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let (dx, dy) = ((bx - ax) as f32, (by - ay) as f32);
    (dx * dx + dy * dy).sqrt()
}

/// b came at least ms after a (wrapping times).
fn at_least(a: u32, b: u32, ms: u32) -> bool {
    b.wrapping_sub(a) as i32 >= ms as i32
}

/// More than ms went from a to b.
fn over(a: u32, b: u32, ms: u32) -> bool {
    b.wrapping_sub(a) as i32 > ms as i32
}

impl TouchGest {
    fn push(&self, out: &mut TgOut, mut e: InEvent, k: u32) {
        if out.n >= out.ev.len() {
            return; // (never: a call makes at most a few)
        }
        e.t_ms = k;
        e.k_ms = k;
        out.ev[out.n] = e;
        out.n += 1;
    }

    /// The pen is near, or left less than PEN_WINDOW_MS ago (by the frame's time).
    fn pen_window(pen: &TgPen, k: u32) -> bool {
        pen.prox || pen.touching || Self::gone_within(pen, k, PEN_WINDOW_MS)
    }

    /// The pen left less than ms before k (a frame older than the leave counts too).
    fn gone_within(pen: &TgPen, k: u32, ms: u32) -> bool {
        let d = k.wrapping_sub(pen.left_ms) as i32;
        d > -(PEN_BEHIND_MS as i32) && d < ms as i32
    }

    fn flags(&self, i: usize) -> i32 {
        if self.t[i].broad {
            TOUCH_F_BROAD
        } else {
            0
        }
    }

    fn touch_ev(&self, type_: InType, i: usize) -> InEvent {
        let c = &self.t[i].c;
        InEvent { type_, x: c.x, y: c.y, nx: c.nx, ny: c.ny, value: self.flags(i), ..Default::default() }
    }

    fn d(&self, a: usize, b: usize) -> f32 {
        let (p, q) = (&self.t[a], &self.t[b]);
        dist(p.ix, p.iy, q.ix, q.iy)
    }

    /// The followed finger's stream ends without a tap: the off-screen cancel.
    fn cancel_one(&mut self, out: &mut TgOut, k: u32) {
        if self.ph == Ph::One && self.one_on {
            let e = InEvent {
                type_: InType::TouchUp,
                x: -10000.0,
                y: -10000.0,
                value: TOUCH_F_CANCEL | self.flags(self.one),
                ..Default::default()
            };
            self.push(out, e, k);
            self.one_on = false;
        }
    }

    fn pinch_end(&mut self, out: &mut TgOut, k: u32, value: i32) {
        if self.begun {
            self.push(out, InEvent { type_: InType::PinchEnd, value, ..Default::default() }, k);
            self.begun = false;
        }
    }

    fn pinch_ev(&self, type_: InType, value: i32) -> InEvent {
        let (a, b) = (&self.t[self.p[0]].c, &self.t[self.p[1]].c);
        InEvent { type_, x: a.x, y: a.y, x2: b.x, y2: b.y, value, ..Default::default() }
    }

    /// Every contact down waits to lift (resting ones stay resting: they were never part of it).
    fn suppress(&mut self) {
        for t in self.t.iter_mut() {
            if t.on && !t.resting {
                t.locked = true;
            }
        }
        self.ph = Ph::Suppressed;
    }

    fn reject(&mut self, out: &mut TgOut, k: u32, r: TgReject) {
        self.pinch_end(out, k, if r == TgReject::Pen { 1 } else { 0 });
        self.last_reject = Some(r);
        out.outcome = TgOutcome::Rejected(r);
        self.result = Result_::Rejected(r);
        self.suppress();
    }

    /// The gesture fires, unless gestures are off, a keyboard is up, or the pen was near.
    fn accept(&mut self, out: &mut TgOut, k: u32, id: i32) {
        if !self.view.gestures_on {
            self.reject(out, k, TgReject::Off);
        } else if self.view.keyboard_open {
            self.reject(out, k, TgReject::Keyboard);
        } else if self.blocked || self.pen_close {
            self.reject(out, k, TgReject::Pen);
        } else {
            self.pinch_end(out, k, 0);
            let (a, b) = (&self.t[self.p[0]], &self.t[self.p[1]]);
            let e = InEvent {
                type_: InType::Gesture,
                value: id,
                x: (a.fx + b.fx) * 0.5,
                y: (a.fy + b.fy) * 0.5,
                x2: (a.c.x + b.c.x) * 0.5,
                y2: (a.c.y + b.c.y) * 0.5,
                ..Default::default()
            };
            self.push(out, e, k);
            self.last_gesture = id;
            self.last_reject = None;
            out.outcome = TgOutcome::Fired(id);
            self.result = Result_::Fired(id);
            self.suppress();
        }
    }

    /// Track i may rest beside a gesture whose first finger landed at first_k: flagged a palm, or
    /// still for REST_MS before; either way FAR_MM from each of the fingers, and not locked.
    fn rest_ok(&self, i: usize, fingers: &[usize], first_k: u32) -> bool {
        let t = &self.t[i];
        !t.locked
            && (t.palm || (t.far <= self.thr.rest && at_least(t.down_k, first_k, REST_MS)))
            && fingers.iter().all(|&f| self.d(i, f) >= self.thr.far)
    }

    /// Every other contact down (pens aside) may rest beside these new fingers: they rest from now on.
    fn rest_others(&mut self, fingers: &[usize], first_k: u32) -> bool {
        let others = |t: &Track, i: usize| t.on && !t.pen && !fingers.contains(&i);
        for (i, t) in self.t.iter().enumerate() {
            if others(t, i) && !self.rest_ok(i, fingers, first_k) {
                return false;
            }
        }
        for i in 0..TG_MAX {
            let t = &mut self.t[i];
            if others(t, i) && !t.resting {
                t.resting = true;
                t.sx = t.ix;
                t.sy = t.iy;
            }
        }
        true
    }

    /// The finger count is fixed at the first of: the landing window's end, a finger moving REST_MM,
    /// the first lift (which ends the session anyway).
    fn count_fixed(&self, k: u32) -> bool {
        let (p, q) = (&self.t[self.p[0]], &self.t[self.p[1]]);
        self.fixed || at_least(self.first_k, k, LAND_MS) || p.far >= self.thr.rest || q.far >= self.thr.rest
    }

    /// A resting contact moved: the hand moved.
    fn rest_moved(&self) -> bool {
        self.t.iter().any(|t| t.on && t.resting && dist(t.sx, t.sy, t.ix, t.iy) > self.thr.rest)
    }

    /// New fingers beside resting contacts (every other contact was found to rest): one waits for a
    /// partner, two make a session, more are three.
    fn land(&mut self, fingers: &[usize], pen: &TgPen, out: &mut TgOut, k: u32) {
        match fingers.len() {
            0 => {}
            1 => {
                self.ph = Ph::Wait;
                self.one = fingers[0];
            }
            2 => self.start_session(fingers[0], fingers[1], pen, out, k, false),
            _ => self.reject(out, k, TgReject::Three),
        }
    }

    /// The session's geometry from the pair as they are now (its start, or a new pair).
    fn pair(&mut self, a: usize, b: usize) {
        // By landing order (the same frame: by slot), not by slot.
        let (ta, tb) = (&self.t[a], &self.t[b]);
        let a_first = (ta.down_k.wrapping_sub(tb.down_k) as i32) < 0 || (ta.down_k == tb.down_k && ta.slot <= tb.slot);
        self.p = if a_first { [a, b] } else { [b, a] };
        self.first_k = self.t[self.p[0]].down_k;
        self.slow = over(self.first_k, self.t[self.p[1]].down_k, LAND_MS);
        self.fixed = self.slow;
        for &i in &self.p {
            let t = &mut self.t[i];
            t.sx = t.ix;
            t.sy = t.iy;
            t.fx = t.c.x;
            t.fy = t.c.y;
        }
        let (p, q) = (&self.t[self.p[0]], &self.t[self.p[1]]);
        self.d0 = dist(p.ix, p.iy, q.ix, q.iy);
        self.mx0 = (p.ix + q.ix) as f32 * 0.5;
        self.my0 = (p.iy + q.iy) as f32 * 0.5;
        self.class = Class::Pending;
        self.inert = false;
    }

    fn start_session(&mut self, a: usize, b: usize, pen: &TgPen, out: &mut TgOut, k: u32, from_one: bool) {
        if from_one {
            self.cancel_one(out, k); // the first finger's press turns into the session
        }
        self.pair(a, b);
        self.in_window = Self::pen_window(pen, k);
        self.blocked = false;
        self.pen_close = false;
        self.ph = Ph::Session;
        self.begun = true;
        let e = self.pinch_ev(InType::PinchBegin, 1);
        self.push(out, e, k);
    }

    /// The pen near a session (every frame of it): a session it touched, hovered or left under
    /// PEN_GONE_MS before never fires, and a zoom or pan under way ends there (as it always has); one
    /// it left PEN_GONE_MS to PEN_WINDOW_MS before fires only with every finger FAR_MM from its last
    /// point. True when the session ended.
    fn pen_near(&mut self, pen: &TgPen, out: &mut TgOut, k: u32) -> bool {
        if pen.touching || pen.prox || Self::gone_within(pen, k, PEN_GONE_MS) {
            self.blocked = true;
            if self.ph == Ph::Session && !self.inert && (self.class == Class::Zoom || self.class == Class::Pan) {
                self.result = if self.class == Class::Zoom { Result_::Zoom } else { Result_::Pan };
                self.pinch_end(out, k, 0);
                self.suppress();
                return true;
            }
        } else if Self::gone_within(pen, k, PEN_WINDOW_MS) {
            let (lx, ly) = (ipx(pen.last.0), ipx(pen.last.1));
            for &i in &self.p {
                let t = &self.t[i];
                if t.on && dist(t.ix, t.iy, lx, ly) < self.thr.far {
                    self.pen_close = true;
                }
            }
        }
        false
    }

    /// The commit's class: a zoom when the spread changed most of the travel (and the pen allows
    /// it), else a drag along its main axis: a pan on an axis the page scrolls on, refused more than
    /// 30 degrees off its axis when nothing scrolls, else a swipe (never for a late second finger).
    fn classify(&mut self, out: &mut TgOut, k: u32, d: f32, dx: f32, dy: f32) {
        let free = !self.in_window && !self.blocked;
        let travel = (dx * dx + dy * dy).sqrt();
        if self.view.zoom_ok && free && (d - self.d0).abs() >= ZOOM_RATIO * travel {
            self.class = Class::Zoom;
            return;
        }
        let h = dx.abs() > dy.abs();
        let (along, across) = if h { (dx.abs(), dy.abs()) } else { (dy.abs(), dx.abs()) };
        let scrolls = if h { self.view.scrolls_h } else { self.view.scrolls_v };
        self.axis_h = h;
        if across > along * TAN30 {
            if !(self.view.scrolls_h || self.view.scrolls_v) {
                self.reject(out, k, TgReject::Straight);
                return;
            }
            self.class = Class::Pan;
        } else if scrolls {
            self.class = Class::Pan;
        } else if self.slow {
            self.reject(out, k, TgReject::Late);
            return;
        } else {
            self.class = Class::Swipe;
        }
        self.inert = self.class == Class::Pan && !free;
    }

    /// A frame of a session with both fingers down: the commit, then a PinchMove each frame.
    fn session_frame(&mut self, out: &mut TgOut, k: u32) {
        let (p, q) = (self.t[self.p[0]], self.t[self.p[1]]);
        if self.class == Class::Pending {
            let d = dist(p.ix, p.iy, q.ix, q.iy);
            let dx = (p.ix + q.ix) as f32 * 0.5 - self.mx0;
            let dy = (p.iy + q.iy) as f32 * 0.5 - self.my0;
            let travel = (dx * dx + dy * dy).sqrt();
            let spread = (d / self.d0.max(10.0) - 1.0).abs();
            if spread < COMMIT_SPREAD && travel < self.thr.commit {
                return; // nothing moves until the fingers clearly spread or travel
            }
            self.classify(out, k, d, dx, dy);
            if self.ph != Ph::Session {
                return;
            }
        }
        if self.inert {
            return;
        }
        let value = match self.class {
            Class::Zoom => 1,
            Class::Pan => 2,
            _ => 3,
        };
        let e = self.pinch_ev(InType::PinchMove, value);
        self.push(out, e, k);
    }

    /// A swipe's first lift: far enough along its axis, straight, both fingers the same way and about
    /// as far, the spread kept, in time; then the gesture of its way.
    fn swipe_lift(&mut self, out: &mut TgOut, k: u32) {
        let (p, q) = (self.t[self.p[0]], self.t[self.p[1]]);
        let dx = (p.ix + q.ix) as f32 * 0.5 - self.mx0;
        let dy = (p.iy + q.iy) as f32 * 0.5 - self.my0;
        let (along, across) = if self.axis_h { (dx, dy) } else { (dy, dx) };
        let m = (dx * dx + dy * dy).sqrt();
        let mut lens = [0f32; 2];
        let mut same_way = true;
        for (n, t) in [p, q].iter().enumerate() {
            let (fx, fy) = ((t.ix - t.sx) as f32, (t.iy - t.sy) as f32);
            lens[n] = (fx * fx + fy * fy).sqrt();
            same_way &= lens[n] >= self.thr.finger && (fx * dx + fy * dy) >= COS_MIN * lens[n] * m;
        }
        let r = if self.d0 < self.thr.apart_min || self.d0 > self.thr.apart_max {
            Some(TgReject::Apart)
        } else if along.abs() < self.thr.swipe {
            Some(TgReject::Short)
        } else if across.abs() > along.abs() * TAN30 {
            Some(TgReject::Straight)
        } else if !same_way || lens[0].max(lens[1]) > FINGER_RATIO * lens[0].min(lens[1]) {
            Some(TgReject::Direction)
        } else if (dist(p.ix, p.iy, q.ix, q.iy) - self.d0).abs() > SPREAD_MAX * self.d0 {
            Some(TgReject::Spread)
        } else if over(self.first_k, k, SWIPE_MS) {
            Some(TgReject::Slow)
        } else {
            None
        };
        match r {
            Some(r) => self.reject(out, k, r),
            None => {
                let id = match (self.axis_h, along < 0.0) {
                    (true, true) => G_2_LEFT,
                    (true, false) => G_2_RIGHT,
                    (false, true) => G_2_UP,
                    (false, false) => G_2_DOWN,
                };
                self.accept(out, k, id);
            }
        }
    }

    /// One of the session's fingers lifted: a zoom or pan ends, a swipe fires or is refused, and an
    /// uncommitted session may be a tap.
    fn first_lift(&mut self, out: &mut TgOut, k: u32) {
        match self.class {
            Class::Zoom | Class::Pan => {
                self.result = if self.class == Class::Zoom { Result_::Zoom } else { Result_::Pan };
                self.pinch_end(out, k, 0);
                self.suppress();
            }
            Class::Swipe => self.swipe_lift(out, k),
            Class::Pending => {
                if self.slow {
                    self.reject(out, k, TgReject::Late);
                } else if self.t[self.p[0]].far > self.thr.tap || self.t[self.p[1]].far > self.thr.tap {
                    self.reject(out, k, TgReject::Short);
                } else if over(self.first_k, k, TAP_MS) {
                    self.reject(out, k, TgReject::Slow);
                } else {
                    self.lift_k = k;
                    self.ph = Ph::TapWait;
                }
            }
        }
    }

    /// The tap's last finger lifted.
    fn tap_lift(&mut self, out: &mut TgOut, k: u32) {
        if over(self.lift_k, k, TAP_LIFT_MS) || over(self.first_k, k, TAP_MS) {
            self.reject(out, k, TgReject::Slow);
        } else if self.d0 < self.thr.apart_min || self.d0 > self.thr.apart_max {
            self.reject(out, k, TgReject::Apart);
        } else {
            self.accept(out, k, G_2_TAP);
        }
    }

    fn lifts(&mut self, out: &mut TgOut, k: u32) {
        let lifted = |t: &Track| t.on && !t.seen;
        let pair_lifted = |g: &TouchGest| lifted(&g.t[g.p[0]]) || lifted(&g.t[g.p[1]]);
        match self.ph {
            Ph::One if lifted(&self.t[self.one]) => {
                if self.one_on {
                    let mut e = self.last;
                    e.type_ = InType::TouchUp;
                    e.value = self.flags(self.one);
                    self.push(out, e, k);
                    self.one_on = false;
                }
                self.ph = Ph::Idle;
            }
            Ph::Wait if lifted(&self.t[self.one]) => self.ph = Ph::Suppressed,
            Ph::Session if pair_lifted(self) => {
                let both = lifted(&self.t[self.p[0]]) && lifted(&self.t[self.p[1]]);
                self.first_lift(out, k);
                if self.ph == Ph::TapWait && both {
                    self.tap_lift(out, k);
                }
            }
            Ph::TapWait if pair_lifted(self) => self.tap_lift(out, k),
            _ => {}
        }
        // (A lifted track keeps what it was: a tap's first finger gives the gesture its end.)
        for t in self.t.iter_mut() {
            if lifted(t) {
                t.on = false;
                if t.si < TG_MAX {
                    self.sess[t.si].1 = k;
                }
            }
        }
    }

    fn landings(&mut self, pen: &TgPen, out: &mut TgOut, k: u32) {
        let mut new = [0usize; TG_MAX];
        let mut n = 0;
        let mut palms = false;
        for (i, t) in self.t.iter().enumerate() {
            if t.on && t.fresh && !t.pen {
                if t.palm {
                    palms = true;
                } else {
                    new[n] = i;
                    n += 1;
                }
            }
        }
        if n == 0 && !palms {
            return;
        }
        let new = &new[..n];
        match self.ph {
            Ph::Idle => {
                self.result = Result_::None;
                if !palms && n == 1 {
                    // One finger: its stream, unless the pen window holds it (a second finger may
                    // still make a gesture of it).
                    self.ph = Ph::One;
                    self.one = new[0];
                    self.one_on = !Self::pen_window(pen, k);
                    if self.one_on {
                        let e = self.touch_ev(InType::TouchDown, new[0]);
                        self.last = e;
                        self.push(out, e, k);
                    }
                } else if !palms {
                    self.land(new, pen, out, k);
                } else if self.rest_others(new, k) {
                    if n == 0 {
                        self.result = Result_::Palm;
                        self.ph = Ph::Suppressed; // a palm alone: nothing locked, a pair may come
                    } else {
                        self.land(new, pen, out, k);
                    }
                } else {
                    self.result = Result_::Palm;
                    self.suppress();
                }
            }
            Ph::One => {
                let a = self.one;
                if palms || n >= 3 {
                    // A palm cancels the finger and suppresses until all lift, as it always did.
                    self.cancel_one(out, k);
                    if palms {
                        self.result = Result_::Palm;
                        self.suppress();
                    } else {
                        self.reject(out, k, TgReject::Three);
                    }
                } else if n == 1 && !over(self.t[a].down_k, k, PALM_MS) {
                    self.start_session(a, new[0], pen, out, k, true);
                } else if self.rest_ok(a, new, k) {
                    // The first finger rests (a grip thumb): the new ones may make a gesture.
                    self.cancel_one(out, k);
                    self.rest_others(new, k);
                    self.land(new, pen, out, k);
                } else {
                    // A real pinch puts both fingers down together; a palm or a second hand arrives
                    // later. Late second contacts are treated as a palm.
                    self.cancel_one(out, k);
                    if n == 1 {
                        self.result = Result_::Palm;
                        self.suppress();
                    } else {
                        self.reject(out, k, TgReject::Three);
                    }
                }
            }
            Ph::Wait => {
                let b = self.one;
                let all: [usize; 2] = [b, if n > 0 { new[0] } else { b }];
                let fingers = if n > 0 { &all[..] } else { &all[..1] };
                if n >= 2 {
                    self.reject(out, k, TgReject::Three);
                } else if !self.rest_others(fingers, self.t[b].down_k) {
                    self.result = Result_::Palm; // too near to rest beside them
                    self.suppress();
                } else if n == 1 {
                    if over(self.t[b].down_k, k, PALM_MS) {
                        self.result = Result_::Palm;
                        self.suppress();
                    } else {
                        self.start_session(b, new[0], pen, out, k, false);
                    }
                }
            }
            Ph::Session => {
                // A palm far from the pair rests; one near it, a hand that moved.
                if palms {
                    let pair = self.p;
                    let near = (0..TG_MAX).any(|i| {
                        let t = &self.t[i];
                        t.on && t.fresh && t.palm && pair.iter().any(|&f| self.d(i, f) < self.thr.far)
                    });
                    if near {
                        self.reject(out, k, TgReject::Hand);
                        return;
                    }
                    for t in self.t.iter_mut() {
                        if t.on && t.fresh && t.palm {
                            t.resting = true;
                            t.sx = t.ix;
                            t.sy = t.iy;
                        }
                    }
                }
                if n == 0 || self.class == Class::Zoom {
                    return; // (a zoom under way keeps its two fingers)
                }
                // A third finger. Before the commit, one of the pair may be a resting thumb beside
                // the two others (it landed early and kept still): they are the pair from now on.
                if n == 1 && self.class == Class::Pending {
                    for w in 0..2 {
                        let (rest, other) = (self.p[w], self.p[1 - w]);
                        let first = self.t[other].down_k;
                        if self.rest_ok(rest, &[other, new[0]], first) {
                            let t = &mut self.t[rest];
                            t.resting = true;
                            t.sx = t.ix;
                            t.sy = t.iy;
                            self.pair(other, new[0]);
                            self.in_window |= Self::pen_window(pen, k);
                            return;
                        }
                    }
                }
                let late = self.count_fixed(k);
                self.reject(out, k, if late { TgReject::Late } else { TgReject::Three });
            }
            Ph::TapWait => self.reject(out, k, TgReject::Late),
            Ph::Suppressed => {
                if self.t.iter().any(|t| t.on && t.locked) || !self.rest_others(new, k) {
                    self.suppress();
                } else {
                    self.land(new, pen, out, k);
                }
            }
        }
    }

    fn work(&mut self, pen: &TgPen, out: &mut TgOut, k: u32) {
        match self.ph {
            Ph::One => {
                let a = self.one;
                let t = self.t[a];
                if t.palm || t.pen {
                    // The panel calls the finger a palm (or a pen) now: it ends without a tap.
                    self.cancel_one(out, k);
                    self.result = Result_::Palm;
                    self.suppress();
                } else if !self.one_on {
                    if at_least(t.down_k, k, LAND_MS) {
                        self.ph = Ph::Suppressed; // the pen's hand: quiet until it lifts
                    }
                } else if Self::pen_window(pen, k) {
                    self.cancel_one(out, k);
                    self.suppress();
                } else if t.c.nx != self.last.nx || t.c.ny != self.last.ny {
                    let e = self.touch_ev(InType::TouchMove, a);
                    self.last = e;
                    self.push(out, e, k);
                }
            }
            Ph::Wait => {
                let t = self.t[self.one];
                if self.rest_moved() {
                    self.reject(out, k, TgReject::Hand);
                } else if t.palm || t.pen || over(t.down_k, k, PALM_MS) {
                    self.ph = Ph::Suppressed;
                }
            }
            Ph::Session | Ph::TapWait => {
                if self.p.iter().any(|&i| self.t[i].on && (self.t[i].palm || self.t[i].pen)) {
                    // A finger the panel calls a palm now ends the session, as a palm always did.
                    self.result = Result_::Palm;
                    self.pinch_end(out, k, 0);
                    self.suppress();
                    return;
                }
                if self.rest_moved() {
                    self.reject(out, k, TgReject::Hand);
                    return;
                }
                if self.pen_near(pen, out, k) {
                    return;
                }
                if self.ph == Ph::Session {
                    self.fixed = self.count_fixed(k);
                    self.session_frame(out, k);
                } else if self.p.iter().any(|&i| self.t[i].on && self.t[i].far > self.thr.tap) {
                    self.reject(out, k, TgReject::Short);
                } else if over(self.lift_k, k, TAP_LIFT_MS) || over(self.first_k, k, TAP_MS) {
                    self.reject(out, k, TgReject::Slow);
                }
            }
            _ => {}
        }
    }
}

/// Converts every millimetre and pixel threshold for the screen's dpi, once (input_open).
pub fn touchgest_init(g: &mut TouchGest, dpi: f32) {
    g.thr = Thr::at(dpi);
}

/// TOUCH_MAJOR above t marks a contact broad (TOUCH_F_BROAD: logged only in this version). Never,
/// until this is called.
pub fn touchgest_set_palm_major(g: &mut TouchGest, t: i32) {
    g.palm_major = t;
}

pub fn touchgest_set_view(g: &mut TouchGest, v: TgView) {
    g.view = v;
}

/// Any contact down: the key scan waits (input_evdev's busy flag).
pub fn touchgest_busy(g: &TouchGest) -> bool {
    g.t.iter().any(|t| t.on)
}

/// A frame of contacts.
pub fn touchgest_frame(g: &mut TouchGest, f: &TgFrame, pen: &TgPen, out: &mut TgOut) {
    *out = TgOut::default();
    let k = f.k_ms;
    for t in g.t.iter_mut() {
        t.seen = false;
        t.fresh = false;
    }
    // (A new contact takes a free track the session does not still read: a tap's first finger, lifted,
    // gives the gesture its end.)
    let session = g.ph == Ph::Session || g.ph == Ph::TapWait;
    let p = g.p;
    if !touchgest_busy(g) && f.n > 0 {
        // A touch session starts.
        g.sess_n = 0;
        g.result = Result_::None;
    }
    for c in &f.c[..f.n.min(TG_MAX)] {
        let i = match g.t.iter().position(|t| t.on && t.id == c.id && t.slot == c.slot) {
            Some(i) => i,
            None => {
                let free = |i: &usize| !g.t[*i].on && !(session && p.contains(i));
                let Some(i) = (0..TG_MAX).find(free).or_else(|| (0..TG_MAX).find(|&i| !g.t[i].on)) else { continue };
                let (x, y) = (ipx(c.x), ipx(c.y));
                let si = g.sess_n;
                if si < TG_MAX {
                    g.sess[si] = (k, k);
                    g.sess_n += 1;
                }
                g.t[i] = Track { on: true, fresh: true, id: c.id, slot: c.slot, down_k: k, x0: x, y0: y, si, ..Default::default() };
                i
            }
        };
        let palm_major = g.palm_major;
        let t = &mut g.t[i];
        t.seen = true;
        t.c = *c;
        t.ix = ipx(c.x);
        t.iy = ipx(c.y);
        t.far = t.far.max(dist(t.x0, t.y0, t.ix, t.iy));
        t.broad |= c.major > palm_major;
        t.pen |= c.tool == 1;
        t.palm |= c.tool == 2;
    }
    if g.hold {
        g.hold = false;
        for t in g.t.iter_mut() {
            t.locked |= t.on;
        }
        if touchgest_busy(g) {
            g.ph = Ph::Suppressed;
        }
    }
    g.lifts(out, k);
    g.landings(pen, out, k);
    g.work(pen, out, k);
    if !touchgest_busy(g) {
        g.ph = Ph::Idle;
    }
}

/// The pen arrived, the kernel dropped touch events, or the Kindle took the panel or gave it back:
/// the finger's stream ends with the cancel, a session with PinchEnd, and every contact down waits to
/// lift, with every contact of the next frame (the panel's state read again after a drop or a regrab,
/// which may hold contacts this module never saw land).
pub fn touchgest_cancel(g: &mut TouchGest, k_ms: u32, out: &mut TgOut) {
    *out = TgOut::default();
    if matches!(g.ph, Ph::One | Ph::Wait | Ph::Session | Ph::TapWait) {
        g.result = Result_::Cancelled;
    }
    g.cancel_one(out, k_ms);
    g.pinch_end(out, k_ms, 0);
    for t in g.t.iter_mut() {
        t.locked |= t.on;
    }
    g.hold = true;
    g.ph = if touchgest_busy(g) { Ph::Suppressed } else { Ph::Idle };
}

/// The windows that expire with no new frame (a still pair gives none): the pen's hand's
/// LAND_MS, a lone finger's wait for its partner, the landing window, a tap's last lift.
pub fn touchgest_tick(g: &mut TouchGest, k_ms: u32, out: &mut TgOut) {
    *out = TgOut::default();
    let k = k_ms;
    match g.ph {
        Ph::One if !g.one_on && at_least(g.t[g.one].down_k, k, LAND_MS) => g.ph = Ph::Suppressed,
        Ph::Wait if over(g.t[g.one].down_k, k, PALM_MS) => g.ph = Ph::Suppressed,
        Ph::Session => g.fixed |= at_least(g.first_k, k, LAND_MS),
        Ph::TapWait if over(g.lift_k, k, TAP_LIFT_MS) || over(g.first_k, k, TAP_MS) => g.reject(out, k, TgReject::Slow),
        _ => {}
    }
}

pub fn touchgest_reject_name(r: TgReject) -> &'static str {
    match r {
        TgReject::Pen => "pen",
        TgReject::Apart => "apart",
        TgReject::Straight => "straight",
        TgReject::Short => "short",
        TgReject::Direction => "direction",
        TgReject::Hand => "hand",
        TgReject::Spread => "spread",
        TgReject::Late => "late",
        TgReject::Three => "three",
        TgReject::Keyboard => "keyboard",
        TgReject::Off => "off",
        TgReject::Slow => "slow",
    }
}

/// A gesture as gestures.ini names it after "touch." ("2.down"); "" for none.
pub fn touchgest_gesture_name(id: i32) -> &'static str {
    const K: [&str; 10] = ["2.up", "2.down", "2.left", "2.right", "2.tap", "3.up", "3.down", "3.left", "3.right", "3.tap"];
    if (G_2_UP..=G_3_TAP).contains(&id) {
        K[(id - G_2_UP) as usize]
    } else {
        ""
    }
}

/// The touch.* probes (docs/SIM.md): "touch.contacts", "touch.class", "touch.suppressed",
/// "touch.gesture", "touch.reject". False for a key it does not answer.
pub fn touchgest_probe(g: &TouchGest, key: &str, out: &mut String) -> bool {
    out.clear();
    let quiet = g.ph == Ph::Suppressed || g.ph == Ph::Wait || (g.ph == Ph::One && !g.one_on);
    match key {
        "touch.contacts" => out.push_str(&cformat!("%d", g.t.iter().filter(|t| t.on).count() as i32)),
        "touch.class" => out.push_str(match g.ph {
            Ph::Idle => "none",
            _ if quiet => "suppressed",
            Ph::Session => match g.class {
                Class::Pending => "landing",
                Class::Zoom => "zoom",
                Class::Pan => "pan",
                Class::Swipe => "swipe",
            },
            Ph::TapWait => "tap",
            _ => "one",
        }),
        "touch.suppressed" => out.push_str(if quiet { "1" } else { "0" }),
        "touch.gesture" => out.push_str(touchgest_gesture_name(g.last_gesture)),
        "touch.reject" => out.push_str(g.last_reject.map_or("", touchgest_reject_name)),
        _ => return false,
    }
    true
}

// ---- the touch log and the trace (input_evdev.rs writes them; nothing here does I/O) ----
//
// grimoire.log gets, while input is grabbed: a multi-contact frame's contacts (no places), each
// touch session's landing and lift gaps and how it went, and a single finger's TOUCH_MAJOR; the
// owner's opt-in touch-trace.txt every frame with places, the pen's coming and going (never where),
// and the gestures' outcomes. Each formatter writes nothing (and says false) when input is not
// grabbed: the Kindle's PIN screen owns the panel then.

/// "touch: k %u n %d" and, per contact, " s%d:id%d:t%d:m%d:p%d" (slot, tracking id, tool,
/// touch_major, pressure): no places.
pub fn touchgest_log_frame(f: &TgFrame, grabbed: bool, out: &mut String) -> bool {
    out.clear();
    if !grabbed {
        return false;
    }
    let mut each = String::new();
    for c in &f.c[..f.n.min(TG_MAX)] {
        each.push_str(&cformat!(" s%d:id%d:t%d:m%d:p%d", c.slot, c.id, c.tool as i32, c.major, c.pressure));
    }
    out.push_str(&cformat!("touch: k %u n %d%s", f.k_ms, f.n as i32, &each));
    true
}

/// How the last touch session went: "fired 2.down", "rejected pen", "zoom", "pan", "palm",
/// "cancelled" (the pen came, the panel dropped events), "none".
pub fn touchgest_result_name(g: &TouchGest, out: &mut String) {
    out.clear();
    match g.result {
        Result_::Fired(id) => out.push_str(&cformat!("fired %s", touchgest_gesture_name(id))),
        Result_::Rejected(r) => out.push_str(&cformat!("rejected %s", touchgest_reject_name(r))),
        Result_::Zoom => out.push_str("zoom"),
        Result_::Pan => out.push_str("pan"),
        Result_::Palm => out.push_str("palm"),
        Result_::Cancelled => out.push_str("cancelled"),
        Result_::None => out.push_str("none"),
    }
}

/// How many contacts the last touch session had (its first contact down to all lifted).
pub fn touchgest_session_contacts(g: &TouchGest) -> i32 {
    g.sess_n as i32
}

/// The last touch session, once all its contacts lifted: "touch: session %d contacts, landing gaps
/// %s ms, lift gaps %s ms: %s", the gaps from the first landing and from the first lift ("0 112").
/// False for a session of one contact.
pub fn touchgest_log_session(g: &TouchGest, grabbed: bool, out: &mut String) -> bool {
    out.clear();
    let n = g.sess_n;
    if !grabbed || n < 2 {
        return false;
    }
    let mut lifts = [0u32; TG_MAX];
    for i in 0..n {
        lifts[i] = g.sess[i].1;
    }
    let lifts = &mut lifts[..n];
    lifts.sort_by_key(|&t| t.wrapping_sub(g.sess[0].0));
    let (mut land, mut lift) = (String::new(), String::new());
    for i in 0..n {
        let sep = if i > 0 { " " } else { "" };
        land.push_str(&cformat!("%s%u", sep, g.sess[i].0.wrapping_sub(g.sess[0].0)));
        lift.push_str(&cformat!("%s%u", sep, lifts[i].wrapping_sub(lifts[0])));
    }
    let mut how = String::new();
    touchgest_result_name(g, &mut how);
    out.push_str(&cformat!("touch: session %d contacts, landing gaps %s ms, lift gaps %s ms: %s", n as i32, &land, &lift, &how));
    true
}

/// A single finger's TOUCH_MAJOR at its lift: "touch: fingertips touch_major median %d max %d over %d
/// frames".
pub fn touchgest_log_fingertips(median: i32, max: i32, frames: i32, grabbed: bool, out: &mut String) -> bool {
    out.clear();
    if !grabbed {
        return false;
    }
    out.push_str(&cformat!("touch: fingertips touch_major median %d max %d over %d frames", median, max, frames));
    true
}

// (The trace's lines are formatted with core's write!, into a String the reader reserved: the
// touch path never allocates, and these are no printf formats of the C's.)

/// The trace's header, at each open of the file (and each turn): "# grimoire touch-trace 1, rotation
/// R, WxH, dpi D".
pub fn touchgest_trace_header(rotation: i32, w: i32, h: i32, dpi: f32, out: &mut String) {
    use std::fmt::Write;
    let _ = writeln!(out, "# grimoire touch-trace 1, rotation {rotation}, {w}x{h}, dpi {}", dpi.round() as i32);
}

/// A frame: "F <k_ms> <slot>:<id>:<x>:<y>:<major>:<tool> ..." (logical px, rounded), appended.
pub fn touchgest_trace_frame(f: &TgFrame, grabbed: bool, out: &mut String) -> bool {
    use std::fmt::Write;
    if !grabbed {
        return false;
    }
    let _ = write!(out, "F {}", f.k_ms);
    for c in &f.c[..f.n.min(TG_MAX)] {
        let _ = write!(out, " {}:{}:{}:{}:{}:{}", c.slot, c.id, ipx(c.x), ipx(c.y), c.major, c.tool);
    }
    out.push('\n');
    true
}

/// The pen's coming and going: "P <k_ms> prox|down|up|leave" (never where), appended.
pub fn touchgest_trace_pen(k_ms: u32, what: &str, grabbed: bool, out: &mut String) -> bool {
    use std::fmt::Write;
    if !grabbed {
        return false;
    }
    let _ = writeln!(out, "P {k_ms} {what}");
    true
}

/// A decision: "E <k_ms> fired 2.down", "E <k_ms> rejected pen" (nothing for none), appended.
pub fn touchgest_trace_outcome(k_ms: u32, o: TgOutcome, grabbed: bool, out: &mut String) -> bool {
    use std::fmt::Write;
    if !grabbed {
        return false;
    }
    match o {
        TgOutcome::None => return false,
        TgOutcome::Fired(id) => {
            let _ = writeln!(out, "E {k_ms} fired {}", touchgest_gesture_name(id));
        }
        TgOutcome::Rejected(r) => {
            let _ = writeln!(out, "E {k_ms} rejected {}", touchgest_reject_name(r));
        }
    }
    true
}
