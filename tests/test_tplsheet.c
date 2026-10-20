// test_tplsheet.c: the Templates sheet (src/tplsheet.h): the paper pickers' grid, its pages of
// tiles and what stays in view through a turn, picking, Back and a tap outside, tiles that wait for
// their previews, and which tile is the page's paper.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "bgjob.h"
#include "testlib.h"
#include "tplsheet.h"

static char root[600];
static const float DPI = 300.f;

// ---- helpers ----

static int drain_jobs(void) {
    int last = -1;
    uint32_t until = now_ms() + 60000;
    while (bg_busy() && (int32_t)(now_ms() - until) < 0) {
        int kind = -1;
        bg_poll_kind(&kind);
        if (kind >= 0) last = kind;
        if (kind == BG_TEMPLATE) bgimage_retry_missing();  // (as app.c does when one ends)
        usleep(5000);
    }
    int kind = -1;
    bg_poll_kind(&kind);
    if (kind == BG_TEMPLATE) bgimage_retry_missing();
    return kind >= 0 ? kind : last;
}

// Template n: a gray picture of n lines, distinct for every n.
static void put_template(int n, const char *name) {
    BgPicture p = {malloc(60 * 80), 60, 80, 1};
    memset(p.px, 255, 60 * 80);
    for (int y = 5; y < 80; y += 3 + n % 11) memset(p.px + y * 60, (uint8_t)(100 + n % 50), 60);
    p.px[n % 60] = 0;
    char path[700];
    snprintf(path, sizeof path, "%s/templates/%s", root, name);
    CHECK(bgimage_write_png(path, &p) == 0);
    bgpicture_free(&p);
}

static void screen(App *a, int w, int h) {
    canvas_free(&a->screen);
    a->w = w;
    a->h = h;
    CHECK(canvas_init(&a->screen, w, h) == 0);
}

static const Hit *find_hit(const TplSheet *s, int id, int arg) {
    for (int i = 0; i < s->hits.n; i++)
        if (s->hits.h[i].id == id && (id != TS_HIT_TILE || s->hits.h[i].arg == arg)) return &s->hits.h[i];
    return NULL;
}

// A tap at the middle of r (down, then up), as the hosts pass them on.
static TsResult tap(TplSheet *s, App *a, Rect r, Rect *dirty) {
    float x = (float)(r.x + r.w / 2), y = (float)(r.y + r.h / 2);
    tplsheet_pointer(s, a, 0, x, y, 1000, dirty);
    return tplsheet_pointer(s, a, 2, x, y, 1080, dirty);
}

static void probe(const TplSheet *s, const char *key, char *out) {
    CHECK(tplsheet_probe(s, key, out, 64) == 0);
}

// ---- the grid ----

TEST(paper_grid_layouts) {
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    PaperGrid g;
    // The editor's Paper sheet (and the Templates sheet in its place): upright the papers keep their
    // 6 columns, More papers... a row of its own; sideways 7 columns keep them as big as before.
    a.w = 1860, a.h = 2480;
    Rect r = paper_sheet_rect(&a, &g);
    CHECK(g.cols == 6 && g.rows == 4 && g.tw == 249 && g.th == 332);
    CHECK(r.y >= 0 && r.y + r.h <= a.h && r.x >= 0 && r.x + r.w <= a.w);
    a.w = 2480, a.h = 1860;
    r = paper_sheet_rect(&a, &g);
    CHECK(g.cols == 7 && g.rows == 3 && g.tw == 274 && g.th == 365);
    CHECK(r.y >= 0 && r.y + r.h <= a.h);
    // New notebook's tiles in the room draw_new gives them (Gothic and Classic): upright 6 columns,
    // sideways 7, at the size 6 had before More papers... came (its labels' room stays 222 px).
    struct {
        int w, h, cols, rows, tw;
    } nb[] = {{1640, 1590, 6, 4, 222}, {1648, 1608, 6, 4, 226}, {1720, 1070, 7, 3, 192}, {1728, 1088, 7, 3, 196}};
    for (size_t i = 0; i < sizeof nb / sizeof nb[0]; i++) {
        g = paper_grid(PAPER_SHEET_TILES, nb[i].w, nb[i].h, 30, 100, 300, PAPER_GRID_MIN_COLS);
        CHECK(g.cols == nb[i].cols && g.rows == nb[i].rows && g.tw == nb[i].tw);
    }
    // Any box: the tiles fit it, and no other number of columns (at least the minimum) makes them
    // bigger; of those as big, none leaves fewer places empty.
    uint32_t rng = 7;
    for (int t = 0; t < 400; t++) {
        rng = rng * 1103515245u + 12345u;
        int n = 1 + (int)(rng >> 16) % 40, aw = 300 + (int)(rng >> 8) % 2500, ah = 300 + (int)(rng >> 4) % 2500;
        int min = 1 + t % 7;
        g = paper_grid(n, aw, ah, 30, 100, 300, min);
        CHECK(g.cols >= 1 && g.rows >= 1 && g.cols * g.rows >= n && g.tw >= 1 && g.tw <= 300);
        CHECK(g.cols >= (min < n ? min : n));
        if (g.tw > 1) {
            CHECK(g.cols * g.tw + (g.cols - 1) * 30 <= aw && g.rows * g.cell_h <= ah);
            for (int c = min < n ? min : n; c <= n; c++) {
                PaperGrid o = paper_grid(n, aw, ah, 30, 100, 300, c);
                if (o.cols != c) continue;
                CHECK(o.tw <= g.tw);
                if (o.tw == g.tw) CHECK(o.rows * o.cols - n >= g.rows * g.cols - n);
            }
        }
    }
}

// ---- the sheet ----

TEST(pages_of_tiles) {
    templates_init();
    char name[64];
    for (int n = 1; n <= 50; n++) {
        snprintf(name, sizeof name, "page-%d.png", n);
        put_template(n, name);
    }
    char path[700];
    snprintf(path, sizeof path, "%s/templates/zz-broken.png", root);
    CHECK(write_file_atomic(path, "no picture", 10) == 0);
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 1860, 2480);
    TplSheet s;
    memset(&s, 0, sizeof s);
    // Opened before any preview is made: every tile waits (none can be picked), and none is read.
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    CHECK(s.open && s.nt == 51 && s.n == 51 && s.pending);  // (the broken file is not known bad yet)
    tplsheet_draw(&s, &a);
    char v[64];
    probe(&s, "count", v);
    CHECK(!strcmp(v, "51"));
    probe(&s, "ready", v);
    CHECK(!strcmp(v, "0"));
    CHECK(!find_hit(&s, TS_HIT_TILE, 0) && find_hit(&s, TS_HIT_BACK, 0) && find_hit(&s, TS_HIT_NEXT, 0));
    // The previews job ends: the tiles fill in, the broken file goes, in natural order.
    CHECK(drain_jobs() == BG_TEMPLATE);
    Rect d;
    CHECK(tplsheet_tick(&s, &a, now_ms(), &d) && !rect_is_empty(d));
    CHECK(s.n == 50 && !s.pending && !strcmp(s.t[s.shown[1]].name, "page-2") && !strcmp(s.t[s.shown[9]].name, "page-10"));
    probe(&s, "ready", v);
    CHECK(!strcmp(v, "50"));
    probe(&s, "pages", v);
    CHECK(!strcmp(v, "3"));  // 24 a page upright
    CHECK(find_hit(&s, TS_HIT_TILE, 0) && find_hit(&s, TS_HIT_TILE, 23) && !find_hit(&s, TS_HIT_TILE, 24));
    CHECK(!find_hit(&s, TS_HIT_PREV, 0));
    CHECK(s.t[s.shown[0]].thumb.px && !s.t[s.shown[30]].thumb.px);  // only the tiles on screen are read
    // The next page; a turn keeps its first tile in view (sideways 21 a page: page 2 of 3 holds 24).
    const Hit *h = find_hit(&s, TS_HIT_NEXT, 0);
    CHECK(h && tap(&s, &a, h->r, &d) == TS_CHANGED);
    probe(&s, "page", v);
    CHECK(!strcmp(v, "2") && find_hit(&s, TS_HIT_TILE, 24) && find_hit(&s, TS_HIT_PREV, 0));
    CHECK(!s.t[s.shown[0]].thumb.px && s.t[s.shown[24]].thumb.px);  // the page before let go of
    screen(&a, 2480, 1860);
    tplsheet_draw(&s, &a);
    probe(&s, "page", v);
    CHECK(!strcmp(v, "2") && s.per == 21 && find_hit(&s, TS_HIT_TILE, 21) && find_hit(&s, TS_HIT_TILE, 24));
    h = find_hit(&s, TS_HIT_NEXT, 0);
    CHECK(h && tap(&s, &a, h->r, &d) == TS_CHANGED);
    probe(&s, "page", v);
    CHECK(!strcmp(v, "3") && find_hit(&s, TS_HIT_TILE, 49) && !find_hit(&s, TS_HIT_NEXT, 0));
    // Back upright: the page that holds the first tile shown sideways (42: page 2).
    screen(&a, 1860, 2480);
    tplsheet_draw(&s, &a);
    probe(&s, "page", v);
    CHECK(!strcmp(v, "2") && find_hit(&s, TS_HIT_TILE, 42));
    // Picking a tile: its file and preview, until tplsheet_close.
    h = find_hit(&s, TS_HIT_TILE, 30);
    CHECK(h && tap(&s, &a, h->r, &d) == TS_PICKED && !s.open);
    snprintf(path, sizeof path, "%s/templates/page-31.png", root);
    CHECK(tplsheet_path(&s) && !strcmp(tplsheet_path(&s), path) && tplsheet_preview(&s));
    probe(&s, "count", v);
    CHECK(!strcmp(v, ""));  // (closed)
    tplsheet_close(&s);
    CHECK(!tplsheet_path(&s) && !s.t && !s.shown);
    // Back, and a tap outside the sheet.
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    CHECK(s.n == 50 && !s.pending && !bg_busy());  // (all made: no job queued again)
    h = find_hit(&s, TS_HIT_BACK, 0);
    CHECK(h && tap(&s, &a, h->r, &d) == TS_BACK && !s.open && !s.t);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    Rect out = {s.rect.x + s.rect.w / 2, s.rect.y + s.rect.h + 20, 4, 4};
    CHECK(tap(&s, &a, out, &d) == TS_CANCEL && !s.open && rect_overlaps(d, s.rect));
    // In New notebook the template chosen before is selected, on its page.
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, path);
    tplsheet_draw(&s, &a);
    probe(&s, "selected", v);
    CHECK(!strcmp(v, "page-31"));
    probe(&s, "page", v);
    CHECK(!strcmp(v, "2"));
    tplsheet_close(&s);
    canvas_free(&a.screen);
}

// A template copied in after the previews were made waits for its own, then fills in; the previews
// job's other tiles stay as they are meanwhile.
TEST(tile_fills_in) {
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 1860, 2480);
    put_template(77, "page-0.png");  // first in natural order
    TplSheet s;
    memset(&s, 0, sizeof s);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    CHECK(s.pending && !s.t[s.shown[0]].ready && s.t[s.shown[1]].ready);
    CHECK(!find_hit(&s, TS_HIT_TILE, 0) && find_hit(&s, TS_HIT_TILE, 1));
    Rect d;
    CHECK(!tplsheet_tick(&s, &a, now_ms(), &d) || s.pending);  // (made only once the job has run)
    drain_jobs();
    CHECK(tplsheet_tick(&s, &a, now_ms(), &d) && !s.pending && find_hit(&s, TS_HIT_TILE, 0));
    CHECK(!tplsheet_tick(&s, &a, now_ms() + 5000, &d));  // nothing more to wait for
    tplsheet_close(&s);
    canvas_free(&a.screen);
}

// The tile that is the open page's paper: the template it was installed from, by the notebook's
// index of its pictures, on its page of tiles. Never another notebook's, nor a document page.
TEST(page_paper_selected) {
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 1860, 2480);
    NotebookInfo nb, other;
    CHECK(store_create("Planner", "", PAPER_MEDIUM, &nb) == 0 && store_create("Other", "", PAPER_MEDIUM, &other) == 0);
    char path[700];
    snprintf(path, sizeof path, "%s/templates/page-40.png", root);
    PaperKind k, k2;
    const char *why;
    CHECK(template_install(&nb, path, &k, &why) == 0 && paper_is_image(k));
    snprintf(path, sizeof path, "%s/templates/page-3.png", root);
    CHECK(template_install(&other, path, &k2, &why) == 0);
    drain_jobs();
    TplSheet s;
    memset(&s, 0, sizeof s);
    tplsheet_open(&s, &a, &nb, k, NULL);
    tplsheet_draw(&s, &a);
    char v[64];
    probe(&s, "selected", v);
    CHECK(!strcmp(v, "page-40"));
    probe(&s, "page", v);
    CHECK(!strcmp(v, "2"));  // (page-0, page-1 ... page-40 is the 41st)
    tplsheet_close(&s);
    PaperKind not_here[] = {k2, paper_image(PAPER_ROLE_DOCUMENT, paper_image_id(k)), PAPER_MEDIUM};
    for (int i = 0; i < 3; i++) {
        tplsheet_open(&s, &a, &nb, not_here[i], NULL);
        tplsheet_draw(&s, &a);
        probe(&s, "selected", v);
        CHECK(!strcmp(v, "") && s.page == 0);
        tplsheet_close(&s);
    }
    canvas_free(&a.screen);
}

// No templates at all (and no folder): the sheet says how to add some, with only Back to tap.
TEST(empty_sheet) {
    char cmd[800];
    snprintf(cmd, sizeof cmd, "rm -rf '%s/templates'", root);
    CHECK(system(cmd) == 0);
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 2480, 1860);
    TplSheet s;
    memset(&s, 0, sizeof s);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    char v[64];
    probe(&s, "count", v);
    CHECK(s.open && !strcmp(v, "0") && s.hits.n == 1 && s.hits.h[0].id == TS_HIT_BACK && !bg_busy());
    probe(&s, "pages", v);
    CHECK(!strcmp(v, "1"));
    tplsheet_close(&s);
    templates_init();  // (the folder again, for whatever runs after)
    canvas_free(&a.screen);
}

int main(void) {
    snprintf(root, sizeof root, "%s/tplsheet-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    store_set_device(DPI, 1860, 2480);
    ui_init(DPI);
    CHECK(font_init() == 0);
    RUN(paper_grid_layouts);
    RUN(pages_of_tiles);
    RUN(tile_fills_in);
    RUN(page_paper_selected);
    RUN(empty_sheet);
    font_shutdown();
    int rc = tl_done();
    if (!rc) system(cmd);
    return rc;
}
