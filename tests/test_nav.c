#include "nav.h"
#include "export.h"
#include "testlib.h"
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

static NotebookInfo nb;
static char path[800], links_path[800];

TEST(names_follow_page_ids) {
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) == 0 && n.writable && n.count == 0);
    CHECK(!*nav_name(&n, 10));
    CHECK(nav_set_name(&n, 40, "  Recipes\n2026  ") == 0);
    CHECK(nav_set_name(&n, 12, "Travel") == 0);
    CHECK(nav_set_name(&n, 0, "Never") != 0);
    CHECK(!strcmp(nav_name(&n, 40), "Recipes 2026"));
    NotebookInfo order = nb;
    order.npages = 4;
    order.page_ids[0] = 40; order.page_ids[1] = 7; order.page_ids[2] = 12; order.page_ids[3] = 5;
    int indices[4] = {-1, -1, -1, -1};
    CHECK(nav_index(&n, &order, indices, 4) == 2 && indices[0] == 0 && indices[1] == 2);
    CHECK(nav_section_of(&n, &order, -1) == -1);
    CHECK(nav_section_of(&n, &order, 1) == 0 && nav_section_of(&n, &order, 3) == 2);
    order.page_ids[0] = 12; order.page_ids[2] = 40;  // reorder, without changing either name
    CHECK(nav_index(&n, &order, indices, 1) == 2 && indices[0] == 0);
    CHECK(!strcmp(nav_name(&n, order.page_ids[2]), "Recipes 2026"));
    CHECK(nav_save(nb.id, &n, &order) == 0);
    CHECK(nav_save("other-notebook", &n, &order) != 0);
    Nav back; nav_init(&back);
    CHECK(nav_load(nb.id, &back) == 0 && back.count == 2);
    CHECK(!strcmp(nav_name(&back, 12), "Travel") && !strcmp(nav_name(&back, 40), "Recipes 2026"));
    CHECK(nav_set_name(&back, 12, "\t \r\n\177") == 0 && !*nav_name(&back, 12) && back.count == 1);
    nav_free(&back); nav_free(&n);
}

TEST(names_keep_unicode) {
    Nav n; nav_init(&n);
    const char *title = "\xe6\x97\xa5\xe8\xa8\x98 \xf0\x9f\x8e\xb5 = m\xc3\xbasica";
    CHECK(nav_set_name(&n, 1, title) == 0 && !strcmp(nav_name(&n, 1), title));
    char big[180];
    memset(big, 'a', 92);
    strcpy(big + 92, "\xc3\xa9\xf0\x9f\x8e\xb5 end");
    CHECK(nav_set_name(&n, 2, big) == 0 && strlen(nav_name(&n, 2)) == 94);
    CHECK(!strcmp(nav_name(&n, 2) + 92, "\xc3\xa9"));  // never half of the four-byte music note
    CHECK(nav_set_name(&n, 2, "\177 \tNew\tsection\177") == 0);
    CHECK(!strcmp(nav_name(&n, 2), "New section"));
    nav_free(&n);
}

TEST(future_lines_and_last_name_win) {
    const char *text = "# section names\r\nname.12=First\r\nlink.40=12345678,index\r\n"
                       "name.40=Recipes=2026\nname.12=Travel\nfuture.target=opaque value\n"
                       "name.4294967296=Future id\nname.-1=Unknown\nname.12=Last";
    CHECK(write_file_atomic(path, text, strlen(text)) == 0);
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) == 0 && n.count == 2);
    CHECK(!strcmp(nav_name(&n, 12), "Last") && !strcmp(nav_name(&n, 40), "Recipes=2026"));
    NotebookInfo order = nb; order.npages = 2; order.page_ids[0] = 12; order.page_ids[1] = 40;
    CHECK(nav_save(nb.id, &n, &order) == 0);
    size_t len; char *saved = read_file(path, &len);
    CHECK(saved && strstr(saved, "link.40=12345678,index\n") && strstr(saved, "future.target=opaque value\n"));
    CHECK(saved && strstr(saved, "name.4294967296=Future id\n") && strstr(saved, "name.-1=Unknown\n"));
    free(saved);
    nav_free(&n);
}

TEST(failed_save_does_not_prune) {
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) == 0 && n.count == 2);
    NotebookInfo order = nb; order.npages = 1; order.page_ids[0] = 40;
    char tmp[850]; snprintf(tmp, sizeof tmp, "%s.tmp", path);
    size_t old_len; char *old = read_file(path, &old_len);
    CHECK(mkdir(tmp, 0700) == 0);
    CHECK(nav_save(nb.id, &n, &order) != 0 && n.count == 2 && *nav_name(&n, 12));
    size_t len; char *saved = read_file(path, &len);
    CHECK(saved && old && len == old_len && !memcmp(saved, old, len));
    free(saved); free(old);
    CHECK(rmdir(tmp) == 0);
    CHECK(nav_save(nb.id, &n, &order) == 0 && n.count == 1 && !*nav_name(&n, 12));
    CHECK(nav_load(nb.id, &n) == 0 && n.count == 1);
    nav_free(&n);
}

static void refuses(const void *bytes, size_t len) {
    CHECK(write_file_atomic(path, bytes, len) == 0);
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) != 0 && !n.writable && n.count == 0);
    CHECK(nav_set_name(&n, 1, "Must not replace") != 0 && nav_save(nb.id, &n, &nb) != 0);
    size_t got; char *back = read_file(path, &got);
    CHECK(back && got == len && !memcmp(back, bytes, len));
    free(back); nav_free(&n);
}

TEST(unreadable_files_stay_untouched) {
    const char binary[] = "name.1=Hidden\0tail";
    refuses(binary, sizeof binary);
    char *huge = malloc(NAV_FILE_MAX + 1);
    CHECK(huge != NULL);
    if (!huge) return;
    memset(huge, 'x', NAV_FILE_MAX + 1);
    refuses(huge, NAV_FILE_MAX + 1);
    memset(huge, 'x', 256); huge[256] = '\n';
    refuses(huge, 257);  // an unknown line too long to carry safely
    for (int i = 0; i < NAV_UNKNOWN_MAX + 2; i++) huge[i] = i & 1 ? '\n' : 'x';
    refuses(huge, NAV_UNKNOWN_MAX + 2);
    free(huge);
    CHECK(unlink(path) == 0);
    CHECK(mkfifo(path, 0600) == 0);
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) != 0 && !n.writable);  // must not wait for a FIFO writer
    CHECK(nav_save(nb.id, &n, &nb) != 0);
    CHECK(unlink(path) == 0);
    CHECK(symlink("absent-index", path) == 0);
    CHECK(nav_load(nb.id, &n) != 0 && !n.writable);  // a dangling link is not an absent index
    CHECK(nav_save(nb.id, &n, &nb) != 0);
    struct stat st; CHECK(lstat(path, &st) == 0 && S_ISLNK(st.st_mode));
    CHECK(unlink(path) == 0);
    nav_free(&n);
}

TEST(raw_zip_keeps_index) {
    Nav n; nav_init(&n);
    CHECK(nav_set_name(&n, nb.page_ids[0], "Notes & recipes") == 0);
    CHECK(nav_save(nb.id, &n, &nb) == 0);
    Buf b = {0}; ZipWriter z;
    zip_begin(&z, &b);
    CHECK(export_notebook_zip(&nb, 300.f, &z, "") == 0 && zip_end(&z) == 0);
    char archive[850]; snprintf(archive, sizeof archive, "%s.zip", path);
    CHECK(write_file_atomic(archive, b.data, b.len) == 0);
    buf_free(&b); nav_free(&n);
    pid_t child = fork();
    CHECK(child >= 0);
    if (child == 0) {
        execlp("python3", "python3", "-c",
               "import zipfile,sys; z=zipfile.ZipFile(sys.argv[1]); "
               "ns=[n for n in z.namelist() if n.endswith('/raw/nav.ini')]; "
               "assert len(ns)==1; assert z.read(ns[0])==open(sys.argv[2],'rb').read()",
               archive, path, (char *)NULL);
        _exit(127);
    }
    int status = -1;
    CHECK(child > 0 && waitpid(child, &status, 0) == child && WIFEXITED(status) && WEXITSTATUS(status) == 0);
}

// A stroke through the points given (x, y pairs), with this seed.
static Stroke *stroke_at(uint32_t seed, ToolKind tool, const float *xy, int n) {
    ToolParams prm = {0};
    prm.size_min_mm = prm.size_max_mm = 0.5f;
    prm.opacity_min = prm.opacity_max = 1.f;
    Stroke *s = stroke_new(tool, &prm, seed);
    for (int i = 0; s && i < n; i++) {
        PenSample pt = {xy[2 * i], xy[2 * i + 1], 0.5f, 0, 0, (uint32_t)i};
        stroke_append(s, &pt, 300.f);
    }
    return s;
}

TEST(links_follow_their_boxes) {
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) == 0 && n.links_writable && !n.links_file && n.nlinks == 0);
    Page *p = page_new(600, 800, PAPER_BLANK);
    const float box[] = {100, 100, 200, 100, 200, 160, 100, 160, 100, 100};
    const float word[] = {120, 120, 180, 140};
    const float dot[] = {400, 400};
    page_add(p, stroke_at(0x1234abcd, TOOL_FINELINER, word, 2));
    page_add(p, stroke_at(0xa0a0a0a0, TOOL_FINELINER, box, 5));
    page_add(p, stroke_at(0x0000beef, TOOL_FINELINER, dot, 1));
    CHECK(nav_link_set(&n, 7, 0xa0a0a0a0, "p9") == 0);
    CHECK(nav_link_set(&n, 7, 0x0000beef, "index") == 0);
    CHECK(nav_link_set(&n, 7, 0x1111, "q9") != 0 && nav_link_set(&n, 7, 0x1111, "p0") != 0);
    CHECK(nav_link_set(&n, 7, 0x1111, "p09") != 0 && nav_link_set(&n, 0, 0x1111, "p9") != 0);
    CHECK(nav_link_set(&n, 3, 0x1111, "p7") == 0 && n.nlinks == 3);
    int first, count = nav_links_of(&n, 7, &first);
    CHECK(count == 2 && n.links[first].seed == 0x0000beef && n.links[first + 1].seed == 0xa0a0a0a0);
    // The box: the box stroke's points, the lone dot made 40 px a side around it.
    NavLink got[4]; Rect areas[4];
    CHECK(nav_link_areas(&n, p, 7, 40.f, got, areas, 4) == 2);
    CHECK(got[0].seed == 0x0000beef && areas[0].x == 380 && areas[0].y == 380 && areas[0].w == 40 && areas[0].h == 40);
    CHECK(got[1].seed == 0xa0a0a0a0 && areas[1].x == 100 && areas[1].y == 100 && areas[1].w == 100 && areas[1].h == 60);
    NavLink at;
    CHECK(nav_link_at(&n, p, 7, 150, 130, 40.f, &at) && at.seed == 0xa0a0a0a0 && !strcmp(at.target, "p9"));
    CHECK(nav_link_at(&n, p, 7, 401, 399, 40.f, &at) && !strcmp(at.target, "index"));
    CHECK(!nav_link_at(&n, p, 7, 300, 300, 40.f, &at));
    CHECK(!nav_link_at(&n, p, 3, 150, 130, 40.f, &at));  // another page's link, not this page's
    // The one drawn last wins where boxes overlap; a box with no strokes left does nothing.
    const float over[] = {140, 110, 260, 150};
    page_add(p, stroke_at(0x5555, TOOL_FINELINER, over, 2));
    CHECK(nav_link_set(&n, 7, 0x5555, "p3") == 0);
    CHECK(nav_link_at(&n, p, 7, 150, 130, 40.f, &at) && at.seed == 0x5555);
    CHECK(nav_link_at(&n, p, 7, 110, 150, 40.f, &at) && at.seed == 0xa0a0a0a0);
    stroke_free(page_remove(p, 3));
    CHECK(nav_link_at(&n, p, 7, 150, 130, 40.f, &at) && at.seed == 0xa0a0a0a0);
    CHECK(nav_link_areas(&n, p, 7, 40.f, NULL, NULL, 4) == 2 && nav_links_of(&n, 7, NULL) == 3);
    // An erase stroke with a link's seed is no part of its box.
    page_add(p, stroke_at(0xa0a0a0a0, TOOL_ERASER, dot, 1));
    CHECK(nav_link_areas(&n, p, 7, 40.f, got, areas, 4) == 2 && areas[1].w == 100);
    // New ink never takes a seed a link or an ink stroke has; an erase stroke's seed is free.
    CHECK(nav_new_seed(&n, p, 7, 0x1234abcd) != 0x1234abcd);
    CHECK(nav_new_seed(&n, p, 7, 0x5555) != 0x5555);  // its box is gone, its link is not
    uint32_t fresh = nav_new_seed(&n, p, 7, 0x0000beef);
    CHECK(fresh != 0x0000beef && fresh != 0xa0a0a0a0 && fresh != 0x1234abcd && fresh != 0x5555);
    CHECK(nav_new_seed(&n, p, 7, 0x42) == 0x42 && nav_new_seed(&n, p, 3, 0x5555) == 0x5555);
    nav_link_remove(&n, 7, 0x5555);
    CHECK(nav_links_of(&n, 7, NULL) == 2);
    CHECK(nav_link_set(&n, 7, 0xa0a0a0a0, "index") == 0 && nav_links_of(&n, 7, NULL) == 2);
    // Where they go.
    NotebookInfo order = nb; order.npages = 3;
    order.page_ids[0] = 9; order.page_ids[1] = 7; order.page_ids[2] = 3;
    NavLink to9 = {7, 1, "p9"}, toindex = {7, 1, "index"}, gone = {7, 1, "p44"}, future = {7, 1, "s2"};
    CHECK(nav_link_page(&to9, &order) == 0 && nav_link_page(&toindex, &order) == NAV_TO_INDEX);
    CHECK(nav_link_page(&gone, &order) == -1 && nav_link_page(&future, &order) == -1);
    page_free(p);
    nav_free(&n);
}

TEST(links_save_prune_and_carry) {
    unlink(links_path);
    Nav n; nav_init(&n);
    CHECK(nav_load(nb.id, &n) == 0);
    NotebookInfo order = nb; order.npages = 2; order.page_ids[0] = 9; order.page_ids[1] = 7;
    CHECK(nav_save_links(nb.id, &n, &order) == 0 && access(links_path, F_OK) != 0);  // none: no file
    CHECK(nav_link_set(&n, 7, 0xa0a0a0a0, "p9") == 0 && nav_link_set(&n, 7, 0xbeef, "index") == 0);
    CHECK(nav_link_set(&n, 9, 0x10, "p7") == 0 && nav_link_set(&n, 12, 0x10, "p7") == 0);  // 12: deleted
    CHECK(nav_link_set(&n, 9, 0x20, "p12") == 0);  // to a deleted page
    CHECK(nav_save_links(nb.id, &n, &order) == 0 && n.nlinks == 3);
    size_t len; char *saved = read_file(links_path, &len);
    CHECK(saved && !strcmp(saved, "# Grimoire: links drawn on pages\nlink.7=0000beef,index\n"
                                  "link.7=a0a0a0a0,p9\nlink.9=00000010,p7\n"));
    free(saved);
    const char *text = "# links\r\nlink.7=A0A0A0A0,p7\r\nlink.7=BEEF,p9\nlink.9=1,s3/e2\nlink.9=123456789,p7\n"
                       "link.9=zz,p7\nlink.0=1,p7\nlink.8=1,\nlink.8=2,0123456789abcdef\nfuture=kept\nlink.7=a0a0a0a0,index";
    CHECK(write_file_atomic(links_path, text, strlen(text)) == 0);
    CHECK(nav_load(nb.id, &n) == 0 && n.nlinks == 3 && n.links_unknown_len);
    int first;
    CHECK(nav_links_of(&n, 7, &first) == 2 && !strcmp(n.links[first].target, "p9") &&
          !strcmp(n.links[first + 1].target, "index"));  // the last line for a link wins
    CHECK(nav_links_of(&n, 9, &first) == 1 && !strcmp(n.links[first].target, "s3/e2"));
    CHECK(nav_save_links(nb.id, &n, &order) == 0);
    saved = read_file(links_path, &len);
    CHECK(saved && strstr(saved, "link.9=00000001,s3/e2\n") && strstr(saved, "link.9=123456789,p7\n"));
    CHECK(saved && strstr(saved, "link.9=zz,p7\n") && strstr(saved, "link.0=1,p7\n") && strstr(saved, "future=kept\n"));
    CHECK(saved && strstr(saved, "link.8=2,0123456789abcdef\n") && strstr(saved, "link.8=1,\n"));
    free(saved);
    // Every link gone: the file stays, empty of them.
    nav_link_remove(&n, 7, 0xbeef); nav_link_remove(&n, 7, 0xa0a0a0a0); nav_link_remove(&n, 9, 1);
    CHECK(nav_save_links(nb.id, &n, &order) == 0 && access(links_path, F_OK) == 0);
    nav_free(&n);
    unlink(links_path);
}

TEST(links_limits_and_unreadable) {
    Nav n; nav_init(&n);
    for (uint32_t s = 0; s < NAV_LINKS_PER_PAGE; s++) CHECK(nav_link_set(&n, 5, s, "index") == 0);
    CHECK(nav_link_set(&n, 5, 9999, "index") != 0);  // a full page
    CHECK(nav_link_set(&n, 5, 3, "p8") == 0);         // a new target for one it has
    nav_free(&n);
    // An unreadable links.ini keeps the names working and is never written over.
    const char binary[] = "link.1=1,index\0tail";
    CHECK(write_file_atomic(links_path, binary, sizeof binary) == 0);
    nav_init(&n);
    CHECK(nav_set_name(&n, nb.page_ids[0], "Still named") == 0 && nav_save(nb.id, &n, &nb) == 0);
    CHECK(nav_load(nb.id, &n) != 0 && n.writable && !n.links_writable && !strcmp(nav_name(&n, nb.page_ids[0]), "Still named"));
    CHECK(nav_link_set(&n, nb.page_ids[0], 1, "index") != 0 && nav_save_links(nb.id, &n, &nb) != 0);
    CHECK(nav_set_name(&n, nb.page_ids[0], "Renamed") == 0 && nav_save(nb.id, &n, &nb) == 0);
    size_t got; char *back = read_file(links_path, &got);
    CHECK(back && got == sizeof binary && !memcmp(back, binary, got));
    free(back);
    // And an unreadable nav.ini keeps the links.
    CHECK(unlink(links_path) == 0);
    CHECK(nav_load(nb.id, &n) == 0 && nav_link_set(&n, nb.page_ids[0], 1, "index") == 0 &&
          nav_save_links(nb.id, &n, &nb) == 0);
    CHECK(write_file_atomic(path, binary, sizeof binary) == 0);
    CHECK(nav_load(nb.id, &n) != 0 && !n.writable && n.links_writable && n.nlinks == 1);
    nav_free(&n);
    CHECK(unlink(links_path) == 0 && unlink(path) == 0);
}

// Every xref entry of a PDF points at its object: "<n> 0 obj".
static bool xref_holds(const Buf *b) {
    char *text = malloc(b->len + 1);
    if (!text) return false;
    memcpy(text, b->data, b->len);
    text[b->len] = 0;
    for (size_t i = 0; i < b->len; i++) if (!text[i]) text[i] = ' ';
    char *sx = strstr(text, "startxref\n");
    bool ok = false;
    if (sx) {
        size_t at = strtoul(sx + 10, NULL, 10);
        int count = 0;
        if (at < b->len && sscanf(text + at, "xref\n0 %d", &count) == 1 && count > 1) {
            char *row = strchr(strchr(text + at, '\n') + 1, '\n') + 1 + 20;  // past the free entry
            ok = true;
            for (int n = 1; n < count && ok; n++, row += 20) {
                size_t obj = strtoul(row, NULL, 10);
                char want[32];
                int wl = snprintf(want, sizeof want, "%d 0 obj\n", n);
                ok = obj < b->len && !strncmp(text + obj, want, (size_t)wl);
            }
        }
    }
    free(text);
    return ok;
}

TEST(pdf_has_the_outline_and_links) {
    NotebookInfo book;
    CHECK(store_create("Linked", "", PAPER_BLANK, &book) == 0);
    CHECK(store_add_page(&book, 1, PAPER_BLANK) == 0 && store_add_page(&book, 2, PAPER_BLANK) == 0);
    Buf plain = {0};
    CHECK(export_notebook_pdf(&book, 300.f, &plain) == 0 && xref_holds(&plain));
    CHECK(plain.data && !memmem(plain.data, plain.len, "/Outlines", 9) && !memmem(plain.data, plain.len, "/Annots", 7));
    Page *p = NULL;
    CHECK(store_load_page(&book, 0, book.page_w, book.page_h, &p) == 0 && p);
    if (!p) return;
    const float box[] = {300, 600, 900, 600, 900, 900, 300, 900};
    const float dot[] = {1000, 1000};
    page_add(p, stroke_at(0x77, TOOL_FINELINER, box, 4));
    page_add(p, stroke_at(0x88, TOOL_FINELINER, dot, 1));
    CHECK(store_save_page(&book, 0, p) == 0);
    page_free(p);
    Nav n; nav_init(&n);
    char to3[16]; snprintf(to3, sizeof to3, "p%u", book.page_ids[2]);
    CHECK(nav_load(book.id, &n) == 0 && nav_set_name(&n, book.page_ids[0], "Start \xc3\xa9") == 0 &&
          nav_set_name(&n, book.page_ids[2], "End") == 0 && nav_save(book.id, &n, &book) == 0);
    CHECK(nav_link_set(&n, book.page_ids[0], 0x77, to3) == 0 && nav_link_set(&n, book.page_ids[0], 0x88, "index") == 0);
    CHECK(nav_save_links(book.id, &n, &book) == 0);
    nav_free(&n);
    Buf b = {0};
    CHECK(export_notebook_pdf(&book, 300.f, &b) == 0 && xref_holds(&b));
    // 3 pages: objects 4 to 12, the outline 13 to 15, the one link to a page 16 (the Index's has none).
    char *t = calloc(1, b.len + 1);
    if (t && b.data) memcpy(t, b.data, b.len);
    for (size_t i = 0; t && i < b.len; i++) if (!t[i]) t[i] = ' ';
    CHECK(t && strstr(t, "<< /Type /Catalog /Pages 2 0 R /Outlines 13 0 R /PageMode /UseOutlines >>"));
    CHECK(t && strstr(t, "/Resources << /XObject << /Im0 6 0 R >> >> /Annots [ 16 0 R ] >>"));
    CHECK(t && strstr(t, "/Resources << /XObject << /Im0 9 0 R >> >> >>"));
    float s = 72.f / 300.f, ph = (float)book.page_h * s;
    char want[256];
    snprintf(want, sizeof want, "16 0 obj\n<< /Type /Annot /Subtype /Link /Rect [%.2f %.2f %.2f %.2f] /Border [0 0 0] "
             "/Dest [10 0 R /Fit] >>", 300 * s, ph - 900 * s, 900 * s, ph - 600 * s);
    CHECK(t && strstr(t, want));
    CHECK(t && strstr(t, "13 0 obj\n<< /Type /Outlines /First 14 0 R /Last 15 0 R /Count 2 >>"));
    CHECK(t && strstr(t, "14 0 obj\n<< /Title <FEFF00530074006100720074002000E9> /Parent 13 0 R "
                         "/Next 15 0 R /Dest [4 0 R /Fit] >>"));
    CHECK(t && strstr(t, "15 0 obj\n<< /Title <FEFF0045006E0064> /Parent 13 0 R /Prev 14 0 R /Dest [10 0 R /Fit] >>"));
    CHECK(t && strstr(t, "trailer\n<< /Size 17 "));
    free(t);
    buf_free(&b); buf_free(&plain);
}

int main(void) {
    char root[700], cmd[800], dir[700];
    snprintf(root, sizeof root, "%s/nav-%d", tl_tmp(), (int)getpid());
    CHECK(store_init(root) == 0 && store_create("Sections", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_notebook_dir(nb.id, dir, sizeof dir) == 0);
    snprintf(path, sizeof path, "%s/nav.ini", dir);
    snprintf(links_path, sizeof links_path, "%s/links.ini", dir);
    RUN(names_follow_page_ids);
    RUN(names_keep_unicode);
    RUN(future_lines_and_last_name_win);
    RUN(failed_save_does_not_prune);
    RUN(unreadable_files_stay_untouched);
    RUN(raw_zip_keeps_index);
    RUN(links_follow_their_boxes);
    RUN(links_save_prune_and_carry);
    RUN(links_limits_and_unreadable);
    RUN(pdf_has_the_outline_and_links);
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    CHECK(system(cmd) == 0);
    return tl_done();
}
