// bgwork_template_probe.c: what src/bgwork_template.c answers for its helpers that need no other module,
// printed as lines. rust/src/bgwork_template.rs (tests::matches_the_c_probe) builds the same lines from the
// Rust twins and compares them with bgwork_template_probe_c.txt, which is what this printed: the hash of a
// preview's name, the natural order, the key file's reading and the pictures' hash are formats the Kindle's
// caches already hold, so the Rust build must name and read them as the C does.
//
// Build and run (from the repository's top level, with the core sources of the Makefile less this one):
//   clang -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -Ibuild/gen -DGRIMOIRE_VERSION='"0"' -ffp-contract=on \
//     rust/tests/fixtures/bgwork_template_probe.c <CORE_SRC but src/bgwork_template.c> <UI_SRC> build/gen/{fonts,l10n,eraser_sample}.c \
//     -o build/probe -lm
//   build/probe <an empty scratch folder> > rust/tests/fixtures/bgwork_template_probe_c.txt
// The folder is cleared and filled with files of fixed sizes and times, and nothing in the output names it.
#include "bgwork_template.c"

#include <sys/time.h>

static char R[600];

static void mkparent(const char *path) {
    char dir[1200];
    snprintf(dir, sizeof dir, "%s", path);
    char *slash = strrchr(dir, '/');
    if (slash) *slash = 0;
    mkdir_p(dir);
}

static void put(const char *rel, const void *text, size_t n, long t) {
    char path[1200];
    snprintf(path, sizeof path, "%s/%s", R, rel);
    mkparent(path);
    FILE *f = fopen(path, "wb");
    if (f) {
        fwrite(text, 1, n, f);
        fclose(f);
    }
    struct timeval tv[2] = {{t, 0}, {t, 0}};
    utimes(path, tv);
}

static const char *rel(const char *p) {
    size_t rl = strlen(R);
    return !strncmp(p, R, rl) ? p + rl : p;
}

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    snprintf(R, sizeof R, "%s", argv[1]);
    char cmd[1300];
    snprintf(cmd, sizeof cmd, "rm -rf '%s' && mkdir -p '%s'", R, R);
    if (system(cmd)) return 2;

    // natural order: every pair of a list
    static const char *const L[] = {"", "a", "A", "b", "B", "a1", "a01", "a001", "a2", "a10", "a1b", "a1B", "a01b", "1", "01", "001", "0", "00", "9",
                                    "10", "x", "x1", "x1y2", "x1y10", "page-2", "page-10", "Page-3", "page-002", "page-3", "scan 1 of 12",
                                    "scan 12 of 12", "z\xc3\xa9", "z\xc3\x89", "\xc3\xa9", ".a", "a.", "a-b", "a_b", "99999999999999999999",
                                    "100000000000000000000", "0000000000000000000001", "1.5", "1.10", "a:", "a/", "a9", "a:9"};
    int n = (int)(sizeof L / sizeof L[0]);
    for (int i = 0; i < n; i++) {
        char line[400] = "";
        for (int j = 0; j < n; j++) {
            int c = natural_cmp(L[i], L[j]);
            strcat(line, c < 0 ? "<" : c > 0 ? ">" : "=");
        }
        printf("NAT %2d %s\n", i, line);
    }
    // names
    static const char *const N[] = {"a.png", "a.PNG", "a.jpg", "a.JPEG", "a.jpeg", "a.pgm", "a.Pgm", ".a.png", "a", "a.gif", "a.png.txt", ".png", "png",
                                    "a.b.jpeg", "x.tiff", "a.jpg ", "A.JpG", "dir.png/", "a.pn", "a.pngg", ""};
    for (size_t i = 0; i < sizeof N / sizeof N[0]; i++) printf("PICNAME [%s] %d\n", N[i], (int)picture_name(N[i]));
    static const char *const F[] = {"/x/y/Name", "Name", "/x/y/", "/", "", "/a/b c/\xc3\xa9", "/mnt/us/grimoire/import/Score"};
    for (size_t i = 0; i < sizeof F / sizeof F[0]; i++) printf("FNAME [%s] %016llx\n", F[i], (unsigned long long)folder_name_hash(F[i]));
    printf("FNV %016llx %016llx\n", (unsigned long long)fnv(FNV0, "", 0), (unsigned long long)fnv(FNV0, "abc", 3));
    char name[STORE_NAME_LEN], longname[300] = "", odd[300] = "a";
    for (int i = 0; i < 70; i++) strcat(longname, "\xc3\xa9");
    copy_name(name, longname, sizeof name);
    printf("COPYNAME %zu\n", strlen(name));
    copy_name(name, "short", sizeof name);
    printf("COPYNAME [%s]\n", name);
    for (int i = 0; i < 94; i++) strcat(odd, "b");
    strcat(odd, "\xc3\xa9");
    copy_name(name, odd, sizeof name);
    printf("COPYNAME %zu\n", strlen(name));

    // a preview's name: the hash of the file's name, size and time
    static const char *const T[] = {"t-10.png", "T-3.JPG", "b.jpeg", "caf\xc3\xa9.png", "a b.pgm", "x"};
    long when = 1700000000L;
    for (size_t i = 0; i < sizeof T / sizeof T[0]; i++) {
        char r[100];
        snprintf(r, sizeof r, "templates/%s", T[i]);
        put(r, "0123456789abcdefghij", 10 + i, when + (long)i * 1000);
    }
    char dir[700];
    snprintf(dir, sizeof dir, "%s/templates", R);
    for (size_t i = 0; i < sizeof T / sizeof T[0]; i++) {
        char a[PATH_BUF], b[PATH_BUF], c[PATH_BUF], d[PATH_BUF];
        int ok = preview_path(dir, T[i], "pgm", a, sizeof a) && preview_path(dir, T[i], "key", b, sizeof b) && preview_path(dir, T[i], "bad", c, sizeof c);
        char full[900];
        snprintf(full, sizeof full, "%s/%s", dir, T[i]);
        int of = preview_of(full, "pgm", d, sizeof d);
        printf("PREVIEW [%s] %d %s %s %s of=%d %d\n", T[i], ok, rel(a), rel(b), rel(c), of, of && !strcmp(a, d));
    }
    char a[PATH_BUF];
    printf("PREVIEW missing %d\n", (int)preview_path(dir, "nope.png", "pgm", a, sizeof a));
    printf("PREVIEW noslash %d\n", (int)preview_of("no-slash.png", "pgm", a, sizeof a));
    char longp[700];
    memset(longp, 'x', 650);
    longp[0] = '/';
    longp[650] = 0;
    printf("PREVIEW long %d\n", (int)preview_of(longp, "pgm", a, sizeof a));

    // the key file a previews job leaves beside a preview
    put("keys/k.png", "0123", 4, when + 5000);
    static const char *const K[] = {"00000000000000ff 600 800\n", "ff 1 2", "  FF  -5  7", "0x1F 3 4", "123456789abcdef01 5 6", "xyz", "", "ff 1", "ff 1 2 3 4",
                                    "ff\n1\n2\n", "1 -1 1", "-1 1 1", "+a 1 1", "ffffffffffffffff 1 1", "abc 0 0", "abc +7 -8x", "abc\t1\t2",
                                    "deadbeefdeadbeef 1280 1707\nextra line\n", "0 0 0", "   \n  1 2 3", "1,2,3", "00ff 600 800", "0000000000000000ff 1 2"};
    char pth[700];
    snprintf(pth, sizeof pth, "%s/keys/k.png", R);
    for (size_t i = 0; i < sizeof K / sizeof K[0]; i++) {
        char kp[PATH_BUF];
        preview_of(pth, "key", kp, sizeof kp);
        mkparent(kp);
        FILE *f = fopen(kp, "wb");
        fputs(K[i], f);
        fclose(f);
        uint64_t h = 7;
        int w = -9, hh = -9;
        int rc = cached_key(pth, &h, &w, &hh);
        if (rc == 0) printf("KEY %zu rc=0 hash=%016llx w=%d h=%d\n", i, (unsigned long long)h, w, hh);
        else printf("KEY %zu rc=%d\n", i, rc);
    }
    snprintf(pth, sizeof pth, "%s/keys/nofile.png", R);
    uint64_t h = 7;
    int w = 0, hh = 0;
    printf("KEY nofile rc=%d\n", cached_key(pth, &h, &w, &hh));
    printf("KEY noslash rc=%d\n", cached_key("k.png", &h, &w, &hh));

    // listing a folder, and the hash of its pictures
    for (int i = 0; i < 6; i++) {
        char r[100];
        snprintf(r, sizeof r, "l/p%d.png", i);
        put(r, "xxxxxxxx", 1 + (size_t)i, when + 3000 + i * 7);
    }
    put("l/q.txt", "x", 1, when);
    put("l/.h.png", "x", 1, when);
    put("l/d1/x", "x", 1, when);
    put("l/d2.png/x", "x", 1, when);
    snprintf(dir, sizeof dir, "%s/l", R);
    char **names;
    printf("LIST pics rc=%d\n", list_names(dir, true, 100, &names, &n));
    for (int i = 0; i < n; i++) printf(" %s", names[i]);
    printf("\nPICSHASH %016llx\n", (unsigned long long)pictures_hash(dir, names, n));
    free_names(names, n);
    printf("LIST dirs rc=%d\n", list_names(dir, false, 100, &names, &n));
    for (int i = 0; i < n; i++) printf(" %s", names[i]);
    printf("\n");
    free_names(names, n);
    printf("LIST max rc=%d\n", list_names(dir, true, 3, &names, &n));
    printf("LIST max n=%d\n", n);
    free_names(names, n);
    printf("FOLDERHASH %016llx\n", (unsigned long long)folder_pictures_hash(dir));
    snprintf(dir, sizeof dir, "%s/l/missing", R);
    printf("LIST missing rc=%d\n", list_names(dir, true, 3, &names, &n));
    printf("FOLDERHASH missing %016llx\n", (unsigned long long)folder_pictures_hash(dir));
    return 0;
}
