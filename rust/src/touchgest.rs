//! touchgest: the touch recognizer, pure and unit-tested on the host like pensync.rs. Both readers
//! of the touch panel hand it contact frames (every contact down at one SYN_REPORT, in logical screen
//! px): input_evdev.rs on the Kindle, and the headless runner (sim_sdl.rs). The screen in front tells
//! it what a two-finger drag may do there (TgView, through input_touch_context).

/// What the screen in front allows, pushed by the editor (input_touch_context).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TgView {
    /// the page can scroll on that axis now
    pub scrolls_v: bool,
    pub scrolls_h: bool,
    /// settings.pinch_zoom, and no sheet in front that refuses pinches
    pub zoom_ok: bool,
    /// settings.gestures
    pub gestures_on: bool,
    /// phase 3's recording; always false now
    pub record: bool,
    pub keyboard_open: bool,
}
