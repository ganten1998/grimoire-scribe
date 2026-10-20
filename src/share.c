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
enum { MAX_PARTS = 6 };  // the code or the icon, the text, the counter, Done, Retry

// A part of the screen and where draw() put it, for check_layout.
typedef struct {
    const char *what;
    Rect r;
} Part;

struct ShareState {
    ScreenId ret;
    HitList hits;
    Rect counter;
    int last_count;
    uint32_t last_paint;
    bool started;
    Part parts[MAX_PARTS];
    int nparts;
};

#define S (a->share)

static const char *const k_help[] = {
    "Pages as PNG, notebooks as PDF or zip, folders and everything as zip.",
    "Effects: transparent, recolored or filtered copies of any page.",
    "Same Wi-Fi network only. Keep this screen open while downloading.",
    "From a terminal: tools/grimoire-pull --http <address above>",
};
enum { N_HELP = sizeof k_help / sizeof k_help[0] };

// The screen checks its own layout as it draws, and logs a "share: layout:" line for anything that
// does not fit (tests/share_check.sh fails on any): a line too wide for its column (which
// font_draw_centered would cut short), or a part of the screen outside the card or over another.

// One line of text centered in r. Returns the bottom of r. (On the Kindle the log leaves the
// line's text out: it may be the address, and grimoire.log goes to public bug reports.)
static int draw_line(Canvas *c, FontFace f, float px, Rect r, const char *s, uint8_t gray) {
    int w = font_measure(f, px, s);
#ifdef GRIMOIRE_SIM
    if (w > r.w) qlog("share: layout: '%s' is cut to fit (%d px wide, room for %d)", s, w, r.w);
#else
    if (w > r.w) qlog("share: layout: a line is cut to fit (%d px wide, room for %d)", w, r.w);
#endif
    font_draw_centered(c, f, px, r, s, gray);
    return r.y + r.h;
}

static void placed(App *a, const char *what, Rect r) {
    if (S->nparts < MAX_PARTS) S->parts[S->nparts++] = (Part){what, r};
}

static void check_layout(App *a, Rect in) {
    for (int i = 0; i < S->nparts; i++) {
        const Part *p = &S->parts[i];
        if (!rect_contains_rect(in, p->r))
            qlog("share: layout: %s (%d,%d %dx%d) runs out of the card (%d,%d %dx%d)", p->what, p->r.x, p->r.y,
                 p->r.w, p->r.h, in.x, in.y, in.w, in.h);
        for (int j = i + 1; j < S->nparts; j++) {
            const Part *q = &S->parts[j];
            if (rect_overlaps(p->r, q->r))
                qlog("share: layout: %s (%d,%d %dx%d) runs into %s (%d,%d %dx%d)", p->what, p->r.x, p->r.y, p->r.w,
                     p->r.h, q->what, q->r.x, q->r.y, q->r.w, q->r.h);
        }
    }
}

static void draw_counter(App *a) {
    char text[96];
    snprintf(text, sizeof text, "Downloads served: %d", httpd_requests_served());
    canvas_fill(&a->screen, S->counter, 255);
    draw_line(&a->screen, FONT_REGULAR, ui_text_size(), S->counter, text, 0);
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

// The text block from y down, centered in the column [x, x + w): the caption, url (the first
// address, the very string in the code) in big type, a line per other address, the help. Returns
// the bottom of its last line.
static int draw_text(App *a, int x, int w, int y, const char *url, const NetAddr *ad, int n, bool qr) {
    Canvas *c = &a->screen;
    char other[64], text[128];
    draw_line(c, FONT_REGULAR, ui_text_size(), (Rect){x, y, w, ui_px(80)},
              qr ? "Scan with your phone, or open this in a browser:"
                 : "On your computer, open this in a browser:",
              0);
    y += ui_px(110);
    draw_line(c, FONT_BOLD, ui_title_size() * 1.5f, (Rect){x, y, w, ui_px(140)}, url, 0);
    y += ui_px(150);
    for (int i = 1; i < n; i++) {
        url_of(a, &ad[i], other, sizeof other);
        snprintf(text, sizeof text, "Also: %s (%s)", other, ad[i].ifname);
        draw_line(c, FONT_REGULAR, ui_small_size(), (Rect){x, y, w, ui_px(64)}, text, 0);
        y += ui_px(70);
    }
    y += ui_px(30);
    int end = y;
    for (int i = 0; i < N_HELP; i++) {
        end = draw_line(c, FONT_REGULAR, ui_small_size(), (Rect){x, y, w, ui_px(64)}, k_help[i], 60);
        y += ui_px(70);
    }
    return end;
}

// The QR code of the first address when a phone can reach it (the Wi-Fi icon when none can, say
// over USBNetwork only), then the text. Portrait stacks them above the counter; landscape puts
// the code on the left and the text, counter and buttons on the right.
static void draw_addresses(App *a, Rect in, const NetAddr *ad, int n) {
    Canvas *c = &a->screen;
    char url[64];
    url_of(a, &ad[0], url, sizeof url);
    int qn = phone_reachable(ad[0].ifname) ? ui_qr_modules(url) : 0;  // modules across, 0: no code
    // grimoire.log goes to public bug reports (kindle/README.txt asks for it): it names the
    // interface and the port, never the address. Only the simulator logs the address too, for
    // tests/share_check.sh, which reads the code back from a screenshot and compares.
    if (qn) qlog("share: QR code for the %s address, port %d", ad[0].ifname, a->settings.http_port);
    else qlog("share: no Wi-Fi address, so no QR code; showing the %s address, port %d", ad[0].ifname,
              a->settings.http_port);
#ifdef GRIMOIRE_SIM
    qlog("share: (simulator) %s %s (%s)", qn ? "the code holds" : "the address shown is", url, ad[0].ifname);
#endif
    // The quiet zone is part of the code's box: with Invert on it shows as a light square, so the
    // box keeps clear of the title, the frame and the caption.
    int top = in.y + ui_px(20), gap = ui_px(30), text_h = text_height(n), bottom = S->counter.y - ui_px(30);
    bool negative = a->settings.invert;  // inverted on the panel: dark modules on light
    if (a->w <= a->h) {
        int y = in.y + ui_px(360);
        if (qn) {
            int mod = clampi((bottom - top - gap - text_h) / qn, ui_px(10), ui_px(16));
            Rect sym = ui_qr(c, (Rect){in.x, top, in.w, qn * mod}, url, negative);
            placed(a, "the code", sym);
            if (!rect_is_empty(sym)) y = sym.y + sym.h + gap;
        } else {
            Rect icon = {in.x + in.w / 2 - ui_px(130), in.y + ui_px(40), ui_px(260), ui_px(260)};
            ui_icon(c, ICON_SHARE, icon, 0);
            placed(a, "the icon", icon);
        }
        int end = draw_text(a, in.x, in.w, y, url, ad, n, qn > 0);
        placed(a, "the text", (Rect){in.x, y, in.w, end - y});
        return;
    }
    int lx = in.x + ui_px(30), mod = qn ? clampi((bottom - top) / qn, ui_px(10), ui_px(16)) : 0;
    int left = qn ? qn * mod : ui_px(260), col = lx + left + ui_px(60), col_w = in.x + in.w - col;
    if (qn) {
        placed(a, "the code", ui_qr(c, (Rect){lx, top, left, left}, url, negative));
    } else {
        Rect icon = {lx, in.y + ui_px(40), left, left};
        ui_icon(c, ICON_SHARE, icon, 0);
        placed(a, "the icon", icon);
    }
    S->counter = (Rect){col, S->counter.y, col_w, S->counter.h};
    int end = draw_text(a, col, col_w, top, url, ad, n, qn > 0);
    placed(a, "the text", (Rect){col, top, col_w, end - top});
}

static void draw(App *a) {
    Canvas *c = &a->screen;
    hits_clear(&S->hits);
    S->nparts = 0;
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
        Rect icon = {in.x + in.w / 2 - ui_px(130), y, ui_px(260), ui_px(260)};
        ui_icon(c, ICON_SHARE, icon, 0);
        placed(a, "the icon", icon);
        y += ui_px(320);
        int y0 = y;
        draw_line(c, FONT_BOLD, ui_title_size(), (Rect){in.x, y, in.w, ui_px(100)},
                  S->started ? "Turn on Wi-Fi on the Kindle first" : "Could not start the server", 0);
        y += ui_px(110);
        int end = draw_line(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y, in.w, ui_px(80)},
                            S->started ? "Then tap Retry." : "Is another app using the port? Try again.", 60);
        placed(a, "the text", (Rect){in.x, y0, in.w, end - y0});
    }
    if (S->started) {
        draw_counter(a);
        placed(a, "the counter", S->counter);
    }
    int bw = ui_px(420), bh = ui_px(150);
    Rect done = {in.x + in.w - bw, in.y + in.h - bh, bw, bh};
    ui_button(c, done, ICON_CHECK, "Done", BTN_PRIMARY);
    hits_add(&S->hits, done, H_DONE, 0);
    placed(a, "Done", done);
    if (n == 0) {
        Rect retry = {in.x, in.y + in.h - bh, bw, bh};
        ui_button(c, retry, ICON_NONE, "Retry", BTN_PLAIN);
        hits_add(&S->hits, retry, H_RETRY, 0);
        placed(a, "Retry", retry);
    }
    check_layout(a, in);
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
    if (a->relayout && httpd_running()) qlog("share: laid out again (turned or woken); the server carries on");
    start(a);  // a server kept across a relayout goes on as it is
    draw(a);
}

// Asleep on this screen nothing accepts a connection (app_tick runs no screen's tick while
// asleep), so one waiting at the server would wake input_poll at once, again and again, and the
// main loop would spin at full speed until the wake. It waits in the server's queue instead, and
// the wake's relayout watches the server again (start, from share_enter).
void share_sleep(App *a) {
    (void)a;
    input_watch_fd(-1);
}

// The sleep screen while this screen is up and no page is shown in its place: the card without
// the code and the addresses, since nothing answers while the Kindle sleeps (share_sleep), and a
// sleeping Kindle need not show its address. The wake lays the screen out again (share_enter).
void share_draw_asleep(App *a) {
    Canvas *c = &a->screen;
    canvas_fill(c, (Rect){0, 0, a->w, a->h}, 255);
    int m = ui_px(70);
    Rect card = {m, ui_px(260), a->w - 2 * m, a->h - ui_px(520)};
    Rect in = ui_sheet(c, card, "Share over Wi-Fi");
    ui_icon(c, ICON_SHARE, (Rect){in.x + in.w / 2 - ui_px(130), in.y + ui_px(40), ui_px(260), ui_px(260)}, 0);
    int y = in.y + ui_px(360);
    draw_line(c, FONT_BOLD, ui_title_size(), (Rect){in.x, y, in.w, ui_px(100)}, "Asleep", 0);
    draw_line(c, FONT_REGULAR, ui_text_size(), (Rect){in.x, y + ui_px(110), in.w, ui_px(80)},
              "Wake the Kindle to share again.", 60);
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
