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
#include "bgjob.h"
#include "musicbuild.h"
#include "history.h"

enum {
    P_NONE, P_TOOL, P_UNDO, P_REDO, P_PREV, P_NEXT, P_PAPER, P_PENSET, P_LIBRARY,
    P_COLLAPSE, P_GRIP, P_EXPAND, P_PICK_PAPER, P_PICK_CUSTOM, P_PICK_CANCEL, P_CLEAR, P_DISPLAY, P_PAGES,
    PG_BACK, PG_DELETE, PG_ALL, PG_TOGGLE, PG_OPEN, PG_PREV, PG_NEXT, P_TOOLPICK, P_PICK_TOOL,
};

typedef enum { OWN_NONE, OWN_DRAW, OWN_ERASE, OWN_PANEL, OWN_MODAL, OWN_DISPLAY, OWN_PAGES, OWN_TOOLS, OWN_BUILD } Owner;

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
    bool thumb_dirty;           // page 1 changed: library thumbnail to write when leaving it
    // Grays settle in one GL16 refresh once the writer pauses (not after every stroke): while
    // strokes come in quick succession only fast DU updates go out, nothing blocks.
    Rect settle;                // page rect still showing dithered live ink
    uint32_t settle_at;
    bool panel_stale;           // Undo/Redo/Clear availability changed: repaint at the settle
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
    bool pinching, pinch_committed;  // committed once the fingers clearly spread or move
    float pinch_d0, pinch_z0, pinch_px, pinch_py;  // start spread/zoom, page point under midpoint
    float pinch_mx0, pinch_my0;
    uint32_t last_pinch_paint;

    DispMenu dm;                // Display sheet (front light, invert)

    // Tool picker (opened from the toolbar's tool button).
    bool tools_open;
    Rect tools_rect;
    HitList tools_hits;
    Press tools_press;

    // Pages view: page previews with checkboxes; delete the ticked ones.
    bool pages_open;
    int pages_screen;           // which screenful of previews
    bool *pages_sel;            // per page index
    int pages_sel_cap;
    HitList pages_hits;
    Press pages_press;
    struct PageThumb {
        uint32_t id;
        Canvas c;
    } *pthumbs;
    int npthumbs;

    // Paper sheet
    bool paper_open;
    Rect paper_rect;
    HitList modal_hits;
    Canvas paper_prev[PAPER_COUNT];
    Canvas custom_prev;         // the Custom music tile's preview (one layout at a time)
    PaperKind custom_prev_k;
    MusicBuild mb;              // the Custom music sheet
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
    // header, 11 buttons (tool, eraser, undo, redo, clear, prev, next, paper, display, pen feel,
    // library), the page label and 3 dividers. Must match draw_panel.
    return head_h() + gap() + 11 * (btn_h() + gap()) + label_h() + gap() + 3 * div_h() + ui_px(6);
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
    if (S->collapsed) {
        ui_frame(c, p, ui_px(22), 255, true, 0);
        ui_frame(c, p, ui_px(22), 0, false, ui_px(6));
        Rect ir = rect_inflate(p, -ui_px(20));
        ir.h -= ui_px(12);
        ui_icon(c, ui_tool_icon(a->settings.current_tool), ir, 0);
        // Expand hint in the corner.
        ui_icon(c, ICON_EXPAND, (Rect){p.x + p.w - ui_px(40), p.y + p.h - ui_px(40), ui_px(30), ui_px(30)}, 0);
        hits_add(&S->hits, p, P_EXPAND, 0);
        return;
    }
    ui_frame(c, p, ui_px(22), 255, true, 0);
    ui_frame(c, p, ui_px(22), 0, false, ui_px(6));
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
    // The drawing tool (tap: the picker) and the eraser.
    ToolKind dt = a->settings.draw_tool == TOOL_ERASER ? TOOL_PENCIL : a->settings.draw_tool;
    Rect rt = {x, y, w, btn_h()};
    ui_button(c, rt, ui_tool_icon(dt), NULL, a->settings.current_tool == dt ? BTN_SELECTED : BTN_PLAIN);
    // A small corner mark says "more tools here".
    canvas_disc_aa(c, (float)(rt.x + rt.w - ui_px(16)), (float)(rt.y + rt.h - ui_px(16)), (float)ui_px(7),
                   a->settings.current_tool == dt ? 255 : 0, 1.f);
    hits_add(&S->hits, rt, P_TOOLPICK, 0);
    y += btn_h() + gap();
    Rect re = {x, y, w, btn_h()};
    ui_button(c, re, ICON_ERASER, NULL, tool_style(a, TOOL_ERASER));
    hits_add(&S->hits, re, P_TOOL, TOOL_ERASER);
    y += btn_h() + gap();
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
    Rect lr = {x - ui_px(6), y, w + ui_px(12), label_h()};
    ui_frame(c, lr, ui_px(10), 0, false, ui_px(2));  // a tappable chip: opens Pages
    font_draw_centered(c, FONT_BOLD, ui_small_size() * (g_pscale < 1.f ? g_pscale : 1.f), lr, lab, 0);
    hits_add(&S->hits, lr, P_PAGES, 0);
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
    const int ntiles = PAPER_PICKER_COUNT;
    int m = ui_px(70), gap_ = ui_px(30), cols = ntiles > 15 ? 6 : ntiles > 8 ? 5 : 4, rows = (ntiles + cols - 1) / cols;
    int tw = (a->w - 2 * m - ui_px(72) - (cols - 1) * gap_) / cols;
    if (tw > a->w / 6) tw = a->w / 6;
    int max_th = (a->h - ui_px(120) - ui_px(72) - ui_px(190) - ui_px(80)) / rows - ui_px(100);
    if (tw * 4 / 3 > max_th) tw = max_th * 3 / 4;
    int th = tw * 4 / 3, cell_h = th + ui_px(100);
    int gridw = cols * tw + (cols - 1) * gap_;
    int sheet_h = ui_px(120) + ui_px(72) + rows * cell_h + ui_px(190);
    S->paper_rect = (Rect){m, (a->h - sheet_h) / 3, a->w - 2 * m, sheet_h};
    Rect in = ui_sheet(c, S->paper_rect, "Paper for this page");
    for (int i = 0; i < ntiles; i++) {
        // The last tile is Custom music: it shows this page's layout, or the last one built.
        bool custom = i == PAPER_COUNT;
        bool page_custom = S->page && paper_is_custom(S->page->paper);
        PaperKind k = custom ? (page_custom ? S->page->paper : a->settings.music_layout) : paper_at(i);
        Rect tr = {in.x + (in.w - gridw) / 2 + (i % cols) * (tw + gap_), in.y + (i / cols) * cell_h, tw, th};
        canvas_fill(c, tr, 255);
        const Canvas *pv = paper_preview(a, k);
        if (pv) canvas_blit_fit(c, pv, tr);
        bool sel = custom ? page_custom : (S->page && S->page->paper == k);
        ui_frame(c, rect_inflate(tr, sel ? ui_px(10) : 0), ui_px(12), 0, false, sel ? ui_px(9) : ui_px(3));
        font_draw_centered(c, sel ? FONT_BOLD : FONT_REGULAR, ui_small_size() * 0.9f,
                           (Rect){tr.x - gap_ / 2, tr.y + th + ui_px(16), tw + gap_, ui_px(64)},
                           custom ? "Custom music..." : paper_name(k), 0);
        hits_add(&S->modal_hits, (Rect){tr.x, tr.y, tw, th + ui_px(90)}, custom ? P_PICK_CUSTOM : P_PICK_PAPER, (int)k);
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

// ---- tool picker ----

// Dry media, then pens, then the nibs and the highlighter (three to a row).
static const ToolKind k_draw_tools[] = {TOOL_PENCIL,    TOOL_MECH_PENCIL, TOOL_CHARCOAL,   TOOL_BALLPOINT, TOOL_FINELINER,
                                        TOOL_MARKER,    TOOL_FOUNTAIN,    TOOL_CALLIGRAPHY, TOOL_HIGHLIGHTER};
#define N_DRAW_TOOLS ((int)(sizeof k_draw_tools / sizeof k_draw_tools[0]))

static void draw_tools(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->tools_hits);
    int bw = ui_px(300), bh = ui_px(150), g = ui_px(16), cols = 3, rows = (N_DRAW_TOOLS + cols - 1) / cols;
    int w = cols * bw + (cols - 1) * g + 2 * ui_px(30), h = ui_px(110) + rows * bh + (rows - 1) * g + ui_px(40);
    int x = S->panel.x - w - ui_px(20);
    if (x < ui_px(10)) x = S->panel.x + S->panel.w + ui_px(20);
    int y = clampi(S->panel.y, ui_px(10), a->h - h - ui_px(10));
    S->tools_rect = (Rect){x, y, w, h};
    ui_frame(c, S->tools_rect, ui_px(22), 255, true, 0);
    ui_frame(c, S->tools_rect, ui_px(22), 0, false, ui_px(6));
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){x + ui_px(30), y, w - ui_px(60), ui_px(100)}, "Tools", 0);
    for (int i = 0; i < N_DRAW_TOOLS; i++) {
        ToolKind t = k_draw_tools[i];
        Rect r = {x + ui_px(30) + (i % cols) * (bw + g), y + ui_px(100) + (i / cols) * (bh + g), bw, bh};
        const char *name = t == TOOL_MECH_PENCIL ? "Mechanical" : t == TOOL_CALLIGRAPHY ? "Calligraphy" : tool_name(t);
        ui_button(c, r, ui_tool_icon(t), name, a->settings.current_tool == t ? BTN_SELECTED : BTN_PLAIN);
        hits_add(&S->tools_hits, r, P_PICK_TOOL, t);
    }
}

// ---- pages view ----

static void pthumbs_clear(App *a) {
    for (int i = 0; i < S->npthumbs; i++) canvas_free(&S->pthumbs[i].c);
    free(S->pthumbs);
    S->pthumbs = NULL;
    S->npthumbs = 0;
}

static Canvas *pthumb_slot(App *a, uint32_t id) {
    for (int i = 0; i < S->npthumbs; i++)
        if (S->pthumbs[i].id == id) return &S->pthumbs[i].c;
    struct PageThumb *n = realloc(S->pthumbs, (size_t)(S->npthumbs + 1) * sizeof *n);
    if (!n) return NULL;
    S->pthumbs = n;
    S->pthumbs[S->npthumbs] = (struct PageThumb){id, {0}};
    return &S->pthumbs[S->npthumbs++].c;
}

// Preview of page `index` (1/4 size), rendered on first use and cached for the session.
static const Canvas *pthumb(App *a, int index) {
    Canvas *t = pthumb_slot(a, S->nb.page_ids[index]);
    if (!t) return NULL;
    if (t->px) return t;
    if (canvas_init(t, S->pgw / 4, S->pgh / 4) != 0) return NULL;
    if (index == S->index) {
        canvas_downsample_keep_lines(t, &S->page_canvas, 0.35f);
        return t;
    }
    // Pages visited this session are in memory; others are read just for the preview.
    Page *p = NULL;
    bool temp = false;
    for (int i = 0; i < S->nhist; i++)
        if (S->hist[i].page_id == S->nb.page_ids[index]) p = S->hist[i].page;
    if (!p) {
        if (store_load_page(&S->nb, index, S->pgw, S->pgh, &p) != 0) return t;
        temp = true;
    }
    Canvas full;
    if (canvas_init(&full, S->pgw, S->pgh) == 0) {
        page_render(&full, p, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
        canvas_downsample_keep_lines(t, &full, 0.35f);
        canvas_free(&full);
    }
    if (temp) page_free(p);
    return t;
}

static void pages_geom(App *a, int *cols, int *rows, int *cw, int *ch, int *th) {
    *cols = a->w > a->h ? 5 : 3;
    int m = ui_px(50);
    *cw = (a->w - m * (*cols + 1)) / *cols;
    *th = *cw * S->pgh / S->pgw;
    *ch = *th + ui_px(90);
    *rows = (a->h - ui_px(190) - ui_px(170) - m) / (*ch + m);
    if (*rows < 1) *rows = 1;
}

static int pages_selected(App *a) {
    int k = 0;
    for (int i = 0; i < S->nb.npages && i < S->pages_sel_cap; i++) k += S->pages_sel[i];
    return k;
}

static void pages_draw(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->pages_hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(50), th0 = ui_px(190), bh = ui_px(150), by = (th0 - bh) / 2;
    Rect back = {m, by, bh, bh};
    ui_button(c, back, ICON_BACK, NULL, BTN_PLAIN);
    hits_add(&S->pages_hits, back, PG_BACK, 0);
    int k = pages_selected(a);
    char del[48];
    snprintf(del, sizeof del, k ? "Delete (%d)" : "Delete", k);
    Rect rdel = {a->w - m - ui_px(360), by, ui_px(360), bh};
    ui_button(c, rdel, ICON_TRASH, del, k ? BTN_PRIMARY : BTN_DISABLED);
    if (k) hits_add(&S->pages_hits, rdel, PG_DELETE, 0);
    Rect rall = {rdel.x - ui_px(20) - ui_px(360), by, ui_px(360), bh};
    bool all = k == S->nb.npages;
    ui_button(c, rall, ICON_CHECK, all ? "Select none" : "Select all", BTN_PLAIN);
    hits_add(&S->pages_hits, rall, PG_ALL, 0);
    char title[48];
    snprintf(title, sizeof title, "Pages (%d)", S->nb.npages);
    font_draw_left(c, FONT_BOLD, ui_title_size() * 1.15f,
                   (Rect){back.x + bh + ui_px(30), 0, rall.x - back.x - bh - ui_px(50), th0}, title, 0);
    ui_rule(c, 0, a->w, th0, ui_px(4), 0);
    int cols, rows, cw, ch, th;
    pages_geom(a, &cols, &rows, &cw, &ch, &th);
    int per = cols * rows, screens = (S->nb.npages + per - 1) / per;
    if (screens < 1) screens = 1;
    S->pages_screen = clampi(S->pages_screen, 0, screens - 1);
    for (int n = 0; n < per; n++) {
        int i = S->pages_screen * per + n;
        if (i >= S->nb.npages) break;
        Rect tr = {m + (n % cols) * (cw + m), th0 + m + (n / cols) * (ch + m), cw, th};
        const Canvas *t = pthumb(a, i);
        if (t && t->px) {
            Rect cl = rect_intersect(tr, (Rect){0, 0, a->w, a->h});
            for (int yy = cl.y; yy < cl.y + cl.h; yy++) {
                int sy = (yy - tr.y) * t->h / tr.h;
                uint8_t *dst = &c->px[(size_t)yy * c->stride];
                const uint8_t *src = &t->px[(size_t)sy * t->stride];
                for (int xx = cl.x; xx < cl.x + cl.w; xx++) dst[xx] = src[(xx - tr.x) * t->w / tr.w];
            }
        }
        bool cur = i == S->index, sel = i < S->pages_sel_cap && S->pages_sel[i];
        ui_frame(c, rect_inflate(tr, cur ? ui_px(8) : 0), ui_px(12), 0, false, cur ? ui_px(8) : ui_px(3));
        Rect cb = {tr.x + tr.w - ui_px(96), tr.y + ui_px(16), ui_px(80), ui_px(80)};
        ui_frame(c, cb, ui_px(12), sel ? 0 : 255, true, 0);
        ui_frame(c, cb, ui_px(12), 0, false, ui_px(5));
        if (sel) ui_icon(c, ICON_CHECK, cb, 255);
        char lab[32];
        snprintf(lab, sizeof lab, cur ? "Page %d (open)" : "Page %d", i + 1);
        font_draw_centered(c, cur ? FONT_BOLD : FONT_REGULAR, ui_text_size(),
                           (Rect){tr.x, tr.y + th + ui_px(12), tr.w, ui_px(70)}, lab, 0);
        hits_add(&S->pages_hits, tr, PG_OPEN, i);
        hits_add(&S->pages_hits, rect_inflate(cb, ui_px(30)), PG_TOGGLE, i);
    }
    int y = a->h - ui_px(170);
    canvas_hline(c, 0, a->w, y, ui_px(3), 0);
    Rect pr = {m, y + ui_px(15), ui_px(220), ui_px(140)}, nr = {a->w - m - ui_px(220), y + ui_px(15), ui_px(220), ui_px(140)};
    ui_button(c, pr, ICON_PREV, NULL, S->pages_screen > 0 ? BTN_PLAIN : BTN_DISABLED);
    ui_button(c, nr, ICON_NEXT, NULL, S->pages_screen < screens - 1 ? BTN_PLAIN : BTN_DISABLED);
    if (S->pages_screen > 0) hits_add(&S->pages_hits, pr, PG_PREV, 0);
    if (S->pages_screen < screens - 1) hits_add(&S->pages_hits, nr, PG_NEXT, 0);
    char pl[64];
    snprintf(pl, sizeof pl, "Tick pages, then Delete.   %d of %d", S->pages_screen + 1, screens);
    font_draw_centered(c, FONT_REGULAR, ui_small_size(), (Rect){pr.x + pr.w, pr.y, nr.x - pr.x - pr.w, pr.h}, pl, 60);
}

// Rebuild a screen rect from the layers.
static void compose(App *a, Rect r) {
    if (S->pages_open) {  // the Pages view covers everything
        pages_draw(a);
        return;
    }
    r = rect_intersect(r, (Rect){0, 0, a->w, a->h});
    if (rect_is_empty(r)) return;
    if (view_identity(a)) canvas_blit(&a->screen, &S->page_canvas, r, r.x, r.y);
    else blit_view(a, r);
    if (zoomed(a) && rect_overlaps(r, zoom_chip(a))) {
        char z[48];
        snprintf(z, sizeof z, "%d%%   tap to reset", (int)lrintf(S->zoom / S->zoom_default * 100.f));
        Rect c = zoom_chip(a);
        ui_frame(&a->screen, c, c.h / 2, 0, true, 0);
        font_draw_centered(&a->screen, FONT_BOLD, ui_small_size(), c, z, 255);
    }
    if (rect_overlaps(r, S->panel)) draw_panel(a);
    if (S->paper_open && rect_overlaps(r, S->paper_rect)) draw_paper_sheet(a);
    if (S->dm.open && rect_overlaps(r, S->dm.rect)) dispmenu_draw(&S->dm, a);
    if (S->mb.open && rect_overlaps(r, S->mb.rect)) musicbuild_draw(&S->mb, a);
    if (S->tools_open && rect_overlaps(r, S->tools_rect)) draw_tools(a);
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

static void settle_now(App *a);
static void write_thumb_if_needed(App *a);

static void save_thumb_from_canvas(App *a) {
    Canvas t;
    if (canvas_init(&t, S->pgw / 3, S->pgh / 3) != 0) return;
    canvas_downsample_keep_lines(&t, &S->page_canvas, 0.35f);
    store_save_thumb(S->nb.id, &t);
    canvas_free(&t);
}

// False if the page could not be written; it then stays dirty (and in memory), so callers
// that would drop it (turning the page, closing) must stay put instead.
static bool save_page(App *a) {
    if (!S->open) return true;
    if (S->page_dirty) {
        if (store_save_page(&S->nb, S->index, S->page) != 0) {
            app_toast(a, "Could not save this page. Is the storage full?", 5000);
            return false;
        }
        if (S->index == 0) S->thumb_dirty = true;  // written when leaving the page (it is slow)
        S->page_dirty = false;
        S->info_dirty = true;
    }
    if (S->info_dirty) {
        if (store_save_info(&S->nb) != 0) return false;
        S->info_dirty = false;
    }
    return true;
}

static void render_page_full(App *a) {
    page_render(&S->page_canvas, S->page, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
    S->canvas_page = S->page;
}

static void go_to_page(App *a, int index) {
    S->settle = RECT_EMPTY;  // the whole screen is about to be redrawn anyway
    S->panel_stale = false;
    if (!save_page(a)) return;
    write_thumb_if_needed(a);
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
    write_thumb_if_needed(a);
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
        bg_mirror_pdf(S->nb.id, a->dpi, app_mirror_dir(), false);
    }
    free_histories(a);
    pthumbs_clear(a);
    S->pages_open = false;
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
    // Settle later, once the writer pauses (see settle_now).
    S->settle = rect_union(S->settle, s->bbox);
    S->settle_at = now_ms() + 450;
    mark_changed(a);
    // Only touch the panel when Undo/Redo availability actually changed.
    if (!had_undo || had_redo || was_empty) S->panel_stale = true;
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
        if (S->owner == OWN_NONE && !S->paper_open && !S->mb.open && !S->pages_open) clear_page(a);
    }
}

static void undo_redo(App *a, bool undo) {
    settle_now(a);
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

// ---- pages view: open, act, delete ----

static void pages_show(App *a) {
    save_page(a);
    if (S->pages_sel_cap < S->nb.npages) {
        bool *n = realloc(S->pages_sel, (size_t)S->nb.npages * sizeof *n);
        if (!n) return;
        S->pages_sel = n;
        S->pages_sel_cap = S->nb.npages;
    }
    memset(S->pages_sel, 0, (size_t)S->pages_sel_cap * sizeof *S->pages_sel);
    // The open page may have changed since its preview was made.
    for (int i = 0; i < S->npthumbs; i++)
        if (S->pthumbs[i].id == S->nb.page_ids[S->index]) canvas_free(&S->pthumbs[i].c);
    S->pages_open = true;
    int cols, rows, cw, ch, th;
    pages_geom(a, &cols, &rows, &cw, &ch, &th);
    S->pages_screen = S->index / (cols * rows);
    compose_all(a, REFRESH_FULL);
}

static void pages_hide(App *a, int open_index) {
    S->pages_open = false;
    if (open_index >= 0 && open_index != S->index) go_to_page(a, open_index);
    else compose_all(a, REFRESH_FULL);
}

// Removes a page's in-memory copy and history (the page is being deleted).
static void forget_page(App *a, uint32_t pid) {
    for (int i = 0; i < S->nhist; i++) {
        if (S->hist[i].page_id != pid) continue;
        if (S->hist[i].page == S->page) S->page = NULL;
        history_free(&S->hist[i].h);
        page_free(S->hist[i].page);
        S->hist[i] = S->hist[--S->nhist];
        return;
    }
}

static void pages_delete(App *a) {
    int n = S->nb.npages, k = pages_selected(a);
    if (k == 0) return;
    if (k == n) {
        // A notebook keeps at least one page: add a fresh blank one to stay behind.
        if (store_add_page(&S->nb, n, S->page ? S->page->paper : S->nb.paper) != 0) {
            app_toast(a, "Could not delete those pages", 2500);
            return;
        }
    }
    int cur = S->index, deleted_before = 0;
    for (int i = n - 1; i >= 0; i--) {
        if (i >= S->pages_sel_cap || !S->pages_sel[i]) continue;
        uint32_t pid = S->nb.page_ids[i];
        if (store_delete_page(&S->nb, i) != 0) continue;  // the file goes to grimoire/trash
        forget_page(a, pid);
        if (i < cur) deleted_before++;
    }
    pthumbs_clear(a);
    memset(S->pages_sel, 0, (size_t)S->pages_sel_cap * sizeof *S->pages_sel);
    S->index = clampi(cur - deleted_before, 0, S->nb.npages - 1);
    S->nb.last_page = S->index;
    S->page = page_at(a, S->index);
    if (S->page) render_page_full(a);
    S->changed = true;
    S->page_dirty = S->index == 0;  // refreshes the library thumbnail when page 1 changed
    S->info_dirty = true;
    save_page(a);
    char msg[64];
    snprintf(msg, sizeof msg, k == 1 ? "Deleted 1 page (kept in grimoire/trash)" : "Deleted %d pages (kept in grimoire/trash)", k);
    compose_all(a, REFRESH_FULL);
    app_toast(a, msg, 2500);
}

static void pages_event(App *a, int ph, const InEvent *ev) {
    if (ph == 0) {
        press_begin(&S->pages_press, ev->x, ev->y, ev->t_ms);
        return;
    }
    press_move(&S->pages_press, ev->x, ev->y);
    if (ph == 1) return;
    const Hit *h = hits_find(&S->pages_hits, S->pages_press.x0, S->pages_press.y0);
    if (!h || !press_is_tap(&S->pages_press, h->r)) return;
    Hit hit = *h;
    switch (hit.id) {
    case PG_BACK: pages_hide(a, -1); break;
    case PG_OPEN: pages_hide(a, hit.arg); break;
    case PG_TOGGLE:
        if (hit.arg < S->pages_sel_cap) S->pages_sel[hit.arg] = !S->pages_sel[hit.arg];
        pages_draw(a);
        app_invalidate_all(a, REFRESH_UI);
        break;
    case PG_ALL: {
        bool all = pages_selected(a) == S->nb.npages;
        for (int i = 0; i < S->nb.npages && i < S->pages_sel_cap; i++) S->pages_sel[i] = !all;
        pages_draw(a);
        app_invalidate_all(a, REFRESH_UI);
        break;
    }
    case PG_DELETE: pages_delete(a); break;
    case PG_PREV:
    case PG_NEXT:
        S->pages_screen += hit.id == PG_NEXT ? 1 : -1;
        compose_all(a, REFRESH_FULL);
        break;
    default: break;
    }
}

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
    case P_TOOLPICK:
        S->tools_open = true;
        draw_tools(a);
        app_invalidate(a, S->tools_rect, REFRESH_UI);
        break;
    case P_TOOL:
        if (a->settings.current_tool == (ToolKind)h->arg) {
            // Tapping the active eraser again goes back to the drawing tool.
            if (h->arg != TOOL_ERASER) break;
            a->settings.current_tool = a->settings.draw_tool;
            settings_save(&a->settings);
            repaint_panel(a, REFRESH_UI);
            break;
        }
        a->settings.current_tool = (ToolKind)h->arg;
        settings_save(&a->settings);
        repaint_panel(a, REFRESH_UI);
        break;
    case P_UNDO: undo_redo(a, true); break;
    case P_REDO: undo_redo(a, false); break;
    case P_PREV: go_to_page(a, S->index - 1); break;
    case P_NEXT:
        if (S->index >= S->nb.npages - 1) {
            if (!save_page(a)) return;
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
    case P_PAGES: pages_show(a); break;
    case P_DISPLAY:
        dispmenu_open(&S->dm, a);
        app_invalidate(a, dispmenu_draw(&S->dm, a), REFRESH_UI);
        break;
    case P_PENSET: pensettings_open(a, a->settings.current_tool); break;
    case P_LIBRARY:
        app_toast(a, "Saving...", 8000);
        app_flush(a);
        if (!save_page(a)) return;  // stay with the unsaved page rather than drop it
        close_notebook(a);
        app_switch(a, SCR_LIBRARY);
        app_toast(a, "Saved", 1200);
        break;
    case P_COLLAPSE: set_collapsed(a, true); break;
    case P_EXPAND: set_collapsed(a, false); break;
    default: break;
    }
}

// Changes this page's paper (undoable) and repaints.
static void apply_paper(App *a, PaperKind k) {
    History *hi = cur_hist(a);
    if (hi) history_push_paper(hi, S->page->paper, k);
    S->page->paper = k;
    render_page_full(a);
    compose_all(a, REFRESH_FULL);
    mark_changed(a);
}

static void modal_act(App *a, const Hit *h) {
    Rect sheet = S->paper_rect;
    S->paper_open = false;
    if (h->id == P_PICK_PAPER && S->page->paper != (PaperKind)h->arg) {
        apply_paper(a, (PaperKind)h->arg);
        return;
    }
    if (h->id == P_PICK_CUSTOM) {
        compose(a, sheet);
        musicbuild_open(&S->mb, a, S->page->paper);
        Rect r = rect_union(sheet, musicbuild_draw(&S->mb, a));
        app_invalidate(a, r, REFRESH_UI);
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
    pthumbs_clear(a);
    free(S->pages_sel);
    canvas_free(&S->page_canvas);
    for (int k = 0; k < PAPER_COUNT; k++) canvas_free(&S->paper_prev[k]);
    canvas_free(&S->custom_prev);
    musicbuild_free(&S->mb);
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
    S->mb.open = false;
    S->tools_open = false;
    compose(a, (Rect){0, 0, a->w, a->h});
}

bool editor_busy(App *a) { return S && (S->owner != OWN_NONE || S->pinching); }

const Canvas *editor_page_canvas(App *a) {
    return (S && S->open && S->page_canvas.px) ? &S->page_canvas : NULL;
}

bool editor_pin_sleep_page(App *a) {
    const Canvas *c = editor_page_canvas(a);
    return c && store_save_sleep_image(c) == 0;
}

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
        if (S->owner != OWN_NONE || S->paper_open || S->mb.open || !a->settings.pinch_zoom) return;
        S->pinching = true;
        S->pinch_committed = false;
        S->pinch_mx0 = mx;
        S->pinch_my0 = my;
        S->pinch_d0 = d > 10.f ? d : 10.f;
        S->pinch_z0 = S->zoom;
        to_page(a, mx, my, &S->pinch_px, &S->pinch_py);
        S->last_pinch_paint = 0;
        return;
    }
    if (!S->pinching) return;
    if (ev->type == IN_PINCH_MOVE) {
        // Nothing happens until the fingers clearly spread/close or travel: resting fingers
        // (or a hand) must not zoom or flash the screen.
        if (!S->pinch_committed) {
            float spread = fabsf(d / S->pinch_d0 - 1.f), travel = hypotf(mx - S->pinch_mx0, my - S->pinch_my0);
            if (spread < 0.10f && travel < (float)ui_px(70)) return;
            S->pinch_committed = true;
            // Re-anchor at the commit point so the view does not jump by the dead zone.
            S->pinch_d0 = d > 10.f ? d : 10.f;
            S->pinch_z0 = S->zoom;
            to_page(a, mx, my, &S->pinch_px, &S->pinch_py);
        }
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
    // Settle without a flash, and only if the view actually changed.
    if (S->pinch_committed) compose_all(a, REFRESH_UI);
}

void editor_leave(App *a) {
    // Leaving for pen settings / calibration keeps the notebook open; just persist.
    if (S->owner == OWN_DRAW) draw_end(a);
    if (S->owner == OWN_ERASE) erase_end(a);
    save_page(a);
    write_thumb_if_needed(a);
}

void editor_event(App *a, const InEvent *ev) {
    if (!S->open) return;
    if (ev->type == IN_PEN_BUTTON) {
        pen_button(a, ev);
        return;
    }
    if (ev->type == IN_PINCH_BEGIN || ev->type == IN_PINCH_MOVE || ev->type == IN_PINCH_END) {
        if (!S->pages_open) pinch(a, ev);
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
        if (S->pages_open) {
            S->owner = OWN_PAGES;
            pages_event(a, 0, ev);
            return;
        }
        if (S->tools_open) {
            S->owner = OWN_TOOLS;
            press_begin(&S->tools_press, ev->x, ev->y, ev->t_ms);
            return;
        }
        if (S->mb.open) {
            S->owner = OWN_BUILD;
            Rect d;
            musicbuild_pointer(&S->mb, a, 0, ev->x, ev->y, ev->t_ms, &d);
            return;
        }
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
    case OWN_PAGES:
        pages_event(a, ph, ev);
        if (ph == 2) S->owner = OWN_NONE;
        break;
    case OWN_TOOLS:
        press_move(&S->tools_press, ev->x, ev->y);
        if (ph != 2) break;
        S->owner = OWN_NONE;
        {
            const Hit *h = hits_find(&S->tools_hits, S->tools_press.x0, S->tools_press.y0);
            if (h && press_is_tap(&S->tools_press, h->r)) {
                a->settings.current_tool = a->settings.draw_tool = (ToolKind)h->arg;
                settings_save(&a->settings);
            }
            // Picked or tapped outside: close it.
            S->tools_open = false;
            compose(a, S->tools_rect);
            app_invalidate(a, S->tools_rect, REFRESH_UI);
            repaint_panel(a, REFRESH_UI);
        }
        break;
    case OWN_BUILD: {
        Rect d;
        MbResult r = musicbuild_pointer(&S->mb, a, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == MB_DONE) {
            compose(a, d);
            PaperKind k = musicbuild_result(&S->mb);
            if (S->page->paper != k) apply_paper(a, k);
            else app_invalidate(a, d, REFRESH_UI);
        } else if (r == MB_CANCEL) {
            compose(a, d);
            app_invalidate(a, d, REFRESH_UI);
        } else if (!rect_is_empty(d)) {
            app_invalidate(a, d, REFRESH_UI);
        }
        if (ph == 2) S->owner = OWN_NONE;
        break;
    }
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

// The deferred settle: one GL16 refresh for everything written since the last pause.
static void settle_now(App *a) {
    if (!rect_is_empty(S->settle)) show(a, S->settle, REFRESH_UI);
    S->settle = RECT_EMPTY;
    if (S->panel_stale) repaint_panel(a, REFRESH_UI);
    S->panel_stale = false;
}

static void write_thumb_if_needed(App *a) {
    if (!S->thumb_dirty || S->index != 0) return;
    save_thumb_from_canvas(a);
    S->thumb_dirty = false;
}

void editor_tick(App *a, uint32_t now) {
    if (S->open && S->owner == OWN_NONE && (!rect_is_empty(S->settle) || S->panel_stale) &&
        (int32_t)(now - S->settle_at) >= 0)
        settle_now(a);
    // Holding the button ~1.2 s while hovering (no ink) clears the page once.
    if (S->open && S->btn_down && !S->btn_used_for_ink && !S->btn_hold_fired && S->owner == OWN_NONE &&
        !S->paper_open && !S->mb.open && !S->pages_open && now - S->btn_down_at >= 1200) {
        S->btn_hold_fired = true;
        S->nclicks = 0;
        clear_page(a);
    }
    // Autosave after a real pause (and never while the pen is down).
    if (S->open && S->page_dirty && S->owner == OWN_NONE && now - S->last_up > 2000) save_page(a);
}

// ---- headless checks ----

// The script runner's "editor.*" keys (app_probe forwards them without the prefix).
int editor_probe(App *a, const char *key, char *out, size_t cap) {
    if (!S) return -1;
    bool has_page = S->open && S->page;
    if (!strcmp(key, "page")) {
        snprintf(out, cap, "%d", S->open ? S->index + 1 : 0);
    } else if (!strcmp(key, "pages")) {
        snprintf(out, cap, "%d", S->open ? S->nb.npages : 0);
    } else if (!strcmp(key, "tool")) {
        const char *name = app_probe_tool(a->settings.current_tool);  // a tool it lacks: its number
        if (name) snprintf(out, cap, "%s", name);
        else snprintf(out, cap, "%d", (int)a->settings.current_tool);
    } else if (!strcmp(key, "zoom")) {
        snprintf(out, cap, "%.2f", S->zoom / S->zoom_default);  // as the zoom chip shows it: 1.00 = not zoomed
    } else if (!strcmp(key, "modal")) {
        // What is in front (input goes to the first one open, in this order).
        const char *m = S->pages_open   ? "pages"
                        : S->tools_open ? "tools"
                        : S->mb.open    ? "music"
                        : S->dm.open    ? "display"
                        : S->paper_open ? "paper"
                                        : "none";
        snprintf(out, cap, "%s", m);
    } else if (!strcmp(key, "strokes")) {
        snprintf(out, cap, "%d", has_page ? S->page->n : 0);
    } else if (!strcmp(key, "ticked")) {
        snprintf(out, cap, "%d", S->pages_open ? pages_selected(a) : 0);
    } else if (!strcmp(key, "paper")) {
        if (!has_page) snprintf(out, cap, "none");
        else if (paper_is_custom(S->page->paper)) snprintf(out, cap, "custom");
        else snprintf(out, cap, "%d", (int)S->page->paper);
    } else if (!strcmp(key, "panel")) {
        snprintf(out, cap, "%s", S->collapsed ? "collapsed" : "expanded");
    } else if (!strcmp(key, "music")) {
        // The layout on the Custom music sheet while it is open, else the page's custom paper.
        MusicLayout m;
        if (S->mb.open) app_probe_music(&S->mb.m, out, cap);
        else app_probe_music(has_page && music_decode(S->page->paper, &m) ? &m : NULL, out, cap);
    } else {
        return -1;
    }
    return 0;
}
