// ui.h: e-ink widgets. Stateless draw functions + hit tests; screens own the state.
// Style: white surfaces, 3 px black outlines, themed corners (ui_frame), bold labels, no grays
// below 128 except ink, so everything stays crisp under GL16/DU refresh.
// Sizes scale with dpi via ui_px(); at 300 dpi a standard touch target is ~ 10 mm.
// Icons come from Terminess Nerd Font (its Material Design glyphs); type follows the theme.
#pragma once
#include "canvas.h"
#include "font.h"

typedef enum {
    ICON_NONE = 0,
    ICON_PENCIL, ICON_MECH_PENCIL, ICON_BALLPOINT, ICON_ERASER,
    ICON_UNDO, ICON_REDO,
    ICON_PREV, ICON_NEXT, ICON_PLUS, ICON_MINUS, ICON_CLOSE,
    ICON_PAPER, ICON_GEAR, ICON_LIBRARY, ICON_COLLAPSE, ICON_EXPAND,
    ICON_GRIP, ICON_FOLDER, ICON_NOTEBOOK, ICON_MORE, ICON_SHARE,
    ICON_TRASH, ICON_CHECK, ICON_BACK, ICON_TARGET, ICON_EXIT,
    ICON_TUNE, ICON_RENAME, ICON_FOLDER_MOVE, ICON_FOLDER_PLUS, ICON_DOWNLOAD, ICON_HIGHLIGHTER,
    ICON_BROOM, ICON_BRIGHTNESS, ICON_INVERT, ICON_FOUNTAIN, ICON_FINELINER, ICON_MARKER,
    ICON_UP, ICON_DOWN, ICON_MUSIC, ICON_CASTLE, ICON_CHARCOAL, ICON_CALLIGRAPHY,
    ICON_COUNT
} IconKind;

typedef struct {
    float dpi;
    float scale;      // dpi / 300
} UiMetrics;

typedef enum { THEME_GOTHIC = 0, THEME_CLASSIC = 1 } UiTheme;
void ui_set_theme(UiTheme t);   // also switches the fonts
UiTheme ui_theme(void);

void ui_init(float dpi);
int ui_px(float px_at_300dpi);   // scales a 300-dpi pixel size to the current dpi
float ui_text_size(void);        // body text px (~ 3.4 mm)
float ui_title_size(void);       // title text px (~ 5 mm)
float ui_small_size(void);       // small text px (~ 2.6 mm)

// The icon for a drawing tool (ToolKind value).
IconKind ui_tool_icon(int tool);
// Icon glyph scaled to fit r (with a small inset), centered.
void ui_icon(Canvas *c, IconKind k, Rect r, uint8_t gray);

typedef enum { BTN_PLAIN = 0, BTN_SELECTED = 1, BTN_DISABLED = 2, BTN_PRIMARY = 3 } ButtonStyle;
// Icon above label if both given; selected = inverted (black fill, white glyph).
void ui_button(Canvas *c, Rect r, IconKind icon, const char *label, ButtonStyle st);

// Horizontal slider: label + value text on the left, track on the right.
// value_text may be NULL (then "%.2f unit" is shown).
typedef struct {
    Rect r;           // whole row
    Rect track;       // filled in by ui_slider_layout
    float min, max, step, value;
    const char *label, *unit;
} Slider;
void ui_slider_layout(Slider *s, Rect row);
void ui_slider_draw(Canvas *c, const Slider *s, const char *value_text);
// Maps an x coordinate to a snapped value; returns true if the value changed.
bool ui_slider_set_from_x(Slider *s, float x);

// A box in the theme's shape (rounded, or cut corners), same arguments as canvas_round_rect.
void ui_frame(Canvas *c, Rect r, int radius, uint8_t gray, bool filled, int t);
// A horizontal rule (gothic: thick over thin).
void ui_rule(Canvas *c, int x0, int x1, int y, int t, uint8_t gray);
// Gothic only: a cornice of pointed arches h px deep hanging from y.
void ui_frieze(Canvas *c, int x0, int x1, int y, int h, uint8_t gray);
// Gothic quatrefoil ornament of radius r.
void ui_emblem(Canvas *c, float cx, float cy, float r, uint8_t ink, uint8_t bg);

// Modal sheet: dims nothing (e-ink), draws a white card with a thick border + title bar.
// Returns the content rect inside the card.
Rect ui_sheet(Canvas *c, Rect card, const char *title);
// Toast-style one-line message box centered near the bottom of the screen.
Rect ui_toast_rect(Rect screen, const char *msg);
void ui_toast(Canvas *c, Rect screen, const char *msg);
void ui_toast_at(Canvas *c, Rect r, const char *msg);  // r: ui_toast_rect's, moved as the app needs
void ui_checkbox(Canvas *c, Rect r, const char *label, bool checked);

// Tap tracking shared by screens: a press that starts and ends in the same rect within
// slop = tap. Long press = held > 600 ms without moving.
typedef struct {
    bool down;
    float x0, y0, x, y;
    uint32_t t0;
    bool moved;
} Press;
void press_begin(Press *p, float x, float y, uint32_t t);
void press_move(Press *p, float x, float y);   // marks moved beyond slop (ui_px(30))
bool press_is_tap(const Press *p, Rect r);     // call at release

// Hit regions registered while drawing; screens dispatch taps through them.
typedef struct {
    Rect r;
    int id;
    int arg;
} Hit;
typedef struct {
    Hit h[128];
    int n;
} HitList;
static inline void hits_clear(HitList *l) { l->n = 0; }
static inline void hits_add(HitList *l, Rect r, int id, int arg) {
    if (l->n < (int)(sizeof l->h / sizeof l->h[0])) l->h[l->n++] = (Hit){r, id, arg};
}
// Topmost (last added) hit containing the point, or NULL.
static inline const Hit *hits_find(const HitList *l, float x, float y) {
    for (int i = l->n - 1; i >= 0; i--)
        if (rect_contains(l->h[i].r, (int)x, (int)y)) return &l->h[i];
    return NULL;
}
