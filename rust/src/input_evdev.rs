//! input_evdev.c: Kindle input from evdev.
//!
//! Devices are classified by capability, not by path:
//!   pen   = EV_ABS ABS_X + ABS_PRESSURE and EV_KEY BTN_TOOL_PEN. The Scribe 2 has two:
//!           "stylus-custom" (event6 since firmware 5.20.1, event5 before), a virtual pen that a
//!           pen service on the Kindle feeds from the raw digitizer, scaled to the screen and
//!           aligned with touch, but it only reports position + buttons (BTN_TOOL_PEN, BTN_TOUCH,
//!           ABS_X, ABS_Y: 2 or 3 events a frame). Which process makes it is not known; not
//!           input_manager, whose device is only "kindle-input-pt". And "WacomDigitizer"
//!           (event3), the raw EMR digitizer (0..15624 x 0..20832) with pressure 0..4095 and tilt
//!           in 1/100 degree (seen on device 2026-10-10). Grimoire takes position and buttons
//!           from the scaled pen (grabbed) and pressure + tilt from the raw one (read without a
//!           grab, so the pen service keeps feeding the scaled pen). The scaled pen says when the
//!           pen comes and goes but not where it hovers: Live view's hovering pen is the raw
//!           one's place (input_hover_hook).
//!   touch = EV_ABS ABS_MT_POSITION_X (the capacitive panel, usually /dev/input/touch)
//!   power = EV_KEY KEY_POWER on a device with few keys, never Amazon's every-key proxy
//!           "kindle-input-pt" (firmware 5.20+); the Scribe 2's is "bd71828-pwrkey" (hidkeys_power)
//! Pen and touch are EVIOCGRAB'ed so the (paused) framework never sees them; the power key is
//! only watched, so the system keeps handling it if we die. Its press and release are both read:
//! holding it INPUT_POWER_HOLD_MS (4 s) is Grimoire's guaranteed way out.
//!
//! Pen events follow the Wacom tool protocol (BTN_TOOL_PEN / BTN_TOOL_RUBBER bracket the
//! proximity, BTN_TOUCH marks contact, pressure/tilt arrive as EV_ABS, SYN_REPORT ends a frame),
//! the same model KOReader uses on the Scribe (input.wacom_protocol).
//!
//! The kernel keeps 128 events of the scaled pen for Grimoire: 120-175 ms of writing, and of the raw
//! digitizer about 100 ms, hover too. Nothing on the main thread may hold it longer while the pen is
//! near, or the kernel drops pen events and the stroke loses that stretch, or its pressure and tilt
//! (docs/INK-PERFORMANCE.md, Reader stalls). Long CPU work pumps (input_pump, through common.h's
//! work_pump), which reads the pens as it goes. A wait cannot pump: that is why key_scan never opens
//! a node Grimoire holds (each close waits out an RCU grace period) and never runs while the pen or
//! a finger is in use.

use crate::prelude::*;

// ---- input.h (input.rs forwards here on the Kindle) ----

pub fn input_open(screen_w: i32, screen_h: i32, pen: Option<&InputCalib>, touch: Option<&InputCalib>) -> i32 {
    todo!()
}
pub fn input_set_calib(pen: Option<&InputCalib>, touch: Option<&InputCalib>) {
    todo!()
}
pub fn input_default_calib(pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
    todo!()
}
pub fn input_caps() -> InputCaps {
    todo!()
}
pub fn input_poll(ev: &mut InEvent, timeout_ms: i32) -> i32 {
    todo!()
}
pub fn input_watch_fd(fd: i32) {
    todo!()
}
pub fn input_shortcuts_enable(enabled: bool) {
    todo!()
}
pub fn input_hids(out: &mut [InputHid]) -> i32 {
    todo!()
}
pub fn input_set_orientation(quarter_turns: i32) {
    todo!()
}
pub fn input_logical_to_panel(lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
    todo!()
}
pub fn input_set_grab(grab: bool) -> bool {
    todo!()
}
pub fn input_hover_hook(f: Option<fn(f32, f32)>) {
    todo!()
}
pub fn input_close() {
    todo!()
}

// ---- latency.h ----

/// Kindle only (read by display_fbink.rs for each live ink frame): the kernel times of the newest
/// and the oldest pen sample emitted since the last call, and of the raw sample the newest came
/// from (0 when it matched none). False when no pen sample came since the last call, or the pen's
/// events are not in CLOCK_MONOTONIC.
pub fn input_ink_times(newest: &mut u64, oldest: &mut u64, raw: &mut u64) -> bool {
    todo!()
}
