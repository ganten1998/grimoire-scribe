// paper.c: page backgrounds. Line positions depend only on page size and dpi, so a
// sub-region render matches a full render pixel for pixel.
#include "paper.h"

#include "font.h"
#include "i18n.h"

#include <math.h>

const char *paper_name(PaperKind k) {
    // A picture: a template the owner copied in, or a page of an imported document.
    if (paper_is_image(k)) return paper_is_document(k) ? C_("paper", "Imported page") : C_("paper", "Template");
    switch (k) {
    case PAPER_NARROW: return "Narrow rule";
    case PAPER_MEDIUM: return "Medium rule";
    case PAPER_WIDE: return "Wide rule";
    case PAPER_MUSIC: return "Sheet music";
    case PAPER_BLANK: return "Blank";
    case PAPER_NARROW_NM: return "Narrow, no margin";
    case PAPER_MEDIUM_NM: return "Medium, no margin";
    case PAPER_WIDE_NM: return "Wide, no margin";
    case PAPER_GRAND: return "Grand staff";
    case PAPER_TAB: return "Guitar tab";
    case PAPER_DOTS: return "Dot grid";
    case PAPER_GRID: return "Graph paper";
    case PAPER_CORNELL: return "Cornell notes";
    case PAPER_BASS4: return "4-string bass";
    case PAPER_BASS5: return "5-string bass";
    case PAPER_BASS6: return "6-string bass";
    case PAPER_BASS7: return "7-string bass";
    default: return paper_is_known(k) ? "Custom music" : "Newer paper";
    }
}

PaperKind paper_at(int index) {
    static const PaperKind order[PAPER_COUNT] = {
        PAPER_NARROW,  PAPER_MEDIUM, PAPER_WIDE,   PAPER_NARROW_NM, PAPER_MEDIUM_NM, PAPER_WIDE_NM, PAPER_CORNELL,
        PAPER_DOTS,    PAPER_GRID,   PAPER_MUSIC,  PAPER_GRAND,     PAPER_TAB,       PAPER_BASS4,
        PAPER_BASS5,   PAPER_BASS6,  PAPER_BASS7,  PAPER_BLANK};
    return order[clampi(index, 0, PAPER_COUNT - 1)];
}

#define NARROW_MM 5.5f
#define MEDIUM_MM 7.1f
#define WIDE_MM 9.0f

static int px_mm(float mm, float dpi) { return (int)lrintf(mm_to_px(mm, dpi)); }
static int thick(float px300, float dpi) {
    int t = (int)lrintf(px300 * dpi / 300.f);
    return t < 1 ? 1 : t;
}

static void ruled(Canvas *c, float pitch_mm, float dpi, bool margin) {
    int w = c->w, h = c->h;
    int line_t = thick(2, dpi);
    float pitch = mm_to_px(pitch_mm, dpi);
    float first = mm_to_px(22.f, dpi);
    int bottom = h - px_mm(10.f, dpi);
    // Header rule: slightly heavier.
    canvas_hline(c, 0, w, (int)lrintf(first), thick(3, dpi), 150);
    for (int i = 1;; i++) {
        int y = (int)lrintf(first + pitch * (float)i);
        if (y > bottom) break;
        canvas_hline(c, 0, w, y, line_t, 175);
    }
    if (!margin) return;
    // Double hairline margin.
    int mx = px_mm(25.f, dpi);
    canvas_vline(c, mx, 0, h, line_t, 140);
    canvas_vline(c, mx + thick(7, dpi), 0, h, line_t, 140);
}

static void music(Canvas *c, float dpi) {
    // 8 staves with roomy 2.7 mm staff spaces: written notes need far more room than printed.
    const int staves = 8;
    const float gap_spaces = 5.f;    // blank space between staves, in staff spaces
    int w = c->w, h = c->h;
    float top = mm_to_px(14.f, dpi), bot = mm_to_px(12.f, dpi);
    float space = ((float)h - top - bot) / (staves * 4.f + (staves - 1) * gap_spaces);
    int lt = thick(2, dpi);
    int x0 = px_mm(10.f, dpi), x1 = w - px_mm(10.f, dpi);
    for (int s = 0; s < staves; s++) {
        float y0 = top + (float)s * space * (4.f + gap_spaces);
        int ytop = (int)lrintf(y0), ybot = (int)lrintf(y0 + 4.f * space);
        for (int l = 0; l < 5; l++) canvas_hline(c, x0, x1, (int)lrintf(y0 + (float)l * space), lt, 90);
        canvas_vline(c, x0, ytop, ybot + lt, lt, 90);
        canvas_vline(c, x1 - lt, ytop, ybot + lt, lt, 90);
    }
}

// Piano grand staff: 5 systems of a treble and a bass staff joined by a brace and a system
// line, with clefs. Staff spaces come out around 2 mm: room to write.
static void grand(Canvas *c, float dpi) {
    const int systems = 5;
    const float inner = 5.f, between = 6.f;  // gaps in staff spaces: treble->bass, system->system
    int w = c->w, h = c->h;
    float top = mm_to_px(16.f, dpi), bot = mm_to_px(12.f, dpi);
    float space = ((float)h - top - bot) / (systems * (8.f + inner) + (systems - 1) * between);
    int lt = thick(2, dpi);
    int x0 = px_mm(16.f, dpi), x1 = w - px_mm(10.f, dpi);
    for (int sys = 0; sys < systems; sys++) {
        float y0 = top + (float)sys * space * (8.f + inner + between);
        float yb = y0 + space * (4.f + inner);  // bass staff top line
        for (int l = 0; l < 5; l++) {
            canvas_hline(c, x0, x1, (int)lrintf(y0 + (float)l * space), lt, 90);
            canvas_hline(c, x0, x1, (int)lrintf(yb + (float)l * space), lt, 90);
        }
        int ytop = (int)lrintf(y0), ybot = (int)lrintf(yb + 4.f * space);
        canvas_vline(c, x0, ytop, ybot + lt, thick(3, dpi), 70);  // system line joins both staves
        canvas_vline(c, x1 - lt, ytop, (int)lrintf(y0 + 4.f * space) + lt, lt, 90);
        canvas_vline(c, x1 - lt, (int)lrintf(yb), ybot + lt, lt, 90);
        // Brace: a bold bracket with curled ends left of the system line.
        float bx = (float)x0 - mm_to_px(3.f, dpi);
        canvas_line_aa(c, bx, (float)ytop + space, bx, (float)ybot - space, (float)thick(5, dpi), 60);
        canvas_line_aa(c, bx, (float)ytop + space, bx + mm_to_px(2.f, dpi), (float)ytop, (float)thick(4, dpi), 60);
        canvas_line_aa(c, bx, (float)ybot - space, bx + mm_to_px(2.f, dpi), (float)ybot, (float)thick(4, dpi), 60);
        // Clefs (Material Design music glyphs from the UI font).
        int cw = (int)lrintf(space * 3.2f);
        font_draw_icon(c, FONT_REGULAR, 0xF0F70, (Rect){x0 + (int)space, (int)lrintf(y0 - space * 1.6f), cw, (int)lrintf(space * 7.2f)}, 110);
        font_draw_icon(c, FONT_REGULAR, 0xF0F6F, (Rect){x0 + (int)space, (int)lrintf(yb - space * 0.2f), cw, (int)lrintf(space * 3.6f)}, 110);
    }
}

// Guitar tablature: six-line staves with "TAB" at the start.
static void tab(Canvas *c, float dpi) {
    const int staves = 9;
    const float gap = 4.f;
    int w = c->w, h = c->h;
    float top = mm_to_px(14.f, dpi), bot = mm_to_px(12.f, dpi);
    float space = ((float)h - top - bot) / (staves * 5.f + (staves - 1) * gap);
    int lt = thick(2, dpi);
    int x0 = px_mm(10.f, dpi), x1 = w - px_mm(10.f, dpi);
    for (int s = 0; s < staves; s++) {
        float y0 = top + (float)s * space * (5.f + gap);
        for (int l = 0; l < 6; l++) canvas_hline(c, x0, x1, (int)lrintf(y0 + (float)l * space), lt, 100);
        canvas_vline(c, x0, (int)lrintf(y0), (int)lrintf(y0 + 5.f * space) + lt, lt, 100);
        canvas_vline(c, x1 - lt, (int)lrintf(y0), (int)lrintf(y0 + 5.f * space) + lt, lt, 100);
        float fs = space * 1.75f;
        static const char *letters[3] = {"T", "A", "B"};
        for (int i = 0; i < 3; i++)
            font_draw(c, FONT_FIXED_BOLD, fs, x0 + (int)lrintf(space * 0.6f), (int)lrintf(y0 + space * (1.6f + 1.6f * (float)i)), letters[i], 60);
    }
}

// ---- custom music layouts ----

bool paper_is_custom(PaperKind k) { return ((uint32_t)k & 0xF0000000u) == (uint32_t)PAPER_CUSTOM_FLAG; }

bool music_decode(PaperKind k, MusicLayout *m) {
    if (!paper_is_custom(k)) return false;
    uint32_t v = (uint32_t)k;
    MusicLayout r = {.systems = (int)((v >> 24) & 15)};
    for (int i = 0; i < MUSIC_MAX_PARTS; i++) {
        int p = (int)((v >> (4 * i)) & 15);
        if (p == PART_NONE) break;
        if (p >= PART_COUNT) return false;
        r.part[r.n++] = (MusicPart)p;
    }
    if (r.n == 0 || r.systems < 1 || r.systems > MUSIC_MAX_SYSTEMS) return false;
    if (m) *m = r;
    return true;
}

PaperKind music_encode(const MusicLayout *m) {
    uint32_t v = (uint32_t)PAPER_CUSTOM_FLAG | (uint32_t)clampi(m->systems, 1, MUSIC_MAX_SYSTEMS) << 24;
    for (int i = 0; i < m->n && i < MUSIC_MAX_PARTS; i++) v |= (uint32_t)(m->part[i] & 15) << (4 * i);
    return (PaperKind)v;
}

// ---- image papers ----

bool paper_is_image(PaperKind k) {
    uint32_t v = (uint32_t)k;
    // The namespace, then bits 24-27: the role (0 or 1) and the colour bits (0).
    return (v & 0xF0000000u) == PAPER_IMAGE_FLAG && (v >> 24 & 15u) <= (uint32_t)PAPER_ROLE_DOCUMENT;
}

bool paper_is_document(PaperKind k) {
    return paper_is_image(k) && ((uint32_t)k >> 24 & 3u) == (uint32_t)PAPER_ROLE_DOCUMENT;
}

PaperKind paper_image(int role, uint32_t id) {
    return (PaperKind)(PAPER_IMAGE_FLAG | ((uint32_t)role & 3u) << 24 | (id & 0xFFFFFFu));
}

bool paper_is_known(PaperKind k) {
    return (uint32_t)k < PAPER_COUNT || music_decode(k, NULL) || paper_is_image(k);
}

PaperKind paper_sanitize(uint32_t v, PaperKind fallback) {
    return v < PAPER_COUNT || music_decode((PaperKind)v, NULL) ? (PaperKind)v : fallback;
}

PaperKind music_default(void) {
    MusicLayout m = {.systems = 5, .n = 2, .part = {PART_TREBLE, PART_TAB6}};
    return music_encode(&m);
}

const char *music_part_name(MusicPart p) {
    switch (p) {
    case PART_TREBLE: return "Treble staff";
    case PART_BASS: return "Bass staff";
    case PART_ALTO: return "Alto staff";
    case PART_TENOR: return "Tenor staff";
    case PART_PERC: return "Drum staff";
    case PART_STAFF: return "Plain staff";
    case PART_TAB4: return "Tab, 4 lines";
    case PART_TAB5: return "Tab, 5 lines";
    case PART_TAB6: return "Tab, 6 lines";
    case PART_TAB7: return "Tab, 7 lines";
    case PART_LYRICS: return "Lyric line";
    default: return "";
    }
}

static int part_lines(MusicPart p) {
    if (p >= PART_TAB4 && p <= PART_TAB7) return 4 + (p - PART_TAB4);
    return p == PART_LYRICS ? 1 : 5;
}

// Room in staff spaces: a part's own height, and the gap above part i of a system
// (staves get room for ledger lines; a lyric line sits close to what it sings).
static float part_height(MusicPart p) { return (float)(part_lines(p) - 1); }
static float gap_above(const MusicLayout *m, int i) {
    if (i == 0) return 0.f;
    if (m->part[i] == PART_LYRICS || m->part[i - 1] == PART_LYRICS) return 3.5f;
    return 4.5f;
}
#define SYSTEM_GAP 6.f
#define MUSIC_TOP_MM 14.f
#define MUSIC_BOT_MM 12.f
#define MIN_SPACE_MM 1.5f   // smaller than this is no longer comfortable to write in
#define MAX_SPACE_MM 3.0f   // larger looks like a children's primer

static float system_units(const MusicLayout *m) {
    float u = 0.f;
    for (int i = 0; i < m->n; i++) u += gap_above(m, i) + part_height(m->part[i]);
    return u;
}

int music_max_systems(const MusicLayout *m, int page_h, float dpi) {
    // The page's height in mm less the margins music_custom keeps: a 1986x2648 page has room for
    // more systems than an 1860x2480 one (whose answers are the same as when 2480 px was assumed).
    float page_mm = (float)page_h / dpi * 25.4f;
    float avail = page_mm - MUSIC_TOP_MM - MUSIC_BOT_MM;
    float su = system_units(m);
    int best = 1;
    for (int s = 1; s <= MUSIC_MAX_SYSTEMS; s++)
        if (avail / ((float)s * su + (float)(s - 1) * SYSTEM_GAP) >= MIN_SPACE_MM) best = s;
    return best;
}

static void clef_glyph(Canvas *c, uint32_t cp, int x, float y, float w, float h) {
    font_draw_icon(c, FONT_REGULAR, cp, (Rect){x, (int)lrintf(y), (int)lrintf(w), (int)lrintf(h)}, 110);
}

static void draw_part(Canvas *c, MusicPart p, float y0, float space, int x0, int x1, float dpi) {
    int lt = thick(2, dpi);
    int lines = part_lines(p);
    if (p == PART_LYRICS) {
        canvas_hline(c, x0, x1, (int)lrintf(y0), lt, 170);
        return;
    }
    bool tab = p >= PART_TAB4 && p <= PART_TAB7;
    uint8_t g = tab ? 100 : 90;
    float h = space * (float)(lines - 1);
    for (int l = 0; l < lines; l++) canvas_hline(c, x0, x1, (int)lrintf(y0 + (float)l * space), lt, g);
    canvas_vline(c, x0, (int)lrintf(y0), (int)lrintf(y0 + h) + lt, lt, g);
    canvas_vline(c, x1 - lt, (int)lrintf(y0), (int)lrintf(y0 + h) + lt, lt, g);
    int cx = x0 + (int)space;
    switch (p) {
    case PART_TREBLE: clef_glyph(c, 0xF0F70, cx, y0 - space * 1.6f, space * 3.2f, space * 7.2f); break;
    case PART_BASS: clef_glyph(c, 0xF0F6F, cx, y0 - space * 0.2f, space * 3.2f, space * 3.6f); break;
    // The C clef centers on its line: the middle line (alto), the second from the top (tenor).
    case PART_ALTO: clef_glyph(c, 0xF0F6E, cx, y0 - space * 0.2f, space * 4.4f, space * 4.4f); break;
    case PART_TENOR: clef_glyph(c, 0xF0F6E, cx, y0 - space * 1.2f, space * 4.4f, space * 4.4f); break;
    case PART_PERC: {
        // The neutral clef: two thick bars across the middle spaces.
        int bw = (int)lrintf(space * 0.4f);
        for (int b = 0; b < 2; b++)
            canvas_fill(c, (Rect){cx + (int)lrintf(space * (0.6f + 0.9f * (float)b)), (int)lrintf(y0 + space), bw, (int)lrintf(space * 2.f)}, 80);
        break;
    }
    default:
        if (tab) {
            // "TAB" spread down the lines, shrinking to fit a 4-line tab.
            float fs = fminf(space * 1.75f, h / 3.f * 1.05f);
            static const char *letters[3] = {"T", "A", "B"};
            for (int i = 0; i < 3; i++) {
                float cy = y0 + h * ((float)i + 0.5f) / 3.f;
                font_draw(c, FONT_FIXED_BOLD, fs, x0 + (int)lrintf(space * 0.6f), (int)lrintf(cy + fs * 0.34f), letters[i], 60);
            }
        }
        break;
    }
}

static void music_custom(Canvas *c, const MusicLayout *m, float dpi) {
    float top = mm_to_px(MUSIC_TOP_MM, dpi), bot = mm_to_px(MUSIC_BOT_MM, dpi);
    float avail = (float)c->h - top - bot;
    float su = system_units(m);
    float space = avail / ((float)m->systems * su + (float)(m->systems - 1) * SYSTEM_GAP);
    float sgap = SYSTEM_GAP;
    float maxs = mm_to_px(MAX_SPACE_MM, dpi);
    if (space > maxs) {
        // Few systems: keep the staff size sane and spread the systems down the page instead.
        space = maxs;
        if (m->systems > 1) sgap = (avail / space - (float)m->systems * su) / (float)(m->systems - 1);
    }
    int staffy = 0;
    for (int i = 0; i < m->n; i++) staffy += m->part[i] != PART_LYRICS;
    bool joined = staffy > 1;
    int x0 = px_mm(joined ? 16.f : 10.f, dpi), x1 = c->w - px_mm(10.f, dpi);
    float y = top;
    for (int s = 0; s < m->systems; s++) {
        float first = -1.f, last = -1.f, py[MUSIC_MAX_PARTS];
        for (int i = 0; i < m->n; i++) {
            y += gap_above(m, i) * space;
            py[i] = y;
            if (m->part[i] != PART_LYRICS) {
                if (first < 0.f) first = y;
                last = y + part_height(m->part[i]) * space;
            }
            draw_part(c, m->part[i], y, space, x0, x1, dpi);
            y += part_height(m->part[i]) * space;
        }
        if (joined) {
            // The system line joins every staff and tab of the system.
            canvas_vline(c, x0, (int)lrintf(first), (int)lrintf(last) + thick(2, dpi), thick(3, dpi), 70);
            // A brace for each treble + bass pair (a piano part).
            for (int i = 0; i + 1 < m->n; i++) {
                if (m->part[i] != PART_TREBLE || m->part[i + 1] != PART_BASS) continue;
                float yt = py[i], yb = py[i + 1] + 4.f * space;
                float bx = (float)x0 - mm_to_px(3.f, dpi);
                canvas_line_aa(c, bx, yt + space, bx, yb - space, (float)thick(5, dpi), 60);
                canvas_line_aa(c, bx, yt + space, bx + mm_to_px(2.f, dpi), yt, (float)thick(4, dpi), 60);
                canvas_line_aa(c, bx, yb - space, bx + mm_to_px(2.f, dpi), yb, (float)thick(4, dpi), 60);
            }
        }
        y += sgap * space;
    }
}

// Bass tab pages are custom layouts with a single tab part; the counts keep about 2 mm
// between lines, like the guitar tab page.
static void bass_tab(Canvas *c, int strings, float dpi) {
    static const int systems[4] = {10, 10, 9, 8};
    MusicLayout m = {.n = 1, .systems = systems[clampi(strings - 4, 0, 3)], .part = {(MusicPart)(PART_TAB4 + strings - 4)}};
    music_custom(c, &m, dpi);
}

// 5 mm dot grid. Only the dots inside the clip are visited.
static void dots(Canvas *c, float dpi) {
    float pitch = mm_to_px(5.f, dpi), r = (float)thick(3, dpi) * 0.8f;
    float ox = fmodf((float)c->w, pitch) / 2.f + pitch / 2.f, oy = mm_to_px(10.f, dpi);
    Rect cl = c->clip;
    int i0 = (int)floorf(((float)cl.x - ox - r) / pitch), i1 = (int)ceilf(((float)(cl.x + cl.w) - ox + r) / pitch);
    int j0 = (int)floorf(((float)cl.y - oy - r) / pitch), j1 = (int)ceilf(((float)(cl.y + cl.h) - oy + r) / pitch);
    if (i0 < 0) i0 = 0;
    if (j0 < 0) j0 = 0;
    for (int j = j0; j <= j1; j++) {
        float y = oy + (float)j * pitch;
        if (y > (float)c->h - mm_to_px(8.f, dpi)) break;
        for (int i = i0; i <= i1; i++) {
            float x = ox + (float)i * pitch;
            if (x > (float)c->w - pitch / 2.f) break;
            canvas_disc_aa(c, x, y, r, 90, 1.f);
        }
    }
}

// 5 mm graph paper, every fifth line a little stronger.
static void grid(Canvas *c, float dpi) {
    float pitch = mm_to_px(5.f, dpi);
    int lt = thick(1, dpi), lt5 = thick(2, dpi);
    for (int i = 0;; i++) {
        int x = (int)lrintf(pitch * (float)i);
        if (x >= c->w) break;
        canvas_vline(c, x, 0, c->h, i % 5 ? lt : lt5, i % 5 ? 200 : 170);
    }
    for (int j = 0;; j++) {
        int y = (int)lrintf(pitch * (float)j);
        if (y >= c->h) break;
        canvas_hline(c, 0, c->w, y, j % 5 ? lt : lt5, j % 5 ? 200 : 170);
    }
}

// Cornell notes: medium rule, a cue column on the left, a summary block at the bottom.
static void cornell(Canvas *c, float dpi) {
    ruled(c, MEDIUM_MM, dpi, false);
    int lt = thick(3, dpi);
    int head = (int)lrintf(mm_to_px(22.f, dpi));
    int sum = c->h - px_mm(52.f, dpi);
    int cue = px_mm(55.f, dpi);
    canvas_vline(c, cue, head, sum, lt, 120);
    canvas_hline(c, 0, c->w, sum, lt, 120);
    float fs = mm_to_px(2.6f, dpi);
    font_draw(c, FONT_FIXED, fs, px_mm(4.f, dpi), head - px_mm(3.f, dpi), "Cues", 150);
    font_draw(c, FONT_FIXED, fs, cue + px_mm(4.f, dpi), head - px_mm(3.f, dpi), "Notes", 150);
    font_draw(c, FONT_FIXED, fs, px_mm(4.f, dpi), sum + px_mm(6.f, dpi), "Summary", 150);
}

void paper_render(Canvas *c, PaperKind k, Rect region, float dpi) {
    Rect saved = c->clip;
    canvas_set_clip(c, rect_intersect(region, saved));
    if (rect_is_empty(c->clip)) {
        c->clip = saved;
        return;
    }
    canvas_fill(c, c->clip, 255);
    switch (k) {
    // Spread ~30% apart so the three are clearly different on screen (the stationery values,
    // 6.35 / 7.14 / 8.73 mm, looked nearly identical between narrow and medium).
    case PAPER_NARROW: ruled(c, NARROW_MM, dpi, true); break;
    case PAPER_MEDIUM: ruled(c, MEDIUM_MM, dpi, true); break;
    case PAPER_WIDE: ruled(c, WIDE_MM, dpi, true); break;
    case PAPER_NARROW_NM: ruled(c, NARROW_MM, dpi, false); break;
    case PAPER_MEDIUM_NM: ruled(c, MEDIUM_MM, dpi, false); break;
    case PAPER_WIDE_NM: ruled(c, WIDE_MM, dpi, false); break;
    case PAPER_MUSIC: music(c, dpi); break;
    case PAPER_GRAND: grand(c, dpi); break;
    case PAPER_TAB: tab(c, dpi); break;
    case PAPER_DOTS: dots(c, dpi); break;
    case PAPER_GRID: grid(c, dpi); break;
    case PAPER_CORNELL: cornell(c, dpi); break;
    case PAPER_BASS4: bass_tab(c, 4, dpi); break;
    case PAPER_BASS5: bass_tab(c, 5, dpi); break;
    case PAPER_BASS6: bass_tab(c, 6, dpi); break;
    case PAPER_BASS7: bass_tab(c, 7, dpi); break;
    default: {
        MusicLayout m;
        if (music_decode(k, &m)) music_custom(c, &m, dpi);
        break;  // anything else (a newer version's paper) stays blank
    }
    }
    c->clip = saved;
}
