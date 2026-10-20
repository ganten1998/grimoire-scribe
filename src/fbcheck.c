// fbcheck.c: framebuffer readback and refresh health (see fbcheck.h).
#include "fbcheck.h"

#include <math.h>
#include <string.h>

uint8_t fbcheck_gray(const uint8_t *px, int bpp, bool flip) {
    unsigned v = bpp == 32 ? ((unsigned)px[0] + px[1] + px[2] + 1) / 3 : px[0];
    return (uint8_t)(flip ? 255 - v : v);
}

bool fbcheck_broken(int attempts, int ok) { return ok == 0 && attempts >= 5; }

float fbcheck_dpi(const char *fbink_name, unsigned fbink_dpi, unsigned xres, unsigned yres, unsigned width_mm,
                  unsigned height_mm, DpiSource *from) {
    bool known = fbink_name && fbink_name[0] && strcmp(fbink_name, "Unidentified!") && strcmp(fbink_name, "Unknown!");
    if (known && fbink_dpi > 0) {
        *from = DPI_FBINK;
        return (float)fbink_dpi;
    }
    // Drivers that do not know their panel report 0 mm (or -1, read as a huge number).
    double d = 0;
    if (xres && yres && width_mm && height_mm && width_mm < 1000 && height_mm < 1000)
        d = hypot((double)xres, (double)yres) * 25.4 / hypot((double)width_mm, (double)height_mm);
    else if (xres && width_mm && width_mm < 1000)
        d = (double)xres * 25.4 / (double)width_mm;
    if (d >= 100 && d <= 600) {
        *from = DPI_PANEL_SIZE;
        return (float)d;
    }
    *from = DPI_DEFAULT;
    return 300.f;
}
