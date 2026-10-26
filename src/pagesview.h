// pagesview.h: the editor's Pages view, which covers the screen: numbered tiles a screenful at a
// time (it never reads or draws another page, so it opens at once in a notebook of any length),
// ticks for Delete, a number pad for Go to page..., New page and Copy page. A page that holds a
// book page (a notebook annotated from KOReader, annotate.h) says which under its number ("p. 12"),
// from the notebook's source.ini, read as the view opens.
//
// A modal hosted by the editor, which keeps the notebook: the view draws itself and takes the taps,
// and says what was asked (PvResult); the editor does it (opens a page, deletes the ticked ones).
#pragma once
#include "app.h"
#include "annotate.h"
#include "keyboard.h"
#include "nav.h"

typedef enum {
    PV_NONE = 0,
    PV_CLOSE,   // its Back button: the view closes on the page that was open
    PV_OPEN,    // a tile tapped, or a page number entered: the view closes on page `target`
    PV_DELETE,  // Delete: the editor deletes the ticked pages (pagesview_ticked), the view stays
    PV_NEW,     // New page: the editor adds a page at index `target` (after the last page ticked, or
                // after the open page: never first) and opens it
    PV_COPY,    // Copy page (one page ticked): the editor copies page `target` right after it
    PV_NAME,    // save kb.text as the open page's section name; empty removes it
} PvResult;

typedef struct {
    bool open;
    const NotebookInfo *nb;   // the open notebook (the editor's)
    int current;              // the open page's index; the editor keeps it up to date
    Keyboard kb;              // Go to page...'s number pad
    int screen;               // which screenful of tiles
    bool *sel;                // ticked, per page index
    int sel_cap;
    HitList hits;
    Press press;
    int target;               // after PV_OPEN, PV_NEW, PV_COPY: a page index (see PvResult)
    AnnotateMap book;         // source.ini: which page holds which book page (empty for most)
    const Nav *nav;
    bool index_tab, tab_initialized, naming;
    int index_screen;
} PagesView;

// Opens it on notebook nb, on the screenful with page `current`, nothing ticked. False when there
// is no memory for its ticks (it stays closed).
bool pagesview_open(PagesView *v, App *a, const NotebookInfo *nb, const Nav *nav, int current);
// Draws the whole screen into a->screen (the caller refreshes it).
void pagesview_draw(PagesView *v, App *a);
// Pointer phase 0 down / 1 move / 2 up. What it draws itself (a tick, another screenful, the
// number pad) it refreshes itself; for the rest, see PvResult.
PvResult pagesview_pointer(PagesView *v, App *a, int phase, const InEvent *ev);
int pagesview_ticked_count(const PagesView *v);
bool pagesview_ticked(const PagesView *v, int index);
void pagesview_untick(PagesView *v);
// Closes it (and its number pad). Its ticks' memory stays for the next open.
void pagesview_close(PagesView *v);
void pagesview_free(PagesView *v);
// The headless runner's keys for it, without "editor." (docs/SIM.md): "tile.N", the label under
// page N's tile (from 1) as drawn, its two lines joined by " | " ("Page 3 | p. 12"); empty while it
// is closed. 0, or -1 for a key it does not know.
int pagesview_probe(const PagesView *v, const char *key, char *out, size_t cap);
