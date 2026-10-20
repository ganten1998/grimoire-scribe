// annotate.c: a page KOReader sent to annotate (annotate.h, docs/FORMAT.md, Annotated pages).
//
// grimoire.log goes into public bug reports: the book's title, its file and its key are never
// logged here, only notebook ids, page numbers and counts.
#include "annotate.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "bgimage.h"
#include "bgjob.h"
#include "i18n.h"

#define REQUEST_MAX 4096                // a request is a few lines
#define PICTURE_MAX ((size_t)16 << 20)  // the most of a page picture read: a template's, which the
                                        // fitting job reads at most (no page's is near it)
#define WORK_PREFIX ".annotate-"        // notebooks/.annotate-<id>/: a notebook being made

// ---- the request ----

// A whole number of at least lo, written as digits only; -1 for anything else.
static long long number(const char *v, long long lo, long long hi) {
    if (!*v) return -1;
    long long x = 0;
    for (const char *c = v; *c; c++) {
        if (*c < '0' || *c > '9') return -1;
        x = x * 10 + (*c - '0');
        if (x > hi) return -1;
    }
    return x < lo ? -1 : x;
}

// A picture's name in the requests' folder: a plain file name ending in .png (letters, digits, '-',
// '_' and '.', not starting with a dot), so it cannot name anything outside the folder.
static bool picture_name_ok(const char *s) {
    size_t n = strlen(s);
    if (n < 5 || n >= sizeof ((AnnotateRequest *)0)->picture || s[0] == '.' || strcmp(s + n - 4, ".png")) return false;
    for (size_t i = 0; i < n; i++)
        if (!((s[i] >= '0' && s[i] <= '9') || (s[i] >= 'a' && s[i] <= 'z') || (s[i] >= 'A' && s[i] <= 'Z') ||
              s[i] == '-' || s[i] == '_' || s[i] == '.'))
            return false;
    return true;
}

static bool word_ok(const char *s, size_t cap) {
    size_t n = strlen(s);
    if (n == 0 || n >= cap) return false;
    for (size_t i = 0; i < n; i++)
        if (!(s[i] >= 'a' && s[i] <= 'z')) return false;
    return true;
}

int annotate_parse(const char *text, AnnotateRequest *r, const char **why) {
    memset(r, 0, sizeof *r);
    r->version = 1;  // (a request without the line: a hand-made one)
    *why = "no request";
    if (!text) return -1;
    long long page = -1, pages = -1;
    for (const char *p = text; *p;) {
        const char *eol = strchr(p, '\n');
        size_t n = eol ? (size_t)(eol - p) : strlen(p);
        char line[ANNOTATE_TITLE_LEN + 16];
        size_t k = n < sizeof line - 1 ? n : sizeof line - 1;  // (a longer line: its start)
        memcpy(line, p, k);
        line[k] = 0;
        if (k && line[k - 1] == '\r') line[--k] = 0;
        p += n + (eol ? 1 : 0);
        char *eq = strchr(line, '=');
        if (line[0] == '#' || !eq) continue;
        *eq = 0;
        const char *key = line, *v = eq + 1;
        if (!strcmp(key, "version")) {
            r->version = (int)number(v, 0, 1000000);
        } else if (!strcmp(key, "book")) {
            if (!store_source_ok(v)) {
                *why = "its book key is not one";
                return -1;
            }
            strlcpy_q(r->book, v, sizeof r->book);
        } else if (!strcmp(key, "title")) {
            strlcpy_q(r->title, v, sizeof r->title);  // (cut between characters later: annotate_name)
        } else if (!strcmp(key, "page")) {
            page = number(v, 1, 10000000);
        } else if (!strcmp(key, "pages")) {
            pages = number(v, 1, 10000000);
        } else if (!strcmp(key, "kind")) {
            if (word_ok(v, sizeof r->kind)) strlcpy_q(r->kind, v, sizeof r->kind);
        } else if (!strcmp(key, "picture")) {
            if (!picture_name_ok(v)) {
                *why = "its picture's name is not a plain .png file name";
                return -1;
            }
            strlcpy_q(r->picture, v, sizeof r->picture);
        } else if (!strcmp(key, "time")) {
            long long t = number(v, 0, 1LL << 40);
            r->time = t > 0 ? t : 0;
        }  // (a key this version does not know: a newer plugin's, left to it)
    }
    if (r->version != 1) {
        *why = "a version of request this Grimoire does not know (from a newer plugin)";
        return -1;
    }
    if (!r->book[0] || !r->picture[0]) {
        *why = r->book[0] ? "no picture named" : "no book key";
        return -1;
    }
    if (page < 1 || pages < page) {
        *why = page < 1 ? "no page number" : "no page count, or one under the page number";
        return -1;
    }
    r->page = (int)page;
    r->pages = (int)pages;
    *why = "";
    return 0;
}

bool annotate_too_old(const AnnotateRequest *r, int64_t now, int64_t file_time) {
    int64_t t = r->time > 0 ? r->time : file_time;
    return t <= 0 || now - t > ANNOTATE_STALE_S || t - now > ANNOTATE_STALE_S;
}

// ---- source.ini ----

static bool in_notebook(const NotebookInfo *nb, uint32_t pid, int *index) {
    for (int i = 0; nb && i < nb->npages; i++)
        if (nb->page_ids[i] == pid) {
            if (index) *index = i;
            return true;
        }
    return false;
}

int annotate_map_add(AnnotateMap *m, int page, int pages, uint32_t pid) {
    for (int i = 0; i < m->n; i++)
        if (m->e[i].page == page && m->e[i].pages == pages) {
            m->e[i].pid = pid;
            return 0;
        }
    if (m->n == m->cap) {
        int nc = m->cap ? m->cap * 2 : 16;
        AnnotatePage *ne = realloc(m->e, (size_t)nc * sizeof *ne);
        if (!ne) return -1;
        m->e = ne;
        m->cap = nc;
    }
    m->e[m->n++] = (AnnotatePage){page, pages, pid};
    return 0;
}

// Lines of a form this version does not read, kept to be written back (whole lines, while they fit).
static void keep_line(AnnotateMap *m, const char *line) {
    size_t have = strnlen(m->extra, sizeof m->extra), need = strlen(line) + 1;
    if (have + need < sizeof m->extra) snprintf(m->extra + have, sizeof m->extra - have, "%s\n", line);
}

int annotate_map_parse(const char *text, AnnotateMap *m) {
    memset(m, 0, sizeof *m);
    for (const char *p = text; p && *p;) {
        const char *eol = strchr(p, '\n');
        size_t n = eol ? (size_t)(eol - p) : strlen(p);
        char line[256];
        bool whole = n < sizeof line;
        size_t k = whole ? n : sizeof line - 1;
        memcpy(line, p, k);
        line[k] = 0;
        if (k && line[k - 1] == '\r') line[--k] = 0;
        p += n + (eol ? 1 : 0);
        if (!line[0] || line[0] == '#' || !whole) continue;
        char *eq = strchr(line, '=');
        long long page = -1, pages = -1, pid = -1;
        if (eq && !strncmp(line, "page.", 5)) {  // page.<book page>.<page count>=<page id>
            *eq = 0;
            char *dot = strchr(line + 5, '.');
            if (dot) {
                *dot = 0;
                page = number(line + 5, 1, 10000000);
                pages = number(dot + 1, 1, 10000000);
                *dot = '.';
            }
            pid = number(eq + 1, 1, 0x7ffffffe);
            *eq = '=';
        }
        if (eq && !strncmp(line, "book=", 5) && store_source_ok(line + 5)) {
            strlcpy_q(m->book, line + 5, sizeof m->book);
        } else if (eq && !strncmp(line, "kind=", 5) && word_ok(line + 5, sizeof m->kind)) {
            strlcpy_q(m->kind, line + 5, sizeof m->kind);
        } else if (page >= 1 && pages >= page && pid >= 1) {
            if (annotate_map_add(m, (int)page, (int)pages, (uint32_t)pid) != 0) {
                annotate_map_free(m);
                return -1;
            }
        } else {
            keep_line(m, line);
        }
    }
    return 0;
}

void annotate_map_free(AnnotateMap *m) {
    free(m->e);
    m->e = NULL;
    m->n = m->cap = 0;
}

// Book order: by the place in the book (page - 1 out of pages), then the page number, then the count.
static int book_cmp(int pa, int na, int pb, int nb) {
    int64_t x = (int64_t)(pa - 1) * nb, y = (int64_t)(pb - 1) * na;
    if (x != y) return x < y ? -1 : 1;
    if (pa != pb) return pa < pb ? -1 : 1;
    return na < nb ? -1 : na > nb;
}

static int cmp_entry(const void *a, const void *b) {
    const AnnotatePage *x = a, *y = b;
    return book_cmp(x->page, x->pages, y->page, y->pages);
}

char *annotate_map_text(const AnnotateMap *m, const NotebookInfo *nb, size_t *len) {
    AnnotatePage *e = malloc((size_t)(m->n ? m->n : 1) * sizeof *e);
    size_t cap = 256 + (size_t)m->n * 48 + sizeof m->extra;
    char *t = malloc(cap);
    if (!e || !t) {
        free(e);
        free(t);
        return NULL;
    }
    int n = 0;
    for (int i = 0; i < m->n; i++)
        if (in_notebook(nb, m->e[i].pid, NULL)) e[n++] = m->e[i];  // a deleted page's goes
    qsort(e, (size_t)n, sizeof *e, cmp_entry);
    size_t k = (size_t)snprintf(t, cap, "# Grimoire: the book this notebook annotates, from KOReader (docs/FORMAT.md)\n");
    if (m->book[0]) k += (size_t)snprintf(t + k, cap - k, "book=%s\n", m->book);
    if (m->kind[0]) k += (size_t)snprintf(t + k, cap - k, "kind=%s\n", m->kind);
    for (int i = 0; i < n; i++) k += (size_t)snprintf(t + k, cap - k, "page.%d.%d=%u\n", e[i].page, e[i].pages, e[i].pid);
    k += (size_t)snprintf(t + k, cap - k, "%s", m->extra);
    free(e);
    *len = k;
    return t;
}

uint32_t annotate_map_find(const AnnotateMap *m, const NotebookInfo *nb, int page, int pages) {
    for (int i = 0; i < m->n; i++)
        if (m->e[i].page == page && m->e[i].pages == pages && in_notebook(nb, m->e[i].pid, NULL)) return m->e[i].pid;
    return 0;
}

int annotate_insert_at(const NotebookInfo *nb, const AnnotateMap *m, int page, int pages) {
    for (int i = 0; i < nb->npages; i++)
        for (int k = 0; k < m->n; k++)
            if (m->e[k].pid == nb->page_ids[i] && book_cmp(m->e[k].page, m->e[k].pages, page, pages) > 0) return i;
    return nb->npages;
}

static void map_path(const char *nbdir, char *buf, size_t cap) { snprintf(buf, cap, "%s/source.ini", nbdir); }

// A notebook's source.ini (none: an empty map). -1 when it is there but does not read: then
// nothing may be written over it (its pages would lose their book pages).
static int map_load(const char *nbdir, AnnotateMap *m) {
    char path[800];
    map_path(nbdir, path, sizeof path);
    size_t len;
    char *t = read_file(path, &len);
    if (!t && access(path, F_OK) == 0) return -1;
    int rc = annotate_map_parse(t, m);
    free(t);
    return rc;
}

static int map_save(const char *nbdir, const AnnotateMap *m, const NotebookInfo *nb) {
    char path[800];
    map_path(nbdir, path, sizeof path);
    size_t len;
    char *t = annotate_map_text(m, nb, &len);
    int rc = t ? write_file_atomic(path, t, len) : -1;
    free(t);
    return rc;
}

// ---- names ----

// The length of the UTF-8 character at s (1 to 4 bytes), or 0 for a byte that starts none: a stray
// continuation byte, an overlong form, a surrogate, past U+10FFFF, or one cut short.
static size_t utf8_len(const uint8_t *s) {
    uint8_t c = s[0];
    if (c < 0x80) return 1;
    size_t n = c >= 0xF0 && c <= 0xF4 ? 4 : c >= 0xE0 ? 3 : c >= 0xC2 && c <= 0xDF ? 2 : 0;
    if (n == 0 || (c >= 0xF0 && n != 4)) return 0;
    for (size_t i = 1; i < n; i++)
        if ((s[i] & 0xC0) != 0x80) return 0;
    if (n == 3 && ((c == 0xE0 && s[1] < 0xA0) || (c == 0xED && s[1] >= 0xA0))) return 0;  // overlong, surrogate
    if (n == 4 && ((c == 0xF0 && s[1] < 0x90) || (c == 0xF4 && s[1] >= 0x90))) return 0;  // overlong, too big
    return n;
}

void annotate_name(const char *title, char *out, size_t cap) {
    if (!cap) return;
    size_t k = 0;
    const uint8_t *s = (const uint8_t *)(title ? title : "");
    while (*s && *s == ' ') s++;  // (leading spaces; control characters become spaces below)
    while (*s) {
        size_t n = utf8_len(s);
        if (n == 0) {  // broken: left out
            s++;
            continue;
        }
        bool control = n == 1 ? (*s < 0x20 || *s == 0x7F) : (n == 2 && s[0] == 0xC2 && s[1] < 0xA0);
        if (k + (control ? 1 : n) >= cap) break;  // the next character would not fit whole
        if (control) out[k++] = ' ';
        else
            for (size_t i = 0; i < n; i++) out[k++] = (char)s[i];
        s += n;
    }
    while (k > 0 && out[k - 1] == ' ') k--;
    out[k] = 0;
    size_t lead = 0;
    while (out[lead] == ' ') lead++;  // (a control character first)
    if (lead) memmove(out, out + lead, k - lead + 1);
}

// The name with " 2", " 3"... added when a notebook in all (n of them) has it already, as the
// Library names new notebooks: two notebooks of one name would share their PDF in grimoire/PDFs.
static void unique_name(const char *base, const NotebookInfo *all, int n, char *out, size_t cap) {
    char cut[STORE_NAME_LEN];
    strlcpy_q(out, base, cap);
    for (int k = 2; k < 1000; k++) {
        bool taken = false;
        for (int i = 0; i < n && !taken; i++) taken = !strcmp(all[i].name, out);
        if (!taken) return;
        char tail[16];
        int tl = snprintf(tail, sizeof tail, " %d", k);
        annotate_name(base, cut, cap - (size_t)tl < sizeof cut ? cap - (size_t)tl : sizeof cut);
        snprintf(out, cap, "%s%s", cut, tail);
    }
}

// ---- the picture ----

static uint32_t be32(const uint8_t *p) { return (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | p[3]; }

static uint32_t crc32_png(const uint8_t *p, size_t n) {
    static uint32_t table[256];
    static bool made;
    if (!made) {
        for (uint32_t i = 0; i < 256; i++) {
            uint32_t c = i;
            for (int k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
            table[i] = c;
        }
        made = true;
    }
    uint32_t c = 0xFFFFFFFFu;
    for (size_t i = 0; i < n; i++) c = table[(c ^ p[i]) & 0xFF] ^ (c >> 8);
    return c ^ 0xFFFFFFFFu;
}

int annotate_png_check(const uint8_t *data, size_t len, int *w, int *h, const char **why) {
    static const uint8_t sig[8] = {0x89, 'P', 'N', 'G', '\r', '\n', 0x1A, '\n'};
    *why = "not a PNG";
    if (!data || len < 8 || memcmp(data, sig, 8)) return -1;
    bool idat = false, iend = false;
    int chunks = 0;
    for (size_t at = 8; at < len && !iend; chunks++) {
        *why = "cut short";
        if (len - at < 12) return -1;
        uint32_t n = be32(data + at);
        const uint8_t *type = data + at + 4;
        if (n > len - at - 12) return -1;
        for (int i = 0; i < 4; i++)
            if (!((type[i] >= 'A' && type[i] <= 'Z') || (type[i] >= 'a' && type[i] <= 'z'))) {
                *why = "damaged (a chunk that is none)";
                return -1;
            }
        bool critical = !(type[0] & 0x20);
        if (critical && crc32_png(type, 4 + (size_t)n) != be32(data + at + 8 + n)) {
            *why = "damaged (a checksum does not match)";
            return -1;
        }
        if (chunks == 0 && (memcmp(type, "IHDR", 4) || n != 13)) {
            *why = "damaged (no header first)";
            return -1;
        }
        idat |= !memcmp(type, "IDAT", 4);
        iend = !memcmp(type, "IEND", 4);
        at += 12 + (size_t)n;
    }
    if (!iend || !idat) {
        *why = "cut short";
        return -1;
    }
    return bgimage_check(data, len, len, w, h, why);  // a picture Grimoire takes: its size, what it takes to decode
}

static void fsync_file(const char *path) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return;
    fsync(fd);
    close(fd);
}

static void sync_dir(const char *dir) {
    int fd = open(dir, O_RDONLY);
    if (fd < 0) return;
    fsync(fd);
    close(fd);
}

// <bgdir>/<id, 6 hex digits>.source.png: a picture of another size than its notebook's pages,
// waiting for BG_TEMPLATE to fit it (bgwork_template.c removes it then).
static void source_file(const char *bgdir, uint32_t id, char *buf, size_t cap) {
    snprintf(buf, cap, "%s/%06x.source.png", bgdir, (unsigned)(id & 0xFFFFFFu));
}

// Puts the request's picture (at src, the file's bytes in png, w x h) where it becomes picture id
// of a notebook whose bg folder is bgdir and whose pages are pw x ph: as it is when it is of that
// size and drawn as it is (bgimage_drawable), else as its source, to be fitted in the background
// (*fit). Nothing to do when the picture is there already. *placed: a file this call put there,
// for the caller to take back if what follows fails. 0, or -1 (errno).
static int place_picture(const char *bgdir, uint32_t id, const char *src, const uint8_t *png, size_t len, int w, int h,
                         int pw, int ph, bool *fit, char *placed, size_t cap) {
    char file[900];
    *fit = false;
    placed[0] = 0;
    bgimage_file(bgdir, id, file, sizeof file);
    if (access(file, F_OK) == 0) return 0;
    int dw, dh;
    const char *why;
    if (!(w == pw && h == ph && bgimage_drawable(png, len, &dw, &dh, &why) == 0)) {
        source_file(bgdir, id, file, sizeof file);
        *fit = true;
        if (access(file, F_OK) == 0) return 0;  // (waiting already)
    }
    if (rename(src, file) != 0) return -1;
    fsync_file(file);  // (KOReader wrote it without)
    sync_dir(bgdir);
    strlcpy_q(placed, file, cap);
    return 0;
}

// ---- the work ----

static char g_last_nb[STORE_ID_LEN];       // the notebook this start's request opened
static AnnotateOutcome g_outcome = ANNOTATE_NONE;

static void fail(AnnotateResult *out, AnnotateWhy why) {
    out->outcome = ANNOTATE_FAILED;
    out->why = why;
}

static void request_dir(char *buf, size_t cap) { snprintf(buf, cap, "%s/" ANNOTATE_DIR, store_root()); }

// Empties the requests' folder (and removes it): everything in it was one request's. How many files went.
static int empty_requests(void) {
    char dir[700];
    request_dir(dir, sizeof dir);
    DIR *d = opendir(dir);
    if (!d) return 0;
    int n = 0;
    for (struct dirent *e; (e = readdir(d));) {
        if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
        char path[1100];
        snprintf(path, sizeof path, "%s/%s", dir, e->d_name);
        n += unlink(path) == 0;
    }
    closedir(d);
    rmdir(dir);
    return n;
}

// Removes a folder a new notebook was being made in (its files, its bg/ and source.ini).
static void remove_work(const char *work) {
    for (int pass = 0; pass < 2; pass++) {
        char dir[800];
        snprintf(dir, sizeof dir, pass == 0 ? "%s/bg" : "%s", work);
        DIR *d = opendir(dir);
        for (struct dirent *e; d && (e = readdir(d));) {
            if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
            char path[1100];
            snprintf(path, sizeof path, "%s/%s", dir, e->d_name);
            unlink(path);
        }
        if (d) closedir(d);
        rmdir(dir);
    }
}

// What a start cut off while it made a notebook left (notebooks/.annotate-<id>/): never whole,
// since it becomes the notebook by its last step, a rename.
static void remove_leftovers(void) {
    char dir[600];
    snprintf(dir, sizeof dir, "%s/notebooks", store_root());
    DIR *d = opendir(dir);
    for (struct dirent *e; d && (e = readdir(d));) {
        if (strncmp(e->d_name, WORK_PREFIX, strlen(WORK_PREFIX))) continue;
        char work[900];
        snprintf(work, sizeof work, "%s/%s", dir, e->d_name);
        remove_work(work);
        qlog("annotate: removed a notebook a cut-off start was making (%s)", e->d_name + strlen(WORK_PREFIX));
    }
    if (d) closedir(d);
}

// Whether notebook id's source.ini says it annotates book: the link, once an older version (which
// drops notebook.ini's source= line on its next save) has lost it there.
static bool map_names(const char *id, const char *book) {
    char dir[600];
    AnnotateMap m;
    if (store_notebook_dir(id, dir, sizeof dir) != 0 || map_load(dir, &m) != 0) return false;
    bool yes = !strcmp(m.book, book);
    annotate_map_free(&m);
    return yes;
}

// The notebook of book in all (n, newest first): notebook.ini's source=, else source.ini's book=
// in a notebook with no source= line. Its index, or -1.
static int find_notebook(const NotebookInfo *all, int n, const char *book) {
    for (int i = 0; i < n; i++)
        if (!strcmp(all[i].source, book)) return i;
    for (int i = 0; i < n; i++)
        if (!all[i].source[0] && map_names(all[i].id, book)) return i;
    return -1;
}

// The request's page in notebook nb (loaded, its compaction stopped): the page that holds it, or a
// new one. Picture: the request's file (src), its bytes and size.
static void into_notebook(NotebookInfo *nb, const AnnotateRequest *r, const char *src, const uint8_t *png, size_t len,
                          int w, int h, AnnotateResult *out) {
    char dir[600], bgdir[700], placed[900] = "";
    AnnotateMap m;
    if (store_notebook_dir(nb->id, dir, sizeof dir) != 0 || bgimage_dir(nb->id, bgdir, sizeof bgdir) != 0) {
        fail(out, ANNOTATE_WHY_LOG);
        return;
    }
    if (map_load(dir, &m) != 0) {
        qlog("annotate: %s: its source.ini does not read; left as it is, nothing opened", nb->id);
        fail(out, ANNOTATE_WHY_LOG);
        return;
    }
    if (!nb->source[0]) qlog("annotate: %s: its link to the book was lost (an older version saved it); linked again", nb->id);
    strlcpy_q(nb->source, r->book, sizeof nb->source);
    if (!m.book[0]) strlcpy_q(m.book, r->book, sizeof m.book);
    if (!m.kind[0]) strlcpy_q(m.kind, r->kind, sizeof m.kind);
    int pw, ph;
    store_notebook_page_size(nb, &pw, &ph);
    uint64_t key = bgimage_source_key(png, len, pw, ph);
    BgIndex ix;
    uint32_t pic = 0, pid = annotate_map_find(&m, nb, r->page, r->pages);
    bool added = false, fit = false;
    if (pid) {
        // There already: opened as it is. Its picture, should it be missing (its fitting never ran),
        // comes from this one when this one is the same.
        int index = 0;
        in_notebook(nb, pid, &index);
        char file[900];
        Page *pg = NULL;
        if (store_read_page(nb, index, &pg) == 0 && paper_is_image(pg->paper) && mkdir_p(bgdir) == 0 &&
            bgindex_load(bgdir, &ix) == 0) {
            uint32_t want = paper_image_id(pg->paper);
            bgimage_file(bgdir, want, file, sizeof file);
            if (access(file, F_OK) != 0 && bgindex_find(&ix, key, &pic) && pic == want &&
                place_picture(bgdir, pic, src, png, len, w, h, pw, ph, &fit, placed, sizeof placed) == 0) {
                qlog("annotate: %s: page %d of %d is there; its missing picture is made again", nb->id, r->page, r->pages);
                if (fit) {
                    source_file(bgdir, pic, file, sizeof file);
                    bg_queue(BG_TEMPLATE, nb->id, file, store_dpi(), false);
                }
            }
            bgindex_free(&ix);
        }
        page_free(pg);
        nb->last_page = index;
        if (store_write_info_in(dir, nb) != 0) qlog("annotate: %s: could not note the page to open", nb->id);
        qlog("annotate: %s: page %d of %d is its page %d", nb->id, r->page, r->pages, index + 1);
        out->outcome = ANNOTATE_OPENED;
        out->pid = pid;
        annotate_map_free(&m);
        return;
    }
    if (nb->npages >= STORE_MAX_PAGES) {
        qlog("annotate: %s: it has %d pages, the most a notebook has; nothing added", nb->id, nb->npages);
        fail(out, ANNOTATE_WHY_LOG);
        annotate_map_free(&m);
        return;
    }
    // A new page, in book order, on a picture of its own (or one of its pages' that is the same).
    if (mkdir_p(bgdir) != 0 || bgindex_load(bgdir, &ix) != 0) {
        qlog("annotate: %s: its bg folder cannot be written, or bg/index.ini does not read; nothing added", nb->id);
        fail(out, ANNOTATE_WHY_STORAGE);
        annotate_map_free(&m);
        return;
    }
    int rc = bgindex_claim(&ix, bgdir, key, &pic, &added);
    if (rc == 0 && added) rc = bgindex_save(bgdir, &ix);
    bgindex_free(&ix);
    if (rc == 0) rc = place_picture(bgdir, pic, src, png, len, w, h, pw, ph, &fit, placed, sizeof placed);
    int at = annotate_insert_at(nb, &m, r->page, r->pages);
    PaperKind paper = paper_image(PAPER_ROLE_DOCUMENT, pic);
    Page *p = rc == 0 ? page_new(pw, ph, paper) : NULL;
    if (p) strlcpy_q(p->nb, nb->id, sizeof p->nb);
    // Added on blank paper first (store_add_page gives no page another's document picture), then
    // given its own; taken back whole if anything fails.
    bool page_added = p && store_add_page(nb, at, PAPER_BLANK) == 0;
    rc = page_added && store_save_page(nb, at, p) == 0 ? 0 : -1;
    page_free(p);
    if (rc == 0) {
        pid = nb->page_ids[at];
        rc = annotate_map_add(&m, r->page, r->pages, pid) == 0 && map_save(dir, &m, nb) == 0 ? 0 : -1;
    }
    if (rc != 0) {
        if (page_added) store_delete_page(nb, at);
        if (placed[0]) unlink(placed);
        qlog("annotate: %s: page %d of %d could not be added (%s); the notebook stays as it was", nb->id, r->page,
             r->pages, strerror(errno));
        fail(out, ANNOTATE_WHY_STORAGE);
        annotate_map_free(&m);
        return;
    }
    // A new first page: the Library's card shows it (the thumbnail of the page before goes, and the
    // notebook's paper, a document page's as the notebook was made, is this page's picture).
    if (at == 0) {
        char thumb[700];
        snprintf(thumb, sizeof thumb, "%s/thumb.pgm", dir);
        unlink(thumb);
        if (paper_is_document(nb->paper)) nb->paper = paper;
    }
    nb->last_page = at;
    if (store_save_info(nb) != 0) qlog("annotate: %s: could not note the page to open", nb->id);
    if (fit) {
        char file[900];
        source_file(bgdir, pic, file, sizeof file);
        bg_queue(BG_TEMPLATE, nb->id, file, store_dpi(), false);
    }
    qlog("annotate: %s: page %d of %d added as its page %d of %d, picture bg/%06x.png%s", nb->id, r->page, r->pages,
         at + 1, nb->npages, (unsigned)pic, fit ? " (fitted to its pages in the background)" : "");
    out->outcome = ANNOTATE_MADE;
    out->pid = pid;
    annotate_map_free(&m);
}

// A new notebook for the book, its one page the request's: made in notebooks/.annotate-<id>/ (a
// dot-folder the Library and every older version skip) and renamed into place once whole, so a
// failure or a start cut off leaves no notebook half made.
static void new_notebook(const AnnotateRequest *r, const NotebookInfo *all, int n, const char *src, const uint8_t *png,
                         size_t len, int w, int h, AnnotateResult *out) {
    NotebookInfo *nb = calloc(1, sizeof *nb);
    char work[700], final[600], bgdir[800], path[900], placed[900] = "";
    if (!nb) {
        fail(out, ANNOTATE_WHY_LOG);
        return;
    }
    for (int tries = 0;; tries++) {  // (a notebook made the same second with the same random part)
        store_make_id(nb->id);
        if (store_notebook_dir(nb->id, final, sizeof final) != 0 || tries == 8) {
            free(nb);
            fail(out, ANNOTATE_WHY_LOG);
            return;
        }
        if (access(final, F_OK) != 0) break;
    }
    snprintf(work, sizeof work, "%s/notebooks/" WORK_PREFIX "%s", store_root(), nb->id);
    snprintf(bgdir, sizeof bgdir, "%s/bg", work);
    int pw, ph;
    store_page_size(&pw, &ph);
    uint64_t key = bgimage_source_key(png, len, pw, ph);
    BgIndex ix = {0};
    uint32_t pic = 0;
    bool added = false, fit = false;
    remove_work(work);
    int rc = mkdir_p(bgdir) == 0 && bgindex_claim(&ix, bgdir, key, &pic, &added) == 0 && bgindex_save(bgdir, &ix) == 0
                 ? 0
                 : -1;
    bgindex_free(&ix);
    if (rc == 0) rc = place_picture(bgdir, pic, src, png, len, w, h, pw, ph, &fit, placed, sizeof placed);
    PaperKind paper = paper_image(PAPER_ROLE_DOCUMENT, pic);
    if (rc == 0) {
        Page *p = page_new(pw, ph, paper);
        uint32_t one = 1;
        rc = p && store_write_page_in(work, 1, p) == 0 && store_write_pages_list_in(bgdir, &one, &paper, 1) == 0 ? 0 : -1;
        page_free(p);
    }
    if (rc == 0) {
        AnnotateMap m = {0};
        strlcpy_q(m.book, r->book, sizeof m.book);
        strlcpy_q(m.kind, r->kind, sizeof m.kind);
        char *t = NULL;
        size_t tl;
        nb->npages = 1;
        nb->page_ids[0] = 1;
        if (annotate_map_add(&m, r->page, r->pages, 1) == 0) t = annotate_map_text(&m, nb, &tl);
        snprintf(path, sizeof path, "%s/source.ini", work);
        rc = t && write_file_atomic(path, t, tl) == 0 ? 0 : -1;
        free(t);
        annotate_map_free(&m);
    }
    if (rc == 0) {
        // Named after the book (one with no title stays unnamed: read as Untitled, as any such);
        // its own paper is its first page's (a document page's: pages added to it get Blank paper,
        // and the Library's card shows that page until it has a thumbnail).
        char name[STORE_NAME_LEN];
        annotate_name(r->title, name, sizeof name);
        unique_name(name, all, n, nb->name, sizeof nb->name);
        nb->paper = paper;
        nb->created = nb->modified = wall_time();
        nb->next_page_id = 2;
        nb->page_w = pw;
        nb->page_h = ph;
        strlcpy_q(nb->source, r->book, sizeof nb->source);
        // notebook.ini last: then the folder becomes the notebook.
        rc = store_write_info_in(work, nb) == 0 ? 0 : -1;
    }
    if (rc == 0) {
        sync_dir(bgdir);
        sync_dir(work);
        rc = rename(work, final);
    }
    if (rc != 0) {
        qlog("annotate: page %d of %d: a notebook for it could not be made (%s); nothing made", r->page, r->pages,
             strerror(errno));
        remove_work(work);
        fail(out, ANNOTATE_WHY_STORAGE);
        free(nb);
        return;
    }
    char notebooks[600];
    snprintf(notebooks, sizeof notebooks, "%s/notebooks", store_root());
    sync_dir(notebooks);
    if (fit) {
        char file[900], fbg[700];
        bgimage_dir(nb->id, fbg, sizeof fbg);
        source_file(fbg, pic, file, sizeof file);
        bg_queue(BG_TEMPLATE, nb->id, file, store_dpi(), false);
    }
    qlog("annotate: notebook %s made for a %s book, page %d of %d, picture bg/%06x.png%s", nb->id,
         r->kind[0] ? r->kind : "(kind not given)", r->page, r->pages, (unsigned)pic,
         fit ? " (fitted to its pages in the background)" : "");
    strlcpy_q(out->nb, nb->id, sizeof out->nb);
    out->outcome = ANNOTATE_MADE;
    out->pid = 1;
    free(nb);
}

// The request read and fresh: its picture checked, then its page found or made.
static void handle(const AnnotateRequest *r, AnnotateResult *out) {
    char dir[700], src[800];
    request_dir(dir, sizeof dir);
    snprintf(src, sizeof src, "%s/%s", dir, r->picture);
    size_t len = 0;
    uint8_t *png = bgimage_read(src, PICTURE_MAX, &len);
    int w = 0, h = 0;
    const char *why = errno == EFBIG ? "bigger than a page picture can be" : "missing or unreadable";
    if (!png || annotate_png_check(png, len, &w, &h, &why) != 0) {
        qlog("annotate: page %d of %d: its picture is %s; nothing opened", r->page, r->pages, why);
        fail(out, ANNOTATE_WHY_PICTURE);
        free(png);
        return;
    }
    NotebookInfo *all = NULL;
    int n = 0;
    if (store_list(&all, &n) != 0) {
        qlog("annotate: the notebooks do not list; nothing opened");
        fail(out, ANNOTATE_WHY_LOG);
        free(png);
        return;
    }
    int at = find_notebook(all, n, r->book);
    if (at >= 0) {
        // Its compaction (queued at this start for what an earlier one left) stops before its page
        // list changes, as when a notebook opens.
        bg_cancel_notebook(all[at].id);
        NotebookInfo *nb = malloc(sizeof *nb);
        if (nb && store_load_info(all[at].id, nb) == 0) {
            strlcpy_q(out->nb, nb->id, sizeof out->nb);
            into_notebook(nb, r, src, png, len, w, h, out);
        } else {
            fail(out, ANNOTATE_WHY_LOG);
        }
        free(nb);
    } else {
        new_notebook(r, all, n, src, png, len, w, h, out);
    }
    free(all);
    free(png);
}

void annotate_take(bool asked, AnnotateResult *out) {
    memset(out, 0, sizeof *out);
    remove_leftovers();
    char dir[700], path[800];
    request_dir(dir, sizeof dir);
    snprintf(path, sizeof path, "%s/" ANNOTATE_REQUEST, dir);
    struct stat st;
    bool have = stat(path, &st) == 0 && S_ISREG(st.st_mode);
    AnnotateRequest r;
    const char *why = "it does not read";
    size_t len;
    char *text = have && st.st_size <= REQUEST_MAX ? read_file(path, &len) : NULL;
    bool parsed = text && annotate_parse(text, &r, &why) == 0;
    free(text);
    if (!have) {
        if (asked) {
            qlog("annotate: KOReader asked for a page, but there is no request in " ANNOTATE_DIR);
            fail(out, ANNOTATE_WHY_LOG);
        }
    } else if (!asked) {
        qlog("annotate: a request from KOReader that this start was not asked to open (Grimoire started from the "
             "Library, or by Open Grimoire); deleted unopened");
        out->outcome = ANNOTATE_STALE;
    } else if (!parsed) {
        qlog("annotate: the request does not read (%s); nothing opened", st.st_size > REQUEST_MAX ? "too big" : why);
        fail(out, ANNOTATE_WHY_LOG);
    } else if (annotate_too_old(&r, wall_time(), (int64_t)st.st_mtime)) {
        long long age = (long long)(wall_time() - (r.time > 0 ? r.time : (int64_t)st.st_mtime));
        qlog("annotate: a request from %lld s %s; deleted unopened", age < 0 ? -age : age,
             age >= 0 ? "ago" : "ahead (the clock was set back?)");
        out->outcome = ANNOTATE_STALE;
    } else {
        out->page = r.page;
        out->pages = r.pages;
        handle(&r, out);
    }
    int gone = empty_requests();
    if (gone > 2) qlog("annotate: %d files in " ANNOTATE_DIR " deleted", gone);
    g_outcome = out->outcome;
    strlcpy_q(g_last_nb, out->outcome == ANNOTATE_OPENED || out->outcome == ANNOTATE_MADE ? out->nb : "",
              sizeof g_last_nb);
}

// ---- probes ----

int annotate_probe(const char *key, int open_page, char *out, size_t cap) {
    static const char *const outcomes[] = {"none", "stale", "opened", "made", "failed"};
    if (!strcmp(key, "result")) {
        snprintf(out, cap, "%s", outcomes[g_outcome]);
    } else if (!strcmp(key, "leftover")) {
        char dir[700];
        request_dir(dir, sizeof dir);
        int n = 0;
        DIR *d = opendir(dir);
        for (struct dirent *e; d && (e = readdir(d));) n += strcmp(e->d_name, ".") && strcmp(e->d_name, "..");
        if (d) closedir(d);
        snprintf(out, cap, "%d", n);
    } else if (!strcmp(key, "book_page")) {
        out[0] = 0;
        NotebookInfo *nb = g_last_nb[0] ? malloc(sizeof *nb) : NULL;
        char dir[600];
        AnnotateMap m;
        if (nb && store_load_info(g_last_nb, nb) == 0 && open_page >= 0 && open_page < nb->npages &&
            store_notebook_dir(nb->id, dir, sizeof dir) == 0 && map_load(dir, &m) == 0) {
            for (int i = 0; i < m.n; i++)
                if (m.e[i].pid == nb->page_ids[open_page]) snprintf(out, cap, "%d/%d", m.e[i].page, m.e[i].pages);
            annotate_map_free(&m);
        }
        free(nb);
    } else {
        return -1;
    }
    return 0;
}
