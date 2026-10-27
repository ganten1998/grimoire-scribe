//! fbcheck.h: pure helpers behind display_fbink.c's safety checks (unit-tested on the host): where a
//! framebuffer pixel lives and which gray it holds, at 8 bpp (the Scribe 2) and 32 bpp (the Scribe
//! Colorsoft), whether the panel has taken any of Grimoire's refreshes at all, and its density.

/// Bytes Grimoire writes per pixel: one gray byte at 8 bpp, v,v,v,0xFF at 32 bpp.
#[inline]
pub fn fbcheck_px_bytes(bpp: i32) -> usize {
    if bpp == 32 {
        4
    } else {
        1
    }
}
/// Byte offset of framebuffer pixel (fx, fy); stride is the scanline length in bytes (padding included).
#[inline]
pub fn fbcheck_offset(fx: i32, fy: i32, stride: usize, bpp: i32) -> usize {
    (fy as usize).wrapping_mul(stride).wrapping_add((fx as usize).wrapping_mul(fbcheck_px_bytes(bpp)))
}
/// The gray at `px`: the byte at 8 bpp, the mean of the three color channels at 32 bpp (exactly v
/// for Grimoire's own v,v,v,0xFF; a fair gray for anything another process drew). `flip` undoes a
/// value inversion (Grimoire's own, and at 8 bpp the framebuffer's inverted grayscale).
pub fn fbcheck_gray(px: &[u8], bpp: i32, flip: bool) -> u8 {
    let v: u32 = if bpp == 32 { (px[0] as u32 + px[1] as u32 + px[2] as u32 + 1) / 3 } else { px[0] as u32 };
    (if flip { 255 - v } else { v }) as u8
}
/// True once the first frame and the next 4 refreshes have all failed (`attempts` refreshes sent,
/// `ok` taken): the panel is not Grimoire's to drive. Once one refresh worked, failures never count.
pub fn fbcheck_broken(attempts: i32, ok: i32) -> bool {
    ok == 0 && attempts >= 5
}

/// Where the UI's density comes from (fbcheck_dpi).
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DpiSource {
    #[default]
    Fbink = 0,  // DPI_FBINK
    PanelSize = 1,  // DPI_PANEL_SIZE
    Default = 2,  // DPI_DEFAULT
}
/// The density to lay the UI out for. FBInk's own value when it identified the device. For a device
/// it does not know (it names it "Unidentified!" or "Unknown!", or nothing, and guesses 167 dpi, a
/// value for early Pearl screens), the panel's physical size as the driver reports it (pixels over
/// mm: the diagonal when both sides are known, so a rotated framebuffer reads the same) when that
/// comes to 100..600 dpi; else 300, the density of every Kindle Scribe.
pub fn fbcheck_dpi(fbink_name: &str, fbink_dpi: u32, xres: u32, yres: u32, width_mm: u32, height_mm: u32, from: &mut DpiSource) -> f32 {
    let known = !fbink_name.is_empty() && fbink_name != "Unidentified!" && fbink_name != "Unknown!";
    if known && fbink_dpi > 0 {
        *from = DpiSource::Fbink;
        return fbink_dpi as f32;
    }
    // Drivers that do not know their panel report 0 mm (or -1, read as a huge number).
    let mut d: f64 = 0.0;
    if xres != 0 && yres != 0 && width_mm != 0 && height_mm != 0 && width_mm < 1000 && height_mm < 1000 {
        d = (xres as f64).hypot(yres as f64) * 25.4 / (width_mm as f64).hypot(height_mm as f64);
    } else if xres != 0 && width_mm != 0 && width_mm < 1000 {
        d = xres as f64 * 25.4 / width_mm as f64;
    }
    if (100.0..=600.0).contains(&d) {
        *from = DpiSource::PanelSize;
        return d as f32;
    }
    *from = DpiSource::Default;
    300.0
}
