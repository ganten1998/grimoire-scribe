//! editor.c: the notebook editor. Live ink, stroke eraser, the floating panel, page
//! navigation, paper changes, autosave.
//!
//! Pixels: page_canvas holds paper + ink at page resolution: the notebook's page size, the screen
//! it was made on (a notebook from another Scribe keeps its own). The view maps page -> screen
//! (zoom + pan): the page's width fits the screen's (100% upright for a notebook made on this
//! screen), pinch to zoom up to 4x that. a->screen = the viewed page + panel (+ paper sheet).
//! compose(r) rebuilds a screen rect from those layers; show(page rect) is the page-side entry.

use crate::prelude::*;

/// The screen's state (editor.c's, which app.h keeps opaque): editor_init makes it, in a.ed.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct EditorState {}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn editor_init(a: &mut App) {
    todo!()
}
pub fn editor_free(a: &mut App) {
    todo!()
}
pub fn editor_enter(a: &mut App) {
    todo!()
}
pub fn editor_leave(a: &mut App) {
    todo!()
}
pub fn editor_event(a: &mut App, ev: &InEvent) {
    todo!()
}
pub fn editor_tick(a: &mut App, now: u32) {
    todo!()
}

// ---- cross-screen entry points (app.h) ----

/// loads, switches to SCR_EDITOR
pub fn editor_open_notebook(a: &mut App, id: &str) {
    todo!()
}
/// the same, at that page
pub fn editor_open_notebook_at(a: &mut App, id: &str, page_id: u32, added: bool) {
    todo!()
}
/// a picture job is done: the open page drawn again
pub fn editor_picture_made(a: &mut App) {
    todo!()
}
pub fn editor_save_now(a: &mut App) -> bool {
    todo!()
}
/// flush pending page/info saves; false if they failed
pub fn editor_suspend(a: &mut App) -> bool {
    todo!()
}
/// optional key/button action, idle editor only
pub fn editor_shortcut(a: &mut App, action: i32) -> bool {
    todo!()
}
/// Erase while held's button let go (false)
pub fn editor_hold_eraser(a: &mut App, held: bool) {
    todo!()
}
/// a stroke / drag / pinch is in progress
pub fn editor_busy(a: &App) -> bool {
    todo!()
}
/// the open page (paper + ink), or None
pub fn editor_page_canvas(a: &App) -> Option<&Canvas> {
    todo!()
}
/// saves the open page as the sleep screen
pub fn editor_pin_sleep_page(a: &mut App) -> bool {
    todo!()
}
/// The headless runner's "editor.*" keys, asked without the prefix (app_probe's contract);
/// "templates.*" keys reach it whole (docs/SIM.md).
pub fn editor_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
