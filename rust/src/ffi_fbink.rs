//! ffi_fbink: FBInk (third_party/FBInk, built for the Kindle only, FBINK_FOR_KINDLE + FBINK_MINIMAL)
//! through rust/csrc/fbink_shim.c, which keeps FBInk's own structs on the C side: display_fbink.rs
//! sees the fields display_fbink.c reads, flat (GrimFbState), and asks for a refresh by waveform.
//! Every call is unsafe; display_fbink.rs wraps them.
//!
//! Also the framebuffer driver's own description (<linux/fb.h>'s fb_var_screeninfo and
//! fb_fix_screeninfo), which display_fbink.c reads with ioctl for its log.
#![allow(non_camel_case_types)]

use libc::{c_char, c_int, c_uchar, c_ulong, size_t};

/// The shim's GrimFbState: FBInkState's fields that display_fbink.c reads. Its C twin is in
/// csrc/fbink_shim.c: change both together.
#[repr(C)]
#[derive(Clone, Copy, Default)]
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

/// FBInk's FBINK_PXFMT_INDEX_E (fbink.h): GrimFbState.pixel_format's values.
pub const FBINK_PXFMT_Y4: u32 = 4;
pub const FBINK_PXFMT_Y8: u32 = 8;
pub const FBINK_PXFMT_BGR565: u32 = 16;
pub const FBINK_PXFMT_RGB565: u32 = 17;
pub const FBINK_PXFMT_BGR24: u32 = 24;
pub const FBINK_PXFMT_RGB24: u32 = 25;
pub const FBINK_PXFMT_BGRA: u32 = 32;
pub const FBINK_PXFMT_RGBA: u32 = 33;
pub const FBINK_PXFMT_BGR32: u32 = 34;
pub const FBINK_PXFMT_RGB32: u32 = 35;

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

// ---- <linux/fb.h> ----

/// FBIOGET_VSCREENINFO and FBIOGET_FSCREENINFO: _IO-less numbers, as <linux/fb.h> has them.
pub const FBIOGET_VSCREENINFO: u32 = 0x4600;
pub const FBIOGET_FSCREENINFO: u32 = 0x4602;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct fb_bitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct fb_var_screeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: fb_bitfield,
    pub green: fb_bitfield,
    pub blue: fb_bitfield,
    pub transp: fb_bitfield,
    pub nonstd: u32,
    pub activate: u32,
    /// mm
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct fb_fix_screeninfo {
    pub id: [c_char; 16],
    pub smem_start: c_ulong,
    pub smem_len: u32,
    pub type_: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: c_ulong,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

// The kernel writes these whole: their sizes as <linux/fb.h> lays them out (the fixed one holds two
// unsigned longs, 68 bytes on the Kindle's 32-bit ARM, 80 on a 64-bit host).
const _: () = assert!(std::mem::size_of::<fb_var_screeninfo>() == 160);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(std::mem::size_of::<fb_fix_screeninfo>() == 68);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<fb_fix_screeninfo>() == 80);
