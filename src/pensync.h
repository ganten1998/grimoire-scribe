// pensync.h: the pure parts of reading the pen (input_evdev.c), unit-tested on the host: how a
// stroke goes on after the kernel dropped pen events, the kernel's sample times in now_ms()'s clock,
// which of the raw digitizer's samples a pen frame takes its pressure and tilt from, and where the pen
// hovers (from the raw digitizer: the scaled pen says only that it is near).
//
// Each reader of an evdev device has its own buffer in the kernel. Grimoire's for the scaled pen
// (stylus-custom) holds 128 events, and its frames carry only X and Y (2 or 3 events each), so it
// holds 42-63 frames: 120-175 ms of writing. When Grimoire's thread stalls longer than that, the
// kernel throws away everything unread and queues SYN_DROPPED and the newest event. What the next
// read finds is a partial frame (the rest of the frame that newest event belongs to), then every
// frame that came after it, in order: the tail. Buttons and axes the tail does not carry stayed as
// they were, and only the device can say what that is (EVIOCGKEY, EVIOCGABS: what it holds now).
// EVIOCGKEY also takes the queued button events out of the reader's queue, so it is asked only
// once the tail has been read.
#pragma once
#include <stdbool.h>
#include <stdint.h>

// Linux evdev numbers (linux/input.h, which the host does not have; input_evdev.c checks them).
enum {
    PS_EV_SYN = 0x00, PS_EV_KEY = 0x01, PS_EV_ABS = 0x03,
    PS_SYN_REPORT = 0, PS_SYN_DROPPED = 3,
    PS_ABS_X = 0x00, PS_ABS_Y = 0x01, PS_ABS_PRESSURE = 0x18, PS_ABS_TILT_X = 0x1a, PS_ABS_TILT_Y = 0x1b,
    PS_BTN_TOOL_PEN = 0x140, PS_BTN_TOOL_RUBBER = 0x141, PS_BTN_TOUCH = 0x14a, PS_BTN_STYLUS = 0x14b,
};

// One event as read (input_event's fields; the time in microseconds of the reader's clock).
typedef struct {
    uint64_t t_us;
    uint16_t type, code;
    int32_t value;
} PenEv;

// The pen's buttons and axes as its frames report them (ABS_MT_PRESSURE already scaled into p).
typedef struct {
    bool tool_pen, tool_rubber, touching, side;
    int x, y, p, tx, ty;
} PenState;

// PenState's fields as bits: which ones a query of the device read.
enum {
    PS_F_TOOL_PEN = 1 << 0, PS_F_TOOL_RUBBER = 1 << 1, PS_F_TOUCH = 1 << 2, PS_F_SIDE = 1 << 3,
    PS_F_X = 1 << 4, PS_F_Y = 1 << 5, PS_F_P = 1 << 6, PS_F_TX = 1 << 7, PS_F_TY = 1 << 8,
    PS_F_KEYS = PS_F_TOOL_PEN | PS_F_TOOL_RUBBER | PS_F_TOUCH | PS_F_SIDE,
    PS_F_ALL = (1 << 9) - 1,
};

// Whether the pen touches the screen: BTN_TOUCH, or for a pen without it, pressure above p_min
// while it is near (BTN_TOOL_PEN or BTN_TOOL_RUBBER).
bool pensync_contact(const PenState *s, bool touch_btn, int p_min);

// What a resync starts from.
typedef struct {
    PenState good;          // the pen at the last complete frame read before the drop
    const PenEv *ev;        // what was read from the SYN_DROPPED on (it included), in order
    int n;
    PenState now;           // the device's state, asked once all of ev was read
    unsigned now_ok;        // PS_F_* of now that the device answered (0: the ioctls failed)
    bool touch_btn;         // pensync_contact's
    int p_min;
} PenTail;

// A frame to report, as a SYN_REPORT would have it.
typedef struct {
    PenState s;
    uint64_t t_us;          // its SYN_REPORT's time
    bool moved;             // it carried an axis: a move (or hover) to report
    bool gap;               // the first after the lost stretch: joined straight from the last good point
} PenFrame;

typedef struct {
    int frames;             // complete frames in the tail
    int skipped;            // events of the partial frame left out (its SYN_REPORT not counted)
    int used;               // ev[used..n) is an unfinished last frame, for the caller to read on as usual
    uint64_t first_us, last_us;  // the first and last tail frames' times (0 with no tail)
    bool keys;              // the tail carried a button event
    bool lifted;            // the pen was found lifted: a lift at the last good point went first
    int x, y;               // where the reported frames end
} PenTailInfo;

// The frames to report after a SYN_DROPPED, in order, into out (cap n + 3 is always enough); their
// count. The tail's frames, each as it was: a button before its first event in the tail was the
// other way (the input core passes only changes), an axis before its first event is taken at that
// first value (the nearest known), and what the tail never carries is as the device says now (else
// as it was). Never the device's present position ahead of older frames. The first frame is the
// gap; if the pen touched at the last good point and the tail shows that stroke ended (no contact
// at the first frame or just before it, or the other end by then), a lift at the last good point
// goes before it, and the first frame starts a stroke of its own. A button that changed after the
// tail was read (its event taken by EVIOCGKEY) comes last, at the last frame's place. With no frame
// after the partial one, one frame of the device's present state.
int pensync_tail(const PenTail *t, PenFrame *out, int cap, PenTailInfo *info);

// A sample's time for the stroke (InEvent.k_ms): the kernel's time of its frame in now_ms()'s
// uint32 milliseconds, when the reader's clock is CLOCK_MONOTONIC (mono: EVIOCSCLOCKID took). Else,
// or for a time of 0, one after now, or one over PENSYNC_K_MAX_AGE_MS before now (none of which a
// real CLOCK_MONOTONIC stamp can be), now: when it was read.
#define PENSYNC_K_MAX_AGE_MS 10000u
uint32_t pensync_ms(uint64_t t_us, bool mono, uint32_t now);

// The pen Grimoire follows (stylus-custom) carries position and buttons only; pressure and tilt come
// from the raw digitizer (WacomDigitizer), which the Kindle's pen service relays it from, a frame
// for a frame, about 0.1 ms later. Both are read in turns, the raw one first, and a read takes
// everything queued: after a wait of 100 ms that is some 30 frames of each. Each pen frame used to
// take the newest raw values, so such a stretch of a stroke was drawn with one flat pressure and
// tilt. The raw frames are kept instead (about 0.8 s of them), and each pen frame takes the raw
// frame of its moment: the newest one stamped at or before it, by the kernel's times (both nodes'
// readers are switched to CLOCK_MONOTONIC).
#define PENSYNC_RAW_N 256
typedef struct {
    uint64_t t_us;  // the raw frame's SYN_REPORT time (or when a resync asked)
    int p, tx, ty;
} PenRaw;
typedef struct {
    PenRaw r[PENSYNC_RAW_N];
    int next, count;
} PenRawRing;

// Forgets every frame kept: the raw digitizer dropped events (what is kept no longer meets what
// comes next), or the pens went to the native PIN screen and back.
void pensync_raw_clear(PenRawRing *g);
// A raw frame read (pressure and tilt as the digitizer reports them), at its kernel time. Frames come
// in time order: a resync after a drop pushes what it asked at the time it asked.
void pensync_raw_push(PenRawRing *g, uint64_t t_us, int p, int tx, int ty);
// The raw frame a pen frame of time t_us goes with: the newest kept at or before t_us. False when
// none is that old (nothing kept, a drop forgot them, or it is older than the oldest kept): the
// caller takes the raw digitizer's newest values, as before there was a ring.
bool pensync_raw_at(const PenRawRing *g, uint64_t t_us, PenRaw *out);

// The pen service relays the pen coming and going, and where it touches, but not where it hovers
// (proximity only: no X or Y between BTN_TOUCH 0 and BTN_TOOL_PEN 0, in the raw pen log of
// 2026-10-26). The raw digitizer streams its hover frames, so Live view's hovering pen is taken from
// it, in the scaled pen's units. The pen service scales each raw axis linearly onto the screen
// (15624 / 1860 = 20832 / 2480 = 8.4 on the Scribe 2, within 3 px), each maybe reversed. Each axis
// is taken as it is (as Grimoire's raw-only fallback and KOReader take the raw axes) until contact
// frames, where both pens report the same moment, say otherwise; and not at all when even the best
// orientation is far from the scaled pen, since a ring in the wrong place is worse than none.
#define PENSYNC_HOVER_FRAMES 32  // contact frames compared before they decide
#define PENSYNC_HOVER_TOL 8      // px: the best orientation's mean distance from the scaled pen, at most
#define PENSYNC_HOVER_CAP 64     // px: one frame's distance counts at most this (a stray frame weighs little)
typedef struct {
    int rmin[2], rmax[2];  // the raw digitizer's x and y ranges
    int smin[2], smax[2];  // the scaled pen's
    uint32_t err[4];       // per orientation (bit 0: x reversed, bit 1: y reversed): distances summed
    uint32_t n;            // contact frames compared (halved with err past 4096, so it keeps up)
} PenHover;

void pensync_hover_init(PenHover *h, const int rmin[2], const int rmax[2], const int smin[2], const int smax[2]);
// A contact frame: the raw digitizer's place (rx, ry) and the scaled pen's (sx, sy) at the same moment.
void pensync_hover_learn(PenHover *h, int rx, int ry, int sx, int sy);
// The orientation in use: 0 (both axes as they are) until PENSYNC_HOVER_FRAMES frames are compared,
// then the nearest of the four; -1 when its mean distance is over PENSYNC_HOVER_TOL px (no hover).
int pensync_hover_orientation(const PenHover *h);
// Orientation o's mean distance from the scaled pen so far, in px (0 before any frame).
float pensync_hover_mean(const PenHover *h, int o);
// A raw place in the scaled pen's units, under the orientation in use; false when there is none.
bool pensync_hover_place(const PenHover *h, int rx, int ry, float *sx, float *sy);
