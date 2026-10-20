// share.c: "Share over Wi-Fi". Runs the HTTP server only while this screen is open; turning the
// Kindle or waking it only lays the screen out again, and the server (with any download) carries
// on. A phone scans the QR code of the Wi-Fi address; a computer types the address under it.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"
#include "httpd.h"
#include "qr.h"

enum { H_DONE = 1, H_RETRY };
enum { MAX_SHOWN = 3 };  // addresses listed: the first in big type, each other one on a small line

struct ShareState {
    ScreenId ret;
    HitList hits;
    Rect counter;
    int last_count;
    uint32_t last_paint;
    bool started;
};

#define S (a->share)

static const char *const k_help[] = {
    "Pages as PNG, notebooks as PDF or zip, folders and everything as zip.",
    "Effects: transparent, recolored or filtered copies of any page.",
    "Same Wi-Fi network only. Keep this screen open while downloading.",
    "From a terminal: tools/grimoire-pull --http <address above>",
};
enum { N_HELP = sizeof k_help / sizeof k_help[0] };

static void draw_counter(App *a) {
    char line[96];
    snprintf(line, sizeof line, "Downloads served: %d", httpd_requests_served());
    canvas_fill(&a->screen, S->counter, 255);
    font_draw_centered(&a->screen, FONT_REGULAR, ui_text_size(), S->counter, line, 0);
}

// Whether a phone can reach this address: the launcher opens the share port on wlan0 only. In the
// simulator the computer's own Wi-Fi or Ethernet (en*) stands in for it.
static bool phone_reachable(const char *ifname) {
#ifdef GRIMOIRE_SIM
    if (!strncmp(ifname, "en", 2)) return true;
#endif
    return !strncmp(ifname, "wlan", 4);
}

static void url_of(const App *a, const NetAddr *ad, char *out, size_t cap) {
    snprintf(out, cap, "http://%s:%d", ad->ip, a->settings.http_port);
}

// The text block for n addresses: caption, the first address, a line per other one, the help.
static int text_height(int n) {
    return ui_px(110) + ui_px(150) + (n - 1) * ui_px(70) + ui_px(30) + N_HELP * ui_px(70);
}

// The text block from y down, centered in the column [x, x + w).
static void draw_text(App *a, int x, int w, int y, const NetAddr *ad, int n, bool qr) {
    Canvas *c = &a->screen;
    char url[64], line[128];
    font_draw_centered(c, FONT_REGULAR, ui_text_size(), (Rect){x, y, w, ui_px(80)},
                       qr ? "Scan with your phone, or open this in a browser:"
                          : "On your computer, open this in a browser:",
                       0);
    y += ui_px(110);
    url_of(a, &ad[0], url, sizeof url);
    font_draw_centered(c, FONT_BOLD, ui_title_size() * 1.5f, (Rect){x, y, w, ui_px(140)}, url, 0);
    y += ui_px(150);
    for (int i = 1; i < n; i++) {
        url_of(a, &ad[i], url, sizeof url);
        snprintf(line, sizeof line, "Also: %s (%s)", url, ad[i].ifname);
        font_draw_centered(c, FONT_REGULAR, ui_small_size(), (Rect){x, y, w, ui_px(64)}, line, 0);
        y += ui_px(70);
    }
    y += ui_px(30);
    for (int i = 0; i < N_HELP; i++) {
        font_draw_centered(c, FONT_REGULAR, ui_small_size(), (Rect){x, y, w, ui_px(64)}, k_help[i], 60);
        y += ui_px(70);
    }
}

// The QR code of the first address when a phone can reach it (the Wi-Fi icon when none can, say
// over USBNetwork only), then the text. Portrait stacks them above the counter; landscape puts
// the code on the left and the text, counter and buttons on the right.
static void draw_addresses(App *a, Rect in, const NetAddr *ad, int n) {
    Canvas *c = &a->screen;
    char url[64];
    url_of(a, &ad[0], url, sizeof url);
    int qn = phone_reachable(ad[0].ifname) ? ui_qr_modules(url) : 0;  // modules across, 0: no code
    if (qn) qlog("share: QR code for %s (%s)", url, ad[0].ifname);
    else qlog("share: no Wi-Fi address, so no QR code; showing %s (%s)", url, ad[0].ifname);
    // The quiet zone is part of the code's box: with Invert on it shows as a light square, so the
    // box keeps clear of the title, the frame and the caption.
    int top = in.y + ui_px(20), gap = ui_px(30), text_h = text_height(n), bottom = S->counter.y - ui_px(30);
    bool negative = a->settings.invert;  // inverted on the panel: dark modules on light
    if (a->w <= a->h) {
        int y = in.y + ui_px(360);
        if (qn) {
            int mod = clampi((bottom - top - gap - text_h) / qn, ui_px(10), ui_px(16));
            Rect sym = ui_qr(c, (Rect){in.x, top, in.w, qn * mod}, url, negative);
            if (!rect_is_empty(sym)) y = sym.y + sym.h + gap;
        } else {
            ui_icon(c, ICON_SHARE, (Rect){in.x + in.w / 2 - ui_px(130), in.y + ui_px(40), ui_px(260), ui_px(260)}, 0);
        }
        draw_text(a, in.x, in.w, y, ad, n, qn > 0);
        return;
    }
    int lx = in.x + ui_px(30), mod = qn ? clampi((bottom - top) / qn, ui_px(10), ui_px(16)) : 0;
    int left = qn ? qn * mod : ui_px(260), col = lx + left + ui_px(60), col_w = in.x + in.w - col;
    if (qn) ui_qr(c, (Rect){lx, top, left, left}, url, negative);
    else ui_icon(c, ICON_SHARE, (Rect){lx, in.y + ui_px(40), left, left}, 0);
    S->counter = (Rect){col, S->counter.y, col_w, S->counter.h};
    draw_text(a, col, col_w, top, ad, n, qn > 0);
}

static void draw(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->hits);
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(70);
    Rect card = {m, ui_px(260), a->w - 2 * m, a->h - ui_px(520)};
    Rect in = ui_sheet(c, card, "Share over Wi-Fi");
    S->counter = (Rect){in.x, in.y + in.h - ui_px(330), in.w, ui_px(80)};
    NetAddr ad[MAX_SHOWN];
    int n = S->started ? net_ipv4_list(ad, MAX_SHOWN) : 0;
    if (n > 0) {
        draw_addresses(a, in, ad, n);
    } else {
        int y = in.y + ui_px(40);
        ui_icon(c, ICON_SHARE, (Rect){in.x + in.w / 2 - ui_px(130), y, ui_px(260), ui_px(260)}, 0);
        y += ui_px(320);
        font_draw_centered(c, FONT_BOLD, ui_title_size(), (Rect){in.x, y, in.w, ui_px(100)},
                           S->started ? "Turn on Wi-Fi on the Kindle first" : "Could not start the server", 0);
        y += ui_px(110);
        font_draw_centered(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)},
                           S->started ? "Then tap Retry." : "Is another app using the port? Try again.", 60);
    }
    if (S->started) draw_counter(a);
    int bw = ui_px(420), bh = ui_px(150);
    Rect done = {in.x + in.w - bw, in.y + in.h - bh, bw, bh};
    ui_button(c, done, ICON_CHECK, "Done", BTN_PRIMARY);
    hits_add(&S->hits, done, H_DONE, 0);
    if (n == 0) {
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
    start(a);  // a server kept across a relayout goes on as it is
    draw(a);
}

void share_leave(App *a) {
    // Turning the Kindle and waking it lay this screen out again: keep the server (and its
    // downloads, connections and queue) for the enter that follows.
    if (a->relayout) return;
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
