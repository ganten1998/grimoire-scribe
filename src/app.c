// app.c: screen switching, the dirty-rect accumulator, toasts.
#include "app.h"
#include "annotate.h"
#include "bgimage.h"
#include "bgjob.h"
#include "calib_fit.h"
#include "i18n.h"

#include "light.h"
#include "power.h"
#include "templates.h"

#include <stdio.h>
#include <stdlib.h>
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
static void app_leave(App *a);
static App *g_probe_app;  // for app_probe (one App per process)

// The app that started this session and gets the screen back after it: "koreader" when
// KOReader's Grimoire plugin did (kindle/app/grimoire.sh --hosted koreader sets GRIMOIRE_HOST),
// "" for a start from the Kindle's Library.
static const char *app_host(void) {
    const char *h = getenv("GRIMOIRE_HOST");
    return h && !strcmp(h, "koreader") ? "koreader" : "";
}

static int g_sleep_shown;  // the sleep screen app_sleep last chose, for app_probe's "asleep.mode"

// The sleep screen to use. Hosted, KOReader keeps the Kindle UI paused for its whole session and
// the launcher never wakes it, so the Kindle's own screensaver (mode 2) cannot show: the page
// does instead.
static int sleep_mode(const App *a) {
    if (a->settings.sleep_mode != 2 || !*app_host()) return a->settings.sleep_mode;
    static bool told;
    if (!told) {
        told = true;
        qlog("sleep: the Kindle's own screensaver needs the Kindle UI, which KOReader keeps paused: the page instead");
    }
    return 0;
}

const char *app_mirror_dir(void) {
    static char buf[600];
    snprintf(buf, sizeof buf, "%s/PDFs", store_root());
    return buf;
}

// A calibration maps to the pixels of the panel it was made on. One made on a panel of another size
// (the grimoire folder copied over from another Scribe) is scaled to this one, w x h upright, before
// it is used, and recorded as this panel's (pen.panel, touch.panel) from the next settings save on.
static void calib_follow_panel(InputCalib *c, int *pw, int *ph, int w, int h, const char *what) {
    if (!c->valid) return;
    bool recorded = *pw > 0 && *ph > 0;
    int rw = recorded ? *pw : 1860, rh = recorded ? *ph : 2480;  // the panel on record (or taken as it)
    const char *rec = recorded ? "" : " (not recorded: taken as an 1860x2480 Scribe's)";
    int fw = *pw, fh = *ph;  // the panel it was made on, its own scale says (calib_to_panel)
    switch (calib_to_panel(c, &fw, &fh, w, h)) {
    case CALIB_SCALED:
        if (fw == rw && fh == rh)
            qlog("calibration: the %s calibration was made on a %dx%d panel%s; scaled to this %dx%d one", what, fw, fh,
                 rec, w, h);
        else
            qlog("calibration: the %s calibration was made on a %dx%d panel%s, but its scale fits a %dx%d one: scaled "
                 "from that to this %dx%d one", what, rw, rh, rec, fw, fh, w, h);
        break;
    case CALIB_MADE_HERE:
        qlog("calibration: the %s calibration was made on a %dx%d panel%s, but its scale fits this %dx%d one: made "
             "here, kept as it is", what, rw, rh, rec, w, h);
        break;
    default: break;
    }
    *pw = w;
    *ph = h;
}

// A page KOReader's Grimoire plugin sent to annotate (annotate.h), at startup, before anything opens
// a notebook: the editor opens on its page. kindle/app/grimoire.sh asks for it (GRIMOIRE_ANNOTATE=1)
// only when KOReader hosts Grimoire; a request this start was not asked to open is deleted
// unopened. True when the page could not be opened, with why in msg, for app_init's startup toast.
static bool annotate_at_start(App *a, char *msg, size_t cap) {
    const char *asked = getenv("GRIMOIRE_ANNOTATE");
    AnnotateResult r;
    annotate_take(asked && !strcmp(asked, "1") && *app_host(), &r);
    if (r.outcome == ANNOTATE_OPENED || r.outcome == ANNOTATE_MADE) {
        editor_open_notebook_at(a, r.nb, r.pid, r.outcome == ANNOTATE_MADE);
        return false;
    }
    if (r.outcome != ANNOTATE_FAILED) return false;
    switch (r.why) {
    case ANNOTATE_WHY_PICTURE:
        // TRANSLATORS: notices when Grimoire opens from KOReader's "Annotate this page in Grimoire"
        // and that page cannot be opened, in a toast: one line of about 1700 px at 44 px in bold, or
        // two. This one: the picture of the page KOReader sent is damaged or missing.
        snprintf(msg, cap, "%s", _("Could not open the page KOReader sent: its picture does not read."));
        break;
    case ANNOTATE_WHY_STORAGE:
        // TRANSLATORS: (as above) the page could not be added to its notebook: a file could not be
        // written.
        snprintf(msg, cap, "%s", _("Could not add the page KOReader sent. Is the storage full?"));
        break;
    default:
        // TRANSLATORS: (as above) any other reason; %s is the log file that says why.
        snprintf(msg, cap, _("Could not open the page KOReader sent (see %s)."), "grimoire.log");
        break;
    }
    return true;
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
    qlog("settings: sleep_mode %d, sleep_minutes %d, side_button %d", a->settings.sleep_mode, a->settings.sleep_minutes,
         a->settings.side_button);
    Settings *st = &a->settings;
    calib_follow_panel(&st->pen_calib, &st->pen_panel_w, &st->pen_panel_h, w, h, "pen");
    calib_follow_panel(&st->touch_calib, &st->touch_panel_w, &st->touch_panel_h, w, h, "touch");
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
    // The owner's pictures (templates.h): the templates and import folders, and the folders of page
    // pictures copied into import/ since the last run, made into notebooks in the background.
    templates_init();
    import_scan(dpi);
    const InputCaps *caps = input_caps();
    if (caps && caps->has_orientation && a->settings.follow_rotation && caps->orientation != 0)
        app_set_rotation(a, caps->orientation);
    // A page KOReader sent to annotate: the editor opens on it now, under the toast below. When it
    // could not be opened, why is that toast, unless the passcode was left off.
    char note[256];
    bool annotate_failed = annotate_at_start(a, note, sizeof note);
    // The launcher pauses the Kindle's passcode while Grimoire runs (kindle/app/grimoire.sh says
    // why) and says so here; "left-off": an earlier session ended without putting it back (told
    // at up to three starts while it stays off). That one goes first: the Kindle is unprotected.
    // Then what became of the page KOReader just sent; the others wait for a later start.
    const char *passcode = getenv("GRIMOIRE_PASSCODE");
    char msg[256];
    if (passcode && !strcmp(passcode, "left-off")) {
        qlog("device passcode: left off by an earlier session; telling the user");
        // TRANSLATORS: notices at startup, in a toast: one line of about 1700 px at 44 px in bold,
        // or two. "Kindle Settings" is the Kindle's own Settings, as its menus name them.
        app_toast(a, _("Your Kindle passcode was left off. Turn it back on in Kindle Settings."), 10000);
    } else if (annotate_failed) {
        app_toast(a, note, 8000);
    } else if (caps && !caps->has_pen) {
        // TRANSLATORS: %s is the app's name, Grimoire.
        snprintf(msg, sizeof msg, _("No pen found. %s is made for the Kindle Scribe; writing needs its pen."), APP_NAME);
        app_toast(a, msg, 10000);
    } else if (passcode && !strcmp(passcode, "paused") && !a->settings.passcode_told) {
        a->settings.passcode_told = true;
        // TRANSLATORS: %s is the app's name, Grimoire.
        snprintf(msg, sizeof msg, _("Your Kindle passcode is paused while %s is open."), APP_NAME);
        app_toast(a, msg, 8000);
    } else if (!a->settings.pen_calib.valid) {
        // TRANSLATORS: Settings is the Library's Settings button, and Calibrate pen the row of its
        // sheet that says "Calibrate pen and touch": name them as they are translated.
        app_toast(a, _("Tip: Settings > Calibrate pen if strokes land off the tip"), 6000);
    }
    return 0;
}

void app_shutdown(App *a) {
    if (a->scr == SCR_EDITOR) editor_save_now(a);
    k_screens[a->scr].leave(a);
    settings_save(&a->settings);
    for (int i = 0; i < SCR_COUNT; i++) k_screens[i].free_(a);
    // Queued PDF exports finish on their own after Grimoire exits. After editor_free: closing the
    // open notebook (leaving straight from the editor) queues its mirror PDF.
    bg_detach_rest();
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
        // sleep screen); just make sure nothing is lost. What the press needs from Grimoire is
        // decided at its release, or at the 4 s hold, which leaves Grimoire.
        qlog("power button: saving");
        if (a->scr == SCR_EDITOR) editor_save_now(a);
        // It replaces the press before it: a fallback that one left waiting must not go off while
        // this key is down (a sleep request in the middle of a hold: press, see nothing, then hold).
        // Its own release arms whatever it needs.
        a->power_pressed_at = 0;
        a->wake_fallback_at = 0;
        a->power_down_at = now_ms() | 1;
        a->power_down_asleep = a->asleep;
        a->power_answered = false;
        return;
    }
    if (ev->type == IN_POWER_UP) {
        if (ev->value >= 0) qlog("power button: up after %d ms", ev->value);
        else qlog("power button: up (its release was missed, so how long it was held is not known)");
        bool followed = a->power_down_at != 0;
        a->power_down_at = 0;
        // A press the power service already answered needs nothing more: a wake announced while
        // the key was still down must not turn the release into a sleep request (that made the
        // Kindle fall asleep again right after waking).
        switch (power_up_action(followed, a->power_answered, a->power_down_asleep, a->asleep, ev->value)) {
        case POWER_UP_SLEEP_WAIT: a->power_pressed_at = now_ms() | 1; break;          // expect goingToScreenSaver soon
        case POWER_UP_WAKE_WAIT: a->wake_fallback_at = (now_ms() + 3000) | 1; break;  // and outOfScreenSaver
        case POWER_UP_LEAVE:
            qlog("power button held %d ms, but Grimoire was busy at the %d s mark: saving and leaving Grimoire",
                 ev->value, INPUT_POWER_HOLD_MS / 1000);
            app_leave(a);
            break;
        default: break;
        }
        return;
    }
    if (ev->type == IN_POWER_HELD) {
        // The way out that always works: a locked Kindle, a panel that shows nothing, any screen.
        qlog("power button held %d s: saving and leaving Grimoire", INPUT_POWER_HOLD_MS / 1000);
        app_leave(a);
        return;
    }
    if (ev->type == IN_SLEEP) {
        a->power_pressed_at = 0;
        a->power_answered = true;
        app_sleep(a);
        return;
    }
    if (ev->type == IN_WAKE) {
        a->power_pressed_at = 0;
        a->power_answered = true;
        if (!a->asleep) {
            // Out of a screensaver Grimoire never saw start: still take the screen back.
            qlog("wake: out of the screensaver while Grimoire thought it was awake; repainting");
            a->asleep = true;
        }
        app_wake(a);
        return;
    }
    // While the Kindle shows its sleep screen, pen and touch are not Grimoire's: drawing over the
    // screensaver because the panel reported a touch is how a wrong screen ended up on it. The side
    // button and pinches start nothing either (three clicks cleared the page); only their ends get
    // through, so one that began before the sleep still finishes.
    if (a->asleep && power_asleep_drops(ev)) return;
    if (!a->asleep && (app_pointer_phase(ev) >= 0 || ev->type == IN_PINCH_BEGIN || ev->type == IN_PEN_BUTTON)) {
        a->last_input = now_ms();  // (event times can run slightly ahead of now_ms)
        a->sleep_requested = false;
        // Grimoire grabs the pen and touch: tell the power service someone is here.
        if (a->last_input - a->last_activity_poke > 60000) {
            a->last_activity_poke = a->last_input;
            sys_activity();
        }
    }
    k_screens[a->scr].event(a, ev);
}

// The toast is never stored in a->screen: it is overlaid just for the refresh and the pixels
// underneath are put straight back, so no screen redraw can ever capture it as "background".
static void toast_hide(App *a) {
    if (!a->toast_visible) return;
    a->toast_visible = false;
    g_self_dirty = rect_union(g_self_dirty, a->toast_rect);
}

// Where the toast goes: its usual place near the bottom, or, when the screen keeps the bottom
// for its controls (the keyboard, a sheet's buttons), just above them, so their labels stay
// readable while it shows.
static Rect toast_place(const App *a) {
    Rect r = ui_toast_rect((Rect){0, 0, a->w, a->h}, a->toast);
    int gap = ui_px(20);
    if (a->toast_floor > 0 && r.y + r.h + gap > a->toast_floor) r.y = a->toast_floor - r.h - gap;
    if (r.y < gap) r.y = gap;
    return r;
}

static void toast_draw(App *a) {
    a->toast_rect = toast_place(a);
    a->toast_visible = true;
    g_self_dirty = rect_union(g_self_dirty, a->toast_rect);
}

// A toast already up when the screen's floor changed (a sheet opened under it) moves: where it
// was is repainted from the screen, and it is shown in its new place.
static void toast_follow(App *a) {
    if (!a->toast_visible) return;
    Rect r = toast_place(a);
    if (r.x == a->toast_rect.x && r.y == a->toast_rect.y && r.w == a->toast_rect.w && r.h == a->toast_rect.h) return;
    g_self_dirty = rect_union(g_self_dirty, rect_union(a->toast_rect, r));
    a->toast_rect = r;
}

void app_toast(App *a, const char *msg, int ms) {
    toast_hide(a);
    i18n_copy(a->toast, msg ? msg : "", sizeof a->toast);  // (too long: cut between two characters)
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
    if (a->asleep) {
        // Already on the sleep screen, so a press that was waiting for a wake has its answer: the
        // Kindle went (back) to sleep. The fallback must not wake Grimoire over it.
        a->wake_fallback_at = 0;
        return;
    }
    a->asleep = true;
    a->wake_redraw_at = 0;
    a->wake_fallback_at = 0;
    if (a->scr == SCR_EDITOR) editor_save_now(a);
    if (a->scr == SCR_SHARE) share_sleep(a);
    qlog("sleep: mode %d", a->settings.sleep_mode);
    int mode = sleep_mode(a);
    g_sleep_shown = mode;
    if (mode == 2) {
        sys_kindle_ui(true);
        return;
    }
    Canvas *c = &a->screen;
    Canvas pinned = {0};
    const Canvas *src = NULL;
    if (mode == 1 && store_load_sleep_image(&pinned) == 0) src = &pinned;
    if (!src) src = editor_page_canvas(a);
    if (src) {
        canvas_reset_clip(c);
        canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
        canvas_blit_fit(c, src, (Rect){0, 0, a->w, a->h});
    } else if (a->scr == SCR_SHARE) {
        canvas_reset_clip(c);
        share_draw_asleep(a);  // not the code: nothing answers it while asleep
    }  // (else, with no page open, the screen as it is, e.g. the library, stays up)
    display_present(c, (Rect){0, 0, a->w, a->h}, REFRESH_FULL);
    display_wait();
    canvas_free(&pinned);
}

static void app_wake(App *a) {
    if (!a->asleep) return;
    a->asleep = false;
    a->wake_fallback_at = 0;
    a->power_pressed_at = 0;  // a press taken for a sleep press was this wake's: no sleep request after it
    a->locked_wakes = 0;      // it woke, so it was not locked
    a->last_input = now_ms();
    a->sleep_requested = false;
    qlog("wake");
    int mode = sleep_mode(a);
    if (mode == 2) sys_kindle_ui(false);
    // A sleep request turned preventScreenSaver off: "Sleep after: never" wants it back on.
    app_apply_sleep_policy(a);
    // Back to work at once (no-flash refresh), then once more a moment later in case the Kindle
    // UI (its screensaver closing, the status bar) drew something on the way out.
    app_redraw_mode(a, REFRESH_UI);
    a->wake_redraw_at = now_ms() + (mode == 2 ? 2500 : 1500);
}

// Saves and leaves Grimoire: the way out of any stuck state (a 4 s hold of the power button, a
// Kindle that stays locked). The launcher's exit trap gives the Kindle its UI back. A page that
// cannot be written (a full store) is never lost unannounced: awake, the first try says so and
// stays, and another within a minute leaves without the page. Asleep (no toast can be seen, and
// the Kindle needs its pen and touch back) it leaves at once, and the log says what was lost.
static void app_leave(App *a) {
    a->power_down_at = 0;  // the press is spent: its release, if it is read at all, asks for nothing
    bool saved = a->scr != SCR_EDITOR || editor_save_now(a);
    uint32_t now = now_ms();
    if (!saved) qlog("leaving: the open page could not be saved");
    if (!a->asleep) {
        bool warned = a->leave_unsaved_at && now - a->leave_unsaved_at < 60000;
        if (!saved && !warned) {
            a->leave_unsaved_at = now | 1;
            qlog("leaving: staying this once (holding the power button 4 s again within a minute leaves without the page)");
            // TRANSLATORS: holding the power button 4 s leaves Grimoire, but this page could not be
            // saved (a full storage), so it stays once.
            app_toast(a, _("Could not save this page. Hold power 4 s again to leave anyway."), 10000);
            return;
        }
        char msg[256];
        // TRANSLATORS: the last notice before Grimoire closes, with the page saved, or without it;
        // %s is the app's name, Grimoire.
        snprintf(msg, sizeof msg, saved ? _("Saved. Leaving %s.") : _("Leaving %s without this page."), APP_NAME);
        app_toast(a, msg, 3000);  // shown by the last flush
    }
    a->running = false;
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
    a->dirty = RECT_EMPTY;
    // Re-lay out the current screen in the new shape.
    a->relayout = true;
    k_screens[a->scr].leave(a);
    k_screens[a->scr].enter(a);
    a->relayout = false;
    if (a->toast_visible) a->toast_rect = toast_place(a);  // the new shape (and the floor its enter set)
    app_invalidate_all(a, REFRESH_FULL);
}

void app_apply_sleep_policy(App *a) {
    int want = a->settings.sleep_minutes == 0;
    if (want == a->prevent_applied) return;
    a->prevent_applied = want;
    sys_prevent_sleep(want);
}

// Asks the power service to sleep. The request turns preventScreenSaver off first (the button is
// ignored while it is on); app_wake puts a "never" policy back.
static void request_sleep(App *a) {
    sys_request_sleep();
    a->prevent_applied = 0;
}

// The power service's state, for a fallback to decide on and log ("unknown" if unreadable). It
// blocks a moment (lipc), which only these rare fallbacks can afford.
static PowerdState powerd_state_now(char *name, size_t cap) {
    if (sys_powerd_state(name, cap) != 0) {
        strlcpy_q(name, "unknown", cap);
        return POWERD_UNKNOWN;
    }
    return powerd_parse_state(name);
}

// A background job that finished: an import says how it went (the Library shows its new notebook),
// a template's picture may now be there to draw. Returns bg_poll's result for the PDF mirror's toast.
static int poll_jobs(App *a) {
    int done = -1, r = bg_poll_kind(&done);
    if (done == BG_TEMPLATE) {
        bgimage_retry_missing();
        editor_picture_made(a);  // (the open page may be on that picture: a page KOReader sent)
    }
    if (done != BG_IMPORT_FOLDER) return r;
    int pages = 0, got = import_finished(r > 0, &pages);
    if (got > 0 && a->scr == SCR_LIBRARY && !a->asleep) app_redraw_mode(a, REFRESH_UI);  // (waking redraws it too)
    if (got > 0) {
        char msg[256];
        // TRANSLATORS: a folder of page pictures (grimoire/import/<name>) became a new notebook.
        snprintf(msg, sizeof msg, NG_("Imported a notebook of %d page.", "Imported a notebook of %d pages.", pages), pages);
        app_toast(a, msg, 8000);
    } else if (got < 0) {
        char msg[256];
        // TRANSLATORS: a folder of page pictures could not become a notebook; %s is the log file
        // that says why. The folder stays where it was.
        snprintf(msg, sizeof msg, _("Could not import a folder of pages (see %s)."), "grimoire.log");
        app_toast(a, msg, 8000);
    }
    return 0;
}

void app_tick(App *a, uint32_t now) {
    int bg = poll_jobs(a);
    if (bg) {
        char msg[256];
        // TRANSLATORS: a PDF saved in the background, and where (the Kindle's folder of them).
        if (bg > 0) snprintf(msg, sizeof msg, _("PDF saved to %s"), "grimoire/PDFs");
        app_toast(a, bg > 0 ? msg : _("Could not save the PDF"), 2500);
    }
    if (store_take_corrupt_count() > 0) {
        char msg[256];
        // TRANSLATORS: a page that could not be read; %s is the log file that says which.
        snprintf(msg, sizeof msg, _("A damaged page was set aside; a blank one took its place (see %s)"), "grimoire.log");
        app_toast(a, msg, 6000);
    }
    // A short power press the power service did not follow with a sleep announcement. If it is in
    // its screensaver already, Grimoire missed the announcement: show the sleep screen and send
    // nothing (a powerButton now would wake the Kindle again). Otherwise ask it to sleep.
    if (a->power_pressed_at && !a->asleep && (int32_t)(now - a->power_pressed_at) > 2500) {
        a->power_pressed_at = 0;
        char st[48];
        if (powerd_dozing(powerd_state_now(st, sizeof st))) {
            qlog("sleep: no announcement after the power button, but the power service is in %s; going to sleep "
                 "without asking", st);
            app_sleep(a);
        } else {
            qlog("sleep: no announcement after the power button (power service: %s); requesting sleep", st);
            request_sleep(a);
        }
    }
    // A press on the sleep screen with no outOfScreenSaver after it. A power service still in its
    // screensaver means the Kindle is still locked (its passcode) or asleep: never draw over it.
    // The second time in a row Grimoire leaves instead, so the Kindle gets its pen and touch back
    // even where the 4 s hold cannot be seen (a power key that reports press and release at once).
    if (a->asleep && a->wake_fallback_at && (int32_t)(now - a->wake_fallback_at) >= 0) {
        a->wake_fallback_at = 0;
        char st[48];
        switch (power_wake_fallback(powerd_state_now(st, sizeof st), &a->locked_wakes)) {
        case WAKE_FALLBACK_STAY:
            qlog("wake: no outOfScreenSaver within 3 s of the power button, and the power service is in %s: the "
                 "Kindle is still locked or asleep, so Grimoire stays on its sleep screen (another press that "
                 "brings no wake, or holding the power button 4 s, leaves Grimoire)", st);
            break;
        case WAKE_FALLBACK_LEAVE:
            qlog("wake: again no outOfScreenSaver within 3 s of the power button, and the power service is in %s: "
                 "the Kindle stays locked, so Grimoire saves and leaves (the Kindle can then ask for its passcode)",
                 st);
            app_leave(a);
            break;
        default:
            qlog("wake: power pressed but no outOfScreenSaver within 3 s (power service: %s); waking anyway", st);
            app_wake(a);
            break;
        }
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
        request_sleep(a);
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
    toast_follow(a);  // (a screen that moves its floor redraws, so there is something to flush)
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
            ui_toast_at(&a->screen, a->toast_rect, a->toast);
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
    a->toast_floor = 0;  // the new screen sets its own
    k_screens[s].enter(a);
    app_invalidate_all(a, REFRESH_FULL);  // a visible toast is overlaid again at the flush
}

void app_press_feedback(App *a, Rect r) {
    canvas_invert(&a->screen, r);
    display_present(&a->screen, r, REFRESH_FAST);
}

// The runner's annotate.* keys: "role", the open page's paper ("document" for a page KOReader sent
// or an imported one, "template", "" for any other); "luma", the open page as the editor holds it
// now, its mean (255: white, as a picture still being made draws); the rest are annotate_probe's,
// for the page open in the editor.
static int annotate_app_probe(App *a, const char *key, char *out, size_t cap) {
    char v[32];
    if (!strcmp(key, "role")) {
        PaperKind k = (PaperKind)(editor_probe(a, "paper", v, sizeof v) == 0 ? (uint32_t)strtoul(v, NULL, 10) : 0u);
        snprintf(out, cap, "%s", paper_is_document(k) ? "document" : paper_is_image(k) ? "template" : "");
        return 0;
    }
    if (!strcmp(key, "luma")) {
        const Canvas *c = editor_page_canvas(a);
        uint64_t sum = 0, np = c ? (uint64_t)c->w * (uint64_t)c->h : 0;
        for (int y = 0; c && y < c->h; y++)
            for (int x = 0; x < c->w; x++) sum += c->px[(size_t)y * (size_t)c->stride + (size_t)x];
        if (np) snprintf(out, cap, "%d", (int)((sum + np / 2) / np));
        else snprintf(out, cap, "%s", "");
        return 0;
    }
    int page = editor_probe(a, "page", v, sizeof v) == 0 ? atoi(v) : 0;
    return annotate_probe(key, page - 1, out, cap);
}

int app_probe(const char *key, char *out, size_t cap) {
    static const char *const k_names[SCR_COUNT] = {"library", "editor", "penset", "calib", "share"};
    App *a = g_probe_app;
    if (!a || !key || !out || cap == 0) return -1;
    if (!strncmp(key, "annotate.", 9)) return annotate_app_probe(a, key + 9, out, cap);
    if (!strncmp(key, "editor.", 7)) return editor_probe(a, key + 7, out, cap);
    if (!strncmp(key, "library.", 8)) return library_probe(a, key + 8, out, cap);
    if (!strncmp(key, "penset.", 7)) return pensettings_probe(a, key + 7, out, cap);
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
        {"settings.eraser_mode", st->eraser_mode},
        {"settings.white", st->white},
        {"bg.busy", bg_busy()},  // a background job (an export, a compaction) runs or waits
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
    // A tool's pen feel, as settings.ini writes it: "settings.ballpoint.size_max_mm".
    const char *tool = !strncmp(key, "settings.", 9) ? key + 9 : NULL, *dot = tool ? strchr(tool, '.') : NULL;
    for (int t = 0; dot && t < TOOL_COUNT; t++) {
        const char *name = app_probe_tool((ToolKind)t);
        if (!name || strlen(name) != (size_t)(dot - tool) || strncmp(tool, name, strlen(name))) continue;
        int n;
        const ParamInfo *pi = tool_param_table(&n);
        for (int i = 0; i < n; i++)
            if (!strcmp(dot + 1, pi[i].key)) {
                snprintf(out, cap, "%.4f", *tool_param_ptr(&a->settings.tools[t], &pi[i]));
                return 0;
            }
    }
    if (!strcmp(key, "screen")) snprintf(out, cap, "%s", k_names[a->scr]);
    else if (!strcmp(key, "toast")) snprintf(out, cap, "%s", a->toast_visible ? a->toast : "");
    else if (!strcmp(key, "toast.rect")) {
        Rect r = a->toast_rect;
        if (a->toast_visible) snprintf(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        else snprintf(out, cap, "%s", "");
    }
    else if (!strcmp(key, "asleep")) snprintf(out, cap, "%d", a->asleep ? 1 : 0);
    else if (!strcmp(key, "asleep.mode")) {
        if (a->asleep) snprintf(out, cap, "%d", g_sleep_shown);
        else snprintf(out, cap, "%s", "");
    }
    else if (!strcmp(key, "host")) snprintf(out, cap, "%s", app_host());
    else if (!strcmp(key, "rotation")) snprintf(out, cap, "%d", a->rotation);
    else if (!strcmp(key, "size")) snprintf(out, cap, "%dx%d", a->w, a->h);
    else if (!strcmp(key, "settings.pen.panel") || !strcmp(key, "settings.touch.panel")) {
        bool pen = !strcmp(key, "settings.pen.panel");
        int pw = pen ? st->pen_panel_w : st->touch_panel_w, ph = pen ? st->pen_panel_h : st->touch_panel_h;
        if (pw > 0) snprintf(out, cap, "%dx%d", pw, ph);
        else snprintf(out, cap, "%s", "");
    }
    else if (!strcmp(key, "settings.theme")) snprintf(out, cap, "%s", st->theme == THEME_CLASSIC ? "classic" : "gothic");
    else if (!strcmp(key, "settings.language")) snprintf(out, cap, "%s", st->language);  // "auto" for one it lacks
    else if (!strcmp(key, "language")) snprintf(out, cap, "%s", i18n_current());  // the language in use
    else return -1;
    return 0;
}

void app_probe_music(const MusicLayout *m, char *out, size_t cap) {
    // The parts by short names (src/paper.h order); a part missing here shows as its number.
    static const char *const parts[PART_COUNT] = {
        [PART_TREBLE] = "treble", [PART_BASS] = "bass",   [PART_ALTO] = "alto", [PART_TENOR] = "tenor",
        [PART_PERC] = "drums",    [PART_STAFF] = "plain", [PART_TAB4] = "tab4", [PART_TAB5] = "tab5",
        [PART_TAB6] = "tab6",     [PART_TAB7] = "tab7",   [PART_LYRICS] = "lyrics",
    };
    if (cap == 0) return;
    out[0] = 0;
    if (!m) return;
    size_t n = (size_t)snprintf(out, cap, "%d:", m->systems);
    for (int i = 0; i < m->n && i < MUSIC_MAX_PARTS && n < cap; i++) {
        int p = (int)m->part[i];
        const char *name = p > 0 && p < PART_COUNT ? parts[p] : NULL;
        n += name ? (size_t)snprintf(out + n, cap - n, " %s", name) : (size_t)snprintf(out + n, cap - n, " %d", p);
    }
}
