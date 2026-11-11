//! display.h: e-ink output. Implemented by display_fbink.c (Kindle) and sim_sdl.c.

use crate::prelude::*;

// display.h is implemented by one platform file per build, as in the C (display_fbink.c on the
// Kindle, sim_sdl.c in the simulator; none on the host: platform_none.rs). These forward to it,
// through the build's platform (platform.rs).
// (the in-tree platforms' methods need the trait in scope; a registered `dyn` platform's do not)
#[cfg_attr(feature = "external-platform", allow(unused_imports))]
use crate::platform::{active, Platform};

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
    /// GC16 flash of the rect alone (KOReader's "flashui"): every pixel in it driven, so what fast DU
    /// refreshes left there is wiped, which a GL16 partial cannot do (it drives only the pixels
    /// whose value changed). The Rust build only: the C tree has no such mode.
    Clean = 3,
}

impl RefreshMode {
    /// Which of two modes asked for one rect wins (app_invalidate keeps the strongest): Fast, Ui,
    /// Clean, then Full, which flashes the whole screen anyway.
    pub fn strength(self) -> i32 {
        match self {
            RefreshMode::Fast => 0,
            RefreshMode::Ui => 1,
            RefreshMode::Clean => 2,
            RefreshMode::Full => 3,
        }
    }
}

// ---- DU wear (issue #11) ----
//
// A fast (DU) refresh drives a pixel to black or white in one short waveform, and one it turns from
// black to white keeps a faint trace of the black (DU ghosting). The grayscale settle that follows
// (GL16, a partial update) does not reach it: a partial update drives only the pixels whose value
// changed, and DU already wrote that pixel's final white. So the traces of erased ink and of the
// toolbar's old places stay until a flashing refresh (a page turn, the Library and back). DuWear
// counts the screen area fast refreshes turned back toward the paper (the eraser's, a dragged
// selection's); past a quarter of the screen the editor cleans where they were with one Clean
// refresh, at a pause. Writing is not counted: DU turning white paper black leaves no trace.

/// The share of the screen past which DU wear is cleaned: a quarter (issue #11's figure). A
/// handwritten word is about 1 % of the Scribe's screen, so small erases add up to a clean only
/// after twenty-odd words, and one erase of a quarter page cleans at once.
pub const DU_WEAR_QUARTERS: i64 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DuWear {
    /// screen px fast refreshes turned toward the paper since the last clean, counted each time
    pub area: i64,
    /// where they were (screen px): the rect a clean refreshes
    pub rect: Rect,
    /// a clean is owed whatever the area (a toolbar drag that could not clean at its release)
    pub owed: bool,
    /// when the last was added (ms), so a clean waits for a pause in what added it
    pub at: u32,
}

/// `area` screen px of `r` were turned toward the paper by a fast refresh (area at most r's).
pub fn du_wear_add(w: &mut DuWear, r: Rect, area: i64, now: u32) {
    if rect_is_empty(r) || area <= 0 {
        return;
    }
    w.area = w.area.saturating_add(area.min(r.w as i64 * r.h as i64));
    w.rect = rect_union(w.rect, r);
    w.at = now;
}

/// A clean of r is owed whatever the area.
pub fn du_wear_owe(w: &mut DuWear, r: Rect, now: u32) {
    if rect_is_empty(r) {
        return;
    }
    w.rect = rect_union(w.rect, r);
    w.owed = true;
    w.at = now;
}

/// Whether a clean is due on a screen of sw x sh: owed, or more than a quarter of it worn.
pub fn du_wear_due(w: &DuWear, sw: i32, sh: i32) -> bool {
    !rect_is_empty(w.rect) && (w.owed || w.area * 4 > DU_WEAR_QUARTERS * sw as i64 * sh as i64)
}

/// A refresh of r in mode m went out: a Full wipes every trace, a Clean wipes those it covers.
pub fn du_wear_presented(w: &mut DuWear, r: Rect, m: RefreshMode) {
    if m == RefreshMode::Full || (m == RefreshMode::Clean && rect_contains_rect(r, w.rect)) {
        *w = DuWear::default();
    }
}

/// Fills portrait width/height and dpi of the panel. 0 on success.
#[inline]
pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    active().display_open(w, h, dpi)
}
/// Copies the rect of `screen` (same size as the display) to the panel and refreshes it.
#[inline]
pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {
    active().display_present(screen, r, mode)
}
/// Device rotation (quarter turns clockwise): the logical screen the app draws is mapped onto
/// the panel accordingly. Fills the logical width/height (swapped for 1 and 3).
#[inline]
pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {
    active().display_set_orientation(quarter_turns, w, h)
}
/// Show Grimoire's colors inverted (white on black) when on.
#[inline]
pub fn display_set_invert(on: bool) {
    active().display_set_invert(on)
}
/// Low-latency handwriting mode (MTK pen mode on the Scribe). While on, only REFRESH_FAST
/// should be used; any other mode turns it off first. No-op where unsupported. The editor turns it
/// on for a stroke of ink, which marks live ink too (display_live_ink).
#[inline]
pub fn display_pen_mode(on: bool) {
    active().display_pen_mode(on)
}
/// Live ink: a stroke being drawn with the pen (writing, erasing, the pen feel pad, the calibration's
/// check line), on at its start and off at its end. On a colour panel every other fast refresh is
/// followed by a wait until the driver has read the frame (display_fbink.c); live ink never waits,
/// so pen reading never stalls. Changes nothing else (no pen mode).
#[inline]
pub fn display_live_ink(on: bool) {
    active().display_live_ink(on)
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
    active().display_band_overwritten(screen, band, skip)
}
/// This one finds a patch anywhere on screen that another process drew (a 24 px sampling grid, so
/// a strip of the Kindle home window is caught even when it covers little of the screen).
/// True with *where = the bounding box of the mismatching samples (padded).
#[inline]
pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    active().display_find_overwrite(screen, skip, where_)
}
/// A rect the panel shows over App.screen for good (Live view's chip, app.c): both checks leave it
/// out, as they do a toast. RECT_EMPTY for none.
#[inline]
pub fn display_skip_overlay(r: Rect) {
    active().display_skip_overlay(r)
}
/// A rect the panel shows over App.screen while the clock and battery corner is up (status.rs):
/// both checks leave it out too, apart from the Live view chip's, which has its own. RECT_EMPTY for
/// none.
#[inline]
pub fn display_skip_status(r: Rect) {
    active().display_skip_status(r)
}
/// True while the panel's driver is in pen mode: set by the first stroke's fast frames and
/// kept past the stroke until a grayscale refresh is due (display_fbink.rs, pen_leave), which then
/// waits for the pen's last frame. The simulator answers with what display_pen_mode last asked.
#[inline]
pub fn display_pen_mode_on() -> bool {
    active().display_pen_mode_on()
}
/// True once the first frame and the next 4 refreshes have all failed: the panel is not Grimoire's
/// to drive, and it should leave. A failure after any refresh worked never counts. Polled by the
/// main loop: until one refresh works it also refreshes the whole panel again once a second, so a
/// dead panel is known within seconds even when nothing else draws. Always false in the simulator.
#[inline]
pub fn display_broken() -> bool {
    active().display_broken()
}
/// Blocks until pending refreshes have completed (used before exit / screenshots).
#[inline]
pub fn display_wait() {
    active().display_wait()
}
/// Finish our updates before handing the display to the screensaver/native UI. Pen mode is a
/// device setting, so leave it explicitly even when no sleep image will be drawn.
pub fn display_sleep() {
    display_wait(); // exactly one waiter: the Kindle backend follows the quiet thread's books
    display_wake();
}
/// Reassert normal display mode after native UI or hardware suspend may have changed it.
pub fn display_wake() {
    active().display_power_reset()
}
#[inline]
pub fn display_close() {
    active().display_close()
}
/// Whether the last fast refresh sent may still be on the panel, its waveform not ended: a held delete's
/// next repeat waits for it rather than queue behind it in the driver (keyboard_repeat). No display
/// operation is issued. Never in the simulator, whose refreshes are done when they are sent.
pub fn display_fast_busy() -> bool {
    active().display_fast_busy()
}
/// Cached indication for pacing non-ink trails on a colour panel. No display operation is issued.
pub fn display_last_fast_waiting() -> bool {
    active().display_last_fast_waiting()
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 1860;
    const H: i32 = 2480;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn strength_orders_the_modes() {
        assert!(RefreshMode::Fast.strength() < RefreshMode::Ui.strength());
        assert!(RefreshMode::Ui.strength() < RefreshMode::Clean.strength());
        assert!(RefreshMode::Clean.strength() < RefreshMode::Full.strength());
    }

    #[test]
    fn small_erases_never_make_a_clean() {
        let mut w = DuWear::default();
        // nineteen word-sized erases (400 x 150 px, 1.3 % of the screen each) stay under a quarter;
        // the twentieth passes it
        for k in 0..19 {
            du_wear_add(&mut w, r(100, 100 + k * 110, 400, 150), 400 * 150, k as u32);
            assert!(!du_wear_due(&w, W, H), "erase {k}");
        }
        assert_eq!(w.rect, r(100, 100, 400, 18 * 110 + 150));
        assert_eq!(w.at, 18);
        du_wear_add(&mut w, r(100, 2200, 400, 150), 400 * 150, 19);
        assert!(du_wear_due(&w, W, H));
    }

    #[test]
    fn a_quarter_of_the_screen_makes_a_clean() {
        let mut w = DuWear::default();
        let quarter = W as i64 * H as i64 / 4;
        du_wear_add(&mut w, r(0, 0, W, H / 4), quarter, 5);
        assert!(!du_wear_due(&w, W, H), "a quarter exactly is not more than one");
        du_wear_add(&mut w, r(0, H / 4, 10, 10), 100, 6);
        assert!(du_wear_due(&w, W, H));
        // the area counted is never more than the rect's (a long thin erase's bbox)
        let mut v = DuWear::default();
        du_wear_add(&mut v, r(0, 0, 10, 10), 1_000_000_000, 1);
        assert_eq!(v.area, 100);
        // nothing for an empty rect or no area
        du_wear_add(&mut v, RECT_EMPTY, 50, 2);
        du_wear_add(&mut v, r(0, 0, 5, 5), 0, 3);
        assert_eq!((v.area, v.at), (100, 1));
    }

    #[test]
    fn a_clean_that_covers_it_starts_again() {
        let mut w = DuWear::default();
        du_wear_add(&mut w, r(100, 100, W / 2, H), W as i64 * H as i64 / 2, 1);
        assert!(du_wear_due(&w, W, H));
        // a grayscale refresh does not reach DU's traces, nor a clean of part of them
        du_wear_presented(&mut w, r(0, 0, W, H), RefreshMode::Ui);
        du_wear_presented(&mut w, r(100, 100, 10, 10), RefreshMode::Clean);
        assert!(du_wear_due(&w, W, H));
        du_wear_presented(&mut w, r(100, 100, W / 2, H), RefreshMode::Clean);
        assert_eq!(w, DuWear::default());
        assert!(!du_wear_due(&w, W, H));
        // a Full wipes it wherever it was
        du_wear_add(&mut w, r(0, 0, W, H), W as i64 * H as i64, 2);
        du_wear_presented(&mut w, r(5, 5, 1, 1), RefreshMode::Full);
        assert_eq!(w, DuWear::default());
    }

    #[test]
    fn an_owed_clean_is_due_whatever_the_area() {
        let mut w = DuWear::default();
        du_wear_owe(&mut w, RECT_EMPTY, 1);
        assert!(!du_wear_due(&w, W, H));
        du_wear_owe(&mut w, r(1700, 200, 128, 1700), 4);
        assert!(du_wear_due(&w, W, H) && w.area == 0 && w.at == 4);
        du_wear_presented(&mut w, r(1600, 0, 260, 2480), RefreshMode::Clean);
        assert!(!du_wear_due(&w, W, H));
    }
}
