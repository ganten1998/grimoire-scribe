// canvas.h: 8-bit grayscale raster (0 black .. 255 white) with a clip rect.
#pragma once
#include "common.h"

typedef struct {
    uint8_t *px;   // row-major, stride bytes per row
    int w, h, stride;
    Rect clip;     // all drawing is clipped to this AND to the canvas bounds
    bool owned;    // px allocated by canvas_init
} Canvas;

int canvas_init(Canvas *c, int w, int h);   // allocates, fills white, clip = full
void canvas_free(Canvas *c);
void canvas_set_clip(Canvas *c, Rect r);    // intersected with bounds
void canvas_reset_clip(Canvas *c);

void canvas_fill(Canvas *c, Rect r, uint8_t gray);
// Outline of thickness t drawn inside r.
void canvas_rect_outline(Canvas *c, Rect r, int t, uint8_t gray);
// Rounded rect, filled or outlined (thickness t), antialiased corners.
void canvas_round_rect(Canvas *c, Rect r, int radius, uint8_t gray, bool filled, int t);
// Axis-aligned crisp lines (used by paper rules). Thickness grows down/right.
// Rect with 45-degree cut corners (ch px), same filled/outline rules as canvas_round_rect.
void canvas_chamfer_rect(Canvas *c, Rect r, int ch, uint8_t gray, bool filled, int t);
// Filled diamond of "radius" r (center to tip).
void canvas_lozenge(Canvas *c, float cx, float cy, float r, uint8_t gray);
void canvas_hline(Canvas *c, int x0, int x1, int y, int t, uint8_t gray);
void canvas_vline(Canvas *c, int x, int y0, int y1, int t, uint8_t gray);
// Antialiased line with round caps; width in px (may be fractional). Opaque paint (not deposit).
void canvas_line_aa(Canvas *c, float x0, float y0, float x1, float y1, float width, uint8_t gray);
// Antialiased filled disc painted toward gray with alpha in 0..1 (normal "over" blend).
void canvas_disc_aa(Canvas *c, float cx, float cy, float r, uint8_t gray, float alpha);

// Ink deposit: darken-only blend. dst = dst - (dst - ink) * a, only if that is darker.
static inline void canvas_deposit_px(Canvas *c, int x, int y, uint8_t ink, float a) {
    if (x < c->clip.x || y < c->clip.y || x >= c->clip.x + c->clip.w || y >= c->clip.y + c->clip.h) return;
    uint8_t *p = &c->px[y * c->stride + x];
    if (*p <= ink) return;
    int v = (int)(*p - (*p - ink) * a + 0.5f);
    if (v < *p) *p = (uint8_t)v;
}

// Paper restore, the deposit's mirror (erase strokes): lighten-only toward the paper value under
// the pixel, never past it. dst = dst + (paper - dst) * a, only if that is lighter; a >= 0.999
// lands exactly on the paper.
static inline void canvas_restore_px(Canvas *c, int x, int y, uint8_t paper, float a) {
    if (x < c->clip.x || y < c->clip.y || x >= c->clip.x + c->clip.w || y >= c->clip.y + c->clip.h) return;
    uint8_t *p = &c->px[y * c->stride + x];
    if (*p >= paper) return;
    if (a >= 0.999f) {
        *p = paper;
        return;
    }
    int v = (int)(*p + (paper - *p) * a + 0.5f);
    if (v > *p) *p = (uint8_t)(v < paper ? v : paper);
}

// Copy src rect to dst at (dx,dy), clipped by dst clip.
void canvas_blit(Canvas *dst, const Canvas *src, Rect srect, int dx, int dy);
// Box-filter downsample whole src into dst (dst size defines the scale).
void canvas_downsample(Canvas *dst, const Canvas *src);
// Downsample keeping thin dark lines visible: each output pixel blends the box average
// toward the box minimum (weight 0..1). Used for paper previews.
void canvas_downsample_keep_lines(Canvas *dst, const Canvas *src, float weight);
// Scales src into box keeping its aspect (box filter down, nearest up), centered in box.
void canvas_blit_fit(Canvas *dst, const Canvas *src, Rect box);
// Invert a rect (used for pressed/selected feedback).
void canvas_invert(Canvas *c, Rect r);
