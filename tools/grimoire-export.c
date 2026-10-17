// grimoire-export: render a Grimoire data root (e.g. rsynced off the Kindle) into PDFs / PNGs on a
// computer, with exactly the same renderer the Kindle uses.
//
//   grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>
//
// Writes <out>/<folder>/<name>.pdf (and <name>/page-NNN.png with --png, <name>.zip with --zip).
// Incremental: a notebook is skipped when its PDF is newer than its last modification.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "export.h"

static int write_out(const char *path, const Buf *b) {
    if (write_file_atomic(path, b->data, b->len) != 0) {
        fprintf(stderr, "grimoire-export: cannot write %s\n", path);
        return -1;
    }
    return 0;
}

int main(int argc, char **argv) {
    bool png = false, zip = false, force = false;
    const char *root = NULL, *out = NULL;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--png")) png = true;
        else if (!strcmp(argv[i], "--zip")) zip = true;
        else if (!strcmp(argv[i], "--force")) force = true;
        else if (!strcmp(argv[i], "-h") || !strcmp(argv[i], "--help")) root = out = NULL, argc = 0;
        else if (!root) root = argv[i];
        else if (!out) out = argv[i];
    }
    if (!root || !out) {
        fprintf(stderr, "usage: grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>\n");
        return 2;
    }
    if (store_init(root) != 0 || mkdir_p(out) != 0) return 1;
    const float dpi = 300.f;
    NotebookInfo *nbs;
    int n;
    if (store_list(&nbs, &n) != 0) {
        fprintf(stderr, "grimoire-export: no notebooks under %s/notebooks\n", root);
        return 1;
    }
    int done = 0, skipped = 0, failed = 0;
    for (int i = 0; i < n; i++) {
        const NotebookInfo *nb = &nbs[i];
        char sf[STORE_NAME_LEN], sn[STORE_NAME_LEN], dir[1200], path[1400];
        export_safe_name(nb->name, sn, sizeof sn);
        if (nb->folder[0]) {
            export_safe_name(nb->folder, sf, sizeof sf);
            snprintf(dir, sizeof dir, "%s/%s", out, sf);
        } else
            snprintf(dir, sizeof dir, "%s", out);
        mkdir_p(dir);
        snprintf(path, sizeof path, "%s/%s.pdf", dir, sn);
        struct stat st;
        if (!force && stat(path, &st) == 0 && (int64_t)st.st_mtime >= nb->modified) {
            skipped++;
            continue;
        }
        Buf b = {0};
        int rc = export_notebook_pdf(nb, dpi, &b);
        if (rc == 0) rc = write_out(path, &b);
        buf_free(&b);
        if (rc == 0 && png) {
            char pdir[1300];
            snprintf(pdir, sizeof pdir, "%s/%s", dir, sn);
            mkdir_p(pdir);
            for (int p = 0; p < nb->npages && rc == 0; p++) {
                Buf pb = {0};
                rc = export_page_png(nb, p, dpi, &pb);
                snprintf(path, sizeof path, "%s/page-%03d.png", pdir, p + 1);
                if (rc == 0) rc = write_out(path, &pb);
                buf_free(&pb);
            }
        }
        if (rc == 0 && zip) {
            Buf zb = {0};
            ZipWriter z;
            zip_begin(&z, &zb);
            rc = export_notebook_zip(nb, dpi, &z, "");
            zip_end(&z);
            snprintf(path, sizeof path, "%s/%s.zip", dir, sn);
            if (rc == 0) rc = write_out(path, &zb);
            buf_free(&zb);
        }
        if (rc == 0) {
            printf("  %s%s%s (%d page%s)\n", nb->folder[0] ? nb->folder : "", nb->folder[0] ? "/" : "", nb->name,
                   nb->npages, nb->npages == 1 ? "" : "s");
            done++;
        } else {
            fprintf(stderr, "grimoire-export: failed: %s\n", nb->name);
            failed++;
        }
    }
    printf("grimoire-export: %d exported, %d unchanged, %d failed -> %s\n", done, skipped, failed, out);
    free(nbs);
    return failed ? 1 : 0;
}
