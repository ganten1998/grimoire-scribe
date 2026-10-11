// brush.c: tools, sensitivity mapping and the deterministic dab renderer.
#include "brush.h"

#include <math.h>
#include <stdlib.h>

const char *tool_name(ToolKind t) {
    switch (t) {
    case TOOL_PENCIL: return "Pencil";
    case TOOL_MECH_PENCIL: return "Mechanical pencil";
    case TOOL_BALLPOINT: return "Ballpoint";
    case TOOL_ERASER: return "Eraser";
    case TOOL_HIGHLIGHTER: return "Highlighter";
    default: return "Tool";
    }
}

#define P(field, key, label, unit, lo, hi, st, er) \
    {key, label, unit, offsetof(ToolParams, field), lo, hi, st, er}
static const ParamInfo k_params[] = {
    P(size_min_mm, "size_min_mm", "Min width", "mm", 0.10f, 12.f, 0.01f, true),
    P(size_max_mm, "size_max_mm", "Max width", "mm", 0.10f, 12.f, 0.01f, true),
    P(opacity_min, "opacity_min", "Light darkness", "", 0.f, 1.f, 0.01f, false),
    P(opacity_max, "opacity_max", "Full darkness", "", 0.f, 1.f, 0.01f, false),
    P(pressure_floor, "pressure_floor", "Pressure deadzone", "", 0.f, 0.5f, 0.01f, false),
    P(pressure_ceiling, "pressure_ceiling", "Pressure full at", "", 0.05f, 1.f, 0.01f, false),
    P(pressure_gamma, "pressure_gamma", "Pressure curve", "", 0.25f, 4.f, 0.05f, false),
    P(tilt_min_deg, "tilt_min_deg", "Tilt starts at", "deg", 0.f, 85.f, 1.f, false),
    P(tilt_max_deg, "tilt_max_deg", "Tilt full at", "deg", 5.f, 90.f, 1.f, false),
    P(tilt_size, "tilt_size", "Tilt widens", "x", 0.f, 4.f, 0.05f, false),
    P(tilt_opacity, "tilt_opacity", "Tilt lightens", "", 0.f, 1.f, 0.01f, false),
    P(grain, "grain", "Grain", "", 0.f, 1.f, 0.01f, false),
    P(smoothing, "smoothing", "Smoothing", "", 0.f, 1.f, 0.01f, false),
};
#undef P
_Static_assert(sizeof k_params / sizeof k_params[0] * sizeof(float) == sizeof(ToolParams),
               "param table must cover every ToolParams field");

const ParamInfo *tool_param_table(int *count) {
    if (count) *count = (int)(sizeof k_params / sizeof k_params[0]);
    return k_params;
}

void tool_params_default(ToolKind t, ToolParams *o) {
    switch (t) {
    case TOOL_MECH_PENCIL:
        *o = (ToolParams){0.26f, 0.36f, 0.38f, 0.85f, 0.03f, 0.80f, 1.10f, 25.f, 70.f, 0.35f, 0.10f, 0.50f, 0.15f};
        break;
    case TOOL_BALLPOINT:
        *o = (ToolParams){0.30f, 0.48f, 0.85f, 1.00f, 0.02f, 0.55f, 0.60f, 30.f, 75.f, 0.10f, 0.00f, 0.06f, 0.35f};
        break;
    case TOOL_ERASER:
        *o = (ToolParams){2.5f, 7.0f, 1.f, 1.f, 0.0f, 0.60f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.f};
        break;
    case TOOL_HIGHLIGHTER:
        // Broad and flat; darken-only blending keeps the writing underneath fully black.
        *o = (ToolParams){4.0f, 5.0f, 0.55f, 0.65f, 0.02f, 0.50f, 0.80f, 30.f, 70.f, 0.2f, 0.f, 0.f, 0.40f};
        break;
    case TOOL_PENCIL:
    default:
        *o = (ToolParams){0.30f, 0.85f, 0.30f, 0.95f, 0.03f, 0.85f, 1.25f, 15.f, 60.f, 2.6f, 0.45f, 0.65f, 0.12f};
        break;
    }
}

// Per-tool look that is not user-tweakable.
typedef struct {
    uint8_t ink;
    float rim;     // fraction of the radius that fades out
    float core;    // extra darkness toward the center (ballpoint)
    float jitter;  // per-dab darkness variation, scaled by grain
    bool graphite; // paper-tooth deposition with ragged edges (pencils)
} ToolLook;

static ToolLook look_of(ToolKind t) {
    switch (t) {
    case TOOL_MECH_PENCIL: return (ToolLook){32, 0.20f, 0.f, 0.06f, true};
    case TOOL_BALLPOINT: return (ToolLook){12, 0.10f, 0.35f, 0.f, false};
    case TOOL_HIGHLIGHTER: return (ToolLook){110, 0.06f, 0.f, 0.f, false};
    default: return (ToolLook){38, 0.35f, 0.f, 0.10f, true};
    }
}

static float sane(float v, float lo, float hi, float dflt) {
    if (!(v == v)) return dflt;  // NaN
    return clampf(v, lo, hi);
}

float brush_pressure(const ToolParams *p, float raw) {
    float lo = sane(p->pressure_floor, 0.f, 0.5f, 0.f);
    float hi = sane(p->pressure_ceiling, 0.05f, 1.f, 1.f);
    if (hi <= lo + 0.01f) hi = lo + 0.01f;
    float x = clampf((sane(raw, 0.f, 1.f, 0.f) - lo) / (hi - lo), 0.f, 1.f);
    return powf(x, sane(p->pressure_gamma, 0.25f, 4.f, 1.f));
}

float brush_tilt(const ToolParams *p, float tx, float ty) {
    tx = sane(tx, -89.f, 89.f, 0.f) * (float)M_PI / 180.f;
    ty = sane(ty, -89.f, 89.f, 0.f) * (float)M_PI / 180.f;
    float a = tanf(tx), b = tanf(ty);
    float deg = atanf(sqrtf(a * a + b * b)) * 180.f / (float)M_PI;
    float lo = sane(p->tilt_min_deg, 0.f, 85.f, 30.f), hi = sane(p->tilt_max_deg, 5.f, 90.f, 70.f);
    if (hi <= lo + 1.f) hi = lo + 1.f;
    float x = clampf((deg - lo) / (hi - lo), 0.f, 1.f);
    return x * x * (3.f - 2.f * x);  // smoothstep
}

void brush_dab(ToolKind t, const ToolParams *p, const PenSample *s, float dpi, float *radius_px, float *opacity) {
    float floor_ = sane(p->pressure_floor, 0.f, 0.5f, 0.f);
    float pr = brush_pressure(p, s->pressure);
    float tl = brush_tilt(p, s->tilt_x, s->tilt_y);
    float w = lerpf(sane(p->size_min_mm, 0.1f, 12.f, 0.3f), sane(p->size_max_mm, 0.1f, 12.f, 0.5f), pr);
    w *= 1.f + sane(p->tilt_size, 0.f, 4.f, 0.f) * tl;
    float r = mm_to_px(w, dpi) * 0.5f;
    *radius_px = r < 0.5f ? 0.5f : r;
    float o = lerpf(sane(p->opacity_min, 0.f, 1.f, 0.5f), sane(p->opacity_max, 0.f, 1.f, 1.f), pr);
    o *= 1.f - sane(p->tilt_opacity, 0.f, 1.f, 0.f) * tl;
    // Below the deadzone the pen is touching but does not mark.
    if (s->pressure < floor_) o = 0.f;
    *opacity = clampf(o, 0.f, 1.f);
    (void)t;
}

float brush_max_radius(const ToolParams *p, float dpi) {
    float w = fmaxf(sane(p->size_min_mm, 0.1f, 12.f, 0.3f), sane(p->size_max_mm, 0.1f, 12.f, 0.5f));
    w *= 1.f + sane(p->tilt_size, 0.f, 4.f, 0.f);
    // + 30%: graphite edges follow the paper tooth slightly past the nominal radius.
    float r = mm_to_px(w, dpi) * 0.5f * (1.f + 0.3f * sane(p->grain, 0.f, 1.f, 0.f));
    return r < 0.5f ? 0.5f : r;
}

// ---- paper tooth: a fixed, tiled value-noise texture over absolute page pixels ----

#define TOOTH_N 512
static uint8_t g_tooth[TOOTH_N * TOOTH_N];
static bool g_tooth_ready;

static uint32_t hash2(uint32_t x, uint32_t y) {
    uint32_t h = x * 374761393u + y * 668265263u;
    h = (h ^ (h >> 13)) * 1274126177u;
    return h ^ (h >> 16);
}

// Bilinear value noise in [0,1] with lattice cells of cw x ch pixels (wrapping on TOOTH_N).
static float vnoise(int x, int y, int cw, int ch, uint32_t salt) {
    int gx = x / cw, gy = y / ch;
    float fx = (float)(x % cw) / (float)cw, fy = (float)(y % ch) / (float)ch;
    int nx = TOOTH_N / cw, ny = TOOTH_N / ch;
    float v00 = (float)(hash2((uint32_t)(gx % nx), (uint32_t)(gy % ny) + salt) & 0xFFFF) / 65535.f;
    float v10 = (float)(hash2((uint32_t)((gx + 1) % nx), (uint32_t)(gy % ny) + salt) & 0xFFFF) / 65535.f;
    float v01 = (float)(hash2((uint32_t)(gx % nx), (uint32_t)((gy + 1) % ny) + salt) & 0xFFFF) / 65535.f;
    float v11 = (float)(hash2((uint32_t)((gx + 1) % nx), (uint32_t)((gy + 1) % ny) + salt) & 0xFFFF) / 65535.f;
    return lerpf(lerpf(v00, v10, fx), lerpf(v01, v11, fx), fy);
}

// Paper tooth: crisp per-pixel speckle, small grains and long horizontal fibers, contrast-
// stretched. A soft, blurry tooth is what made pencil look "bubbly"; graphite catches on fine,
// sharp-edged peaks of the paper.
static void tooth_init(void) {
    for (int y = 0; y < TOOTH_N; y++)
        for (int x = 0; x < TOOTH_N; x++) {
            float white = (float)(hash2((uint32_t)x, (uint32_t)y + 0x7F4Au) & 0xFFFF) / 65535.f;
            float grains = vnoise(x, y, 3, 2, 0x9E37u);
            float fibers = vnoise(x, y, 16, 2, 0x51EDu);
            float n = 0.42f * white + 0.33f * grains + 0.25f * fibers;
            n = clampf((n - 0.18f) / 0.64f, 0.f, 1.f);
            n = n * n * (3.f - 2.f * n);
            g_tooth[y * TOOTH_N + x] = (uint8_t)lrintf(n * 255.f);
        }
    g_tooth_ready = true;
}

static inline float tooth_at(int x, int y) {
    return (float)g_tooth[((unsigned)y & (TOOTH_N - 1)) * TOOTH_N + ((unsigned)x & (TOOTH_N - 1))] / 255.f;
}

// ---- renderer ----

void brush_begin(BrushState *b, ToolKind t, const ToolParams *p, uint32_t seed, float dpi) {
    if (!g_tooth_ready) tooth_init();
    *b = (BrushState){0};
    b->tool = t;
    b->params = *p;
    b->seed = seed;
    b->dpi = (dpi > 50.f && dpi < 2000.f) ? dpi : 300.f;
}

static float spacing_for(float r) { return fmaxf(0.5f, 0.15f * r); }

static Rect stamp(BrushState *b, Canvas *c, const PenSample *s) {
    float r, op;
    brush_dab(b->tool, &b->params, s, b->dpi, &r, &op);
    uint32_t idx = b->dab_index++;
    if (op <= 0.f) return RECT_EMPTY;
    ToolLook lk = look_of(b->tool);
    float grain = sane(b->params.grain, 0.f, 1.f, 0.f);
    float pr = brush_pressure(&b->params, s->pressure);
    if (lk.jitter > 0.f) {
        float j = (float)(hash2(b->seed, idx) & 0xFFFF) / 65535.f - 0.5f;
        op = clampf(op * (1.f + 2.f * j * lk.jitter * grain), 0.f, 1.f);
    }
    // Spacing compensation: about 2r/spacing dabs cover a pixel on the stroke's spine, so
    // each dab deposits the fraction that composes to `op` overall.
    float n = fmaxf(1.f, 2.f * r / spacing_for(r));
    float inv_n = 1.f / n;
    float g_eff = grain * (1.f - 0.5f * pr);
    float r_in = fmaxf(0.f, r * (1.f - lk.rim) - 0.5f);
    float r_out = r + 0.5f;
    // Graphite: light pressure only reaches the peaks of the tooth, pressing harder fills the
    // valleys; the stroke edge follows the tooth too, so it is ragged instead of a smooth tube.
    float thr = grain * (0.80f - 0.70f * pr);
    float edge_w = fmaxf(0.9f, r * 0.22f);
    float rough = r * 0.6f * grain;
    Rect box = rect_intersect(rect_from_points(s->x, s->y, s->x, s->y, r + 1.f + (lk.graphite ? rough * 0.5f : 0.f)), c->clip);
    for (int y = box.y; y < box.y + box.h; y++) {
        float dy = (float)y + 0.5f - s->y;
        for (int x = box.x; x < box.x + box.w; x++) {
            float dx = (float)x + 0.5f - s->x;
            float d = sqrtf(dx * dx + dy * dy);
            float a;
            if (lk.graphite) {
                float t = tooth_at(x, y);
                float de = d + (0.5f - t) * rough;  // peaks pick up graphite a little further out
                if (de >= r_out) continue;
                float prof = clampf((r_out - de) / edge_w, 0.f, 1.f);
                float lo = thr - 0.16f, hi = thr + 0.16f;
                float k = clampf((t - lo) / (hi - lo), 0.f, 1.f);
                k = k * k * (3.f - 2.f * k);
                a = op * prof * lerpf(1.f, k, grain);
            } else {
                if (d >= r_out) continue;
                float prof = d <= r_in ? 1.f : (r_out - d) / (r_out - r_in);
                if (lk.core > 0.f) prof = fminf(1.f, prof * (1.f + lk.core * (1.f - d / r_out)));
                a = op * prof;
                if (g_eff > 0.f) {
                    float v = 1.f - tooth_at(x, y);
                    a *= clampf(1.f - g_eff * 1.4f * v * v, 0.f, 1.f);
                }
            }
            if (a <= 0.002f) continue;
            float ad = 1.f - powf(1.f - fminf(a, 0.999f), inv_n);
            canvas_deposit_px(c, x, y, lk.ink, ad);
        }
    }
    return box;
}

static PenSample interp(const PenSample *a, const PenSample *b, float t) {
    PenSample s;
    s.x = lerpf(a->x, b->x, t);
    s.y = lerpf(a->y, b->y, t);
    s.pressure = lerpf(a->pressure, b->pressure, t);
    s.tilt_x = lerpf(a->tilt_x, b->tilt_x, t);
    s.tilt_y = lerpf(a->tilt_y, b->tilt_y, t);
    s.t_ms = a->t_ms + (uint32_t)((float)(b->t_ms - a->t_ms) * t);
    return s;
}

Rect brush_add(BrushState *b, Canvas *c, const PenSample *s) {
    if (b->tool == TOOL_ERASER) {
        b->prev = *s;
        b->have_prev = true;
        return RECT_EMPTY;
    }
    Rect dirty = RECT_EMPTY;
    if (!b->have_prev) {
        dirty = stamp(b, c, s);
        float r, op;
        brush_dab(b->tool, &b->params, s, b->dpi, &r, &op);
        b->carry = spacing_for(r);
        b->prev = *s;
        b->have_prev = true;
        return dirty;
    }
    float dx = s->x - b->prev.x, dy = s->y - b->prev.y;
    float len = sqrtf(dx * dx + dy * dy);
    if (len > 20000.f) {  // corrupt data: do not bridge with ink
        b->prev = *s;
        return dirty;
    }
    float d = b->carry;
    while (d <= len) {
        PenSample m = interp(&b->prev, s, len > 0.f ? d / len : 1.f);
        dirty = rect_union(dirty, stamp(b, c, &m));
        float r, op;
        brush_dab(b->tool, &b->params, &m, b->dpi, &r, &op);
        d += spacing_for(r);
    }
    b->carry = d - len;
    b->prev = *s;
    return dirty;
}

void smoother_reset(Smoother *s) { *s = (Smoother){0}; }

void smoother_apply(Smoother *s, float amount, PenSample *io) {
    amount = clampf(amount, 0.f, 1.f);
    if (!s->init || amount <= 0.f) {
        s->init = true;
        s->x = io->x;
        s->y = io->y;
        s->pressure = io->pressure;
        return;
    }
    float dx = io->x - s->x, dy = io->y - s->y;
    float dist = sqrtf(dx * dx + dy * dy);
    // Strong smoothing for slow, careful motion; quick strokes follow closely.
    float alpha = clampf(1.f - 0.85f * amount + dist / 60.f, 0.08f, 1.f);
    s->x += dx * alpha;
    s->y += dy * alpha;
    s->pressure += (io->pressure - s->pressure) * fmaxf(alpha, 0.5f);
    io->x = s->x;
    io->y = s->y;
    io->pressure = s->pressure;
}
