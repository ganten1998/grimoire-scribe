// test_qr.c: the Share screen's QR code: whole-pixel modules, a light quiet zone, pure black and
// white, the exact negative for Invert, and nothing drawn when the text does not fit.
mod testlib;

use grimoire::ffi_qr::*;
use grimoire::ffi_stb::stbi_write_png;
use grimoire::prelude::*;
use testlib::*;

/// the canvas's color before drawing: whatever still has it was not drawn on
const MARK: u8 = 128;

fn out_png(name: &str, c: &Canvas) {
    let dir = cformat!("%s/../out", &tl_tmp());
    mkdir_p(&dir);
    let path = cformat!("%s/%s", &dir, name);
    let cpath = std::ffi::CString::new(path).unwrap();
    // SAFETY: cpath is NUL-terminated; c.px holds h rows of stride bytes, one component per pixel,
    // and stb only reads from it.
    unsafe { stbi_write_png(cpath.as_ptr(), c.w, c.h, 1, c.px.as_ptr() as *const libc::c_void, c.stride) };
}

fn canvas_mark(c: &mut Canvas) {
    let r = Rect { x: 0, y: 0, w: c.w, h: c.h };
    canvas_fill(c, r, MARK);
}

/// The encoding qr.c promises: ECC M raised when free, versions 3 to 10, automatic mask.
fn reference(text: &str, qr: &mut [u8]) -> bool {
    let mut tmp = [0u8; qrcodegen_buffer_len_for_version(10)];
    let z = std::ffi::CString::new(text).unwrap();
    // SAFETY: z is NUL-terminated; tmp and qr are the buffer size qrcodegen asks for at version 10.
    unsafe { qrcodegen_encodeText(z.as_ptr(), tmp.as_mut_ptr(), qr.as_mut_ptr(), qrcodegen_Ecc_MEDIUM, 3, 10, qrcodegen_Mask_AUTO, true) }
}

fn get_size(qr: &[u8]) -> i32 {
    // SAFETY: qr holds a symbol qrcodegen_encodeText made.
    unsafe { qrcodegen_getSize(qr.as_ptr()) }
}

fn get_module(qr: &[u8], x: i32, y: i32) -> bool {
    // SAFETY: qr holds a symbol qrcodegen_encodeText made; qrcodegen_getModule checks the bounds.
    unsafe { qrcodegen_getModule(qr.as_ptr(), x, y) }
}

fn count_not(c: &Canvas, r: Rect, v: u8) -> i32 {
    let mut k = 0;
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            k += (c.px[(y * c.stride + x) as usize] != v) as i32;
        }
    }
    k
}

/// Everything the screen relies on, for one address drawn into box.
fn check_symbol(url: &str, box_: Rect, png: Option<&str>) {
    let mut c = Canvas::default();
    canvas_init(&mut c, 700, 700);
    canvas_mark(&mut c);
    let mut qr = [0u8; qrcodegen_buffer_len_for_version(10)];
    check!(reference(url, &mut qr));
    let n = get_size(&qr);
    check!(n == 29); // version 3, whatever the address
    check!(ui_qr_modules(url) == n + 8);
    let s = ui_qr(&mut c, box_, url, false);
    check!(!rect_is_empty(s));
    check!(s.w == s.h && s.w % (n + 8) == 0);
    let m = s.w / (n + 8);
    check!(m >= 10);
    check!(m == (if box_.w < box_.h { box_.w } else { box_.h }) / (n + 8)); // the largest that fits
    check!(rect_contains_rect(box_, s));
    check!(s.x - box_.x == (box_.w - s.w) / 2 && s.y - box_.y == (box_.h - s.h) / 2); // centered
    // Pure black and white inside, untouched outside.
    let (mut other, mut outside) = (0, 0);
    for y in 0..c.h {
        for x in 0..c.w {
            let v = c.px[(y * c.stride + x) as usize];
            if rect_contains(s, x, y) {
                other += (v != 0 && v != 255) as i32;
            } else {
                outside += (v != MARK) as i32;
            }
        }
    }
    check!(other == 0);
    check!(outside == 0);
    // The quiet zone: 4 light modules on every side.
    let q = 4 * m;
    check!(count_not(&c, Rect { x: s.x, y: s.y, w: s.w, h: q }, 255) == 0);
    check!(count_not(&c, Rect { x: s.x, y: s.y + s.h - q, w: s.w, h: q }, 255) == 0);
    check!(count_not(&c, Rect { x: s.x, y: s.y, w: q, h: s.h }, 255) == 0);
    check!(count_not(&c, Rect { x: s.x + s.w - q, y: s.y, w: q, h: s.h }, 255) == 0);
    // Each module is one flat square of the encoder's color (its center, and all of it).
    let (mut wrong, mut ragged) = (0, 0);
    for my in 0..n {
        for mx in 0..n {
            let cell = Rect { x: s.x + q + mx * m, y: s.y + q + my * m, w: m, h: m };
            let want: u8 = if get_module(&qr, mx, my) { 0 } else { 255 };
            wrong += (c.px[((cell.y + m / 2) * c.stride + cell.x + m / 2) as usize] != want) as i32;
            ragged += (count_not(&c, cell, want) != 0) as i32;
        }
    }
    check!(wrong == 0);
    check!(ragged == 0);
    if let Some(png) = png {
        out_png(png, &c);
    }
    // The negative is the exact inverse, pixel for pixel, in the same place.
    let mut neg = Canvas::default();
    canvas_init(&mut neg, 700, 700);
    canvas_mark(&mut neg);
    let s2 = ui_qr(&mut neg, box_, url, true);
    check!(s2.x == s.x && s2.y == s.y && s2.w == s.w && s2.h == s.h);
    let mut mismatch = 0;
    for y in 0..c.h {
        for x in 0..c.w {
            let a = c.px[(y * c.stride + x) as usize];
            let b = neg.px[(y * neg.stride + x) as usize];
            mismatch += if rect_contains(s, x, y) { a != 255 - b } else { a != b } as i32;
        }
    }
    check!(mismatch == 0);
    canvas_free(&mut neg);
    canvas_free(&mut c);
}

fn share_url_symbol() {
    let (u27, u28) = ("http://192.168.100.200:8765", "http://255.255.255.255:65535");
    check!(u27.len() == 27 && u28.len() == 28);
    check_symbol(u27, Rect { x: 40, y: 60, w: 518, h: 518 }, Some("qr-share.png")); // 14 px modules, the box exactly
    check_symbol(u28, Rect { x: 20, y: 10, w: 620, h: 530 }, None); // not square: centered, 14 px
    check_symbol("http://10.0.0.5:8765", Rect { x: 0, y: 0, w: 592, h: 600 }, None); // short: still 29 x 29
}

fn nothing_drawn_when_it_cannot_be_encoded() {
    let mut c = Canvas::default();
    canvas_init(&mut c, 300, 300);
    canvas_mark(&mut c);
    let longtext = "x".repeat(399); // 399 bytes: more than version 10 holds
    let all = Rect { x: 0, y: 0, w: 300, h: 300 };
    let r = ui_qr(&mut c, all, &longtext, false);
    check!(rect_is_empty(r));
    check!(ui_qr_modules(&longtext) == 0);
    // (the C also calls both with a NULL text; the text is a &str here, which cannot be NULL, and
    // no caller of the C passes NULL)
    // A box smaller than one pixel per module draws nothing either.
    check!(rect_is_empty(ui_qr(&mut c, Rect { x: 10, y: 10, w: 36, h: 200 }, "http://10.0.0.5:8765", false)));
    check!(rect_is_empty(ui_qr(&mut c, Rect { x: 10, y: 10, w: -5, h: 200 }, "http://10.0.0.5:8765", false)));
    check!(count_not(&c, all, MARK) == 0);
    canvas_free(&mut c);
}

#[test]
fn test_qr() {
    run!(share_url_symbol);
    run!(nothing_drawn_when_it_cannot_be_encoded);
    done();
}
