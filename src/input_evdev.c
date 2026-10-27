// input_evdev.c: Kindle input from evdev.
//
// Devices are classified by capability, not by path:
//   pen   = EV_ABS ABS_X + ABS_PRESSURE and EV_KEY BTN_TOOL_PEN. The Scribe 2 has two:
//           "stylus-custom" (event5), a virtual pen fed by Amazon's pen service, scaled to the
//           screen and aligned with touch, but it only reports position + buttons; and
//           "WacomDigitizer" (event3), the raw EMR digitizer (0..15624 x 0..20832) with pressure
//           0..4095 and tilt in 1/100 degree (seen on device 2026-10-10). Grimoire takes position
//           and buttons from the scaled pen (grabbed) and pressure + tilt from the raw one (read
//           without a grab, so the pen service keeps feeding the scaled pen).
//   touch = EV_ABS ABS_MT_POSITION_X (the capacitive panel, usually /dev/input/touch)
//   power = EV_KEY KEY_POWER on a device with few keys, never Amazon's every-key proxy
//           "kindle-input-pt" (firmware 5.20+); the Scribe 2's is "bd71828-pwrkey" (hidkeys_power)
// Pen and touch are EVIOCGRAB'ed so the (paused) framework never sees them; the power key is
// only watched, so the system keeps handling it if we die. Its press and release are both read:
// holding it INPUT_POWER_HOLD_MS (4 s) is Grimoire's guaranteed way out.
//
// Pen events follow the Wacom tool protocol (BTN_TOOL_PEN / BTN_TOOL_RUBBER bracket the
// proximity, BTN_TOUCH marks contact, pressure/tilt arrive as EV_ABS, SYN_REPORT ends a frame),
// the same model KOReader uses on the Scribe (input.wacom_protocol).
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <math.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <sys/prctl.h>

#include "calib_fit.h"
#include "input.h"
#include "latency.h"
#include "power.h"
#include "shortcuts.h"
#include "hidkeys.h"

#define BITS_PER_LONG (sizeof(long) * 8)
#define NLONGS(x) (((x) + BITS_PER_LONG - 1) / BITS_PER_LONG)
static bool test_bit(const unsigned long *a, int bit) { return (a[bit / BITS_PER_LONG] >> (bit % BITS_PER_LONG)) & 1UL; }

typedef struct {
    int fd;
    struct input_absinfo ax, ay, ap, atx, aty, amp;  // amp: ABS_MT_PRESSURE, if any
    bool has_touch_btn;
} Dev;

static int g_raw_logged;          // raw pen events written to the log (bring-up)
static int g_contact_frames;      // pen frames in contact so far
static bool g_pressure_seen;      // any nonzero pressure ever reported
static bool g_pressure_missing;   // contact without pressure for a while: use a fixed value
static bool g_tilt_flip;          // grimoire/tilt-flip: the digitizer's tilt axes are reversed

static Dev g_pen = {.fd = -1}, g_touch = {.fd = -1};
static int g_orient_fd = -1;  // accelerometer orientation codes (bma4xy_feature on the Scribe 2)
// Power state from the Kindle's power service (the events KOReader uses for its screensaver).
// A child shell loops `lipc-wait-event` in one-shot mode: each run prints a single line and
// exits, which flushes it. (The continuous -m mode looked block-buffered into a pipe on device
// 2026-10-10, but the launcher had preventScreenSaver 1 set that day, which alone stops every
// sleep; KOReader reads `-m -s 0` through a pipe. An empty /mnt/us/grimoire/watcher-multi runs it
// that way, for a device test: see powerd_start.)
static int g_powerd_fd = -1;
static pid_t g_powerd_pid = -1;
static char g_powerd_buf[512];
static size_t g_powerd_len;
static int g_powerd_noise;    // watcher lines that are not events (failures, errors), for the log's sake
static int g_rot;             // logical screen rotation, quarter turns clockwise
static Dev g_wacom = {.fd = -1};  // raw digitizer used only for pressure + tilt (see top)
static struct {
    int p, tx, ty;
    int x, y;  // position, only for timing the pen service's relay (latency.h)
    bool seen;
    bool rubber, side;  // the raw digitizer also reports the eraser end and the side button
} W;
// Ink timing (latency.h): both pens' events stamped in CLOCK_MONOTONIC, the raw samples kept to
// match the relayed ones, and the samples emitted since display_fbink.c last asked.
static bool g_ink_mono;
static LatMatch g_match;
static uint64_t g_pen_frame_us;  // kernel time of the relayed pen frame being read
static uint64_t g_ink_newest, g_ink_oldest, g_ink_raw;
static bool g_input_grabbed = true;
static bool g_initial_grab_failed;
static bool g_pen_resume_block;
static bool g_side_reported;  // last side-button state sent as IN_PEN_BUTTON
static int g_power_fd = -1;
static PowerKey g_pkey;       // the press being followed (release, or the 4 s hold)
static bool g_power_mono;     // the power key's events are stamped in now_ms()'s clock (EVIOCSCLOCKID)
static int g_watch = -1;
static int g_sigpipe[2] = {-1, -1};
static volatile sig_atomic_t g_quit;
static int g_w, g_h;
static InputCalib g_pen_cal, g_touch_cal;
static InputCaps g_caps;

// ---- event queue ----

#define QN 4096
static InEvent g_q[QN];
static int g_qh, g_qt;
static int g_logged_pen, g_logged_touch;
static void q_push(InEvent e) {
    // PIN touches belong only to Amazon. Do not even log their coordinates.
    if (!g_input_grabbed && (power_asleep_drops(&e) || e.type == IN_PEN_HOVER || e.type == IN_PEN_LEAVE || e.type == IN_PEN_BUTTON)) return;
    // Device bring-up: the first events of each kind go to the log.
    bool pen = e.type >= IN_PEN_DOWN && e.type <= IN_PEN_LEAVE;
    bool touch = e.type >= IN_TOUCH_DOWN && e.type <= IN_TOUCH_UP;
    if ((pen && e.type != IN_PEN_HOVER && g_logged_pen++ < 120) || (touch && g_logged_touch++ < 120))
        qlog("input: ev %d at %.0f,%.0f (raw %.3f,%.3f) p %.2f tilt %.0f,%.0f eraser %d side %d", e.type, e.x, e.y, e.nx,
             e.ny, e.pressure, e.tilt_x, e.tilt_y, e.eraser, e.side_button);
    int n = (g_qt + 1) % QN;
    if (n == g_qh) return;  // full: drop (the app drains every loop, so this is pathological)
    g_q[g_qt] = e;
    g_qt = n;
}
static bool q_pop(InEvent *e) {
    if (g_qh == g_qt) return false;
    *e = g_q[g_qh];
    g_qh = (g_qh + 1) % QN;
    return true;
}

// ---- pen state ----

static struct {
    bool tool_pen, tool_rubber;  // tracked separately: a flip sends PEN 0 / RUBBER 1 in either order
    bool prox, eraser, touching, side;
    bool was_prox, was_touching;
    int x, y, p, tx, ty;
    bool dirty;
} P;
static uint32_t g_pen_left_ms;

// ---- touch state ----

#define MAX_SLOTS 16
static struct {
    int slot;
    int id[MAX_SLOTS];
    int x[MAX_SLOTS], y[MAX_SLOTS];
    int tool[MAX_SLOTS];         // ABS_MT_TOOL_TYPE: 0 finger, 1 pen, 2 palm (MT_TOOL_PALM)
    uint32_t down_at[MAX_SLOTS]; // when the contact landed
    int main;          // slot we follow, -1 none
    bool active;       // main finger down (as reported to the app)
    bool suppressed;   // palm: ignore until all fingers lift
    bool suppressed_pinch;  // a late second contact: no pinch until all fingers lift
    float last_x, last_y;
} T = {.main = -1};

static float norm(int v, const struct input_absinfo *a) {
    int span = a->maximum - a->minimum;
    return span > 0 ? clampf((float)(v - a->minimum) / (float)span, 0.f, 1.f) : 0.f;
}

// Tilt in degrees. Linux reports ABS_TILT resolution in units per radian; without one, small
// ranges (Wacom's typical -64..63) are taken as degrees, larger ones scaled to +-90.
static float tilt_deg(int v, const struct input_absinfo *a) {
    if (a->maximum == a->minimum) return 0.f;
    if (a->resolution > 0) return clampf((float)v / (float)a->resolution * 57.29578f, -90.f, 90.f);
    int m = abs(a->minimum) > abs(a->maximum) ? abs(a->minimum) : abs(a->maximum);
    if (m <= 127) return clampf((float)v, -90.f, 90.f);
    return clampf((float)v * 90.f / (float)m, -90.f, 90.f);
}

// Panel <-> logical screen for a device turned `g_rot` quarter turns clockwise. g_w x g_h is the
// panel; the logical screen is g_h x g_w when turned sideways.
static void panel_to_logical(float px, float py, float *lx, float *ly) {
    float W = (float)(g_w - 1), H = (float)(g_h - 1);
    switch (g_rot & 3) {
    case 1: *lx = H - py; *ly = px; break;
    case 2: *lx = W - px; *ly = H - py; break;
    case 3: *lx = py; *ly = W - px; break;
    default: *lx = px; *ly = py; break;
    }
}

void input_logical_to_panel(float lx, float ly, float *px, float *py) {
    float W = (float)(g_w - 1), H = (float)(g_h - 1);
    switch (g_rot & 3) {
    case 1: *px = ly; *py = H - lx; break;
    case 2: *px = W - lx; *py = H - ly; break;
    case 3: *px = W - ly; *py = lx; break;
    default: *px = lx; *py = ly; break;
    }
}

void input_set_orientation(int quarter_turns) { g_rot = quarter_turns & 3; }

static void map(const InputCalib *c, float nx, float ny, float *x, float *y) {
    float px, py;
    calib_apply(c, nx, ny, &px, &py);
    px = clampf(px, 0.f, (float)(g_w - 1));
    py = clampf(py, 0.f, (float)(g_h - 1));
    panel_to_logical(px, py, x, y);
}

static uint64_t ev_us(const struct input_event *ev) {
    return (uint64_t)ev->input_event_sec * 1000000u + (uint64_t)ev->input_event_usec;
}

static void handle_wacom(const struct input_event *ev) {
    if (ev->type == EV_SYN && ev->code == SYN_REPORT) {
        if (g_ink_mono) lat_match_raw(&g_match, ev_us(ev), W.x, W.y);
        return;
    }
    if (ev->type == EV_KEY) {
        if (ev->code == BTN_TOOL_RUBBER) W.rubber = ev->value != 0;
        else if (ev->code == BTN_STYLUS) W.side = ev->value != 0;
        return;
    }
    if (ev->type != EV_ABS) return;
    switch (ev->code) {
    case ABS_PRESSURE: W.p = ev->value; W.seen = true; break;
    case ABS_X: W.x = ev->value; break;
    case ABS_Y: W.y = ev->value; break;
    case ABS_TILT_X: W.tx = ev->value; break;
    case ABS_TILT_Y: W.ty = ev->value; break;
    default: break;
    }
}

static void emit_pen(InType t) {
    InEvent e = {0};
    e.type = t;
    e.nx = norm(P.x, &g_pen.ax);
    e.ny = norm(P.y, &g_pen.ay);
    map(&g_pen_cal, e.nx, e.ny, &e.x, &e.y);
    e.pressure = t == IN_PEN_UP || t == IN_PEN_HOVER || t == IN_PEN_LEAVE ? 0.f : norm(P.p, &g_pen.ap);
    if (g_wacom.fd >= 0 && W.seen && (t == IN_PEN_DOWN || t == IN_PEN_MOVE)) e.pressure = norm(W.p, &g_wacom.ap);
    if ((t == IN_PEN_DOWN || t == IN_PEN_MOVE)) {
        if (e.pressure > 0.f || W.seen) g_pressure_seen = true;
        if (!g_pressure_seen && ++g_contact_frames > 40 && !g_pressure_missing) {
            g_pressure_missing = true;
            qlog("input: pen touches but reports no pressure; using a fixed 0.6 until it does");
        }
        if (g_pressure_missing && !g_pressure_seen) e.pressure = 0.6f;
    }
    if (g_wacom.fd >= 0) {
        e.tilt_x = tilt_deg(W.tx, &g_wacom.atx);
        e.tilt_y = tilt_deg(W.ty, &g_wacom.aty);
    } else {
        e.tilt_x = g_caps.has_tilt ? tilt_deg(P.tx, &g_pen.atx) : 0.f;
        e.tilt_y = g_caps.has_tilt ? tilt_deg(P.ty, &g_pen.aty) : 0.f;
    }
    // Into the screen's frame, like the position (the calligraphy nib turns with the lean).
    // On the owner's Scribe 2, held the way its sensor calls upside down, a right hand reads
    // (-40, -46) raw: leaning toward the screen's bottom right, (+40, +46) here.
    calib_tilt(&g_pen_cal, g_rot, g_tilt_flip, &e.tilt_x, &e.tilt_y);
    e.eraser = P.eraser;
    e.side_button = P.side || W.side;
    e.t_ms = now_ms();
    if (g_ink_mono && (t == IN_PEN_DOWN || t == IN_PEN_MOVE)) {
        g_ink_newest = g_pen_frame_us;
        if (!g_ink_oldest) g_ink_oldest = g_pen_frame_us;
        g_ink_raw = g_wacom.fd >= 0 ? lat_match_pen(&g_match, P.x, P.y, 3) : 0;
    }
    q_push(e);
}

bool input_ink_times(uint64_t *newest, uint64_t *oldest, uint64_t *raw) {
    if (!g_ink_newest) return false;
    *newest = g_ink_newest;
    *oldest = g_ink_oldest;
    *raw = g_ink_raw;
    g_ink_newest = g_ink_oldest = g_ink_raw = 0;
    return true;
}

static void emit_touch(InType t, float x, float y) {
    InEvent e = {0};
    e.type = t;
    e.nx = x;
    e.ny = y;
    map(&g_touch_cal, x, y, &e.x, &e.y);
    e.t_ms = now_ms();
    q_push(e);
}

static void pen_frame(void) {
    // A pen still touching the final PIN key must lift before it can start new ink.
    if (g_pen_resume_block) {
        if (P.touching) return;
        g_pen_resume_block = false;
        P.was_touching = false;
    }
    P.prox = P.tool_pen || P.tool_rubber;
    // The eraser end may only be visible on the raw digitizer (the virtual pen can drop it).
    P.eraser = P.tool_rubber || W.rubber;
    bool touching = P.touching;
    if (!g_pen.has_touch_btn) touching = P.prox && P.p > g_pen.ap.minimum;
    if (P.prox && !P.was_prox && T.active) {
        // The pen arrived while a finger (likely the palm) was down: end that touch.
        emit_touch(IN_TOUCH_UP, T.last_x, T.last_y);
        T.active = false;
        T.suppressed = true;
    }
    if (touching && !P.was_touching) emit_pen(IN_PEN_DOWN);
    else if (touching && P.dirty) emit_pen(IN_PEN_MOVE);
    else if (!touching && P.was_touching) emit_pen(IN_PEN_UP);
    else if (!touching && P.prox && P.dirty) emit_pen(IN_PEN_HOVER);
    if (!P.prox && P.was_prox) {
        emit_pen(IN_PEN_LEAVE);
        g_pen_left_ms = now_ms();
    }
    P.was_touching = touching;
    P.was_prox = P.prox;
    P.dirty = false;
}

static bool g_pen_dropping;
static int g_drops;

// Reads the pen's current buttons and axes straight from the device (after a SYN_DROPPED).
static void pen_resync(void) {
    unsigned long keys[NLONGS(KEY_MAX + 1)] = {0};
    if (ioctl(g_pen.fd, EVIOCGKEY(sizeof keys), keys) >= 0) {
        P.tool_pen = test_bit(keys, BTN_TOOL_PEN);
        P.tool_rubber = test_bit(keys, BTN_TOOL_RUBBER);
        P.touching = test_bit(keys, BTN_TOUCH);
        P.side = test_bit(keys, BTN_STYLUS);
    }
    struct input_absinfo ai;
    if (ioctl(g_pen.fd, EVIOCGABS(ABS_X), &ai) >= 0) P.x = ai.value;
    if (ioctl(g_pen.fd, EVIOCGABS(ABS_Y), &ai) >= 0) P.y = ai.value;
    if (ioctl(g_pen.fd, EVIOCGABS(ABS_PRESSURE), &ai) >= 0) P.p = ai.value;
    P.dirty = true;
}

static void handle_pen(const struct input_event *ev) {
    if (g_pen_dropping && !(ev->type == EV_SYN && ev->code == SYN_REPORT)) return;
    if (g_raw_logged < 160 && ev->type != EV_SYN) {
        g_raw_logged++;
        qlog("input: raw pen type %u code %u value %d", ev->type, ev->code, ev->value);
    }
    switch (ev->type) {
    case EV_KEY:
        switch (ev->code) {
        case BTN_TOOL_PEN: P.tool_pen = ev->value != 0; break;
        case BTN_TOOL_RUBBER: P.tool_rubber = ev->value != 0; break;
        case BTN_TOUCH: P.touching = ev->value != 0; break;
        case BTN_STYLUS: P.side = ev->value != 0; break;
        default: break;
        }
        break;
    case EV_ABS:
        switch (ev->code) {
        case ABS_X: P.x = ev->value; break;
        case ABS_Y: P.y = ev->value; break;
        case ABS_PRESSURE: P.p = ev->value; break;
        case ABS_MT_PRESSURE:  // some digitizers only report the MT flavour
            if (g_pen.amp.maximum > g_pen.amp.minimum)
                P.p = g_pen.ap.minimum + (int)((int64_t)(ev->value - g_pen.amp.minimum) *
                                               (g_pen.ap.maximum - g_pen.ap.minimum) /
                                               (g_pen.amp.maximum - g_pen.amp.minimum));
            break;
        case ABS_TILT_X: P.tx = ev->value; break;
        case ABS_TILT_Y: P.ty = ev->value; break;
        default: return;
        }
        P.dirty = true;
        break;
    case EV_SYN:
        if (ev->code == SYN_DROPPED) {
            // The kernel's buffer overflowed: everything up to the next SYN_REPORT is partial.
            // Ignore it, then re-read the pen's real state so a stroke ends or continues cleanly.
            g_pen_dropping = true;
            g_drops++;
            qlog("input: pen events dropped by the kernel (%d so far); resyncing", g_drops);
        } else if (ev->code == SYN_REPORT) {
            g_pen_frame_us = ev_us(ev);
            if (g_pen_dropping) {
                g_pen_dropping = false;
                pen_resync();
            }
            pen_frame();
        }
        break;
    default: break;
    }
}

// Touches are ignored while the pen is near and for a second after it leaves: the writing hand
// rests on the screen between strokes.
static bool palm_window(void) { return P.prox || (now_ms() - g_pen_left_ms) < 1000; }

// Ends the single-finger stream without a tap (the UP lands nowhere near any control).
static void cancel_touch(void) {
    if (!T.active) return;
    InEvent e = {0};
    e.type = IN_TOUCH_UP;
    e.x = e.y = -10000.f;
    e.t_ms = now_ms();
    q_push(e);
    T.active = false;
}

static bool g_pinching;

static void emit_pinch(InType t, int s1, int s2) {
    InEvent e = {0};
    e.type = t;
    float n1x = norm(T.x[s1], &g_touch.ax), n1y = norm(T.y[s1], &g_touch.ay);
    float n2x = norm(T.x[s2], &g_touch.ax), n2y = norm(T.y[s2], &g_touch.ay);
    map(&g_touch_cal, n1x, n1y, &e.x, &e.y);
    map(&g_touch_cal, n2x, n2y, &e.x2, &e.y2);
    e.t_ms = now_ms();
    q_push(e);
}

static void touch_frame(void) {
    // Two fingers down = pinch (zoom/pan). The single-finger stream pauses until all lift.
    // A contact the panel flags as a palm cancels touch input until every contact lifts.
    int fingers = 0, s1 = -1, s2 = -1;
    bool palm = false;
    for (int s = 0; s < MAX_SLOTS; s++) {
        if (T.id[s] < 0) continue;
        if (T.tool[s] == 2) {
            palm = true;
            continue;
        }
        fingers++;
        if (s1 < 0) s1 = s;
        else if (s2 < 0) s2 = s;
    }
    if (palm && !g_pinching) {
        cancel_touch();
        T.suppressed = true;
    }
    if (g_pinching && (palm_window() || palm)) {
        // The pen came near (or a palm landed) mid-pinch: stop zooming right there.
        g_pinching = false;
        q_push((InEvent){.type = IN_PINCH_END, .t_ms = now_ms()});
        T.suppressed = true;
        return;
    }
    if (fingers >= 2 && !palm_window() && !palm && !T.suppressed_pinch) {
        if (!g_pinching) {
            // A real pinch puts both fingers down together; a palm or a second hand arrives
            // later. Late second contacts are treated as a palm.
            uint32_t a = T.down_at[s1], b = T.down_at[s2];
            if ((a > b ? a - b : b - a) > 400) {
                cancel_touch();
                T.suppressed = true;
                T.suppressed_pinch = true;
                return;
            }
            g_pinching = true;
            cancel_touch();        // the first finger's press turns into the pinch
            T.suppressed = true;   // and no tap fires when the fingers lift
            emit_pinch(IN_PINCH_BEGIN, s1, s2);
        } else {
            emit_pinch(IN_PINCH_MOVE, s1, s2);
        }
        return;
    }
    if (g_pinching) {
        g_pinching = false;
        InEvent e = {0};
        e.type = IN_PINCH_END;
        e.t_ms = now_ms();
        q_push(e);
    }
    // Follow one finger: the first to land; another finger never moves the pointer.
    if (T.main >= 0 && T.id[T.main] < 0) {
        if (T.active) emit_touch(IN_TOUCH_UP, T.last_x, T.last_y);
        T.active = false;
        T.main = -1;
    }
    bool any = false;
    for (int s = 0; s < MAX_SLOTS; s++) any |= T.id[s] >= 0;
    if (!any) T.suppressed = T.suppressed_pinch = false;
    if (T.main < 0 && !T.suppressed)
        for (int s = 0; s < MAX_SLOTS; s++)
            if (T.id[s] >= 0) {
                T.main = s;
                break;
            }
    if (T.main < 0 || T.suppressed) return;
    if (palm_window()) {
        T.suppressed = true;
        if (T.active) emit_touch(IN_TOUCH_UP, T.last_x, T.last_y);
        T.active = false;
        return;
    }
    float x = norm(T.x[T.main], &g_touch.ax), y = norm(T.y[T.main], &g_touch.ay);
    if (!T.active) {
        T.active = true;
        emit_touch(IN_TOUCH_DOWN, x, y);
    } else if (x != T.last_x || y != T.last_y)
        emit_touch(IN_TOUCH_MOVE, x, y);
    T.last_x = x;
    T.last_y = y;
}

static void handle_touch(const struct input_event *ev) {
    if (ev->type == EV_ABS) {
        int s = T.slot;
        switch (ev->code) {
        case ABS_MT_SLOT: T.slot = clampi(ev->value, 0, MAX_SLOTS - 1); break;
        case ABS_MT_TRACKING_ID:
            if (ev->value >= 0 && T.id[s] < 0) {
                T.down_at[s] = now_ms();
                T.tool[s] = 0;
            }
            T.id[s] = ev->value;
            break;
        case ABS_MT_TOOL_TYPE: T.tool[s] = ev->value; break;
        case ABS_MT_POSITION_X: T.x[s] = ev->value; break;
        case ABS_MT_POSITION_Y: T.y[s] = ev->value; break;
        default: break;
        }
    } else if (ev->type == EV_KEY && ev->code == BTN_TOUCH && ev->value == 0) {
        // Belt and braces: some panels only signal the final lift through BTN_TOUCH.
        for (int s = 0; s < MAX_SLOTS; s++) T.id[s] = -1;
    } else if (ev->type == EV_SYN && ev->code == SYN_REPORT) {
        touch_frame();
    }
}

static void drain(int fd, void (*fn)(const struct input_event *)) {
    struct input_event evs[64];
    for (;;) {
        ssize_t r = read(fd, evs, sizeof evs);
        if (r <= 0) break;
        for (size_t i = 0; i < (size_t)r / sizeof evs[0]; i++) fn(&evs[i]);
        if ((size_t)r < sizeof evs) break;
    }
}

static int orient_code_to_turns(int v) {
    // KOReader's KindleGyroTransform (kx132.h / bma4xy): 15 portrait up, 16 portrait down,
    // 17 landscape left (device turned clockwise), 18 landscape right (counter-clockwise).
    switch (v) {
    case 15: return 0;
    case 17: return 1;
    case 16: return 2;
    case 18: return 3;
    default: return -1;  // face up/down and other codes: keep the current orientation
    }
}

static void handle_orient_fd(void) {
    struct input_event evs[16];
    ssize_t r = read(g_orient_fd, evs, sizeof evs);
    for (ssize_t i = 0; r > 0 && i < r / (ssize_t)sizeof evs[0]; i++) {
        if (evs[i].type != EV_ABS || evs[i].code != ABS_PRESSURE) continue;
        int t = orient_code_to_turns(evs[i].value);
        qlog("input: orientation code %d -> %d", evs[i].value, t);
        if (t < 0) continue;
        InEvent e = {0};
        e.type = IN_ORIENTATION;
        e.value = t;
        e.t_ms = now_ms();
        q_push(e);
    }
}

// The watcher shell. `-s 0` waits with no timeout (KOReader and Amazon's own scripts always pass
// -s). A run that fails prints a marker line (handle_powerd_fd logs it) and the loop tries again
// 2 s later. lipc-wait-event runs in the background so that the TERM trap can stop it: the shell
// gets TERM when Grimoire goes, even on a crash (PR_SET_PDEATHSIG), and takes its child along.
// No more event names in the one-shot loop: each one is one more restart around suspend and
// resume, and an outOfScreenSaver could fall into a restart's gap.
#define WATCH_EVENTS "com.lab126.powerd goingToScreenSaver,outOfScreenSaver"
#define WATCH_FAILED "grimoire-watcher-failed"
#define WATCH_TRAP "trap 'kill $w 2>/dev/null; exit 0' TERM; "
static const char k_watch_one_shot[] = WATCH_TRAP
    "while :; do lipc-wait-event -s 0 " WATCH_EVENTS " 2>&1 & w=$!; "
    "wait $w || { echo \"" WATCH_FAILED " $?\"; sleep 2; }; done";
// /mnt/us/grimoire/watcher-multi: one continuous run for every event, KOReader's way (no gap
// between runs), for a device test of whether -m output reaches Grimoire through the pipe.
static const char k_watch_multi[] = WATCH_TRAP
    "while :; do lipc-wait-event -m -s 0 " WATCH_EVENTS " 2>&1 & w=$!; "
    "wait $w; echo \"" WATCH_FAILED " $?\"; sleep 2; done";

static void powerd_start(void) {
    bool multi = access("/mnt/us/grimoire/watcher-multi", F_OK) == 0;
    int p[2];
    if (pipe(p) != 0) return;
    pid_t pid = fork();
    if (pid < 0) {
        close(p[0]);
        close(p[1]);
        return;
    }
    if (pid == 0) {
        setpgid(0, 0);  // its own group, so the loop and its child can be stopped together
#ifdef __linux__
        prctl(PR_SET_PDEATHSIG, SIGTERM);  // and it goes when Grimoire does, even on a crash
#endif
        dup2(p[1], 1);
        close(p[0]);
        close(p[1]);
        execlp("sh", "sh", "-c", multi ? k_watch_multi : k_watch_one_shot, (char *)NULL);
        _exit(127);
    }
    close(p[1]);
    fcntl(p[0], F_SETFL, O_NONBLOCK);
    g_powerd_fd = p[0];
    g_powerd_pid = pid;
    qlog("input: watching the power service (pid %d): %s", (int)pid,
         multi ? "one continuous lipc-wait-event -m (the watcher-multi switch)" : "one-shot lipc-wait-event runs");
}

static void handle_powerd_fd(void) {
    ssize_t r = read(g_powerd_fd, g_powerd_buf + g_powerd_len, sizeof g_powerd_buf - 1 - g_powerd_len);
    if (r <= 0) {
        if (r == 0) {  // the watcher died: stop polling it
            close(g_powerd_fd);
            g_powerd_fd = -1;
            qlog("input: power service watcher ended");
        }
        return;
    }
    g_powerd_len += (size_t)r;
    g_powerd_buf[g_powerd_len] = 0;
    char *line;
    while ((line = strchr(g_powerd_buf, '\n'))) {
        *line = 0;
        InEvent e = {0};
        if (!strncmp(g_powerd_buf, "goingToScreenSaver", 18)) e.type = IN_SLEEP;
        // Only outOfScreenSaver is a wake. (wakeupFromSuspend is not: the Kindle also resumes
        // briefly on its own timers while the screensaver stays up, and treating that as a wake
        // drew Grimoire over the screensaver and lost track of the real wake that followed.)
        else if (!strncmp(g_powerd_buf, "outOfScreenSaver", 16))
            e.type = IN_WAKE;
        if (e.type) {
            qlog("input: power service: %s", g_powerd_buf);
            e.value = atoi(g_powerd_buf + strcspn(g_powerd_buf, " "));
            e.t_ms = now_ms();
            q_push(e);
        } else if (g_powerd_noise++ < 20 || g_powerd_noise % 100 == 0) {
            // A failed run, or what lipc-wait-event said on the way (its errors come through
            // too). A watcher failing over and over would fill the log: the first 20, then 1 in 100.
            if (!strncmp(g_powerd_buf, WATCH_FAILED, strlen(WATCH_FAILED)))
                qlog("input: power service watcher failed, retrying (lipc-wait-event exit status%s; watcher message %d)",
                     g_powerd_buf + strlen(WATCH_FAILED), g_powerd_noise);
            else
                qlog("input: power service watcher said: %s", g_powerd_buf);
        }
        size_t used = (size_t)(line - g_powerd_buf) + 1;
        memmove(g_powerd_buf, line + 1, g_powerd_len - used + 1);
        g_powerd_len -= used;
    }
    if (g_powerd_len >= sizeof g_powerd_buf - 1) g_powerd_len = 0;  // no newline in sight: drop
}

static void push_power(InType t, int held_ms) {
    if (t == IN_NONE) return;
    InEvent e = {0};
    e.type = t;
    e.value = held_ms;
    e.t_ms = now_ms();
    q_push(e);
}

static void handle_power_fd(void) {
    struct input_event evs[16];
    ssize_t r = read(g_power_fd, evs, sizeof evs);
    for (ssize_t i = 0; r > 0 && i < r / (ssize_t)sizeof evs[0]; i++)
        if (evs[i].type == EV_KEY && evs[i].code == KEY_POWER) {
            // Timed by when the kernel saw it, not when it was read: a short press whose release is
            // read late (the loop was busy) must not count as a 4 s hold, nor a real one as short.
            uint32_t t = g_power_mono ? (uint32_t)((uint64_t)evs[i].input_event_sec * 1000u +
                                                   (uint64_t)evs[i].input_event_usec / 1000u)
                                      : now_ms();
            int held = 0;
            push_power(powerkey_event(&g_pkey, evs[i].value, t, &held), held);
        }
}

// Whether the power key is down right now, from the device itself: 1, 0, or -1 if it cannot tell.
static int power_key_down(void) {
    unsigned long keys[NLONGS(KEY_MAX + 1)] = {0};
    if (g_power_fd < 0 || ioctl(g_power_fd, EVIOCGKEY(sizeof keys), keys) < 0) return -1;
    return test_bit(keys, KEY_POWER) ? 1 : 0;
}

// The 4 s hold, checked on every poll (they come at least every 30 ms), timeouts included.
static void power_hold_check(void) {
    int held = 0;
    InType t = powerkey_tick(&g_pkey, now_ms(), power_key_down, &held);
    if (t == IN_POWER_UP) qlog("input: the power key is up at the %d ms check: its release was missed", INPUT_POWER_HOLD_MS);
    push_power(t, held);
}

// ---- setup ----

static void on_signal(int sig) {
    (void)sig;
    g_quit = 1;
    if (g_sigpipe[1] >= 0) {
        char c = 1;
        ssize_t w = write(g_sigpipe[1], &c, 1);
        (void)w;
    }
}

void input_default_calib(InputCalib *pen, InputCalib *touch) {
    // KOReader applies no axis swap/mirror for the Scribe: the axes map straight onto the
    // upright framebuffer. Swap only if the ranges say the axes are crossed.
    InputCalib p = {false, false, (float)g_w, 0.f, (float)g_h, 0.f};
    if (g_pen.fd >= 0) {
        int mx = g_pen.ax.maximum, my = g_pen.ay.maximum;
        if (abs(mx - g_h) < abs(mx - g_w) && abs(my - g_w) < abs(my - g_h)) p.swap_xy = true;
    }
    InputCalib t = {false, false, (float)g_w, 0.f, (float)g_h, 0.f};
    if (g_touch.fd >= 0) {
        int mx = g_touch.ax.maximum, my = g_touch.ay.maximum;
        float aspect_dev = my ? (float)mx / (float)my : 1.f, aspect_scr = g_h ? (float)g_w / (float)g_h : 1.f;
        if ((aspect_dev > 1.f) != (aspect_scr > 1.f)) t.swap_xy = true;
    }
    if (pen) *pen = p;
    if (touch) *touch = t;
}

void input_set_calib(const InputCalib *pen, const InputCalib *touch) {
    if (pen) g_pen_cal = *pen;
    if (touch) g_touch_cal = *touch;
}

const InputCaps *input_caps(void) { return &g_caps; }
void input_watch_fd(int fd) { g_watch = fd; }

// Optional external inputs are watched without exclusive grabs. Native PIN input is discarded
// at the same boundary as pen/touch, and key values and device names are never written to the log.
#define KEY_DEVS 4
typedef struct {
    int fd;
    char path[300], name[41];  // name: hidkeys_name's, for IN_HID_* and input_hids
    HidKind kind;
    bool held;                 // another app held it exclusively when it came
    unsigned hatmask;
    HidKeys keys;
} KeyDev;
static KeyDev g_keys[KEY_DEVS];
static bool g_keys_enabled, g_keys_init;
static uint32_t g_keys_scan;
static KeyDev *g_key_read;
static void key_push(const HidPress *p) {
    q_push((InEvent){.type = p->up ? IN_KEY_UP : IN_KEY, .value = p->code, .key_mods = (unsigned)p->mods,
                     .gamepad = p->gamepad, .t_ms = now_ms()});
}
// A resync's releases: what was held before and is not now (a hold ends with them, app.c).
static void key_ups(KeyDev *k, const uint8_t *held, int hat_x, int hat_y) {
    HidPress ups[16];
    int n = hidkeys_resync(&k->keys, k->kind, held, hat_x, hat_y, ups, 16);
    for (int i = 0; i < n; i++) key_push(&ups[i]);
}
static bool key_resync(KeyDev *k) {
    uint8_t keys[HID_KEY_BYTES] = {0};
    if (ioctl(k->fd, EVIOCGKEY(sizeof keys), keys) < 0) return false;
    int hats[2] = {0};
    k->hatmask = 0;
    for (int axis = 0; axis < 2; axis++) {
        struct input_absinfo info = {0};
        if (ioctl(k->fd, EVIOCGABS(ABS_HAT0X + axis), &info) >= 0 && info.minimum == -1 && info.maximum == 1) {
            hats[axis] = info.value;
            k->hatmask |= 1u << axis;
        }
    }
    key_ups(k, keys, hats[0], hats[1]);
    return true;
}
// Linux's native bitmap byte layout matches the decoder's on our little-endian armhf target.
_Static_assert(KEY_MAX + 1 <= HID_KEY_BYTES * 8, "external key bitmap too small");
_Static_assert(ABS_MAX + 1 <= HID_ABS_BYTES * 8, "external axis bitmap too small");
#define BYTES(bits) ((const uint8_t *)(bits))
static void key_announce(const KeyDev *k, InType type) {
    InEvent e = {.type = type, .gamepad = k->kind == HID_GAMEPAD, .value = k->held, .t_ms = now_ms()};
    strlcpy_q(e.hid_name, k->name, sizeof e.hid_name);
    q_push(e);
}
// A device that went away: unplugged, out of range, or unreadable after a resync.
static void key_detach(KeyDev *k) {
    close(k->fd);
    k->fd = -1;
    qlog("input: optional input at %s went away", k->path);
    key_announce(k, IN_HID_DETACH);
}
// Devices found by the scan that starts watching are not announced: they did not just come.
static void key_scan(bool announce) {
    g_keys_scan = now_ms();
    DIR *d = opendir("/dev/input");
    if (!d) return;
    struct dirent *e;
    while ((e = readdir(d))) {
        if (strncmp(e->d_name, "event", 5)) continue;
        char path[300]; snprintf(path, sizeof path, "/dev/input/%s", e->d_name);
        int slot = -1; bool seen = false;
        for (int i = 0; i < KEY_DEVS; i++) {
            if (g_keys[i].fd < 0) slot = i;
            else if (!strcmp(g_keys[i].path, path)) seen = true;
        }
        if (seen || slot < 0) continue;
        int fd = open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
        if (fd < 0) continue;
        unsigned long keys[NLONGS(KEY_MAX + 1)] = {0}, axes[NLONGS(ABS_MAX + 1)] = {0};
        char name[64] = "";
        ioctl(fd, EVIOCGNAME(sizeof name - 1), name);
        ioctl(fd, EVIOCGBIT(EV_KEY, sizeof keys), keys);
        ioctl(fd, EVIOCGBIT(EV_ABS, sizeof axes), axes);
        HidKind kind = hidkeys_optional(name, BYTES(keys), BYTES(axes));
        if (kind == HID_NONE) {
            close(fd); continue;
        }
        KeyDev *k = &g_keys[slot];
        *k = (KeyDev){.fd = fd, .kind = kind};
        strlcpy_q(k->path, path, sizeof k->path);
        hidkeys_name(k->name, sizeof k->name, name);
        // Another app's exclusive grab (a button mapper) would starve Grimoire of its presses: ask
        // for one ourselves, let go at once, and read without one as before.
        int grabbed = ioctl(fd, EVIOCGRAB, 1), err = errno;
        k->held = hidkeys_held(grabbed, err);
        if (grabbed == 0) ioctl(fd, EVIOCGRAB, 0);
        if (!key_resync(k)) { close(fd); k->fd = -1; continue; }
        qlog("input: optional %s attached at %s%s", kind == HID_GAMEPAD ? "controller" : "keyboard/remote", path,
             k->held ? ", held by another app" : "");
        if (announce) key_announce(k, IN_HID_ATTACH);
    }
    closedir(d);
}
void input_shortcuts_enable(bool enabled) {
    if (!g_keys_init) {
        for (int i = 0; i < KEY_DEVS; i++) g_keys[i].fd = -1;
        g_keys_init = true;
    }
    g_keys_enabled = enabled;
    if (enabled) key_scan(false);
    else for (int i = 0; i < KEY_DEVS; i++) {
        if (g_keys[i].fd >= 0) close(g_keys[i].fd);
        g_keys[i].fd = -1;
    }
}
int input_hids(InputHid *out, int cap) {
    int n = 0;
    for (int i = 0; g_keys_init && g_keys_enabled && i < KEY_DEVS && n < cap; i++) {
        if (g_keys[i].fd < 0) continue;
        out[n] = (InputHid){.gamepad = g_keys[i].kind == HID_GAMEPAD, .held = g_keys[i].held};
        strlcpy_q(out[n++].name, g_keys[i].name, sizeof out->name);
    }
    return n;
}
static void handle_key(const struct input_event *ev) {
    KeyDev *k = g_key_read;
    if (k->fd < 0) return;  // went away earlier in this read (key_detach)
    if (!g_input_grabbed) { key_ups(k, NULL, 0, 0); return; }
    if (k->keys.dropped) {
        if (ev->type == EV_SYN && ev->code == SYN_REPORT && !key_resync(k)) key_detach(k);
        return;
    }
    if (ev->type == EV_ABS && (ev->code == ABS_HAT0X || ev->code == ABS_HAT0Y) &&
        !(k->hatmask & (1u << (ev->code - ABS_HAT0X)))) return;
    HidPress out[2];
    int n = hidkeys_event(&k->keys, k->kind, ev->type, ev->code, ev->value, out);
    for (int i = 0; i < n; i++) key_push(&out[i]);
}

int input_open(int screen_w, int screen_h, const InputCalib *pen, const InputCalib *touch) {
    g_w = screen_w;
    g_h = screen_h;
    g_tilt_flip = access("/mnt/us/grimoire/tilt-flip", F_OK) == 0;
    if (g_tilt_flip) qlog("input: tilt-flip: reversing the pen's tilt direction");
    memset(&P, 0, sizeof P);
    for (int s = 0; s < MAX_SLOTS; s++) T.id[s] = -1;
    DIR *d = opendir("/dev/input");
    if (!d) {
        qlog("input: cannot open /dev/input: %s", strerror(errno));
        return -1;
    }
    struct dirent *e;
    int power_keys = 0, power_rank = 0;
    char power_path[300] = "";
    while ((e = readdir(d))) {
        if (strncmp(e->d_name, "event", 5)) continue;
        char path[300];
        snprintf(path, sizeof path, "/dev/input/%s", e->d_name);
        int fd = open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
        if (fd < 0) {
            qlog("input: cannot open %s: %s", path, strerror(errno));
            continue;
        }
        char name[64] = "?";
        ioctl(fd, EVIOCGNAME(sizeof name), name);
        unsigned long evb[NLONGS(EV_MAX + 1)] = {0}, keyb[NLONGS(KEY_MAX + 1)] = {0}, absb[NLONGS(ABS_MAX + 1)] = {0};
        ioctl(fd, EVIOCGBIT(0, sizeof evb), evb);
        ioctl(fd, EVIOCGBIT(EV_KEY, sizeof keyb), keyb);
        ioctl(fd, EVIOCGBIT(EV_ABS, sizeof absb), absb);
        bool has_abs = test_bit(evb, EV_ABS), has_key = test_bit(evb, EV_KEY);
        {
            char axes[256] = "";
            size_t al = 0;
            static const struct { int code; const char *n; } names[] = {
                {ABS_X, "X"}, {ABS_Y, "Y"}, {ABS_PRESSURE, "P"}, {ABS_DISTANCE, "DIST"}, {ABS_TILT_X, "TX"},
                {ABS_TILT_Y, "TY"}, {ABS_MT_POSITION_X, "MTX"}, {ABS_MT_POSITION_Y, "MTY"},
                {ABS_MT_PRESSURE, "MTP"}, {ABS_MT_TOOL_TYPE, "MTTOOL"}};
            for (size_t k = 0; k < sizeof names / sizeof names[0] && al < sizeof axes - 40; k++) {
                if (!test_bit(absb, names[k].code)) continue;
                struct input_absinfo ai = {0};
                ioctl(fd, EVIOCGABS(names[k].code), &ai);
                al += (size_t)snprintf(axes + al, sizeof axes - al, " %s[%d..%d r%d]", names[k].n, ai.minimum, ai.maximum,
                                       ai.resolution);
            }
            qlog("input: found %s \"%s\" abs:%s keys:%s%s%s%s%s", path, name, axes[0] ? axes : " none",
                 test_bit(keyb, BTN_TOOL_PEN) ? " TOOL_PEN" : "", test_bit(keyb, BTN_TOOL_RUBBER) ? " RUBBER" : "",
                 test_bit(keyb, BTN_TOUCH) ? " TOUCH" : "", test_bit(keyb, BTN_STYLUS) ? " STYLUS" : "",
                 test_bit(keyb, KEY_POWER) ? " POWER" : "");
        }
        bool pen = has_abs && has_key && test_bit(absb, ABS_X) && test_bit(absb, ABS_PRESSURE) &&
                   test_bit(keyb, BTN_TOOL_PEN);
        bool touch = !pen && has_abs && test_bit(absb, ABS_MT_POSITION_X);
        // The power key is decided by its own rule (hidkeys_power), not by what makes a keyboard.
        int power = !pen && !touch && has_key ? hidkeys_power(name, BYTES(keyb), BYTES(absb)) : -1;
        // Orientation: ABS_PRESSURE with the accelerometer's tiny code range (15..22), not a pen.
        if (!pen && !touch && has_abs && test_bit(absb, ABS_PRESSURE) && !test_bit(absb, ABS_X) && g_orient_fd < 0) {
            struct input_absinfo ai = {0};
            ioctl(fd, EVIOCGABS(ABS_PRESSURE), &ai);
            if (ai.minimum >= 10 && ai.maximum <= 30) {
                g_orient_fd = fd;
                g_caps.has_orientation = true;
                int t = orient_code_to_turns(ai.value);
                g_caps.orientation = t < 0 ? 0 : t;
                qlog("input: orientation sensor %s (%s), current code %d -> %d", path, name, ai.value, g_caps.orientation);
                continue;
            }
            qlog("input: %s (%s) has ABS_PRESSURE %d..%d, outside an orientation sensor's 10..30: not used for "
                 "rotation", path, name, ai.minimum, ai.maximum);
        }
        // The scaled pen (axis maxima match the screen) drives the pointer; another pen device,
        // if any, only supplies pressure + tilt.
        bool scaled = false;
        if (pen) {
            struct input_absinfo ax = {0}, ay = {0};
            ioctl(fd, EVIOCGABS(ABS_X), &ax);
            ioctl(fd, EVIOCGABS(ABS_Y), &ay);
            scaled = (abs(ax.maximum - g_w) <= 2 && abs(ay.maximum - g_h) <= 2) ||
                     (abs(ax.maximum - g_h) <= 2 && abs(ay.maximum - g_w) <= 2);
            if (!scaled && g_wacom.fd < 0 && test_bit(absb, ABS_PRESSURE)) {
                g_wacom.fd = fd;
                ioctl(fd, EVIOCGABS(ABS_X), &g_wacom.ax);
                ioctl(fd, EVIOCGABS(ABS_Y), &g_wacom.ay);
                ioctl(fd, EVIOCGABS(ABS_PRESSURE), &g_wacom.ap);
                ioctl(fd, EVIOCGABS(ABS_TILT_X), &g_wacom.atx);
                ioctl(fd, EVIOCGABS(ABS_TILT_Y), &g_wacom.aty);
                qlog("input: pressure/tilt source %s (%s), p %d..%d tilt %d..%d (not grabbed)", path, name,
                     g_wacom.ap.minimum, g_wacom.ap.maximum, g_wacom.atx.minimum, g_wacom.atx.maximum);
                continue;
            }
        }
        if (pen && g_pen.fd >= 0 && scaled) {
            // A scaled pen found after a raw one: the raw one becomes the pressure source.
            close(fd);
            continue;
        }
        if (pen && g_pen.fd < 0) {
            g_pen.fd = fd;
            ioctl(fd, EVIOCGABS(ABS_X), &g_pen.ax);
            ioctl(fd, EVIOCGABS(ABS_Y), &g_pen.ay);
            ioctl(fd, EVIOCGABS(ABS_PRESSURE), &g_pen.ap);
            if (test_bit(absb, ABS_MT_PRESSURE)) ioctl(fd, EVIOCGABS(ABS_MT_PRESSURE), &g_pen.amp);
            g_caps.has_tilt = test_bit(absb, ABS_TILT_X) && test_bit(absb, ABS_TILT_Y);
            if (g_caps.has_tilt) {
                ioctl(fd, EVIOCGABS(ABS_TILT_X), &g_pen.atx);
                ioctl(fd, EVIOCGABS(ABS_TILT_Y), &g_pen.aty);
            }
            g_pen.has_touch_btn = test_bit(keyb, BTN_TOUCH);
            if (ioctl(fd, EVIOCGRAB, 1) != 0) {
                g_initial_grab_failed = true;
                qlog("input: could not grab the pen: %s", strerror(errno));
            }
            g_caps.has_pen = true;
            g_caps.has_pressure = g_pen.ap.maximum > g_pen.ap.minimum;
            strlcpy_q(g_caps.pen_name, name, sizeof g_caps.pen_name);
            qlog("input: pen %s (%s) x %d..%d y %d..%d p %d..%d tilt %s %d..%d res %d", path, name, g_pen.ax.minimum,
                 g_pen.ax.maximum, g_pen.ay.minimum, g_pen.ay.maximum, g_pen.ap.minimum, g_pen.ap.maximum,
                 g_caps.has_tilt ? "yes" : "no", g_pen.atx.minimum, g_pen.atx.maximum, g_pen.atx.resolution);
        } else if (touch && g_touch.fd < 0) {
            g_touch.fd = fd;
            ioctl(fd, EVIOCGABS(ABS_MT_POSITION_X), &g_touch.ax);
            ioctl(fd, EVIOCGABS(ABS_MT_POSITION_Y), &g_touch.ay);
            if (ioctl(fd, EVIOCGRAB, 1) != 0) {
                g_initial_grab_failed = true;
                qlog("input: could not grab touch: %s", strerror(errno));
            }
            g_caps.has_touch = true;
            strlcpy_q(g_caps.touch_name, name, sizeof g_caps.touch_name);
            qlog("input: touch %s (%s) x %d..%d y %d..%d", path, name, g_touch.ax.minimum, g_touch.ax.maximum,
                 g_touch.ay.minimum, g_touch.ay.maximum);
        } else if (power >= 0 && (g_power_fd < 0 || power < power_rank)) {
            if (g_power_fd >= 0) {
                qlog("input: %s is another power key with more keys, not watched", power_path);
                close(g_power_fd);
            }
            g_power_fd = fd;
            power_rank = power;
            g_caps.has_power = true;
            power_keys++;
            strlcpy_q(power_path, path, sizeof power_path);
            // Its events stamped in now_ms()'s clock (only for this reader), to time a hold exactly.
            int clk = CLOCK_MONOTONIC;
            g_power_mono = ioctl(fd, EVIOCSCLOCKID, &clk) == 0;
            qlog("input: power key %s (%s)%s", path, name,
                 g_power_mono ? "" : ", timed when read (EVIOCSCLOCKID failed)");
        } else {
            if (power >= 0) {
                power_keys++;
                qlog("input: %s (%s) is another power key, not watched", path, name);
            }
            close(fd);
        }
    }
    closedir(d);
    // One power key is watched, the one with the fewest keys (the first found of those): if another
    // device is the one that reports presses, Grimoire never sees the button. (The pen, the touch
    // panel, keyboards and Amazon's every-key proxy are never taken for one, so a KEY_POWER they
    // advertise does not count here: hidkeys_power, pinned by tests/test_hidkeys.c.)
    if (power_keys > 1)
        qlog("input: WARNING %d devices report a power key; watching only %s", power_keys, power_path);
    if (g_power_fd < 0) qlog("input: WARNING no power key found: no fallbacks, and no 4 s hold to leave");
    if (!g_caps.has_orientation) qlog("input: no orientation sensor found");
    if (g_pen.fd < 0 && g_wacom.fd >= 0) {
        // Only a raw digitizer: use it for everything (calibration maps its axes).
        g_pen = g_wacom;
        g_wacom.fd = -1;
        ioctl(g_pen.fd, EVIOCGABS(ABS_X), &g_pen.ax);
        ioctl(g_pen.fd, EVIOCGABS(ABS_Y), &g_pen.ay);
        if (ioctl(g_pen.fd, EVIOCGRAB, 1) != 0) g_initial_grab_failed = true;
        g_caps.has_pen = g_caps.has_pressure = true;
        g_caps.has_tilt = g_pen.atx.maximum > g_pen.atx.minimum;
        qlog("input: using the raw digitizer as the pen");
    }
    // Ink timing (latency.h, with /mnt/us/grimoire/ink-metrics): the pens' events stamped in now_ms()'s
    // clock. The clock is per reader, so the pen service and the Kindle's other readers keep theirs.
    if (g_pen.fd >= 0 && access("/mnt/us/grimoire/ink-metrics", F_OK) == 0) {
        int clk = CLOCK_MONOTONIC;
        g_ink_mono = ioctl(g_pen.fd, EVIOCSCLOCKID, &clk) == 0 &&
                     (g_wacom.fd < 0 || ioctl(g_wacom.fd, EVIOCSCLOCKID, &clk) == 0);
        if (!g_ink_mono) qlog("input: EVIOCSCLOCKID failed: %s (no ink timing)", strerror(errno));
        int rmin[2] = {g_wacom.ax.minimum, g_wacom.ay.minimum}, rmax[2] = {g_wacom.ax.maximum, g_wacom.ay.maximum};
        int smin[2] = {g_pen.ax.minimum, g_pen.ay.minimum}, smax[2] = {g_pen.ax.maximum, g_pen.ay.maximum};
        lat_match_init(&g_match, rmin, rmax, smin, smax);
    }
    if (g_wacom.fd >= 0) {
        g_caps.has_pressure = g_wacom.ap.maximum > g_wacom.ap.minimum;
        g_caps.has_tilt = g_wacom.atx.maximum > g_wacom.atx.minimum;
    }
    if (g_pen.fd < 0) qlog("input: WARNING no pen digitizer found");
    if (g_touch.fd < 0) qlog("input: WARNING no touch panel found");
    input_default_calib(&g_pen_cal, &g_touch_cal);
    if (pen && pen->valid) g_pen_cal = *pen;
    if (touch && touch->valid) g_touch_cal = *touch;

    powerd_start();
    if (pipe(g_sigpipe) == 0) {
        fcntl(g_sigpipe[0], F_SETFL, O_NONBLOCK);
        fcntl(g_sigpipe[1], F_SETFL, O_NONBLOCK);
    }
    struct sigaction sa = {0};
    sa.sa_handler = on_signal;
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGHUP, &sa, NULL);
    if (g_initial_grab_failed) qlog("input: another app holds an input device; refusing to open an unresponsive UI");
    return !g_initial_grab_failed && (g_pen.fd >= 0 || g_touch.fd >= 0) ? 0 : -1;
}

static void discard_input(const struct input_event *ev);

int input_poll(InEvent *ev, int timeout_ms) {
    if (g_quit) {
        *ev = (InEvent){.type = IN_QUIT, .t_ms = now_ms()};
        g_quit = 0;
        return 1;
    }
    if (q_pop(ev)) return 1;
    if (g_keys_enabled && (uint32_t)(now_ms() - g_keys_scan) >= 5000) key_scan(true);
    struct pollfd pf[8 + KEY_DEVS];
    int ikey[KEY_DEVS];
    int n = 0, ipen = -1, itouch = -1, ipow = -1, isig = -1, iwac = -1, iori = -1, ipwd = -1;
    if (g_powerd_fd >= 0) ipwd = n, pf[n++] = (struct pollfd){g_powerd_fd, POLLIN, 0};
    if (g_orient_fd >= 0) iori = n, pf[n++] = (struct pollfd){g_orient_fd, POLLIN, 0};
    if (g_wacom.fd >= 0) iwac = n, pf[n++] = (struct pollfd){g_wacom.fd, POLLIN, 0};
    if (g_pen.fd >= 0) ipen = n, pf[n++] = (struct pollfd){g_pen.fd, POLLIN, 0};
    if (g_touch.fd >= 0) itouch = n, pf[n++] = (struct pollfd){g_touch.fd, POLLIN, 0};
    if (g_power_fd >= 0) ipow = n, pf[n++] = (struct pollfd){g_power_fd, POLLIN, 0};
    if (g_sigpipe[0] >= 0) isig = n, pf[n++] = (struct pollfd){g_sigpipe[0], POLLIN, 0};
    if (g_watch >= 0) pf[n++] = (struct pollfd){g_watch, POLLIN, 0};
    for (int i = 0; i < KEY_DEVS; i++) {
        ikey[i] = -1;
        if (g_keys_enabled && g_keys[i].fd >= 0) ikey[i] = n, pf[n++] = (struct pollfd){g_keys[i].fd, POLLIN, 0};
    }
    int r = poll(pf, (nfds_t)n, timeout_ms);
    if (r > 0) {
        // Raw digitizer first, so a pen frame uses this sample's pressure.
        if (iwac >= 0 && (pf[iwac].revents & POLLIN)) drain(g_wacom.fd, g_input_grabbed ? handle_wacom : discard_input);
        if (ipen >= 0 && (pf[ipen].revents & POLLIN)) drain(g_pen.fd, g_input_grabbed ? handle_pen : discard_input);
        if (itouch >= 0 && (pf[itouch].revents & POLLIN)) drain(g_touch.fd, g_input_grabbed ? handle_touch : discard_input);
        if (ipow >= 0 && (pf[ipow].revents & POLLIN)) handle_power_fd();
        if (iori >= 0 && (pf[iori].revents & POLLIN)) handle_orient_fd();
        if (ipwd >= 0 && (pf[ipwd].revents & (POLLIN | POLLHUP))) handle_powerd_fd();
        // A simultaneous pen-down or native sleep event takes ownership before external keys.
        for (int i = 0; i < KEY_DEVS; i++) if (ikey[i] >= 0) {
            short re = pf[ikey[i]].revents;
            if (re & POLLIN) { g_key_read = &g_keys[i]; drain(g_keys[i].fd, handle_key); }
            if (g_keys[i].fd >= 0 && (re & (POLLHUP | POLLERR | POLLNVAL))) key_detach(&g_keys[i]);
        }
        if (isig >= 0 && (pf[isig].revents & POLLIN)) {
            char buf[16];
            while (read(g_sigpipe[0], buf, sizeof buf) > 0) {
            }
        }
    }
    power_hold_check();
    if (g_quit) {
        *ev = (InEvent){.type = IN_QUIT, .t_ms = now_ms()};
        g_quit = 0;
        return 1;
    }
    // Side button presses/releases, from whichever pen device reports them.
    bool side = P.side || W.side;
    if (side != g_side_reported) {
        g_side_reported = side;
        InEvent b = {0};
        b.type = IN_PEN_BUTTON;
        b.side_button = side;
        b.nx = norm(P.x, &g_pen.ax);
        b.ny = norm(P.y, &g_pen.ay);
        map(&g_pen_cal, b.nx, b.ny, &b.x, &b.y);
        b.t_ms = now_ms();
        qlog("input: pen button %s", side ? "down" : "up");
        q_push(b);
    }
    return q_pop(ev) ? 1 : 0;
}

static void discard_input(const struct input_event *ev) { (void)ev; }

bool input_set_grab(bool grab) {
    bool ok = true;
    for (int i = 0; g_keys_init && i < KEY_DEVS; i++) if (g_keys[i].fd >= 0) {
        drain(g_keys[i].fd, discard_input);
        key_ups(&g_keys[i], NULL, 0, 0);  // (what was held is let go: a hold ends)
        if (grab && !key_resync(&g_keys[i])) key_detach(&g_keys[i]);
    }
    // Forget touches that were used to type the PIN, including the final finger lift.
    if (grab) {
        if (g_pen.fd >= 0) drain(g_pen.fd, discard_input);
        if (g_touch.fd >= 0) drain(g_touch.fd, discard_input);
        if (g_wacom.fd >= 0) drain(g_wacom.fd, discard_input);
    }
    if (g_pen.fd >= 0 && ioctl(g_pen.fd, EVIOCGRAB, grab ? 1 : 0) != 0) ok = false;
    if (g_touch.fd >= 0 && ioctl(g_touch.fd, EVIOCGRAB, grab ? 1 : 0) != 0) ok = false;
    if (!ok && grab) {
        if (g_pen.fd >= 0) ioctl(g_pen.fd, EVIOCGRAB, 0);
        if (g_touch.fd >= 0) ioctl(g_touch.fd, EVIOCGRAB, 0);
    }
    g_input_grabbed = grab && ok;
    memset(&P, 0, sizeof P);
    memset(&W, 0, sizeof W);
    memset(&T, 0, sizeof T);
    T.main = -1;
    T.suppressed = true;
    for (int i = 0; i < MAX_SLOTS; i++) T.id[i] = -1;
    g_pinching = false;
    g_side_reported = false;
    g_pen_resume_block = false;
    if (g_input_grabbed) {
        if (g_pen.fd >= 0) {
            pen_resync();
            g_pen_resume_block = P.touching;
            g_side_reported = P.side;
        }
        if (g_touch.fd >= 0) {
            int slots[MAX_SLOTS + 1];
            slots[0] = ABS_MT_TRACKING_ID;
            for (int i = 1; i <= MAX_SLOTS; i++) slots[i] = -1;
            if (ioctl(g_touch.fd, EVIOCGMTSLOTS(sizeof slots), slots) >= 0) {
                T.suppressed = false;
                for (int i = 0; i < MAX_SLOTS; i++) {
                    T.id[i] = slots[i + 1];
                    T.suppressed |= T.id[i] >= 0;
                }
            }
            struct input_absinfo ai;
            if (ioctl(g_touch.fd, EVIOCGABS(ABS_MT_SLOT), &ai) >= 0) T.slot = clampi(ai.value, 0, MAX_SLOTS - 1);
        }
    }
    qlog("input: native lock: %s %s", grab ? "grab" : "release", ok ? "ok" : "failed");
    return ok;
}

void input_close(void) {
    input_shortcuts_enable(false);
    if (g_pen.fd >= 0) {
        ioctl(g_pen.fd, EVIOCGRAB, 0);
        close(g_pen.fd);
    }
    if (g_touch.fd >= 0) {
        ioctl(g_touch.fd, EVIOCGRAB, 0);
        close(g_touch.fd);
    }
    if (g_power_fd >= 0) close(g_power_fd);
    if (g_wacom.fd >= 0) close(g_wacom.fd);
    g_wacom.fd = -1;
    if (g_orient_fd >= 0) close(g_orient_fd);
    g_orient_fd = -1;
    if (g_powerd_pid > 0) {
        kill(-g_powerd_pid, SIGTERM);  // the shell and its current lipc-wait-event
        kill(g_powerd_pid, SIGTERM);
        waitpid(g_powerd_pid, NULL, 0);
    }
    if (g_powerd_fd >= 0) close(g_powerd_fd);
    g_powerd_fd = -1;
    g_powerd_pid = -1;
    for (int i = 0; i < 2; i++)
        if (g_sigpipe[i] >= 0) close(g_sigpipe[i]);
    g_pen.fd = g_touch.fd = g_power_fd = -1;
    g_sigpipe[0] = g_sigpipe[1] = -1;
}
