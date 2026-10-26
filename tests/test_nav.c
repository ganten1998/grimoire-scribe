#include "nav.h"
#include "export.h"
#include "testlib.h"
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

static NotebookInfo nb;
static char path[800];

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

TEST(back_history_uses_ids) {
    NavHistory h = {0};
    NotebookInfo order = nb; order.npages = 3;
    order.page_ids[0] = 12; order.page_ids[1] = 40; order.page_ids[2] = 7;
    nav_history_push(&h, 12); nav_history_push(&h, 12); nav_history_push(&h, 40);
    CHECK(h.count == 2);
    order.page_ids[0] = 7; order.page_ids[2] = 12;
    CHECK(nav_history_pop(&h, &order, 40) == 2);
    CHECK(nav_history_pop(&h, &order, 12) == -1);
    for (int i = 1; i <= 40; i++) nav_history_push(&h, (uint32_t)i);
    CHECK(h.count == NAV_HISTORY_MAX && h.ids[0] == 25 && h.ids[15] == 40);
    CHECK(nav_history_pop(&h, &order, 12) == 1);  // skip ids of pages not in the notebook
    CHECK(nav_history_pop(&h, &order, 40) == -1);
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

int main(void) {
    char root[700], cmd[800], dir[700];
    snprintf(root, sizeof root, "%s/nav-%d", tl_tmp(), (int)getpid());
    CHECK(store_init(root) == 0 && store_create("Sections", "", PAPER_MEDIUM, &nb) == 0);
    CHECK(store_notebook_dir(nb.id, dir, sizeof dir) == 0);
    snprintf(path, sizeof path, "%s/nav.ini", dir);
    RUN(names_follow_page_ids);
    RUN(names_keep_unicode);
    RUN(future_lines_and_last_name_win);
    RUN(failed_save_does_not_prune);
    RUN(unreadable_files_stay_untouched);
    RUN(back_history_uses_ids);
    RUN(raw_zip_keeps_index);
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    CHECK(system(cmd) == 0);
    return tl_done();
}
