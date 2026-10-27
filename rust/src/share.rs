//! share.c: "Share over Wi-Fi". Keeps the HTTP server up (for its routes) only while this screen is
//! open; turning the Kindle or waking it only lays the screen out again, and the server (with any
//! download) carries on. A phone scans the QR code of the Wi-Fi address; a computer types the
//! address under it.
//!
//! The same screen is Live view's pairing card (share_open_live, live.h): the address of /live, a
//! QR code that pairs a phone at once, and a one-use code a computer types. While it is up nothing
//! is streamed (the code must not show on a stream), and it closes by itself once a browser pairs.

use crate::prelude::*;

/// The screen's state (share.c's, which app.h keeps opaque): share_init makes it, in a.share.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct ShareState {}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn share_init(a: &mut App) {
    todo!()
}
pub fn share_free(a: &mut App) {
    todo!()
}
pub fn share_enter(a: &mut App) {
    todo!()
}
pub fn share_leave(a: &mut App) {
    todo!()
}
pub fn share_event(a: &mut App, ev: &InEvent) {
    todo!()
}
pub fn share_tick(a: &mut App, now: u32) {
    todo!()
}

// ---- cross-screen entry points (app.h) ----

/// switch to SCR_SHARE (returns to prev_scr)
pub fn share_open(a: &mut App) {
    todo!()
}
/// the same, as Live view's pairing card
pub fn share_open_live(a: &mut App) {
    todo!()
}
/// Live view's pairing card is the screen
pub fn share_live_card_up(a: &App) -> bool {
    todo!()
}
/// asleep on Share: the server waits unwatched
pub fn share_sleep(a: &mut App) {
    todo!()
}
/// Share's sleep screen when no page is shown
pub fn share_draw_asleep(a: &mut App) {
    todo!()
}
