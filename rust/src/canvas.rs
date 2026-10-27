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
    todo!()
}
/// Rounded rect, filled or outlined (thickness t), antialiased corners.
pub fn canvas_round_rect(c: &mut Canvas, r: Rect, radius: i32, gray: u8, filled: bool, t: i32) {
    todo!()
}
/// Rect with 45-degree cut corners (ch px), same filled/outline rules as canvas_round_rect.
pub fn canvas_chamfer_rect(c: &mut Canvas, r: Rect, ch: i32, gray: u8, filled: bool, t: i32) {
    todo!()
}
/// Filled diamond of "radius" r (center to tip).
pub fn canvas_lozenge(c: &mut Canvas, cx: f32, cy: f32, r: f32, gray: u8) {
    todo!()
}
/// Axis-aligned crisp lines (used by paper rules). Thickness grows down/right.
pub fn canvas_hline(c: &mut Canvas, x0: i32, x1: i32, y: i32, t: i32, gray: u8) {
    todo!()
}
pub fn canvas_vline(c: &mut Canvas, x: i32, y0: i32, y1: i32, t: i32, gray: u8) {
    todo!()
}
/// Antialiased line with round caps; width in px (may be fractional). Opaque paint (not deposit).
pub fn canvas_line_aa(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: u8) {
    todo!()
}
/// Antialiased filled disc painted toward gray with alpha in 0..1 (normal "over" blend).
pub fn canvas_disc_aa(c: &mut Canvas, cx: f32, cy: f32, r: f32, gray: u8, alpha: f32) {
    todo!()
}

/// Ink deposit: darken-only blend. dst = dst - (dst - ink) * a, only if that is darker.
/// (clang contracts `*p - (*p - ink) * a` into one fused multiply-add, as its default
/// -ffp-contract=on allows, so this is written as mul_add: PORTING.md, Floating point.)
#[inline]
pub fn canvas_deposit_px(c: &mut Canvas, x: i32, y: i32, ink: u8, a: f32) {
    if x < c.clip.x || y < c.clip.y || x >= c.clip.x + c.clip.w || y >= c.clip.y + c.clip.h {
        return;
    }
    let p = &mut c.px[(y * c.stride + x) as usize];
    if *p <= ink {
        return;
    }
    let v = ((-((*p - ink) as f32)).mul_add(a, *p as f32) + 0.5) as i32;
    if v < *p as i32 {
        *p = v as u8;
    }
}

/// Paper restore, the deposit's mirror (erase strokes): lighten-only toward the paper value under
/// the pixel, never past it. dst = dst + (paper - dst) * a, only if that is lighter; a >= 0.999
/// lands exactly on the paper.
/// (`*p + (paper - *p) * a` is one fused multiply-add in the C build, as above.)
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
    let v = (((paper - *p) as f32).mul_add(a, *p as f32) + 0.5) as i32;
    if v > *p as i32 {
        *p = if v < paper as i32 { v as u8 } else { paper };
    }
}

/// Copy src rect to dst at (dx,dy), clipped by dst clip.
pub fn canvas_blit(dst: &mut Canvas, src: &Canvas, srect: Rect, dx: i32, dy: i32) {
    todo!()
}
/// Box-filter downsample whole src into dst (dst size defines the scale).
pub fn canvas_downsample(dst: &mut Canvas, src: &Canvas) {
    todo!()
}
/// Downsample keeping thin dark lines visible: each output pixel blends the box average
/// toward the box minimum (weight 0..1). Used for paper previews.
pub fn canvas_downsample_keep_lines(dst: &mut Canvas, src: &Canvas, weight: f32) {
    todo!()
}
/// Scales src into box keeping its aspect (box filter down, nearest up), centered in box.
pub fn canvas_blit_fit(dst: &mut Canvas, src: &Canvas, box_: Rect) {
    todo!()
}
/// Invert a rect (used for pressed/selected feedback).
pub fn canvas_invert(c: &mut Canvas, r: Rect) {
    todo!()
}
