// httpd.c: a tiny HTTP/1.0 server for the share screen. Pages, fonts and the notebook list are
// answered in-process; anything that renders (PDF, zip, PNG) is answered by a forked child, so
// a big export can neither freeze the UI nor trip the launcher's watchdog. The pages wear the
// Kindle's style (its fonts are served from the copies built into the binary), and the studio
// page recolors pages in the browser from the paper and ink layers (web/studio.js).
#include "httpd.h"
#include "bat.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <ifaddrs.h>
#include <net/if.h>
#include <netinet/in.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <stdatomic.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <unistd.h>

#include "export.h"
#include "i18n.h"
#include "live.h"
#include "pair.h"

#ifndef MSG_NOSIGNAL
#define MSG_NOSIGNAL 0
#endif

// Embedded at build time (build/gen/fonts.c and build/gen/web.c, xxd -i).
extern unsigned char TerminessNerdFontMono_Regular_ttf[];
extern unsigned int TerminessNerdFontMono_Regular_ttf_len;
extern unsigned char TerminessNerdFontMono_Bold_ttf[];
extern unsigned int TerminessNerdFontMono_Bold_ttf_len;
extern unsigned char Jacquard24_Regular_ttf[];
extern unsigned int Jacquard24_Regular_ttf_len;
extern unsigned char Jersey10_Regular_ttf[];
extern unsigned int Jersey10_Regular_ttf_len;
extern unsigned char studio_js[];
extern unsigned int studio_js_len;
extern unsigned char studio_css[];
extern unsigned int studio_css_len;
extern unsigned char download_js[];
extern unsigned int download_js_len;
extern unsigned char live_js[];
extern unsigned int live_js_len;
extern unsigned char live_css[];
extern unsigned int live_css_len;

// Small, anonymous shared records let the parent answer progress while a child renders.
// No notebook contents, temporary files, or second download are needed. Retain the last 32
// records; queued/running jobs cannot be recycled, including a child not yet reaped.
enum { DL_EMPTY, DL_QUEUED, DL_PREPARING, DL_SENDING, DL_DONE, DL_FAILED };
typedef struct {
    char id[33];
    atomic_uint state, pages, total, sent, bytes;
    bool busy; // parent only: the child still owns this record
} Download;
_Static_assert(ATOMIC_INT_LOCK_FREE == 2, "download counters must work across fork");
static Download *g_downloads;
static unsigned g_download_next;
static bool g_child_response, g_child_head;

static bool download_id_ok(const char *id) {
    if (strlen(id) != 32) return false;
    for (int i = 0; i < 32; i++) if (!isxdigit((unsigned char)id[i])) return false;
    return true;
}

static Download *download_find(const char *id) {
    if (g_downloads && download_id_ok(id))
        for (int i = 0; i < 32; i++) if (!strcmp(id, g_downloads[i].id)) return &g_downloads[i];
    return NULL;
}

static Download *download_begin(const char *id) {
    if (!download_id_ok(id) || download_find(id)) return NULL;
    if (!g_downloads) {
        void *p = mmap(NULL, 32 * sizeof(Download), PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANON, -1, 0);
        if (p == MAP_FAILED) return NULL;
        g_downloads = p;
        for (int i = 0; i < 32; i++) {
            atomic_init(&g_downloads[i].state, DL_EMPTY);
            atomic_init(&g_downloads[i].pages, 0); atomic_init(&g_downloads[i].total, 0);
            atomic_init(&g_downloads[i].sent, 0); atomic_init(&g_downloads[i].bytes, 0);
        }
    }
    for (int i = 0; i < 32; i++) {
        Download *d = &g_downloads[g_download_next++ % 32];
        unsigned state = atomic_load(&d->state);
        if (d->busy || (state != DL_EMPTY && state != DL_DONE && state != DL_FAILED)) continue;
        strlcpy_q(d->id, id, sizeof d->id);
        atomic_store(&d->pages, 0); atomic_store(&d->total, 0);
        atomic_store(&d->sent, 0); atomic_store(&d->bytes, 0);
        atomic_store(&d->state, DL_QUEUED);
        return d;
    }
    return NULL;
}

// One connection: it reads its request, then writes its answer, waits for a render slot, or is
// handed to a forked child. Each has its own buffer, so a new connection is never answered from
// an older one's request, and a slow or idle one (browsers open spare connections) holds up
// nobody.
typedef enum { C_FREE, C_READING, C_WRITING, C_QUEUED } ClientState;
typedef struct {
    ClientState st;
    int fd;
    char req[4096];
    size_t reqlen;
    Buf out;
    size_t sent;
    uint32_t t0;   // last progress; for C_QUEUED, when it began waiting
    uint32_t seq;  // arrival order: queued renders start oldest first
    Download *download;
} Client;

static int g_listen = -1;
static float g_dpi = 300.f;
static bool g_gothic = true;
static int g_served;
static unsigned g_owners;    // HTTPD_SHARE, HTTPD_LIVE: who keeps the server up
static int g_live_pairings;
static bool g_head;          // the request being answered is a HEAD
#define MAX_CLIENTS 16
static Client g_cl[MAX_CLIENTS];
static Client *g_cur;  // the connection a route is answering
static uint32_t g_seq;

// Renders run in forked children, two at a time; more wait their turn (C_QUEUED). A page of
// previews asks for many at once.
#define MAX_CHILDREN 2
static pid_t g_children[MAX_CHILDREN];
static Download *g_child_download[MAX_CHILDREN];

static void reap_children(void) {
    for (int i = 0; i < MAX_CHILDREN; i++)
        if (g_children[i] > 0 && waitpid(g_children[i], NULL, WNOHANG) > 0) {
            Download *d = g_child_download[i];
            if (d) {
                if (atomic_load(&d->state) != DL_DONE) atomic_store(&d->state, DL_FAILED);
                d->busy = false;
            }
            g_children[i] = 0;
            g_child_download[i] = NULL;
        }
}

static void set_nonblock(int fd) { fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) | O_NONBLOCK); }

static int httpd_start(int port, float dpi, bool gothic) {
    httpd_stop();
    signal(SIGPIPE, SIG_IGN);
    g_dpi = dpi;
    g_gothic = gothic;
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    int one = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
    struct sockaddr_in sa = {0};
    sa.sin_family = AF_INET;
    sa.sin_port = htons((uint16_t)port);
    sa.sin_addr.s_addr = htonl(INADDR_ANY);
    if (bind(fd, (struct sockaddr *)&sa, sizeof sa) != 0 || listen(fd, 64) != 0) {
        qlog("httpd: bind/listen on %d failed: %s", port, strerror(errno));
        close(fd);
        return -1;
    }
    set_nonblock(fd);
    g_listen = fd;
    qlog("httpd: listening on port %d", port);
    return 0;
}

int httpd_acquire(unsigned owner, int port, float dpi, bool gothic) {
    if (g_listen < 0 && httpd_start(port, dpi, gothic) != 0) return -1;
    g_owners |= owner;
    return 0;
}

void httpd_release(unsigned owner) {
    g_owners &= ~owner;
    if (!g_owners) httpd_stop();
}

unsigned httpd_owners(void) { return g_owners; }
int httpd_live_pairings(void) { return g_live_pairings; }

bool httpd_running(void) { return g_listen >= 0; }
int httpd_fd(void) { return g_listen; }
int httpd_requests_served(void) { return g_served; }

static void client_close(Client *c) {
    if (c->download && atomic_load(&c->download->state) == DL_QUEUED)
        atomic_store(&c->download->state, DL_FAILED);
    c->download = NULL;
    if (c->st != C_FREE && c->fd >= 0) close(c->fd);
    c->fd = -1;
    c->st = C_FREE;
    c->reqlen = 0;
    c->req[0] = 0;
    c->sent = 0;
    buf_free(&c->out);
}

void httpd_stop(void) {
    for (int i = 0; i < MAX_CLIENTS; i++) client_close(&g_cl[i]);
    if (g_listen >= 0) close(g_listen);
    g_listen = -1;
    g_owners = 0;
}

// ---- helpers ----

static void url_decode(char *s);

// Text as HTML, safe in an element and in an attribute in either quotes: names, and every
// translation (a catalog is third-party input; none may add markup).
static void html_esc_n(Buf *b, const char *s, size_t n) {
    for (const char *end = s + n; s < end && *s; s++) {
        switch (*s) {
        case '<': buf_printf(b, "&lt;"); break;
        case '>': buf_printf(b, "&gt;"); break;
        case '&': buf_printf(b, "&amp;"); break;
        case '"': buf_printf(b, "&quot;"); break;
        case '\'': buf_printf(b, "&#39;"); break;
        default: buf_append(b, s, 1);
        }
    }
}

static void html_esc(Buf *b, const char *s) { html_esc_n(b, s, strlen(s)); }

// Whether the UTF-8 character at s is a letter past ASCII: U+00C0 to U+1FFF (Latin with its
// accents, Greek, Cyrillic and the rest of the alphabets), but for the multiplication and division
// signs. Signs below (inverted marks, quotes) and above (punctuation, arrows) are not.
static bool accented_letter(const unsigned char *s) {
    if (s[0] == 0xC3) return s[1] != 0x97 && s[1] != 0xB7;
    return s[0] >= 0xC4 && s[0] <= 0xE1;
}

// Text in Gothic's bold (.big): the bold is a shadow one design pixel to the right, which fills
// the gaps inside an accent (the dots of an u-umlaut run into a bar, the tilde of an n-tilde turns
// into a lump), as the Kindle's overprint did before font.c spared the marks. So the letters past
// ASCII go without it, in spans of class na (studio.js does the same): legible, a little lighter
// than the rest.
static void html_esc_bold(Buf *b, const char *s) {
    if (!g_gothic) return html_esc(b, s);
    while (*s) {
        bool letters = accented_letter((const unsigned char *)s);
        size_t n = 0;
        while (s[n] && accented_letter((const unsigned char *)s + n) == letters) {
            n++;
            while (((unsigned char)s[n] & 0xC0) == 0x80) n++;  // the rest of the character
        }
        if (letters) buf_printf(b, "<span class=na>");
        html_esc_n(b, s, n);
        if (letters) buf_printf(b, "</span>");
        s += n;
    }
}

static void url_esc(Buf *b, const char *s) {
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        if ((*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') || (*p >= '0' && *p <= '9') || strchr("-_.~", *p))
            buf_append(b, p, 1);
        else
            buf_printf(b, "%%%02X", *p);
    }
}

// Also safe inside an HTML <script>: < > & are escaped, so a name cannot close the script.
static void json_str(Buf *b, const char *s) {
    buf_append(b, "\"", 1);
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        if (*p == '"' || *p == '\\') buf_printf(b, "\\%c", *p);
        else if (*p < 0x20 || *p == '<' || *p == '>' || *p == '&') buf_printf(b, "\\u%04x", *p);
        else buf_append(b, p, 1);
    }
    buf_append(b, "\"", 1);
}

static void buf_puts(Buf *b, const char *s) { buf_append(b, s, strlen(s)); }

// Value of `key` in a query string ("a=1&layer=ink"), decoded; false if absent.
static bool query_get(const char *query, const char *key, char *out, size_t cap) {
    size_t kl = strlen(key);
    for (const char *p = query; p && *p; p = strchr(p, '&') ? strchr(p, '&') + 1 : NULL) {
        if (strncmp(p, key, kl) || p[kl] != '=') continue;
        size_t n = strcspn(p + kl + 1, "&");
        if (n >= cap) n = cap - 1;
        memcpy(out, p + kl + 1, n);
        out[n] = 0;
        url_decode(out);
        return true;
    }
    return false;
}

static void url_decode(char *s) {
    char *o = s;
    for (char *p = s; *p; p++) {
        if (*p == '%' && p[1] && p[2]) {
            char hx[3] = {p[1], p[2], 0};
            *o++ = (char)strtol(hx, NULL, 16);
            p += 2;
        } else if (*p == '+')
            *o++ = ' ';
        else
            *o++ = *p;
    }
    *o = 0;
}

// The answer to g_cur. cache: a Cache-Control value, NULL for no-store (everything but the
// fonts); extra: more header lines, each ending in \r\n, or NULL.
static bool child_send(const void *data, size_t len, Download *d) {
    const uint8_t *p = data;
    size_t sent = 0;
    while (sent < len) {
        size_t n = len - sent;
        if (n > 65536) n = 65536;
        ssize_t w = send(g_cur->fd, p + sent, n, MSG_NOSIGNAL);
        if (w < 0 && errno == EINTR) continue;
        if (w <= 0) return false;
        sent += (size_t)w;
        if (d) atomic_store(&d->sent, (unsigned)sent);
    }
    return true;
}

static void download_pages(int done, int total, void *ctx) {
    Download *d = ctx;
    if (!d) return;
    atomic_store(&d->total, (unsigned)total);
    atomic_store(&d->pages, (unsigned)done);
}

static void respond_bytes(int code, const char *ctype, const void *data, size_t len, const char *download_name,
                          const char *cache, const char *extra) {
    const char *reason = code == 200 ? "OK" : code == 404 ? "Not Found" : code == 405 ? "Method Not Allowed"
                                          : code == 400 ? "Bad Request" : code == 503 ? "Service Unavailable"
                                          : code == 302 ? "Found" : code == 401 ? "Unauthorized"
                                          : code == 403 ? "Forbidden" : "Internal Server Error";
    Buf *o = &g_cur->out;
    buf_printf(o, "HTTP/1.0 %d %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\n"
                  "Cache-Control: %s\r\n%s", code, reason, ctype, len, cache ? cache : "no-store", extra ? extra : "");
    if (download_name) {
        char ascii[STORE_NAME_LEN + 8];
        size_t n = 0;
        for (const unsigned char *p = (const unsigned char *)download_name; *p && n + 1 < sizeof ascii; p++)
            ascii[n++] = (*p < 128 && *p != '"' && *p != '\\') ? (char)*p : '_';
        ascii[n] = 0;
        buf_printf(o, "Content-Disposition: attachment; filename=\"%s\"; filename*=UTF-8''", ascii);
        url_esc(o, download_name);
        buf_printf(o, "\r\n");
    }
    buf_printf(o, "\r\n");
    if (g_child_response) {
        // The fork owns a blocking socket. Send directly from the export buffer instead
        // of copying the whole PDF/ZIP into a second, equally large response buffer.
        Download *d = g_cur->download;
        if (d) {
            atomic_store(&d->bytes, (unsigned)len);
            atomic_store(&d->state, DL_SENDING);
        }
        bool ok = !o->err && child_send(o->data, o->len, NULL);
        if (ok && !g_child_head) ok = child_send(data, len, d);
        if (d) atomic_store(&d->state, ok && code == 200 ? DL_DONE : DL_FAILED);
        buf_free(o);
        return;
    }
    if (len) buf_append(o, data, len);
    g_cur->st = C_WRITING;
    g_cur->sent = 0;
}

static void respond(int code, const char *ctype, const Buf *body, const char *download_name) {
    respond_bytes(code, ctype, body ? body->data : NULL, body ? body->len : 0, download_name, NULL, NULL);
}

// A plain-text answer. The ones a person can meet in a browser (a link to a notebook since
// deleted, an export that failed) come in the Kindle's language; those only a script or a
// hand-made address gets (bad request, layer is ink, paper or all) stay English.
static void respond_text(int code, const char *msg) {
    Buf b = {0};
    buf_printf(&b, "%s\n", msg);
    respond(code, "text/plain; charset=utf-8", &b, NULL);
    buf_free(&b);
}

// Grimoire's pixel bat (bat.h) as an SVG path: a rect for each run of blocks in a row, a block one
// unit, in a BAT_W x BAT_H box.
static void bat_path(Buf *b) {
    for (int r = 0; r < BAT_H; r++)
        for (int c = 0; c < BAT_W; c++) {
            if (k_bat[r][c] != 'X' || (c && k_bat[r][c - 1] == 'X')) continue;
            int n = 1;
            while (c + n < BAT_W && k_bat[r][c + n] == 'X') n++;
            buf_printf(b, "M%d %dh%dv1h-%dz", c, r, n, n);
        }
}

// The bat in a page, in the text's color (so it turns with a dark or inverted page), its blocks
// whole design pixels (.bat, below). Decorative: the title beside it says the name.
static void bat_svg(Buf *b) {
    buf_printf(b, "<svg class=bat viewBox='0 0 %d %d' aria-hidden=true><path d='", BAT_W, BAT_H);
    bat_path(b);
    buf_puts(b, "'/></svg>");
}

// The pages are dark (the Kindle's own pictures in them stay as the Kindle draws them: thumbnails,
// pages, Live view's screen), in the colors of :root, which web/live.css and web/studio.css use too.
// The pages wear the Kindle's style. Classic is Terminess throughout. Gothic sets titles in
// Jacquard 24 and text in Jersey 10, pixel faces whose outlines sit on a grid of design pixels
// (30 and 75 font units): sized so one design pixel is a whole number of device pixels (--px
// for text, --pxs for small print, --tp/--tp1 for titles, set from the screen's pixel ratio)
// they stay sharp, and bold is the Kindle's overprint, one design pixel to the right.
static const char k_css_common[] =
    ":root{--fg:#eee;--bg:#181818;--soft:#555;--quiet:#aaa;--well:#242424;--field:#000;color-scheme:dark}"
    "body{margin:24px;color:var(--fg);background:var(--bg)}"
    "h1{margin:0 0 4px}h2{margin:28px 0 10px;border-bottom:2px solid var(--fg)}"
    "a{color:var(--fg)}.grid{display:flex;flex-wrap:wrap;gap:18px}"
    ".card{width:200px;border:2px solid var(--fg);border-radius:10px;padding:10px}"
    ".card img{width:100%;border:1px solid var(--soft);background:#fff}"
    ".name{margin:6px 0 2px;word-break:break-word}.meta{color:var(--quiet)}"
    ".links a{margin-right:10px}"
    ".pages{display:flex;flex-wrap:wrap;gap:12px}.pages figure{margin:0}"
    ".pages img{width:280px;border:1px solid var(--soft);background:#fff;display:block}.pages figcaption{margin-top:4px}"
    ".big{display:inline-block;border:2px solid var(--fg);border-radius:8px;padding:8px 14px;text-decoration:none;"
    "margin:0 8px 8px 0}"
    ".bat{width:calc(var(--bp,2px)*43);height:calc(var(--bp,2px)*14);fill:currentColor;shape-rendering:crispEdges;"
    "flex:none}h1.brand{display:flex;align-items:center;gap:.35em}";
static const char k_css_classic[] =
    "body{font-family:'Grimoire Text','Terminess Nerd Font',ui-monospace,Menlo,monospace;font-size:16px}"
    "h1{font-size:28px}h2{font-size:20px}h1,h2,.name,.big,b,strong{font-weight:bold}.meta{font-size:13px}";
static const char k_css_gothic[] =
    ":root{--px:1.5px;--pxs:1px;--tp:1px;--tp1:1.5px;--bp:var(--tp1)}"
    "body{font-family:'Grimoire Text',ui-monospace,monospace;font-size:calc(var(--px)*18.6667);"
    "line-height:calc(var(--px)*20);font-synthesis:none}"
    ".meta{font-size:calc(var(--pxs)*18.6667);line-height:calc(var(--pxs)*20)}"
    "h1,h2{font-family:'Grimoire Title',serif;font-weight:normal}"
    "h1{font-size:calc(var(--tp1)*43);line-height:calc(var(--tp1)*43)}"
    "h2{font-size:calc(var(--tp)*43);line-height:calc(var(--tp)*43)}"
    ".name{font-weight:normal}.big,b,strong{font-weight:normal;text-shadow:var(--px) 0 currentColor}"
    ".na{text-shadow:none}";  // (html_esc_bold)
// One design pixel per theme size, rounded to whole device pixels (re-run on zoom).
static const char k_js_pixels[] =
    "<script>(function(){function f(){var d=window.devicePixelRatio||1,s=document.documentElement.style;"
    "function k(n,t){s.setProperty(n,Math.max(1,Math.round(t*d))/d+'px')}"
    "k('--px',1.25);k('--pxs',0.9);k('--tp',1);k('--tp1',1.5)}f();addEventListener('resize',f)})()</script>";

// The pages are in the Kindle's language (i18n.h): the lang attribute says which.
static void page_head(Buf *b, const char *title) {
    buf_printf(b, "<!doctype html><html lang=%s><head><meta charset=utf-8>"
                  "<meta name=viewport content='width=device-width,initial-scale=1'><title>", i18n_current());
    html_esc(b, title);
    buf_printf(b, "</title><link rel=icon type=image/svg+xml href='/icon.svg?v=%s'>"
                  "<link rel=apple-touch-icon href='/apple-touch-icon.png?v=%s'>", GRIMOIRE_VERSION, GRIMOIRE_VERSION);
    if (g_gothic) {
        buf_puts(b, k_js_pixels);
        buf_printf(b, "<style>@font-face{font-family:'Grimoire Title';src:url(/font/jacquard24.ttf?v=%s) "
                      "format('truetype');font-display:swap}@font-face{font-family:'Grimoire Text';"
                      "src:url(/font/jersey10.ttf?v=%s) format('truetype');font-display:swap}",
                   GRIMOIRE_VERSION, GRIMOIRE_VERSION);
        buf_puts(b, k_css_gothic);
    } else {
        buf_printf(b, "<style>@font-face{font-family:'Grimoire Text';src:url(/font/terminess.ttf?v=%s) "
                      "format('truetype');font-weight:400;font-display:swap}@font-face{font-family:'Grimoire Text';"
                      "src:url(/font/terminess-bold.ttf?v=%s) format('truetype');font-weight:700;font-display:swap}",
                   GRIMOIRE_VERSION, GRIMOIRE_VERSION);
        buf_puts(b, k_css_classic);
    }
    buf_puts(b, k_css_common);
    buf_printf(b, "</style></head><body class=%s>", g_gothic ? "gothic" : "classic");
}

// The fonts the pages use: the binary's own copies, cached by the browser for a year (the
// version in the URL changes when they could).
static void route_font(const char *name) {
    static const struct {
        const char *name;
        const unsigned char *data;
        const unsigned int *len;
    } fonts[] = {
        {"jacquard24.ttf", Jacquard24_Regular_ttf, &Jacquard24_Regular_ttf_len},
        {"jersey10.ttf", Jersey10_Regular_ttf, &Jersey10_Regular_ttf_len},
        {"terminess.ttf", TerminessNerdFontMono_Regular_ttf, &TerminessNerdFontMono_Regular_ttf_len},
        {"terminess-bold.ttf", TerminessNerdFontMono_Bold_ttf, &TerminessNerdFontMono_Bold_ttf_len},
    };
    for (size_t i = 0; i < sizeof fonts / sizeof fonts[0]; i++)
        if (!strcmp(name, fonts[i].name))
            return respond_bytes(200, "font/ttf", fonts[i].data, *fonts[i].len, NULL,
                                 "public, max-age=31536000, immutable", NULL);
    // TRANSLATORS: a plain-text answer in the browser: an address the share server has nothing at.
    respond_text(404, _("not found"));
}

static void nb_card(Buf *b, const NotebookInfo *nb) {
    buf_printf(b, "<div class=card><a href='/nb/%s'><img loading=lazy src='/nb/%s/thumb.png' alt=''></a>"
                  "<div class=name>", nb->id, nb->id);
    html_esc(b, nb->name);
    char when[64], pages[64];
    format_time_short(nb->modified, when, sizeof when);
    // TRANSLATORS: on the share index, under a notebook: its pages, when it last changed and its
    // paper, then links to it as a page of pictures and to the studio (Effects).
    snprintf(pages, sizeof pages, NG_("%d page", "%d pages", nb->npages), nb->npages);
    buf_printf(b, "</div><div class=meta>");
    html_esc(b, pages);
    buf_printf(b, " &middot; ");
    html_esc(b, when);
    buf_printf(b, " &middot; ");
    html_esc(b, paper_label(nb->paper));
    buf_printf(b, "</div><div class=links><a href='/nb/%s.pdf'>PDF</a><a href='/nb/%s.zip'>ZIP</a><a href='/nb/%s'>",
               nb->id, nb->id, nb->id);
    html_esc(b, C_("share link", "pages"));
    buf_printf(b, "</a><a href='/nb/%s/studio'>", nb->id);
    html_esc(b, C_("share link", "effects"));
    buf_printf(b, "</a></div></div>");
}

static void download_footer(Buf *b) {
    // TRANSLATORS: PDF download status in Share. {done}/{total} are page counts;
    // {percent} includes the percent sign: sent to the browser, not saved to its disk.
    const char *words[] = {_("Preparing PDF..."), _("Preparing PDF: {done} / {total} pages"),
        _("Sending PDF: {percent}"), _("Sent to your browser. Check Downloads."),
        _("PDF download failed. Please try again."),
        _("Download status unavailable. Check your browser's Downloads and the Kindle's Wi-Fi."),
        _("Dismiss"), _("Waiting for an export slot...")};
    buf_printf(b, "<div id=download-status></div><script>window.GRIMOIRE_DOWNLOAD_WORDS=[");
    for (size_t i = 0; i < sizeof words / sizeof *words; i++) {
        if (i) buf_printf(b, ",");
        json_str(b, words[i]);
    }
    buf_printf(b, "];</script><script>");
    buf_append(b, download_js, download_js_len);
    buf_printf(b, "</script></body></html>");
}

static void route_index(void) {
    NotebookInfo *nbs;
    int n;
    char (*folders)[STORE_NAME_LEN];
    int nf;
    store_list(&nbs, &n);
    store_list_folders(&folders, &nf);
    Buf b = {0};
    page_head(&b, "Grimoire");
    char line[160];
    // TRANSLATORS: the share index, the page a browser opens at the Share screen's address.
    snprintf(line, sizeof line, NG_("%d notebook on your Kindle", "%d notebooks on your Kindle", n), n);
    buf_puts(&b, "<h1 class=brand>");
    bat_svg(&b);
    buf_puts(&b, "Grimoire</h1><p class=meta>");
    html_esc(&b, line);
    buf_printf(&b, "</p><p><a class=big href='/all.zip'>");
    // TRANSLATORS: the share index's big link to a zip of every notebook.
    html_esc_bold(&b, _("Download everything (zip)"));
    buf_printf(&b, "</a></p>");
    for (int f = -1; f < nf; f++) {
        const char *folder = f < 0 ? "" : folders[f];
        int count = 0;
        for (int i = 0; i < n; i++) count += !strcmp(nbs[i].folder, folder);
        if (f < 0 && !count) continue;
        buf_printf(&b, "<h2>");
        // TRANSLATORS: the share index's heading over the notebooks in no folder.
        html_esc(&b, f < 0 ? _("Library") : folder);
        if (f >= 0) {
            buf_printf(&b, " <a class=meta href='/folder/");
            url_esc(&b, folder);
            buf_printf(&b, ".zip'>(zip)</a>");
        }
        buf_printf(&b, "</h2><div class=grid>");
        for (int i = 0; i < n; i++)
            if (!strcmp(nbs[i].folder, folder)) nb_card(&b, &nbs[i]);
        if (!count) {
            buf_printf(&b, "<p class=meta>");
            // TRANSLATORS: on the share index, under a folder with no notebooks.
            html_esc(&b, C_("folder", "empty"));
            buf_printf(&b, "</p>");
        }
        buf_printf(&b, "</div>");
    }
    buf_printf(&b, "<p class=meta style='margin-top:40px'>Grimoire %s</p>", GRIMOIRE_VERSION);
    download_footer(&b);
    respond(200, "text/html; charset=utf-8", &b, NULL);
    buf_free(&b);
    free(nbs);
    free(folders);
}

static void route_notebook_html(const NotebookInfo *nb, const char *query) {
    char value[32], *end;
    long wanted = query_get(query, "p", value, sizeof value) ? strtol(value, &end, 10) : 1;
    int current = wanted < 1 ? 1 : wanted > nb->npages ? nb->npages : (int)wanted;
    if (current < 1) current = 1;
    Buf b = {0};
    page_head(&b, nb->name);
    buf_printf(&b, "<nav aria-label='Pages' style='position:sticky;top:0;background:var(--bg,#fff);padding:12px;z-index:1'>"
                   "<form method=get style='display:flex;flex-wrap:wrap;align-items:center;justify-content:center;gap:12px'>");
    if (current > 1) buf_printf(&b, "<a class=big href='?p=%d' rel=prev>&larr;</a>", current - 1);
    buf_printf(&b, "<label for=page-number>");
    html_esc(&b, _("Page"));
    buf_printf(&b, "</label><input id=page-number name=p type=number inputmode=numeric min=1 max=%d value=%d "
                   "style='width:6ch;font:inherit;padding:8px;text-align:center' required onchange='this.form.submit()'><span>/ %d</span><button type=submit>",
                   nb->npages, current, nb->npages);
    html_esc(&b, _("Go"));
    buf_printf(&b, "</button>");
    if (current < nb->npages) buf_printf(&b, "<a class=big href='?p=%d' rel=next>&rarr;</a>", current + 1);
    buf_printf(&b, "</form></nav>");
    buf_printf(&b, "<p><a href='/'>&larr; ");
    // TRANSLATORS: a notebook's share page: back to the index, then its downloads (the zip holds
    // the PDF, a PNG of each page and the notebook's own files), the studio, and its pages.
    html_esc(&b, _("all notebooks"));
    buf_printf(&b, "</a></p><h1>");
    html_esc(&b, nb->name);
    buf_printf(&b, "</h1><p><a class=big href='/nb/%s.pdf'>PDF</a><a class=big href='/nb/%s.zip'>", nb->id, nb->id);
    html_esc_bold(&b, _("ZIP (PDF + PNG pages + raw)"));
    buf_printf(&b, "</a><a class=big href='/nb/%s/studio'>", nb->id);
    html_esc_bold(&b, _("Effects (transparent, colors, gradients, filters)"));
    buf_printf(&b, "</a></p>");
    buf_printf(&b, "<figure style='margin:16px 0;text-align:center'><a href='/nb/%s/%d.png'>"
                   "<img style='max-width:100%%;height:auto' src='/nb/%s/%d.png?f=2' alt='", nb->id, current, nb->id, current);
    char page_label[64];
    snprintf(page_label, sizeof page_label, _("Page %d"), current);
    html_esc(&b, page_label);
    buf_printf(&b, "'></a><figcaption>");
    html_esc(&b, page_label);
    buf_printf(&b, " &middot; <a href='/nb/%s/studio?p=%d'>", nb->id, current);
    html_esc(&b, C_("share link", "effects"));
    buf_printf(&b, "</a></figcaption></figure>");
    download_footer(&b);
    respond(200, "text/html; charset=utf-8", &b, NULL);
    buf_free(&b);
}

// The studio's words in the language in use (window.GRIMOIRE_L10N), from the catalog's context
// "studio" (make l10n puts web/studio.js's there): {"English": "translation"}, through json_str
// (safe inside the script). A plural comes in its form for the notebook's page count, the one
// count the studio words (studio.js, tPages). English: {}, and studio.js keeps its own words.
typedef struct {
    Buf *b;
    int n;
    long pages;
} StudioWords;

static void studio_word(const I18nEntry *e, void *arg) {
    StudioWords *w = arg;
    if (strcmp(e->ctx, "studio")) return;
    const char *s = e->id_plural ? i18n_getnc("studio", e->id, e->id_plural, w->pages) : e->str;
    if (!s || !*s) return;
    if (w->n++) buf_printf(w->b, ",");
    json_str(w->b, e->id);
    buf_printf(w->b, ":");
    json_str(w->b, s);
}

// The studio: the page's controls, the notebook as JSON, and web/studio.{css,js} inline. The
// browser does the image work from /nb/<id>/<n>.png?layer=ink|paper.
static void route_studio(const NotebookInfo *nb) {
    Buf b = {0};
    page_head(&b, nb->name);
    buf_printf(&b, "<p><a href='/nb/%s'>&larr; ", nb->id);
    html_esc(&b, nb->name);
    buf_printf(&b, "</a></p><h1>");
    html_esc(&b, nb->name);
    buf_printf(&b, "</h1><div id=studio><p class=meta>");
    // TRANSLATORS: the studio's page while its script starts, or in a browser without JavaScript.
    html_esc(&b, _("Loading the studio..."));
    buf_printf(&b, "</p></div><noscript><p>");
    html_esc(&b, _("The studio needs JavaScript."));
    buf_printf(&b, "</p></noscript><script>window.GRIMOIRE_NB={\"id\":");
    json_str(&b, nb->id);
    buf_printf(&b, ",\"name\":");
    json_str(&b, nb->name);
    buf_printf(&b, ",\"pages\":%d,\"dpi\":%.0f,\"version\":", nb->npages, g_dpi);
    json_str(&b, GRIMOIRE_VERSION);
    buf_printf(&b, "};window.GRIMOIRE_L10N={");
    StudioWords w = {&b, 0, nb->npages};
    i18n_walk(i18n_current(), studio_word, &w);
    buf_printf(&b, "};</script><style>");
    buf_append(&b, studio_css, studio_css_len);
    buf_printf(&b, "</style><script>");
    buf_append(&b, studio_js, studio_js_len);
    buf_printf(&b, "</script></body></html>");
    respond(200, "text/html; charset=utf-8", &b, NULL);
    buf_free(&b);
}

static void route_list_json(void) {
    NotebookInfo *nbs;
    int n;
    char (*folders)[STORE_NAME_LEN];
    int nf;
    store_list(&nbs, &n);
    store_list_folders(&folders, &nf);
    Buf b = {0};
    buf_printf(&b, "{\"device\":\"Grimoire\",\"version\":");
    json_str(&b, GRIMOIRE_VERSION);
    buf_printf(&b, ",\"notebooks\":[");
    for (int i = 0; i < n; i++) {
        const NotebookInfo *nb = &nbs[i];
        buf_printf(&b, "%s{\"id\":", i ? "," : "");
        json_str(&b, nb->id);
        buf_printf(&b, ",\"name\":");
        json_str(&b, nb->name);
        buf_printf(&b, ",\"folder\":");
        json_str(&b, nb->folder);
        buf_printf(&b, ",\"pages\":%d,\"paper\":", nb->npages);
        json_str(&b, paper_name(nb->paper));
        buf_printf(&b, ",\"modified\":%lld,\"pdf\":\"/nb/%s.pdf\",\"zip\":\"/nb/%s.zip\",\"pngs\":[",
                   (long long)nb->modified, nb->id, nb->id);
        for (int p = 0; p < nb->npages; p++) buf_printf(&b, "%s\"/nb/%s/%d.png\"", p ? "," : "", nb->id, p + 1);
        buf_printf(&b, "]}");
    }
    buf_printf(&b, "],\"folders\":[");
    for (int f = 0; f < nf; f++) {
        if (f) buf_printf(&b, ",");
        json_str(&b, folders[f]);
    }
    buf_printf(&b, "]}\n");
    respond(200, "application/json", &b, NULL);
    buf_free(&b);
    free(nbs);
    free(folders);
}

// ---- Live view ----

// A request header's value (the first line naming it), or false.
static bool header_get(const char *req, const char *name, char *out, size_t cap) {
    size_t nl = strlen(name);
    for (const char *h = strchr(req, '\n'); h; h = strchr(h, '\n')) {
        h++;
        if (strncasecmp(h, name, nl) || h[nl] != ':') continue;
        h += nl + 1;
        while (*h == ' ' || *h == '\t') h++;
        size_t n = strcspn(h, "\r\n");
        if (n >= cap) return false;
        memcpy(out, h, n);
        out[n] = 0;
        return true;
    }
    return false;
}

static unsigned session_scopes(void) {
    char cookie[1024];
    return header_get(g_cur->req, "Cookie", cookie, sizeof cookie) ? pair_cookie_scopes(cookie) : 0;
}

static int64_t now_s(void) { return (int64_t)(now_ms() / 1000u); }

static void redirect(const char *to, const char *cookie) {
    char hdr[256];
    snprintf(hdr, sizeof hdr, "Location: %s\r\n%s%s%s", to, cookie ? "Set-Cookie: " : "", cookie ? cookie : "",
             cookie ? "\r\n" : "");
    respond_bytes(302, "text/plain; charset=utf-8", NULL, 0, NULL, NULL, hdr);
}

static void live_word(Buf *b, int *n, const char *key, const char *text) {
    buf_printf(b, "%s\"%s\":", (*n)++ ? "," : "", key);
    json_str(b, text);
}

// The page: the code form until this browser has paired, then the viewer (web/live.{css,js}).
static void route_live_page(const char *query) {
    Buf b = {0};
    // TRANSLATORS: Live view's name: the Kindle's screen watched in a computer's browser. The
    // browser page's title, and the pairing card's on the Kindle.
    page_head(&b, _("Live view"));
    if (session_scopes() & PAIR_LIVE) {
        buf_printf(&b, "<div id=live><p class=meta>");
        // TRANSLATORS: Live view's page in the browser while its script starts.
        html_esc(&b, _("Connecting..."));
        buf_printf(&b, "</p></div><noscript><p>");
        // TRANSLATORS: Live view's page in a browser without JavaScript.
        html_esc(&b, _("Live view needs JavaScript."));
        buf_printf(&b, "</p></noscript><script>window.GRIMOIRE_LIVE_WORDS={");
        int n = 0;
        // TRANSLATORS: Live view's status in the browser, and its buttons (the browser's own words
        // for these are fine: they are not on the Kindle).
        live_word(&b, &n, "connecting", _("Connecting..."));
        live_word(&b, &n, "reconnecting", _("Reconnecting..."));
        live_word(&b, &n, "live", _("Live"));
        live_word(&b, &n, "asleep", _("The Kindle is asleep"));
        live_word(&b, &n, "pairing", _("Pairing on the Kindle..."));
        live_word(&b, &n, "ended", _("Live view is off on the Kindle"));
        live_word(&b, &n, "fit", _("Fit"));
        live_word(&b, &n, "actual", _("Actual size"));
        live_word(&b, &n, "clear", _("Transparent paper"));
        // TRANSLATORS: Live view's switch in the browser: the picture white on black. It follows
        // Invert colors on the Kindle until it is switched; then "As on the Kindle" goes back to that.
        live_word(&b, &n, "invert", _("Invert colors"));
        live_word(&b, &n, "follow", _("As on the Kindle"));
        // TRANSLATORS: Live view's slider in the browser: the picture's light from white to warm,
        // following the Kindle's front light warmth until it is moved ("As on the Kindle" again).
        live_word(&b, &n, "warmth", _("Warmth"));
        // TRANSLATORS: Live view's switch in the browser: where the Kindle's pen is, drawn over the
        // picture (a dot, see-through while it hovers over the screen, solid while it touches), in
        // recordings too.
        live_word(&b, &n, "pen", _("Show pen"));
        live_word(&b, &n, "record", _("Record"));
        // TRANSLATORS: Live view's Record button where the browser can make no MP4 at all (WebM is the
        // other video format), and its tooltip saying why.
        live_word(&b, &n, "recordWebm", _("Record (WebM)"));
        live_word(&b, &n, "webmBrowser", _("This browser cannot record MP4, so the video is WebM."));
        live_word(&b, &n, "stop", _("Stop recording"));
        live_word(&b, &n, "full", _("Full screen"));
        // TRANSLATORS: Live view's button in the browser: the screen in a small window that floats over
        // the computer's other windows (the browser's own name for it is fine).
        live_word(&b, &n, "pip", _("Picture in picture"));
        live_word(&b, &n, "old", _("This browser cannot show Live view."));
        buf_printf(&b, "};</script><template id=live-bat>");
        bat_svg(&b);
        buf_printf(&b, "</template><style>");
        buf_append(&b, live_css, live_css_len);
        buf_printf(&b, "</style><script>");
        buf_append(&b, live_js, live_js_len);
        buf_printf(&b, "</script></body></html>");
        respond(200, "text/html; charset=utf-8", &b, NULL);
        buf_free(&b);
        return;
    }
    char e[16] = "";
    if (query) query_get(query, "e", e, sizeof e);
    buf_printf(&b, "<style>");
    buf_append(&b, live_css, live_css_len);
    buf_printf(&b, "</style><h1 class=brand>");
    bat_svg(&b);
    html_esc(&b, _("Live view"));
    buf_printf(&b, "</h1>");
    if (!(g_owners & HTTPD_LIVE)) {
        buf_printf(&b, "<p>");
        // TRANSLATORS: Live view's page while it is off on the Kindle.
        html_esc(&b, _("Live view is off. On the Kindle, open Share and tap Live view."));
        buf_printf(&b, "</p></body></html>");
        respond(200, "text/html; charset=utf-8", &b, NULL);
        buf_free(&b);
        return;
    }
    buf_printf(&b, "<form class=code action='/live/pair' method=get><p><label>");
    // TRANSLATORS: Live view's code form in the browser: the six digits the Kindle shows.
    html_esc(&b, _("Type the code shown on the Kindle:"));
    buf_printf(&b, "<br><input name=code inputmode=numeric autocomplete=one-time-code maxlength=7 required "
                   "autofocus></label><button>");
    // TRANSLATORS: the code form's button: start watching the Kindle's screen.
    html_esc(&b, _("Watch"));
    buf_printf(&b, "</button></p>");
    if (!strcmp(e, "wrong") || !strcmp(e, "locked")) {
        buf_printf(&b, "<p class=err role=alert>");
        // TRANSLATORS: the code form after a wrong code, and after five (the Kindle shows a new one).
        html_esc(&b, !strcmp(e, "wrong") ? _("That code is not the one on the Kindle.")
                                         : _("Too many tries. Wait 30 seconds, then type the new code."));
        buf_printf(&b, "</p>");
    }
    buf_printf(&b, "</form></body></html>");
    respond(200, "text/html; charset=utf-8", &b, NULL);
    buf_free(&b);
}

static void route_live(const char *path, const char *query) {
    if (!strcmp(path, "/live") || !strcmp(path, "/live/")) return route_live_page(query);
    if (!(g_owners & HTTPD_LIVE)) return respond_text(404, _("not found"));
    if (!strcmp(path, "/live/pair")) {
        // The code as typed (spaces dropped: "482 913"), or the QR code's token.
        char cred[48] = "", clean[48];
        if (!query_get(query, "code", cred, sizeof cred)) query_get(query, "t", cred, sizeof cred);
        size_t n = 0;
        for (const char *p = cred; *p && n + 1 < sizeof clean; p++)
            if (*p != ' ' && *p != '-') clean[n++] = *p;
        clean[n] = 0;
        char tok[PAIR_TOKEN_LEN + 1], cookie[128];
        int rc = pair_redeem(clean, now_s(), tok);
        if (rc == PAIR_OK && pair_cookie_header(tok, cookie, sizeof cookie) == 0) {
            g_live_pairings++;
            qlog("live: a browser paired");
            return redirect("/live", cookie);
        }
        qlog("live: a pairing was refused (%s)", rc == PAIR_LOCKED ? "locked" : "wrong code");
        return redirect(rc == PAIR_LOCKED ? "/live?e=locked" : "/live?e=wrong", NULL);
    }
    if (!strcmp(path, "/live/stream")) {
        if (!(session_scopes() & PAIR_LIVE)) return respond_text(401, "pair first");
        if (g_head || g_child_response) return respond_text(400, "GET only");
        if (!live_on()) return respond_text(503, "live view is off");
        // TRANSLATORS: Live view's answer to a third computer (two can watch at once).
        if (!live_take_viewer(g_cur->fd)) return respond_text(503, _("Two computers are watching already."));
        qlog("live: a viewer started (%d watching)", live_viewers());
        client_close(g_cur);  // the viewer child owns the connection now
        return;
    }
    respond_text(404, _("not found"));
}

// The browser's icon for the pages: the bat in a square, black, or light where the browser is dark.
static void route_icon_svg(void) {
    Buf b = {0};
    buf_printf(&b, "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 %g %d %d' shape-rendering='crispEdges'>"
                   "<style>path{fill:#111}@media (prefers-color-scheme:dark){path{fill:#eee}}</style><path d='",
               -(BAT_W - BAT_H) / 2.0, BAT_W, BAT_W);
    bat_path(&b);
    buf_puts(&b, "'/></svg>");
    respond_bytes(200, "image/svg+xml", b.data, b.len, NULL, "public, max-age=31536000, immutable", NULL);
    buf_free(&b);
}

// The icon a phone keeps for the pages (an iPhone's home screen or bookmarks): the bat in blocks of
// 4 px, black on white, 180 px square.
static void route_touch_icon(void) {
    enum { S = 180, K = 4 };
    Canvas c;
    if (canvas_init(&c, S, S) != 0) return respond_text(500, "no memory");
    draw_bat(&c, (S - BAT_W * K) / 2, (S - BAT_H * K) / 2, K);
    int len = 0;
    extern unsigned char *stbi_write_png_to_mem(const unsigned char *, int, int, int, int, int *);
    unsigned char *png = stbi_write_png_to_mem(c.px, c.stride, c.w, c.h, 1, &len);
    canvas_free(&c);
    if (!png) return respond_text(500, "encode failed");
    respond_bytes(200, "image/png", png, (size_t)len, NULL, "public, max-age=31536000, immutable", NULL);
    free(png);
}

static void route(char *path) {
    char query[128] = "";
    char *q = strchr(path, '?');
    if (q) {
        strlcpy_q(query, q + 1, sizeof query);
        *q = 0;
    }
    url_decode(path);
    if (strstr(path, "..")) return respond_text(400, "bad path");
    if (!strncmp(path, "/live", 5) && (!path[5] || path[5] == '/')) return route_live(path, query);
    if (!strncmp(path, "/font/", 6)) return route_font(path + 6);
    if (!strcmp(path, "/icon.svg")) return route_icon_svg();
    if (!strcmp(path, "/apple-touch-icon.png") || !strcmp(path, "/apple-touch-icon-precomposed.png"))
        return route_touch_icon();
    // TRANSLATORS: the answer in a browser to a download (or the notebooks' page) while only Live view
    // keeps the server up: the Share screen must be open for those.
    if (!(g_owners & HTTPD_SHARE)) return respond_text(403, _("Open Share on the Kindle to download."));
    if (!strcmp(path, "/") || !strcmp(path, "/index.html")) return route_index();
    if (!strcmp(path, "/api/list.json")) return route_list_json();
    if (!strncmp(path, "/api/download/", 14)) {
        Download *d = download_find(path + 14);
        if (!d) return respond_text(404, "unknown download");
        Buf b = {0};
        buf_printf(&b, "{\"state\":%u,\"pages\":%u,\"total\":%u,\"sent\":%u,\"bytes\":%u}",
                   atomic_load(&d->state), atomic_load(&d->pages), atomic_load(&d->total),
                   atomic_load(&d->sent), atomic_load(&d->bytes));
        respond(200, "application/json", &b, NULL);
        buf_free(&b);
        return;
    }
    if (!strcmp(path, "/all.zip")) {
        Buf b = {0};
        int rc = export_folder_zip("", g_dpi, &b);
        if (rc == 0) respond(200, "application/zip", &b, "Grimoire.zip");
        else respond_text(500, _("export failed"));
        buf_free(&b);
        return;
    }
    if (!strncmp(path, "/folder/", 8)) {
        char folder[STORE_NAME_LEN + 8];  // the longest folder name, and ".zip" after it
        strlcpy_q(folder, path + 8, sizeof folder);
        size_t l = strlen(folder);
        if (l < 5 || strcmp(folder + l - 4, ".zip")) return respond_text(404, _("not found"));
        folder[l - 4] = 0;
        if (l - 4 >= STORE_NAME_LEN) return respond_text(404, _("not found"));  // longer than any folder
        Buf b = {0};
        char fname[STORE_NAME_LEN + 8], safe[STORE_NAME_LEN];
        export_safe_name(folder, safe, sizeof safe);
        snprintf(fname, sizeof fname, "%s.zip", safe);
        if (export_folder_zip(folder, g_dpi, &b) == 0) respond(200, "application/zip", &b, fname);
        else respond_text(500, _("export failed"));
        buf_free(&b);
        return;
    }
    if (strncmp(path, "/nb/", 4)) return respond_text(404, _("not found"));
    // /nb/<id>[.pdf|.zip|/<n>.png[?layer=ink|paper]|/thumb.png|/studio]
    char id[STORE_ID_LEN + 16];
    const char *rest = path + 4;
    size_t il = strcspn(rest, "./");
    if (il == 0 || il >= STORE_ID_LEN) return respond_text(404, _("not found"));
    memcpy(id, rest, il);
    id[il] = 0;
    rest += il;
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb) return respond_text(500, "out of memory");
    if (store_load_info(id, nb) != 0) {
        free(nb);
        return respond_text(404, _("no such notebook"));
    }
    char safe[STORE_NAME_LEN], fname[STORE_NAME_LEN + 16];
    export_safe_name(nb->name, safe, sizeof safe);
    Buf b = {0};
    int page = 0, end = 0;
    if (!*rest || !strcmp(rest, "/")) {
        route_notebook_html(nb, query);
    } else if (!strcmp(rest, "/studio")) {
        route_studio(nb);
    } else if (!strcmp(rest, ".pdf")) {
        snprintf(fname, sizeof fname, "%s.pdf", safe);
        if (export_notebook_pdf_progress(nb, g_dpi, &b, download_pages, g_cur->download) == 0)
            respond(200, "application/pdf", &b, fname);
        else respond_text(500, _("export failed"));
    } else if (!strcmp(rest, ".zip")) {
        ZipWriter z;
        zip_begin(&z, &b);
        int rc = export_notebook_zip(nb, g_dpi, &z, "");
        if (zip_end(&z) != 0) rc = -1;
        snprintf(fname, sizeof fname, "%s.zip", safe);
        if (rc == 0) respond(200, "application/zip", &b, fname);
        else respond_text(500, _("export failed"));
    } else if (!strcmp(rest, "/thumb.png")) {
        Canvas t;
        if (store_load_thumb(nb->id, &t) == 0) {
            int len = 0;
            extern unsigned char *stbi_write_png_to_mem(const unsigned char *, int, int, int, int, int *);
            unsigned char *png = stbi_write_png_to_mem(t.px, t.stride, t.w, t.h, 1, &len);
            canvas_free(&t);
            if (png) {
                buf_append(&b, png, (size_t)len);
                free(png);
                respond(200, "image/png", &b, NULL);
            } else
                respond_text(500, "encode failed");
        } else if (export_page_png(nb, 0, g_dpi, &b) == 0) {
            respond(200, "image/png", &b, NULL);  // no thumbnail yet: first page at full size
        } else
            respond_text(404, "no thumbnail");
    } else if (sscanf(rest, "/%d.png%n", &page, &end) == 1 && end && !rest[end] && page >= 1 && page <= nb->npages) {
        // ?layer=ink: the strokes alone on white; ?layer=paper: the empty page; ?f=2..8: shrunk
        // f times (previews). For the studio, headers carry the full page size, the paper kind
        // and, for the ink, the box it covers at full size.
        char layer[16] = "all", fs[8] = "1";
        query_get(query, "layer", layer, sizeof layer);
        query_get(query, "f", fs, sizeof fs);
        int layers = !strcmp(layer, "ink") ? PAGE_LAYER_INK : !strcmp(layer, "paper") ? PAGE_LAYER_PAPER
                   : !strcmp(layer, "all") ? PAGE_LAYER_ALL : 0;
        int f = atoi(fs);
        PageRender pr;
        if (!layers || f < 1 || f > 8) {
            respond_text(400, "layer is ink, paper or all; f is 1 to 8");
        } else if (export_page_png_ex(nb, page - 1, g_dpi, layers, f, &pr, &b) == 0) {
            char hdr[160];
            int n = snprintf(hdr, sizeof hdr, "X-Page-Size: %dx%d\r\nX-Paper: %u\r\n", pr.w, pr.h, pr.paper);
            if (layers == PAGE_LAYER_INK && n > 0 && (size_t)n < sizeof hdr) {
                if (rect_is_empty(pr.ink)) snprintf(hdr + n, sizeof hdr - (size_t)n, "X-Ink-Box: none\r\n");
                else snprintf(hdr + n, sizeof hdr - (size_t)n, "X-Ink-Box: %d,%d,%d,%d\r\n", pr.ink.x, pr.ink.y,
                              pr.ink.w, pr.ink.h);
            }
            respond_bytes(200, "image/png", b.data, b.len, NULL, NULL, hdr);
        } else {
            respond_text(500, _("export failed"));
        }
    } else {
        respond_text(404, _("not found"));
    }
    buf_free(&b);
    free(nb);
}

// The Host header must name the Kindle by address (or be absent). A page on another site that
// rebinds its own hostname to the Kindle's address sends that hostname, and is refused.
static bool host_ok(const char *req) {
    const char *h = req;
    while ((h = strchr(h, '\n'))) {
        h++;
        if (strncasecmp(h, "Host:", 5)) continue;
        h += 5;
        while (*h == ' ' || *h == '\t') h++;
        size_t n = strcspn(h, "\r\n");
        char host[128];
        if (n == 0 || n >= sizeof host) return false;
        memcpy(host, h, n);
        host[n] = 0;
        if (host[0] == '[') return true;  // IPv6 literal
        char *colon = strchr(host, ':');
        if (colon) *colon = 0;
        if (!strcmp(host, "localhost")) return true;
        struct in_addr ia;
        return inet_pton(AF_INET, host, &ia) == 1;
    }
    return true;
}

static bool heavy(const char *path) {
    if (!strncmp(path, "/all.zip", 8) || !strncmp(path, "/folder/", 8)) return true;
    if (strncmp(path, "/nb/", 4)) return false;
    return strstr(path, ".pdf") || strstr(path, ".zip") || (strstr(path, ".png") && !strstr(path, "thumb.png"));
}

// Milliseconds since t (signed, so a t taken a moment after `now` is not 49 days ago).
static int32_t ms_since(uint32_t now, uint32_t t) { return (int32_t)(now - t); }

static void keep_headers(Buf *out) {  // HEAD: the answer without its body
    char *end = memmem(out->data, out->len, "\r\n\r\n", 4);
    if (end) out->len = (size_t)(end - (char *)out->data) + 4;
}

// Child side of a heavy request: render, send everything (blocking), exit.
static void serve_in_child(Client *c, const char *method, char *path) {
    work_pump_set(NULL);  // the pens are Grimoire's to read (common.h), never a child's
    int keep = c->fd;
    for (int fd = 3; fd < 1024; fd++)
        if (fd != keep) close(fd);
    if (nice(5) < 0) { /* best effort */ }
#ifdef GRIMOIRE_SIM
    const char *slow = getenv("GRIMOIRE_HTTPD_DELAY_MS");  // tests: renders as slow as the Kindle's
    if (slow) sleep_ms(atoi(slow));
#endif
    fcntl(keep, F_SETFL, fcntl(keep, F_GETFL, 0) & ~O_NONBLOCK);
    struct timeval tv = {60, 0};  // a reader that stalls for a minute is gone
    setsockopt(keep, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof tv);
    g_cur = c;
    g_child_response = true;
    g_child_head = !strcmp(method, "HEAD");
    route(path);
    shutdown(keep, SHUT_WR);
    _exit(0);
}

static void handle_request(Client *c) {
    char method[8] = {0}, path[600] = {0};
    g_cur = c;
    g_head = false;
    if (sscanf(c->req, "%7s %599s", method, path) != 2) return respond_text(400, "bad request");
    g_head = !strcmp(method, "HEAD");
    if (strcmp(method, "GET") && strcmp(method, "HEAD")) return respond_text(405, "GET only");
    if (!host_ok(c->req)) {
        qlog("httpd: refused a request for another host");
        // TRANSLATORS: the answer to a browser that asked for the Kindle by another name than its
        // address (as a page on another site would).
        return respond_text(400, _("use the address shown on the Kindle"));
    }
    // grimoire.log goes to public bug reports: a folder's name (the user's own words) stays out,
    // however the path is spelled, and so does anything that is not a plain path (a request in
    // proxy form carries this Kindle's address).
    char shown[sizeof path];
    strlcpy_q(shown, path, sizeof shown);
    url_decode(shown);
    // Nor does Live view's code or token, nor its query at all.
    bool live = !strncmp(shown, "/live", 5) && (!shown[5] || shown[5] == '/' || shown[5] == '?');
    char bare[sizeof path];
    strlcpy_q(bare, path, sizeof bare);
    bare[strcspn(bare, "?")] = 0;
    qlog("httpd: %s %s", method,
         path[0] != '/' ? "(not a path)" : !strncmp(shown, "/folder/", 8) ? "/folder/<name>.zip" : live ? bare : path);
    if (heavy(path) && !(g_owners & HTTPD_SHARE)) {
        route(path);  // (refused: no render for it)
        return;
    }
    if (heavy(path)) {
        char token[64];
        const char *query = strchr(path, '?');
        const char *ext = strstr(path, ".pdf");
        if (!strcmp(method, "GET") && !strncmp(path, "/nb/", 4) && ext && (!ext[4] || ext[4] == '?') &&
            query && query_get(query + 1, "progress", token, sizeof token)) {
            c->download = download_begin(token);
            if (!c->download) return respond_text(400, "invalid or busy download identifier");
        }
        c->st = C_QUEUED;  // start_renders() takes it from here; t0 = when the request came in
        return;
    }
    route(path);
    if (!strcmp(method, "HEAD")) keep_headers(&c->out);
    g_served++;
}

// True once the browser has hung up (the studio drops pages you flipped past).
static bool peer_gone(const Client *c) {
    char b;
    ssize_t r = recv(c->fd, &b, 1, MSG_PEEK | MSG_DONTWAIT);
    return r == 0 || (r < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR);
}

// Queued renders start oldest first while a child slot is free; one whose browser has gone is
// dropped before it costs a render.
static void start_renders(void) {
    for (;;) {
        Client *next = NULL;
        for (int i = 0; i < MAX_CLIENTS; i++) {
            Client *c = &g_cl[i];
            if (c->st != C_QUEUED) continue;
            if (peer_gone(c)) {
                qlog("httpd: dropped a render the browser no longer wants");
                client_close(c);
            } else if (!next || c->seq < next->seq) {
                next = c;
            }
        }
        int slot = -1;
        for (int i = 0; i < MAX_CHILDREN; i++)
            if (g_children[i] <= 0) slot = i;
        if (!next || slot < 0) return;
        char method[8] = {0}, path[600] = {0};
        sscanf(next->req, "%7s %599s", method, path);
        g_served++;
        Download *d = next->download;
        if (d) { d->busy = true; atomic_store(&d->state, DL_PREPARING); }
        pid_t pid = fork();
        if (pid == 0) serve_in_child(next, method, path);
        if (pid > 0) {
            g_children[slot] = pid;
            g_child_download[slot] = d;
            client_close(next);  // the child owns the connection now
            continue;
        }
        qlog("httpd: fork failed (%s), exporting in-process", strerror(errno));
        g_cur = next;
        route(path);
        if (d) { d->busy = false; atomic_store(&d->state, DL_FAILED); }
        if (!strcmp(method, "HEAD")) keep_headers(&next->out);
    }
}

// New connections while there is room (the rest wait in the listen backlog). With every slot
// taken and someone waiting, the oldest connection that has sent nothing for 3 s makes room.
static void accept_new(uint32_t now) {
    for (;;) {
        Client *c = NULL, *idle = NULL;
        for (int i = 0; i < MAX_CLIENTS; i++) {
            Client *k = &g_cl[i];
            if (k->st == C_FREE) c = c ? c : k;
            else if (k->st == C_READING && !k->reqlen && ms_since(now, k->t0) > 3000 && (!idle || k->t0 < idle->t0))
                idle = k;
        }
        if (!c) {
            struct pollfd pf = {g_listen, POLLIN, 0};
            if (!idle || poll(&pf, 1, 0) <= 0) return;
            client_close(idle);
            c = idle;
        }
        int fd = accept(g_listen, NULL, NULL);
        if (fd < 0) return;
        set_nonblock(fd);
#ifdef SO_NOSIGPIPE
        int one = 1;
        setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &one, sizeof one);
#endif
        *c = (Client){.st = C_READING, .fd = fd, .t0 = now, .seq = ++g_seq};
    }
}

static void client_read(Client *c, uint32_t now) {
    if (ms_since(now, c->t0) > 30000) return client_close(c);  // said nothing (more) for 30 s
    ssize_t r = recv(c->fd, c->req + c->reqlen, sizeof c->req - 1 - c->reqlen, 0);
    if (r == 0 || (r < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)) return client_close(c);
    if (r < 0) return;  // the request has not arrived yet
    c->reqlen += (size_t)r;
    c->req[c->reqlen] = 0;
    c->t0 = now;
    if (strstr(c->req, "\r\n\r\n") || strstr(c->req, "\n\n")) {
        handle_request(c);
    } else if (c->reqlen >= sizeof c->req - 1) {
        g_cur = c;
        respond_text(400, "request too large");
    }
}

static void client_write(Client *c, uint32_t now) {
    if (ms_since(now, c->t0) > 30000) return client_close(c);  // the reader stalled
    size_t left = c->out.len - c->sent;
    size_t chunk = left > 256 * 1024 ? 256 * 1024 : left;
    ssize_t w = chunk ? send(c->fd, c->out.data + c->sent, chunk, MSG_NOSIGNAL) : 0;
    if (w < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR) return client_close(c);
    if (w > 0) {
        c->sent += (size_t)w;
        c->t0 = now;  // progress resets the stall timer
    }
    if (c->sent >= c->out.len) client_close(c);
}

void httpd_poll(void) {
    if (g_listen < 0) return;
    reap_children();
    uint32_t now = now_ms();
    accept_new(now);
    for (int i = 0; i < MAX_CLIENTS; i++) {
        Client *c = &g_cl[i];
        if (c->st == C_READING) client_read(c, now);
        if (c->st == C_WRITING) client_write(c, now);
        if (c->st == C_QUEUED && ms_since(now, c->t0) > 120000) client_close(c);  // waited two minutes: gone
    }
    start_renders();
}

#define MAX_ADDRS 32

#ifdef GRIMOIRE_SIM
// Tests: GRIMOIRE_SIM_ADDRS="wlan0=192.168.0.20,usb0=192.168.15.244" stands in for the machine's
// own interfaces ("" for none), so the Share screen can be laid out with any set of addresses.
static int sim_addrs(const char *spec, NetAddr *out, int max) {
    char buf[512];
    strlcpy_q(buf, spec, sizeof buf);
    int n = 0;
    char *save = NULL;
    for (char *t = strtok_r(buf, ", ", &save); t && n < max; t = strtok_r(NULL, ", ", &save)) {
        char *eq = strchr(t, '=');
        struct in_addr ia;
        if (!eq || eq == t) continue;
        *eq = 0;
        if (inet_pton(AF_INET, eq + 1, &ia) != 1) continue;
        strlcpy_q(out[n].ifname, t, sizeof out[n].ifname);
        strlcpy_q(out[n].ip, eq + 1, sizeof out[n].ip);
        n++;
    }
    return n;
}
#endif

// Every UP, non-loopback IPv4 address, in the order the system lists its interfaces.
static int collect_addrs(NetAddr *out, int max) {
#ifdef GRIMOIRE_SIM
    const char *spec = getenv("GRIMOIRE_SIM_ADDRS");
    if (spec) return sim_addrs(spec, out, max);
#endif
    struct ifaddrs *ifs, *i;
    int n = 0;
    if (getifaddrs(&ifs) != 0) return 0;
    for (i = ifs; i && n < max; i = i->ifa_next) {
        if (!i->ifa_addr || i->ifa_addr->sa_family != AF_INET) continue;
        if (!(i->ifa_flags & IFF_UP) || (i->ifa_flags & IFF_LOOPBACK)) continue;
        struct sockaddr_in *sa = (struct sockaddr_in *)i->ifa_addr;
        if (!inet_ntop(AF_INET, &sa->sin_addr, out[n].ip, sizeof out[n].ip)) continue;
        strlcpy_q(out[n].ifname, i->ifa_name ? i->ifa_name : "", sizeof out[n].ifname);
        // usb0 on a Kindle is the USBNetwork link (192.168.15.x); still useful, keep it.
        n++;
    }
    freeifaddrs(ifs);
    return n;
}

int net_ipv4_addrs(char out[][16], int max) {
    NetAddr all[MAX_ADDRS];
    int n = collect_addrs(all, max < MAX_ADDRS ? max : MAX_ADDRS);
    for (int i = 0; i < n; i++) strlcpy_q(out[i], all[i].ip, 16);
    return n;
}

// Wi-Fi is what a phone can reach (and the only interface the launcher opens the port on).
static int if_rank(const char *name) {
    if (!strncmp(name, "wlan", 4)) return 0;
    if (!strncmp(name, "en", 2) || !strncmp(name, "eth", 3)) return 1;
    if (!strncmp(name, "usb", 3)) return 3;
    return 2;
}

int net_ipv4_list(NetAddr *out, int max) {
    NetAddr all[MAX_ADDRS];
    int n = collect_addrs(all, MAX_ADDRS);  // all of them: the wlan0 one may come last
    for (int i = 1; i < n; i++) {           // stable: the system's order within a rank
        NetAddr t = all[i];
        int j = i;
        for (; j > 0 && if_rank(all[j - 1].ifname) > if_rank(t.ifname); j--) all[j] = all[j - 1];
        all[j] = t;
    }
    if (n > max) n = max < 0 ? 0 : max;
    for (int i = 0; i < n; i++) out[i] = all[i];
    return n;
}
