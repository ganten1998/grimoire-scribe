// live.h: Live view, a computer's browser watching what Grimoire shows (docs/DESIGN.md, Live view).
//
// While it is on, every rect the main loop presents is copied from App.screen into a shared frame
// (one memcpy of a rect it presents anyway; toasts and the live chip are never in it) and its
// 64 px tiles are marked. A browser that has paired (pair.h) gets a viewer child of its own, forked
// by the share server: it sends the tiles that changed as gray PNGs over one long HTTP response,
// at most about 30 batches a second, and never holds up the pen: a slow browser only makes the
// marks pile up, and the next batch carries what is newest. Two viewers at most. Nothing is
// written to disk.
//
// The stream's frames are little-endian: u32 length (of the type byte and the payload), u8 type,
// payload:
//   1 hello      u16 version (1), w, h, tile; u32 epoch: the screen's size, after a start or a turn
//   2 tile       u16 x, y, w, h, then the PNG of that rect
//   3 batch end  u32 seq, u32 the Kindle's ms: draw what came since the last one
//   4 ping       nothing: sent after 15 s without a batch
//   5 state      u8 LIVE_AWAKE, LIVE_ASLEEP, LIVE_PAIRING or LIVE_BYE
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "common.h"
#include "canvas.h"

#define LIVE_TILE 64
#define LIVE_WORDS 64        // tile marks: 2048 tiles, more than either Scribe's screen has
#define LIVE_VIEWERS 2
#define LIVE_IDLE_OFF_MS (10 * 60 * 1000)  // no viewer for this long: Live view turns itself off

enum { LIVE_AWAKE = 0, LIVE_ASLEEP = 1, LIVE_PAIRING = 2, LIVE_BYE = 3 };
enum { LIVE_HELLO = 1, LIVE_TILE_FRAME = 2, LIVE_BATCH = 3, LIVE_PING = 4, LIVE_STATE = 5 };

// ---- the main loop ----

// Turns it on for a w x h screen: the shared frame, all of it marked. -1 when there is no memory
// (or the screen has more tiles than the marks hold).
int live_start(int w, int h);
// Turns it off: each viewer is told (LIVE_BYE), then stopped and reaped; the frame goes.
void live_stop(void);
bool live_on(void);
int live_viewers(void);
// A rect of the screen presented: copied into the frame, its tiles marked, the viewers woken.
void live_push(const Canvas *screen, Rect r);
// The screen turned (w x h now): a new hello, every tile again.
void live_resize(int w, int h);
// LIVE_AWAKE, LIVE_ASLEEP or LIVE_PAIRING (the pairing card is up: nothing is sent while it is).
void live_set_state(int state);
// Reaps viewers that ended. Returns how many viewers there are now. *idle_off is set when Live
// view has had no viewer for LIVE_IDLE_OFF_MS (the caller turns it off and says so).
int live_poll(uint32_t now, bool *idle_off);

// ---- the share server ----

// Hands the connection fd (an authenticated GET /live/stream) to a new viewer child: true when it
// took it (the caller closes its own copy), false when two are watching already or fork failed.
bool live_take_viewer(int fd);

// ---- pure parts (tests/test_live.c) ----

int live_tiles_across(int w);
// Sets the marks (tiles of a w x h screen, row by row) of every tile r touches.
void live_mark(uint32_t *words, int w, int h, Rect r);
// The marked tiles as rects: each run of marked tiles in a tile row, cut to the screen. Returns
// how many it wrote (at most cap).
int live_runs(const uint32_t *words, int w, int h, Rect *out, int cap);
// A frame's 5-byte header for a payload of len bytes.
void live_frame_header(uint8_t out[5], uint32_t len, uint8_t type);
