//! httpd.h: tiny single-threaded HTTP/1.0 share server, polled from the main loop.
//! Serves a phone/desktop-friendly HTML index of folders + notebooks with thumbnails and
//! download links, in the Kindle's style (Gothic or Classic fonts). Routes (ids are notebook
//! ids, n is 1-based):
//!   GET /                         HTML index (folders, notebooks, page links)
//!   GET /nb/<id>                  HTML notebook page with per-page PNG links
//!   GET /nb/<id>/studio           HTML studio: transparent / recolored / filtered exports
//!   GET /nb/<id>.pdf              notebook PDF
//!   GET /nb/<id>.zip              notebook zip (pdf + pngs + raw)
//!   GET /nb/<id>/<n>.png          one page PNG (?layer=ink or ?layer=paper: that layer alone;
//!                                 ?f=2..8: shrunk; headers X-Page-Size, X-Paper, X-Ink-Box)
//!   GET /nb/<id>/thumb.png        thumbnail
//!   GET /font/<name>.ttf          the page fonts (the binary's own copies)
//!   GET /folder/<name>.zip        every notebook in a folder
//!   GET /all.zip                  everything
//!   GET /api/list.json            machine-readable listing (used by tools/grimoire-pull)
//!   GET /live                     Live view (live.h): the code form, or the viewer once paired
//!   GET /live/pair?code=|t=       trades the Kindle's code (or its QR code's token) for a session
//!   GET /live/stream              the screen, for a paired browser (a viewer child answers it)
//! The server runs while an owner keeps it (httpd_acquire): the Share screen, whose routes are every
//! one above but /live, and Live view, whose are /live and the fonts. A route no owner has answers
//! 403 (a download while only Live view runs) or 404.
//! Non-blocking sockets, up to 16 connections at once, each with its own buffer; responses are
//! built into a Buf then written in chunks from httpd_poll so the UI stays responsive. Renders
//! run in forked children, two at a time; more wait in a queue (oldest first) and are dropped if
//! their browser hangs up first.

use crate::prelude::*;

pub const HTTPD_SHARE: u32 = 1;
pub const HTTPD_LIVE: u32 = 2;
/// Keeps the server up for owner (HTTPD_SHARE or HTTPD_LIVE), starting it on 0.0.0.0:port if no
/// owner had it: 0, or -1 when it could not start. gothic: the pages use the Gothic style's fonts.
pub fn httpd_acquire(owner: u32, port: i32, dpi: f32, gothic: bool) -> i32 {
    todo!()
}
/// owner is done with it: the server stops once no owner is left (its downloads and connections
/// with it), and the routes only owner had stop answering at once.
pub fn httpd_release(owner: u32) {
    todo!()
}
pub fn httpd_owners() -> u32 {
    todo!()
}
pub fn httpd_running() -> bool {
    todo!()
}
/// listening fd (for input_watch_fd), -1 if stopped
pub fn httpd_fd() -> i32 {
    todo!()
}
/// Accept / read / write some work; returns quickly. Call every loop iteration while running.
pub fn httpd_poll() {
    todo!()
}
/// Stops it whoever has it (Grimoire leaving).
pub fn httpd_stop() {
    todo!()
}
/// Browsers that paired for Live view since the start (the pairing card closes when it moves on).
pub fn httpd_live_pairings() -> i32 {
    todo!()
}
pub fn httpd_requests_served() -> i32 {
    todo!()
}
/// Best-effort list of this device's IPv4 addresses (non-loopback), "a.b.c.d" each.
pub fn net_ipv4_addrs(out: &mut Vec<String>, max: i32) -> i32 {
    todo!()
}
/// The same addresses with the interface each is on, Wi-Fi first: wlan*, then en* and eth*, then
/// the rest, usb* (USBNetwork) last. The launcher opens the share port on wlan0 only.
#[derive(Clone, Copy, Debug, Default)]
pub struct NetAddr {
    /// "a.b.c.d"
    pub ip: FixedStr<16>,
    /// "wlan0"
    pub ifname: FixedStr<16>,
}
pub fn net_ipv4_list(out: &mut [NetAddr]) -> i32 {
    todo!()
}
