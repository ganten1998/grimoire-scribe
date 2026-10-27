//! qr.h: QR codes on the screen (the Share screen's address, for a phone camera), drawn the way
//! e-ink shows best: whole-pixel square modules in pure black and white.

use crate::prelude::*;

/// Modules across the symbol for text, its quiet zone included (37 for every share address), or
/// 0 when the text cannot be encoded. A box of that many times m pixels gets modules of m pixels.
pub fn ui_qr_modules(text: &str) -> i32 {
    todo!()
}
/// Draws text as a QR code centered in box: square modules of the largest whole number of pixels
/// that fits, inside a quiet zone of 4 light modules. negative draws the exact inverse, for the
/// Invert setting (it inverts the panel at display time, so the code still shows dark on light).
/// Returns the symbol's rect, quiet zone included, or RECT_EMPTY (nothing drawn) when the text
/// cannot be encoded or the box is too small.
pub fn ui_qr(c: &mut Canvas, box_: Rect, text: &str, negative: bool) -> Rect {
    todo!()
}
