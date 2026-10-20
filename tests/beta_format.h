// beta_format.h: the page reader and writer of the released Grimoire 1.0.0-beta, frozen, so tests
// can ask what 1.0.0-beta does with a file a later version writes. 1.0.0-rc.1 reads and writes
// pages the same way (its store.c, paper.c and brush.c are identical to 1.0.0-beta's).
//
// Copied from `git show v1.0.0-beta:src/store.c` (page_serialize, page_deserialize and what
// store_load_page does with a page it cannot read), src/paper.c (paper_sanitize and the custom
// music decoding it needs) and src/brush.c (the parameter ranges the reader clamps to), renamed
// beta_* and working on plain bytes and structs of their own, with no dependency on the current
// sources. Never update this file: 1.0.0-beta is in people's hands as it is.
#pragma once
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define BETA_PAPER_MUSIC 3u
#define BETA_PAPER_BLANK 4u
#define BETA_PAPER_COUNT 17u
#define BETA_PAPER_CUSTOM_FLAG 0x40000000u
#define BETA_PART_NONE 0
#define BETA_PART_COUNT 12
#define BETA_MUSIC_MAX_PARTS 6
#define BETA_MUSIC_MAX_SYSTEMS 12
#define BETA_TOOL_COUNT 10u

typedef struct {
    float x, y, pressure, tilt_x, tilt_y;
    uint32_t t_ms;
} BetaPoint;

typedef struct {
    uint8_t tool;
    uint32_t seed;
    float params[13];  // ToolParams field order
    uint32_t n;
    BetaPoint *pts;
} BetaStroke;

typedef struct {
    uint32_t paper;  // always a kind 1.0.0-beta knows (sanitized on read)
    uint32_t w, h;
    uint32_t n;
    BetaStroke *strokes;
} BetaPage;

static inline float beta_clampf(float v, float lo, float hi) { return v < lo ? lo : (v > hi ? hi : v); }

// src/brush.c k_params: {min, max} per ToolParams field, in field order.
static const float beta_param_range[13][2] = {
    {0.10f, 12.f}, {0.10f, 12.f}, {0.f, 1.f}, {0.f, 1.f},  {0.f, 0.5f}, {0.05f, 1.f}, {0.25f, 4.f},
    {0.f, 85.f},   {5.f, 90.f},   {0.f, 4.f}, {0.f, 1.f}, {0.f, 1.f},  {0.f, 1.f},
};

// ---- src/paper.c ----

static inline bool beta_paper_is_custom(uint32_t k) { return (k & 0xF0000000u) == BETA_PAPER_CUSTOM_FLAG; }

static inline bool beta_music_decode(uint32_t v) {
    if (!beta_paper_is_custom(v)) return false;
    int systems = (int)((v >> 24) & 15), n = 0;
    for (int i = 0; i < BETA_MUSIC_MAX_PARTS; i++) {
        int p = (int)((v >> (4 * i)) & 15);
        if (p == BETA_PART_NONE) break;
        if (p >= BETA_PART_COUNT) return false;
        n++;
    }
    return !(n == 0 || systems < 1 || systems > BETA_MUSIC_MAX_SYSTEMS);
}

static inline uint32_t beta_paper_sanitize(uint32_t v, uint32_t fallback) {
    if (v < BETA_PAPER_COUNT) return v;
    if (beta_music_decode(v)) return v;
    return fallback;
}

// ---- src/store.c: page format ----

typedef struct {
    uint8_t *d;
    size_t n, cap;
    bool err;
} BetaW;

static inline void beta_w_bytes(BetaW *w, const void *p, size_t n) {
    if (w->err) return;
    if (w->n + n > w->cap) {
        size_t nc = w->cap ? w->cap * 2 : 4096;
        while (nc < w->n + n) nc *= 2;
        uint8_t *nd = realloc(w->d, nc);
        if (!nd) {
            w->err = true;
            return;
        }
        w->d = nd;
        w->cap = nc;
    }
    memcpy(w->d + w->n, p, n);
    w->n += n;
}
static inline void beta_w_u8(BetaW *w, uint8_t v) { beta_w_bytes(w, &v, 1); }
static inline void beta_w_u16(BetaW *w, uint16_t v) {
    uint8_t b[2] = {(uint8_t)v, (uint8_t)(v >> 8)};
    beta_w_bytes(w, b, 2);
}
static inline void beta_w_u32(BetaW *w, uint32_t v) {
    uint8_t b[4] = {(uint8_t)v, (uint8_t)(v >> 8), (uint8_t)(v >> 16), (uint8_t)(v >> 24)};
    beta_w_bytes(w, b, 4);
}
static inline void beta_w_f32(BetaW *w, float f) {
    uint32_t u;
    memcpy(&u, &f, 4);
    beta_w_u32(w, u);
}

// page_serialize: version 2 only for custom music; the pad bytes are always written as 0, and
// nothing follows the last stroke.
static inline int beta_page_serialize(const BetaPage *p, uint8_t **out, size_t *len) {
    BetaW w = {0};
    bool custom = beta_paper_is_custom(p->paper);
    beta_w_bytes(&w, "QPG1", 4);
    beta_w_u16(&w, custom ? 2 : 1);
    beta_w_u16(&w, custom ? (uint16_t)BETA_PAPER_MUSIC : (uint16_t)p->paper);
    beta_w_u32(&w, p->w);
    beta_w_u32(&w, p->h);
    if (custom) beta_w_u32(&w, p->paper);
    beta_w_u32(&w, p->n);
    for (uint32_t i = 0; i < p->n; i++) {
        const BetaStroke *s = &p->strokes[i];
        beta_w_u8(&w, s->tool);
        beta_w_u8(&w, 0);
        beta_w_u8(&w, 0);
        beta_w_u8(&w, 0);
        beta_w_u32(&w, s->seed);
        for (int k = 0; k < 13; k++) beta_w_f32(&w, s->params[k]);
        beta_w_u32(&w, s->n);
        for (uint32_t k = 0; k < s->n; k++) {
            const BetaPoint *q = &s->pts[k];
            beta_w_f32(&w, q->x);
            beta_w_f32(&w, q->y);
            beta_w_u16(&w, (uint16_t)lrintf(beta_clampf(q->pressure, 0.f, 1.f) * 65535.f));
            beta_w_u16(&w, (uint16_t)(int16_t)lrintf(beta_clampf(q->tilt_x, -90.f, 90.f) * 100.f));
            beta_w_u16(&w, (uint16_t)(int16_t)lrintf(beta_clampf(q->tilt_y, -90.f, 90.f) * 100.f));
            beta_w_u32(&w, q->t_ms);
        }
    }
    if (w.err) {
        free(w.d);
        return -1;
    }
    *out = w.d;
    *len = w.n;
    return 0;
}

typedef struct {
    const uint8_t *d;
    size_t n, pos;
    bool err;
} BetaR;

static inline const uint8_t *beta_r_take(BetaR *r, size_t n) {
    if (r->err || r->n - r->pos < n) {
        r->err = true;
        return NULL;
    }
    const uint8_t *p = r->d + r->pos;
    r->pos += n;
    return p;
}
static inline uint8_t beta_r_u8(BetaR *r) {
    const uint8_t *p = beta_r_take(r, 1);
    return p ? p[0] : 0;
}
static inline uint16_t beta_r_u16(BetaR *r) {
    const uint8_t *p = beta_r_take(r, 2);
    return p ? (uint16_t)(p[0] | p[1] << 8) : 0;
}
static inline uint32_t beta_r_u32(BetaR *r) {
    const uint8_t *p = beta_r_take(r, 4);
    return p ? (uint32_t)p[0] | (uint32_t)p[1] << 8 | (uint32_t)p[2] << 16 | (uint32_t)p[3] << 24 : 0;
}
static inline float beta_r_f32(BetaR *r) {
    uint32_t u = beta_r_u32(r);
    float f;
    memcpy(&f, &u, 4);
    return isfinite(f) ? f : 0.f;
}

#define BETA_STROKE_HEADER_BYTES (4 + 4 + 13 * 4 + 4)
#define BETA_POINT_BYTES (4 + 4 + 2 + 2 + 2 + 4)

static inline void beta_page_free(BetaPage *p) {
    for (uint32_t i = 0; i < p->n; i++) free(p->strokes[i].pts);
    free(p->strokes);
    memset(p, 0, sizeof *p);
}

// page_deserialize: 0, or -1 when the page is rejected (bad magic, a version other than 1 or 2,
// impossible sizes, a tool id of 10 or more, or a cut-off file). The pad bytes and anything after
// the last stroke are not read; parameters are clamped to their ranges; the paper is sanitized.
static inline int beta_page_deserialize(const uint8_t *buf, size_t len, BetaPage *out) {
    memset(out, 0, sizeof *out);
    BetaR r = {buf, len, 0, false};
    const uint8_t *m = beta_r_take(&r, 4);
    if (!m || memcmp(m, "QPG1", 4) != 0) return -1;
    uint16_t ver = beta_r_u16(&r);
    uint32_t paper = beta_r_u16(&r);
    uint32_t w = beta_r_u32(&r), h = beta_r_u32(&r);
    if (ver == 2) paper = beta_r_u32(&r);
    uint32_t ns = beta_r_u32(&r);
    if (r.err || (ver != 1 && ver != 2) || w == 0 || h == 0 || w > 20000 || h > 20000) return -1;
    if (ns > (len - r.pos) / BETA_STROKE_HEADER_BYTES) return -1;
    BetaPage p = {beta_paper_sanitize(paper, BETA_PAPER_BLANK), w, h, 0, NULL};
    p.strokes = calloc(ns ? ns : 1, sizeof *p.strokes);
    if (!p.strokes) return -1;
    for (uint32_t i = 0; i < ns && !r.err; i++) {
        uint8_t tool = beta_r_u8(&r);
        beta_r_take(&r, 3);
        uint32_t seed = beta_r_u32(&r);
        float prm[13];
        for (int k = 0; k < 13; k++) prm[k] = beta_r_f32(&r);
        uint32_t np = beta_r_u32(&r);
        if (r.err || tool >= BETA_TOOL_COUNT || np > (len - r.pos) / BETA_POINT_BYTES) {
            r.err = true;
            break;
        }
        BetaStroke *s = &p.strokes[p.n++];
        s->tool = tool;
        s->seed = seed;
        for (int k = 0; k < 13; k++) s->params[k] = beta_clampf(prm[k], beta_param_range[k][0], beta_param_range[k][1]);
        s->pts = calloc(np ? np : 1, sizeof *s->pts);
        if (!s->pts) {
            r.err = true;
            break;
        }
        for (uint32_t k = 0; k < np; k++) {
            BetaPoint q;
            q.x = beta_clampf(beta_r_f32(&r), -10000.f, 30000.f);
            q.y = beta_clampf(beta_r_f32(&r), -10000.f, 30000.f);
            q.pressure = (float)beta_r_u16(&r) / 65535.f;
            q.tilt_x = (float)(int16_t)beta_r_u16(&r) / 100.f;
            q.tilt_y = (float)(int16_t)beta_r_u16(&r) / 100.f;
            q.t_ms = beta_r_u32(&r);
            s->pts[s->n++] = q;
        }
    }
    if (r.err) {
        beta_page_free(&p);
        return -1;
    }
    *out = p;
    return 0;
}

// What 1.0.0-beta writes back after opening a page and saving it (its autosave after any edit):
// read, then written. -1 if it rejects the page.
static inline int beta_resave(const uint8_t *buf, size_t len, uint8_t **out, size_t *outlen) {
    BetaPage p;
    if (beta_page_deserialize(buf, len, &p) != 0) return -1;
    int rc = beta_page_serialize(&p, out, outlen);
    beta_page_free(&p);
    return rc;
}

// What store_load_page does with a page file it cannot read: it renames it to
// <path>.damaged-<time> (and shows a blank page in its place). Returns the reader's result.
static inline int beta_load_page_file(const char *path, long long now) {
    FILE *f = fopen(path, "rb");
    if (!f) return 0;  // a missing page is a blank page, nothing set aside
    uint8_t *buf = NULL;
    size_t len = 0, cap = 0;
    for (;;) {
        if (len == cap) {
            cap = cap ? cap * 2 : 65536;
            uint8_t *nb = realloc(buf, cap);
            if (!nb) break;
            buf = nb;
        }
        size_t got = fread(buf + len, 1, cap - len, f);
        len += got;
        if (got == 0) break;
    }
    fclose(f);
    BetaPage p;
    int rc = buf ? beta_page_deserialize(buf, len, &p) : -1;
    if (rc == 0) beta_page_free(&p);
    else {
        char aside[1200];
        snprintf(aside, sizeof aside, "%s.damaged-%lld", path, now);
        rename(path, aside);
    }
    free(buf);
    return rc;
}
