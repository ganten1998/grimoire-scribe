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
    H_MORE_OPEN, H_MORE_RENAME, H_MORE_MOVE, H_More_PDF, H_More_Delete,
    H_FMore_Rename, H_FMore_Remove,
    H_MOVE_TO, H_MOVE_TOP, H_MOVE_NEW,
    H_DELETE_YES,
    H_PAPER_MORE, H_KOREADER, H_KO_ITEM, H_SHORTCUTS, H_SHORTCUT_ENABLE, H_SHORTCUT_RELOAD,
    H_BUTTONS, H_BUTTON_GROUP, H_BUTTON_ROW, H_BUTTONs_RESET, H_BUTTONS_DONE,
    H_BT_TOGGLE, H_PAIR, H_PAIR_FOUND, H_PAIR_FORGET, H_PAIR_AGAIN,
    H_UPDATES, H_UPD_CHANNEL, H_UPD_Check, H_UPD_NOW,
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
    {NULL, {SHORTCUT_ADD_LINK, 0}, ICON_LINK},
    {NULL, {SHORTCUT_NAME_PAGE, 0}, ICON_RENAME},
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
    // TRANSLATORS: (Buttons sheet) its group of page turning, new pages, the Pages view and the
    // Index, and links and page names.
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
        const char *label, *ctx;  // ctx: its msgctxt, for a name the Index shares in English
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
        // TRANSLATORS: (Buttons sheet) the tool picker's previous and next pen (\"Press a button for
        // Next in the picker\").
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
        // TRANSLATORS: (Buttons sheet) starts a link on the open page, as the Index's Add a link
        // does: what the pen draws next is its box, then a tap picks where it goes (\"Press a button
        // for Add a link\").
        {SHORTCUT_ADD_LINK, NC_("buttons", "Add a link"), "buttons"},
        // TRANSLATORS: (Buttons sheet) the naming keyboard for the open page, as the Index's Name
        // this page opens it.
        {SHORTCUT_NAME_PAGE, N_("Name the page")},
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
        if (names[i].action == action) return names[i].ctx ? C_(names[i].ctx, names[i].label) : _(names[i].label);
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
        snprintf(out + len, cap - len, " (%d)", n - 1);
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
}// Draw settings with sidebar categories
// This redesign organizes the 15 existing settings into 5 logical categories
// with a sidebar on the left and submenus on the right, inspired by Kindle settings
// and fullscreen settings.
static void draw_settings(App *a) {
    Canvas *c = &a->screen;
    Rect card = sheet_rect(a, ui_px(1820));
    Rect in = ui_sheet(c, card, _("Settings"));

    /* Category state - track which category is active */
    static int s_active_category = -1;

    /* Category definitions - icons and labels (string literals for static init, translated at draw time) */
    typedef struct {
        IconKind icon;
        const char *label;  // string literal, translated at draw time via _()
    } Category;

    /* Category definitions with string literals for static initialization */
    static const Category k_categories[5] = {
        {ICON_TARGET, "Calibration"},
        {ICON_BRIGHTNESS, "Display"},
        {ICON_HIGHLIGHTER, "Pen Feel"},
        {ICON_LANGUAGE, "Language"},
        {ICON_GEAR, "General"},
    };

    /* Submenu definitions for each category - using existing hit IDs */
    typedef struct {
        const char *label;
        int hit_id;  /* hit test ID for this setting row */
        int arg;     /* optional argument */
    } SubmenuEntry;

    /* Submenus for each category */
    static const SubmenuEntry k_calibration_submenu[] = {
        {"Calibrate pen and touch", H_SET_CALIB, 0},
        {"Mirror PDFs to USB", H_SET_MIRROR, 0},
        {"Side button action", H_SET_SIDE, 0},
        {"Auto-rotate screen", H_SET_ROTATE, 0},
        {"Two-finger pinch to zoom", H_SET_PINCH, 0},
    };

    static const SubmenuEntry k_display_submenu[] = {
        {"Brightness", H_SET_DISPLAY, 0},
        {"Warmth", 0, 0},
        {"Invert colors", 0, 0},
        {"Pin this page as sleep screen", 0, 0},
        {"Sleep after (minutes)", 0, 0},
    };

    static const SubmenuEntry k_penfeel_submenu[] = {
        {"Pressure per tool", 0, 0},
        {"Tilt per tool", 0, 0},
        {"Eraser mode", 0, 0},
        {"White ink", 0, 0},
    };

    static const SubmenuEntry k_language_submenu[] = {
        {"Language", H_SET_LANGUAGE, 0},
        {"Done", 0, 0},
    };

    static const SubmenuEntry k_general_submenu[] = {
        {"Theme: Gothic", 0, 0},
        {"Theme: Classic", 0, 0},
        {"Data location", 0, 0},
        {"About Grimoire", 0, 0},
    };

    /* Draw the sidebar */
    int sidebar_w = ui_px(180);
    int category_h = ui_px(80);
    int gap = ui_px(12);
    int y = in.y;

    /* Draw each category button in the sidebar */
    for (int i = 0; i < 5; i++) {
        Rect cat_r = {in.x, y, sidebar_w, category_h};
        bool is_active = (s_active_category == i);

        /* Draw category background - selected gets a different color */
        if (is_active) {
            canvas_fill(c, cat_r, 240);  /* light blue highlight */
        } else {
            canvas_fill(c, cat_r, 255);
        }

        /* Draw icon */
        int icon_size = ui_px(36);
        Rect icon_r = {cat_r.x + ui_px(20), cat_r.y + (category_h - icon_size) / 2, icon_size, icon_size};
        ui_icon(c, k_categories[i].icon, icon_r, is_active ? 255 : 200);

        /* Draw label - translated at draw time using _() */
        float label_x = cat_r.x + icon_size + ui_px(20);
        float label_y = cat_r.y + (category_h - ui_text_size()) / 2;
        uint8_t label_fg = is_active ? 0 : 0;  /* black text for selected, dark gray for others */
        /* Use _() wrapped string for i18n at draw time */
        font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){label_x, label_y, cat_r.w - icon_size - ui_px(40), category_h},
                       _(k_categories[i].label), label_fg);

        y += category_h + gap;
    }

    /* Draw the active category's submenu on the right side */
    if (s_active_category >= 0 && s_active_category < 5) {
        int submenu_w = a->w - in.x - sidebar_w;  /* remaining width on the right */
        int entry_h = ui_px(60);
        int gap2 = ui_px(8);
        int start_y = in.y;

        /* Get the submenu entries for the active category */
        const SubmenuEntry *submenu;
        int n_entries = 0;

        switch (s_active_category) {
            case 0: /* Calibration */
                submenu = k_calibration_submenu;
                n_entries = sizeof(k_calibration_submenu) / sizeof(k_calibration_submenu[0]);
                break;
            case 1: /* Display */
                submenu = k_display_submenu;
                n_entries = sizeof(k_display_submenu) / sizeof(k_display_submenu[0]);
                break;
            case 2: /* Pen Feel */
                submenu = k_penfeel_submenu;
                n_entries = sizeof(k_penfeel_submenu) / sizeof(k_penfeel_submenu[0]);
                break;
            case 3: /* Language */
                submenu = k_language_submenu;
                n_entries = sizeof(k_language_submenu) / sizeof(k_language_submenu[0]);
                break;
            case 4: /* General */
                submenu = k_general_submenu;
                n_entries = sizeof(k_general_submenu) / sizeof(k_general_submenu[0]);
                break;
        }

        /* Draw submenu background panel */
        Rect submenu_r = {in.x + sidebar_w + ui_px(20), in.y, submenu_w, in.h};
        canvas_fill(c, submenu_r, 255);

        /* Draw submenu title */
        Rect title_r = {in.x + sidebar_w + ui_px(20), in.y, submenu_w, ui_px(40)};
        canvas_fill(c, title_r, 245);
        /* Draw submenu title translated at draw time */
        font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){title_r.x + ui_px(10), title_r.y, title_r.w - ui_px(20), title_r.h},
                       _(k_categories[s_active_category].label), 0);

        /* Draw each submenu entry */
        for (int i = 0; i < n_entries; i++) {
            Rect entry_r = {in.x + sidebar_w + ui_px(20), start_y + i * (entry_h + gap2), submenu_w - ui_px(40), entry_h};

            /* Draw entry background */
            bool is_selected = false;  /* could track selection if needed */
            if (is_selected) {
                canvas_fill(c, entry_r, 240);
            } else {
                canvas_fill(c, entry_r, 255);
            }

            /* Draw label - use _() wrapped string from submenu for i18n at draw time */
            font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){entry_r.x + ui_px(10), entry_r.y, entry_r.w - ui_px(20), entry_h},
                           _(submenu[i].label), 0);

            /* Add hit area for tapping */
            if (submenu[i].hit_id > 0) {
                hits_add(&S->hits, entry_r, submenu[i].hit_id, s_active_category);
            }

            start_y += entry_h + gap2;
        }
    }

    /* Draw Done button at the bottom */
    Rect done = {in.x + in.w - ui_px(380), in.y + in.h - ui_px(140), ui_px(380), ui_px(140)};
    ui_button(c, done, ICON_CHECK, _("Done"), BTN_PRIMARY);
    hits_add(&S->hits, done, H_SETTINGS, 0);  // Use existing hit ID
}
