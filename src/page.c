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

// A stroke in pieces (compaction): runs of its points, each with the brush as it stands before the
// run and the rect its dabs can draw into, measured by a dry run of the brush. A region drawn from
// the pieces that reach into it, each from its own brush, gets exactly what a replay from the first
// point gives it: the brush's course does not depend on the canvas, and the other pieces draw
// nothing there.
typedef struct {
    int first, end;  // points first..end-1
    Rect box;        // on the page (empty: they draw nothing)
    BrushState b;    // before point `first`
} Piece;

// A page's strokes in pieces, each stroke cut the first time it is drawn so.
typedef struct {
    const Page *p;
    float dpi;
    Piece **pc;  // per stroke: its pieces once cut, else NULL
    int *n;      // per stroke: how many
    Piece *tmp;  // the pieces of the stroke being cut
    int tmp_cap;
} Pieces;

static const Piece *pieces_of(Pieces *ps, int i, int *count);

// page_render_layers, leaving out the strokes marked in skip and stroke left_out, and drawing each
// stroke from its pieces if ps is given (compaction).
static void render_layers(Canvas *c, const Page *p, Rect region, float dpi, int layers, const bool *skip,
                          int left_out, Pieces *ps) {
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
            render_layers(c, p, a, dpi, layers, skip, left_out, ps);
            render_layers(c, p, b, dpi, layers, skip, left_out, ps);
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
        int np = 0;
        const Piece *pcs = ps ? pieces_of(ps, i, &np) : NULL;
        if (!pcs) {  // whole (also when there was no memory to cut it)
            stroke_brush_begin(&b, s, dpi, copy, u.x, u.y, u.w);
            for (int k = 0; k < s->n; k++) brush_add(&b, c, &s->pts[k]);
            continue;
        }
        for (int k = 0, next = -1; k < np; k++) {  // next: the point b stands before
            const Piece *pc = &pcs[k];
            if (!rect_overlaps(pc->box, r)) continue;
            if (pc->first != next) {
                b = pc->b;
                if (stroke_is_erase(s)) brush_set_erase(&b, copy, u.x, u.y, u.w);
            }
            for (int m = pc->first; m < pc->end; m++) brush_add(&b, c, &s->pts[m]);
            next = pc->end;
        }
    }
    free(copy);
    c->clip = saved;
}

void page_render_layers(Canvas *c, const Page *p, Rect region, float dpi, int layers) {
    render_layers(c, p, region, dpi, layers, NULL, -1, NULL);
}

void page_render_thumb(Canvas *thumb, const Page *p, float dpi) {
    Canvas full;
    if (canvas_init(&full, p->w, p->h) != 0) return;
    page_render(&full, p, (Rect){0, 0, p->w, p->h}, dpi);
    canvas_downsample(thumb, &full);
    canvas_free(&full);
}

// ---- compaction ----

// Stroke i's pieces, cut now if they are not yet (a piece ends at the first point that takes its
// rect past 16 mm either way), or NULL when out of memory: then the stroke is drawn whole.
static const Piece *pieces_of(Pieces *ps, int i, int *count) {
    if (ps->pc[i]) {
        *count = ps->n[i];
        return ps->pc[i];
    }
    const Page *p = ps->p;
    const Stroke *s = p->strokes[i];
    float span = mm_to_px(16.f, ps->dpi);
    Canvas none = {.w = p->w, .h = p->h, .stride = p->w, .clip = {0, 0, p->w, p->h}};  // no pixels: dry
    BrushState b;
    stroke_brush_begin(&b, s, ps->dpi, NULL, 0, 0, 0);
    b.dry = true;
    int n = 0;
    for (int k = 0; k < s->n;) {
        if (n == ps->tmp_cap) {
            int nc = ps->tmp_cap ? ps->tmp_cap * 2 : 64;
            Piece *np = realloc(ps->tmp, (size_t)nc * sizeof *np);
            if (!np) return NULL;
            ps->tmp = np;
            ps->tmp_cap = nc;
        }
        Piece *pc = &ps->tmp[n++];
        pc->first = k;
        pc->box = RECT_EMPTY;
        pc->b = b;
        pc->b.dry = false;
        do pc->box = rect_union(pc->box, brush_add(&b, &none, &s->pts[k++]));
        while (k < s->n && pc->box.w < span && pc->box.h < span);
        pc->end = k;
    }
    Piece *own = malloc(n ? (size_t)n * sizeof *own : 1);
    if (!own) return NULL;
    if (n) memcpy(own, ps->tmp, (size_t)n * sizeof *own);
    ps->pc[i] = own;
    *count = ps->n[i] = n;
    return own;
}

// What compaction works with.
typedef struct {
    const Page *p;
    float dpi;
    const bool *drop;  // the strokes marked so far
    Pieces ps;
    // The reference: the page as it is, each layer drawn where a check of that layer first needs
    // it (`ready` per REF_BLOCK square). Marked strokes change nothing, so drawing it without them
    // gives the same pixels and it never goes stale.
    Canvas layer[2];   // the page, the ink layer
    uint8_t *ready[2];
    int blocks_x;
    // Where the paper is white, a pixel of the ink layer is that pixel of the page, whatever is
    // drawn: the same strokes over the same white, restoring toward the same white. So the ink
    // layer is checked only in the rows where the paper has marks (`marked`, per row and REF_BLOCK
    // of columns), and not at all on paper white all over (then there is no ink canvas either).
    bool ink_too;
    uint8_t *marked;
    uint8_t *keep;     // the reference under a region, while a check draws over it
    size_t keep_cap;
} Compact;

#define REF_BLOCK 16

// Makes sure the reference of layer l (0 the page, 1 the ink layer) is drawn over r, a rect on the
// page: the rect around the blocks r meets that are not yet (drawing a block again changes nothing).
static void ref_ready(Compact *x, int l, Rect r) {
    uint8_t *ready = x->ready[l];
    int bx0 = r.x / REF_BLOCK, bx1 = (r.x + r.w - 1) / REF_BLOCK;
    int by0 = r.y / REF_BLOCK, by1 = (r.y + r.h - 1) / REF_BLOCK;
    int mx0 = bx1 + 1, mx1 = bx0 - 1, my0 = by1 + 1, my1 = by0 - 1;
    for (int by = by0; by <= by1; by++)
        for (int bx = bx0; bx <= bx1; bx++)
            if (!ready[by * x->blocks_x + bx]) {
                mx0 = bx < mx0 ? bx : mx0;
                mx1 = bx > mx1 ? bx : mx1;
                my0 = by < my0 ? by : my0;
                my1 = by > my1 ? by : my1;
            }
    if (mx1 < mx0) return;
    Rect t = {mx0 * REF_BLOCK, my0 * REF_BLOCK, (mx1 - mx0 + 1) * REF_BLOCK, (my1 - my0 + 1) * REF_BLOCK};
    t = rect_intersect(t, (Rect){0, 0, x->p->w, x->p->h});
    if (l) canvas_fill(&x->layer[l], t, 255);  // the ink layer starts white
    render_layers(&x->layer[l], x->p, t, x->dpi, l ? PAGE_LAYER_INK : PAGE_LAYER_ALL, x->drop, -1, &x->ps);
    for (int by = my0; by <= my1; by++) memset(&ready[by * x->blocks_x + mx0], 1, (size_t)(mx1 - mx0 + 1));
}

// Whether leaving out stroke k too (besides the marked ones) changes r, a rect on the page, in layer
// l: 1 if it does, 0 if not, -1 out of memory. Draws r without them over the reference, compares
// the two and puts the reference back.
static int differs_in(Compact *x, int l, Rect r, int k) {
    size_t need = (size_t)r.w * (size_t)r.h;
    if (need > x->keep_cap) {
        uint8_t *nk = realloc(x->keep, need);
        if (!nk) return -1;
        x->keep = nk;
        x->keep_cap = need;
    }
    Canvas *c = &x->layer[l];
    ref_ready(x, l, r);
    for (int y = 0; y < r.h; y++)
        memcpy(x->keep + (size_t)y * r.w, &c->px[(r.y + y) * c->stride + r.x], (size_t)r.w);
    if (l) canvas_fill(c, r, 255);
    render_layers(c, x->p, r, x->dpi, l ? PAGE_LAYER_INK : PAGE_LAYER_ALL, x->drop, k, &x->ps);
    bool same = true;
    for (int y = 0; y < r.h; y++) {
        uint8_t *row = &c->px[(r.y + y) * c->stride + r.x];
        const uint8_t *was = x->keep + (size_t)y * r.w;
        same = same && !memcmp(row, was, (size_t)r.w);
        memcpy(row, was, (size_t)r.w);
    }
    return !same;
}

// Whether row y of the paper has marks in the column blocks r spans.
static bool row_marked(const Compact *x, int y, Rect r) {
    const uint8_t *m = &x->marked[(size_t)y * (size_t)x->blocks_x];
    for (int bx = r.x / REF_BLOCK; bx <= (r.x + r.w - 1) / REF_BLOCK; bx++)
        if (m[bx]) return true;
    return false;
}

// The same for both layers: the page over r, then the ink layer over the runs of rows of r where
// the paper has marks.
static int differs(Compact *x, Rect r, int k) {
    int d = differs_in(x, 0, r, k);
    for (int y = r.y; x->ink_too && !d && y < r.y + r.h; y++) {
        if (!row_marked(x, y, r)) continue;
        int end = y + 1;
        while (end < r.y + r.h && row_marked(x, end, r)) end++;
        d = differs_in(x, 1, (Rect){r.x, y, r.w, end - y}, k);
        y = end;
    }
    return d;
}

// Whether an erase stroke after stroke i reaches into r.
static bool erased_after(const Page *p, int i, Rect r) {
    for (int j = i + 1; j < p->n; j++)
        if (stroke_is_erase(p->strokes[j]) && rect_overlaps(p->strokes[j]->bbox, r)) return true;
    return false;
}

// Whether leaving out ink stroke i changes the page (1, 0, or -1 out of memory). It can change only
// what its pieces draw over, so each piece's rect is checked. Pieces no later erase stroke reaches
// go first: they show unless later ink covers them, so a stroke that stays is told at once.
static int ink_matters(Compact *x, int i) {
    int np;
    const Piece *pcs = pieces_of(&x->ps, i, &np);
    if (!pcs) return -1;
    for (int pass = 0; pass < 2; pass++)
        for (int k = 0; k < np; k++) {
            Rect r = pcs[k].box;
            if (rect_is_empty(r) || erased_after(x->p, i, r) != (pass == 1)) continue;
            int d = differs(x, r, i);
            if (d) return d;
        }
    return 0;
}

// The same for erase stroke j. It only lightens, and a pixel no ink before it darkened is at its
// paper already, so it can change only where its pieces meet the pieces of earlier ink that is
// not marked (inside that ink's bbox): those rects are checked, and an erase stroke over nothing
// else is marked without drawing anything.
static int erase_matters(Compact *x, int j) {
    const Page *p = x->p;
    int np;
    if (!pieces_of(&x->ps, j, &np)) return -1;
    for (int k = 0; k < np; k++) {
        Rect box = x->ps.pc[j][k].box, r = RECT_EMPTY;
        for (int i = 0; i < j && !rect_is_empty(box); i++) {
            const Stroke *s = p->strokes[i];
            bool ink = !stroke_is_erase(s) && (unsigned)s->tool < TOOL_COUNT;
            if (x->drop[i] || !ink || !rect_overlaps(s->bbox, box)) continue;
            int ni;
            const Piece *pi = pieces_of(&x->ps, i, &ni);
            if (!pi) return -1;
            for (int m = 0; m < ni; m++) r = rect_union(r, rect_intersect(box, pi[m].box));
        }
        if (rect_is_empty(r)) continue;
        int d = differs(x, r, j);
        if (d) return d;
    }
    return 0;
}

// Gets x ready: room for the pieces, the reference canvases (nothing drawn in them yet) and
// whether the paper is white all over. 0, or -1 out of memory.
static int compact_start(Compact *x) {
    const Page *p = x->p;
    x->ps = (Pieces){.p = p, .dpi = x->dpi};
    x->ps.pc = calloc((size_t)p->n, sizeof *x->ps.pc);
    x->ps.n = calloc((size_t)p->n, sizeof *x->ps.n);
    if (!x->ps.pc || !x->ps.n || canvas_init(&x->layer[0], p->w, p->h) != 0) return -1;
    x->blocks_x = (p->w + REF_BLOCK - 1) / REF_BLOCK;
    size_t blocks = (size_t)x->blocks_x * (size_t)((p->h + REF_BLOCK - 1) / REF_BLOCK);
    x->ready[0] = calloc(blocks, 1);
    x->ready[1] = calloc(blocks, 1);
    x->marked = calloc((size_t)x->blocks_x * (size_t)p->h, 1);
    if (!x->ready[0] || !x->ready[1] || !x->marked) return -1;
    // Where the paper has marks (the reference is drawn over it later).
    paper_render(&x->layer[0], p->paper, (Rect){0, 0, p->w, p->h}, x->dpi);
    for (int y = 0; y < p->h; y++) {
        const uint8_t *row = &x->layer[0].px[y * x->layer[0].stride];
        uint8_t *m = &x->marked[(size_t)y * (size_t)x->blocks_x];
        for (int k = 0; k < p->w; k++)
            if (row[k] != 255) m[k / REF_BLOCK] = x->ink_too = true;
    }
    return x->ink_too && canvas_init(&x->layer[1], p->w, p->h) != 0 ? -1 : 0;
}

static void compact_free(Compact *x) {
    for (int i = 0; x->ps.pc && i < x->p->n; i++) free(x->ps.pc[i]);
    free(x->ps.pc);
    free(x->ps.n);
    free(x->ps.tmp);
    for (int l = 0; l < 2; l++) {
        canvas_free(&x->layer[l]);
        free(x->ready[l]);
    }
    free(x->marked);
    free(x->keep);
}

int page_droppable(const Page *p, float dpi, bool *drop) {
    bool any = false;
    for (int i = 0; i < p->n; i++) {
        drop[i] = false;
        any = any || stroke_is_erase(p->strokes[i]);
    }
    if (!any || p->w <= 0 || p->h <= 0) return 0;
    // A stroke whose leaving out changes neither the page nor its ink layer is marked. Marked
    // strokes change nothing, so the page drawn without them is the page itself, and each check
    // sees the ones marked before it. Ink goes first: once it is out, the erase strokes over it may
    // have nothing left to change.
    Compact x = {.p = p, .dpi = dpi, .drop = drop};
    const Rect page = {0, 0, p->w, p->h};
    int n = compact_start(&x) == 0 ? 0 : -1;
    for (int pass = 0; pass < 2 && n >= 0; pass++)
        for (int i = 0; i < p->n && n >= 0; i++) {
            const Stroke *s = p->strokes[i];
            if ((unsigned)s->tool >= TOOL_COUNT || stroke_is_erase(s) != (pass == 1)) continue;
            if (pass == 0 && !erased_after(p, i, s->bbox)) continue;  // only erasing makes ink go
            if (rect_is_empty(rect_intersect(s->bbox, page))) continue;
            int d = pass ? erase_matters(&x, i) : ink_matters(&x, i);
            if (d < 0) {
                n = -1;
            } else if (!d) {
                drop[i] = true;
                n++;
            }
        }
    compact_free(&x);
    if (n < 0) memset(drop, 0, (size_t)p->n * sizeof *drop);
    return n;
}
