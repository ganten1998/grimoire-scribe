// test_panel.c: the editor's floating panel (src/panel.h). With 0 to 4 pens pinned it fits the
// 2022/2024 Scribes' screen and the Scribe 3's and Colorsoft's, upright and sideways, keeping its
// 12 px margins and its buttons at full size; the collapsed square never changes.
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

int main(void) {
    ui_init(300.f);
    RUN(fits_both_scribes_both_ways);
    RUN(shrinks_to_fit_a_short_screen);
    return tl_done();
}
