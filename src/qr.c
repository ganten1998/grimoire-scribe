// qr.c: QR codes with Nayuki's QR Code generator (third_party/qrcodegen, MIT, vendored unmodified
// from https://github.com/nayuki/QR-Code-generator at 3c6d0b3). ECC level M, raised when it costs
// no size, and at least version 3: every share address (http://<IPv4>:<port>, at most 28 bytes)
// then comes out as the same 29 x 29 symbol, so the layout never jumps with the address.
#include "qr.h"

#include "qrcodegen/qrcodegen.h"

enum { QR_MIN_VERSION = 3, QR_MAX_VERSION = 10, QR_QUIET = 4 };
#define QR_BUF qrcodegen_BUFFER_LEN_FOR_VERSION(QR_MAX_VERSION)

static bool encode(const char *text, uint8_t qr[QR_BUF]) {
    uint8_t tmp[QR_BUF];
    return text && qrcodegen_encodeText(text, tmp, qr, qrcodegen_Ecc_MEDIUM, QR_MIN_VERSION, QR_MAX_VERSION,
                                        qrcodegen_Mask_AUTO, true);
}

int ui_qr_modules(const char *text) {
    uint8_t qr[QR_BUF];
    return encode(text, qr) ? qrcodegen_getSize(qr) + 2 * QR_QUIET : 0;
}

Rect ui_qr(Canvas *c, Rect box, const char *text, bool negative) {
    uint8_t qr[QR_BUF];
    if (!encode(text, qr)) return RECT_EMPTY;
    int n = qrcodegen_getSize(qr), across = n + 2 * QR_QUIET;
    int m = (box.w < box.h ? box.w : box.h) / across;
    if (m < 1) return RECT_EMPTY;
    int side = across * m;
    Rect sym = {box.x + (box.w - side) / 2, box.y + (box.h - side) / 2, side, side};
    uint8_t light = negative ? 0 : 255, dark = negative ? 255 : 0;
    canvas_fill(c, sym, light);
    int x0 = sym.x + QR_QUIET * m, y0 = sym.y + QR_QUIET * m;
    for (int y = 0; y < n; y++)
        for (int x = 0; x < n; x++) {
            if (!qrcodegen_getModule(qr, x, y)) continue;
            int run = 1;  // a row's dark modules side by side go in one fill
            while (x + run < n && qrcodegen_getModule(qr, x + run, y)) run++;
            canvas_fill(c, (Rect){x0 + x * m, y0 + y * m, run * m, m}, dark);
            x += run - 1;
        }
    return sym;
}
