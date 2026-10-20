// keyboard.c: on-screen keyboard modal. Layout is computed from the area it was drawn in,
// so hit testing and drawing always agree. keyboard_draw paints the whole modal (cheap);
// keyboard_pointer reports only the rect that needs refreshing. The text is UTF-8 throughout:
// keys carry code points, a key appends a whole character and backspace removes one.
#include "keyboard.h"

#include <string.h>

typedef struct {
    Rect r;
    uint32_t code;  // a code point, or a KB_KEY_* code
} Key;

#define MAX_KEYS 48

// The character keys of each page, row by row (zero-terminated). Rows 0-2 span the keyboard;
// row 3 sits between shift and delete.
static const uint16_t k_rows[2][4][11] = {
    {{'1', '2', '3', '4', '5', '6', '7', '8', '9', '0'},
     {'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'},
     {'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'},
     {'z', 'x', 'c', 'v', 'b', 'n', 'm'}},
    // Accents, enough for Spanish, Portuguese, French, Italian, German, Dutch, Catalan, Danish,
    // Norwegian, Swedish and Finnish (shift gives the capitals): a e i o u with an acute, then
    // a grave; with a circumflex, then a diaeresis; n tilde, c cedilla, a and o tilde, sharp s,
    // oe, ae, o slash, a ring; the inverted question and exclamation marks, the angle quotes and
    // the middle dot Catalan puts between two l's.
    {{0xE1, 0xE9, 0xED, 0xF3, 0xFA, 0xE0, 0xE8, 0xEC, 0xF2, 0xF9},
     {0xE2, 0xEA, 0xEE, 0xF4, 0xFB, 0xE4, 0xEB, 0xEF, 0xF6, 0xFC},
     {0xF1, 0xE7, 0xE3, 0xF5, 0xDF, 0x153, 0xE6, 0xF8, 0xE5},
     {0xBF, 0xA1, 0xAB, 0xBB, 0xB7}},
};

// In Spanish the letters page's middle row ends with n tilde, as on Spanish keyboards (to the right
// of l): a letter of the alphabet there, not an accent to fetch from the other page.
static const uint16_t k_row_es[11] = {'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 0xF1};

static int row_len(const uint16_t *row) {
    int n = 0;
    while (n < 11 && row[n]) n++;
    return n;
}

// ---- UTF-8 ----

// Encodes cp into buf (room for 4 bytes); returns the length.
static int utf8_put(uint32_t cp, char *buf) {
    if (cp < 0x80) {
        buf[0] = (char)cp;
        return 1;
    }
    if (cp < 0x800) {
        buf[0] = (char)(0xC0 | cp >> 6);
        buf[1] = (char)(0x80 | (cp & 0x3F));
        return 2;
    }
    if (cp < 0x10000) {
        buf[0] = (char)(0xE0 | cp >> 12);
        buf[1] = (char)(0x80 | (cp >> 6 & 0x3F));
        buf[2] = (char)(0x80 | (cp & 0x3F));
        return 3;
    }
    buf[0] = (char)(0xF0 | cp >> 18);
    buf[1] = (char)(0x80 | (cp >> 12 & 0x3F));
    buf[2] = (char)(0x80 | (cp >> 6 & 0x3F));
    buf[3] = (char)(0x80 | (cp & 0x3F));
    return 4;
}

// Length of the character at s (1-4) with its code point in *cp, or 0 if the bytes there are
// not valid UTF-8: a stray or missing continuation byte, an overlong form, a surrogate, or a
// code point past U+10FFFF.
static int utf8_get(const char *s, uint32_t *cp) {
    const unsigned char *p = (const unsigned char *)s;
    if (p[0] < 0x80) {
        *cp = p[0];
        return 1;
    }
    int len = p[0] >= 0xF0 ? 4 : p[0] >= 0xE0 ? 3 : p[0] >= 0xC0 ? 2 : 0;
    if (!len || p[0] > 0xF4) return 0;
    uint32_t c = p[0] & (0x7Fu >> len);
    for (int i = 1; i < len; i++) {
        if ((p[i] & 0xC0) != 0x80) return 0;  // also stops at the terminator
        c = c << 6 | (p[i] & 0x3Fu);
    }
    static const uint32_t least[5] = {0, 0, 0x80, 0x800, 0x10000};
    if (c < least[len] || c > 0x10FFFF || (c >= 0xD800 && c <= 0xDFFF)) return 0;
    *cp = c;
    return len;
}

// The character after the one at s (not at the terminator).
static const char *utf8_next(const char *s) {
    s++;
    while (((unsigned char)*s & 0xC0) == 0x80) s++;
    return s;
}

// Characters (not bytes) in s.
static int utf8_count(const char *s) {
    int n = 0;
    for (; *s; s++) n += ((unsigned char)*s & 0xC0) != 0x80;
    return n;
}

// Copies the valid characters of src into dst (cap bytes with the terminator), dropping control
// characters and broken UTF-8 and stopping before a character that would not fit.
static void copy_text(char *dst, const char *src, size_t cap) {
    size_t n = 0;
    for (const char *s = src ? src : ""; *s;) {
        uint32_t cp;
        int len = utf8_get(s, &cp);
        if (!len) {
            s++;  // not UTF-8: drop the byte
            continue;
        }
        if (cp >= 32 && (cp < 0x7F || cp >= 0xA0)) {
            if (n + (size_t)len >= cap) break;
            memcpy(dst + n, s, (size_t)len);
            n += (size_t)len;
        }
        s += len;
    }
    dst[n] = 0;
}

uint32_t keyboard_upper(uint32_t c) {
    if (c >= 'a' && c <= 'z') return c - 32;
    if (c >= 0xE0 && c <= 0xFE && c != 0xF7) return c - 32;  // Latin-1 (not the division sign)
    if (c == 0xFF) return 0x178;                             // y diaeresis
    if (c == 0x131) return 'I';                              // dotless i
    // Latin Extended-A pairs each capital with its small letter: the capital at the even code
    // point, except in two runs where it takes the odd one.
    if ((c >= 0x100 && c <= 0x137) || (c >= 0x14A && c <= 0x177)) return c & ~1u;
    if ((c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17E)) return c & 1 ? c : c - 1;
    return c;
}

// ---- layout ----

static Rect modal_rect(Rect screen) {
    int h = ui_px(170) + 5 * ui_px(150) + ui_px(60);
    return (Rect){screen.x + ui_px(20), screen.y + screen.h - h - ui_px(20), screen.w - ui_px(40), h};
}

static Rect field_rect(Rect area) {
    return (Rect){area.x + ui_px(30), area.y + ui_px(30), area.w - ui_px(60), ui_px(120)};
}

static int layout(Rect area, int page, Key *keys) {
    const uint16_t (*rows)[11] = k_rows[page ? 1 : 0];
    int n = 0;
    int kh = ui_px(150), gap = ui_px(10);
    int top = area.y + ui_px(170);
    int inner_x = area.x + ui_px(30), inner_w = area.w - ui_px(60);
    int kw = (inner_w - 9 * gap) / 10;
    bool enye = !page && !strcmp(i18n_current(), "es");
    for (int r = 0; r < 3; r++) {
        const uint16_t *row = enye && r == 2 ? k_row_es : rows[r];
        int cnt = row_len(row);
        int rowx = inner_x + (inner_w - (cnt * kw + (cnt - 1) * gap)) / 2;
        for (int i = 0; i < cnt; i++)
            keys[n++] = (Key){{rowx + i * (kw + gap), top + r * (kh + gap), kw, kh}, row[i]};
    }
    // Row 4: shift, up to seven keys (centered between), backspace.
    int y = top + 3 * (kh + gap);
    int wide = kw + kw / 2;
    keys[n++] = (Key){{inner_x, y, wide, kh}, KB_KEY_SHIFT};
    int zx = inner_x + wide + gap, bx = zx + 7 * (kw + gap);
    int cnt = row_len(rows[3]);
    int rowx = zx + (bx - gap - zx - (cnt * kw + (cnt - 1) * gap)) / 2;
    for (int i = 0; i < cnt; i++) keys[n++] = (Key){{rowx + i * (kw + gap), y, kw, kh}, rows[3][i]};
    keys[n++] = (Key){{bx, y, inner_x + inner_w - bx, kh}, KB_KEY_BKSP};
    // Row 5: cancel, -, the other page (as wide as shift), space, ., done.
    y += kh + gap;
    int cw = kw * 2;
    keys[n++] = (Key){{inner_x, y, cw, kh}, KB_KEY_CANCEL};
    keys[n++] = (Key){{inner_x + cw + gap, y, kw, kh}, '-'};
    int px = inner_x + cw + kw + 2 * gap;
    keys[n++] = (Key){{px, y, wide, kh}, KB_KEY_PAGE};
    int sx = px + wide + gap;
    int dx = inner_x + inner_w - cw - gap - kw;
    keys[n++] = (Key){{sx, y, dx - gap - sx, kh}, ' '};
    keys[n++] = (Key){{dx, y, kw, kh}, '.'};
    keys[n++] = (Key){{inner_x + inner_w - cw, y, cw, kh}, KB_KEY_DONE};
    return n;
}

void keyboard_open(Keyboard *kb, const char *title, const char *initial) {
    memset(kb, 0, sizeof *kb);
    kb->open = true;
    kb->pressed_key = -1;
    copy_text(kb->title, title, sizeof kb->title);
    copy_text(kb->text, initial, sizeof kb->text);
}

void keyboard_close(Keyboard *kb) { kb->open = false; }

// What a key shows: a name for the named keys, else its character as it would type now (in
// ch, room for 5 bytes).
static const char *key_label(const Keyboard *kb, uint32_t code, char *ch) {
    switch (code) {
    // TRANSLATORS: the keyboard's named keys, in bold at 44 px: shift (250 px wide), delete
    // (261), the space bar, and Cancel and Done (334).
    case KB_KEY_SHIFT: return C_("key", "shift");
    case KB_KEY_BKSP: return C_("key", "del");
    case KB_KEY_DONE: return _("Done");
    case KB_KEY_CANCEL: return _("Cancel");
    case KB_KEY_PAGE: return kb->page ? "abc" : "\xc3\xa1\xc3\xa9";  // a and e acute
    case ' ': return C_("key", "space");
    default:
        ch[utf8_put(kb->shift ? keyboard_upper(code) : code, ch)] = 0;
        return ch;
    }
}

Rect keyboard_draw(Keyboard *kb, Canvas *c, Rect screen) {
    Rect area = modal_rect(screen);
    kb->area = area;
    ui_frame(c, area, ui_px(24), 255, true, 0);
    ui_frame(c, area, ui_px(24), 0, false, ui_px(6));
    Rect f = field_rect(area);
    ui_frame(c, f, ui_px(12), 255, true, 0);
    ui_frame(c, f, ui_px(12), 0, false, ui_px(3));
    float ts = ui_text_size();
    int tw = kb->title[0] ? font_measure(FONT_BOLD, ts, kb->title) + ui_px(30) : 0;
    if (tw) font_draw_left(c, FONT_BOLD, ts, (Rect){f.x + ui_px(24), f.y, tw, f.h}, kb->title, 0);
    // Show the tail of long text so the caret stays visible.
    Rect tr = {f.x + ui_px(24) + tw, f.y, f.w - ui_px(48) - tw, f.h};
    const char *shown = kb->text;
    while (*shown && font_measure(FONT_REGULAR, ts * 1.1f, shown) > tr.w - ui_px(20)) shown = utf8_next(shown);
    int w = font_measure(FONT_REGULAR, ts * 1.1f, shown);
    font_draw_left(c, FONT_REGULAR, ts * 1.1f, tr, shown, 0);
    canvas_fill(c, (Rect){tr.x + w + ui_px(4), f.y + ui_px(24), ui_px(5), f.h - ui_px(48)}, 0);
    Key keys[MAX_KEYS];
    int n = layout(area, kb->page, keys);
    for (int i = 0; i < n; i++) {
        char ch[8];
        const char *lab = key_label(kb, keys[i].code, ch);
        ButtonStyle st = (i == kb->pressed_key || (keys[i].code == KB_KEY_SHIFT && kb->shift)) ? BTN_SELECTED
                         : keys[i].code == KB_KEY_DONE                                         ? BTN_PRIMARY
                                                                                               : BTN_PLAIN;
        ui_frame(c, keys[i].r, ui_px(12), st == BTN_SELECTED ? 0 : 255, true, 0);
        ui_frame(c, keys[i].r, ui_px(12), 0, false, st == BTN_PRIMARY ? ui_px(6) : ui_px(3));
        float ks = utf8_count(lab) > 1 ? ui_text_size() : ui_title_size();
        font_draw_centered(c, FONT_BOLD, ks, keys[i].r, lab, st == BTN_SELECTED ? 255 : 0);
    }
    return area;
}

static int key_at(const Keyboard *kb, float x, float y) {
    Key keys[MAX_KEYS];
    int n = layout(kb->area, kb->page, keys);
    for (int i = 0; i < n; i++)
        if (rect_contains(rect_inflate(keys[i].r, ui_px(5)), (int)x, (int)y)) return i;
    return -1;
}

// Appends the key's character (its capital while shift is on) if it fits; shift is one-shot.
static void type_key(Keyboard *kb, uint32_t code, Rect *dirty) {
    uint32_t ch = kb->shift ? keyboard_upper(code) : code;
    char enc[4];
    size_t len = strlen(kb->text), n = (size_t)utf8_put(ch, enc);
    if (len + n < sizeof kb->text) {  // a character that does not fit is not typed
        memcpy(kb->text + len, enc, n);
        kb->text[len + n] = 0;
    }
    if (kb->shift && ch != code) {
        kb->shift = false;
        *dirty = kb->area;
    }
}

bool keyboard_key_rect(const Keyboard *kb, uint32_t code, Rect *r) {
    Key keys[MAX_KEYS];
    int n = layout(kb->area, kb->page, keys);
    for (int i = 0; i < n; i++)
        if (keys[i].code == code) {
            *r = keys[i].r;
            return true;
        }
    return false;
}

KbResult keyboard_pointer(Keyboard *kb, int phase, float x, float y, uint32_t t, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!kb->open) return KB_NONE;
    kb->field_dirty = RECT_EMPTY;
    Key keys[MAX_KEYS];
    int n = layout(kb->area, kb->page, keys);
    if (phase == 0) {
        press_begin(&kb->press, x, y, t);
        kb->pressed_key = key_at(kb, x, y);
        kb->typed = false;
        if (kb->pressed_key < 0) return KB_NONE;
        *dirty = keys[kb->pressed_key].r;
        uint32_t c = keys[kb->pressed_key].code;
        if (c == KB_KEY_SHIFT || c == KB_KEY_PAGE || c == KB_KEY_DONE || c == KB_KEY_CANCEL) return KB_NONE;
        kb->typed = true;  // type now: the lift only releases the key
    } else if (phase == 1) {
        press_move(&kb->press, x, y);
        return KB_NONE;
    } else {
        int k = kb->pressed_key;
        kb->pressed_key = -1;
        if (k < 0 || k >= n) return KB_NONE;
        *dirty = keys[k].r;
        if (kb->typed) return KB_NONE;
        if (key_at(kb, x, y) != k) return KB_NONE;  // slid off the key: no input
    }
    int k = kb->pressed_key >= 0 ? kb->pressed_key : key_at(kb, x, y);
    if (k < 0 || k >= n) return KB_NONE;
    size_t len = strlen(kb->text);
    switch (keys[k].code) {
    case KB_KEY_CANCEL: kb->open = false; *dirty = kb->area; return KB_CANCEL;
    case KB_KEY_DONE: kb->open = false; *dirty = kb->area; return KB_DONE;
    case KB_KEY_SHIFT:
        kb->shift = !kb->shift;
        *dirty = kb->area;
        return KB_CHANGED;
    case KB_KEY_PAGE:
        kb->page = !kb->page;
        *dirty = kb->area;
        return KB_CHANGED;
    case KB_KEY_BKSP:
        // A whole character: back over its continuation bytes to its first byte.
        while (len > 1 && ((unsigned char)kb->text[len - 1] & 0xC0) == 0x80) len--;
        if (len) kb->text[len - 1] = 0;
        break;
    default: type_key(kb, keys[k].code, dirty);
    }
    kb->field_dirty = field_rect(kb->area);
    return KB_CHANGED;
}
