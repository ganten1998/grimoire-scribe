// keyboard.h: on-screen keyboard modal for naming notebooks and folders.
// QWERTY + numbers row, shift, space, backspace, Cancel / Done, and a key to a page of accented
// letters (and back). Pen or finger taps. The text is UTF-8: a key types a whole character and
// backspace takes a whole character away. In Japanese the accents key is a kana key: a flick
// keypad, twelve keys in three columns, where a tap types the row's first kana and a swipe left,
// up, right or down the rest of the row (a mobile phone's kana pad), with ー beside it, one
// modifier that cycles the kana typed last (゛小: か to が, は to ば to ぱ, つ to づ to っ), カ for
// katakana and 、 for 、。？！.
#pragma once
#include "ui.h"
#include "input.h"

#define KB_TEXT_MAX 96

typedef enum { KB_NONE = 0, KB_CHANGED, KB_DONE, KB_CANCEL } KbResult;

// Key codes: a character key's code is its Unicode code point as typed without shift; the
// other keys have codes below 32.
enum {
    KB_KEY_SHIFT = 1,
    KB_KEY_PAGE = 2,
    KB_KEY_KANA = 4,    // the kana page (Japanese: the letters page's accents key becomes this)
    KB_KEY_KANA_MOD = 5, // ゛小: the kana typed last in its next form, cycling (か が か; は ば ぱ;
                         // つ づ っ; や ゃ)
    KB_KEY_KATA = 7,     // カ: the kana page in katakana (カ キ ク), or back to hiragana
    KB_KEY_BKSP = 8,
    KB_KEY_DONE = 13,
    KB_KEY_FLICK = 16,  // + row: the kana page's own keys, whose swipe picks one of five kana
    KB_KEY_CANCEL = 27,
};

// The punctuation key, which the kana page's bottom row holds: 、。？！・ on its flicks.
#define KB_KEY_PUNCT (KB_KEY_FLICK + 10)

typedef struct {
    bool open;
    char title[64];
    char text[KB_TEXT_MAX];
    bool shift;
    bool kata;       // the kana page in katakana (カ toggles it): the pad types カ キ ク, not か き く
    int page;        // 0 letters, 1 accents, 2 number pad
    Rect area;       // where it was last drawn (whole modal)
    Press press;
    int pressed_key; // -1 if none
    int flick;       // on the kana page, how the pressed key was swiped: 0 while still on it (a
                     // tap), 1 left, 2 up, 3 right, 4 down. Its kana is typed on the lift.
    bool typed;      // the pressed key already typed on touch-down
    Rect field_dirty;// set by keyboard_pointer when the text field changed (refresh it first)
    uint32_t settle_at;  // when a pause in typing gets the keys their grayscale refresh
                         // (app_keyboard_tick), 0 for none due
} Keyboard;

// Starts on the letters page with initial as the text: every valid character is kept (accents
// included); control characters and broken UTF-8 are dropped, and the text is cut, if it must
// be, between characters.
void keyboard_open(Keyboard *kb, const char *title, const char *initial);
void keyboard_close(Keyboard *kb);
// Draws the modal (text field at top, keys below) anchored to the bottom of screen.
// Returns the rect it covered (to invalidate).
Rect keyboard_draw(Keyboard *kb, Canvas *c, Rect screen);
// How many times keyboard_draw drew a keyboard since this was last called (the headless runner's
// keyboard.draws check, docs/SIM.md): every draw of the whole modal is work on the main thread, and a
// key press should cost one.
unsigned keyboard_draws_take(void);
// Pointer events only (pen or touch, already unified by the caller as down/move/up).
// *dirty receives the rect needing redraw (may be empty). Characters and backspace type on
// touch-down (no waiting for the lift); Shift, the page key, Cancel and Done act on release.
KbResult keyboard_pointer(Keyboard *kb, int phase /*0 down,1 move,2 up*/, float x, float y, uint32_t t, Rect *dirty);
// Where the key with this code is on the page showing now (as last drawn); false if that page
// has no such key. For tests and scripted runs.
bool keyboard_key_rect(const Keyboard *kb, uint32_t code, Rect *r);
// The capital shift types for a letter (Latin-1 and Latin Extended-A); anything without one
// comes back as it is.
uint32_t keyboard_upper(uint32_t cp);
