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
    PAPER_BASS4 = 13,     // bass tablature, 4 / 5 / 6 / 7 strings
    PAPER_BASS5 = 14,
    PAPER_BASS6 = 15,
    PAPER_BASS7 = 16,
    PAPER_COUNT,          // fixed kinds end here
    // A custom music layout (see MusicLayout): this flag | systems << 24 | part i << 4*i.
    // The top four bits name the namespace; 0x5 (image papers) and 0x6 (a ruled-paper builder)
    // are reserved for later versions (docs/FORMAT.md).
    PAPER_CUSTOM_FLAG = 0x40000000
} PaperKind;

// The parts a custom music system stacks, top to bottom. Stored on disk: never reorder.
typedef enum {
    PART_NONE = 0,
    PART_TREBLE, PART_BASS, PART_ALTO, PART_TENOR, PART_PERC, PART_STAFF,
    PART_TAB4, PART_TAB5, PART_TAB6, PART_TAB7, PART_LYRICS,
    PART_COUNT
} MusicPart;

#define MUSIC_MAX_PARTS 6
#define MUSIC_MAX_SYSTEMS 12

typedef struct {
    int systems;                    // systems per page, 1..MUSIC_MAX_SYSTEMS
    int n;                          // parts per system, 1..MUSIC_MAX_PARTS
    MusicPart part[MUSIC_MAX_PARTS];
} MusicLayout;

bool paper_is_custom(PaperKind k);   // in the custom music namespace (well-formed or not)
// A fixed kind or a well-formed custom layout. Anything else is a paper from a newer version:
// pages keep it, draw it blank and call it "Newer paper".
bool paper_is_known(PaperKind k);
// A known kind as it is; anything else becomes `fallback` (where a known kind is required).
PaperKind paper_sanitize(uint32_t v, PaperKind fallback);
// An image paper of 1.1's document page role (docs/FORMAT.md): a picture of one imported page,
// 0x5 in the top four bits, role 1 in bits 24-25, bits 26-27 clear. This version draws it blank as
// it does every newer paper, but a page added after one never takes it (store_new_page_paper).
bool paper_is_document(PaperKind k);
PaperKind music_encode(const MusicLayout *m);
bool music_decode(PaperKind k, MusicLayout *m);   // false if k is not a custom layout
const char *music_part_name(MusicPart p);         // "Treble staff", "Tab, 6 lines", ...
// The most systems that fit on a page with staff spaces still big enough to write in.
int music_max_systems(const MusicLayout *m);
PaperKind music_default(void);                    // treble staff over 6-line tab, 5 systems

// Pickers offer the fixed kinds in paper_at() order, then one "Custom music" tile.
#define PAPER_PICKER_COUNT (PAPER_COUNT + 1)

const char *paper_name(PaperKind k);   // e.g. "Narrow rule", "Narrow, no margin", "Newer paper"
// The order papers are offered in pickers (index 0..PAPER_COUNT-1 -> kind).
PaperKind paper_at(int index);

// Renders the background for `region` of a full page canvas (canvas size = page size).
// Must be a pure function of (kind, canvas w/h, dpi, pixel position) so any sub-region
// re-render matches a full render exactly. A kind this build does not know is blank paper.
void paper_render(Canvas *c, PaperKind k, Rect region, float dpi);
