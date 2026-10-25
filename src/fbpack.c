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
        // DU leaves gray target pixels alone. Keep unchanged gray paper gray, and keep earlier
        // dithered ink in its current state, instead of dithering the whole dirty rectangle.
        if (fast && known && previous && previous[i * previous_step] == v) continue;
        if (previous) previous[i * previous_step] = v;
        if (fast && v != 0 && v != 255) {
            int threshold = k_bayer8[((y & 7) << 3) | ((x + i) & 7)] * 4 + 2;
            v = v > threshold ? 255 : 0;
        }
        if (invert) v = (uint8_t)(255 - v);
        uint8_t *p = dst + i * step;
        p[0] = v;
        if (bpp == 32) { p[1] = p[2] = v; p[3] = 255; }
    }
}
