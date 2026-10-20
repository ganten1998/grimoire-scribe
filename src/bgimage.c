// bgimage.c: image papers (see bgimage.h). Pictures are decoded by stb_image (third_party),
// compiled here: PNG, JPEG and PNM only, at most 8192 px a side, no SIMD (every build decodes a
// picture to the same bytes), and every byte it allocates counted against a budget (below).
#include "bgimage.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "store.h"

// stb_image allocates through these (defined below, with the budget they keep).
static void *stb_malloc(size_t n);
static void *stb_realloc(void *p, size_t n);
static void stb_free(void *p);

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wsign-compare"
#pragma clang diagnostic ignored "-Wunused-function"
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wmissing-field-initializers"
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
#pragma clang diagnostic ignored "-Wimplicit-fallthrough"
// stb_image's JPEG IDCT (stb_image.h:2502-2512) can overflow an int on crafted input: harmless on
// every CPU Grimoire runs on (it wraps), but the test builds stop at any undefined behaviour, so
// stb_image's own functions are left out of that one check (the header stays as upstream has it).
#pragma clang attribute push(__attribute__((no_sanitize("signed-integer-overflow"))), apply_to = function)
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wsign-compare"
#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wunused-parameter"
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#pragma GCC diagnostic ignored "-Wimplicit-fallthrough"
#endif
#define STB_IMAGE_IMPLEMENTATION
#define STBI_ONLY_PNG
#define STBI_ONLY_JPEG
#define STBI_ONLY_PNM
#define STBI_MAX_DIMENSIONS 8192
#define STBI_NO_STDIO
#define STBI_NO_SIMD
#define STBI_NO_LINEAR
#define STBI_NO_HDR
#define STBI_MALLOC(sz) stb_malloc(sz)
#define STBI_REALLOC(p, newsz) stb_realloc(p, newsz)
#define STBI_FREE(p) stb_free(p)
#include "stb_image.h"
#if defined(__clang__)
#pragma clang attribute pop
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

// Defined in stb_image_write's implementation section (stb_impl.c) but not in its API header.
unsigned char *stbi_write_png_to_mem(const unsigned char *pixels, int stride_bytes, int x, int y, int n, int *out_len);

_Static_assert(PAGE_NB_LEN == STORE_ID_LEN, "a page's notebook link holds a notebook id");

// What a picture drawn on the UI thread may take: a notebook's own picture, a PNG of about a page
// (an RGB 1860x2480 one takes about 30 MB to decode, one of noise at 1986x2648 about 65 MB). One
// past these is no picture Grimoire made.
#define BG_UI_FILE_MAX ((size_t)32 << 20)
#define BG_UI_DECODE_BUDGET ((size_t)96 << 20)

// ---- stb_image's memory ----
//
// Each block stb_image holds is listed with its size while a decode runs (stb_begin .. stb_end),
// and one that would take what it holds past the decode's budget is refused: stb_image then gives
// up ("outofmem"), and g_stb_over says the budget, not the system, ran out. Outside a decode
// nothing is listed (the picture a decode returns is an ordinary malloc block, freed with free).
#define STB_BLOCKS 64  // stb_image holds a dozen at most (a JPEG: 3 a component, and its state)
static struct {
    void *p;
    size_t n;
} g_blk[STB_BLOCKS];
static size_t g_stb_used, g_stb_peak, g_stb_last_peak, g_stb_budget = SIZE_MAX;
static bool g_stb_over;

static void stb_begin(size_t budget) {
    memset(g_blk, 0, sizeof g_blk);
    g_stb_used = g_stb_peak = 0;
    g_stb_budget = budget;
    g_stb_over = false;
}

static void stb_end(void) {
    g_stb_last_peak = g_stb_peak;
    memset(g_blk, 0, sizeof g_blk);
    g_stb_used = 0;
    g_stb_budget = SIZE_MAX;
}

size_t bgimage_last_decode_peak(void) { return g_stb_last_peak; }

static int blk_find(const void *p) {
    for (int i = 0; i < STB_BLOCKS; i++)
        if (g_blk[i].p == p) return i;
    return -1;
}

// Whether `more` bytes fit in the budget beside what is held; if not, the decode is over it.
static bool stb_room(size_t more) {
    if (more <= g_stb_budget - g_stb_used) return true;
    g_stb_over = true;
    return false;
}

static void stb_hold(int i, void *p, size_t n) {
    g_blk[i].p = p;
    g_blk[i].n = n;
    g_stb_used += n;
    if (g_stb_used > g_stb_peak) g_stb_peak = g_stb_used;
}

static void *stb_malloc(size_t n) {
    int i = blk_find(NULL);
    if (i < 0 || !stb_room(n)) {
        g_stb_over = true;
        return NULL;
    }
    void *p = malloc(n ? n : 1);
    if (p) stb_hold(i, p, n);
    return p;
}

static void stb_free(void *p) {
    if (!p) return;
    int i = blk_find(p);
    if (i >= 0) {
        g_stb_used -= g_blk[i].n;
        g_blk[i].p = NULL;
        g_blk[i].n = 0;
    }
    free(p);
}

static void *stb_realloc(void *p, size_t n) {
    if (!p) return stb_malloc(n);
    int i = blk_find(p);
    if (i < 0) {  // (never: stb_image reallocates only what it allocated)
        g_stb_over = true;
        return NULL;
    }
    size_t old = g_blk[i].n;
    if (n > old && !stb_room(n - old)) return NULL;
    void *q = realloc(p, n ? n : 1);
    if (!q) return NULL;
    g_stb_used -= old;
    stb_hold(i, q, n);
    return q;
}

// ---- files ----

int bgimage_dir(const char *nb, char *buf, size_t cap) {
    char dir[600];
    if (!nb || store_notebook_dir(nb, dir, sizeof dir) != 0) return -1;
    snprintf(buf, cap, "%s/bg", dir);
    return 0;
}

void bgimage_file(const char *bgdir, uint32_t id, char *buf, size_t cap) {
    snprintf(buf, cap, "%s/%06x.png", bgdir, (unsigned)(id & 0xFFFFFFu));
}

void bgpicture_free(BgPicture *p) {
    free(p->px);
    memset(p, 0, sizeof *p);
}

// ---- reading ----

uint8_t *bgimage_read(const char *path, size_t max, size_t *len) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return NULL;
    struct stat st;
    uint8_t *buf = NULL;
    int err = 0;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode) || st.st_size < 0) err = ENOENT;
    else if ((uint64_t)st.st_size > (uint64_t)max) err = EFBIG;
    else {
        size_t n = (size_t)st.st_size, got = 0;
        buf = malloc(n ? n : 1);
        while (buf && got < n) {
            ssize_t r = read(fd, buf + got, n - got);
            if (r < 0 && errno == EINTR) continue;
            if (r <= 0) {  // an error, or a file shorter than it said (it changed)
                err = r < 0 ? errno : EIO;
                free(buf);
                buf = NULL;
                break;
            }
            got += (size_t)r;
        }
        if (buf) *len = n;
        else if (!err) err = ENOMEM;
    }
    close(fd);
    if (!buf) errno = err;
    return buf;
}

// ---- what a decode takes ----

static size_t max_z(size_t a, size_t b) { return a > b ? a : b; }

// A PNG's peak by stb_image's steps (stb_image.h, stbi__parse_png_file): the compressed data
// gathered (a buffer that doubles: up to twice the file), then inflated (one filter byte a row; an
// interlaced picture's passes have more rows than stb_image guesses, so its buffer doubles), then
// unfiltered into the picture at its own depth (two rows of workspace, and for an interlaced one
// each pass's picture beside it), its palette expanded while the inflated data is still held,
// then 16 bits cut to 8 beside the picture. n: the channels stbi_info gives (a palette's 3 or 4;
// one more with a tRNS chunk).
static size_t png_cost(const uint8_t *d, size_t len, size_t file_len, size_t w, size_t h, int n) {
    size_t at = 8;  // the first chunk: IHDR, or an iPhone PNG's CgBI and then IHDR
    if (len >= at + 8 && !memcmp(d + at + 4, "CgBI", 4))
        at += 12 + ((size_t)d[at] << 24 | (size_t)d[at + 1] << 16 | (size_t)d[at + 2] << 8 | d[at + 3]);
    if (at > len || len - at < 21 || memcmp(d + at + 4, "IHDR", 4)) return 0;
    const uint8_t *ih = d + at + 8;  // width, height, then depth, colour type, compression, filter, interlace
    int depth = ih[8], color = ih[9], interlace = ih[12];
    bool palette = color == 3;
    size_t in_n = palette ? 1 : (size_t)((color & 2 ? 3 : 1) + (color & 4 ? 1 : 0));
    size_t bytes = depth == 16 ? 2 : 1, out_n = palette ? 1 : (size_t)n;
    size_t row = (w * in_n * (size_t)depth + 7) / 8, raw = (row + 1) * h, out = w * h * out_n * bytes;
    size_t inflated = interlace ? 2 * raw : raw;
    size_t peak = 2 * (file_len < ((size_t)1 << 30) ? file_len : ((size_t)1 << 30)) + inflated;
    peak = max_z(peak, inflated + out + 2 * row + (interlace ? out : 0));
    if (palette) peak = max_z(peak, inflated + out + w * h * (size_t)n);
    if (bytes == 2) peak = max_z(peak, out + w * h * out_n);
    return peak;
}

// A JPEG's: each component's plane padded to whole blocks of its sampling (stb_image.h,
// stbi__process_frame_header), for a progressive one its coefficients too (2 bytes each), a line
// buffer each, and the picture (gray or RGB), from its frame header (SOFn).
static size_t jpeg_cost(const uint8_t *d, size_t len, size_t w, size_t h) {
    size_t i = 2;
    while (i + 4 <= len) {
        if (d[i] != 0xFF) {  // padding after a segment: stb_image scans past it too
            i++;
            continue;
        }
        uint8_t m = d[i + 1];
        if (m == 0xFF) {  // fill
            i++;
            continue;
        }
        if (m == 0x01 || (m >= 0xD0 && m <= 0xD8)) {  // markers without a length
            i += 2;
            continue;
        }
        size_t seg = (size_t)d[i + 2] << 8 | d[i + 3];
        if (seg < 2) return 0;
        if (m >= 0xC0 && m <= 0xCF && m != 0xC4 && m != 0xC8 && m != 0xCC) {
            const uint8_t *f = d + i + 4;  // precision, height, width, components, then 3 bytes each
            size_t nc = i + 10 <= len ? f[5] : 0;
            if (nc < 1 || nc > 4 || seg < 8 + 3 * nc || i + 2 + seg > len) return 0;
            bool progressive = m == 0xC2 || m == 0xC6 || m == 0xCA || m == 0xCE;
            size_t hmax = 1, vmax = 1;
            for (size_t c = 0; c < nc; c++) {
                hmax = max_z(hmax, (size_t)(f[7 + 3 * c] >> 4));
                vmax = max_z(vmax, (size_t)(f[7 + 3 * c] & 15));
            }
            size_t mx = (w + 8 * hmax - 1) / (8 * hmax), my = (h + 8 * vmax - 1) / (8 * vmax), total = 0;
            for (size_t c = 0; c < nc; c++) {
                size_t hs = max_z(f[7 + 3 * c] >> 4, 1), vs = max_z(f[7 + 3 * c] & 15, 1);
                size_t plane = mx * hs * 8 * my * vs * 8 + 15;
                total += plane + (progressive ? 2 * plane : 0) + w + 3;
            }
            return total + w * h * (nc >= 3 ? 3 : 1) + sizeof(stbi__jpeg);
        }
        i += 2 + seg;
    }
    return 0;
}

size_t bgimage_decode_cost(const uint8_t *data, size_t len, size_t file_len) {
    int x = 0, y = 0, n = 0;
    if (!data || len < 8 || len > (size_t)INT_MAX) return 0;
    stb_begin(SIZE_MAX);
    bool info = stbi_info_from_memory(data, (int)len, &x, &y, &n) != 0;
    bool is16 = info && stbi_is_16_bit_from_memory(data, (int)len);
    stb_end();
    if (!info || x < 1 || y < 1) return 0;
    size_t w = (size_t)x, h = (size_t)y, cost = 0;
    if (data[0] == 0x89 && data[1] == 'P') cost = png_cost(data, len, file_len, w, h, n);
    else if (data[0] == 0xFF && data[1] == 0xD8) cost = jpeg_cost(data, len, w, h);
    else cost = w * h * (size_t)n * (is16 ? 3 : 1);  // PNM: the picture, and cut to 8 bits beside it
    return cost ? cost + (64u << 10) : 0;              // (stb_image's own state)
}

static int check_within(const uint8_t *data, size_t len, size_t file_len, size_t budget, int *w, int *h, size_t *cost,
                        const char **why) {
    int x = 0, y = 0, comp = 0;
    // (stb_image tries every format it has, so its reason is the last one's: not worth giving.)
    *why = "not a PNG, JPEG or PNM picture of at most 8192 pixels a side";
    if (!data || len == 0 || len > (size_t)INT_MAX) return -1;
    stb_begin(SIZE_MAX);
    bool info = stbi_info_from_memory(data, (int)len, &x, &y, &comp) != 0;
    stb_end();
    if (!info) return -1;
    if (x < 1 || y < 1 || x > STBI_MAX_DIMENSIONS || y > STBI_MAX_DIMENSIONS) {
        *why = "more than 8192 pixels a side";
        return -1;
    }
    if ((long)x * (long)y > BG_MAX_PIXELS) {
        *why = "more than 40 megapixels";
        return -1;
    }
    *cost = file_len <= budget ? bgimage_decode_cost(data, len, file_len) : 0;
    if (*cost == 0) return -1;  // (a header stb_image takes and this does not read: none is known)
    if (*cost > budget - file_len) {
        *why = "too big to decode in the memory Grimoire gives a picture (a 16-bit or progressive one this size)";
        return -1;
    }
    *w = x;
    *h = y;
    return 0;
}

int bgimage_check(const uint8_t *data, size_t len, size_t file_len, int *w, int *h, const char **why) {
    size_t cost;
    return check_within(data, len, file_len, BG_DECODE_BUDGET, w, h, &cost, why);
}

// A channel over white by its alpha.
static inline uint8_t over_white(int v, int a) { return (uint8_t)((v * a + 255 * (255 - a) + 127) / 255); }

// bgimage_decode within budget bytes, the data's own included.
static int decode_within(const uint8_t *data, size_t len, size_t budget, BgPicture *out, const char **why) {
    memset(out, 0, sizeof *out);
    int w, h;
    size_t cost;
    if (check_within(data, len, len, budget, &w, &h, &cost, why) != 0) return -1;
    // stb_image may hold twice what the header says it needs (that is already the most it takes),
    // never more than the budget leaves beside the data: a picture whose data inflates past that
    // (a "zlib bomb") fails as it gets there.
    size_t room = budget - len, want = 2 * cost + (1u << 20);  // (check_within: cost + len <= budget)
    stb_begin(want < room ? want : room);
    int n = 0;
    uint8_t *px = stbi_load_from_memory(data, (int)len, &w, &h, &n, 0);
    bool over = g_stb_over;
    stb_end();
    if (!px) {
        const char *r = stbi_failure_reason();
        if (over) {
            *why = "a damaged picture (it takes more memory to decode than its size says)";
            return -1;
        }
        if (r && !strcmp(r, "outofmem")) {
            *why = "out of memory";
            return -2;
        }
        *why = "a damaged or unsupported picture";
        return -1;
    }
    size_t np = (size_t)w * (size_t)h;
    int ch = n >= 3 ? 3 : 1;
    // Transparency over white, in place (each pixel moves to a lower or the same offset).
    if (n == 2)
        for (size_t i = 0; i < np; i++) px[i] = over_white(px[2 * i], px[2 * i + 1]);
    else if (n == 4)
        for (size_t i = 0; i < np; i++) {
            uint8_t a = px[4 * i + 3];
            for (int k = 0; k < 3; k++) px[3 * i + (size_t)k] = over_white(px[4 * i + (size_t)k], a);
        }
    if (ch == 3) {
        bool gray = true;
        for (size_t i = 0; i < np && gray; i++) gray = px[3 * i] == px[3 * i + 1] && px[3 * i + 1] == px[3 * i + 2];
        if (gray) {
            for (size_t i = 0; i < np; i++) px[i] = px[3 * i];
            ch = 1;
        }
    }
    uint8_t *fit = realloc(px, np * (size_t)ch);  // only ever smaller
    out->px = fit ? fit : px;
    out->w = w;
    out->h = h;
    out->ch = ch;
    return 0;
}

int bgimage_decode(const uint8_t *data, size_t len, BgPicture *out, const char **why) {
    return decode_within(data, len, BG_DECODE_BUDGET, out, why);
}

// ---- fitting: the area filter ----

// Where a sw x sh picture goes on a dw x dh page: fitted inside it, aspect kept (rounded to whole
// pixels), centred.
static void fit_box(int sw, int sh, int dw, int dh, Rect *box) {
    int fw, fh;
    if ((int64_t)dw * sh <= (int64_t)dh * sw) {  // as wide as the page
        fw = dw;
        fh = (int)(((int64_t)sh * dw * 2 + sw) / ((int64_t)sw * 2));
    } else {                                     // as tall as the page
        fh = dh;
        fw = (int)(((int64_t)sw * dh * 2 + sh) / ((int64_t)sh * 2));
    }
    fw = clampi(fw, 1, dw);
    fh = clampi(fh, 1, dh);
    *box = (Rect){(dw - fw) / 2, (dh - fh) / 2, fw, fh};
}

// Destination pixel u of n covers source pixels [u * s / n, (u + 1) * s / n) of s, each by the part
// of it inside: taps first..first + count - 1, their weights at wt[at] on. The weights are 12-bit
// fixed point adding up to 4096 exactly, rounded from the running total, so a pixel's weights
// depend on u, s and n alone.
typedef struct {
    int first, count;
    size_t at;
} Taps;
#define TAP_ONE 4096u

static int taps_make(int s, int n, int u0, int u1, Taps *tp, uint16_t **wt_out) {
    size_t total = 0;
    for (int u = u0; u < u1; u++) {
        int64_t lo = (int64_t)u * s, hi = lo + s;
        total += (size_t)((hi - 1) / n - lo / n + 1);
    }
    uint16_t *wt = malloc(total ? total * sizeof *wt : 1);
    if (!wt) return -1;
    size_t at = 0;
    for (int u = u0; u < u1; u++) {
        int64_t lo = (int64_t)u * s, hi = lo + s;
        int i0 = (int)(lo / n), i1 = (int)((hi - 1) / n);
        tp[u - u0] = (Taps){i0, i1 - i0 + 1, at};
        int64_t cum = 0;
        int prev = 0;
        for (int i = i0; i <= i1; i++) {
            int64_t a = (int64_t)i * n > lo ? (int64_t)i * n : lo;
            int64_t b = (int64_t)(i + 1) * n < hi ? (int64_t)(i + 1) * n : hi;
            cum += b - a;
            int q = (int)((cum * (int64_t)TAP_ONE + s / 2) / s);
            wt[at++] = (uint16_t)(q - prev);
            prev = q;
        }
    }
    *wt_out = wt;
    return 0;
}

// Draws channel c of src (sw x sh, sch channels a pixel, sstride bytes a row) fitted inside a
// dw x dh page into rect r of that page in dst (dch channels a pixel, channel c, dstride bytes a
// row, dst at the page's top left): white outside the picture. Each pixel depends on its page
// coordinates alone, so a region gets exactly what the whole page gets there. 0, or -1 out of
// memory.
static int fit_draw(const uint8_t *src, int sw, int sh, size_t sstride, int sch, int c, uint8_t *dst, size_t dstride,
                    int dch, int dw, int dh, Rect r) {
    Rect box;
    fit_box(sw, sh, dw, dh, &box);
    r = rect_intersect(r, (Rect){0, 0, dw, dh});
    for (int y = r.y; y < r.y + r.h; y++) {
        uint8_t *row = dst + (size_t)y * dstride + (size_t)c;
        bool band = y < box.y || y >= box.y + box.h;
        for (int x = r.x; x < r.x + r.w; x++)
            if (band || x < box.x || x >= box.x + box.w) row[(size_t)x * (size_t)dch] = 255;
    }
    Rect in = rect_intersect(r, box);
    if (rect_is_empty(in)) return 0;
    int u0 = in.x - box.x, v0 = in.y - box.y;
    Taps *tx = malloc((size_t)in.w * sizeof *tx), *ty = malloc((size_t)in.h * sizeof *ty);
    uint32_t *acc = malloc((size_t)in.w * sizeof *acc);
    uint16_t *wx = NULL, *wy = NULL;
    int rc = -1;
    if (!tx || !ty || !acc || taps_make(sw, box.w, u0, u0 + in.w, tx, &wx) != 0 ||
        taps_make(sh, box.h, v0, v0 + in.h, ty, &wy) != 0)
        goto done;
    for (int v = 0; v < in.h; v++) {
        memset(acc, 0, (size_t)in.w * sizeof *acc);
        const Taps *t = &ty[v];
        for (int k = 0; k < t->count; k++) {
            uint32_t a = wy[t->at + (size_t)k];
            if (!a) continue;
            const uint8_t *row = src + (size_t)(t->first + k) * sstride + (size_t)c;
            for (int u = 0; u < in.w; u++) {
                const Taps *x = &tx[u];
                const uint16_t *w = wx + x->at;
                const uint8_t *p = row + (size_t)x->first * (size_t)sch;
                uint32_t hsum = 0;  // at most 255 * 4096
                for (int m = 0; m < x->count; m++) hsum += (uint32_t)w[m] * p[(size_t)m * (size_t)sch];
                acc[u] += a * hsum;  // in all at most 255 * 4096 * 4096 < 2^32
            }
        }
        uint8_t *out = dst + (size_t)(in.y + v) * dstride + (size_t)in.x * (size_t)dch + (size_t)c;
        for (int u = 0; u < in.w; u++) out[(size_t)u * (size_t)dch] = (uint8_t)((acc[u] + (1u << 23)) >> 24);
    }
    rc = 0;
done:
    free(tx);
    free(ty);
    free(acc);
    free(wx);
    free(wy);
    return rc;
}

int bgimage_normalize(const BgPicture *src, int pw, int ph, BgPicture *out) {
    memset(out, 0, sizeof *out);
    if (!src->px || src->w < 1 || src->h < 1 || (src->ch != 1 && src->ch != 3) || pw < 1 || ph < 1) return -1;
    size_t stride = (size_t)pw * (size_t)src->ch;
    uint8_t *px = malloc(stride * (size_t)ph);
    if (!px) return -1;
    for (int c = 0; c < src->ch; c++)
        if (fit_draw(src->px, src->w, src->h, (size_t)src->w * (size_t)src->ch, src->ch, c, px, stride, src->ch, pw, ph,
                     (Rect){0, 0, pw, ph}) != 0) {
            free(px);
            return -1;
        }
    *out = (BgPicture){px, pw, ph, src->ch};
    return 0;
}

int bgimage_luma(const BgPicture *p, Canvas *out) {
    if (canvas_init(out, p->w, p->h) != 0) return -1;
    size_t np = (size_t)p->w * (size_t)p->h;
    if (p->ch == 1) {
        memcpy(out->px, p->px, np);
        return 0;
    }
    for (size_t i = 0; i < np; i++) {
        const uint8_t *q = p->px + 3 * i;
        out->px[i] = (uint8_t)((2126u * q[0] + 7152u * q[1] + 722u * q[2] + 5000u) / 10000u);
    }
    return 0;
}

int bgimage_write_png(const char *path, const BgPicture *p) {
    int len = 0;
    unsigned char *png = stbi_write_png_to_mem(p->px, p->w * p->ch, p->w, p->h, p->ch, &len);
    if (!png) return -1;
    int rc = write_file_atomic(path, png, (size_t)len);
    free(png);
    return rc;
}

// ---- content keys ----

static uint64_t mix(uint64_t h, uint64_t v) {
    h ^= v;
    h *= 0x9E3779B97F4A7C15ull;
    return h ^ h >> 29;
}

static uint64_t hash_bytes(uint64_t h, const uint8_t *d, size_t n) {
    for (; n >= 8; d += 8, n -= 8) {
        uint64_t v = 0;
        for (int i = 7; i >= 0; i--) v = v << 8 | d[i];  // little-endian, on any CPU
        h = mix(h, v);
    }
    uint64_t v = (uint64_t)n << 56;
    for (size_t i = 0; i < n; i++) v |= (uint64_t)d[i] << (8 * i);
    return mix(h, v);
}

static uint64_t finish(uint64_t h) {
    h ^= h >> 33;
    h *= 0xFF51AFD7ED558CCDull;
    h ^= h >> 33;
    h *= 0xC4CEB9FE1A85EC53ull;
    return h ^ h >> 33;
}

static uint64_t seed(const char *what) { return hash_bytes(0x243F6A8885A308D3ull, (const uint8_t *)what, strlen(what)); }

uint64_t bgimage_key(const BgPicture *p) {
    uint64_t h = seed("grimoire picture 1");
    h = mix(mix(mix(h, (uint64_t)p->w), (uint64_t)p->h), (uint64_t)p->ch);
    return finish(hash_bytes(h, p->px, (size_t)p->w * (size_t)p->h * (size_t)p->ch));
}

uint64_t bgimage_content_hash(const uint8_t *data, size_t len) {
    return finish(hash_bytes(mix(seed("grimoire file 1"), (uint64_t)len), data, len));
}

int bgimage_content_hash_file(const char *path, uint64_t *hash, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    struct stat st;
    if (fstat(fileno(f), &st) != 0 || !S_ISREG(st.st_mode) || st.st_size < 0) {
        fclose(f);
        return -1;
    }
    size_t total = (size_t)st.st_size, got = 0;
    uint64_t h = mix(seed("grimoire file 1"), (uint64_t)total);
    enum { PIECE = 1 << 16 };  // a multiple of 8: whole words until the last piece
    uint8_t *buf = malloc(PIECE);
    size_t n = 0;
    while (buf && (n = fread(buf, 1, PIECE, f)) == PIECE) {
        for (size_t i = 0; i < PIECE; i += 8) {
            uint64_t v = 0;
            for (int k = 7; k >= 0; k--) v = v << 8 | buf[i + (size_t)k];
            h = mix(h, v);
        }
        got += PIECE;
    }
    bool ok = buf && !ferror(f) && got + n == total;
    if (ok) {
        *hash = finish(hash_bytes(h, buf, n));  // the rest: its whole words, then the tail
        *len = total;
    }
    free(buf);
    fclose(f);
    return ok ? 0 : -1;
}

uint64_t bgimage_template_key(uint64_t content, size_t len, int pw, int ph) {
    uint64_t h = seed("grimoire template 2");
    return finish(mix(mix(mix(mix(h, (uint64_t)pw), (uint64_t)ph), (uint64_t)len), content));
}

uint64_t bgimage_source_key(const uint8_t *data, size_t len, int pw, int ph) {
    return bgimage_template_key(bgimage_content_hash(data, len), len, pw, ph);
}

// ---- bg/index.ini ----

static void index_path(const char *bgdir, char *buf, size_t cap) { snprintf(buf, cap, "%s/index.ini", bgdir); }

static int index_add(BgIndex *ix, uint32_t id, uint64_t key) {
    if (ix->n == ix->cap) {
        int nc = ix->cap ? ix->cap * 2 : 16;
        BgIndexEntry *ne = realloc(ix->e, (size_t)nc * sizeof *ne);
        if (!ne) return -1;
        ix->e = ne;
        ix->cap = nc;
    }
    ix->e[ix->n++] = (BgIndexEntry){id, key};
    return 0;
}

int bgindex_load(const char *bgdir, BgIndex *ix) {
    memset(ix, 0, sizeof *ix);
    char path[700];
    index_path(bgdir, path, sizeof path);
    size_t len;
    char *txt = read_file(path, &len);
    if (!txt) return access(path, F_OK) == 0 ? -1 : 0;  // there but unreadable: not to be saved over
    int rc = 0;
    char *save = NULL;
    for (char *line = strtok_r(txt, "\n", &save); line && rc == 0; line = strtok_r(NULL, "\n", &save)) {
        if (line[0] == '#') continue;
        char *eq = strchr(line, '='), *end;
        if (!eq) continue;
        unsigned long id = strtoul(line, &end, 16);
        if (end != eq || end == line || id == 0 || id > 0xFFFFFFul) continue;
        unsigned long long key = strtoull(eq + 1, &end, 16);
        if (end == eq + 1) continue;
        uint32_t have;
        if (bgindex_find(ix, (uint64_t)key, &have)) continue;  // (the first line for a key counts)
        bool taken = false;
        for (int i = 0; i < ix->n && !taken; i++) taken = ix->e[i].id == (uint32_t)id;
        if (!taken) rc = index_add(ix, (uint32_t)id, (uint64_t)key);
    }
    free(txt);
    if (rc) bgindex_free(ix);
    return rc;
}

static int cmp_entry(const void *a, const void *b) {
    const BgIndexEntry *x = a, *y = b;
    return x->id < y->id ? -1 : x->id > y->id;
}

int bgindex_save(const char *bgdir, const BgIndex *ix) {
    BgIndexEntry *e = malloc((size_t)(ix->n ? ix->n : 1) * sizeof *e);
    size_t cap = 96 + (size_t)ix->n * 32;
    char *buf = malloc(cap);
    int rc = -1;
    if (e && buf) {
        if (ix->n) memcpy(e, ix->e, (size_t)ix->n * sizeof *e);
        qsort(e, (size_t)ix->n, sizeof *e, cmp_entry);
        size_t len = (size_t)snprintf(buf, cap, "# Grimoire background pictures: id=key (docs/FORMAT.md)\n");
        for (int i = 0; i < ix->n; i++)
            len += (size_t)snprintf(buf + len, cap - len, "%06x=%016llx\n", (unsigned)e[i].id, (unsigned long long)e[i].key);
        char path[700];
        index_path(bgdir, path, sizeof path);
        rc = write_file_atomic(path, buf, len);
    }
    free(e);
    free(buf);
    return rc;
}

void bgindex_free(BgIndex *ix) {
    free(ix->e);
    memset(ix, 0, sizeof *ix);
}

bool bgindex_find(const BgIndex *ix, uint64_t key, uint32_t *id) {
    for (int i = 0; i < ix->n; i++)
        if (ix->e[i].key == key) {
            *id = ix->e[i].id;
            return true;
        }
    return false;
}

int bgindex_claim(BgIndex *ix, const char *bgdir, uint64_t key, uint32_t *id, bool *added) {
    *added = false;
    if (bgindex_find(ix, key, id)) return 0;
    uint32_t cand = (uint32_t)key & 0xFFFFFFu;
    for (uint32_t tries = 0; tries < 0xFFFFFFu; tries++, cand = (cand + 1) & 0xFFFFFFu) {
        if (cand == 0) cand = 1;
        bool taken = false;
        for (int i = 0; i < ix->n && !taken; i++) taken = ix->e[i].id == cand;
        if (taken) continue;
        char path[700];
        bgimage_file(bgdir, cand, path, sizeof path);
        if (access(path, F_OK) == 0) continue;  // a picture no entry names: whatever it holds, not ours
        if (index_add(ix, cand, key) != 0) return -1;
        *id = cand;
        *added = true;
        return 0;
    }
    return -1;
}

// ---- drawing ----

#define BG_SLOTS 3      // decoded pictures kept: about 4.6 MB each at 1860x2480
#define BG_FAILED 16    // pictures remembered as missing or unreadable

static struct {
    char nb[PAGE_NB_LEN];
    uint32_t id;
    Canvas img;      // its luma; px NULL: a free slot
    uint32_t used;   // for the least recently used
} g_slot[BG_SLOTS];
static uint32_t g_clock;
static struct {
    char nb[PAGE_NB_LEN];
    uint32_t id;
} g_failed[BG_FAILED];
static int g_nfailed, g_failed_next;
static bool g_missing;

bool bgimage_take_missing(void) {
    bool m = g_missing;
    g_missing = false;
    return m;
}

void bgimage_forget(const char *nb) {
    for (int i = 0; i < BG_SLOTS; i++)
        if (g_slot[i].img.px && (!nb || !strcmp(g_slot[i].nb, nb))) canvas_free(&g_slot[i].img);
    if (!nb) {
        g_nfailed = g_failed_next = 0;
        return;
    }
    int m = 0;
    for (int i = 0; i < g_nfailed; i++)
        if (strcmp(g_failed[i].nb, nb)) g_failed[m++] = g_failed[i];
    g_nfailed = m;
    g_failed_next = m % BG_FAILED;
}

void bgimage_retry_missing(void) { g_nfailed = g_failed_next = 0; }

static void remember_failure(const char *nb, uint32_t id) {
    strlcpy_q(g_failed[g_failed_next].nb, nb, PAGE_NB_LEN);
    g_failed[g_failed_next].id = id;
    g_failed_next = (g_failed_next + 1) % BG_FAILED;
    if (g_nfailed < BG_FAILED) g_nfailed++;
}

// Reads notebook nb's picture id as its luma. A kept picture is a PNG of about a page: a file that
// is no PNG (Grimoire writes no other), or one bigger than a page picture can be, is not decoded on
// the UI thread.
static int load_picture(const char *nb, uint32_t id, Canvas *out, const char **why) {
    static const uint8_t png_sig[8] = {0x89, 'P', 'N', 'G', '\r', '\n', 0x1A, '\n'};
    char dir[700], path[800];
    *why = "not in this notebook";
    if (bgimage_dir(nb, dir, sizeof dir) != 0) return -1;
    bgimage_file(dir, id, path, sizeof path);
    size_t len;
    uint8_t *data = bgimage_read(path, BG_UI_FILE_MAX, &len);
    if (!data) {
        *why = errno == EFBIG ? "bigger than a page picture can be" : "missing";
        return -1;
    }
    int w, h;
    size_t cost;
    BgPicture p;
    int rc = -1;
    if (len < 8 || memcmp(data, png_sig, 8)) {
        *why = "not a PNG";
    } else if (check_within(data, len, len, BG_UI_DECODE_BUDGET, &w, &h, &cost, why) != 0) {
        // (*why says what)
    } else if (w > 6000 || h > 6000) {
        *why = "bigger than any page";
    } else if (decode_within(data, len, BG_UI_DECODE_BUDGET, &p, why) == 0) {
        rc = bgimage_luma(&p, out);
        if (rc) *why = "out of memory";
        bgpicture_free(&p);
    }
    free(data);
    return rc;
}

// What load_picture asks of a file before it decodes it.
int bgimage_drawable(const uint8_t *data, size_t len, int *w, int *h, const char **why) {
    static const uint8_t png_sig[8] = {0x89, 'P', 'N', 'G', '\r', '\n', 0x1A, '\n'};
    size_t cost;
    *why = "bigger than a page picture can be";
    if (len > BG_UI_FILE_MAX) return -1;
    *why = "not a PNG";
    if (!data || len < 8 || memcmp(data, png_sig, 8)) return -1;
    if (check_within(data, len, len, BG_UI_DECODE_BUDGET, w, h, &cost, why) != 0) return -1;
    *why = "bigger than any page";
    return *w > 6000 || *h > 6000 ? -1 : 0;
}

// Notebook nb's picture id, decoded, or NULL (missing or unreadable: remembered).
static const Canvas *picture(const char *nb, uint32_t id) {
    for (int i = 0; i < BG_SLOTS; i++)
        if (g_slot[i].img.px && g_slot[i].id == id && !strcmp(g_slot[i].nb, nb)) {
            g_slot[i].used = ++g_clock;
            return &g_slot[i].img;
        }
    for (int i = 0; i < g_nfailed; i++)
        if (g_failed[i].id == id && !strcmp(g_failed[i].nb, nb)) return NULL;
    int s = 0;  // a free slot, else the least recently used
    for (int i = 0; i < BG_SLOTS; i++) {
        if (!g_slot[i].img.px) {
            s = i;
            break;
        }
        if (g_slot[i].used < g_slot[s].used) s = i;
    }
    canvas_free(&g_slot[s].img);
    const char *why;
    if (load_picture(nb, id, &g_slot[s].img, &why) != 0) {
        canvas_free(&g_slot[s].img);
        qlog("bg: %s: bg/%06x.png is %s; its pages show white paper", nb, (unsigned)id, why);
        remember_failure(nb, id);
        return NULL;
    }
    strlcpy_q(g_slot[s].nb, nb, PAGE_NB_LEN);
    g_slot[s].id = id;
    g_slot[s].used = ++g_clock;
    return &g_slot[s].img;
}

void bgimage_render(Canvas *c, const char *nb, PaperKind k, Rect region) {
    Rect r = rect_intersect(region, c->clip);
    if (rect_is_empty(r)) return;
    const Canvas *img = paper_is_image(k) && nb && *nb ? picture(nb, paper_image_id(k)) : NULL;
    if (!img) {
        canvas_fill(c, r, 255);
        g_missing = true;
        return;
    }
    if (img->w == c->w && img->h == c->h) {
        for (int y = r.y; y < r.y + r.h; y++)
            memcpy(&c->px[(size_t)y * (size_t)c->stride + (size_t)r.x], &img->px[(size_t)y * (size_t)img->stride + (size_t)r.x],
                   (size_t)r.w);
        return;
    }
    // A picture made for a page of another size (a notebook an older version moved to another
    // screen): fitted as it was made, from page coordinates; white if there is no memory for it.
    if (fit_draw(img->px, img->w, img->h, (size_t)img->stride, 1, 0, c->px, (size_t)c->stride, 1, c->w, c->h, r) != 0)
        canvas_fill(c, r, 255);
}
