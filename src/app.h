// app.h: the App, screen switching, and the dirty-rect / refresh accumulator.
#pragma once
#include "canvas.h"
#include "display.h"
#include "input.h"
#include "store.h"
#include "ui.h"

typedef enum { SCR_LIBRARY = 0, SCR_EDITOR, SCR_PENSET, SCR_CALIB, SCR_SHARE, SCR_COUNT } ScreenId;

// Screen modules keep their own state behind opaque pointers; each allocates it in *_init.
struct LibraryState;
struct EditorState;
struct PenSetState;
struct CalibState;
struct ShareState;

typedef struct App {
    int w, h;
    float dpi;
    Canvas screen;          // mirrors the panel
    Settings settings;
    ScreenId scr, prev_scr;
    bool running;

    // Dirty accumulator, flushed once per loop iteration.
    Rect dirty;
    RefreshMode dirty_mode;
    int ui_refreshes_since_full;   // promote to FULL every 30
    int rotation;                  // applied device rotation (quarter turns clockwise)
    int pending_rotation;          // reported by the sensor, applied when it is safe; -1 none

    // Toast (drawn by app over any screen, auto-hidden after toast_until).
    char toast[128];
    uint32_t toast_until;
    Rect toast_rect;
    bool toast_visible;
    Canvas toast_under;     // pixels the toast covers, restored when it hides

    struct LibraryState *lib;
    struct EditorState *ed;
    struct PenSetState *ps;
    struct CalibState *cal;
    struct ShareState *share;
} App;

int app_init(App *a, int w, int h, float dpi);
void app_shutdown(App *a);          // saves everything
void app_event(App *a, const InEvent *ev);
void app_tick(App *a, uint32_t now); // timers: autosave, toast, long press, httpd
void app_flush(App *a);             // presents dirty rect (if any)

void app_invalidate(App *a, Rect r, RefreshMode m);
void app_invalidate_all(App *a, RefreshMode m);
void app_switch(App *a, ScreenId s); // calls leave/enter and redraws fully (REFRESH_FULL)
void app_toast(App *a, const char *msg, int ms);
// Turn the UI to a device rotation now (relayouts every screen).
void app_set_rotation(App *a, int quarter_turns);
// Where documents mirror PDFs go (/mnt/us/documents on Kindle, <root>/documents elsewhere).
const char *app_docs_dir(void);
// Pressed-state feedback for a tapped control: invert + FAST refresh now.
void app_press_feedback(App *a, Rect r);
// Unified pointer phase for UI screens: pen and finger both act as a pointer.
// Returns -1 if the event is not a pointer event, else 0 down / 1 move / 2 up.
int app_pointer_phase(const InEvent *ev);

// Screen module interface (each implemented in its own .c):
//   X_init(App*) allocates state; X_free(App*);
//   X_enter(App*) on switch-in (draw everything into a->screen and invalidate);
//   X_leave(App*) on switch-out; X_event(App*, const InEvent*); X_tick(App*, uint32_t now).
#define DECLARE_SCREEN(X)                         \
    void X##_init(App *a);                        \
    void X##_free(App *a);                        \
    void X##_enter(App *a);                       \
    void X##_leave(App *a);                       \
    void X##_event(App *a, const InEvent *ev);    \
    void X##_tick(App *a, uint32_t now);

DECLARE_SCREEN(library)
DECLARE_SCREEN(editor)
DECLARE_SCREEN(pensettings)
DECLARE_SCREEN(calibrate)
DECLARE_SCREEN(share)

// Cross-screen entry points.
void editor_open_notebook(App *a, const char *id);   // loads, switches to SCR_EDITOR
void editor_save_now(App *a);                        // flush pending page/info saves
bool editor_busy(App *a);                            // a stroke / drag / pinch is in progress
void pensettings_open(App *a, ToolKind tab);         // switch to SCR_PENSET (returns to prev_scr)
void calibrate_open(App *a);                         // switch to SCR_CALIB (returns to prev_scr)
void share_open(App *a);                             // switch to SCR_SHARE (returns to prev_scr)
