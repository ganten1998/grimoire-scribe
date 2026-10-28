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
//! reader made of them before this module existed.
//!
//! One finger is the stream it always was: TouchDown at once, TouchMove when the place changes,
//! TouchUp at the lift, nothing added in between. While the pen is near, or for a second after it
//! leaves, a finger is the writing hand resting on the screen and gives nothing, and a finger that
//! was already down ends with the off-screen cancel (TOUCH_F_CANCEL, x = y = -10000), never a lift
//! in its place, which could follow a link or press a button under the palm.
//!
//! A second finger soon after the first starts a two-finger session: the first finger's stream ends
//! with the cancel, and PinchBegin (value 1) names both fingers by the order they landed in. Nothing
//! moves until the session commits (the spread changes by a tenth, or the midpoint travels 70 px at
//! 300 dpi, the editor's own dead zone): then it is classified once, and each frame brings a
//! PinchMove whose value is the class (1 zoom, 2 pan). A second finger long after the first is a
//! palm or a second hand, as it always was: every contact is suppressed until all have lifted.

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

// ---- the rules' numbers (provisional ones are marked: the owner's traces will set them) ----

/// Later than this, a second contact is a palm or a second hand, as it always was.
const PALM_MS: u32 = 400;
/// A finger this soon after the pen was near is the writing hand.
const PEN_WINDOW_MS: u32 = 1000;
/// A kernel time this far before the pen's leave still counts as the pen being near: a touch frame
/// read after the leave can be older than it (PENSYNC_K_MAX_AGE_MS bounds how old k_ms gets).
const PEN_BEHIND_MS: u32 = 10000;
/// The commit: the spread changes by this much, or the midpoint travels COMMIT_PX at 300 dpi.
const COMMIT_SPREAD: f32 = 0.10;
const COMMIT_PX: f32 = 70.0;
/// A commit is a zoom when the spread changed at least this much of the midpoint's travel.
const ZOOM_RATIO: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Ph {
    /// nothing down (or only contacts the panel calls a pen: never fingers)
    #[default]
    Idle,
    /// one finger followed (one_on: its TouchDown went out; else the pen window holds it)
    One,
    /// two fingers: PinchBegin went out
    Session,
    /// every contact down waits to lift
    Suppressed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Class {
    /// not committed yet
    #[default]
    Pending,
    Zoom,
    Pan,
}

/// How a touch session (first contact down to all lifted) went, for grimoire.log.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Result_ {
    #[default]
    None,
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
    /// where it landed, where it is, where it was as the session began (integer px)
    x0: i32,
    y0: i32,
    ix: i32,
    iy: i32,
    sx: i32,
    sy: i32,
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
}

/// The pixel sizes of the rules, from millimetres at the screen's dpi (touchgest_init).
#[derive(Clone, Copy, Debug)]
struct Thr {
    commit: f32,
}

impl Thr {
    fn at(dpi: f32) -> Thr {
        // (ui_px's scale: 300 dpi is 1)
        let scale = if dpi > 50.0 { dpi / 300.0 } else { 1.0 };
        Thr { commit: (COMMIT_PX * scale).round_ties_even() }
    }
}

/// The recognizer's state (the reader keeps it: input_evdev's State, the runner's).
#[derive(Clone, Copy, Debug)]
pub struct TouchGest {
    t: [Track; TG_MAX],
    ph: Ph,
    /// One: the finger followed, and whether its TouchDown went out
    one: usize,
    one_on: bool,
    /// the last TouchDown or TouchMove, whose place the TouchUp repeats
    last: InEvent,
    /// Session: the pair by landing order, the first one's landing
    p: [usize; 2],
    first_k: u32,
    class: Class,
    /// spread and midpoint as the session began (integer px; the spread at least 10 px)
    d0: f32,
    mx0: f32,
    my0: f32,
    /// PinchBegin went out and its PinchEnd has not
    begun: bool,
    view: TgView,
    palm_major: i32,
    thr: Thr,
    last_gesture: i32,
    last_reject: Option<TgReject>,
    result: Result_,
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
            class: Class::Pending,
            d0: 0.0,
            mx0: 0.0,
            my0: 0.0,
            begun: false,
            view: TgView::default(),
            palm_major: i32::MAX,
            thr: Thr::at(300.0),
            last_gesture: 0,
            last_reject: None,
            result: Result_::None,
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

    /// Every contact down waits to lift.
    fn suppress(&mut self) {
        self.ph = Ph::Suppressed;
    }

    fn reject(&mut self, out: &mut TgOut, k: u32, r: TgReject) {
        self.pinch_end(out, k, 0);
        self.last_reject = Some(r);
        out.outcome = TgOutcome::Rejected(r);
        self.result = Result_::Rejected(r);
        self.suppress();
    }

    fn start_session(&mut self, a: usize, b: usize, out: &mut TgOut, k: u32) {
        // By landing order (the same frame: by slot), not by slot.
        let (ta, tb) = (&self.t[a], &self.t[b]);
        let a_first = (ta.down_k.wrapping_sub(tb.down_k) as i32) < 0 || (ta.down_k == tb.down_k && ta.slot <= tb.slot);
        self.p = if a_first { [a, b] } else { [b, a] };
        self.first_k = self.t[self.p[0]].down_k;
        for &i in &self.p {
            let t = &mut self.t[i];
            t.sx = t.ix;
            t.sy = t.iy;
        }
        let (p, q) = (&self.t[self.p[0]], &self.t[self.p[1]]);
        self.d0 = dist(p.ix, p.iy, q.ix, q.iy).max(10.0);
        self.mx0 = (p.ix + q.ix) as f32 * 0.5;
        self.my0 = (p.iy + q.iy) as f32 * 0.5;
        self.class = Class::Pending;
        self.ph = Ph::Session;
        self.begun = true;
        let e = self.pinch_ev(InType::PinchBegin, 1);
        self.push(out, e, k);
    }

    /// A frame of a session with both fingers down: the commit, then a PinchMove each frame.
    fn session_frame(&mut self, out: &mut TgOut, k: u32) {
        let (p, q) = (self.t[self.p[0]], self.t[self.p[1]]);
        if self.class == Class::Pending {
            let d = dist(p.ix, p.iy, q.ix, q.iy);
            let mx = (p.ix + q.ix) as f32 * 0.5;
            let my = (p.iy + q.iy) as f32 * 0.5;
            let travel = ((mx - self.mx0) * (mx - self.mx0) + (my - self.my0) * (my - self.my0)).sqrt();
            let spread = (d / self.d0 - 1.0).abs();
            if spread < COMMIT_SPREAD && travel < self.thr.commit {
                return; // nothing moves until the fingers clearly spread or travel
            }
            let spread_px = (d - self.d0).abs();
            self.class = if self.view.zoom_ok && spread_px >= ZOOM_RATIO * travel { Class::Zoom } else { Class::Pan };
        }
        let value = if self.class == Class::Zoom { 1 } else { 2 };
        let e = self.pinch_ev(InType::PinchMove, value);
        self.push(out, e, k);
    }

    /// One of the session's fingers lifted.
    fn first_lift(&mut self, out: &mut TgOut, k: u32) {
        self.result = match self.class {
            Class::Zoom => Result_::Zoom,
            Class::Pan => Result_::Pan,
            Class::Pending => Result_::None,
        };
        self.pinch_end(out, k, 0);
        self.suppress();
    }

    fn lifts(&mut self, out: &mut TgOut, k: u32) {
        let lifted = |t: &Track| t.on && !t.seen;
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
            Ph::Session if lifted(&self.t[self.p[0]]) || lifted(&self.t[self.p[1]]) => self.first_lift(out, k),
            _ => {}
        }
        for t in self.t.iter_mut() {
            if lifted(t) {
                t.on = false;
            }
        }
    }

    fn landings(&mut self, pen: &TgPen, out: &mut TgOut, k: u32) {
        let mut new = [0usize; TG_MAX];
        let mut n = 0;
        let mut palm = false;
        for (i, t) in self.t.iter().enumerate() {
            if t.on && t.fresh && !t.pen {
                if t.palm {
                    palm = true;
                } else {
                    new[n] = i;
                    n += 1;
                }
            }
        }
        if n == 0 && !palm {
            return;
        }
        match self.ph {
            Ph::Idle => {
                self.result = Result_::None;
                if palm {
                    self.result = Result_::Palm;
                    self.suppress();
                } else if n >= 3 {
                    self.reject(out, k, TgReject::Three);
                } else if Self::pen_window(pen, k) {
                    // The writing hand: no pinch starts in the pen window, and one finger stays quiet.
                    if n == 2 {
                        self.suppress();
                    } else {
                        self.ph = Ph::One;
                        self.one = new[0];
                        self.one_on = false;
                        self.suppress();
                    }
                } else if n == 2 {
                    self.start_session(new[0], new[1], out, k);
                } else {
                    self.ph = Ph::One;
                    self.one = new[0];
                    self.one_on = true;
                    let e = self.touch_ev(InType::TouchDown, new[0]);
                    self.last = e;
                    self.push(out, e, k);
                }
            }
            Ph::One => {
                let a = self.one;
                let gap = k.wrapping_sub(self.t[a].down_k);
                self.cancel_one(out, k); // the first finger's press turns into whatever comes
                if palm {
                    self.result = Result_::Palm;
                    self.suppress();
                } else if n >= 2 {
                    self.reject(out, k, TgReject::Three);
                } else if gap > PALM_MS || Self::pen_window(pen, k) {
                    // A real pinch puts both fingers down together; a palm or a second hand arrives
                    // later. Late second contacts are treated as a palm.
                    self.result = Result_::Palm;
                    self.suppress();
                } else {
                    self.start_session(a, new[0], out, k);
                }
            }
            Ph::Session => {
                // A palm ends the session there; a third finger too, except that a zoom under way
                // keeps its two fingers.
                if palm {
                    self.result = Result_::Palm;
                    self.pinch_end(out, k, 0);
                    self.suppress();
                } else if self.class != Class::Zoom {
                    self.reject(out, k, TgReject::Three);
                }
            }
            Ph::Suppressed => {}
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
                } else if Self::pen_window(pen, k) {
                    self.cancel_one(out, k);
                    self.suppress();
                } else if self.one_on && (t.c.nx != self.last.nx || t.c.ny != self.last.ny) {
                    let e = self.touch_ev(InType::TouchMove, a);
                    self.last = e;
                    self.push(out, e, k);
                }
            }
            Ph::Session => {
                if Self::pen_window(pen, k) {
                    // The pen came near mid-pinch: stop zooming right there.
                    self.first_lift(out, k);
                } else {
                    self.session_frame(out, k);
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
    for c in &f.c[..f.n.min(TG_MAX)] {
        let i = match g.t.iter().position(|t| t.on && t.id == c.id && t.slot == c.slot) {
            Some(i) => i,
            None => {
                let Some(i) = g.t.iter().position(|t| !t.on) else { continue };
                let (x, y) = (ipx(c.x), ipx(c.y));
                g.t[i] = Track { on: true, fresh: true, id: c.id, slot: c.slot, down_k: k, x0: x, y0: y, ..Default::default() };
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
    g.lifts(out, k);
    g.landings(pen, out, k);
    g.work(pen, out, k);
    if !touchgest_busy(g) {
        g.ph = Ph::Idle;
    }
}

/// The pen arrived, or the kernel dropped touch events: the finger's stream ends with the cancel, a
/// session with PinchEnd, and every contact down waits to lift.
pub fn touchgest_cancel(g: &mut TouchGest, k_ms: u32, out: &mut TgOut) {
    *out = TgOut::default();
    let busy = g.ph == Ph::One || g.ph == Ph::Session;
    g.cancel_one(out, k_ms);
    g.pinch_end(out, k_ms, 0);
    if busy {
        g.result = Result_::Cancelled;
    }
    g.ph = if touchgest_busy(g) { Ph::Suppressed } else { Ph::Idle };
}

/// The windows that expire with no new frame (a still pair gives none).
pub fn touchgest_tick(g: &mut TouchGest, k_ms: u32, out: &mut TgOut) {
    *out = TgOut::default();
    let _ = (g, k_ms);
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
    match key {
        "touch.contacts" => out.push_str(&cformat!("%d", g.t.iter().filter(|t| t.on).count() as i32)),
        "touch.class" => out.push_str(match g.ph {
            Ph::Idle => "none",
            Ph::One if g.one_on => "one",
            Ph::One | Ph::Suppressed => "suppressed",
            Ph::Session => match g.class {
                Class::Pending => "landing",
                Class::Zoom => "zoom",
                Class::Pan => "pan",
            },
        }),
        "touch.suppressed" => {
            let s = g.ph == Ph::Suppressed || (g.ph == Ph::One && !g.one_on);
            out.push_str(if s { "1" } else { "0" });
        }
        "touch.gesture" => out.push_str(touchgest_gesture_name(g.last_gesture)),
        "touch.reject" => out.push_str(g.last_reject.map_or("", touchgest_reject_name)),
        _ => return false,
    }
    true
}
