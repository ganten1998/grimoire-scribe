#include "fbpack.h"
#include <string.h>

static const uint8_t k_bayer8[64] = {
    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26,
    12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25,
    15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21
};

void fbpack_row(uint8_t *dst, ptrdiff_t step, uint8_t *previous, ptrdiff_t previous_step,
                const uint8_t *src, int width, int x, int y, int bpp, bool invert, bool fast, bool known) {
    if (width <= 0 || (bpp != 8 && bpp != 32)) return;
    if (!fast && !invert && bpp == 8 && step == 1) {
        memcpy(dst, src, (size_t)width);
        if (previous && previous_step == 1) memcpy(previous, src, (size_t)width);
        else if (previous)
            for (int i = 0; i < width; i++) previous[i * previous_step] = src[i];
        return;
    }
    for (int i = 0; i < width; i++) {
        uint8_t v = src[i];
        uint8_t *p = dst + i * step;
        // DU leaves gray target pixels alone. Keep unchanged gray paper gray, and keep earlier
        // dithered ink in its current state, instead of dithering the whole dirty rectangle.
        if (fast && known && previous && previous[i * previous_step] == v) continue;
        if (previous) previous[i * previous_step] = v;
        if (fast) {
            int threshold = k_bayer8[((y & 7) << 3) | ((x + i) & 7)] * 4 + 2;
            if (known && previous) {
                // A DU transition ends in black or white, and a pixel cannot start another until
                // it has. So move what the panel shows only toward the new value: black or not
                // yet when it darkens, white or not yet when it lightens, as often as keeps the
                // average. Ink on gray paper then goes straight to black, instead of white first
                // and black a whole waveform later. Over black and white this is the dither below.
                int g = invert ? 255 - p[0] : p[0];
                if (v < g) {
                    if (v * 255 > threshold * g) continue;
                    v = 0;
                } else if (v > g) {
                    if ((v - g) * 255 <= threshold * (255 - g)) continue;
                    v = 255;
                } else {
                    continue;
                }
            } else if (v != 0 && v != 255) {
                v = v > threshold ? 255 : 0;
            }
        }
        if (invert) v = (uint8_t)(255 - v);
        p[0] = v;
        if (bpp == 32) { p[1] = p[2] = v; p[3] = 255; }
    }
}

Rect fbpack_pen_region(Rect r, int width, int height) {
    if (width <= 0 || height <= 0) return RECT_EMPTY;
    r = rect_intersect(r, (Rect){0, 0, width, height});
    if (rect_is_empty(r) || (int64_t)r.w * r.h >= 6400) return r;
    int w = r.h >= 80 ? (6400 + r.h - 1) / r.h : 80;
    w = clampi(w, r.w, width);
    int h = clampi((6400 + w - 1) / w, r.h, height);
    // Narrow panels may need more width after the height reaches their edge.
    if ((int64_t)w * h < 6400) w = clampi((6400 + h - 1) / h, w, width);
    int x = clampi(r.x - (w - r.w) / 2, 0, width - w);
    int y = clampi(r.y - (h - r.h) / 2, 0, height - h);
    return (Rect){x, y, w, h};
}
