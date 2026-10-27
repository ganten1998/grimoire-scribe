// main.c: entry point for the Kindle build and the desktop simulator.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "i18n.h"
#include "light.h"

#ifdef GRIMOIRE_KINDLE
#include <ctype.h>
#include <fcntl.h>
#include <unistd.h>
// The launcher's watchdog kills Grimoire and restores the Kindle UI if this file goes stale,
// so a hang can never leave the device stuck with its UI paused.
#define HEARTBEAT "/tmp/grimoire.alive"
static void heartbeat(void) {
    int fd = open(HEARTBEAT, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) {
        if (write(fd, "1", 1) < 0) { /* best effort */ }
        close(fd);
    }
}

// The value of the first "<key> : value" line of a /proc file, trimmed ("?" if absent).
static void proc_value(const char *file, const char *key, char *out, size_t cap) {
    strlcpy_q(out, "?", cap);
    FILE *f = fopen(file, "r");
    if (!f) return;
    char line[256];
    size_t kl = strlen(key);
    while (fgets(line, sizeof line, f)) {
        if (strncmp(line, key, kl) || (line[kl] != ':' && !isspace((unsigned char)line[kl]))) continue;
        char *v = strchr(line, ':');
        if (!v) continue;
        for (v++; isspace((unsigned char)*v); v++) {
        }
        v[strcspn(v, "\r\n")] = 0;
        strlcpy_q(out, v, cap);
        break;
    }
    fclose(f);
}

// What the device is, at the top of grimoire.log, so a tester's log answers the questions about
// their Kindle. Nothing that identifies the unit or a network: people attach this log to public
// issues, so /proc/usid gives only its first 6 characters (the model code, not the serial number).
static void log_device(void) {
    char code[8] = "?";
    FILE *f = fopen("/proc/usid", "r");
    if (f) {
        size_t n = fread(code, 1, 6, f);
        code[n] = 0;
        for (size_t i = 0; i < n; i++)
            if (!isalnum((unsigned char)code[i])) code[i] = '?';
        fclose(f);
    }
    char total[32], avail[32], part[32], hw[64];
    proc_value("/proc/meminfo", "MemTotal", total, sizeof total);
    proc_value("/proc/meminfo", "MemAvailable", avail, sizeof avail);
    proc_value("/proc/cpuinfo", "CPU part", part, sizeof part);
    proc_value("/proc/cpuinfo", "Hardware", hw, sizeof hw);
    qlog("device: model code %s (the first 6 characters of /proc/usid)", code[0] ? code : "?");
    qlog("device: memory %s total, %s available; CPU part %s, hardware %s", total, avail, part, hw);
}

// The loop's watchdog: a phase that held the thread over 50 ms (beyond the wait it is allowed) gets
// a line. Outside the work pump's reach (common.h: a popen, a fork, an fsync, code that does not pump)
// nothing reads the pens meanwhile, and the kernel keeps only 50-175 ms of them
// (docs/INK-PERFORMANCE.md, Reader stalls).
static void phase_done(const char *name, uint32_t *since, uint32_t wait_ms) {
    uint32_t t = now_ms(), took = t - *since;
    if (took > wait_ms + 50) qlog("main: %s took %u ms", name, (unsigned)took);
    *since = t;
}
#define PHASE(name, since, wait_ms) phase_done(name, since, wait_ms)

// What the watchdog calls an event (InType's order) and a screen (ScreenId's): a slow batch names its
// slowest event, which tells a page turn from a sheet opening. Never a key's value or a place.
static const char *event_name(int t) {
    static const char *const k[] = {"none", "pen down", "pen move", "pen up", "pen hover", "pen leave", "touch down",
                                    "touch move", "touch up", "power", "quit", "pen button", "orientation",
                                    "pinch begin", "pinch move", "pinch end", "sleep", "wake", "power up", "key",
                                    "power held", "controller or keyboard came", "controller or keyboard went",
                                    "key up"};
    _Static_assert(sizeof k / sizeof k[0] == IN_KEY_UP + 1, "an event name for each InType");
    return t >= 0 && t <= IN_KEY_UP ? k[t] : "?";
}
static const char *screen_name(int s) {
    static const char *const k[SCR_COUNT] = {"library", "editor", "pen feel", "calibration", "share"};
    return s >= 0 && s < SCR_COUNT ? k[s] : "?";
}
#endif

// The event batch's slowest event: how long, which, and the screen it came to.
typedef struct {
    uint32_t ms;
    int type, scr;
    unsigned n;
} Slowest;

static void event_timed(App *a, const InEvent *ev, Slowest *slow) {
    uint32_t t0 = now_ms();
    int scr = (int)a->scr;
    app_event(a, ev);
    uint32_t took = now_ms() - t0;
    if (!slow->n++ || took > slow->ms) *slow = (Slowest){took, (int)ev->type, scr, slow->n};
}

#ifdef GRIMOIRE_KINDLE
static void batch_done(uint32_t *since, const Slowest *slow) {
    uint32_t t = now_ms(), took = t - *since;
    if (took > 50)
        qlog("main: the event batch took %u ms (slowest: %s on the %s screen, %u ms, of %u events)", (unsigned)took,
             event_name(slow->type), screen_name(slow->scr), (unsigned)slow->ms, slow->n);
    *since = t;
}
#define BATCH_DONE(since, slow) batch_done(since, slow)
#else
#define PHASE(name, since, wait_ms) ((void)(since))
#define BATCH_DONE(since, slow) ((void)(since), (void)(slow))
#endif

#ifdef GRIMOIRE_SIM
// The fit check (make l10n-check runs every screen with GRIMOIRE_CHECK_FIT=1): each text cut
// short to fit its place is listed once (per text and room), and the run ends with status 3.
static int g_fit_cuts;
static void fit_report(const char *text, int need, int have) {
    static uint32_t seen[512];
    static int nseen;
    uint32_t h = 2166136261u ^ (uint32_t)have;  // FNV-1a of the room and the text
    for (const char *p = text; *p; p++) h = (h ^ (uint8_t)*p) * 16777619u;
    for (int i = 0; i < nseen; i++)
        if (seen[i] == h) return;
    if (nseen < (int)(sizeof seen / sizeof seen[0])) seen[nseen++] = h;
    g_fit_cuts++;
    fprintf(stderr, "fit[%s]: \"%s\" needs %d px, has %d\n", i18n_current(), text, need, have);
}
#endif

static void usage(void) {
    printf("Grimoire %s: a notebook for the Kindle Scribe\n"
           "  env GRIMOIRE_ROOT   data directory (default %s)\n"
           "  --version        print the version\n",
           GRIMOIRE_VERSION,
#ifdef GRIMOIRE_KINDLE
           "/mnt/us/grimoire"
#else
           "./grimoire-data"
#endif
    );
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--version")) {
            printf("%s\n", GRIMOIRE_VERSION);
            return 0;
        }
        if (!strcmp(argv[i], "--help") || !strcmp(argv[i], "-h")) {
            usage();
            return 0;
        }
    }
    const char *root = getenv("GRIMOIRE_ROOT");
#ifdef GRIMOIRE_KINDLE
    if (!root || !*root) root = "/mnt/us/grimoire";
#else
    if (!root || !*root) root = "./grimoire-data";
#endif
    qlog("Grimoire %s starting, data in %s", GRIMOIRE_VERSION, root);
#ifdef GRIMOIRE_KINDLE
    log_device();
#endif
    if (store_init(root) != 0) return 1;
    // The language, before anything is drawn or said: Settings > Language (settings.ini), and
    // for "auto" (the default) the Kindle's own (i18n.h says where that is found).
    i18n_detect();
    char lang[16];
    settings_language(lang, sizeof lang);
    i18n_apply(lang);
#ifdef GRIMOIRE_SIM
    const char *check_fit = getenv("GRIMOIRE_CHECK_FIT");
    if (check_fit && !strcmp(check_fit, "1")) font_set_overflow_hook(fit_report);
#endif

    int w, h;
    float dpi;
    if (display_open(&w, &h, &dpi) != 0) {
        qlog("display_open failed");
        return 1;
    }
    qlog("display %dx%d @ %.0f dpi", w, h, dpi);
    // The front light's levels, once: asking takes 95-191 ms on the Kindle, nothing can read the pens
    // meanwhile, and they are not open yet (light.h).
    light_refresh();
    if (input_open(w, h, NULL, NULL) != 0) {
        qlog("input_open failed");
        display_close();
        return 1;
    }
    static App app;
    if (app_init(&app, w, h, dpi) != 0) {
        qlog("app_init failed");
        input_close();
        display_close();
        return 1;
    }
    app_flush(&app);
    qlog("first frame presented");
    // The Kindle UI may repaint once more after we start (the scriptlet "book" closing, a
    // late home-screen refresh). Check the panel still shows us; if not, repaint everything.
    uint32_t started = now_ms(), last_guard = started;
    int guards = 0, guard_streak = 0, startup_repaints = 0, status = 0;
    bool guard_on = true;
#ifdef GRIMOIRE_KINDLE
    heartbeat();
    uint32_t last_beat = now_ms();
#endif

    while (app.running) {
        // A panel that took none of the first five refreshes is not Grimoire's to drive (a device
        // FBInk does not know, say): leave, so the launcher gives the Kindle back its screen, pen
        // and touch, instead of a device that looks frozen. (app_shutdown below saves.)
        if (display_broken()) {
            qlog("display: the first frame and the next 4 refreshes all failed; leaving so the Kindle UI comes back");
            status = 1;
            break;
        }
        InEvent ev;
        uint32_t phase = now_ms();
        // Batch nearby samples, but keep showing ink during a backlog. A costly brush or
        // a continuously readable device must not postpone refreshes and timers until the
        // queue empties. Leave remaining events queued, in order, for the next iteration.
        if (input_poll(&ev, 30)) {
            PHASE("reading input", &phase, 30);
            uint32_t batch_at = now_ms();
            unsigned count = 0;
            Slowest slow = {0};
            do {
                event_timed(&app, &ev, &slow);
                count++;
            } while (app.running && count < 64 && now_ms() - batch_at < 8 && input_poll(&ev, 0));
            BATCH_DONE(&phase, &slow);
        } else {
            PHASE("reading input", &phase, 30);
        }
        app_tick(&app, now_ms());
        PHASE("app_tick", &phase, 0);
        app_flush(&app);
        PHASE("app_flush", &phase, 0);
        uint32_t t = now_ms();
        uint32_t every = (t - started) < 15000 ? 700 : 1500;
        if (t - last_guard >= every && !app.asleep && guard_on) {
            last_guard = t;
            // The Kindle's status bar strip (clock, battery) lives at the top of the panel: check
            // that band densely, and repaint just it (no flash) when something drew there.
            Rect skip = app.toast_visible ? app.toast_rect : RECT_EMPTY;
            Rect top = {0, 0, app.w, ui_px(150)};
            Rect where;
            int before = guards;
            if (display_find_overwrite(&app.screen, skip, &where)) {
                guards++;
                // Most of the screen: a clean full refresh. A patch: repaint just it, no flash.
                bool most = (int64_t)where.w * where.h > (int64_t)app.w * app.h * 2 / 5;
                qlog("screen guard: repainting %s (#%d)", most ? "everything" : "the patch", guards);
                if (most) app_invalidate_all(&app, REFRESH_FULL);
                else app_invalidate(&app, where, REFRESH_UI);
                app_flush(&app);
            } else if (display_band_overwritten(&app.screen, top, skip)) {
                guards++;
                qlog("screen guard: repainting the top band (#%d)", guards);
                app_invalidate(&app, top, REFRESH_UI);
                app_flush(&app);
            }
            // A repaint puts the panel right, so a mismatch at every check means the framebuffer
            // does not read back as drawn (a panel nobody here has tested): rather than repaint
            // forever, the guard stops.
            guard_streak = guards > before ? guard_streak + 1 : 0;
            if (guard_streak >= 10) {
                guard_on = false;
                qlog("screen guard: off after 10 repaints in a row (the framebuffer does not read back as drawn)");
            }
            PHASE("the screen guard", &phase, 0);
        }
        // Two unconditional repaints shortly after launch catch the Kindle UI's last frame (the
        // home screen strip that could land on top of Grimoire's first frame).
        if (!app.asleep && ((startup_repaints == 0 && t - started > 1200) || (startup_repaints == 1 && t - started > 3500))) {
            startup_repaints++;
            app_invalidate_all(&app, REFRESH_UI);
            app_flush(&app);
        }
#ifdef GRIMOIRE_KINDLE
        if (now_ms() - last_beat > 2000) {
            heartbeat();
            last_beat = now_ms();
        }
#endif
    }
    qlog("exiting");
    app_shutdown(&app);
    display_wait();
    input_close();
    display_close();  // (a headless run whose expect checks failed exits here, with status 3)
#ifdef GRIMOIRE_SIM
    if (g_fit_cuts) {
        fprintf(stderr, "fit: %d text%s cut short to fit\n", g_fit_cuts, g_fit_cuts == 1 ? "" : "s");
        return 3;
    }
#endif
    return status;
}
