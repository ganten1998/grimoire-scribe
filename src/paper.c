// paper.c: page backgrounds. Line positions depend only on page size and dpi, so a
// sub-region render matches a full render pixel for pixel.
#include "paper.h"

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
    default: return "Paper";
    }
}

PaperKind paper_at(int index) {
    static const PaperKind order[PAPER_COUNT] = {PAPER_NARROW,    PAPER_MEDIUM,    PAPER_WIDE,    PAPER_MUSIC,
                                                 PAPER_NARROW_NM, PAPER_MEDIUM_NM, PAPER_WIDE_NM, PAPER_BLANK};
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
    default: break;
    }
    c->clip = saved;
}
