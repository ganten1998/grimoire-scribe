// test_inkwire.c: white ink and the precise eraser in the editor (src/inkmode.h: what a stroke of
// the pen does, the eraser's size at a zoom), the settings that choose them, and compaction
// (src/compact.h, src/bgwork_compact.c): a page file loses the strokes that change nothing, looks
// exactly as before, and only pages the list names are touched.
#include <signal.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

#include "bgjob.h"
#include "compact.h"
#include "inkmode.h"
#include "store.h"
#include "testlib.h"

static const float DPI = 300.f;
enum { W = 1860, H = 2480 };
static const Rect FULL = {0, 0, W, H};
static char root[600];

// ---- what a stroke of the pen does ----

TEST(pen_stroke_follows_the_settings) {
    Settings s;
    settings_default(&s);
    // Defaults: ink with the pen's tool, the eraser takes whole strokes, the button highlights.
    PenStroke p = pen_stroke(&s, false, false);
    CHECK(p.act == PEN_INK && p.tool == TOOL_PENCIL);
    CHECK(pen_stroke(&s, true, false).act == PEN_WHOLE);
    p = pen_stroke(&s, false, true);
    CHECK(p.act == PEN_INK && p.tool == TOOL_HIGHLIGHTER);
    for (int mode = 0; mode < 2; mode++)
        for (int white = 0; white < 2; white++)
            for (int side = 0; side < 4; side++)
                for (int cur = 0; cur < TOOL_COUNT; cur++) {
                    s.eraser_mode = mode;
                    s.white = white;
                    s.side_button = side;
                    s.current_tool = (ToolKind)cur;
                    s.draw_tool = TOOL_CHARCOAL;
                    PenAct eraser = mode ? PEN_PRECISE : PEN_WHOLE;
                    ToolKind pen = cur == TOOL_ERASER ? TOOL_CHARCOAL : (ToolKind)cur;
                    // The eraser end erases, whatever else.
                    for (int b = 0; b < 2; b++) {
                        p = pen_stroke(&s, true, b);
                        CHECK(p.act == eraser && p.tool == TOOL_ERASER);
                    }
                    // The tip alone: the pen's tool, white with `white`; the eraser tool erases.
                    p = pen_stroke(&s, false, false);
                    if (cur == TOOL_ERASER) CHECK(p.act == eraser && p.tool == TOOL_ERASER);
                    else CHECK(p.act == (white ? PEN_WHITE : PEN_INK) && p.tool == pen);
                    // With the side button held.
                    p = pen_stroke(&s, false, true);
                    if (side == 1) CHECK(p.act == eraser && p.tool == TOOL_ERASER);
                    else if (side == 2) CHECK(p.act == PEN_INK && p.tool == TOOL_HIGHLIGHTER);
                    else if (side == 3) CHECK(p.act == PEN_WHITE && p.tool == pen);
                    else if (cur == TOOL_ERASER) CHECK(p.act == eraser && p.tool == TOOL_ERASER);
                    else CHECK(p.act == (white ? PEN_WHITE : PEN_INK) && p.tool == pen);
                }
    // The eraser picked with the toolbar's tool still showing the eraser (a hand-edited file): the
    // pencil is the pen's drawing tool.
    s.current_tool = s.draw_tool = TOOL_ERASER;
    s.side_button = 3;
    CHECK(pen_drawing_tool(&s) == TOOL_PENCIL && pen_stroke(&s, false, true).tool == TOOL_PENCIL);
    s.draw_tool = (ToolKind)99;
    CHECK(pen_drawing_tool(&s) == TOOL_PENCIL);
}

// The precise eraser keeps its size on screen: its widths divided by the zoom, within the range a
// page file keeps (so the stroke reads back as drawn).
TEST(eraser_size_follows_the_zoom) {
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    int ranges = 0;
    for (int i = 0; i < n; i++)
        if (pi[i].offset == offsetof(ToolParams, size_min_mm) || pi[i].offset == offsetof(ToolParams, size_max_mm)) {
            CHECK(pi[i].min == ERASER_MM_MIN && pi[i].max == ERASER_MM_MAX);
            ranges++;
        }
    CHECK(ranges == 2);
    ToolParams e;
    tool_params_default(TOOL_ERASER, &e);
    ToolParams z = eraser_at_zoom(&e, 1.f);
    CHECK(!memcmp(&z, &e, sizeof z));
    z = eraser_at_zoom(&e, 2.f);
    CHECK_NEAR(z.size_min_mm, e.size_min_mm / 2.f, 1e-6);
    CHECK_NEAR(z.size_max_mm, e.size_max_mm / 2.f, 1e-6);
    CHECK(z.opacity_max == e.opacity_max && z.pressure_gamma == e.pressure_gamma && z.smoothing == e.smoothing);
    z = eraser_at_zoom(&e, 4.f);  // the 2.5 mm minimum at 4x: about 0.6 mm on the page
    CHECK_NEAR(z.size_min_mm, 0.625f, 1e-6);
    z = eraser_at_zoom(&e, 100.f);  // never below the range
    CHECK(z.size_min_mm == ERASER_MM_MIN && z.size_max_mm == ERASER_MM_MIN);
    z = eraser_at_zoom(&e, 0.25f);  // zoomed out past the page: never above it
    CHECK(z.size_min_mm == 10.f && z.size_max_mm == ERASER_MM_MAX);
    float nan = strtof("nan", NULL);
    z = eraser_at_zoom(&e, nan);
    CHECK(z.size_min_mm == e.size_min_mm && z.size_max_mm == e.size_max_mm);
    z = eraser_at_zoom(&e, 0.f);
    CHECK(z.size_min_mm == e.size_min_mm);
    ToolParams bad = e;
    bad.size_min_mm = nan;
    z = eraser_at_zoom(&bad, 2.f);
    CHECK(z.size_min_mm == ERASER_MM_MIN);
    // An erase stroke with the zoomed settings reads back with exactly them.
    for (int k = 0; k < 4; k++) {
        float zoom = (float[]){1.f, 1.37f, 3.9f, 0.4f}[k];
        ToolParams prm = eraser_at_zoom(&e, zoom);
        Page *pg = page_new(W, H, PAPER_MEDIUM);
        Stroke *s = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_ERASER, 77u));
        PenSample q = {500, 600, 0.7f, 0, 0, 0};
        stroke_append(s, &q, DPI);
        page_add(pg, s);
        uint8_t *d;
        size_t len;
        Page *back = NULL;
        CHECK(page_serialize(pg, &d, &len) == 0 && page_deserialize(d, len, &back) == 0 && back && back->n == 1);
        if (back && back->n == 1) {
            CHECK(!memcmp(&back->strokes[0]->params, &prm, sizeof prm));
            CHECK(back->strokes[0]->seed == erase_seed(TOOL_ERASER, 77u) && stroke_render_tool(back->strokes[0]) == TOOL_ERASER);
        }
        page_free(back);
        free(d);
        page_free(pg);
    }
}

// The new keys and side_button's fourth value go through settings.ini as they are; a value past
// what this version has is used as the nearest one and written back as it was.
TEST(settings_round_trip) {
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    Settings s;
    settings_default(&s);
    CHECK(s.eraser_mode == 0 && !s.white && s.side_button == 2);
    s.eraser_mode = 1;
    s.white = true;
    s.side_button = 3;
    CHECK(settings_save(&s) == 0);
    Settings b;
    CHECK(settings_load(&b) == 0 && b.eraser_mode == 1 && b.white && b.side_button == 3);
    const char *newer = "eraser_mode=2\nwhite=2\nside_button=5\n";
    CHECK(write_file_atomic(path, newer, strlen(newer)) == 0);
    CHECK(settings_load(&b) == 0 && b.eraser_mode == 1 && b.white && b.side_button == 3);
    CHECK(settings_save(&b) == 0);
    size_t len;
    char *t = read_file(path, &len);
    CHECK(t && strstr(t, "\neraser_mode=2\n") && strstr(t, "\nwhite=2\n") && strstr(t, "\nside_button=5\n"));
    free(t);
    unlink(path);
}

// ---- compaction ----

static Stroke *line(ToolKind t, float x0, float y0, float x1, float y1, float p, uint32_t seed) {
    ToolParams prm;
    tool_params_default(t, &prm);
    Stroke *s = stroke_new(t, &prm, seed);
    int steps = (int)(hypotf(x1 - x0, y1 - y0) / 10.f) + 1;
    for (int i = 0; i <= steps; i++) {
        float f = (float)i / (float)steps;
        PenSample q = {x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, p, 10.f, -5.f, (uint32_t)i * 7u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// The precise eraser's round tip along a line at pressure p (1: its full 7 mm).
static Stroke *rubber(float x0, float y0, float x1, float y1, float p, uint32_t hash) {
    Stroke *s = line(TOOL_ERASER, x0, y0, x1, y1, p, 0);
    s->seed = erase_seed(TOOL_ERASER, hash);
    return s;
}

// White ink with a pen's texture along a line.
static Stroke *white_line(ToolKind texture, float x0, float y0, float x1, float y1, float p, uint32_t hash) {
    Stroke *s = line(texture, x0, y0, x1, y1, p, 0);
    s->tool = TOOL_ERASER;
    s->seed = erase_seed(texture, hash);
    s->bbox = RECT_EMPTY;  // its reach is its texture's: built again
    int n = s->n;
    PenSample *pts = s->pts;
    s->pts = NULL;
    s->n = s->cap = 0;
    for (int k = 0; k < n; k++) stroke_append(s, &pts[k], DPI);
    free(pts);
    return s;
}

static void page_path(const NotebookInfo *nb, int index, char *buf, size_t cap) {
    snprintf(buf, cap, "%s/notebooks/%s/p%u.qpg", root, nb->id, nb->page_ids[index]);
}

static void list_path(const NotebookInfo *nb, char *buf, size_t cap) {
    snprintf(buf, cap, "%s/notebooks/%s/erased.txt", root, nb->id);
}

static bool exists(const char *path) {
    struct stat st;
    return stat(path, &st) == 0;
}

static uint8_t *file_bytes(const char *path, size_t *len) { return read_file(path, len); }

// The page in its file, drawn: with its paper, or its ink layer alone (fresh white canvas).
static void render_file(const NotebookInfo *nb, int index, int layers, Canvas *c, int *strokes) {
    Page *p = NULL;
    CHECK(store_read_page(nb, index, &p) == 0 && p);
    canvas_init(c, W, H);
    if (!p) return;
    page_render_layers(c, p, FULL, DPI, layers);
    if (strokes) *strokes = p->n;
    page_free(p);
}

static bool same_px(const Canvas *a, const Canvas *b) {
    return a->w == b->w && a->h == b->h && !memcmp(a->px, b->px, (size_t)a->w * (size_t)a->h);
}

// The editor's live path: the page drawn, its paper alone beside it (the editor's cache), and each
// erase stroke started with stroke_brush_begin over them, sample by sample, with the editor's
// settings (the precise eraser at a zoom, white with a pen's own); one that changes no pixel is
// dropped. The page drawn afresh is the live canvas, pixel for pixel.
static Stroke *live_erase(Page *pg, Canvas *live, const Canvas *paper, ToolKind texture, const ToolParams *prm,
                          float x0, float y0, float x1, float y1, uint32_t hash, bool *restored) {
    Stroke *n = stroke_new(TOOL_ERASER, prm, erase_seed(texture, hash));
    BrushState b;
    stroke_brush_begin(&b, n, DPI, paper->px, 0, 0, paper->stride);
    int steps = (int)(hypotf(x1 - x0, y1 - y0) / 9.f) + 1;
    for (int i = 0; i <= steps; i++) {
        float f = (float)i / (float)steps;
        PenSample q = {x0 + (x1 - x0) * f, y0 + (y1 - y0) * f + 25.f * sinf(f * 7.f), 0.4f + 0.6f * f, 40.f, 20.f,
                       (uint32_t)i * 6u};
        stroke_append(n, &q, DPI);
        brush_add(&b, live, &q);
    }
    *restored = b.restored;
    if (!b.restored) {
        stroke_free(n);
        return NULL;
    }
    page_add(pg, n);
    return n;
}

TEST(live_erase_equals_rerender) {
    const PaperKind papers[] = {PAPER_MEDIUM, PAPER_MUSIC, PAPER_GRID, PAPER_CORNELL};
    Settings st;
    settings_default(&st);
    for (size_t pk = 0; pk < sizeof papers / sizeof papers[0]; pk++) {
        Page *pg = page_new(W, H, papers[pk]);
        Canvas live, paper, fresh;
        canvas_init(&live, W, H);
        canvas_init(&paper, W, H);
        canvas_init(&fresh, W, H);
        paper_render(&paper, papers[pk], FULL, DPI);
        for (int t = 0; t < TOOL_COUNT; t++)
            if (t != TOOL_ERASER) page_add(pg, line((ToolKind)t, 200, 300 + t * 220, 1600, 360 + t * 220, 0.8f, 40u + (uint32_t)t));
        page_render(&live, pg, FULL, DPI);
        bool r;
        float zooms[] = {1.f, 2.5f, 4.f, 0.6f};
        for (int z = 0; z < 4; z++) {
            ToolParams e = eraser_at_zoom(&st.tools[TOOL_ERASER], zooms[z]);
            CHECK(live_erase(pg, &live, &paper, TOOL_ERASER, &e, 300 + z * 350, 200, 380 + z * 350, 2300, 50u + (uint32_t)z, &r));
            CHECK(r);
        }
        CHECK(live_erase(pg, &live, &paper, TOOL_PENCIL, &st.tools[TOOL_PENCIL], 150, 320, 1700, 345, 60u, &r) && r);
        CHECK(live_erase(pg, &live, &paper, TOOL_CHARCOAL, &st.tools[TOOL_CHARCOAL], 150, 1200, 1700, 1230, 61u, &r) && r);
        // Over blank paper (its lines are the paper): nothing changes, nothing is kept. (Over ink
        // erased already it may: the eraser's soft rim lightens its edge a little more each time.)
        ToolParams e = eraser_at_zoom(&st.tools[TOOL_ERASER], 1.f);
        int n = pg->n;
        CHECK(!live_erase(pg, &live, &paper, TOOL_ERASER, &e, 1750, 2350, 1800, 2400, 70u, &r) && !r);
        CHECK(!live_erase(pg, &live, &paper, TOOL_CALLIGRAPHY, &st.tools[TOOL_CALLIGRAPHY], 40, 40, 150, 200, 71u, &r) && !r);
        CHECK(pg->n == n);
        page_render(&fresh, pg, FULL, DPI);
        CHECK(same_px(&live, &fresh));
        canvas_free(&live);
        canvas_free(&paper);
        canvas_free(&fresh);
        page_free(pg);
    }
}

// A page with ink the eraser takes away entirely (and the erase stroke over it then changes
// nothing), ink it takes away in part, white charcoal over a word and ink drawn after it all.
static Page *erased_page(PaperKind k) {
    Page *p = page_new(W, H, k);
    page_add(p, line(TOOL_BALLPOINT, 300, 500, 900, 520, 0.7f, 11u));   // 0: gone under 1
    page_add(p, rubber(270, 505, 930, 515, 1.f, 12u));                   // 1: then changes nothing
    page_add(p, line(TOOL_PENCIL, 300, 800, 900, 800, 0.8f, 13u));      // 2: erased in part by 3
    page_add(p, rubber(600, 740, 610, 860, 1.f, 14u));                   // 3
    page_add(p, line(TOOL_MARKER, 300, 1200, 1300, 1260, 0.9f, 15u));   // 4: whited out in part
    page_add(p, white_line(TOOL_CHARCOAL, 700, 1150, 900, 1300, 1.f, 16u));
    page_add(p, line(TOOL_FINELINER, 300, 1600, 1500, 1650, 0.6f, 17u));  // 6: drawn after it all
    page_add(p, rubber(1500, 2100, 1700, 2200, 1.f, 18u));               // 7: over blank paper
    return p;
}

// Compaction takes out exactly what page_droppable says, the page looks the same (with its paper
// and its ink layer), the strokes it keeps are byte for byte what they were, and a page it is not
// asked about (or one without erase strokes) is not touched. The list goes once it is done.
TEST(compaction_takes_erased_ink_out) {
    const PaperKind papers[] = {PAPER_MEDIUM, PAPER_GRAND, PAPER_DOTS, PAPER_BLANK};
    for (size_t pk = 0; pk < sizeof papers / sizeof papers[0]; pk++) {
        NotebookInfo nb;
        CHECK(store_create("Compacted", "", papers[pk], &nb) == 0);
        CHECK(store_add_page(&nb, 1, papers[pk]) == 0 && store_add_page(&nb, 2, papers[pk]) == 0);
        Page *p = erased_page(papers[pk]);
        CHECK(store_save_page(&nb, 0, p) == 0);
        Page *q = erased_page(papers[pk]);  // the same, on a page nobody lists
        CHECK(store_save_page(&nb, 2, q) == 0);
        Page *plain = page_new(W, H, papers[pk]);
        page_add(plain, line(TOOL_PENCIL, 300, 300, 900, 900, 0.7f, 21u));
        CHECK(store_save_page(&nb, 1, plain) == 0);
        // What the file should become: the page as read from it, without what page_droppable marks.
        Page *rd = NULL;
        CHECK(store_read_page(&nb, 0, &rd) == 0 && rd && rd->n == 8);
        bool drop[16] = {0};
        int expect = rd ? page_droppable(rd, DPI, drop) : -1;
        CHECK(expect >= 3 && drop[0] && drop[1] && drop[7] && !drop[2] && !drop[3] && !drop[6]);
        for (int i = rd ? rd->n - 1 : -1; i >= 0; i--)
            if (drop[i]) stroke_free(page_remove(rd, i));
        uint8_t *want = NULL;
        size_t want_len = 0;
        CHECK(rd && page_serialize(rd, &want, &want_len) == 0);
        page_free(rd);
        Canvas before, before_ink, after, after_ink;
        int n_before = 0, n_after = 0;
        render_file(&nb, 0, PAGE_LAYER_ALL, &before, &n_before);
        render_file(&nb, 0, PAGE_LAYER_INK, &before_ink, NULL);
        char p1[800], p2[800], p3[800], lp[800];
        page_path(&nb, 0, p1, sizeof p1);
        page_path(&nb, 1, p2, sizeof p2);
        page_path(&nb, 2, p3, sizeof p3);
        list_path(&nb, lp, sizeof lp);
        size_t l2, l3;
        uint8_t *b2 = file_bytes(p2, &l2), *b3 = file_bytes(p3, &l3);
        // Listed: page 1 (twice: once is kept), page 2 (no erase strokes), a page no longer there.
        CHECK(!compact_listed(nb.id));
        CHECK(compact_note_page(nb.id, nb.page_ids[0]) == 0 && compact_note_page(nb.id, nb.page_ids[0]) == 0);
        CHECK(compact_note_page(nb.id, nb.page_ids[1]) == 0 && compact_note_page(nb.id, 999) == 0);
        CHECK(compact_listed(nb.id) && exists(lp));
        CHECK(bg_compact_pages(nb.id, DPI) == 0);
        render_file(&nb, 0, PAGE_LAYER_ALL, &after, &n_after);
        render_file(&nb, 0, PAGE_LAYER_INK, &after_ink, NULL);
        CHECK(n_before == 8 && n_after == 8 - expect);
        CHECK(same_px(&before, &after) && same_px(&before_ink, &after_ink));
        size_t l1;
        uint8_t *b1 = file_bytes(p1, &l1);
        CHECK(b1 && l1 == want_len && !memcmp(b1, want, want_len));
        size_t m2, m3;
        uint8_t *c2 = file_bytes(p2, &m2), *c3 = file_bytes(p3, &m3);
        CHECK(b2 && c2 && l2 == m2 && !memcmp(b2, c2, l2));  // listed, nothing to do: as it was
        CHECK(b3 && c3 && l3 == m3 && !memcmp(b3, c3, l3));  // not listed: as it was
        CHECK(!compact_listed(nb.id) && !exists(lp));
        char tmp[900];
        snprintf(tmp, sizeof tmp, "%s.compact.tmp", p1);
        CHECK(!exists(tmp));
        // Again: nothing listed, nothing done.
        CHECK(bg_compact_pages(nb.id, DPI) == 0);
        free(c2);
        free(c3);
        uint8_t *again = file_bytes(p1, &m2);
        CHECK(again && m2 == l1 && !memcmp(again, b1, l1));
        free(again);
        free(b1);
        free(b2);
        free(b3);
        free(want);
        canvas_free(&before);
        canvas_free(&before_ink);
        canvas_free(&after);
        canvas_free(&after_ink);
        page_free(p);
        page_free(q);
        page_free(plain);
        CHECK(store_delete(nb.id) == 0);
    }
}

// Whether notebook nb's list names page pid.
static bool lists(const NotebookInfo *nb, uint32_t pid) {
    char lp[800];
    list_path(nb, lp, sizeof lp);
    size_t len;
    char *t = read_file(lp, &len);
    bool found = false;
    for (char *l = t; l && *l && !found;) {
        found = *l != '#' && strtoul(l, NULL, 10) == pid;
        char *nl = strchr(l, '\n');
        if (!nl) break;
        l = nl + 1;
    }
    free(t);
    return found;
}

// Pages compaction must leave as they are: from a newer version (read-only), at another size than
// the notebook's (an older version wrote it on another Scribe), damaged, or missing. Each file
// stays byte for byte. A missing page goes off the list (there is no ink in it); the others stay
// listed, and the one of another size is compacted once it is written at the notebook's size.
TEST(compaction_leaves_what_it_cannot_write) {
    NotebookInfo nb;
    CHECK(store_create("Left", "", PAPER_MEDIUM, &nb) == 0);
    for (int i = 1; i < 4; i++) CHECK(store_add_page(&nb, i, PAPER_MEDIUM) == 0);
    char path[4][800];
    for (int i = 0; i < 4; i++) page_path(&nb, i, path[i], sizeof path[i]);
    // 0: version 3 (a newer Grimoire's); 1: a page of 1986x2648 with erased ink; 2: damage; 3: gone.
    uint8_t v3[64] = {'Q', 'P', 'G', '1', 3, 0, 1, 0, 0x44, 0x07, 0, 0, 0xB0, 0x09, 0, 0};
    CHECK(write_file_atomic(path[0], v3, sizeof v3) == 0);
    Page *big = page_new(1986, 2648, PAPER_MEDIUM);
    page_add(big, line(TOOL_BALLPOINT, 300, 500, 900, 520, 0.7f, 31u));
    page_add(big, rubber(270, 505, 930, 515, 1.f, 32u));
    uint8_t *d;
    size_t len;
    CHECK(page_serialize(big, &d, &len) == 0 && write_file_atomic(path[1], d, len) == 0);
    free(d);
    page_free(big);
    CHECK(write_file_atomic(path[2], "QPG1\x01\x00", 6) == 0);
    unlink(path[3]);
    size_t was_len[3];
    uint8_t *was[3];
    for (int i = 0; i < 3; i++) was[i] = file_bytes(path[i], &was_len[i]);
    for (int i = 0; i < 4; i++) CHECK(compact_note_page(nb.id, nb.page_ids[i]) == 0);
    CHECK(bg_compact_pages(nb.id, DPI) == 0);
    for (int i = 0; i < 3; i++) {
        size_t l;
        uint8_t *now = file_bytes(path[i], &l);
        CHECK(was[i] && now && l == was_len[i] && !memcmp(now, was[i], l));
        free(now);
        free(was[i]);
    }
    CHECK(!exists(path[3]) && !lists(&nb, nb.page_ids[3]));
    for (int i = 0; i < 3; i++) CHECK(lists(&nb, nb.page_ids[i]));
    // The page of another size, written at the notebook's (as the editor writes it after an edit):
    // compacted now, and off the list.
    Page *back = NULL;
    CHECK(store_read_page(&nb, 1, &back) == 0 && back && back->w == W && back->n == 2);
    CHECK(back && store_save_page(&nb, 1, back) == 0);
    page_free(back);
    CHECK(bg_compact_pages(nb.id, DPI) == 0);
    back = NULL;
    CHECK(store_read_page(&nb, 1, &back) == 0 && back && back->n == 0);
    page_free(back);
    CHECK(!lists(&nb, nb.page_ids[1]) && lists(&nb, nb.page_ids[0]) && lists(&nb, nb.page_ids[2]));
    CHECK(store_delete(nb.id) == 0);
}

// Data this version reads past (docs/FORMAT.md, Registry): ext bytes on ink or on an erase stroke,
// an erase stroke's texture it lacks (a later tool's, or 4, the eraser's own), a chunk, a paper it
// does not know. The version that wrote it may draw the page otherwise (a colour, a layer, a
// texture that covers less than the round tip), so such a page is never judged by how this version
// draws it: left byte for byte, and listed for the version that wrote it. The same page without
// that data is compacted.
TEST(newer_data_is_left_as_it_is) {
    enum { CASES = 7 };
    NotebookInfo nb;
    CHECK(store_create("Newer", "", PAPER_MEDIUM, &nb) == 0);
    for (int i = 1; i < CASES; i++) CHECK(store_add_page(&nb, i, PAPER_MEDIUM) == 0);
    Page *pg[CASES];
    for (int i = 0; i < CASES; i++) pg[i] = erased_page(i == 5 ? (PaperKind)0x52000001u : PAPER_MEDIUM);
    pg[0]->strokes[0]->ext[0] = 5;                   // ink in a newer version's colour
    pg[1]->strokes[1]->ext[2] = 1;                   // a newer version's flag on the eraser
    pg[2]->strokes[1]->seed = 0x20u << 24 | 12u;      // a texture a later tool would have
    pg[3]->strokes[1]->seed = 4u << 24 | 12u;         // texture 4, the eraser's own
    static const uint8_t chunk[] = {'n', 'o', 't', 'e', 3, 0, 0, 0, 'a', 'b', 'c'};  // safe to copy
    pg[4]->tail = malloc(sizeof chunk);
    if (pg[4]->tail) {
        memcpy(pg[4]->tail, chunk, sizeof chunk);
        pg[4]->tail_len = sizeof chunk;
    }
    // 5: an image paper of a role this version lacks (2: a newer version's); 6: none of it.
    size_t was_len[CASES];
    uint8_t *was[CASES];
    for (int i = 0; i < CASES; i++) {
        CHECK(store_save_page(&nb, i, pg[i]) == 0);
        Page *rd = NULL;
        CHECK(store_read_page(&nb, i, &rd) == 0 && rd && !rd->readonly && rd->n == 8);
        page_free(rd);
        char path[800];
        page_path(&nb, i, path, sizeof path);
        was[i] = file_bytes(path, &was_len[i]);
        CHECK(compact_note_page(nb.id, nb.page_ids[i]) == 0);
    }
    CHECK(bg_compact_pages(nb.id, DPI) == 0);
    for (int i = 0; i < CASES; i++) {
        char path[800];
        page_path(&nb, i, path, sizeof path);
        size_t l;
        uint8_t *now = file_bytes(path, &l);
        bool same = was[i] && now && l == was_len[i] && !memcmp(now, was[i], l);
        if (i < CASES - 1) CHECK(same && lists(&nb, nb.page_ids[i]));
        else CHECK(!same && now && l < was_len[i] && !lists(&nb, nb.page_ids[i]));
        free(now);
        free(was[i]);
        page_free(pg[i]);
    }
    CHECK(store_delete(nb.id) == 0);
}

// The file is replaced only if it still holds what the pass read: a page written meanwhile (by the
// editor, an older version or a copy from a computer) stays as that writer left it, with no
// temporary file left behind, and listed; the next pass compacts what it holds then.
static Page *g_meanwhile;
static void write_meanwhile(const char *path) {
    uint8_t *d;
    size_t len;
    if (page_serialize(g_meanwhile, &d, &len) != 0) return;
    CHECK(write_file_atomic(path, d, len) == 0);
    free(d);
}

TEST(a_page_written_meanwhile_stays) {
    NotebookInfo nb;
    CHECK(store_create("Meanwhile", "", PAPER_MEDIUM, &nb) == 0);
    Page *p = erased_page(PAPER_MEDIUM);
    CHECK(store_save_page(&nb, 0, p) == 0);
    g_meanwhile = erased_page(PAPER_MEDIUM);
    page_add(g_meanwhile, line(TOOL_FOUNTAIN, 300, 1900, 1200, 1950, 0.7f, 41u));
    uint8_t *want;
    size_t want_len;
    CHECK(page_serialize(g_meanwhile, &want, &want_len) == 0);
    char path[800], tmp[900];
    page_path(&nb, 0, path, sizeof path);
    snprintf(tmp, sizeof tmp, "%s.compact.tmp", path);
    CHECK(compact_note_page(nb.id, nb.page_ids[0]) == 0);
    compact_test_before_replace = write_meanwhile;
    CHECK(bg_compact_pages(nb.id, DPI) == 0);
    compact_test_before_replace = NULL;
    size_t l;
    uint8_t *now = file_bytes(path, &l);
    CHECK(now && l == want_len && !memcmp(now, want, l));
    CHECK(!exists(tmp) && lists(&nb, nb.page_ids[0]));
    free(now);
    // The next pass leaves out of it what the writer's page lets go (its new stroke stays).
    Page *rd = NULL;
    CHECK(store_read_page(&nb, 0, &rd) == 0 && rd && rd->n == 9);
    bool drop[16] = {0};
    int expect = rd ? page_droppable(rd, DPI, drop) : -1;
    CHECK(expect >= 3 && !drop[8]);
    page_free(rd);
    CHECK(bg_compact_pages(nb.id, DPI) == 0);
    rd = NULL;
    CHECK(store_read_page(&nb, 0, &rd) == 0 && rd && rd->n == 9 - expect);
    CHECK(rd && rd->n && rd->strokes[rd->n - 1]->tool == TOOL_FOUNTAIN);
    page_free(rd);
    CHECK(!lists(&nb, nb.page_ids[0]));
    free(want);
    page_free(g_meanwhile);
    g_meanwhile = NULL;
    page_free(p);
    CHECK(store_delete(nb.id) == 0);
}

// A stop (bg_cancel_notebook's SIGTERM) that comes while a page file is being replaced waits for
// the rename: the page is its new file whole, no temporary file is left, and the list, not updated
// yet, still names it (the next pass finds nothing more to leave out and lets it go).
static void stop_now(const char *path) {
    (void)path;
    kill(getpid(), SIGTERM);
}

TEST(a_stop_waits_for_the_rename) {
    NotebookInfo nb;
    CHECK(store_create("Stopped", "", PAPER_MEDIUM, &nb) == 0);
    Page *p = erased_page(PAPER_MEDIUM);
    CHECK(store_save_page(&nb, 0, p) == 0);
    Page *rd = NULL;
    CHECK(store_read_page(&nb, 0, &rd) == 0 && rd && rd->n == 8);
    bool drop[16] = {0};
    CHECK(rd && page_droppable(rd, DPI, drop) >= 3);
    for (int i = rd ? rd->n - 1 : -1; i >= 0; i--)
        if (drop[i]) stroke_free(page_remove(rd, i));
    uint8_t *want = NULL;
    size_t want_len = 0;
    CHECK(rd && page_serialize(rd, &want, &want_len) == 0);
    page_free(rd);
    char path[800], tmp[900];
    page_path(&nb, 0, path, sizeof path);
    snprintf(tmp, sizeof tmp, "%s.compact.tmp", path);
    CHECK(compact_note_page(nb.id, nb.page_ids[0]) == 0);
    pid_t pid = fork();
    if (pid == 0) {
        compact_test_before_replace = stop_now;
        bg_compact_pages(nb.id, DPI);
        _exit(0);  // (not reached: the stop lands as soon as the page is replaced)
    }
    int st = 0;
    CHECK(pid > 0 && waitpid(pid, &st, 0) == pid && WIFSIGNALED(st) && WTERMSIG(st) == SIGTERM);
    size_t l;
    uint8_t *now = file_bytes(path, &l);
    CHECK(now && want && l == want_len && !memcmp(now, want, l));
    CHECK(!exists(tmp) && lists(&nb, nb.page_ids[0]));
    free(now);
    CHECK(bg_compact_pages(nb.id, DPI) == 0 && !compact_listed(nb.id));
    now = file_bytes(path, &l);
    CHECK(now && l == want_len && !memcmp(now, want, l));
    free(now);
    free(want);
    page_free(p);
    CHECK(store_delete(nb.id) == 0);
}

// A list that is there but does not read now (a read error, no memory) is never written over: a
// page noted then would have it list that page alone and forget the others. It is left as it is,
// the editor tries again at its next save, and once it reads the page is added to what it lists.
TEST(a_list_that_does_not_read_is_kept) {
    NotebookInfo nb;
    CHECK(store_create("Unread", "", PAPER_MEDIUM, &nb) == 0);
    char lp[800];
    list_path(&nb, lp, sizeof lp);
    const char *t = "# a list\n3\n4\n";
    CHECK(write_file_atomic(lp, t, strlen(t)) == 0);
    CHECK(chmod(lp, 0) == 0);
    FILE *f = fopen(lp, "rb");
    if (f) {
        fclose(f);  // (run as root, which reads it anyway: nothing to see here)
    } else {
        CHECK(compact_note_page(nb.id, 5) == -1);
        CHECK(compact_listed(nb.id));  // it may list pages: the pass is tried
        CHECK(bg_compact_pages(nb.id, DPI) == -1);
    }
    CHECK(chmod(lp, 0644) == 0);
    size_t len;
    char *back = (char *)read_file(lp, &len);
    CHECK(back && !strcmp(back, t));
    free(back);
    CHECK(compact_note_page(nb.id, 5) == 0);
    back = (char *)read_file(lp, &len);
    const char *ids = back ? strchr(back, '\n') : NULL;
    CHECK(ids && !strcmp(ids, "\n3\n4\n5\n"));
    free(back);
    CHECK(store_delete(nb.id) == 0);
}

// The list: one id per line, each once; comments and lines that are not page ids are skipped; a
// list with no ids lists nothing; and a notebook id that could not be one is never touched.
TEST(the_list) {
    NotebookInfo nb;
    CHECK(store_create("Listed", "", PAPER_MEDIUM, &nb) == 0);
    char lp[800];
    list_path(&nb, lp, sizeof lp);
    const char *t = "# a comment\n\n7\n  9 \nx12\n7\n4294967295\n-3\n0\n12abc\n5\r\n";
    CHECK(write_file_atomic(lp, t, strlen(t)) == 0);
    CHECK(compact_listed(nb.id));
    CHECK(compact_note_page(nb.id, 9) == 0 && compact_note_page(nb.id, 8) == 0);
    size_t len;
    char *back = (char *)read_file(lp, &len);
    CHECK(back && back[0] == '#');
    // 7, 9, 5 kept in their order, 8 added at the end, nothing else.
    const char *ids = back ? strchr(back, '\n') : NULL;
    CHECK(ids && !strcmp(ids, "\n7\n9\n5\n8\n"));
    free(back);
    const char *none = "# only a comment\nnothing\n";
    CHECK(write_file_atomic(lp, none, strlen(none)) == 0);
    CHECK(!compact_listed(nb.id));
    CHECK(bg_compact_pages(nb.id, DPI) == 0 && !exists(lp));  // nothing listed: the list goes
    CHECK(!compact_listed("../x") && compact_note_page("../x", 1) != 0 && !compact_listed(""));
    CHECK(store_delete(nb.id) == 0);
}

// At startup every notebook with pages listed is compacted in the background (bgjob.c's child),
// and the page comes out as it would have in the foreground.
TEST(compaction_left_from_an_earlier_run) {
    NotebookInfo nb, other;
    CHECK(store_create("Earlier", "", PAPER_WIDE, &nb) == 0 && store_create("Untouched", "", PAPER_WIDE, &other) == 0);
    Page *p = erased_page(PAPER_WIDE);
    CHECK(store_save_page(&nb, 0, p) == 0 && store_save_page(&other, 0, p) == 0);
    Page *rd = NULL;
    CHECK(store_read_page(&nb, 0, &rd) == 0 && rd);
    bool drop[16] = {0};
    int expect = rd ? page_droppable(rd, DPI, drop) : -1;
    page_free(rd);
    CHECK(expect >= 3 && compact_note_page(nb.id, nb.page_ids[0]) == 0);
    Canvas before, after;
    int n0 = 0, n1 = 0, n2 = 0;
    render_file(&nb, 0, PAGE_LAYER_ALL, &before, &n0);
    CHECK(compact_queue_listed(DPI) == 1);
    for (int ms = 0; bg_busy() && ms < 60000; ms += 10) {
        bg_poll();
        sleep_ms(10);
    }
    CHECK(!bg_busy());
    render_file(&nb, 0, PAGE_LAYER_ALL, &after, &n1);
    CHECK(n0 == 8 && n1 == 8 - expect && same_px(&before, &after));
    canvas_free(&after);
    render_file(&other, 0, PAGE_LAYER_ALL, &after, &n2);
    CHECK(n2 == 8 && same_px(&before, &after));
    CHECK(!compact_listed(nb.id) && compact_queue_listed(DPI) == 0);
    canvas_free(&before);
    canvas_free(&after);
    page_free(p);
    CHECK(store_delete(nb.id) == 0 && store_delete(other.id) == 0);
}

// A sketch reworked under scrubs (page.h: page_droppable takes seconds on it), with strokes to
// leave out.
static Page *heavy_page(void) {
    Page *p = page_new(W, H, PAPER_MEDIUM);
    for (int i = 0; i < 360; i++)
        page_add(p, line(TOOL_PENCIL, 100 + (i % 7) * 20, 100 + i * 6, 1700 - (i % 5) * 30, 140 + i * 6, 0.8f,
                         100u + (uint32_t)i));
    for (int k = 0; k < 20; k++) page_add(p, rubber(150, 150 + k * 60, 1650, 1300 + k * 50, 1.f, 7000u + (uint32_t)k));
    return p;
}

static void signal_noted(int sig) { (void)sig; }

// Opening a notebook stops its compaction at once, in the middle of a page, on the Kindle too:
// there Grimoire's SIGTERM handler (input_evdev.c) only notes the signal, and bgjob.c's child is a
// fork that keeps it, so a pass that kept it would run to its end while Grimoire waits in
// bg_cancel_notebook (and the launcher's watchdog kills it after 15 s). The page it was at is as it
// was, still listed, with no temporary file; the next pass compacts it.
TEST(opening_stops_a_running_pass) {
    NotebookInfo nb;
    CHECK(store_create("Heavy", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_MEDIUM) == 0);
    Page *light = page_new(W, H, PAPER_MEDIUM);
    page_add(light, line(TOOL_PENCIL, 300, 300, 900, 900, 0.7f, 21u));
    Page *heavy = heavy_page();
    CHECK(store_save_page(&nb, 0, light) == 0 && store_save_page(&nb, 1, heavy) == 0);
    char path[800], tmp[900];
    page_path(&nb, 1, path, sizeof path);
    snprintf(tmp, sizeof tmp, "%s.compact.tmp", path);
    size_t was_len;
    uint8_t *was = file_bytes(path, &was_len);
    CHECK(compact_note_page(nb.id, nb.page_ids[0]) == 0 && compact_note_page(nb.id, nb.page_ids[1]) == 0);
    CHECK(!bg_busy());
    struct sigaction sa, old;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = signal_noted;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGTERM, &sa, &old);
    bg_queue(BG_COMPACT_PAGES, nb.id, NULL, DPI, false);
    // Once the light page is done (off the list), the pass is in the heavy one.
    for (int ms = 0; lists(&nb, nb.page_ids[0]) && ms < 60000; ms += 2) sleep_ms(2);
    CHECK(!lists(&nb, nb.page_ids[0]) && lists(&nb, nb.page_ids[1]));
    uint32_t t0 = now_ms();
    bg_cancel_notebook(nb.id);
    uint32_t took = now_ms() - t0;
    sigaction(SIGTERM, &old, NULL);
    CHECK(took < 1000);
    CHECK(!bg_busy());
    size_t l;
    uint8_t *now = file_bytes(path, &l);
    CHECK(was && now && l == was_len && !memcmp(now, was, l));
    CHECK(lists(&nb, nb.page_ids[1]) && !exists(tmp));
    free(now);
    // The next pass gets through, and finds ink to leave out (so the stop above came in time).
    CHECK(bg_compact_pages(nb.id, DPI) == 0 && !compact_listed(nb.id));
    Page *rd = NULL;
    CHECK(store_read_page(&nb, 1, &rd) == 0 && rd && rd->n < heavy->n);
    page_free(rd);
    free(was);
    page_free(light);
    page_free(heavy);
    CHECK(store_delete(nb.id) == 0);
}

int main(void) {
    snprintf(root, sizeof root, "%s/inkwire-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    store_set_device(DPI, W, H);
    RUN(pen_stroke_follows_the_settings);
    RUN(eraser_size_follows_the_zoom);
    RUN(settings_round_trip);
    RUN(live_erase_equals_rerender);
    RUN(compaction_takes_erased_ink_out);
    RUN(compaction_leaves_what_it_cannot_write);
    RUN(newer_data_is_left_as_it_is);
    RUN(a_page_written_meanwhile_stays);
    RUN(a_stop_waits_for_the_rename);
    RUN(a_list_that_does_not_read_is_kept);
    RUN(the_list);
    RUN(compaction_left_from_an_earlier_run);
    RUN(opening_stops_a_running_pass);
    int rc = tl_done();
    system(cmd);
    return rc;
}
