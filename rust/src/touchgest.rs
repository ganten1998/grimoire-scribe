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
    /// Final coordinates reported in the same SYN_REPORT as a tracking-ID release.
    pub ended_n: usize,
    pub ended: [TgContact;TG_MAX],
}

/// One column of the panel's slots, as EVIOCGMTSLOTS answers it for one ABS_MT code (`ask` is that
/// ioctl: it gets a buffer whose first int is the code and returns whether the kernel took the
/// request). The kernel writes only as many ints as the panel has slots (the smaller of that and the
/// buffer) and leaves the rest of the buffer as it was, so the buffer goes in filled with `empty`: for
/// the tracking id column that is -1 (no contact; a 0 there is a valid id, a finger that never lifts),
/// for the others 0. False, and `col` untouched, when the ioctl failed.
pub fn mt_column(ask: &mut dyn FnMut(&mut [i32; TG_MAX + 1]) -> bool, code: i32, empty: i32, col: &mut [i32; TG_MAX],
) -> bool {
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
/// latest learned hover point, logical px and its kernel time; absent on panels without a
    /// trustworthy raw-digitizer mapping
    pub hover: Option<(f32, f32, u32)>,
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
/// Record-card refusal code (zero when this call did not refuse a recorded pattern).
    pub reject_code: i32,
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
    /// Record mode keeps a zoom for the page rather than binding it as a pattern.
    Zoom,
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


// ---- one tuning table for runtime, recording and replay ----

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TgTune {
    pub land_ms: u32 ,
    pub palm_ms: u32,
    pub pen_window_ms: u32,
    pub pen_gone_ms: u32,
    pub far_mm: f32,
    pub rest_ms: u32,
    pub rest_mm: f32,
    pub commit_spread: f32,
    pub commit_px: f32,
    pub zoom_ratio: f32,
    pub zoom_opposite_mm: f32,
    pub zoom_upgrade: f32,
    pub zoom_after_pen: bool,
    pub grace_ms: u32,
    pub regrab_mm: f32,
    pub tan_axis: f32,
    pub swipe_mm: f32,
    pub finger_mm: f32,
    pub cos_min: f32,
    pub finger_ratio: f32,
    pub spread_max: f32,
    pub spread_opposite: bool,
    pub swipe_ms: u32,
    pub follow_ms: u32,
    pub tap_lift_ms: u32,
    pub tap_ms: u32,
    pub tap_mm: f32,
    pub apart_min_mm: f32,
    pub apart_max_mm: f32,
    pub palm_major: i32,
    pub hover_far_mm: f32,
    pub hover_age_ms: u32,
}

/// The beta.5 recognizer, with every new rule disabled.
pub const TG_TUNE_V1: TgTune = TgTune {
    land_ms: 150,
    palm_ms: 400,
    pen_window_ms: 1000,
    pen_gone_ms: 300,
    far_mm: 40.0,
    rest_ms: 300,
    rest_mm: 3.0,
    commit_spread: 0.10,
    commit_px: 70.0,
    zoom_ratio: 0.6,
    zoom_opposite_mm: 0.0,
    zoom_upgrade: 0.0,
    zoom_after_pen: false,
    grace_ms: 0,
    regrab_mm: 0.0,
    tan_axis: 0.577_350_3,
    swipe_mm: 20.0,
    finger_mm: 12.0,
    cos_min: 0.8,
    finger_ratio: 2.0,
    spread_max: 0.25,
    spread_opposite: false,
    swipe_ms: 1000,
    follow_ms: 0,
    tap_lift_ms: 150,
    tap_ms: 300,
    tap_mm: 5.0,
    apart_min_mm: 8.0,
    apart_max_mm: 70.0,
    palm_major: i32::MAX,
    hover_far_mm: 0.0,
    hover_age_ms: 0,
};
/// Provisional phase 3 defaults. No owner trace was supplied for the threshold study, so these
/// remain the brief's measured-log-informed starting values rather than frozen study results.
pub const TG_TUNE: TgTune = TgTune {
    land_ms: 200,
    palm_ms: 400,
    pen_window_ms: 1000,
    pen_gone_ms: 300,
    far_mm: 40.0,
    rest_ms: 300,
    rest_mm: 3.0,
    commit_spread: 0.10,
    commit_px: 70.0,
    zoom_ratio: 0.4,
    zoom_opposite_mm: 2.0,
    zoom_upgrade: 0.15,
    zoom_after_pen: true,
    grace_ms: 80,
    regrab_mm: 15.0,
    tan_axis: 0.577_350_3,
    swipe_mm: 16.0,
    finger_mm: 8.0,
    cos_min: 0.7,
    finger_ratio: 3.0,
    spread_max: 0.50,
    spread_opposite: true,
    swipe_ms: 2500,
    follow_ms: 250,
    tap_lift_ms: 200,
    tap_ms: 400,
    tap_mm: 6.0,
    apart_min_mm: 8.0,
    apart_max_mm: 80.0,
    palm_major: i32::MAX,
    hover_far_mm: 60.0,
    hover_age_ms: 50,
};

// A queued touch frame may precede the pen's leave on the same wrapping clock.
const PEN_BEHIND_MS: u32 = 10000;

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
    /// a one-finger continuation may complete a short or uncommitted two-finger swipe
    Follow,
    /// a committed zoom or pan waits briefly for the lifted finger to return
    Grace,
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

/// Integer-only snapshot for the opt-in measure line. It contains no contact coordinates.
#[derive(Clone, Copy, Debug, Default)]
struct TgMeasure {
    ready: bool,
    d0: i32,
    along: i32,
    across: i32,
    spread: i32,
    fingers: [i32; 2],
    cos: i32,
    ms: u32,
    first_lift: u32,
    pen: i32,
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
opposite: f32,
    regrab: f32,
    hover_far: f32,
}

impl Thr {
    fn at(dpi: f32, t: &TgTune) -> Thr {
        // (ui_px's scale: 300 dpi is 1)
        let scale = if dpi > 50.0 { dpi / 300.0 } else { 1.0 };
        Thr {
            commit: (t.commit_px * scale).round_ties_even(),
            rest: mm_to_px(t.rest_mm, dpi),
            tap: mm_to_px(t.tap_mm, dpi),
            apart_min: mm_to_px(t.apart_min_mm, dpi),
            apart_max: mm_to_px(t.apart_max_mm, dpi),
            finger: mm_to_px(t.finger_mm, dpi),
            swipe: mm_to_px(t.swipe_mm, dpi),
            far: mm_to_px(t.far_mm, dpi),
            opposite: mm_to_px(t.zoom_opposite_mm, dpi),
            regrab: mm_to_px(t.regrab_mm, dpi),
            hover_far: mm_to_px(t.hover_far_mm, dpi),
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
    p: [usize; 3],
    p_n: usize,
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
    /// original two-finger spread, retained for measurement across a re-grab
    measure_d0: f32,
    mx0: f32,
    my0: f32,
    /// PinchBegin went out and its PinchEnd has not
    begun: bool,
    /// a cancel came: every contact of the next frame waits to lift (the panel's state after a drop,
    /// or the fingers still down when the Kindle hands the panel back)
    hold: bool,
    view: TgView,
    tune: TgTune,
    dpi: f32,
    pen_age: i32,
    measure_lift: Option<u32>,
    follow_i: usize,
    follow_start: (i32, i32),
    follow_offset: (f32, f32),
    follow_mid0: (f32, f32),
    follow_deadline: u32,
    grace_i: usize,
    grace_at: (i32, i32),
    grace_lift_k: u32,
    slots: usize,
    regrabbed: bool,
    measure: TgMeasure,
    last_k: u32,
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
            p: [0; 3],
            p_n: 0,
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
            measure_d0: 0.0,
            mx0: 0.0,
            my0: 0.0,
            begun: false,
            hold: false,
            view: TgView::default(),
            tune: TG_TUNE,
            dpi: 300.0,
            pen_age: -1,
            measure_lift: None,
            follow_i: 0,
            follow_start: (0, 0),
            follow_offset: (0.0, 0.0),
            follow_mid0: (0.0, 0.0),
            follow_deadline: 0,
            grace_i: 0,
            grace_at: (0, 0),
            grace_lift_k: 0,
            slots: 2,
            regrabbed: false,
            measure: TgMeasure::default(),
            last_k: 0,
            thr: Thr::at(300.0, &TG_TUNE),
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

    /// The pen is near, or left less than self.tune.pen_window_ms ago (by the frame's time).
    fn pen_window(&self, pen: &TgPen, k: u32) -> bool {
        pen.prox || pen.touching || Self::gone_within(pen, k, self.tune.pen_window_ms)
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
            self.push(out, InEvent { type_: InType::PinchEnd, value, ..Default::default() }, k,
            );
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
        out.reject_code = touchgest_reject_code(r);
        out.outcome = TgOutcome::Rejected(r);
        self.result = Result_::Rejected(r);
        if self.view.record {
            self.push(
                out,
                InEvent {
                    type_: InType::Gesture,
                    value: out.reject_code,
                    ..Default::default()
                },
                k,
            );
        }
        self.suppress();
    }

    /// The gesture fires, unless gestures are off, a keyboard is up, or the pen was near.
    fn accept(&mut self, out: &mut TgOut, k: u32, id: i32) {
        if !self.view.record && !self.view.gestures_on {
            self.reject(out, k, TgReject::Off);
        } else if !self.view.record && self.view.keyboard_open {
            self.reject(out, k, TgReject::Keyboard);
        } else if self.blocked || self.pen_close {
            self.reject(out, k, TgReject::Pen);
        } else {
            self.pinch_end(out, k, 0);
            let (mut sx, mut sy, mut ex, mut ey) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for &i in &self.p[..self.p_n] {
                let t = &self.t[i];
                sx += t.fx;
                sy += t.fy;
                ex += t.c.x;
                ey += t.c.y;
            }
            let n = self.p_n.max(1)as f32;
            let e = InEvent {
                type_: InType::Gesture,
                value: id,
                x: sx / n,
                y: sy / n,
                x2: ex / n,
                y2: ey / n,
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
    /// still for self.tune.rest_ms before; either way FAR_MM from each of the fingers, and not locked.
    fn rest_ok(&self, i: usize, fingers: &[usize], first_k: u32) -> bool {
        let t = &self.t[i];
        !t.locked
            && (t.palm || (t.far <= self.thr.rest && at_least(t.down_k, first_k, self.tune.rest_ms)))
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
        self.fixed || at_least(self.first_k, k, self.tune.land_ms) || p.far >= self.thr.rest || q.far >= self.thr.rest
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
            3 if self.slots >= 3 => self.start_three(fingers, pen, out, k),
            _ => self.reject(out, k, TgReject::Three),
        }
    }

    fn promote_three(&mut self, third: usize, pen: &TgPen, out: &mut TgOut, k: u32) {
        let old = [self.p[0], self.p[1]];
        self.pinch_end(out, k, 0);
        self.p = [old[0], old[1], third];
        self.p_n = 3;
        let t = &mut self.t[third];
        t.sx = t.x0;
        t.sy = t.y0;
        t.fx = t.c.x;
        t.fy = t.c.y;
        self.mx0 = (self.t[old[0]].sx + self.t[old[1]].sx + self.t[third].sx) as f32 / 3.0;
        self.my0 = (self.t[old[0]].sy + self.t[old[1]].sy + self.t[third].sy) as f32 / 3.0;
        self.d0 = 0.0;
        self.class = Class::Pending;
        self.axis_h = false;
        self.slow = false;
        self.fixed = false;
        self.inert = false;
        self.in_window |= self.pen_window(pen, k);
        self.begun = false;
    }

    /// The session's geometry from its fingers as they are now (its start, or a new pair).
    fn fingers(&mut self, fingers: &[usize]) {
        let mut order = [0usize; 3];
        let n = fingers.len().min(order.len());
        order[..n].copy_from_slice(&fingers[..n]);
        // At most three items; keep the fixed-size recognizer allocation-free.
        for i in 1..n {
            let mut j = i;
            while j > 0 {
                let (a, b) = (order[j - 1], order[j]);
                let ta = &self.t[a];
                let tb = &self.t[b];
        let a_first = (ta.down_k.wrapping_sub(tb.down_k) as i32) < 0 || (ta.down_k == tb.down_k && ta.slot <= tb.slot);
        if a_first {
                    break;
                }
                order.swap(j - 1, j);
                j -= 1;
            }
        }
        self.p = order;
        self.p_n = n;
        self.first_k = self.t[self.p[0]].down_k;
        self.slow = n == 2 && over(self.first_k, self.t[self.p[1]].down_k, self.tune.land_ms);
        self.fixed = self.slow;
        for &i in &self.p [..n] {
            let t = &mut self.t[i];
            t.sx = t.ix;
            t.sy = t.iy;
            t.fx = t.c.x;
            t.fy = t.c.y;
        }
        let (p, q) = (&self.t[self.p[0]], &self.t[self.p[1]]);
        self.d0 = if n == 2 {
            dist(p.ix, p.iy, q.ix, q.iy)} else {
            0.0
        };
        let (mut mx, mut my) = (0.0f32, 0.0f32);
        for &i in &self.p[..n] {
            mx += self.t[i].ix as f32;
            my += self.t[i].iy as f32;
        }
        self.mx0 = mx / n.max(1) as f32 ;
        self.my0 = my / n.max(1) as f32 ;
        self.class = Class::Pending;
        self.inert = false;
    }

    fn pair(&mut self, a: usize, b: usize) {
        self.fingers(&[a, b]);
    }

    fn start_session(&mut self, a: usize, b: usize, pen: &TgPen, out: &mut TgOut, k: u32, from_one: bool,
    ) {
        if from_one {
            self.cancel_one(out, k); // the first finger's press turns into the session
        }
        self.pair(a, b);
        self.measure_d0 = self.d0;
        self.measure_lift = None;
        self.measure = TgMeasure::default();
        self.pen_age = if pen.prox || pen.touching {
            -1
        } else {
            k.wrapping_sub(pen.left_ms) as i32
        };
        self.in_window = self.pen_window(pen, k);
        self.blocked = false;
        self.pen_close = false;
        self.ph = Ph::Session;
        self.begun = !self.view.record;
        if self.begun {
        let e = self.pinch_ev(InType::PinchBegin, 1);
        self.push(out, e, k);
    }

    }

    fn start_three(&mut self, fingers: &[usize], pen: &TgPen, _out: &mut TgOut, k: u32) {
        self.fingers(fingers);
        self.measure_d0 = 0.0;
        self.measure_lift = None;
        self.measure = TgMeasure::default();
        self.pen_age = if pen.prox || pen.touching {
            -1
        } else {
            k.wrapping_sub(pen.left_ms) as i32
        };
        self.in_window = self.pen_window(pen, k);
        self.blocked = false;
        self.pen_close = false;
        self.ph = Ph::Session;
        self.begun = false;
    }

    fn hover_far(&self, pen: &TgPen, k: u32) -> bool {
        if self.tune.hover_far_mm <= 0.0 || self.tune.hover_age_ms == 0 || pen.touching {
            return false;
        }
        let Some((x, y, at)) = pen.hover else {
            return false;
        };
        let age = k.wrapping_sub(at) as i32;
        if age < 0 || age as u32 > self.tune.hover_age_ms {
            return false;
        }
        let (x, y) = (ipx(x), ipx(y));
        self.p[..self.p_n].iter().all(|&i| {
            let t = &self.t[i];
            !t.on || dist(t.ix, t.iy, x, y) >= self.thr.hover_far
        })
    }

    /// The pen near a session (every frame of it): a fresh far hover sample can clear proximity;
    /// otherwise touching, proximity or a recent leave blocks it. A two-finger zoom or pan may use
    /// the separate post-writing rule when it is old and far enough.
    fn pen_near(&mut self, pen: &TgPen, out: &mut TgOut, k: u32) -> bool {
        let hover_far = self.hover_far(pen, k);
        if pen.touching || ((pen.prox || Self::gone_within(pen, k, self.tune.pen_gone_ms)) && !hover_far) {
            self.blocked = true;
            if matches!(self.ph , Ph::Session | Ph::Grace)
                && !self.inert && (self.class == Class::Zoom || self.class == Class::Pan) {
                self.result = if self.class == Class::Zoom { Result_::Zoom } else { Result_::Pan };
                self.pinch_end(out, k, 0);
                self.suppress();
                return true;
            }
        } else if Self::gone_within(pen, k, self.tune.pen_window_ms) {
            let (lx, ly) = (ipx(pen.last.0), ipx(pen.last.1));
            for &i in &self.p [..self.p_n] {
                let t = &self.t[i];
                if !hover_far && t.on && dist(t.ix, t.iy, lx, ly) < self.thr.far {
                    self.pen_close = true;
                }
            }
        }
        false
    }

    fn motion_free(&self, pen: &TgPen, k: u32) -> bool {
        if self.hover_far(pen, k) {
            return true;
        }
        if !self.in_window && !self.blocked && !self.pen_close {
            return true;
        }
        if self.tune.zoom_after_pen
            && self.p_n == 2
            && !pen.prox
            && !pen.touching
            && at_least(pen.left_ms, k, self.tune.pen_gone_ms)
        {
            let (x, y) = (ipx(pen.last.0), ipx(pen.last.1));
            return self.p[..self.p_n].iter().all(|&i| {
                let t = &self.t[i];
                !t.on || dist(t.ix, t.iy, x, y) >= self.thr.far
            });
        }
        false
    }

    fn opposite_motion(&self) -> bool {
        if self.p_n != 2 || self.thr.opposite <= 0.0 {
            return false;
        }
        let (a, b) = (self.t[self.p[0]], self.t[self.p[1]]);
        let (ax, ay) = ((a.ix - a.sx) as f32, (a.iy - a.sy) as f32);
        let (bx, by) = ((b.ix - b.sx) as f32, (b.iy - b.sy) as f32);
        let al = (ax * ax + ay * ay).sqrt();
        let bl = (bx * bx + by * by).sqrt();
        al >= self.thr.opposite && bl >= self.thr.opposite && ax * bx + ay * by < 0.0
    }

    fn is_zoom(&self, d: f32, travel: f32) -> bool {
        (self.view.zoom_ok || self.view.record)
            && self.p_n == 2
            && ((d - self.d0).abs() >= self.tune.zoom_ratio * travel || self.opposite_motion())
    }

    /// The commit's class: a zoom when the spread changed most of the travel (and the pen allows
    /// it), else a drag along its main axis: a pan on an axis the page scrolls on, refused more than
    /// 30 degrees off its axis when nothing scrolls, else a swipe (never for a late second finger).
    fn classify(&mut self, out: &mut TgOut, k: u32, pen: &TgPen, d: f32, dx: f32, dy: f32) {
        let free = self.motion_free(pen, k);
        let travel = (dx * dx + dy * dy).sqrt();
        if free && self.is_zoom(d , travel){
            self.class = Class::Zoom;
            return;
        }
        if self.p_n == 3 {
            self.class = Class::Swipe;
            self.axis_h = dx.abs() > dy.abs();
            return;
        }
        let h = dx.abs() > dy.abs();
        let (along, across) = if h { (dx.abs(), dy.abs()) } else { (dy.abs(), dx.abs()) };
        let scrolls = if h { self.view.scrolls_h } else { self.view.scrolls_v };
        self.axis_h = h;
        if across > along * self.tune.tan_axis {
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
    fn session_frame(&mut self, out: &mut TgOut, k: u32, pen: &TgPen) {
        let (mut mx, mut my) = (0.0f32, 0.0f32);
        for &i in &self.p[..self.p_n] {
            mx += self.t[i].ix as f32;
            my += self.t[i].iy as f32;
        }
        mx /= self.p_n.max(1) as f32 ;
        my /= self.p_n.max(1) as f32;
        let (dx, dy) = (mx - self.mx0, my - self.my0);
            let travel = (dx * dx + dy * dy).sqrt();
            let d = if self.p_n == 2 {
            self.d(self.p[0], self.p[1])
        } else {
            0.0
        };
        if self.class == Class::Pending {
            let spread = if self.p_n == 2 {
                (d / self.d0.max(10.0) - 1.0).abs()} else {
                0.0
            };
            if spread < self.tune.commit_spread && travel < self.thr.commit {
                return; // nothing moves until the fingers clearly spread or travel
            }
            self.classify(out, k, pen, d, dx, dy);
            if self.ph != Ph::Session {
                return;
            }
        } else if matches!(self.class, Class::Swipe | Class::Pan)
            && (self.view.zoom_ok || self.view.record)
            && self.tune.zoom_upgrade > 0.0
            && self.motion_free(pen, k)
            && self.opposite_motion()
            && (d - self.d0).abs() >= self.tune.zoom_upgrade * self.d0
        {
            self.class = Class::Zoom;
            self.inert = false;
            if !self.view.record {
                let e = self.pinch_ev(InType::PinchMove, 1);
                self.push(out, e, k);
            }
            return;
        }
        if self.view.record || self.p_n == 3 || self.regrabbed {
            return;
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

    fn midpoint(&self) -> (f32, f32) {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        for &i in &self.p[..self.p_n] {
            x += self.t[i].ix as f32;
            y += self.t[i].iy as f32;
        }
        let n = self.p_n.max(1) as f32;
        (x / n, y / n)
    }

    /// Apply the swipe thresholds to the current pair or triple.
    fn swipe_verdict(&self, k: u32) -> Result<i32, TgReject> {
        let (mx, my) = self.midpoint();
        let dx = mx - self.mx0;
        let dy = my - self.my0;
        let (along, across) = if self.axis_h { (dx, dy) } else { (dy, dx) };
        let m = (dx * dx + dy * dy).sqrt();
        let mut lens = [0f32; 3];
        let mut same_way = true;
        let mut min_apart = f32::MAX;
        let mut max_apart = 0.0f32;
        for a in 0..self.p_n {
            for b in a + 1..self.p_n {
                let d = self.d(self.p[a], self.p[b]);
                min_apart = min_apart.min(d);
                max_apart = max_apart.max(d);
            }
        }
        for (n, &i) in self.p[..self.p_n].iter().enumerate() {
            let t = self.t[i];
            let (fx, fy) = ((t.ix - t.sx) as f32, (t.iy - t.sy) as f32);
            lens[n] = (fx * fx + fy * fy).sqrt();
            same_way &= lens[n] >= self.thr.finger && (fx * dx + fy * dy) >= self.tune.cos_min * lens[n] * m;
        }
        let apart = if self.p_n == 2 { self.d0 } else { min_apart };
        let too_far = if self.p_n == 2 { self.d0 } else { max_apart };
        let spread_changed = self.p_n == 2
            && (self.d(self.p[0], self.p[1]) - self.d0).abs() > self.tune.spread_max * self.d0;
        let along_separation = if self.axis_h {
            ((self.t[self.p[0]].ix - self.t[self.p[0]].sx)
                - (self.t[self.p[1]].ix - self.t[self.p[1]].sx))
                .abs() as f32
        } else {
            ((self.t[self.p[0]].iy - self.t[self.p[0]].sy)
                - (self.t[self.p[1]].iy - self.t[self.p[1]].sy))
                .abs() as f32
        };
        let spread_reject = if !spread_changed {
            false
        } else if self.tune.spread_opposite {
            along_separation > self.tune.spread_max * self.d0
        } else {
            true
        };
        if apart < self.thr.apart_min || too_far > self.thr.apart_max {
            Err(TgReject::Apart)
        } else if along.abs() < self.thr.swipe {
            Err(TgReject::Short)
        } else if across.abs() > along.abs() * self.tune.tan_axis {
            Err(TgReject::Straight)
        } else if !same_way
            || lens[..self.p_n].iter().copied().fold(0.0f32, f32::max)
                > self.tune.finger_ratio * lens[..self.p_n].iter().copied().fold(f32::MAX, f32::min)
        {
            Err(TgReject::Direction)
        } else if spread_reject {
            Err(TgReject::Spread)
        } else if over(self.first_k, k, self.tune.swipe_ms) {
            Err(TgReject::Slow)
        } else {
            let two = match (self.axis_h, along < 0.0) {
                (true, true) => G_2_LEFT,
                (true, false) => G_2_RIGHT,
                (false, true) => G_2_UP,
                (false, false) => G_2_DOWN,
        };
            let three = match (self.axis_h, along < 0.0) {
                (true, true) => G_3_LEFT,
                (true, false) => G_3_RIGHT,
                (false, true) => G_3_UP,
                (false, false) => G_3_DOWN,
            };
            Ok(if self.p_n == 3 { three } else { two })
        }
    }

    fn start_follow(&mut self, remaining: usize, k: u32) {
        let (mx, my) = self.midpoint();
        let t = &self.t[remaining];
        if self.class == Class::Pending {
            let (dx, dy) = ((t.ix - t.sx) as f32, (t.iy - t.sy) as f32);
            self.axis_h = dx.abs() > dy.abs();
            self.class = Class::Swipe;
        }
        self.follow_i = remaining;
        self.follow_start = (t.sx, t.sy);
        self.follow_offset = (mx - t.ix as f32, my - t.iy as f32);
        self.follow_mid0 = (self.mx0, self.my0);
        self.follow_deadline = k.wrapping_add(self.tune.follow_ms);
        self.ph = Ph::Follow;
    }

    fn follow_verdict(&self, k: u32) -> Result<i32, TgReject> {
        let t = &self.t[self.follow_i];
        let (mx, my) = (
            t.ix as f32 + self.follow_offset.0,
            t.iy as f32 + self.follow_offset.1,
        );
        let (dx, dy) = (mx - self.follow_mid0.0, my - self.follow_mid0.1);
        let (along, across) = if self.axis_h { (dx, dy) } else { (dy, dx) };
        let (fx, fy) = (
            (t.ix - self.follow_start.0) as f32,
            (t.iy - self.follow_start.1) as f32,
        );
        let len = (fx * fx + fy * fy).sqrt();
        let m = (dx * dx + dy * dy).sqrt();
        if self.d0 < self.thr.apart_min || self.d0 > self.thr.apart_max {
            Err(TgReject::Apart)
        } else if along.abs() < self.thr.swipe {
            Err(TgReject::Short)
        } else if across.abs() > along.abs() * self.tune.tan_axis {
            Err(TgReject::Straight)
        } else if len < self.thr.finger || fx * dx + fy * dy < self.tune.cos_min * len * m {
            Err(TgReject::Direction)
        } else if over(self.first_k, k, self.tune.swipe_ms) {
            Err(TgReject::Slow)
        } else {
                let id = match (self.axis_h, along < 0.0) {
                    (true, true) => G_2_LEFT,
                    (true, false) => G_2_RIGHT,
                    (false, true) => G_2_UP,
                    (false, false) => G_2_DOWN,
                };
                Ok(id)
        }
    }

    fn finish_follow(&mut self, out: &mut TgOut, k: u32, force: bool) {
        let lifted = !self.t[self.follow_i].on || !self.t[self.follow_i].seen;
        let timed_out =
            over(self.follow_deadline, k, 0) || over(self.first_k, k, self.tune.swipe_ms);
        match self.follow_verdict(k) {
            Ok(id) => self.accept(out, k, id),
            Err(TgReject::Short) if !force && !lifted && !timed_out => {}
            Err(TgReject::Short) if over(self.first_k, k, self.tune.swipe_ms) => {
                self.reject(out, k, TgReject::Slow)
            }
            Err(r) if force || lifted || timed_out => self.reject(out, k, r),
            Err(_) => {}
            }
        }

    fn start_grace(&mut self, lifted_pos: usize, k: u32) {
        self.grace_i = lifted_pos;
        self.grace_at = (self.t[self.p[lifted_pos]].ix, self.t[self.p[lifted_pos]].iy);
        self.grace_lift_k = k;
        self.ph = Ph::Grace;
        self.result = if self.class == Class::Zoom {
            Result_::Zoom
        } else {
            Result_::Pan
        };
            }
        fn end_grace(&mut self, out: &mut TgOut) {
        self.pinch_end(out, self.grace_lift_k, 0);
        self.suppress();
    }

    /// A swipe's first lift, or its continued one-finger tail.
    fn swipe_lift(&mut self, out: &mut TgOut, k: u32, can_follow: bool) {
        match self.swipe_verdict(k) {
            Err(TgReject::Short) if can_follow && self.tune.follow_ms > 0 => {
                let lifted = self.p[..self.p_n]
                    .iter()
                    .position(|&i| self.t[i].on && !self.t[i].seen);
                let remaining = self.p[..self.p_n]
                    .iter()
                    .copied()
                    .find(|&i| self.t[i].on && self.t[i].seen);
                if self.p_n == 2 {
                    if let Some(r) = remaining {
                        if self.class != Class::Pending || self.t[r].far > self.thr.tap {
                            self.start_follow(r, k);
                            return;
                        }
                    }
                }
                if lifted.is_none() {
                    return;
                }
                self.reject(out, k, TgReject::Short);
            }
            Err(r) => self.reject(out, k, r),
            Ok(id) => self.accept(out, k, id),
        }
    }

    /// One of the session's fingers lifted: a zoom or pan ends, a swipe fires or is refused, and an
    /// uncommitted session may be a tap.
    fn first_lift(&mut self, out: &mut TgOut, k: u32, all_lifted: bool) {
        self.measure_lift.get_or_insert(k);
        match self.class {
            Class::Zoom if self.view.record => self.reject(out, k, TgReject::Zoom),
            Class::Pan if self.view.record => self.swipe_lift(out, k, false),
            Class::Zoom | Class::Pan if self.tune.grace_ms > 0 && !all_lifted && self.p_n == 2 => {
                let lifted_pos = self.p[..self.p_n]
                    .iter()
                    .position(|&i| self.t[i].on && !self.t[i].seen)
                    .unwrap_or(0);
                self.start_grace(lifted_pos, k);
            }
            Class::Zoom | Class::Pan => {
                self.result = if self.class == Class::Zoom { Result_::Zoom } else { Result_::Pan };
                self.pinch_end(out, k, 0);
                self.suppress();
            }
            Class::Swipe => self.swipe_lift(out, k, true),
            Class::Pending => {
                if !all_lifted && self.p_n == 2 && self.tune.follow_ms > 0 {
                    if let Some(r) = self.p[..self.p_n]
                        .iter()
                        .copied()
                        .find(|&i| self.t[i].on && self.t[i].seen && self.t[i].far > self.thr.tap)
                    {
                        self.start_follow(r, k);
                        return;
                    }
                }
                if self.slow {
                    self.reject(out, k, TgReject::Late);
                } else if self.p[..self.p_n]
                    .iter()
                    .any(|&i| self.t[i].far > self.thr.tap )
                {
                    self.reject(out, k, TgReject::Short);
                } else if over(self.first_k, k, self.tune.tap_ms) {
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
        if over(self.lift_k, k, self.tune.tap_lift_ms) || over(self.first_k, k, self.tune.tap_ms) {
            self.reject(out, k, TgReject::Slow);
        } else if self.p_n == 2 && (self.d0 < self.thr.apart_min || self.d0 > self.thr.apart_max ) {
            self.reject(out, k, TgReject::Apart);
        } else if self.p_n == 3 && self.swipe_verdict(k) == Err(TgReject::Apart) {
            self.reject(out, k, TgReject::Apart);
        } else {
            self.accept(out, k, if self.p_n == 3 { G_3_TAP } else { G_2_TAP});
        }
    }

    fn lifts(&mut self, out: &mut TgOut, k: u32) {
        let lifted = |t: &Track| t.on && !t.seen;
        let pair_lifted = |g: &TouchGest| g.p[..g.p_n].iter() .any(|&i| lifted(&g.t[i]));
        let all_pair_lifted = |g: &TouchGest| g.p[..g.p_n].iter().all(|&i| lifted(&g.t[i]));
        match self.ph {
            Ph::One if lifted(&self.t[self.one]) => {
                if self.one_on {
                    let mut e = self.last;
                    e.type_ = InType::TouchUp;
                    e.value = self.flags(self.one);
                    self.push(out, e, k);
                    self.one_on = false;
                }
                else if self.view.record {
                    self.reject(out, k, TgReject::Pen);
                }
                self.ph = Ph::Idle;
            }
            Ph::Wait if lifted(&self.t[self.one]) => self.ph = Ph::Suppressed,
            Ph::Session if pair_lifted(self) => {
                let all = all_pair_lifted(self);
                self.first_lift(out, k, all);
                if self.ph == Ph::TapWait && all {
                    self.tap_lift(out, k);
                }
            }
            Ph::TapWait if pair_lifted(self) => self.tap_lift(out, k),
            Ph::Follow if lifted(&self.t[self.follow_i]) => self.finish_follow(out, k, true),
            Ph::Grace if pair_lifted(self) => self.end_grace(out),
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
                    self.one_on = !self.pen_window(pen, k);
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
                if !palms && n == 2 && self.slots >= 3 && !over(self.t[a].down_k, k, 250) {
                    self.cancel_one(out, k);
                    self.start_three(&[a, new[0], new[1]], pen, out, k);
                } else if palms || n >= 3 {
                    // A palm cancels the finger and suppresses until all lift, as it always did.
                    self.cancel_one(out, k);
                    if palms {
                        self.result = Result_::Palm;
                        self.suppress();
                    } else {
                        self.reject(out, k, TgReject::Three);
                    }
                } else if n == 1 && !over(self.t[a].down_k, k, self.tune.palm_ms) {
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
                if !palms && n == 2 && self.slots >= 3 && !over(self.t[b].down_k, k, 250) {
                    if self.rest_others(&[b, new[0], new[1]], self.t[b].down_k) {
                        self.start_three(&[b, new[0], new[1]], pen, out, k);
                    } else {
                        self.result = Result_::Palm;
                        self.suppress();
                    }
                    return;
                }
                let all: [usize; 2] = [b, if n > 0 { new[0] } else { b }];
                let fingers = if n > 0 { &all[..] } else { &all[..1] };
                if n >= 2 {
                    self.reject(out, k, TgReject::Three);
                } else if !self.rest_others(fingers, self.t[b].down_k) {
                    self.result = Result_::Palm; // too near to rest beside them
                    self.suppress();
                } else if n == 1 {
                    if over(self.t[b].down_k, k, self.tune.palm_ms) {
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
                // Preserve the legacy two-slot zoom: incidental extra contacts stay outside its pair.
                if self.class == Class::Zoom && self.slots < 3 { return; }
                if n == 1 && self.p_n == 2 && self.slots >= 3 && !over(self.first_k, k, 250) {
                    self.promote_three(new[0], pen, out, k);
                    return;
                }
                if n == 0 {
                    return; // (a zoom under way keeps its two fingers)
                }
                // A third finger. Before the commit, one of the pair may be a resting thumb beside
                // the two others (it landed early and kept still): they are the pair from now on.
                if n == 1 && self.p_n == 2 && self.class == Class::Pending {
                    for w in 0..2 {
                        let (rest, other) = (self.p[w], self.p[1 - w]);
                        let first = self.t[other].down_k;
                        if self.rest_ok(rest, &[other, new[0]], first) {
                            let t = &mut self.t[rest];
                            t.resting = true;
                            t.sx = t.ix;
                            t.sy = t.iy;
                            self.pair(other, new[0]);
                            self.in_window |= self.pen_window(pen, k);
                            return;
                        }
                    }
                }
                let late = self.count_fixed(k);
                self.reject(out, k, if late { TgReject::Late } else { TgReject::Three },
                );
            }
            Ph::TapWait => self.reject(out, k, TgReject::Late),
            Ph::Follow => self.reject(out, k, TgReject::Three),
            Ph::Grace => {
                if over(self.grace_lift_k, k, self.tune.grace_ms) {
                    self.end_grace(out);
                    return;
                }
                let (mut best, mut best_d) = (None, f32::MAX);
                for &i in new {
                    let d = dist(self.grace_at.0, self.grace_at.1, self.t[i].ix, self.t[i].iy);
                    if d < best_d {
                        best = Some(i);
                        best_d = d;
                    }
                }
                if let Some(i) = best.filter(|_| best_d <= self.thr.regrab) {
                    self.p[self.grace_i] = i;
                    let (a, b) = (self.p[0], self.p[1]);
                    for j in [a, b] {
                        let t = &mut self.t[j];
                        t.sx = t.ix;
                        t.sy = t.iy;
                        t.fx = t.c.x;
                        t.fy = t.c.y;
                    }
                    self.d0 = self.d(a, b);
                    self.mx0 = (self.t[a].ix + self.t[b].ix) as f32 * 0.5;
                    self.my0 = (self.t[a].iy + self.t[b].iy) as f32 * 0.5;
                    self.ph = Ph::Session;
                    self.begun = true;
                    self.regrabbed = true;
                    let value = if self.class == Class::Pan { 2 } else { 1 };
                    let e = self.pinch_ev(InType::PinchBegin, value);
                    self.push(out, e, k);
                    for &extra in new.iter().filter(|&&j| j != i) {
                        self.t[extra].locked = true;
                    }
                } else {
                    self.end_grace(out);
                }
            }
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
                    if at_least(t.down_k, k, self.tune.land_ms) {
                        self.ph = Ph::Suppressed; // the pen's hand: quiet until it lifts
                    }
                } else if self.pen_window(pen, k) {
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
                } else if t.palm || t.pen || over(t.down_k, k, self.tune.palm_ms) {
                    self.ph = Ph::Suppressed;
                }
            }
            Ph::Session | Ph::TapWait => {
                if self.p[..self.p_n]
                    .iter().any(|&i| self.t[i].on && (self.t[i].palm || self.t[i].pen)) {
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
                    self.session_frame(out, k, pen);
                } else if self.p[..self.p_n]
                    .iter().any(|&i| self.t[i].on && self.t[i].far > self.thr.tap) {
                    self.reject(out, k, TgReject::Short);
                } else if over(self.lift_k, k, self.tune.tap_lift_ms) || over(self.first_k, k, self.tune.tap_ms) {
                    self.reject(out, k, TgReject::Slow);
                }
            }
            Ph::Follow => {
                if self.t[self.follow_i].palm || self.t[self.follow_i].pen {
                    self.reject(out, k, TgReject::Hand);
                    return;
                }
                if self.rest_moved() {
                    self.reject(out, k, TgReject::Hand);
                    return;
                }
                let _ = self.pen_near(pen, out, k);
                self.finish_follow(out, k, false);
            }
            Ph::Grace => {
                if self.pen_near(pen, out, k) {
                    return;
                }
                if over(self.grace_lift_k, k, self.tune.grace_ms) {
                    self.end_grace(out);
                }
            }
            _ => {}
        }
    }

    fn measure_finish(&mut self, k: u32, pen: &TgPen) {
        let (a, b) = (self.t[self.p[0]], self.t[self.p[1]]);
        let (dx, dy) = (
            ((a.ix + b.ix) as f32 * 0.5) - self.mx0,
            ((a.iy + b.iy) as f32 * 0.5) - self.my0,
        );
        let horizontal = if matches!(self.class, Class::Swipe | Class::Pan) {
            self.axis_h
        } else {
            dx.abs() > dy.abs()
        };
        let (along, across) = if horizontal {
            (dx.abs(), dy.abs())
        } else {
            (dy.abs(), dx.abs())
        };
        let d = dist(a.ix, a.iy, b.ix, b.iy);
        let mm10 = |px: f32| (px * (254.0 / self.dpi.max(1.0))).round_ties_even() as i32;
        let (ax, ay) = ((a.ix - a.sx) as f32, (a.iy - a.sy) as f32);
        let (bx, by) = ((b.ix - b.sx) as f32, (b.iy - b.sy) as f32);
        let m = (dx * dx + dy * dy).sqrt();
        let cosine = |x: f32, y: f32| {
            let l = (x * x + y * y).sqrt();
            if l == 0.0 || m == 0.0 {
                0.0
            } else {
                ((x * dx + y * dy) / (l * m) * 100.0).clamp(-100.0, 100.0)
            }
        };
        let since_left =
            if pen.prox || pen.touching || Self::gone_within(pen, k, self.tune.pen_gone_ms) {
                -1
            } else {
                k.wrapping_sub(pen.left_ms) as i32
            };
        let first = self.measure_lift.unwrap_or(k);
        self.measure = TgMeasure {
            ready: true,
            d0: mm10(self.measure_d0),
            along: mm10(along),
            across: mm10(across),
            spread: if self.measure_d0 > 0.0 {
                ((d - self.measure_d0) * 100.0 / self.measure_d0).round_ties_even() as i32
            } else {
                0
            },
            fingers: [
                mm10((ax * ax + ay * ay).sqrt()),
                mm10((bx * bx + by * by).sqrt()),
            ],
            cos: cosine(ax, ay).min(cosine(bx, by)).round_ties_even() as i32,
            ms: k.wrapping_sub(self.first_k),
            first_lift: first.wrapping_sub(self.first_k),
            pen: since_left,
        };
    }
}

/// Converts every millimetre and pixel threshold for the screen's dpi, once (input_open).
pub fn touchgest_init(g: &mut TouchGest, dpi: f32) {
    g.dpi = if dpi > 50.0 { dpi } else { 300.0 };
    touchgest_set_tune(g, &TG_TUNE);
}

/// Apply tuning without resetting the current input stream. Pixel thresholds are derived once.
pub fn touchgest_set_tune(g: &mut TouchGest, t: &TgTune) {
    g.tune = *t;
    g.thr = Thr::at(g.dpi, t);
}

pub fn touchgest_tune(g: &TouchGest) -> TgTune {
    g.tune
}

/// Set the number of contact slots the physical panel can report. Two is the conservative default.
pub fn touchgest_set_slots(g: &mut TouchGest, n: usize) {
    g.slots = n.min(TG_MAX);
}

/// TOUCH_MAJOR above t marks a contact broad (TOUCH_F_BROAD: logged only in this version). Never,
/// until this is called.
pub fn touchgest_set_palm_major(g: &mut TouchGest, t: i32) {
    g.tune.palm_major = t;
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
    g.regrabbed = false;
    let k = f.k_ms;
    g.last_k = k;
    for t in g.t.iter_mut() {
        t.seen = false;
        t.fresh = false;
    }
    // (A new contact takes a free track the session does not still read: a tap's first finger, lifted,
    // gives the gesture its end.)
    let session = matches!(g.ph , Ph::Session |Ph::TapWait| Ph::Follow | Ph::Grace);
    if !touchgest_busy(g) && f.n > 0 {
        // A touch session starts.
        g.sess_n = 0;
        g.result = Result_::None;
    }
    for c in &f.c[..f.n.min(TG_MAX)] {
        let i = match g.t.iter().position(|t| t.on && t.id == c.id && t.slot == c.slot) {
            Some(i) => i,
            None => {
                let free = |i: &usize| !g.t[*i].on && !(session && g.p[..g.p_n].contains(i));
                let Some(i) = (0..TG_MAX).find(free).or_else(|| (0..TG_MAX).find(|&i| !g.t[i].on)) else { continue ;
                };
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
        let palm_major = g.tune.palm_major;
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
    // A fast stroke can report its final movement and tracking-ID release together. The
    // released contacts are no longer down, but their last coordinates still decide the swipe.
    if matches!(g.ph,Ph::Session|Ph::Follow|Ph::Grace|Ph::TapWait) {
        for c in &f.ended[..f.ended_n.min(TG_MAX)] {
            if let Some(t)=g.t.iter_mut().find(|t|t.on && t.id==c.id && t.slot==c.slot && !t.seen) {
                t.c=*c;t.ix=ipx(c.x);t.iy=ipx(c.y);
                t.far=t.far.max(dist(t.x0,t.y0,t.ix,t.iy));
                t.broad|=c.major>g.tune.palm_major;
                t.pen|=c.tool==1;t.palm|=c.tool==2;
            }
        }
        if f.ended_n>0 && g.ph==Ph::Session && g.p[..g.p_n].iter().any(|&i|g.t[i].on && !g.t[i].seen)
            && !g.pen_near(pen,out,k) {
            if g.p[..g.p_n].iter().any(|&i|g.t[i].pen || g.t[i].palm) {g.reject(out,k,TgReject::Hand);}
            else {g.session_frame(out,k,pen);}
        }
    }
    g.lifts(out, k);
    g.landings(pen, out, k);
    g.work(pen, out, k);
    if !touchgest_busy(g) && g.p_n == 2 && g.measure_lift.is_some() && !g.measure.ready {
        g.measure_finish(k, pen);
    }
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
    if matches!(g.ph, Ph::One | Ph::Wait | Ph::Session | Ph::TapWait| Ph::Follow | Ph::Grace
    ) {
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
        Ph::One if !g.one_on && at_least(g.t[g.one].down_k, k, g.tune.land_ms) => {
            g.ph = Ph::Suppressed}
        Ph::Wait if over(g.t[g.one].down_k, k, g.tune.palm_ms) => g.ph = Ph::Suppressed,
        Ph::Session => g.fixed |= at_least(g.first_k, k, g.tune.land_ms),
        Ph::TapWait if over(g.lift_k, k, g.tune.tap_lift_ms) || over(g.first_k, k, g.tune.tap_ms) => {
            g.reject(out, k, TgReject::Slow)
        }
        Ph::Follow if over(g.follow_deadline, k, 0) || over(g.first_k, k, g.tune.swipe_ms) => {
            g.finish_follow(out, k, true)
        }
        Ph::Grace if over(g.grace_lift_k, k, g.tune.grace_ms) => g.end_grace(out),
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
    TgReject::Zoom => "zoom",
    }
}

/// Record-card refusal codes. -8 belongs to the card's own one-finger tap/link message.
pub fn touchgest_reject_code(r: TgReject) -> i32 {
    match r {
        TgReject::Pen => -1,
        TgReject::Late => -2,
        TgReject::Straight => -3,
        TgReject::Short => -4,
        TgReject::Direction | TgReject::Spread => -5,
        TgReject::Hand | TgReject::Off | TgReject::Keyboard => -6,
        TgReject::Zoom => -7,
        TgReject::Apart => -9,
        TgReject::Slow => -10,
        TgReject::Three => -11,
    }
}

/// A gesture as gestures.ini names it after "touch." ("2.down"); "" for none.
pub fn touchgest_gesture_name(id: i32) -> &'static str {
    const K: [&str; 10] = ["2.up", "2.down", "2.left", "2.right", "2.tap", "3.up", "3.down", "3.left", "3.right", "3.tap",
    ];
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
        "touch.contacts" => {
            out.push_str(&cformat!("%d", g.t.iter().filter(|t| t.on).count() as i32))}
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
        "touch.tune" => out.push_str(if g.tune == TG_TUNE_V1 { "v1" } else { "new" }),
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

/// Measurement companion to `touchgest_log_session`. Values are integer tenths of a millimetre,
/// except spread (signed percent), cosine (0..100), elapsed milliseconds, and pen age (-1 near).
/// It contains no coordinates and is silent when input is not grabbed.
pub fn touchgest_log_measure(g: &TouchGest, grabbed: bool, out: &mut String) -> bool {
    use std::fmt::Write;
    out.clear();
    if !grabbed || !g.measure.ready || g.p_n != 2 {
        return false;
    }
    let m = g.measure;
    let _ = write!(
        out,
        "touch: measure d0 {} along {} across {} spread {}% fingers {} {} cos {} ms {} first-lift {} pen {}",
        m.d0, m.along, m.across, m.spread, m.fingers[0], m.fingers[1], m.cos, m.ms, m.first_lift, m.pen
    );
    true
}

/// A single finger's TOUCH_MAJOR at its lift: "touch: fingertips touch_major median %d max %d over %d
/// frames".
pub fn touchgest_log_fingertips(median: i32, max: i32, frames: i32, grabbed: bool, out: &mut String,
) -> bool {
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
/// Pen marker in a trace. The trace deliberately contains no pen coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TgTracePen {
    Prox,
    Down,
    Up,
    Leave,
}

/// A parsed touch trace line. Frame normalized coordinates are zero because calibration is not
/// stored in the trace; callers that need them can scale from the parsed header dimensions.
#[derive(Clone, Copy, Debug)]
pub enum TgTraceLine<'a> {
    Ignore,
    Header { rotation: i32, width: i32, height: i32, dpi: i32 },
    Frame(TgFrame),
    Pen { k_ms: u32, what: TgTracePen },
    Outcome { k_ms: u32, text: &'a str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TgTraceParseError {
    BadHeader,
    BadFrame,
    BadPen,
    BadOutcome,
}

/// Parse one `touch-trace.txt` line without allocating. F preserves logical contact positions, P
/// preserves only pen state and timing, and E text is borrowed so the runner can compare it.
pub fn touchgest_trace_parse_line(line: &str) -> Result<TgTraceLine<'_>, TgTraceParseError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(TgTraceLine::Ignore);
    }
    if line.starts_with('#') {
        if !line.starts_with("# grimoire touch-trace") {
            return Ok(TgTraceLine::Ignore);
        }
        let mut words = line.split_ascii_whitespace();
        let valid = words.next() == Some("#")
            && words.next() == Some("grimoire")
            && words.next() == Some("touch-trace")
            && words.next() == Some("1,")
            && words.next() == Some("rotation");
        if !valid { return Err(TgTraceParseError::BadHeader); }
        let rotation = words.next().and_then(|s| s.strip_suffix(',')).and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadHeader)?;
        let dims = words.next().ok_or(TgTraceParseError::BadHeader)?;
        let (w, h) = dims.split_once('x').ok_or(TgTraceParseError::BadHeader)?;
        let width = w.parse().map_err(|_| TgTraceParseError::BadHeader)?;
        let height = h.strip_suffix(',').ok_or(TgTraceParseError::BadHeader)?.parse().map_err(|_| TgTraceParseError::BadHeader)?;
        if words.next() != Some("dpi") { return Err(TgTraceParseError::BadHeader); }
        let dpi = words.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadHeader)?;
        if words.next().is_some() { return Err(TgTraceParseError::BadHeader); }
        return Ok(TgTraceLine::Header { rotation, width, height, dpi });
    }
    let mut words = line.split_ascii_whitespace();
    match words.next() {
        Some("F") => {
            let k_ms = words.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
            let mut f = TgFrame { k_ms, ..Default::default() };
            for token in words {
                if f.n == TG_MAX { return Err(TgTraceParseError::BadFrame); }
                let mut p = token.split(':');
                let slot: i32 = p.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
                let id: i32 = p.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
                let x: i32 = p.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
                let y: i32 = p.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
                let major: i32 = p.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
                let tool: u8 = p.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadFrame)?;
                if p.next().is_some() || slot < 0 || slot as usize >= TG_MAX || tool > 2 {
                    return Err(TgTraceParseError::BadFrame);
                }
                f.c[f.n] = TgContact { id, slot, x: x as f32, y: y as f32, major, tool, ..Default::default() };
                f.n += 1;
            }
            Ok(TgTraceLine::Frame(f))
        }
        Some("P") => {
            let k_ms = words.next().and_then(|s| s.parse().ok()).ok_or(TgTraceParseError::BadPen)?;
            let what = match words.next() {
                Some("prox") => TgTracePen::Prox,
                Some("down") => TgTracePen::Down,
                Some("up") => TgTracePen::Up,
                Some("leave") => TgTracePen::Leave,
                _ => return Err(TgTraceParseError::BadPen),
            };
            if words.next().is_some() { return Err(TgTraceParseError::BadPen); }
            Ok(TgTraceLine::Pen { k_ms, what })
        }
        Some("E") => {
            let rest = line.strip_prefix("E ").ok_or(TgTraceParseError::BadOutcome)?;
            let (time, text) = rest.split_once(char::is_whitespace).ok_or(TgTraceParseError::BadOutcome)?;
            let k_ms = time.parse().map_err(|_| TgTraceParseError::BadOutcome)?;
            let text = text.trim();
            if text.is_empty() { return Err(TgTraceParseError::BadOutcome); }
            Ok(TgTraceLine::Outcome { k_ms, text })
        }
        _ => Err(TgTraceParseError::BadFrame),
    }
}
