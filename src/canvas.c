// canvas.c: 8-bit gray raster primitives.
#include "canvas.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

int canvas_init(Canvas *c, int w, int h) {
    memset(c, 0, sizeof *c);
    c->px = malloc((size_t)w * (size_t)h);
    if (!c->px) return -1;
    memset(c->px, 255, (size_t)w * (size_t)h);
    c->w = w;
    c->h = h;
    c->stride = w;
    c->clip = (Rect){0, 0, w, h};
    c->owned = true;
    return 0;
}

void canvas_free(Canvas *c) {
    if (c->owned) free(c->px);
    memset(c, 0, sizeof *c);
}

void canvas_set_clip(Canvas *c, Rect r) { c->clip = rect_intersect(r, (Rect){0, 0, c->w, c->h}); }
void canvas_reset_clip(Canvas *c) { c->clip = (Rect){0, 0, c->w, c->h}; }

void canvas_fill(Canvas *c, Rect r, uint8_t gray) {
    r = rect_intersect(r, c->clip);
    for (int y = r.y; y < r.y + r.h; y++) memset(&c->px[y * c->stride + r.x], gray, (size_t)r.w);
}

void canvas_rect_outline(Canvas *c, Rect r, int t, uint8_t gray) {
    if (t * 2 >= r.w || t * 2 >= r.h) {
        canvas_fill(c, r, gray);
        return;
    }
    canvas_fill(c, (Rect){r.x, r.y, r.w, t}, gray);
    canvas_fill(c, (Rect){r.x, r.y + r.h - t, r.w, t}, gray);
    canvas_fill(c, (Rect){r.x, r.y + t, t, r.h - 2 * t}, gray);
    canvas_fill(c, (Rect){r.x + r.w - t, r.y + t, t, r.h - 2 * t}, gray);
}

// "Over" blend of gray at alpha a into one pixel (clip already checked).
static inline void blend_px(uint8_t *p, uint8_t gray, float a) {
    if (a <= 0.f) return;
    if (a >= 1.f) {
        *p = gray;
        return;
    }
    *p = (uint8_t)(*p + (gray - *p) * a + 0.5f);
}

// Coverage of a pixel whose center is at signed distance d from an edge (negative = inside).
static inline float cov(float d) { return clampf(0.5f - d, 0.f, 1.f); }

void canvas_round_rect(Canvas *c, Rect r, int radius, uint8_t gray, bool filled, int t) {
    if (rect_is_empty(r)) return;
    int maxr = (r.w < r.h ? r.w : r.h) / 2;
    if (radius > maxr) radius = maxr;
    if (radius < 0) radius = 0;
    Rect b = rect_intersect(r, c->clip);
    float rr = (float)radius;
    for (int y = b.y; y < b.y + b.h; y++) {
        float py = y + 0.5f;
        for (int x = b.x; x < b.x + b.w; x++) {
            float px = x + 0.5f;
            // Distance to the rounded rect boundary (negative inside).
            float cx = clampf(px, r.x + rr, r.x + r.w - rr);
            float cy = clampf(py, r.y + rr, r.y + r.h - rr);
            float dx = px - cx, dy = py - cy;
            float d;
            if (dx != 0.f || dy != 0.f)
                d = sqrtf(dx * dx + dy * dy) - rr;  // in a corner zone
            else  // core: distance to the nearest straight edge
                d = -fminf(fminf(px - r.x, r.x + r.w - px), fminf(py - r.y, r.y + r.h - py));
            float a = cov(d);
            if (!filled) a = fminf(a, cov(-(d + t)));
            if (a > 0.f) blend_px(&c->px[y * c->stride + x], gray, a);
        }
    }
}

void canvas_hline(Canvas *c, int x0, int x1, int y, int t, uint8_t gray) {
    if (x1 < x0) {
        int s = x0;
        x0 = x1;
        x1 = s;
    }
    canvas_fill(c, (Rect){x0, y, x1 - x0, t}, gray);
}

void canvas_vline(Canvas *c, int x, int y0, int y1, int t, uint8_t gray) {
    if (y1 < y0) {
        int s = y0;
        y0 = y1;
        y1 = s;
    }
    canvas_fill(c, (Rect){x, y0, t, y1 - y0}, gray);
}

void canvas_line_aa(Canvas *c, float x0, float y0, float x1, float y1, float width, uint8_t gray) {
    float hw = width * 0.5f;
    Rect b = rect_intersect(rect_from_points(x0, y0, x1, y1, hw + 1.f), c->clip);
    float dx = x1 - x0, dy = y1 - y0;
    float len2 = dx * dx + dy * dy;
    for (int y = b.y; y < b.y + b.h; y++) {
        float py = y + 0.5f;
        for (int x = b.x; x < b.x + b.w; x++) {
            float px = x + 0.5f;
            float t = len2 > 0.f ? clampf(((px - x0) * dx + (py - y0) * dy) / len2, 0.f, 1.f) : 0.f;
            float ex = px - (x0 + dx * t), ey = py - (y0 + dy * t);
            float a = cov(sqrtf(ex * ex + ey * ey) - hw);
            if (a > 0.f) blend_px(&c->px[y * c->stride + x], gray, a);
        }
    }
}

void canvas_disc_aa(Canvas *c, float cx, float cy, float r, uint8_t gray, float alpha) {
    Rect b = rect_intersect(rect_from_points(cx, cy, cx, cy, r + 1.f), c->clip);
    for (int y = b.y; y < b.y + b.h; y++) {
        float py = y + 0.5f - cy;
        for (int x = b.x; x < b.x + b.w; x++) {
            float px = x + 0.5f - cx;
            float a = cov(sqrtf(px * px + py * py) - r) * alpha;
            if (a > 0.f) blend_px(&c->px[y * c->stride + x], gray, a);
        }
    }
}

void canvas_blit(Canvas *dst, const Canvas *src, Rect s, int dx, int dy) {
    s = rect_intersect(s, (Rect){0, 0, src->w, src->h});
    Rect d = rect_intersect((Rect){dx, dy, s.w, s.h}, dst->clip);
    for (int y = d.y; y < d.y + d.h; y++) {
        int sy = s.y + (y - dy);
        int sx = s.x + (d.x - dx);
        memcpy(&dst->px[y * dst->stride + d.x], &src->px[sy * src->stride + sx], (size_t)d.w);
    }
}

void canvas_downsample(Canvas *dst, const Canvas *src) {
    for (int y = 0; y < dst->h; y++) {
        int sy0 = (int)((int64_t)y * src->h / dst->h), sy1 = (int)((int64_t)(y + 1) * src->h / dst->h);
        if (sy1 <= sy0) sy1 = sy0 + 1;
        for (int x = 0; x < dst->w; x++) {
            int sx0 = (int)((int64_t)x * src->w / dst->w), sx1 = (int)((int64_t)(x + 1) * src->w / dst->w);
            if (sx1 <= sx0) sx1 = sx0 + 1;
            uint32_t sum = 0, n = 0;
            for (int yy = sy0; yy < sy1 && yy < src->h; yy++)
                for (int xx = sx0; xx < sx1 && xx < src->w; xx++) {
                    sum += src->px[yy * src->stride + xx];
                    n++;
                }
            dst->px[y * dst->stride + x] = n ? (uint8_t)((sum + n / 2) / n) : 255;
        }
    }
}

void canvas_downsample_keep_lines(Canvas *dst, const Canvas *src, float weight) {
    for (int y = 0; y < dst->h; y++) {
        int sy0 = (int)((int64_t)y * src->h / dst->h), sy1 = (int)((int64_t)(y + 1) * src->h / dst->h);
        if (sy1 <= sy0) sy1 = sy0 + 1;
        for (int x = 0; x < dst->w; x++) {
            int sx0 = (int)((int64_t)x * src->w / dst->w), sx1 = (int)((int64_t)(x + 1) * src->w / dst->w);
            if (sx1 <= sx0) sx1 = sx0 + 1;
            uint32_t sum = 0, n = 0;
            uint8_t mn = 255;
            for (int yy = sy0; yy < sy1 && yy < src->h; yy++)
                for (int xx = sx0; xx < sx1 && xx < src->w; xx++) {
                    uint8_t v = src->px[yy * src->stride + xx];
                    sum += v;
                    n++;
                    if (v < mn) mn = v;
                }
            float avg = n ? (float)sum / n : 255.f;
            dst->px[y * dst->stride + x] = (uint8_t)lrintf(lerpf(avg, mn, weight));
        }
    }
}

void canvas_invert(Canvas *c, Rect r) {
    r = rect_intersect(r, c->clip);
    for (int y = r.y; y < r.y + r.h; y++) {
        uint8_t *p = &c->px[y * c->stride + r.x];
        for (int x = 0; x < r.w; x++) p[x] = (uint8_t)(255 - p[x]);
    }
}
