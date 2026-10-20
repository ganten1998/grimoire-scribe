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
    // An erase stroke reaches as far as its texture's tool does (white charcoal's specks).
    float pad = brush_max_radius(stroke_render_tool(s), &s->params, dpi) + 2.f;
    s->bbox = rect_union(s->bbox, rect_from_points(pt->x, pt->y, pt->x, pt->y, pad));
}

ToolKind stroke_render_tool(const Stroke *s) {
    if (s->tool != TOOL_ERASER) return s->tool;
    int t = (int)(s->seed >> 24);  // the texture: a drawing tool + 1, or 0 for the round tip
    return t >= 1 && t <= TOOL_COUNT && t - 1 != TOOL_ERASER ? (ToolKind)(t - 1) : TOOL_ERASER;
}

uint32_t erase_seed(ToolKind texture, uint32_t hash) {
    int t = (int)texture;
    uint32_t top = t >= 0 && t < TOOL_COUNT && t != TOOL_ERASER ? (uint32_t)t + 1u : 0u;
    return top << 24 | (hash & 0xFFFFFFu);
}

void stroke_brush_begin(BrushState *b, const Stroke *s, float dpi, const uint8_t *paper, int x0, int y0, int stride) {
    bool erase = stroke_is_erase(s);
    brush_begin(b, stroke_render_tool(s), &s->params, erase ? s->seed & 0xFFFFFFu : s->seed, dpi);
    if (erase) brush_set_erase(b, paper, x0, y0, stride);
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
    p->paper = paper;  // an unknown kind stays as it is: a newer Grimoire draws it (docs/FORMAT.md)
    return p;
}

void page_free(Page *p) {
    if (!p) return;
    page_clear_strokes(p);
    free(p->strokes);
    free(p->tail);
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
    page_render_layers(c, p, region, dpi, PAGE_LAYER_ALL);
}

// Whether stroke i is drawn: it is not `left_out`, nor marked in skip (NULL = none marked).
static bool drawn(const bool *skip, int left_out, int i) { return i != left_out && !(skip && skip[i]); }

// page_render_layers, leaving out the strokes marked in skip and stroke left_out (compaction).
static void render_layers(Canvas *c, const Page *p, Rect region, float dpi, int layers, const bool *skip,
                          int left_out) {
    Rect saved = c->clip;
    Rect r = rect_intersect(region, saved);
    if (rect_is_empty(r)) return;
    // Erase strokes restore the paper under them: with the paper layer, a copy of the paper drawn
    // here, of the part of r their bboxes reach (u); without it, white.
    Rect u = RECT_EMPTY;
    if ((layers & PAGE_LAYER_PAPER) && (layers & PAGE_LAYER_INK))
        for (int i = 0; i < p->n; i++)
            if (drawn(skip, left_out, i) && stroke_is_erase(p->strokes[i]))
                u = rect_union(u, rect_intersect(p->strokes[i]->bbox, r));
    uint8_t *copy = NULL;
    if (!rect_is_empty(u) && !(copy = malloc((size_t)u.w * (size_t)u.h))) {
        if (r.w > 1 || r.h > 1) {
            // Out of memory: the same pixels in two halves, each with a smaller copy.
            Rect a = r, b = r;
            if (r.h >= r.w) {
                a.h = r.h / 2;
                b.y += a.h;
                b.h -= a.h;
            } else {
                a.w = r.w / 2;
                b.x += a.w;
                b.w -= a.w;
            }
            render_layers(c, p, a, dpi, layers, skip, left_out);
            render_layers(c, p, b, dpi, layers, skip, left_out);
            return;
        }
        static bool logged;
        if (!logged) qlog("page: out of memory, erase strokes restore toward white");
        logged = true;
    }
    if (layers & PAGE_LAYER_PAPER) paper_render(c, p->paper, r, dpi);
    if (!(layers & PAGE_LAYER_INK)) return;
    for (int y = 0; copy && y < u.h; y++)
        memcpy(copy + (size_t)y * (size_t)u.w, &c->px[(u.y + y) * c->stride + u.x], (size_t)u.w);
    BrushState b;
    for (int i = 0; i < p->n; i++) {
        const Stroke *s = p->strokes[i];
        if (!drawn(skip, left_out, i) || (unsigned)s->tool >= TOOL_COUNT || !rect_overlaps(s->bbox, r)) continue;
        // An erase stroke stays inside its bbox, which u covers where it meets r: clipped to u it
        // draws the same pixels, and it never reads past the copy.
        canvas_set_clip(c, copy && stroke_is_erase(s) ? u : r);
        stroke_brush_begin(&b, s, dpi, copy, u.x, u.y, u.w);
        for (int k = 0; k < s->n; k++) brush_add(&b, c, &s->pts[k]);
    }
    free(copy);
    c->clip = saved;
}

void page_render_layers(Canvas *c, const Page *p, Rect region, float dpi, int layers) {
    render_layers(c, p, region, dpi, layers, NULL, -1);
}

void page_render_thumb(Canvas *thumb, const Page *p, float dpi) {
    Canvas full;
    if (canvas_init(&full, p->w, p->h) != 0) return;
    page_render(&full, p, (Rect){0, 0, p->w, p->h}, dpi);
    canvas_downsample(thumb, &full);
    canvas_free(&full);
}

// ---- compaction ----

// Whether drawing r without stroke i (and the strokes marked in drop) gives the pixels cur holds
// there, drawn with it. Every pixel's value depends only on what was drawn over that pixel, and a
// stroke draws inside its bbox, so r = its bbox settles it for the whole page. tmp is scratch.
static bool same_without(Canvas *tmp, const Canvas *cur, const Page *p, Rect r, float dpi, int layers,
                         const bool *drop, int i) {
    if (!(layers & PAGE_LAYER_PAPER)) canvas_fill(tmp, r, 255);  // the ink layer starts white
    render_layers(tmp, p, r, dpi, layers, drop, i);
    for (int y = r.y; y < r.y + r.h; y++)
        if (memcmp(&tmp->px[y * tmp->stride + r.x], &cur->px[y * cur->stride + r.x], (size_t)r.w)) return false;
    return true;
}

// Whether an erase stroke after stroke i reaches into its bbox (only such ink can be erased).
static bool erased_later(const Page *p, int i) {
    for (int j = i + 1; j < p->n; j++)
        if (stroke_is_erase(p->strokes[j]) && rect_overlaps(p->strokes[j]->bbox, p->strokes[i]->bbox)) return true;
    return false;
}

int page_droppable(const Page *p, float dpi, bool *drop) {
    bool any = false;
    for (int i = 0; i < p->n; i++) {
        drop[i] = false;
        any = any || stroke_is_erase(p->strokes[i]);
    }
    if (!any) return 0;
    Canvas all, ink, tmp;
    if (canvas_init(&all, p->w, p->h) != 0) return -1;
    if (canvas_init(&ink, p->w, p->h) != 0) {
        canvas_free(&all);
        return -1;
    }
    if (canvas_init(&tmp, p->w, p->h) != 0) {
        canvas_free(&all);
        canvas_free(&ink);
        return -1;
    }
    Rect page = {0, 0, p->w, p->h};
    render_layers(&all, p, page, dpi, PAGE_LAYER_ALL, NULL, -1);
    render_layers(&ink, p, page, dpi, PAGE_LAYER_INK, NULL, -1);
    // A stroke whose leaving out changes neither render is marked. all and ink then still hold
    // the page as drawn without everything marked, so each check sees the ones before it. Ink
    // goes first: once it is out, the erase strokes over it may have nothing left to change.
    int n = 0;
    for (int pass = 0; pass < 2; pass++)
        for (int i = 0; i < p->n; i++) {
            const Stroke *s = p->strokes[i];
            if ((unsigned)s->tool >= TOOL_COUNT || stroke_is_erase(s) != (pass == 1)) continue;
            if (pass == 0 && !erased_later(p, i)) continue;
            Rect r = rect_intersect(s->bbox, page);
            if (rect_is_empty(r) || !same_without(&tmp, &all, p, r, dpi, PAGE_LAYER_ALL, drop, i) ||
                !same_without(&tmp, &ink, p, r, dpi, PAGE_LAYER_INK, drop, i))
                continue;
            drop[i] = true;
            n++;
        }
    canvas_free(&all);
    canvas_free(&ink);
    canvas_free(&tmp);
    return n;
}
