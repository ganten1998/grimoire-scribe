// Offline model of live (DU) frames on gray paper. Draws strokes with the real brush and default
// tools, one fast frame per pen sample, and packs each frame with fbpack_row two ways: without
// the panel's state (every changed pixel dithered to black or white, as 2.0.0-beta.3 did) and
// with it (each shown pixel only moves toward its new value). The panel model: a pixel's DU
// transition takes T ms and cannot start before its previous one has ended. Reports transitions
// per touched pixel, pixels that changed more than once, pixels that went white over gray paper,
// and the time from a pixel's first ink to its black being complete (median and 90th percentile).
// Never opens the display or input. Usage: du-model [T_ms] [samples_per_second]
#include "brush.h"
#include "fbpack.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { W = 900, H = 400 };

typedef struct {
    double touched, transitions, multi, white;
    double lag[W * H];
    int nlag;
} Result;

static int cmp_double(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

static void run(int tool, int paper_gray, double mm_per_s, double hz, double T, bool panel_state, Result *res) {
    static uint8_t shown[W * H], previous[W * H];
    static double busy[W * H], first_ink[W * H], black_at[W * H];
    static unsigned char changes[W * H];
    Canvas c;
    if (canvas_init(&c, W, H)) exit(1);
    unsigned noise = 12345;
    for (int i = 0; i < W * H; i++) {
        int g = paper_gray;
        if (g < 255) {  // scanned paper is not one flat gray
            noise = noise * 1103515245u + 12345u;
            g += (int)((noise >> 16) % 17) - 8;
        }
        c.px[i] = shown[i] = previous[i] = (uint8_t)(g < 0 ? 0 : g > 255 ? 255 : g);
        busy[i] = -1e9;
        first_ink[i] = black_at[i] = -1;
        changes[i] = 0;
    }
    ToolParams p;
    tool_params_default((ToolKind)tool, &p);
    BrushState b;
    brush_begin(&b, (ToolKind)tool, &p, 1729, 300.f);
    // A handwriting-like path (a slanted wave with loops) at constant speed, 700 px long.
    double px_per_ms = mm_per_s * 300.0 / 25.4 / 1000.0, dt = 1000.0 / hz, t = 0, travelled = 0, u = 0;
    double x = 100, y = 220;  // where the path starts (u = 0)
    memset(res, 0, sizeof *res);
    while (travelled < 700) {
        double step = 0;
        while (step < px_per_ms * dt) {
            u += 0.002;
            double nx = 100 + u * 60 + 25 * sin(u * 2.1), ny = 200 + 70 * sin(u * 1.3) + 20 * cos(u * 3.7);
            step += hypot(nx - x, ny - y);
            x = nx;
            y = ny;
        }
        travelled += step;
        PenSample s = {(float)x, (float)y, 0.55f + 0.1f * (float)sin(u), 0, 0, (uint32_t)t};
        Rect r = brush_add(&b, &c, &s);
        for (int py = r.y; py < r.y + r.h; py++)
            for (int px = r.x; px < r.x + r.w; px++) {
                int i = py * W + px;
                uint8_t v = c.px[i], was = shown[i];
                if (v == previous[i]) continue;
                previous[i] = v;
                if (first_ink[i] < 0) first_ink[i] = t;
                uint8_t other = (uint8_t)(v ^ 1);  // makes fbpack_row see a changed pixel
                if (panel_state) fbpack_row(&shown[i], 1, &other, 1, &v, 1, px, py, 8, false, true, true);
                else fbpack_row(&shown[i], 1, NULL, 0, &v, 1, px, py, 8, false, true, false);
                if (shown[i] == was) continue;
                double start = busy[i] > t ? busy[i] : t;
                busy[i] = start + T;
                if (was != 0 && was != 255 && shown[i] == 255) res->white++;
                if (changes[i] < 255) changes[i]++;
                res->transitions++;
                black_at[i] = shown[i] == 0 ? busy[i] : -1;
            }
        t += dt;
    }
    for (int i = 0; i < W * H; i++) {
        if (first_ink[i] < 0) continue;
        res->touched++;
        if (changes[i] > 1) res->multi++;
        if (black_at[i] >= 0) res->lag[res->nlag++] = black_at[i] - first_ink[i];
    }
    qsort(res->lag, (size_t)res->nlag, sizeof(double), cmp_double);
    canvas_free(&c);
}

int main(int argc, char **argv) {
    double T = argc > 1 ? atof(argv[1]) : 150, hz = argc > 2 ? atof(argv[2]) : 240;
    static Result a, n;
    const int papers[] = {255, 225, 195, 160};
    const double speeds[] = {20, 60, 150};
    printf("DU %.0f ms, %.0f samples/s. Each pair: beta.3 rule / current rule.\n", T, hz);
    puts("tool,paper,mm_per_s,transitions_per_px,changed_twice_pct,white_over_gray_pct,black_p50_ms,black_p90_ms");
    for (int tool = 0; tool < TOOL_COUNT; tool++) {
        if (tool == TOOL_ERASER) continue;
        for (size_t pi = 0; pi < sizeof papers / sizeof *papers; pi++)
            for (size_t si = 0; si < sizeof speeds / sizeof *speeds; si++) {
                run(tool, papers[pi], speeds[si], hz, T, false, &a);
                run(tool, papers[pi], speeds[si], hz, T, true, &n);
                double ta = a.touched > 0 ? a.touched : 1, tn = n.touched > 0 ? n.touched : 1;
                printf("%s,%d,%.0f,%.2f/%.2f,%.1f/%.1f,%.1f/%.1f,%.0f/%.0f,%.0f/%.0f\n", tool_name((ToolKind)tool),
                       papers[pi], speeds[si], a.transitions / ta, n.transitions / tn, 100 * a.multi / ta,
                       100 * n.multi / tn, 100 * a.white / ta, 100 * n.white / tn, a.nlag ? a.lag[a.nlag / 2] : 0,
                       n.nlag ? n.lag[n.nlag / 2] : 0, a.nlag ? a.lag[a.nlag * 9 / 10] : 0,
                       n.nlag ? n.lag[n.nlag * 9 / 10] : 0);
            }
    }
    return 0;
}
