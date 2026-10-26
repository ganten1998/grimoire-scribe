// display_fbink.c: Kindle e-ink output through FBInk (MTK / Bellatrix3 on the Scribe 2).
//
// Pixels go straight into the mmap'ed framebuffer (FBInk's raw-data blitter is compiled out
// of MINIMAL builds, and a direct copy is the fastest path anyway). FBInk then issues the
// refresh ioctl with the waveform for the mode:
//   FAST -> DU   (pen mode on MTK: the driver's low-latency path for handwriting)
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
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>

#include "display.h"
#include "fbcheck.h"
#include "fbpack.h"
#include "fbink.h"

static int g_fd = -1;
static FBInkConfig g_cfg;
static FBInkState g_st;
static uint8_t *g_fb;
static size_t g_fblen;
static Canvas g_previous;     // last logical frame, in physical order: +4.6 MB on the 2024 Scribe
static bool g_frame_known;
static bool g_readback;       // the mapping covers the whole panel at 8 or 32 bpp: the guards can read it
static bool g_pen_mode;
static bool g_pen_ok = true;  // cleared if the driver refuses pen mode
// MTK pen mode is opt-in (empty file /mnt/us/grimoire/pen-mode): FBInk notes that on Bellatrix3
// small pen-mode DU updates skip the MDP (the rotation stage) at 90/270 degree rotations, and
// the Scribe 2 has used fb rotate 1 and 3. Opt-in updates below pad small refreshes above
// that threshold; physical behavior still needs testing on each model/firmware.
static bool g_pen_wanted;
static int g_count[3], g_fail, g_ok;
static uint32_t g_stats_at;
static uint32_t g_last_refresh;  // when the last refresh went out (paces the first-frame retries)
static uint64_t g_last_submit_us, g_copy_us, g_submit_us, g_copy_max_us, g_submit_max_us;
static unsigned g_perf_frames;
static uint32_t g_perf_at;
static bool g_perf_enabled;
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
    if (on && !g_pen_wanted) return;
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

void display_pen_mode(bool on) {
    display_live_ink(on);
    set_pen_mode(on);
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
    g_pen_wanted = access("/mnt/us/grimoire/pen-mode", F_OK) == 0;
    g_perf_enabled = access("/mnt/us/grimoire/ink-metrics", F_OK) == 0;
    qlog("display: MTK pen mode %s", g_pen_wanted ? "enabled (flag file)" : "off (plain DU for live ink)");
    if (g_pen_wanted && (g_st.current_rota & 1))
        qlog("display: odd-rotation pen updates keep at least 6400 pixels for coordinate processing");
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
// pixel black made live strokes look thick and dark, then "fade" when they settled.)
static bool g_live;
// Logical area pushed with DU (dithered) and not yet repainted in grayscale: the framebuffer
// there legitimately differs from the canvas, so the overwrite checks skip it.
static Rect g_live_area;
void display_set_invert(bool on) {
    if (on != g_invert) g_frame_known = false;
    g_invert = on;
}

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
    // Pen mode only tolerates DU updates; anything else must leave it first. And let pending DU
    // updates finish before a grayscale update over them, so the settle is not merged away.
    // (No wait for pending DU updates here: blocking stalls input reading, the kernel's evdev
    // buffer overflows and strokes get cut. Grayscale settles now only run after a pause.)
    if (mode != REFRESH_FAST) set_pen_mode(false);
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
    if (mode == REFRESH_FAST && g_pen_mode && (g_st.current_rota & 1)) r = fbpack_pen_region(r, PW, PH);
    int ret = send_refresh(r, mode, &rc);
    if (mode == REFRESH_FAST && g_perf_enabled) {
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
            g_perf_at = now_ms();
            g_perf_frames = 0;
            g_copy_us = g_submit_us = g_copy_max_us = g_submit_max_us = 0;
        }
    }
    if (ret < 0) g_frame_known = false;
    else if (mode != REFRESH_FAST && whole_frame && g_previous.px) g_frame_known = true;
    bool ink = live_ink_frame(&g_ink, mode, now_ms());
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
            if (rect_contains(skip, x, y) || rect_contains(g_live_area, x, y)) continue;
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
            if (rect_contains(skip, x, y) || rect_contains(g_live_area, x, y)) continue;
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

void display_wait(void) {
    if (g_fd >= 0) fbink_wait_for_complete(g_fd, LAST_MARKER);
}

void display_close(void) {
    if (g_fd < 0) return;
    set_pen_mode(false);
    if (g_st.is_mtk) fbink_mtk_toggle_auto_reagl(g_fd, true);
    fbink_close(g_fd);
    g_fd = -1;
    canvas_free(&g_previous);
    g_frame_known = false;
}
