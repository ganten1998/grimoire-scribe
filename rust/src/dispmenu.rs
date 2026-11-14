//! dispmenu.h: Settings' Display and Sleep & Clock sheets (front light and appearance; sleep
//! screen, timer and clock). A modal hosted by the editor and the library; it draws itself and
//! handles pointer input.

// dispmenu.c: the Display sheet: front light, invert, and sleep (screen + timeout).

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DmResult {
    #[default]
    None = 0,  // DM_NONE
    Changed = 1,  // DM_CHANGED
    Closed = 2,  // DM_CLOSED
    Inverted = 3,  // DM_INVERTED
    /// Sideways, show the whole page was flipped (settings.landscape_fit): the host lays the page out again
    Refit = 4,
}

/// Which of Settings' two sheets it is: Display's front light and appearance, or Sleep & Clock's
/// sleep screen, timer and clock. The editor hosts the Display one alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DmPage {
    #[default]
    Display,
    SleepClock,
}

#[derive(Clone, Debug, Default)]
pub struct DispMenu {
    pub page: DmPage,
    pub open: bool,
    pub rect: Rect,
    pub hits: HitList,
    /// 0..LIGHT_MAX, -1 unknown
    pub intensity: i32,
    pub warmth: i32,
    /// slider being dragged (its D_* id), 0 none
    pub active: i32,
    /// that slider's row
    pub active_row: Rect,
    /// its value changed since its row was last drawn
    pub row_stale: bool,
    /// when its row was last drawn
    pub last_draw: u32,
    pub last_apply: u32,
    pub press: Press,
    /// the button pressed, shown pressed until the lift (app_press_feedback)
    pub pressed: Option<Rect>,
}

const D_BRIGHT: i32 = 1;
const D_WARM: i32 = 2;
const D_INVERT: i32 = 3;
const D_SLEEP_MODE: i32 = 4;
const D_PIN: i32 = 5;
const D_SLEEP_AFTER: i32 = 6;
const D_DONE: i32 = 7;
const D_STATUS: i32 = 8;
const D_FIT: i32 = 9;
const D_CORNER: i32 = 10;

pub fn dispmenu_control_point(m:&DispMenu,name:&str)->Option<(f32,f32)> {
    if !m.open{return None;}
    let id=match name {"size"=>D_STATUS,"corner"=>D_CORNER,"done"=>D_DONE,"back"=>D_BACK,"fit"=>D_FIT,"newpage"=>D_NEWPAGE,"time12"=>D_TIME12,"invert"=>D_INVERT,"brightness"=>D_BRIGHT,"warmth"=>D_WARM,_=>return None};
    m.hits.h.iter().find(|h|h.id==id).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32))
}
const D_NEWPAGE: i32 = 11;
const D_TIME12: i32 = 12;
const D_BACK: i32 = 13;

/// A dragged slider's row is drawn at most this often, as Pen feel's sliders are.
const DRAG_DRAW_MS: u32 = 80;

/// "Sleep after" choices (minutes, 0 = never); the slider moves along this list.
const K_SLEEP_STEPS: [i32; 8] = [0, 1, 2, 5, 10, 15, 30, 60];
const N_SLEEP_STEPS: i32 = K_SLEEP_STEPS.len() as i32;

/// The sleep screen row, one sentence per choice (settings.ini sleep_mode 0, 1, 2).
static K_MODE_ROWS: [&str; 4] = [
    // TRANSLATORS: the Sleep & Clock sheet's sleep screen row, which a tap moves to the next choice:
    // what the Kindle shows while it sleeps. About 1280 px at 44 px, upright and sideways.
    n_("Sleep screen: the page I'm on (tap to change)"),
    n_("Sleep screen: a pinned page (tap to change)"),
    n_("Sleep screen: the Kindle's own screensaver (tap to change)"),
    n_("Sleep screen: KOReader wallpaper (tap to change)"),
];

/// The clock and battery's size (settings.ini status_size 0, 1, 2) and corner (status_corner 0, 1, 2, 3):
/// two rows on Sleep & Clock's page, each a tap that moves to the next choice, one sentence for each
/// choice so a language can put its words in order. Each has room for an icon and two lines of text at
/// 34 px.
static K_STATUS_ROWS: [&str; 3] = [
    // TRANSLATORS: the Sleep & Clock sheet's row for how large the clock and battery on a notebook page
    // are, which a tap moves to the next size. Two lines of about 365 px at 34 px.
    n_("Clock size: small"),
    n_("Clock size: medium"),
    n_("Clock size: large"),
];
static K_CORNER_ROWS: [&str; 4] = [
    // TRANSLATORS: the Sleep & Clock sheet's row under the clock's size: which corner of a notebook page
    // the clock and battery sit in, which a tap moves to the next (clockwise). Two lines of about 365 px
    // at 34 px.
    n_("Clock corner: top left"),
    n_("Clock corner: top right"),
    n_("Clock corner: bottom right"),
    n_("Clock corner: bottom left"),
];

/// The corner's pictogram in r: the page as an outline, a little taller than wide, with a solid square in the
/// corner the clock sits in.
fn draw_corner_icon(c: &mut Canvas, r: Rect, corner: i32) {
    let (w, h) = (r.w * 3 / 4, r.h);
    let page = Rect { x: r.x + (r.w - w) / 2, y: r.y, w, h };
    let t = ui_px(4.0);
    canvas_rect_outline(c, page, t, 0);
    let d = w * 2 / 5;
    let pad = t + ui_px(5.0);
    let right = corner == 1 || corner == 2;
    let bottom = corner == 2 || corner == 3;
    let dot = Rect { x: if right { page.x + w - pad - d } else { page.x + pad }, y: if bottom { page.y + h - pad - d } else { page.y + pad }, w: d, h: d * 2 / 3 };
    canvas_fill(c, dot, 0);
}

fn step_of(minutes: i32) -> i32 {
    let mut best = 0usize;
    for (i, step) in K_SLEEP_STEPS.iter().enumerate() {
        if (step - minutes).abs() < (K_SLEEP_STEPS[best] - minutes).abs() {
            best = i;
        }
    }
    best as i32
}

pub fn dispmenu_open(m: &mut DispMenu, a: &mut App) {
    *m = DispMenu { open: true, ..Default::default() };
    light_get(&mut m.intensity, &mut m.warmth);
    m.rect = Rect::new(0, 0, a.w, a.h);
}

/// Settings > Sleep & Clock: the same sheet, on its sleep and clock page.
pub fn dispmenu_open_sleep(m: &mut DispMenu, a: &mut App) {
    dispmenu_open(m, a);
    m.page = DmPage::SleepClock;
}

fn slider_for(m: &DispMenu, a: &App, which: i32, row: Rect, s: &mut Slider) {
    *s = Slider { step: 1.0, unit: "", ..Default::default() };
    if which == D_SLEEP_AFTER {
        s.min = 0.0;
        s.max = (N_SLEEP_STEPS - 1) as f32;
        // TRANSLATORS: the sheets' sliders, each left of its value ("12", "5 min", "never"):
        // about 440 px at 44 px beside a time in Gothic (460 in Classic), 520 beside a number.
        s.label = tr("Sleep after");
        s.value = step_of(a.settings.sleep_minutes) as f32;
    } else {
        s.min = 0.0;
        s.max = LIGHT_MAX as f32;
        s.label = if which == D_BRIGHT { tr("Brightness") } else { tr("Warmth") };
        s.value = (if which == D_BRIGHT { m.intensity } else { m.warmth }) as f32;
        if s.value < 0.0 {
            s.value = 0.0;
        }
    }
    ui_slider_layout(s, row);
}

/// A slider's row, all of it (ui_slider_draw fills the row first): what dispmenu_draw draws there,
/// and what a drag redraws alone.
fn draw_slider_row(m: &DispMenu, a: &mut App, which: i32, row: Rect) {
    let mut s = Slider::default();
    slider_for(m, a, which, row, &mut s);
    let mut vt = String::new();
    if which == D_SLEEP_AFTER {
        let mins = K_SLEEP_STEPS[clampi(s.value as i32, 0, N_SLEEP_STEPS - 1) as usize];
        // TRANSLATORS: the Sleep after slider's value: minutes, or never.
        if mins != 0 { csnprintf!(&mut vt, 24, tr("%d min"), mins); }
        else { csnprintf!(&mut vt, 24, "%s", c_("sleep after", "never")); }
    } else {
        csnprintf!(&mut vt, 24, "%d", s.value as i32);
    }
    ui_slider_draw(&mut a.screen, &s, Some(&vt));
}

/// One row of either page: a slider, a checkbox, the sleep screen's choice, Pin this page, or one
/// of the clock's two cells.
enum DRowKind {
    Slider(i32),
    Check { id: i32, label: String, on: bool },
    Note(String),
    SleepMode { label: String, can_tap: bool },
    Pin,
    Status { id: i32, label: String, corner: i32 },
}

fn draw_drow(m: &mut DispMenu, a: &mut App, kind: &DRowKind, outer: Rect) {
    let inner = rect_inflate(outer, -ui_px(10.0));
    match kind {
        DRowKind::Slider(id) => {
            draw_slider_row(m, a, *id, inner);
            hits_add(&mut m.hits, inner, *id, 0);
        }
        DRowKind::Check { id, label, on } => {
            ui_checkbox(&mut a.screen, inner, label, *on);
            hits_add(&mut m.hits, inner, *id, 0);
        }
        DRowKind::Note(text) => {
            ui_text(&mut a.screen, FontFace::Regular, ui_text_size(), ui_small_size(), inner, text, 2, false, 0);
        }
        DRowKind::SleepMode { label, can_tap } => {
            let isz = inner.h * 3 / 4;
            ui_icon(&mut a.screen, IconKind::Notebook, Rect::new(inner.x, inner.y + (inner.h - isz) / 2, isz, isz), 0);
            ui_text(&mut a.screen, FontFace::Regular, ui_text_size(), ui_small_size(), Rect::new(inner.x + isz + ui_px(20.0), inner.y, inner.w - isz - ui_px(20.0), inner.h), label, 2, false, 0);
            if *can_tap {
                hits_add(&mut m.hits, inner, D_SLEEP_MODE, 0);
            }
        }
        DRowKind::Pin => {
            // TRANSLATORS: makes the open page the sleep screen (528 px at 44 px, or 34 px).
            let isz = inner.h * 3 / 4;
            ui_icon(
                &mut a.screen,
                IconKind::Pin,
                Rect::new(inner.x, inner.y + (inner.h - isz) / 2, isz, isz),
                0,
            );
            ui_text(
                &mut a.screen,
                FontFace::Regular,
                ui_text_size(),
                ui_small_size(),
                Rect::new(inner.x + isz + ui_px(20.0), inner.y, inner.w - isz - ui_px(20.0), inner.h),
                tr("Pin this page"),
                2,
                false,
                0,
            );
            hits_add(&mut m.hits, inner, D_PIN, 0);
        }
        DRowKind::Status { id, label, corner } => {
            let isz = inner.h * 3 / 4;
            let icon = Rect::new(inner.x, inner.y + (inner.h - isz) / 2, isz, isz);
            if *id == D_CORNER {
                draw_corner_icon(&mut a.screen, icon, *corner);
            } else {
                ui_icon(&mut a.screen, IconKind::Clock, icon, 0);
            }
            ui_text(&mut a.screen, FontFace::Regular, ui_small_size(), ui_small_size(), Rect::new(inner.x + isz + ui_px(20.0), inner.y, inner.w - isz - ui_px(20.0), inner.h), label, 2, false, 0);
            hits_add(&mut m.hits, inner, *id, 0);
        }
    }
}

/// Draws into a->screen; returns the rect covered. Either page is a full screen, as Settings'
/// submenus are: Back and the title above a rule, the rows below (one column, or two when the screen
/// is short) and X at the top right.
pub fn dispmenu_draw(m: &mut DispMenu, a: &mut App) -> Rect {
    hits_clear(&mut m.hits);
    m.pressed = None;
    let (aw, ah) = (a.w, a.h);
    m.rect = Rect::new(0, 0, aw, ah);
    // TRANSLATORS: the title of Settings' Sleep & Clock sheet: the sleep screen, the timer and the
    // clock and battery on the page.
    let title = match m.page {
        DmPage::Display => tr("Display"),
        DmPage::SleepClock => tr("Sleep & Clock"),
    };
    let page = crate::library::settings_page(&mut a.screen, aw, ah, title, true, false);
    hits_add(&mut m.hits, page.back, D_BACK, 0);
    hits_add(&mut m.hits, page.close, D_DONE, 0);
    let in_ = page.content;
    let mut rows: Vec<DRowKind> = Vec::new();
    match m.page {
        DmPage::Display => {
            let have = light_available() && m.intensity >= 0;
            if !have {
                // TRANSLATORS: in the Display page where the sliders would be (1420 px at 44 px).
                rows.push(DRowKind::Note(tr("Front light controls are not available here.").to_string()));
            } else {
                rows.push(DRowKind::Slider(D_BRIGHT));
                if m.warmth < 0 {
                    rows.push(DRowKind::Note(tr("This Kindle has no warmth control.").to_string()));
                } else {
                    rows.push(DRowKind::Slider(D_WARM));
                }
            }
            rows.push(DRowKind::Check {
                id: D_INVERT,
                label: tr("Invert colors in Grimoire (white on black)").to_string(),
                on: a.settings.invert,
            });
            rows.push(DRowKind::Check {
                id: D_FIT,
                label: tr("Sideways, show the whole page").to_string(),
                on: a.settings.landscape_fit,
            });
            rows.push(DRowKind::Check {
                id: D_NEWPAGE,
                label: tr("Swiping past the last page adds a page").to_string(),
                on: a.settings.gesture_new_page,
            });
        }
        DmPage::SleepClock => {
            let label = if a.native_pin {
                tr("Sleep screen: Kindle PIN (native unlock)")
            } else {
                tr(K_MODE_ROWS[clampi(a.settings.sleep_mode, 0, 3) as usize])
            };
            rows.push(DRowKind::SleepMode { label: label.to_string(), can_tap: !a.native_pin });
            if editor_page_canvas(a).is_some() {
                rows.push(DRowKind::Pin);
            }
            rows.push(DRowKind::Slider(D_SLEEP_AFTER));
            rows.push(DRowKind::Check {
                id: D_TIME12,
                label: tr("Display time as 12 hour").to_string(),
                on: a.settings.clock_12h,
            });
            rows.push(DRowKind::Status {
                id: D_STATUS,
                label: tr(K_STATUS_ROWS[clampi(a.settings.status_size, 0, STATUS_SIZES - 1) as usize]).to_string(),
                corner: 0,
            });
            rows.push(DRowKind::Status {
                id: D_CORNER,
                label: tr(K_CORNER_ROWS[clampi(a.settings.status_corner, 0, STATUS_CORNERS - 1) as usize]).to_string(),
                corner: a.settings.status_corner,
            });
        }
    }
    let n = rows.len() as i32;
    let gap = ui_px(14.0);
    let cols = if in_.h / n.max(1) < ui_px(88.0) { 2 } else { 1 };
    let per = (n + cols - 1) / cols;
    let rh = ((in_.h - (per - 1) * gap) / per.max(1)).clamp(ui_px(60.0), ui_px(140.0));
    let cw = (in_.w - (cols - 1) * gap) / cols;
    for (k, kind) in rows.iter().enumerate() {
        let col = k as i32 / per;
        let ri = k as i32 % per;
        let r = Rect::new(in_.x + col * (cw + gap), in_.y + ri * (rh + gap), cw, rh);
        draw_drow(m, a, kind, r);
    }
    m.rect
}

/// The touch ended with the recognizer's cancel (the pen came near, a second finger or a palm): a
/// slider being dragged keeps the value it has now, applied and saved as its release would, never
/// one read from the cancel's off-screen place; a button pressed is let go, doing nothing.
pub fn dispmenu_cancel(m: &mut DispMenu, a: &mut App, dirty: &mut Rect) -> DmResult {
    *dirty = RECT_EMPTY;
    if !m.open || m.active == 0 {
        m.active = 0;
        *dirty = app_press_release(a, &mut m.pressed);
        return DmResult::None;
    }
    match m.active {
        D_SLEEP_AFTER => {
            settings_save(&a.settings);
            app_apply_sleep_policy(a);
        }
        D_BRIGHT => light_set_intensity(m.intensity),
        _ => light_set_warmth(m.warmth),
    }
    dispmenu_draw(m, a);
    *dirty = m.rect;
    m.active = 0;
    m.row_stale = false;
    DmResult::None
}

/// Pointer phase 0 down / 1 move / 2 up (a cancelled touch goes to dispmenu_cancel). DM_CLOSED: the host should recompose m->rect.
/// DM_INVERTED: colors flipped, the host should repaint everything (full refresh).
pub fn dispmenu_pointer(m: &mut DispMenu, a: &mut App, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> DmResult {
    let ph = phase;
    *dirty = RECT_EMPTY;
    if !m.open {
        return DmResult::None;
    }
    if ph == 0 {
        press_begin(&mut m.press, x, y, t);
        if !rect_contains(m.rect, x as i32, y as i32) {
            return DmResult::None;
        }
        let h = hits_find(&m.hits, x, y).copied();
        m.active = match h {
            Some(h) if h.id == D_BRIGHT || h.id == D_WARM || h.id == D_SLEEP_AFTER => h.id,
            _ => 0,
        };
        if m.active != 0 {
            if let Some(h) = h {
                m.active_row = h.r;
            }
        }
        if m.active == 0 {
            if let Some(h) = h {
                app_press_feedback(a, h.r);
                m.pressed = Some(h.r);
            }
            return DmResult::None;
        }
    }
    if m.active != 0 {
        let mut s = Slider::default();
        slider_for(m, a, m.active, m.active_row, &mut s);
        ui_slider_set_from_x(&mut s, x);
        let changed;
        if m.active == D_SLEEP_AFTER {
            let mins = K_SLEEP_STEPS[clampi(s.value as i32, 0, N_SLEEP_STEPS - 1) as usize];
            changed = mins != a.settings.sleep_minutes;
            a.settings.sleep_minutes = mins;
            if ph == 2 {
                settings_save(&a.settings);
                app_apply_sleep_policy(a);
            }
        } else {
            let v = if m.active == D_BRIGHT { &mut m.intensity } else { &mut m.warmth };
            changed = s.value as i32 != *v;
            *v = s.value as i32;
            // The light reacts live, but do not flood the power service while dragging.
            if ph == 2 || (changed && t.wrapping_sub(m.last_apply) > 200) {
                if m.active == D_BRIGHT {
                    light_set_intensity(m.intensity);
                } else {
                    light_set_warmth(m.warmth);
                }
                m.last_apply = t;
            }
        }
        // While it is dragged only its row is drawn, at most every DRAG_DRAW_MS: each step used to
        // redraw the whole card and copy its 1.8 Mpx to the panel (90-100 ms on the Kindle, and
        // the raw digitizer under the hovering pen overflowed at 23 of them). The whole card at the
        // release.
        m.row_stale |= changed;
        if ph == 2 {
            dispmenu_draw(m, a);
            *dirty = m.rect;
            m.active = 0;
            m.row_stale = false;
        } else if m.row_stale && (ph == 0 || t.wrapping_sub(m.last_draw) >= DRAG_DRAW_MS) {
            draw_slider_row(m, a, m.active, m.active_row);
            *dirty = m.active_row;
            m.last_draw = t;
            m.row_stale = false;
        }
        return if changed { DmResult::Changed } else { DmResult::None };
    }
    if ph != 2 {
        return DmResult::None;
    }
    press_move(&mut m.press, x, y);
    *dirty = app_press_release(a, &mut m.pressed);
    if !rect_contains(m.rect, m.press.x0 as i32, m.press.y0 as i32) {
        m.open = false; // a tap outside the sheet closes it
        *dirty = m.rect;
        return DmResult::Closed;
    }
    let Some(h) = hits_find(&m.hits, x, y).copied() else { return DmResult::None };
    if !press_is_tap(&m.press, h.r) {
        return DmResult::None;
    }
    match h.id {
        D_DONE | D_BACK => {
            m.open = false;
            *dirty = m.rect;
            return DmResult::Closed;
        }
        D_INVERT => {
            a.settings.invert = !a.settings.invert;
            settings_save(&a.settings);
            display_set_invert(a.settings.invert);
            dispmenu_draw(m, a);
            return DmResult::Inverted;
        }
        D_FIT => {
            a.settings.landscape_fit = !a.settings.landscape_fit;
            settings_save(&a.settings);
            dispmenu_draw(m, a);
            *dirty = m.rect;
            return DmResult::Refit;
        }
        D_NEWPAGE => {
            a.settings.gesture_new_page = !a.settings.gesture_new_page;
            settings_save(&a.settings);
            dispmenu_draw(m, a);
            *dirty = m.rect;
            return DmResult::Changed;
        }
        D_TIME12 => {
            a.settings.clock_12h = !a.settings.clock_12h;
            settings_save(&a.settings);
            crate::status::ask_change(a);
            dispmenu_draw(m, a);
            *dirty = m.rect;
            return DmResult::Changed;
        }
        D_SLEEP_MODE => {
            a.settings.sleep_mode = (a.settings.sleep_mode + 1) % 4;
            settings_save(&a.settings);
        }
        D_STATUS => status_cycle_size(a),
        D_CORNER => status_cycle_corner(a),
        D_PIN => {
            if editor_pin_sleep_page(a) {
                a.settings.sleep_mode = 1;
                settings_save(&a.settings);
                app_toast(a, tr("This page is now the sleep screen"), 2000);
            } else {
                app_toast(a, tr("Couldn't save sleep screen"), 2500);
            }
        }
        _ => return DmResult::None,
    }
    dispmenu_draw(m, a);
    *dirty = m.rect;
    DmResult::Changed
}
