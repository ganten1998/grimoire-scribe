// editor.c: the notebook editor. Live ink, stroke eraser, the floating panel, page
// navigation, paper changes, autosave.
//
// Pixels: page_canvas holds paper + ink at page resolution: the notebook's page size, the screen
// it was made on (a notebook from another Scribe keeps its own). The view maps page -> screen
// (zoom + pan): the page's width fits the screen's (100% upright for a notebook made on this
// screen), pinch to zoom up to 4x that. a->screen = the viewed page + panel (+ paper sheet).
// compose(r) rebuilds a screen rect from those layers; show(page rect) is the page-side entry.
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "export.h"
#include "dispmenu.h"
#include "bgjob.h"
#include "bgimage.h"
#include "compact.h"
#include "inkmode.h"
#include "musicbuild.h"
#include "history.h"
#include "panel.h"
#include "tplsheet.h"
#include "keyboard.h"
#include "pagesview.h"

enum {
    P_NONE, P_TOOL, P_UNDO, P_REDO, P_PREV, P_NEXT, P_PAPER, P_PENSET, P_LIBRARY,
    P_COLLAPSE, P_GRIP, P_EXPAND, P_PICK_PAPER, P_PICK_CUSTOM, P_PICK_CANCEL, P_CLEAR, P_DISPLAY, P_PAGES,
    P_TOOLPICK, P_PICK_TOOL, P_PIN_MODE, P_PICK_INK, P_PICK_MORE,
};

typedef enum { OWN_NONE, OWN_DRAW, OWN_ERASE, OWN_PANEL, OWN_MODAL, OWN_DISPLAY, OWN_PAGES, OWN_TOOLS, OWN_BUILD,
               OWN_TEMPLATES } Owner;

// Every page visited while a notebook is open stays in memory with its history: undo
// actions point at the page's live Stroke objects, so a page must never be freed and
// reloaded from disk while its history exists.
typedef struct {
    uint32_t page_id;
    History h;
    Page *page;   // NULL until loaded
    bool to_list; // it got an erase stroke this session that erased.txt may not list yet (compact.h)
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
    // White ink and the precise eraser (erase strokes) restore the page's paper: this is it alone,
    // at page size, for their strokes drawn live (page_render draws them over the same paper, so
    // live equals re-render). Made at the first erase stroke, drawn again when one starts on a page
    // of another paper or size than it holds, freed with the notebook.
    Canvas paper;
    PaperKind paper_k;          // the paper it holds (while paper.px)
    bool paper_stale;           // drawn again in place at its next use (the next erase stroke's
                                // start; never freed then, a stroke reads it): the page was drawn
                                // whole on an image paper since, or its picture came after it
    // The open notebook's pages listed for compaction this session (compact_note_page); a page
    // with an erase stroke its file does not have yet is listed once the page is saved (to_list).
    uint32_t *noted;
    int nnoted, capnoted;

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
    bool white_mark;            // as last drawn: the pen in use shows that it draws white ink
    int tool_button_shows;      // as last drawn: the tool button's pen, TB_GRID, or TB_NONE collapsed
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

    // Tool picker (opened from the toolbar's tool button). In Pin mode a tap pins a pen to the
    // panel, or unpins it.
    bool tools_open;
    bool pin_mode;
    int tools_y;                // where its top is meant to be: the panel's, as it opened (it stays
                                // there while pins move the panel; laid out again at a turn)
    Rect tools_rect;
    HitList tools_hits;
    Press tools_press;

    PagesView pv;               // the Pages view (pagesview.h)

    // Paper sheet
    bool paper_open;
    Rect paper_rect;
    HitList modal_hits;
    Canvas paper_prev[PAPER_COUNT];
    Canvas custom_prev;         // the Custom music tile's preview (one layout at a time)
    PaperKind custom_prev_k;
    Canvas image_prev;          // an image paper's preview: one of this notebook's pictures at a time
    PaperKind image_prev_k;
    char image_prev_nb[STORE_ID_LEN];
    MusicBuild mb;              // the Custom music sheet
    TplSheet ts;                // the Templates sheet (More papers...), in the Paper sheet's place
    // The open page's picture (an image paper's) was not there when the page canvas was drawn (a
    // template's, made in the background): it drew white, and is drawn again once a job has ended
    // since (picture_gen: bgimage_generation then).
    bool picture_missing;
    unsigned picture_gen;
    // A cache drew a picture white the same way (the paper erase strokes restore toward, the More
    // papers tile's preview, a Pages preview): they are all dropped once a job has ended since the
    // first such draw (white_gen), whichever page is open by then.
    bool white_caches;
    unsigned white_gen;
};

#define S (a->ed)

// ---- layout ----

// Panel metrics shrink (g_fit, panel.h) when the screen is too short for the full panel: the
// Kindle turned sideways, with pens pinned.
static PanelFit g_fit = {1.f, 1.f};
static int sc(float v) { return ui_px(v * g_fit.scale); }
static int pw(void) { return panel_w(); }        // expanded width (icons only)
static int btn_h(void) { return panel_btn_h(g_fit); }
static int gap(void) { return panel_gap(g_fit); }
static int div_h(void) { return panel_div_h(g_fit); }
static int head_h(void) { return panel_head_h(g_fit); }
static int box(void) { return panel_box(); }      // collapsed square
static int label_h(void) { return panel_label_h(g_fit); }
static int npins(App *a) { return clampi(a->settings.npins, 0, PANEL_PINS_MAX); }

static void fit_panel_to(App *a, int screen_h) { g_fit = panel_fit(screen_h, npins(a)); }

static Rect panel_rect_at(App *a, int x, int y) {
    int w = S->collapsed ? box() : pw(), h = S->collapsed ? box() : panel_height(g_fit, npins(a));
    // Its margins above and below stay free wherever it goes (fit_panel_to leaves room for them).
    int m = h + 2 * ui_px(PANEL_MARGIN) <= a->h ? ui_px(PANEL_MARGIN) : 0;
    x = clampi(x, 0, a->w - w);
    y = clampi(y, m, a->h - h - m);
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
    e->to_list = false;
    history_init(&e->h);
    return e;
}

static History *cur_hist(App *a) {
    PageHistory *e = entry_for(a, S->nb.page_ids[S->index]);
    return e ? &e->h : NULL;
}

// Page `index` from disk, at the notebook's page size (S->pgw x S->pgh, use_page_size): every page
// the editor reads comes through here, so none can come at the screen's size instead.
static int load_page(App *a, int index, Page **out) { return store_load_page(&S->nb, index, S->pgw, S->pgh, out); }

// The page at index, from the session cache or (first visit) from disk.
static Page *page_at(App *a, int index) {
    PageHistory *e = entry_for(a, S->nb.page_ids[index]);
    if (!e) return NULL;
    if (!e->page && load_page(a, index, &e->page) != 0) return NULL;
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

// The drawing tool the tool button shows (the pencil if settings.ini names the eraser there).
static ToolKind button_tool(App *a) {
    ToolKind t = a->settings.draw_tool;
    return (unsigned)t < TOOL_COUNT && t != TOOL_ERASER ? t : TOOL_PENCIL;
}

// Where a tap on the eraser in use goes back to, and what the pen's button draws white with while
// the eraser is in use: the pen used just before it (settings.ini before_eraser) while that is
// still on the panel, a pin or the tool button's; else the tool button's.
static ToolKind eraser_back(App *a) {
    int b = a->settings.before_eraser;
    bool shown = b >= 0 && b < TOOL_COUNT && b != TOOL_ERASER &&
                 ((ToolKind)b == button_tool(a) || settings_pinned(&a->settings, (ToolKind)b));
    return shown ? (ToolKind)b : button_tool(a);
}

// What the tool button showed as the panel was last drawn (editor.tool_button): its pen, or these.
enum { TB_GRID = -1, TB_NONE = -2 };

// While the drawing tools draw white ink, the pen in use shows its icon hollow: white on white
// paper shows nothing, and without a mark the pen would seem broken.
static bool draws_white(App *a, ToolKind t) {
    return a->settings.white && t == a->settings.current_tool && (unsigned)t < TOOL_COUNT && t != TOOL_ERASER;
}

// A pen's button on the panel (the tool button's, or a pinned one's), selected while it is the pen
// in use, hollow while that draws white.
static void draw_pen_button(App *a, Rect r, ToolKind t) {
    if (draws_white(a, t)) {
        ui_button_hollow(&a->screen, r, ui_tool_icon(t), BTN_SELECTED);
        S->white_mark = true;
    } else {
        ui_button(&a->screen, r, ui_tool_icon(t), NULL, tool_style(a, t));
    }
}

static void draw_panel(App *a) {
    Canvas *c = &a->screen;
    Rect p = S->panel;
    hits_clear(&S->hits);
    S->white_mark = false;
    S->tool_button_shows = TB_NONE;
    if (S->collapsed) {
        ui_frame(c, p, ui_px(22), 255, true, 0);
        ui_frame(c, p, ui_px(22), 0, false, ui_px(6));
        Rect ir = rect_inflate(p, -ui_px(20));
        ir.h -= ui_px(12);
        if (draws_white(a, a->settings.current_tool)) {
            ui_icon_hollow(c, ui_tool_icon(a->settings.current_tool), ir, 0, 255);
            S->white_mark = true;
        } else {
            ui_icon(c, ui_tool_icon(a->settings.current_tool), ir, 0);
        }
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
    // The drawing tool (tap: the picker), the pens pinned under it, and the eraser. When the
    // drawing tool is one of the pins, its pin shows it: the tool button shows a grid instead.
    ToolKind dt = button_tool(a);
    bool dt_pinned = settings_pinned(&a->settings, dt), dt_sel = !dt_pinned && a->settings.current_tool == dt;
    Rect rt = {x, y, w, btn_h()};
    if (dt_pinned) {
        ui_button(c, rt, ICON_GRID, NULL, BTN_PLAIN);
        S->tool_button_shows = TB_GRID;
    } else {
        draw_pen_button(a, rt, dt);
        S->tool_button_shows = (int)dt;
    }
    // A small corner mark says "more tools here" (the grid says it by itself).
    if (!dt_pinned)
        canvas_disc_aa(c, (float)(rt.x + rt.w - ui_px(16)), (float)(rt.y + rt.h - ui_px(16)), (float)ui_px(7),
                       dt_sel ? 255 : 0, 1.f);
    hits_add(&S->hits, rt, P_TOOLPICK, 0);
    y += btn_h() + gap();
    for (int i = 0; i < npins(a); i++) {
        Rect rp = {x, y, w, btn_h()};
        draw_pen_button(a, rp, a->settings.pins[i]);
        hits_add(&S->hits, rp, P_TOOL, a->settings.pins[i]);
        y += btn_h() + gap();
    }
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
    font_draw_centered(c, FONT_BOLD, ui_small_size() * (g_fit.scale < 1.f ? g_fit.scale : 1.f), lr, lab, 0);
    hits_add(&S->hits, lr, P_PAGES, 0);
    y += label_h() + gap();
    bool last = S->index >= S->nb.npages - 1;
    Rect rn = {x, y, w, btn_h()};
    ui_button(c, rn, last ? ICON_PLUS : ICON_NEXT, NULL, BTN_PLAIN);
    hits_add(&S->hits, rn, P_NEXT, 0);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6), ui_px(3), 160);
    y += div_h();
    // Paper, Display, Pen feel, Library (icons only, like the rest of the panel).
    struct {
        IconKind icon;
        int id;
    } rest[] = {{ICON_PAPER, P_PAPER}, {ICON_BRIGHTNESS, P_DISPLAY}, {ICON_TUNE, P_PENSET}, {ICON_LIBRARY, P_LIBRARY}};
    for (int i = 0; i < 4; i++) {
        Rect r = {x, y, w, btn_h()};
        ui_button(c, r, rest[i].icon, NULL, BTN_PLAIN);
        hits_add(&S->hits, r, rest[i].id, 0);
        y += btn_h() + gap();
    }
}

// A cache was just drawn with a picture white in it (bgimage_take_missing; at generation gen):
// it is dropped once a background job has ended since (pictures_arrived).
static void note_white(App *a, unsigned gen) {
    if (!S->white_caches || (int)(gen - S->white_gen) < 0) S->white_gen = gen;
    S->white_caches = true;
}

static const Canvas *paper_preview(App *a, PaperKind k) {
    Canvas *c;
    if (!paper_is_known(k)) k = PAPER_BLANK;  // a newer version's paper: blank, as it renders here
    if (paper_is_custom(k)) {
        c = &S->custom_prev;
        if (c->px && S->custom_prev_k != k) canvas_free(c);
        S->custom_prev_k = k;
    } else if (paper_is_image(k)) {
        // A picture of this notebook's (the same value in another notebook can be another one).
        c = &S->image_prev;
        if (c->px && (S->image_prev_k != k || strcmp(S->image_prev_nb, S->nb.id) || c->w != S->pgw / 6 ||
                      c->h != S->pgh / 6))
            canvas_free(c);
        S->image_prev_k = k;
        strlcpy_q(S->image_prev_nb, S->nb.id, sizeof S->image_prev_nb);
    } else {
        c = &S->paper_prev[clampi((int)k, 0, PAPER_COUNT - 1)];
    }
    if (!c->px) {
        Canvas full;
        if (canvas_init(&full, S->pgw, S->pgh) != 0) return NULL;
        if (paper_is_image(k)) {
            bgimage_take_missing();  // (what drew white before)
            bgimage_render(&full, S->nb.id, k, (Rect){0, 0, S->pgw, S->pgh});
            if (bgimage_take_missing()) note_white(a, bgimage_generation());
        } else {
            paper_render(&full, k, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
        }
        if (canvas_init(c, S->pgw / 6, S->pgh / 6) == 0) canvas_downsample_keep_lines(c, &full, 0.6f);
        canvas_free(&full);
    }
    return c->px ? c : NULL;
}

static void draw_paper_sheet(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->modal_hits);
    PaperGrid g;
    S->paper_rect = paper_sheet_rect(a, &g);
    int gap_ = ui_px(30), gridw = g.cols * g.tw + (g.cols - 1) * gap_;
    // TRANSLATORS: the title of the editor's paper sheet, which changes the open page's paper.
    Rect in = ui_sheet(c, S->paper_rect, _("Paper for this page"));
    bool page_custom = S->page && paper_is_known(S->page->paper) && paper_is_custom(S->page->paper);
    bool page_picture = S->page && paper_is_image(S->page->paper);
    for (int i = 0; i < PAPER_SHEET_TILES; i++) {
        // The last two tiles. Custom music shows this page's layout, or the last one built (also
        // for a newer layout this build cannot draw, which is then not selected either). More
        // papers opens the Templates sheet: it shows this page's picture when the page is on one.
        bool custom = i == PAPER_COUNT, more = i == PAPER_COUNT + 1;
        PaperKind k = more     ? (page_picture ? S->page->paper : PAPER_BLANK)
                      : custom ? (page_custom ? S->page->paper : a->settings.music_layout)
                               : paper_at(i);
        Rect tr = {in.x + (in.w - gridw) / 2 + (i % g.cols) * (g.tw + gap_), in.y + (i / g.cols) * g.cell_h, g.tw, g.th};
        canvas_fill(c, tr, 255);
        const Canvas *pv = more && !page_picture ? NULL : paper_preview(a, k);
        if (pv) canvas_blit_fit(c, pv, tr);
        else if (more) paper_tile_more(c, tr);
        bool sel = more ? page_picture : custom ? page_custom : (S->page && S->page->paper == k);
        ui_frame(c, rect_inflate(tr, sel ? ui_px(10) : 0), ui_px(12), 0, false, sel ? ui_px(9) : ui_px(3));
        // (Custom music... and More papers... as in New notebook, which says their room.)
        const char *label = more ? _("More papers...") : custom ? _("Custom music...") : paper_label(k);
        paper_tile_label(c, tr, gap_, label, sel);
        hits_add(&S->modal_hits, (Rect){tr.x, tr.y, g.tw, g.th + ui_px(90)},
                 more ? P_PICK_MORE : custom ? P_PICK_CUSTOM : P_PICK_PAPER, (int)k);
    }
    Rect cancel = {in.x + in.w - ui_px(380), in.y + in.h - ui_px(140), ui_px(380), ui_px(140)};
    ui_button(c, cancel, ICON_NONE, _("Cancel"), BTN_PLAIN);
    hits_add(&S->modal_hits, cancel, P_PICK_CANCEL, 0);
}

// ---- view ----

static bool view_identity(App *a) {
    return S->zoom == 1.f && S->ox == 0.f && S->oy == 0.f && S->pgw == a->w && S->pgh == a->h;
}

static void view_clamp(App *a) {
    S->zoom = clampf(S->zoom, S->zoom_min, S->zoom_default * 4.f);
    float vw = (float)a->w / S->zoom, vh = (float)a->h / S->zoom;  // visible page area
    // A page fitted to a screen of another size (a notebook from another Scribe) spans it but for
    // float rounding: exactly, so the fitted view sits at no offset (and shows no zoom chip).
    if (fabsf(vw - (float)S->pgw) < 0.01f) vw = (float)S->pgw;
    if (fabsf(vh - (float)S->pgh) < 0.01f) vh = (float)S->pgh;
    S->ox = vw >= (float)S->pgw ? -(vw - (float)S->pgw) / 2.f : clampf(S->ox, 0.f, (float)S->pgw - vw);
    S->oy = vh >= (float)S->pgh ? -(vh - (float)S->pgh) / 2.f : clampf(S->oy, 0.f, (float)S->pgh - vh);
    // Snap to exactly 100% / no offset in portrait so the fast identity path is used.
    if (fabsf(S->zoom - 1.f) < 0.01f && S->pgw == a->w && S->pgh == a->h) {
        S->zoom = 1.f;
        S->ox = S->oy = 0.f;
    }
}

// The page fills the screen's width: 100% upright for a page of this screen's size, the whole page
// for a notebook from another Scribe (theirs are 3:4 too), and sideways the width (pan with two
// fingers; pinching out shows the whole page).
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

// The zoom chip's text: the zoom, and what a tap on the chip does.
static void zoom_text(App *a, char *out, size_t cap) {
    // TRANSLATORS: the chip at the top of a zoomed page: the zoom, then that a tap on the chip takes
    // the page back to its usual size. In bold at 34 px; the chip grows with its text.
    snprintf(out, cap, _("%d%%   tap to reset"), (int)lrintf(S->zoom / S->zoom_default * 100.f));
}

// Centered at the top, 380 px wide, or as wide as a longer text needs (ui_chip_w: the English
// always fits the 380).
static Rect zoom_chip(App *a) {
    char z[96];
    zoom_text(a, z, sizeof z);
    int h = ui_px(100), w = ui_chip_w(FONT_BOLD, ui_small_size(), z, h, ui_px(380), a->w - ui_px(80));
    return (Rect){a->w / 2 - w / 2, ui_px(24), w, h};
}
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

// The ink row under the pens: one choice each, the pens drawing black ink or white (erase strokes:
// docs/FORMAT.md). A later colour adds its choices to this control.
static const struct {
    const char *label;  // looked up where drawn, C_("ink", ...)
    bool white;
} k_inks[] = {
    // TRANSLATORS: the tool picker's ink row under the pens, two choices side by side; this one:
    // the pens draw black ink. 426 px at 44 px, or two lines at 34 px.
    {NC_("ink", "Black"), false},
    // TRANSLATORS: the other choice of the tool picker's ink row: the pens draw white ink, which
    // takes away the ink under it and keeps the paper's lines (426 px at 44 px).
    {NC_("ink", "White"), true},
};
#define N_INKS ((int)(sizeof k_inks / sizeof k_inks[0]))

static void draw_tools(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->tools_hits);
    int bw = ui_px(300), bh = ui_px(150), g = ui_px(16), cols = 3, rows = (N_DRAW_TOOLS + cols - 1) / cols;
    int gridw = cols * bw + (cols - 1) * g, inkh = ui_px(120);
    int w = gridw + 2 * ui_px(30), h = ui_px(110) + rows * bh + (rows - 1) * g + g + inkh + ui_px(40);
    int x = S->panel.x - w - ui_px(20);
    if (x < ui_px(10)) x = S->panel.x + S->panel.w + ui_px(20);
    // Neither side has room for it beside a panel dragged to the middle: it goes over the panel
    // then, whole on the screen (while it is open it takes every tap).
    x = clampi(x, ui_px(10), a->w - w - ui_px(10));
    int y = clampi(S->tools_y, ui_px(10), a->h - h - ui_px(10));
    S->tools_rect = (Rect){x, y, w, h};
    ui_frame(c, S->tools_rect, ui_px(22), 255, true, 0);
    ui_frame(c, S->tools_rect, ui_px(22), 0, false, ui_px(6));
    // The title row: at its right Pin, which turns the grid into the choice of the pens pinned to
    // the panel, and there Done, which turns it back. Its width holds either word.
    // TRANSLATORS: the tool picker's button at the top right that starts choosing the pens pinned
    // to the panel, beside a pin (116 px at 34 px; the button widens for a longer word).
    const char *pin = C_("panel", "Pin");
    // TRANSLATORS: the same button while the pens are being chosen, beside a check mark: it ends
    // that (116 px at 34 px, as Pin).
    const char *done = _("Done");
    float hs = ui_small_size();
    int hh = ui_px(78), hi = hh - ui_px(24), lx = ui_px(14) + hi + ui_px(10), pad = ui_px(16);
    int lw = font_measure(FONT_CAPS, hs, pin), dw = font_measure(FONT_CAPS, hs, done);
    int hw = clampi(lx + (lw > dw ? lw : dw) + pad, ui_px(210), w / 2);
    Rect hb = {x + w - ui_px(30) - hw, y + ui_px(14), hw, hh};
    ui_button(c, hb, ICON_NONE, NULL, S->pin_mode ? BTN_PRIMARY : BTN_PLAIN);
    ui_icon(c, S->pin_mode ? ICON_CHECK : ICON_PIN, (Rect){hb.x + ui_px(14), hb.y + (hh - hi) / 2, hi, hi}, 0);
    ui_text(c, FONT_CAPS, hs, hs, (Rect){hb.x + lx, hb.y, hb.w - lx - pad, hh}, S->pin_mode ? done : pin, 1, true, 0);
    hits_add(&S->tools_hits, hb, P_PIN_MODE, 0);
    char title[160];
    // TRANSLATORS: the tool picker's title while it chooses the pens pinned to the panel under the
    // tool button: a tap pins a pen or unpins it; %d is how many can be (4). About 700 px at 44 px
    // in bold, or two lines at 34 px. Normally the title is Tools.
    if (S->pin_mode) snprintf(title, sizeof title, _("Pin to the panel (up to %d)"), PANEL_PINS_MAX);
    // TRANSLATORS: the title of the editor's tool picker.
    else snprintf(title, sizeof title, "%s", _("Tools"));
    ui_text(c, FONT_BOLD, ui_text_size(), ui_small_size(), (Rect){x + ui_px(30), y, hb.x - x - ui_px(50), ui_px(100)},
            title, 2, false, 0);
    for (int i = 0; i < N_DRAW_TOOLS; i++) {
        ToolKind t = k_draw_tools[i];
        Rect r = {x + ui_px(30) + (i % cols) * (bw + g), y + ui_px(100) + (i / cols) * (bh + g), bw, bh};
        // TRANSLATORS: the tool picker's shorter names for the Mechanical pencil and the
        // Calligraphy pen (268 px at 34 px in Gothic, 284 in Classic, as the other tools' names).
        const char *name = t == TOOL_MECH_PENCIL   ? C_("tool", "Mechanical")
                           : t == TOOL_CALLIGRAPHY ? C_("tool", "Calligraphy")
                                                   : tool_label(t);
        // Selected: the pen in use, or in Pin mode a pen pinned.
        bool sel = S->pin_mode ? settings_pinned(&a->settings, t) : a->settings.current_tool == t;
        ui_button(c, r, ui_tool_icon(t), name, sel ? BTN_SELECTED : BTN_PLAIN);
        hits_add(&S->tools_hits, r, P_PICK_TOOL, t);
    }
    // The ink row, out of use in Pin mode (the pins are pens, whatever they draw).
    int iy = y + ui_px(100) + rows * bh + (rows - 1) * g + g, iw = (gridw - (N_INKS - 1) * g) / N_INKS;
    for (int i = 0; i < N_INKS; i++) {
        Rect r = {x + ui_px(30) + i * (iw + g), iy, iw, inkh};
        ButtonStyle st = S->pin_mode ? BTN_DISABLED : a->settings.white == k_inks[i].white ? BTN_SELECTED : BTN_PLAIN;
        ui_button(c, r, ICON_NONE, C_("ink", k_inks[i].label), st);
        if (!S->pin_mode) hits_add(&S->tools_hits, r, P_PICK_INK, i);
    }
}

// Rebuild a screen rect from the layers.
static void compose(App *a, Rect r) {
    if (S->pv.open) {  // the Pages view covers everything
        pagesview_draw(&S->pv, a);
        return;
    }
    a->toast_floor = 0;
    r = rect_intersect(r, (Rect){0, 0, a->w, a->h});
    if (rect_is_empty(r)) return;
    if (view_identity(a)) canvas_blit(&a->screen, &S->page_canvas, r, r.x, r.y);
    else blit_view(a, r);
    if (zoomed(a) && rect_overlaps(r, zoom_chip(a))) {
        char z[96];
        zoom_text(a, z, sizeof z);
        Rect c = zoom_chip(a);
        ui_frame(&a->screen, c, c.h / 2, 0, true, 0);
        font_draw_centered(&a->screen, FONT_BOLD, ui_small_size(), c, z, 255);
    }
    if (rect_overlaps(r, S->panel)) draw_panel(a);
    if (S->paper_open && rect_overlaps(r, S->paper_rect)) draw_paper_sheet(a);
    if (S->ts.open && rect_overlaps(r, S->ts.rect)) tplsheet_draw(&S->ts, a);
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

// Page `pid` was written with an erase stroke it did not have: its file keeps the ink under it
// until compaction takes that out when the notebook closes (compact.h). Listed once per page and
// session; false if the list could not be written.
static bool note_for_compaction(App *a, uint32_t pid) {
    for (int i = 0; i < S->nnoted; i++)
        if (S->noted[i] == pid) return true;
    if (compact_note_page(S->nb.id, pid) != 0) return false;
    if (S->nnoted == S->capnoted) {
        int nc = S->capnoted ? S->capnoted * 2 : 16;
        uint32_t *nn = realloc(S->noted, (size_t)nc * sizeof *nn);
        if (!nn) return true;  // (listed: only noted again next time)
        S->noted = nn;
        S->capnoted = nc;
    }
    S->noted[S->nnoted++] = pid;
    return true;
}

// The pages that got an erase stroke this session, listed for compaction once their files have it:
// at a save, not as the pen lifts, so the list's write never holds up the next stroke. A page the
// list could not take (it could not be written, or did not read) is tried again at the next save.
static void list_erased_pages(App *a) {
    for (int i = 0; i < S->nhist; i++) {
        PageHistory *e = &S->hist[i];
        if (e->to_list && !(e->page == S->page && S->page_dirty) && note_for_compaction(a, e->page_id))
            e->to_list = false;
    }
}

// False if the page could not be written; it then stays dirty (and in memory), so callers
// that would drop it (turning the page, closing) must stay put instead.
static bool save_page(App *a) {
    if (!S->open) return true;
    if (S->page_dirty) {
        // A read-only page (from a newer Grimoire) is never written: its file stays as it is.
        if (S->page && !S->page->readonly && store_save_page(&S->nb, S->index, S->page) != 0) {
            app_toast(a, _("Could not save this page. Is the storage full?"), 5000);
            return false;
        }
        if (S->index == 0) S->thumb_dirty = true;  // written when leaving the page (it is slow)
        S->page_dirty = false;
        S->info_dirty = true;
    }
    list_erased_pages(a);  // their files have the erase strokes now (and the ink under them)
    if (S->info_dirty) {
        if (store_save_info(&S->nb) != 0) return false;
        S->info_dirty = false;
    }
    return true;
}

// Draws r of the open page into the page canvas. An image paper's picture that is not there (a
// template's, made in the background) draws white: noted, to be drawn once it is (editor_tick).
static void draw_page(App *a, Rect r) {
    bgimage_take_missing();  // (what drew white before)
    page_render(&S->page_canvas, S->page, r, a->dpi);
    bool whole = r.x <= 0 && r.y <= 0 && r.x + r.w >= S->pgw && r.y + r.h >= S->pgh;
    if (bgimage_take_missing()) {
        S->picture_missing = true;
        S->picture_gen = bgimage_generation();
    } else if (whole) {
        S->picture_missing = false;
    }
}

static void render_page_full(App *a) {
    draw_page(a, (Rect){0, 0, S->pgw, S->pgh});
    S->canvas_page = S->page;
    // An image paper's picture can have come since its paper alone was drawn (S->paper: a picture
    // made in the background draws white until it is there), and the page now shows it.
    if (paper_is_image(S->page->paper)) S->paper_stale = true;
}

// A page from a newer Grimoire (or one this build cannot fully read) is shown as far as it can
// be and never changed: writing, erasing, clearing and paper changes are refused with this.
// TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines).
static const char *const k_readonly_msg = N_("This page was made by a newer Grimoire. It is read-only here.");

static bool refuse_readonly(App *a) {
    if (!S->page || !S->page->readonly) return false;
    app_toast(a, _(k_readonly_msg), 3000);
    return true;
}

// On arriving at a page: a read-only page says so, and any other page takes that message down if
// it is still up from the page before, so it never seems to be about this one.
static void arrive_readonly(App *a) {
    if (refuse_readonly(a) || !a->toast_visible || strcmp(a->toast, _(k_readonly_msg))) return;
    a->toast_visible = false;
    app_invalidate(a, a->toast_rect, REFRESH_UI);  // (already inside the arriving page's full repaint)
}

static void go_to_page(App *a, int index) {
    S->settle = RECT_EMPTY;  // the whole screen is about to be redrawn anyway
    S->panel_stale = false;
    if (!save_page(a)) return;
    write_thumb_if_needed(a);
    index = clampi(index, 0, S->nb.npages - 1);
    Page *np = page_at(a, index);
    if (!np) {
        app_toast(a, _("Could not open that page"), 3000);
        return;
    }
    S->page = np;
    S->index = index;
    S->nb.last_page = index;
    S->info_dirty = true;
    render_page_full(a);
    view_reset(a);
    compose_all(a, REFRESH_FULL);
    arrive_readonly(a);
}

// The open notebook's page size (it keeps the one it was made with, whatever the screen). The page
// canvas and the paper previews are made at it: when it changes they go, to be made again.
static void use_page_size(App *a) {
    int w, h;
    store_notebook_page_size(&S->nb, &w, &h);
    if (w == S->pgw && h == S->pgh) return;
    S->pgw = w;
    S->pgh = h;
    canvas_free(&S->page_canvas);
    S->canvas_page = NULL;
    for (int k = 0; k < PAPER_COUNT; k++) canvas_free(&S->paper_prev[k]);
    canvas_free(&S->custom_prev);
    canvas_free(&S->paper);
}

#ifdef GRIMOIRE_SIM
// Tests (editor.matches_closed): a hash of the open page drawn from its file as the notebook closed
// to be compacted, so the page drawn from its file when the notebook opens again (compacted by
// then, or not) can be held to it. (From its file, not the page canvas: a file keeps pressure in
// 16 bits, so a page read back can differ from the one drawn live by a level here and there.)
static struct {
    char id[STORE_ID_LEN];
    uint32_t page;
    uint64_t hash;
} g_closed;

static uint64_t canvas_hash(const Canvas *c) {
    uint64_t h = 1469598103934665603ull;  // FNV-1a
    for (int y = 0; y < c->h; y++) {
        const uint8_t *row = &c->px[(size_t)y * (size_t)c->stride];
        for (int x = 0; x < c->w; x++) h = (h ^ row[x]) * 1099511628211ull;
    }
    return h;
}

static void closed_record(App *a) {
    g_closed.id[0] = 0;
    Page *p = NULL;
    Canvas c;
    if (store_read_page(&S->nb, S->index, &p) != 0 || canvas_init(&c, S->pgw, S->pgh) != 0) {
        page_free(p);
        return;
    }
    page_render(&c, p, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
    strlcpy_q(g_closed.id, S->nb.id, sizeof g_closed.id);
    g_closed.page = S->nb.page_ids[S->index];
    g_closed.hash = canvas_hash(&c);
    canvas_free(&c);
    page_free(p);
}
#else
static void closed_record(App *a) {}
#endif

static void close_notebook(App *a) {
    if (!S->open) return;
    // Pages an older version had saved on blank paper got their pictures back as they opened: the
    // thumbnail and the PDF it drew of them are drawn again.
    if (store_take_restored_count() > 0) {
        S->changed = true;
        if (S->index == 0) S->thumb_dirty = true;
    }
    save_page(a);
    write_thumb_if_needed(a);
    if (S->index != 0 && S->changed) {
        // Keep the library thumbnail (page 1) in sync even if we never visited it.
        Page *p0;
        if (load_page(a, 0, &p0) == 0) {
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
    // The ink erase strokes took away entirely, out of the page files (compact.h): in the
    // background, as it can take long on the Kindle, and after the PDF (which it does not change).
    // Not while Grimoire leaves: the compaction would go with it, so the pages stay listed for the
    // next start.
    if (compact_listed(S->nb.id)) {
        if (!a->running) {
            qlog("compact: %s left listed for the next start (Grimoire is leaving)", S->nb.id);
        } else {
            closed_record(a);
            bg_queue(BG_COMPACT_PAGES, S->nb.id, NULL, a->dpi, false);
        }
    }
    int unlisted = 0;
    for (int i = 0; i < S->nhist; i++) unlisted += S->hist[i].to_list;
    if (unlisted)
        qlog("compact: %s: %d page(s) with erased ink could not be listed; their files keep it until listed again",
             S->nb.id, unlisted);
    free_histories(a);
    canvas_free(&S->paper);
    canvas_free(&S->image_prev);  // (it may have drawn a picture white: made again from the file)
    tplsheet_close(&S->ts);
    S->picture_missing = S->white_caches = false;
    S->nnoted = 0;
    pagesview_close(&S->pv);
    S->open = false;
    store_take_restored_count();  // (page 1, read for the thumbnail above, was drawn as it is now)
    bgimage_forget(NULL);         // the notebook's pictures (up to 3 pages' worth) and its failures
}

void editor_open_notebook(App *a, const char *id) {
    close_notebook(a);
    // Every way into a notebook comes here (the Library's card and its menu, a new notebook): its
    // compaction, queued or running, is stopped before anything here reads or writes its pages (a
    // stopped compaction leaves each page file as it was; the list keeps the pages for later).
    bg_cancel_notebook(id);
    if (store_load_info(id, &S->nb) != 0) {
        app_toast(a, _("Could not open that notebook"), 3000);
        return;
    }
    store_recover_pages(&S->nb);  // pages an older version set aside that open now
    // Pages an older version saved on blank paper get their pictures back (bg/pages.ini), all of
    // them now: the thumbnail and PDF it drew are drawn again when the notebook closes.
    store_restore_listed_pages(&S->nb);
    bgimage_retry_missing();      // (a picture the Library found missing may be there now)
    use_page_size(a);
    S->index = clampi(S->nb.last_page, 0, S->nb.npages - 1);
    S->page = page_at(a, S->index);
    if (!S->page) {
        free_histories(a);
        app_toast(a, _("Could not open that notebook"), 3000);
        return;
    }
    S->open = true;
    S->page_dirty = S->info_dirty = S->changed = false;
    if (!S->page_canvas.px && canvas_init(&S->page_canvas, S->pgw, S->pgh) != 0) return;
    render_page_full(a);
    S->view_w = 0;  // set the view up afresh on enter
    app_switch(a, SCR_EDITOR);
    arrive_readonly(a);
}

// A page KOReader sent (annotate.h): notebook id open at its page page_id, or where
// editor_open_notebook opens it should it have no such page. added: the page was added to the
// notebook (or the notebook made) at this start, a change like any other: the notebook's PDF is
// made again as it closes, and the Library's card too when the page is its first.
void editor_open_notebook_at(App *a, const char *id, uint32_t page_id, bool added) {
    editor_open_notebook(a, id);
    if (!S->open || strcmp(S->nb.id, id)) return;
    for (int i = 0; i < S->nb.npages; i++)
        if (S->nb.page_ids[i] == page_id) {
            if (i != S->index) go_to_page(a, i);
            break;
        }
    if (!added) return;
    S->changed = true;
    if (S->index == 0) S->thumb_dirty = true;
}

// A picture made in the background (a template's, a page KOReader sent: bgimage.h) may be the open
// page's paper, drawn white while its file was missing: the page is drawn again with it, now, or once
// the stroke or gesture under way is over (editor_tick), or as the editor comes back in front. Only
// such a page: one that shows its picture already (the job was another notebook's, or the templates'
// previews), or whose picture is still to come, stays as it is.
void editor_picture_made(App *a) {
    if (!S || !S->open || !S->page || !paper_is_image(S->page->paper)) return;
    if (!bgimage_undrawn(S->nb.id, paper_image_id(S->page->paper))) return;
    S->canvas_page = NULL;
    S->paper_stale = true;  // (what erase strokes restore toward: drawn white too)
    if (a->scr == SCR_EDITOR && !a->asleep && !editor_busy(a) && S->page_canvas.px) {
        render_page_full(a);
        compose_all(a, REFRESH_UI);
    }
}

bool editor_save_now(App *a) { return save_page(a); }

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

// The open page's paper alone (S->paper), drawn again first if it holds another paper or size, or
// the page was drawn whole on an image paper since (paper_stale): what an erase stroke restores
// toward live, as page_render draws it (an image paper's picture too). NULL when there is no memory
// for it.
static const Canvas *page_paper(App *a) {
    Canvas *c = &S->paper;
    if (c->px && (c->w != S->pgw || c->h != S->pgh)) canvas_free(c);
    bool fresh = !c->px;
    if (fresh && canvas_init(c, S->pgw, S->pgh) != 0) return NULL;
    if (fresh || S->paper_k != S->page->paper || S->paper_stale) {
        S->paper_k = S->page->paper;
        S->paper_stale = false;
        bgimage_take_missing();
        page_paper_render(c, S->page, (Rect){0, 0, c->w, c->h}, a->dpi);  // (an image paper's picture too)
        if (bgimage_take_missing()) note_white(a, bgimage_generation());
    }
    return c;
}

static void draw_begin(App *a, const InEvent *ev) {
    if (refuse_readonly(a)) {  // neither ink nor the eraser (tool, eraser end or side button)
        if (S->btn_down) S->btn_used_for_ink = true;
        S->owner = OWN_MODAL;  // swallow the rest of this stroke
        S->has_pressed = false;
        return;
    }
    PenStroke ps = pen_stroke(&a->settings, ev->eraser, ev->side_button);
    // The pen's button drawing white while the eraser is in use: with the pen a tap on the eraser
    // goes back to, a pin too (pen_drawing_tool knows only the tool button's).
    if (ps.act == PEN_WHITE && a->settings.current_tool == TOOL_ERASER) ps.tool = eraser_back(a);
    if (ps.act == PEN_WHOLE) {
        S->owner = OWN_ERASE;
        free(S->snap);
        S->nsnap = S->page->n;
        S->snap = malloc((size_t)(S->nsnap ? S->nsnap : 1) * sizeof *S->snap);
        if (S->snap) memcpy(S->snap, S->page->strokes, (size_t)S->nsnap * sizeof *S->snap);
        S->nremoved = 0;
        to_page(a, ev->x, ev->y, &S->ex, &S->ey);
        S->erase_dirty = RECT_EMPTY;
        if (S->btn_down) S->btn_used_for_ink = true;
        display_live_ink(true);  // a pen stroke too: its frames never wait (erase_end repaints them)
        return;
    }
    if (S->btn_down) S->btn_used_for_ink = true;
    S->owner = OWN_DRAW;
    uint32_t seed = (ev->t_ms * 2654435761u) ^ (uint32_t)(S->page->n * 40503u) ^ (uint32_t)(ev->x * 97.f);
    // White ink and the precise eraser are erase strokes (the same draw path as ink: its dabs,
    // dirty rects, refreshes, settle, zoom, rotation and undo): white with the pen's own settings
    // and texture, the eraser with its round tip, at the size on screen it has at any zoom.
    bool erase = ps.act != PEN_INK;
    const Canvas *paper = erase ? page_paper(a) : NULL;
    if (erase && !paper) {
        qlog("editor: no memory for the page's paper; the erase stroke is not drawn");
        S->owner = OWN_MODAL;  // swallow the rest of this stroke
        S->has_pressed = false;
        return;
    }
    ToolParams prm = ps.act == PEN_PRECISE ? eraser_at_zoom(&a->settings.tools[TOOL_ERASER], S->zoom)
                                           : a->settings.tools[ps.tool];
    S->cur = erase ? stroke_new(TOOL_ERASER, &prm, erase_seed(ps.tool, seed)) : stroke_new(ps.tool, &prm, seed);
    if (!S->cur) {
        S->owner = OWN_NONE;
        return;
    }
    stroke_brush_begin(&S->bs, S->cur, a->dpi, paper ? paper->px : NULL, 0, 0, paper ? paper->stride : 0);
    smoother_reset(&S->sm);
    // Ink asks for the pen mode (where it is on); an erase stroke is live ink without it, as the
    // stroke eraser is (the driver's pen path is for ink going down).
    if (erase) display_live_ink(true);
    else display_pen_mode(true);
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
    // Settle later, once the writer pauses (see settle_now).
    S->settle = rect_union(S->settle, s->bbox);
    S->settle_at = now_ms() + 450;
    if (stroke_is_erase(s) && !S->bs.restored) {
        // An erase stroke that changed no pixel of the page (over blank paper, or ink the eraser
        // had taken already): not kept, and no undo step that would do nothing. Its fast frames
        // still settle with the rest.
        stroke_free(s);
        return;
    }
    History *h = cur_hist(a);
    bool had_undo = h && history_can_undo(h), had_redo = h && history_can_redo(h);
    bool was_empty = S->page->n == 0, erase = stroke_is_erase(s);
    page_add(S->page, s);
    bool kept = S->page->n && S->page->strokes[S->page->n - 1] == s;
    if (h && kept) history_push_add(h, S->page->n - 1, s);
    PageHistory *e = erase && kept ? entry_for(a, S->nb.page_ids[S->index]) : NULL;
    if (e) e->to_list = true;  // listed for compaction once the page is saved
    mark_changed(a);
    // Only touch the panel when Undo/Redo availability actually changed.
    if (!had_undo || had_redo || was_empty) S->panel_stale = true;
}

static float dist2_to_segment(float px, float py, float ax, float ay, float bx, float by) {
    float dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy;
    float t = l2 > 0.f ? clampf(((px - ax) * dx + (py - ay) * dy) / l2, 0.f, 1.f) : 0.f;
    float ex = px - (ax + dx * t), ey = py - (ay + dy * t);
    return ex * ex + ey * ey;
}

// A pixel shows ink when the page there is darker than its paper by more than this: a level or
// two left by a dab's rounding is no ink anyone can see.
#define INK_SHOWS 6

// Whether the page is darker than its paper by more than INK_SHOWS anywhere over R (page px) within
// r px of the segment (x0, y0)-(x1, y1) and within stroke s's reach of its line (as far as its bbox
// reaches): the only pixels there where its ink can show. The page canvas may still show strokes
// taken off the page in this sweep, which only makes more of it dark.
static bool dark_near(App *a, const Stroke *s, const Canvas *paper, Rect R, float x0, float y0, float x1, float y1,
                      float r) {
    const Canvas *pc = &S->page_canvas;
    float reach = brush_max_radius(s->tool, &s->params, a->dpi) + 2.f;
    for (int j = 0; j < s->n; j++) {
        const PenSample *pa = &s->pts[j], *pb = &s->pts[j + 1 < s->n ? j + 1 : j];
        Rect sr = rect_intersect(rect_from_points(pa->x, pa->y, pb->x, pb->y, reach + 1.f), R);
        for (int y = sr.y; y < sr.y + sr.h; y++) {
            const uint8_t *row = &pc->px[(size_t)y * (size_t)pc->stride], *pr = &paper->px[(size_t)y * (size_t)paper->stride];
            for (int x = sr.x; x < sr.x + sr.w; x++) {
                if (pr[x] - row[x] <= INK_SHOWS) continue;
                float qx = (float)x + 0.5f, qy = (float)y + 0.5f;
                if (dist2_to_segment(qx, qy, x0, y0, x1, y1) <= r * r &&
                    dist2_to_segment(qx, qy, pa->x, pa->y, pb->x, pb->y) <= reach * reach)
                    return true;
            }
        }
    }
    return false;
}

// Whether stroke i's own ink shows over R (page px, on the page and in its bbox) within r px of the
// segment (x0, y0)-(x1, y1): the page drawn there without it is lighter than with it by more than
// INK_SHOWS somewhere in that reach, as page_droppable tells ink that matters. Both are drawn afresh
// from the page as it is now (without the strokes this sweep took off, which the page canvas may
// still show), over the page canvas, which gets its own pixels back. 1 or 0; -1 out of memory.
static int own_ink_shows(App *a, int i, Rect R, float x0, float y0, float x1, float y1, float r) {
    Canvas *pc = &S->page_canvas, keep, with;
    if (canvas_init(&keep, R.w, R.h) != 0) return -1;
    if (canvas_init(&with, R.w, R.h) != 0) {
        canvas_free(&keep);
        return -1;
    }
    canvas_blit(&keep, pc, R, 0, 0);  // (put back below)
    page_render(pc, S->page, R, a->dpi);
    canvas_blit(&with, pc, R, 0, 0);
    // Without it: taken off the list and put back in its place (the list then has room for it, so
    // page_insert cannot fail).
    Stroke *s = page_remove(S->page, i);
    page_render(pc, S->page, R, a->dpi);
    page_insert(S->page, i, s);
    bool shows = false;
    for (int y = 0; y < R.h && !shows; y++) {
        const uint8_t *without = &pc->px[(size_t)(R.y + y) * (size_t)pc->stride + (size_t)R.x];
        const uint8_t *drawn = &with.px[(size_t)y * (size_t)with.stride];
        float qy = (float)(R.y + y) + 0.5f;
        for (int x = 0; x < R.w && !shows; x++)
            shows = without[x] - drawn[x] > INK_SHOWS &&
                    dist2_to_segment((float)(R.x + x) + 0.5f, qy, x0, y0, x1, y1) <= r * r;
    }
    canvas_blit(pc, &keep, (Rect){0, 0, R.w, R.h}, R.x, R.y);
    canvas_free(&keep);
    canvas_free(&with);
    return shows;
}

// Whether the stroke eraser's segment (x0, y0)-(x1, y1), r wide each side, touches ink of stroke i
// (page px) that shows. A stroke no erase stroke drawn after it reaches shows wherever it is, as
// before 1.1. One that white ink or the precise eraser drawn after it reaches shows only where its
// own ink is on the page (own_ink_shows): not where they took it away, though the rest of it shows,
// nor where all there is to see is ink drawn later over that place, or a paper darker than it. That
// takes drawing the page there twice, so first the page canvas is held to the paper (S->paper):
// where nothing in reach is darker than the paper, nothing of it shows (scrubbing over white ink).
static bool touches_visible_ink(App *a, int i, float x0, float y0, float x1, float y1, float r) {
    const Stroke *s = S->page->strokes[i];
    bool covered = false;
    for (int k = i + 1; k < S->page->n && !covered; k++)
        covered = stroke_is_erase(S->page->strokes[k]) && rect_overlaps(S->page->strokes[k]->bbox, s->bbox);
    if (!covered) return true;
    const Canvas *pc = &S->page_canvas;
    if (S->canvas_page != S->page || !pc->px) return true;  // nothing to tell by: as before
    Rect R = rect_intersect(rect_intersect(rect_from_points(x0, y0, x1, y1, r + 1.f), s->bbox),
                            (Rect){0, 0, pc->w, pc->h});
    if (rect_is_empty(R)) return false;
    // A pixel the eraser touches anywhere has its centre within half its diagonal more (so even the
    // smallest eraser, zoomed in, touches a pixel wherever it is set down).
    float rp = r + 0.71f;
    const Canvas *paper = page_paper(a);
    if (paper && paper->w == pc->w && paper->h == pc->h && !dark_near(a, s, paper, R, x0, y0, x1, y1, rp)) return false;
    return own_ink_shows(a, i, R, x0, y0, x1, y1, rp) != 0;  // (out of memory: as before)
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
        // Erase strokes stay: taking one away would bring back the ink it covers. Ink they took
        // away is not there to touch (touches_visible_ink).
        if (stroke_is_erase(s) || !stroke_hit_segment(s, S->ex, S->ey, x, y, r, a->dpi) ||
            !touches_visible_ink(a, i, S->ex, S->ey, x, y, r))
            continue;
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
        draw_page(a, d);
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
    display_live_ink(false);
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
    if (refuse_readonly(a)) return;
    if (!S->page || S->page->n == 0) {
        app_toast(a, _("This page is already empty"), 1500);
        return;
    }
    History *h = cur_hist(a);
    if (!h) return;
    history_push_clear(h, S->page);  // takes every stroke off the page, undoable
    render_page_full(a);
    compose_all(a, REFRESH_FULL);
    mark_changed(a);
    repaint_panel(a, REFRESH_FULL);
    app_toast(a, _("Page cleared"), 1500);
}

// Whether a sheet is in front of the page: the Pages view, the tool picker, Custom music, Display
// or the paper sheet. The pen's button clears no page from under one.
static bool sheet_open(App *a) {
    return S->pv.open || S->tools_open || S->mb.open || S->dm.open || S->paper_open || S->ts.open;
}

// Side button: clicks and holds that happen while not drawing. A press used for something else (a
// stroke, or the pen on a button or a sheet while it is held) is neither a click nor a hold.
static void pen_button(App *a, const InEvent *ev) {
    uint32_t t = ev->t_ms;
    if (ev->side_button) {
        S->btn_down = true;
        S->btn_down_at = t;
        S->btn_used_for_ink = S->owner != OWN_NONE || sheet_open(a);
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
        if (S->owner == OWN_NONE && !sheet_open(a)) clear_page(a);
    }
}

static void undo_redo(App *a, bool undo) {
    settle_now(a);
    if (refuse_readonly(a)) return;
    History *h = cur_hist(a);
    Rect d;
    if (!h || !(undo ? history_undo(h, S->page, &d) : history_redo(h, S->page, &d))) return;
    draw_page(a, d);  // (back to a template whose picture is still being made: white until it is)
    bool full = d.w >= S->pgw && d.h >= S->pgh;
    show(a, d, full ? REFRESH_FULL : REFRESH_UI);
    mark_changed(a);
    repaint_panel(a, REFRESH_UI);
}

// ---- panel interaction ----

// ---- pages view: open, act, delete ----

static void pages_show(App *a) {
    save_page(a);
    if (!pagesview_open(&S->pv, a, &S->nb, S->index)) return;
    compose_all(a, REFRESH_FULL);
}

static void pages_hide(App *a, int open_index) {
    pagesview_close(&S->pv);
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
    int n = S->nb.npages, k = pagesview_ticked_count(&S->pv);
    if (k == 0) return;
    if (k == n) {
        // A notebook keeps at least one page: add a fresh blank one to stay behind.
        if (store_add_page(&S->nb, n, S->page ? S->page->paper : S->nb.paper) != 0) {
            app_toast(a, _("Could not delete those pages"), 2500);
            return;
        }
    }
    int cur = S->index, deleted_before = 0;
    for (int i = n - 1; i >= 0; i--) {
        if (!pagesview_ticked(&S->pv, i)) continue;
        uint32_t pid = S->nb.page_ids[i];
        if (store_delete_page(&S->nb, i) != 0) continue;  // the file goes to grimoire/trash
        forget_page(a, pid);
        if (i < cur) deleted_before++;
    }
    pagesview_untick(&S->pv);
    S->index = clampi(cur - deleted_before, 0, S->nb.npages - 1);
    S->pv.current = S->index;
    S->nb.last_page = S->index;
    S->page = page_at(a, S->index);
    if (S->page) render_page_full(a);
    S->changed = true;
    S->page_dirty = S->index == 0;  // refreshes the library thumbnail when page 1 changed
    S->info_dirty = true;
    save_page(a);
    char msg[256];
    // TRANSLATORS: deleted pages go to the trash folder on the Kindle's storage (%s).
    snprintf(msg, sizeof msg, NG_("Deleted %d page (kept in %s)", "Deleted %d pages (kept in %s)", k), k, "grimoire/trash");
    compose_all(a, REFRESH_FULL);
    app_toast(a, msg, 2500);
}

// A page added (or copied) at index `at` this session: the open page moves along if it was there or
// after, and the new page opens (the Pages view closes).
static void pages_added(App *a, int at) {
    if (at <= S->index) S->index++;
    S->nb.last_page = S->index;
    S->pv.current = S->index;
    S->changed = true;
    pages_hide(a, at);
}

// New page: the notebook's own paper (Blank in a book's notebook: annotate.h), at the index the
// Pages view gives, never the first, so the Library's card stays the same page. A page that holds
// no book page (in no line of source.ini) is a note page: a book page annotated later still goes
// before the first page that holds a later book page, after the notes on the one before.
static void pages_new(App *a, int at) {
    if (!save_page(a)) return;
    if (at < 1 || store_add_page(&S->nb, at, S->nb.paper) != 0) {
        app_toast(a, _("Could not add a page"), 3000);
        return;
    }
    pages_added(a, at);
}

// Copy page: page `index` copied right after it (store_duplicate_page: its ink and its paper, a
// picture shared), which opens. The copy is a note page; one with erase strokes is listed for
// compaction, as the page it copies may be.
static void pages_copy(App *a, int index) {
    if (!save_page(a)) return;
    if (store_duplicate_page(&S->nb, index, index + 1) != 0) {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Copy page in the
        // Pages view could not copy the page (the storage is full, or the notebook has 2000 pages).
        app_toast(a, _("Could not copy that page"), 3000);
        return;
    }
    pages_added(a, index + 1);
    bool erased = false;
    for (int i = 0; S->page && i < S->page->n && !erased; i++) erased = stroke_is_erase(S->page->strokes[i]);
    if (erased && S->index == index + 1) note_for_compaction(a, S->nb.page_ids[S->index]);
    char msg[160];
    // TRANSLATORS: a toast as the copy Copy page made opens (about 1700 px at 44 px in bold, or two
    // lines): %d is the page it copies.
    snprintf(msg, sizeof msg, _("This page is a copy of page %d."), index + 1);
    app_toast(a, msg, 2500);
}

static void pages_event(App *a, int ph, const InEvent *ev) {
    switch (pagesview_pointer(&S->pv, a, ph, ev)) {
    case PV_CLOSE: pages_hide(a, -1); break;
    case PV_OPEN: pages_hide(a, S->pv.target); break;
    case PV_DELETE: pages_delete(a); break;
    case PV_NEW: pages_new(a, S->pv.target); break;
    case PV_COPY: pages_copy(a, S->pv.target); break;
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

// ---- the tool picker's taps ----

// The tool picker closed (a pen picked, or a tap outside it), and its Pin mode with it.
static void close_tools(App *a) {
    S->tools_open = false;
    S->pin_mode = false;
    compose(a, S->tools_rect);
    app_invalidate(a, S->tools_rect, REFRESH_UI);
    repaint_panel(a, REFRESH_UI);
}

// The panel's buttons changed (a pen pinned or unpinned): it is fitted again where it is, and drawn
// with the picker beside it; where they no longer reach, the page shows again.
static void panel_changed(App *a) {
    Rect old = rect_union(S->panel, S->tools_open ? S->tools_rect : RECT_EMPTY);
    fit_panel_to(a, a->h);
    S->panel = panel_rect_at(a, S->panel.x, S->panel.y);
    compose(a, old);  // (draws the panel and the picker whole, where they are now)
    app_invalidate(a, rect_union(old, rect_union(S->panel, S->tools_open ? S->tools_rect : RECT_EMPTY)), REFRESH_UI);
}

// Pin mode: a pen pinned to the panel (after the others), or unpinned. Unpinned while in use, it
// goes to the tool button, so the panel still shows the pen that draws.
static void toggle_pin(App *a, ToolKind t) {
    Settings *st = &a->settings;
    bool was = settings_pinned(st, t);
    if (!settings_toggle_pin(st, t)) {
        char msg[200];
        // TRANSLATORS: a toast when the tool picker's Pin mode is asked for one more pen than the
        // panel holds: %d is how many it holds, 4 in this version. About 1700 px at 44 px in bold,
        // or two lines.
        snprintf(msg, sizeof msg, NG_("The panel holds up to %d pen. Unpin one first.",
                                      "The panel holds up to %d pens. Unpin one first.", PANEL_PINS_MAX),
                 PANEL_PINS_MAX);
        app_toast(a, msg, 2500);
        return;
    }
    if (was && st->current_tool == t) st->draw_tool = t;
    settings_save(st);
    panel_changed(a);
}

// A tap on the tool picker (the press is S->tools_press): a pen picked, which closes it, or in
// Pin mode pinned or unpinned; Pin or Done, which turn Pin mode on or off; an ink chosen. A tap
// outside it closes it; one inside, between its buttons, does nothing.
static void tools_tap(App *a) {
    const Press *p = &S->tools_press;
    const Hit *h = hits_find(&S->tools_hits, p->x0, p->y0);
    if (!h) {
        if (!rect_contains(S->tools_rect, (int)p->x0, (int)p->y0)) close_tools(a);
        return;
    }
    if (!press_is_tap(p, h->r)) return;
    Settings *st = &a->settings;
    switch (h->id) {
    case P_PIN_MODE:
        S->pin_mode = !S->pin_mode;
        draw_tools(a);
        app_invalidate(a, S->tools_rect, REFRESH_UI);
        break;
    case P_PICK_INK:
        st->white = k_inks[clampi(h->arg, 0, N_INKS - 1)].white;
        settings_save(st);
        draw_panel(a);  // the pen in use shows whether it draws white
        draw_tools(a);  // (over the panel where it covers it)
        app_invalidate(a, rect_union(S->panel, S->tools_rect), REFRESH_UI);
        break;
    case P_PICK_TOOL: {
        ToolKind t = (ToolKind)h->arg;
        if (S->pin_mode) {
            toggle_pin(a, t);  // (the panel and the picker drawn again, or a toast at the cap)
            break;
        }
        // A pinned pen picked is the one in use; the tool button keeps the pen it shows.
        st->current_tool = t;
        if (!settings_pinned(st, t)) st->draw_tool = t;
        settings_save(st);
        close_tools(a);
        break;
    }
    default: break;
    }
}

static void panel_act(App *a, const Hit *h) {
    switch (h->id) {
    case P_TOOLPICK: {
        // With a pinned pen in use, the tool button's own pen is one tap away, as a pin's is; a tap
        // while that pen is in use opens the picker. So does every tap on the grid (its pen pinned).
        ToolKind cur = a->settings.current_tool, dt = button_tool(a);
        if (cur != dt && settings_pinned(&a->settings, cur) && !settings_pinned(&a->settings, dt)) {
            a->settings.current_tool = dt;
            settings_save(&a->settings);
            repaint_panel(a, REFRESH_UI);
            break;
        }
        S->tools_open = true;
        S->pin_mode = false;
        S->tools_y = S->panel.y;
        draw_tools(a);
        app_invalidate(a, S->tools_rect, REFRESH_UI);
        break;
    }
    case P_TOOL: {
        ToolKind t = (ToolKind)h->arg, cur = a->settings.current_tool;
        if (t == cur && t != TOOL_ERASER) break;  // the pen in use already
        if (t == cur) {
            // Tapping the active eraser again goes back to the pen used just before it (a pinned
            // one, or the tool button's), or to the tool button's if that one is off the panel now.
            a->settings.current_tool = eraser_back(a);
        } else {
            if (t == TOOL_ERASER) a->settings.before_eraser = (int)cur;
            a->settings.current_tool = t;
        }
        settings_save(&a->settings);
        repaint_panel(a, REFRESH_UI);
        break;
    }
    case P_UNDO: undo_redo(a, true); break;
    case P_REDO: undo_redo(a, false); break;
    case P_PREV: go_to_page(a, S->index - 1); break;
    case P_NEXT:
        if (S->index >= S->nb.npages - 1) {
            if (!save_page(a)) return;
            if (store_add_page(&S->nb, S->nb.npages, S->page->paper) != 0) {
                app_toast(a, _("Could not add a page"), 3000);
                return;
            }
            S->changed = true;
        }
        go_to_page(a, S->index + 1);
        break;
    case P_PAPER:
        if (refuse_readonly(a)) break;
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
        app_toast(a, _("Saving..."), 8000);
        app_flush(a);
        if (!save_page(a)) return;  // stay with the unsaved page rather than drop it
        close_notebook(a);
        app_switch(a, SCR_LIBRARY);
        app_toast(a, _("Saved"), 1200);
        break;
    case P_COLLAPSE: set_collapsed(a, true); break;
    case P_EXPAND: set_collapsed(a, false); break;
    default: break;
    }
}

// Optional external buttons act only in the idle editor, never through an open sheet.
bool editor_shortcut(App *a, int action) {
    if (!S || !S->open || a->asleep || S->owner != OWN_NONE || S->pinching || sheet_open(a) || !action) return false;
    if (action == SHORTCUT_NEXT_TOOL || action == SHORTCUT_PREV_TOOL) {
        int index = -1;
        for (int i = 0; i < N_DRAW_TOOLS; i++) if (k_draw_tools[i] == a->settings.current_tool) index = i;
        index = action == SHORTCUT_NEXT_TOOL ? (index + 1) % N_DRAW_TOOLS : (index < 0 ? N_DRAW_TOOLS - 1 : (index + N_DRAW_TOOLS - 1) % N_DRAW_TOOLS);
        action = SHORTCUT_TOOL_BASE + k_draw_tools[index];
    }
    if (action >= SHORTCUT_TOOL_BASE && action < SHORTCUT_TOOL_BASE + TOOL_COUNT) {
        ToolKind t = (ToolKind)(action - SHORTCUT_TOOL_BASE);
        if (t != TOOL_ERASER && !settings_pinned(&a->settings, t)) a->settings.draw_tool = t;
        panel_act(a, &(Hit){.id = P_TOOL, .arg = t});
    } else if (action == SHORTCUT_UNDO || action == SHORTCUT_REDO) {
        panel_act(a, &(Hit){.id = action == SHORTCUT_UNDO ? P_UNDO : P_REDO});
    } else if (action == SHORTCUT_PREV_PAGE || action == SHORTCUT_NEXT_PAGE) {
        // A controller page button never creates an unrequested new page at the end.
        go_to_page(a, clampi(S->index + (action == SHORTCUT_NEXT_PAGE ? 1 : -1), 0, S->nb.npages - 1));
    } else if (action == SHORTCUT_WHITE) {
        a->settings.white = !a->settings.white;
        settings_save(&a->settings);
        repaint_panel(a, REFRESH_UI);
    } else return false;
    return true;
}

static void apply_paper(App *a, PaperKind k) {
    if (refuse_readonly(a)) return;
    History *hi = cur_hist(a);
    if (hi) history_push_paper(hi, S->page->paper, k);
    S->page->paper = k;
    render_page_full(a);
    compose_all(a, REFRESH_FULL);
    mark_changed(a);
}

// A template picked on the Templates sheet becomes this page's paper, as any paper does (one undo
// step): template_install gives its paper value at once, and the page draws white until the
// background job has made its picture for this notebook (editor_tick then draws it). One that
// cannot be installed changes nothing.
static void apply_template(App *a, const char *path) {
    PaperKind k;
    const char *why = "";
    if (template_install(&S->nb, path, &k, &why) != 0) {
        qlog("editor: %s: a template could not be used: %s", S->nb.id, why);
        char msg[256];
        // TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines): a template picked
        // on the Templates sheet could not become the page's paper (or the new notebook's); %s is
        // the log file that says why.
        snprintf(msg, sizeof msg, _("Could not use that template (see %s)."), "grimoire.log");
        app_toast(a, msg, 4000);
        return;
    }
    if (S->page->paper != k) apply_paper(a, k);
}

static void modal_act(App *a, const Hit *h) {
    Rect sheet = S->paper_rect;
    S->paper_open = false;
    if (h->id == P_PICK_PAPER && S->page->paper != (PaperKind)h->arg) {
        apply_paper(a, (PaperKind)h->arg);
        return;
    }
    if (h->id == P_PICK_MORE) {
        compose(a, sheet);  // (the Templates sheet takes the Paper sheet's place)
        tplsheet_open(&S->ts, a, &S->nb, S->page->paper, NULL);
        app_invalidate(a, rect_union(sheet, tplsheet_draw(&S->ts, a)), REFRESH_UI);
        return;
    }
    if (h->id == P_PICK_CUSTOM) {
        compose(a, sheet);
        musicbuild_open_page(&S->mb, a, S->page->paper, S->pgw, S->pgh);
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
    S->tool_button_shows = TB_NONE;  // (no panel drawn yet)
    // Erased ink an earlier run left in page files (it left with the notebook open, or a compaction
    // was stopped): taken out now, in the background, unless the notebook is opened first.
    compact_queue_listed(a->dpi);
}

void editor_free(App *a) {
    if (!S) return;
    close_notebook(a);
    stroke_free(S->cur);
    free(S->snap);
    for (int k = 0; k < S->nremoved; k++) stroke_free(S->removed[k]);
    free(S->removed);
    pagesview_free(&S->pv);
    canvas_free(&S->page_canvas);
    for (int k = 0; k < PAPER_COUNT; k++) canvas_free(&S->paper_prev[k]);
    canvas_free(&S->custom_prev);
    canvas_free(&S->image_prev);
    canvas_free(&S->paper);
    free(S->noted);
    musicbuild_free(&S->mb);
    tplsheet_close(&S->ts);
    free(S);
    a->ed = NULL;
}

// Every sheet over the page closed: the paper sheet, Display, Custom music, the tool picker (and its
// Pin mode). A sheet closed here stays open through a turn or a wake (editor_enter), laid out again
// by compose for the new size, as each of these lays itself out where it is drawn. (The Pages view
// closes with its notebook, and stays open through those too.)
static void close_sheets(App *a) {
    S->paper_open = false;
    S->dm.open = false;
    S->mb.open = false;
    S->tools_open = false;
    S->pin_mode = false;
    tplsheet_close(&S->ts);
}

void editor_enter(App *a) {
    if (!S->page_canvas.px && canvas_init(&S->page_canvas, S->pgw, S->pgh) != 0) return;
    if (S->open && S->page && S->canvas_page != S->page) render_page_full(a);
    if (S->view_w != a->w || S->view_h != a->h) view_reset(a);  // new notebook or turned
    S->pinching = false;
    fit_panel_to(a, a->h);
    S->collapsed = a->settings.panel_collapsed;
    int x = a->settings.panel_x, y = a->settings.panel_y;
    if (x < 0 || y < 0) {
        x = a->w - (S->collapsed ? box() : pw()) - ui_px(28);
        y = ui_px(220);
    }
    S->panel = panel_rect_at(a, x, y);
    S->tools_y = S->panel.y;
    S->owner = OWN_NONE;
    // The pen in use always has its button on the panel. One that has none (a pin a version without
    // pins dropped, or one taken out of settings.ini by hand) goes to the tool button.
    Settings *st = &a->settings;
    if ((unsigned)st->current_tool < TOOL_COUNT && st->current_tool != TOOL_ERASER &&
        st->current_tool != button_tool(a) && !settings_pinned(st, st->current_tool)) {
        st->draw_tool = st->current_tool;
        settings_save(st);
    }
    // A turn, or the redraw after waking (a->relayout), lays out the same screen again: the sheet
    // that was open stays open. Coming in from another screen, none is.
    if (!a->relayout) close_sheets(a);
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
        if (S->owner != OWN_NONE || S->paper_open || S->mb.open || S->ts.open || !a->settings.pinch_zoom) return;
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

bool editor_suspend(App *a) {
    if (!S || !S->open) return true;
    // Native unlock will consume the pen/finger releases. Commit live ink before yielding input.
    if (S->owner == OWN_DRAW) draw_end(a);
    if (S->owner == OWN_ERASE) erase_end(a);
    S->owner = OWN_NONE;
    S->has_pressed = S->dragging = S->pinching = false;
    S->btn_down = false;
    S->nclicks = 0;
    return save_page(a);
}

void editor_event(App *a, const InEvent *ev) {
    if (!S->open) return;
    if (ev->type == IN_PEN_BUTTON) {
        pen_button(a, ev);
        return;
    }
    if (ev->type == IN_PINCH_BEGIN || ev->type == IN_PINCH_MOVE || ev->type == IN_PINCH_END) {
        if (!S->pv.open) pinch(a, ev);
        return;
    }
    bool pen = ev->type == IN_PEN_DOWN || ev->type == IN_PEN_MOVE || ev->type == IN_PEN_UP ||
               ev->type == IN_PEN_LEAVE;
    // A pen leaving proximity mid-gesture ends it.
    InType type = ev->type == IN_PEN_LEAVE ? IN_PEN_UP : ev->type;
    int ph = (type == IN_PEN_UP) ? 2 : app_pointer_phase(ev);
    if (ph < 0) return;
    if (ph == 0) {
        if (pen && S->btn_down) S->btn_used_for_ink = true;  // (pen_button: no click, no hold)
        if (S->owner != OWN_NONE || S->pinching) return;  // one gesture at a time
        S->owner_is_pen = pen;
        if (S->pv.open) {
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
        if (S->ts.open) {
            S->owner = OWN_TEMPLATES;
            Rect d;
            tplsheet_pointer(&S->ts, a, 0, ev->x, ev->y, ev->t_ms, &d);
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
            // A stroke just ended: Undo/Clear may have become available, but the panel (and its
            // hit list) is only rebuilt at the settle. Settle now so the tap finds the live buttons.
            if (S->panel_stale) settle_now(a);
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
        tools_tap(a);
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
    case OWN_TEMPLATES: {
        Rect d;
        TsResult r = tplsheet_pointer(&S->ts, a, ph, ev->x, ev->y, ev->t_ms, &d);
        if (r == TS_PICKED) {
            char path[TEMPLATE_PATH_MAX];
            const char *p = tplsheet_path(&S->ts);
            strlcpy_q(path, p ? p : "", sizeof path);
            tplsheet_close(&S->ts);
            compose(a, d);
            app_invalidate(a, d, REFRESH_UI);
            if (path[0]) apply_template(a, path);
        } else if (r == TS_BACK) {
            // Back to the Paper sheet, in the same place.
            compose(a, d);
            S->paper_open = true;
            draw_paper_sheet(a);
            app_invalidate(a, rect_union(d, S->paper_rect), REFRESH_UI);
        } else if (r == TS_CANCEL) {
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

// A background job ended since the open page (page), or a cache (caches: note_white), drew a
// picture white (a template's, being made for this notebook): they are drawn again, with the
// picture if it is there now, and so is what shows them.
static void pictures_arrived(App *a, bool page, bool caches) {
    if (caches) {
        S->white_caches = false;
        canvas_free(&S->paper);
        canvas_free(&S->image_prev);
        }
    if (page) render_page_full(a);
    if (page && !S->picture_missing) {  // (still missing: another job ended, this one is to come)
        if (S->index == 0) S->thumb_dirty = true;  // the Library's card of the notebook
        compose_all(a, REFRESH_FULL);
    } else if (caches && S->pv.open) {
        compose_all(a, REFRESH_UI);
    } else if (caches && S->paper_open) {
        draw_paper_sheet(a);
        app_invalidate(a, S->paper_rect, REFRESH_UI);
    }
}

void editor_tick(App *a, uint32_t now) {
    if (S->open && S->owner == OWN_NONE && (!rect_is_empty(S->settle) || S->panel_stale) &&
        (int32_t)(now - S->settle_at) >= 0)
        settle_now(a);
    // (Never under a stroke: an erase stroke restores toward S->paper as it goes.)
    if (S->open && S->owner == OWN_NONE && !S->pinching) {
        bool page = S->picture_missing && S->picture_gen != bgimage_generation();
        bool caches = S->white_caches && S->white_gen != bgimage_generation();
        if (page || caches) pictures_arrived(a, page, caches);
    }
    Rect d;
    if (S->open && S->owner == OWN_NONE && tplsheet_tick(&S->ts, a, now, &d)) app_invalidate(a, d, REFRESH_UI);
    // Holding the button ~1.2 s while hovering (no ink) clears the page once: not a hold during
    // which a sheet was in front of the page, nor one the pen pressed anything during.
    if (S->btn_down && sheet_open(a)) S->btn_used_for_ink = true;
    if (S->open && S->btn_down && !S->btn_used_for_ink && !S->btn_hold_fired && S->owner == OWN_NONE &&
        !sheet_open(a) && now - S->btn_down_at >= 1200) {
        S->btn_hold_fired = true;
        S->nclicks = 0;
        clear_page(a);
    }
    // Autosave after a real pause (and never while the pen is down). One that fails (a full store)
    // tries again 2 s later, not at every tick: each try says so with a toast.
    if (S->open && S->page_dirty && S->owner == OWN_NONE && now - S->last_up > 2000 && !save_page(a)) S->last_up = now;
    // A page whose picture was made while a stroke or gesture was under way (editor_picture_made).
    if (S->open && S->page && S->canvas_page != S->page && S->page_canvas.px && S->owner == OWN_NONE && !S->pinching) {
        render_page_full(a);
        compose_all(a, REFRESH_UI);
    }
}

// ---- headless checks ----

// The script runner's "editor.*" keys (app_probe forwards them without the prefix).
int editor_probe(App *a, const char *key, char *out, size_t cap) {
    if (!S) return -1;
    bool has_page = S->open && S->page;
    if (!strcmp(key, "page")) {
        snprintf(out, cap, "%d", S->open ? S->index + 1 : 0);
    } else if (!strcmp(key, "loaded_pages")) {
        snprintf(out, cap, "%d", S->open ? S->nhist : 0);
    } else if (!strcmp(key, "pages")) {
        snprintf(out, cap, "%d", S->open ? S->nb.npages : 0);
    } else if (!strcmp(key, "tool")) {
        const char *name = app_probe_tool(a->settings.current_tool);  // a tool it lacks: its number
        if (name) snprintf(out, cap, "%s", name);
        else snprintf(out, cap, "%d", (int)a->settings.current_tool);
    } else if (!strcmp(key, "zoom")) {
        snprintf(out, cap, "%.2f", S->zoom / S->zoom_default);  // as the zoom chip shows it: 1.00 = not zoomed
    } else if (!strcmp(key, "page_size")) {
        // The open page's own size: its notebook's, whatever the screen (a page that came at the
        // screen's size instead would show here).
        if (has_page) snprintf(out, cap, "%dx%d", S->page->w, S->page->h);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "scale")) {
        snprintf(out, cap, "%.2f", S->open ? S->zoom : 0.f);  // screen px per page px
    } else if (!strcmp(key, "zoomed")) {
        snprintf(out, cap, "%d", S->open && zoomed(a) ? 1 : 0);  // the zoom chip shows
    } else if (!strcmp(key, "stroke_start")) {
        // Where the page's newest stroke starts, in page px: ink lands under the pen through the view.
        const Stroke *st = has_page && S->page->n ? S->page->strokes[S->page->n - 1] : NULL;
        if (st && st->n) snprintf(out, cap, "%.0f %.0f", st->pts[0].x, st->pts[0].y);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "music_max")) {
        // The most systems the Custom music sheet's layout fits on the page it is for.
        if (S->mb.open) snprintf(out, cap, "%d", music_max_systems(&S->mb.m, S->mb.page_h, a->dpi));
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "modal")) {
        // What is in front (input goes to the first one open, in this order).
        const char *m = S->pv.open      ? "pages"
                        : S->tools_open ? "tools"
                        : S->mb.open    ? "music"
                        : S->dm.open    ? "display"
                        : S->ts.open    ? "templates"
                        : S->paper_open ? "paper"
                                        : "none";
        snprintf(out, cap, "%s", m);
    } else if (!strcmp(key, "strokes")) {
        snprintf(out, cap, "%d", has_page ? S->page->n : 0);
    } else if (!strcmp(key, "erase_strokes")) {
        int n = 0;
        for (int i = 0; has_page && i < S->page->n; i++) n += stroke_is_erase(S->page->strokes[i]);
        snprintf(out, cap, "%d", n);
    } else if (!strcmp(key, "live_matches_render")) {
        // The open page drawn afresh, pixel for pixel what drawing, erasing and undoing left in the
        // page canvas: 1 when they are the same.
        bool same = false;
        Canvas c;
        if (has_page && S->canvas_page == S->page && S->page_canvas.px && canvas_init(&c, S->pgw, S->pgh) == 0) {
            page_render(&c, S->page, (Rect){0, 0, S->pgw, S->pgh}, a->dpi);
            same = S->page_canvas.w == c.w && S->page_canvas.h == c.h && S->page_canvas.stride == c.stride &&
                   !memcmp(c.px, S->page_canvas.px, (size_t)c.stride * (size_t)c.h);
            canvas_free(&c);
        }
        snprintf(out, cap, "%d", same ? 1 : 0);
    } else if (!strcmp(key, "stroke_screen_mm")) {
        // The newest stroke's widest, as the screen shows it at the zoom now (the precise eraser's
        // keeps its setting at any zoom); empty with no stroke.
        const Stroke *st = has_page && S->page->n ? S->page->strokes[S->page->n - 1] : NULL;
        if (st) snprintf(out, cap, "%.2f", st->params.size_max_mm * S->zoom);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "stroke_look")) {
        // The newest stroke's look: its tool for ink ("pencil"), "white" and its texture for white
        // ink ("white charcoal"), "round tip" for the precise eraser; empty with no stroke.
        const Stroke *st = has_page && S->page->n ? S->page->strokes[S->page->n - 1] : NULL;
        ToolKind t = st ? stroke_render_tool(st) : TOOL_PENCIL;
        const char *name = app_probe_tool(t);
        if (!st) snprintf(out, cap, "%s", "");
        else if (!stroke_is_erase(st)) snprintf(out, cap, "%s", name ? name : "?");
        else if (t == TOOL_ERASER) snprintf(out, cap, "%s", "round tip");
        else snprintf(out, cap, "white %s", name ? name : "?");
    } else if (!strcmp(key, "stroke_settings")) {
        // Whose pen feel the newest stroke was drawn with: the tool whose settings its parameters
        // are (its look's tool tried first), "eraser" also for the eraser's at the zoom now (the
        // precise eraser's widths are divided by it), "other" for none; empty with no stroke.
        const Stroke *st = has_page && S->page->n ? S->page->strokes[S->page->n - 1] : NULL;
        const char *who = NULL;
        ToolKind look = st ? stroke_render_tool(st) : TOOL_PENCIL;
        for (int k = -1; st && !who && k < TOOL_COUNT; k++) {
            ToolKind t = k < 0 ? look : (ToolKind)k;
            if (!memcmp(&st->params, &a->settings.tools[t], sizeof st->params)) who = app_probe_tool(t);
        }
        ToolParams ez = eraser_at_zoom(&a->settings.tools[TOOL_ERASER], S->zoom);
        if (st && !who && !memcmp(&st->params, &ez, sizeof ez)) who = app_probe_tool(TOOL_ERASER);
        snprintf(out, cap, "%s", !st ? "" : who ? who : "other");
    } else if (!strcmp(key, "compact_listed")) {
        // 1 while the open notebook's erased.txt lists pages for compaction
        snprintf(out, cap, "%d", S->open && compact_listed(S->nb.id) ? 1 : 0);
    } else if (!strcmp(key, "paper_cache")) {
        snprintf(out, cap, "%d", S->paper.px ? 1 : 0);  // the paper erase strokes restore toward
    } else if (!strcmp(key, "matches_closed")) {
        // The open page shows as its file did when its notebook last closed to be compacted (1 the
        // same, 0 not); empty when that was another notebook or page.
#ifdef GRIMOIRE_SIM
        bool known = has_page && S->canvas_page == S->page && S->page_canvas.px && !strcmp(g_closed.id, S->nb.id) &&
                     g_closed.page == S->nb.page_ids[S->index];
        if (known) snprintf(out, cap, "%d", canvas_hash(&S->page_canvas) == g_closed.hash ? 1 : 0);
        else snprintf(out, cap, "%s", "");
#else
        snprintf(out, cap, "%s", "");
#endif
    } else if (!strcmp(key, "unsaved")) {
        snprintf(out, cap, "%d", S->open && (S->page_dirty || S->info_dirty) ? 1 : 0);
    } else if (!strcmp(key, "ticked")) {
        snprintf(out, cap, "%d", S->pv.open ? pagesview_ticked_count(&S->pv) : 0);
    } else if (!strcmp(key, "paper")) {
        if (!has_page) snprintf(out, cap, "none");
        else if (paper_is_custom(S->page->paper)) snprintf(out, cap, "custom");
        else snprintf(out, cap, "%d", (int)S->page->paper);
    } else if (!strcmp(key, "paper_hex")) {
        // The page's paper value as 8 hex digits (an image paper: 0x50 for a template, 0x51 for an
        // imported page, then its picture's id), empty with no notebook open.
        if (has_page) snprintf(out, cap, "0x%08x", (unsigned)S->page->paper);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "picture")) {
        // Whether the page's picture (an image paper's) shows: "shown", or "missing" while the page
        // draws white for it (a template's picture not made yet); empty for any other paper.
        if (has_page && paper_is_image(S->page->paper)) snprintf(out, cap, "%s", S->picture_missing ? "missing" : "shown");
        else snprintf(out, cap, "%s", "");
    } else if (!strncmp(key, "tile.", 5)) {
        return pagesview_probe(&S->pv, key, out, cap);  // the Pages view's
    } else if (!strncmp(key, "templates.", 10)) {
        return tplsheet_probe(&S->ts, key + 10, out, cap);  // the Templates sheet over the page
    } else if (!strcmp(key, "panel")) {
        snprintf(out, cap, "%s", S->collapsed ? "collapsed" : "expanded");
    } else if (!strcmp(key, "panel_rect")) {
        snprintf(out, cap, "%d %d %d %d", S->panel.x, S->panel.y, S->panel.w, S->panel.h);  // as toast.rect
    } else if (!strcmp(key, "panel_button")) {
        snprintf(out, cap, "%d", btn_h());  // an expanded panel's buttons' height, px
    } else if (!strcmp(key, "panel_fits")) {
        // 1 while the panel is inside the screen with its margins free above and below it, its
        // buttons at full size (as without pins, upright or sideways)
        Rect p = S->panel;
        int m = ui_px(PANEL_MARGIN);
        bool in = p.x >= 0 && p.x + p.w <= a->w && p.y >= m && p.y + p.h <= a->h - m;
        snprintf(out, cap, "%d", in && btn_h() == ui_px(100) ? 1 : 0);
    } else if (!strcmp(key, "pins")) {
        settings_pins_text(&a->settings, out, cap);  // the pinned pens in pin order, as settings.ini names them
    } else if (!strcmp(key, "draw_tool")) {
        const char *name = app_probe_tool(button_tool(a));  // the tool button's pen
        snprintf(out, cap, "%s", name ? name : "?");
    } else if (!strcmp(key, "tool_button")) {
        // What the tool button showed as the panel was last drawn: its pen, "grid" while that pen
        // is pinned, empty while the panel is collapsed.
        int t = S->tool_button_shows;
        const char *name = t >= 0 ? app_probe_tool((ToolKind)t) : NULL;
        snprintf(out, cap, "%s", t == TB_GRID ? "grid" : t == TB_NONE ? "" : name ? name : "?");
    } else if (!strcmp(key, "pin_mode")) {
        snprintf(out, cap, "%d", S->tools_open && S->pin_mode ? 1 : 0);
    } else if (!strcmp(key, "tools_rect")) {
        // Where the tool picker is while it is open, "x y w h" (as toast.rect), else empty.
        Rect r = S->tools_rect;
        if (S->tools_open) snprintf(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        else snprintf(out, cap, "%s", "");
    } else if (!strcmp(key, "before_eraser")) {
        // The pen used before the eraser (settings.ini before_eraser: where a tap on the eraser in
        // use goes back to, if it is still on the panel), empty for none.
        int b = a->settings.before_eraser;
        const char *name = b >= 0 && b < TOOL_COUNT ? app_probe_tool((ToolKind)b) : NULL;
        snprintf(out, cap, "%s", name ? name : "");
    } else if (!strcmp(key, "white_mark")) {
        snprintf(out, cap, "%d", S->white_mark ? 1 : 0);  // the panel as drawn last shows the pen draws white
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
