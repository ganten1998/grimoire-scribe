// font.c: TrueType text through stb_truetype with a small glyph cache.
#include "font.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "stb_truetype.h"

extern unsigned char TerminessNerdFontMono_Regular_ttf[];
extern unsigned int TerminessNerdFontMono_Regular_ttf_len;
extern unsigned char TerminessNerdFontMono_Bold_ttf[];
extern unsigned int TerminessNerdFontMono_Bold_ttf_len;

static stbtt_fontinfo g_font[2];
static bool g_ready;

typedef struct {
    bool used;
    uint8_t face;
    int size4;          // size * 4, rounded
    uint32_t cp;
    int w, h, xoff, yoff;
    int advance;        // unscaled advance (font units)
    uint8_t *bmp;
} Glyph;

#define CACHE_SLOTS 1024
static Glyph g_cache[CACHE_SLOTS];
static int g_cache_used;

static void cache_flush(void) {
    for (int i = 0; i < CACHE_SLOTS; i++) {
        if (g_cache[i].used) stbtt_FreeBitmap(g_cache[i].bmp, NULL);
        g_cache[i] = (Glyph){0};
    }
    g_cache_used = 0;
}

int font_init(void) {
    if (g_ready) return 0;
    const unsigned char *data[2] = {TerminessNerdFontMono_Regular_ttf, TerminessNerdFontMono_Bold_ttf};
    for (int i = 0; i < 2; i++)
        if (!stbtt_InitFont(&g_font[i], data[i], stbtt_GetFontOffsetForIndex(data[i], 0))) {
            qlog("font: init failed for face %d", i);
            return -1;
        }
    g_ready = true;
    return 0;
}

void font_shutdown(void) {
    cache_flush();
    g_ready = false;
}

static const Glyph *glyph_get(FontFace f, float px, uint32_t cp) {
    int size4 = (int)lrintf(px * 4.f);
    uint32_t hsh = (cp * 2654435761u) ^ ((uint32_t)size4 * 40503u) ^ ((uint32_t)f * 977u);
    for (int probe = 0; probe < 16; probe++) {
        Glyph *g = &g_cache[(hsh + (uint32_t)probe) % CACHE_SLOTS];
        if (g->used && g->cp == cp && g->size4 == size4 && g->face == f) return g;
        if (!g->used) {
            if (g_cache_used > CACHE_SLOTS * 3 / 4) {
                cache_flush();
                return glyph_get(f, px, cp);
            }
            const stbtt_fontinfo *fi = &g_font[f];
            float scale = stbtt_ScaleForPixelHeight(fi, size4 / 4.f);
            int adv, lsb;
            stbtt_GetCodepointHMetrics(fi, (int)cp, &adv, &lsb);
            g->bmp = stbtt_GetCodepointBitmap(fi, scale, scale, (int)cp, &g->w, &g->h, &g->xoff, &g->yoff);
            g->advance = adv;
            g->used = true;
            g->cp = cp;
            g->size4 = size4;
            g->face = (uint8_t)f;
            g_cache_used++;
            return g;
        }
    }
    cache_flush();
    return glyph_get(f, px, cp);
}

// Decodes one UTF-8 code point; advances *s. Invalid bytes become U+FFFD.
static uint32_t utf8_next(const char **s) {
    const unsigned char *p = (const unsigned char *)*s;
    uint32_t c = *p++;
    int extra = 0;
    if (c >= 0xF0 && c < 0xF8) {
        c &= 0x07;
        extra = 3;
    } else if (c >= 0xE0) {
        c &= 0x0F;
        extra = 2;
    } else if (c >= 0xC0) {
        c &= 0x1F;
        extra = 1;
    } else if (c >= 0x80) {
        *s = (const char *)p;
        return 0xFFFD;
    }
    while (extra--) {
        if ((*p & 0xC0) != 0x80) {
            *s = (const char *)p;
            return 0xFFFD;
        }
        c = (c << 6) | (*p++ & 0x3F);
    }
    *s = (const char *)p;
    return c;
}

static int layout(Canvas *c, FontFace f, float px, int x, int baseline, const char *s, uint8_t gray) {
    if (!g_ready || !s) return 0;
    const stbtt_fontinfo *fi = &g_font[f];
    float scale = stbtt_ScaleForPixelHeight(fi, px);
    float pen = (float)x;
    uint32_t prev = 0;
    while (*s) {
        uint32_t cp = utf8_next(&s);
        if (prev) pen += scale * (float)stbtt_GetCodepointKernAdvance(fi, (int)prev, (int)cp);
        const Glyph *g = glyph_get(f, px, cp);
        if (c && g->bmp) {
            int gx = (int)lrintf(pen) + g->xoff, gy = baseline + g->yoff;
            Rect gr = rect_intersect((Rect){gx, gy, g->w, g->h}, c->clip);
            for (int yy = gr.y; yy < gr.y + gr.h; yy++) {
                const uint8_t *src = &g->bmp[(yy - gy) * g->w];
                uint8_t *dst = &c->px[yy * c->stride];
                for (int xx = gr.x; xx < gr.x + gr.w; xx++) {
                    int a = src[xx - gx];
                    if (a) dst[xx] = (uint8_t)(dst[xx] + ((gray - dst[xx]) * a + 127) / 255);
                }
            }
        }
        pen += scale * (float)g->advance;
        prev = cp;
    }
    return (int)lrintf(pen) - x;
}

int font_draw(Canvas *c, FontFace f, float px, int x, int baseline_y, const char *utf8, uint8_t gray) {
    return layout(c, f, px, x, baseline_y, utf8, gray);
}

int font_measure(FontFace f, float px, const char *utf8) { return layout(NULL, f, px, 0, 0, utf8, 0); }

int font_ascent(FontFace f, float px) {
    if (!g_ready) return (int)px;
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[f], &a, &d, &l);
    return (int)lrintf(a * stbtt_ScaleForPixelHeight(&g_font[f], px));
}

int font_descent(FontFace f, float px) {
    if (!g_ready) return 0;
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[f], &a, &d, &l);
    return (int)lrintf(-d * stbtt_ScaleForPixelHeight(&g_font[f], px));
}

// Copies text into buf, truncated with "..." so it measures <= maxw.
static const char *fit(FontFace f, float px, const char *s, int maxw, char *buf, size_t cap) {
    if (font_measure(f, px, s) <= maxw) return s;
    size_t n = strlcpy_q(buf, s, cap - 4);
    while (n > 0) {
        // Step back over a whole UTF-8 sequence.
        do n--;
        while (n > 0 && ((unsigned char)buf[n] & 0xC0) == 0x80);
        memcpy(buf + n, "...", 4);
        if (font_measure(f, px, buf) <= maxw) return buf;
    }
    return "";
}

void font_draw_centered(Canvas *c, FontFace f, float px, Rect r, const char *utf8, uint8_t gray) {
    char buf[256];
    const char *s = fit(f, px, utf8, r.w, buf, sizeof buf);
    int w = font_measure(f, px, s);
    int asc = font_ascent(f, px), desc = font_descent(f, px);
    int base = r.y + (r.h - (asc + desc)) / 2 + asc;
    font_draw(c, f, px, r.x + (r.w - w) / 2, base, s, gray);
}

void font_draw_left(Canvas *c, FontFace f, float px, Rect r, const char *utf8, uint8_t gray) {
    char buf[256];
    const char *s = fit(f, px, utf8, r.w, buf, sizeof buf);
    int asc = font_ascent(f, px), desc = font_descent(f, px);
    int base = r.y + (r.h - (asc + desc)) / 2 + asc;
    font_draw(c, f, px, r.x, base, s, gray);
}

void font_draw_icon(Canvas *c, FontFace f, uint32_t cp, Rect r, uint8_t gray) {
    if (!g_ready || rect_is_empty(r)) return;
    const stbtt_fontinfo *fi = &g_font[f];
    int x0, y0, x1, y1;
    if (!stbtt_GetCodepointBox(fi, (int)cp, &x0, &y0, &x1, &y1) || x1 <= x0 || y1 <= y0) return;
    // Pick the pixel height whose scale fits the glyph's ink box into r.
    float unit = stbtt_ScaleForPixelHeight(fi, 100.f) / 100.f;  // scale per px of height
    float sx = (float)r.w / ((float)(x1 - x0) * unit), sy = (float)r.h / ((float)(y1 - y0) * unit);
    float px = floorf(fminf(sx, sy) * 4.f) / 4.f;
    const Glyph *g = glyph_get(f, px, cp);
    if (!g->bmp) return;
    int gx = r.x + (r.w - g->w) / 2, gy = r.y + (r.h - g->h) / 2;
    Rect gr = rect_intersect((Rect){gx, gy, g->w, g->h}, c->clip);
    for (int yy = gr.y; yy < gr.y + gr.h; yy++) {
        const uint8_t *src = &g->bmp[(yy - gy) * g->w];
        uint8_t *dst = &c->px[yy * c->stride];
        for (int xx = gr.x; xx < gr.x + gr.w; xx++) {
            int a = src[xx - gx];
            if (a) dst[xx] = (uint8_t)(dst[xx] + ((gray - dst[xx]) * a + 127) / 255);
        }
    }
}
