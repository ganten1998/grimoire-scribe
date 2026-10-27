//! app.h: the App, screen switching, and the dirty-rect / refresh accumulator.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScreenId {
    #[default]
    Library = 0,  // SCR_LIBRARY
    Editor = 1,  // SCR_EDITOR
    Penset = 2,  // SCR_PENSET
    Calib = 3,  // SCR_CALIB
    Share = 4,  // SCR_SHARE
}
impl ScreenId {
    /// SCR_COUNT
    pub const COUNT: usize = 5;
}

/// Screen modules keep their own state; each allocates it in its *_init (library.rs, editor.rs,
/// pensettings.rs, calibrate.rs, share.rs define these).
pub use crate::calibrate::CalibState;
pub use crate::editor::EditorState;
pub use crate::library::LibraryState;
pub use crate::pensettings::PenSetState;
pub use crate::share::ShareState;

/// The one App of a process (mainloop.rs owns it, boxed). There is no global pointer to it: what the
/// C reaches through one (app_probe's g_probe_app, live_hover's g_live_app) is passed or mirrored
/// (PORTING.md, The App).
#[derive(Default)]
pub struct App {
    pub w: i32,
    pub h: i32,
    pub dpi: f32,
    /// mirrors the panel
    pub screen: Canvas,
    pub settings: Settings,
    pub shortcuts: Shortcuts,
    pub last_shortcut_code: i32,
    pub last_shortcut_mods: i32,
    pub last_shortcut_gamepad: bool,
    /// Erase while held's key or button, held now (0 none), and whether it is a controller's: its
    /// release ends it
    pub hold_code: i32,
    pub hold_pad: bool,
    /// keyboards and controllers that came or went (IN_HID_*)
    pub hid_changes: u32,
    /// the Bluetooth helper's state changes and answers (bluetooth.h)
    pub bt_changes: u32,
    /// when to start the helper (after the first frame), 0 none
    pub bt_start_at: u32,
    /// their last toasts: came, went (0 never)
    pub hid_toast_at: [u32; 2],
    pub scr: ScreenId,
    pub prev_scr: ScreenId,
    pub running: bool,
    /// Dirty accumulator, flushed once per loop iteration.
    pub dirty: Rect,
    pub dirty_mode: RefreshMode,
    /// promote to FULL every 30
    pub ui_refreshes_since_full: i32,
    /// applied device rotation (quarter turns clockwise)
    pub rotation: i32,
    /// reported by the sensor, applied when it is safe; -1 none
    pub pending_rotation: i32,
    pub native_pin: bool,
    pub input_yielded: bool,
    pub koreader_wallpaper: Canvas,
    /// the Kindle is showing the sleep screen
    pub asleep: bool,
    /// for the sleep timeout
    pub last_input: u32,
    pub sleep_requested: bool,
    /// 0 none: a follow-up redraw after waking
    pub wake_redraw_at: u32,
    /// when the power service was last told the user is active
    pub last_activity_poke: u32,
    /// a press waiting for the power service to announce sleep
    pub power_pressed_at: u32,
    /// asleep + power pressed: wake by then if outOfScreenSaver never came
    pub wake_fallback_at: u32,
    /// last preventScreenSaver sent (-1 none)
    pub prevent_applied: i32,
    /// leave/enter called only to re-lay out the same screen (rotation,
    /// a redraw after sleep): screens keep what they run (the share server)
    pub relayout: bool,
    /// Toast (drawn by app over any screen, auto-hidden after toast_until).
    /// room for two lines, in any language
    pub toast: FixedStr<256>,
    pub toast_until: u32,
    pub toast_rect: Rect,
    pub toast_visible: bool,
    /// pixels the toast covers, restored when it hides
    pub toast_under: Canvas,
    /// pixels Live view's chip covers, put back after each present
    pub chip_under: Canvas,
    /// toasts stay above this y (the keyboard, a sheet's buttons); 0 none.
    /// The screen sets it as it draws; app_switch clears it.
    pub toast_floor: i32,
    pub lib: Option<Box<LibraryState>>,
    pub ed: Option<Box<EditorState>>,
    pub ps: Option<Box<PenSetState>>,
    pub cal: Option<Box<CalibState>>,
    pub share: Option<Box<ShareState>>,
    /// The power button press being followed (its release decides which fallback it needs).
    /// 0 none
    pub power_down_at: u32,
    /// it went down while Grimoire showed its sleep screen
    pub power_down_asleep: bool,
    /// the power service announced a sleep or a wake since then
    pub power_answered: bool,
    /// wake fallbacks in a row that found the Kindle still locked
    pub locked_wakes: i32,
    /// when leaving stayed once because the page could not be saved (0 never)
    pub leave_unsaved_at: u32,
    /// a whole book KOReader sent (annotate.h): while its pages are being
    /// added, pen and touch wait and a toast says how far it got
    pub book: BookStatus,
    /// when that toast last changed
    pub book_told: u32,
}

pub fn app_init(a: &mut App, w: i32, h: i32, dpi: f32) -> i32 {
    todo!()
}
/// saves everything
pub fn app_shutdown(a: &mut App) {
    todo!()
}
pub fn app_event(a: &mut App, ev: &InEvent) {
    todo!()
}
/// timers: autosave, toast, long press, httpd
pub fn app_tick(a: &mut App, now: u32) {
    todo!()
}
/// presents dirty rect (if any)
pub fn app_flush(a: &mut App) {
    todo!()
}

pub fn app_invalidate(a: &mut App, r: Rect, m: RefreshMode) {
    todo!()
}
pub fn app_invalidate_all(a: &mut App, m: RefreshMode) {
    todo!()
}
/// calls leave/enter and redraws fully (REFRESH_FULL)
pub fn app_switch(a: &mut App, s: ScreenId) {
    todo!()
}
pub fn app_toast(a: &mut App, msg: &str, ms: i32) {
    todo!()
}
/// Turn the UI to a device rotation now (relayouts every screen).
pub fn app_set_rotation(a: &mut App, quarter_turns: i32) {
    todo!()
}
/// Where documents mirror PDFs go (/mnt/us/documents on Kindle, <root>/documents elsewhere).
/// Where the PDF mirror lives: grimoire/PDFs (outside documents/, so the Kindle never indexes
/// it: each indexed PDF made the home screen redraw its library, on top of Grimoire).
pub fn app_mirror_dir() -> String {
    todo!()
}
/// Pressed-state feedback for a tapped control: invert + FAST refresh now.
pub fn app_press_feedback(a: &mut App, r: Rect) {
    todo!()
}
/// A pen or finger event on an on-screen keyboard (keyboard.h), drawn and refreshed the one way every
/// keyboard is, so a key costs the same over any screen: only when the key changed something, only
/// the keyboard (never what is under it), and on the fast waveform, the text line first and on its
/// own, then the key. Returns keyboard_pointer's result; on KB_DONE and KB_CANCEL nothing is drawn
/// (the caller closes it and draws what it covered). (kb is not inside a: a screen takes its
/// keyboard out of its state for the call, PORTING.md, The App.)
pub fn app_keyboard_pointer(a: &mut App, kb: &mut Keyboard, phase: i32, ev: &InEvent) -> KbResult {
    todo!()
}
/// From the screen's tick while its keyboard is up: once typing pauses, one grayscale refresh of the
/// keyboard cleans up what the fast ones left.
pub fn app_keyboard_tick(a: &mut App, kb: &mut Keyboard, now: u32) {
    todo!()
}
/// Unified pointer phase for UI screens: pen and finger both act as a pointer.
/// Returns -1 if the event is not a pointer event, else 0 down / 1 move / 2 up.
pub fn app_pointer_phase(ev: &InEvent) -> i32 {
    todo!()
}

// Screen module interface (each implemented in its own module, as in its own .c):
//   X_init(a) allocates state; X_free(a);
//   X_enter(a) on switch-in (draw everything into a.screen and invalidate);
//   X_leave(a) on switch-out; X_event(a, ev); X_tick(a, now).
// (The C's DECLARE_SCREEN declares these; the definitions are the screens'.)
pub use crate::calibrate::{calibrate_enter, calibrate_event, calibrate_free, calibrate_init, calibrate_leave, calibrate_tick};
pub use crate::editor::{editor_enter, editor_event, editor_free, editor_init, editor_leave, editor_tick};
pub use crate::library::{library_enter, library_event, library_free, library_init, library_leave, library_tick};
pub use crate::pensettings::{
    pensettings_enter, pensettings_event, pensettings_free, pensettings_init, pensettings_leave, pensettings_tick,
};
pub use crate::share::{share_enter, share_event, share_free, share_init, share_leave, share_tick};

// Cross-screen entry points: the screens' (documented at their definitions), and app.c's own.
pub use crate::calibrate::calibrate_open;
pub use crate::editor::{
    editor_busy, editor_hold_eraser, editor_open_notebook, editor_open_notebook_at, editor_page_canvas,
    editor_picture_made, editor_pin_sleep_page, editor_probe, editor_save_now, editor_shortcut, editor_suspend,
};
pub use crate::library::{library_bt, library_learn_key, library_probe};
pub use crate::pensettings::{pensettings_open, pensettings_probe, pensettings_shortcut};
pub use crate::share::{share_draw_asleep, share_live_card_up, share_open, share_open_live, share_sleep};

/// Redraw the current screen from scratch (after sleep, or anything that drew over us).
pub fn app_redraw(a: &mut App) {
    todo!()
}
/// Same, with the refresh to use (waking uses a no-flash refresh: faster to get back to work).
pub fn app_redraw_mode(a: &mut App, m: RefreshMode) {
    todo!()
}
/// Apply the sleep policy to the power service (call after changing settings.sleep_minutes).
pub fn app_apply_sleep_policy(a: &mut App) {
    todo!()
}
/// Settings > Updates' Update now: the request is written; Grimoire leaves (the launcher hands the
/// update to update.sh).
pub fn app_leave_for_update(a: &mut App) {
    todo!()
}
/// Live view (live.h): the Share screen's button turns it on and shows the pairing card (again,
/// while it is on); live_view_stop turns it off (say: with a toast). app_watch_server keeps
/// input_poll watching the share server while it runs and the Kindle is awake.
pub fn live_view_open(a: &mut App) {
    todo!()
}
pub fn live_view_stop(a: &mut App, say: bool) {
    todo!()
}
pub fn app_watch_server(a: &mut App) {
    todo!()
}

/// The headless runner's `expect` checks read the app's state through this (tests only; cheap).
/// Keys: "screen" (library|editor|penset|calib|share), "toast" and "toast.rect" (the visible toast
/// and its rect, "" if none), "asleep" (0|1), "asleep.mode", "host", "rotation" (0..3), "size"
/// (WxH), "language", "bg.busy", "settings.<settings.ini key>" for a few and for every tool's pen
/// feel ("settings.ballpoint.size_max_mm"), "light.intensity", "light.warmth", and "editor.*",
/// "library.*", "penset.*" (below), "annotate.*" (app.c) and "templates.*" (the open Templates
/// sheet: New notebook's on the Library, else the editor's); docs/SIM.md has them all. 0 and the
/// value in out, -1 for an unknown key. (The C finds the App through g_probe_app; the runner passes
/// it here: sim_sdl.rs's probe point is handed the App by mainloop.rs.)
pub fn app_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
/// A custom music layout for the probes: "5: treble tab6 bass" (systems per page, then the parts
/// top to bottom); "" for None.
pub fn app_probe_music(m: Option<&MusicLayout>, out: &mut String, cap: usize) {
    todo!()
}
/// settings.ini's name for a tool ("pencil", "mech", ...), None past the list: how the probes name
/// tools. tests/test_probe.c holds it to what settings_save writes.
pub fn app_probe_tool(t: ToolKind) -> Option<&'static str> {
    const K: [&str; TOOL_COUNT] =
        ["pencil", "mech", "ballpoint", "eraser", "highlighter", "fountain", "fineliner", "marker", "charcoal", "calligraphy"];
    if t.0 >= 0 && (t.0 as usize) < TOOL_COUNT {
        Some(K[t.0 as usize])
    } else {
        None
    }
}
