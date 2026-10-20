// bgwork_template.c: templates and imports (templates.h), and the background work behind them:
// BG_TEMPLATE (a template's picture made for a notebook, or the previews of the templates folder)
// and BG_IMPORT_FOLDER (a folder of page pictures made into a notebook), run in a forked child by
// bgjob.c.
//
// grimoire.log goes into public bug reports: the names of the owner's files and folders are never
// logged here, only notebook ids and counts.
#include "templates.h"

#include <ctype.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#include "bgimage.h"
#include "bgjob.h"
#include "i18n.h"

#define MAX_SOURCE_BYTES (256u << 20)  // a picture file bigger than this is not read
#define TEMPLATE_MAX_BYTES (64u << 20) // nor a template this big (its key is read on the UI thread)
#define MAX_TEMPLATES 500              // files of the templates folder looked at
#define IMPORT_QUEUE_MAX 8             // import folders queued at one start (the job queue holds 16)
#define IMPORT_MARK ".grimoire-import" // in an import folder: the notebook it becomes ("done <id>" once made)

static void root_path(const char *sub, char *buf, size_t cap) { snprintf(buf, cap, "%s/%s", store_root(), sub); }

// ---- names ----

static bool is_digit(char c) { return c >= '0' && c <= '9'; }

int natural_cmp(const char *a, const char *b) {
    const char *x = a, *y = b;
    while (*x && *y) {
        if (is_digit(*x) && is_digit(*y)) {
            while (*x == '0') x++;
            while (*y == '0') y++;
            const char *xe = x, *ye = y;
            while (is_digit(*xe)) xe++;
            while (is_digit(*ye)) ye++;
            if (xe - x != ye - y) return xe - x < ye - y ? -1 : 1;  // more digits: a bigger number
            int c = memcmp(x, y, (size_t)(xe - x));
            if (c) return c < 0 ? -1 : 1;
            x = xe;
            y = ye;
            continue;
        }
        int cx = tolower((unsigned char)*x), cy = tolower((unsigned char)*y);
        if (cx != cy) return cx < cy ? -1 : 1;
        x++;
        y++;
    }
    if (*x || *y) return *x ? 1 : -1;  // the one that ran out first goes first
    int c = strcmp(a, b);
    return c < 0 ? -1 : c > 0;
}

static int cmp_names(const void *a, const void *b) { return natural_cmp(*(char *const *)a, *(char *const *)b); }

// A picture file by its name: .png, .jpg, .jpeg or .pgm in any case, and never a dot-file.
static bool picture_name(const char *n) {
    const char *dot = strrchr(n, '.');
    if (n[0] == '.' || !dot) return false;
    return !strcasecmp(dot, ".png") || !strcasecmp(dot, ".jpg") || !strcasecmp(dot, ".jpeg") || !strcasecmp(dot, ".pgm");
}

static bool is_dir(const char *path) {
    struct stat st;
    return stat(path, &st) == 0 && S_ISDIR(st.st_mode);
}

static void free_names(char **v, int n) {
    for (int i = 0; i < n; i++) free(v[i]);
    free(v);
}

// The entries of dir that are picture files (pictures) or folders (!pictures), in natural order,
// at most max. *out is freed with free_names. 0, or -1 without the folder.
static int list_names(const char *dir, bool pictures, int max, char ***out, int *n) {
    *out = NULL;
    *n = 0;
    DIR *d = opendir(dir);
    if (!d) return -1;
    int cap = 0;
    struct dirent *e;
    while ((e = readdir(d)) && *n < max) {
        if (e->d_name[0] == '.') continue;
        if (pictures && !picture_name(e->d_name)) continue;
        char path[1200];
        snprintf(path, sizeof path, "%s/%s", dir, e->d_name);
        if (is_dir(path) == pictures) continue;
        if (*n == cap) {
            int nc = cap ? cap * 2 : 16;
            char **nv = realloc(*out, (size_t)nc * sizeof *nv);
            if (!nv) break;
            *out = nv;
            cap = nc;
        }
        if (!((*out)[*n] = strdup(e->d_name))) break;
        (*n)++;
    }
    closedir(d);
    if (*n) qsort(*out, (size_t)*n, sizeof **out, cmp_names);
    return 0;
}

// A name cut to fit cap bytes at the start of a character, never inside one.
static void copy_name(char *dst, const char *src, size_t cap) {
    size_t n = strlcpy_q(dst, src, cap);
    if (((uint8_t)src[n] & 0xC0) == 0x80) {
        while (n > 0 && ((uint8_t)dst[n - 1] & 0xC0) == 0x80) n--;
        if (n > 0 && ((uint8_t)dst[n - 1] & 0xC0) == 0xC0) n--;
        dst[n] = 0;
    }
}

// A whole picture file, or NULL (missing, or bigger than MAX_SOURCE_BYTES).
static uint8_t *read_source(const char *path, size_t *len) {
    struct stat st;
    if (stat(path, &st) != 0 || !S_ISREG(st.st_mode) || (uint64_t)st.st_size > MAX_SOURCE_BYTES) return NULL;
    return read_file(path, len);
}

// A small file written whole (write, fsync, rename), its path never logged: it is in the owner's
// own folder, whose name is theirs.
static int write_quiet(const char *path, const char *text) {
    char tmp[1300];
    snprintf(tmp, sizeof tmp, "%s.tmp", path);
    int fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) return -1;
    size_t len = strlen(text);
    bool ok = write(fd, text, len) == (ssize_t)len && fsync(fd) == 0;
    ok = close(fd) == 0 && ok;
    if (!ok || rename(tmp, path) != 0) {
        unlink(tmp);
        return -1;
    }
    return 0;
}

// ---- the folders ----

void templates_init(void) {
    char dir[600], path[700];
    root_path("templates", dir, sizeof dir);
    if (mkdir_p(dir) == 0) {
        snprintf(path, sizeof path, "%s/README.txt", dir);
        if (access(path, F_OK) != 0 &&
            write_quiet(path, _("Templates for Grimoire\n\n"
                                "Copy pictures here to write on them as paper: PNG or JPG files, upright, in the shape of a "
                                "Kindle Scribe page (3:4). 1860 x 2480 pixels is best. Light lines on white work best on "
                                "e-ink: dark areas hide your writing.\n\n"
                                "Copy them in over USB while Grimoire is closed. A notebook keeps its own copy of each "
                                "template it uses, so changing or deleting a file here never changes pages already written "
                                "on it. Grimoire keeps its previews of these files in the .cache folder.\n")) != 0)
            qlog("templates: could not write templates/README.txt");
    }
    root_path("import", dir, sizeof dir);
    if (mkdir_p(dir) == 0) {
        snprintf(path, sizeof path, "%s/README.txt", dir);
        if (access(path, F_OK) != 0 &&
            write_quiet(path, _("Importing pages into Grimoire\n\n"
                                "To write on pages you already have, such as a PDF saved as pictures, put a folder of PNG or "
                                "JPG files here, one picture a page, and start Grimoire. The folder's name becomes the "
                                "notebook's name, and the pages follow the file names in order (page-2 comes before "
                                "page-10). Gray pictures import faster and take less room than colour ones.\n\n"
                                "Grimoire imports each folder in the background and then moves it into the done folder. Copy "
                                "folders in over USB while Grimoire is closed.\n")) != 0)
            qlog("import: could not write import/README.txt");
    }
}

// ---- template previews ----

#define THUMB_DIV 6  // previews are 1/6 of the page, as the paper sheets' tiles

// The preview of a template file: .cache/<hash of its name, size and time>.pgm (and .bad for one that
// does not read), so a file replaced by another gets a preview of its own.
static bool preview_path(const char *dir, const char *name, const char *ext, char *buf, size_t cap) {
    char path[1200];
    snprintf(path, sizeof path, "%s/%s", dir, name);
    struct stat st;
    if (stat(path, &st) != 0) return false;
    uint64_t h = 14695981039346656037ull;  // FNV-1a
    char tail[64];
    snprintf(tail, sizeof tail, "/%lld/%lld", (long long)st.st_size, (long long)st.st_mtime);
    for (const char *s = name; *s; s++) h = (h ^ (uint8_t)*s) * 1099511628211ull;
    for (const char *s = tail; *s; s++) h = (h ^ (uint8_t)*s) * 1099511628211ull;
    snprintf(buf, cap, "%s/.cache/%016llx.%s", dir, (unsigned long long)h, ext);
    return true;
}

int template_scan(Template **out, int *n, float dpi) {
    *out = NULL;
    *n = 0;
    char dir[600];
    root_path("templates", dir, sizeof dir);
    char **names;
    int nn;
    if (list_names(dir, true, MAX_TEMPLATES, &names, &nn) != 0) return -1;
    Template *t = calloc((size_t)(nn ? nn : 1), sizeof *t);
    if (!t) {
        free_names(names, nn);
        return -1;
    }
    bool missing = false;
    for (int i = 0; i < nn; i++) {
        snprintf(t[i].path, sizeof t[i].path, "%s/%s", dir, names[i]);
        char base[STORE_NAME_LEN * 2];
        strlcpy_q(base, names[i], sizeof base);
        char *dot = strrchr(base, '.');
        if (dot) *dot = 0;
        copy_name(t[i].name, base, sizeof t[i].name);
        char pv[1300];
        if (!preview_path(dir, names[i], "pgm", pv, sizeof pv)) continue;
        if (store_read_pgm(pv, &t[i].thumb) == 0) continue;
        preview_path(dir, names[i], "bad", pv, sizeof pv);
        if (access(pv, F_OK) == 0) t[i].bad = true;
        else missing = true;
    }
    free_names(names, nn);
    if (missing) bg_queue(BG_TEMPLATE, "", dir, dpi, false);
    *out = t;
    *n = nn;
    return 0;
}

void template_list_free(Template *t, int n) {
    for (int i = 0; i < n && t; i++) canvas_free(&t[i].thumb);
    free(t);
}

// In the child: the previews the templates folder lacks, made at the page size of this Kindle;
// previews of files no longer there are removed.
static int make_previews(const char *dir) {
    char cache[700];
    snprintf(cache, sizeof cache, "%s/.cache", dir);
    if (mkdir_p(cache) != 0) return -1;
    char **names;
    int nn;
    if (list_names(dir, true, MAX_TEMPLATES, &names, &nn) != 0) return -1;
    int pw, ph, made = 0, bad = 0;
    store_page_size(&pw, &ph);
    char (*keep)[32] = calloc((size_t)(nn ? nn : 1), sizeof *keep);
    for (int i = 0; i < nn && keep; i++) {
        char pv[1300], bp[1300], path[1200];
        if (!preview_path(dir, names[i], "pgm", pv, sizeof pv)) continue;
        preview_path(dir, names[i], "bad", bp, sizeof bp);
        const char *slash = strrchr(pv, '/');
        strlcpy_q(keep[i], slash ? slash + 1 : pv, sizeof keep[i]);
        keep[i][16] = 0;  // the hash, without the extension
        if (access(pv, F_OK) == 0 || access(bp, F_OK) == 0) continue;
        snprintf(path, sizeof path, "%s/%s", dir, names[i]);
        size_t len;
        uint8_t *data = read_source(path, &len);
        BgPicture src = {0}, fit = {0};
        Canvas full = {0}, small = {0};
        const char *why = "too big a file";
        bool ok = data && bgimage_decode(data, len, &src, &why) == 0 && bgimage_normalize(&src, pw, ph, &fit) == 0 &&
                  bgimage_luma(&fit, &full) == 0 && canvas_init(&small, pw / THUMB_DIV, ph / THUMB_DIV) == 0;
        if (ok) {
            canvas_downsample_keep_lines(&small, &full, 0.6f);
            ok = store_write_pgm(pv, &small) == 0;
            made += ok;
        } else {
            qlog("templates: template %d of %d is not a picture Grimoire can use (%s)", i + 1, nn, why);
            write_quiet(bp, "");
            bad++;
        }
        free(data);
        bgpicture_free(&src);
        bgpicture_free(&fit);
        canvas_free(&full);
        canvas_free(&small);
    }
    // Previews of files that are gone.
    DIR *d = opendir(cache);
    for (struct dirent *e; d && keep && (e = readdir(d));) {
        if (e->d_name[0] == '.') continue;
        bool kept = false;
        for (int i = 0; i < nn && !kept; i++) kept = keep[i][0] && !strncmp(e->d_name, keep[i], 16);
        if (kept) continue;
        char path[1300];
        snprintf(path, sizeof path, "%s/%s", cache, e->d_name);
        unlink(path);
    }
    if (d) closedir(d);
    free(keep);
    free_names(names, nn);
    qlog("templates: %d preview(s) made, %d file(s) that do not read", made, bad);
    return keep ? 0 : -1;
}

// ---- templates into notebooks ----

// The first `max` bytes of a file (all of it if it is shorter), or NULL.
static uint8_t *read_head(const char *path, size_t max, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    uint8_t *buf = malloc(max);
    *len = buf ? fread(buf, 1, max, f) : 0;
    fclose(f);
    return buf;
}

int template_install(const NotebookInfo *nb, const char *path, PaperKind *kind, const char **why) {
    *why = "the file does not read";
    // On the UI thread: what the picture is from its header (the first MB holds it, metadata and
    // all), and its key from its bytes, read a piece at a time; it is decoded in the background.
    struct stat st;
    if (stat(path, &st) != 0 || !S_ISREG(st.st_mode)) return -1;
    if ((uint64_t)st.st_size > TEMPLATE_MAX_BYTES) {
        *why = "a file over 64 MB";
        return -1;
    }
    size_t hl = 0, len = 0;
    uint8_t *head = read_head(path, 1u << 20, &hl);
    int w, h, pw, ph;
    int ok = head && bgimage_check(head, hl, &w, &h, why) == 0;
    free(head);
    if (!ok) return -1;
    store_notebook_page_size(nb, &pw, &ph);
    uint64_t key;
    *why = "the file does not read";
    if (bgimage_source_key_file(path, pw, ph, &key, &len) != 0) return -1;
    char dir[700], file[800];
    *why = "the notebook cannot be written";
    if (bgimage_dir(nb->id, dir, sizeof dir) != 0 || mkdir_p(dir) != 0) return -1;
    BgIndex ix;
    uint32_t id;
    bool added = false;
    if (bgindex_load(dir, &ix) != 0) return -1;
    int rc = bgindex_claim(&ix, dir, key, &id, &added);
    if (rc == 0 && added) rc = bgindex_save(dir, &ix);
    bgindex_free(&ix);
    if (rc != 0) return -1;
    *kind = paper_image(PAPER_ROLE_TEMPLATE, id);
    bgimage_file(dir, id, file, sizeof file);
    if (access(file, F_OK) != 0) {
        qlog("templates: %s: bg/%06x.png to be made (%dx%d picture for a %dx%d page)", nb->id, (unsigned)id, w, h, pw, ph);
        bg_queue(BG_TEMPLATE, nb->id, path, store_dpi(), false);
    }
    return 0;
}

// In the child: the picture template_install gave an id to, made for the notebook's page size.
static int make_template(const char *id, const char *src) {
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb || store_load_info(id, nb) != 0) {
        free(nb);
        qlog("templates: notebook %s does not open", id);
        return -1;
    }
    int pw, ph;
    store_notebook_page_size(nb, &pw, &ph);
    free(nb);
    size_t len;
    uint8_t *data = read_source(src, &len);
    if (!data) {
        qlog("templates: %s: the template file is gone", id);
        return -1;
    }
    char dir[700], file[800];
    BgIndex ix = {0};
    uint32_t pic = 0;
    uint64_t key = bgimage_source_key(data, len, pw, ph);
    bool known = bgimage_dir(id, dir, sizeof dir) == 0 && bgindex_load(dir, &ix) == 0 && bgindex_find(&ix, key, &pic);
    bgindex_free(&ix);
    if (!known) {
        free(data);
        qlog("templates: %s: no id was given to this template (it changed since it was picked?)", id);
        return -1;
    }
    bgimage_file(dir, pic, file, sizeof file);
    int rc = 0;
    if (access(file, F_OK) != 0) {
        BgPicture p = {0}, fit = {0};
        const char *why = "out of memory";
        rc = bgimage_decode(data, len, &p, &why) == 0 && bgimage_normalize(&p, pw, ph, &fit) == 0 &&
                     bgimage_write_png(file, &fit) == 0 ? 0 : -1;
        if (rc) qlog("templates: %s: bg/%06x.png could not be made (%s)", id, (unsigned)pic, why);
        else qlog("templates: %s: bg/%06x.png made (%dx%d, %s)", id, (unsigned)pic, pw, ph, fit.ch == 3 ? "colour" : "gray");
        bgpicture_free(&p);
        bgpicture_free(&fit);
    }
    free(data);
    return rc;
}

int bg_template(const char *id, const char *src, float dpi) {
    (void)dpi;
    if (!id || !*id) return make_previews(src);  // no notebook: the templates folder's previews
    return make_template(id, src);
}

// ---- imports ----

// Only one import runs at a time, in whichever process (a run's last jobs go on in a detached child
// after Grimoire closes): grimoire/import/.lock, held while it works. A start that finds it held
// leaves the import folders for the next start. -1, or the descriptor that holds the lock.
static int import_lock(bool wait) {
    char path[700];
    root_path("import/.lock", path, sizeof path);
    int fd = open(path, O_RDWR | O_CREAT, 0644);
    if (fd < 0) return -1;
    while (flock(fd, LOCK_EX | (wait ? 0 : LOCK_NB)) != 0) {
        if (errno == EINTR) continue;
        close(fd);
        return -1;
    }
    return fd;
}

// Removes a folder an import was building (notebooks/.import-<id>/: its files and its bg/).
static void remove_work(const char *work) {
    for (int pass = 0; pass < 2; pass++) {
        char dir[700];
        snprintf(dir, sizeof dir, pass == 0 ? "%s/bg" : "%s", work);
        DIR *d = opendir(dir);
        for (struct dirent *e; d && (e = readdir(d));) {
            if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
            char path[1000];
            snprintf(path, sizeof path, "%s/%s", dir, e->d_name);
            unlink(path);
        }
        if (d) closedir(d);
        rmdir(dir);
    }
}

// The notebook id written in an import folder's mark, or false (none, or "done").
static bool read_mark(const char *src, char id[STORE_ID_LEN], bool *done) {
    char path[1200];
    snprintf(path, sizeof path, "%s/" IMPORT_MARK, src);
    size_t len;
    char *t = read_file(path, &len);
    *done = t && !strncmp(t, "done", 4);
    bool ok = false;
    if (t && !*done) {
        t[strcspn(t, "\r\n")] = 0;
        char dir[700];
        ok = strlen(t) < STORE_ID_LEN && store_notebook_dir(t, dir, sizeof dir) == 0;
        if (ok) strlcpy_q(id, t, STORE_ID_LEN);
    }
    free(t);
    return ok;
}

static bool write_mark(const char *src, const char *text) {
    char path[1200];
    snprintf(path, sizeof path, "%s/" IMPORT_MARK, src);
    return write_quiet(path, text) == 0;
}

static char g_pending[IMPORT_QUEUE_MAX][STORE_ID_LEN];  // imports queued by this run, oldest first
static int g_npending;

// Whether notebooks/ holds what an import was building (notebooks/.import-<id>/); with `remove`,
// removes it all.
static bool import_leftovers(bool remove) {
    char notebooks[600];
    root_path("notebooks", notebooks, sizeof notebooks);
    bool any = false;
    DIR *d = opendir(notebooks);
    for (struct dirent *e; d && (e = readdir(d));) {
        if (strncmp(e->d_name, ".import-", 8)) continue;
        any = true;
        if (!remove) break;
        char work[1000];
        snprintf(work, sizeof work, "%s/%s", notebooks, e->d_name);
        remove_work(work);
        qlog("import: removed what a cut-off import left (%s)", e->d_name + 8);
    }
    if (d) closedir(d);
    return any;
}

int import_scan(float dpi) {
    char dir[600];
    root_path("import", dir, sizeof dir);
    char **names;
    int nn, folders = 0;
    if (list_names(dir, false, 1000, &names, &nn) != 0) nn = 0;
    for (int i = 0; i < nn; i++) folders += strcmp(names[i], "done") != 0;
    if (!folders && !import_leftovers(false)) {  // nothing to do (and no lock file made for it)
        free_names(names, nn);
        return 0;
    }
    int lock = import_lock(false);
    if (lock < 0) {
        qlog("import: an import from an earlier run is still going; the import folders wait for the next start");
        free_names(names, nn);
        return 0;
    }
    // Leftovers of an import that was cut off (Grimoire or the Kindle stopped mid-way): no import
    // runs now (the lock), so they are no one's.
    import_leftovers(true);
    int queued = 0, empty = 0;
    for (int i = 0; i < nn && g_npending < IMPORT_QUEUE_MAX; i++) {
        if (!strcmp(names[i], "done")) continue;
        char src[1200], id[STORE_ID_LEN];
        snprintf(src, sizeof src, "%s/%s", dir, names[i]);
        char **pics;
        int np;
        if (list_names(src, true, 1, &pics, &np) != 0) continue;
        free_names(pics, np);
        bool done;
        read_mark(src, id, &done);
        if (np == 0 || done) {
            empty += np == 0;
            continue;
        }
        store_make_id(id);
        char text[STORE_ID_LEN + 2];
        snprintf(text, sizeof text, "%s\n", id);
        if (!write_mark(src, text)) {
            qlog("import: an import folder cannot be written to; left as it is");
            continue;
        }
        bg_queue(BG_IMPORT_FOLDER, id, src, dpi, true);
        strlcpy_q(g_pending[g_npending++], id, STORE_ID_LEN);
        queued++;
    }
    free_names(names, nn);
    close(lock);
    if (queued || empty) qlog("import: %d folder(s) queued, %d without page pictures left as they are", queued, empty);
    return queued;
}

int import_finished(bool ok, int *pages) {
    NotebookInfo *nb = malloc(sizeof *nb);
    for (int i = 0; nb && i < g_npending; i++) {
        if (store_load_info(g_pending[i], nb) != 0) continue;
        *pages = nb->npages;
        memmove(g_pending[i], g_pending[i + 1], (size_t)(g_npending - i - 1) * sizeof g_pending[0]);
        g_npending--;
        free(nb);
        return 1;
    }
    free(nb);
    if (ok || !g_npending) return 0;
    memmove(g_pending[0], g_pending[1], (size_t)(g_npending - 1) * sizeof g_pending[0]);  // the oldest failed
    g_npending--;
    return -1;
}

// Moves a folder that became notebook id to import/done/<Name>-<time>; one that cannot move is
// marked done in place, so no later start imports it again.
static void finish_source(const char *src, const char *id) {
    char done[700], to[1400], stamp[32];
    root_path("import/done", done, sizeof done);
    const char *slash = strrchr(src, '/');
    const char *name = slash ? slash + 1 : src;
    time_t t = (time_t)wall_time();
    struct tm tm;
    localtime_r(&t, &tm);
    snprintf(stamp, sizeof stamp, "%04d%02d%02d-%02d%02d%02d", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour,
             tm.tm_min, tm.tm_sec);
    bool moved = false;
    int err = 0;
    if (mkdir_p(done) != 0) err = errno;
    else
        for (int k = 1; k < 100 && !moved; k++) {
            if (k == 1) snprintf(to, sizeof to, "%s/%s-%s", done, name, stamp);
            else snprintf(to, sizeof to, "%s/%s-%s-%d", done, name, stamp, k);
            if (access(to, F_OK) == 0) continue;
            moved = rename(src, to) == 0;
            if (!moved) {
                err = errno;
                break;
            }
        }
    if (moved) {
        char mark[1500];
        snprintf(mark, sizeof mark, "%s/" IMPORT_MARK, to);
        unlink(mark);
        return;
    }
    char text[STORE_ID_LEN + 8];
    snprintf(text, sizeof text, "done %s\n", id);
    write_mark(src, text);
    qlog("import: %s: its folder could not move to import/done (%s); marked as imported", id, strerror(err));
}

// The import itself, under the lock: the notebook is built in notebooks/.import-<id>/ (which the
// Library never lists) and renamed into place only once whole; anything going wrong removes it and
// leaves the folder as it was.
static int import_folder(const char *src) {
    if (!is_dir(src)) {
        qlog("import: the folder is gone (an earlier run imported it)");
        return 0;
    }
    char id[STORE_ID_LEN], final[600], work[700];
    bool done;
    if (!read_mark(src, id, &done)) {
        if (done) return 0;
        store_make_id(id);
    }
    if (store_notebook_dir(id, final, sizeof final) != 0) return -1;
    if (is_dir(final)) {  // made before, but its folder never moved
        finish_source(src, id);
        return 0;
    }
    char **names;
    int n;
    if (list_names(src, true, STORE_MAX_PAGES + 1, &names, &n) != 0) return -1;
    if (n == 0 || n > STORE_MAX_PAGES) {
        qlog("import: %s: %s page pictures; nothing imported", id, n ? "more than 2000" : "no");
        free_names(names, n);
        return -1;
    }
    snprintf(work, sizeof work, "%s/notebooks/.import-%s", store_root(), id);
    char bgdir[800];
    snprintf(bgdir, sizeof bgdir, "%s/bg", work);
    remove_work(work);
    int pw, ph;
    store_page_size(&pw, &ph);
    BgIndex ix = {0};
    uint32_t *pids = calloc((size_t)n, sizeof *pids);
    PaperKind *papers = calloc((size_t)n, sizeof *papers);
    NotebookInfo *nb = calloc(1, sizeof *nb);
    int rc = -1;
    if (!pids || !papers || !nb || mkdir_p(bgdir) != 0) goto out;
    qlog("import: %s: %d page(s), for %dx%d pages", id, n, pw, ph);
    for (int i = 0; i < n; i++) {
        char path[1300];
        snprintf(path, sizeof path, "%s/%s", src, names[i]);
        size_t len;
        uint8_t *data = read_source(path, &len);
        BgPicture p = {0}, fit = {0};
        const char *why = "the file does not read";
        bool ok = data && bgimage_decode(data, len, &p, &why) == 0;
        free(data);
        if (ok && !(ok = bgimage_normalize(&p, pw, ph, &fit) == 0)) why = "out of memory";
        bgpicture_free(&p);
        uint32_t pic = 0;
        bool added = false;
        if (ok && !(ok = bgindex_claim(&ix, bgdir, bgimage_key(&fit), &pic, &added) == 0)) why = "out of memory";
        if (ok && added) {
            char file[900];
            bgimage_file(bgdir, pic, file, sizeof file);
            if (!(ok = bgimage_write_png(file, &fit) == 0)) why = "a picture could not be written";
        }
        if (ok && i == 0) {  // the Library's thumbnail: page 1 as page_render_thumb makes it
            Canvas full = {0}, thumb = {0};
            char tp[900];
            snprintf(tp, sizeof tp, "%s/thumb.pgm", work);
            if (bgimage_luma(&fit, &full) == 0 && canvas_init(&thumb, pw / 3, ph / 3) == 0) {
                canvas_downsample(&thumb, &full);
                store_write_pgm(tp, &thumb);
            }
            canvas_free(&full);
            canvas_free(&thumb);
        }
        bgpicture_free(&fit);
        if (ok) {
            pids[i] = (uint32_t)i + 1;
            papers[i] = paper_image(PAPER_ROLE_DOCUMENT, pic);
            Page *pg = page_new(pw, ph, papers[i]);
            if (!(ok = pg && store_write_page_in(work, pids[i], pg) == 0)) why = "a page could not be written";
            page_free(pg);
        }
        if (!ok) {
            qlog("import: %s: page %d of %d: %s; nothing imported", id, i + 1, n, why);
            goto out;
        }
        if ((i + 1) % 10 == 0 && i + 1 < n) qlog("import: %s: %d of %d pages", id, i + 1, n);
    }
    // The notebook around them: its pages are document pages, and pages added to it are blank.
    strlcpy_q(nb->id, id, sizeof nb->id);
    const char *slash = strrchr(src, '/');
    copy_name(nb->name, slash ? slash + 1 : src, sizeof nb->name);  // (a long one cut between characters)
    nb->paper = PAPER_BLANK;
    nb->created = nb->modified = wall_time();
    nb->npages = n;
    for (int i = 0; i < n; i++) nb->page_ids[i] = pids[i];
    nb->next_page_id = (uint32_t)n + 1;
    nb->page_w = pw;
    nb->page_h = ph;
    if (bgindex_save(bgdir, &ix) != 0 || store_write_pages_list_in(bgdir, pids, papers, n) != 0 ||
        store_write_info_in(work, nb) != 0) {
        qlog("import: %s: its notebook files could not be written; nothing imported", id);
        goto out;
    }
    if (rename(work, final) != 0) {
        qlog("import: %s: could not move the notebook into place (%s); nothing imported", id, strerror(errno));
        goto out;
    }
    rc = 0;
    qlog("import: notebook %s made of %d page(s), %d picture(s)", id, n, ix.n);
    finish_source(src, id);
out:
    if (rc) remove_work(work);
    bgindex_free(&ix);
    free(pids);
    free(papers);
    free(nb);
    free_names(names, n);
    return rc;
}

int bg_import_folder(const char *path, float dpi) {
    (void)dpi;
    int lock = import_lock(true);
    if (lock < 0) {
        qlog("import: no lock (%s)", strerror(errno));
        return -1;
    }
    int rc = import_folder(path);
    close(lock);
    return rc;
}
