// test_erase.c: erase strokes, the white-ink engine (docs/FORMAT.md, Erase strokes). White ink and
// the precise eraser restore the paper under them: live drawing equals every re-render, the
// eraser's core lands exactly on the paper with its lines, ink drawn later is untouched, the fast
// path is the per-pixel path byte for byte, compaction drops only what changes nothing, and
// 1.0.0-beta still opens every page with erase strokes.
#include <string.h>
#include <unistd.h>

#include "beta_format.h"
#include "font.h"
#include "history.h"
#include "store.h"
#include "testlib.h"

static const float DPI = 300.f;
enum { W = 1860, H = 2480 };
static const Rect FULL = {0, 0, W, H};

// Profiles used to record the 1.1 erase renders. A stroke stores these values; current pen
// defaults may change, but these profiles and the recorded CRCs must stay frozen.
static const ToolParams k_legacy_params[TOOL_COUNT] = {
    {0.20f, 0.45f, 0.30f, 0.95f, 0.03f, 0.85f, 1.25f, 15.f, 60.f, 1.6f, 0.45f, 0.65f, 0.10f},
    {0.26f, 0.36f, 0.38f, 0.85f, 0.03f, 0.80f, 1.10f, 25.f, 70.f, 0.35f, 0.10f, 0.50f, 0.15f},
    {0.30f, 0.48f, 0.85f, 1.00f, 0.02f, 0.55f, 0.60f, 30.f, 75.f, 0.10f, 0.00f, 0.06f, 0.35f},
    {2.5f, 7.0f, 1.f, 1.f, 0.0f, 0.60f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.f},
    {4.0f, 5.0f, 0.55f, 0.65f, 0.02f, 0.50f, 0.80f, 30.f, 70.f, 0.2f, 0.f, 0.f, 0.40f},
    {0.35f, 1.10f, 0.90f, 1.00f, 0.02f, 0.60f, 0.80f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.30f},
    {0.28f, 0.34f, 1.00f, 1.00f, 0.01f, 0.30f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.30f},
    {1.50f, 2.00f, 0.92f, 1.00f, 0.01f, 0.40f, 0.70f, 30.f, 75.f, 0.3f, 0.f, 0.04f, 0.30f},
    {0.80f, 2.40f, 0.30f, 1.00f, 0.03f, 0.85f, 1.00f, 45.f, 75.f, 2.0f, 0.35f, 0.85f, 0.15f},
    {0.70f, 1.40f, 0.92f, 1.00f, 0.02f, 0.60f, 0.80f, 10.f, 25.f, 0.f, 0.f, 0.f, 0.30f},
};

// ---- building strokes ----

// Fills s with a wavy line from (x0, y0) to (x1, y1): pressure p0..p1, tilt (tx, ty) wobbling.
static void wave_points(Stroke *s, float x0, float y0, float x1, float y1, float amp, float p0, float p1, float tx,
                        float ty) {
    int steps = (int)(sqrtf((x1 - x0) * (x1 - x0) + (y1 - y0) * (y1 - y0)) / 12.f) + 8;
    for (int i = 0; i <= steps; i++) {
        float f = (float)i / (float)steps;
        PenSample q = {lerpf(x0, x1, f), lerpf(y0, y1, f) + amp * sinf(f * 9.f + (float)(s->seed & 7)),
                       lerpf(p0, p1, f), tx + 6.f * sinf(f * 11.f), ty - 5.f * cosf(f * 7.f), (uint32_t)i * 5u};
        stroke_append(s, &q, DPI);
    }
}

static Stroke *ink(ToolKind t, float x0, float y0, float x1, float y1, float amp, float p0, float p1, float tx, float ty,
                   uint32_t seed) {
    ToolParams prm;
    tool_params_default(t, &prm);
    Stroke *s = stroke_new(t, &prm, seed);
    wave_points(s, x0, y0, x1, y1, amp, p0, p1, tx, ty);
    return s;
}

// An erase stroke with a texture (TOOL_ERASER: the round tip) and that tool's default settings.
static Stroke *white(ToolKind texture, float x0, float y0, float x1, float y1, float amp, float p0, float p1, float tx,
                     float ty, uint32_t hash) {
    ToolParams prm;
    tool_params_default(texture, &prm);
    Stroke *s = stroke_new(TOOL_ERASER, &prm, erase_seed(texture, hash));
    wave_points(s, x0, y0, x1, y1, amp, p0, p1, tx, ty);
    return s;
}

// The round eraser along a straight line, upright, at pressure p (1 = its 7 mm).
static Stroke *rubber(float x0, float y0, float x1, float y1, float p, uint32_t hash) {
    ToolParams prm;
    tool_params_default(TOOL_ERASER, &prm);
    Stroke *s = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_ERASER, hash));
    int steps = (int)(sqrtf((x1 - x0) * (x1 - x0) + (y1 - y0) * (y1 - y0)) / 10.f) + 1;
    for (int i = 0; i <= steps; i++) {
        float f = (float)i / (float)steps;
        PenSample q = {lerpf(x0, x1, f), lerpf(y0, y1, f), p, 0, 0, (uint32_t)i * 5u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// Draws s the way the editor does: a new stroke, then stroke_append and brush_add sample by
// sample over c, with the page's paper beside it (NULL = white). Returns the new stroke.
static Stroke *draw_live(Canvas *c, const Canvas *paper, const Stroke *s, bool *restored) {
    Stroke *n = stroke_new(s->tool, &s->params, s->seed);
    BrushState b;
    stroke_brush_begin(&b, n, DPI, paper ? paper->px : NULL, 0, 0, paper ? paper->stride : 0);
    for (int k = 0; k < s->n; k++) {
        stroke_append(n, &s->pts[k], DPI);
        brush_add(&b, c, &s->pts[k]);
    }
    if (restored) *restored = b.restored;
    return n;
}

static long diff_px(const Canvas *a, const Canvas *b, Rect r) {
    long n = 0;
    r = rect_intersect(r, (Rect){0, 0, a->w, a->h});
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) n += a->px[y * a->stride + x] != b->px[y * b->stride + x];
    return n;
}

static float seg_dist(float px, float py, float ax, float ay, float bx, float by) {
    float dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy;
    float t = l2 > 0.f ? clampf(((px - ax) * dx + (py - ay) * dy) / l2, 0.f, 1.f) : 0.f;
    float ex = px - (ax + dx * t), ey = py - (ay + dy * t);
    return sqrtf(ex * ex + ey * ey);
}

// Distance from a pixel's centre to a stroke's polyline.
static float poly_dist(const Stroke *s, int x, int y) {
    float m = 1e9f;
    for (int k = 0; k + 1 < s->n; k++)
        m = fminf(m, seg_dist(x + 0.5f, y + 0.5f, s->pts[k].x, s->pts[k].y, s->pts[k + 1].x, s->pts[k + 1].y));
    return m;
}

// The same for a straight stroke (rubber): the segment from its first point to its last.
static float line_dist(const Stroke *s, int x, int y) {
    return seg_dist(x + 0.5f, y + 0.5f, s->pts[0].x, s->pts[0].y, s->pts[s->n - 1].x, s->pts[s->n - 1].y);
}

// The round eraser at full pressure: its core (full coverage) reaches r_in from each dab; a pixel
// this close to its line is that close to a dab too (dabs are 6 px apart, the last one at most a
// spacing short of the end).
static float rubber_core(void) {
    float r = mm_to_px(7.f, DPI) * 0.5f;
    return r * (1.f - 0.08f) - 0.5f - 8.f;
}

// A view of p without the strokes marked in drop: it borrows p's strokes (free it with view_free).
static Page *page_without(const Page *p, const bool *drop) {
    Page *v = page_new(p->w, p->h, p->paper);
    v->strokes = malloc((size_t)(p->n ? p->n : 1) * sizeof *v->strokes);
    for (int i = 0; i < p->n; i++)
        if (!drop[i]) v->strokes[v->n++] = p->strokes[i];
    v->cap = v->n;
    return v;
}

// Frees a page_without view (its strokes belong to the page it was made from).
static void view_free(Page *v) {
    free(v->strokes);
    v->strokes = NULL;
    v->n = v->cap = 0;
    page_free(v);
}

// A page that has it all, drawn live over `live` (the paper beside it in `paper`): ink with every
// tool, white with every texture and the round tip at several pressures, a soft eraser (no fast
// path), strokes running off all four edges, ink drawn after the erasing, then more erasing.
static Page *busy_page(PaperKind k, Canvas *live, Canvas *paper) {
    Page *pg = page_new(W, H, k);
    paper_render(paper, k, FULL, DPI);
    canvas_blit(live, paper, FULL, 0, 0);
    Stroke *tpl[64];
    int n = 0;
    uint32_t seed = 1;
    for (int t = 0; t < TOOL_COUNT; t++)
        if (t != TOOL_ERASER)
            tpl[n++] = ink((ToolKind)t, -40, 140 + t * 250, 1900, 160 + t * 250, 70, 0.1f, 1.f, 35, 30, seed++);
    for (int t = 0; t < TOOL_COUNT; t++)  // every texture (TOOL_ERASER: the round tip), off the top and bottom
        tpl[n++] = white((ToolKind)t, 90 + t * 175, -40, 150 + t * 175, 2520, 40, 0.2f, 1.f, 40, 46, seed++);
    tpl[n++] = rubber(-60, 900, 1920, 1010, 0.f, seed++);  // the thinnest round tip, off both sides
    tpl[n++] = rubber(200, 1500, 1700, 2300, 0.5f, seed++);
    Stroke *soft = white(TOOL_ERASER, 300, 300, 1500, 2100, 300, 0.3f, 1.f, 0, 0, seed++);
    soft->params.opacity_min = soft->params.opacity_max = 0.5f;  // half strength: no fast path
    tpl[n++] = soft;
    for (int t = 0; t < TOOL_COUNT; t++)
        if (t != TOOL_ERASER)
            tpl[n++] = ink((ToolKind)t, -30, 220 + t * 230, 1890, 300 + t * 230, 250, 0.2f, 0.9f, -30, 40, seed++);
    tpl[n++] = white(TOOL_CALLIGRAPHY, -50, 1500, 1910, 1600, 900, 0.4f, 1.f, 40, 46, seed++);
    tpl[n++] = white(TOOL_CHARCOAL, -50, 2000, 1910, 1700, 300, 0.5f, 1.f, 75, 10, seed++);  // on its side
    tpl[n++] = rubber(-80, 2470, 1940, 2490, 1.f, seed++);  // off the bottom edge
    tpl[n++] = rubber(930, -60, 940, 320, 1.f, seed++);     // off the top edge
    for (int i = 0; i < n; i++) {
        page_add(pg, draw_live(live, paper, tpl[i], NULL));
        stroke_free(tpl[i]);
    }
    return pg;
}

// ---- tests ----

TEST(restore_blend) {
    Canvas c;
    CHECK(canvas_init(&c, 64, 64) == 0);
    uint8_t *p = &c.px[10 * 64 + 10];
    *p = 200;
    canvas_restore_px(&c, 10, 10, 150, 1.f);  // already lighter than the paper: kept
    CHECK(*p == 200);
    *p = 100;
    canvas_restore_px(&c, 10, 10, 200, 0.5f);
    CHECK(*p == 150);
    canvas_restore_px(&c, 10, 10, 200, 1.f);
    CHECK(*p == 200);
    *p = 254;
    canvas_restore_px(&c, 10, 10, 255, 0.4f);  // a partial share stops a level short...
    CHECK(*p == 254);
    canvas_restore_px(&c, 10, 10, 255, 0.999f);  // ...full coverage lands on the paper
    CHECK(*p == 255);
    *p = 37;
    canvas_restore_px(&c, 10, 10, 180, 0.9995f);
    CHECK(*p == 180);
    // Never past the paper, never darker, whatever the alpha.
    srand(3);
    for (int i = 0; i < 20000; i++) {
        uint8_t v = (uint8_t)(rand() & 255), paper = (uint8_t)(rand() & 255);
        float a = (float)(rand() % 1500) / 1000.f;
        *p = v;
        canvas_restore_px(&c, 10, 10, paper, a);
        CHECK(*p >= v && (*p <= paper || *p == v));
    }
    // The clip holds.
    canvas_set_clip(&c, (Rect){20, 20, 10, 10});
    *p = 0;
    canvas_restore_px(&c, 10, 10, 255, 1.f);
    CHECK(*p == 0);
    canvas_free(&c);
}

// Live drawing over the editor's page canvas, with the paper beside it, gives exactly what every
// re-render gives: the full page, column tiles, 131x97 tiles and random rects in any order; the
// ink layer too, in full and in tiles; and no pixel ends up lighter than the paper.
TEST(live_equals_rerender) {
    static const PaperKind kinds[2] = {PAPER_MEDIUM, PAPER_GRAND};
    for (int ki = 0; ki < 2; ki++) {
        Canvas live, paper, re, part;
        canvas_init(&live, W, H);
        canvas_init(&paper, W, H);
        canvas_init(&re, W, H);
        canvas_init(&part, W, H);
        Page *pg = busy_page(kinds[ki], &live, &paper);
        page_render(&re, pg, FULL, DPI);
        CHECK(diff_px(&live, &re, FULL) == 0);
        canvas_fill(&part, FULL, 77);
        for (int i = 0; i < 6; i++) page_render(&part, pg, (Rect){i * 310, 0, 310, H}, DPI);
        CHECK(diff_px(&live, &part, FULL) == 0);
        if (ki == 0) {  // (small tiles replay every long stroke over and over: one paper is enough)
            canvas_fill(&part, FULL, 77);
            for (int y = 0; y < H; y += 97)
                for (int x = 0; x < W; x += 131) page_render(&part, pg, (Rect){x, y, 131, 97}, DPI);
            CHECK(diff_px(&live, &part, FULL) == 0);
        }
        // Random overlapping rects in a random order, as undo and the stroke eraser redraw.
        canvas_blit(&part, &re, FULL, 0, 0);
        srand(7 + ki);
        for (int i = 0; i < (ki ? 100 : 300); i++)
            page_render(&part, pg, (Rect){rand() % W - 50, rand() % H - 50, 1 + rand() % 400, 1 + rand() % 400}, DPI);
        CHECK(diff_px(&live, &part, FULL) == 0);
        long lighter = 0, erased = 0;
        for (int i = 0; i < W * H; i++) lighter += re.px[i] > paper.px[i];
        CHECK(lighter == 0);
        // The ink layer (?layer=ink): erase strokes restore toward white there, full or in tiles.
        canvas_fill(&re, FULL, 255);
        page_render_layers(&re, pg, FULL, DPI, PAGE_LAYER_INK);
        canvas_fill(&part, FULL, 255);
        for (int i = 0; i < 6; i++) page_render_layers(&part, pg, (Rect){i * 310, 0, 310, H}, DPI, PAGE_LAYER_INK);
        CHECK(diff_px(&re, &part, FULL) == 0);
        // The erasing did something: without the erase strokes, much more ink.
        bool *drop = calloc((size_t)pg->n, sizeof *drop);
        for (int i = 0; i < pg->n; i++) drop[i] = stroke_is_erase(pg->strokes[i]);
        Page *inkonly = page_without(pg, drop);
        canvas_fill(&part, FULL, 255);
        page_render_layers(&part, inkonly, FULL, DPI, PAGE_LAYER_INK);
        for (int i = 0; i < W * H; i++) erased += part.px[i] < 128 && re.px[i] >= 250;
        CHECK(erased > 10000);
        view_free(inkonly);
        free(drop);
        canvas_free(&live);
        canvas_free(&paper);
        canvas_free(&re);
        canvas_free(&part);
        page_free(pg);
    }
}

// The fast path (a round dab's core restored a row at a time) is the per-pixel path, byte for
// byte: single dabs of every size up to the largest at 300 dpi and past the fast path's limit, at
// any centre, over any content (pixels lighter than their paper too) and any clip, toward white or
// a random paper; and whole pages.
TEST(span_fast_path_is_the_per_pixel_path) {
    enum { N = 1000 };
    Canvas a, b, paper;
    canvas_init(&a, N, N);
    canvas_init(&b, N, N);
    canvas_init(&paper, N, N);
    srand(11);
    for (int i = 0; i < N * N; i++) paper.px[i] = (uint8_t)(rand() & 255);
    static const float dpis[3] = {300.f, 433.f, 600.f};
    static const ToolKind round[3] = {TOOL_ERASER, TOOL_FINELINER, TOOL_BALLPOINT};
    int eligible = 0;
    long bad = 0;
    for (int it = 0; it < 1500; it++) {
        if (it % 8 == 0)  // fresh content: random gray blocks on both canvases
            for (int k = 0; k < 12; k++) {
                Rect r = {rand() % N - 100, rand() % N - 100, 50 + rand() % 400, 50 + rand() % 400};
                uint8_t g = (uint8_t)(rand() & 255);
                canvas_fill(&a, r, g);
                canvas_fill(&b, r, g);
            }
        Rect clip = {0, 0, N, N};
        if (it % 3 == 0) clip = (Rect){rand() % 600, rand() % 600, 50 + rand() % 500, 50 + rand() % 500};
        canvas_set_clip(&a, clip);
        canvas_set_clip(&b, clip);
        float dpi = dpis[rand() % 3];
        ToolKind t = round[rand() % 3];
        ToolParams prm;
        tool_params_default(t, &prm);
        prm.size_min_mm = prm.size_max_mm = 0.1f + (float)(rand() % 1190) / 100.f;
        prm.tilt_size = (float)(rand() % 5);
        prm.opacity_min = prm.opacity_max = 1.f;
        prm.grain = 0.f;
        PenSample s = {(float)(rand() % 1300) - 150.f + (float)(rand() % 1000) / 1000.f,
                       (float)(rand() % 1300) - 150.f + (float)(rand() % 1000) / 1000.f, (float)(rand() % 1001) / 1000.f,
                       (float)(rand() % 179) - 89.f, (float)(rand() % 179) - 89.f, 0};
        const uint8_t *pp = rand() % 4 ? paper.px : NULL;
        BrushState ba, bb;
        brush_begin(&ba, t, &prm, (uint32_t)it, dpi);
        brush_set_erase(&ba, pp, 0, 0, N);
        bb = ba;
        g_brush_erase_spans = true;
        Rect da = brush_add(&ba, &a, &s);
        g_brush_erase_spans = false;
        Rect db = brush_add(&bb, &b, &s);
        g_brush_erase_spans = true;
        CHECK(da.x == db.x && da.y == db.y && da.w == db.w && da.h == db.h);
        CHECK(ba.restored == bb.restored);
        long d = diff_px(&a, &b, da);
        bad += d;
        if (d) fprintf(stderr, "  dab %d (dpi %.0f, tool %d, %.2f mm): %ld px differ\n", it, dpi, t, prm.size_max_mm, d);
        float r, op;
        brush_dab(t, &prm, &s, ba.dpi, &r, &op);
        float r_in = r * (1.f - (t == TOOL_ERASER ? 0.08f : t == TOOL_FINELINER ? 0.08f : 0.10f)) - 0.5f;
        eligible += r_in > 2.5f && r_in <= 512.f && !rect_is_empty(da);
    }
    CHECK(bad == 0);
    CHECK(eligible > 600);  // most dabs took the fast path
    canvas_free(&a);
    canvas_free(&b);
    canvas_free(&paper);
    // Whole pages, and their ink layers.
    Canvas live, pap, on, off;
    canvas_init(&live, W, H);
    canvas_init(&pap, W, H);
    canvas_init(&on, W, H);
    canvas_init(&off, W, H);
    Page *pg = busy_page(PAPER_DOTS, &live, &pap);
    page_render(&on, pg, FULL, DPI);
    g_brush_erase_spans = false;
    page_render(&off, pg, FULL, DPI);
    CHECK(diff_px(&on, &off, FULL) == 0);
    canvas_fill(&off, FULL, 255);
    page_render_layers(&off, pg, FULL, DPI, PAGE_LAYER_INK);
    g_brush_erase_spans = true;
    canvas_fill(&on, FULL, 255);
    page_render_layers(&on, pg, FULL, DPI, PAGE_LAYER_INK);
    CHECK(diff_px(&on, &off, FULL) == 0);
    canvas_free(&live);
    canvas_free(&pap);
    canvas_free(&on);
    canvas_free(&off);
    page_free(pg);
}

// The round eraser's core goes back exactly to the paper, keeping every line of it, on ruled,
// music, graph, dotted, grand staff and custom music paper (ruled once more without the fast
// path); and erasing never makes a pixel lighter than its paper.
TEST(eraser_core_returns_to_paper) {
    MusicLayout m = {.systems = 3, .n = 4, .part = {PART_TREBLE, PART_BASS, PART_TAB6, PART_LYRICS}};
    const PaperKind kinds[7] = {PAPER_MEDIUM, PAPER_MUSIC, PAPER_GRID,   PAPER_DOTS,
                                PAPER_GRAND,  music_encode(&m), PAPER_MEDIUM};
    const float lim = rubber_core();
    Canvas re, before, paper;
    canvas_init(&re, W, H);
    canvas_init(&before, W, H);
    canvas_init(&paper, W, H);
    for (int ki = 0; ki < 7; ki++) {
        g_brush_erase_spans = ki < 6;
        Page *pg = page_new(W, H, kinds[ki]);
        for (int i = 0; i < 27; i++) {
            ToolKind t = (ToolKind)(i % 9 >= TOOL_ERASER ? i % 9 + 1 : i % 9);
            page_add(pg, ink(t, 100, 700 + i * 40.f, 1760, 760 + i * 40.f, 30, 0.4f, 1.f, 20, 20, 1u + (uint32_t)i));
        }
        page_render(&before, pg, FULL, DPI);
        Stroke *er = rubber(300, 600, 1500, 1900, 1.f, 77u);
        page_add(pg, er);
        page_render(&re, pg, FULL, DPI);
        paper_render(&paper, kinds[ki], FULL, DPI);
        long core = 0, off = 0, inked = 0, lines = 0, kept = 0, lighter = 0;
        Rect bb = rect_intersect(er->bbox, FULL);
        for (int y = bb.y; y < bb.y + bb.h; y++)
            for (int x = bb.x; x < bb.x + bb.w; x++) {
                if (line_dist(er, x, y) > lim) continue;
                int i = y * W + x;
                core++;
                off += re.px[i] != paper.px[i];
                inked += before.px[i] < paper.px[i];
                if (paper.px[i] < 255) {
                    lines++;
                    kept += re.px[i] == paper.px[i];
                }
            }
        for (int i = 0; i < W * H; i++) lighter += re.px[i] > paper.px[i];
        CHECK(core > 80000 && inked > 20000);  // the eraser had plenty to erase
        CHECK(off == 0);
        CHECK(lines > 20 && kept == lines);
        CHECK(lighter == 0);
        if (off || kept != lines || lighter)
            fprintf(stderr, "  paper %d: core %ld px, %ld off; lines %ld px, %ld kept; %ld lighter\n", ki, core, off,
                    lines, kept, lighter);
        page_free(pg);
    }
    g_brush_erase_spans = true;
    canvas_free(&re);
    canvas_free(&before);
    canvas_free(&paper);
}

// Ink drawn after an erase stroke is untouched by it: inside the eraser's core it is exactly the
// same ink drawn alone on fresh paper.
TEST(ink_after_erasing_is_untouched) {
    Page *pg = page_new(W, H, PAPER_MEDIUM), *later = page_new(W, H, PAPER_MEDIUM);
    for (int i = 0; i < 8; i++)
        page_add(pg, ink(TOOL_MARKER, 500, 400 + i * 220.f, 1300, 420 + i * 220.f, 60, 1.f, 1.f, 0, 0, 3u + (uint32_t)i));
    Stroke *er = rubber(900, 300, 905, 2200, 1.f, 9u);
    page_add(pg, er);
    for (int t = 0, row = 0; t < TOOL_COUNT; t++) {
        if (t == TOOL_ERASER) continue;
        float y = 380 + row * 220.f;
        page_add(pg, ink((ToolKind)t, 650, y, 1150, y + 140, 50, 0.3f, 1.f, 30, 30, 50u + (uint32_t)t));
        page_add(later, ink((ToolKind)t, 650, y, 1150, y + 140, 50, 0.3f, 1.f, 30, 30, 50u + (uint32_t)t));
        row++;
    }
    Canvas a, b;
    canvas_init(&a, W, H);
    canvas_init(&b, W, H);
    page_render(&a, pg, FULL, DPI);
    page_render(&b, later, FULL, DPI);
    const float lim = rubber_core();
    long core = 0, differ = 0, dark = 0;
    Rect bb = rect_intersect(er->bbox, FULL);
    for (int y = bb.y; y < bb.y + bb.h; y++)
        for (int x = bb.x; x < bb.x + bb.w; x++) {
            if (line_dist(er, x, y) > lim) continue;
            core++;
            differ += a.px[y * W + x] != b.px[y * W + x];
            dark += b.px[y * W + x] < 100;
        }
    CHECK(core > 50000 && dark > 1500 && differ == 0);
    canvas_free(&a);
    canvas_free(&b);
    page_free(pg);
    page_free(later);
}

// Every white texture renders the same every time, live or replayed, and hashes with the seed's
// low 24 bits (its top byte is the texture). The grainy ones leave the paper's tooth; the round tip
// leaves nothing.
TEST(white_textures_are_deterministic) {
    enum { PW = 900, PH = 700 };
    Canvas paper, a, b, live;
    canvas_init(&paper, PW, PH);
    canvas_init(&a, PW, PH);
    canvas_init(&b, PW, PH);
    canvas_init(&live, PW, PH);
    for (int t = 0; t < TOOL_COUNT; t++) {
        Page *pg = page_new(PW, PH, PAPER_MEDIUM);
        for (int i = 0; i < 6; i++)
            page_add(pg, ink(TOOL_MARKER, 60, 200 + i * 50.f, 840, 210 + i * 50.f, 10, 1.f, 1.f, 0, 0, 1u + (uint32_t)i));
        paper_render(&paper, PAPER_MEDIUM, (Rect){0, 0, PW, PH}, DPI);
        canvas_blit(&live, &paper, (Rect){0, 0, PW, PH}, 0, 0);
        for (int i = 0; i < pg->n; i++) stroke_free(draw_live(&live, &paper, pg->strokes[i], NULL));
        Stroke *w = white((ToolKind)t, 100, 330, 800, 420, 60, 0.6f, 1.f, 30, 25, 0x00C0FFEEu + (uint32_t)t);
        CHECK(stroke_render_tool(w) == (ToolKind)t);
        CHECK(w->seed >> 24 == (t == TOOL_ERASER ? 0u : (uint32_t)t + 1u));
        bool restored = false;
        stroke_free(draw_live(&live, &paper, w, &restored));
        CHECK(restored);
        page_add(pg, w);
        page_render(&a, pg, (Rect){0, 0, PW, PH}, DPI);
        canvas_fill(&b, (Rect){0, 0, PW, PH}, 0);
        page_render(&b, pg, (Rect){0, 0, PW, PH}, DPI);
        CHECK(diff_px(&a, &b, (Rect){0, 0, PW, PH}) == 0);
        CHECK(diff_px(&a, &live, (Rect){0, 0, PW, PH}) == 0);
        // Tooth: marker pixels near the white stroke's middle that stay well below the paper.
        long tooth = 0;
        for (int y = 340; y < 410; y++)
            for (int x = 300; x < 600; x++)
                if (poly_dist(w, x, y) < 2.f) tooth += a.px[y * PW + x] + 40 < paper.px[y * PW + x];
        bool grainy = t == TOOL_PENCIL || t == TOOL_MECH_PENCIL || t == TOOL_CHARCOAL;
        CHECK(grainy ? tooth > 0 : t != TOOL_ERASER || tooth == 0);
        page_free(pg);
    }
    // The brush hashes the low 24 bits: the same as a charcoal brush begun with them, by hand.
    ToolParams prm;
    tool_params_default(TOOL_CHARCOAL, &prm);
    Stroke *s = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_CHARCOAL, 0xAB123456u));
    CHECK(s->seed == ((uint32_t)(TOOL_CHARCOAL + 1) << 24 | 0x123456u));
    wave_points(s, 100, 300, 800, 400, 80, 0.5f, 1.f, 60, 20);
    canvas_fill(&a, (Rect){0, 0, PW, PH}, 0);
    canvas_fill(&b, (Rect){0, 0, PW, PH}, 0);
    stroke_free(draw_live(&a, NULL, s, NULL));
    BrushState hand;
    brush_begin(&hand, TOOL_CHARCOAL, &prm, 0x123456u, DPI);
    brush_set_erase(&hand, NULL, 0, 0, 0);
    for (int k = 0; k < s->n; k++) brush_add(&hand, &b, &s->pts[k]);
    CHECK(diff_px(&a, &b, (Rect){0, 0, PW, PH}) == 0);
    // Other low bits: other grain and specks.
    Stroke *s2 = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_CHARCOAL, 0x654321u));
    wave_points(s2, 100, 300, 800, 400, 80, 0.5f, 1.f, 60, 20);
    canvas_fill(&b, (Rect){0, 0, PW, PH}, 0);
    stroke_free(draw_live(&b, NULL, s2, NULL));
    CHECK(diff_px(&a, &b, (Rect){0, 0, PW, PH}) > 100);
    stroke_free(s);
    stroke_free(s2);
    canvas_free(&paper);
    canvas_free(&a);
    canvas_free(&b);
    canvas_free(&live);
}

// A texture byte that is not a drawing tool (4, the eraser itself, or a tool this version lacks)
// draws the round tip; erase_seed writes the bytes FORMAT.md gives.
TEST(other_texture_bytes_draw_the_round_tip) {
    CHECK(erase_seed(TOOL_ERASER, 0xFF123456u) == 0x00123456u);
    CHECK(erase_seed(TOOL_PENCIL, 7u) == 0x01000007u);
    CHECK(erase_seed(TOOL_CALLIGRAPHY, 7u) == 0x0A000007u);
    CHECK(erase_seed((ToolKind)TOOL_COUNT, 7u) == 7u && erase_seed((ToolKind)200, 7u) == 7u);
    ToolParams prm;
    tool_params_default(TOOL_ERASER, &prm);
    Stroke *ink_s = stroke_new(TOOL_PENCIL, &prm, 0x05000000u);
    CHECK(stroke_render_tool(ink_s) == TOOL_PENCIL && !stroke_is_erase(ink_s));  // ink keeps its seed whole
    stroke_free(ink_s);
    static const uint32_t tops[5] = {0, 4, 11, 200, 255};
    Canvas ref, c;
    canvas_init(&ref, 600, 400);
    canvas_init(&c, 600, 400);
    for (int i = 0; i < 5; i++) {
        Stroke *s = stroke_new(TOOL_ERASER, &prm, tops[i] << 24 | 0x00BEEFu);
        CHECK(stroke_render_tool(s) == TOOL_ERASER);
        wave_points(s, 50, 200, 550, 220, 60, 0.f, 1.f, 0, 0);
        Canvas *dst = i ? &c : &ref;
        canvas_fill(dst, (Rect){0, 0, 600, 400}, 0);
        stroke_free(draw_live(dst, NULL, s, NULL));
        if (i) CHECK(diff_px(&ref, &c, (Rect){0, 0, 600, 400}) == 0);
        stroke_free(s);
    }
    canvas_free(&ref);
    canvas_free(&c);
}

// Every texture stays inside its stroke's bbox (what region re-renders and the paper copy rely
// on), with its roughest grain and laid on its side: the bbox is padded by the texture's reach.
TEST(erase_inside_bbox) {
    for (int t = 0; t < TOOL_COUNT; t++)
        for (int side = 0; side < 2; side++) {
            ToolParams prm;
            tool_params_default((ToolKind)t, &prm);
            prm.grain = side ? 1.f : prm.grain;
            Stroke *s = stroke_new(TOOL_ERASER, &prm, erase_seed((ToolKind)t, 4242u + (uint32_t)t));
            for (int i = 0; i <= 60; i++) {
                float f = (float)i / 60.f;
                float tx = side ? 80.f : 30.f, ty = side ? 10.f : 30.f;
                PenSample q = {300.f + f * 1200.f, 700.f + 200.f * sinf(f * 5.f), 1.f, tx, ty, (uint32_t)i * 5};
                stroke_append(s, &q, DPI);
            }
            Canvas c;
            canvas_init(&c, W, H);
            canvas_fill(&c, FULL, 0);
            stroke_free(draw_live(&c, NULL, s, NULL));
            long touched = 0, outside = 0;
            for (int y = 0; y < H; y++)
                for (int x = 0; x < W; x++)
                    if (c.px[y * W + x] > 0) {
                        touched++;
                        outside += !rect_contains(s->bbox, x, y);
                    }
            CHECK(touched > 0 && outside == 0);
            if (outside) fprintf(stderr, "  texture %d side %d: %ld px outside the bbox\n", t, side, outside);
            canvas_free(&c);
            stroke_free(s);
        }
}

// A stroke erased whole leaves an all-white ink layer and the bare paper, and compaction drops it
// with its eraser.
TEST(fully_erased_stroke_leaves_nothing) {
    Page *pg = page_new(W, H, PAPER_GRAND);
    Stroke *s = ink(TOOL_FINELINER, 300, 1200, 1500, 1260, 40, 0.5f, 1.f, 0, 0, 21u);
    page_add(pg, s);
    // The round eraser along the same points, from 60 px before the first to 60 px past the last.
    ToolParams prm;
    tool_params_default(TOOL_ERASER, &prm);
    Stroke *e = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_ERASER, 22u));
    PenSample q = s->pts[0];
    q.x -= 60.f;
    q.pressure = 1.f;
    stroke_append(e, &q, DPI);
    for (int k = 0; k < s->n; k++) {
        q = s->pts[k];
        q.pressure = 1.f;
        stroke_append(e, &q, DPI);
    }
    q.x += 60.f;
    stroke_append(e, &q, DPI);
    page_add(pg, e);
    Canvas c, paper;
    canvas_init(&c, W, H);
    canvas_init(&paper, W, H);
    page_render_layers(&c, pg, FULL, DPI, PAGE_LAYER_INK);
    long ink_left = 0;
    for (int i = 0; i < W * H; i++) ink_left += c.px[i] != 255;
    CHECK(ink_left == 0);
    page_render(&c, pg, FULL, DPI);
    paper_render(&paper, PAPER_GRAND, FULL, DPI);
    CHECK(diff_px(&c, &paper, FULL) == 0);
    bool drop[2];
    CHECK(page_droppable(pg, DPI, drop) == 2 && drop[0] && drop[1]);
    canvas_free(&c);
    canvas_free(&paper);
    page_free(pg);
}

// A reworked page: words erased whole, half, by two erasers together, a white pencil over a word,
// erasers on bare paper, and words written after the erasing.
enum { RW_A, RW_EA, RW_B, RW_EB, RW_C, RW_EC1, RW_EC2, RW_D, RW_ED, RW_E, RW_WP, RW_NA, RW_NB, RW_F, RW_G, RW_H, RW_COUNT };
static Page *reworked_page(PaperKind k) {
    Page *pg = page_new(W, H, k);
    Stroke *s[RW_COUNT];
    s[RW_A] = ink(TOOL_PENCIL, 200, 400, 500, 400, 12, 0.6f, 0.9f, 0, 0, 1u);
    s[RW_EA] = rubber(170, 400, 530, 400, 1.f, 2u);
    s[RW_B] = ink(TOOL_BALLPOINT, 200, 600, 500, 600, 12, 0.6f, 0.9f, 0, 0, 3u);
    s[RW_EB] = rubber(170, 600, 530, 600, 1.f, 4u);
    s[RW_C] = ink(TOOL_HIGHLIGHTER, 200, 800, 500, 800, 0, 0.7f, 0.7f, 0, 0, 5u);  // too wide for one eraser
    s[RW_EC1] = rubber(150, 780, 550, 780, 1.f, 6u);
    s[RW_EC2] = rubber(150, 820, 550, 820, 1.f, 7u);
    s[RW_D] = ink(TOOL_FOUNTAIN, 200, 1000, 500, 1000, 12, 0.6f, 0.9f, 0, 0, 8u);
    s[RW_ED] = rubber(170, 1000, 350, 1000, 1.f, 9u);  // half of it
    s[RW_E] = ink(TOOL_CHARCOAL, 200, 1200, 500, 1200, 12, 0.8f, 1.f, 0, 0, 10u);
    s[RW_WP] = white(TOOL_PENCIL, 180, 1200, 520, 1200, 10, 0.9f, 0.9f, 20, 10, 11u);  // leaves tooth
    s[RW_NA] = rubber(900, 1400, 1500, 1450, 1.f, 12u);                                // bare paper
    s[RW_NB] = white(TOOL_CHARCOAL, 900, 1600, 1500, 1650, 30, 0.8f, 1.f, 30, 30, 13u);
    s[RW_F] = ink(TOOL_MARKER, 220, 400, 480, 410, 10, 0.8f, 0.8f, 0, 0, 14u);  // over the erased word A
    s[RW_G] = ink(TOOL_PENCIL, 900, 2000, 1500, 2000, 10, 0.f, 0.f, 0, 0, 15u);  // below the deadzone, no eraser near
    s[RW_H] = ink(TOOL_PENCIL, 950, 1420, 1450, 1430, 10, 0.f, 0.f, 0, 0, 16u);  // the same, over the earlier RW_NA
    for (int i = 0; i < RW_COUNT; i++) page_add(pg, s[i]);
    return pg;
}

// A staff line of music paper (drawn into scratch) at x = 1000: the centre of its rows, which are
// darker than the highlighter's ink, and how many rows thick it is.
static float staff_line(Canvas *scratch, int *thickness) {
    paper_render(scratch, PAPER_MUSIC, FULL, DPI);
    int y0 = 0, lt = 0;
    for (int y = 0; y < H && !lt; y++)
        if (scratch->px[y * W + 1000] <= 110)
            for (y0 = y; scratch->px[(y0 + lt) * W + 1000] <= 110;) lt++;
    *thickness = lt;
    return (float)y0 + (float)lt * 0.5f;
}

// Compaction lists exactly what can go: the full render and the ink layer of the page without
// those strokes are byte for byte the page's own, and the page is left as it was.
TEST(compaction_drops_only_what_changes_nothing) {
    static const PaperKind kinds[2] = {PAPER_MEDIUM, PAPER_GRAND};
    Canvas a, b;
    canvas_init(&a, W, H);
    canvas_init(&b, W, H);
    for (int ki = 0; ki < 2; ki++) {
        Page *pg = reworked_page(kinds[ki]);
        Stroke *before[RW_COUNT];
        Rect boxes[RW_COUNT];
        for (int i = 0; i < RW_COUNT; i++) {
            before[i] = pg->strokes[i];
            boxes[i] = pg->strokes[i]->bbox;
        }
        bool drop[RW_COUNT];
        int n = page_droppable(pg, DPI, drop);
        CHECK(n > 0);
        int marked = 0;
        for (int i = 0; i < RW_COUNT; i++) marked += drop[i];
        CHECK(marked == n);
        // The page is untouched.
        CHECK(pg->n == RW_COUNT);
        for (int i = 0; i < RW_COUNT && i < pg->n; i++) {
            CHECK(pg->strokes[i] == before[i]);
            CHECK(!memcmp(&pg->strokes[i]->bbox, &boxes[i], sizeof boxes[i]));
        }
        Page *v = page_without(pg, drop);
        page_render(&a, pg, FULL, DPI);
        page_render(&b, v, FULL, DPI);
        CHECK(diff_px(&a, &b, FULL) == 0);
        canvas_fill(&a, FULL, 255);
        canvas_fill(&b, FULL, 255);
        page_render_layers(&a, pg, FULL, DPI, PAGE_LAYER_INK);
        page_render_layers(&b, v, FULL, DPI, PAGE_LAYER_INK);
        CHECK(diff_px(&a, &b, FULL) == 0);
        view_free(v);
        if (kinds[ki] == PAPER_MEDIUM) {
            // Gone: the words erased whole and their erasers, the erasers on bare paper. Kept: what
            // still shows, and ink no later eraser reaches even when it draws nothing (not erasing's
            // to drop), also over an earlier eraser.
            static const int gone[] = {RW_A, RW_EA, RW_B, RW_EB, RW_C, RW_EC1, RW_EC2, RW_NA, RW_NB};
            static const int kept[] = {RW_D, RW_ED, RW_E, RW_WP, RW_F, RW_G, RW_H};
            for (size_t i = 0; i < sizeof gone / sizeof gone[0]; i++) CHECK(drop[gone[i]]);
            for (size_t i = 0; i < sizeof kept / sizeof kept[0]; i++) CHECK(!drop[kept[i]]);
            CHECK(n == (int)(sizeof gone / sizeof gone[0]));
        }
        page_free(pg);
    }
    // A thin highlighter lying on a staff line darker than its ink draws nothing on the page, but
    // the ink layer shows it: it stays, though the page alone would let it go. An eraser just
    // below makes it a candidate (their boxes meet, their pixels do not).
    {
        Page *mp = page_new(W, H, PAPER_MUSIC);
        int lt;
        float yc = staff_line(&a, &lt);
        CHECK(lt >= 2 && lt <= 4);
        ToolParams hp;
        tool_params_default(TOOL_HIGHLIGHTER, &hp);
        hp.size_min_mm = hp.size_max_mm = 0.1f;
        Stroke *hl = stroke_new(TOOL_HIGHLIGHTER, &hp, 31u);
        for (int i = 0; i <= 50; i++) {
            PenSample q = {700.f + (float)i * 10.f, yc, 0.7f, 0, 0, (uint32_t)i * 5u};
            stroke_append(hl, &q, DPI);
        }
        page_add(mp, hl);
        page_add(mp, rubber(650, yc + 46.f, 1250, yc + 46.f, 1.f, 32u));
        CHECK(rect_overlaps(mp->strokes[0]->bbox, mp->strokes[1]->bbox));
        bool none[2] = {false, false}, first[2] = {true, false}, md[2];
        Page *with = page_without(mp, none), *without = page_without(mp, first);
        page_render(&a, with, FULL, DPI);
        page_render(&b, without, FULL, DPI);
        CHECK(diff_px(&a, &b, FULL) == 0);  // nothing on the page...
        canvas_fill(&a, FULL, 255);
        canvas_fill(&b, FULL, 255);
        page_render_layers(&a, with, FULL, DPI, PAGE_LAYER_INK);
        page_render_layers(&b, without, FULL, DPI, PAGE_LAYER_INK);
        CHECK(diff_px(&a, &b, FULL) > 500);  // ...but a line on the ink layer
        CHECK(page_droppable(mp, DPI, md) >= 0 && !md[0]);
        Page *v = page_without(mp, md);
        view_free(with);
        canvas_fill(&b, FULL, 255);
        page_render_layers(&b, v, FULL, DPI, PAGE_LAYER_INK);
        CHECK(diff_px(&a, &b, FULL) == 0);
        view_free(v);
        view_free(without);
        page_free(mp);
    }
    // A page without erase strokes has nothing to drop (and is never rendered for it).
    Page *plain = page_new(W, H, PAPER_MEDIUM);
    page_add(plain, ink(TOOL_PENCIL, 100, 100, 300, 100, 0, 0.f, 0.f, 0, 0, 1u));  // below the deadzone: draws nothing
    page_add(plain, ink(TOOL_PENCIL, 100, 100, 300, 100, 0, 0.8f, 0.8f, 0, 0, 2u));
    bool d2[2] = {true, true};
    CHECK(page_droppable(plain, DPI, d2) == 0 && !d2[0] && !d2[1]);
    page_free(plain);
    canvas_free(&a);
    canvas_free(&b);
}

// The compaction rule the slow way: in stroke order, ink first, then erase strokes, a stroke on the
// page is marked when the page and its ink layer, drawn whole without it and the strokes marked
// before it, are what they are without those alone. Only ink a later erase stroke's bbox reaches
// is a candidate.
static int droppable_slowly(const Page *p, bool *drop) {
    int n = 0;
    bool any = false;
    for (int i = 0; i < p->n; i++) {
        drop[i] = false;
        any = any || stroke_is_erase(p->strokes[i]);
    }
    if (!any) return 0;
    const Rect all = {0, 0, p->w, p->h};
    Canvas with[2], without[2];  // the page and its ink layer, without the marked strokes / and stroke i
    for (int l = 0; l < 2; l++) {
        canvas_init(&with[l], p->w, p->h);
        canvas_init(&without[l], p->w, p->h);
    }
    page_render(&with[0], p, all, DPI);
    page_render_layers(&with[1], p, all, DPI, PAGE_LAYER_INK);
    for (int pass = 0; pass < 2; pass++)
        for (int i = 0; i < p->n; i++) {
            const Stroke *s = p->strokes[i];
            if ((unsigned)s->tool >= TOOL_COUNT || stroke_is_erase(s) != (pass == 1)) continue;
            bool later = false;
            for (int j = i + 1; j < p->n; j++)
                later = later || (stroke_is_erase(p->strokes[j]) && rect_overlaps(p->strokes[j]->bbox, s->bbox));
            if ((pass == 0 && !later) || rect_is_empty(rect_intersect(s->bbox, all))) continue;
            drop[i] = true;
            Page *v = page_without(p, drop);
            page_render(&without[0], v, all, DPI);
            canvas_fill(&without[1], all, 255);
            page_render_layers(&without[1], v, all, DPI, PAGE_LAYER_INK);
            view_free(v);
            drop[i] = diff_px(&with[0], &without[0], all) == 0 && diff_px(&with[1], &without[1], all) == 0;
            n += drop[i];
        }
    for (int l = 0; l < 2; l++) {
        canvas_free(&with[l]);
        canvas_free(&without[l]);
    }
    return n;
}

static uint32_t g_rand = 1;
static uint32_t rnd(void) {
    g_rand ^= g_rand << 13;
    g_rand ^= g_rand >> 17;
    g_rand ^= g_rand << 5;
    return g_rand;
}
static float frand(float lo, float hi) { return lo + (hi - lo) * (float)(rnd() % 10001) / 10000.f; }

// A random stroke over (and off) a w x h page: ink of any tool, or an erase stroke of any texture
// (soft, grainy and laid on its side at times), wandering from a random start.
static Stroke *random_stroke(int w, int h, bool erase) {
    ToolKind t = (ToolKind)(rnd() % TOOL_COUNT);
    if (!erase && t == TOOL_ERASER) t = TOOL_MARKER;
    ToolParams prm;
    tool_params_default(t, &prm);
    if (rnd() % 3 == 0) prm.size_min_mm = prm.size_max_mm = frand(0.1f, 8.f);
    if (rnd() % 5 == 0) prm.opacity_min = prm.opacity_max = frand(0.2f, 1.f);
    if (rnd() % 5 == 0) prm.grain = frand(0.f, 1.f);
    Stroke *s = stroke_new(erase ? TOOL_ERASER : t, &prm, erase ? erase_seed(t, rnd()) : rnd() & 0xFFFFFFu);
    float x = frand(-60.f, (float)w + 60.f), y = frand(-60.f, (float)h + 60.f), ang = frand(0.f, 6.28f);
    float step = frand(3.f, 30.f), pr = frand(0.f, 1.f), tx = frand(-70.f, 70.f), ty = frand(-70.f, 70.f);
    int n = 1 + (int)(rnd() % (rnd() % 4 ? 40 : 160));
    for (int k = 0; k < n; k++) {
        PenSample q = {x, y, pr, tx, ty, (uint32_t)k * 6u};
        stroke_append(s, &q, DPI);
        ang += frand(-0.7f, 0.7f);
        x += cosf(ang) * step;
        y += sinf(ang) * step;
        pr = clampf(pr + frand(-0.1f, 0.1f), 0.f, 1.f);
    }
    return s;
}

// Compaction marks exactly what the rule done the slow way marks (so the page without the marked
// strokes renders as before), on random pages of ink and erase strokes of every kind, on blank,
// ruled, dotted, music and graph paper. The fast way draws only the pieces of strokes near what it
// checks, and the ink layer only where the paper has marks: this is what says that changes nothing.
TEST(compaction_is_the_rule) {
    static const PaperKind kinds[5] = {PAPER_BLANK, PAPER_MEDIUM, PAPER_DOTS, PAPER_MUSIC, PAPER_GRID};
    long marked = 0;
    for (int it = 0; it < 16; it++) {
        g_rand = 0x9E3779B9u + (uint32_t)it * 2654435761u;
        int w = 250 + (int)(rnd() % 300), h = 250 + (int)(rnd() % 400);
        Page *p = page_new(w, h, kinds[it % 5]);
        for (int round = 0, rounds = 1 + (int)(rnd() % 3); round < rounds; round++) {
            for (int i = 0, ni = 3 + (int)(rnd() % 12); i < ni; i++) page_add(p, random_stroke(w, h, false));
            for (int i = 0, ne = 1 + (int)(rnd() % 6); i < ne; i++) page_add(p, random_stroke(w, h, true));
        }
        bool *fast = calloc((size_t)p->n, sizeof *fast), *slow = calloc((size_t)p->n, sizeof *slow);
        int nf = page_droppable(p, DPI, fast), ns = droppable_slowly(p, slow);
        CHECK(nf == ns && !memcmp(fast, slow, (size_t)p->n * sizeof *fast));
        if (nf != ns) fprintf(stderr, "  page %d: %d marked, the slow way %d\n", it, nf, ns);
        marked += nf;
        free(fast);
        free(slow);
        page_free(p);
    }
    CHECK(marked > 20);  // the pages gave it plenty to mark
}

// Ink the page shows and the ink layer does not: a light pencil line along a staff line, under a
// highlighter at full strength. On the ink layer the highlighter buries the pencil (both settle on
// the same gray), but on the page the staff line is darker than the highlighter's ink, which leaves
// it alone, so there the pencil still darkens the line. An eraser after them reaches into the
// pencil's box without touching it, which makes the pencil a candidate: only the page's check
// keeps it.
TEST(compaction_checks_the_page_too) {
    Canvas a, b;
    canvas_init(&a, W, H);
    canvas_init(&b, W, H);
    int lt;
    float yc = staff_line(&a, &lt);
    CHECK(lt >= 2 && lt <= 4);
    // This contrast needs the original light pencil, not a later darker writing profile.
    ToolParams pp = k_legacy_params[TOOL_PENCIL], hp = k_legacy_params[TOOL_HIGHLIGHTER];
    hp.opacity_min = hp.opacity_max = 1.f;
    Stroke *pencil = stroke_new(TOOL_PENCIL, &pp, 41u), *hl = stroke_new(TOOL_HIGHLIGHTER, &hp, 42u);
    for (int i = 0; i <= 50; i++) {
        PenSample q = {700.f + (float)i * 10.f, yc, 0.3f, 0, 0, (uint32_t)i * 5u};
        stroke_append(pencil, &q, DPI);
    }
    // The narrow vertical tip needs repeated passes to bury the light pencil completely.
    for (int pass = 0; pass < 3; pass++) for (int i = 0; i <= 80; i++) {
        PenSample q = {600.f + (float)i * 10.f, yc, 1.f, 0, 0, (uint32_t)i * 5u};
        stroke_append(hl, &q, DPI);
    }
    Page *mp = page_new(W, H, PAPER_MUSIC);
    page_add(mp, pencil);
    page_add(mp, hl);
    page_add(mp, rubber(1250, yc, 1500, yc, 1.f, 43u));
    CHECK(rect_overlaps(mp->strokes[0]->bbox, mp->strokes[2]->bbox));
    // Without the pencil: another page, the same ink layer.
    bool none[3] = {false, false, false}, first[3] = {true, false, false}, md[3];
    Page *with = page_without(mp, none), *without = page_without(mp, first);
    page_render(&a, with, FULL, DPI);
    page_render(&b, without, FULL, DPI);
    CHECK(diff_px(&a, &b, FULL) > 20);
    canvas_fill(&a, FULL, 255);
    canvas_fill(&b, FULL, 255);
    page_render_layers(&a, with, FULL, DPI, PAGE_LAYER_INK);
    page_render_layers(&b, without, FULL, DPI, PAGE_LAYER_INK);
    CHECK(diff_px(&a, &b, FULL) == 0);
    CHECK(page_droppable(mp, DPI, md) == 0 && !md[0] && !md[1] && !md[2]);
    view_free(with);
    view_free(without);
    page_free(mp);
    canvas_free(&a);
    canvas_free(&b);
}

// BrushState.restored tells whether a stroke changed the canvas it drew on, the page. An eraser
// over ink the paper hides there (a thin highlighter on a staff line darker than its ink) reports
// no change, yet it changes the ink layer: page_droppable, which checks both, keeps it. It, not
// `restored`, decides what a page may leave out.
TEST(restored_is_the_page_only) {
    Canvas paper, live, a, b;
    canvas_init(&paper, W, H);
    canvas_init(&live, W, H);
    canvas_init(&a, W, H);
    canvas_init(&b, W, H);
    int lt;
    float yc = staff_line(&paper, &lt);
    CHECK(lt >= 2 && lt <= 4);
    canvas_blit(&live, &paper, FULL, 0, 0);
    ToolParams hp, ep;
    tool_params_default(TOOL_HIGHLIGHTER, &hp);
    hp.size_min_mm = hp.size_max_mm = 0.1f;
    tool_params_default(TOOL_ERASER, &ep);
    ep.size_min_mm = ep.size_max_mm = 0.1f;  // the precise eraser at its smallest, along the same points
    Stroke *hl = stroke_new(TOOL_HIGHLIGHTER, &hp, 51u);
    Stroke *er = stroke_new(TOOL_ERASER, &ep, erase_seed(TOOL_ERASER, 52u));
    for (int i = 0; i <= 50; i++) {
        PenSample q = {700.f + (float)i * 10.f, yc, 0.7f, 0, 0, (uint32_t)i * 5u};
        stroke_append(hl, &q, DPI);
        stroke_append(er, &q, DPI);
    }
    Page *mp = page_new(W, H, PAPER_MUSIC);
    bool restored = true, md[2], no_eraser[2] = {false, true};
    page_add(mp, draw_live(&live, &paper, hl, NULL));
    page_add(mp, draw_live(&live, &paper, er, &restored));
    CHECK(!restored);
    CHECK(diff_px(&live, &paper, FULL) == 0);  // nothing changed on the page...
    canvas_fill(&a, FULL, 255);
    page_render_layers(&a, mp, FULL, DPI, PAGE_LAYER_INK);
    Page *v = page_without(mp, no_eraser);
    canvas_fill(&b, FULL, 255);
    page_render_layers(&b, v, FULL, DPI, PAGE_LAYER_INK);
    CHECK(diff_px(&a, &b, FULL) > 100);  // ...but the eraser changed the ink layer
    CHECK(page_droppable(mp, DPI, md) == 0 && !md[0] && !md[1]);
    view_free(v);
    page_free(mp);
    stroke_free(hl);
    stroke_free(er);
    canvas_free(&paper);
    canvas_free(&live);
    canvas_free(&a);
    canvas_free(&b);
}

// The page and its ink layer must be drawn in one call: the ink layer alone restores toward white,
// so drawing it over the paper in a second pass wipes the lines under every erase stroke.
TEST(one_call_keeps_the_lines) {
    Page *pg = page_new(W, H, PAPER_MEDIUM);
    for (int i = 0; i < 5; i++)
        page_add(pg, ink(TOOL_BALLPOINT, 200, 500 + i * 90.f, 1600, 520 + i * 90.f, 30, 0.8f, 0.8f, 0, 0, 1u + i));
    Stroke *er = rubber(800, 400, 820, 1000, 1.f, 9u);
    page_add(pg, er);
    Canvas one, two, paper;
    canvas_init(&one, W, H);
    canvas_init(&two, W, H);
    canvas_init(&paper, W, H);
    page_render(&one, pg, FULL, DPI);
    page_render_layers(&two, pg, FULL, DPI, PAGE_LAYER_PAPER);
    page_render_layers(&two, pg, FULL, DPI, PAGE_LAYER_INK);
    paper_render(&paper, PAPER_MEDIUM, FULL, DPI);
    long lines = 0, kept = 0, wiped = 0;
    const float lim = rubber_core();
    for (int y = 400; y < 1000; y++)
        for (int x = 760; x < 860; x++) {
            if (line_dist(er, x, y) > lim || paper.px[y * W + x] == 255) continue;
            lines++;
            kept += one.px[y * W + x] == paper.px[y * W + x];
            wiped += two.px[y * W + x] == 255;
        }
    CHECK(lines > 100 && kept == lines && wiped == lines);
    canvas_free(&one);
    canvas_free(&two);
    canvas_free(&paper);
    page_free(pg);
}

// BrushState.restored says whether an erase stroke changed the canvas it drew on: never on bare
// paper, always over ink, with or without the fast path.
TEST(restored_tells_a_no_op) {
    enum { PW = 600, PH = 400 };
    Canvas paper, c;
    canvas_init(&paper, PW, PH);
    canvas_init(&c, PW, PH);
    for (int fast = 0; fast < 2; fast++) {
        g_brush_erase_spans = fast;
        for (int t = 0; t < TOOL_COUNT; t++) {
            paper_render(&paper, PAPER_MEDIUM, (Rect){0, 0, PW, PH}, DPI);
            canvas_blit(&c, &paper, (Rect){0, 0, PW, PH}, 0, 0);
            Stroke *w = white((ToolKind)t, 50, 200, 550, 210, 40, 0.f, 1.f, 30, 30, 5u);
            bool restored = true;
            stroke_free(draw_live(&c, &paper, w, &restored));
            CHECK(!restored);
            CHECK(diff_px(&c, &paper, (Rect){0, 0, PW, PH}) == 0);
            Stroke *m = ink(TOOL_MARKER, 50, 200, 550, 210, 40, 1.f, 1.f, 0, 0, 6u);
            stroke_free(draw_live(&c, &paper, m, NULL));
            stroke_free(draw_live(&c, &paper, w, &restored));
            CHECK(restored);
            stroke_free(m);
            stroke_free(w);
        }
    }
    g_brush_erase_spans = true;
    canvas_free(&paper);
    canvas_free(&c);
}

// An erase stroke is an ordinary stroke to undo and redo: undo gives back the page as it was
// before it, redo the page with it.
TEST(undo_and_redo_an_erase_stroke) {
    Page *p = page_new(W, H, PAPER_MEDIUM);
    History h;
    history_init(&h);
    for (int i = 0; i < 4; i++) {
        float y = 600 + i * 100.f;
        // Broad ink gives the erase/undo comparison plenty of changed pixels.
        Stroke *s = stroke_new(TOOL_MARKER, &k_legacy_params[TOOL_MARKER], 1u + (uint32_t)i);
        wave_points(s, 300, y, 1500, y + 40, 30, 0.6f, 1.f, 30, 20);
        page_add(p, s);
        history_push_add(&h, p->n - 1, s);
    }
    Canvas before, after, cur;
    canvas_init(&before, W, H);
    canvas_init(&after, W, H);
    canvas_init(&cur, W, H);
    page_render(&before, p, FULL, DPI);
    // A fixed broad erase profile makes this an undo test independent of fresh defaults.
    Stroke *e = stroke_new(TOOL_ERASER, &k_legacy_params[TOOL_CHARCOAL], erase_seed(TOOL_CHARCOAL, 5u));
    wave_points(e, 600, 500, 1100, 1100, 80, 0.8f, 1.f, 30, 30);
    page_add(p, e);
    history_push_add(&h, p->n - 1, e);
    page_render(&after, p, FULL, DPI);
    CHECK(diff_px(&before, &after, FULL) > 1000);
    canvas_blit(&cur, &after, FULL, 0, 0);
    Rect d;
    CHECK(history_undo(&h, p, &d));
    page_render(&cur, p, d, DPI);
    CHECK(diff_px(&cur, &before, FULL) == 0);
    CHECK(history_redo(&h, p, &d));
    page_render(&cur, p, d, DPI);
    CHECK(diff_px(&cur, &after, FULL) == 0);
    history_free(&h);
    page_free(p);
    canvas_free(&before);
    canvas_free(&after);
    canvas_free(&cur);
}

// The frozen 1.0.0-beta reader opens every page with erase strokes (version 1, or 2 for custom
// music), keeps tool 3 and every seed byte for byte, and its re-save brings each texture back here
// as it was. It draws erase strokes as nothing, so there the erased ink shows again.
TEST(beta_opens_erase_strokes) {
    MusicLayout m = {.systems = 2, .n = 2, .part = {PART_TREBLE, PART_BASS}};
    const PaperKind kinds[2] = {PAPER_MEDIUM, music_encode(&m)};
    Canvas ours, back;
    canvas_init(&ours, W, H);
    canvas_init(&back, W, H);
    for (int ki = 0; ki < 2; ki++) {
        Page *pg = page_new(W, H, kinds[ki]);
        for (int t = 0; t < TOOL_COUNT; t++)
            if (t != TOOL_ERASER)
                page_add(pg, ink((ToolKind)t, 100, 200 + t * 240.f, 1700, 260 + t * 240.f, 60, 0.3f, 1.f, 30, 20, 1u + t));
        for (int t = 0; t < TOOL_COUNT; t++)
            page_add(pg, white((ToolKind)t, 150 + t * 170, 100, 200 + t * 170, 2400, 30, 0.5f, 1.f, 35, 30, 0xF000u + t));
        uint8_t *d = NULL;
        size_t len = 0;
        CHECK(page_serialize(pg, &d, &len) == 0 && d);
        if (!d) continue;
        CHECK((d[4] | d[5] << 8) == (ki ? 2 : 1));
        BetaPage bp;
        CHECK(beta_page_deserialize(d, len, &bp) == 0);
        CHECK(bp.n == (uint32_t)pg->n);
        for (uint32_t i = 0; i < bp.n && i < (uint32_t)pg->n; i++)
            CHECK(bp.strokes[i].tool == (uint8_t)pg->strokes[i]->tool && bp.strokes[i].seed == pg->strokes[i]->seed);
        beta_page_free(&bp);
        uint8_t *d2 = NULL;
        size_t l2 = 0;
        CHECK(beta_resave(d, len, &d2, &l2) == 0 && d2);
        Page *q = NULL, *q0 = NULL;
        CHECK(d2 && page_deserialize(d2, l2, &q) == 0 && q && q->n == pg->n);
        CHECK(page_deserialize(d, len, &q0) == 0 && q0);  // ours, read back (a save rounds pressure and tilt)
        if (q0) page_render(&ours, q0, FULL, DPI);
        page_free(q0);
        if (q) {
            for (int i = 0; i < q->n && i < pg->n; i++) {
                CHECK(q->strokes[i]->seed == pg->strokes[i]->seed);
                CHECK(stroke_render_tool(q->strokes[i]) == stroke_render_tool(pg->strokes[i]));
            }
            page_render(&back, q, FULL, DPI);
            CHECK(diff_px(&ours, &back, FULL) == 0);
            page_free(q);
        }
        // As 1.0.0-beta shows it: the erase strokes draw nothing (its brush_add returns at once
        // for tool 3; test_golden holds its ink), so the erased ink is back.
        bool *drop = calloc((size_t)pg->n, sizeof *drop);
        for (int i = 0; i < pg->n; i++) drop[i] = stroke_is_erase(pg->strokes[i]);
        Page *beta_view = page_without(pg, drop);
        page_render(&back, beta_view, FULL, DPI);
        CHECK(diff_px(&ours, &back, FULL) > 10000);
        view_free(beta_view);
        free(drop);
        free(d);
        free(d2);
        page_free(pg);
    }
    canvas_free(&ours);
    canvas_free(&back);
}

// The texture is known before the first point everywhere a stroke is built: the reader gives every
// stroke the bbox it was drawn with, and a page opened at another size (scale_page) pads each
// erase stroke by its texture's reach.
TEST(reader_and_rescale_know_the_texture) {
    Page *pg = page_new(W, H, PAPER_MEDIUM);
    for (int t = 0; t < TOOL_COUNT; t++) {
        // Fixed historical widths establish that charcoal can reach beyond the round eraser.
        Stroke *s = stroke_new(TOOL_ERASER, &k_legacy_params[t], erase_seed((ToolKind)t, 9u + (uint32_t)t));
        wave_points(s, 150 + t * 170, 300, 200 + t * 170, 2000, 30, 0.5f, 1.f, 80, 10);
        page_add(pg, s);
    }
    uint8_t *d = NULL;
    size_t len = 0;
    CHECK(page_serialize(pg, &d, &len) == 0);
    Page *q = NULL;
    CHECK(d && page_deserialize(d, len, &q) == 0 && q && q->n == pg->n);
    for (int i = 0; q && i < q->n && i < pg->n; i++)
        CHECK(!memcmp(&q->strokes[i]->bbox, &pg->strokes[i]->bbox, sizeof(Rect)));
    // White charcoal laid on its side reaches further than the round tip: padded by charcoal.
    CHECK(pg->strokes[TOOL_CHARCOAL]->bbox.w > pg->strokes[TOOL_ERASER]->bbox.w + 20);
    free(d);
    page_free(q);
    page_free(pg);

    char root[600], cmd[700];
    snprintf(root, sizeof root, "%s/erase-%d", tl_tmp(), (int)getpid());
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    CHECK(system(cmd) == 0);
    CHECK(store_init(root) == 0);
    store_set_device(DPI, W, H);
    NotebookInfo nb;
    CHECK(store_create("Erase strokes", "", PAPER_MEDIUM, &nb) == 0);
    Page *half = page_new(W / 2, H / 2, PAPER_MEDIUM);
    for (int t = 0; t < TOOL_COUNT; t++) {
        ToolParams prm;
        tool_params_default((ToolKind)t, &prm);
        prm.grain = 1.f;
        Stroke *s = stroke_new(TOOL_ERASER, &prm, erase_seed((ToolKind)t, 3u + (uint32_t)t));
        wave_points(s, 60 + t * 85, 100, 90 + t * 85, 1100, 20, 0.5f, 1.f, 80, 10);
        page_add(half, s);
    }
    CHECK(store_save_page(&nb, 0, half) == 0);
    Page *big = NULL;
    CHECK(store_load_page(&nb, 0, W, H, &big) == 0 && big && big->n == TOOL_COUNT && big->w == W);
    Canvas c;
    canvas_init(&c, W, H);
    for (int i = 0; big && i < big->n; i++) {
        const Stroke *s = big->strokes[i];
        // The same bbox as a stroke built at the new size from the start.
        Stroke *fresh = stroke_new(s->tool, &s->params, s->seed);
        for (int k = 0; k < s->n; k++) stroke_append(fresh, &s->pts[k], DPI);
        CHECK(!memcmp(&fresh->bbox, &s->bbox, sizeof(Rect)));
        stroke_free(fresh);
        canvas_fill(&c, FULL, 0);
        stroke_free(draw_live(&c, NULL, s, NULL));
        long touched = 0, outside = 0;
        for (int y = 0; y < H; y++)
            for (int x = 0; x < W; x++)
                if (c.px[y * W + x] > 0) {
                    touched++;
                    outside += !rect_contains(s->bbox, x, y);
                }
        CHECK(touched > 0 && outside == 0);
    }
    canvas_free(&c);
    page_free(big);
    page_free(half);
    CHECK(system(cmd) == 0);
}

// ---- the look of erase strokes, recorded ----

// GRIMOIRE_GOLDEN_PRINT=1 prints the table below instead of checking it (that is how it was made),
// as in test_golden.c. FORMAT.md freezes how an erase stroke draws from 1.1 on, and until a
// released version has drawn one, this table is what keeps a change to it from passing unnoticed:
// re-record it only for a change meant to alter that look, before 1.1 ships; never afterwards, and
// never to make another change pass. The values hold for the host build this suite runs on (clang
// on arm64 macOS); elsewhere floats may round differently, so there the comparison is skipped.
#if defined(__clang__) && defined(__aarch64__) && defined(__APPLE__)
#define GOLDEN_HOST 1
#else
#define GOLDEN_HOST 0
#endif

// Recorded on 2026-10-20 from the 1.1 engine (lane inkcore).
static const struct {
    const char *name;
    uint32_t crc;
} k_look[] = {
    {"medium rule, white pencil", 0x807def5du},
    {"medium rule, white pencil, ink", 0x0ca259eau},
    {"medium rule, white mechanical pencil", 0x20505b6eu},
    {"medium rule, white mechanical pencil, ink", 0xc3f29e67u},
    {"medium rule, white ballpoint", 0xfa3c4357u},
    {"medium rule, white ballpoint, ink", 0x140df6f3u},
    {"medium rule, round tip", 0x7de3880bu},
    {"medium rule, round tip, ink", 0x6533900bu},
    {"medium rule, white highlighter", 0x800f1d04u},
    {"medium rule, white highlighter, ink", 0x2b56527cu},
    {"medium rule, white fountain pen", 0x17a61725u},
    {"medium rule, white fountain pen, ink", 0x32a41ca7u},
    {"medium rule, white fineliner", 0x73b1784eu},
    {"medium rule, white fineliner, ink", 0x1754e09fu},
    {"medium rule, white marker", 0xbab3dfeeu},
    {"medium rule, white marker, ink", 0x3c47f3e8u},
    {"medium rule, white charcoal", 0x9f243fbdu},
    {"medium rule, white charcoal, ink", 0xb3961079u},
    {"medium rule, white calligraphy pen", 0xb1fc2850u},
    {"medium rule, white calligraphy pen, ink", 0x5cec3bd2u},
    {"medium rule, soft round tip", 0x621d8e19u},
    {"medium rule, soft round tip, ink", 0x36779c1eu},
    {"medium rule, fine round tip", 0xf477bc76u},
    {"medium rule, fine round tip, ink", 0xacafd213u},
    {"grand staff, white pencil", 0x5ab58951u},
    {"grand staff, white pencil, ink", 0x0ca259eau},
    {"grand staff, white mechanical pencil", 0x639afd54u},
    {"grand staff, white mechanical pencil, ink", 0xc3f29e67u},
    {"grand staff, white ballpoint", 0xc2109e2au},
    {"grand staff, white ballpoint, ink", 0x140df6f3u},
    {"grand staff, round tip", 0xd7a8a412u},
    {"grand staff, round tip, ink", 0x6533900bu},
    {"grand staff, white highlighter", 0x76ea94e2u},
    {"grand staff, white highlighter, ink", 0x2b56527cu},
    {"grand staff, white fountain pen", 0x00c8770cu},
    {"grand staff, white fountain pen, ink", 0x32a41ca7u},
    {"grand staff, white fineliner", 0x3e3fa675u},
    {"grand staff, white fineliner, ink", 0x1754e09fu},
    {"grand staff, white marker", 0x96a6610fu},
    {"grand staff, white marker, ink", 0x3c47f3e8u},
    {"grand staff, white charcoal", 0x8dde2f93u},
    {"grand staff, white charcoal, ink", 0xb3961079u},
    {"grand staff, white calligraphy pen", 0x5443ff59u},
    {"grand staff, white calligraphy pen, ink", 0x5cec3bd2u},
    {"grand staff, soft round tip", 0xa8a7d6c1u},
    {"grand staff, soft round tip, ink", 0x36779c1eu},
    {"grand staff, fine round tip", 0x18e2011bu},
    {"grand staff, fine round tip, ink", 0xacafd213u},
};

static bool g_print;

static uint32_t crc_table[256];
static uint32_t crc32_bytes(uint32_t crc, const uint8_t *p, size_t n) {
    if (!crc_table[1])
        for (uint32_t i = 0; i < 256; i++) {
            uint32_t c = i;
            for (int k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
            crc_table[i] = c;
        }
    crc = ~crc;
    for (size_t i = 0; i < n; i++) crc = crc_table[(crc ^ p[i]) & 0xFF] ^ (crc >> 8);
    return ~crc;
}

static uint32_t crc_rect(const Canvas *c, Rect r) {
    uint32_t crc = 0;
    for (int y = r.y; y < r.y + r.h; y++) crc = crc32_bytes(crc, c->px + (size_t)y * c->stride + r.x, (size_t)r.w);
    return crc;
}

static void look(const char *name, uint32_t crc) {
    if (g_print) {
        printf("    {\"%s\", 0x%08xu},\n", name, crc);
        return;
    }
    if (!GOLDEN_HOST) return;
    // White ink in the highlighter's texture drawn with the vertical tip (TIP_HIGHLIGHTER_VERTICAL), as
    // strokes drawn since 2.0.0-beta.5 record it: what 2.0.0-beta.2 to beta.4 drew for every such
    // stroke (issue #5). The table above is the round tip, which older strokes (no tip recorded) get.
    static const struct { const char *name; uint32_t crc; } vertical[] = {
        {"medium rule, white highlighter, vertical tip", 0x1d8309cau},
        {"medium rule, white highlighter, vertical tip, ink", 0x14d61db0u},
        {"grand staff, white highlighter, vertical tip", 0x0a4bc45du},
        {"grand staff, white highlighter, vertical tip, ink", 0x14d61db0u},
    };
    for (size_t i = 0; i < sizeof vertical / sizeof vertical[0]; i++)
        if (!strcmp(vertical[i].name, name)) {
            CHECK(vertical[i].crc == crc);
            if (vertical[i].crc != crc) fprintf(stderr, "  %s: 0x%08x, recorded 0x%08x\n", name, crc, vertical[i].crc);
            return;
        }
    for (size_t i = 0; i < sizeof k_look / sizeof k_look[0]; i++)
        if (!strcmp(k_look[i].name, name)) {
            uint32_t expected = k_look[i].crc;
            CHECK(expected == crc);
            if (expected != crc) fprintf(stderr, "  %s: 0x%08x, recorded 0x%08x\n", name, crc, expected);
            return;
        }
    fprintf(stderr, "  %s: not in the table\n", name);
    CHECK(!"every render has a recorded value");
}

// test_golden.c's stroke: a zigzag whose pressure starts below every deadzone, swells to full and
// fades again, the pen leaning from left to right and nodding. Plain arithmetic only.
static Stroke *zigzag(ToolKind tool, const ToolParams *prm, uint32_t seed, float y0) {
    Stroke *s = stroke_new(tool, prm, seed);
    enum { N = 161 };
    for (int i = 0; i < N; i++) {
        float f = (float)i / (float)(N - 1);
        float tri = fabsf(fmodf(f * 6.f, 2.f) - 1.f);
        float bell = 1.f - (2.f * f - 1.f) * (2.f * f - 1.f);
        PenSample q = {220.f + 1420.f * f, y0 + 170.f * tri - 85.f + 60.f * f * f, i < 3 ? 0.f : 0.04f + 0.96f * bell,
                       -60.f + 120.f * f, 50.f * tri - 15.f, (uint32_t)i * 7u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// The erase stroke across that ink: a slower wave the other way, pressing hardest in the middle,
// the pen leaning from right to left (so the calligraphy nib turns and charcoal lies on its side).
static Stroke *wave(const ToolParams *prm, uint32_t seed, float y0) {
    Stroke *s = stroke_new(TOOL_ERASER, prm, seed);
    enum { N = 121 };
    for (int i = 0; i < N; i++) {
        float f = (float)i / (float)(N - 1);
        float bell = 1.f - (2.f * f - 1.f) * (2.f * f - 1.f);
        PenSample q = {1660.f - 1460.f * f, y0 + 110.f * sinf(f * 7.f), i < 2 ? 0.f : 0.1f + 0.9f * bell,
                       65.f - 130.f * f, 20.f - 40.f * f * f, (uint32_t)i * 9u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// The erase strokes recorded: every texture (TOOL_ERASER: the round tip, with its default 2.5 to
// 7 mm), the round tip at half strength (the per-pixel path) and at its smallest.
enum { LOOK_SOFT = TOOL_COUNT, LOOK_FINE, LOOK_COUNT };
static const char *const k_look_key[LOOK_COUNT] = {
    "white pencil", "white mechanical pencil", "white ballpoint", "round tip", "white highlighter",
    "white fountain pen", "white fineliner", "white marker", "white charcoal", "white calligraphy pen",
    "soft round tip", "fine round tip"};

// How erase strokes draw: each one over ink of five tools, on ruled paper and a grand staff, the
// page and its ink layer, compared with the recorded table (and each one erases plenty).
TEST(erase_look_is_recorded) {
    static const PaperKind papers[2] = {PAPER_MEDIUM, PAPER_GRAND};
    static const ToolKind under[5] = {TOOL_MARKER, TOOL_PENCIL, TOOL_CHARCOAL, TOOL_FOUNTAIN, TOOL_BALLPOINT};
    const float y0 = 1240.f;
    const Rect band = {0, 980, W, 520};
    Canvas c, inked;
    CHECK(canvas_init(&c, W, H) == 0);
    CHECK(canvas_init(&inked, W, H) == 0);
    char key[96];
    for (int pi = 0; pi < 2; pi++)
        for (int v = 0; v < LOOK_COUNT; v++) {
            Page *pg = page_new(W, H, papers[pi]);
            for (int u = 0; u < 5; u++) {
                ToolParams ip = k_legacy_params[under[u]];
                page_add(pg, zigzag(under[u], &ip, 0x00A5F00Du + (uint32_t)u * 40503u, y0 - 60.f + 30.f * (float)u));
            }
            if (v == 0 && !g_print) page_render(&inked, pg, band, DPI);  // the ink before any erasing
            ToolKind tex = v < TOOL_COUNT ? (ToolKind)v : TOOL_ERASER;
            ToolParams prm = k_legacy_params[tex];
            if (v == LOOK_SOFT) prm.opacity_min = prm.opacity_max = 0.5f;
            if (v == LOOK_FINE) prm.size_min_mm = prm.size_max_mm = 0.10f;
            page_add(pg, wave(&prm, erase_seed(tex, 0x00C0DE00u + (uint32_t)v), y0));
            const char *paper = papers[pi] == PAPER_MEDIUM ? "medium rule" : "grand staff";
            page_render(&c, pg, band, DPI);
            snprintf(key, sizeof key, "%s, %s", paper, k_look_key[v]);
            look(key, crc_rect(&c, band));
            if (!g_print) CHECK(diff_px(&c, &inked, band) > 1000);
            canvas_fill(&c, band, 255);
            page_render_layers(&c, pg, band, DPI, PAGE_LAYER_INK);
            snprintf(key, sizeof key, "%s, %s, ink", paper, k_look_key[v]);
            look(key, crc_rect(&c, band));
            if (tex == TOOL_HIGHLIGHTER && !g_print) {  // the same white stroke drawn since 2.0.0-beta.5
                stroke_mark_tip(pg->strokes[pg->n - 1]);
                CHECK(stroke_tip(pg->strokes[pg->n - 1]) == TIP_HIGHLIGHTER_VERTICAL);
                page_render(&c, pg, band, DPI);
                snprintf(key, sizeof key, "%s, white highlighter, vertical tip", paper);
                look(key, crc_rect(&c, band));
                canvas_fill(&c, band, 255);
                page_render_layers(&c, pg, band, DPI, PAGE_LAYER_INK);
                snprintf(key, sizeof key, "%s, white highlighter, vertical tip, ink", paper);
                look(key, crc_rect(&c, band));
            }
            page_free(pg);
        }
    canvas_free(&c);
    canvas_free(&inked);
}

int main(void) {
    font_init();  // some papers draw labels and clefs
    g_print = getenv("GRIMOIRE_GOLDEN_PRINT") != NULL;
    if (g_print) {  // the table only
        printf("static const struct {\n    const char *name;\n    uint32_t crc;\n} k_look[] = {\n");
        RUN(erase_look_is_recorded);
        printf("};\n");
        return 0;
    }
    if (!GOLDEN_HOST) fprintf(stderr, "  erase look: not the recording host (clang, arm64 macOS): renders not compared\n");
    RUN(erase_look_is_recorded);
    RUN(restore_blend);
    RUN(live_equals_rerender);
    RUN(span_fast_path_is_the_per_pixel_path);
    RUN(eraser_core_returns_to_paper);
    RUN(ink_after_erasing_is_untouched);
    RUN(white_textures_are_deterministic);
    RUN(other_texture_bytes_draw_the_round_tip);
    RUN(erase_inside_bbox);
    RUN(fully_erased_stroke_leaves_nothing);
    RUN(compaction_drops_only_what_changes_nothing);
    RUN(compaction_is_the_rule);
    RUN(compaction_checks_the_page_too);
    RUN(restored_is_the_page_only);
    RUN(one_call_keeps_the_lines);
    RUN(restored_tells_a_no_op);
    RUN(undo_and_redo_an_erase_stroke);
    RUN(beta_opens_erase_strokes);
    RUN(reader_and_rescale_know_the_texture);
    return tl_done();
}
