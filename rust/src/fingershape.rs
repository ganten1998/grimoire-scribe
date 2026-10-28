//! fingershape: recorded one-finger shape gestures (ruling D4), the pure part. A finger that lands
//! on the idle page and draws a shape fires the action the user recorded that shape for. This module
//! holds everything about that which can be decided without a screen: the capture of one finger's
//! touch, the gates that tell a shape from a tap, a swipe, a scribble or a resting hand, the matcher
//! that compares it with the recorded shapes, the checks a new recording must pass, and the codec of
//! a gestures.ini shape value. The design is docs/index-hook/index-hook-shape-gestures-2026-10-27.json
//! (sections 2 to 6, 8 and 10); the section numbers below are its.
//!
//! Integer arithmetic only, no allocation, no clock and no I/O: the caller passes every point and
//! every time in (now_ms()'s clock, compared with wrapping_sub), and the same input gives the same
//! score on the Kindle (armv7), the Mac (arm64) and the runner (x86_64), because nothing here is a
//! float. Float contraction differs between those builds; integer division does not. So the tests
//! assert exact scores.
//!
//! The pipeline, at runtime and in recording alike:
//! 1. `fs_begin` at the landing, `fs_add` per move, `fs_tick` on wall time, `fs_finish` at the lift
//!    (`fs_abort` on the off-screen cancel, `fs_kill` when the link hold fires). The gates run in
//!    `fs_finish`, from the one table `FS_GATES`.
//! 2. A stroke that passed becomes an `FsSample`: 32 points spaced evenly along the path, scaled into
//!    0-255 with the aspect kept (exactly what gestures.ini stores, so a shape read back from the
//!    file scores as it did when it was drawn).
//! 3. `fs_match` compares it with every recorded `FsGesture` ($P point clouds, tried at -10, 0 and
//!    +10 degrees) and fires only a clear winner.
//! 4. Recording: `fs_record_first` (the slip bank at T_MIN), `fs_record_consistent` per later sample,
//!    then `fs_prepare`, `fs_record_separation` and `fs_record_bank` before Use is offered.

use crate::prelude::*;

// ---------------------------------------------------------------------------------------------
// The constants
// ---------------------------------------------------------------------------------------------

/// Points per resampled shape: $P's usual count, and O(n^2) stays cheap at 32 (section 4).
pub const FS_N: usize = 32;
/// Raw points the capture keeps. Past that it halves them in place: 2.5 s at 80 Hz is 200 points.
pub const FS_RAW_MAX: usize = 256;
/// Samples a recorded shape keeps (the record card's three slots).
pub const FS_SAMPLES_MAX: usize = 3;
/// Recorded shapes in all (gestures.ini's shape lines).
pub const FS_SHAPES_MAX: usize = 32;
/// The larger box side of a normalized cloud, in cloud units. Scores are per mille of it.
pub const FS_SCALE: i32 = 512;
/// The accept threshold's floor and ceiling, per mille (T_g = clamp(1.3 x spread + 30, 70, 95)).
pub const FS_T_MIN: i32 = 70;
pub const FS_T_MAX: i32 = 95;
/// The margin the winner needs over the second best: max(35, 30% of D1).
pub const FS_MARGIN_MIN: i32 = 35;
pub const FS_MARGIN_PCT: i32 = 30;
/// The structure guard: the candidate's path/diagonal ratio within 0.6x-1.6x of the gesture's.
pub const FS_GUARD_LO_X10: i32 = 6;
pub const FS_GUARD_HI_X10: i32 = 16;
/// A later recording sample must lie within this of an earlier one (6.3).
pub const FS_CONSIST_MAX: i32 = 60;
/// Separation: too close to another action's shape under max(T_h, T_new) + this (6.3).
pub const FS_SEPARATE_PAD: i32 = 20;
/// The rotations tried, in degrees: a hand that holds the Kindle a little turned (section 4).
pub const FS_ROT_DEG: i32 = 10;
/// Distances the matcher need not know exactly: past T_MAX + the margin, a gesture can neither win
/// nor stand in the winner's way, so the matcher stops counting there and reports FS_FAR + 1.
pub const FS_FAR: i32 = FS_T_MAX + FS_MARGIN_MIN;
/// The slip-bank check calls its pump this often (in bank paths): about 1-2 s on the Kindle in all.
pub const FS_BANK_PUMP: usize = 64;
/// fs_match calls its pump after this many recorded samples (the work_pump rhythm, section 4).
pub const FS_MATCH_PUMP: usize = 4;
/// In recording, a refused straight stroke and another landing within this read as an X or = drawn
/// in two strokes (section 4, Multi-stroke).
pub const FS_TWO_STROKE_MS: u32 = 700;
/// Vertices of the longest slip-bank path (a 220 degree arc in 5 degree segments).
pub const FS_BANK_PTS: usize = 48;

/// Every capture and gate constant, in one table that the page and the record card both read
/// (section 3: "identical at runtime and in recording"). Lengths are tenths of a mm on the screen,
/// so a shape's size does not depend on the page's zoom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsGates {
    /// a finger becomes a shape once it is this far from its landing (Euclidean)
    pub slop_mm10: i32,
    /// ...but not before this (the two-finger landing window: a late second finger never shows a
    /// shape's trail first)
    pub trail_ms: u32,
    /// ...and only if it left the slop within this of landing (below the 600 ms link hold)
    pub lead_ms: u32,
    /// landing to lift at most this
    pub dur_max_ms: u32,
    /// holding still within this radius...
    pub pause_mm10: i32,
    /// ...for this long cancels the shape ("hold still to cancel")
    pub pause_ms: u32,
    /// raw frames at least
    pub frames_min: u32,
    /// the bounding box's larger side at least
    pub box_mm10: i32,
    /// the path at least
    pub path_mm10: i32,
    /// mean speed from the slop exit to the lift at least, mm/s
    pub speed_mm_s: i32,
    /// not a line: the farthest point from the chord at least this % of max(chord, box side)
    pub chord_pct: i32,
    /// not tangled: the path at most this many tenths of the box diagonal
    pub tangle_x10: i32,
    /// a one-finger swipe (touch.1.<dir>): a chord of at least this...
    pub swipe_mm10: i32,
    /// ...within this of landing...
    pub swipe_ms: u32,
    /// ...within 30 degrees of an axis (tan 30 = 0.5774)
    pub swipe_tan_x10000: i32,
}

/// The one gate table (section 3; the device traces may retune it before the P4 freeze). A static,
/// not a const: one object at one address, which fs_gates hands to both modes.
pub static FS_GATES: FsGates = FsGates {
    slop_mm10: 60,
    trail_ms: 150,
    lead_ms: 400,
    dur_max_ms: 2500,
    pause_mm10: 15,
    pause_ms: 500,
    frames_min: 8,
    box_mm10: 200,
    path_mm10: 300,
    speed_mm_s: 50,
    chord_pct: 15,
    tangle_x10: 50,
    swipe_mm10: 300,
    swipe_ms: 600,
    swipe_tan_x10000: 5774,
};

/// Who is capturing: the page or the record card. Both get the same gates (the unit test holds
/// fs_gates to that); the mode exists so a caller says which it is, and nothing can drift apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum FsMode {
    Runtime,
    Record,
}

/// The gate table for a mode: the same one for both, by design.
pub fn fs_gates(mode: FsMode) -> &'static FsGates {
    match mode {
        FsMode::Runtime | FsMode::Record => &FS_GATES,
    }
}

// ---------------------------------------------------------------------------------------------
// Reasons
// ---------------------------------------------------------------------------------------------

/// Why a stroke is not a shape, why a shape fired nothing, or why a recording is refused. The
/// screens map each to its sentence (6.4); this module has no text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FsReason {
    /// (no reason: the default of a fresh stroke)
    #[default]
    None,
    /// fewer raw frames than frames_min
    Frames,
    /// box or path below the minimum ("Draw it bigger")
    TooSmall,
    /// over 2.5 s, or slower than 50 mm/s ("Too slow")
    TooSlow,
    /// it left the slop after the lead ("Start moving as soon as you touch")
    LateStart,
    /// it held still for the pause ("You held still, so it stopped")
    Paused,
    /// a line that is not a valid swipe ("Too straight")
    Straight,
    /// path over 5x the box diagonal: a scribble ("Too many turns")
    Tangled,
    /// a palm-sized contact (TOUCH_F_BROAD) at some frame ("Use a fingertip")
    Broad,
    /// the link hold fired first (the caller's fs_kill)
    Held,
    /// the off-screen cancel: a second finger, the palm flag or the pen (fs_abort)
    Cancelled,
    /// runtime: no recorded shape within its threshold
    NoMatch,
    /// runtime: the best is not clearly better than the second best (another action)
    Unclear,
    /// runtime: the path/diagonal ratio is unlike the winner's (a spiral is not a circle)
    Structure,
    /// recording: not like the earlier samples ("Not like the first one")
    Inconsistent,
    /// recording: it is another action's shape ("That is the shape for %s. Use moves it here.")
    SameAs,
    /// recording: too near another action's shape ("Too close to the shape for %s.")
    TooClose,
    /// recording: a single slip of the finger can draw it ("Too easy to make by accident.")
    Slip,
    /// recording: FS_SHAPES_MAX shapes are recorded already
    Full,
    /// recording: a straight stroke and another right after it ("One stroke, without lifting.")
    TwoStrokes,
}

// ---------------------------------------------------------------------------------------------
// Integer helpers
// ---------------------------------------------------------------------------------------------

/// floor(sqrt(v)), bit by bit: exact on every target.
fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut op = v as u64;
    let mut res = 0u64;
    let mut one = 1u64 << 62;
    while one > op {
        one >>= 2;
    }
    while one != 0 {
        if op >= res + one {
            op -= res + one;
            res = (res >> 1) + one;
        } else {
            res >>= 1;
        }
        one >>= 2;
    }
    res as i64
}

/// a / b rounded to the nearest, halves away from zero (b > 0). Plain division truncates toward
/// zero, which would pull every negative coordinate toward the centre.
fn div_round(a: i64, b: i64) -> i64 {
    if a >= 0 {
        (a + b / 2) / b
    } else {
        -((-a + b / 2) / b)
    }
}

/// sin(deg) in Q14 (16384 = 1), one entry per degree of the first quadrant, rounded from the exact
/// values once, so no libm is involved and every build reads the same numbers.
const SIN_Q14: [i32; 91] = [
    0, 286, 572, 857, 1143, 1428, 1713, 1997, 2280, 2563, //
    2845, 3126, 3406, 3686, 3964, 4240, 4516, 4790, 5063, 5334, //
    5604, 5872, 6138, 6402, 6664, 6924, 7182, 7438, 7692, 7943, //
    8192, 8438, 8682, 8923, 9162, 9397, 9630, 9860, 10087, 10311, //
    10531, 10749, 10963, 11174, 11381, 11585, 11786, 11982, 12176, 12365, //
    12551, 12733, 12911, 13085, 13255, 13421, 13583, 13741, 13894, 14044, //
    14189, 14330, 14466, 14598, 14726, 14849, 14968, 15082, 15191, 15296, //
    15396, 15491, 15582, 15668, 15749, 15826, 15897, 15964, 16026, 16083, //
    16135, 16182, 16225, 16262, 16294, 16322, 16344, 16362, 16374, 16382, //
    16384,
];

/// sin of a whole number of degrees (any sign) in Q14. The slip bank and the synthetic corpus
/// draw their paths with it, so they come out the same everywhere too.
pub fn fs_sin_q14(deg: i32) -> i32 {
    let d = deg.rem_euclid(360);
    match d {
        0..=90 => SIN_Q14[d as usize],
        91..=180 => SIN_Q14[(180 - d) as usize],
        181..=270 => -SIN_Q14[(d - 180) as usize],
        _ => -SIN_Q14[(360 - d) as usize],
    }
}

/// cos of a whole number of degrees in Q14.
pub fn fs_cos_q14(deg: i32) -> i32 {
    fs_sin_q14(deg + 90)
}

/// Tenths of a mm to screen px, rounded, for a screen of px_per_100mm (ui_px(1181.0) on the
/// Scribe: 11.81 px/mm at 300 dpi, so 6 mm is 71 px and 20 mm 236 px, the design's numbers).
pub fn fs_mm_px(mm10: i32, px_per_100mm: i32) -> i32 {
    div_round(mm10 as i64 * px_per_100mm as i64, 1000) as i32
}

fn dist2(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    // (widened first: any two i32 points, even the off-screen cancel's -10000, fit)
    let dx = bx as i64 - ax as i64;
    let dy = by as i64 - ay as i64;
    dx * dx + dy * dy
}

// ---------------------------------------------------------------------------------------------
// Capture (section 2)
// ---------------------------------------------------------------------------------------------

/// A raw point: logical screen px (panel_to_logical already applied, so "up" is the user's up).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FsPt {
    pub x: i32,
    pub y: i32,
}

/// Where a one-finger session is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum FsState {
    /// no finger, or not armed
    #[default]
    Idle,
    /// landed; still a tap, a link hold or a shape
    Pending,
    /// a shape: the trail is shown
    Drawing,
    /// not a shape any more; the rest of the touch is ignored until the lift (FsStroke::dead says why)
    Dead,
}

/// One finger's session: the capture state machine and its buffer. Fixed size (about 2 KiB), so
/// the editor and the record card keep one each and nothing is allocated per touch.
#[derive(Clone, Copy, Debug)]
pub struct FsStroke {
    pub state: FsState,
    /// why the session went Dead
    pub dead: FsReason,
    /// raw frames seen, the landing included (the buffer may hold fewer: see decimation)
    pub frames: u32,
    pts: [FsPt; FS_RAW_MAX],
    /// stored points in pts
    n: usize,
    /// whether pts[n] holds the newest point, which decimation did not store
    tail: bool,
    /// store every step-th frame: 1, then 2, 4... each time the buffer fills
    step: u32,
    skip: u32,
    last: FsPt,
    t0: u32,
    t_exit: u32,
    exited: bool,
    /// the pause test's reference: where the finger last moved more than the pause radius, and when
    anchor: FsPt,
    anchor_t: u32,
    /// the path's length in 1/16 px, over every frame (not only the stored ones)
    path16: i64,
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
    px100: i32,
    broad: bool,
}

impl Default for FsStroke {
    fn default() -> Self {
        FsStroke {
            state: FsState::Idle,
            dead: FsReason::None,
            frames: 0,
            pts: [FsPt::default(); FS_RAW_MAX],
            n: 0,
            tail: false,
            step: 1,
            skip: 0,
            last: FsPt::default(),
            t0: 0,
            t_exit: 0,
            exited: false,
            anchor: FsPt::default(),
            anchor_t: 0,
            path16: 0,
            minx: 0,
            miny: 0,
            maxx: 0,
            maxy: 0,
            px100: 1181,
            broad: false,
        }
    }
}

/// What a lift (or a cancel) made of the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsLift {
    /// it never became a shape: a tap or a hold, the caller's ordinary finger handling
    Nothing,
    /// a shape that passed the gates (fs_finish wrote its sample)
    Shape,
    /// a straight line that is a valid one-finger swipe (touch.1.<dir>)
    Swipe(FsSwipe),
    /// a shape refused by a gate, or a session that died
    Refused(FsReason),
}

/// A one-finger swipe's direction, in logical screen terms (y grows downward).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum FsSwipe {
    Up,
    Down,
    Left,
    Right,
}

/// The finger landed on the idle page (OWN_LINK, armed). Points are kept from the landing, so a
/// shape includes its own start. px_per_100mm is the screen's scale (fs_mm_px).
pub fn fs_begin(s: &mut FsStroke, x: i32, y: i32, t: u32, px_per_100mm: i32) {
    *s = FsStroke::default();
    let p = FsPt { x, y };
    s.state = FsState::Pending;
    s.frames = 1;
    s.pts[0] = p;
    s.n = 1;
    s.last = p;
    s.t0 = t;
    s.anchor = p;
    s.anchor_t = t;
    s.minx = x;
    s.maxx = x;
    s.miny = y;
    s.maxy = y;
    s.px100 = px_per_100mm.max(1);
}

fn die(s: &mut FsStroke, why: FsReason) {
    s.state = FsState::Dead;
    s.dead = why;
}

/// Keeps a point. A full buffer keeps every other point and from then on every other frame, so a
/// long shape keeps its whole extent in O(1) per move; the newest point is always kept as the tail.
fn store(s: &mut FsStroke, p: FsPt) {
    s.skip += 1;
    if s.skip >= s.step {
        s.skip = 0;
        s.pts[s.n] = p;
        s.n += 1;
        s.tail = false;
        if s.n == FS_RAW_MAX {
            for i in 0..FS_RAW_MAX / 2 {
                s.pts[i] = s.pts[2 * i];
            }
            s.n = FS_RAW_MAX / 2;
            s.step *= 2;
            // (the newest point was at an odd index and is now only the tail)
            s.pts[s.n] = p;
            s.tail = true;
        }
    } else {
        s.pts[s.n] = p;
        s.tail = true;
    }
}

/// A move of the finger at frame time t (the kernel's SYN_REPORT time, k_ms). broad is
/// TOUCH_F_BROAD: the contact has been palm-sized at some frame. Returns the state after it.
pub fn fs_add(s: &mut FsStroke, x: i32, y: i32, t: u32, broad: bool) -> FsState {
    if s.state != FsState::Pending && s.state != FsState::Drawing {
        return s.state;
    }
    let g = &FS_GATES;
    let p = FsPt { x, y };
    s.frames += 1;
    let seg = isqrt(dist2(s.last.x, s.last.y, x, y) * 256);
    s.path16 += seg;
    s.minx = s.minx.min(x);
    s.maxx = s.maxx.max(x);
    s.miny = s.miny.min(y);
    s.maxy = s.maxy.max(y);
    store(s, p);
    s.last = p;
    if broad {
        s.broad = true;
        die(s, FsReason::Broad);
        return s.state;
    }
    // The pause is judged before this frame moves the anchor: a still finger sends no frames, so
    // the first frame after a rest is the one that tells how long the rest was.
    if s.state == FsState::Drawing && t.wrapping_sub(s.anchor_t) >= g.pause_ms {
        die(s, FsReason::Paused);
        return s.state;
    }
    let pause_px = fs_mm_px(g.pause_mm10, s.px100) as i64;
    if dist2(s.anchor.x, s.anchor.y, x, y) > pause_px * pause_px {
        s.anchor = p;
        s.anchor_t = t;
    }
    if !s.exited {
        let slop = fs_mm_px(g.slop_mm10, s.px100) as i64;
        if dist2(s.pts[0].x, s.pts[0].y, x, y) >= slop * slop {
            s.exited = true;
            s.t_exit = t;
            // A finger that rested and then slid is a grip or a hand settling, not a shape.
            if t.wrapping_sub(s.t0) > g.lead_ms {
                die(s, FsReason::LateStart);
                return s.state;
            }
        }
    }
    fs_tick(s, t)
}

/// Wall time passing with no frame (editor_tick, only when the input is drained, so a queued lift
/// is never read as a pause): promotes a moved finger to a shape once the trail delay is over, and
/// ends a shape that ran over time or held still. Returns the state after it.
pub fn fs_tick(s: &mut FsStroke, now: u32) -> FsState {
    let g = &FS_GATES;
    if s.state == FsState::Pending && s.exited && now.wrapping_sub(s.t0) >= g.trail_ms {
        s.state = FsState::Drawing;
    }
    if s.state == FsState::Drawing {
        if now.wrapping_sub(s.t0) > g.dur_max_ms {
            die(s, FsReason::TooSlow);
        } else if now.wrapping_sub(s.anchor_t) >= g.pause_ms {
            die(s, FsReason::Paused);
        }
    }
    s.state
}

/// The off-screen cancel (a second contact, the palm flag, the pen): nothing fires, the trail goes.
pub fn fs_abort(s: &mut FsStroke) -> FsLift {
    let was = s.state;
    s.state = FsState::Idle;
    if was == FsState::Drawing {
        FsLift::Refused(FsReason::Cancelled)
    } else {
        FsLift::Nothing
    }
}

/// The caller ended the session for its own reason (the link hold fired: FsReason::Held).
pub fn fs_kill(s: &mut FsStroke, why: FsReason) {
    if s.state == FsState::Pending || s.state == FsState::Drawing {
        die(s, why);
    }
}

/// The points kept so far, the newest included: the trail's dots and the sample come from these.
pub fn fs_points(s: &FsStroke) -> &[FsPt] {
    let n = if s.n == 0 { 0 } else { s.n + s.tail as usize };
    &s.pts[..n]
}

/// The stroke's bounding box (every frame's point inside). Its centre is the anchor the action
/// is dispatched at (editor_shortcut_at), so a zoom-in zooms there.
pub fn fs_box(s: &FsStroke) -> Rect {
    if s.frames == 0 {
        return RECT_EMPTY;
    }
    Rect { x: s.minx, y: s.miny, w: s.maxx - s.minx + 1, h: s.maxy - s.miny + 1 }
}

/// The real lift at t_lift. Runs the gates (section 3) for a shape and writes its sample to out;
/// the session is Idle afterwards (its points and box stay readable for the trail).
pub fn fs_finish(s: &mut FsStroke, t_lift: u32, mode: FsMode, out: &mut FsSample) -> FsLift {
    fs_tick(s, t_lift);
    let state = s.state;
    s.state = FsState::Idle;
    match state {
        FsState::Idle | FsState::Pending => FsLift::Nothing,
        FsState::Dead => FsLift::Refused(s.dead),
        FsState::Drawing => {
            let r = gate(s, t_lift, fs_gates(mode));
            if r == FsLift::Shape {
                fs_sample_of(fs_points(s), out);
            }
            r
        }
    }
}

/// The gates, in the order the record card reports them (6.4).
fn gate(s: &FsStroke, t_lift: u32, g: &FsGates) -> FsLift {
    let refuse = FsLift::Refused;
    if s.frames < g.frames_min {
        return refuse(FsReason::Frames);
    }
    if s.broad {
        return refuse(FsReason::Broad);
    }
    let dur = t_lift.wrapping_sub(s.t0);
    if dur > g.dur_max_ms {
        return refuse(FsReason::TooSlow);
    }
    let w = (s.maxx - s.minx) as i64;
    let h = (s.maxy - s.miny) as i64;
    let major = w.max(h);
    if major < fs_mm_px(g.box_mm10, s.px100) as i64 || s.path16 < 16 * fs_mm_px(g.path_mm10, s.px100) as i64 {
        return refuse(FsReason::TooSmall);
    }
    if straight(s, major, g) {
        return match swipe(s, dur, g) {
            Some(d) => FsLift::Swipe(d),
            None => refuse(FsReason::Straight),
        };
    }
    // A scribble or scrubbing covers its box many times over.
    let diag16 = isqrt((w * w + h * h) * 256);
    if s.path16 * 10 > g.tangle_x10 as i64 * diag16 {
        return refuse(FsReason::Tangled);
    }
    // Mean speed over the drawn part: path / (lift - slop exit) >= speed_mm_s, in integers:
    // path16 / 16 px * 100 / px100 mm * 1000 / dt >= speed.
    let dt = t_lift.wrapping_sub(s.t_exit).max(1) as i64;
    if s.path16 * 100 * 1000 < g.speed_mm_s as i64 * 16 * s.px100 as i64 * dt {
        return refuse(FsReason::TooSlow);
    }
    FsLift::Shape
}

/// Not a line: the farthest point from the start-end chord is under chord_pct of max(chord, box
/// side). For a closed shape (the chord under a quarter of the path) the start point stands in for
/// the chord, or every circle would read as a line.
fn straight(s: &FsStroke, major: i64, g: &FsGates) -> bool {
    let pts = fs_points(s);
    let a = pts[0];
    let b = s.last;
    let cx = (b.x - a.x) as i64;
    let cy = (b.y - a.y) as i64;
    let chord16 = isqrt((cx * cx + cy * cy) * 256);
    let closed = chord16 * 4 < s.path16;
    let mut dev16 = 0i64;
    for p in pts {
        let px = (p.x - a.x) as i64;
        let py = (p.y - a.y) as i64;
        let d = if closed {
            isqrt((px * px + py * py) * 256)
        } else {
            // |cross| / |chord|, in 1/16 px: |cross| * 256 / chord16
            (px * cy - py * cx).abs() * 256 / chord16.max(1)
        };
        dev16 = dev16.max(d);
    }
    let ref16 = chord16.max(major * 16);
    dev16 * 100 < g.chord_pct as i64 * ref16
}

/// The one-finger swipe classifier (touch.1.<dir>, the plan's rule): at least swipe_mm10 from
/// landing to lift within swipe_ms, within 30 degrees of an axis.
fn swipe(s: &FsStroke, dur: u32, g: &FsGates) -> Option<FsSwipe> {
    let a = s.pts[0];
    let b = s.last;
    let dx = (b.x - a.x) as i64;
    let dy = (b.y - a.y) as i64;
    let len = fs_mm_px(g.swipe_mm10, s.px100) as i64;
    if dur > g.swipe_ms || dx * dx + dy * dy < len * len {
        return None;
    }
    let (ax, ay) = (dx.abs(), dy.abs());
    let tan = g.swipe_tan_x10000 as i64;
    if ax >= ay {
        if ay * 10000 > tan * ax {
            return None;
        }
        return Some(if dx > 0 { FsSwipe::Right } else { FsSwipe::Left });
    }
    if ax * 10000 > tan * ay {
        return None;
    }
    Some(if dy > 0 { FsSwipe::Down } else { FsSwipe::Up })
}

/// Whether a stroke is an X or = drawn in two strokes (recording only): the previous lift was a
/// refused straight stroke and this finger landed within FS_TWO_STROKE_MS of it.
pub fn fs_two_strokes(prev: FsLift, prev_lift_ms: u32, land_ms: u32) -> bool {
    prev == FsLift::Refused(FsReason::Straight) && land_ms.wrapping_sub(prev_lift_ms) <= FS_TWO_STROKE_MS
}

// ---------------------------------------------------------------------------------------------
// Normalization (section 4)
// ---------------------------------------------------------------------------------------------

/// A shape as gestures.ini stores it: 32 points in drawing order, x then y, the larger box side
/// spanning 0-255 and the smaller one centred. Matching ignores the order; thumbnails draw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsSample {
    pub xy: [u8; 2 * FS_N],
}

impl Default for FsSample {
    fn default() -> Self {
        FsSample { xy: [128; 2 * FS_N] }
    }
}

/// Resamples a polyline (screen px) to 32 points evenly spaced along it, in 1/16 px, and scales
/// them into an FsSample. Streaming, in two passes, so a path of any length needs no buffer.
pub fn fs_sample_of(pts: &[FsPt], out: &mut FsSample) {
    *out = FsSample::default();
    if pts.is_empty() {
        return;
    }
    let seg = |i: usize| -> i64 {
        let (a, b) = (pts[i], pts[i + 1]);
        isqrt(dist2(a.x, a.y, b.x, b.y) * 256)
    };
    let mut total = 0i64;
    for i in 0..pts.len() - 1 {
        total += seg(i);
    }
    let mut rx = [0i64; FS_N];
    let mut ry = [0i64; FS_N];
    let mut k = 0usize;
    let mut acc = 0i64;
    let last = FS_N as i64 - 1;
    for i in 0..pts.len() - 1 {
        let d = seg(i);
        let (a, b) = (pts[i], pts[i + 1]);
        while k < FS_N && k as i64 * total / last <= acc + d {
            let along = k as i64 * total / last - acc;
            let (ax, ay) = (a.x as i64 * 16, a.y as i64 * 16);
            if d == 0 {
                rx[k] = ax;
                ry[k] = ay;
            } else {
                rx[k] = ax + div_round(along * (b.x - a.x) as i64 * 16, d);
                ry[k] = ay + div_round(along * (b.y - a.y) as i64 * 16, d);
            }
            k += 1;
        }
        acc += d;
    }
    // (a lone point, or the rounding's leftovers: the path's end)
    let end = pts[pts.len() - 1];
    while k < FS_N {
        rx[k] = end.x as i64 * 16;
        ry[k] = end.y as i64 * 16;
        k += 1;
    }
    let (mut minx, mut maxx, mut miny, mut maxy) = (rx[0], rx[0], ry[0], ry[0]);
    for i in 1..FS_N {
        minx = minx.min(rx[i]);
        maxx = maxx.max(rx[i]);
        miny = miny.min(ry[i]);
        maxy = maxy.max(ry[i]);
    }
    let (w, h) = (maxx - minx, maxy - miny);
    let major = w.max(h);
    if major == 0 {
        return;
    }
    let offx = (255 - div_round(w * 255, major)) / 2;
    let offy = (255 - div_round(h * 255, major)) / 2;
    for i in 0..FS_N {
        out.xy[2 * i] = (offx + div_round((rx[i] - minx) * 255, major)) as u8;
        out.xy[2 * i + 1] = (offy + div_round((ry[i] - miny) * 255, major)) as u8;
    }
}

/// A point cloud: the sample's centroid at 0 and its larger box side 512 units.
#[derive(Clone, Copy, Debug, Default)]
struct Cloud {
    x: [i32; FS_N],
    y: [i32; FS_N],
}

fn cloud_of(s: &FsSample) -> Cloud {
    let mut c = Cloud::default();
    let (mut sx, mut sy) = (0i64, 0i64);
    let (mut minx, mut maxx, mut miny, mut maxy) = (255i64, 0i64, 255i64, 0i64);
    for i in 0..FS_N {
        let (x, y) = (s.xy[2 * i] as i64, s.xy[2 * i + 1] as i64);
        sx += x;
        sy += y;
        minx = minx.min(x);
        maxx = maxx.max(x);
        miny = miny.min(y);
        maxy = maxy.max(y);
    }
    let major = (maxx - minx).max(maxy - miny);
    if major == 0 {
        return c;
    }
    // (x - mean) * 512 / major, with the mean kept exact: (32 x - sum) * 16 / major
    for i in 0..FS_N {
        c.x[i] = div_round((FS_N as i64 * s.xy[2 * i] as i64 - sx) * (FS_SCALE as i64 / FS_N as i64), major) as i32;
        c.y[i] = div_round((FS_N as i64 * s.xy[2 * i + 1] as i64 - sy) * (FS_SCALE as i64 / FS_N as i64), major) as i32;
    }
    c
}

/// The cloud turned by deg degrees about its centroid, in Q14 fixed point.
fn rotate(c: &Cloud, deg: i32) -> Cloud {
    let (cs, sn) = (fs_cos_q14(deg) as i64, fs_sin_q14(deg) as i64);
    let mut r = Cloud::default();
    for i in 0..FS_N {
        let (x, y) = (c.x[i] as i64, c.y[i] as i64);
        r.x[i] = div_round(x * cs - y * sn, 16384) as i32;
        r.y[i] = div_round(x * sn + y * cs, 16384) as i32;
    }
    r
}

/// The path over the box diagonal, x1000, of a sample's 32 points: the structure guard's measure.
/// A circle reads about 2200, the same circle drawn twice about 4400.
fn ratio_of(s: &FsSample) -> i32 {
    let (mut minx, mut maxx, mut miny, mut maxy) = (255i32, 0i32, 255i32, 0i32);
    let mut path = 0i64;
    for i in 0..FS_N {
        let (x, y) = (s.xy[2 * i] as i32, s.xy[2 * i + 1] as i32);
        minx = minx.min(x);
        maxx = maxx.max(x);
        miny = miny.min(y);
        maxy = maxy.max(y);
        if i > 0 {
            path += isqrt(dist2(s.xy[2 * i - 2] as i32, s.xy[2 * i - 1] as i32, x, y) * 256);
        }
    }
    let diag = isqrt(dist2(minx, miny, maxx, maxy) * 256);
    if diag == 0 {
        return 0;
    }
    (path * 1000 / diag) as i32
}

// ---------------------------------------------------------------------------------------------
// The $P distance (section 4)
// ---------------------------------------------------------------------------------------------

/// The sum of the weights 32, 31, ... 1: the weights fall from 1 to 1/32 along the matching order.
const WSUM: i64 = (FS_N * (FS_N + 1) / 2) as i64;
/// A weighted sum over WSUM x FS_SCALE, per mille: D = sum * 1000 / DEN, rounded.
const DEN: i64 = WSUM * FS_SCALE as i64;
/// "No bound": larger than any distance two clouds can have.
const UNBOUNDED: i32 = 100_000;

fn per_mille(sum: i64) -> i32 {
    ((sum * 1000 + DEN / 2) / DEN) as i32
}

/// The smallest weighted sum that rounds to more than bound per mille: a partial sum that reaches
/// it can stop, since sums only grow.
fn abandon_at(bound: i32) -> i64 {
    ((bound as i64 + 1) * DEN - DEN / 2 + 999) / 1000
}

/// One greedy pass of $P: from point `start` of a, each point in turn takes the nearest point of b
/// not yet taken (the first such on a tie). The weighted distance sum, or a sum >= stop once it is
/// sure to end there.
fn greedy_pass(a: &Cloud, b: &Cloud, start: usize, stop: i64) -> i64 {
    let mut taken = [false; FS_N];
    let mut sum = 0i64;
    for k in 0..FS_N {
        let i = (start + k) % FS_N;
        let (ax, ay) = (a.x[i], a.y[i]);
        let mut best = i32::MAX;
        let mut bj = 0;
        for j in 0..FS_N {
            if taken[j] {
                continue;
            }
            let (dx, dy) = (b.x[j] - ax, b.y[j] - ay);
            let d = dx * dx + dy * dy;
            if d < best {
                best = d;
                bj = j;
            }
        }
        taken[bj] = true;
        sum += (FS_N - k) as i64 * isqrt(best as i64);
        if sum >= stop {
            return sum;
        }
    }
    sum
}

/// The $P cloud distance, per mille of the shape's size: the minimum over starts every 5 points
/// (floor(sqrt(32))) in both directions (a matched to b and b to a). Exact when it is at most
/// bound, bound + 1 otherwise: the early abandoning that keeps 32 shapes fast.
fn cloud_dist(a: &Cloud, b: &Cloud, bound: i32) -> i32 {
    let mut best = bound + 1;
    let step = 5;
    let mut start = 0;
    while start < FS_N {
        for (p, q) in [(a, b), (b, a)] {
            // Each pass need only beat the best so far.
            let stop = abandon_at(best - 1);
            let sum = greedy_pass(p, q, start, stop);
            if sum < stop {
                best = best.min(per_mille(sum));
            }
        }
        start += step;
    }
    best
}

/// The distance between two samples as drawn (no rotation tried), per mille: the measure of a
/// recording's spread and consistency. Exact, and symmetric.
pub fn fs_distance(a: &FsSample, b: &FsSample) -> i32 {
    cloud_dist(&cloud_of(a), &cloud_of(b), UNBOUNDED)
}

/// A shape to be matched: its cloud as drawn and turned by -10 and +10 degrees, and its ratio.
#[derive(Clone, Copy, Debug, Default)]
struct Cand {
    rot: [Cloud; 3],
    ratio: i32,
}

fn cand_of(s: &FsSample) -> Cand {
    let c = cloud_of(s);
    Cand { rot: [c, rotate(&c, -FS_ROT_DEG), rotate(&c, FS_ROT_DEG)], ratio: ratio_of(s) }
}

// ---------------------------------------------------------------------------------------------
// Recorded gestures and the decision (section 4)
// ---------------------------------------------------------------------------------------------

/// A recorded shape, ready to match: its 2-3 samples and what fs_prepare derives from them (never
/// stored, so every device derives the same numbers from the same file).
#[derive(Clone, Copy, Debug)]
pub struct FsGesture {
    /// samples in use (1 only for the record card's first-sample check)
    pub n: usize,
    pub samples: [FsSample; FS_SAMPLES_MAX],
    clouds: [Cloud; FS_SAMPLES_MAX],
    /// the largest pairwise distance among the samples
    pub spread: i32,
    /// the accept threshold T_g, per mille
    pub thr: i32,
    /// the samples' mean path/diagonal ratio, x1000 (the structure guard's reference)
    pub ratio: i32,
}

impl Default for FsGesture {
    fn default() -> Self {
        FsGesture {
            n: 0,
            samples: [FsSample::default(); FS_SAMPLES_MAX],
            clouds: [Cloud::default(); FS_SAMPLES_MAX],
            spread: 0,
            thr: FS_T_MIN,
            ratio: 0,
        }
    }
}

/// The accept threshold for a spread: clamp(1.3 x spread + 30, 70, 95) per mille, rounded. A loose
/// recording can never become a loose gesture: the ceiling holds whatever the spread.
pub fn fs_threshold(spread: i32) -> i32 {
    (div_round(13 * spread as i64, 10) as i32 + 30).clamp(FS_T_MIN, FS_T_MAX)
}

/// Derives a gesture from its samples (at most FS_SAMPLES_MAX are used): the clouds, the spread,
/// T_g and the ratio. About 3 ms per shape on the Kindle; gesturemap does it at load.
pub fn fs_prepare(g: &mut FsGesture, samples: &[FsSample]) {
    *g = FsGesture::default();
    g.n = samples.len().min(FS_SAMPLES_MAX);
    let mut rsum = 0i64;
    for i in 0..g.n {
        g.samples[i] = samples[i];
        g.clouds[i] = cloud_of(&samples[i]);
        rsum += ratio_of(&samples[i]) as i64;
    }
    for i in 0..g.n {
        for j in i + 1..g.n {
            g.spread = g.spread.max(cloud_dist(&g.clouds[i], &g.clouds[j], UNBOUNDED));
        }
    }
    g.thr = fs_threshold(g.spread);
    if g.n > 0 {
        g.ratio = (rsum / g.n as i64) as i32;
    }
}

/// The medoid sample: the one nearest the others in all, which the row's thumbnail draws.
pub fn fs_medoid(g: &FsGesture) -> usize {
    let mut best = (i32::MAX, 0);
    for i in 0..g.n {
        let mut sum = 0;
        for j in 0..g.n {
            if i != j {
                sum += cloud_dist(&g.clouds[i], &g.clouds[j], UNBOUNDED);
            }
        }
        if sum < best.0 {
            best = (sum, i);
        }
    }
    best.1
}

/// D_g(c): the minimum over the gesture's samples and the three rotations, exact up to bound and
/// bound + 1 past it. The turned clouds are tried only when the upright one is within 1.5x the
/// gesture's threshold: a shape that far off is not 10 degrees from matching, and it halves the work.
fn gesture_dist(g: &FsGesture, c: &Cand, bound: i32) -> i32 {
    let rot_gate = 3 * g.thr / 2;
    let mut d = bound.max(rot_gate) + 1;
    for i in 0..g.n {
        d = d.min(cloud_dist(&c.rot[0], &g.clouds[i], d - 1));
    }
    if d <= rot_gate {
        for r in 1..3 {
            for i in 0..g.n {
                d = d.min(cloud_dist(&c.rot[r], &g.clouds[i], d - 1));
            }
        }
    }
    d.min(bound + 1)
}

/// D_g for a sample, exactly (no bound): what the tests assert and the probe can show.
pub fn fs_gesture_distance(g: &FsGesture, s: &FsSample) -> i32 {
    gesture_dist(g, &cand_of(s), UNBOUNDED)
}

/// What fs_match decided, and the numbers behind it (the fshape.last probe: d1, thr, d2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsMatch {
    /// the gesture that fires, or None (why says why not)
    pub fired: Option<usize>,
    pub why: FsReason,
    /// the nearest gesture's index (-1 with none recorded), its distance and threshold
    pub best: i32,
    pub d1: i32,
    pub thr: i32,
    /// the second nearest's distance (FS_FAR + 1 when there is none or it is past FS_FAR)
    pub d2: i32,
}

/// The decision at runtime (after the gates): the nearest gesture fires when it is within its
/// threshold, clearly nearer than the second nearest (another action: one shape per action), and
/// alike in structure. Anything else fires nothing: an unknown shape never does anything. pump is
/// called every FS_MATCH_PUMP samples (work_pump, so the pen's evdev buffer never overflows).
pub fn fs_match(s: &FsSample, gestures: &[FsGesture], pump: &mut dyn FnMut()) -> FsMatch {
    let c = cand_of(s);
    let mut m = FsMatch { fired: None, why: FsReason::NoMatch, best: -1, d1: FS_FAR + 1, thr: 0, d2: FS_FAR + 1 };
    let mut done = 0usize;
    for (i, g) in gestures.iter().enumerate() {
        if g.n == 0 {
            continue;
        }
        let d = gesture_dist(g, &c, FS_FAR);
        if m.best < 0 || d < m.d1 {
            m.d2 = m.d1;
            m.d1 = d;
            m.best = i as i32;
            m.thr = g.thr;
        } else if d < m.d2 {
            m.d2 = d;
        }
        let was = done / FS_MATCH_PUMP;
        done += g.n;
        if done / FS_MATCH_PUMP != was {
            pump();
        }
    }
    if m.best < 0 || m.d1 > m.thr {
        return m;
    }
    let (d1, d2) = (m.d1, m.d2);
    if d2 - d1 < FS_MARGIN_MIN || (d2 - d1) * 100 < FS_MARGIN_PCT * d1 {
        m.why = FsReason::Unclear;
        return m;
    }
    let g = &gestures[m.best as usize];
    if c.ratio * 10 < FS_GUARD_LO_X10 * g.ratio || c.ratio * 10 > FS_GUARD_HI_X10 * g.ratio {
        m.why = FsReason::Structure;
        return m;
    }
    m.fired = Some(m.best as usize);
    m.why = FsReason::None;
    m
}

// ---------------------------------------------------------------------------------------------
// Recording (sections 5 and 6.3)
// ---------------------------------------------------------------------------------------------

/// Slip-bank paths: one-turn slides (16 turns x 5 corner places x 18 orientations), then arcs
/// (10 sweeps x 18 orientations).
const BANK_TURNS: [i32; 16] = [30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 175];
const BANK_CORNER_PCT: [i32; 5] = [20, 35, 50, 65, 80];
const BANK_ORIENTS: usize = 18;
const BANK_ARCS: usize = 10;
const BANK_SLIDES: usize = BANK_TURNS.len() * BANK_CORNER_PCT.len() * BANK_ORIENTS;
/// The length of a bank path, in bank units (only its shape matters: matching ignores scale).
const BANK_LEN: i64 = 4000;

/// How many paths the slip bank holds (1620).
pub fn fs_bank_len() -> usize {
    BANK_SLIDES + BANK_ARCS * BANK_ORIENTS
}

/// Bank path i as a polyline (out needs FS_BANK_PTS points); returns its point count, 0 past the
/// end. What a single slip of the finger draws: a slide that turns once (30-175 degrees, the
/// corner at 20-80% of the way), or an arc (40-220 degrees), every 20 degrees of orientation (the
/// +-10 degree tolerance fills the gaps). Generated on the fly: there is no table. A slide's
/// mirror image is the same cloud as another slide drawn backward (a corner at 1 - f), and an arc
/// is its own mirror, so one turning sense covers both.
pub fn fs_bank_path(i: usize, out: &mut [FsPt; FS_BANK_PTS]) -> usize {
    if i >= fs_bank_len() {
        return 0;
    }
    let orient = (i % BANK_ORIENTS) as i32 * 20;
    let rest = i / BANK_ORIENTS;
    // A turtle: from the origin, heading `orient`, legs of a length, turning between them.
    let mut x = 0i64;
    let mut y = 0i64;
    let mut heading = orient;
    out[0] = FsPt::default();
    let mut leg = |n: &mut usize, len: i64, heading: i32| {
        x += div_round(len * fs_cos_q14(heading) as i64, 16384);
        y += div_round(len * fs_sin_q14(heading) as i64, 16384);
        out[*n] = FsPt { x: x as i32, y: y as i32 };
        *n += 1;
    };
    let mut n = 1;
    if i < BANK_SLIDES {
        let corner = BANK_CORNER_PCT[rest % BANK_CORNER_PCT.len()] as i64;
        let turn = BANK_TURNS[rest / BANK_CORNER_PCT.len()];
        leg(&mut n, BANK_LEN * corner / 100, heading);
        heading += turn;
        leg(&mut n, BANK_LEN * (100 - corner) / 100, heading);
    } else {
        let sweep = 40 + 20 * (rest - BANK_SLIDES / BANK_ORIENTS) as i32;
        let segs = sweep / 5;
        // (each leg turned half a step in, so the arc's chord lies along `orient`)
        heading -= sweep / 2 - 2;
        for _ in 0..segs {
            leg(&mut n, BANK_LEN / segs as i64, heading);
            heading += 5;
        }
    }
    n
}

/// Whether any slip-bank path comes within thr of the gesture: the first such path's index.
/// pump is called every FS_BANK_PUMP paths.
pub fn fs_bank_hit(g: &FsGesture, thr: i32, pump: &mut dyn FnMut()) -> Option<usize> {
    let mut path = [FsPt::default(); FS_BANK_PTS];
    let mut s = FsSample::default();
    for i in 0..fs_bank_len() {
        if i > 0 && i % FS_BANK_PUMP == 0 {
            pump();
        }
        let n = fs_bank_path(i, &mut path);
        fs_sample_of(&path[..n], &mut s);
        if gesture_dist(g, &cand_of(&s), thr) <= thr {
            return Some(i);
        }
    }
    None
}

/// Sample 1's quick check (6.3): the gates have passed; refuses at once a shape a slip reaches at
/// T_MIN, so a V is refused after one try instead of three.
pub fn fs_record_first(s: &FsSample, pump: &mut dyn FnMut()) -> Result<(), FsReason> {
    let mut g = FsGesture::default();
    fs_prepare(&mut g, std::slice::from_ref(s));
    match fs_bank_hit(&g, FS_T_MIN, pump) {
        Some(_) => Err(FsReason::Slip),
        None => Ok(()),
    }
}

/// Samples 2 and 3: each must lie within FS_CONSIST_MAX of an earlier sample.
pub fn fs_record_consistent(s: &FsSample, earlier: &[FsSample]) -> Result<(), FsReason> {
    if earlier.is_empty() || earlier.iter().any(|e| fs_distance(s, e) <= FS_CONSIST_MAX) {
        return Ok(());
    }
    Err(FsReason::Inconsistent)
}

/// Room for one more shape (fs_record_* for a new action when nshapes are recorded).
pub fn fs_record_room(nshapes: usize) -> Result<(), FsReason> {
    if nshapes >= FS_SHAPES_MAX {
        return Err(FsReason::Full);
    }
    Ok(())
}

/// The separation verdict for a new shape against the other actions' shapes (6.3a).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsSeparation {
    Clear,
    /// within that shape's own threshold: it is that action's shape, and Use moves it here
    SameAs(usize),
    /// near enough to be confused with it: the samples are discarded
    TooClose(usize),
}

/// Separation (6.3a): d = the minimum distance from the new samples to another action's shape h.
/// d <= T_h is h's shape (SameAs); d < max(T_h, T_new) + FS_SEPARATE_PAD is too close. A SameAs
/// anywhere wins over a TooClose; among each, the nearest. others holds every other action's
/// shape (the action's own old shape, which the new one replaces, left out).
pub fn fs_record_separation(new: &FsGesture, others: &[FsGesture]) -> FsSeparation {
    let mut same: Option<(i32, usize)> = None;
    let mut close: Option<(i32, usize)> = None;
    let bound = FS_T_MAX + FS_SEPARATE_PAD;
    for (k, h) in others.iter().enumerate() {
        if h.n == 0 {
            continue;
        }
        let mut d = bound + 1;
        for i in 0..new.n {
            d = d.min(gesture_dist(h, &cand_of(&new.samples[i]), bound));
        }
        if d <= h.thr {
            if same.is_none_or(|(sd, _)| d < sd) {
                same = Some((d, k));
            }
        } else if d < h.thr.max(new.thr) + FS_SEPARATE_PAD && close.is_none_or(|(cd, _)| d < cd) {
            close = Some((d, k));
        }
    }
    match (same, close) {
        (Some((_, k)), _) => FsSeparation::SameAs(k),
        (None, Some((_, k))) => FsSeparation::TooClose(k),
        (None, None) => FsSeparation::Clear,
    }
}

/// The full slip-bank check before Use (6.3b), at the gesture's own threshold.
pub fn fs_record_bank(g: &FsGesture, pump: &mut dyn FnMut()) -> Result<(), FsReason> {
    match fs_bank_hit(g, g.thr, pump) {
        Some(_) => Err(FsReason::Slip),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------------
// The gestures.ini shape value (section 8)
// ---------------------------------------------------------------------------------------------

/// The value of a shape.<action> line: format 1.
pub const FS_FORMAT: u32 = 1;

/// Why a shape value cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum FsCodec {
    /// a format 1 value with the wrong count, bad hex or a degenerate sample: refused and counted,
    /// gone at the next write
    Malformed,
    /// another format (a newer Grimoire's): carried verbatim and written back
    Newer,
}

/// Writes "1,N,<128 hex>,..." (lowercase) for 2 or 3 samples into out (replacing it).
pub fn fs_encode(samples: &[FsSample], out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.clear();
    let n = samples.len().min(FS_SAMPLES_MAX);
    out.push_str("1,");
    out.push((b'0' + n as u8) as char);
    for s in &samples[..n] {
        out.push(',');
        for &b in &s.xy {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
    }
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Reads a shape value into out; returns the sample count (2 or 3). The hex is case-insensitive.
/// A sample whose box is under 32 units on both axes is degenerate (a written sample's larger side
/// always spans 0-255).
pub fn fs_decode(v: &str, out: &mut [FsSample; FS_SAMPLES_MAX]) -> Result<usize, FsCodec> {
    let mut fields = v.split(',');
    let ver = fields.next().unwrap_or("");
    if ver.is_empty() || ver.len() > 9 || !ver.bytes().all(|c| c.is_ascii_digit()) {
        return Err(FsCodec::Malformed);
    }
    let ver: u32 = ver.parse().map_err(|_| FsCodec::Malformed)?;
    if ver == 0 {
        return Err(FsCodec::Malformed);
    }
    if ver != FS_FORMAT {
        return Err(FsCodec::Newer);
    }
    let n = match fields.next() {
        Some("2") => 2,
        Some("3") => 3,
        _ => return Err(FsCodec::Malformed),
    };
    for s in out.iter_mut().take(n) {
        let f = fields.next().ok_or(FsCodec::Malformed)?.as_bytes();
        if f.len() != 4 * FS_N {
            return Err(FsCodec::Malformed);
        }
        for i in 0..2 * FS_N {
            let hi = hexval(f[2 * i]).ok_or(FsCodec::Malformed)?;
            let lo = hexval(f[2 * i + 1]).ok_or(FsCodec::Malformed)?;
            s.xy[i] = hi << 4 | lo;
        }
        let (mut minx, mut maxx, mut miny, mut maxy) = (255u8, 0u8, 255u8, 0u8);
        for i in 0..FS_N {
            minx = minx.min(s.xy[2 * i]);
            maxx = maxx.max(s.xy[2 * i]);
            miny = miny.min(s.xy[2 * i + 1]);
            maxy = maxy.max(s.xy[2 * i + 1]);
        }
        if maxx - minx < 32 && maxy - miny < 32 {
            return Err(FsCodec::Malformed);
        }
    }
    if fields.next().is_some() {
        return Err(FsCodec::Malformed);
    }
    Ok(n)
}
