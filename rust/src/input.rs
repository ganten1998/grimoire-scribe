//! input.h: pen / touch / button events in screen pixels. Implemented by input_evdev.c
//! (Kindle) and sim_sdl.c (desktop).

use crate::prelude::*;

// input.h is implemented by one platform file per build, as in the C: input_evdev.c on the Kindle,
// sim_sdl.c in the simulator; host builds (tools, tests) link none (platform_none.rs). The functions
// below forward to it, so the header's documentation stays here and the platform's signatures are
// checked against it.
#[cfg(feature = "kindle")]
use crate::input_evdev as platform;
#[cfg(not(any(feature = "kindle", feature = "sim")))]
use crate::platform_none as platform;
#[cfg(feature = "sim")]
use crate::sim_sdl as platform;

/// Maps a device axis pair, normalized to 0..1 over the device's absinfo range, to screen
/// px:  if swap_xy, (u,v) = (ny, nx) else (nx, ny);  sx = ax*u + bx;  sy = ay*v + by.
/// Inverted axes are negative a. valid=false means "use the built-in default guess".
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputCalib {
    pub valid: bool,
    pub swap_xy: bool,
    pub ax: f32,
    pub bx: f32,
    pub ay: f32,
    pub by: f32,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InType {
    #[default]
    None = 0,  // IN_NONE
    /// contact (pressure > 0 / BTN_TOUCH)
    PenDown = 1,  // IN_PEN_DOWN
    PenMove = 2,  // IN_PEN_MOVE
    PenUp = 3,  // IN_PEN_UP
    /// in proximity, not touching
    PenHover = 4,  // IN_PEN_HOVER
    /// left proximity
    PenLeave = 5,  // IN_PEN_LEAVE
    /// first finger only
    TouchDown = 6,  // IN_TOUCH_DOWN
    TouchMove = 7,  // IN_TOUCH_MOVE
    TouchUp = 8,  // IN_TOUCH_UP
    /// power button pressed
    Power = 9,  // IN_POWER
    /// signal / window closed
    Quit = 10,  // IN_QUIT
    /// pen side button pressed/released (side_button = state)
    PenButton = 11,  // IN_PEN_BUTTON
    /// device turned: value = quarter turns clockwise (0..3)
    Orientation = 12,  // IN_ORIENTATION
    /// two fingers: (x,y) and (x2,y2)
    PinchBegin = 13,  // IN_PINCH_BEGIN
    PinchMove = 14,  // IN_PINCH_MOVE
    PinchEnd = 15,  // IN_PINCH_END
    /// the Kindle is going to sleep (goingToScreenSaver)
    Sleep = 16,  // IN_SLEEP
    /// and waking up (outOfScreenSaver)
    Wake = 17,  // IN_WAKE
    /// power button released: value = ms it was held (-1 unknown)
    PowerUp = 18,  // IN_POWER_UP
    /// external keyboard/gamepad press: value=evdev key code
    Key = 19,  // IN_KEY
    /// power button held INPUT_POWER_HOLD_MS: once per press
    PowerHeld = 20,  // IN_POWER_HELD
    /// an optional keyboard or controller came: hid_name,
    /// gamepad, value 1 when another app holds it (input_hids)
    HidAttach = 21,  // IN_HID_ATTACH
    /// and went: hid_name, gamepad
    HidDetach = 22,  // IN_HID_DETACH
    /// an IN_KEY's key or button let go: value, gamepad
    KeyUp = 23,  // IN_KEY_UP
}

/// Holding the power button this long saves and leaves Grimoire (below the Kindle's own ~7 s menu).
pub const INPUT_POWER_HOLD_MS: i32 = 4000;

#[derive(Clone, Copy, Debug, Default)]
pub struct InEvent {
    pub type_: InType,
    /// screen px after calibration
    pub x: f32,
    pub y: f32,
    /// device-normalized 0..1 before calibration (for calibrate.c)
    pub nx: f32,
    pub ny: f32,
    /// 0..1 normalized raw
    pub pressure: f32,
    /// degrees from vertical, 0 if unsupported
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub key_mods: u32,
    pub gamepad: bool,
    /// eraser end in use (BTN_TOOL_RUBBER)
    pub eraser: bool,
    /// BTN_STYLUS held
    pub side_button: bool,
    /// IN_ORIENTATION: quarter turns clockwise; IN_POWER_UP/HELD: ms held
    pub value: i32,
    /// pinch: the second finger (screen px)
    pub x2: f32,
    pub y2: f32,
    /// when it was read (now_ms()): what the UI times gestures by
    pub t_ms: u32,
    /// Pen samples: when the kernel saw the frame, in now_ms()'s clock (pensync_ms), 0 where nobody
    /// says (the simulator). Only a stroke's samples take it (in_sample_ms): after a stall the read
    /// times bunch up, the kernel's keep the stroke's real spacing.
    pub k_ms: u32,
    /// Live view sample already captured, 0 when off
    pub live_seq: u32,
    /// the first sample after pen events the kernel dropped: joined straight
    pub gap: bool,
    /// IN_HID_*: the device's name, printable ASCII (hidkeys_name)
    pub hid_name: FixedStr<41>,
}

/// The time a stroke records for a pen sample: the kernel's when known, else when it was read.
#[inline]
pub fn in_sample_ms(e: &InEvent) -> u32 {
    if e.k_ms != 0 {
        e.k_ms
    } else {
        e.t_ms
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputCaps {
    pub has_pen: bool,
    pub has_pressure: bool,
    pub has_tilt: bool,
    pub has_touch: bool,
    pub has_power: bool,
    pub has_orientation: bool,
    /// current device orientation at open (quarter turns clockwise)
    pub orientation: i32,
    pub pen_name: FixedStr<64>,
    pub touch_name: FixedStr<64>,
}

#[inline]
pub fn input_open(screen_w: i32, screen_h: i32, pen: Option<&InputCalib>, touch: Option<&InputCalib>) -> i32 {
    platform::input_open(screen_w, screen_h, pen, touch)
}
/// NULL leaves that mapping unchanged; pass input_default_calib's values to reset one.
#[inline]
pub fn input_set_calib(pen: Option<&InputCalib>, touch: Option<&InputCalib>) {
    platform::input_set_calib(pen, touch)
}
/// The default (uncalibrated) mapping guess, so calibrate.c can show/compare it.
#[inline]
pub fn input_default_calib(pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
    platform::input_default_calib(pen, touch)
}
#[inline]
pub fn input_caps() -> InputCaps {
    platform::input_caps()
}
/// Returns 1 and fills ev if an event is ready within timeout_ms (0 = non-blocking,
/// -1 = forever), else 0. Never blocks once events are queued.
#[inline]
pub fn input_poll(ev: &mut InEvent, timeout_ms: i32) -> i32 {
    platform::input_poll(ev, timeout_ms)
}
/// Extra fd to wake input_poll (e.g. the HTTP share server socket); -1 to clear.
/// When it becomes readable, input_poll returns 0 early.
#[inline]
pub fn input_watch_fd(fd: i32) {
    platform::input_watch_fd(fd)
}
#[inline]
pub fn input_shortcuts_enable(enabled: bool) {
    platform::input_shortcuts_enable(enabled)
}
/// The optional keyboards and controllers watched now, none while shortcuts are off: their names
/// (as hid_name), whether each is a controller, whether another app holds it, and whether it is a
/// Switch-mode one (hidkeys_switch: the Buttons sheet names its buttons as printed). Returns how many.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputHid {
    pub name: FixedStr<41>,
    pub gamepad: bool,
    pub held: bool,
    pub switch_layout: bool,
}
#[inline]
pub fn input_hids(out: &mut [InputHid]) -> i32 {
    platform::input_hids(out)
}
/// Logical screen rotation (quarter turns clockwise of the device): events are reported in the
/// rotated screen's coordinates from now on. Calibration stays in panel coordinates.
#[inline]
pub fn input_set_orientation(quarter_turns: i32) {
    platform::input_set_orientation(quarter_turns)
}
/// Panel (unrotated) coordinates of a logical screen point, for calibration targets.
#[inline]
pub fn input_logical_to_panel(lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
    platform::input_logical_to_panel(lx, ly, px, py)
}
/// Release pen and touch to the native PIN screen, then grab them after native unlock.
#[inline]
pub fn input_set_grab(grab: bool) -> bool {
    platform::input_set_grab(grab)
}
/// Where the pen hovers, for Live view: fn gets the screen px (after the turn) at each of the raw
/// digitizer's frames while the pen is near and not touching; NULL stops it. On the Kindle the pen
/// service relays the pen coming and going but not where it hovers, so this is the only way to know
/// (pensync.h); never while input_set_grab(false) has given the pen to the Kindle. The simulator's
/// hovering mouse comes as IN_PEN_HOVER events, so there it does nothing. While input_sample_hook is
/// set, the raw hover goes to its sample callback instead (an IN_PEN_HOVER that is never queued),
/// with or without this hook.
#[inline]
pub fn input_hover_hook(f: Option<fn(f32, f32)>) {
    platform::input_hover_hook(f)
}
/// Called before a pen event is queued, including during work_pump; NULL removes it. On the Kindle
/// it also gets the raw digitizer's hover (input_hover_hook's place, as an IN_PEN_HOVER event that is
/// never queued: no screen sees it). The batch callback runs once after reading the pens (the
/// simulator: before each event is taken). Neither callback may wait, allocate or log.
#[inline]
pub fn input_sample_hook(sample: Option<fn(&mut InEvent)>, batch: Option<fn()>) {
    platform::input_sample_hook(sample, batch)
}
/// In a forked child, before it closes what it inherited: the stop handler (SIGTERM, SIGINT,
/// SIGHUP) writes nowhere from now on. The handler stays as it was, but the signal pipe's numbers
/// go to the files the child opens next, and a stop would write a byte into one of them.
#[inline]
pub fn input_forget_signal_pipe() {
    platform::input_forget_signal_pipe()
}
#[inline]
pub fn input_close() {
    platform::input_close()
}
