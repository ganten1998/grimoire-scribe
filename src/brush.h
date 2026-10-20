// brush.h: drawing tools, their tweakable sensitivity, and the deterministic dab renderer.
#pragma once
#include "canvas.h"

// Stored on disk: never reorder.
typedef enum {
    TOOL_PENCIL = 0,
    TOOL_MECH_PENCIL = 1,
    TOOL_BALLPOINT = 2,
    TOOL_ERASER = 3,
    TOOL_HIGHLIGHTER = 4,
    TOOL_FOUNTAIN = 5,     // italic nib: thick/thin by stroke direction
    TOOL_FINELINER = 6,    // constant thin crisp line
    TOOL_MARKER = 7,       // broad felt tip
    TOOL_CHARCOAL = 8,     // soft stick: coarse grain, crumbly edges, broad on its side
    TOOL_CALLIGRAPHY = 9,  // broad-edge nib held at 90 degrees to the pen's lean
    TOOL_COUNT
} ToolKind;

const char *tool_name(ToolKind t);   // "Pencil", "Mechanical pencil", "Ballpoint", "Eraser", "Highlighter"...
const char *tool_label(ToolKind t);  // the same in the language in use, for the screens

// Every field is user-tweakable per tool (pensettings.c draws one slider per entry of
// tool_param_table). Stored in settings.ini and snapshotted into each stroke.
typedef struct {
    float size_min_mm;       // width at zero (post-curve) pressure
    float size_max_mm;       // width at full pressure
    float opacity_min;       // darkness 0..1 at zero pressure
    float opacity_max;       // darkness 0..1 at full pressure
    float pressure_floor;    // raw pressure 0..1 below which the pen reads as 0 (deadzone)
    float pressure_ceiling;  // raw pressure 0..1 at/above which it reads as full
    float pressure_gamma;    // curve: out = in^gamma (<1 = soft/sensitive, >1 = firm)
    float tilt_min_deg;      // tilt from vertical below which tilt counts as 0
    float tilt_max_deg;      // tilt at/above which tilt counts as 1
    float tilt_size;         // width *= 1 + tilt_size * tilt  (shading with the side)
    float tilt_opacity;      // opacity *= 1 - tilt_opacity * tilt
    float grain;             // 0..1 paper-tooth texture
    float smoothing;         // 0..1 input smoothing (applied before storing points)
} ToolParams;

typedef struct {
    const char *key;     // settings.ini key, e.g. "pressure_gamma"
    const char *label;   // UI label, e.g. "Pressure curve"
    const char *unit;    // "mm", "deg", "" ...
    size_t offset;       // offsetof(ToolParams, field)
    float min, max, step;
    bool eraser_uses;    // shown on the eraser tab (size only)
} ParamInfo;

const ParamInfo *tool_param_table(int *count);
void tool_params_default(ToolKind t, ToolParams *out);
static inline float *tool_param_ptr(ToolParams *p, const ParamInfo *pi) {
    return (float *)((char *)p + pi->offset);
}

typedef struct {
    float x, y;          // page px
    float pressure;      // raw normalized 0..1 (device ABS_PRESSURE / max)
    float tilt_x, tilt_y;// degrees, 0 = vertical; 0,0 if the digitizer lacks tilt. In the page's
                         // frame: positive = the pen's top leans toward +x (right) / +y (down)
    uint32_t t_ms;
} PenSample;

float brush_pressure(const ToolParams *p, float raw);           // -> 0..1
float brush_tilt(const ToolParams *p, float tilt_x, float tilt_y); // -> 0..1
// Dab radius (px) and opacity (0..1) for a sample.
void brush_dab(ToolKind t, const ToolParams *p, const PenSample *s, float dpi, float *radius_px, float *opacity);
// Largest radius any sample of this tool with these params can produce (for bbox padding).
float brush_max_radius(ToolKind t, const ToolParams *p, float dpi);
// Calligraphy: the nib's pen angle for a pen tilted (tilt_x, tilt_y), in degrees above the
// horizontal (45 = the edge rises to the right at 45 degrees). The broad edge sits at 90
// degrees to the pen's lean; a pen held nearly upright gets the default 45 (leaning bottom right).
float brush_nib_angle(const ToolParams *p, float tilt_x, float tilt_y);

// Incremental renderer. Live drawing and re-rendering both go through brush_add so the
// results are pixel-identical. Ink color is fixed per tool (pencil graphite ~40,
// mech ~30, ballpoint ~10).
typedef struct {
    ToolKind tool;
    ToolParams params;
    uint32_t seed;
    float dpi;
    bool have_prev;
    PenSample prev;
    float carry;         // distance carried to the next dab
    uint32_t dab_index;
    bool nib_init;       // calligraphy: smoothed nib orientation as (cos, sin) of twice the
    float nib_c, nib_s;  // edge angle, so an edge and its reverse are the same orientation
} BrushState;

void brush_begin(BrushState *b, ToolKind t, const ToolParams *p, uint32_t seed, float dpi);
// Renders dabs from the previous sample to s (the first call stamps one dab at s).
// Returns the dirty rect touched (may be empty).
Rect brush_add(BrushState *b, Canvas *c, const PenSample *s);

// Input smoothing (exponential, distance-aware). amount = ToolParams.smoothing.
typedef struct {
    bool init;
    float x, y, pressure;
} Smoother;
void smoother_reset(Smoother *s);
void smoother_apply(Smoother *s, float amount, PenSample *io);
