// test_ink.c: canvas, paper, brush, page, history.
#include <string.h>

#include "font.h"
#include "history.h"
#include "stb_image_write.h"
#include "testlib.h"

static const float DPI = 300.f;
enum { W = 1860, H = 2480 };

static void out_png(const char *name, const Canvas *c) {
    char dir[512], path[600];
    snprintf(dir, sizeof dir, "%s/../out", tl_tmp());
    mkdir_p(dir);
    snprintf(path, sizeof path, "%s/%s", dir, name);
    stbi_write_png(path, c->w, c->h, 1, c->px, c->stride);
}

TEST(rects) {
    Rect a = {0, 0, 10, 10}, b = {5, 5, 10, 10};
    Rect u = rect_union(a, b), i = rect_intersect(a, b);
    CHECK(u.x == 0 && u.y == 0 && u.w == 15 && u.h == 15);
    CHECK(i.x == 5 && i.y == 5 && i.w == 5 && i.h == 5);
    CHECK(rect_is_empty(rect_intersect(a, (Rect){20, 20, 5, 5})));
    CHECK(rect_union(RECT_EMPTY, b).w == 10);
    CHECK(rect_contains(a, 9, 9) && !rect_contains(a, 10, 9));
}

TEST(canvas_basics) {
    Canvas c;
    CHECK(canvas_init(&c, 100, 80) == 0);
    canvas_set_clip(&c, (Rect){10, 10, 20, 20});
    canvas_fill(&c, (Rect){0, 0, 100, 80}, 0);
    CHECK(c.px[0] == 255 && c.px[15 * 100 + 15] == 0 && c.px[40 * 100 + 40] == 255);
    canvas_reset_clip(&c);
    // deposit only darkens
    c.px[50 * 100 + 50] = 100;
    canvas_deposit_px(&c, 50, 50, 200, 1.f);
    CHECK(c.px[50 * 100 + 50] == 100);
    canvas_deposit_px(&c, 50, 50, 0, 0.5f);
    CHECK(c.px[50 * 100 + 50] == 50);
    // downsample averages
    Canvas d;
    canvas_init(&d, 50, 40);
    canvas_fill(&c, (Rect){0, 0, 100, 80}, 255);
    canvas_fill(&c, (Rect){0, 0, 100, 80}, 255);
    for (int y = 0; y < 80; y++)
        for (int x = 0; x < 100; x++) c.px[y * 100 + x] = (x & 1) ? 0 : 200;
    canvas_downsample(&d, &c);
    CHECK(d.px[0] == 100);
    canvas_free(&d);
    canvas_free(&c);
}

TEST(paper_subregion_matches_full) {
    Canvas full, part;
    canvas_init(&full, W, H);
    canvas_init(&part, W, H);
    for (int k = 0; k < PAPER_COUNT; k++) {
        paper_render(&full, k, (Rect){0, 0, W, H}, DPI);
        canvas_fill(&part, (Rect){0, 0, W, H}, 77);
        Rect tiles[] = {{0, 0, 700, 900}, {700, 0, W - 700, 900}, {0, 900, 333, H - 900}, {333, 900, W - 333, H - 900}};
        for (int i = 0; i < 4; i++) paper_render(&part, k, tiles[i], DPI);
        CHECK(memcmp(full.px, part.px, (size_t)W * H) == 0);
        char name[64];
        snprintf(name, sizeof name, "paper-%d.png", k);
        out_png(name, &full);
    }
    // Pitch for medium rule: find consecutive rule rows in a column away from the margin.
    paper_render(&full, PAPER_MEDIUM, (Rect){0, 0, W, H}, DPI);
    int rows[64], n = 0;
    for (int y = 1; y < H && n < 64; y++)
        if (full.px[y * W + 1000] < 230 && full.px[(y - 1) * W + 1000] >= 230) rows[n++] = y;
    CHECK(n > 20);
    if (n > 3) CHECK_NEAR(rows[3] - rows[2], mm_to_px(7.1f, DPI), 1.0);
    canvas_free(&full);
    canvas_free(&part);
}

TEST(pressure_tilt_mapping) {
    for (int t = 0; t < TOOL_COUNT; t++) {
        ToolParams p;
        tool_params_default(t, &p);
        CHECK(brush_pressure(&p, 0.f) == 0.f);
        CHECK_NEAR(brush_pressure(&p, 1.f), 1.f, 1e-6);
        float prev = -1;
        for (int i = 0; i <= 100; i++) {
            float v = brush_pressure(&p, i / 100.f);
            CHECK(v >= prev);
            prev = v;
        }
        CHECK(brush_tilt(&p, 0, 0) == 0.f);
        CHECK_NEAR(brush_tilt(&p, 89, 0), 1.f, 1e-4);
        int n;
        const ParamInfo *pi = tool_param_table(&n);
        for (int i = 0; i < n; i++) {
            float v = *tool_param_ptr(&p, &pi[i]);
            CHECK(v >= pi[i].min && v <= pi[i].max);
        }
    }
    int n;
    tool_param_table(&n);
    CHECK(n == 13);
}

static Stroke *make_stroke(ToolKind t, float x0, float y0, float x1, float y1, int steps, float p0, float p1,
                           float tilt) {
    ToolParams prm;
    tool_params_default(t, &prm);
    Stroke *s = stroke_new(t, &prm, 1234u + (uint32_t)(x0 * 7 + y0));
    for (int i = 0; i <= steps; i++) {
        float f = (float)i / steps;
        PenSample ps = {lerpf(x0, x1, f), lerpf(y0, y1, f) + 25.f * sinf(f * 9.f), lerpf(p0, p1, f), tilt, 0,
                        (uint32_t)i * 5};
        stroke_append(s, &ps, DPI);
    }
    return s;
}

TEST(live_equals_rerender) {
    Page *pg = page_new(W, H, PAPER_MEDIUM);
    Canvas live, re, chunk;
    canvas_init(&live, W, H);
    canvas_init(&re, W, H);
    canvas_init(&chunk, W, H);
    paper_render(&live, pg->paper, (Rect){0, 0, W, H}, DPI);
    for (int t = 0; t < 3; t++) {
        Stroke *s = make_stroke(t, 200, 400 + t * 300, 1600, 500 + t * 300, 90, 0.1f, 1.f, t * 20.f);
        BrushState b;
        brush_begin(&b, s->tool, &s->params, s->seed, DPI);
        for (int i = 0; i < s->n; i++) brush_add(&b, &live, &s->pts[i]);
        page_add(pg, s);
    }
    page_render(&re, pg, (Rect){0, 0, W, H}, DPI);
    CHECK(memcmp(live.px, re.px, (size_t)W * H) == 0);
    // Region re-render matches too.
    for (int i = 0; i < 6; i++) page_render(&chunk, pg, (Rect){i * 310, 0, 310, H}, DPI);
    CHECK(memcmp(live.px, chunk.px, (size_t)W * H) == 0);
    // Some ink was actually put down.
    int dark = 0;
    for (int i = 0; i < W * H; i++) dark += live.px[i] < 100;
    CHECK(dark > 2000);
    canvas_free(&live);
    canvas_free(&re);
    canvas_free(&chunk);
    page_free(pg);
}

TEST(hit_test) {
    Stroke *s = make_stroke(TOOL_BALLPOINT, 100, 100, 500, 100, 20, 0.5f, 0.5f, 0);
    CHECK(stroke_hit(s, 300, 100 + 25.f * sinf(0.5f * 9.f), 3, DPI));
    CHECK(!stroke_hit(s, 300, 400, 3, DPI));
    CHECK(stroke_hit_segment(s, 300, 0, 300, 300, 2, DPI));
    stroke_free(s);
}

TEST(history_roundtrip) {
    Page *p = page_new(W, H, PAPER_WIDE);
    History h;
    history_init(&h);
    Stroke *a = make_stroke(TOOL_PENCIL, 0, 0, 100, 100, 5, .5f, .5f, 0);
    Stroke *b = make_stroke(TOOL_PENCIL, 0, 200, 100, 300, 5, .5f, .5f, 0);
    Stroke *c = make_stroke(TOOL_PENCIL, 0, 400, 100, 500, 5, .5f, .5f, 0);
    page_add(p, a); history_push_add(&h, 0, a);
    page_add(p, b); history_push_add(&h, 1, b);
    page_add(p, c); history_push_add(&h, 2, c);
    // erase a and c in one gesture
    int idx[2] = {0, 2};
    Stroke *rm[2];
    rm[1] = page_remove(p, 2);
    rm[0] = page_remove(p, 0);
    history_push_erase(&h, 2, idx, rm);
    CHECK(p->n == 1 && p->strokes[0] == b);
    Rect d;
    CHECK(history_undo(&h, p, &d));
    CHECK(p->n == 3 && p->strokes[0] == a && p->strokes[1] == b && p->strokes[2] == c);
    CHECK(history_redo(&h, p, &d));
    CHECK(p->n == 1);
    CHECK(history_undo(&h, p, &d));
    CHECK(history_undo(&h, p, &d));  // undo add c
    CHECK(p->n == 2);
    history_push_paper(&h, PAPER_WIDE, PAPER_MUSIC);  // clears redo (frees c)
    p->paper = PAPER_MUSIC;
    CHECK(!history_can_redo(&h));
    CHECK(history_undo(&h, p, &d) && p->paper == PAPER_WIDE && d.w == W);
    history_push_clear(&h, p);
    CHECK(p->n == 0);
    CHECK(history_undo(&h, p, &d) && p->n == 2);
    // limit trimming
    h.limit = 3;
    for (int i = 0; i < 10; i++) {
        Stroke *s = make_stroke(TOOL_BALLPOINT, 0, 0, 10, 10, 2, .5f, .5f, 0);
        page_add(p, s);
        history_push_add(&h, p->n - 1, s);
    }
    CHECK(h.nundo == 3);
    history_free(&h);
    page_free(p);
}

// A sampler sheet for eyeballing the tools.
TEST(tool_sampler) {
    Page *pg = page_new(W, H, PAPER_MEDIUM);
    float y = 340;
    for (int t = 0; t < 3; t++) {
        for (int k = 0; k < 6; k++) {  // pressure ramps
            float pr = 0.15f + 0.17f * k;
            page_add(pg, make_stroke(t, 330 + k * 250, y, 330 + k * 250 + 200, y + 40, 40, pr, pr, 0));
        }
        page_add(pg, make_stroke(t, 330, y + 170, 1700, y + 170, 160, 0.05f, 1.f, 0));
        for (int k = 0; k < 3; k++)  // tilt 0 / 35 / 65
            page_add(pg, make_stroke(t, 330 + k * 480, y + 330, 330 + k * 480 + 400, y + 330, 60, 0.6f, 0.6f, k * 32.f));
        y += 700;
    }
    Canvas c;
    canvas_init(&c, W, H);
    page_render(&c, pg, (Rect){0, 0, W, H}, DPI);
    if (font_init() == 0)
        for (int t = 0; t < 3; t++) font_draw(&c, FONT_BOLD, 60, 330, 300 + t * 700, tool_name(t), 0);
    out_png("tools.png", &c);
    canvas_free(&c);
    page_free(pg);
}

int main(void) {
    RUN(rects);
    RUN(canvas_basics);
    RUN(paper_subregion_matches_full);
    RUN(pressure_tilt_mapping);
    RUN(live_equals_rerender);
    RUN(hit_test);
    RUN(history_roundtrip);
    RUN(tool_sampler);
    return tl_done();
}
