// test_fbcheck.c: framebuffer readback at 8 and 32 bpp (the screen guard's view of the panel),
// and when a panel counts as never taking a refresh.
#include "fbcheck.h"
#include "testlib.h"

TEST(gray_8bpp) {
    uint8_t px = 37;
    CHECK(fbcheck_gray(&px, 8, false) == 37);
    CHECK(fbcheck_gray(&px, 8, true) == 218);
}

TEST(gray_32bpp) {
    // Grimoire's own pixels are v,v,v,0xFF in every 32 bpp format FBInk knows.
    for (int v = 0; v < 256; v += 17) {
        uint8_t px[4] = {(uint8_t)v, (uint8_t)v, (uint8_t)v, 0xFF};
        CHECK(fbcheck_gray(px, 32, false) == v);
        CHECK(fbcheck_gray(px, 32, true) == 255 - v);
    }
    // Something another process drew in color reads as a fair gray, and the pad byte is ignored.
    uint8_t other[4] = {30, 60, 90, 0x00};
    CHECK(fbcheck_gray(other, 32, false) == 60);
    uint8_t white[4] = {255, 255, 255, 0x00};
    CHECK(fbcheck_gray(white, 32, false) == 255);
}

// A whole small panel written the way copy_rect does and read back the way the guard does, with a
// padded scanline (the Colorsoft's 1986 px rows are 2000 px apart).
TEST(roundtrip_padded) {
    enum { W = 7, H = 5, PAD = 3 };
    for (int bpp = 8; bpp <= 32; bpp += 24) {
        size_t pxb = fbcheck_px_bytes(bpp), stride = (W + PAD) * pxb;
        size_t len = stride * H;
        uint8_t fb[(W + PAD) * 4 * H];
        memset(fb, 0xAA, sizeof fb);
        for (int y = 0; y < H; y++)
            for (int x = 0; x < W; x++) {
                uint8_t v = (uint8_t)(x * 31 + y * 7);
                size_t off = fbcheck_offset(x, y, stride, bpp);
                CHECK(off + pxb <= len);
                if (bpp == 8) fb[off] = v;
                else {
                    fb[off] = fb[off + 1] = fb[off + 2] = v;
                    fb[off + 3] = 0xFF;
                }
            }
        for (int y = 0; y < H; y++)
            for (int x = 0; x < W; x++)
                CHECK(fbcheck_gray(fb + fbcheck_offset(x, y, stride, bpp), bpp, false) == (uint8_t)(x * 31 + y * 7));
        // The padding was never touched.
        CHECK(fb[fbcheck_offset(W, 0, stride, bpp)] == 0xAA);
    }
    CHECK(fbcheck_offset(1985, 2647, 8000, 32) + 4 <= 8000u * 2648u);
    CHECK(fbcheck_offset(10, 3, 1872, 8) == 3 * 1872 + 10);
    CHECK(fbcheck_offset(10, 3, 8000, 32) == 3 * 8000 + 40);
}

TEST(broken_panel) {
    CHECK(!fbcheck_broken(0, 0));
    CHECK(!fbcheck_broken(4, 0));  // the first frame and 3 more: not yet
    CHECK(fbcheck_broken(5, 0));
    CHECK(fbcheck_broken(9, 0));
    // One refresh that worked, and failures never count again (a transient one included).
    CHECK(!fbcheck_broken(5, 1));
    CHECK(!fbcheck_broken(200, 1));
    CHECK(!fbcheck_broken(1, 1));
}

int main(void) {
    RUN(gray_8bpp);
    RUN(gray_32bpp);
    RUN(roundtrip_padded);
    RUN(broken_panel);
    return tl_done();
}
