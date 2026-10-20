// page.h: strokes and pages.
#pragma once
#include "brush.h"
#include "paper.h"

typedef struct {
    ToolKind tool;
    ToolParams params;   // snapshot at draw time
    uint32_t seed;
    uint8_t ext[3];      // the record's spare bytes, carried as read: colour, layer, flags in a
                         // newer version (docs/FORMAT.md); this one reads none of them
    int n, cap;
    PenSample *pts;
    Rect bbox;           // covers every rendered pixel (points + max radius + 2)
} Stroke;

typedef struct {
    PaperKind paper;     // may be a kind this build does not know (paper_is_known): drawn blank, kept
    int w, h;            // page px
    int n, cap;
    Stroke **strokes;    // owned, draw order
    uint8_t *tail;       // chunks after the last stroke, written back as they were (owned)
    size_t tail_len;
    bool readonly;       // from a newer Grimoire (or not all readable): shown, never changed or written
} Page;

Stroke *stroke_new(ToolKind t, const ToolParams *p, uint32_t seed);
void stroke_free(Stroke *s);
void stroke_append(Stroke *s, const PenSample *pt, float dpi);   // grows bbox
// True if (x,y) lies within r px of the rendered stroke (polyline distance <= r + half width).
bool stroke_hit(const Stroke *s, float x, float y, float r, float dpi);
// True if the segment (x0,y0)-(x1,y1) swept with radius r touches the stroke.
bool stroke_hit_segment(const Stroke *s, float x0, float y0, float x1, float y1, float r, float dpi);

Page *page_new(int w, int h, PaperKind paper);    // any paper value is kept as it is
void page_free(Page *p);                          // frees strokes and tail too
void page_add(Page *p, Stroke *s);                // takes ownership, appends
void page_insert(Page *p, int index, Stroke *s);  // takes ownership
Stroke *page_remove(Page *p, int index);          // returns ownership
void page_clear_strokes(Page *p);

// Renders paper + every stroke intersecting region, clipped to region. Strokes are
// replayed from their first sample with brush_add so the result equals the live render.
// A stroke with a tool this build does not know is never drawn.
void page_render(Canvas *c, const Page *p, Rect region, float dpi);
// The same with either layer left out (the canvas keeps what it had there, white if fresh):
// the share studio recolors paper and ink separately.
enum { PAGE_LAYER_PAPER = 1, PAGE_LAYER_INK = 2, PAGE_LAYER_ALL = 3 };
void page_render_layers(Canvas *c, const Page *p, Rect region, float dpi, int layers);
// Renders the whole page then box-downsamples into thumb (thumb size sets the scale).
void page_render_thumb(Canvas *thumb, const Page *p, float dpi);
