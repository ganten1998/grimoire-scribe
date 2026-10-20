// qr.h: QR codes on the screen (the Share screen's address, for a phone camera), drawn the way
// e-ink shows best: whole-pixel square modules in pure black and white.
#pragma once
#include "canvas.h"

// Modules across the symbol for text, its quiet zone included (37 for every share address), or
// 0 when the text cannot be encoded. A box of that many times m pixels gets modules of m pixels.
int ui_qr_modules(const char *text);
// Draws text as a QR code centered in box: square modules of the largest whole number of pixels
// that fits, inside a quiet zone of 4 light modules. negative draws the exact inverse, for the
// Invert setting (it inverts the panel at display time, so the code still shows dark on light).
// Returns the symbol's rect, quiet zone included, or RECT_EMPTY (nothing drawn) when the text
// cannot be encoded or the box is too small.
Rect ui_qr(Canvas *c, Rect box, const char *text, bool negative);
