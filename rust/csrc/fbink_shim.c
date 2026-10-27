// fbink_shim.c: the part of FBInk's API Grimoire uses (src/display_fbink.c), behind plain C types,
// so Rust never mirrors FBInk's large FBInkConfig and FBInkState structs (their layout is FBInk's to
// change). The shim keeps the one FBInkConfig the C keeps (g_cfg: zeroed, is_quiet), and each refresh
// copies it with the waveform and flash set, as display_fbink.c's present and first-frame retry do.
// No logic of Grimoire's is here: ffi_fbink.rs declares these, display_fbink.rs calls them.
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

#include "fbink.h"

static FBInkConfig g_cfg;

// FBInkState's fields that display_fbink.c reads, flat. ffi_fbink.rs mirrors this struct
// (#[repr(C)]): change both together.
typedef struct {
    uint32_t screen_width, screen_height, scanline_stride, bpp;
    bool inverted_grayscale;
    char device_name[32], device_codename[32], device_platform[32];
    uint32_t device_id;
    uint32_t screen_dpi;
    bool is_mtk;
    uint8_t current_rota;
    bool can_hw_invert, has_eclipse_wfm, has_color_panel;
    uint32_t pixel_format;
} GrimFbState;

int grim_fbink_open(void) { return fbink_open(); }

int grim_fbink_close(int fd) { return fbink_close(fd); }

// display_fbink.c's display_open: memset(&g_cfg, 0, ...), is_quiet, fbink_init.
int grim_fbink_init(int fd) {
    memset(&g_cfg, 0, sizeof g_cfg);
    g_cfg.is_quiet = true;
    return fbink_init(fd, &g_cfg);
}

void grim_fbink_get_state(GrimFbState *out) {
    FBInkState st;
    memset(&st, 0, sizeof st);
    fbink_get_state(&g_cfg, &st);
    memset(out, 0, sizeof *out);
    out->screen_width = st.screen_width;
    out->screen_height = st.screen_height;
    out->scanline_stride = st.scanline_stride;
    out->bpp = st.bpp;
    out->inverted_grayscale = st.inverted_grayscale;
    memcpy(out->device_name, st.device_name, sizeof out->device_name);
    memcpy(out->device_codename, st.device_codename, sizeof out->device_codename);
    memcpy(out->device_platform, st.device_platform, sizeof out->device_platform);
    out->device_name[sizeof out->device_name - 1] = 0;
    out->device_codename[sizeof out->device_codename - 1] = 0;
    out->device_platform[sizeof out->device_platform - 1] = 0;
    out->device_id = (uint32_t)st.device_id;
    out->screen_dpi = st.screen_dpi;
    out->is_mtk = st.is_mtk;
    out->current_rota = st.current_rota;
    out->can_hw_invert = st.can_hw_invert;
    out->has_eclipse_wfm = st.has_eclipse_wfm;
    out->has_color_panel = st.has_color_panel;
    out->pixel_format = (uint32_t)st.pixel_format;
}

unsigned char *grim_fbink_get_fb_pointer(int fd, size_t *len) { return fbink_get_fb_pointer(fd, len); }

// The waveforms display_fbink.c asks for, by Grimoire's RefreshMode order.
enum { GRIM_WFM_DU = 0, GRIM_WFM_GL16 = 1, GRIM_WFM_GC16 = 2 };

int grim_fbink_refresh(int fd, uint32_t top, uint32_t left, uint32_t width, uint32_t height, int wfm, bool flashing) {
    FBInkConfig rc = g_cfg;
    rc.is_flashing = flashing;
    rc.wfm_mode = wfm == GRIM_WFM_DU ? WFM_DU : wfm == GRIM_WFM_GL16 ? WFM_GL16 : WFM_GC16;
    return fbink_refresh(fd, top, left, width, height, &rc);
}

uint32_t grim_fbink_get_last_marker(void) { return fbink_get_last_marker(); }

int grim_fbink_wait_for_complete(int fd, uint32_t marker) { return fbink_wait_for_complete(fd, marker); }

int grim_fbink_mtk_toggle_auto_reagl(int fd, bool on) { return fbink_mtk_toggle_auto_reagl(fd, on); }

int grim_fbink_mtk_toggle_pen_mode(int fd, bool on) { return fbink_mtk_toggle_pen_mode(fd, on); }
