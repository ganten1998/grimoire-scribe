//! display.h: e-ink output. Implemented by display_fbink.c (Kindle) and sim_sdl.c.

use crate::prelude::*;

// display.h is implemented by one platform file per build, as in the C (display_fbink.c on the
// Kindle, sim_sdl.c in the simulator; none on the host: platform_none.rs). These forward to it.
#[cfg(feature = "kindle")]
use crate::display_fbink as platform;
#[cfg(not(any(feature = "kindle", feature = "sim")))]
use crate::platform_none as platform;
#[cfg(feature = "sim")]
use crate::sim_sdl as platform;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RefreshMode {
    /// DU: black/white, fast, for live ink
    #[default]
    Fast = 0,  // REFRESH_FAST
    /// GL16 partial, no flash
    Ui = 1,  // REFRESH_UI
    /// GC16 full flash
    Full = 2,  // REFRESH_FULL
}

/// Fills portrait width/height and dpi of the panel. 0 on success.
#[inline]
pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    platform::display_open(w, h, dpi)
}
/// Copies the rect of `screen` (same size as the display) to the panel and refreshes it.
#[inline]
pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {
    platform::display_present(screen, r, mode)
}
/// Device rotation (quarter turns clockwise): the logical screen the app draws is mapped onto
/// the panel accordingly. Fills the logical width/height (swapped for 1 and 3).
#[inline]
pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {
    platform::display_set_orientation(quarter_turns, w, h)
}
/// Show Grimoire's colors inverted (white on black) when on.
#[inline]
pub fn display_set_invert(on: bool) {
    platform::display_set_invert(on)
}
/// Low-latency handwriting mode (MTK pen mode on the Scribe). While on, only REFRESH_FAST
/// should be used; any other mode turns it off first. No-op where unsupported. The editor turns it
/// on for a stroke of ink, which marks live ink too (display_live_ink).
#[inline]
pub fn display_pen_mode(on: bool) {
    platform::display_pen_mode(on)
}
/// Live ink: a stroke being drawn with the pen (writing, erasing, the pen feel pad, the calibration's
/// check line), on at its start and off at its end. On a colour panel every other fast refresh is
/// followed by a wait until the driver has read the frame (display_fbink.c); live ink never waits,
/// so pen reading never stalls. Changes nothing else (no pen mode).
#[inline]
pub fn display_live_ink(on: bool) {
    platform::display_live_ink(on)
}

/// Which fast refreshes are live ink, the rule display_fbink.c follows (the simulator follows it
/// too, and reports it to scripts): those sent while a stroke is drawn, and the first one within
/// 100 ms of its end (the samples that came with the pen lifting go out just after the stroke ends).
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveInk {
    /// a stroke is being drawn
    pub on: bool,
    /// when the last one ended (0: none since the last fast refresh)
    pub ended: u32,
}
#[inline]
pub fn live_ink_mark(l: &mut LiveInk, on: bool, now: u32) {
    if l.on && !on {
        l.ended = now | 1;
    }
    l.on = on;
}
/// For a refresh in `mode` sent at `now`: true when it is a fast refresh of live ink.
#[inline]
pub fn live_ink_frame(l: &mut LiveInk, mode: RefreshMode, now: u32) -> bool {
    if mode != RefreshMode::Fast {
        return false;
    }
    let ink = l.on || (l.ended != 0 && (now.wrapping_sub(l.ended) as i32) < 100);
    l.ended = 0;
    ink
}
/// The screen guard. Both checks read the framebuffer back (8 or 32 bpp) to see whether another
/// process drew over Grimoire; cheap. The simulator never reports an overwrite.
/// This one samples a band of the screen densely: it catches small intruders such as the Kindle's
/// status bar clock. `skip` (may be empty) is excluded, e.g. a visible toast.
#[inline]
pub fn display_band_overwritten(screen: &Canvas, band: Rect, skip: Rect) -> bool {
    platform::display_band_overwritten(screen, band, skip)
}
/// This one finds a patch anywhere on screen that another process drew (a 24 px sampling grid, so
/// a strip of the Kindle home window is caught even when it covers little of the screen).
/// True with *where = the bounding box of the mismatching samples (padded).
#[inline]
pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    platform::display_find_overwrite(screen, skip, where_)
}
/// A rect the panel shows over App.screen for good (Live view's chip, app.c): both checks leave it
/// out, as they do a toast. RECT_EMPTY for none.
#[inline]
pub fn display_skip_overlay(r: Rect) {
    platform::display_skip_overlay(r)
}
/// True once the first frame and the next 4 refreshes have all failed: the panel is not Grimoire's
/// to drive, and it should leave. A failure after any refresh worked never counts. Polled by the
/// main loop: until one refresh works it also refreshes the whole panel again once a second, so a
/// dead panel is known within seconds even when nothing else draws. Always false in the simulator.
#[inline]
pub fn display_broken() -> bool {
    platform::display_broken()
}
/// Blocks until pending refreshes have completed (used before exit / screenshots).
#[inline]
pub fn display_wait() {
    platform::display_wait()
}
#[inline]
pub fn display_close() {
    platform::display_close()
}
