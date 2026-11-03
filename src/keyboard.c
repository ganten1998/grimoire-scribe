// keyboard.c: on-screen keyboard modal. Layout is computed from the area it was drawn in,
// so hit testing and drawing always agree. keyboard_draw paints the whole modal (cheap);
// keyboard_pointer reports only the rect that needs refreshing. The text is UTF-8 throughout:
// keys carry code points, a key appends a whole character and backspace removes one.
#include "keyboard.h"

#include <math.h>
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

// What each kana key types, in the order a finger swipes it: tap, left, up, right, down (0 where
// the row has no such kana). あ is the gojuon's first row, わ its last; the last row is the
// punctuation key's. A phone's kana pad puts や with its brackets on the sides and ゆ above, よ
// below; 、。？！ are on the punctuation key's flicks.
static const uint16_t k_kana[11][5] = {
    {0x3042, 0x3044, 0x3046, 0x3048, 0x304A},  // あ い う え お
    {0x304B, 0x304D, 0x304F, 0x3051, 0x3053},  // か き く け こ
    {0x3055, 0x3057, 0x3059, 0x305B, 0x305D},  // さ し す せ そ
    {0x305F, 0x3061, 0x3064, 0x3066, 0x3068},  // た ち つ て と
    {0x306A, 0x306B, 0x306C, 0x306D, 0x306E},  // な に ぬ ね の
    {0x306F, 0x3072, 0x3075, 0x3078, 0x307B},  // は ひ ふ へ ほ
    {0x307E, 0x307F, 0x3080, 0x3081, 0x3082},  // ま み む め も
    {0x3084, 0x300C, 0x3086, 0x300D, 0x3088},  // や 「 ゆ 」 よ
    {0x3089, 0x308A, 0x308B, 0x308C, 0x308D},  // ら り る れ ろ
    {0x308F, 0x3092, 0x3093, 0, 0},              // わ を ん
    {0x3001, 0x3002, 0xFF1F, 0xFF01, 0x30FB},  // 、 。 ？ ！ ・
};

// The kana pad's eleventh and twelfth cells: the modifier and ー, after the ten kana rows.
#define K_KANA_MOD_CELL 10
#define KANA_CHOON 0x30FCu  // ー, the long vowel mark

static bool is_flick(uint32_t code) { return code >= KB_KEY_FLICK && code < KB_KEY_FLICK + 11; }

// The voiced kana of one that has a voiced form (か to が, う to ゔ, つ to づ, は to ば); anything
// else comes back as it is.
static uint32_t kana_dakuten(uint32_t cp) {
    switch (cp) {
    case 0x3046: return 0x3094;                                       // う to ゔ
    case 0x304B: case 0x304D: case 0x304F: case 0x3051: case 0x3053:  // か row
    case 0x3055: case 0x3057: case 0x3059: case 0x305B: case 0x305D:  // さ row
    case 0x305F: case 0x3061: case 0x3064: case 0x3066: case 0x3068:  // た row
    case 0x306F: case 0x3072: case 0x3075: case 0x3078: case 0x307B:  // は row
        return cp + 1;
    default: return cp;
    }
}

// The half-voiced kana of one that has that form (は to ぱ); anything else comes back as it is.
static uint32_t kana_handakuten(uint32_t cp) {
    switch (cp) {
    case 0x306F: case 0x3072: case 0x3075: case 0x3078: case 0x307B: return cp + 2;  // は row to ぱ row
    default: return cp;
    }
}

static uint32_t kana_small(uint32_t cp);  // (below: the cycle asks it what a small kana's base is)

// The unvoiced kana a voiced or half-voiced one comes from (が to か, ぱ to は, ゔ to う), or cp when
// it has neither form: the kana a cycle starts from.
static uint32_t kana_unvoice(uint32_t cp) {
    if (cp == 0x3094) return 0x3046;          // ゔ
    if (kana_small(cp + 1) == cp) return cp + 1;  // っ to つ, ゃ to や, ぁ to あ
    for (uint32_t back = 1; back <= 2; back++) {
        if (cp > back) {
            uint32_t b = cp - back;
            if (kana_dakuten(b) == cp || kana_handakuten(b) == cp) return b;
        }
    }
    return cp;
}

// The small form of a kana that has one (あ to ぁ, つ to っ, や to ゃ, わ to ゎ); anything else
// comes back as it is.
static uint32_t kana_small(uint32_t cp) {
    switch (cp) {
    case 0x3042: case 0x3044: case 0x3046: case 0x3048: case 0x304A:  // あ い う え お
    case 0x3064:                                                     // つ
    case 0x3084: case 0x3086: case 0x3088:                           // や ゆ よ
    case 0x308F: return cp - 1;                                      // わ
    default: return cp;
    }
}

// The katakana of a hiragana, and anything else (ー, 、, Latin) as it is: the kana page in katakana
// types the same rows as katakana.
static uint32_t kana_katakana(uint32_t cp) {
    return cp >= 0x3041 && cp <= 0x3096 ? cp + 0x60 : cp;
}

// The next form of a kana, for the pad's one modifier key: its unvoiced form, then voiced, then
// half-voiced, then small, then round again (か が か; は ば ぱ は; つ づ っ つ; や ゃ や). ー and
// the punctuation stay as they are; katakana cycles in its own script.
static uint32_t kana_cycle(uint32_t cp) {
    bool kata = cp >= 0x30A1 && cp <= 0x30F6;
    uint32_t h = kata ? cp - 0x60 : cp, base = kana_unvoice(h);
    uint32_t forms[4] = {base, 0, 0, 0};
    int n = 1;
    const uint32_t more[3] = {kana_dakuten(base), kana_handakuten(base), kana_small(base)};
    for (int i = 0; i < 3; i++) {
        bool seen = false;
        for (int j = 0; j < n && !seen; j++) seen = forms[j] == more[i];
        if (more[i] != base && !seen) forms[n++] = more[i];
    }
    uint32_t next = h;
    for (int i = 0; i < n; i++)
        if (forms[i] == h) next = forms[(i + 1) % n];
    return kata ? next + 0x60 : next;
}

// Which way a finger was swiped from where it went down: 0 for none (a tap, or a wobble no bigger
// than slop), 1 left, 2 up, 3 right, 4 down.
static int flick_dir(float x0, float y0, float x, float y) {
    float dx = x - x0, dy = y - y0, slop = (float)ui_px(40);
    if (fabsf(dx) < slop && fabsf(dy) < slop) return 0;
    if (fabsf(dx) >= fabsf(dy)) return dx < 0.f ? 1 : 3;
    return dy < 0.f ? 2 : 4;
}

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

// Replaces the last character of the text with f(that character): the kana page's modifier takes
// it to its next form. No character, or one with no such form, leaves the text as it is.
static void change_last(Keyboard *kb, uint32_t (*f)(uint32_t)) {
    size_t start = strlen(kb->text);
    while (start > 0 && ((unsigned char)kb->text[start - 1] & 0xC0) == 0x80) start--;
    if (!start) return;
    start--;
    uint32_t cp;
    if (!utf8_get(kb->text + start, &cp)) return;
    uint32_t ch = f(cp);
    if (ch == cp) return;
    char enc[4];
    int n = utf8_put(ch, enc);
    if (start + (size_t)n < sizeof kb->text) {
        memcpy(kb->text + start, enc, (size_t)n);
        kb->text[start + (size_t)n] = 0;
    }
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
    if (page == 2) {
        int n = 0, gap = ui_px(16), h = ui_px(150), w = (area.w - ui_px(60) - 2 * gap) / 3;
        int x = area.x + ui_px(30), y = area.y + ui_px(170);
        for (int i = 0; i < 9; i++)
            keys[n++] = (Key){{x + (i % 3) * (w + gap), y + (i / 3) * (h + gap), w, h}, '1' + i};
        y += 3 * (h + gap);
        keys[n++] = (Key){{x, y, w, h}, KB_KEY_CANCEL};
        keys[n++] = (Key){{x + w + gap, y, w, h}, '0'};
        keys[n++] = (Key){{x + 2 * (w + gap), y, w, h}, KB_KEY_BKSP};
        keys[n++] = (Key){{x, y + h + gap, area.w - ui_px(60), h}, KB_KEY_DONE};
        return n;
    }
    if (page == 4) {
        // The kana pad: three columns of four keys, あ か さ / た な は / ま や ら / わ ゛小 ー, then
        // cancel, カ, 、, abc, space, delete and done.
        int n = 0, kh = ui_px(150), gap = ui_px(10);
        int top = area.y + ui_px(170);
        int inner_x = area.x + ui_px(30), inner_w = area.w - ui_px(60);
        int kw = (inner_w - 2 * gap) / 3;
        for (int i = 0; i < 12; i++) {
            uint32_t code = i < K_KANA_MOD_CELL ? (uint32_t)(KB_KEY_FLICK + i) : i == K_KANA_MOD_CELL ? (uint32_t)KB_KEY_KANA_MOD : KANA_CHOON;
            keys[n++] = (Key){{inner_x + (i % 3) * (kw + gap), top + (i / 3) * (kh + gap), kw, kh}, code};
        }
        // (the bottom row's keys are the letters page's sizes, one tenth of the width each, as
        // the grid's three columns are far wider)
        int sk = (inner_w - 9 * gap) / 10, wide = sk + sk / 2;
        int y = top + 4 * (kh + gap);
        keys[n++] = (Key){{inner_x, y, wide, kh}, KB_KEY_CANCEL};
        int x = inner_x + wide + gap;
        const uint32_t small_keys[3] = {KB_KEY_KATA, KB_KEY_PUNCT, KB_KEY_PAGE};
        for (int i = 0; i < 3; i++) {
            keys[n++] = (Key){{x, y, sk, kh}, small_keys[i]};
            x += sk + gap;
        }
        int done_x = inner_x + inner_w - wide, bx = done_x - sk - gap;
        keys[n++] = (Key){{x, y, bx - gap - x, kh}, ' '};
        keys[n++] = (Key){{bx, y, sk, kh}, KB_KEY_BKSP};
        keys[n++] = (Key){{done_x, y, wide, kh}, KB_KEY_DONE};
        return n;
    }
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
    // In Japanese the accents key is the kana key (the accents page has nothing a Japanese name
    // needs); every other language keeps áé there.
    bool kana = !page && !strcmp(i18n_current(), "ja");
    keys[n++] = (Key){{px, y, wide, kh}, kana ? KB_KEY_KANA : KB_KEY_PAGE};
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
    // TRANSLATORS: the kana page's named keys, in bold at 44 px: ゛小 (one tenth of the modal
    // wide), カ, 、, the space bar, and あ, the letters page's way to the kana page (as wide as
    // shift).
    case KB_KEY_KANA: return "\xe3\x81\x82";          // あ
    case KB_KEY_KANA_MOD: return "\xe3\x82\x9b\xe5\xb0\x8f";  // ゛小: the kana typed last, in its next form
    case KB_KEY_KATA: return kb->kata ? "\xe3\x81\x82" : "\xe3\x82\xab";  // あ back to hiragana, カ to katakana
    case ' ': return C_("key", "space");
    default: {
        // A kana key shows the kana the swipe would type (its first while the finger is still on
        // it).
        uint32_t cp = code;
        if (is_flick(code)) {
            const uint16_t *row = k_kana[code - KB_KEY_FLICK];
            int at = kb->flick > 0 ? kb->flick : 0;
            cp = row[at] ? row[at] : row[0];
            if (kb->kata) cp = kana_katakana(cp);
        }
        ch[utf8_put(kb->shift ? keyboard_upper(cp) : cp, ch)] = 0;
        return ch;
    }
    }
}

static unsigned g_draws;  // keyboard_draws_take

unsigned keyboard_draws_take(void) {
    unsigned n = g_draws;
    g_draws = 0;
    return n;
}

Rect keyboard_draw(Keyboard *kb, Canvas *c, Rect screen) {
    g_draws++;
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
        kb->flick = 0;
        // (a kana key waits for the lift: which kana it types is the swipe, and the modifier and
        // カ act on the character before them, or on the page)
        if (c == KB_KEY_SHIFT || c == KB_KEY_PAGE || c == KB_KEY_KANA || c == KB_KEY_DONE ||
            c == KB_KEY_CANCEL || c == KB_KEY_KANA_MOD || c == KB_KEY_KATA || is_flick(c))
            return KB_NONE;
        kb->typed = true;  // type now: the lift only releases the key
    } else if (phase == 1) {
        press_move(&kb->press, x, y);
        // A swipe on a kana key shows the kana it would type there, once for each way it turns.
        int k = kb->pressed_key;
        if (k >= 0 && k < n && is_flick(keys[k].code)) {
            int dir = flick_dir(kb->press.x0, kb->press.y0, x, y);
            if (dir != kb->flick) {
                kb->flick = dir;
                *dirty = keys[k].r;
            }
        }
        return KB_NONE;
    } else {
        int k = kb->pressed_key;
        kb->pressed_key = -1;
        if (k < 0 || k >= n) {
            kb->flick = 0;
            return KB_NONE;
        }
        if (is_flick(keys[k].code)) {
            // The kana the swipe picked: the pen may end well off the key by the lift.
            const uint16_t *row = k_kana[keys[k].code - KB_KEY_FLICK];
            int at = kb->flick > 0 ? kb->flick : 0;
            uint32_t kana = row[at] ? row[at] : row[0];
            if (kb->kata) kana = kana_katakana(kana);
            kb->flick = 0;
            *dirty = keys[k].r;
            type_key(kb, kana, dirty);
            kb->field_dirty = field_rect(kb->area);
            return KB_CHANGED;
        }
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
    case KB_KEY_KANA:
        // abc is the way back from the kana page; áé (or あ) goes to its own page.
        if (keys[k].code == KB_KEY_KANA)
            kb->page = kb->page == 4 ? 0 : 4;
        else
            kb->page = kb->page ? 0 : 1;
        *dirty = kb->area;
        return KB_CHANGED;
    case KB_KEY_KANA_MOD: change_last(kb, kana_cycle); break;
    case KB_KEY_KATA:
        kb->kata = !kb->kata;
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
