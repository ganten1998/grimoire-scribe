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

TEST(slider_fine_drag) {
    Slider s = {.min = 0, .max = 1, .step = 0.01f, .value = 0.28f};
    ui_slider_layout(&s, (Rect){0,0,1000,100});
    SliderDrag d;
    float x = s.track.x + s.track.w * 0.28f, y = s.track.y + s.track.h * 0.5f;
    ui_slider_drag_begin(&s, &d, x, 100);
    CHECK(!ui_slider_drag_to(&s, &d, x, y + 160, 500));
    CHECK_NEAR(s.value, 0.28f, 1e-6);  // moving away alone never jumps
    x += s.track.w * 0.02f * 16.f;
    CHECK(ui_slider_drag_to(&s, &d, x, y + 160, 510));
    CHECK_NEAR(s.value, 0.30f, 1e-6);
    for (int i = 0; i < 10; i++) {
        x += s.track.w * 0.001f * 16.f;
        ui_slider_drag_to(&s, &d, x, y + 160, 520u + (uint32_t)i);
    }
    CHECK_NEAR(s.value, 0.31f, 1e-6);  // fractional motion accumulates rather than rounding away
    CHECK(ui_slider_drag_to(&s, &d, x, y + 10, 600));
    CHECK_NEAR(s.value, 0.76f, 1e-6);  // entering the knob-height band catches up to the pointer
    x = s.track.x + s.track.w * 0.4f;
    ui_slider_drag_to(&s, &d, x, y - 10, 610);
    CHECK_NEAR(s.value, 0.4f, 1e-6);  // staying in the band gives direct, large adjustments
    CHECK(!ui_slider_drag_to(&s, &d, x, y - 160, 620));
    CHECK_NEAR(s.value, 0.4f, 1e-6);  // leaving again alone does not change the value
    x += s.track.w * 0.02f * 16.f;
    ui_slider_drag_to(&s, &d, x, y - 160, 630);
    CHECK_NEAR(s.value, 0.42f, 1e-6);  // fine motion resumes above as well as below
    ui_slider_drag_to(&s, &d, x + 10000, y - 160, 640);
    CHECK_NEAR(s.value, 1.f, 1e-6);
    ui_slider_drag_to(&s, &d, x + 10000 - s.track.w * 0.1f * 16.f, y - 160, 650);
    CHECK_NEAR(s.value, 0.9f, 1e-6);
    // A fresh press restores ordinary absolute tracking; holding is specific to each gesture.
    ui_slider_drag_begin(&s, &d, x, 1000);
    ui_slider_drag_to(&s, &d, s.track.x + s.track.w * 0.2f, y, 1100);
    CHECK_NEAR(s.value, 0.2f, 1e-6);
    CHECK(!d.held);
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
    // (and the Pages view's Copy page)
    ui_icon(&c, ICON_COPY, (Rect){200, 0, 100, 100}, 0);
    int dark[3] = {0, 0, 0};
    for (int y = 0; y < 100; y++)
        for (int x = 0; x < 300; x++) dark[x / 100] += c.px[y * c.stride + x] < 128;
    CHECK(dark[0] > 500 && dark[1] > 500 && dark[2] > 500);
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

// A name keeps every letter in the files; the screens leave out what the fonts cannot draw (it
// would show as a box), and a name left with no letter reads Untitled, with what is left.
TEST(name_labels) {
    char buf[200];
    const char *plain = "Le Petit Prince", *greek = "\xce\xb1\xce\xb2 \xd0\x92\xd0\xbe\xd0\xb9\xd0\xbd\xd0\xb0";
    CHECK(font_drawable_copy(plain, buf, sizeof buf) == 0 && !strcmp(buf, plain));
    CHECK(ui_name_label(plain, buf, sizeof buf) == plain && ui_name_label(greek, buf, sizeof buf) == greek);
    CHECK(ui_name_label("1984", buf, sizeof buf)[0] == '1');  // (all of it drawn: as it is)
    // Japanese, in the middle and alone; Vietnamese marks Terminess lacks (e with circumflex and
    // acute, e with circumflex and dot below).
    CHECK(font_drawable_copy("Pok\xc3\xa9mon \xe6\xbc\xab\xe7\x94\xbb Vol. 3", buf, sizeof buf) == 2);
    CHECK(!strcmp(buf, "Pok\xc3\xa9mon Vol. 3"));
    CHECK(!strcmp(ui_name_label("Pok\xc3\xa9mon \xe6\xbc\xab\xe7\x94\xbb Vol. 3", buf, sizeof buf), "Pok\xc3\xa9mon Vol. 3"));
    const char *ja = "\xe5\x90\xbe\xe8\xbc\xa9\xe3\x81\xaf\xe7\x8c\xab\xe3\x81\xa7\xe3\x81\x82\xe3\x82\x8b";
    CHECK(font_drawable_copy(ja, buf, sizeof buf) == 7 && !buf[0]);
    CHECK(!strcmp(ui_name_label(ja, buf, sizeof buf), "Untitled"));
    char numbered[64];
    snprintf(numbered, sizeof numbered, "%s 2", ja);
    CHECK(!strcmp(ui_name_label(numbered, buf, sizeof buf), "Untitled 2"));
    CHECK(!strcmp(ui_name_label("Ti\xe1\xba\xbfng Vi\xe1\xbb\x87t", buf, sizeof buf), "Ting Vit"));
    // Cut to the buffer between two characters.
    char small[6];
    CHECK(font_drawable_copy("ab\xe2\x82\xac\xe2\x82\xac", small, sizeof small) == 0 && !strcmp(small, "ab\xe2\x82\xac"));
}

int main(void) {
    ui_init(300.f);
    if (font_init() != 0) return 1;
    RUN(pin_grid_and_hollow_icons);
    RUN(slider_snap);
    RUN(slider_fine_drag);
    RUN(press_logic);
    RUN(keyboard_typing);
    RUN(sheets);
    RUN(name_labels);
    return tl_done();
}
