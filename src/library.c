// library.c: the home screen. Folders and notebooks as cards, paged (no scrolling on e-ink),
// plus the sheets: new notebook (paper picker), settings, notebook actions, move, delete.
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "app.h"
#include "export.h"
#include "dispmenu.h"
#include "bgjob.h"
#include "musicbuild.h"
#include "keyboard.h"

typedef enum { MOD_NONE, MOD_NEW, MOD_SETTINGS, MOD_MORE, MOD_FOLDER_MORE, MOD_MOVE, MOD_DELETE, MOD_KB, MOD_DISPLAY, MOD_MUSIC } Modal;
typedef enum { KBP_NEW_NAME, KBP_RENAME, KBP_NEW_FOLDER, KBP_MOVE_NEW_FOLDER, KBP_RENAME_FOLDER } KbPurpose;

enum {
    H_NONE, H_BACK, H_NEW, H_NEWFOLDER, H_SHARE, H_SETTINGS, H_EXIT, H_PREV, H_NEXT,
    H_CARD, H_CARD_MORE,
    H_PAPER, H_PAPER_CUSTOM, H_NEW_RENAME, H_NEW_CREATE, H_CANCEL,
    H_SET_PENS, H_SET_CALIB, H_SET_MIRROR, H_SET_SIDE, H_SET_ROTATE, H_SET_DISPLAY, H_SET_PINCH, H_SET_THEME,
    H_MORE_OPEN, H_MORE_RENAME, H_MORE_MOVE, H_MORE_PDF, H_MORE_DELETE,
    H_FMORE_RENAME, H_FMORE_REMOVE,
    H_MOVE_TO, H_MOVE_TOP, H_MOVE_NEW,
    H_DELETE_YES,
};

typedef struct {
    bool is_folder;
    int nb;               // index into books (notebooks)
    char folder[STORE_NAME_LEN];
    int count;            // notebooks inside (folders)
} Item;

struct LibraryState {
    NotebookInfo *books;
    int nbooks;
    char (*folders)[STORE_NAME_LEN];
    int nfolders;
    Item *items;
    int nitems;
    char cur_folder[STORE_NAME_LEN];  // "" = top level
    int page;
    Canvas *thumbs;       // per notebook, lazily loaded (px == NULL until then)
    bool *thumb_tried;
    Canvas paper_prev[PAPER_COUNT];
    Canvas custom_prev;   // one custom music layout's preview at a time
    PaperKind custom_prev_k;
    MusicBuild mb;

    Modal modal;
    KbPurpose kbp;
    Keyboard kb;
    DispMenu dm;
    int target;           // item index for MORE / MOVE / DELETE
    PaperKind new_paper;
    char new_name[STORE_NAME_LEN];
    int move_page;
    uint32_t kb_settle_at;  // when to clean the keyboard up with one GL16 (0 = none)

    HitList hits;
    Press press;
    const Hit *pressed;
    Hit pressed_copy;
};

#define S (a->lib)

// ---- layout ----
static int top_h(void) { return ui_px(190); }
static int bottom_h(void) { return ui_px(170); }
static int margin(void) { return ui_px(50); }

static void grid_geom(App *a, int *cols, int *rows, int *cw, int *ch) {
    *cols = a->w > a->h ? 5 : 3;  // more, smaller cards when the Kindle is sideways
    *cw = (a->w - margin() * (*cols + 1)) / *cols;
    int th = *cw * 4 / 3;
    *ch = th + ui_px(150);
    int avail = a->h - top_h() - bottom_h() - margin();
    *rows = avail / (*ch + margin());
    if (*rows < 1) *rows = 1;
}

static int per_page(App *a) {
    int c, r, cw, ch;
    grid_geom(a, &c, &r, &cw, &ch);
    return c * r;
}

// ---- data ----

static void free_lists(App *a) {
    for (int i = 0; i < S->nbooks; i++) canvas_free(&S->thumbs[i]);
    free(S->thumbs);
    free(S->thumb_tried);
    free(S->books);
    free(S->folders);
    free(S->items);
    S->thumbs = NULL;
    S->thumb_tried = NULL;
    S->books = NULL;
    S->folders = NULL;
    S->items = NULL;
    S->nbooks = S->nfolders = S->nitems = 0;
}

static void reload(App *a) {
    free_lists(a);
    store_list(&S->books, &S->nbooks);
    store_list_folders(&S->folders, &S->nfolders);
    S->thumbs = calloc((size_t)(S->nbooks ? S->nbooks : 1), sizeof *S->thumbs);
    S->thumb_tried = calloc((size_t)(S->nbooks ? S->nbooks : 1), sizeof *S->thumb_tried);
    S->items = calloc((size_t)(S->nbooks + S->nfolders + 1), sizeof *S->items);
    if (!S->items) return;
    if (!S->cur_folder[0])
        for (int f = 0; f < S->nfolders; f++) {
            Item *it = &S->items[S->nitems++];
            it->is_folder = true;
            strlcpy_q(it->folder, S->folders[f], sizeof it->folder);
            for (int i = 0; i < S->nbooks; i++) it->count += !strcmp(S->books[i].folder, S->folders[f]);
        }
    for (int i = 0; i < S->nbooks; i++)
        if (!strcmp(S->books[i].folder, S->cur_folder)) S->items[S->nitems++] = (Item){.nb = i};
    int pages = (S->nitems + per_page(a) - 1) / per_page(a);
    S->page = clampi(S->page, 0, pages > 0 ? pages - 1 : 0);
}

static void unique_name(App *a, const char *base, char *out, size_t cap) {
    strlcpy_q(out, base, cap);
    for (int k = 2; k < 1000; k++) {
        bool taken = false;
        for (int i = 0; i < S->nbooks && !taken; i++) taken = !strcmp(S->books[i].name, out);
        if (!taken) return;
        snprintf(out, cap, "%s %d", base, k);
    }
}

static const Canvas *paper_preview(App *a, PaperKind k) {
    Canvas *c;
    if (paper_is_custom(k)) {
        c = &S->custom_prev;
        if (c->px && S->custom_prev_k != k) canvas_free(c);
        S->custom_prev_k = k;
    } else {
        c = &S->paper_prev[clampi((int)k, 0, PAPER_COUNT - 1)];
    }
    if (!c->px) {
        Canvas full;
        int pw, ph;
        store_page_size(&pw, &ph);
        if (canvas_init(&full, pw, ph) != 0) return NULL;
        paper_render(&full, k, (Rect){0, 0, pw, ph}, a->dpi);
        if (canvas_init(c, pw / 4, ph / 4) == 0) canvas_downsample_keep_lines(c, &full, 0.6f);
        canvas_free(&full);
    }
    return c->px ? c : NULL;
}

static const Canvas *thumb_for(App *a, int nb) {
    if (!S->thumb_tried[nb]) {
        S->thumb_tried[nb] = true;
        store_load_thumb(S->books[nb].id, &S->thumbs[nb]);
    }
    if (S->thumbs[nb].px) return &S->thumbs[nb];
    return paper_preview(a, S->books[nb].paper);
}

// ---- drawing ----

static void draw_card(App *a, Rect r, int idx) {
    Canvas *c = &a->screen;
    Item *it = &S->items[idx];
    int th = r.w * 4 / 3;
    Rect tb = {r.x, r.y, r.w, th};
    ui_frame(c, tb, ui_px(16), 255, true, 0);
    if (it->is_folder) {
        ui_icon(c, ICON_FOLDER, rect_inflate(tb, -r.w / 5), 0);
    } else {
        const Canvas *t = thumb_for(a, it->nb);
        Canvas *cc = c;
        Rect saved = cc->clip;
        canvas_set_clip(cc, rect_inflate(tb, -ui_px(3)));
        if (t) canvas_blit_fit(cc, t, rect_inflate(tb, -ui_px(3)));
        cc->clip = saved;
    }
    ui_frame(c, tb, ui_px(16), 0, false, ui_px(4));
    char line2[96];
    const char *title;
    if (it->is_folder) {
        title = it->folder;
        snprintf(line2, sizeof line2, "%d notebook%s", it->count, it->count == 1 ? "" : "s");
    } else {
        const NotebookInfo *nb = &S->books[it->nb];
        title = nb->name;
        char when[32];
        format_time_short(nb->modified, when, sizeof when);
        snprintf(line2, sizeof line2, "%d page%s, %s", nb->npages, nb->npages == 1 ? "" : "s", when);
    }
    int more = ui_px(110);
    Rect tr = {r.x, r.y + th + ui_px(14), r.w - more - ui_px(10), ui_px(70)};
    // Gothic names use the pixel face's regular weight: its overprinted bold turns a name in
    // capitals into a solid black bar.
    font_draw_left(c, ui_theme() == THEME_GOTHIC ? FONT_REGULAR : FONT_BOLD, ui_text_size(), tr, title, 0);
    font_draw_left(c, FONT_REGULAR, ui_small_size(), (Rect){r.x, tr.y + tr.h, r.w - more - ui_px(10), ui_px(56)}, line2,
                   60);
    Rect mr = {r.x + r.w - more, r.y + th + ui_px(20), more, ui_px(110)};
    ui_button(c, mr, ICON_MORE, NULL, BTN_PLAIN);
    hits_add(&S->hits, tb, H_CARD, idx);
    hits_add(&S->hits, mr, H_CARD_MORE, idx);
}

static void draw_base(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int th = top_h(), m = margin();
    int bw = ui_px(170), bh = ui_px(150), by = (th - bh) / 2;
    int x = m;
    if (S->cur_folder[0]) {
        Rect br = {x, by, bh, bh};
        ui_button(c, br, ICON_BACK, NULL, BTN_PLAIN);
        hits_add(&S->hits, br, H_BACK, 0);
        x += bh + ui_px(30);
    }
    struct {
        IconKind icon;
        const char *label;
        int id;
        bool show;
    } btns[] = {
        {ICON_EXIT, "Exit", H_EXIT, true},
        {ICON_GEAR, "Settings", H_SETTINGS, true},
        {ICON_SHARE, "Share", H_SHARE, true},
        {ICON_FOLDER_PLUS, "Folder", H_NEWFOLDER, !S->cur_folder[0]},
        {ICON_PLUS, "New", H_NEW, true},
    };
    int rx = a->w - m;
    for (size_t i = 0; i < sizeof btns / sizeof btns[0]; i++) {
        if (!btns[i].show) continue;
        rx -= bw;
        Rect r = {rx, by, bw, bh};
        ui_button(c, r, btns[i].icon, btns[i].label, i == 4 ? BTN_PRIMARY : BTN_PLAIN);
        hits_add(&S->hits, r, btns[i].id, 0);
        rx -= ui_px(20);
    }
    if (S->cur_folder[0]) {
        ui_icon(c, ICON_FOLDER, (Rect){x, by + ui_px(25), ui_px(100), ui_px(100)}, 0);
        x += ui_px(115);
    } else if (ui_theme() == THEME_GOTHIC) {
        ui_emblem(c, (float)(x + ui_px(55)), (float)th / 2.f, (float)ui_px(52), 0, 255);
        x += ui_px(140);
    }
    font_draw_left(c, FONT_TITLE, ui_title_size() * (ui_theme() == THEME_GOTHIC ? 1.8f : 1.25f),
                   (Rect){x, 0, rx - x - ui_px(20), th},
                   S->cur_folder[0] ? S->cur_folder : APP_NAME, 0);
    ui_rule(c, 0, a->w, th, ui_px(4), 0);
    ui_frieze(c, 0, a->w, th + ui_px(9), ui_px(28), 0);

    int cols, rows, cw, ch;
    grid_geom(a, &cols, &rows, &cw, &ch);
    int pp = cols * rows;
    if (S->nitems == 0) {
        Rect er = {m, th + ui_px(300), a->w - 2 * m, ui_px(100)};
        font_draw_centered(c, FONT_BOLD, ui_title_size(), er,
                           S->cur_folder[0] ? "This folder is empty" : "No notebooks yet", 0);
        er.y += ui_px(110);
        font_draw_centered(c, FONT_REGULAR, ui_text_size(), er, "Tap New to start one.", 60);
    }
    for (int i = 0; i < pp; i++) {
        int idx = S->page * pp + i;
        if (idx >= S->nitems) break;
        int col = i % cols, row = i / cols;
        draw_card(a, (Rect){m + col * (cw + m), th + m + row * (ch + m), cw, ch}, idx);
    }
    // Pager.
    int pages = (S->nitems + pp - 1) / pp;
    if (pages < 1) pages = 1;
    int y = a->h - bottom_h();
    ui_rule(c, 0, a->w, y - ui_px(10), ui_px(3), 0);
    Rect pr = {m, y + ui_px(15), ui_px(220), bottom_h() - ui_px(30)};
    Rect nr = {a->w - m - ui_px(220), pr.y, ui_px(220), pr.h};
    ui_button(c, pr, ICON_PREV, NULL, S->page > 0 ? BTN_PLAIN : BTN_DISABLED);
    ui_button(c, nr, ICON_NEXT, NULL, S->page < pages - 1 ? BTN_PLAIN : BTN_DISABLED);
    if (S->page > 0) hits_add(&S->hits, pr, H_PREV, 0);
    if (S->page < pages - 1) hits_add(&S->hits, nr, H_NEXT, 0);
    char pl[48];
    snprintf(pl, sizeof pl, "Page %d of %d", S->page + 1, pages);
    font_draw_centered(c, FONT_BOLD, ui_text_size(), (Rect){pr.x + pr.w, pr.y, nr.x - pr.x - pr.w, pr.h}, pl, 0);
}

static Rect sheet_rect(App *a, int h) {
    int m = ui_px(70);
    if (h > a->h - 2 * ui_px(40)) h = a->h - 2 * ui_px(40);  // landscape: keep it on screen
    int w = a->w - 2 * m;
    if (w > ui_px(1800)) w = ui_px(1800);
    return (Rect){(a->w - w) / 2, (a->h - h) / 3, w, h};
}

static void draw_new(App *a) {
    Canvas *c = &a->screen;
    Rect card = sheet_rect(a, ui_px(2300));
    Rect in = ui_sheet(c, card, "New notebook");
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){in.x, in.y, in.w, ui_px(70)}, "Paper", 0);
    const int ntiles = PAPER_PICKER_COUNT;
    int gap = ui_px(30), cols = ntiles > 15 ? 6 : ntiles > 8 ? 5 : 4, rows = (ntiles + cols - 1) / cols;
    int tw = (in.w - gap * (cols - 1)) / cols;
    if (tw > ui_px(300)) tw = ui_px(300);
    // Fit the height too (landscape): leave room for the name rows and the buttons.
    int avail = in.h - ui_px(80) - ui_px(420);
    int max_th = avail / rows - ui_px(100);
    if (tw * 4 / 3 > max_th) tw = max_th * 3 / 4;
    int tht = tw * 4 / 3, cell_h = tht + ui_px(100);
    int gridw = cols * tw + (cols - 1) * gap;
    int ty = in.y + ui_px(80);
    for (int i = 0; i < ntiles; i++) {
        // The last tile is Custom music: the chosen layout, or the last one built.
        bool custom = i == PAPER_COUNT;
        bool new_custom = paper_is_custom(S->new_paper);
        PaperKind k = custom ? (new_custom ? S->new_paper : a->settings.music_layout) : paper_at(i);
        Rect tr = {in.x + (in.w - gridw) / 2 + (i % cols) * (tw + gap), ty + (i / cols) * cell_h, tw, tht};
        canvas_fill(c, tr, 255);
        const Canvas *pv = paper_preview(a, k);
        if (pv) canvas_blit_fit(c, pv, tr);
        bool sel = custom ? new_custom : S->new_paper == k;
        ui_frame(c, rect_inflate(tr, sel ? ui_px(10) : 0), ui_px(12), 0, false, sel ? ui_px(9) : ui_px(3));
        Rect lr = {tr.x - gap / 2, tr.y + tht + ui_px(18), tw + gap, ui_px(64)};
        font_draw_centered(c, sel ? FONT_BOLD : FONT_REGULAR, ui_small_size() * 0.9f, lr,
                           custom ? "Custom music..." : paper_name(k), 0);
        hits_add(&S->hits, (Rect){tr.x, tr.y, tr.w, tht + ui_px(90)}, custom ? H_PAPER_CUSTOM : H_PAPER, (int)k);
    }
    int y = ty + rows * cell_h + ui_px(20);
    char line[200];
    snprintf(line, sizeof line, "Name: %s", S->new_name);
    Rect rn = {in.x + in.w - ui_px(300), y, ui_px(300), ui_px(120)};
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){in.x, y, in.w - ui_px(330), ui_px(120)}, line, 0);
    ui_button(c, rn, ICON_RENAME, "Rename", BTN_PLAIN);
    hits_add(&S->hits, rn, H_NEW_RENAME, 0);
    y += ui_px(140);
    snprintf(line, sizeof line, "In: %s", S->cur_folder[0] ? S->cur_folder : "Library (top level)");
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)}, line, 60);
    int bh = ui_px(140), bw = ui_px(380);
    Rect cancel = {in.x, in.y + in.h - bh, bw, bh};
    Rect create = {in.x + in.w - bw, in.y + in.h - bh, bw, bh};
    ui_button(c, cancel, ICON_NONE, "Cancel", BTN_PLAIN);
    ui_button(c, create, ICON_NONE, "Create", BTN_PRIMARY);
    hits_add(&S->hits, cancel, H_CANCEL, 0);
    hits_add(&S->hits, create, H_NEW_CREATE, 0);
}

typedef struct {
    IconKind icon;
    const char *label;
    int id, arg;
    ButtonStyle st;
} MenuRow;

// A vertical list of full-width buttons inside a sheet.
static void draw_menu(App *a, const char *title, const MenuRow *rows, int n) {
    int rh = ui_px(140), gap = ui_px(20);
    Rect card = sheet_rect(a, ui_px(120) + ui_px(72) + n * (rh + gap));
    Rect in = ui_sheet(&a->screen, card, title);
    for (int i = 0; i < n; i++) {
        Rect r = {in.x, in.y + i * (rh + gap), in.w, rh};
        Canvas *c = &a->screen;
        ui_frame(c, r, ui_px(16), rows[i].st == BTN_SELECTED ? 0 : 255, true, 0);
        ui_frame(c, r, ui_px(16), rows[i].st == BTN_DISABLED ? 170 : 0, false,
                          rows[i].st == BTN_PRIMARY ? ui_px(6) : ui_px(3));
        uint8_t fg = rows[i].st == BTN_SELECTED ? 255 : rows[i].st == BTN_DISABLED ? 170 : 0;
        if (rows[i].icon) ui_icon(c, rows[i].icon, (Rect){r.x + ui_px(20), r.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, fg);
        font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r.x + rh + ui_px(10), r.y, r.w - rh - ui_px(30), r.h},
                       rows[i].label, fg);
        if (rows[i].st != BTN_DISABLED) hits_add(&S->hits, r, rows[i].id, rows[i].arg);
    }
}

static void draw_settings(App *a) {
    Canvas *c = &a->screen;
    Rect card = sheet_rect(a, ui_px(1820));
    Rect in = ui_sheet(c, card, "Settings");
    int rh = ui_px(140), gap = ui_px(24), y = in.y;
    Rect r1 = {in.x, y, in.w, rh};
    ui_button(c, r1, ICON_NONE, "", BTN_PLAIN);
    ui_icon(c, ICON_TUNE, (Rect){r1.x + ui_px(20), r1.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, 0);
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r1.x + rh + ui_px(10), r1.y, r1.w, rh},
                   "Pen sensitivity (pressure + tilt per tool)", 0);
    hits_add(&S->hits, r1, H_SET_PENS, 0);
    y += rh + gap;
    Rect r2 = {in.x, y, in.w, rh};
    ui_button(c, r2, ICON_NONE, "", BTN_PLAIN);
    ui_icon(c, ICON_TARGET, (Rect){r2.x + ui_px(20), r2.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, 0);
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r2.x + rh + ui_px(10), r2.y, r2.w, rh}, "Calibrate pen and touch",
                   0);
    hits_add(&S->hits, r2, H_SET_CALIB, 0);
    y += rh + gap;
    Rect r2b = {in.x, y, in.w, rh};
    ui_button(c, r2b, ICON_NONE, "", BTN_PLAIN);
    ui_icon(c, ICON_BRIGHTNESS, (Rect){r2b.x + ui_px(20), r2b.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, 0);
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r2b.x + rh + ui_px(10), r2b.y, r2b.w, rh},
                   "Display: brightness, warmth, invert", 0);
    hits_add(&S->hits, r2b, H_SET_DISPLAY, 0);
    y += rh + gap;
    Rect rt = {in.x, y, in.w, ui_px(110)};
    canvas_fill(c, rt, 255);
    ui_icon(c, ICON_CASTLE, (Rect){rt.x, rt.y + ui_px(15), ui_px(80), ui_px(80)}, 0);
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){rt.x + ui_px(110), rt.y, rt.w - ui_px(110), rt.h},
                   ui_theme() == THEME_GOTHIC ? "Style: Gothic (tap for Classic)" : "Style: Classic (tap for Gothic)", 0);
    hits_add(&S->hits, rt, H_SET_THEME, 0);
    y += ui_px(130);
    Rect r3 = {in.x, y, in.w, ui_px(110)};
    ui_checkbox(c, r3, "Mirror PDFs to grimoire/PDFs (USB)", a->settings.mirror_pdf);
    hits_add(&S->hits, r3, H_SET_MIRROR, 0);
    y += ui_px(130);
    Rect r4 = {in.x, y, in.w, ui_px(110)};
    static const char *btn_names[3] = {"does nothing", "erases", "highlights"};
    char btn_line[96];
    snprintf(btn_line, sizeof btn_line, "Holding the pen button %s (tap to change)",
             btn_names[clampi(a->settings.side_button, 0, 2)]);
    canvas_fill(c, r4, 255);
    ui_icon(c, ICON_HIGHLIGHTER, (Rect){r4.x, r4.y + ui_px(15), ui_px(80), ui_px(80)}, 0);
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){r4.x + ui_px(110), r4.y, r4.w - ui_px(110), r4.h},
                   btn_line, 0);
    hits_add(&S->hits, r4, H_SET_SIDE, 0);
    y += ui_px(130);
    Rect r5 = {in.x, y, in.w, ui_px(110)};
    ui_checkbox(c, r5, "Turn with the Kindle (portrait and landscape)", a->settings.follow_rotation);
    hits_add(&S->hits, r5, H_SET_ROTATE, 0);
    y += ui_px(130);
    Rect r6 = {in.x, y, in.w, ui_px(110)};
    ui_checkbox(c, r6, "Pinch with two fingers to zoom", a->settings.pinch_zoom);
    hits_add(&S->hits, r6, H_SET_PINCH, 0);
    y += ui_px(150);
    const InputCaps *caps = input_caps();
    char about[256];
    snprintf(about, sizeof about, "Grimoire %s  pen: %s%s%s", GRIMOIRE_VERSION, caps && caps->has_pen ? caps->pen_name : "none",
             caps && caps->has_pressure ? ", pressure" : "", caps && caps->has_tilt ? ", tilt" : "");
    font_draw_left(c, FONT_REGULAR, ui_small_size(), (Rect){in.x, y, in.w, ui_px(60)}, about, 60);
    snprintf(about, sizeof about, "Data: %s", store_root());
    font_draw_left(c, FONT_REGULAR, ui_small_size(), (Rect){in.x, y + ui_px(60), in.w, ui_px(60)}, about, 60);
    Rect done = {in.x + in.w - ui_px(380), in.y + in.h - ui_px(140), ui_px(380), ui_px(140)};
    ui_button(c, done, ICON_NONE, "Done", BTN_PRIMARY);
    hits_add(&S->hits, done, H_CANCEL, 0);
}

static void draw_modal(App *a) {
    Item *it = (S->target >= 0 && S->target < S->nitems) ? &S->items[S->target] : NULL;
    switch (S->modal) {
    case MOD_NEW: draw_new(a); break;
    case MOD_SETTINGS: draw_settings(a); break;
    case MOD_MORE: {
        if (!it) break;
        MenuRow rows[] = {
            {ICON_NOTEBOOK, "Open", H_MORE_OPEN, 0, BTN_PLAIN},
            {ICON_RENAME, "Rename", H_MORE_RENAME, 0, BTN_PLAIN},
            {ICON_FOLDER_MOVE, "Move to folder", H_MORE_MOVE, 0, BTN_PLAIN},
            {ICON_DOWNLOAD, "Save PDF to grimoire/PDFs now", H_MORE_PDF, 0, BTN_PLAIN},
            {ICON_TRASH, "Delete", H_MORE_DELETE, 0, BTN_PLAIN},
            {ICON_CLOSE, "Cancel", H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, S->books[it->nb].name, rows, 6);
        break;
    }
    case MOD_FOLDER_MORE: {
        if (!it) break;
        MenuRow rows[] = {
            {ICON_RENAME, "Rename folder", H_FMORE_RENAME, 0, BTN_PLAIN},
            {ICON_TRASH, it->count ? "Remove folder (move its notebooks out first)" : "Remove folder", H_FMORE_REMOVE, 0,
             it->count ? BTN_DISABLED : BTN_PLAIN},
            {ICON_CLOSE, "Cancel", H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, it->folder, rows, 3);
        break;
    }
    case MOD_MOVE: {
        if (!it) break;
        MenuRow rows[14];
        int n = 0;
        const char *curf = S->books[it->nb].folder;
        rows[n++] = (MenuRow){ICON_LIBRARY, "Library (top level)", H_MOVE_TOP, 0, curf[0] ? BTN_PLAIN : BTN_SELECTED};
        for (int f = 0; f < S->nfolders && n < 12; f++)
            rows[n++] = (MenuRow){ICON_FOLDER, S->folders[f], H_MOVE_TO, f, strcmp(curf, S->folders[f]) ? BTN_PLAIN : BTN_SELECTED};
        rows[n++] = (MenuRow){ICON_FOLDER_PLUS, "New folder...", H_MOVE_NEW, 0, BTN_PLAIN};
        rows[n++] = (MenuRow){ICON_CLOSE, "Cancel", H_CANCEL, 0, BTN_PLAIN};
        draw_menu(a, "Move to", rows, n);
        break;
    }
    case MOD_DELETE: {
        if (!it) break;
        char title[160];
        snprintf(title, sizeof title, "Delete \"%s\"?", S->books[it->nb].name);
        MenuRow rows[] = {
            {ICON_TRASH, "Delete (moves it to grimoire/trash)", H_DELETE_YES, 0, BTN_PRIMARY},
            {ICON_CLOSE, "Keep it", H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, title, rows, 2);
        break;
    }
    case MOD_KB: keyboard_draw(&S->kb, &a->screen, (Rect){0, 0, a->w, a->h}); break;
    case MOD_DISPLAY: dispmenu_draw(&S->dm, a); break;
    case MOD_MUSIC: draw_new(a); musicbuild_draw(&S->mb, a); break;
    default: break;
    }
}

static void redraw(App *a, RefreshMode m) {
    draw_base(a);
    if (S->modal != MOD_NONE) {
        hits_clear(&S->hits);  // a modal captures all input
        draw_modal(a);
    }
    app_invalidate_all(a, m);
}

static void open_modal(App *a, Modal m) {
    S->modal = m;
    redraw(a, REFRESH_UI);
}

static void close_modal(App *a) {
    S->modal = MOD_NONE;
    redraw(a, REFRESH_UI);
}

static void open_kb(App *a, KbPurpose p, const char *title, const char *initial) {
    S->kbp = p;
    keyboard_open(&S->kb, title, initial);
    S->modal = MOD_KB;
    redraw(a, REFRESH_UI);
}

// ---- actions ----

// Keeps grimoire/PDFs in step when a notebook is renamed, moved or deleted: the old mirror
// PDF goes away, and (unless deleting) a fresh one is written under the new name/folder.
static void mirror_follow(App *a, const char *id, bool deleting) {
    NotebookInfo *now = malloc(sizeof *now);
    if (!now) return;
    if (store_load_info(id, now) == 0) {
        for (int i = 0; i < S->nbooks; i++) {
            if (strcmp(S->books[i].id, id)) continue;
            char oldp[1200], newp[1200];
            export_mirror_path(&S->books[i], app_mirror_dir(), oldp, sizeof oldp);
            export_mirror_path(now, app_mirror_dir(), newp, sizeof newp);
            bool had = access(oldp, F_OK) == 0;
            if (had && !deleting && strcmp(oldp, newp)) {
                // A rename or move: move the PDF along (instant) instead of re-rendering it.
                char dir[1200];
                strlcpy_q(dir, newp, sizeof dir);
                char *slash = strrchr(dir, '/');
                if (slash) *slash = 0;
                mkdir_p(dir);
                if (rename(oldp, newp) != 0 && a->settings.mirror_pdf) {
                    unlink(oldp);
                    bg_mirror_pdf(id, a->dpi, app_mirror_dir(), false);
                }
            } else if (had && deleting) {
                unlink(oldp);
            }
        }
    }
    free(now);
}

static void create_notebook(App *a) {
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb) return;
    if (store_create(S->new_name, S->cur_folder, S->new_paper, nb) == 0) {
        S->modal = MOD_NONE;
        char id[STORE_ID_LEN];
        strlcpy_q(id, nb->id, sizeof id);
        free(nb);
        editor_open_notebook(a, id);
        return;
    }
    free(nb);
    app_toast(a, "Could not create the notebook (storage full?)", 3000);
}

static void kb_finished(App *a, bool ok) {
    const char *txt = S->kb.text;
    Item *it = (S->target >= 0 && S->target < S->nitems) ? &S->items[S->target] : NULL;
    Modal back = MOD_NONE;
    if (ok && txt[0]) switch (S->kbp) {
        case KBP_NEW_NAME:
            strlcpy_q(S->new_name, txt, sizeof S->new_name);
            back = MOD_NEW;
            break;
        case KBP_RENAME:
            if (it && !it->is_folder) {
                store_rename(S->books[it->nb].id, txt);
                mirror_follow(a, S->books[it->nb].id, false);
            }
            break;
        case KBP_NEW_FOLDER: store_add_folder(txt); break;
        case KBP_MOVE_NEW_FOLDER:
            store_add_folder(txt);
            if (it && !it->is_folder) {
                store_set_folder(S->books[it->nb].id, txt);
                mirror_follow(a, S->books[it->nb].id, false);
            }
            break;
        case KBP_RENAME_FOLDER:
            if (it && it->is_folder) {
                for (int i = 0; i < S->nbooks; i++)
                    if (!strcmp(S->books[i].folder, it->folder)) {
                        store_set_folder(S->books[i].id, txt);
                        mirror_follow(a, S->books[i].id, false);
                    }
                store_remove_folder(it->folder);
                store_add_folder(txt);
            }
            break;
        }
    else if (S->kbp == KBP_NEW_NAME)
        back = MOD_NEW;
    S->modal = back;
    reload(a);
    redraw(a, REFRESH_UI);
}

static void act(App *a, const Hit *h) {
    Item *it = (S->target >= 0 && S->target < S->nitems) ? &S->items[S->target] : NULL;
    switch (h->id) {
    case H_BACK:
        S->cur_folder[0] = 0;
        S->page = 0;
        reload(a);
        redraw(a, REFRESH_FULL);
        break;
    case H_NEW: {
        char base[64], when[32];
        format_time_short(wall_time(), when, sizeof when);
        char *comma = strchr(when, ',');
        if (comma) *comma = 0;  // "Oct 10"
        snprintf(base, sizeof base, "Notebook %s", when);
        unique_name(a, base, S->new_name, sizeof S->new_name);
        S->new_paper = PAPER_MEDIUM;
        open_modal(a, MOD_NEW);
        break;
    }
    case H_NEWFOLDER: open_kb(a, KBP_NEW_FOLDER, "Folder", ""); break;
    case H_SHARE: share_open(a); break;
    case H_SETTINGS: open_modal(a, MOD_SETTINGS); break;
    case H_EXIT: a->running = false; break;
    case H_PREV:
    case H_NEXT:
        // Only the visible page's thumbnails stay in memory (each is ~0.5 MB).
        for (int i = 0; i < S->nbooks; i++) {
            canvas_free(&S->thumbs[i]);
            S->thumb_tried[i] = false;
        }
        S->page += h->id == H_NEXT ? 1 : -1;
        redraw(a, REFRESH_UI);
        break;
    case H_CARD:
        if (h->arg >= S->nitems) break;
        if (S->items[h->arg].is_folder) {
            strlcpy_q(S->cur_folder, S->items[h->arg].folder, sizeof S->cur_folder);
            S->page = 0;
            reload(a);
            redraw(a, REFRESH_FULL);
        } else {
            char id[STORE_ID_LEN];
            strlcpy_q(id, S->books[S->items[h->arg].nb].id, sizeof id);
            editor_open_notebook(a, id);
        }
        break;
    case H_CARD_MORE:
        S->target = h->arg;
        open_modal(a, S->items[h->arg].is_folder ? MOD_FOLDER_MORE : MOD_MORE);
        break;
    case H_PAPER: S->new_paper = (PaperKind)h->arg; redraw(a, REFRESH_UI); break;
    case H_PAPER_CUSTOM:
        musicbuild_open(&S->mb, a, S->new_paper);
        open_modal(a, MOD_MUSIC);
        break;
    case H_NEW_RENAME: open_kb(a, KBP_NEW_NAME, "Name", S->new_name); break;
    case H_NEW_CREATE: create_notebook(a); break;
    case H_CANCEL: close_modal(a); break;
    case H_SET_PENS: S->modal = MOD_NONE; pensettings_open(a, a->settings.current_tool); break;
    case H_SET_CALIB: S->modal = MOD_NONE; calibrate_open(a); break;
    case H_SET_MIRROR:
        a->settings.mirror_pdf = !a->settings.mirror_pdf;
        settings_save(&a->settings);
        redraw(a, REFRESH_UI);
        break;
    case H_SET_PINCH:
        a->settings.pinch_zoom = !a->settings.pinch_zoom;
        settings_save(&a->settings);
        redraw(a, REFRESH_UI);
        break;
    case H_SET_THEME:
        a->settings.theme = a->settings.theme == THEME_CLASSIC ? THEME_GOTHIC : THEME_CLASSIC;
        ui_set_theme((UiTheme)a->settings.theme);
        settings_save(&a->settings);
        redraw(a, REFRESH_FULL);
        break;
    case H_SET_DISPLAY:
        dispmenu_open(&S->dm, a);
        open_modal(a, MOD_DISPLAY);
        break;
    case H_SET_ROTATE:
        a->settings.follow_rotation = !a->settings.follow_rotation;
        settings_save(&a->settings);
        if (!a->settings.follow_rotation && a->rotation != 0) {
            app_set_rotation(a, 0);  // back to portrait and stay there
            return;
        }
        if (a->settings.follow_rotation && input_caps() && input_caps()->has_orientation)
            a->pending_rotation = input_caps()->orientation;
        redraw(a, REFRESH_UI);
        break;
    case H_SET_SIDE:
        a->settings.side_button = (a->settings.side_button + 1) % 3;
        settings_save(&a->settings);
        redraw(a, REFRESH_UI);
        break;
    case H_MORE_OPEN:
        if (it) {
            char id[STORE_ID_LEN];
            strlcpy_q(id, S->books[it->nb].id, sizeof id);
            S->modal = MOD_NONE;
            editor_open_notebook(a, id);
        }
        break;
    case H_MORE_RENAME:
        if (it) open_kb(a, KBP_RENAME, "Name", S->books[it->nb].name);
        break;
    case H_MORE_MOVE: open_modal(a, MOD_MOVE); break;
    case H_MORE_PDF:
        if (it) {
            bg_mirror_pdf(S->books[it->nb].id, a->dpi, app_mirror_dir(), true);
            S->modal = MOD_NONE;
            redraw(a, REFRESH_UI);
            app_toast(a, "Saving the PDF in the background...", 2500);
        }
        break;
    case H_MORE_DELETE: open_modal(a, MOD_DELETE); break;
    case H_DELETE_YES:
        if (it && !it->is_folder) {
            char nb_path[1200];
            export_mirror_path(&S->books[it->nb], app_mirror_dir(), nb_path, sizeof nb_path);
            unlink(nb_path);  // the notebook goes to trash; its mirror PDF should not linger
            store_delete(S->books[it->nb].id);
        }
        S->modal = MOD_NONE;
        reload(a);
        redraw(a, REFRESH_FULL);
        app_toast(a, "Moved to grimoire/trash", 2500);
        break;
    case H_FMORE_RENAME:
        if (it) open_kb(a, KBP_RENAME_FOLDER, "Folder", it->folder);
        break;
    case H_FMORE_REMOVE:
        if (it) store_remove_folder(it->folder);
        S->modal = MOD_NONE;
        reload(a);
        redraw(a, REFRESH_UI);
        break;
    case H_MOVE_TOP:
    case H_MOVE_TO:
        if (it && !it->is_folder) {
            store_set_folder(S->books[it->nb].id, h->id == H_MOVE_TOP ? "" : S->folders[h->arg]);
            mirror_follow(a, S->books[it->nb].id, false);
        }
        S->modal = MOD_NONE;
        reload(a);
        redraw(a, REFRESH_UI);
        break;
    case H_MOVE_NEW: open_kb(a, KBP_MOVE_NEW_FOLDER, "Folder", ""); break;
    default: break;
    }
}

// ---- screen interface ----

void library_init(App *a) {
    a->lib = calloc(1, sizeof *a->lib);
    S->target = -1;
}

void library_free(App *a) {
    if (!S) return;
    free_lists(a);
    for (int k = 0; k < PAPER_COUNT; k++) canvas_free(&S->paper_prev[k]);
    canvas_free(&S->custom_prev);
    musicbuild_free(&S->mb);
    free(S);
    a->lib = NULL;
}

void library_enter(App *a) {
    S->modal = MOD_NONE;
    S->pressed = NULL;
    reload(a);
    redraw(a, REFRESH_FULL);
}

void library_leave(App *a) { S->pressed = NULL; }

void library_event(App *a, const InEvent *ev) {
    int ph = app_pointer_phase(ev);
    if (ph < 0) return;
    if (S->modal == MOD_DISPLAY) {
        Rect d;
        DmResult r = dispmenu_pointer(&S->dm, a, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == DM_CLOSED) {
            S->modal = MOD_SETTINGS;  // back to the settings sheet it was opened from
            redraw(a, REFRESH_UI);
        } else if (r == DM_INVERTED) {
            redraw(a, REFRESH_FULL);
        } else if (!rect_is_empty(d)) {
            app_invalidate(a, d, ph == 2 ? REFRESH_UI : REFRESH_FAST);
        }
        return;
    }
    if (S->modal == MOD_MUSIC) {
        Rect d;
        MbResult r = musicbuild_pointer(&S->mb, a, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == MB_DONE || r == MB_CANCEL) {
            if (r == MB_DONE) S->new_paper = musicbuild_result(&S->mb);
            musicbuild_free(&S->mb);
            S->modal = MOD_NEW;
            redraw(a, REFRESH_UI);
        } else if (!rect_is_empty(d)) {
            app_invalidate(a, d, REFRESH_UI);
        }
        return;
    }
    if (S->modal == MOD_KB) {
        Rect d;
        KbResult r = keyboard_pointer(&S->kb, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == KB_DONE || r == KB_CANCEL) {
            kb_finished(a, r == KB_DONE);
            return;
        }
        if (!rect_is_empty(d) || !rect_is_empty(S->kb.field_dirty)) {
            // Typing stays on the fast waveform: the text first (what the eye is on), then
            // the key. One clean GL16 of the keyboard follows once typing pauses.
            keyboard_draw(&S->kb, &a->screen, (Rect){0, 0, a->w, a->h});
            if (!rect_is_empty(S->kb.field_dirty)) {
                app_invalidate(a, S->kb.field_dirty, REFRESH_FAST);
                app_flush(a);
            }
            app_invalidate(a, d, REFRESH_FAST);
            S->kb_settle_at = now_ms() + 900;
            if (!S->kb_settle_at) S->kb_settle_at = 1;
        }
        return;
    }
    if (ph == 0) {
        press_begin(&S->press, ev->x, ev->y, ev->t_ms);
        const Hit *h = hits_find(&S->hits, ev->x, ev->y);
        S->pressed = NULL;
        if (h) {
            S->pressed_copy = *h;
            S->pressed = &S->pressed_copy;
            app_press_feedback(a, h->r);
        }
    } else if (ph == 1) {
        press_move(&S->press, ev->x, ev->y);
    } else if (S->pressed) {
        Hit h = *S->pressed;
        S->pressed = NULL;
        canvas_invert(&a->screen, h.r);  // undo the pressed look
        app_invalidate(a, h.r, REFRESH_UI);
        press_move(&S->press, ev->x, ev->y);
        if (press_is_tap(&S->press, h.r) || rect_contains(h.r, (int)ev->x, (int)ev->y)) act(a, &h);
    }
}

void library_tick(App *a, uint32_t now) {
    if (S->kb_settle_at && (int32_t)(now - S->kb_settle_at) >= 0) {
        S->kb_settle_at = 0;
        if (S->modal == MOD_KB && S->kb.pressed_key < 0) app_invalidate(a, S->kb.area, REFRESH_UI);
    }
}

// ---- headless checks ----

// The script runner's "library.*" keys (app_probe forwards them without the prefix).
int library_probe(App *a, const char *key, char *out, size_t cap) {
    static const char *const sheets[] = {
        [MOD_NONE] = "none",
        [MOD_NEW] = "new",
        [MOD_SETTINGS] = "settings",
        [MOD_MORE] = "more",
        [MOD_FOLDER_MORE] = "folder-more",
        [MOD_MOVE] = "move",
        [MOD_DELETE] = "delete",
        [MOD_KB] = "keyboard",
        [MOD_DISPLAY] = "display",
        [MOD_MUSIC] = "music",
    };
    if (!S) return -1;
    if (!strcmp(key, "sheet")) {
        const char *m = (size_t)S->modal < sizeof sheets / sizeof sheets[0] ? sheets[S->modal] : NULL;
        if (m) snprintf(out, cap, "%s", m);
        else snprintf(out, cap, "%d", (int)S->modal);
    } else if (!strcmp(key, "folder")) {
        snprintf(out, cap, "%s", S->cur_folder);
    } else if (!strcmp(key, "items")) {
        snprintf(out, cap, "%d", S->nitems);
    } else if (!strcmp(key, "page")) {
        snprintf(out, cap, "%d", S->page + 1);
    } else if (!strcmp(key, "pages")) {
        int pp = per_page(a), pages = (S->nitems + pp - 1) / pp;
        snprintf(out, cap, "%d", pages > 0 ? pages : 1);
    } else if (!strcmp(key, "text")) {
        snprintf(out, cap, "%s", S->modal == MOD_KB ? S->kb.text : "");
    } else if (!strcmp(key, "paper")) {
        if (paper_is_custom(S->new_paper)) snprintf(out, cap, "custom");
        else snprintf(out, cap, "%d", (int)S->new_paper);
    } else {
        return -1;
    }
    return 0;
}
