// test_qr.c: the Share screen's QR code: whole-pixel modules, a light quiet zone, pure black and
// white, the exact negative for Invert, and nothing drawn when the text does not fit.
#include <string.h>

#include "qr.h"
#include "qrcodegen/qrcodegen.h"
#include "stb_image_write.h"
#include "testlib.h"

#define MARK 128  // the canvas's color before drawing: whatever still has it was not drawn on

static void out_png(const char *name, const Canvas *c) {
    char dir[512], path[600];
    snprintf(dir, sizeof dir, "%s/../out", tl_tmp());
    mkdir_p(dir);
    snprintf(path, sizeof path, "%s/%s", dir, name);
    stbi_write_png(path, c->w, c->h, 1, c->px, c->stride);
}

static void canvas_mark(Canvas *c) { canvas_fill(c, (Rect){0, 0, c->w, c->h}, MARK); }

// The encoding qr.c promises: ECC M raised when free, versions 3 to 10, automatic mask.
static bool reference(const char *text, uint8_t qr[]) {
    uint8_t tmp[qrcodegen_BUFFER_LEN_FOR_VERSION(10)];
    return qrcodegen_encodeText(text, tmp, qr, qrcodegen_Ecc_MEDIUM, 3, 10, qrcodegen_Mask_AUTO, true);
}

static int count_not(const Canvas *c, Rect r, uint8_t v) {
    int k = 0;
    for (int y = r.y; y < r.y + r.h; y++)
        for (int x = r.x; x < r.x + r.w; x++) k += c->px[y * c->stride + x] != v;
    return k;
}

// Everything the screen relies on, for one address drawn into box.
static void check_symbol(const char *url, Rect box, const char *png) {
    Canvas c;
    canvas_init(&c, 700, 700);
    canvas_mark(&c);
    uint8_t qr[qrcodegen_BUFFER_LEN_FOR_VERSION(10)];
    CHECK(reference(url, qr));
    int n = qrcodegen_getSize(qr);
    CHECK(n == 29);  // version 3, whatever the address
    CHECK(ui_qr_modules(url) == n + 8);
    Rect s = ui_qr(&c, box, url, false);
    CHECK(!rect_is_empty(s));
    CHECK(s.w == s.h && s.w % (n + 8) == 0);
    int m = s.w / (n + 8);
    CHECK(m >= 10);
    CHECK(m == (box.w < box.h ? box.w : box.h) / (n + 8));  // the largest that fits
    CHECK(rect_contains_rect(box, s));
    CHECK(s.x - box.x == (box.w - s.w) / 2 && s.y - box.y == (box.h - s.h) / 2);  // centered
    // Pure black and white inside, untouched outside.
    int other = 0, outside = 0;
    for (int y = 0; y < c.h; y++)
        for (int x = 0; x < c.w; x++) {
            uint8_t v = c.px[y * c.stride + x];
            if (rect_contains(s, x, y)) other += v != 0 && v != 255;
            else outside += v != MARK;
        }
    CHECK(other == 0);
    CHECK(outside == 0);
    // The quiet zone: 4 light modules on every side.
    int q = 4 * m;
    CHECK(count_not(&c, (Rect){s.x, s.y, s.w, q}, 255) == 0);
    CHECK(count_not(&c, (Rect){s.x, s.y + s.h - q, s.w, q}, 255) == 0);
    CHECK(count_not(&c, (Rect){s.x, s.y, q, s.h}, 255) == 0);
    CHECK(count_not(&c, (Rect){s.x + s.w - q, s.y, q, s.h}, 255) == 0);
    // Each module is one flat square of the encoder's color (its center, and all of it).
    int wrong = 0, ragged = 0;
    for (int my = 0; my < n; my++)
        for (int mx = 0; mx < n; mx++) {
            Rect cell = {s.x + q + mx * m, s.y + q + my * m, m, m};
            uint8_t want = qrcodegen_getModule(qr, mx, my) ? 0 : 255;
            wrong += c.px[(cell.y + m / 2) * c.stride + cell.x + m / 2] != want;
            ragged += count_not(&c, cell, want) != 0;
        }
    CHECK(wrong == 0);
    CHECK(ragged == 0);
    if (png) out_png(png, &c);
    // The negative is the exact inverse, pixel for pixel, in the same place.
    Canvas neg;
    canvas_init(&neg, 700, 700);
    canvas_mark(&neg);
    Rect s2 = ui_qr(&neg, box, url, true);
    CHECK(s2.x == s.x && s2.y == s.y && s2.w == s.w && s2.h == s.h);
    int mismatch = 0;
    for (int y = 0; y < c.h; y++)
        for (int x = 0; x < c.w; x++) {
            uint8_t a = c.px[y * c.stride + x], b = neg.px[y * neg.stride + x];
            mismatch += rect_contains(s, x, y) ? a != 255 - b : a != b;
        }
    CHECK(mismatch == 0);
    canvas_free(&neg);
    canvas_free(&c);
}

TEST(share_url_symbol) {
    const char *u27 = "http://192.168.100.200:8765", *u28 = "http://255.255.255.255:65535";
    CHECK(strlen(u27) == 27 && strlen(u28) == 28);
    check_symbol(u27, (Rect){40, 60, 518, 518}, "qr-share.png");  // 14 px modules, the box exactly
    check_symbol(u28, (Rect){20, 10, 620, 530}, NULL);             // not square: centered, 14 px
    check_symbol("http://10.0.0.5:8765", (Rect){0, 0, 592, 600}, NULL);  // short: still 29 x 29
}

TEST(nothing_drawn_when_it_cannot_be_encoded) {
    Canvas c;
    canvas_init(&c, 300, 300);
    canvas_mark(&c);
    char longtext[400];
    memset(longtext, 'x', sizeof longtext - 1);
    longtext[sizeof longtext - 1] = 0;  // 399 bytes: more than version 10 holds
    Rect all = {0, 0, 300, 300};
    Rect r = ui_qr(&c, all, longtext, false);
    CHECK(rect_is_empty(r));
    CHECK(ui_qr_modules(longtext) == 0);
    CHECK(rect_is_empty(ui_qr(&c, all, NULL, true)));
    CHECK(ui_qr_modules(NULL) == 0);
    // A box smaller than one pixel per module draws nothing either.
    CHECK(rect_is_empty(ui_qr(&c, (Rect){10, 10, 36, 200}, "http://10.0.0.5:8765", false)));
    CHECK(rect_is_empty(ui_qr(&c, (Rect){10, 10, -5, 200}, "http://10.0.0.5:8765", false)));
    CHECK(count_not(&c, all, MARK) == 0);
    canvas_free(&c);
}

int main(void) {
    RUN(share_url_symbol);
    RUN(nothing_drawn_when_it_cannot_be_encoded);
    return tl_done();
}
