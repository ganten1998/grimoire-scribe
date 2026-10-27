//! display_fbink.c: Kindle e-ink output through FBInk (MTK / Bellatrix3 on the Scribe 2).
//!
//! Pixels go straight into the mmap'ed framebuffer (FBInk's raw-data blitter is compiled out
//! of MINIMAL builds, and a direct copy is the fastest path anyway). FBInk then issues the
//! refresh ioctl with the waveform for the mode:
//!   FAST -> DU   (in pen mode on MTK: the driver's path for handwriting, see below)
//!   UI   -> GL16 (partial, no flash)
//!   FULL -> GC16 + flash
//! "Fast mode" (no automatic REAGL promotion) is enabled for the session like KOReader does on
//! the Scribe, so large partial refreshes keep the waveform we ask for.
//!
//! Orientation, two layers:
//!  1. Device rotation (display_set_orientation): the app draws a logical screen (portrait, or
//!     landscape when the Kindle is turned sideways) that is mapped onto the panel.
//!  2. Framebuffer quirk: on the Scribe 2 the framebuffer's memory can be upside down relative
//!     to the touch/pen panel, depending on the rotation the Kindle UI left it in (the stock UI
//!     renders rotated itself). Seen on device 2026-10-10: at fb rotate 1 every tap landed exactly
//!     180 degrees from the drawn button. Override with an empty /mnt/us/grimoire/screen-flip or
//!     /mnt/us/grimoire/screen-noflip.
//!
//! Colour panels (the Scribe Colorsoft: FBInk's has_color_panel, 32 bpp): the driver reads the
//! framebuffer for an update some time after it was sent (about 23 ms, as KOReader's developers
//! measured it on a Scribe Colorsoft; Grimoire has not run on one), in tiles. Pixels changed again
//! before that show half old, striped. So after a fast
//! refresh that is not live ink (a pressed button, the panel being dragged, a pinch, a slider)
//! Grimoire waits until the driver has read the frame (MXCFB_WAIT_FOR_MDP_COMPLETION), as KOReader
//! does. Live ink (display_live_ink: a pen stroke, erasing included) never waits, so pen reading
//! never stalls: a stroke's frames only add ink the next one sends again, and erasing ends with a
//! grayscale refresh of all it erased, which repaints anything a frame's read caught half done.

use crate::prelude::*;

// ---- display.h (display.rs forwards here on the Kindle) ----

pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    todo!()
}
pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {
    todo!()
}
pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {
    todo!()
}
pub fn display_set_invert(on: bool) {
    todo!()
}
pub fn display_pen_mode(on: bool) {
    todo!()
}
pub fn display_live_ink(on: bool) {
    todo!()
}
pub fn display_band_overwritten(screen: &Canvas, band: Rect, skip: Rect) -> bool {
    todo!()
}
pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    todo!()
}
pub fn display_skip_overlay(r: Rect) {
    todo!()
}
pub fn display_broken() -> bool {
    todo!()
}
pub fn display_wait() {
    todo!()
}
pub fn display_close() {
    todo!()
}
