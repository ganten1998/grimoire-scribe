// font.h: TrueType text via stb_truetype, fonts embedded at build time
// (build/gen/fonts.c, from assets/*.ttf via xxd -i). Glyph bitmaps are cached per size.
#pragma once
#include "canvas.h"

// What the text is for; the theme picks the typeface. Classic: all Terminess. Gothic: pixel
// faces, Jersey 10 for text and labels (overprinted one design pixel for bold, except where
// that would fill in an accent or turn a letter into another), Jacquard 24 (pixel blackletter)
// for titles. FIXED faces are Terminess in every theme: paper labels.
typedef enum {
    FONT_REGULAR = 0,
    FONT_BOLD = 1,
    FONT_TITLE,
    FONT_CAPS,
    FONT_FIXED,
    FONT_FIXED_BOLD,
    FONT_FACE_COUNT
} FontFace;

int font_init(void);   // 0 on success
void font_shutdown(void);
void font_set_gothic(bool gothic);
bool font_gothic(void);

// Draws UTF-8 text with its baseline at y. Returns advance width in px.
int font_draw(Canvas *c, FontFace f, float px, int x, int baseline_y, const char *utf8, uint8_t gray);
int font_measure(FontFace f, float px, const char *utf8);
// Whether the face (as the current theme maps it) has its own glyph for cp, not Terminess's.
bool font_has_glyph(FontFace f, uint32_t cp);
int font_ascent(FontFace f, float px);   // px above baseline
int font_descent(FontFace f, float px);  // px below baseline (positive)
// Centered in r both ways; text longer than r is truncated with an ellipsis.
void font_draw_centered(Canvas *c, FontFace f, float px, Rect r, const char *utf8, uint8_t gray);
// Draws one glyph (e.g. a Nerd Font icon code point) scaled so its ink box fits r, centered.
void font_draw_icon(Canvas *c, FontFace f, uint32_t codepoint, Rect r, uint8_t gray);
// Left-aligned, vertically centered in r, truncated with ellipsis.
void font_draw_left(Canvas *c, FontFace f, float px, Rect r, const char *utf8, uint8_t gray);
// The same for a name someone gave (a notebook's, a folder's), which "..." shortens by design:
// the overflow hook (below) does not hear of it.
void font_draw_left_name(Canvas *c, FontFace f, float px, Rect r, const char *utf8, uint8_t gray);

// Up to max_lines lines broken at spaces (a word wider than r on its own is broken between its
// characters), each centered in r or left-aligned, in slots line_h px apart (0: the face's own
// height) with the block centered vertically in r. Text that fits on one line is drawn exactly as
// font_draw_centered or font_draw_left draws it. Two lines are balanced: the break that keeps the
// longer line shortest, unless one right after punctuation makes it at most half again as long
// ("Wide, no margin" breaks after its comma). Only text that still does not fit ends in "...".
// Returns the lines drawn.
int font_draw_wrapped(Canvas *c, FontFace f, float px, Rect r, const char *utf8, int max_lines, int line_h,
                      bool center, uint8_t gray);
// How many lines font_draw_wrapped needs for the text in width w (it draws at most its
// max_lines), and the width of the widest of them in *widest (may be NULL).
int font_wrap_lines(FontFace f, float px, const char *utf8, int w, int *widest);
// The largest size from px down to min_px at which the text fits maxw on one line, among the
// sizes the face really has: a pixel face changes only at whole design pixels (Jersey 10 looks the
// same from 27 to 43 px), an outline face in quarter pixels. px if it fits as it is; 0 if even
// the smallest does not.
float font_fit_px(FontFace f, float px, float min_px, const char *utf8, int maxw);
// Whenever text is cut short to fit (font_draw_centered, font_draw_left, font_draw_wrapped), or
// font_draw_wrapped breaks a word between its letters, the hook hears of it: the text as given,
// the width it needs (for wrapped text, its last line's, or the broken word's) and the width it
// has, in px. NULL takes it away. The headless runner's fit check uses it.
typedef void (*FontOverflowHook)(const char *utf8, int need, int have);
void font_set_overflow_hook(FontOverflowHook hook);
