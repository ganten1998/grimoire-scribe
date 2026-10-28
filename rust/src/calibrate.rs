//! calibrate.c: pen then touch calibration. Targets are matched by ORDER using raw device
//! coordinates, so a badly wrong current mapping cannot get in the way. Buttons are full-width
//! bars that accept the other input (finger while calibrating the pen, pen for touch).

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Pen, // PH_PEN
    Touch, // PH_TOUCH
    Verify, // PH_VERIFY
}

/// 4 corners + center check
const NT: i32 = 5;
const K_TX: [f32; NT as usize] = [0.12, 0.88, 0.88, 0.12, 0.5];
const K_TY: [f32; NT as usize] = [0.12, 0.12, 0.88, 0.88, 0.5];

const B_NONE: i32 = 0;
const B_CANCEL: i32 = 1;
const B_SKIP: i32 = 2;
const B_SAVE: i32 = 3;
const B_REDO: i32 = 4;

/// The screen's state (calibrate.c's, which app.h keeps opaque): calibrate_init makes it, in a.cal.
#[derive(Default)]
pub struct CalibState {
    phase: Phase,
    target: i32,
    pts: [CalPoint; NT as usize],
    old_pen: InputCalib,
    old_touch: InputCalib,
    new_pen: InputCalib,
    new_touch: InputCalib,
    ret: ScreenId,
    /// a note under what to do: as long as a toast can be (App.toast's size)
    msg: FixedStr<256>,
    pad: Rect,
    drawing: bool,
    bs: BrushState,
    hits: HitList,
}

/// The C's `#define S (a->cal)`.
fn s(a: &mut App) -> &mut CalibState {
    a.cal.as_deref_mut().unwrap()
}

fn bar(a: &App, slot: i32) -> Rect {
    let h = ui_px(170.0);
    let m = ui_px(60.0);
    Rect { x: m, y: a.h / 2 + ui_px(260.0) + slot * (h + ui_px(30.0)), w: a.w - 2 * m, h }
}

fn draw(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let bars = [bar(a, 0), bar(a, 1), bar(a, 2)];
    {
        let App { screen: c, cal, .. } = a;
        let st = cal.as_deref_mut().unwrap();
        hits_clear(&mut st.hits);
        canvas_fill(c, Rect { x: 0, y: 0, w: aw, h: ah }, 255);
        let m = ui_px(60.0);
        // TRANSLATORS: calibration's three steps, a title each (1740 px at 77 px).
        let title = if st.phase == Phase::Pen { tr("Calibrate the pen") } else if st.phase == Phase::Touch { tr("Calibrate touch") } else { tr("Check it") };
        let mut tr_ = Rect { x: m, y: ah / 2 - ui_px(420.0), w: aw - 2 * m, h: ui_px(110.0) };
        font_draw_centered(c, FontFace::Bold, ui_title_size() * 1.2, tr_, title, 0);
        let mut sub = String::new();
        if st.phase == Phase::Verify {
            // TRANSLATORS: what to do at each step of calibration, under its title (1740 px at 44 px):
            // tap five targets, one after another, with the pen, then with a finger; then draw to check.
            csnprintf!(&mut sub, 256, "%s", tr("Draw in the box: the line should sit right under the tip."));
        } else if st.phase == Phase::Pen {
            // TRANSLATORS: the target is the circle with a cross on it; the second number is how many.
            csnprintf!(&mut sub, 256, tr("Tap with the pen tip on the center of the target (%d of %d)."), st.target + 1, NT);
        } else {
            csnprintf!(&mut sub, 256, tr("Tap with a finger on the center of the target (%d of %d)."), st.target + 1, NT);
        }
        tr_.y += ui_px(120.0);
        font_draw_centered(c, FontFace::Regular, ui_text_size(), tr_, &sub, 0);
        if !st.msg.is_empty() {
            tr_.y += ui_px(100.0);
            font_draw_centered(c, FontFace::Bold, ui_text_size(), tr_, st.msg.as_str(), 0);
        }
        if st.phase != Phase::Verify {
            let cx = K_TX[st.target as usize] * aw as f32;
            let cy = K_TY[st.target as usize] * ah as f32;
            let r = ui_px(80.0);
            let rf = r as f32;
            canvas_disc_aa(c, cx, cy, rf, 0, 1.0);
            canvas_disc_aa(c, cx, cy, rf - ui_px(8.0) as f32, 255, 1.0);
            canvas_disc_aa(c, cx, cy, ui_px(14.0) as f32, 0, 1.0);
            // (cx - r * 1.6f and cx + r * 1.6f: contracted, as clang makes them)
            canvas_line_aa(c, fmla(-rf, 1.6, cx), cy, fmla(rf, 1.6, cx), cy, ui_px(4.0) as f32, 0);
            canvas_line_aa(c, cx, fmla(-rf, 1.6, cy), cx, fmla(rf, 1.6, cy), ui_px(4.0) as f32, 0);
            let cancel = bars[0];
            // TRANSLATORS: calibration's buttons, bars across the screen (1708 px at 44 px). While the
            // pen is calibrated, a finger works them, and the pen while touch is.
            ui_button(c, cancel, IconKind::None, if st.phase == Phase::Pen { tr("Cancel (tap with a finger)") } else { tr("Cancel") }, ButtonStyle::Plain);
            hits_add(&mut st.hits, cancel, B_CANCEL, 0);
            if st.phase == Phase::Touch {
                let skip = bars[1];
                ui_button(c, skip, IconKind::None, tr("Touch was fine: skip (tap with the pen)"), ButtonStyle::Plain);
                hits_add(&mut st.hits, skip, B_SKIP, 0);
            }
        } else {
            st.pad = Rect { x: m, y: ah / 2 - ui_px(120.0), w: aw - 2 * m, h: ui_px(340.0) };
            ui_frame(c, rect_inflate(st.pad, ui_px(4.0)), ui_px(14.0), 0, false, ui_px(4.0));
            // (the C's slot++ for each: save, then redo, then cancel)
            let (save, redo, cancel) = (bars[0], bars[1], bars[2]);
            ui_button(c, save, IconKind::Check, tr("Save"), ButtonStyle::Primary);
            ui_button(c, redo, IconKind::None, tr("Start over"), ButtonStyle::Plain);
            ui_button(c, cancel, IconKind::None, tr("Cancel (keep the old calibration)"), ButtonStyle::Plain);
            hits_add(&mut st.hits, save, B_SAVE, 0);
            hits_add(&mut st.hits, redo, B_REDO, 0);
            hits_add(&mut st.hits, cancel, B_CANCEL, 0);
        }
    }
    app_invalidate_all(a, RefreshMode::Ui);
}

fn restart(a: &mut App, ph: Phase, msg: &str) {
    if s(a).drawing {
        display_live_ink(false); // a check stroke cut short (a finger on Start over)
    }
    let st = s(a);
    st.drawing = false;
    st.phase = ph;
    st.target = 0;
    st.msg.set_cut(msg); // (too long: cut between two characters)
    draw(a);
}

fn finish_pass(a: &mut App) {
    let mut c = InputCalib::default();
    let phase = s(a).phase;
    let pen = phase == Phase::Pen;
    // Fit on the 4 corners, then check the center.
    if !calib_fit(&s(a).pts[..4], &mut c) {
        // TRANSLATORS: calibration's notes, in bold under what to do (1740 px at 44 px).
        restart(a, phase, tr("Those taps did not make sense. Let's try again."));
        return;
    }
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    let p4 = s(a).pts[4];
    calib_apply(&c, p4.nx, p4.ny, &mut sx, &mut sy);
    let err = (sx - p4.sx).hypot(sy - p4.sy);
    if err > mm_to_px(4.0, a.dpi) {
        let mut m = String::new();
        csnprintf!(&mut m, 256, tr("The center was off by %.1f mm. Let's try again."), err * 25.4 / a.dpi);
        restart(a, phase, &m);
        return;
    }
    if pen {
        s(a).new_pen = c;
        input_set_calib(Some(&s(a).new_pen), None);
        restart(a, Phase::Touch, tr("Pen done."));
    } else {
        s(a).new_touch = c;
        input_set_calib(None, Some(&s(a).new_touch));
        restart(a, Phase::Verify, "");
    }
}

fn leave_to(a: &mut App) {
    let ret = s(a).ret;
    app_switch(a, if ret == ScreenId::Calib { ScreenId::Library } else { ret });
}

fn restore_old(a: &mut App) {
    let (mut dp, mut dt) = (InputCalib::default(), InputCalib::default());
    input_default_calib(Some(&mut dp), Some(&mut dt));
    let st = s(a);
    input_set_calib(Some(if st.old_pen.valid { &st.old_pen } else { &dp }), Some(if st.old_touch.valid { &st.old_touch } else { &dt }));
}

// ---- cross-screen entry points (app.h) ----

/// switch to SCR_CALIB (returns to prev_scr)
pub fn calibrate_open(a: &mut App) {
    let scr = a.scr;
    s(a).ret = scr;
    app_switch(a, ScreenId::Calib);
}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn calibrate_init(a: &mut App) {
    a.cal = Some(Box::default());
}
pub fn calibrate_free(a: &mut App) {
    a.cal = None;
}

pub fn calibrate_enter(a: &mut App) {
    let (old_pen, old_touch) = (a.settings.pen_calib, a.settings.touch_calib);
    let st = s(a);
    st.old_pen = old_pen;
    st.old_touch = old_touch;
    st.new_pen = InputCalib::default();
    st.new_touch = InputCalib::default();
    st.drawing = false;
    restart(a, Phase::Pen, "");
}

pub fn calibrate_leave(a: &mut App) {
    if s(a).drawing {
        display_live_ink(false); // left in the middle of a check stroke (a finger on Save)
    }
    s(a).drawing = false;
}

/// A calibration is saved with the panel it was made on, upright (the mapping is in its pixels), so
/// it follows the data to a panel of another size: app_init scales it there.
fn made_on(a: &App, w: &mut i32, h: &mut i32) {
    *w = if a.rotation & 1 != 0 { a.h } else { a.w };
    *h = if a.rotation & 1 != 0 { a.w } else { a.h };
}

fn button_at(a: &mut App, x: f32, y: f32) -> i32 {
    match hits_find(&s(a).hits, x, y) {
        Some(h) => h.id,
        None => B_NONE,
    }
}

/// Whether the calibration has a button at (x, y). The clock and battery corner leaves a tap there
/// to it; the targets are well clear of the corner.
pub fn calibrate_control_at(a: &App, x: f32, y: f32) -> bool {
    a.cal.as_deref().is_none_or(|s| hits_find(&s.hits, x, y).is_some())
}

pub fn calibrate_event(a: &mut App, ev: &InEvent) {
    let pen = ev.type_ == InType::PenDown || ev.type_ == InType::PenMove || ev.type_ == InType::PenUp;
    let ph = app_pointer_phase(ev);
    if ph < 0 {
        return;
    }
    if s(a).phase == Phase::Verify {
        if pen && ph == 0 && rect_contains(s(a).pad, ev.x as i32, ev.y as i32) {
            let mut p = ToolParams::default();
            tool_params_default(TOOL_BALLPOINT, &mut p);
            let dpi = a.dpi;
            brush_begin(&mut s(a).bs, TOOL_BALLPOINT, &p, 7, dpi);
            s(a).drawing = true;
            display_live_ink(true); // live ink, as in the editor (but no pen mode: plain DU, as before)
        }
        if s(a).drawing && pen {
            let smp = PenSample { x: ev.x, y: ev.y, pressure: ev.pressure.max(0.5), tilt_x: 0.0, tilt_y: 0.0, t_ms: ev.t_ms };
            let pad = s(a).pad;
            let saved = a.screen.clip;
            canvas_set_clip(&mut a.screen, pad);
            let d = brush_add(&mut a.cal.as_deref_mut().unwrap().bs, &mut a.screen, &smp);
            a.screen.clip = saved;
            app_invalidate(a, rect_intersect(d, pad), RefreshMode::Fast);
            if ph == 2 {
                s(a).drawing = false;
                display_live_ink(false);
            }
            return;
        }
        if ph != 2 {
            return;
        }
        match button_at(a, ev.x, ev.y) {
            B_SAVE => {
                a.settings.pen_calib = s(a).new_pen;
                let (mut w, mut h) = (0, 0);
                made_on(a, &mut w, &mut h);
                a.settings.pen_panel_w = w;
                a.settings.pen_panel_h = h;
                if s(a).new_touch.valid {
                    a.settings.touch_calib = s(a).new_touch;
                    made_on(a, &mut w, &mut h);
                    a.settings.touch_panel_w = w;
                    a.settings.touch_panel_h = h;
                }
                settings_save(&a.settings);
                input_set_calib(Some(&a.settings.pen_calib), if a.settings.touch_calib.valid { Some(&a.settings.touch_calib) } else { None });
                leave_to(a);
                app_toast(a, tr("Calibration saved"), 2000);
            }
            B_REDO => {
                restore_old(a);
                restart(a, Phase::Pen, "");
            }
            B_CANCEL => {
                restore_old(a);
                leave_to(a);
            }
            _ => {}
        }
        return;
    }
    if ph != 2 {
        return;
    }
    let target_input = (s(a).phase == Phase::Pen) == pen;
    if !target_input {
        // The other input operates the buttons.
        let b = button_at(a, ev.x, ev.y);
        if b == B_CANCEL {
            restore_old(a);
            leave_to(a);
        } else if b == B_SKIP {
            s(a).new_touch = InputCalib::default();
            restart(a, Phase::Verify, "");
        }
        return;
    }
    let (mut px, mut py) = (0.0f32, 0.0f32); // the calibration maps the device to the panel, whatever the rotation
    let t = s(a).target as usize;
    input_logical_to_panel(K_TX[t] * a.w as f32, K_TY[t] * a.h as f32, &mut px, &mut py);
    let st = s(a);
    st.pts[t] = CalPoint { nx: ev.nx, ny: ev.ny, sx: px, sy: py };
    st.msg.clear();
    st.target += 1;
    if st.target >= NT {
        finish_pass(a);
    } else {
        draw(a);
    }
}

pub fn calibrate_tick(_a: &mut App, _now: u32) {}
