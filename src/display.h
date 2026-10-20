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
// Show Grimoire's colors inverted (white on black) when on.
void display_set_invert(bool on);
// Low-latency handwriting mode (MTK pen mode on the Scribe). While on, only REFRESH_FAST
// should be used; any other mode turns it off first. No-op where unsupported.
void display_pen_mode(bool on);
// The screen guard. Both checks read the framebuffer back (8 or 32 bpp) to see whether another
// process drew over Grimoire; cheap. The simulator never reports an overwrite.
// This one samples a band of the screen densely: it catches small intruders such as the Kindle's
// status bar clock. `skip` (may be empty) is excluded, e.g. a visible toast.
bool display_band_overwritten(const Canvas *screen, Rect band, Rect skip);
// This one finds a patch anywhere on screen that another process drew (a 24 px sampling grid, so
// a strip of the Kindle home window is caught even when it covers little of the screen).
// True with *where = the bounding box of the mismatching samples (padded).
bool display_find_overwrite(const Canvas *screen, Rect skip, Rect *where);
// True once the first frame and the next 4 refreshes have all failed: the panel is not Grimoire's
// to drive, and it should leave. A failure after any refresh worked never counts. Polled by the
// main loop: until one refresh works it also refreshes the whole panel again once a second, so a
// dead panel is known within seconds even when nothing else draws. Always false in the simulator.
bool display_broken(void);
// Blocks until pending refreshes have completed (used before exit / screenshots).
void display_wait(void);
void display_close(void);
