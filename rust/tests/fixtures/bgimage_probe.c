// bgimage_probe.c: runs ../../../src/bgimage.c (the C build, as the Makefile compiles it) over the
// fixtures in bgimage/ and prints what it does, one section after the other ("## name" lines), for
// comparing the Rust port with it: rust/tests/test_bgimage.rs runs the same calls on the same data and
// compares line by line with bgimage_probe_c.txt, which is what this printed. The pictures are
// generated with integer arithmetic only (a linear congruential generator), so the Rust twin builds
// the very same ones; the fixtures are bgimage/mkfix.py's (PNGs of every depth and colour type, PNM,
// JPEGs from cjpeg, Exif blocks of both byte orders).
//
// To record the fixture again, from the top level of the tree:
//   clang -O1 -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -DGRIMOIRE_VERSION='"x"' -w \
//     rust/tests/fixtures/bgimage_probe.c src/bgimage.c src/canvas.c src/util.c src/stb_impl.c -lm -o /tmp/bgimage_probe
//   mkdir /tmp/bgimage_probe_root && /tmp/bgimage_probe rust/tests/fixtures/bgimage /tmp/bgimage_probe_root \
//     > rust/tests/fixtures/bgimage_probe_c.txt
// (the stderr it writes, the log lines of drawing a picture that is missing, is not compared.)
//
// The few things bgimage.c needs from the rest of the app are stubbed here: a notebook's folder is
// <root>/<id>, an image paper is PAPER_IMAGE_FLAG | role << 24 | id, and no language is loaded.
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "bgimage.h"
#include "store.h"

#define P(...) printf(__VA_ARGS__)
static const char *FIX, *ROOT;

bool paper_is_image(PaperKind k) {
    uint32_t v = (uint32_t)k;
    return (v & 0xF0000000u) == 0x50000000u && (v >> 24 & 15u) <= 1u;
}
int store_notebook_dir(const char *id, char *buf, size_t cap) {
    if (!*id) return -1;
    snprintf(buf, cap, "%s/%s", ROOT, id);
    return 0;
}
const char *i18n_getc(const char *c, const char *s) {
    (void)c;
    return s;
}

static uint32_t g_s;
static uint32_t lcg(void) { g_s = g_s * 1664525u + 1013904223u; return g_s >> 8; }
static uint64_t fnv(const uint8_t *p, size_t n) {
    uint64_t h = 1469598103934665603ull;
    for (size_t i = 0; i < n; i++) { h ^= p[i]; h *= 1099511628211ull; }
    return h;
}
static BgPicture mkpic(int w, int h, int ch, uint32_t seed) {
    g_s = seed;
    BgPicture p = {malloc((size_t)w * h * ch), w, h, ch};
    for (size_t i = 0; i < (size_t)w * h * ch; i++) p.px[i] = (uint8_t)lcg();
    return p;
}
static void dump_pic(const char *tag, const BgPicture *p) {
    P("%s %dx%d ch%d fnv %016llx\n", tag, p->w, p->h, p->ch, (unsigned long long)fnv(p->px, (size_t)p->w * p->h * p->ch));
}

static void sec_keys(void) {
    P("## keys\n");
    static const int ns[] = {0, 1, 7, 8, 9, 15, 16, 17, 63, 64, 65, 1000, 65539};
    for (size_t k = 0; k < sizeof ns / sizeof *ns; k++) {
        int n = ns[k];
        uint8_t *d = malloc((size_t)n + 1);
        g_s = 1000 + (uint32_t)n;
        for (int i = 0; i < n; i++) d[i] = (uint8_t)lcg();
        uint64_t c = bgimage_content_hash(d, (size_t)n);
        P("hash %d %016llx src %016llx tpl %016llx photo %016llx\n", n, (unsigned long long)c,
          (unsigned long long)bgimage_source_key(d, (size_t)n, 1860, 2480), (unsigned long long)bgimage_template_key(c, (size_t)n, 100, 200),
          (unsigned long long)bgimage_photo_key(c, (size_t)n, 100, 200));
        free(d);
    }
    int dims[][3] = {{5, 3, 3}, {4, 4, 1}, {1, 1, 3}, {17, 9, 1}};
    for (int i = 0; i < 4; i++) {
        BgPicture p = mkpic(dims[i][0], dims[i][1], dims[i][2], 77 + (uint32_t)i);
        P("key %dx%d ch%d %016llx\n", p.w, p.h, p.ch, (unsigned long long)bgimage_key(&p));
        bgpicture_free(&p);
    }
}

static void sec_normalize(void) {
    P("## normalize\n");
    int cs[][5] = {{7, 5, 1, 20, 20},    {100, 60, 3, 31, 47}, {64, 64, 3, 64, 64}, {3, 200, 1, 100, 100}, {500, 333, 3, 120, 160},
                   {1, 1, 3, 9, 9},      {300, 300, 1, 50, 10}, {20, 10, 3, 200, 100}, {17, 31, 1, 17, 31}, {1000, 3, 1, 33, 40},
                   {123, 456, 3, 61, 99}};
    for (size_t i = 0; i < sizeof cs / sizeof *cs; i++) {
        BgPicture s = mkpic(cs[i][0], cs[i][1], cs[i][2], 500 + (uint32_t)i), o;
        int rc = bgimage_normalize(&s, cs[i][3], cs[i][4], &o);
        P("normalize %d\n", rc);
        dump_pic("  out", &o);
        bgpicture_free(&o);
        BgPicture b = {NULL, 0, 0, 0};
        P("normalize bad %d %d %d\n", bgimage_normalize(&b, 10, 10, &o), bgimage_normalize(&s, 0, 10, &o), bgimage_normalize(&s, 10, -1, &o));
        Canvas lum;
        if (bgimage_luma(&s, &lum) == 0) {
            P("  luma %dx%d stride %d fnv %016llx\n", lum.w, lum.h, lum.stride, (unsigned long long)fnv(lum.px, (size_t)lum.w * lum.h));
            canvas_free(&lum);
        }
        bgpicture_free(&s);
    }
}

static void sec_orient(void) {
    P("## orient\n");
    int dims[][3] = {{5, 3, 3}, {4, 7, 1}, {1, 6, 3}};
    for (int i = 0; i < 3; i++)
        for (int o = 0; o <= 9; o++) {
            BgPicture p = mkpic(dims[i][0], dims[i][1], dims[i][2], 900 + (uint32_t)i);
            int rc = bgimage_orient(&p, o);
            char tag[64];
            snprintf(tag, sizeof tag, "orient %d rc %d", o, rc);
            dump_pic(tag, &p);
            bgpicture_free(&p);
        }
    BgPicture e = {NULL, 3, 3, 3};
    P("orient empty %d\n", bgimage_orient(&e, 6));
}

static uint8_t *slurp(const char *name, size_t *len) {
    char path[1000];
    snprintf(path, sizeof path, "%s/%s", FIX, name);
    return bgimage_read(path, (size_t)1 << 30, len);
}

static void sec_exif(void) {
    P("## exif\n");
    static const char *names[] = {"exif_le_1.jpg", "exif_le_2.jpg", "exif_le_3.jpg", "exif_le_4.jpg", "exif_le_5.jpg", "exif_le_6.jpg",
                                  "exif_le_7.jpg", "exif_le_8.jpg", "exif_be_1.jpg", "exif_be_2.jpg", "exif_be_3.jpg", "exif_be_4.jpg",
                                  "exif_be_5.jpg", "exif_be_6.jpg", "exif_be_7.jpg", "exif_be_8.jpg", "exif_0.jpg", "exif_9.jpg",
                                  "exif_pad.jpg", "base_420.jpg", "gray8.png", "tiny.bin", "empty.bin", "junk.bin"};
    for (size_t i = 0; i < sizeof names / sizeof *names; i++) {
        size_t n = 0;
        uint8_t *d = slurp(names[i], &n);
        P("exif %s %d\n", names[i], d ? bgimage_exif_orientation(d, n) : -99);
        free(d);
    }
    // a fuzz: mutate the first 80 bytes of an Exif jpeg
    size_t n = 0;
    uint8_t *d = slurp("exif_le_6.jpg", &n), *m = malloc(n);
    uint64_t acc = 0;
    int counts[10] = {0};
    g_s = 4242;
    for (int it = 0; it < 20000; it++) {
        memcpy(m, d, n);
        int k = 1 + (int)(lcg() % 3);
        while (k--) m[lcg() % 80] = (uint8_t)lcg();
        size_t cut = (lcg() % 4 == 0) ? lcg() % n : n;
        int o = bgimage_exif_orientation(m, cut);
        counts[o]++;
        acc = acc * 31 + (uint64_t)o;
    }
    P("exif fuzz %016llx", (unsigned long long)acc);
    for (int i = 0; i < 10; i++) P(" %d", counts[i]);
    P("\n");
    free(d);
    free(m);
}

static void sec_decode(void) {
    P("## decode\n");
    static const char *names[] = {
        "gray8.png", "rgb8.png", "rgba8.png", "ga8.png", "grayrgb.png", "grayrgb1.png", "rgb16.png", "gray16.png", "rgba16.png", "pal4.png",
        "pal8trns.png", "gray1.png", "interlaced_bad.png", "adam7.png", "cgbi.png", "fake_9000.png", "fake_7000sq.png", "fake_5000x4000.png",
        "fake_rgb16_6000x3000.png", "fake_rgb16_3000x3000.png", "fake_pal_6000x5000.png", "fake_inter_4000x4000.png", "fake_rgba_6000x6000.png",
        "fake_6000.png", "fake_6001.png", "fake_1bit_8000.png", "bomb.png", "short.png", "trunc.png", "junk.bin", "tiny.bin", "empty.bin",
        "p5.pgm", "p6.ppm", "p6_16.ppm", "p5_16.pgm", "p6_short.ppm", "p6_100.ppm", "base_420.jpg", "base_444.jpg", "base_422.jpg",
        "base_gray.jpg", "prog_420.jpg", "prog_444.jpg", "prog_gray.jpg", "exif_le_6.jpg", "exif_pad.jpg"};
    for (size_t i = 0; i < sizeof names / sizeof *names; i++) {
        size_t n = 0;
        uint8_t *d = slurp(names[i], &n);
        if (!d) { P("%s: unreadable\n", names[i]); continue; }
        int w = -1, h = -1;
        const char *why = "";
        int rc = bgimage_check(d, n, n, &w, &h, &why);
        P("%s: check %d %d %d [%s]\n", names[i], rc, w, h, rc ? why : "");
        P("  cost %zu\n", bgimage_decode_cost(d, n, n));
        if (n > 20) {
            w = h = -1; why = "";
            rc = bgimage_check(d, 20, n, &w, &h, &why);
            P("  head20 %d %d %d [%s]\n", rc, w, h, rc ? why : "");
        }
        w = h = -1; why = "";
        rc = bgimage_drawable(d, n, &w, &h, &why);
        P("  drawable %d %d %d [%s]\n", rc, w, h, rc ? why : "");
        if (n < 100000 || !strncmp(names[i], "fake_6000", 9) || !strcmp(names[i], "fake_5000x4000.png")) {
            BgPicture p;
            why = "";
            rc = bgimage_decode(d, n, &p, &why);
            P("  decode %d [%s]", rc, rc ? why : "");
            if (rc == 0) P(" %dx%d ch%d fnv %016llx", p.w, p.h, p.ch, (unsigned long long)fnv(p.px, (size_t)p.w * p.h * p.ch));
            P(" peak %zu\n", bgimage_last_decode_peak());
            if (rc == 0) {
                BgPicture o;
                if (bgimage_normalize(&p, 41, 53, &o) == 0) dump_pic("  fit41x53", &o);
                bgpicture_free(&o);
                bgpicture_free(&p);
            }
        }
        free(d);
    }
}

static void put(const char *path, const char *text, size_t n) {
    FILE *f = fopen(path, "wb");
    fwrite(text, 1, n, f);
    fclose(f);
}

static void dump_ix(const BgIndex *ix) {
    P("  n=%d", ix->n);
    for (int i = 0; i < ix->n; i++) P(" %06x=%016llx", (unsigned)ix->e[i].id, (unsigned long long)ix->e[i].key);
    P("\n");
}

static void sec_index(void) {
    P("## index\n");
    char dir[900], path[1000];
    snprintf(dir, sizeof dir, "%s/ixdir", ROOT);
    mkdir(dir, 0755);
    snprintf(path, sizeof path, "%s/index.ini", dir);
    unlink(path);
    BgIndex ix;
    P("load none %d\n", bgindex_load(dir, &ix));
    dump_ix(&ix);
    bgindex_free(&ix);
    static const char *texts[] = {
        "# comment\n000010=00000000000000aa\n000002=00000000000000bb\n",
        "\n\n  5 = 3\n5=3\n05=ff\n0x1f=abc\n0X20=ABCDEF\n",
        "1=10\n2=10\n3=20\n1=30\n",
        "000000=1\n1000000=2\nffffff=3\n1000001=4\n",
        "-1=5\n+7=6\n8=-1\n9=zz\n10=\nA=1g\n",
        "nonsense\n=5\n1==5\n\r\n1=5\r\n2=6\r\n",
        "1=ffffffffffffffffffff\n2=18446744073709551616\n3=0x\n4=0x1\n",
        "1=2\n\0002=3\n",
        "ffffffff=1\n100000000=2\n-ffffffff=3\n",
    };
    static const size_t lens[] = {0, 0, 0, 0, 0, 0, 0, 9, 0};
    for (size_t i = 0; i < sizeof texts / sizeof *texts; i++) {
        size_t n = lens[i] ? lens[i] : strlen(texts[i]);
        put(path, texts[i], n);
        int rc = bgindex_load(dir, &ix);
        P("load text %zu rc %d\n", i, rc);
        dump_ix(&ix);
        if (rc == 0) {
            int sv = bgindex_save(dir, &ix);
            size_t m = 0;
            uint8_t *b = bgimage_read(path, 1 << 20, &m);
            P("  save %d bytes %zu fnv %016llx\n", sv, m, (unsigned long long)fnv(b, m));
            fwrite(b, 1, m, stdout);
            free(b);
        }
        bgindex_free(&ix);
    }
    // claim
    unlink(path);
    char f[1000];
    for (unsigned id = 1; id <= 3; id++) {
        bgimage_file(dir, id, f, sizeof f);
        put(f, "x", 1);
    }
    bgimage_file(dir, 0x20, f, sizeof f);
    put(f, "x", 1);
    bgindex_load(dir, &ix);
    uint64_t keys[] = {0x0000000000000000ull, 0x1000000000000001ull, 0x0000000000000000ull, 0x00000000000000ffull, 0xabcdef0123456789ull,
                       0x0000000000000020ull, 0x0000000000000021ull, 0x0000000000fffffeull, 0x0000000000ffffffull, 0x00000000ffffffffull,
                       0xabcdef0123456789ull};
    for (size_t i = 0; i < sizeof keys / sizeof *keys; i++) {
        uint32_t id = 0;
        bool added = false;
        int rc = bgindex_claim(&ix, dir, keys[i], &id, &added);
        P("claim %016llx rc %d id %06x added %d\n", (unsigned long long)keys[i], rc, (unsigned)id, (int)added);
    }
    dump_ix(&ix);
    P("save %d\n", bgindex_save(dir, &ix));
    uint32_t got;
    P("find %d %d\n", bgindex_find(&ix, 0xabcdef0123456789ull, &got), bgindex_find(&ix, 12345, &got));
    bgindex_free(&ix);
    // unreadable
    chmod(path, 0);
    P("load unreadable %d n=%d\n", bgindex_load(dir, &ix), ix.n);
    chmod(path, 0644);
    P("load readable %d n=%d\n", bgindex_load(dir, &ix), ix.n);
    bgindex_free(&ix);
}

static void sec_files(void) {
    P("## files\n");
    char path[1000];
    static const size_t sizes[] = {0, 1, 7, 8, 9, 65535, 65536, 65537, 131072, 200003};
    for (size_t i = 0; i < sizeof sizes / sizeof *sizes; i++) {
        snprintf(path, sizeof path, "%s/hf%zu.bin", ROOT, sizes[i]);
        uint8_t *d = malloc(sizes[i] + 1);
        g_s = 31 + (uint32_t)sizes[i];
        for (size_t k = 0; k < sizes[i]; k++) d[k] = (uint8_t)lcg();
        put(path, (char *)d, sizes[i]);
        uint64_t h = 0;
        size_t len = 0;
        int rc = bgimage_content_hash_file(path, &h, &len);
        P("hashfile %zu rc %d len %zu same %d\n", sizes[i], rc, len, h == bgimage_content_hash(d, sizes[i]));
        size_t rl = 0;
        uint8_t *r = bgimage_read(path, sizes[i], &rl);
        P("  read max=size %d %zu same %d\n", r != NULL, rl, r && !memcmp(r, d, sizes[i]));
        free(r);
        if (sizes[i] > 0) {
            r = bgimage_read(path, sizes[i] - 1, &rl);
            P("  read max=size-1 %d errno %d\n", r != NULL, r ? 0 : errno);
            free(r);
        }
        free(d);
    }
    uint64_t h;
    size_t len;
    P("hashfile missing %d\n", bgimage_content_hash_file("/nonexistent/x.png", &h, &len));
    P("hashfile dir %d\n", bgimage_content_hash_file(ROOT, &h, &len));
    size_t rl;
    uint8_t *r = bgimage_read("/nonexistent/x.png", 100, &rl);
    P("read missing %d errno %d\n", r != NULL, r ? 0 : errno);
    r = bgimage_read(ROOT, 100, &rl);
    P("read dir %d errno %d\n", r != NULL, r ? 0 : errno);
}

static uint32_t paper(uint32_t id) { return 0x50000000u | id; }

static void render_dump(const char *tag, const Canvas *c) {
    P("%s %dx%d fnv %016llx\n", tag, c->w, c->h, (unsigned long long)fnv(c->px, (size_t)c->stride * c->h));
}

static void sec_render(void) {
    P("## render\n");
    char dir[900], f[1000];
    snprintf(dir, sizeof dir, "%s/nb1/bg", ROOT);
    char mk[1000];
    snprintf(mk, sizeof mk, "mkdir -p '%s'", dir);
    system(mk);
    // pictures
    struct { uint32_t id; int w, h, ch; } pics[] = {{0x123, 40, 30, 1}, {0x124, 40, 30, 1}, {0x201, 8, 6, 1}, {0x202, 8, 6, 1}, {0x203, 8, 6, 1},
                                                     {0x204, 8, 6, 1}, {0x205, 8, 6, 1}, {0x400, 25, 50, 3}};
    for (size_t i = 0; i < sizeof pics / sizeof *pics; i++) {
        BgPicture p = mkpic(pics[i].w, pics[i].h, pics[i].ch, 3000 + (uint32_t)i);
        bgimage_file(dir, pics[i].id, f, sizeof f);
        P("write png %06x %d\n", (unsigned)pics[i].id, bgimage_write_png(f, &p));
        uint8_t *b; size_t n = 0;
        b = bgimage_read(f, 1 << 20, &n);
        P("  file %zu fnv %016llx\n", n, (unsigned long long)fnv(b, n));
        free(b);
        bgpicture_free(&p);
    }
    bgimage_file(dir, 0x300, f, sizeof f);
    put(f, "this is not a png at all, really", 32);
    bgimage_file(dir, 0x301, f, sizeof f);
    put(f, "\x89PNG\r\n\x1a\n", 8);
    Canvas c;
    PaperKind k;
    canvas_init(&c, 40, 30);
    k = (PaperKind)paper(0x123);
    P("undrawn 123 %d (before)\n", (int)bgimage_undrawn("nb1", 0x123));
    bgimage_render(&c, "nb1", k, (Rect){0, 0, 40, 30});
    render_dump("render same size", &c);
    P("undrawn 123 %d (after) missing %d\n", (int)bgimage_undrawn("nb1", 0x123), (int)bgimage_take_missing());
    canvas_fill(&c, (Rect){0, 0, 40, 30}, 17);
    bgimage_render(&c, "nb1", k, (Rect){5, 7, 20, 10});
    render_dump("render region", &c);
    canvas_set_clip(&c, (Rect){10, 10, 10, 10});
    bgimage_render(&c, "nb1", k, (Rect){0, 0, 40, 30});
    render_dump("render clipped", &c);
    canvas_free(&c);
    // fitted: canvas of another size
    int sizes[][2] = {{60, 90}, {100, 40}, {40, 31}, {13, 9}};
    for (int i = 0; i < 4; i++) {
        canvas_init(&c, sizes[i][0], sizes[i][1]);
        bgimage_render(&c, "nb1", k, (Rect){0, 0, sizes[i][0], sizes[i][1]});
        char t[64];
        snprintf(t, sizeof t, "render fitted %dx%d", sizes[i][0], sizes[i][1]);
        render_dump(t, &c);
        Canvas r;
        canvas_init(&r, sizes[i][0], sizes[i][1]);
        canvas_fill(&r, (Rect){0, 0, sizes[i][0], sizes[i][1]}, 99);
        bgimage_render(&r, "nb1", k, (Rect){3, 4, sizes[i][0] / 2, sizes[i][1] / 2});
        render_dump("  region", &r);
        // the region must agree with the whole page
        int agree = 1;
        Rect g = {3, 4, sizes[i][0] / 2, sizes[i][1] / 2};
        g = rect_intersect(g, (Rect){0, 0, sizes[i][0], sizes[i][1]});
        for (int y = g.y; y < g.y + g.h; y++)
            for (int x = g.x; x < g.x + g.w; x++) agree &= r.px[y * r.stride + x] == c.px[y * c.stride + x];
        P("  region agrees %d\n", agree);
        canvas_free(&r);
        canvas_free(&c);
    }
    // an RGB picture drawn as luma
    canvas_init(&c, 25, 50);
    bgimage_render(&c, "nb1", (PaperKind)paper(0x400), (Rect){0, 0, 25, 50});
    render_dump("render rgb", &c);
    canvas_free(&c);
    // missing, wrong, failures
    canvas_init(&c, 40, 30);
    fprintf(stderr, "== missing\n");
    bgimage_render(&c, "nb1", (PaperKind)paper(0x999), (Rect){0, 0, 40, 30});
    render_dump("render missing", &c);
    P("take missing %d then %d\n", (int)bgimage_take_missing(), (int)bgimage_take_missing());
    bgimage_render(&c, "nb1", (PaperKind)paper(0x999), (Rect){0, 0, 40, 30});
    P("again missing %d\n", (int)bgimage_take_missing());
    bgimage_render(&c, "nb1", (PaperKind)paper(0x300), (Rect){0, 0, 40, 30});
    bgimage_render(&c, "nb1", (PaperKind)paper(0x301), (Rect){0, 0, 40, 30});
    bgimage_render(&c, "nb1", (PaperKind)paper(0x300), (Rect){0, 0, 40, 30});
    P("bad pngs missing %d\n", (int)bgimage_take_missing());
    bgimage_render(&c, "", (PaperKind)paper(0x123), (Rect){0, 0, 40, 30});
    P("empty nb missing %d\n", (int)bgimage_take_missing());
    bgimage_render(&c, "nb1", (PaperKind)4, (Rect){0, 0, 40, 30});
    P("not an image paper missing %d\n", (int)bgimage_take_missing());
    bgimage_render(&c, "nb-nope", (PaperKind)paper(0x123), (Rect){0, 0, 40, 30});
    P("other notebook missing %d\n", (int)bgimage_take_missing());
    bgimage_render(&c, "nb1", (PaperKind)paper(0x123), (Rect){100, 100, 4, 4});
    P("empty region missing %d\n", (int)bgimage_take_missing());
    // the generation and the remembered failures
    unsigned g0 = bgimage_generation();
    bgimage_retry_missing();
    P("generation +%u\n", bgimage_generation() - g0);
    fprintf(stderr, "== after retry\n");
    bgimage_render(&c, "nb1", (PaperKind)paper(0x999), (Rect){0, 0, 40, 30});
    // a picture made now is found after a retry
    BgPicture p = mkpic(40, 30, 1, 5555);
    bgimage_file(dir, 0x999, f, sizeof f);
    bgimage_write_png(f, &p);
    bgpicture_free(&p);
    P("take %d\n", (int)bgimage_take_missing());
    bgimage_render(&c, "nb1", (PaperKind)paper(0x999), (Rect){0, 0, 40, 30});
    P("still missing %d\n", (int)bgimage_take_missing());
    bgimage_retry_missing();
    bgimage_render(&c, "nb1", (PaperKind)paper(0x999), (Rect){0, 0, 40, 30});
    P("after retry missing %d\n", (int)bgimage_take_missing());
    render_dump("render made", &c);
    // LRU
    bgimage_forget(NULL);
    canvas_free(&c);
    canvas_init(&c, 8, 6);
    for (uint32_t id = 0x201; id <= 0x205; id++) {
        bgimage_render(&c, "nb1", (PaperKind)paper(id), (Rect){0, 0, 8, 6});
        P("drew %x undrawn:", (unsigned)id);
        for (uint32_t q = 0x201; q <= 0x205; q++) P(" %d", (int)bgimage_undrawn("nb1", q));
        P("\n");
        if (id == 0x203) bgimage_render(&c, "nb1", (PaperKind)paper(0x201), (Rect){0, 0, 8, 6});  // 201 used again
    }
    bgimage_forget("nb1");
    P("forgot:");
    for (uint32_t q = 0x201; q <= 0x205; q++) P(" %d", (int)bgimage_undrawn("nb1", q));
    P("\n");
    P("undrawn missing file %d, bad nb %d\n", (int)bgimage_undrawn("nb1", 0x7777), (int)bgimage_undrawn("", 0x201));
    // failures: more than 16 remembered
    fprintf(stderr, "== many failures\n");
    for (uint32_t id = 0x500; id < 0x500 + 20; id++) bgimage_render(&c, "nb1", (PaperKind)paper(id), (Rect){0, 0, 8, 6});
    for (uint32_t id = 0x500; id < 0x500 + 20; id++) bgimage_render(&c, "nb1", (PaperKind)paper(id), (Rect){0, 0, 8, 6});
    bgimage_forget("nb1");
    bgimage_render(&c, "nb1", (PaperKind)paper(0x500), (Rect){0, 0, 8, 6});
    canvas_free(&c);
}

int main(int argc, char **argv) {
    if (argc < 3) return 2;
    FIX = argv[1];
    ROOT = argv[2];
    sec_keys();
    sec_normalize();
    sec_orient();
    sec_exif();
    sec_decode();
    sec_index();
    sec_files();
    sec_render();
    return 0;
}
