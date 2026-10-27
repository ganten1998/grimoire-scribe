// test_panel.c: the editor's floating panel (src/panel.h). With 0 to 4 pens pinned it fits the
// 2022/2024 Scribes' screen and the Scribe 3's and Colorsoft's, upright and sideways, keeping its
// 12 px margins and its buttons at full size; the collapsed square never changes. Next and previous
// tool go through its pens alone, and the tool picker's next and previous through the picker's nine.
#include "panel.h"
#include "store.h"
#include "testlib.h"

TEST(fits_both_scribes_both_ways) {
    const int screens[][2] = {{1860, 2480}, {1986, 2648}};
    for (int s = 0; s < 2; s++)
        for (int turned = 0; turned < 2; turned++) {
            int w = turned ? screens[s][1] : screens[s][0], h = turned ? screens[s][0] : screens[s][1];
            for (int pins = 0; pins <= PANEL_PINS_MAX; pins++) {
                PanelFit f = panel_fit(h, pins);
                int ph = panel_height(f, pins);
                // Inside the screen with 12 px free above and below, as wide as ever.
                CHECK(ph + 2 * ui_px(PANEL_MARGIN) <= h && panel_w() <= w);
                // The buttons stay as big as without pins (and sideways as upright): only the
                // room between them gives.
                CHECK(panel_btn_h(f) == ui_px(100) && panel_head_h(f) == ui_px(80) && panel_label_h(f) == ui_px(60));
                CHECK(f.scale == 1.f && f.space >= 0.5f);
                if (pins == 0) CHECK(f.space == 1.f && ph == ui_px(1410));  // as before pins existed
            }
        }
    // The only one that needs the room between the buttons: four pins on the 2022/2024 Scribe
    // sideways (1842 px at full size, 1836 px of room).
    PanelFit f = panel_fit(1860, 4);
    CHECK(panel_height((PanelFit){1.f, 1.f}, 4) == ui_px(1842) && f.space < 1.f && f.scale == 1.f);
    CHECK(panel_height(f, 4) <= 1860 - 24 && panel_gap(f) >= ui_px(4) && panel_div_h(f) >= ui_px(10));
    CHECK(panel_fit(1860, 3).space == 1.f);
    CHECK(panel_box() == ui_px(120));  // collapsed: the same square, pins or not
}

// A screen far shorter than any Scribe's: the room between the buttons goes down to half, then
// everything shrinks, and the panel still keeps its margins (the measures round to whole pixels).
TEST(shrinks_to_fit_a_short_screen) {
    for (int h = 900; h <= 1900; h += 37)
        for (int pins = 0; pins <= PANEL_PINS_MAX; pins++) {
            PanelFit f = panel_fit(h, pins);
            CHECK(panel_height(f, pins) + 2 * ui_px(PANEL_MARGIN) <= h);
            CHECK(f.scale == 1.f || f.space == 0.5f);  // the buttons give only after the room
            CHECK(panel_btn_h(f) >= ui_px(40));
        }
}

// Pressed again and again from s's pen in use, each step taken as the editor takes it (the pen it
// gives is the one in use): the n pens it goes through, into out.
static void walk(Settings s, ToolKind (*step)(const Settings *, bool), bool next, ToolKind *out, int n) {
    for (int i = 0; i < n; i++) s.current_tool = out[i] = step(&s, next);
}

// Two pens or more on the panel: next and previous tool go through them alone, top to bottom (the
// tool button's, then the pins in pin order) and round again; the eraser keeps the place of the pen
// it goes back to. Fewer than two: the tool picker's nine, as before pins existed.
TEST(next_tool_walks_the_panels_pens) {
    Settings s;
    settings_default(&s);
    s.draw_tool = TOOL_FOUNTAIN;
    s.pins[0] = TOOL_HIGHLIGHTER;
    s.pins[1] = TOOL_FINELINER;
    s.npins = 2;
    s.current_tool = TOOL_FOUNTAIN;
    s.before_eraser = -1;
    ToolKind pens[PANEL_PINS_MAX + 1], got[6];
    CHECK(panel_pens(&s, pens) == 3 && pens[0] == TOOL_FOUNTAIN && pens[1] == TOOL_HIGHLIGHTER && pens[2] == TOOL_FINELINER);
    walk(s, panel_step, true, got, 6);
    CHECK(got[0] == TOOL_HIGHLIGHTER && got[1] == TOOL_FINELINER && got[2] == TOOL_FOUNTAIN);
    CHECK(got[3] == TOOL_HIGHLIGHTER && got[4] == TOOL_FINELINER && got[5] == TOOL_FOUNTAIN);
    walk(s, panel_step, false, got, 3);
    CHECK(got[0] == TOOL_FINELINER && got[1] == TOOL_HIGHLIGHTER && got[2] == TOOL_FOUNTAIN);
    // From the eraser: the pen it goes back to (a pin), or the tool button's once that is off the panel.
    s.current_tool = TOOL_ERASER;
    s.before_eraser = TOOL_HIGHLIGHTER;
    CHECK(panel_step(&s, true) == TOOL_FINELINER && panel_step(&s, false) == TOOL_FOUNTAIN);
    s.before_eraser = TOOL_MARKER;
    CHECK(panel_step(&s, true) == TOOL_HIGHLIGHTER && panel_step(&s, false) == TOOL_FINELINER);
    // The tool button's pen pinned (the button shows a grid): the pins alone, its pin in its place.
    s.draw_tool = TOOL_FINELINER;
    s.current_tool = TOOL_FINELINER;
    CHECK(panel_pens(&s, pens) == 2 && pens[0] == TOOL_HIGHLIGHTER && pens[1] == TOOL_FINELINER);
    CHECK(panel_step(&s, true) == TOOL_HIGHLIGHTER && panel_step(&s, false) == TOOL_HIGHLIGHTER);
    // A pen in use that is not on the panel (settings.ini changed by hand): the first, or the last.
    s.current_tool = TOOL_CHARCOAL;
    CHECK(panel_step(&s, true) == TOOL_HIGHLIGHTER && panel_step(&s, false) == TOOL_FINELINER);
    // One pen on the panel: the picker's nine (pencil, mechanical, charcoal, ballpoint, fineliner,
    // marker, fountain, calligraphy, highlighter), round from the highlighter to the pencil.
    s.npins = 1;  // (the highlighter unpinned: the fineliner's pin alone, the tool button a grid)
    s.pins[0] = TOOL_FINELINER;
    s.current_tool = TOOL_FINELINER;
    CHECK(panel_pens(&s, pens) == 1 && pens[0] == TOOL_FINELINER);
    ToolKind nine[TOOL_PENS];
    walk(s, panel_step, true, nine, TOOL_PENS);
    for (int i = 0; i < TOOL_PENS; i++) CHECK(nine[i] == tool_order[(5 + i) % TOOL_PENS]);
    s.npins = 0;
    s.current_tool = TOOL_PENCIL;
    CHECK(panel_step(&s, false) == TOOL_HIGHLIGHTER && panel_step(&s, true) == TOOL_MECH_PENCIL);
    s.current_tool = TOOL_ERASER;
    s.before_eraser = TOOL_MARKER;  // (off the panel: from the tool button's fineliner)
    CHECK(panel_step(&s, true) == TOOL_MARKER && panel_step(&s, false) == TOOL_BALLPOINT);
}

// The picker's next and previous: its nine in its order whatever the panel holds, round again, and
// from the eraser the pen it goes back to.
TEST(picker_step_walks_the_nine) {
    Settings s;
    settings_default(&s);
    s.draw_tool = TOOL_FOUNTAIN;
    s.pins[0] = TOOL_HIGHLIGHTER;
    s.pins[1] = TOOL_FINELINER;
    s.npins = 2;
    s.current_tool = TOOL_FOUNTAIN;
    s.before_eraser = -1;
    ToolKind nine[TOOL_PENS];
    walk(s, picker_step, true, nine, TOOL_PENS);
    for (int i = 0; i < TOOL_PENS; i++) CHECK(nine[i] == tool_order[(7 + i) % TOOL_PENS]);
    walk(s, picker_step, false, nine, TOOL_PENS);
    for (int i = 0; i < TOOL_PENS; i++) CHECK(nine[i] == tool_order[(5 - i + TOOL_PENS) % TOOL_PENS]);
    s.current_tool = TOOL_ERASER;
    s.before_eraser = TOOL_FINELINER;
    CHECK(picker_step(&s, true) == TOOL_MARKER && picker_step(&s, false) == TOOL_BALLPOINT);
}

int main(void) {
    ui_init(300.f);
    RUN(fits_both_scribes_both_ways);
    RUN(shrinks_to_fit_a_short_screen);
    RUN(next_tool_walks_the_panels_pens);
    RUN(picker_step_walks_the_nine);
    return tl_done();
}
