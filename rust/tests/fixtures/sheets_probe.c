// sheets_probe.c: draws the screens of src/musicbuild.c, src/tplsheet.c, src/pagesview.c and
// src/pensettings.c with fixed inputs and prints one hash of every screen, for comparing the Rust
// ports (rust/tests/test_sheets_probe.rs) with this C build (clang -ffp-contract=on, as the
// Makefile's test flags). A mismatch of one pixel changes the hash, so the layouts, the
// translations' fit, the fonts' placement, the floating point of the pressure curve and the
// readout, the brush on the test pad and the slider drags are all held to the C.
//
// It includes pensettings.c itself, which keeps its state and its helpers private. Everything the
// screens ask of app.c, the display and the input is stubbed here to do nothing, except that
// app_keyboard_pointer draws the keyboard as app.c's does: the Rust twin runs app.rs's real ones,
// which draw nothing else on a->screen either (but a toast, and the pressed-button feedback: the
// probe never taps a button that gives one). Inputs are made with integer arithmetic and exact
// float conversions, so the Rust twin builds the very same events.
//
// To record rust/tests/fixtures/sheets_probe_c.txt again (the machine's libm decides powf and tanf
// in the pen's brush and its readout, so the answers are the recording machine's: Apple's on the
// owner's Mac), from the repository's top level:
//   B=/tmp/sheets-build
//   make BUILD=$B $B/gen/fonts.c $B/gen/l10n.c $B/gen/eraser_sample.c
//   clang -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -I$B/gen -DGRIMOIRE_VERSION='"x"' -O1 -g \
//     -fsanitize=address,undefined -fno-sanitize-recover=undefined -ffp-contract=on \
//     rust/tests/fixtures/sheets_probe.c <the Makefile's CORE_SRC and UI_SRC> src/pagesview.c \
//     src/musicbuild.c $B/gen/fonts.c $B/gen/l10n.c $B/gen/eraser_sample.c -lm
//   ./a.out /tmp/sheets-root > rust/tests/fixtures/sheets_probe_c.txt
// (src/pensettings.c is not in that list: the probe includes it.) The run is deterministic: twice
// gives the same text. Its stderr is the app's log (the background jobs'), not part of the record.
// rust/tests/fixtures/sheets_probe_c_tilt.txt is the "penset events" section of the same build with
// -DPROBE_TILT added (a pen that reports tilt, as the headless runner's does: the Rust twin's sim
// build), the only section the pen's tilt changes:
//   ./a.out /tmp/sheets-root | awk '/^== /{on = ($0 == "== penset events")} on' \
//     > rust/tests/fixtures/sheets_probe_c_tilt.txt
#include "pensettings.c"

#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "annotate.h"
#include "bgimage.h"
#include "bgjob.h"
#include "keyboard.h"
#include "musicbuild.h"
#include "nav.h"
#include "pagesview.h"
#include "tplsheet.h"

// (pensettings.c's S is a->ps; the probe has the one App)
#undef S
#define S (app.ps)

typedef uint64_t u64;
#define H0 14695981039346656037ull

static u64 fnv(u64 h, const uint8_t *p, size_t n) {
    for (size_t i = 0; i < n; i++) {
        h ^= p[i];
        h *= 1099511628211ull;
    }
    return h;
}
static u64 fnv_u32(u64 h, uint32_t v) {
    uint8_t b[4] = {(uint8_t)v, (uint8_t)(v >> 8), (uint8_t)(v >> 16), (uint8_t)(v >> 24)};
    return fnv(h, b, 4);
}
static u64 canvas_hash(const Canvas *c) {
    u64 h = H0;
    h = fnv_u32(h, (uint32_t)c->w);
    h = fnv_u32(h, (uint32_t)c->h);
    for (int y = 0; y < c->h; y++) h = fnv(h, c->px + (size_t)y * (size_t)c->stride, (size_t)c->w);
    return h;
}
static void emit(const char *label, const App *a) { printf("%s %016llx\n", label, (unsigned long long)canvas_hash(&a->screen)); }

// ---- what the screens reach for in app.c, the display and the input ----

void app_invalidate(App *a, Rect r, RefreshMode m) { (void)a; (void)r; (void)m; }
void app_invalidate_all(App *a, RefreshMode m) { (void)a; (void)m; }
void app_flush(App *a) { (void)a; }
void app_toast(App *a, const char *msg, int ms) { (void)a; (void)msg; (void)ms; }
void app_switch(App *a, ScreenId s) { (void)a; (void)s; }
void app_press_feedback(App *a, Rect r) { (void)a; (void)r; }
int app_pointer_phase(const InEvent *ev) {
    switch (ev->type) {
    case IN_PEN_DOWN:
    case IN_TOUCH_DOWN: return 0;
    case IN_PEN_MOVE:
    case IN_TOUCH_MOVE: return 1;
    case IN_PEN_UP:
    case IN_TOUCH_UP: return 2;
    default: return -1;
    }
}
// app.c's, without the refreshes: what it draws is the same.
KbResult app_keyboard_pointer(App *a, Keyboard *kb, int phase, const InEvent *ev) {
    Rect d;
    KbResult r = keyboard_pointer(kb, phase, ev->x, ev->y, ev->t_ms, &d);
    if (r == KB_DONE || r == KB_CANCEL) return r;
    if (rect_is_empty(d) && rect_is_empty(kb->field_dirty)) return r;
    keyboard_draw(kb, &a->screen, (Rect){0, 0, a->w, a->h});
    kb->settle_at = now_ms() + 900;
    return r;
}
void app_keyboard_tick(App *a, Keyboard *kb, uint32_t now) { (void)a; (void)kb; (void)now; }
void display_live_ink(bool on) { (void)on; }
const InputCaps *input_caps(void) {
    static InputCaps c;  // (a pen that reports no tilt, as the Rust twin's host platform says)
#ifdef PROBE_TILT
    c.has_tilt = true;  // (the headless runner's pen reports tilt: the Rust twin's sim build)
#endif
    return &c;
}

// ---- helpers ----

static char root[600];
static App app;

static void screen(int w, int h) {
    canvas_free(&app.screen);
    app.w = w;
    app.h = h;
    if (canvas_init(&app.screen, w, h) != 0) abort();
}
static const char *tag(int turn, int th) { return turn ? (th ? "lc" : "lg") : (th ? "pc" : "pg"); }
static void look(int th) { ui_set_theme(th ? THEME_CLASSIC : THEME_GOTHIC); }

static int drain_jobs(void) {
    uint32_t until = now_ms() + 60000;
    while (bg_busy() && (int32_t)(now_ms() - until) < 0) {
        int kind = -1;
        bg_poll_kind(&kind);
        if (kind == BG_TEMPLATE) bgimage_retry_missing();
        usleep(5000);
    }
    int kind = -1;
    bg_poll_kind(&kind);
    if (kind == BG_TEMPLATE) bgimage_retry_missing();
    return kind;
}

static void put_template(int n, const char *name) {
    BgPicture p = {malloc(60 * 80), 60, 80, 1};
    memset(p.px, 255, 60 * 80);
    for (int y = 5; y < 80; y += 3 + n % 11) memset(p.px + y * 60, (uint8_t)(100 + n % 50), 60);
    p.px[n % 60] = 0;
    char path[700];
    snprintf(path, sizeof path, "%s/templates/%s", root, name);
    if (bgimage_write_png(path, &p) != 0) abort();
    bgpicture_free(&p);
}

// ---- Custom music ----

enum { MB_UP = 1, MB_DOWN, MB_REMOVE, MB_ADD, MB_LESS, MB_MORE, MB_CANCEL_ID, MB_DONE_ID };

static MbResult mb_tap(MusicBuild *b, int id, int arg, Rect *d) {
    for (int i = 0; i < b->hits.n; i++) {
        const Hit *h = &b->hits.h[i];
        if (h->id != id || h->arg != arg) continue;
        float x = (float)(h->r.x + h->r.w / 2), y = (float)(h->r.y + h->r.h / 2);
        musicbuild_pointer(b, &app, 0, x, y, 1000, d);
        return musicbuild_pointer(b, &app, 2, x, y, 1080, d);
    }
    return (MbResult)-1;
}

static void music(void) {
    MusicLayout L[4];
    memset(L, 0, sizeof L);
    L[0] = (MusicLayout){5, 2, {PART_TREBLE, PART_TAB6}};
    L[1] = (MusicLayout){3, 6, {PART_TREBLE, PART_BASS, PART_ALTO, PART_TENOR, PART_PERC, PART_LYRICS}};
    L[2] = (MusicLayout){1, 1, {PART_STAFF}};
    L[3] = (MusicLayout){12, 3, {PART_TAB4, PART_TAB5, PART_TAB7}};
    for (int turn = 0; turn < 2; turn++) {
        screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
        int pw = turn ? 1986 : 1860, ph = turn ? 2648 : 2480;
        for (int th = 0; th < 2; th++) {
            look(th);
            for (int li = 0; li < 4; li++) {
                MusicBuild b;
                memset(&b, 0, sizeof b);
                musicbuild_open_page(&b, &app, music_encode(&L[li]), pw, ph);
                canvas_fill(&app.screen, (Rect){0, 0, app.w, app.h}, 255);
                Rect r = musicbuild_draw(&b, &app);
                printf("music %s L%d rect %d %d %d %d systems %d ", tag(turn, th), li, r.x, r.y, r.w, r.h, b.m.systems);
                emit("hash", &app);
                musicbuild_free(&b);
            }
        }
    }
    // A sheet's taps: each prints what the pointer said, the layout it leaves and the screen.
    screen(1860, 2480);
    look(0);
    MusicBuild b;
    memset(&b, 0, sizeof b);
    musicbuild_open(&b, &app, PAPER_BLANK);
    musicbuild_draw(&b, &app);
    static const int steps[][2] = {{MB_ADD, PART_BASS}, {MB_ADD, PART_LYRICS}, {MB_DOWN, 0}, {MB_UP, 3}, {MB_REMOVE, 1},
                                   {MB_MORE, 0}, {MB_MORE, 0}, {MB_LESS, 0}, {MB_ADD, PART_TAB6}, {MB_ADD, PART_ALTO},
                                   {MB_ADD, PART_TENOR}, {MB_ADD, PART_PERC}, {MB_REMOVE, 0}, {MB_UP, 0}, {MB_LESS, 0},
                                   {MB_LESS, 0}, {MB_LESS, 0}, {MB_LESS, 0}, {MB_LESS, 0}, {MB_MORE, 0}};
    for (size_t i = 0; i < sizeof steps / sizeof steps[0]; i++) {
        Rect d = RECT_EMPTY;
        MbResult r = mb_tap(&b, steps[i][0], steps[i][1], &d);
        printf("music tap %d %d -> %d dirty %d %d %d %d systems %d n %d kind %u ", steps[i][0], steps[i][1], (int)r, d.x, d.y, d.w,
               d.h, b.m.systems, b.m.n, (unsigned)musicbuild_result(&b));
        emit("hash", &app);
    }
    Rect d;
    MbResult r = mb_tap(&b, MB_DONE_ID, 0, &d);
    printf("music done -> %d open %d kind %u saved %u\n", (int)r, b.open, (unsigned)musicbuild_result(&b),
           (unsigned)app.settings.music_layout);
    // Opened again: from the layout saved. And a tap outside, and Cancel.
    musicbuild_open(&b, &app, PAPER_BLANK);
    printf("music again systems %d n %d kind %u\n", b.m.systems, b.m.n, (unsigned)musicbuild_result(&b));
    musicbuild_draw(&b, &app);
    musicbuild_pointer(&b, &app, 0, 5, 5, 1000, &d);
    r = musicbuild_pointer(&b, &app, 2, 5, 5, 1080, &d);
    printf("music outside -> %d open %d dirty %d %d %d %d\n", (int)r, b.open, d.x, d.y, d.w, d.h);
    musicbuild_open(&b, &app, PAPER_BLANK);
    musicbuild_draw(&b, &app);
    r = mb_tap(&b, MB_CANCEL_ID, 0, &d);
    printf("music cancel -> %d open %d\n", (int)r, b.open);
    musicbuild_free(&b);
}

// ---- Templates ----

static void labels(void) {
    Canvas c;
    if (canvas_init(&c, 700, 600) != 0) abort();
    PaperGrid g[6];
    App a;
    memset(&a, 0, sizeof a);
    a.dpi = 300.f;
    a.w = 1860, a.h = 2480;
    paper_sheet_rect(&a, &g[0]);
    a.w = 2480, a.h = 1860;
    paper_sheet_rect(&a, &g[1]);
    static const int box[4][2] = {{1640, 1590}, {1648, 1608}, {1720, 1070}, {1728, 1088}};
    for (int i = 0; i < 4; i++) g[2 + i] = paper_grid(PAPER_SHEET_TILES, box[i][0], box[i][1], 30, 100, 300, PAPER_GRID_MIN_COLS);
    for (int k = 0; k < 6; k++) printf("grid %d cols %d rows %d tw %d th %d cell %d\n", k, g[k].cols, g[k].rows, g[k].tw, g[k].th, g[k].cell_h);
    int gap = ui_px(30);
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        for (int th = 0; th < 2; th++) {
            look(th);
            u64 h = H0;
            for (int k = 0; k < 6; k++) {
                Rect tr = {200, 20, g[k].tw, g[k].th};
                for (int i = 0; i < PAPER_SHEET_TILES; i++) {
                    const char *label = i < PAPER_COUNT ? paper_label(paper_at(i)) : i == PAPER_COUNT ? _("Custom music...") : _("More papers...");
                    for (int sel = 0; sel < 2; sel++) {
                        canvas_fill(&c, (Rect){0, 0, c.w, c.h}, 255);
                        paper_tile_label(&c, tr, gap, label, sel);
                        h = fnv_u32(h, (uint32_t)canvas_hash(&c));
                        h = fnv_u32(h, (uint32_t)(canvas_hash(&c) >> 32));
                    }
                }
                canvas_fill(&c, (Rect){0, 0, c.w, c.h}, 255);
                paper_tile_more(&c, tr);
                h = fnv_u32(h, (uint32_t)canvas_hash(&c));
            }
            printf("labels %s %s %016llx\n", i18n_language_code(l), th ? "classic" : "gothic", (unsigned long long)h);
        }
    }
    i18n_set("en");
    canvas_free(&c);
}

static const Hit *ts_hit(const TplSheet *s, int id, int arg) {
    for (int i = 0; i < s->hits.n; i++)
        if (s->hits.h[i].id == id && (id != TS_HIT_TILE || s->hits.h[i].arg == arg)) return &s->hits.h[i];
    return NULL;
}

static void templates(void) {
    char cmd[800];
    snprintf(cmd, sizeof cmd, "rm -rf '%s/templates'", root);
    if (system(cmd) != 0) abort();
    templates_init();
    // An empty folder, then with files that are no pictures, in every language (the empty state's
    // paragraphs wrap differently in each), then the Photos sheet's.
    char path[700];
    for (int phase = 0; phase < 2; phase++) {
        if (phase == 1) {
            snprintf(path, sizeof path, "%s/templates/Scan.png", root);
            if (write_file_atomic(path, "no picture", 10) != 0) abort();
            snprintf(path, sizeof path, "%s/templates/Photo.jpg", root);
            if (write_file_atomic(path, "no picture either", 17) != 0) abort();
            // (opened once, the previews job finds them bad: the sheets below say how many files it cannot use)
            TplSheet first;
            memset(&first, 0, sizeof first);
            tplsheet_open(&first, &app, NULL, PAPER_BLANK, NULL);
            drain_jobs();
            tplsheet_close(&first);
        }
        for (int l = 0; l < i18n_language_count(); l++) {
            i18n_set(i18n_language_code(l));
            for (int turn = 0; turn < 2; turn++) {
                screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
                for (int th = 0; th < 2; th++) {
                    look(th);
                    for (int photos = 0; photos < 2; photos++) {
                        TplSheet s;
                        memset(&s, 0, sizeof s);
                        if (photos) tplsheet_open_photos(&s, &app, &(NotebookInfo){0}, PAPER_BLANK);
                        else tplsheet_open(&s, &app, NULL, PAPER_BLANK, NULL);
                        canvas_fill(&app.screen, (Rect){0, 0, app.w, app.h}, 255);
                        tplsheet_draw(&s, &app);
                        char label[64];
                        snprintf(label, sizeof label, "tplsheet empty%d %s %s %s", phase, i18n_language_code(l), tag(turn, th), photos ? "photos" : "templates");
                        emit(label, &app);
                        tplsheet_close(&s);
                    }
                }
            }
        }
        i18n_set("en");
        drain_jobs();
    }
    // Pictures: waiting tiles, then their previews, one of them the paper now, a broken one crossed out.
    snprintf(cmd, sizeof cmd, "rm -rf '%s/templates'", root);
    if (system(cmd) != 0) abort();
    templates_init();
    char name[64];
    for (int n = 1; n <= 30; n++) {
        snprintf(name, sizeof name, "page-%d.png", n);
        put_template(n, name);
    }
    snprintf(path, sizeof path, "%s/templates/zz-broken.png", root);
    if (write_file_atomic(path, "no picture", 10) != 0) abort();
    for (int turn = 0; turn < 2; turn++) {
        screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
        for (int th = 0; th < 2; th++) {
            look(th);
            TplSheet s;
            memset(&s, 0, sizeof s);
            char current[700];
            snprintf(current, sizeof current, "%s/templates/page-17.png", root);
            tplsheet_open(&s, &app, NULL, PAPER_BLANK, current);
            canvas_fill(&app.screen, (Rect){0, 0, app.w, app.h}, 255);
            tplsheet_draw(&s, &app);
            char label[64];
            printf("tplsheet %s waiting page %d of %d selected %d ", tag(turn, th), s.page + 1, s.pages, s.selected);
            emit("hash", &app);
            drain_jobs();
            Rect d;
            int ticked = tplsheet_tick(&s, &app, now_ms(), &d);
            printf("tplsheet %s ready tick %d dirty %d %d %d %d page %d of %d ", tag(turn, th), ticked, d.x, d.y, d.w, d.h, s.page + 1, s.pages);
            emit("hash", &app);
            // Every page of tiles, the broken one's last.
            for (int p = 1; p < s.pages + 1; p++) {
                const Hit *h = ts_hit(&s, TS_HIT_NEXT, 0);
                if (!h) break;
                float x = (float)(h->r.x + h->r.w / 2), y = (float)(h->r.y + h->r.h / 2);
                tplsheet_pointer(&s, &app, 0, x, y, 1000, &d);
                TsResult r = tplsheet_pointer(&s, &app, 2, x, y, 1080, &d);
                snprintf(label, sizeof label, "tplsheet %s turn %d -> %d page %d", tag(turn, th), p, (int)r, s.page + 1);
                emit(label, &app);
            }
            tplsheet_close(&s);
        }
    }
    drain_jobs();
}

// ---- Pages ----

static NotebookInfo nb;
static Nav nav, many;

static void setup_notebook(void) {
    if (store_create("Probe", "", PAPER_MEDIUM, &nb) != 0) abort();
    for (int i = 1; i < 30; i++)
        if (store_add_page(&nb, i, PAPER_MEDIUM) != 0) abort();
    nav_init(&nav);
    nav_init(&many);
    static const struct {
        int page;
        const char *name;
    } names[] = {{1, "Intro"}, {6, "Chapter two, a rather long name that must be cut short somewhere in the tile"},
                 {12, "\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e"}, {20, "\xc3\x89t\xc3\xa9"}, {25, "Notes"}};
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++) nav_set_name(&nav, nb.page_ids[names[i].page], names[i].name);
    for (int i = 0; i < 20; i++) {
        char nm[64];
        snprintf(nm, sizeof nm, "Section %d", i + 1);
        nav_set_name(&many, nb.page_ids[i + (i > 9 ? 3 : 0)], nm);
    }
    char dir[700], file[800], text[800];
    if (store_notebook_dir(nb.id, dir, sizeof dir) != 0) abort();
    snprintf(file, sizeof file, "%s/source.ini", dir);
    snprintf(text, sizeof text, "# probe\npage.12.300=%u\npage.13.300=%u\npage.5.300=%u\npage.44.340=%u\npage.7.300=%u\n",
             nb.page_ids[2], nb.page_ids[6], nb.page_ids[9], nb.page_ids[17], nb.page_ids[26]);
    if (write_file_atomic(file, text, strlen(text)) != 0) abort();
}

static const Hit *pv_hit(const PagesView *v, int id, int arg) {
    for (int i = 0; i < v->hits.n; i++)
        if (v->hits.h[i].id == id && v->hits.h[i].arg == arg) return &v->hits.h[i];
    return NULL;
}
// Hit ids, as pagesview.c numbers them.
enum { PG_BACK = 1, PG_DELETE, PG_ALL, PG_TOGGLE, PG_OPEN, PG_PREV, PG_NEXT, PG_JUMP, PG_NEW, PG_COPY, PG_TAB, PG_NAME, PG_LINK,
       PG_TO_INDEX, PG_UNLINK };

static PvResult pv_tap(PagesView *v, int id, int arg) {
    const Hit *h = pv_hit(v, id, arg);
    if (!h) return (PvResult)-1;
    InEvent ev;
    memset(&ev, 0, sizeof ev);
    ev.type = IN_TOUCH_DOWN;
    ev.x = (float)(h->r.x + h->r.w / 2);
    ev.y = (float)(h->r.y + h->r.h / 2);
    ev.t_ms = 1000;
    pagesview_pointer(v, &app, 0, &ev);
    ev.type = IN_TOUCH_UP;
    ev.t_ms = 1080;
    return pagesview_pointer(v, &app, 2, &ev);
}

static void pv_reset(PagesView *v) {
    pagesview_free(v);
    memset(v, 0, sizeof *v);
}

static void pv_probe_all(const PagesView *v, const char *tagname) {
    static const char *const keys[] = {"pages_tab", "picking", "keyboard", "text", "index_rows", "tile.1", "tile.3", "tile.7", "tile.10",
                                       "tile.13", "tile.18", "tile.27", "tile.30", "tile.31", "tile.0", "tile.x", "bogus"};
    for (size_t i = 0; i < sizeof keys / sizeof keys[0]; i++) {
        char out[200];
        memset(out, 'x', sizeof out);
        out[sizeof out - 1] = 0;
        int rc = pagesview_probe(v, keys[i], out, sizeof out);
        printf("pages %s probe %s rc %d '%s'\n", tagname, keys[i], rc, rc == 0 ? out : "");
    }
}

static void pages_views(void) {
    for (int turn = 0; turn < 2; turn++) {
        screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
        for (int th = 0; th < 2; th++) {
            look(th);
            const char *t = tag(turn, th);
            char label[96];
            PagesView v;
            memset(&v, 0, sizeof v);
            // The Pages tab: the first screenful, then the one with the open page and some pages ticked.
            if (!pagesview_open(&v, &app, &nb, &nav, 0)) abort();
            v.index_tab = false;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s tab0 screen %d", t, v.screen);
            emit(label, &app);
            pagesview_close(&v);
            if (!pagesview_open(&v, &app, &nb, &nav, 17)) abort();
            v.index_tab = false;
            v.sel[3] = v.sel[4] = v.sel[18] = true;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s tab17 ticked %d screen %d", t, pagesview_ticked_count(&v), v.screen);
            emit(label, &app);
            for (int i = 0; i < 30; i++) v.sel[i] = true;
            v.screen = 99;  // (clamped to the last)
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s all ticked %d screen %d", t, pagesview_ticked_count(&v), v.screen);
            emit(label, &app);
            pagesview_untick(&v);
            v.sel[7] = true;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s one ticked %d", t, pagesview_ticked_count(&v));
            emit(label, &app);
            pv_probe_all(&v, t);
            // The Index tab, with five names, and with twenty (two screenfuls).
            v.index_tab = true;
            v.index_screen = 0;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s index", t);
            emit(label, &app);
            pagesview_close(&v);
            pv_reset(&v);
            if (!pagesview_open(&v, &app, &nb, &many, 4)) abort();
            for (int s = 0; s < 3; s++) {
                v.index_screen = s;
                pagesview_draw(&v, &app);
                snprintf(label, sizeof label, "pages %s many index %d (screen %d)", t, s, v.index_screen);
                emit(label, &app);
            }
            pv_probe_all(&v, t);
            pagesview_close(&v);
            // Picking where a link goes: one just drawn, one the page had.
            for (int had = 0; had < 2; had++) {
                pv_reset(&v);
                if (!pagesview_pick(&v, &app, &nb, &nav, 12, had)) abort();
                pagesview_draw(&v, &app);
                snprintf(label, sizeof label, "pages %s pick had %d tab %d", t, had, v.index_tab);
                emit(label, &app);
                pv_probe_all(&v, t);
                pagesview_close(&v);
            }
            // Picking in a notebook with no names: the Pages tab.
            Nav none;
            nav_init(&none);
            pv_reset(&v);
            if (!pagesview_pick(&v, &app, &nb, &none, 12, true)) abort();
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s pick none tab %d", t, v.index_tab);
            emit(label, &app);
            pagesview_close(&v);
            // The naming keyboard, over the Index; and Go to page's number pad.
            pv_reset(&v);
            if (!pagesview_name(&v, &app, &nb, &nav, 6)) abort();
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s name only %d", t, v.name_only);
            emit(label, &app);
            pv_probe_all(&v, t);
            pagesview_close(&v);
            printf("pages %s after name: tab %d init %d\n", t, v.index_tab, v.tab_initialized);
            if (!pagesview_open(&v, &app, &nb, &nav, 9)) abort();
            v.index_tab = false;
            v.naming = false;
            keyboard_open(&v.kb, "Go to page", "");
            v.kb.page = 2;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s jump", t);
            emit(label, &app);
            pv_probe_all(&v, t);
            pagesview_close(&v);
            // Names that cannot be saved: Name this page greyed out, Add a link greyed out.
            Nav ro = nav;
            ro.writable = false;
            ro.links_writable = false;
            if (!pagesview_open(&v, &app, &nb, &ro, 6)) abort();
            v.index_tab = true;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s readonly index", t);
            emit(label, &app);
            pagesview_close(&v);
            Nav empty;
            nav_init(&empty);
            empty.writable = false;
            if (!pagesview_open(&v, &app, &nb, &empty, 6)) abort();
            v.index_tab = true;
            pagesview_draw(&v, &app);
            snprintf(label, sizeof label, "pages %s unreadable index", t);
            emit(label, &app);
            pagesview_close(&v);
            pagesview_free(&v);
        }
    }
    // The translations: the Pages and Index tabs and the number pad in every language.
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        for (int turn = 0; turn < 2; turn++) {
            screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
            for (int th = 0; th < 2; th++) {
                look(th);
                PagesView v;
                memset(&v, 0, sizeof v);
                if (!pagesview_open(&v, &app, &nb, &nav, 6)) abort();
                u64 h = H0;
                v.index_tab = false;
                v.sel[2] = true;
                pagesview_draw(&v, &app);
                h = fnv_u32(fnv_u32(h, (uint32_t)canvas_hash(&app.screen)), (uint32_t)(canvas_hash(&app.screen) >> 32));
                v.index_tab = true;
                pagesview_draw(&v, &app);
                h = fnv_u32(fnv_u32(h, (uint32_t)canvas_hash(&app.screen)), (uint32_t)(canvas_hash(&app.screen) >> 32));
                pagesview_close(&v);
                if (!pagesview_pick(&v, &app, &nb, &nav, 6, true)) abort();
                pagesview_draw(&v, &app);
                h = fnv_u32(fnv_u32(h, (uint32_t)canvas_hash(&app.screen)), (uint32_t)(canvas_hash(&app.screen) >> 32));
                pagesview_close(&v);
                if (!pagesview_open(&v, &app, &nb, &nav, 6)) abort();
                v.index_tab = false;
                keyboard_open(&v.kb, "Go to page", "");
                v.kb.page = 2;
                pagesview_draw(&v, &app);
                h = fnv_u32(fnv_u32(h, (uint32_t)canvas_hash(&app.screen)), (uint32_t)(canvas_hash(&app.screen) >> 32));
                pagesview_free(&v);
                printf("pages lang %s %s %016llx\n", i18n_language_code(l), tag(turn, th), (unsigned long long)h);
            }
        }
    }
    i18n_set("en");
}

// A scripted sequence of taps, as the editor would see them: what each says, and the view's state.
static void pages_taps(void) {
    screen(1860, 2480);
    look(0);
    PagesView v;
    memset(&v, 0, sizeof v);
    if (!pagesview_open(&v, &app, &nb, &nav, 17)) abort();
    v.index_tab = false;
    pagesview_draw(&v, &app);
    static const int steps[][2] = {{PG_TOGGLE, 3}, {PG_TOGGLE, 5}, {PG_TOGGLE, 3}, {PG_ALL, 0}, {PG_ALL, 0}, {PG_NEXT, 0},
                                   {PG_TOGGLE, 29}, {PG_COPY, 0}, {PG_NEW, 0}, {PG_DELETE, 0}, {PG_PREV, 0}, {PG_PREV, 0},
                                   {PG_TOGGLE, 12}, {PG_COPY, 0}, {PG_NEW, 0}, {PG_TAB, 1}, {PG_NAME, 0}, {PG_TAB, 0},
                                   {PG_OPEN, 11}, {PG_BACK, 0}, {PG_TAB, 1}, {PG_LINK, 0}, {PG_NEXT, 0}, {PG_PREV, 0}, {PG_JUMP, 0}};
    for (size_t i = 0; i < sizeof steps / sizeof steps[0]; i++) {
        PvResult r = pv_tap(&v, steps[i][0], steps[i][1]);
        char label[96];
        snprintf(label, sizeof label, "pages tap %d %d -> %d target %d tab %d screen %d idx %d ticked %d kb %d", steps[i][0], steps[i][1],
                 (int)r, v.target, v.index_tab, v.screen, v.index_screen, pagesview_ticked_count(&v), v.kb.open);
        emit(label, &app);
        if (v.kb.open) break;
    }
    // The number pad: a page in the notebook, one that is not, and Cancel.
    static const char *const typed[] = {"12", "31", "0", "7x"};
    for (size_t t = 0; t < 4; t++) {
        pagesview_close(&v);
        if (!pagesview_open(&v, &app, &nb, &nav, 17)) abort();
        v.index_tab = false;
        pagesview_draw(&v, &app);
        pv_tap(&v, PG_JUMP, 0);
        keyboard_draw(&v.kb, &app.screen, (Rect){0, 0, app.w, app.h});
        InEvent ev;
        memset(&ev, 0, sizeof ev);
        for (const char *p = typed[t]; *p; p++) {
            Rect kr;
            if (!keyboard_key_rect(&v.kb, (uint32_t)*p, &kr)) {
                printf("pages key %c: none\n", *p);
                continue;
            }
            ev.type = IN_TOUCH_DOWN;
            ev.x = (float)(kr.x + kr.w / 2);
            ev.y = (float)(kr.y + kr.h / 2);
            ev.t_ms = 2000;
            pagesview_pointer(&v, &app, 0, &ev);
            ev.type = IN_TOUCH_UP;
            ev.t_ms = 2080;
            pagesview_pointer(&v, &app, 2, &ev);
        }
        Rect kr;
        keyboard_key_rect(&v.kb, KB_KEY_DONE, &kr);
        ev.type = IN_TOUCH_DOWN;
        ev.x = (float)(kr.x + kr.w / 2);
        ev.y = (float)(kr.y + kr.h / 2);
        ev.t_ms = 3000;
        pagesview_pointer(&v, &app, 0, &ev);
        ev.type = IN_TOUCH_UP;
        ev.t_ms = 3080;
        PvResult r = pagesview_pointer(&v, &app, 2, &ev);
        char label[96];
        snprintf(label, sizeof label, "pages typed '%s' -> %d target %d kb %d text '%s'", typed[t], (int)r, v.target, v.kb.open, v.kb.text);
        emit(label, &app);
    }
    pagesview_free(&v);
}

// ---- Pen feel ----

static const char *const pen_keys[] = {"slider_pending", "param.size_min_mm", "param.size_max_mm", "param.opacity_max", "param.pressure_floor",
                                       "param.pressure_ceiling", "param.tilt_min_deg", "param.tilt_max_deg", "param.grain",
                                       "param.nonesuch", "pad_strokes", "pad_sample_rect", "pad_white", "pad_ink", "pad_lighter",
                                       "pad_ink_near.1000.1000", "pad_ink_near.-5.x", "pad_ink_near.700.900", "bogus"};

static void pen_probe_all(const char *tagname) {
    for (size_t i = 0; i < sizeof pen_keys / sizeof pen_keys[0]; i++) {
        char out[200];
        memset(out, 'x', sizeof out);
        out[sizeof out - 1] = 0;
        int rc = pensettings_probe(&app, pen_keys[i], out, sizeof out);
        printf("penset %s probe %s rc %d '%s'\n", tagname, pen_keys[i], rc, rc == 0 ? out : "");
    }
}

static u64 params_hash(const ToolParams *p) {
    u64 h = H0;
    const float *f = (const float *)p;
    for (size_t i = 0; i < sizeof *p / sizeof *f; i++) {
        uint32_t bits;
        memcpy(&bits, &f[i], 4);
        h = fnv_u32(h, bits);
    }
    return h;
}

static void pen(InType type, float x, float y, float pressure, float tx, float ty, uint32_t ms, bool eraser) {
    InEvent ev;
    memset(&ev, 0, sizeof ev);
    ev.type = type;
    ev.x = x;
    ev.y = y;
    ev.pressure = pressure;
    ev.tilt_x = tx;
    ev.tilt_y = ty;
    ev.t_ms = ms;
    ev.eraser = eraser;
    pensettings_event(&app, &ev);
}

// A stroke across the test pad, in 40 samples: a zigzag, the pressure ramping up and down, the pen
// leaning one way and the other. Made of integers, so the Rust twin makes the same.
static void pad_stroke(bool eraser, uint32_t t0, int phase) {
    Rect p = S->pad;
    float x0 = (float)(p.x + p.w / 10), y0 = (float)(p.y + p.h / 2 + phase * 40);
    pen(IN_PEN_DOWN, x0, y0, 0.15f, 20.f, -15.f, t0, eraser);
    for (int i = 1; i <= 40; i++) {
        float x = x0 + (float)(i * (p.w * 8 / 10) / 40);
        float y = y0 + (float)((i * 37) % 160 - 80);
        float pr = (float)(i < 20 ? 10 + i * 4 : 90 - (i - 20) * 3) / 100.f;
        float tx = (float)((i * 5) % 50) - 10.f, ty = (float)((i * 3) % 60) - 30.f;
        pen(IN_PEN_MOVE, x, y, pr, tx, ty, t0 + (uint32_t)i * 12, eraser);
    }
    pen(IN_PEN_UP, x0 + (float)(p.w * 8 / 10), y0, 0.f, 0.f, 0.f, t0 + 600, eraser);
}

static void pen_screen(int turn, int th, ToolKind tab, const char *mode) {
    S->tab = tab;
    S->ret = SCR_EDITOR;
    pensettings_enter(&app);
    (void)turn;
    (void)th;
    (void)mode;
}

static void penset_draws(void) {
    pensettings_init(&app);
    settings_default(&app.settings);
    app.scr = SCR_PENSET;
    // Every tool's tab, upright and sideways, in both looks; the eraser's two modes; white ink.
    for (int turn = 0; turn < 2; turn++) {
        screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
        for (int th = 0; th < 2; th++) {
            look(th);
            for (int ti = 0; ti < TOOL_COUNT; ti++) {
                ToolKind tab = tool_order[ti];
                for (int mode = 0; mode < 3; mode++) {
                    app.settings.eraser_mode = mode == 1 ? 1 : 0;
                    app.settings.white = mode == 2;
                    pen_screen(turn, th, tab, "");
                    char label[96];
                    snprintf(label, sizeof label, "penset %s tab %d mode %d", tag(turn, th), (int)tab, mode);
                    emit(label, &app);
                }
            }
        }
    }
    app.settings.eraser_mode = 0;
    app.settings.white = false;
    // The languages: the calligraphy tab (its own names for two sliders) and the eraser tab.
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        screen(1860, 2480);
        u64 h = H0;
        for (int th = 0; th < 2; th++) {
            look(th);
            static const ToolKind tabs[] = {TOOL_CALLIGRAPHY, TOOL_ERASER, TOOL_PENCIL, TOOL_CHARCOAL};
            for (int k = 0; k < 4; k++) {
                for (int mode = 0; mode < 2; mode++) {
                    app.settings.eraser_mode = mode;
                    pen_screen(0, th, tabs[k], "");
                    h = fnv_u32(fnv_u32(h, (uint32_t)canvas_hash(&app.screen)), (uint32_t)(canvas_hash(&app.screen) >> 32));
                }
            }
        }
        printf("penset lang %s %016llx\n", i18n_language_code(l), (unsigned long long)h);
    }
    i18n_set("en");
    pensettings_free(&app);
}

static void penset_events(void) {
    pensettings_init(&app);
    settings_default(&app.settings);
    app.scr = SCR_PENSET;
    // The pen on the pad, on each tool's tab: the stroke, the readout, the graph's live dot.
    screen(1860, 2480);
    look(0);
    for (int ti = 0; ti < TOOL_COUNT; ti++) {
        ToolKind tab = tool_order[ti];
        pen_screen(0, 0, tab, "");
        pad_stroke(false, 100000, 0);
        char label[96];
        snprintf(label, sizeof label, "penset pad tool %d", (int)tab);
        emit(label, &app);
        pad_stroke(false, 200000, 1);
        snprintf(label, sizeof label, "penset pad tool %d again", (int)tab);
        emit(label, &app);
        pen_probe_all(label + 7);
    }
    // The same in the other look and sideways, for two tools.
    for (int turn = 0; turn < 2; turn++) {
        screen(turn ? 2480 : 1860, turn ? 1860 : 2480);
        for (int th = 0; th < 2; th++) {
            look(th);
            static const ToolKind two[] = {TOOL_CALLIGRAPHY, TOOL_HIGHLIGHTER};
            for (int k = 0; k < 2; k++) {
                pen_screen(turn, th, two[k], "");
                pad_stroke(false, 300000, 0);
                char label[96];
                snprintf(label, sizeof label, "penset pad %s tool %d", tag(turn, th), (int)two[k]);
                emit(label, &app);
            }
        }
    }
    screen(1860, 2480);
    look(0);
    // White ink on a drawing tab (the swatch), its eraser end, the precise eraser, the whole-stroke eraser.
    app.settings.white = true;
    pen_screen(0, 0, TOOL_PENCIL, "");
    emit("penset white tab", &app);
    pad_stroke(false, 400000, 0);
    emit("penset white stroke", &app);
    pen_probe_all("white");
    app.settings.white = false;
    app.settings.eraser_mode = 1;
    pen_screen(0, 0, TOOL_BALLPOINT, "");
    pad_stroke(false, 500000, 0);
    emit("penset ballpoint ink", &app);
    pad_stroke(true, 600000, 0);  // the pen's eraser end: the precise eraser
    emit("penset ballpoint erased (end)", &app);
    pen_probe_all("end");
    app.settings.eraser_mode = 0;
    pen_screen(0, 0, TOOL_FOUNTAIN, "");
    pad_stroke(false, 700000, 0);
    pad_stroke(true, 800000, 0);  // the pen's eraser end, whole strokes: the pad is drawn bare
    emit("penset fountain wiped (end)", &app);
    pen_probe_all("wiped");
    // The eraser's tab: the practice ink, whole strokes taken away, then the precise eraser.
    for (int mode = 0; mode < 2; mode++) {
        app.settings.eraser_mode = mode;
        pen_screen(0, 0, TOOL_ERASER, "");
        char label[96];
        snprintf(label, sizeof label, "penset eraser mode %d", mode);
        emit(label, &app);
        pen_probe_all(label + 7);
        for (int k = 0; k < 3; k++) {
            pad_stroke(false, 900000 + (uint32_t)k * 10000, k - 1);
            snprintf(label, sizeof label, "penset eraser mode %d stroke %d", mode, k);
            emit(label, &app);
            pen_probe_all(label + 7);
        }
    }
    // Sliders: a drag on each row, with a hold and a wide y for the fine mode, and the pairs' limits.
    app.settings.eraser_mode = 0;
    for (int tabi = 0; tabi < 2; tabi++) {
        ToolKind tab = tabi ? TOOL_CALLIGRAPHY : TOOL_PENCIL;
        pen_screen(0, 0, tab, "");
        int n = S->nsliders;
        for (int row = 0; row < n; row++) {
            Rect r = S->slider_rows[row];
            float xs[] = {(float)(r.x + r.w * 45 / 100), (float)(r.x + r.w * 60 / 100), (float)(r.x + r.w * 99 / 100),
                          (float)(r.x + r.w * 50 / 100), (float)(r.x + r.w * 41 / 100)};
            float y = (float)(r.y + r.h / 2);
            uint32_t t = 1000000 + (uint32_t)row * 5000;
            pen(IN_PEN_DOWN, xs[0], y, 0.5f, 0.f, 0.f, t, false);
            pen(IN_PEN_MOVE, xs[1], y, 0.5f, 0.f, 0.f, t + 30, false);
            pen(IN_PEN_MOVE, xs[2], y, 0.5f, 0.f, 0.f, t + 60, false);
            pen(IN_PEN_MOVE, xs[3], y + 130.f, 0.5f, 0.f, 0.f, t + 500, false);
            pen(IN_PEN_MOVE, xs[4], y + 130.f, 0.5f, 0.f, 0.f, t + 700, false);
            pen(IN_PEN_UP, xs[4], y + 130.f, 0.f, 0.f, 0.f, t + 800, false);
            char label[96];
            snprintf(label, sizeof label, "penset slider tab %d row %d params %016llx", (int)tab, row,
                     (unsigned long long)params_hash(&app.settings.tools[tab]));
            emit(label, &app);
            pen_probe_all("slider");
        }
        // A drag held still (the tick repaints it), then released.
        {
            Rect r = S->slider_rows[1];
            float y = (float)(r.y + r.h / 2);
            pen(IN_PEN_DOWN, (float)(r.x + r.w * 70 / 100), y, 0.5f, 0.f, 0.f, 2000000, false);
            pen(IN_PEN_MOVE, (float)(r.x + r.w * 72 / 100), y, 0.5f, 0.f, 0.f, 2000010, false);
            pensettings_tick(&app, 2000020);
            printf("penset tick pending %d\n", S->slider_dirty);
            pensettings_tick(&app, 2000100);
            printf("penset tick pending %d\n", S->slider_dirty);
            pen(IN_PEN_UP, (float)(r.x + r.w * 72 / 100), y, 0.f, 0.f, 0.f, 2000200, false);
            emit("penset slider held", &app);
        }
    }
    // Pairs that keep each other in order (the widths, the pressure range, the tilt range): a slider
    // dragged past its partner takes the partner along.
    app.settings.eraser_mode = 0;
    pen_screen(0, 0, TOOL_PENCIL, "");
    {
        static const struct {
            int row, to;  // the row, and where the drag ends: its share of the row's width, in percent (0 and 100: the ends)
        } pairs[] = {{4, 50}, {5, 0}, {4, 100}, {8, 0}, {7, 100}, {1, 0}, {0, 100}, {5, 100}, {4, 0}, {7, 0}, {8, 100}};
        for (size_t i = 0; i < sizeof pairs / sizeof pairs[0]; i++) {
            Rect r = S->slider_rows[pairs[i].row];
            float y = (float)(r.y + r.h / 2);
            float x_to = pairs[i].to == 0 ? (float)(r.x + r.w * 42 / 100) : pairs[i].to == 100 ? (float)(r.x + r.w - 2) : (float)(r.x + r.w * pairs[i].to / 100);
            uint32_t t = 3000000 + (uint32_t)i * 1000;
            pen(IN_PEN_DOWN, (float)(r.x + r.w * 70 / 100), y, 0.5f, 0.f, 0.f, t, false);
            pen(IN_PEN_MOVE, x_to, y, 0.5f, 0.f, 0.f, t + 100, false);
            pen(IN_PEN_UP, x_to, y, 0.f, 0.f, 0.f, t + 200, false);
            char label[96];
            snprintf(label, sizeof label, "penset pairs %d row %d to %d params %016llx", (int)i, pairs[i].row, pairs[i].to,
                     (unsigned long long)params_hash(&app.settings.tools[TOOL_PENCIL]));
            emit(label, &app);
        }
    }
    pensettings_leave(&app);
    pensettings_free(&app);
}

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    snprintf(root, sizeof root, "%s", argv[1]);
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    if (system(cmd) != 0) return 2;
    if (store_init(root) != 0) return 2;
    store_set_device(300.f, 1860, 2480);
    ui_init(300.f);
    if (font_init() != 0) return 2;
    app.dpi = 300.f;
    settings_default(&app.settings);
    puts("== labels");
    labels();
    puts("== music");
    music();
    puts("== templates");
    templates();
    setup_notebook();
    puts("== pages views");
    pages_views();
    puts("== pages taps");
    pages_taps();
    puts("== penset draws");
    penset_draws();
    puts("== penset events");
    penset_events();
    font_shutdown();
    return 0;
}
