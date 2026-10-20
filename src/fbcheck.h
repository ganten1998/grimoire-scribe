// fbcheck.h: pure helpers behind display_fbink.c's safety checks (unit-tested on the host): where a
// framebuffer pixel lives and which gray it holds, at 8 bpp (the Scribe 2) and 32 bpp (the Scribe
// Colorsoft), and whether the panel has taken any of Grimoire's refreshes at all.
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
