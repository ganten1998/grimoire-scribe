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
extern unsigned char UnifrakturMaguntia_Book_ttf[];
extern unsigned int UnifrakturMaguntia_Book_ttf_len;
extern unsigned char IMFeENrm28P_ttf[];
extern unsigned int IMFeENrm28P_ttf_len;
extern unsigned char IMFeENsc28P_ttf[];
extern unsigned int IMFeENsc28P_ttf_len;

// The embedded TrueType files. Terminess (with the Nerd Font icons) is always there; the
// others belong to the gothic theme.
enum { TTF_TERM, TTF_TERM_BOLD, TTF_FRAKTUR, TTF_FELL, TTF_FELL_SC, TTF_COUNT };
static stbtt_fontinfo g_font[TTF_COUNT];
static bool g_ready;

// What a FontFace means under the current theme: which file, how much bigger than asked (the
// book faces have a smaller x-height than Terminess), and whether to embolden by overprinting.
typedef struct {
    uint8_t ttf;
    float scale;
    bool embolden;
} FaceMap;
static FaceMap g_map[FONT_FACE_COUNT];

static void map_faces(bool gothic) {
    static const FaceMap classic[FONT_FACE_COUNT] = {
        [FONT_REGULAR] = {TTF_TERM, 1.f, false},    [FONT_BOLD] = {TTF_TERM_BOLD, 1.f, false},
        [FONT_TITLE] = {TTF_TERM_BOLD, 1.f, false}, [FONT_CAPS] = {TTF_TERM_BOLD, 1.f, false},
        [FONT_FIXED] = {TTF_TERM, 1.f, false},      [FONT_FIXED_BOLD] = {TTF_TERM_BOLD, 1.f, false},
    };
    static const FaceMap goth[FONT_FACE_COUNT] = {
        [FONT_REGULAR] = {TTF_FELL, 1.16f, false},  [FONT_BOLD] = {TTF_FELL, 1.16f, true},
        [FONT_TITLE] = {TTF_FRAKTUR, 1.22f, false}, [FONT_CAPS] = {TTF_FELL_SC, 1.16f, true},
        [FONT_FIXED] = {TTF_TERM, 1.f, false},      [FONT_FIXED_BOLD] = {TTF_TERM_BOLD, 1.f, false},
    };
    memcpy(g_map, gothic ? goth : classic, sizeof g_map);
}

static bool g_gothic;

typedef struct {
    bool used;
    uint8_t face;       // TTF_* index
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
    const unsigned char *data[TTF_COUNT] = {TerminessNerdFontMono_Regular_ttf, TerminessNerdFontMono_Bold_ttf,
                                            UnifrakturMaguntia_Book_ttf, IMFeENrm28P_ttf, IMFeENsc28P_ttf};
    for (int i = 0; i < TTF_COUNT; i++)
        if (!stbtt_InitFont(&g_font[i], data[i], stbtt_GetFontOffsetForIndex(data[i], 0))) {
            qlog("font: init failed for face %d", i);
            return -1;
        }
    map_faces(g_gothic);
    g_ready = true;
    return 0;
}

void font_set_gothic(bool gothic) {
    g_gothic = gothic;
    map_faces(gothic);
}

bool font_gothic(void) { return g_gothic; }

void font_shutdown(void) {
    cache_flush();
    g_ready = false;
}

static const Glyph *glyph_get(int ttf, float px, uint32_t cp) {
    int size4 = (int)lrintf(px * 4.f);
    uint32_t hsh = (cp * 2654435761u) ^ ((uint32_t)size4 * 40503u) ^ ((uint32_t)ttf * 977u);
    for (int probe = 0; probe < 16; probe++) {
        Glyph *g = &g_cache[(hsh + (uint32_t)probe) % CACHE_SLOTS];
        if (g->used && g->cp == cp && g->size4 == size4 && g->face == ttf) return g;
        if (!g->used) {
            if (g_cache_used > CACHE_SLOTS * 3 / 4) {
                cache_flush();
                return glyph_get(ttf, px, cp);
            }
            const stbtt_fontinfo *fi = &g_font[ttf];
            float scale = stbtt_ScaleForPixelHeight(fi, size4 / 4.f);
            int adv, lsb;
            stbtt_GetCodepointHMetrics(fi, (int)cp, &adv, &lsb);
            g->bmp = stbtt_GetCodepointBitmap(fi, scale, scale, (int)cp, &g->w, &g->h, &g->xoff, &g->yoff);
            g->advance = adv;
            g->used = true;
            g->cp = cp;
            g->size4 = size4;
            g->face = (uint8_t)ttf;
            g_cache_used++;
            return g;
        }
    }
    cache_flush();
    return glyph_get(ttf, px, cp);
}

static FaceMap face_of(FontFace f) {
    if ((int)f < 0 || f >= FONT_FACE_COUNT) f = FONT_REGULAR;
    return g_map[f];
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

static void blit_glyph(Canvas *c, const Glyph *g, int gx, int gy, uint8_t gray) {
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

static int layout(Canvas *c, FontFace f, float px, int x, int baseline, const char *s, uint8_t gray) {
    if (!g_ready || !s) return 0;
    FaceMap m = face_of(f);
    float fpx = px * m.scale;
    // Overprinting a second copy this far to the right makes a book face read as bold on e-ink.
    int bold = m.embolden ? (int)fmaxf(1.f, lrintf(fpx / 34.f)) : 0;
    float pen = (float)x;
    uint32_t prev = 0;
    int prev_ttf = -1;
    while (*s) {
        uint32_t cp = utf8_next(&s);
        // Glyphs a book face lacks (symbols, arrows) come from Terminess at the asked size.
        int ttf = m.ttf;
        float gpx = fpx;
        if (ttf != TTF_TERM && ttf != TTF_TERM_BOLD && cp > 0x7F && !stbtt_FindGlyphIndex(&g_font[ttf], (int)cp)) {
            ttf = m.embolden ? TTF_TERM_BOLD : TTF_TERM;
            gpx = px;
        }
        const stbtt_fontinfo *fi = &g_font[ttf];
        float scale = stbtt_ScaleForPixelHeight(fi, gpx);
        if (prev && prev_ttf == ttf) pen += scale * (float)stbtt_GetCodepointKernAdvance(fi, (int)prev, (int)cp);
        const Glyph *g = glyph_get(ttf, gpx, cp);
        if (c && g->bmp) {
            int gx = (int)lrintf(pen) + g->xoff, gy = baseline + g->yoff;
            blit_glyph(c, g, gx, gy, gray);
            for (int b = 1; b <= bold; b++) blit_glyph(c, g, gx + b, gy, gray);
        }
        pen += scale * (float)g->advance + (float)bold;
        prev = cp;
        prev_ttf = ttf;
    }
    return (int)lrintf(pen) - x;
}

int font_draw(Canvas *c, FontFace f, float px, int x, int baseline_y, const char *utf8, uint8_t gray) {
    return layout(c, f, px, x, baseline_y, utf8, gray);
}

int font_measure(FontFace f, float px, const char *utf8) { return layout(NULL, f, px, 0, 0, utf8, 0); }

int font_ascent(FontFace f, float px) {
    if (!g_ready) return (int)px;
    FaceMap m = face_of(f);
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[m.ttf], &a, &d, &l);
    return (int)lrintf(a * stbtt_ScaleForPixelHeight(&g_font[m.ttf], px * m.scale));
}

int font_descent(FontFace f, float px) {
    if (!g_ready) return 0;
    FaceMap m = face_of(f);
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[m.ttf], &a, &d, &l);
    return (int)lrintf(-d * stbtt_ScaleForPixelHeight(&g_font[m.ttf], px * m.scale));
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
    // Icons are Terminess Nerd Font glyphs in every theme.
    int ttf = f == FONT_BOLD || f == FONT_FIXED_BOLD ? TTF_TERM_BOLD : TTF_TERM;
    const stbtt_fontinfo *fi = &g_font[ttf];
    int x0, y0, x1, y1;
    if (!stbtt_GetCodepointBox(fi, (int)cp, &x0, &y0, &x1, &y1) || x1 <= x0 || y1 <= y0) return;
    // Pick the pixel height whose scale fits the glyph's ink box into r.
    float unit = stbtt_ScaleForPixelHeight(fi, 100.f) / 100.f;  // scale per px of height
    float sx = (float)r.w / ((float)(x1 - x0) * unit), sy = (float)r.h / ((float)(y1 - y0) * unit);
    float px = floorf(fminf(sx, sy) * 4.f) / 4.f;
    const Glyph *g = glyph_get(ttf, px, cp);
    if (!g->bmp) return;
    blit_glyph(c, g, r.x + (r.w - g->w) / 2, r.y + (r.h - g->h) / 2, gray);
}
