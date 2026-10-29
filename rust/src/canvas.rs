//! canvas.h: 8-bit grayscale raster (0 black .. 255 white) with a clip rect.

use crate::prelude::*;

/// The C's `owned` flag is the Vec: a canvas always owns its pixels. A canvas with no pixels (px
/// empty, w and h set) is page.c's dry canvas, which a dry brush measures on and never touches.
#[derive(Clone, Debug, Default)]
pub struct Canvas {
    /// row-major, stride bytes per row
    pub px: Vec<u8>,
    pub w: i32,
    pub h: i32,
    pub stride: i32,
    /// all drawing is clipped to this AND to the canvas bounds
    pub clip: Rect,
}

/// allocates, fills white, clip = full. -1 when the memory is not there (the C's malloc failing:
/// a page-size canvas is megabytes, and the Kindle has 1 GB), as try_reserve tells it.
pub fn canvas_init(c: &mut Canvas, w: i32, h: i32) -> i32 {
    *c = Canvas::default();
    let n = (w.max(0) as usize) * (h.max(0) as usize);
    let mut px = Vec::new();
    if px.try_reserve_exact(n).is_err() {
        return -1;
    }
    px.resize(n, 255u8);
    *c = Canvas { px, w, h, stride: w, clip: Rect { x: 0, y: 0, w, h } ,
    };
    0
}
pub fn canvas_free(c: &mut Canvas) {
    *c = Canvas::default();
}
/// intersected with bounds
pub fn canvas_set_clip(c: &mut Canvas, r: Rect) {
    c.clip = rect_intersect(r, Rect { x: 0, y: 0, w: c.w, h: c.h ,
        },
    );
}
pub fn canvas_reset_clip(c: &mut Canvas) {
    c.clip = Rect { x: 0, y: 0, w: c.w, h: c.h ,
    };
}

pub fn canvas_fill(c: &mut Canvas, r: Rect, gray: u8) {
    let r = rect_intersect(r, c.clip);
    for y in r.y..r.y + r.h {
        let at = (y * c.stride + r.x) as usize;
        c.px[at..at + r.w as usize].fill(gray);
    }
}
/// Outline of thickness t drawn inside r.
pub fn canvas_rect_outline(c: &mut Canvas, r: Rect, t: i32, gray: u8) {
    if t * 2 >= r.w || t * 2 >= r.h {
        canvas_fill(c, r, gray);
        return;
    }
    canvas_fill(c, Rect { x: r.x, y: r.y, w: r.w, h: t ,
        }, gray,
    );
    canvas_fill(c, Rect { x: r.x, y: r.y + r.h - t, w: r.w, h: t ,
        }, gray,
    );
    canvas_fill(c, Rect { x: r.x, y: r.y + t, w: t, h: r.h - 2 * t ,
        }, gray,
    );
    canvas_fill(c, Rect { x: r.x + r.w - t, y: r.y + t, w: t, h: r.h - 2 * t ,
        }, gray,
    );
}

/// A solid black slash rising across r. The mask stays inside r and the canvas clip;
/// only black and white are used, as required by the e-ink panel's DU waveform.
pub fn canvas_slash(c: &mut Canvas, r: Rect, thick: i32) {
    if thick <= 0 || rect_is_empty(r) || c.px.is_empty() { return; }
    let clip = rect_intersect(rect_intersect(r, c.clip), Rect { x: 0, y: 0, w: c.w, h: c.h });
    if rect_is_empty(clip) { return; }
    let pad = thick.min(r.w).min(r.h) as f64 * 0.5;
    let (x0, y0) = (r.x as f64 + pad, (r.y + r.h) as f64 - pad);
    let (dx, dy) = ((r.w as f64 - 2.0 * pad).max(0.0), -(r.h as f64 - 2.0 * pad).max(0.0));
    let len2 = dx * dx + dy * dy;
    for y in clip.y..clip.y + clip.h {
        for x in clip.x..clip.x + clip.w {
            let (px, py) = (x as f64 + 0.5 - x0, y as f64 + 0.5 - y0);
            let t = if len2 > 0.0 { ((px * dx + py * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            let (ex, ey) = (px - t * dx, py - t * dy);
            if ex * ex + ey * ey <= pad * pad { c.px[(y * c.stride + x) as usize] = 0; }
        }
    }
}

/// "Over" blend of gray at alpha a into one pixel (clip already checked).
/// (`*p + (gray - *p) * a` is one contracted multiply-add in the C build, see fmla, then the 0.5 is
/// added on its own.)
#[inline]
fn blend_px(p: &mut u8, gray: u8, a: f32) {
    if a <= 0.0 {
        return;
    }
    if a >= 1.0 {
        *p = gray;
        return;
    }
    *p = (fmla((gray as i32 - *p as i32) as f32, a, *p as f32) + 0.5) as u8;
}

/// Coverage of a pixel whose center is at signed distance d from an edge (negative = inside).
#[inline]
fn cov(d: f32) -> f32 {
    clampf(0.5 - d, 0.0, 1.0)
}

/// Coverage of the pixel centered at (px, py) by a rounded rect (or its outline of width t).
#[inline]
fn rr_alpha(r: Rect, rr: f32, px: f32, py: f32, filled: bool, t: i32) -> f32 {
    // Distance to the rounded rect boundary (negative inside).
    let cx = clampf(px, r.x as f32 + rr, (r.x + r.w) as f32 - rr);
    let cy = clampf(py, r.y as f32 + rr, (r.y + r.h) as f32 - rr);
    let dx = px - cx;
    let dy = py - cy;
    let d = if dx != 0.0 || dy != 0.0 {
        // in a corner zone (dx * dx + dy * dy: the first product fused, the second rounded)
        fmla(dx, dx, dy * dy).sqrt() - rr
    } else {
        // core: distance to the nearest straight edge
        -((px - r.x as f32).min((r.x + r.w) as f32 - px)).min((py - r.y as f32).min((r.y + r.h) as f32 - py))
    };
    let mut a = cov(d);
    if !filled {
        a = a.min(cov(-(d + t as f32)));
    }
    a
}

/// Rounded rect, filled or outlined (thickness t), antialiased corners.
pub fn canvas_round_rect(c: &mut Canvas, r: Rect, mut radius: i32, gray: u8, filled: bool, t: i32) {
    if rect_is_empty(r) {
        return;
    }
    let maxr = (if r.w < r.h { r.w } else { r.h }) / 2;
    if radius > maxr {
        radius = maxr;
    }
    if radius < 0 {
        radius = 0;
    }
    let b = rect_intersect(r, c.clip);
    let rr = radius as f32;
    // Pixels farther than E from the left and right edges all get the same coverage within a
    // row (their distance to the boundary is vertical, or saturates the coverage either way),
    // so each row computes the left and right edge zones per pixel and the middle span once.
    // Same output as the per-pixel loop, at a fraction of the cost (a keyboard redraw went
    // from ~6 M evaluated pixels to a few hundred thousand).
    let e = (if radius > t { radius } else { t }) + 2;
    let (m0, m1) = (r.x + e, r.x + r.w - e); // middle span [m0, m1)
    for y in b.y..b.y + b.h {
        let py = y as f32 + 0.5;
        let row = y as usize * c.stride as usize;
        let mut x = b.x;
        while x < b.x + b.w {
            if x >= m0 && x < m1 {
                let a = rr_alpha(r, rr, x as f32 + 0.5, py, filled, t);
                let x1 = if m1 < b.x + b.w { m1 } else { b.x + b.w };
                if a >= 1.0 {
                    c.px[row + x as usize..row + x1 as usize].fill(gray);
                } else if a > 0.0 {
                    for k in x..x1 {
                        blend_px(&mut c.px[row + k as usize], gray, a);
                    }
                }
                x = x1;
                continue;
            }
            let a = rr_alpha(r, rr, x as f32 + 0.5, py, filled, t);
            if a > 0.0 {
                blend_px(&mut c.px[row + x as usize], gray, a);
            }
            x += 1;
        }
    }
}

/// Coverage of a rect with its corners cut off at 45 degrees by ch px (an octagon; a square with
/// ch = half its side is a lozenge). The boundary is the intersection of half-planes, so the max
/// of the plane distances is the exact signed distance inside and a close one outside.
// (0.70710678f is the C's literal: the nearest float to 1 / sqrt(2), as FRAC_1_SQRT_2 is)
#[allow(clippy::approx_constant, clippy::too_many_arguments)]
#[inline]
fn ch_alpha(cx: f32, cy: f32, hw: f32, hh: f32, k: f32, px: f32, py: f32, filled: bool, t: i32,
) -> f32 {
    let ax = (px - cx).abs();
    let ay = (py - cy).abs();
    let d = (ax - hw).max(ay - hh).max((ax + ay - k) * 0.70710678);
    let mut a = cov(d);
    if !filled {
        a = a.min(cov(-(d + t as f32)));
    }
    a
}

/// Rect with 45-degree cut corners (ch px), same filled/outline rules as canvas_round_rect.
pub fn canvas_chamfer_rect(c: &mut Canvas, r: Rect, mut ch: i32, gray: u8, filled: bool, t: i32) {
    if rect_is_empty(r) {
        return;
    }
    let maxc = (if r.w < r.h { r.w } else { r.h }) / 2;
    if ch > maxc {
        ch = maxc;
    }
    if ch < 0 {
        ch = 0;
    }
    let b = rect_intersect(r, c.clip);
    let hw = r.w as f32 * 0.5;
    let hh = r.h as f32 * 0.5;
    let cx = r.x as f32 + hw;
    let cy = r.y as f32 + hh;
    let k = hw + hh - ch as f32;
    // As in canvas_round_rect: away from the cut corners a row's coverage depends on y only.
    let e = ch + t + 2;
    let (m0, m1) = (r.x + e, r.x + r.w - e);
    for y in b.y..b.y + b.h {
        let py = y as f32 + 0.5;
        let row = y as usize * c.stride as usize;
        let mut x = b.x;
        while x < b.x + b.w {
            if x >= m0 && x < m1 {
                let a = ch_alpha(cx, cy, hw, hh, k, x as f32 + 0.5, py, filled, t);
                let x1 = if m1 < b.x + b.w { m1 } else { b.x + b.w };
                if a >= 1.0 {
                    c.px[row + x as usize..row + x1 as usize].fill(gray);
                } else if a > 0.0 {
                    for q in x..x1 {
                        blend_px(&mut c.px[row + q as usize], gray, a);
                    }
                }
                x = x1;
                continue;
            }
            let a = ch_alpha(cx, cy, hw, hh, k, x as f32 + 0.5, py, filled, t);
            if a > 0.0 {
                blend_px(&mut c.px[row + x as usize], gray, a);
            }
            x += 1;
        }
    }
}

/// Filled diamond of "radius" r (center to tip).
pub fn canvas_lozenge(c: &mut Canvas, cx: f32, cy: f32, r: f32, gray: u8) {
    let s = (r * 2.0).round_ties_even() as i32;
    canvas_chamfer_rect(
        c,
        Rect { x: (cx - r).round_ties_even() as i32, y: (cy - r).round_ties_even() as i32, w: s, h: s ,
        },
        s / 2,
        gray,
        true,
        0,
    );
}

/// Axis-aligned crisp lines (used by paper rules). Thickness grows down/right.
pub fn canvas_hline(c: &mut Canvas, mut x0: i32, mut x1: i32, y: i32, t: i32, gray: u8) {
    if x1 < x0 {
        std::mem::swap(&mut x0, &mut x1);
    }
    canvas_fill(c, Rect { x: x0, y, w: x1 - x0, h: t ,
        }, gray,
    );
}
pub fn canvas_vline(c: &mut Canvas, x: i32, mut y0: i32, mut y1: i32, t: i32, gray: u8) {
    if y1 < y0 {
        std::mem::swap(&mut y0, &mut y1);
    }
    canvas_fill(c, Rect { x, y: y0, w: t, h: y1 - y0 ,
        }, gray,
    );
}

/// The dashes of an edge len px long, each `on` long with the gaps about `off`: (start, length) in whole
/// pixels. The nearest whole number of dashes to on-and-off fits, the gaps stretched or squeezed to it, so
/// both ends of the edge are dashes (the corners are solid). An edge too short for two dashes is one run.
fn dash_runs(len: i32, on: i32, off: i32) -> Vec<(i32, i32)> {
    let on = on.max(1);
    if len <= 2 * on {
        return vec![(0, len.max(0))];
    }
    let off = off.max(0);
    let n = (((len + off) as f32 / (on + off) as f32).round() as i32).clamp(2, (len / on).max(2));
    let span = (len - on) as f32 / (n - 1) as f32;
    (0..n).map(|k| ((k as f32 * span).round() as i32, on)).collect()
}

/// A dashed rectangle for an overlay that is not ink: `thick` px dashes, `on` long with about `off` between,
/// along the four edges of r (drawn inside it, as canvas_rect_outline does), each edge starting and ending
/// on a dash so the corners are solid. A white halo band of `halo` px on both sides of every edge goes down
/// first, so the dashes read over ink and over rules alike. Only 0 and 255, so a 1-bit refresh shows it
/// as it is; everything is clipped to the canvas's clip like canvas_fill.
pub fn canvas_dash_rect(c: &mut Canvas, r: Rect, thick: i32, on: i32, off: i32, halo: i32) {
    if rect_is_empty(r) || thick <= 0 {
        return;
    }
    let t = thick.min(r.w).min(r.h);
    let top = Rect { x: r.x, y: r.y, w: r.w, h: t ,
    };
    let bottom = Rect { x: r.x, y: r.y + r.h - t, w: r.w, h: t ,
    };
    let left = Rect { x: r.x, y: r.y, w: t, h: r.h ,
    };
    let right = Rect { x: r.x + r.w - t, y: r.y, w: t, h: r.h ,
    };
    if halo > 0 {
        for e in [top, bottom, left, right] {
            canvas_fill(c, Rect { x: e.x - halo, y: e.y - halo, w: e.w + 2 * halo, h: e.h + 2 * halo ,
                }, 255,
            );
        }
    }
    for (x, w) in dash_runs(r.w, on, off) {
        canvas_fill(c, Rect { x: r.x + x, y: top.y, w, h: t ,
            }, 0,
        );
        canvas_fill(c, Rect { x: r.x + x, y: bottom.y, w, h: t ,
            }, 0,
        );
    }
    for (y, h) in dash_runs(r.h, on, off) {
        canvas_fill(c, Rect { x: left.x, y: r.y + y, w: t, h ,
            }, 0,
        );
        canvas_fill(c, Rect { x: right.x, y: r.y + y, w: t, h ,
            }, 0,
        );
    }
}

/// A dashed polyline for an overlay that is not ink (the lasso around a link's area): a square pen
/// `thick` px wide walks the path, on for `on` px and off for `off` px measured along it, starting `phase` px
/// into that pattern, and the phase at the path's end comes back, so a path drawn piece by piece (each piece
/// starting at the last one's final point, with the phase the last one returned) is the path drawn whole,
/// pixel for pixel. Lengths are counted in whole pixels per segment (each segment's length rounded), which is
/// what keeps the pieces exact; the pen's own width counts toward a dash, so a horizontal or vertical run
/// shows dashes `on` long and gaps `off` long. A path's final point is never stamped: it is the next piece's
/// first. With `halo` > 0 a white band of `halo` px goes down first on both sides of the whole path (dashes
/// and gaps), so the dashes read over ink; drawn piece by piece, a halo may clip the end of the piece before,
/// so a caller that draws in pieces passes 0. Only 0 and 255; clipped to the canvas's clip like canvas_fill.
pub fn canvas_dash_poly(c: &mut Canvas, pts: &[(f32, f32)], thick: i32, on: i32, off: i32, halo: i32, phase: i32,
) -> i32 {
    let period = (on.max(0) + off.max(0)).max(1);
    let mut ph = phase.rem_euclid(period);
    if pts.len() < 2 {
        return ph;
    }
    // the samples a dash inks: the pen's width already adds thick - 1 px to every run of them
    let ink_len = if off <= 0 { period } else { (on - (thick - 1)).max(1).min(on.max(1)) };
    let stamp = |c: &mut Canvas, x: f32, y: f32, t: i32, gray: u8| {
        let x0 = (x - t as f32 / 2.0 + 0.5).floor() as i32;
        let y0 = (y - t as f32 / 2.0 + 0.5).floor() as i32;
        canvas_fill(c, Rect { x: x0, y: y0, w: t, h: t ,
            }, gray,
        );
    };
    // the integer length of each segment, the same in the whole path and in any piece of it
    let seg_len = |a: (f32, f32), b: (f32, f32)| -> i32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt();
        if l.is_finite() { l.round().min(1.0e6) as i32 } else { 0 }
    };
    if thick > 0 && halo > 0 {
        let ht = thick + 2 * halo;
        for w in pts.windows(2) {
            let l = seg_len(w[0], w[1]);
            for t in 0..l {
                let f = t as f32 / l as f32;
                stamp(c, w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f, ht, 255,
                );
            }
        }
    }
    for w in pts.windows(2) {
        let l = seg_len(w[0], w[1]);
        if thick > 0 && on > 0 {
            for t in 0..l {
                if (ph + t) % period < ink_len {
                    let f = t as f32 / l as f32;
                    stamp(c, w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f, thick, 0,
                    );
                }
            }
        }
        ph = (ph + l % period) % period;
    }
    ph
}

/// A dotted polyline for a screen-only overlay: round dots of radius `r` at `pitch` px along
/// the path. A white ring of `halo` px is painted first, so the dots read over ink and paper
/// rules alike. Only 0 and 255 are written. `phase` is distance already walked into the current
/// pitch; the returned phase lets a path be painted in pieces without changing the dot spacing.
/// As with canvas_dash_poly, each segment's length is rounded to a whole pixel and the final
/// point is included, so adjoining pieces can share their boundary point.
fn dot_pass(
    c: &mut Canvas,
    pts: &[(f32, f32)],
    radius: i32,
    pitch: i32,
    phase: i32,
    gray: u8,
) -> i32 {
    if pts.is_empty() {
        return phase;
    }
    // A dot is a binary raster disc, centred on the logical pixel coordinates supplied by the
    // path. The clip includes the canvas bounds, as it does for canvas_fill.
    let stamp = |c: &mut Canvas, x: f32, y: f32| {
        if radius < 0 || !x.is_finite() || !y.is_finite() {
            return;
        }
        let b = rect_intersect(
            Rect {
                x: (x - radius as f32).floor() as i32,
                y: (y - radius as f32).floor() as i32,
                w: 2 * radius + 1,
                h: 2 * radius + 1,
            },
            c.clip,
        );
        let rr = radius as f32 * radius as f32;
        for py in b.y..b.y + b.h {
            let dy = py as f32 - y;
            for px in b.x..b.x + b.w {
                let dx = px as f32 - x;
                if fmla(dx, dx, dy * dy) <= rr {
                    c.px[(py * c.stride + px) as usize] = gray;
                }
            }
        }
    };
    let seg_len = |a: (f32, f32), b: (f32, f32)| -> i32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt();
        if l.is_finite() {
            l.round().min(1.0e6) as i32
        } else {
            0
        }
    };
    if pts.len() == 1 {
        if phase == 0 {
            stamp(c, pts[0].0, pts[0].1);
        }
        return phase;
    }
    let mut ph = phase;
    for w in pts.windows(2) {
        let len = seg_len(w[0], w[1]);
        let first = if ph == 0 { 0 } else { pitch - ph };
        let mut t = first;
        while t <= len {
            let f = if len > 0 { t as f32 / len as f32 } else { 0.0 };
            stamp(
                c,
                w[0].0 + (w[1].0 - w[0].0) * f,
                w[0].1 + (w[1].1 - w[0].1) * f,
            );
            t += pitch;
        }
        ph = (ph + len % pitch).rem_euclid(pitch);
    }
    ph
}

pub fn canvas_dot_poly(
    c: &mut Canvas,
    pts: &[(f32, f32)],
    r: i32,
    pitch: i32,
    halo: i32,
    phase: i32,
) -> i32 {
    let pitch = pitch.max(1);
    let phase = phase.rem_euclid(pitch);
    // Paint all halos first, so no halo erases a neighbouring dot's black core.
    if halo > 0 {
        dot_pass(c, pts, r.max(0).saturating_add(halo), pitch, phase, 255);
    }
    dot_pass(c, pts, r.max(0), pitch, phase, 0)
}

/// Antialiased line with round caps; width in px (may be fractional). Opaque paint (not deposit).
pub fn canvas_line_aa(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: u8) {
    let hw = width * 0.5;
    let b = rect_intersect(rect_from_points(x0, y0, x1, y1, hw + 1.0), c.clip);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = fmla(dx, dx, dy * dy);
    for y in b.y..b.y + b.h {
        let py = y as f32 + 0.5;
        // The capsule lies inside this infinite strip. Restrict work to its row span,
        // then retain the original coverage arithmetic for identical edge pixels and caps.
        let (left, right) = if dy != 0.0 && len2.is_finite() {
            let centre = x0 as f64 + (py as f64 - y0 as f64) * dx as f64 / dy as f64;
            let reach = (hw as f64 + 0.5).max(0.0) * (len2 as f64).sqrt() / (dy as f64).abs();
            (b.x.max(((centre - reach).floor() as i32).saturating_sub(2)),
             (b.x + b.w).min(((centre + reach).ceil() as i32).saturating_add(2)))
        } else { (b.x, b.x + b.w) };
        for x in left..right {
            let px = x as f32 + 0.5;
            let t = if len2 > 0.0 { clampf(fmla(px - x0, dx, (py - y0) * dy) / len2, 0.0, 1.0) } else { 0.0 };
            let ex = px - fmla(dx, t, x0);
            let ey = py - fmla(dy, t, y0);
            let a = cov(fmla(ex, ex, ey * ey).sqrt() - hw);
            if a > 0.0 {
                blend_px(&mut c.px[(y * c.stride + x) as usize], gray, a);
            }
        }
    }
}

#[cfg(test)]
fn canvas_line_aa_reference(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: u8) {
    let hw = width * 0.5;
    let b = rect_intersect(rect_from_points(x0, y0, x1, y1, hw + 1.0), c.clip);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = fmla(dx, dx, dy * dy);
    for y in b.y..b.y + b.h {
        let py = y as f32 + 0.5;
        for x in b.x..b.x + b.w {
            let px = x as f32 + 0.5;
            let t = if len2 > 0.0 { clampf(fmla(px - x0, dx, (py - y0) * dy) / len2, 0.0, 1.0) } else { 0.0 };
            let ex = px - fmla(dx, t, x0);
            let ey = py - fmla(dy, t, y0);
            let a = cov(fmla(ex, ex, ey * ey).sqrt() - hw);
            if a > 0.0 {
                blend_px(&mut c.px[(y * c.stride + x) as usize], gray, a);
            }
        }
    }
}

/// Binary Bayer fill for a map node: 0 empty, 1 quarter, 2 half, 3 three quarters, 4 solid.
/// Pattern cells are 2 by 2 pixels and anchored to screen coordinates.
pub fn canvas_disc_pattern(c: &mut Canvas, cx: f32, cy: f32, radius: f32, level: i32) {
    const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    if radius <= 0.0 || !radius.is_finite() || c.px.is_empty() { return; }
    let b = rect_intersect(rect_from_points(cx, cy, cx, cy, radius), c.clip);
    let threshold = level.clamp(0, 4) * 4;
    for y in b.y..b.y+b.h {
        for x in b.x..b.x+b.w {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            if fmla(dx, dx, dy * dy) <= radius * radius {
                let black = BAYER[((y / 2) & 3) as usize][((x / 2) & 3) as usize] < threshold;
                c.px[(y * c.stride + x) as usize] = if black { 0 } else { 255 };
            }
        }
    }
}

/// Antialiased filled disc painted toward gray with alpha in 0..1 (normal "over" blend).
pub fn canvas_disc_aa(c: &mut Canvas, cx: f32, cy: f32, r: f32, gray: u8, alpha: f32) {
    let b = rect_intersect(rect_from_points(cx, cy, cx, cy, r + 1.0), c.clip);
    for y in b.y..b.y + b.h {
        let py = y as f32 + 0.5 - cy;
        for x in b.x..b.x + b.w {
            let px = x as f32 + 0.5 - cx;
            let a = cov(fmla(px, px, py * py).sqrt() - r) * alpha;
            if a > 0.0 {
                blend_px(&mut c.px[(y * c.stride + x) as usize], gray, a);
            }
        }
    }
}

/// Ink deposit: darken-only blend. dst = dst - (dst - ink) * a, only if that is darker.
/// (clang contracts `*p - (*p - ink) * a` into one multiply-add, as its default -ffp-contract=on
/// allows; see fmla.)
#[inline]
pub fn canvas_deposit_px(c: &mut Canvas, x: i32, y: i32, ink: u8, a: f32) {
    if x < c.clip.x || y < c.clip.y || x >= c.clip.x + c.clip.w || y >= c.clip.y + c.clip.h {
        return;
    }
    let p = &mut c.px[(y * c.stride + x) as usize];
    if *p <= ink {
        return;
    }
    let v = (fmla(-((*p - ink) as f32), a, *p as f32) + 0.5) as i32;
    if v < *p as i32 {
        *p = v as u8;
    }
}

/// Paper restore, the deposit's mirror (erase strokes): lighten-only toward the paper value under
/// the pixel, never past it. dst = dst + (paper - dst) * a, only if that is lighter; a >= 0.999
/// lands exactly on the paper.
/// (`*p + (paper - *p) * a` is one contracted multiply-add in the C build, as above.)
#[inline]
pub fn canvas_restore_px(c: &mut Canvas, x: i32, y: i32, paper: u8, a: f32) {
    if x < c.clip.x || y < c.clip.y || x >= c.clip.x + c.clip.w || y >= c.clip.y + c.clip.h {
        return;
    }
    let p = &mut c.px[(y * c.stride + x) as usize];
    if *p >= paper {
        return;
    }
    if a >= 0.999 {
        *p = paper;
        return;
    }
    let v = (fmla((paper - *p) as f32, a, *p as f32) + 0.5) as i32;
    if v > *p as i32 {
        *p = if v < paper as i32 { v as u8 } else { paper };
    }
}

/// Copy src rect to dst at (dx,dy), clipped by dst clip.
pub fn canvas_blit(dst: &mut Canvas, src: &Canvas, srect: Rect, dx: i32, dy: i32) {
    let s = rect_intersect(srect, Rect { x: 0, y: 0, w: src.w, h: src.h ,
        },
    );
    let d = rect_intersect(Rect { x: dx, y: dy, w: s.w, h: s.h ,
        }, dst.clip,
    );
    for y in d.y..d.y + d.h {
        let sy = s.y + (y - dy);
        let sx = s.x + (d.x - dx);
        let to = (y * dst.stride + d.x) as usize;
        let from = (sy * src.stride + sx) as usize;
        dst.px[to..to + d.w as usize].copy_from_slice(&src.px[from..from + d.w as usize]);
    }
}

// The two downsamples read a whole page for a thumbnail or a paper's preview: the pens are read
// every 64 rows of it (work_pump, common.h).
/// Box-filter downsample whole src into dst (dst size defines the scale).
pub fn canvas_downsample(dst: &mut Canvas, src: &Canvas) {
    for y in 0..dst.h {
        let sy0 = (y as i64 * src.h as i64 / dst.h as i64) as i32;
        let mut sy1 = ((y + 1) as i64 * src.h as i64 / dst.h as i64) as i32;
        if sy1 <= sy0 {
            sy1 = sy0 + 1;
        }
        if (sy0 >> 6) != (sy1 >> 6) {
            work_pump();
        }
        for x in 0..dst.w {
            let sx0 = (x as i64 * src.w as i64 / dst.w as i64) as i32;
            let mut sx1 = ((x + 1) as i64 * src.w as i64 / dst.w as i64) as i32;
            if sx1 <= sx0 {
                sx1 = sx0 + 1;
            }
            let (mut sum, mut n) = (0u32, 0u32);
            let mut yy = sy0;
            while yy < sy1 && yy < src.h {
                let mut xx = sx0;
                while xx < sx1 && xx < src.w {
                    sum += src.px[(yy * src.stride + xx) as usize] as u32;
                    n += 1;
                    xx += 1;
                }
                yy += 1;
            }
            dst.px[(y * dst.stride + x) as usize] = (sum + n / 2).checked_div(n).map_or(255, |v| v as u8);
        }
    }
}

/// Downsample keeping thin dark lines visible: each output pixel blends the box average
/// toward the box minimum (weight 0..1). Used for paper previews.
pub fn canvas_downsample_keep_lines(dst: &mut Canvas, src: &Canvas, weight: f32) {
    for y in 0..dst.h {
        let sy0 = (y as i64 * src.h as i64 / dst.h as i64) as i32;
        let mut sy1 = ((y + 1) as i64 * src.h as i64 / dst.h as i64) as i32;
        if sy1 <= sy0 {
            sy1 = sy0 + 1;
        }
        if (sy0 >> 6) != (sy1 >> 6) {
            work_pump();
        }
        for x in 0..dst.w {
            let sx0 = (x as i64 * src.w as i64 / dst.w as i64) as i32;
            let mut sx1 = ((x + 1) as i64 * src.w as i64 / dst.w as i64) as i32;
            if sx1 <= sx0 {
                sx1 = sx0 + 1;
            }
            let (mut sum, mut n) = (0u32, 0u32);
            let mut mn = 255u8;
            let mut yy = sy0;
            while yy < sy1 && yy < src.h {
                let mut xx = sx0;
                while xx < sx1 && xx < src.w {
                    let v = src.px[(yy * src.stride + xx) as usize];
                    sum += v as u32;
                    n += 1;
                    if v < mn {
                        mn = v;
                    }
                    xx += 1;
                }
                yy += 1;
            }
            let avg = if n != 0 { sum as f32 / n as f32 } else { 255.0 };
            let v = lerpf(avg, mn as f32, weight);
            dst.px[(y * dst.stride + x) as usize] = v.round_ties_even() as i32 as u8;
        }
    }
}

/// Scales src into box keeping its aspect (box filter down, nearest up), centered in box.
pub fn canvas_blit_fit(dst: &mut Canvas, src: &Canvas, box_: Rect) {
    let sc = (box_.w as f32 / src.w as f32).min(box_.h as f32 / src.h as f32);
    let w = (src.w as f32 * sc) as i32;
    let h = (src.h as f32 * sc) as i32;
    let ox = box_.x + (box_.w - w) / 2;
    let oy = box_.y + (box_.h - h) / 2;
    let clip = rect_intersect(Rect { x: ox, y: oy, w, h }, dst.clip);
    for y in clip.y..clip.y + clip.h {
        let sy0 = ((y - oy) as f32 / sc) as i32;
        let mut sy1 = ((y - oy + 1) as f32 / sc) as i32;
        if sy1 <= sy0 {
            sy1 = sy0 + 1;
        }
        for x in clip.x..clip.x + clip.w {
            let sx0 = ((x - ox) as f32 / sc) as i32;
            let mut sx1 = ((x - ox + 1) as f32 / sc) as i32;
            if sx1 <= sx0 {
                sx1 = sx0 + 1;
            }
            let (mut sum, mut n) = (0u32, 0u32);
            let mut yy = sy0;
            while yy < sy1 && yy < src.h {
                let mut xx = sx0;
                while xx < sx1 && xx < src.w {
                    sum += src.px[(yy * src.stride + xx) as usize] as u32;
                    n += 1;
                    xx += 1;
                }
                yy += 1;
            }
            dst.px[(y * dst.stride + x) as usize] = sum.checked_div(n).map_or(255, |v| v as u8);
        }
    }
}

/// Invert a rect (used for pressed/selected feedback).
pub fn canvas_invert(c: &mut Canvas, r: Rect) {
    let r = rect_intersect(r, c.clip);
    for y in r.y..r.y + r.h {
        let at = (y * c.stride + r.x) as usize;
        for p in &mut c.px[at..at + r.w as usize] {
            *p = 255 - *p;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(w: i32, h: i32) -> Canvas {
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, w, h), 0);
        c
    }

    fn at(c: &Canvas, x: i32, y: i32) -> u8 {
        c.px[(y * c.stride + x) as usize]
    }

    #[test]
    fn fill_stays_inside_clip_and_bounds() {
        let mut c = canvas(20, 10);
        canvas_set_clip(&mut c, Rect { x: 5, y: 2, w: 100, h: 3 ,
            },
        );
        assert_eq!(c.clip, Rect { x: 5, y: 2, w: 15, h: 3 });
        canvas_fill(&mut c, Rect { x: -4, y: 0, w: 200, h: 200 ,
            }, 7,
        );
        let dark = c.px.iter().filter(|&&p| p == 7).count();
        assert_eq!(dark, 15 * 3);
        assert_eq!(at(&c, 4, 3), 255);
        assert_eq!(at(&c, 5, 3), 7);
        assert_eq!(at(&c, 19, 4), 7);
        assert_eq!(at(&c, 19, 5), 255);
    }

    #[test]
    fn blit_is_clipped_on_both_sides() {
        let mut src = canvas(8, 8);
        canvas_fill(&mut src, Rect { x: 0, y: 0, w: 8, h: 8 ,
            }, 10,
        );
        canvas_fill(&mut src, Rect { x: 4, y: 4, w: 4, h: 4 ,
            }, 20,
        );
        let mut dst = canvas(10, 10);
        // the source rect hangs out of the source; the destination hangs out of the canvas
        canvas_blit(&mut dst, &src, Rect { x: 2, y: 2, w: 20, h: 20 ,
            }, -1, 6,
        );
        assert_eq!(at(&dst, 0, 6), 10);
        assert_eq!(at(&dst, 2, 8), 20);
        assert_eq!(at(&dst, 4, 9), 20);
        assert_eq!(at(&dst, 5, 9), 255);
        assert_eq!(at(&dst, 6, 6), 255);
        assert_eq!(at(&dst, 0, 5), 255);
    }

    #[test]
    fn rounded_rect_leaves_its_corner_and_keeps_its_middle() {
        let mut c = canvas(40, 40);
        canvas_round_rect(&mut c, Rect { x: 5, y: 5, w: 30, h: 30 ,
            }, 10, 0, true, 0,
        );
        assert_eq!(at(&c, 5, 5), 255);
        assert_eq!(at(&c, 20, 20), 0);
        assert_eq!(at(&c, 20, 5), 0);
        // a radius past half the side is cut to it, a negative one to none
        let mut d = canvas(40, 40);
        canvas_round_rect(&mut d, Rect { x: 5, y: 5, w: 30, h: 30 ,
            }, -4, 0, true, 0,
        );
        assert_eq!(at(&d, 5, 5), 0);
        canvas_round_rect(&mut d, Rect { x: 0, y: 0, w: 0, h: 9 ,
            }, 3, 128, true, 0,
        );
        assert_eq!(at(&d, 0, 0), 255);
    }

    #[test]
    fn outline_of_a_thick_stroke_is_a_fill() {
        let mut c = canvas(10, 10);
        canvas_rect_outline(&mut c, Rect { x: 1, y: 1, w: 6, h: 6 ,
            }, 3, 0,
        );
        assert_eq!(at(&c, 4, 4), 0);
        let mut d = canvas(10, 10);
        canvas_rect_outline(&mut d, Rect { x: 1, y: 1, w: 8, h: 8 ,
            }, 2, 0,
        );
        assert_eq!(at(&d, 1, 1), 0);
        assert_eq!(at(&d, 5, 5), 255);
    }

    #[test]
    fn downsample_rounds_the_box_average() {
        let mut src = canvas(2, 2);
        src.px.copy_from_slice(&[0, 0, 0, 255]);
        let mut dst = canvas(1, 1);
        canvas_downsample(&mut dst, &src);
        assert_eq!(dst.px[0], ((255 + 2) / 4) as u8);
        // the keep-lines version pulls the average toward the darkest pixel
        let mut keep = canvas(1, 1);
        canvas_downsample_keep_lines(&mut keep, &src, 1.0);
        assert_eq!(keep.px[0], 0);
        canvas_downsample_keep_lines(&mut keep, &src, 0.0);
        assert_eq!(keep.px[0], 64);
    }

    #[test]
    fn deposit_only_darkens_and_restore_only_lightens() {
        let mut c = canvas(4, 4);
        c.px[0] = 100;
        canvas_deposit_px(&mut c, 0, 0, 200, 1.0);
        assert_eq!(c.px[0], 100);
        canvas_deposit_px(&mut c, 0, 0, 0, 0.5);
        assert_eq!(c.px[0], 50);
        canvas_restore_px(&mut c, 0, 0, 40, 1.0);
        assert_eq!(c.px[0], 50);
        canvas_restore_px(&mut c, 0, 0, 250, 0.5);
        assert_eq!(c.px[0], 150);
        canvas_restore_px(&mut c, 0, 0, 250, 0.999);
        assert_eq!(c.px[0], 250);
        // outside the clip nothing happens
        canvas_set_clip(&mut c, Rect { x: 1, y: 1, w: 1, h: 1 ,
            },
        );
        canvas_deposit_px(&mut c, 0, 0, 0, 1.0);
        assert_eq!(c.px[0], 250);
    }

    // The run lengths along one row or column of the canvas, from x0 to x1 (exclusive): (dark?, length).
    fn runs(c: &Canvas, x0: i32, y0: i32, dx: i32, dy: i32, n: i32) -> Vec<(bool, i32)> {
        let mut out: Vec<(bool, i32)> = Vec::new();
        for i in 0..n {
            let dark = at(c, x0 + i * dx, y0 + i * dy) == 0;
            match out.last_mut() {
                Some(l) if l.0 == dark => l.1 += 1,
                _ => out.push((dark, 1)),
            }
        }
        out
    }

    #[test]
    fn a_dashed_rect_has_its_dashes_and_gaps_and_solid_corners() {
        // 5 dashes of 4 and 4 gaps of 3 make exactly 32 px: every edge of a 32 x 32 box is that.
        let mut c = canvas(60, 60);
        let r = Rect { x: 10, y: 10, w: 32, h: 32 ,
        };
        canvas_dash_rect(&mut c, r, 2, 4, 3, 2);
        let want = vec![(true, 4), (false, 3), (true, 4), (false, 3), (true, 4), (false, 3), (true, 4), (false, 3), (true, 4),
        ];
        assert_eq!(runs(&c, r.x, r.y, 1, 0, 32), want); // the top edge's first row
        assert_eq!(runs(&c, r.x, r.y + r.h - 1, 1, 0, 32), want); // the bottom edge's last row
        assert_eq!(runs(&c, r.x, r.y, 0, 1, 32), want); // the left edge
        assert_eq!(runs(&c, r.x + r.w - 1, r.y, 0, 1, 32), want); // the right edge
        // The dashes are 2 px thick: the second row of the top edge is the same, the third is not on.
        assert_eq!(runs(&c, r.x, r.y + 1, 1, 0, 32), want);
        assert_eq!(at(&c, r.x + 5, r.y + 2), 255);
        // The four corners are solid (2 x 2 each).
        for (cx, cy) in [(r.x, r.y), (r.x + r.w - 2, r.y), (r.x, r.y + r.h - 2), (r.x + r.w - 2, r.y + r.h - 2),
        ] {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert_eq!(at(&c, cx + dx, cy + dy), 0);
            }
        }
        // The halo: 2 px of white on both sides of an edge's gap. (Over a dark page it shows.)
        let mut d = canvas(60, 60);
        canvas_fill(&mut d, Rect { x: 0, y: 0, w: 60, h: 60 ,
            }, 100,
        );
        canvas_dash_rect(&mut d, r, 2, 4, 3, 2);
        assert_eq!(at(&d, r.x + 5, r.y - 1), 255);
        assert_eq!(at(&d, r.x + 5, r.y - 2), 255);
        assert_eq!(at(&d, r.x + 5, r.y - 3), 100);
        assert_eq!(at(&d, r.x + 5, r.y + 2), 255); // inside: the halo
        assert_eq!(at(&d, r.x + 5, r.y + 3), 255);
        assert_eq!(at(&d, r.x + 5, r.y + 4), 100);
        assert_eq!(at(&d, r.x + 5, r.y), 255); // the gap itself is white
        assert_eq!(at(&d, r.x + 1, r.y), 0); // the dash is black
        // Only 0 and 255 land on the page where the dashes are.
        assert!(d.px.iter().all(|&p| p == 0 || p == 100 || p == 255));
        // An edge that does not fit whole dashes and gaps is stretched to: both ends are still dashes.
        let mut e = canvas(120, 40);
        let r2 = Rect { x: 5, y: 5, w: 101, h: 30 ,
        };
        canvas_dash_rect(&mut e, r2, 2, 16, 12, 0);
        let top = runs(&e, r2.x, r2.y, 1, 0, 101);
        assert_eq!(top.first(), Some(&(true, 16)));
        assert_eq!(top.last(), Some(&(true, 16)));
        assert!(top.iter().filter(|r| r.0).all(|r| r.1 == 16));
        assert!(top.iter().filter(|r| !r.0).all(|r| (10..=14).contains(&r.1)));
        // A box too small for two dashes is a solid outline.
        let mut f = canvas(40, 40);
        canvas_dash_rect(&mut f, Rect { x: 4, y: 4, w: 20, h: 20 ,
            }, 2, 16, 12, 0,
        );
        assert_eq!(runs(&f, 4, 4, 1, 0, 20), vec![(true, 20)]);
        assert_eq!(runs(&f, 4, 4, 0, 1, 20), vec![(true, 20)]);
    }

    #[test]
    fn a_dashed_rect_is_clipped() {
        let mut c = canvas(50, 50);
        canvas_set_clip(&mut c, Rect { x: 20, y: 20, w: 10, h: 10 ,
            },
        );
        canvas_dash_rect(&mut c, Rect { x: 5, y: 5, w: 40, h: 40 ,
            }, 2, 4, 3, 2,
        );
        // Nothing outside the clip changed: the screen is still all white and nothing dark is there.
        assert!(c.px.iter().all(|&p| p == 255));
        let mut d = canvas(50, 50);
        canvas_set_clip(&mut d, Rect { x: 0, y: 0, w: 12, h: 50 ,
            },
        );
        canvas_dash_rect(&mut d, Rect { x: 5, y: 5, w: 40, h: 40 ,
            }, 2, 4, 3, 2,
        );
        for y in 0..50 {
            for x in 12..50 {
                assert_eq!(at(&d, x, y), 255);
            }
        }
        assert_eq!(at(&d, 5, 5), 0);
        assert_eq!(at(&d, 5, 44), 0);
        // A rect with no area draws nothing.
        let mut e = canvas(10, 10);
        canvas_dash_rect(&mut e, RECT_EMPTY, 2, 4, 3, 2);
        assert!(e.px.iter().all(|&p| p == 255));
    }

    #[test]
    fn invert_and_lines() {
        let mut c = canvas(6, 6);
        canvas_fill(&mut c, Rect { x: 0, y: 0, w: 3, h: 6 ,
            }, 30,
        );
        canvas_invert(&mut c, Rect { x: 1, y: 1, w: 4, h: 2 ,
            },
        );
        assert_eq!(at(&c, 1, 1), 225);
        assert_eq!(at(&c, 4, 2), 0);
        assert_eq!(at(&c, 0, 1), 30);
        canvas_hline(&mut c, 5, 1, 4, 1, 9);
        assert_eq!(at(&c, 1, 4), 9);
        assert_eq!(at(&c, 4, 4), 9);
        assert_eq!(at(&c, 5, 4), 255);
        canvas_vline(&mut c, 5, 5, 0, 1, 8);
        assert_eq!(at(&c, 5, 0), 8);
        assert_eq!(at(&c, 5, 4), 8);
        assert_eq!(at(&c, 5, 5), 255);
    }

    // canvas_dash_poly: the pattern measured along a path with a corner; a path drawn in two pieces with the
    // returned phase is the whole path pixel for pixel; the clip; only 0 and 255.
    #[test]
    fn a_dashed_polyline_keeps_its_pattern_across_corners_and_pieces() {
        // A 1 px pen: along the top row x 10..29 the samples t 0..19 ink where t mod 7 < 4, then down x 30
        // the pattern goes on from 20.
        let mut c = canvas(60, 60);
        let ph = canvas_dash_poly(&mut c, &[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)], 1, 4, 3, 0, 0,
        );
        assert_eq!(ph, 40 % 7);
        for t in 0..20 {
            assert_eq!(at(&c, 10 + t, 10) == 0, t % 7 < 4, "top t {t}");
            assert_eq!(at(&c, 30, 10 + t) == 0, (20 + t) % 7 < 4, "down t {t}");
        }
        assert_eq!(at(&c, 30, 30), 255); // the final point is the next piece's
        // A 2 px pen: the dashes along a straight run are exactly 4 long and the gaps exactly 3.
        let mut d = canvas(80, 20);
        canvas_dash_poly(&mut d, &[(10.0, 10.0), (60.0, 10.0)], 2, 4, 3, 0, 0);
        let row = runs(&d, 9, 10, 1, 0, 49);
        assert_eq!(&row[..6], &[(true, 4), (false, 3), (true, 4), (false, 3), (true, 4), (false, 3)]);
        assert_eq!(runs(&d, 9, 9, 1, 0, 49), row); // two rows thick
        assert_eq!(at(&d, 10, 11), 255);
        // The phase starts the pattern part way in.
        let mut e = canvas(40, 10);
        canvas_dash_poly(&mut e, &[(5.0, 5.0), (30.0, 5.0)], 1, 4, 3, 0, 5);
        assert_eq!(at(&e, 5, 5), 255); // 5 and 6 are the gap
        assert_eq!(at(&e, 6, 5), 255);
        assert_eq!(at(&e, 7, 5), 0); // 7 = a new dash
        // A ring of float points, whole and in two pieces, at the lasso's sizes.
        let ring: Vec<(f32, f32)> = (0..=60)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 60.0;
                (100.0 + 70.3 * a.cos(), 100.0 + 55.7 * a.sin())
            })
            .collect();
        let mut whole = canvas(200, 200);
        let pw = canvas_dash_poly(&mut whole, &ring, 4, 12, 8, 0, 3);
        let mut parts = canvas(200, 200);
        let p1 = canvas_dash_poly(&mut parts, &ring[..=23], 4, 12, 8, 0, 3);
        let p2 = canvas_dash_poly(&mut parts, &ring[23..], 4, 12, 8, 0, p1);
        assert_eq!(pw, p2);
        assert!(whole.px == parts.px);
        assert!(whole.px.contains(&0) && whole.px.iter().all(|&p| p == 0 || p == 255));
        // With a halo over a gray page: only the page's gray, 0 and 255.
        let mut g = canvas(200, 200);
        canvas_fill(&mut g, Rect { x: 0, y: 0, w: 200, h: 200 ,
            }, 100,
        );
        canvas_dash_poly(&mut g, &ring, 4, 12, 8, 2, 0);
        assert!(g.px.iter().all(|&p| p == 0 || p == 100 || p == 255));
        assert!(g.px.contains(&255) && g.px.contains(&0));
        // The clip: nothing lands outside it.
        let mut k = canvas(200, 200);
        canvas_set_clip(&mut k, Rect { x: 0, y: 0, w: 100, h: 200 ,
            },
        );
        canvas_dash_poly(&mut k, &ring, 4, 12, 8, 2, 0);
        for y in 0..200 {
            for x in 100..200 {
                assert_eq!(at(&k, x, y), 255);
            }
        }
        assert!(k.px.contains(&0));
        // Fewer than two points, or no pen: nothing drawn, the phase kept (and wrapped).
        let mut n = canvas(20, 20);
        assert_eq!(canvas_dash_poly(&mut n, &[(5.0, 5.0)], 2, 4, 3, 0, 9), 2);
        assert_eq!(canvas_dash_poly(&mut n, &[(5.0, 5.0), (15.0, 5.0)], 0, 4, 3, 0, 0), 3);
        assert_eq!(canvas_dash_poly(&mut n, &[], 2, 4, 3, 0, -1), 6);
        assert!(n.px.iter().all(|&p| p == 255));
    }
}

#[cfg(test)]
mod slash_tests {
    use super::*;
    #[test]
    fn slash_stays_in_its_rect_and_clip_and_has_binary_pixels() {
        let mut c = Canvas::default(); assert_eq!(canvas_init(&mut c, 48, 48), 0);
        let r = Rect { x: 8, y: 8, w: 32, h: 32 };
        canvas_slash(&mut c, r, 4);
        assert_eq!(c.px[(24 * c.stride + 24) as usize], 0);
        assert_eq!(c.px[(12 * c.stride + 12) as usize], 255);
        assert!(c.px.iter().all(|&p| p == 0 || p == 255));
        let black = c.px.iter().filter(|&&p| p == 0).count();
        assert!(black > 120 && black < 200);
        for y in 0..c.h { for x in 0..c.w {
            if !rect_contains(r, x, y) { assert_eq!(c.px[(y*c.stride+x) as usize], 255); }
        }}
        canvas_fill(&mut c, Rect { x:0, y:0, w:48, h:48 }, 255);
        canvas_set_clip(&mut c, Rect { x:0, y:0, w:24, h:48 });
        canvas_slash(&mut c, r, 4);
        for y in 0..48 { for x in 24..48 { assert_eq!(c.px[(y*48+x) as usize], 255); }}
    }
}

#[cfg(test)]
mod map_primitive_tests {
    use super::*;
    #[test]
    fn line_row_spans_match_the_old_coverage_pixel_for_pixel() {
        let mut state = 12345u32;
        let mut next = || { state = state.wrapping_mul(1664525).wrapping_add(1013904223); state as f32 / u32::MAX as f32 };
        for _ in 0..300 {
            let mut a = Canvas::default(); canvas_init(&mut a, 240, 240);
            for (i, pixel) in a.px.iter_mut().enumerate() { *pixel = (i % 256) as u8; }
            canvas_set_clip(&mut a, Rect { x: 10, y: 8, w: 220, h: 222 });
            let mut b = a.clone();
            let (x0,y0,x1,y1,w) = (next()*320.0-40.0,next()*320.0-40.0,next()*320.0-40.0,next()*320.0-40.0,next()*12.0+0.1);
            canvas_line_aa(&mut a, x0,y0,x1,y1,w,31);
            canvas_line_aa_reference(&mut b, x0,y0,x1,y1,w,31);
            assert_eq!(a.px,b.px,"line {x0},{y0} -> {x1},{y1}, width {w}");
        }
    }
    #[test]
    fn bayer_discs_have_the_requested_binary_coverage() {
        for level in 0..=4 {
            let mut c = Canvas::default(); canvas_init(&mut c, 160,160);
            canvas_disc_pattern(&mut c,80.0,80.0,64.0,level);
            assert!(c.px.iter().all(|&p| p==0 || p==255));
            let black = c.px.iter().filter(|&&p| p==0).count() as f32;
            let area = std::f32::consts::PI*64.0*64.0;
            assert!((black/area-level as f32*0.25).abs()<1.0/16.0);
            assert_eq!(c.px[0],255);
        }
    }
}
