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
// should be used; any other mode turns it off first. No-op where unsupported. The editor turns it
// on for a stroke of ink, which marks live ink too (display_live_ink).
void display_pen_mode(bool on);
// Live ink: a stroke being drawn with the pen (writing, erasing, the pen feel pad, the calibration's
// check line), on at its start and off at its end. On a colour panel every other fast refresh is
// followed by a wait until the driver has read the frame (display_fbink.c); live ink never waits,
// so pen reading never stalls. Changes nothing else (no pen mode).
void display_live_ink(bool on);

// Which fast refreshes are live ink, the rule display_fbink.c follows (the simulator follows it
// too, and reports it to scripts): those sent while a stroke is drawn, and the first one within
// 100 ms of its end (the samples that came with the pen lifting go out just after the stroke ends).
typedef struct {
    bool on;         // a stroke is being drawn
    uint32_t ended;  // when the last one ended (0: none since the last fast refresh)
} LiveInk;
static inline void live_ink_mark(LiveInk *l, bool on, uint32_t now) {
    if (l->on && !on) l->ended = now | 1;
    l->on = on;
}
// For a refresh in `mode` sent at `now`: true when it is a fast refresh of live ink.
static inline bool live_ink_frame(LiveInk *l, RefreshMode mode, uint32_t now) {
    if (mode != REFRESH_FAST) return false;
    bool ink = l->on || (l->ended && (int32_t)(now - l->ended) < 100);
    l->ended = 0;
    return ink;
}
// The screen guard. Both checks read the framebuffer back (8 or 32 bpp) to see whether another
// process drew over Grimoire; cheap. The simulator never reports an overwrite.
// This one samples a band of the screen densely: it catches small intruders such as the Kindle's
// status bar clock. `skip` (may be empty) is excluded, e.g. a visible toast.
bool display_band_overwritten(const Canvas *screen, Rect band, Rect skip);
// This one finds a patch anywhere on screen that another process drew (a 24 px sampling grid, so
// a strip of the Kindle home window is caught even when it covers little of the screen).
// True with *where = the bounding box of the mismatching samples (padded).
bool display_find_overwrite(const Canvas *screen, Rect skip, Rect *where);
// A rect the panel shows over App.screen for good (Live view's chip, app.c): both checks leave it
// out, as they do a toast. RECT_EMPTY for none.
void display_skip_overlay(Rect r);
// True once the first frame and the next 4 refreshes have all failed: the panel is not Grimoire's
// to drive, and it should leave. A failure after any refresh worked never counts. Polled by the
// main loop: until one refresh works it also refreshes the whole panel again once a second, so a
// dead panel is known within seconds even when nothing else draws. Always false in the simulator.
bool display_broken(void);
// Blocks until pending refreshes have completed (used before exit / screenshots).
void display_wait(void);
void display_close(void);
