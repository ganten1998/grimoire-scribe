// test_format.c: carrying what a newer version writes (docs/FORMAT.md). Pages this version
// writes are checked against a frozen copy of the 1.0.0-beta reader and writer (beta_format.h):
// 1.0.0-beta must open every one of them, and its re-save must lose only what FORMAT.md says.
// This version must carry all of it through a save, open newer pages read-only instead of setting
// them aside, bring back pages 1.0.0-beta set aside, and keep ini lines it does not know.
#include <dirent.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "beta_format.h"
#include "store.h"
#include "testlib.h"

static char root[600];
static const float DPI = 300.f;

// ---- building pages ----

static Stroke *wave(ToolKind t, float y, uint32_t seed, const uint8_t ext[3]) {
    ToolParams p;
    tool_params_default(t, &p);
    Stroke *s = stroke_new(t, &p, seed);
    if (ext) memcpy(s->ext, ext, 3);
    for (int i = 0; i < 24; i++) {
        PenSample q = {200.f + (float)i * 50.f, y + (float)(i % 5) * 9.f, 0.1f + (float)i / 30.f, (float)i - 12.f,
                       5.f - (float)i * 0.5f, (uint32_t)i * 6u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// A chunk: tag, u32 length, payload.
static size_t chunk(uint8_t *out, const char tag[4], const char *payload, size_t n) {
    memcpy(out, tag, 4);
    out[4] = (uint8_t)n;
    out[5] = (uint8_t)(n >> 8);
    out[6] = (uint8_t)(n >> 16);
    out[7] = (uint8_t)(n >> 24);
    memcpy(out + 8, payload, n);
    return 8 + n;
}

static void set_tail(Page *p, const uint8_t *d, size_t n) {
    free(p->tail);
    p->tail = malloc(n);
    memcpy(p->tail, d, n);
    p->tail_len = n;
}

enum {
    K_PLAIN, K_EMPTY, K_CUSTOM_MUSIC, K_PADS, K_ERASE_TEXTURE, K_SAFE_CHUNKS, K_CRITICAL_CHUNK, K_IMAGE_PAPER,
    K_MUSIC_NEWER, K_EMPTY_NEWER_PAPER, K_EVERYTHING, K_COUNT
};
static const char *const k_kind_name[K_COUNT] = {"plain",          "empty",         "custom music",
                                                 "pad bytes",      "eraser seeds",  "safe chunks",
                                                 "critical chunk", "image paper",   "newer music part",
                                                 "empty, newer paper", "everything"};

static const uint32_t IMAGE_PAPER = 0x50000001u;  // a 1.1 image paper id (namespace 0x5)
static const uint32_t MUSIC_NEWER = 0x4100000Cu;  // custom music with a part this version lacks

static MusicLayout custom_layout(void) {
    return (MusicLayout){.systems = 4, .n = 3, .part = {PART_TREBLE, PART_BASS, PART_LYRICS}};
}

// Whether the kind's page carries data 1.0.0-beta does not keep.
static bool has_new_data(int k) { return k != K_PLAIN && k != K_EMPTY && k != K_CUSTOM_MUSIC; }

static PaperKind kind_paper(int kind) {
    MusicLayout m = custom_layout();
    switch (kind) {
    case K_CUSTOM_MUSIC: return music_encode(&m);
    case K_IMAGE_PAPER:
    case K_EMPTY_NEWER_PAPER:
    case K_EVERYTHING: return (PaperKind)IMAGE_PAPER;
    case K_MUSIC_NEWER: return (PaperKind)MUSIC_NEWER;
    default: return PAPER_MEDIUM;
    }
}

static Page *make_page(int kind) {
    static const uint8_t pads[3][3] = {{3, 1, 0x80}, {0, 2, 0}, {255, 255, 255}};
    Page *p = page_new(1860, 2480, kind_paper(kind));
    if (kind == K_EMPTY || kind == K_EMPTY_NEWER_PAPER) return p;
    bool pad = kind == K_PADS || kind == K_EVERYTHING;
    page_add(p, wave(TOOL_PENCIL, 300.f, 11u, pad ? pads[0] : NULL));
    page_add(p, wave(TOOL_BALLPOINT, 600.f, 22u, pad ? pads[1] : NULL));
    page_add(p, wave(TOOL_HIGHLIGHTER, 900.f, 33u, pad ? pads[2] : NULL));
    if (kind == K_ERASE_TEXTURE || kind == K_EVERYTHING) {
        // Erase strokes: a white-ink texture in seed bits 24-31 (tool + 1), and a round tip (0).
        page_add(p, wave(TOOL_ERASER, 1200.f, (uint32_t)(TOOL_CHARCOAL + 1) << 24 | 0x123456u, NULL));
        page_add(p, wave(TOOL_ERASER, 1500.f, 0x00ABCDEFu, NULL));
    }
    uint8_t tail[256];
    size_t n = 0;
    if (kind == K_SAFE_CHUNKS || kind == K_EVERYTHING) {
        n += chunk(tail + n, "layr", "\x01\x00\x02\x00" "base", 8);
        n += chunk(tail + n, "pal0", "", 0);
        n += chunk(tail + n, "zz99", "0123456789abcdefXYZ", 19);
    }
    if (kind == K_CRITICAL_CHUNK) {
        n += chunk(tail + n, "note", "safe first", 10);
        n += chunk(tail + n, "LAYR", "\x02\x00", 2);
    }
    if (n) set_tail(p, tail, n);
    return p;
}

// ---- reading the bytes back by hand ----

static uint32_t rd32(const uint8_t *d) { return (uint32_t)d[0] | (uint32_t)d[1] << 8 | (uint32_t)d[2] << 16 | (uint32_t)d[3] << 24; }
static uint16_t rd16(const uint8_t *d) { return (uint16_t)(d[0] | d[1] << 8); }

// Offsets of each stroke record, and where the strokes end. Returns the stroke count.
static uint32_t walk(const uint8_t *d, size_t len, size_t *rec, uint32_t cap, size_t *end) {
    size_t pos = rd16(d + 4) == 2 ? 24 : 20;
    uint32_t ns = rd32(d + pos - 4);
    for (uint32_t i = 0; i < ns; i++) {
        if (i < cap) rec[i] = pos;
        uint32_t np = rd32(d + pos + 60);
        pos += 64 + (size_t)np * 18;
        if (pos > len) break;
    }
    *end = pos;
    return ns;
}

static int count_files(const char *dir, const char *prefix) {
    int n = 0;
    DIR *d = opendir(dir);
    for (struct dirent *e; d && (e = readdir(d));)
        if (!strncmp(e->d_name, prefix, strlen(prefix))) n++;
    if (d) closedir(d);
    return n;
}

static uint8_t *slurp(const char *path, size_t *len) { return read_file(path, len); }

static bool same_file(const char *path, const uint8_t *d, size_t len) {
    size_t l2;
    uint8_t *f = slurp(path, &l2);
    bool same = f && l2 == len && !memcmp(f, d, len);
    free(f);
    return same;
}

// ---- tests ----

TEST(papers_newer_than_this_version) {
    CHECK(paper_is_known(PAPER_NARROW) && paper_is_known(PAPER_BASS7));
    MusicLayout m = custom_layout();
    CHECK(paper_is_known(music_encode(&m)));
    CHECK(!paper_is_known((PaperKind)PAPER_COUNT) && !paper_is_known((PaperKind)IMAGE_PAPER));
    CHECK(!paper_is_known((PaperKind)MUSIC_NEWER) && !paper_is_known((PaperKind)PAPER_CUSTOM_FLAG));
    CHECK(!paper_is_known((PaperKind)0x60000123u) && !paper_is_known((PaperKind)0xFFFFFFFFu));
    CHECK(!strcmp(paper_name((PaperKind)IMAGE_PAPER), "Newer paper"));
    CHECK(!strcmp(paper_name((PaperKind)MUSIC_NEWER), "Newer paper"));
    CHECK(!strcmp(paper_name(music_encode(&m)), "Custom music"));
    // Picker choices and new notebooks still need a known kind.
    CHECK(paper_sanitize(IMAGE_PAPER, PAPER_MEDIUM) == PAPER_MEDIUM);
    CHECK(paper_sanitize(PAPER_GRID, PAPER_MEDIUM) == PAPER_GRID);
    // A page keeps whatever paper it is given, and draws an unknown one as blank paper.
    Page *p = page_new(186, 248, (PaperKind)IMAGE_PAPER);
    CHECK((uint32_t)p->paper == IMAGE_PAPER && !p->readonly && !p->tail && p->tail_len == 0);
    Canvas a, b;
    canvas_init(&a, 186, 248);
    canvas_init(&b, 186, 248);
    canvas_fill(&a, (Rect){0, 0, 186, 248}, 0);
    page_render(&a, p, (Rect){0, 0, 186, 248}, 30.f);
    paper_render(&b, PAPER_BLANK, (Rect){0, 0, 186, 248}, 30.f);
    CHECK(memcmp(a.px, b.px, 186 * 248) == 0);
    for (uint32_t k = PAPER_COUNT; k < PAPER_COUNT + 40; k++) {  // every unknown kind: blank
        paper_render(&a, (PaperKind)(k * 0x0F0F0F1u), (Rect){0, 0, 186, 248}, 30.f);
        if (!paper_is_known((PaperKind)(k * 0x0F0F0F1u))) CHECK(memcmp(a.px, b.px, 186 * 248) == 0);
    }
    canvas_free(&a);
    canvas_free(&b);
    page_free(p);
    Stroke *s = stroke_new(TOOL_PENCIL, &(ToolParams){0}, 5u);
    CHECK(s->ext[0] == 0 && s->ext[1] == 0 && s->ext[2] == 0);
    stroke_free(s);
}

// (a) 1.0.0-beta opens every page this version writes, with every stroke; (b) its re-save keeps
// the strokes and seeds, zeroes the pad bytes, drops the chunks and turns an unknown paper into
// version 1 blank; (c) this version reads it all back and writes the same bytes again.
TEST(beta_opens_what_we_write) {
    for (int k = 0; k < K_COUNT; k++) {
        Page *p = make_page(k);
        uint8_t *d = NULL;
        size_t len = 0;
        CHECK(page_serialize(p, &d, &len) == 0);
        if (!d) continue;
        uint16_t ver = rd16(d + 4);
        CHECK(ver == 1 || ver == 2);  // never a version 1.0.0-beta sets aside
        bool known = paper_is_known(p->paper);
        CHECK(ver == ((uint32_t)p->paper >= PAPER_COUNT ? 2 : 1));
        if (ver == 2) CHECK(rd16(d + 6) == (paper_is_custom(p->paper) ? PAPER_MUSIC : PAPER_BLANK));
        // (a)
        BetaPage bp;
        int rc = beta_page_deserialize(d, len, &bp);
        CHECK(rc == 0);
        if (rc == 0) {
            CHECK(bp.n == (uint32_t)p->n);
            CHECK(bp.paper == (known ? (uint32_t)p->paper : BETA_PAPER_BLANK));
            for (uint32_t i = 0; i < bp.n && i < (uint32_t)p->n; i++) {
                CHECK(bp.strokes[i].tool == (uint8_t)p->strokes[i]->tool);
                CHECK(bp.strokes[i].seed == p->strokes[i]->seed);
                CHECK(bp.strokes[i].n == (uint32_t)p->strokes[i]->n);
            }
            beta_page_free(&bp);
        }
        // (b)
        uint8_t *b2 = NULL;
        size_t l2 = 0;
        CHECK(beta_resave(d, len, &b2, &l2) == 0);
        if (b2) {
            CHECK(rd16(b2 + 4) == (known && (uint32_t)p->paper >= PAPER_COUNT ? 2 : 1));
            if (!known) CHECK(rd16(b2 + 6) == BETA_PAPER_BLANK);
            size_t rec[16], end = 0, rec0[16], end0 = 0;
            uint32_t n2 = walk(b2, l2, rec, 16, &end), n0 = walk(d, len, rec0, 16, &end0);
            CHECK(n2 == (uint32_t)p->n && n0 == n2 && end == l2);  // nothing after the strokes
            CHECK(end0 + p->tail_len == len);
            for (uint32_t i = 0; i < n2 && i < 16; i++) {
                CHECK(b2[rec[i] + 1] == 0 && b2[rec[i] + 2] == 0 && b2[rec[i] + 3] == 0);
                CHECK(rd32(b2 + rec[i] + 4) == p->strokes[i]->seed);
                // The rest of each record (params, points) is what we wrote.
                size_t rl = (i + 1 < n2 ? rec[i + 1] : end) - rec[i];
                CHECK(!memcmp(b2 + rec[i] + 4, d + rec0[i] + 4, rl - 4));
            }
            // Without new data, 1.0.0-beta writes exactly what we wrote.
            if (!has_new_data(k)) CHECK(l2 == len && !memcmp(b2, d, len));
            free(b2);
        }
        // (c)
        Page *q = NULL;
        CHECK(page_deserialize(d, len, &q) == 0 && q);
        if (q) {
            CHECK(q->readonly == (k == K_CRITICAL_CHUNK));
            CHECK(q->n == p->n && q->paper == p->paper && q->tail_len == p->tail_len);
            for (int i = 0; i < q->n && i < p->n; i++) {
                CHECK(!memcmp(q->strokes[i]->ext, p->strokes[i]->ext, 3));
                CHECK(q->strokes[i]->seed == p->strokes[i]->seed);
            }
            uint8_t *d2 = NULL;
            size_t len2 = 0;
            if (!q->readonly) {
                CHECK(page_serialize(q, &d2, &len2) == 0);
                CHECK(len2 == len && d2 && !memcmp(d2, d, len));
                if (len2 != len || (d2 && memcmp(d2, d, len))) fprintf(stderr, "  %s: not the same bytes\n", k_kind_name[k]);
            } else {
                CHECK(page_serialize(q, &d2, &len2) != 0);  // a read-only page is never written
            }
            free(d2);
            page_free(q);
        }
        free(d);
        page_free(p);
    }
}

// (h) Pages with no new data come out byte for byte as 1.0.0-beta's own writer makes them.
TEST(plain_pages_match_beta_writer) {
    for (int k = 0; k < K_COUNT; k++) {
        if (has_new_data(k)) continue;
        Page *p = make_page(k);
        // The same page built for the frozen writer, field by field.
        BetaPage bp = {(uint32_t)p->paper, (uint32_t)p->w, (uint32_t)p->h, (uint32_t)p->n, NULL};
        bp.strokes = calloc((size_t)(p->n ? p->n : 1), sizeof *bp.strokes);
        for (int i = 0; i < p->n; i++) {
            const Stroke *s = p->strokes[i];
            BetaStroke *b = &bp.strokes[i];
            b->tool = (uint8_t)s->tool;
            b->seed = s->seed;
            memcpy(b->params, &s->params, sizeof b->params);
            b->n = (uint32_t)s->n;
            b->pts = calloc((size_t)s->n, sizeof *b->pts);
            for (int j = 0; j < s->n; j++)
                b->pts[j] = (BetaPoint){s->pts[j].x, s->pts[j].y, s->pts[j].pressure, s->pts[j].tilt_x, s->pts[j].tilt_y, s->pts[j].t_ms};
        }
        uint8_t *mine = NULL, *theirs = NULL;
        size_t lm = 0, lt = 0;
        CHECK(page_serialize(p, &mine, &lm) == 0);
        CHECK(beta_page_serialize(&bp, &theirs, &lt) == 0);
        CHECK(lm == lt && mine && theirs && !memcmp(mine, theirs, lm));
        free(mine);
        free(theirs);
        beta_page_free(&bp);
        page_free(p);
    }
}

// Bytes for the read-only cases, made from a plain page.
enum { RO_VERSION3, RO_VERSION3_ODD_SIZE, RO_TOOL10, RO_CRITICAL, RO_CUT_TAIL, RO_NOT_A_CHUNK, RO_LONG_CHUNK, RO_DIGIT_TAG, RO_COUNT };
static const char *const k_ro_name[RO_COUNT] = {"version 3", "version 3, odd size", "tool 10", "critical chunk",
                                                "cut-off tail", "not a chunk", "chunk past the end", "tag starting with a digit"};

static uint8_t *readonly_bytes(int which, size_t *len) {
    Page *p = make_page(which == RO_CRITICAL ? K_CRITICAL_CHUNK : K_PLAIN);
    uint8_t *d = NULL;
    page_serialize(p, &d, len);
    page_free(p);
    d = realloc(d, *len + 64);
    size_t rec[4], end;
    walk(d, *len, rec, 4, &end);
    switch (which) {
    case RO_VERSION3: d[4] = 3; break;
    case RO_VERSION3_ODD_SIZE:
        d[4] = 3;
        memset(d + 8, 0, 4);  // w = 0: the device's page size is used
        break;
    case RO_TOOL10: d[rec[1]] = 10; break;
    case RO_CUT_TAIL: memcpy(d + *len, "lay", 3), *len += 3; break;
    case RO_NOT_A_CHUNK: memcpy(d + *len, "\x00\x01\x02\x03\x00\x00\x00\x00", 8), *len += 8; break;
    case RO_LONG_CHUNK: *len += chunk(d + *len, "layr", "abcd", 4), d[*len - 8] = 9; break;  // says 9, has 4
    case RO_DIGIT_TAG: *len += chunk(d + *len, "1lay", "abcd", 4); break;
    default: break;
    }
    return d;
}

static void make_notebook(const char *name, NotebookInfo *nb, int pages) {
    CHECK(store_create(name, "", PAPER_MEDIUM, nb) == 0);
    for (int i = 1; i < pages; i++) CHECK(store_add_page(nb, nb->npages, PAPER_MEDIUM) == 0);
}

static void page_file(const NotebookInfo *nb, uint32_t pid, char *path, size_t cap) {
    snprintf(path, cap, "%s/notebooks/%s/p%u.qpg", root, nb->id, pid);
}

// (d) A newer page opens read-only: never set aside, never counted as damage, never rewritten.
TEST(newer_pages_open_readonly) {
    NotebookInfo nb;
    make_notebook("Newer pages", &nb, 2);
    char dir[700], path[800];
    snprintf(dir, sizeof dir, "%s/notebooks/%s", root, nb.id);
    page_file(&nb, nb.page_ids[1], path, sizeof path);
    for (int w = 0; w < RO_COUNT; w++) {
        size_t len;
        uint8_t *d = readonly_bytes(w, &len);
        if (w == RO_VERSION3_ODD_SIZE) store_set_device(300.f, 1404, 1872);  // a screen of another size
        Page *p = NULL;
        CHECK(page_deserialize(d, len, &p) == 0 && p && p->readonly);
        if (w == RO_VERSION3_ODD_SIZE) store_set_device(300.f, 1860, 2480);
        if (!p) {
            free(d);
            continue;
        }
        if (!p->readonly) fprintf(stderr, "  %s: not read-only\n", k_ro_name[w]);
        if (w == RO_VERSION3 || w == RO_VERSION3_ODD_SIZE) {
            CHECK(p->n == 0 && p->paper == PAPER_BLANK);  // shown blank
            // Its own size from the frozen prefix, or the screen's when that makes no sense.
            CHECK(w == RO_VERSION3 ? (p->w == 1860 && p->h == 2480) : (p->w == 1404 && p->h == 1872));
        }
        if (w == RO_TOOL10) CHECK(p->n == 2 && p->strokes[0]->tool == TOOL_PENCIL && p->strokes[1]->tool == TOOL_HIGHLIGHTER);
        if (w == RO_CRITICAL || w == RO_CUT_TAIL || w == RO_NOT_A_CHUNK || w == RO_LONG_CHUNK || w == RO_DIGIT_TAG)
            CHECK(p->n == 3);  // every stroke is still shown
        uint8_t *out = NULL;
        size_t ol;
        CHECK(page_serialize(p, &out, &ol) != 0 && !out);
        page_free(p);
        // Through the store, as the editor opens it.
        CHECK(write_file_atomic(path, d, len) == 0);
        CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0 && p && p->readonly);
        CHECK(store_take_corrupt_count() == 0);
        CHECK(same_file(path, d, len) && count_files(dir, "p2.qpg.damaged-") == 0);
        if (p) {
            // An edit that slipped through would still never reach the file.
            page_add(p, wave(TOOL_PENCIL, 1700.f, 1u, NULL));
            CHECK(store_save_page(&nb, 1, p) == 0);
            CHECK(same_file(path, d, len));
            page_free(p);
        }
        // Scaled to another screen size, still read-only, still untouched on disk.
        CHECK(store_load_page(&nb, 1, 930, 1240, &p) == 0 && p && p->readonly && p->w == 930);
        page_free(p);
        CHECK(same_file(path, d, len) && store_take_corrupt_count() == 0);
        free(d);
    }
    // Damage is still damage: versions no Grimoire writes, impossible sizes, cut-off strokes.
    size_t len;
    uint8_t *d = readonly_bytes(RO_COUNT, &len);  // a plain page
    struct {
        const char *name;
        int at;
        uint8_t v;
        size_t cut;
    } bad[] = {
        {"version 0", 4, 0, 0},  {"version 256", 5, 1, 0},     {"bad magic", 0, 'X', 0},
        {"zero width", 8, 0, 0}, {"cut in the strokes", -1, 0, 100}, {"only a prefix", -1, 0, 16},
    };
    for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++) {
        uint8_t *m = malloc(len);
        memcpy(m, d, len);
        size_t ml = len;
        if (bad[i].at >= 0) m[bad[i].at] = bad[i].v;
        if (!strcmp(bad[i].name, "version 256")) m[4] = 0;
        if (!strcmp(bad[i].name, "zero width")) memset(m + 8, 0, 4);
        if (bad[i].cut) ml = bad[i].cut;
        Page *p = NULL;
        CHECK(page_deserialize(m, ml, &p) != 0 && !p);
        CHECK(write_file_atomic(path, m, ml) == 0);
        CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0 && p && p->n == 0 && !p->readonly);
        page_free(p);
        CHECK(store_take_corrupt_count() == 1);
        struct stat st;
        CHECK(stat(path, &st) != 0);  // set aside, as before
        if (stat(path, &st) == 0) fprintf(stderr, "  %s: not set aside\n", bad[i].name);
        free(m);
    }
    CHECK(count_files(dir, "p2.qpg.damaged-") >= 1);
    free(d);
}

// Unknown papers and the pad bytes survive the store's own paths: a save, a scaled load.
TEST(store_carries_new_data) {
    NotebookInfo nb;
    make_notebook("Carried", &nb, 1);
    Page *p = make_page(K_EVERYTHING);
    CHECK(store_save_page(&nb, 0, p) == 0);
    uint8_t *want = NULL;
    size_t wl = 0;
    page_serialize(p, &want, &wl);
    char path[800];
    page_file(&nb, nb.page_ids[0], path, sizeof path);
    CHECK(same_file(path, want, wl));
    Page *q = NULL;
    // Loaded at another size (points rescaled), the extras are all still there.
    CHECK(store_load_page(&nb, 0, 930, 1240, &q) == 0 && q && !q->readonly);
    if (q) {
        CHECK((uint32_t)q->paper == IMAGE_PAPER && q->n == p->n && q->tail_len == p->tail_len);
        CHECK(!memcmp(q->tail, p->tail, p->tail_len));
        for (int i = 0; i < q->n && i < p->n; i++) CHECK(!memcmp(q->strokes[i]->ext, p->strokes[i]->ext, 3));
        page_free(q);
    }
    // Loaded at its own size and saved after an edit: the new stroke joins, nothing is lost.
    CHECK(store_load_page(&nb, 0, 1860, 2480, &q) == 0 && q);
    if (q) {
        page_add(q, wave(TOOL_MARKER, 2000.f, 77u, NULL));
        CHECK(store_save_page(&nb, 0, q) == 0);
        page_free(q);
    }
    CHECK(store_load_page(&nb, 0, 1860, 2480, &q) == 0 && q);
    if (q) {
        CHECK((uint32_t)q->paper == IMAGE_PAPER && q->n == p->n + 1 && q->tail_len == p->tail_len);
        CHECK(q->strokes[0]->ext[0] == 3 && q->strokes[2]->ext[2] == 255 && q->strokes[3]->seed >> 24 == TOOL_CHARCOAL + 1);
        page_free(q);
    }
    // The notebook's default paper may be a newer one too: new pages get it, version 2.
    nb.paper = (PaperKind)IMAGE_PAPER;
    CHECK(store_save_info(&nb) == 0);
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0 && (uint32_t)back.paper == IMAGE_PAPER);
    CHECK(store_add_page(&back, back.npages, back.paper) == 0);
    page_file(&back, back.page_ids[1], path, sizeof path);
    size_t len;
    uint8_t *d = slurp(path, &len);
    CHECK(d && len >= 24 && rd16(d + 4) == 2 && rd16(d + 6) == PAPER_BLANK && rd32(d + 16) == IMAGE_PAPER);
    free(d);
    // A missing page file shows the notebook's paper.
    CHECK(unlink(path) == 0);
    CHECK(store_load_page(&back, 1, 1860, 2480, &q) == 0 && q && (uint32_t)q->paper == IMAGE_PAPER);
    page_free(q);
    // But a new notebook only ever starts on a paper this version offers.
    NotebookInfo fresh;
    CHECK(store_create("Fresh", "", (PaperKind)IMAGE_PAPER, &fresh) == 0 && fresh.paper == PAPER_MEDIUM);
    free(want);
    page_free(p);
}

// (f) Pages 1.0.0-beta set aside come back when the notebook is opened.
TEST(set_aside_pages_come_back) {
    NotebookInfo nb;
    make_notebook("Set aside", &nb, 4);
    char dir[700], path[800], aside[900];
    snprintf(dir, sizeof dir, "%s/notebooks/%s", root, nb.id);
    // Page 1: a version 3 page; 1.0.0-beta opens it and sets it aside.
    size_t l3, l10, lg;
    uint8_t *v3 = readonly_bytes(RO_VERSION3, &l3), *t10 = readonly_bytes(RO_TOOL10, &l10);
    page_file(&nb, nb.page_ids[0], path, sizeof path);
    CHECK(write_file_atomic(path, v3, l3) == 0);
    CHECK(beta_load_page_file(path, 1792000100) != 0);
    CHECK(access(path, F_OK) != 0 && count_files(dir, "p1.qpg.damaged-") == 1);
    // Page 2: set aside twice (two tool-10 versions); the newer copy is the one that comes back.
    page_file(&nb, nb.page_ids[1], path, sizeof path);
    CHECK(write_file_atomic(path, v3, l3) == 0);
    CHECK(beta_load_page_file(path, 1792000100) != 0);
    CHECK(write_file_atomic(path, t10, l10) == 0);
    CHECK(beta_load_page_file(path, 1792000200) != 0);
    // Page 3: its newest copy is real damage, an older one is a newer version's page.
    page_file(&nb, nb.page_ids[2], path, sizeof path);
    CHECK(write_file_atomic(path, t10, l10) == 0);
    CHECK(beta_load_page_file(path, 1792000100) != 0);
    CHECK(write_file_atomic(path, "QPG1 garbage", 12) == 0);
    CHECK(beta_load_page_file(path, 1792000300) != 0);
    // Page 4: set aside, then edited in 1.0.0-beta, which wrote a fresh page: both stay.
    page_file(&nb, nb.page_ids[3], path, sizeof path);
    CHECK(write_file_atomic(path, v3, l3) == 0);
    CHECK(beta_load_page_file(path, 1792000100) != 0);
    Page *fresh = make_page(K_PLAIN);
    uint8_t *fd = NULL;
    size_t fl = 0;
    page_serialize(fresh, &fd, &fl);
    CHECK(write_file_atomic(path, fd, fl) == 0);
    // A page that is no longer in the notebook's list stays aside, and so does real damage.
    char aside9[900];
    snprintf(aside9, sizeof aside9, "%s/p9.qpg.damaged-1792000100", dir);
    CHECK(write_file_atomic(aside9, v3, l3) == 0);
    snprintf(aside, sizeof aside, "%s/p8.qpg.damaged-1792000100", dir);
    uint8_t *garbage = (uint8_t *)strdup("QPG1 not a page at all");
    lg = strlen((char *)garbage);
    CHECK(write_file_atomic(aside, garbage, lg) == 0);

    CHECK(store_recover_pages(&nb) == 3);  // pages 1, 2 and 3
    page_file(&nb, nb.page_ids[0], path, sizeof path);
    CHECK(same_file(path, v3, l3) && count_files(dir, "p1.qpg.damaged-") == 0);
    page_file(&nb, nb.page_ids[1], path, sizeof path);
    CHECK(same_file(path, t10, l10) && count_files(dir, "p2.qpg.damaged-") == 1);  // the older copy stays
    page_file(&nb, nb.page_ids[2], path, sizeof path);
    CHECK(same_file(path, t10, l10) && count_files(dir, "p3.qpg.damaged-") == 1);  // the garbage stays
    page_file(&nb, nb.page_ids[3], path, sizeof path);
    CHECK(same_file(path, fd, fl) && count_files(dir, "p4.qpg.damaged-") == 1);   // both kept
    snprintf(path, sizeof path, "%s/p9.qpg", dir);
    CHECK(access(path, F_OK) != 0 && same_file(aside9, v3, l3));
    CHECK(count_files(dir, "p8.qpg.damaged-") == 1 && access(aside, F_OK) == 0);
    // They open read-only, as pages from a newer version do; nothing is counted as damage.
    Page *p = NULL;
    CHECK(store_load_page(&nb, 0, 1860, 2480, &p) == 0 && p && p->readonly);
    page_free(p);
    CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0 && p && p->readonly && p->n == 2);
    page_free(p);
    CHECK(store_take_corrupt_count() == 0);
    // Opening it again changes nothing.
    CHECK(store_recover_pages(&nb) == 0);
    free(v3);
    free(t10);
    free(fd);
    free(garbage);
    page_free(fresh);
}

// Damaged and hostile bytes never crash the reader, and whatever it accepts as an editable page
// settles after one save: writing what it read back gives the same bytes every time after.
TEST(fuzz_new_paths) {
    Page *p = make_page(K_EVERYTHING);
    uint8_t *d = NULL;
    size_t len = 0;
    CHECK(page_serialize(p, &d, &len) == 0);
    page_free(p);
    if (!d) return;
    uint8_t *m = malloc(len + 64);
    srand(7);
    int editable = 0, readonly = 0, failed0 = tl_failures;
    // The reader logs every read-only page it meets, over a thousand here: that log goes to a
    // file (kept if anything fails, an ASan report included).
    char logp[700];
    snprintf(logp, sizeof logp, "%s/fuzz-%d.log", tl_tmp(), (int)getpid());
    fprintf(stderr, "  (the reader's log for this test: %s)\n", logp);
    fflush(stderr);
    int saved = dup(2), fd = open(logp, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) dup2(fd, 2);
    for (int it = 0; it < 4000; it++) {
        memcpy(m, d, len);
        size_t ml = len;
        int mode = it % 4;
        if (mode == 0) ml = (size_t)rand() % (len + 1);                       // cut anywhere
        else if (mode == 1) m[len - 1 - (size_t)rand() % 48] = (uint8_t)rand();  // in the chunks
        else if (mode == 2) m[(size_t)rand() % 24] = (uint8_t)rand();         // in the header
        else {
            for (int k = 0; k < 4; k++) m[(size_t)rand() % len] = (uint8_t)rand();
            for (int k = 0; k < rand() % 40; k++) m[ml++] = (uint8_t)rand();  // and a tail of noise
        }
        Page *q = NULL;
        if (page_deserialize(m, ml, &q) != 0) continue;
        if (q->readonly) {
            readonly++;
            uint8_t *o = NULL;
            size_t ol;
            CHECK(page_serialize(q, &o, &ol) != 0);
            free(o);
            page_free(q);
            continue;
        }
        editable++;
        uint8_t *b1 = NULL, *b2 = NULL;
        size_t l1 = 0, l2 = 0;
        CHECK(page_serialize(q, &b1, &l1) == 0);
        page_free(q);
        q = NULL;
        CHECK(b1 && page_deserialize(b1, l1, &q) == 0 && q && !q->readonly);
        if (q) CHECK(page_serialize(q, &b2, &l2) == 0 && l1 == l2 && !memcmp(b1, b2, l1));
        page_free(q);
        free(b1);
        free(b2);
    }
    fflush(stderr);
    if (saved >= 0) {
        dup2(saved, 2);
        close(saved);
    }
    if (fd >= 0) close(fd);
    CHECK(editable > 100 && readonly > 100);
    if (tl_failures != failed0) fprintf(stderr, "  fuzz: %d checks failed, see %s\n", tl_failures - failed0, logp);
    else unlink(logp);
    free(m);
    free(d);
}

static char *file_text(const char *path) {
    size_t len;
    return (char *)slurp(path, &len);
}

// The number of lines in `text` exactly equal to `line`.
static int has_line(const char *text, const char *line) {
    int n = 0;
    size_t l = strlen(line);
    for (const char *p = text; p && *p;) {
        const char *e = strchr(p, '\n');
        size_t ll = e ? (size_t)(e - p) : strlen(p);
        if (ll == l && !memcmp(p, line, l)) n++;
        p = e ? e + 1 : NULL;
    }
    return n;
}

// (g) settings.ini: lines this version does not know survive a save, and so do values it cannot
// use as written, unless the setting is changed.
TEST(settings_carry_unknown_lines) {
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    char *longv = malloc(20001);
    memset(longv, 'x', 20000);
    longv[20000] = 0;
    longv[10] = '=';
    size_t cap = 40000;
    char *f = malloc(cap);
    snprintf(f, cap,
             "# Grimoire settings\r\n"
             "current_tool=2\r\n"
             "language=es\r\n"                       // a newer version's key (CRLF file)
             "side_button=4\n"                       // a value this version does not have
             "theme=dark\n"
             "sleep_minutes=500\n"
             "pencil.size_max_mm=20\n"               // past this version's range
             "ballpoint.pressure_gamma=2.5\n"
             "pencil.color=3\n"                      // a tool setting this version lacks
             "white.size_min_mm=0.5\n"               // a tool this version lacks
             "pen.rotation=1\n"                      // a calibration field it lacks
             "averyveryverylongheadthatisover32chars.k=v\n"
             "url=http://host/?a=b&c=d==\n"          // '=' inside the value
             "empty=\n"
             "music_layout=%u\n"                     // custom music with a part it lacks
             "settings_version=7\n"
             "http_port=80\n"                        // below the range: saved as used (the launcher reads it)
             "long=%s\n",
             MUSIC_NEWER, longv);
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    Settings s;
    CHECK(settings_load(&s) == 0);
    CHECK(s.current_tool == TOOL_BALLPOINT && s.side_button == 3 && s.theme == 0 && s.sleep_minutes == 240);
    CHECK_NEAR(s.tools[TOOL_PENCIL].size_max_mm, 12.f, 1e-4);
    CHECK_NEAR(s.tools[TOOL_BALLPOINT].pressure_gamma, 2.5f, 1e-4);
    CHECK(s.music_layout == music_default() && s.version == 7 && s.http_port == 1024);
    CHECK(settings_save(&s) == 0);
    char *t = file_text(path);
    CHECK(t != NULL);
    if (t) {
        CHECK(has_line(t, "language=es") == 1 && has_line(t, "pencil.color=3") == 1);
        CHECK(has_line(t, "white.size_min_mm=0.5") == 1 && has_line(t, "pen.rotation=1") == 1);
        CHECK(has_line(t, "averyveryverylongheadthatisover32chars.k=v") == 1);
        CHECK(has_line(t, "url=http://host/?a=b&c=d==") == 1 && has_line(t, "empty=") == 1);
        char *ll = malloc(20010);
        snprintf(ll, 20010, "long=%s", longv);
        CHECK(has_line(t, ll) == 1);
        free(ll);
        // Known keys this version could not use as written go back as they were.
        CHECK(has_line(t, "side_button=4") == 1 && has_line(t, "theme=dark") == 1);
        CHECK(has_line(t, "sleep_minutes=500") == 1 && has_line(t, "pencil.size_max_mm=20") == 1);
        char ml[64];
        snprintf(ml, sizeof ml, "music_layout=%u", MUSIC_NEWER);
        CHECK(has_line(t, ml) == 1);
        CHECK(has_line(t, "settings_version=7") == 1 && has_line(t, "current_tool=2") == 1);
        CHECK(has_line(t, "http_port=1024") == 1 && !strstr(t, "http_port=80\n"));
        CHECK(has_line(t, "ballpoint.pressure_gamma=2.5000") == 1);
        CHECK(!strchr(t, '\r'));
        // Known keys come first, the unknown lines after them.
        CHECK(strstr(t, "language=es") > strstr(t, "calligraphy.smoothing="));
    }
    // A second round trip writes the very same file.
    Settings s2;
    CHECK(settings_load(&s2) == 0 && settings_save(&s2) == 0);
    char *t2 = file_text(path);
    CHECK(t && t2 && !strcmp(t, t2));
    free(t2);
    free(t);
    // A setting changed here is written as changed; the others keep their file values.
    s2.side_button = 0;
    s2.theme = 1;
    s2.tools[TOOL_PENCIL].size_max_mm = 0.5f;
    CHECK(settings_save(&s2) == 0);
    t = file_text(path);
    CHECK(t && has_line(t, "side_button=0") == 1 && has_line(t, "theme=classic") == 1);
    CHECK(t && has_line(t, "pencil.size_max_mm=0.5000") == 1 && has_line(t, "sleep_minutes=500") == 1);
    CHECK(t && has_line(t, "language=es") == 1);
    free(t);
    // The last line for a key counts: a later value this version has forgets the kept one.
    const char *dup = "side_button=4\nside_button=2\ntheme=dark\ntheme=gothic\n";
    CHECK(write_file_atomic(path, dup, strlen(dup)) == 0);
    CHECK(settings_load(&s) == 0 && s.side_button == 2 && settings_save(&s) == 0);
    t = file_text(path);
    CHECK(t && has_line(t, "side_button=2") == 1 && has_line(t, "theme=gothic") == 1 && !strstr(t, "dark"));
    free(t);
    // Far more unknown text than any real file: the known keys are all written (no fixed buffer),
    // and unknown lines are kept up to a bound.
    size_t big = 200000, n = 0;
    char *g = malloc(big + 100);
    while (n + 40 < big) n += (size_t)snprintf(g + n, big + 100 - n, "newer.key%06zu=%s\n", n, "some value or other");
    n += (size_t)snprintf(g + n, big + 100 - n, "side_button=1\n");
    CHECK(write_file_atomic(path, g, n) == 0);
    CHECK(settings_load(&s) == 0 && s.side_button == 1);
    CHECK(settings_save(&s) == 0);
    t = file_text(path);
    CHECK(t && has_line(t, "side_button=1") == 1 && has_line(t, "newer.key000000=some value or other") == 1);
    CHECK(t && strstr(t, "calligraphy.smoothing=") != NULL && strlen(t) > 65536 && strlen(t) < 65536 + 8192);
    free(t);
    free(g);
    // A file with no settings at all, then defaults: nothing kept from before.
    unlink(path);
    CHECK(settings_load(&s) != 0 && settings_save(&s) == 0);
    t = file_text(path);
    CHECK(t && !strstr(t, "newer.key") && has_line(t, "side_button=2") == 1);
    free(t);
    free(f);
    free(longv);
}

// (g) notebook.ini: lines this version does not know survive every save of the notebook.
TEST(notebook_ini_carries_unknown_lines) {
    NotebookInfo nb;
    make_notebook("Keys", &nb, 2);
    char path[800];
    snprintf(path, sizeof path, "%s/notebooks/%s/notebook.ini", root, nb.id);
    char *t = file_text(path);
    CHECK(t != NULL);
    char *longv = malloc(1001);
    memset(longv, 'v', 1000);
    longv[1000] = 0;
    char *huge = malloc(3001);
    memset(huge, 'h', 3000);
    huge[3000] = 0;
    size_t cap = strlen(t) + 8000;
    char *f = malloc(cap);
    snprintf(f, cap, "%spage_w=1986\r\npage_h=2648\nlang=es\ntmpl=a=b=c\nlong=%s\nhuge=%s\nlast=1\npaper=%u\n", t, longv,
             huge, IMAGE_PAPER);
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0 && back.npages == 2 && (uint32_t)back.paper == IMAGE_PAPER);
    CHECK(store_save_info(&back) == 0);
    // Each of these writes the file from what it just read: every one must carry the lines on.
    CHECK(store_rename(nb.id, "Keys, renamed") == 0);  // load + save
    free(t);
    t = file_text(path);
    CHECK(t && has_line(t, "lang=es") == 1 && has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "name=Keys, renamed") == 1);
    CHECK(store_set_folder(nb.id, "Somewhere") == 0);  // load + save
    free(t);
    t = file_text(path);
    CHECK(t && has_line(t, "lang=es") == 1 && has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "folder=Somewhere") == 1);
    CHECK(t && has_line(t, "name=Keys, renamed") == 1);
    // The in-memory copy the editor would hold now: read again after those, a page added, saved.
    CHECK(store_load_info(nb.id, &back) == 0);
    CHECK(store_add_page(&back, back.npages, PAPER_GRID) == 0);
    free(t);
    t = file_text(path);
    CHECK(t != NULL);
    if (t) {
        CHECK(has_line(t, "page_w=1986") == 1 && has_line(t, "page_h=2648") == 1 && has_line(t, "lang=es") == 1);
        CHECK(has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "last=1") == 1);
        char *ll = malloc(1010);
        snprintf(ll, 1010, "long=%s", longv);
        CHECK(has_line(t, ll) == 1);
        free(ll);
        CHECK(!strstr(t, "huge="));  // too long to keep: left out, the rest is intact
        char pl[32];
        snprintf(pl, sizeof pl, "paper=%u", IMAGE_PAPER);
        CHECK(has_line(t, pl) == 1);
        CHECK(!strchr(t, '\r'));
    }
    NotebookInfo again;
    CHECK(store_load_info(nb.id, &again) == 0 && again.npages == 3 && !strcmp(again.name, "Keys, renamed") &&
          !strcmp(again.folder, "Somewhere"));
    // And deleting a page saves them on as well.
    CHECK(store_delete_page(&again, 2) == 0 && again.npages == 2);
    free(t);
    t = file_text(path);
    CHECK(t && has_line(t, "lang=es") == 1 && has_line(t, "tmpl=a=b=c") == 1 && has_line(t, "last=1") == 1);
    free(t);
    free(f);
    free(longv);
    free(huge);
}

// Names longer than the 95 bytes notebook.ini keeps (1.0.0-beta's limit too, docs/FORMAT.md) are
// cut at the start of a character, never inside one: the cut name is written back, and becomes
// the mirror PDF's file name and the share pages' text.
static bool valid_utf8(const char *s) {
    const uint8_t *u = (const uint8_t *)s;
    while (*u) {
        int n = *u < 0x80 ? 0 : (*u & 0xE0) == 0xC0 ? 1 : (*u & 0xF0) == 0xE0 ? 2 : (*u & 0xF8) == 0xF0 ? 3 : -1;
        if (n < 0) return false;
        u++;
        for (int i = 0; i < n; i++, u++)
            if ((*u & 0xC0) != 0x80) return false;
    }
    return true;
}

TEST(long_names_cut_at_a_character) {
    NotebookInfo nb, back;
    make_notebook("Short", &nb, 1);
    // 93 ASCII bytes, then n with a tilde (2 bytes) across the 95-byte limit, then more.
    char name[200];
    memset(name, 'a', 93);
    snprintf(name + 93, sizeof name - 93, "\xc3\xb1\xc3\xb1 tail");
    CHECK(store_rename(nb.id, name) == 0);
    CHECK(store_load_info(nb.id, &back) == 0);
    CHECK(strlen(back.name) == 93 + 2 && valid_utf8(back.name) && !memcmp(back.name + 93, "\xc3\xb1", 2));
    // The limit inside the second byte of a 2-byte letter: that letter goes.
    memset(name, 'b', 94);
    snprintf(name + 94, sizeof name - 94, "\xc3\xb1 tail");
    CHECK(store_rename(nb.id, name) == 0);
    CHECK(store_load_info(nb.id, &back) == 0);
    CHECK(strlen(back.name) == 94 && valid_utf8(back.name) && back.name[93] == 'b');
    // A 4-byte character (an emoji) across the limit, in a folder name.
    memset(name, 'c', 92);
    snprintf(name + 92, sizeof name - 92, "\xf0\x9f\x8e\xb5 tail");
    CHECK(store_set_folder(nb.id, name) == 0);
    CHECK(store_load_info(nb.id, &back) == 0);
    CHECK(strlen(back.folder) == 92 && valid_utf8(back.folder));
    // folders.ini's lines the same.
    CHECK(store_add_folder(name) == 0);
    char (*fs)[STORE_NAME_LEN] = NULL;
    int n = 0;
    CHECK(store_list_folders(&fs, &n) == 0);
    bool all_valid = n > 0;
    for (int i = 0; i < n; i++) all_valid &= valid_utf8(fs[i]) && strlen(fs[i]) <= 95;
    CHECK(all_valid);
    free(fs);
    // A name that fits is kept byte for byte, accents and all.
    const char *fits = "M\xc3\xbasica espa\xc3\xb1ola \xe2\x82\xac";
    CHECK(store_rename(nb.id, fits) == 0 && store_load_info(nb.id, &back) == 0 && !strcmp(back.name, fits));
    CHECK(store_set_folder(nb.id, "") == 0);
}

// A value kept from settings.ini goes back only while the setting is untouched: once a save finds
// it changed, it is saved as set from then on, also when it is set back to the value used.
TEST(changed_settings_stay_changed) {
    char path[700], f[256];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    snprintf(f, sizeof f, "side_button=4\ntheme=dark\npencil.size_max_mm=20\nmusic_layout=%u\nsleep_minutes=500\n",
             MUSIC_NEWER);
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    Settings s;
    CHECK(settings_load(&s) == 0 && s.side_button == 3 && s.theme == 0 && s.music_layout == music_default());
    // Changed (every settings change is saved as it is made), then set back to what was used.
    MusicLayout m = custom_layout();
    s.side_button = 0;
    s.theme = 1;
    s.tools[TOOL_PENCIL].size_max_mm = 5.f;
    s.music_layout = music_encode(&m);
    CHECK(settings_save(&s) == 0);
    s.side_button = 3;
    s.theme = 0;
    s.tools[TOOL_PENCIL].size_max_mm = 12.f;
    s.music_layout = music_default();
    CHECK(settings_save(&s) == 0);
    char ml[64];
    snprintf(ml, sizeof ml, "music_layout=%u", (unsigned)music_default());
    for (int round = 0; round < 2; round++) {  // and the same again on the next save
        char *t = file_text(path);
        CHECK(t && has_line(t, "side_button=3") == 1 && has_line(t, "theme=gothic") == 1);
        CHECK(t && has_line(t, "pencil.size_max_mm=12.0000") == 1 && has_line(t, ml) == 1);
        CHECK(t && has_line(t, "sleep_minutes=500") == 1);  // the setting nobody changed
        free(t);
        CHECK(settings_save(&s) == 0);
    }
}

// A 0/1 setting holding another value (a newer version's third state) is used as on or off and
// written back as it was until the setting changes; calibration's valid and swap the same.
TEST(bool_settings_carry_other_values) {
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    const char *f = "invert=2\nmirror_pdf=2\nfollow_rotation=2\npinch_zoom=auto\npanel_collapsed=2\n"
                    "passcode_told=2\npen.valid=2\npen.swap=1\ntouch.valid=0\ntouch.swap=2\n";
    CHECK(write_file_atomic(path, f, strlen(f)) == 0);
    Settings s;
    CHECK(settings_load(&s) == 0);
    CHECK(s.invert && s.mirror_pdf && s.follow_rotation && !s.pinch_zoom && s.panel_collapsed && s.passcode_told);
    CHECK(s.pen_calib.valid && s.pen_calib.swap_xy && !s.touch_calib.valid && s.touch_calib.swap_xy);
    for (int round = 0; round < 2; round++) {  // kept on every save while unchanged
        CHECK(settings_save(&s) == 0);
        char *t = file_text(path);
        CHECK(t && has_line(t, "invert=2") == 1 && has_line(t, "mirror_pdf=2") == 1);
        CHECK(t && has_line(t, "follow_rotation=2") == 1 && has_line(t, "pinch_zoom=auto") == 1);
        CHECK(t && has_line(t, "panel_collapsed=2") == 1 && has_line(t, "passcode_told=2") == 1);
        CHECK(t && has_line(t, "pen.valid=2") == 1 && has_line(t, "pen.swap=1") == 1);
        CHECK(t && has_line(t, "touch.valid=0") == 1 && has_line(t, "touch.swap=2") == 1);
        free(t);
    }
    // Changed here: saved as set from then on, also when set back.
    s.invert = false;
    s.pen_calib.valid = false;
    CHECK(settings_save(&s) == 0);
    s.invert = true;
    s.pen_calib.valid = true;
    CHECK(settings_save(&s) == 0);
    char *t = file_text(path);
    CHECK(t && has_line(t, "invert=1") == 1 && has_line(t, "pen.valid=1") == 1 && has_line(t, "mirror_pdf=2") == 1);
    free(t);
    unlink(path);
}

static void put_page_list_aside(const char *dir) {
    char ini[800];
    snprintf(ini, sizeof ini, "%s/notebook.ini", dir);
    const char *bare = "# Grimoire notebook\nname=Rebuilt\npaper=1\n";  // no page list, no next id
    CHECK(write_file_atomic(ini, bare, strlen(bare)) == 0);
}

// A set-aside copy of a page the notebook no longer lists stays aside, where no new page can take
// its number and write over it; and a new page never writes over a page file that is there.
TEST(recovery_never_loses_a_page) {
    NotebookInfo nb;
    make_notebook("Rebuilt", &nb, 3);
    char dir[700], path[800], aside[900];
    snprintf(dir, sizeof dir, "%s/notebooks/%s", root, nb.id);
    size_t l3;
    uint8_t *v3 = readonly_bytes(RO_VERSION3, &l3);
    // Page 4 was set aside by 1.0.0-beta, then notebook.ini lost its page list.
    snprintf(aside, sizeof aside, "%s/p4.qpg.damaged-1792000000", dir);
    CHECK(write_file_atomic(aside, v3, l3) == 0);
    put_page_list_aside(dir);
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0 && back.npages == 3 && back.next_page_id == 4);  // rebuilt
    CHECK(store_recover_pages(&back) == 0);
    snprintf(path, sizeof path, "%s/p4.qpg", dir);
    CHECK(access(path, F_OK) != 0 && same_file(aside, v3, l3));
    // The next new page takes number 4; the copy is still there as it was, and then kept beside it.
    CHECK(store_add_page(&back, back.npages, PAPER_GRID) == 0 && back.page_ids[3] == 4);
    CHECK(same_file(aside, v3, l3));
    CHECK(store_recover_pages(&back) == 0 && same_file(aside, v3, l3) && access(path, F_OK) == 0);
    // A page file at the next number that the page list does not name is never written over.
    uint32_t taken = back.next_page_id;
    snprintf(path, sizeof path, "%s/p%u.qpg", dir, taken);
    CHECK(write_file_atomic(path, v3, l3) == 0);
    CHECK(store_add_page(&back, 0, PAPER_GRID) == 0);
    CHECK(back.page_ids[0] == taken + 1 && back.next_page_id == taken + 2 && same_file(path, v3, l3));
    NotebookInfo again;
    CHECK(store_load_info(nb.id, &again) == 0 && again.npages == 5 && again.page_ids[0] == taken + 1);
    free(v3);
}

// The log, for a while: stderr goes to a file between log_begin and log_end (which returns it).
static int g_log_saved = -1;
static char g_log_path[700];

static void log_begin(void) {
    snprintf(g_log_path, sizeof g_log_path, "%s/log-%d.txt", tl_tmp(), (int)getpid());
    fflush(stderr);
    g_log_saved = dup(2);
    int fd = open(g_log_path, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) {
        dup2(fd, 2);
        close(fd);
    }
}

static char *log_end(void) {
    fflush(stderr);
    if (g_log_saved >= 0) {
        dup2(g_log_saved, 2);
        close(g_log_saved);
        g_log_saved = -1;
    }
    char *t = file_text(g_log_path);
    unlink(g_log_path);
    return t;
}

static int count_text(const char *text, const char *what) {
    int n = 0;
    for (const char *p = text; p && (p = strstr(p, what)); p += strlen(what)) n++;
    return n;
}

// What stays as it is (a set-aside copy beside its page or of a page no longer listed, notebook.ini
// lines that do not fit) is logged once, not on every opening or listing; checking the copies
// logs no read-only reasons of its own.
TEST(lasting_states_logged_once) {
    NotebookInfo nb;
    make_notebook("Logged once", &nb, 2);
    char dir[700], aside[900], ini[800];
    snprintf(dir, sizeof dir, "%s/notebooks/%s", root, nb.id);
    size_t l10;
    uint8_t *t10 = readonly_bytes(RO_TOOL10, &l10);
    snprintf(aside, sizeof aside, "%s/p2.qpg.damaged-1792000000", dir);
    CHECK(write_file_atomic(aside, t10, l10) == 0);  // p2.qpg is there too: both kept
    snprintf(aside, sizeof aside, "%s/p7.qpg.damaged-1792000000", dir);
    CHECK(write_file_atomic(aside, t10, l10) == 0);  // not in the page list
    snprintf(ini, sizeof ini, "%s/notebook.ini", dir);
    char *txt = file_text(ini);
    size_t cap = (txt ? strlen(txt) : 0) + 8000, n = 0;
    char *f = malloc(cap);
    n += (size_t)snprintf(f + n, cap - n, "%s", txt ? txt : "");
    for (int i = 0; i < 40; i++) n += (size_t)snprintf(f + n, cap - n, "newer%02d=%0100d\n", i, i);  // 4 KB
    CHECK(write_file_atomic(ini, f, n) == 0);
    int ok = 0;
    log_begin();
    for (int i = 0; i < 3; i++) {
        NotebookInfo back;
        ok += store_load_info(nb.id, &back) == 0 && back.npages == 2;
        ok += store_recover_pages(&back) == 0;
    }
    char *log = log_end();
    CHECK(ok == 6 && log != NULL);
    CHECK(count_text(log, "both kept") == 1 && count_text(log, "not in the page list") == 1);
    CHECK(count_text(log, "do not fit") == 1 && count_text(log, "read-only page") == 0);
    if (log && (count_text(log, "both kept") != 1 || count_text(log, "do not fit") != 1)) fprintf(stderr, "%s", log);
    free(log);
    free(f);
    free(txt);
    free(t10);
}

// A stroke parameter past its range, as a later version that widened one would write it: this
// version and 1.0.0-beta both read it at the range's end, and both save the same narrowed bytes.
TEST(params_past_their_ranges) {
    ToolParams prm;
    tool_params_default(TOOL_PENCIL, &prm);
    prm.size_min_mm = 0.01f;
    prm.size_max_mm = 50.f;
    prm.pressure_gamma = 0.f;
    prm.tilt_max_deg = -3.f;
    prm.smoothing = 7.f;
    Stroke *s = stroke_new(TOOL_PENCIL, &prm, 9u);
    for (int i = 0; i < 12; i++) {
        PenSample q = {300.f + (float)i * 40.f, 500.f, 0.5f, 0.f, 0.f, (uint32_t)i * 7u};
        stroke_append(s, &q, DPI);
    }
    Page *p = page_new(1860, 2480, PAPER_MEDIUM);
    page_add(p, s);
    uint8_t *d = NULL, *mine = NULL, *theirs = NULL;
    size_t len = 0, lm = 0, lt = 0;
    CHECK(page_serialize(p, &d, &len) == 0);
    Page *q = NULL;
    CHECK(d && page_deserialize(d, len, &q) == 0 && q && !q->readonly && q->n == 1);
    if (q && q->n == 1) {
        const ToolParams *r = &q->strokes[0]->params;
        CHECK(r->size_min_mm == 0.10f && r->size_max_mm == 12.f && r->pressure_gamma == 0.25f);
        CHECK(r->tilt_max_deg == 5.f && r->smoothing == 1.f && r->opacity_max == prm.opacity_max);
        CHECK(page_serialize(q, &mine, &lm) == 0);
    }
    CHECK(d && beta_resave(d, len, &theirs, &lt) == 0);
    CHECK(mine && theirs && lm == lt && !memcmp(mine, theirs, lm));
    CHECK(mine && lm == len && memcmp(mine, d, len) != 0);  // narrowed for good
    free(d);
    free(mine);
    free(theirs);
    page_free(q);
    page_free(p);
}

int main(void) {
    snprintf(root, sizeof root, "%s/format-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    RUN(papers_newer_than_this_version);
    RUN(beta_opens_what_we_write);
    RUN(plain_pages_match_beta_writer);
    RUN(newer_pages_open_readonly);
    RUN(store_carries_new_data);
    RUN(set_aside_pages_come_back);
    RUN(fuzz_new_paths);
    RUN(settings_carry_unknown_lines);
    RUN(notebook_ini_carries_unknown_lines);
    RUN(changed_settings_stay_changed);
    RUN(bool_settings_carry_other_values);
    RUN(long_names_cut_at_a_character);
    RUN(recovery_never_loses_a_page);
    RUN(lasting_states_logged_once);
    RUN(params_past_their_ranges);
    int rc = tl_done();
    system(cmd);
    return rc;
}
