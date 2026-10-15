// ui.c: widgets. Type and icons are Terminess Nerd Font.
#include "ui.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static float g_scale = 1.f;

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
};

IconKind ui_tool_icon(int tool) {
    static const IconKind k[] = {ICON_PENCIL,   ICON_MECH_PENCIL, ICON_BALLPOINT, ICON_ERASER,
                                 ICON_HIGHLIGHTER, ICON_FOUNTAIN, ICON_FINELINER,  ICON_MARKER};
    return tool >= 0 && tool < (int)(sizeof k / sizeof k[0]) ? k[tool] : ICON_PENCIL;
}

void ui_icon(Canvas *c, IconKind k, Rect r, uint8_t gray) {
    if (k <= ICON_NONE || k >= ICON_COUNT || !k_icon_cp[k]) return;
    int inset = (r.w < r.h ? r.w : r.h) / 10;
    font_draw_icon(c, FONT_REGULAR, k_icon_cp[k], rect_inflate(r, -inset), gray);
}

static int radius(void) { return ui_px(16); }

void ui_button(Canvas *c, Rect r, IconKind icon, const char *label, ButtonStyle st) {
    uint8_t bg = st == BTN_SELECTED ? 0 : 255;
    uint8_t fg = st == BTN_SELECTED ? 255 : (st == BTN_DISABLED ? 170 : 0);
    uint8_t edge = st == BTN_DISABLED ? 170 : 0;
    canvas_round_rect(c, r, radius(), bg, true, 0);
    canvas_round_rect(c, r, radius(), edge, false, st == BTN_PRIMARY ? ui_px(6) : ui_px(3));
    bool has_label = label && *label;
    Rect in = rect_inflate(r, -ui_px(8));
    if (icon && has_label) {
        float ts = fminf(ui_small_size(), (float)in.h * 0.28f);
        int lh = (int)(ts * 1.15f);
        Rect ir = {in.x, in.y, in.w, in.h - lh};
        ui_icon(c, icon, ir, fg);
        font_draw_centered(c, FONT_BOLD, ts, (Rect){in.x, in.y + in.h - lh, in.w, lh}, label, fg);
    } else if (icon) {
        ui_icon(c, icon, rect_inflate(in, -ui_px(6)), fg);
    } else if (has_label) {
        float ts = fminf(ui_text_size(), (float)in.h * 0.6f);
        font_draw_centered(c, FONT_BOLD, ts, in, label, fg);
    }
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
    canvas_round_rect(c, tr, tr.h / 2, 190, true, 0);
    float t = slider_t(s);
    int fx = tr.x + (int)lrintf(t * (float)tr.w);
    canvas_round_rect(c, (Rect){tr.x, tr.y, fx - tr.x + tr.h / 2, tr.h}, tr.h / 2, 0, true, 0);
    float cy = (float)tr.y + tr.h / 2.f;
    canvas_disc_aa(c, (float)fx, cy, (float)ui_px(26), 0, 1.f);
    canvas_disc_aa(c, (float)fx, cy, (float)ui_px(12), 255, 1.f);
}

bool ui_slider_set_from_x(Slider *s, float x) {
    float t = s->track.w > 0 ? clampf((x - (float)s->track.x) / (float)s->track.w, 0.f, 1.f) : 0.f;
    float v = s->min + t * (s->max - s->min);
    if (s->step > 0.f) v = s->min + roundf((v - s->min) / s->step) * s->step;
    v = clampf(v, s->min, s->max);
    if (fabsf(v - s->value) < 1e-6f) return false;
    s->value = v;
    return true;
}

Rect ui_sheet(Canvas *c, Rect card, const char *title) {
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

Rect ui_toast_rect(Rect screen, const char *msg) {
    int w = font_measure(FONT_BOLD, ui_text_size(), msg) + ui_px(100);
    if (w > screen.w - ui_px(60)) w = screen.w - ui_px(60);
    int h = ui_px(110);
    return (Rect){screen.x + (screen.w - w) / 2, screen.y + screen.h - h - ui_px(120), w, h};
}

void ui_toast(Canvas *c, Rect screen, const char *msg) {
    float ts = ui_text_size();
    Rect r = ui_toast_rect(screen, msg);
    int h = r.h;
    canvas_round_rect(c, r, h / 2, 0, true, 0);
    font_draw_centered(c, FONT_BOLD, ts, rect_inflate(r, -ui_px(30)), msg, 255);
}

void ui_checkbox(Canvas *c, Rect r, const char *label, bool checked) {
    canvas_fill(c, r, 255);
    int bs = ui_px(64);
    Rect b = {r.x, r.y + (r.h - bs) / 2, bs, bs};
    canvas_round_rect(c, b, ui_px(10), checked ? 0 : 255, true, 0);
    canvas_round_rect(c, b, ui_px(10), 0, false, ui_px(4));
    if (checked) ui_icon(c, ICON_CHECK, b, 255);
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){r.x + bs + ui_px(28), r.y, r.w - bs - ui_px(28), r.h},
                   label, 0);
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
