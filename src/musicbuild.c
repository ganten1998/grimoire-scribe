// musicbuild.c: the Custom music sheet (see musicbuild.h).
#include "musicbuild.h"

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

static void clamp_systems(MusicBuild *b) {
    int mx = music_max_systems(&b->m);
    b->m.systems = clampi(b->m.systems, 1, mx);
}

void musicbuild_open(MusicBuild *b, App *a, PaperKind start) {
    canvas_free(&b->prev);
    *b = (MusicBuild){.open = true};
    if (!music_decode(start, &b->m) && !music_decode(a->settings.music_layout, &b->m)) music_decode(music_default(), &b->m);
    clamp_systems(b);
    b->rect = card_rect(a);
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
    int pw, ph;
    store_page_size(&pw, &ph);
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
    Rect in = ui_sheet(c, b->rect, "Custom music");
    int rh = row_h(a), pad = ui_px(12);
    int lw = in.w * 58 / 100;
    float ts = ui_text_size();
    int y = in.y;
    font_draw_left(c, FONT_BOLD, ts, (Rect){in.x, y, lw, ui_px(70)}, "Each system, top to bottom", 0);
    y += ui_px(70);
    for (int i = 0; i < MUSIC_MAX_PARTS; i++, y += rh) {
        Rect row = {in.x, y + pad / 2, lw, rh - pad};
        if (i >= b->m.n) {
            if (i == b->m.n) font_draw_left(c, FONT_REGULAR, ui_small_size(), rect_inflate(row, -ui_px(20)), "(add a part below)", 150);
            continue;
        }
        ui_frame(c, row, ui_px(12), 0, false, ui_px(3));
        char lab[48];
        snprintf(lab, sizeof lab, "%d  %s", i + 1, music_part_name(b->m.part[i]));
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
    font_draw_left(c, FONT_BOLD, ts, (Rect){in.x, y, lw, ui_px(70)}, "Add a part", 0);
    y += ui_px(70);
    bool full = b->m.n >= MUSIC_MAX_PARTS;
    int bw = (lw - (ADD_COLS - 1) * pad) / ADD_COLS;
    for (int i = 0; i < N_PARTS; i++) {
        Rect r = {in.x + (i % ADD_COLS) * (bw + pad), y + (i / ADD_COLS) * rh + pad / 2, bw, rh - pad};
        ui_button(c, r, ICON_NONE, music_part_name(k_parts[i]), full ? BTN_DISABLED : BTN_PLAIN);
        if (!full) hits_add(&b->hits, r, B_ADD, (int)k_parts[i]);
    }
    y += ADD_ROWS * rh + ui_px(20);
    // Systems per page: [-] N [+]
    int mx = music_max_systems(&b->m);
    Rect srow = {in.x, y, lw, rh};
    font_draw_left(c, FONT_BOLD, ts, (Rect){srow.x, srow.y, lw / 2, rh}, "Systems per page", 0);
    int bs = rh - pad;
    Rect less = {in.x + lw / 2, y + pad / 2, bs, bs};
    Rect more = {in.x + lw - bs, y + pad / 2, bs, bs};
    ui_button(c, less, ICON_MINUS, NULL, b->m.systems > 1 ? BTN_PLAIN : BTN_DISABLED);
    ui_button(c, more, ICON_PLUS, NULL, b->m.systems < mx ? BTN_PLAIN : BTN_DISABLED);
    if (b->m.systems > 1) hits_add(&b->hits, less, B_LESS, 0);
    if (b->m.systems < mx) hits_add(&b->hits, more, B_MORE, 0);
    char val[32];
    snprintf(val, sizeof val, "%d of %d", b->m.systems, mx);
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
    ui_button(c, cancel, ICON_NONE, "Cancel", BTN_PLAIN);
    ui_button(c, done, ICON_CHECK, "Use layout", BTN_PRIMARY);
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
    bool at_max = m->systems >= music_max_systems(m);  // stay "as many as fit" through edits
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
    if (at_max && h->id != B_LESS) m->systems = music_max_systems(m);
    clamp_systems(b);
    musicbuild_draw(b, a);
    *dirty = b->rect;
    return MB_CHANGED;
}
