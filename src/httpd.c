// httpd.c: a tiny, single-client HTTP/1.0 server for the share screen.
#include "httpd.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <ifaddrs.h>
#include <net/if.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "export.h"

#ifndef MSG_NOSIGNAL
#define MSG_NOSIGNAL 0
#endif

typedef enum { C_IDLE, C_READING, C_WRITING } ClientState;

static int g_listen = -1;
static float g_dpi = 300.f;
static int g_served;
static struct {
    ClientState st;
    int fd;
    char req[4096];
    size_t reqlen;
    Buf out;
    size_t sent;
    uint32_t t0;
} g_c = {.fd = -1};

static void set_nonblock(int fd) { fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) | O_NONBLOCK); }

int httpd_start(int port, float dpi) {
    httpd_stop();
    signal(SIGPIPE, SIG_IGN);
    g_dpi = dpi;
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    int one = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
    struct sockaddr_in sa = {0};
    sa.sin_family = AF_INET;
    sa.sin_port = htons((uint16_t)port);
    sa.sin_addr.s_addr = htonl(INADDR_ANY);
    if (bind(fd, (struct sockaddr *)&sa, sizeof sa) != 0 || listen(fd, 8) != 0) {
        qlog("httpd: bind/listen on %d failed: %s", port, strerror(errno));
        close(fd);
        return -1;
    }
    set_nonblock(fd);
    g_listen = fd;
    qlog("httpd: listening on port %d", port);
    return 0;
}

bool httpd_running(void) { return g_listen >= 0; }
int httpd_fd(void) { return g_listen; }
int httpd_requests_served(void) { return g_served; }

static void client_close(void) {
    if (g_c.fd >= 0) close(g_c.fd);
    g_c.fd = -1;
    g_c.st = C_IDLE;
    g_c.reqlen = 0;
    g_c.sent = 0;
    buf_free(&g_c.out);
}

void httpd_stop(void) {
    client_close();
    if (g_listen >= 0) close(g_listen);
    g_listen = -1;
}

// ---- helpers ----

static void html_esc(Buf *b, const char *s) {
    for (; *s; s++) {
        switch (*s) {
        case '<': buf_printf(b, "&lt;"); break;
        case '>': buf_printf(b, "&gt;"); break;
        case '&': buf_printf(b, "&amp;"); break;
        case '"': buf_printf(b, "&quot;"); break;
        default: buf_append(b, s, 1);
        }
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

static void json_str(Buf *b, const char *s) {
    buf_append(b, "\"", 1);
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        if (*p == '"' || *p == '\\') buf_printf(b, "\\%c", *p);
        else if (*p < 0x20) buf_printf(b, "\\u%04x", *p);
        else buf_append(b, p, 1);
    }
    buf_append(b, "\"", 1);
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

static void respond(int code, const char *ctype, const Buf *body, const char *download_name) {
    const char *reason = code == 200 ? "OK" : code == 404 ? "Not Found" : code == 405 ? "Method Not Allowed"
                                          : code == 400 ? "Bad Request" : "Internal Server Error";
    Buf *o = &g_c.out;
    buf_printf(o, "HTTP/1.0 %d %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\n"
                  "Cache-Control: no-store\r\n", code, reason, ctype, body ? body->len : 0);
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
    if (body && body->len) buf_append(o, body->data, body->len);
    g_c.st = C_WRITING;
    g_c.sent = 0;
}

static void respond_text(int code, const char *msg) {
    Buf b = {0};
    buf_printf(&b, "%s\n", msg);
    respond(code, "text/plain; charset=utf-8", &b, NULL);
    buf_free(&b);
}

static void page_head(Buf *b, const char *title) {
    buf_printf(b, "<!doctype html><html><head><meta charset=utf-8>"
                  "<meta name=viewport content='width=device-width,initial-scale=1'><title>");
    html_esc(b, title);
    buf_printf(b, "</title><style>"
                  "body{font-family:'Terminess Nerd Font',ui-monospace,Menlo,monospace;margin:24px;color:#111;background:#fff}"
                  "h1{font-size:28px;margin:0 0 4px}h2{font-size:20px;margin:28px 0 10px;border-bottom:2px solid #111}"
                  "a{color:#111}.grid{display:flex;flex-wrap:wrap;gap:18px}"
                  ".card{width:200px;border:2px solid #111;border-radius:10px;padding:10px}"
                  ".card img{width:100%%;border:1px solid #999;background:#fff}"
                  ".name{font-weight:bold;margin:6px 0 2px;word-break:break-word}.meta{color:#555;font-size:13px}"
                  ".links a{margin-right:10px}.pages img{width:280px;border:1px solid #999;margin:0 12px 12px 0}"
                  ".big{display:inline-block;border:2px solid #111;border-radius:8px;padding:8px 14px;text-decoration:none;font-weight:bold}"
                  "</style></head><body>");
}

static void nb_card(Buf *b, const NotebookInfo *nb) {
    buf_printf(b, "<div class=card><a href='/nb/%s'><img loading=lazy src='/nb/%s/thumb.png' alt=''></a>"
                  "<div class=name>", nb->id, nb->id);
    html_esc(b, nb->name);
    char when[32];
    format_time_short(nb->modified, when, sizeof when);
    buf_printf(b, "</div><div class=meta>%d page%s &middot; %s &middot; %s</div>"
                  "<div class=links><a href='/nb/%s.pdf'>PDF</a><a href='/nb/%s.zip'>ZIP</a><a href='/nb/%s'>pages</a></div></div>",
               nb->npages, nb->npages == 1 ? "" : "s", when, paper_name(nb->paper), nb->id, nb->id, nb->id);
}

static void route_index(void) {
    NotebookInfo *nbs;
    int n;
    char (*folders)[STORE_NAME_LEN];
    int nf;
    store_list(&nbs, &n);
    store_list_folders(&folders, &nf);
    Buf b = {0};
    page_head(&b, "Quire");
    buf_printf(&b, "<h1>Quire</h1><p class=meta>%d notebook%s on your Kindle</p>"
                   "<p><a class=big href='/all.zip'>Download everything (zip)</a></p>", n, n == 1 ? "" : "s");
    for (int f = -1; f < nf; f++) {
        const char *folder = f < 0 ? "" : folders[f];
        int count = 0;
        for (int i = 0; i < n; i++) count += !strcmp(nbs[i].folder, folder);
        if (f < 0 && !count) continue;
        buf_printf(&b, "<h2>");
        html_esc(&b, f < 0 ? "Library" : folder);
        if (f >= 0) {
            buf_printf(&b, " <a class=meta href='/folder/");
            url_esc(&b, folder);
            buf_printf(&b, ".zip'>(zip)</a>");
        }
        buf_printf(&b, "</h2><div class=grid>");
        for (int i = 0; i < n; i++)
            if (!strcmp(nbs[i].folder, folder)) nb_card(&b, &nbs[i]);
        if (!count) buf_printf(&b, "<p class=meta>empty</p>");
        buf_printf(&b, "</div>");
    }
    buf_printf(&b, "<p class=meta style='margin-top:40px'>Quire %s</p></body></html>", QUIRE_VERSION);
    respond(200, "text/html; charset=utf-8", &b, NULL);
    buf_free(&b);
    free(nbs);
    free(folders);
}

static void route_notebook_html(const NotebookInfo *nb) {
    Buf b = {0};
    page_head(&b, nb->name);
    buf_printf(&b, "<p><a href='/'>&larr; all notebooks</a></p><h1>");
    html_esc(&b, nb->name);
    buf_printf(&b, "</h1><p><a class=big href='/nb/%s.pdf'>PDF</a> <a class=big href='/nb/%s.zip'>ZIP (PDF + PNG pages + raw)</a></p>"
                   "<div class=pages>", nb->id, nb->id);
    for (int i = 0; i < nb->npages; i++)
        buf_printf(&b, "<a href='/nb/%s/%d.png'><img loading=lazy src='/nb/%s/%d.png' alt='page %d'></a>", nb->id, i + 1,
                   nb->id, i + 1, i + 1);
    buf_printf(&b, "</div></body></html>");
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
    buf_printf(&b, "{\"device\":\"Quire\",\"version\":");
    json_str(&b, QUIRE_VERSION);
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

static void route(char *path) {
    char *q = strchr(path, '?');
    if (q) *q = 0;
    url_decode(path);
    if (strstr(path, "..")) return respond_text(400, "bad path");
    if (!strcmp(path, "/") || !strcmp(path, "/index.html")) return route_index();
    if (!strcmp(path, "/api/list.json")) return route_list_json();
    if (!strcmp(path, "/all.zip")) {
        Buf b = {0};
        int rc = export_folder_zip("", g_dpi, &b);
        if (rc == 0) respond(200, "application/zip", &b, "Quire.zip");
        else respond_text(500, "export failed");
        buf_free(&b);
        return;
    }
    if (!strncmp(path, "/folder/", 8)) {
        char folder[STORE_NAME_LEN];
        strlcpy_q(folder, path + 8, sizeof folder);
        size_t l = strlen(folder);
        if (l < 5 || strcmp(folder + l - 4, ".zip")) return respond_text(404, "not found");
        folder[l - 4] = 0;
        Buf b = {0};
        char fname[STORE_NAME_LEN + 8], safe[STORE_NAME_LEN];
        export_safe_name(folder, safe, sizeof safe);
        snprintf(fname, sizeof fname, "%s.zip", safe);
        if (export_folder_zip(folder, g_dpi, &b) == 0) respond(200, "application/zip", &b, fname);
        else respond_text(500, "export failed");
        buf_free(&b);
        return;
    }
    if (strncmp(path, "/nb/", 4)) return respond_text(404, "not found");
    // /nb/<id>[.pdf|.zip|/<n>.png|/thumb.png]
    char id[STORE_ID_LEN + 16];
    const char *rest = path + 4;
    size_t il = strcspn(rest, "./");
    if (il == 0 || il >= STORE_ID_LEN) return respond_text(404, "not found");
    memcpy(id, rest, il);
    id[il] = 0;
    rest += il;
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb) return respond_text(500, "out of memory");
    if (store_load_info(id, nb) != 0) {
        free(nb);
        return respond_text(404, "no such notebook");
    }
    char safe[STORE_NAME_LEN], fname[STORE_NAME_LEN + 16];
    export_safe_name(nb->name, safe, sizeof safe);
    Buf b = {0};
    int page = 0;
    if (!*rest || !strcmp(rest, "/")) {
        route_notebook_html(nb);
    } else if (!strcmp(rest, ".pdf")) {
        snprintf(fname, sizeof fname, "%s.pdf", safe);
        if (export_notebook_pdf(nb, g_dpi, &b) == 0) respond(200, "application/pdf", &b, fname);
        else respond_text(500, "export failed");
    } else if (!strcmp(rest, ".zip")) {
        ZipWriter z;
        zip_begin(&z, &b);
        int rc = export_notebook_zip(nb, g_dpi, &z, "");
        zip_end(&z);
        snprintf(fname, sizeof fname, "%s.zip", safe);
        if (rc == 0) respond(200, "application/zip", &b, fname);
        else respond_text(500, "export failed");
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
    } else if (sscanf(rest, "/%d.png", &page) == 1 && page >= 1 && page <= nb->npages) {
        snprintf(fname, sizeof fname, "%s-page-%03d.png", safe, page);
        if (export_page_png(nb, page - 1, g_dpi, &b) == 0) respond(200, "image/png", &b, NULL);
        else respond_text(500, "export failed");
    } else {
        respond_text(404, "not found");
    }
    buf_free(&b);
    free(nb);
}

static void handle_request(void) {
    char method[8] = {0}, path[600] = {0};
    if (sscanf(g_c.req, "%7s %599s", method, path) != 2) return respond_text(400, "bad request");
    if (strcmp(method, "GET") && strcmp(method, "HEAD")) return respond_text(405, "GET only");
    qlog("httpd: %s %s", method, path);
    route(path);
    if (!strcmp(method, "HEAD")) {
        // Keep only the headers.
        char *end = memmem(g_c.out.data, g_c.out.len, "\r\n\r\n", 4);
        if (end) g_c.out.len = (size_t)(end - (char *)g_c.out.data) + 4;
    }
    g_served++;
}

void httpd_poll(void) {
    if (g_listen < 0) return;
    if (g_c.st == C_IDLE) {
        int fd = accept(g_listen, NULL, NULL);
        if (fd < 0) return;
        set_nonblock(fd);
#ifdef SO_NOSIGPIPE
        int one = 1;
        setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &one, sizeof one);
#endif
        g_c.fd = fd;
        g_c.st = C_READING;
        g_c.reqlen = 0;
        g_c.t0 = now_ms();
    }
    if (now_ms() - g_c.t0 > 30000) {  // stuck client
        client_close();
        return;
    }
    if (g_c.st == C_READING) {
        ssize_t r = recv(g_c.fd, g_c.req + g_c.reqlen, sizeof g_c.req - 1 - g_c.reqlen, 0);
        if (r == 0 || (r < 0 && errno != EAGAIN && errno != EWOULDBLOCK)) {
            client_close();
            return;
        }
        if (r > 0) {
            g_c.reqlen += (size_t)r;
            g_c.req[g_c.reqlen] = 0;
        }
        if (strstr(g_c.req, "\r\n\r\n") || strstr(g_c.req, "\n\n")) handle_request();
        else if (g_c.reqlen >= sizeof g_c.req - 1) respond_text(400, "request too large");
    }
    if (g_c.st == C_WRITING) {
        size_t left = g_c.out.len - g_c.sent;
        size_t chunk = left > 256 * 1024 ? 256 * 1024 : left;
        ssize_t w = chunk ? send(g_c.fd, g_c.out.data + g_c.sent, chunk, MSG_NOSIGNAL) : 0;
        if (w < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
            client_close();
            return;
        }
        if (w > 0) {
            g_c.sent += (size_t)w;
            g_c.t0 = now_ms();  // progress resets the stuck timer
        }
        if (g_c.sent >= g_c.out.len) client_close();
    }
}

int net_ipv4_addrs(char out[][16], int max) {
    struct ifaddrs *ifs, *i;
    int n = 0;
    if (getifaddrs(&ifs) != 0) return 0;
    for (i = ifs; i && n < max; i = i->ifa_next) {
        if (!i->ifa_addr || i->ifa_addr->sa_family != AF_INET) continue;
        if (!(i->ifa_flags & IFF_UP) || (i->ifa_flags & IFF_LOOPBACK)) continue;
        struct sockaddr_in *sa = (struct sockaddr_in *)i->ifa_addr;
        if (!inet_ntop(AF_INET, &sa->sin_addr, out[n], 16)) continue;
        // usb0 on a Kindle is the USBNetwork link (192.168.15.x); still useful, keep it.
        n++;
    }
    freeifaddrs(ifs);
    return n;
}
