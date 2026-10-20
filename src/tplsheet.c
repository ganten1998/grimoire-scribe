// tplsheet.c: the Templates sheet, and the layout the paper pickers share (see tplsheet.h).
#include "tplsheet.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bgjob.h"

#define CHECK_MS 1500  // how often a sheet waiting for previews looks for them, while a job runs

// ---- the paper pickers' layout ----

PaperGrid paper_grid(int n, int avail_w, int avail_h, int gap, int label_h, int max_w, int min_cols) {
    PaperGrid best = {0};
    int best_empty = 0;
    if (n < 1) n = 1;
    for (int cols = clampi(min_cols, 1, n); cols <= n; cols++) {
        int rows = (n + cols - 1) / cols;
        int tw = (avail_w - (cols - 1) * gap) / cols;
        if (tw > max_w) tw = max_w;
        int max_th = avail_h / rows - label_h;  // a tile's height, its label under it
        if (tw * 4 / 3 > max_th) tw = max_th * 3 / 4;
        int empty = rows * cols - n;
        if (tw < 1 || (best.tw && (tw < best.tw || (tw == best.tw && empty >= best_empty)))) continue;
        best = (PaperGrid){cols, rows, tw, tw * 4 / 3, tw * 4 / 3 + label_h};
        best_empty = empty;
    }
    if (!best.tw) best = (PaperGrid){n, 1, 1, 1, 1 + label_h};  // (a box with no room at all)
    return best;
}

Rect paper_sheet_rect(const App *a, PaperGrid *g) {
    int m = ui_px(70), gap = ui_px(30);
    // Beside the tiles: the title (120), the sheet's insets (72), its buttons (190), a margin (80).
    int avail_w = a->w - 2 * m - ui_px(72), avail_h = a->h - ui_px(120) - ui_px(72) - ui_px(190) - ui_px(80);
    *g = paper_grid(PAPER_SHEET_TILES, avail_w, avail_h, gap, ui_px(100), a->w / 6, PAPER_GRID_MIN_COLS);
    int sheet_h = ui_px(120) + ui_px(72) + g->rows * g->cell_h + ui_px(190);
    return (Rect){m, (a->h - sheet_h) / 3, a->w - 2 * m, sheet_h};
}

void paper_tile_label(Canvas *c, Rect tr, int gap, const char *label, bool selected) {
    FontFace lf = selected ? FONT_BOLD : FONT_REGULAR;
    float ls = ui_small_size() * 0.9f;
    // By its regular width, so that a label keeps its lines when its tile is selected (in bold).
    if (font_measure(FONT_REGULAR, ls, label) > tr.w)
        font_draw_wrapped(c, lf, ls, (Rect){tr.x, tr.y + tr.h + ui_px(8), tr.w, ui_px(80)}, label, 2, ui_px(40), true, 0);
    else
        font_draw_centered(c, lf, ls, (Rect){tr.x - gap / 2, tr.y + tr.h + ui_px(16), tr.w + gap, ui_px(64)}, label, 0);
}

void paper_tile_more(Canvas *c, Rect tr) {
    canvas_fill(c, tr, 255);
    int s = tr.w * 2 / 5;
    ui_icon(c, ICON_MORE, (Rect){tr.x + (tr.w - s) / 2, tr.y + (tr.h - s) / 2, s, s}, 0);
}

// ---- the sheet ----

// The templates the sheet offers as it opens (every one not bad), and whether any of them waits for
// a preview.
static void rebuild_shown(TplSheet *s) {
    s->n = 0;
    s->pending = false;
    for (int i = 0; i < s->nt && s->shown; i++) {
        if (s->t[i].bad) continue;
        s->shown[s->n++] = i;
        if (!s->t[i].ready) s->pending = true;
    }
}

// A shown template the previews job found it cannot use since the sheet opened keeps its tile, so
// that no tile moves under the pen, until the sheet opens again. How many it offers: the others.
static int usable(const TplSheet *s) {
    int u = 0;
    for (int k = 0; k < s->n; k++) u += !s->t[s->shown[k]].bad;
    return u;
}

// Whether one of those still waits for its preview.
static bool waiting(const TplSheet *s) {
    for (int k = 0; k < s->n; k++)
        if (!s->t[s->shown[k]].bad && !s->t[s->shown[k]].ready) return true;
    return false;
}

// The offered template that is the paper now (an index into shown), or -1.
static int find_selected(TplSheet *s) {
    bool by_paper = !s->current_path[0] && s->nb && paper_is_image(s->current) && !paper_is_document(s->current);
    for (int k = 0; k < s->n; k++) {
        const Template *t = &s->t[s->shown[k]];
        PaperKind kind;
        if (s->current_path[0] ? !strcmp(t->path, s->current_path)
                               : by_paper && template_paper(t, s->nb, &s->ix, &kind) == 0 && kind == s->current)
            return k;
    }
    return -1;
}

void tplsheet_close(TplSheet *s) {
    template_list_free(s->t, s->nt);
    free(s->shown);
    bgindex_free(&s->ix);
    s->t = NULL;
    s->shown = NULL;
    s->nt = s->n = 0;
    s->open = false;
    s->selected = s->picked = -1;
}

void tplsheet_open(TplSheet *s, App *a, const NotebookInfo *nb, PaperKind current, const char *current_path) {
    tplsheet_close(s);
    *s = (TplSheet){.open = true, .nb = nb, .current = current, .selected = -1, .picked = -1, .gen = bgimage_generation()};
    if (current_path) strlcpy_q(s->current_path, current_path, sizeof s->current_path);
    if (template_scan(&s->t, &s->nt, a->dpi) != 0) {
        s->t = NULL;  // (no templates folder: nothing to offer)
        s->nt = 0;
    }
    s->shown = malloc((size_t)(s->nt ? s->nt : 1) * sizeof *s->shown);
    if (!s->shown) {  // (no memory: nothing offered, rather than every file called unusable)
        template_list_free(s->t, s->nt);
        s->t = NULL;
        s->nt = 0;
    }
    // In the editor, which template the page's paper is comes from the notebook's index of its
    // pictures (the page's paper is in the notebook, its file in the templates folder).
    char bgdir[700];
    if (nb && paper_is_image(current) && bgimage_dir(nb->id, bgdir, sizeof bgdir) == 0 && bgindex_load(bgdir, &s->ix) != 0)
        qlog("templates: %s: bg/index.ini does not read; no template shows as this page's", nb->id);
    rebuild_shown(s);
    s->selected = find_selected(s);
    s->anchor = s->selected > 0 ? s->selected : 0;
    s->next_check = now_ms() + CHECK_MS;
}

// A template's name under its tile, on one line no wider than the tile: centred, or cut short as
// names are (the fit check does not hear of a name, which is the owner's).
static void tile_name(Canvas *c, Rect tr, const char *name, bool selected) {
    FontFace f = selected && ui_theme() != THEME_GOTHIC ? FONT_BOLD : FONT_REGULAR;  // (as the cards' names)
    float px = ui_small_size() * 0.9f;
    Rect lr = {tr.x, tr.y + tr.h + ui_px(16), tr.w, ui_px(64)};
    int w = font_measure(f, px, name);
    if (w < lr.w) lr = (Rect){lr.x + (lr.w - w) / 2, lr.y, w + 1, lr.h};
    font_draw_left_name(c, f, px, lr, name, 0);
}

// A tile whose preview is not made yet: the sign for more, in gray, until it is.
static void tile_waiting(Canvas *c, Rect tr) {
    int s = tr.w / 4;
    ui_icon(c, ICON_MORE, (Rect){tr.x + (tr.w - s) / 2, tr.y + (tr.h - s) / 2, s, s}, 170);
}

// A tile whose template the previews job found it cannot use since the sheet opened: a cross, in
// gray. It cannot be picked, and is gone when the sheet opens again.
static void tile_unusable(Canvas *c, Rect tr) {
    int s = tr.w / 4;
    ui_icon(c, ICON_CLOSE, (Rect){tr.x + (tr.w - s) / 2, tr.y + (tr.h - s) / 2, s, s}, 170);
}

// A paragraph of the empty state at y, centred, on the lines it takes (at most max, line px
// apart): the y under it, so that every paragraph starts as far under the one before it.
static int paragraph(Canvas *c, float px, Rect w, int y, const char *text, int max, int line) {
    int n = clampi(font_wrap_lines(FONT_REGULAR, px, text, w.w, NULL), 1, max);
    font_draw_wrapped(c, FONT_REGULAR, px, (Rect){w.x, y, w.w, n * line}, text, max, line, true, 0);
    return y + n * line;
}

// No template to offer: how to add some.
static void draw_empty(TplSheet *s, App *a, Rect r) {
    Canvas *c = &a->screen;
    int pad = ui_px(40), y = r.y + ui_px(40);
    Rect w = {r.x + pad, 0, r.w - 2 * pad, 0};
    // TRANSLATORS: the heading of the Templates sheet when grimoire/templates has no picture to
    // offer (1560 px at 64 px in bold, sideways 2180).
    font_draw_centered(c, FONT_BOLD, ui_title_size(), (Rect){w.x, y, w.w, ui_px(100)}, _("No templates yet"), 0);
    y += ui_px(150);
    char text[640];
    // TRANSLATORS: under it, what to do, centred (up to 4 lines of 1560 px at 44 px, sideways
    // 2180); %s is the folder on the Kindle's storage.
    snprintf(text, sizeof text, _("Copy PNG or JPG pictures into the %s folder over USB while Grimoire is closed."),
             "grimoire/templates");
    float ts = ui_text_size();
    int line = ui_px(62);
    y = paragraph(c, ts, w, y, text, 4, line) + ui_px(40);
    // TRANSLATORS: the paragraph after it, centred too (up to 4 lines of 1560 px at 44 px, sideways
    // 2180); README.txt is the file Grimoire wrote in that folder.
    const char *more = _("Upright pictures in the shape of a page (3:4) work best, ideally 1860 x 2480 pixels. The "
                         "README.txt there says more.");
    y = paragraph(c, ts, w, y, more, 4, line) + ui_px(40);
    int bad = 0;
    for (int i = 0; i < s->nt; i++) bad += s->t[i].bad;
    if (bad) {
        // TRANSLATORS: under them, when the folder holds files Grimoire cannot use (a picture that
        // does not read, or too big): how many; %s is the log file, which says which (by number). Up
        // to 2 lines of 1560 px at 34 px, sideways 2180.
        snprintf(text, sizeof text, NG_("Grimoire cannot use %d file there (see %s).",
                                        "Grimoire cannot use %d files there (see %s).", bad),
                 bad, "grimoire.log");
        paragraph(c, ui_small_size(), w, y, text, 2, line);
    }
}

Rect tplsheet_draw(TplSheet *s, App *a) {
    Canvas *c = &a->screen;
    hits_clear(&s->hits);
    PaperGrid g;
    s->rect = paper_sheet_rect(a, &g);
    // TRANSLATORS: the title of the sheet of templates (pictures copied into grimoire/templates, to
    // write on as paper), which More papers... opens (1560 px at 64 px).
    Rect in = ui_sheet(c, s->rect, _("Templates"));
    int gap = ui_px(30), bh = ui_px(140), offered = usable(s);
    s->per = g.cols * g.rows;
    s->pages = offered ? (s->n + s->per - 1) / s->per : 1;
    s->page = clampi(s->anchor / s->per, 0, s->pages - 1);
    Rect back = {in.x, in.y + in.h - bh, ui_px(380), bh};
    // TRANSLATORS: the Templates sheet's button back to the papers, under its icon (348 px at
    // 34 px).
    ui_button(c, back, ICON_BACK, _("Back"), BTN_PLAIN);
    hits_add(&s->hits, back, TS_HIT_BACK, 0);
    // Only the previews of the tiles on screen stay in memory (each about 130 KB).
    int first = s->page * s->per;
    for (int k = 0; k < s->n; k++)
        if (k < first || k >= first + s->per) canvas_free(&s->t[s->shown[k]].thumb);
    if (!offered) {
        draw_empty(s, a, (Rect){in.x, in.y, in.w, back.y - in.y});
        return s->rect;
    }
    int gridw = g.cols * g.tw + (g.cols - 1) * gap;
    for (int k = 0; k < s->per && first + k < s->n; k++) {
        int i = first + k;
        Template *t = &s->t[s->shown[i]];
        Rect tr = {in.x + (in.w - gridw) / 2 + (k % g.cols) * (g.tw + gap), in.y + (k / g.cols) * g.cell_h, g.tw, g.th};
        canvas_fill(c, tr, 255);
        const Canvas *pv = t->bad ? NULL : template_thumb(t);
        if (pv) {
            canvas_blit_fit(c, pv, tr);
        } else if (t->bad) {
            tile_unusable(c, tr);
        } else {
            tile_waiting(c, tr);
            s->pending = true;  // (also a preview gone since the scan: made again, then shown)
        }
        bool sel = i == s->selected;
        ui_frame(c, rect_inflate(tr, sel ? ui_px(10) : 0), ui_px(12), 0, false, sel ? ui_px(9) : ui_px(3));
        tile_name(c, tr, t->name, sel);
        if (pv) hits_add(&s->hits, (Rect){tr.x, tr.y, tr.w, tr.h + ui_px(90)}, TS_HIT_TILE, i);  // (none waiting, unusable)
    }
    if (s->pages > 1) {
        Rect nr = {in.x + in.w - ui_px(220), back.y, ui_px(220), bh};
        Rect lr = {nr.x - ui_px(440), back.y, ui_px(440), bh};
        Rect pr = {lr.x - ui_px(220), back.y, ui_px(220), bh};
        ui_button(c, pr, ICON_PREV, NULL, s->page > 0 ? BTN_PLAIN : BTN_DISABLED);
        ui_button(c, nr, ICON_NEXT, NULL, s->page < s->pages - 1 ? BTN_PLAIN : BTN_DISABLED);
        if (s->page > 0) hits_add(&s->hits, pr, TS_HIT_PREV, 0);
        if (s->page < s->pages - 1) hits_add(&s->hits, nr, TS_HIT_NEXT, 0);
        char pl[96];
        // TRANSLATORS: (also) between the Templates sheet's page buttons, in bold: 440 px at 44 px.
        snprintf(pl, sizeof pl, _("Page %d of %d"), s->page + 1, s->pages);
        font_draw_centered(c, FONT_BOLD, ui_text_size(), lr, pl, 0);
    }
    return s->rect;
}

TsResult tplsheet_pointer(TplSheet *s, App *a, int ph, float x, float y, uint32_t t, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!s->open) return TS_NONE;
    if (ph == 0) {
        press_begin(&s->press, x, y, t);
        return TS_NONE;
    }
    press_move(&s->press, x, y);
    if (ph != 2) return TS_NONE;
    if (!rect_contains(s->rect, (int)s->press.x0, (int)s->press.y0)) {
        *dirty = s->rect;  // a tap outside the sheet cancels
        tplsheet_close(s);
        return TS_CANCEL;
    }
    const Hit *h = hits_find(&s->hits, s->press.x0, s->press.y0);
    if (!h || !press_is_tap(&s->press, h->r)) return TS_NONE;
    switch (h->id) {
    case TS_HIT_BACK:
        *dirty = s->rect;
        tplsheet_close(s);
        return TS_BACK;
    case TS_HIT_TILE:
        if (h->arg < 0 || h->arg >= s->n) return TS_NONE;
        s->picked = h->arg;
        s->open = false;  // (the list stays for tplsheet_path until tplsheet_close)
        *dirty = s->rect;
        return TS_PICKED;
    case TS_HIT_PREV:
    case TS_HIT_NEXT:
        s->anchor = clampi(s->page + (h->id == TS_HIT_NEXT ? 1 : -1), 0, s->pages - 1) * s->per;
        tplsheet_draw(s, a);
        *dirty = s->rect;
        return TS_CHANGED;
    default: return TS_NONE;
    }
}

bool tplsheet_tick(TplSheet *s, App *a, uint32_t now, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!s->open || !s->pending) return false;
    // A background job that ended (the previews job among them) says so by bgimage_generation;
    // while one runs, the previews it made so far are looked for now and then.
    bool ended = s->gen != bgimage_generation();
    if (!ended && !(bg_busy() && (int32_t)(now - s->next_check) >= 0)) return false;
    s->gen = bgimage_generation();
    s->next_check = now + CHECK_MS;
    if (template_refresh(s->t, s->nt) == 0) return false;
    s->pending = waiting(s);  // (a template found unusable keeps its tile: usable)
    s->selected = find_selected(s);
    tplsheet_draw(s, a);
    *dirty = s->rect;
    return true;
}

const char *tplsheet_path(const TplSheet *s) {
    return s->t && s->picked >= 0 && s->picked < s->n ? s->t[s->shown[s->picked]].path : NULL;
}

const Canvas *tplsheet_preview(TplSheet *s) {
    return s->t && s->picked >= 0 && s->picked < s->n ? template_thumb(&s->t[s->shown[s->picked]]) : NULL;
}

int tplsheet_probe(const TplSheet *s, const char *key, char *out, size_t cap) {
    bool open = s && s->open;
    int ready = 0;
    for (int k = 0; open && k < s->n; k++) ready += s->t[s->shown[k]].ready;
    if (!strcmp(key, "count")) {
        if (open) snprintf(out, cap, "%d", usable(s));
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "ready")) {
        if (open) snprintf(out, cap, "%d", ready);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "page")) {
        if (open) snprintf(out, cap, "%d", s->page + 1);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "pages")) {
        if (open) snprintf(out, cap, "%d", s->pages);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "selected")) {
        snprintf(out, cap, "%s", open && s->selected >= 0 ? s->t[s->shown[s->selected]].name : "");
    } else {
        return -1;
    }
    return 0;
}
