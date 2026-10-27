//! ffi_qr: Nayuki's QR Code generator (third_party/qrcodegen, MIT, vendored unmodified), compiled by
//! build.rs. Only what qr.c calls.
#![allow(non_camel_case_types, non_upper_case_globals)]

use libc::{c_char, c_int};

/// enum qrcodegen_Ecc
pub const qrcodegen_Ecc_MEDIUM: c_int = 1;
/// enum qrcodegen_Mask
pub const qrcodegen_Mask_AUTO: c_int = -1;

/// qrcodegen_BUFFER_LEN_FOR_VERSION(n): the bytes a symbol of version n takes.
pub const fn qrcodegen_buffer_len_for_version(n: usize) -> usize {
    ((n * 4 + 17) * (n * 4 + 17) + 7) / 8 + 1
}

extern "C" {
    /// text: NUL-terminated. temp_buffer and qrcode: at least
    /// qrcodegen_buffer_len_for_version(max_version) bytes each.
    pub fn qrcodegen_encodeText(
        text: *const c_char,
        temp_buffer: *mut u8,
        qrcode: *mut u8,
        ecl: c_int,
        min_version: c_int,
        max_version: c_int,
        mask: c_int,
        boost_ecl: bool,
    ) -> bool;
    pub fn qrcodegen_getSize(qrcode: *const u8) -> c_int;
    pub fn qrcodegen_getModule(qrcode: *const u8, x: c_int, y: c_int) -> bool;
}
