// scribe-tablet: the Kindle Scribe's pen as an absolute tablet for a Mac, over SSH (docs/TABLET.md).
//
// ssh runs `cat` on the pen's evdev node on the Kindle (KOReader's SSH server), and this reads the
// stream: 32-bit ARM input_events (16 bytes: tv_sec u32, tv_usec u32, type u16, code u16, value
// i32, little-endian). It keeps the pen's state and posts a CGEvent at each SYN_REPORT: the pointer
// moved while the pen hovers, a button down, dragged and up while it touches. Nothing is smoothed,
// coalesced or held back: a frame is posted the moment its SYN_REPORT is read.
//
// The default device is the raw digitizer (WacomDigitizer): it streams where the pen hovers, which
// osu! aims with, and KOReader never opens it. The scaled pen (stylus-custom) says that the pen is
// near but not where it hovers, and KOReader grabs it (EVIOCGRAB) while it runs, so a second reader
// gets nothing from it then.
//
// The map from the pen to the screen is affine, worked out once (map_init) from place(), which
// spells out its steps; a frame costs a few multiplies and two clamps.
//
// Build: make tablet (clang, ApplicationServices; macOS only). Check: tests/tablet_check.sh.

#include <ApplicationServices/ApplicationServices.h>
#include <errno.h>
#include <fcntl.h>
#include <getopt.h>
#include <math.h>
#include <pthread/qos.h>
#include <signal.h>
#include <spawn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

extern char **environ;

// ---- linux/input.h (the numbers the stream carries) ----
enum { EV_SYN = 0x00, EV_KEY = 0x01, EV_ABS = 0x03 };
enum { SYN_REPORT = 0, SYN_DROPPED = 3 };
enum { ABS_X = 0x00, ABS_Y = 0x01, ABS_PRESSURE = 0x18 };
enum { BTN_TOOL_PEN = 0x140, BTN_TOOL_RUBBER = 0x141, BTN_TOUCH = 0x14a, BTN_STYLUS = 0x14b, BTN_STYLUS2 = 0x14c };
// input_event on 32-bit ARM: struct timeval of two 32-bit longs, then type, code, value.
enum { EV_SIZE = 16 };

// The remote command's exit status when the Kindle has no device of that name.
enum { NO_DEVICE = 4 };
// Taps closer than this in time (us) and place (points) count as a double click.
enum { CLICK_US = 500000, CLICK_PT = 4 };

// The Scribe 2022/2024: the raw digitizer's and the scaled pen's ranges (both from 0), and the
// panel the digitizer covers: 1860 x 2480 px at 300 dpi.
#define RAW_NAME "WacomDigitizer"
#define RAW_X 15624
#define RAW_Y 20832
#define SCALED_NAME "stylus-custom"
#define SCALED_X 1860
#define SCALED_Y 2480
#define PANEL_W_MM (1860 * 25.4 / 300)
#define PANEL_H_MM (2480 * 25.4 / 300)

enum { RIGHT_SIDE = 1, RIGHT_ERASER = 2 };

typedef struct {
    const char *host, *user, *bind, *device, *replay, *capture;
    int port;
    char **ssh_args;            // after "--": given to ssh before its own options, so they win
    int n_ssh_args;
    double range_x, range_y;    // the device's maxima (each axis from 0)
    bool flip_x, flip_y;
    int rotate;                 // how the Kindle is turned, clockwise: 0, 90, 180, 270
    double panel_w, panel_h;    // mm, portrait
    double area_w, area_h, area_cx, area_cy; // mm, in the frame the Kindle is held in
    int n_area;                 // numbers given to --area (0: the largest of the screen's shape)
    int display;                // index among the active displays (0 is the main one)
    double scr_x, scr_y, scr_w, scr_h; // points, global display space (y down)
    bool screen_set;
    int pressure, release;      // touch at pressure >= pressure, up below release; 0: BTN_TOUCH
    bool hover_only;
    int right;                  // RIGHT_* that make a touch the right button
    bool dry_run, stats, list_devices;
    long bench;
} Opt;

// ---- the map ----

typedef struct {
    double c[6];                // screen = (c0 + c1 x + c2 y, c3 + c4 x + c5 y), raw x, y
    double x0, y0, x1, y1;      // the screen rect's first and last point
} Map;

// The panel's size in mm as the Kindle is held.
static void held_panel(const Opt *o, double *w, double *h) {
    bool turned = o->rotate == 90 || o->rotate == 270;
    *w = turned ? o->panel_h : o->panel_w;
    *h = turned ? o->panel_w : o->panel_h;
}

// Where raw (x, y) lands, unclamped: a fraction of each axis (flipped if asked), turned as the
// Kindle is held (at 90 its top edge is on the right), in mm, as a fraction of the area, on the
// screen rect (the area's corners on the rect's first and last points).
static void place(const Opt *o, double x, double y, double *sx, double *sy) {
    double u = x / o->range_x, v = y / o->range_y, hu, hv, pw, ph;
    if (o->flip_x) u = 1 - u;
    if (o->flip_y) v = 1 - v;
    switch (o->rotate) {
    case 90: hu = 1 - v, hv = u; break;
    case 180: hu = 1 - u, hv = 1 - v; break;
    case 270: hu = v, hv = 1 - u; break;
    default: hu = u, hv = v;
    }
    held_panel(o, &pw, &ph);
    double a = (hu * pw - (o->area_cx - o->area_w / 2)) / o->area_w;
    double b = (hv * ph - (o->area_cy - o->area_h / 2)) / o->area_h;
    *sx = o->scr_x + a * (o->scr_w - 1);
    *sy = o->scr_y + b * (o->scr_h - 1);
}

static void map_init(Map *m, const Opt *o) {
    double ox, oy, xx, xy, yx, yy;
    place(o, 0, 0, &ox, &oy);
    place(o, o->range_x, 0, &xx, &xy);
    place(o, 0, o->range_y, &yx, &yy);
    m->c[0] = ox, m->c[1] = (xx - ox) / o->range_x, m->c[2] = (yx - ox) / o->range_y;
    m->c[3] = oy, m->c[4] = (xy - oy) / o->range_x, m->c[5] = (yy - oy) / o->range_y;
    m->x0 = o->scr_x, m->x1 = o->scr_x + o->scr_w - 1;
    m->y0 = o->scr_y, m->y1 = o->scr_y + o->scr_h - 1;
}

static double clampd(double v, double lo, double hi) { return v < lo ? lo : v > hi ? hi : v; }

static void map_apply(const Map *m, int x, int y, double *sx, double *sy) {
    *sx = clampd(m->c[0] + m->c[1] * x + m->c[2] * y, m->x0, m->x1);
    *sy = clampd(m->c[3] + m->c[4] * x + m->c[5] * y, m->y0, m->y1);
}

// ---- where the events go ----

enum { OUT_MOVE, OUT_DOWN, OUT_DRAG, OUT_UP };
enum { SINK_POST, SINK_PRINT, SINK_COUNT };

typedef struct {
    int mode;
    CGEventSourceRef src;
    double last_x, last_y;      // the last place posted, for the events' deltas
    bool have_last;
    uint64_t n;                 // SINK_COUNT: events and a checksum (keeps the work from being optimized out)
    double sum;
} Sink;

static void emit(Sink *k, int kind, int button, double x, double y, int clicks, uint64_t t_us) {
    if (k->mode == SINK_COUNT) {
        k->n++, k->sum += x + y + kind + button;
        return;
    }
    if (k->mode == SINK_PRINT) {
        static const char *const names[] = {"move", "down", "drag", "up"};
        printf("%llu.%06llu %s", (unsigned long long)(t_us / 1000000), (unsigned long long)(t_us % 1000000),
               names[kind]);
        if (kind != OUT_MOVE) printf(" %s", button == 2 ? "right" : "left");
        printf(" %.2f %.2f", x, y);
        if (kind == OUT_DOWN || kind == OUT_UP) printf(" %d", clicks);
        putchar('\n');
        return;
    }
    static const CGEventType left[] = {kCGEventMouseMoved, kCGEventLeftMouseDown, kCGEventLeftMouseDragged,
                                       kCGEventLeftMouseUp};
    static const CGEventType right[] = {kCGEventMouseMoved, kCGEventRightMouseDown, kCGEventRightMouseDragged,
                                        kCGEventRightMouseUp};
    CGEventRef e = CGEventCreateMouseEvent(k->src, button == 2 ? right[kind] : left[kind], CGPointMake(x, y),
                                           button == 2 ? kCGMouseButtonRight : kCGMouseButtonLeft);
    if (!e) return;
    if (kind == OUT_DOWN || kind == OUT_UP) CGEventSetIntegerValueField(e, kCGMouseEventClickState, clicks);
    // A game reading the pointer relatively sees the same motion.
    if (k->have_last) {
        CGEventSetIntegerValueField(e, kCGMouseEventDeltaX, (int64_t)lround(x - k->last_x));
        CGEventSetIntegerValueField(e, kCGMouseEventDeltaY, (int64_t)lround(y - k->last_y));
    }
    CGEventPost(kCGHIDEventTap, e);
    CFRelease(e);
    k->last_x = x, k->last_y = y, k->have_last = true;
}

// ---- the pen ----

typedef struct {
    int x, y, p;
    bool have_x, have_y;
    bool pen, rubber, touch, side;
    bool near;                  // the pen is in range: a tool came, or a place was reported
    bool unsure;                // since the start or a drop: no event has said whether it touches
    int button;                 // the button posted and held: 0, 1 left, 2 right
    double sx, sy;              // the last place posted
    bool placed;
    uint64_t down_t;            // the last button down: when, where, which, its click count
    double down_x, down_y;
    int down_button, clicks;
    uint8_t part[EV_SIZE];      // an event split between two reads
    int n_part;
    uint64_t events, frames, drops;
    uint64_t last_t;            // the newest event's time
    uint64_t arrive_us;         // when the read that holds this frame returned (--stats)
    void (*on_frame)(void *, uint64_t t_us);
    void *on_frame_ctx;
} Pen;

static void pen_init(Pen *s) {
    memset(s, 0, sizeof *s);
    s->unsure = true;
}

// What the device still holds after a connection ends: forgotten (the place posted is kept).
static void pen_forget(Pen *s) {
    s->have_x = s->have_y = s->pen = s->rubber = s->touch = s->side = s->near = false;
    s->p = 0, s->n_part = 0;
    s->unsure = true;
}

// Lets go of a held button where the pointer is: the stream ended, or a frame with no place let go.
static void pen_release(Pen *s, Sink *k, uint64_t t_us) {
    if (!s->button) return;
    emit(k, OUT_UP, s->button, s->sx, s->sy, s->clicks, t_us);
    s->button = 0;
}

static bool touching(const Pen *s, const Opt *o) {
    if (!s->near || o->hover_only) return false;
    // Since the start or a drop: keep what is posted until an event says.
    if (s->unsure) return s->button != 0;
    if (o->pressure > 0) return s->p >= (s->button ? o->release : o->pressure);
    return s->touch;
}

static void pen_frame(Pen *s, const Opt *o, const Map *m, Sink *k, uint64_t t) {
    s->frames++;
    if (s->on_frame) s->on_frame(s->on_frame_ctx, t);
    bool contact = touching(s, o);
    if (!s->have_x || !s->have_y) {
        if (!contact) pen_release(s, k, t);
        return;
    }
    double x, y;
    map_apply(m, s->x, s->y, &x, &y);
    bool moved = !s->placed || x != s->sx || y != s->sy;
    if (contact && !s->button) {
        int b = ((o->right & RIGHT_SIDE) && s->side) || ((o->right & RIGHT_ERASER) && s->rubber) ? 2 : 1;
        bool again = s->clicks && b == s->down_button && t - s->down_t <= CLICK_US &&
                     fabs(x - s->down_x) <= CLICK_PT && fabs(y - s->down_y) <= CLICK_PT;
        s->clicks = again ? s->clicks + 1 : 1;
        s->down_t = t, s->down_x = x, s->down_y = y, s->down_button = b;
        s->button = b;
        emit(k, OUT_DOWN, b, x, y, s->clicks, t);
    } else if (contact) {
        if (!moved) return;
        emit(k, OUT_DRAG, s->button, x, y, 0, t);
    } else if (s->button) {
        emit(k, OUT_UP, s->button, x, y, s->clicks, t);
        s->button = 0;
    } else {
        if (!s->near || !moved) return;
        emit(k, OUT_MOVE, 0, x, y, 0, t);
    }
    s->sx = x, s->sy = y, s->placed = true;
}

static void pen_event(Pen *s, const Opt *o, const Map *m, Sink *k, uint64_t t, unsigned type, unsigned code,
                      int32_t v) {
    s->events++;
    if (type == EV_ABS) {
        if (code == ABS_X) s->x = v, s->have_x = true, s->near = true;
        else if (code == ABS_Y) s->y = v, s->have_y = true, s->near = true;
        else if (code == ABS_PRESSURE) {
            s->p = v;
            // A pressure settles a doubt: the raw digitizer's is above 0 only while it touches.
            if (s->unsure) s->touch = v > 0, s->unsure = false;
        }
    } else if (type == EV_KEY) {
        if (code == BTN_TOOL_PEN || code == BTN_TOOL_RUBBER) {
            if (code == BTN_TOOL_PEN) s->pen = v != 0;
            else s->rubber = v != 0;
            // The tool coming or going brackets a touch (the Wacom tool protocol): either way the
            // pen is not touching unless this frame says so.
            s->near = s->pen || s->rubber;
            s->touch = false, s->p = 0, s->unsure = false;
            if (!s->near) s->side = false;
        } else if (code == BTN_TOUCH) s->touch = v != 0, s->unsure = false;
        else if (code == BTN_STYLUS || code == BTN_STYLUS2) s->side = v != 0;
    } else if (type == EV_SYN) {
        // The kernel threw away this reader's unread events (it fell behind by its 128): what
        // follows is the rest of a frame, then frames as usual. A button change may be among the
        // lost, and only the device could say what it holds now, which a stream cannot ask.
        if (code == SYN_DROPPED) s->drops++, s->unsure = true;
        else if (code == SYN_REPORT) pen_frame(s, o, m, k, t);
    }
}

static uint32_t le32(const uint8_t *b) {
    return (uint32_t)b[0] | (uint32_t)b[1] << 8 | (uint32_t)b[2] << 16 | (uint32_t)b[3] << 24;
}

static void pen_decode(Pen *s, const Opt *o, const Map *m, Sink *k, const uint8_t *b) {
    uint64_t t = (uint64_t)le32(b) * 1000000 + le32(b + 4);
    s->last_t = t;
    pen_event(s, o, m, k, t, (unsigned)(b[8] | b[9] << 8), (unsigned)(b[10] | b[11] << 8), (int32_t)le32(b + 12));
}

// Whatever a read returned: whole events at once, an event split across reads kept for the next.
static void pen_feed(Pen *s, const Opt *o, const Map *m, Sink *k, const uint8_t *b, size_t n) {
    if (s->n_part) {
        size_t take = (size_t)(EV_SIZE - s->n_part) < n ? (size_t)(EV_SIZE - s->n_part) : n;
        memcpy(s->part + s->n_part, b, take);
        s->n_part += (int)take, b += take, n -= take;
        if (s->n_part < EV_SIZE) return;
        pen_decode(s, o, m, k, s->part);
        s->n_part = 0;
    }
    for (; n >= EV_SIZE; b += EV_SIZE, n -= EV_SIZE) pen_decode(s, o, m, k, b);
    memcpy(s->part, b, n);
    s->n_part = (int)n;
}

// ---- time ----

static uint64_t now_us(void) { return clock_gettime_nsec_np(CLOCK_UPTIME_RAW) / 1000; }

static volatile sig_atomic_t stop;
static void on_signal(int sig) {
    (void)sig;
    stop = 1;
}

// Sleeps until a signal or the time; false when a signal came.
static bool nap_us(uint64_t us) {
    struct timespec ts = {(time_t)(us / 1000000), (long)(us % 1000000) * 1000};
    while (!stop && nanosleep(&ts, &ts) && errno == EINTR) {}
    return !stop;
}

// ---- --stats: how far each frame's delay is above the best of the last 5 seconds ----
// The stamps are the Kindle's clock, the arrivals the Mac's: their difference holds an unknown
// offset, which the window's least delay takes out (and the clocks' drift with it).

enum { STATS_N = 8192, STATS_US = 5000000 };
typedef struct {
    int64_t d[STATS_N];
    int n;
    uint64_t frames, start, drops0;
    Pen *pen;
} Stats;

static void stats_frame(void *ctx, uint64_t t_us) {
    Stats *st = ctx;
    if (st->n < STATS_N) st->d[st->n++] = (int64_t)st->pen->arrive_us - (int64_t)t_us;
    st->frames++;
}

static int cmp_i64(const void *a, const void *b) {
    int64_t x = *(const int64_t *)a, y = *(const int64_t *)b;
    return (x > y) - (x < y);
}

static void stats_tick(Stats *st, uint64_t now) {
    if (!st->start) st->start = now, st->drops0 = st->pen->drops;
    if (now - st->start < STATS_US) return;
    if (st->n) {
        qsort(st->d, (size_t)st->n, sizeof st->d[0], cmp_i64);
        int64_t lo = st->d[0];
        fprintf(stderr,
                "scribe-tablet: %.0f frames/s; delay over the best of these %.0f s: median %.1f ms, p99 %.1f ms, "
                "max %.1f ms; %llu drops\n",
                st->frames * 1e6 / (double)(now - st->start), (now - st->start) / 1e6,
                (st->d[st->n / 2] - lo) / 1e3, (st->d[st->n * 99 / 100] - lo) / 1e3, (st->d[st->n - 1] - lo) / 1e3,
                (unsigned long long)(st->pen->drops - st->drops0));
    }
    st->n = 0, st->frames = 0, st->start = now, st->drops0 = st->pen->drops;
}

// ---- replay pacing: a frame posted at its own time after the first ----

typedef struct {
    uint64_t first_t, start;
    bool started;
} Pace;

static void pace_frame(void *ctx, uint64_t t_us) {
    Pace *p = ctx;
    if (!p->started) {
        p->first_t = t_us, p->start = now_us(), p->started = true;
        return;
    }
    uint64_t due = p->start + (t_us > p->first_t ? t_us - p->first_t : 0), now = now_us();
    if (due > now) nap_us(due - now);
}

// ---- ssh ----

// eventN: a node, not a name.
static bool is_node(const char *d) {
    return !strncmp(d, "event", 5) && d[5] && strspn(d + 5, "0123456789") == strlen(d + 5);
}

// The command the Kindle runs: finds the device by its name in /proc/bus/input/devices (or takes
// an eventN as it is), stops the reader a dropped connection left behind (it would last until its
// next write; its pid is kept in /tmp and checked to still be a cat of an input node), and becomes
// `cat` of the node: busybox's writes each read at once, and evdev's read returns every whole
// event queued.
static char *remote_command(const Opt *o) {
    static char cmd[2048];
    const char *find =
        "d=$(awk -v n=\"$n\" '/^N: /{m=index($0, \"Name=\\\"\" n \"\\\"\")>0} "
        "m&&/^H: /{for(i=2;i<=NF;i++) if($i~/^event[0-9]+$/){print $i; exit}}' /proc/bus/input/devices)";
    bool node = is_node(o->device);
    snprintf(cmd, sizeof cmd,
             "n='%s'; %s; "
             "[ -n \"$d\" ] && [ -c /dev/input/$d ] || { echo \"scribe-tablet: the Kindle has no input device named $n\" >&2; exit %d; }; "
             "f=/tmp/scribe-tablet.pid; p=$(cat $f 2>/dev/null); "
             "[ -n \"$p\" ] && grep -q /dev/input/ /proc/$p/cmdline 2>/dev/null && kill $p; "
             "echo $$ > $f; echo \"scribe-tablet: reading /dev/input/$d ($n)\" >&2; exec cat /dev/input/$d",
             o->device, node ? "d=$n" : find, NO_DEVICE);
    return cmd;
}

static pid_t spawn_ssh(const Opt *o, const char *remote, int *out) {
    char port[16], *av[64 + 64];
    int n = 0, p[2] = {-1, -1};
    snprintf(port, sizeof port, "%d", o->port);
    av[n++] = "ssh";
    for (int i = 0; i < o->n_ssh_args && i < 64; i++) av[n++] = o->ssh_args[i];
    // No terminal (binary data), no forwarding, never a prompt (it would hang a reconnect); a
    // dead link is noticed in 6 s. No compression: it holds data back to fill its blocks. IPQoS ef
    // marks what the Mac sends (OpenSSH 10 takes lowdelay no more); the stream's own direction is
    // the Kindle's dropbear, which sets TCP_NODELAY on its listening socket, and Linux copies that
    // to each connection it accepts.
    const char *fixed[] = {"-T", "-x", "-a", "-o", "BatchMode=yes", "-o", "ConnectTimeout=5", "-o",
                           "ServerAliveInterval=2", "-o", "ServerAliveCountMax=3", "-o", "Compression=no",
                           "-o", "IPQoS=ef", "-p", port, "-l", o->user};
    for (size_t i = 0; i < sizeof fixed / sizeof *fixed; i++) av[n++] = (char *)fixed[i];
    if (o->bind) av[n++] = "-b", av[n++] = (char *)o->bind;
    av[n++] = (char *)o->host;
    av[n++] = (char *)remote;
    av[n] = NULL;
    posix_spawn_file_actions_t fa;
    posix_spawn_file_actions_init(&fa);
    posix_spawn_file_actions_addopen(&fa, 0, "/dev/null", O_RDONLY, 0);
    if (out) {
        if (pipe(p)) return -1;
        posix_spawn_file_actions_adddup2(&fa, p[1], 1);
        posix_spawn_file_actions_addclose(&fa, p[0]);
        posix_spawn_file_actions_addclose(&fa, p[1]);
    }
    pid_t pid;
    int err = posix_spawnp(&pid, "ssh", &fa, NULL, av, environ);
    posix_spawn_file_actions_destroy(&fa);
    if (out) close(p[1]);
    if (err) {
        fprintf(stderr, "scribe-tablet: cannot run ssh: %s\n", strerror(err));
        if (out) close(p[0]);
        return -1;
    }
    if (out) *out = p[0];
    return pid;
}

// ---- options ----

static void usage(FILE *f) {
    fputs("usage: scribe-tablet --host ADDR [options] [-- ssh options]\n"
          "       scribe-tablet --replay FILE [options]\n"
          "       scribe-tablet --bench N [options]\n"
          "The Kindle Scribe's pen as an absolute tablet, over SSH to KOReader's server (docs/TABLET.md).\n"
          "  --host ADDR        the Kindle's address\n"
          "  --port N           its SSH port (2222)\n"
          "  --user NAME        (root)\n"
          "  --bind ADDR        the Mac's address to connect from (ssh -b): the right interface\n"
          "  --device NAME      the pen's input device, by name or as eventN (" RAW_NAME ")\n"
          "  --range X,Y        the device's maxima (" RAW_NAME ": 15624,20832; " SCALED_NAME ": 1860,2480)\n"
          "  --flip-x, --flip-y reverse a device axis\n"
          "  --rotate DEG       the Kindle turned clockwise: 0 (portrait), 90 (top edge right), 180, 270\n"
          "  --panel-mm W,H     the panel's size, portrait (157.48,209.97: the 2022 and 2024 Scribes)\n"
          "  --area W[,H[,X,Y]] the pen area in mm as the Kindle is held: width, height (default: the\n"
          "                     screen's shape), centre (default: the panel's). Default: the largest\n"
          "                     area of the screen's shape, centred\n"
          "  --display N        the display, 0 the main one (default)\n"
          "  --screen X,Y,W,H   a rectangle in points instead (global coordinates, y down)\n"
          "  --pressure N       touch at this raw pressure (0..4095) instead of the pen's own touch\n"
          "  --hover-only       never click: the pen only moves the pointer (osu!: click with its keys)\n"
          "  --right MODE       what makes a touch the right button: both (side button or eraser),\n"
          "                     side, eraser, none (both)\n"
          "  --dry-run          print the events instead of posting them\n"
          "  --replay FILE      read a captured stream instead of ssh (in real time unless --dry-run)\n"
          "  --capture FILE     also write the stream as received to FILE\n"
          "  --stats            every 5 s: frames per second and how late they come\n"
          "  --bench N          time the parsing and mapping of N synthetic events\n"
          "  --list-devices     print the Kindle's input devices\n",
          f);
}

static bool parse_list(const char *s, double *v, int max, int *n) {
    char *end;
    *n = 0;
    for (;;) {
        if (*n == max) return false;
        v[(*n)++] = strtod(s, &end);
        if (end == s || !isfinite(v[*n - 1])) return false;
        if (!*end) return true;
        if (*end != ',') return false;
        s = end + 1;
    }
}

static bool parse_int(const char *s, long lo, long hi, long *v) {
    char *end;
    errno = 0;
    *v = strtol(s, &end, 10);
    return !errno && end != s && !*end && *v >= lo && *v <= hi;
}

static int bad(const char *opt, const char *arg) {
    fprintf(stderr, "scribe-tablet: bad %s: %s (see --help)\n", opt, arg);
    return 2;
}

static int parse_options(Opt *o, int argc, char **argv) {
    enum { O_HOST = 256, O_PORT, O_USER, O_BIND, O_DEVICE, O_RANGE, O_FLIPX, O_FLIPY, O_ROTATE, O_PANEL, O_AREA,
           O_DISPLAY, O_SCREEN, O_PRESSURE, O_HOVER, O_RIGHT, O_DRY, O_REPLAY, O_CAPTURE, O_STATS, O_BENCH,
           O_LIST };
    static const struct option longs[] = {
        {"host", 1, 0, O_HOST}, {"port", 1, 0, O_PORT}, {"user", 1, 0, O_USER}, {"bind", 1, 0, O_BIND},
        {"device", 1, 0, O_DEVICE}, {"range", 1, 0, O_RANGE}, {"flip-x", 0, 0, O_FLIPX}, {"flip-y", 0, 0, O_FLIPY},
        {"rotate", 1, 0, O_ROTATE}, {"panel-mm", 1, 0, O_PANEL}, {"area", 1, 0, O_AREA},
        {"display", 1, 0, O_DISPLAY}, {"screen", 1, 0, O_SCREEN}, {"pressure", 1, 0, O_PRESSURE},
        {"hover-only", 0, 0, O_HOVER}, {"right", 1, 0, O_RIGHT}, {"dry-run", 0, 0, O_DRY},
        {"replay", 1, 0, O_REPLAY}, {"capture", 1, 0, O_CAPTURE}, {"stats", 0, 0, O_STATS},
        {"bench", 1, 0, O_BENCH}, {"list-devices", 0, 0, O_LIST}, {"help", 0, 0, 'h'}, {0, 0, 0, 0}};
    *o = (Opt){.user = "root", .port = 2222, .device = RAW_NAME, .panel_w = PANEL_W_MM, .panel_h = PANEL_H_MM,
               .right = RIGHT_SIDE | RIGHT_ERASER};
    bool range_set = false;
    int c, n;
    long l;
    double v[4];
    while ((c = getopt_long(argc, argv, "h", longs, NULL)) != -1) {
        switch (c) {
        case O_HOST: o->host = optarg; break;
        case O_PORT:
            if (!parse_int(optarg, 1, 65535, &l)) return bad("--port", optarg);
            o->port = (int)l;
            break;
        case O_USER: o->user = optarg; break;
        case O_BIND: o->bind = optarg; break;
        case O_DEVICE:
            // It goes into the remote command in single quotes.
            if (!*optarg || strspn(optarg, "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 _.:-") !=
                                strlen(optarg))
                return bad("--device", optarg);
            o->device = optarg;
            break;
        case O_RANGE:
            if (!parse_list(optarg, v, 2, &n) || n != 2 || v[0] < 1 || v[1] < 1) return bad("--range", optarg);
            o->range_x = v[0], o->range_y = v[1], range_set = true;
            break;
        case O_FLIPX: o->flip_x = true; break;
        case O_FLIPY: o->flip_y = true; break;
        case O_ROTATE:
            if (!parse_int(optarg, 0, 270, &l) || l % 90) return bad("--rotate", optarg);
            o->rotate = (int)l;
            break;
        case O_PANEL:
            if (!parse_list(optarg, v, 2, &n) || n != 2 || v[0] <= 0 || v[1] <= 0) return bad("--panel-mm", optarg);
            o->panel_w = v[0], o->panel_h = v[1];
            break;
        case O_AREA:
            if (!parse_list(optarg, v, 4, &n) || n == 3 || v[0] <= 0 || (n > 1 && v[1] <= 0))
                return bad("--area", optarg);
            o->n_area = n, o->area_w = v[0], o->area_h = n > 1 ? v[1] : 0;
            if (n == 4) o->area_cx = v[2], o->area_cy = v[3];
            break;
        case O_DISPLAY:
            if (!parse_int(optarg, 0, 31, &l)) return bad("--display", optarg);
            o->display = (int)l;
            break;
        case O_SCREEN:
            if (!parse_list(optarg, v, 4, &n) || n != 4 || v[2] < 2 || v[3] < 2) return bad("--screen", optarg);
            o->scr_x = v[0], o->scr_y = v[1], o->scr_w = v[2], o->scr_h = v[3], o->screen_set = true;
            break;
        case O_PRESSURE:
            if (!parse_int(optarg, 1, 1 << 20, &l)) return bad("--pressure", optarg);
            // Up again below three quarters of it, so a pressure near the threshold does not chatter.
            o->pressure = (int)l, o->release = (int)(l * 3 / 4) > 0 ? (int)(l * 3 / 4) : 1;
            break;
        case O_HOVER: o->hover_only = true; break;
        case O_RIGHT:
            if (!strcmp(optarg, "both")) o->right = RIGHT_SIDE | RIGHT_ERASER;
            else if (!strcmp(optarg, "side")) o->right = RIGHT_SIDE;
            else if (!strcmp(optarg, "eraser")) o->right = RIGHT_ERASER;
            else if (!strcmp(optarg, "none")) o->right = 0;
            else return bad("--right", optarg);
            break;
        case O_DRY: o->dry_run = true; break;
        case O_REPLAY: o->replay = optarg; break;
        case O_CAPTURE: o->capture = optarg; break;
        case O_STATS: o->stats = true; break;
        case O_BENCH:
            if (!parse_int(optarg, 1, 1L << 34, &l)) return bad("--bench", optarg);
            o->bench = l;
            break;
        case O_LIST: o->list_devices = true; break;
        case 'h': usage(stdout); exit(0);
        default: return 2;
        }
    }
    o->ssh_args = argv + optind, o->n_ssh_args = argc - optind;
    if (o->n_ssh_args && (optind < 2 || strcmp(argv[optind - 1], "--"))) {
        fprintf(stderr, "scribe-tablet: unexpected argument: %s (ssh's options go after --)\n", argv[optind]);
        return 2;
    }
    if (o->n_ssh_args > 64) {
        fputs("scribe-tablet: too many ssh options\n", stderr);
        return 2;
    }
    if (!o->replay && !o->bench && !o->host) {
        fputs("scribe-tablet: --host is needed (or --replay or --bench); see --help\n", stderr);
        return 2;
    }
    if (!range_set) {
        if (!strcmp(o->device, RAW_NAME)) o->range_x = RAW_X, o->range_y = RAW_Y;
        else if (!strcmp(o->device, SCALED_NAME)) o->range_x = SCALED_X, o->range_y = SCALED_Y;
        else {
            fprintf(stderr, "scribe-tablet: the range of %s is not known: give --range X,Y\n", o->device);
            return 2;
        }
    }
    return 0;
}

// The screen rect, then the area (which takes the screen's shape where not given).
static int resolve_geometry(Opt *o) {
    if (!o->screen_set) {
        if (o->bench) {
            o->scr_w = 1920, o->scr_h = 1080;
        } else {
            CGDirectDisplayID ids[32];
            uint32_t n = 0;
            if (CGGetActiveDisplayList(32, ids, &n) || !n) {
                fputs("scribe-tablet: no display found: give --screen X,Y,W,H\n", stderr);
                return 1;
            }
            // The list is not in a promised order: the main display first, then the rest as listed.
            CGDirectDisplayID main = CGMainDisplayID(), order[32];
            uint32_t k = 0;
            order[k++] = main;
            for (uint32_t i = 0; i < n; i++)
                if (ids[i] != main) order[k++] = ids[i];
            if ((uint32_t)o->display >= k) {
                fprintf(stderr, "scribe-tablet: there is no display %d (%u active)\n", o->display, k);
                return 1;
            }
            CGRect r = CGDisplayBounds(order[o->display]);
            o->scr_x = r.origin.x, o->scr_y = r.origin.y, o->scr_w = r.size.width, o->scr_h = r.size.height;
        }
    }
    double pw, ph, aspect = o->scr_w / o->scr_h;
    held_panel(o, &pw, &ph);
    if (!o->n_area) {
        if (pw / ph > aspect) o->area_h = ph, o->area_w = ph * aspect;
        else o->area_w = pw, o->area_h = pw / aspect;
    } else if (o->n_area == 1) {
        o->area_h = o->area_w / aspect;
    }
    if (o->n_area < 4) o->area_cx = pw / 2, o->area_cy = ph / 2;
    if (o->area_cx - o->area_w / 2 < -0.01 || o->area_cx + o->area_w / 2 > pw + 0.01 ||
        o->area_cy - o->area_h / 2 < -0.01 || o->area_cy + o->area_h / 2 > ph + 0.01)
        fprintf(stderr, "scribe-tablet: the area runs off the panel (%.1f x %.1f mm as held): part of the screen "
                        "is out of reach\n", pw, ph);
    if (fabs(o->area_w / o->area_h - aspect) > 0.01 * aspect)
        fprintf(stderr, "scribe-tablet: the area's shape is not the screen's: the pointer moves faster one way\n");
    return 0;
}

static void describe(const Opt *o) {
    static const char *const rights[] = {"never", "side button", "eraser", "side button or eraser"};
    fprintf(stderr,
            "scribe-tablet: pen area %.1f x %.1f mm centred at %.1f, %.1f (rotated %d) onto %.0f x %.0f at %.0f, %.0f; "
            "%s; right button: %s\n",
            o->area_w, o->area_h, o->area_cx, o->area_cy, o->rotate, o->scr_w, o->scr_h, o->scr_x, o->scr_y,
            o->hover_only ? "hover only, no clicks" : o->pressure ? "touch by pressure" : "touch as the pen says",
            o->hover_only ? "never" : rights[o->right]);
}

// ---- the modes ----

// Synthetic frames like the raw digitizer's: hovering strokes and touching ones, 6 events a frame
// at 3 ms; events are written into b (n of them) and their frames and touches counted.
static void synth(uint8_t *b, long n, uint64_t *frames) {
    long i = 0, f = 0;
    *frames = 0;
    while (i < n) {
        uint32_t t_us = (uint32_t)(f * 3000);
        int phase = (int)(f % 400), x = (int)(1000 + (f * 37) % 13000), y = (int)(2000 + (f * 53) % 16000);
        int ev[6][3] = {{EV_ABS, ABS_X, x}, {EV_ABS, ABS_Y, y}, {EV_ABS, ABS_PRESSURE, phase < 200 ? 0 : 900 + phase},
                        {EV_ABS, 0x1a, phase - 200}, {EV_KEY, BTN_TOUCH, phase >= 200}, {EV_SYN, SYN_REPORT, 0}};
        for (int e = 0; e < 6 && i < n; e++, i++) {
            uint8_t *r = b + i * EV_SIZE;
            uint32_t sec = t_us / 1000000, usec = t_us % 1000000;
            memcpy(r, &sec, 4), memcpy(r + 4, &usec, 4);
            uint16_t ty = (uint16_t)ev[e][0], co = (uint16_t)ev[e][1];
            int32_t va = ev[e][2];
            memcpy(r + 8, &ty, 2), memcpy(r + 10, &co, 2), memcpy(r + 12, &va, 4);
        }
        f++;
        (*frames)++;
    }
}

static int run_bench(const Opt *o, const Map *m) {
    uint8_t *b = malloc((size_t)o->bench * EV_SIZE);
    if (!b) return 1;
    uint64_t frames;
    synth(b, o->bench, &frames);
    Pen s;
    pen_init(&s);
    Sink k = {.mode = SINK_COUNT};
    // Read-sized chunks, as a stream comes.
    uint64_t t0 = clock_gettime_nsec_np(CLOCK_UPTIME_RAW);
    size_t total = (size_t)o->bench * EV_SIZE;
    for (size_t at = 0; at < total; at += 4096) pen_feed(&s, o, m, &k, b + at, total - at < 4096 ? total - at : 4096);
    uint64_t ns = clock_gettime_nsec_np(CLOCK_UPTIME_RAW) - t0;
    printf("%ld events, %llu frames, %llu posted: %.1f ns per event, %.1f ns per frame (%.0f M events/s; "
           "checksum %.0f)\n",
           o->bench, (unsigned long long)frames, (unsigned long long)k.n, (double)ns / o->bench,
           (double)ns / (double)frames, o->bench / (ns / 1e3), k.sum);
    free(b);
    return 0;
}

static int run_replay(const Opt *o, const Map *m, Sink *k) {
    int fd = open(o->replay, O_RDONLY);
    if (fd < 0) {
        fprintf(stderr, "scribe-tablet: cannot open %s: %s\n", o->replay, strerror(errno));
        return 1;
    }
    Pen s;
    pen_init(&s);
    Pace pace = {0};
    if (!o->dry_run) s.on_frame = pace_frame, s.on_frame_ctx = &pace;
    uint8_t buf[4096];
    ssize_t r;
    while (!stop && (r = read(fd, buf, sizeof buf)) > 0) pen_feed(&s, o, m, k, buf, (size_t)r);
    close(fd);
    if (s.n_part) fprintf(stderr, "scribe-tablet: %d trailing bytes, not a whole event, left out\n", s.n_part);
    pen_release(&s, k, s.last_t);
    fflush(stdout);
    fprintf(stderr, "scribe-tablet: %llu events, %llu frames, %llu drops\n", (unsigned long long)s.events,
            (unsigned long long)s.frames, (unsigned long long)s.drops);
    return 0;
}

static int run_live(const Opt *o, const Map *m, Sink *k) {
    int cap = -1;
    if (o->capture && (cap = open(o->capture, O_WRONLY | O_CREAT | O_TRUNC, 0644)) < 0) {
        fprintf(stderr, "scribe-tablet: cannot write %s: %s\n", o->capture, strerror(errno));
        return 1;
    }
    const char *remote = remote_command(o);
    Pen s;
    pen_init(&s);
    static Stats st;
    if (o->stats) st.pen = &s, s.on_frame = stats_frame, s.on_frame_ctx = &st;
    uint64_t delay_s = 1;
    static uint8_t buf[65536];
    int status = 0;
    while (!stop) {
        int fd;
        pid_t pid = spawn_ssh(o, remote, &fd);
        if (pid < 0) return 1;
        uint64_t began = now_us();
        for (;;) {
            ssize_t r = read(fd, buf, sizeof buf);
            if (r > 0) {
                if (cap >= 0 && write(cap, buf, (size_t)r) != r) {
                    fprintf(stderr, "scribe-tablet: the capture failed: %s\n", strerror(errno));
                    close(cap), cap = -1;
                }
                s.arrive_us = now_us();
                pen_feed(&s, o, m, k, buf, (size_t)r);
                if (o->stats) stats_tick(&st, s.arrive_us);
                continue;
            }
            if (r < 0 && errno == EINTR && !stop) continue;
            break;
        }
        close(fd);
        pen_release(&s, k, s.last_t);
        pen_forget(&s);
        if (stop) kill(pid, SIGTERM);
        int ws;
        while (waitpid(pid, &ws, 0) < 0 && errno == EINTR) {}
        if (stop) break;
        int code = WIFEXITED(ws) ? WEXITSTATUS(ws) : -1;
        if (code == NO_DEVICE) {
            fprintf(stderr, "scribe-tablet: no such device on the Kindle (try --list-devices)\n");
            status = 1;
            break;
        }
        // Again in 1 s, then 2, 4 and 8 while it keeps failing; 1 again after a connection that lasted.
        if (now_us() - began > 10000000) delay_s = 1;
        fprintf(stderr, "scribe-tablet: the connection ended (ssh status %d); again in %llu s\n", code,
                (unsigned long long)delay_s);
        if (!nap_us(delay_s * 1000000)) break;
        delay_s = delay_s * 2 > 8 ? 8 : delay_s * 2;
    }
    if (cap >= 0) close(cap);
    return status;
}

static int run_list(const Opt *o) {
    pid_t pid = spawn_ssh(o, "grep -E '^(N: Name|H: Handlers)=' /proc/bus/input/devices", NULL);
    if (pid < 0) return 1;
    int ws;
    while (waitpid(pid, &ws, 0) < 0 && errno == EINTR) {}
    return WIFEXITED(ws) ? WEXITSTATUS(ws) : 1;
}

int main(int argc, char **argv) {
    Opt o;
    int rc = parse_options(&o, argc, argv);
    if (rc) return rc;
    if (o.list_devices) return run_list(&o);
    if (resolve_geometry(&o)) return 1;
    Map m;
    map_init(&m, &o);
    if (o.bench) return run_bench(&o, &m);
    describe(&o);

    struct sigaction sa = {.sa_handler = on_signal}; // no SA_RESTART: a read ends at a signal
    sigemptyset(&sa.sa_mask);
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGHUP, &sa, NULL);
    signal(SIGPIPE, SIG_IGN);
    pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);

    Sink k = {.mode = o.dry_run ? SINK_PRINT : SINK_POST};
    if (!o.dry_run) {
        // Posted events from a process the system does not trust are dropped without a word.
        const void *key = kAXTrustedCheckOptionPrompt, *yes = kCFBooleanTrue;
        CFDictionaryRef ask = CFDictionaryCreate(NULL, &key, &yes, 1, &kCFTypeDictionaryKeyCallBacks,
                                                 &kCFTypeDictionaryValueCallBacks);
        bool trusted = AXIsProcessTrustedWithOptions(ask);
        CFRelease(ask);
        if (!trusted) {
            fputs("scribe-tablet: this terminal may not control the pointer: allow it in System Settings, Privacy & "
                  "Security, Accessibility, then start it again\n",
                  stderr);
            return 1;
        }
        k.src = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
        // The trackpad and mouse keep working alongside the pen.
        if (k.src) CGEventSourceSetLocalEventsSuppressionInterval(k.src, 0);
    } else if (!o.replay) {
        setvbuf(stdout, NULL, _IOLBF, 0);
    }
    rc = o.replay ? run_replay(&o, &m, &k) : run_live(&o, &m, &k);
    if (k.src) CFRelease(k.src);
    return rc;
}
