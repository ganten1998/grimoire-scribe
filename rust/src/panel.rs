//! panel.h: the editor's floating panel's measures, as pure logic (unit-tested on the host by
//! tests/test_panel.c): its 15 buttons and one more for each pen pinned under the tool button, and
//! how they shrink to fit a screen too short for them (the Kindle turned sideways, with pens
//! pinned). The panel is sized to leave 12 px free above and below it, though it may be dragged up
//! to the screen's very edges. Then the pens on it, which next and previous tool go through, and the
//! tool picker's, which its own next and previous go through.

use crate::prelude::*;

/// Tool, eraser, undo, redo, clear, previous, next, links, widgets, paper, display, pen feel, settings, library.
pub const PANEL_BUTTONS: i32 = 15;
/// px at 300 dpi, the room panel_fit leaves above and below the panel
pub const PANEL_MARGIN: i32 = 12;

/// How far the panel is shrunk: every measure by scale, and the room between the buttons (their
/// gaps and the dividers) by space as well. The room gives first, so that the buttons, which are
/// what a finger aims at, keep their size as long as it can make up the difference.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PanelFit {
    pub scale: f32,
    pub space: f32,
}

/// expanded (icons only)
#[inline]
pub fn panel_w() -> i32 {
    ui_px(128.0)
}
/// the collapsed square
#[inline]
pub fn panel_box() -> i32 {
    ui_px(120.0)
}
#[inline]
pub fn panel_btn_h(f: PanelFit) -> i32 {
    ui_px(100.0 * f.scale)
}
#[inline]
pub fn panel_gap(f: PanelFit) -> i32 {
    ui_px(8.0 * f.scale * f.space)
}
#[inline]
pub fn panel_div_h(f: PanelFit) -> i32 {
    ui_px(20.0 * f.scale * f.space)
}
#[inline]
pub fn panel_head_h(f: PanelFit) -> i32 {
    ui_px(80.0 * f.scale)
}
#[inline]
pub fn panel_label_h(f: PanelFit) -> i32 {
    ui_px(60.0 * f.scale)
}
/// The page chip under previous (it opens Pages): the buttons' width (the panel's, less 14 px each side) and
/// 6 px more each side.
#[inline]
pub fn panel_chip_w() -> i32 {
    panel_w() - ui_px(28.0) + ui_px(12.0)
}
/// The page chip's text size: the small size, smaller with the panel where panel_fit shrinks it.
#[inline]
pub fn panel_chip_px(f: PanelFit) -> f32 {
    ui_small_size() * if f.scale < 1.0 { f.scale } else { 1.0 }
}
/// What panel_chip_whole found an answer for: the page count, Gothic, Classic's font (font_look, 0 in Gothic),
/// the chip's text size (its bits) and the chip's width.
type ChipKey = (i32, bool, usize, u32, i32);
thread_local! {
    /// panel_chip_whole's last answer, with what it was found for
    static CHIP_WHOLE: std::cell::Cell<Option<(ChipKey, bool)>> = const { std::cell::Cell::new(None) };
}

/// Whether the page chip of a notebook of `npages` pages shows the page and the count: true when "p / npages"
/// fits the chip for every page p, measured as draw_panel draws it (FontFace::Bold at panel_chip_px, within
/// panel_chip_w), false when any one would be cut short. Found once for each page count, look (Gothic, or
/// Classic's font as font_look numbers it), text size and chip width, and again when one of them changes (pages
/// added or deleted, the font switched, the screen turned): at most `npages` measures, from the last page down
/// to the first label that does not fit. A count that fits measures every label: at most 999 in the built-in
/// fonts, as no count of 1000 or more fits in any of them.
pub fn panel_chip_whole(npages: i32, f: PanelFit) -> bool {
    let (px, w) = (panel_chip_px(f), panel_chip_w());
    let gothic = font_gothic();
    let key = (npages, gothic, if gothic { 0 } else { font_look() }, px.to_bits(), w);
    if let Some((k, whole)) = CHIP_WHOLE.get() {
        if k == key {
            return whole;
        }
    }
    let mut lab = String::new();
    let whole = (1..=npages).rev().all(|p| {
        csnprintf!(&mut lab, 32, "%d / %d", p, npages);
        font_measure(FontFace::Bold, px, &lab) <= w
    });
    CHIP_WHOLE.set(Some((key, whole)));
    whole
}

/// The page chip's text for page `page` (from 1) of `npages`, decided once for the notebook (#119), so the chip
/// keeps one form while paging: "12 / 40" on every page where every page's label fits the chip at its size
/// (panel_chip_whole), else the page alone, "1995", on every page. Every count under 100 fits in every font at
/// every size the panel takes, and no count of 1000 or more does in any built-in font; between, it depends on
/// the font and the size (in Gothic "100 / 100" is cut, while every label of 101 or 111 pages fits). The page
/// alone keeps the chip's size, readable at arm's length, where the whole count made small enough to fit would
/// be two thirds to three quarters of it at "9999 / 9999"; Pages, which the chip opens, says the count in its
/// title.
pub fn panel_page_label(page: i32, npages: i32, f: PanelFit) -> String {
    let mut lab = String::new();
    if panel_chip_whole(npages, f) {
        csnprintf!(&mut lab, 32, "%d / %d", page, npages);
    } else {
        csnprintf!(&mut lab, 32, "%d", page);
    }
    lab
}

/// The header, the buttons (pins of them pinned pens) each with the gap under it, the page label,
/// the 3 dividers. Must match draw_panel.
pub fn panel_height(f: PanelFit, pins: i32) -> i32 {
    panel_head_h(f)
        + panel_gap(f)
        + (PANEL_BUTTONS + pins) * (panel_btn_h(f) + panel_gap(f))
        + panel_label_h(f)
        + panel_gap(f)
        + 3 * panel_div_h(f)
        + ui_px(6.0)
}

/// The fit for a screen screen_h px tall: full size where the panel fits within its margins; else
/// the room between the buttons down to half, then everything, each step measured in whole pixels
/// as the panel is drawn (the measures round, so a scale that should fit can round back over).
pub fn panel_fit(screen_h: i32, pins: i32) -> PanelFit {
    let mut f = PanelFit { scale: 1.0, space: 1.0 };
    let room = screen_h - 2 * ui_px(PANEL_MARGIN as f32);
    let mut s = 95;
    while s >= 50 && panel_height(f, pins) > room {
        f.space = s as f32 / 100.0;
        s -= 5;
    }
    let mut s = 99;
    while s >= 20 && panel_height(f, pins) > room {
        f.scale = s as f32 / 100.0;
        s -= 1;
    }
    f
}

// ---- its pens ----

/// The drawing tool the tool button shows (the pencil if settings.ini names the eraser there).
pub fn panel_button_pen(s: &Settings) -> ToolKind {
    let t = s.draw_tool;
    if (t.0 as u32) < TOOL_COUNT as u32 && t != TOOL_ERASER {
        t
    } else {
        TOOL_PENCIL
    }
}

/// Where a tap on the eraser in use goes back to, and what the pen's button draws white with while
/// the eraser is in use: the pen used just before it (settings.ini before_eraser) while that is
/// still on the panel, a pin or the tool button's; else the tool button's.
pub fn panel_eraser_back(s: &Settings) -> ToolKind {
    let b = s.before_eraser;
    let shown = b >= 0
        && (b as usize) < TOOL_COUNT
        && ToolKind(b) != TOOL_ERASER
        && (ToolKind(b) == panel_button_pen(s) || settings_pinned(s, ToolKind(b)));
    if shown {
        ToolKind(b)
    } else {
        panel_button_pen(s)
    }
}

/// The pens on the panel, top to bottom, into out; how many. The tool button's, but while that pen
/// is pinned (the button shows a grid then, and the pin the pen), then the pinned ones in pin order.
pub fn panel_pens(s: &Settings, out: &mut [ToolKind; PANEL_PINS_MAX + 1]) -> i32 {
    let mut n = 0;
    let dt = panel_button_pen(s);
    if !settings_pinned(s, dt) {
        out[n] = dt;
        n += 1;
    }
    let mut i = 0;
    while (i as i32) < s.npins && i < PANEL_PINS_MAX {
        out[n] = s.pins[i];
        n += 1;
        i += 1;
    }
    n as i32
}

/// The next or previous of n pens, round from the last to the first, after the pen in use. The
/// eraser is not one of them: from it the step is from the pen it goes back to, so erasing keeps
/// the place. A pen in use that is not among them (settings.ini changed by hand) steps to the first
/// or the last.
pub fn panel_step_among(s: &Settings, pens: &[ToolKind], next: bool) -> ToolKind {
    let n = pens.len();
    let cur = if s.current_tool == TOOL_ERASER { panel_eraser_back(s) } else { s.current_tool };
    let mut at: i32 = -1;
    for (i, p) in pens.iter().enumerate() {
        if *p == cur {
            at = i as i32;
        }
    }
    if at < 0 {
        return pens[if next { 0 } else { n - 1 }];
    }
    pens[(at as usize + if next { 1 } else { n - 1 }) % n]
}

/// The tool picker's next or previous pen: its nine in its order (tool_order's first TOOL_PENS),
/// which picker-next and picker-previous go through.
pub fn picker_step(s: &Settings, next: bool) -> ToolKind {
    panel_step_among(s, &TOOL_ORDER[..TOOL_PENS], next)
}

/// Next or previous tool: the pens on the panel alone, so a press never leaves them for the tool
/// picker; with fewer than two there, the picker's nine (as picker_step).
pub fn panel_step(s: &Settings, next: bool) -> ToolKind {
    let mut pens = [ToolKind::default(); PANEL_PINS_MAX + 1];
    let n = panel_pens(s, &mut pens);
    if n >= 2 {
        panel_step_among(s, &pens[..n as usize], next)
    } else {
        picker_step(s, next)
    }
}
