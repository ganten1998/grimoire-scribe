//! live.h: Live view, a computer's browser watching what Grimoire shows (docs/DESIGN.md, Live view).
//!
//! While it is on, every rect the main loop presents is copied from App.screen into a shared frame
//! (one memcpy of a rect it presents anyway; toasts and the live chip are never in it) and its
//! 64 px tiles are marked. A browser that has paired (pair.h) gets a viewer child of its own, forked
//! by the share server: it sends the tiles that changed as gray PNGs over one long HTTP response,
//! at most about 30 batches a second, and never holds up the pen: a slow browser only makes the
//! marks pile up, and the next batch carries what is newest. Two viewers at most. Nothing is
//! written to disk.
//!
//! The pen's place is one word in the shared frame (live_pen): a sample stores it, which is all the
//! pen path pays; only its coming into range wakes the viewers (a byte down each wake pipe), and
//! while it is in range each viewer looks at it again every LIVE_PEN_MS by itself.
//!
//! The stream's frames are little-endian: u32 length (of the type byte and the payload), u8 type,
//! payload:
//!   1 hello      u16 version (1), w, h, tile; u32 epoch: the screen's size, after a start or a turn
//!   2 tile       u16 x, y, w, h, then the PNG of that rect
//!   3 batch end  u32 seq, u32 the Kindle's ms: draw what came since the last one
//!   4 ping       nothing: sent after 15 s without a batch
//!   5 state      u8 LIVE_AWAKE, LIVE_ASLEEP, LIVE_PAIRING or LIVE_BYE; u8 1 when Grimoire shows its
//!                colors inverted (white on black: the frame never is), else 0; u8 the front light's
//!                warmth, 0..LIGHT_MAX, or LIVE_NO_WARMTH (no warm light, or not known). First thing
//!                on every stream, then whenever any of them changes.
//!   6 pen        u16 x, y in the screen's pixels (the tiles'), u8 LIVE_PEN_AWAY, LIVE_PEN_HOVER or
//!                LIVE_PEN_TOUCH: where the Kindle's pen is, for the browser to draw over the picture.
//!                After the tiles of the same batch, at most every LIVE_PEN_MS and only when it
//!                changed: the newest place, so a browser that falls behind skips the ones between.
//!                Only while the state is LIVE_AWAKE; a hello or any other state takes the pen away
//!                (the browser forgets it), and the next pen frame says where it is again.

use crate::prelude::*;


pub const LIVE_TILE: i32 = 64;
/// tile marks: 2048 tiles, more than either Scribe's screen has
pub const LIVE_WORDS: usize = 64;
pub const LIVE_VIEWERS: usize = 2;
/// no viewer for this long: Live view turns itself off
pub const LIVE_IDLE_OFF_MS: u32 = 10 * 60 * 1000;
/// the state frame's warmth when there is none to follow
pub const LIVE_NO_WARMTH: i32 = 255;
/// pen frames at most this often (about 30 a second)
pub const LIVE_PEN_MS: u32 = 33;

pub const LIVE_AWAKE: i32 = 0;
pub const LIVE_ASLEEP: i32 = 1;
pub const LIVE_PAIRING: i32 = 2;
pub const LIVE_BYE: i32 = 3;
pub const LIVE_HELLO: i32 = 1;
pub const LIVE_TILE_FRAME: i32 = 2;
pub const LIVE_BATCH: i32 = 3;
pub const LIVE_PING: i32 = 4;
pub const LIVE_STATE: i32 = 5;
pub const LIVE_PEN_FRAME: i32 = 6;
pub const LIVE_PEN_AWAY: i32 = 0;
pub const LIVE_PEN_HOVER: i32 = 1;
pub const LIVE_PEN_TOUCH: i32 = 2;

// ---- the main loop ----

/// Turns it on for a w x h screen: the shared frame, all of it marked. -1 when there is no memory
/// (or the screen has more tiles than the marks hold).
pub fn live_start(w: i32, h: i32) -> i32 {
    todo!()
}
/// Turns it off: each viewer is told (LIVE_BYE), then stopped and reaped; the frame goes.
pub fn live_stop() {
    todo!()
}
pub fn live_on() -> bool {
    todo!()
}
pub fn live_viewers() -> i32 {
    todo!()
}
/// A rect of the screen presented: copied into the frame, its tiles marked, the viewers woken.
pub fn live_push(screen: &Canvas, r: Rect) {
    todo!()
}
/// The screen turned (w x h now): a new hello, every tile again, the pen away until it is told again.
pub fn live_resize(w: i32, h: i32) {
    todo!()
}
/// The pen: LIVE_PEN_AWAY, LIVE_PEN_HOVER or LIVE_PEN_TOUCH at (x, y), the screen's pixels as the
/// events have them (after the turn). One atomic store; a write to the viewers' wake pipes only when
/// it comes into range. The caller never tells it while the Kindle sleeps (live_set_state clears it).
pub fn live_pen(state: i32, x: f32, y: f32) {
    todo!()
}
/// LIVE_AWAKE, LIVE_ASLEEP or LIVE_PAIRING (the pairing card is up: nothing is sent while it is).
/// Any but LIVE_AWAKE takes the pen away.
pub fn live_set_state(state: i32) {
    todo!()
}
/// Grimoire's colors inverted on the Kindle or not (settings.invert, applied as it presents: the
/// frame holds them as drawn): the viewers tell their browsers, which follow it unless told not to.
pub fn live_set_invert(on: bool) {
    todo!()
}
/// The Kindle's front light warmth (0..LIGHT_MAX; -1: no warm light, or not known): the browsers
/// tint the picture's light with it, unless told otherwise.
pub fn live_set_warmth(level: i32) {
    todo!()
}
/// Reaps viewers that ended. Returns how many viewers there are now. *idle_off is set when Live
/// view has had no viewer for LIVE_IDLE_OFF_MS (the caller turns it off and says so).
pub fn live_poll(now: u32, idle_off: &mut bool) -> i32 {
    todo!()
}

// ---- the share server ----

/// Hands the connection fd (an authenticated GET /live/stream) to a new viewer child: true when it
/// took it (the caller closes its own copy), false when two are watching already or fork failed.
pub fn live_take_viewer(fd: i32) -> bool {
    todo!()
}

// ---- pure parts (tests/test_live.c) ----

pub fn live_tiles_across(w: i32) -> i32 {
    todo!()
}
/// Sets the marks (tiles of a w x h screen, row by row) of every tile r touches.
pub fn live_mark(words: &mut [u32; LIVE_WORDS], w: i32, h: i32, r: Rect) {
    todo!()
}
/// The marked tiles as rects: each run of marked tiles in a tile row, cut to the screen. Returns
/// how many it wrote (at most cap).
pub fn live_runs(words: &[u32; LIVE_WORDS], w: i32, h: i32, out: &mut [Rect]) -> i32 {
    todo!()
}
/// A frame's 5-byte header for a payload of len bytes.
pub fn live_frame_header(out: &mut [u8; 5], len: u32, type_: u8) {
    todo!()
}
/// The pen as the shared word holds it: the state in the top 2 bits, y and x in 15 bits each, cut
/// to the w x h screen (rounded to the nearest pixel); 0 while it is away (or the state is not one).
pub fn live_pen_word(state: i32, x: f32, y: f32, w: i32, h: i32) -> u32 {
    todo!()
}
/// A pen frame's payload for that word: u16 x, y, u8 state.
pub fn live_pen_payload(out: &mut [u8; 5], word: u32) {
    todo!()
}
