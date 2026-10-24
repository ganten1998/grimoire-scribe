// test_annotate.c: a page KOReader sent to annotate (src/annotate.h, docs/FORMAT.md, Annotated
// pages): reading requests and source.ini, the order a book's pages go in, names, the checks a
// page picture gets, and what a start does with a request, on a scratch data root.
#include <dirent.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "annotate.h"
#include "bgimage.h"
#include "bgjob.h"
#include "export.h"
#include "font.h"
#include "i18n.h"
#include "testlib.h"
#include "ui.h"

// stb_image_write's deflate (stb_impl.c), for PNGs of the kinds KOReader's encoder (lodepng) writes.
unsigned char *stbi_zlib_compress(unsigned char *data, int data_len, int *out_len, int quality);

static char root[600];
static const float DPI = 300.f;
enum { PW = 300, PH = 400 };  // the page size the tests' notebooks get (store_set_device in main)
static const char *const BOOK = "0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70";
static const char *const TITLE = "Le Petit Prince : \xc3\xa9" "dition illustr\xc3\xa9" "e";

// ---- files ----

static bool exists(const char *path) { return access(path, F_OK) == 0; }

static void put_file(const char *path, const void *d, size_t n) {
    char dir[700];
    strlcpy_q(dir, path, sizeof dir);
    char *slash = strrchr(dir, '/');
    if (slash) *slash = 0;
    mkdir_p(dir);
    CHECK(write_file_atomic(path, d, n) == 0);
}

static void request_path(const char *name, char *buf, size_t cap) { snprintf(buf, cap, "%s/" ANNOTATE_DIR "/%s", root, name); }

static int files_in(const char *dir) {
    int n = 0;
    DIR *d = opendir(dir);
    for (struct dirent *e; d && (e = readdir(d));) n += strcmp(e->d_name, ".") && strcmp(e->d_name, "..");
    if (d) closedir(d);
    return n;
}

// ---- pictures ----

static uint32_t g_rng = 4242u;
static uint8_t rnd8(void) {
    g_rng = g_rng * 1103515245u + 12345u;
    return (uint8_t)(g_rng >> 16);
}

// A page of "text": dark runs on white, set by seed, w x h gray, as an encoded PNG (caller frees).
static uint8_t *text_png(int w, int h, uint32_t seed, size_t *len) {
    BgPicture p = {malloc((size_t)w * (size_t)h), w, h, 1};
    g_rng = seed;
    memset(p.px, 255, (size_t)w * (size_t)h);
    for (int y = 30; y + 12 < h - 30; y += 24)
        for (int x = 20; x < w - 40;) {
            int run = 8 + rnd8() % 40;
            for (int yy = y; yy < y + 12; yy++) memset(p.px + (size_t)yy * (size_t)w + (size_t)x, 30, (size_t)run);
            x += run + 8;
        }
    char path[700];
    snprintf(path, sizeof path, "%s/scratch.png", root);
    CHECK(bgimage_write_png(path, &p) == 0);
    free(p.px);
    uint8_t *d = read_file(path, len);
    unlink(path);
    return d;
}

static uint32_t crc(const uint8_t *p, size_t n) {
    uint32_t c = 0xFFFFFFFFu;
    for (size_t i = 0; i < n; i++) {
        c ^= p[i];
        for (int k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
    }
    return c ^ 0xFFFFFFFFu;
}

static size_t put_chunk(uint8_t *out, const char *type, const uint8_t *d, uint32_t n) {
    out[0] = (uint8_t)(n >> 24);
    out[1] = (uint8_t)(n >> 16);
    out[2] = (uint8_t)(n >> 8);
    out[3] = (uint8_t)n;
    memcpy(out + 4, type, 4);
    if (n) memcpy(out + 8, d, n);
    uint32_t c = crc(out + 4, 4 + (size_t)n);
    out[8 + n] = (uint8_t)(c >> 24);
    out[9 + n] = (uint8_t)(c >> 16);
    out[10 + n] = (uint8_t)(c >> 8);
    out[11 + n] = (uint8_t)c;
    return 12 + (size_t)n;
}

// A 4-bit palette PNG of w x h, as lodepng writes a page of a few grays: stripes of 16 levels.
static uint8_t *palette_png(int w, int h, size_t *len) {
    size_t row = (size_t)(w + 1) / 2 + 1, rawn = row * (size_t)h;
    uint8_t *raw = calloc(1, rawn);
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            uint8_t v = (uint8_t)((x / 10 + y / 10) & 15);
            raw[(size_t)y * row + 1 + (size_t)x / 2] |= x & 1 ? v : (uint8_t)(v << 4);
        }
    int zn = 0;
    uint8_t *z = stbi_zlib_compress(raw, (int)rawn, &zn, 8);
    free(raw);
    uint8_t ihdr[13] = {(uint8_t)(w >> 24), (uint8_t)(w >> 16), (uint8_t)(w >> 8), (uint8_t)w, (uint8_t)(h >> 24),
                        (uint8_t)(h >> 16), (uint8_t)(h >> 8), (uint8_t)h, 4, 3, 0, 0, 0};
    uint8_t plte[48];
    for (int i = 0; i < 16; i++) plte[3 * i] = plte[3 * i + 1] = plte[3 * i + 2] = (uint8_t)(i * 17);
    uint8_t *out = malloc(8 + 25 + 60 + (size_t)zn + 12 + 12);
    memcpy(out, "\x89PNG\r\n\x1a\n", 8);
    size_t k = 8;
    k += put_chunk(out + k, "IHDR", ihdr, 13);
    k += put_chunk(out + k, "PLTE", plte, 48);
    k += put_chunk(out + k, "IDAT", z, (uint32_t)zn);
    k += put_chunk(out + k, "IEND", NULL, 0);
    free(z);
    *len = k;
    return out;
}

// ---- requests ----

static void put_request(int page, int pages, const char *kind, const uint8_t *png, size_t len, const char *extra) {
    char path[800], text[1024];
    snprintf(text, sizeof text,
             "# Grimoire: a page to annotate, from KOReader's Grimoire plugin (docs/FORMAT.md)\nversion=1\nbook=%s\n"
             "title=%s\npage=%d\npages=%d\nkind=%s\npicture=page.png\ntime=%lld\n%s",
             BOOK, TITLE, page, pages, kind, (long long)wall_time(), extra ? extra : "");
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("page.png", path, sizeof path);
    if (png) put_file(path, png, len);
}

// What the code under test logs (grimoire.log is its stderr), from log_capture to log_captured.
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
    if (t) fputs(t, stderr);
    return t;
}

// Waits for the background jobs to finish (sanitized children on a busy machine: by the clock).
static void drain_jobs(void) {
    uint32_t until = now_ms() + 60000;
    while (bg_busy() && (int32_t)(now_ms() - until) < 0) {
        bg_poll();
        usleep(5000);
    }
    bg_poll();
}

static bool page_paper(const NotebookInfo *nb, int index, PaperKind *k) {
    Page *p = NULL;
    bool ok = store_read_page(nb, index, &p) == 0;
    if (ok) *k = p->paper;
    page_free(p);
    return ok;
}

// ---- reading a request ----

TEST(request_parsing) {
    AnnotateRequest r;
    const char *why;
    const char *good = "# a comment\r\nversion=1\r\nbook=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\r\ntitle=Le Petit Prince\r\n"
                       "page=12\r\npages=96\r\nkind=paged\r\npicture=page.png\r\ntime=1792500000\r\nfrom=a newer plugin\r\n";
    CHECK(annotate_parse(good, &r, &why) == 0);
    CHECK(r.version == 1 && !strcmp(r.book, BOOK) && !strcmp(r.title, "Le Petit Prince") && r.page == 12 && r.pages == 96);
    CHECK(!strcmp(r.kind, "paged") && !strcmp(r.picture, "page.png") && r.time == 1792500000);
    // No version line (a hand-made request): taken as version 1. A version it does not know: refused.
    CHECK(annotate_parse("book=ab12\npage=1\npages=1\npicture=a.png\n", &r, &why) == 0 && r.version == 1 && r.time == 0);
    CHECK(annotate_parse("version=2\nbook=ab12\npage=1\npages=1\npicture=a.png\n", &r, &why) != 0 && strstr(why, "version"));
    // What every request needs: a book key that is one, a page, a count at least the page, a picture.
    CHECK(annotate_parse("page=1\npages=1\npicture=a.png\n", &r, &why) != 0 && strstr(why, "book"));
    CHECK(annotate_parse("book=ab/12\npage=1\npages=1\npicture=a.png\n", &r, &why) != 0);
    CHECK(annotate_parse("book=ab12\npages=1\npicture=a.png\n", &r, &why) != 0 && strstr(why, "page number"));
    CHECK(annotate_parse("book=ab12\npage=0\npages=1\npicture=a.png\n", &r, &why) != 0);
    CHECK(annotate_parse("book=ab12\npage=5\npages=4\npicture=a.png\n", &r, &why) != 0 && strstr(why, "count"));
    CHECK(annotate_parse("book=ab12\npage=5\npicture=a.png\n", &r, &why) != 0);
    CHECK(annotate_parse("book=ab12\npage=-3\npages=4\npicture=a.png\n", &r, &why) != 0);
    CHECK(annotate_parse("book=ab12\npage=1\npages=1\n", &r, &why) != 0 && strstr(why, "picture"));
    // The picture is a plain .png name in the requests' folder: nothing that reaches elsewhere.
    const char *names[] = {"../x.png", "sub/x.png", ".x.png", "x.jpg", "x.png.tmp", "x y.png", ".png"};
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++) {
        char t[256];
        snprintf(t, sizeof t, "book=ab12\npage=1\npages=1\npicture=%s\n", names[i]);
        CHECK(annotate_parse(t, &r, &why) != 0);
    }
    CHECK(annotate_parse("book=ab12\npage=1\npages=1\npicture=page-12_v2.png\n", &r, &why) == 0);
    // A kind that is no word is left out; a title too long for a request is its start.
    CHECK(annotate_parse("book=ab12\npage=1\npages=1\npicture=a.png\nkind=Paged!\n", &r, &why) == 0 && !r.kind[0]);
    char big[2048];
    int k = snprintf(big, sizeof big, "book=ab12\npage=1\npages=9\npicture=a.png\ntitle=");
    for (int i = 0; i < 700; i++) big[k++] = 'x';
    snprintf(big + k, sizeof big - (size_t)k, "\npage=7\n");
    CHECK(annotate_parse(big, &r, &why) == 0 && strlen(r.title) == ANNOTATE_TITLE_LEN - 1 && r.page == 7);
    CHECK(annotate_parse(NULL, &r, &why) != 0);
}

TEST(request_age) {
    AnnotateRequest r = {.time = 1000000};
    CHECK(!annotate_too_old(&r, 1000000 + ANNOTATE_STALE_S, 0));
    CHECK(annotate_too_old(&r, 1000000 + ANNOTATE_STALE_S + 1, 0));
    CHECK(!annotate_too_old(&r, 1000000 - ANNOTATE_STALE_S, 0));
    CHECK(annotate_too_old(&r, 1000000 - ANNOTATE_STALE_S - 1, 0));  // from the future: the clock went back
    r.time = 0;  // no time: the file's own
    CHECK(!annotate_too_old(&r, 5000, 4990) && annotate_too_old(&r, 5000, 4000) && annotate_too_old(&r, 5000, 0));
}

// ---- source.ini and the order of a book's pages ----

static NotebookInfo pages_of(const uint32_t *ids, int n) {
    NotebookInfo nb;
    memset(&nb, 0, sizeof nb);
    for (int i = 0; i < n; i++) nb.page_ids[i] = ids[i];
    nb.npages = n;
    return nb;
}

TEST(map_round_trip) {
    const char *text = "# Grimoire: the book this notebook annotates\nbook=" "0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\nkind=paged\n"
                       "page.40.96=12\npage.12.96=10\nlater=a newer version's line\npage.x.96=3\npage.9.4=2\n"
                       "page.5.96=99\n";
    AnnotateMap m;
    CHECK(annotate_map_parse(text, &m) == 0);
    CHECK(!strcmp(m.book, BOOK) && !strcmp(m.kind, "paged") && m.n == 3);
    CHECK(strstr(m.extra, "later=a newer version's line\n") && strstr(m.extra, "page.x.96=3\n") && strstr(m.extra, "page.9.4=2\n"));
    uint32_t ids[] = {10, 11, 12, 13};
    NotebookInfo nb = pages_of(ids, 4);
    CHECK(annotate_map_find(&m, &nb, 12, 96) == 10 && annotate_map_find(&m, &nb, 40, 96) == 12);
    CHECK(annotate_map_find(&m, &nb, 5, 96) == 0);   // its page (99) is gone from the notebook
    CHECK(annotate_map_find(&m, &nb, 12, 300) == 0);  // another layout's page 12
    size_t len;
    char *out = annotate_map_text(&m, &nb, &len);
    CHECK(out && len == strlen(out));
    // In book order, the deleted page's line gone, the lines it does not know kept.
    const char *p12 = out ? strstr(out, "page.12.96=10\n") : NULL, *p40 = out ? strstr(out, "page.40.96=12\n") : NULL;
    CHECK(p12 && p40 && p12 < p40 && !strstr(out, "=99") && strstr(out, "later=a newer version's line\n"));
    CHECK(out && strstr(out, "book=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n") && strstr(out, "kind=paged\n"));
    AnnotateMap again;
    CHECK(annotate_map_parse(out, &again) == 0 && again.n == 2 && !strcmp(again.extra, m.extra));
    annotate_map_free(&again);
    free(out);
    CHECK(annotate_map_add(&m, 12, 96, 11) == 0 && m.n == 3 && annotate_map_find(&m, &nb, 12, 96) == 11);  // (moved)
    annotate_map_free(&m);
    CHECK(annotate_map_parse(NULL, &m) == 0 && m.n == 0 && !m.book[0]);
    annotate_map_free(&m);
}

TEST(page_order) {
    // Page 12 of 96 is the notebook's page 10, with a page of notes (11) after it; page 40 is 12,
    // with notes (13) after it too.
    uint32_t ids[] = {10, 11, 12, 13};
    NotebookInfo nb = pages_of(ids, 4);
    AnnotateMap m = {0};
    CHECK(annotate_map_add(&m, 12, 96, 10) == 0 && annotate_map_add(&m, 40, 96, 12) == 0);
    CHECK(annotate_insert_at(&nb, &m, 5, 96) == 0);   // before page 12
    CHECK(annotate_insert_at(&nb, &m, 20, 96) == 2);  // after page 12 and its notes, before page 40
    CHECK(annotate_insert_at(&nb, &m, 50, 96) == 4);  // at the end, after page 40's notes
    // A reflowable book read at two font sizes: by the place in the book (page - 1 of pages).
    annotate_map_free(&m);
    memset(&m, 0, sizeof m);
    CHECK(annotate_map_add(&m, 12, 300, 10) == 0);  // 11/300 into the book
    CHECK(annotate_insert_at(&nb, &m, 16, 400) == 4);  // 15/400, further: after
    CHECK(annotate_insert_at(&nb, &m, 15, 400) == 0);  // 14/400, not as far: before
    CHECK(annotate_insert_at(&nb, &m, 12, 400) == 0);
    // No book page in the notebook yet (or its pages deleted): at the end.
    annotate_map_free(&m);
    memset(&m, 0, sizeof m);
    CHECK(annotate_insert_at(&nb, &m, 3, 96) == 4);
    CHECK(annotate_map_add(&m, 50, 96, 77) == 0 && annotate_insert_at(&nb, &m, 3, 96) == 4);
    annotate_map_free(&m);
}

TEST(names) {
    char out[STORE_NAME_LEN];
    annotate_name("  Le Petit\tPrince\n ", out, sizeof out);
    CHECK(!strcmp(out, "Le Petit Prince"));
    annotate_name("\x01" "Caf\xc3\xa9 \xff\xfe" "cr\xe2\x82\xac" "me\xc2\x85", out, sizeof out);  // C0, broken, C1
    CHECK(!strcmp(out, "Caf\xc3\xa9 cr\xe2\x82\xac" "me"));
    annotate_name("\xed\xa0\x80" "x" "\xc0\xaf" "y" "\xf4\x90\x80\x80" "z", out, sizeof out);  // surrogate, overlong, too big
    CHECK(!strcmp(out, "xyz"));
    annotate_name("", out, sizeof out);
    CHECK(!out[0]);
    annotate_name(NULL, out, sizeof out);
    CHECK(!out[0]);
    // Cut to the name limit between two characters: 120 e acutes are 240 bytes; 47 fit in 95.
    char title[400] = "";
    for (int i = 0; i < 120; i++) strcat(title, "\xc3\xa9");
    annotate_name(title, out, sizeof out);
    CHECK(strlen(out) == 94);
    char ends[8];
    annotate_name("ab\xe2\x82\xac", ends, 5);  // "ab" + a 3-byte character: it does not fit whole
    CHECK(!strcmp(ends, "ab"));
    // Every letter stays, also what Grimoire's fonts cannot draw (the name is the PDF's too; only the
    // screens leave that out, ui_name_label): Greek, Cyrillic, Japanese, Vietnamese.
    annotate_name("\xd0\x92\xd0\xbe\xd0\xb9\xd0\xbd\xd0\xb0 \xce\xb1\xce\xb2 \xe6\x88\xa6\xe4\xba\x89", out, sizeof out);
    CHECK(!strcmp(out, "\xd0\x92\xd0\xbe\xd0\xb9\xd0\xbd\xd0\xb0 \xce\xb1\xce\xb2 \xe6\x88\xa6\xe4\xba\x89"));
    annotate_name("Pok\xc3\xa9mon \xe6\xbc\xab\xe7\x94\xbb Vol. 3", out, sizeof out);
    CHECK(!strcmp(out, "Pok\xc3\xa9mon \xe6\xbc\xab\xe7\x94\xbb Vol. 3"));
    annotate_name("\xe5\x90\xbe\xe8\xbc\xa9\xe3\x81\xaf\xe7\x8c\xab\xe3\x81\xa7\xe3\x81\x82\xe3\x82\x8b", out, sizeof out);
    CHECK(!strcmp(out, "\xe5\x90\xbe\xe8\xbc\xa9\xe3\x81\xaf\xe7\x8c\xab\xe3\x81\xa7\xe3\x81\x82\xe3\x82\x8b"));
    annotate_name("Ti\xe1\xba\xbfng Vi\xe1\xbb\x87t", out, sizeof out);  // (Tieng Viet, its marks whole)
    CHECK(!strcmp(out, "Ti\xe1\xba\xbfng Vi\xe1\xbb\x87t"));
    annotate_name("Le   Petit \t Prince", out, sizeof out);  // (one space between two words)
    CHECK(!strcmp(out, "Le Petit Prince"));
}

// notebook.ini's source= line: read, written back; a value of another form kept as it was.
TEST(notebook_source_line) {
    CHECK(store_source_ok(BOOK) && store_source_ok("a-b_C9") && !store_source_ok("") && !store_source_ok("a:b"));
    char long_key[80];
    memset(long_key, 'a', 63);
    long_key[63] = 0;
    CHECK(store_source_ok(long_key));
    long_key[63] = 'a';
    long_key[64] = 0;
    CHECK(!store_source_ok(long_key));
    NotebookInfo *nb = calloc(1, sizeof *nb);
    CHECK(store_create("Sourced", "", PAPER_BLANK, nb) == 0);
    char id[STORE_ID_LEN], dir[600], path[700];
    strlcpy_q(id, nb->id, sizeof id);
    store_notebook_dir(id, dir, sizeof dir);
    snprintf(path, sizeof path, "%s/notebook.ini", dir);
    size_t len;
    char *t = read_file(path, &len);
    CHECK(t && !strstr(t, "source="));  // none: no line
    free(t);
    strlcpy_q(nb->source, BOOK, sizeof nb->source);
    CHECK(store_save_info(nb) == 0);
    t = read_file(path, &len);
    CHECK(t && strstr(t, "\nsource=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n"));
    free(t);
    CHECK(store_load_info(id, nb) == 0 && !strcmp(nb->source, BOOK));
    // A newer version's form of the line: not this version's key, kept and written back as it was.
    t = read_file(path, &len);
    char *mixed = malloc(len + 64);
    snprintf(mixed, len + 64, "%ssource=pdf:42\n", t);
    put_file(path, mixed, strlen(mixed));
    free(t);
    free(mixed);
    CHECK(store_load_info(id, nb) == 0 && !strcmp(nb->source, BOOK) && strstr(nb->extra, "source=pdf:42\n"));
    CHECK(store_save_info(nb) == 0);
    t = read_file(path, &len);
    CHECK(t && strstr(t, "source=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n") && strstr(t, "source=pdf:42\n"));
    free(t);
    CHECK(store_delete(id) == 0);
    free(nb);
}

// ---- page pictures ----

TEST(png_checks) {
    size_t len;
    int w = 0, h = 0;
    const char *why;
    uint8_t *png = text_png(PW, PH, 7, &len);
    CHECK(png && annotate_png_check(png, len, &w, &h, &why) == 0 && w == PW && h == PH);
    CHECK(annotate_png_check(png, len * 3 / 5, &w, &h, &why) != 0 && !strcmp(why, "cut short"));
    CHECK(annotate_png_check(png, len - 12, &w, &h, &why) != 0);  // no IEND
    CHECK(annotate_png_check(png, 20, &w, &h, &why) != 0);
    uint8_t *bad = malloc(len);
    memcpy(bad, png, len);
    bad[len / 2] ^= 0x10;  // a bit flipped in its data: the CRC catches what the header cannot
    CHECK(annotate_png_check(bad, len, &w, &h, &why) != 0 && strstr(why, "checksum"));
    CHECK(bgimage_check(bad, len, len, &w, &h, &why) == 0);
    CHECK(annotate_png_check((const uint8_t *)"not a picture at all\n", 21, &w, &h, &why) != 0 && !strcmp(why, "not a PNG"));
    // An ancillary chunk with a wrong CRC (decoders skip those) does not count.
    uint8_t *anc = malloc(len + 20);
    memcpy(anc, png, 33);
    size_t k = 33 + put_chunk(anc + 33, "tEXt", (const uint8_t *)"Grimoire", 8);
    anc[k - 1] ^= 0xFF;
    memcpy(anc + k, png + 33, len - 33);
    CHECK(annotate_png_check(anc, k + len - 33, &w, &h, &why) == 0);
    free(anc);
    // Bytes after its end (an older, longer file written over without being cut, say): refused.
    uint8_t *tail = malloc(len + 7);
    memcpy(tail, png, len);
    memcpy(tail + len, "garbage", 7);
    CHECK(annotate_png_check(tail, len + 7, &w, &h, &why) != 0 && strstr(why, "after its end"));
    CHECK(annotate_png_check(tail, len, &w, &h, &why) == 0);
    free(tail);
    free(bad);
    free(png);
    // A palette PNG, as KOReader's encoder writes a page of few grays.
    uint8_t *pal = palette_png(PW, PH, &len);
    CHECK(annotate_png_check(pal, len, &w, &h, &why) == 0 && w == PW && h == PH);
    CHECK(bgimage_drawable(pal, len, &w, &h, &why) == 0);
    free(pal);
}

// ---- a start with a request ----

// What no start may log (grimoire.log goes into public bug reports): the requests' titles, book keys
// and picture names, as the tests below send them.
static const char *const k_private[] = {"Petit", "illustr", "0f3c6e2a9b1d", "ffff0000", "Read Sideways", "abcd1234",
                                        "Full Book", "beef4242", "Tolstoy", "\xe5\x90\xbe\xe8\xbc\xa9"};

// A start with whatever request is there: what it logged is in g_log (for the test to read further),
// held to the rule above.
static char *g_log;

static AnnotateResult take(bool asked) {
    AnnotateResult r;
    free(g_log);
    log_capture();
    annotate_take(asked, &r);
    g_log = log_captured();
    for (size_t i = 0; g_log && i < sizeof k_private / sizeof k_private[0]; i++) {
        if (strstr(g_log, k_private[i])) fprintf(stderr, "  a start logged \"%s\"\n", k_private[i]);
        CHECK(!strstr(g_log, k_private[i]));
    }
    return r;
}

static bool load(const char *id, NotebookInfo *nb) { return store_load_info(id, nb) == 0; }

TEST(starts_with_requests) {
    char dir[700], path[900];
    snprintf(dir, sizeof dir, "%s/" ANNOTATE_DIR, root);
    // No request: nothing, unless one was asked for.
    CHECK(take(false).outcome == ANNOTATE_NONE);
    AnnotateResult r = take(true);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_LOG);

    // Page 12 of a 96-page PDF: a notebook made for the book, its one page on the picture.
    size_t len12;
    uint8_t *png12 = text_png(PW, PH, 12, &len12);
    put_request(12, 96, "paged", png12, len12, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && r.pid == 1 && r.page == 12 && r.pages == 96);
    CHECK(g_log && strstr(g_log, "annotate: notebook "));  // (what take checks the log of, it logged)
    CHECK(!exists(dir));  // the request and its folder gone
    NotebookInfo *nb = calloc(1, sizeof *nb);
    char id[STORE_ID_LEN];
    strlcpy_q(id, r.nb, sizeof id);
    CHECK(load(id, nb) && !strcmp(nb->source, BOOK) && !strcmp(nb->name, TITLE) && nb->npages == 1);
    CHECK(nb->page_w == PW && nb->page_h == PH && nb->last_page == 0);
    // Its page a document page on the picture; its own paper Blank, as an imported notebook's (what a
    // page added to it gets, and what a page that does not read shows: never a book page's picture).
    PaperKind k = PAPER_BLANK;
    CHECK(page_paper(nb, 0, &k) && paper_is_document(k) && nb->paper == PAPER_BLANK);
    char bgdir[700], file[900];
    CHECK(bgimage_dir(id, bgdir, sizeof bgdir) == 0);
    bgimage_file(bgdir, paper_image_id(k), file, sizeof file);
    size_t got;
    uint8_t *kept = read_file(file, &got);
    CHECK(kept && got == len12 && !memcmp(kept, png12, len12));  // KOReader's picture, as it is
    free(kept);
    char nbdir[600];
    store_notebook_dir(id, nbdir, sizeof nbdir);
    snprintf(path, sizeof path, "%s/source.ini", nbdir);
    char *map = read_file(path, &got);
    CHECK(map && strstr(map, "book=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n") && strstr(map, "kind=paged\n") && strstr(map, "page.12.96=1\n"));
    free(map);
    snprintf(path, sizeof path, "%s/bg/pages.ini", nbdir);
    CHECK(exists(path));
    CHECK(annotate_probe("result", 0, file, sizeof file) == 0 && !strcmp(file, "made"));
    CHECK(annotate_probe("book_page", 0, file, sizeof file) == 0 && !strcmp(file, "12/96"));

    // The same page again: opened as it is, nothing added.
    put_request(12, 96, "paged", png12, len12, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_OPENED && !strcmp(r.nb, id) && r.pid == 1);
    CHECK(load(id, nb) && nb->npages == 1 && files_in(bgdir) == 3);  // the picture, index.ini, pages.ini

    // Page 5: before page 12. The card's thumbnail of the page that was first goes (the editor makes
    // the new one), and the notebook's paper stays Blank.
    snprintf(path, sizeof path, "%s/thumb.pgm", nbdir);
    put_file(path, "P5\n1 1\n255\n\x80", 12);
    size_t len5;
    uint8_t *png5 = text_png(PW, PH, 5, &len5);
    put_request(5, 96, "paged", png5, len5, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && !strcmp(r.nb, id));
    CHECK(load(id, nb) && nb->npages == 2 && nb->page_ids[0] == r.pid && nb->page_ids[1] == 1 && nb->last_page == 0);
    PaperKind k5 = PAPER_BLANK;
    CHECK(page_paper(nb, 0, &k5) && paper_is_document(k5) && k5 != k && nb->paper == PAPER_BLANK && !exists(path));
    uint32_t pid5 = r.pid;
    // Page 40: at the end, after a page of notes the owner added after page 12.
    CHECK(store_add_page(nb, 2, k) == 0);
    PaperKind knote = PAPER_MEDIUM;
    CHECK(page_paper(nb, 2, &knote) && knote == PAPER_BLANK);  // (no document page's picture is copied)
    size_t len40;
    uint8_t *png40 = text_png(PW, PH, 40, &len40);
    put_request(40, 96, "paged", png40, len40, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && load(id, nb) && nb->npages == 4 && nb->page_ids[3] == r.pid && nb->last_page == 3);
    CHECK(nb->paper == PAPER_BLANK && nb->page_ids[0] == pid5);  // (the first page stays page 5)
    // Page 20: after page 12 and its notes, before page 40.
    size_t len20;
    uint8_t *png20 = text_png(PW, PH, 20, &len20);
    put_request(20, 96, "paged", png20, len20, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && load(id, nb) && nb->npages == 5 && nb->page_ids[3] == r.pid);
    CHECK(nb->page_ids[0] == pid5 && nb->page_ids[1] == 1);
    CHECK(annotate_probe("book_page", 3, file, sizeof file) == 0 && !strcmp(file, "20/96"));
    CHECK(annotate_probe("book_page", 2, file, sizeof file) == 0 && !strcmp(file, ""));  // the notes

    // Not asked to open it (Grimoire started from the Library, or by Open Grimoire): deleted unopened.
    put_request(60, 96, "paged", png12, len12, NULL);
    r = take(false);
    CHECK(r.outcome == ANNOTATE_STALE && !exists(dir) && load(id, nb) && nb->npages == 5);
    int lines = 0;
    for (const char *c = g_log; c && (c = strstr(c, "annotate:")); c++) lines++;
    CHECK(lines == 1);  // one log line
    // Too old (or ahead of the clock), however asked.
    put_request(60, 96, "paged", png12, len12, "time=978307200\n");
    r = take(true);
    CHECK(r.outcome == ANNOTATE_STALE && !exists(dir) && load(id, nb) && nb->npages == 5);
    // A picture cut short, or no picture at all: nothing made, the folder emptied.
    put_request(60, 96, "paged", png12, len12 * 3 / 5, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_PICTURE && !exists(dir) && load(id, nb) && nb->npages == 5);
    put_request(60, 96, "paged", NULL, 0, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_PICTURE && !exists(dir));
    // A picture that is a link to a file elsewhere (the plugin writes a file of its own): not taken.
    put_request(60, 96, "paged", NULL, 0, NULL);
    snprintf(file, sizeof file, "%s/elsewhere.png", root);
    put_file(file, png12, len12);
    request_path("page.png", path, sizeof path);
    CHECK(symlink(file, path) == 0);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_PICTURE && !exists(dir) && exists(file));
    unlink(file);
    // A request that does not read.
    request_path("request.ini", path, sizeof path);
    put_file(path, "book=\n", 6);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_LOG && !exists(dir));

    // An older version saved the notebook and dropped its source= line: source.ini still names the
    // book, so its next page goes there, and the line comes back.
    nb->source[0] = 0;
    CHECK(store_save_info(nb) == 0 && load(id, nb) && !nb->source[0]);
    size_t len70;
    uint8_t *png70 = text_png(PW, PH, 70, &len70);
    put_request(70, 96, "paged", png70, len70, NULL);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && !strcmp(r.nb, id) && load(id, nb) && !strcmp(nb->source, BOOK) && nb->npages == 6);

    // Another book (its picture under another name): a notebook of its own.
    char text[700];
    snprintf(text, sizeof text,
             "version=1\nbook=ffff0000\ntitle=%s\npage=1\npages=1\nkind=paged\npicture=Tolstoy-1.png\ntime=%lld\n", TITLE,
             (long long)wall_time());
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("Tolstoy-1.png", path, sizeof path);
    put_file(path, png12, len12);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && strcmp(r.nb, id));
    NotebookInfo *other = calloc(1, sizeof *other);
    CHECK(load(r.nb, other) && !strcmp(other->source, "ffff0000"));
    CHECK(!strcmp(other->name, "Le Petit Prince : \xc3\xa9" "dition illustr\xc3\xa9" "e 2"));  // (its PDF's name apart)
    // Books whose titles Grimoire's fonts draw none of (Japanese): named after their titles all the
    // same, the second with " 2", as any two notebooks of one name would share a PDF; that PDF has
    // the title as its name. The screens show them as Untitled and Untitled 2 (ui_name_label).
    const char *ja = "\xe5\x90\xbe\xe8\xbc\xa9\xe3\x81\xaf\xe7\x8c\xab\xe3\x81\xa7\xe3\x81\x82\xe3\x82\x8b";
    for (int i = 0; i < 2; i++) {
        snprintf(text, sizeof text,
                 "version=1\nbook=abcd1234%d\ntitle=\xe5\x90\xbe\xe8\xbc\xa9\xe3\x81\xaf\xe7\x8c\xab\xe3\x81\xa7\xe3\x81\x82"
                 "\xe3\x82\x8b\npage=1\npages=1\nkind=paged\npicture=page.png\ntime=%lld\n",
                 i, (long long)wall_time());
        request_path("request.ini", path, sizeof path);
        put_file(path, text, strlen(text));
        request_path("page.png", path, sizeof path);
        put_file(path, png12, len12);
        r = take(true);
        char want[64], pdf[700], label[64];
        snprintf(want, sizeof want, "%s%s", ja, i ? " 2" : "");
        CHECK(r.outcome == ANNOTATE_MADE && load(r.nb, other) && !strcmp(other->name, want));
        export_mirror_path(other, "PDFs", pdf, sizeof pdf);
        snprintf(want, sizeof want, "PDFs/%s%s.pdf", ja, i ? " 2" : "");
        CHECK(!strcmp(pdf, want));
        CHECK(!strcmp(ui_name_label(other->name, label, sizeof label), i ? "Untitled 2" : "Untitled"));
    }
    free(other);

    // A page whose file is gone (or does not read) shows the notebook's paper, Blank: never another
    // book page's picture, which an edit would then save into it.
    int at12 = -1;
    for (int i = 0; i < nb->npages; i++)
        if (nb->page_ids[i] == 1) at12 = i;
    snprintf(path, sizeof path, "%s/p1.qpg", nbdir);
    CHECK(at12 >= 0 && unlink(path) == 0);
    PaperKind gone = PAPER_MEDIUM;
    CHECK(page_paper(nb, at12, &gone) && gone == PAPER_BLANK);

    free(png5);
    free(png20);
    free(png40);
    free(png70);
    free(png12);
    free(nb);
}

TEST(pictures_of_another_size) {
    // A notebook from a Scribe of the other size, or a page sent sideways: fitted in the background.
    size_t len;
    uint8_t *png = text_png(PH, PW, 3, &len);  // sideways
    char text[700], path[900];
    snprintf(text, sizeof text, "version=1\nbook=abcd1234\ntitle=A Book Read Sideways\npage=3\npages=10\nkind=reflowable\npicture=page.png\n"
                                "time=%lld\n", (long long)wall_time());
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("page.png", path, sizeof path);
    put_file(path, png, len);
    AnnotateResult r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE);
    NotebookInfo *nb = calloc(1, sizeof *nb);
    PaperKind k = PAPER_BLANK;
    CHECK(load(r.nb, nb) && page_paper(nb, 0, &k) && paper_is_document(k));
    char bgdir[700], file[900], source[900];
    bgimage_dir(r.nb, bgdir, sizeof bgdir);
    bgimage_file(bgdir, paper_image_id(k), file, sizeof file);
    snprintf(source, sizeof source, "%s/%06x.source.png", bgdir, (unsigned)paper_image_id(k));
    CHECK(!exists(file) || bg_busy());  // made in the background
    drain_jobs();
    CHECK(exists(file) && !exists(source));
    size_t got;
    uint8_t *made = read_file(file, &got);
    int w = 0, h = 0;
    const char *why;
    CHECK(made && bgimage_drawable(made, got, &w, &h, &why) == 0 && w == PW && h == PH);  // the page's size
    free(made);
    // Its picture gone missing (its job never ran): the same page sent again makes it again. Drawn
    // meanwhile, the page is white; once the picture is there it is "undrawn" (for the editor: draw
    // that page again), and once drawn, no longer.
    char id[STORE_ID_LEN];
    strlcpy_q(id, r.nb, sizeof id);
    uint32_t pic = paper_image_id(k);
    unlink(file);
    bgimage_forget(NULL);
    Canvas c;
    CHECK(canvas_init(&c, PW, PH) == 0);
    bgimage_render(&c, id, k, (Rect){0, 0, PW, PH});
    CHECK(c.px[(size_t)(PH / 2) * (size_t)c.stride + PW / 2] == 255 && !bgimage_undrawn(id, pic));  // (no file yet)
    // The page left before its picture is made: the editor wrote the Library's card from the page
    // drawn white. The job that makes the picture draws the card again, the page on its picture.
    Canvas white;
    CHECK(canvas_init(&white, PW / 3, PH / 3) == 0 && store_save_thumb(id, &white) == 0);
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("page.png", path, sizeof path);
    put_file(path, png, len);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_OPENED);
    drain_jobs();
    CHECK(exists(file) && !exists(source));
    bgimage_retry_missing();  // (as poll_jobs does once a picture job is done)
    Canvas card, want;
    Page *first = NULL;
    CHECK(store_load_thumb(id, &card) == 0 && store_read_page(nb, 0, &first) == 0 && first);
    CHECK(canvas_init(&want, PW / 3, PH / 3) == 0);
    if (first) page_render_thumb(&want, first, DPI);
    CHECK(card.w == want.w && card.h == want.h && !memcmp(card.px, want.px, (size_t)want.w * (size_t)want.h));
    CHECK(card.px && memcmp(card.px, white.px, (size_t)white.w * (size_t)white.h) != 0);
    page_free(first);
    canvas_free(&card);
    canvas_free(&want);
    canvas_free(&white);
    bgimage_forget(NULL);
    CHECK(bgimage_undrawn(id, pic) && !bgimage_undrawn(id, (pic + 1) & 0xFFFFFFu));
    bgimage_render(&c, id, k, (Rect){0, 0, PW, PH});
    CHECK(!bgimage_undrawn(id, pic));
    canvas_free(&c);
    free(nb);
    free(png);
}

// A notebook's raw zip carries its source.ini, so one put back from the zip keeps which of its pages
// is which book page: the same page sent again opens it, rather than adding a second one.
TEST(raw_zip_keeps_the_map) {
    size_t len;
    uint8_t *png = text_png(PW, PH, 7, &len);
    char text[700], path[900];
    snprintf(text, sizeof text, "version=1\nbook=feedc0de\ntitle=Zipped Book\npage=7\npages=20\nkind=paged\npicture=page.png\n"
                                "time=%lld\n", (long long)wall_time());
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("page.png", path, sizeof path);
    put_file(path, png, len);
    AnnotateResult r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE);
    NotebookInfo *nb = calloc(1, sizeof *nb);
    CHECK(load(r.nb, nb));
    Buf b = {0};
    ZipWriter z;
    zip_begin(&z, &b);
    CHECK(export_notebook_zip(nb, DPI, &z, "") == 0 && zip_end(&z) == 0);
    char zp[700], nbdir[600], cmd[2000];
    snprintf(zp, sizeof zp, "%s/zipped.zip", root);
    put_file(zp, b.data, b.len);
    buf_free(&b);
    store_notebook_dir(r.nb, nbdir, sizeof nbdir);
    snprintf(cmd, sizeof cmd, "unzip -p '%s' 'Zipped Book/raw/source.ini' | cmp -s - '%s/source.ini'", zp, nbdir);
    CHECK(system(cmd) == 0);
    unlink(zp);
    free(nb);
    free(png);
}

TEST(failures_leave_nothing) {
    char text[700], path[900], nbdir[600];
    size_t len;
    uint8_t *png = text_png(PW, PH, 9, &len);
    snprintf(text, sizeof text, "version=1\nbook=beef4242\ntitle=Full Book\npage=1\npages=9\nkind=paged\npicture=page.png\ntime=%lld\n",
             (long long)wall_time());
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("page.png", path, sizeof path);
    put_file(path, png, len);
    AnnotateResult r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE);
    char id[STORE_ID_LEN];
    strlcpy_q(id, r.nb, sizeof id);
    store_notebook_dir(id, nbdir, sizeof nbdir);
    NotebookInfo *nb = calloc(1, sizeof *nb);
    // The notebook's folder cannot be written (a full storage, as far as this can tell): the page
    // is not added, and the picture put in bg/ for it is taken back.
    char bgdir[700];
    snprintf(bgdir, sizeof bgdir, "%s/bg", nbdir);
    int before = files_in(bgdir);
    uint8_t *png2 = text_png(PW, PH, 10, &len);
    snprintf(text, sizeof text, "version=1\nbook=beef4242\ntitle=Full Book\npage=2\npages=9\nkind=paged\npicture=page.png\ntime=%lld\n",
             (long long)wall_time());
    request_path("request.ini", path, sizeof path);
    put_file(path, text, strlen(text));
    request_path("page.png", path, sizeof path);
    put_file(path, png2, len);
    chmod(nbdir, 0555);
    r = take(true);
    chmod(nbdir, 0755);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_STORAGE);
    CHECK(load(id, nb) && nb->npages == 1 && files_in(bgdir) == before);
    // A source.ini that does not read: left as it is, nothing added over it.
    snprintf(path, sizeof path, "%s/source.ini", nbdir);
    chmod(path, 0);
    snprintf(text, sizeof text, "version=1\nbook=beef4242\ntitle=Full Book\npage=2\npages=9\nkind=paged\npicture=page.png\ntime=%lld\n",
             (long long)wall_time());  // (the same request again)
    char req[900];
    request_path("request.ini", req, sizeof req);
    put_file(req, text, strlen(text));
    request_path("page.png", req, sizeof req);
    put_file(req, png2, len);
    r = take(true);
    chmod(path, 0644);
    CHECK(r.outcome == ANNOTATE_FAILED && load(id, nb) && nb->npages == 1);

    // A page is listed last, its id taken first. notebook.ini cannot be written: nothing is (not its
    // file, nor its line in bg/pages.ini or in source.ini).
    char nbini[700], spare[800], side[800];
    snprintf(nbini, sizeof nbini, "%s/notebook.ini.tmp", nbdir);
    snprintf(path, sizeof path, "%s/source.ini", nbdir);
    snprintf(side, sizeof side, "%s/pages.ini", bgdir);
    size_t map0n, side0n, got;
    char *map0 = read_file(path, &map0n), *side0 = read_file(side, &side0n);
    uint32_t next0 = nb->next_page_id;
    CHECK(map0 && side0 && mkdir(nbini, 0755) == 0);  // (a folder where its new copy goes)
    request_path("request.ini", req, sizeof req);
    put_file(req, text, strlen(text));
    request_path("page.png", req, sizeof req);
    put_file(req, png2, len);
    r = take(true);
    rmdir(nbini);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_STORAGE);
    CHECK(load(id, nb) && nb->npages == 1 && nb->next_page_id == next0 && files_in(bgdir) == before);
    snprintf(spare, sizeof spare, "%s/p%u.qpg", nbdir, next0);
    char *t = read_file(path, &got);
    CHECK(!exists(spare) && t && got == map0n && !memcmp(t, map0, got));
    free(t);
    t = read_file(side, &got);
    CHECK(t && got == side0n && !memcmp(t, side0, got));
    free(t);
    free(map0);
    free(side0);
    // source.ini cannot be written, after the page's file was: taken back, its id given up (never
    // another page's), the picture too.
    snprintf(spare, sizeof spare, "%s/source.ini.tmp", nbdir);
    CHECK(mkdir(spare, 0755) == 0);
    request_path("request.ini", req, sizeof req);
    put_file(req, text, strlen(text));
    request_path("page.png", req, sizeof req);
    put_file(req, png2, len);
    r = take(true);
    rmdir(spare);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_STORAGE);
    CHECK(load(id, nb) && nb->npages == 1 && nb->next_page_id == next0 + 1 && files_in(bgdir) == before);
    snprintf(spare, sizeof spare, "%s/p%u.qpg", nbdir, next0);
    CHECK(!exists(spare));
    // Sent again, the page goes in with the next id.
    request_path("request.ini", req, sizeof req);
    put_file(req, text, strlen(text));
    request_path("page.png", req, sizeof req);
    put_file(req, png2, len);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && r.pid == next0 + 1 && load(id, nb) && nb->npages == 2 && nb->page_ids[1] == r.pid);
    // A start cut off just before notebook.ini listed the page left its file and its lines in
    // bg/pages.ini and source.ini, its id taken: sent again, the page is a new one (the next id),
    // and the one left behind never becomes a page.
    uint32_t left = r.pid;
    nb->npages = 1;
    CHECK(store_write_info_in(nbdir, nb) == 0 && load(id, nb) && nb->next_page_id == left + 1);
    snprintf(spare, sizeof spare, "%s/p%u.qpg", nbdir, left);
    CHECK(exists(spare));
    request_path("request.ini", req, sizeof req);
    put_file(req, text, strlen(text));
    request_path("page.png", req, sizeof req);
    put_file(req, png2, len);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_MADE && r.pid == left + 1 && load(id, nb) && nb->npages == 2 && nb->page_ids[1] == r.pid);
    t = read_file(path, &got);
    char want[64], gone[64];
    snprintf(want, sizeof want, "page.2.9=%u\n", r.pid);
    snprintf(gone, sizeof gone, "=%u\n", left);
    CHECK(t && strstr(t, want) && !strstr(t, gone));
    free(t);

    // A notebook as full as one gets (2000 pages): nothing added.
    for (int i = 0; i < STORE_MAX_PAGES; i++) nb->page_ids[i] = (uint32_t)i + 1;
    nb->npages = STORE_MAX_PAGES;
    CHECK(store_save_info(nb) == 0);
    snprintf(text, sizeof text, "version=1\nbook=beef4242\ntitle=Full Book\npage=3\npages=9\nkind=paged\npicture=page.png\ntime=%lld\n",
             (long long)wall_time());
    request_path("request.ini", req, sizeof req);
    put_file(req, text, strlen(text));
    request_path("page.png", req, sizeof req);
    put_file(req, png2, len);
    r = take(true);
    CHECK(r.outcome == ANNOTATE_FAILED && r.why == ANNOTATE_WHY_LOG);
    // What a start cut off while making a notebook left: removed at the next start.
    char work[800];
    snprintf(work, sizeof work, "%s/notebooks/.annotate-20261020-000000-dead/bg", root);
    CHECK(mkdir_p(work) == 0);
    snprintf(path, sizeof path, "%s/000001.png", work);
    put_file(path, png, 10);
    CHECK(take(false).outcome == ANNOTATE_NONE);
    snprintf(work, sizeof work, "%s/notebooks/.annotate-20261020-000000-dead", root);
    CHECK(!exists(work));
    free(nb);
    free(png);
    free(png2);
}

// The notices a start with a request can end with (app.c), in every language and style, on the
// narrow screen: a toast has two lines. Keep this list in step with the messages.
TEST(notices_fit) {
    Rect screen = {0, 0, 1860, 2480};
    for (int l = 0; l < i18n_language_count(); l++) {
        i18n_set(i18n_language_code(l));
        char log[256];
        snprintf(log, sizeof log, _("Could not open the page KOReader sent (see %s)."), "grimoire.log");
        const char *notes[] = {_("Could not open the page KOReader sent: its picture does not read."),
                               _("Could not add the page KOReader sent. Is the storage full?"), log};
        for (int th = THEME_GOTHIC; th <= THEME_CLASSIC; th++) {
            ui_set_theme((UiTheme)th);
            for (int i = 0; i < 3; i++) {
                int lines = font_wrap_lines(FONT_BOLD, ui_text_size(), notes[i], ui_toast_room(screen), NULL);
                if (lines < 1 || lines > 2) fprintf(stderr, "  %s: \"%s\" needs %d lines\n", i18n_current(), notes[i], lines);
                CHECK(lines >= 1 && lines <= 2);
                CHECK(strlen(notes[i]) < 256);  // whole in a toast's buffer
            }
        }
        // Translated (every language but English has its words for them).
        if (strcmp(i18n_language_code(l), "en"))
            CHECK(strcmp(notes[0], "Could not open the page KOReader sent: its picture does not read."));
    }
    ui_set_theme(THEME_GOTHIC);
    i18n_set("en");
}

int main(void) {
    snprintf(root, sizeof root, "%s/annotate-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    store_set_device(DPI, PW, PH);
    ui_init(300.f);
    if (font_init() != 0) return 1;
    RUN(request_parsing);
    RUN(request_age);
    RUN(map_round_trip);
    RUN(page_order);
    RUN(names);
    RUN(notebook_source_line);
    RUN(png_checks);
    RUN(starts_with_requests);
    RUN(pictures_of_another_size);
    RUN(raw_zip_keeps_the_map);
    RUN(failures_leave_nothing);
    RUN(notices_fit);
    free(g_log);
    int rc = tl_done();
    if (!rc) system(cmd);
    return rc;
}
