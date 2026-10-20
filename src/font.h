// font.h: TrueType text via stb_truetype, fonts embedded at build time
// (build/gen/fonts.c, from assets/*.ttf via xxd -i). Glyph bitmaps are cached per size.
#pragma once
#include "canvas.h"

// What the text is for; the theme picks the typeface. Classic: all Terminess. Gothic: pixel
// faces, Jersey 10 for text and labels (overprinted one design pixel for bold, except accents
// the overprint would fill in), Jacquard 24 (pixel blackletter) for titles. FIXED faces are
// Terminess in every theme: paper labels.
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
