//! qr.h: QR codes on the screen (the Share screen's address, for a phone camera), drawn the way
//! e-ink shows best: whole-pixel square modules in pure black and white.

use crate::ffi_qr::*;
use crate::prelude::*;

// qr.c: QR codes with Nayuki's QR Code generator (third_party/qrcodegen, MIT, vendored unmodified
// from https://github.com/nayuki/QR-Code-generator at 3c6d0b3). ECC level M, raised when it costs
// no size, and at least version 3: every share address (http://<IPv4>:<port>, at most 28 bytes)
// then comes out as the same 29 x 29 symbol, so the layout never jumps with the address.

const QR_MIN_VERSION: i32 = 3;
const QR_MAX_VERSION: i32 = 10;
const QR_QUIET: i32 = 4;
const QR_BUF: usize = qrcodegen_buffer_len_for_version(QR_MAX_VERSION as usize);

/// Encodes text into qr (QR_BUF bytes); false when it cannot be encoded.
fn encode(text: &str, qr: &mut [u8; QR_BUF]) -> bool {
    let mut tmp = [0u8; QR_BUF];
    // The C passes a NUL-terminated string: the text ends at a NUL byte if it has one.
    let bytes = text.as_bytes();
    let cut = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let mut z = Vec::with_capacity(cut + 1);
    z.extend_from_slice(&bytes[..cut]);
    z.push(0);
    // SAFETY: z is NUL-terminated; tmp and qr are QR_BUF bytes each, the size qrcodegen asks for
    // at QR_MAX_VERSION, and it writes only inside them.
    unsafe {
        qrcodegen_encodeText(
            z.as_ptr() as *const libc::c_char,
            tmp.as_mut_ptr(),
            qr.as_mut_ptr(),
            qrcodegen_Ecc_MEDIUM,
            QR_MIN_VERSION,
            QR_MAX_VERSION,
            qrcodegen_Mask_AUTO,
            true,
        )
    }
}

fn get_size(qr: &[u8; QR_BUF]) -> i32 {
    // SAFETY: qr holds a symbol qrcodegen_encodeText made.
    unsafe { qrcodegen_getSize(qr.as_ptr()) }
}

fn get_module(qr: &[u8; QR_BUF], x: i32, y: i32) -> bool {
    // SAFETY: qr holds a symbol qrcodegen_encodeText made; x and y are inside it (the callers
    // stay below its size, as qrcodegen_getModule requires).
    unsafe { qrcodegen_getModule(qr.as_ptr(), x, y) }
}


/// Modules across the symbol for text, its quiet zone included (37 for every share address), or
/// 0 when the text cannot be encoded. A box of that many times m pixels gets modules of m pixels.
pub fn ui_qr_modules(text: &str) -> i32 {
    let mut qr = [0u8; QR_BUF];
    if encode(text, &mut qr) {
        get_size(&qr) + 2 * QR_QUIET
    } else {
        0
    }
}

/// Draws text as a QR code centered in box: square modules of the largest whole number of pixels
/// that fits, inside a quiet zone of 4 light modules. negative draws the exact inverse, for the
/// Invert setting (it inverts the panel at display time, so the code still shows dark on light).
/// Returns the symbol's rect, quiet zone included, or RECT_EMPTY (nothing drawn) when the text
/// cannot be encoded or the box is too small.
pub fn ui_qr(c: &mut Canvas, box_: Rect, text: &str, negative: bool) -> Rect {
    let mut qr = [0u8; QR_BUF];
    if !encode(text, &mut qr) {
        return RECT_EMPTY;
    }
    let n = get_size(&qr);
    let across = n + 2 * QR_QUIET;
    let m = (if box_.w < box_.h { box_.w } else { box_.h }) / across;
    if m < 1 {
        return RECT_EMPTY;
    }
    let side = across * m;
    let sym = Rect { x: box_.x + (box_.w - side) / 2, y: box_.y + (box_.h - side) / 2, w: side, h: side };
    let light: u8 = if negative { 0 } else { 255 };
    let dark: u8 = if negative { 255 } else { 0 };
    canvas_fill(c, sym, light);
    let x0 = sym.x + QR_QUIET * m;
    let y0 = sym.y + QR_QUIET * m;
    for y in 0..n {
        let mut x = 0;
        while x < n {
            if !get_module(&qr, x, y) {
                x += 1;
                continue;
            }
            let mut run = 1; // a row's dark modules side by side go in one fill
            while x + run < n && get_module(&qr, x + run, y) {
                run += 1;
            }
            canvas_fill(c, Rect { x: x0 + x * m, y: y0 + y * m, w: run * m, h: m }, dark);
            x += run;
        }
    }
    sym
}
