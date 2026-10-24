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
#include "bgimage.h"
#include "musicbuild.h"
#include "keyboard.h"
#include "i18n.h"
#include "tplsheet.h"
#include "koreader.h"

typedef enum { MOD_NONE, MOD_NEW, MOD_SETTINGS, MOD_MORE, MOD_FOLDER_MORE, MOD_MOVE, MOD_DELETE, MOD_KB, MOD_DISPLAY, MOD_MUSIC, MOD_LANGUAGE,
               MOD_TEMPLATES, MOD_KOREADER } Modal;
typedef enum { KBP_NEW_NAME, KBP_RENAME, KBP_NEW_FOLDER, KBP_MOVE_NEW_FOLDER, KBP_RENAME_FOLDER } KbPurpose;

enum {
    H_NONE, H_BACK, H_NEW, H_NEWFOLDER, H_SHARE, H_SETTINGS, H_EXIT, H_PREV, H_NEXT,
    H_CARD, H_CARD_MORE,
    H_PAPER, H_PAPER_CUSTOM, H_NEW_RENAME, H_NEW_CREATE, H_CANCEL,
    H_SET_PENS, H_SET_CALIB, H_SET_MIRROR, H_SET_SIDE, H_SET_ROTATE, H_SET_DISPLAY, H_SET_PINCH, H_SET_THEME,
    H_SET_LANGUAGE, H_LANG_PICK, H_LANG_BACK,
    H_MORE_OPEN, H_MORE_RENAME, H_MORE_MOVE, H_MORE_PDF, H_MORE_DELETE,
    H_FMORE_RENAME, H_FMORE_REMOVE,
    H_MOVE_TO, H_MOVE_TOP, H_MOVE_NEW,
    H_DELETE_YES,
    H_PAPER_MORE, H_KOREADER, H_KO_ITEM,
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
    bool *thumb_picture;  // thumbs[i] is the notebook's picture paper (picture_card), not its thumb.pgm
    Canvas paper_prev[PAPER_COUNT];
    Canvas custom_prev;   // one custom music layout's preview at a time
    PaperKind custom_prev_k;
    MusicBuild mb;
    TplSheet ts;          // New notebook's More papers...: the Templates sheet over it
    char new_tpl[TEMPLATE_PATH_MAX];  // the template New notebook makes its notebook on ("" none)
    Canvas tpl_prev;      // and its preview, for its tile
    unsigned card_gen;    // bgimage_generation when the cards were last read
    uint64_t *card_stamp; // per notebook: what thumbs[i] was read from (store_thumb_stamp), or CARD_STALE
    int cards_redrawn;    // times a background job changed a card on screen (the probe's)

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

// Whether an empty Library (or folder) says so, and what to do: not under a sheet (a short one,
// the language list, would cut through the lines), but above the keyboard, which leaves them be.
static bool empty_hint_shown(App *a) { return S->nitems == 0 && (S->modal == MOD_NONE || S->modal == MOD_KB); }

// ---- data ----

static void free_lists(App *a) {
    for (int i = 0; i < S->nbooks; i++) canvas_free(&S->thumbs[i]);
    free(S->thumbs);
    free(S->thumb_tried);
    free(S->thumb_picture);
    free(S->card_stamp);
    free(S->books);
    free(S->folders);
    free(S->items);
    S->thumbs = NULL;
    S->thumb_tried = NULL;
    S->thumb_picture = NULL;
    S->card_stamp = NULL;
    S->books = NULL;
    S->folders = NULL;
    S->items = NULL;
    S->nbooks = S->nfolders = S->nitems = 0;
}

static void reload(App *a) {
    free_lists(a);
    S->card_gen = bgimage_generation();  // (the cards read from now on are up to date)
    store_list(&S->books, &S->nbooks);
    store_list_folders(&S->folders, &S->nfolders);
    S->thumbs = calloc((size_t)(S->nbooks ? S->nbooks : 1), sizeof *S->thumbs);
    S->thumb_tried = calloc((size_t)(S->nbooks ? S->nbooks : 1), sizeof *S->thumb_tried);
    S->thumb_picture = calloc((size_t)(S->nbooks ? S->nbooks : 1), sizeof *S->thumb_picture);
    S->card_stamp = calloc((size_t)(S->nbooks ? S->nbooks : 1), sizeof *S->card_stamp);
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
    // A newer version's paper (also as a card's fallback), and an image paper: that is one of a
    // notebook's own pictures (picture_card), here blank.
    if (!paper_is_known(k) || paper_is_image(k)) k = PAPER_BLANK;
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

// A card drawn with a picture white in it (its file not made yet): read again once a background
// job has ended (library_tick). No thumb.pgm has this stamp (store_thumb_stamp: 0 or odd).
#define CARD_STALE 2u

// The card of a notebook with no thumbnail yet whose paper is a picture: that picture (white while
// its file is missing), never another paper.
static void picture_card(App *a, int nb) {
    const NotebookInfo *b = &S->books[nb];
    int pw, ph;
    store_notebook_page_size(b, &pw, &ph);
    Canvas full;
    if (canvas_init(&full, pw, ph) != 0) return;
    bgimage_take_missing();  // (what drew white before)
    bgimage_render(&full, b->id, b->paper, (Rect){0, 0, pw, ph});
    if (bgimage_take_missing() && S->card_stamp) S->card_stamp[nb] = CARD_STALE;
    if (canvas_init(&S->thumbs[nb], pw / 4, ph / 4) == 0) canvas_downsample_keep_lines(&S->thumbs[nb], &full, 0.6f);
    canvas_free(&full);
    S->thumb_picture[nb] = S->thumbs[nb].px != NULL;
}

static const Canvas *thumb_for(App *a, int nb) {
    if (!S->thumb_tried[nb]) {
        S->thumb_tried[nb] = true;
        S->thumb_picture[nb] = false;
        if (S->card_stamp) S->card_stamp[nb] = store_thumb_stamp(S->books[nb].id);  // (before it is read)
        if (store_load_thumb(S->books[nb].id, &S->thumbs[nb]) != 0 && paper_is_image(S->books[nb].paper))
            picture_card(a, nb);
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
    char line2[128], label[STORE_NAME_LEN + 32];
    const char *title;
    if (it->is_folder) {
        title = ui_name_label(it->folder, label, sizeof label);
        // TRANSLATORS: under a folder's card in the Library.
        snprintf(line2, sizeof line2, NG_("%d notebook", "%d notebooks", it->count), it->count);
    } else {
        const NotebookInfo *nb = &S->books[it->nb];
        title = ui_name_label(nb->name, label, sizeof label);
        char when[64];
        format_time_short(nb->modified, when, sizeof when);
        // TRANSLATORS: under a notebook's card in the Library: its pages and when it last changed (a
        // date and time, "Oct 10, 14:32"). Sideways the line has 316 px at 34 px; when it is longer,
        // the time is left out.
        snprintf(line2, sizeof line2, NG_("%d page, %s", "%d pages, %s", nb->npages), nb->npages, when);
    }
    int more = ui_px(110);
    Rect tr = {r.x, r.y + th + ui_px(14), r.w - more - ui_px(10), ui_px(70)};
    // Gothic names use the pixel face's regular weight: its overprinted bold turns a name in
    // capitals into a solid black bar.
    font_draw_left_name(c, ui_theme() == THEME_GOTHIC ? FONT_REGULAR : FONT_BOLD, ui_text_size(), tr, title, 0);
    Rect lr = {r.x, tr.y + tr.h, r.w - more - ui_px(10), ui_px(56)};
    float ls = ui_small_size();
    if (font_measure(FONT_REGULAR, ls, line2) > lr.w) {
        // Too long for the card (sideways in Classic, or in a longer language): a size smaller by
        // up to a fifth where the face has one (Classic's), else the date without its time.
        float fs = font_fit_px(FONT_REGULAR, ls, ls * 0.8f, line2, lr.w);
        if (fs > 0.f) {
            ls = fs;
        } else if (!it->is_folder) {
            const NotebookInfo *nb = &S->books[it->nb];
            char day[64];
            format_date_short(nb->modified, day, sizeof day);
            snprintf(line2, sizeof line2, NG_("%d page, %s", "%d pages, %s", nb->npages), nb->npages, day);
        }
    }
    font_draw_left(c, FONT_REGULAR, ls, lr, line2, 60);
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
        // TRANSLATORS: the Library's top bar, a word under each icon, at 34 px: 132 px in Gothic
        // (138 for all but New) and 154 px in Classic. A longer word widens all five buttons, up to
        // 182 px in Gothic (188 for all but New) and 204 px in Classic. Exit leaves Grimoire.
        {ICON_EXIT, _("Exit"), H_EXIT, true},
        // TRANSLATORS: the Library's top bar (see Exit).
        {ICON_GEAR, _("Settings"), H_SETTINGS, true},
        // TRANSLATORS: the Library's top bar (see Exit): to the Share screen (over Wi-Fi).
        {ICON_SHARE, _("Share"), H_SHARE, true},
        // TRANSLATORS: the Library's top bar (see Exit): makes a new folder.
        {ICON_FOLDER_PLUS, _("Folder"), H_NEWFOLDER, !S->cur_folder[0]},
        // TRANSLATORS: the Library's top bar (see Exit): makes a new notebook.
        {ICON_PLUS, _("New"), H_NEW, true},
    };
    // Started from KOReader's Grimoire plugin (kindle/app/grimoire.sh --hosted koreader), Exit goes
    // back to KOReader.
    const char *host = getenv("GRIMOIRE_HOST");
    bool hosted = host && !strcmp(host, "koreader");
    // TRANSLATORS: the Library's top bar when Grimoire was opened from KOReader: in place of Exit,
    // it goes back to KOReader. This one button takes the width it needs.
    if (hosted) btns[0].label = _("Back to KOReader");
    enum { NBTNS = sizeof btns / sizeof btns[0] };
    // A word too long for its button (a translation) widens all five alike, as far as leaves the
    // app's name room in the bar (beyond that a word is cut, and the fit check hears of it).
    // Hosted, Exit's longer "Back to KOReader" takes only its own room.
    for (int i = hosted ? 1 : 0; i < NBTNS; i++) {
        int need = ui_button_need_w(bh, btns[i].icon, btns[i].label, i == 4 ? BTN_PRIMARY : BTN_PLAIN);
        if (btns[i].show) bw = clampi(need, bw, ui_px(220));
    }
    int rx = a->w - m;
    for (int i = 0; i < NBTNS; i++) {
        if (!btns[i].show) continue;
        int w = bw;
        if (i == 0 && hosted) {  // as wide as its label needs, never narrower than the rest
            int need = ui_button_need_w(bh, btns[0].icon, btns[0].label, BTN_PLAIN);
            if (need > w) w = need;
        }
        rx -= w;
        Rect r = {rx, by, w, bh};
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
    // (A folder's name, cut short if it must be; else the app's.)
    char label[STORE_NAME_LEN + 32];
    font_draw_left_name(c, FONT_TITLE, ui_title_size() * (ui_theme() == THEME_GOTHIC ? 1.8f : 1.25f),
                        (Rect){x, 0, rx - x - ui_px(20), th},
                        S->cur_folder[0] ? ui_name_label(S->cur_folder, label, sizeof label) : APP_NAME, 0);
    ui_rule(c, 0, a->w, th, ui_px(4), 0);
    ui_frieze(c, 0, a->w, th + ui_px(9), ui_px(28), 0);

    int cols, rows, cw, ch;
    grid_geom(a, &cols, &rows, &cw, &ch);
    int pp = cols * rows;
    if (empty_hint_shown(a)) {
        Rect er = {m, th + ui_px(300), a->w - 2 * m, ui_px(100)};
        // TRANSLATORS: the Library with nothing in it (1760 px at 64 px), and what to do (at 44 px).
        const char *empty = S->cur_folder[0] ? _("This folder is empty") : _("No notebooks yet");
        font_draw_centered(c, FONT_BOLD, ui_title_size(), er, empty, 0);
        er.y += ui_px(110);
        // TRANSLATORS: New is the top bar's button (as translated there).
        font_draw_centered(c, FONT_REGULAR, ui_text_size(), er, _("Tap New to start one."), 60);
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
    char pl[96];
    // TRANSLATORS: between the Library's page buttons (1320 px at 44 px).
    snprintf(pl, sizeof pl, _("Page %d of %d"), S->page + 1, pages);
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
    // TRANSLATORS: the title of the sheet that makes a notebook, and its heading over the papers.
    Rect in = ui_sheet(c, card, _("New notebook"));
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){in.x, in.y, in.w, ui_px(70)}, _("Paper"), 0);
    // The tiles over the name rows and the buttons (420 px), under the heading (80): upright in 6
    // columns, sideways in 7 (tplsheet.h).
    int gap = ui_px(30);
    PaperGrid g = paper_grid(PAPER_SHEET_TILES, in.w, in.h - ui_px(80) - ui_px(420), gap, ui_px(100), ui_px(300),
                             PAPER_GRID_MIN_COLS);
    int gridw = g.cols * g.tw + (g.cols - 1) * gap;
    int ty = in.y + ui_px(80);
    bool tpl = S->new_tpl[0] != 0, new_custom = !tpl && paper_is_custom(S->new_paper);
    for (int i = 0; i < PAPER_SHEET_TILES; i++) {
        // The last two tiles. Custom music: the chosen layout, or the last one built. More papers:
        // the template chosen on the Templates sheet it opens (S->new_paper is then what the
        // notebook is made on if the template cannot be used).
        bool custom = i == PAPER_COUNT, more = i == PAPER_COUNT + 1;
        PaperKind k = more ? PAPER_BLANK : custom ? (new_custom ? S->new_paper : a->settings.music_layout) : paper_at(i);
        Rect tr = {in.x + (in.w - gridw) / 2 + (i % g.cols) * (g.tw + gap), ty + (i / g.cols) * g.cell_h, g.tw, g.th};
        canvas_fill(c, tr, 255);
        const Canvas *pv = more ? (tpl && S->tpl_prev.px ? &S->tpl_prev : NULL) : paper_preview(a, k);
        if (pv) canvas_blit_fit(c, pv, tr);
        else if (more) paper_tile_more(c, tr);
        bool sel = more ? tpl : custom ? new_custom : !tpl && S->new_paper == k;
        ui_frame(c, rect_inflate(tr, sel ? ui_px(10) : 0), ui_px(12), 0, false, sel ? ui_px(9) : ui_px(3));
        const char *label = paper_label(k);
        // TRANSLATORS: the last paper's tile but one, here and in the editor's paper sheet: it opens
        // a sheet that builds a music paper from parts (192 px at 31 px in New notebook sideways, on
        // one line or two).
        if (custom) label = _("Custom music...");
        // TRANSLATORS: the last paper's tile, here and in the editor's paper sheet: it opens the
        // Templates sheet, of pictures copied into grimoire/templates to write on as paper (192 px
        // at 31 px in New notebook sideways, on one line or two).
        if (more) label = _("More papers...");
        // (A label wider than its tile goes on two lines under it, as most do sideways: labels side
        // by side never run into each other.)
        paper_tile_label(c, tr, gap, label, sel);
        hits_add(&S->hits, (Rect){tr.x, tr.y, tr.w, g.th + ui_px(90)}, more ? H_PAPER_MORE : custom ? H_PAPER_CUSTOM : H_PAPER,
                 (int)k);
    }
    int y = ty + g.rows * g.cell_h + ui_px(20);
    char line[320];
    // TRANSLATORS: the new notebook's name, then its Rename button (268 px at 29 px, under its
    // icon, inside Gothic's carved line), then the folder it goes in, or the Library itself.
    snprintf(line, sizeof line, _("Name: %s"), S->new_name);
    Rect rn = {in.x + in.w - ui_px(300), y, ui_px(300), ui_px(120)};
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){in.x, y, in.w - ui_px(330), ui_px(120)}, line, 0);
    ui_button(c, rn, ICON_RENAME, _("Rename"), BTN_PLAIN);
    hits_add(&S->hits, rn, H_NEW_RENAME, 0);
    y += ui_px(140);
    snprintf(line, sizeof line, _("In: %s"), S->cur_folder[0] ? S->cur_folder : _("Library (top level)"));
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)}, line, 60);
    int bh = ui_px(140), bw = ui_px(380);
    Rect cancel = {in.x, in.y + in.h - bh, bw, bh};
    Rect create = {in.x + in.w - bw, in.y + in.h - bh, bw, bh};
    // TRANSLATORS: the sheets' buttons: 348 px at 44 px inside Gothic's carved line (342 for Create,
    // the main one), or at 34 px, or two lines of that.
    ui_button(c, cancel, ICON_NONE, _("Cancel"), BTN_PLAIN);
    ui_button(c, create, ICON_NONE, _("Create"), BTN_PRIMARY);
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

// The language Settings > Language has picked: its index, or -1 for automatic (language=auto,
// or a language this version does not speak).
static int language_picked(App *a) {
    return i18n_setting_known(a->settings.language) ? i18n_find(a->settings.language) : -1;
}

// "Automatic (English)": the choice that follows the Kindle, with what it stands for here.
static void language_auto_label(char *out, size_t cap) {
    // TRANSLATORS: in Settings > Language, the choice that follows the Kindle's own language, with
    // that language's name (each language is named in itself: "English", "Español").
    snprintf(out, cap, _("Automatic (%s)"), i18n_language_name(i18n_find(i18n_auto())));
}

static void draw_settings(App *a) {
    Canvas *c = &a->screen;
    Rect card = sheet_rect(a, ui_px(1820));
    Rect in = ui_sheet(c, card, _("Settings"));
    int rh = ui_px(140), gap = ui_px(24), y = in.y;
    Rect r1 = {in.x, y, in.w, rh};
    ui_button(c, r1, ICON_NONE, "", BTN_PLAIN);
    ui_icon(c, ICON_TUNE, (Rect){r1.x + ui_px(20), r1.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, 0);
    // TRANSLATORS: the rows of Settings: buttons with an icon (1470 px at 44 px in bold), then
    // lines a tap changes (1530 px at 44 px) and checkboxes (1548 px at 44 px, or two lines at
    // 34 px). Sideways they are 80 px wider.
    const char *pens = _("Pen sensitivity (pressure + tilt per tool)");
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r1.x + rh + ui_px(10), r1.y, r1.w - rh - ui_px(30), rh}, pens, 0);
    hits_add(&S->hits, r1, H_SET_PENS, 0);
    y += rh + gap;
    Rect r2 = {in.x, y, in.w, rh};
    ui_button(c, r2, ICON_NONE, "", BTN_PLAIN);
    ui_icon(c, ICON_TARGET, (Rect){r2.x + ui_px(20), r2.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, 0);
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r2.x + rh + ui_px(10), r2.y, r2.w - rh - ui_px(30), rh},
                   _("Calibrate pen and touch"), 0);
    hits_add(&S->hits, r2, H_SET_CALIB, 0);
    y += rh + gap;
    Rect r2b = {in.x, y, in.w, rh};
    ui_button(c, r2b, ICON_NONE, "", BTN_PLAIN);
    ui_icon(c, ICON_BRIGHTNESS, (Rect){r2b.x + ui_px(20), r2b.y + ui_px(15), rh - ui_px(30), rh - ui_px(30)}, 0);
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){r2b.x + rh + ui_px(10), r2b.y, r2b.w - rh - ui_px(30), rh},
                   _("Display: brightness, warmth, invert"), 0);
    hits_add(&S->hits, r2b, H_SET_DISPLAY, 0);
    y += rh + gap;
    Rect rt = {in.x, y, in.w, ui_px(110)};
    canvas_fill(c, rt, 255);
    ui_icon(c, ICON_CASTLE, (Rect){rt.x, rt.y + ui_px(15), ui_px(80), ui_px(80)}, 0);
    // TRANSLATORS: the style Grimoire is drawn in, and the other one a tap changes to.
    const char *style = _("Style: Gothic (tap for Classic)");
    if (ui_theme() == THEME_CLASSIC) style = _("Style: Classic (tap for Gothic)");
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){rt.x + ui_px(110), rt.y, rt.w - ui_px(110), rt.h}, style, 0);
    hits_add(&S->hits, rt, H_SET_THEME, 0);
    y += ui_px(130);
    Rect r3 = {in.x, y, in.w, ui_px(110)};
    char row[256];
    // TRANSLATORS: %s is the folder on the Kindle's storage (seen over USB) that gets a PDF of
    // every notebook.
    snprintf(row, sizeof row, _("Mirror PDFs to %s (USB)"), "grimoire/PDFs");
    ui_checkbox(c, r3, row, a->settings.mirror_pdf);
    hits_add(&S->hits, r3, H_SET_MIRROR, 0);
    y += ui_px(130);
    Rect r4 = {in.x, y, in.w, ui_px(110)};
    // What holding the pen's side button does while writing (settings.ini side_button 0 to 3).
    static const char *const btn_rows[4] = {
        // TRANSLATORS: what holding the pen's button while writing does; a tap moves to the next.
        N_("Holding the pen button does nothing (tap to change)"),
        N_("Holding the pen button erases (tap to change)"),
        N_("Holding the pen button highlights (tap to change)"),
        // TRANSLATORS: the fourth choice of the same row: while the pen's button is held, the pen
        // draws white ink (which takes away the ink under it and keeps the paper's lines). 1530 px
        // at 44 px, sideways 1610, or two lines at 34 px.
        N_("Holding the pen button draws white (tap to change)"),
    };
    canvas_fill(c, r4, 255);
    ui_icon(c, ICON_HIGHLIGHTER, (Rect){r4.x, r4.y + ui_px(15), ui_px(80), ui_px(80)}, 0);
    ui_text(c, FONT_REGULAR, ui_text_size(), ui_small_size(), (Rect){r4.x + ui_px(110), r4.y, r4.w - ui_px(110), r4.h},
            _(btn_rows[clampi(a->settings.side_button, 0, 3)]), 2, false, 0);
    hits_add(&S->hits, r4, H_SET_SIDE, 0);
    y += ui_px(130);
    Rect r5 = {in.x, y, in.w, ui_px(110)};
    ui_checkbox(c, r5, _("Turn with the Kindle (portrait and landscape)"), a->settings.follow_rotation);
    hits_add(&S->hits, r5, H_SET_ROTATE, 0);
    y += ui_px(130);
    Rect r6 = {in.x, y, in.w, ui_px(110)};
    ui_checkbox(c, r6, _("Pinch with two fingers to zoom"), a->settings.pinch_zoom);
    hits_add(&S->hits, r6, H_SET_PINCH, 0);
    y += ui_px(130);
    // The last row the sheet has room for: sideways in Gothic 18 px are left above Done.
    Rect rl = {in.x, y, in.w, ui_px(110)};
    char lang[96], lang_line[160];
    if (language_picked(a) < 0) language_auto_label(lang, sizeof lang);
    else snprintf(lang, sizeof lang, "%s", i18n_language_name(language_picked(a)));
    // TRANSLATORS: a row of Settings, with the choice: "Automatic (English)", or a language's name.
    snprintf(lang_line, sizeof lang_line, _("Language: %s"), lang);
    canvas_fill(c, rl, 255);
    ui_icon(c, ICON_LANGUAGE, (Rect){rl.x, rl.y + ui_px(15), ui_px(80), ui_px(80)}, 0);
    ui_text(c, FONT_REGULAR, ui_text_size(), ui_small_size(), (Rect){rl.x + ui_px(110), rl.y, rl.w - ui_px(110), rl.h},
            lang_line, 2, false, 0);
    hits_add(&S->hits, rl, H_SET_LANGUAGE, 0);
    y += ui_px(150);
    const InputCaps *caps = input_caps();
    char pen[160], about[320];
    bool pressure = caps && caps->has_pressure, tilt = caps && caps->has_tilt;
    // TRANSLATORS: Settings' last lines (34 px): after the version, the pen Grimoire found (its
    // device's name, %s) and what it reports; then where Grimoire keeps its data (%s).
    if (!caps || !caps->has_pen) snprintf(pen, sizeof pen, "%s", _("pen: none"));
    else if (pressure && tilt) snprintf(pen, sizeof pen, _("pen: %s, pressure, tilt"), caps->pen_name);
    else if (pressure) snprintf(pen, sizeof pen, _("pen: %s, pressure"), caps->pen_name);
    else if (tilt) snprintf(pen, sizeof pen, _("pen: %s, tilt"), caps->pen_name);
    else snprintf(pen, sizeof pen, _("pen: %s"), caps->pen_name);
    snprintf(about, sizeof about, "Grimoire %s  %s", GRIMOIRE_VERSION, pen);
    font_draw_left(c, FONT_REGULAR, ui_small_size(), (Rect){in.x, y, in.w, ui_px(60)}, about, 60);
    // (The data's path is a name: a long one is cut short, as names are. tests/test_i18n.c checks
    // that the label fits with the Kindle's path.)
    snprintf(about, sizeof about, _("Data: %s"), store_root());
    font_draw_left_name(c, FONT_REGULAR, ui_small_size(), (Rect){in.x, y + ui_px(60), in.w, ui_px(60)}, about, 60);
    Rect done = {in.x + in.w - ui_px(380), in.y + in.h - ui_px(140), ui_px(380), ui_px(140)};
    ui_button(c, done, ICON_NONE, _("Done"), BTN_PRIMARY);
    hits_add(&S->hits, done, H_CANCEL, 0);
    Rect ko = {in.x, done.y, ui_px(650), done.h};
    ui_button(c, ko, ICON_NOTEBOOK, "KOReader", BTN_PLAIN);
    hits_add(&S->hits, ko, H_KOREADER, 0);
}

static void draw_modal(App *a) {
    Item *it = (S->target >= 0 && S->target < S->nitems) ? &S->items[S->target] : NULL;
    char label[STORE_NAME_LEN + 32];  // (a name as the screens show it: ui_name_label)
    switch (S->modal) {
    case MOD_NEW: draw_new(a); break;
    case MOD_SETTINGS: draw_settings(a); break;
    case MOD_KOREADER: {
        char ko[800];
        bool found = koreader_find(ko, sizeof ko), installed = koreader_item_installed(0);
        MenuRow rows[] = {
            {ICON_NOTEBOOK, found ? _("Install / update Grimoire plugin") : _("KOReader not found"), H_KO_ITEM, 0,
                found ? BTN_PLAIN : BTN_DISABLED},
            {ICON_TRASH, _("Remove Grimoire plugin"), H_KO_ITEM, -1, installed ? BTN_PLAIN : BTN_DISABLED},
            {koreader_item_installed(1) ? ICON_CHECK : ICON_NONE, _("Disable diagonal screenshot gestures"), H_KO_ITEM, 1,
                found ? BTN_PLAIN : BTN_DISABLED},
            {koreader_item_installed(2) ? ICON_CHECK : ICON_NONE, _("Square folder corners"), H_KO_ITEM, 2,
                found ? BTN_PLAIN : BTN_DISABLED},
            {koreader_item_installed(3) ? ICON_CHECK : ICON_NONE, _("Terminess menu font"), H_KO_ITEM, 3,
                found ? BTN_PLAIN : BTN_DISABLED},
            {ICON_CLOSE, _("Done"), H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, "KOReader", rows, 6);
        break;
    }
    case MOD_MORE: {
        if (!it) break;
        char pdf[256];
        // TRANSLATORS: a notebook's menu, rows of 1470 px at 44 px in bold. %s is the folder on the
        // Kindle's storage the PDF goes in.
        snprintf(pdf, sizeof pdf, _("Save PDF to %s now"), "grimoire/PDFs");
        MenuRow rows[] = {
            {ICON_NOTEBOOK, _("Open"), H_MORE_OPEN, 0, BTN_PLAIN},
            {ICON_RENAME, _("Rename"), H_MORE_RENAME, 0, BTN_PLAIN},
            {ICON_FOLDER_MOVE, _("Move to folder"), H_MORE_MOVE, 0, BTN_PLAIN},
            {ICON_DOWNLOAD, pdf, H_MORE_PDF, 0, BTN_PLAIN},
            {ICON_TRASH, _("Delete"), H_MORE_DELETE, 0, BTN_PLAIN},
            {ICON_CLOSE, _("Cancel"), H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, ui_name_label(S->books[it->nb].name, label, sizeof label), rows, 6);
        break;
    }
    case MOD_FOLDER_MORE: {
        if (!it) break;
        // TRANSLATORS: a folder's menu (1470 px at 44 px in bold); only an empty folder can go.
        MenuRow rows[] = {
            {ICON_RENAME, _("Rename folder"), H_FMORE_RENAME, 0, BTN_PLAIN},
            {ICON_TRASH, it->count ? _("Remove folder (move its notebooks out first)") : _("Remove folder"), H_FMORE_REMOVE, 0,
             it->count ? BTN_DISABLED : BTN_PLAIN},
            {ICON_CLOSE, _("Cancel"), H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, ui_name_label(it->folder, label, sizeof label), rows, 3);
        break;
    }
    case MOD_MOVE: {
        if (!it) break;
        MenuRow rows[14];
        int n = 0;
        const char *curf = S->books[it->nb].folder;
        // TRANSLATORS: where a notebook can go: the Library itself, out of every folder (also in New
        // notebook: "In: Library (top level)").
        rows[n++] = (MenuRow){ICON_LIBRARY, _("Library (top level)"), H_MOVE_TOP, 0, curf[0] ? BTN_PLAIN : BTN_SELECTED};
        for (int f = 0; f < S->nfolders && n < 12; f++)
            rows[n++] = (MenuRow){ICON_FOLDER, S->folders[f], H_MOVE_TO, f, strcmp(curf, S->folders[f]) ? BTN_PLAIN : BTN_SELECTED};
        rows[n++] = (MenuRow){ICON_FOLDER_PLUS, _("New folder..."), H_MOVE_NEW, 0, BTN_PLAIN};
        rows[n++] = (MenuRow){ICON_CLOSE, _("Cancel"), H_CANCEL, 0, BTN_PLAIN};
        // TRANSLATORS: the title of the list of folders a notebook can be moved to.
        draw_menu(a, _("Move to"), rows, n);
        break;
    }
    case MOD_DELETE: {
        if (!it) break;
        char title[256], del[256];
        // TRANSLATORS: the title of the question before a notebook is deleted, with its name.
        snprintf(title, sizeof title, _("Delete \"%s\"?"), ui_name_label(S->books[it->nb].name, label, sizeof label));
        // TRANSLATORS: the answers (1470 px at 44 px in bold): delete, into the trash folder on the
        // Kindle's storage (%s), or keep the notebook.
        snprintf(del, sizeof del, _("Delete (moves it to %s)"), "grimoire/trash");
        MenuRow rows[] = {
            {ICON_TRASH, del, H_DELETE_YES, 0, BTN_PRIMARY},
            {ICON_CLOSE, _("Keep it"), H_CANCEL, 0, BTN_PLAIN},
        };
        draw_menu(a, title, rows, 2);
        break;
    }
    case MOD_LANGUAGE: {
        // Automatic, then every language by its own name; a pick applies at once.
        MenuRow rows[12];
        int n = 0, cur = language_picked(a);
        char auto_label[96];
        language_auto_label(auto_label, sizeof auto_label);
        rows[n++] = (MenuRow){ICON_LANGUAGE, auto_label, H_LANG_PICK, -1, cur < 0 ? BTN_SELECTED : BTN_PLAIN};
        for (int i = 0; i < i18n_language_count() && n < 11; i++)
            rows[n++] = (MenuRow){ICON_NONE, i18n_language_name(i), H_LANG_PICK, i, cur == i ? BTN_SELECTED : BTN_PLAIN};
        rows[n++] = (MenuRow){ICON_CLOSE, _("Cancel"), H_LANG_BACK, 0, BTN_PLAIN};
        // TRANSLATORS: the title of the list of languages (Settings > Language).
        draw_menu(a, _("Language"), rows, n);
        break;
    }
    case MOD_KB: keyboard_draw(&S->kb, &a->screen, (Rect){0, 0, a->w, a->h}); break;
    case MOD_DISPLAY: dispmenu_draw(&S->dm, a); break;
    case MOD_MUSIC: draw_new(a); musicbuild_draw(&S->mb, a); break;
    case MOD_TEMPLATES: draw_new(a); tplsheet_draw(&S->ts, a); break;
    default: break;
    }
}

// Where toasts must stay above while a sheet is open, so they never cover its controls: the
// keyboard's top; else, when a toast in its usual place would cover any of the sheet's controls
// (the buttons along a tall sheet's bottom), the top of a band of the toast's height clear of
// them, found going up from there. 0: the usual place is clear.
static int toast_floor(App *a) {
    if (S->modal == MOD_NONE) return 0;
    if (S->modal == MOD_KB) return S->kb.area.y;
    // (The Custom music and Templates sheets lie over the New notebook sheet: only their own
    // controls count.)
    const HitList *l = S->modal == MOD_MUSIC ? &S->mb.hits : S->modal == MOD_TEMPLATES ? &S->ts.hits : &S->hits;
    // (A toast of two lines is taller: the band is as tall as the toast up now, else one line.)
    Rect t = ui_toast_rect((Rect){0, 0, a->w, a->h}, a->toast_visible ? a->toast : "");
    int gap = ui_px(20), y = t.y, below = 0;  // below: the control the toast must stay above
    for (bool moved = true; moved && y >= gap;) {
        moved = false;
        for (int i = 0; i < l->n; i++) {
            Rect r = l->h[i].r;
            if (r.y < y + t.h && r.y + r.h > y) {
                y = r.y - t.h - gap;  // above this control, then look again from there
                below = r.y;
                moved = true;
            }
        }
    }
    return below;
}

static void redraw(App *a, RefreshMode m) {
    draw_base(a);
    if (S->modal != MOD_NONE) {
        hits_clear(&S->hits);  // a modal captures all input
        draw_modal(a);
    }
    a->toast_floor = toast_floor(a);
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

// A notebook on a template (More papers...): templates.h. False if that could not be done: the
// notebook stays on the paper picked before.
static bool start_on_template(NotebookInfo *nb, const char *path) {
    const char *why = "";
    if (template_start_notebook(nb, path, &why) == 0) return true;
    qlog("library: %s: the new notebook's template could not be used: %s", nb->id, why);
    return false;
}

static void create_notebook(App *a) {
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb) return;
    if (store_create(S->new_name, S->cur_folder, S->new_paper, nb) == 0) {
        S->modal = MOD_NONE;
        bool tpl_ok = !S->new_tpl[0] || start_on_template(nb, S->new_tpl);
        char id[STORE_ID_LEN];
        strlcpy_q(id, nb->id, sizeof id);
        free(nb);
        editor_open_notebook(a, id);
        if (!tpl_ok) {
            char msg[256];
            snprintf(msg, sizeof msg, _("Could not use that template (see %s)."), "grimoire.log");
            app_toast(a, msg, 4000);
        }
        return;
    }
    free(nb);
    app_toast(a, _("Could not create the notebook (storage full?)"), 3000);
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
        char base[STORE_NAME_LEN], day[64];
        format_date_short(wall_time(), day, sizeof day);
        // TRANSLATORS: a new notebook's name until it is renamed, with the day ("Notebook Oct 10").
        snprintf(base, sizeof base, _("Notebook %s"), day);
        unique_name(a, base, S->new_name, sizeof S->new_name);
        S->new_paper = PAPER_MEDIUM;
        S->new_tpl[0] = 0;
        canvas_free(&S->tpl_prev);
        open_modal(a, MOD_NEW);
        break;
    }
    // TRANSLATORS: the keyboard's label for the name being typed: a folder's, or a notebook's.
    case H_NEWFOLDER: open_kb(a, KBP_NEW_FOLDER, C_("keyboard", "Folder"), ""); break;
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
    case H_PAPER:
        S->new_paper = (PaperKind)h->arg;
        S->new_tpl[0] = 0;
        redraw(a, REFRESH_UI);
        break;
    case H_PAPER_CUSTOM:
        musicbuild_open(&S->mb, a, S->new_paper);
        open_modal(a, MOD_MUSIC);
        break;
    case H_PAPER_MORE:
        tplsheet_open(&S->ts, a, NULL, PAPER_BLANK, S->new_tpl);
        open_modal(a, MOD_TEMPLATES);
        break;
    case H_NEW_RENAME: open_kb(a, KBP_NEW_NAME, C_("keyboard", "Name"), S->new_name); break;
    case H_NEW_CREATE: create_notebook(a); break;
    case H_CANCEL: close_modal(a); break;
    case H_KOREADER: open_modal(a, MOD_KOREADER); break;
    case H_KO_ITEM: {
        int item = h->arg < 0 ? 0 : h->arg;
        bool enabled = h->arg < 0 ? false : item == 0 || !koreader_item_installed(item);
        if (koreader_item_set(item, enabled) == 0)
            app_toast(a, _("Updated. Restart KOReader to apply."), 5000);
        else
            app_toast(a, _("Could not update KOReader. A different patch or a storage error?"), 7000);
        redraw(a, REFRESH_UI);
        break;
    }
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
        a->settings.side_button = (clampi(a->settings.side_button, 0, 3) + 1) % 4;
        settings_save(&a->settings);
        redraw(a, REFRESH_UI);
        break;
    case H_SET_LANGUAGE: open_modal(a, MOD_LANGUAGE); break;
    case H_LANG_PICK: {
        // Saved and applied at once: the Settings sheet comes back in the language picked.
        const char *code = h->arg >= 0 ? i18n_language_code(h->arg) : "auto";
        if (code && strcmp(code, a->settings.language)) {
            strlcpy_q(a->settings.language, code, sizeof a->settings.language);
            settings_save(&a->settings);
            i18n_apply(code);
        }
        S->modal = MOD_SETTINGS;
        redraw(a, REFRESH_FULL);
        break;
    }
    case H_LANG_BACK:
        S->modal = MOD_SETTINGS;
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
        if (it) open_kb(a, KBP_RENAME, C_("keyboard", "Name"), S->books[it->nb].name);
        break;
    case H_MORE_MOVE: open_modal(a, MOD_MOVE); break;
    case H_MORE_PDF:
        if (it) {
            bg_mirror_pdf(S->books[it->nb].id, a->dpi, app_mirror_dir(), true);
            S->modal = MOD_NONE;
            redraw(a, REFRESH_UI);
            app_toast(a, _("Saving the PDF in the background..."), 2500);
        }
        break;
    case H_MORE_DELETE: open_modal(a, MOD_DELETE); break;
    case H_DELETE_YES: {
        if (it && !it->is_folder) {
            char nb_path[1200];
            export_mirror_path(&S->books[it->nb], app_mirror_dir(), nb_path, sizeof nb_path);
            unlink(nb_path);  // the notebook goes to trash; its mirror PDF should not linger
            // A compaction of it, queued or running, stops first: it would write into the folder
            // in the trash (or leave a temporary file there).
            bg_cancel_notebook(S->books[it->nb].id);
            store_delete(S->books[it->nb].id);
        }
        S->modal = MOD_NONE;
        reload(a);
        redraw(a, REFRESH_FULL);
        char msg[256];
        // TRANSLATORS: a deleted notebook went to the trash folder on the Kindle's storage (%s).
        snprintf(msg, sizeof msg, _("Moved to %s"), "grimoire/trash");
        app_toast(a, msg, 2500);
        break;
    }
    case H_FMORE_RENAME:
        if (it) open_kb(a, KBP_RENAME_FOLDER, C_("keyboard", "Folder"), it->folder);
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
    case H_MOVE_NEW: open_kb(a, KBP_MOVE_NEW_FOLDER, C_("keyboard", "Folder"), ""); break;
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
    tplsheet_close(&S->ts);
    canvas_free(&S->tpl_prev);
    free(S);
    a->lib = NULL;
}

void library_enter(App *a) {
    // A turn, or the redraw after waking, re-lays out the same screen: the open sheet stays open
    // (every sheet lays itself out when drawn).
    if (!a->relayout) S->modal = MOD_NONE;
    S->pressed = NULL;
    reload(a);
    redraw(a, REFRESH_FULL);
}

void library_leave(App *a) { S->pressed = NULL; }

void library_event(App *a, const InEvent *ev) {
    int ph = app_pointer_phase(ev);
    if (ph < 0) return;
    if (S->modal == MOD_TEMPLATES) {
        Rect d;
        TsResult r = tplsheet_pointer(&S->ts, a, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == TS_PICKED) {
            // The notebook is made on it (create_notebook); its tile shows its preview meanwhile.
            const char *path = tplsheet_path(&S->ts);
            const Canvas *pv = tplsheet_preview(&S->ts);
            strlcpy_q(S->new_tpl, path ? path : "", sizeof S->new_tpl);
            canvas_free(&S->tpl_prev);
            if (pv && canvas_init(&S->tpl_prev, pv->w, pv->h) == 0)
                canvas_blit(&S->tpl_prev, pv, (Rect){0, 0, pv->w, pv->h}, 0, 0);
            tplsheet_close(&S->ts);
        }
        if (r == TS_PICKED || r == TS_BACK || r == TS_CANCEL) {
            S->modal = MOD_NEW;  // back to New notebook, also from a tap outside the sheet
            redraw(a, REFRESH_UI);
        } else if (!rect_is_empty(d)) {
            app_invalidate(a, d, REFRESH_UI);
        }
        return;
    }
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
            if (r == MB_DONE) {
                S->new_paper = musicbuild_result(&S->mb);
                S->new_tpl[0] = 0;  // (a layout built takes the place of a template picked before)
            }
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
    Rect d;
    if (S->modal == MOD_TEMPLATES && tplsheet_tick(&S->ts, a, now, &d)) app_invalidate(a, d, REFRESH_UI);
    // A background job ended (bgimage.h): the cards it may have changed are read again. A
    // template's picture made for a notebook since may be on its first page, whose card the job drew
    // again (bgwork_template.c: its thumb.pgm is another file now), or a card drawn from a
    // notebook's own picture (picture_card) drew it white meanwhile. Any other job (the templates'
    // previews) changes no card, and the Library is not drawn again for it.
    if (S->card_gen != bgimage_generation()) {
        S->card_gen = bgimage_generation();
        bool changed = false;
        for (int i = 0; i < S->nbooks && S->card_stamp; i++) {
            if (!S->thumb_tried[i] || S->card_stamp[i] == store_thumb_stamp(S->books[i].id)) continue;
            canvas_free(&S->thumbs[i]);
            S->thumb_tried[i] = false;
            changed = true;
        }
        if (changed && S->modal == MOD_NONE) {
            S->cards_redrawn++;
            redraw(a, REFRESH_UI);
        }
    }
}

// ---- headless checks ----

// card.N (from 1, as library.items counts them): what card N shows, "folder", "thumb" (its
// thumb.pgm), "picture" (its own picture paper, picture_card) or "paper K" (the preview of paper
// K); card.N.luma: the mean of the pixels it shows, 0 to 255 ("none" for a folder); card.N.name: the
// name under it, as drawn (ui_name_label).
static int card_probe(App *a, const char *key, char *out, size_t cap) {
    char *end;
    long n = strtol(key, &end, 10);
    bool luma = !strcmp(end, ".luma"), name = !strcmp(end, ".name");
    if (n < 1 || n > S->nitems || (*end && !luma && !name)) return -1;
    const Item *it = &S->items[n - 1];
    if (name) {  // the name under the card, as drawn (ui_name_label)
        char label[STORE_NAME_LEN + 32];
        snprintf(out, cap, "%s", ui_name_label(it->is_folder ? it->folder : S->books[it->nb].name, label, sizeof label));
        return 0;
    }
    if (it->is_folder) {
        snprintf(out, cap, "%s", luma ? "none" : "folder");
        return 0;
    }
    const Canvas *t = thumb_for(a, it->nb);
    PaperKind k = S->books[it->nb].paper;
    if (luma) {
        uint64_t sum = 0;
        for (int y = 0; t && y < t->h; y++)
            for (int x = 0; x < t->w; x++) sum += t->px[(size_t)y * (size_t)t->stride + (size_t)x];
        uint64_t np = t ? (uint64_t)t->w * (uint64_t)t->h : 0;
        snprintf(out, cap, "%d", np ? (int)((sum + np / 2) / np) : 255);
    } else if (S->thumbs[it->nb].px) {
        snprintf(out, cap, "%s", S->thumb_picture[it->nb] ? "picture" : "thumb");
    } else {  // (as paper_preview draws it)
        snprintf(out, cap, "paper %u", (unsigned)(!paper_is_known(k) || paper_is_image(k) ? PAPER_BLANK : k));
    }
    return 0;
}

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
        [MOD_LANGUAGE] = "language",
        [MOD_TEMPLATES] = "templates",
        [MOD_KOREADER] = "koreader",
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
    } else if (!strcmp(key, "hint")) {
        snprintf(out, cap, "%d", empty_hint_shown(a) ? 1 : 0);
    } else if (!strcmp(key, "cards_redrawn")) {
        snprintf(out, cap, "%d", S->cards_redrawn);  // after background jobs that changed a card
    } else if (!strcmp(key, "paper")) {
        if (S->new_tpl[0]) snprintf(out, cap, "template");
        else if (paper_is_custom(S->new_paper)) snprintf(out, cap, "custom");
        else snprintf(out, cap, "%d", (int)S->new_paper);
    } else if (!strcmp(key, "template")) {
        // The template New notebook makes its notebook on: its file's name without its extension.
        const char *slash = strrchr(S->new_tpl, '/'), *name = slash ? slash + 1 : S->new_tpl, *dot = strrchr(name, '.');
        int len = dot ? (int)(dot - name) : (int)strlen(name);
        snprintf(out, cap, "%.*s", len, name);
    } else if (!strncmp(key, "templates.", 10)) {
        return tplsheet_probe(S->modal == MOD_TEMPLATES ? &S->ts : NULL, key + 10, out, cap);
    } else if (!strcmp(key, "music")) {
        // The layout on the Custom music sheet while it is open, else New notebook's custom paper.
        MusicLayout m;
        if (S->modal == MOD_MUSIC) app_probe_music(&S->mb.m, out, cap);
        else app_probe_music(music_decode(S->new_paper, &m) ? &m : NULL, out, cap);
    } else if (!strncmp(key, "card.", 5)) {
        return card_probe(a, key + 5, out, cap);
    } else {
        return -1;
    }
    return 0;
}
