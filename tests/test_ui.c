// test_ui.c: widget logic + render sheets for visual review.
#include <string.h>

#include "keyboard.h"
#include "stb_image_write.h"
#include "testlib.h"

static void out_png(const char *name, const Canvas *c) {
    char dir[512], path[600];
    snprintf(dir, sizeof dir, "%s/../out", tl_tmp());
    mkdir_p(dir);
    snprintf(path, sizeof path, "%s/%s", dir, name);
    stbi_write_png(path, c->w, c->h, 1, c->px, c->stride);
}

TEST(slider_snap) {
    Slider s = {.min = 0.f, .max = 1.f, .step = 0.05f, .value = 0.f};
    ui_slider_layout(&s, (Rect){0, 0, 1000, 100});
    CHECK(ui_slider_set_from_x(&s, (float)s.track.x + s.track.w * 0.512f));
    CHECK_NEAR(s.value, 0.5f, 1e-5);
    CHECK(!ui_slider_set_from_x(&s, (float)s.track.x + s.track.w * 0.51f));
    ui_slider_set_from_x(&s, -100);
    CHECK(s.value == 0.f);
    ui_slider_set_from_x(&s, 5000);
    CHECK(s.value == 1.f);
}

TEST(press_logic) {
    Press p;
    press_begin(&p, 100, 100, 0);
    press_move(&p, 105, 103);
    CHECK(press_is_tap(&p, (Rect){50, 50, 100, 100}));
    press_move(&p, 300, 100);
    CHECK(!press_is_tap(&p, (Rect){50, 50, 400, 100}));
}

TEST(keyboard_typing) {
    Canvas c;
    canvas_init(&c, 1860, 2480);
    Keyboard kb;
    keyboard_open(&kb, "Name", "Hi");
    keyboard_draw(&kb, &c, (Rect){0, 0, 1860, 2480});
    // Find the 'q' key by scanning: it is the first key of row 2. Tap the center of each
    // key via pointer events derived from the drawn layout through key_at semantics.
    Rect d;
    // The 'a' row starts below; easier: type using the internal layout by probing points.
    int typed = 0;
    for (int y = kb.area.y; y < kb.area.y + kb.area.h && !typed; y += 20)
        for (int x = kb.area.x; x < kb.area.x + kb.area.w && !typed; x += 20) {
            keyboard_pointer(&kb, 0, x, y, 0, &d);
            if (kb.pressed_key == 0) {  // '1'
                keyboard_pointer(&kb, 2, x, y, 10, &d);
                typed = 1;
            } else
                keyboard_pointer(&kb, 2, -1, -1, 10, &d);  // release off-key: no input
        }
    CHECK(typed && !strcmp(kb.text, "Hi1"));
    keyboard_draw(&kb, &c, (Rect){0, 0, 1860, 2480});
    out_png("keyboard.png", &c);
    canvas_free(&c);
}

TEST(sheets) {
    Canvas c;
    canvas_init(&c, 1860, 2480);
    int x = 40, y = 40;
    for (int k = 1; k < ICON_COUNT; k++) {
        Rect r = {x, y, 160, 160};
        ui_button(&c, r, (IconKind)k, NULL, (k % 5 == 0) ? BTN_SELECTED : BTN_PLAIN);
        x += 180;
        if (x > 1860 - 180) {
            x = 40;
            y += 180;
        }
    }
    y += 220;
    const char *labels[] = {"Pencil", "Mech", "Ballpoint", "Eraser"};
    IconKind icons[] = {ICON_PENCIL, ICON_MECH_PENCIL, ICON_BALLPOINT, ICON_ERASER};
    for (int i = 0; i < 4; i++) ui_button(&c, (Rect){40 + i * 200, y, 160, 150}, icons[i], labels[i], i == 1 ? BTN_SELECTED : BTN_PLAIN);
    ui_button(&c, (Rect){880, y, 400, 150}, ICON_NONE, "Create", BTN_PRIMARY);
    ui_button(&c, (Rect){1300, y, 400, 150}, ICON_NONE, "Disabled", BTN_DISABLED);
    y += 200;
    Slider s = {.min = 0.25f, .max = 4.f, .step = 0.05f, .value = 1.25f, .label = "Pressure curve", .unit = ""};
    ui_slider_layout(&s, (Rect){40, y, 1780, 110});
    ui_slider_draw(&c, &s, NULL);
    y += 130;
    s.value = 3.f;
    s.label = "Tilt widens";
    s.unit = "x";
    ui_slider_layout(&s, (Rect){40, y, 1780, 110});
    ui_slider_draw(&c, &s, NULL);
    y += 140;
    ui_checkbox(&c, (Rect){40, y, 900, 100}, "Mirror PDFs to Documents", true);
    ui_checkbox(&c, (Rect){940, y, 900, 100}, "Side button erases", false);
    y += 140;
    Rect inner = ui_sheet(&c, (Rect){40, y, 1780, 500}, "New notebook");
    font_draw_left(&c, FONT_REGULAR, ui_text_size(), (Rect){inner.x, inner.y, inner.w, 80},
                   "The quick brown fox jumps over the lazy dog. 0123456789", 0);
    ui_toast(&c, (Rect){0, 0, 1860, 2480}, "PDF saved to grimoire/PDFs");
    out_png("widgets.png", &c);
    canvas_free(&c);
}

int main(void) {
    ui_init(300.f);
    if (font_init() != 0) return 1;
    RUN(slider_snap);
    RUN(press_logic);
    RUN(keyboard_typing);
    RUN(sheets);
    return tl_done();
}
