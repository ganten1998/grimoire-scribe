// paper.c: page backgrounds. Line positions depend only on page size and dpi, so a
// sub-region render matches a full render pixel for pixel.
#include "paper.h"

#include "font.h"

#include <math.h>

const char *paper_name(PaperKind k) {
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
    default: return "Paper";
    }
}

PaperKind paper_at(int index) {
    static const PaperKind order[PAPER_COUNT] = {
        PAPER_NARROW,  PAPER_MEDIUM, PAPER_WIDE,   PAPER_NARROW_NM, PAPER_MEDIUM_NM, PAPER_WIDE_NM, PAPER_CORNELL,
        PAPER_DOTS,    PAPER_GRID,   PAPER_MUSIC,  PAPER_GRAND,     PAPER_TAB,       PAPER_BLANK};
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
            font_draw(c, FONT_BOLD, fs, x0 + (int)lrintf(space * 0.6f), (int)lrintf(y0 + space * (1.6f + 1.6f * (float)i)), letters[i], 60);
    }
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
    font_draw(c, FONT_REGULAR, fs, px_mm(4.f, dpi), head - px_mm(3.f, dpi), "Cues", 150);
    font_draw(c, FONT_REGULAR, fs, cue + px_mm(4.f, dpi), head - px_mm(3.f, dpi), "Notes", 150);
    font_draw(c, FONT_REGULAR, fs, px_mm(4.f, dpi), sum + px_mm(6.f, dpi), "Summary", 150);
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
    default: break;
    }
    c->clip = saved;
}
