// fbpack.h: copy one logical grayscale row into an 8/32-bit framebuffer, in either direction.
// The caller bounds the destination and optional previous-frame row; strides are byte/pixel
// steps and can be negative. Previous holds logical grays, before inversion or live dithering.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

void fbpack_row(uint8_t *dst, ptrdiff_t step, uint8_t *previous, ptrdiff_t previous_step,
                const uint8_t *src, int width, int x, int y, int bpp, bool invert, bool fast, bool known);
