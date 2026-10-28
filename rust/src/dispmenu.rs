//! dispmenu.h: the Display sheet (front light brightness + warmth, invert Grimoire's colors).
//! A modal hosted by the editor and the library; it draws itself and handles pointer input.

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
}

#[derive(Clone, Debug, Default)]
pub struct DispMenu {
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
}

const D_BRIGHT: i32 = 1;
const D_WARM: i32 = 2;
const D_INVERT: i32 = 3;
const D_SLEEP_MODE: i32 = 4;
const D_PIN: i32 = 5;
const D_SLEEP_AFTER: i32 = 6;
const D_DONE: i32 = 7;
const D_STATUS: i32 = 8;

/// A dragged slider's row is drawn at most this often, as Pen feel's sliders are.
const DRAG_DRAW_MS: u32 = 80;

/// "Sleep after" choices (minutes, 0 = never); the slider moves along this list.
const K_SLEEP_STEPS: [i32; 8] = [0, 1, 2, 5, 10, 15, 30, 60];
const N_SLEEP_STEPS: i32 = K_SLEEP_STEPS.len() as i32;

/// The sleep screen row, one sentence per choice (settings.ini sleep_mode 0, 1, 2).
static K_MODE_ROWS: [&str; 4] = [
    // TRANSLATORS: the Display sheet's sleep screen row, which a tap moves to the next choice: what
    // the Kindle shows while it sleeps. About 1280 px at 44 px, upright and sideways.
    n_("Sleep screen: the page I'm on (tap to change)"),
    n_("Sleep screen: a pinned page (tap to change)"),
    n_("Sleep screen: the Kindle's own screensaver (tap to change)"),
    n_("Sleep screen: KOReader wallpaper (tap to change)"),
];

/// The clock and battery corner's size row (settings.ini status_size 0, 1, 2), one sentence per
/// choice. It sits beside Done, in the room the buttons row leaves, so the sheet's other rows stay
/// where they were.
static K_STATUS_ROWS: [&str; 3] = [
    // TRANSLATORS: the Display sheet's row beside Done: how large the clock and battery in the top
    // left corner are (a tap on that corner turns them on and off), which a tap moves to the next
    // size. 580 to 720 px at 34 px, with 890 px to hold it on one line (898 in Classic), or two lines.
    n_("Clock and battery size: small (tap to change)"),
    n_("Clock and battery size: medium (tap to change)"),
    n_("Clock and battery size: large (tap to change)"),
];

fn step_of(minutes: i32) -> i32 {
    let mut best = 0usize;
    for (i, step) in K_SLEEP_STEPS.iter().enumerate() {
        if (step - minutes).abs() < (K_SLEEP_STEPS[best] - minutes).abs() {
            best = i;
        }
    }
    best as i32
}

/// Rows: brightness, warmth, invert, [heading], sleep screen, [pin], sleep after.
fn row_h(a: &App) -> i32 {
    let rows = 6 + if editor_page_canvas(a).is_some() { 1 } else { 0 };
    let room = a.h - ui_px(120.0) - ui_px(72.0) - ui_px(190.0) - ui_px(80.0);
    let h = room / rows;
    clampi(h, ui_px(100.0), ui_px(140.0))
}

fn card_rect(a: &App) -> Rect {
    let mut w = a.w - 2 * ui_px(70.0);
    if w > ui_px(1500.0) {
        w = ui_px(1500.0);
    }
    let rows = 6 + if editor_page_canvas(a).is_some() { 1 } else { 0 };
    let h = ui_px(120.0) + ui_px(72.0) + rows * row_h(a) + ui_px(190.0);
    Rect { x: (a.w - w) / 2, y: (a.h - h) / 3, w, h }
}

pub fn dispmenu_open(m: &mut DispMenu, a: &mut App) {
    *m = DispMenu { open: true, ..Default::default() };
    light_get(&mut m.intensity, &mut m.warmth);
    m.rect = card_rect(a);
}

fn slider_for(m: &DispMenu, a: &App, which: i32, row: Rect, s: &mut Slider) {
    *s = Slider { step: 1.0, unit: "", ..Default::default() };
    if which == D_SLEEP_AFTER {
        s.min = 0.0;
        s.max = (N_SLEEP_STEPS - 1) as f32;
        // TRANSLATORS: the Display sheet's sliders, each left of its value ("12", "5 min", "never"):
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

/// Draws into a->screen; returns the rect covered.
pub fn dispmenu_draw(m: &mut DispMenu, a: &mut App) -> Rect {
    hits_clear(&mut m.hits);
    m.rect = card_rect(a);
    let in_ = ui_sheet(&mut a.screen, m.rect, tr("Display"));
    let rh = row_h(a);
    let mut y = in_.y;
    let have = light_available() && m.intensity >= 0;
    for i in 0..2 {
        let row = Rect { x: in_.x, y, w: in_.w, h: rh };
        y += rh; // (the C's loop step: the row is taken)
        canvas_fill(&mut a.screen, row, 255);
        if !have {
            if i == 0 {
                // TRANSLATORS: in the Display sheet where the sliders would be (1420 px at 44 px).
                font_draw_left(&mut a.screen, FontFace::Regular, ui_text_size(), row, tr("Front light controls are not available here."), 60);
            }
            continue;
        }
        if i == 1 && m.warmth < 0 {
            // no amber LEDs on this Kindle
            font_draw_left(&mut a.screen, FontFace::Regular, ui_text_size(), row, tr("This Kindle has no warmth control."), 60);
            continue;
        }
        draw_slider_row(m, a, if i == 0 { D_BRIGHT } else { D_WARM }, row);
        hits_add(&mut m.hits, row, if i == 0 { D_BRIGHT } else { D_WARM }, 0);
    }
    let inv = Rect { x: in_.x, y, w: in_.w, h: rh };
    ui_checkbox(&mut a.screen, inv, tr("Invert colors in Grimoire (white on black)"), a.settings.invert);
    hits_add(&mut m.hits, inv, D_INVERT, 0);
    y += rh;
    // Sleep section.
    canvas_hline(&mut a.screen, in_.x, in_.x + in_.w, y + rh / 3, ui_px(3.0), 0);
    // TRANSLATORS: the heading of the Display sheet's rows about the Kindle's sleep.
    font_draw_left(&mut a.screen, FontFace::Bold, ui_text_size(), Rect { x: in_.x, y: y + rh / 3 + ui_px(8.0), w: in_.w, h: rh * 2 / 3 }, tr("Sleep"), 0);
    y += rh;
    let mode = Rect { x: in_.x, y, w: in_.w, h: rh };
    canvas_fill(&mut a.screen, mode, 255);
    ui_icon(&mut a.screen, IconKind::Notebook, Rect { x: mode.x, y: mode.y + rh / 8, w: rh * 3 / 4, h: rh * 3 / 4 }, 0);
    let label = if a.native_pin { tr("Sleep screen: Kindle PIN (native unlock)") } else { tr(K_MODE_ROWS[clampi(a.settings.sleep_mode, 0, 3) as usize]) };
    font_draw_left(&mut a.screen, FontFace::Regular, ui_text_size(), Rect { x: mode.x + rh, y: mode.y, w: mode.w - rh, h: rh }, label, 0);
    if !a.native_pin {
        hits_add(&mut m.hits, mode, D_SLEEP_MODE, 0);
    }
    y += rh;
    if editor_page_canvas(a).is_some() {
        let pin = Rect { x: in_.x, y: y + ui_px(8.0), w: ui_px(560.0), h: rh - ui_px(16.0) };
        // TRANSLATORS: makes the open page the sleep screen (528 px at 44 px, or 34 px).
        ui_button(&mut a.screen, pin, IconKind::None, tr("Pin this page"), ButtonStyle::Plain);
        hits_add(&mut m.hits, pin, D_PIN, 0);
        y += rh;
    }
    let after = Rect { x: in_.x, y, w: in_.w, h: rh };
    draw_slider_row(m, a, D_SLEEP_AFTER, after);
    hits_add(&mut m.hits, after, D_SLEEP_AFTER, 0);
    let done = Rect { x: in_.x + in_.w - ui_px(360.0), y: in_.y + in_.h - ui_px(140.0), w: ui_px(360.0), h: ui_px(140.0) };
    // TRANSLATORS: closes a sheet: under its check mark here (322 px at 34 px), on Settings 342 px
    // at 44 px, on the keyboard 334 px at 44 px in bold.
    ui_button(&mut a.screen, done, IconKind::Check, tr("Done"), ButtonStyle::Primary);
    hits_add(&mut m.hits, done, D_DONE, 0);
    // The size of the clock and battery corner (status.rs), left of Done.
    let sz = Rect { x: in_.x, y: done.y, w: done.x - in_.x - ui_px(30.0), h: done.h };
    canvas_fill(&mut a.screen, sz, 255);
    ui_icon(&mut a.screen, IconKind::Clock, Rect { x: sz.x, y: sz.y + sz.h / 8, w: sz.h * 3 / 4, h: sz.h * 3 / 4 }, 0);
    // (At the small size throughout, so the three sentences look alike: the middle one would not fit
    // on a line at 44 px.)
    ui_text(
        &mut a.screen,
        FontFace::Regular,
        ui_small_size(),
        ui_small_size(),
        Rect { x: sz.x + sz.h, y: sz.y, w: sz.w - sz.h, h: sz.h },
        tr(K_STATUS_ROWS[clampi(a.settings.status_size, 0, STATUS_SIZES - 1) as usize]),
        2,
        false,
        0,
    );
    hits_add(&mut m.hits, sz, D_STATUS, 0);
    m.rect
}

/// Pointer phase 0 down / 1 move / 2 up. DM_CLOSED: the host should recompose m->rect.
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
        D_DONE => {
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
        D_SLEEP_MODE => {
            a.settings.sleep_mode = (a.settings.sleep_mode + 1) % 4;
            settings_save(&a.settings);
        }
        D_STATUS => status_cycle_size(a),
        D_PIN => {
            if editor_pin_sleep_page(a) {
                a.settings.sleep_mode = 1;
                settings_save(&a.settings);
                app_toast(a, tr("This page is now the sleep screen"), 2000);
            } else {
                app_toast(a, tr("Could not save the sleep screen"), 2500);
            }
        }
        _ => return DmResult::None,
    }
    dispmenu_draw(m, a);
    *dirty = m.rect;
    DmResult::Changed
}
