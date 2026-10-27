//! library.c: the home screen. Folders and notebooks as cards, paged (no scrolling on e-ink),
//! plus the sheets: new notebook (paper picker), settings, notebook actions, move, delete.

use crate::prelude::*;

/// The screen's state (library.c's, which app.h keeps opaque): library_init makes it, in a.lib.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct LibraryState {}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn library_init(a: &mut App) {
    todo!()
}
pub fn library_free(a: &mut App) {
    todo!()
}
pub fn library_enter(a: &mut App) {
    todo!()
}
pub fn library_leave(a: &mut App) {
    todo!()
}
pub fn library_event(a: &mut App, ev: &InEvent) {
    todo!()
}
pub fn library_tick(a: &mut App, now: u32) {
    todo!()
}

// ---- cross-screen entry points (app.h) ----

/// The Bluetooth helper's news (bluetooth.h) while the library is the screen: its Pair sheet's.
pub fn library_bt(a: &mut App, event: BtEvent, reply: &BtReply) {
    todo!()
}
/// A controller's press while Settings > Controller > Buttons learns one: true when it took it.
pub fn library_learn_key(a: &mut App, code: i32, gamepad: bool) -> bool {
    todo!()
}
/// The headless runner's "library.*" keys, asked without the prefix (app_probe's contract);
/// "templates.*" keys reach it whole (docs/SIM.md).
pub fn library_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
