// page.h: strokes and pages.
#pragma once
#include "brush.h"
#include "paper.h"

typedef struct {
    ToolKind tool;
    ToolParams params;   // snapshot at draw time
    uint32_t seed;       // fixed from stroke_new on: an erase stroke's texture is its top byte
    uint8_t ext[3];      // the record's spare bytes, carried as read: colour, layer, flags in a
                         // newer version (docs/FORMAT.md); this one reads none of them
    int n, cap;
    PenSample *pts;
    Rect bbox;           // covers every rendered pixel (points + the render tool's max radius + 2)
} Stroke;

// Erase strokes (tool TOOL_ERASER) restore the paper under them instead of depositing ink: white
// ink with a drawing tool's texture and settings, or the precise eraser's round hard tip
// (docs/FORMAT.md, Erase strokes). The seed's top byte holds the texture, the tool + 1 (0 = the
// round tip), and the brush hashes only its low 24 bits.
static inline bool stroke_is_erase(const Stroke *s) { return s->tool == TOOL_ERASER; }
// The tool whose look and reach a stroke draws with: its own, or an erase stroke's texture
// (TOOL_ERASER, the round tip, also for a texture this version does not have). It comes from the
// seed, so every stroke knows it before its first point: the reader, scale_page and the editor.
ToolKind stroke_render_tool(const Stroke *s);
// The seed for a new erase stroke: the texture (a drawing tool, or TOOL_ERASER for the round tip;
// anything else is the round tip too) in the top byte, the low 24 bits of hash below it.
uint32_t erase_seed(ToolKind texture, uint32_t hash);
// Starts b to draw s, live or replayed: one start for both, so they give the same pixels. An
// erase stroke's brush restores toward the paper given (as in brush_set_erase, NULL = white),
// which must cover the canvas's clip while it draws; an ink stroke's ignores it.
void stroke_brush_begin(BrushState *b, const Stroke *s, float dpi, const uint8_t *paper, int x0, int y0, int stride);

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
void stroke_append(Stroke *s, const PenSample *pt, float dpi);   // grows bbox (by the render tool)
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
// A stroke with a tool this build does not know is never drawn. Erase strokes restore the paper
// drawn here (a copy of it where they reach into the region).
void page_render(Canvas *c, const Page *p, Rect region, float dpi);
// The same with either layer left out (the canvas keeps what it had there, white if fresh):
// the share studio recolors paper and ink separately. Without the paper layer, erase strokes
// restore toward white, which is right on a fresh white canvas only: never draw a page in two
// passes (the paper layer, then the ink layer over it), as that wipes the paper's lines under
// every erase stroke. Draw both layers in one call.
enum { PAGE_LAYER_PAPER = 1, PAGE_LAYER_INK = 2, PAGE_LAYER_ALL = 3 };
void page_render_layers(Canvas *c, const Page *p, Rect region, float dpi, int layers);
// Renders the whole page then box-downsamples into thumb (thumb size sets the scale).
void page_render_thumb(Canvas *thumb, const Page *p, float dpi);

// Compaction, for privacy: erased ink stays in the page until it is dropped (zips and
// grimoire-pull copy the page files), and every erase stroke replays on each render. Marks
// drop[i] (p->n entries) for each stroke that can be left out without changing a pixel of the
// page or of its ink layer: ink that later erase strokes restore entirely, and erase strokes that
// change nothing. Each is checked with the ones marked before it already left out, so leave them
// out together. Returns how many, or -1 when out of memory (none marked). A page without erase
// strokes has none. p is not changed (the editor's undo history holds pointers to its strokes):
// leave the marked strokes out of a copy. It draws each candidate's bbox two to four times, in
// two page-size canvases: work for the moment a notebook closes, not for every save.
int page_droppable(const Page *p, float dpi, bool *drop);
