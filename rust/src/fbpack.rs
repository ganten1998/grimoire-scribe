//! fbpack.h: copy one logical grayscale row into an 8/32-bit framebuffer, in either direction.
//! The caller bounds the destination and optional previous-frame row; strides are byte/pixel
//! steps and can be negative. Previous holds logical grays, before inversion or live dithering.

use crate::prelude::*;

/// In Rust the rows are slices and offsets: pixel i of the destination is dst[dst_at + i * step]
/// (step in bytes, negative for a mirrored framebuffer), of previous previous[previous_at + i *
/// previous_step]; None for no previous frame. Out-of-range offsets panic (a bounds check per
/// pixel, which the C leaves to its caller).
pub fn fbpack_row(dst: &mut [u8], dst_at: usize, step: isize, previous: Option<&mut [u8]>, previous_at: usize, previous_step: isize, src: &[u8], width: i32, x: i32, y: i32, bpp: i32, invert: bool, fast: bool, known: bool) {
    todo!()
}

/// Odd-rotation MTK pen updates below 6000 pixels can bypass coordinate processing. Expand the
/// refresh, without changing its pixels, to at least 6400 pixels while staying inside the panel.
pub fn fbpack_pen_region(r: Rect, width: i32, height: i32) -> Rect {
    todo!()
}
