// export.c: PNG / PDF / zip output for notebooks.
#include "export.h"

#include <dirent.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "stb_image_write.h"

// Defined in stb_image_write's implementation section (stb_impl.c) but not in its API header.
unsigned char *stbi_write_png_to_mem(const unsigned char *pixels, int stride_bytes, int x, int y, int n, int *out_len);
unsigned char *stbi_zlib_compress(unsigned char *data, int data_len, int *out_len, int quality);

// ---- Buf ----

void buf_free(Buf *b) {
    free(b->data);
    memset(b, 0, sizeof *b);
}

int buf_append(Buf *b, const void *p, size_t n) {
    if (b->len + n > b->cap) {
        size_t nc = b->cap ? b->cap : 4096;
        while (nc < b->len + n) nc *= 2;
        uint8_t *nd = realloc(b->data, nc);
        if (!nd) {
            b->err = true;
            return -1;
        }
        b->data = nd;
        b->cap = nc;
    }
    if (n) memcpy(b->data + b->len, p, n);
    b->len += n;
    return 0;
}

int buf_printf(Buf *b, const char *fmt, ...) {
    char tmp[1024];
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(tmp, sizeof tmp, fmt, ap);
    va_end(ap);
    if (n < 0) return -1;
    if ((size_t)n < sizeof tmp) return buf_append(b, tmp, (size_t)n);
    char *big = malloc((size_t)n + 1);
    if (!big) return -1;
    va_start(ap, fmt);
    vsnprintf(big, (size_t)n + 1, fmt, ap);
    va_end(ap);
    int rc = buf_append(b, big, (size_t)n);
    free(big);
    return rc;
}

void export_safe_name(const char *in, char *out, size_t cap) {
    size_t n = 0;
    for (const char *c = in ? in : ""; *c && n + 1 < cap; c++) {
        unsigned char ch = (unsigned char)*c;
        out[n++] = (ch < 32 || strchr("/\\:*?\"<>|", ch)) ? '_' : (char)ch;
    }
    out[n] = 0;
    // Trim leading/trailing spaces and dots (hidden files, Windows quirks).
    size_t s = 0;
    while (out[s] == ' ' || out[s] == '.') s++;
    if (s) memmove(out, out + s, n - s + 1), n -= s;
    while (n && (out[n - 1] == ' ' || out[n - 1] == '.')) out[--n] = 0;
    if (!n) strlcpy_q(out, "Untitled", cap);
}

// ---- rendering helpers ----

static int render_page(const NotebookInfo *nb, int index, float dpi, int layers, Canvas *c, uint32_t *paper) {
    // Rendered at the notebook's page size, as the editor has the page: one an older version scaled
    // to another screen goes back to that size, and a missing or damaged page is blank paper of it.
    Page *p = NULL;
    if (store_read_page(nb, index, &p) != 0) return -1;
    // A page bigger than any e-ink screen is damaged or hostile: do not try to allocate it.
    if (p && (p->w > 6000 || p->h > 6000)) {
        page_free(p);
        return -1;
    }
    if (!p || canvas_init(c, p->w, p->h) != 0) {
        page_free(p);
        return -1;
    }
    page_render_layers(c, p, (Rect){0, 0, p->w, p->h}, dpi, layers);
    if (paper) *paper = (uint32_t)p->paper;
    page_free(p);
    return 0;
}

int export_page_png(const NotebookInfo *nb, int index, float dpi, Buf *out) {
    return export_page_png_ex(nb, index, dpi, PAGE_LAYER_ALL, 1, NULL, out);
}

// The box of the pixels darker than 250 (the studio's "ink" threshold too), empty if none.
static Rect dark_box(const Canvas *c) {
    int x0 = c->w, y0 = c->h, x1 = -1, y1 = -1;
    for (int y = 0; y < c->h; y++) {
        const uint8_t *row = c->px + y * c->stride;
        int first = -1, last = -1;
        for (int x = 0; x < c->w; x++)
            if (row[x] < 250) {
                if (first < 0) first = x;
                last = x;
            }
        if (first < 0) continue;
        if (first < x0) x0 = first;
        if (last > x1) x1 = last;
        if (y < y0) y0 = y;
        y1 = y;
    }
    return x1 < 0 ? (Rect){0, 0, 0, 0} : (Rect){x0, y0, x1 - x0 + 1, y1 - y0 + 1};
}

int export_page_png_ex(const NotebookInfo *nb, int index, float dpi, int layers, int shrink, PageRender *info,
                       Buf *out) {
    Canvas c;
    uint32_t paper = 0;
    if (index < 0 || index >= nb->npages || shrink < 1 || render_page(nb, index, dpi, layers, &c, &paper) != 0)
        return -1;
    if (info) *info = (PageRender){c.w, c.h, paper, layers == PAGE_LAYER_INK ? dark_box(&c) : (Rect){0, 0, 0, 0}};
    if (shrink > 1) {  // whole shrink x shrink blocks from the top left; leftover edge pixels drop
        Canvas s;
        if (c.w / shrink < 1 || c.h / shrink < 1 || canvas_init(&s, c.w / shrink, c.h / shrink) != 0) {
            canvas_free(&c);
            return -1;
        }
        int n = shrink * shrink;
        for (int y = 0; y < s.h; y++)
            for (int x = 0; x < s.w; x++) {
                int sum = 0;
                for (int j = 0; j < shrink; j++) {
                    const uint8_t *row = c.px + (y * shrink + j) * c.stride + x * shrink;
                    for (int i = 0; i < shrink; i++) sum += row[i];
                }
                s.px[y * s.stride + x] = (uint8_t)((sum + n / 2) / n);
            }
        canvas_free(&c);
        c = s;
    }
    int len = 0;
    unsigned char *png = stbi_write_png_to_mem(c.px, c.stride, c.w, c.h, 1, &len);
    canvas_free(&c);
    if (!png) return -1;
    int rc = buf_append(out, png, (size_t)len);
    free(png);
    return rc;
}

// ---- PDF ----

// PDF text string as UTF-16BE hex with BOM (handles any UTF-8 name).
static void pdf_text(Buf *b, const char *s) {
    buf_printf(b, "<FEFF");
    const unsigned char *p = (const unsigned char *)s;
    while (*p) {
        uint32_t c = *p++;
        int extra = c >= 0xF0 ? 3 : c >= 0xE0 ? 2 : c >= 0xC0 ? 1 : 0;
        if (extra) c &= (0x3F >> extra);
        while (extra-- && (*p & 0xC0) == 0x80) c = (c << 6) | (*p++ & 0x3F);
        if (c >= 0x10000) {
            c -= 0x10000;
            buf_printf(b, "%04X%04X", 0xD800 + (c >> 10), 0xDC00 + (c & 0x3FF));
        } else
            buf_printf(b, "%04X", c);
    }
    buf_printf(b, ">");
}

int export_notebook_pdf(const NotebookInfo *nb, float dpi, Buf *out) {
    int np = nb->npages;
    // Objects: 1 catalog, 2 pages, 3 info, then 3 per page (page, content, image).
    int nobj = 3 + 3 * np;
    size_t *off = calloc((size_t)nobj + 1, sizeof *off);
    if (!off) return -1;
    size_t base = out->len;
    int rc = 0;
#define OBJ(i) (off[i] = out->len - base, buf_printf(out, "%d 0 obj\n", (i)))
    buf_printf(out, "%%PDF-1.4\n%%\xE2\xE3\xCF\xD3\n");
    OBJ(1);
    buf_printf(out, "<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    OBJ(2);
    buf_printf(out, "<< /Type /Pages /Count %d /Kids [", np);
    for (int i = 0; i < np; i++) buf_printf(out, " %d 0 R", 4 + 3 * i);
    buf_printf(out, " ] >>\nendobj\n");
    OBJ(3);
    buf_printf(out, "<< /Title ");
    pdf_text(out, nb->name);
    buf_printf(out, " /Creator (Grimoire %s) /Producer (Grimoire) >>\nendobj\n", GRIMOIRE_VERSION);
    for (int i = 0; i < np && rc == 0; i++) {
        Canvas c;
        if (render_page(nb, i, dpi, PAGE_LAYER_ALL, &c, NULL) != 0) {
            rc = -1;
            break;
        }
        float pw = (float)c.w * 72.f / dpi, ph = (float)c.h * 72.f / dpi;
        int pobj = 4 + 3 * i, cobj = pobj + 1, iobj = pobj + 2;
        OBJ(pobj);
        buf_printf(out,
                   "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %.2f %.2f] /Contents %d 0 R "
                   "/Resources << /XObject << /Im0 %d 0 R >> >> >>\nendobj\n",
                   pw, ph, cobj, iobj);
        char content[128];
        int cl = snprintf(content, sizeof content, "q %.2f 0 0 %.2f 0 0 cm /Im0 Do Q\n", pw, ph);
        OBJ(cobj);
        buf_printf(out, "<< /Length %d >>\nstream\n%s\nendstream\nendobj\n", cl, content);
        int zl = 0;
        unsigned char *z = stbi_zlib_compress(c.px, c.w * c.h, &zl, 8);
        canvas_free(&c);
        if (!z) {
            rc = -1;
            break;
        }
        OBJ(iobj);
        buf_printf(out,
                   "<< /Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /DeviceGray "
                   "/BitsPerComponent 8 /Filter /FlateDecode /Length %d >>\nstream\n",
                   (int)(pw * dpi / 72.f + 0.5f), (int)(ph * dpi / 72.f + 0.5f), zl);
        buf_append(out, z, (size_t)zl);
        buf_printf(out, "\nendstream\nendobj\n");
        free(z);
    }
#undef OBJ
    if (rc == 0) {
        size_t xref = out->len - base;
        buf_printf(out, "xref\n0 %d\n0000000000 65535 f \n", nobj + 1);
        for (int i = 1; i <= nobj; i++) buf_printf(out, "%010zu 00000 n \n", off[i]);
        buf_printf(out, "trailer\n<< /Size %d /Root 1 0 R /Info 3 0 R >>\nstartxref\n%zu\n%%%%EOF\n", nobj + 1, xref);
    }
    free(off);
    return rc != 0 || out->err ? -1 : 0;
}

// ---- zip ----

static uint32_t crc_table[256];
static void crc_init(void) {
    if (crc_table[1]) return;
    for (uint32_t i = 0; i < 256; i++) {
        uint32_t c = i;
        for (int k = 0; k < 8; k++) c = (c & 1) ? 0xEDB88320u ^ (c >> 1) : c >> 1;
        crc_table[i] = c;
    }
}
static uint32_t crc32_of(const uint8_t *p, size_t n) {
    crc_init();
    uint32_t c = 0xFFFFFFFFu;
    for (size_t i = 0; i < n; i++) c = crc_table[(c ^ p[i]) & 0xFF] ^ (c >> 8);
    return c ^ 0xFFFFFFFFu;
}

static void le16(Buf *b, uint16_t v) {
    uint8_t x[2] = {(uint8_t)v, (uint8_t)(v >> 8)};
    buf_append(b, x, 2);
}
static void le32(Buf *b, uint32_t v) {
    uint8_t x[4] = {(uint8_t)v, (uint8_t)(v >> 8), (uint8_t)(v >> 16), (uint8_t)(v >> 24)};
    buf_append(b, x, 4);
}

static void dos_time(uint16_t *t, uint16_t *d) {
    time_t now = (time_t)wall_time();
    struct tm tm;
    localtime_r(&now, &tm);
    *t = (uint16_t)((tm.tm_hour << 11) | (tm.tm_min << 5) | (tm.tm_sec / 2));
    *d = (uint16_t)(((tm.tm_year - 80) << 9) | ((tm.tm_mon + 1) << 5) | tm.tm_mday);
}

void zip_begin(ZipWriter *z, Buf *out) {
    memset(z, 0, sizeof *z);
    z->out = out;
}

int zip_add(ZipWriter *z, const char *name, const void *data, size_t len, bool deflate) {
    if (len > 0xFFFFFFF0u || z->out->len > 0xFFFFFFF0u || z->count >= 0xFFFF) return -1;  // no zip64
    uint32_t crc = crc32_of(data, len);
    const uint8_t *payload = data;
    size_t plen = len;
    unsigned char *zbuf = NULL;
    uint16_t method = 0;
    if (deflate && len > 64) {
        int zl = 0;
        zbuf = stbi_zlib_compress((unsigned char *)data, (int)len, &zl, 8);
        // Zip wants raw deflate: drop the 2-byte zlib header and 4-byte adler32 trailer.
        if (zbuf && zl > 6 && (size_t)(zl - 6) < len) {
            payload = zbuf + 2;
            plen = (size_t)zl - 6;
            method = 8;
        }
    }
    uint16_t t, d;
    dos_time(&t, &d);
    uint16_t nl = (uint16_t)strlen(name);
    uint32_t local_off = (uint32_t)z->out->len;
    Buf *o = z->out;
    le32(o, 0x04034b50);
    le16(o, 20);
    le16(o, 0x0800);  // UTF-8 names
    le16(o, method);
    le16(o, t);
    le16(o, d);
    le32(o, crc);
    le32(o, (uint32_t)plen);
    le32(o, (uint32_t)len);
    le16(o, nl);
    le16(o, 0);
    buf_append(o, name, nl);
    int rc = buf_append(o, payload, plen);
    Buf *c = &z->central;
    le32(c, 0x02014b50);
    le16(c, 0x0314);  // made by: unix, 2.0
    le16(c, 20);
    le16(c, 0x0800);
    le16(c, method);
    le16(c, t);
    le16(c, d);
    le32(c, crc);
    le32(c, (uint32_t)plen);
    le32(c, (uint32_t)len);
    le16(c, nl);
    le16(c, 0);
    le16(c, 0);
    le16(c, 0);
    le16(c, 0);
    le32(c, 0100644u << 16);
    le32(c, local_off);
    buf_append(c, name, nl);
    free(zbuf);
    z->count++;
    return rc;
}

int zip_end(ZipWriter *z) {
    uint32_t cd_off = (uint32_t)z->out->len;
    buf_append(z->out, z->central.data, z->central.len);
    Buf *o = z->out;
    le32(o, 0x06054b50);
    le16(o, 0);
    le16(o, 0);
    le16(o, (uint16_t)z->count);
    le16(o, (uint16_t)z->count);
    le32(o, (uint32_t)z->central.len);
    le32(o, cd_off);
    int rc = buf_append(o, "\0\0", 2);
    bool bad = z->central.err || o->err;
    buf_free(&z->central);
    return rc != 0 || bad ? -1 : 0;
}

static int cmp_str(const void *a, const void *b) { return strcmp(*(char *const *)a, *(char *const *)b); }

// The notebook's image papers (docs/FORMAT.md): raw/bg/<id>.png, index.ini and pages.ini, so a raw
// copy of the notebook draws its pages as the Kindle does. PNGs are stored, not deflated again.
static int add_raw_pictures(const char *dir, ZipWriter *z, const char *dirprefix) {
    char bg[760];
    snprintf(bg, sizeof bg, "%s/bg", dir);
    DIR *dd = opendir(bg);
    if (!dd) return 0;
    char **names = NULL;
    int n = 0, cap = 0;
    for (struct dirent *e; (e = readdir(dd));) {
        const char *f = e->d_name;  // <6 hex digits>.png
        bool pic = strlen(f) == 10 && !strcmp(f + 6, ".png");
        for (int k = 0; k < 6 && pic; k++) pic = (f[k] >= '0' && f[k] <= '9') || (f[k] >= 'a' && f[k] <= 'f');
        if (!pic && strcmp(f, "index.ini") && strcmp(f, "pages.ini")) continue;
        if (n == cap) {
            int nc = cap ? cap * 2 : 16;
            char **nn = realloc(names, (size_t)nc * sizeof *nn);
            if (!nn) break;
            names = nn;
            cap = nc;
        }
        if (!(names[n] = strdup(e->d_name))) break;
        n++;
    }
    closedir(dd);
    if (n) qsort(names, (size_t)n, sizeof *names, cmp_str);
    for (int i = 0; i < n; i++) {
        char path[1000], name[700];
        size_t len;
        snprintf(path, sizeof path, "%s/%s", bg, names[i]);
        uint8_t *d = read_file(path, &len);
        if (d) {
            snprintf(name, sizeof name, "%sraw/bg/%s", dirprefix, names[i]);
            zip_add(z, name, d, len, !strstr(names[i], ".png"));
            free(d);
        }
        free(names[i]);
    }
    free(names);
    return 0;
}

static int add_raw_files(const NotebookInfo *nb, ZipWriter *z, const char *dirprefix) {
    char dir[700], path[900], name[600];
    store_notebook_dir(nb->id, dir, sizeof dir);
    size_t len;
    snprintf(path, sizeof path, "%s/notebook.ini", dir);
    uint8_t *d = read_file(path, &len);
    if (d) {
        snprintf(name, sizeof name, "%sraw/notebook.ini", dirprefix);
        zip_add(z, name, d, len, true);
        free(d);
    }
    for (int i = 0; i < nb->npages; i++) {
        snprintf(path, sizeof path, "%s/p%u.qpg", dir, nb->page_ids[i]);
        d = read_file(path, &len);
        if (!d) continue;
        snprintf(name, sizeof name, "%sraw/p%u.qpg", dirprefix, nb->page_ids[i]);
        zip_add(z, name, d, len, true);
        free(d);
    }
    return add_raw_pictures(dir, z, dirprefix);
}

static int notebook_into_zip(const NotebookInfo *nb, float dpi, ZipWriter *z, const char *prefix, const char *dirname_,
                             bool pngs) {
    char safe[STORE_NAME_LEN], dirp[400], name[600];
    export_safe_name(nb->name, safe, sizeof safe);
    snprintf(dirp, sizeof dirp, "%s%s/", prefix ? prefix : "", dirname_ ? dirname_ : safe);
    Buf pdf = {0};
    if (export_notebook_pdf(nb, dpi, &pdf) != 0) {
        buf_free(&pdf);
        return -1;
    }
    snprintf(name, sizeof name, "%s%s.pdf", dirp, safe);
    zip_add(z, name, pdf.data, pdf.len, false);
    buf_free(&pdf);
    if (pngs)
        for (int i = 0; i < nb->npages; i++) {
            Buf png = {0};
            if (export_page_png(nb, i, dpi, &png) == 0) {
                snprintf(name, sizeof name, "%spage-%03d.png", dirp, i + 1);
                zip_add(z, name, png.data, png.len, false);
            }
            buf_free(&png);
        }
    return add_raw_files(nb, z, dirp);
}

int export_notebook_zip(const NotebookInfo *nb, float dpi, ZipWriter *z, const char *prefix) {
    return notebook_into_zip(nb, dpi, z, prefix, NULL, true);
}

int export_folder_zip(const char *folder, float dpi, Buf *out) {
    NotebookInfo *nbs;
    int n;
    if (store_list(&nbs, &n) != 0) return -1;
    ZipWriter z;
    zip_begin(&z, out);
    // Track used "<folder>/<name>" paths to disambiguate duplicates.
    char (*used)[400] = calloc((size_t)(n ? n : 1), sizeof *used);
    int nused = 0;
    for (int i = 0; i < n && used; i++) {
        const NotebookInfo *nb = &nbs[i];
        if (folder && *folder && strcmp(nb->folder, folder)) continue;
        char sf[STORE_NAME_LEN] = "", sn[STORE_NAME_LEN], prefix[200], dir[400];
        if ((!folder || !*folder) && nb->folder[0]) {
            export_safe_name(nb->folder, sf, sizeof sf);
            snprintf(prefix, sizeof prefix, "%s/", sf);
        } else
            prefix[0] = 0;
        export_safe_name(nb->name, sn, sizeof sn);
        strlcpy_q(dir, sn, sizeof dir);
        for (int k = 2;; k++) {
            char key[400];
            snprintf(key, sizeof key, "%s%s", prefix, dir);
            bool dup = false;
            for (int u = 0; u < nused && !dup; u++) dup = !strcmp(used[u], key);
            if (!dup) {
                strlcpy_q(used[nused++], key, sizeof used[0]);
                break;
            }
            snprintf(dir, sizeof dir, "%s (%d)", sn, k);
        }
        notebook_into_zip(nb, dpi, &z, prefix, dir, false);
    }
    free(used);
    free(nbs);
    return zip_end(&z);
}

void export_mirror_path(const NotebookInfo *nb, const char *mirror_dir, char *buf, size_t cap) {
    char sf[STORE_NAME_LEN], sn[STORE_NAME_LEN];
    export_safe_name(nb->name, sn, sizeof sn);
    if (nb->folder[0]) {
        export_safe_name(nb->folder, sf, sizeof sf);
        snprintf(buf, cap, "%s/%s/%s.pdf", mirror_dir, sf, sn);
    } else
        snprintf(buf, cap, "%s/%s.pdf", mirror_dir, sn);
}

int export_mirror_pdf(const NotebookInfo *nb, float dpi, const char *mirror_dir) {
    char sf[STORE_NAME_LEN], sn[STORE_NAME_LEN], dir[800], path[1000];
    export_safe_name(nb->name, sn, sizeof sn);
    if (nb->folder[0]) {
        export_safe_name(nb->folder, sf, sizeof sf);
        snprintf(dir, sizeof dir, "%s/%s", mirror_dir, sf);
    } else
        snprintf(dir, sizeof dir, "%s", mirror_dir);
    if (mkdir_p(dir) != 0) return -1;
    snprintf(path, sizeof path, "%s/%s.pdf", dir, sn);
    Buf pdf = {0};
    int rc = export_notebook_pdf(nb, dpi, &pdf);
    if (rc == 0) rc = write_file_atomic(path, pdf.data, pdf.len);
    buf_free(&pdf);
    return rc;
}
