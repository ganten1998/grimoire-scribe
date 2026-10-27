//! ffi_fbink: FBInk (third_party/FBInk, built for the Kindle only, FBINK_FOR_KINDLE + FBINK_MINIMAL)
//! through rust/csrc/fbink_shim.c, which keeps FBInk's own structs on the C side: display_fbink.rs
//! sees the fields display_fbink.c reads, flat (GrimFbState), and asks for a refresh by waveform.
//! Every call is unsafe; display_fbink.rs wraps them.
#![allow(non_camel_case_types)]

use libc::{c_char, c_int, c_uchar, size_t};

/// The shim's GrimFbState: FBInkState's fields that display_fbink.c reads. Its C twin is in
/// csrc/fbink_shim.c: change both together.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GrimFbState {
    pub screen_width: u32,
    pub screen_height: u32,
    pub scanline_stride: u32,
    pub bpp: u32,
    pub inverted_grayscale: bool,
    pub device_name: [c_char; 32],
    pub device_codename: [c_char; 32],
    pub device_platform: [c_char; 32],
    pub device_id: u32,
    pub screen_dpi: u32,
    pub is_mtk: bool,
    pub current_rota: u8,
    pub can_hw_invert: bool,
    pub has_eclipse_wfm: bool,
    pub has_color_panel: bool,
    pub pixel_format: u32,
}

/// The shim's waveforms, in RefreshMode's order (FAST: DU, UI: GL16, FULL: GC16).
pub const GRIM_WFM_DU: c_int = 0;
pub const GRIM_WFM_GL16: c_int = 1;
pub const GRIM_WFM_GC16: c_int = 2;
/// FBInk's LAST_MARKER: the marker of the last update this FBInk session sent.
pub const LAST_MARKER: u32 = 0;

extern "C" {
    pub fn grim_fbink_open() -> c_int;
    pub fn grim_fbink_close(fd: c_int) -> c_int;
    /// fbink_init with display_fbink.c's g_cfg (zeroed, is_quiet).
    pub fn grim_fbink_init(fd: c_int) -> c_int;
    pub fn grim_fbink_get_state(out: *mut GrimFbState);
    pub fn grim_fbink_get_fb_pointer(fd: c_int, len: *mut size_t) -> *mut c_uchar;
    /// fbink_refresh(fd, top, left, width, height) with g_cfg's copy: wfm (GRIM_WFM_*) and flashing.
    pub fn grim_fbink_refresh(fd: c_int, top: u32, left: u32, width: u32, height: u32, wfm: c_int, flashing: bool) -> c_int;
    pub fn grim_fbink_get_last_marker() -> u32;
    /// MXCFB_WAIT_FOR_UPDATE_COMPLETE on marker. Never call it on a marker another thread waits on
    /// (PORTING.md, The display's hard rules).
    pub fn grim_fbink_wait_for_complete(fd: c_int, marker: u32) -> c_int;
    pub fn grim_fbink_mtk_toggle_auto_reagl(fd: c_int, on: bool) -> c_int;
    pub fn grim_fbink_mtk_toggle_pen_mode(fd: c_int, on: bool) -> c_int;
}
