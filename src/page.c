// page.c: strokes, pages, hit testing and page rendering.
#include "page.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

Stroke *stroke_new(ToolKind t, const ToolParams *p, uint32_t seed) {
    Stroke *s = calloc(1, sizeof *s);
    if (!s) return NULL;
    s->tool = t;
    s->params = *p;
    s->seed = seed;
    return s;
}

void stroke_free(Stroke *s) {
    if (!s) return;
    free(s->pts);
    free(s);
}

void stroke_append(Stroke *s, const PenSample *pt, float dpi) {
    if (s->n == s->cap) {
        int nc = s->cap ? s->cap * 2 : 64;
        PenSample *np = realloc(s->pts, (size_t)nc * sizeof *np);
        if (!np) return;
        s->pts = np;
        s->cap = nc;
    }
    s->pts[s->n++] = *pt;
    float pad = brush_max_radius(s->tool, &s->params, dpi) + 2.f;
    s->bbox = rect_union(s->bbox, rect_from_points(pt->x, pt->y, pt->x, pt->y, pad));
}

static float seg_point_dist2(float px, float py, float ax, float ay, float bx, float by) {
    float dx = bx - ax, dy = by - ay;
    float l2 = dx * dx + dy * dy;
    float t = l2 > 0.f ? clampf(((px - ax) * dx + (py - ay) * dy) / l2, 0.f, 1.f) : 0.f;
    float ex = px - (ax + dx * t), ey = py - (ay + dy * t);
    return ex * ex + ey * ey;
}

static bool segs_intersect(float ax, float ay, float bx, float by, float cx, float cy, float dx, float dy) {
    float d1 = (dx - cx) * (ay - cy) - (dy - cy) * (ax - cx);
    float d2 = (dx - cx) * (by - cy) - (dy - cy) * (bx - cx);
    float d3 = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    float d4 = (bx - ax) * (dy - ay) - (by - ay) * (dx - ax);
    return ((d1 > 0) != (d2 > 0)) && ((d3 > 0) != (d4 > 0));
}

static float seg_seg_dist2(float ax, float ay, float bx, float by, float cx, float cy, float dx, float dy) {
    if (segs_intersect(ax, ay, bx, by, cx, cy, dx, dy)) return 0.f;
    float m = seg_point_dist2(ax, ay, cx, cy, dx, dy);
    m = fminf(m, seg_point_dist2(bx, by, cx, cy, dx, dy));
    m = fminf(m, seg_point_dist2(cx, cy, ax, ay, bx, by));
    m = fminf(m, seg_point_dist2(dx, dy, ax, ay, bx, by));
    return m;
}

bool stroke_hit_segment(const Stroke *s, float x0, float y0, float x1, float y1, float r, float dpi) {
    if (!s->n) return false;
    Rect sweep = rect_from_points(x0, y0, x1, y1, r + 1.f);
    if (!rect_overlaps(sweep, s->bbox)) return false;
    for (int i = 0; i < s->n; i++) {
        const PenSample *a = &s->pts[i], *b = &s->pts[i + 1 < s->n ? i + 1 : i];
        float ra, op;
        brush_dab(s->tool, &s->params, a, dpi, &ra, &op);
        float lim = r + ra;
        if (seg_seg_dist2(x0, y0, x1, y1, a->x, a->y, b->x, b->y) <= lim * lim) return true;
    }
    return false;
}

bool stroke_hit(const Stroke *s, float x, float y, float r, float dpi) {
    return stroke_hit_segment(s, x, y, x, y, r, dpi);
}

Page *page_new(int w, int h, PaperKind paper) {
    Page *p = calloc(1, sizeof *p);
    if (!p) return NULL;
    p->w = w;
    p->h = h;
    p->paper = paper_sanitize((uint32_t)paper, PAPER_BLANK);
    return p;
}

void page_free(Page *p) {
    if (!p) return;
    page_clear_strokes(p);
    free(p->strokes);
    free(p);
}

static bool page_grow(Page *p) {
    if (p->n < p->cap) return true;
    int nc = p->cap ? p->cap * 2 : 32;
    Stroke **ns = realloc(p->strokes, (size_t)nc * sizeof *ns);
    if (!ns) return false;
    p->strokes = ns;
    p->cap = nc;
    return true;
}

void page_add(Page *p, Stroke *s) { page_insert(p, p->n, s); }

void page_insert(Page *p, int index, Stroke *s) {
    if (!s) return;
    if (!page_grow(p)) {
        qlog("page: out of memory, stroke dropped");
        stroke_free(s);
        return;
    }
    index = clampi(index, 0, p->n);
    memmove(&p->strokes[index + 1], &p->strokes[index], (size_t)(p->n - index) * sizeof *p->strokes);
    p->strokes[index] = s;
    p->n++;
}

Stroke *page_remove(Page *p, int index) {
    if (index < 0 || index >= p->n) return NULL;
    Stroke *s = p->strokes[index];
    memmove(&p->strokes[index], &p->strokes[index + 1], (size_t)(p->n - index - 1) * sizeof *p->strokes);
    p->n--;
    return s;
}

void page_clear_strokes(Page *p) {
    for (int i = 0; i < p->n; i++) stroke_free(p->strokes[i]);
    p->n = 0;
}

void page_render(Canvas *c, const Page *p, Rect region, float dpi) {
    Rect saved = c->clip;
    Rect r = rect_intersect(region, saved);
    if (rect_is_empty(r)) return;
    paper_render(c, p->paper, r, dpi);
    canvas_set_clip(c, r);
    BrushState b;
    for (int i = 0; i < p->n; i++) {
        const Stroke *s = p->strokes[i];
        if (!rect_overlaps(s->bbox, r)) continue;
        brush_begin(&b, s->tool, &s->params, s->seed, dpi);
        for (int k = 0; k < s->n; k++) brush_add(&b, c, &s->pts[k]);
    }
    c->clip = saved;
}

void page_render_thumb(Canvas *thumb, const Page *p, float dpi) {
    Canvas full;
    if (canvas_init(&full, p->w, p->h) != 0) return;
    page_render(&full, p, (Rect){0, 0, p->w, p->h}, dpi);
    canvas_downsample(thumb, &full);
    canvas_free(&full);
}
