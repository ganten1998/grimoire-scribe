// pagesview.c: the editor's Pages view (pagesview.h): numbered tiles; never render other pages on
// this screen.
#include "pagesview.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { PG_BACK = 1, PG_DELETE, PG_ALL, PG_TOGGLE, PG_OPEN, PG_PREV, PG_NEXT, PG_JUMP };

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

bool pagesview_open(PagesView *v, App *a, const NotebookInfo *nb, int current) {
    v->nb = nb;
    v->current = current;
    if (v->sel_cap < nb->npages) {
        bool *n = realloc(v->sel, (size_t)nb->npages * sizeof *n);
        if (!n) return false;
        v->sel = n;
        v->sel_cap = nb->npages;
    }
    pagesview_untick(v);
    keyboard_close(&v->kb);
    v->open = true;
    int cols, rows, cw, ch, th;
    geom(a, &cols, &rows, &cw, &ch, &th);
    v->screen = current / (cols * rows);
    return true;
}

void pagesview_close(PagesView *v) {
    v->open = false;
    keyboard_close(&v->kb);
}

void pagesview_free(PagesView *v) {
    free(v->sel);
    v->sel = NULL;
    v->sel_cap = 0;
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
    ui_button(c, rdel, ICON_TRASH, del, k ? BTN_PRIMARY : BTN_DISABLED);
    if (k) hits_add(&v->hits, rdel, PG_DELETE, 0);
    Rect rall = {rdel.x - ui_px(20) - ui_px(360), by, ui_px(360), bh};
    bool all = k == nb->npages;
    // TRANSLATORS: the Pages view's button that ticks every page, or none (328 px at 34 px in
    // Gothic, 344 in Classic).
    ui_button(c, rall, ICON_CHECK, all ? _("Select none") : _("Select all"), BTN_PLAIN);
    hits_add(&v->hits, rall, PG_ALL, 0);
    char title[64];
    // TRANSLATORS: the Pages view's title, with the notebook's pages (820 px at 74 px).
    snprintf(title, sizeof title, _("Pages (%d)"), nb->npages);
    font_draw_left(c, FONT_BOLD, ui_title_size() * 1.15f,
                   (Rect){back.x + bh + ui_px(30), 0, rall.x - back.x - bh - ui_px(50), th0}, title, 0);
    ui_rule(c, 0, a->w, th0, ui_px(4), 0);
    int cols, rows, cw, ch, th;
    geom(a, &cols, &rows, &cw, &ch, &th);
    int per = cols * rows, screens = (nb->npages + per - 1) / per;
    if (screens < 1) screens = 1;
    v->screen = clampi(v->screen, 0, screens - 1);
    for (int n = 0; n < per; n++) {
        int i = v->screen * per + n;
        if (i >= nb->npages) break;
        Rect tr = {m + (n % cols) * (cw + m), th0 + m + (n / cols) * (ch + m), cw, th};
        char number[24];
        snprintf(number, sizeof number, "%d", i + 1);
        font_draw_centered(c, FONT_BOLD, ui_title_size() * 1.4f, tr, number, 0);
        bool cur = i == v->current, sel = i < v->sel_cap && v->sel[i];
        ui_frame(c, rect_inflate(tr, cur ? ui_px(8) : 0), ui_px(12), 0, false, cur ? ui_px(8) : ui_px(3));
        Rect cb = {tr.x + tr.w - ui_px(96), tr.y + ui_px(16), ui_px(80), ui_px(80)};
        ui_frame(c, cb, ui_px(12), sel ? 0 : 255, true, 0);
        ui_frame(c, cb, ui_px(12), 0, false, ui_px(5));
        if (sel) ui_icon(c, ICON_CHECK, cb, 255);
        char lab[64];
        // TRANSLATORS: under each page's picture in the Pages view (436 px at 44 px sideways), the
        // open page in bold.
        snprintf(lab, sizeof lab, cur ? _("Page %d (open)") : _("Page %d"), i + 1);
        ui_text(c, cur ? FONT_BOLD : FONT_REGULAR, ui_text_size(), ui_small_size(),
                (Rect){tr.x, tr.y + th + ui_px(12), tr.w, ui_px(70)}, lab, 2, true, 0);
        hits_add(&v->hits, tr, PG_OPEN, i);
        hits_add(&v->hits, rect_inflate(cb, ui_px(30)), PG_TOGGLE, i);
    }
    int y = a->h - ui_px(170);
    canvas_hline(c, 0, a->w, y, ui_px(3), 0);
    Rect pr = {m, y + ui_px(15), ui_px(220), ui_px(140)}, nr = {a->w - m - ui_px(220), y + ui_px(15), ui_px(220), ui_px(140)};
    ui_button(c, pr, ICON_PREV, NULL, v->screen > 0 ? BTN_PLAIN : BTN_DISABLED);
    ui_button(c, nr, ICON_NEXT, NULL, v->screen < screens - 1 ? BTN_PLAIN : BTN_DISABLED);
    if (v->screen > 0) hits_add(&v->hits, pr, PG_PREV, 0);
    if (v->screen < screens - 1) hits_add(&v->hits, nr, PG_NEXT, 0);
    Rect jump = {a->w / 2 - ui_px(300), pr.y, ui_px(600), pr.h};
    ui_button(c, jump, ICON_NONE, _("Go to page..."), BTN_PLAIN);
    hits_add(&v->hits, jump, PG_JUMP, 0);
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
            char *end;
            long page = strtol(v->kb.text, &end, 10);
            if (*v->kb.text && !*end && page >= 1 && page <= v->nb->npages) {
                v->target = (int)page - 1;
                return PV_OPEN;
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
    case PG_JUMP:
        keyboard_open(&v->kb, _("Go to page"), "");
        v->kb.page = 2;
        redraw(v, a, REFRESH_UI);
        break;
    case PG_BACK: return PV_CLOSE;
    case PG_OPEN:
        v->target = hit.arg;
        return PV_OPEN;
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
    case PG_PREV:
    case PG_NEXT:
        v->screen += hit.id == PG_NEXT ? 1 : -1;
        redraw(v, a, REFRESH_FULL);
        break;
    default: break;
    }
    return PV_NONE;
}
