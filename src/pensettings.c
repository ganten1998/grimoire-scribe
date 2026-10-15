// pensettings.c: per-tool pressure / tilt sensitivity. Tabs per tool, one slider per
// tweakable parameter, a live pressure curve, a raw-vs-mapped readout, and a test pad that
// draws with the tool being tuned.
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"

enum { H_BACK = 1, H_TAB, H_RESET, H_CLEAR, H_SLIDER, H_PAD };

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
    uint32_t last_paint;
    // test pad
    Rect pad, graph, readout;
    bool drawing;
    bool pen_owner;
    BrushState bs;
    Smoother sm;
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
    if (!strcmp(pi->unit, "deg")) snprintf(buf, cap, "%d deg", (int)lrintf(v));
    else if (!strcmp(pi->unit, "mm")) snprintf(buf, cap, "%.2f mm", v);
    else if (!strcmp(pi->unit, "x")) snprintf(buf, cap, "+%.0f%%", v * 100.f);
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
    font_draw(c, FONT_REGULAR, ss, g.x, g.y + g.h + ui_px(44), "pen pressure", 60);
    font_draw(c, FONT_REGULAR, ss, g.x, g.y - ui_px(14), "ink", 60);
}

static void draw_readout(App *a) {
    Canvas *c = &a->screen;
    canvas_fill(c, S->readout, 255);
    const ToolParams *p = &a->settings.tools[S->tab];
    char line[200];
    if (S->live_raw < 0.f) {
        snprintf(line, sizeof line, "Draw on the pad to see live pressure and tilt.");
    } else {
        PenSample s = {0, 0, S->live_raw, S->live_tx, S->live_ty, 0};
        float r, op;
        brush_dab(S->tab, p, &s, a->dpi, &r, &op);
        float tilt_deg = atanf(sqrtf(powf(tanf(S->live_tx * (float)M_PI / 180.f), 2) +
                                     powf(tanf(S->live_ty * (float)M_PI / 180.f), 2))) * 180.f / (float)M_PI;
        const InputCaps *caps = input_caps();
        char tilt[64];
        if (caps && !caps->has_tilt) snprintf(tilt, sizeof tilt, "tilt: not reported");
        else snprintf(tilt, sizeof tilt, "tilt %d deg -> %.2f", (int)lrintf(tilt_deg), brush_tilt(p, S->live_tx, S->live_ty));
        snprintf(line, sizeof line, "pressure %.2f -> %.2f  %s  width %.2f mm  ink %.2f", S->live_raw,
                 brush_pressure(p, S->live_raw), tilt, r * 2.f * 25.4f / a->dpi, op);
    }
    font_draw_left(c, FONT_REGULAR, ui_small_size(), S->readout, line, 0);
}

static void draw_slider_row(App *a, int i) {
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    const ParamInfo *p = &pi[S->slider_param[i]];
    Slider sl = {.min = p->min, .max = p->max, .step = p->step, .label = p->label, .unit = p->unit};
    sl.value = *tool_param_ptr(&a->settings.tools[S->tab], p);
    ui_slider_layout(&sl, S->slider_rows[i]);
    char vt[32];
    value_text(p, sl.value, vt, sizeof vt);
    ui_slider_draw(&a->screen, &sl, vt);
}

static Rect clear_rect(App *a) {
    return (Rect){S->pad.x + S->pad.w - ui_px(220), S->pad.y + ui_px(14), ui_px(206), ui_px(100)};
}

static void draw_pad_frame(App *a) {
    Canvas *c = &a->screen;
    canvas_fill(c, S->pad, 255);
    // A few ruled lines so stroke widths read like they will on paper.
    float pitch = mm_to_px(7.14f, a->dpi);
    for (float y = S->pad.y + pitch; y < S->pad.y + S->pad.h - 4; y += pitch)
        canvas_hline(c, S->pad.x, S->pad.x + S->pad.w, (int)y, ui_px(2), 185);
    canvas_round_rect(c, rect_inflate(S->pad, ui_px(4)), ui_px(14), 0, false, ui_px(4));
    ui_button(c, clear_rect(a), ICON_NONE, "Clear", BTN_PLAIN);
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
    ui_button(c, reset, ICON_NONE, "Reset tool", BTN_PLAIN);
    hits_add(&S->hits, reset, H_RESET, 0);
    char title[64];
    snprintf(title, sizeof title, "Pen feel: %s", tool_name(S->tab));
    font_draw_left(c, FONT_BOLD, ui_title_size() * 1.1f, (Rect){back.x + bh + ui_px(30), 0, reset.x - back.x - bh - ui_px(50), th},
                   title, 0);
    canvas_hline(c, 0, a->w, th, ui_px(4), 0);
    // Tabs.
    static const ToolKind order[TOOL_COUNT] = {TOOL_PENCIL,    TOOL_MECH_PENCIL, TOOL_BALLPOINT,   TOOL_FOUNTAIN,
                                               TOOL_FINELINER, TOOL_MARKER,      TOOL_HIGHLIGHTER, TOOL_ERASER};
    int tw = (a->w - 2 * m - (TOOL_COUNT - 1) * ui_px(16)) / TOOL_COUNT, ty = th + ui_px(24), tabh = ui_px(150);
    for (int i = 0; i < TOOL_COUNT; i++) {
        ToolKind t = order[i];
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
    font_draw(c, FONT_REGULAR, ui_small_size(), S->pad.x, S->pad.y - ui_px(14),
              S->tab == TOOL_ERASER ? "test pad (the eraser clears it)" : "test pad", 60);
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
    if (S->tab == TOOL_ERASER)
        font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){m, y + ui_px(30), a->w - 2 * m, ui_px(80)},
                       "The eraser removes whole strokes; harder pressure = bigger reach.", 60);
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
    free(a->ps);
    a->ps = NULL;
}
void pensettings_enter(App *a) {
    S->active = -1;
    S->drawing = false;
    S->has_pressed = false;
    S->live_raw = -1.f;
    draw_all(a);
}
void pensettings_leave(App *a) { settings_save(&a->settings); }

static void set_slider(App *a, int row, float x, RefreshMode m) {
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    const ParamInfo *p = &pi[S->slider_param[row]];
    Slider sl = {.min = p->min, .max = p->max, .step = p->step};
    sl.value = *tool_param_ptr(&a->settings.tools[S->tab], p);
    ui_slider_layout(&sl, S->slider_rows[row]);
    if (!ui_slider_set_from_x(&sl, x) && m == REFRESH_FAST) return;
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
    // Paired sliders may have moved too: repaint every row (cheap) + graph + readout.
    for (int i = 0; i < S->nsliders; i++) draw_slider_row(a, i);
    draw_graph(a);
    draw_readout(a);
    Rect dirty = rect_union(rect_union(S->slider_rows[0], S->slider_rows[S->nsliders - 1]),
                            rect_inflate(S->graph, ui_px(70)));
    dirty = rect_union(dirty, S->readout);
    app_invalidate(a, dirty, m);
}

static void pad_sample(App *a, const InEvent *ev, bool first) {
    S->live_raw = ev->pressure;
    S->live_tx = ev->tilt_x;
    S->live_ty = ev->tilt_y;
    if (S->tab == TOOL_ERASER) return;
    PenSample s = {ev->x, ev->y, ev->pressure, ev->tilt_x, ev->tilt_y, ev->t_ms};
    if (first) {
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
            if (ev->eraser || S->tab == TOOL_ERASER) {
                draw_pad_frame(a);
                app_invalidate(a, rect_inflate(S->pad, ui_px(6)), REFRESH_UI);
            }
            S->drawing = true;
            pad_sample(a, ev, true);
            return;
        }
        if (h->id == H_SLIDER) {
            S->active = h->arg;
            S->last_paint = ev->t_ms;
            set_slider(a, h->arg, ev->x, REFRESH_FAST);
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
            draw_readout(a);
            draw_graph(a);
            app_invalidate(a, rect_union(rect_union(S->readout, rect_inflate(S->graph, ui_px(70))), S->pad), REFRESH_UI);
        }
        return;
    }
    if (S->active >= 0) {
        if (ph == 1 && ev->t_ms - S->last_paint > 80) {
            S->last_paint = ev->t_ms;
            set_slider(a, S->active, ev->x, REFRESH_FAST);
        } else if (ph == 2) {
            set_slider(a, S->active, ev->x, REFRESH_UI);
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
        settings_save(&a->settings);
        draw_all(a);
        app_invalidate_all(a, REFRESH_UI);
        app_toast(a, "Reset to defaults", 1500);
        break;
    case H_CLEAR:
        draw_pad_frame(a);
        app_invalidate(a, rect_inflate(S->pad, ui_px(6)), REFRESH_UI);
        break;
    default: break;
    }
}

void pensettings_tick(App *a, uint32_t now) {}
