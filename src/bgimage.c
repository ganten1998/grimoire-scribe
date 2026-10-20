// bgimage.c: image papers (see bgimage.h). Pictures are decoded by stb_image (third_party),
// compiled here: PNG, JPEG and PNM only, at most 8192 px a side, no SIMD (every build decodes a
// picture to the same bytes).
#include "bgimage.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "store.h"

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wsign-compare"
#pragma clang diagnostic ignored "-Wunused-function"
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wmissing-field-initializers"
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
#pragma clang diagnostic ignored "-Wimplicit-fallthrough"
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
#include "stb_image.h"
#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

// Defined in stb_image_write's implementation section (stb_impl.c) but not in its API header.
unsigned char *stbi_write_png_to_mem(const unsigned char *pixels, int stride_bytes, int x, int y, int n, int *out_len);

_Static_assert(PAGE_NB_LEN == STORE_ID_LEN, "a page's notebook link holds a notebook id");

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

// ---- decoding ----

int bgimage_check(const uint8_t *data, size_t len, int *w, int *h, const char **why) {
    int x = 0, y = 0, comp = 0;
    // (stb_image tries every format it has, so its reason is the last one's: not worth giving.)
    *why = "not a PNG, JPEG or PNM picture of at most 8192 pixels a side";
    if (!data || len == 0 || len > (size_t)INT_MAX) return -1;
    if (!stbi_info_from_memory(data, (int)len, &x, &y, &comp)) return -1;
    if (x < 1 || y < 1 || x > STBI_MAX_DIMENSIONS || y > STBI_MAX_DIMENSIONS) {
        *why = "more than 8192 pixels a side";
        return -1;
    }
    if ((long)x * (long)y > BG_MAX_PIXELS) {
        *why = "more than 40 megapixels";
        return -1;
    }
    *w = x;
    *h = y;
    return 0;
}

// A channel over white by its alpha.
static inline uint8_t over_white(int v, int a) { return (uint8_t)((v * a + 255 * (255 - a) + 127) / 255); }

int bgimage_decode(const uint8_t *data, size_t len, BgPicture *out, const char **why) {
    memset(out, 0, sizeof *out);
    int w, h;
    if (bgimage_check(data, len, &w, &h, why) != 0) return -1;
    int n = 0;
    uint8_t *px = stbi_load_from_memory(data, (int)len, &w, &h, &n, 0);
    if (!px) {
        const char *r = stbi_failure_reason();
        *why = r && !strcmp(r, "outofmem") ? "out of memory" : "a damaged or unsupported picture";
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

static uint64_t source_start(size_t len, int pw, int ph) {
    uint64_t h = seed("grimoire template 1");
    return mix(mix(mix(h, (uint64_t)pw), (uint64_t)ph), (uint64_t)len);
}

uint64_t bgimage_source_key(const uint8_t *data, size_t len, int pw, int ph) {
    return finish(hash_bytes(source_start(len, pw, ph), data, len));
}

int bgimage_source_key_file(const char *path, int pw, int ph, uint64_t *key, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    struct stat st;
    if (fstat(fileno(f), &st) != 0 || !S_ISREG(st.st_mode) || st.st_size < 0) {
        fclose(f);
        return -1;
    }
    size_t total = (size_t)st.st_size, got = 0;
    uint64_t h = source_start(total, pw, ph);
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
        *key = finish(hash_bytes(h, buf, n));  // the rest: its whole words, then the tail
        *len = total;
    }
    free(buf);
    fclose(f);
    return ok ? 0 : -1;
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
    if (!txt) return 0;
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

// Reads notebook nb's picture id as its luma. A kept picture is page-size: one of more than 6000
// px a side is not one (an e-ink screen has fewer), so it is not decoded on the UI thread.
static int load_picture(const char *nb, uint32_t id, Canvas *out, const char **why) {
    char dir[700], path[800];
    *why = "not in this notebook";
    if (bgimage_dir(nb, dir, sizeof dir) != 0) return -1;
    bgimage_file(dir, id, path, sizeof path);
    size_t len;
    uint8_t *data = read_file(path, &len);
    if (!data) {
        *why = "missing";
        return -1;
    }
    int w, h;
    BgPicture p;
    int rc = -1;
    if (bgimage_check(data, len, &w, &h, why) == 0) {
        if (w > 6000 || h > 6000) *why = "bigger than any page";
        else if (bgimage_decode(data, len, &p, why) == 0) {
            rc = bgimage_luma(&p, out);
            if (rc) *why = "out of memory";
            bgpicture_free(&p);
        }
    }
    free(data);
    return rc;
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
