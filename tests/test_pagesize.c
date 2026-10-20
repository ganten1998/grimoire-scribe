// test_pagesize.c: a notebook keeps the page size it was made with (the Scribe 3 and the Scribe
// Colorsoft are 1986x2648, the 2022 and 2024 Scribes 1860x2480), wherever it is opened; a page an
// older version scaled to another screen goes back to its notebook's size; calibrations follow the
// panel; how many music systems fit is worked out for the page's own height; and which fast
// refreshes a colour panel waits after (all but live ink).
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "calib_fit.h"
#include "display.h"
#include "export.h"
#include "store.h"
#include "testlib.h"

static char root[600];
static const float DPI = 300.f;
enum { W2 = 1860, H2 = 2480, W3 = 1986, H3 = 2648 };  // the Scribe (2022, 2024); the Scribe 3 / Colorsoft

// ---- helpers ----

static Stroke *wave(ToolKind t, float y, uint32_t seed) {
    ToolParams p;
    tool_params_default(t, &p);
    Stroke *s = stroke_new(t, &p, seed);
    for (int i = 0; i < 30; i++) {
        PenSample q = {150.f + (float)i * 55.f, y + (float)(i % 6) * 11.f, 0.15f + (float)i / 40.f, (float)(i % 9) - 4.f,
                       3.f - (float)(i % 5), (uint32_t)i * 7u};
        stroke_append(s, &q, DPI);
    }
    return s;
}

static void page_file(const NotebookInfo *nb, int index, char *path, size_t cap) {
    snprintf(path, cap, "%s/notebooks/%s/p%u.qpg", root, nb->id, nb->page_ids[index]);
}

static void info_file(const NotebookInfo *nb, char *path, size_t cap) {
    snprintf(path, cap, "%s/notebooks/%s/notebook.ini", root, nb->id);
}

// The size in a page file's first 16 bytes; false if it has none.
static bool file_size(const char *path, int *w, int *h) {
    uint8_t b[16];
    int fd = open(path, O_RDONLY);
    if (fd < 0) return false;
    bool ok = read(fd, b, sizeof b) == (ssize_t)sizeof b;
    close(fd);
    if (!ok || memcmp(b, "QPG1", 4)) return false;
    *w = (int)(b[8] | b[9] << 8 | b[10] << 16 | (uint32_t)b[11] << 24);
    *h = (int)(b[12] | b[13] << 8 | b[14] << 16 | (uint32_t)b[15] << 24);
    return true;
}

static char *text_of(const char *path) {
    size_t len;
    return read_file(path, &len);
}

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

// notebook.ini without the lines that start with prefix1 or prefix2 (what 1.0.0-beta's save leaves
// of the keys it does not know).
static void drop_lines(const NotebookInfo *nb, const char *prefix1, const char *prefix2) {
    char path[800];
    info_file(nb, path, sizeof path);
    char *t = text_of(path);
    CHECK(t != NULL);
    if (!t) return;
    size_t cap = strlen(t) + 1, n = 0;
    char *out = calloc(1, cap);
    for (char *line = strtok(t, "\n"); line; line = strtok(NULL, "\n")) {
        if (!strncmp(line, prefix1, strlen(prefix1)) || (prefix2 && !strncmp(line, prefix2, strlen(prefix2)))) continue;
        n += (size_t)snprintf(out + n, cap - n, "%s\n", line);
    }
    CHECK(write_file_atomic(path, out, n) == 0);
    free(out);
    free(t);
}

static void append_line(const NotebookInfo *nb, const char *line) {
    char path[800];
    info_file(nb, path, sizeof path);
    char *t = text_of(path);
    CHECK(t != NULL);
    if (!t) return;
    size_t cap = strlen(t) + strlen(line) + 2;
    char *out = malloc(cap);
    snprintf(out, cap, "%s%s\n", t, line);
    CHECK(write_file_atomic(path, out, strlen(out)) == 0);
    free(out);
    free(t);
}

static bool render(const Page *p, Canvas *c) {
    if (canvas_init(c, p->w, p->h) != 0) return false;
    page_render(c, p, (Rect){0, 0, p->w, p->h}, DPI);
    return true;
}

static bool same_pixels(const Canvas *a, const Canvas *b) {
    if (a->w != b->w || a->h != b->h) return false;
    for (int y = 0; y < a->h; y++)
        if (memcmp(a->px + (size_t)y * a->stride, b->px + (size_t)y * b->stride, (size_t)a->w)) return false;
    return true;
}

// Writes page `index` of nb as a version before 1.1 would after an edit on a screen of w x h: it
// loaded the page scaled to its screen and saved it at that size.
static void older_version_rescales(const NotebookInfo *nb, int index, int w, int h) {
    Page *p = NULL;
    CHECK(store_load_page(nb, index, w, h, &p) == 0 && p && p->w == w && p->h == h);
    if (p) CHECK(store_save_page(nb, index, p) == 0);
    page_free(p);
}

// ---- notebooks keep their size ----

// A notebook made on an 1860x2480 Scribe, opened with the device at 1986x2648: it keeps its size,
// its pages render byte for byte as they did, and new pages get its size, not the screen's.
TEST(notebook_keeps_its_size) {
    store_set_device(DPI, W2, H2);
    NotebookInfo nb;
    CHECK(store_create("Made on a Scribe 2", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(nb.page_w == W2 && nb.page_h == H2);
    CHECK(store_add_page(&nb, 1, PAPER_MUSIC) == 0 && nb.npages == 2);
    const ToolKind tools[2][3] = {{TOOL_PENCIL, TOOL_FOUNTAIN, TOOL_HIGHLIGHTER}, {TOOL_CALLIGRAPHY, TOOL_CHARCOAL, TOOL_MARKER}};
    Canvas ref[2];
    for (int i = 0; i < 2; i++) {
        Page *p = NULL;
        CHECK(store_load_page(&nb, i, W2, H2, &p) == 0 && p);
        if (!p) return;
        for (int k = 0; k < 3; k++) page_add(p, wave(tools[i][k], 400.f + 700.f * (float)k, 11u + (uint32_t)k));
        CHECK(store_save_page(&nb, i, p) == 0);
        page_free(p);
        // How the Scribe 2 shows it, read back (pressure and tilt are stored rounded).
        CHECK(store_load_page(&nb, i, W2, H2, &p) == 0 && p && p->n == 3);
        if (!p || !render(p, &ref[i])) return;
        page_free(p);
    }
    char path[800];
    info_file(&nb, path, sizeof path);
    char *t = text_of(path);
    CHECK(t && has_line(t, "page_w=1860") == 1 && has_line(t, "page_h=2480") == 1);
    free(t);

    // Now on a Scribe Colorsoft.
    store_set_device(DPI, W3, H3);
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    int w = 0, h = 0;
    store_notebook_page_size(&back, &w, &h);
    CHECK(w == W2 && h == H2);
    for (int i = 0; i < 2; i++) {
        Page *p = NULL;
        CHECK(store_load_page(&back, i, w, h, &p) == 0 && p && p->w == W2 && p->h == H2 && p->n == 3);
        Canvas c;
        if (p && render(p, &c)) {
            CHECK(same_pixels(&c, &ref[i]));
            canvas_free(&c);
        }
        page_free(p);
        // Exports read it the same way, with nothing changed on disk.
        CHECK(store_read_page(&back, i, &p) == 0 && p && p->w == W2 && p->h == H2 && p->n == 3);
        if (p && render(p, &c)) {
            CHECK(same_pixels(&c, &ref[i]));
            canvas_free(&c);
        }
        page_free(p);
        Buf png = {0};
        PageRender pr;
        CHECK(export_page_png_ex(&back, i, DPI, PAGE_LAYER_ALL, 1, &pr, &png) == 0 && pr.w == W2 && pr.h == H2);
        buf_free(&png);
    }
    // A new page takes the notebook's size, not the screen's.
    CHECK(store_add_page(&back, back.npages, PAPER_GRID) == 0 && back.npages == 3);
    page_file(&back, 2, path, sizeof path);
    CHECK(file_size(path, &w, &h) && w == W2 && h == H2);
    Page *p = NULL;
    CHECK(store_load_page(&back, 2, W2, H2, &p) == 0 && p && p->w == W2 && p->paper == PAPER_GRID);
    page_free(p);
    // A page whose file is missing is blank paper of the notebook's size, for the editor and exports.
    CHECK(unlink(path) == 0);
    CHECK(store_read_page(&back, 2, &p) == 0 && p && p->w == W2 && p->h == H2 && p->paper == PAPER_MEDIUM);
    page_free(p);
    Buf png = {0};
    PageRender pr;
    CHECK(export_page_png_ex(&back, 2, DPI, PAGE_LAYER_ALL, 1, &pr, &png) == 0 && pr.w == W2 && pr.h == H2);
    buf_free(&png);
    // The PDF's pages are the notebook's paper size: 1860 x 2480 px at 300 dpi.
    Buf pdf = {0};
    CHECK(export_notebook_pdf(&back, DPI, &pdf) == 0 && pdf.len > 0);
    int boxes = 0;
    for (size_t i = 0; i + 30 < pdf.len; i++) boxes += !memcmp(pdf.data + i, "/MediaBox [0 0 446.40 595.20]", 29);
    CHECK(boxes == 3);
    buf_free(&pdf);
    // A rename (a load and a save) keeps the size.
    CHECK(store_rename(back.id, "Still a Scribe 2 notebook") == 0);
    CHECK(store_load_info(back.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    // A notebook made here is the Colorsoft's size.
    NotebookInfo ksc;
    CHECK(store_create("Made on a Colorsoft", "", PAPER_WIDE, &ksc) == 0 && ksc.page_w == W3 && ksc.page_h == H3);
    page_file(&ksc, 0, path, sizeof path);
    CHECK(file_size(path, &w, &h) && w == W3 && h == H3);
    CHECK(store_add_page(&ksc, 1, PAPER_WIDE) == 0);
    page_file(&ksc, 1, path, sizeof path);
    CHECK(file_size(path, &w, &h) && w == W3 && h == H3);
    // And back on the Scribe 2, it stays 1986x2648.
    store_set_device(DPI, W2, H2);
    CHECK(store_load_info(ksc.id, &ksc) == 0 && ksc.page_w == W3 && ksc.page_h == H3);
    CHECK(store_add_page(&ksc, 2, PAPER_WIDE) == 0);
    page_file(&ksc, 2, path, sizeof path);
    CHECK(file_size(path, &w, &h) && w == W3 && h == H3);
    canvas_free(&ref[0]);
    canvas_free(&ref[1]);
}

// A page a version before 1.1 scaled to another screen (and saved so after an edit) goes back to
// its notebook's size, its ink where it was written.
TEST(rescaled_pages_come_back) {
    store_set_device(DPI, W2, H2);
    NotebookInfo nb;
    CHECK(store_create("Moved about", "", PAPER_NARROW, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_NARROW) == 0);
    Page *orig = NULL;
    CHECK(store_load_page(&nb, 1, W2, H2, &orig) == 0 && orig);
    if (!orig) return;
    page_add(orig, wave(TOOL_BALLPOINT, 900.f, 5u));
    page_add(orig, wave(TOOL_PENCIL, 1900.f, 6u));
    CHECK(store_save_page(&nb, 1, orig) == 0);
    // 1.0.0-beta.2 on a Colorsoft edits page 2: saved at 1986x2648, its points scaled up.
    older_version_rescales(&nb, 1, W3, H3);
    char path[800];
    page_file(&nb, 1, path, sizeof path);
    int w = 0, h = 0;
    CHECK(file_size(path, &w, &h) && w == W3 && h == H3);
    // Opened here (on either device), it is the notebook's size again.
    for (int dev = 0; dev < 2; dev++) {
        store_set_device(DPI, dev ? W3 : W2, dev ? H3 : H2);
        NotebookInfo back;
        CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
        for (int via = 0; via < 2; via++) {
            Page *p = NULL;
            if (via == 0) CHECK(store_load_page(&back, 1, W2, H2, &p) == 0);
            else CHECK(store_read_page(&back, 1, &p) == 0);
            CHECK(p && p->w == W2 && p->h == H2 && p->n == orig->n);
            for (int s = 0; p && s < p->n && s < orig->n; s++)
                for (int k = 0; k < p->strokes[s]->n; k += 7) {
                    CHECK_NEAR(p->strokes[s]->pts[k].x, orig->strokes[s]->pts[k].x, 0.01);
                    CHECK_NEAR(p->strokes[s]->pts[k].y, orig->strokes[s]->pts[k].y, 0.01);
                }
            page_free(p);
        }
        Buf png = {0};
        PageRender pr;
        CHECK(export_page_png_ex(&back, 1, DPI, PAGE_LAYER_ALL, 1, &pr, &png) == 0 && pr.w == W2 && pr.h == H2);
        buf_free(&png);
        // Exports changed nothing on disk.
        CHECK(file_size(path, &w, &h) && w == W3 && h == H3);
    }
    // Written at the notebook's size again by the first save here.
    store_set_device(DPI, W3, H3);
    Page *p = NULL;
    CHECK(store_load_page(&nb, 1, W2, H2, &p) == 0 && p);
    if (p) CHECK(store_save_page(&nb, 1, p) == 0);
    page_free(p);
    CHECK(file_size(path, &w, &h) && w == W2 && h == H2);
    page_free(orig);
}

// notebook.ini without the size (from before 1.1, or saved by 1.0.0-beta, which drops the keys):
// the size most of its page files have gives it (a tie: the first page's), else the device does.
// The next save records it.
TEST(size_found_without_the_keys) {
    store_set_device(DPI, W2, H2);
    NotebookInfo nb;
    CHECK(store_create("No size recorded", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_MEDIUM) == 0 && store_add_page(&nb, 2, PAPER_MEDIUM) == 0);
    drop_lines(&nb, "page_w=", "page_h=");
    store_set_device(DPI, W3, H3);  // opened on a Colorsoft
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    // Pages that do not read do not count: the others say.
    char p1[800], p2[800], p3[800];
    page_file(&nb, 0, p1, sizeof p1);
    page_file(&nb, 1, p2, sizeof p2);
    page_file(&nb, 2, p3, sizeof p3);
    CHECK(write_file_atomic(p1, "QPG1", 4) == 0);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    CHECK(write_file_atomic(p1, "not a page at all, longer than sixteen bytes", 44) == 0);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    unlink(p2);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    // A page from a newer version still has its size in the first 16 bytes. One page of each size
    // (a tie) gives the size of the first in page order; two of one size outnumber one.
    uint8_t v3[24] = {'Q', 'P', 'G', '1', 3, 0, 4, 0, W3 & 255, W3 >> 8, 0, 0, H3 & 255, H3 >> 8, 0, 0};
    uint8_t v2[24] = {'Q', 'P', 'G', '1', 3, 0, 4, 0, W2 & 255, W2 >> 8, 0, 0, H2 & 255, H2 >> 8, 0, 0};
    CHECK(write_file_atomic(p1, v3, sizeof v3) == 0);  // 1986x2648, none, 1860x2480
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W3 && back.page_h == H3);
    CHECK(write_file_atomic(p1, v2, sizeof v2) == 0 && write_file_atomic(p3, v3, sizeof v3) == 0);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    CHECK(write_file_atomic(p2, v3, sizeof v3) == 0);  // 1860x2480, 1986x2648, 1986x2648
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W3 && back.page_h == H3);
    store_set_device(DPI, W2, H2);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W3 && back.page_h == H3);
    store_set_device(DPI, W3, H3);
    // No page file with a size at all: the device's.
    unlink(p1);
    unlink(p2);
    unlink(p3);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W3 && back.page_h == H3);
    store_set_device(DPI, W2, H2);
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
    // Values no screen has are not taken.
    const char *bad[] = {"page_w=0", "page_w=99", "page_w=7000", "page_w=-1986", "page_w=abc"};
    for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++) {
        store_set_device(DPI, W3, H3);
        CHECK(store_save_info(&back) == 0);  // writes page_w=1860, page_h=2480
        drop_lines(&back, "page_w=", NULL);
        append_line(&back, bad[i]);
        NotebookInfo b2;
        CHECK(store_load_info(nb.id, &b2) == 0 && b2.page_w == W3 && b2.page_h == H3);  // no page file: the device
    }
    // The next save records what was found, once (no copy among the lines kept for newer versions).
    store_set_device(DPI, W2, H2);
    CHECK(store_load_info(nb.id, &back) == 0 && store_save_info(&back) == 0);
    char path[800];
    info_file(&nb, path, sizeof path);
    char *t = text_of(path);
    CHECK(t && has_line(t, "page_w=1860") == 1 && has_line(t, "page_h=2480") == 1 && !strstr(t, "page_w=abc"));
    free(t);
    // Read by this version, a 1.0.0-beta.2 notebook.ini that carried the keys among its unknown
    // lines (after the ones it knows) gives the size too.
    drop_lines(&nb, "page_w=", "page_h=");
    append_line(&nb, "lang=es");
    append_line(&nb, "page_w=1986");
    append_line(&nb, "page_h=2648");
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W3 && back.page_h == H3);
    CHECK(strstr(back.extra, "lang=es") && !strstr(back.extra, "page_"));
}

// A ballpoint line across the page at height y.
static Stroke *line_at(float y) {
    ToolParams p;
    tool_params_default(TOOL_BALLPOINT, &p);
    Stroke *s = stroke_new(TOOL_BALLPOINT, &p, 1);
    for (int i = 0; i < 10; i++) {
        PenSample q = {200.f + 100.f * (float)i, y, 0.5f, 0, 0, (uint32_t)i};
        stroke_append(s, &q, DPI);
    }
    return s;
}

// A notebook made on an 1860x2480 Scribe, three pages with a line near the bottom of each. A
// version before 1.1 on a Colorsoft re-saves page 1 alone (1.0.0-beta does after a page delete)
// and drops the size from notebook.ini. Most of its pages say 1860x2480: on either Scribe every
// page opens at that size with its line where it was written, page 1's scaled back.
TEST(most_pages_give_the_size) {
    store_set_device(DPI, W2, H2);
    NotebookInfo nb;
    CHECK(store_create("Mixed by an older version", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_MEDIUM) == 0 && store_add_page(&nb, 2, PAPER_MEDIUM) == 0);
    for (int i = 0; i < 3; i++) {
        Page *p = NULL;
        CHECK(store_load_page(&nb, i, W2, H2, &p) == 0 && p);
        if (!p) return;
        page_add(p, line_at(2300.f));
        CHECK(store_save_page(&nb, i, p) == 0);
        page_free(p);
    }
    older_version_rescales(&nb, 0, W3, H3);
    drop_lines(&nb, "page_w=", "page_h=");
    for (int dev = 0; dev < 2; dev++) {
        store_set_device(DPI, dev ? W3 : W2, dev ? H3 : H2);
        NotebookInfo back;
        CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W2 && back.page_h == H2);
        for (int i = 0; i < 3; i++) {
            Page *p = NULL;
            CHECK(store_load_page(&back, i, W2, H2, &p) == 0 && p && p->n == 1);
            if (p && p->n == 1) CHECK_NEAR(p->strokes[0]->pts[0].y, 2300.0, 0.01);
            page_free(p);
        }
    }
    // Page 2 re-saved there too: now most pages are 1986x2648, and the notebook takes that size.
    older_version_rescales(&nb, 1, W3, H3);
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0 && back.page_w == W3 && back.page_h == H3);
}

// ---- calibration follows the panel ----

// calib_to_panel with the record given as values.
static CalibMove to_panel(InputCalib *c, int from_w, int from_h, int w, int h) {
    return calib_to_panel(c, &from_w, &from_h, w, h);
}

// A listing records the size it had to work out (notebooks from before 1.1), once: as written,
// with the notebook's modified time unchanged, so the Library's order stays as it was.
TEST(listing_records_a_found_size) {
    store_set_device(DPI, W2, H2);
    NotebookInfo nb;
    CHECK(store_create("Listed before 1.1", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_MEDIUM) == 0);
    drop_lines(&nb, "page_w=", "page_h=");
    NotebookInfo before;
    CHECK(store_load_info(nb.id, &before) == 0);
    char path[800];
    info_file(&nb, path, sizeof path);
    char *t = text_of(path);
    CHECK(t && has_line(t, "page_w=1860") == 0);
    free(t);
    store_set_device(DPI, W3, H3);  // listed on a Colorsoft: the pages still say 1860x2480
    NotebookInfo *all = NULL;
    int n = 0;
    CHECK(store_list(&all, &n) == 0 && n > 0);
    free(all);
    t = text_of(path);
    CHECK(t && has_line(t, "page_w=1860") == 1 && has_line(t, "page_h=2480") == 1);
    free(t);
    NotebookInfo after;
    CHECK(store_load_info(nb.id, &after) == 0 && after.page_w == W2 && after.page_h == H2);
    CHECK(after.modified == before.modified && !strcmp(after.name, before.name) && after.npages == before.npages);
    store_set_device(DPI, W2, H2);
}

TEST(calibration_scaling_math) {
    // The default mapping scales exactly.
    InputCalib d = {true, false, (float)W2, 0.f, (float)H2, 0.f};
    calib_scale(&d, W2, H2, W3, H3);
    CHECK(d.ax == (float)W3 && d.ay == (float)H3 && d.bx == 0.f && d.by == 0.f);
    // A Scribe 2 calibration (not recorded: taken as 1860x2480) on a Colorsoft: proportional.
    InputCalib c = {true, false, 1851.f, 7.5f, 2493.f, -12.f}, c0 = c;
    CHECK(to_panel(&c, 0, 0, W3, H3) == CALIB_SCALED);
    CHECK_NEAR(c.ax, 1851.0 * W3 / W2, 1e-3);
    CHECK_NEAR(c.bx, 7.5 * W3 / W2, 1e-4);
    CHECK_NEAR(c.ay, 2493.0 * H3 / H2, 1e-3);
    CHECK_NEAR(c.by, -12.0 * H3 / H2, 1e-4);
    // Where it maps a point is that point scaled to the new panel.
    const float pts[3][2] = {{0.1f, 0.2f}, {0.5f, 0.5f}, {0.93f, 0.81f}};
    for (int i = 0; i < 3; i++) {
        float x0, y0, x1, y1;
        calib_apply(&c0, pts[i][0], pts[i][1], &x0, &y0);
        calib_apply(&c, pts[i][0], pts[i][1], &x1, &y1);
        CHECK_NEAR(x1, x0 * W3 / W2, 0.01);
        CHECK_NEAR(y1, y0 * H3 / H2, 0.01);
    }
    // Back on the Scribe 2 (recorded as the Colorsoft's now): where it was.
    CHECK(to_panel(&c, W3, H3, W2, H2) == CALIB_SCALED);
    CHECK_NEAR(c.ax, c0.ax, 1e-3);
    CHECK_NEAR(c.bx, c0.bx, 1e-4);
    CHECK_NEAR(c.ay, c0.ay, 1e-3);
    CHECK_NEAR(c.by, c0.by, 1e-4);
    // Swapped and mirrored axes: ax and bx still go with the panel's width.
    InputCalib s = {true, true, -1860.f, 1860.f, 2480.f, 0.f};
    CHECK(to_panel(&s, W2, H2, W3, H3) == CALIB_SCALED);
    CHECK(s.swap_xy && s.ax == -1986.f && s.bx == 1986.f && s.ay == 2648.f && s.by == 0.f);
    // The same panel, or no calibration: nothing changes.
    InputCalib same = c0;
    CHECK(to_panel(&same, W2, H2, W2, H2) == CALIB_SAME && !memcmp(&same, &c0, sizeof c0));
    CHECK(to_panel(&same, 0, 0, W2, H2) == CALIB_SAME && !memcmp(&same, &c0, sizeof c0));
    InputCalib off = {false, false, 1860.f, 0.f, 2480.f, 0.f}, off0 = off;
    CHECK(to_panel(&off, 0, 0, W3, H3) == CALIB_SAME && !memcmp(&off, &off0, sizeof off));
    // Not recorded, but its scale says this Colorsoft (a calibration 1.0.0-beta.2 made here): kept.
    InputCalib here = {true, false, 1979.f, 4.f, 2655.f, -3.f}, here0 = here;
    CHECK(to_panel(&here, 0, 0, W3, H3) == CALIB_MADE_HERE && !memcmp(&here, &here0, sizeof here));
    // Recorded as a Colorsoft's, but redone on this Scribe 2 by a version that carried the line.
    InputCalib stale = {true, false, 1858.f, 2.f, 2477.f, 1.f}, stale0 = stale;
    CHECK(to_panel(&stale, W3, H3, W2, H2) == CALIB_MADE_HERE && !memcmp(&stale, &stale0, sizeof stale));
    // In between (3% off both): the record, or the 1860x2480 it is taken as, decides.
    InputCalib mid = {true, false, 1926.f, 0.f, 2568.f, 0.f};
    CHECK(to_panel(&mid, 0, 0, W3, H3) == CALIB_SCALED);
    CHECK_NEAR(mid.ax, 1926.0 * W3 / W2, 1e-3);
    // Panels whose sides change by different ratios (the Scribes' two do not: 1986 / 1860 is
    // 2648 / 2480): ax and bx go by the widths, ay and by by the heights, the axes swapped or not.
    for (int swap = 0; swap < 2; swap++) {
        InputCalib u = {true, swap != 0, 1000.f, 12.f, 2000.f, -8.f}, u0 = u;
        CHECK(to_panel(&u, 1000, 2000, 1500, 2200) == CALIB_SCALED);
        CHECK(u.swap_xy == (swap != 0));
        CHECK_NEAR(u.ax, 1500.0, 1e-3);
        CHECK_NEAR(u.bx, 18.0, 1e-4);
        CHECK_NEAR(u.ay, 2200.0, 1e-3);
        CHECK_NEAR(u.by, -8.8, 1e-4);
        for (int i = 0; i < 3; i++) {
            float x0, y0, x1, y1;
            calib_apply(&u0, pts[i][0], pts[i][1], &x0, &y0);
            calib_apply(&u, pts[i][0], pts[i][1], &x1, &y1);
            CHECK_NEAR(x1, x0 * 1.5, 0.01);
            CHECK_NEAR(y1, y0 * 1.1, 0.01);
        }
    }
    // A record gone stale the other way: recorded as this panel's, but spanning a Scribe's of the
    // other size and not this one (1.0.0-beta.2 carried the line through a calibration it made on
    // the other Scribe). It was made there, and is scaled from there, on either Scribe.
    int fw = W3, fh = H3;
    InputCalib s2 = {true, false, 1858.f, 2.f, 2477.f, 1.f};
    CHECK(calib_to_panel(&s2, &fw, &fh, W3, H3) == CALIB_SCALED && fw == W2 && fh == H2);
    CHECK_NEAR(s2.ax, 1858.0 * W3 / W2, 1e-3);
    CHECK_NEAR(s2.bx, 2.0 * W3 / W2, 1e-4);
    CHECK_NEAR(s2.ay, 2477.0 * H3 / H2, 1e-3);
    CHECK_NEAR(s2.by, 1.0 * H3 / H2, 1e-4);
    InputCalib s3 = {true, false, 1979.f, 4.f, 2655.f, -3.f}, s30 = s3;
    fw = W2;
    fh = H2;
    CHECK(calib_to_panel(&s3, &fw, &fh, W2, H2) == CALIB_SCALED && fw == W3 && fh == H3);
    CHECK_NEAR(s3.ax, 1979.0 * W2 / W3, 1e-3);
    CHECK_NEAR(s3.ay, 2655.0 * H2 / H3, 1e-3);
    // Not recorded (so taken as an 1860x2480 Scribe's) on a Scribe 2, but a Colorsoft's: scaled too.
    InputCalib n3 = s30;
    fw = fh = 0;
    CHECK(calib_to_panel(&n3, &fw, &fh, W2, H2) == CALIB_SCALED && fw == W3 && fh == H3);
    CHECK_NEAR(n3.ax, 1979.0 * W2 / W3, 1e-3);
    // Made here, recorded as the other's: the panel it was made on is this one.
    InputCalib h3 = s30;
    fw = W2;
    fh = H2;
    CHECK(calib_to_panel(&h3, &fw, &fh, W3, H3) == CALIB_MADE_HERE && fw == W3 && fh == H3);
    CHECK(!memcmp(&h3, &s30, sizeof h3));
    // Recorded as this panel's and spanning no Scribe's panel (a calibration far off): the record
    // stands, as it is.
    InputCalib loose = {true, false, 1700.f, 5.f, 2300.f, 0.f}, loose0 = loose;
    CHECK(to_panel(&loose, W2, H2, W2, H2) == CALIB_SAME && !memcmp(&loose, &loose0, sizeof loose));
    CHECK(to_panel(&loose, 0, 0, W3, H3) == CALIB_SCALED);  // not recorded, not here: an 1860x2480 one's
    CHECK_NEAR(loose.ax, 1700.0 * W3 / W2, 1e-3);
    // Spans: within 2.5% on both axes.
    InputCalib sp = {true, false, 1860.f * 1.024f, 0.f, -2480.f * 0.976f, 0.f};
    CHECK(calib_spans(&sp, W2, H2) && !calib_spans(&sp, W3, H3));
    sp.ax = 1860.f * 1.026f;
    CHECK(!calib_spans(&sp, W2, H2));
    CHECK(!calib_spans(&sp, 0, 0));
}

// settings.ini: pen.panel and touch.panel, the panel each calibration was made on.
TEST(calibration_panel_in_settings) {
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    unlink(path);
    Settings s;
    settings_load(&s);
    CHECK(s.pen_panel_w == 0 && s.pen_panel_h == 0 && s.touch_panel_w == 0 && s.touch_panel_h == 0);
    CHECK(settings_save(&s) == 0);
    char *t = text_of(path);
    CHECK(t && !strstr(t, ".panel="));  // nothing recorded, nothing written
    free(t);
    s.pen_calib = (InputCalib){true, false, 1986.f, 0.f, 2648.f, 0.f};
    s.pen_panel_w = W3;
    s.pen_panel_h = H3;
    CHECK(settings_save(&s) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=1986x2648") == 1 && !strstr(t, "touch.panel="));
    free(t);
    Settings b;
    CHECK(settings_load(&b) == 0 && b.pen_panel_w == W3 && b.pen_panel_h == H3 && b.touch_panel_w == 0);
    CHECK(b.pen_calib.valid && b.pen_calib.ax == 1986.f);
    b.touch_calib = (InputCalib){true, true, 1860.f, 0.f, 2480.f, 0.f};
    b.touch_panel_w = W2;
    b.touch_panel_h = H2;
    CHECK(settings_save(&b) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=1986x2648") == 1 && has_line(t, "touch.panel=1860x2480") == 1);
    // A round trip writes the same file.
    Settings b2;
    CHECK(settings_load(&b2) == 0 && b2.touch_panel_w == W2 && b2.touch_panel_h == H2 && settings_save(&b2) == 0);
    char *t2 = text_of(path);
    CHECK(t && t2 && !strcmp(t, t2));
    free(t);
    free(t2);
    // A value written otherwise is read and goes back as it was while the panel stays; one that is
    // no panel size is a line this version does not understand: kept, the panel unrecorded.
    const char *odd = "pen.valid=1\npen.ax=1986\npen.ay=2648\npen.panel=1986x2648x2\ntouch.valid=1\ntouch.panel=wide\n";
    CHECK(write_file_atomic(path, odd, strlen(odd)) == 0);
    CHECK(settings_load(&b) == 0 && b.pen_panel_w == W3 && b.pen_panel_h == H3 && b.touch_panel_w == 0);
    CHECK(settings_save(&b) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=1986x2648x2") == 1 && has_line(t, "touch.panel=wide") == 1);
    free(t);
    b.pen_panel_w = W2;  // scaled to another panel: written as it is now
    b.pen_panel_h = H2;
    CHECK(settings_save(&b) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=1860x2480") == 1 && !strstr(t, "1986x2648x2"));
    free(t);
    // A panel line this version cannot read stays while that calibration's panel is not recorded,
    // and goes once it is (app_init records this panel at startup): one line, the recorded one.
    const char *wide = "pen.valid=1\npen.ax=1986\npen.ay=2648\npen.panel=wide\ntouch.valid=1\ntouch.panel=narrow\n";
    CHECK(write_file_atomic(path, wide, strlen(wide)) == 0);
    CHECK(settings_load(&b) == 0 && b.pen_panel_w == 0 && b.touch_panel_w == 0);
    CHECK(settings_save(&b) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=wide") == 1 && has_line(t, "touch.panel=narrow") == 1);
    free(t);
    b.pen_panel_w = W3;
    b.pen_panel_h = H3;
    CHECK(settings_save(&b) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=1986x2648") == 1 && !strstr(t, "pen.panel=wide"));
    CHECK(t && has_line(t, "touch.panel=narrow") == 1);
    free(t);
    CHECK(settings_load(&b) == 0 && b.pen_panel_w == W3 && b.pen_panel_h == H3 && settings_save(&b) == 0);
    t = text_of(path);
    CHECK(t && has_line(t, "pen.panel=1986x2648") == 1 && !strstr(t, "pen.panel=wide"));
    free(t);
    unlink(path);
}

// ---- music layouts at both heights ----

// music_max_systems as it was before 1.1, with the page taken as 2480 px at 300 dpi.
static int part_lines(MusicPart p) {
    if (p >= PART_TAB4 && p <= PART_TAB7) return 4 + (p - PART_TAB4);
    return p == PART_LYRICS ? 1 : 5;
}
static int old_max_systems(const MusicLayout *m) {
    float su = 0.f;
    for (int i = 0; i < m->n; i++) {
        float gap = i == 0 ? 0.f : (m->part[i] == PART_LYRICS || m->part[i - 1] == PART_LYRICS) ? 3.5f : 4.5f;
        su += gap + (float)(part_lines(m->part[i]) - 1);
    }
    float avail = 2480.f / 300.f * 25.4f - 14.f - 12.f;
    int best = 1;
    for (int s = 1; s <= MUSIC_MAX_SYSTEMS; s++)
        if (avail / ((float)s * su + (float)(s - 1) * 6.f) >= 1.5f) best = s;
    return best;
}

TEST(music_layouts_at_both_heights) {
    // Every layout of up to five parts: on a 2480 px page the answer is what it always was, and a
    // 2648 px page fits as many or more.
    const int nparts = PART_COUNT - PART_TREBLE;  // treble staff to lyric line
    int layouts = 0, more = 0, wrong = 0;
    MusicLayout m = {.systems = 1};
    for (int n = 1; n <= 5; n++) {
        int idx[5] = {0};
        for (;;) {
            m.n = n;
            for (int i = 0; i < n; i++) m.part[i] = (MusicPart)(PART_TREBLE + idx[i]);
            int a = music_max_systems(&m, H2, DPI), b = music_max_systems(&m, H3, DPI);
            if (a != old_max_systems(&m) || b < a) wrong++;
            more += b > a;
            layouts++;
            int k = 0;
            while (k < n && ++idx[k] == nparts) idx[k++] = 0;
            if (k == n) break;
        }
    }
    CHECK(wrong == 0 && layouts > 100000 && more > 0);
    // The default (treble over 6-line tab): 6 systems on a Scribe 2 page, 7 on a Colorsoft page.
    MusicLayout d;
    CHECK(music_decode(music_default(), &d));
    CHECK(music_max_systems(&d, H2, DPI) == 6 && music_max_systems(&d, H3, DPI) == 7);
    // Rendering stays a pure function of the page: any region of a 1986x2648 or an 1860x2480 page
    // renders as the same region of the whole page does.
    MusicLayout big = {.systems = 7, .n = 2, .part = {PART_TREBLE, PART_TAB6}};
    const PaperKind kinds[] = {PAPER_MUSIC, PAPER_GRAND, PAPER_TAB, PAPER_BASS5, music_encode(&big), music_default()};
    const int sizes[2][2] = {{W3, H3}, {W2, H2}};
    for (int z = 0; z < 2; z++)
        for (size_t k = 0; k < sizeof kinds / sizeof kinds[0]; k++) {
            Canvas full, part;
            if (canvas_init(&full, sizes[z][0], sizes[z][1]) != 0 || canvas_init(&part, sizes[z][0], sizes[z][1]) != 0) {
                CHECK(0);
                return;
            }
            paper_render(&full, kinds[k], (Rect){0, 0, full.w, full.h}, DPI);
            Rect bands[3] = {{0, 0, full.w, 700}, {0, 700, full.w, 1300}, {0, 2000, full.w, full.h - 2000}};
            for (int i = 0; i < 3; i++) paper_render(&part, kinds[k], bands[i], DPI);
            CHECK(same_pixels(&full, &part));
            canvas_free(&full);
            canvas_free(&part);
        }
}

// ---- the colour panel's wait ----

// Which fast refreshes are live ink (display.h): on a colour panel the Kindle waits for the
// driver's read after every other fast refresh, so pen reading stalls for none of a stroke's.
TEST(live_ink_frames) {
    LiveInk l = {0};
    CHECK(!live_ink_frame(&l, REFRESH_FAST, 1000));  // a pressed button, a dragged panel: waits
    live_ink_mark(&l, true, 1000);                    // a stroke starts
    CHECK(live_ink_frame(&l, REFRESH_FAST, 1010) && live_ink_frame(&l, REFRESH_FAST, 5000));
    CHECK(!live_ink_frame(&l, REFRESH_UI, 5010) && !live_ink_frame(&l, REFRESH_FULL, 5020));  // only fast ones
    live_ink_mark(&l, false, 6000);                   // the pen lifts
    CHECK(!live_ink_frame(&l, REFRESH_UI, 6010));     // (a grayscale refresh in between is not one)
    CHECK(live_ink_frame(&l, REFRESH_FAST, 6050));    // the samples that came with the lifting
    CHECK(!live_ink_frame(&l, REFRESH_FAST, 6060));   // and no more
    live_ink_mark(&l, true, 7000);
    live_ink_mark(&l, false, 7010);
    CHECK(!live_ink_frame(&l, REFRESH_FAST, 7120));   // 100 ms or more after the end: not
    live_ink_mark(&l, false, 8000);                   // ended again (leaving a screen): nothing ends
    CHECK(!live_ink_frame(&l, REFRESH_FAST, 8001));
    // An end at the clock's wrap, and at 0 ms (never taken for no end at all).
    live_ink_mark(&l, true, 0xFFFFFFF0u);
    live_ink_mark(&l, false, 0xFFFFFFF0u);
    CHECK(live_ink_frame(&l, REFRESH_FAST, 0x20u));
    live_ink_mark(&l, true, 0);
    live_ink_mark(&l, false, 0);
    CHECK(live_ink_frame(&l, REFRESH_FAST, 5));
}

int main(void) {
    snprintf(root, sizeof root, "%s/pagesize-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    RUN(notebook_keeps_its_size);
    RUN(rescaled_pages_come_back);
    RUN(size_found_without_the_keys);
    RUN(most_pages_give_the_size);
    RUN(listing_records_a_found_size);
    RUN(calibration_scaling_math);
    RUN(calibration_panel_in_settings);
    RUN(music_layouts_at_both_heights);
    RUN(live_ink_frames);
    int rc = tl_done();
    system(cmd);
    return rc;
}
