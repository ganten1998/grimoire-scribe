// brush_probe.c: prints one hash per brush scenario, for comparing the Rust port with this C build
// (clang -ffp-contract=on, as the Makefile's test flags). The scenarios are generated with integer
// arithmetic and exact float conversions only, so the Rust twin (rust/tests/test_brush.rs) builds
// the very same inputs.
#include "brush.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

typedef uint64_t u64;

static u64 fnv(u64 h, const void *p, size_t n) {
    const uint8_t *b = p;
    for (size_t i = 0; i < n; i++) {
        h ^= b[i];
        h *= 1099511628211ull;
    }
    return h;
}
static u64 fnv_u32(u64 h, uint32_t v) {
    uint8_t b[4] = {(uint8_t)v, (uint8_t)(v >> 8), (uint8_t)(v >> 16), (uint8_t)(v >> 24)};
    return fnv(h, b, 4);
}
static u64 fnv_f(u64 h, float f) {
    uint32_t v;
    memcpy(&v, &f, 4);
    return fnv_u32(h, v);
}
#define H0 14695981039346656037ull

static uint32_t rng(uint32_t *s) {
    *s = *s * 1664525u + 1013904223u;
    return *s >> 8;
}

enum { N = 48, CW = 320, CH = 240 };

static void make_stroke(PenSample *out, int n, uint32_t seed) {
    uint32_t r = seed;
    int xi = 600 + (int)(rng(&r) % 1200), yi = 400 + (int)(rng(&r) % 900);
    for (int i = 0; i < n; i++) {
        xi += (int)(rng(&r) % 161) - 60;
        yi += (int)(rng(&r) % 161) - 80;
        if (xi < 160) xi = 160 + (int)(rng(&r) % 40);
        if (xi > 2400) xi = 2400 - (int)(rng(&r) % 40);
        if (yi < 160) yi = 160 + (int)(rng(&r) % 40);
        if (yi > 1760) yi = 1760 - (int)(rng(&r) % 40);
        out[i].x = (float)xi / 8.f;
        out[i].y = (float)yi / 8.f;
        out[i].pressure = (float)(rng(&r) % 1001) / 1000.f;
        out[i].tilt_x = (float)((int)(rng(&r) % 141) - 70);
        out[i].tilt_y = (float)((int)(rng(&r) % 141) - 70);
        out[i].t_ms = 1000u + (uint32_t)i * 7u;
    }
}

static void variant(ToolKind t, int var, ToolParams *p) {
    tool_params_default(t, p);
    if (var == 1) {
        p->grain = 0.9f;
        p->tilt_size = 2.f;
        p->tilt_opacity = 0.5f;
        p->smoothing = 0.f;
    } else if (var == 2) {
        p->pressure_gamma = 0.6f;
        p->pressure_floor = 0.1f;
        p->pressure_ceiling = 0.8f;
        p->size_max_mm = p->size_max_mm * 3.f;
        p->grain = 0.4f;
    }
}

static const ToolKind TOOLS[] = {TOOL_PENCIL, TOOL_MECH_PENCIL, TOOL_BALLPOINT, TOOL_ERASER, TOOL_HIGHLIGHTER,
                                 TOOL_FOUNTAIN, TOOL_FINELINER, TOOL_MARKER, TOOL_CHARCOAL, TOOL_CALLIGRAPHY, (ToolKind)12};
enum { NT = sizeof TOOLS / sizeof TOOLS[0] };

static void gradient(uint8_t *px) {
    for (int y = 0; y < CH; y++)
        for (int x = 0; x < CW; x++) px[y * CW + x] = (uint8_t)((x * 3 + y * 5 + ((x * y) >> 5)) & 255);
}

static void make_paper(uint8_t *paper) {
    for (int y = 0; y < CH; y++)
        for (int x = 0; x < CW; x++) paper[y * CW + x] = (uint8_t)(y % 17 == 0 ? 90 : 235 + (x + y) % 21);
}

#ifndef REPS
#define REPS 1
#endif

static void scenario(int ti, int var, float dpi, int mode, int rep) {
    static PenSample st[N];
    static uint8_t paper[CW * CH], px[CW * CH];
    ToolKind t = TOOLS[ti];
    ToolParams p;
    variant(t, var, &p);
    make_stroke(st, N, 77u + (uint32_t)(ti * 31 + var * 7 + mode) + (uint32_t)rep * 1000003u);
    Canvas c = {0};
    if (mode == 5) {
        c.px = NULL;
        c.w = CW;
        c.h = CH;
        c.stride = CW;
        c.clip = (Rect){0, 0, CW, CH};
    } else {
        canvas_init(&c, CW, CH);
        if (mode != 0) gradient(c.px);
        if (mode == 6 || mode == 7) c.clip = (Rect){40, 30, 200, 150};
    }
    make_paper(paper);
    BrushState b;
    brush_begin(&b, t, &p, 0xC0FFEEu + (uint32_t)ti + (uint32_t)rep, dpi);
    if (mode == 2) brush_set_erase(&b, NULL, 0, 0, 0);
    if (mode == 3 || mode == 4 || mode == 7) brush_set_erase(&b, paper, 0, 0, CW);
    if (mode == 5) b.dry = true;
    g_brush_erase_spans = mode != 4;
    Rect u = RECT_EMPTY;
    Smoother sm;
    smoother_reset(&sm);
    for (int i = 0; i < N; i++) {
        PenSample s = st[i];
        if (var == 2) smoother_apply(&sm, p.smoothing + 0.5f, &s);
        u = rect_union(u, brush_add(&b, &c, &s));
    }
    g_brush_erase_spans = true;
    u64 h = H0;
    if (c.px) h = fnv(h, c.px, (size_t)CW * CH);
    h = fnv_u32(h, (uint32_t)u.x);
    h = fnv_u32(h, (uint32_t)u.y);
    h = fnv_u32(h, (uint32_t)u.w);
    h = fnv_u32(h, (uint32_t)u.h);
    h = fnv_u32(h, b.restored);
    h = fnv_f(h, b.carry);
    h = fnv_u32(h, b.dab_index);
    h = fnv_f(h, b.nib_c);
    h = fnv_f(h, b.nib_s);
    if (rep) printf("stroke t%d v%d dpi%d m%d r%d %016llx\n", (int)t, var, (int)dpi, mode, rep, (unsigned long long)h);
    else printf("stroke t%d v%d dpi%d m%d %016llx\n", (int)t, var, (int)dpi, mode, (unsigned long long)h);
    if (c.px) canvas_free(&c);
    (void)px;
}

static void pure(void) {
    for (int ti = 0; ti < NT; ti++)
        for (int var = 0; var < 3; var++) {
            ToolKind t = TOOLS[ti];
            ToolParams p;
            variant(t, var, &p);
            u64 h = H0;
            for (int i = 0; i <= 20; i++) h = fnv_f(h, brush_pressure(&p, (float)i / 20.f));
            h = fnv_f(h, brush_pressure(&p, NAN));
            h = fnv_f(h, brush_pressure(&p, -3.f));
            h = fnv_f(h, brush_pressure(&p, 7.f));
            for (int tx = -90; tx <= 90; tx += 15)
                for (int ty = -90; ty <= 90; ty += 15) {
                    h = fnv_f(h, brush_tilt(&p, (float)tx, (float)ty));
                    h = fnv_f(h, brush_nib_angle(&p, (float)tx, (float)ty));
                }
            h = fnv_f(h, brush_tilt(&p, NAN, 5.f));
            h = fnv_f(h, brush_nib_angle(&p, 0.f, 0.f));
            static PenSample st[N];
            make_stroke(st, N, 900u + (uint32_t)(ti * 3 + var));
            for (int i = 0; i < N; i++) {
                float r, op;
                brush_dab(t, &p, &st[i], 226.f + (float)i, &r, &op);
                h = fnv_f(h, r);
                h = fnv_f(h, op);
            }
            for (int d = 100; d <= 600; d += 125) h = fnv_f(h, brush_max_radius(t, &p, (float)d));
            printf("pure t%d v%d %016llx\n", (int)t, var, (unsigned long long)h);
        }
    // the param table, the defaults, the names and the tips
    u64 h = H0;
    int n;
    const ParamInfo *tab = tool_param_table(&n);
    h = fnv_u32(h, (uint32_t)n);
    for (int i = 0; i < n; i++) {
        h = fnv(h, tab[i].key, strlen(tab[i].key));
        h = fnv(h, tab[i].label, strlen(tab[i].label));
        h = fnv(h, tab[i].unit, strlen(tab[i].unit));
        h = fnv_u32(h, (uint32_t)(tab[i].offset / sizeof(float)));
        h = fnv_f(h, tab[i].min);
        h = fnv_f(h, tab[i].max);
        h = fnv_f(h, tab[i].step);
        h = fnv_u32(h, tab[i].eraser_uses);
    }
    for (int ti = 0; ti < NT; ti++) {
        ToolParams p;
        tool_params_default(TOOLS[ti], &p);
        h = fnv(h, &p, sizeof p);
        const char *nm = tool_name(TOOLS[ti]);
        h = fnv(h, nm, strlen(nm));
        h = fnv_u32(h, brush_tip_current(TOOLS[ti]));
        for (int v = 0; v < 3; v++) h = fnv_u32(h, brush_tip_known(TOOLS[ti], (uint8_t)v));
    }
    for (int i = 0; i < TOOL_COUNT; i++) h = fnv_u32(h, (uint32_t)tool_order[i]);
    printf("table %016llx\n", (unsigned long long)h);
}

static void smoothing(void) {
    static PenSample st[N];
    for (int a = 0; a < 5; a++) {
        float amount = (float)a * 0.25f;
        Smoother sm;
        smoother_reset(&sm);
        make_stroke(st, N, 5150u + (uint32_t)a);
        u64 h = H0;
        for (int i = 0; i < N; i++) {
            PenSample s = st[i];
            smoother_apply(&sm, amount, &s);
            h = fnv_f(h, s.x);
            h = fnv_f(h, s.y);
            h = fnv_f(h, s.pressure);
        }
        printf("smooth a%d %016llx\n", a, (unsigned long long)h);
    }
}

// a corrupt jump is not bridged; a stroke of one sample is one dab; times running backwards
static void odd(void) {
    Canvas c;
    canvas_init(&c, CW, CH);
    ToolParams p;
    tool_params_default(TOOL_PENCIL, &p);
    BrushState b;
    brush_begin(&b, TOOL_PENCIL, &p, 5u, 300.f);
    PenSample s = {100.f, 100.f, 0.7f, 10.f, 20.f, 5000u};
    brush_add(&b, &c, &s);
    PenSample j = {30000.f, 100.f, 0.7f, 10.f, 20.f, 100u};
    Rect d = brush_add(&b, &c, &j);
    PenSample k = {150.f, 120.f, 0.2f, 0.f, 0.f, 20u};
    Rect e = brush_add(&b, &c, &k);
    u64 h = fnv(H0, c.px, (size_t)CW * CH);
    h = fnv_u32(h, (uint32_t)d.w);
    h = fnv_u32(h, (uint32_t)e.x);
    h = fnv_u32(h, (uint32_t)e.w);
    h = fnv_f(h, b.carry);
    h = fnv_u32(h, b.prev.t_ms);
    printf("odd %016llx\n", (unsigned long long)h);
    canvas_free(&c);
}

int main(void) {
    pure();
    smoothing();
    odd();
    const float dpis[2] = {300.f, 226.f};
    // (-DREPS=n: n times the strokes, each repeat with other samples, for a wider sweep than the fixture)
    for (int rep = 0; rep < REPS; rep++)
        for (int ti = 0; ti < NT; ti++)
            for (int var = 0; var < 3; var++)
                for (int di = 0; di < 2; di++)
                    for (int mode = 0; mode < 8; mode++) scenario(ti, var, dpis[di], mode, rep);
    return 0;
}
