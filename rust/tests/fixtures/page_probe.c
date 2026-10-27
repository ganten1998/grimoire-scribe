// page_probe.c: prints, for strokes of every tool, how many of a few thousand hit tests hit and one
// hash of all their answers, for comparing the Rust port of page.c's stroke_hit_segment (and so
// stroke_hit) with this C build (clang -ffp-contract=on, as the Makefile's test flags). The
// strokes and the queries are made with integer arithmetic, exact float conversions and double
// arithmetic on values that are exact in it, so the Rust twin (rust/tests/test_page_probe.rs)
// builds the very same inputs. Besides random segments there are the cases the fused multiply-add
// decides: query segments lying along a stroke's own segments (almost collinear: the sign of the
// cross products in segs_intersect is rounding noise) and ones sharing a point with it.
#include "page.h"

#include <stdio.h>
#include <string.h>

typedef uint64_t u64;

static u64 fnv_u32(u64 h, uint32_t v) {
    for (int i = 0; i < 4; i++) {
        h ^= (uint8_t)(v >> (8 * i));
        h *= 1099511628211ull;
    }
    return h;
}
#define H0 14695981039346656037ull

static uint32_t rng(uint32_t *s) {
    *s = *s * 1664525u + 1013904223u;
    return *s >> 8;
}

// Stubs for what page.c reaches for and the probe never draws.
void bgimage_render(Canvas *c, const char *nb, PaperKind k, Rect region) { (void)c; (void)nb; (void)k; (void)region; }
void paper_render(Canvas *c, PaperKind k, Rect region, float dpi) { (void)c; (void)k; (void)region; (void)dpi; }
bool paper_is_image(PaperKind k) { (void)k; return false; }

static const ToolKind TOOLS[] = {TOOL_PENCIL, TOOL_MECH_PENCIL, TOOL_BALLPOINT, TOOL_ERASER, TOOL_HIGHLIGHTER,
                                 TOOL_FOUNTAIN, TOOL_FINELINER, TOOL_MARKER, TOOL_CHARCOAL, TOOL_CALLIGRAPHY};
enum { NT = sizeof TOOLS / sizeof TOOLS[0], NS = 8, NQ = 3000 };

// A coordinate on a 400 px square from the base, in 1/1000 px (not exact in a float).
static float fx(uint32_t *r, float base) { return base + (float)(rng(r) % 400001) / 1000.f; }

// a + (b - a) * t, in double, for values that make every step exact there; to float once.
static float along(float a, float b, float t) { return (float)((double)a + ((double)b - (double)a) * (double)t); }

int main(void) {
    for (int ti = 0; ti < NT; ti++)
        for (int si = 0; si < NS; si++) {
            uint32_t r = 0x1234u + (uint32_t)ti * 131u + (uint32_t)si * 7919u;
            ToolParams prm;
            tool_params_default(TOOLS[ti], &prm);
            Stroke *s = stroke_new(TOOLS[ti], &prm, 77u + (uint32_t)ti);
            int n = si == 0 ? 1 : si == 1 ? 2 : 1 + (int)(rng(&r) % 40);
            float bx = (float)(rng(&r) % 1200), by = (float)(rng(&r) % 1600);
            for (int i = 0; i < n; i++) {
                PenSample q = {fx(&r, bx), fx(&r, by), (float)(rng(&r) % 1001) / 1000.f, (float)((int)(rng(&r) % 141) - 70),
                               (float)((int)(rng(&r) % 141) - 70), (uint32_t)i * 7u};
                stroke_append(s, &q, 300.f);
            }
            u64 h = H0;
            int hits = 0;
            for (int qi = 0; qi < NQ; qi++) {
                float x0, y0, x1, y1, rad = (float)(rng(&r) % 20001) / 1000.f;
                int j = (int)(rng(&r) % (uint32_t)n);
                const PenSample *a = &s->pts[j], *b = &s->pts[j + 1 < n ? j + 1 : j];
                switch (qi % 4) {
                case 0:  // anywhere near
                    x0 = fx(&r, bx); y0 = fx(&r, by); x1 = fx(&r, bx); y1 = fx(&r, by);
                    break;
                case 1: {  // along the stroke's own segment, past both ends
                    float t0 = (float)((int)(rng(&r) % 3073) - 1024) / 1024.f, t1 = (float)((int)(rng(&r) % 3073) - 1024) / 1024.f;
                    x0 = along(a->x, b->x, t0); y0 = along(a->y, b->y, t0);
                    x1 = along(a->x, b->x, t1); y1 = along(a->y, b->y, t1);
                    rad = (float)(rng(&r) % 3u) * 0.25f;
                    break;
                }
                case 2:  // from a point of the stroke
                    x0 = a->x; y0 = a->y; x1 = fx(&r, bx); y1 = fx(&r, by);
                    rad = 0.f;
                    break;
                default:  // a point (stroke_hit)
                    x0 = x1 = fx(&r, bx); y0 = y1 = fx(&r, by);
                    break;
                }
                bool hit = qi % 4 == 3 ? stroke_hit(s, x0, y0, rad, 300.f) : stroke_hit_segment(s, x0, y0, x1, y1, rad, 300.f);
                h = fnv_u32(h, hit);
                hits += hit;
            }
            printf("hit t%d s%d %016llx %d\n", ti, si, (unsigned long long)h, hits);
            stroke_free(s);
        }
    return 0;
}
