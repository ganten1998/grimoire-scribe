//! font.h: TrueType text via stb_truetype, fonts embedded at build time
//! (build/gen/fonts.c, from assets/*.ttf via xxd -i). Glyph bitmaps are cached per size.

use crate::prelude::*;

/// What the text is for; the theme picks the typeface. Classic: all Terminess. Gothic: pixel
/// faces, Jersey 10 for text and labels (overprinted one design pixel for bold, except where
/// that would fill in an accent or turn a letter into another), Jacquard 24 (pixel blackletter)
/// for titles. FIXED faces are Terminess in every theme: paper labels.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFace {
    #[default]
    Regular = 0,  // FONT_REGULAR
    Bold = 1,  // FONT_BOLD
    Title = 2,  // FONT_TITLE
    Caps = 3,  // FONT_CAPS
    Fixed = 4,  // FONT_FIXED
    FixedBold = 5,  // FONT_FIXED_BOLD
}
impl FontFace {
    /// FONT_FACE_COUNT
    pub const COUNT: usize = 6;
}

/// 0 on success
pub fn font_init() -> i32 {
    todo!()
}
pub fn font_shutdown() {
    todo!()
}
pub fn font_set_gothic(gothic: bool) {
    todo!()
}
pub fn font_gothic() -> bool {
    todo!()
}

/// Draws UTF-8 text with its baseline at y. Returns advance width in px.
pub fn font_draw(c: &mut Canvas, f: FontFace, px: f32, x: i32, baseline_y: i32, utf8: &str, gray: u8) -> i32 {
    todo!()
}
pub fn font_measure(f: FontFace, px: f32, utf8: &str) -> i32 {
    todo!()
}
/// Whether the face (as the current theme maps it) has its own glyph for cp, not Terminess's.
pub fn font_has_glyph(f: FontFace, cp: u32) -> bool {
    todo!()
}
/// utf8 without the characters no face draws (every face takes what it lacks from Terminess, so one
/// Terminess lacks too would show as a box: Chinese, Japanese, Korean, most Vietnamese letters), and
/// without the spaces that leaves doubled, first or last. Returns how many characters it left out:
/// 0, and out is utf8 as it is (cut to cap between characters). Before font_init, none.
pub fn font_drawable_copy(utf8: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
/// px above baseline
pub fn font_ascent(f: FontFace, px: f32) -> i32 {
    todo!()
}
/// px below baseline (positive)
pub fn font_descent(f: FontFace, px: f32) -> i32 {
    todo!()
}
/// Centered in r both ways; text longer than r is truncated with an ellipsis.
pub fn font_draw_centered(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    todo!()
}
/// Draws one glyph (e.g. a Nerd Font icon code point) scaled so its ink box fits r, centered.
pub fn font_draw_icon(c: &mut Canvas, f: FontFace, codepoint: u32, r: Rect, gray: u8) {
    todo!()
}
/// Left-aligned, vertically centered in r, truncated with ellipsis.
pub fn font_draw_left(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    todo!()
}
/// The same for a name someone gave (a notebook's, a folder's), which "..." shortens by design:
/// the overflow hook (below) does not hear of it.
pub fn font_draw_left_name(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    todo!()
}

/// Up to max_lines lines broken at spaces (a word wider than r on its own is broken between its
/// characters), each centered in r or left-aligned, in slots line_h px apart (0: the face's own
/// height) with the block centered vertically in r. Text that fits on one line is drawn exactly as
/// font_draw_centered or font_draw_left draws it. Two lines are balanced: the break that keeps the
/// longer line shortest, unless one right after punctuation makes it at most half again as long
/// ("Wide, no margin" breaks after its comma). Only text that still does not fit ends in "...".
/// Returns the lines drawn.
pub fn font_draw_wrapped(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, max_lines: i32, line_h: i32, center: bool, gray: u8) -> i32 {
    todo!()
}
/// How many lines font_draw_wrapped needs for the text in width w (it draws at most its
/// max_lines), and the width of the widest of them in *widest (may be NULL).
pub fn font_wrap_lines(f: FontFace, px: f32, utf8: &str, w: i32, widest: Option<&mut i32>) -> i32 {
    todo!()
}
/// The largest size from px down to min_px at which the text fits maxw on one line, among the
/// sizes the face really has: a pixel face changes only at whole design pixels (Jersey 10 looks the
/// same from 27 to 43 px), an outline face in quarter pixels. px if it fits as it is; 0 if even
/// the smallest does not.
pub fn font_fit_px(f: FontFace, px: f32, min_px: f32, utf8: &str, maxw: i32) -> f32 {
    todo!()
}
/// Whenever text is cut short to fit (font_draw_centered, font_draw_left, font_draw_wrapped), or
/// font_draw_wrapped breaks a word between its letters, the hook hears of it: the text as given,
/// the width it needs (for wrapped text, its last line's, or the broken word's) and the width it
/// has, in px. NULL takes it away. The headless runner's fit check uses it.
pub type FontOverflowHook = fn(utf8: &str, need: i32, have: i32);
pub fn font_set_overflow_hook(hook: Option<FontOverflowHook>) {
    todo!()
}
