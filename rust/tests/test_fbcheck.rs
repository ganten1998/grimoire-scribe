// test_fbcheck.rs: framebuffer readback at 8 and 32 bpp (the screen guard's view of the panel),
// and when a panel counts as never taking a refresh.
// (the C tests index by position, and so do these ports of them)
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn gray_8bpp() {
    let px = [37u8];
    check!(fbcheck_gray(&px, 8, false) == 37);
    check!(fbcheck_gray(&px, 8, true) == 218);
}

fn gray_32bpp() {
    // Grimoire's own pixels are v,v,v,0xFF in every 32 bpp format FBInk knows.
    let mut v = 0;
    while v < 256 {
        let px = [v as u8, v as u8, v as u8, 0xFF];
        check!(fbcheck_gray(&px, 32, false) as i32 == v);
        check!(fbcheck_gray(&px, 32, true) as i32 == 255 - v);
        v += 17;
    }
    // Something another process drew in color reads as a fair gray, and the pad byte is ignored.
    let other = [30u8, 60, 90, 0x00];
    check!(fbcheck_gray(&other, 32, false) == 60);
    let white = [255u8, 255, 255, 0x00];
    check!(fbcheck_gray(&white, 32, false) == 255);
}

// A whole small panel written the way copy_rect does and read back the way the guard does, with a
// padded scanline (the Colorsoft's 1986 px rows are 2000 px apart).
fn roundtrip_padded() {
    const W: usize = 7;
    const H: usize = 5;
    const PAD: usize = 3;
    for bpp in [8, 32] {
        let pxb = fbcheck_px_bytes(bpp);
        let stride = (W + PAD) * pxb;
        let len = stride * H;
        let mut fb = [0xAAu8; (W + PAD) * 4 * H];
        for y in 0..H {
            for x in 0..W {
                let v = (x * 31 + y * 7) as u8;
                let off = fbcheck_offset(x as i32, y as i32, stride, bpp);
                check!(off + pxb <= len);
                if bpp == 8 {
                    fb[off] = v;
                } else {
                    fb[off] = v;
                    fb[off + 1] = v;
                    fb[off + 2] = v;
                    fb[off + 3] = 0xFF;
                }
            }
        }
        for y in 0..H {
            for x in 0..W {
                check!(fbcheck_gray(&fb[fbcheck_offset(x as i32, y as i32, stride, bpp)..], bpp, false) == (x * 31 + y * 7) as u8);
            }
        }
        // The padding was never touched.
        check!(fb[fbcheck_offset(W as i32, 0, stride, bpp)] == 0xAA);
    }
    check!(fbcheck_offset(1985, 2647, 8000, 32) + 4 <= 8000usize * 2648usize);
    check!(fbcheck_offset(10, 3, 1872, 8) == 3 * 1872 + 10);
    check!(fbcheck_offset(10, 3, 8000, 32) == 3 * 8000 + 40);
}

fn broken_panel() {
    check!(!fbcheck_broken(0, 0));
    check!(!fbcheck_broken(4, 0)); // the first frame and 3 more: not yet
    check!(fbcheck_broken(5, 0));
    check!(fbcheck_broken(9, 0));
    // One refresh that worked, and failures never count again (a transient one included).
    check!(!fbcheck_broken(5, 1));
    check!(!fbcheck_broken(200, 1));
    check!(!fbcheck_broken(1, 1));
}

fn dpi_choice() {
    let mut from = DpiSource::default();
    // A device FBInk knows: its own value, whatever the driver says (old 167 dpi panels included).
    check!(fbcheck_dpi("Scribe 2", 300, 1860, 2480, 0, 0, &mut from) == 300.0);
    check!(from == DpiSource::Fbink);
    check!(fbcheck_dpi("Scribe ColorSoft", 300, 1986, 2648, 999, 999, &mut from) == 300.0);
    check!(from == DpiSource::Fbink);
    check!(fbcheck_dpi("Basic", 167, 600, 800, 0, 0, &mut from) == 167.0);
    check!(from == DpiSource::Fbink);
    // A device it does not know: FBInk's 167 is a guess, so the panel's size decides. A 1860x2480
    // panel of 158 x 210 mm is about 300 dpi; with the sides swapped (a rotated framebuffer) too.
    check_near!(fbcheck_dpi("Unidentified!", 167, 1860, 2480, 158, 210, &mut from), 300.0, 2.0);
    check!(from == DpiSource::PanelSize);
    check_near!(fbcheck_dpi("Unidentified!", 167, 2480, 1860, 158, 210, &mut from), 300.0, 2.0);
    check!(from == DpiSource::PanelSize);
    check_near!(fbcheck_dpi("Unknown!", 167, 1860, 2480, 158, 0, &mut from), 299.0, 2.0); // one side only
    check!(from == DpiSource::PanelSize);
    check_near!(fbcheck_dpi("", 167, 1860, 2480, 158, 210, &mut from), 300.0, 2.0); // FBInk named nothing
    check!(from == DpiSource::PanelSize);
    // (the C passes NULL here: a name that is not there reads as none, which "" is)
    check_near!(fbcheck_dpi("", 0, 1860, 2480, 158, 210, &mut from), 300.0, 2.0);
    // No usable size: 0 mm, -1 mm (all bits set), or a density no e-reader has: 300.
    check!(fbcheck_dpi("Unidentified!", 167, 1860, 2480, 0, 0, &mut from) == 300.0);
    check!(from == DpiSource::Default);
    check!(fbcheck_dpi("Unidentified!", 167, 1860, 2480, 0xFFFFFFFF, 0xFFFFFFFF, &mut from) == 300.0);
    check!(from == DpiSource::Default);
    check!(fbcheck_dpi("Unknown!", 167, 1860, 2480, 2, 3, &mut from) == 300.0); // 18000 dpi
    check!(from == DpiSource::Default);
    check!(fbcheck_dpi("Unknown!", 167, 1860, 2480, 900, 999, &mut from) == 300.0); // 60 dpi
    check!(from == DpiSource::Default);
    check!(fbcheck_dpi("Unknown!", 167, 0, 0, 158, 210, &mut from) == 300.0);
    check!(from == DpiSource::Default);
    // Known by name but with no density: treated as unknown.
    check_near!(fbcheck_dpi("Scribe 2", 0, 1860, 2480, 158, 210, &mut from), 300.0, 2.0);
    check!(from == DpiSource::PanelSize);
}

#[test]
fn test_fbcheck() {
    run!(gray_8bpp);
    run!(gray_32bpp);
    run!(roundtrip_padded);
    run!(broken_panel);
    run!(dpi_choice);
    done();
}
