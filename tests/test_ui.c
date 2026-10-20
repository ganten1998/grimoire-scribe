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

// The pin and grid icons (the tool picker's Pin, the panel's tool button when its pen is pinned)
// are in the embedded font. A hollow icon, the pen in use drawing white ink, is the glyph's
// outline: where the glyph is solid it shows the inside colour, around it a ring of the edge
// colour; on a selected (black) button the same in white.
TEST(pin_grid_and_hollow_icons) {
    Canvas c;
    canvas_init(&c, 600, 300);
    canvas_fill(&c, (Rect){0, 0, 600, 300}, 255);
    ui_icon(&c, ICON_PIN, (Rect){0, 0, 100, 100}, 0);
    ui_icon(&c, ICON_GRID, (Rect){100, 0, 100, 100}, 0);
    int dark[2] = {0, 0};
    for (int y = 0; y < 100; y++)
        for (int x = 0; x < 200; x++) dark[x / 100] += c.px[y * c.stride + x] < 128;
    CHECK(dark[0] > 500 && dark[1] > 500);
    // Solid, then hollow, the same icon in the same place: the solid's full pixels are the hollow's
    // inside, and the hollow's ring reaches past the solid's edge.
    Rect r = {0, 100, 120, 120};
    canvas_fill(&c, (Rect){0, 100, 600, 200}, 255);
    ui_icon(&c, ICON_PENCIL, r, 0);
    ui_icon_hollow(&c, ICON_PENCIL, (Rect){r.x + 200, r.y, r.w, r.h}, 0, 255);
    int inside = 0, inside_white = 0, ring = 0;
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) {
            uint8_t s = c.px[y * c.stride + x], h = c.px[y * c.stride + x + 200];
            if (s < 8) inside++, inside_white += h > 247;  // (a glyph blends to within a level)
            if (s == 255 && h < 128) ring++;
        }
    CHECK(inside > 300 && inside_white == inside && ring > inside / 4);
    // On a selected button: the white glyph's solid pixels are black, inside a white ring.
    Rect b = {330, 100, 120, 120};
    ui_button(&c, b, ICON_PENCIL, NULL, BTN_SELECTED);
    ui_button_hollow(&c, (Rect){b.x + 140, b.y, b.w, b.h}, ICON_PENCIL, BTN_SELECTED);
    int core = 0, core_black = 0, white_ring = 0;
    Rect in = rect_inflate(b, -22);  // inside the frame and its carved line
    for (int y = in.y; y < in.y + in.h; y++)
        for (int x = in.x; x < in.x + in.w; x++) {
            uint8_t s = c.px[y * c.stride + x], h = c.px[y * c.stride + x + 140];
            if (s == 255) core++, core_black += h < 8;
            if (s < 8 && h > 128) white_ring++;
        }
    CHECK(core > 300 && core_black == core && white_ring > core / 4);
    out_png("hollow.png", &c);
    canvas_free(&c);
}

int main(void) {
    ui_init(300.f);
    if (font_init() != 0) return 1;
    RUN(pin_grid_and_hollow_icons);
    RUN(slider_snap);
    RUN(press_logic);
    RUN(keyboard_typing);
    RUN(sheets);
    return tl_done();
}
