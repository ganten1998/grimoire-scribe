// share.c: "Share over Wi-Fi". Runs the HTTP server only while this screen is open.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "httpd.h"

enum { H_DONE = 1, H_RETRY };

struct ShareState {
    ScreenId ret;
    HitList hits;
    Rect counter;
    int last_count;
    uint32_t last_paint;
    bool started;
};

#define S (a->share)

static void draw_counter(App *a) {
    char line[96];
    snprintf(line, sizeof line, "Downloads served: %d", httpd_requests_served());
    canvas_fill(&a->screen, S->counter, 255);
    font_draw_centered(&a->screen, FONT_REGULAR, ui_text_size(), S->counter, line, 0);
}

static void draw(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(70);
    Rect card = {m, ui_px(260), a->w - 2 * m, a->h - ui_px(520)};
    Rect in = ui_sheet(c, card, "Share over Wi-Fi");
    int y = in.y + ui_px(40);
    ui_icon(c, ICON_SHARE, (Rect){in.x + in.w / 2 - ui_px(130), y, ui_px(260), ui_px(260)}, 0);
    y += ui_px(320);
    char ips[8][16];
    int n = S->started ? net_ipv4_addrs(ips, 8) : 0;
    if (!S->started) {
        font_draw_centered(c, FONT_BOLD, ui_title_size(), (Rect){in.x, y, in.w, ui_px(100)},
                           "Could not start the server", 0);
        y += ui_px(110);
        font_draw_centered(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)},
                           "Is another app using the port? Try again.", 60);
    } else if (n == 0) {
        font_draw_centered(c, FONT_BOLD, ui_title_size(), (Rect){in.x, y, in.w, ui_px(100)},
                           "Turn on Wi-Fi on the Kindle first", 0);
        y += ui_px(110);
        font_draw_centered(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)},
                           "Then tap Retry.", 60);
    } else {
        font_draw_centered(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)},
                           "On your computer, open this in a browser:", 0);
        y += ui_px(110);
        for (int i = 0; i < n && i < 3; i++) {
            char url[64];
            snprintf(url, sizeof url, "http://%s:%d", ips[i], a->settings.http_port);
            font_draw_centered(c, FONT_BOLD, ui_title_size() * 1.5f, (Rect){in.x, y, in.w, ui_px(140)}, url, 0);
            y += ui_px(150);
        }
        y += ui_px(30);
        const char *lines[] = {
            "Pages as PNG, notebooks as PDF or zip, folders and everything as zip.",
            "Effects: transparent, recolored or filtered copies of any page.",
            "Same Wi-Fi network only. Keep this screen open while downloading.",
            "From a terminal: tools/grimoire-pull --http <address above>",
        };
        for (int i = 0; i < 4; i++) {
            font_draw_centered(c, FONT_REGULAR, ui_small_size(), (Rect){in.x, y, in.w, ui_px(64)}, lines[i], 60);
            y += ui_px(70);
        }
    }
    S->counter = (Rect){in.x, in.y + in.h - ui_px(330), in.w, ui_px(80)};
    if (S->started) draw_counter(a);
    int bw = ui_px(420), bh = ui_px(150);
    Rect done = {in.x + in.w - bw, in.y + in.h - bh, bw, bh};
    ui_button(c, done, ICON_CHECK, "Done", BTN_PRIMARY);
    hits_add(&S->hits, done, H_DONE, 0);
    if (!S->started || n == 0) {
        Rect retry = {in.x, in.y + in.h - bh, bw, bh};
        ui_button(c, retry, ICON_NONE, "Retry", BTN_PLAIN);
        hits_add(&S->hits, retry, H_RETRY, 0);
    }
}

static void start(App *a) {
    if (!httpd_running()) S->started = httpd_start(a->settings.http_port, a->dpi, ui_theme() == THEME_GOTHIC) == 0;
    else S->started = true;
    input_watch_fd(S->started ? httpd_fd() : -1);
    S->last_count = httpd_requests_served();
}

void share_open(App *a) {
    S->ret = a->scr;
    app_switch(a, SCR_SHARE);
}

void share_init(App *a) { a->share = calloc(1, sizeof *a->share); }
void share_free(App *a) {
    httpd_stop();
    free(a->share);
    a->share = NULL;
}

void share_enter(App *a) {
    start(a);
    draw(a);
}

void share_leave(App *a) {
    input_watch_fd(-1);
    httpd_stop();
}

void share_event(App *a, const InEvent *ev) {
    httpd_poll();
    if (app_pointer_phase(ev) != 2) return;
    const Hit *h = hits_find(&S->hits, ev->x, ev->y);
    if (!h) return;
    if (h->id == H_DONE) app_switch(a, S->ret == SCR_SHARE ? SCR_LIBRARY : S->ret);
    else if (h->id == H_RETRY) {
        start(a);
        draw(a);
        app_invalidate_all(a, REFRESH_UI);
    }
}

void share_tick(App *a, uint32_t now) {
    httpd_poll();
    int n = httpd_requests_served();
    if (n != S->last_count && now - S->last_paint > 2000) {
        S->last_count = n;
        S->last_paint = now;
        draw_counter(a);
        app_invalidate(a, S->counter, REFRESH_UI);
    }
}
