#include "nav.h"
#include "i18n.h"
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

void nav_init(Nav *n) { memset(n, 0, sizeof *n); n->writable = true; }
void nav_free(Nav *n) { free(n->names); free(n->unknown); memset(n, 0, sizeof *n); }

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

static int path_for(const char *id, char *path, size_t cap) {
    char dir[600];
    if (store_notebook_dir(id, dir, sizeof dir) != 0) return -1;
    int len = snprintf(path, cap, "%s/nav.ini", dir);
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

int nav_load(const char *id, Nav *n) {
    Nav fresh; nav_init(&fresh);
    char path[700], *text = NULL;
    int fd = -1, rc = -1;
    if (path_for(id, path, sizeof path) != 0) goto done;
    fd = open(path, O_RDONLY | O_NONBLOCK | O_NOFOLLOW);
    if (fd < 0) {
        struct stat st;
        if (errno == ENOENT && lstat(path, &st) != 0 && errno == ENOENT) rc = 0;
        goto done;
    }
    struct stat st;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode) || st.st_size < 0 || st.st_size > NAV_FILE_MAX) goto done;
    text = malloc(NAV_FILE_MAX + 1);
    if (!text) goto done;
    size_t len = 0;
    for (;;) {
        ssize_t got = read(fd, text + len, NAV_FILE_MAX + 1 - len);
        if (got < 0) { if (errno == EINTR) continue; goto done; }
        if (!got) break;
        len += (size_t)got;
        if (len > NAV_FILE_MAX) goto done;
    }
    if (memchr(text, 0, len)) goto done;  // never rewrite a file with a hidden binary tail
    text[len] = 0;
    for (char *line = text; line < text + len;) {
        char *end = strchr(line, '\n');
        if (end) *end = 0;
        size_t size = strlen(line);
        if (size && line[size - 1] == '\r') line[--size] = 0;
        uint32_t pid; const char *value;
        if (name_key(line, &pid, &value)) {
            if (nav_set_name(&fresh, pid, value) != 0) goto done;
        } else if (size && line[0] != '#') {
            if (size > 255 || fresh.unknown_len + size + 1 > NAV_UNKNOWN_MAX) goto done;
            char *p = realloc(fresh.unknown, fresh.unknown_len + size + 1);
            if (!p) goto done;
            fresh.unknown = p;
            memcpy(p + fresh.unknown_len, line, size);
            fresh.unknown_len += size;
            p[fresh.unknown_len++] = '\n';
        }
        if (!end) break;
        line = end + 1;
    }
    rc = 0;
done:
    if (fd >= 0) close(fd);
    free(text);
    if (rc != 0) {
        nav_free(&fresh); nav_init(&fresh); fresh.writable = false;
        qlog("index: page names could not be read; left unchanged");
    }
    nav_free(n); *n = fresh;
    return rc;
}

static bool has_page(const NotebookInfo *nb, uint32_t pid) {
    for (int i = 0; i < nb->npages; i++) if (nb->page_ids[i] == pid) return true;
    return false;
}

int nav_save(const char *id, Nav *n, const NotebookInfo *nb) {
    if (!id || strcmp(id, nb->id) || !n->writable || nb->npages < 0 || nb->npages > STORE_MAX_PAGES) return -1;
    char path[700];
    if (path_for(id, path, sizeof path) != 0) return -1;
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
