//! pensettings.c: per-tool pressure / tilt sensitivity. Tabs per tool, one slider per
//! tweakable parameter, a live pressure curve, a raw-vs-mapped readout, and a test pad that
//! draws with the tool being tuned.

use crate::prelude::*;

/// The screen's state (pensettings.c's, which app.h keeps opaque): pensettings_init makes it, in a.ps.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct PenSetState {}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn pensettings_init(a: &mut App) {
    todo!()
}
pub fn pensettings_free(a: &mut App) {
    todo!()
}
pub fn pensettings_enter(a: &mut App) {
    todo!()
}
pub fn pensettings_leave(a: &mut App) {
    todo!()
}
pub fn pensettings_event(a: &mut App, ev: &InEvent) {
    todo!()
}
pub fn pensettings_tick(a: &mut App, now: u32) {
    todo!()
}

// ---- cross-screen entry points (app.h) ----

/// switch to SCR_PENSET (returns to prev_scr)
pub fn pensettings_open(a: &mut App, tab: ToolKind) {
    todo!()
}
/// Pen feel's own button and Close menu: Back
pub fn pensettings_shortcut(a: &mut App, action: i32) -> bool {
    todo!()
}
/// "penset.*": the pen feel pad (app_probe's contract, without the prefix)
pub fn pensettings_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
