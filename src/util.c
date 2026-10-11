// util.c: implementation of common.h.
#include "common.h"

#include <errno.h>
#include <fcntl.h>
#include <libgen.h>
#include <math.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

Rect rect_union(Rect a, Rect b) {
    if (rect_is_empty(a)) return b;
    if (rect_is_empty(b)) return a;
    int x0 = a.x < b.x ? a.x : b.x;
    int y0 = a.y < b.y ? a.y : b.y;
    int x1 = (a.x + a.w) > (b.x + b.w) ? (a.x + a.w) : (b.x + b.w);
    int y1 = (a.y + a.h) > (b.y + b.h) ? (a.y + a.h) : (b.y + b.h);
    return (Rect){x0, y0, x1 - x0, y1 - y0};
}

Rect rect_intersect(Rect a, Rect b) {
    int x0 = a.x > b.x ? a.x : b.x;
    int y0 = a.y > b.y ? a.y : b.y;
    int x1 = (a.x + a.w) < (b.x + b.w) ? (a.x + a.w) : (b.x + b.w);
    int y1 = (a.y + a.h) < (b.y + b.h) ? (a.y + a.h) : (b.y + b.h);
    if (x1 <= x0 || y1 <= y0) return RECT_EMPTY;
    return (Rect){x0, y0, x1 - x0, y1 - y0};
}

Rect rect_inflate(Rect r, int by) {
    r.x -= by;
    r.y -= by;
    r.w += 2 * by;
    r.h += 2 * by;
    if (r.w < 0) r.w = 0;
    if (r.h < 0) r.h = 0;
    return r;
}

bool rect_contains(Rect r, int x, int y) {
    return x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h;
}

bool rect_overlaps(Rect a, Rect b) { return !rect_is_empty(rect_intersect(a, b)); }

Rect rect_from_points(float x0, float y0, float x1, float y1, float pad) {
    float lx = (x0 < x1 ? x0 : x1) - pad, hx = (x0 > x1 ? x0 : x1) + pad;
    float ly = (y0 < y1 ? y0 : y1) - pad, hy = (y0 > y1 ? y0 : y1) + pad;
    int ix = (int)floorf(lx), iy = (int)floorf(ly);
    return (Rect){ix, iy, (int)ceilf(hx) - ix + 1, (int)ceilf(hy) - iy + 1};
}

uint32_t now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint32_t)((uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u);
}

int64_t wall_time(void) { return (int64_t)time(NULL); }

void sleep_ms(int ms) {
    if (ms <= 0) return;
    struct timespec ts = {ms / 1000, (long)(ms % 1000) * 1000000L};
    while (nanosleep(&ts, &ts) == -1 && errno == EINTR) {
    }
}

void qlog(const char *fmt, ...) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    struct tm tm;
    time_t t = ts.tv_sec;
    localtime_r(&t, &tm);
    char line[1024];
    int n = snprintf(line, sizeof line, "%02d:%02d:%02d.%03ld ", tm.tm_hour, tm.tm_min, tm.tm_sec,
                     ts.tv_nsec / 1000000L);
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(line + n, sizeof line - (size_t)n, fmt, ap);
    va_end(ap);
    fprintf(stderr, "%s\n", line);
}

size_t strlcpy_q(char *dst, const char *src, size_t cap) {
    if (!cap) return 0;
    size_t n = 0;
    if (src)
        while (src[n] && n + 1 < cap) {
            dst[n] = src[n];
            n++;
        }
    dst[n] = 0;
    return n;
}

void format_time_short(int64_t t, char *buf, size_t cap) {
    static const char *mon[] = {"Jan", "Feb", "Mar", "Apr", "May", "Jun",
                                "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"};
    struct tm tm;
    time_t tt = (time_t)t;
    localtime_r(&tt, &tm);
    snprintf(buf, cap, "%s %d, %02d:%02d", mon[tm.tm_mon % 12], tm.tm_mday, tm.tm_hour, tm.tm_min);
}

int mkdir_p(const char *path) {
    char tmp[1024];
    if (strlcpy_q(tmp, path, sizeof tmp) >= sizeof tmp - 1) return -1;
    for (char *p = tmp + 1; *p; p++) {
        if (*p != '/') continue;
        *p = 0;
        if (mkdir(tmp, 0755) != 0 && errno != EEXIST) return -1;
        *p = '/';
    }
    if (mkdir(tmp, 0755) != 0 && errno != EEXIST) return -1;
    return 0;
}

int write_file_atomic(const char *path, const void *data, size_t len) {
    char tmp[1100];
    snprintf(tmp, sizeof tmp, "%s.tmp", path);
    int fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        qlog("write %s: %s", tmp, strerror(errno));
        return -1;
    }
    const uint8_t *p = data;
    size_t left = len;
    while (left) {
        ssize_t w = write(fd, p, left);
        if (w < 0) {
            if (errno == EINTR) continue;
            qlog("write %s: %s", tmp, strerror(errno));
            close(fd);
            unlink(tmp);
            return -1;
        }
        p += w;
        left -= (size_t)w;
    }
    fsync(fd);
    close(fd);
    if (rename(tmp, path) != 0) {
        qlog("rename %s: %s", path, strerror(errno));
        unlink(tmp);
        return -1;
    }
    // Best effort: make the rename itself durable.
    char dir[1100];
    strlcpy_q(dir, path, sizeof dir);
    int dfd = open(dirname(dir), O_RDONLY);
    if (dfd >= 0) {
        fsync(dfd);
        close(dfd);
    }
    return 0;
}

void *read_file(const char *path, size_t *len) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    size_t cap = 4096, n = 0;
    uint8_t *buf = malloc(cap + 1);
    while (buf) {
        size_t r = fread(buf + n, 1, cap - n, f);
        n += r;
        if (n < cap) break;
        cap *= 2;
        uint8_t *nb = realloc(buf, cap + 1);
        if (!nb) {
            free(buf);
            buf = NULL;
        }
        buf = nb;
    }
    fclose(f);
    if (!buf) return NULL;
    buf[n] = 0;
    if (len) *len = n;
    return buf;
}
