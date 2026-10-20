// font.c: TrueType text through stb_truetype with a small glyph cache. Pixel fonts (drawn on
// a grid of `unit` font units) are rendered one bitmap pixel per design pixel, without
// smoothing, then blown up to whole k x k blocks, so they stay razor sharp at any size.
#include "font.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "stb_truetype.h"

extern unsigned char TerminessNerdFontMono_Regular_ttf[];
extern unsigned int TerminessNerdFontMono_Regular_ttf_len;
extern unsigned char TerminessNerdFontMono_Bold_ttf[];
extern unsigned int TerminessNerdFontMono_Bold_ttf_len;
extern unsigned char Jacquard24_Regular_ttf[];
extern unsigned int Jacquard24_Regular_ttf_len;
extern unsigned char Jersey10_Regular_ttf[];
extern unsigned int Jersey10_Regular_ttf_len;

// The embedded TrueType files. Terminess (with the Nerd Font icons) is always there; the
// pixel faces belong to the gothic theme: Jacquard 24 (pixel blackletter) for titles, Jersey
// 10 for text and labels.
enum { TTF_TERM, TTF_TERM_BOLD, TTF_JACQUARD, TTF_JERSEY, TTF_COUNT };
static stbtt_fontinfo g_font[TTF_COUNT];
static bool g_ready;
// Font units per design pixel (0 = an outline font, drawn smooth). Measured: every outline
// point of these files sits on this grid.
static const int k_unit[TTF_COUNT] = {[TTF_JACQUARD] = 30, [TTF_JERSEY] = 75};

// What a FontFace means under the current theme: which file, how much bigger than asked, and
// whether to embolden by overprinting (one design pixel for pixel faces).
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
        [FONT_REGULAR] = {TTF_JERSEY, 1.15f, false},  [FONT_BOLD] = {TTF_JERSEY, 1.15f, true},
        [FONT_TITLE] = {TTF_JACQUARD, 1.1f, false},   [FONT_CAPS] = {TTF_JERSEY, 1.15f, true},
        [FONT_FIXED] = {TTF_TERM, 1.f, false},        [FONT_FIXED_BOLD] = {TTF_TERM_BOLD, 1.f, false},
    };
    memcpy(g_map, gothic ? goth : classic, sizeof g_map);
}

// Pixel faces: how many screen pixels one design pixel becomes at a requested line height.
static int pixel_k(int ttf, float px) {
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[ttf], &a, &d, &l);
    float line = (float)(a - d) / (float)k_unit[ttf];
    int k = (int)lrintf(px / line);
    return k < 1 ? 1 : k;
}

static bool g_gothic;

typedef struct {
    bool used;
    uint8_t face;       // TTF_* index
    int size4;          // size * 4, rounded; pixel faces: -k
    uint32_t cp;
    int w, h, xoff, yoff;
    int advance;        // unscaled advance (font units)
    uint8_t *bmp;
    uint8_t *bold;      // pixel glyphs above U+007F: the bold weight, w + k wide (see pixel_bold)
} Glyph;

#define CACHE_SLOTS 1024
static Glyph g_cache[CACHE_SLOTS];
static int g_cache_used;

static void cache_flush(void) {
    for (int i = 0; i < CACHE_SLOTS; i++) {
        if (g_cache[i].used) {
            stbtt_FreeBitmap(g_cache[i].bmp, NULL);
            free(g_cache[i].bold);
        }
        g_cache[i] = (Glyph){0};
    }
    g_cache_used = 0;
}

int font_init(void) {
    if (g_ready) return 0;
    const unsigned char *data[TTF_COUNT] = {TerminessNerdFontMono_Regular_ttf, TerminessNerdFontMono_Bold_ttf,
                                            Jacquard24_Regular_ttf, Jersey10_Regular_ttf};
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

// Design pixels a pixel face's glyph is set below where the font has it. Jersey 10 draws the
// inverted exclamation mark exactly as its i, a dot over a stem standing on the baseline, so a
// Spanish exclamation would open with what reads as an i ("iHola!"). Lowered by the depth of
// the face's descenders (g j p q y reach two design pixels below the baseline), it hangs as type
// usually sets it: the dot at the x-height, the stem below the baseline.
static int pixel_drop(int ttf, uint32_t cp) { return ttf == TTF_JERSEY && cp == 0xA1 ? 2 : 0; }

// A pixel face's glyph at k screen pixels per design pixel: hard-edged k x k blocks.
static void make_pixel_glyph(Glyph *g, int ttf, int k, uint32_t cp) {
    float s = 1.f / (float)k_unit[ttf];
    int w, h, xo, yo;
    uint8_t *small = stbtt_GetCodepointBitmap(&g_font[ttf], s, s, (int)cp, &w, &h, &xo, &yo);
    g->bmp = NULL;
    g->w = w * k;
    g->h = h * k;
    g->xoff = xo * k;
    g->yoff = (yo + pixel_drop(ttf, cp)) * k;
    if (small && w > 0 && h > 0) {
        g->bmp = malloc((size_t)g->w * (size_t)g->h);  // freed with stbtt_FreeBitmap (free)
        if (g->bmp)
            for (int y = 0; y < g->h; y++)
                for (int x = 0; x < g->w; x++) g->bmp[y * g->w + x] = small[(y / k) * w + x / k] >= 128 ? 255 : 0;
    }
    if (small) stbtt_FreeBitmap(small, NULL);
}

static Glyph *glyph_get(int ttf, float px, uint32_t cp) {
    int size4 = k_unit[ttf] ? -pixel_k(ttf, px) : (int)lrintf(px * 4.f);
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
            int adv, lsb;
            stbtt_GetCodepointHMetrics(fi, (int)cp, &adv, &lsb);
            if (size4 < 0) {
                make_pixel_glyph(g, ttf, -size4, cp);
            } else {
                float scale = stbtt_ScaleForPixelHeight(fi, size4 / 4.f);
                g->bmp = stbtt_GetCodepointBitmap(fi, scale, scale, (int)cp, &g->w, &g->h, &g->xoff, &g->yoff);
            }
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

static void blit(Canvas *c, const uint8_t *bmp, int w, int h, int gx, int gy, uint8_t gray) {
    Rect gr = rect_intersect((Rect){gx, gy, w, h}, c->clip);
    for (int yy = gr.y; yy < gr.y + gr.h; yy++) {
        const uint8_t *src = &bmp[(yy - gy) * w];
        uint8_t *dst = &c->px[yy * c->stride];
        for (int xx = gr.x; xx < gr.x + gr.w; xx++) {
            int a = src[xx - gx];
            if (a) dst[xx] = (uint8_t)(dst[xx] + ((gray - dst[xx]) * a + 127) / 255);
        }
    }
}

static void blit_glyph(Canvas *c, const Glyph *g, int gx, int gy, uint8_t gray) { blit(c, g->bmp, g->w, g->h, gx, gy, gray); }

// Letters the overprint would turn into others: sharp s (U+00DF) closes into a B, the notch
// between its stem and its bowls being all that tells them apart, and o slash (U+00F8, U+00D8)
// fills its counter around the slash until it reads as a theta.
static bool bold_keeps_gaps(uint32_t cp) { return cp == 0xDF || cp == 0xF8 || cp == 0xD8; }

// The bold weight of a pixel glyph above U+007F, made on first use. Bold is the glyph printed
// again one design pixel to the right, which closes every one-pixel gap: the dots of u-umlaut
// run together into a macron, the tilde and the circumflex fill in. So here the letter takes the
// overprint as before, but a detached mark (a component wholly above or below the letter's
// largest part: an accent, a cedilla that does not touch) is drawn as designed when the
// overprint would close a one-pixel gap in it. Marks it cannot harm (an acute, the dot of an
// inverted exclamation mark) keep the overprint. In the letters of bold_keeps_gaps a one-pixel
// gap moves one design pixel right instead of closing: the stroke on its left takes the
// overprint, the one on its right keeps its width. ASCII never comes here: it draws exactly as
// it always has. Returns NULL (plain overprint) if memory runs out.
static const uint8_t *pixel_bold(Glyph *g) {
    if (g->bold || !g->bmp || g->size4 >= 0) return g->bold;
    int k = -g->size4, w = g->w / k, h = g->h / k, n = w * h;
    // Per design pixel: its component (0 = paper), then a flood-fill stack; per component: its
    // first and last row, its size and its zone (0 the letter, 1 a mark above, 2 a mark below).
    int *mem = calloc((size_t)n * 6, sizeof *mem);
    uint8_t *out = calloc((size_t)(w + 1) * (size_t)h, 1);
    if (!mem || !out) {
        free(mem);
        free(out);
        return NULL;
    }
    int *lab = mem, *stack = mem + n, *top = mem + 2 * n, *bot = mem + 3 * n, *size = mem + 4 * n,
        *zone = mem + 5 * n;
    int nc = 0, big = 0;
    for (int i = 0; i < n; i++) {
        if (lab[i] || !g->bmp[(i / w) * k * g->w + (i % w) * k]) continue;
        int id = nc++, sp = 0;
        top[id] = h;
        bot[id] = -1;
        lab[i] = id + 1;
        stack[sp++] = i;
        while (sp) {
            int p = stack[--sp], px = p % w, py = p / w;
            size[id]++;
            if (py < top[id]) top[id] = py;
            if (py > bot[id]) bot[id] = py;
            for (int dy = -1; dy <= 1; dy++)
                for (int dx = -1; dx <= 1; dx++) {
                    int qx = px + dx, qy = py + dy;
                    if (qx < 0 || qy < 0 || qx >= w || qy >= h) continue;
                    int q = qy * w + qx;
                    if (!lab[q] && g->bmp[qy * k * g->w + qx * k]) {
                        lab[q] = id + 1;
                        stack[sp++] = q;
                    }
                }
        }
        if (size[id] > size[big]) big = id;
    }
    for (int i = 0; i < nc; i++) zone[i] = bot[i] < top[big] ? 1 : top[i] > bot[big] ? 2 : 0;
    // Marks (above, below) keep their designed shape if the overprint would fill a gap one
    // design pixel wide in them: paper between two inked pixels of a row.
    bool plain[3] = {false, false, false};
    for (int y = 0; y < h; y++)
        for (int x = 0; x + 2 < w; x++) {
            int l = lab[y * w + x];
            if (l && zone[l - 1] && !lab[y * w + x + 1] && lab[y * w + x + 2]) plain[zone[l - 1]] = true;
        }
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            int l = lab[y * w + x];
            if (!l) continue;
            out[y * (w + 1) + x] = 1;
            if (!plain[zone[l - 1]]) out[y * (w + 1) + x + 1] = 1;
        }
    if (bold_keeps_gaps(g->cp))
        for (int y = 0; y < h; y++)
            for (int x = 1; x + 1 < w; x++)
                if (!lab[y * w + x] && lab[y * w + x - 1] && lab[y * w + x + 1]) out[y * (w + 1) + x + 1] = 0;
    int bw = (w + 1) * k;
    g->bold = malloc((size_t)bw * (size_t)g->h);  // freed with the cache
    if (g->bold)
        for (int y = 0; y < g->h; y++)
            for (int x = 0; x < bw; x++) g->bold[y * bw + x] = out[(y / k) * (w + 1) + x / k] ? 255 : 0;
    free(mem);
    free(out);
    return g->bold;
}

// Horizontal font units -> screen pixels for a face at a size (pixel faces: whole design
// pixels, so every glyph starts on the grid).
static float units_to_px(int ttf, float gpx, int units) {
    if (k_unit[ttf]) return (float)(lrintf((float)units / (float)k_unit[ttf]) * pixel_k(ttf, gpx));
    return stbtt_ScaleForPixelHeight(&g_font[ttf], gpx) * (float)units;
}

static int layout(Canvas *c, FontFace f, float px, int x, int baseline, const char *s, uint8_t gray) {
    if (!g_ready || !s) return 0;
    FaceMap m = face_of(f);
    float fpx = px * m.scale;
    // Overprinting a second copy to the right reads as bold on e-ink: by one design pixel for a
    // pixel face, by a size-relative amount for an outline face.
    int bold = !m.embolden ? 0 : k_unit[m.ttf] ? pixel_k(m.ttf, fpx) : (int)fmaxf(1.f, lrintf(fpx / 34.f));
    float pen = (float)x;
    uint32_t prev = 0;
    int prev_ttf = -1;
    while (*s) {
        uint32_t cp = utf8_next(&s);
        // Glyphs a theme face lacks (symbols, arrows) come from Terminess at the asked size.
        int ttf = m.ttf;
        float gpx = fpx;
        if (ttf != TTF_TERM && ttf != TTF_TERM_BOLD && cp > 0x7F && !stbtt_FindGlyphIndex(&g_font[ttf], (int)cp)) {
            ttf = m.embolden ? TTF_TERM_BOLD : TTF_TERM;
            gpx = px;
        }
        const stbtt_fontinfo *fi = &g_font[ttf];
        if (prev && prev_ttf == ttf) pen += units_to_px(ttf, gpx, stbtt_GetCodepointKernAdvance(fi, (int)prev, (int)cp));
        Glyph *g = glyph_get(ttf, gpx, cp);
        if (c && g->bmp) {
            int gx = (int)lrintf(pen) + g->xoff, gy = baseline + g->yoff;
            // Accented letters in a pixel face's bold spare their marks (pixel_bold); that
            // bitmap already holds the overprint of one design pixel (bold == k px).
            const uint8_t *bb = bold && cp > 0x7F && k_unit[ttf] && bold == -g->size4 ? pixel_bold(g) : NULL;
            if (bb) {
                blit(c, bb, g->w + bold, g->h, gx, gy, gray);
            } else {
                blit_glyph(c, g, gx, gy, gray);
                for (int b = 1; b <= bold; b++) blit_glyph(c, g, gx + b, gy, gray);
            }
        }
        pen += units_to_px(ttf, gpx, g->advance) + (float)bold;
        prev = cp;
        prev_ttf = ttf;
    }
    return (int)lrintf(pen) - x;
}

int font_draw(Canvas *c, FontFace f, float px, int x, int baseline_y, const char *utf8, uint8_t gray) {
    return layout(c, f, px, x, baseline_y, utf8, gray);
}

int font_measure(FontFace f, float px, const char *utf8) { return layout(NULL, f, px, 0, 0, utf8, 0); }

bool font_has_glyph(FontFace f, uint32_t cp) {
    return g_ready && stbtt_FindGlyphIndex(&g_font[face_of(f).ttf], (int)cp) != 0;
}

int font_ascent(FontFace f, float px) {
    if (!g_ready) return (int)px;
    FaceMap m = face_of(f);
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[m.ttf], &a, &d, &l);
    if (k_unit[m.ttf]) return (int)lrintf((float)a / (float)k_unit[m.ttf]) * pixel_k(m.ttf, px * m.scale);
    return (int)lrintf(a * stbtt_ScaleForPixelHeight(&g_font[m.ttf], px * m.scale));
}

int font_descent(FontFace f, float px) {
    if (!g_ready) return 0;
    FaceMap m = face_of(f);
    int a, d, l;
    stbtt_GetFontVMetrics(&g_font[m.ttf], &a, &d, &l);
    if (k_unit[m.ttf]) return (int)lrintf((float)-d / (float)k_unit[m.ttf]) * pixel_k(m.ttf, px * m.scale);
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
