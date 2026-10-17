// httpd.h: tiny single-threaded HTTP/1.0 share server, polled from the main loop.
// Serves a phone/desktop-friendly HTML index of folders + notebooks with thumbnails and
// download links. Routes (ids are notebook ids, n is 1-based):
//   GET /                         HTML index (folders, notebooks, page links)
//   GET /nb/<id>                  HTML notebook page with per-page PNG links
//   GET /nb/<id>.pdf              notebook PDF
//   GET /nb/<id>.zip              notebook zip (pdf + pngs + raw)
//   GET /nb/<id>/<n>.png          one page PNG
//   GET /nb/<id>/thumb.png        thumbnail
//   GET /folder/<name>.zip        every notebook in a folder
//   GET /all.zip                  everything
//   GET /api/list.json            machine-readable listing (used by tools/grimoire-pull)
// Non-blocking sockets; one request at a time; responses are built into a Buf then
// written in chunks from httpd_poll so the UI stays responsive.
#pragma once
#include <stdbool.h>

int httpd_start(int port, float dpi);  // binds 0.0.0.0:port; 0 on success
bool httpd_running(void);
int httpd_fd(void);                    // listening fd (for input_watch_fd), -1 if stopped
// Accept / read / write some work; returns quickly. Call every loop iteration while running.
void httpd_poll(void);
void httpd_stop(void);
int httpd_requests_served(void);
// Best-effort list of this device's IPv4 addresses (non-loopback), "a.b.c.d" each.
int net_ipv4_addrs(char out[][16], int max);
