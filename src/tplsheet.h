// tplsheet.h: the Templates sheet, behind the "More papers..." tile of the editor's Paper sheet and
// of New notebook: the pictures in grimoire/templates/ (templates.h) as tiles, a page of them at a
// time, to pick one as paper. A tile's preview is read only while the tile is on screen, and a
// tile whose preview the background job has not made yet shows a placeholder until it has. Also
// the layout the paper pickers share: their grid of tiles and the labels under them.
//
// A modal hosted by the editor (in the Paper sheet's place) and the library (over New notebook).
// It never shows or logs more of a template than its name on screen: names are the owner's.
#pragma once
#include "app.h"
#include "templates.h"

// The paper pickers' tiles: the fixed papers in paper_at() order, then Custom music..., then More
// papers... (this sheet).
#define PAPER_SHEET_TILES (PAPER_PICKER_COUNT + 1)

// A grid of n tiles (each 3:4 with label_h px for its label under it, gap px apart) in a box
// avail_w wide and avail_h tall: the number of columns, at least min_cols, that makes the tiles
// biggest (no wider than max_w), and of those that make them as big, the one that leaves the
// fewest places empty on the last row. The paper pickers take at least 6, as they always had:
// upright 6 (the papers where they were, More papers... on a row of its own), sideways 7, which
// keeps them as big as 6 were before More papers... came.
typedef struct {
    int cols, rows;
    int tw, th;     // a tile
    int cell_h;     // a row: a tile and its label
} PaperGrid;
PaperGrid paper_grid(int n, int avail_w, int avail_h, int gap, int label_h, int max_w, int min_cols);
#define PAPER_GRID_MIN_COLS 6
// The editor's Paper sheet on this screen, and the grid of its PAPER_SHEET_TILES tiles (the
// Templates sheet takes the same place and tiles).
Rect paper_sheet_rect(const App *a, PaperGrid *g);
// A paper tile's label, under the tile (tile: its rect; gap: the grid's): on one line, centred,
// or on two lines no wider than the tile when it is wider than the tile (a break after a comma
// preferred: "Narrow," over "no margin"), so that labels side by side never run into each other.
// In bold for the selected tile, on as many lines as it has in regular.
void paper_tile_label(Canvas *c, Rect tile, int gap, const char *label, bool selected);
// The More papers... tile's picture when it has none to show (the paper is no picture): a blank
// page with the sign for more.
void paper_tile_more(Canvas *c, Rect tile);

typedef enum { TS_NONE = 0, TS_CHANGED, TS_PICKED, TS_BACK, TS_CANCEL } TsResult;
// The hits it registers as it draws (tests find its buttons by them): a tile (arg: the template, an
// index into shown), the page buttons, Back.
enum { TS_HIT_TILE = 1, TS_HIT_PREV, TS_HIT_NEXT, TS_HIT_BACK };

typedef struct {
    bool open;
    Rect rect;                // on screen, as last drawn
    HitList hits;
    Press press;
    Template *t;              // every template of the folder (template_scan), bad ones too
    int nt;
    int *shown;               // the ones the sheet offers (not bad), indices into t
    int n;
    int anchor;               // a template to keep in view, an index into shown: the first of the
                              // page last paged to (the selected one's, at first)
    int page, pages, per;     // as last laid out
    int selected;             // the one that is the paper now (index into shown), or -1
    int picked;               // after TS_PICKED: the one tapped (index into shown)
    bool pending;             // some offered template has no preview yet
    unsigned gen;             // bgimage_generation when the list was last brought up to date
    uint32_t next_check;      // when to look again for previews made since
    // What the paper now is: in the editor a picture of nb's (current), in New notebook a file.
    const NotebookInfo *nb;
    BgIndex ix;
    PaperKind current;
    char current_path[TEMPLATE_PATH_MAX];
} TplSheet;

// Opens it on the templates as they are now (the previews job queued for those that have none):
// on the page with the template that is the paper now, which is selected. In the editor that is
// the open page's paper `current` (an image paper of notebook nb's); in New notebook (nb NULL) the
// file current_path (NULL or "" for none).
void tplsheet_open(TplSheet *s, App *a, const NotebookInfo *nb, PaperKind current, const char *current_path);
Rect tplsheet_draw(TplSheet *s, App *a);   // draws into a->screen, returns its rect
// Pointer phase 0 down / 1 move / 2 up. TS_PICKED (a template tapped), TS_BACK (its Back button)
// and TS_CANCEL (a tap outside it) close it: the host recomposes *dirty, and after TS_PICKED reads
// tplsheet_path (and tplsheet_preview), then calls tplsheet_close. TS_CHANGED: another page of
// tiles, drawn; refresh *dirty.
TsResult tplsheet_pointer(TplSheet *s, App *a, int phase, float x, float y, uint32_t t, Rect *dirty);
// From the host's tick while it is open: previews made since (looked for every 1.5 s while a
// background job runs, and when one ends) are drawn in their tiles. True with *dirty to refresh.
bool tplsheet_tick(TplSheet *s, App *a, uint32_t now, Rect *dirty);
// After TS_PICKED, until tplsheet_close: the picked template's file, and its preview.
const char *tplsheet_path(const TplSheet *s);
const Canvas *tplsheet_preview(TplSheet *s);
// Closes it and lets go of its list (also when it is closed already).
void tplsheet_close(TplSheet *s);
// The headless runner's "templates.*" keys (key without the prefix; docs/SIM.md): count (the
// templates it offers), ready (how many of those have their previews), page and pages (of tiles,
// from 1), selected (the selected one's name). Empty values while it is closed. 0, -1 for a key
// it does not know.
int tplsheet_probe(const TplSheet *s, const char *key, char *out, size_t cap);
