// test_tplsheet.c: the Templates sheet (src/tplsheet.h): the paper pickers' grid, its pages of
// tiles and what stays in view through a turn, picking, Back and a tap outside, tiles that wait for
// their previews, and which tile is the page's paper.
#include <dirent.h>
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
    // The previews job ends: the tiles fill in, in natural order; the broken file is offered no more
    // but keeps its tile (the last) while the sheet is open.
    CHECK(drain_jobs() == BG_TEMPLATE);
    Rect d;
    CHECK(tplsheet_tick(&s, &a, now_ms(), &d) && !rect_is_empty(d));
    CHECK(s.n == 51 && s.t[s.shown[50]].bad && !s.pending && !strcmp(s.t[s.shown[1]].name, "page-2") &&
          !strcmp(s.t[s.shown[9]].name, "page-10"));
    probe(&s, "count", v);
    CHECK(!strcmp(v, "50"));
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

// A file the previews job finds it cannot use, sorted before the page shown: the tiles on that page
// stay where they were (it keeps its tile, crossed out, until the sheet opens again).
TEST(bad_file_keeps_its_place) {
    char cmd[800];
    snprintf(cmd, sizeof cmd, "rm -rf '%s/templates'", root);
    CHECK(system(cmd) == 0);
    templates_init();
    char path[700], name[64];
    snprintf(path, sizeof path, "%s/templates/0 broken.png", root);
    CHECK(write_file_atomic(path, "no picture", 10) == 0);
    for (int n = 1; n <= 40; n++) {
        snprintf(name, sizeof name, "t%d.png", n);
        put_template(n, name);
    }
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 1860, 2480);
    TplSheet s;
    memset(&s, 0, sizeof s);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    Rect d;
    const Hit *h = find_hit(&s, TS_HIT_NEXT, 0);
    CHECK(h && tap(&s, &a, h->r, &d) == TS_CHANGED);
    char before[64], v[64];
    snprintf(before, sizeof before, "%s", s.t[s.shown[24]].name);  // page 2's first tile
    CHECK(!strcmp(before, "t24"));
    CHECK(drain_jobs() == BG_TEMPLATE);
    CHECK(tplsheet_tick(&s, &a, now_ms(), &d));
    probe(&s, "page", v);
    CHECK(!strcmp(v, "2") && !strcmp(s.t[s.shown[24]].name, before) && find_hit(&s, TS_HIT_TILE, 24));
    probe(&s, "count", v);
    CHECK(!strcmp(v, "40") && s.n == 41 && s.t[s.shown[0]].bad && !s.pending);
    // Back on page 1: the file's tile, which cannot be picked; opened again, it is gone.
    h = find_hit(&s, TS_HIT_PREV, 0);
    CHECK(h && tap(&s, &a, h->r, &d) == TS_CHANGED);
    CHECK(!find_hit(&s, TS_HIT_TILE, 0) && find_hit(&s, TS_HIT_TILE, 1));
    tplsheet_close(&s);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    CHECK(s.n == 40 && !strcmp(s.t[s.shown[0]].name, "t1") && find_hit(&s, TS_HIT_TILE, 0) && !bg_busy());
    tplsheet_close(&s);
    canvas_free(&a.screen);
}

static int count_previews(void) {
    char dir[700];
    snprintf(dir, sizeof dir, "%s/templates/.cache", root);
    DIR *d = opendir(dir);
    int n = 0;
    struct dirent *e;
    while (d && (e = readdir(d))) {
        size_t l = strlen(e->d_name);
        n += l > 4 && !strcmp(e->d_name + l - 4, ".pgm");
    }
    if (d) closedir(d);
    return n;
}

// While the previews job runs (here: its child has not been reaped, so it still counts as running),
// the sheet looks for the previews made so far every 1.5 s, and fills their tiles in before the job
// is over; not sooner.
TEST(fills_in_while_the_job_runs) {
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 1860, 2480);
    put_template(88, "t0.png");  // first in natural order, no preview yet
    int had = count_previews();
    TplSheet s;
    memset(&s, 0, sizeof s);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    tplsheet_draw(&s, &a);
    CHECK(bg_busy() && s.pending && !s.t[s.shown[0]].ready && !find_hit(&s, TS_HIT_TILE, 0));
    uint32_t until = now_ms() + 30000;
    while (count_previews() == had && (int32_t)(now_ms() - until) < 0) usleep(5000);
    CHECK(count_previews() == had + 1);
    Rect d;
    unsigned gen = bgimage_generation();
    CHECK(!tplsheet_tick(&s, &a, s.next_check - 1, &d) && !s.t[s.shown[0]].ready);
    CHECK(tplsheet_tick(&s, &a, s.next_check, &d) && s.t[s.shown[0]].ready && !s.pending);
    CHECK(find_hit(&s, TS_HIT_TILE, 0) && bg_busy() && bgimage_generation() == gen);
    drain_jobs();
    tplsheet_close(&s);
    canvas_free(&a.screen);
}

// The columns that hold ink (a pixel under 128) in r of c: *x0 the first, *x1 the last (-1, -1
// for none).
static void ink_columns(const Canvas *c, Rect r, int *x0, int *x1) {
    *x0 = *x1 = -1;
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++)
            if (c->px[(size_t)y * (size_t)c->stride + (size_t)x] < 128) {
                if (*x0 < 0 || x < *x0) *x0 = x;
                if (x > *x1) *x1 = x;
            }
}

// UI-11: a paper tile's label never reaches past its tile (one wider than the tile goes on two
// lines under it), so labels side by side never run into each other; a selected one, in bold,
// stays within its column. In every language and style, in each picker's grid upright and
// sideways.
TEST(labels_keep_to_their_tiles) {
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    PaperGrid g[6];
    a.w = 1860, a.h = 2480;
    paper_sheet_rect(&a, &g[0]);
    a.w = 2480, a.h = 1860;
    paper_sheet_rect(&a, &g[1]);
    static const int box[4][2] = {{1640, 1590}, {1648, 1608}, {1720, 1070}, {1728, 1088}};  // New notebook's
    for (int i = 0; i < 4; i++) g[2 + i] = paper_grid(PAPER_SHEET_TILES, box[i][0], box[i][1], 30, 100, 300, PAPER_GRID_MIN_COLS);
    Canvas c;
    CHECK(canvas_init(&c, 700, 600) == 0);
    int gap = ui_px(30), wrapped = 0;
    for (int l = 0; l < i18n_language_count(); l++) {
        CHECK(i18n_set(i18n_language_code(l)));
        for (int th = 0; th < 2; th++) {
            ui_set_theme(th ? THEME_CLASSIC : THEME_GOTHIC);
            for (int k = 0; k < 6; k++) {
                Rect tr = {200, 20, g[k].tw, g[k].th};
                for (int i = 0; i < PAPER_SHEET_TILES; i++) {
                    const char *label = i < PAPER_COUNT    ? paper_label(paper_at(i))
                                        : i == PAPER_COUNT ? _("Custom music...")
                                                           : _("More papers...");
                    for (int sel = 0; sel < 2; sel++) {
                        canvas_fill(&c, (Rect){0, 0, c.w, c.h}, 255);
                        paper_tile_label(&c, tr, gap, label, sel);
                        int x0, x1, room = sel ? gap / 2 : ui_px(2);
                        ink_columns(&c, (Rect){0, tr.y + tr.h, c.w, c.h - tr.y - tr.h}, &x0, &x1);
                        bool in = x0 >= tr.x - room && x1 < tr.x + tr.w + room;
                        if (!in)
                            fprintf(stderr, "  (%s, %s, a %d px tile%s: \"%s\" inked from %d to %d)\n",
                                    i18n_language_code(l), th ? "classic" : "gothic", tr.w, sel ? ", bold" : "", label,
                                    x0 - tr.x, x1 - tr.x);
                        CHECK(x0 >= 0 && in);
                        wrapped += !sel && font_measure(FONT_REGULAR, ui_small_size() * 0.9f, label) > tr.w;
                    }
                }
            }
        }
    }
    CHECK(wrapped > 0);  // (some labels are wider than their tiles: the test sees two lines)
    i18n_set("en");
    ui_set_theme(THEME_GOTHIC);
    canvas_free(&c);
}

// The empty state with files it cannot use: its heading, then three paragraphs (how to add
// templates, what works best, how many files it cannot use), each starting as far under the one
// before it whatever their numbers of lines, never as close as a line of it, nor far under the
// heading. In every language and style, upright and sideways.
TEST(empty_state_paragraphs) {
    char cmd[800];
    snprintf(cmd, sizeof cmd, "rm -rf '%s/templates'", root);
    CHECK(system(cmd) == 0);
    templates_init();
    char path[700];
    snprintf(path, sizeof path, "%s/templates/Scan.png", root);
    CHECK(write_file_atomic(path, "no picture", 10) == 0);
    snprintf(path, sizeof path, "%s/templates/Photo.jpg", root);
    CHECK(write_file_atomic(path, "no picture either", 17) == 0);
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = DPI;
    screen(&a, 1860, 2480);
    TplSheet s;
    memset(&s, 0, sizeof s);
    tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
    drain_jobs();
    tplsheet_close(&s);
    for (int l = 0; l < i18n_language_count(); l++) {
        CHECK(i18n_set(i18n_language_code(l)));
        for (int th = 0; th < 2; th++) {
            ui_set_theme(th ? THEME_CLASSIC : THEME_GOTHIC);
            for (int turn = 0; turn < 2; turn++) {
                screen(&a, turn ? 2480 : 1860, turn ? 1860 : 2480);
                canvas_fill(&a.screen, (Rect){0, 0, a.w, a.h}, 255);
                tplsheet_open(&s, &a, NULL, PAPER_BLANK, NULL);
                tplsheet_draw(&s, &a);
                char v[64];
                probe(&s, "count", v);
                CHECK(!strcmp(v, "0") && s.nt == 2 && s.hits.n == 1);
                // The text's column: the sheet's inside (ui_sheet says where), above Back.
                Canvas scratch;
                CHECK(canvas_init(&scratch, a.w, a.h) == 0);
                Rect in = ui_sheet(&scratch, s.rect, _("Templates"));
                canvas_free(&scratch);
                Rect col = {in.x + ui_px(40), in.y, in.w - 2 * ui_px(40), in.h - ui_px(160)};
                // Its bands of rows with ink (a few blank rows inside one, as under an accent,
                // do not end it), and the blank between each and the next.
                int gaps[64], ng = 0, last = -1, bands = 0;
                for (int y = col.y; y < col.y + col.h; y++) {
                    int x0, x1;
                    ink_columns(&a.screen, (Rect){col.x, y, col.w, 1}, &x0, &x1);
                    if (x0 < 0) continue;
                    if (last < 0 || y - last > ui_px(6)) {
                        if (last >= 0 && ng < 64) gaps[ng++] = y - last - 1;
                        bands++;
                    }
                    last = y;
                }
                int wide = 0, widest = 0;
                for (int i = 0; i < ng; i++) {
                    wide += gaps[i] >= ui_px(45);  // (40 between the paragraphs, and the lines' own room)
                    if (gaps[i] > widest) widest = gaps[i];
                }
                if (wide != 3 || widest > ui_px(120)) {
                    fprintf(stderr, "  (%s, %s, %s: %d bands, gaps", i18n_language_code(l), th ? "classic" : "gothic",
                            turn ? "sideways" : "upright", bands);
                    for (int i = 0; i < ng; i++) fprintf(stderr, " %d", gaps[i]);
                    fprintf(stderr, ")\n");
                }
                CHECK(wide == 3 && widest <= ui_px(120));
                tplsheet_close(&s);
            }
        }
    }
    i18n_set("en");
    ui_set_theme(THEME_GOTHIC);
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
    RUN(bad_file_keeps_its_place);
    RUN(fills_in_while_the_job_runs);
    RUN(labels_keep_to_their_tiles);
    RUN(empty_state_paragraphs);
    font_shutdown();
    int rc = tl_done();
    if (!rc) system(cmd);
    return rc;
}
