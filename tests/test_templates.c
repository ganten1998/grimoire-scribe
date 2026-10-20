// test_templates.c: image papers (pictures as page backgrounds), templates and imports
// (src/bgimage.h, src/templates.h, docs/FORMAT.md, Image papers).
#include <dirent.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "beta_format.h"
#include "bgimage.h"
#include "bgjob.h"
#include "export.h"
#include "templates.h"
#include "testlib.h"

static char root[600];
static const float DPI = 300.f;
enum { PW = 600, PH = 800 };  // the page size the tests' notebooks get (store_set_device in main)

// ---- making pictures ----

static uint32_t g_rng = 12345u;
static uint8_t rnd8(void) {
    g_rng = g_rng * 1103515245u + 12345u;
    return (uint8_t)(g_rng >> 16);
}

static BgPicture picture(int w, int h, int ch) {
    BgPicture p = {malloc((size_t)w * (size_t)h * (size_t)ch), w, h, ch};
    return p;
}

// Noise: any filter or offset mistake shows.
static BgPicture noise(int w, int h, int ch) {
    BgPicture p = picture(w, h, ch);
    for (size_t i = 0; i < (size_t)w * (size_t)h * (size_t)ch; i++) p.px[i] = rnd8();
    return p;
}

static BgPicture solid(int w, int h, int ch, uint8_t v) {
    BgPicture p = picture(w, h, ch);
    memset(p.px, v, (size_t)w * (size_t)h * (size_t)ch);
    return p;
}

static void put_file(const char *path, const void *d, size_t n) {
    char dir[700];
    strlcpy_q(dir, path, sizeof dir);
    char *slash = strrchr(dir, '/');
    if (slash) *slash = 0;
    mkdir_p(dir);
    CHECK(write_file_atomic(path, d, n) == 0);
}

static void put_png(const char *path, const BgPicture *p) {
    char dir[700];
    strlcpy_q(dir, path, sizeof dir);
    char *slash = strrchr(dir, '/');
    if (slash) *slash = 0;
    mkdir_p(dir);
    CHECK(bgimage_write_png(path, p) == 0);
}

// Writes picture p as notebook nb's bg/<id>.png, as a template's or an import's would be.
static void put_picture(const NotebookInfo *nb, uint32_t id, const BgPicture *p) {
    char dir[700], file[800];
    CHECK(bgimage_dir(nb->id, dir, sizeof dir) == 0);
    bgimage_file(dir, id, file, sizeof file);
    put_png(file, p);
}

static uint8_t *slurp(const char *path, size_t *len) { return read_file(path, len); }

// A PNG of `bytes` bytes or more: picture p, with a chunk of its own (an ancillary one, which
// decoders skip) of zeros after its header, as a big ICC profile or text would be.
static void padded_png(const char *path, const BgPicture *p, size_t bytes) {
    char tmp[700];
    snprintf(tmp, sizeof tmp, "%s.plain.png", path);
    put_png(tmp, p);
    size_t len;
    uint8_t *png = slurp(tmp, &len);
    unlink(tmp);
    uint8_t *out = calloc(1, len + bytes + 12);
    if (png && out) {
        memcpy(out, png, 33);  // the signature and IHDR
        uint8_t *c = out + 33;
        c[0] = (uint8_t)(bytes >> 24);
        c[1] = (uint8_t)(bytes >> 16);
        c[2] = (uint8_t)(bytes >> 8);
        c[3] = (uint8_t)bytes;
        memcpy(c + 4, "prvT", 4);
        memcpy(c + 8 + bytes + 4, png + 33, len - 33);  // after its CRC: the rest
        put_file(path, out, len + bytes + 12);
    }
    free(png);
    free(out);
}

static bool exists(const char *path) { return access(path, F_OK) == 0; }

// What the code under test logs (grimoire.log is its stderr), from log_capture to log_captured
// (the text, which the caller frees).
static int g_log_saved = -1;
static char g_log_path[700];

static void log_capture(void) {
    snprintf(g_log_path, sizeof g_log_path, "%s/captured.log", root);
    fflush(stderr);
    g_log_saved = dup(2);
    int fd = open(g_log_path, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) {
        dup2(fd, 2);
        close(fd);
    }
}

static char *log_captured(void) {
    fflush(stderr);
    if (g_log_saved >= 0) {
        dup2(g_log_saved, 2);
        close(g_log_saved);
        g_log_saved = -1;
    }
    size_t len;
    char *t = read_file(g_log_path, &len);
    if (t) fputs(t, stderr);  // (still in the test's own log)
    return t;
}

static int count_prefix(const char *dir, const char *prefix) {
    int n = 0;
    DIR *d = opendir(dir);
    for (struct dirent *e; d && (e = readdir(d));)
        if (!strncmp(e->d_name, prefix, strlen(prefix))) n++;
    if (d) closedir(d);
    return n;
}

// Waits for the background jobs to finish (sanitized children on a busy machine: by the clock).
static int drain_jobs(int *last_kind) {
    int result = 0;
    uint32_t until = now_ms() + 60000;
    while (bg_busy() && (int32_t)(now_ms() - until) < 0) {
        int kind = -1, r = bg_poll_kind(&kind);
        if (kind >= 0 && last_kind) *last_kind = kind;
        if (r) result = r;
        usleep(5000);
    }
    int kind = -1, r = bg_poll_kind(&kind);
    if (kind >= 0 && last_kind) *last_kind = kind;
    return r ? r : result;
}

static Stroke *line_stroke(ToolKind t, float y, uint32_t seed) {
    ToolParams p;
    tool_params_default(t, &p);
    Stroke *s = stroke_new(t, &p, seed);
    for (int i = 0; i < 30; i++) {
        PenSample q = {40.f + (float)i * 17.f, y + (float)(i % 4) * 6.f, 0.3f + (float)i / 60.f, 4.f, -3.f, (uint32_t)i * 7u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// ---- the paper values ----

TEST(image_kinds) {
    PaperKind t = paper_image(PAPER_ROLE_TEMPLATE, 0x0A1B2Cu), d = paper_image(PAPER_ROLE_DOCUMENT, 0x0A1B2Cu);
    CHECK((uint32_t)t == 0x500A1B2Cu && (uint32_t)d == 0x510A1B2Cu);
    CHECK(paper_is_image(t) && paper_is_image(d) && !paper_is_document(t) && paper_is_document(d));
    CHECK(paper_image_id(t) == 0x0A1B2Cu && paper_image_id(d) == 0x0A1B2Cu);
    CHECK(paper_is_known(t) && paper_is_known(d) && paper_is_known((PaperKind)0x50000000u));
    // Roles 2 and 3, and the colour bits (26, 27): a later version's paper.
    const uint32_t newer[] = {0x52000001u, 0x53000001u, 0x54000001u, 0x58000001u, 0x5C000001u, 0x5F000001u};
    for (size_t i = 0; i < sizeof newer / sizeof newer[0]; i++) {
        CHECK(!paper_is_image((PaperKind)newer[i]) && !paper_is_known((PaperKind)newer[i]));
        CHECK(!strcmp(paper_name((PaperKind)newer[i]), "Newer paper"));
    }
    CHECK(!strcmp(paper_name(t), "Template") && !strcmp(paper_name(d), "Imported page"));
    // A new notebook or a setting never takes one: it needs its notebook's picture.
    CHECK(paper_sanitize((uint32_t)t, PAPER_MEDIUM) == PAPER_MEDIUM);
    CHECK(paper_sanitize((uint32_t)d, PAPER_BLANK) == PAPER_BLANK);
    NotebookInfo nb;
    CHECK(store_create("Fresh", "", t, &nb) == 0 && nb.paper == PAPER_MEDIUM);
    // Drawn alone (no notebook), an image paper is blank, like a paper this version does not know.
    Canvas a, b;
    canvas_init(&a, 60, 80);
    canvas_init(&b, 60, 80);
    canvas_fill(&a, (Rect){0, 0, 60, 80}, 0);
    paper_take_image_blank_count();
    paper_render(&a, t, (Rect){0, 0, 60, 80}, 30.f);
    CHECK(memcmp(a.px, b.px, 60 * 80) == 0);
    CHECK(paper_take_image_blank_count() == 1 && paper_take_image_blank_count() == 0);  // (a caller to fix)
    canvas_free(&a);
    canvas_free(&b);
    // Page version 2, the u16 Blank (1.0.0-beta's reading of it), the value kept.
    Page *p = page_new(PW, PH, d);
    uint8_t *data;
    size_t len;
    CHECK(page_serialize(p, &data, &len) == 0);
    CHECK(data[4] == 2 && data[5] == 0 && data[6] == PAPER_BLANK && data[7] == 0);
    Page *q = NULL;
    CHECK(page_deserialize(data, len, &q) == 0 && q && q->paper == d && !q->readonly);
    page_free(q);
    free(data);
    page_free(p);
}

// ---- drawing ----

// Renders page p whole, in four tiles and in random rects: all three agree, pixel for pixel.
static void check_pure(const Page *p, const char *what) {
    int w = p->w, h = p->h;
    Canvas full, tiles, part;
    canvas_init(&full, w, h);
    canvas_init(&tiles, w, h);
    canvas_init(&part, w, h);
    page_render(&full, p, (Rect){0, 0, w, h}, DPI);
    canvas_fill(&tiles, (Rect){0, 0, w, h}, 7);
    int sx = w / 3 + 1, sy = h / 2 - 3;
    Rect q[4] = {{0, 0, sx, sy}, {sx, 0, w - sx, sy}, {0, sy, sx, h - sy}, {sx, sy, w - sx, h - sy}};
    for (int i = 0; i < 4; i++) page_render(&tiles, p, q[i], DPI);
    bool same = !memcmp(full.px, tiles.px, (size_t)w * (size_t)h);
    for (int i = 0; i < 40 && same; i++) {
        int x = (int)(rnd8() * (uint32_t)w / 256u), y = (int)(rnd8() * (uint32_t)h / 256u);
        Rect r = {x, y, 1 + (int)(rnd8() % 200u), 1 + (int)(rnd8() % 200u)};
        r = rect_intersect(r, (Rect){0, 0, w, h});
        canvas_fill(&part, (Rect){0, 0, w, h}, 3);
        page_render(&part, p, r, DPI);
        for (int yy = r.y; yy < r.y + r.h && same; yy++)
            same = !memcmp(&part.px[yy * part.stride + r.x], &full.px[yy * full.stride + r.x], (size_t)r.w);
    }
    if (!same) fprintf(stderr, "  %s: a region drawn alone differs from the whole page\n", what);
    CHECK(same);
    canvas_free(&full);
    canvas_free(&tiles);
    canvas_free(&part);
}

TEST(image_paper_is_pure) {
    NotebookInfo nb;
    CHECK(store_create("Pure", "", PAPER_BLANK, &nb) == 0);
    BgPicture src = noise(500, 700, 3), fit;
    CHECK(bgimage_normalize(&src, PW, PH, &fit) == 0 && fit.w == PW && fit.h == PH && fit.ch == 3);
    put_picture(&nb, 0x123456u, &fit);
    Page *p = page_new(PW, PH, paper_image(PAPER_ROLE_TEMPLATE, 0x123456u));
    strlcpy_q(p->nb, nb.id, sizeof p->nb);
    // The paper alone is the picture's luma, exactly.
    Canvas paper, luma;
    canvas_init(&paper, PW, PH);
    page_paper_render(&paper, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(bgimage_luma(&fit, &luma) == 0 && !memcmp(paper.px, luma.px, (size_t)PW * PH));
    canvas_free(&luma);
    canvas_free(&paper);
    // With ink, and an erase stroke that restores toward the picture under it.
    page_add(p, line_stroke(TOOL_MARKER, 200.f, 5u));
    page_add(p, line_stroke(TOOL_PENCIL, 230.f, 6u));
    Stroke *e = line_stroke(TOOL_ERASER, 215.f, erase_seed(TOOL_ERASER, 77u));
    page_add(p, e);
    check_pure(p, "same size");
    // Drawn at another page size (a notebook an older version moved to a Scribe of the other size):
    // the picture is fitted to it, and still the same whichever way it is cut.
    const int sizes[3][2] = {{643, 857}, {600, 900}, {700, 700}};
    for (int i = 0; i < 3; i++) {
        p->w = sizes[i][0];
        p->h = sizes[i][1];
        check_pure(p, "another size");
    }
    // A 3:4 picture fills a 3:4 page of any size: no white band on either side.
    Canvas c;
    canvas_init(&c, 643, 857);
    canvas_fill(&c, (Rect){0, 0, 643, 857}, 0);
    Page *bare = page_new(643, 857, p->paper);
    strlcpy_q(bare->nb, nb.id, sizeof bare->nb);
    BgPicture black = solid(PW, PH, 1, 0);
    put_picture(&nb, 0x000777u, &black);
    bare->paper = paper_image(PAPER_ROLE_TEMPLATE, 0x000777u);
    page_render(&c, bare, (Rect){0, 0, 643, 857}, DPI);
    int white = 0;
    for (int i = 0; i < 643 * 857; i++) white += c.px[i] == 255;
    CHECK(white == 0);
    // Every page render took the paper from the picture, never from paper_render.
    CHECK(paper_take_image_blank_count() == 0);
    canvas_free(&c);
    page_free(bare);
    bgpicture_free(&black);
    page_free(p);
    bgpicture_free(&src);
    bgpicture_free(&fit);
}

TEST(missing_picture_draws_white) {
    NotebookInfo nb;
    CHECK(store_create("Missing", "", PAPER_BLANK, &nb) == 0);
    Page *p = page_new(PW, PH, paper_image(PAPER_ROLE_TEMPLATE, 0xABCDEFu));
    strlcpy_q(p->nb, nb.id, sizeof p->nb);
    bgimage_take_missing();
    Canvas c;
    canvas_init(&c, PW, PH);
    canvas_fill(&c, (Rect){0, 0, PW, PH}, 0);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    int white = 0;
    for (int i = 0; i < PW * PH; i++) white += c.px[i] == 255;
    CHECK(white == PW * PH);
    CHECK(bgimage_take_missing());
    CHECK(!bgimage_take_missing());
    // Remembered: the file arriving later changes nothing until the failures are forgotten.
    BgPicture g = solid(PW, PH, 1, 40);
    put_picture(&nb, 0xABCDEFu, &g);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(c.px[100 * PW + 100] == 255 && bgimage_take_missing());
    bgimage_retry_missing();
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(c.px[100 * PW + 100] == 40 && !bgimage_take_missing());
    // An unreadable file draws white too; so does a page of no notebook.
    char dir[700], file[800];
    bgimage_dir(nb.id, dir, sizeof dir);
    bgimage_file(dir, 0x0000AAu, file, sizeof file);
    put_file(file, "not a picture", 13);
    p->paper = paper_image(PAPER_ROLE_DOCUMENT, 0x0000AAu);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(c.px[0] == 255 && c.px[PW * PH - 1] == 255 && bgimage_take_missing());
    p->nb[0] = 0;
    p->paper = paper_image(PAPER_ROLE_TEMPLATE, 0xABCDEFu);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(c.px[100 * PW + 100] == 255);
    canvas_free(&c);
    bgpicture_free(&g);
    page_free(p);
}

// ---- decoding and fitting ----

static BgPicture load_fixture(const char *name, const char **why) {
    char path[700];
    snprintf(path, sizeof path, "tests/fixtures/pictures/%s", name);
    size_t len = 0;
    uint8_t *d = slurp(path, &len);
    BgPicture p = {0};
    *why = "no fixture";
    if (d && bgimage_decode(d, len, &p, why) != 0) p.px = NULL;
    free(d);
    return p;
}

static uint8_t at(const BgPicture *p, int x, int y, int c) { return p->px[((size_t)y * (size_t)p->w + (size_t)x) * (size_t)p->ch + (size_t)c]; }

TEST(decoding) {
    const char *why;
    // A 16-bit gray PNG: cut to 8 bits, a gradient from black at the top to white at the bottom.
    BgPicture g16 = load_fixture("gray16.png", &why);
    CHECK(g16.px && g16.w == 60 && g16.h == 80 && g16.ch == 1);
    if (g16.px) CHECK(at(&g16, 30, 0, 0) < 10 && at(&g16, 30, 79, 0) > 245 && at(&g16, 30, 20, 0) < at(&g16, 30, 60, 0));
    // A palette PNG with colour: RGB.
    BgPicture pal = load_fixture("palette.png", &why);
    CHECK(pal.px && pal.ch == 3);
    if (pal.px) CHECK(at(&pal, 10, 10, 0) > 180 && at(&pal, 10, 10, 1) < 90 && at(&pal, 50, 70, 2) > 180);
    // Transparency over white: the opaque black quarter stays black, transparent pixels turn white,
    // half-transparent red turns pink.
    BgPicture al = load_fixture("alpha.png", &why);
    CHECK(al.px && al.ch == 3);
    if (al.px) {
        CHECK(at(&al, 5, 5, 0) == 0 && at(&al, 5, 5, 1) == 0 && at(&al, 5, 5, 2) == 0);
        CHECK(at(&al, 50, 5, 0) == 255 && at(&al, 50, 5, 1) == 255 && at(&al, 50, 5, 2) == 255);
        CHECK(at(&al, 50, 70, 0) == 255 && at(&al, 50, 70, 1) > 110 && at(&al, 50, 70, 1) < 145);
    }
    // A progressive JPEG.
    BgPicture pj = load_fixture("progressive.jpg", &why);
    CHECK(pj.px && pj.w == 60 && pj.h == 80 && pj.ch == 3);
    if (pj.px) CHECK(at(&pj, 30, 40, 1) > at(&pj, 30, 40, 0) && at(&pj, 30, 40, 1) > at(&pj, 30, 40, 2));
    // PGM.
    BgPicture pgm = load_fixture("gray.pgm", &why);
    CHECK(pgm.px && pgm.w == 30 && pgm.h == 40 && pgm.ch == 1);
    // Damage.
    BgPicture bad = load_fixture("broken.png", &why);
    CHECK(!bad.px);
    // An RGB picture whose pixels are all gray is kept gray.
    BgPicture rgb = solid(20, 30, 3, 90);
    for (int i = 0; i < 20 * 30; i++) rgb.px[3 * i] = rgb.px[3 * i + 1] = rgb.px[3 * i + 2] = (uint8_t)(i % 251);
    char path[700];
    snprintf(path, sizeof path, "%s/grayrgb.png", root);
    put_png(path, &rgb);
    size_t len;
    uint8_t *d = slurp(path, &len);
    BgPicture back = {0};
    CHECK(d && bgimage_decode(d, len, &back, &why) == 0 && back.ch == 1 && back.px[123] == 123 % 251);
    free(d);
    // Luma, Rec. 709.
    BgPicture col = picture(3, 1, 3);
    const uint8_t px[9] = {255, 0, 0, 0, 255, 0, 0, 0, 255};
    memcpy(col.px, px, 9);
    Canvas l;
    CHECK(bgimage_luma(&col, &l) == 0 && l.px[0] == 54 && l.px[1] == 182 && l.px[2] == 18);
    canvas_free(&l);
    bgpicture_free(&col);
    bgpicture_free(&g16);
    bgpicture_free(&pal);
    bgpicture_free(&al);
    bgpicture_free(&pj);
    bgpicture_free(&pgm);
    bgpicture_free(&rgb);
    bgpicture_free(&back);
}

// A PNG of w x h, `depth` bits a channel of colour type `color` (0 gray, 2 RGB, 6 RGBA), that is
// only its header (signature, IHDR, the start of an empty IDAT): enough for the size checks, which
// stop at the first IDAT.
static size_t png_header_of(uint8_t *out, uint32_t w, uint32_t h, uint8_t depth, uint8_t color) {
    static const uint8_t sig[8] = {0x89, 'P', 'N', 'G', '\r', '\n', 0x1A, '\n'};
    memcpy(out, sig, 8);
    const uint8_t ihdr[25] = {0, 0, 0, 13, 'I', 'H', 'D', 'R', (uint8_t)(w >> 24), (uint8_t)(w >> 16), (uint8_t)(w >> 8),
                              (uint8_t)w, (uint8_t)(h >> 24), (uint8_t)(h >> 16), (uint8_t)(h >> 8), (uint8_t)h, depth, color, 0,
                              0, 0, 0, 0, 0, 0};
    memcpy(out + 8, ihdr, 25);
    const uint8_t idat[12] = {0, 0, 0, 0, 'I', 'D', 'A', 'T', 0, 0, 0, 0};
    memcpy(out + 33, idat, 12);
    return 45;
}

static size_t png_header(uint8_t *out, uint32_t w, uint32_t h) { return png_header_of(out, w, h, 8, 0); }

TEST(oversized_refused) {
    uint8_t b[64];
    int w, h;
    const char *why = "";
    size_t n = png_header(b, 8000, 6000);  // 48 megapixels
    CHECK(bgimage_check(b, n, n, &w, &h, &why) != 0 && strstr(why, "40 megapixels"));
    n = png_header(b, 9000, 10);  // past 8192 a side
    CHECK(bgimage_check(b, n, n, &w, &h, &why) != 0 && strstr(why, "8192"));
    n = png_header(b, 6000, 6000);  // 36 megapixels: taken
    CHECK(bgimage_check(b, n, n, &w, &h, &why) == 0 && w == 6000 && h == 6000);
    BgPicture p;
    n = png_header(b, 8000, 6000);
    CHECK(bgimage_decode(b, n, &p, &why) != 0 && !p.px);
    // The memory it takes to decode, not only its pixels: 7900 x 5000 (39.5 MP) is taken in 8-bit
    // gray (about 80 MB), and refused as 16-bit RGBA (about 630 MB).
    n = png_header_of(b, 7900, 5000, 8, 0);
    CHECK(bgimage_check(b, n, n, &w, &h, &why) == 0);
    size_t gray = bgimage_decode_cost(b, n, n);
    CHECK(gray > (size_t)79000000 && gray < (size_t)82000000);
    n = png_header_of(b, 7900, 5000, 16, 6);
    CHECK(bgimage_check(b, n, n, &w, &h, &why) != 0 && strstr(why, "memory"));
    CHECK(bgimage_decode_cost(b, n, n) > ((size_t)600 << 20));
    CHECK(bgimage_decode(b, n, &p, &why) == -1 && !p.px && strstr(why, "memory"));
    // 16-bit RGB: 12 bytes a pixel at its peak, so 20 MP is taken and 25 MP is not.
    n = png_header_of(b, 5000, 4000, 16, 2);
    CHECK(bgimage_check(b, n, n, &w, &h, &why) == 0);
    n = png_header_of(b, 5000, 5000, 16, 2);
    CHECK(bgimage_check(b, n, n, &w, &h, &why) != 0 && strstr(why, "memory"));
    // The file's own bytes count too: the same header as the start of a 250 MB file is refused.
    n = png_header_of(b, 5000, 4000, 16, 2);
    CHECK(bgimage_check(b, n, (size_t)250 << 20, &w, &h, &why) != 0);
}

// ---- the decode budget ----

typedef struct {
    uint8_t *d;
    size_t n;
    uint32_t acc;
    int bits;
} Bits;

static void put_bits(Bits *b, uint32_t v, int n) {  // deflate packs bits from the lowest up
    for (int i = 0; i < n; i++) {
        b->acc |= ((v >> i) & 1u) << b->bits;
        if (++b->bits == 8) {
            b->d[b->n++] = (uint8_t)b->acc;
            b->acc = 0;
            b->bits = 0;
        }
    }
}

static void put_code(Bits *b, uint32_t code, int n) {  // and a Huffman code from its top bit down
    for (int i = n - 1; i >= 0; i--) put_bits(b, (code >> i) & 1u, 1);
}

// A 1x1 gray PNG whose IDAT inflates to `bytes` zeros (a "zlib bomb": the picture needs 2): a zlib
// stream of one block of fixed codes, a 0, then copies of 258 bytes from 1 back.
static uint8_t *png_bomb(size_t bytes, size_t *len) {
    size_t copies = bytes / 258, zcap = 16 + (copies * 13 + 32) / 8;
    uint8_t *png = calloc(1, 64 + zcap);
    size_t n = png_header_of(png, 1, 1, 8, 0) - 12;  // the signature and IHDR, no IDAT yet
    Bits b = {png + n + 8, 0, 0, 0};
    b.d[b.n++] = 0x78;  // zlib: deflate, 32 KB window
    b.d[b.n++] = 0x01;
    put_bits(&b, 1, 1);  // the last block,
    put_bits(&b, 1, 2);  // of fixed codes
    put_code(&b, 0x30, 8);  // literal 0
    for (size_t i = 0; i < copies; i++) {
        put_code(&b, 0xC5, 8);  // length 258
        put_code(&b, 0, 5);     // distance 1
    }
    put_code(&b, 0, 7);  // the end of the block
    if (b.bits) put_bits(&b, 0, 8 - b.bits);
    b.n += 4;  // its Adler-32 (left 0: never reached)
    uint8_t *c = png + n;
    c[0] = (uint8_t)(b.n >> 24);
    c[1] = (uint8_t)(b.n >> 16);
    c[2] = (uint8_t)(b.n >> 8);
    c[3] = (uint8_t)b.n;
    memcpy(c + 4, "IDAT", 4);
    size_t at = n + 8 + b.n + 4;  // (its CRC left 0: stb_image does not check it)
    static const uint8_t iend[12] = {0, 0, 0, 0, 'I', 'E', 'N', 'D', 0xAE, 0x42, 0x60, 0x82};
    memcpy(png + at, iend, 12);
    *len = at + 12;
    return png;
}

TEST(decode_budget) {
    // A bomb: its header is a fine 1x1 picture, and decoding it stops at twice what that needs.
    size_t len;
    uint8_t *bomb = png_bomb((size_t)64 << 20, &len);
    int w = 0, h = 0;
    const char *why = "";
    CHECK(bgimage_check(bomb, len, len, &w, &h, &why) == 0 && w == 1 && h == 1);
    size_t cost = bgimage_decode_cost(bomb, len, len);
    BgPicture p;
    CHECK(bgimage_decode(bomb, len, &p, &why) == -1 && !p.px && strstr(why, "damaged"));
    CHECK(bgimage_last_decode_peak() <= 2 * cost + (1u << 20) && bgimage_last_decode_peak() < ((size_t)4 << 20));
    free(bomb);
    // Zeros after the picture's own data are allowed (stb_image reads what it needs), up to there.
    bomb = png_bomb(16u << 10, &len);
    CHECK(bgimage_decode(bomb, len, &p, &why) == 0 && p.w == 1 && p.h == 1 && p.px && p.px[0] == 0);
    bgpicture_free(&p);
    free(bomb);
    // Every kind of picture takes no more to decode than its header says.
    static const char *const files[] = {"gray16.png",       "gray16-big.png",  "palette.png",    "palette-trns.png",
                                        "alpha.png",        "interlaced.png",  "interlaced16.png", "progressive.jpg",
                                        "subsampled.jpg",   "baseline.jpg",    "gray.pgm",       "gray16.pgm"};
    for (size_t i = 0; i < sizeof files / sizeof files[0]; i++) {
        char path[700];
        snprintf(path, sizeof path, "tests/fixtures/pictures/%s", files[i]);
        uint8_t *d = slurp(path, &len);
        size_t c = d ? bgimage_decode_cost(d, len, len) : 0;
        int dec = d ? bgimage_decode(d, len, &p, &why) : -1;
        size_t peak = bgimage_last_decode_peak();
        if (dec != 0 || peak == 0 || peak > c) fprintf(stderr, "  %s: decode %d, peak %zu, cost %zu\n", files[i], dec, peak, c);
        CHECK(dec == 0 && peak > 0 && peak <= c);
        bgpicture_free(&p);
        free(d);
    }
    // Headers stb_image reads that are less plain: an iPhone PNG (its CgBI chunk before IHDR, its
    // data raw deflate) and a JPEG with padding between two segments. Taken, and decoded.
    {
        BgPicture g = noise(97, 61, 1);
        char path[700];
        snprintf(path, sizeof path, "%s/plain.png", root);
        put_png(path, &g);
        uint8_t *png = slurp(path, &len), *ip = calloc(1, len + 64);
        size_t at = 8, n = 0;
        memcpy(ip, png, 8);
        n = 8;
        static const uint8_t cgbi[16] = {0, 0, 0, 4, 'C', 'g', 'B', 'I', 0x50, 0x00, 0x20, 0x02, 0, 0, 0, 0};
        memcpy(ip + n, cgbi, 16);
        n += 16;
        while (png && at + 12 <= len) {  // the chunks, the IDAT's zlib header and Adler-32 taken off
            uint32_t cl = (uint32_t)png[at] << 24 | (uint32_t)png[at + 1] << 16 | (uint32_t)png[at + 2] << 8 | png[at + 3];
            if (!memcmp(png + at + 4, "IDAT", 4)) {
                uint32_t raw = cl - 6;
                ip[n] = (uint8_t)(raw >> 24);
                ip[n + 1] = (uint8_t)(raw >> 16);
                ip[n + 2] = (uint8_t)(raw >> 8);
                ip[n + 3] = (uint8_t)raw;
                memcpy(ip + n + 4, "IDAT", 4);
                memcpy(ip + n + 8, png + at + 10, raw);
                n += 12 + raw;
            } else {
                memcpy(ip + n, png + at, 12 + cl);
                n += 12 + cl;
            }
            at += 12 + cl;
        }
        CHECK(bgimage_check(ip, n, n, &w, &h, &why) == 0 && w == 97 && h == 61);
        size_t c = bgimage_decode_cost(ip, n, n);
        CHECK(bgimage_decode(ip, n, &p, &why) == 0 && p.px && !memcmp(p.px, g.px, 97 * 61));
        CHECK(bgimage_last_decode_peak() <= c);
        bgpicture_free(&p);
        free(png);
        free(ip);
        bgpicture_free(&g);
        uint8_t *jpg = slurp("tests/fixtures/pictures/baseline.jpg", &len), *pad = calloc(1, len + 8);
        size_t app0 = 4 + ((size_t)jpg[4] << 8 | jpg[5]);  // after SOI and APP0 (JFIF)
        memcpy(pad, jpg, app0);
        memset(pad + app0, 0, 3);
        memcpy(pad + app0 + 3, jpg + app0, len - app0);
        BgPicture a = {0};
        CHECK(jpg[2] == 0xFF && jpg[3] == 0xE0 && bgimage_check(pad, len + 3, len + 3, &w, &h, &why) == 0 && w == 900);
        CHECK(bgimage_decode(jpg, len, &a, &why) == 0 && bgimage_decode(pad, len + 3, &p, &why) == 0);
        CHECK(a.px && p.px && a.ch == p.ch && !memcmp(a.px, p.px, (size_t)a.w * (size_t)a.h * (size_t)a.ch));
        bgpicture_free(&a);
        bgpicture_free(&p);
        free(jpg);
        free(pad);
    }
    // A colour page picture as the store writes them (8 bits a channel), and a gray one.
    for (int ch = 1; ch <= 3; ch += 2) {
        BgPicture big = noise(931, 1240, ch);
        char path[700];
        snprintf(path, sizeof path, "%s/budget-%d.png", root, ch);
        put_png(path, &big);
        uint8_t *d = slurp(path, &len);
        size_t c = d ? bgimage_decode_cost(d, len, len) : 0;
        CHECK(d && bgimage_decode(d, len, &p, &why) == 0 && bgimage_last_decode_peak() <= c);
        CHECK(p.px && !memcmp(p.px, big.px, (size_t)931 * 1240 * (size_t)ch));
        bgpicture_free(&p);
        bgpicture_free(&big);
        free(d);
    }
}

// What the UI thread decodes is a notebook's own picture: a file there that is no PNG, or bigger
// than a page picture can be, draws white (the editor is told), and is never decoded.
TEST(ui_picture_limits) {
    NotebookInfo nb;
    CHECK(store_create("Odd", "", PAPER_BLANK, &nb) == 0);
    char dir[700], file[800];
    CHECK(bgimage_dir(nb.id, dir, sizeof dir) == 0);
    Page *p = page_new(PW, PH, PAPER_BLANK);
    strlcpy_q(p->nb, nb.id, sizeof p->nb);
    Canvas c;
    canvas_init(&c, PW, PH);
    // A JPEG under a picture's name.
    size_t len;
    uint8_t *jpg = slurp("tests/fixtures/pictures/baseline.jpg", &len);
    bgimage_file(dir, 0x0000B1u, file, sizeof file);
    put_file(file, jpg, len);
    free(jpg);
    // A PNG of more than 32 MB (a picture, then an ancillary chunk of zeros).
    BgPicture g = solid(PW, PH, 1, 50);
    bgimage_file(dir, 0x0000B2u, file, sizeof file);
    padded_png(file, &g, (size_t)33 << 20);
    // One that takes more than 96 MB to decode (a 6000 x 5000 RGB header): fine for an import's
    // child, not here.
    uint8_t hdr[64];
    size_t hl = png_header_of(hdr, 6000, 5000, 8, 2);
    int w, h;
    const char *why = "";
    CHECK(bgimage_check(hdr, hl, hl, &w, &h, &why) == 0);
    bgimage_file(dir, 0x0000B3u, file, sizeof file);
    put_file(file, hdr, hl);
    const uint32_t ids[3] = {0x0000B1u, 0x0000B2u, 0x0000B3u};
    static const char *const whys[3] = {"is not a PNG", "is bigger than a page picture can be", "is too big to decode"};
    for (int i = 0; i < 3; i++) {
        p->paper = paper_image(PAPER_ROLE_TEMPLATE, ids[i]);
        canvas_fill(&c, (Rect){0, 0, PW, PH}, 0);
        bgimage_take_missing();
        log_capture();
        page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
        char *log = log_captured();
        CHECK(c.px[0] == 255 && c.px[(PH / 2) * PW + PW / 2] == 255 && bgimage_take_missing());
        CHECK(log && strstr(log, whys[i]));  // (a file of 33 MB is not even read)
        free(log);
    }
    // The same picture as a PNG of its own size draws.
    bgimage_file(dir, 0x0000B4u, file, sizeof file);
    put_png(file, &g);
    p->paper = paper_image(PAPER_ROLE_TEMPLATE, 0x0000B4u);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(c.px[0] == 50 && !bgimage_take_missing());
    canvas_free(&c);
    page_free(p);
    bgpicture_free(&g);
}

// Rows [y0, y1) all v, columns [x0, x1).
static bool block_is(const BgPicture *p, int x0, int y0, int x1, int y1, uint8_t v) {
    for (int y = y0; y < y1; y++)
        for (int x = x0; x < x1; x++)
            for (int c = 0; c < p->ch; c++)
                if (at(p, x, y, c) != v) return false;
    return true;
}

TEST(normalization) {
    // Exact 3:4 (reMarkable and Supernote templates are 1404x1872): it fills the page.
    BgPicture s = solid(1404, 1872, 1, 0), f;
    CHECK(bgimage_normalize(&s, 1860, 2480, &f) == 0 && block_is(&f, 0, 0, 1860, 2480, 0));
    bgpicture_free(&s);
    bgpicture_free(&f);
    // Letter (8.5 x 11): as wide as the page, white bands above and below (2407 rows from row 36).
    s = solid(850, 1100, 1, 0);
    CHECK(bgimage_normalize(&s, 1860, 2480, &f) == 0);
    CHECK(block_is(&f, 0, 0, 1860, 36, 255) && block_is(&f, 0, 36, 1860, 36 + 2407, 0) &&
          block_is(&f, 0, 36 + 2407, 1860, 2480, 255));
    bgpicture_free(&s);
    bgpicture_free(&f);
    // A4: as tall as the page, white bands left and right (1754 columns from column 53).
    s = solid(840, 1188, 3, 0);
    CHECK(bgimage_normalize(&s, 1860, 2480, &f) == 0 && f.ch == 3);
    CHECK(block_is(&f, 0, 0, 53, 2480, 255) && block_is(&f, 53, 0, 53 + 1754, 2480, 0) &&
          block_is(&f, 53 + 1754, 0, 1860, 2480, 255));
    bgpicture_free(&s);
    bgpicture_free(&f);
    // A picture of the page's own size is kept as it is.
    s = noise(PW, PH, 1);
    CHECK(bgimage_normalize(&s, PW, PH, &f) == 0 && !memcmp(s.px, f.px, (size_t)PW * PH));
    bgpicture_free(&f);
    // The same picture gives the same bytes every time (and so the same key).
    BgPicture f2;
    CHECK(bgimage_normalize(&s, 1860, 2480, &f) == 0 && bgimage_normalize(&s, 1860, 2480, &f2) == 0);
    CHECK(!memcmp(f.px, f2.px, (size_t)1860 * 2480) && bgimage_key(&f) == bgimage_key(&f2));
    f2.px[999] ^= 1;
    CHECK(bgimage_key(&f) != bgimage_key(&f2));
    bgpicture_free(&s);
    bgpicture_free(&f);
    bgpicture_free(&f2);
    // Halving averages 2x2 blocks: blocks of one value stay that value exactly.
    s = picture(8, 8, 1);
    for (int y = 0; y < 8; y++)
        for (int x = 0; x < 8; x++) s.px[y * 8 + x] = (uint8_t)(((x / 2) + (y / 2) * 4) * 16);
    CHECK(bgimage_normalize(&s, 4, 4, &f) == 0);
    bool exact = true;
    for (int y = 0; y < 4; y++)
        for (int x = 0; x < 4; x++) exact = exact && f.px[y * 4 + x] == (uint8_t)((x + y * 4) * 16);
    CHECK(exact);
    bgpicture_free(&s);
    bgpicture_free(&f);
    // Scaled up (1404 -> 1860), a 1 px line stays a line, with its darkness: about 1.32 px of black,
    // spread over the two page pixels it covers.
    s = solid(1404, 1872, 1, 255);
    for (int y = 0; y < 1872; y++) s.px[(size_t)y * 1404 + 700] = 0;
    CHECK(bgimage_normalize(&s, 1860, 2480, &f) == 0);
    int darkest = 255, ink = 0;
    for (int x = 920; x < 935; x++) {
        int v = f.px[(size_t)1000 * 1860 + (size_t)x];
        darkest = v < darkest ? v : darkest;
        ink += 255 - v;
    }
    CHECK(darkest < 100 && abs(ink - 255 * 1860 / 1404) <= 3);
    bgpicture_free(&s);
    bgpicture_free(&f);
}

// ---- templates ----

static void template_file(const char *name, const BgPicture *p, char *path, size_t cap) {
    snprintf(path, cap, "%s/templates/%s", root, name);
    put_png(path, p);
}

static int index_entries(const NotebookInfo *nb) {
    char dir[700];
    bgimage_dir(nb->id, dir, sizeof dir);
    BgIndex ix;
    if (bgindex_load(dir, &ix) != 0) return -1;
    int n = ix.n;
    bgindex_free(&ix);
    return n;
}

TEST(templates_same_file_same_id) {
    NotebookInfo nb;
    CHECK(store_create("Planner", "", PAPER_MEDIUM, &nb) == 0);
    BgPicture lines = solid(450, 600, 1, 255);
    for (int y = 40; y < 600; y += 30) memset(lines.px + (size_t)y * 450, 120, 450);
    char path[700];
    template_file("Lines.png", &lines, path, sizeof path);
    PaperKind k1 = PAPER_BLANK, k2 = PAPER_BLANK;
    const char *why = "";
    CHECK(template_install(&nb, path, &k1, &why) == 0);
    CHECK(paper_is_image(k1) && !paper_is_document(k1));
    // Its id: the low 24 bits of the file's key for this page size (no other picture here).
    size_t len;
    uint8_t *d = slurp(path, &len);
    uint32_t want = (uint32_t)bgimage_source_key(d, len, PW, PH) & 0xFFFFFFu;
    free(d);
    CHECK(paper_image_id(k1) == (want ? want : 1u));
    CHECK(template_install(&nb, path, &k2, &why) == 0 && k2 == k1 && index_entries(&nb) == 1);
    int kind = -1;
    drain_jobs(&kind);
    CHECK(kind == BG_TEMPLATE);
    // The picture is there now, made for the notebook's page size, and its page draws it.
    char dir[700], file[800];
    bgimage_dir(nb.id, dir, sizeof dir);
    bgimage_file(dir, paper_image_id(k1), file, sizeof file);
    d = slurp(file, &len);
    BgPicture made = {0};
    CHECK(d && bgimage_decode(d, len, &made, &why) == 0 && made.w == PW && made.h == PH && made.ch == 1);
    BgPicture want_pic;
    CHECK(bgimage_normalize(&lines, PW, PH, &want_pic) == 0 && made.px && !memcmp(made.px, want_pic.px, (size_t)PW * PH));
    free(d);
    bgimage_retry_missing();
    Page *p = page_new(PW, PH, k1);
    strlcpy_q(p->nb, nb.id, sizeof p->nb);
    Canvas c;
    canvas_init(&c, PW, PH);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(!memcmp(c.px, want_pic.px, (size_t)PW * PH));
    // Another file: another paper.
    BgPicture dots = solid(300, 400, 1, 255);
    for (int y = 10; y < 400; y += 20)
        for (int x = 10; x < 300; x += 20) dots.px[(size_t)y * 300 + (size_t)x] = 0;
    char path2[700];
    template_file("Dots.png", &dots, path2, sizeof path2);
    PaperKind k3 = PAPER_BLANK;
    CHECK(template_install(&nb, path2, &k3, &why) == 0 && paper_is_image(k3) && k3 != k1 && index_entries(&nb) == 2);
    drain_jobs(NULL);
    // Picking it again restores a picture that went missing: same id, same bytes.
    unlink(file);
    PaperKind k4 = PAPER_BLANK;
    CHECK(template_install(&nb, path, &k4, &why) == 0 && k4 == k1);
    drain_jobs(NULL);
    d = slurp(file, &len);
    BgPicture again = {0};
    CHECK(d && bgimage_decode(d, len, &again, &why) == 0 && again.px && !memcmp(again.px, want_pic.px, (size_t)PW * PH));
    free(d);
    // Not a picture: refused at once, nothing queued.
    char junk[700];
    snprintf(junk, sizeof junk, "%s/templates/junk.png", root);
    put_file(junk, "junk", 4);
    CHECK(template_install(&nb, junk, &k4, &why) != 0 && !bg_busy());
    unlink(junk);
    canvas_free(&c);
    page_free(p);
    bgpicture_free(&lines);
    bgpicture_free(&dots);
    bgpicture_free(&made);
    bgpicture_free(&want_pic);
    bgpicture_free(&again);
}

// A file's hash read a piece at a time (as the UI may, for a small template) and from the whole
// file in memory (as the children do): the same, whatever the file's length.
TEST(source_key_streamed) {
    const size_t sizes[] = {0, 1, 7, 8, 9, 65535, 65536, 65537, 131072, 200003};
    uint8_t *d = malloc(200003);
    for (size_t i = 0; i < 200003; i++) d[i] = rnd8();
    char path[700];
    snprintf(path, sizeof path, "%s/keyed.bin", root);
    for (size_t i = 0; i < sizeof sizes / sizeof sizes[0]; i++) {
        put_file(path, d, sizes[i]);
        uint64_t k = 0;
        size_t len = 99;
        CHECK(bgimage_content_hash_file(path, &k, &len) == 0 && len == sizes[i]);
        CHECK(k == bgimage_content_hash(d, sizes[i]));
        uint64_t key = bgimage_template_key(k, len, PW, PH);
        CHECK(key == bgimage_source_key(d, sizes[i], PW, PH));
        CHECK(key != bgimage_template_key(k, len, PH, PW));  // made for another page: another key
        if (sizes[i] > 1) CHECK(k != bgimage_content_hash(d + 1, sizes[i] - 1));
    }
    CHECK(bgimage_content_hash_file("/nonexistent/x.png", &(uint64_t){0}, &(size_t){0}) != 0);
    free(d);
}

TEST(templates_collision_steps) {
    NotebookInfo nb, nb2;
    CHECK(store_create("Collide", "", PAPER_MEDIUM, &nb) == 0 && store_create("Orphan", "", PAPER_MEDIUM, &nb2) == 0);
    BgPicture g = noise(120, 160, 1);
    char path[700];
    template_file("Noise.png", &g, path, sizeof path);
    size_t len;
    uint8_t *d = slurp(path, &len);
    uint64_t key = bgimage_source_key(d, len, PW, PH);
    free(d);
    uint32_t id0 = (uint32_t)key & 0xFFFFFFu;
    if (!id0) id0 = 1;
    uint32_t next = (id0 + 1) & 0xFFFFFFu;
    if (!next) next = 1;
    // Its id is taken by another key: the next one.
    char dir[700], text[96];
    bgimage_dir(nb.id, dir, sizeof dir);
    snprintf(text, sizeof text, "# taken\n%06x=%016llx\n", (unsigned)id0, (unsigned long long)(key ^ 1u));
    char ipath[800];
    snprintf(ipath, sizeof ipath, "%s/index.ini", dir);
    put_file(ipath, text, strlen(text));
    PaperKind k = PAPER_BLANK;
    const char *why;
    CHECK(template_install(&nb, path, &k, &why) == 0 && paper_image_id(k) == next);
    CHECK(index_entries(&nb) == 2);
    // A picture file with no entry takes its id too, whatever it holds.
    BgPicture other = solid(PW, PH, 1, 9);
    put_picture(&nb2, id0, &other);
    PaperKind k2 = PAPER_BLANK;
    CHECK(template_install(&nb2, path, &k2, &why) == 0 && paper_image_id(k2) == next);
    drain_jobs(NULL);
    // The squatter is never touched.
    char d2[700], f2[800];
    bgimage_dir(nb2.id, d2, sizeof d2);
    bgimage_file(d2, id0, f2, sizeof f2);
    d = slurp(f2, &len);
    BgPicture back = {0};
    CHECK(d && bgimage_decode(d, len, &back, &why) == 0 && back.px && back.px[0] == 9);
    free(d);
    bgpicture_free(&back);
    // The children made each picture under the id its notebook gave it, the next one, and nothing
    // under the id that was taken; a page on it draws the template.
    BgPicture want;
    CHECK(bgimage_normalize(&g, PW, PH, &want) == 0);
    const NotebookInfo *both[2] = {&nb, &nb2};
    const PaperKind kinds[2] = {k, k2};
    bgimage_retry_missing();
    for (int i = 0; i < 2; i++) {
        char bd[700], fn[800], f0[800];
        bgimage_dir(both[i]->id, bd, sizeof bd);
        bgimage_file(bd, next, fn, sizeof fn);
        bgimage_file(bd, id0, f0, sizeof f0);
        CHECK(exists(fn));
        if (i == 0) CHECK(!exists(f0));
        Page *pg = page_new(PW, PH, kinds[i]);
        strlcpy_q(pg->nb, both[i]->id, sizeof pg->nb);
        Canvas c;
        canvas_init(&c, PW, PH);
        page_render(&c, pg, (Rect){0, 0, PW, PH}, DPI);
        CHECK(!memcmp(c.px, want.px, (size_t)PW * PH));
        canvas_free(&c);
        page_free(pg);
    }
    bgpicture_free(&want);
    bgpicture_free(&g);
    bgpicture_free(&other);
    // bgindex_claim on its own: an existing key keeps its id, new keys step past taken ones.
    BgIndex ix = {0};
    uint32_t a, b, c;
    bool added;
    CHECK(bgindex_claim(&ix, dir, 0x0000000000000000ull, &a, &added) == 0 && a == 1 && added);  // never 0
    CHECK(bgindex_claim(&ix, dir, 0x1000000000000001ull, &b, &added) == 0 && b == 2 && added);
    CHECK(bgindex_claim(&ix, dir, 0x0000000000000000ull, &c, &added) == 0 && c == 1 && !added);
    bgindex_free(&ix);
}

// One id in two notebooks names two pictures: the cache holds each by its notebook too, so
// pages drawn one after the other from either notebook each get their own.
TEST(same_id_two_notebooks) {
    NotebookInfo a, b;
    CHECK(store_create("Left", "", PAPER_BLANK, &a) == 0 && store_create("Right", "", PAPER_BLANK, &b) == 0);
    BgPicture pa = noise(PW, PH, 1), pb = noise(PW, PH, 1);
    put_picture(&a, 0x00D00Du, &pa);
    put_picture(&b, 0x00D00Du, &pb);
    Page *qa = page_new(PW, PH, paper_image(PAPER_ROLE_DOCUMENT, 0x00D00Du));
    Page *qb = page_new(PW, PH, qa->paper);
    strlcpy_q(qa->nb, a.id, sizeof qa->nb);
    strlcpy_q(qb->nb, b.id, sizeof qb->nb);
    Canvas c;
    canvas_init(&c, PW, PH);
    bool right = true;
    for (int i = 0; i < 6; i++) {
        const Page *q = i % 2 ? qb : qa;
        const BgPicture *want = i % 2 ? &pb : &pa;
        page_render(&c, q, (Rect){0, 0, PW, PH}, DPI);
        right = right && !memcmp(c.px, want->px, (size_t)PW * PH);
        Rect r = {37, 51, 211, 97};  // a region alone, from the cache
        canvas_fill(&c, (Rect){0, 0, PW, PH}, 0);
        page_render(&c, q, r, DPI);
        for (int y = r.y; y < r.y + r.h && right; y++)
            right = !memcmp(&c.px[y * c.stride + r.x], &want->px[(size_t)y * PW + (size_t)r.x], (size_t)r.w);
    }
    CHECK(right);
    // Forgetting one notebook's pictures leaves the other's.
    bgimage_forget(a.id);
    page_render(&c, qb, (Rect){0, 0, PW, PH}, DPI);
    CHECK(!memcmp(c.px, pb.px, (size_t)PW * PH));
    page_render(&c, qa, (Rect){0, 0, PW, PH}, DPI);
    CHECK(!memcmp(c.px, pa.px, (size_t)PW * PH));
    CHECK(paper_take_image_blank_count() == 0);
    canvas_free(&c);
    page_free(qa);
    page_free(qb);
    bgpicture_free(&pa);
    bgpicture_free(&pb);
}

TEST(template_previews) {
    templates_init();
    char readme[700];
    snprintf(readme, sizeof readme, "%s/templates/README.txt", root);
    CHECK(exists(readme));
    snprintf(readme, sizeof readme, "%s/import/README.txt", root);
    CHECK(exists(readme));
    char path[700];
    BgPicture a = solid(300, 400, 1, 200), b = solid(300, 400, 3, 100);
    for (int i = 0; i < 300 * 400; i++) b.px[3 * i] = 250;
    template_file("t-10.png", &a, path, sizeof path);
    template_file("t-2.png", &b, path, sizeof path);
    snprintf(path, sizeof path, "%s/templates/t-3.jpg", root);
    put_file(path, "not a jpeg", 10);
    Template *t;
    int n;
    CHECK(template_scan(&t, &n, DPI) == 0);
    // Every picture file by name, in natural order, Lines and Dots and Noise from the tests above too.
    int i2 = -1, i3 = -1, i10 = -1;
    for (int i = 0; i < n; i++) {
        if (!strcmp(t[i].name, "t-2")) i2 = i;
        if (!strcmp(t[i].name, "t-3")) i3 = i;
        if (!strcmp(t[i].name, "t-10")) i10 = i;
        CHECK(strcmp(t[i].name, "README"));
    }
    CHECK(i2 >= 0 && i2 < i3 && i3 < i10);
    if (i10 >= 0) CHECK(!t[i10].ready && !t[i10].bad && !template_thumb(&t[i10]));
    template_list_free(t, n);
    drain_jobs(NULL);  // the previews job
    CHECK(template_scan(&t, &n, DPI) == 0);
    for (int i = 0; i < n; i++) {
        CHECK(!t[i].thumb.px);  // (none read by the scan)
        const Canvas *pv = template_thumb(&t[i]);
        if (!strcmp(t[i].name, "t-3")) CHECK(t[i].bad && !pv);
        if (!strcmp(t[i].name, "t-10")) CHECK(pv && pv->w == PW / 6 && pv->px[0] == 200 && template_thumb(&t[i]) == pv);
        if (!strcmp(t[i].name, "t-2")) CHECK(pv && !t[i].bad);
    }
    template_list_free(t, n);
    CHECK(!bg_busy());  // all made: nothing queued again
    bgpicture_free(&a);
    bgpicture_free(&b);
}

// A template's key without reading it whole on the UI thread: a big template is taken once the
// previews job has seen it (it keeps the hash of the file's bytes beside its preview), and the
// picture the BG_TEMPLATE child makes from the bytes is the one that key names.
TEST(big_template_uses_its_preview_key) {
    NotebookInfo nb;
    CHECK(store_create("Big", "", PAPER_BLANK, &nb) == 0);
    BgPicture g = solid(300, 400, 1, 255);
    for (int y = 0; y < 400; y += 25) memset(g.px + (size_t)y * 300, 30, 300);
    char path[700];
    snprintf(path, sizeof path, "%s/templates/Big lines.png", root);
    padded_png(path, &g, (size_t)5 << 20);  // past the 4 MB read on the UI thread
    struct stat st;
    CHECK(stat(path, &st) == 0 && st.st_size > (5 << 20));
    PaperKind k = PAPER_BLANK;
    const char *why = "";
    CHECK(template_install(&nb, path, &k, &why) != 0 && strstr(why, "preview") && !bg_busy());
    Template *t;
    int n;
    CHECK(template_scan(&t, &n, DPI) == 0);  // queues the previews job
    template_list_free(t, n);
    drain_jobs(NULL);
    CHECK(template_install(&nb, path, &k, &why) == 0 && paper_is_image(k));
    size_t len;
    uint8_t *d = slurp(path, &len);
    CHECK(d && paper_image_id(k) == ((uint32_t)bgimage_source_key(d, len, PW, PH) & 0xFFFFFFu));
    free(d);
    int kind = -1;
    drain_jobs(&kind);
    CHECK(kind == BG_TEMPLATE);
    bgimage_retry_missing();
    BgPicture want;
    CHECK(bgimage_normalize(&g, PW, PH, &want) == 0);
    Page *p = page_new(PW, PH, k);
    strlcpy_q(p->nb, nb.id, sizeof p->nb);
    Canvas c;
    canvas_init(&c, PW, PH);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    CHECK(!memcmp(c.px, want.px, (size_t)PW * PH));
    canvas_free(&c);
    page_free(p);
    bgpicture_free(&want);
    bgpicture_free(&g);
    unlink(path);
}

// ---- pages ----

TEST(document_role_not_inherited) {
    NotebookInfo nb;
    CHECK(store_create("Score", "", PAPER_MUSIC, &nb) == 0);
    PaperKind doc = paper_image(PAPER_ROLE_DOCUMENT, 0x00BEEFu), tpl = paper_image(PAPER_ROLE_TEMPLATE, 0x00CAFEu);
    CHECK(store_new_page_paper(&nb, doc) == PAPER_MUSIC);
    CHECK(store_new_page_paper(&nb, tpl) == tpl);
    CHECK(store_new_page_paper(&nb, PAPER_GRID) == PAPER_GRID);
    CHECK(store_add_page(&nb, 1, doc) == 0 && store_add_page(&nb, 2, tpl) == 0);
    Page *p = NULL;
    CHECK(store_load_page(&nb, 1, PW, PH, &p) == 0 && p && p->paper == PAPER_MUSIC);
    page_free(p);
    CHECK(store_load_page(&nb, 2, PW, PH, &p) == 0 && p && p->paper == tpl && !strcmp(p->nb, nb.id));
    page_free(p);
    // The added template page is in bg/pages.ini, the music one is not.
    char side[700], want[64];
    snprintf(side, sizeof side, "%s/notebooks/%s/bg/pages.ini", root, nb.id);
    size_t len;
    char *txt = (char *)slurp(side, &len);
    snprintf(want, sizeof want, "p%u=%u\n", nb.page_ids[2], (unsigned)tpl);
    CHECK(txt && strstr(txt, want));
    snprintf(want, sizeof want, "p%u=", nb.page_ids[1]);
    CHECK(txt && !strstr(txt, want));
    free(txt);
    // A notebook whose own paper is a document page's: blank.
    nb.paper = doc;
    CHECK(store_new_page_paper(&nb, doc) == PAPER_BLANK);
}

// The page file as 1.0.0-beta saves it after opening it (a written page, a page delete).
static void beta_resaves(const NotebookInfo *nb, uint32_t pid) {
    char path[700];
    snprintf(path, sizeof path, "%s/notebooks/%s/p%u.qpg", root, nb->id, pid);
    size_t len;
    uint8_t *d = slurp(path, &len), *b = NULL;
    size_t bl = 0;
    CHECK(d && beta_resave(d, len, &b, &bl) == 0);
    if (b) put_file(path, b, bl);
    free(d);
    free(b);
}

static uint16_t file_version(const NotebookInfo *nb, uint32_t pid, uint16_t *paper16) {
    char path[700];
    snprintf(path, sizeof path, "%s/notebooks/%s/p%u.qpg", root, nb->id, pid);
    size_t len;
    uint8_t *d = slurp(path, &len);
    uint16_t v = d && len >= 8 ? (uint16_t)(d[4] | d[5] << 8) : 0;
    if (paper16) *paper16 = d && len >= 8 ? (uint16_t)(d[6] | d[7] << 8) : 0;
    free(d);
    return v;
}

TEST(sidecar_restores_beta_resave) {
    NotebookInfo nb;
    CHECK(store_create("Downgrade", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_MEDIUM) == 0 && store_add_page(&nb, 2, PAPER_MEDIUM) == 0);
    BgPicture g = solid(PW, PH, 1, 77);
    put_picture(&nb, 0x0000C1u, &g);
    PaperKind k = paper_image(PAPER_ROLE_TEMPLATE, 0x0000C1u);
    Page *p = NULL;
    CHECK(store_load_page(&nb, 0, PW, PH, &p) == 0 && p);
    p->paper = k;
    page_add(p, line_stroke(TOOL_PENCIL, 300.f, 3u));
    CHECK(store_save_page(&nb, 0, p) == 0);
    page_free(p);
    char side[700];
    snprintf(side, sizeof side, "%s/notebooks/%s/bg/pages.ini", root, nb.id);
    size_t len;
    char *txt = (char *)slurp(side, &len);
    char want[64];
    snprintf(want, sizeof want, "p%u=%u\n", nb.page_ids[0], (unsigned)k);
    CHECK(txt && strstr(txt, want));
    free(txt);
    // 1.0.0-beta re-saves the page: version 1 Blank, the stroke kept.
    beta_resaves(&nb, nb.page_ids[0]);
    uint16_t p16;
    CHECK(file_version(&nb, nb.page_ids[0], &p16) == 1 && p16 == PAPER_BLANK);
    // Back here, the page has its picture again. Exports read it so, changing nothing on disk.
    store_take_restored_count();
    CHECK(store_read_page(&nb, 0, &p) == 0 && p && p->paper == k && !strcmp(p->nb, nb.id));
    page_free(p);
    Buf png = {0};
    PageRender info;
    CHECK(export_page_png_ex(&nb, 0, DPI, PAGE_LAYER_PAPER, 1, &info, &png) == 0 && info.paper == (uint32_t)k);
    BgPicture back = {0};
    const char *why;
    CHECK(bgimage_decode(png.data, png.len, &back, &why) == 0 && back.px && block_is(&back, 0, 0, PW, PH, 77));
    bgpicture_free(&back);
    buf_free(&png);
    CHECK(file_version(&nb, nb.page_ids[0], NULL) == 1 && store_take_restored_count() == 0);
    // The editor's way in writes it back with its picture (version 2), once, and counts it: the
    // editor then redraws the thumbnail and the PDF the older version drew blank.
    CHECK(store_load_page(&nb, 0, PW, PH, &p) == 0 && p && p->paper == k && p->n == 1);
    page_free(p);
    CHECK(file_version(&nb, nb.page_ids[0], &p16) == 2 && p16 == PAPER_BLANK && store_take_restored_count() == 1);
    CHECK(store_load_page(&nb, 0, PW, PH, &p) == 0 && p && p->paper == k && p->n == 1);
    page_free(p);
    CHECK(store_take_restored_count() == 0);
    // A list that does not read (no permission here; damage, a card error on the Kindle) is never
    // saved over: the entries it holds stay for when it reads again. (Root reads it anyway.)
    char before[256] = "";
    txt = (char *)slurp(side, &len);
    if (txt) strlcpy_q(before, txt, sizeof before);
    free(txt);
    if (geteuid() != 0 && chmod(side, 0) == 0) {
        CHECK(store_load_page(&nb, 2, PW, PH, &p) == 0 && p);
        p->paper = k;
        CHECK(store_save_page(&nb, 2, p) == 0);  // the page is saved; the list is left alone
        page_free(p);
        CHECK(chmod(side, 0644) == 0);
        txt = (char *)slurp(side, &len);
        CHECK(txt && !strcmp(txt, before));
        free(txt);
        CHECK(store_load_page(&nb, 2, PW, PH, &p) == 0 && p && p->paper == k);  // (its own file says so)
        p->paper = PAPER_MEDIUM;
        CHECK(store_save_page(&nb, 2, p) == 0);
        page_free(p);
    }
    // A page set to Blank here leaves the list: it stays Blank.
    CHECK(store_load_page(&nb, 0, PW, PH, &p) == 0 && p);
    p->paper = PAPER_BLANK;
    CHECK(store_save_page(&nb, 0, p) == 0);
    page_free(p);
    txt = (char *)slurp(side, &len);
    CHECK(txt && !strstr(txt, want));
    free(txt);
    CHECK(store_load_page(&nb, 0, PW, PH, &p) == 0 && p && p->paper == PAPER_BLANK);
    page_free(p);
    // A page 1.0.0-beta gave another paper (its picker) keeps it.
    CHECK(store_load_page(&nb, 1, PW, PH, &p) == 0 && p);
    p->paper = k;
    CHECK(store_save_page(&nb, 1, p) == 0);
    p->paper = PAPER_GRID;  // what 1.0.0-beta writes (version 1 Graph paper), the list left as it was
    uint8_t *d;
    size_t dl;
    CHECK(page_serialize(p, &d, &dl) == 0);
    char path[700];
    snprintf(path, sizeof path, "%s/notebooks/%s/p%u.qpg", root, nb.id, nb.page_ids[1]);
    put_file(path, d, dl);
    free(d);
    page_free(p);
    CHECK(store_load_page(&nb, 1, PW, PH, &p) == 0 && p && p->paper == PAPER_GRID);
    page_free(p);
    // A deleted page leaves the list.
    snprintf(want, sizeof want, "p%u=", nb.page_ids[1]);
    txt = (char *)slurp(side, &len);
    CHECK(txt && strstr(txt, want));
    free(txt);
    CHECK(store_delete_page(&nb, 1) == 0);
    txt = (char *)slurp(side, &len);
    CHECK(txt && !strstr(txt, want));
    free(txt);
    bgpicture_free(&g);
}

TEST(beta_reads_image_pages) {
    PaperKind kinds[2] = {paper_image(PAPER_ROLE_TEMPLATE, 0x00F00Du), paper_image(PAPER_ROLE_DOCUMENT, 0xFFFFFFu)};
    for (int i = 0; i < 2; i++) {
        Page *p = page_new(1860, 2480, kinds[i]);
        page_add(p, line_stroke(TOOL_BALLPOINT, 400.f, 9u));
        page_add(p, line_stroke(TOOL_ERASER, 400.f, erase_seed(TOOL_PENCIL, 4u)));
        uint8_t *d = NULL;
        size_t len = 0;
        CHECK(page_serialize(p, &d, &len) == 0 && d[4] == 2);
        BetaPage bp;
        CHECK(beta_page_deserialize(d, len, &bp) == 0);
        CHECK(bp.paper == BETA_PAPER_BLANK && bp.n == 2 && bp.strokes[0].n == 30 && bp.strokes[1].seed == p->strokes[1]->seed);
        beta_page_free(&bp);
        uint8_t *b2 = NULL;
        size_t l2 = 0;
        CHECK(beta_resave(d, len, &b2, &l2) == 0 && b2 && b2[4] == 1 && b2[6] == BETA_PAPER_BLANK);
        Page *q = NULL;
        CHECK(page_deserialize(b2, l2, &q) == 0 && q && q->paper == PAPER_BLANK && q->n == 2);  // (the store restores it)
        page_free(q);
        free(b2);
        free(d);
        page_free(p);
    }
}

// Erasing over a picture restores the picture, and compaction keeps the page as it looks.
TEST(erasing_over_a_picture) {
    NotebookInfo nb;
    CHECK(store_create("Erase", "", PAPER_BLANK, &nb) == 0);
    BgPicture n = noise(PW, PH, 1);
    put_picture(&nb, 0x0000E1u, &n);
    Page *p = page_new(PW, PH, paper_image(PAPER_ROLE_DOCUMENT, 0x0000E1u));
    strlcpy_q(p->nb, nb.id, sizeof p->nb);
    page_add(p, line_stroke(TOOL_MARKER, 300.f, 1u));
    ToolParams big;
    tool_params_default(TOOL_ERASER, &big);
    big.size_min_mm = big.size_max_mm = 12.f;
    Stroke *e = stroke_new(TOOL_ERASER, &big, erase_seed(TOOL_ERASER, 2u));
    for (int i = 0; i < 60; i++) {
        PenSample q = {20.f + (float)i * 10.f, 310.f, 1.f, 0.f, 0.f, (uint32_t)i};
        stroke_append(e, &q, DPI);
    }
    page_add(p, e);
    Canvas c, bare;
    canvas_init(&c, PW, PH);
    canvas_init(&bare, PW, PH);
    page_render(&c, p, (Rect){0, 0, PW, PH}, DPI);
    page_paper_render(&bare, p, (Rect){0, 0, PW, PH}, DPI);
    // Under the eraser's middle the picture shows again, not white.
    int restored = 0, white = 0;
    for (int x = 100; x < 500; x++) {
        restored += c.px[310 * PW + x] == bare.px[310 * PW + x];
        white += c.px[310 * PW + x] == 255 && bare.px[310 * PW + x] != 255;
    }
    CHECK(restored > 390 && white == 0);
    bool *drop = calloc((size_t)p->n, sizeof *drop);
    int nd = page_droppable(p, DPI, drop);
    CHECK(nd >= 0);
    Page *q = page_new(PW, PH, p->paper);
    strlcpy_q(q->nb, nb.id, sizeof q->nb);
    for (int i = 0; i < p->n; i++)
        if (!drop[i]) {
            Stroke *s = stroke_new(p->strokes[i]->tool, &p->strokes[i]->params, p->strokes[i]->seed);
            for (int k = 0; k < p->strokes[i]->n; k++) stroke_append(s, &p->strokes[i]->pts[k], DPI);
            page_add(q, s);
        }
    Canvas c2;
    canvas_init(&c2, PW, PH);
    page_render(&c2, q, (Rect){0, 0, PW, PH}, DPI);
    CHECK(!memcmp(c.px, c2.px, (size_t)PW * PH));
    CHECK(paper_take_image_blank_count() == 0);  // (compaction's paper too)
    free(drop);
    canvas_free(&c);
    canvas_free(&c2);
    canvas_free(&bare);
    page_free(p);
    page_free(q);
    bgpicture_free(&n);
}

// ---- imports ----

TEST(natural_order) {
    CHECK(natural_cmp("page-2", "page-10") < 0 && natural_cmp("page-10", "page-2") > 0);
    CHECK(natural_cmp("page-9.png", "page-10.png") < 0);
    CHECK(natural_cmp("Page-3", "page-4") < 0 && natural_cmp("a", "B") < 0);
    CHECK(natural_cmp("page-002", "page-3") < 0 && natural_cmp("page-02", "page-2") != 0);
    CHECK(natural_cmp("x", "x1") < 0 && natural_cmp("x1y2", "x1y10") < 0 && natural_cmp("same", "same") == 0);
    CHECK(natural_cmp("scan 1 of 12", "scan 12 of 12") < 0);
    CHECK(natural_cmp("100", "99") > 0 && natural_cmp("0", "00") != 0);
}

static void import_folder_of(const char *name, const char *const *files, const uint8_t *values, int n, char *src,
                             size_t cap) {
    snprintf(src, cap, "%s/import/%s", root, name);
    for (int i = 0; i < n; i++) {
        char path[900];
        snprintf(path, sizeof path, "%s/%s", src, files[i]);
        if (values[i] == 1) {
            put_file(path, "broken", 6);
            continue;
        }
        BgPicture p = solid(150, 200, 1, values[i]);
        put_png(path, &p);
        bgpicture_free(&p);
    }
}

// The mark import_scan leaves in a folder: the notebook it becomes.
static void mark(const char *src, const char *id) {
    char path[900], text[40];
    snprintf(path, sizeof path, "%s/.grimoire-import", src);
    snprintf(text, sizeof text, "%s\n", id);
    put_file(path, text, strlen(text));
}

static uint8_t page_value(const NotebookInfo *nb, int index) {
    Page *p = NULL;
    uint8_t v = 0;
    if (store_read_page(nb, index, &p) == 0 && p) {
        Canvas c;
        canvas_init(&c, p->w, p->h);
        page_render(&c, p, (Rect){0, 0, p->w, p->h}, DPI);
        v = c.px[(p->h / 2) * c.stride + p->w / 2];
        canvas_free(&c);
    }
    page_free(p);
    return v;
}

TEST(import_makes_a_notebook) {
    const char *files[] = {"page-10.png", "page-2.png", "page-1.png", "notes.txt", "page-3.PNG"};
    const uint8_t values[] = {90, 60, 30, 0, 70};
    char src[700];
    snprintf(src, sizeof src, "%s/import/Score", root);
    import_folder_of("Score", files, values, 3, src, sizeof src);
    char path[900];
    snprintf(path, sizeof path, "%s/notes.txt", src);
    put_file(path, "notes", 5);
    snprintf(path, sizeof path, "%s/page-3.PNG", src);
    BgPicture p3 = solid(150, 200, 3, 70);
    p3.px[0] = 200;  // one coloured pixel: kept in colour
    put_png(path, &p3);
    bgpicture_free(&p3);
    const char *id = "20261020-120000-abcd";
    mark(src, id);
    CHECK(bg_import_folder(src, DPI) == 0);
    NotebookInfo nb;
    CHECK(store_load_info(id, &nb) == 0);
    CHECK(!strcmp(nb.name, "Score") && !nb.folder[0] && nb.npages == 4 && nb.paper == PAPER_BLANK);
    CHECK(nb.page_w == PW && nb.page_h == PH && nb.next_page_id == 5);
    // Natural order: page-1, page-2, page-3, page-10.
    CHECK(page_value(&nb, 0) == 30 && page_value(&nb, 1) == 60 && page_value(&nb, 3) == 90);
    uint8_t v3 = page_value(&nb, 2);
    CHECK(v3 == 70);
    for (int i = 0; i < nb.npages; i++) {
        Page *p = NULL;
        CHECK(store_read_page(&nb, i, &p) == 0 && p && paper_is_document(p->paper));
        page_free(p);
    }
    char bg[700];
    bgimage_dir(nb.id, bg, sizeof bg);
    CHECK(count_prefix(bg, "") >= 4 + 2 + 2);  // 4 pictures, index.ini, pages.ini (and . ..)
    CHECK(index_entries(&nb) == 4);
    snprintf(path, sizeof path, "%s/notebooks/%s/thumb.pgm", root, id);
    CHECK(exists(path));
    // The folder moved to import/done, its mark gone; nothing half-built is left.
    char done[700];
    snprintf(done, sizeof done, "%s/import/done", root);
    CHECK(!exists(src) && count_prefix(done, "Score-") == 1);
    snprintf(path, sizeof path, "%s/notebooks", root);
    CHECK(count_prefix(path, ".import-") == 0);
    // A page added after the last (a document page) is blank: the notebook's paper.
    CHECK(store_add_page(&nb, nb.npages, PAPER_BLANK) == 0);
    Page *p = NULL;
    CHECK(store_load_page(&nb, 4, PW, PH, &p) == 0 && p && p->paper == PAPER_BLANK);
    page_free(p);
    // The raw zip carries the pictures.
    Buf b = {0};
    ZipWriter z;
    zip_begin(&z, &b);
    CHECK(export_notebook_zip(&nb, DPI, &z, "") == 0 && zip_end(&z) == 0);
    char zp[700], cmd[1600];
    snprintf(zp, sizeof zp, "%s/score.zip", root);
    put_file(zp, b.data, b.len);
    buf_free(&b);
    Page *first = NULL;
    store_read_page(&nb, 0, &first);
    snprintf(cmd, sizeof cmd,
             "unzip -tq '%s' >/dev/null && unzip -l '%s' | grep -q 'Score/raw/bg/%06x.png' && "
             "unzip -l '%s' | grep -q 'Score/raw/bg/index.ini' && unzip -l '%s' | grep -q 'Score/raw/bg/pages.ini' && "
             "unzip -l '%s' | grep -q 'Score/page-004.png'",
             zp, zp, first ? (unsigned)paper_image_id(first->paper) : 0u, zp, zp, zp);
    page_free(first);
    CHECK(system(cmd) == 0);
}

TEST(import_same_picture_twice) {
    const char *files[] = {"a.png", "b.png"};
    const uint8_t values[] = {120, 120};
    char src[700];
    import_folder_of("Twins", files, values, 2, src, sizeof src);
    const char *id = "20261020-120001-beef";
    mark(src, id);
    CHECK(bg_import_folder(src, DPI) == 0);
    NotebookInfo nb;
    CHECK(store_load_info(id, &nb) == 0 && nb.npages == 2 && index_entries(&nb) == 1);
    Page *a = NULL, *b = NULL;
    CHECK(store_read_page(&nb, 0, &a) == 0 && store_read_page(&nb, 1, &b) == 0 && a && b && a->paper == b->paper);
    page_free(a);
    page_free(b);
}

// A folder name longer than a notebook name holds (95 bytes) is cut before a character, never
// inside one.
TEST(import_long_name) {
    char name[160] = "";
    for (int i = 0; i < 50; i++) strcat(name, "\xc3\xa9");  // 50 e-acute: 100 bytes
    const char *files[] = {"p.png"};
    const uint8_t values[] = {80};
    char src[700];
    import_folder_of(name, files, values, 1, src, sizeof src);
    const char *id = "20261020-120003-cafe";
    mark(src, id);
    CHECK(bg_import_folder(src, DPI) == 0);
    NotebookInfo nb;
    CHECK(store_load_info(id, &nb) == 0 && strlen(nb.name) == 94 && !strncmp(nb.name, name, 94));
}

TEST(import_failure_leaves_nothing) {
    const char *files[] = {"page-1.png", "page-2.png", "page-3.png"};
    const uint8_t values[] = {50, 1, 60};  // page 2 is damaged
    char src[700];
    import_folder_of("Broken", files, values, 3, src, sizeof src);
    const char *id = "20261020-120002-dead";
    mark(src, id);
    NotebookInfo *before;
    int nbefore;
    CHECK(store_list(&before, &nbefore) == 0);
    free(before);
    log_capture();
    CHECK(bg_import_folder(src, DPI) != 0);
    // Refused from its headers, before any page was made ("N page(s), for WxH pages" starts that).
    char *log = log_captured();
    CHECK(log && strstr(log, "page 2 of 3: not a PNG") && !strstr(log, "page(s), for"));
    free(log);
    NotebookInfo nb, *after;
    int nafter;
    CHECK(store_load_info(id, &nb) != 0);
    CHECK(store_list(&after, &nafter) == 0 && nafter == nbefore);
    free(after);
    char path[900];
    snprintf(path, sizeof path, "%s/notebooks", root);
    CHECK(count_prefix(path, ".import-") == 0);
    for (int i = 0; i < 3; i++) {
        snprintf(path, sizeof path, "%s/%s", src, files[i]);
        CHECK(exists(path));
    }
    // The folder is as it was (its mark gone), and the refusal is remembered (import_scan).
    snprintf(path, sizeof path, "%s/.grimoire-import", src);
    CHECK(!exists(path));
    snprintf(path, sizeof path, "%s/import/.attempts", root);
    char *txt = (char *)slurp(path, &(size_t){0});
    CHECK(txt && strstr(txt, " 99\n"));
    free(txt);
    // Remove the damage: the next start imports it.
    snprintf(path, sizeof path, "%s/page-2.png", src);
    unlink(path);
}

// Runs the imports import_scan queued, as app.c does: the results import_finished gives, in order.
static int finish_imports(int *results, int max, int *pages) {
    int got = 0;
    uint32_t until = now_ms() + 90000;
    for (bool last = false; !last;) {
        last = !bg_busy() || (int32_t)(now_ms() - until) >= 0;
        int kind = -1, r = bg_poll_kind(&kind);
        if (kind == BG_IMPORT_FOLDER && got < max) {
            results[got] = import_finished(r > 0, &pages[got]);
            got++;
        }
        if (!last) usleep(5000);
    }
    return got;
}

static int notebooks_named(const char *name) {
    NotebookInfo *all;
    int n, count = 0;
    if (store_list(&all, &n) != 0) return -1;
    for (int i = 0; i < n; i++) count += !strcmp(all[i].name, name);
    free(all);
    return count;
}

// A page whose header comes after a lot of metadata (an ICC profile, text) is still a page: the
// headers are checked from each file's first 64 KB, and from the whole file when that is not enough.
TEST(import_big_header) {
    char src[700], path[900];
    snprintf(src, sizeof src, "%s/import/Annotated", root);
    BgPicture g = solid(150, 200, 1, 110);
    snprintf(path, sizeof path, "%s/1.png", src);
    padded_png(path, &g, 300u << 10);
    snprintf(path, sizeof path, "%s/2.png", src);
    put_png(path, &g);
    bgpicture_free(&g);
    int res[2] = {0}, pages[2] = {0};
    CHECK(import_scan(DPI) == 1 && finish_imports(res, 2, pages) == 1 && res[0] == 1 && pages[0] == 2);
    CHECK(notebooks_named("Annotated") == 1 && !exists(src));
}

// A folder whose page is no picture fails once (one toast), then waits for its pictures to change,
// however many starts there are; changed, it is imported.
TEST(import_refused_waits) {
    const char *files[] = {"page-1.png", "page-2.png"};
    const uint8_t values[] = {40, 1};  // page 2 is text
    char src[700], path[900];
    import_folder_of("Waits", files, values, 2, src, sizeof src);
    int res[4] = {0}, pages[4] = {0};
    CHECK(import_scan(DPI) == 1 && finish_imports(res, 4, pages) == 1 && res[0] == -1);
    snprintf(path, sizeof path, "%s/.grimoire-import", src);
    CHECK(!exists(path));
    for (int start = 0; start < 3; start++) CHECK(import_scan(DPI) == 0 && !bg_busy());
    CHECK(notebooks_named("Waits") == 0);
    // The page replaced by a picture: taken at the next start.
    snprintf(path, sizeof path, "%s/page-2.png", src);
    BgPicture g = solid(150, 200, 1, 90);
    unlink(path);
    put_png(path, &g);
    bgpicture_free(&g);
    CHECK(import_scan(DPI) == 1 && finish_imports(res, 4, pages) == 1 && res[0] == 1 && pages[0] == 2);
    CHECK(notebooks_named("Waits") == 1 && !exists(src));
    snprintf(path, sizeof path, "%s/import/.attempts", root);
    char *txt = (char *)slurp(path, &(size_t){0});
    CHECK(!txt || !strstr(txt, " 99\n"));  // (the record went with the folder)
    free(txt);
}

// A failure that may pass (here: notebooks/ cannot be written) is tried at three starts, then the
// folder waits for its pictures to change, so a failure that comes back costs three tries, not one
// at every start: an import cut off each time (memory, a crash) is the same.
TEST(import_retries_three_times) {
    if (geteuid() == 0) return;  // (root writes anywhere)
    const char *files[] = {"a.png"};
    const uint8_t values[] = {70};
    char src[700], nbdir[700];
    import_folder_of("Thrice", files, values, 1, src, sizeof src);
    snprintf(nbdir, sizeof nbdir, "%s/notebooks", root);
    CHECK(chmod(nbdir, 0555) == 0);
    int res[4] = {0}, pages[4] = {0}, tried = 0;
    for (int start = 0; start < 5; start++)
        if (import_scan(DPI) == 1) {
            tried++;
            CHECK(finish_imports(res, 4, pages) == 1 && res[0] == -1);
        }
    CHECK(chmod(nbdir, 0755) == 0);
    CHECK(tried == 3 && notebooks_named("Thrice") == 0);
    // Touched (a file added), it is tried again.
    char path[900];
    snprintf(path, sizeof path, "%s/b.png", src);
    BgPicture g = solid(150, 200, 1, 80);
    put_png(path, &g);
    bgpicture_free(&g);
    CHECK(import_scan(DPI) == 1 && finish_imports(res, 4, pages) == 1 && res[0] == 1 && pages[0] == 2);
}

// Cut off between the notebook's rename into place and its folder's move: the next start finishes
// the move (the folder's mark names the notebook made) and makes no second notebook.
TEST(import_made_before_moved) {
    const char *files[] = {"p-1.png", "p-2.png"};
    const uint8_t values[] = {20, 30};
    char src[700];
    import_folder_of("Twice", files, values, 2, src, sizeof src);
    const char *id = "20261020-120005-f00d";
    mark(src, id);
    CHECK(bg_import_folder(src, DPI) == 0 && notebooks_named("Twice") == 1);
    // Put the folder back as the cut-off run left it.
    char done[700], back[900];
    snprintf(done, sizeof done, "%s/import/done", root);
    DIR *d = opendir(done);
    for (struct dirent *e; d && (e = readdir(d));)
        if (!strncmp(e->d_name, "Twice-", 6)) snprintf(back, sizeof back, "%s/%s", done, e->d_name);
    if (d) closedir(d);
    CHECK(rename(back, src) == 0);
    mark(src, id);
    int res[4] = {0}, pages[4] = {0};
    CHECK(import_scan(DPI) == 1 && finish_imports(res, 4, pages) == 1 && res[0] == 1 && pages[0] == 2);
    CHECK(notebooks_named("Twice") == 1 && !exists(src) && count_prefix(done, "Twice-") == 1);
}

// A notebook an import had finished (its notebook.ini written, last) but whose rename into place
// was lost (a power cut after the folder's move was durable and before the rename was): the next
// start puts it in place, never removes it.
TEST(import_leftover_whole) {
    const char *files[] = {"only.png"};
    const uint8_t values[] = {60};
    char src[700];
    import_folder_of("Kept", files, values, 1, src, sizeof src);
    const char *id = "20261020-120006-beef", *lost = "20261020-120007-cafe";
    mark(src, id);
    CHECK(bg_import_folder(src, DPI) == 0);
    char from[700], to[700];
    snprintf(from, sizeof from, "%s/notebooks/%s", root, id);
    snprintf(to, sizeof to, "%s/notebooks/.import-%s", root, lost);
    CHECK(rename(from, to) == 0 && notebooks_named("Kept") == 0);
    CHECK(import_scan(DPI) == 0);
    NotebookInfo nb;
    CHECK(!exists(to) && store_load_info(lost, &nb) == 0 && nb.npages == 1 && !strcmp(nb.name, "Kept"));
    CHECK(page_value(&nb, 0) == 60);
}

TEST(import_at_startup) {
    // What a cut-off import left: removed. A folder without pictures and import/done: left alone.
    char stale[700], path[900];
    snprintf(stale, sizeof stale, "%s/notebooks/.import-20261019-090000-0000", root);
    snprintf(path, sizeof path, "%s/bg/00abcd.png", stale);
    put_file(path, "half", 4);
    snprintf(path, sizeof path, "%s/p1.qpg", stale);
    put_file(path, "half", 4);
    snprintf(path, sizeof path, "%s/import/Empty/readme.txt", root);
    put_file(path, "nothing", 7);
    // "Broken" (from the test above) is whole now: it is imported again, with "Lieder".
    const char *files[] = {"01.jpg.png", "02.png"};
    const uint8_t values[] = {10, 20};
    char src[700];
    import_folder_of("Lieder", files, values, 2, src, sizeof src);
    int queued = import_scan(DPI);
    CHECK(queued == 2);
    CHECK(!exists(stale));
    int got = 0, pages = 0, total = 0;
    uint32_t until = now_ms() + 90000;
    while (got < 2 && (int32_t)(now_ms() - until) < 0) {
        int kind = -1, r = bg_poll_kind(&kind);
        if (kind == BG_IMPORT_FOLDER) {
            int f = import_finished(r > 0, &pages);
            CHECK(f == 1);
            if (f == 1) total += pages;
            got++;
        }
        usleep(5000);
    }
    CHECK(got == 2 && total == 4);  // Broken: 2 pages, Lieder: 2
    snprintf(path, sizeof path, "%s/import/Empty/readme.txt", root);
    CHECK(exists(path));
    snprintf(path, sizeof path, "%s/import", root);
    CHECK(count_prefix(path, "Lieder") == 0 && count_prefix(path, "Broken") == 0);
    // Nothing left to import: a second start queues nothing.
    CHECK(import_scan(DPI) == 0 && !bg_busy());
}

// A lock that cannot be taken for another reason than an import holding it says why (it is not
// "an import from an earlier run is still going"); the folders wait for the next start either way.
TEST(import_lock_errors) {
    const char *files[] = {"x.png"};
    const uint8_t values[] = {90};
    char src[700], lock[700];
    import_folder_of("Locked", files, values, 1, src, sizeof src);
    snprintf(lock, sizeof lock, "%s/import/.lock", root);
    unlink(lock);
    CHECK(mkdir(lock, 0755) == 0);  // (no file can be opened there)
    log_capture();
    CHECK(import_scan(DPI) == 0 && !bg_busy());
    char *log = log_captured();
    CHECK(log && strstr(log, "lock cannot be taken (") && !strstr(log, "still going"));
    free(log);
    CHECK(rmdir(lock) == 0);
    int res[2] = {0}, pages[2] = {0};
    CHECK(import_scan(DPI) == 1 && finish_imports(res, 2, pages) == 1 && res[0] == 1 && pages[0] == 1);
}

// A folder or template whose path would not reach a background job whole (a job holds 600 bytes:
// a long name in another script, under vfat's 255 characters) is left out, never cut and then
// tried at every start. Here a deep root makes the paths long. (Last: it moves the store.)
TEST(long_paths_left_out) {
    char deep[512], seg[240];
    memset(seg, 'd', 200);
    seg[200] = 0;
    snprintf(deep, sizeof deep, "%s/%s/", root, seg);
    memset(seg, 'e', 150);
    seg[150] = 0;
    strcat(deep, seg);
    CHECK(strlen(deep) < 500 && store_init(deep) == 0);
    templates_init();
    char name[240];
    memset(name, 'n', 230);
    name[230] = 0;
    char path[1600];
    BgPicture g = solid(150, 200, 1, 100);
    snprintf(path, sizeof path, "%s/import/%s/p.png", deep, name);
    put_png(path, &g);
    snprintf(path, sizeof path, "%s/templates/%s.png", deep, name);
    put_png(path, &g);
    char shortp[700];
    snprintf(shortp, sizeof shortp, "%s/templates/short.png", deep);
    put_png(shortp, &g);
    CHECK(import_scan(DPI) == 0 && !bg_busy());
    snprintf(path, sizeof path, "%s/import/%s/.grimoire-import", deep, name);
    CHECK(!exists(path));
    Template *t;
    int n;
    CHECK(template_scan(&t, &n, DPI) == 0 && n == 1 && !strcmp(t[0].name, "short"));
    template_list_free(t, n);
    drain_jobs(NULL);
    NotebookInfo nb;
    PaperKind k;
    const char *why = "";
    CHECK(store_create("Deep", "", PAPER_BLANK, &nb) == 0);
    snprintf(path, sizeof path, "%s/templates/%s.png", deep, name);
    CHECK(template_install(&nb, path, &k, &why) != 0 && strstr(why, "long"));
    bgpicture_free(&g);
}

int main(void) {
    snprintf(root, sizeof root, "%s/templates-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    store_set_device(DPI, PW, PH);
    RUN(image_kinds);
    RUN(image_paper_is_pure);
    RUN(missing_picture_draws_white);
    RUN(decoding);
    RUN(oversized_refused);
    RUN(decode_budget);
    RUN(ui_picture_limits);
    RUN(normalization);
    RUN(templates_same_file_same_id);
    RUN(source_key_streamed);
    RUN(templates_collision_steps);
    RUN(same_id_two_notebooks);
    RUN(template_previews);
    RUN(big_template_uses_its_preview_key);
    RUN(document_role_not_inherited);
    RUN(sidecar_restores_beta_resave);
    RUN(beta_reads_image_pages);
    RUN(erasing_over_a_picture);
    RUN(natural_order);
    RUN(import_makes_a_notebook);
    RUN(import_same_picture_twice);
    RUN(import_long_name);
    RUN(import_failure_leaves_nothing);
    RUN(import_at_startup);
    RUN(import_big_header);
    RUN(import_refused_waits);
    RUN(import_retries_three_times);
    RUN(import_made_before_moved);
    RUN(import_leftover_whole);
    RUN(import_lock_errors);
    RUN(long_paths_left_out);
    int rc = tl_done();
    if (!rc) system(cmd);
    return rc;
}
