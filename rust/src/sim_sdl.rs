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
use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::ffi::OsStrExt;
use std::sync::atomic::Ordering;

// Exercise the evdev frame adapters on the host as well as cross-checking the Kindle build.
#[cfg(test)]
#[allow(dead_code)]
#[path = "input_evdev.rs"]
mod evdev_tests;

// ---- shared event queue ----

const QN: usize = 8192;

// The refreshes sent since the script last checked "display.presents" (display_probe): what one
// input put on the panel, in order, the first few of them where and how, and how many in all.
const SIM_PRESENTS_KEPT: usize = 8;
#[derive(Clone, Copy, Default)]
struct SimPresent {
    mode: RefreshMode,
    r: Rect,
}

// ---- expect: checks in a script ----
//
// "expect KEY is VALUE" (exact), "expect KEY has TEXT" (substring), "expect KEY not VALUE". The
// value is the rest of the line after one space; nothing there means the empty string. KEY is
// anything app_probe() answers (docs/SIM.md lists them), "running": 1 while the app runs, 0
// once it has quit, "watching": 1 while input_poll also watches a file descriptor (the share
// server's), else 0, or a display.* key (display_probe). A check sees the app once everything
// queued before it was handled, ticked and flushed: at once after a line that ends a batch (a
// tap, wait or shot; that is one main-loop pass before a shot in its place would be taken), else
// at the next poll, like a shot. A failed check is printed and the run goes on; at the end the
// run exits with status 3 (display_close).
// A line the runner does not know fails the same way, or a misspelt check would pass unseen.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ExpectOp {
    #[default]
    Is, // EXPECT_IS
    Has, // EXPECT_HAS
    Not, // EXPECT_NOT
}
const K_EXPECT_OPS: [&str; 3] = ["is", "has", "not"];
const K_EXPECT_OP_VALUES: [ExpectOp; 3] = [ExpectOp::Is, ExpectOp::Has, ExpectOp::Not];

// (The C's key[64] and value[512]: the script's bytes, which need not be UTF-8.)
const EXPECT_KEY_CAP: usize = 64;
const EXPECT_VALUE_CAP: usize = 512;

#[derive(Default)]
struct Expect {
    /// read, not checked yet
    pending: bool,
    /// the line could not be read: it fails when its turn comes
    bad: bool,
    line: i32,
    op: ExpectOp,
    key: Vec<u8>,
    value: Vec<u8>,
}

// What a check's probe may answer (the C's char got[512]).
const GOT_CAP: usize = 512;
// The script's lines, as fgets reads them into char line[512].
const LINE_CAP: usize = 512;

/// sim_sdl.c's file-level statics (PORTING.md, Global state).
struct State {
    w: i32,
    h: i32,
    headless: bool,

    // the shared event queue
    q: Vec<InEvent>,
    qh: usize,
    qt: usize,

    pen: InputCalib,
    touch: InputCalib,
    watch_fd: i32,
    /// quarter turns clockwise the screen is turned from the panel (input_set_orientation)
    rot: i32,
    shortcuts_enabled: bool,
    // The script's keyboards and controllers ("hid attach", "hid detach"): connected whether or not
    // shortcuts are on, but watched, listed and announced only while they are, as on the Kindle.
    hids: [InputHid; 4],
    nhids: usize,

    // input_sample_hook's (Live view's): called for each pen event as it is queued, and before
    // each event is taken. They run under this module's borrow: by input.h's rule they only store
    // into memory (app's and live's), so they never reach the runner.
    sample_fn: Option<fn(&mut InEvent)>,
    sample_batch: Option<fn()>,
    // A stand-in for the raw digitizer's node as the main loop reads it (GRIMOIRE_SIM_RAWPEN_MAIN, a
    // FIFO of input_event records in the host's layout, x and y in screen pixels, pressure 0..4095:
    // tests/live_pen_stall.py writes it): its frames become pen events as the Kindle's would, each
    // with its kernel time. raw_stall_ms (GRIMOIRE_SIM_STALL_MS): after each read that brought
    // frames, the main loop is held that long, as an expensive ink frame holds it on the Kindle.
    raw_fd: i32,
    raw_pen: LiveRawPen,
    raw_touching: bool,
    raw_stall_ms: i32,

    // ---- display ----
    /// g_last_screen: whether anything was presented yet (a shot before that writes nothing)
    presented: bool,
    /// display_set_invert
    sim_invert: bool,
    /// what the simulated panel shows
    eink: Vec<u8>,
    stats: [i32; 3],
    ink_frames: i32,
    area: [i64; 3],
    // Live ink as display_fbink.c keeps it, for the script's display.* checks: whether a colour panel
    // would wait after the last fast refresh, and whether the app asks for the pen mode.
    ink: LiveInk,
    /// the last fast refresh: 1 live ink, 0 one to wait after, -1 none yet
    last_fast: i32,
    /// and where it was
    last_fast_rect: Rect,
    /// display_pen_mode
    pen_mode: bool,
    presents: [SimPresent; SIM_PRESENTS_KEPT],
    npresents: i32,

    // Simulated front light (the simulator has no light to drive).
    light_i: i32,
    light_w: i32,
    // The simulated power service's state, what sys_powerd_state reports: set by the script's
    // `powerd <state>` ("unknown" reads as unreadable). It never changes on its own.
    powerd_state: FixedStr<32>,
    // (what input_set_grab was told; only the SDL window, not ported, would read it)
    #[allow(dead_code)]
    sim_grab: bool,
    // How often the app told the power service someone is here: the runner's "power.pokes".
    pokes: i32,
    // The clock and battery corner's inputs (status.rs): the script's `clock` and `battery`. The
    // clock stays where the script put it (None: the real one); the battery starts full.
    clock: Option<i64>,
    battery: Battery,

    // ---- script mode ----
    script: Option<BufReader<File>>,
    wait_until: u32,
    /// hold's PATH: the script resumes once that file exists ("" for none)
    hold_file: FixedStr<256>,
    /// external test driver owns lifetime after the hold command
    script_hold: bool,
    /// waitbg: the wait ends early once no background job is queued or running
    wait_bg: bool,
    pending_shot: FixedStr<128>,
    script_done: bool,
    /// last pen position
    px: f32,
    py: f32,
    side: bool,
    side2: bool,
    pen_button2: bool,
    /// The click helper owns the queue until its last scheduled release is delivered.
    side_click_until: Option<u32>,
    raw_side2: bool,
    raw_side_reported: [bool; 2],
    eraser: bool,

    expect: Expect,
    /// the script line being read (1-based)
    line_no: i32,
    /// a line read too early and given back (`tap panel` with events still queued): the next
    /// script_line returns it again
    unread: Option<Vec<u8>>,
    expect_total: i32,
    expect_failed: i32,

    // The simulated power key's press (power down), for its release's time held.
    power_down_at: u32,
    // "axis": the controller's state and triggers, decided once (the C's statics in script_step).
    axis_keys: HidKeys,
    axis_trig: HidTriggers,

    // GRIMOIRE_SIM_PRESSURE: the SDL window's mouse pressure (that window is not ported, so
    // nothing reads it; input_open still takes it as the C does).
    #[allow(dead_code)]
    pressure: f32,

    // ---- contact-level touch (finger, frame, swipe, tapn): through the same touchgest as the Kindle ----
    tg: TouchGest,
    /// the frame being built: finger ID (1..16) -> its contact (a finger line's)
    fingers: [Option<TgContact>; TG_MAX + 1],
    /// the frame clock: the next frame's time, and the last frame's. A batch's first contact line
    /// starts it at now_ms() if that is later; frame MS moves it on by MS, so a contact sequence is
    /// timed by its offsets whatever expects or waits split it, and never goes back.
    fclock: u32,
    flast: u32,
    fclock_fresh: bool,
    /// the pen as touchgest sees it, from the script's pen lines
    tpen: TgPen,
}

impl Default for State {
    fn default() -> Self {
        State {
            w: 1860,
            h: 2480,
            headless: false,
            q: vec![InEvent::default(); QN],
            qh: 0,
            qt: 0,
            pen: InputCalib::default(),
            touch: InputCalib::default(),
            watch_fd: -1,
            rot: 0,
            shortcuts_enabled: false,
            hids: [InputHid::default(); 4],
            nhids: 0,
            sample_fn: None,
            sample_batch: None,
            raw_fd: -1,
            raw_pen: LiveRawPen::default(),
            raw_touching: false,
            raw_stall_ms: 0,
            presented: false,
            sim_invert: false,
            eink: Vec::new(),
            stats: [0; 3],
            ink_frames: 0,
            area: [0; 3],
            ink: LiveInk::default(),
            last_fast: -1,
            last_fast_rect: RECT_EMPTY,
            pen_mode: false,
            presents: [SimPresent::default(); SIM_PRESENTS_KEPT],
            npresents: 0,
            light_i: 12,
            light_w: 8,
            powerd_state: FixedStr::from_str("active"),
            sim_grab: true,
            pokes: 0,
            clock: None,
            battery: Battery { percent: 100, charging: false },
            script: None,
            wait_until: 0,
            hold_file: FixedStr::new(),
            script_hold: false,
            wait_bg: false,
            pending_shot: FixedStr::new(),
            script_done: false,
            px: 0.0,
            py: 0.0,
            side: false,
            side2: false,
            pen_button2: false,
            side_click_until: None,
            raw_side2: false,
            raw_side_reported: [false; 2],
            eraser: false,
            expect: Expect::default(),
            line_no: 0,
            unread: None,
            expect_total: 0,
            expect_failed: 0,
            power_down_at: 0,
            axis_keys: HidKeys::default(),
            axis_trig: HidTriggers::default(),
            pressure: 0.6,
            tg: TouchGest::default(),
            fingers: [None; TG_MAX + 1],
            fclock: 0,
            flast: 0,
            fclock_fresh: true,
            tpen: TgPen::default(),
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
    // The work pump's count, apart from the rest: work_pump can run from anywhere in the app's long
    // paths, so its hook takes no other borrow.
    static G_PUMPS: Cell<i32> = const { Cell::new(0) };
    // main.longest_unpumped_ms, the same way: when input was last read (a poll, a pump, the end of a
    // present or a save), and the longest stretch without since the key was last checked.
    static G_LAST_READ: Cell<u32> = const { Cell::new(0) };
    static G_UNPUMPED: Cell<u32> = const { Cell::new(0) };
}

/// The runner's bound for main.longest_unpumped_ms: host time of a debug build on a shared machine,
/// so it catches a missing pump (a whole page read or drawn unpumped), not the Kindle's ~100 ms.
const UNPUMPED_OK_MS: u32 = 250;

/// Input is read now: the stretch since it was last read counts toward main.longest_unpumped_ms.
fn unpumped_note() {
    let now = now_ms();
    let gap = now.wrapping_sub(G_LAST_READ.with(|c| c.get()));
    G_UNPUMPED.with(|c| c.set(c.get().max(gap)));
    G_LAST_READ.with(|c| c.set(now));
}

/// A present or a save ended (work_pump_mark): its time is not counted.
fn unpumped_mark() {
    G_LAST_READ.with(|c| c.set(now_ms()));
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

impl State {
    fn q_push(&mut self, mut e: InEvent) {
        let n = (self.qt + 1) % QN;
        if n == self.qh {
            return; // full: drop
        }
        let t = e.type_ as i32;
        if let Some(f) = self.sample_fn {
            if t >= InType::PenDown as i32 && t <= InType::PenLeave as i32 {
                f(&mut e);
            }
        }
        self.q[self.qt] = e;
        self.qt = n;
    }
    fn q_pop(&mut self, e: &mut InEvent) -> bool {
        self.q_pop_at(e, now_ms())
    }
    fn q_pop_at(&mut self, e: &mut InEvent, now: u32) -> bool {
        if self.qh == self.qt {
            return false;
        }
        // Never hand a future button press to the wall-clock hold timer. If a stall
        // passed a queued release, it is drained before the app gets an idle tick.
        if self.side_click_until.is_some() && (now.wrapping_sub(self.q[self.qh].t_ms) as i32) < 0 {
            return false;
        }
        if let Some(b) = self.sample_batch {
            b();
        }
        *e = self.q[self.qh];
        self.qh = (self.qh + 1) % QN;
        if self.qh == self.qt { self.side_click_until = None; }
        true
    }

    // The simulated panel is the screen as it is upright: its sides swap while the Kindle is sideways.
    fn panel_size(&self, pw: &mut f32, ph: &mut f32) {
        *pw = (if self.rot & 1 != 0 { self.h } else { self.w }) as f32;
        *ph = (if self.rot & 1 != 0 { self.w } else { self.h }) as f32;
    }

    fn input_default_calib(&self, pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
        let (mut pw, mut ph) = (0.0, 0.0);
        self.panel_size(&mut pw, &mut ph);
        let c = InputCalib { valid: false, swap_xy: false, ax: pw, bx: 0.0, ay: ph, by: 0.0 };
        if let Some(pen) = pen {
            *pen = c;
        }
        if let Some(touch) = touch {
            *touch = c;
        }
    }

    // Panel <-> screen for a Kindle turned g_rot quarter turns clockwise, as in input_evdev.c.
    // display_set_orientation has already swapped g_w and g_h when this is set.
    fn panel_to_logical(&self, px: f32, py: f32, lx: &mut f32, ly: &mut f32) {
        let (mut pw, mut ph) = (0.0f32, 0.0f32);
        self.panel_size(&mut pw, &mut ph);
        let (big_w, big_h) = (pw - 1.0, ph - 1.0);
        match self.rot & 3 {
            1 => {
                *lx = big_h - py;
                *ly = px;
            }
            2 => {
                *lx = big_w - px;
                *ly = big_h - py;
            }
            3 => {
                *lx = py;
                *ly = big_w - px;
            }
            _ => {
                *lx = px;
                *ly = py;
            }
        }
    }
    fn input_logical_to_panel(&self, lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
        let (mut pw, mut ph) = (0.0f32, 0.0f32);
        self.panel_size(&mut pw, &mut ph);
        let (big_w, big_h) = (pw - 1.0, ph - 1.0);
        match self.rot & 3 {
            1 => {
                *px = ly;
                *py = big_h - lx;
            }
            2 => {
                *px = big_w - lx;
                *py = big_h - ly;
            }
            3 => {
                *px = big_w - ly;
                *py = lx;
            }
            _ => {
                *px = lx;
                *py = ly;
            }
        }
    }

    // Mouse and script coordinates are screen pixels. Like the Kindle's digitizer, the sim reports
    // "device" coordinates as a place on the upright panel (0..1), which the calibration maps to
    // panel pixels, turned to the screen after: a calibration made upright stays right sideways, and
    // the calibration math is exercised.
    fn make_ev(&self, t: InType, sx: f32, sy: f32, pen: bool) -> InEvent {
        let mut e = InEvent { type_: t, ..Default::default() };
        let (mut px, mut py, mut pw, mut ph) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        self.input_logical_to_panel(sx, sy, &mut px, &mut py);
        self.panel_size(&mut pw, &mut ph);
        e.nx = px / pw;
        e.ny = py / ph;
        calib_apply(if pen { &self.pen } else { &self.touch }, e.nx, e.ny, &mut px, &mut py);
        let (mut x, mut y) = (0.0f32, 0.0f32);
        self.panel_to_logical(px, py, &mut x, &mut y);
        e.x = x;
        e.y = y;
        e.t_ms = now_ms();
        if pen && matches!(t, InType::PenDown | InType::PenMove | InType::PenUp | InType::PenHover | InType::PenLeave | InType::PenButton) {
            e.side_button = self.side;
            e.side_button2 = self.side2;
        }
        e
    }

    fn pen_ev(&mut self, t: InType, x: f32, y: f32, p: f32, tx: f32, ty: f32, dt: u32) -> u32 {
        let mut e = self.make_ev(t, x, y, true);
        e.pressure = p;
        e.tilt_x = tx;
        e.tilt_y = ty;
        e.eraser = self.eraser;
        e.side_button = self.side;
        e.side_button2 = self.side2;
        e.t_ms = e.t_ms.wrapping_add(dt);
        self.pen_arrives(&e);
        let t_ms = e.t_ms;
        self.q_push(e);
        self.px = x;
        self.py = y;
        t_ms
    }

    fn side_event(&mut self, second: bool, down: bool, at: u32) {
        if second { self.side2 = down; } else { self.side = down; }
        let mut e = self.make_ev(InType::PenButton, self.px, self.py, true);
        e.value = if second { 2 } else { 0 };
        e.t_ms = at;
        self.q_push(e);
    }

    /// N clicks at one hovering point, paced on the same wrapping clock as input_poll.
    fn side_click(&mut self, second: bool, count: u32, gap: u32, press: u32) {
        let at = now_ms();
        let mut hover = self.make_ev(InType::PenHover, self.px, self.py, true);
        hover.eraser = self.eraser;
        hover.t_ms = at; // one clock read for the whole sequence, including its hover
        if self.tpen.touching {
            let mut up = hover;
            up.type_ = InType::PenUp;
            self.pen_arrives(&up);
            self.q_push(up);
        }
        self.pen_arrives(&hover);
        self.q_push(hover);
        let mut t = at;
        for i in 0..count {
            self.side_event(second, true, t);
            t = t.wrapping_add(press);
            self.side_event(second, false, t);
            if i + 1 < count { t = t.wrapping_add(gap); }
        }
        self.side_click_until = Some(t);
    }

    // A pen event about to be queued (the script's, or the stand-in raw node's): the pen ends what
    // fingers are doing (the Kindle's pen_frame arrival rule), before its event ...
    fn pen_arrives(&mut self, e: &InEvent) {
        let arrives = matches!(e.type_, InType::PenDown | InType::PenMove)
            || (e.type_ == InType::PenHover && !self.tpen.prox);
        if arrives && touchgest_busy(&self.tg) {
            let mut out = TgOut::default();
            let k = self.tg_now();
            touchgest_cancel(&mut self.tg, k, &mut out);
            for c in &out.ev[..out.n] {
                self.q_push(*c);
            }
        }
        // ... and touchgest sees it near, touching, or gone.
        let at = self.tg_now();
        match e.type_ {
            InType::PenDown | InType::PenMove => {
                self.tpen.prox = true;
                self.tpen.touching = true;
                self.tpen.last = (e.x, e.y);
            self.tpen.hover = None;
            }
            InType::PenUp => {
                self.tpen.prox = true;
                self.tpen.touching = false;
                self.tpen.hover = None;
            }
            InType::PenHover => {
                self.tpen.prox = true;
                self.tpen.touching = false;
                self.tpen.last = (e.x, e.y);
                self.tpen.hover = Some((e.x, e.y, at));
            }
            InType::PenLeave => {
                self.tpen.prox = false;
                self.tpen.touching = false;
                self.tpen.left_ms = at;
                self.tpen.hover = None;
            }
            _ => {}
        }
    }

    /// The frame clock's now: where the last frame was, or now_ms() if that is later.
    fn tg_now(&self) -> u32 {
        let now = now_ms();
        if (now.wrapping_sub(self.flast) as i32) > 0 {
            now
        } else {
            self.flast
        }
    }

    /// A contact line's frame clock: the batch's first starts it at now_ms() if that is later.
    fn fclock_start(&mut self) {
        if self.fclock_fresh {
            let now = now_ms();
            if (now.wrapping_sub(self.fclock) as i32) > 0 {
                self.fclock = now;
            }
            self.fclock_fresh = false;
        }
    }

    /// A finger line's contact: its place as make_ev makes it (the calibration, the turn).
    fn finger_contact(&self, id: i32, x: f32, y: f32, major: i32, tool: u8) -> TgContact {
        let e = self.make_ev(InType::TouchMove, x, y, false);
        TgContact { id, slot: id, x: e.x, y: e.y, nx: e.nx, ny: e.ny, major, tool, pressure: 0 }
    }

    /// frame MS: the fingers down now go through touchgest at the frame clock, its events are queued
    /// (timed by the frame), and the clock moves on by MS.
    fn tg_frame(&mut self, ms: u32) {
        self.fclock_start();
        let mut f = TgFrame { k_ms: self.fclock, ..Default::default() };
        for c in self.fingers.iter().flatten() {
            f.c[f.n] = *c;
            f.n += 1;
        }
        let mut out = TgOut::default();
        touchgest_frame(&mut self.tg, &f, &self.tpen, &mut out);
        for e in &out.ev[..out.n] {
            self.q_push(*e);
        }
        self.flast = self.fclock;
        self.fclock = self.fclock.wrapping_add(ms);
    }

    /// swipe N X Y DX DY STEPS SEP LANDGAP: N fingers centred on (X, Y), SEP px apart across the
    /// motion, landing LANDGAP ms apart, moving by (DX, DY) over STEPS frames 20 ms apart, then
    /// lifting one frame (5 ms) apart.
    fn tg_swipe(&mut self, n: usize, x: f32, y: f32, dx: f32, dy: f32, steps: i32, sep: f32, gap: u32) {
        let l = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = if l > 0.0 { (-dy / l, dx / l) } else { (1.0, 0.0) };
        let at = |i: usize, f: f32| {
            let o = (i as f32 - (n - 1) as f32 * 0.5) * sep;
            (fmla(ux, o, x) + dx * f, fmla(uy, o, y) + dy * f)
        };
        for i in 0..n {
            let (px, py) = at(i, 0.0);
            self.fingers[i + 1] = Some(self.finger_contact(i as i32 + 1, px, py, 0, 0));
            self.tg_frame(if i + 1 < n { gap } else { 20 });
        }
        for s in 1..=steps {
            let f = s as f32 / steps as f32;
            for i in 0..n {
                let (px, py) = at(i, f);
                self.fingers[i + 1] = Some(self.finger_contact(i as i32 + 1, px, py, 0, 0));
            }
            self.tg_frame(if s < steps { 20 } else { 5 });
        }
        for i in 0..n {
            self.fingers[i + 1] = None;
            self.tg_frame(5);
        }
    }

    /// tapn N X Y SEP MS: N fingers in a row centred on (X, Y), SEP px apart (each kept on the
    /// screen), landing one frame each 10 ms apart, still for MS, lifting one frame each 10 ms apart.
    fn tg_tapn(&mut self, n: usize, x: f32, y: f32, sep: f32, ms: u32) {
        for i in 0..n {
            let px = clampf(fmla(i as f32 - (n - 1) as f32 * 0.5, sep, x), 0.0, (self.w - 1) as f32);
            self.fingers[i + 1] = Some(self.finger_contact(i as i32 + 1, px, y, 0, 0));
            self.tg_frame(if i + 1 < n { 10 } else { ms });
        }
        for i in 0..n {
            self.fingers[i + 1] = None;
            self.tg_frame(10);
        }
    }

    /// A synthetic one-finger path, on the contact-frame clock used by finger/frame/swipe/tapn.
    /// The first point lands at the current frame time; later points keep their supplied offsets.
    fn tg_path(&mut self, points: &[(u32, f32, f32)]) {
        if points.is_empty() {
            return;
        }
        self.fingers = [None; TG_MAX + 1];
        self.fclock_start();
        let mut previous = points[0].0;
        for (i, &(at, x, y)) in points.iter().enumerate() {
            self.fingers[1] = Some(self.finger_contact(1, x, y, 0, 0));
            self.tg_frame(if i == 0 { 0 } else { at.wrapping_sub(previous) });
            previous = at;
        }
        self.fingers[1] = None;
        self.tg_frame(13);
    }

    /// A polyline kept exact at every supplied vertex, sampled every 13 ms between its vertices.
    fn tg_fpath(&mut self, xy: &[f32], ms: u32) {
        if xy.len() < 4 || xy.len() & 1 != 0 || ms == 0 {
            return;
        }
        let n = xy.len() / 2;
        let mut lengths = vec![0.0f32; n];
        for i in 1..n {
            let dx = xy[2 * i] - xy[2 * (i - 1)];
            let dy = xy[2 * i + 1] - xy[2 * (i - 1) + 1];
            lengths[i] = lengths[i - 1] + (dx * dx + dy * dy).sqrt();
        }
        let total = lengths[n - 1];
        let mut times = Vec::with_capacity((ms / 13 + 1) as usize + n);
        let mut t = 0;
        while t < ms {
            times.push(t);
            t = t.saturating_add(13);
        }
        times.push(ms);
        for i in 1..n - 1 {
            let at = if total > 0.0 { (lengths[i] / total * ms as f32).round_ties_even() as u32 } else { (i as u32 * ms) / (n as u32 - 1) };
            times.push(at.min(ms));
        }
        times.sort_unstable();
        times.dedup();
        let mut points = Vec::with_capacity(times.len());
        for at in times {
            let d = if total > 0.0 { total * at as f32 / ms as f32 } else { total };
            let mut i = 1;
            while i < n - 1 && lengths[i] < d {
                i += 1;
            }
            let (x, y) = if total == 0.0 {
                (xy[0], xy[1])
            } else {
                let span = lengths[i] - lengths[i - 1];
                let f = if span > 0.0 { ((d - lengths[i - 1]) / span).clamp(0.0, 1.0) } else { 0.0 };
                (lerpf(xy[(i - 1) * 2], xy[i * 2], f), lerpf(xy[(i - 1) * 2 + 1], xy[i * 2 + 1], f))
            };
            points.push((at, x, y));
        }
        self.tg_path(&points);
    }

    /// Replay trace records through the same recognizer and monotonic clock as contact commands.
    fn tg_trace(&mut self, text: &str) -> Result<(), String> {
        let mut source_origin = None;
        let target_origin = self.tg_now();
        let mut last_k = target_origin;
        for (line_no, raw) in text.lines().enumerate() {
            let line = raw.trim_end_matches('\r');
            let parsed = touchgest_trace_parse_line(line).map_err(|e| format!("line {}: {e:?}", line_no + 1))?;
            match parsed {
                TgTraceLine::Ignore | TgTraceLine::Header { .. } => {}
                TgTraceLine::Outcome { k_ms, .. } => {
                    let origin = *source_origin.get_or_insert(k_ms);
                    let at = target_origin.wrapping_add(k_ms.wrapping_sub(origin));
                    self.tg_trace_ticks(&mut last_k, at);
                    self.flast = at;
                    self.fclock = at;
                }
                TgTraceLine::Pen { k_ms, what } => {
                    let origin = *source_origin.get_or_insert(k_ms);
                    let at = target_origin.wrapping_add(k_ms.wrapping_sub(origin));
                    self.tg_trace_ticks(&mut last_k, at);
                    self.tpen.hover = None;
                    self.tpen.last = (-1_000_000.0, -1_000_000.0);
                    match what {
                        TgTracePen::Prox => { self.tpen.prox = true; self.tpen.touching = false; }
                        TgTracePen::Down => { self.tpen.prox = true; self.tpen.touching = true; }
                        TgTracePen::Up => { self.tpen.prox = true; self.tpen.touching = false; }
                        TgTracePen::Leave => { self.tpen.prox = false; self.tpen.touching = false; self.tpen.left_ms = at; }
                    }
                    if what != TgTracePen::Leave && touchgest_busy(&self.tg) {
                        let mut out = TgOut::default();
                        touchgest_cancel(&mut self.tg, at, &mut out);
                        for event in &out.ev[..out.n] { self.q_push(*event); }
                    }
                    self.flast = at;
                    self.fclock = at;
                }
                TgTraceLine::Frame(mut frame) => {
                    let origin = *source_origin.get_or_insert(frame.k_ms);
                    let at = target_origin.wrapping_add(frame.k_ms.wrapping_sub(origin));
                    self.tg_trace_ticks(&mut last_k, at);
                    frame.k_ms = at;
                    let mut out = TgOut::default();
                    touchgest_frame(&mut self.tg, &frame, &self.tpen, &mut out);
                    for event in &out.ev[..out.n] { self.q_push(*event); }
                    self.fingers = [None; TG_MAX + 1];
                    for c in &frame.c[..frame.n.min(TG_MAX)] {
                        let slot = (c.slot + 1).clamp(1, TG_MAX as i32) as usize;
                        self.fingers[slot] = Some(*c);
                    }
                    self.flast = at;
                    self.fclock = at;
                }
            }
        }
        self.fclock_fresh = true;
        Ok(())
    }

    fn tg_trace_ticks(&mut self, from: &mut u32, to: u32) {
        let mut gap = to.wrapping_sub(*from);
        if gap as i32 <= 0 { return; }
        while gap >= 10 {
            *from = from.wrapping_add(10);
            let mut out = TgOut::default();
            touchgest_tick(&mut self.tg, *from, &mut out);
            for event in &out.ev[..out.n] { self.q_push(*event); }
            gap -= 10;
        }
        *from = to;
    }

    fn touch_ev(&mut self, t: InType, x: f32, y: f32) {
        let e = self.make_ev(t, x, y, false);
        self.q_push(e);
    }

    /// What `tap X Y` queues (a finger down and up), or `pentap X Y` (the nib down, then up 20 ms later).
    fn tap_events(&mut self, x: f32, y: f32, pen: bool) {
        if !pen {
            self.touch_ev(InType::TouchDown, x, y);
            self.touch_ev(InType::TouchUp, x, y);
        } else {
            self.pen_ev(InType::PenDown, x, y, 0.5, 0.0, 0.0, 0);
            let up = self.pen_ev(InType::PenUp, x, y, 0.0, 0.0, 0.0, 20);
            // A nib tap lifts away: touchgest sees the pen leave at its up (its events are as
            // they were: no PenLeave is queued).
            self.tpen.prox = false;
            self.tpen.left_ms = up;
        }
    }

    // The simulated power key: one event, its value how long the key was held (IN_POWER_UP and
    // IN_POWER_HELD, as input_evdev.c reports them).
    fn power_key(&mut self, t: InType, held_ms: i32) {
        let mut e = self.make_ev(t, 0.0, 0.0, true);
        e.value = held_ms;
        self.q_push(e);
    }

    /// The power service's state as powerd_parse_state reads it. (A state the script gave in bytes
    /// that are not UTF-8 is no name it knows, as in the C: "?" says so.)
    fn powerd_now(&self) -> PowerdState {
        powerd_parse_state(Some(std::str::from_utf8(self.powerd_state.as_bytes()).unwrap_or("?")))
    }

    /// The script's next line as fgets reads it, counted.
    fn script_line(&mut self) -> Option<Vec<u8>> {
        if let Some(line) = self.unread.take() {
            self.line_no += 1;
            return Some(line);
        }
        let line = fgets(self.script.as_mut()?, LINE_CAP)?;
        self.line_no += 1;
        Some(line)
    }
}

/// fprintf(stderr, ...): the runner's messages, written as they are (the script's own bytes among
/// them).
fn err(b: &[u8]) {
    let _ = std::io::stderr().write_all(b);
}

/// An argument of bformat: the script's bytes (%s) or a number (%d).
enum B<'a> {
    S(&'a [u8]),
    D(i32),
}

/// The runner's messages that print the script's bytes (a line, a key, a value, a path, which need
/// not be UTF-8) keep the C's format strings: this formats their %s (bytes, as they are) and %d as
/// printf does. Every other message goes through cformat!.
fn bformat(fmt: &str, args: &[B]) -> Vec<u8> {
    let f = fmt.as_bytes();
    let mut out = Vec::with_capacity(f.len() + 64);
    let mut next = 0;
    let mut i = 0;
    while i < f.len() {
        if f[i] == b'%' && i + 1 < f.len() && (f[i + 1] == b's' || f[i + 1] == b'd') {
            match args.get(next) {
                Some(B::S(s)) => out.extend_from_slice(s),
                Some(B::D(d)) => out.extend_from_slice(cformat!("%d", *d).as_bytes()),
                None => {}
            }
            next += 1;
            i += 2;
            continue;
        }
        if f[i] == b'%' && i + 1 < f.len() && f[i + 1] == b'%' {
            out.push(b'%');
            i += 2;
            continue;
        }
        out.push(f[i]);
        i += 1;
    }
    out
}

// ---- display ----

fn parse_size(st: &mut State) {
    let Some(s) = std::env::var_os("GRIMOIRE_SIM_SIZE") else { return };
    // sscanf(s, "%dx%d", &w, &h) == 2
    let mut sc = Scan::new(s.as_bytes());
    let Some(w) = sc.int() else { return };
    if !sc.literal(b'x') {
        return;
    }
    let Some(h) = sc.int() else { return };
    if w > 100 && h > 100 {
        st.w = w;
        st.h = h;
    }
}

/// A white panel of w x h (the C's malloc and memset); None when the memory is not there.
fn white_panel(w: i32, h: i32) -> Option<Vec<u8>> {
    let n = w as usize * h as usize;
    let mut v = Vec::new();
    v.try_reserve_exact(n).ok()?;
    v.resize(n, 255);
    Some(v)
}

pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    let ok = with(|st| {
        parse_size(st);
        st.headless = std::env::var_os("GRIMOIRE_SCRIPT").is_some();
        let Some(eink) = white_panel(st.w, st.h) else { return false };
        st.eink = eink;
        *w = st.w;
        *h = st.h;
        true
    });
    if !ok {
        return -1;
    }
    *dpi = 300.0;
    // (grimoire-headless is the only build of this file: no SDL window to open instead)
    if !with(|st| st.headless) {
        err(b"grimoire-headless needs GRIMOIRE_SCRIPT=<file> (use grimoire-sim for a window)\n");
        return -1;
    }
    0
}

// Same 8x8 ordered dither as display_fbink.c, so simulator shots look like the device.
fn sim_dither(v: u8, x: i32, y: i32) -> u8 {
    const B: [u8; 64] = [
        0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62,
        30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31,
        55, 23, 61, 29, 53, 21,
    ];
    if v == 0 || v == 255 {
        return v;
    }
    if v as i32 > B[(((y & 7) << 3) | (x & 7)) as usize] as i32 * 4 + 2 {
        255
    } else {
        0
    }
}

pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {
    with(|st| {
        st.presented = true;
        let r = rect_intersect(r, Rect { x: 0, y: 0, w: st.w, h: st.h });
        // (the C also leaves for a mode outside 0..2, which a RefreshMode cannot be)
        if rect_is_empty(r) {
            return;
        }
        if mode == RefreshMode::Fast {
            st.last_fast = if live_ink_frame(&mut st.ink, mode, now_ms()) { 1 } else { 0 };
        }
        if mode == RefreshMode::Fast {
            st.last_fast_rect = r;
        }
        if (st.npresents as usize) < SIM_PRESENTS_KEPT {
            st.presents[st.npresents as usize] = SimPresent { mode, r };
        }
        st.npresents += 1;
        st.stats[mode as usize] += 1;
        if mode == RefreshMode::Fast && st.ink.on {
            st.ink_frames += 1;
        }
        st.area[mode as usize] += r.w as i64 * r.h as i64;
        let (w, invert) = (st.w as usize, st.sim_invert);
        for y in r.y..r.y + r.h {
            let s = &screen.px[y as usize * screen.stride as usize..];
            let d = &mut st.eink[y as usize * w..];
            for x in r.x..r.x + r.w {
                let i = x as usize;
                d[i] = if mode == RefreshMode::Fast { sim_dither(s[i], x, y) } else { s[i] }; // DU is 1-bit
                if invert {
                    d[i] = 255 - d[i];
                }
            }
        }
    });
}

pub fn display_wait() {}

pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {
    with(|st| {
        // Portrait size is whichever of the configured dimensions is the narrow one.
        let pw = if st.w < st.h { st.w } else { st.h };
        let ph = if st.w < st.h { st.h } else { st.w };
        st.w = if quarter_turns & 1 != 0 { ph } else { pw };
        st.h = if quarter_turns & 1 != 0 { pw } else { ph };
        st.eink = Vec::new();
        // (a failed malloc leaves no panel, as in the C: the next present or shot then ends the run)
        st.eink = white_panel(st.w, st.h).unwrap_or_default();
        *w = st.w;
        *h = st.h;
    });
}
pub fn display_live_ink(on: bool) {
    with(|st| live_ink_mark(&mut st.ink, on, now_ms()));
}
pub fn display_pen_mode(on: bool) {
    display_live_ink(on);
    with(|st| st.pen_mode = on);
}
pub fn display_set_invert(on: bool) {
    with(|st| st.sim_invert = on);
}

// ---- light.h: the simulated front light and power service ----

pub fn light_available() -> bool {
    true
}
pub fn light_refresh() {}
pub fn light_get(intensity: &mut i32, warmth: &mut i32) {
    with(|st| {
        *intensity = st.light_i;
        *warmth = st.light_w;
    });
}
pub fn light_set_intensity(level: i32) {
    with(|st| st.light_i = clampi(level, 0, LIGHT_MAX));
}
pub fn light_set_warmth(level: i32) {
    with(|st| st.light_w = clampi(level, 0, LIGHT_MAX));
}
pub fn light_warmth() -> i32 {
    with(|st| st.light_w)
}
pub fn sys_powerd_state(out: &mut String, cap: usize) -> i32 {
    with(|st| {
        if st.powerd_state.is_empty() || st.powerd_state == "unknown" {
            return -1;
        }
        // (out is a String: a state the script gave in bytes that are not UTF-8 reads with U+FFFD)
        strlcpy_q(out, &String::from_utf8_lossy(st.powerd_state.as_bytes()), cap);
        0
    })
}
// Simulated sleep: the "power service" answers a sleep request straight away, like powerd takes a
// power button: an awake Kindle goes to sleep, one already in its screensaver (by `powerd`) wakes.
pub fn sys_request_sleep() {
    with(|st| {
        let dozing = powerd_dozing(st.powerd_now());
        if dozing {
            err(&bformat(
                "sim: powerButton 1 while the power service is in %s: it wakes the Kindle\n",
                &[B::S(st.powerd_state.as_bytes())],
            ));
        }
        let e = InEvent {
            type_: if dozing { InType::Wake } else { InType::Sleep },
            value: if dozing { 1 } else { 2 },
            t_ms: now_ms(),
            ..Default::default()
        };
        st.q_push(e);
    });
}
pub fn sys_kindle_ui(run: bool) -> bool {
    let _ = run;
    true
}
pub fn input_set_grab(grab: bool) -> bool {
    with(|st| st.sim_grab = grab);
    true
}
// (Its hover is the mouse's, as IN_PEN_HOVER events: Live view takes it from app_event.)
pub fn input_hover_hook(f: Option<fn(f32, f32)>) {
    let _ = f;
}
// The raw digitizer a pen viewer reads (GRIMOIRE_SIM_RAWPEN: a FIFO of input_event records, x and y
// in screen pixels, pressure 0..4095, tilt in hundredths of a degree), placed as it is.
pub fn input_raw_pen(out: &mut LiveRawMap) -> bool {
    let Some(path) = std::env::var_os("GRIMOIRE_SIM_RAWPEN") else { return false };
    *out = LiveRawMap {
        m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        rmin: [0, 0],
        rmax: [32767, 32767],
        pmin: 0,
        pmax: 4095,
        tilt_min: [-9000, -9000],
        tilt_max: [9000, 9000],
        tm: [1.0, 0.0, 0.0, 1.0],
        ..Default::default()
    };
    out.path.set_bytes(path.as_bytes());
    !out.path.is_empty()
}
pub fn input_sample_hook(sample: Option<fn(&mut InEvent)>, batch: Option<fn()>) {
    with(|st| {
        st.sample_fn = sample;
        st.sample_batch = batch;
    });
}
pub fn sys_prevent_sleep(prevent: bool) {
    let _ = prevent;
}
pub fn sys_activity() {
    with(|st| st.pokes += 1);
}
pub fn sys_battery() -> Battery {
    with(|st| st.battery)
}
pub fn sys_battery_reap() {}
pub fn sys_wall_clock() -> i64 {
    with(|st| st.clock).unwrap_or_else(wall_time)
}
pub fn display_broken() -> bool {
    false
}
pub fn display_skip_overlay(r: Rect) {
    let _ = r;
}
pub fn display_skip_status(r: Rect) {
    let _ = r;
}
pub fn display_pen_mode_on() -> bool {
    with(|st| st.pen_mode)
}

pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    let _ = (screen, skip);
    *where_ = RECT_EMPTY;
    false
}
pub fn display_band_overwritten(screen: &Canvas, band: Rect, skip: Rect) -> bool {
    let _ = (screen, band, skip);
    false
}

// A headless run ends once the PDF exports it started are written (bgjob.c leaves them to child
// processes that outlive Grimoire), so what reads its data next (a scenario's "# root:" copy,
// share_check.sh, the exporter) never meets a file half-written. A minute at most.
fn wait_for_exports() {
    let mut said = false;
    let mut ms = 0;
    while ms < 60000 {
        // SAFETY: waitpid with no status pointer only reaps a child of this process, if one ended.
        let r = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        if r < 0 {
            return; // no child left
        }
        if r > 0 {
            continue; // one ended: any more?
        }
        if !said {
            err(b"headless: waiting for the PDF exports to finish\n");
        }
        said = true;
        sleep_ms(20);
        ms += 20;
    }
    err(b"headless: an export still runs after a minute; not waiting for it\n");
}

pub fn display_close() {
    let (headless, stats, area) = with(|st| (st.headless, st.stats, st.area));
    if headless || std::env::var_os("GRIMOIRE_SIM_STATS").is_some() {
        err(cformat!(
            "refreshes: fast %d (%.1f MP), ui %d (%.1f MP), full %d (%.1f MP)\n",
            stats[0],
            area[0] as f64 / 1e6,
            stats[1],
            area[1] as f64 / 1e6,
            stats[2],
            area[2] as f64 / 1e6
        )
        .as_bytes());
    }
    with(|st| st.eink = Vec::new());
    if headless {
        wait_for_exports();
    }
    expect_finish(); // exits with status 3 if a check failed
}

// ---- expect ----

impl State {
    // Reads "expect KEY OP VALUE" (s starts at "expect") into g_expect.
    fn expect_read(&mut self, s: &[u8]) {
        self.expect = Expect { pending: true, line: self.line_no, ..Default::default() };
        let at = |i: usize| -> u8 { s.get(i).copied().unwrap_or(0) };
        let mut k = 6;
        while at(k) == b' ' || at(k) == b'\t' {
            k += 1;
        }
        let kl = strcspn(&s[k.min(s.len())..], b" \t\r\n");
        let mut o = k + kl;
        while at(o) == b' ' || at(o) == b'\t' {
            o += 1;
        }
        let ol = strcspn(&s[o.min(s.len())..], b" \t\r\n");
        let mut v = o + ol;
        if at(v) == b' ' || at(v) == b'\t' {
            v += 1; // the one space before the value
        }
        let vl = strcspn(&s[v.min(s.len())..], b"\r\n");
        let mut op: i32 = -1;
        for (i, name) in K_EXPECT_OPS.iter().enumerate() {
            if ol == name.len() && &s[o..o + ol] == name.as_bytes() {
                op = i as i32;
            }
        }
        if kl == 0 || kl >= EXPECT_KEY_CAP || op < 0 || vl >= EXPECT_VALUE_CAP {
            self.expect.bad = true;
            return;
        }
        self.expect.key = s[k..k + kl].to_vec();
        self.expect.value = s[v..v + vl].to_vec();
        self.expect.op = K_EXPECT_OP_VALUES[op as usize];
    }

    // The display.* keys, which the simulated panel answers: "display.ink" 1 while a stroke is drawn
    // (display_live_ink), "display.pen_mode" 1 while the app asks for the pen mode (display_pen_mode),
    // "display.last_fast" "ink" when the last fast refresh was live ink, "wait" when a colour panel
    // would have waited for the driver's read after it (display_fbink.c), empty before any;
    // "display.last_fast_rect" where that refresh was, "x y w h", empty before any; "display.presents"
    // the refreshes sent since this key was last checked, each "fast", "ui" or "full" and its "x y w h",
    // joined by ", " ("fast 50 1510 1760 120, fast 138 1970 167 150"), with ", and N more" past the
    // eighth, empty for none; the check starts the count again, so a first one only clears it.
    fn display_probe(&mut self, key: &[u8], out: &mut String, cap: usize) -> bool {
        if key == b"display.presents" {
            const NAMES: [&str; 3] = ["fast", "ui", "full"];
            let mut n = 0usize;
            out.clear();
            let mut piece = String::new();
            let mut i = 0;
            while (i as i32) < self.npresents && i < SIM_PRESENTS_KEPT && n < cap {
                let r = self.presents[i].r;
                n += csnprintf!(
                    &mut piece,
                    cap - n,
                    "%s%s %d %d %d %d",
                    if i != 0 { ", " } else { "" },
                    NAMES[self.presents[i].mode as usize],
                    r.x,
                    r.y,
                    r.w,
                    r.h
                ) as usize;
                out.push_str(&piece);
                i += 1;
            }
            if self.npresents > SIM_PRESENTS_KEPT as i32 && n < cap {
                csnprintf!(&mut piece, cap - n, ", and %d more", self.npresents - SIM_PRESENTS_KEPT as i32);
                out.push_str(&piece);
            }
            self.npresents = 0;
        } else if key == b"display.last_fast_rect" {
            let r = self.last_fast_rect;
            if self.last_fast < 0 {
                csnprintf!(out, cap, "%s", "");
            } else {
                csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
            }
        } else if key == b"display.ink" {
            csnprintf!(out, cap, "%d", if self.ink.on { 1 } else { 0 });
        } else if key == b"display.pen_mode" {
            csnprintf!(out, cap, "%d", if self.pen_mode { 1 } else { 0 });
        } else if key == b"display.ink_frames" {
            csnprintf!(out, cap, "%d", self.ink_frames);
        } else if key == b"display.last_fast" {
            csnprintf!(
                out,
                cap,
                "%s",
                if self.last_fast < 0 {
                    ""
                } else if self.last_fast != 0 {
                    "ink"
                } else {
                    "wait"
                }
            );
        } else {
            return false;
        }
        true
    }

    // A line script_step does not know ("expcet", "Expect"): it fails like a check, since a misspelt
    // check is a check never made.
    fn expect_unknown_line(&mut self) {
        err(&bformat("expect failed (line %d): unknown command\n", &[B::D(self.line_no)]));
        self.expect_total += 1;
        self.expect_failed += 1;
    }
}

// "power.pokes": how many times the app told the simulated power service someone is here
// (sys_activity), which it does at most once a minute. "pump.calls": how many times long work called
// the work pump (work_pump, common.h) since this key was last checked, so a check counts what the
// lines before it did. "keyboard.draws": how many times an on-screen keyboard was drawn
// (keyboard_draw, keyboard.h) since this key was last checked, the same way.
fn power_probe(key: &[u8], out: &mut String, cap: usize) -> bool {
    if key.starts_with(b"touch.") {
        // touchgest's: what the contact lines' fingers are doing (touchgest_probe)
        let Ok(key) = std::str::from_utf8(key) else { return false };
        let mut got = String::new();
        if !with(|st| touchgest_probe(&st.tg, key, &mut got)) {
            return false;
        }
        csnprintf!(out, cap, "%s", &got);
        return true;
    }
    if key == b"main.longest_unpumped_ms" {
        // The longest stretch without reading input since the last check (it starts the count again):
        // "ok" under the runner's bound, else the stretch in ms (expect compares no numbers).
        let gap = G_UNPUMPED.with(|c| c.replace(0));
        if gap < UNPUMPED_OK_MS {
            csnprintf!(out, cap, "%s", "ok");
        } else {
            csnprintf!(out, cap, "%u", gap);
        }
        return true;
    }
    if key == b"pump.calls" {
        csnprintf!(out, cap, "%d", G_PUMPS.with(|p| p.replace(0)));
        return true;
    }
    if key == b"keyboard.draws" {
        csnprintf!(out, cap, "%u", keyboard_draws_take());
        return true;
    }
    if key != b"power.pokes" {
        return false;
    }
    csnprintf!(out, cap, "%d", with(|st| st.pokes));
    true
}

/// app_probe, which the C reaches through g_probe_app: the App the main loop handed the poll, none
/// before there is one (the C's NULL g_probe_app: -1). Every key app_probe answers is ASCII, so a
/// key in bytes that are not UTF-8 is unknown there too.
fn probe_app(a: Option<&mut App>, key: &[u8], out: &mut String, cap: usize) -> i32 {
    match (a, std::str::from_utf8(key)) {
        (Some(a), Ok(key)) => app_probe(a, key, out, cap),
        _ => -1,
    }
}

// Checks the expect just read. gone: the app quit before the script got to it, so only "running"
// has an answer (0); any other key fails as not run.
fn expect_check_as(gone: bool, a: Option<&mut App>) {
    let (bad, line, op, key, value, watch_fd) = with(|st| {
        st.expect.pending = false;
        st.expect_total += 1;
        (st.expect.bad, st.expect.line, st.expect.op, st.expect.key.clone(), st.expect.value.clone(), st.watch_fd)
    });
    let mut got = String::new();
    let running = key == b"running";
    let watching = key == b"watching";
    if bad {
        err(&bformat("expect failed (line %d): cannot read it (expect KEY is|has|not VALUE)\n", &[B::D(line)]));
    } else if gone && !running {
        err(&bformat("expect failed (line %d): not run (the app quit before the script got here)\n", &[B::D(line)]));
    } else if !running
        && !watching
        && !with(|st| st.display_probe(&key, &mut got, GOT_CAP))
        && !power_probe(&key, &mut got, GOT_CAP)
        && probe_app(a, &key, &mut got, GOT_CAP) != 0
    {
        err(&bformat("expect failed (line %d): unknown key \"%s\"\n", &[B::D(line), B::S(&key)]));
    } else {
        if running {
            csnprintf!(&mut got, GOT_CAP, "%d", if gone { 0 } else { 1 });
        }
        if watching {
            csnprintf!(&mut got, GOT_CAP, "%d", if watch_fd >= 0 { 1 } else { 0 });
        }
        let g = c_str(got.as_bytes());
        let ok = match op {
            ExpectOp::Is => g == value.as_slice(),
            ExpectOp::Has => strstr(g, &value),
            ExpectOp::Not => g != value.as_slice(),
        };
        if ok {
            return;
        }
        err(&bformat(
            "expect failed (line %d): %s is \"%s\", wanted %s \"%s\"\n",
            &[B::D(line), B::S(&key), B::S(g), B::S(K_EXPECT_OPS[op as usize].as_bytes()), B::S(&value)],
        ));
    }
    with(|st| st.expect_failed += 1);
}

fn expect_check(a: Option<&mut App>) {
    expect_check_as(false, a);
}

// The app quit before the script ended (Library > Exit, say): the checks it never got to fail
// too, or a tap that lands on Exit by mistake would let the rest of a script pass unseen. Where
// a script means the app to quit, "expect running is 0" says so.
fn expect_unrun() {
    if with(|st| st.expect.pending) {
        expect_check_as(true, None);
    }
    loop {
        let read = with(|st| {
            let line = st.script_line()?;
            let s = skip_space(c_str(&line));
            let is_expect = s.starts_with(b"expect") && (s.len() == 6 || isspace(s[6]));
            if is_expect {
                st.expect_read(s);
            }
            Some(is_expect)
        });
        match read {
            None => break,
            Some(true) => expect_check_as(true, None),
            Some(false) => {}
        }
    }
}

fn expect_finish() {
    let (total, failed) = with(|st| (st.expect_total, st.expect_failed));
    if total == 0 {
        return;
    }
    if failed == 0 {
        err(cformat!("expect: all %d checks passed\n", total).as_bytes());
        return;
    }
    err(cformat!("expect: %d of %d checks failed\n", failed, total).as_bytes());
    std::process::exit(3);
}

impl State {
    fn shot(&self, name: &[u8]) {
        if !self.presented {
            return;
        }
        let dir_env = std::env::var_os("GRIMOIRE_SHOTS");
        let dir: &[u8] = match &dir_env {
            Some(d) if !d.is_empty() => d.as_bytes(),
            _ => b"shots",
        };
        // snprintf into char path[700]
        let mut path = Vec::with_capacity(dir.len() + name.len() + 5);
        path.extend_from_slice(dir);
        path.push(b'/');
        path.extend_from_slice(name);
        path.extend_from_slice(b".png");
        path.truncate(699);
        // (mkdir_p takes a str: a GRIMOIRE_SHOTS in bytes that are not UTF-8 reaches it with U+FFFD)
        mkdir_p(&String::from_utf8_lossy(dir));
        // The panel state (with DU thresholding where it applies) is what a user would see.
        write_png(&path, self.w, self.h, &self.eink);
        err(&bformat("shot: %s\n", &[B::S(&path)]));
    }
}

/// stbi_write_png of the panel (one gray plane, w x h, stride w) to path.
fn write_png(path: &[u8], w: i32, h: i32, eink: &[u8]) {
    // The C hands stb whatever g_eink is: a panel whose malloc failed (display_set_orientation)
    // ends the run there, as this does.
    assert!(eink.len() >= w as usize * h as usize, "no panel to write");
    let Ok(cpath) = CString::new(path) else { return };
    // SAFETY: cpath is NUL-terminated; eink holds w * h bytes (checked above), read as h rows of w
    // bytes with a stride of w; stb only reads them.
    unsafe {
        crate::ffi_stb::stbi_write_png(cpath.as_ptr(), w, h, 1, eink.as_ptr() as *const libc::c_void, w);
    }
}

enum Step {
    /// a batch boundary (wait/shot/end): the app handles what is queued
    Batch,
    /// an expect with nothing queued since the last flush: check it now, then read on
    Check,
    /// `tap panel NAME` / `pentap panel NAME` (true: the pen): the panel's button NAME is found where the
    /// App is (script_step), then tapped like `tap` and `pentap` do, and the batch ends
    Panel(FixedStr<32>, bool),
    MapNode(u32, bool),
}

// Reads script lines until a batch boundary (wait/shot/end). Returns false at EOF.
// (In Rust: an expect to check at once returns Check, and script_step checks it, outside the
// state, as app_probe needs the App, then reads on from the next line, as the C's `continue` does.
// The curve's 6.2831853f and 3.14159f are the C's own constants, kept: they place its samples.)
#[allow(clippy::approx_constant)]
fn script_step_in(st: &mut State) -> Step {
    while let Some(line) = st.script_line() {
        let s = skip_space(c_str(&line));
        if s.is_empty() || s[0] == b'#' {
            continue;
        }
        let mut v = [0f32; 10];
        let mut sc = Scan::new(s);
        let a = sc.word(15);
        let b = sc.word(63);
        let n = a.is_some() as i32 + b.is_some() as i32;
        if n < 1 {
            continue;
        }
        let a = a.unwrap_or(b"");
        let b = b.unwrap_or(b"");
        if a == b"shape" {
            let (Some(x), Some(y), Some(size)) = (sc.int(), sc.int(), sc.int()) else {
                err(&bformat("script: bad shape line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            };
            let ms = sc.int().unwrap_or(900).max(1) as u32;
            let rot = sc.int().unwrap_or(0);
            let seed = sc.int().unwrap_or(1) as u32;
            let reverse = sc.word(16).is_some();
            let Ok(name) = std::str::from_utf8(b) else {
                err(&bformat("script: bad shape line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            };
            if !crate::fingershape_synth::SHAPES.contains(&name) || size <= 0 {
                err(&bformat("script: bad shape line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            let draw = crate::fingershape_synth::Draw { cx: x, cy: y, size_mm: size, ms, rot, seed, reverse, ..Default::default() };
            let frames = crate::fingershape_synth::shape_frames(name, &draw);
            let points: Vec<_> = frames.iter().map(|f| (f.t, f.x as f32, f.y as f32)).collect();
            st.tg_path(&points);
            return Step::Batch;
        } else if a == b"fpath" {
            let mut args = Scan::new(s);
            let _ = args.word(usize::MAX);
            let mut coords = Vec::new();
            while let Some(value) = args.float() { coords.push(value); }
            let ms = if coords.len() > 4 && coords.len() & 1 != 0 {
                let t = coords.pop().unwrap_or(600.0);
                if !t.is_finite() || t <= 0.0 { 0 } else { t as u32 }
            } else { 600 };
            if coords.len() < 4 || coords.len() & 1 != 0 || ms == 0 {
                err(&bformat("script: bad fpath line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            st.tg_fpath(&coords, ms);
            return Step::Batch;
        } else if a == b"trace" {
            let mut path_scan = Scan::new(s);
            let _ = path_scan.word(usize::MAX);
            let Some(path) = path_scan.word(4095) else {
                err(&bformat("script: bad trace line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            };
            let Ok(path) = std::str::from_utf8(path) else {
                err(&bformat("script: bad trace path: %s", &[B::S(path)]));
                st.expect_unknown_line();
                continue;
            };
            let text = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(_) => {
                    err(&bformat("script: cannot read trace: %s", &[B::S(path.as_bytes())]));
                    st.expect_unknown_line();
                    continue;
                }
            };
            if let Err(e) = st.tg_trace(&text) {
                err(&bformat("script: bad trace: %s", &[B::S(e.as_bytes())]));
                st.expect_unknown_line();
                continue;
            }
            return Step::Batch;
        } else if a == b"pen" || a == b"eraser" {
            st.eraser = a == b"eraser";
            let k = scan_floats(s, 2, &mut v);
            if b == b"down" {
                st.pen_ev(InType::PenDown, v[0], v[1], if k > 2 { v[2] } else { 0.6 }, v[3], v[4], 0);
            } else if b == b"move" {
                st.pen_ev(InType::PenMove, v[0], v[1], if k > 2 { v[2] } else { 0.6 }, v[3], v[4], 0);
            } else if b == b"up" {
                st.pen_ev(InType::PenUp, v[0], v[1], 0.0, 0.0, 0.0, 0);
            } else if b == b"hover" {
                st.pen_ev(InType::PenHover, v[0], v[1], 0.0, 0.0, 0.0, 0);
            } else if b == b"leave" {
                let (px, py) = (st.px, st.py);
                st.pen_ev(InType::PenLeave, px, py, 0.0, 0.0, 0.0, 0);
            } else if b == b"line" {
                // x0 y0 x1 y1 steps p0 p1 [tx ty]
                let mut steps = if k > 4 { v[4] as i32 } else { 20 };
                if steps < 1 {
                    steps = 1;
                }
                let p0 = if k > 5 { v[5] } else { 0.6 };
                let p1 = if k > 6 { v[6] } else { p0 };
                for i in 0..=steps {
                    let f = i as f32 / steps as f32;
                    st.pen_ev(
                        if i != 0 { InType::PenMove } else { InType::PenDown },
                        lerpf(v[0], v[2], f),
                        lerpf(v[1], v[3], f),
                        lerpf(p0, p1, f),
                        v[7],
                        v[8],
                        (i as u32).wrapping_mul(5),
                    );
                }
                st.pen_ev(InType::PenUp, v[2], v[3], 0.0, 0.0, 0.0, ((steps + 1) as u32).wrapping_mul(5));
            } else if b == b"curve" {
                // cx cy r steps p [turns] [tx ty]: a loop like cursive handwriting
                let steps = if k > 3 { v[3] as i32 } else { 60 };
                let p = if k > 4 { v[4] } else { 0.6 };
                let turns = if k > 5 && v[5] > 0.0 { v[5] } else { 1.0 };
                for i in 0..=steps {
                    let f = i as f32 / steps as f32;
                    let ang = f * turns * 6.2831853f32;
                    // v[0] + v[2] * (cosf(ang) - 1.f) + f * v[2] * 3.f * turns: clang fuses both sums
                    // (PORTING.md, Floating point), the right product into the left sum's result.
                    let x = fmla(f * v[2] * 3.0, turns, fmla(v[2], ang.cos() - 1.0, v[0]));
                    let y = fmla(v[2], ang.sin(), v[1]);
                    st.pen_ev(
                        if i != 0 { InType::PenMove } else { InType::PenDown },
                        x,
                        y,
                        p * fmla(0.3, (f * 3.14159f32).sin(), 0.7),
                        v[6],
                        v[7],
                        (i as u32).wrapping_mul(5),
                    );
                }
                let (px, py) = (st.px, st.py);
                st.pen_ev(InType::PenUp, px, py, 0.0, 0.0, 0.0, ((steps + 1) as u32).wrapping_mul(5));
            }
            st.eraser = false;
        } else if a == b"key" || a == b"keyup" {
            let mut iv = [0i32; 3]; // code, mods, pad
            scan_ints(s, 1, &mut iv);
            let e = InEvent {
                type_: if a.len() > 3 { InType::KeyUp } else { InType::Key },
                value: iv[0],
                key_mods: iv[1] as u32,
                gamepad: iv[2] != 0,
                t_ms: now_ms(),
                ..Default::default()
            };
            st.q_push(e);
        } else if a == b"axis" {
            // axis CODE VALUE: an axis of a controller as Linux's generic driver reports an Xbox One
            // S's (X, Y, RX and RY 0 to 65535, its triggers ABS_Z and ABS_RZ 0 to 1023, the hat),
            // decided and decoded as input_evdev.c does: a trigger pulled past half is LT's (or RT's)
            // press, let back to a quarter its release. Its other axes do nothing.
            if st.axis_trig.axis[0] == 0 {
                let none = [0u8; HID_KEY_BYTES];
                let mut axes = [0u8; HID_ABS_BYTES];
                let mut info = [HidAxis::default(); HIDKEYS_AXES];
                for (c, axis) in info.iter_mut().enumerate().take(6) {
                    axes[0] |= 1u8 << c;
                    *axis = if c == 2 || c == 5 {
                        HidAxis { minimum: 0, maximum: 1023, value: 0 }
                    } else {
                        HidAxis { minimum: 0, maximum: 65535, value: 32768 }
                    };
                }
                axes[2] |= 3; // ABS_HAT0X, ABS_HAT0Y
                hidkeys_triggers(&mut st.axis_trig, &none, &axes, &info);
                hidkeys_sync(&mut st.axis_keys, None, 0, 0);
            }
            let mut iv = [-1i32, 0]; // code, value
            let mut p = HidPress::default();
            if scan_ints(s, 1, &mut iv) != 2 {
                err(&bformat("script: bad axis line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            let trig = st.axis_trig;
            if hidkeys_trigger_event(&mut st.axis_keys, &trig, iv[0], iv[1], &mut p) != 0 {
                let e = InEvent {
                    type_: if p.up { InType::KeyUp } else { InType::Key },
                    value: p.code,
                    key_mods: p.mods as u32,
                    gamepad: p.gamepad,
                    t_ms: now_ms(),
                    ..Default::default()
                };
                st.q_push(e);
            }
        } else if a == b"hid" {
            // hid attach pad|key|held NAME, hid detach NAME: a controller (held: one another app
            // grabs), or a keyboard or page turner, comes or goes, as input_evdev.c reports it. A
            // controller has sixteen buttons from BTN_A, as Linux's generic driver gives one, so its
            // name alone makes it a Switch-mode one ("Pro Controller", hidkeys_switch).
            let attach = b == b"attach";
            let (kind, raw) = if attach { scan_hid_attach(s) } else { (&b""[..], scan_hid_detach(s)) };
            let mut e = InEvent {
                type_: if attach { InType::HidAttach } else { InType::HidDetach },
                t_ms: now_ms(),
                ..Default::default()
            };
            let mut name = String::new();
            hidkeys_name(&mut name, FixedStr::<41>::CAP, raw);
            e.hid_name.set(&name);
            let mut i = 0;
            while i < st.nhids && st.hids[i].name != e.hid_name {
                i += 1;
            }
            if attach && st.nhids < 4 && (kind == b"pad" || kind == b"key" || kind == b"held") {
                let mut h = InputHid { gamepad: kind != b"key", held: kind == b"held", ..Default::default() };
                h.name = e.hid_name;
                let mut buttons = [0u8; HID_KEY_BYTES];
                for code in 304..=319usize {
                    buttons[code / 8] |= 1u8 << (code % 8);
                }
                h.switch_layout = h.gamepad && hidkeys_switch(h.name.as_str(), &buttons);
                st.hids[st.nhids] = h;
                st.nhids += 1;
                e.gamepad = h.gamepad;
                e.value = h.held as i32;
            } else if !attach && b == b"detach" && i < st.nhids {
                e.gamepad = st.hids[i].gamepad;
                st.nhids -= 1;
                st.hids[i] = st.hids[st.nhids];
            } else {
                err(&bformat("script: bad hid line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            if st.shortcuts_enabled {
                st.q_push(e);
            }
        } else if a == b"side" || a == b"side2" {
            let second = a == b"side2";
            if second && !st.pen_button2 {
                err(&bformat("script: side2 needs GRIMOIRE_SIM_PEN_BUTTON2=1: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            if b == b"click" {
                // Start on a drained queue: every timestamp belongs to this helper, and
                // preceding pen samples reach the app before its hovering frame does.
                if st.qh != st.qt {
                    st.unread = Some(line.clone());
                    return Step::Batch;
                }
                let mut args = Vec::new();
                let mut valid = true;
                while let Some(word) = sc.word(64) {
                    let (v, used) = strtol10(word);
                    if used != word.len() || v < 0 || v > i32::MAX as i64 { valid = false; }
                    args.push(v as u32);
                }
                let count = args.first().copied().unwrap_or(0);
                let gap = args.get(1).copied().unwrap_or(150);
                let press = args.get(2).copied().unwrap_or(80);
                let duration = count as u64 * press as u64 + count.saturating_sub(1) as u64 * gap as u64;
                if !valid || args.is_empty() || args.len() > 3 || count == 0
                    || count as usize > (QN - 4) / 2 || duration > i32::MAX as u64
                {
                    err(&bformat("script: bad side click line: %s", &[B::S(s)]));
                    st.expect_unknown_line();
                    continue;
                }
                st.side_click(second, count, gap, press);
                return Step::Batch;
            }
            if b != b"on" && b != b"off" {
                err(&bformat("script: bad side line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            st.side_event(second, b == b"on", now_ms());
        } else if a == b"touch" {
            scan_floats(s, 2, &mut v[..2]);
            let t = if b == b"down" {
                InType::TouchDown
            } else if b == b"move" {
                InType::TouchMove
            } else {
                InType::TouchUp
            };
            st.touch_ev(t, v[0], v[1]);
        } else if a == b"tap" || a == b"pentap" {
            // `panel NAME` first: scan_floats would read "panel" as the float 0 and tap (0, 0), the clock
            // corner. The App finds the button (script_step), outside this borrow.
            if b == b"map-node" {
                if st.qh != st.qt {
                    st.unread = Some(line.clone());
                    st.line_no -= 1;
                    return Step::Batch;
                }
                let pid = std::str::from_utf8(sc.word(15).unwrap_or(b"0")).ok().and_then(|s| s.parse().ok()).unwrap_or(0);
                return Step::MapNode(pid, a == b"pentap");
            }
            if b == b"panel" {
                // The buttons are those the app drew from what is queued ahead of this line (a stroke
                // makes Undo): it ends the batch first, as an expect does, and is read again then.
                if st.qh != st.qt {
                    st.unread = Some(line.clone());
                    st.line_no -= 1;
                    return Step::Batch;
                }
                let mut name = FixedStr::<32>::new();
                name.set_bytes(sc.word(31).unwrap_or(b""));
                return Step::Panel(name, a == b"pentap");
            }
            scan_floats(s, 1, &mut v[..2]);
            st.tap_events(v[0], v[1], a == b"pentap");
            return Step::Batch; // one tap per batch, like a human
        } else if a == b"drag" {
            // drag pen|touch x0 y0 x1 y1 steps
            scan_floats(s, 2, &mut v[..5]);
            let pen = b == b"pen";
            let steps = if v[4] > 0.0 { v[4] as i32 } else { 10 };
            for i in 0..=steps {
                let f = i as f32 / steps as f32;
                let x = lerpf(v[0], v[2], f);
                let y = lerpf(v[1], v[3], f);
                if pen {
                    st.pen_ev(
                        if i != 0 { InType::PenMove } else { InType::PenDown },
                        x,
                        y,
                        0.5,
                        0.0,
                        0.0,
                        (i as u32).wrapping_mul(100),
                    );
                } else {
                    let mut e = st.make_ev(if i != 0 { InType::TouchMove } else { InType::TouchDown }, x, y, false);
                    e.t_ms = e.t_ms.wrapping_add((i as u32).wrapping_mul(100));
                    st.q_push(e);
                }
            }
            if pen {
                st.pen_ev(InType::PenUp, v[2], v[3], 0.0, 0.0, 0.0, ((steps + 1) as u32).wrapping_mul(100));
            } else {
                let mut e = st.make_ev(InType::TouchUp, v[2], v[3], false);
                e.t_ms = e.t_ms.wrapping_add(((steps + 1) as u32).wrapping_mul(100));
                st.q_push(e);
            }
            return Step::Batch;
        } else if a == b"wait" {
            st.wait_until = now_ms().wrapping_add(atoi(b) as u32);
            st.wait_bg = false;
            return Step::Batch;
        } else if a == b"hold" {
            st.script_hold = true;
            st.hold_file.set_bytes(b);
            return Step::Batch;
        } else if a == b"waitbg" {
            // Background work (an import, a template's picture, a PDF) takes as long as the machine
            // lets it: wait for it to end, not a time that holds on a quiet machine only.
            st.wait_until = now_ms().wrapping_add((if !b.is_empty() { atoi(b) } else { 60000 }) as u32);
            st.wait_bg = true;
            return Step::Batch;
        } else if a == b"shot" {
            st.pending_shot.set_bytes(b);
            return Step::Batch;
        } else if a == b"expect" {
            st.expect_read(s);
            // Nothing queued since the last flush: check now and read on (no extra refresh or
            // timer tick slips in). Else once this batch is handled and flushed, like a shot.
            if st.qh == st.qt {
                return Step::Check;
            }
            return Step::Batch;
        } else if a == b"rotate" {
            let mut e = st.make_ev(InType::Orientation, 0.0, 0.0, true);
            e.value = atoi(b) & 3;
            st.q_push(e);
            return Step::Batch;
        } else if a == b"pinch" {
            // pinch CX CY D0 D1 STEPS [DX DY]: two fingers around (CX,CY), spread D0 -> D1,
            // the midpoint drifting by (DX,DY): zoom and pan in one gesture.
            let k = scan_floats(s, 1, &mut v[..7]);
            let steps = if k > 4 && v[4] > 0.0 { v[4] as i32 } else { 8 };
            for i in 0..=steps {
                let f = i as f32 / steps as f32;
                let d = lerpf(v[2], v[3], f) * 0.5;
                // v[0] + (k > 5 ? v[5] : 0) * f, contracted as clang does (PORTING.md, Floating point)
                let cx = fmla(if k > 5 { v[5] } else { 0.0 }, f, v[0]);
                let cy = fmla(if k > 6 { v[6] } else { 0.0 }, f, v[1]);
                let mut e = st.make_ev(if i != 0 { InType::PinchMove } else { InType::PinchBegin }, cx - d, cy, false);
                e.x2 = cx + d;
                e.y2 = cy;
                e.t_ms = e.t_ms.wrapping_add((i as u32).wrapping_mul(160));
                st.q_push(e);
            }
            let mut e = st.make_ev(InType::PinchEnd, 0.0, 0.0, false);
            e.t_ms = e.t_ms.wrapping_add(((steps + 1) as u32).wrapping_mul(160));
            st.q_push(e);
            return Step::Batch;
        } else if a == b"sleep" || a == b"wake" {
            let sleep = a == b"sleep";
            let mut e = st.make_ev(if sleep { InType::Sleep } else { InType::Wake }, 0.0, 0.0, true);
            e.value = if sleep { 2 } else { 1 };
            st.q_push(e);
            return Step::Batch;
        } else if a == b"power" {
            // power: a short press and its release; power down / power up: one half at a time;
            // power held: the 4 s mark of a press that is still down (what input_evdev.c sends
            // then), so "power down", "power held" leaves while the key is down
            if b == b"down" {
                st.power_key(InType::Power, 0);
                st.power_down_at = now_ms();
            } else if b == b"up" {
                let held = now_ms().wrapping_sub(st.power_down_at) as i32;
                st.power_key(InType::PowerUp, held);
            } else if b == b"held" {
                st.power_key(InType::PowerHeld, INPUT_POWER_HOLD_MS);
            } else {
                st.power_key(InType::Power, 0);
                st.power_key(InType::PowerUp, 150);
            }
        } else if a == b"powerhold" {
            // held 4 s: the press, the hold (which leaves Grimoire), then the release
            st.power_key(InType::Power, 0);
            st.power_key(InType::PowerHeld, INPUT_POWER_HOLD_MS);
            st.power_key(InType::PowerUp, INPUT_POWER_HOLD_MS + 200);
            return Step::Batch;
        } else if a == b"storage" {
            // storage full: every file write fails from now on (a full userstore); storage ok
            SIM_STORAGE_FULL.store(b == b"full", Ordering::Relaxed);
            if b != b"full" && b != b"ok" {
                st.expect_unknown_line();
            }
        } else if a == b"powerd" {
            // powerd STATE: what the power service reports from now on (active, screenSaver, ...)
            st.powerd_state.set_bytes(b);
        } else if a == b"clock" {
            // clock SECONDS | real: the time the corner shows (unix seconds), kept where the script
            // puts it; real is the machine's again
            let (v, used) = strtol10(b);
            if b == b"real" {
                st.clock = None;
            } else if used != 0 && used == b.len() {
                st.clock = Some(v);
            } else {
                err(&bformat("script: bad clock line: %s", &[B::S(s)]));
                st.expect_unknown_line();
            }
        } else if a == b"battery" {
            // battery PERCENT [charging] | none: the battery the corner shows (none: unreadable)
            let (v, used) = strtol10(b);
            if b == b"none" {
                st.battery = Battery::default();
            } else if used != 0 && used == b.len() && (0..=100).contains(&v) {
                let charging = sc.word(15) == Some(&b"charging"[..]);
                st.battery = Battery { percent: v as i32, charging };
            } else {
                err(&bformat("script: bad battery line: %s", &[B::S(s)]));
                st.expect_unknown_line();
            }
        } else if a == b"finger" {
            // finger ID down|move X Y [MAJOR] [TOOL], finger ID up X Y: the frame being built
            let (id, used) = strtol10(b);
            let what = sc.word(15).unwrap_or(b"");
            let k = scan_floats(s, 3, &mut v[..4]);
            let id = if used != 0 && used == b.len() && (1..=TG_MAX as i64).contains(&id) { id as usize } else { 0 };
            let major = if k > 2 { v[2] } else { 0.0 };
            let tool = if k > 3 { v[3] } else { 0.0 };
            let ok = id != 0
                && k >= 2
                && major >= 0.0
                && (tool == 0.0 || tool == 1.0 || tool == 2.0)
                && (what == b"down" || ((what == b"move" || what == b"up") && st.fingers[id].is_some()));
            if !ok {
                err(&bformat("script: bad finger line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            st.fclock_start();
            st.fingers[id] = if what == b"up" {
                None // (the next frame has it lifted)
            } else {
                Some(st.finger_contact(id as i32, v[0], v[1], major as i32, tool as u8))
            };
        } else if a == b"frame" {
            // frame [MS]: the fingers down now through touchgest; the frame clock moves on by MS
            let (ms, used) = strtol10(b);
            if !b.is_empty() && (used != b.len() || ms < 0) {
                err(&bformat("script: bad frame line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            st.tg_frame(if b.is_empty() { 13 } else { ms as u32 });
        } else if a == b"swipe" {
            // swipe N X Y DX DY [STEPS] [SEP] [LANDGAP]
            let k = scan_floats(s, 1, &mut v[..8]);
            let n = v[0] as usize;
            if k < 5 || !(v[0] == 2.0 || v[0] == 3.0) || (k > 5 && v[5] < 1.0) || (k > 7 && v[7] < 0.0) {
                err(&bformat("script: bad swipe line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            let steps = if k > 5 { v[5] as i32 } else { 10 };
            let sep = if k > 6 { v[6] } else { 295.0 };
            let gap = if k > 7 { v[7] as u32 } else { 30 };
            st.tg_swipe(n, v[1], v[2], v[3], v[4], steps, sep, gap);
            return Step::Batch;
        } else if a == b"tapn" {
            // tapn N X Y [SEP] [MS]
            let k = scan_floats(s, 1, &mut v[..5]);
            if k < 3 || v[0] < 1.0 || v[0] > 10.0 || v[0] != v[0].floor() || (k > 4 && v[4] < 0.0) {
                err(&bformat("script: bad tapn line: %s", &[B::S(s)]));
                st.expect_unknown_line();
                continue;
            }
            let sep = if k > 3 { v[3] } else { 150.0 };
            let ms = if k > 4 { v[4] as u32 } else { 80 };
            st.tg_tapn(v[0] as usize, v[1], v[2], sep, ms);
            return Step::Batch;
        } else if a == b"quit" {
            let e = st.make_ev(InType::Quit, 0.0, 0.0, true);
            st.q_push(e);
            st.script_done = true;
            return Step::Batch;
        } else {
            err(&bformat("script: unknown command: %s", &[B::S(s)]));
            st.expect_unknown_line();
        }
    }
    if !st.script_done {
        st.script_done = true;
        let e = st.make_ev(InType::Quit, 0.0, 0.0, true);
        st.q_push(e);
    }
    Step::Batch
}

// Reads script lines until a batch boundary (wait/shot/end), checking an expect that needs no
// batch first as it comes.
fn script_step(mut a: Option<&mut App>) {
    loop {
        match with(script_step_in) {
            Step::Check => expect_check(a.as_deref_mut()),
            Step::Panel(name, pen) => {
                // The App says where the button is; the tap is then the one `tap` and `pentap` queue.
                let at = a.as_deref_mut().and_then(|a| editor_panel_point(a, name.as_str()));
                with(|st| match at {
                    Some((x, y)) => st.tap_events(x, y, pen),
                    None => {
                        // Fails the run like a failed check does (exit 3 at the end).
                        err(&bformat("expect failed (line %d): no panel button %s\n", &[B::D(st.line_no), B::S(name.as_bytes())]));
                        st.expect_total += 1;
                        st.expect_failed += 1;
                    }
                });
                break;
            }
            Step::MapNode(pid, pen) => {
                let at = a.as_deref_mut().and_then(|a| crate::editor::editor_map_point(a, pid));
                with(|st| match at {
                    Some((x, y)) => st.tap_events(x, y, pen),
                    None => {
                        err(format!("expect failed (line {}): no map node {}\n", st.line_no, pid).as_bytes());
                        st.expect_total += 1;
                        st.expect_failed += 1;
                    }
                });
                break;
            }
            Step::Batch => break,
        }
    }
    with(|st| st.fclock_fresh = true); // (a batch ended: the next contact line starts the clock again)
}

// ---- input ----

fn sim_touch_slots() -> i32 {
    std::env::var("GRIMOIRE_SIM_TOUCH_SLOTS")
        .or_else(|_| std::env::var("SIM_TOUCH_SLOTS"))
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(10)
        .max(0)
}

// The work pump (common.h): the simulator has no input devices to read during long work, so its hook
// only counts the calls, for the runner's "pump.calls" (the app's own long paths reach it).
fn sim_pump() {
    G_PUMPS.with(|p| p.set(p.get().wrapping_add(1)));
    unpumped_note();
}

pub fn input_open(screen_w: i32, screen_h: i32, dpi: f32, pen: Option<&InputCalib>, touch: Option<&InputCalib>) -> i32 {
    let _ = (screen_w, screen_h);
    work_pump_set(Some(sim_pump));
    work_pump_mark_set(Some(unpumped_mark));
    G_LAST_READ.with(|c| c.set(now_ms()));
    with(|st| {
        touchgest_init(&mut st.tg, dpi);
        st.pen_button2 = std::env::var("GRIMOIRE_SIM_PEN_BUTTON2").is_ok_and(|v| v == "1");
    });
    let tune_name = std::env::var("GRIMOIRE_TOUCH_TUNE").or_else(|_| std::env::var("TOUCH_TUNE")).unwrap_or_default();
    let tune = if tune_name == "v1" { TG_TUNE_V1 } else { TG_TUNE };
    let slots = sim_touch_slots();
    with(|st| {
        touchgest_set_tune(&mut st.tg, &tune);
        touchgest_set_slots(&mut st.tg, slots as usize);
    });
    with(|st| {
        let (mut p, mut t) = (InputCalib::default(), InputCalib::default());
        st.input_default_calib(Some(&mut p), Some(&mut t));
        st.pen = p;
        st.touch = t;
        if let Some(pen) = pen {
            st.pen = *pen;
        }
        if let Some(touch) = touch {
            st.touch = *touch;
        }
    });
    if let Some(path) = std::env::var_os("GRIMOIRE_SIM_RAWPEN_MAIN") {
        if let Ok(c) = CString::new(path.as_bytes()) {
            // SAFETY: open reads the NUL-terminated path; the result is checked. (O_RDWR: a FIFO
            // with no writer yet never reads as ended.)
            let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC) };
            with(|st| st.raw_fd = fd);
        }
        let stall = std::env::var_os("GRIMOIRE_SIM_STALL_MS").map_or(0, |v| atoi(v.as_bytes()));
        with(|st| st.raw_stall_ms = stall.max(0));
    }
    if let Some(sp) = std::env::var_os("GRIMOIRE_SIM_PRESSURE") {
        let p = clampf(atof(sp.as_bytes()) as f32, 0.0, 1.0);
        with(|st| st.pressure = p);
    }
    if let Some(script) = std::env::var_os("GRIMOIRE_SCRIPT") {
        match File::open(&script) {
            // The wait clock starts now: poll compares now_ms() with wait_until as a signed 32-bit
            // difference, which holds only within 2^31 ms of it. (The C's g_wait_until starts at 0,
            // so once CLOCK_MONOTONIC passes 2^31 ms, 24.8 days up, every run waited forever.)
            Ok(f) => with(|st| {
                st.script = Some(BufReader::new(f));
                st.wait_until = now_ms();
            }),
            Err(_) => {
                err(&bformat("cannot open script %s\n", &[B::S(script.as_bytes())]));
                return -1;
            }
        }
    }
    0
}
pub fn input_set_calib(pen: Option<&InputCalib>, touch: Option<&InputCalib>) {
    with(|st| {
        if let Some(pen) = pen {
            st.pen = *pen;
        }
        if let Some(touch) = touch {
            st.touch = *touch;
        }
    });
}
pub fn input_default_calib(pen: Option<&mut InputCalib>, touch: Option<&mut InputCalib>) {
    with(|st| st.input_default_calib(pen, touch));
}
pub fn input_caps() -> InputCaps {
    InputCaps {
        pen_button2: with(|st| st.pen_button2),
        has_pen: true,
        has_pressure: true,
        has_tilt: true,
        has_touch: true,
        has_power: true,
        has_orientation: false,
        orientation: 0,
        pen_name: FixedStr::from_str("SDL mouse (sim)"),
        touch_name: FixedStr::from_str("SDL right button (sim)"),
        touch_slots: sim_touch_slots(),
    }
}
pub fn input_watch_fd(fd: i32) {
    with(|st| st.watch_fd = fd);
}
pub fn input_shortcuts_enable(enabled: bool) {
    with(|st| st.shortcuts_enabled = enabled);
}
pub fn input_hids(out: &mut [InputHid]) -> i32 {
    with(|st| {
        let mut n = 0;
        let mut i = 0;
        while st.shortcuts_enabled && i < st.nhids && n < out.len() {
            out[n] = st.hids[i];
            n += 1;
            i += 1;
        }
        n as i32
    })
}
// Panel <-> screen for a Kindle turned g_rot quarter turns clockwise, as in input_evdev.c.
// display_set_orientation has already swapped g_w and g_h when this is set.
pub fn input_set_orientation(quarter_turns: i32) {
    with(|st| st.rot = quarter_turns & 3);
}
pub fn input_logical_to_panel(lx: f32, ly: f32, px: &mut f32, py: &mut f32) {
    with(|st| st.input_logical_to_panel(lx, ly, px, py));
}
pub fn input_touch_context(v: TgView) {
    with(|st| touchgest_set_view(&mut st.tg, v));
}
// (The simulator writes no trace: the formatter is touchgest's, unit-tested there.)
pub fn input_touch_trace(path: Option<&str>) {
    let _ = path;
}
pub fn input_touch_trace_event(line: &str) {
    let _ = line;
}
pub fn input_forget_signal_pipe() {}
pub fn input_close() {
    work_pump_set(None);
    work_pump_mark_set(None);
    expect_unrun();
    with(|st| st.script = None);
}

impl State {
    // The Live decoder's wire format stays unchanged. Button2 belongs to this
    // input reader and accompanies its ordinary frames and explicit button events.
    fn raw_event(&mut self, ev: &LiveInputEvent) -> bool {
        if ev.type_ == 0 && ev.code == 3 { self.raw_side2 = false; }
        if !self.raw_pen.dropping && ev.type_ == 1 && ev.code == 0x14c {
            self.raw_side2 = ev.value != 0;
        }
        let mut f = LiveRawFrame::default();
        if !live_raw_event(&mut self.raw_pen, 0, ev, &mut f) { return false; }
        let t = match f.state {
            LIVE_PEN_TOUCH if f.down => InType::PenDown,
            LIVE_PEN_TOUCH => InType::PenMove,
            LIVE_PEN_HOVER if self.raw_touching => InType::PenUp,
            LIVE_PEN_HOVER => InType::PenHover,
            _ => InType::PenLeave,
        };
        self.raw_touching = f.state == LIVE_PEN_TOUCH;
        if t == InType::PenLeave { self.raw_side2 = false; }
        let mut e = self.make_ev(t, f.x as f32, f.y as f32, true);
        e.pressure = if t == InType::PenDown || t == InType::PenMove { clampf(f.p as f32 / 4095.0, 0.0, 1.0) } else { 0.0 };
        e.tilt_x = f.tx as f32 / 100.0;
        e.tilt_y = f.ty as f32 / 100.0;
        e.eraser = f.eraser;
        e.side_button = f.side && t != InType::PenLeave;
        e.side_button2 = self.raw_side2 && self.pen_button2;
        e.gap = f.gap;
        e.k_us = f.us;
        e.k_ms = f.ms;
        self.side = e.side_button;
        self.side2 = e.side_button2;
        self.px = f.x as f32;
        self.py = f.y as f32;
        if t == InType::PenLeave {
            self.pen_arrives(&e);
            self.q_push(e);
            self.raw_side_reported = [false; 2];
            return true; // proximity loss itself cancels holds and pending clicks
        }
        let states = [e.side_button, e.side_button2];
        for i in 0..2 {
            if states[i] != self.raw_side_reported[i] {
                let mut b = e;
                b.type_ = InType::PenButton;
                b.value = if i == 0 { 0 } else { 2 };
                self.q_push(b);
            }
        }
        self.raw_side_reported = states;
        self.pen_arrives(&e);
        self.q_push(e);
        true
    }

    // The stand-in raw node's frames as pen events (raw_fd): true when any came.
    fn raw_read(&mut self) -> bool {
        if self.raw_fd < 0 {
            return false;
        }
        let mut any = false;
        let mut evs = [LiveInputEvent::default(); 64];
        loop {
            // SAFETY: read into the local array of records, at most its size in bytes.
            let got = unsafe { libc::read(self.raw_fd, evs.as_mut_ptr() as *mut libc::c_void, std::mem::size_of_val(&evs)) };
            if got <= 0 {
                break;
            }
            for ev in evs[..got as usize / std::mem::size_of::<LiveInputEvent>()].iter() {
                any |= self.raw_event(ev);
            }
        }
        any
    }
}

// The poll itself, with the App where the main loop hands it one (input_poll_app). Input is read here:
// the stretches between polls (and pumps) are what main.longest_unpumped_ms measures, the poll's own
// waits not.
fn poll(ev: &mut InEvent, timeout_ms: i32, a: Option<&mut App>) -> i32 {
    unpumped_note();
    let r = poll_in(ev, timeout_ms, a);
    G_LAST_READ.with(|c| c.set(now_ms()));
    r
}

fn poll_in(ev: &mut InEvent, timeout_ms: i32, mut a: Option<&mut App>) -> i32 {
    if with(|st| st.raw_read()) {
        let stall = with(|st| st.raw_stall_ms);
        if stall > 0 {
            sleep_ms(stall); // (the main loop busy: nothing of the pen is read meanwhile)
        }
    }
    if with(|st| st.q_pop(ev)) {
        return 1;
    }
    if let Some(due) = with(|st| st.side_click_until.map(|_| st.q[st.qh].t_ms)) {
        // No script command or expect passes an undelivered helper release. Polling at
        // timeout 0 still drains every due event, including releases after a slow frame.
        if timeout_ms != 0 {
            let left = due.wrapping_sub(now_ms()) as i32;
            if left > 0 { sleep_ms(if timeout_ms < 0 { left } else { left.min(timeout_ms) }); }
            if with(|st| st.q_pop(ev)) { return 1; }
        }
        return 0;
    }
    // Fingers held still give no frames: what only time decides goes by the frame clock's now.
    let ticked = with(|st| {
        if !touchgest_busy(&st.tg) {
            return false;
        }
        let mut out = TgOut::default();
        let k = st.tg_now();
        touchgest_tick(&mut st.tg, k, &mut out);
        for e in &out.ev[..out.n] {
            st.q_push(*e);
        }
        out.n > 0
    });
    if ticked && with(|st| st.q_pop(ev)) {
        return 1;
    }
    if with(|st| st.headless) {
        if timeout_ms == 0 {
            return 0; // end of the current batch: the app flushes next
        }
        // A shot requested by the script is taken here, after that flush, so it shows exactly
        // what the panel shows once the batch's refresh went out.
        with(|st| {
            if !st.pending_shot.is_empty() {
                let name = st.pending_shot;
                st.shot(name.as_bytes());
                st.pending_shot.clear();
            }
        });
        if with(|st| st.expect.pending) {
            expect_check(a.as_deref_mut()); // an expect waits for the same moment
        }
        with(|st| {
            if st.script_hold && !st.hold_file.is_empty() && access_exists(st.hold_file.as_bytes()) {
                st.script_hold = false;
            }
        });
        if with(|st| st.script_hold) {
            let raw = with(|st| st.raw_fd);
            if raw >= 0 {
                let mut pf = libc::pollfd { fd: raw, events: libc::POLLIN, revents: 0 };
                // SAFETY: poll reads and writes the one local pollfd.
                unsafe { libc::poll(&mut pf, 1, timeout_ms) };
            } else {
                sleep_ms(timeout_ms);
            }
            return 0;
        }
        if with(|st| st.wait_bg) && !bg_busy() {
            with(|st| {
                st.wait_bg = false;
                st.wait_until = now_ms(); // (reaped: its toast is up)
            });
        }
        let (wait_until, wait_bg) = with(|st| (st.wait_until, st.wait_bg));
        // (One reading of the clock for both: the wait is over once the signed difference from it
        // is no longer negative, and what is left is measured from that same reading, so a wait
        // that ended between two readings never comes out as 2^32 ms still to go.)
        let now = now_ms();
        if (now.wrapping_sub(wait_until) as i32) < 0 {
            let mut left = wait_until.wrapping_sub(now);
            if wait_bg && left > 20 {
                left = 20; // look again soon
            }
            let t = timeout_ms as u32;
            sleep_ms((if left < t { left } else { t }) as i32);
            return 0;
        }
        if !with(|st| st.script_done) {
            script_step(a);
        }
        return if with(|st| st.q_pop(ev)) { 1 } else { 0 };
    }
    // (no SDL window to pump: grimoire-headless always runs a script)
    0
}

pub fn input_poll(ev: &mut InEvent, timeout_ms: i32) -> i32 {
    poll(ev, timeout_ms, None)
}

// access(path, F_OK) == 0
fn access_exists(path: &[u8]) -> bool {
    let Ok(c) = std::ffi::CString::new(path) else { return false };
    // SAFETY: access reads the NUL-terminated path c holds.
    unsafe { libc::access(c.as_ptr(), libc::F_OK) == 0 }
}

// ---- the runner's probe point ----

/// input_poll as the main loop calls it in the headless build, with the App: where the C's runner
/// takes a pending shot and checks a pending expect (inside input_poll, after the flush that ended
/// the batch), app_probe needs the App, which the C reaches through a global pointer (app.c's
/// g_probe_app) and the Rust is handed here (mainloop.rs; PORTING.md, The App). Otherwise exactly
/// input_poll.
pub fn input_poll_app(ev: &mut InEvent, timeout_ms: i32, a: &mut App) -> i32 {
    poll(ev, timeout_ms, Some(a))
}

// ---- the C library's string and number reading, as the runner uses it ----

/// isspace() in the C locale: space, \t, \n, \v, \f, \r.
fn isspace(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}

/// A line as the C's string functions see it: up to its first NUL.
fn c_str(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}

/// `while (isspace(*s)) s++;`
fn skip_space(b: &[u8]) -> &[u8] {
    &b[b.iter().position(|&c| !isspace(c)).unwrap_or(b.len())..]
}

/// strcspn(s, reject)
fn strcspn(s: &[u8], reject: &[u8]) -> usize {
    s.iter().position(|c| reject.contains(c)).unwrap_or(s.len())
}

/// strstr(h, n) != NULL (an empty needle is found everywhere)
fn strstr(h: &[u8], n: &[u8]) -> bool {
    n.is_empty() || h.windows(n.len()).any(|w| w == n)
}

/// fgets(line, cap, f): the next line with its '\n', or the next cap - 1 bytes of a longer one (the
/// rest comes as the next line, as the C reads it); None at the end of the file or on an error.
fn fgets(f: &mut BufReader<File>, cap: usize) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    while line.len() + 1 < cap {
        let buf = match f.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        if buf.is_empty() {
            break;
        }
        let room = &buf[..buf.len().min(cap - 1 - line.len())];
        let (n, nl) = match room.iter().position(|&c| c == b'\n') {
            Some(i) => (i + 1, true),
            None => (room.len(), false),
        };
        line.extend_from_slice(&room[..n]);
        f.consume(n);
        if nl {
            break;
        }
    }
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

/// sscanf's reading of its input: every conversion used here but %[ skips white space first.
struct Scan<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Scan<'a> {
    fn new(s: &'a [u8]) -> Self {
        Scan { s, i: 0 }
    }
    fn space(&mut self) {
        while self.i < self.s.len() && isspace(self.s[self.i]) {
            self.i += 1;
        }
    }
    /// %Ns (%s, %*s: usize::MAX): None at the end of the input.
    fn word(&mut self, max: usize) -> Option<&'a [u8]> {
        self.space();
        let start = self.i;
        while self.i < self.s.len() && self.i - start < max && !isspace(self.s[self.i]) {
            self.i += 1;
        }
        if self.i == start {
            None
        } else {
            Some(&self.s[start..self.i])
        }
    }
    /// %f
    fn float(&mut self) -> Option<f32> {
        self.space();
        let (num, n) = scan_number(&self.s[self.i..])?;
        self.i += n;
        Some(num.to_f32())
    }
    /// %d
    fn int(&mut self) -> Option<i32> {
        self.space();
        let (v, n) = strtol10(&self.s[self.i..]);
        if n == 0 {
            return None;
        }
        self.i += n;
        Some(v as i32)
    }
    /// An ordinary character of the format: matched as it is, nothing skipped first.
    fn literal(&mut self, c: u8) -> bool {
        if self.i < self.s.len() && self.s[self.i] == c {
            self.i += 1;
            true
        } else {
            false
        }
    }
    /// %N[^\n]: nothing skipped first; None when no byte is read.
    fn up_to_newline(&mut self, max: usize) -> Option<&'a [u8]> {
        let start = self.i;
        while self.i < self.s.len() && self.i - start < max && self.s[self.i] != b'\n' {
            self.i += 1;
        }
        if self.i == start {
            None
        } else {
            Some(&self.s[start..self.i])
        }
    }
}

/// sscanf(s, "%*s ... %f %f ...", v...) with `skip` words first: fills v from its start as far as
/// numbers are read (the rest keeps its values) and returns what sscanf does (the count; -1 when
/// the input ends before the first word).
fn scan_floats(s: &[u8], skip: usize, v: &mut [f32]) -> i32 {
    let mut sc = Scan::new(s);
    for k in 0..skip {
        if sc.word(usize::MAX).is_none() {
            return if k == 0 { -1 } else { 0 };
        }
    }
    let mut n = 0;
    for slot in v.iter_mut() {
        match sc.float() {
            Some(f) => {
                *slot = f;
                n += 1;
            }
            None => break,
        }
    }
    n
}

/// The same for "%*s %d %d ...".
fn scan_ints(s: &[u8], skip: usize, v: &mut [i32]) -> i32 {
    let mut sc = Scan::new(s);
    for k in 0..skip {
        if sc.word(usize::MAX).is_none() {
            return if k == 0 { -1 } else { 0 };
        }
    }
    let mut n = 0;
    for slot in v.iter_mut() {
        match sc.int() {
            Some(d) => {
                *slot = d;
                n += 1;
            }
            None => break,
        }
    }
    n
}

/// sscanf(s, "%*s %*s %15s %255[^\n]", kind, raw): the kind and the name of "hid attach" ("" each
/// where none was read).
fn scan_hid_attach(s: &[u8]) -> (&[u8], &[u8]) {
    let mut sc = Scan::new(s);
    if sc.word(usize::MAX).is_none() || sc.word(usize::MAX).is_none() {
        return (b"", b"");
    }
    let Some(kind) = sc.word(15) else { return (b"", b"") };
    sc.space();
    (kind, sc.up_to_newline(255).unwrap_or(b""))
}

/// sscanf(s, "%*s %*s %255[^\n]", raw): the name of "hid detach".
fn scan_hid_detach(s: &[u8]) -> &[u8] {
    let mut sc = Scan::new(s);
    if sc.word(usize::MAX).is_none() || sc.word(usize::MAX).is_none() {
        return b"";
    }
    sc.space();
    sc.up_to_newline(255).unwrap_or(b"")
}

/// strtol(s, &end, 10) as the C library reads it (white space, a sign, digits; past long's range it
/// stays at its end) and how many bytes it read (0: no number).
fn strtol10(s: &[u8]) -> (i64, usize) {
    let mut i = 0;
    while i < s.len() && isspace(s[i]) {
        i += 1;
    }
    let neg = i < s.len() && s[i] == b'-';
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        i += 1;
    }
    let start = i;
    let mut v: i64 = 0;
    let mut over = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        match v.checked_mul(10).and_then(|x| if neg { x.checked_sub(d) } else { x.checked_add(d) }) {
            Some(x) => v = x,
            None => over = true,
        }
        i += 1;
    }
    if i == start {
        return (0, 0);
    }
    if over {
        v = if neg { i64::MIN } else { i64::MAX };
    }
    (v, i)
}

/// atoi: (int)strtol(s, NULL, 10).
fn atoi(s: &[u8]) -> i32 {
    strtol10(s).0 as i32
}

/// atof: strtod(s, NULL).
fn atof(s: &[u8]) -> f64 {
    let s = skip_space(s);
    match scan_number(s) {
        Some((num, _)) => num.to_f64(),
        None => 0.0,
    }
}

/// A number as strtod reads it, before it is rounded to a float or a double.
enum CNum<'a> {
    /// decimal digits, as written (a sign, digits, a point, an exponent)
    Dec(&'a str),
    Inf(bool),
    Nan(bool),
    /// a hexadecimal float, its value
    Hex(f64),
}

impl CNum<'_> {
    /// strtof's value
    fn to_f32(&self) -> f32 {
        match *self {
            CNum::Dec(s) => s.parse::<f32>().unwrap_or(0.0),
            CNum::Inf(neg) => {
                if neg {
                    f32::NEG_INFINITY
                } else {
                    f32::INFINITY
                }
            }
            CNum::Nan(neg) => {
                if neg {
                    -f32::NAN
                } else {
                    f32::NAN
                }
            }
            CNum::Hex(v) => v as f32,
        }
    }
    /// strtod's value
    fn to_f64(&self) -> f64 {
        match *self {
            CNum::Dec(s) => s.parse::<f64>().unwrap_or(0.0),
            CNum::Inf(neg) => {
                if neg {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            }
            CNum::Nan(neg) => {
                if neg {
                    -f64::NAN
                } else {
                    f64::NAN
                }
            }
            CNum::Hex(v) => v,
        }
    }
}

/// The longest start of b that strtod reads as a number (no white space skipped), and its length;
/// None when there is none.
fn scan_number(b: &[u8]) -> Option<(CNum<'_>, usize)> {
    let at = |i: usize| -> u8 { b.get(i).copied().unwrap_or(0) };
    let ci = |i: usize, word: &[u8]| -> bool {
        b.len() >= i + word.len() && b[i..i + word.len()].eq_ignore_ascii_case(word)
    };
    let mut i = 0;
    let neg = at(0) == b'-';
    if at(0) == b'-' || at(0) == b'+' {
        i += 1;
    }
    if ci(i, b"inf") {
        let n = if ci(i, b"infinity") { i + 8 } else { i + 3 };
        return Some((CNum::Inf(neg), n));
    }
    if ci(i, b"nan") {
        let mut n = i + 3;
        // nan(n-char-sequence)
        if at(n) == b'(' {
            let mut j = n + 1;
            while at(j).is_ascii_alphanumeric() || at(j) == b'_' {
                j += 1;
            }
            if at(j) == b')' {
                n = j + 1;
            }
        }
        return Some((CNum::Nan(neg), n));
    }
    if at(i) == b'0' && (at(i + 1) == b'x' || at(i + 1) == b'X') {
        let h = i + 2;
        if at(h).is_ascii_hexdigit() || (at(h) == b'.' && at(h + 1).is_ascii_hexdigit()) {
            return Some(scan_hex(b, h, neg));
        }
    }
    let mut digits = 0;
    while at(i).is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if at(i) == b'.' {
        i += 1;
        while at(i).is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if at(i) == b'e' || at(i) == b'E' {
        let mut j = i + 1;
        if at(j) == b'-' || at(j) == b'+' {
            j += 1;
        }
        if at(j).is_ascii_digit() {
            while at(j).is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    // (ASCII by construction)
    let text = std::str::from_utf8(&b[..i]).unwrap_or("0");
    Some((CNum::Dec(text), i))
}

/// A hexadecimal float from its first digit (at h): hex digits, a point, more, and a binary
/// exponent (p, a sign, decimal digits).
fn scan_hex(b: &[u8], h: usize, neg: bool) -> (CNum<'_>, usize) {
    let at = |i: usize| -> u8 { b.get(i).copied().unwrap_or(0) };
    let mut i = h;
    let mut mant: u64 = 0;
    let mut exp: i64 = 0;
    let mut seen_point = false;
    loop {
        let c = at(i);
        if c == b'.' && !seen_point {
            seen_point = true;
        } else if let Some(d) = (c as char).to_digit(16) {
            if mant >> 60 == 0 {
                mant = (mant << 4) | d as u64;
                if seen_point {
                    exp -= 4;
                }
            } else if !seen_point {
                exp += 4; // past what the mantissa holds: the digit only scales it
            }
        } else {
            break;
        }
        i += 1;
    }
    if at(i) == b'p' || at(i) == b'P' {
        let mut j = i + 1;
        let eneg = at(j) == b'-';
        if at(j) == b'-' || at(j) == b'+' {
            j += 1;
        }
        if at(j).is_ascii_digit() {
            let mut e: i64 = 0;
            while at(j).is_ascii_digit() {
                e = (e * 10 + (at(j) - b'0') as i64).min(100_000);
                j += 1;
            }
            exp += if eneg { -e } else { e };
            i = j;
        }
    }
    let v = mant as f64 * 2f64.powi(exp.clamp(-100_000, 100_000) as i32);
    (CNum::Hex(if neg { -v } else { v }), i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() {
        with(|st| *st = State::default());
    }

    /// A script of these lines, as input_open opens one, and a headless run (display_open's part).
    fn script(name: &str, text: &str) {
        let dir = std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into());
        let path = format!("{dir}/sim_sdl_{}_{name}.qs", std::process::id());
        let mut f = File::create(&path).unwrap();
        f.write_all(text.as_bytes()).unwrap();
        drop(f);
        with(|st| {
            st.headless = true;
            st.eink = white_panel(st.w, st.h).unwrap();
            let (mut p, mut t) = (InputCalib::default(), InputCalib::default());
            st.input_default_calib(Some(&mut p), Some(&mut t));
            st.pen = p;
            st.touch = t;
            st.script = Some(BufReader::new(File::open(&path).unwrap()));
            // (the wait clock starts now, as input_open starts it with the script it opens)
            st.wait_until = now_ms();
        });
        let _ = std::fs::remove_file(&path);
    }

    /// The next batch that brings events (past the waits before it).
    fn next_batch() -> Vec<InEvent> {
        for _ in 0..100 {
            let b = batch();
            if !b.is_empty() {
                return b;
            }
        }
        Vec::new()
    }

    /// The events the next batch brings, as the main loop takes them.
    fn batch() -> Vec<InEvent> {
        let mut out = Vec::new();
        let mut ev = InEvent::default();
        if poll(&mut ev, 30, None) == 0 {
            return out;
        }
        out.push(ev);
        while poll(&mut ev, 0, None) != 0 {
            out.push(ev);
        }
        out
    }

    #[test]
    fn sscanf_reads_as_the_c_library() {
        let mut sc = Scan::new(b"  abcdefghijklmnopqrs x\n");
        // %15s stops after 15 bytes, and %63s reads on from there, as sscanf does
        assert_eq!(sc.word(15), Some(&b"abcdefghijklmno"[..]));
        assert_eq!(sc.word(63), Some(&b"pqrs"[..]));
        let mut v = [0f32; 10];
        assert_eq!(scan_floats(b"pen down 300 700.5 .5 -2e1 1.\n", 2, &mut v), 5);
        assert_eq!(&v[..5], &[300.0, 700.5, 0.5, -20.0, 1.0]);
        let mut v = [9f32; 3];
        assert_eq!(scan_floats(b"pen down\n", 2, &mut v), 0);
        assert_eq!(v, [9.0; 3]);
        assert_eq!(scan_floats(b"tap 10 abc 30", 1, &mut v), 1);
        assert_eq!(v, [10.0, 9.0, 9.0]);
        // strtod's longest number: "1e" is the 1, and the "e" ends the reading there
        assert_eq!(scan_floats(b"tap 0x10 1e 2", 1, &mut v), 2);
        assert_eq!(v, [16.0, 1.0, 9.0]);
        let mut iv = [0i32; 3];
        assert_eq!(scan_ints(b"key 30 4\n", 1, &mut iv), 2);
        assert_eq!(iv, [30, 4, 0]);
        assert_eq!(atoi(b"200"), 200);
        assert_eq!(atoi(b"-7x"), -7);
        assert_eq!(atoi(b"abc"), 0);
        assert_eq!(atoi(b"99999999999"), 99999999999i64 as i32);
        assert_eq!(atof(b" 0.75"), 0.75);
        assert_eq!(scan_hid_attach(b"hid attach pad Pro Controller\n"), (&b"pad"[..], &b"Pro Controller"[..]));
        assert_eq!(scan_hid_detach(b"hid detach Pro Controller\n"), &b"Pro Controller"[..]);
        assert_eq!(scan_hid_attach(b"hid attach pad\n"), (&b"pad"[..], &b""[..]));
    }

    #[test]
    fn expect_lines_read_as_the_c_reads_them() {
        fresh();
        let read = |line: &str| {
            with(|st| {
                st.line_no = 7;
                st.expect_read(line.as_bytes());
                (st.expect.bad, st.expect.op, st.expect.key.clone(), st.expect.value.clone(), st.expect.line)
            })
        };
        assert_eq!(read("expect screen is library\n"), (false, ExpectOp::Is, b"screen".to_vec(), b"library".to_vec(), 7));
        assert_eq!(read("expect toast is\n"), (false, ExpectOp::Is, b"toast".to_vec(), b"".to_vec(), 7));
        assert_eq!(read("expect toast is \n"), (false, ExpectOp::Is, b"toast".to_vec(), b"".to_vec(), 7));
        assert_eq!(read("expect toast is  two\r\n"), (false, ExpectOp::Is, b"toast".to_vec(), b" two".to_vec(), 7));
        assert_eq!(read("expect\tlibrary.text has a b\n").1, ExpectOp::Has);
        assert_eq!(read("expect running not 1").2, b"running".to_vec());
        assert!(read("expect screen\n").0);
        assert!(read("expect screen was library\n").0);
        assert!(read("expect\n").0);
        let long = format!("expect {} is 1\n", "k".repeat(64));
        assert!(read(&long).0);
        assert!(!read(&format!("expect {} is 1\n", "k".repeat(63))).0);
    }

    #[test]
    fn a_tap_is_one_batch_and_a_line_its_samples() {
        fresh();
        script("tap", "# a comment\n\ntap 100 200\npen line 0 0 100 0 4 0.5 0.5\nside on\nwait 0\nquit\n");
        let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
        let b = batch();
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].type_, InType::TouchDown);
        assert!(near(b[0].x, 100.0) && near(b[0].y, 200.0));
        assert_eq!(b[1].type_, InType::TouchUp);
        // the line's down, 4 moves and its up, then the side button, up to the wait
        let b = batch();
        assert_eq!(b.len(), 7);
        assert_eq!(b[0].type_, InType::PenDown);
        assert!(near(b[1].x, 25.0) && near(b[2].x, 50.0) && b[2].pressure == 0.5);
        assert!(b[2].t_ms.wrapping_sub(b[0].t_ms) >= 10);
        assert_eq!((b[5].type_, b[5].pressure), (InType::PenUp, 0.0));
        assert_eq!((b[6].type_, b[6].side_button), (InType::PenButton, true));
        assert!(near(b[6].x, 100.0));
        let b = batch();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].type_, InType::Quit);
        assert!(with(|st| st.script_done));
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (0, 0));
    }

    fn queued() -> Vec<InEvent> {
        with(|st| {
            let mut out = Vec::new();
            let mut i = st.qh;
            while i != st.qt { out.push(st.q[i]); i = (i + 1) % QN; }
            out
        })
    }

    #[test]
    fn side_click_defaults_and_overrides_have_exact_frame_spacing() {
        fresh();
        script("side-click-defaults", "side click 3\nquit\n");
        with(|st| { st.px = 123.0; st.py = 456.0; assert!(matches!(script_step_in(st), Step::Batch)); });
        let q = queued();
        assert_eq!(q.len(), 7);
        assert_eq!(q[0].type_, InType::PenHover);
        assert!(q.iter().all(|e| (e.x - 123.0).abs() < 0.001 && (e.y - 456.0).abs() < 0.001));
        let start = q[0].t_ms;
        assert_eq!(q.iter().map(|e| e.t_ms.wrapping_sub(start)).collect::<Vec<_>>(), [0, 0, 80, 230, 310, 460, 540]);
        assert_eq!(q[1..].iter().map(|e| (e.type_, e.value, e.side_button, e.side_button2)).collect::<Vec<_>>(),
            (0..6).map(|i| (InType::PenButton, 0, i % 2 == 0, false)).collect::<Vec<_>>());
        assert!(with(|st| st.tpen.prox && !st.tpen.touching && st.side_click_until == Some(start.wrapping_add(540))));

        fresh();
        script("side-click-overrides", "side2 click 2 351 450\nquit\n");
        with(|st| { st.pen_button2 = true; script_step_in(st); });
        let q = queued();
        let start = q[0].t_ms;
        assert_eq!(q.iter().map(|e| e.t_ms.wrapping_sub(start)).collect::<Vec<_>>(), [0, 0, 450, 801, 1251]);
        assert!(q[1..].iter().all(|e| e.value == 2 && !e.side_button));
        assert_eq!(q[1..].iter().map(|e| e.side_button2).collect::<Vec<_>>(), [true, false, true, false]);
    }

    #[test]
    fn side_on_off_keeps_its_sequence_and_both_states_accompany_every_pen_frame() {
        fresh();
        with(|st| st.pen_button2 = true);
        script("side-both", "pen hover 120 340\nside on\nside2 on\npen down 120 340\npen move 125 345\npen up 125 345\npen hover 125 345\npen leave\nside off\nside2 off\nwait 0\n");
        let q = batch();
        assert_eq!(q.iter().map(|e| e.type_).collect::<Vec<_>>(), [InType::PenHover, InType::PenButton, InType::PenButton,
            InType::PenDown, InType::PenMove, InType::PenUp, InType::PenHover, InType::PenLeave, InType::PenButton, InType::PenButton]);
        assert_eq!((q[1].value, q[1].side_button, q[1].side_button2), (0, true, false));
        assert_eq!((q[2].value, q[2].side_button, q[2].side_button2), (2, true, true));
        assert!(q[3..8].iter().all(|e| e.side_button && e.side_button2));
        assert_eq!((q[8].value, q[8].side_button, q[8].side_button2), (0, false, true));
        assert_eq!((q[9].value, q[9].side_button, q[9].side_button2), (2, false, false));
    }

    #[test]
    fn second_button_requires_capability_and_bad_helpers_fail_without_events() {
        fresh();
        assert!(!input_caps().pen_button2);
        script("side-refused", "side2 on\nside2 off\nside2 click 2\nside click 0\nside click -1\nside click 1 -2\nside click 1 2 -3\nside click 1 2 3 4\nside click 99999\nside click 2 2147483647\nwait 0\n");
        assert!(batch().is_empty());
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (10, 10));
        assert!(with(|st| !st.side && !st.side2 && st.side_click_until.is_none()));
        with(|st| st.pen_button2 = true);
        assert!(input_caps().pen_button2);
    }

    #[test]
    fn side_click_waits_on_wall_time_and_drains_a_late_release_before_idle() {
        fresh();
        script("side-paced", "side click 1 150 80\nhold\n");
        with(|st| { script_step_in(st); });
        let start = queued()[0].t_ms;
        let c = PbConfig::default();
        let mut pb = PenButtons::default();
        let pen = PbPen { prox: true, touching: false };
        with(|st| {
            let mut e = InEvent::default();
            assert!(st.q_pop_at(&mut e, start));
            assert_eq!(e.type_, InType::PenHover);
            assert!(st.q_pop_at(&mut e, start));
            assert_eq!((e.type_, e.side_button), (InType::PenButton, true));
            assert_eq!(penbutton_event(&mut pb, &c, PbIn { t_ms: e.t_ms, button: PenBtn::Button, down: true }, pen), PbOut::None);
            assert!(!st.q_pop_at(&mut e, start.wrapping_add(79)));
            assert!(st.side_click_until.is_some() && !st.script_hold);
            // A slow frame passed even the hold deadline. Poll must hand out the
            // already-due release before the consumer gets an idle hold tick.
            assert!(st.q_pop_at(&mut e, start.wrapping_add(1300)));
            assert_eq!((e.type_, e.side_button), (InType::PenButton, false));
            assert_eq!(e.t_ms.wrapping_sub(start), 80);
            assert_eq!(penbutton_event(&mut pb, &c, PbIn { t_ms: e.t_ms, button: PenBtn::Button, down: false }, pen), PbOut::None);
            assert_eq!(penbutton_tick(&mut pb, &c, start.wrapping_add(1300)), PbOut::Fire(PenBtn::Button, PenKind::Tap));
            assert!(!st.q_pop_at(&mut e, start.wrapping_add(1300)));
            assert!(st.side_click_until.is_none() && !st.script_hold);
        });
        let mut e = InEvent::default();
        assert_eq!(poll(&mut e, 0, None), 0);
        assert!(!with(|st| st.script_hold));
        assert_eq!(poll(&mut e, 1, None), 0);
        assert!(with(|st| st.script_hold));
    }

    #[test]
    fn side_click_release_is_not_delivered_early_and_wraps_its_clock() {
        fresh();
        script("side-release", "side2 click 1 0 12\nwait 0\n");
        with(|st| st.pen_button2 = true);
        let first = batch();
        assert_eq!(first.len(), 2);
        assert!(first[1].side_button2);
        let release = next_batch();
        assert_eq!(release.len(), 1);
        assert_eq!((release[0].value, release[0].side_button2), (2, false));
        assert_eq!(release[0].t_ms.wrapping_sub(first[1].t_ms), 12);
        assert!((now_ms().wrapping_sub(release[0].t_ms) as i32) >= 0);
        // Exercise the queue itself across wrap, including a delayed poll that
        // drains both a press and its due release before declaring itself idle.
        fresh();
        with(|st| {
            st.side_event(true, true, u32::MAX - 5);
            st.side_event(true, false, 4);
            st.side_click_until = Some(4);
            let mut e = InEvent::default();
            assert!(!st.q_pop_at(&mut e, u32::MAX - 6));
            assert!(st.q_pop_at(&mut e, u32::MAX));
            assert!(e.side_button2);
            assert!(!st.q_pop_at(&mut e, 3));
            assert!(st.q_pop_at(&mut e, 4));
            assert!(!e.side_button2);
            assert!(st.side_click_until.is_none());
        });
    }

    #[test]
    fn raw_second_button_survives_frames_ignores_partial_drop_and_cancels_on_leave() {
        fresh();
        with(|st| {
            st.pen_button2 = true;
            let mut send = |type_, code, value| st.raw_event(&LiveInputEvent { type_, code, value, ..Default::default() });
            for (t, c, v) in [(1, 0x140, 1), (3, 0, 300), (3, 1, 400), (1, 0x14c, 1), (0, 0, 0),
                (1, 0x14a, 1), (0, 0, 0), (1, 0x14a, 0), (0, 0, 0),
                (0, 3, 0), (1, 0x14c, 1), (0, 0, 0), (0, 0, 0),
                (1, 0x14c, 1), (0, 0, 0), (1, 0x140, 0), (0, 0, 0)] { send(t, c, v); }
        });
        let q = queued();
        assert_eq!(q.iter().filter(|e| e.type_ == InType::PenButton).map(|e| (e.value, e.side_button2)).collect::<Vec<_>>(),
            [(2, true), (2, false), (2, true)]);
        assert!(q.iter().filter(|e| matches!(e.type_, InType::PenDown | InType::PenUp)).all(|e| e.side_button2));
        let gap = q.iter().find(|e| e.gap && e.type_ == InType::PenHover).unwrap();
        assert!(!gap.side_button2);
        let leave = q.iter().position(|e| e.type_ == InType::PenLeave).unwrap();
        assert!(!q[leave].side_button2);
        assert_eq!(leave + 1, q.len()); // no button event may revive proximity after leave
    }

    #[test]
    fn tap_panel_is_read_before_the_floats_and_fails_without_a_button() {
        fresh();
        // `panel` is no float: the tap arm's own test comes first, so it is never `tap 0 0` (the clock
        // corner). With no App there is no editor, and no panel button: the line fails like a check
        // does, by its number, and queues nothing.
        script("tappanel", "tap panel undo\nexpect running is 1\npentap panel nosuchbutton\nwait 0\nquit\n");
        let mut all = Vec::new();
        for _ in 0..10 {
            all.extend(batch());
            if with(|st| st.script_done) {
                break;
            }
        }
        assert!(with(|st| st.script_done));
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].type_, InType::Quit);
        // the two panel lines failed, the expect between them held
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (3, 2));
    }

    #[test]
    fn the_runner_checks_what_it_answers_itself() {
        fresh();
        script(
            "expect",
            "expect running is 1\n\
             expect watching is 0\n\
             expect display.presents is\n\
             expect display.last_fast is\n\
             expect display.ink is 0\n\
             expect power.pokes is 0\n\
             expect pump.calls is 0\n\
             expect keyboard.draws is 0\n\
             expect screen is library\n\
             expcet screen is library\n\
             storage sideways\n\
             expect screen\n\
             expect running is 0\n\
             tpa 1 2\n\
             wait 0\n",
        );
        let b = batch();
        assert!(b.is_empty());
        // line 9 (no App to answer "screen"), 10, 11, 12, 13, 14 fail; the eight before pass
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (14, 6));
        assert!(!SIM_STORAGE_FULL.load(Ordering::Relaxed));
    }

    #[test]
    fn display_presents_lists_eight_and_counts_the_rest() {
        fresh();
        with(|st| st.eink = white_panel(st.w, st.h).unwrap());
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 1860, 2480), 0);
        display_live_ink(true);
        for i in 0..9 {
            display_present(&c, Rect { x: i, y: 10, w: 100, h: 20 }, RefreshMode::Fast);
        }
        display_present(&c, Rect { x: -50, y: 0, w: 100, h: 10 }, RefreshMode::Ui);
        display_live_ink(false);
        let mut got = String::new();
        assert!(with(|st| st.display_probe(b"display.presents", &mut got, GOT_CAP)));
        assert_eq!(
            got,
            "fast 0 10 100 20, fast 1 10 100 20, fast 2 10 100 20, fast 3 10 100 20, fast 4 10 100 20, \
             fast 5 10 100 20, fast 6 10 100 20, fast 7 10 100 20, and 2 more"
        );
        assert!(with(|st| st.display_probe(b"display.presents", &mut got, GOT_CAP)));
        assert_eq!(got, "");
        with(|st| st.display_probe(b"display.last_fast", &mut got, GOT_CAP));
        assert_eq!(got, "ink");
        with(|st| st.display_probe(b"display.last_fast_rect", &mut got, GOT_CAP));
        assert_eq!(got, "8 10 100 20");
        with(|st| st.display_probe(b"display.ink_frames", &mut got, GOT_CAP));
        assert_eq!(got, "9");
        assert!(!with(|st| st.display_probe(b"display.nonsense", &mut got, GOT_CAP)));
        assert_eq!(with(|st| st.stats), [9, 1, 0]);
        assert_eq!(with(|st| st.area[1]), 500);
    }

    #[test]
    fn sideways_the_panel_turns_under_the_screen() {
        fresh();
        let (mut w, mut h) = (0, 0);
        display_set_orientation(1, &mut w, &mut h);
        assert_eq!((w, h), (2480, 1860));
        input_set_orientation(1);
        let (mut cp, mut ct) = (InputCalib::default(), InputCalib::default());
        input_default_calib(Some(&mut cp), Some(&mut ct));
        // the calibration is the upright panel's
        assert_eq!((cp.ax, cp.ay), (1860.0, 2480.0));
        input_set_calib(Some(&cp), Some(&ct));
        let (mut px, mut py) = (0.0, 0.0);
        input_logical_to_panel(10.0, 20.0, &mut px, &mut py);
        assert_eq!((px, py), (20.0, 2479.0 - 10.0));
        let e = with(|st| st.make_ev(InType::PenDown, 10.0, 20.0, true));
        assert!((e.x - 10.0).abs() < 1e-3 && (e.y - 20.0).abs() < 1e-3);
        assert_eq!((e.nx, e.ny), (20.0 / 1860.0, 2469.0 / 2480.0));
    }

    #[test]
    fn the_power_service_answers_a_sleep_request() {
        fresh();
        let mut ev = InEvent::default();
        sys_request_sleep();
        assert!(with(|st| st.q_pop(&mut ev)));
        assert_eq!((ev.type_, ev.value), (InType::Sleep, 2));
        with(|st| st.powerd_state.set("screenSaver"));
        sys_request_sleep();
        assert!(with(|st| st.q_pop(&mut ev)));
        assert_eq!((ev.type_, ev.value), (InType::Wake, 1));
        let mut out = String::new();
        assert_eq!(sys_powerd_state(&mut out, 32), 0);
        assert_eq!(out, "screenSaver");
        with(|st| st.powerd_state.set("unknown"));
        assert_eq!(sys_powerd_state(&mut out, 32), -1);
        light_set_intensity(99);
        light_set_warmth(-3);
        let (mut i, mut w) = (0, 0);
        light_get(&mut i, &mut w);
        assert_eq!((i, w, light_warmth()), (LIGHT_MAX, 0, 0));
    }

    #[test]
    fn hid_lines_attach_and_detach() {
        fresh();
        script(
            "hid",
            "hid attach pad Pro Controller\nhid attach key Page Turner\nhid detach Nobody\nhid detach Pro Controller\nwait 0\n",
        );
        input_shortcuts_enable(true);
        // the hid lines do not end a batch: the wait does
        let got: Vec<_> =
            batch().iter().map(|ev| (ev.type_, ev.gamepad, ev.hid_name.as_str().to_string())).collect();
        assert_eq!(
            got,
            vec![
                (InType::HidAttach, true, "Pro Controller".to_string()),
                (InType::HidAttach, false, "Page Turner".to_string()),
                (InType::HidDetach, true, "Pro Controller".to_string()),
            ]
        );
        let mut hids = [InputHid::default(); 4];
        assert_eq!(input_hids(&mut hids), 1);
        assert_eq!(hids[0].name.as_str(), "Page Turner");
        // "hid detach Nobody" failed as an unknown line
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (1, 1));
    }

    // tests/fixtures/sim_probe.c's twin: tests/fixtures/sim_probe.qs through this runner, the same
    // stand-in for the app, the same lines on stderr. It runs in a process of its own (the test
    // below starts it), as display_close ends the run with status 3, as the C's does.
    #[test]
    #[ignore = "run by sim_probe_matches_the_c, in a process of its own"]
    fn sim_probe_child() {
        if std::env::var_os("GRIMOIRE_SIM_PROBE_CHILD").is_none() {
            return;
        }
        fn bits(f: f32) -> u32 {
            f.to_bits()
        }
        fn print_ev(e: &InEvent, t0: u32) {
            err(cformat!(
                "ev %d x %08x y %08x nx %08x ny %08x p %08x tx %08x ty %08x x2 %08x y2 %08x v %d m %u g %d e %d s %d dt %u hid \"%s\"\n",
                e.type_ as i32,
                bits(e.x),
                bits(e.y),
                bits(e.nx),
                bits(e.ny),
                bits(e.pressure),
                bits(e.tilt_x),
                bits(e.tilt_y),
                bits(e.x2),
                bits(e.y2),
                e.value,
                e.key_mods,
                e.gamepad,
                e.eraser,
                e.side_button,
                e.t_ms.wrapping_sub(t0) / 5 * 5,
                &e.hid_name
            )
            .as_bytes());
        }
        fn canvas_make(c: &mut Canvas, w: i32, h: i32) {
            c.px = (0..h).flat_map(|y| (0..w).map(move |x| ((x * 7 + y * 13) & 255) as u8)).collect();
            c.w = w;
            c.h = h;
            c.stride = w;
        }
        let mut c = Canvas::default();
        let (mut w, mut h, mut dpi) = (0, 0, 0f32);
        let r = display_open(&mut w, &mut h, &mut dpi);
        err(cformat!("open %d: %dx%d %08x\n", r, w, h, bits(dpi)).as_bytes());
        let r = input_open(w, h, dpi, None, None);
        let caps = input_caps();
        err(cformat!("input %d: %s, %s\n", r, &caps.pen_name, &caps.touch_name).as_bytes());
        // A calibration with the axes swapped and one mirrored, for the pen; the finger keeps the default.
        let pen = InputCalib { valid: true, swap_xy: true, ax: -1210.5, bx: 1205.25, ay: 1590.75, by: 3.5 };
        input_set_calib(Some(&pen), None);
        light_set_intensity(30);
        light_set_warmth(-2);
        let (mut li, mut lw) = (0, 0);
        light_get(&mut li, &mut lw);
        err(cformat!("light %d %d %d %d\n", li, lw, light_warmth(), light_available()).as_bytes());
        canvas_make(&mut c, w, h);
        display_live_ink(true);
        display_present(&c, Rect { x: 10, y: 20, w: 30, h: 40 }, RefreshMode::Fast);
        display_live_ink(false);
        display_present(&c, Rect { x: -5, y: 0, w: 50, h: 10 }, RefreshMode::Ui);
        display_present(&c, Rect { x: 0, y: 0, w, h }, RefreshMode::Full);
        display_present(&c, Rect { x: w, y: 0, w: 10, h: 10 }, RefreshMode::Full);
        sys_activity();
        sys_activity();
        with(|st| st.wait_until = now_ms());
        let mut quit = false;
        let mut pass = 0;
        while pass < 1000 && !quit {
            pass += 1;
            let mut ev = InEvent::default();
            if input_poll(&mut ev, 30) == 0 {
                err(b"(none)\n");
                continue;
            }
            let t0 = ev.t_ms;
            loop {
                print_ev(&ev, t0);
                // react: what the app would do, as far as the runner sees it
                if ev.type_ == InType::PenDown {
                    display_live_ink(true);
                }
                if ev.type_ == InType::PenUp {
                    let r = Rect { x: ev.x as i32 - 20, y: ev.y as i32 - 20, w: 40, h: 40 };
                    display_present(&c, r, RefreshMode::Fast);
                    display_live_ink(false);
                }
                if ev.type_ == InType::TouchUp {
                    display_present(&c, Rect { x: 0, y: 0, w: 100, h: 50 }, RefreshMode::Ui);
                }
                if ev.type_ == InType::Orientation {
                    display_set_orientation(ev.value, &mut w, &mut h);
                    input_set_orientation(ev.value);
                    canvas_make(&mut c, w, h);
                    let (mut p, mut t) = (InputCalib::default(), InputCalib::default());
                    input_default_calib(Some(&mut p), Some(&mut t));
                    let (mut px, mut py) = (0f32, 0f32);
                    input_logical_to_panel(10.0, 20.0, &mut px, &mut py);
                    err(cformat!(
                        "turned %d: %dx%d, default calib %08x %08x, (10, 20) on the panel %08x %08x\n",
                        ev.value,
                        w,
                        h,
                        bits(p.ax),
                        bits(p.ay),
                        bits(px),
                        bits(py)
                    )
                    .as_bytes());
                }
                if ev.type_ == InType::Power {
                    let mut st = String::new();
                    let r = sys_powerd_state(&mut st, 32);
                    err(cformat!("powerd %d \"%s\"\n", r, if r == 0 { st.as_str() } else { "-" }).as_bytes());
                    sys_request_sleep();
                }
                if ev.type_ == InType::Key && ev.value == 59 {
                    input_shortcuts_enable(true);
                }
                if ev.type_ == InType::HidAttach || ev.type_ == InType::HidDetach {
                    let mut hids = [InputHid::default(); 4];
                    let n = input_hids(&mut hids);
                    let mut line = cformat!("hids %d", n);
                    for hd in &hids[..n as usize] {
                        line.push_str(&cformat!(" [%s %d %d %d]", &hd.name, hd.gamepad, hd.held, hd.switch_layout));
                    }
                    line.push('\n');
                    err(line.as_bytes());
                }
                if ev.type_ == InType::Quit {
                    quit = true;
                }
                if input_poll(&mut ev, 0) == 0 {
                    break;
                }
            }
            err(b"-- batch\n");
        }
        input_close();
        let (total, failed, line_no) = with(|st| (st.expect_total, st.expect_failed, st.line_no));
        err(cformat!(
            "tally %d %d, line %d, storage %d\n",
            total,
            failed,
            line_no,
            SIM_STORAGE_FULL.load(Ordering::Relaxed)
        )
        .as_bytes());
        display_close();
        err(b"(not reached: a check failed)\n");
    }

    #[test]
    fn sim_probe_matches_the_c() {
        let dir = std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into());
        let cwd = std::path::PathBuf::from(dir).join(format!("sim_probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cwd);
        std::fs::create_dir_all(&cwd).unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "sim_sdl::tests::sim_probe_child", "--include-ignored", "--test-threads=1"])
            .current_dir(&cwd)
            .env("GRIMOIRE_SIM_PROBE_CHILD", "1")
            .env("GRIMOIRE_SCRIPT", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sim_probe.qs"))
            .env("GRIMOIRE_SHOTS", "shots-sim-probe")
            .env("GRIMOIRE_SIM_SIZE", "1200x1600")
            .env_remove("GRIMOIRE_SIM_STATS")
            .env_remove("GRIMOIRE_SIM_PRESSURE")
            .output()
            .unwrap();
        let want: &[u8] = include_bytes!("../tests/fixtures/sim_probe_c.txt");
        if out.stderr != want {
            let (g, w) = (String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(want));
            for (i, (a, b)) in g.lines().zip(w.lines()).enumerate() {
                if a != b {
                    panic!("line {}:\n  rust: {a}\n  c:    {b}", i + 1);
                }
            }
            panic!("the outputs differ in length: rust {} lines, c {} lines", g.lines().count(), w.lines().count());
        }
        assert_eq!(out.status.code(), Some(3));
        // The shots the C stand-in left unwritten: one gray plane of the panel's size.
        let png = std::fs::read(cwd.join("shots-sim-probe/first.png")).unwrap();
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..24], &[0, 0, 0x04, 0xb0, 0, 0, 0x06, 0x40]); // 1200 x 1600
        assert_eq!((png[24], png[25]), (8, 0)); // 8-bit gray
        assert!(cwd.join("shots-sim-probe/a-name-that-is-long-enough-to-see.png").exists());
        let _ = std::fs::remove_dir_all(&cwd);
    }

    // finger and frame lines build contact frames through touchgest without ending a batch; the frame
    // clock times them by their offsets, and a check between two frames never sends it back.
    #[test]
    fn contact_lines_and_the_frame_clock() {
        fresh();
        // This fixture asserts the legacy first-lift timestamps; the new follow window is tested separately.
        with(|st| touchgest_set_tune(&mut st.tg, &TG_TUNE_V1));
        script(
            "fingers",
            "finger 1 down 500 600\nframe 30\nfinger 2 down 800 600\nframe 20\nexpect touch.contacts is 2\n\
             finger 1 move 500 700\nfinger 2 move 800 700\nframe 20\nfinger 1 up 500 700\nframe 5\n\
             finger 2 up 800 700\nframe\nwait 0\n\
             finger 9 move 1 1\nfinger 17 down 1 1\nfinger 1 down 5\nframe x\nfinger 3 down 1 1 0 4\nwait 0\n",
        );
        let b = batch();
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types, [InType::TouchDown, InType::TouchUp, InType::PinchBegin]);
        let t0 = b[0].k_ms;
        assert_eq!((b[0].t_ms, b[1].k_ms, b[2].k_ms), (t0, t0.wrapping_add(30), t0.wrapping_add(30)));
        assert_eq!((b[1].x, b[1].value), (-10000.0, TOUCH_F_CANCEL));
        assert!((b[2].x - 500.0).abs() < 1e-3 && (b[2].x2 - 800.0).abs() < 1e-3);
        let b = batch();
        // the check came once that batch was handled: it saw both fingers down
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (1, 0));
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types, [InType::PinchMove, InType::PinchEnd]);
        // (the clock went on from where the first batch left it: never back, the offsets kept)
        assert!(b[0].k_ms.wrapping_sub(t0) as i32 >= 50 && b[1].k_ms.wrapping_sub(b[0].k_ms) == 20);
        assert_eq!(b[0].value, 3); // a drag where nothing scrolls: a swipe that is still too short
        assert!(batch().is_empty());
        // Bad finger and frame lines fail the run like unknown commands: an ID no finger has, one past
        // 16, no Y, a frame that is no number, a tool past 2.
        assert_eq!(with(|st| (st.expect_total, st.expect_failed)), (6, 5));
        let mut got = String::new();
        assert!(power_probe(b"touch.contacts", &mut got, GOT_CAP) && got == "0");
        assert!(power_probe(b"touch.class", &mut got, GOT_CAP) && got == "none");
        assert!(!power_probe(b"touch.nonsense", &mut got, GOT_CAP));
    }

    // swipe and tapn are a batch each, through the same touchgest; with gestures on, they fire.
    #[test]
    fn swipe_and_tapn_are_a_batch_each() {
        fresh();
        script("swipe", "swipe 2 930 900 0 400\ntapn 2 100 100\nswipe 2 930 900 0 400 10 295 160\nwait 0\n");
        input_touch_context(TgView { gestures_on: true, ..Default::default() });
        let b = batch();
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types.len(), 14);
        assert_eq!(&types[..3], [InType::TouchDown, InType::TouchUp, InType::PinchBegin]);
        assert!(types[3..12].iter().all(|&t| t == InType::PinchMove));
        assert_eq!(&types[12..], [InType::PinchEnd, InType::Gesture]);
        let g = &b[13];
        assert_eq!(g.value, G_2_DOWN);
        assert!((g.x - 930.0).abs() < 1e-3 && (g.y - 900.0).abs() < 1e-3 && (g.y2 - 1300.0).abs() < 1e-3);
        // the timeline: the second finger 30 ms after the first, moves 20 ms apart (the first PinchMove
        // at the commit, the second move), the lift 5 ms on
        assert_eq!(b[2].k_ms.wrapping_sub(b[0].k_ms), 30);
        assert_eq!(b[3].k_ms.wrapping_sub(b[2].k_ms), 40);
        assert_eq!(b[4].k_ms.wrapping_sub(b[3].k_ms), 20);
        assert_eq!(b[13].k_ms.wrapping_sub(b[11].k_ms), 5);
        let mut got = String::new();
        assert!(power_probe(b"touch.gesture", &mut got, GOT_CAP) && got == "2.down");
        // tapn 2 100 100: fingers at x 25 and 175, the midpoint 100: a two-finger tap
        let b = batch();
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types, [InType::TouchDown, InType::TouchUp, InType::PinchBegin, InType::PinchEnd, InType::Gesture]);
        assert!((b[2].x - 25.0).abs() < 1e-3 && (b[2].x2 - 175.0).abs() < 1e-3);
        assert_eq!((b[4].value, b[4].k_ms.wrapping_sub(b[0].k_ms)), (G_2_TAP, 100));
        // a second finger 160 ms after the first may zoom or pan, never swipe
        let b = batch();
        assert!(!b.iter().any(|e| e.type_ == InType::Gesture));
        assert!(power_probe(b"touch.reject", &mut got, GOT_CAP) && got == "late");
    }

    // The pen lines are touchgest's pen: near, touching, gone (pen leave stamps when); a pentap's nib
    // lifts away; a pen line while fingers are down ends them first (the Kindle's arrival rule).
    #[test]
    fn pen_lines_tell_touchgest() {
        fresh();
        script(
            "tgpen",
            "pen hover 300 300\nwait 0\npen leave\nwait 0\npentap 400 400\nwait 0\n\
             finger 1 down 900 1200\nframe\nwait 1100\nfinger 1 move 950 1200\nframe\npen down 300 300\npen up 300 300\nwait 0\n",
        );
        batch();
        assert!(with(|st| st.tpen.prox && !st.tpen.touching));
        let b = batch();
        assert_eq!(b.last().map(|e| e.type_), Some(InType::PenLeave));
        assert!(with(|st| !st.tpen.prox && st.tpen.left_ms == b.last().unwrap().t_ms));
        let b = batch();
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types, [InType::PenDown, InType::PenUp]); // (as it always was: no leave queued)
        assert!(with(|st| !st.tpen.prox && st.tpen.left_ms == b[1].t_ms && st.tpen.last.0 == b[0].x));
        // A finger within the second after the pentap is the writing hand: quiet.
        assert!(batch().is_empty());
        let b = next_batch();
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types, [InType::PenDown, InType::PenUp]);
        // After the second, a finger's stream ends with the cancel before the pen's down.
        fresh();
        script("tgpen2", "finger 1 down 900 1200\nframe\npen down 300 300\npen up 300 300\nwait 0\n");
        let b = batch();
        let types: Vec<InType> = b.iter().map(|e| e.type_).collect();
        assert_eq!(types, [InType::TouchDown, InType::TouchUp, InType::PenDown, InType::PenUp]);
        assert_eq!(b[1].value, TOUCH_F_CANCEL);
    }

    // A still pair gives no frames: the tick decides by the frame clock (a tap whose last finger
    // stays is refused 150 ms after the first lift), never by a clock behind the last frame.
    #[test]
    fn the_tick_uses_the_frame_clock() {
        fresh();
        script("tgtick", "finger 1 down 850 1200\nfinger 2 down 1150 1200\nframe 60\nfinger 1 up 850 1200\nframe 10\nwait 400\n");
        let b = batch();
        assert_eq!(b.iter().map(|e| e.type_).collect::<Vec<_>>(), [InType::PinchBegin]);
        let lift = with(|st| st.flast);
        let mut end = None;
        for _ in 0..40 {
            if let Some(e) = batch().into_iter().find(|e| e.type_ == InType::PinchEnd) {
                end = Some(e);
                break;
            }
        }
        let end = end.expect("the tick refused the tap");
        assert!(end.k_ms.wrapping_sub(lift) as i32 > 150);
        let mut got = String::new();
        assert!(power_probe(b"touch.reject", &mut got, GOT_CAP) && got == "slow");
    }

    #[test]
    fn a_messages_bytes_are_the_scripts() {
        assert_eq!(
            bformat("expect failed (line %d): unknown key \"%s\"\n", &[B::D(12), B::S(b"k\xff")]),
            b"expect failed (line 12): unknown key \"k\xff\"\n".to_vec()
        );
        assert_eq!(bformat("100%% %s", &[B::S(b"x")]), b"100% x".to_vec());
    }
}
