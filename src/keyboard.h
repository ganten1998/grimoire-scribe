// keyboard.h: on-screen keyboard modal for naming notebooks and folders.
// QWERTY + numbers row, shift, space, backspace, Cancel / Done. Pen or finger taps.
#pragma once
#include "ui.h"
#include "input.h"

#define KB_TEXT_MAX 96

typedef enum { KB_NONE = 0, KB_CHANGED, KB_DONE, KB_CANCEL } KbResult;

typedef struct {
    bool open;
    char title[64];
    char text[KB_TEXT_MAX];
    bool shift;
    Rect area;       // where it was last drawn (whole modal)
    Press press;
    int pressed_key; // -1 if none
} Keyboard;

void keyboard_open(Keyboard *kb, const char *title, const char *initial);
void keyboard_close(Keyboard *kb);
// Draws the modal (text field at top, keys below) anchored to the bottom of screen.
// Returns the rect it covered (to invalidate).
Rect keyboard_draw(Keyboard *kb, Canvas *c, Rect screen);
// Pointer events only (pen or touch, already unified by the caller as down/move/up).
// *dirty receives the rect needing redraw (may be empty).
KbResult keyboard_pointer(Keyboard *kb, int phase /*0 down,1 move,2 up*/, float x, float y, uint32_t t, Rect *dirty);
