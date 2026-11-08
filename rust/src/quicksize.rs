//! Quick sizes (2.1, lane L1): a pen button on the toolbar held HOLD_MS opens a card beside it with five chips,
//! each a width in millimetres drawn as a short stroke of that tool's own brush. A tap on a chip, or sliding the
//! contact that opened the card onto one and lifting there, sets the tool's width and closes the card. The eraser's card has a second row, Whole strokes / Precise.
//!
//! An editor-hosted card like linkcard.rs: it draws itself and takes the contacts, and says what was asked
//! (QsResult); the editor does it. A size is a setting, never a page change: it writes the tool's own
//! `size_min_mm` and `size_max_mm` (settings.ini, the keys every version reads) and nothing else, with no history
//! step: the max is the chip's width, the min keeps the tool's own thin-to-thick ratio. Its place is the shared popout's (popout.rs, popout_place).
//!
//! Layout (logical px at 300 dpi, scaled with ui_px): 24 px padding; a 64 px header (the tool's name at the
//! left, its width now at the right); 12 px; the chip row, 170 px tall: five size chips, 160 x 170 each (a white
//! well 136 x 100 with the sample, the width under it), 16 px apart. The eraser's second row is 110 px tall,
//! 12 px under the chips. 912 x 294 px (the eraser's 416 tall).

use crate::prelude::*;

// Hit ids.
const QS_CHIP: i32 = 1;
const QS_MODE: i32 = 3;

/// The chips of a card.
pub const QS_CHIPS: usize = 5;

/// The full-pressure widths in millimetres of a tool's five chips, smallest first: the pens follow the ISO
/// sqrt(2) pen-width series (each step about 1.4x), the mechanical pencil has lead sizes, the eraser doubles
/// (screen millimetres, 12 the format's largest). The tool's default max is always one of them. One array: the
/// owner's to change.
const PRESETS: [(ToolKind, [f32; QS_CHIPS]); TOOL_COUNT] = [
    (TOOL_PENCIL, [0.25, 0.35, 0.5, 0.7, 1.0]),
    (TOOL_MECH_PENCIL, [0.3, 0.5, 0.7, 0.9, 1.3]),
    (TOOL_CHARCOAL, [0.5, 0.7, 1.0, 1.4, 2.0]),
    (TOOL_BALLPOINT, [0.25, 0.35, 0.5, 0.7, 1.0]),
    (TOOL_FINELINER, [0.25, 0.35, 0.5, 0.7, 1.0]),
    (TOOL_MARKER, [0.7, 1.0, 1.4, 2.0, 2.8]),
    (TOOL_FOUNTAIN, [0.5, 0.7, 1.0, 1.4, 2.0]),
    (TOOL_CALLIGRAPHY, [1.0, 1.4, 2.0, 2.8, 4.0]),
    (TOOL_HIGHLIGHTER, [2.0, 2.8, 4.0, 5.6, 8.0]),
    (TOOL_ERASER, [1.0, 2.0, 4.0, 8.0, 12.0]),
];

/// Tool `t`'s five chip widths (the max, at full pressure); the pencil's for a tool out of range.
pub fn quicksize_presets(t: ToolKind) -> [f32; QS_CHIPS] {
    PRESETS.iter().find(|(k, _)| *k == t).map_or(PRESETS[0].1, |(_, p)| *p)
}

/// The chip whose width matches `p`'s size_max_mm within 0.005 mm (the tolerance settings_upgrade uses), whatever
/// the min; None after Pen feel tuned the max to something between.
pub fn quicksize_current(t: ToolKind, p: &ToolParams) -> Option<usize> {
    quicksize_presets(t).iter().position(|&hi| (p.size_max_mm - hi).abs() < 0.005)
}

/// Chip `i` written into `p`: size_max_mm is the chip's width, size_min_mm the chip's width times the tool's
/// current min/max ratio (its default ratio when the current max is 0 or not finite), rounded to 0.01 and
/// clamped to 0.10 .. the width; the eleven other fields as they were.
pub fn quicksize_apply(t: ToolKind, p: &mut ToolParams, i: usize) {
    let hi = quicksize_presets(t)[i.min(QS_CHIPS - 1)];
    let ratio = if p.size_max_mm.is_finite() && p.size_max_mm > 0.0 && p.size_min_mm.is_finite() {
        p.size_min_mm / p.size_max_mm
    } else {
        let mut d = ToolParams::default();
        tool_params_default(t, &mut d);
        d.size_min_mm / d.size_max_mm
    };
    p.size_max_mm = hi;
    p.size_min_mm = ((hi * ratio * 100.0).round() / 100.0).max(0.10).min(hi);
}

/// "0.28 mm", "0.5 mm", "1 mm", "12 mm": at most two decimals, no trailing zeros, a point in every language (as
/// Pen feel writes millimetres), the unit untranslated.
pub fn quicksize_mm_label(mm: f32) -> String {
    let mut s = format!("{:.2}", mm);
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s.push_str(" mm");
    s
}

fn u(v: f32) -> i32 {
    ui_px(v)
}

/// The card's size for tool `t`: 912 x 294, the eraser's 416 tall.
pub fn quicksize_size(t: ToolKind) -> (i32, i32) {
    let w = u(24.0) + QS_CHIPS as i32 * u(160.0) + (QS_CHIPS as i32 - 1) * u(16.0) + u(24.0);
    let mut h = u(24.0) + u(64.0) + u(12.0) + u(170.0) + u(24.0);
    if t == TOOL_ERASER {
        h += u(12.0) + u(110.0);
    }
    (w, h)
}

/// Where the card for `t` goes: beside the held button `anchor`, off the panel, clear of the clock's corner and
/// anything else in `avoid` (popout_place).
pub fn quicksize_place(t: ToolKind, anchor: Rect, screen: Rect, panel: Rect, avoid: &[Rect]) -> Rect {
    let (w, h) = quicksize_size(t);
    let mut list = Vec::with_capacity(1 + avoid.len());
    list.push(panel);
    list.extend_from_slice(avoid);
    popout_place(w, h, anchor, screen, &list)
}

/// What sits where on a card at `card`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QsLayout {
    pub header: Rect,
    pub chips: [Rect; QS_CHIPS],
    /// the eraser's Whole strokes and Precise (empty for a pen)
    pub modes: [Rect; 2],
}

pub fn quicksize_layout(t: ToolKind, card: Rect) -> QsLayout {
    let (x, y) = (card.x + u(24.0), card.y + u(24.0));
    let header = Rect { x, y, w: card.w - 2 * u(24.0), h: u(64.0) };
    let row_y = y + u(64.0) + u(12.0);
    let mut chips = [RECT_EMPTY; QS_CHIPS];
    for (i, c) in chips.iter_mut().enumerate() {
        *c = Rect { x: x + i as i32 * (u(160.0) + u(16.0)), y: row_y, w: u(160.0), h: u(170.0) };
    }
    let mut modes = [RECT_EMPTY; 2];
    if t == TOOL_ERASER {
        let inner = card.w - 2 * u(24.0);
        let w = (inner - u(16.0)) / 2;
        let my = row_y + u(170.0) + u(12.0);
        modes[0] = Rect { x, y: my, w, h: u(110.0) };
        modes[1] = Rect { x: x + w + u(16.0), y: my, w, h: u(110.0) };
    }
    QsLayout { header, chips, modes }
}

/// The card's state while it is up (memory only).
#[derive(Clone, Debug, Default)]
pub struct QuickSize {
    pub open: bool,
    pub tool: ToolKind,
    /// the button that was held, and the card's rect
    pub anchor: Rect,
    pub rect: Rect,
    pub lay: QsLayout,
    pub hits: HitList,
    /// the view's zoom when it opened: the samples are drawn at it
    pub zoom: f32,
    /// the contact that opened the card is still down (it may slide to pick)
    pub opener: bool,
    /// ... and has moved past the slop since: only then does its lift pick
    pub slid: bool,
    pub press: Press,
    /// the control drawn pressed now
    pub pressed: Option<Hit>,
}

/// What a contact on the card asks the editor to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QsResult {
    #[default]
    None,
    /// a chip picked
    Chip(usize),
    /// the eraser's Whole strokes (0) or Precise (1) picked
    Mode(i32),
    /// a control was pressed or let go without a pick: its rect (`dirty`) is to be refreshed
    Changed,
}

/// The card for tool `t`, beside `anchor`, at the view's `zoom`.
pub fn quicksize_open(q: &mut QuickSize, t: ToolKind, anchor: Rect, screen: Rect, panel: Rect, avoid: &[Rect], zoom: f32) {
    let rect = quicksize_place(t, anchor, screen, panel, avoid);
    *q = QuickSize { open: true, tool: t, anchor, rect, lay: quicksize_layout(t, rect), zoom, ..Default::default() };
}

// One sample stroke across a well: about 20 points along a gently rising curve, the pressure 0.2 rising to 1.0
// and easing to 0.6, the pen upright, a fixed clock (the same pixels every time).
fn sample_points(well: Rect) -> Vec<PenSample> {
    const N: usize = 20;
    let (x0, x1) = (well.x as f32 + well.w as f32 * 0.12, well.x as f32 + well.w as f32 * 0.88);
    let cy = well.y as f32 + well.h as f32 * 0.5;
    (0..N)
        .map(|i| {
            let t = i as f32 / (N - 1) as f32;
            let y = cy + well.h as f32 * (0.14 - 0.28 * t + 0.06 * (t * std::f32::consts::TAU).sin());
            let pressure = if t < 0.5 { 0.2 + 1.6 * t } else { 1.0 - 0.8 * (t - 0.5) };
            PenSample { x: lerpf(x0, x1, t), y, pressure, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 8 }
        })
        .collect()
}

/// The widths a chip's sample is drawn with: what a pick would write (the chip's width, the min in the tool's own
/// ratio) times the zoom (page millimetres), for the view the next stroke is written in; the eraser's are screen
/// millimetres and keep their size.
fn sample_params(t: ToolKind, i: usize, base: &ToolParams, zoom: f32) -> ToolParams {
    let mut p = *base;
    let z = if t == TOOL_ERASER || !(zoom > 0.0) { 1.0 } else { zoom };
    let mut picked = *base;
    quicksize_apply(t, &mut picked, i);
    p.size_min_mm = picked.size_min_mm * z;
    p.size_max_mm = picked.size_max_mm * z;
    p
}

// The sample in `well` (the canvas's clip is set to it): the tool's own brush at the chip's widths, a ring for the
// eraser, the highlighter's band over a short black line, white ink as an erase stroke on a black well.
fn draw_sample(a: &mut App, q: &QuickSize, well: Rect, i: usize) {
    let (t, dpi) = (q.tool, a.dpi);
    let white = a.settings.white && t != TOOL_ERASER;
    let width = quicksize_presets(t)[i];
    let App { screen, settings, .. } = &mut *a;
    let saved = screen.clip;
    screen.clip = rect_intersect(saved, well);
    if t == TOOL_ERASER {
        let r = width * dpi / 25.4 / 2.0;
        let (cx, cy) = ((well.x + well.w / 2) as f32, (well.y + well.h / 2) as f32);
        canvas_disc_aa(screen, cx, cy, r, 0, 1.0);
        canvas_disc_aa(screen, cx, cy, (r - u(4.0) as f32).max(0.0), 255, 1.0);
        screen.clip = saved;
        return;
    }
    let pts = sample_points(well);
    let prm = sample_params(t, i, &settings.tools[t.0 as usize], q.zoom);
    if white {
        canvas_fill(screen, well, 0);
    } else if t == TOOL_HIGHLIGHTER {
        let (a0, a1) = (pts[0], pts[pts.len() - 1]);
        canvas_line_aa(screen, a0.x, (a0.y + a1.y) / 2.0, a1.x, (a0.y + a1.y) / 2.0, u(4.0) as f32, 0);
    }
    let mut b = BrushState::default();
    brush_begin(&mut b, t, &prm, 0x51_5a + i as u32, dpi);
    if white {
        brush_set_erase(&mut b, None, 0, 0, 0);
    }
    for p in &pts {
        brush_add(&mut b, screen, p);
    }
    screen.clip = saved;
}

/// Draws the card at q.rect into a.screen and registers its hits; returns its rect.
pub fn quicksize_draw(q: &mut QuickSize, a: &mut App) -> Rect {
    hits_clear(&mut q.hits);
    let card = q.rect;
    let t = q.tool;
    {
        let s = &mut a.screen;
        ui_frame(s, card, u(24.0), 255, true, 0);
        ui_frame(s, card, u(24.0), 0, false, u(6.0));
    }
    let now = quicksize_current(t, &a.settings.tools[t.0 as usize]);
    let cur = a.settings.tools[t.0 as usize].size_max_mm;
    // The header: the tool's name, and at the right its width now (where a width tuned in Pen feel shows).
    let h = q.lay.header;
    let value = quicksize_mm_label(cur);
    let vw = (font_measure(FontFace::Regular, ui_small_size(), &value) + u(8.0)).min(h.w / 2);
    ui_text(&mut a.screen, FontFace::Bold, ui_small_size(), ui_small_size() * 0.8,
        Rect { x: h.x + u(8.0), y: h.y, w: h.w - vw - u(24.0), h: h.h }, tool_label(t), 1, false, 0);
    font_draw_centered(&mut a.screen, FontFace::Regular, ui_small_size(), Rect { x: h.x + h.w - vw, y: h.y, w: vw, h: h.h }, &value, 0);
    let white = a.settings.white && t != TOOL_ERASER;
    for i in 0..QS_CHIPS {
        let r = q.lay.chips[i];
        let on = now == Some(i);
        let width = quicksize_presets(t)[i];
        ui_button(&mut a.screen, r, IconKind::None, "", if on { ButtonStyle::Selected } else { ButtonStyle::Plain });
        // (the well stays white inside a Selected chip, so the ink sample stays black on white)
        let well = Rect { x: r.x + u(12.0), y: r.y + u(12.0), w: r.w - u(24.0), h: u(100.0) };
        canvas_fill(&mut a.screen, well, if white { 0 } else { 255 });
        if !on {
            canvas_rect_outline(&mut a.screen, well, u(2.0), 170);
        }
        draw_sample(a, q, well, i);
        let lr = Rect { x: r.x + u(6.0), y: well.y + well.h, w: r.w - u(12.0), h: r.y + r.h - well.y - well.h - u(6.0) };
        ui_text(&mut a.screen, FontFace::Regular, ui_small_size(), ui_small_size() * 0.8, lr, &quicksize_mm_label(width), 1, true, if on { 255 } else { 0 });
        hits_add(&mut q.hits, r, QS_CHIP, i as i32);
    }
    if t == TOOL_ERASER {
        const MODES: [&str; 2] = [
            // TRANSLATORS: the eraser's sizes card, the first choice of its second row: the eraser takes every
            // stroke it touches away whole. The same words as Pen feel's Eraser row (468 px at 44 px).
            nc_("eraser", "Whole strokes"),
            // TRANSLATORS: the eraser's sizes card, the second choice of its second row: the eraser takes away
            // only what it touches. The same words as Pen feel's Eraser row (468 px at 44 px).
            nc_("eraser", "Precise"),
        ];
        let mode = a.settings.eraser_mode;
        for i in 0..2 {
            let r = q.lay.modes[i];
            ui_button(&mut a.screen, r, IconKind::None, c_("eraser", MODES[i]), if mode == i as i32 { ButtonStyle::Selected } else { ButtonStyle::Plain });
            hits_add(&mut q.hits, r, QS_MODE, i as i32);
        }
    }
    card
}

fn hit_at(q: &QuickSize, x: f32, y: f32) -> Option<Hit> {
    hits_find(&q.hits, x, y).copied()
}

/// A pen or finger on the card (phase 0 down, 1 move, 2 up). A control inverts at once, on a DU of its own
/// (app_press_feedback). The contact that opened the card (`opener`, its down was the toolbar's) picks only after
/// it has moved past the slop onto a control and lifts there; a lift where the card opened never picks. Any other
/// contact picks the control it went down on, if it lifts there. A control entered while sliding is pressed on a
/// DU, one left is let go by the caller on a Ui (GL16 turns its black back to white exactly: a DU there would
/// leave issue #11's trace): `dirty` gets that rect. A pick leaves the controls as drawn: the caller composes
/// the card's place again.
pub fn quicksize_pointer(q: &mut QuickSize, a: &mut App, ph: i32, ev: &InEvent, dirty: &mut Rect) -> QsResult {
    *dirty = RECT_EMPTY;
    if !q.open {
        return QsResult::None;
    }
    let (x, y) = (ev.x, ev.y);
    if ph == 0 {
        press_begin(&mut q.press, x, y, ev.t_ms);
        q.opener = false;
        q.slid = false;
        q.pressed = hit_at(q, x, y);
        if let Some(h) = q.pressed {
            app_press_feedback(a, h.r);
        }
        return QsResult::None;
    }
    press_move(&mut q.press, x, y);
    if ph == 1 {
        if q.opener && q.press.moved {
            q.slid = true;
        }
        if !(q.opener && q.slid) {
            return QsResult::None;
        }
        let now = hit_at(q, x, y);
        if now == q.pressed {
            return QsResult::None;
        }
        if let Some(old) = q.pressed {
            canvas_invert(&mut a.screen, old.r);
            *dirty = old.r;
        }
        if let Some(new) = now {
            app_press_feedback(a, new.r);
        }
        q.pressed = now;
        return if rect_is_empty(*dirty) { QsResult::None } else { QsResult::Changed };
    }
    // The lift.
    let was = q.pressed.take();
    let opener = std::mem::take(&mut q.opener);
    let slid = std::mem::take(&mut q.slid);
    let Some(h) = was else {
        return QsResult::None;
    };
    let target = hit_at(q, x, y);
    let picked = !touch_cancelled(ev)
        && if opener { slid && target == Some(h) } else { press_is_tap(&q.press, h.r) };
    if !picked {
        canvas_invert(&mut a.screen, h.r);
        *dirty = h.r;
        return QsResult::Changed;
    }
    match h.id {
        QS_CHIP if (0..QS_CHIPS as i32).contains(&h.arg) => QsResult::Chip(h.arg as usize),
        QS_MODE => QsResult::Mode(h.arg),
        _ => QsResult::None,
    }
}

/// The centre of the card's control `name` for the runner's `tap panel`: `quicksize.0` to `quicksize.4`,
/// and the eraser's `quicksize.whole` and `quicksize.precise`. None for another name or
/// a card that is not up.
pub fn quicksize_control_point(q: &QuickSize, name: &str) -> Option<(f32, f32)> {
    if !q.open {
        return None;
    }
    let r = match name {
        "whole" if q.tool == TOOL_ERASER => q.lay.modes[0],
        "precise" if q.tool == TOOL_ERASER => q.lay.modes[1],
        n => *q.lay.chips.get(n.parse::<usize>().ok()?)?,
    };
    Some(((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32))
}

/// The runner's keys for the card, without "editor.quicksize.": "current" (the chip matching the tool's
/// max width, 0 to 4, or -1), "chip.N" (its label), "rect" (`x y w h`, empty closed), "white" (1 while its
/// samples are white on black) and "mode" (the eraser's, 0 or 1). false for another key.
pub fn quicksize_probe(q: &QuickSize, st: &Settings, key: &str, out: &mut String, cap: usize) -> bool {
    let t = q.tool;
    if key == "current" {
        let c = if q.open { quicksize_current(t, &st.tools[t.0 as usize]).map_or(-1, |i| i as i32) } else { -1 };
        csnprintf!(out, cap, "%d", c);
    } else if let Some(n) = key.strip_prefix("chip.") {
        let label = n.parse::<usize>().ok().filter(|&i| q.open && i < QS_CHIPS).map(|i| quicksize_mm_label(quicksize_presets(t)[i]));
        csnprintf!(out, cap, "%s", label.as_deref().unwrap_or(""));
    } else if key == "rect" {
        if q.open {
            csnprintf!(out, cap, "%d %d %d %d", q.rect.x, q.rect.y, q.rect.w, q.rect.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "white" {
        csnprintf!(out, cap, "%d", (q.open && st.white && t != TOOL_ERASER) as i32);
    } else if key == "mode" {
        csnprintf!(out, cap, "%d", if q.open && t == TOOL_ERASER { st.eraser_mode } else { -1 });
    } else {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const UP: Rect = Rect { x: 0, y: 0, w: 1860, h: 2480 };
    const SIDE: Rect = Rect { x: 0, y: 0, w: 2480, h: 1860 };
    const CORNER: Rect = Rect { x: 0, y: 0, w: 165, h: 165 };

    fn up() {
        ui_init(300.0);
    }

    fn inside_screen(r: Rect) -> bool {
        r.x >= 20 && r.y >= 20 && r.x + r.w <= 1860 - 20 && r.y + r.h <= 2480 - 20
    }

    fn defaults(t: ToolKind) -> ToolParams {
        let mut p = ToolParams::default();
        tool_params_default(t, &mut p);
        p
    }

    /// The table: every tool has five strictly ascending chips inside the format's range, its default max is
    /// one of them, and the eraser's top chip is at most what a page file keeps.
    #[test]
    fn the_table_holds_to_its_rules() {
        for k in 0..TOOL_COUNT {
            let t = ToolKind(k as i32);
            let chips = quicksize_presets(t);
            assert_eq!(chips.len(), 5);
            assert!(chips.windows(2).all(|w| w[0] < w[1]), "{}: ascending", tool_name(t));
            assert!(chips.iter().all(|&c| (0.10..=12.0).contains(&c)), "{}: range", tool_name(t));
            let d = defaults(t);
            assert!(quicksize_current(t, &d).is_some(), "{}: its default max is a chip", tool_name(t));
        }
        assert!(quicksize_presets(TOOL_ERASER)[4] <= ERASER_MM_MAX);
        // (the table covers every tool, once)
        assert_eq!(PRESETS.len(), TOOL_COUNT);
        assert!((0..TOOL_COUNT).all(|k| PRESETS.iter().filter(|(t, _)| t.0 == k as i32).count() == 1));
    }

    /// apply changes fields 0 and 1 only (the other eleven bit for bit), keeps the tool's own min/max ratio
    /// (a pencil tuned to 0.05..0.32 becomes 0.11..0.70 at the 0.7 chip), a constant-width tool stays min = max,
    /// an unusable max falls back to the default ratio; current finds the chip by its max whatever the min.
    #[test]
    fn apply_keeps_the_ratio_and_the_other_fields() {
        for k in 0..TOOL_COUNT {
            let t = ToolKind(k as i32);
            let mut p = defaults(t);
            p.grain = 0.123;
            p.smoothing = 0.456;
            p.pressure_gamma = 1.7;
            let before = p;
            for i in 0..QS_CHIPS {
                let mut q = before;
                quicksize_apply(t, &mut q, i);
                assert_eq!(quicksize_current(t, &q), Some(i));
                for f in 2..ToolParams::COUNT {
                    assert_eq!(q.get(f).to_bits(), before.get(f).to_bits(), "{} field {f}", tool_name(t));
                }
                assert_eq!(q.size_max_mm, quicksize_presets(t)[i]);
                assert!(q.size_min_mm >= 0.10 && q.size_min_mm <= q.size_max_mm);
                if before.size_min_mm == before.size_max_mm {
                    assert_eq!(q.size_min_mm, q.size_max_mm, "{}: constant stays constant", tool_name(t));
                }
            }
            // between chips (one tuned in Pen feel): none
            let mut odd = before;
            odd.size_max_mm = quicksize_presets(t)[1] + 0.03;
            assert_eq!(quicksize_current(t, &odd), None);
        }
        let mut p = defaults(TOOL_PENCIL);
        p.size_min_mm = 0.05;
        p.size_max_mm = 0.32;
        quicksize_apply(TOOL_PENCIL, &mut p, 3);
        assert_eq!((p.size_min_mm, p.size_max_mm), (0.11, 0.7));
        // current ignores the min
        assert_eq!(quicksize_current(TOOL_PENCIL, &p), Some(3));
        // a max that is not usable: the default ratio (pencil 0.13/0.35)
        for bad in [0.0, f32::NAN, f32::INFINITY, -1.0] {
            let mut p = defaults(TOOL_PENCIL);
            p.size_max_mm = bad;
            quicksize_apply(TOOL_PENCIL, &mut p, 3);
            assert_eq!((p.size_min_mm, p.size_max_mm), (0.26, 0.7), "max {bad}");
        }
    }

    /// The same pick twice gives the same bytes as once. (A different pick between is not undone: the ratio is
    /// rounded to 0.01 and floored at 0.10 each time, so min can drift across picks; that is the rule.)
    #[test]
    fn a_pick_is_repeatable() {
        for k in 0..TOOL_COUNT {
            let t = ToolKind(k as i32);
            for i in 0..QS_CHIPS {
                let mut once = defaults(t);
                quicksize_apply(t, &mut once, i);
                let mut twice = once;
                quicksize_apply(t, &mut twice, i);
                assert!((0..ToolParams::COUNT).all(|f| once.get(f).to_bits() == twice.get(f).to_bits()), "{} chip {i}", tool_name(t));
            }
        }
    }

    #[test]
    fn labels_drop_trailing_zeros() {
        for (mm, want) in [(0.25, "0.25 mm"), (0.5, "0.5 mm"), (1.0, "1 mm"), (1.75, "1.75 mm"), (12.0, "12 mm"), (0.35, "0.35 mm"), (5.6, "5.6 mm")] {
            assert_eq!(quicksize_mm_label(mm), want);
        }
    }

    /// Placement through popout_place: left of a right-edge toolbar, right of a left-edge one, below the held
    /// button for a toolbar in the middle, never over the held button or the clock corner, sideways too.
    #[test]
    fn the_card_is_placed_beside_the_held_button() {
        up();
        for t in [TOOL_FINELINER, TOOL_ERASER] {
            let (w, h) = quicksize_size(t);
            assert_eq!(w, 912);
            assert_eq!(h, if t == TOOL_ERASER { 416 } else { 294 });
            for screen in [UP, SIDE] {
                // the toolbar at the right edge: the card goes left of it, top level with the button
                let panel = Rect { x: screen.w - 156, y: 220, w: 128, h: 1520 };
                let button = Rect { x: panel.x + 8, y: 420, w: 112, h: 112 };
                let r = quicksize_place(t, button, screen, panel, &[CORNER]);
                assert!(r.x + r.w <= panel.x, "left of the panel: {r:?}");
                assert_eq!(r.y, 420);
                assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER) && !rect_overlaps(r, panel));
                // the toolbar at the left edge: right of it
                let panel = Rect { x: 28, y: 220, w: 128, h: 1520 };
                let button = Rect { x: panel.x + 8, y: 420, w: 112, h: 112 };
                let r = quicksize_place(t, button, screen, panel, &[CORNER]);
                assert!(r.x >= panel.x + panel.w, "right of the panel: {r:?}");
                assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER) && !rect_overlaps(r, panel));
            }
            // the toolbar well in from the edge, room for the card on its left: off it and off the button
            let panel = Rect { x: 1000, y: 300, w: 128, h: 1520 };
            let button = Rect { x: 1008, y: 1000, w: 112, h: 112 };
            let r = quicksize_place(t, button, UP, panel, &[CORNER]);
            assert!(!rect_overlaps(r, button) && !rect_overlaps(r, panel) && !rect_overlaps(r, CORNER));
            // the toolbar in the middle, no room beside it: never over the held button or the corner
            let panel = Rect { x: 700, y: 300, w: 128, h: 1520 };
            let button = Rect { x: 708, y: 1000, w: 112, h: 112 };
            let r = quicksize_place(t, button, UP, panel, &[CORNER]);
            assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER) && inside_screen(r));
            // no room beside a toolbar as wide as the page: below the held button
            let panel = Rect { x: 40, y: 300, w: 1780, h: 400 };
            let button = Rect { x: 800, y: 320, w: 112, h: 112 };
            let r = quicksize_place(t, button, UP, panel, &[CORNER]);
            assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER));
            assert!(r.y >= button.y + button.h);
        }
    }

    /// The layout: five chips in a row 16 px apart, all inside the card, upright and sideways; no Pen feel chip;
    /// the eraser's mode row under them, a pen's has none.
    #[test]
    fn the_layout_fits_the_card() {
        up();
        for t in [TOOL_PENCIL, TOOL_ERASER] {
            let (w, h) = quicksize_size(t);
            assert_eq!(w, 24 + 5 * 160 + 4 * 16 + 24);
            for screen in [UP, SIDE] {
                let panel = Rect { x: screen.w - 156, y: 220, w: 128, h: 1520 };
                let button = Rect { x: panel.x + 8, y: 420, w: 112, h: 112 };
                let card = quicksize_place(t, button, screen, panel, &[CORNER]);
                assert_eq!((card.w, card.h), (w, h));
                assert!(card.x >= 20 && card.y >= 20 && card.x + card.w <= screen.w - 20 && card.y + card.h <= screen.h - 20, "{card:?}");
                let l = quicksize_layout(t, card);
                for r in l.chips.iter() {
                    assert!(rect_contains_rect(card, *r));
                }
                assert!(rect_contains_rect(card, l.header));
                assert_eq!(l.chips[1].x - (l.chips[0].x + l.chips[0].w), 16);
                assert_eq!((l.chips[0].w, l.chips[0].h), (160, 170));
                assert_eq!(l.chips[4].x + l.chips[4].w + 24, card.x + card.w);
                if t == TOOL_ERASER {
                    assert!(l.modes.iter().all(|m| rect_contains_rect(card, *m) && m.y >= l.chips[0].y + 170 + 12));
                    assert!(!rect_overlaps(l.modes[0], l.modes[1]));
                } else {
                    assert_eq!(l.modes, [RECT_EMPTY; 2]);
                }
            }
        }
    }

    /// The probe's names and the runner's control points.
    #[test]
    fn the_runner_finds_the_controls() {
        up();
        let mut q = QuickSize::default();
        assert_eq!(quicksize_control_point(&q, "0"), None);
        quicksize_open(&mut q, TOOL_ERASER, Rect { x: 1700, y: 600, w: 112, h: 112 }, UP, Rect { x: 1690, y: 220, w: 128, h: 1520 }, &[CORNER], 1.0);
        for n in ["0", "1", "2", "3", "4", "whole", "precise"] {
            assert!(quicksize_control_point(&q, n).is_some(), "{n}");
        }
        assert_eq!(quicksize_control_point(&q, "5"), None);
        assert_eq!(quicksize_control_point(&q, "pen-feel"), None);
        assert_eq!(quicksize_control_point(&q, "nope"), None);
        quicksize_open(&mut q, TOOL_PENCIL, Rect { x: 1700, y: 600, w: 112, h: 112 }, UP, Rect { x: 1690, y: 220, w: 128, h: 1520 }, &[CORNER], 1.0);
        assert_eq!(quicksize_control_point(&q, "whole"), None);
        let mut st = Settings::default();
        tool_params_default(TOOL_PENCIL, &mut st.tools[TOOL_PENCIL.0 as usize]);
        let mut out = String::new();
        assert!(quicksize_probe(&q, &st, "current", &mut out, 64));
        assert_eq!(out, "1");
        assert!(quicksize_probe(&q, &st, "chip.4", &mut out, 64));
        assert_eq!(out, "1 mm");
        assert!(!quicksize_probe(&q, &st, "nope", &mut out, 64));
    }
}
