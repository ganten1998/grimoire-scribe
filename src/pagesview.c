// pagesview.c: the editor's Pages view (pagesview.h): numbered tiles; never render other pages on
// this screen.
#include "pagesview.h"
#include "i18n.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { PG_BACK = 1, PG_DELETE, PG_ALL, PG_TOGGLE, PG_OPEN, PG_PREV, PG_NEXT, PG_JUMP, PG_NEW, PG_COPY, PG_TAB, PG_NAME,
       PG_LINK, PG_TO_INDEX, PG_UNLINK };

static void geom(App *a, int *cols, int *rows, int *cw, int *ch, int *th) {
    *cols = a->w > a->h ? 6 : 4;
    int m = ui_px(50);
    *cw = (a->w - m * (*cols + 1)) / *cols;
    *th = ui_px(200);
    *ch = *th + ui_px(90);
    *rows = (a->h - ui_px(190) - ui_px(170) - m) / (*ch + m);
    if (*rows < 1) *rows = 1;
}

int pagesview_ticked_count(const PagesView *v) {
    int k = 0;
    for (int i = 0; i < v->nb->npages && i < v->sel_cap; i++) k += v->sel[i];
    return k;
}

bool pagesview_ticked(const PagesView *v, int index) { return index >= 0 && index < v->sel_cap && v->sel[index]; }

void pagesview_untick(PagesView *v) { memset(v->sel, 0, (size_t)v->sel_cap * sizeof *v->sel); }

bool pagesview_open(PagesView *v, App *a, const NotebookInfo *nb, const Nav *nav, int current) {
    v->nb = nb;
    v->nav = nav;
    if (!v->tab_initialized) {
        v->index_tab = nav_index(nav, nb, NULL, 0) > 0;
        v->index_screen = 0;
        v->tab_initialized = true;
    }
    v->current = current;
    v->picking = v->pick_had = false;
    if (v->sel_cap < nb->npages) {
        bool *n = realloc(v->sel, (size_t)nb->npages * sizeof *n);
        if (!n) return false;
        v->sel = n;
        v->sel_cap = nb->npages;
    }
    pagesview_untick(v);
    keyboard_close(&v->kb);
    annotate_map_free(&v->book);
    annotate_map_load(nb->id, &v->book);  // (no labels if it is not there or does not read)
    v->open = true;
    int cols, rows, cw, ch, th;
    geom(a, &cols, &rows, &cw, &ch, &th);
    v->screen = current / (cols * rows);
    return true;
}

bool pagesview_pick(PagesView *v, App *a, const NotebookInfo *nb, const Nav *nav, int current, bool had) {
    if (!pagesview_open(v, a, nb, nav, current)) return false;
    if (!nav_index(nav, nb, NULL, 0)) v->index_tab = false;  // an Index with nothing to pick
    v->picking = true;
    v->pick_had = had;
    return true;
}

void pagesview_close(PagesView *v) {
    v->open = false;
    v->picking = v->pick_had = false;
    keyboard_close(&v->kb);
    annotate_map_free(&v->book);
}

void pagesview_free(PagesView *v) {
    annotate_map_free(&v->book);
    free(v->sel);
    v->sel = NULL;
    v->sel_cap = 0;
    v->tab_initialized = false;
    v->nav = NULL;
}

// The book page page pid holds, or 0.
static int book_page(const PagesView *v, uint32_t pid) {
    for (int i = 0; i < v->book.n; i++)
        if (v->book.e[i].pid == pid) return v->book.e[i].page;
    return 0;
}

// Tile i's label: its page number (and that it is the open page), and on a second line (book, ""
// for none) the book page it holds.
static void tile_label(const PagesView *v, int i, char *lab, size_t cap, char *book, size_t bcap) {
    // TRANSLATORS: under each page's picture in the Pages view (436 px at 44 px sideways), the
    // open page in bold.
    snprintf(lab, cap, i == v->current ? _("Page %d (open)") : _("Page %d"), i + 1);
    const char *name = nav_name(v->nav, v->nb->page_ids[i]);
    if (*name) i18n_copy(lab, name, cap);
    int bp = book_page(v, v->nb->page_ids[i]);
    // TRANSLATORS: under a page's label in the Pages view, the page of the book it holds (a page
    // annotated from KOReader): "p. 12". 355 px at 34 px.
    if (bp) snprintf(book, bcap, _("p. %d"), bp);
    else book[0] = 0;
}

// The last page ticked, or -1.
static int last_ticked(const PagesView *v) {
    int last = -1;
    for (int i = 0; i < v->nb->npages && i < v->sel_cap; i++)
        if (v->sel[i]) last = i;
    return last;
}

static int index_rows(App *a) { int rows = (a->h - ui_px(410)) / ui_px(140); return rows > 0 ? rows : 1; }

static void draw_index(PagesView *v, App *a, int total) {
    Canvas *c = &a->screen;
    if (!total) {
        // TRANSLATORS: empty Index tab, body text with nearly the screen's width.
        ui_text(c, FONT_REGULAR, ui_text_size(), ui_small_size(),
                (Rect){ui_px(80), ui_px(350), a->w - ui_px(160), ui_px(200)},
                v->nav->writable ? _("Name a page to list it here.") : _("Page names could not be read."), 3, true, 0);
        return;
    }
    int indices[STORE_MAX_PAGES];
    nav_index(v->nav, v->nb, indices, STORE_MAX_PAGES);
    int per = index_rows(a), section = nav_section_of(v->nav, v->nb, v->current);
    for (int row = 0; row < per && v->index_screen * per + row < total; row++) {
        int i = indices[v->index_screen * per + row];
        Rect r = {ui_px(50), ui_px(240) + row * ui_px(140), a->w - ui_px(100), ui_px(128)};
        if (i == section) canvas_fill(c, (Rect){r.x, r.y + ui_px(12), ui_px(10), r.h - ui_px(24)}, 0);
        char name[160], number[32];
        const char *title = ui_name_label(nav_name(v->nav, v->nb->page_ids[i]), name, sizeof name);
        font_draw_left_name(c, i == section ? FONT_BOLD : FONT_REGULAR, ui_text_size(),
                            (Rect){r.x + ui_px(30), r.y, r.w - ui_px(250), r.h}, title, 0);
        snprintf(number, sizeof number, "%d", i + 1);
        font_draw_centered(c, FONT_BOLD, ui_text_size(),
                           (Rect){r.x + r.w - ui_px(200), r.y, ui_px(180), r.h}, number, 0);
        canvas_hline(c, r.x + ui_px(30), r.w - ui_px(30), r.y + r.h, ui_px(2), 160);
        hits_add(&v->hits, r, PG_OPEN, i);
    }
}

void pagesview_draw(PagesView *v, App *a) {
    Canvas *c = &a->screen;
    const NotebookInfo *nb = v->nb;
    hits_clear(&v->hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(50), th0 = ui_px(190), bh = ui_px(150), by = (th0 - bh) / 2;
    Rect back = {m, by, bh, bh};
    ui_button(c, back, ICON_BACK, NULL, BTN_PLAIN);
    hits_add(&v->hits, back, PG_BACK, 0);
    int k = pagesview_ticked_count(v);
    char del[64];
    // TRANSLATORS: the Pages view's buttons, each under its icon (322 px at 34 px in Gothic, 344 in
    // Classic): Delete, with how many pages are ticked; tick all of them, or none.
    if (k) snprintf(del, sizeof del, _("Delete (%d)"), k);
    else snprintf(del, sizeof del, "%s", _("Delete"));
    Rect rdel = {a->w - m - ui_px(360), by, ui_px(360), bh};
    Rect rall = {rdel.x - ui_px(20) - ui_px(360), by, ui_px(360), bh};
    bool all = k == nb->npages;
    // TRANSLATORS: the Pages view's button that ticks every page, or none (328 px at 34 px in
    // Gothic, 344 in Classic).
    if (v->picking) {
        // Where a link goes: the Index, or (one the page had) nowhere any more.
        Rect to = v->pick_had ? rall : (Rect){rall.x, by, rdel.x + rdel.w - rall.x, bh};
        // TRANSLATORS: while picking where a link drawn on a page goes, the button that sends it to
        // the notebook's Index (the list of named pages): 322 px at 34 px under its icon, or about
        // 740 px when Remove link is not beside it.
        ui_button(c, to, ICON_LINK, _("The Index"), BTN_PLAIN);
        hits_add(&v->hits, to, PG_TO_INDEX, 0);
        if (v->pick_had) {
            // TRANSLATORS: while picking where a link goes, the button that removes the link (the
            // ink drawn around it stays): 322 px at 34 px under its icon.
            ui_button(c, rdel, ICON_TRASH, _("Remove link"), BTN_PLAIN);
            hits_add(&v->hits, rdel, PG_UNLINK, 0);
        }
    } else if (v->index_tab) {
        Rect name = {rall.x, by, rdel.x + rdel.w - rall.x, bh};
        // TRANSLATORS: Index tab header button (about 740 px wide, 44 px text).
        ui_button(c, name, ICON_NONE, *nav_name(v->nav, nb->page_ids[v->current]) ?
                  _("Rename this page") : _("Name this page"), v->nav->writable ? BTN_PLAIN : BTN_DISABLED);
        if (v->nav->writable) hits_add(&v->hits, name, PG_NAME, 0);
    } else {
        ui_button(c, rdel, ICON_TRASH, del, k ? BTN_PRIMARY : BTN_DISABLED);
        if (k) hits_add(&v->hits, rdel, PG_DELETE, 0);
        ui_button(c, rall, ICON_CHECK, all ? _("Select none") : _("Select all"), BTN_PLAIN);
        hits_add(&v->hits, rall, PG_ALL, 0);
    }
    char title[64];
    int tx = back.x + bh + ui_px(30), gap = ui_px(16), tw = (rall.x - tx - ui_px(20) - gap) / 2;
    Rect pages_tab = {tx, by, tw, bh}, index_tab = {tx + tw + gap, by, tw, bh};
    // TRANSLATORS: Pages tab, with the notebook's page count (about 400 px wide, 44 px text).
    snprintf(title, sizeof title, _("Pages (%d)"), nb->npages);
    ui_button(c, pages_tab, ICON_NONE, title, v->index_tab ? BTN_PLAIN : BTN_SELECTED);
    int named = nav_index(v->nav, nb, NULL, 0);
    // TRANSLATORS: Index tab, with the number of named sections (about 400 px, 44 px text).
    snprintf(title, sizeof title, _("Index (%d)"), named);
    ui_button(c, index_tab, ICON_NONE, title, v->index_tab ? BTN_SELECTED : BTN_PLAIN);
    hits_add(&v->hits, pages_tab, PG_TAB, 0);
    hits_add(&v->hits, index_tab, PG_TAB, 1);
    ui_rule(c, 0, a->w, th0, ui_px(4), 0);
    int cols, rows, cw, ch, th;
    geom(a, &cols, &rows, &cw, &ch, &th);
    int per = cols * rows, screens = (nb->npages + per - 1) / per;
    if (v->index_tab) {
        screens = (named + index_rows(a) - 1) / index_rows(a);
        if (screens < 1) screens = 1;
        v->index_screen = clampi(v->index_screen, 0, screens - 1);
        draw_index(v, a, named);
    }
    if (screens < 1) screens = 1;
    if (!v->index_tab) v->screen = clampi(v->screen, 0, screens - 1);
    for (int n = 0; !v->index_tab && n < per; n++) {
        int i = v->screen * per + n;
        if (i >= nb->npages) break;
        Rect tr = {m + (n % cols) * (cw + m), th0 + m + (n / cols) * (ch + m), cw, th};
        char number[24];
        snprintf(number, sizeof number, "%d", i + 1);
        font_draw_centered(c, FONT_BOLD, ui_title_size() * 1.4f, tr, number, 0);
        bool cur = i == v->current, sel = i < v->sel_cap && v->sel[i];
        ui_frame(c, rect_inflate(tr, cur ? ui_px(8) : 0), ui_px(12), 0, false, cur ? ui_px(8) : ui_px(3));
        Rect cb = {tr.x + tr.w - ui_px(96), tr.y + ui_px(16), ui_px(80), ui_px(80)};
        if (!v->picking) {
            ui_frame(c, cb, ui_px(12), sel ? 0 : 255, true, 0);
            ui_frame(c, cb, ui_px(12), 0, false, ui_px(5));
            if (sel) ui_icon(c, ICON_CHECK, cb, 255);
        }
        char lab[NAV_NAME_LEN], book[32];
        tile_label(v, i, lab, sizeof lab, book, sizeof book);
        const char *name = nav_name(v->nav, nb->page_ids[i]);
        if (*name) {
            char display[160];
            font_draw_left_name(c, cur ? FONT_BOLD : FONT_REGULAR, ui_text_size(),
                                (Rect){tr.x, tr.y + th + ui_px(12), tr.w, ui_px(70)},
                                ui_name_label(name, display, sizeof display), 0);
            if (book[0]) font_draw_left(c, FONT_REGULAR, ui_small_size(),
                                      (Rect){tr.x, tr.y + th + ui_px(64), tr.w, ui_px(44)}, book, 0);
        } else if (book[0]) {
            // A book page's: under its label, on a line of its own, the book page it holds.
            ui_text(c, cur ? FONT_BOLD : FONT_REGULAR, ui_text_size(), ui_small_size(),
                    (Rect){tr.x, tr.y + th + ui_px(12), tr.w, ui_px(52)}, lab, 1, true, 0);
            ui_text(c, FONT_REGULAR, ui_small_size(), ui_small_size(), (Rect){tr.x, tr.y + th + ui_px(64), tr.w, ui_px(44)},
                    book, 1, true, 0);
        } else {
            ui_text(c, cur ? FONT_BOLD : FONT_REGULAR, ui_text_size(), ui_small_size(),
                    (Rect){tr.x, tr.y + th + ui_px(12), tr.w, ui_px(70)}, lab, 2, true, 0);
        }
        hits_add(&v->hits, tr, PG_OPEN, i);
        if (!v->picking) hits_add(&v->hits, rect_inflate(cb, ui_px(30)), PG_TOGGLE, i);
    }
    int y = a->h - ui_px(170);
    a->toast_floor = y;  // keep a Copy page notice above the bottom controls when Pages reopens
    canvas_hline(c, 0, a->w, y, ui_px(3), 0);
    Rect pr = {m, y + ui_px(15), ui_px(220), ui_px(140)}, nr = {a->w - m - ui_px(220), y + ui_px(15), ui_px(220), ui_px(140)};
    int screen = v->index_tab ? v->index_screen : v->screen;
    ui_button(c, pr, ICON_PREV, NULL, screen > 0 ? BTN_PLAIN : BTN_DISABLED);
    ui_button(c, nr, ICON_NEXT, NULL, screen < screens - 1 ? BTN_PLAIN : BTN_DISABLED);
    if (screen > 0) hits_add(&v->hits, pr, PG_PREV, 0);
    if (screen < screens - 1) hits_add(&v->hits, nr, PG_NEXT, 0);
    Rect jump = {a->w / 2 - ui_px(300), pr.y, ui_px(600), pr.h};
    ui_button(c, jump, ICON_NONE, _("Go to page..."), BTN_PLAIN);
    hits_add(&v->hits, jump, PG_JUMP, 0);
    // New page before Go to page..., Copy page after it, each up to 400 px of the room beside it.
    int g = ui_px(20), bw = jump.x - g - (pr.x + pr.w + g);
    if (bw > ui_px(400)) bw = ui_px(400);
    Rect rnew = {jump.x - g - bw, pr.y, bw, pr.h}, rcopy = {jump.x + jump.w + g, pr.y, bw, pr.h};
    // TRANSLATORS: the Pages view's bottom buttons, each under its icon (290 px at 34 px): a new
    // blank page after the open page (or after the last page ticked), and a copy of the one page
    // ticked, right after it.
    if (!v->picking) {
        ui_button(c, rnew, ICON_PLUS, _("New page"), BTN_PLAIN);
        hits_add(&v->hits, rnew, PG_NEW, 0);
        if (v->index_tab) {
            // In the Index, where nothing is ticked: a link drawn on the open page.
            bool can = v->nav->links_writable;
            // TRANSLATORS: the Index tab's bottom button, under its icon (290 px at 34 px): draw a
            // link on the open page to another page (or to the Index).
            ui_button(c, rcopy, ICON_LINK, _("Add a link"), can ? BTN_PLAIN : BTN_DISABLED);
            if (can) hits_add(&v->hits, rcopy, PG_LINK, 0);
        } else {
            ui_button(c, rcopy, ICON_COPY, _("Copy page"), k == 1 ? BTN_PLAIN : BTN_DISABLED);
            if (k == 1) hits_add(&v->hits, rcopy, PG_COPY, 0);
        }
    }
    if (v->kb.open) keyboard_draw(&v->kb, c, (Rect){0, 0, a->w, a->h});
}

// The whole view drawn again and refreshed (what the editor's compose_all does while it is open).
static void redraw(PagesView *v, App *a, RefreshMode m) {
    pagesview_draw(v, a);
    app_invalidate_all(a, m);
}

PvResult pagesview_pointer(PagesView *v, App *a, int ph, const InEvent *ev) {
    if (v->kb.open) {
        Rect dirty;
        KbResult r = keyboard_pointer(&v->kb, ph, ev->x, ev->y, ev->t_ms, &dirty);
        if (r == KB_DONE) {
            if (v->naming) return PV_NAME;
            char *end;
            long page = strtol(v->kb.text, &end, 10);
            if (*v->kb.text && !*end && page >= 1 && page <= v->nb->npages) {
                v->target = (int)page - 1;
                return v->picking ? PV_PICK : PV_OPEN;
            }
            v->kb.open = true;
            app_toast(a, _("Enter a page number in this notebook."), 2500);
        }
        if (r == KB_CANCEL) redraw(v, a, REFRESH_UI);
        else {
            keyboard_draw(&v->kb, &a->screen, (Rect){0, 0, a->w, a->h});
            app_invalidate(a, rect_union(dirty, v->kb.field_dirty), REFRESH_FAST);
        }
        return PV_NONE;
    }
    if (ph == 0) {
        press_begin(&v->press, ev->x, ev->y, ev->t_ms);
        return PV_NONE;
    }
    press_move(&v->press, ev->x, ev->y);
    if (ph == 1) return PV_NONE;
    const Hit *h = hits_find(&v->hits, v->press.x0, v->press.y0);
    if (!h || !press_is_tap(&v->press, h->r)) return PV_NONE;
    Hit hit = *h;
    switch (hit.id) {
    case PG_TAB:
        v->index_tab = hit.arg != 0;
        if (v->index_tab) pagesview_untick(v);
        redraw(v, a, REFRESH_FULL);
        break;
    case PG_NAME:
        v->naming = true;
        // TRANSLATORS: the naming keyboard's title, 96 px text-field header.
        keyboard_open(&v->kb, _("Page name"), nav_name(v->nav, v->nb->page_ids[v->current]));
        redraw(v, a, REFRESH_UI);
        break;
    case PG_JUMP:
        v->naming = false;
        keyboard_open(&v->kb, _("Go to page"), "");
        v->kb.page = 2;
        redraw(v, a, REFRESH_UI);
        break;
    case PG_BACK: return PV_CLOSE;
    case PG_OPEN:
        v->target = hit.arg;
        return v->picking ? PV_PICK : PV_OPEN;
    case PG_LINK: return PV_LINK;
    case PG_TO_INDEX: return PV_PICK_INDEX;
    case PG_UNLINK: return PV_UNLINK;
    case PG_TOGGLE:
        if (hit.arg < v->sel_cap) v->sel[hit.arg] = !v->sel[hit.arg];
        redraw(v, a, REFRESH_UI);
        break;
    case PG_ALL: {
        bool all = pagesview_ticked_count(v) == v->nb->npages;
        for (int i = 0; i < v->nb->npages && i < v->sel_cap; i++) v->sel[i] = !all;
        redraw(v, a, REFRESH_UI);
        break;
    }
    case PG_DELETE: return PV_DELETE;
    case PG_NEW: {
        int after = last_ticked(v);
        v->target = (after >= 0 ? after : v->current) + 1;
        return PV_NEW;
    }
    case PG_COPY:
        if (pagesview_ticked_count(v) != 1) break;
        v->target = last_ticked(v);
        return PV_COPY;
    case PG_PREV:
    case PG_NEXT:
        if (v->index_tab) v->index_screen += hit.id == PG_NEXT ? 1 : -1;
        else v->screen += hit.id == PG_NEXT ? 1 : -1;
        redraw(v, a, REFRESH_FULL);
        break;
    default: break;
    }
    return PV_NONE;
}

int pagesview_probe(const PagesView *v, const char *key, char *out, size_t cap) {
    out[0] = 0;
    if (!strcmp(key, "pages_tab")) {
        snprintf(out, cap, "%s", v->index_tab ? "index" : "pages");
    } else if (!strcmp(key, "picking")) {
        snprintf(out, cap, "%s", !v->open || !v->picking ? "" : v->pick_had ? "had" : "new");
    } else if (!strcmp(key, "index_rows")) {
        snprintf(out, cap, "%d", v->nav ? nav_index(v->nav, v->nb, NULL, 0) : 0);
    } else if (!strncmp(key, "tile.", 5)) {
        int i = atoi(key + 5) - 1;
        char lab[NAV_NAME_LEN], book[32];
        if (!v->open || i < 0 || i >= v->nb->npages) return 0;
        tile_label(v, i, lab, sizeof lab, book, sizeof book);
        snprintf(out, cap, "%s%s%s", lab, book[0] ? " | " : "", book);
    } else {
        return -1;
    }
    return 0;
}
