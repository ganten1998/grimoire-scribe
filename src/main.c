// main.c: entry point for the Kindle build and the desktop simulator.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"

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

    int w, h;
    float dpi;
    if (display_open(&w, &h, &dpi) != 0) {
        qlog("display_open failed");
        return 1;
    }
    qlog("display %dx%d @ %.0f dpi", w, h, dpi);
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
        // Block briefly (timers and the share server need regular ticks), then drain
        // everything queued so a burst of pen samples becomes one refresh.
        if (input_poll(&ev, 30)) {
            app_event(&app, &ev);
            while (app.running && input_poll(&ev, 0)) app_event(&app, &ev);
        }
        app_tick(&app, now_ms());
        app_flush(&app);
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
    display_close();
    return status;
}
