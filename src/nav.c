#include "nav.h"
#include "i18n.h"
#include <errno.h>
#include <math.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

void nav_init(Nav *n) { memset(n, 0, sizeof *n); n->writable = n->links_writable = true; }
void nav_free(Nav *n) {
    free(n->names); free(n->unknown); free(n->links); free(n->links_unknown);
    memset(n, 0, sizeof *n);
}

static int slot(const Nav *n, uint32_t pid) {
    int lo = 0, hi = n->count;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;
        if (n->names[mid].pid < pid) lo = mid + 1;
        else hi = mid;
    }
    return lo;
}

const char *nav_name(const Nav *n, uint32_t pid) {
    int at = slot(n, pid);
    return at < n->count && n->names[at].pid == pid ? n->names[at].text : "";
}

int nav_set_name(Nav *n, uint32_t pid, const char *name) {
    if (!n->writable || !pid) return -1;
    char text[NAV_NAME_LEN];
    // Trim before cutting, so leading whitespace does not use up the name's byte budget.
    while (name && *name && ((unsigned char)*name <= 32 || *name == 127)) name++;
    size_t len = i18n_copy(text, name ? name : "", sizeof text);
    for (size_t i = 0; i < len; i++)
        if ((unsigned char)text[i] < 32 || text[i] == 127) text[i] = ' ';
    while (len && text[len - 1] == ' ') text[--len] = 0;
    int at = slot(n, pid);
    bool found = at < n->count && n->names[at].pid == pid;
    if (!len) {
        if (found) {
            memmove(&n->names[at], &n->names[at + 1], (size_t)(n->count - at - 1) * sizeof *n->names);
            n->count--;
        }
        return 0;
    }
    if (!found) {
        if (n->count >= STORE_MAX_PAGES) return -1;
        if (n->count == n->capacity) {
            int cap = clampi(n->capacity ? n->capacity * 2 : 16, 16, STORE_MAX_PAGES);
            NavName *p = realloc(n->names, (size_t)cap * sizeof *p);
            if (!p) return -1;
            n->names = p; n->capacity = cap;
        }
        memmove(&n->names[at + 1], &n->names[at], (size_t)(n->count - at) * sizeof *n->names);
        n->count++;
    }
    n->names[at].pid = pid;
    memcpy(n->names[at].text, text, len + 1);
    return 0;
}

static int path_for(const char *id, const char *file, char *path, size_t cap) {
    char dir[600];
    if (store_notebook_dir(id, dir, sizeof dir) != 0) return -1;
    int len = snprintf(path, cap, "%s/%s", dir, file);
    return len < 0 || (size_t)len >= cap ? -1 : 0;
}

static bool name_key(const char *line, uint32_t *pid, const char **value) {
    if (strncmp(line, "name.", 5)) return false;
    const char *p = line + 5;
    if (*p < '0' || *p > '9') return false;
    uint64_t id = 0;
    while (*p >= '0' && *p <= '9') {
        id = id * 10 + (unsigned)(*p++ - '0');
        if (id > UINT32_MAX) return false;
    }
    if (*p != '=' || !id) return false;
    *pid = (uint32_t)id; *value = p + 1;
    return true;
}

// A sidecar's text, NUL-terminated (*text NULL when there is no file): 0, or -1 when it is there and
// does not read, is not a regular file, is a symlink, is over NAV_FILE_MAX or holds a NUL.
static int read_sidecar(const char *path, char **text, size_t *len) {
    *text = NULL; *len = 0;
    int fd = open(path, O_RDONLY | O_NONBLOCK | O_NOFOLLOW);
    if (fd < 0) {
        struct stat st;
        return errno == ENOENT && lstat(path, &st) != 0 && errno == ENOENT ? 0 : -1;
    }
    struct stat st;
    char *buf = NULL;
    size_t got_len = 0;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode) || st.st_size < 0 || st.st_size > NAV_FILE_MAX) goto fail;
    buf = malloc(NAV_FILE_MAX + 1);
    if (!buf) goto fail;
    for (;;) {
        ssize_t got = read(fd, buf + got_len, NAV_FILE_MAX + 1 - got_len);
        if (got < 0) { if (errno == EINTR) continue; goto fail; }
        if (!got) break;
        got_len += (size_t)got;
        if (got_len > NAV_FILE_MAX) goto fail;
    }
    if (memchr(buf, 0, got_len)) goto fail;  // never rewrite a file with a hidden binary tail
    buf[got_len] = 0;
    close(fd);
    *text = buf; *len = got_len;
    return 0;
fail:
    close(fd);
    free(buf);
    return -1;
}

// Keeps a line this version does not read, to write back: -1 past the limits.
static int carry(char **buf, size_t *len, const char *line, size_t size) {
    if (size > 255 || *len + size + 1 > NAV_UNKNOWN_MAX) return -1;
    char *p = realloc(*buf, *len + size + 1);
    if (!p) return -1;
    *buf = p;
    memcpy(p + *len, line, size);
    *len += size;
    p[(*len)++] = '\n';
    return 0;
}

// Calls line_fn for each line of text (its end cut off, one carriage return too) until it fails.
static int each_line(char *text, size_t len, int (*line_fn)(Nav *, char *, size_t), Nav *n) {
    for (char *line = text; line < text + len;) {
        char *end = strchr(line, '\n');
        if (end) *end = 0;
        size_t size = strlen(line);
        if (size && line[size - 1] == '\r') line[--size] = 0;
        if (line_fn(n, line, size) != 0) return -1;
        if (!end) break;
        line = end + 1;
    }
    return 0;
}

static int name_line(Nav *n, char *line, size_t size) {
    uint32_t pid; const char *value;
    if (name_key(line, &pid, &value)) return nav_set_name(n, pid, value);
    if (size && line[0] != '#') return carry(&n->unknown, &n->unknown_len, line, size);
    return 0;
}

static int link_put(Nav *n, uint32_t pid, uint32_t seed, const char *target);

// link.<pid>=<seed, 1 to 8 hex digits>,<target, 1 to 15 bytes from ! to ~>
static bool link_parse(const char *line, uint32_t *pid, uint32_t *seed, const char **target) {
    if (strncmp(line, "link.", 5)) return false;
    const char *p = line + 5;
    uint64_t id = 0;
    if (*p < '0' || *p > '9') return false;
    while (*p >= '0' && *p <= '9') {
        id = id * 10 + (unsigned)(*p++ - '0');
        if (id > UINT32_MAX) return false;
    }
    if (*p++ != '=' || !id) return false;
    uint32_t s = 0;
    int digits = 0;
    for (; digits < 9; digits++, p++) {
        int v = *p >= '0' && *p <= '9' ? *p - '0' : *p >= 'a' && *p <= 'f' ? *p - 'a' + 10
              : *p >= 'A' && *p <= 'F' ? *p - 'A' + 10 : -1;
        if (v < 0) break;
        s = s << 4 | (uint32_t)v;
    }
    if (!digits || digits > 8 || *p++ != ',') return false;
    size_t tl = strlen(p);
    if (!tl || tl >= NAV_TARGET_LEN) return false;
    for (size_t i = 0; i < tl; i++) if (p[i] < '!' || p[i] > '~') return false;
    *pid = (uint32_t)id; *seed = s; *target = p;
    return true;
}

static int link_line(Nav *n, char *line, size_t size) {
    uint32_t pid, seed; const char *target;
    if (link_parse(line, &pid, &seed, &target)) return link_put(n, pid, seed, target);
    if (size && line[0] != '#') return carry(&n->links_unknown, &n->links_unknown_len, line, size);
    return 0;
}

int nav_load(const char *id, Nav *n) {
    Nav fresh; nav_init(&fresh);
    char path[700], *text = NULL;
    size_t len = 0;
    bool names_ok = path_for(id, "nav.ini", path, sizeof path) == 0 && read_sidecar(path, &text, &len) == 0 &&
                    (!text || each_line(text, len, name_line, &fresh) == 0);
    free(text); text = NULL;
    if (!names_ok) {
        free(fresh.names); free(fresh.unknown);
        fresh.names = NULL; fresh.unknown = NULL;
        fresh.count = fresh.capacity = 0; fresh.unknown_len = 0;
        fresh.writable = false;
        qlog("index: page names could not be read; left unchanged");
    }
    bool links_ok = path_for(id, "links.ini", path, sizeof path) == 0 && read_sidecar(path, &text, &len) == 0 &&
                    (!text || each_line(text, len, link_line, &fresh) == 0);
    fresh.links_file = text != NULL;
    free(text);
    if (!links_ok) {
        free(fresh.links); free(fresh.links_unknown);
        fresh.links = NULL; fresh.links_unknown = NULL;
        fresh.nlinks = fresh.links_cap = 0; fresh.links_unknown_len = 0;
        fresh.links_writable = false;
        fresh.links_file = true;
        qlog("index: links could not be read; left unchanged");
    }
    nav_free(n); *n = fresh;
    return names_ok && links_ok ? 0 : -1;
}

static bool has_page(const NotebookInfo *nb, uint32_t pid) {
    for (int i = 0; i < nb->npages; i++) if (nb->page_ids[i] == pid) return true;
    return false;
}

int nav_save(const char *id, Nav *n, const NotebookInfo *nb) {
    if (!id || strcmp(id, nb->id) || !n->writable || nb->npages < 0 || nb->npages > STORE_MAX_PAGES) return -1;
    char path[700];
    if (path_for(id, "nav.ini", path, sizeof path) != 0) return -1;
    char *text = malloc(NAV_FILE_MAX + 1);
    if (!text) return -1;
    size_t len = (size_t)snprintf(text, NAV_FILE_MAX + 1, "# Grimoire: page names\n");
    for (int i = 0; i < n->count; i++) {
        if (!has_page(nb, n->names[i].pid)) continue;
        int wrote = snprintf(text + len, NAV_FILE_MAX + 1 - len, "name.%u=%s\n", n->names[i].pid, n->names[i].text);
        if (wrote < 0 || (size_t)wrote > NAV_FILE_MAX - len) { free(text); return -1; }
        len += (size_t)wrote;
    }
    if (n->unknown_len > NAV_FILE_MAX - len) { free(text); return -1; }
    if (n->unknown_len) memcpy(text + len, n->unknown, n->unknown_len);
    int rc = write_file_atomic(path, text, len + n->unknown_len);
    free(text);
    if (rc == 0) {
        int kept = 0;
        for (int i = 0; i < n->count; i++) if (has_page(nb, n->names[i].pid)) n->names[kept++] = n->names[i];
        n->count = kept;
    }
    return rc;
}

int nav_index(const Nav *n, const NotebookInfo *nb, int *indices, int cap) {
    int count = 0;
    for (int i = 0; i < nb->npages; i++) if (*nav_name(n, nb->page_ids[i])) {
        if (indices && count < cap) indices[count] = i;
        count++;
    }
    return count;
}

int nav_section_of(const Nav *n, const NotebookInfo *nb, int index) {
    for (int i = clampi(index, -1, nb->npages - 1); i >= 0; i--)
        if (*nav_name(n, nb->page_ids[i])) return i;
    return -1;
}

// ---- links ----

// Where (pid, seed) is or would go in n->links.
static int link_slot(const Nav *n, uint32_t pid, uint32_t seed) {
    int lo = 0, hi = n->nlinks;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;
        const NavLink *l = &n->links[mid];
        if (l->pid < pid || (l->pid == pid && l->seed < seed)) lo = mid + 1;
        else hi = mid;
    }
    return lo;
}

static int link_put(Nav *n, uint32_t pid, uint32_t seed, const char *target) {
    size_t tl = strlen(target);
    if (!pid || !tl || tl >= NAV_TARGET_LEN) return -1;
    int at = link_slot(n, pid, seed);
    if (!(at < n->nlinks && n->links[at].pid == pid && n->links[at].seed == seed)) {
        if (n->nlinks >= NAV_LINKS_MAX) return -1;
        if (n->nlinks == n->links_cap) {
            int cap = clampi(n->links_cap ? n->links_cap * 2 : 16, 16, NAV_LINKS_MAX);
            NavLink *p = realloc(n->links, (size_t)cap * sizeof *p);
            if (!p) return -1;
            n->links = p; n->links_cap = cap;
        }
        memmove(&n->links[at + 1], &n->links[at], (size_t)(n->nlinks - at) * sizeof *n->links);
        n->nlinks++;
        n->links[at].pid = pid;
        n->links[at].seed = seed;
    }
    memcpy(n->links[at].target, target, tl + 1);
    return 0;
}

// A page target's page id: "p<id>", the id from 1 to UINT32_MAX in decimal without leading zeros.
static uint32_t target_page(const char *t) {
    if (t[0] != 'p' || t[1] < '1' || t[1] > '9') return 0;
    uint64_t id = 0;
    for (const char *p = t + 1; *p; p++) {
        if (*p < '0' || *p > '9') return 0;
        id = id * 10 + (unsigned)(*p - '0');
        if (id > UINT32_MAX) return 0;
    }
    return (uint32_t)id;
}

int nav_link_set(Nav *n, uint32_t pid, uint32_t seed, const char *target) {
    if (!n->links_writable || !target || (strcmp(target, "index") && !target_page(target))) return -1;
    int first, count = nav_links_of(n, pid, &first);
    bool found = false;
    for (int i = first; i < first + count; i++) found |= n->links[i].seed == seed;
    if (!found && count >= NAV_LINKS_PER_PAGE) return -1;
    return link_put(n, pid, seed, target);
}

void nav_link_remove(Nav *n, uint32_t pid, uint32_t seed) {
    if (!n->links_writable) return;
    int at = link_slot(n, pid, seed);
    if (at >= n->nlinks || n->links[at].pid != pid || n->links[at].seed != seed) return;
    memmove(&n->links[at], &n->links[at + 1], (size_t)(n->nlinks - at - 1) * sizeof *n->links);
    n->nlinks--;
}

int nav_links_of(const Nav *n, uint32_t pid, int *first) {
    int at = link_slot(n, pid, 0), end = at;
    while (end < n->nlinks && n->links[end].pid == pid) end++;
    if (first) *first = at;
    return end - at;
}

static int page_index(const NotebookInfo *nb, uint32_t pid) {
    for (int i = 0; i < nb->npages; i++) if (nb->page_ids[i] == pid) return i;
    return -1;
}

int nav_link_page(const NavLink *l, const NotebookInfo *nb) {
    if (!strcmp(l->target, "index")) return NAV_TO_INDEX;
    uint32_t to = target_page(l->target);
    return to ? page_index(nb, to) : -1;
}

// The box of the points of page p's ink strokes with this seed, at least min_px a side around its
// centre, and the place in the page of the last of them (-1, and an empty box, for none).
static int link_box(const Page *p, uint32_t seed, float min_px, Rect *box) {
    float x0 = 0, y0 = 0, x1 = 0, y1 = 0;
    int last = -1;
    for (int i = 0; i < p->n; i++) {
        const Stroke *s = p->strokes[i];
        if (s->seed != seed || stroke_is_erase(s)) continue;
        for (int k = 0; k < s->n; k++) {
            float x = s->pts[k].x, y = s->pts[k].y;
            if (last < 0 && k == 0) { x0 = x1 = x; y0 = y1 = y; }
            if (x < x0) x0 = x;
            if (x > x1) x1 = x;
            if (y < y0) y0 = y;
            if (y > y1) y1 = y;
        }
        if (s->n) last = i;
    }
    if (last < 0) { *box = RECT_EMPTY; return -1; }
    float cx = (x0 + x1) * 0.5f, cy = (y0 + y1) * 0.5f;
    if (x1 - x0 < min_px) { x0 = cx - min_px * 0.5f; x1 = cx + min_px * 0.5f; }
    if (y1 - y0 < min_px) { y0 = cy - min_px * 0.5f; y1 = cy + min_px * 0.5f; }
    int ix = (int)floorf(x0), iy = (int)floorf(y0);
    *box = (Rect){ix, iy, (int)ceilf(x1) - ix, (int)ceilf(y1) - iy};
    return last;
}

int nav_link_areas(const Nav *n, const Page *p, uint32_t pid, float min_px, NavLink *links, Rect *areas, int cap) {
    int first, count = nav_links_of(n, pid, &first), wrote = 0;
    for (int i = first; i < first + count && wrote < cap; i++) {
        Rect box;
        if (link_box(p, n->links[i].seed, min_px, &box) < 0) continue;
        if (links) links[wrote] = n->links[i];
        if (areas) areas[wrote] = box;
        wrote++;
    }
    return wrote;
}

bool nav_link_at(const Nav *n, const Page *p, uint32_t pid, float x, float y, float min_px, NavLink *out) {
    int first, count = nav_links_of(n, pid, &first), best = -1, best_last = -1;
    for (int i = first; i < first + count; i++) {
        Rect box;
        int last = link_box(p, n->links[i].seed, min_px, &box);
        if (last <= best_last) continue;
        if (x >= (float)box.x && x < (float)(box.x + box.w) && y >= (float)box.y && y < (float)(box.y + box.h)) {
            best = i;
            best_last = last;
        }
    }
    if (best < 0) return false;
    if (out) *out = n->links[best];
    return true;
}

uint32_t nav_new_seed(const Nav *n, const Page *p, uint32_t pid, uint32_t want) {
    int first, count = nav_links_of(n, pid, &first);
    for (int tries = 0; tries < 64; tries++) {
        bool taken = false;
        for (int i = first; i < first + count && !taken; i++) taken = n->links[i].seed == want;
        for (int i = 0; i < p->n && !taken; i++)
            taken = p->strokes[i]->seed == want && !stroke_is_erase(p->strokes[i]);
        if (!taken) break;
        want = want * 2654435761u + 0x9e3779b9u;
    }
    return want;
}

// A link stays while its page is in the notebook and it goes to the Index, a page of it, or a place
// this version does not know.
static bool link_kept(const NavLink *l, const NotebookInfo *nb) {
    uint32_t to = target_page(l->target);
    return has_page(nb, l->pid) && (!to || has_page(nb, to));
}

int nav_save_links(const char *id, Nav *n, const NotebookInfo *nb) {
    if (!id || strcmp(id, nb->id) || !n->links_writable || nb->npages < 0 || nb->npages > STORE_MAX_PAGES) return -1;
    char path[700];
    if (path_for(id, "links.ini", path, sizeof path) != 0) return -1;
    int kept = 0;
    for (int i = 0; i < n->nlinks; i++) kept += link_kept(&n->links[i], nb);
    if (!n->links_file && !kept && !n->links_unknown_len) return 0;
    char *text = malloc(NAV_FILE_MAX + 1);
    if (!text) return -1;
    size_t len = (size_t)snprintf(text, NAV_FILE_MAX + 1, "# Grimoire: links drawn on pages\n");
    for (int i = 0; i < n->nlinks; i++) {
        const NavLink *l = &n->links[i];
        if (!link_kept(l, nb)) continue;
        int wrote = snprintf(text + len, NAV_FILE_MAX + 1 - len, "link.%u=%08x,%s\n", l->pid, l->seed, l->target);
        if (wrote < 0 || (size_t)wrote > NAV_FILE_MAX - len) { free(text); return -1; }
        len += (size_t)wrote;
    }
    if (n->links_unknown_len > NAV_FILE_MAX - len) { free(text); return -1; }
    if (n->links_unknown_len) memcpy(text + len, n->links_unknown, n->links_unknown_len);
    int rc = write_file_atomic(path, text, len + n->links_unknown_len);
    free(text);
    if (rc == 0) {
        n->links_file = true;
        kept = 0;
        for (int i = 0; i < n->nlinks; i++) if (link_kept(&n->links[i], nb)) n->links[kept++] = n->links[i];
        n->nlinks = kept;
    }
    return rc;
}
