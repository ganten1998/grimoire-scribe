//! keyboard.h: on-screen keyboard modal for naming notebooks and folders.
//! QWERTY + numbers row, shift, space, backspace, Cancel / Done, and a key to a page of accented
//! letters (and back). Pen or finger taps. The text is UTF-8: a key types a whole character and
//! backspace takes a whole character away.

use crate::prelude::*;

pub const KB_TEXT_MAX: usize = 96;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KbResult {
    #[default]
    None = 0,  // KB_NONE
    Changed = 1,  // KB_CHANGED
    Done = 2,  // KB_DONE
    Cancel = 3,  // KB_CANCEL
}

/// Key codes: a character key's code is its Unicode code point as typed without shift; the
/// other keys have codes below 32.
pub const KB_KEY_SHIFT: u32 = 1;
pub const KB_KEY_PAGE: u32 = 2;
pub const KB_KEY_BKSP: u32 = 8;
pub const KB_KEY_DONE: u32 = 13;
pub const KB_KEY_CANCEL: u32 = 27;

#[derive(Clone, Copy, Debug, Default)]
pub struct Keyboard {
    pub open: bool,
    pub title: FixedStr<64>,
    pub text: FixedStr<KB_TEXT_MAX>,
    pub shift: bool,
    /// 0 letters, 1 accents, 2 number pad
    pub page: i32,
    /// where it was last drawn (whole modal)
    pub area: Rect,
    pub press: Press,
    /// -1 if none
    pub pressed_key: i32,
    /// the pressed key already typed on touch-down
    pub typed: bool,
    /// set by keyboard_pointer when the text field changed (refresh it first)
    pub field_dirty: Rect,
    /// when a pause in typing gets the keys their grayscale refresh
    /// (app_keyboard_tick), 0 for none due
    pub settle_at: u32,
}

/// Starts on the letters page with initial as the text: every valid character is kept (accents
/// included); control characters and broken UTF-8 are dropped, and the text is cut, if it must
/// be, between characters.
pub fn keyboard_open(kb: &mut Keyboard, title: &str, initial: &[u8]) {
    todo!()
}
pub fn keyboard_close(kb: &mut Keyboard) {
    todo!()
}
/// Draws the modal (text field at top, keys below) anchored to the bottom of screen.
/// Returns the rect it covered (to invalidate).
pub fn keyboard_draw(kb: &mut Keyboard, c: &mut Canvas, screen: Rect) -> Rect {
    todo!()
}
/// How many times keyboard_draw drew a keyboard since this was last called (the headless runner's
/// keyboard.draws check, docs/SIM.md): every draw of the whole modal is work on the main thread, and a
/// key press should cost one.
pub fn keyboard_draws_take() -> u32 {
    todo!()
}
/// Pointer events only (pen or touch, already unified by the caller as down/move/up).
/// *dirty receives the rect needing redraw (may be empty). Characters and backspace type on
/// touch-down (no waiting for the lift); Shift, the page key, Cancel and Done act on release.
/// phase: 0 down, 1 move, 2 up.
pub fn keyboard_pointer(kb: &mut Keyboard, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> KbResult {
    todo!()
}
/// Where the key with this code is on the page showing now (as last drawn); false if that page
/// has no such key. For tests and scripted runs.
pub fn keyboard_key_rect(kb: &Keyboard, code: u32, r: &mut Rect) -> bool {
    todo!()
}
/// The capital shift types for a letter (Latin-1 and Latin Extended-A); anything without one
/// comes back as it is.
pub fn keyboard_upper(cp: u32) -> u32 {
    todo!()
}
