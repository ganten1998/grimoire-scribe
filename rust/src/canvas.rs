//! canvas.h: 8-bit grayscale raster (0 black .. 255 white) with a clip rect.

use crate::prelude::*;

/// `a * b + c` as the C build rounds it. clang's default -ffp-contract=on turns it into one fused
/// multiply-add on arm64, which is the host build the unit tests and the golden renders run on, so
/// there it is `mul_add`. The Kindle's build (zig cc for the Cortex-A7) does not fuse: its ARM
/// backend lowers the contracted form to a separate multiply and add (vmul.f32, vadd.f32; there is
/// no vfma anywhere in the C binary), so on arm it is the plain form, which Rust never fuses. For
/// `c - a * b` pass `-a`, for `a * b - c` pass `-c`: negating is exact, so the unfused form is the
/// same number. (PORTING.md, Floating point, says the Cortex-A7 fuses too; it does not.)
#[inline(always)]
pub(crate) fn fmla(a: f32, b: f32, c: f32) -> f32 {
    if cfg!(target_arch = "arm") {
        a * b + c
    } else {
        a.mul_add(b, c)
    }
}

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
    *c = Canvas { px, w, h, stride: w, clip: Rect { x: 0, y: 0, w, h } };
    0
}
pub fn canvas_free(c: &mut Canvas) {
    *c = Canvas::default();
}
/// intersected with bounds
pub fn canvas_set_clip(c: &mut Canvas, r: Rect) {
    c.clip = rect_intersect(r, Rect { x: 0, y: 0, w: c.w, h: c.h });
}
pub fn canvas_reset_clip(c: &mut Canvas) {
    c.clip = Rect { x: 0, y: 0, w: c.w, h: c.h };
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
    canvas_fill(c, Rect { x: r.x, y: r.y, w: r.w, h: t }, gray);
    canvas_fill(c, Rect { x: r.x, y: r.y + r.h - t, w: r.w, h: t }, gray);
    canvas_fill(c, Rect { x: r.x, y: r.y + t, w: t, h: r.h - 2 * t }, gray);
    canvas_fill(c, Rect { x: r.x + r.w - t, y: r.y + t, w: t, h: r.h - 2 * t }, gray);
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
fn ch_alpha(cx: f32, cy: f32, hw: f32, hh: f32, k: f32, px: f32, py: f32, filled: bool, t: i32) -> f32 {
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
        Rect { x: (cx - r).round_ties_even() as i32, y: (cy - r).round_ties_even() as i32, w: s, h: s },
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
    canvas_fill(c, Rect { x: x0, y, w: x1 - x0, h: t }, gray);
}
pub fn canvas_vline(c: &mut Canvas, x: i32, mut y0: i32, mut y1: i32, t: i32, gray: u8) {
    if y1 < y0 {
        std::mem::swap(&mut y0, &mut y1);
    }
    canvas_fill(c, Rect { x, y: y0, w: t, h: y1 - y0 }, gray);
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
    let s = rect_intersect(srect, Rect { x: 0, y: 0, w: src.w, h: src.h });
    let d = rect_intersect(Rect { x: dx, y: dy, w: s.w, h: s.h }, dst.clip);
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
            // lerpf(avg, mn, weight) is `a + (b - a) * t`: one contracted multiply-add in the C build.
            let v = fmla(mn as f32 - avg, weight, avg);
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
        canvas_set_clip(&mut c, Rect { x: 5, y: 2, w: 100, h: 3 });
        assert_eq!(c.clip, Rect { x: 5, y: 2, w: 15, h: 3 });
        canvas_fill(&mut c, Rect { x: -4, y: 0, w: 200, h: 200 }, 7);
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
        canvas_fill(&mut src, Rect { x: 0, y: 0, w: 8, h: 8 }, 10);
        canvas_fill(&mut src, Rect { x: 4, y: 4, w: 4, h: 4 }, 20);
        let mut dst = canvas(10, 10);
        // the source rect hangs out of the source; the destination hangs out of the canvas
        canvas_blit(&mut dst, &src, Rect { x: 2, y: 2, w: 20, h: 20 }, -1, 6);
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
        canvas_round_rect(&mut c, Rect { x: 5, y: 5, w: 30, h: 30 }, 10, 0, true, 0);
        assert_eq!(at(&c, 5, 5), 255);
        assert_eq!(at(&c, 20, 20), 0);
        assert_eq!(at(&c, 20, 5), 0);
        // a radius past half the side is cut to it, a negative one to none
        let mut d = canvas(40, 40);
        canvas_round_rect(&mut d, Rect { x: 5, y: 5, w: 30, h: 30 }, -4, 0, true, 0);
        assert_eq!(at(&d, 5, 5), 0);
        canvas_round_rect(&mut d, Rect { x: 0, y: 0, w: 0, h: 9 }, 3, 128, true, 0);
        assert_eq!(at(&d, 0, 0), 255);
    }

    #[test]
    fn outline_of_a_thick_stroke_is_a_fill() {
        let mut c = canvas(10, 10);
        canvas_rect_outline(&mut c, Rect { x: 1, y: 1, w: 6, h: 6 }, 3, 0);
        assert_eq!(at(&c, 4, 4), 0);
        let mut d = canvas(10, 10);
        canvas_rect_outline(&mut d, Rect { x: 1, y: 1, w: 8, h: 8 }, 2, 0);
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
        canvas_set_clip(&mut c, Rect { x: 1, y: 1, w: 1, h: 1 });
        canvas_deposit_px(&mut c, 0, 0, 0, 1.0);
        assert_eq!(c.px[0], 250);
    }

    #[test]
    fn invert_and_lines() {
        let mut c = canvas(6, 6);
        canvas_fill(&mut c, Rect { x: 0, y: 0, w: 3, h: 6 }, 30);
        canvas_invert(&mut c, Rect { x: 1, y: 1, w: 4, h: 2 });
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

    #[test]
    fn contracted_multiply_add_is_the_c_builds() {
        // 0.1 * 0.1 + (-0.01) keeps the product's rounding error only when fused
        let (a, b, c) = (0.1f32, 0.1f32, -0.01f32);
        if cfg!(target_arch = "arm") {
            assert_eq!(fmla(a, b, c), a * b + c);
        } else {
            assert_eq!(fmla(a, b, c), a.mul_add(b, c));
            assert_ne!(fmla(a, b, c), a * b + c);
        }
    }
}
