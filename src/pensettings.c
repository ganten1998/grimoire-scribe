// pensettings.c: per-tool pressure / tilt sensitivity. Tabs per tool, one slider per
// tweakable parameter, a live pressure curve, a raw-vs-mapped readout, and a test pad that
// draws with the tool being tuned.
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "erasersample.h"

enum { H_BACK = 1, H_TAB, H_RESET, H_CLEAR, H_SLIDER, H_PAD, H_ERASER_MODE };

struct PenSetState {
    ToolKind tab;
    ScreenId ret;
    HitList hits;
    Press press;
    Hit pressed;
    bool has_pressed;
    // slider drag
    int active;            // param table index being dragged, -1 none
    Slider sl;
    SliderDrag slider_drag;
    bool slider_dirty;
    uint32_t last_paint;
    // test pad
    Rect pad, graph, readout;
    bool drawing;
    bool pen_owner;
    BrushState bs;
    Smoother sm;
    // The precise eraser (eraser_mode 1) and white ink (white) on the pad: erase strokes, as in the
    // editor, restoring the pad as it was drawn bare (its lines and its Clear button), copied here
    // when it is. White on that shows nothing: the pad then has a black swatch to try it on.
    Canvas paper;
    Rect swatch;           // empty when there is none
    Page *sample;        // owned copy of the handwritten practice ink
    Rect sample_box;
    float wipe_x, wipe_y;
    bool erasing;          // the stroke on the pad is the precise eraser's
    bool whiting;          // the stroke on the pad is white ink
    bool wiping;           // whole-stroke eraser: hits practice ink on its tab, clears the other pads
    float live_raw;        // last pen pressure seen on the pad (-1 none)
    float live_tx, live_ty;
    uint32_t last_readout;
    Rect slider_rows[16];
    int slider_param[16];
    int nsliders;
};

#define S (a->ps)

static bool visible_param(const ParamInfo *pi, ToolKind t) { return t != TOOL_ERASER || pi->eraser_uses; }

static void value_text(const ParamInfo *pi, float v, char *buf, size_t cap) {
    // TRANSLATORS: an angle on Pen feel's sliders and in its readout ("45 deg"); "%d°" is fine.
    if (!strcmp(pi->unit, "deg")) snprintf(buf, cap, _("%d deg"), (int)lrintf(v));
    else if (!strcmp(pi->unit, "mm")) snprintf(buf, cap, "%.2f mm", v);
    // TRANSLATORS: the value of Pen feel's Tilt widens slider: how much wider the line gets with the
    // pen on its side, a percentage ("+35%"; in Spanish "+35 %", the usual spacing), up
    // to +400%. Right of its slider's name, at 44 px in bold: the two share 740 px, the value takes
    // what it needs (about 150 px for "+400 %" in Gothic, 120 in Classic) and the name the rest.
    else if (!strcmp(pi->unit, "x")) snprintf(buf, cap, _("+%.0f%%"), v * 100.f);
    else snprintf(buf, cap, "%.2f", v);
}

static void draw_graph(App *a) {
    Canvas *c = &a->screen;
    Rect g = S->graph;
    const ToolParams *p = &a->settings.tools[S->tab];
    // Clear enough margin for the live dot (it can sit on the graph's edge).
    canvas_fill(c, rect_inflate(g, ui_px(26)), 255);
    canvas_rect_outline(c, g, ui_px(3), 0);
    // Quarter grid.
    for (int i = 1; i < 4; i++) {
        canvas_hline(c, g.x, g.x + g.w, g.y + g.h * i / 4, 1, 200);
        canvas_vline(c, g.x + g.w * i / 4, g.y, g.y + g.h, 1, 200);
    }
    // Deadzone + "full at" markers.
    int fx = g.x + (int)(p->pressure_floor * g.w), cx = g.x + (int)(p->pressure_ceiling * g.w);
    for (int y = g.y; y < g.y + g.h; y += ui_px(24)) {
        canvas_vline(c, fx, y, y + ui_px(12), ui_px(3), 120);
        canvas_vline(c, cx, y, y + ui_px(12), ui_px(3), 120);
    }
    float px = 0, py = 0;
    for (int i = 0; i <= 64; i++) {
        float x = (float)i / 64.f;
        float y = brush_pressure(p, x);
        float sx = g.x + x * g.w, sy = g.y + g.h - y * g.h;
        if (i) canvas_line_aa(c, px, py, sx, sy, (float)ui_px(6), 0);
        px = sx;
        py = sy;
    }
    if (S->live_raw >= 0.f) {
        float y = brush_pressure(p, S->live_raw);
        canvas_disc_aa(c, g.x + S->live_raw * g.w, g.y + g.h - y * g.h, (float)ui_px(18), 0, 1.f);
        canvas_disc_aa(c, g.x + S->live_raw * g.w, g.y + g.h - y * g.h, (float)ui_px(8), 255, 1.f);
    }
    float ss = ui_small_size();
    // TRANSLATORS: the axes of Pen feel's pressure graph: under it, how hard the pen presses; over
    // it, how much ink that puts down. Up to 570 px at 34 px (the test pad starts there).
    font_draw(c, FONT_REGULAR, ss, g.x, g.y + g.h + ui_px(44), _("pen pressure"), 60);
    // TRANSLATORS: over Pen feel's pressure graph (see pen pressure): how much ink.
    font_draw(c, FONT_REGULAR, ss, g.x, g.y - ui_px(14), _("ink"), 60);
}

static void draw_readout(App *a) {
    Canvas *c = &a->screen;
    canvas_fill(c, S->readout, 255);
    const ToolParams *p = &a->settings.tools[S->tab];
    char line[200];
    if (S->live_raw < 0.f) {
        // TRANSLATORS: Pen feel's readout line, until the pen draws on the test pad (1760 px at
        // 34 px; the readings below take that room too).
        snprintf(line, sizeof line, "%s", _("Draw on the pad to see live pressure and tilt."));
    } else {
        PenSample s = {0, 0, S->live_raw, S->live_tx, S->live_ty, 0};
        float r, op;
        brush_dab(S->tab, p, &s, a->dpi, &r, &op);
        float tilt_deg = atanf(sqrtf(powf(tanf(S->live_tx * (float)M_PI / 180.f), 2) +
                                     powf(tanf(S->live_ty * (float)M_PI / 180.f), 2))) * 180.f / (float)M_PI;
        const InputCaps *caps = input_caps();
        char tilt[96];
        if (caps && !caps->has_tilt) {
            // TRANSLATORS: the tilt in Pen feel's readout (below), from a pen that does not report it.
            snprintf(tilt, sizeof tilt, "%s", _("tilt: not reported"));
        } else {
            // TRANSLATORS: the tilt in Pen feel's readout (below): the pen's lean, and what it counts
            // for (0 to 1).
            snprintf(tilt, sizeof tilt, _("tilt %d deg -> %.2f"), (int)lrintf(tilt_deg), brush_tilt(p, S->live_tx, S->live_ty));
        }
        if (S->tab == TOOL_CALLIGRAPHY) {
            // TRANSLATORS: Pen feel's readout while the calligraphy pen draws on the test pad, two
            // spaces apart: the raw pressure and what the curve makes of it, the tilt (above), the
            // nib's width and angle (1760 px at 34 px).
            snprintf(line, sizeof line, _("pressure %.2f -> %.2f  %s  nib %.2f mm at %d deg"), S->live_raw,
                     brush_pressure(p, S->live_raw), tilt, r * 2.f * 25.4f / a->dpi,
                     (int)lrintf(brush_nib_angle(p, S->live_tx, S->live_ty)));
        } else {
            // TRANSLATORS: Pen feel's readout while a tool draws on the test pad, two spaces apart:
            // the raw pressure and what the curve makes of it, the tilt (above), the line's width
            // and its ink (0 to 1) (1760 px at 34 px).
            snprintf(line, sizeof line, _("pressure %.2f -> %.2f  %s  width %.2f mm  ink %.2f"), S->live_raw,
                     brush_pressure(p, S->live_raw), tilt, r * 2.f * 25.4f / a->dpi, op);
        }
    }
    font_draw_left(c, FONT_REGULAR, ui_small_size(), S->readout, line, 0);
}

static void draw_slider_row(App *a, int i) {
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    const ParamInfo *p = &pi[S->slider_param[i]];
    Slider sl = {.min = p->min, .max = p->max, .step = p->step, .label = _(p->label), .unit = p->unit};
    // The calligraphy nib turns with the pen's lean once the pen tilts this far.
    // TRANSLATORS: the calligraphy pen's names for "Tilt starts at" and "Tilt full at" (570 px at
    // 44 px in Gothic, 600 in Classic, beside an angle).
    if (S->tab == TOOL_CALLIGRAPHY && !strcmp(p->key, "tilt_min_deg")) sl.label = _("Nib follows lean from");
    if (S->tab == TOOL_CALLIGRAPHY && !strcmp(p->key, "tilt_max_deg")) sl.label = _("Nib fully follows at");
    sl.value = *tool_param_ptr(&a->settings.tools[S->tab], p);
    ui_slider_layout(&sl, S->slider_rows[i]);
    char vt[32];
    value_text(p, sl.value, vt, sizeof vt);
    ui_slider_draw(&a->screen, &sl, vt);
}

static Rect clear_rect(App *a) {
    return (Rect){S->pad.x + S->pad.w - ui_px(220), S->pad.y + ui_px(14), ui_px(206), ui_px(100)};
}

// White ink on the pad: while the drawing tools draw white (settings.white), the pen draws white
// on a drawing tool's tab, with its settings and texture, as in the editor.
static bool pad_white(App *a) { return a->settings.white && S->tab != TOOL_ERASER; }

static void pad_targets_free(App *a) {
    page_free(S->sample);
    S->sample = NULL;
    S->sample_box = RECT_EMPTY;
}

static void pad_targets_draw(App *a) {
    if (!S->sample) return;
    Rect saved = a->screen.clip;
    canvas_set_clip(&a->screen, S->pad);
    for (int i = 0; i < S->sample->n; i++) {
        Stroke *s = S->sample->strokes[i];
        BrushState b;
        stroke_brush_begin(&b, s, a->dpi, S->paper.px, S->pad.x, S->pad.y, S->paper.stride);
        for (int j = 0; j < s->n; j++) brush_add(&b, &a->screen, &s->pts[j]);
    }
    a->screen.clip = saved;
}

static void pad_targets_seed(App *a) {
    Page *p = NULL;
    if (page_deserialize(grimoire_eraser_sample, grimoire_eraser_sample_len, &p) != 0 || !p || p->readonly) {
        page_free(p);
        return;
    }
    // The source was written at 300 dpi. Fit its ink, without its page's empty margins or paper.
    Rect ink = RECT_EMPTY;
    for (int i = 0; i < p->n; i++) {
        Stroke *s = p->strokes[i];
        float r = brush_max_radius(stroke_render_tool(s), &s->params, 300.f) + 2.f;
        for (int j = 0; j < s->n; j++)
            ink = rect_union(ink, rect_from_points(s->pts[j].x, s->pts[j].y, s->pts[j].x, s->pts[j].y, r));
    }
    if (rect_is_empty(ink)) { page_free(p); return; }
    Rect area = {S->pad.x + ui_px(30), S->pad.y + ui_px(130), S->pad.w - ui_px(60), S->pad.h - ui_px(260)};
    float scale = fminf((float)area.w / ink.w, (float)area.h / ink.h);
    float x = area.x + (area.w - ink.w * scale) / 2.f;
    float y = area.y + (area.h - ink.h * scale) / 2.f;
    S->sample_box = (Rect){(int)lrintf(x), (int)lrintf(y), (int)lrintf(ink.w * scale), (int)lrintf(ink.h * scale)};
    for (int i = 0; i < p->n; i++) {
        Stroke *s = p->strokes[i];
        int n = s->n;
        s->params.size_min_mm *= scale * 300.f / a->dpi;
        s->params.size_max_mm *= scale * 300.f / a->dpi;
        s->n = 0; s->bbox = RECT_EMPTY;
        for (int j = 0; j < n; j++) {
            PenSample q = s->pts[j];
            q.x = x + (q.x - ink.x) * scale;
            q.y = y + (q.y - ink.y) * scale;
            stroke_append(s, &q, a->dpi);
        }
    }
    S->sample = p;
    pad_targets_draw(a);
}

// With the precise eraser or white ink, the pad as just drawn bare is its paper: they restore it,
// lines and Clear button, as they restore a page's paper in the editor.
static void pad_paper_take(App *a) {
    if (a->settings.eraser_mode != 1 && !pad_white(a) && S->tab != TOOL_ERASER) {
        canvas_free(&S->paper);
        return;
    }
    if (S->paper.px && (S->paper.w != S->pad.w || S->paper.h != S->pad.h)) canvas_free(&S->paper);
    if (!S->paper.px && canvas_init(&S->paper, S->pad.w, S->pad.h) != 0) return;
    canvas_blit(&S->paper, &a->screen, S->pad, 0, 0);
}

static void draw_pad_bare(App *a) {
    Canvas *c = &a->screen;
    canvas_fill(c, S->pad, 255);
    // A few ruled lines so stroke widths read like they will on paper.
    float pitch = mm_to_px(7.14f, a->dpi);
    for (float y = S->pad.y + pitch; y < S->pad.y + S->pad.h - 4; y += pitch)
        canvas_hline(c, S->pad.x, S->pad.x + S->pad.w, (int)y, ui_px(2), 185);
    ui_frame(c, rect_inflate(S->pad, ui_px(4)), ui_px(14), 0, false, ui_px(4));
    // TRANSLATORS: the button that wipes Pen feel's test pad (174 px at 44 px, or 34 px).
    // TRANSLATORS: on the eraser tab, restore the handwritten practice ink (174 px at 34 px).
    ui_button(c, clear_rect(a), ICON_NONE, S->tab == TOOL_ERASER ? _("Reset pad") : _("Clear"), BTN_PLAIN);
}

// Capture the bare paper before seeding the eraser targets or white-ink swatch as ink.
// For white ink, a black swatch lies
// under the Clear button's row (white on the bare pad shows nothing: on the swatch it shows the
// tool's look, its grain and pressure, and the pad's lines it brings back).
static void draw_pad_frame(App *a) {
    pad_targets_free(a);
    draw_pad_bare(a);
    pad_paper_take(a);
    S->swatch = RECT_EMPTY;
    if (S->tab == TOOL_ERASER) {
        pad_targets_seed(a);
    } else if (pad_white(a) && S->paper.px) {
        S->swatch = (Rect){S->pad.x + ui_px(30), S->pad.y + ui_px(130), S->pad.w - ui_px(60), S->pad.h - ui_px(160)};
        canvas_fill(&a->screen, S->swatch, 0);
    }
}

static void draw_all(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(50), th = ui_px(190);
    int bh = ui_px(150), by = (th - bh) / 2;
    Rect back = {m, by, bh, bh};
    ui_button(c, back, ICON_BACK, NULL, BTN_PLAIN);
    hits_add(&S->hits, back, H_BACK, 0);
    Rect reset = {a->w - m - ui_px(360), by, ui_px(360), bh};
    // TRANSLATORS: puts the tool's pen feel back as it came (328 px at 44 px, or 34 px).
    ui_button(c, reset, ICON_NONE, _("Reset tool"), BTN_PLAIN);
    hits_add(&S->hits, reset, H_RESET, 0);
    char title[128];
    // TRANSLATORS: the title of the screen where each tool's pressure and tilt are tuned, with the
    // tool's name (1200 px at 70 px).
    snprintf(title, sizeof title, _("Pen feel: %s"), tool_label(S->tab));
    font_draw_left(c, FONT_BOLD, ui_title_size() * 1.1f, (Rect){back.x + bh + ui_px(30), 0, reset.x - back.x - bh - ui_px(50), th},
                   title, 0);
    ui_rule(c, 0, a->w, th, ui_px(4), 0);
    // Tabs.
    int tw = (a->w - 2 * m - (TOOL_COUNT - 1) * ui_px(16)) / TOOL_COUNT, ty = th + ui_px(24), tabh = ui_px(150);
    for (int i = 0; i < TOOL_COUNT; i++) {
        ToolKind t = tool_order[i];
        Rect r = {m + i * (tw + ui_px(16)), ty, tw, tabh};
        // With this many tools the tabs are icons only; the title names the selected one.
        ui_button(c, r, ui_tool_icon(t), NULL, S->tab == (ToolKind)t ? BTN_SELECTED : BTN_PLAIN);
        hits_add(&S->hits, r, H_TAB, t);
    }
    // Graph + pad.
    int top = ty + tabh + ui_px(70);
    int gs = ui_px(500);
    S->graph = (Rect){m + ui_px(10), top, gs, gs};
    S->pad = (Rect){S->graph.x + gs + ui_px(70), top, a->w - m - (S->graph.x + gs + ui_px(70)), gs};
    S->readout = (Rect){m, top + gs + ui_px(70), a->w - 2 * m, ui_px(70)};
    draw_graph(a);
    draw_pad_frame(a);
    hits_add(&S->hits, S->pad, H_PAD, 0);
    hits_add(&S->hits, clear_rect(a), H_CLEAR, 0);
    // TRANSLATORS: over Pen feel's test pad, which the pen draws on with the tool (1180 px at 34 px).
    const char *pad = _("test pad");
    // TRANSLATORS: the same while the pens draw white ink, which shows only over ink: the pad then
    // has a black part to try it on (1180 px at 34 px).
    if (pad_white(a)) pad = _("test pad (white ink: try it on the black)");
    font_draw(c, FONT_REGULAR, ui_small_size(), S->pad.x, S->pad.y - ui_px(14), pad, 60);
    draw_readout(a);
    // Sliders.
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    int y = S->readout.y + S->readout.h + ui_px(30);
    canvas_hline(c, m, a->w - m, y, ui_px(3), 0);
    y += ui_px(16);
    int avail = a->h - y - ui_px(20);
    int count = 0;
    for (int i = 0; i < n; i++) count += visible_param(&pi[i], S->tab);
    int rh = count ? avail / count : avail;
    if (rh > ui_px(118)) rh = ui_px(118);
    S->nsliders = 0;
    for (int i = 0; i < n && S->nsliders < 16; i++) {
        if (!visible_param(&pi[i], S->tab)) continue;
        S->slider_rows[S->nsliders] = (Rect){m, y, a->w - 2 * m, rh};
        S->slider_param[S->nsliders] = i;
        draw_slider_row(a, S->nsliders);
        hits_add(&S->hits, S->slider_rows[S->nsliders], H_SLIDER, S->nsliders);
        S->nsliders++;
        y += rh;
    }
    if (S->tab != TOOL_ERASER) return;
    // What the eraser does (settings.ini eraser_mode; its button in the editor, the pen's eraser
    // end, the pen's button set to erase): two choices right of the row's name, where a slider row
    // has its name and its track.
    Rect row = {m, y + ui_px(24), a->w - 2 * m, ui_px(120)};
    int name_w = row.w * 42 / 100, g = ui_px(20), cw = (row.w - name_w - g) / 2;
    // TRANSLATORS: Pen feel's eraser tab: the name of the row that picks how the eraser works, left
    // of its two choices, as a slider's name is left of it (720 px at 44 px, or two lines at 34 px).
    const char *name = _("Eraser:");
    ui_text(c, FONT_REGULAR, ui_text_size(), ui_small_size(), (Rect){row.x, row.y, name_w - ui_px(16), row.h}, name,
            2, false, 0);
    static const char *const modes[2] = {
        // TRANSLATORS: the first choice of Pen feel's Eraser row: the eraser takes every stroke it
        // touches away whole (as before 1.1). 468 px at 44 px, or two lines at 34 px.
        NC_("eraser", "Whole strokes"),
        // TRANSLATORS: the second choice of Pen feel's Eraser row: the eraser takes away only what
        // it touches, as white ink does, keeping the paper's lines (468 px at 44 px).
        NC_("eraser", "Precise"),
    };
    for (int i = 0; i < 2; i++) {
        Rect r = {row.x + name_w + i * (cw + g), row.y, cw, row.h};
        ui_button(c, r, ICON_NONE, C_("eraser", modes[i]), a->settings.eraser_mode == i ? BTN_SELECTED : BTN_PLAIN);
        hits_add(&S->hits, r, H_ERASER_MODE, i);
    }
    y = row.y + row.h;
    const char *note = _("The eraser removes whole strokes; harder pressure = bigger reach.");
    // TRANSLATORS: under Pen feel's Eraser row while the precise eraser is picked (1760 px at 44 px).
    if (a->settings.eraser_mode == 1) note = _("The eraser takes away only what it touches; harder pressure = bigger reach.");
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){m, y + ui_px(30), a->w - 2 * m, ui_px(80)}, note, 60);
}

void pensettings_open(App *a, ToolKind tab) {
    S->tab = tab;
    S->ret = a->scr;
    app_switch(a, SCR_PENSET);
}

void pensettings_init(App *a) {
    a->ps = calloc(1, sizeof *a->ps);
    S->active = -1;
    S->live_raw = -1.f;
}
void pensettings_free(App *a) {
    if (a->ps) { pad_targets_free(a); canvas_free(&a->ps->paper); }
    free(a->ps);
    a->ps = NULL;
}
void pensettings_enter(App *a) {
    S->active = -1;
    S->slider_dirty = false;
    S->drawing = false;
    S->erasing = false;
    S->whiting = false;
    S->wiping = false;
    S->has_pressed = false;
    S->live_raw = -1.f;
    draw_all(a);
}
void pensettings_leave(App *a) {
    if (S->drawing) display_live_ink(false);  // left in the middle of a stroke on the pad
    S->drawing = false;
    S->erasing = false;
    S->whiting = false;
    S->wiping = false;
    canvas_free(&S->paper);
    pad_targets_free(a);
    S->swatch = RECT_EMPTY;
    settings_save(&a->settings);
}

static void repaint_sliders(App *a, RefreshMode m) {
    for (int i = 0; i < S->nsliders; i++) draw_slider_row(a, i);
    draw_graph(a);
    draw_readout(a);
    Rect dirty = rect_union(rect_union(S->slider_rows[0], S->slider_rows[S->nsliders - 1]),
                            rect_inflate(S->graph, ui_px(70)));
    app_invalidate(a, rect_union(dirty, S->readout), m);
    S->slider_dirty = false;
}

static void set_slider(App *a, int row, const InEvent *ev, bool first, RefreshMode m) {
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    const ParamInfo *p = &pi[S->slider_param[row]];
    Slider sl = {.min = p->min, .max = p->max, .step = p->step};
    sl.value = *tool_param_ptr(&a->settings.tools[S->tab], p);
    ui_slider_layout(&sl, S->slider_rows[row]);
    bool changed;
    if (first) {
        changed = ui_slider_set_from_x(&sl, ev->x);
        ui_slider_drag_begin(&sl, &S->slider_drag, ev->x, ev->t_ms);
    } else changed = ui_slider_drag_to(&sl, &S->slider_drag, ev->x, ev->y, ev->t_ms);
    S->slider_dirty |= changed;
    if (!changed && !S->slider_dirty && m == REFRESH_FAST) return;
    float *v = tool_param_ptr(&a->settings.tools[S->tab], p);
    *v = sl.value;
    // Keep min <= max pairs sensible.
    ToolParams *tp = &a->settings.tools[S->tab];
    if (tp->size_min_mm > tp->size_max_mm) {
        if (v == &tp->size_min_mm) tp->size_max_mm = tp->size_min_mm;
        else tp->size_min_mm = tp->size_max_mm;
    }
    if (tp->pressure_floor > tp->pressure_ceiling - 0.05f) {
        if (v == &tp->pressure_floor) tp->pressure_ceiling = fminf(1.f, tp->pressure_floor + 0.05f);
        else tp->pressure_floor = fmaxf(0.f, tp->pressure_ceiling - 0.05f);
    }
    if (tp->tilt_min_deg > tp->tilt_max_deg - 1.f) {
        if (v == &tp->tilt_min_deg) tp->tilt_max_deg = fminf(90.f, tp->tilt_min_deg + 1.f);
        else tp->tilt_min_deg = fmaxf(0.f, tp->tilt_max_deg - 1.f);
    }
    // Process every motion for fractional accuracy, but keep the existing display throttle.
    if (!first && m == REFRESH_FAST && ev->t_ms - S->last_paint < 80u) return;
    S->last_paint = ev->t_ms;
    // Paired sliders may have moved too: repaint every row (cheap) + graph + readout.
    repaint_sliders(a, m);
}

static void pad_erase_targets(App *a, const PenSample *s, bool first) {
    float x0 = first ? s->x : S->wipe_x, y0 = first ? s->y : S->wipe_y, r, opacity;
    brush_dab(TOOL_ERASER, &a->settings.tools[TOOL_ERASER], s, a->dpi, &r, &opacity);
    bool changed = false;
    if (S->sample) for (int i = S->sample->n - 1; i >= 0; i--)
        if (stroke_hit_segment(S->sample->strokes[i], x0, y0, s->x, s->y, r, a->dpi)) {
            stroke_free(page_remove(S->sample, i));
            changed = true;
        }
    S->wipe_x = s->x; S->wipe_y = s->y;
    if (changed) {
        draw_pad_bare(a);
        pad_targets_draw(a);
        app_invalidate(a, S->pad, REFRESH_FAST);
    }
}

static void pad_sample(App *a, const InEvent *ev, bool first) {
    S->live_raw = ev->pressure;
    S->live_tx = ev->tilt_x;
    S->live_ty = ev->tilt_y;
    PenSample s = {ev->x, ev->y, ev->pressure, ev->tilt_x, ev->tilt_y, ev->t_ms};
    if (S->wiping) {
        if (S->tab == TOOL_ERASER) pad_erase_targets(a, &s, first);
    } else {
        if (first && (S->erasing || S->whiting)) {
            // The precise eraser's round tip with its own settings (the pad is at 100%), or white ink
            // with the tool's texture and settings, restoring the pad's paper as the editor's erase
            // strokes restore a page's (their brush hashes the seed's low 24 bits).
            ToolKind t = S->erasing ? TOOL_ERASER : S->tab;
            brush_begin(&S->bs, t, &a->settings.tools[t], ev->t_ms & 0xFFFFFFu, a->dpi);
            brush_set_erase(&S->bs, S->paper.px, S->pad.x, S->pad.y, S->paper.stride);
            smoother_reset(&S->sm);
        } else if (first) {
            ToolParams *p = &a->settings.tools[S->tab];
            brush_begin(&S->bs, S->tab, p, ev->t_ms, a->dpi);
            smoother_reset(&S->sm);
        }
        smoother_apply(&S->sm, S->bs.params.smoothing, &s);
        Rect saved = a->screen.clip;
        canvas_set_clip(&a->screen, S->pad);
        Rect d = brush_add(&S->bs, &a->screen, &s);
        a->screen.clip = saved;
        app_invalidate(a, rect_intersect(d, S->pad), REFRESH_FAST);
    }
    if (ev->t_ms - S->last_readout > 150) {
        S->last_readout = ev->t_ms;
        draw_readout(a);
        draw_graph(a);
        app_invalidate(a, rect_union(S->readout, rect_inflate(S->graph, ui_px(70))), REFRESH_FAST);
    }
}

void pensettings_event(App *a, const InEvent *ev) {
    bool pen = ev->type == IN_PEN_DOWN || ev->type == IN_PEN_MOVE || ev->type == IN_PEN_UP;
    int ph = app_pointer_phase(ev);
    if (ph < 0) return;
    if (ph == 0) {
        if (S->has_pressed || S->drawing || S->active >= 0) return;
        press_begin(&S->press, ev->x, ev->y, ev->t_ms);
        const Hit *h = hits_find(&S->hits, ev->x, ev->y);
        if (!h) return;
        S->pen_owner = pen;
        if (h->id == H_PAD) {
            if (!pen) return;  // fingers do not draw
            // The eraser (its tab, or the pen's eraser end): the precise one erases what it
            // touches; the stroke eraser hits the practice strokes on its tab.
            bool eraser = ev->eraser || S->tab == TOOL_ERASER;
            S->erasing = eraser && a->settings.eraser_mode == 1 && S->paper.px;
            S->whiting = !eraser && pad_white(a) && S->paper.px;
            S->wiping = eraser && !S->erasing;
            if (S->wiping && S->tab != TOOL_ERASER) {
                draw_pad_frame(a);
                app_invalidate(a, rect_inflate(S->pad, ui_px(6)), REFRESH_UI);
            }
            S->drawing = true;
            display_live_ink(true);  // the pad's ink is live, as in the editor
            pad_sample(a, ev, true);
            return;
        }
        if (h->id == H_SLIDER) {
            S->active = h->arg;
            S->last_paint = ev->t_ms;
            set_slider(a, h->arg, ev, true, REFRESH_FAST);
            return;
        }
        S->pressed = *h;
        S->has_pressed = true;
        app_press_feedback(a, h->r);
        return;
    }
    if (pen != S->pen_owner) return;
    if (S->drawing) {
        if (ph == 1) pad_sample(a, ev, false);
        else {
            S->drawing = false;
            S->erasing = false;
            S->whiting = false;
            S->wiping = false;
            display_live_ink(false);
            draw_readout(a);
            draw_graph(a);
            app_invalidate(a, rect_union(rect_union(S->readout, rect_inflate(S->graph, ui_px(70))), S->pad), REFRESH_UI);
        }
        return;
    }
    if (S->active >= 0) {
        if (ph == 1) {
            set_slider(a, S->active, ev, false, REFRESH_FAST);
        } else if (ph == 2) {
            set_slider(a, S->active, ev, false, REFRESH_UI);
            S->active = -1;
            settings_save(&a->settings);
        }
        return;
    }
    if (!S->has_pressed) return;
    if (ph == 1) {
        press_move(&S->press, ev->x, ev->y);
        return;
    }
    S->has_pressed = false;
    Hit h = S->pressed;
    canvas_invert(&a->screen, h.r);
    app_invalidate(a, h.r, REFRESH_UI);
    press_move(&S->press, ev->x, ev->y);
    if (!rect_contains(h.r, (int)ev->x, (int)ev->y)) return;
    switch (h.id) {
    case H_BACK: app_switch(a, S->ret == SCR_PENSET ? SCR_LIBRARY : S->ret); break;
    case H_TAB:
        S->tab = (ToolKind)h.arg;
        S->live_raw = -1.f;
        draw_all(a);
        app_invalidate_all(a, REFRESH_UI);
        break;
    case H_RESET:
        tool_params_default(S->tab, &a->settings.tools[S->tab]);
        if (S->tab == TOOL_ERASER) a->settings.eraser_mode = 0;
        settings_save(&a->settings);
        draw_all(a);
        app_invalidate_all(a, REFRESH_UI);
        app_toast(a, _("Reset to defaults"), 1500);
        break;
    case H_CLEAR:
        draw_pad_frame(a);
        app_invalidate(a, rect_inflate(S->pad, ui_px(6)), REFRESH_UI);
        break;
    case H_ERASER_MODE:
        // Restore the test targets when changing what the eraser does to them.
        a->settings.eraser_mode = h.arg;
        settings_save(&a->settings);
        draw_all(a);
        app_invalidate_all(a, REFRESH_UI);
        break;
    default: break;
    }
}

void pensettings_tick(App *a, uint32_t now) {
    if (S->active >= 0 && S->slider_dirty && now - S->last_paint >= 80u) {
        S->last_paint = now;
        repaint_sliders(a, REFRESH_FAST);
    }
}

// Tests (app_probe's "penset." keys), while the pad keeps its copy of itself drawn bare (the precise
// eraser on, or white ink): "pad_ink" 1 while ink shows on the pad (a pixel darker than the bare
// pad there; white ink's swatch counts), "pad_lighter" 1 when a pixel is lighter than the bare pad
// (one of its lines, or its Clear button, erased away), "pad_ink_near.X.Y" 1 while ink shows
// within 12 px of the screen point X, Y. Empty without the copy. And "pad_white": 1 once white ink
// (or the precise eraser) went through the swatch (a pixel of it lighter than black), empty
// without a swatch.
int pensettings_probe(App *a, const char *key, char *out, size_t cap) {
    if (S && !strcmp(key, "slider_pending")) {
        snprintf(out, cap, "%d", S->slider_dirty);
        return 0;
    }
    if (S && !strncmp(key, "param.", 6)) {
        int n; const ParamInfo *pi = tool_param_table(&n);
        for (int i = 0; i < n; i++) if (!strcmp(key + 6, pi[i].key)) {
            snprintf(out, cap, "%.4f", *tool_param_ptr(&a->settings.tools[S->tab], &pi[i]));
            return 0;
        }
    }
    if (S && !strcmp(key, "pad_strokes")) {
        snprintf(out, cap, "%d", S->sample ? S->sample->n : 0);
        return 0;
    }
    if (S && !strcmp(key, "pad_sample_rect")) {
        snprintf(out, cap, "%d %d %d %d", S->sample_box.x, S->sample_box.y, S->sample_box.w, S->sample_box.h);
        return 0;
    }
    if (S && !strcmp(key, "pad_white")) {
        bool lit = false;
        for (int y = S->swatch.y; y < S->swatch.y + S->swatch.h && !lit; y++)
            for (int x = S->swatch.x; x < S->swatch.x + S->swatch.w && !lit; x++)
                lit = a->screen.px[(size_t)y * (size_t)a->screen.stride + (size_t)x] > 0;
        if (rect_is_empty(S->swatch)) snprintf(out, cap, "%s", "");
        else snprintf(out, cap, "%d", lit ? 1 : 0);
        return 0;
    }
    int px = 0, py = 0;
    bool near = !strncmp(key, "pad_ink_near.", 13) && sscanf(key + 13, "%d.%d", &px, &py) == 2;
    bool lighter = !strcmp(key, "pad_lighter");
    if (!S || (!near && !lighter && strcmp(key, "pad_ink"))) return -1;
    if (!S->paper.px || S->paper.w != S->pad.w || S->paper.h != S->pad.h) {
        snprintf(out, cap, "%s", "");
        return 0;
    }
    Rect r = S->pad;
    if (near) r = rect_intersect(r, (Rect){px - 12, py - 12, 25, 25});
    bool found = false;
    for (int y = r.y; y < r.y + r.h && !found; y++) {
        const uint8_t *s = &a->screen.px[(size_t)y * (size_t)a->screen.stride + (size_t)r.x];
        const uint8_t *b = &S->paper.px[(size_t)(y - S->pad.y) * (size_t)S->paper.stride + (size_t)(r.x - S->pad.x)];
        for (int x = 0; x < r.w && !found; x++) found = lighter ? s[x] > b[x] : s[x] < b[x];
    }
    snprintf(out, cap, "%d", found ? 1 : 0);
    return 0;
}
