// test_store.c: store + export.
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "export.h"
#include "testlib.h"

static char root[600];

static Stroke *squiggle(ToolKind t, float y) {
    ToolParams p;
    tool_params_default(t, &p);
    Stroke *s = stroke_new(t, &p, 99);
    for (int i = 0; i < 50; i++) {
        PenSample q = {300.f + i * 20.f, y + 30.f * sinf(i * 0.4f), 0.2f + i / 70.f, 10.f, -5.f, (uint32_t)i * 4};
        stroke_append(s, &q, 300.f);
    }
    return s;
}

TEST(notebooks) {
    NotebookInfo nb, back;
    CHECK(store_create("Lecture notes", "", PAPER_NARROW, &nb) == 0);
    CHECK(store_load_info(nb.id, &back) == 0);
    CHECK(!strcmp(back.name, "Lecture notes") && back.paper == PAPER_NARROW && back.npages == 1);
    CHECK(store_add_page(&nb, 1, PAPER_MUSIC) == 0);
    CHECK(store_add_page(&nb, 0, PAPER_WIDE) == 0);
    CHECK(nb.npages == 3 && nb.page_ids[0] == 3 && nb.page_ids[1] == 1 && nb.page_ids[2] == 2);
    Page *p;
    CHECK(store_load_page(&nb, 2, 1860, 2480, &p) == 0 && p->paper == PAPER_MUSIC);
    page_add(p, squiggle(TOOL_PENCIL, 500));
    page_add(p, squiggle(TOOL_BALLPOINT, 900));
    CHECK(store_save_page(&nb, 2, p) == 0);
    Page *q;
    CHECK(store_load_page(&nb, 2, 1860, 2480, &q) == 0);
    CHECK(q->n == 2 && q->strokes[1]->n == 50 && q->strokes[1]->tool == TOOL_BALLPOINT);
    CHECK_NEAR(q->strokes[0]->pts[10].x, p->strokes[0]->pts[10].x, 1e-4);
    CHECK_NEAR(q->strokes[0]->pts[10].pressure, p->strokes[0]->pts[10].pressure, 1e-4);
    CHECK_NEAR(q->strokes[0]->pts[10].tilt_y, -5.f, 0.01);
    page_free(q);
    // Scaled load
    CHECK(store_load_page(&nb, 2, 930, 1240, &q) == 0);
    CHECK(q->w == 930 && fabsf(q->strokes[0]->pts[10].x - p->strokes[0]->pts[10].x * 0.5f) < 1e-3);
    page_free(q);
    page_free(p);
    CHECK(store_delete_page(&nb, 0) == 0 && nb.npages == 2);
    CHECK(store_rename(nb.id, "Physics") == 0);
    CHECK(store_set_folder(nb.id, "School") == 0);
    NotebookInfo nb2;
    CHECK(store_create("Songs", "", PAPER_MUSIC, &nb2) == 0);
    store_add_folder("Empty folder");
    char (*folders)[STORE_NAME_LEN];
    int nf;
    CHECK(store_list_folders(&folders, &nf) == 0);
    CHECK(nf == 2 && !strcmp(folders[0], "Empty folder") && !strcmp(folders[1], "School"));
    free(folders);
    NotebookInfo *list;
    int n;
    CHECK(store_list(&list, &n) == 0 && n == 2);
    free(list);
    // Export everything
    Buf b = {0};
    CHECK(store_load_info(nb.id, &nb) == 0);
    CHECK(export_notebook_pdf(&nb, 300.f, &b) == 0);
    CHECK(b.len > 1000 && !memcmp(b.data, "%PDF-1.4", 8));
    // xref offsets must point at "N 0 obj"
    char *x = strstr((char *)b.data + b.len - 200, "startxref");
    CHECK(x != NULL);
    if (x) {
        size_t xo = strtoul(x + 10, NULL, 10);
        CHECK(!memcmp(b.data + xo, "xref", 4));
        char *row = (char *)b.data + xo;
        row = strchr(row, '\n') + 1;
        row = strchr(row, '\n') + 1;  // past "0 N"
        row = strchr(row, '\n') + 1;  // past free entry
        size_t o1 = strtoul(row, NULL, 10);
        CHECK(!memcmp(b.data + o1, "1 0 obj", 7));
    }
    char path[700];
    snprintf(path, sizeof path, "%s/../out/sample.pdf", tl_tmp());
    write_file_atomic(path, b.data, b.len);
    buf_free(&b);
    CHECK(export_page_png(&nb, 1, 300.f, &b) == 0 && !memcmp(b.data + 1, "PNG", 3));
    buf_free(&b);
    CHECK(export_folder_zip("", 300.f, &b) == 0);
    snprintf(path, sizeof path, "%s/../out/all.zip", tl_tmp());
    write_file_atomic(path, b.data, b.len);
    buf_free(&b);
    char cmd[900];
    snprintf(cmd, sizeof cmd, "unzip -tq '%s' >/dev/null", path);
    CHECK(system(cmd) == 0);
    ZipWriter z;
    zip_begin(&z, &b);
    CHECK(export_notebook_zip(&nb, 300.f, &z, "") == 0);
    zip_end(&z);
    snprintf(path, sizeof path, "%s/../out/notebook.zip", tl_tmp());
    write_file_atomic(path, b.data, b.len);
    buf_free(&b);
    snprintf(cmd, sizeof cmd, "unzip -tq '%s' >/dev/null && unzip -l '%s' | grep -q 'Physics/page-002.png'", path, path);
    CHECK(system(cmd) == 0);
    char docs[700];
    snprintf(docs, sizeof docs, "%s/documents", root);
    CHECK(export_mirror_pdf(&nb, 300.f, docs) == 0);
    snprintf(path, sizeof path, "%s/Quire/School/Physics.pdf", docs);
    CHECK(access(path, F_OK) == 0);
    CHECK(store_delete(nb2.id) == 0);
    CHECK(store_list(&list, &n) == 0 && n == 1);
    free(list);
}

TEST(fuzz_deserialize) {
    Page *p = page_new(1860, 2480, PAPER_MEDIUM);
    for (int i = 0; i < 5; i++) page_add(p, squiggle(i % 3, 200.f + i * 100));
    uint8_t *d;
    size_t len;
    CHECK(page_serialize(p, &d, &len) == 0);
    page_free(p);
    Page *q;
    // Every truncation must fail cleanly (except the full length).
    for (size_t l = 0; l < len; l += 7) CHECK(page_deserialize(d, l, &q) != 0 || (page_free(q), 1));
    srand(1);
    uint8_t *m = malloc(len);
    for (int it = 0; it < 3000; it++) {
        memcpy(m, d, len);
        for (int k = 0; k < 1 + it % 8; k++) m[rand() % len] = (uint8_t)rand();
        if (page_deserialize(m, len, &q) == 0) page_free(q);
    }
    for (int it = 0; it < 500; it++) {
        for (size_t k = 0; k < len; k++) m[k] = (uint8_t)rand();
        memcpy(m, "QPG1\x01\x00", 6);
        if (page_deserialize(m, len, &q) == 0) page_free(q);
    }
    free(m);
    free(d);
}

TEST(settings_roundtrip) {
    Settings s, t;
    settings_default(&s);
    s.tools[TOOL_BALLPOINT].pressure_gamma = 2.5f;
    s.tools[TOOL_PENCIL].tilt_size = 99.f;  // out of range, must clamp on load
    s.panel_x = 123;
    s.panel_collapsed = true;
    s.pen_calib = (InputCalib){true, true, -1860.f, 1860.f, 2480.f, 0.f};
    CHECK(settings_save(&s) == 0);
    settings_load(&t);
    CHECK_NEAR(t.tools[TOOL_BALLPOINT].pressure_gamma, 2.5f, 1e-3);
    CHECK_NEAR(t.tools[TOOL_PENCIL].tilt_size, 4.f, 1e-3);
    CHECK(t.panel_x == 123 && t.panel_collapsed && t.pen_calib.valid && t.pen_calib.swap_xy);
    CHECK_NEAR(t.pen_calib.ax, -1860.f, 1e-2);
}

TEST(safe_names) {
    char out[64];
    export_safe_name("a/b:c?", out, sizeof out);
    CHECK(!strcmp(out, "a_b_c_"));
    export_safe_name("  ..  ", out, sizeof out);
    CHECK(!strcmp(out, "Untitled"));
}

int main(void) {
    snprintf(root, sizeof root, "%s/store-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    RUN(notebooks);
    RUN(fuzz_deserialize);
    RUN(settings_roundtrip);
    RUN(safe_names);
    int rc = tl_done();
    system(cmd);
    return rc;
}
