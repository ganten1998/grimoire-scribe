// editor.c: the notebook editor. Live ink, stroke eraser, the floating panel, page
// navigation, paper changes, autosave.
//
// Pixels: page_canvas holds paper + ink at page resolution (the device's portrait size, fixed).
// The view maps page -> screen (zoom + pan): 100% in portrait, fit-to-width when the Kindle is
// turned sideways, pinch to zoom 1x..4x. a->screen = the viewed page + panel (+ paper sheet).
// compose(r) rebuilds a screen rect from those layers; show(page rect) is the page-side entry.
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "export.h"
#include "dispmenu.h"
#include "history.h"

enum {
    P_NONE, P_TOOL, P_UNDO, P_REDO, P_PREV, P_NEXT, P_PAPER, P_PENSET, P_LIBRARY,
    P_COLLAPSE, P_GRIP, P_EXPAND, P_PICK_PAPER, P_PICK_CANCEL, P_CLEAR, P_DISPLAY,
};

typedef enum { OWN_NONE, OWN_DRAW, OWN_ERASE, OWN_PANEL, OWN_MODAL, OWN_DISPLAY } Owner;

// Every page visited while a notebook is open stays in memory with its history: undo
// actions point at the page's live Stroke objects, so a page must never be freed and
// reloaded from disk while its history exists.
typedef struct {
    uint32_t page_id;
    History h;
    Page *page;   // NULL until loaded
} PageHistory;

struct EditorState {
    bool open;
    // Pen side button gestures (when not drawing): 3 quick clicks, or a ~1.2 s hold while
    // hovering, clears the page (undoable). Holding it while drawing picks the button's tool.
    bool btn_down, btn_used_for_ink, btn_hold_fired;
    uint32_t btn_down_at;
    uint32_t clicks[3];
    int nclicks;
    NotebookInfo nb;
    int index;
    Page *page;
    Canvas page_canvas;
    bool page_dirty, info_dirty, changed;
    uint32_t last_up;

    PageHistory *hist;
    int nhist;

    // Pointer ownership: which input started the current gesture and what it does.
    Owner owner;
    bool owner_is_pen;

    // Drawing
    Stroke *cur;
    BrushState bs;
    Smoother sm;

    // Erasing: one gesture = one undo step.
    Stroke **snap;      // page stroke list at gesture start (to recover original indices)
    int nsnap;
    Stroke **removed;
    int nremoved, capremoved;
    float ex, ey;
    Rect erase_dirty;

    // Panel
    Rect panel;
    bool collapsed;
    bool dragging;
    float grab_dx, grab_dy;
    Rect drag_start_rect;
    uint32_t last_drag_paint;
    HitList hits;
    Press press;
    Hit pressed;
    bool has_pressed;

    // View (page -> screen): screen = (page - o) * zoom.
    int pgw, pgh;               // page size
    float zoom, ox, oy;
    float zoom_default, zoom_min;
    int view_w, view_h;         // screen size the view was set up for
    Page *canvas_page;          // which page page_canvas currently shows
    bool pinching;
    float pinch_d0, pinch_z0, pinch_px, pinch_py;  // start spread/zoom, page point under midpoint
    uint32_t last_pinch_paint;

    DispMenu dm;                // Display sheet (front light, invert)

    // Paper sheet
    bool paper_open;
    Rect paper_rect;
    HitList modal_hits;
    Canvas paper_prev[PAPER_COUNT];
};

#define S (a->ed)

// ---- layout ----

// Panel metrics shrink (S->pscale) when the screen is too short for the full panel, i.e. when
// the Kindle is turned sideways.
static float g_pscale = 1.f;
static int sc(float v) { return ui_px(v * g_pscale); }
static int pw(void) { return ui_px(128); }       // expanded width (icons only)
static int btn_h(void) { return sc(100); }
static int gap(void) { return sc(8); }
static int div_h(void) { return sc(20); }
static int head_h(void) { return sc(80); }
static int box(void) { return ui_px(120); }       // collapsed square
static int label_h(void) { return sc(60); }

static int panel_height(void) {
    // header, 14 buttons (5 tools, undo, redo, clear, prev, next, paper, display, pen feel,
    // library), the page label and 3 dividers. Must match draw_panel.
    return head_h() + gap() + 14 * (btn_h() + gap()) + label_h() + gap() + 3 * div_h() + ui_px(6);
}

static void fit_panel_to(int screen_h) {
    g_pscale = 1.f;
    int full = panel_height(), room = screen_h - ui_px(24);
    if (full > room) g_pscale = (float)room / (float)full;
}

static Rect panel_rect_at(App *a, int x, int y) {
    int w = S->collapsed ? box() : pw(), h = S->collapsed ? box() : panel_height();
    x = clampi(x, 0, a->w - w);
    y = clampi(y, 0, a->h - h);
    return (Rect){x, y, w, h};
}

// ---- histories ----

static PageHistory *entry_for(App *a, uint32_t pid) {
    for (int i = 0; i < S->nhist; i++)
        if (S->hist[i].page_id == pid) return &S->hist[i];
    PageHistory *nh = realloc(S->hist, (size_t)(S->nhist + 1) * sizeof *nh);
    if (!nh) return NULL;
    S->hist = nh;
    PageHistory *e = &S->hist[S->nhist++];
    e->page_id = pid;
    e->page = NULL;
    history_init(&e->h);
    return e;
}

static History *cur_hist(App *a) {
    PageHistory *e = entry_for(a, S->nb.page_ids[S->index]);
    return e ? &e->h : NULL;
}

// The page at index, from the session cache or (first visit) from disk.
static Page *page_at(App *a, int index) {
    PageHistory *e = entry_for(a, S->nb.page_ids[index]);
    if (!e) return NULL;
    if (!e->page && store_load_page(&S->nb, index, S->pgw, S->pgh, &e->page) != 0) return NULL;
    return e->page;
}

static void free_histories(App *a) {
    for (int i = 0; i < S->nhist; i++) {
        history_free(&S->hist[i].h);
        page_free(S->hist[i].page);
    }
    free(S->hist);
    S->hist = NULL;
    S->nhist = 0;
    S->page = NULL;  // it lived in the cache
}

// ---- drawing the panel ----

static ButtonStyle tool_style(App *a, ToolKind t) {
    return a->settings.current_tool == t ? BTN_SELECTED : BTN_PLAIN;
}

static void draw_panel(App *a) {
    Canvas *c = &a->screen;
    Rect p = S->panel;
    hits_clear(&S->hits);
    static const IconKind tool_icons[TOOL_COUNT] = {ICON_PENCIL, ICON_MECH_PENCIL, ICON_BALLPOINT, ICON_ERASER,
                                                    ICON_HIGHLIGHTER};
    if (S->collapsed) {
        canvas_round_rect(c, p, ui_px(22), 255, true, 0);
        canvas_round_rect(c, p, ui_px(22), 0, false, ui_px(6));
        Rect ir = rect_inflate(p, -ui_px(20));
        ir.h -= ui_px(12);
        ui_icon(c, tool_icons[a->settings.current_tool], ir, 0);
        // Expand hint in the corner.
        ui_icon(c, ICON_EXPAND, (Rect){p.x + p.w - ui_px(40), p.y + p.h - ui_px(40), ui_px(30), ui_px(30)}, 0);
        hits_add(&S->hits, p, P_EXPAND, 0);
        return;
    }
    canvas_round_rect(c, p, ui_px(22), 255, true, 0);
    canvas_round_rect(c, p, ui_px(22), 0, false, ui_px(6));
    int x = p.x + ui_px(14), w = p.w - ui_px(28);
    int y = p.y + ui_px(10);
    // Header: grip (drag) + collapse.
    Rect grip = {p.x, p.y, p.w - ui_px(62), head_h()};
    Rect col = {p.x + p.w - ui_px(64), p.y + sc(12), ui_px(54), sc(56)};
    ui_icon(c, ICON_GRIP, (Rect){p.x + ui_px(10), p.y + sc(14), ui_px(52), sc(52)}, 0);
    ui_icon(c, ICON_COLLAPSE, col, 0);
    hits_add(&S->hits, grip, P_GRIP, 0);
    hits_add(&S->hits, rect_inflate(col, ui_px(10)), P_COLLAPSE, 0);
    y = p.y + head_h();
    canvas_hline(c, p.x + ui_px(6), p.x + p.w - ui_px(6), y - ui_px(4), ui_px(3), 0);
    y += gap();
    static const ToolKind order[TOOL_COUNT] = {TOOL_PENCIL, TOOL_MECH_PENCIL, TOOL_BALLPOINT, TOOL_HIGHLIGHTER,
                                               TOOL_ERASER};
    for (int i = 0; i < TOOL_COUNT; i++) {
        ToolKind t = order[i];
        Rect r = {x, y, w, btn_h()};
        ui_button(c, r, tool_icons[t], NULL, tool_style(a, (ToolKind)t));
        hits_add(&S->hits, r, P_TOOL, t);
        y += btn_h() + gap();
    }
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6), ui_px(3), 160);
    y += div_h();
    History *h = S->open ? cur_hist(a) : NULL;
    bool cu = h && history_can_undo(h), cr = h && history_can_redo(h);
    Rect ru = {x, y, w, btn_h()};
    ui_button(c, ru, ICON_UNDO, NULL, cu ? BTN_PLAIN : BTN_DISABLED);
    if (cu) hits_add(&S->hits, ru, P_UNDO, 0);
    y += btn_h() + gap();
    Rect rr = {x, y, w, btn_h()};
    ui_button(c, rr, ICON_REDO, NULL, cr ? BTN_PLAIN : BTN_DISABLED);
    if (cr) hits_add(&S->hits, rr, P_REDO, 0);
    y += btn_h() + gap();
    Rect rc = {x, y, w, btn_h()};  // clear the page right away (no confirmation)
    bool can_clear = S->page && S->page->n > 0;
    ui_button(c, rc, ICON_BROOM, NULL, can_clear ? BTN_PLAIN : BTN_DISABLED);
    if (can_clear) hits_add(&S->hits, rc, P_CLEAR, 0);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6), ui_px(3), 160);
    y += div_h();
    Rect rp = {x, y, w, btn_h()};
    ui_button(c, rp, ICON_PREV, NULL, S->index > 0 ? BTN_PLAIN : BTN_DISABLED);
    if (S->index > 0) hits_add(&S->hits, rp, P_PREV, 0);
    y += btn_h() + gap();
    char lab[32];
    snprintf(lab, sizeof lab, "%d / %d", S->index + 1, S->nb.npages);
    font_draw_centered(c, FONT_BOLD, ui_small_size() * (g_pscale < 1.f ? g_pscale : 1.f),
                       (Rect){x - ui_px(6), y, w + ui_px(12), label_h()}, lab, 0);
    y += label_h() + gap();
    bool last = S->index >= S->nb.npages - 1;
    Rect rn = {x, y, w, btn_h()};
    ui_button(c, rn, last ? ICON_PLUS : ICON_NEXT, NULL, BTN_PLAIN);
    hits_add(&S->hits, rn, P_NEXT, 0);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6), ui_px(3), 160);
    y += div_h();
    struct {
        IconKind icon;
        const char *label;
        int id;
    } rest[] = {{ICON_PAPER, "Paper", P_PAPER},
                {ICON_BRIGHTNESS, "Display", P_DISPLAY},
                {ICON_TUNE, "Pen feel", P_PENSET},
                {ICON_LIBRARY, "Library", P_LIBRARY}};
    for (int i = 0; i < 4; i++) {
        Rect r = {x, y, w, btn_h()};
        ui_button(c, r, rest[i].icon, NULL, BTN_PLAIN);
        hits_add(&S->hits, r, rest[i].id, 0);
        y += btn_h() + gap();
    }
}

static const Canvas *paper_preview(App *a, PaperKind k) {
    Canvas *c = &S->paper_prev[k];
    if (!c->px) {
        Canvas full;
        if (canvas_init(&full, S->pgw, S->pgh) != 0) return NULL;
        paper_render(&full, k, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
        if (canvas_init(c, S->pgw / 6, S->pgh / 6) == 0) canvas_downsample_keep_lines(c, &full, 0.6f);
        canvas_free(&full);
    }
    return c->px ? c : NULL;
}

static void draw_paper_sheet(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->modal_hits);
    int m = ui_px(70), gap_ = ui_px(30), cols = 4, rows = (PAPER_COUNT + cols - 1) / cols;
    int tw = (a->w - 2 * m - ui_px(72) - (cols - 1) * gap_) / cols;
    if (tw > a->w / 6) tw = a->w / 6;
    int th = tw * 4 / 3, cell_h = th + ui_px(100);
    int gridw = cols * tw + (cols - 1) * gap_;
    S->paper_rect = (Rect){m, a->h / 6, a->w - 2 * m, ui_px(120) + ui_px(72) + rows * cell_h + ui_px(190)};
    Rect in = ui_sheet(c, S->paper_rect, "Paper for this page");
    for (int i = 0; i < PAPER_COUNT; i++) {
        PaperKind k = paper_at(i);
        Rect tr = {in.x + (in.w - gridw) / 2 + (i % cols) * (tw + gap_), in.y + (i / cols) * cell_h, tw, th};
        canvas_fill(c, tr, 255);
        const Canvas *pv = paper_preview(a, k);
        if (pv) canvas_blit(c, pv, (Rect){0, 0, tw, th}, tr.x, tr.y);
        bool sel = S->page && S->page->paper == (PaperKind)k;
        canvas_round_rect(c, rect_inflate(tr, sel ? ui_px(10) : 0), ui_px(12), 0, false, sel ? ui_px(9) : ui_px(3));
        font_draw_centered(c, sel ? FONT_BOLD : FONT_REGULAR, ui_small_size() * 0.9f,
                           (Rect){tr.x - gap_ / 2, tr.y + th + ui_px(16), tw + gap_, ui_px(64)}, paper_name(k), 0);
        hits_add(&S->modal_hits, (Rect){tr.x, tr.y, tw, th + ui_px(90)}, P_PICK_PAPER, (int)k);
    }
    Rect cancel = {in.x + in.w - ui_px(380), in.y + in.h - ui_px(140), ui_px(380), ui_px(140)};
    ui_button(c, cancel, ICON_NONE, "Cancel", BTN_PLAIN);
    hits_add(&S->modal_hits, cancel, P_PICK_CANCEL, 0);
}

// ---- view ----

static bool view_identity(App *a) {
    return S->zoom == 1.f && S->ox == 0.f && S->oy == 0.f && S->pgw == a->w && S->pgh == a->h;
}

static void view_clamp(App *a) {
    S->zoom = clampf(S->zoom, S->zoom_min, S->zoom_default * 4.f);
    float vw = (float)a->w / S->zoom, vh = (float)a->h / S->zoom;  // visible page area
    S->ox = vw >= (float)S->pgw ? -(vw - (float)S->pgw) / 2.f : clampf(S->ox, 0.f, (float)S->pgw - vw);
    S->oy = vh >= (float)S->pgh ? -(vh - (float)S->pgh) / 2.f : clampf(S->oy, 0.f, (float)S->pgh - vh);
    // Snap to exactly 100% / no offset in portrait so the fast identity path is used.
    if (fabsf(S->zoom - 1.f) < 0.01f && S->pgw == a->w && S->pgh == a->h) {
        S->zoom = 1.f;
        S->ox = S->oy = 0.f;
    }
}

// Portrait: 100%. Sideways: the page fills the width (pan with two fingers).
static void view_reset(App *a) {
    S->zoom_min = fminf((float)a->w / (float)S->pgw, (float)a->h / (float)S->pgh);
    S->zoom_default = (float)a->w / (float)S->pgw;
    S->zoom = S->zoom_default;
    S->ox = S->oy = 0.f;
    S->view_w = a->w;
    S->view_h = a->h;
    view_clamp(a);
}

static void to_page(App *a, float sx, float sy, float *px, float *py) {
    *px = sx / S->zoom + S->ox;
    *py = sy / S->zoom + S->oy;
}

static Rect page_to_screen(App *a, Rect p) {
    if (rect_is_empty(p)) return RECT_EMPTY;
    float x0 = ((float)p.x - S->ox) * S->zoom, y0 = ((float)p.y - S->oy) * S->zoom;
    float x1 = ((float)(p.x + p.w) - S->ox) * S->zoom, y1 = ((float)(p.y + p.h) - S->oy) * S->zoom;
    Rect r = {(int)floorf(x0) - 2, (int)floorf(y0) - 2, (int)ceilf(x1 - x0) + 5, (int)ceilf(y1 - y0) + 5};
    return rect_intersect(r, (Rect){0, 0, a->w, a->h});
}

static Rect zoom_chip(App *a) { return (Rect){a->w / 2 - ui_px(190), ui_px(24), ui_px(380), ui_px(100)}; }
static bool zoomed(App *a) { return fabsf(S->zoom - S->zoom_default) > 0.01f || S->ox != 0.f || S->oy > 0.5f; }

// Draws the visible page into a screen rect: bilinear sampling, a gray desk outside the page.
static void blit_view(App *a, Rect r) {
    const Canvas *pc = &S->page_canvas;
    static int ix[8192];
    static uint8_t wx[8192];
    if (r.w > 8192) r.w = 8192;
    for (int i = 0; i < r.w; i++) {
        float fx = ((float)(r.x + i) + 0.5f) / S->zoom + S->ox - 0.5f;
        int x = (int)floorf(fx);
        ix[i] = x;
        wx[i] = (uint8_t)((fx - (float)x) * 255.f);
    }
    for (int y = r.y; y < r.y + r.h; y++) {
        uint8_t *dst = &a->screen.px[(size_t)y * a->screen.stride];
        float fy = ((float)y + 0.5f) / S->zoom + S->oy - 0.5f;
        int py = (int)floorf(fy);
        unsigned wy = (unsigned)((fy - (float)py) * 255.f);
        int py0 = clampi(py, 0, pc->h - 1), py1 = clampi(py + 1, 0, pc->h - 1);
        const uint8_t *r0 = &pc->px[(size_t)py0 * pc->stride], *r1 = &pc->px[(size_t)py1 * pc->stride];
        bool row_out = py < -1 || py >= pc->h;
        for (int i = 0; i < r.w; i++) {
            int x = ix[i];
            if (row_out || x < -1 || x >= pc->w) {
                dst[r.x + i] = 205;
                continue;
            }
            int x0 = clampi(x, 0, pc->w - 1), x1 = clampi(x + 1, 0, pc->w - 1);
            unsigned w = wx[i];
            unsigned top = r0[x0] * (255 - w) + r0[x1] * w, bot = r1[x0] * (255 - w) + r1[x1] * w;
            dst[r.x + i] = (uint8_t)((top * (255 - wy) + bot * wy + 32512) / 65025);
        }
    }
}

// Rebuild a screen rect from the layers.
static void compose(App *a, Rect r) {
    r = rect_intersect(r, (Rect){0, 0, a->w, a->h});
    if (rect_is_empty(r)) return;
    if (view_identity(a)) canvas_blit(&a->screen, &S->page_canvas, r, r.x, r.y);
    else blit_view(a, r);
    if (zoomed(a) && rect_overlaps(r, zoom_chip(a))) {
        char z[48];
        snprintf(z, sizeof z, "%d%%   tap to reset", (int)lrintf(S->zoom / S->zoom_default * 100.f));
        Rect c = zoom_chip(a);
        canvas_round_rect(&a->screen, c, c.h / 2, 0, true, 0);
        font_draw_centered(&a->screen, FONT_BOLD, ui_small_size(), c, z, 255);
    }
    if (rect_overlaps(r, S->panel)) draw_panel(a);
    if (S->paper_open && rect_overlaps(r, S->paper_rect)) draw_paper_sheet(a);
    if (S->dm.open && rect_overlaps(r, S->dm.rect)) dispmenu_draw(&S->dm, a);
}

// A page-space change: recompose what it covers on screen and refresh it.
static void show(App *a, Rect page_rect, RefreshMode m) {
    Rect r = page_to_screen(a, page_rect);
    if (rect_is_empty(r)) return;
    compose(a, r);
    app_invalidate(a, r, m);
}

static void compose_all(App *a, RefreshMode m) {
    compose(a, (Rect){0, 0, a->w, a->h});
    app_invalidate_all(a, m);
}

static void repaint_panel(App *a, RefreshMode m) {
    draw_panel(a);
    app_invalidate(a, S->panel, m);
}

// ---- pages ----

static void save_thumb_from_canvas(App *a) {
    Canvas t;
    if (canvas_init(&t, S->pgw / 3, S->pgh / 3) != 0) return;
    canvas_downsample_keep_lines(&t, &S->page_canvas, 0.35f);
    store_save_thumb(S->nb.id, &t);
    canvas_free(&t);
}

static void save_page(App *a) {
    if (!S->open) return;
    if (S->page_dirty) {
        if (store_save_page(&S->nb, S->index, S->page) != 0) app_toast(a, "Could not save this page!", 4000);
        if (S->index == 0) save_thumb_from_canvas(a);
        S->page_dirty = false;
        S->info_dirty = true;
    }
    if (S->info_dirty) {
        store_save_info(&S->nb);
        S->info_dirty = false;
    }
}

static void render_page_full(App *a) {
    page_render(&S->page_canvas, S->page, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
    S->canvas_page = S->page;
}

static void go_to_page(App *a, int index) {
    save_page(a);
    index = clampi(index, 0, S->nb.npages - 1);
    Page *np = page_at(a, index);
    if (!np) {
        app_toast(a, "Could not open that page", 3000);
        return;
    }
    S->page = np;
    S->index = index;
    S->nb.last_page = index;
    S->info_dirty = true;
    render_page_full(a);
    view_reset(a);
    compose_all(a, REFRESH_FULL);
}

static void close_notebook(App *a) {
    if (!S->open) return;
    save_page(a);
    if (S->index != 0 && S->changed) {
        // Keep the library thumbnail (page 1) in sync even if we never visited it.
        Page *p0;
        if (store_load_page(&S->nb, 0, S->pgw, S->pgh, &p0) == 0) {
            Canvas t;
            if (canvas_init(&t, S->pgw / 3, S->pgh / 3) == 0) {
                page_render_thumb(&t, p0, a->dpi);
                store_save_thumb(S->nb.id, &t);
                canvas_free(&t);
            }
            page_free(p0);
        }
    }
    if (a->settings.mirror_pdf && S->changed) {
        int rc = export_mirror_pdf(&S->nb, a->dpi, app_docs_dir());
        if (rc != 0) qlog("mirror pdf failed for %s", S->nb.id);
    }
    free_histories(a);
    S->open = false;
}

void editor_open_notebook(App *a, const char *id) {
    close_notebook(a);
    if (store_load_info(id, &S->nb) != 0) {
        app_toast(a, "Could not open that notebook", 3000);
        return;
    }
    S->index = clampi(S->nb.last_page, 0, S->nb.npages - 1);
    S->page = page_at(a, S->index);
    if (!S->page) {
        free_histories(a);
        app_toast(a, "Could not open that notebook", 3000);
        return;
    }
    S->open = true;
    S->page_dirty = S->info_dirty = S->changed = false;
    if (!S->page_canvas.px && canvas_init(&S->page_canvas, S->pgw, S->pgh) != 0) return;
    render_page_full(a);
    S->view_w = 0;  // set the view up afresh on enter
    app_switch(a, SCR_EDITOR);
}

void editor_save_now(App *a) { save_page(a); }

// ---- ink ----

static void mark_changed(App *a) {
    S->page_dirty = true;
    S->changed = true;
    S->last_up = now_ms();
}

static PenSample sample_of(App *a, const InEvent *ev) {
    PenSample s = {0, 0, ev->pressure, ev->tilt_x, ev->tilt_y, ev->t_ms};
    to_page(a, ev->x, ev->y, &s.x, &s.y);
    return s;
}

static ToolKind effective_tool(App *a, const InEvent *ev) {
    if (ev->eraser) return TOOL_ERASER;  // the Premium Pen's eraser end
    if (ev->side_button && a->settings.side_button == 1) return TOOL_ERASER;
    if (ev->side_button && a->settings.side_button == 2) return TOOL_HIGHLIGHTER;
    return a->settings.current_tool;
}

static void draw_begin(App *a, const InEvent *ev) {
    ToolKind t = effective_tool(a, ev);
    if (t == TOOL_ERASER) {
        S->owner = OWN_ERASE;
        free(S->snap);
        S->nsnap = S->page->n;
        S->snap = malloc((size_t)(S->nsnap ? S->nsnap : 1) * sizeof *S->snap);
        if (S->snap) memcpy(S->snap, S->page->strokes, (size_t)S->nsnap * sizeof *S->snap);
        S->nremoved = 0;
        to_page(a, ev->x, ev->y, &S->ex, &S->ey);
        S->erase_dirty = RECT_EMPTY;
        if (S->btn_down) S->btn_used_for_ink = true;
        return;
    }
    if (S->btn_down) S->btn_used_for_ink = true;
    S->owner = OWN_DRAW;
    uint32_t seed = (ev->t_ms * 2654435761u) ^ (uint32_t)(S->page->n * 40503u) ^ (uint32_t)(ev->x * 97.f);
    S->cur = stroke_new(t, &a->settings.tools[t], seed);
    if (!S->cur) {
        S->owner = OWN_NONE;
        return;
    }
    brush_begin(&S->bs, t, &a->settings.tools[t], seed, a->dpi);
    smoother_reset(&S->sm);
    display_pen_mode(true);
}

static void draw_sample(App *a, const InEvent *ev) {
    PenSample s = sample_of(a, ev);
    smoother_apply(&S->sm, S->cur->params.smoothing, &s);
    stroke_append(S->cur, &s, a->dpi);
    Rect d = brush_add(&S->bs, &S->page_canvas, &s);
    if (!rect_is_empty(d)) show(a, d, REFRESH_FAST);
}

static void draw_end(App *a) {
    Stroke *s = S->cur;
    S->cur = NULL;
    S->owner = OWN_NONE;
    display_pen_mode(false);
    if (!s) return;
    if (s->n == 0) {
        stroke_free(s);
        return;
    }
    History *h = cur_hist(a);
    bool had_undo = h && history_can_undo(h), had_redo = h && history_can_redo(h);
    bool was_empty = S->page->n == 0;
    page_add(S->page, s);
    if (h && S->page->n && S->page->strokes[S->page->n - 1] == s) history_push_add(h, S->page->n - 1, s);
    // Settle the grays the fast waveform could not show.
    show(a, s->bbox, REFRESH_UI);
    mark_changed(a);
    // Only touch the panel when Undo/Redo availability actually changed.
    if (!had_undo || had_redo || was_empty) repaint_panel(a, REFRESH_UI);
}

static void erase_segment(App *a, float sx, float sy, float pressure) {
    const ToolParams *ep = &a->settings.tools[TOOL_ERASER];
    float p = brush_pressure(ep, pressure);
    // The eraser keeps its size on screen whatever the zoom.
    float r = mm_to_px(lerpf(ep->size_min_mm, ep->size_max_mm, p), a->dpi) * 0.5f / S->zoom;
    float x, y;
    to_page(a, sx, sy, &x, &y);
    Rect d = RECT_EMPTY;
    for (int i = S->page->n - 1; i >= 0; i--) {
        Stroke *s = S->page->strokes[i];
        if (!stroke_hit_segment(s, S->ex, S->ey, x, y, r, a->dpi)) continue;
        page_remove(S->page, i);
        if (S->nremoved == S->capremoved) {
            int nc = S->capremoved ? S->capremoved * 2 : 16;
            Stroke **nr = realloc(S->removed, (size_t)nc * sizeof *nr);
            if (!nr) {
                page_insert(S->page, i, s);  // could not track it: put it back
                continue;
            }
            S->removed = nr;
            S->capremoved = nc;
        }
        S->removed[S->nremoved++] = s;
        d = rect_union(d, s->bbox);
    }
    S->ex = x;
    S->ey = y;
    if (!rect_is_empty(d)) {
        page_render(&S->page_canvas, S->page, d, a->dpi);
        show(a, d, REFRESH_FAST);
        S->erase_dirty = rect_union(S->erase_dirty, d);
    }
}

static int snap_index(App *a, const Stroke *s) {
    for (int i = 0; i < S->nsnap; i++)
        if (S->snap[i] == s) return i;
    return -1;
}

static void erase_end(App *a) {
    S->owner = OWN_NONE;
    if (S->nremoved > 0) {
        // History wants original indices ascending, paired with their strokes.
        int n = S->nremoved;
        int *idx = malloc((size_t)n * sizeof *idx);
        Stroke **st = malloc((size_t)n * sizeof *st);
        if (idx && st) {
            int m = 0;
            for (int i = 0; i < S->nsnap; i++)
                for (int k = 0; k < n; k++)
                    if (S->removed[k] == S->snap[i]) {
                        idx[m] = i;
                        st[m++] = S->removed[k];
                    }
            // Strokes not found in the snapshot cannot happen (erase only touches the page),
            // but never leak them.
            for (int k = 0; k < n; k++)
                if (snap_index(a, S->removed[k]) < 0) stroke_free(S->removed[k]);
            History *h = cur_hist(a);
            if (h) history_push_erase(h, m, idx, st);
            else
                for (int k = 0; k < m; k++) stroke_free(st[k]);
        } else {
            for (int k = 0; k < n; k++) stroke_free(S->removed[k]);
        }
        free(idx);
        free(st);
        S->nremoved = 0;
        show(a, S->erase_dirty, REFRESH_UI);
        mark_changed(a);
        repaint_panel(a, REFRESH_UI);
    }
    free(S->snap);
    S->snap = NULL;
    S->nsnap = 0;
}

static void clear_page(App *a) {
    if (!S->page || S->page->n == 0) {
        app_toast(a, "This page is already empty", 1500);
        return;
    }
    History *h = cur_hist(a);
    if (!h) return;
    history_push_clear(h, S->page);  // takes every stroke off the page, undoable
    render_page_full(a);
    compose_all(a, REFRESH_FULL);
    mark_changed(a);
    repaint_panel(a, REFRESH_FULL);
    app_toast(a, "Page cleared", 1500);
}

// Side button: clicks and holds that happen while not drawing.
static void pen_button(App *a, const InEvent *ev) {
    uint32_t t = ev->t_ms;
    if (ev->side_button) {
        S->btn_down = true;
        S->btn_down_at = t;
        S->btn_used_for_ink = S->owner == OWN_DRAW || S->owner == OWN_ERASE;
        S->btn_hold_fired = false;
        return;
    }
    if (!S->btn_down) return;
    S->btn_down = false;
    if (S->btn_used_for_ink || S->btn_hold_fired || t - S->btn_down_at > 450) return;
    // A click: keep the last three; three within 1.2 s clear the page.
    if (S->nclicks == 3) {
        S->clicks[0] = S->clicks[1];
        S->clicks[1] = S->clicks[2];
        S->nclicks = 2;
    }
    S->clicks[S->nclicks++] = t;
    if (S->nclicks == 3 && S->clicks[2] - S->clicks[0] <= 1200) {
        S->nclicks = 0;
        if (S->owner == OWN_NONE && !S->paper_open) clear_page(a);
    }
}

static void undo_redo(App *a, bool undo) {
    History *h = cur_hist(a);
    Rect d;
    if (!h || !(undo ? history_undo(h, S->page, &d) : history_redo(h, S->page, &d))) return;
    page_render(&S->page_canvas, S->page, d, a->dpi);
    bool full = d.w >= S->pgw && d.h >= S->pgh;
    show(a, d, full ? REFRESH_FULL : REFRESH_UI);
    mark_changed(a);
    repaint_panel(a, REFRESH_UI);
}

// ---- panel interaction ----

static void move_panel_to(App *a, int x, int y, RefreshMode m) {
    Rect old = S->panel;
    S->panel = panel_rect_at(a, x, y);
    compose(a, old);
    draw_panel(a);
    app_invalidate(a, rect_union(old, S->panel), m);
}

static void persist_panel(App *a) {
    a->settings.panel_x = S->panel.x;
    a->settings.panel_y = S->panel.y;
    a->settings.panel_collapsed = S->collapsed;
    settings_save(&a->settings);
}

static void set_collapsed(App *a, bool c) {
    Rect old = S->panel;
    S->collapsed = c;
    // Collapse toward the top-right corner of where the panel was (and expand from there).
    int x = c ? old.x + old.w - box() : old.x + old.w - pw();
    S->panel = panel_rect_at(a, x, old.y);
    compose(a, old);
    draw_panel(a);
    app_invalidate(a, rect_union(old, S->panel), REFRESH_UI);
    persist_panel(a);
}

static void panel_act(App *a, const Hit *h) {
    switch (h->id) {
    case P_TOOL:
        if (a->settings.current_tool == (ToolKind)h->arg) break;
        a->settings.current_tool = (ToolKind)h->arg;
        settings_save(&a->settings);
        repaint_panel(a, REFRESH_UI);
        break;
    case P_UNDO: undo_redo(a, true); break;
    case P_REDO: undo_redo(a, false); break;
    case P_PREV: go_to_page(a, S->index - 1); break;
    case P_NEXT:
        if (S->index >= S->nb.npages - 1) {
            save_page(a);
            if (store_add_page(&S->nb, S->nb.npages, S->page->paper) != 0) {
                app_toast(a, "Could not add a page", 3000);
                return;
            }
            S->changed = true;
        }
        go_to_page(a, S->index + 1);
        break;
    case P_PAPER:
        S->paper_open = true;
        draw_paper_sheet(a);
        app_invalidate(a, S->paper_rect, REFRESH_UI);
        break;
    case P_CLEAR: clear_page(a); break;
    case P_DISPLAY:
        dispmenu_open(&S->dm, a);
        app_invalidate(a, dispmenu_draw(&S->dm, a), REFRESH_UI);
        break;
    case P_PENSET: pensettings_open(a, a->settings.current_tool); break;
    case P_LIBRARY:
        app_toast(a, "Saving...", 8000);
        app_flush(a);
        close_notebook(a);
        app_switch(a, SCR_LIBRARY);
        app_toast(a, "Saved", 1200);
        break;
    case P_COLLAPSE: set_collapsed(a, true); break;
    case P_EXPAND: set_collapsed(a, false); break;
    default: break;
    }
}

static void modal_act(App *a, const Hit *h) {
    Rect sheet = S->paper_rect;
    S->paper_open = false;
    if (h->id == P_PICK_PAPER && S->page->paper != (PaperKind)h->arg) {
        History *hi = cur_hist(a);
        if (hi) history_push_paper(hi, S->page->paper, (PaperKind)h->arg);
        S->page->paper = (PaperKind)h->arg;
        render_page_full(a);
        compose_all(a, REFRESH_FULL);
        mark_changed(a);
        return;
    }
    compose(a, sheet);
    app_invalidate(a, sheet, REFRESH_UI);
}

static void ui_pointer(App *a, HitList *hl, int ph, const InEvent *ev, bool modal) {
    if (ph == 0) {
        press_begin(&S->press, ev->x, ev->y, ev->t_ms);
        const Hit *h = hits_find(hl, ev->x, ev->y);
        S->has_pressed = h != NULL;
        S->dragging = false;
        if (h) {
            S->pressed = *h;
            S->grab_dx = ev->x - S->panel.x;
            S->grab_dy = ev->y - S->panel.y;
            S->drag_start_rect = S->panel;
            // Drag handles do not invert; buttons do.
            if (h->id != P_GRIP && h->id != P_EXPAND) app_press_feedback(a, h->r);
        }
        return;
    }
    if (!S->has_pressed) return;
    bool draggable = !modal && (S->pressed.id == P_GRIP || S->pressed.id == P_EXPAND);
    if (ph == 1) {
        press_move(&S->press, ev->x, ev->y);
        if (draggable && S->press.moved) {
            S->dragging = true;
            if (ev->t_ms - S->last_drag_paint > 70) {
                S->last_drag_paint = ev->t_ms;
                move_panel_to(a, (int)(ev->x - S->grab_dx), (int)(ev->y - S->grab_dy), REFRESH_FAST);
            }
        }
        return;
    }
    // Release.
    press_move(&S->press, ev->x, ev->y);
    S->has_pressed = false;
    if (S->dragging) {
        S->dragging = false;
        move_panel_to(a, (int)(ev->x - S->grab_dx), (int)(ev->y - S->grab_dy), REFRESH_UI);
        // Also clean up wherever the panel passed through.
        Rect trail = rect_union(S->drag_start_rect, S->panel);
        compose(a, trail);
        app_invalidate(a, trail, REFRESH_UI);
        persist_panel(a);
        return;
    }
    Hit h = S->pressed;
    if (h.id != P_GRIP && h.id != P_EXPAND) {
        canvas_invert(&a->screen, h.r);
        app_invalidate(a, h.r, REFRESH_UI);
    }
    if (!press_is_tap(&S->press, h.r) && !rect_contains(h.r, (int)ev->x, (int)ev->y)) return;
    if (modal) modal_act(a, &h);
    else panel_act(a, &h);
}

// ---- screen interface ----

void editor_init(App *a) {
    a->ed = calloc(1, sizeof *a->ed);
    if (!a->ed) return;
    store_page_size(&S->pgw, &S->pgh);
    S->zoom = S->zoom_default = S->zoom_min = 1.f;
}

void editor_free(App *a) {
    if (!S) return;
    close_notebook(a);
    stroke_free(S->cur);
    free(S->snap);
    for (int k = 0; k < S->nremoved; k++) stroke_free(S->removed[k]);
    free(S->removed);
    canvas_free(&S->page_canvas);
    for (int k = 0; k < PAPER_COUNT; k++) canvas_free(&S->paper_prev[k]);
    free(S);
    a->ed = NULL;
}

void editor_enter(App *a) {
    if (!S->page_canvas.px && canvas_init(&S->page_canvas, S->pgw, S->pgh) != 0) return;
    if (S->open && S->page && S->canvas_page != S->page) render_page_full(a);
    if (S->view_w != a->w || S->view_h != a->h) view_reset(a);  // new notebook or turned
    S->pinching = false;
    fit_panel_to(a->h);
    S->collapsed = a->settings.panel_collapsed;
    int x = a->settings.panel_x, y = a->settings.panel_y;
    if (x < 0 || y < 0) {
        x = a->w - (S->collapsed ? box() : pw()) - ui_px(28);
        y = ui_px(220);
    }
    S->panel = panel_rect_at(a, x, y);
    S->owner = OWN_NONE;
    S->paper_open = false;
    S->dm.open = false;
    compose(a, (Rect){0, 0, a->w, a->h});
}

bool editor_busy(App *a) { return S && (S->owner != OWN_NONE || S->pinching); }

// Two fingers: zoom around the midpoint and pan with it.
static void pinch(App *a, const InEvent *ev) {
    float mx = (ev->x + ev->x2) * 0.5f, my = (ev->y + ev->y2) * 0.5f;
    float d = hypotf(ev->x2 - ev->x, ev->y2 - ev->y);
    if (ev->type == IN_PINCH_BEGIN) {
        if (S->owner == OWN_PANEL && !S->owner_is_pen) {
            // The first finger had pressed a panel button: that press is cancelled.
            if (S->has_pressed && S->pressed.id != P_GRIP && S->pressed.id != P_EXPAND) {
                canvas_invert(&a->screen, S->pressed.r);
                app_invalidate(a, S->pressed.r, REFRESH_UI);
            }
            S->has_pressed = false;
            S->dragging = false;
            S->owner = OWN_NONE;
        }
        if (S->owner != OWN_NONE || S->paper_open) return;
        S->pinching = true;
        S->pinch_d0 = d > 10.f ? d : 10.f;
        S->pinch_z0 = S->zoom;
        to_page(a, mx, my, &S->pinch_px, &S->pinch_py);
        S->last_pinch_paint = 0;
        return;
    }
    if (!S->pinching) return;
    if (ev->type == IN_PINCH_MOVE) {
        S->zoom = S->pinch_z0 * d / S->pinch_d0;
        S->zoom = clampf(S->zoom, S->zoom_min, S->zoom_default * 4.f);
        S->ox = S->pinch_px - mx / S->zoom;  // keep the pinched spot under the fingers
        S->oy = S->pinch_py - my / S->zoom;
        view_clamp(a);
        if (ev->t_ms - S->last_pinch_paint > 180) {
            S->last_pinch_paint = ev->t_ms;
            compose_all(a, REFRESH_FAST);
        }
        return;
    }
    S->pinching = false;  // IN_PINCH_END
    compose_all(a, REFRESH_FULL);
}

void editor_leave(App *a) {
    // Leaving for pen settings / calibration keeps the notebook open; just persist.
    if (S->owner == OWN_DRAW) draw_end(a);
    if (S->owner == OWN_ERASE) erase_end(a);
    save_page(a);
}

void editor_event(App *a, const InEvent *ev) {
    if (!S->open) return;
    if (ev->type == IN_PEN_BUTTON) {
        pen_button(a, ev);
        return;
    }
    if (ev->type == IN_PINCH_BEGIN || ev->type == IN_PINCH_MOVE || ev->type == IN_PINCH_END) {
        pinch(a, ev);
        return;
    }
    bool pen = ev->type == IN_PEN_DOWN || ev->type == IN_PEN_MOVE || ev->type == IN_PEN_UP ||
               ev->type == IN_PEN_LEAVE;
    // A pen leaving proximity mid-gesture ends it.
    InType type = ev->type == IN_PEN_LEAVE ? IN_PEN_UP : ev->type;
    int ph = (type == IN_PEN_UP) ? 2 : app_pointer_phase(ev);
    if (ph < 0) return;
    if (ph == 0) {
        if (S->owner != OWN_NONE || S->pinching) return;  // one gesture at a time
        S->owner_is_pen = pen;
        if (S->dm.open) {
            S->owner = OWN_DISPLAY;
            Rect d;
            dispmenu_pointer(&S->dm, a, 0, ev->x, ev->y, ev->t_ms, &d);
            return;
        }
        if (zoomed(a) && rect_contains(zoom_chip(a), (int)ev->x, (int)ev->y) && !S->paper_open &&
            !rect_contains(S->panel, (int)ev->x, (int)ev->y)) {
            view_reset(a);
            compose_all(a, REFRESH_FULL);
            S->owner = OWN_MODAL;  // swallow the rest of this tap
            S->has_pressed = false;
            return;
        }
        if (S->paper_open) {
            if (!rect_contains(S->paper_rect, (int)ev->x, (int)ev->y)) {
                // Tapping outside the sheet dismisses it.
                S->paper_open = false;
                compose(a, S->paper_rect);
                app_invalidate(a, S->paper_rect, REFRESH_UI);
                S->owner = OWN_MODAL;  // swallow the rest of this gesture
                S->has_pressed = false;
                return;
            }
            S->owner = OWN_MODAL;
            ui_pointer(a, &S->modal_hits, 0, ev, true);
        } else if (rect_contains(S->panel, (int)ev->x, (int)ev->y)) {
            S->owner = OWN_PANEL;
            ui_pointer(a, &S->hits, 0, ev, false);
        } else if (pen) {
            draw_begin(a, ev);
            if (S->owner == OWN_DRAW) draw_sample(a, ev);
        }
        return;
    }
    if (S->owner == OWN_NONE || pen != S->owner_is_pen) return;
    switch (S->owner) {
    case OWN_DRAW:
        if (ph == 1) draw_sample(a, ev);
        else draw_end(a);
        break;
    case OWN_ERASE:
        erase_segment(a, ev->x, ev->y, ev->pressure);
        if (ph == 2) erase_end(a);
        break;
    case OWN_DISPLAY: {
        Rect d;
        DmResult r = dispmenu_pointer(&S->dm, a, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == DM_INVERTED) {
            compose_all(a, REFRESH_FULL);
        } else if (r == DM_CLOSED) {
            compose(a, d);
            app_invalidate(a, d, REFRESH_UI);
        } else if (!rect_is_empty(d)) {
            app_invalidate(a, d, ph == 2 ? REFRESH_UI : REFRESH_FAST);
        }
        if (ph == 2) S->owner = OWN_NONE;
        break;
    }
    case OWN_PANEL:
    case OWN_MODAL:
        ui_pointer(a, S->owner == OWN_MODAL ? &S->modal_hits : &S->hits, ph, ev, S->owner == OWN_MODAL);
        if (ph == 2) S->owner = OWN_NONE;
        break;
    default: break;
    }
}

void editor_tick(App *a, uint32_t now) {
    // Holding the button ~1.2 s while hovering (no ink) clears the page once.
    if (S->open && S->btn_down && !S->btn_used_for_ink && !S->btn_hold_fired && S->owner == OWN_NONE &&
        !S->paper_open && now - S->btn_down_at >= 1200) {
        S->btn_hold_fired = true;
        S->nclicks = 0;
        clear_page(a);
    }
    if (S->open && S->page_dirty && S->owner == OWN_NONE && now - S->last_up > 1500) save_page(a);
}
