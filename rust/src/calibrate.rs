//! calibrate.c: pen then touch calibration. Targets are matched by ORDER using raw device
//! coordinates, so a badly wrong current mapping cannot get in the way. Buttons are full-width
//! bars that accept the other input (finger while calibrating the pen, pen for touch).

use crate::prelude::*;

/// The screen's state (calibrate.c's, which app.h keeps opaque): calibrate_init makes it, in a.cal.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct CalibState {}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn calibrate_init(a: &mut App) {
    todo!()
}
pub fn calibrate_free(a: &mut App) {
    todo!()
}
pub fn calibrate_enter(a: &mut App) {
    todo!()
}
pub fn calibrate_leave(a: &mut App) {
    todo!()
}
pub fn calibrate_event(a: &mut App, ev: &InEvent) {
    todo!()
}
pub fn calibrate_tick(a: &mut App, now: u32) {
    todo!()
}

// ---- cross-screen entry points (app.h) ----

/// switch to SCR_CALIB (returns to prev_scr)
pub fn calibrate_open(a: &mut App) {
    todo!()
}
