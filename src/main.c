// main.c: entry point for the Kindle build and the desktop simulator.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"

#ifdef QUIRE_KINDLE
#include <fcntl.h>
#include <unistd.h>
// The launcher's watchdog kills Quire and restores the Kindle UI if this file goes stale,
// so a hang can never leave the device stuck with its UI paused.
#define HEARTBEAT "/tmp/quire.alive"
static void heartbeat(void) {
    int fd = open(HEARTBEAT, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) {
        if (write(fd, "1", 1) < 0) { /* best effort */ }
        close(fd);
    }
}
#endif

static void usage(void) {
    printf("Quire %s: a notebook for the Kindle Scribe\n"
           "  env QUIRE_ROOT   data directory (default %s)\n"
           "  --version        print the version\n",
           QUIRE_VERSION,
#ifdef QUIRE_KINDLE
           "/mnt/us/quire"
#else
           "./quire-data"
#endif
    );
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--version")) {
            printf("%s\n", QUIRE_VERSION);
            return 0;
        }
        if (!strcmp(argv[i], "--help") || !strcmp(argv[i], "-h")) {
            usage();
            return 0;
        }
    }
    const char *root = getenv("QUIRE_ROOT");
#ifdef QUIRE_KINDLE
    if (!root || !*root) root = "/mnt/us/quire";
#else
    if (!root || !*root) root = "./quire-data";
#endif
    qlog("Quire %s starting, data in %s", QUIRE_VERSION, root);
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
    int guards = 0, startup_repaints = 0;
#ifdef QUIRE_KINDLE
    heartbeat();
    uint32_t last_beat = now_ms();
#endif

    while (app.running) {
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
        if (t - last_guard >= every && !app.asleep) {
            last_guard = t;
            // The Kindle's status bar strip (clock, battery) lives at the top of the panel: check
            // that band densely, and repaint just it (no flash) when something drew there.
            Rect skip = app.toast_visible ? app.toast_rect : RECT_EMPTY;
            Rect top = {0, 0, app.w, ui_px(150)};
            if (display_overwritten(&app.screen)) {
                guards++;
                qlog("screen guard: repainting (#%d)", guards);
                app_invalidate_all(&app, REFRESH_FULL);
                app_flush(&app);
            } else if (display_band_overwritten(&app.screen, top, skip)) {
                guards++;
                qlog("screen guard: repainting the top band (#%d)", guards);
                app_invalidate(&app, top, REFRESH_UI);
                app_flush(&app);
            }
        }
        // Two unconditional repaints shortly after launch catch the Kindle UI's last frame (the
        // home screen strip that could land on top of Quire's first frame).
        if (!app.asleep && ((startup_repaints == 0 && t - started > 1200) || (startup_repaints == 1 && t - started > 3500))) {
            startup_repaints++;
            app_invalidate_all(&app, REFRESH_UI);
            app_flush(&app);
        }
#ifdef QUIRE_KINDLE
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
    return 0;
}
