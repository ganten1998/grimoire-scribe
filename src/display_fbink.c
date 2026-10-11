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
//     180 degrees from the drawn button. Override with an empty /mnt/us/quire/screen-flip or
//     /mnt/us/quire/screen-noflip.
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "display.h"
#include "fbink.h"

static int g_fd = -1;
static FBInkConfig g_cfg;
static FBInkState g_st;
static uint8_t *g_fb;
static size_t g_fblen;
static bool g_pen_mode;
static bool g_pen_ok = true;  // cleared if the driver refuses pen mode
// MTK pen mode is opt-in (empty file /mnt/us/quire/pen-mode): FBInk notes that on Bellatrix3
// small pen-mode DU updates skip the MDP (the rotation stage) at 90/270 degree rotations, and
// the Scribe 2 runs at fb rotate 1, so live ink could land nowhere visible. Plain DU partial
// updates are what KOReader uses for its pen input too.
static bool g_pen_wanted;
static RefreshMode g_last_mode = REFRESH_FULL;
static int g_count[3], g_fail;
static uint32_t g_stats_at;
static int g_presents;        // the first few refreshes are logged in full (device bring-up)
static bool g_flip;           // framebuffer is 180 degrees from the panel's touch orientation
static int g_rot;             // device rotation, quarter turns clockwise
static int PW, PH;            // panel (framebuffer view) size
static int LW, LH;            // logical screen size (swapped when turned sideways)

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

static size_t fb_index(int lx, int ly) {
    int fx, fy;
    to_fb(lx, ly, &fx, &fy);
    return (size_t)fy * g_st.scanline_stride + (size_t)fx;
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

void display_pen_mode(bool on) { set_pen_mode(on); }

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
    if (!g_fb || (g_st.bpp != 8 && g_st.bpp != 32)) {
        qlog("display: unsupported framebuffer (bpp %u)", g_st.bpp);
        fbink_close(g_fd);
        g_fd = -1;
        return -1;
    }
    if (g_st.is_mtk) qlog("display: auto REAGL off -> %d", fbink_mtk_toggle_auto_reagl(g_fd, false));
    g_flip = g_st.current_rota == 1;  // FB_ROTATE_CW: observed upside down vs. the panel
    if (access("/mnt/us/quire/screen-flip", F_OK) == 0) g_flip = true;
    if (access("/mnt/us/quire/screen-noflip", F_OK) == 0) g_flip = false;
    qlog("display: fb rotate %u -> %s", g_st.current_rota, g_flip ? "drawing rotated 180 degrees" : "drawing as is");
    g_pen_wanted = access("/mnt/us/quire/pen-mode", F_OK) == 0;
    qlog("display: MTK pen mode %s", g_pen_wanted ? "enabled (flag file)" : "off (plain DU for live ink)");
    PW = LW = (int)g_st.screen_width;
    PH = LH = (int)g_st.screen_height;
    *w = PW;
    *h = PH;
    *dpi = g_st.screen_dpi ? (float)g_st.screen_dpi : 300.f;
    return 0;
}

// DU is 1-bit. Live (FAST) frames are ordered-dithered (8x8 Bayer) so a gray shows as the same
// overall darkness in black and white dots: the stroke looks as wide and as dark while drawing
// as after the GL16 settle on pen-up. (Thresholding made light pencil vanish; forcing every inked
// pixel black made live strokes look thick and dark, then "fade" when they settled.)
static bool g_live;
static bool g_invert;  // Quire's colors inverted (user setting), on top of the fb's own flag
void display_set_invert(bool on) { g_invert = on; }
static const uint8_t k_bayer8[64] = {
    0,  32, 8,  40, 2,  34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4,  36, 14, 46,
    6,  38, 60, 28, 52, 20, 62, 30, 54, 22, 3,  35, 11, 43, 1,  33, 9,  41, 51, 19, 59, 27,
    49, 17, 57, 25, 15, 47, 7,  39, 13, 45, 5,  37, 63, 31, 55, 23, 61, 29, 53, 21};
static inline uint8_t live(uint8_t v, int x, int y) {
    if (!g_live || v == 0 || v == 255) return v;
    // Threshold in (2..254): a gray v lights up v/255 of the dots.
    int t = k_bayer8[((y & 7) << 3) | (x & 7)] * 4 + 2;
    return v > t ? 255 : 0;
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
        if (g_st.bpp == 8) {
            bool flip_px = g_st.inverted_grayscale != g_invert;
            if (step == 1 && !g_live && !flip_px) {
                memcpy(g_fb + off, src, (size_t)r.w);
                continue;
            }
            for (int x = 0; x < r.w; x++, off += step) {
                uint8_t v = live(src[x], r.x + x, y);
                g_fb[off] = flip_px ? (uint8_t)(255 - v) : v;
            }
        } else {
            for (int x = 0; x < r.w; x++, off += step) {
                uint8_t v = live(src[x], r.x + x, y);
                if (g_invert) v = (uint8_t)(255 - v);
                g_fb[off] = v;
                g_fb[off + 1] = v;
                g_fb[off + 2] = v;
                g_fb[off + 3] = 0xFF;
            }
        }
    }
}

void display_present(const Canvas *screen, Rect r, RefreshMode mode) {
    if (g_fd < 0) return;
    r = rect_intersect(r, (Rect){0, 0, LW, LH});
    r = rect_intersect(r, (Rect){0, 0, screen->w, screen->h});
    if (rect_is_empty(r)) return;
    // Pen mode only tolerates DU updates; anything else must leave it first. And let pending DU
    // updates finish before a grayscale update over them, so the settle is not merged away.
    if (mode != REFRESH_FAST) {
        set_pen_mode(false);
        if (g_last_mode == REFRESH_FAST) fbink_wait_for_complete(g_fd, LAST_MARKER);
    }
    g_live = mode == REFRESH_FAST;
    copy_rect(screen, r);
    g_live = false;
    g_last_mode = mode;
    FBInkConfig rc = g_cfg;
    rc.is_flashing = mode == REFRESH_FULL;
    rc.wfm_mode = mode == REFRESH_FAST ? WFM_DU : mode == REFRESH_UI ? WFM_GL16 : WFM_GC16;
    r = mode == REFRESH_FULL ? (Rect){0, 0, PW, PH} : fb_rect(r);
    int ret = fbink_refresh(g_fd, (uint32_t)r.y, (uint32_t)r.x, (uint32_t)r.w, (uint32_t)r.h, &rc);
    g_count[mode]++;
    if (ret < 0) g_fail++;
    if (now_ms() - g_stats_at > 30000) {
        g_stats_at = now_ms();
        qlog("display: refreshes so far: fast %d, ui %d, full %d, failed %d", g_count[0], g_count[1], g_count[2], g_fail);
    }
    if (ret < 0)
        qlog("display: refresh %d,%d %dx%d mode %d failed (%d): %s", r.x, r.y, r.w, r.h, mode, ret, strerror(errno));
    else if (g_presents < 40)
        qlog("display: refresh #%d %d,%d %dx%d mode %d pen %d -> ok, marker %u, fb[center]=%u", g_presents, r.x, r.y,
             r.w, r.h, mode, g_pen_mode, fbink_get_last_marker(),
             g_fb[fb_index(LW / 2, LH / 2)]);
    if (g_presents == 0)
        qlog("display: readback after first frame: fb(100,191)=%u (expect ~0, the top rule) fb(100,600)=%u",
             g_fb[fb_index(100, 191)], g_fb[fb_index(100, 600)]);
    g_presents++;
}

bool display_overwritten(const Canvas *screen) {
    if (g_fd < 0 || g_st.bpp != 8 || g_last_mode == REFRESH_FAST) return false;
    int w = screen->w < LW ? screen->w : LW;
    int h = screen->h < LH ? screen->h : LH;
    int differ = 0, total = 0;
    for (int gy = 1; gy < 48; gy++)
        for (int gx = 1; gx < 36; gx++) {
            int x = w * gx / 36, y = h * gy / 48;
            uint8_t want = screen->px[(size_t)y * screen->stride + x];
            uint8_t have = g_fb[fb_index(x, y)];
            if (g_st.inverted_grayscale != g_invert) have = (uint8_t)(255 - have);
            total++;
            if (abs((int)want - (int)have) > 8) differ++;
        }
    // A handful of mismatches can be ink in flight; a real overwrite changes a lot.
    // (The toast is overlaid only on the panel, never in the canvas: a few percent at most.)
    if (differ > total / 8) {
        qlog("display: framebuffer differs from Quire's screen at %d of %d samples (another process drew)", differ, total);
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
}
