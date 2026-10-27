//! ffi_stb: the stb single-file libraries (third_party/stb_*.h), compiled by build.rs: stb_truetype
//! and stb_image_write from ../src/stb_impl.c as the C build compiles them, stb_image as
//! ../src/bgimage.c configures it (rust/csrc/stb_image_impl.c: PNG, JPEG and PNM only, at most 8192
//! px a side, no stdio, its allocations through grimoire_stb_* below). Only what Grimoire calls is
//! declared. Every call is unsafe (raw pointers into C); the modules that use them (font.rs,
//! bgimage.rs, store.rs, export.rs, live.rs, httpd.rs, sim_sdl.rs, the tools) wrap each in a safe
//! function with a SAFETY comment, and free what stb allocates with the matching free.
#![allow(non_camel_case_types, non_snake_case)]

use libc::{c_char, c_int, c_uchar, c_void, size_t};

// ---- stb_truetype ----

/// stbtt_fontinfo is opaque to Grimoire, but it lives in Grimoire's memory (font.c keeps one per
/// face): this is room for it, larger than stb_truetype.h's struct on any target (it holds a
/// pointer, a dozen ints and six stbtt__buf, under 200 bytes on 64-bit), aligned for its pointer.
/// font.rs must keep it at a fixed address once stbtt_InitFont has filled it (it holds no pointer
/// into itself, but keep it boxed anyway, as the C keeps it in a static).
#[repr(C, align(8))]
pub struct stbtt_fontinfo {
    _opaque: [u8; 256],
}

impl Default for stbtt_fontinfo {
    fn default() -> Self {
        stbtt_fontinfo { _opaque: [0; 256] }
    }
}

extern "C" {
    pub fn stbtt_InitFont(info: *mut stbtt_fontinfo, data: *const c_uchar, offset: c_int) -> c_int;
    pub fn stbtt_GetFontOffsetForIndex(data: *const c_uchar, index: c_int) -> c_int;
    pub fn stbtt_ScaleForPixelHeight(info: *const stbtt_fontinfo, pixels: f32) -> f32;
    pub fn stbtt_GetFontVMetrics(info: *const stbtt_fontinfo, ascent: *mut c_int, descent: *mut c_int, line_gap: *mut c_int);
    pub fn stbtt_FindGlyphIndex(info: *const stbtt_fontinfo, codepoint: c_int) -> c_int;
    pub fn stbtt_GetCodepointHMetrics(info: *const stbtt_fontinfo, codepoint: c_int, advance: *mut c_int, lsb: *mut c_int);
    pub fn stbtt_GetCodepointKernAdvance(info: *const stbtt_fontinfo, ch1: c_int, ch2: c_int) -> c_int;
    pub fn stbtt_GetCodepointBox(info: *const stbtt_fontinfo, codepoint: c_int, x0: *mut c_int, y0: *mut c_int, x1: *mut c_int, y1: *mut c_int) -> c_int;
    pub fn stbtt_GetCodepointBitmap(
        info: *const stbtt_fontinfo,
        scale_x: f32,
        scale_y: f32,
        codepoint: c_int,
        width: *mut c_int,
        height: *mut c_int,
        xoff: *mut c_int,
        yoff: *mut c_int,
    ) -> *mut c_uchar;
    pub fn stbtt_FreeBitmap(bitmap: *mut c_uchar, userdata: *mut c_void);
}

// ---- stb_image_write ----

extern "C" {
    /// In stb_image_write's implementation section but not its API header (the C declares it where
    /// it uses it: export.c, bgimage.c, live.c, httpd.c). Free the result with libc::free.
    pub fn stbi_write_png_to_mem(pixels: *const c_uchar, stride_bytes: c_int, x: c_int, y: c_int, n: c_int, out_len: *mut c_int) -> *mut c_uchar;
    /// The same: the zlib stream export.c's PDF images and zips use. Free with libc::free.
    pub fn stbi_zlib_compress(data: *mut c_uchar, data_len: c_int, out_len: *mut c_int, quality: c_int) -> *mut c_uchar;
    /// sim_sdl.c's shots, tools/mkicon.c and tools/mklogo.c.
    pub fn stbi_write_png(filename: *const c_char, w: c_int, h: c_int, comp: c_int, data: *const c_void, stride_bytes: c_int) -> c_int;
    /// A process-wide setting (live.c sets it in its viewer child: the fastest filter).
    pub static mut stbi_write_force_png_filter: c_int;
}

// ---- stb_image (csrc/stb_image_impl.c) ----

extern "C" {
    pub fn stbi_info_from_memory(buffer: *const c_uchar, len: c_int, x: *mut c_int, y: *mut c_int, comp: *mut c_int) -> c_int;
    pub fn stbi_is_16_bit_from_memory(buffer: *const c_uchar, len: c_int) -> c_int;
    /// The pixels come from grimoire_stb_malloc (below): free them with stbi_image_free.
    pub fn stbi_load_from_memory(buffer: *const c_uchar, len: c_int, x: *mut c_int, y: *mut c_int, comp: *mut c_int, req_comp: c_int) -> *mut c_uchar;
    pub fn stbi_image_free(retval_from_stbi_load: *mut c_void);
    pub fn stbi_failure_reason() -> *const c_char;
    /// sizeof(stbi__jpeg), for bgimage.c's decode cost (csrc/stb_image_impl.c).
    pub fn grimoire_stbi_jpeg_size() -> size_t;
    /// sizeof(stbtt_fontinfo) as the C compiler lays it out (the test below).
    pub fn grimoire_stbtt_fontinfo_size() -> size_t;
}

#[cfg(test)]
mod tests {
    #[test]
    fn fontinfo_room() {
        // SAFETY: a sizeof, no arguments.
        let n = unsafe { super::grimoire_stbtt_fontinfo_size() };
        assert!(n <= std::mem::size_of::<super::stbtt_fontinfo>(), "stbtt_fontinfo is {n} bytes");
    }
}

// stb_image's allocator (STBI_MALLOC, STBI_REALLOC, STBI_FREE in csrc/stb_image_impl.c): bgimage.c's
// stb_malloc, stb_realloc and stb_free, which count a decode's blocks against its budget. The
// accounting is bgimage.rs's (bgimage_stb_malloc and friends); these only cross the boundary.

#[no_mangle]
pub extern "C" fn grimoire_stb_malloc(n: size_t) -> *mut c_void {
    crate::bgimage::stb_malloc(n)
}

#[no_mangle]
pub extern "C" fn grimoire_stb_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    crate::bgimage::stb_realloc(p, n)
}

#[no_mangle]
pub extern "C" fn grimoire_stb_free(p: *mut c_void) {
    crate::bgimage::stb_free(p)
}
