// Not a port of a C test: a platform that comes from outside the crate (the external-platform
// feature, PORTING.md, Variants), run through the whole app. A recording stand-in is registered and
// mainloop::main runs on it: the start, the first frame, a few idle polls, a quit, the shutdown.
// What it checks is the seam itself: the app's calls reach the registered platform, in the app's
// order, and the app runs to a clean exit on a platform it has never seen.
//
// Only in that build:
//   cargo test --features external-platform --test test_platform_external -- --test-threads=1
#![cfg(feature = "external-platform")]
mod testlib;
use grimoire::platform::{register, Platform};
use grimoire::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use testlib::*;

// A Scribe-sized panel.
const W: i32 = 1860;
const H: i32 = 2480;
/// Idle polls before the stand-in says quit.
const POLLS: u32 = 5;

struct Recorder {
    /// the calls that mark the app's order, by name
    calls: Mutex<Vec<&'static str>>,
    /// each present's rect and mode
    presents: Mutex<Vec<(Rect, RefreshMode)>>,
    polls: AtomicU32,
}

impl Recorder {
    fn note(&self, what: &'static str) {
        self.calls.lock().unwrap().push(what);
    }
}

static REC: Recorder = Recorder { calls: Mutex::new(Vec::new()), presents: Mutex::new(Vec::new()), polls: AtomicU32::new(0) };

impl Platform for Recorder {
    fn display_open(&self, w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
        self.note("display_open");
        *w = W;
        *h = H;
        *dpi = 300.0;
        0
    }
    fn display_present(&self, screen: &Canvas, r: Rect, mode: RefreshMode) {
        // the screen handed over is the display's size, whatever the rect
        check!(screen.w == W && screen.h == H);
        self.presents.lock().unwrap().push((r, mode));
    }
    fn display_set_orientation(&self, quarter_turns: i32, w: &mut i32, h: &mut i32) {
        let turned = quarter_turns & 1 != 0;
        *w = if turned { H } else { W };
        *h = if turned { W } else { H };
    }
    fn display_set_invert(&self, _on: bool) {}
    fn display_pen_mode(&self, on: bool) {
        self.note(if on { "display_pen_mode(true)" } else { "display_pen_mode(false)" });
    }
    fn display_live_ink(&self, _on: bool) {}
    fn display_band_overwritten(&self, _screen: &Canvas, _band: Rect, _skip: Rect) -> bool {
        false
    }
    fn display_find_overwrite(&self, _screen: &Canvas, _skip: Rect, where_: &mut Rect) -> bool {
        *where_ = RECT_EMPTY;
        false
    }
    fn display_skip_overlay(&self, _r: Rect) {}
    fn display_skip_status(&self, _r: Rect) {}
    fn display_pen_mode_on(&self) -> bool {
        false
    }
    fn display_broken(&self) -> bool {
        false
    }
    fn display_wait(&self) {
        self.note("display_wait");
    }
    fn display_close(&self) {
        self.note("display_close");
    }

    fn input_open(&self, screen_w: i32, screen_h: i32, dpi: f32, _pen: Option<&InputCalib>, _touch: Option<&InputCalib>) -> i32 {
        // the app opens input with what display_open answered
        check!(screen_w == W && screen_h == H && dpi == 300.0);
        self.note("input_open");
        0
    }
    fn input_set_calib(&self, _pen: Option<&InputCalib>, _touch: Option<&InputCalib>) {}
    fn input_default_calib(&self, _pen: Option<&mut InputCalib>, _touch: Option<&mut InputCalib>) {}
    fn input_caps(&self) -> InputCaps {
        InputCaps { has_pen: true, has_pressure: true, has_touch: true, ..InputCaps::default() }
    }
    fn input_poll(&self, ev: &mut InEvent, _timeout_ms: i32) -> i32 {
        // nothing for a few rounds of the loop, then the stop a signal or a closed window gives
        if self.polls.fetch_add(1, Ordering::Relaxed) < POLLS {
            return 0;
        }
        *ev = InEvent { type_: InType::Quit, ..InEvent::default() };
        1
    }
    fn input_watch_fd(&self, _fd: i32) {}
    fn input_shortcuts_enable(&self, _enabled: bool) {}
    fn input_hids(&self, _out: &mut [InputHid]) -> i32 {
        0
    }
    fn input_set_orientation(&self, _quarter_turns: i32) {}
    fn input_logical_to_panel(&self, lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
        *px = lx;
        *py = ly;
    }
    fn input_set_grab(&self, _grab: bool) -> bool {
        false
    }
    fn input_hover_hook(&self, _f: Option<fn(f32, f32)>) {}
    fn input_sample_hook(&self, _sample: Option<fn(&mut InEvent)>, _batch: Option<fn()>) {}
    fn input_touch_context(&self, _v: TgView) {}
    fn input_touch_trace(&self, _path: Option<&str>) {}
    fn input_raw_pen(&self, _out: &mut LiveRawMap) -> bool {
        false
    }
    fn input_forget_signal_pipe(&self) {}
    fn input_close(&self) {
        self.note("input_close");
    }

    fn light_available(&self) -> bool {
        false
    }
    fn light_refresh(&self) {
        self.note("light_refresh");
    }
    fn light_get(&self, intensity: &mut i32, warmth: &mut i32) {
        *intensity = -1;
        *warmth = -1;
    }
    fn light_set_intensity(&self, _level: i32) {}
    fn light_set_warmth(&self, _level: i32) {}
    fn light_warmth(&self) -> i32 {
        -1
    }
    fn sys_request_sleep(&self) {}
    fn sys_powerd_state(&self, _out: &mut String, _cap: usize) -> i32 {
        -1
    }
    fn sys_kindle_ui(&self, _run: bool) -> bool {
        false
    }
    fn sys_prevent_sleep(&self, _prevent: bool) {}
    fn sys_activity(&self) {}
    fn sys_battery(&self) -> Battery {
        Battery::default()
    }
    fn sys_battery_reap(&self) {}
    fn sys_wall_clock(&self) -> i64 {
        wall_time()
    }
    fn sys_services(&self) -> Services {
        Services::ALL
    }
    fn sys_device_name(&self) -> &'static str {
        "Kindle"
    }
}

/// Where `what` first comes in the recorded order.
fn at(calls: &[&'static str], what: &str) -> Option<usize> {
    calls.iter().position(|c| *c == what)
}

fn no_platform_until_one_is_registered() {
    // as a host build with no platform at all: there is no panel to open
    let (mut w, mut h, mut dpi) = (0i32, 0i32, 0f32);
    check!(display_open(&mut w, &mut h, &mut dpi) == -1);
    check!(REC.calls.lock().unwrap().is_empty());
}

fn one_platform_is_registered_once() {
    check!(register(&REC));
    check!(!register(&REC));
}

fn the_app_runs_on_it_from_start_to_exit() {
    let root = format!("{}/platform-external-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(mkdir_p(&root) == 0);
    std::env::set_var("GRIMOIRE_ROOT", &root);
    let status = grimoire::mainloop::main(vec!["grimoire".to_string()]);
    check!(status == 0);

    let calls = REC.calls.lock().unwrap().clone();
    // the start, in main's order: the display, its normal mode (display_wake, which is leaving pen
    // mode where the platform has no reset of its own), the light, then input
    let open = at(&calls, "display_open");
    let wake = at(&calls, "display_pen_mode(false)");
    let light = at(&calls, "light_refresh");
    let input = at(&calls, "input_open");
    check!(open == Some(0));
    check!(open < wake && wake < light && light < input);
    // the end, in main's order
    let n = calls.len();
    check!(n >= 7);
    if n >= 3 {
        check!(calls[n - 3] == "display_wait" && calls[n - 2] == "input_close" && calls[n - 1] == "display_close");
    }
    // each opened and closed once
    for what in ["display_open", "input_open", "input_close", "display_close"] {
        check!(calls.iter().filter(|c| **c == what).count() == 1);
    }

    // the first frame went to the platform: the whole screen, before the first poll returned
    let presents = REC.presents.lock().unwrap().clone();
    check!(!presents.is_empty());
    if let Some((r, _mode)) = presents.first() {
        check!(r.x == 0 && r.y == 0 && r.w == W && r.h == H);
    }
    // and the loop polled until the stand-in said quit, no further
    check!(REC.polls.load(Ordering::Relaxed) == POLLS + 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_platform_external() {
    run!(no_platform_until_one_is_registered);
    run!(one_platform_is_registered_once);
    run!(the_app_runs_on_it_from_start_to_exit);
    done();
}
