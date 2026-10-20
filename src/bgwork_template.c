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
#include <sys/resource.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#include "bgimage.h"
#include "bgjob.h"
#include "i18n.h"

#define TEMPLATE_MAX_BYTES (16u << 20) // a template file bigger than this is not taken
#define TEMPLATE_HASH_NOW (4u << 20)   // the most read on the UI thread for a template's key
#define MAX_TEMPLATES 500              // files of the templates folder looked at
#define IMPORT_QUEUE_MAX 8             // import folders queued at one start (the job queue holds 16)
#define IMPORT_MARK ".grimoire-import" // in an import folder: the notebook it becomes ("done <id>" once made)
#define IMPORT_TRIES 3                 // starts that try a folder whose pictures stay the same
#define JOB_PATH_MAX TEMPLATE_PATH_MAX // what a background job's path holds (bgjob.c): a longer one is cut
#define PATH_BUF 1600                  // a folder's path (< JOB_PATH_MAX) and a name in it (vfat: up to 765 bytes)

// What import_folder came to: done, failed in a way that may pass (memory, a full disk), or
// refused (a page is no picture Grimoire takes: the same pictures fail the same way).
enum { IMPORT_DONE = 0, IMPORT_RETRY = -1, IMPORT_REFUSED = -2 };

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
        char path[PATH_BUF];
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

// A whole picture file of at most max bytes, or NULL (missing, unreadable, or bigger: errno EFBIG).
static uint8_t *read_source(const char *path, size_t max, size_t *len) { return bgimage_read(path, max, len); }

// The first `max` bytes of a file (all of it if it is shorter), or NULL.
static uint8_t *read_head(const char *path, size_t max, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    uint8_t *buf = malloc(max);
    *len = buf ? fread(buf, 1, max, f) : 0;
    fclose(f);
    return buf;
}

// Whether the file at path, of at most max bytes, is a picture Grimoire takes (bgimage_check), from
// its header: in its first 64 KB, or else (metadata before it: an ICC profile, a JPEG's
// thumbnail) anywhere in the file, read whole for that. 0 with its size, or -1 and why.
static int check_file(const char *path, size_t max, int *w, int *h, const char **why) {
    *why = "the file does not read";
    struct stat st;
    if (stat(path, &st) != 0 || !S_ISREG(st.st_mode)) return -1;
    if ((uint64_t)st.st_size > (uint64_t)max) {
        *why = "too big a file";
        return -1;
    }
    size_t hl = 0, want = 64u << 10;
    uint8_t *head = read_head(path, want, &hl);
    bool more = head && hl == want;
    int rc = head ? bgimage_check(head, hl, (size_t)st.st_size, w, h, why) : -1;
    free(head);
    if (rc != 0 && more) {
        size_t len;
        uint8_t *all = read_source(path, max, &len);
        if (all) rc = bgimage_check(all, len, len, w, h, why);
        free(all);
    }
    return rc;
}

// A small file written whole (write, fsync, rename), its path never logged: it is in the owner's
// own folder, whose name is theirs.
static int write_quiet(const char *path, const char *text) {
    char tmp[PATH_BUF + 64];
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

// Makes what a rename did to the entries of dir durable, as write_file_atomic does after its own
// (best effort).
static void sync_dir(const char *dir) {
    int fd = open(dir, O_RDONLY);
    if (fd < 0) return;
    fsync(fd);
    close(fd);
}

// ---- memory, in the children ----
//
// A ceiling on a child's address space: what it holds now, plus what one decode may take
// (BG_DECODE_BUDGET) and room for the page picture it makes. Past it an allocation fails, and the
// job with it (an import keeps its folder), before the Kindle runs out of memory and its kernel
// picks a process to kill. The decode budget keeps stb_image within it already: this catches the
// rest. Linux only, and never under the sanitizers (their shadow memory is address space too).
#if defined(__has_feature)
#if __has_feature(address_sanitizer) || __has_feature(memory_sanitizer)
#define BG_SANITIZED 1
#endif
#endif
#if defined(__SANITIZE_ADDRESS__)
#define BG_SANITIZED 1
#endif

typedef struct {
    bool set;
    struct rlimit was;
} MemCap;

static void cap_memory(MemCap *m) {
    m->set = false;
#if defined(__linux__) && !defined(BG_SANITIZED)
    unsigned long pages = 0;
    FILE *f = fopen("/proc/self/statm", "r");
    if (!f) return;
    int got = fscanf(f, "%lu", &pages);
    fclose(f);
    long psz = sysconf(_SC_PAGESIZE);
    if (got != 1 || psz <= 0 || getrlimit(RLIMIT_AS, &m->was) != 0) return;
    rlim_t cap = (rlim_t)pages * (rlim_t)psz + (rlim_t)BG_DECODE_BUDGET + ((rlim_t)128 << 20);
    if (m->was.rlim_cur != RLIM_INFINITY && m->was.rlim_cur <= cap) return;  // as low already
    if (m->was.rlim_max != RLIM_INFINITY && cap > m->was.rlim_max) return;
    struct rlimit r = {cap, m->was.rlim_max};
    m->set = setrlimit(RLIMIT_AS, &r) == 0;
#endif
}

static void uncap_memory(const MemCap *m) {
    if (m->set) setrlimit(RLIMIT_AS, &m->was);
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

// The preview of a template file: .cache/<hash of its name, size and time>.pgm (with .key, the
// hash of its bytes, and .bad for one that does not read), so a file replaced by another gets a
// preview of its own.
static bool preview_path(const char *dir, const char *name, const char *ext, char *buf, size_t cap) {
    char path[PATH_BUF];
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

// The same for a template given by its path.
static bool preview_of(const char *path, const char *ext, char *buf, size_t cap) {
    char dir[TEMPLATE_PATH_MAX];
    if (strlcpy_q(dir, path, sizeof dir) != strlen(path)) return false;
    char *slash = strrchr(dir, '/');
    if (!slash) return false;
    *slash = 0;
    return preview_path(dir, slash + 1, ext, buf, cap);
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
    int m = 0, toolong = 0;
    for (int i = 0; i < nn; i++) {
        if (strlen(dir) + 1 + strlen(names[i]) >= sizeof t[m].path) {  // (it would be cut on its way to a job)
            toolong++;
            continue;
        }
        Template *tp = &t[m++];
        snprintf(tp->path, sizeof tp->path, "%s/%s", dir, names[i]);
        char base[STORE_NAME_LEN * 2];
        strlcpy_q(base, names[i], sizeof base);
        char *dot = strrchr(base, '.');
        if (dot) *dot = 0;
        copy_name(tp->name, base, sizeof tp->name);
        char pv[PATH_BUF], kp[PATH_BUF];
        if (!preview_path(dir, names[i], "pgm", pv, sizeof pv) || !preview_path(dir, names[i], "key", kp, sizeof kp))
            continue;
        if (access(pv, F_OK) == 0) {
            tp->ready = true;
            if (access(kp, F_OK) != 0) missing = true;
            continue;
        }
        preview_path(dir, names[i], "bad", pv, sizeof pv);
        if (access(pv, F_OK) == 0) tp->bad = true;
        else missing = true;
    }
    free_names(names, nn);
    if (toolong) qlog("templates: %d template(s) with too long a name left out", toolong);
    if (missing) bg_queue(BG_TEMPLATE, "", dir, dpi, false);
    *out = t;
    *n = m;
    return 0;
}

const Canvas *template_thumb(Template *t) {
    if (t->thumb.px) return &t->thumb;
    if (!t->ready) return NULL;
    char pv[PATH_BUF];
    if (!preview_of(t->path, "pgm", pv, sizeof pv) || store_read_pgm(pv, &t->thumb) != 0) {
        t->ready = false;  // gone since the scan (the file changed): a scan makes it again
        return NULL;
    }
    return &t->thumb;
}

void template_list_free(Template *t, int n) {
    for (int i = 0; i < n && t; i++) canvas_free(&t[i].thumb);
    free(t);
}

// In the child: the previews the templates folder lacks, made at the page size of this Kindle, with
// the hash of each file's bytes beside its preview (a template's key comes from it, and so the UI
// never reads a big file to know it); previews of files no longer there are removed.
static int make_previews(const char *dir) {
    char cache[700];
    snprintf(cache, sizeof cache, "%s/.cache", dir);
    if (mkdir_p(cache) != 0) return -1;
    char **names;
    int nn;
    if (list_names(dir, true, MAX_TEMPLATES, &names, &nn) != 0) return -1;
    int pw, ph, made = 0, bad = 0, later = 0;
    store_page_size(&pw, &ph);
    char (*keep)[32] = calloc((size_t)(nn ? nn : 1), sizeof *keep);
    for (int i = 0; i < nn && keep; i++) {
        char pv[PATH_BUF], kp[PATH_BUF], bp[PATH_BUF], path[PATH_BUF];
        if (!preview_path(dir, names[i], "pgm", pv, sizeof pv)) continue;
        preview_path(dir, names[i], "key", kp, sizeof kp);
        preview_path(dir, names[i], "bad", bp, sizeof bp);
        const char *slash = strrchr(pv, '/');
        strlcpy_q(keep[i], slash ? slash + 1 : pv, sizeof keep[i]);
        keep[i][16] = 0;  // the hash, without the extension
        if ((access(pv, F_OK) == 0 && access(kp, F_OK) == 0) || access(bp, F_OK) == 0) continue;
        snprintf(path, sizeof path, "%s/%s", dir, names[i]);
        size_t len = 0;
        uint8_t *data = read_source(path, TEMPLATE_MAX_BYTES, &len);
        const char *why = data ? "out of memory" : errno == EFBIG ? "a file over 16 MB" : "the file does not read";
        BgPicture src = {0}, fit = {0};
        Canvas full = {0}, small = {0};
        int dec = data ? bgimage_decode(data, len, &src, &why) : -1;
        bool ok = dec == 0 && bgimage_normalize(&src, pw, ph, &fit) == 0 && bgimage_luma(&fit, &full) == 0 &&
                  canvas_init(&small, pw / THUMB_DIV, ph / THUMB_DIV) == 0;
        if (ok) {
            canvas_downsample_keep_lines(&small, &full, 0.6f);
            char key[64];
            snprintf(key, sizeof key, "%016llx %d %d\n", (unsigned long long)bgimage_content_hash(data, len), src.w, src.h);
            ok = write_quiet(kp, key) == 0 && store_write_pgm(pv, &small) == 0;  // (the key first: a preview has one)
            made += ok;
        } else if (dec == -1) {
            qlog("templates: template %d of %d is not a picture Grimoire can use (%s)", i + 1, nn, why);
            write_quiet(bp, "");
            bad++;
        } else {
            later++;  // out of memory: the next scan tries again
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
        char path[PATH_BUF];
        snprintf(path, sizeof path, "%s/%s", cache, e->d_name);
        unlink(path);
    }
    if (d) closedir(d);
    free(keep);
    free_names(names, nn);
    qlog("templates: %d preview(s) made, %d file(s) that do not read, %d left for lack of memory", made, bad, later);
    return keep ? 0 : -1;
}

// ---- templates into notebooks ----

// What the previews job found of a template file as it is now (its name, size and time): the hash
// of its bytes and its size in pixels. 0, or -1 when it has not seen it.
static int cached_key(const char *path, uint64_t *hash, int *w, int *h) {
    char kp[PATH_BUF];
    if (!preview_of(path, "key", kp, sizeof kp)) return -1;
    size_t len;
    char *t = read_file(kp, &len);
    unsigned long long v = 0;
    bool ok = t && sscanf(t, "%16llx %d %d", &v, w, h) == 3;
    free(t);
    if (ok) *hash = (uint64_t)v;
    return ok ? 0 : -1;
}

int template_install(const NotebookInfo *nb, const char *path, PaperKind *kind, const char **why) {
    *why = "the file does not read";
    // On the UI thread: its key, from the hash of its bytes and the page size; the picture is made
    // in the background. The previews job, which reads every template whole anyway, keeps the hash
    // and whether the file is a picture Grimoire takes; only a small file it has not seen yet is
    // read here (whole, once).
    struct stat st;
    if (strlen(path) >= JOB_PATH_MAX) {
        *why = "too long a path for a background job";
        return -1;
    }
    if (stat(path, &st) != 0 || !S_ISREG(st.st_mode)) return -1;
    if ((uint64_t)st.st_size > TEMPLATE_MAX_BYTES) {
        *why = "a file over 16 MB";
        return -1;
    }
    int w = 0, h = 0, pw, ph;
    uint64_t content;
    size_t len = (size_t)st.st_size;
    if (cached_key(path, &content, &w, &h) != 0) {
        if ((uint64_t)st.st_size > TEMPLATE_HASH_NOW) {
            *why = "its preview is not made yet (a file over 4 MB is not read here)";
            return -1;
        }
        uint8_t *data = read_source(path, TEMPLATE_HASH_NOW, &len);
        int ok = data && bgimage_check(data, len, len, &w, &h, why) == 0;
        if (ok) content = bgimage_content_hash(data, len);
        free(data);
        if (!ok) return -1;
    }
    store_notebook_page_size(nb, &pw, &ph);
    uint64_t key = bgimage_template_key(content, len, pw, ph);
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
    uint8_t *data = read_source(src, TEMPLATE_MAX_BYTES, &len);
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
    MemCap cap;
    cap_memory(&cap);
    int rc = !id || !*id ? make_previews(src) : make_template(id, src);  // no notebook: the folder's previews
    uncap_memory(&cap);
    return rc;
}

// ---- imports ----

// Only one import runs at a time, in whichever process (a run's last jobs go on in a detached child
// after Grimoire closes): grimoire/import/.lock, held while it works. A start that finds it held
// leaves the import folders for the next start. -1 (errno says why: EWOULDBLOCK, it is held), or
// the descriptor that holds the lock.
static int import_lock(bool wait) {
    char path[700];
    root_path("import/.lock", path, sizeof path);
    int fd = open(path, O_RDWR | O_CREAT, 0644);
    if (fd < 0) return -1;
    while (flock(fd, LOCK_EX | (wait ? 0 : LOCK_NB)) != 0) {
        if (errno == EINTR) continue;
        int err = errno;
        close(fd);
        errno = err;
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
    char path[PATH_BUF];
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
    char path[PATH_BUF];
    snprintf(path, sizeof path, "%s/" IMPORT_MARK, src);
    return write_quiet(path, text) == 0;
}

// An import that did not happen leaves the folder as the owner made it.
static void drop_mark(const char *src) {
    char path[PATH_BUF];
    snprintf(path, sizeof path, "%s/" IMPORT_MARK, src);
    unlink(path);
}

// ---- import attempts: import/.attempts ----
//
// A line "<name> <pictures> <tries>" for each folder still in import/ that an import tried: a hash
// of its name and one of its pictures (their names, sizes and times; 16 hex digits each), and how
// many starts tried it with those pictures (TRIES_REFUSED once a page was no picture Grimoire
// takes). A folder refused, or tried IMPORT_TRIES times (a failure that might have passed, or an
// import cut off each time), waits until its pictures change: it costs one try, not one at every
// start, and its failure shows once.
#define ATTEMPTS_MAX 64
#define TRIES_REFUSED 99
typedef struct {
    uint64_t name, pictures;
    int tries;
} Attempt;
enum { ATTEMPT_TRY, ATTEMPT_REFUSED, ATTEMPT_DONE };

#define FNV0 14695981039346656037ull
static uint64_t fnv(uint64_t h, const void *d, size_t n) {
    const uint8_t *p = d;
    for (size_t i = 0; i < n; i++) h = (h ^ p[i]) * 1099511628211ull;
    return h;
}

static uint64_t folder_name_hash(const char *src) {
    const char *slash = strrchr(src, '/');
    const char *name = slash ? slash + 1 : src;
    return fnv(FNV0, name, strlen(name));
}

// What a folder's pictures (names, in order) are: their names, sizes and times.
static uint64_t pictures_hash(const char *src, char *const *names, int n) {
    uint64_t h = fnv(FNV0, &n, sizeof n);
    for (int i = 0; i < n; i++) {
        char path[PATH_BUF];
        snprintf(path, sizeof path, "%s/%s", src, names[i]);
        struct stat st;
        int64_t v[2] = {-1, -1};
        if (stat(path, &st) == 0) {
            v[0] = (int64_t)st.st_size;
            v[1] = (int64_t)st.st_mtime;
        }
        h = fnv(fnv(h, names[i], strlen(names[i]) + 1), v, sizeof v);
    }
    return h;
}

static uint64_t folder_pictures_hash(const char *src) {
    char **names;
    int n;
    if (list_names(src, true, STORE_MAX_PAGES + 1, &names, &n) != 0) return 0;
    uint64_t h = pictures_hash(src, names, n);
    free_names(names, n);
    return h;
}

static int attempts_load(Attempt *a) {
    char path[700];
    root_path("import/.attempts", path, sizeof path);
    size_t len;
    char *t = read_file(path, &len);
    int n = 0;
    char *save = NULL;
    for (char *line = t ? strtok_r(t, "\n", &save) : NULL; line && n < ATTEMPTS_MAX; line = strtok_r(NULL, "\n", &save)) {
        unsigned long long nh, ph;
        int tries;
        if (sscanf(line, "%16llx %16llx %d", &nh, &ph, &tries) == 3 && tries > 0) a[n++] = (Attempt){nh, ph, tries};
    }
    free(t);
    return n;
}

static void attempts_save(const Attempt *a, int n) {
    char path[700], text[ATTEMPTS_MAX * 48 + 96];
    root_path("import/.attempts", path, sizeof path);
    if (n == 0) {
        unlink(path);
        return;
    }
    size_t len = (size_t)snprintf(text, sizeof text, "# Grimoire: import folders tried (docs/FORMAT.md)\n");
    for (int i = 0; i < n; i++)
        len += (size_t)snprintf(text + len, sizeof text - len, "%016llx %016llx %d\n", (unsigned long long)a[i].name,
                                (unsigned long long)a[i].pictures, a[i].tries);
    if (write_file_atomic(path, text, len) != 0) qlog("import: could not write import/.attempts");
}

// In the child, under the lock: the folder at src (with these pictures) was tried once more, or was
// refused, or was imported (its record goes).
static void attempts_note(const char *src, uint64_t pictures, int what) {
    Attempt a[ATTEMPTS_MAX];
    int n = attempts_load(a), at = -1;
    uint64_t nh = folder_name_hash(src);
    for (int i = 0; i < n; i++)
        if (a[i].name == nh) at = i;
    if (what == ATTEMPT_DONE) {
        if (at < 0) return;
        a[at] = a[--n];
    } else if (at >= 0) {
        a[at].tries = what == ATTEMPT_REFUSED            ? TRIES_REFUSED
                      : a[at].pictures != pictures       ? 1
                      : a[at].tries < TRIES_REFUSED - 1 ? a[at].tries + 1
                                                         : a[at].tries;
        a[at].pictures = pictures;
    } else {
        if (n == ATTEMPTS_MAX) memmove(a, a + 1, (size_t)--n * sizeof *a);  // the oldest record goes
        a[n++] = (Attempt){nh, pictures, what == ATTEMPT_REFUSED ? TRIES_REFUSED : 1};
    }
    attempts_save(a, n);
}

static char g_pending[IMPORT_QUEUE_MAX][STORE_ID_LEN];  // imports queued by this run, oldest first
static int g_npending;

// What an import was building when it was cut off (notebooks/.import-<id>/). One holding its
// notebook.ini is whole (that file is written last, each one before it made durable): it goes into
// place, and its folder moves when its id comes up (the folder's mark names it). Any other is
// removed. Whether there were any; with `act`, deals with them all.
static bool import_leftovers(bool act) {
    char notebooks[600];
    root_path("notebooks", notebooks, sizeof notebooks);
    bool any = false, placed = false;
    DIR *d = opendir(notebooks);
    for (struct dirent *e; d && (e = readdir(d));) {
        if (strncmp(e->d_name, ".import-", 8)) continue;
        any = true;
        if (!act) break;
        const char *id = e->d_name + 8;
        char work[1000], ini[1100], final[700];
        snprintf(work, sizeof work, "%s/%s", notebooks, e->d_name);
        snprintf(ini, sizeof ini, "%s/notebook.ini", work);
        if (access(ini, F_OK) == 0 && store_notebook_dir(id, final, sizeof final) == 0 && access(final, F_OK) != 0 &&
            rename(work, final) == 0) {
            placed = true;
            qlog("import: put in place the notebook a cut-off import had finished (%s)", id);
            continue;
        }
        remove_work(work);
        qlog("import: removed what a cut-off import left (%s)", id);
    }
    if (d) closedir(d);
    if (placed) sync_dir(notebooks);
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
        if (errno == EWOULDBLOCK)
            qlog("import: an import from an earlier run is still going; the import folders wait for the next start");
        else qlog("import: the import lock cannot be taken (%s); the import folders wait for the next start", strerror(errno));
        free_names(names, nn);
        return 0;
    }
    // Leftovers of an import that was cut off (Grimoire or the Kindle stopped mid-way): no import
    // runs now (the lock), so they are no one's.
    import_leftovers(true);
    Attempt att[ATTEMPTS_MAX];
    bool seen[ATTEMPTS_MAX] = {false};
    int natt = attempts_load(att);
    int queued = 0, empty = 0, waiting = 0, toolong = 0;
    for (int i = 0; i < nn; i++) {
        if (!strcmp(names[i], "done")) continue;
        char src[PATH_BUF], id[STORE_ID_LEN];
        snprintf(src, sizeof src, "%s/%s", dir, names[i]);
        uint64_t nh = folder_name_hash(src);
        int at = -1;
        for (int k = 0; k < natt; k++)
            if (att[k].name == nh) {
                at = k;
                seen[k] = true;
            }
        if (g_npending >= IMPORT_QUEUE_MAX) continue;  // (the next start takes it)
        if (strlen(src) >= JOB_PATH_MAX) {  // it would be cut on its way to the job
            toolong++;
            continue;
        }
        char **pics;
        int np;
        if (list_names(src, true, 1, &pics, &np) != 0) continue;
        free_names(pics, np);
        bool done, have = read_mark(src, id, &done);
        if (np == 0 || done) {
            empty += np == 0;
            continue;
        }
        if (at >= 0 && att[at].tries >= IMPORT_TRIES && att[at].pictures == folder_pictures_hash(src)) {
            waiting++;
            continue;
        }
        // A mark left by a run that was cut off names the notebook the folder was becoming: that id
        // again, so a notebook it made (its folder not moved yet) is finished, never made twice.
        if (!have) {
            store_make_id(id);
            char text[STORE_ID_LEN + 2];
            snprintf(text, sizeof text, "%s\n", id);
            if (!write_mark(src, text)) {
                qlog("import: an import folder cannot be written to; left as it is");
                continue;
            }
        }
        bg_queue(BG_IMPORT_FOLDER, id, src, dpi, true);
        strlcpy_q(g_pending[g_npending++], id, STORE_ID_LEN);
        queued++;
    }
    int kept = 0;  // the records of folders no longer here (imported, or taken away) go
    for (int k = 0; k < natt; k++)
        if (seen[k]) att[kept++] = att[k];
    if (kept != natt) attempts_save(att, kept);
    free_names(names, nn);
    close(lock);
    if (queued || empty || waiting || toolong)
        qlog("import: %d folder(s) queued; left as they are: %d without page pictures, %d that failed and have not "
             "changed since, %d with too long a name",
             queued, empty, waiting, toolong);
    return queued;
}

int import_finished(bool ok, int *pages) {
    // The imports run in the order they were queued: the oldest pending one has just finished,
    // unless the job queue dropped some (then any later one that made its notebook did).
    NotebookInfo *nb = malloc(sizeof *nb);
    for (int i = 0; nb && i < g_npending; i++) {
        if ((i > 0 && !ok) || store_load_info(g_pending[i], nb) != 0) continue;
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
    char import[600], done[700], to[PATH_BUF + 64], stamp[32];
    root_path("import", import, sizeof import);
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
        sync_dir(done);
        sync_dir(import);
        drop_mark(to);
        return;
    }
    char text[STORE_ID_LEN + 8];
    snprintf(text, sizeof text, "done %s\n", id);
    write_mark(src, text);
    qlog("import: %s: its folder could not move to import/done (%s); marked as imported", id, strerror(err));
}

// Every page's header (a millisecond or so a page), before any picture is made: a folder with a
// page that is no picture Grimoire takes fails at once, whatever its place. IMPORT_REFUSED, or 0.
static int check_pages(const char *id, const char *src, char *const *names, int n) {
    if (n == 0 || n > STORE_MAX_PAGES) {
        qlog("import: %s: %s page pictures; nothing imported", id, n ? "more than 2000" : "no");
        return IMPORT_REFUSED;
    }
    for (int i = 0; i < n; i++) {
        char path[PATH_BUF];
        int w, h;
        const char *why;
        snprintf(path, sizeof path, "%s/%s", src, names[i]);
        if (check_file(path, BG_SOURCE_MAX_BYTES, &w, &h, &why) != 0) {
            qlog("import: %s: page %d of %d: %s; nothing imported", id, i + 1, n, why);
            return IMPORT_REFUSED;
        }
    }
    return 0;
}

// The notebook itself, built in notebooks/.import-<id>/ (which the Library never lists) and renamed
// into place only once whole; anything going wrong removes it. IMPORT_DONE (its folder moved to
// import/done), IMPORT_RETRY or IMPORT_REFUSED.
static int build_notebook(const char *id, const char *src, const char *final, char *const *names, int n) {
    char work[700], bgdir[800];
    snprintf(work, sizeof work, "%s/notebooks/.import-%s", store_root(), id);
    snprintf(bgdir, sizeof bgdir, "%s/bg", work);
    remove_work(work);
    int pw, ph;
    store_page_size(&pw, &ph);
    BgIndex ix = {0};
    uint32_t *pids = calloc((size_t)n, sizeof *pids);
    PaperKind *papers = calloc((size_t)n, sizeof *papers);
    NotebookInfo *nb = calloc(1, sizeof *nb);
    int rc = IMPORT_RETRY;
    if (!pids || !papers || !nb) {
        qlog("import: %s: out of memory; nothing imported", id);
        goto out;
    }
    if (mkdir_p(bgdir) != 0) {
        qlog("import: %s: notebooks/ cannot be written (%s); nothing imported", id, strerror(errno));
        goto out;
    }
    qlog("import: %s: %d page(s), for %dx%d pages", id, n, pw, ph);
    for (int i = 0; i < n; i++) {
        char path[PATH_BUF];
        snprintf(path, sizeof path, "%s/%s", src, names[i]);
        size_t len;
        uint8_t *data = read_source(path, BG_SOURCE_MAX_BYTES, &len);
        BgPicture p = {0}, fit = {0};
        const char *why = "the file does not read";
        int fail = IMPORT_RETRY;
        if (data) {
            int dec = bgimage_decode(data, len, &p, &why);
            fail = dec == 0 ? 0 : dec == -2 ? IMPORT_RETRY : IMPORT_REFUSED;
        }
        free(data);
        if (!fail && bgimage_normalize(&p, pw, ph, &fit) != 0) {
            why = "out of memory";
            fail = IMPORT_RETRY;
        }
        bgpicture_free(&p);
        uint32_t pic = 0;
        bool added = false;
        if (!fail && bgindex_claim(&ix, bgdir, bgimage_key(&fit), &pic, &added) != 0) {
            why = "out of memory";
            fail = IMPORT_RETRY;
        }
        if (!fail && added) {
            char file[900];
            bgimage_file(bgdir, pic, file, sizeof file);
            if (bgimage_write_png(file, &fit) != 0) {
                why = "a picture could not be written";
                fail = IMPORT_RETRY;
            }
        }
        if (!fail && i == 0) {  // the Library's thumbnail: page 1 as page_render_thumb makes it
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
        if (!fail) {
            pids[i] = (uint32_t)i + 1;
            papers[i] = paper_image(PAPER_ROLE_DOCUMENT, pic);
            Page *pg = page_new(pw, ph, papers[i]);
            if (!pg || store_write_page_in(work, pids[i], pg) != 0) {
                why = "a page could not be written";
                fail = IMPORT_RETRY;
            }
            page_free(pg);
        }
        if (fail) {
            qlog("import: %s: page %d of %d: %s; nothing imported", id, i + 1, n, why);
            rc = fail;
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
    // notebook.ini last: a folder holding it is whole (import_leftovers).
    if (bgindex_save(bgdir, &ix) != 0 || store_write_pages_list_in(bgdir, pids, papers, n) != 0 ||
        store_write_info_in(work, nb) != 0) {
        qlog("import: %s: its notebook files could not be written; nothing imported", id);
        goto out;
    }
    if (rename(work, final) != 0) {
        qlog("import: %s: could not move the notebook into place (%s); nothing imported", id, strerror(errno));
        goto out;
    }
    char notebooks[600];
    root_path("notebooks", notebooks, sizeof notebooks);
    sync_dir(notebooks);  // the notebook in place for good before its folder leaves import/
    rc = IMPORT_DONE;
    qlog("import: notebook %s made of %d page(s), %d picture(s)", id, n, ix.n);
    finish_source(src, id);
out:
    if (rc != IMPORT_DONE) remove_work(work);
    bgindex_free(&ix);
    free(pids);
    free(papers);
    free(nb);
    return rc;
}

// The import itself, under the lock. A failed one leaves no notebook, and the folder as it was.
static int import_folder(const char *src) {
    if (!is_dir(src)) {
        qlog("import: the folder is gone (an earlier run imported it)");
        return IMPORT_DONE;
    }
    char id[STORE_ID_LEN], final[600];
    bool done;
    if (!read_mark(src, id, &done)) {
        if (done) return IMPORT_DONE;
        store_make_id(id);
    }
    if (store_notebook_dir(id, final, sizeof final) != 0) return IMPORT_RETRY;
    if (is_dir(final)) {  // made by a run cut off before its folder moved
        qlog("import: %s: made by an earlier run; its folder moves now", id);
        finish_source(src, id);
        attempts_note(src, 0, ATTEMPT_DONE);
        return IMPORT_DONE;
    }
    char **names;
    int n;
    if (list_names(src, true, STORE_MAX_PAGES + 1, &names, &n) != 0) {
        qlog("import: %s: its folder does not read (%s)", id, strerror(errno));
        return IMPORT_RETRY;
    }
    uint64_t pictures = pictures_hash(src, names, n);
    attempts_note(src, pictures, ATTEMPT_TRY);  // (counted first: a try cut off counts too)
    int rc = check_pages(id, src, names, n);
    if (rc == 0) rc = build_notebook(id, src, final, names, n);
    free_names(names, n);
    if (rc == IMPORT_DONE) {
        attempts_note(src, pictures, ATTEMPT_DONE);
    } else {
        if (rc == IMPORT_REFUSED) attempts_note(src, pictures, ATTEMPT_REFUSED);
        drop_mark(src);
    }
    return rc;
}

int bg_import_folder(const char *path, float dpi) {
    (void)dpi;
    int lock = import_lock(true);
    if (lock < 0) {
        qlog("import: no lock (%s)", strerror(errno));
        return -1;
    }
    MemCap cap;
    cap_memory(&cap);
    int rc = import_folder(path);
    uncap_memory(&cap);
    close(lock);
    return rc == IMPORT_DONE ? 0 : -1;
}
