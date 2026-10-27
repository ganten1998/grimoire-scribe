//! sim_sdl.c: desktop simulator implementing display.h + input.h.
//!
//! Window mode (SDL3, built with GRIMOIRE_SIM_SDL): mouse = pen, right button = finger, keys
//! tweak pressure/tilt. Headless mode (GRIMOIRE_SCRIPT=<file>, and the only mode of the
//! grimoire-headless build, which does not link SDL at all): events come from a script and the
//! screen can be dumped to PNG ("shot NAME") for automated checks. See docs/SIM.md.
//!
//! The Rust port is the headless runner only (the grimoire-headless build: GRIMOIRE_SCRIPT=<file>,
//! the same script protocol, the same messages on stderr, exit status 3 when an expect fails). The
//! SDL3 window simulator (grimoire-sim, GRIMOIRE_SIM_SDL) is a development aid that no check runs;
//! it is not ported (PORTING.md, What is not ported).

use crate::prelude::*;

// ---- display.h (display.rs forwards here in the simulator) ----

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

// ---- input.h ----

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

// ---- light.h: the simulated front light and power service ----

pub fn light_available() -> bool {
    todo!()
}
pub fn light_refresh() {
    todo!()
}
pub fn light_get(intensity: &mut i32, warmth: &mut i32) {
    todo!()
}
pub fn light_set_intensity(level: i32) {
    todo!()
}
pub fn light_set_warmth(level: i32) {
    todo!()
}
pub fn light_warmth() -> i32 {
    todo!()
}
pub fn sys_request_sleep() {
    todo!()
}
pub fn sys_powerd_state(out: &mut String, cap: usize) -> i32 {
    todo!()
}
pub fn sys_kindle_ui(run: bool) -> bool {
    todo!()
}
pub fn sys_prevent_sleep(prevent: bool) {
    todo!()
}
pub fn sys_activity() {
    todo!()
}

// ---- the runner's probe point ----

/// input_poll as the main loop calls it in the headless build, with the App: where the C's runner
/// takes a pending shot and checks a pending expect (inside input_poll, after the flush that ended
/// the batch), app_probe needs the App, which the C reaches through a global pointer (app.c's
/// g_probe_app) and the Rust is handed here (mainloop.rs; PORTING.md, The App). Otherwise exactly
/// input_poll.
pub fn input_poll_app(ev: &mut InEvent, timeout_ms: i32, a: &mut App) -> i32 {
    todo!()
}
