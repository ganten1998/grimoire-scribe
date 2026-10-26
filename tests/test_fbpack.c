// Live updates must not turn gray paper around a stroke into a temporary black/white halo.
#include "fbpack.h"
#include "testlib.h"

enum { PW = 9, PH = 7, ROOM = 512, GUARD = 16 };

static void position(int x, int y, int rotation, bool flip, int *fx, int *fy) {
    switch (rotation) {
    case 1: *fx = y; *fy = PH - 1 - x; break;
    case 2: *fx = PW - 1 - x; *fy = PH - 1 - y; break;
    case 3: *fx = PW - 1 - y; *fy = x; break;
    default: *fx = x; *fy = y; break;
    }
    if (flip) { *fx = PW - 1 - *fx; *fy = PH - 1 - *fy; }
}

static void frame(uint8_t *fb, uint8_t *previous, const uint8_t *src, int bpp, int rotation,
                  bool flip, bool invert, bool fast, bool known) {
    int w = rotation & 1 ? PH : PW, h = rotation & 1 ? PW : PH;
    int bytes = bpp / 8, stride = PW * bytes + 8;
    for (int y = 0; y < h; y++) {
        int fx, fy, nx, ny;
        position(0, y, rotation, flip, &fx, &fy);
        position(1, y, rotation, flip, &nx, &ny);
        ptrdiff_t step = (ny - fy) * stride + (nx - fx) * bytes;
        ptrdiff_t pstep = (ny - fy) * PW + nx - fx;
        fbpack_row(fb + fy * stride + fx * bytes, step, previous + fy * PW + fx, pstep,
                    src + y * w, w, 0, y, bpp, invert, fast, known);
    }
}

TEST(gray_paper_survives_live_ink_and_overlap) {
    for (int bpp = 8; bpp <= 32; bpp += 24)
        for (int rotation = 0; rotation < 4; rotation++)
            for (int flip = 0; flip < 2; flip++)
                for (int invert = 0; invert < 2; invert++) {
                    uint8_t memory[ROOM], before[ROOM], live[ROOM], previous[PW * PH], src[PW * PH];
                    memset(memory, 0xCD, sizeof memory);
                    memset(previous, 0, sizeof previous);
                    int w = rotation & 1 ? PH : PW, bytes = bpp / 8, stride = PW * bytes + 8;
                    for (int i = 0; i < PW * PH; i++) src[i] = (uint8_t)(40 + i % 180);
                    uint8_t *fb = memory + GUARD;
                    frame(fb, previous, src, bpp, rotation, flip, invert, false, false);
                    memcpy(before, memory, sizeof memory);
                    // One ink sample in the middle of a dirty rectangle containing gray paper.
                    int written = 2 * w + 3;
                    src[written] = 0;
                    src[written + 1] = 73;
                    frame(fb, previous, src, bpp, rotation, flip, invert, true, true);
                    for (int i = 0; i < PW * PH; i++) {
                        int fx, fy;
                        position(i % w, i / w, rotation, flip, &fx, &fy);
                        int off = GUARD + fy * stride + fx * bytes;
                        if (i != written && i != written + 1) {
                            CHECK(!memcmp(memory + off, before + off, (size_t)bytes));
                            CHECK(memory[off] != 0 && memory[off] != 255);
                        } else {
                            CHECK(memory[off] == 0 || memory[off] == 255);
                        }
                        CHECK(previous[fy * PW + fx] == src[i]);
                    }
                    // An overlapping refresh must neither erase earlier live ink nor dither paper.
                    memcpy(live, memory, sizeof memory);
                    frame(fb, previous, src, bpp, rotation, flip, invert, true, true);
                    CHECK(!memcmp(live, memory, sizeof memory));
                    // Settle restores the exact gray values, including the stroke's soft edge.
                    frame(fb, previous, src, bpp, rotation, flip, invert, false, true);
                    for (int i = 0; i < PW * PH; i++) {
                        int fx, fy;
                        position(i % w, i / w, rotation, flip, &fx, &fy);
                        uint8_t *p = fb + fy * stride + fx * bytes;
                        CHECK(p[0] == (invert ? 255 - src[i] : src[i]));
                        if (bpp == 32) CHECK(p[1] == p[0] && p[2] == p[0] && p[3] == 255);
                    }
                    for (int i = 0; i < GUARD; i++) CHECK(memory[i] == 0xCD);
                    for (int y = 0; y < PH; y++)
                        for (int x = PW * bytes; x < stride; x++) CHECK(fb[y * stride + x] == 0xCD);
                    for (int i = GUARD + PH * stride; i < ROOM; i++) CHECK(memory[i] == 0xCD);
                }
}

TEST(no_cache_falls_back_and_invalidating_repaints) {
    uint8_t src[16], dst[16], previous[16];
    memset(src, 96, sizeof src);
    memset(dst, 19, sizeof dst);
    memset(previous, 96, sizeof previous);
    fbpack_row(dst, 1, NULL, 0, src, 16, 3, 7, 8, false, true, false);
    for (int i = 0; i < 16; i++) CHECK(dst[i] == 0 || dst[i] == 255);
    // Changing inversion invalidates the cache; equal logical values still need new pixels.
    fbpack_row(dst, 1, previous, 1, src, 16, 3, 7, 8, true, false, false);
    for (int i = 0; i < 16; i++) CHECK(dst[i] == 159 && previous[i] == 96);
    uint8_t before[16];
    memcpy(before, dst, sizeof before);
    fbpack_row(dst, 1, previous, 1, src, 0, 0, 0, 8, false, true, true);
    fbpack_row(dst, 1, previous, 1, src, 16, 0, 0, 16, false, true, true);
    CHECK(!memcmp(before, dst, sizeof dst));
}

int main(void) {
    RUN(gray_paper_survives_live_ink_and_overlap);
    RUN(no_cache_falls_back_and_invalidating_repaints);
    return tl_done();
}
