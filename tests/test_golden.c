// test_golden.c: renders that must never change. Every paper 1.0.0-beta knows and a stroke of
// every tool, rendered at the Scribe's 1860x2480 and 300 dpi, checked against CRC32s recorded
// from the 1.0.0-beta sources (this table was made at the base of 1.0.0-beta.2, whose src/ was
// the released 1.0.0-beta's). A page someone already wrote must look exactly the same in every
// later version: if one of these fails, old pages changed. Never re-record the table to make a
// change pass; fix the change. (1.2.0's vertical highlighter tip drew old strokes with the new tip
// until 2.0.0-beta.5 recorded the tip on each stroke, issue #5: old strokes match this table
// again, and the vertical tip's own renders are listed separately below.)
//
// GRIMOIRE_GOLDEN_PRINT=1 prints the table instead of checking it (that is how it was made).
// The values hold for the host build this suite runs on (clang on arm64 macOS): another
// compiler or CPU may round floats differently, so there the comparison is skipped.
#include <string.h>

#include "font.h"
#include "store.h"
#include "testlib.h"

static const float DPI = 300.f;
enum { W = 1860, H = 2480 };
#define BETA_PAPER_COUNT 17  // the fixed kinds 1.0.0-beta knows: PAPER_NARROW..PAPER_BASS7
#define BETA_TOOL_COUNT 10   // the tool ids 1.0.0-beta knows: TOOL_PENCIL..TOOL_CALLIGRAPHY

#if defined(__clang__) && defined(__aarch64__) && defined(__APPLE__)
#define GOLDEN_HOST 1
#else
#define GOLDEN_HOST 0
#endif

// A stroke stores its settings as 13 floats (docs/FORMAT.md): ToolParams never changes size.
_Static_assert(sizeof(ToolParams) == 13 * sizeof(float), "ToolParams is frozen at 13 floats");

// 1.0.0-beta's default settings for each tool (src/brush.c, tool_params_default), frozen here:
// defaults may change in a later version, but a stroke keeps the snapshot it was drawn with.
static const ToolParams k_beta_params[BETA_TOOL_COUNT] = {
    {0.20f, 0.45f, 0.30f, 0.95f, 0.03f, 0.85f, 1.25f, 15.f, 60.f, 1.6f, 0.45f, 0.65f, 0.10f},   // pencil
    {0.26f, 0.36f, 0.38f, 0.85f, 0.03f, 0.80f, 1.10f, 25.f, 70.f, 0.35f, 0.10f, 0.50f, 0.15f},  // mechanical
    {0.30f, 0.48f, 0.85f, 1.00f, 0.02f, 0.55f, 0.60f, 30.f, 75.f, 0.10f, 0.00f, 0.06f, 0.35f},  // ballpoint
    {2.5f, 7.0f, 1.f, 1.f, 0.0f, 0.60f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.f},                  // eraser
    {4.0f, 5.0f, 0.55f, 0.65f, 0.02f, 0.50f, 0.80f, 30.f, 70.f, 0.2f, 0.f, 0.f, 0.40f},         // highlighter
    {0.35f, 1.10f, 0.90f, 1.00f, 0.02f, 0.60f, 0.80f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.30f},        // fountain
    {0.28f, 0.34f, 1.00f, 1.00f, 0.01f, 0.30f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.30f},        // fineliner
    {1.50f, 2.00f, 0.92f, 1.00f, 0.01f, 0.40f, 0.70f, 30.f, 75.f, 0.3f, 0.f, 0.04f, 0.30f},     // marker
    {0.80f, 2.40f, 0.30f, 1.00f, 0.03f, 0.85f, 1.00f, 45.f, 75.f, 2.0f, 0.35f, 0.85f, 0.15f},   // charcoal
    {0.70f, 1.40f, 0.92f, 1.00f, 0.02f, 0.60f, 0.80f, 10.f, 25.f, 0.f, 0.f, 0.f, 0.30f},        // calligraphy
};

// Three custom music layouts, as stored on disk (music_encode must still produce these).
static const struct {
    MusicLayout m;
    uint32_t stored;
} k_music[3] = {
    {{.systems = 3, .n = 5, .part = {PART_TREBLE, PART_BASS, PART_TAB6, PART_LYRICS, PART_TAB4}}, 0x4307b921u},
    {{.systems = 2, .n = 4, .part = {PART_ALTO, PART_TENOR, PART_PERC, PART_TAB7}}, 0x4200a543u},
    {{.systems = 6, .n = 3, .part = {PART_LYRICS, PART_TREBLE, PART_BASS}}, 0x4600021bu},
};

// Recorded from the 1.0.0-beta sources on 2026-10-19 (GRIMOIRE_GOLDEN_PRINT=1).
static const struct {
    const char *name;
    uint32_t crc;
} k_golden[] = {
    {"paper 0", 0x57638e39u},
    {"paper 1", 0x98cd1286u},
    {"paper 2", 0x1b0f66e6u},
    {"paper 3", 0x68b3566cu},
    {"paper 4", 0xe08d3aceu},
    {"paper 5", 0x5fa6d50cu},
    {"paper 6", 0xbda74f6du},
    {"paper 7", 0x6ecc544cu},
    {"paper 8", 0x0531cc67u},
    {"paper 9", 0x91a5d1f7u},
    {"paper 10", 0x2144d948u},
    {"paper 11", 0xca11ef8bu},
    {"paper 12", 0x1dd6d7a4u},
    {"paper 13", 0xa631f68bu},
    {"paper 14", 0xb57c5a8bu},
    {"paper 15", 0xfd5d47f4u},
    {"paper 16", 0x13fa2364u},
    {"custom music 0", 0xa34002e2u},
    {"custom music 1", 0x26ce7de9u},
    {"custom music 2", 0xb9a665aau},
    {"medium rule, pencil", 0x62d3bb3au},
    {"medium rule, pencil, ink", 0x0cd4f241u},
    {"medium rule, pencil, paper", 0x98cd1286u},
    {"medium rule, mechanical pencil", 0x897d012fu},
    {"medium rule, mechanical pencil, ink", 0x8f2475bfu},
    {"medium rule, mechanical pencil, paper", 0x98cd1286u},
    {"medium rule, ballpoint", 0x92c42d6du},
    {"medium rule, ballpoint, ink", 0xee1629ceu},
    {"medium rule, ballpoint, paper", 0x98cd1286u},
    {"medium rule, eraser", 0x98cd1286u},
    {"medium rule, eraser, ink", 0xe08d3aceu},
    {"medium rule, eraser, paper", 0x98cd1286u},
    {"medium rule, highlighter", 0x6eda0d5bu},
    {"medium rule, highlighter, ink", 0x4c159364u},
    {"medium rule, highlighter, paper", 0x98cd1286u},
    {"medium rule, fountain pen", 0x04ef983bu},
    {"medium rule, fountain pen, ink", 0x772a3f9bu},
    {"medium rule, fountain pen, paper", 0x98cd1286u},
    {"medium rule, fineliner", 0x431d1354u},
    {"medium rule, fineliner, ink", 0xd177bf82u},
    {"medium rule, fineliner, paper", 0x98cd1286u},
    {"medium rule, marker", 0xefcaee3eu},
    {"medium rule, marker, ink", 0xb203d91fu},
    {"medium rule, marker, paper", 0x98cd1286u},
    {"medium rule, charcoal", 0xb65bb58au},
    {"medium rule, charcoal, ink", 0xddce7929u},
    {"medium rule, charcoal, paper", 0x98cd1286u},
    {"medium rule, calligraphy pen", 0x3143fdc1u},
    {"medium rule, calligraphy pen, ink", 0xa42da566u},
    {"medium rule, calligraphy pen, paper", 0x98cd1286u},
    {"grand staff, pencil", 0xe78a9cd7u},
    {"grand staff, pencil, ink", 0x0cd4f241u},
    {"grand staff, pencil, paper", 0x0531cc67u},
    {"grand staff, mechanical pencil", 0xb96ee1d4u},
    {"grand staff, mechanical pencil, ink", 0x8f2475bfu},
    {"grand staff, mechanical pencil, paper", 0x0531cc67u},
    {"grand staff, ballpoint", 0x3679b20au},
    {"grand staff, ballpoint, ink", 0xee1629ceu},
    {"grand staff, ballpoint, paper", 0x0531cc67u},
    {"grand staff, eraser", 0x0531cc67u},
    {"grand staff, eraser, ink", 0xe08d3aceu},
    {"grand staff, eraser, paper", 0x0531cc67u},
    {"grand staff, highlighter", 0x665532feu},
    {"grand staff, highlighter, ink", 0x4c159364u},
    {"grand staff, highlighter, paper", 0x0531cc67u},
    {"grand staff, fountain pen", 0xc6e3a133u},
    {"grand staff, fountain pen, ink", 0x772a3f9bu},
    {"grand staff, fountain pen, paper", 0x0531cc67u},
    {"grand staff, fineliner", 0xc24dc4edu},
    {"grand staff, fineliner, ink", 0xd177bf82u},
    {"grand staff, fineliner, paper", 0x0531cc67u},
    {"grand staff, marker", 0x3d9d8de1u},
    {"grand staff, marker, ink", 0xb203d91fu},
    {"grand staff, marker, paper", 0x0531cc67u},
    {"grand staff, charcoal", 0xa5643825u},
    {"grand staff, charcoal, ink", 0xddce7929u},
    {"grand staff, charcoal, paper", 0x0531cc67u},
    {"grand staff, calligraphy pen", 0x433efb0du},
    {"grand staff, calligraphy pen, ink", 0xa42da566u},
    {"grand staff, calligraphy pen, paper", 0x0531cc67u},
    {"medium rule, every tool", 0x1d589bf4u},
    {"medium rule, every tool, ink", 0xe653463au},
    {"medium rule, every tool, paper", 0x98cd1286u},
    {"medium rule, every tool, file of 26678 bytes", 0x1ab5b75fu},
    {"medium rule, every tool, read back", 0xb91c7feau},
    {"grand staff, every tool", 0x23997deau},
    {"grand staff, every tool, ink", 0xe653463au},
    {"grand staff, every tool, paper", 0x0531cc67u},
    {"grand staff, every tool, file of 26678 bytes", 0x4bc35d72u},
    {"grand staff, every tool, read back", 0x09bc38ebu},
};

static uint32_t crc_table[256];
static uint32_t crc32_bytes(uint32_t crc, const uint8_t *p, size_t n) {
    if (!crc_table[1])
        for (uint32_t i = 0; i < 256; i++) {
            uint32_t c = i;
            for (int k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
            crc_table[i] = c;
        }
    crc = ~crc;
    for (size_t i = 0; i < n; i++) crc = crc_table[(crc ^ p[i]) & 0xFF] ^ (crc >> 8);
    return ~crc;
}

static uint32_t crc_canvas(const Canvas *c) {
    uint32_t crc = 0;
    for (int y = 0; y < c->h; y++) crc = crc32_bytes(crc, c->px + (size_t)y * c->stride, (size_t)c->w);
    return crc;
}

// The highlighter stroke above drawn with the vertical tip (TIP_HIGHLIGHTER_VERTICAL, since 1.2.0),
// as strokes drawn since 2.0.0-beta.5 record it. These are the renders 2.0.0-beta.2 to beta.4
// gave every highlighter stroke, old ones too.
static const struct { const char *name; uint32_t crc; } k_vertical_tip[] = {
    {"medium rule, highlighter, vertical tip", 0xe1df8dfcu},
    {"medium rule, highlighter, vertical tip, ink", 0xe83cb220u},
    {"medium rule, highlighter, vertical tip, paper", 0x98cd1286u},
    {"grand staff, highlighter, vertical tip", 0xb2bc9b09u},
    {"grand staff, highlighter, vertical tip, ink", 0xe83cb220u},
    {"grand staff, highlighter, vertical tip, paper", 0x0531cc67u},
};

static bool g_print;

static void golden(const char *name, uint32_t crc) {
    if (g_print) {
        printf("    {\"%s\", 0x%08xu},\n", name, crc);
        return;
    }
    if (!GOLDEN_HOST) return;
    for (size_t i = 0; i < sizeof k_golden / sizeof k_golden[0] + sizeof k_vertical_tip / sizeof k_vertical_tip[0]; i++) {
        size_t ng = sizeof k_golden / sizeof k_golden[0];
        const char *n = i < ng ? k_golden[i].name : k_vertical_tip[i - ng].name;
        if (strcmp(n, name)) continue;
        uint32_t expected = i < ng ? k_golden[i].crc : k_vertical_tip[i - ng].crc;
        CHECK(expected == crc);
        if (expected != crc) fprintf(stderr, "  %s: 0x%08x, recorded 0x%08x\n", name, crc, expected);
        return;
    }
    fprintf(stderr, "  %s: not in the table\n", name);
    CHECK(!"every render has a recorded value");
}

// One stroke across the page: a zigzag whose pressure starts below every deadzone, swells to
// full and fades again, with the pen leaning from left to right and nodding, so each tool shows
// its widths, darkness, grain, tilt shading and (calligraphy) its turning nib. Plain arithmetic
// only; seeds stay below 2^24.
static Stroke *golden_stroke(int tool, float y0, uint32_t seed) {
    Stroke *s = stroke_new((ToolKind)tool, &k_beta_params[tool], seed);
    enum { N = 161 };
    for (int i = 0; i < N; i++) {
        float f = (float)i / (float)(N - 1);
        float tri = fabsf(fmodf(f * 6.f, 2.f) - 1.f);
        float bell = 1.f - (2.f * f - 1.f) * (2.f * f - 1.f);
        PenSample q = {220.f + 1420.f * f, y0 + 170.f * tri - 85.f + 60.f * f * f, i < 3 ? 0.f : 0.04f + 0.96f * bell,
                       -60.f + 120.f * f, 50.f * tri - 15.f, (uint32_t)i * 7u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

static uint32_t seed_for(int tool) { return 0x00A5F00Du + (uint32_t)tool * 40503u; }

// Names for the table, fixed here (paper_name and tool_name may be translated one day).
static const char *const k_tool_key[BETA_TOOL_COUNT] = {"pencil", "mechanical pencil", "ballpoint", "eraser",
                                                        "highlighter", "fountain pen", "fineliner", "marker",
                                                        "charcoal", "calligraphy pen"};
static const char *paper_key(PaperKind k) { return k == PAPER_MEDIUM ? "medium rule" : "grand staff"; }

// Renders `pg` three ways: whole, ink only (on white) and paper only.
static void golden_page(Canvas *c, const Page *pg, const char *name) {
    char key[96];
    const Rect full = {0, 0, W, H};
    page_render(c, pg, full, DPI);
    snprintf(key, sizeof key, "%s", name);
    golden(key, crc_canvas(c));
    canvas_fill(c, full, 255);
    page_render_layers(c, pg, full, DPI, PAGE_LAYER_INK);
    snprintf(key, sizeof key, "%s, ink", name);
    golden(key, crc_canvas(c));
    canvas_fill(c, full, 0);
    page_render_layers(c, pg, full, DPI, PAGE_LAYER_PAPER);
    snprintf(key, sizeof key, "%s, paper", name);
    golden(key, crc_canvas(c));
}

TEST(papers) {
    Canvas c;
    CHECK(canvas_init(&c, W, H) == 0);
    char key[64];
    for (int gothic = 0; gothic < 2; gothic++) {
        // Paper labels and clefs come from Terminess in both themes: the theme never shows on a page.
        font_set_gothic(gothic != 0);
        for (int k = 0; k < BETA_PAPER_COUNT; k++) {
            canvas_fill(&c, (Rect){0, 0, W, H}, 0);
            paper_render(&c, (PaperKind)k, (Rect){0, 0, W, H}, DPI);
            snprintf(key, sizeof key, "paper %d", k);
            if (!gothic || !g_print) golden(key, crc_canvas(&c));
        }
        for (int i = 0; i < 3; i++) {
            PaperKind k = music_encode(&k_music[i].m);
            CHECK((uint32_t)k == k_music[i].stored);
            canvas_fill(&c, (Rect){0, 0, W, H}, 0);
            paper_render(&c, (PaperKind)k_music[i].stored, (Rect){0, 0, W, H}, DPI);
            snprintf(key, sizeof key, "custom music %d", i);
            if (!gothic || !g_print) golden(key, crc_canvas(&c));
        }
    }
    font_set_gothic(false);
    canvas_free(&c);
}

TEST(one_stroke_of_every_tool) {
    static const PaperKind papers[2] = {PAPER_MEDIUM, PAPER_GRAND};
    Canvas c;
    CHECK(canvas_init(&c, W, H) == 0);
    char key[64];
    for (int pi = 0; pi < 2; pi++)
        for (int t = 0; t < BETA_TOOL_COUNT; t++) {
            Page *pg = page_new(W, H, papers[pi]);
            page_add(pg, golden_stroke(t, 1240.f, seed_for(t)));
            snprintf(key, sizeof key, "%s, %s", paper_key(papers[pi]), k_tool_key[t]);
            golden_page(&c, pg, key);
            if (t == TOOL_HIGHLIGHTER) {  // the same stroke as drawn since 2.0.0-beta.5: vertical tip
                stroke_mark_tip(pg->strokes[0]);
                CHECK(stroke_tip(pg->strokes[0]) == TIP_HIGHLIGHTER_VERTICAL);
                snprintf(key, sizeof key, "%s, highlighter, vertical tip", paper_key(papers[pi]));
                golden_page(&c, pg, key);
            }
            page_free(pg);
        }
    canvas_free(&c);
}

// Every drawing tool on one page, overlapping its neighbours (no eraser: no released build
// ever wrote one). The same pages also go through the file format: the bytes 1.0.0-beta wrote
// for them, and the page read back from those bytes, must not change either.
TEST(all_tools_on_one_page) {
    static const PaperKind papers[2] = {PAPER_MEDIUM, PAPER_GRAND};
    Canvas c;
    CHECK(canvas_init(&c, W, H) == 0);
    char key[96];
    for (int pi = 0; pi < 2; pi++) {
        Page *pg = page_new(W, H, papers[pi]);
        int row = 0;
        for (int t = 0; t < BETA_TOOL_COUNT; t++) {
            if (t == TOOL_ERASER) continue;
            page_add(pg, golden_stroke(t, 260.f + 245.f * (float)row++, seed_for(t)));
        }
        snprintf(key, sizeof key, "%s, every tool", paper_key(papers[pi]));
        golden_page(&c, pg, key);
        uint8_t *bytes = NULL;
        size_t len = 0;
        CHECK(page_serialize(pg, &bytes, &len) == 0);
        snprintf(key, sizeof key, "%s, every tool, file of %zu bytes", paper_key(papers[pi]), len);
        golden(key, crc32_bytes(0, bytes, len));
        Page *back = NULL;
        CHECK(page_deserialize(bytes, len, &back) == 0 && back);
        if (back) {
            CHECK(back->n == pg->n && back->paper == pg->paper);
            page_render(&c, back, (Rect){0, 0, W, H}, DPI);
            snprintf(key, sizeof key, "%s, every tool, read back", paper_key(papers[pi]));
            golden(key, crc_canvas(&c));
            page_free(back);
        }
        free(bytes);
        page_free(pg);
    }
    canvas_free(&c);
}

int main(void) {
    g_print = getenv("GRIMOIRE_GOLDEN_PRINT") != NULL;
    CHECK(font_init() == 0);  // papers draw labels and clefs
    CHECK(PAPER_COUNT >= BETA_PAPER_COUNT && TOOL_COUNT >= BETA_TOOL_COUNT);
    if (!GOLDEN_HOST && !g_print) fprintf(stderr, "  golden: not the recording host (clang, arm64 macOS): renders not compared\n");
    if (g_print) printf("static const struct {\n    const char *name;\n    uint32_t crc;\n} k_golden[] = {\n");
    RUN(papers);
    RUN(one_stroke_of_every_tool);
    RUN(all_tools_on_one_page);
    if (g_print) printf("};\n");
    return tl_done();
}
