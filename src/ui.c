// ui.c: widgets, in two themes. Classic: Terminess, rounded corners. Gothic: book and
// blackletter type, cut-stone corners, double rules, lozenges and pointed arches. Icons are
// Terminess Nerd Font in both.
#include "ui.h"

#include "brush.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static float g_scale = 1.f;
static UiTheme g_theme = THEME_GOTHIC;

void ui_set_theme(UiTheme t) {
    g_theme = t == THEME_CLASSIC ? THEME_CLASSIC : THEME_GOTHIC;
    font_set_gothic(g_theme == THEME_GOTHIC);
}
UiTheme ui_theme(void) { return g_theme; }
static bool gothic(void) { return g_theme == THEME_GOTHIC; }

void ui_frame(Canvas *c, Rect r, int radius, uint8_t gray, bool filled, int t) {
    if (gothic())
        canvas_chamfer_rect(c, r, radius + radius / 4, gray, filled, t);
    else
        canvas_round_rect(c, r, radius, gray, filled, t);
}

// A pointed (equilateral) arch: springers at (x0, ys) and (x1, ys), stroked with width w.
static void arch(Canvas *c, float x0, float x1, float ys, float w, uint8_t gray) {
    float span = x1 - x0;
    enum { SEG = 8 };
    for (int side = 0; side < 2; side++) {
        // Left half: arc centred on the right springer; right half mirrored.
        float ccx = side == 0 ? x1 : x0;
        float a0 = side == 0 ? 3.14159265f : 0.f, a1 = side == 0 ? 2.0943951f : 1.0471976f;
        float px = ccx + span * cosf(a0), py = ys - span * sinf(a0);
        for (int i = 1; i <= SEG; i++) {
            float th = a0 + (a1 - a0) * (float)i / SEG;
            float qx = ccx + span * cosf(th), qy = ys - span * sinf(th);
            canvas_line_aa(c, px, py, qx, qy, w, gray);
            px = qx;
            py = qy;
        }
    }
}

void ui_frieze(Canvas *c, int x0, int x1, int y, int h, uint8_t gray) {
    if (!gothic() || x1 - x0 < h) return;
    // A cornice of pointed arches hanging from the rule at y, a lozenge drop at every springer.
    float span = (float)h / 0.866f;
    int n = (int)((float)(x1 - x0) / span);
    if (n < 1) return;
    span = (float)(x1 - x0) / (float)n;
    float ys = (float)y + span * 0.866f, w = (float)ui_px(3);
    for (int i = 0; i < n; i++) arch(c, x0 + span * i, x0 + span * (i + 1), ys, w, gray);
    for (int i = 0; i <= n; i++) {
        float x = x0 + span * i;
        canvas_line_aa(c, x, (float)y, x, ys, w, gray);
        canvas_lozenge(c, x, ys + (float)ui_px(7), (float)ui_px(7), gray);
    }
}

void ui_rule(Canvas *c, int x0, int x1, int y, int t, uint8_t gray) {
    if (!gothic()) {
        canvas_hline(c, x0, x1, y, t, gray);
        return;
    }
    // Thick over thin, the way a printer's rule frames a title page.
    canvas_hline(c, x0, x1, y, t, gray);
    canvas_hline(c, x0, x1, y + t + ui_px(5), ui_px(2), gray);
}

// An ornamental divider: thin rule broken by a lozenge with two small ones beside it.
static void ornament(Canvas *c, int x0, int x1, int y, uint8_t gray) {
    int mid = (x0 + x1) / 2, gap = ui_px(60);
    canvas_hline(c, x0, mid - gap, y - ui_px(1), ui_px(2), gray);
    canvas_hline(c, mid + gap, x1, y - ui_px(1), ui_px(2), gray);
    canvas_lozenge(c, (float)mid, (float)y, (float)ui_px(16), gray);
    canvas_lozenge(c, (float)(mid - ui_px(36)), (float)y, (float)ui_px(8), gray);
    canvas_lozenge(c, (float)(mid + ui_px(36)), (float)y, (float)ui_px(8), gray);
}

void ui_emblem(Canvas *c, float cx, float cy, float r, uint8_t ink, uint8_t bg) {
    // Quatrefoil: four overlapping lobes drawn as one outline, a lozenge at the heart.
    float lr = r * 0.5f, t = fmaxf(2.f, r * 0.09f);
    const float off[4][2] = {{-lr, 0}, {lr, 0}, {0, -lr}, {0, lr}};
    for (int i = 0; i < 4; i++) canvas_disc_aa(c, cx + off[i][0], cy + off[i][1], lr, ink, 1.f);
    for (int i = 0; i < 4; i++) canvas_disc_aa(c, cx + off[i][0], cy + off[i][1], lr - t, bg, 1.f);
    canvas_lozenge(c, cx, cy, r * 0.32f, ink);
    canvas_lozenge(c, cx, cy, r * 0.32f - t * 1.4f, bg);
    canvas_lozenge(c, cx, cy, r * 0.12f, ink);
}

void ui_init(float dpi) { g_scale = dpi > 50.f ? dpi / 300.f : 1.f; }
int ui_px(float v) { return (int)lrintf(v * g_scale); }
float ui_text_size(void) { return 44.f * g_scale; }
float ui_title_size(void) { return 64.f * g_scale; }
float ui_small_size(void) { return 34.f * g_scale; }

// Material Design glyphs inside Terminess Nerd Font (codes from nerd-fonts glyphnames.json).
static const uint32_t k_icon_cp[ICON_COUNT] = {
    [ICON_PENCIL] = 0xF064F,       // md-lead_pencil
    [ICON_MECH_PENCIL] = 0xF0CB6,  // md-pencil_outline
    [ICON_BALLPOINT] = 0xF03EA,    // md-pen
    [ICON_ERASER] = 0xF01FE,       // md-eraser
    [ICON_UNDO] = 0xF054C,         // md-undo
    [ICON_REDO] = 0xF044E,         // md-redo
    [ICON_PREV] = 0xF0141,         // md-chevron_left
    [ICON_NEXT] = 0xF0142,         // md-chevron_right
    [ICON_PLUS] = 0xF0415,         // md-plus
    [ICON_MINUS] = 0xF0374,        // md-minus
    [ICON_CLOSE] = 0xF0156,        // md-close
    [ICON_PAPER] = 0xF09EE,        // md-file_document_outline
    [ICON_GEAR] = 0xF0493,         // md-cog
    [ICON_LIBRARY] = 0xF125F,      // md-bookshelf
    [ICON_COLLAPSE] = 0xF0615,     // md-arrow_collapse
    [ICON_EXPAND] = 0xF0616,       // md-arrow_expand
    [ICON_GRIP] = 0xF01DB,         // md-drag
    [ICON_FOLDER] = 0xF024B,       // md-folder
    [ICON_NOTEBOOK] = 0xF082E,     // md-notebook
    [ICON_MORE] = 0xF01D8,         // md-dots_horizontal
    [ICON_SHARE] = 0xF05A9,        // md-wifi
    [ICON_TRASH] = 0xF0A7A,        // md-trash_can_outline
    [ICON_CHECK] = 0xF012C,        // md-check
    [ICON_BACK] = 0xF004D,         // md-arrow_left
    [ICON_TARGET] = 0xF01A3,       // md-crosshairs
    [ICON_EXIT] = 0xF0206,         // md-exit_to_app
    [ICON_TUNE] = 0xF062E,         // md-tune
    [ICON_RENAME] = 0xF0455,       // md-rename_box
    [ICON_FOLDER_MOVE] = 0xF0252,  // md-folder_move
    [ICON_FOLDER_PLUS] = 0xF0257,  // md-folder_plus
    [ICON_DOWNLOAD] = 0xF01DA,     // md-download
    [ICON_HIGHLIGHTER] = 0xF0652,  // md-marker
    [ICON_BROOM] = 0xF00E2,        // md-broom
    [ICON_BRIGHTNESS] = 0xF00DF,   // md-brightness_6
    [ICON_INVERT] = 0xF0301,       // md-invert_colors
    [ICON_FOUNTAIN] = 0xF0D13,     // md-fountain_pen_tip
    [ICON_FINELINER] = 0xF0F49,    // md-draw
    [ICON_MARKER] = 0xF0648,       // md-grease_pencil
    [ICON_UP] = 0xF0143,           // md-chevron_up
    [ICON_DOWN] = 0xF0140,         // md-chevron_down
    [ICON_MUSIC] = 0xF075A,        // md-music
    [ICON_CASTLE] = 0xF011A,       // md-castle
    [ICON_CHARCOAL] = 0xF00B5,     // md-blur (grainy dots)
    [ICON_CALLIGRAPHY] = 0xF06D3,  // md-feather (a quill)
    [ICON_LANGUAGE] = 0xF05CA,     // md-translate
    [ICON_PIN] = 0xF0403,          // md-pin
    [ICON_GRID] = 0xF0570,         // md-view_grid
    [ICON_COPY] = 0xF018F,         // md-content_copy
    [ICON_LINK] = 0xF0339,         // md-link_variant
    [ICON_LIVE] = 0xF0119,         // md-cast_connected
    [ICON_GAMEPAD] = 0xF0297,      // md-gamepad_variant
    [ICON_UPDATE] = 0xF06B0,       // md-update
};

IconKind ui_tool_icon(int tool) {
    static const IconKind k[] = {ICON_PENCIL,      ICON_MECH_PENCIL, ICON_BALLPOINT, ICON_ERASER,   ICON_HIGHLIGHTER,
                                 ICON_FOUNTAIN,    ICON_FINELINER,   ICON_MARKER,    ICON_CHARCOAL, ICON_CALLIGRAPHY};
    _Static_assert(sizeof k / sizeof k[0] == TOOL_COUNT, "an icon for every tool");
    return tool >= 0 && tool < (int)(sizeof k / sizeof k[0]) ? k[tool] : ICON_PENCIL;
}

void ui_icon(Canvas *c, IconKind k, Rect r, uint8_t gray) {
    if (k <= ICON_NONE || k >= ICON_COUNT || !k_icon_cp[k]) return;
    int inset = (r.w < r.h ? r.w : r.h) / 10;
    font_draw_icon(c, FONT_REGULAR, k_icon_cp[k], rect_inflate(r, -inset), gray);
}

void ui_icon_hollow(Canvas *c, IconKind k, Rect r, uint8_t edge, uint8_t inside) {
    // The glyph drawn in edge all round within t px (a disc of offsets), then in inside where it is.
    int t = ui_px(4);
    for (int dy = -t; dy <= t; dy++)
        for (int dx = -t; dx <= t; dx++)
            if ((dx || dy) && dx * dx + dy * dy <= t * t) ui_icon(c, k, (Rect){r.x + dx, r.y + dy, r.w, r.h}, edge);
    ui_icon(c, k, r, inside);
}

static int radius(void) { return ui_px(16); }

// In Gothic a button big enough for one has the inner line of a carved panel, 2 px wide, this far
// inside its edge.
static bool carved(int w, int h) { return gothic() && h >= ui_px(90) && w >= ui_px(90); }
static int carve_in(ButtonStyle st) { return st == BTN_PRIMARY ? ui_px(13) : ui_px(10); }
// How far a label stays from each side of its button: inside the edge, and inside the carved line
// where there is one, clear of it by 4 px (a word up to the edge's inset would run into it).
static int label_inset(bool carve, ButtonStyle st) {
    return carve ? carve_in(st) + ui_px(2) + ui_px(4) : ui_px(8);
}
// A label's size under an icon, and a label's alone (before a long one shrinks), for a button
// whose inside (its rect less 8 px all round) is in_h tall.
static float icon_label_px(int in_h) { return fminf(ui_small_size(), (float)in_h * 0.28f); }
static float text_label_px(int in_h) { return fminf(ui_text_size(), (float)in_h * 0.6f); }

// ui_button, with a label alone drawn at text_px when that is not 0 (one line at that size, else
// two), not at the size of its own that fits.
static void button(Canvas *c, Rect r, IconKind icon, const char *label, ButtonStyle st, float text_px) {
    uint8_t bg = st == BTN_SELECTED ? 0 : 255;
    uint8_t fg = st == BTN_SELECTED ? 255 : (st == BTN_DISABLED ? 170 : 0);
    uint8_t edge = st == BTN_DISABLED ? 170 : 0;
    ui_frame(c, r, radius(), bg, true, 0);
    ui_frame(c, r, radius(), edge, false, st == BTN_PRIMARY ? ui_px(6) : ui_px(3));
    bool carve = carved(r.w, r.h);
    if (carve) {
        // The inner line of a carved panel.
        int in = carve_in(st);
        canvas_chamfer_rect(c, rect_inflate(r, -in), radius() - in / 3, st == BTN_SELECTED ? 255 : edge, false,
                            ui_px(2));
    }
    bool has_label = label && *label;
    Rect in = rect_inflate(r, -ui_px(8));
    int li = label_inset(carve, st), lx = r.x + li, lw = r.w - 2 * li;  // the label's columns
    if (icon && has_label) {
        float ts = icon_label_px(in.h);
        int lh = (int)(ts * 1.15f);
        Rect ir = {in.x, in.y, in.w, in.h - lh};
        ui_icon(c, icon, ir, fg);
        Rect lr = {lx, in.y + in.h - lh, lw, lh};
        if (gothic()) lr = (Rect){lx, in.y + in.h - lh - ui_px(6), lw, lh};
        font_draw_centered(c, FONT_CAPS, ts, lr, label, fg);
    } else if (icon) {
        ui_icon(c, icon, rect_inflate(in, -ui_px(6)), fg);
    } else if (has_label) {
        // A label too long for the button (a translation) takes the small size, then two lines.
        float ts = text_px > 0.f ? text_px : text_label_px(in.h);
        ui_text(c, FONT_CAPS, ts, text_px > 0.f ? ts : fminf(ts, ui_small_size()), (Rect){lx, in.y, lw, in.h}, label, 2,
                true, fg);
    }
}

void ui_button(Canvas *c, Rect r, IconKind icon, const char *label, ButtonStyle st) {
    button(c, r, icon, label, st, 0.f);
}

void ui_button_hollow(Canvas *c, Rect r, IconKind icon, ButtonStyle st) {
    button(c, r, ICON_NONE, NULL, st, 0.f);
    uint8_t bg = st == BTN_SELECTED ? 0 : 255, fg = st == BTN_SELECTED ? 255 : (st == BTN_DISABLED ? 170 : 0);
    ui_icon_hollow(c, icon, rect_inflate(rect_inflate(r, -ui_px(8)), -ui_px(6)), fg, bg);  // where button() puts it
}

float ui_button_text_px(Rect r, const char *label, ButtonStyle st) {
    if (!label || !*label) return 0.f;
    float ts = text_label_px(r.h - 2 * ui_px(8)), min = fminf(ts, ui_small_size());
    float s = font_fit_px(FONT_CAPS, ts, min, label, r.w - 2 * label_inset(carved(r.w, r.h), st));
    return s > 0.f ? s : min;
}

void ui_button_text_at(Canvas *c, Rect r, const char *label, ButtonStyle st, float px) {
    button(c, r, ICON_NONE, label, st, px);
}

int ui_button_need_w(int h, IconKind icon, const char *label, ButtonStyle st) {
    if (!label || !*label) return 0;
    int in_h = h - 2 * ui_px(8);
    float ts = icon ? icon_label_px(in_h) : text_label_px(in_h);
    return font_measure(FONT_CAPS, ts, label) + 2 * label_inset(carved(ui_px(90), h), st);
}

void ui_slider_layout(Slider *s, Rect row) {
    s->r = row;
    int label_w = row.w * 42 / 100;
    int knob = ui_px(26);
    s->track = (Rect){row.x + label_w + knob, row.y + row.h / 2 - ui_px(5), row.w - label_w - 2 * knob, ui_px(10)};
}

static float slider_t(const Slider *s) {
    return s->max > s->min ? clampf((s->value - s->min) / (s->max - s->min), 0.f, 1.f) : 0.f;
}

static int slider_knob_radius(void) { return ui_px(gothic() ? 32 : 26); }

void ui_slider_draw(Canvas *c, const Slider *s, const char *value_text) {
    canvas_fill(c, s->r, 255);
    int label_w = s->r.w * 42 / 100;
    char vbuf[48];
    if (!value_text) {
        snprintf(vbuf, sizeof vbuf, "%.2f%s%s", s->value, s->unit && *s->unit ? " " : "", s->unit ? s->unit : "");
        value_text = vbuf;
    }
    float ts = ui_text_size();
    int vw = font_measure(FONT_BOLD, ts, value_text);
    Rect lr = {s->r.x, s->r.y, label_w - vw - ui_px(16), s->r.h};
    font_draw_left(c, FONT_REGULAR, ts, lr, s->label ? s->label : "", 0);
    font_draw_left(c, FONT_BOLD, ts, (Rect){s->r.x + label_w - vw, s->r.y, vw + 2, s->r.h}, value_text, 0);
    Rect tr = s->track;
    float t = slider_t(s);
    int fx = tr.x + (int)lrintf(t * (float)tr.w);
    float cy = (float)tr.y + tr.h / 2.f;
    if (gothic()) {
        // A groove of two hairlines, filled solid up to a lozenge knob.
        canvas_hline(c, tr.x, tr.x + tr.w, tr.y, ui_px(2), 0);
        canvas_hline(c, tr.x, tr.x + tr.w, tr.y + tr.h - ui_px(2), ui_px(2), 0);
        canvas_fill(c, (Rect){tr.x, tr.y, fx - tr.x, tr.h}, 0);
        canvas_lozenge(c, (float)fx, cy, (float)slider_knob_radius(), 0);
        canvas_lozenge(c, (float)fx, cy, (float)ui_px(20), 255);
        canvas_lozenge(c, (float)fx, cy, (float)ui_px(10), 0);
        return;
    }
    canvas_round_rect(c, tr, tr.h / 2, 190, true, 0);
    canvas_round_rect(c, (Rect){tr.x, tr.y, fx - tr.x + tr.h / 2, tr.h}, tr.h / 2, 0, true, 0);
    canvas_disc_aa(c, (float)fx, cy, (float)slider_knob_radius(), 0, 1.f);
    canvas_disc_aa(c, (float)fx, cy, (float)ui_px(12), 255, 1.f);
}

static bool slider_set_value(Slider *s, float v) {
    if (s->step > 0.f) v = s->min + roundf((v - s->min) / s->step) * s->step;
    v = clampf(v, s->min, s->max);
    if (fabsf(v - s->value) < 1e-6f) return false;
    s->value = v;
    return true;
}

bool ui_slider_set_from_x(Slider *s, float x) {
    float t = s->track.w > 0 ? clampf((x - (float)s->track.x) / (float)s->track.w, 0.f, 1.f) : 0.f;
    return slider_set_value(s, s->min + t * (s->max - s->min));
}

void ui_slider_drag_begin(const Slider *s, SliderDrag *d, float x, uint32_t now) {
    *d = (SliderDrag){.x = x, .value = s->value, .started = now};
}

bool ui_slider_drag_to(Slider *s, SliderDrag *d, float x, float y, uint32_t now) {
    if (now - d->started >= 350u) d->held = true;
    float distance = fabsf(y - (s->track.y + s->track.h * 0.5f));
    bool changed;
    // In a band as tall as the visible knob, catch up to the pointer and track it directly.
    // Leaving that band after a hold resumes fine motion from the current value.
    if (!d->held || distance <= slider_knob_radius()) {
        changed = ui_slider_set_from_x(s, x);
        d->value = s->value;
    } else {
        float gain = fmaxf(0.01f, powf(0.25f, distance / (float)ui_px(80)));
        if (s->track.w > 0)
            d->value += (x - d->x) * (s->max - s->min) / s->track.w * gain;
        d->value = clampf(d->value, s->min, s->max);
        changed = slider_set_value(s, d->value);
    }
    d->x = x;
    return changed;
}

Rect ui_sheet(Canvas *c, Rect card, const char *title) {
    if (gothic()) {
        canvas_chamfer_rect(c, card, ui_px(34), 255, true, 0);
        canvas_chamfer_rect(c, card, ui_px(34), 0, false, ui_px(6));
        Rect inner = rect_inflate(card, -ui_px(16));
        canvas_chamfer_rect(c, inner, ui_px(26), 0, false, ui_px(2));
        int th = ui_px(130);
        if (title && *title) {
            font_draw_centered(c, FONT_TITLE, ui_title_size(), (Rect){card.x + ui_px(80), card.y + ui_px(10),
                                                                      card.w - ui_px(160), th - ui_px(20)},
                               title, 0);
            ornament(c, card.x + ui_px(60), card.x + card.w - ui_px(60), card.y + th, 0);
            return rect_inflate((Rect){card.x, card.y + th, card.w, card.h - th}, -ui_px(40));
        }
        return rect_inflate(card, -ui_px(44));
    }
    canvas_round_rect(c, card, ui_px(24), 255, true, 0);
    canvas_round_rect(c, card, ui_px(24), 0, false, ui_px(6));
    int th = ui_px(120);
    if (title && *title) {
        font_draw_left(c, FONT_BOLD, ui_title_size(), (Rect){card.x + ui_px(40), card.y, card.w - ui_px(80), th}, title,
                       0);
        canvas_hline(c, card.x + ui_px(6), card.x + card.w - ui_px(6), card.y + th, ui_px(3), 0);
        return rect_inflate((Rect){card.x, card.y + th, card.w, card.h - th}, -ui_px(36));
    }
    return rect_inflate(card, -ui_px(36));
}

// A toast is one line of text, or two at the same size when it needs them: the box grows by a
// line, as wide as the longer line needs, and keeps the ends of a one-line box (Gothic's points,
// Classic's round ends), which the text stays clear of.
static int toast_one_h(void) { return ui_px(110); }
static int toast_inset(void) { return gothic() ? ui_px(50) : ui_px(30); }

int ui_toast_room(Rect screen) { return screen.w - ui_px(60) - 2 * toast_inset(); }

Rect ui_toast_rect(Rect screen, const char *msg) {
    float ts = ui_text_size();
    int max_w = screen.w - ui_px(60);
    int w = font_measure(FONT_BOLD, ts, msg) + ui_px(100), h = toast_one_h();
    if (w > max_w) {
        w = max_w;
        int widest, lines = font_wrap_lines(FONT_BOLD, ts, msg, ui_toast_room(screen), &widest);
        if (lines > 1) h += font_ascent(FONT_BOLD, ts) + font_descent(FONT_BOLD, ts);
        if (lines == 2 && widest + ui_px(100) < max_w) w = widest + ui_px(100);
    }
    return (Rect){screen.x + (screen.w - w) / 2, screen.y + screen.h - h - ui_px(120), w, h};
}

void ui_toast(Canvas *c, Rect screen, const char *msg) { ui_toast_at(c, ui_toast_rect(screen, msg), msg); }

void ui_toast_at(Canvas *c, Rect r, const char *msg) {
    float ts = ui_text_size();
    int ends = toast_one_h() / 2;  // the ends' depth (or radius): half a one-line box
    Rect tr = rect_inflate(r, -toast_inset());
    if (gothic()) {
        // A black banner with pointed ends and a white inner line.
        canvas_chamfer_rect(c, r, ends, 0, true, 0);
        canvas_chamfer_rect(c, rect_inflate(r, -ui_px(9)), ends - ui_px(9), 255, false, ui_px(2));
    } else {
        canvas_round_rect(c, r, ends, 0, true, 0);
    }
    if (r.h > toast_one_h()) font_draw_wrapped(c, FONT_BOLD, ts, tr, msg, 2, 0, true, 255);
    else font_draw_centered(c, FONT_BOLD, ts, tr, msg, 255);
}

void ui_checkbox(Canvas *c, Rect r, const char *label, bool checked) {
    canvas_fill(c, r, 255);
    int bs = ui_px(64);
    Rect b = {r.x, r.y + (r.h - bs) / 2, bs, bs};
    if (gothic()) {
        // A lozenge, filled when ticked.
        float cx = b.x + bs / 2.f, cy = b.y + bs / 2.f, rr = bs / 2.f + ui_px(4);
        canvas_lozenge(c, cx, cy, rr, 0);
        canvas_lozenge(c, cx, cy, rr - ui_px(5), 255);
        if (checked) canvas_lozenge(c, cx, cy, rr - ui_px(14), 0);
    } else {
        canvas_round_rect(c, b, ui_px(10), checked ? 0 : 255, true, 0);
        canvas_round_rect(c, b, ui_px(10), 0, false, ui_px(4));
        if (checked) ui_icon(c, ICON_CHECK, b, 255);
    }
    ui_text(c, FONT_REGULAR, ui_text_size(), ui_small_size(), (Rect){r.x + bs + ui_px(28), r.y, r.w - bs - ui_px(28), r.h},
            label, 2, false, 0);
}

void ui_text(Canvas *c, FontFace f, float px, float min_px, Rect r, const char *text, int max_lines, bool center,
             uint8_t gray) {
    if (min_px > px) min_px = px;
    if (font_measure(f, px, text) > r.w) {
        float s = font_fit_px(f, px, min_px, text, r.w);
        if (s > 0.f) {
            px = s;
        } else {
            int lh = font_ascent(f, min_px) + font_descent(f, min_px), room = lh > 0 ? r.h / lh : 1;
            font_draw_wrapped(c, f, min_px, r, text, max_lines < room ? max_lines : room < 1 ? 1 : room, 0, center, gray);
            return;
        }
    }
    if (center) font_draw_centered(c, f, px, r, text, gray);
    else font_draw_left(c, f, px, r, text, gray);
}

int ui_text_box_w(FontFace f, float px, const char *text, int pad, int min_w, int max_w) {
    return clampi(font_measure(f, px, text) + 2 * pad, min_w, max_w);
}

int ui_chip_w(FontFace f, float px, const char *text, int h, int min_w, int max_w) {
    // Gothic's ends are points as deep as half the chip (ui_frame's cut corners): the text keeps
    // all but 7 px of that from each tip. Classic's round ends leave the text room closer in.
    return ui_text_box_w(f, px, text, gothic() ? h / 2 - ui_px(7) : ui_px(24), min_w, max_w);
}

const char *ui_name_label(const char *name, char *buf, size_t cap) {
    if (!name) name = "";
    if (!cap || !font_drawable_copy(name, buf, cap)) return name;
    for (const unsigned char *p = (const unsigned char *)buf; *p; p++)
        if (*p >= 0x80 || (unsigned)((*p | 0x20) - 'a') < 26u) return buf;  // a letter is left
    char rest[256];
    strlcpy_q(rest, buf, sizeof rest);
    snprintf(buf, cap, "%s%s%s", _("Untitled"), rest[0] ? " " : "", rest);
    return buf;
}

void press_begin(Press *p, float x, float y, uint32_t t) {
    *p = (Press){.down = true, .x0 = x, .y0 = y, .x = x, .y = y, .t0 = t};
}

void press_move(Press *p, float x, float y) {
    p->x = x;
    p->y = y;
    float s = (float)ui_px(30);
    if (fabsf(x - p->x0) > s || fabsf(y - p->y0) > s) p->moved = true;
}

bool press_is_tap(const Press *p, Rect r) {
    return !p->moved && rect_contains(r, (int)p->x0, (int)p->y0) && rect_contains(r, (int)p->x, (int)p->y);
}
