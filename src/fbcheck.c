// fbcheck.c: framebuffer readback and refresh health (see fbcheck.h).
#include "fbcheck.h"

uint8_t fbcheck_gray(const uint8_t *px, int bpp, bool flip) {
    unsigned v = bpp == 32 ? ((unsigned)px[0] + px[1] + px[2] + 1) / 3 : px[0];
    return (uint8_t)(flip ? 255 - v : v);
}

bool fbcheck_broken(int attempts, int ok) { return ok == 0 && attempts >= 5; }
