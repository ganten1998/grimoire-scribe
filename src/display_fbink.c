// display_fbink.c: Kindle e-ink output through FBInk (MTK / Bellatrix3 on the Scribe 2).
//
// Pixels go straight into the mmap'ed framebuffer (FBInk's raw-data blitter is compiled out
// of MINIMAL builds, and a direct copy is the fastest path anyway). FBInk then issues the
// refresh ioctl with the waveform for the mode:
//   FAST -> DU   (in pen mode on MTK: the driver's path for handwriting, see below)
//   UI   -> GL16 (partial, no flash)
//   FULL -> GC16 + flash
// "Fast mode" (no automatic REAGL promotion) is enabled for the session like KOReader does on
// the Scribe, so large partial refreshes keep the waveform we ask for.
//
// Orientation, two layers:
//  1. Device rotation (display_set_orientation): the app draws a logical screen (portrait, or
//     landscape when the Kindle is turned sideways) that is mapped onto the panel.
//  2. Framebuffer quirk: on the Scribe 2 the framebuffer's memory can be upside down relative
//     to the touch/pen panel, depending on the rotation the Kindle UI left it in (the stock UI
//     renders rotated itself). Seen on device 2026-10-10: at fb rotate 1 every tap landed exactly
//     180 degrees from the drawn button. Override with an empty /mnt/us/grimoire/screen-flip or
//     /mnt/us/grimoire/screen-noflip.
//
// Colour panels (the Scribe Colorsoft: FBInk's has_color_panel, 32 bpp): the driver reads the
// framebuffer for an update some time after it was sent (about 23 ms, as KOReader's developers
// measured it on a Scribe Colorsoft; Grimoire has not run on one), in tiles. Pixels changed again
// before that show half old, striped. So after a fast
// refresh that is not live ink (a pressed button, the panel being dragged, a pinch, a slider)
// Grimoire waits until the driver has read the frame (MXCFB_WAIT_FOR_MDP_COMPLETION), as KOReader
// does. Live ink (display_live_ink: a pen stroke, erasing included) never waits, so pen reading
// never stalls: a stroke's frames only add ink the next one sends again, and erasing ends with a
// grayscale refresh of all it erased, which repaints anything a frame's read caught half done.
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>

#include "display.h"
#include "fbcheck.h"
#include "fbpack.h"
#include "fbink.h"
#include "latency.h"

static int g_fd = -1;
static FBInkConfig g_cfg;
static FBInkState g_st;
static uint8_t *g_fb;
static size_t g_fblen;
static Canvas g_previous;     // last logical frame, in physical order: +4.6 MB on the 2024 Scribe
static bool g_frame_known;
static bool g_readback;       // the mapping covers the whole panel at 8 or 32 bpp: the guards can read it
// MTK pen mode: the driver's path for handwriting. Measured on the owner's Scribe 2 (2026-10-26,
// docs/INK-PERFORMANCE.md): a live ink frame starts on the panel within 1 ms in pen mode, while
// plain DU frames over ink still on the panel queue in the driver, up to 300 ms with quick circles.
// FBInk notes that on Bellatrix3 small pen-mode DU updates skip the MDP (the image stage that also
// rotates) at 90/270 degree rotations, and the Scribe 2 runs at fb rotate 1 or 3: they still land
// in the right place. On by default on grayscale panels; /mnt/us/grimoire/pen-mode's first word
// overrides it: "off" for plain DU, "padded" to pad small updates to 6400 px so the MDP still takes
// them (beta.4's first form), anything else (an empty file too) the direct form. Colour panels,
// where Grimoire has never run, keep plain DU unless the file asks.
// The driver requires that pen-mode updates never collide with standard ones (FBInk's note): so a
// stroke enters pen mode only while no standard update can still be on the panel (the quiet thread
// below follows them), the driver stays in it until a grayscale refresh is due, and that refresh
// first waits for the pen's last update to end.
typedef enum { PEN_OFF, PEN_PADDED, PEN_DIRECT } PenCfg;
static const char *const k_pen_cfg[] = {"off", "padded", "direct"};
static PenCfg g_pen_cfg;
static bool g_pen_mode;           // the driver is in pen mode
static bool g_pen_ok = true;      // cleared if the driver refuses pen mode
static bool g_stroke_pen;         // the stroke being drawn asked for the pen path (display_pen_mode)
static uint32_t g_pen_marker, g_pen_last_ms;  // the last update sent in pen mode
static uint32_t g_pen_checked;    // with ink metrics both files are read again, every 2 s at most
static unsigned g_pen_busy;       // strokes that kept plain DU: a standard update was still on the panel
static bool g_pen_busy_noted;     // (this stroke counted)
// /mnt/us/grimoire/latency-ab (a device test, with ink metrics): strokes take turns in blocks of
// 12 at plain DU, direct pen mode and padded pen mode, so one session times all three (in the
// same conditions: the same pages, the same hand).
static bool g_ab;
static unsigned g_strokes;
static int g_count[3], g_fail, g_ok;
static uint32_t g_stats_at;
static uint32_t g_last_refresh;  // when the last refresh went out (paces the first-frame retries)
static uint64_t g_last_submit_us, g_copy_us, g_submit_us, g_copy_max_us, g_submit_max_us;
static unsigned g_perf_frames;
static uint32_t g_perf_at;
static bool g_perf_enabled;
// Ink metrics (latency.h): live ink frames timed from the pen, by how they went out.
enum { INK_DU, INK_PEN_DIRECT, INK_PEN_PADDED, INK_KINDS };
static const char *const k_ink_kind[] = {"DU", "pen mode", "padded pen mode"};
static LatHist g_lat_pen[INK_KINDS], g_lat_oldest[INK_KINDS], g_lat_raw[INK_KINDS], g_lat_hop[INK_KINDS];
static unsigned g_raw_miss[INK_KINDS];
static int g_presents;        // refreshes sent; the first few are logged in full (device bring-up)
static bool g_flip;           // framebuffer is 180 degrees from the panel's touch orientation
static int g_rot;             // device rotation, quarter turns clockwise
static int PW, PH;            // panel (framebuffer view) size
static int LW, LH;            // logical screen size (swapped when turned sideways)
// Not in FBInk's eink/mtk-kindle.h: blocks until the driver has read the framebuffer for the
// update with this marker (KOReader uses it on the Scribe Colorsoft's FW 5.19.6 kernels).
#define MXCFB_WAIT_FOR_MDP_COMPLETION _IOW('F', 0x4D, uint32_t)
static bool g_mdp_wait;       // a colour panel: fast refreshes outside live ink wait for the driver's read
static int g_mdp_logged;      // waits logged (the first few, for a tester's log)
static LiveInk g_ink;         // strokes being drawn (display_live_ink): their fast frames never wait

// Logical screen pixel -> framebuffer pixel (device rotation, then the framebuffer quirk).
static void to_fb(int lx, int ly, int *fx, int *fy) {
    int px, py;
    switch (g_rot) {
    case 1: px = ly; py = PH - 1 - lx; break;
    case 2: px = PW - 1 - lx; py = PH - 1 - ly; break;
    case 3: px = PW - 1 - ly; py = lx; break;
    default: px = lx; py = ly; break;
    }
    if (g_flip) {
        px = PW - 1 - px;
        py = PH - 1 - py;
    }
    *fx = px;
    *fy = py;
}

// Byte offset of logical pixel (lx, ly) in the framebuffer: one byte per pixel at 8 bpp, four at
// 32 bpp (the Scribe Colorsoft).
static size_t fb_index(int lx, int ly) {
    int fx, fy;
    to_fb(lx, ly, &fx, &fy);
    return fbcheck_offset(fx, fy, g_st.scanline_stride, (int)g_st.bpp);
}

static bool g_invert;  // Grimoire's colors inverted (user setting), on top of the fb's own flag

static uint64_t clock_us(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 0;
    return (uint64_t)ts.tv_sec * 1000000u + (uint64_t)ts.tv_nsec / 1000u;
}

// The gray the panel's memory holds at logical (lx, ly), Grimoire's own inversion undone: what the
// canvas has there unless another process drew. Only while g_readback.
static uint8_t fb_gray(int lx, int ly) {
    bool flip = g_st.bpp == 8 ? g_st.inverted_grayscale != g_invert : g_invert;  // as copy_rect writes
    return fbcheck_gray(g_fb + fb_index(lx, ly), (int)g_st.bpp, flip);
}

static const char *pixel_format_name(unsigned f) {
    switch (f) {
    case FBINK_PXFMT_Y4: return "Y4";
    case FBINK_PXFMT_Y8: return "Y8";
    case FBINK_PXFMT_BGR565: return "BGR565";
    case FBINK_PXFMT_RGB565: return "RGB565";
    case FBINK_PXFMT_BGR24: return "BGR24";
    case FBINK_PXFMT_RGB24: return "RGB24";
    case FBINK_PXFMT_BGRA: return "BGRA";
    case FBINK_PXFMT_RGBA: return "RGBA";
    case FBINK_PXFMT_BGR32: return "BGR32";
    case FBINK_PXFMT_RGB32: return "RGB32";
    default: return "unknown";
    }
}

// Logical rect -> the framebuffer rect it covers.
static Rect fb_rect(Rect r) {
    int x0, y0, x1, y1;
    to_fb(r.x, r.y, &x0, &y0);
    to_fb(r.x + r.w - 1, r.y + r.h - 1, &x1, &y1);
    int lx = x0 < x1 ? x0 : x1, ly = y0 < y1 ? y0 : y1;
    return (Rect){lx, ly, abs(x1 - x0) + 1, abs(y1 - y0) + 1};
}

// The driver's waits, on an fd of their own (the thread below), in its marker protocol. Never the
// submission wait, and never two waits on one of a marker's signals. The driver (hwtcon_v2) ends an
// update by signalling each of its markers twice, submitted and then complete, and a marker that
// still has waiters is freed by the last one to leave: a submission waiter woken by the first
// signal can free it before the second, and the kernel panics (the Kindle reboots on its watchdog;
// the latency report's submission wait did this). Each signal wakes one waiter, so a second waiter
// on one sits out the driver's 5 s timeout. (mdp_wait, on colour panels, waits on a signal of its
// own.)
struct grim_marker_data {
    uint32_t update_marker, collision_test;
};
#define GRIM_WAIT_FOR_UPDATE_COMPLETE _IOWR('F', 0x2F, struct grim_marker_data)
static int wait_done(int fd, uint32_t marker) {
    struct grim_marker_data d = {marker, 0};
    return ioctl(fd, GRIM_WAIT_FOR_UPDATE_COMPLETE, &d);
}
static bool start_thread(void *(*fn)(void *)) {
    pthread_t t;
    if (pthread_create(&t, NULL, fn, NULL) != 0) return false;
    pthread_detach(t);
    return true;
}

// Standard updates still on the panel, for pen mode's no-collision rule. A thread waits for each
// grayscale one to end, in order (their waveforms differ in length), and for the newest standard DU
// one (DU updates all last as long, so the newest ends last). It only waits and keeps books: no
// allocation, no log, nothing a bgjob fork could find half done.
#define QUIET_GRAY 32
static struct {
    pthread_mutex_t mu;
    pthread_cond_t cv, idle;  // work for the thread; it finished some (display_wait)
    bool on;
    int fd, errors;
    uint32_t gray[QUIET_GRAY];
    int gray_head, gray_n;
    uint32_t overflow_ms;  // the queue was full: unknown until 2 s later
    uint32_t du_sent, du_wait, du_done;
    uint32_t handed;  // the newest marker it took on
} Q = {.mu = PTHREAD_MUTEX_INITIALIZER, .cv = PTHREAD_COND_INITIALIZER, .fd = -1};

static void *quiet_main(void *arg) {
    (void)arg;
    pthread_mutex_lock(&Q.mu);
    for (;;) {
        while (!Q.gray_n && !Q.du_wait) pthread_cond_wait(&Q.cv, &Q.mu);
        bool gray = Q.gray_n > 0;
        uint32_t m = gray ? Q.gray[Q.gray_head] : Q.du_wait;
        if (!gray) Q.du_wait = 0;
        pthread_mutex_unlock(&Q.mu);
        int rc = wait_done(Q.fd, m);
        if (rc < 0) usleep(1000 * 1000);  // no word from the driver: give the update its longest time
        pthread_mutex_lock(&Q.mu);
        if (rc < 0 && ++Q.errors >= 3) Q.on = false;  // pen mode off for the session (logged by the main thread)
        if (gray) {
            Q.gray_head = (Q.gray_head + 1) % QUIET_GRAY;
            Q.gray_n--;
        } else {
            Q.du_done = m;
        }
        pthread_cond_broadcast(&Q.idle);
    }
    return NULL;
}

static void quiet_sent(uint32_t marker, bool gray) {
    if (Q.fd < 0) return;
    pthread_mutex_lock(&Q.mu);
    if (!gray) {
        Q.du_sent = Q.du_wait = Q.handed = marker;
    } else if (Q.gray_n < QUIET_GRAY) {
        Q.gray[(Q.gray_head + Q.gray_n) % QUIET_GRAY] = Q.handed = marker;
        Q.gray_n++;
    } else {
        Q.overflow_ms = now_ms() | 1;
    }
    pthread_cond_signal(&Q.cv);
    pthread_mutex_unlock(&Q.mu);
}

// No standard update can still be on the panel.
static bool quiet_now(void) {
    pthread_mutex_lock(&Q.mu);
    bool quiet = Q.on && !Q.gray_n && Q.du_done == Q.du_sent && (!Q.overflow_ms || now_ms() - Q.overflow_ms > 2000);
    pthread_mutex_unlock(&Q.mu);
    return quiet;
}

// A thread with its own fd on the framebuffer (FBInk's stays the main thread's); false if either fails.
static bool open_watch(int *fd, bool *on, void *(*fn)(void *)) {
    *fd = open("/dev/fb0", O_RDWR | O_CLOEXEC);
    if (*fd < 0) return false;
    *on = true;
    if (start_thread(fn)) return true;
    *on = false;
    close(*fd);
    *fd = -1;
    return false;
}

static void lat_reset(void) {
    for (int k = 0; k < INK_KINDS; k++) {
        lat_hist_init(&g_lat_pen[k], 100);
        lat_hist_init(&g_lat_oldest[k], 100);
        lat_hist_init(&g_lat_raw[k], 100);
        lat_hist_init(&g_lat_hop[k], 100);
        g_raw_miss[k] = 0;
    }
}

// The panel's temperature as the driver reads it ("temp:25 temp_zone:8"): the waveforms' lengths
// follow it.
static void panel_temp(char *out, size_t cap) {
    snprintf(out, cap, "?");
    FILE *f = fopen("/proc/hwtcon/temp", "r");
    if (!f) return;
    if (fgets(out, (int)cap, f)) out[strcspn(out, "\r\n")] = 0;
    fclose(f);
}

static void lat_log(void) {
    char a[96], b[96], c[96], d[96];
    for (int k = 0; k < INK_KINDS; k++) {
        if (!g_lat_pen[k].n) continue;
        lat_hist_format(&g_lat_pen[k], a, sizeof a);
        lat_hist_format(&g_lat_oldest[k], b, sizeof b);
        lat_hist_format(&g_lat_raw[k], c, sizeof c);
        lat_hist_format(&g_lat_hop[k], d, sizeof d);
        qlog("display: ink latency, %s: pen->submit %s; oldest pen->submit %s; raw->submit %s; relay hop %s; "
             "unmatched %u", k_ink_kind[k], a, b, c, d, g_raw_miss[k]);
    }
    char t[48];
    panel_temp(t, sizeof t);
    pthread_mutex_lock(&Q.mu);
    int quiet_errors = Q.errors;
    pthread_mutex_unlock(&Q.mu);
    qlog("display: driver: panel %s; strokes kept on plain DU %u; standard-update waits failed %d%s", t, g_pen_busy,
         quiet_errors, quiet_errors >= 3 ? " (no pen mode)" : "");
    lat_reset();
}

void display_set_orientation(int turns, int *w, int *h) {
    g_frame_known = false;
    g_rot = turns & 3;
    LW = (g_rot & 1) ? PH : PW;
    LH = (g_rot & 1) ? PW : PH;
    qlog("display: orientation %d quarter turns -> logical %dx%d", g_rot, LW, LH);
    *w = LW;
    *h = LH;
}

static void set_pen_mode(bool on) {
    if (!g_st.is_mtk || !g_pen_ok || on == g_pen_mode) return;
    int rc = fbink_mtk_toggle_pen_mode(g_fd, on);
    if (g_presents < 400) qlog("display: pen mode %s -> %d", on ? "on" : "off", rc);
    if (rc < 0) {
        qlog("display: pen mode %s refused (%d), using plain DU", on ? "on" : "off", rc);
        g_pen_ok = false;
        g_pen_mode = false;
        return;
    }
    g_pen_mode = on;
}

void display_live_ink(bool on) { live_ink_mark(&g_ink, on, now_ms()); }

static PenCfg read_pen_cfg(void) {
    FILE *f = fopen("/mnt/us/grimoire/pen-mode", "r");
    if (!f) return g_st.has_color_panel ? PEN_OFF : PEN_DIRECT;
    char w[16] = "";
    if (fscanf(f, "%15s", w) != 1) w[0] = 0;
    fclose(f);
    return !strcmp(w, "off") ? PEN_OFF : !strcmp(w, "padded") ? PEN_PADDED : PEN_DIRECT;
}

// A stroke of ink starts (on) or ends. Its fast frames take pen mode when it is on and no standard
// update can still be on the panel (display_present); the driver stays in pen mode past the stroke,
// until a grayscale refresh is due (pen_leave).
void display_pen_mode(bool on) {
    display_live_ink(on);
    g_stroke_pen = on;
    if (!on) return;
    g_strokes++;
    g_pen_busy_noted = false;
    PenCfg want = g_pen_cfg;
    if (g_perf_enabled && now_ms() - g_pen_checked >= 2000) {
        g_pen_checked = now_ms();
        bool ab = access("/mnt/us/grimoire/latency-ab", F_OK) == 0;
        if (ab != g_ab) qlog("display: latency-ab %s", ab ? "on: strokes take turns, 12 each" : "off");
        g_ab = ab;
        if (!g_ab) want = read_pen_cfg();
    }
    if (g_ab) {
        static const PenCfg turns[3] = {PEN_OFF, PEN_DIRECT, PEN_PADDED};
        want = turns[(g_strokes / 12) % 3];
    }
    if (want != g_pen_cfg) {
        qlog("display: pen mode %s (stroke %u)", k_pen_cfg[want], g_strokes);
        g_pen_cfg = want;
    }
}

// Before a standard update: out of pen mode, once the panel has finished the pen's last update (a
// standard one over it would collide). Grayscale refreshes come after a pause, so this rarely waits.
static void pen_leave(void) {
    if (!g_pen_mode) return;
    if (now_ms() - g_pen_last_ms < 2000) fbink_wait_for_complete(g_fd, g_pen_marker);
    set_pen_mode(false);
}

// After a fast refresh on a colour panel: wait until the driver has read the frame, so the next
// change to those pixels cannot show through half done. A driver that refuses the wait, or takes
// more than a second over it, is not asked again this session (logged once).
static void mdp_wait(void) {
    uint32_t marker = fbink_get_last_marker(), t0 = now_ms();
    int rc = ioctl(g_fd, MXCFB_WAIT_FOR_MDP_COMPLETION, &marker);
    uint32_t took = now_ms() - t0;
    if (rc < 0 && errno == EINTR) return;
    if (rc < 0 || took > 1000) {
        g_mdp_wait = false;
        qlog("display: MXCFB_WAIT_FOR_MDP_COMPLETION %s after %u ms (marker %u): no longer waiting for the colour "
             "panel's read", rc < 0 ? strerror(errno) : "took too long", took, marker);
        return;
    }
    if (g_mdp_logged < 20) {
        g_mdp_logged++;
        qlog("display: waited %u ms for the colour panel's driver to read fast refresh %u", took, marker);
    }
}

int display_open(int *w, int *h, float *dpi) {
    memset(&g_cfg, 0, sizeof g_cfg);
    g_cfg.is_quiet = true;
    g_fd = fbink_open();
    if (g_fd < 0) {
        qlog("display: fbink_open failed: %s", strerror(errno));
        return -1;
    }
    if (fbink_init(g_fd, &g_cfg) < 0) {
        qlog("display: fbink_init failed");
        fbink_close(g_fd);
        g_fd = -1;
        return -1;
    }
    fbink_get_state(&g_cfg, &g_st);
    g_fb = fbink_get_fb_pointer(g_fd, &g_fblen);
    qlog("display: %s (%s) %ux%u %ubpp stride %u rota %u dpi %u mtk %d inverted %d", g_st.device_name,
         g_st.device_codename, g_st.screen_width, g_st.screen_height, g_st.bpp, g_st.scanline_stride, g_st.current_rota,
         g_st.screen_dpi, g_st.is_mtk, g_st.inverted_grayscale);
    qlog("display: FBInk device id 0x%X, platform %s, pixel format %s (%u), color panel %d, eclipse waveforms %d, "
         "hw invert %d", (unsigned)g_st.device_id, g_st.device_platform, pixel_format_name(g_st.pixel_format),
         (unsigned)g_st.pixel_format, g_st.has_color_panel, g_st.has_eclipse_wfm, g_st.can_hw_invert);
    // The driver's own description, untouched, for devices nobody here has seen (channel order,
    // a second buffer, the physical size).
    struct fb_var_screeninfo v;
    struct fb_fix_screeninfo f;
    memset(&v, 0, sizeof v);
    memset(&f, 0, sizeof f);
    bool have_v = ioctl(g_fd, FBIOGET_VSCREENINFO, &v) == 0;
    if (have_v)
        qlog("display: vinfo %ux%u virtual %ux%u offset %u,%u bpp %u grayscale %u red %u/%u green %u/%u blue %u/%u "
             "transp %u/%u rotate %u size %ux%u mm", v.xres, v.yres, v.xres_virtual, v.yres_virtual, v.xoffset,
             v.yoffset, v.bits_per_pixel, v.grayscale, v.red.offset, v.red.length, v.green.offset, v.green.length,
             v.blue.offset, v.blue.length, v.transp.offset, v.transp.length, v.rotate, v.width, v.height);
    else
        qlog("display: FBIOGET_VSCREENINFO failed: %s", strerror(errno));
    if (ioctl(g_fd, FBIOGET_FSCREENINFO, &f) == 0)
        qlog("display: finfo id \"%.16s\" line_length %u smem_len %u, mapped %zu bytes", f.id, f.line_length,
             f.smem_len, g_fblen);
    else
        qlog("display: FBIOGET_FSCREENINFO failed: %s", strerror(errno));
    if (!g_fb || (g_st.bpp != 8 && g_st.bpp != 32)) {
        qlog("display: unsupported framebuffer (bpp %u)", g_st.bpp);
        fbink_close(g_fd);
        g_fd = -1;
        return -1;
    }
    if (g_st.is_mtk) qlog("display: auto REAGL off -> %d", fbink_mtk_toggle_auto_reagl(g_fd, false));
    g_flip = g_st.current_rota == 1;  // FB_ROTATE_CW: observed upside down vs. the panel
    if (access("/mnt/us/grimoire/screen-flip", F_OK) == 0) g_flip = true;
    if (access("/mnt/us/grimoire/screen-noflip", F_OK) == 0) g_flip = false;
    qlog("display: fb rotate %u -> %s", g_st.current_rota, g_flip ? "drawing rotated 180 degrees" : "drawing as is");
    g_pen_cfg = read_pen_cfg();
    g_perf_enabled = access("/mnt/us/grimoire/ink-metrics", F_OK) == 0;
    g_ab = g_perf_enabled && access("/mnt/us/grimoire/latency-ab", F_OK) == 0;
    if (g_st.is_mtk)
        qlog("display: MTK pen mode %s", g_ab ? "taking turns with plain DU (latency-ab)"
                                       : g_pen_cfg == PEN_OFF ? "off (plain DU for live ink)"
                                       : g_pen_cfg == PEN_DIRECT ? "on" : "on, padded");
    if (g_st.is_mtk && (g_pen_cfg != PEN_OFF || g_ab || g_perf_enabled)) {
        // display_wait's deadline on the monotonic clock: a step of the wall clock (the Kindle's
        // time sync) must not end it early.
        pthread_condattr_t ca;
        pthread_condattr_init(&ca);
        pthread_condattr_setclock(&ca, CLOCK_MONOTONIC);
        pthread_cond_init(&Q.idle, &ca);
        pthread_condattr_destroy(&ca);
        if (!open_watch(&Q.fd, &Q.on, quiet_main))
            qlog("display: no thread to follow standard updates (%s): no pen mode", strerror(errno));
    }
    if (g_perf_enabled) lat_reset();
    g_mdp_wait = g_st.has_color_panel && g_st.bpp == 32;
    if (g_mdp_wait)
        qlog("display: colour panel: fast refreshes other than live ink wait until the driver has read the frame");
    PW = LW = (int)g_st.screen_width;
    PH = LH = (int)g_st.screen_height;
    g_frame_known = false;
    if (canvas_init(&g_previous, PW, PH) != 0)
        qlog("display: no memory for the live pixel cache; dithering whole fast rectangles");
    *w = PW;
    *h = PH;
    // The screen guard reads the panel back; it must stay inside the mapping.
    g_readback = PW > 0 && PH > 0 &&
                 fbcheck_offset(PW - 1, PH - 1, g_st.scanline_stride, (int)g_st.bpp) + fbcheck_px_bytes((int)g_st.bpp) <=
                     g_fblen;
    if (!g_readback)
        qlog("display: the framebuffer mapping (%zu bytes) does not cover the panel: no readback, no screen guard",
             g_fblen);
    // FBInk never reports 0 dpi: for a device it does not know it guesses 167 (fbink.c), which would
    // lay a Scribe's UI out at about half size. Then the driver's physical size decides.
    DpiSource from;
    float d = fbcheck_dpi(g_st.device_name, g_st.screen_dpi, have_v ? v.xres : 0, have_v ? v.yres : 0,
                          have_v ? v.width : 0, have_v ? v.height : 0, &from);
    if (from == DPI_PANEL_SIZE)
        qlog("display: FBInk does not know this device (\"%s\", it guesses %u dpi); %ux%u px over %ux%u mm makes "
             "%.0f dpi", g_st.device_name, g_st.screen_dpi, v.xres, v.yres, v.width, v.height, d);
    else if (from == DPI_DEFAULT)
        qlog("display: FBInk does not know this device (\"%s\", it guesses %u dpi) and the driver gives no usable "
             "size (%ux%u mm); using %.0f dpi", g_st.device_name, g_st.screen_dpi, v.width, v.height, d);
    *dpi = d;
    return 0;
}

// DU is 1-bit. Changed pixels in live (FAST) frames are ordered-dithered (8x8 Bayer), so a gray shows as the same
// overall darkness in black and white dots: the stroke looks as wide and as dark while drawing
// as after the GL16 settle on pen-up. (Thresholding made light pencil vanish; forcing every inked
// pixel black made live strokes look thick and dark, then "fade" when they settled.) Over gray
// paper the dots are black and the paper itself: fbpack_row only moves a shown pixel toward its
// new value, so no pixel goes white and then black a waveform later.
static bool g_live;
// Logical area pushed with DU (dithered) and not yet repainted in grayscale: the framebuffer
// there legitimately differs from the canvas, so the overwrite checks skip it.
static Rect g_live_area;
static Rect g_overlay;  // display_skip_overlay
void display_skip_overlay(Rect r) { g_overlay = r; }
void display_set_invert(bool on) {
    if (on != g_invert) g_frame_known = false;
    g_invert = on;
}

// A full-screen copy takes 45-100 ms on the Kindle (fbpack_row, 20-40 ns a pixel): the pens are read
// every 64 rows of it (work_pump, common.h), which a stroke's small frames never reach.
static void copy_rect(const Canvas *s, Rect r) {
    size_t stride = g_st.scanline_stride;
    size_t px_bytes = g_st.bpp == 8 ? 1 : 4;
    for (int y = r.y; y < r.y + r.h; y++) {
        const uint8_t *src = s->px + (size_t)y * s->stride + r.x;
        // Along a logical row the framebuffer moves by a constant step (a pixel or a whole
        // scanline, either direction, depending on the rotation).
        int fx0, fy0, fx1, fy1;
        to_fb(r.x, y, &fx0, &fy0);
        to_fb(r.x + 1, y, &fx1, &fy1);
        ptrdiff_t step = (ptrdiff_t)(fy1 - fy0) * (ptrdiff_t)stride + (ptrdiff_t)(fx1 - fx0) * (ptrdiff_t)px_bytes;
        ptrdiff_t off = (ptrdiff_t)fy0 * (ptrdiff_t)stride + (ptrdiff_t)fx0 * (ptrdiff_t)px_bytes;
        ptrdiff_t last = off + step * (r.w - 1);
        if (off < 0 || last < 0 || (size_t)(off > last ? off : last) + px_bytes > g_fblen) return;
        uint8_t *previous = g_previous.px ? g_previous.px + (size_t)fy0 * g_previous.stride + fx0 : NULL;
        ptrdiff_t previous_step = (ptrdiff_t)(fy1 - fy0) * g_previous.stride + fx1 - fx0;
        bool flip_px = g_st.bpp == 8 ? g_st.inverted_grayscale != g_invert : g_invert;
        fbpack_row(g_fb + off, step, previous, previous_step, src, r.w, r.x, y, (int)g_st.bpp,
                    flip_px, g_live, g_frame_known);
        if (((y - r.y) & 63) == 63) work_pump();
    }
}

// Sends the refresh of framebuffer rect r and keeps the books: counts, failures, and whether the
// panel has taken any refresh at all (display_broken).
static int send_refresh(Rect r, RefreshMode mode, const FBInkConfig *rc) {
    uint64_t t0 = g_perf_enabled ? clock_us() : 0;
    int ret = fbink_refresh(g_fd, (uint32_t)r.y, (uint32_t)r.x, (uint32_t)r.w, (uint32_t)r.h, rc);
    if (g_perf_enabled) g_last_submit_us = clock_us() - t0;
    g_count[mode]++;
    if (ret < 0) g_fail++;
    else g_ok++;
    g_last_refresh = now_ms();
    if (now_ms() - g_stats_at > 30000) {
        g_stats_at = now_ms();
        qlog("display: refreshes so far: fast %d, ui %d, full %d, failed %d", g_count[0], g_count[1], g_count[2], g_fail);
    }
    if (ret < 0)
        qlog("display: refresh %d,%d %dx%d mode %d failed (%d): %s", r.x, r.y, r.w, r.h, mode, ret, strerror(errno));
    return ret;
}

void display_present(const Canvas *screen, Rect r, RefreshMode mode) {
    if (g_fd < 0) return;
    r = rect_intersect(r, (Rect){0, 0, LW, LH});
    r = rect_intersect(r, (Rect){0, 0, screen->w, screen->h});
    if (rect_is_empty(r)) return;
    // Pen mode only tolerates DU updates; anything else must leave it first (pen_leave: the only
    // wait, for the pen's last update, which grayscale settles find long done since they run after
    // a pause). No other wait for pending DU updates here: blocking stalls input reading, the
    // kernel's evdev buffer overflows and strokes get cut.
    if (mode != REFRESH_FAST) {
        pen_leave();
    } else if (g_stroke_pen && g_pen_cfg != PEN_OFF && !g_pen_mode && g_pen_ok) {
        if (quiet_now()) {
            set_pen_mode(true);
        } else if (!g_pen_busy_noted) {
            g_pen_busy++;
            g_pen_busy_noted = true;
        }
    }
    // The pen samples this frame draws are the ones emitted until now: taken before the copy, since
    // the copy pumps (copy_rect), and what it reads is drawn by the next frame.
    uint64_t newest = 0, oldest = 0, raw = 0;
    bool timed = g_perf_enabled && input_ink_times(&newest, &oldest, &raw);
    g_live = mode == REFRESH_FAST;
    uint64_t t0 = g_perf_enabled ? clock_us() : 0;
    copy_rect(screen, r);
    uint64_t copy_us = g_perf_enabled ? clock_us() - t0 : 0;
    g_live = false;
    if (mode == REFRESH_FAST) g_live_area = rect_union(g_live_area, r);
    else if (mode == REFRESH_FULL || rect_contains_rect(r, g_live_area)) g_live_area = RECT_EMPTY;
    FBInkConfig rc = g_cfg;
    rc.is_flashing = mode == REFRESH_FULL;
    rc.wfm_mode = mode == REFRESH_FAST ? WFM_DU : mode == REFRESH_UI ? WFM_GL16 : WFM_GC16;
    bool whole_frame = rect_contains_rect(r, (Rect){0, 0, LW, LH});
    r = mode == REFRESH_FULL ? (Rect){0, 0, PW, PH} : fb_rect(r);
    bool padded = g_pen_cfg != PEN_DIRECT;
    if (mode == REFRESH_FAST && g_pen_mode && padded && (g_st.current_rota & 1)) r = fbpack_pen_region(r, PW, PH);
    int ret = send_refresh(r, mode, &rc);
    uint32_t marker = fbink_get_last_marker();
    if (ret >= 0) {
        if (g_pen_mode) {
            g_pen_marker = marker;
            g_pen_last_ms = now_ms();
        } else {
            quiet_sent(marker, mode != REFRESH_FAST);
        }
    }
    bool ink = live_ink_frame(&g_ink, mode, now_ms());
    // (A present with no ink took the pen samples emitted before it all the same, untimed: a stroke's
    // last ones that never got a fast frame would otherwise wait to be the next stroke's "oldest".)
    if (mode == REFRESH_FAST && g_perf_enabled) {
        uint64_t sent_us = clock_us();
        if (ink && ret >= 0) {
            int kind = !g_pen_mode ? INK_DU : padded ? INK_PEN_PADDED : INK_PEN_DIRECT;
            if (timed) {
                lat_hist_add(&g_lat_pen[kind], (int64_t)(sent_us - newest));
                lat_hist_add(&g_lat_oldest[kind], (int64_t)(sent_us - oldest));
                if (raw) {
                    lat_hist_add(&g_lat_raw[kind], (int64_t)(sent_us - raw));
                    lat_hist_add(&g_lat_hop[kind], (int64_t)(newest - raw));
                } else {
                    g_raw_miss[kind]++;
                }
            }
        }
        g_copy_us += copy_us;
        g_submit_us += g_last_submit_us;
        if (copy_us > g_copy_max_us) g_copy_max_us = copy_us;
        if (g_last_submit_us > g_submit_max_us) g_submit_max_us = g_last_submit_us;
        g_perf_frames++;
        if (now_ms() - g_perf_at >= 10000) {
            qlog("display: fast timing (%u frames, pen %d): copy avg/max %llu/%llu us; submit avg/max %llu/%llu us",
                 g_perf_frames, g_pen_mode, (unsigned long long)(g_copy_us / g_perf_frames),
                 (unsigned long long)g_copy_max_us, (unsigned long long)(g_submit_us / g_perf_frames),
                 (unsigned long long)g_submit_max_us);
            lat_log();
            g_perf_at = now_ms();
            g_perf_frames = 0;
            g_copy_us = g_submit_us = g_copy_max_us = g_submit_max_us = 0;
        }
    }
    if (ret < 0) g_frame_known = false;
    else if (mode != REFRESH_FAST && whole_frame && g_previous.px) g_frame_known = true;
    if (ret >= 0 && mode == REFRESH_FAST && g_mdp_wait && !ink) mdp_wait();
    if (ret >= 0 && g_presents < 40)
        qlog("display: refresh #%d %d,%d %dx%d mode %d pen %d -> ok, marker %u, fb[center]=%u", g_presents, r.x, r.y,
             r.w, r.h, mode, g_pen_mode, fbink_get_last_marker(), g_readback ? fb_gray(LW / 2, LH / 2) : 0u);
    if (g_presents == 0 && g_readback) {
        if (g_st.bpp == 32) {
            // The raw bytes too: they show the channel order the driver really uses.
            const uint8_t *p = g_fb + fb_index(100, 191), *q = g_fb + fb_index(100, 600);
            qlog("display: readback after first frame: fb(100,191)=%u (expect ~0, the top rule) fb(100,600)=%u, bytes "
                 "%02x %02x %02x %02x and %02x %02x %02x %02x (Grimoire writes v v v ff)", fb_gray(100, 191),
                 fb_gray(100, 600), p[0], p[1], p[2], p[3], q[0], q[1], q[2], q[3]);
        } else {
            qlog("display: readback after first frame: fb(100,191)=%u (expect ~0, the top rule) fb(100,600)=%u",
                 fb_gray(100, 191), fb_gray(100, 600));
        }
    }
    g_presents++;
}

bool display_broken(void) {
    if (g_fd < 0 || g_ok > 0 || g_presents == 0) return false;
    if (fbcheck_broken(g_presents, g_ok)) return true;
    // Nothing has shown yet: refresh the whole panel again once a second (the framebuffer already
    // holds the frame), so a panel that takes nothing is known within seconds.
    if (now_ms() - g_last_refresh < 1000) return false;
    qlog("display: nothing has shown yet; refreshing the whole panel again (refresh %d of 5)", g_presents + 1);
    FBInkConfig rc = g_cfg;
    rc.is_flashing = true;
    rc.wfm_mode = WFM_GC16;
    send_refresh((Rect){0, 0, PW, PH}, REFRESH_FULL, &rc);
    g_presents++;
    return fbcheck_broken(g_presents, g_ok);
}

bool display_find_overwrite(const Canvas *screen, Rect skip, Rect *where) {
    *where = RECT_EMPTY;
    if (g_fd < 0 || !g_readback) return false;
    int w = screen->w < LW ? screen->w : LW;
    int h = screen->h < LH ? screen->h : LH;
    const int step = 24;
    int differ = 0, x0 = w, y0 = h, x1 = 0, y1 = 0;
    for (int y = step / 2; y < h; y += step)
        for (int x = step / 2; x < w; x += step) {
            if (rect_contains(skip, x, y) || rect_contains(g_live_area, x, y) || rect_contains(g_overlay, x, y))
                continue;
            uint8_t want = screen->px[(size_t)y * screen->stride + x];
            uint8_t have = fb_gray(x, y);
            if (abs((int)want - (int)have) <= 24) continue;
            differ++;
            if (x < x0) x0 = x;
            if (y < y0) y0 = y;
            if (x > x1) x1 = x;
            if (y > y1) y1 = y;
        }
    // A few stray samples are ink still settling; another window's strip is dozens.
    if (differ < 8) return false;
    *where = rect_intersect((Rect){x0 - step, y0 - step, x1 - x0 + 2 * step, y1 - y0 + 2 * step}, (Rect){0, 0, w, h});
    qlog("display: %d samples differ in %d,%d %dx%d (something drew there)", differ, where->x, where->y, where->w, where->h);
    return true;
}

bool display_band_overwritten(const Canvas *screen, Rect band, Rect skip) {
    if (g_fd < 0 || !g_readback) return false;
    band = rect_intersect(band, (Rect){0, 0, screen->w < LW ? screen->w : LW, screen->h < LH ? screen->h : LH});
    int differ = 0, total = 0;
    for (int y = band.y + 2; y < band.y + band.h; y += 6)
        for (int x = band.x + 2; x < band.x + band.w; x += 6) {
            if (rect_contains(skip, x, y) || rect_contains(g_live_area, x, y) || rect_contains(g_overlay, x, y))
                continue;
            uint8_t want = screen->px[(size_t)y * screen->stride + x];
            uint8_t have = fb_gray(x, y);
            total++;
            if (abs((int)want - (int)have) > 24) differ++;
        }
    if (total && differ > total / 100 + 2) {
        qlog("display: %d of %d samples in band %d,%d %dx%d differ (something drew there)", differ, total, band.x,
             band.y, band.w, band.h);
        return true;
    }
    return false;
}

// Until the last update has ended (before sleep and exit). One the quiet thread took on is waited
// for in its books, not at the driver: a second waiter on its marker would sit out the driver's 5 s.
void display_wait(void) {
    if (g_fd < 0) return;
    pthread_mutex_lock(&Q.mu);
    bool theirs = Q.fd >= 0 && fbink_get_last_marker() == Q.handed;
    if (theirs) {
        struct timespec until;
        clock_gettime(CLOCK_MONOTONIC, &until);
        until.tv_sec += 6;  // past the driver's own timeout
        while ((Q.gray_n || Q.du_done != Q.du_sent) && pthread_cond_timedwait(&Q.idle, &Q.mu, &until) == 0) {
        }
    }
    pthread_mutex_unlock(&Q.mu);
    if (!theirs) fbink_wait_for_complete(g_fd, LAST_MARKER);
}

void display_close(void) {
    if (g_fd < 0) return;
    pen_leave();
    if (g_st.is_mtk) fbink_mtk_toggle_auto_reagl(g_fd, true);
    fbink_close(g_fd);
    g_fd = -1;
    canvas_free(&g_previous);
    g_frame_known = false;
}
