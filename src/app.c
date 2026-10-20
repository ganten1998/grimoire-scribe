// app.c: screen switching, the dirty-rect accumulator, toasts.
#include "app.h"
#include "bgjob.h"

#include "light.h"

#include <stdio.h>
#include <string.h>

// The app's own repaints (toast show/hide) are kept apart from the screens' so a toast
// never mistakes its own pixels for fresh screen content underneath it.
static Rect g_self_dirty;
typedef struct {
    void (*init)(App *);
    void (*free_)(App *);
    void (*enter)(App *);
    void (*leave)(App *);
    void (*event)(App *, const InEvent *);
    void (*tick)(App *, uint32_t);
} ScreenOps;

#define OPS(X) {X##_init, X##_free, X##_enter, X##_leave, X##_event, X##_tick}
static const ScreenOps k_screens[SCR_COUNT] = {
    [SCR_LIBRARY] = OPS(library),   [SCR_EDITOR] = OPS(editor), [SCR_PENSET] = OPS(pensettings),
    [SCR_CALIB] = OPS(calibrate), [SCR_SHARE] = OPS(share),
};
#undef OPS

static void app_sleep(App *a);
static void app_wake(App *a);
static App *g_probe_app;  // for app_probe (one App per process)

const char *app_mirror_dir(void) {
    static char buf[600];
    snprintf(buf, sizeof buf, "%s/PDFs", store_root());
    return buf;
}

int app_init(App *a, int w, int h, float dpi) {
    memset(a, 0, sizeof *a);
    g_probe_app = a;
    a->w = w;
    a->h = h;
    a->dpi = dpi;
    a->running = true;
    if (canvas_init(&a->screen, w, h) != 0) return -1;
    store_set_device(dpi, w, h);
    settings_load(&a->settings);
    if (a->settings.pen_calib.valid || a->settings.touch_calib.valid)
        input_set_calib(a->settings.pen_calib.valid ? &a->settings.pen_calib : NULL,
                        a->settings.touch_calib.valid ? &a->settings.touch_calib : NULL);
    ui_init(dpi);
    if (font_init() != 0) return -1;
    ui_set_theme((UiTheme)a->settings.theme);
    display_set_invert(a->settings.invert);
    for (int i = 0; i < SCR_COUNT; i++) k_screens[i].init(a);
    a->scr = SCR_LIBRARY;
    a->prev_scr = SCR_LIBRARY;
    k_screens[SCR_LIBRARY].enter(a);
    app_invalidate_all(a, REFRESH_FULL);
    a->pending_rotation = -1;
    a->last_input = now_ms();
    a->prevent_applied = -1;
    app_apply_sleep_policy(a);
    const InputCaps *caps = input_caps();
    if (caps && caps->has_orientation && a->settings.follow_rotation && caps->orientation != 0)
        app_set_rotation(a, caps->orientation);
    if (caps && !caps->has_pen)
        app_toast(a, "No pen found. " APP_NAME " is made for the Kindle Scribe; writing needs its pen.", 10000);
    else if (!a->settings.pen_calib.valid)
        app_toast(a, "Tip: Settings > Calibrate pen if strokes land off the tip", 6000);
    return 0;
}

void app_shutdown(App *a) {
    if (a->scr == SCR_EDITOR) editor_save_now(a);
    k_screens[a->scr].leave(a);
    bg_detach_rest();  // queued PDF exports finish on their own after Grimoire exits
    settings_save(&a->settings);
    for (int i = 0; i < SCR_COUNT; i++) k_screens[i].free_(a);
    canvas_free(&a->toast_under);
    canvas_free(&a->screen);
    font_shutdown();
}

int app_pointer_phase(const InEvent *ev) {
    switch (ev->type) {
    case IN_PEN_DOWN:
    case IN_TOUCH_DOWN: return 0;
    case IN_PEN_MOVE:
    case IN_TOUCH_MOVE: return 1;
    case IN_PEN_UP:
    case IN_TOUCH_UP: return 2;
    default: return -1;
    }
}

void app_event(App *a, const InEvent *ev) {
    if (ev->type == IN_QUIT) {
        a->running = false;
        return;
    }
    if (ev->type == IN_ORIENTATION) {
        if (a->settings.follow_rotation) a->pending_rotation = ev->value & 3;
        return;
    }
    if (ev->type == IN_POWER) {
        // The power service handles the button itself (it announces the sleep, which draws the
        // sleep screen); just make sure nothing is lost. Quitting is Library > Exit.
        qlog("power button: saving");
        if (a->scr == SCR_EDITOR) editor_save_now(a);
        if (!a->asleep) a->power_pressed_at = now_ms() | 1;  // expect goingToScreenSaver soon
        else a->wake_fallback_at = (now_ms() + 3000) | 1;    // expect outOfScreenSaver soon
        return;
    }
    if (ev->type == IN_SLEEP) {
        a->power_pressed_at = 0;
        app_sleep(a);
        return;
    }
    if (ev->type == IN_WAKE) {
        if (!a->asleep) {
            // Out of a screensaver Grimoire never saw start: still take the screen back.
            qlog("wake: out of the screensaver while Grimoire thought it was awake; repainting");
            a->asleep = true;
        }
        app_wake(a);
        return;
    }
    if (app_pointer_phase(ev) >= 0 || ev->type == IN_PINCH_BEGIN || ev->type == IN_PEN_BUTTON) {
        a->last_input = now_ms();  // (event times can run slightly ahead of now_ms)
        a->sleep_requested = false;
        // Grimoire grabs the pen and touch: tell the power service someone is here.
        if (a->last_input - a->last_activity_poke > 60000) {
            a->last_activity_poke = a->last_input;
            sys_activity();
        }
    }
    // While the Kindle shows its sleep screen, pen and touch are not Grimoire's: drawing over the
    // screensaver because the panel reported a touch is how a wrong screen ended up on it.
    if (a->asleep && app_pointer_phase(ev) >= 0) return;
    k_screens[a->scr].event(a, ev);
}

// The toast is never stored in a->screen: it is overlaid just for the refresh and the pixels
// underneath are put straight back, so no screen redraw can ever capture it as "background".
static void toast_hide(App *a) {
    if (!a->toast_visible) return;
    a->toast_visible = false;
    g_self_dirty = rect_union(g_self_dirty, a->toast_rect);
}

static void toast_draw(App *a) {
    a->toast_rect = ui_toast_rect((Rect){0, 0, a->w, a->h}, a->toast);
    a->toast_visible = true;
    g_self_dirty = rect_union(g_self_dirty, a->toast_rect);
}

void app_toast(App *a, const char *msg, int ms) {
    toast_hide(a);
    strlcpy_q(a->toast, msg, sizeof a->toast);
    a->toast_until = now_ms() + (uint32_t)ms;
    toast_draw(a);
}

void app_redraw_mode(App *a, RefreshMode m) {
    a->toast_visible = false;
    a->dirty = RECT_EMPTY;
    a->relayout = true;
    k_screens[a->scr].leave(a);
    k_screens[a->scr].enter(a);
    a->relayout = false;
    app_invalidate_all(a, m);
}

void app_redraw(App *a) { app_redraw_mode(a, REFRESH_FULL); }

// The sleep screen: the page you were on (or the pinned page), so the Kindle "sleeps on" your
// notebook and wakes straight back into it. Mode 2 hands over to the Kindle's own screensaver.
static void app_sleep(App *a) {
    if (a->asleep) return;
    a->asleep = true;
    a->wake_redraw_at = 0;
    a->wake_fallback_at = 0;
    if (a->scr == SCR_EDITOR) editor_save_now(a);
    qlog("sleep: mode %d", a->settings.sleep_mode);
    if (a->settings.sleep_mode == 2) {
        sys_kindle_ui(true);
        return;
    }
    Canvas *c = &a->screen;
    Canvas pinned = {0};
    const Canvas *src = NULL;
    if (a->settings.sleep_mode == 1 && store_load_sleep_image(&pinned) == 0) src = &pinned;
    if (!src) src = editor_page_canvas(a);
    if (src) {
        canvas_reset_clip(c);
        canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
        canvas_blit_fit(c, src, (Rect){0, 0, a->w, a->h});
    }  // (no page open: the screen as it is, e.g. the library, stays up)
    display_present(c, (Rect){0, 0, a->w, a->h}, REFRESH_FULL);
    display_wait();
    canvas_free(&pinned);
}

static void app_wake(App *a) {
    if (!a->asleep) return;
    a->asleep = false;
    a->wake_fallback_at = 0;
    a->last_input = now_ms();
    a->sleep_requested = false;
    qlog("wake");
    if (a->settings.sleep_mode == 2) sys_kindle_ui(false);
    // Back to work at once (no-flash refresh), then once more a moment later in case the Kindle
    // UI (its screensaver closing, the status bar) drew something on the way out.
    app_redraw_mode(a, REFRESH_UI);
    a->wake_redraw_at = now_ms() + (a->settings.sleep_mode == 2 ? 2500 : 1500);
}

void app_set_rotation(App *a, int r) {
    r &= 3;
    int w, h;
    display_set_orientation(r, &w, &h);
    input_set_orientation(r);
    a->rotation = r;
    if (w != a->w || h != a->h) {
        Canvas nc;
        if (canvas_init(&nc, w, h) != 0) return;
        canvas_free(&a->screen);
        a->screen = nc;
        a->w = w;
        a->h = h;
    }
    if (a->toast_visible) a->toast_rect = ui_toast_rect((Rect){0, 0, w, h}, a->toast);
    a->dirty = RECT_EMPTY;
    // Re-lay out the current screen in the new shape.
    a->relayout = true;
    k_screens[a->scr].leave(a);
    k_screens[a->scr].enter(a);
    a->relayout = false;
    app_invalidate_all(a, REFRESH_FULL);
}

void app_apply_sleep_policy(App *a) {
    int want = a->settings.sleep_minutes == 0;
    if (want == a->prevent_applied) return;
    a->prevent_applied = want;
    sys_prevent_sleep(want);
}

void app_tick(App *a, uint32_t now) {
    int bg = bg_poll();
    if (bg) app_toast(a, bg > 0 ? "PDF saved to grimoire/PDFs" : "Could not save the PDF", 2500);
    if (store_take_corrupt_count() > 0)
        app_toast(a, "A damaged page was set aside and a blank one put in its place (see grimoire.log)", 6000);
    // A power press the power service did not follow with a sleep announcement: ask for sleep.
    if (a->power_pressed_at && !a->asleep && (int32_t)(now - a->power_pressed_at) > 2500) {
        a->power_pressed_at = 0;
        qlog("sleep: no announcement after the power button; requesting sleep");
        sys_prevent_sleep(false);
        a->prevent_applied = 0;
        sys_request_sleep();
    }
    if (a->asleep && a->wake_fallback_at && (int32_t)(now - a->wake_fallback_at) >= 0) {
        qlog("wake: power pressed but no outOfScreenSaver within 3 s; waking anyway");
        app_wake(a);
    }
    if (a->wake_redraw_at && (int32_t)(now - a->wake_redraw_at) >= 0 && !(a->scr == SCR_EDITOR && editor_busy(a))) {
        a->wake_redraw_at = 0;
        app_invalidate_all(a, REFRESH_UI);  // the screen is already right: just push it again
    }
    // Sleep after a while without input, the same way the power button does.
    if (!a->asleep && !a->sleep_requested && a->settings.sleep_minutes > 0 &&
        (int32_t)(now - a->last_input) >= a->settings.sleep_minutes * 60000 &&
        !(a->scr == SCR_EDITOR && editor_busy(a)) && a->scr != SCR_SHARE) {
        a->sleep_requested = true;
        sys_request_sleep();
    }
    if (a->asleep) return;
    if (a->pending_rotation >= 0 && a->pending_rotation != a->rotation && !(a->scr == SCR_EDITOR && editor_busy(a))) {
        int r = a->pending_rotation;
        a->pending_rotation = -1;
        app_set_rotation(a, r);
    } else if (a->pending_rotation == a->rotation) {
        a->pending_rotation = -1;
    }
    k_screens[a->scr].tick(a, now);
    if (a->toast_visible && (int32_t)(now - a->toast_until) >= 0) toast_hide(a);
}

void app_invalidate(App *a, Rect r, RefreshMode m) {
    r = rect_intersect(r, (Rect){0, 0, a->w, a->h});
    if (rect_is_empty(r)) return;
    // Keep the strongest mode requested since the last flush.
    if (rect_is_empty(a->dirty) || m > a->dirty_mode) a->dirty_mode = m;
    a->dirty = rect_union(a->dirty, r);
}

void app_invalidate_all(App *a, RefreshMode m) { app_invalidate(a, (Rect){0, 0, a->w, a->h}, m); }

void app_flush(App *a) {
    if (a->asleep) return;  // the sleep screen stays up until wake
    if (rect_is_empty(a->dirty) && rect_is_empty(g_self_dirty)) return;
    RefreshMode m = a->dirty_mode;
    if (!rect_is_empty(g_self_dirty)) {
        if (rect_is_empty(a->dirty) || m < REFRESH_UI) m = REFRESH_UI;
        a->dirty = rect_union(a->dirty, g_self_dirty);
        g_self_dirty = RECT_EMPTY;
    }
    if (m == REFRESH_UI && ++a->ui_refreshes_since_full >= 30) m = REFRESH_FULL;
    if (m == REFRESH_FULL) a->ui_refreshes_since_full = 0;
    bool overlay = a->toast_visible && rect_overlaps(a->dirty, a->toast_rect);
    if (overlay) {
        a->dirty = rect_union(a->dirty, a->toast_rect);
        canvas_free(&a->toast_under);
        overlay = canvas_init(&a->toast_under, a->toast_rect.w, a->toast_rect.h) == 0;
        if (overlay) {
            canvas_blit(&a->toast_under, &a->screen, a->toast_rect, 0, 0);
            ui_toast(&a->screen, (Rect){0, 0, a->w, a->h}, a->toast);
        }
    }
    display_present(&a->screen, a->dirty, m);
    if (overlay)
        canvas_blit(&a->screen, &a->toast_under, (Rect){0, 0, a->toast_rect.w, a->toast_rect.h}, a->toast_rect.x,
                    a->toast_rect.y);
    a->dirty = RECT_EMPTY;
    a->dirty_mode = REFRESH_FAST;
}

void app_switch(App *a, ScreenId s) {
    if (s < 0 || s >= SCR_COUNT) return;
    k_screens[a->scr].leave(a);
    a->prev_scr = a->scr;
    a->scr = s;
    k_screens[s].enter(a);
    app_invalidate_all(a, REFRESH_FULL);  // a visible toast is overlaid again at the flush
}

void app_press_feedback(App *a, Rect r) {
    canvas_invert(&a->screen, r);
    display_present(&a->screen, r, REFRESH_FAST);
}

int app_probe(const char *key, char *out, size_t cap) {
    static const char *const k_names[SCR_COUNT] = {"library", "editor", "penset", "calib", "share"};
    App *a = g_probe_app;
    if (!a || !key || !out || cap == 0) return -1;
    if (!strncmp(key, "editor.", 7)) return editor_probe(a, key + 7, out, cap);
    if (!strncmp(key, "library.", 8)) return library_probe(a, key + 8, out, cap);
    // Settings, under their settings.ini names.
    const Settings *st = &a->settings;
    const struct {
        const char *key;
        int v;
    } nums[] = {
        {"settings.invert", st->invert},
        {"settings.mirror_pdf", st->mirror_pdf},
        {"settings.follow_rotation", st->follow_rotation},
        {"settings.pinch_zoom", st->pinch_zoom},
        {"settings.side_button", st->side_button},
        {"settings.sleep_mode", st->sleep_mode},
        {"settings.sleep_minutes", st->sleep_minutes},
        {"settings.pen.valid", st->pen_calib.valid},
        {"settings.touch.valid", st->touch_calib.valid},
    };
    for (size_t i = 0; i < sizeof nums / sizeof nums[0]; i++)
        if (!strcmp(key, nums[i].key)) {
            snprintf(out, cap, "%d", nums[i].v);
            return 0;
        }
    if (!strcmp(key, "light.intensity") || !strcmp(key, "light.warmth")) {
        int li, lw;
        light_get(&li, &lw);
        snprintf(out, cap, "%d", !strcmp(key, "light.intensity") ? li : lw);
        return 0;
    }
    if (!strcmp(key, "screen")) snprintf(out, cap, "%s", k_names[a->scr]);
    else if (!strcmp(key, "toast")) snprintf(out, cap, "%s", a->toast_visible ? a->toast : "");
    else if (!strcmp(key, "asleep")) snprintf(out, cap, "%d", a->asleep ? 1 : 0);
    else if (!strcmp(key, "rotation")) snprintf(out, cap, "%d", a->rotation);
    else if (!strcmp(key, "size")) snprintf(out, cap, "%dx%d", a->w, a->h);
    else if (!strcmp(key, "settings.theme")) snprintf(out, cap, "%s", st->theme == THEME_CLASSIC ? "classic" : "gothic");
    else return -1;
    return 0;
}
