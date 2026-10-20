// httpd.h: tiny single-threaded HTTP/1.0 share server, polled from the main loop.
// Serves a phone/desktop-friendly HTML index of folders + notebooks with thumbnails and
// download links, in the Kindle's style (Gothic or Classic fonts). Routes (ids are notebook
// ids, n is 1-based):
//   GET /                         HTML index (folders, notebooks, page links)
//   GET /nb/<id>                  HTML notebook page with per-page PNG links
//   GET /nb/<id>/studio           HTML studio: transparent / recolored / filtered exports
//   GET /nb/<id>.pdf              notebook PDF
//   GET /nb/<id>.zip              notebook zip (pdf + pngs + raw)
//   GET /nb/<id>/<n>.png          one page PNG (?layer=ink or ?layer=paper: that layer alone;
//                                 ?f=2..8: shrunk; headers X-Page-Size, X-Paper, X-Ink-Box)
//   GET /nb/<id>/thumb.png        thumbnail
//   GET /font/<name>.ttf          the page fonts (the binary's own copies)
//   GET /folder/<name>.zip        every notebook in a folder
//   GET /all.zip                  everything
//   GET /api/list.json            machine-readable listing (used by tools/grimoire-pull)
// Non-blocking sockets, up to 16 connections at once, each with its own buffer; responses are
// built into a Buf then written in chunks from httpd_poll so the UI stays responsive. Renders
// run in forked children, two at a time; more wait in a queue (oldest first) and are dropped if
// their browser hangs up first.
#pragma once
#include <stdbool.h>

// Binds 0.0.0.0:port; 0 on success. gothic: the pages use the Gothic style's fonts.
int httpd_start(int port, float dpi, bool gothic);
bool httpd_running(void);
int httpd_fd(void);                    // listening fd (for input_watch_fd), -1 if stopped
// Accept / read / write some work; returns quickly. Call every loop iteration while running.
void httpd_poll(void);
void httpd_stop(void);
int httpd_requests_served(void);
// Best-effort list of this device's IPv4 addresses (non-loopback), "a.b.c.d" each.
int net_ipv4_addrs(char out[][16], int max);
// The same addresses with the interface each is on, Wi-Fi first: wlan*, then en* and eth*, then
// the rest, usb* (USBNetwork) last. The launcher opens the share port on wlan0 only.
typedef struct {
    char ip[16];      // "a.b.c.d"
    char ifname[16];  // "wlan0"
} NetAddr;
int net_ipv4_list(NetAddr *out, int max);
