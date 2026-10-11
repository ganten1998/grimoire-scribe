// keyboard.c: on-screen keyboard modal. Layout is computed from the area it was drawn in,
// so hit testing and drawing always agree. keyboard_draw paints the whole modal (cheap);
// keyboard_pointer reports only the rect that needs refreshing.
#include "keyboard.h"

#include <ctype.h>
#include <string.h>

enum { K_SHIFT = 1, K_BKSP = 8, K_DONE = 13, K_CANCEL = 27 };

typedef struct {
    Rect r;
    int code;
} Key;

#define MAX_KEYS 48

static Rect modal_rect(Rect screen) {
    int h = ui_px(170) + 5 * ui_px(150) + ui_px(60);
    return (Rect){screen.x + ui_px(20), screen.y + screen.h - h - ui_px(20), screen.w - ui_px(40), h};
}

static Rect field_rect(Rect area) {
    return (Rect){area.x + ui_px(30), area.y + ui_px(30), area.w - ui_px(60), ui_px(120)};
}

static int layout(Rect area, Key *keys) {
    static const char *rows[3] = {"1234567890", "qwertyuiop", "asdfghjkl"};
    int n = 0;
    int kh = ui_px(150), gap = ui_px(10);
    int top = area.y + ui_px(170);
    int inner_x = area.x + ui_px(30), inner_w = area.w - ui_px(60);
    int kw = (inner_w - 9 * gap) / 10;
    for (int r = 0; r < 3; r++) {
        int cnt = (int)strlen(rows[r]);
        int rowx = inner_x + (inner_w - (cnt * kw + (cnt - 1) * gap)) / 2;
        for (int i = 0; i < cnt; i++)
            keys[n++] = (Key){{rowx + i * (kw + gap), top + r * (kh + gap), kw, kh}, rows[r][i]};
    }
    // Row 4: shift, z..m, backspace.
    int y = top + 3 * (kh + gap);
    int wide = kw + kw / 2;
    keys[n++] = (Key){{inner_x, y, wide, kh}, K_SHIFT};
    const char *zr = "zxcvbnm";
    int zx = inner_x + wide + gap;
    for (int i = 0; i < 7; i++) keys[n++] = (Key){{zx + i * (kw + gap), y, kw, kh}, zr[i]};
    int bx = zx + 7 * (kw + gap);
    keys[n++] = (Key){{bx, y, inner_x + inner_w - bx, kh}, K_BKSP};
    // Row 5: cancel, -, space, ., done.
    y += kh + gap;
    int cw = kw * 2;
    keys[n++] = (Key){{inner_x, y, cw, kh}, K_CANCEL};
    keys[n++] = (Key){{inner_x + cw + gap, y, kw, kh}, '-'};
    int sx = inner_x + cw + kw + 2 * gap;
    int dx = inner_x + inner_w - cw - gap - kw;
    keys[n++] = (Key){{sx, y, dx - gap - sx, kh}, ' '};
    keys[n++] = (Key){{dx, y, kw, kh}, '.'};
    keys[n++] = (Key){{inner_x + inner_w - cw, y, cw, kh}, K_DONE};
    return n;
}

void keyboard_open(Keyboard *kb, const char *title, const char *initial) {
    memset(kb, 0, sizeof *kb);
    kb->open = true;
    kb->pressed_key = -1;
    strlcpy_q(kb->title, title ? title : "", sizeof kb->title);
    // ASCII only: the font has the glyphs, but the keyboard can only type ASCII anyway.
    size_t n = 0;
    for (const char *c = initial ? initial : ""; *c && n + 1 < sizeof kb->text; c++)
        if ((unsigned char)*c >= 32 && (unsigned char)*c < 127) kb->text[n++] = *c;
    kb->text[n] = 0;
}

void keyboard_close(Keyboard *kb) { kb->open = false; }

static void key_label(const Keyboard *kb, int code, char *buf) {
    switch (code) {
    case K_SHIFT: strcpy(buf, "shift"); break;
    case K_BKSP: strcpy(buf, "del"); break;
    case K_DONE: strcpy(buf, "Done"); break;
    case K_CANCEL: strcpy(buf, "Cancel"); break;
    case ' ': strcpy(buf, "space"); break;
    default:
        buf[0] = (char)(kb->shift ? toupper(code) : code);
        buf[1] = 0;
    }
}

Rect keyboard_draw(Keyboard *kb, Canvas *c, Rect screen) {
    Rect area = modal_rect(screen);
    kb->area = area;
    canvas_round_rect(c, area, ui_px(24), 255, true, 0);
    canvas_round_rect(c, area, ui_px(24), 0, false, ui_px(6));
    Rect f = field_rect(area);
    canvas_round_rect(c, f, ui_px(12), 255, true, 0);
    canvas_round_rect(c, f, ui_px(12), 0, false, ui_px(3));
    float ts = ui_text_size();
    int tw = kb->title[0] ? font_measure(FONT_BOLD, ts, kb->title) + ui_px(30) : 0;
    if (tw) font_draw_left(c, FONT_BOLD, ts, (Rect){f.x + ui_px(24), f.y, tw, f.h}, kb->title, 0);
    // Show the tail of long text so the caret stays visible.
    Rect tr = {f.x + ui_px(24) + tw, f.y, f.w - ui_px(48) - tw, f.h};
    const char *shown = kb->text;
    while (*shown && font_measure(FONT_REGULAR, ts * 1.1f, shown) > tr.w - ui_px(20)) shown++;
    int w = font_measure(FONT_REGULAR, ts * 1.1f, shown);
    font_draw_left(c, FONT_REGULAR, ts * 1.1f, tr, shown, 0);
    canvas_fill(c, (Rect){tr.x + w + ui_px(4), f.y + ui_px(24), ui_px(5), f.h - ui_px(48)}, 0);
    Key keys[MAX_KEYS];
    int n = layout(area, keys);
    for (int i = 0; i < n; i++) {
        char lab[16];
        key_label(kb, keys[i].code, lab);
        ButtonStyle st = (i == kb->pressed_key || (keys[i].code == K_SHIFT && kb->shift)) ? BTN_SELECTED
                         : keys[i].code == K_DONE                                       ? BTN_PRIMARY
                                                                                        : BTN_PLAIN;
        canvas_round_rect(c, keys[i].r, ui_px(12), st == BTN_SELECTED ? 0 : 255, true, 0);
        canvas_round_rect(c, keys[i].r, ui_px(12), 0, false, st == BTN_PRIMARY ? ui_px(6) : ui_px(3));
        float ks = strlen(lab) > 1 ? ui_text_size() : ui_title_size();
        font_draw_centered(c, FONT_BOLD, ks, keys[i].r, lab, st == BTN_SELECTED ? 255 : 0);
    }
    return area;
}

static int key_at(const Keyboard *kb, float x, float y) {
    Key keys[MAX_KEYS];
    int n = layout(kb->area, keys);
    for (int i = 0; i < n; i++)
        if (rect_contains(rect_inflate(keys[i].r, ui_px(5)), (int)x, (int)y)) return i;
    return -1;
}

KbResult keyboard_pointer(Keyboard *kb, int phase, float x, float y, uint32_t t, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!kb->open) return KB_NONE;
    Key keys[MAX_KEYS];
    int n = layout(kb->area, keys);
    if (phase == 0) {
        press_begin(&kb->press, x, y, t);
        kb->pressed_key = key_at(kb, x, y);
        if (kb->pressed_key >= 0) *dirty = keys[kb->pressed_key].r;
        return KB_NONE;
    }
    if (phase == 1) {
        press_move(&kb->press, x, y);
        return KB_NONE;
    }
    int k = kb->pressed_key;
    kb->pressed_key = -1;
    if (k < 0 || k >= n) return KB_NONE;
    *dirty = keys[k].r;
    if (key_at(kb, x, y) != k) return KB_NONE;  // slid off the key: no input
    int code = keys[k].code;
    size_t len = strlen(kb->text);
    switch (code) {
    case K_CANCEL: kb->open = false; *dirty = kb->area; return KB_CANCEL;
    case K_DONE: kb->open = false; *dirty = kb->area; return KB_DONE;
    case K_SHIFT:
        kb->shift = !kb->shift;
        *dirty = kb->area;
        return KB_CHANGED;
    case K_BKSP:
        if (len) kb->text[len - 1] = 0;
        break;
    default:
        if (len + 1 < sizeof kb->text) {
            kb->text[len] = (char)(kb->shift ? toupper(code) : code);
            kb->text[len + 1] = 0;
        }
        if (kb->shift && isalpha(code)) {
            kb->shift = false;  // one-shot
            *dirty = kb->area;
        }
    }
    *dirty = rect_union(*dirty, field_rect(kb->area));
    return KB_CHANGED;
}
