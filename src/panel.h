// panel.h: the editor's floating panel's measures, as pure logic (unit-tested on the host by
// tests/test_panel.c): its 11 buttons and one more for each pen pinned under the tool button, and
// how they shrink to fit a screen too short for them (the Kindle turned sideways, with pens
// pinned). The panel keeps 12 px free above and below it.
#pragma once
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
