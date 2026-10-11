// display.h: e-ink output. Implemented by display_fbink.c (Kindle) and sim_sdl.c.
#pragma once
#include "canvas.h"

typedef enum {
    REFRESH_FAST = 0,  // DU: black/white, fast, for live ink
    REFRESH_UI = 1,    // GL16 partial, no flash
    REFRESH_FULL = 2,  // GC16 full flash
} RefreshMode;

// Fills portrait width/height and dpi of the panel. 0 on success.
int display_open(int *w, int *h, float *dpi);
// Copies the rect of `screen` (same size as the display) to the panel and refreshes it.
void display_present(const Canvas *screen, Rect r, RefreshMode mode);
// Device rotation (quarter turns clockwise): the logical screen the app draws is mapped onto
// the panel accordingly. Fills the logical width/height (swapped for 1 and 3).
void display_set_orientation(int quarter_turns, int *w, int *h);
// Show Quire's colors inverted (white on black) when on.
void display_set_invert(bool on);
// Low-latency handwriting mode (MTK pen mode on the Scribe). While on, only REFRESH_FAST
// should be used; any other mode turns it off first. No-op where unsupported.
void display_pen_mode(bool on);
// True if the panel no longer shows `screen` (another process drew over us). Samples a grid
// of pixels from the framebuffer; cheap. The simulator never reports an overwrite.
bool display_overwritten(const Canvas *screen);
// Blocks until pending refreshes have completed (used before exit / screenshots).
void display_wait(void);
void display_close(void);
