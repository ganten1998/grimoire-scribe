//! fbpack.h: copy one logical grayscale row into an 8/32-bit framebuffer, in either direction.
//! The caller bounds the destination and optional previous-frame row; strides are byte/pixel
//! steps and can be negative. Previous holds logical grays, before inversion or live dithering.

use crate::prelude::*;

/// The 8x8 ordered dither's thresholds (live ink frames are bilevel in DU mode).
static K_BAYER8: [u8; 64] = [
    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26,
    12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25,
    15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
];

/// In Rust the rows are slices and offsets: pixel i of the destination is dst[dst_at + i * step]
/// (step in bytes, negative for a mirrored framebuffer), of previous previous[previous_at + i *
/// previous_step]; None for no previous frame. Out-of-range offsets panic (a bounds check per
/// pixel, which the C leaves to its caller).
pub fn fbpack_row(dst: &mut [u8], dst_at: usize, step: isize, previous: Option<&mut [u8]>, previous_at: usize, previous_step: isize, src: &[u8], width: i32, x: i32, y: i32, bpp: i32, invert: bool, fast: bool, known: bool) {
    if width <= 0 || (bpp != 8 && bpp != 32) {
        return;
    }
    let width = width as usize;
    let mut previous = previous;
    if !fast && !invert && bpp == 8 && step == 1 {
        dst[dst_at..dst_at + width].copy_from_slice(&src[..width]);
        if let Some(pv) = previous.as_deref_mut() {
            if previous_step == 1 {
                pv[previous_at..previous_at + width].copy_from_slice(&src[..width]);
            } else {
                for i in 0..width {
                    pv[(previous_at as isize + i as isize * previous_step) as usize] = src[i];
                }
            }
        }
        return;
    }
    for i in 0..width {
        // (the C's uint8_t v, used as an int in every expression below)
        let mut v = src[i] as i32;
        let p = (dst_at as isize + i as isize * step) as usize;
        let pi = (previous_at as isize + i as isize * previous_step) as usize;
        // DU leaves gray target pixels alone. Keep unchanged gray paper gray, and keep earlier
        // dithered ink in its current state, instead of dithering the whole dirty rectangle.
        if fast && known {
            if let Some(pv) = previous.as_deref() {
                if pv[pi] as i32 == v {
                    continue;
                }
            }
        }
        if let Some(pv) = previous.as_deref_mut() {
            pv[pi] = v as u8;
        }
        if fast {
            let threshold = K_BAYER8[(((y & 7) << 3) | ((x + i as i32) & 7)) as usize] as i32 * 4 + 2;
            if known && previous.is_some() {
                // A DU transition ends in black or white, and a pixel cannot start another until
                // it has. So move what the panel shows only toward the new value: black or not
                // yet when it darkens, white or not yet when it lightens, as often as keeps the
                // average. Ink on gray paper then goes straight to black, instead of white first
                // and black a whole waveform later. Over black and white this is the dither below.
                let g = if invert { 255 - dst[p] as i32 } else { dst[p] as i32 };
                if v < g {
                    if v * 255 > threshold * g {
                        continue;
                    }
                    v = 0;
                } else if v > g {
                    if (v - g) * 255 <= threshold * (255 - g) {
                        continue;
                    }
                    v = 255;
                } else {
                    continue;
                }
            } else if v != 0 && v != 255 {
                v = if v > threshold { 255 } else { 0 };
            }
        }
        if invert {
            v = 255 - v;
        }
        dst[p] = v as u8;
        if bpp == 32 {
            dst[p + 1] = v as u8;
            dst[p + 2] = v as u8;
            dst[p + 3] = 255;
        }
    }
}

/// Odd-rotation MTK pen updates below 6000 pixels can bypass coordinate processing. Expand the
/// refresh, without changing its pixels, to at least 6400 pixels while staying inside the panel.
pub fn fbpack_pen_region(r: Rect, width: i32, height: i32) -> Rect {
    if width <= 0 || height <= 0 {
        return RECT_EMPTY;
    }
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: width, h: height });
    if rect_is_empty(r) || (r.w as i64) * (r.h as i64) >= 6400 {
        return r;
    }
    let mut w = if r.h >= 80 { (6400 + r.h - 1) / r.h } else { 80 };
    w = clampi(w, r.w, width);
    let h = clampi((6400 + w - 1) / w, r.h, height);
    // Narrow panels may need more width after the height reaches their edge.
    if (w as i64) * (h as i64) < 6400 {
        w = clampi((6400 + h - 1) / h, w, width);
    }
    let x = clampi(r.x - (w - r.w) / 2, 0, width - w);
    let y = clampi(r.y - (h - r.h) / 2, 0, height - h);
    Rect { x, y, w, h }
}
