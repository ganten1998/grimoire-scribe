// dispmenu.h: the Display sheet (front light brightness + warmth, invert Quire's colors).
// A modal hosted by the editor and the library; it draws itself and handles pointer input.
#pragma once
#include "app.h"

typedef enum { DM_NONE = 0, DM_CHANGED, DM_CLOSED, DM_INVERTED } DmResult;

typedef struct {
    bool open;
    Rect rect;
    HitList hits;
    int intensity, warmth;   // 0..LIGHT_MAX, -1 unknown
    int active;              // slider being dragged: 0 none, 1 brightness, 2 warmth
    uint32_t last_apply;
    Press press;
} DispMenu;

void dispmenu_open(DispMenu *m, App *a);
// Draws into a->screen; returns the rect covered.
Rect dispmenu_draw(DispMenu *m, App *a);
// Pointer phase 0 down / 1 move / 2 up. DM_CLOSED: the host should recompose m->rect.
// DM_INVERTED: colors flipped, the host should repaint everything (full refresh).
DmResult dispmenu_pointer(DispMenu *m, App *a, int phase, float x, float y, uint32_t t, Rect *dirty);
