//! status.rs: the clock and battery corner. A small indicator in the top left corner of the panel:
//! the time (format_time_short, as the app writes times everywhere), the battery's percent, and a
//! small bolt while it charges. A finger tap in a 12 mm square there turns it on and off, on any
//! screen where no control is under the finger; the shortcut action `status-toggle` does the same,
//! and the Display sheet sets its size (settings.ini status_on, status_size; docs/FORMAT.md).
//!
//! Rust build only: the C tree has no such thing, so nothing here is a port.
//!
//! It is an overlay, like a toast and Live view's chip: app_flush paints it over a.screen just for
//! the refresh and puts the pixels under it straight back, so it is never in a page, an export, a
//! thumbnail, a Live view frame (live_push comes before it) or the sleep screen (app_sleep presents
//! a.screen as it is). The screen guard leaves its rect out (display_skip_status).
//!
//! It refreshes itself at each minute and when the battery or charging changes: one small GL16 of
//! its rect (and of what it covered, when it shrinks or goes), through display_present like every
//! other refresh. No display waits of any kind. Nothing is sent while the pen is near, a stroke is
//! in progress or the driver is in pen mode (a grayscale refresh would end that mode and wait for
//! the pen's last frame, display_fbink.rs pen_leave): the corner waits for the pen to leave.

use crate::prelude::*;
use std::cell::RefCell;

/// The tap zone's side, in mm. The indicator joins it while it is up, so a tap on it turns it off.
const ZONE_MM: f32 = 12.0;
/// A finger down longer than this is not a tap (a long press is the page's, a link's).
const TAP_MS: u32 = 600;
/// While the corner is up the battery is read this often; a change shows at the next refresh. The
/// simulator's is read in a tenth of a second, so a script sees a change at once (the Kindle's costs
/// a few small file reads, and the power service's fallback has its own once-a-minute limit).
#[cfg(not(feature = "sim"))]
const POLL_MS: u32 = 10000;
#[cfg(feature = "sim")]
const POLL_MS: u32 = 100;
/// Text heights at 300 dpi for settings.ini status_size 0, 1 and 2: small, medium and large.
const SIZE_PX: [f32; 3] = [30.0, 42.0, 56.0];
/// How many sizes there are.
pub const STATUS_SIZES: i32 = 3;
/// The bolt shown while charging: md-flash, in the font's Material Design icons.
const BOLT_CP: u32 = 0xF0241;

/// What the power supply says now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    /// 0 to 100, or -1 when it cannot be read
    pub percent: i32,
    /// it takes charge now
    pub charging: bool,
}

impl Default for Battery {
    fn default() -> Self {
        Battery { percent: -1, charging: false }
    }
}

// ---- the text ----

/// "Oct 10, 14:32  87%": the time as the app writes times (the language in use, local time), then
/// the percent; just the time where the battery cannot be read. The bolt is drawn, not written.
pub fn status_text(t: i64, b: &Battery) -> String {
    let mut when = String::new();
    format_time_short(t, &mut when, 64);
    if b.percent >= 0 {
        cformat!("%s  %d%%", &when, b.percent)
    } else {
        when
    }
}

// ---- the battery, as the kernel tells it ----

/// A whole number in a file of the power supply class ("87\n"): None for anything else.
pub fn battery_parse_number(b: &[u8]) -> Option<i32> {
    let t = std::str::from_utf8(b).ok()?.trim();
    if t.is_empty() || t.len() > 9 {
        return None;
    }
    t.parse::<i32>().ok()
}

/// The supply's `status` file: "Charging" charges; "Discharging", "Full", "Not charging" and
/// "Unknown" do not.
pub fn battery_parse_charging(b: &[u8]) -> bool {
    std::str::from_utf8(b).is_ok_and(|t| t.trim().eq_ignore_ascii_case("charging"))
}

/// The battery among the supplies under root (/sys/class/power_supply): the one whose `type` is
/// Battery (the chargers and the USB port are supplies too, with no capacity), its `capacity` and
/// whether its `status` says it charges. None where there is no such supply or it has no readable
/// capacity.
pub fn battery_from_sysfs(root: &str) -> Option<Battery> {
    let mut names: Vec<String> =
        std::fs::read_dir(root).ok()?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    for name in names {
        let dir = cformat!("%s/%s", root, &name);
        let is_battery =
            read_file(&cformat!("%s/type", &dir)).is_some_and(|t| String::from_utf8_lossy(&t).trim().eq_ignore_ascii_case("battery"));
        if !is_battery {
            continue;
        }
        let Some(percent) = read_file(&cformat!("%s/capacity", &dir)).and_then(|c| battery_parse_number(&c)) else {
            continue;
        };
        let charging = read_file(&cformat!("%s/status", &dir)).is_some_and(|s| battery_parse_charging(&s));
        return Some(Battery { percent: clampi(percent, 0, 100), charging });
    }
    None
}

// ---- a tap in the corner ----

/// A finger that went down in the zone on no control, followed to its release.
#[derive(Clone, Copy, Debug, Default)]
struct Tap {
    armed: bool,
    press: Press,
}

impl Tap {
    /// The first finger went down: it is a candidate only inside the zone and with no control there.
    fn down(&mut self, zone: Rect, control: bool, x: f32, y: f32, t: u32) {
        self.armed = !control && rect_contains(zone, x as i32, y as i32);
        if self.armed {
            press_begin(&mut self.press, x, y, t);
        }
    }
    fn moved(&mut self, x: f32, y: f32) {
        if self.armed {
            press_move(&mut self.press, x, y);
        }
    }
    /// The finger lifted: true when it was a tap (short, still, down and up inside the zone).
    fn up(&mut self, zone: Rect, x: f32, y: f32, t: u32) -> bool {
        if !self.armed {
            return false;
        }
        self.armed = false;
        press_move(&mut self.press, x, y);
        t.wrapping_sub(self.press.t0) <= TAP_MS && press_is_tap(&self.press, zone)
    }
    /// Something else began (a second finger, the pen): this was not a tap.
    fn cancel(&mut self) {
        self.armed = false;
    }
}

// ---- state ----

/// This module's state (the main thread's alone, PORTING.md, Global state).
#[derive(Default)]
struct State {
    /// the pen is in range: events since it came, until it leaves
    pen_near: bool,
    tap: Tap,
    /// the battery as last read, and when (now_ms, 0 never)
    battery: Battery,
    read_at: u32,
    /// what the panel shows now: the rect (empty for nothing), what it says, and the minute and
    /// battery that says
    drawn: Rect,
    drawn_text: String,
    drawn_charging: bool,
    drawn_key: Option<(i64, Battery)>,
    /// a refresh asked for and not sent yet: the rect (empty for none)
    pending: Rect,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// ---- where it goes and what it looks like ----

/// What the indicator says and where it sits.
struct Layout {
    rect: Rect,
    text: String,
    text_r: Rect,
    bolt: Option<Rect>,
    px: f32,
    /// the minute and battery it shows
    key: (i64, Battery),
}

fn size_px(size: i32) -> f32 {
    ui_px(SIZE_PX[clampi(size, 0, STATUS_SIZES - 1) as usize]) as f32
}

/// The indicator for a size, a time and a battery. Its text is in the monospaced face every theme
/// shares, so its box changes only with the number of characters.
fn layout_of(size: i32, t: i64, b: &Battery) -> Layout {
    let px = size_px(size);
    let text = status_text(t, b);
    let face = FontFace::Fixed;
    let tw = font_measure(face, px, &text);
    let lh = font_ascent(face, px) + font_descent(face, px);
    let padx = (px * 0.5) as i32;
    let pady = (px * 0.3) as i32;
    let (gap, bw) = if b.charging { ((px * 0.3) as i32, (px * 0.6) as i32) } else { (0, 0) };
    let m = ui_px(6.0);
    let rect = Rect { x: m, y: m, w: padx + tw + gap + bw + padx, h: lh + 2 * pady };
    Layout {
        rect,
        text,
        text_r: Rect { x: m + padx, y: m, w: tw + 2, h: rect.h },
        bolt: if b.charging { Some(Rect { x: m + padx + tw + gap, y: m + pady, w: bw, h: lh }) } else { None },
        px,
        key: (t.div_euclid(60), *b),
    }
}

fn layout(a: &App) -> Layout {
    let b = with(|s| s.battery);
    layout_of(a.settings.status_size, sys_wall_clock(), &b)
}

fn draw(c: &mut Canvas, l: &Layout) {
    ui_frame(c, l.rect, ui_px(8.0), 255, true, 0);
    ui_frame(c, l.rect, ui_px(8.0), 0, false, ui_px(2.0));
    font_draw_left(c, FontFace::Fixed, l.px, l.text_r, &l.text, 0);
    if let Some(b) = l.bolt {
        font_draw_icon(c, FontFace::Regular, BOLT_CP, b, 0);
    }
}

/// The corner as a tap finds it: 12 mm square, and the indicator while it is up.
pub fn status_zone(a: &App) -> Rect {
    let side = (ZONE_MM * a.dpi / 25.4).round() as i32;
    rect_union(Rect { x: 0, y: 0, w: side, h: side }, with(|s| s.drawn))
}

// ---- the pen, a finger, the shortcut ----

/// Every pen event, for "the pen is near": in range from its first event until it leaves.
pub fn status_pen(ev: &InEvent) {
    match ev.type_ {
        InType::PenDown | InType::PenMove | InType::PenHover | InType::PenUp => with(|s| {
            s.pen_near = true;
            s.tap.cancel();
        }),
        InType::PenLeave => with(|s| s.pen_near = false),
        _ => {}
    }
}

/// Whether the corner must hold its refreshes back: the pen is near, a stroke or another gesture
/// is in progress, or the driver is in pen mode.
fn status_busy(a: &App) -> bool {
    with(|s| s.pen_near) || editor_busy(a) || display_pen_mode_on()
}

/// A finger's events, before the screen sees them (it sees them all as ever; the corner only
/// watches): a tap in the zone with no control under it turns the indicator on or off. app_event
/// calls it awake only.
pub fn status_touch(a: &mut App, ev: &InEvent) {
    match ev.type_ {
        InType::TouchDown => {
            let zone = status_zone(a);
            // (asked before the screen handles the touch: what it does may change what is there)
            let control = rect_contains(zone, ev.x as i32, ev.y as i32) && app_control_at(a, ev.x, ev.y);
            with(|s| s.tap.down(zone, control, ev.x, ev.y, ev.t_ms));
        }
        InType::TouchMove => with(|s| s.tap.moved(ev.x, ev.y)),
        InType::TouchUp => {
            let zone = status_zone(a);
            if with(|s| s.tap.up(zone, ev.x, ev.y, ev.t_ms)) {
                status_toggle(a);
            }
        }
        InType::PinchBegin | InType::PinchMove | InType::PinchEnd => with(|s| s.tap.cancel()),
        _ => {}
    }
}

fn read_battery() {
    let b = sys_battery();
    with(|s| {
        s.battery = b;
        s.read_at = now_ms() | 1;
    });
}

/// The rect to refresh for a change of the indicator: what it covers now and what it covered.
fn changed_rect(a: &App) -> Rect {
    let now = if a.settings.status_on { layout(a).rect } else { RECT_EMPTY };
    rect_intersect(rect_union(now, with(|s| s.drawn)), Rect { x: 0, y: 0, w: a.w, h: a.h })
}

fn ask(r: Rect) {
    with(|s| s.pending = rect_union(s.pending, r));
}

/// Turns the indicator on or off: a tap in the corner, or the status-toggle shortcut.
pub fn status_toggle(a: &mut App) {
    a.settings.status_on = !a.settings.status_on;
    settings_save(&a.settings);
    qlog!("status: the clock and battery corner is %s (size %d)", if a.settings.status_on { "on" } else { "off" }, a.settings.status_size);
    if a.settings.status_on && !status_busy(a) {
        read_battery(); // (so its first drawing has the percent; else the next tick reads it)
    }
    ask(changed_rect(a));
}

/// Settings > Display > the size row: the next size, small to medium to large and round.
pub fn status_cycle_size(a: &mut App) {
    a.settings.status_size = (a.settings.status_size + 1) % STATUS_SIZES;
    settings_save(&a.settings);
    if a.settings.status_on {
        ask(changed_rect(a));
    }
}

/// The Kindle went to sleep or woke: the panel is the sleep screen's or about to be drawn whole,
/// and the pen's leaving may have gone unseen meanwhile.
pub fn status_reset() {
    with(|s| {
        s.pen_near = false;
        s.tap.cancel();
        s.drawn = RECT_EMPTY;
        s.drawn_text.clear();
        s.drawn_key = None;
        s.pending = RECT_EMPTY;
    });
    display_skip_status(RECT_EMPTY);
}

/// From app_tick, awake: reads the battery now and then and asks for a refresh when the minute or
/// the battery is not what the panel shows.
pub fn status_tick(a: &mut App, now: u32) {
    if !a.settings.status_on || status_busy(a) {
        return;
    }
    if with(|s| s.read_at == 0 || now.wrapping_sub(s.read_at) >= POLL_MS) {
        read_battery();
    }
    let key = (sys_wall_clock().div_euclid(60), with(|s| s.battery));
    if with(|s| s.drawn_key) != Some(key) {
        ask(changed_rect(a));
    }
}

// ---- painting it, for app_flush ----

/// The corner painted over a.screen for one refresh: the pixels it covers, to put back.
pub struct StatusCover {
    rect: Rect,
    under: Canvas,
}

fn set_drawn(l: Option<&Layout>) {
    let r = l.map_or(RECT_EMPTY, |l| l.rect);
    with(|s| {
        s.drawn = r;
        s.drawn_key = l.map(|l| l.key);
        s.drawn_text = l.map(|l| l.text.clone()).unwrap_or_default();
        s.drawn_charging = l.is_some_and(|l| l.bolt.is_some());
    });
    display_skip_status(r);
}

/// Paints the indicator into a.screen; the cover holds what was there. It counts as on the panel
/// from here (the refresh follows at once), even when there was no memory to save the pixels under
/// it (then nothing is painted, and the minute passes before it is tried again).
fn paint(a: &mut App, l: &Layout) -> Option<StatusCover> {
    set_drawn(Some(l));
    let mut under = Canvas::default();
    if canvas_init(&mut under, l.rect.w, l.rect.h) != 0 {
        return None;
    }
    canvas_blit(&mut under, &a.screen, l.rect, 0, 0);
    let clip = a.screen.clip;
    canvas_reset_clip(&mut a.screen);
    draw(&mut a.screen, l);
    a.screen.clip = clip;
    Some(StatusCover { rect: l.rect, under })
}

/// For a refresh of `dirty` in app_flush: when the indicator is up and the refresh reaches it,
/// paints it and widens dirty to hold it whole, and where it was if that was elsewhere (a size
/// that changed, a text that got shorter).
pub fn status_cover(a: &mut App, dirty: &mut Rect) -> Option<StatusCover> {
    if !a.settings.status_on {
        return None;
    }
    let l = layout(a);
    if !rect_overlaps(*dirty, l.rect) {
        return None;
    }
    *dirty = rect_union(*dirty, rect_union(l.rect, with(|s| s.drawn)));
    paint(a, &l)
}

/// Puts back what the cover painted over, after the refresh.
pub fn status_uncover(a: &mut App, mut c: StatusCover) {
    canvas_blit(&mut a.screen, &c.under, Rect { x: 0, y: 0, w: c.rect.w, h: c.rect.h }, c.rect.x, c.rect.y);
    canvas_free(&mut c.under);
}

/// A refresh of `r` was sent: it was the one asked for if it held that, and an indicator that is
/// off is gone from the panel if it held where it was.
pub fn status_presented(a: &App, r: Rect) {
    let off = !a.settings.status_on;
    let gone = with(|s| {
        if rect_contains_rect(r, s.pending) {
            s.pending = RECT_EMPTY;
        }
        off && !rect_is_empty(s.drawn) && rect_contains_rect(r, s.drawn)
    });
    if gone {
        set_drawn(None);
    }
}

/// A refresh of the corner alone, when one is asked for and the pen allows it: a small GL16 of the
/// indicator and of what it covered.
pub fn status_flush(a: &mut App) {
    let pending = with(|s| s.pending);
    if rect_is_empty(pending) || status_busy(a) {
        return;
    }
    let on = a.settings.status_on;
    if !on && rect_is_empty(with(|s| s.drawn)) {
        with(|s| s.pending = RECT_EMPTY); // nothing of it is on the panel: nothing to take down
        return;
    }
    let mut region = rect_intersect(pending, Rect { x: 0, y: 0, w: a.w, h: a.h });
    let mut cover = None;
    if on {
        let l = layout(a);
        region = rect_union(region, l.rect);
        cover = paint(a, &l);
    }
    display_present(&a.screen, region, RefreshMode::Ui);
    if let Some(c) = cover {
        status_uncover(a, c);
    }
    status_presented(a, region);
}

// ---- for the runner ----

/// The runner's status.* keys: "shown" 1 while the panel holds the indicator, "text" and "rect"
/// ("x y w h") what it says and where (empty while it is not shown), "charging" 1 when it shows the
/// bolt, "zone" the corner a tap finds ("x y w h"). The setting itself is "settings.status_on".
pub fn status_probe(a: &App, key: &str, out: &mut String, cap: usize) -> i32 {
    let (drawn, text, charging) = with(|s| (s.drawn, s.drawn_text.clone(), s.drawn_charging));
    let shown = !rect_is_empty(drawn);
    match key {
        "shown" => csnprintf!(out, cap, "%d", shown as i32),
        "text" => csnprintf!(out, cap, "%s", if shown { text.as_str() } else { "" }),
        "rect" => {
            if shown {
                csnprintf!(out, cap, "%d %d %d %d", drawn.x, drawn.y, drawn.w, drawn.h)
            } else {
                csnprintf!(out, cap, "%s", "")
            }
        }
        "charging" => csnprintf!(
            out,
            cap,
            "%s",
            if !shown {
                ""
            } else if charging {
                "1"
            } else {
                "0"
            }
        ),
        "zone" => {
            let z = status_zone(a);
            csnprintf!(out, cap, "%d %d %d %d", z.x, z.y, z.w, z.h)
        }
        _ => return -1,
    };
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        ui_set_theme(UiTheme::Classic);
        i18n_apply("en");
    }

    extern "C" {
        // (the libc crate does not declare it on every target)
        fn tzset();
    }

    /// A time of day in UTC: the text tests set TZ for the process (they run one at a time).
    fn utc() {
        std::env::set_var("TZ", "UTC");
        // SAFETY: tzset takes no arguments and only rereads TZ.
        unsafe { tzset() };
    }

    #[test]
    fn the_text_is_the_time_then_the_percent() {
        utc();
        up();
        // 2023-11-14 22:13:20 UTC
        let b = Battery { percent: 87, charging: false };
        assert_eq!(status_text(1700000000, &b), "Nov 14, 22:13  87%");
        // charging does not change the text (the bolt is drawn beside it)
        assert_eq!(status_text(1700000000, &Battery { percent: 87, charging: true }), "Nov 14, 22:13  87%");
        assert_eq!(status_text(1700000000, &Battery { percent: 5, charging: false }), "Nov 14, 22:13  5%");
        assert_eq!(status_text(1700000000, &Battery { percent: 100, charging: false }), "Nov 14, 22:13  100%");
        // without a battery reading, the time alone
        assert_eq!(status_text(1700000000, &Battery::default()), "Nov 14, 22:13");
        // the language in use: Spanish writes the day first
        i18n_apply("es");
        assert_eq!(status_text(1700000000, &b), "14 nov, 22:13  87%");
        i18n_apply("en");
    }

    #[test]
    fn the_time_follows_the_minute() {
        utc();
        up();
        let b = Battery { percent: 50, charging: false };
        let (t0, t1, t2) = (1700000000 - 20, 1700000000 - 20 + 59, 1700000000 - 20 + 60);
        assert_eq!(status_text(t0, &b), "Nov 14, 22:13  50%");
        assert_eq!(status_text(t1, &b), "Nov 14, 22:13  50%");
        assert_eq!(status_text(t2, &b), "Nov 14, 22:14  50%");
        // one key per minute, which is what the refresh compares
        assert_eq!(t0.div_euclid(60), t1.div_euclid(60));
        assert_ne!(t1.div_euclid(60), t2.div_euclid(60));
    }

    #[test]
    fn numbers_and_states_of_the_power_supply() {
        assert_eq!(battery_parse_number(b"87\n"), Some(87));
        assert_eq!(battery_parse_number(b" 100 "), Some(100));
        assert_eq!(battery_parse_number(b"0"), Some(0));
        assert_eq!(battery_parse_number(b""), None);
        assert_eq!(battery_parse_number(b"\n"), None);
        assert_eq!(battery_parse_number(b"87%"), None);
        assert_eq!(battery_parse_number(b"99999999999"), None);
        assert_eq!(battery_parse_number(b"\xff"), None);
        assert!(battery_parse_charging(b"Charging\n"));
        assert!(battery_parse_charging(b"charging"));
        assert!(!battery_parse_charging(b"Not charging\n"));
        assert!(!battery_parse_charging(b"Discharging\n"));
        assert!(!battery_parse_charging(b"Full\n"));
        assert!(!battery_parse_charging(b""));
    }

    fn supply(root: &std::path::Path, name: &str, files: &[(&str, &str)]) {
        let d = root.join(name);
        std::fs::create_dir_all(&d).unwrap();
        for (f, v) in files {
            std::fs::write(d.join(f), v).unwrap();
        }
    }

    #[test]
    fn the_battery_is_the_supply_of_that_type() {
        let root = std::env::temp_dir().join(format!("grimoire-status-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let r = root.to_str().unwrap();
        // no supplies, or none that is a battery
        assert_eq!(battery_from_sysfs(r), None);
        assert_eq!(battery_from_sysfs("/nonexistent/power_supply"), None);
        supply(&root, "a_charger", &[("type", "Mains\n"), ("online", "1\n")]);
        supply(&root, "b_usb", &[("type", "USB\n"), ("online", "0\n")]);
        assert_eq!(battery_from_sysfs(r), None);
        // a battery among them, picked by its type whatever the name or the order
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "64\n"), ("status", "Discharging\n")]);
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 64, charging: false }));
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "65\n"), ("status", "Charging\n")]);
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 65, charging: true }));
        // a capacity past 100 reads as 100; one that does not read passes the supply by
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "140\n"), ("status", "Full\n")]);
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 100, charging: false }));
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "n/a\n")]);
        assert_eq!(battery_from_sysfs(r), None);
        // a battery with no status file is not charging
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "30\n")]);
        std::fs::remove_file(root.join("c_bat").join("status")).ok();
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 30, charging: false }));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tap_is_short_still_and_in_the_zone() {
        let zone = Rect { x: 0, y: 0, w: 142, h: 142 };
        let mut t = Tap::default();
        // down and up in the zone, quickly
        t.down(zone, false, 60.0, 60.0, 1000);
        assert!(t.up(zone, 62.0, 61.0, 1120));
        // the same finger counts once
        assert!(!t.up(zone, 62.0, 61.0, 1130));
        // a control under the finger: not ours
        t.down(zone, true, 60.0, 60.0, 1000);
        assert!(!t.up(zone, 60.0, 60.0, 1100));
        // down outside the zone
        t.down(zone, false, 300.0, 60.0, 1000);
        assert!(!t.up(zone, 300.0, 60.0, 1100));
        // down in the zone, up outside it (a swipe), or moved past the slop on the way
        t.down(zone, false, 100.0, 100.0, 1000);
        assert!(!t.up(zone, 400.0, 100.0, 1100));
        t.down(zone, false, 20.0, 20.0, 1000);
        t.moved(120.0, 120.0);
        assert!(!t.up(zone, 21.0, 21.0, 1100));
        // held too long
        t.down(zone, false, 60.0, 60.0, 1000);
        assert!(!t.up(zone, 60.0, 60.0, 1000 + TAP_MS + 1));
        t.down(zone, false, 60.0, 60.0, 1000);
        assert!(t.up(zone, 60.0, 60.0, 1000 + TAP_MS));
        // a second finger or the pen takes it back
        t.down(zone, false, 60.0, 60.0, 1000);
        t.cancel();
        assert!(!t.up(zone, 60.0, 60.0, 1100));
        // the clock of the events wraps like now_ms does
        t.down(zone, false, 60.0, 60.0, u32::MAX - 50);
        assert!(t.up(zone, 60.0, 60.0, 70));
    }

    #[test]
    fn the_box_grows_with_the_size_and_holds_its_text() {
        utc();
        up();
        let b = Battery { percent: 87, charging: false };
        let t = 1700000000;
        let (s0, s1, s2) = (layout_of(0, t, &b), layout_of(1, t, &b), layout_of(2, t, &b));
        assert!(s0.rect.w < s1.rect.w && s1.rect.w < s2.rect.w);
        assert!(s0.rect.h < s1.rect.h && s1.rect.h < s2.rect.h);
        for l in [&s0, &s1, &s2] {
            // in the corner, inside the screen, and the text inside the box
            assert!(l.rect.x > 0 && l.rect.y > 0 && l.rect.w < 1860);
            assert!(rect_contains_rect(l.rect, l.text_r), "text {:?} in {:?}", l.text_r, l.rect);
            assert_eq!(l.text, "Nov 14, 22:13  87%");
            assert!(l.bolt.is_none());
        }
        // the large one still sits in a corner of an upright panel, not a quarter of its width
        assert!(s2.rect.w < 1860 / 2);
        // the bolt widens the box on its right and sits inside it
        let c = layout_of(1, t, &Battery { percent: 87, charging: true });
        assert!(c.rect.w > s1.rect.w);
        let bolt = c.bolt.unwrap();
        assert!(rect_contains_rect(c.rect, bolt) && bolt.x >= c.text_r.x + c.text_r.w - 2);
        // one box a minute, however the minute reads
        assert_eq!(layout_of(1, t, &b).key, layout_of(1, t + 10, &b).key);
        assert_ne!(layout_of(1, t, &b).key, layout_of(1, t + 60, &b).key);
        // without a percent the box is shorter
        assert!(layout_of(1, t, &Battery::default()).rect.w < s1.rect.w);
    }

    #[test]
    fn the_zone_is_twelve_millimetres_and_takes_the_indicator() {
        utc();
        up();
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        let z = status_zone(&a);
        assert_eq!(z, Rect { x: 0, y: 0, w: 142, h: 142 });
        a.dpi = 150.0;
        assert_eq!(status_zone(&a).w, 71);
        // while the indicator is up, its box is part of the corner
        let l = layout_of(2, 1700000000, &Battery { percent: 87, charging: true });
        set_drawn(Some(&l));
        let z = status_zone(&a);
        assert!(rect_contains_rect(z, l.rect) && z.w >= l.rect.x + l.rect.w);
        status_reset();
        assert_eq!(status_zone(&a).w, 71);
    }

    #[test]
    fn the_probe_answers_for_the_panel() {
        utc();
        up();
        status_reset();
        let a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        let mut out = String::new();
        assert_eq!(status_probe(&a, "shown", &mut out, 64), 0);
        assert_eq!(out, "0");
        status_probe(&a, "text", &mut out, 64);
        assert_eq!(out, "");
        status_probe(&a, "rect", &mut out, 64);
        assert_eq!(out, "");
        let l = layout_of(0, 1700000000, &Battery { percent: 9, charging: true });
        set_drawn(Some(&l));
        status_probe(&a, "shown", &mut out, 64);
        assert_eq!(out, "1");
        status_probe(&a, "text", &mut out, 64);
        assert_eq!(out, "Nov 14, 22:13  9%");
        status_probe(&a, "charging", &mut out, 64);
        assert_eq!(out, "1");
        status_probe(&a, "rect", &mut out, 64);
        assert_eq!(out, format!("{} {} {} {}", l.rect.x, l.rect.y, l.rect.w, l.rect.h));
        assert_eq!(status_probe(&a, "nope", &mut out, 64), -1);
        status_reset();
    }

    #[test]
    fn the_bolt_and_the_clock_are_in_the_font() {
        up();
        // The glyphs the corner and the Display sheet draw must exist in the embedded face, or they
        // draw nothing.
        for cp in [BOLT_CP, 0xF0150] {
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, 64, 64), 0);
            canvas_fill(&mut c, Rect { x: 0, y: 0, w: 64, h: 64 }, 255);
            font_draw_icon(&mut c, FontFace::Regular, cp, Rect { x: 0, y: 0, w: 64, h: 64 }, 0);
            assert!(c.px.iter().any(|&p| p < 128), "U+{cp:X} draws nothing");
        }
    }

    #[test]
    fn the_pen_holds_the_refresh_back() {
        status_reset();
        let a = App::default();
        let ev = |t| InEvent { type_: t, ..Default::default() };
        assert!(!status_busy(&a));
        // near from its first event of any kind, until it leaves
        for t in [InType::PenHover, InType::PenDown, InType::PenMove, InType::PenUp] {
            status_pen(&ev(t));
            assert!(status_busy(&a), "{t:?}");
            status_pen(&ev(InType::PenLeave));
            assert!(!status_busy(&a), "{t:?}");
        }
        // a finger or a power key is not the pen
        status_pen(&ev(InType::TouchDown));
        status_pen(&ev(InType::Power));
        assert!(!status_busy(&a));
        // a pen that was near when the Kindle slept is not near after it
        status_pen(&ev(InType::PenHover));
        status_reset();
        assert!(!status_busy(&a));
        // the driver in pen mode holds it back too, where there is one that says so (the runner's)
        #[cfg(feature = "sim")]
        {
            display_pen_mode(true);
            assert!(status_busy(&a));
            display_pen_mode(false);
            assert!(!status_busy(&a));
        }
    }

    // (The host's panel is none, so a refresh is sent to nowhere here; the runner's panel is another
    // thread's state in a test, and the scenario y_status_corner.qs covers that build.)
    #[cfg(not(any(feature = "sim", feature = "kindle")))]
    #[test]
    fn a_refresh_waits_for_the_pen_and_goes_once() {
        utc();
        up();
        status_reset();
        let mut a = App { w: 600, h: 400, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, 600, 400), 0);
        canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w: 600, h: 400 }, 200);
        let before = a.screen.px.clone();
        a.settings.status_on = true;
        let r = Rect { x: 6, y: 6, w: 300, h: 50 };
        ask(r);
        status_pen(&InEvent { type_: InType::PenHover, ..Default::default() });
        status_flush(&mut a);
        assert_eq!(with(|s| s.pending), r, "kept while the pen is near");
        assert!(rect_is_empty(with(|s| s.drawn)));
        status_pen(&InEvent { type_: InType::PenLeave, ..Default::default() });
        status_flush(&mut a);
        assert!(rect_is_empty(with(|s| s.pending)), "sent once the pen is gone");
        assert!(!rect_is_empty(with(|s| s.drawn)));
        assert_eq!(a.screen.px, before, "and the screen holds none of it");
        // off, what it covered goes with one more, and then nothing is asked for
        a.settings.status_on = false;
        ask(with(|s| s.drawn));
        status_flush(&mut a);
        assert!(rect_is_empty(with(|s| s.pending)) && rect_is_empty(with(|s| s.drawn)));
        status_flush(&mut a);
        status_reset();
    }

    #[test]
    fn the_painted_indicator_is_put_back() {
        utc();
        up();
        status_reset();
        let mut a = App { w: 600, h: 400, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, 600, 400), 0);
        canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w: 600, h: 400 }, 200);
        a.settings.status_on = true;
        with(|s| s.battery = Battery { percent: 42, charging: true });
        let before = a.screen.px.clone();
        let mut dirty = Rect { x: 0, y: 0, w: 600, h: 400 };
        let cover = status_cover(&mut a, &mut dirty).expect("covered");
        // painted: the box is white inside and black at its text, the rest of the screen untouched
        let r = cover.rect;
        assert!(a.screen.px != before);
        let outside = |x: i32, y: i32| !rect_contains(r, x, y);
        for y in 0..400 {
            for x in 0..600 {
                if outside(x, y) {
                    assert_eq!(a.screen.px[(y * a.screen.stride + x) as usize], 200);
                }
            }
        }
        assert!(a.screen.px.contains(&255) && a.screen.px.contains(&0));
        // it is on the panel from here, with its text and its bolt
        let mut out = String::new();
        status_probe(&a, "shown", &mut out, 8);
        assert_eq!(out, "1");
        status_probe(&a, "charging", &mut out, 8);
        assert_eq!(out, "1");
        status_uncover(&mut a, cover);
        assert_eq!(a.screen.px, before);
        // a refresh elsewhere does not touch it
        let mut far = Rect { x: 300, y: 300, w: 50, h: 50 };
        assert!(status_cover(&mut a, &mut far).is_none() && far == Rect { x: 300, y: 300, w: 50, h: 50 });
        // off, it is not painted at all
        a.settings.status_on = false;
        let mut dirty = Rect { x: 0, y: 0, w: 600, h: 400 };
        assert!(status_cover(&mut a, &mut dirty).is_none());
        status_presented(&a, dirty);
        status_probe(&a, "shown", &mut out, 8);
        assert_eq!(out, "0");
        status_reset();
    }
}
