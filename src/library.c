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
#include "update.h"

typedef enum { MOD_NONE, MOD_NEW, MOD_SETTINGS, MOD_MORE, MOD_FOLDER_MORE, MOD_MOVE, MOD_DELETE, MOD_KB, MOD_DISPLAY, MOD_MUSIC, MOD_LANGUAGE,
               MOD_TEMPLATES, MOD_KOREADER, MOD_SHORTCUTS, MOD_BUTTONS, MOD_PAIR, MOD_UPDATES } Modal;
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
    H_PAPER_MORE, H_KOREADER, H_KO_ITEM, H_SHORTCUTS, H_SHORTCUT_ENABLE, H_SHORTCUT_RELOAD,
    H_BUTTONS, H_BUTTON_GROUP, H_BUTTON_ROW, H_BUTTONS_RESET, H_BUTTONS_DONE,
    H_BT_TOGGLE, H_PAIR, H_PAIR_FOUND, H_PAIR_FORGET, H_PAIR_AGAIN,
    H_UPDATES, H_UPD_CHANNEL, H_UPD_CHECK, H_UPD_NOW,
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

    int shown_shortcut_code, shown_shortcut_mods;
    bool shown_shortcut_gamepad;
    unsigned shown_hid_changes;  // App.hid_changes as the Controller sheet's status line (or Buttons' rows) last showed
    // The Buttons sheet: its group open (k_groups; -1 the list of them), the row learning its
    // buttons (-1 none), which of its actions, and since when (it gives up after LEARN_MS without a
    // press).
    int buttons_group, learn_row, learn_step;
    uint32_t learn_at;
    // The Pair sheet (Settings > Controller > Pair a controller): what it asked the Bluetooth helper
    // last (ASK_*), what it asks next and when, the controllers found and those paired.
    int bt_asked, bt_next;
    uint32_t bt_next_at, scan_until, pair_until;
    bool scanning, pairing;
    char pair_addr[BT_ADDR_LEN];
    BtDevice found[4], paired[2];
    int nfound, npaired;
    unsigned shown_bt_changes;
    // The Updates sheet: its check (UPD_*), the newest version it found, why it failed.
    int upd;
    char upd_latest[UPDATE_VERSION_LEN], upd_why[16];
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
    uint32_t pressed_at;  // when the finger went down on it (a Buttons row held ROW_HOLD_MS: forget_row)
};

#define S (a->lib)

// ---- layout ----
static int top_h(void) { return ui_px(190); }
static int bottom_h(void) { return ui_px(170); }
static int margin(void) { return ui_px(50); }
static int grid_top(void) {
    // The Gothic frieze hangs below the header rule; leave air under its drops.
    return top_h() + margin() + (ui_theme() == THEME_GOTHIC ? ui_px(24) : 0);
}

static void grid_geom(App *a, int *cols, int *rows, int *cw, int *ch) {
    *cols = a->w > a->h ? 5 : 3;  // more, smaller cards when the Kindle is sideways
    *cw = (a->w - margin() * (*cols + 1)) / *cols;
    int th = *cw * 4 / 3;
    *ch = th + ui_px(150);
    int avail = a->h - grid_top() - bottom_h();
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
        draw_card(a, (Rect){m + col * (cw + m), grid_top() + row * (ch + m), cw, ch}, idx);
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

// ---- the controller's buttons (Settings > Controller > Buttons) ----

#define LEARN_MS 10000
// A Buttons row held this long without moving loses its buttons (forget_row), as a link held in the
// editor opens (LINK_HOLD_MS).
#define ROW_HOLD_MS 600

static void redraw(App *a, RefreshMode m);

// A row learns one action's button, or two in turn (the previous page's, then the next page's).
// label NULL: the action's own name (action_label).
typedef struct {
    const char *label;
    int act[2];
    IconKind icon;
} LearnRow;

// The Buttons sheet's groups, as the editor's panel groups its buttons: each a sheet of rows.
static const LearnRow k_editing[] = {
    {NULL, {SHORTCUT_UNDO, 0}, ICON_UNDO},
    {NULL, {SHORTCUT_REDO, 0}, ICON_REDO},
    {NULL, {SHORTCUT_TOOL_BASE + TOOL_ERASER, 0}, ICON_ERASER},
    {NULL, {SHORTCUT_HOLD_ERASER, 0}, ICON_GAMEPAD},
    {NULL, {SHORTCUT_ERASER_MODE, 0}, ICON_NONE},
    {NULL, {SHORTCUT_WHITE, 0}, ICON_NONE},
    {NULL, {SHORTCUT_CLEAR_PAGE, 0}, ICON_BROOM},
};
// The Tools group: tool_order's pens first (learn_row_at), then this.
static const LearnRow k_tools[] = {
    // TRANSLATORS: (Buttons sheet) the previous and the next tool's buttons, in that order (as the
    // other rows, about 1100 px at 44 px in bold, with the buttons' names). They go through the pens
    // on the panel (the tool button's and the pinned ones): a pin beside it.
    {N_("Previous / next tool"), {SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL}, ICON_PIN},
    // TRANSLATORS: (Buttons sheet) the buttons that move the open tool picker's choice to its
    // previous and its next pen (with it closed, they switch tools through all nine), in that order.
    {N_("Previous / next in the picker"), {SHORTCUT_PICKER_PREV, SHORTCUT_PICKER_NEXT}, ICON_GRID},
};
static const LearnRow k_pages[] = {
    // TRANSLATORS: (Buttons sheet) the previous and the next page's buttons, in that order.
    {N_("Previous / next page"), {SHORTCUT_PREV_PAGE, SHORTCUT_NEXT_PAGE}, ICON_NEXT},
    // TRANSLATORS: (Buttons sheet) the first and the last page's buttons, in that order.
    {N_("First / last page"), {SHORTCUT_FIRST_PAGE, SHORTCUT_LAST_PAGE}, ICON_NONE},
    {NULL, {SHORTCUT_NEW_PAGE, 0}, ICON_PLUS},
    {NULL, {SHORTCUT_FIT_PAGE, 0}, ICON_EXPAND},
    {NULL, {SHORTCUT_PAGES, 0}, ICON_NOTEBOOK},
    {NULL, {SHORTCUT_INDEX, 0}, ICON_NONE},
};
static const LearnRow k_menus[] = {
    {NULL, {SHORTCUT_TOOL_PICKER, 0}, ICON_GRID},
    {NULL, {SHORTCUT_PAPER, 0}, ICON_PAPER},
    {NULL, {SHORTCUT_DISPLAY, 0}, ICON_BRIGHTNESS},
    {NULL, {SHORTCUT_PEN_FEEL, 0}, ICON_TUNE},
    {NULL, {SHORTCUT_FOLD_PANEL, 0}, ICON_COLLAPSE},
    {NULL, {SHORTCUT_LIBRARY, 0}, ICON_LIBRARY},
    {NULL, {SHORTCUT_CLOSE_MENU, 0}, ICON_CLOSE},
};
#define ROWS(r) r, (int)(sizeof r / sizeof r[0])
static const struct {
    const char *label, *title;  // its row in the list, the title of its sheet
    IconKind icon;
    const char *probe;          // library.buttons_group
    const LearnRow *rows;
    int n;
} k_groups[] = {
    // TRANSLATORS: the Buttons sheet's list (Settings > Controller > Buttons): its groups of actions,
    // each opening a sheet of them (about 1100 px at 44 px in bold), and those sheets' titles. Editing:
    // undo, redo, the eraser, white ink, clearing the page.
    {N_("Editing..."), N_("Editing"), ICON_UNDO, "editing", ROWS(k_editing)},
    // TRANSLATORS: (Buttons sheet) its group of the drawing tools, one by one.
    {N_("Tools..."), N_("Tools"), ICON_GRID, "tools", ROWS(k_tools)},
    // TRANSLATORS: (Buttons sheet) its group of page turning, new pages and the Pages view.
    {N_("Pages..."), N_("Pages"), ICON_NEXT, "pages", ROWS(k_pages)},
    // TRANSLATORS: (Buttons sheet) its group of the editor's menus: each button opens one (and closes
    // it again).
    {N_("Menus..."), N_("Menus"), ICON_MORE, "menus", ROWS(k_menus)},
};
#undef ROWS
enum { N_GROUPS = sizeof k_groups / sizeof k_groups[0] };

// How many rows group g has (the Tools group's pens too).
static int group_rows(int g) { return k_groups[g].n + (k_groups[g].rows == k_tools ? TOOL_PENS : 0); }

// Row i of group g: the Tools group's first rows are tool_order's pens, the picker's order.
static LearnRow learn_row_at(int g, int i) {
    if (k_groups[g].rows == k_tools && i < TOOL_PENS)
        return (LearnRow){NULL, {SHORTCUT_TOOL_BASE + (int)tool_order[i], 0}, ui_tool_icon(tool_order[i])};
    return k_groups[g].rows[k_groups[g].rows == k_tools ? i - TOOL_PENS : i];
}

// An action's name as the Buttons sheet shows it: its row's, and in what it asks while it learns.
static const char *action_label(int action) {
    if (action >= SHORTCUT_TOOL_BASE && action < SHORTCUT_TOOL_BASE + TOOL_COUNT)
        return tool_label((ToolKind)(action - SHORTCUT_TOOL_BASE));
    static const struct {
        int action;
        const char *label;
    } names[] = {
        // TRANSLATORS: the Buttons sheet's rows (Settings > Controller > Buttons), each followed by
        // the controller buttons that do it ("Undo: South"): about 1100 px at 44 px in bold, with the
        // names. The same names fill "Press a button for %s" and "That button is no longer for %s.".
        {SHORTCUT_UNDO, N_("Undo")},
        {SHORTCUT_REDO, N_("Redo")},
        // TRANSLATORS: (Buttons sheet) the pen erases while the controller's button is held down.
        {SHORTCUT_HOLD_ERASER, N_("Erase while held")},
        // TRANSLATORS: (Buttons sheet) the eraser takes whole strokes, or only what it touches, in turn.
        {SHORTCUT_ERASER_MODE, N_("Eraser mode")},
        {SHORTCUT_WHITE, N_("White ink")},
        // TRANSLATORS: (Buttons sheet) takes every stroke off the page, as the panel's broom.
        {SHORTCUT_CLEAR_PAGE, N_("Clear page")},
        {SHORTCUT_PREV_TOOL, N_("Previous tool")},
        {SHORTCUT_NEXT_TOOL, N_("Next tool")},
        // TRANSLATORS: (Buttons sheet) the tool picker's previous and next pen ("Press a button for
        // Next in the picker").
        {SHORTCUT_PICKER_PREV, N_("Previous in the picker")},
        {SHORTCUT_PICKER_NEXT, N_("Next in the picker")},
        {SHORTCUT_PREV_PAGE, N_("Previous page")},
        {SHORTCUT_NEXT_PAGE, N_("Next page")},
        {SHORTCUT_FIRST_PAGE, N_("First page")},
        {SHORTCUT_LAST_PAGE, N_("Last page")},
        {SHORTCUT_NEW_PAGE, N_("New page")},
        // TRANSLATORS: (Buttons sheet) the page's whole width again after a pinch zoomed it.
        {SHORTCUT_FIT_PAGE, N_("Fit page")},
        {SHORTCUT_PAGES, N_("Pages")},
        // TRANSLATORS: (Buttons sheet) the Pages view's Index tab, the list of named pages.
        {SHORTCUT_INDEX, N_("Index")},
        // TRANSLATORS: (Buttons sheet) the editor's tool picker, which its tool button opens.
        {SHORTCUT_TOOL_PICKER, N_("Tool picker")},
        {SHORTCUT_PAPER, N_("Paper")},
        {SHORTCUT_DISPLAY, N_("Display")},
        // TRANSLATORS: (Buttons sheet) the screen of each tool's pressure and tilt ("Pen feel: %s").
        {SHORTCUT_PEN_FEEL, N_("Pen feel")},
        // TRANSLATORS: (Buttons sheet) folds the editor's panel into its small box, or unfolds it.
        {SHORTCUT_FOLD_PANEL, N_("Fold the panel")},
        {SHORTCUT_LIBRARY, N_("Library")},
        // TRANSLATORS: (Buttons sheet) closes whichever of the editor's menus is open.
        {SHORTCUT_CLOSE_MENU, N_("Close menu")},
    };
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++)
        if (names[i].action == action) return _(names[i].label);
    return "?";
}

// The action the open row waits for a press for.
static int learning_action(App *a) {
    return learn_row_at(S->buttons_group, S->learn_row).act[S->learn_step];
}

// "South, Up" for an action's buttons, "none" when it has none; "B, Up" while the first controller
// connected is a Switch-mode one, as its buttons are printed.
static void pads_text(App *a, int action, char *out, size_t cap) {
    int codes[4], n = shortcuts_pads_for(&a->shortcuts, action, codes, 4);
    InputHid h[4];
    int nh = input_hids(h, 4);
    bool switch_layout = false;
    for (int i = 0; i < nh; i++)
        if (h[i].gamepad) {
            switch_layout = h[i].switch_layout;
            break;
        }
    out[0] = 0;
    for (int i = 0; i < n && i < 4; i++) {
        char name[16];
        shortcut_pad_label(codes[i], switch_layout, name, sizeof name);
        size_t len = strlen(out);
        snprintf(out + len, cap - len, "%s%s", i ? ", " : "", name);
    }
    // TRANSLATORS: (Buttons sheet) an action no controller button does.
    if (!n) snprintf(out, cap, "%s", C_("buttons", "none"));
}

static void controller_path(char *out, size_t cap) { snprintf(out, cap, "%s/controller.ini", store_root()); }

// The shortcuts again: shortcuts.ini's keys (and its buttons while there is no controller.ini).
static void reload_shortcuts(App *a) {
    char sp[1024], cp[1024];
    snprintf(sp, sizeof sp, "%s/shortcuts.ini", store_root());
    controller_path(cp, sizeof cp);
    shortcuts_load_all(&a->shortcuts, sp, cp);
}

// A controller's press while the Buttons sheet learns: true when it was taken (the row's action has
// that button now, or the file could not be written: a toast says so).
bool library_learn_key(App *a, int code, bool gamepad) {
    if (!S || S->modal != MOD_BUTTONS || S->buttons_group < 0 || S->learn_row < 0) return false;
    if (!gamepad) return true;  // (a keyboard's key is shortcuts.ini's: nothing to learn here)
    char cp[1024];
    controller_path(cp, sizeof cp);
    int action = learning_action(a), before = shortcuts_lookup(&a->shortcuts, true, code, 0);
    if (shortcuts_learn(&a->shortcuts, cp, code, action) != 0) {
        // TRANSLATORS: a toast: the Buttons sheet could not write its file (the storage is full).
        app_toast(a, _("Could not save controller.ini"), 4000);
        S->learn_row = -1;
        redraw(a, REFRESH_UI);
        return true;
    }
    if (before && before != action) {
        char msg[200];
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the button just learned
        // did something else, which it no longer does; %s is that action's name, as the Buttons sheet
        // names it ("Undo", "Pencil").
        snprintf(msg, sizeof msg, _("That button is no longer for %s."), action_label(before));
        app_toast(a, msg, 2500);
    }
    if (S->learn_step == 0 && learn_row_at(S->buttons_group, S->learn_row).act[1]) {
        S->learn_step = 1;
        S->learn_at = now_ms();
    } else {
        S->learn_row = -1;
    }
    redraw(a, REFRESH_UI);
    return true;
}

// A Buttons row held (ROW_HOLD_MS): its actions lose their buttons, this row's alone, and it reads
// "none"; a toast says so. Its release starts no learning.
static void forget_row(App *a, int row) {
    LearnRow r = learn_row_at(S->buttons_group, row);
    char cp[1024], msg[200];
    controller_path(cp, sizeof cp);
    S->learn_row = -1;
    if (shortcuts_forget(&a->shortcuts, cp, r.act[0], r.act[1]) != 0) {
        app_toast(a, _("Could not save controller.ini"), 4000);
    } else {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a finger held on a
        // Buttons sheet row, whose actions no controller button does now; %s is the row's name as the
        // sheet shows it ("Undo", "Previous / next page").
        snprintf(msg, sizeof msg, _("No buttons for %s now."), r.label ? _(r.label) : action_label(r.act[0]));
        app_toast(a, msg, 2000);
    }
    redraw(a, REFRESH_UI);
}

// The Controller sheet's status line: the controllers and keyboards Grimoire reads now.
static void hid_status(App *a, char *out, size_t cap) {
    InputHid h[4];
    int n = input_hids(h, 4);
    S->shown_hid_changes = a->hid_changes;
    // TRANSLATORS: the Controller sheet's status line: no controller or keyboard Grimoire reads now
    // (about 1100 px at 44 px in bold, and the same for the two below).
    if (!n) snprintf(out, cap, "%s", a->settings.shortcuts_enabled ? _("No controller or keyboard connected") : "");
    // TRANSLATORS: (Controller sheet) %s is the controller's own name, as it reports it: another app
    // (a button mapper) holds it, so Grimoire gets none of its presses.
    else if (h[0].held) snprintf(out, cap, _("%s: held by another app"), h[0].name);
    // TRANSLATORS: (Controller sheet) %s is the controller's or keyboard's own name.
    else snprintf(out, cap, _("%s connected"), h[0].name);
    if (n > 1) {
        size_t len = strlen(out);
        snprintf(out + len, cap - len, " (+%d)", n - 1);
    }
}

// ---- pairing a controller (the Bluetooth helper's API, bluetooth.h) ----

enum { ASK_NONE, ASK_STATUS, ASK_SCAN, ASK_SCAN_STATUS, ASK_PAIR, ASK_PAIR_STATUS, ASK_REMOVE };
#define SCAN_MS 15000
#define PAIR_MS 30000

// Asks the helper what (an ASK_*) after `in` ms, or as soon as the one under way is answered.
static void bt_ask(App *a, int what, uint32_t in) {
    S->bt_next = what;
    S->bt_next_at = now_ms() + in;
}

static void bt_send(App *a) {
    if (!S->bt_next || bt_busy() || (int32_t)(now_ms() - S->bt_next_at) < 0) return;
    char path[160];
    int what = S->bt_next;
    switch (what) {
    case ASK_STATUS: snprintf(path, sizeof path, "/status"); break;
    case ASK_SCAN: snprintf(path, sizeof path, "/scan"); break;
    case ASK_SCAN_STATUS: snprintf(path, sizeof path, "/scan-status"); break;
    case ASK_PAIR_STATUS: snprintf(path, sizeof path, "/pair-status"); break;
    case ASK_PAIR: {
        bool classic = false;
        for (int i = 0; i < S->nfound; i++)
            if (!strcmp(S->found[i].address, S->pair_addr)) classic = S->found[i].classic;
        snprintf(path, sizeof path, "/pair?addr=%s&protocol=%s", S->pair_addr, classic ? "classic" : "ble");
        break;
    }
    case ASK_REMOVE: snprintf(path, sizeof path, "/remove?addr=%s", S->pair_addr); break;
    default: return;
    }
    if (!bt_request(path)) return;
    S->bt_asked = what;
    S->bt_next = ASK_NONE;
}

static void pair_open(App *a) {
    S->nfound = S->npaired = 0;
    S->scanning = true;
    S->pairing = false;
    S->scan_until = now_ms() + SCAN_MS;
    S->bt_asked = ASK_NONE;
    bt_ask(a, ASK_SCAN, 0);
    S->modal = MOD_PAIR;
}

// The helper's answer (or none) to what the Pair sheet asked.
void library_bt(App *a, BtEvent e, const BtReply *r) {
    if (!S) return;
    if (e == BT_EV_STATE) {
        if (S->modal == MOD_SHORTCUTS || S->modal == MOD_PAIR) redraw(a, REFRESH_UI);
        return;
    }
    int asked = S->bt_asked;
    S->bt_asked = ASK_NONE;
    if (S->modal != MOD_PAIR) return;
    bool ok = e == BT_EV_REPLY && r->ok;
    switch (asked) {
    case ASK_SCAN:
        if (ok) bt_ask(a, ASK_SCAN_STATUS, 1000);
        else S->scanning = false;
        break;
    case ASK_SCAN_STATUS:
        if (e == BT_EV_REPLY) {
            S->nfound = 0;
            for (int i = 0; i < r->ndev && S->nfound < 4; i++) S->found[S->nfound++] = r->dev[i];
        }
        if (ok && r->scanning && (int32_t)(now_ms() - S->scan_until) < 0) {
            bt_ask(a, ASK_SCAN_STATUS, 1000);
        } else {
            S->scanning = false;
            bt_ask(a, ASK_STATUS, 0);  // and which are paired
        }
        break;
    case ASK_STATUS:
        S->npaired = 0;
        for (int i = 0; e == BT_EV_REPLY && i < r->ndev && S->npaired < 2; i++) S->paired[S->npaired++] = r->dev[i];
        break;
    case ASK_PAIR:
        if (ok) bt_ask(a, ASK_PAIR_STATUS, 1000);
        else {
            S->pairing = false;
            app_toast(a, _("Could not pair that controller"), 3000);
        }
        break;
    case ASK_PAIR_STATUS:
        if (ok && r->pairing && (int32_t)(now_ms() - S->pair_until) < 0) {
            bt_ask(a, ASK_PAIR_STATUS, 1000);
            break;
        }
        S->pairing = false;
        // TRANSLATORS: toasts as pairing a controller ends (Settings > Controller > Pair a controller).
        app_toast(a, ok && !r->pairing ? _("Paired. The controller connects in a moment.") : _("Could not pair that controller"), 3000);
        bt_ask(a, ASK_STATUS, 0);
        break;
    case ASK_REMOVE:
        // TRANSLATORS: a toast: the controller is no longer paired with the Kindle.
        app_toast(a, ok ? _("Forgotten.") : _("Could not forget that controller"), 2500);
        bt_ask(a, ASK_STATUS, 0);
        break;
    default: break;
    }
    redraw(a, REFRESH_UI);
}

// ---- Settings > Updates (update.h) ----

enum { UPD_IDLE, UPD_CHECKING, UPD_DONE, UPD_FAILED };

// Which channel's updates: the setting, or by the version (a beta or an RC: preview).
static bool upd_preview(App *a) {
    return a->settings.updates == 2 || (a->settings.updates == 0 && update_is_prerelease(GRIMOIRE_VERSION));
}

static bool upd_newer(App *a) { return S->upd == UPD_DONE && update_version_cmp(S->upd_latest, GRIMOIRE_VERSION) > 0; }

// The update check's news (from library_tick).
static void upd_poll(App *a) {
    if (S->upd != UPD_CHECKING) return;
    UpdateEvent e = update_tick(S->upd_latest, S->upd_why, sizeof S->upd_why);
    if (!e) return;
    S->upd = e == UPDATE_EV_DONE ? UPD_DONE : UPD_FAILED;
    if (S->modal == MOD_UPDATES) redraw(a, REFRESH_UI);
}

// A vertical list of full-width buttons inside a sheet.
static void draw_menu(App *a, const char *title, const MenuRow *rows, int n) {
    int rh = ui_px(140), gap = ui_px(20);
    // A list longer than the screen has room for gets shorter rows, never below a 10 mm touch target,
    // then narrower gaps between them (sideways the Buttons sheet's Tools: 12 rows).
    int room = a->h - 2 * ui_px(40) - ui_px(120) - ui_px(72);
    if (n * (rh + gap) > room) rh = clampi(room / n - gap, ui_px(118), rh);
    if (n * (rh + gap) > room) gap = clampi(room / n - rh, ui_px(10), gap);
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
    Rect ko = {in.x, done.y, ui_px(420), done.h};
    ui_button(c, ko, ICON_NOTEBOOK, "KOReader", BTN_PLAIN);
    hits_add(&S->hits, ko, H_KOREADER, 0);
    // Between KOReader and Done: Controller and Updates, each half the room.
    int mid = done.x - ko.x - ko.w - ui_px(90);
    Rect keys = {ko.x + ko.w + ui_px(30), done.y, mid / 2, done.h};
    Rect upd = {keys.x + keys.w + ui_px(30), done.y, mid - mid / 2, done.h};
    // TRANSLATORS: Settings' bottom buttons, each a word under its icon (about 360 px at 34 px):
    // to the Controller sheet (a controller's or keyboard's buttons as shortcuts), and to the
    // Updates sheet (check for a newer Grimoire and install it).
    ui_button(c, keys, ICON_GAMEPAD, _("Controller"), BTN_PLAIN);
    hits_add(&S->hits, keys, H_SHORTCUTS, 0);
    ui_button(c, upd, ICON_UPDATE, _("Updates"), BTN_PLAIN);
    hits_add(&S->hits, upd, H_UPDATES, 0);
}

static void draw_modal(App *a) {
    Item *it = (S->target >= 0 && S->target < S->nitems) ? &S->items[S->target] : NULL;
    char label[STORE_NAME_LEN + 32];  // (a name as the screens show it: ui_name_label)
    switch (S->modal) {
    case MOD_NEW: draw_new(a); break;
    case MOD_SETTINGS: draw_settings(a); break;
    case MOD_UPDATES: {
        char status[200], channel[200], now[160];
        bool avail = update_available();
        // TRANSLATORS: the Updates sheet's first line (about 1100 px at 44 px in bold): the version
        // running; then while it checks; then what the check found: a newer one (%s), or that this
        // one (%s) is the newest; or why it could not check.
        if (!avail) snprintf(status, sizeof status, "%s", _("Updates come through KPM, which is not here."));
        else if (S->upd == UPD_CHECKING) snprintf(status, sizeof status, "%s", _("Checking for updates..."));
        else if (S->upd == UPD_DONE && upd_newer(a)) snprintf(status, sizeof status, _("Grimoire %s is available."), S->upd_latest);
        else if (S->upd == UPD_DONE) snprintf(status, sizeof status, _("Grimoire %s is the newest."), GRIMOIRE_VERSION);
        else if (S->upd == UPD_FAILED && !strcmp(S->upd_why, "network")) snprintf(status, sizeof status, "%s", _("Could not check. Is Wi-Fi on?"));
        else if (S->upd == UPD_FAILED && !strcmp(S->upd_why, "no-kpm")) snprintf(status, sizeof status, "%s", _("Updates come through KPM, which is not here."));
        else if (S->upd == UPD_FAILED) snprintf(status, sizeof status, "%s", _("Could not check for updates."));
        else snprintf(status, sizeof status, _("This is Grimoire %s."), GRIMOIRE_VERSION);
        // TRANSLATORS: (Updates sheet) which updates: betas too (the preview channel), or releases
        // only (stable); a tap changes it.
        snprintf(channel, sizeof channel, "%s", upd_preview(a) ? _("Updates: betas and releases (tap for releases only)")
                                                              : _("Updates: releases only (tap for betas too)"));
        // TRANSLATORS: (Updates sheet) installs the newer Grimoire (%s): Grimoire closes, KPM
        // installs it, and Grimoire starts again.
        snprintf(now, sizeof now, _("Update to %s"), S->upd_latest);
        MenuRow rows[] = {
            {ICON_NONE, status, H_NONE, 0, BTN_DISABLED},
            {ICON_NONE, channel, H_UPD_CHANNEL, 0, avail && S->upd != UPD_CHECKING ? BTN_PLAIN : BTN_DISABLED},
            // TRANSLATORS: (Updates sheet) asks Grimoire's feed for a newer Grimoire (through KPM).
            {ICON_UPDATE, _("Check for updates"), H_UPD_CHECK, 0, avail && S->upd != UPD_CHECKING ? BTN_PLAIN : BTN_DISABLED},
            {ICON_DOWNLOAD, upd_newer(a) ? now : _("Update"), H_UPD_NOW, 0, upd_newer(a) ? BTN_PRIMARY : BTN_DISABLED},
            {ICON_CLOSE, _("Done"), H_CANCEL, 0, BTN_PLAIN},
        };
        // TRANSLATORS: the Updates sheet's title (Settings > Updates).
        draw_menu(a, _("Updates"), rows, 5);
        break;
    }
    case MOD_PAIR: {
        MenuRow rows[8];
        char names[6][BT_NAME_LEN + 40];
        int n = 0;
        // TRANSLATORS: the Pair sheet's first line: what to do with the controller (each has its own
        // way: often a button held a few seconds).
        rows[n++] = (MenuRow){ICON_NONE, S->scanning ? _("Searching... Put the controller in pairing mode.")
                                       : S->pairing ? _("Pairing...") : _("Tap a controller to pair it."),
                              H_NONE, 0, BTN_DISABLED};
        for (int i = 0; i < S->nfound && n < 5; i++) {
            const BtDevice *d = &S->found[i];
            snprintf(names[n], sizeof names[n], "%s", d->name[0] ? d->name : d->address);
            bool chosen = S->pairing && !strcmp(d->address, S->pair_addr);
            rows[n] = (MenuRow){ICON_NONE, names[n], H_PAIR_FOUND, i,
                                chosen ? BTN_SELECTED : S->pairing ? BTN_DISABLED : BTN_PLAIN};
            n++;
        }
        for (int i = 0; i < S->npaired && n < 6; i++) {
            // TRANSLATORS: (Pair sheet) unpairs a controller paired before; %s is its own name.
            snprintf(names[n], sizeof names[n], _("Forget %s"), S->paired[i].name[0] ? S->paired[i].name : S->paired[i].address);
            rows[n] = (MenuRow){ICON_TRASH, names[n], H_PAIR_FORGET, i, S->pairing ? BTN_DISABLED : BTN_PLAIN};
            n++;
        }
        // TRANSLATORS: (Pair sheet) searches for controllers again.
        rows[n++] = (MenuRow){ICON_NONE, _("Search again"), H_PAIR_AGAIN, 0, S->scanning || S->pairing ? BTN_DISABLED : BTN_PLAIN};
        rows[n++] = (MenuRow){ICON_BACK, _("Done"), H_BUTTONS_DONE, 0, BTN_PLAIN};
        // TRANSLATORS: the Pair sheet's title (Settings > Controller > Pair a controller).
        draw_menu(a, _("Pair a controller"), rows, n);
        break;
    }
    case MOD_BUTTONS: {
        char text[16][200];
        MenuRow rows[16];
        int n = 0, g = S->buttons_group;
        if (g < 0) {
            for (int i = 0; i < N_GROUPS; i++)
                rows[n++] = (MenuRow){k_groups[i].icon, _(k_groups[i].label), H_BUTTON_GROUP, i, BTN_PLAIN};
            char cp[1024];
            controller_path(cp, sizeof cp);
            // TRANSLATORS: the Buttons sheet: forget the buttons learned (back to shortcuts.ini's).
            rows[n++] = (MenuRow){ICON_TRASH, _("Reset buttons"), H_BUTTONS_RESET, 0,
                                  access(cp, F_OK) == 0 ? BTN_PLAIN : BTN_DISABLED};
            rows[n++] = (MenuRow){ICON_BACK, _("Done"), H_BUTTONS_DONE, 0, BTN_PLAIN};
            // TRANSLATORS: the Buttons sheet's title (Settings > Controller > Buttons): a group of
            // actions, then a tap on a row and a press on the controller, makes that button do it.
            draw_menu(a, _("Controller buttons"), rows, n);
            break;
        }
        S->shown_hid_changes = a->hid_changes;  // (pads_text names the buttons for the controller connected)
        for (int i = 0; i < group_rows(g); i++) {
            LearnRow r = learn_row_at(g, i);
            bool learning = S->learn_row == i;
            if (learning) {
                // TRANSLATORS: the Buttons sheet's row while it waits for a press (10 s): %s is the
                // action's name as the rows show it ("Undo", "Pencil", "Next page"). About 1100 px at
                // 44 px in bold.
                snprintf(text[n], sizeof text[n], _("Press a button for %s"), action_label(r.act[S->learn_step]));
            } else {
                char p1[64], p2[64];
                pads_text(a, r.act[0], p1, sizeof p1);
                const char *label = r.label ? _(r.label) : action_label(r.act[0]);
                if (r.act[1]) {
                    pads_text(a, r.act[1], p2, sizeof p2);
                    snprintf(text[n], sizeof text[n], "%s: %s / %s", label, p1, p2);
                } else {
                    snprintf(text[n], sizeof text[n], "%s: %s", label, p1);
                }
            }
            rows[n] = (MenuRow){r.icon, text[n], H_BUTTON_ROW, i, learning ? BTN_SELECTED : BTN_PLAIN};
            n++;
        }
        rows[n++] = (MenuRow){ICON_BACK, _("Done"), H_BUTTONS_DONE, 0, BTN_PLAIN};
        draw_menu(a, _(k_groups[g].title), rows, n);
        break;
    }
    case MOD_SHORTCUTS: {
        char count[160], last[160], status[160];
        // TRANSLATORS: (Controller sheet) how many shortcuts grimoire/shortcuts.ini (and controller.ini)
        // hold, and how many of their lines were refused.
        snprintf(count, sizeof count, _("Mappings: %d; rejected: %d"), a->shortcuts.count, a->shortcuts.rejected);
        if (a->last_shortcut_code)
            snprintf(last, sizeof last, _("Last input: %s %d; modifiers: %d"), a->last_shortcut_gamepad ? "pad" : "key",
                     a->last_shortcut_code, a->last_shortcut_mods);
        else snprintf(last, sizeof last, "%s", _("Press a connected key or controller button"));
        S->shown_shortcut_code = a->last_shortcut_code;
        S->shown_shortcut_mods = a->last_shortcut_mods;
        S->shown_shortcut_gamepad = a->last_shortcut_gamepad;
        hid_status(a, status, sizeof status);
        S->shown_bt_changes = a->bt_changes;
        BtState bs = bt_state();
        // TRANSLATORS: (Controller sheet) its status line while the Bluetooth helper starts, and when it
        // would not start.
        if (bs == BT_STARTING) snprintf(status, sizeof status, "%s", _("Bluetooth: starting..."));
        else if (bs == BT_FAILED) snprintf(status, sizeof status, "%s", _("Bluetooth did not start (see grimoire.log)"));
        bool on = a->settings.shortcuts_enabled, present = bt_present();
        bool bt_up = bs == BT_OWNED || bs == BT_EXTERNAL;
        char reload[160];
        // TRANSLATORS: (Controller sheet) reads grimoire/shortcuts.ini (and controller.ini) again: how
        // many shortcuts they hold, and how many of their lines were refused.
        snprintf(reload, sizeof reload, _("Reload shortcuts.ini (%d, %d refused)"), a->shortcuts.count, a->shortcuts.rejected);
        MenuRow rows[] = {
            {on ? ICON_CHECK : ICON_NONE, _("Enable shortcuts"), H_SHORTCUT_ENABLE, 0, BTN_PLAIN},
            // TRANSLATORS: (Controller sheet) starts the Bluetooth helper with Grimoire, for a
            // Bluetooth controller; without the helper unpacked in grimoire/bluetooth, the second.
            {a->settings.controller && present ? ICON_CHECK : ICON_NONE,
             present ? _("Bluetooth controller") : _("No Bluetooth helper in grimoire/bluetooth"), H_BT_TOGGLE, 0,
             present && on ? BTN_PLAIN : BTN_DISABLED},
            // TRANSLATORS: (Controller sheet) opens the Pair sheet: find a Bluetooth controller and pair it.
            {ICON_PLUS, _("Pair a controller..."), H_PAIR, 0, bt_up && on ? BTN_PLAIN : BTN_DISABLED},
            {ICON_NONE, status, H_NONE, 0, BTN_DISABLED},
            // TRANSLATORS: (Controller sheet) opens the Buttons sheet: which controller button does what.
            {ICON_NONE, _("Buttons..."), H_BUTTONS, 0, on ? BTN_PLAIN : BTN_DISABLED},
            {ICON_NONE, reload, H_SHORTCUT_RELOAD, 0, BTN_PLAIN},
            {ICON_NONE, last, H_NONE, 0, BTN_DISABLED},
            {ICON_CLOSE, _("Done"), H_CANCEL, 0, BTN_PLAIN},
        };
        (void)count;
        // TRANSLATORS: the Controller sheet's title (Settings > Controller): a controller's or a
        // keyboard's buttons as shortcuts.
        draw_menu(a, _("Controller and keys"), rows, 8);
        break;
    }
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
                if (store_remove_folder(it->folder) == 0)
                    export_mirror_prune_folder(it->folder, app_mirror_dir());
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
    case H_SHORTCUTS: open_modal(a, MOD_SHORTCUTS); break;
    case H_SHORTCUT_ENABLE:
    case H_SHORTCUT_RELOAD: {
        char path[1024]; snprintf(path, sizeof path, "%s/shortcuts.ini", store_root());
        if (h->id == H_SHORTCUT_ENABLE) {
            if (!a->settings.shortcuts_enabled && shortcuts_write_defaults(path) != 0) {
                app_toast(a, _("Could not save shortcuts.ini"), 5000); break;
            }
            a->settings.shortcuts_enabled = !a->settings.shortcuts_enabled;
            settings_save(&a->settings);
            qlog("settings: shortcuts %d, controller %d", a->settings.shortcuts_enabled, a->settings.controller);
            input_shortcuts_enable(a->settings.shortcuts_enabled);
            // The Bluetooth helper serves shortcuts alone: it goes with them, and comes back with
            // them while Bluetooth controller stays on.
            if (!a->settings.shortcuts_enabled) bt_stop();
            else if (a->settings.controller && bt_present()) bt_start();
        }
        reload_shortcuts(a);
        redraw(a, REFRESH_UI);
        break;
    }
    case H_BUTTONS:
        S->learn_row = S->buttons_group = -1;
        open_modal(a, MOD_BUTTONS);
        break;
    case H_BUTTON_GROUP:
        S->buttons_group = clampi(h->arg, 0, N_GROUPS - 1);
        S->learn_row = -1;
        redraw(a, REFRESH_UI);
        break;
    case H_BUTTON_ROW:
        S->learn_row = S->learn_row == h->arg ? -1 : h->arg;  // (a second tap stops it)
        S->learn_step = 0;
        S->learn_at = now_ms();
        redraw(a, REFRESH_UI);
        break;
    case H_BUTTONS_RESET: {
        char cp[1024];
        controller_path(cp, sizeof cp);
        unlink(cp);
        S->learn_row = -1;
        reload_shortcuts(a);
        redraw(a, REFRESH_UI);
        break;
    }
    case H_BUTTONS_DONE:
        S->learn_row = -1;
        if (S->modal == MOD_BUTTONS && S->buttons_group >= 0) {  // a group: back to the list
            S->buttons_group = -1;
            redraw(a, REFRESH_UI);
            break;
        }
        S->bt_next = ASK_NONE;
        open_modal(a, MOD_SHORTCUTS);
        break;
    case H_UPDATES:
        if (S->upd != UPD_CHECKING) S->upd = UPD_IDLE;
        open_modal(a, MOD_UPDATES);
        break;
    case H_UPD_CHANNEL:
        a->settings.updates = upd_preview(a) ? 1 : 2;
        settings_save(&a->settings);
        S->upd = UPD_IDLE;  // (what a check found was the other channel's)
        redraw(a, REFRESH_UI);
        break;
    case H_UPD_CHECK:
        if (update_check(upd_preview(a))) S->upd = UPD_CHECKING;
        redraw(a, REFRESH_UI);
        break;
    case H_UPD_NOW:
        if (!upd_newer(a)) break;
        if (update_request(upd_preview(a)) != 0) {
            // TRANSLATORS: a toast: the update could not be asked for (the storage is full, say).
            app_toast(a, _("Could not start the update"), 3000);
            break;
        }
        qlog("update: leaving so KPM can install %s", S->upd_latest);
        app_leave_for_update(a);
        break;
    case H_BT_TOGGLE:
        a->settings.controller = !a->settings.controller;
        settings_save(&a->settings);
        qlog("settings: shortcuts %d, controller %d", a->settings.shortcuts_enabled, a->settings.controller);
        if (a->settings.controller) bt_start();
        else bt_stop();
        redraw(a, REFRESH_UI);
        break;
    case H_PAIR:
        pair_open(a);
        redraw(a, REFRESH_UI);
        break;
    case H_PAIR_AGAIN:
        pair_open(a);
        redraw(a, REFRESH_UI);
        break;
    case H_PAIR_FOUND:
        if (S->pairing || h->arg < 0 || h->arg >= S->nfound) break;
        snprintf(S->pair_addr, sizeof S->pair_addr, "%s", S->found[h->arg].address);
        S->pairing = true;
        S->scanning = false;
        S->pair_until = now_ms() + PAIR_MS;
        bt_ask(a, ASK_PAIR, 0);
        redraw(a, REFRESH_UI);
        break;
    case H_PAIR_FORGET:
        if (S->pairing || h->arg < 0 || h->arg >= S->npaired) break;
        snprintf(S->pair_addr, sizeof S->pair_addr, "%s", S->paired[h->arg].address);
        bt_ask(a, ASK_REMOVE, 0);
        break;
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
        if (it && store_remove_folder(it->folder) == 0)
            export_mirror_prune_folder(it->folder, app_mirror_dir());
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
    S->learn_row = S->buttons_group = -1;
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
        S->pressed_at = now_ms();
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
    if (S->modal == MOD_SHORTCUTS && (S->shown_shortcut_code != a->last_shortcut_code ||
        S->shown_shortcut_mods != a->last_shortcut_mods || S->shown_shortcut_gamepad != a->last_shortcut_gamepad ||
        S->shown_hid_changes != a->hid_changes))
        redraw(a, REFRESH_UI);
    // A group's rows name the buttons as the controller connected labels them.
    if (S->modal == MOD_BUTTONS && S->buttons_group >= 0 && S->shown_hid_changes != a->hid_changes)
        redraw(a, REFRESH_UI);
    if (S->modal == MOD_PAIR) bt_send(a);
    upd_poll(a);
    if (S->modal == MOD_SHORTCUTS && S->shown_bt_changes != a->bt_changes) redraw(a, REFRESH_UI);
    if (S->modal == MOD_BUTTONS && S->learn_row >= 0 && now - S->learn_at >= LEARN_MS) {
        S->learn_row = -1;  // no press in 10 s: as it was
        redraw(a, REFRESH_UI);
    }
    // A finger held still on a Buttons row: its buttons go, and nothing is pressed any more (the
    // release does nothing; the redraw took the pressed look away).
    if (S->modal == MOD_BUTTONS && S->buttons_group >= 0 && S->pressed && S->pressed->id == H_BUTTON_ROW &&
        !S->press.moved && now - S->pressed_at >= ROW_HOLD_MS) {
        int row = S->pressed->arg;
        S->pressed = NULL;
        forget_row(a, row);
    }
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
        [MOD_SHORTCUTS] = "shortcuts",
        [MOD_BUTTONS] = "buttons",
        [MOD_PAIR] = "pair",
        [MOD_UPDATES] = "updates",
    };
    if (!S) return -1;
    if (!strcmp(key, "sheet")) {
        const char *m = (size_t)S->modal < sizeof sheets / sizeof sheets[0] ? sheets[S->modal] : NULL;
        if (m) snprintf(out, cap, "%s", m);
        else snprintf(out, cap, "%d", (int)S->modal);
    } else if (!strcmp(key, "learning")) {
        // The action the Buttons sheet waits for a press for ("undo", "next-page"), else empty.
        const char *n = S->modal == MOD_BUTTONS && S->buttons_group >= 0 && S->learn_row >= 0
                            ? shortcut_action_name(learning_action(a)) : NULL;
        snprintf(out, cap, "%s", n ? n : "");
    } else if (!strncmp(key, "buttons.", 8)) {
        // The Buttons sheet's buttons for an action ("buttons.undo"): "B, Up", "none".
        pads_text(a, shortcut_action(key + 8), out, cap);
    } else if (!strcmp(key, "buttons_group")) {
        // The Buttons sheet's group open ("editing", "tools", "pages", "menus"), else empty.
        bool open = S->modal == MOD_BUTTONS && S->buttons_group >= 0;
        snprintf(out, cap, "%s", open ? k_groups[S->buttons_group].probe : "");
    } else if (!strcmp(key, "update")) {
        // The Updates sheet's check: "idle", "checking", "newer <version>", "newest <version>" or
        // "failed <why>".
        if (S->upd == UPD_CHECKING) snprintf(out, cap, "checking");
        else if (S->upd == UPD_DONE) snprintf(out, cap, "%s %s", upd_newer(a) ? "newer" : "newest", S->upd_latest);
        else if (S->upd == UPD_FAILED) snprintf(out, cap, "failed %s", S->upd_why);
        else snprintf(out, cap, "idle");
    } else if (!strcmp(key, "pair")) {
        // The Pair sheet: "scanning", "pairing" or "idle", then the controllers found and paired.
        snprintf(out, cap, "%s %d %d", S->scanning ? "scanning" : S->pairing ? "pairing" : "idle", S->nfound, S->npaired);
    } else if (!strcmp(key, "bluetooth")) {
        static const char *const names[] = {"stopped", "starting", "owned", "external", "failed", "stopping"};
        snprintf(out, cap, "%s", names[bt_state()]);
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
