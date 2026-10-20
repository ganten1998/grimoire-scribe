// bgwork_compact.c: BG_COMPACT_PAGES, taking erased ink out of a notebook's page files (compact.h).
//
// It runs in bgjob.c's background child: page_droppable can take tens of seconds on a heavily
// erased page on the Kindle, never something for the UI thread. For each page listed in the
// notebook's erased.txt it reads the page file, and when page_droppable finds strokes that change
// nothing (ink that later erase strokes restore entirely, erase strokes left with nothing to
// change) it writes the page without them, through a temporary file and a rename, and only if the
// file still holds what it read (an older version, or a copy from a computer, may have written it
// since). A page that is done (compacted, or with nothing to leave out) goes off the list, and the
// list goes once it is empty.
//
// Stopping it is safe at any point. bg_cancel_notebook (SIGTERM) runs before the notebook opens,
// and SIGTERM waits while a file is being replaced, so a stop never leaves a temporary file behind:
// each page is its old file or its new one. It stops by itself when Grimoire goes (its parent), so
// a Grimoire started again can open the notebook at once: the list keeps the pages for next time.
//
// Compaction never changes how a page looks (page_droppable checks the page with its paper and its
// ink layer), so the notebook's PDF mirror comes out the same whether it runs before compaction or
// after it. The two never run at once anyway (bgjob.c runs one child at a time, and keeps
// compaction out of what it hands to the child that outlives Grimoire), and a page file is replaced
// in one rename, so any reader (the mirror, the share server) sees the old file or the new one.
#include "bgjob.h"
#include "compact.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#ifdef __linux__
#include <sys/prctl.h>
#endif

#include "store.h"

#define LIST_NAME "erased.txt"
#define LIST_MAX 4096  // ids read from one list (a notebook has at most STORE_MAX_PAGES pages)

static int list_path(const char *id, char *buf, size_t cap) {
    char dir[600];
    if (store_notebook_dir(id, dir, sizeof dir) != 0) return -1;
    snprintf(buf, cap, "%s/" LIST_NAME, dir);
    return 0;
}

// The page ids notebook `id` lists, each once, in the order listed: 0 (*n may be 0), or -1 when it
// has no list (or there is no memory to read it). A line that is not a page id is skipped.
static int list_read(const char *id, uint32_t **out, int *n) {
    *out = NULL;
    *n = 0;
    char path[700];
    if (list_path(id, path, sizeof path) != 0) return -1;
    size_t len;
    char *text = read_file(path, &len);
    if (!text) return -1;
    uint32_t *ids = malloc(LIST_MAX * sizeof *ids);
    if (!ids) {
        free(text);
        return -1;
    }
    int k = 0;
    for (char *l = text; *l && k < LIST_MAX;) {
        char *nl = strchr(l, '\n'), *end = l;
        if (nl) *nl = 0;
        while (*l == ' ' || *l == '\t') l++;
        unsigned long v = *l >= '0' && *l <= '9' ? strtoul(l, &end, 10) : 0;
        if (v > 0 && v < 0x7fffffffUL && (*end == 0 || *end == '\r' || *end == ' ')) {
            bool seen = false;
            for (int i = 0; i < k && !seen; i++) seen = ids[i] == (uint32_t)v;
            if (!seen) ids[k++] = (uint32_t)v;
        }
        if (!nl) break;
        l = nl + 1;
    }
    free(text);
    *out = ids;
    *n = k;
    return 0;
}

// Writes notebook `id`'s list (n ids), or removes it when n is 0. 0, or -1.
static int list_write(const char *id, const uint32_t *ids, int n) {
    char path[700];
    if (list_path(id, path, sizeof path) != 0) return -1;
    if (n == 0) return unlink(path) == 0 || errno == ENOENT ? 0 : -1;
    size_t cap = 128 + (size_t)n * 12, k;
    char *t = malloc(cap);
    if (!t) return -1;
    k = (size_t)snprintf(t, cap, "# Grimoire: pages that may still hold erased ink, until compaction takes it out\n");
    for (int i = 0; i < n && k < cap; i++) k += (size_t)snprintf(t + k, cap - k, "%u\n", (unsigned)ids[i]);
    int rc = k < cap ? write_file_atomic(path, t, k) : -1;
    free(t);
    return rc;
}

int compact_note_page(const char *id, uint32_t page_id) {
    uint32_t *ids;
    int n;
    if (list_read(id, &ids, &n) != 0) {
        ids = malloc(sizeof *ids);
        n = 0;
        if (!ids) return -1;
    }
    for (int i = 0; i < n; i++)
        if (ids[i] == page_id) {
            free(ids);
            return 0;
        }
    int rc = -1;
    if (n < LIST_MAX) {
        uint32_t *more = realloc(ids, (size_t)(n + 1) * sizeof *ids);
        if (more) {
            ids = more;
            ids[n++] = page_id;
            rc = list_write(id, ids, n);
        }
    }
    free(ids);
    return rc;
}

bool compact_listed(const char *id) {
    uint32_t *ids;
    int n;
    if (list_read(id, &ids, &n) != 0) return false;
    free(ids);
    return n > 0;
}

int compact_queue_listed(float dpi) {
    char dir[600];
    snprintf(dir, sizeof dir, "%s/notebooks", store_root());
    DIR *d = opendir(dir);
    if (!d) return 0;
    int n = 0;
    struct dirent *e;
    while ((e = readdir(d))) {
        // (compact_listed is false for a name that cannot be a notebook id)
        if (e->d_name[0] == '.' || !compact_listed(e->d_name)) continue;
        bg_queue(BG_COMPACT_PAGES, e->d_name, NULL, dpi, false);
        n++;
    }
    closedir(d);
    if (n) qlog("compact: %d notebook(s) with erased ink left to take out from an earlier run", n);
    return n;
}

// ---- the background pass ----

// SIGTERM waits while a file is replaced (bg_cancel_notebook's stop then lands after it).
static void hold_stops(sigset_t *old) {
    sigset_t s;
    sigemptyset(&s);
    sigaddset(&s, SIGTERM);
    sigprocmask(SIG_BLOCK, &s, old);
}
static void allow_stops(const sigset_t *old) { sigprocmask(SIG_SETMASK, old, NULL); }

// Grimoire (the parent the pass started under) is gone: stop before writing anything more.
static bool parent_gone(pid_t parent) {
    pid_t p = getppid();
    return p != parent || p == 1;
}

// Replaces `path` with `data` through a temporary file, but only if the file still holds `was`
// (checked once the new file is written, just before the rename): 0 replaced, 1 the file is not
// the one read any more (it stays as it is), -1 could not write.
static int replace_if_same(const char *dir, const char *path, const uint8_t *was, size_t was_len, const void *data,
                           size_t len) {
    char tmp[760];
    snprintf(tmp, sizeof tmp, "%s.compact.tmp", path);
#ifdef GRIMOIRE_SIM
    if (g_sim_storage_full) return -1;
#endif
    int fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) return -1;
    const uint8_t *p = data;
    size_t left = len;
    bool ok = true;
    while (left && ok) {
        ssize_t w = write(fd, p, left);
        if (w < 0 && errno == EINTR) continue;
        ok = w > 0;
        if (ok) {
            p += w;
            left -= (size_t)w;
        }
    }
    ok = ok && fsync(fd) == 0;
    ok = close(fd) == 0 && ok;
    if (!ok) {
        unlink(tmp);
        return -1;
    }
    size_t now_len;
    uint8_t *now = read_file(path, &now_len);
    bool same = now && now_len == was_len && !memcmp(now, was, was_len);
    free(now);
    if (!same) {
        unlink(tmp);
        return 1;
    }
    if (rename(tmp, path) != 0) {
        unlink(tmp);
        return -1;
    }
    int dfd = open(dir, O_RDONLY);  // best effort: the rename itself durable
    if (dfd >= 0) {
        fsync(dfd);
        close(dfd);
    }
    return 0;
}

typedef enum { PAGE_DONE, PAGE_AGAIN } PageDone;  // off the list / stays listed for next time

// One listed page of nb: its file without the strokes that change nothing, if there are any.
static PageDone compact_page(const NotebookInfo *nb, const char *dir, uint32_t pid, pid_t parent, int *left_out) {
    char path[700];
    snprintf(path, sizeof path, "%s/p%u.qpg", dir, (unsigned)pid);
    size_t len;
    uint8_t *was = read_file(path, &len);
    if (!was) return PAGE_DONE;  // no file (never written, or gone): no ink in it
    Page *p = NULL;
    PageDone res = PAGE_DONE;
    int w, h;
    store_notebook_page_size(nb, &w, &h);
    if (page_deserialize(was, len, &p) != 0 || !p) {
        qlog("compact: %s p%u does not read; left for the editor", nb->id, (unsigned)pid);
    } else if (p->readonly || p->w != w || p->h != h) {
        // A newer version's page is never written; one at another size (an older version wrote it
        // on another Scribe) goes back to the notebook's size once the editor writes it.
        qlog("compact: %s p%u is %s; left as it is", nb->id, (unsigned)pid,
             p->readonly ? "read-only" : "of another size");
    } else {
        int erase = 0;
        for (int i = 0; i < p->n; i++) erase += stroke_is_erase(p->strokes[i]);
        bool *drop = erase ? calloc((size_t)p->n, sizeof *drop) : NULL;
        uint32_t t0 = now_ms();
        // At the dpi the reader built the strokes' reach with (store_dpi), the editor's.
        int n = drop ? page_droppable(p, store_dpi(), drop) : 0;
        uint32_t ms = now_ms() - t0;
        if (erase && !drop) n = -1;
        if (n < 0) {
            qlog("compact: %s p%u: out of memory; tried again next time", nb->id, (unsigned)pid);
            res = PAGE_AGAIN;
        } else if (n == 0) {
            if (erase)
                qlog("compact: %s p%u: nothing to leave out (%d strokes, %d erase), %u ms", nb->id, (unsigned)pid,
                     p->n, erase, (unsigned)ms);
        } else {
            int total = p->n, kept = 0, gone_erase = 0;
            for (int i = 0; i < p->n; i++) {
                if (!drop[i]) {
                    p->strokes[kept++] = p->strokes[i];
                    continue;
                }
                gone_erase += stroke_is_erase(p->strokes[i]);
                stroke_free(p->strokes[i]);
            }
            p->n = kept;
            uint8_t *data;
            size_t dlen;
            int rc = -1;
            if (page_serialize(p, &data, &dlen) == 0) {
                sigset_t old;
                hold_stops(&old);
                rc = parent_gone(parent) ? 2 : replace_if_same(dir, path, was, len, data, dlen);
                allow_stops(&old);
                free(data);
            }
            if (rc == 0) {
                qlog("compact: %s p%u: %d of %d strokes left out (%d erase), %u ms", nb->id, (unsigned)pid, n, total,
                     gone_erase, (unsigned)ms);
                *left_out += n;
            } else {
                qlog("compact: %s p%u: %s; tried again next time", nb->id, (unsigned)pid,
                     rc == 1 ? "the file changed meanwhile" : rc == 2 ? "Grimoire has gone" : "could not write it");
                res = PAGE_AGAIN;
            }
        }
        free(drop);
    }
    page_free(p);
    free(was);
    return res;
}

int bg_compact_pages(const char *id, float dpi) {
    (void)dpi;  // the pages are checked at store_dpi(), the dpi their strokes were read at
    pid_t parent = getppid();
#ifdef __linux__
    prctl(PR_SET_PDEATHSIG, SIGTERM);  // Grimoire gone: stop (its list keeps the pages)
#endif
    uint32_t *ids;
    int n;
    if (list_read(id, &ids, &n) != 0) return 0;  // nothing listed
    NotebookInfo *nb = malloc(sizeof *nb);
    char dir[600];
    if (!nb || store_load_info(id, nb) != 0 || store_notebook_dir(id, dir, sizeof dir) != 0) {
        qlog("compact: %s cannot be read; its list stays", id);
        free(nb);
        free(ids);
        return -1;
    }
    int listed = n, left_out = 0;
    if (n == 0) {  // a list with no page ids in it lists nothing: it goes
        sigset_t old;
        hold_stops(&old);
        list_write(id, ids, 0);
        allow_stops(&old);
    }
    for (int i = 0; i < n && !parent_gone(parent);) {
        bool in_book = false;
        for (int k = 0; k < nb->npages && !in_book; k++) in_book = nb->page_ids[k] == ids[i];
        if ((in_book ? compact_page(nb, dir, ids[i], parent, &left_out) : PAGE_DONE) == PAGE_AGAIN) {
            i++;
            continue;
        }
        memmove(&ids[i], &ids[i + 1], (size_t)(n - i - 1) * sizeof *ids);
        n--;
        sigset_t old;
        hold_stops(&old);
        if (!parent_gone(parent) && list_write(id, ids, n) != 0) qlog("compact: %s: could not update its list", id);
        allow_stops(&old);
    }
    if (listed) qlog("compact: %s: %d of %d listed page(s) done, %d stroke(s) left out", id, listed - n, listed, left_out);
    free(ids);
    free(nb);
    return 0;
}
