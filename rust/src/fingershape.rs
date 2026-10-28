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
/// The bank refuses a shape that any bank path comes within its threshold plus this of. The bank's
/// paths are ideal and on a grid (20 degrees of orientation, 10 of turn, 15% of corner place),
/// while a real slip lands between the grid's points and wobbles: either can bring it nearer than
/// the nearest bank path. 10 is the least pad that refuses a C (a 270 degree arc, which the bank's
/// 220 degree arcs reach only within T + 10) in every recording, as the design requires of every
/// one-turn shape; longer bank arcs would refuse circles. A design tuning, to be confirmed with
/// the device traces at the P4 freeze like the other bank constants.
pub const FS_BANK_PAD: i32 = 10;
/// fs_match calls its pump after this many comparisons of a recorded sample with one try of the
/// candidate (the work_pump rhythm, section 4).
pub const FS_MATCH_PUMP: usize = 4;
/// In recording, a refused straight stroke and another landing within this read as an X or = drawn
/// in two strokes (section 4, Multi-stroke).
pub const FS_TWO_STROKE_MS: u32 = 700;
/// Vertices of the longest slip-bank path (a 220 degree arc in 5 degree segments).
pub const FS_BANK_PTS: usize = 48;
/// Coordinates are clamped into +-this (about a million px, hundreds of screens; the off-screen
/// cancel's -10000 is well inside). Squared distances of clamped points, even times 256 for the
/// 1/16 px lengths, stay far below i64's range, so no input can overflow the arithmetic.
pub const FS_COORD_MAX: i32 = 1 << 20;
/// The screen scale is clamped into 1..=this px per 100 mm (1000 px/mm), for the same reason.
pub const FS_PX100_MAX: i32 = 100_000;

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

/// floor(sqrt(v)), bit by bit: exact on every target. It starts at the highest power of 4 not
/// above v (from its leading bit), so a 32-point distance takes about 11 steps, not 32.
fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut op = v as u64;
    let mut res = 0u64;
    let mut one = 1u64 << ((63 - op.leading_zeros()) & !1);
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

/// The squared distance of two points that are within +-FS_COORD_MAX (every point this module
/// keeps is clamped there first): at most 2^43, so callers may multiply it by 256 and more.
fn dist2(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = bx as i64 - ax as i64;
    let dy = by as i64 - ay as i64;
    dx * dx + dy * dy
}

/// A point from outside, clamped into +-FS_COORD_MAX.
fn clamp_pt(x: i32, y: i32) -> FsPt {
    FsPt { x: x.clamp(-FS_COORD_MAX, FS_COORD_MAX), y: y.clamp(-FS_COORD_MAX, FS_COORD_MAX) }
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
/// shape includes its own start. px_per_100mm is the screen's scale (fs_mm_px). Coordinates are
/// clamped into +-FS_COORD_MAX and the scale into 1..=FS_PX100_MAX.
pub fn fs_begin(s: &mut FsStroke, x: i32, y: i32, t: u32, px_per_100mm: i32) {
    *s = FsStroke::default();
    let p = clamp_pt(x, y);
    s.state = FsState::Pending;
    s.frames = 1;
    s.pts[0] = p;
    s.n = 1;
    s.last = p;
    s.t0 = t;
    s.anchor = p;
    s.anchor_t = t;
    s.minx = p.x;
    s.maxx = p.x;
    s.miny = p.y;
    s.maxy = p.y;
    s.px100 = px_per_100mm.clamp(1, FS_PX100_MAX);
}

fn die(s: &mut FsStroke, why: FsReason) {
    s.state = FsState::Dead;
    s.dead = why;
}

/// Pending to Drawing: the finger left the slop (within the lead, or it would be Dead) and the
/// two-finger landing window is over.
fn promote(s: &mut FsStroke, t: u32) {
    if s.state == FsState::Pending && s.exited && t.wrapping_sub(s.t0) >= FS_GATES.trail_ms {
        s.state = FsState::Drawing;
    }
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
    let p = clamp_pt(x, y);
    let (x, y) = (p.x, p.y);
    s.frames = s.frames.saturating_add(1);
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
    // A stroke that left the slop is a shape from trail_ms on, whether or not a tick came to say
    // so. Promote it before the pause test: when ticks did not arrive (a render backlog, a replay
    // of frames only), the rest that this frame ends may have begun while it was still Pending,
    // and it is a shape's pause all the same. Ticks or no ticks, the verdict is the same.
    promote(s, t);
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
    promote(s, now);
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
/// the session is Idle afterwards (its points and box stay readable for the trail, and for
/// fs_two_strokes at the next landing).
pub fn fs_finish(s: &mut FsStroke, t_lift: u32, mode: FsMode, out: &mut FsSample) -> FsLift {
    fs_tick(s, t_lift);
    let state = s.state;
    s.state = FsState::Idle;
    match state {
        FsState::Idle => FsLift::Nothing,
        // A finger that left the slop in time and lifted inside the two-finger landing window,
        // with no cancel: a quick flick (100-150 ms is a normal one-finger swipe). No second
        // finger can come any more, so a straight flick is judged as a swipe like a slower one.
        // Anything else stays nothing: a shape never fires without the trail having shown it.
        FsState::Pending if s.exited => match gate(s, t_lift, fs_gates(mode)) {
            FsLift::Swipe(d) => FsLift::Swipe(d),
            _ => FsLift::Nothing,
        },
        FsState::Pending => FsLift::Nothing,
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
    // Mean speed over the drawn part, read as section 3 words it ("path length / time from the
    // 6 mm exit to the lift"): the whole path, its first 6 mm included, over the time since the
    // exit. In integers: path16 / 16 px * 100 / px100 mm * 1000 / dt >= speed.
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

/// Whether a landing starts the second stroke of an X or = drawn in two strokes (recording only),
/// asked at the landing before fs_begin reuses the stroke: the previous stroke was a bar and this
/// finger landed within FS_TWO_STROKE_MS of its lift. A bar is a straight stroke that left the
/// slop, whatever its lift made of it: too straight, a swipe (a quick bar), too small or too few
/// frames (a short one), or nothing (a flick lifted inside the two-finger window). A stroke that
/// died (a late start, a pause, a palm) is not a bar: its points stop where it died.
pub fn fs_two_strokes(prev: &FsStroke, prev_lift: FsLift, prev_lift_ms: u32, land_ms: u32) -> bool {
    let bar_lift = matches!(
        prev_lift,
        FsLift::Nothing
            | FsLift::Swipe(_)
            | FsLift::Refused(FsReason::Straight)
            | FsLift::Refused(FsReason::TooSmall)
            | FsLift::Refused(FsReason::Frames)
    );
    if !bar_lift || !prev.exited || prev.dead != FsReason::None || prev.n < 2 {
        return false;
    }
    let major = ((prev.maxx - prev.minx) as i64).max((prev.maxy - prev.miny) as i64);
    straight(prev, major, &FS_GATES) && land_ms.wrapping_sub(prev_lift_ms) <= FS_TWO_STROKE_MS
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
/// them into an FsSample. Streaming, in two passes, so a path of any length needs no buffer. The
/// points are clamped into +-FS_COORD_MAX as they are read.
pub fn fs_sample_of(pts: &[FsPt], out: &mut FsSample) {
    *out = FsSample::default();
    if pts.is_empty() {
        return;
    }
    let pt = |i: usize| clamp_pt(pts[i].x, pts[i].y);
    let seg = |i: usize| -> i64 {
        let (a, b) = (pt(i), pt(i + 1));
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
        let (a, b) = (pt(i), pt(i + 1));
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
    let end = pt(pts.len() - 1);
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

/// A point cloud: the sample's centroid at 0 and its larger box side 512 units. Every point lies
/// within 512 of the centroid (the centroid is inside the box), so the coordinates are i16 and
/// the difference of two fits too: the distance loops then map onto the widening multiply-adds
/// the targets have (NEON's vmlal.s16 on the Kindle and the Mac, SSE2's pmaddwd on a PC).
#[derive(Clone, Copy, Debug, Default)]
struct Cloud {
    x: [i16; FS_N],
    y: [i16; FS_N],
}

/// The sample's cloud, turned by deg degrees about its centroid (0: as drawn), and then scaled so
/// that the larger side of its box is 512 units again. Turning changes the box (a squarish Z
/// turned by 10 degrees has a larger one, a long thin shape a shorter one), so a turned try left
/// at the upright scale is compared with the recorded clouds at another size than theirs, and the
/// tolerance shrinks, by a different amount for each way of turning. Normalizing after the turn
/// compares like with like.
fn cloud_of(s: &FsSample, deg: i32) -> Cloud {
    let mut c = Cloud::default();
    let (mut sx, mut sy) = (0i64, 0i64);
    for i in 0..FS_N {
        sx += s.xy[2 * i] as i64;
        sy += s.xy[2 * i + 1] as i64;
    }
    // The points about the centroid, times 32 so the mean stays exact, turned in Q14. The turned
    // centroid is still exactly 0: the turned sums are sin and cos times sums that are 0.
    let (cs, sn) = (fs_cos_q14(deg) as i64, fs_sin_q14(deg) as i64);
    let mut u = [0i64; FS_N];
    let mut v = [0i64; FS_N];
    for i in 0..FS_N {
        let x = FS_N as i64 * s.xy[2 * i] as i64 - sx;
        let y = FS_N as i64 * s.xy[2 * i + 1] as i64 - sy;
        u[i] = x * cs - y * sn;
        v[i] = x * sn + y * cs;
    }
    let (mut minx, mut maxx, mut miny, mut maxy) = (u[0], u[0], v[0], v[0]);
    for i in 1..FS_N {
        minx = minx.min(u[i]);
        maxx = maxx.max(u[i]);
        miny = miny.min(v[i]);
        maxy = maxy.max(v[i]);
    }
    let major = (maxx - minx).max(maxy - miny);
    if major == 0 {
        return c;
    }
    // (At 0 degrees this is (32 x - sum) * 16 / major in sample units: the same numbers, so the
    // as-drawn distances do not depend on the turned tries existing.)
    // (|u| <= major, as the centroid is inside the box: so |x| <= 512)
    for i in 0..FS_N {
        c.x[i] = div_round(u[i] * FS_SCALE as i64, major) as i16;
        c.y[i] = div_round(v[i] * FS_SCALE as i64, major) as i16;
    }
    c
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

// (greedy_pass keeps a point's index in a key's low 5 bits)
const _: () = assert!(FS_N <= 32);

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
///
/// The search packs each candidate into one key, squared distance above and index below, so the
/// least key is the nearest point and the first on a tie; a taken point's key is u32::MAX. That is
/// the same choice as a scan with a strict "<", without a branch per point, so the compiler can
/// run it in vector lanes. A cloud's points lie within 512 of 0 (cloud_of), so a squared distance
/// is at most 2 x 1024^2 = 2^21 and its key fits below 2^26.
fn greedy_pass(a: &Cloud, b: &Cloud, start: usize, stop: i64) -> i64 {
    let mut taken = [0u32; FS_N];
    let mut sum = 0i64;
    for k in 0..FS_N {
        let i = (start + k) % FS_N;
        let (ax, ay) = (a.x[i], a.y[i]);
        let mut key = u32::MAX;
        for j in 0..FS_N {
            let (dx, dy) = ((b.x[j] - ax) as i32, (b.y[j] - ay) as i32);
            let d = (dx * dx + dy * dy) as u32;
            key = key.min((d << 5 | j as u32) | taken[j]);
        }
        let bj = (key & 31) as usize;
        taken[bj] = u32::MAX;
        sum += (FS_N - k) as i64 * isqrt((key >> 5) as i64);
        if sum >= stop {
            return sum;
        }
    }
    sum
}

/// A weighted sum no greedy pass between a and b can go below, in either direction. In a pass
/// each point is matched at least as far as its nearest neighbour in the other cloud, whatever is
/// taken, and the weights 32..1 fall on the points in some order; the least such sum puts the
/// largest weights on the nearest neighbours (the rearrangement inequality). So the sum of the
/// sorted nearest-neighbour distances, weighted 32..1, bounds every pass one way, and the lesser
/// of the two ways bounds cloud_dist. It costs about one pass, and a pair it puts past the bound
/// needs none of the fourteen.
fn lower_sum(a: &Cloud, b: &Cloud) -> i64 {
    let mut na = [u32::MAX; FS_N];
    let mut nb = [u32::MAX; FS_N];
    for i in 0..FS_N {
        for j in 0..FS_N {
            let (dx, dy) = ((b.x[j] - a.x[i]) as i32, (b.y[j] - a.y[i]) as i32);
            let d = (dx * dx + dy * dy) as u32;
            na[i] = na[i].min(d);
            nb[j] = nb[j].min(d);
        }
    }
    // (isqrt keeps the order, so sorting the squares sorts the distances)
    na.sort_unstable();
    nb.sort_unstable();
    let (mut sa, mut sb) = (0i64, 0i64);
    for k in 0..FS_N {
        sa += (FS_N - k) as i64 * isqrt(na[k] as i64);
        sb += (FS_N - k) as i64 * isqrt(nb[k] as i64);
    }
    sa.min(sb)
}

/// The $P cloud distance, per mille of the shape's size: the minimum over starts every 5 points
/// (floor(sqrt(32))) in both directions (a matched to b and b to a). Exact when it is at most
/// bound, bound + 1 otherwise: the early abandoning (and lower_sum's shortcut) that keeps 32
/// shapes fast.
fn cloud_dist(a: &Cloud, b: &Cloud, bound: i32) -> i32 {
    if bound < UNBOUNDED && lower_sum(a, b) >= abandon_at(bound) {
        return bound + 1;
    }
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
    cloud_dist(&cloud_of(a, 0), &cloud_of(b, 0), UNBOUNDED)
}

/// A sample's path over its box diagonal, x1000 (the structure guard's measure; ratio_of).
pub fn fs_ratio(s: &FsSample) -> i32 {
    ratio_of(s)
}

/// A shape to be matched: its cloud as drawn and turned by -10 and +10 degrees, and its ratio.
#[derive(Clone, Copy, Debug, Default)]
struct Cand {
    rot: [Cloud; 3],
    ratio: i32,
}

fn cand_of(s: &FsSample) -> Cand {
    Cand { rot: [cloud_of(s, 0), cloud_of(s, -FS_ROT_DEG), cloud_of(s, FS_ROT_DEG)], ratio: ratio_of(s) }
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
        g.clouds[i] = cloud_of(&samples[i], 0);
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

/// D_g(c) over the turns in `rots` (indices into c.rot: 0 as drawn, 1 and 2 turned): the minimum
/// over those and the gesture's samples, exact up to bound and bound + 1 past it. Every turn is
/// always tried: a shortcut that skips the turned tries when the upright one is far would make
/// D_g depend on the shortcut (a Z turned 15 degrees would read 121 where its turned try reads 40),
/// and separation and the slip bank must see the very distance the matcher can reach.
fn gesture_dist_rots(g: &FsGesture, c: &Cand, rots: core::ops::Range<usize>, bound: i32) -> i32 {
    let mut d = bound + 1;
    for r in rots {
        for i in 0..g.n {
            d = d.min(cloud_dist(&c.rot[r], &g.clouds[i], d - 1));
        }
    }
    d
}

/// D_g(c): the minimum over the gesture's samples and the candidate's three turns (section 4),
/// exact up to bound and bound + 1 past it.
fn gesture_dist(g: &FsGesture, c: &Cand, bound: i32) -> i32 {
    gesture_dist_rots(g, c, 0..3, bound)
}

/// D_g for a sample, exactly (no bound): the distance fs_match ranks by, and what separation and
/// the slip bank measure.
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
    /// the candidate's path/diagonal ratio, x1000 (fs_ratio), and the nearest gesture's
    pub ratio: i32,
    pub g_ratio: i32,
}

/// The decision from the numbers (section 4, steps 2 to 4): None to fire, or why not. d1 and thr
/// are the nearest gesture's distance and threshold, d2 the second nearest's (another action's),
/// ratio and g_ratio the candidate's and that gesture's path/diagonal ratios.
/// - d1 <= thr, or NoMatch;
/// - the margin d2 - d1 >= max(35, 30% of d1), or Unclear (as d1 <= thr <= 95 here, 30% of d1
///   is at most 28 and the 35 always binds; the percentage is kept as the design states it, for a
///   retuned FS_T_MAX);
/// - ratio within 0.6x-1.6x of g_ratio, both ends included, or Structure: a spiral is not a circle.
pub fn fs_decide(d1: i32, thr: i32, d2: i32, ratio: i32, g_ratio: i32) -> FsReason {
    if d1 > thr {
        return FsReason::NoMatch;
    }
    if d2 - d1 < FS_MARGIN_MIN || (d2 - d1) * 100 < FS_MARGIN_PCT * d1 {
        return FsReason::Unclear;
    }
    if ratio * 10 < FS_GUARD_LO_X10 * g_ratio || ratio * 10 > FS_GUARD_HI_X10 * g_ratio {
        return FsReason::Structure;
    }
    FsReason::None
}

/// The decision at runtime (after the gates): the nearest gesture fires when it is within its
/// threshold, clearly nearer than the second nearest (another action: one shape per action), and
/// alike in structure (fs_decide). Anything else fires nothing: an unknown shape never does
/// anything. d1 and d2 are the exact D_g of fs_gesture_distance up to FS_FAR (FS_FAR + 1 past
/// it), so the margin sees every turn of every gesture. At most FS_SHAPES_MAX gestures are read.
/// pump is called after a gesture whenever the comparisons so far (a recorded sample against one
/// of the candidate's three tries) have passed another multiple of FS_MATCH_PUMP (work_pump, so
/// the pen's evdev buffer never overflows: a gesture is at most 9 comparisons).
pub fn fs_match(s: &FsSample, gestures: &[FsGesture], pump: &mut dyn FnMut()) -> FsMatch {
    let c = cand_of(s);
    let mut m = FsMatch {
        fired: None,
        why: FsReason::NoMatch,
        best: -1,
        d1: FS_FAR + 1,
        thr: 0,
        d2: FS_FAR + 1,
        ratio: c.ratio,
        g_ratio: 0,
    };
    let gestures = &gestures[..gestures.len().min(FS_SHAPES_MAX)];
    let mut done = 0usize;
    let mut tick = |n: usize| {
        let was = done / FS_MATCH_PUMP;
        done += n;
        if done / FS_MATCH_PUMP != was {
            pump();
        }
    };
    // First a cheap bound: one greedy pass per sample (the first start, as drawn) is one of the
    // passes D_g takes the least of, so it bounds D_g from above, at a fortieth of the cost.
    let n = gestures.len();
    let mut d = [UNBOUNDED; FS_SHAPES_MAX];
    for (i, g) in gestures.iter().enumerate() {
        for j in 0..g.n {
            d[i] = d[i].min(per_mille(greedy_pass(&c.rot[0], &g.clouds[j], 0, i64::MAX)));
        }
    }
    // Then each D_g exactly, every turn, nearest bound first. A gesture need only be measured up
    // to the second smallest value so far (an upper bound of the final d2): past that it can be
    // neither of the two nearest, which is all the decision reads, and the measure abandons it
    // early. So d1 and d2 come out exactly as one plain pass over all of them would give (ties
    // keep the earlier gesture), and far gestures cost little.
    let mut order = [0usize; FS_SHAPES_MAX];
    for (k, o) in order.iter_mut().enumerate() {
        *o = k;
    }
    let order = &mut order[..n];
    order.sort_unstable_by_key(|&i| (d[i], i));
    for &i in order.iter() {
        let g = &gestures[i];
        if g.n == 0 {
            continue;
        }
        // (D_g <= d[i], so a bound of d[i] or more is always exact; a bound of the second
        // smallest gives that bound + 1 for a gesture past it, which stays past it)
        let b = d[i].min(second_least(&d[..n])).min(FS_FAR);
        d[i] = gesture_dist(g, &c, b);
        tick(3 * g.n);
    }
    for (i, g) in gestures.iter().enumerate() {
        if g.n == 0 {
            continue;
        }
        let di = d[i].min(FS_FAR + 1);
        if m.best < 0 || di < m.d1 {
            m.d2 = m.d1;
            m.d1 = di;
            m.best = i as i32;
            m.thr = g.thr;
            m.g_ratio = g.ratio;
        } else if di < m.d2 {
            m.d2 = di;
        }
    }
    if m.best < 0 {
        return m;
    }
    m.why = fs_decide(m.d1, m.thr, m.d2, m.ratio, m.g_ratio);
    if m.why == FsReason::None {
        m.fired = Some(m.best as usize);
    }
    m
}

/// The second smallest value (the smallest twice when it repeats); FS_FAR + 1 for fewer than two.
fn second_least(d: &[i32]) -> i32 {
    let (mut a, mut b) = (FS_FAR + 1, FS_FAR + 1);
    for &v in d {
        if v < a {
            b = a;
            a = v;
        } else if v < b {
            b = v;
        }
    }
    b
}

// ---------------------------------------------------------------------------------------------
// Recording (sections 5 and 6.3)
// ---------------------------------------------------------------------------------------------

/// Slip-bank paths, each at 18 orientations: one-turn slides (16 turns x 5 corner places), arcs
/// (10 sweeps), then hairpins (2 senses x 2 return lengths x 2 tip widths).
const BANK_TURNS: [i32; 16] = [30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 175];
const BANK_CORNER_PCT: [i32; 5] = [20, 35, 50, 65, 80];
const BANK_ORIENTS: usize = 18;
const BANK_ARCS: usize = 10;
const BANK_PIN_BACK_PCT: [i64; 2] = [60, 100];
const BANK_PIN_TIP_PCT: [i64; 2] = [6, 14];
const BANK_PINS: usize = 2 * BANK_PIN_BACK_PCT.len() * BANK_PIN_TIP_PCT.len();
const BANK_SLIDES: usize = BANK_TURNS.len() * BANK_CORNER_PCT.len() * BANK_ORIENTS;
const BANK_ARCS_END: usize = BANK_SLIDES + BANK_ARCS * BANK_ORIENTS;
/// The length of a bank path, in bank units (only its shape matters: matching ignores scale).
const BANK_LEN: i64 = 4000;

/// How many paths the slip bank holds (1764).
pub fn fs_bank_len() -> usize {
    BANK_ARCS_END + BANK_PINS * BANK_ORIENTS
}

/// Bank path i as a polyline (out needs FS_BANK_PTS points); returns its point count, 0 past the
/// end. What a single slip of the finger draws, every 20 degrees of orientation (the +-10 degree
/// tries fill the gaps), generated on the fly: there is no table.
/// - A slide that turns once: 30-175 degrees, the corner at 20-80% of the way. Its mirror image is
///   the same cloud as another slide drawn backward (a corner at 1 - f), so one sense covers both.
/// - An arc: 40-220 degrees, its own mirror.
/// - A hairpin: out and back round a rounded tip (6% or 14% of the way out across), coming back
///   60% or all of the way, the legs parallel; both senses, as the return lengths are not
///   symmetric. A slide's sharpest turn has a pointed tip, and a finger that turns all the way
///   round draws a round one: the design's "one-turn slide or hairpin". (Hairpins whose legs
///   spread 15 degrees come within reach of most S recordings, a shape with two turns that the
///   design keeps available and whose refusal would say "Give it two turns or a loop" to a shape
///   that has them, so the bank holds the parallel ones.)
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
    let mut n = 1;
    let mut leg = |n: &mut usize, len: i64, heading: i32| {
        x += div_round(len * fs_cos_q14(heading) as i64, 16384);
        y += div_round(len * fs_sin_q14(heading) as i64, 16384);
        out[*n] = FsPt { x: x as i32, y: y as i32 };
        *n += 1;
    };
    if i < BANK_SLIDES {
        let corner = BANK_CORNER_PCT[rest % BANK_CORNER_PCT.len()] as i64;
        let turn = BANK_TURNS[rest / BANK_CORNER_PCT.len()];
        leg(&mut n, BANK_LEN * corner / 100, heading);
        heading += turn;
        leg(&mut n, BANK_LEN * (100 - corner) / 100, heading);
    } else if i < BANK_ARCS_END {
        let sweep = 40 + 20 * (rest - BANK_SLIDES / BANK_ORIENTS) as i32;
        let segs = sweep / 5;
        // (each leg turned half a step in, so the arc's chord lies along `orient`)
        heading -= sweep / 2 - 2;
        for _ in 0..segs {
            leg(&mut n, BANK_LEN / segs as i64, heading);
            heading += 5;
        }
    } else {
        let k = rest - BANK_ARCS_END / BANK_ORIENTS;
        let tip = BANK_PIN_TIP_PCT[k % BANK_PIN_TIP_PCT.len()];
        let back = BANK_PIN_BACK_PCT[k / BANK_PIN_TIP_PCT.len() % BANK_PIN_BACK_PCT.len()];
        let sense = if k / (BANK_PIN_TIP_PCT.len() * BANK_PIN_BACK_PCT.len()) == 0 { 1 } else { -1 };
        // Out, round the tip (a half circle, 12 points on it, 15 degrees apart), and back: the
        // circle's centre is a radius to the turning side of the way out's end.
        let out_len = BANK_LEN / 2;
        let r = out_len * tip / 200;
        leg(&mut n, out_len, heading);
        let (ex, ey) = (out[1].x as i64, out[1].y as i64);
        let side = heading + sense * 90;
        let (cx, cy) = (ex + div_round(r * fs_cos_q14(side) as i64, 16384), ey + div_round(r * fs_sin_q14(side) as i64, 16384));
        for s in 1..=12 {
            let a = side + 180 + sense * 15 * s;
            out[n] = FsPt {
                x: (cx + div_round(r * fs_cos_q14(a) as i64, 16384)) as i32,
                y: (cy + div_round(r * fs_sin_q14(a) as i64, 16384)) as i32,
            };
            n += 1;
        }
        let (tx, ty) = (out[n - 1].x as i64, out[n - 1].y as i64);
        let h2 = heading + sense * 180;
        let back_len = out_len * back / 100;
        out[n] = FsPt {
            x: (tx + div_round(back_len * fs_cos_q14(h2) as i64, 16384)) as i32,
            y: (ty + div_round(back_len * fs_sin_q14(h2) as i64, 16384)) as i32,
        };
        n += 1;
    }
    n
}

/// Whether any slip-bank path comes within thr + FS_BANK_PAD of the gesture, by the very D_g the
/// matcher ranks by (every turn tried): the first such path's index. pump is called every
/// FS_BANK_PUMP paths.
pub fn fs_bank_hit(g: &FsGesture, thr: i32, pump: &mut dyn FnMut()) -> Option<usize> {
    let mut path = [FsPt::default(); FS_BANK_PTS];
    let mut s = FsSample::default();
    let reach = thr + FS_BANK_PAD;
    for i in 0..fs_bank_len() {
        if i > 0 && i % FS_BANK_PUMP == 0 {
            pump();
        }
        let n = fs_bank_path(i, &mut path);
        fs_sample_of(&path[..n], &mut s);
        if gesture_dist(g, &cand_of(&s), reach) <= reach {
            return Some(i);
        }
    }
    None
}

/// Sample 1's quick check (6.3): the gates have passed; refuses at once a shape a slip reaches at
/// T_MIN (plus the bank's pad), so a V is refused after one try instead of three. It never
/// refuses what the full check would pass: T_g >= T_MIN, and a gesture's D_g is at most its first
/// sample's distance.
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

/// The distance between two recorded shapes, for separation: the least over every pair of their
/// samples and every turn, taken both ways (a's samples turned against b's, and b's turned against
/// a's). Turning one shape by +10 degrees is not exactly turning the other by -10 in integers, and
/// the least of both makes the distance symmetric, so a verdict cannot depend on which of the two
/// was recorded first. Exact up to bound (bound + 1 past it).
fn pair_dist(a: &FsGesture, b: &FsGesture, bound: i32) -> i32 {
    let mut d = bound + 1;
    for i in 0..a.n {
        d = d.min(gesture_dist(b, &cand_of(&a.samples[i]), d - 1));
    }
    for j in 0..b.n {
        d = d.min(gesture_dist(a, &cand_of(&b.samples[j]), d - 1));
    }
    d
}

/// The distance separation measures between a new shape and another action's (pair_dist,
/// exact): what the tests assert.
pub fn fs_shape_distance(a: &FsGesture, b: &FsGesture) -> i32 {
    pair_dist(a, b, UNBOUNDED)
}

/// Separation (6.3a): d = the distance between the new shape and another action's shape h
/// (fs_shape_distance, every sample and every turn both ways). h is near when d < max(T_h, T_new)
/// + FS_SEPARATE_PAD, and the new shape is h's shape when also d <= T_h.
/// - No near shape: Clear.
/// - One near shape, and it is h's shape: SameAs(h), "Use moves it here". Moving it is safe only
///   then: nothing else stays near the shape once h gives it up.
/// - Otherwise TooClose, naming the nearest near shape it is not the shape of (or the nearest
///   when it is the shape of several): the samples are discarded. So a shape too close to one
///   action is never saved by being another's.
///
/// TooClose is symmetric (d and max(T_h, T_new) are), so two shapes are kept apart whichever was
/// recorded first. others holds every other action's shape (the action's own old shape, which
/// the new one replaces, left out).
pub fn fs_record_separation(new: &FsGesture, others: &[FsGesture]) -> FsSeparation {
    let bound = FS_T_MAX + FS_SEPARATE_PAD;
    let mut near = 0usize;
    // the nearest near shape (d, index, whether it is h's shape), and the nearest that is not
    let mut first: Option<(i32, usize, bool)> = None;
    let mut close: Option<(i32, usize)> = None;
    for (k, h) in others.iter().enumerate() {
        if h.n == 0 {
            continue;
        }
        let d = pair_dist(new, h, bound);
        if d >= h.thr.max(new.thr) + FS_SEPARATE_PAD {
            continue;
        }
        near += 1;
        let same = d <= h.thr;
        if first.is_none_or(|(fd, _, _)| d < fd) {
            first = Some((d, k, same));
        }
        if !same && close.is_none_or(|(cd, _)| d < cd) {
            close = Some((d, k));
        }
    }
    match (first, close) {
        (None, _) => FsSeparation::Clear,
        (Some((_, k, true)), _) if near == 1 => FsSeparation::SameAs(k),
        (_, Some((_, k))) => FsSeparation::TooClose(k),
        (Some((_, k, _)), None) => FsSeparation::TooClose(k),
    }
}

/// The full slip-bank check before Use (6.3b), at the gesture's own threshold (plus the pad).
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

/// The longest shape value: "1,3" and three samples of 128 hex digits, each after a comma.
pub const FS_VALUE_MAX: usize = 3 + FS_SAMPLES_MAX * (1 + 4 * FS_N);

/// Whether a sample is degenerate: its box under 32 units on both axes (a sample fs_sample_of
/// wrote from a real stroke always spans 0-255 on its larger side).
fn degenerate(s: &FsSample) -> bool {
    let (mut minx, mut maxx, mut miny, mut maxy) = (255u8, 0u8, 255u8, 0u8);
    for i in 0..FS_N {
        minx = minx.min(s.xy[2 * i]);
        maxx = maxx.max(s.xy[2 * i]);
        miny = miny.min(s.xy[2 * i + 1]);
        maxy = maxy.max(s.xy[2 * i + 1]);
    }
    maxx - minx < 32 && maxy - miny < 32
}

/// Writes "1,N,<128 hex>,..." (lowercase) into out and returns it: exactly what fs_decode reads
/// back. Only 2 or 3 samples that are not degenerate make a value; anything else is Malformed
/// and nothing is written, so a line this writes is never one the reader refuses. A fixed
/// buffer, so nothing is allocated.
pub fn fs_encode<'a>(samples: &[FsSample], out: &'a mut [u8; FS_VALUE_MAX]) -> Result<&'a str, FsCodec> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let n = samples.len();
    if !(2..=FS_SAMPLES_MAX).contains(&n) || samples.iter().any(degenerate) {
        return Err(FsCodec::Malformed);
    }
    out[0] = b'0' + FS_FORMAT as u8;
    out[1] = b',';
    out[2] = b'0' + n as u8;
    let mut k = 3;
    for s in samples {
        out[k] = b',';
        k += 1;
        for &b in &s.xy {
            out[k] = HEX[(b >> 4) as usize];
            out[k + 1] = HEX[(b & 15) as usize];
            k += 2;
        }
    }
    // (only ASCII digits, letters and commas were written)
    core::str::from_utf8(&out[..k]).map_err(|_| FsCodec::Malformed)
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
        if degenerate(s) {
            return Err(FsCodec::Malformed);
        }
    }
    if fields.next().is_some() {
        return Err(FsCodec::Malformed);
    }
    Ok(n)
}
