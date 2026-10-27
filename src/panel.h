// panel.h: the editor's floating panel's measures, as pure logic (unit-tested on the host by
// tests/test_panel.c): its 11 buttons and one more for each pen pinned under the tool button, and
// how they shrink to fit a screen too short for them (the Kindle turned sideways, with pens
// pinned). The panel keeps 12 px free above and below it. Then the pens on it, which next and
// previous tool go through, and the tool picker's, which its own next and previous go through.
#pragma once
#include "store.h"
#include "ui.h"

// Tool, eraser, undo, redo, clear, previous, next, paper, display, pen feel, library.
#define PANEL_BUTTONS 11
#define PANEL_MARGIN 12  // px at 300 dpi

// How far the panel is shrunk: every measure by scale, and the room between the buttons (their
// gaps and the dividers) by space as well. The room gives first, so that the buttons, which are
// what a finger aims at, keep their size as long as it can make up the difference.
typedef struct {
    float scale, space;
} PanelFit;

static inline int panel_w(void) { return ui_px(128); }    // expanded (icons only)
static inline int panel_box(void) { return ui_px(120); }  // the collapsed square
static inline int panel_btn_h(PanelFit f) { return ui_px(100.f * f.scale); }
static inline int panel_gap(PanelFit f) { return ui_px(8.f * f.scale * f.space); }
static inline int panel_div_h(PanelFit f) { return ui_px(20.f * f.scale * f.space); }
static inline int panel_head_h(PanelFit f) { return ui_px(80.f * f.scale); }
static inline int panel_label_h(PanelFit f) { return ui_px(60.f * f.scale); }

// The header, the buttons (pins of them pinned pens) each with the gap under it, the page label,
// the 3 dividers. Must match draw_panel.
static inline int panel_height(PanelFit f, int pins) {
    return panel_head_h(f) + panel_gap(f) + (PANEL_BUTTONS + pins) * (panel_btn_h(f) + panel_gap(f)) +
           panel_label_h(f) + panel_gap(f) + 3 * panel_div_h(f) + ui_px(6);
}

// The fit for a screen screen_h px tall: full size where the panel fits within its margins; else
// the room between the buttons down to half, then everything, each step measured in whole pixels
// as the panel is drawn (the measures round, so a scale that should fit can round back over).
static inline PanelFit panel_fit(int screen_h, int pins) {
    PanelFit f = {1.f, 1.f};
    int room = screen_h - 2 * ui_px(PANEL_MARGIN);
    for (int s = 95; s >= 50 && panel_height(f, pins) > room; s -= 5) f.space = (float)s / 100.f;
    for (int s = 99; s >= 20 && panel_height(f, pins) > room; s--) f.scale = (float)s / 100.f;
    return f;
}

// ---- its pens ----

// The drawing tool the tool button shows (the pencil if settings.ini names the eraser there).
static inline ToolKind panel_button_pen(const Settings *s) {
    ToolKind t = s->draw_tool;
    return (unsigned)t < TOOL_COUNT && t != TOOL_ERASER ? t : TOOL_PENCIL;
}

// Where a tap on the eraser in use goes back to, and what the pen's button draws white with while
// the eraser is in use: the pen used just before it (settings.ini before_eraser) while that is
// still on the panel, a pin or the tool button's; else the tool button's.
static inline ToolKind panel_eraser_back(const Settings *s) {
    int b = s->before_eraser;
    bool shown = b >= 0 && b < TOOL_COUNT && b != TOOL_ERASER &&
                 ((ToolKind)b == panel_button_pen(s) || settings_pinned(s, (ToolKind)b));
    return shown ? (ToolKind)b : panel_button_pen(s);
}

// The pens on the panel, top to bottom, into out; how many. The tool button's, but while that pen
// is pinned (the button shows a grid then, and the pin the pen), then the pinned ones in pin order.
static inline int panel_pens(const Settings *s, ToolKind out[PANEL_PINS_MAX + 1]) {
    int n = 0;
    ToolKind dt = panel_button_pen(s);
    if (!settings_pinned(s, dt)) out[n++] = dt;
    for (int i = 0; i < s->npins && i < PANEL_PINS_MAX; i++) out[n++] = s->pins[i];
    return n;
}

// The next or previous of n pens, round from the last to the first, after the pen in use. The
// eraser is not one of them: from it the step is from the pen it goes back to, so erasing keeps
// the place. A pen in use that is not among them (settings.ini changed by hand) steps to the first
// or the last.
static inline ToolKind panel_step_among(const Settings *s, const ToolKind *pens, int n, bool next) {
    ToolKind cur = s->current_tool == TOOL_ERASER ? panel_eraser_back(s) : s->current_tool;
    int at = -1;
    for (int i = 0; i < n; i++)
        if (pens[i] == cur) at = i;
    if (at < 0) return pens[next ? 0 : n - 1];
    return pens[(at + (next ? 1 : n - 1)) % n];
}

// The tool picker's next or previous pen: its nine in its order (tool_order's first TOOL_PENS),
// which picker-next and picker-previous go through.
static inline ToolKind picker_step(const Settings *s, bool next) {
    return panel_step_among(s, tool_order, TOOL_PENS, next);
}

// Next or previous tool: the pens on the panel alone, so a press never leaves them for the tool
// picker; with fewer than two there, the picker's nine (as picker_step).
static inline ToolKind panel_step(const Settings *s, bool next) {
    ToolKind pens[PANEL_PINS_MAX + 1];
    int n = panel_pens(s, pens);
    return n >= 2 ? panel_step_among(s, pens, n, next) : picker_step(s, next);
}
