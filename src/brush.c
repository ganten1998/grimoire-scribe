// brush.c: tools, sensitivity mapping and the deterministic dab renderer.
#include "brush.h"

#include <math.h>
#include <stdlib.h>

#include "i18n.h"

// The English names go to grimoire.log too; tool_label is what the screens show.
const char *tool_name(ToolKind t) {
    switch (t) {
    // TRANSLATORS: the drawing tools: in the editor's tool picker (268 px at 34 px in Gothic, 284 in
    // Classic, where the Mechanical pencil and the Calligraphy pen go by shorter names) and after
    // "Pen feel:".
    case TOOL_PENCIL: return NC_("tool", "Pencil");
    case TOOL_MECH_PENCIL: return NC_("tool", "Mechanical pencil");
    case TOOL_BALLPOINT: return NC_("tool", "Ballpoint");
    case TOOL_ERASER: return NC_("tool", "Eraser");
    case TOOL_HIGHLIGHTER: return NC_("tool", "Highlighter");
    case TOOL_FOUNTAIN: return NC_("tool", "Fountain pen");
    case TOOL_FINELINER: return NC_("tool", "Fineliner");
    case TOOL_MARKER: return NC_("tool", "Marker");
    case TOOL_CHARCOAL: return NC_("tool", "Charcoal");
    case TOOL_CALLIGRAPHY: return NC_("tool", "Calligraphy pen");
    default: return NC_("tool", "Tool");
    }
}

const char *tool_label(ToolKind t) { return C_("tool", tool_name(t)); }

#define P(field, key, label, unit, lo, hi, st, er) \
    {key, label, unit, offsetof(ToolParams, field), lo, hi, st, er}
// The labels are looked up where Pen feel draws them (pensettings.c); keys and units stay English.
static const ParamInfo k_params[] = {
    // TRANSLATORS: Pen feel's sliders, each label left of its value, at 44 px: in Gothic about 530 px
    // beside a width (up to "12.00 mm"), 570 beside an angle, 630 beside a plain number ("1.25"); in
    // Classic 560, 600 and 640.
    P(size_min_mm, "size_min_mm", N_("Min width"), "mm", 0.10f, 12.f, 0.01f, true),
    P(size_max_mm, "size_max_mm", N_("Max width"), "mm", 0.10f, 12.f, 0.01f, true),
    P(opacity_min, "opacity_min", N_("Light darkness"), "", 0.f, 1.f, 0.01f, false),
    P(opacity_max, "opacity_max", N_("Full darkness"), "", 0.f, 1.f, 0.01f, false),
    P(pressure_floor, "pressure_floor", N_("Pressure deadzone"), "", 0.f, 0.5f, 0.01f, false),
    P(pressure_ceiling, "pressure_ceiling", N_("Pressure full at"), "", 0.05f, 1.f, 0.01f, false),
    P(pressure_gamma, "pressure_gamma", N_("Pressure curve"), "", 0.25f, 4.f, 0.05f, false),
    P(tilt_min_deg, "tilt_min_deg", N_("Tilt starts at"), "deg", 0.f, 85.f, 1.f, false),
    P(tilt_max_deg, "tilt_max_deg", N_("Tilt full at"), "deg", 5.f, 90.f, 1.f, false),
    P(tilt_size, "tilt_size", N_("Tilt widens"), "x", 0.f, 4.f, 0.05f, false),
    P(tilt_opacity, "tilt_opacity", N_("Tilt lightens"), "", 0.f, 1.f, 0.01f, false),
    P(grain, "grain", N_("Grain"), "", 0.f, 1.f, 0.01f, false),
    P(smoothing, "smoothing", N_("Smoothing"), "", 0.f, 1.f, 0.01f, false),
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
        *o = (ToolParams){0.30f, 0.30f, 0.50f, 0.95f, 0.00f, 1.00f, 1.50f, 0.f, 90.f, 0.f, 0.f, 0.50f, 0.f};
        break;
    case TOOL_BALLPOINT:
        *o = (ToolParams){0.30f, 0.48f, 0.85f, 1.00f, 0.00f, 1.00f, 1.50f, 0.f, 90.f, 0.f, 0.f, 0.f, 1.00f};
        break;
    case TOOL_ERASER:
        *o = (ToolParams){2.5f, 7.0f, 1.f, 1.f, 0.0f, 0.60f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.f};
        break;
    case TOOL_FOUNTAIN:
        // Nib width 0.24..0.60 mm with pressure; the angled nib makes it calligraphic.
        *o = (ToolParams){0.24f, 0.60f, 0.90f, 1.00f, 0.02f, 1.00f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.30f};
        break;
    case TOOL_FINELINER:
        *o = (ToolParams){0.28f, 0.34f, 1.00f, 1.00f, 0.01f, 0.30f, 1.00f, 30.f, 75.f, 0.f, 0.f, 0.f, 0.30f};
        break;
    case TOOL_MARKER:
        *o = (ToolParams){1.50f, 2.00f, 0.92f, 1.00f, 0.01f, 0.40f, 0.70f, 30.f, 75.f, 0.3f, 0.f, 0.04f, 0.30f};
        break;
    case TOOL_CHARCOAL:
        // 0.8..2.4 mm, strongly pressure-led; only a pen laid low (past normal writing tilt)
        // turns it on its side: much broader and lighter. Heavy grain.
        *o = (ToolParams){0.80f, 2.40f, 0.30f, 1.00f, 0.03f, 0.85f, 1.00f, 45.f, 75.f, 2.0f, 0.35f, 0.85f, 0.15f};
        break;
    case TOOL_CALLIGRAPHY:
        // A broad, flat nib with a constant 1.86 mm width. The tilt
        // range here says when the nib starts to follow the pen's lean (see brush_nib_angle).
        *o = (ToolParams){1.86f, 1.86f, 0.75f, 1.00f, 0.00f, 1.00f, 1.60f, 10.f, 25.f, 0.f, 0.f, 0.f, 1.00f};
        break;
    case TOOL_HIGHLIGHTER:
        // Broad and flat; darken-only blending keeps the writing underneath fully black.
        *o = (ToolParams){5.50f, 5.50f, 0.25f, 0.50f, 0.00f, 1.00f, 1.50f, 0.f, 90.f, 0.f, 0.f, 0.f, 0.f};
        break;
    case TOOL_PENCIL:
    default:
        // 0.12..0.32 mm: a sharp pencil; tilt still shades wider.
        *o = (ToolParams){0.12f, 0.32f, 0.75f, 0.95f, 0.00f, 0.85f, 0.40f, 15.f, 60.f, 1.8f, 0.45f, 1.00f, 0.00f};
        break;
    }
}

// Per-tool look that is not user-tweakable.
typedef enum { NIB_NONE = 0, NIB_ITALIC, NIB_BROAD_EDGE } NibKind;
typedef struct {
    uint8_t ink;
    float rim;     // fraction of the radius that fades out
    float core;    // extra darkness toward the center (ballpoint)
    float jitter;  // per-dab darkness variation, scaled by grain
    bool graphite; // paper-tooth deposition with ragged edges (pencils, charcoal)
    NibKind nib;   // italic: an ellipse at a fixed 40 degrees (fountain pen); broad edge: a
                   // square-cut flat nib turned by the pen's lean (calligraphy)
    // Graphite only (pencils use the defaults):
    bool coarse;   // tooth sampled at twice the scale: clumpy charcoal grit
    float rough;   // how far the edge follows the tooth (x radius x grain)
    float edge;    // soft edge width (x radius)
    float fill;    // how much full pressure fills the tooth's valleys
    float dust;    // chance of a loose speck just outside the edge
    float band;    // half-width of the tooth threshold's soft step (wider = grayer, less speckle)
} ToolLook;

static ToolLook look_of(ToolKind t) {
    ToolLook lk;
    switch (t) {
    case TOOL_MECH_PENCIL: lk = (ToolLook){32, 0.20f, 0.f, 0.06f, true}; break;
    case TOOL_BALLPOINT: lk = (ToolLook){12, 0.10f, 0.35f, 0.f, false}; break;
    case TOOL_HIGHLIGHTER: lk = (ToolLook){110, 0.06f, 0.f, 0.f, false}; break;
    case TOOL_FOUNTAIN: lk = (ToolLook){8, 0.10f, 0.f, 0.f, false, NIB_ITALIC}; break;
    case TOOL_CALLIGRAPHY: lk = (ToolLook){6, 0.08f, 0.f, 0.f, false, NIB_BROAD_EDGE}; break;
    case TOOL_FINELINER: lk = (ToolLook){5, 0.08f, 0.f, 0.f, false}; break;
    case TOOL_MARKER: lk = (ToolLook){22, 0.14f, 0.f, 0.f, false}; break;
    case TOOL_CHARCOAL: lk = (ToolLook){14, 0.f, 0.f, 0.12f, true, NIB_NONE, true, 0.95f, 0.32f, 0.50f, 0.05f, 0.24f}; break;
    // The precise eraser's round hard tip, for erase strokes only (it never deposits ink).
    case TOOL_ERASER: lk = (ToolLook){255, 0.08f, 0.f, 0.f, false}; break;
    default: lk = (ToolLook){38, 0.35f, 0.f, 0.10f, true}; break;
    }
    if (lk.graphite && !lk.coarse) {  // the pencils
        lk.rough = 0.6f;
        lk.edge = 0.22f;
        lk.fill = 0.70f;
        lk.band = 0.16f;
    }
    return lk;
}

// Nib shapes: long axis along the edge, the short one this fraction of it.
#define ITALIC_RATIO 0.22f
#define BROAD_RATIO 0.13f

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

float brush_max_radius(ToolKind t, const ToolParams *p, float dpi) {
    float w = fmaxf(sane(p->size_min_mm, 0.1f, 12.f, 0.3f), sane(p->size_max_mm, 0.1f, 12.f, 0.5f));
    w *= 1.f + sane(p->tilt_size, 0.f, 4.f, 0.f);
    // + 30%: graphite edges follow the paper tooth slightly past the nominal radius. Charcoal
    // reaches further: a rougher edge, and loose specks beyond it.
    float grain = sane(p->grain, 0.f, 1.f, 0.f);
    float k = t == TOOL_CHARCOAL ? 1.f + fmaxf(0.5f * 0.95f * grain, 0.40f) : 1.f + 0.3f * grain;
    float r = mm_to_px(w, dpi) * 0.5f * k + (t == TOOL_CHARCOAL ? 1.f : 0.f);
    return r < 0.5f ? 0.5f : r;
}

// Default lean for a pen held nearly upright: toward the bottom right, the way a right hand
// holds a calligraphy pen. As twice-the-edge-angle: the edge is at 90 degrees to the lean.
static void nib_target(const ToolParams *p, float tx, float ty, float *c2, float *s2) {
    // Lean direction on the page: the pen's top, projected (screen frame, y down).
    float lx = tanf(sane(tx, -89.f, 89.f, 0.f) * (float)M_PI / 180.f);
    float ly = tanf(sane(ty, -89.f, 89.f, 0.f) * (float)M_PI / 180.f);
    float w = brush_tilt(p, tx, ty);   // 0 = too upright to trust the lean
    float def2 = -0.5f * (float)M_PI;  // lean 45 deg down-right -> edge at -45 deg -> 2x = -90
    float a2 = def2;
    if (w > 0.f && (lx != 0.f || ly != 0.f)) {
        // Edge angle = lean + 90 deg, so twice it = 2 * lean + 180 deg.
        float lean2 = 2.f * atan2f(ly, lx) + (float)M_PI;
        float d = lean2 - def2;
        while (d > (float)M_PI) d -= 2.f * (float)M_PI;
        while (d <= -(float)M_PI) d += 2.f * (float)M_PI;
        a2 = def2 + w * d;  // along the shorter way round, so the blend never jumps
    }
    *c2 = cosf(a2);
    *s2 = sinf(a2);
}

float brush_nib_angle(const ToolParams *p, float tx, float ty) {
    float c2, s2;
    nib_target(p, tx, ty, &c2, &s2);
    // Screen angle of the edge (y down) -> pen angle above the horizontal, in 0..180.
    float deg = -0.5f * atan2f(s2, c2) * 180.f / (float)M_PI;
    while (deg < 0.f) deg += 180.f;
    while (deg >= 180.f) deg -= 180.f;
    return deg;
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

void brush_set_erase(BrushState *b, const uint8_t *paper, int x0, int y0, int stride) {
    b->erase = true;
    b->restored = false;
    b->paper = paper;
    b->paper_x0 = x0;
    b->paper_y0 = y0;
    b->paper_stride = stride;
}

bool g_brush_erase_spans = true;

static float spacing_for(float r) { return fmaxf(0.5f, 0.15f * r); }

// Where a dab's pixels go: ink deposited, or for an erase stroke the paper restored.
typedef struct {
    uint8_t ink;
    const uint8_t *paper;  // erase: NULL = white
    int x0, y0, stride;
    bool restored;         // erase: set once a pixel is lightened
} DabTarget;

// One pixel of a dab, the only place a dab writes one: a is the dab's own coverage, ad its share
// once the spacing is compensated (what the deposit uses). An erase dab restores toward paper, the
// paper under the pixel (stamp_as reads it).
static inline void put_px(DabTarget *t, Canvas *c, int x, int y, float ad, float a, bool erase, uint8_t paper) {
    if (!erase) {
        canvas_deposit_px(c, x, y, t->ink, ad);
        return;
    }
    const uint8_t *p = &c->px[y * c->stride + x];
    uint8_t was = *p;
    // Full coverage lands exactly on the paper: the spacing-compensated share alone would leave
    // the pixel one level short of it.
    canvas_restore_px(c, x, y, paper, a >= 0.999f ? 1.f : ad);
    if (*p != was) t->restored = true;
}

// The erase fast path's part of a row: pixels x0..x1 of row y become the lighter of themselves
// and the paper, which is what put_px makes of a pixel the dab covers fully.
static void restore_span(DabTarget *t, Canvas *c, int y, int x0, int x1) {
    uint8_t *row = &c->px[y * c->stride + x0];
    int n = x1 - x0 + 1;
    uint8_t changed = 0;
    if (!t->paper) {
        for (int i = 0; i < n; i++) {
            changed |= (uint8_t)~row[i];
            row[i] = 255;
        }
    } else {
        const uint8_t *pp = &t->paper[(ptrdiff_t)(y - t->y0) * t->stride + (x0 - t->x0)];
        for (int i = 0; i < n; i++) {
            uint8_t v = row[i] > pp[i] ? row[i] : pp[i];
            changed |= (uint8_t)(v ^ row[i]);
            row[i] = v;
        }
    }
    if (changed) t->restored = true;
}

// The fast path covers dabs up to this core radius: there the margin it leaves (below) is over
// twenty times the float rounding of the per-pixel distance. The largest dab the parameter ranges
// allow at 300 dpi has a radius of 354 px.
#define SPAN_MAX_R 512.f

// One dab. `erase` and `dry` are constants at each call (stamp), so each gets a copy of its own: the
// ink copy is the dab as it was before erase strokes, with no erase test left in its pixel loop.
static inline __attribute__((always_inline)) Rect stamp_as(BrushState *b, Canvas *c, const PenSample *s,
                                                           bool erase, bool dry) {
    float r, op;
    brush_dab(b->tool, &b->params, s, b->dpi, &r, &op);
    uint32_t idx = b->dab_index++;
    ToolLook lk = look_of(b->tool);
    // Calligraphy: the nib follows the pen's lean, lightly smoothed along the stroke (about
    // 1.5 mm), so sensor noise does not wobble the edge. Updated for every dab, inked or not.
    float nca = 0.7660444f, nsa = 0.6427876f;  // italic nib at 40 degrees (fountain pen)
    if (lk.nib == NIB_BROAD_EDGE) {
        float c2, s2;
        nib_target(&b->params, s->tilt_x, s->tilt_y, &c2, &s2);
        if (!b->nib_init) {
            b->nib_init = true;
            b->nib_c = c2;
            b->nib_s = s2;
        } else {
            float k = 1.f - expf(-spacing_for(r) / mm_to_px(1.5f, b->dpi));
            float nc = b->nib_c + (c2 - b->nib_c) * k, ns = b->nib_s + (s2 - b->nib_s) * k;
            float l = sqrtf(nc * nc + ns * ns);
            if (l > 1e-3f) {
                b->nib_c = nc / l;
                b->nib_s = ns / l;
            } else {
                b->nib_c = c2;
                b->nib_s = s2;
            }
        }
        float a = 0.5f * atan2f(b->nib_s, b->nib_c);
        nca = cosf(a);
        nsa = sinf(a);
    }
    if (op <= 0.f) return RECT_EMPTY;
    float grain = sane(b->params.grain, 0.f, 1.f, 0.f);
    float pr = brush_pressure(&b->params, s->pressure);
    if (lk.jitter > 0.f) {
        float j = (float)(hash2(b->seed, idx) & 0xFFFF) / 65535.f - 0.5f;
        op = clampf(op * (1.f + 2.f * j * lk.jitter * grain), 0.f, 1.f);
    }
    // Spacing compensation: about 2r/spacing dabs cover a pixel on the stroke's spine, so
    // each dab deposits the fraction that composes to `op` overall.
    // (A nib only overlaps itself across its narrow axis: its ratio of the round tip.)
    float nib_ratio = lk.nib == NIB_ITALIC ? ITALIC_RATIO : BROAD_RATIO;
    float n = fmaxf(1.f, 2.f * r * (lk.nib ? nib_ratio : 1.f) / spacing_for(r));
    float inv_n = 1.f / n;
    float g_eff = grain * (1.f - 0.5f * pr);
    float r_in = fmaxf(0.f, r * (1.f - lk.rim) - 0.5f);
    float r_out = r + 0.5f;
    // Graphite: light pressure only reaches the peaks of the tooth, pressing harder fills the
    // valleys; the stroke edge follows the tooth too, so it is ragged instead of a smooth tube.
    float thr = grain * (0.80f - lk.fill * pr);
    float edge_w = fmaxf(0.9f, r * lk.edge);
    float rough = r * lk.rough * grain;
    // Charcoal dust: loose specks up to this far past the edge, fixed to the paper.
    float dust_w = lk.dust > 0.f ? r * 0.40f : 0.f;
    float reach = lk.graphite ? fmaxf(rough * 0.5f, dust_w) : 0.f;
    Rect box = rect_intersect(rect_from_points(s->x, s->y, s->x, s->y, r + 1.f + reach), c->clip);
    if (dry) return box;  // where it would draw, and nothing drawn (BrushState.dry)
    DabTarget tg = {lk.ink, b->paper, b->paper_x0, b->paper_y0, b->paper_stride, false};
    // Erase fast path: a round dab (no nib, not graphite) with no grain at full opacity gives
    // every pixel of its core full coverage, so a row's core is restored in one pass (restore_span)
    // and the per-pixel loop skips it. The span stops a pixel short of the core's edge at both
    // ends, on rows whose core is wider than 4 px: its pixels are then at squared distance below
    // r_in^2 - 3, more than 1.4 / r_in inside the core, where the per-pixel path's float distance
    // cannot reach r_in, so it would have given each of them alpha 1 too. Byte for byte the same
    // (tests/test_erase.c compares the two).
    bool spans = erase && g_brush_erase_spans && !lk.graphite && lk.nib == NIB_NONE && g_eff <= 0.f &&
                 op >= 0.999f && r_in <= SPAN_MAX_R;
    for (int y = box.y; y < box.y + box.h; y++) {
        // Erase: the row, and the paper under it ((x, y) is inside the clip, stamp's box being
        // clipped to it, so the paper covers it).
        const uint8_t *row = &c->px[y * c->stride];
        const uint8_t *prow = erase && tg.paper ? &tg.paper[(ptrdiff_t)(y - tg.y0) * tg.stride] : NULL;
        float dy = (float)y + 0.5f - s->y;
        int sx0 = box.x + box.w, sx1 = sx0;  // the row's core span (none: x never gets there)
        float q = spans ? r_in * r_in - dy * dy : 0.f;
        if (q > 4.f) {
            float hw = sqrtf(q);
            int a0 = (int)ceilf(s->x - 0.5f - hw) + 1, a1 = (int)floorf(s->x - 0.5f + hw) - 1;
            if (a0 < box.x) a0 = box.x;
            if (a1 > box.x + box.w - 1) a1 = box.x + box.w - 1;
            if (a0 <= a1) {
                restore_span(&tg, c, y, a0, a1);
                sx0 = a0;
                sx1 = a1;
            }
        }
        for (int x = box.x; x < box.x + box.w; x++) {
            uint8_t paper = 255;
            if (erase) {
                if (x == sx0) {
                    x = sx1;
                    continue;
                }
                // A pixel already at its paper (or lighter) stays as it is (canvas_restore_px), so
                // its coverage need not be worked out: most of a scrub goes over pixels its earlier
                // dabs restored.
                if (prow) paper = prow[x - tg.x0];
                if (row[x] >= paper) continue;
            }
            float dx = (float)x + 0.5f - s->x;
            float d = sqrtf(dx * dx + dy * dy);
            float a;
            if (lk.graphite) {
                float t = lk.coarse ? 0.78f * tooth_at(x >> 1, y >> 1) + 0.22f * tooth_at(x, y) : tooth_at(x, y);
                float de = d + (0.5f - t) * rough;  // peaks pick up graphite a little further out
                if (de >= r_out) {
                    if (dust_w <= 0.f || de >= r_out + dust_w) continue;
                    // A speck where the paper catches it: rarer further from the edge.
                    float chance = lk.dust * (1.f - (de - r_out) / dust_w);
                    float h = (float)(hash2((uint32_t)x * 7u + b->seed, (uint32_t)y * 13u + 0xD057u) & 0xFFFF) / 65535.f;
                    if (h >= chance) continue;
                    a = op * 0.75f * (0.5f + 0.5f * t);
                    float ad = 1.f - powf(1.f - fminf(a, 0.999f), inv_n);
                    put_px(&tg, c, x, y, ad, a, erase, paper);
                    continue;
                }
                float prof = clampf((r_out - de) / edge_w, 0.f, 1.f);
                float lo = thr - lk.band, hi = thr + lk.band;
                float k = clampf((t - lo) / (hi - lo), 0.f, 1.f);
                k = k * k * (3.f - 2.f * k);
                a = op * prof * lerpf(1.f, k, grain);
            } else {
                if (lk.nib == NIB_ITALIC) {
                    // Italic nib at 40 degrees, 4.5:1: the distance is measured in the nib's frame.
                    float u = dx * nca + dy * nsa, v = (-dx * nsa + dy * nca) / ITALIC_RATIO;
                    d = sqrtf(u * u + v * v);
                } else if (lk.nib == NIB_BROAD_EDGE) {
                    // A flat, square-cut nib (a superellipse): crisp angled ends like a real
                    // broad-edge pen, hairlines along the edge, full width across it.
                    float u = dx * nca + dy * nsa, v = (-dx * nsa + dy * nca) / BROAD_RATIO;
                    float u2 = u * u, v2 = v * v;
                    d = sqrtf(sqrtf(u2 * u2 + v2 * v2));
                }
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
            put_px(&tg, c, x, y, ad, a, erase, paper);
        }
    }
    if (tg.restored) b->restored = true;
    return box;
}

static Rect stamp(BrushState *b, Canvas *c, const PenSample *s) {
    if (b->dry) return stamp_as(b, c, s, false, true);
    return b->erase ? stamp_as(b, c, s, true, false) : stamp_as(b, c, s, false, false);
}

static PenSample interp(const PenSample *a, const PenSample *b, float t) {
    PenSample s;
    s.x = lerpf(a->x, b->x, t);
    s.y = lerpf(a->y, b->y, t);
    s.pressure = lerpf(a->pressure, b->pressure, t);
    s.tilt_x = lerpf(a->tilt_x, b->tilt_x, t);
    s.tilt_y = lerpf(a->tilt_y, b->tilt_y, t);
    // In signed 64-bit: times read from a file may run backwards.
    s.t_ms = (uint32_t)((int64_t)a->t_ms + (int64_t)((double)((int64_t)b->t_ms - (int64_t)a->t_ms) * t));
    return s;
}

Rect brush_add(BrushState *b, Canvas *c, const PenSample *s) {
    if (b->tool == TOOL_ERASER && !b->erase) {
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
