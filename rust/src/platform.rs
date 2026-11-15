//! The platform: everything the app asks of the device it runs on (the panel, the pen and touch,
//! the light and the system's services), as one interface. display.rs, input.rs and light.rs keep
//! each function's documentation and types, and forward to the build's platform through `active()`:
//!
//! | build | platform | its modules |
//! |---|---|---|
//! | feature `kindle` | `Kindle` | display_fbink, input_evdev, light_kindle |
//! | feature `sim` | `Sim` | sim_sdl (the headless runner) |
//! | neither | `NoPlatform` | platform_none (host tools, cargo test) |
//! | feature `external-platform` | what another crate handed to `register` | that crate's own |
//!
//! The three in-tree platforms are unit types whose methods call their module's functions one for
//! one, and `active()` returns the build's own type: those calls are resolved at compile time and
//! inlined, exactly as the module aliases they replace were. Only an external platform is reached
//! through a `dyn` reference.
//!
//! A platform's methods are called from the app's one thread (PORTING.md, The process model), from
//! its forked children (which have that thread alone), and nowhere else. `Sync` is asked only so a
//! reference to the platform can be kept in a static.

use crate::prelude::*;

/// What a build's platform implements. The methods are display.rs's, input.rs's and light.rs's
/// functions under their own names; each is documented where it is forwarded from.
pub trait Platform: Sync {
    // ---- display.rs ----
    fn display_open(&self, w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32;
    fn display_present(&self, screen: &Canvas, r: Rect, mode: RefreshMode);
    fn display_set_orientation(&self, quarter_turns: i32, w: &mut i32, h: &mut i32);
    fn display_set_invert(&self, on: bool);
    fn display_pen_mode(&self, on: bool);
    fn display_live_ink(&self, on: bool);
    fn display_band_overwritten(&self, screen: &Canvas, band: Rect, skip: Rect) -> bool;
    fn display_find_overwrite(&self, screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool;
    fn display_skip_overlay(&self, r: Rect);
    fn display_skip_status(&self, r: Rect);
    fn display_pen_mode_on(&self) -> bool;
    fn display_broken(&self) -> bool;
    fn display_wait(&self);
    fn display_close(&self);
    /// display_wake's work: put the panel back in its normal mode after the native UI or a suspend
    /// may have changed it. Where the driver keeps no mode of its own, leaving pen mode is all of it.
    fn display_power_reset(&self) {
        self.display_pen_mode(false)
    }
    /// display_fast_busy: false where a refresh is done when it is sent.
    fn display_fast_busy(&self) -> bool {
        false
    }
    /// display_last_fast_waiting: false where no refresh waits on the driver.
    fn display_last_fast_waiting(&self) -> bool {
        false
    }

    // ---- input.rs ----
    fn input_open(&self, screen_w: i32, screen_h: i32, dpi: f32, pen: Option<&InputCalib>, touch: Option<&InputCalib>) -> i32;
    fn input_set_calib(&self, pen: Option<&InputCalib>, touch: Option<&InputCalib>);
    fn input_default_calib(&self, pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>);
    fn input_caps(&self) -> InputCaps;
    fn input_poll(&self, ev: &mut InEvent, timeout_ms: i32) -> i32;
    fn input_watch_fd(&self, fd: i32);
    fn input_shortcuts_enable(&self, enabled: bool);
    fn input_hids(&self, out: &mut [InputHid]) -> i32;
    fn input_set_orientation(&self, quarter_turns: i32);
    fn input_logical_to_panel(&self, lx: f32, ly: f32, px: &mut f32, py: &mut f32);
    fn input_set_grab(&self, grab: bool) -> bool;
    fn input_hover_hook(&self, f: Option<fn(f32, f32)>);
    fn input_sample_hook(&self, sample: Option<fn(&mut InEvent)>, batch: Option<fn()>);
    fn input_touch_context(&self, v: TgView);
    fn input_touch_trace(&self, path: Option<&str>);
    /// input_touch_trace_event: nothing where no touch trace is written.
    fn input_touch_trace_event(&self, line: &str) {
        let _ = line;
    }
    fn input_raw_pen(&self, out: &mut LiveRawMap) -> bool;
    fn input_forget_signal_pipe(&self);
    fn input_close(&self);

    // ---- light.rs ----
    fn light_available(&self) -> bool;
    fn light_refresh(&self);
    fn light_get(&self, intensity: &mut i32, warmth: &mut i32);
    fn light_set_intensity(&self, level: i32);
    fn light_set_warmth(&self, level: i32);
    fn light_warmth(&self) -> i32;
    fn sys_request_sleep(&self);
    fn sys_powerd_state(&self, out: &mut String, cap: usize) -> i32;
    fn sys_kindle_ui(&self, run: bool) -> bool;
    fn sys_prevent_sleep(&self, prevent: bool);
    fn sys_activity(&self);
    fn sys_battery(&self) -> Battery;
    fn sys_battery_reap(&self);
    fn sys_wall_clock(&self) -> i64;
    fn sys_services(&self) -> Services;
    fn sys_device_name(&self) -> &'static str;
}

/// An in-tree platform: a unit type whose methods are its modules' functions. `$display`, `$input`
/// and `$light` are the modules that implement display.h, input.h and light.h for the build; the
/// braces hold the methods it has beyond the required ones.
macro_rules! in_tree_platform {
    ($(#[$doc:meta])* $name:ident, $display:ident, $input:ident, $light:ident, { $($more:tt)* }) => {
        $(#[$doc])*
        pub struct $name;

        impl Platform for $name {
            #[inline]
            fn display_open(&self, w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
                crate::$display::display_open(w, h, dpi)
            }
            #[inline]
            fn display_present(&self, screen: &Canvas, r: Rect, mode: RefreshMode) {
                crate::$display::display_present(screen, r, mode)
            }
            #[inline]
            fn display_set_orientation(&self, quarter_turns: i32, w: &mut i32, h: &mut i32) {
                crate::$display::display_set_orientation(quarter_turns, w, h)
            }
            #[inline]
            fn display_set_invert(&self, on: bool) {
                crate::$display::display_set_invert(on)
            }
            #[inline]
            fn display_pen_mode(&self, on: bool) {
                crate::$display::display_pen_mode(on)
            }
            #[inline]
            fn display_live_ink(&self, on: bool) {
                crate::$display::display_live_ink(on)
            }
            #[inline]
            fn display_band_overwritten(&self, screen: &Canvas, band: Rect, skip: Rect) -> bool {
                crate::$display::display_band_overwritten(screen, band, skip)
            }
            #[inline]
            fn display_find_overwrite(&self, screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
                crate::$display::display_find_overwrite(screen, skip, where_)
            }
            #[inline]
            fn display_skip_overlay(&self, r: Rect) {
                crate::$display::display_skip_overlay(r)
            }
            #[inline]
            fn display_skip_status(&self, r: Rect) {
                crate::$display::display_skip_status(r)
            }
            #[inline]
            fn display_pen_mode_on(&self) -> bool {
                crate::$display::display_pen_mode_on()
            }
            #[inline]
            fn display_broken(&self) -> bool {
                crate::$display::display_broken()
            }
            #[inline]
            fn display_wait(&self) {
                crate::$display::display_wait()
            }
            #[inline]
            fn display_close(&self) {
                crate::$display::display_close()
            }

            #[inline]
            fn input_open(&self, screen_w: i32, screen_h: i32, dpi: f32, pen: Option<&InputCalib>, touch: Option<&InputCalib>) -> i32 {
                crate::$input::input_open(screen_w, screen_h, dpi, pen, touch)
            }
            #[inline]
            fn input_set_calib(&self, pen: Option<&InputCalib>, touch: Option<&InputCalib>) {
                crate::$input::input_set_calib(pen, touch)
            }
            #[inline]
            fn input_default_calib(&self, pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
                crate::$input::input_default_calib(pen, touch)
            }
            #[inline]
            fn input_caps(&self) -> InputCaps {
                crate::$input::input_caps()
            }
            #[inline]
            fn input_poll(&self, ev: &mut InEvent, timeout_ms: i32) -> i32 {
                crate::$input::input_poll(ev, timeout_ms)
            }
            #[inline]
            fn input_watch_fd(&self, fd: i32) {
                crate::$input::input_watch_fd(fd)
            }
            #[inline]
            fn input_shortcuts_enable(&self, enabled: bool) {
                crate::$input::input_shortcuts_enable(enabled)
            }
            #[inline]
            fn input_hids(&self, out: &mut [InputHid]) -> i32 {
                crate::$input::input_hids(out)
            }
            #[inline]
            fn input_set_orientation(&self, quarter_turns: i32) {
                crate::$input::input_set_orientation(quarter_turns)
            }
            #[inline]
            fn input_logical_to_panel(&self, lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
                crate::$input::input_logical_to_panel(lx, ly, px, py)
            }
            #[inline]
            fn input_set_grab(&self, grab: bool) -> bool {
                crate::$input::input_set_grab(grab)
            }
            #[inline]
            fn input_hover_hook(&self, f: Option<fn(f32, f32)>) {
                crate::$input::input_hover_hook(f)
            }
            #[inline]
            fn input_sample_hook(&self, sample: Option<fn(&mut InEvent)>, batch: Option<fn()>) {
                crate::$input::input_sample_hook(sample, batch)
            }
            #[inline]
            fn input_touch_context(&self, v: TgView) {
                crate::$input::input_touch_context(v)
            }
            #[inline]
            fn input_touch_trace(&self, path: Option<&str>) {
                crate::$input::input_touch_trace(path)
            }
            #[inline]
            fn input_raw_pen(&self, out: &mut LiveRawMap) -> bool {
                crate::$input::input_raw_pen(out)
            }
            #[inline]
            fn input_forget_signal_pipe(&self) {
                crate::$input::input_forget_signal_pipe()
            }
            #[inline]
            fn input_close(&self) {
                crate::$input::input_close()
            }

            #[inline]
            fn light_available(&self) -> bool {
                crate::$light::light_available()
            }
            #[inline]
            fn light_refresh(&self) {
                crate::$light::light_refresh()
            }
            #[inline]
            fn light_get(&self, intensity: &mut i32, warmth: &mut i32) {
                crate::$light::light_get(intensity, warmth)
            }
            #[inline]
            fn light_set_intensity(&self, level: i32) {
                crate::$light::light_set_intensity(level)
            }
            #[inline]
            fn light_set_warmth(&self, level: i32) {
                crate::$light::light_set_warmth(level)
            }
            #[inline]
            fn light_warmth(&self) -> i32 {
                crate::$light::light_warmth()
            }
            #[inline]
            fn sys_request_sleep(&self) {
                crate::$light::sys_request_sleep()
            }
            #[inline]
            fn sys_powerd_state(&self, out: &mut String, cap: usize) -> i32 {
                crate::$light::sys_powerd_state(out, cap)
            }
            #[inline]
            fn sys_kindle_ui(&self, run: bool) -> bool {
                crate::$light::sys_kindle_ui(run)
            }
            #[inline]
            fn sys_prevent_sleep(&self, prevent: bool) {
                crate::$light::sys_prevent_sleep(prevent)
            }
            #[inline]
            fn sys_activity(&self) {
                crate::$light::sys_activity()
            }
            #[inline]
            fn sys_battery(&self) -> Battery {
                crate::$light::sys_battery()
            }
            #[inline]
            fn sys_battery_reap(&self) {
                crate::$light::sys_battery_reap()
            }
            #[inline]
            fn sys_wall_clock(&self) -> i64 {
                crate::$light::sys_wall_clock()
            }
            #[inline]
            fn sys_services(&self) -> Services {
                crate::$light::sys_services()
            }
            #[inline]
            fn sys_device_name(&self) -> &'static str {
                crate::$light::sys_device_name()
            }

            $($more)*
        }
    };
}

#[cfg(feature = "kindle")]
in_tree_platform!(
    /// The Kindle: FBInk's display, the evdev pens, the power service's light.
    Kindle, display_fbink, input_evdev, light_kindle, {
        #[inline]
        fn display_power_reset(&self) {
            crate::display_fbink::display_power_reset()
        }
        #[inline]
        fn display_fast_busy(&self) -> bool {
            crate::display_fbink::display_fast_busy()
        }
        #[inline]
        fn display_last_fast_waiting(&self) -> bool {
            crate::display_fbink::display_last_fast_waiting()
        }
        #[inline]
        fn input_touch_trace_event(&self, line: &str) {
            crate::input_evdev::input_touch_trace_event(line)
        }
    }
);

#[cfg(feature = "sim")]
in_tree_platform!(
    /// The headless runner.
    Sim, sim_sdl, sim_sdl, sim_sdl, {
        #[inline]
        fn input_touch_trace_event(&self, line: &str) {
            crate::sim_sdl::input_touch_trace_event(line)
        }
    }
);

#[cfg(not(any(feature = "kindle", feature = "sim")))]
in_tree_platform!(
    /// No platform: host tools and tests, and an external-platform build before `register`.
    NoPlatform, platform_none, platform_none, platform_none, {}
);

/// The build's platform.
#[cfg(feature = "kindle")]
#[inline(always)]
pub(crate) fn active() -> &'static Kindle {
    &Kindle
}

/// The build's platform.
#[cfg(feature = "sim")]
#[inline(always)]
pub(crate) fn active() -> &'static Sim {
    &Sim
}

/// The build's platform.
#[cfg(not(any(feature = "kindle", feature = "sim", feature = "external-platform")))]
#[inline(always)]
pub(crate) fn active() -> &'static NoPlatform {
    &NoPlatform
}

#[cfg(feature = "external-platform")]
static REGISTERED: std::sync::OnceLock<&'static dyn Platform> = std::sync::OnceLock::new();

/// Hands the app its platform, in a build whose platform is another crate's. Once, before
/// mainloop::main, on the thread that will run it. False (and nothing changes) when one was
/// registered already.
#[cfg(feature = "external-platform")]
pub fn register(platform: &'static dyn Platform) -> bool {
    REGISTERED.set(platform).is_ok()
}

/// The registered platform; until there is one, no platform at all (opening the display fails).
#[cfg(feature = "external-platform")]
#[inline]
pub(crate) fn active() -> &'static dyn Platform {
    match REGISTERED.get() {
        Some(p) => *p,
        None => &NoPlatform,
    }
}
