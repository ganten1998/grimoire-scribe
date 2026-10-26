// Offscreen brush benchmark. Uses synthetic paper only, never opens the display or input.
// Compare the hashes with a build from the previous revision before comparing timings.
// Build with the same optimization flags as the Kindle app (see docs/INK-PERFORMANCE.md).
#include "brush.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static uint64_t micros(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (uint64_t)t.tv_sec * 1000000 + (uint64_t)t.tv_nsec / 1000;
}

static uint64_t mix(uint64_t h, unsigned v) { return (h ^ v) * UINT64_C(1099511628211); }
static int cmp_time(const void *a, const void *b) {
    uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return (x > y) - (x < y);
}

int main(int argc, char **argv) {
    bool verify = argc > 1 && !strcmp(argv[1], "--verify");
    enum { W = 1280, H = 800, N = 800 };
    Canvas c, paper;
    if (canvas_init(&c, W, H) || canvas_init(&paper, W, H)) return 1;
    puts("tool,paper,variant,hash,mean_us,p95_us,max_us");
    for (int tool = 0; tool < TOOL_COUNT; tool++) for (int bg = 0; bg < 3; bg++) {
        for (int variant = 0; variant < (verify ? 8 : 1); variant++) {
            for (int y = 0; y < H; y++) for (int x = 0; x < W; x++) {
                uint8_t v = bg == 0 ? 255 : bg == 1 ? (uint8_t)(32 + (x * 3 + y * 7) % 224)
                                                     : ((x / 24 + y / 32) & 1) ? 0 : 255;
                paper.px[y * W + x] = v;
                c.px[y * W + x] = tool == TOOL_ERASER || variant == 7 ? v / 3 : v;
            }
            ToolParams p;
            tool_params_default(tool, &p);
            if (variant == 1) p.grain = 0;
            if (variant == 2) p.grain = 1;
            if (variant == 3) { p.opacity_min = 0.01f; p.opacity_max = 0.28f; }
            if (variant == 4) p.size_min_mm = p.size_max_mm = 0.2f;
            if (variant == 5) p.size_min_mm = p.size_max_mm = 8.f;
            canvas_set_clip(&c, variant == 6 ? (Rect){80, 100, 950, 500} : (Rect){0, 0, W, H});
            BrushState b;
            float dpi = variant == 4 ? 144.f : variant == 5 ? 600.f : 300.f;
            brush_begin(&b, tool, &p, 1729, dpi); // warms the paper texture before timing
            if (tool == TOOL_ERASER || variant == 7) brush_set_erase(&b, paper.px, 0, 0, W);
            uint64_t times[N], sum = 0, h = UINT64_C(14695981039346656037);
            for (int i = 0; i < N; i++) {
                float t = (float)i / (N - 1);
                PenSample s = {20 + 1230 * t, 400 + 210 * sinf(t * 17.f),
                               0.15f + 0.85f * t, 40 * sinf(t * 11.f), 30 * cosf(t * 7.f), (uint32_t)i * 4};
                uint64_t t0 = micros();
                Rect r = brush_add(&b, &c, &s);
                times[i] = micros() - t0;
                sum += times[i];
                h = mix(mix(mix(mix(h, r.x), r.y), r.w), r.h);
            }
            for (int i = 0; i < W * H; i++) h = mix(h, c.px[i]);
            h = mix(mix(h, b.dab_index), b.restored);
            qsort(times, N, sizeof *times, cmp_time);
            printf("%s,%d,%d,%016llx,%.2f,%llu,%llu\n", tool_name(tool), bg, variant,
                   (unsigned long long)h, (double)sum / N,
                   (unsigned long long)times[N * 95 / 100], (unsigned long long)times[N - 1]);
            fflush(stdout);
        }
    }
    canvas_free(&c);
    canvas_free(&paper);
    return 0;
}
