// inkmode.h: what a stroke of the pen does in the editor, as pure logic (unit-tested on the host by
// tests/test_inkwire.c): ink, white ink, the precise eraser or the stroke eraser, from the settings
// (eraser_mode, white, side_button), the pen's eraser end and its side button; and the precise
// eraser's settings at a zoom. White ink and the precise eraser are erase strokes (src/page.h,
// docs/FORMAT.md): they restore the paper under them.
#pragma once
#include "store.h"

typedef enum {
    PEN_INK,      // ink with `tool`
    PEN_WHITE,    // white ink: an erase stroke with `tool`'s texture and settings
    PEN_PRECISE,  // the precise eraser: an erase stroke, the round tip with the eraser's settings
    PEN_WHOLE,    // the stroke eraser: every stroke it touches goes, but erase strokes (taking one
                  // away would bring back the ink it covers)
} PenAct;

typedef struct {
    PenAct act;
    ToolKind tool;  // PEN_INK, PEN_WHITE: the drawing tool; the erasers: TOOL_ERASER
} PenStroke;

// The drawing tool "the pen" means: the one it draws with, or while that is the eraser the drawing
// tool the toolbar shows (the pencil if that is the eraser too, or out of range).
static inline ToolKind pen_drawing_tool(const Settings *s) {
    if ((unsigned)s->current_tool < TOOL_COUNT && s->current_tool != TOOL_ERASER) return s->current_tool;
    if ((unsigned)s->draw_tool < TOOL_COUNT && s->draw_tool != TOOL_ERASER) return s->draw_tool;
    return TOOL_PENCIL;
}

// What a stroke does that the pen starts with its eraser end on the page (`eraser_end`) or with its
// side button held (`side`). The eraser end always erases. The side button erases (side_button 1),
// highlights with ink, whatever `white` says (2), draws white with the pen's drawing tool (3) or
// does nothing (0). Otherwise the pen's tool: the eraser is the precise one with eraser_mode 1 and
// the stroke eraser with 0, and a drawing tool draws white while `white` is set.
static inline PenStroke pen_stroke(const Settings *s, bool eraser_end, bool side) {
    PenStroke eraser = {s->eraser_mode == 1 ? PEN_PRECISE : PEN_WHOLE, TOOL_ERASER};
    if (eraser_end) return eraser;
    if (side) {
        if (s->side_button == 1) return eraser;
        if (s->side_button == 2) return (PenStroke){PEN_INK, TOOL_HIGHLIGHTER};
        if (s->side_button == 3) return (PenStroke){PEN_WHITE, pen_drawing_tool(s)};
    }
    if (s->current_tool == TOOL_ERASER) return eraser;
    return (PenStroke){s->white ? PEN_WHITE : PEN_INK, pen_drawing_tool(s)};
}

// The range a stroke's widths keep in a page file: every reader clamps them to it
// (docs/FORMAT.md; tests/test_inkwire.c holds these to the slider table).
#define ERASER_MM_MIN 0.10f
#define ERASER_MM_MAX 12.f

// The precise eraser's settings for a stroke drawn at `zoom` (screen pixels per page pixel): its
// widths divided by the zoom, so it covers as much of the screen at any zoom as the stroke eraser
// does, within the range a page file keeps (so the stroke reads back exactly as it was drawn).
static inline ToolParams eraser_at_zoom(const ToolParams *e, float zoom) {
    ToolParams p = *e;
    if (!(zoom > 0.f)) zoom = 1.f;  // NaN, 0 or less: as at 100%
    float lo = p.size_min_mm / zoom, hi = p.size_max_mm / zoom;
    p.size_min_mm = lo == lo ? clampf(lo, ERASER_MM_MIN, ERASER_MM_MAX) : ERASER_MM_MIN;
    p.size_max_mm = hi == hi ? clampf(hi, ERASER_MM_MIN, ERASER_MM_MAX) : ERASER_MM_MIN;
    return p;
}
