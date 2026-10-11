// calib_fit.c: calibration math.
#include "calib_fit.h"

#include <math.h>

void calib_apply(const InputCalib *c, float nx, float ny, float *sx, float *sy) {
    float u = c->swap_xy ? ny : nx, v = c->swap_xy ? nx : ny;
    *sx = c->ax * u + c->bx;
    *sy = c->ay * v + c->by;
}

static float corr(const float *a, const float *b, int n) {
    double ma = 0, mb = 0;
    for (int i = 0; i < n; i++) {
        ma += a[i];
        mb += b[i];
    }
    ma /= n;
    mb /= n;
    double sab = 0, saa = 0, sbb = 0;
    for (int i = 0; i < n; i++) {
        sab += (a[i] - ma) * (b[i] - mb);
        saa += (a[i] - ma) * (a[i] - ma);
        sbb += (b[i] - mb) * (b[i] - mb);
    }
    if (saa <= 1e-12 || sbb <= 1e-12) return 0.f;
    return (float)(sab / sqrt(saa * sbb));
}

// y = a*x + b least squares.
static bool fit_line(const float *x, const float *y, int n, float *a, float *b) {
    double mx = 0, my = 0;
    for (int i = 0; i < n; i++) {
        mx += x[i];
        my += y[i];
    }
    mx /= n;
    my /= n;
    double sxy = 0, sxx = 0;
    for (int i = 0; i < n; i++) {
        sxy += (x[i] - mx) * (y[i] - my);
        sxx += (x[i] - mx) * (x[i] - mx);
    }
    if (sxx < 1e-6) return false;
    *a = (float)(sxy / sxx);
    *b = (float)(my - *a * mx);
    return true;
}

bool calib_fit(const CalPoint *p, int n, InputCalib *out) {
    if (n < 3 || n > 32) return false;
    float nx[32], ny[32], sx[32], sy[32];
    for (int i = 0; i < n; i++) {
        nx[i] = p[i].nx;
        ny[i] = p[i].ny;
        sx[i] = p[i].sx;
        sy[i] = p[i].sy;
    }
    bool swap = fabsf(corr(ny, sx, n)) > fabsf(corr(nx, sx, n));
    InputCalib c = {.valid = true, .swap_xy = swap};
    if (!fit_line(swap ? ny : nx, sx, n, &c.ax, &c.bx)) return false;
    if (!fit_line(swap ? nx : ny, sy, n, &c.ay, &c.by)) return false;
    if (!isfinite(c.ax) || !isfinite(c.ay) || fabsf(c.ax) < 1.f || fabsf(c.ay) < 1.f) return false;
    *out = c;
    return true;
}
