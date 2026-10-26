// test_store.c: store + export.
#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "bgjob.h"
#include "export.h"
#include "testlib.h"

static char root[600];

static void count_pdf_pages(int done, int total, void *ctx) {
    int *count = ctx;
    CHECK(done == *count && done <= total);
    (*count)++;
}

TEST(buffer_failure_stays_failed) {
    Buf b = {.len = SIZE_MAX - 1};
    CHECK(buf_append(&b, "xx", 2) == -1 && b.err);
    CHECK(b.len == SIZE_MAX - 1 && b.data == NULL);
    b.len = 0;
    CHECK(buf_append(&b, "x", 1) == -1 && b.len == 0);
    buf_free(&b);
    CHECK(buf_append(&b, "abc", 3) == 0 && b.len == 3 && !b.err);
    buf_free(&b);
}

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

// A page added after a page of 1.1's document role (an imported page image, 0x51......) gets the
// notebook's paper, as 1.1 does: the + button and the page left when every page is deleted pass the
// page they follow, and 1.1 would draw the other page's picture on the new one. A template (role 0)
// and any other newer paper are kept, as are the known kinds.
TEST(new_page_after_a_document_page) {
    NotebookInfo nb;
    CHECK(store_create("Annotated", "", PAPER_NARROW, &nb) == 0);
    const PaperKind doc = (PaperKind)0x51000123u, tpl = (PaperKind)0x50000123u;
    CHECK(paper_is_document(doc) && !paper_is_document(tpl));
    CHECK(!paper_is_document((PaperKind)0x55000123u) && !paper_is_document((PaperKind)0x53000123u));
    CHECK(!paper_is_document(PAPER_GRID) && !paper_is_document(music_default()));
    CHECK(store_new_page_paper(&nb, doc) == PAPER_NARROW);
    CHECK(store_new_page_paper(&nb, tpl) == tpl && store_new_page_paper(&nb, PAPER_GRID) == PAPER_GRID);
    CHECK(store_new_page_paper(&nb, (PaperKind)0x55000123u) == (PaperKind)0x55000123u);
    CHECK(store_add_page(&nb, 1, doc) == 0 && store_add_page(&nb, 2, tpl) == 0);
    Page *p;
    CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0 && p->paper == PAPER_NARROW);
    page_free(p);
    CHECK(store_load_page(&nb, 2, 1860, 2480, &p) == 0 && p->paper == tpl);
    page_free(p);
    NotebookInfo odd = nb;
    odd.paper = doc;  // a notebook whose own paper is a document page's: Blank, never the picture
    CHECK(store_new_page_paper(&odd, doc) == PAPER_BLANK);
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
    Buf with_progress = {0};
    int completed = 0;
    CHECK(export_notebook_pdf_progress(&nb, 300.f, &with_progress, count_pdf_pages, &completed) == 0);
    CHECK(completed == nb.npages + 1);
    CHECK(with_progress.len == b.len && !memcmp(with_progress.data, b.data, b.len));
    buf_free(&with_progress);
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
    char path[700], out[700];
    snprintf(out, sizeof out, "%s/../out", tl_tmp());
    CHECK(mkdir_p(out) == 0);  // (other tests make it too; this one must not depend on running after them)
    snprintf(path, sizeof path, "%s/sample.pdf", out);
    write_file_atomic(path, b.data, b.len);
    buf_free(&b);
    CHECK(export_page_png(&nb, 1, 300.f, &b) == 0 && !memcmp(b.data + 1, "PNG", 3));
    buf_free(&b);
    CHECK(export_folder_zip("", 300.f, &b) == 0);
    snprintf(path, sizeof path, "%s/all.zip", out);
    write_file_atomic(path, b.data, b.len);
    buf_free(&b);
    char cmd[900];
    snprintf(cmd, sizeof cmd, "unzip -tq '%s' >/dev/null", path);
    CHECK(system(cmd) == 0);
    ZipWriter z;
    zip_begin(&z, &b);
    CHECK(export_notebook_zip(&nb, 300.f, &z, "") == 0);
    zip_end(&z);
    snprintf(path, sizeof path, "%s/notebook.zip", out);
    write_file_atomic(path, b.data, b.len);
    buf_free(&b);
    snprintf(cmd, sizeof cmd, "unzip -tq '%s' >/dev/null && unzip -l '%s' | grep -q 'Physics/page-002.png'", path, path);
    CHECK(system(cmd) == 0);
    char docs[700];
    snprintf(docs, sizeof docs, "%s/PDFs", root);
    CHECK(export_mirror_pdf(&nb, 300.f, docs) == 0);
    snprintf(path, sizeof path, "%s/School/Physics.pdf", docs);
    CHECK(access(path, F_OK) == 0);
    // The same export through the background queue: the UI never waits for it.
    unlink(path);
    bg_mirror_pdf(nb.id, 300.f, docs, true);
    bg_mirror_pdf(nb.id, 300.f, docs, false);  // queued behind the running one (it may be stale)
    bg_mirror_pdf(nb.id, 300.f, docs, false);  // a duplicate of a queued job merges into it
    // Two exports by children built with the sanitizers and niced: on a loaded machine (several
    // gates at once) that takes far longer than on an idle one, so wait for them by the clock.
    int got = 0;
    uint32_t until = now_ms() + 60000;
    while ((bg_busy() || !got) && (int32_t)(now_ms() - until) < 0) {
        int r = bg_poll();
        if (r) got = r;
        usleep(10000);
    }
    CHECK(got == 1 && !bg_busy());
    CHECK(access(path, F_OK) == 0);
    CHECK(store_delete(nb2.id) == 0);
    CHECK(store_list(&list, &n) == 0 && n == 1);
    free(list);
}

TEST(copy_pages_keeps_bytes_and_order) {
    NotebookInfo nb, back;
    CHECK(store_create("Copies", "", PAPER_NARROW, &nb) == 0);
    Page *p = NULL;
    CHECK(store_load_page(&nb, 0, 1860, 2480, &p) == 0);
    p->paper = (PaperKind)0x51000123u;
    page_add(p, squiggle(TOOL_PENCIL, 500));
    CHECK(store_save_page(&nb, 0, p) == 0);
    page_free(p);
    char dir[700], source[800], dest[800];
    CHECK(store_notebook_dir(nb.id, dir, sizeof dir) == 0);
    snprintf(source, sizeof source, "%s/p1.qpg", dir);
    size_t n = 0, m = 0;
    uint8_t *before = read_file(source, &n);
    CHECK(before != NULL);
    CHECK(store_duplicate_page(&nb, 0, 1) == 0);
    CHECK(nb.npages == 2 && nb.page_ids[0] == 1 && nb.page_ids[1] == 2);
    snprintf(dest, sizeof dest, "%s/p2.qpg", dir);
    uint8_t *after = read_file(dest, &m);
    CHECK(before && after && n == m && !memcmp(before, after, n));
    free(after);
    CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0);
    CHECK(p && p->n == 1 && p->paper == (PaperKind)0x51000123u);
    page_free(p);
    CHECK(store_load_info(nb.id, &back) == 0 && back.npages == 2 && back.page_ids[1] == 2);

    // A page from a future header version is read-only here. Copy retains its original bytes.
    CHECK(n > 6);
    if (before && n > 6) {
        before[4] = 3;
        before[5] = 0;
        CHECK(write_file_atomic(source, before, n) == 0);
        CHECK(store_duplicate_page(&nb, 0, 1) == 0);
        snprintf(dest, sizeof dest, "%s/p3.qpg", dir);
        after = read_file(dest, &m);
        CHECK(after && n == m && !memcmp(before, after, n));
        free(after);
        CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0 && p && p->readonly);
        page_free(p);
    }
    free(before);
    CHECK(nb.npages == 3 && nb.page_ids[0] == 1 && nb.page_ids[1] == 3 && nb.page_ids[2] == 2);

    // Only a genuinely missing file is a blank page. It copies the notebook's paper.
    CHECK(unlink(source) == 0);
    CHECK(store_duplicate_page(&nb, 0, nb.npages) == 0);
    CHECK(store_load_page(&nb, nb.npages - 1, 1860, 2480, &p) == 0);
    CHECK(p && !p->n && p->paper == PAPER_NARROW);
    page_free(p);
}

TEST(copy_and_insert_fail_without_changing_the_page_list) {
    NotebookInfo nb, back;
    CHECK(store_create("Copy errors", "", PAPER_MEDIUM, &nb) == 0);
    char dir[700], source[800], info[800], blocked[810], dest[800];
    CHECK(store_notebook_dir(nb.id, dir, sizeof dir) == 0);
    snprintf(source, sizeof source, "%s/p1.qpg", dir);
    snprintf(info, sizeof info, "%s/notebook.ini", dir);
    snprintf(blocked, sizeof blocked, "%s.tmp", info);
    snprintf(dest, sizeof dest, "%s/p2.qpg", dir);
    size_t n = 0, m = 0;
    uint8_t *original = read_file(source, &n);
    CHECK(original != NULL);
    CHECK(write_file_atomic(source, "not a page", 10) == 0);
    CHECK(store_duplicate_page(&nb, 0, 1) != 0);
    CHECK(nb.npages == 1 && nb.next_page_id == 2 && access(dest, F_OK) != 0);
    uint8_t *data = read_file(source, &m);
    CHECK(data && m == 10 && !memcmp(data, "not a page", 10));
    free(data);
    CHECK(unlink(source) == 0 && mkdir(source, 0755) == 0);
    CHECK(store_duplicate_page(&nb, 0, 1) != 0);
    CHECK(nb.npages == 1 && nb.next_page_id == 2);
    CHECK(rmdir(source) == 0);
    CHECK(original && write_file_atomic(source, original, n) == 0);
    free(original);

    // Writing the page succeeds, but saving notebook.ini fails. The new file is removed,
    // the old notebook.ini stays intact and the in-memory list is safe to save again.
    uint8_t *old_info = read_file(info, &n);
    CHECK(old_info != NULL && mkdir(blocked, 0755) == 0);
    uint32_t next = nb.next_page_id;
    int64_t modified = nb.modified;
    CHECK(store_duplicate_page(&nb, 0, 1) != 0);
    CHECK(nb.npages == 1 && nb.page_ids[0] == 1 && nb.next_page_id == next && nb.modified == modified);
    CHECK(access(dest, F_OK) != 0);
    CHECK(store_add_page(&nb, 1, PAPER_GRID) != 0);
    CHECK(nb.npages == 1 && nb.page_ids[0] == 1 && nb.next_page_id == next && nb.modified == modified);
    CHECK(access(dest, F_OK) != 0);
    data = read_file(info, &m);
    CHECK(old_info && data && n == m && !memcmp(old_info, data, n));
    free(old_info);
    free(data);
    CHECK(rmdir(blocked) == 0 && store_save_info(&nb) == 0);
    CHECK(store_load_info(nb.id, &back) == 0 && back.npages == 1 && back.page_ids[0] == 1);
    CHECK(store_duplicate_page(&nb, 0, 1) == 0 && nb.page_ids[1] == next);
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

TEST(custom_music_paper_roundtrip) {
    MusicLayout m = {.systems = 3, .n = 5, .part = {PART_TREBLE, PART_BASS, PART_TAB6, PART_LYRICS, PART_TAB4}};
    PaperKind k = music_encode(&m);
    MusicLayout r;
    CHECK(paper_is_custom(k) && music_decode(k, &r));
    CHECK(r.systems == 3 && r.n == 5 && r.part[1] == PART_BASS && r.part[4] == PART_TAB4);
    CHECK(paper_sanitize((uint32_t)k, PAPER_BLANK) == k);
    CHECK(paper_sanitize((uint32_t)PAPER_CUSTOM_FLAG, PAPER_BLANK) == PAPER_BLANK);         // no parts
    CHECK(paper_sanitize((uint32_t)PAPER_CUSTOM_FLAG | 1u << 24 | 15u, PAPER_BLANK) == PAPER_BLANK);  // bad part
    CHECK(paper_sanitize(999u, PAPER_MEDIUM) == PAPER_MEDIUM);
    CHECK(music_max_systems(&m, 2480, 300.f) >= 2 && music_max_systems(&m, 2480, 300.f) <= MUSIC_MAX_SYSTEMS);
    Page *p = page_new(1860, 2480, k);
    page_add(p, squiggle(0, 300.f));
    uint8_t *d;
    size_t len;
    CHECK(page_serialize(p, &d, &len) == 0);
    CHECK(d[4] == 2);  // version 2 only for custom paper
    Page *q = NULL;
    CHECK(page_deserialize(d, len, &q) == 0 && q && q->paper == k && q->n == 1);
    for (size_t l = 0; l < len; l += 5) {
        Page *t;
        CHECK(page_deserialize(d, l, &t) != 0 || (page_free(t), 1));
    }
    page_free(q);
    free(d);
    page_free(p);
    p = page_new(1860, 2480, PAPER_BASS5);
    CHECK(page_serialize(p, &d, &len) == 0 && d[4] == 1);  // fixed papers stay version 1
    free(d);
    page_free(p);
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

TEST(settings_upgrade_keeps_tweaks) {
    // An old settings file (no settings_version): pencil size still the old default,
    // tilt widening tuned by hand.
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    const char *old = "current_tool=0\npencil.size_min_mm=0.30\npencil.size_max_mm=0.85\npencil.tilt_size=3.10\n"
                      "mech.grain=0.35\n";
    write_file_atomic(path, old, strlen(old));
    Settings s;
    settings_load(&s);
    CHECK_NEAR(s.tools[TOOL_PENCIL].size_min_mm, 0.12f, 1e-3);
    CHECK_NEAR(s.tools[TOOL_PENCIL].size_max_mm, 0.32f, 1e-3);
    CHECK_NEAR(s.tools[TOOL_PENCIL].tilt_size, 3.10f, 1e-3);  // the owner's own value stays
    CHECK_NEAR(s.tools[TOOL_MECH_PENCIL].grain, 0.50f, 1e-3);
    CHECK(s.version == SETTINGS_VERSION);
    // Saved again, an upgraded file never migrates twice.
    s.tools[TOOL_PENCIL].size_max_mm = 0.85f;  // now a deliberate choice
    settings_save(&s);
    Settings t;
    settings_load(&t);
    CHECK_NEAR(t.tools[TOOL_PENCIL].size_max_mm, 0.85f, 1e-3);
}

TEST(settings_upgrade_ballpoint_smoothing) {
    // A version-2 file: the ballpoint's smoothing still at its old default moves to the new one.
    // A pencil smoothing equal to a version-1 default stays: version 2's upgrades ran already.
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    const char *v2 = "settings_version=2\nballpoint.smoothing=1.0000\npencil.smoothing=0.2500\n"
                     "fineliner.smoothing=1.0000\n";
    write_file_atomic(path, v2, strlen(v2));
    Settings s;
    settings_load(&s);
    CHECK_NEAR(s.tools[TOOL_BALLPOINT].smoothing, 0.50f, 1e-3);
    CHECK_NEAR(s.tools[TOOL_PENCIL].smoothing, 0.25f, 1e-3);
    CHECK_NEAR(s.tools[TOOL_FINELINER].smoothing, 1.00f, 1e-3);  // its default did not change
    CHECK(s.version == SETTINGS_VERSION);
    // Tuned by hand: stays.
    const char *tuned = "settings_version=2\nballpoint.smoothing=0.8000\n";
    write_file_atomic(path, tuned, strlen(tuned));
    Settings t;
    settings_load(&t);
    CHECK_NEAR(t.tools[TOOL_BALLPOINT].smoothing, 0.80f, 1e-3);
    // A version-1 file gets every change.
    const char *v1 = "settings_version=1\nballpoint.smoothing=1.0000\npencil.smoothing=0.2500\n";
    write_file_atomic(path, v1, strlen(v1));
    Settings u;
    settings_load(&u);
    CHECK_NEAR(u.tools[TOOL_BALLPOINT].smoothing, 0.50f, 1e-3);
    CHECK_NEAR(u.tools[TOOL_PENCIL].smoothing, 0.00f, 1e-3);
}

TEST(pen_defaults_from_device) {
    // Pen-only snapshot chosen on the Kindle, independent of the compiled defaults.
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    size_t len = 0;
    char *snapshot = read_file("tests/fixtures/pen-defaults.ini", &len);
    CHECK(snapshot != NULL);
    if (!snapshot) return;
    CHECK(write_file_atomic(path, snapshot, len) == 0);
    free(snapshot);
    Settings chosen, fresh, saved;
    CHECK(settings_load(&chosen) == 0);
    settings_default(&fresh);
    CHECK(fresh.eraser_mode == 0);  // whole strokes remain the fresh-install default
    int n;
    const ParamInfo *pi = tool_param_table(&n);
    for (int t = 0; t < TOOL_COUNT; t++) {
        ToolParams reset;
        tool_params_default(t, &reset);  // the Pen feel reset action uses this function too
        for (int i = 0; i < n; i++) {
            float want = *tool_param_ptr(&chosen.tools[t], &pi[i]);
            CHECK_NEAR(*tool_param_ptr(&fresh.tools[t], &pi[i]), want, 1e-6);
            CHECK_NEAR(*tool_param_ptr(&reset, &pi[i]), want, 1e-6);
        }
        // A saved profile (the current version) is not migrated to these defaults.
        chosen.tools[t].size_min_mm = 0.26f;
        chosen.tools[t].size_max_mm = 0.36f;
        chosen.tools[t].smoothing = 0.15f;
    }
    CHECK(settings_save(&chosen) == 0 && settings_load(&saved) == 0);
    for (int t = 0; t < TOOL_COUNT; t++)
        for (int i = 0; i < n; i++)
            CHECK_NEAR(*tool_param_ptr(&saved.tools[t], &pi[i]),
                       *tool_param_ptr(&chosen.tools[t], &pi[i]), 1e-6);
}

// settings.ini's panel_tools: the pens pinned to the editor's panel, in pin order. A line written
// by hand or by a newer version keeps the drawing tools this version has, each once, the eraser
// left out, at most PANEL_PINS_MAX; it is written back as it was until the pins change.
static bool has_pins_line(const char *path, const char *line) {
    size_t len;
    char *t = read_file(path, &len), want[200];
    snprintf(want, sizeof want, "\n%s\n", line);
    bool ok = t && strstr(t, want);
    free(t);
    return ok;
}

TEST(panel_tools) {
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    Settings s, t;
    settings_default(&s);
    CHECK(s.npins == 0 && !settings_pinned(&s, TOOL_PENCIL));
    CHECK(settings_toggle_pin(&s, TOOL_HIGHLIGHTER) && settings_toggle_pin(&s, TOOL_FINELINER));
    CHECK(!settings_toggle_pin(&s, TOOL_ERASER) && !settings_toggle_pin(&s, TOOL_COUNT));
    CHECK(s.npins == 2 && settings_pinned(&s, TOOL_FINELINER) && !settings_pinned(&s, TOOL_ERASER));
    CHECK(settings_save(&s) == 0 && has_pins_line(path, "panel_tools=highlighter,fineliner"));
    CHECK(settings_load(&t) == 0 && t.npins == 2 && t.pins[0] == TOOL_HIGHLIGHTER && t.pins[1] == TOOL_FINELINER);
    // None pinned: an empty line.
    settings_default(&s);
    CHECK(settings_save(&s) == 0 && has_pins_line(path, "panel_tools="));
    CHECK(settings_load(&t) == 0 && t.npins == 0);
    // Unknown names, the eraser and repeats are left out; the first four left are the pins.
    const char *mixed = "panel_tools=eraser,nope,highlighter,highlighter,pencil,mech,ballpoint,marker\n";
    CHECK(write_file_atomic(path, mixed, strlen(mixed)) == 0);
    CHECK(settings_load(&t) == 0 && t.npins == 4);
    CHECK(t.pins[0] == TOOL_HIGHLIGHTER && t.pins[1] == TOOL_PENCIL && t.pins[2] == TOOL_MECH_PENCIL &&
          t.pins[3] == TOOL_BALLPOINT);
    for (int i = 0; i < t.npins; i++)
        for (int k = 0; k < i; k++) CHECK(t.pins[i] != t.pins[k] && t.pins[i] != TOOL_ERASER);
    char text[128];
    settings_pins_text(&t, text, sizeof text);
    CHECK(!strcmp(text, "highlighter,pencil,mech,ballpoint"));
    // Full: a fifth is refused, the pins stay as they were.
    CHECK(!settings_toggle_pin(&t, TOOL_MARKER) && t.npins == 4 && !settings_pinned(&t, TOOL_MARKER));
    // Saved unchanged, the line goes back as it was (a newer version's pen is still in it).
    CHECK(settings_save(&t) == 0 &&
          has_pins_line(path, "panel_tools=eraser,nope,highlighter,highlighter,pencil,mech,ballpoint,marker"));
    // Changed, the pins are written as they are: unpinning one keeps the others' order.
    CHECK(settings_toggle_pin(&t, TOOL_PENCIL) && t.npins == 3);
    CHECK(settings_save(&t) == 0 && has_pins_line(path, "panel_tools=highlighter,mech,ballpoint"));
    CHECK(settings_toggle_pin(&t, TOOL_CALLIGRAPHY) && t.pins[3] == TOOL_CALLIGRAPHY);
    CHECK(settings_save(&t) == 0 && has_pins_line(path, "panel_tools=highlighter,mech,ballpoint,calligraphy"));
    CHECK(settings_load(&s) == 0 && s.npins == 4 && s.pins[3] == TOOL_CALLIGRAPHY);
    // Written by hand with spaces, and with nothing between commas.
    const char *spaced = "panel_tools= fineliner ,, marker,\n";
    CHECK(write_file_atomic(path, spaced, strlen(spaced)) == 0);
    CHECK(settings_load(&t) == 0 && t.npins == 2 && t.pins[0] == TOOL_FINELINER && t.pins[1] == TOOL_MARKER);
    unlink(path);
}

// settings.ini's before_eraser: the pen in use when the eraser was picked, -1 for none. A pen this
// version lacks, the eraser or no number is none, and the line goes back as it was until it changes.
TEST(before_eraser) {
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    Settings s, t;
    settings_default(&s);
    CHECK(s.before_eraser == -1);
    CHECK(settings_save(&s) == 0 && has_pins_line(path, "before_eraser=-1"));
    CHECK(settings_load(&t) == 0 && t.before_eraser == -1);
    s.before_eraser = TOOL_MARKER;
    CHECK(settings_save(&s) == 0 && has_pins_line(path, "before_eraser=7"));
    CHECK(settings_load(&t) == 0 && t.before_eraser == TOOL_MARKER);
    const char *odd[] = {"12", "3", "pencil", "4x", ""};
    for (size_t i = 0; i < sizeof odd / sizeof odd[0]; i++) {
        char f[64], line[64];
        snprintf(f, sizeof f, "before_eraser=%s\n", odd[i]);
        CHECK(write_file_atomic(path, f, strlen(f)) == 0);
        CHECK(settings_load(&t) == 0 && t.before_eraser == -1);
        snprintf(line, sizeof line, "before_eraser=%s", odd[i]);
        CHECK(settings_save(&t) == 0 && has_pins_line(path, line));  // written back as it was
    }
    // Changed, it is written as it is.
    t.before_eraser = TOOL_HIGHLIGHTER;
    CHECK(settings_save(&t) == 0 && has_pins_line(path, "before_eraser=4"));
    CHECK(settings_load(&s) == 0 && s.before_eraser == TOOL_HIGHLIGHTER);
    unlink(path);
}

// A damaged page is renamed aside, never overwritten by the blank shown in its place; an empty
// notebook.ini gets its page list back from the page files.
TEST(damage_is_kept) {
    NotebookInfo nb;
    CHECK(store_create("Fragile", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_add_page(&nb, 1, PAPER_MEDIUM) == 0);
    CHECK(store_add_page(&nb, 2, PAPER_MEDIUM) == 0);
    char dir[700], path[800];
    snprintf(dir, sizeof dir, "%s/notebooks/%s", root, nb.id);
    snprintf(path, sizeof path, "%s/p2.qpg", dir);
    FILE *f = fopen(path, "wb");
    CHECK(f != NULL);
    if (f) {
        fputs("QPG1 this is not a page", f);
        fclose(f);
    }
    Page *p = NULL;
    CHECK(store_load_page(&nb, 1, 1860, 2480, &p) == 0 && p && p->n == 0);
    page_free(p);
    CHECK(store_take_corrupt_count() == 1);
    CHECK(store_take_corrupt_count() == 0);
    struct stat st;
    CHECK(stat(path, &st) != 0);  // moved aside...
    int aside = 0;
    DIR *d = opendir(dir);
    for (struct dirent *e; d && (e = readdir(d));)
        if (!strncmp(e->d_name, "p2.qpg.damaged-", 15)) aside++;
    if (d) closedir(d);
    CHECK(aside == 1);  // ...not deleted

    snprintf(path, sizeof path, "%s/notebook.ini", dir);
    f = fopen(path, "wb");
    if (f) fclose(f);  // truncated to nothing
    NotebookInfo back;
    CHECK(store_load_info(nb.id, &back) == 0);
    CHECK(back.npages == 2 && back.page_ids[0] == 1 && back.page_ids[1] == 3);  // p2 is set aside
}

// Folders sort by their letters, accents or not: an accented first letter sorts with its base
// letter, not after Z (the keyboard types accents).
TEST(folders_sort_with_accents) {
    const char *names[] = {"Zoolog\xc3\xad" "a", "\xc3\x81lgebra", "Arte", "\xc3\x89tudes", "school", "Stra\xc3\x9f" "e",
                           "Strasse", "\xc5\x81\xc3\xb3" "d\xc5\xba", "Lapiz", "e\xcc\x81poca"};
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++) CHECK(store_add_folder(names[i]) == 0);
    char (*fs)[STORE_NAME_LEN] = NULL;
    int n = 0;
    CHECK(store_list_folders(&fs, &n) == 0);
    // (ß sorts as ss; equal letters, the bytes decide: "Strasse" before "Straße"; a decomposed
    // e-acute sorts as e.)
    const char *want[] = {"\xc3\x81lgebra", "Arte", "e\xcc\x81poca", "\xc3\x89tudes", "Lapiz", "\xc5\x81\xc3\xb3" "d\xc5\xba",
                          "school", "Strasse", "Stra\xc3\x9f" "e", "Zoolog\xc3\xad" "a"};
    int at = 0;
    for (int i = 0; i < n && at < (int)(sizeof want / sizeof want[0]); i++)
        if (!strcmp(fs[i], want[at])) at++;
        else
            for (size_t k = 0; k < sizeof want / sizeof want[0]; k++)
                if (!strcmp(fs[i], want[k])) fprintf(stderr, "  folder %d: %s (wanted %s)\n", i, fs[i], want[at]);
    CHECK(at == (int)(sizeof want / sizeof want[0]));  // all of them, in this order
    free(fs);
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++) store_remove_folder(names[i]);
}

TEST(folder_named_like_a_comment) {
    CHECK(store_add_folder("#1") == 0);
    char (*fs)[STORE_NAME_LEN] = NULL;
    int n = 0;
    CHECK(store_list_folders(&fs, &n) == 0);
    bool found = false;
    for (int i = 0; i < n; i++) found |= !strcmp(fs[i], "#1");
    CHECK(found);
    free(fs);
}

TEST(safe_names) {
    char out[64];
    export_safe_name("a/b:c?", out, sizeof out);
    CHECK(!strcmp(out, "a_b_c_"));
    export_safe_name("  ..  ", out, sizeof out);
    CHECK(!strcmp(out, "Untitled"));
}

TEST(empty_mirror_folder_cleanup) {
    char mirror[700], path[850], outside[700], link[850];
    snprintf(mirror, sizeof mirror, "%s/prune-mirror", root);
    CHECK(mkdir_p(mirror) == 0);
    snprintf(path, sizeof path, "%s/Gone", mirror);
    CHECK(mkdir_p(path) == 0 && store_add_folder("Gone") == 0);
    CHECK(export_mirror_prune_folder("Gone", mirror) == 0 && access(path, F_OK) == 0);
    CHECK(store_remove_folder("Gone") == 0);
    CHECK(export_mirror_prune_folder("Gone", mirror) == 1 && access(path, F_OK) != 0);
    CHECK(export_mirror_prune_folder("", mirror) == 0 && access(mirror, F_OK) == 0);
    // Different logical names can share the same sanitized directory.
    snprintf(path, sizeof path, "%s/Shared_Name", mirror);
    CHECK(mkdir_p(path) == 0 && store_add_folder("Shared:Name") == 0);
    CHECK(export_mirror_prune_folder("Shared/Name", mirror) == 0 && access(path, F_OK) == 0);
    CHECK(store_remove_folder("Shared:Name") == 0);
    CHECK(export_mirror_prune_folder("Shared/Name", mirror) == 1);
    // Files and KOReader's nested sidecars are never removed recursively.
    snprintf(path, sizeof path, "%s/Kept/book.sdr", mirror);
    CHECK(mkdir_p(path) == 0);
    CHECK(export_mirror_prune_folder("Kept", mirror) == 0 && access(path, F_OK) == 0);
    snprintf(path, sizeof path, "%s/Kept/notes.txt", mirror);
    CHECK(write_file_atomic(path, "keep", 4) == 0);
    CHECK(export_mirror_prune_folder("Kept", mirror) == 0 && access(path, F_OK) == 0);
    // No following either a candidate directory symlink or a symlink mirror root.
    snprintf(outside, sizeof outside, "%s/outside", root);
    CHECK(mkdir_p(outside) == 0);
    snprintf(link, sizeof link, "%s/Link", mirror);
    char *absolute_outside = realpath(outside, NULL); CHECK(absolute_outside != NULL);
    CHECK(absolute_outside && symlink(absolute_outside, link) == 0); free(absolute_outside);
    CHECK(export_mirror_prune_folder("Link", mirror) == 0 && access(outside, F_OK) == 0);
    struct stat st; CHECK(lstat(link, &st) == 0 && S_ISLNK(st.st_mode));
    snprintf(path, sizeof path, "%s/Empty", mirror); CHECK(mkdir_p(path) == 0);
    snprintf(link, sizeof link, "%s/mirror-link", root);
    char *absolute_mirror = realpath(mirror, NULL); CHECK(absolute_mirror != NULL);
    CHECK(absolute_mirror && symlink(absolute_mirror, link) == 0); free(absolute_mirror);
    CHECK(export_mirror_prune_folder("Empty", link) == -1 && access(path, F_OK) == 0);
    CHECK(export_mirror_prune_folder("../outside", mirror) == 0 && access(outside, F_OK) == 0);
    // A remaining logical folder may refer to the candidate through an alias.
    snprintf(path, sizeof path, "%s/AliasTarget", mirror); CHECK(mkdir_p(path) == 0);
    char *absolute_target = realpath(path, NULL); CHECK(absolute_target != NULL);
    snprintf(link, sizeof link, "%s/Alias", mirror);
    CHECK(absolute_target && symlink(absolute_target, link) == 0); free(absolute_target);
    CHECK(store_add_folder("Alias") == 0);
    CHECK(export_mirror_prune_folder("AliasTarget", mirror) == 0 && access(path, F_OK) == 0);
    CHECK(store_remove_folder("Alias") == 0);
    CHECK(export_mirror_prune_folder("AliasTarget", mirror) == 1);
}

int main(void) {
    snprintf(root, sizeof root, "%s/store-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    RUN(notebooks);
    RUN(buffer_failure_stays_failed);
    RUN(new_page_after_a_document_page);
    RUN(copy_pages_keeps_bytes_and_order);
    RUN(copy_and_insert_fail_without_changing_the_page_list);
    RUN(fuzz_deserialize);
    RUN(custom_music_paper_roundtrip);
    RUN(settings_roundtrip);
    RUN(settings_upgrade_keeps_tweaks);
    RUN(settings_upgrade_ballpoint_smoothing);
    RUN(pen_defaults_from_device);
    RUN(panel_tools);
    RUN(before_eraser);
    RUN(safe_names);
    RUN(empty_mirror_folder_cleanup);
    RUN(damage_is_kept);
    RUN(folder_named_like_a_comment);
    RUN(folders_sort_with_accents);
    int rc = tl_done();
    system(cmd);
    return rc;
}
