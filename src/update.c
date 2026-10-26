// update.c: see update.h.
#include "update.h"

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

#include "common.h"

// ---- pure parts ----

static bool number(const char **p, uint32_t *out, uint32_t max) {
    const char *s = *p;
    if (*s < '0' || *s > '9') return false;
    uint64_t v = 0;
    while (*s >= '0' && *s <= '9') {
        v = v * 10 + (uint64_t)(*s++ - '0');
        if (v > max) return false;
    }
    *out = (uint32_t)v;
    *p = s;
    return true;
}

bool update_version_key(const char *version, uint32_t key[3]) {
    if (!version) return false;
    const char *p = version;
    uint32_t major, minor, patch, stage = 9999;
    if (!number(&p, &major, 9999) || *p++ != '.' || !number(&p, &minor, 9999) || *p++ != '.' ||
        !number(&p, &patch, 9999))
        return false;
    if (*p == '-') {
        p++;
        uint32_t n;
        if (!strncmp(p, "beta.", 5)) stage = 1000, p += 5;
        else if (!strncmp(p, "rc.", 3)) stage = 5000, p += 3;
        else return false;
        if (*p == '0' || !number(&p, &n, 999) || n < 1) return false;
        stage += n;
    }
    if (*p) return false;
    key[0] = major;
    key[1] = minor;
    key[2] = patch * 10000 + stage;
    return true;
}

int update_version_cmp(const char *a, const char *b) {
    uint32_t ka[3], kb[3];
    bool ra = update_version_key(a, ka), rb = update_version_key(b, kb);
    if (!ra || !rb) return (int)ra - (int)rb;
    for (int i = 0; i < 3; i++)
        if (ka[i] != kb[i]) return ka[i] < kb[i] ? -1 : 1;
    return 0;
}

bool update_is_prerelease(const char *version) { return version && strchr(version, '-') != NULL; }

bool update_latest(const char *out, char version[UPDATE_VERSION_LEN]) {
    version[0] = 0;
    if (!out) return false;
    bool found = false;
    for (const char *line = out; *line;) {
        const char *end = strchr(line, '\n');
        size_t len = end ? (size_t)(end - line) : strlen(line);
        char buf[512];
        // The line without colour codes (ESC [ ... letter).
        size_t n = 0;
        for (size_t i = 0; i < len && n + 1 < sizeof buf; i++) {
            if (line[i] == 0x1b && i + 1 < len && line[i + 1] == '[') {
                i += 2;
                while (i < len && !((line[i] >= 'A' && line[i] <= 'Z') || (line[i] >= 'a' && line[i] <= 'z'))) i++;
                continue;
            }
            buf[n++] = line[i];
        }
        buf[n] = 0;
        while (n && (buf[n - 1] == '\r' || buf[n - 1] == ' ')) buf[--n] = 0;
        const char *at = strstr(buf, "- grimoire (");
        if (at && strstr(at, "):")) {
            const char *last = strrchr(buf, ' ');
            const char *v = last ? last + 1 : NULL;
            uint32_t key[3];
            if (v && strlen(v) < UPDATE_VERSION_LEN && update_version_key(v, key) &&
                (!found || update_version_cmp(v, version) > 0)) {
                snprintf(version, UPDATE_VERSION_LEN, "%s", v);
                found = true;
            }
        }
        if (!end) break;
        line = end + 1;
    }
    return found;
}

// ---- on the Kindle ----

#define OUT_MAX (64 * 1024)

static char g_script[512], g_request[512];
static pid_t g_pid;
static int g_fd = -1;
static char g_out[OUT_MAX + 1];
static size_t g_len;

void update_init(const char *script, const char *request) {
    snprintf(g_script, sizeof g_script, "%s", script ? script : "");
    snprintf(g_request, sizeof g_request, "%s", request && *request ? request : "/tmp/grimoire-update");
}

const char *update_request_path(void) { return g_request; }

bool update_available(void) {
    struct stat st;
    return g_script[0] && stat(g_script, &st) == 0 && S_ISREG(st.st_mode);
}

bool update_checking(void) { return g_pid > 0; }

bool update_check(bool preview) {
    if (!update_available() || g_pid > 0) return false;
    int p[2];
    if (pipe(p) != 0) return false;
    pid_t pid = fork();
    if (pid < 0) {
        close(p[0]);
        close(p[1]);
        return false;
    }
    if (pid == 0) {
        dup2(p[1], 1);
        dup2(p[1], 2);
        for (int fd = 3; fd < 1024; fd++) close(fd);
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_handler = SIG_DFL;
        sigaction(SIGTERM, &sa, NULL);
        sigaction(SIGINT, &sa, NULL);
        sigaction(SIGHUP, &sa, NULL);
        sigaction(SIGPIPE, &sa, NULL);
        execl("/bin/sh", "sh", g_script, "check", preview ? "preview" : "stable", (char *)NULL);
        _exit(127);
    }
    close(p[1]);
    fcntl(p[0], F_SETFL, fcntl(p[0], F_GETFL, 0) | O_NONBLOCK);
    g_pid = pid;
    g_fd = p[0];
    g_len = 0;
    qlog("update: checking the %s channel", preview ? "preview" : "stable");
    return true;
}

static void drain(void) {
    for (ssize_t r; g_len < OUT_MAX && (r = read(g_fd, g_out + g_len, OUT_MAX - g_len)) > 0;) g_len += (size_t)r;
}

UpdateEvent update_tick(char latest[UPDATE_VERSION_LEN], char *why, size_t why_cap) {
    if (g_pid <= 0) return UPDATE_EV_NONE;
    drain();
    int status;
    if (waitpid(g_pid, &status, WNOHANG) != g_pid) return UPDATE_EV_NONE;
    drain();
    g_out[g_len] = 0;
    close(g_fd);
    g_fd = -1;
    g_pid = 0;
    int code = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    if (code != 0) {
        // update.sh's codes: 3 no KPM, 4 the feed could not be added, 5 the index could not be
        // refreshed (no network, mostly).
        snprintf(why, why_cap, "%s", code == 3 ? "no-kpm" : code == 4 ? "feed" : code == 5 ? "network" : "failed");
        qlog("update: the check did not work (%s, status %d)", why, code);
        return UPDATE_EV_FAILED;
    }
    update_latest(g_out, latest);
    qlog("update: the channel offers %s", latest[0] ? latest : "no Grimoire");
    return UPDATE_EV_DONE;
}

int update_request(bool preview) {
    const char *what = preview ? "preview\n" : "stable\n";
    return write_file_atomic(g_request, what, strlen(what));
}
