//! shape: the smart-shape recognizer (the pen's "Smart shapes", docs/index-hook, plan.smart_shapes).
//!
//! The editor hands it a raw ink stroke when the nib has been held still at the end of one; it says
//! whether the stroke is a line, a rectangle, a square, a quad, a triangle, an ellipse or a circle,
//! and gives the clean points that replace the raw ones. When it says no it says why (a
//! ShapeReason), for the shape-log the owner tunes the gates from.
//!
//! It is pure geometry. Timing features would be useless: the headless runner's batches share one
//! time, and a relayed pen sends nothing while it is still. So only the positions decide, and the
//! sample times are only carried into the result (interpolated by arc length).
//!
//! Sizes are in screen millimetres, so a shape drawn at any zoom needs the same hand movement:
//! page px = mm x dpi / 25.4 / zoom. The stored points are page geometry, so their spacing (a point
//! every 2 mm) and the ellipse's vertex count are in page units and do not depend on the zoom.
//!
//! The gates are there to say no. On the owner's 47 real strokes a hold-only trigger fired 21
//! times; here size, closure, winding, the scribble budget and the fit residuals each have to pass, and a stroke started in a writing burst (just after the last lift, close to it) gets
//! tolerances 0.7x as wide. Every refusal is an enum: the screens map them to text, this module
//! has none.
//!
//! Floats are f64 inside (the fits sum hundreds of products) and f32 at the edges, as PenSample
//! is. Every `a * b + c` is written `fmla(a, b, c)` (PORTING.md, Floating point).

use crate::prelude::*;

// ---- the gates (plan.smart_shapes, RECOGNIZER) ----

/// The stroke is resampled at its bounding box's diagonal / this.
pub const SHAPE_RESAMPLE_DIV: f32 = 40.0;
/// A line's chord / path at least this.
pub const SHAPE_LINE_STRAIGHT: f32 = 0.97;
/// A line's largest offset from its chord: max(this x its length, SHAPE_LINE_OFFSET_MM) plus
/// SHAPE_LINE_OFFSET_PAD_MM.
pub const SHAPE_LINE_OFFSET_FRAC: f32 = 0.04;
pub const SHAPE_LINE_OFFSET_MM: f32 = 1.0;
pub const SHAPE_LINE_OFFSET_PAD_MM: f32 = 0.5;
/// A line at least this long (screen mm).
pub const SHAPE_LINE_MIN_MM: f32 = 12.0;
/// A line within this many degrees of 0, 45, 90 or 135 is turned onto it.
pub const SHAPE_LINE_SNAP_DEG: f32 = 4.0;
/// On ruled paper a line within this many degrees of horizontal is made exactly horizontal: the
/// printed rules say what the writer meant, so the window is wider than SHAPE_LINE_SNAP_DEG.
pub const SHAPE_RULED_SNAP_DEG: f32 = 10.0;
/// A closed shape's end gap at most this fraction of the bounding box's diagonal.
pub const SHAPE_CLOSE_GAP: f32 = 0.22;
/// A closed shape winds around its centroid between these many pi.
pub const SHAPE_WIND_MIN: f32 = 1.7;
pub const SHAPE_WIND_MAX: f32 = 2.6;
/// The scribble budget: total absolute turning at most this x the winding plus SHAPE_TURN_PAD pi.
pub const SHAPE_TURN_FACTOR: f32 = 1.6;
pub const SHAPE_TURN_PAD: f32 = 0.6;
/// A closed shape at least this many screen mm across and thick: its least width over all
/// directions is the thickness, its width square to that the size across (a square's side both).
pub const SHAPE_CLOSED_ACROSS_MM: f32 = 10.0;
pub const SHAPE_CLOSED_THICK_MM: f32 = 4.0;
/// Each polygon edge's chord / path at least this.
pub const SHAPE_SEG_STRAIGHT: f32 = 0.93;
/// Every corner within this many degrees of 90 makes a rectangle (otherwise a quad).
pub const SHAPE_RECT_DEG: f32 = 15.0;
/// A rectangle within this many degrees of the page's axes is put on them.
pub const SHAPE_AXIS_DEG: f32 = 5.0;
/// A rectangle whose sides differ by at most this fraction of the longer is a square.
pub const SHAPE_SQUARE_SIDES: f32 = 0.10;
/// A triangle within this many degrees of equilateral or right-angled is made so.
pub const SHAPE_TRI_DEG: f32 = 7.0;
/// An ellipse's RMS and largest radial errors at most these fractions of its minor semi-axis.
pub const SHAPE_ELLIPSE_RMS: f32 = 0.06;
pub const SHAPE_ELLIPSE_MAX: f32 = 0.20;
/// An ellipse whose minor / major axis is at least this is a circle.
pub const SHAPE_CIRCLE_RATIO: f32 = 0.88;
/// An ellipse within this many degrees of the page's axes is put on them.
pub const SHAPE_ELLIPSE_AXIS_DEG: f32 = 6.0;

// ---- paper (paper.rs draws both grids at 5 page mm) ----

/// The dot grid's and graph paper's pitch (page mm), as paper.rs draws them.
pub const SHAPE_GRID_PITCH_MM: f32 = 5.0;
/// Line ends and rectangle edges move onto the grid when they are this close (screen mm).
pub const SHAPE_GRID_SNAP_MM: f32 = 1.0;

// ---- the snapped points (plan.smart_shapes, STORAGE) ----

/// Edges and lines get a point at least every this many page mm.
pub const SHAPE_POINT_MM: f32 = 2.0;
/// The vertex formula's px term (page px; see shape_ellipse_vertices), and its clamp.
pub const SHAPE_ELLIPSE_PX: f32 = 0.25;
pub const SHAPE_ELLIPSE_MIN_N: i32 = 24;
pub const SHAPE_ELLIPSE_MAX_N: i32 = 720;

// ---- the writing-burst gate (the editor fills ShapeParams::burst with shape_in_burst) ----

/// A stroke started within this many ms of the last lift...
pub const SHAPE_BURST_MS: u32 = 600;
/// ...and this many screen mm of where it lifted is part of a writing burst.
pub const SHAPE_BURST_MM: f32 = 8.0;
/// A burst stroke's tolerances are this times as wide.
pub const SHAPE_BURST_TOL: f32 = 0.7;
/// A burst stroke's hold is this times as long (the editor's: the hold timing is not here).
pub const SHAPE_BURST_HOLD: f32 = 1.5;

/// More resampled points than this (a path longer than 8 diagonals) is ink no shape has: no closed
/// shape with an overshoot comes near it, and it keeps the work bounded.
const MAX_RESAMPLED: usize = 8 * 40 + 1;
/// ShortStraw's window (Wolin, Eoff and Hammond 2008) and its corner threshold.
const STRAW_W: usize = 3;
const STRAW_MEDIAN: f64 = 0.95;
/// The share of the path at each end that the closure may cut away (an overshoot past the start, or
/// a lead-in before the first corner).
const CLOSE_TRIM: f64 = 0.15;

/// What a stroke was recognized as. The values go into the shape-log.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShapeKind {
    /// not a shape (see ShapeReason)
    #[default]
    None = 0,
    Line = 1,
    Rectangle = 2,
    Square = 3,
    /// four straight edges that are not a rectangle (a diamond, a trapezium)
    Quad = 4,
    Triangle = 5,
    Ellipse = 6,
    Circle = 7,
    /// v1.5 (ShapeParams::extras): a shaft and one or two barbs at its tip, in one stroke
    Arrow = 8,
    /// v1.5: an open chain of straight segments, its near-square turns made square
    Polyline = 9,
    /// v1.5: an open piece of a circle
    Arc = 10,
}

/// Why a stroke was not snapped, or Snapped. In the order the gates run.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShapeReason {
    #[default]
    Snapped = 0,
    /// fewer than two distinct points, a point that is not a number, or a dpi or zoom of none
    Degenerate = 1,
    /// its bounding box's diagonal is under the smallest shape (SHAPE_CLOSED_ACROSS_MM)
    TooSmall = 2,
    /// straight, but shorter than SHAPE_LINE_MIN_MM
    LineTooShort = 3,
    /// neither straight nor closed: the ends are too far apart
    NotClosed = 4,
    /// closed, but less than SHAPE_CLOSED_ACROSS_MM across
    TooNarrow = 5,
    /// closed, but less than SHAPE_CLOSED_THICK_MM thick
    TooThin = 6,
    /// it does not go around its centroid once (a spiral, twice round, a hook)
    Winding = 7,
    /// it turns far more than a shape that goes around once: a scribble or writing
    Scribble = 8,
    /// closed and round enough, but neither a polygon nor an ellipse fits it
    NoFit = 9,
}

/// The parameters the editor supplies with a stroke.
#[derive(Clone, Copy, Debug)]
pub struct ShapeParams {
    /// the page's px per inch (the page px of PenSample)
    pub dpi: f32,
    /// screen px per page px: the gates are in screen mm
    pub zoom: f32,
    /// the page's paper: dots and graph paper snap to the grid, ruled papers snap lines level
    pub paper: PaperKind,
    /// the page's width in page px: the dot grid is centred on it (paper.rs)
    pub page_w: i32,
    /// the stroke started in a writing burst (shape_in_burst): tolerances SHAPE_BURST_TOL as wide
    pub burst: bool,
    /// v1.5: also arrows, open polylines and arcs (plan step 22: off until the corpus gate passes
    /// with them on)
    pub extras: bool,
}

/// The parameters for a page drawn at `zoom`, outside a writing burst, v1.5's kinds off.
pub fn shape_params(dpi: f32, zoom: f32, paper: PaperKind, page_w: i32) -> ShapeParams {
    ShapeParams { dpi, zoom, paper, page_w, burst: false, extras: false }
}

/// What the gates measured, for the shape-log (no coordinates). Lengths are screen mm, angles are
/// in units of pi, ratios are plain. A gate that did not run leaves its field 0.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShapeFeatures {
    /// the raw stroke's bounding box diagonal
    pub diag_mm: f32,
    /// the resampled path's chord / length
    pub straight: f32,
    /// the largest distance from the chord
    pub offset_mm: f32,
    /// the closure's gap / the diagonal
    pub gap: f32,
    /// the closed path's width square to its least width, and its least width (over all directions)
    pub across_mm: f32,
    pub thick_mm: f32,
    /// around the centroid, signed (the sign is the direction the stroke went)
    pub winding: f32,
    /// total absolute turning
    pub turning: f32,
    /// ShortStraw's corners on the closed path (on the open one, ends included, for v1.5's kinds)
    pub corners: i32,
    /// the ellipse fit's RMS and largest radial errors / the minor semi-axis (an arc's / its
    /// radius; 0 when not fitted)
    pub rms: f32,
    pub max_err: f32,
    /// the fitted ellipse's minor / major axis
    pub axis_ratio: f32,
}

/// A recognition's result. `pts` is empty unless `kind` is a shape.
#[derive(Clone, Debug, Default)]
pub struct ShapeResult {
    pub kind: ShapeKind,
    pub reason: ShapeReason,
    pub features: ShapeFeatures,
    /// the snapped stroke's points: closed shapes end exactly on their first point
    pub pts: Vec<PenSample>,
}

// ---- small geometry (f64) ----

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct P {
    x: f64,
    y: f64,
}

fn p(x: f64, y: f64) -> P {
    P { x, y }
}
fn sub(a: P, b: P) -> P {
    p(a.x - b.x, a.y - b.y)
}
fn add(a: P, b: P) -> P {
    p(a.x + b.x, a.y + b.y)
}
fn scale(a: P, k: f64) -> P {
    p(a.x * k, a.y * k)
}
fn dot(a: P, b: P) -> f64 {
    fmla(a.x, b.x, a.y * b.y)
}
fn cross(a: P, b: P) -> f64 {
    fmla(a.x, b.y, -(a.y * b.x))
}
fn norm(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn dist(a: P, b: P) -> f64 {
    norm(sub(a, b))
}
/// a + d * t
fn madd(a: P, d: P, t: f64) -> P {
    p(fmla(d.x, t, a.x), fmla(d.y, t, a.y))
}
fn lerp(a: P, b: P, t: f64) -> P {
    madd(a, sub(b, a), t)
}
fn unit(angle: f64) -> P {
    p(angle.cos(), angle.sin())
}
/// The signed angle from a to b, in (-pi, pi].
fn angle_between(a: P, b: P) -> f64 {
    cross(a, b).atan2(dot(a, b))
}
/// The point of the segment a-b nearest q, as the segment's parameter in [0, 1].
fn seg_param(q: P, a: P, b: P) -> f64 {
    let d = sub(b, a);
    let l2 = dot(d, d);
    if l2 > 0.0 {
        (dot(sub(q, a), d) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    }
}
fn seg_dist(q: P, a: P, b: P) -> f64 {
    dist(q, lerp(a, b, seg_param(q, a, b)))
}
fn path_len(q: &[P]) -> f64 {
    q.windows(2).map(|w| dist(w[0], w[1])).sum()
}
/// The centroid of the points (not of the area: the loops are resampled evenly, so it is close).
fn centroid(q: &[P]) -> P {
    let mut s = P::default();
    for &a in q {
        s = add(s, a);
    }
    scale(s, 1.0 / q.len() as f64)
}
/// Twice the polygon's signed area (positive when it goes the way angles grow).
fn area2(v: &[P]) -> f64 {
    let n = v.len();
    let mut s = 0.0;
    for i in 0..n {
        s += cross(v[i], v[(i + 1) % n]);
    }
    s
}

/// The principal frame of a point set: its mean and the angle of its long axis.
fn pca(q: &[P]) -> (P, f64) {
    let c = centroid(q);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for &a in q {
        let d = sub(a, c);
        sxx = fmla(d.x, d.x, sxx);
        syy = fmla(d.y, d.y, syy);
        sxy = fmla(d.x, d.y, sxy);
    }
    (c, 0.5 * (2.0 * sxy).atan2(sxx - syy))
}

/// A least-squares line through the points: a point on it and its unit direction.
fn fit_line(q: &[P]) -> (P, P) {
    let (c, th) = pca(q);
    (c, unit(th))
}

/// Where two lines (point, direction) cross; None when they are nearly parallel.
fn intersect(a: (P, P), b: (P, P)) -> Option<P> {
    let den = cross(a.1, b.1);
    if den.abs() < 1e-6 {
        return None;
    }
    let t = cross(sub(b.0, a.0), b.1) / den;
    Some(madd(a.0, a.1, t))
}

/// The points every `step` along the path (the first and, within half a step, the last).
/// None when that is more than MAX_RESAMPLED points.
fn resample(raw: &[P], step: f64) -> Option<Vec<P>> {
    let mut out = vec![raw[0]];
    let mut prev = raw[0];
    let mut acc = 0.0;
    for &cur in &raw[1..] {
        let mut d = dist(prev, cur);
        while d > 0.0 && acc + d >= step {
            let np = lerp(prev, cur, (step - acc) / d);
            out.push(np);
            if out.len() > MAX_RESAMPLED {
                return None;
            }
            prev = np;
            d = dist(prev, cur);
            acc = 0.0;
        }
        acc += d;
        prev = cur;
    }
    // A last bit shorter than half a step would give the path a last direction made of noise:
    // the end moves onto the last point instead.
    let last = raw[raw.len() - 1];
    if acc >= 0.5 * step {
        out.push(last);
    } else if out.len() > 1 {
        let n = out.len();
        out[n - 1] = last;
    }
    if out.len() > MAX_RESAMPLED {
        return None;
    }
    Some(out)
}

// ---- the writing-burst gate ----

/// The writing-burst gate: whether a stroke whose first sample is `first` started within
/// SHAPE_BURST_MS and SHAPE_BURST_MM (screen mm) of the last lift (`lift`, the previous stroke's
/// last sample; None when there was none). The editor puts the answer in ShapeParams::burst and
/// holds SHAPE_BURST_HOLD times as long. Times are now_ms's wrapping u32 (PORTING.md, Timing).
pub fn shape_in_burst(lift: Option<&PenSample>, first: &PenSample, dpi: f32, zoom: f32) -> bool {
    let Some(l) = lift else {
        return false;
    };
    if first.t_ms.wrapping_sub(l.t_ms) > SHAPE_BURST_MS {
        return false;
    }
    let mm = px_per_mm(dpi, zoom);
    if mm <= 0.0 {
        return false;
    }
    let d = dist(p(first.x as f64, first.y as f64), p(l.x as f64, l.y as f64));
    d <= SHAPE_BURST_MM as f64 * mm
}

/// Page px per screen mm.
fn px_per_mm(dpi: f32, zoom: f32) -> f64 {
    if !dpi.is_finite() || !zoom.is_finite() || dpi <= 0.0 || zoom <= 0.0 {
        return 0.0;
    }
    dpi as f64 / 25.4 / zoom as f64
}

/// The ellipse vertex count: N = clamp(ceil(2 pi / acos(1 - 0.25 px / r)), 24, 720), where r is
/// the ellipse's major semi-axis (a circle's radius) in page px and px is one page px, so the
/// count follows the shape's size on the page, not the zoom it was drawn at. With x = acos(1 - s
/// / r) the step, r(1 - cos x) is s, and the chord's sagitta r(1 - cos(x / 2)) about a quarter of
/// that: a sixteenth of a pixel, so the polygon cannot be told from the curve.
pub fn shape_ellipse_vertices(r_px: f32) -> i32 {
    let r = r_px as f64;
    if r.is_nan() || r <= 0.0 || r.is_infinite() {
        return if r.is_infinite() { SHAPE_ELLIPSE_MAX_N } else { SHAPE_ELLIPSE_MIN_N };
    }
    let a = (1.0 - SHAPE_ELLIPSE_PX as f64 / r).clamp(-1.0, 1.0).acos();
    if a <= 0.0 {
        return SHAPE_ELLIPSE_MAX_N;
    }
    let n = (2.0 * std::f64::consts::PI / a).ceil();
    if n >= SHAPE_ELLIPSE_MAX_N as f64 {
        SHAPE_ELLIPSE_MAX_N
    } else if n <= SHAPE_ELLIPSE_MIN_N as f64 {
        SHAPE_ELLIPSE_MIN_N
    } else {
        n as i32
    }
}

/// Which papers have printed rules a line should sit level with: the three rulings with and
/// without the margin, and Cornell (medium rule). Music and tab staves are not: their lines are
/// for notes, and a slanted beam or slur drawn on them is meant slanted.
pub fn shape_paper_ruled(k: PaperKind) -> bool {
    matches!(k, PAPER_NARROW | PAPER_MEDIUM | PAPER_WIDE | PAPER_NARROW_NM | PAPER_MEDIUM_NM | PAPER_WIDE_NM | PAPER_CORNELL)
}

// ---- the recognizer ----

/// Recognizes a raw stroke (`pts`, page px) on a page drawn with `prm`. Never fails: a stroke that
/// is no shape comes back with ShapeKind::None and the reason.
pub fn shape_recognize(pts: &[PenSample], prm: &ShapeParams) -> ShapeResult {
    let mut r = ShapeResult::default();
    let mm = px_per_mm(prm.dpi, prm.zoom);
    let tol = if prm.burst { SHAPE_BURST_TOL as f64 } else { 1.0 };
    let mut raw: Vec<P> = Vec::with_capacity(pts.len());
    for s in pts {
        if !s.x.is_finite() || !s.y.is_finite() {
            r.reason = ShapeReason::Degenerate;
            return r;
        }
        let q = p(s.x as f64, s.y as f64);
        // (repeated points add nothing to the path)
        if raw.last() != Some(&q) {
            raw.push(q);
        }
    }
    if mm <= 0.0 || raw.len() < 2 {
        r.reason = ShapeReason::Degenerate;
        return r;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (raw[0].x, raw[0].y, raw[0].x, raw[0].y);
    for a in &raw {
        x0 = x0.min(a.x);
        y0 = y0.min(a.y);
        x1 = x1.max(a.x);
        y1 = y1.max(a.y);
    }
    let diag = norm(p(x1 - x0, y1 - y0));
    r.features.diag_mm = (diag / mm) as f32;
    // Nothing smaller than the smallest closed shape (a 12 mm line has a 12 mm diagonal).
    if diag < SHAPE_CLOSED_ACROSS_MM as f64 * mm {
        r.reason = ShapeReason::TooSmall;
        return r;
    }
    let step = diag / SHAPE_RESAMPLE_DIV as f64;
    let Some(q) = resample(&raw, step) else {
        r.reason = ShapeReason::Scribble;
        return r;
    };
    let len = path_len(&q);

    // LINE: straight, close to its chord, long enough.
    let (a, b) = (q[0], q[q.len() - 1]);
    let chord = dist(a, b);
    let straight = if len > 0.0 { chord / len } else { 0.0 };
    let mut offset = 0.0f64;
    for &c in &q {
        offset = offset.max(seg_dist(c, a, b));
    }
    r.features.straight = straight as f32;
    r.features.offset_mm = (offset / mm) as f32;
    let allow = fmla(SHAPE_LINE_OFFSET_PAD_MM as f64, mm, (SHAPE_LINE_OFFSET_FRAC as f64 * chord).max(SHAPE_LINE_OFFSET_MM as f64 * mm)) * tol;
    // (a tolerance is the distance from perfect: 1 - 0.97 for the straightness)
    if straight >= fmla(-(1.0 - SHAPE_LINE_STRAIGHT as f64), tol, 1.0) && offset <= allow {
        if chord < SHAPE_LINE_MIN_MM as f64 * mm {
            r.reason = ShapeReason::LineTooShort;
            return r;
        }
        let ends = line_ends(&q, &raw, prm, mm);
        r.kind = ShapeKind::Line;
        r.pts = finish(pts, &segments(&[ends.0, ends.1], false, page_step(prm)));
        return r;
    }

    // CLOSED: find where the loop closes, then the size, winding and scribble gates.
    let (i, j, gap) = closure(&q, len);
    r.features.gap = (gap / diag) as f32;
    if gap > SHAPE_CLOSE_GAP as f64 * diag * tol {
        if prm.extras {
            if let Some((kind, out)) = open_shape(&q, diag, prm, mm, tol, &mut r.features) {
                r.kind = kind;
                r.pts = finish(pts, &out);
                return r;
            }
        }
        r.reason = ShapeReason::NotClosed;
        return r;
    }
    let lp = &q[i..=j];
    let (thick, across) = widths(lp);
    r.features.across_mm = (across / mm) as f32;
    r.features.thick_mm = (thick / mm) as f32;
    if across < SHAPE_CLOSED_ACROSS_MM as f64 * mm {
        r.reason = ShapeReason::TooNarrow;
        return r;
    }
    if thick < SHAPE_CLOSED_THICK_MM as f64 * mm {
        r.reason = ShapeReason::TooThin;
        return r;
    }
    let pi = std::f64::consts::PI;
    let cen = centroid(lp);
    let mut wind = 0.0;
    for w in lp.windows(2) {
        let (da, db) = (sub(w[0], cen), sub(w[1], cen));
        if norm(da) > 0.0 && norm(db) > 0.0 {
            wind += angle_between(da, db);
        }
    }
    r.features.winding = (wind / pi) as f32;
    let aw = wind.abs() / pi;
    // The window is narrowed around one full turn (2 pi), so a burst keeps it centred there.
    let wlo = fmla(-(2.0 - SHAPE_WIND_MIN as f64), tol, 2.0);
    let whi = fmla(SHAPE_WIND_MAX as f64 - 2.0, tol, 2.0);
    if aw < wlo || aw > whi {
        r.reason = ShapeReason::Winding;
        return r;
    }
    // The turning is summed over the loop smoothed twice with [1 2 1] / 4. That keeps every real
    // turn whole (a corner, or a scribble's reversal, turns as far as before, over a step or two
    // more) and drops the pen's jitter, which at a 40th of a small shape's diagonal would add
    // about a turn of its own.
    let sm = smooth121(&smooth121(lp));
    let mut turn = 0.0;
    for w in sm.windows(3) {
        turn += angle_between(sub(w[1], w[0]), sub(w[2], w[1])).abs();
    }
    r.features.turning = (turn / pi) as f32;
    // The budget's slack over the winding (0.6 x the winding + 0.6 pi) is what a burst narrows.
    let slack = fmla(SHAPE_TURN_FACTOR as f64 - 1.0, aw, SHAPE_TURN_PAD as f64) * tol;
    if turn / pi > aw + slack {
        r.reason = ShapeReason::Scribble;
        return r;
    }

    // The loop, closed: its points then the gap's, so every corner has neighbours on both sides.
    let mut cyc: Vec<P> = lp.to_vec();
    let close = dist(lp[lp.len() - 1], lp[0]);
    let nfill = (close / step).floor() as usize;
    for k in 1..nfill {
        cyc.push(lerp(lp[lp.len() - 1], lp[0], k as f64 / nfill as f64));
    }
    let sign = if wind >= 0.0 { 1.0 } else { -1.0 };
    let seg_min = fmla(-(1.0 - SHAPE_SEG_STRAIGHT as f64), tol, 1.0);
    let poly = polygon(&cyc, seg_min, diag, &mut r.features);
    let ell = ellipse(lp, diag, tol, &mut r.features);
    let first = raw[0];
    let pstep = page_step(prm);
    // Both can pass (a long ellipse's quarters are nearly straight): the closer outline wins.
    let use_poly = match (&poly, &ell) {
        (Some(pg), Some(el)) => outline_rms(lp, &pg.1) <= el.rms_px,
        (Some(_), None) => true,
        _ => false,
    };
    if let (true, Some((kind, verts))) = (use_poly, poly) {
        let (kind, verts) = snap_polygon(kind, verts, prm, mm);
        r.kind = kind;
        r.pts = finish(pts, &start_closed(&verts, sign, first, pstep));
        return r;
    }
    if let Some(el) = ell {
        let (kind, outline) = ellipse_outline(&el, sign, first);
        r.kind = kind;
        r.pts = finish(pts, &outline);
        return r;
    }
    r.reason = ShapeReason::NoFit;
    r
}

/// The loop's least width over all directions (every 2 degrees) and its width across that: a
/// square's side twice, however it is turned (a principal frame is arbitrary for a square and
/// would measure it along its diagonal).
fn widths(q: &[P]) -> (f64, f64) {
    let extent = |d: P| -> f64 {
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for &a in q {
            let t = dot(a, d);
            lo = lo.min(t);
            hi = hi.max(t);
        }
        hi - lo
    };
    let mut best = (f64::MAX, 0.0);
    for k in 0..90 {
        let th = (k as f64 * 2.0).to_radians();
        let w = extent(unit(th));
        if w < best.0 {
            best = (w, th);
        }
    }
    (best.0, extent(unit(best.1 + std::f64::consts::FRAC_PI_2)))
}

/// The path filtered with [1 2 1] / 4, its ends kept.
fn smooth121(q: &[P]) -> Vec<P> {
    let n = q.len();
    if n < 3 {
        return q.to_vec();
    }
    let mut out = Vec::with_capacity(n);
    out.push(q[0]);
    for k in 1..n - 1 {
        let s = add(add(q[k - 1], q[k + 1]), add(q[k], q[k]));
        out.push(scale(s, 0.25));
    }
    out.push(q[n - 1]);
    out
}

/// Page px between the snapped points (SHAPE_POINT_MM page mm).
fn page_step(prm: &ShapeParams) -> f64 {
    SHAPE_POINT_MM as f64 * prm.dpi as f64 / 25.4
}

/// Where the closed path closes: the nearest pair of points, one in the first CLOSE_TRIM of the
/// path and one in the last. An overshoot past the start is cut there, a gap is measured there.
fn closure(q: &[P], len: f64) -> (usize, usize, f64) {
    let n = q.len();
    let mut cum = vec![0.0; n];
    for k in 1..n {
        cum[k] = cum[k - 1] + dist(q[k - 1], q[k]);
    }
    let head = CLOSE_TRIM * len;
    let tail = (1.0 - CLOSE_TRIM) * len;
    let mut best = (0, n - 1, dist(q[0], q[n - 1]));
    for i in 0..n {
        if cum[i] > head {
            break;
        }
        for j in (i + 1..n).rev() {
            if cum[j] < tail {
                break;
            }
            let d = dist(q[i], q[j]);
            if d < best.2 {
                best = (i, j, d);
            }
        }
    }
    best
}

// ---- lines ----

/// A line's ends: the raw ends projected onto the least-squares line, turned onto 0, 45, 90 or 135
/// degrees (level on ruled paper within the wider window), then onto the grid on grid papers.
fn line_ends(q: &[P], raw: &[P], prm: &ShapeParams, mm: f64) -> (P, P) {
    let (c, d) = fit_line(q);
    let a = madd(c, d, dot(sub(raw[0], c), d));
    let b = madd(c, d, dot(sub(raw[raw.len() - 1], c), d));
    let dir = sub(b, a);
    let l = norm(dir);
    let ang = dir.y.atan2(dir.x);
    let (t, square) = snap_angle(ang, prm);
    let (mut a, mut b) = (a, b);
    if t != ang {
        let m = lerp(a, b, 0.5);
        let h = scale(unit(t), 0.5 * l);
        a = sub(m, h);
        b = add(m, h);
    }
    if let Some(g) = Grid::of(prm) {
        let tolp = SHAPE_GRID_SNAP_MM as f64 * mm;
        if square {
            // A line the snap made level, upright or diagonal must stay so on the grid: one end
            // put on its node by itself would tilt it against the printed lines.
            (a, b) = g.line(a, b, tolp);
        } else {
            // (a slanted line has no direction to keep: each end goes to its own node)
            a = g.node(a, tolp);
            b = g.node(b, tolp);
        }
    }
    (a, b)
}

/// A straight stroke's direction, turned onto 0, 45, 90 or 135 degrees within SHAPE_LINE_SNAP_DEG,
/// or level within SHAPE_RULED_SNAP_DEG on ruled paper; otherwise as it is. The flag says the
/// direction is one of the four now (the grid snap keeps it).
fn snap_angle(ang: f64, prm: &ShapeParams) -> (f64, bool) {
    let q45 = std::f64::consts::FRAC_PI_4;
    let target = (ang / q45).round() * q45;
    let level = (ang / std::f64::consts::PI).round() * std::f64::consts::PI;
    if shape_paper_ruled(prm.paper) && (ang - level).abs().to_degrees() <= SHAPE_RULED_SNAP_DEG as f64 {
        (level, true)
    } else if (ang - target).abs().to_degrees() <= SHAPE_LINE_SNAP_DEG as f64 {
        (target, true)
    } else {
        (ang, false)
    }
}

// ---- the grid ----

/// The printed grid's lines (graph paper) or dot rows and columns (dot grid), in page px, exactly
/// as paper.rs puts them.
struct Grid {
    pitch: f64,
    ox: f64,
    oy: f64,
    /// graph paper's lines are at whole px (lrintf(pitch * i)); the dots are where they fall
    round: bool,
}

impl Grid {
    fn of(prm: &ShapeParams) -> Option<Grid> {
        let pitch = mm_to_px(SHAPE_GRID_PITCH_MM, prm.dpi);
        if pitch.is_nan() || pitch <= 0.0 {
            return None;
        }
        if prm.paper == PAPER_GRID {
            return Some(Grid { pitch: pitch as f64, ox: 0.0, oy: 0.0, round: true });
        }
        if prm.paper == PAPER_DOTS && prm.page_w > 0 {
            // paper.rs dots(): the columns centred on the page, the first row 10 mm down
            let ox = (prm.page_w as f32 % pitch) / 2.0 + pitch / 2.0;
            let oy = mm_to_px(10.0, prm.dpi);
            return Some(Grid { pitch: pitch as f64, ox: ox as f64, oy: oy as f64, round: false });
        }
        None
    }
    /// The grid coordinate nearest x along one axis (origin o). Paper draws no line or dot before
    /// its first, so neither does this.
    fn coord(&self, x: f64, o: f64) -> f64 {
        let i = ((x - o) / self.pitch).round().max(0.0);
        let g = fmla(i, self.pitch, o);
        if self.round {
            g.round_ties_even()
        } else {
            g
        }
    }
    /// The grid node nearest a.
    fn nearest(&self, a: P) -> P {
        p(self.coord(a.x, self.ox), self.coord(a.y, self.oy))
    }
    /// a on the nearest grid node when it is within tol of it.
    fn node(&self, a: P, tol: f64) -> P {
        let g = self.nearest(a);
        if dist(g, a) <= tol {
            g
        } else {
            a
        }
    }
    /// A line from a to b at 0, 45, 90 or 135 degrees, onto the grid without turning it. The end
    /// nearer a node (within tol) carries the whole line onto that node; then the other end slides
    /// along the line, and only along it, onto a node there within tol. On graph paper a node on a
    /// diagonal can be a px off the exact 45 degrees (its lines are at whole px): the end then
    /// stops level with that node on the line, so the angle stays exact.
    fn line(&self, a: P, b: P, tol: f64) -> (P, P) {
        let (na, nb) = (self.nearest(a), self.nearest(b));
        let (da, db) = (dist(na, a), dist(nb, b));
        if da > tol && db > tol {
            return (a, b);
        }
        let (anchor, node, other) = if da <= db { (a, na, b) } else { (b, nb, a) };
        let o = add(other, sub(node, anchor));
        let along = sub(o, node);
        let l = norm(along);
        if l > 0.0 {
            let u = scale(along, 1.0 / l);
            let m = self.nearest(o);
            let t = dot(sub(m, node), u);
            let off = cross(u, sub(m, node)).abs();
            if dist(m, o) <= tol && off <= 1.0 && t > 0.0 {
                let o2 = madd(node, u, t);
                return if da <= db { (node, o2) } else { (o2, node) };
            }
        }
        if da <= db {
            (node, o)
        } else {
            (o, node)
        }
    }
}

// ---- polygons ----

/// The points from index a to b along the path (forward, around the end when it is closed: a
/// closed path's segment from a corner back to itself is the whole loop).
fn span(n: usize, cyclic: bool, a: usize, b: usize) -> usize {
    if b > a || !cyclic {
        b.saturating_sub(a)
    } else {
        b + n - a
    }
}

/// A segment's chord / path.
fn seg_straight(c: &[P], cyclic: bool, a: usize, b: usize) -> f64 {
    let n = c.len();
    let k = span(n, cyclic, a, b);
    let mut path = 0.0;
    for o in 0..k {
        path += dist(c[(a + o) % n], c[(a + o + 1) % n]);
    }
    if path > 0.0 {
        dist(c[a], c[b % n]) / path
    } else {
        0.0
    }
}

/// ShortStraw's corners (Wolin, Eoff and Hammond 2008) on the resampled path, closed or open (an
/// open path's ends are corners). Its two passes test segments with the edge test, so they agree
/// with it: a segment that is not straight gets a corner at its smallest straw in its middle
/// half; a corner whose two segments make one straight one goes (the straightest such first).
fn straw_corners(c: &[P], cyclic: bool, seg_min: f64) -> Vec<usize> {
    let m = c.len();
    if m < 2 * STRAW_W + 3 {
        return Vec::new();
    }
    // An open path has no straw within the window of its ends: the median stands in.
    let raw_straw = |i: usize| -> Option<f64> {
        if cyclic {
            Some(dist(c[(i + m - STRAW_W) % m], c[(i + STRAW_W) % m]))
        } else if i >= STRAW_W && i + STRAW_W < m {
            Some(dist(c[i - STRAW_W], c[i + STRAW_W]))
        } else {
            None
        }
    };
    let known: Vec<f64> = (0..m).filter_map(raw_straw).collect();
    let mut sorted = known.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let med = sorted[sorted.len() / 2];
    let straw: Vec<f64> = (0..m).map(|i| raw_straw(i).unwrap_or(med)).collect();
    let thr = med * STRAW_MEDIAN;
    // One corner per run of straws under the threshold: its smallest. A closed path's scan starts
    // at its largest straw, so no run is cut in two by the wrap.
    let start = if cyclic { (0..m).max_by(|&a, &b| straw[a].total_cmp(&straw[b])).unwrap_or(0) } else { 0 };
    let mut corners: Vec<usize> = Vec::new();
    let mut run: Option<usize> = None;
    for k in 0..=m {
        let i = (start + k) % m;
        if k < m && straw[i] < thr {
            run = Some(match run {
                Some(b) if straw[b] <= straw[i] => b,
                _ => i,
            });
        } else if let Some(b) = run.take() {
            corners.push(b);
        }
    }
    if !cyclic {
        corners.push(0);
        corners.push(m - 1);
    }
    corners.sort_unstable();
    corners.dedup();
    // (an open path's segments run between neighbours, without the wrap)
    let pairs = |nc: usize| if cyclic { nc } else { nc - 1 };
    for _ in 0..12 {
        if corners.is_empty() || corners.len() > 8 {
            break;
        }
        let mut added = None;
        let nc = corners.len();
        for k in 0..pairs(nc) {
            let (a, b) = (corners[k], corners[(k + 1) % nc]);
            let n = span(m, cyclic, a, b);
            if seg_straight(c, cyclic, a, b) >= seg_min || n < 4 {
                continue;
            }
            let mut best = (a + n / 4) % m;
            for o in n / 4..=(3 * n) / 4 {
                let i = (a + o) % m;
                if straw[i] < straw[best] {
                    best = i;
                }
            }
            added = Some(best);
            break;
        }
        match added {
            Some(b) if !corners.contains(&b) => {
                corners.push(b);
                corners.sort_unstable();
            }
            _ => break,
        }
    }
    loop {
        let nc = corners.len();
        if nc <= if cyclic { 3 } else { 2 } {
            break;
        }
        let mut best: Option<(usize, f64)> = None;
        let ks = if cyclic { 0..nc } else { 1..nc - 1 };
        for k in ks {
            let s = seg_straight(c, cyclic, corners[(k + nc - 1) % nc], corners[(k + 1) % nc]);
            if s >= seg_min && best.is_none_or(|b| s > b.1) {
                best = Some((k, s));
            }
        }
        match best {
            Some((k, _)) => {
                corners.remove(k);
            }
            None => break,
        }
    }
    corners
}

/// A segment's least-squares line, from its middle (the corners' rounding left out).
fn seg_line(c: &[P], cyclic: bool, a: usize, b: usize) -> Option<(P, P)> {
    let m = c.len();
    let n = span(m, cyclic, a, b);
    let cut = n / 5;
    let pts: Vec<P> = (cut..=n - cut).map(|o| c[(a + o) % m]).collect();
    if pts.len() < 2 {
        return None;
    }
    Some(fit_line(&pts))
}

/// ShortStraw on the closed loop, then the edges' lines: Some((Triangle or Quad, the vertices in
/// the loop's order)) when it has 3 or 4 straight edges. (Rectangles and squares are told apart
/// from quads afterwards, in snap_polygon.)
fn polygon(cyc: &[P], seg_min: f64, diag: f64, f: &mut ShapeFeatures) -> Option<(ShapeKind, Vec<P>)> {
    let corners = straw_corners(cyc, true, seg_min);
    f.corners = corners.len() as i32;
    let nc = corners.len();
    if nc != 3 && nc != 4 {
        return None;
    }
    let mut lines = Vec::with_capacity(nc);
    for k in 0..nc {
        let (a, b) = (corners[k], corners[(k + 1) % nc]);
        if seg_straight(cyc, true, a, b) < seg_min {
            return None;
        }
        lines.push(seg_line(cyc, true, a, b)?);
    }
    let mut verts = Vec::with_capacity(nc);
    for k in 0..nc {
        let v = intersect(lines[(k + nc - 1) % nc], lines[k])?;
        // Nearly parallel neighbours cross far away: that is no corner of this stroke.
        if dist(v, cyc[corners[k]]) > 0.25 * diag {
            return None;
        }
        verts.push(v);
    }
    Some((if nc == 3 { ShapeKind::Triangle } else { ShapeKind::Quad }, verts))
}

/// The RMS distance of the loop's points from a closed polygon.
fn outline_rms(lp: &[P], v: &[P]) -> f64 {
    let n = v.len();
    let mut s = 0.0;
    for &q in lp {
        let mut d = f64::MAX;
        for k in 0..n {
            d = d.min(seg_dist(q, v[k], v[(k + 1) % n]));
        }
        s = fmla(d, d, s);
    }
    (s / lp.len() as f64).sqrt()
}

/// The interior angle at each vertex (radians, in [0, pi]).
fn corner_angles(v: &[P]) -> Vec<f64> {
    let n = v.len();
    (0..n)
        .map(|k| {
            let a = sub(v[(k + n - 1) % n], v[k]);
            let b = sub(v[(k + 1) % n], v[k]);
            angle_between(a, b).abs()
        })
        .collect()
}

/// The mean of angles taken modulo 2 pi / k (the doubled-angle trick: an edge and its opposite,
/// or a triangle's three vertices, vote for one direction).
fn mean_mod(angles: &[f64], k: f64) -> f64 {
    let (mut sx, mut sy) = (0.0, 0.0);
    for &a in angles {
        sx += (a * k).cos();
        sy += (a * k).sin();
    }
    sy.atan2(sx) / k
}

/// Makes the polygon what it is closest to: a rectangle or square (on the page's axes when near
/// them, then on the grid), an equilateral or right triangle, or the quad or triangle as found.
fn snap_polygon(kind: ShapeKind, v: Vec<P>, prm: &ShapeParams, mm: f64) -> (ShapeKind, Vec<P>) {
    let ang = corner_angles(&v);
    let deg = |a: f64| a.to_degrees();
    if kind == ShapeKind::Quad {
        if !ang.iter().all(|&a| (deg(a) - 90.0).abs() <= SHAPE_RECT_DEG as f64) {
            return (kind, v);
        }
        return rectangle(&v, prm, mm);
    }
    let tdeg = SHAPE_TRI_DEG as f64;
    if ang.iter().all(|&a| (deg(a) - 60.0).abs() <= tdeg) {
        // Equilateral: the vertices' mean distance from their centroid, turned as they vote.
        let c = centroid(&v);
        let rad = v.iter().map(|&a| dist(a, c)).sum::<f64>() / 3.0;
        let dirs: Vec<f64> = v.iter().map(|&a| (a.y - c.y).atan2(a.x - c.x)).collect();
        let base = mean_mod(&dirs, 3.0);
        let third = 2.0 * std::f64::consts::PI / 3.0;
        // (start_closed turns it the stroke's way and picks the start)
        let out: Vec<P> = (0..3).map(|k| madd(c, unit(fmla(k as f64, third, base)), rad)).collect();
        return (ShapeKind::Triangle, out);
    }
    if let Some(k) = (0..3).find(|&k| (deg(ang[k]) - 90.0).abs() <= tdeg) {
        // Right-angled at k: its legs keep their lengths and are turned evenly about their
        // bisector to meet square.
        let (a, b, c) = (v[k], v[(k + 1) % 3], v[(k + 2) % 3]);
        let (db, dc) = (sub(b, a), sub(c, a));
        let (lb, lc) = (norm(db), norm(dc));
        let bis = add(scale(db, 1.0 / lb), scale(dc, 1.0 / lc));
        let t = bis.y.atan2(bis.x);
        let half = std::f64::consts::FRAC_PI_4;
        // which leg is on which side of the bisector
        let s = if cross(bis, db) < 0.0 { -1.0 } else { 1.0 };
        let nb = madd(a, unit(fmla(s, half, t)), lb);
        let nc = madd(a, unit(fmla(-s, half, t)), lc);
        let mut out = vec![a, nb, nc];
        out.rotate_right(k);
        return (ShapeKind::Triangle, out);
    }
    (kind, v)
}

/// A quad whose corners are all near square, as a rectangle: its edges' mean direction (on the
/// page's axes within SHAPE_AXIS_DEG), each pair of opposite edges at its mean offset, a square
/// when the sides are within SHAPE_SQUARE_SIDES, and on the grid when it is level on grid paper.
fn rectangle(v: &[P], prm: &ShapeParams, mm: f64) -> (ShapeKind, Vec<P>) {
    let dirs: Vec<f64> = (0..4).map(|k| (v[(k + 1) % 4].y - v[k].y).atan2(v[(k + 1) % 4].x - v[k].x)).collect();
    let mut th = mean_mod(&dirs, 4.0);
    if th.to_degrees().abs() <= SHAPE_AXIS_DEG as f64 {
        th = 0.0;
    }
    let (u, w) = (unit(th), unit(th + std::f64::consts::FRAC_PI_2));
    // The edges alternate between running along u and along w; each sits at its two ends' mean.
    let (mut us, mut ws) = (Vec::new(), Vec::new());
    for k in 0..4 {
        let (a, b) = (v[k], v[(k + 1) % 4]);
        let d = sub(b, a);
        let m = lerp(a, b, 0.5);
        if dot(d, u).abs() >= dot(d, w).abs() {
            ws.push(dot(m, w));
        } else {
            us.push(dot(m, u));
        }
    }
    if us.len() != 2 || ws.len() != 2 {
        return (ShapeKind::Quad, v.to_vec());
    }
    let (mut ua, mut ub) = (us[0].min(us[1]), us[0].max(us[1]));
    let (mut wa, mut wb) = (ws[0].min(ws[1]), ws[0].max(ws[1]));
    let (lu, lw) = (ub - ua, wb - wa);
    let square = (lu - lw).abs() <= SHAPE_SQUARE_SIDES as f64 * lu.max(lw);
    if square {
        let s = 0.5 * (lu + lw);
        let (cu, cw) = (0.5 * (ua + ub), 0.5 * (wa + wb));
        ua = fmla(-0.5, s, cu);
        ub = fmla(0.5, s, cu);
        wa = fmla(-0.5, s, cw);
        wb = fmla(0.5, s, cw);
    }
    // On the grid: only a level rectangle (then u and w are the page's x and y), corner by corner.
    // A corner within SHAPE_GRID_SNAP_MM of a node goes onto it, and with it the two edges that
    // meet there; a corner farther from every node moves only as its edges do. (Edge by edge, an
    // edge near a grid line would move although no corner is near a node, and a corner a little
    // off in both directions would land on a node from farther than the window.)
    if th == 0.0 {
        if let Some(g) = Grid::of(prm) {
            let tolp = SHAPE_GRID_SNAP_MM as f64 * mm;
            let near = |x: f64, y: f64| -> Option<P> {
                let c = p(x, y);
                let n = g.nearest(c);
                if dist(n, c) <= tolp {
                    Some(n)
                } else {
                    None
                }
            };
            if square {
                // A square keeps its sides equal: it moves whole onto the node its nearest corner
                // is near; then its side becomes the one that also puts the corner beside that one
                // (along its row or its column) on a node, when that corner is near one.
                let cs = [(ua, wa, 1.0, 1.0), (ub, wa, -1.0, 1.0), (ub, wb, -1.0, -1.0), (ua, wb, 1.0, -1.0)];
                let mut best: Option<(f64, usize, P)> = None;
                for (k, &(x, y, _, _)) in cs.iter().enumerate() {
                    if let Some(n) = near(x, y) {
                        let d = dist(n, p(x, y));
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, k, n));
                        }
                    }
                }
                if let Some((_, k, n)) = best {
                    let (sx, sy) = (cs[k].2, cs[k].3);
                    let mut s = ub - ua;
                    let bx = near(fmla(sx, s, n.x), n.y).map(|m| ((m.x - n.x).abs(), dist(m, p(fmla(sx, s, n.x), n.y))));
                    let by = near(n.x, fmla(sy, s, n.y)).map(|m| ((m.y - n.y).abs(), dist(m, p(n.x, fmla(sy, s, n.y)))));
                    let side = match (bx, by) {
                        (Some(a), Some(b)) => Some(if a.1 <= b.1 { a.0 } else { b.0 }),
                        (a, b) => a.or(b).map(|v| v.0),
                    };
                    if let Some(v) = side.filter(|&v| v > 0.0) {
                        s = v;
                    }
                    (ua, ub) = if sx > 0.0 { (n.x, n.x + s) } else { (n.x - s, n.x) };
                    (wa, wb) = if sy > 0.0 { (n.y, n.y + s) } else { (n.y - s, n.y) };
                }
            } else {
                // (when both corners of an edge are near nodes they put it on the same grid line:
                // they share the edge's coordinate, and the nearest grid line is the coordinate's)
                let (mut nu, mut nw) = ([ua, ub], [wa, wb]);
                for (iu, iw) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
                    if let Some(n) = near([ua, ub][iu], [wa, wb][iw]) {
                        nu[iu] = n.x;
                        nw[iw] = n.y;
                    }
                }
                [ua, ub] = nu;
                [wa, wb] = nw;
            }
        }
    }
    let at = |a: f64, b: f64| madd(scale(u, a), w, b);
    let out = vec![at(ua, wa), at(ub, wa), at(ub, wb), at(ua, wb)];
    (if square { ShapeKind::Square } else { ShapeKind::Rectangle }, out)
}

/// A closed polygon's outline as the snapped points: turned to go the stroke's way (`sign`, the
/// winding's), starting at the outline's point nearest the stroke's first point (its nearest vertex
/// when that is within a point step), with a point at least every `step` and the last point exactly
/// the first.
fn start_closed(v: &[P], sign: f64, first: P, step: f64) -> Vec<P> {
    let mut v = v.to_vec();
    if area2(&v) * sign < 0.0 {
        v.reverse();
    }
    let n = v.len();
    let (mut be, mut bt, mut bd) = (0, 0.0, f64::MAX);
    for k in 0..n {
        let t = seg_param(first, v[k], v[(k + 1) % n]);
        let d = dist(first, lerp(v[k], v[(k + 1) % n], t));
        if d < bd {
            (be, bt, bd) = (k, t, d);
        }
    }
    let mut start = lerp(v[be], v[(be + 1) % n], bt);
    let mut chain: Vec<P> = Vec::with_capacity(n + 2);
    if dist(start, v[be]) <= step {
        start = v[be];
        bt = 0.0;
    } else if dist(start, v[(be + 1) % n]) <= step {
        be = (be + 1) % n;
        start = v[be];
        bt = 0.0;
    }
    chain.push(start);
    for k in 1..=n {
        chain.push(v[(be + k) % n]);
    }
    if bt > 0.0 {
        // the start is inside edge be: the walk ends there, after vertex be
        chain.push(start);
    }
    // (chain ends on v[be], the start, when the start is a vertex)
    segments(&chain, true, step)
}

/// The points along a chain of vertices, each piece cut into equal parts no longer than `step`.
/// `closed`: the chain's last point is its first, and is written as exactly that.
fn segments(chain: &[P], closed: bool, step: f64) -> Vec<P> {
    let mut out = vec![chain[0]];
    for w in chain.windows(2) {
        let l = dist(w[0], w[1]);
        if l <= 0.0 {
            continue;
        }
        let k = ((l / step).ceil() as usize).max(1);
        for s in 1..=k {
            out.push(if s == k { w[1] } else { lerp(w[0], w[1], s as f64 / k as f64) });
        }
    }
    if closed {
        let n = out.len();
        out[n - 1] = out[0];
    }
    out
}

// ---- ellipses ----

struct Ellipse {
    c: P,
    /// the major axis's angle
    th: f64,
    a: f64,
    b: f64,
    /// the fit's RMS distance from the points (px), to compare with a polygon's
    rms_px: f64,
}

/// Solves the N x N system m x = r (Gaussian elimination with partial pivoting).
fn solve<const N: usize>(mut m: [[f64; N]; N], mut r: [f64; N]) -> Option<[f64; N]> {
    for col in 0..N {
        let piv = (col..N).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))?;
        if m[piv][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, piv);
        r.swap(col, piv);
        for row in col + 1..N {
            let f = m[row][col] / m[col][col];
            for k in col..N {
                m[row][k] = fmla(-f, m[col][k], m[row][k]);
            }
            r[row] = fmla(-f, r[col], r[row]);
        }
    }
    let mut x = [0.0; N];
    for row in (0..N).rev() {
        let mut s = r[row];
        for k in row + 1..N {
            s = fmla(-m[row][k], x[k], s);
        }
        x[row] = s / m[row][row];
    }
    Some(x)
}

/// The ellipse fit: in the loop's principal frame (so the ellipse has no cross term), the least-
/// squares A s^2 + B t^2 + C s + D t = 1, then the radial errors. Some when both errors pass.
fn ellipse(lp: &[P], diag: f64, tol: f64, f: &mut ShapeFeatures) -> Option<Ellipse> {
    let (c0, th0) = pca(lp);
    let (u, v) = (unit(th0), unit(th0 + std::f64::consts::FRAC_PI_2));
    // (scaled to about 1, so the normal equations stay well conditioned)
    let k = 2.0 / diag;
    let mut m = [[0.0; 4]; 4];
    let mut rhs = [0.0; 4];
    for &q in lp {
        let d = sub(q, c0);
        let (s, t) = (dot(d, u) * k, dot(d, v) * k);
        let phi = [s * s, t * t, s, t];
        for i in 0..4 {
            for j in 0..4 {
                m[i][j] = fmla(phi[i], phi[j], m[i][j]);
            }
            rhs[i] += phi[i];
        }
    }
    let [aa, bb, cc, dd] = solve(m, rhs)?;
    if aa.is_nan() || bb.is_nan() || aa <= 0.0 || bb <= 0.0 {
        return None;
    }
    let s0 = -cc / (2.0 * aa);
    let t0 = -dd / (2.0 * bb);
    let ff = 1.0 + cc * cc / (4.0 * aa) + dd * dd / (4.0 * bb);
    if ff.is_nan() || ff <= 0.0 {
        return None;
    }
    let (mut ea, mut eb) = ((ff / aa).sqrt() / k, (ff / bb).sqrt() / k);
    let c = madd(madd(c0, u, s0 / k), v, t0 / k);
    let mut th = th0;
    if eb > ea {
        std::mem::swap(&mut ea, &mut eb);
        th += std::f64::consts::FRAC_PI_2;
    }
    let (u, v) = (unit(th), unit(th + std::f64::consts::FRAC_PI_2));
    let (mut s2, mut emax) = (0.0, 0.0f64);
    for &q in lp {
        let d = sub(q, c);
        let rq = norm(d);
        let phi = dot(d, v).atan2(dot(d, u));
        let (bc, as_) = (eb * phi.cos(), ea * phi.sin());
        let re = ea * eb / fmla(bc, bc, as_ * as_).sqrt();
        let e = rq - re;
        s2 = fmla(e, e, s2);
        emax = emax.max(e.abs());
    }
    let rms_px = (s2 / lp.len() as f64).sqrt();
    f.rms = (rms_px / eb) as f32;
    f.max_err = (emax / eb) as f32;
    f.axis_ratio = (eb / ea) as f32;
    if rms_px > SHAPE_ELLIPSE_RMS as f64 * eb * tol || emax > SHAPE_ELLIPSE_MAX as f64 * eb * tol {
        return None;
    }
    Some(Ellipse { c, th, a: ea, b: eb, rms_px })
}

/// The ellipse's (or circle's) N vertices, from the one at the eccentric angle of the stroke's
/// first point, the stroke's way round, then the first again: N + 1 points.
fn ellipse_outline(e: &Ellipse, sign: f64, first: P) -> (ShapeKind, Vec<P>) {
    let circle = e.b / e.a >= SHAPE_CIRCLE_RATIO as f64;
    let (mut th, mut a, mut b) = (e.th, e.a, e.b);
    if circle {
        let r = 0.5 * (a + b);
        (th, a, b) = (0.0, r, r);
    } else {
        let q = std::f64::consts::FRAC_PI_2;
        let snapped = (th / q).round() * q;
        if (th - snapped).abs().to_degrees() <= SHAPE_ELLIPSE_AXIS_DEG as f64 {
            th = snapped;
        }
    }
    let (u, v) = (unit(th), unit(th + std::f64::consts::FRAC_PI_2));
    let d = sub(first, e.c);
    let t0 = (dot(d, v) / b).atan2(dot(d, u) / a);
    let n = shape_ellipse_vertices(a as f32) as usize;
    let dt = sign * 2.0 * std::f64::consts::PI / n as f64;
    let mut out: Vec<P> = (0..n)
        .map(|k| {
            let t = fmla(k as f64, dt, t0);
            madd(madd(e.c, u, a * t.cos()), v, b * t.sin())
        })
        .collect();
    out.push(out[0]);
    (if circle { ShapeKind::Circle } else { ShapeKind::Ellipse }, out)
}

// ---- v1.5: arrows, open polylines, arcs (ShapeParams::extras) ----

/// The thresholds of the v1.5 kinds (plan step 22). An arrow's barbs leave its tip at this many
/// degrees from the shaft, each this fraction of the shaft long.
pub const SHAPE_ARROW_MIN_DEG: f32 = 20.0;
pub const SHAPE_ARROW_MAX_DEG: f32 = 70.0;
pub const SHAPE_ARROW_MIN_FRAC: f32 = 0.15;
pub const SHAPE_ARROW_MAX_FRAC: f32 = 0.45;
/// An arc sweeps at least this many degrees (less is a sloppy line) and at most this many (more
/// should have closed).
pub const SHAPE_ARC_MIN_DEG: f32 = 75.0;
pub const SHAPE_ARC_MAX_DEG: f32 = 330.0;

/// An open stroke that is an arrow, a polyline or an arc: the kind and its outline. Each piece of
/// an arrow's shaft or a polyline is at least SHAPE_LINE_MIN_MM, as a line is, so writing (whose
/// strokes are shorter) stays writing; a polyline and an arc that both fit go to the closer.
fn open_shape(q: &[P], diag: f64, prm: &ShapeParams, mm: f64, tol: f64, f: &mut ShapeFeatures) -> Option<(ShapeKind, Vec<P>)> {
    let seg_min = fmla(-(1.0 - SHAPE_SEG_STRAIGHT as f64), tol, 1.0);
    let corners = straw_corners(q, false, seg_min);
    f.corners = corners.len() as i32;
    let step = page_step(prm);
    let min_len = SHAPE_LINE_MIN_MM as f64 * mm;
    if let Some(chain) = arrow(q, &corners, seg_min, min_len, prm) {
        return Some((ShapeKind::Arrow, segments(&chain, false, step)));
    }
    let poly = polyline(q, &corners, seg_min, min_len, diag, prm);
    let arc = arc_fit(q, min_len, tol, f);
    let use_poly = match (&poly, &arc) {
        (Some(pl), Some(ar)) => open_rms(q, pl) <= ar.rms_px,
        (Some(_), None) => true,
        _ => false,
    };
    if let (true, Some(v)) = (use_poly, poly) {
        return Some((ShapeKind::Polyline, segments(&v, false, step)));
    }
    arc.map(|a| (ShapeKind::Arc, arc_outline(&a)))
}

/// The RMS distance of the points from an open chain.
fn open_rms(q: &[P], v: &[P]) -> f64 {
    let mut s = 0.0;
    for &a in q {
        let mut d = f64::MAX;
        for w in v.windows(2) {
            d = d.min(seg_dist(a, w[0], w[1]));
        }
        s = fmla(d, d, s);
    }
    (s / q.len() as f64).sqrt()
}

/// The unsigned angle between two directions, in degrees.
fn deg_between(a: P, b: P) -> f64 {
    angle_between(a, b).abs().to_degrees()
}

/// An arrow drawn in one stroke: the shaft (tail to tip), a barb, and optionally back to the tip
/// and the other barb. The shaft is snapped as a line is, from its tail; two barbs are made
/// mirror images (their mean angle and length). The chain: tail, tip, barb[, tip, barb].
fn arrow(q: &[P], corners: &[usize], seg_min: f64, min_len: f64, prm: &ShapeParams) -> Option<Vec<P>> {
    let nc = corners.len();
    if nc != 3 && nc != 5 {
        return None;
    }
    for k in 0..nc - 1 {
        if seg_straight(q, false, corners[k], corners[k + 1]) < seg_min {
            return None;
        }
    }
    let at = |k: usize| q[corners[k]];
    let (tail, tip) = (at(0), at(1));
    let shaft = norm(sub(tip, tail));
    if shaft < min_len {
        return None;
    }
    let back = scale(sub(tail, tip), 1.0 / shaft);
    let barb_ok = |b: P| -> bool {
        let (l, a) = (norm(b), deg_between(back, b));
        let lens = SHAPE_ARROW_MIN_FRAC as f64 * shaft..=SHAPE_ARROW_MAX_FRAC as f64 * shaft;
        lens.contains(&l) && (SHAPE_ARROW_MIN_DEG as f64..=SHAPE_ARROW_MAX_DEG as f64).contains(&a)
    };
    let b1 = sub(at(2), tip);
    if !barb_ok(b1) {
        return None;
    }
    let mut barbs = vec![b1];
    if nc == 5 {
        // back to the tip, then the other barb, on the other side of the shaft
        if dist(at(3), tip) > 0.35 * norm(b1) {
            return None;
        }
        let b2 = sub(at(4), tip);
        if !barb_ok(b2) || cross(back, b1) * cross(back, b2) >= 0.0 {
            return None;
        }
        barbs.push(b2);
    }
    let (ang, _) = snap_angle((tip.y - tail.y).atan2(tip.x - tail.x), prm);
    let tip = madd(tail, unit(ang), shaft);
    let back_ang = ang + std::f64::consts::PI;
    let mut chain = vec![tail, tip];
    if barbs.len() == 2 {
        let th = 0.5 * (angle_between(back, barbs[0]).abs() + angle_between(back, barbs[1]).abs());
        let l = 0.5 * (norm(barbs[0]) + norm(barbs[1]));
        let s = if cross(back, barbs[0]) < 0.0 { -1.0 } else { 1.0 };
        chain.push(madd(tip, unit(fmla(s, th, back_ang)), l));
        chain.push(tip);
        chain.push(madd(tip, unit(fmla(-s, th, back_ang)), l));
    } else {
        let th = angle_between(back, barbs[0]);
        chain.push(madd(tip, unit(back_ang + th), norm(barbs[0])));
    }
    Some(chain)
}

/// An open polyline: 2 to 7 straight segments, each at least min_len. Its first segment is snapped
/// as a line is; each next one is made square to the one before when it turns within
/// SHAPE_RECT_DEG of 90 degrees, else snapped as a line. Each keeps its length along its line.
fn polyline(q: &[P], corners: &[usize], seg_min: f64, min_len: f64, diag: f64, prm: &ShapeParams) -> Option<Vec<P>> {
    let nc = corners.len();
    if !(3..=8).contains(&nc) {
        return None;
    }
    let mut lines = Vec::with_capacity(nc - 1);
    for k in 0..nc - 1 {
        let (a, b) = (corners[k], corners[k + 1]);
        if seg_straight(q, false, a, b) < seg_min || dist(q[a], q[b]) < min_len {
            return None;
        }
        lines.push(seg_line(q, false, a, b)?);
    }
    // the vertices as drawn: the ends projected on the end lines, the corners where lines cross
    let proj = |l: (P, P), a: P| madd(l.0, l.1, dot(sub(a, l.0), l.1));
    let mut v = vec![proj(lines[0], q[0])];
    for k in 1..nc - 1 {
        let c = intersect(lines[k - 1], lines[k])?;
        if dist(c, q[corners[k]]) > 0.25 * diag {
            return None;
        }
        v.push(c);
    }
    v.push(proj(lines[nc - 2], q[q.len() - 1]));
    // then rebuilt piece by piece from the start
    let mut out = vec![v[0]];
    let mut prev: Option<f64> = None;
    for k in 0..nc - 1 {
        let d = sub(v[k + 1], v[k]);
        let ang = d.y.atan2(d.x);
        let t = match prev {
            Some(pa) if (deg_between(unit(pa), d) - 90.0).abs() <= SHAPE_RECT_DEG as f64 => {
                let quarter = std::f64::consts::FRAC_PI_2;
                if cross(unit(pa), d) >= 0.0 {
                    pa + quarter
                } else {
                    pa - quarter
                }
            }
            _ => snap_angle(ang, prm).0,
        };
        let l = dot(d, unit(t));
        if l <= 0.0 {
            return None;
        }
        let last = out[out.len() - 1];
        out.push(madd(last, unit(t), l));
        prev = Some(t);
    }
    Some(out)
}

struct ArcFit {
    c: P,
    r: f64,
    /// the start's angle around c and the signed sweep (the stroke's way)
    a0: f64,
    sweep: f64,
    rms_px: f64,
}

/// A circle through the open stroke (the algebraic least squares x^2 + y^2 + D x + E y + F = 0),
/// its radial errors held to the ellipse's limits of its radius, sweeping SHAPE_ARC_MIN_DEG to
/// SHAPE_ARC_MAX_DEG, at least min_len long.
fn arc_fit(q: &[P], min_len: f64, tol: f64, f: &mut ShapeFeatures) -> Option<ArcFit> {
    if path_len(q) < min_len {
        return None;
    }
    let c0 = centroid(q);
    let mut sc = 0.0f64;
    for &a in q {
        sc = sc.max(dist(a, c0));
    }
    if sc <= 0.0 {
        return None;
    }
    // (in the centroid's frame, scaled to about 1, so the normal equations stay well conditioned)
    let mut m = [[0.0; 3]; 3];
    let mut rhs = [0.0; 3];
    for &a in q {
        let d = scale(sub(a, c0), 1.0 / sc);
        let phi = [d.x, d.y, 1.0];
        let z = -dot(d, d);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = fmla(phi[i], phi[j], m[i][j]);
            }
            rhs[i] = fmla(phi[i], z, rhs[i]);
        }
    }
    let [dd, ee, ff] = solve(m, rhs)?;
    let r2 = fmla(0.25 * dd, dd, fmla(0.25 * ee, ee, -ff));
    if r2.is_nan() || r2 <= 0.0 {
        return None;
    }
    let r = r2.sqrt() * sc;
    let c = madd(c0, p(-0.5 * dd, -0.5 * ee), sc);
    let (mut s2, mut emax) = (0.0, 0.0f64);
    for &a in q {
        let e = dist(a, c) - r;
        s2 = fmla(e, e, s2);
        emax = emax.max(e.abs());
    }
    let rms_px = (s2 / q.len() as f64).sqrt();
    f.rms = (rms_px / r) as f32;
    f.max_err = (emax / r) as f32;
    if rms_px > SHAPE_ELLIPSE_RMS as f64 * r * tol || emax > SHAPE_ELLIPSE_MAX as f64 * r * tol {
        return None;
    }
    let mut sweep = 0.0;
    for w in q.windows(2) {
        sweep += angle_between(sub(w[0], c), sub(w[1], c));
    }
    let sd = sweep.abs().to_degrees();
    if !(SHAPE_ARC_MIN_DEG as f64..=SHAPE_ARC_MAX_DEG as f64).contains(&sd) {
        return None;
    }
    let a0 = (q[0].y - c.y).atan2(q[0].x - c.x);
    Some(ArcFit { c, r, a0, sweep, rms_px })
}

/// The arc's points: the circle's vertex spacing (shape_ellipse_vertices of its radius) over its
/// sweep, from the stroke's start round its way: ceil(N x sweep / 2 pi) pieces, at least 2.
fn arc_outline(a: &ArcFit) -> Vec<P> {
    let n_full = shape_ellipse_vertices(a.r as f32) as f64;
    let k = ((n_full * a.sweep.abs() / (2.0 * std::f64::consts::PI)).ceil() as usize).max(2);
    (0..=k).map(|i| madd(a.c, unit(fmla(a.sweep, i as f64 / k as f64, a.a0)), a.r)).collect()
}

// ---- the samples' other fields ----

fn median(v: &mut [f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// The snapped points as samples: the pressure is the median of the raw samples in the middle 80%
/// of the stroke by arc length (the lead-in and lift-off are light), the tilt the median of all of
/// them, and each point's time the raw time at the same fraction of the arc length.
fn finish(raw: &[PenSample], out: &[P]) -> Vec<PenSample> {
    let n = raw.len();
    let mut cum = vec![0.0f64; n];
    for k in 1..n {
        let d = p((raw[k].x - raw[k - 1].x) as f64, (raw[k].y - raw[k - 1].y) as f64);
        cum[k] = cum[k - 1] + norm(d);
    }
    let total = cum[n - 1];
    let mut pr: Vec<f32> = (0..n).filter(|&k| cum[k] >= 0.1 * total && cum[k] <= 0.9 * total).map(|k| raw[k].pressure).collect();
    if pr.is_empty() {
        pr = raw.iter().map(|s| s.pressure).collect();
    }
    let pressure = median(&mut pr);
    let tilt_x = median(&mut raw.iter().map(|s| s.tilt_x).collect::<Vec<_>>());
    let tilt_y = median(&mut raw.iter().map(|s| s.tilt_y).collect::<Vec<_>>());
    // Times as offsets from the first sample (now_ms wraps: PORTING.md, Timing).
    let t0 = raw[0].t_ms;
    let off: Vec<f64> = raw.iter().map(|s| s.t_ms.wrapping_sub(t0) as f64).collect();
    let olen = path_len(out);
    let mut ocum = 0.0;
    let mut seg = 0;
    let mut res = Vec::with_capacity(out.len());
    for (k, &q) in out.iter().enumerate() {
        if k > 0 {
            ocum += dist(out[k - 1], q);
        }
        let target = if olen > 0.0 { ocum / olen * total } else { 0.0 };
        while seg + 1 < n - 1 && cum[seg + 1] < target {
            seg += 1;
        }
        let t = if k == out.len() - 1 {
            off[n - 1]
        } else if n == 1 || cum[seg + 1] <= cum[seg] {
            off[seg]
        } else {
            let f = ((target - cum[seg]) / (cum[seg + 1] - cum[seg])).clamp(0.0, 1.0);
            fmla(off[seg + 1] - off[seg], f, off[seg])
        };
        res.push(PenSample {
            x: q.x as f32,
            y: q.y as f32,
            pressure,
            tilt_x,
            tilt_y,
            t_ms: t0.wrapping_add(t.round() as u32),
        });
    }
    res
}

/// The snapped stroke: the raw stroke's tool, params, seed and ext (so its colour, layer and tip
/// variant), with the result's points; its bbox is grown point by point as stroke_append grows
/// every stroke's. None when the result is no shape.
pub fn shape_stroke(raw: &Stroke, r: &ShapeResult, dpi: f32) -> Option<Box<Stroke>> {
    if r.kind == ShapeKind::None || r.pts.is_empty() {
        return None;
    }
    let mut s = stroke_new(raw.tool, &raw.params, raw.seed);
    s.ext = raw.ext;
    for q in &r.pts {
        stroke_append(&mut s, q, dpi);
    }
    Some(s)
}
