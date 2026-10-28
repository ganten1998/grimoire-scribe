//! platform_none: host builds (tools, cargo test) link no platform, as the C's host tools and unit
//! tests link neither display_fbink.c nor sim_sdl.c. display.rs, input.rs and light.rs forward
//! here: there is no panel, no input and no light, and opening either fails (-1). Nothing in a host
//! build calls these but app code that a host build never runs (the C's host builds do not even
//! link it); they do nothing and never panic.

use crate::prelude::*;

// ---- display.h ----

pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    -1
}
pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {}
pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {}
pub fn display_set_invert(on: bool) {}
pub fn display_pen_mode(on: bool) {}
pub fn display_live_ink(on: bool) {}
pub fn display_band_overwritten(screen: &Canvas, band: Rect, skip: Rect) -> bool {
    false
}
pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    *where_ = RECT_EMPTY;
    false
}
pub fn display_skip_overlay(r: Rect) {}
pub fn display_skip_status(r: Rect) {}
pub fn display_pen_mode_on() -> bool {
    false
}
pub fn display_broken() -> bool {
    false
}
pub fn display_wait() {}
pub fn display_close() {}

// ---- input.h ----

pub fn input_open(screen_w: i32, screen_h: i32, pen: Option<&InputCalib>, touch: Option<&InputCalib>) -> i32 {
    -1
}
pub fn input_set_calib(pen: Option<&InputCalib>, touch: Option<&InputCalib>) {}
pub fn input_default_calib(pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
    let _ = (pen, touch);
}
pub fn input_caps() -> InputCaps {
    InputCaps::default()
}
pub fn input_poll(ev: &mut InEvent, timeout_ms: i32) -> i32 {
    0
}
pub fn input_watch_fd(fd: i32) {}
pub fn input_shortcuts_enable(enabled: bool) {}
pub fn input_hids(out: &mut [InputHid]) -> i32 {
    0
}
pub fn input_set_orientation(quarter_turns: i32) {}
pub fn input_logical_to_panel(lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
    *px = lx;
    *py = ly;
}
pub fn input_set_grab(grab: bool) -> bool {
    false
}
pub fn input_hover_hook(f: Option<fn(f32, f32)>) {}
pub fn input_close() {}

// ---- light.h ----

pub fn light_available() -> bool {
    false
}
pub fn light_refresh() {}
pub fn light_get(intensity: &mut i32, warmth: &mut i32) {
    *intensity = -1;
    *warmth = -1;
}
pub fn light_set_intensity(level: i32) {}
pub fn light_set_warmth(level: i32) {}
pub fn light_warmth() -> i32 {
    -1
}
pub fn sys_request_sleep() {}
pub fn sys_powerd_state(out: &mut String, cap: usize) -> i32 {
    -1
}
pub fn sys_kindle_ui(run: bool) -> bool {
    false
}
pub fn sys_prevent_sleep(prevent: bool) {}
pub fn sys_activity() {}
pub fn sys_battery() -> Battery {
    Battery::default()
}
pub fn sys_wall_clock() -> i64 {
    wall_time()
}
