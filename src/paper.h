// paper.h: page backgrounds. The numeric values are stored on disk: never reorder.
#pragma once
#include "canvas.h"

typedef enum {
    PAPER_NARROW = 0,  // 5.5 mm rule
    PAPER_MEDIUM = 1,  // 7.1 mm rule
    PAPER_WIDE = 2,    // 9.0 mm rule
    PAPER_MUSIC = 3,   // 8 roomy five-line staves
    PAPER_BLANK = 4,
    PAPER_NARROW_NM = 5,  // the three rules again, without the margin line
    PAPER_MEDIUM_NM = 6,
    PAPER_WIDE_NM = 7,
    PAPER_GRAND = 8,      // piano grand staff: treble + bass systems with clefs and a brace
    PAPER_TAB = 9,        // guitar tablature: six-line staves
    PAPER_DOTS = 10,      // 5 mm dot grid
    PAPER_GRID = 11,      // 5 mm graph paper
    PAPER_CORNELL = 12,   // Cornell notes: cue column, notes, summary
    PAPER_COUNT
} PaperKind;

const char *paper_name(PaperKind k);   // e.g. "Narrow rule", "Narrow, no margin", "Sheet music"
// The order papers are offered in pickers (index 0..PAPER_COUNT-1 -> kind).
PaperKind paper_at(int index);

// Renders the background for `region` of a full page canvas (canvas size = page size).
// Must be a pure function of (kind, canvas w/h, dpi, pixel position) so any sub-region
// re-render matches a full render exactly.
void paper_render(Canvas *c, PaperKind k, Rect region, float dpi);
