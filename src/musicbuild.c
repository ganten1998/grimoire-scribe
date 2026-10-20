// musicbuild.c: the Custom music sheet (see musicbuild.h).
#include "musicbuild.h"

#include <math.h>
#include <stdio.h>

#include "store.h"

enum { B_UP = 1, B_DOWN, B_REMOVE, B_ADD, B_LESS, B_MORE, B_CANCEL, B_DONE };

// The order parts are offered in the "Add" grid.
static const MusicPart k_parts[] = {PART_TREBLE, PART_BASS,  PART_ALTO,  PART_TENOR,
                                    PART_PERC,   PART_STAFF, PART_LYRICS, PART_TAB4,
                                    PART_TAB5,   PART_TAB6,  PART_TAB7};
#define N_PARTS ((int)(sizeof k_parts / sizeof k_parts[0]))
#define ADD_COLS 3
#define ADD_ROWS ((N_PARTS + ADD_COLS - 1) / ADD_COLS)
// Fixed rows: heading, gap, heading, gap; plus title 120, padding 72, buttons 190.
#define FIXED_H (120 + 72 + 70 + 20 + 70 + 20 + 190)
#define ROWS (MUSIC_MAX_PARTS + ADD_ROWS + 1)

static int row_h(App *a) { return clampi((a->h - ui_px(80) - ui_px(FIXED_H)) / ROWS, ui_px(80), ui_px(115)); }

static Rect card_rect(App *a) {
    int w = a->w - 2 * ui_px(60);
    if (w > ui_px(1740)) w = ui_px(1740);
    int h = ui_px(FIXED_H) + ROWS * row_h(a);
    return (Rect){(a->w - w) / 2, (a->h - h) / 3, w, h};
}

// The most systems of the layout that fit on the page it is for.
static int fit(const MusicBuild *b, App *a) { return music_max_systems(&b->m, b->page_h, a->dpi); }

static void clamp_systems(MusicBuild *b, App *a) {
    int mx = fit(b, a);
    b->m.systems = clampi(b->m.systems, 1, mx);
}

void musicbuild_open_page(MusicBuild *b, App *a, PaperKind start, int page_w, int page_h) {
    canvas_free(&b->prev);
    *b = (MusicBuild){.open = true, .page_w = page_w, .page_h = page_h};
    if (!music_decode(start, &b->m) && !music_decode(a->settings.music_layout, &b->m)) music_decode(music_default(), &b->m);
    clamp_systems(b, a);
    b->rect = card_rect(a);
}

void musicbuild_open(MusicBuild *b, App *a, PaperKind start) {
    int pw, ph;
    store_page_size(&pw, &ph);
    musicbuild_open_page(b, a, start, pw, ph);
}

void musicbuild_free(MusicBuild *b) {
    canvas_free(&b->prev);
    b->open = false;
}

PaperKind musicbuild_result(const MusicBuild *b) { return music_encode(&b->m); }

static const Canvas *preview(MusicBuild *b, App *a) {
    PaperKind k = music_encode(&b->m);
    if (b->prev.px && b->prev_k == k) return &b->prev;
    canvas_free(&b->prev);
    int pw = b->page_w, ph = b->page_h;
    Canvas full;
    if (canvas_init(&full, pw, ph) != 0) return NULL;
    paper_render(&full, k, (Rect){0, 0, pw, ph}, a->dpi);
    if (canvas_init(&b->prev, pw / 4, ph / 4) == 0) canvas_downsample_keep_lines(&b->prev, &full, 0.6f);
    canvas_free(&full);
    b->prev_k = k;
    return b->prev.px ? &b->prev : NULL;
}

Rect musicbuild_draw(MusicBuild *b, App *a) {
    Canvas *c = &a->screen;
    hits_clear(&b->hits);
    b->rect = card_rect(a);
    // TRANSLATORS: the title of the sheet that builds a music paper from parts.
    Rect in = ui_sheet(c, b->rect, _("Custom music"));
    int rh = row_h(a), pad = ui_px(12);
    int lw = in.w * 58 / 100;
    float ts = ui_text_size();
    int y = in.y;
    // TRANSLATORS: the Custom music sheet's headings, over the layout's parts and over the parts to
    // add (962 px at 44 px).
    font_draw_left(c, FONT_BOLD, ts, (Rect){in.x, y, lw, ui_px(70)}, _("Each system, top to bottom"), 0);
    y += ui_px(70);
    for (int i = 0; i < MUSIC_MAX_PARTS; i++, y += rh) {
        Rect row = {in.x, y + pad / 2, lw, rh - pad};
        if (i >= b->m.n) {
            if (i == b->m.n)
                font_draw_left(c, FONT_REGULAR, ui_small_size(), rect_inflate(row, -ui_px(20)), _("(add a part below)"), 150);
            continue;
        }
        ui_frame(c, row, ui_px(12), 0, false, ui_px(3));
        char lab[96];
        snprintf(lab, sizeof lab, "%d  %s", i + 1, part_label(b->m.part[i]));
        font_draw_left(c, FONT_REGULAR, ts, (Rect){row.x + ui_px(24), row.y, row.w, row.h}, lab, 0);
        int bs = row.h - 2 * pad, bx = row.x + row.w - pad - bs;
        struct { int id; IconKind ic; bool ok; } btn[3] = {
            {B_REMOVE, ICON_CLOSE, b->m.n > 1}, {B_DOWN, ICON_DOWN, i + 1 < b->m.n}, {B_UP, ICON_UP, i > 0}};
        for (int k = 0; k < 3; k++, bx -= bs + pad) {
            Rect br = {bx, row.y + pad, bs, bs};
            ui_button(c, br, btn[k].ic, NULL, btn[k].ok ? BTN_PLAIN : BTN_DISABLED);
            if (btn[k].ok) hits_add(&b->hits, rect_inflate(br, pad / 2), btn[k].id, i);
        }
    }
    y += ui_px(20);
    font_draw_left(c, FONT_BOLD, ts, (Rect){in.x, y, lw, ui_px(70)}, _("Add a part"), 0);
    y += ui_px(70);
    bool full = b->m.n >= MUSIC_MAX_PARTS;
    int bw = (lw - (ADD_COLS - 1) * pad) / ADD_COLS;
    // One size for every part's name, the smallest any of them needs (a longer translation takes a
    // smaller size, or two lines): a grid of sizes would read as parts of different weight.
    float ps = ui_text_size();
    for (int i = 0; i < N_PARTS; i++)
        ps = fminf(ps, ui_button_text_px((Rect){0, 0, bw, rh - pad}, part_label(k_parts[i]), BTN_PLAIN));
    for (int i = 0; i < N_PARTS; i++) {
        Rect r = {in.x + (i % ADD_COLS) * (bw + pad), y + (i / ADD_COLS) * rh + pad / 2, bw, rh - pad};
        ui_button_text_at(c, r, part_label(k_parts[i]), full ? BTN_DISABLED : BTN_PLAIN, ps);
        if (!full) hits_add(&b->hits, r, B_ADD, (int)k_parts[i]);
    }
    y += ADD_ROWS * rh + ui_px(20);
    // Systems per page: [-] N [+]
    int mx = fit(b, a);
    Rect srow = {in.x, y, lw, rh};
    // TRANSLATORS: left of the systems' count and its - and + buttons (481 px at 44 px).
    font_draw_left(c, FONT_BOLD, ts, (Rect){srow.x, srow.y, lw / 2, rh}, _("Systems per page"), 0);
    int bs = rh - pad;
    Rect less = {in.x + lw / 2, y + pad / 2, bs, bs};
    Rect more = {in.x + lw - bs, y + pad / 2, bs, bs};
    ui_button(c, less, ICON_MINUS, NULL, b->m.systems > 1 ? BTN_PLAIN : BTN_DISABLED);
    ui_button(c, more, ICON_PLUS, NULL, b->m.systems < mx ? BTN_PLAIN : BTN_DISABLED);
    if (b->m.systems > 1) hits_add(&b->hits, less, B_LESS, 0);
    if (b->m.systems < mx) hits_add(&b->hits, more, B_MORE, 0);
    char val[32];
    // TRANSLATORS: the systems per page, of the most that fit (275 px at 44 px).
    snprintf(val, sizeof val, _("%d of %d"), b->m.systems, mx);
    font_draw_centered(c, FONT_BOLD, ts, (Rect){less.x + bs, y, more.x - less.x - bs, rh}, val, 0);
    // The page as it will look.
    Rect pv = {in.x + lw + ui_px(50), in.y, in.w - lw - ui_px(50), in.h - ui_px(190)};
    if (pv.h * 3 / 4 < pv.w) {
        pv.x += (pv.w - pv.h * 3 / 4) / 2;
        pv.w = pv.h * 3 / 4;
    } else {
        pv.h = pv.w * 4 / 3;
    }
    canvas_fill(c, pv, 255);
    const Canvas *p = preview(b, a);
    if (p) canvas_blit_fit(c, p, pv);
    ui_frame(c, rect_inflate(pv, ui_px(3)), ui_px(8), 0, false, ui_px(3));
    int bh = ui_px(140), bw2 = ui_px(380);
    Rect cancel = {in.x, in.y + in.h - bh, bw2, bh};
    Rect done = {in.x + in.w - bw2, in.y + in.h - bh, bw2, bh};
    ui_button(c, cancel, ICON_NONE, _("Cancel"), BTN_PLAIN);
    // TRANSLATORS: takes the layout built on the Custom music sheet (364 px at 34 px).
    ui_button(c, done, ICON_CHECK, _("Use layout"), BTN_PRIMARY);
    hits_add(&b->hits, cancel, B_CANCEL, 0);
    hits_add(&b->hits, done, B_DONE, 0);
    return b->rect;
}

MbResult musicbuild_pointer(MusicBuild *b, App *a, int ph, float x, float y, uint32_t t, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!b->open) return MB_NONE;
    if (ph == 0) {
        press_begin(&b->press, x, y, t);
        return MB_NONE;
    }
    press_move(&b->press, x, y);
    if (ph != 2) return MB_NONE;
    if (!rect_contains(b->rect, (int)b->press.x0, (int)b->press.y0)) {
        b->open = false;  // a tap outside the sheet cancels
        *dirty = b->rect;
        return MB_CANCEL;
    }
    const Hit *h = hits_find(&b->hits, b->press.x0, b->press.y0);
    if (!h || !press_is_tap(&b->press, h->r)) return MB_NONE;
    MusicLayout *m = &b->m;
    int i = h->arg;
    bool at_max = m->systems >= fit(b, a);  // stay "as many as fit" through edits
    switch (h->id) {
    case B_UP:
    case B_DOWN: {
        int j = h->id == B_UP ? i - 1 : i + 1;
        if (i < 0 || j < 0 || i >= m->n || j >= m->n) return MB_NONE;
        MusicPart tmp = m->part[i];
        m->part[i] = m->part[j];
        m->part[j] = tmp;
        break;
    }
    case B_REMOVE:
        if (m->n <= 1 || i < 0 || i >= m->n) return MB_NONE;
        for (int k = i; k + 1 < m->n; k++) m->part[k] = m->part[k + 1];
        m->n--;
        break;
    case B_ADD:
        if (m->n >= MUSIC_MAX_PARTS) return MB_NONE;
        m->part[m->n++] = (MusicPart)i;
        break;
    case B_LESS: m->systems--; break;
    case B_MORE: m->systems++; break;
    case B_CANCEL:
        b->open = false;
        *dirty = b->rect;
        return MB_CANCEL;
    case B_DONE:
        b->open = false;
        a->settings.music_layout = music_encode(m);
        settings_save(&a->settings);
        *dirty = b->rect;
        return MB_DONE;
    default: return MB_NONE;
    }
    if (at_max && h->id != B_LESS) m->systems = fit(b, a);
    clamp_systems(b, a);
    musicbuild_draw(b, a);
    *dirty = b->rect;
    return MB_CHANGED;
}
