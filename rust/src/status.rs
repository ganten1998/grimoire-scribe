//! status.rs: the clock and battery corner. A small indicator in one corner of a notebook page (settings.ini
//! status_corner: the top left unless the owner picks another): the time (format_time_short, as the app
//! writes times everywhere), a pixel art battery that shows the charge and, while it charges, a bolt, and the
//! percent. A finger tap in a 14 mm square in that corner turns it on and off; the shortcut action
//! `status-toggle` does the same, `status-corner` (and the Display sheet's row) moves it to the next
//! corner, and the Display sheet sets its size (settings.ini status_on, status_corner, status_size;
//! docs/FORMAT.md).
//!
//! It shows on the notebook page itself and nowhere else: not on the Library, Settings, Share or Pen
//! feel, and not while a sheet or menu is in front of the page (the Pages view, the Links bar, Find,
//! Display, Paper, the tool picker and the keyboards). `wanted` is that rule; a tap works under it too:
//! no other screen takes a tap in the corner for the clock.
//!
//! Rust build only: the C tree has no such thing, so nothing here is a port.
//!
//! It is an overlay, like a toast and Live view's chip: app_flush paints it over a.screen just for
//! the refresh and puts the pixels under it straight back, so it is never in a page, an export, a
//! thumbnail, a Live view frame (live_push comes before it) or the sleep screen (app_sleep presents
//! a.screen as it is). The screen guard leaves its rect out (display_skip_status).
//!
//! It refreshes itself at each minute and when the battery or charging changes: one small GL16 of
//! its rect (and of what it covered, when it shrinks, moves or goes, each a refresh of its own: two
//! corners are never one refresh across the screen), through display_present like every other
//! refresh. No display waits of any kind. Nothing is sent while the pen is near, a stroke is in
//! progress or the driver is in pen mode (a grayscale refresh would end that mode and wait for the
//! pen's last frame, display_fbink.rs pen_leave): the corner waits for the pen to leave. A finger the
//! touch panel lets through is never the pen's hand, so a tap clears a "near" the pen's leaving left
//! behind: one tap is always one toggle, at once, with nothing to wait out.
//!
//! The toggle is decided at the finger's lift: down in the zone, on no control, lifted in the zone (or within
//! 5 mm of where it landed) and up within 800 ms. The path between is not asked: a deliberate press flattens
//! the fingertip and its centroid drifts, which lost the slow taps while light fast ones passed. Every
//! decision (down, up) is one grimoire.log line. The setting is saved after the refresh went out,
//! not before it (a save waits for the storage twice).

use crate::prelude::*;
use std::cell::RefCell;

/// The tap zone's side, in mm: a corner square. The indicator joins it while it is up, so a tap on it turns it off.
const ZONE_MM: f32 = 14.0;
/// A finger that moves farther than this from where it landed is not tapping.
const SLOP_MM: f32 = 5.0;
/// Around the box's place, a tap still finds the corner: 4 mm.
const PAD_MM: f32 = 4.0;
/// A finger down longer than this is not a tap (a long press is the page's, a link's). A deliberate press
/// is slow: 800 ms.
const TAP_MS: u32 = 800;
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
/// How many corners there are: settings.ini status_corner 0 top left, 1 top right, 2 bottom right, 3 bottom left.
pub const STATUS_CORNERS: i32 = 4;

/// The battery's pixel art, in units (one unit is a few screen px, so the art keeps its look at every size):
/// a body 16 wide and 9 tall with its notched corners, a terminal 2 wide, and inside the body up to
/// FILL_COLS columns of charge 5 tall, one unit in from the outline.
const ICON_UW: i32 = 18;
const ICON_UH: i32 = 9;
pub const FILL_COLS: i32 = 12;
/// The charging bolt, 5 by 7 units, centred in the body's inside (which is 14 by 7).
const BOLT: [&[u8; 5]; 7] = [b"...XX", b"..XX.", b".XX..", b"XXXXX", b"..XX.", b".XX..", b"XX..."];

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
/// the percent; just the time where the battery cannot be read. This is what the runner's `status.text`
/// reads: the percent is drawn right of the battery, which sits between the two.
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


// ---- where the corners are ----

/// The corner's square of `side` px on a w by h screen: corner 0 top left, 1 top right, 2 bottom right, 3 bottom
/// left (clockwise from the top left).
pub fn corner_square(corner: i32, w: i32, h: i32, side: i32) -> Rect {
    let side = side.min(w).min(h);
    let right = corner == 1 || corner == 2;
    let bottom = corner == 2 || corner == 3;
    Rect { x: if right { w - side } else { 0 }, y: if bottom { h - side } else { 0 }, w: side, h: side }
}

/// A box of bw by bh px in that corner, m px from the two edges it is nearest.
fn corner_box(corner: i32, w: i32, h: i32, bw: i32, bh: i32, m: i32) -> Rect {
    let right = corner == 1 || corner == 2;
    let bottom = corner == 2 || corner == 3;
    Rect { x: if right { w - m - bw } else { m }, y: if bottom { h - m - bh } else { m }, w: bw, h: bh }
}

/// The clock corner overlays the page; no status strip is reserved.
pub fn status_reserved(_a: &App) -> i32 { 0 }

/// Compatibility for the retired status-mode action.
pub fn status_cycle_mode(a: &mut App) { status_toggle(a); }

fn bar_layout(a: &App, b: &Battery) -> Layout {
    let mut l = layout_of(0, 0, a.w, a.h, sys_wall_clock(), b);
    let h = status_reserved(a);
    let pad = ui_px(16.0);
    let half = h / 2;
    let clock_w = l.rect.w + pad;
    let clock_x = (a.w - clock_w).max(a.w / 2);
    let dx = clock_x - l.rect.x;
    let dy = (half - l.rect.h) / 2 - l.rect.y;
    l.time_r.x += dx; l.time_r.y += dy;
    l.pct_r.x += dx; l.pct_r.y += dy;
    if let Some(r) = l.icon.as_mut() { r.x += dx; r.y += dy; }
    l.rect = Rect { x: 0, y: 0, w: a.w, h };
    if let Some((book, page, links)) = crate::editor::editor_status_text(a) {
        let mut add = |x, y, w, text: String, icon| l.cells.push(BarCell { rect: Rect { x, y, w, h: half }, text, icon });
        add(pad, 0, (clock_x - pad * 2).max(1), book, IconKind::None);
        let page_end = a.w * 38 / 100;
        let links_end = a.w * 60 / 100;
        let tool_end = a.w - ui_px(132.0);
        add(pad, half, page_end - pad, page, IconKind::None);
        add(page_end, half, links_end - page_end, links, IconKind::Link);
        add(links_end, half, (tool_end - links_end).max(1), tool_label(a.settings.current_tool).to_string(), ui_tool_icon(a.settings.current_tool.0 as i32));
        add(tool_end, half, ui_px(56.0), String::new(), if live_on() { IconKind::Live } else { IconKind::None });
        let mut hids = [InputHid::default(); 16];
        input_hids(&mut hids);
        let connected = hids.iter().any(|h| h.gamepad && !h.held && !h.name.is_empty());
        add(tool_end + ui_px(64.0), half, ui_px(56.0), String::new(), if connected { IconKind::Gamepad } else { IconKind::None });
    }
    l.text = format!("{}  {}", l.text, l.cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("  "));
    l
}

// ---- a tap in the corner ----

/// A finger that went down in the zone on no control, followed to its lift.
#[derive(Clone, Copy, Debug, Default)]
struct Tap {
    armed: bool,
    x0: f32,
    y0: f32,
    t0: u32,
}

impl Tap {
    /// The first finger went down: it is a candidate only where `hit` says (inside the zone, no control there).
    fn down(&mut self, hit: bool, x: f32, y: f32, t: u32) {
        *self = if hit { Tap { armed: true, x0: x, y0: y, t0: t } } else { Tap::default() };
    }
    fn far(&self, x: f32, y: f32, slop: f32) -> bool {
        (x - self.x0).hypot(y - self.y0) > slop
    }
    /// The finger lifted: Ok when it was a tap, else why not. A press that landed in the zone and lifts in it
    /// (`in_zone`, or within `slop` of where it landed: a press at the zone's edge rolls out) within TAP_MS is
    /// a tap, whatever the fingertip did between: a deliberate press flattens and its centroid drifts, so the
    /// path is not asked, only the two ends.
    fn up(&mut self, x: f32, y: f32, t: u32, in_zone: bool, slop: f32) -> Result<(), &'static str> {
        if !self.armed {
            return Err("no tap was armed");
        }
        self.armed = false;
        if t.wrapping_sub(self.t0) > TAP_MS {
            return Err("held too long");
        }
        if !in_zone && self.far(x, y, slop) {
            return Err("lifted away from the zone");
        }
        Ok(())
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
    /// what the panel shows now: the rect (empty for nothing), what it says, its battery icon and how much
    /// of it is lit, and the minute and battery that says
    drawn: Rect,
    drawn_text: String,
    drawn_charging: bool,
    drawn_icon: Rect,
    drawn_fill: i32,
    drawn_key: Option<(i64, Battery)>,
    /// refreshes asked for and not sent yet: a few rects, apart (empty for none)
    pending: Vec<Rect>,
    /// the touch event status_touch saw last was the lift of a tap in the corner, which turned the
    /// indicator on or off (status_tapped: the screen that handles that same event asks)
    tapped: bool,
    /// a setting changed that settings.ini does not hold yet: saved after the refresh went out
    save_due: bool,
    bar_cells: Vec<BarCell>,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// ---- where it goes and what it looks like ----

/// The measures of the indicator at a size.
struct Metrics {
    px: f32,
    padx: i32,
    /// the box's height
    bh: i32,
    /// the battery art's unit, and its height
    unit: i32,
    ih: i32,
}

fn metrics(size: i32) -> Metrics {
    let px = ui_px(SIZE_PX[clampi(size, 0, STATUS_SIZES - 1) as usize]) as f32;
    let face = FontFace::Fixed;
    let lh = font_ascent(face, px) + font_descent(face, px);
    let pady = (px * 0.3) as i32;
    // (a unit a tenth of the text's height: the art stands about as tall as the digits beside it)
    let unit = ((px / 9.5).round() as i32).max(2);
    let ih = ICON_UH * unit;
    Metrics { px, padx: (px * 0.5) as i32, bh: (lh + 2 * pady).max(ih + 2 * ui_px(3.0)), unit, ih }
}

/// What the indicator says and where it sits.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BarCell { rect: Rect, text: String, icon: IconKind }

struct Layout {
    cells: Vec<BarCell>,
    rect: Rect,
    /// "Oct 10, 14:32  87%", as `status.text` reads it
    text: String,
    time: String,
    time_r: Rect,
    pct: String,
    pct_r: Rect,
    /// the battery art, with no battery reading none
    icon: Option<Rect>,
    unit: i32,
    battery: Battery,
    px: f32,
    /// the minute and battery it shows
    key: (i64, Battery),
}

/// How many of the FILL_COLS columns a charge lights: the nearest to its share, one for any charge at all.
pub fn battery_fill(percent: i32) -> i32 {
    if percent <= 0 {
        return 0;
    }
    clampi((percent * FILL_COLS + 50) / 100, 1, FILL_COLS)
}

/// The indicator for a size, a corner of a w by h screen, a time and a battery: the time, the battery's
/// art and then its percent, left to right in every corner. Its text is in the monospaced face every
/// theme shares, so its box changes only with the number of characters.
fn layout_of(size: i32, corner: i32, sw: i32, sh: i32, t: i64, b: &Battery) -> Layout {
    let m = metrics(size);
    let mut time = String::new();
    format_time_short(t, &mut time, 64);
    let face = FontFace::Fixed;
    let tw = font_measure(face, m.px, &time);
    let has_battery = b.percent >= 0;
    let pct = if has_battery { cformat!("%d%%", b.percent) } else { String::new() };
    let pw = if has_battery { font_measure(face, m.px, &pct) } else { 0 };
    let (iw, g1, g2) = if has_battery { (ICON_UW * m.unit, (m.px * 0.6) as i32, (m.px * 0.3) as i32) } else { (0, 0, 0) };
    let bw = m.padx + tw + g1 + iw + g2 + pw + m.padx;
    let rect = corner_box(corner, sw, sh, bw, m.bh, ui_px(6.0));
    let time_r = Rect { x: rect.x + m.padx, y: rect.y, w: tw + 2, h: rect.h };
    let ix = time_r.x + tw + g1;
    let icon = has_battery.then_some(Rect { x: ix, y: rect.y + (rect.h - m.ih) / 2, w: iw, h: m.ih });
    Layout {
        cells: Vec::new(),
        rect,
        text: status_text(t, b),
        time,
        time_r,
        pct,
        pct_r: Rect { x: ix + iw + g2, y: rect.y, w: pw + 2, h: rect.h },
        icon,
        unit: m.unit,
        battery: *b,
        px: m.px,
        key: (t.div_euclid(60), *b),
    }
}

/// The band along the top or bottom of the screen that the indicator of the current size stays inside,
/// whatever it says: its box starts at the margin and is as tall as layout_of makes it. Asked
/// before a layout, which costs a clock, a format and a measure, so a refresh far away costs none.
fn band(a: &App) -> Rect {
    if status_reserved(a) > 0 { return Rect { x: 0, y: 0, w: a.w, h: status_reserved(a) }; }
    let h = ui_px(6.0) + metrics(a.settings.status_size).bh;
    let bottom = a.settings.status_corner == 2 || a.settings.status_corner == 3;
    Rect { x: 0, y: if bottom { a.h - h } else { 0 }, w: a.w, h }
}

fn layout(a: &App) -> Layout {
    let b = with(|s| s.battery);
    if status_reserved(a) > 0 { bar_layout(a, &b) } else {
        layout_of(a.settings.status_size, a.settings.status_corner, a.w, a.h, sys_wall_clock(), &b)
    }
}

/// The battery in pixel art in r (ICON_UW by ICON_UH units of u px): an outline with notched corners, its
/// terminal, the charge as a bar of lit columns (battery_fill) and, while it charges, the bolt over it
/// on a white halo, so it reads over any charge.
fn draw_battery(c: &mut Canvas, r: Rect, u: i32, b: &Battery) {
    let cell = |c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, gray: u8| {
        canvas_fill(c, Rect { x: r.x + x * u, y: r.y + y * u, w: w * u, h: h * u }, gray)
    };
    cell(c, 1, 0, 14, 1, 0);
    cell(c, 1, 8, 14, 1, 0);
    cell(c, 0, 1, 1, 7, 0);
    cell(c, 15, 1, 1, 7, 0);
    cell(c, 16, 2, 2, 5, 0);
    let n = battery_fill(b.percent);
    if n > 0 {
        cell(c, 2, 2, n, 5, 0);
    }
    if b.charging {
        let lit = |f: &mut dyn FnMut(i32, i32)| {
            for (dy, row) in BOLT.iter().enumerate() {
                for (dx, &ch) in row.iter().enumerate() {
                    if ch == b'X' {
                        f(6 + dx as i32, 1 + dy as i32);
                    }
                }
            }
        };
        lit(&mut |bx, by| {
            for ny in by - 1..=by + 1 {
                for nx in bx - 1..=bx + 1 {
                    if (1..=14).contains(&nx) && (1..=7).contains(&ny) {
                        cell(c, nx, ny, 1, 1, 255);
                    }
                }
            }
        });
        lit(&mut |bx, by| cell(c, bx, by, 1, 1, 0));
    }
}

fn draw(c: &mut Canvas, l: &Layout) {
    if !l.cells.is_empty() {
        canvas_fill(c, l.rect, 255);
        canvas_fill(c, Rect { x: 0, y: l.rect.h - ui_px(2.0), w: l.rect.w, h: ui_px(2.0) }, 0);
        for cell in &l.cells {
            let mut r = cell.rect;
            if cell.icon != IconKind::None {
                let side = ui_px(36.0);
                ui_icon(c, cell.icon, Rect { x: r.x, y: r.y + (r.h - side) / 2, w: side, h: side }, 0);
                r.x += side + ui_px(6.0); r.w -= side + ui_px(10.0);
            }
            // ui_text shrinks/truncates at UTF-8 boundaries through the shared font renderer.
            font_draw_left_name(c, FontFace::Regular, ui_px(30.0) as f32, r, &cell.text, 0);
        }
    } else {
    ui_frame(c, l.rect, ui_px(8.0), 255, true, 0);
    ui_frame(c, l.rect, ui_px(8.0), 0, false, ui_px(2.0));
    }
    font_draw_left(c, FontFace::Fixed, l.px, l.time_r, &l.time, 0);
    if let Some(ir) = l.icon {
        draw_battery(c, ir, l.unit, &l.battery);
        font_draw_left(c, FontFace::Fixed, l.px, l.pct_r, &l.pct, 0);
    }
}

/// The square a tap finds in the corner the setting names: 14 mm.
pub fn status_square(a: &App) -> Rect {
    let side = (ZONE_MM * a.dpi / 25.4).round() as i32;
    corner_square(a.settings.status_corner, a.w, a.h, side)
}

/// The corner's zone: its square and where its box shows with the room a tap has around it, together (what
/// a card or keyboard docked at the top keeps below, so it never meets the corner).
pub fn status_zone(a: &App) -> Rect {
    let pad = mm_to_px(PAD_MM, a.dpi).round() as i32;
    let b = layout(a).rect;
    let around = rect_intersect(Rect { x: b.x - pad, y: b.y - pad, w: b.w + 2 * pad, h: b.h + 2 * pad }, Rect { x: 0, y: 0, w: a.w, h: a.h });
    rect_union(status_square(a), around)
}

/// Whether (x, y) is in the corner as a tap finds it: its square, the indicator's box while it is up, and
/// where its box shows in the corner the setting names, with PAD_MM around it, up or not. (The owner's
/// taps to bring it back went where it had been: the box is twice as wide as the square, and half of them
/// fell outside it.) Separate places, never the box around them: a corner just moved leaves the old box
/// on the panel until its refresh goes, and that must not make the screen between a tap's target.
pub fn status_hit(a: &App, x: f32, y: f32) -> bool {
    let (px, py) = (x as i32, y as i32);
    let pad = mm_to_px(PAD_MM, a.dpi).round() as i32;
    let b = layout(a).rect;
    let around = rect_intersect(Rect { x: b.x - pad, y: b.y - pad, w: b.w + 2 * pad, h: b.h + 2 * pad }, Rect { x: 0, y: 0, w: a.w, h: a.h });
    rect_contains(status_square(a), px, py) || rect_contains(around, px, py) || rect_contains(with(|s| s.drawn), px, py)
}

/// Whether the clock and battery may show on this screen now: the notebook's page is in front, with no sheet,
/// menu, Pages view or keyboard over it. The only place it shows, and the only place a tap in the corner is
/// its.
fn page_in_front(a: &App) -> bool {
    #[cfg(test)]
    if let Some(f) = TEST_FRONT.with(|c| c.get()) {
        return f;
    }
    a.scr == ScreenId::Editor && editor_page_in_front(a)
}

// (The unit tests have no notebook: they say whether the page is in front.)
#[cfg(test)]
thread_local! {
    static TEST_FRONT: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Whether the panel should hold the indicator now: it is on, and the page is in front.
fn wanted(a: &App) -> bool {
    a.settings.status_on && page_in_front(a)
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

/// Whether the touch event status_touch was given last was the lift of a corner tap, which turned the
/// indicator on or off. app_event hands every touch to status_touch before the screen, so the screen
/// that handles the same event asks here, and leaves that tap to the corner.
pub fn status_tapped() -> bool {
    with(|s| s.tapped)
}

/// Whether the corner must hold its refreshes back: the pen is near, a stroke or another gesture
/// is in progress, or the driver is in pen mode.
fn status_busy(a: &App) -> bool {
    with(|s| s.pen_near) || editor_busy(a) || display_pen_mode_on()
}

/// A finger's events, before the screen sees them (it sees them all as ever; the corner only
/// watches): a tap in the zone with no control under it turns the indicator on or off. Only the page
/// has the corner (page_in_front); a sheet opened under a finger takes the corner's tap with it. app_event
/// calls it awake only.
pub fn status_touch(a: &mut App, ev: &InEvent) {
    with(|s| s.tapped = false);
    if !page_in_front(a) {
        with(|s| s.tap.cancel());
        return;
    }
    let slop = mm_to_px(SLOP_MM, a.dpi);
    match ev.type_ {
        InType::TouchDown => {
            // A finger the touch panel let through is not the writing hand: the pen is not near, whatever a
            // pen leaving without a word left behind. (Asked before the screen handles the touch: what it
            // does may change what is there.)
            with(|s| s.pen_near = false);
            let in_zone = status_hit(a, ev.x, ev.y);
            let ctl = in_zone && editor_control_at(a, ev.x, ev.y);
            if in_zone {
                if ctl {
                    qlog!("status: corner down at %d,%d is on a control: not the clock's", ev.x as i32, ev.y as i32);
                } else {
                    qlog!("status: corner down at %d,%d in the zone: armed", ev.x as i32, ev.y as i32);
                }
            }
            with(|s| s.tap.down(in_zone && !ctl, ev.x, ev.y, ev.t_ms));
        }
        // (the path between the two ends is not asked)
        InType::TouchMove => {}
        InType::TouchUp if ev.value & TOUCH_F_CANCEL != 0 => {
            if with(|s| s.tap.armed) {
                qlog!("status: corner up: the touch was cancelled (a second finger, a palm or the pen): no toggle");
            }
            with(|s| s.tap.cancel())
        }
        InType::TouchUp => {
            let in_zone = status_hit(a, ev.x, ev.y);
            let armed = with(|s| s.tap.armed);
            match with(|s| s.tap.up(ev.x, ev.y, ev.t_ms, in_zone, slop)) {
                Ok(()) => {
                    qlog!("status: corner up at %d,%d: toggled", ev.x as i32, ev.y as i32);
                    status_toggle(a);
                    with(|s| s.tapped = true);
                }
                Err(why) => {
                    if armed {
                        qlog!("status: corner up at %d,%d: no toggle, %s", ev.x as i32, ev.y as i32, why);
                    }
                }
            }
        }
        InType::PinchBegin | InType::PinchMove | InType::PinchEnd => {
            if with(|s| s.tap.armed) {
                qlog!("status: corner tap dropped: a pinch began");
            }
            with(|s| s.tap.cancel())
        }
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

/// Asks for a refresh of r: joined to one asked for already that it touches, else one of its own.
fn ask(r: Rect) {
    if rect_is_empty(r) {
        return;
    }
    with(|s| {
        for p in s.pending.iter_mut() {
            if rect_overlaps(*p, r) {
                *p = rect_union(*p, r);
                return;
            }
        }
        if s.pending.len() < 4 {
            s.pending.push(r);
        } else {
            s.pending[0] = rect_union(s.pending[0], r);
        }
    });
}

/// The indicator changed (on, off, size, corner): a refresh of what it covers now, and one of what it covered.
fn ask_change(a: &App) {
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    if wanted(a) {
        ask(rect_intersect(layout(a).rect, screen));
    }
    ask(rect_intersect(with(|s| s.drawn), screen));
}

/// What a change says when the page is not in front, so nothing shows: a toast, so a press of a shortcut
/// is never a silent one.
fn say(a: &mut App, msg: &str) {
    app_toast(a, msg, 2500);
}

/// The setting changed: settings.ini is written after the refresh (status_flush).
fn save_later() {
    with(|s| s.save_due = true);
}

/// Turns the indicator on or off: a tap in the corner, or the status-toggle shortcut.
pub fn status_toggle(a: &mut App) {
    a.settings.status_mode = 1;
    a.settings.status_on = !a.settings.status_on;
    save_later();
    qlog!(
        "status: the clock and battery corner is %s (size %d, corner %d)",
        if a.settings.status_on { "on" } else { "off" },
        a.settings.status_size,
        a.settings.status_corner
    );
    let front = page_in_front(a);
    if a.settings.status_on && front && !status_busy(a) {
        read_battery(); // (so its first drawing has the percent; else the next tick reads it)
    }
    ask_change(a);
    if !front {
        if a.settings.status_on {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the clock and battery were turned
            // on with a shortcut where no notebook page is in front; they show on a page only.
            say(a, tr("Clock and battery on. They show on a notebook page."));
        } else {
            // TRANSLATORS: a toast: the clock and battery were turned off with a shortcut.
            say(a, tr("Clock and battery off."));
        }
    }
}

/// Settings > Display > the size row: the next size, small to medium to large and round.
pub fn status_cycle_size(a: &mut App) {
    a.settings.status_size = (a.settings.status_size + 1) % STATUS_SIZES;
    save_later();
    ask_change(a);
}

/// The corner's name for a toast, one sentence for each (settings.ini status_corner 0, 1, 2, 3).
static K_CORNER_TOASTS: [&str; STATUS_CORNERS as usize] = [
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines) when the status-corner shortcut moves the
    // clock and battery where no notebook page is in front: which corner of the page they sit in now.
    n_("Clock and battery: top left corner of the page"),
    n_("Clock and battery: top right corner of the page"),
    n_("Clock and battery: bottom right corner of the page"),
    n_("Clock and battery: bottom left corner of the page"),
];

/// The next corner, clockwise from the top left and round (the status-corner shortcut, the Display sheet's row).
pub fn status_cycle_corner(a: &mut App) {
    a.settings.status_corner = (a.settings.status_corner + 1) % STATUS_CORNERS;
    save_later();
    qlog!("status: the clock and battery corner moves to corner %d", a.settings.status_corner);
    ask_change(a);
    if !page_in_front(a) {
        say(a, tr(K_CORNER_TOASTS[clampi(a.settings.status_corner, 0, STATUS_CORNERS - 1) as usize]));
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
        s.drawn_icon = RECT_EMPTY;
        s.drawn_fill = 0;
        s.drawn_key = None;
        s.bar_cells.clear();
        s.pending.clear();
        // (The battery ran down meanwhile: the first tick awake asks again. The kernel answers at
        // once; lipc with its last reading until the next one is due.)
        s.read_at = 0;
    });
    display_skip_status(RECT_EMPTY);
}

/// A setting waits to be saved (the corner's refresh went first): written now, as the Kindle sleeps.
pub fn status_save_now(a: &App) {
    if with(|s| std::mem::take(&mut s.save_due)) {
        settings_save(&a.settings);
    }
}

/// From app_tick, awake: reads the battery now and then and asks for a refresh when the minute or
/// the battery is not what the panel shows, or when the indicator is on the panel and the page no
/// longer wants it there (a sheet opened, the screen changed: only its own rect is taken down).
pub fn status_tick(a: &mut App, now: u32) {
    sys_battery_reap(); // (a reading under way ends, or is stopped, even with the corner off or held back)
    let want = wanted(a);
    let drawn = with(|s| s.drawn);
    if (!want && rect_is_empty(drawn)) || status_busy(a) {
        return;
    }
    if !want {
        ask(drawn);
        return;
    }
    if with(|s| s.read_at == 0 || now.wrapping_sub(s.read_at) >= POLL_MS) {
        read_battery();
    }
    let key = (sys_wall_clock().div_euclid(60), with(|s| s.battery));
    if status_reserved(a) > 0 {
        let l = layout(a);
        let old = with(|s| s.bar_cells.clone());
        if drawn != l.rect || old.len() != l.cells.len() { ask(l.rect); }
        else {
            for (before, after) in old.iter().zip(&l.cells) {
                if before != after { ask(after.rect); }
            }
            if with(|s| s.drawn_key) != Some(key) {
                ask(Rect { x: l.time_r.x, y: 0, w: a.w - l.time_r.x, h: l.rect.h / 2 });
            }
        }
    } else if with(|s| s.drawn_key) != Some(key) { ask_change(a); }
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
        s.bar_cells = l.map(|l| l.cells.clone()).unwrap_or_default();
        s.drawn_key = l.map(|l| l.key);
        s.drawn_text = l.map(|l| l.text.clone()).unwrap_or_default();
        s.drawn_charging = l.is_some_and(|l| l.battery.charging && l.icon.is_some());
        s.drawn_icon = l.and_then(|l| l.icon).unwrap_or(RECT_EMPTY);
        s.drawn_fill = l.map_or(0, |l| if l.icon.is_some() { battery_fill(l.battery.percent) } else { 0 });
    });
    display_skip_status(r);
}

/// Paints the indicator into a.screen; the cover holds what was there. It counts as on the panel from
/// here (the refresh follows at once), even when there was no memory to save the pixels under it (then
/// nothing is painted, and the minute passes before it is tried again).
fn paint(a: &mut App, l: &Layout) -> Option<StatusCover> {
    set_drawn(Some(l));
    paint_into(a, l)
}

// The indicator into a.screen, and into Live view's layer (live_layer: the browser shows it over the
// page, as the panel shows it over the screen); the cover holds what was there.
fn paint_into(a: &mut App, l: &Layout) -> Option<StatusCover> {
    let mut under = Canvas::default();
    if canvas_init(&mut under, l.rect.w, l.rect.h) != 0 {
        return None;
    }
    canvas_blit(&mut under, &a.screen, l.rect, 0, 0);
    let clip = a.screen.clip;
    canvas_reset_clip(&mut a.screen);
    draw(&mut a.screen, l);
    a.screen.clip = clip;
    live_layer(LIVE_LAYER_STATUS, &a.screen, l.rect);
    Some(StatusCover { rect: l.rect, under })
}

/// For a refresh of `dirty` in app_flush: when the indicator is wanted and the refresh reaches it,
/// paints it and widens dirty to hold it whole, and where it was if that touches the refresh (a size that
/// changed, a text that got shorter; a corner it left is asked a refresh of its own).
pub fn status_cover(a: &mut App, dirty: &mut Rect) -> Option<StatusCover> {
    if !wanted(a) {
        return None;
    }
    // (every fast ink frame comes here: most are nowhere near the corner)
    let old = with(|s| s.drawn);
    if !rect_overlaps(*dirty, band(a)) && !rect_overlaps(*dirty, old) {
        return None;
    }
    let l = layout(a);
    if !rect_overlaps(*dirty, l.rect) {
        return None;
    }
    *dirty = rect_union(*dirty, l.rect);
    if !rect_is_empty(old) && old != l.rect {
        if rect_overlaps(*dirty, old) {
            *dirty = rect_union(*dirty, old);
        } else {
            ask(old);
        }
    }
    paint(a, &l)
}

/// Puts back what the cover painted over, after the refresh.
pub fn status_uncover(a: &mut App, mut c: StatusCover) {
    canvas_blit(&mut a.screen, &c.under, Rect { x: 0, y: 0, w: c.rect.w, h: c.rect.h }, c.rect.x, c.rect.y);
    canvas_free(&mut c.under);
}

/// A refresh of `r` was sent: it was the one asked for if it held that, and an indicator that is not
/// wanted is gone from the panel if it held where it was.
pub fn status_presented(a: &App, r: Rect) {
    let off = !wanted(a);
    let gone = with(|s| {
        s.pending.retain(|p| !rect_contains_rect(r, *p));
        off && !rect_is_empty(s.drawn) && rect_contains_rect(r, s.drawn)
    });
    if gone {
        set_drawn(None);
        live_layer(LIVE_LAYER_STATUS, &a.screen, RECT_EMPTY);
    }
}

/// Live view starts: the indicator's layer, when the panel shows it (painted into a.screen, copied,
/// put back; nothing is presented).
pub fn status_live(a: &mut App) {
    if !wanted(a) || rect_is_empty(with(|s| s.drawn)) {
        return;
    }
    let l = layout(a);
    if let Some(c) = paint_into(a, &l) {
        status_uncover(a, c);
    }
}

/// Joins r to the regions to send, where it touches one.
fn add_region(v: &mut Vec<Rect>, r: Rect) {
    if rect_is_empty(r) {
        return;
    }
    let mut r = r;
    let mut i = 0;
    while i < v.len() {
        if rect_overlaps(v[i], r) {
            r = rect_union(v.remove(i), r);
            i = 0;
        } else {
            i += 1;
        }
    }
    v.push(r);
}

/// A refresh of the corner alone, when one is asked for and the pen allows it: a small GL16 of the
/// indicator and of each place it covered, each its own. Then the setting that changed is saved, if one
/// is waiting: the refresh is on its way first.
pub fn status_flush(a: &mut App) {
    flush_corner(a);
    if with(|s| s.save_due) && !status_busy(a) {
        with(|s| s.save_due = false);
        settings_save(&a.settings);
    }
}

fn flush_corner(a: &mut App) {
    if with(|s| s.pending.is_empty()) || status_busy(a) {
        return;
    }
    let on = wanted(a);
    if !on && rect_is_empty(with(|s| s.drawn)) {
        with(|s| s.pending.clear()); // nothing of it is on the panel: nothing to take down
        return;
    }
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let pending = with(|s| std::mem::take(&mut s.pending));
    let mut regions: Vec<Rect> = Vec::new();
    for p in pending {
        add_region(&mut regions, rect_intersect(p, screen));
    }
    let l = on.then(|| layout(a));
    if let Some(l) = &l {
        if l.cells.is_empty() { add_region(&mut regions, rect_intersect(l.rect, screen)); }
    }
    let cover = l.as_ref().and_then(|l| paint(a, l));
    for r in &regions {
        display_present(&a.screen, *r, RefreshMode::Ui);
    }
    if let Some(c) = cover {
        status_uncover(a, c);
    }
    for r in &regions {
        status_presented(a, *r);
    }
}

// ---- for the runner ----

/// The runner's status.* keys: "shown" 1 while the panel holds the indicator, "text" and "rect"
/// ("x y w h") what it says and where (empty while it is not shown), "charging" 1 when it shows the
/// bolt, "icon" the battery art's rect and "fill" how many of its 12 columns are lit (empty while it is
/// not shown or has no battery), "zone" the corner's square a tap finds ("x y w h"; the indicator's box
/// is part of the corner too while it is up) and "front" 1 while the page is in front, the only place it
/// shows. The settings themselves are "settings.status_on", "settings.status_size" and
/// "settings.status_corner".
pub fn status_probe(a: &App, key: &str, out: &mut String, cap: usize) -> i32 {
    let (drawn, text, charging, icon, fill) = with(|s| (s.drawn, s.drawn_text.clone(), s.drawn_charging, s.drawn_icon, s.drawn_fill));
    let shown = !rect_is_empty(drawn);
    let rect_text = |out: &mut String, r: Rect, show: bool| {
        if show {
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h)
        } else {
            csnprintf!(out, cap, "%s", "")
        }
    };
    match key {
        "reserved" => csnprintf!(out, cap, "%d", status_reserved(a)),
        "live" => csnprintf!(out, cap, "%d", with(|s| s.bar_cells.iter().any(|c| c.icon == IconKind::Live)) as i32),
        "controller" => csnprintf!(out, cap, "%d", with(|s| s.bar_cells.iter().any(|c| c.icon == IconKind::Gamepad)) as i32),
        "shown" => csnprintf!(out, cap, "%d", shown as i32),
        "text" => csnprintf!(out, cap, "%s", if shown { text.as_str() } else { "" }),
        "rect" => rect_text(out, drawn, shown),
        "icon" => rect_text(out, icon, shown && !rect_is_empty(icon)),
        "fill" => {
            if shown && !rect_is_empty(icon) {
                csnprintf!(out, cap, "%d", fill)
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
        "zone" => rect_text(out, status_square(a), true),
        "front" => csnprintf!(out, cap, "%d", page_in_front(a) as i32),
        _ => return -1,
    };
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The page is in front (or not) for the tests that have no notebook.
    fn front(on: bool) {
        TEST_FRONT.with(|c| c.set(Some(on)));
    }

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
    fn a_tap_is_short_and_lands_and_lifts_in_the_zone() {
        let slop = mm_to_px(SLOP_MM, 300.0);
        assert_eq!(slop.round() as i32, 59);
        let mut t = Tap::default();
        t.down(true, 60.0, 60.0, 1000);
        assert!(t.up(62.0, 61.0, 1120, true, slop).is_ok());
        // the same finger counts once
        assert!(t.up(62.0, 61.0, 1130, true, slop).is_err());
        // not a hit (outside the zone, or a control there): not ours
        t.down(false, 60.0, 60.0, 1000);
        assert!(t.up(60.0, 60.0, 1100, true, slop).is_err());
        // a lift in the zone is a tap wherever it is: the path is not asked
        t.down(true, 100.0, 100.0, 1000);
        assert!(t.up(160.0, 160.0, 1100, true, slop).is_ok());
        // a lift outside the zone but near where it landed (a press at the edge rolls out) is one too
        t.down(true, 150.0, 150.0, 1000);
        assert!(t.up(175.0, 160.0, 1100, false, slop).is_ok());
        // outside the zone and far: a swipe out
        t.down(true, 100.0, 100.0, 1000);
        assert!(t.up(100.0, 100.0 + slop + 5.0, 1100, false, slop).is_err());
        // held too long
        t.down(true, 60.0, 60.0, 1000);
        assert!(t.up(60.0, 60.0, 1000 + TAP_MS + 1, true, slop).is_err());
        t.down(true, 60.0, 60.0, 1000);
        assert!(t.up(60.0, 60.0, 1000 + TAP_MS, true, slop).is_ok());
        // a second finger or the pen takes it back
        t.down(true, 60.0, 60.0, 1000);
        t.cancel();
        assert!(t.up(60.0, 60.0, 1100, true, slop).is_err());
        // the clock of the events wraps like now_ms does
        t.down(true, 60.0, 60.0, u32::MAX - 50);
        assert!(t.up(60.0, 60.0, 70, true, slop).is_ok());
        // no lockout: tap after tap, each its own
        for i in 0..20u32 {
            t.down(true, 60.0, 60.0, 5000 + i * 90);
            assert!(t.up(60.0, 60.0, 5000 + i * 90 + 40, true, slop).is_ok(), "tap {i}");
        }
    }

    #[test]
    fn the_corners_are_the_four_of_the_screen() {
        let (w, h, side) = (1860, 2480, 165);
        assert_eq!(corner_square(0, w, h, side), Rect { x: 0, y: 0, w: 165, h: 165 });
        assert_eq!(corner_square(1, w, h, side), Rect { x: 1695, y: 0, w: 165, h: 165 });
        assert_eq!(corner_square(2, w, h, side), Rect { x: 1695, y: 2315, w: 165, h: 165 });
        assert_eq!(corner_square(3, w, h, side), Rect { x: 0, y: 2315, w: 165, h: 165 });
        // a box keeps its margin from the two edges it is nearest
        assert_eq!(corner_box(0, w, h, 300, 50, 6), Rect { x: 6, y: 6, w: 300, h: 50 });
        assert_eq!(corner_box(1, w, h, 300, 50, 6), Rect { x: 1554, y: 6, w: 300, h: 50 });
        assert_eq!(corner_box(2, w, h, 300, 50, 6), Rect { x: 1554, y: 2424, w: 300, h: 50 });
        assert_eq!(corner_box(3, w, h, 300, 50, 6), Rect { x: 6, y: 2424, w: 300, h: 50 });
        // sideways, the screen's own corners
        assert_eq!(corner_square(2, 2480, 1860, side), Rect { x: 2315, y: 1695, w: 165, h: 165 });
        // never past a small screen
        assert_eq!(corner_square(0, 100, 80, 165), Rect { x: 0, y: 0, w: 80, h: 80 });
    }

    #[test]
    fn the_battery_lights_its_share_of_twelve_columns() {
        assert_eq!(battery_fill(-1), 0);
        assert_eq!(battery_fill(0), 0);
        // any charge at all lights one
        assert_eq!(battery_fill(1), 1);
        assert_eq!(battery_fill(4), 1);
        assert_eq!(battery_fill(50), 6);
        assert_eq!(battery_fill(87), 10);
        assert_eq!(battery_fill(99), 12);
        assert_eq!(battery_fill(100), 12);
        assert_eq!(battery_fill(250), 12);
        // never a column less for more charge
        let mut last = 0;
        for p in 0..=100 {
            let f = battery_fill(p);
            assert!(f >= last && (0..=FILL_COLS).contains(&f));
            last = f;
        }
    }

    // The black pixels of a unit square of the art: (unit column, unit row) of a canvas drawn at u px a unit.
    fn art(b: &Battery, u: i32) -> (Canvas, Rect) {
        let r = Rect { x: u, y: u, w: ICON_UW * u, h: ICON_UH * u };
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, r.w + 2 * u, r.h + 2 * u), 0);
        let (w, h) = (c.w, c.h);
        canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 255);
        draw_battery(&mut c, r, u, b);
        (c, r)
    }
    fn black(c: &Canvas, r: Rect, u: i32, cx: i32, cy: i32) -> bool {
        // (the middle of the unit: a unit is one colour)
        c.px[((r.y + cy * u + u / 2) * c.stride + r.x + cx * u + u / 2) as usize] < 128
    }

    #[test]
    fn the_battery_art_shows_the_charge_and_the_bolt() {
        for u in [3, 4, 6] {
            let empty = Battery { percent: 0, charging: false };
            let (c, r) = art(&empty, u);
            // the outline: notched corners, edges, the terminal; nothing inside
            assert!(!black(&c, r, u, 0, 0) && !black(&c, r, u, 15, 0) && !black(&c, r, u, 0, 8) && !black(&c, r, u, 15, 8));
            assert!(black(&c, r, u, 1, 0) && black(&c, r, u, 14, 8) && black(&c, r, u, 0, 4) && black(&c, r, u, 15, 4));
            assert!(black(&c, r, u, 16, 4) && black(&c, r, u, 17, 2) && black(&c, r, u, 17, 6));
            assert!(!black(&c, r, u, 16, 1) && !black(&c, r, u, 16, 7), "the terminal is smaller than the body");
            for cx in 2..14 {
                assert!(!black(&c, r, u, cx, 4), "u {u}: empty at column {cx}");
            }
            // the charge fills from the left, by battery_fill's share, in the five rows inside
            for p in [1, 25, 50, 87, 100] {
                let (c, r) = art(&Battery { percent: p, charging: false }, u);
                let lit = (2..14).filter(|&cx| black(&c, r, u, cx, 4)).count() as i32;
                assert_eq!(lit, battery_fill(p), "u {u}: {p}%");
                for cx in 2..2 + lit {
                    for cy in 2..7 {
                        assert!(black(&c, r, u, cx, cy), "u {u}: {p}% solid at {cx},{cy}");
                    }
                }
                assert!(!black(&c, r, u, 2, 1) && !black(&c, r, u, 2, 7), "the padding inside the outline stays white");
            }
            // charging: the bolt is there over an empty battery, black on white
            let (c, r) = art(&Battery { percent: 0, charging: true }, u);
            let dark = (0..ICON_UW).flat_map(|cx| (0..ICON_UH).map(move |cy| (cx, cy))).filter(|&(cx, cy)| black(&c, r, u, cx, cy)).count();
            let plain = art(&empty, u);
            let dark0 = (0..ICON_UW).flat_map(|cx| (0..ICON_UH).map(move |cy| (cx, cy))).filter(|&(cx, cy)| black(&plain.0, plain.1, u, cx, cy)).count();
            let bolt: usize = BOLT.iter().map(|row| row.iter().filter(|&&ch| ch == b'X').count()).sum();
            assert_eq!(dark, dark0 + bolt, "u {u}: the bolt's units");
            // over a full battery it has a halo, so it reads: white units beside the bolt in the black
            let (c, r) = art(&Battery { percent: 100, charging: true }, u);
            assert!(black(&c, r, u, 8, 4), "the bolt's crossbar is black");
            assert!(!black(&c, r, u, 5, 4) || !black(&c, r, u, 11, 4), "a white halo breaks the fill beside the bolt");
            assert!(black(&c, r, u, 3, 3) && black(&c, r, u, 13, 5), "the charge away from the bolt is whole");
        }
    }

    #[test]
    fn the_box_grows_with_the_size_and_holds_its_text_and_its_battery() {
        utc();
        up();
        let b = Battery { percent: 87, charging: false };
        let t = 1700000000;
        for corner in 0..STATUS_CORNERS {
            let (s0, s1, s2) = (layout_of(0, corner, 1860, 2480, t, &b), layout_of(1, corner, 1860, 2480, t, &b), layout_of(2, corner, 1860, 2480, t, &b));
            assert!(s0.rect.w < s1.rect.w && s1.rect.w < s2.rect.w);
            assert!(s0.rect.h < s1.rect.h && s1.rect.h < s2.rect.h);
            assert!(s0.unit < s1.unit && s1.unit < s2.unit, "the art scales with the size");
            for l in [&s0, &s1, &s2] {
                // in its corner, inside the screen, the time, the battery and the percent inside the box, in that order
                let screen = Rect { x: 0, y: 0, w: 1860, h: 2480 };
                assert!(rect_contains_rect(screen, l.rect) && l.rect.w < 1860 / 2);
                let near_top = corner < 2;
                assert!(if near_top { l.rect.y == 6 } else { l.rect.y + l.rect.h == 2480 - 6 }, "corner {corner}");
                let near_left = corner == 0 || corner == 3;
                assert!(if near_left { l.rect.x == 6 } else { l.rect.x + l.rect.w == 1860 - 6 }, "corner {corner}");
                let icon = l.icon.expect("a battery");
                assert!(rect_contains_rect(l.rect, l.time_r) && rect_contains_rect(l.rect, icon) && rect_contains_rect(l.rect, l.pct_r));
                assert!(l.time_r.x + l.time_r.w <= icon.x && icon.x + icon.w <= l.pct_r.x, "time, then the battery, then its percent");
                assert_eq!(icon.w, ICON_UW * l.unit);
                assert_eq!(icon.h, ICON_UH * l.unit);
                // the art is centred in the box's height (to a pixel)
                assert!((icon.y - l.rect.y - (l.rect.y + l.rect.h - (icon.y + icon.h))).abs() <= 1);
                assert_eq!(l.text, "Nov 14, 22:13  87%");
                assert_eq!((l.time.as_str(), l.pct.as_str()), ("Nov 14, 22:13", "87%"));
            }
        }
        // charging does not move or widen anything: the bolt is inside the art
        let c = layout_of(1, 0, 1860, 2480, t, &Battery { percent: 87, charging: true });
        let n = layout_of(1, 0, 1860, 2480, t, &b);
        assert_eq!(c.rect, n.rect);
        assert_eq!(c.icon, n.icon);
        // one box a minute, however the minute reads
        assert_eq!(layout_of(1, 0, 1860, 2480, t, &b).key, layout_of(1, 0, 1860, 2480, t + 10, &b).key);
        assert_ne!(layout_of(1, 0, 1860, 2480, t, &b).key, layout_of(1, 0, 1860, 2480, t + 60, &b).key);
        // without a percent the time alone, no battery drawn
        let none = layout_of(1, 0, 1860, 2480, t, &Battery::default());
        assert!(none.icon.is_none() && none.pct.is_empty() && none.rect.w < n.rect.w);
        assert_eq!(none.text, "Nov 14, 22:13");
        // the box is as tall as the art and its margins, so the art never touches the frame
        for size in 0..STATUS_SIZES {
            let l = layout_of(size, 0, 1860, 2480, t, &b);
            let icon = l.icon.unwrap();
            assert!(icon.y - l.rect.y >= ui_px(3.0) && l.rect.h >= l.unit * ICON_UH + 2 * ui_px(3.0));
        }
    }

    #[test]
    fn the_band_holds_the_box_of_each_corner() {
        utc();
        up();
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        a.settings.status_mode = 1;
        let b = Battery { percent: 87, charging: true };
        for size in 0..STATUS_SIZES {
            for corner in 0..STATUS_CORNERS {
                a.settings.status_size = size;
                a.settings.status_corner = corner;
                let l = layout_of(size, corner, a.w, a.h, 1700000000, &b);
                assert!(rect_contains_rect(band(&a), l.rect), "size {size} corner {corner}: {:?} in {:?}", l.rect, band(&a));
                assert_eq!(band(&a).h, ui_px(6.0) + l.rect.h);
            }
        }
    }

    #[test]
    fn the_zone_is_fourteen_millimetres_in_the_corner_and_takes_the_indicator() {
        utc();
        up();
        status_reset();
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        a.settings.status_mode = 1;
        assert_eq!(status_square(&a), Rect { x: 0, y: 0, w: 165, h: 165 });
        a.dpi = 150.0;
        assert_eq!(status_square(&a).w, 83);
        a.dpi = 300.0;
        // it follows the corner
        a.settings.status_corner = 2;
        assert_eq!(status_square(&a), Rect { x: 1695, y: 2315, w: 165, h: 165 });
        assert!(status_hit(&a, 1850.0, 2470.0) && !status_hit(&a, 70.0, 70.0) && !status_hit(&a, 1200.0, 2470.0));
        // off, where its box would show (twice as wide as the square) and 4 mm around it, a tap finds it
        let w = layout(&a).rect;
        assert!(rect_is_empty(with(|s| s.drawn)) && w.w > 165);
        assert!(status_hit(&a, (w.x + 10) as f32, (w.y + w.h / 2) as f32) && status_hit(&a, (w.x - 40) as f32, (w.y + 5) as f32));
        assert!(!status_hit(&a, (w.x - 60) as f32, (w.y + 5) as f32) && !status_hit(&a, (w.x + 10) as f32, (w.y - 60) as f32));
        // while the indicator is up, its box is part of the corner: the box and the square, not the screen between
        a.settings.status_corner = 0;
        let l = layout_of(2, 0, a.w, a.h, 1700000000, &Battery { percent: 87, charging: true });
        set_drawn(Some(&l));
        assert!(status_hit(&a, 70.0, 70.0) && status_hit(&a, (l.rect.x + l.rect.w - 5) as f32, 30.0));
        assert!(!status_hit(&a, (l.rect.x + l.rect.w + 20) as f32, 30.0) && !status_hit(&a, 400.0, 400.0));
        // the corner moved, the old box still on the panel until its refresh: both places take the tap, nothing between
        a.settings.status_corner = 2;
        assert!(status_hit(&a, 1850.0, 2470.0) && status_hit(&a, 100.0, 30.0));
        assert!(!status_hit(&a, 900.0, 1200.0) && !status_hit(&a, 1700.0, 100.0));
        status_reset();
        assert!(!status_hit(&a, 100.0, 30.0));
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
        for k in ["text", "rect", "icon", "fill", "charging"] {
            status_probe(&a, k, &mut out, 64);
            assert_eq!(out, "", "{k}");
        }
        status_probe(&a, "zone", &mut out, 64);
        assert_eq!(out, "0 0 165 165");
        status_probe(&a, "front", &mut out, 64);
        assert_eq!(out, "0", "no notebook page in front");
        let l = layout_of(0, 0, 1860, 2480, 1700000000, &Battery { percent: 9, charging: true });
        set_drawn(Some(&l));
        status_probe(&a, "shown", &mut out, 64);
        assert_eq!(out, "1");
        status_probe(&a, "text", &mut out, 64);
        assert_eq!(out, "Nov 14, 22:13  9%");
        status_probe(&a, "charging", &mut out, 64);
        assert_eq!(out, "1");
        status_probe(&a, "rect", &mut out, 64);
        assert_eq!(out, format!("{} {} {} {}", l.rect.x, l.rect.y, l.rect.w, l.rect.h));
        let ir = l.icon.unwrap();
        status_probe(&a, "icon", &mut out, 64);
        assert_eq!(out, format!("{} {} {} {}", ir.x, ir.y, ir.w, ir.h));
        status_probe(&a, "fill", &mut out, 64);
        assert_eq!(out, "1");
        // no battery: no icon, no fill
        let l = layout_of(0, 0, 1860, 2480, 1700000000, &Battery::default());
        set_drawn(Some(&l));
        status_probe(&a, "icon", &mut out, 64);
        assert_eq!(out, "");
        status_probe(&a, "fill", &mut out, 64);
        assert_eq!(out, "");
        assert_eq!(status_probe(&a, "nope", &mut out, 64), -1);
        status_reset();
    }

    #[test]
    fn the_clock_is_in_the_font() {
        up();
        // The glyph the Display sheet's rows draw must exist in the embedded face, or they draw nothing.
        for cp in [0xF0150u32] {
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

    #[test]
    fn refreshes_asked_for_join_where_they_touch_and_stay_apart_where_they_do_not() {
        status_reset();
        let near = |r: Rect| with(|s| s.pending.contains(&r));
        let (a, b, c) = (Rect { x: 6, y: 6, w: 300, h: 50 }, Rect { x: 100, y: 20, w: 400, h: 60 }, Rect { x: 1554, y: 2424, w: 300, h: 50 });
        ask(a);
        ask(RECT_EMPTY);
        assert_eq!(with(|s| s.pending.len()), 1);
        // a corner's two boxes are two refreshes, never the screen between
        ask(c);
        assert_eq!(with(|s| s.pending.len()), 2);
        assert!(near(a) && near(c));
        // one that touches a joins it
        ask(b);
        assert_eq!(with(|s| s.pending.len()), 2);
        assert!(near(rect_union(a, b)) && near(c));
        // a presented refresh takes what it held
        let reached = rect_union(a, b);
        with(|s| s.pending.retain(|p| !rect_contains_rect(reached, *p)));
        assert_eq!(with(|s| s.pending.clone()), vec![c]);
        // regions to send join the same way
        let mut v = Vec::new();
        add_region(&mut v, a);
        add_region(&mut v, c);
        add_region(&mut v, b);
        assert_eq!(v.len(), 2);
        assert!(v.contains(&rect_union(a, b)) && v.contains(&c));
        status_reset();
    }

    // (The host's panel is none, so a refresh is sent to nowhere here; the runner's panel is another
    // thread's state in a test, and the scenario y_status_corner.qs covers that build.)
    #[cfg(not(any(feature = "sim", feature = "kindle")))]
    #[test]
    fn a_refresh_waits_for_the_pen_and_goes_once() {
        utc();
        up();
        status_reset();
        front(true);
        let mut a = App { w: 600, h: 400, dpi: 300.0, ..Default::default() };
        a.settings.status_mode = 1;
        assert_eq!(canvas_init(&mut a.screen, 600, 400), 0);
        canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w: 600, h: 400 }, 200);
        let before = a.screen.px.clone();
        a.settings.status_on = true;
        let r = Rect { x: 6, y: 6, w: 300, h: 50 };
        ask(r);
        status_pen(&InEvent { type_: InType::PenHover, ..Default::default() });
        status_flush(&mut a);
        assert_eq!(with(|s| s.pending.clone()), vec![r], "kept while the pen is near");
        assert!(rect_is_empty(with(|s| s.drawn)));
        status_pen(&InEvent { type_: InType::PenLeave, ..Default::default() });
        status_flush(&mut a);
        assert!(with(|s| s.pending.is_empty()), "sent once the pen is gone");
        assert!(!rect_is_empty(with(|s| s.drawn)));
        assert_eq!(a.screen.px, before, "and the screen holds none of it");
        // a corner moved: two small refreshes, the new place and the old, and none across the screen
        a.settings.status_corner = 2;
        ask_change(&a);
        assert_eq!(with(|s| s.pending.len()), 2);
        let old = with(|s| s.drawn);
        let new = layout(&a).rect;
        assert!(with(|s| s.pending.contains(&old) && s.pending.contains(&new)) && !rect_overlaps(old, new));
        status_flush(&mut a);
        assert!(with(|s| s.pending.is_empty()) && with(|s| s.drawn) == new, "the new place is what is on the panel now");
        // off, what it covered goes with one more, and then nothing is asked for
        a.settings.status_on = false;
        ask_change(&a);
        status_flush(&mut a);
        assert!(with(|s| s.pending.is_empty()) && rect_is_empty(with(|s| s.drawn)));
        status_flush(&mut a);
        front(false);
        status_reset();
    }

    #[test]
    fn the_painted_indicator_is_put_back() {
        utc();
        up();
        status_reset();
        front(true);
        let mut a = App { w: 600, h: 400, dpi: 300.0, ..Default::default() };
        a.settings.status_mode = 1;
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
        // it is on the panel from here, with its text, its battery and its bolt
        let mut out = String::new();
        status_probe(&a, "shown", &mut out, 8);
        assert_eq!(out, "1");
        status_probe(&a, "charging", &mut out, 8);
        assert_eq!(out, "1");
        status_probe(&a, "fill", &mut out, 8);
        assert_eq!(out, battery_fill(42).to_string());
        status_uncover(&mut a, cover);
        assert_eq!(a.screen.px, before);
        // a refresh elsewhere does not touch it
        let mut far = Rect { x: 300, y: 300, w: 50, h: 50 };
        assert!(status_cover(&mut a, &mut far).is_none() && far == Rect { x: 300, y: 300, w: 50, h: 50 });
        // in a bottom corner it is painted there, and the top's old place goes with its own refresh when
        // the refresh does not reach it
        a.settings.status_corner = 3;
        let old = with(|s| s.drawn);
        let mut low = Rect { x: 0, y: 300, w: 600, h: 100 };
        let c = status_cover(&mut a, &mut low).expect("covered in the bottom corner");
        assert!(rect_contains_rect(low, c.rect) && c.rect.y > 300 && c.rect.x < 100);
        assert!(with(|s| s.pending.contains(&old)), "the old place is asked a refresh of its own");
        status_uncover(&mut a, c);
        // off, or the page not in front: it is not painted at all
        a.settings.status_on = false;
        let mut dirty = Rect { x: 0, y: 0, w: 600, h: 400 };
        assert!(status_cover(&mut a, &mut dirty).is_none());
        status_presented(&a, dirty);
        status_probe(&a, "shown", &mut out, 8);
        assert_eq!(out, "0");
        a.settings.status_on = true;
        front(false);
        assert!(status_cover(&mut a, &mut dirty).is_none());
        front(true);
        status_reset();
        front(false);
    }

    #[test]
    fn a_sheet_over_the_page_takes_the_indicator_down_where_it_is() {
        utc();
        up();
        status_reset();
        front(true);
        let mut a = App { w: 600, h: 400, dpi: 300.0, ..Default::default() };
        a.settings.status_mode = 1;
        assert_eq!(canvas_init(&mut a.screen, 600, 400), 0);
        a.settings.status_on = true;
        let l = layout_of(0, 0, 600, 400, 1700000000, &Battery { percent: 80, charging: false });
        set_drawn(Some(&l));
        // the page goes out of front (a sheet opened): the tick asks for the indicator's own rect, and a
        // refresh that holds it takes it off the panel
        front(false);
        status_tick(&mut a, 1000);
        assert_eq!(with(|s| s.pending.clone()), vec![l.rect]);
        status_presented(&a, rect_inflate(l.rect, 4));
        assert!(rect_is_empty(with(|s| s.drawn)) && with(|s| s.pending.is_empty()));
        // and nothing more is asked while there is nothing on the panel
        status_tick(&mut a, 1000);
        assert!(with(|s| s.pending.is_empty()));
        status_reset();
    }
}
