// calibrate.c: pen then touch calibration. Targets are matched by ORDER using raw device
// coordinates, so a badly wrong current mapping cannot get in the way. Buttons are full-width
// bars that accept the other input (finger while calibrating the pen, pen for touch).
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "calib_fit.h"

typedef enum { PH_PEN, PH_TOUCH, PH_VERIFY } Phase;

enum { NT = 5 };  // 4 corners + center check
static const float k_tx[NT] = {0.12f, 0.88f, 0.88f, 0.12f, 0.5f};
static const float k_ty[NT] = {0.12f, 0.12f, 0.88f, 0.88f, 0.5f};

enum { B_NONE, B_CANCEL, B_SKIP, B_SAVE, B_REDO };

struct CalibState {
    Phase phase;
    int target;
    CalPoint pts[NT];
    InputCalib old_pen, old_touch, new_pen, new_touch;
    ScreenId ret;
    char msg[128];
    Rect pad;
    bool drawing;
    BrushState bs;
    HitList hits;
};

#define S (a->cal)

static Rect bar(App *a, int slot) {
    int h = ui_px(170), m = ui_px(60);
    return (Rect){m, a->h / 2 + ui_px(260) + slot * (h + ui_px(30)), a->w - 2 * m, h};
}

static void draw(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(60);
    const char *title = S->phase == PH_PEN ? "Calibrate the pen" : S->phase == PH_TOUCH ? "Calibrate touch" : "Check it";
    Rect tr = {m, a->h / 2 - ui_px(420), a->w - 2 * m, ui_px(110)};
    font_draw_centered(c, FONT_BOLD, ui_title_size() * 1.2f, tr, title, 0);
    char sub[160];
    if (S->phase == PH_VERIFY)
        snprintf(sub, sizeof sub, "Draw in the box: the line should sit right under the tip.");
    else
        snprintf(sub, sizeof sub, "%s the center of the target (%d of %d).",
                 S->phase == PH_PEN ? "Tap with the pen tip on" : "Tap with a finger on", S->target + 1, NT);
    tr.y += ui_px(120);
    font_draw_centered(c, FONT_REGULAR, ui_text_size(), tr, sub, 0);
    if (S->msg[0]) {
        tr.y += ui_px(100);
        font_draw_centered(c, FONT_BOLD, ui_text_size(), tr, S->msg, 0);
    }
    if (S->phase != PH_VERIFY) {
        float cx = k_tx[S->target] * a->w, cy = k_ty[S->target] * a->h;
        int r = ui_px(80);
        canvas_disc_aa(c, cx, cy, (float)r, 0, 1.f);
        canvas_disc_aa(c, cx, cy, (float)r - ui_px(8), 255, 1.f);
        canvas_disc_aa(c, cx, cy, (float)ui_px(14), 0, 1.f);
        canvas_line_aa(c, cx - r * 1.6f, cy, cx + r * 1.6f, cy, (float)ui_px(4), 0);
        canvas_line_aa(c, cx, cy - r * 1.6f, cx, cy + r * 1.6f, (float)ui_px(4), 0);
        Rect cancel = bar(a, 0);
        ui_button(c, cancel, ICON_NONE, S->phase == PH_PEN ? "Cancel (tap with a finger)" : "Cancel", BTN_PLAIN);
        hits_add(&S->hits, cancel, B_CANCEL, 0);
        if (S->phase == PH_TOUCH) {
            Rect skip = bar(a, 1);
            ui_button(c, skip, ICON_NONE, "Touch was fine: skip (tap with the pen)", BTN_PLAIN);
            hits_add(&S->hits, skip, B_SKIP, 0);
        }
    } else {
        S->pad = (Rect){m, a->h / 2 - ui_px(120), a->w - 2 * m, ui_px(340)};
        ui_frame(c, rect_inflate(S->pad, ui_px(4)), ui_px(14), 0, false, ui_px(4));
        int slot = 0;
        Rect save = bar(a, slot++), redo = bar(a, slot++), cancel = bar(a, slot++);
        ui_button(c, save, ICON_CHECK, "Save", BTN_PRIMARY);
        ui_button(c, redo, ICON_NONE, "Start over", BTN_PLAIN);
        ui_button(c, cancel, ICON_NONE, "Cancel (keep the old calibration)", BTN_PLAIN);
        hits_add(&S->hits, save, B_SAVE, 0);
        hits_add(&S->hits, redo, B_REDO, 0);
        hits_add(&S->hits, cancel, B_CANCEL, 0);
    }
    app_invalidate_all(a, REFRESH_UI);
}

static void restart(App *a, Phase ph, const char *msg) {
    S->phase = ph;
    S->target = 0;
    strlcpy_q(S->msg, msg ? msg : "", sizeof S->msg);
    draw(a);
}

static void finish_pass(App *a) {
    InputCalib c;
    bool pen = S->phase == PH_PEN;
    // Fit on the 4 corners, then check the center.
    if (!calib_fit(S->pts, 4, &c)) {
        restart(a, S->phase, "Those taps did not make sense. Let's try again.");
        return;
    }
    float sx, sy;
    calib_apply(&c, S->pts[4].nx, S->pts[4].ny, &sx, &sy);
    float err = hypotf(sx - S->pts[4].sx, sy - S->pts[4].sy);
    if (err > mm_to_px(4.f, a->dpi)) {
        char m[128];
        snprintf(m, sizeof m, "The center was off by %.1f mm. Let's try again.", err * 25.4f / a->dpi);
        restart(a, S->phase, m);
        return;
    }
    if (pen) {
        S->new_pen = c;
        input_set_calib(&S->new_pen, NULL);
        restart(a, PH_TOUCH, "Pen done.");
    } else {
        S->new_touch = c;
        input_set_calib(NULL, &S->new_touch);
        restart(a, PH_VERIFY, "");
    }
}

static void leave_to(App *a) { app_switch(a, S->ret == SCR_CALIB ? SCR_LIBRARY : S->ret); }

static void restore_old(App *a) {
    InputCalib dp, dt;
    input_default_calib(&dp, &dt);
    input_set_calib(S->old_pen.valid ? &S->old_pen : &dp, S->old_touch.valid ? &S->old_touch : &dt);
}

void calibrate_open(App *a) {
    S->ret = a->scr;
    app_switch(a, SCR_CALIB);
}

void calibrate_init(App *a) { a->cal = calloc(1, sizeof *a->cal); }
void calibrate_free(App *a) {
    free(a->cal);
    a->cal = NULL;
}

void calibrate_enter(App *a) {
    S->old_pen = a->settings.pen_calib;
    S->old_touch = a->settings.touch_calib;
    S->new_pen = S->new_touch = (InputCalib){0};
    S->drawing = false;
    restart(a, PH_PEN, "");
}

void calibrate_leave(App *a) {}

static int button_at(App *a, float x, float y) {
    const Hit *h = hits_find(&S->hits, x, y);
    return h ? h->id : B_NONE;
}

void calibrate_event(App *a, const InEvent *ev) {
    bool pen = ev->type == IN_PEN_DOWN || ev->type == IN_PEN_MOVE || ev->type == IN_PEN_UP;
    int ph = app_pointer_phase(ev);
    if (ph < 0) return;
    if (S->phase == PH_VERIFY) {
        if (pen && ph == 0 && rect_contains(S->pad, (int)ev->x, (int)ev->y)) {
            ToolParams p;
            tool_params_default(TOOL_BALLPOINT, &p);
            brush_begin(&S->bs, TOOL_BALLPOINT, &p, 7, a->dpi);
            S->drawing = true;
        }
        if (S->drawing && pen) {
            PenSample s = {ev->x, ev->y, fmaxf(ev->pressure, 0.5f), 0, 0, ev->t_ms};
            Rect saved = a->screen.clip;
            canvas_set_clip(&a->screen, S->pad);
            Rect d = brush_add(&S->bs, &a->screen, &s);
            a->screen.clip = saved;
            app_invalidate(a, rect_intersect(d, S->pad), REFRESH_FAST);
            if (ph == 2) S->drawing = false;
            return;
        }
        if (ph != 2) return;
        switch (button_at(a, ev->x, ev->y)) {
        case B_SAVE:
            a->settings.pen_calib = S->new_pen;
            if (S->new_touch.valid) a->settings.touch_calib = S->new_touch;
            settings_save(&a->settings);
            input_set_calib(&a->settings.pen_calib, a->settings.touch_calib.valid ? &a->settings.touch_calib : NULL);
            leave_to(a);
            app_toast(a, "Calibration saved", 2000);
            break;
        case B_REDO: restore_old(a); restart(a, PH_PEN, ""); break;
        case B_CANCEL: restore_old(a); leave_to(a); break;
        default: break;
        }
        return;
    }
    if (ph != 2) return;
    bool target_input = (S->phase == PH_PEN) == pen;
    if (!target_input) {
        // The other input operates the buttons.
        int b = button_at(a, ev->x, ev->y);
        if (b == B_CANCEL) {
            restore_old(a);
            leave_to(a);
        } else if (b == B_SKIP) {
            S->new_touch = (InputCalib){0};
            restart(a, PH_VERIFY, "");
        }
        return;
    }
    float px, py;  // the calibration maps the device to the panel, whatever the rotation
    input_logical_to_panel(k_tx[S->target] * a->w, k_ty[S->target] * a->h, &px, &py);
    S->pts[S->target] = (CalPoint){ev->nx, ev->ny, px, py};
    S->msg[0] = 0;
    if (++S->target >= NT) finish_pass(a);
    else draw(a);
}

void calibrate_tick(App *a, uint32_t now) {}
