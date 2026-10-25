// pagesview.h: the editor's Pages view, which covers the screen: numbered tiles a screenful at a
// time (it never reads or draws another page, so it opens at once in a notebook of any length),
// ticks for Delete, and a number pad for Go to page....
//
// A modal hosted by the editor, which keeps the notebook: the view draws itself and takes the taps,
// and says what was asked (PvResult); the editor does it (opens a page, deletes the ticked ones).
#pragma once
#include "app.h"
#include "keyboard.h"

typedef enum {
    PV_NONE = 0,
    PV_CLOSE,   // its Back button: the view closes on the page that was open
    PV_OPEN,    // a tile tapped, or a page number entered: the view closes on page `target`
    PV_DELETE,  // Delete: the editor deletes the ticked pages (pagesview_ticked), the view stays
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
    int target;               // after PV_OPEN: the page to open (an index)
} PagesView;

// Opens it on notebook nb, on the screenful with page `current`, nothing ticked. False when there
// is no memory for its ticks (it stays closed).
bool pagesview_open(PagesView *v, App *a, const NotebookInfo *nb, int current);
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
