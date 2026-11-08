//! Quick sizes (2.1, lane L1): a pen button on the toolbar held HOLD_MS opens a card beside it with four chips,
//! each a width in millimetres drawn as a short stroke of that tool's own brush, and a Pen feel chip. A tap on a
//! chip, or sliding the contact that opened the card onto one and lifting there, sets the tool's width and closes
//! the card. The eraser's card has a second row, Whole strokes / Precise.
//!
//! An editor-hosted card like linkcard.rs: it draws itself and takes the contacts, and says what was asked
//! (QsResult); the editor does it. A size is a setting, never a page change: it writes the tool's own
//! `size_min_mm` and `size_max_mm` (settings.ini, the keys every version reads) and nothing else, with no history
//! step. Its place is the shared popout's (popout.rs, popout_place).
//!
//! Layout (logical px at 300 dpi, scaled with ui_px): 24 px padding; a 64 px header (the tool's name at the
//! left, its width now at the right); 12 px; the chip row, 170 px tall: four size chips, 160 x 170 each (a white
//! well 136 x 100 with the sample, the width under it), 16 px apart, then, 24 px on, the Pen feel chip (the
//! Tune icon over its label, as wide as the label needs, 160 to 300 px). The eraser's second row is 110 px
//! tall, 12 px under the chips. About 960 x 294 px (the eraser's 416 tall).

use crate::prelude::*;

// Hit ids.
const QS_CHIP: i32 = 1;
const QS_FEEL: i32 = 2;
const QS_MODE: i32 = 3;

/// The chips of a card.
pub const QS_CHIPS: usize = 4;

/// The pairs (min, max) in millimetres of a tool's four chips, widest last. The label of a chip is the max, the
/// width at full pressure; the min is the max times the tool's own proportion (rounded to 0.01, never under the
/// format's 0.10). The tool's default pair is always one of them. The eraser's are screen millimetres. One array:
/// the owner's to change.
const PRESETS: [(ToolKind, [(f32, f32); QS_CHIPS]); TOOL_COUNT] = [
    (TOOL_PENCIL, [(0.10, 0.25), (0.12, 0.32), (0.19, 0.50), (0.30, 0.80)]),
    (TOOL_MECH_PENCIL, [(0.3, 0.3), (0.5, 0.5), (0.7, 0.7), (0.9, 0.9)]),
    (TOOL_CHARCOAL, [(0.22, 0.40), (0.30, 0.55), (0.55, 1.00), (1.09, 2.00)]),
    (TOOL_BALLPOINT, [(0.22, 0.35), (0.30, 0.48), (0.44, 0.70), (0.62, 1.00)]),
    (TOOL_FINELINER, [(0.2, 0.2), (0.28, 0.28), (0.4, 0.4), (0.6, 0.6)]),
    (TOOL_MARKER, [(0.7, 0.7), (1.0, 1.0), (1.6, 1.6), (2.5, 2.5)]),
    (TOOL_FOUNTAIN, [(0.15, 0.60), (0.25, 1.00), (0.35, 1.40), (0.50, 2.00)]),
    (TOOL_CALLIGRAPHY, [(1.0, 1.0), (1.75, 1.75), (2.4, 2.4), (3.8, 3.8)]),
    (TOOL_HIGHLIGHTER, [(3.0, 3.0), (4.0, 4.0), (5.0, 5.0), (7.0, 7.0)]),
    (TOOL_ERASER, [(0.71, 2.0), (1.43, 4.0), (2.50, 7.0), (4.29, 12.0)]),
];

/// Tool `t`'s four chips as (size_min_mm, size_max_mm); the pencil's for a tool out of range.
pub fn quicksize_presets(t: ToolKind) -> [(f32, f32); QS_CHIPS] {
    PRESETS.iter().find(|(k, _)| *k == t).map_or(PRESETS[0].1, |(_, p)| *p)
}

/// The chip whose pair both of `p`'s widths match within 0.005 mm (the tolerance settings_upgrade uses), or
/// None after Pen feel tuned the widths to something between.
pub fn quicksize_current(t: ToolKind, p: &ToolParams) -> Option<usize> {
    quicksize_presets(t).iter().position(|&(lo, hi)| (p.size_min_mm - lo).abs() < 0.005 && (p.size_max_mm - hi).abs() < 0.005)
}

/// Chip `i`'s pair written into `p`: size_min_mm and size_max_mm, the eleven other fields as they were.
pub fn quicksize_apply(t: ToolKind, p: &mut ToolParams, i: usize) {
    let (lo, hi) = quicksize_presets(t)[i.min(QS_CHIPS - 1)];
    p.size_min_mm = lo;
    p.size_max_mm = hi;
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

/// The Pen feel chip's width: what its label needs, 160 to 300 px.
pub fn quicksize_feel_w() -> i32 {
    clampi(ui_button_need_w(u(170.0), IconKind::Tune, tr("Pen feel"), ButtonStyle::Plain), u(160.0), u(300.0))
}

/// The card's size for tool `t` with a Pen feel chip `feel_w` wide: about 960 x 294, the eraser's 416 tall.
pub fn quicksize_size(t: ToolKind, feel_w: i32) -> (i32, i32) {
    let w = u(24.0) + QS_CHIPS as i32 * u(160.0) + (QS_CHIPS as i32 - 1) * u(16.0) + u(24.0) + feel_w + u(24.0);
    let mut h = u(24.0) + u(64.0) + u(12.0) + u(170.0) + u(24.0);
    if t == TOOL_ERASER {
        h += u(12.0) + u(110.0);
    }
    (w, h)
}

/// Where the card for `t` goes: beside the held button `anchor`, off the panel, clear of the clock's corner and
/// anything else in `avoid` (popout_place).
pub fn quicksize_place(t: ToolKind, feel_w: i32, anchor: Rect, screen: Rect, panel: Rect, avoid: &[Rect]) -> Rect {
    let (w, h) = quicksize_size(t, feel_w);
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
    pub feel: Rect,
    /// the eraser's Whole strokes and Precise (empty for a pen)
    pub modes: [Rect; 2],
}

pub fn quicksize_layout(t: ToolKind, card: Rect, feel_w: i32) -> QsLayout {
    let (x, y) = (card.x + u(24.0), card.y + u(24.0));
    let header = Rect { x, y, w: card.w - 2 * u(24.0), h: u(64.0) };
    let row_y = y + u(64.0) + u(12.0);
    let mut chips = [RECT_EMPTY; QS_CHIPS];
    for (i, c) in chips.iter_mut().enumerate() {
        *c = Rect { x: x + i as i32 * (u(160.0) + u(16.0)), y: row_y, w: u(160.0), h: u(170.0) };
    }
    let feel = Rect { x: x + QS_CHIPS as i32 * u(160.0) + (QS_CHIPS as i32 - 1) * u(16.0) + u(24.0), y: row_y, w: feel_w, h: u(170.0) };
    let mut modes = [RECT_EMPTY; 2];
    if t == TOOL_ERASER {
        let inner = card.w - 2 * u(24.0);
        let w = (inner - u(16.0)) / 2;
        let my = row_y + u(170.0) + u(12.0);
        modes[0] = Rect { x, y: my, w, h: u(110.0) };
        modes[1] = Rect { x: x + w + u(16.0), y: my, w, h: u(110.0) };
    }
    QsLayout { header, chips, feel, modes }
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
    /// the Pen feel chip
    PenFeel,
    /// a control was pressed or let go without a pick: its rect (`dirty`) is to be refreshed
    Changed,
}

/// The card for tool `t`, beside `anchor`, at the view's `zoom`.
pub fn quicksize_open(q: &mut QuickSize, t: ToolKind, anchor: Rect, screen: Rect, panel: Rect, avoid: &[Rect], zoom: f32) {
    let feel_w = quicksize_feel_w();
    let rect = quicksize_place(t, feel_w, anchor, screen, panel, avoid);
    *q = QuickSize { open: true, tool: t, anchor, rect, lay: quicksize_layout(t, rect, feel_w), zoom, ..Default::default() };
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

/// The widths a chip's sample is drawn with: the pair times the zoom (page millimetres), for the view the next
/// stroke is written in; the eraser's are screen millimetres and keep their size.
fn sample_params(t: ToolKind, pair: (f32, f32), base: &ToolParams, zoom: f32) -> ToolParams {
    let mut p = *base;
    let z = if t == TOOL_ERASER || !(zoom > 0.0) { 1.0 } else { zoom };
    p.size_min_mm = pair.0 * z;
    p.size_max_mm = pair.1 * z;
    p
}

// The sample in `well` (the canvas's clip is set to it): the tool's own brush at the chip's widths, a ring for the
// eraser, the highlighter's band over a short black line, white ink as an erase stroke on a black well.
fn draw_sample(a: &mut App, q: &QuickSize, well: Rect, i: usize) {
    let (t, dpi) = (q.tool, a.dpi);
    let white = a.settings.white && t != TOOL_ERASER;
    let pair = quicksize_presets(t)[i];
    let App { screen, settings, .. } = &mut *a;
    let saved = screen.clip;
    screen.clip = rect_intersect(saved, well);
    if t == TOOL_ERASER {
        let r = pair.1 * dpi / 25.4 / 2.0;
        let (cx, cy) = ((well.x + well.w / 2) as f32, (well.y + well.h / 2) as f32);
        canvas_disc_aa(screen, cx, cy, r, 0, 1.0);
        canvas_disc_aa(screen, cx, cy, (r - u(4.0) as f32).max(0.0), 255, 1.0);
        screen.clip = saved;
        return;
    }
    let pts = sample_points(well);
    let prm = sample_params(t, pair, &settings.tools[t.0 as usize], q.zoom);
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
        let pair = quicksize_presets(t)[i];
        ui_button(&mut a.screen, r, IconKind::None, "", if on { ButtonStyle::Selected } else { ButtonStyle::Plain });
        // (the well stays white inside a Selected chip, so the ink sample stays black on white)
        let well = Rect { x: r.x + u(12.0), y: r.y + u(12.0), w: r.w - u(24.0), h: u(100.0) };
        canvas_fill(&mut a.screen, well, if white { 0 } else { 255 });
        if !on {
            canvas_rect_outline(&mut a.screen, well, u(2.0), 170);
        }
        draw_sample(a, q, well, i);
        let lr = Rect { x: r.x + u(6.0), y: well.y + well.h, w: r.w - u(12.0), h: r.y + r.h - well.y - well.h - u(6.0) };
        ui_text(&mut a.screen, FontFace::Regular, ui_small_size(), ui_small_size() * 0.8, lr, &quicksize_mm_label(pair.1), 1, true, if on { 255 } else { 0 });
        hits_add(&mut q.hits, r, QS_CHIP, i as i32);
    }
    // TRANSLATORS: the label of the sizes card's Pen feel chip, under the Tune icon (the card a pen button on the
    // toolbar held opens): up to 300 px at 34 px. The same words as the toolbar action that opens Pen feel.
    ui_button(&mut a.screen, q.lay.feel, IconKind::Tune, tr("Pen feel"), ButtonStyle::Plain);
    hits_add(&mut q.hits, q.lay.feel, QS_FEEL, 0);
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
        QS_FEEL => QsResult::PenFeel,
        _ => QsResult::None,
    }
}

/// The centre of the card's control `name` for the runner's `tap panel`: `quicksize.0` to `quicksize.3`,
/// `quicksize.pen-feel`, and the eraser's `quicksize.whole` and `quicksize.precise`. None for another name or
/// a card that is not up.
pub fn quicksize_control_point(q: &QuickSize, name: &str) -> Option<(f32, f32)> {
    if !q.open {
        return None;
    }
    let r = match name {
        "pen-feel" => q.lay.feel,
        "whole" if q.tool == TOOL_ERASER => q.lay.modes[0],
        "precise" if q.tool == TOOL_ERASER => q.lay.modes[1],
        n => *q.lay.chips.get(n.parse::<usize>().ok()?)?,
    };
    Some(((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32))
}

/// The runner's keys for the card, without "editor.quicksize.": "current" (the chip matching the tool's
/// widths, 0 to 3, or -1), "chip.N" (its label), "rect" (`x y w h`, empty closed), "white" (1 while its
/// samples are white on black) and "mode" (the eraser's, 0 or 1). false for another key.
pub fn quicksize_probe(q: &QuickSize, st: &Settings, key: &str, out: &mut String, cap: usize) -> bool {
    let t = q.tool;
    if key == "current" {
        let c = if q.open { quicksize_current(t, &st.tools[t.0 as usize]).map_or(-1, |i| i as i32) } else { -1 };
        csnprintf!(out, cap, "%d", c);
    } else if let Some(n) = key.strip_prefix("chip.") {
        let label = n.parse::<usize>().ok().filter(|&i| q.open && i < QS_CHIPS).map(|i| quicksize_mm_label(quicksize_presets(t)[i].1));
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

    /// The table: every tool has four ascending chips inside the format's range with min at or under max; the
    /// tool's own default is one of them; each min is the max times the default proportion, rounded to 0.01 and
    /// floored at 0.10; the eraser's top chip is at most what a page file keeps.
    #[test]
    fn the_table_holds_to_its_rules() {
        for k in 0..TOOL_COUNT {
            let t = ToolKind(k as i32);
            let chips = quicksize_presets(t);
            let d = defaults(t);
            let prop = d.size_min_mm / d.size_max_mm;
            assert!(chips.windows(2).all(|w| w[0].1 < w[1].1), "{}: ascending", tool_name(t));
            for &(lo, hi) in &chips {
                assert!((0.10..=12.0).contains(&lo) && (0.10..=12.0).contains(&hi) && lo <= hi, "{}: {lo}..{hi}", tool_name(t));
                let want = ((hi * prop * 100.0).round() / 100.0).max(0.10);
                assert!((lo - want).abs() <= 0.0101, // (a half step: the table keeps 0.62 where 0.625 rounds either way)
                     "{}: min {lo} for max {hi} (proportion {prop}) wants {want}", tool_name(t));
            }
            assert!(quicksize_current(t, &d).is_some(), "{}: its default is a chip", tool_name(t));
        }
        assert!(quicksize_presets(TOOL_ERASER)[3].1 <= ERASER_MM_MAX);
        // (the table covers every tool, once)
        assert_eq!(PRESETS.len(), TOOL_COUNT);
        assert!((0..TOOL_COUNT).all(|k| PRESETS.iter().filter(|(t, _)| t.0 == k as i32).count() == 1));
    }

    /// apply changes fields 0 and 1 only (the other eleven bit for bit); current finds the chip just applied and
    /// None for a Pen feel value between chips.
    #[test]
    fn apply_touches_the_two_widths_and_current_finds_them() {
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
                assert_eq!((q.size_min_mm, q.size_max_mm), quicksize_presets(t)[i]);
            }
            // between chips (one tuned in Pen feel): none
            let mut odd = before;
            odd.size_min_mm = 0.11;
            odd.size_max_mm = quicksize_presets(t)[1].1 + 0.03;
            assert_eq!(quicksize_current(t, &odd), None);
        }
    }

    /// Apply 2, then 0, then 2 gives the same bytes as apply 2 once.
    #[test]
    fn a_pick_is_repeatable() {
        for k in 0..TOOL_COUNT {
            let t = ToolKind(k as i32);
            let mut once = defaults(t);
            quicksize_apply(t, &mut once, 2);
            let mut again = defaults(t);
            for i in [2, 0, 2] {
                quicksize_apply(t, &mut again, i);
            }
            assert!((0..ToolParams::COUNT).all(|f| once.get(f).to_bits() == again.get(f).to_bits()));
        }
    }

    #[test]
    fn labels_drop_trailing_zeros() {
        for (mm, want) in [(0.25, "0.25 mm"), (0.5, "0.5 mm"), (1.0, "1 mm"), (1.75, "1.75 mm"), (12.0, "12 mm"), (0.28, "0.28 mm"), (7.0, "7 mm")] {
            assert_eq!(quicksize_mm_label(mm), want);
        }
    }

    /// Placement through popout_place: left of a right-edge toolbar, right of a left-edge one, below the held
    /// button for a toolbar in the middle, never over the held button or the clock corner, sideways too.
    #[test]
    fn the_card_is_placed_beside_the_held_button() {
        up();
        let feel = u(200.0);
        for t in [TOOL_FINELINER, TOOL_ERASER] {
            let (w, h) = quicksize_size(t, feel);
            assert_eq!(w, 960);
            assert_eq!(h, if t == TOOL_ERASER { 416 } else { 294 });
            for screen in [UP, SIDE] {
                // the toolbar at the right edge: the card goes left of it, top level with the button
                let panel = Rect { x: screen.w - 156, y: 220, w: 128, h: 1520 };
                let button = Rect { x: panel.x + 8, y: 420, w: 112, h: 112 };
                let r = quicksize_place(t, feel, button, screen, panel, &[CORNER]);
                assert!(r.x + r.w <= panel.x, "left of the panel: {r:?}");
                assert_eq!(r.y, 420);
                assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER) && !rect_overlaps(r, panel));
                // the toolbar at the left edge: right of it
                let panel = Rect { x: 28, y: 220, w: 128, h: 1520 };
                let button = Rect { x: panel.x + 8, y: 420, w: 112, h: 112 };
                let r = quicksize_place(t, feel, button, screen, panel, &[CORNER]);
                assert!(r.x >= panel.x + panel.w, "right of the panel: {r:?}");
                assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER) && !rect_overlaps(r, panel));
            }
            // the toolbar well in from the edge, room for the card on its left: off it and off the button
            let panel = Rect { x: 1000, y: 300, w: 128, h: 1520 };
            let button = Rect { x: 1008, y: 1000, w: 112, h: 112 };
            let r = quicksize_place(t, feel, button, UP, panel, &[CORNER]);
            assert!(!rect_overlaps(r, button) && !rect_overlaps(r, panel) && !rect_overlaps(r, CORNER));
            // the toolbar in the middle, no room beside it: never over the held button or the corner
            let panel = Rect { x: 700, y: 300, w: 128, h: 1520 };
            let button = Rect { x: 708, y: 1000, w: 112, h: 112 };
            let r = quicksize_place(t, feel, button, UP, panel, &[CORNER]);
            assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER) && inside_screen(r));
            // no room beside a toolbar as wide as the page: below the held button
            let panel = Rect { x: 40, y: 300, w: 1780, h: 400 };
            let button = Rect { x: 800, y: 320, w: 112, h: 112 };
            let r = quicksize_place(t, feel, button, UP, panel, &[CORNER]);
            assert!(!rect_overlaps(r, button) && !rect_overlaps(r, CORNER));
            assert!(r.y >= button.y + button.h);
        }
    }

    /// The layout: four chips in a row 16 px apart, the Pen feel chip 24 px on, all inside the card; the
    /// eraser's mode row under them, a pen's has none.
    #[test]
    fn the_layout_fits_the_card() {
        up();
        let feel = u(200.0);
        for t in [TOOL_PENCIL, TOOL_ERASER] {
            let (w, h) = quicksize_size(t, feel);
            let card = Rect { x: 100, y: 200, w, h };
            let l = quicksize_layout(t, card, feel);
            for r in l.chips.iter().chain(std::iter::once(&l.feel)) {
                assert!(rect_contains_rect(card, *r));
            }
            assert_eq!(l.chips[1].x - (l.chips[0].x + l.chips[0].w), 16);
            assert_eq!(l.feel.x - (l.chips[3].x + l.chips[3].w), 24);
            assert_eq!((l.chips[0].w, l.chips[0].h), (160, 170));
            assert_eq!(l.feel.x + l.feel.w + 24, card.x + card.w);
            if t == TOOL_ERASER {
                assert!(l.modes.iter().all(|m| rect_contains_rect(card, *m) && m.y >= l.chips[0].y + 170 + 12));
                assert!(!rect_overlaps(l.modes[0], l.modes[1]));
            } else {
                assert_eq!(l.modes, [RECT_EMPTY; 2]);
            }
        }
    }

    /// The probe's names and the runner's control points.
    #[test]
    fn the_runner_finds_the_controls() {
        up();
        let mut q = QuickSize::default();
        assert_eq!(quicksize_control_point(&q, "0"), None);
        let feel = u(200.0);
        quicksize_open(&mut q, TOOL_ERASER, Rect { x: 1700, y: 600, w: 112, h: 112 }, UP, Rect { x: 1690, y: 220, w: 128, h: 1520 }, &[CORNER], 1.0);
        for n in ["0", "1", "2", "3", "pen-feel", "whole", "precise"] {
            assert!(quicksize_control_point(&q, n).is_some(), "{n}");
        }
        assert_eq!(quicksize_control_point(&q, "4"), None);
        assert_eq!(quicksize_control_point(&q, "nope"), None);
        quicksize_open(&mut q, TOOL_PENCIL, Rect { x: 1700, y: 600, w: 112, h: 112 }, UP, Rect { x: 1690, y: 220, w: 128, h: 1520 }, &[CORNER], 1.0);
        assert_eq!(quicksize_control_point(&q, "whole"), None);
        let mut st = Settings::default();
        tool_params_default(TOOL_PENCIL, &mut st.tools[TOOL_PENCIL.0 as usize]);
        let mut out = String::new();
        assert!(quicksize_probe(&q, &st, "current", &mut out, 64));
        assert_eq!(out, "1");
        assert!(quicksize_probe(&q, &st, "chip.3", &mut out, 64));
        assert_eq!(out, "0.8 mm");
        assert!(!quicksize_probe(&q, &st, "nope", &mut out, 64));
    }
}
