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
    // The top four bits name the namespace: 0x5 is image papers (below), 0x6 is reserved for a
    // ruled-paper builder (docs/FORMAT.md).
    PAPER_CUSTOM_FLAG = 0x40000000
} PaperKind;

// An image paper: PAPER_IMAGE_FLAG | role << 24 | id, a picture kept in the page's notebook as
// bg/<id, 6 hex digits>.png (src/bgimage.h) and drawn by page_render (paper_render alone draws it
// blank: it has no notebook). Role 0 is a template (a page added after it gets it too), 1 a document
// page (an imported page image: a page added after it gets the notebook's paper). Bits 26-27 are
// kept for colour and are 0: a value with them set, or with role 2 or 3, is a newer version's.
#define PAPER_IMAGE_FLAG 0x50000000u
enum { PAPER_ROLE_TEMPLATE = 0, PAPER_ROLE_DOCUMENT = 1 };
bool paper_is_image(PaperKind k);               // a well-formed image paper
bool paper_is_document(PaperKind k);            // an image paper of the document page role
PaperKind paper_image(int role, uint32_t id);   // id: the low 24 bits are used
static inline uint32_t paper_image_id(PaperKind k) { return (uint32_t)k & 0xFFFFFFu; }

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
// A fixed kind, a well-formed custom layout or a well-formed image paper. Anything else is a paper
// from a newer version: pages keep it, draw it blank and call it "Newer paper".
bool paper_is_known(PaperKind k);
// A kind the pickers offer (a fixed kind or a well-formed custom layout) as it is; anything else
// becomes `fallback`, an image paper too: it needs its notebook's picture (a new notebook, the
// music layout setting).
PaperKind paper_sanitize(uint32_t v, PaperKind fallback);
PaperKind music_encode(const MusicLayout *m);
bool music_decode(PaperKind k, MusicLayout *m);   // false if k is not a custom layout
const char *music_part_name(MusicPart p);         // "Treble staff", "Tab, 6 lines", ...
// The most systems that fit on a page page_h px tall (at dpi) with staff spaces still big enough
// to write in. A layout keeps its systems on a page of another height: paper_render spaces them
// to fit (with smaller staff spaces on a shorter page), it never drops one.
int music_max_systems(const MusicLayout *m, int page_h, float dpi);
PaperKind music_default(void);                    // treble staff over 6-line tab, 5 systems

// Pickers offer the fixed kinds in paper_at() order, then one "Custom music" tile.
#define PAPER_PICKER_COUNT (PAPER_COUNT + 1)

// e.g. "Narrow rule", "Narrow, no margin", "Template", "Imported page", "Newer paper"
const char *paper_name(PaperKind k);
// The order papers are offered in pickers (index 0..PAPER_COUNT-1 -> kind).
PaperKind paper_at(int index);

// Renders the background for `region` of a full page canvas (canvas size = page size).
// Must be a pure function of (kind, canvas w/h, dpi, pixel position) so any sub-region
// re-render matches a full render exactly. A kind this build does not know is blank paper, and so
// is an image paper here: page_paper_render (page.h) draws it from its notebook's picture, and is
// what every page's paper must come from (the ink layer's erase strokes, drawn live, too).
void paper_render(Canvas *c, PaperKind k, Rect region, float dpi);
// How many times paper_render drew an image paper blank since the last call (the first is logged):
// a caller that should have used page_paper_render. The tests hold page rendering to 0.
int paper_take_image_blank_count(void);
