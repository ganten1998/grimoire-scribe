// common.h: small shared helpers. Implemented in util.c.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define APP_NAME "Quire"
#ifndef QUIRE_VERSION
#define QUIRE_VERSION "dev"
#endif

typedef struct {
    int x, y, w, h;
} Rect;

#define RECT_EMPTY ((Rect){0, 0, 0, 0})

static inline bool rect_is_empty(Rect r) { return r.w <= 0 || r.h <= 0; }
Rect rect_union(Rect a, Rect b);       // empty operands are ignored
Rect rect_intersect(Rect a, Rect b);   // RECT_EMPTY when disjoint
Rect rect_inflate(Rect r, int by);     // grow on all sides (negative shrinks)
bool rect_contains(Rect r, int x, int y);
bool rect_contains_rect(Rect outer, Rect inner);  // an empty inner is contained
bool rect_overlaps(Rect a, Rect b);
Rect rect_from_points(float x0, float y0, float x1, float y1, float pad); // bbox, floor/ceil, padded

static inline float clampf(float v, float lo, float hi) { return v < lo ? lo : (v > hi ? hi : v); }
static inline int clampi(int v, int lo, int hi) { return v < lo ? lo : (v > hi ? hi : v); }
static inline float lerpf(float a, float b, float t) { return a + (b - a) * t; }
static inline float mm_to_px(float mm, float dpi) { return mm * dpi / 25.4f; }

uint32_t now_ms(void);      // monotonic milliseconds
int64_t wall_time(void);    // unix seconds
void sleep_ms(int ms);

// Logging goes to stderr; on Kindle the launcher redirects it to root/quire.log.
void qlog(const char *fmt, ...) __attribute__((format(printf, 1, 2)));

// Bounded string copy that always terminates. Returns strlen(dst).
size_t strlcpy_q(char *dst, const char *src, size_t cap);
// Format a unix time as "Oct 10, 14:32" (local time) into buf.
void format_time_short(int64_t t, char *buf, size_t cap);
// Path helpers (POSIX). mkdir -p; returns 0 on success.
int mkdir_p(const char *path);
// Atomic file write: path.tmp, fsync, rename. Returns 0 on success.
int write_file_atomic(const char *path, const void *data, size_t len);
// Reads a whole file; caller frees. Returns NULL on failure. *len set on success.
void *read_file(const char *path, size_t *len);
