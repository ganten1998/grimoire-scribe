// musicbuild.h: the "Custom music" sheet. Stack staves (with clefs), tabs and lyric lines
// into a system, choose how many systems go on a page, see the page as it will look.
// A modal hosted by the editor (paper for this page) and the library (new notebook).
#pragma once
#include "app.h"

typedef enum { MB_NONE = 0, MB_CHANGED, MB_CANCEL, MB_DONE } MbResult;

typedef struct {
    bool open;
    Rect rect;
    HitList hits;
    MusicLayout m;
    Press press;
    Canvas prev;          // page preview at 1/4 size, rebuilt when the layout changes
    PaperKind prev_k;
} MusicBuild;

// Starts from `start` if it is a custom layout, else from the last one built (settings).
void musicbuild_open(MusicBuild *b, App *a, PaperKind start);
Rect musicbuild_draw(MusicBuild *b, App *a);   // draws into a->screen, returns its rect
// Pointer phase 0 down / 1 move / 2 up. MB_DONE: read musicbuild_result (already saved as
// the last layout); MB_DONE and MB_CANCEL close the sheet (the host recomposes b->rect).
MbResult musicbuild_pointer(MusicBuild *b, App *a, int phase, float x, float y, uint32_t t, Rect *dirty);
PaperKind musicbuild_result(const MusicBuild *b);
void musicbuild_free(MusicBuild *b);
