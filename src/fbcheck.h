// fbcheck.h: pure helpers behind display_fbink.c's safety checks (unit-tested on the host): where a
// framebuffer pixel lives and which gray it holds, at 8 bpp (the Scribe 2) and 32 bpp (the Scribe
// Colorsoft), whether the panel has taken any of Grimoire's refreshes at all, and its density.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Bytes Grimoire writes per pixel: one gray byte at 8 bpp, v,v,v,0xFF at 32 bpp.
static inline size_t fbcheck_px_bytes(int bpp) { return bpp == 32 ? 4 : 1; }
// Byte offset of framebuffer pixel (fx, fy); stride is the scanline length in bytes (padding included).
static inline size_t fbcheck_offset(int fx, int fy, size_t stride, int bpp) {
    return (size_t)fy * stride + (size_t)fx * fbcheck_px_bytes(bpp);
}
// The gray at `px`: the byte at 8 bpp, the mean of the three color channels at 32 bpp (exactly v
// for Grimoire's own v,v,v,0xFF; a fair gray for anything another process drew). `flip` undoes a
// value inversion (Grimoire's own, and at 8 bpp the framebuffer's inverted grayscale).
uint8_t fbcheck_gray(const uint8_t *px, int bpp, bool flip);
// True once the first frame and the next 4 refreshes have all failed (`attempts` refreshes sent,
// `ok` taken): the panel is not Grimoire's to drive. Once one refresh worked, failures never count.
bool fbcheck_broken(int attempts, int ok);

// Where the UI's density comes from (fbcheck_dpi).
typedef enum { DPI_FBINK, DPI_PANEL_SIZE, DPI_DEFAULT } DpiSource;
// The density to lay the UI out for. FBInk's own value when it identified the device. For a device
// it does not know (it names it "Unidentified!" or "Unknown!", or nothing, and guesses 167 dpi, a
// value for early Pearl screens), the panel's physical size as the driver reports it (pixels over
// mm: the diagonal when both sides are known, so a rotated framebuffer reads the same) when that
// comes to 100..600 dpi; else 300, the density of every Kindle Scribe.
float fbcheck_dpi(const char *fbink_name, unsigned fbink_dpi, unsigned xres, unsigned yres, unsigned width_mm,
                  unsigned height_mm, DpiSource *from);
