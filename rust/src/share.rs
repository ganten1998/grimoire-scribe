//! share.c: "Share over Wi-Fi". Keeps the HTTP server up (for its routes) only while this screen is
//! open; turning the Kindle or waking it only lays the screen out again, and the server (with any
//! download) carries on. A phone scans the QR code of the Wi-Fi address; a computer types the
//! address under it.
//!
//! The same screen is Live view's pairing card (share_open_live, live.h): the address of /live, a
//! QR code that pairs a phone at once, and a one-use code a computer types. While it is up nothing
//! is streamed (the code must not show on a stream), and it closes by itself once a browser pairs.

use crate::prelude::*;

const H_DONE: i32 = 1;
const H_RETRY: i32 = 2;
const H_STOP: i32 = 3;
const H_LIVE: i32 = 4;
/// addresses listed: the first in big type, each other one on a small line
const MAX_SHOWN: usize = 3;
/// the code or the icon, the text, the counter, Done, Retry
const MAX_PARTS: usize = 6;

/// A part of the screen and where draw() put it, for check_layout.
#[derive(Clone, Copy)]
struct Part {
    what: &'static str,
    r: Rect,
}

/// The screen's state (share.c's, which app.h keeps opaque): share_init makes it, in a.share.
#[derive(Default)]
pub struct ShareState {
    ret: ScreenId,
    hits: HitList,
    counter: Rect,
    last_count: i32,
    last_paint: u32,
    started: bool,
    parts: Vec<Part>,
    /// Live view's pairing card
    live: bool,
    /// ... the code it shows (pair.h), to see when it changes
    code: FixedStr<8>,
    /// ... and when it was made
    code_at: u32,
    /// httpd_live_pairings() as it opened
    pairings: i32,
    /// live_viewers() as last drawn
    viewers: i32,
}

/// The screen's state, which share_init has made (the C's `S`, a->share).
fn s(a: &mut App) -> &mut ShareState {
    a.share.as_deref_mut().unwrap()
}

// The help under the address, a line each, and last the command line of a script that pulls
// everything (k_pull).
const K_HELP: [&str; 3] = [
    // TRANSLATORS: the Share screen's help, under the address: a line each, 1578 px at 34 px
    // sideways (beside the QR code), 1640 upright.
    n_("Pages as PNG, notebooks as PDF or zip, folders and everything as zip."),
    n_("Effects: transparent, recolored or filtered copies of any page."),
    n_("Same Wi-Fi network only. Keep this screen open while downloading."),
];
const K_PULL: &str = "tools/grimoire-pull --http";
const N_HELP: usize = K_HELP.len() + 1;

// The screen checks its own layout as it draws, and logs a "share: layout:" line for anything that
// does not fit (tests/share_check.sh fails on any): a line too wide for its column (which
// font_draw_centered would cut short), or a part of the screen outside the card or over another.

// One line of text centered in r. Returns the bottom of r. (On the Kindle the log leaves the
// line's text out: it may be the address, and grimoire.log goes to public bug reports.)
fn draw_line(c: &mut Canvas, f: FontFace, px: f32, r: Rect, s: &str, gray: u8) -> i32 {
    let w = font_measure(f, px, s);
    #[cfg(feature = "sim")]
    if w > r.w {
        qlog!("share: layout: '%s' is cut to fit (%d px wide, room for %d)", s, w, r.w);
    }
    #[cfg(not(feature = "sim"))]
    if w > r.w {
        qlog!("share: layout: a line is cut to fit (%d px wide, room for %d)", w, r.w);
    }
    font_draw_centered(c, f, px, r, s, gray);
    r.y + r.h
}

fn placed(a: &mut App, what: &'static str, r: Rect) {
    let parts = &mut s(a).parts;
    if parts.len() < MAX_PARTS {
        parts.push(Part { what, r });
    }
}

fn check_layout(a: &mut App, in_: Rect) {
    let parts = s(a).parts.clone();
    for i in 0..parts.len() {
        let p = &parts[i];
        if !rect_contains_rect(in_, p.r) {
            qlog!(
                "share: layout: %s (%d,%d %dx%d) runs out of the card (%d,%d %dx%d)",
                p.what,
                p.r.x,
                p.r.y,
                p.r.w,
                p.r.h,
                in_.x,
                in_.y,
                in_.w,
                in_.h
            );
        }
        for q in &parts[i + 1..] {
            if rect_overlaps(p.r, q.r) {
                qlog!(
                    "share: layout: %s (%d,%d %dx%d) runs into %s (%d,%d %dx%d)",
                    p.what,
                    p.r.x,
                    p.r.y,
                    p.r.w,
                    p.r.h,
                    q.what,
                    q.r.x,
                    q.r.y,
                    q.r.w,
                    q.r.h
                );
            }
        }
    }
}

fn draw_counter(a: &mut App) {
    let mut text = String::new();
    let (live, viewers, counter) = {
        let st = s(a);
        (st.live, st.viewers, st.counter)
    };
    if !live {
        // TRANSLATORS: how many downloads the Share screen has answered since it opened.
        csnprintf!(&mut text, 96, tr("Downloads served: %d"), httpd_requests_served());
    } else {
        // TRANSLATORS: Live view's pairing card: how many computers watch the screen now.
        csnprintf!(&mut text, 96, ng_("%d computer watching", "%d computers watching", viewers as i64), viewers);
    }
    canvas_fill(&mut a.screen, counter, 255);
    draw_line(&mut a.screen, FontFace::Regular, ui_text_size(), counter, &text, 0);
}

// Whether a phone can reach this address: the launcher opens the share port on wlan0 only. In the
// simulator the computer's own Wi-Fi or Ethernet (en*) stands in for it.
fn phone_reachable(ifname: &str) -> bool {
    #[cfg(feature = "sim")]
    if ifname.starts_with("en") {
        return true;
    }
    ifname.starts_with("wlan")
}

fn url_of(a: &mut App, ad: &NetAddr, out: &mut String, cap: usize) {
    let live = s(a).live;
    csnprintf!(out, cap, "http://%s:%d%s", &ad.ip, a.settings.http_port, if live { "/live" } else { "" });
}

// The code as shown: two groups of three digits ("482 913").
fn code_shown(code: &str, out: &mut String, cap: usize) {
    if code.len() == 6 {
        csnprintf!(out, cap, "%.3s %.3s", code, &code[3..]);
    } else {
        csnprintf!(out, cap, "%s", code);
    }
}

// Live view's help, under its code.
const K_LIVE_HELP: [&str; 2] = [
    // TRANSLATORS: Live view's pairing card, under the code: a line each, 1578 px at 34 px
    // sideways (beside the QR code), 1640 upright.
    n_("Same Wi-Fi network only. The code works once."),
    n_("The computer sees what this screen shows, not the code."),
];
const N_LIVE_HELP: usize = K_LIVE_HELP.len();

// How many addresses the text lists: in Live view, with its code, room for one "Also:" line.
fn listed(a: &mut App, n: i32) -> i32 {
    if s(a).live && n > 2 {
        2
    } else {
        n
    }
}

// The text block for n addresses: caption, the first address, a line per other one, the help (in
// Live view: the code and its help in place of the help).
fn text_height(a: &mut App, n: i32) -> i32 {
    let listed = listed(a, n);
    if s(a).live {
        return ui_px(110.0)
            + ui_px(150.0)
            + (listed - 1) * ui_px(70.0)
            + ui_px(30.0)
            + ui_px(80.0)
            + ui_px(170.0)
            + N_LIVE_HELP as i32 * ui_px(70.0);
    }
    ui_px(110.0) + ui_px(150.0) + (n - 1) * ui_px(70.0) + ui_px(30.0) + N_HELP as i32 * ui_px(70.0)
}

// The text block from y down, centered in the column [x, x + w): the caption, url (the first
// address, the very string in the code) in big type, a line per other address, the help. Returns
// the bottom of its last line.
fn draw_text(a: &mut App, x: i32, w: i32, y: i32, url: &str, ad: &[NetAddr], n: i32, qr: bool) -> i32 {
    let mut y = y;
    let mut other = String::new();
    let mut text = String::new();
    // TRANSLATORS: over the Share screen's address (1640 px at 44 px upright, 1578 sideways beside
    // the QR code), with its QR code, or without one (no Wi-Fi address).
    let mut caption = tr("Scan with your phone, or open this in a browser:");
    if !qr {
        caption = tr("On your computer, open this in a browser:");
    }
    draw_line(&mut a.screen, FontFace::Regular, ui_text_size(), Rect { x, y, w, h: ui_px(80.0) }, caption, 0);
    y += ui_px(110.0);
    draw_line(&mut a.screen, FontFace::Bold, ui_title_size() * 1.5f32, Rect { x, y, w, h: ui_px(140.0) }, url, 0);
    y += ui_px(150.0);
    for i in 1..listed(a, n) {
        url_of(a, &ad[i as usize], &mut other, 64);
        // TRANSLATORS: another address the Kindle answers at, and its network interface ("usb0").
        csnprintf!(&mut text, 160, tr("Also: %s (%s)"), &other, &ad[i as usize].ifname);
        draw_line(&mut a.screen, FontFace::Regular, ui_small_size(), Rect { x, y, w, h: ui_px(64.0) }, &text, 0);
        y += ui_px(70.0);
    }
    y += ui_px(30.0);
    let mut end = y;
    if s(a).live {
        let mut shown = String::new();
        draw_line(
            &mut a.screen,
            FontFace::Regular,
            ui_text_size(),
            Rect { x, y, w, h: ui_px(80.0) },
            // TRANSLATORS: Live view's pairing card, over the code a computer types (1640 px at 44 px
            // upright, 1578 sideways).
            tr("Then type this code:"),
            0,
        );
        y += ui_px(80.0);
        code_shown(&pair_code(), &mut shown, 16);
        end = draw_line(&mut a.screen, FontFace::Bold, ui_title_size() * 1.8f32, Rect { x, y, w, h: ui_px(160.0) }, &shown, 0);
        y += ui_px(170.0);
        for i in 0..N_LIVE_HELP {
            end = draw_line(
                &mut a.screen,
                FontFace::Regular,
                ui_small_size(),
                Rect { x, y, w, h: ui_px(64.0) },
                tr(K_LIVE_HELP[i]),
                60,
            );
            y += ui_px(70.0);
        }
        return end;
    }
    for i in 0..N_HELP {
        if i == N_HELP - 1 {
            // TRANSLATORS: the last help line, with the command a computer pulls everything with.
            csnprintf!(&mut text, 160, tr("From a terminal: %s <address above>"), K_PULL);
        }
        end = draw_line(
            &mut a.screen,
            FontFace::Regular,
            ui_small_size(),
            Rect { x, y, w, h: ui_px(64.0) },
            if i < N_HELP - 1 { tr(K_HELP[i]) } else { &text },
            60,
        );
        y += ui_px(70.0);
    }
    end
}

// The QR code of the first address when a phone can reach it (the Wi-Fi icon when none can, say
// over USBNetwork only), then the text. Portrait stacks them above the counter; landscape puts
// the code on the left and the text, counter and buttons on the right.
fn draw_addresses(a: &mut App, in_: Rect, ad: &[NetAddr], n: i32) {
    let mut url = String::new();
    let mut qr = String::new();
    url_of(a, &ad[0], &mut url, 64);
    // Live view's code pairs the phone that scans it: the address with the invitation's token.
    if s(a).live {
        csnprintf!(&mut qr, 128, "%s/pair?t=%s", &url, &pair_qr_token());
    } else {
        csnprintf!(&mut qr, 128, "%s", &url);
    }
    let qn = if phone_reachable(ad[0].ifname.as_str()) { ui_qr_modules(&qr) } else { 0 }; // modules across, 0: no code
    // grimoire.log goes to public bug reports (kindle/README.txt asks for it): it names the
    // interface and the port, never the address. Only the simulator logs the address too, for
    // tests/share_check.sh, which reads the code back from a screenshot and compares.
    if qn != 0 {
        qlog!("share: QR code for the %s address, port %d", &ad[0].ifname, a.settings.http_port);
    } else {
        qlog!(
            "share: no Wi-Fi address, so no QR code; showing the %s address, port %d",
            &ad[0].ifname,
            a.settings.http_port
        );
    }
    #[cfg(feature = "sim")]
    {
        qlog!(
            "share: (simulator) %s %s (%s)",
            if qn != 0 { "the code holds" } else { "the address shown is" },
            if qn != 0 { &qr } else { &url },
            &ad[0].ifname
        );
        if s(a).live {
            qlog!("live: (simulator) the code is %s", &pair_code());
        }
    }
    // The quiet zone is part of the code's box: with Invert on it shows as a light square, so the
    // box keeps clear of the title, the frame and the caption.
    let counter = s(a).counter;
    let top = in_.y + ui_px(20.0);
    let gap = ui_px(30.0);
    let text_h = text_height(a, n);
    let bottom = counter.y - ui_px(30.0);
    let negative = a.settings.invert; // inverted on the panel: dark modules on light
    if a.w <= a.h {
        let mut y = in_.y + ui_px(360.0);
        if qn != 0 {
            let mod_ = clampi((bottom - top - gap - text_h) / qn, ui_px(10.0), ui_px(16.0));
            let sym = ui_qr(&mut a.screen, Rect { x: in_.x, y: top, w: in_.w, h: qn * mod_ }, &qr, negative);
            placed(a, "the code", sym);
            if !rect_is_empty(sym) {
                y = sym.y + sym.h + gap;
            }
        } else {
            let icon = Rect { x: in_.x + in_.w / 2 - ui_px(130.0), y: in_.y + ui_px(40.0), w: ui_px(260.0), h: ui_px(260.0) };
            ui_icon(&mut a.screen, IconKind::Share, icon, 0);
            placed(a, "the icon", icon);
        }
        let end = draw_text(a, in_.x, in_.w, y, &url, ad, n, qn > 0);
        placed(a, "the text", Rect { x: in_.x, y, w: in_.w, h: end - y });
        return;
    }
    let lx = in_.x + ui_px(30.0);
    let mod_ = if qn != 0 { clampi((bottom - top) / qn, ui_px(10.0), ui_px(16.0)) } else { 0 };
    let left = if qn != 0 { qn * mod_ } else { ui_px(260.0) };
    let col = lx + left + ui_px(60.0);
    let col_w = in_.x + in_.w - col;
    if qn != 0 {
        let sym = ui_qr(&mut a.screen, Rect { x: lx, y: top, w: left, h: left }, &qr, negative);
        placed(a, "the code", sym);
    } else {
        let icon = Rect { x: lx, y: in_.y + ui_px(40.0), w: left, h: left };
        ui_icon(&mut a.screen, IconKind::Share, icon, 0);
        placed(a, "the icon", icon);
    }
    let st = s(a);
    st.counter = Rect { x: col, y: st.counter.y, w: col_w, h: st.counter.h };
    let end = draw_text(a, col, col_w, top, &url, ad, n, qn > 0);
    placed(a, "the text", Rect { x: col, y: top, w: col_w, h: end - top });
}

fn draw(a: &mut App) {
    hits_clear(&mut s(a).hits);
    s(a).parts.clear();
    let (w, h) = (a.w, a.h);
    canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w, h }, 255);
    let m = ui_px(70.0);
    let card = Rect { x: m, y: ui_px(260.0), w: w - 2 * m, h: h - ui_px(520.0) };
    let (live, started) = {
        let st = s(a);
        (st.live, st.started)
    };
    // TRANSLATORS: the Share screen's title.
    let in_ = ui_sheet(&mut a.screen, card, if live { tr("Live view") } else { tr("Share over Wi-Fi") });
    s(a).counter = Rect { x: in_.x, y: in_.y + in_.h - ui_px(330.0), w: in_.w, h: ui_px(80.0) };
    let mut ad = [NetAddr::default(); MAX_SHOWN];
    let n = if started { net_ipv4_list(&mut ad) } else { 0 };
    if n > 0 {
        draw_addresses(a, in_, &ad, n);
    } else {
        let mut y = in_.y + ui_px(40.0);
        let icon = Rect { x: in_.x + in_.w / 2 - ui_px(130.0), y, w: ui_px(260.0), h: ui_px(260.0) };
        ui_icon(&mut a.screen, IconKind::Share, icon, 0);
        placed(a, "the icon", icon);
        y += ui_px(320.0);
        let y0 = y;
        // TRANSLATORS: the Share screen without an address (1640 px at 64 px), and what to do (at
        // 44 px); Retry is the button's name.
        let what = if started { tr("Turn on Wi-Fi on the Kindle first") } else { tr("Could not start the server") };
        let then = if started { tr("Then tap Retry.") } else { tr("Is another app using the port? Try again.") };
        draw_line(&mut a.screen, FontFace::Bold, ui_title_size(), Rect { x: in_.x, y, w: in_.w, h: ui_px(100.0) }, what, 0);
        y += ui_px(110.0);
        let end = draw_line(&mut a.screen, FontFace::Regular, ui_text_size(), Rect { x: in_.x, y, w: in_.w, h: ui_px(80.0) }, then, 60);
        placed(a, "the text", Rect { x: in_.x, y: y0, w: in_.w, h: end - y0 });
    }
    if started {
        draw_counter(a);
        let counter = s(a).counter;
        placed(a, "the counter", counter);
    }
    let bw = ui_px(420.0);
    let bh = ui_px(150.0);
    let done = Rect { x: in_.x + in_.w - bw, y: in_.y + in_.h - bh, w: bw, h: bh };
    ui_button(&mut a.screen, done, IconKind::Check, tr("Done"), ButtonStyle::Primary);
    hits_add(&mut s(a).hits, done, H_DONE, 0);
    placed(a, "Done", done);
    if live {
        // Stop, beside Done (Retry, when there is no address, goes between them).
        let stop = Rect { x: in_.x, y: in_.y + in_.h - bh, w: bw, h: bh };
        // TRANSLATORS: Live view's pairing card: turns Live view off (388 px at 44 px, or 34 px).
        ui_button(&mut a.screen, stop, IconKind::Close, tr("Stop"), ButtonStyle::Plain);
        hits_add(&mut s(a).hits, stop, H_STOP, 0);
        placed(a, "Stop", stop);
    }
    if n == 0 && live {
        let retry = Rect { x: in_.x + (in_.w - bw) / 2, y: in_.y + in_.h - bh, w: bw, h: bh };
        ui_button(&mut a.screen, retry, IconKind::None, tr("Retry"), ButtonStyle::Plain);
        hits_add(&mut s(a).hits, retry, H_RETRY, 0);
        placed(a, "Retry", retry);
    } else if n > 0 && !live {
        // Live view starts here: it uses the same server, on the same Wi-Fi.
        let live_btn = Rect { x: in_.x, y: in_.y + in_.h - bh, w: bw, h: bh };
        // TRANSLATORS: the Share screen's button that turns Live view on (or shows its card while it
        // is on): 388 px at 44 px, or 34 px under its icon.
        ui_button(&mut a.screen, live_btn, IconKind::Live, tr("Live view"), ButtonStyle::Plain);
        hits_add(&mut s(a).hits, live_btn, H_LIVE, 0);
        placed(a, "Live view", live_btn);
    } else if n == 0 {
        let retry = Rect { x: in_.x, y: in_.y + in_.h - bh, w: bw, h: bh };
        // TRANSLATORS: starts the share server again (388 px at 44 px, or 34 px).
        ui_button(&mut a.screen, retry, IconKind::None, tr("Retry"), ButtonStyle::Plain);
        hits_add(&mut s(a).hits, retry, H_RETRY, 0);
        placed(a, "Retry", retry);
    }
    check_layout(a, in_);
}

fn start(a: &mut App) {
    let gothic = ui_theme() == UiTheme::Gothic;
    let (port, dpi) = (a.settings.http_port, a.dpi);
    let started = if s(a).live {
        httpd_acquire(HTTPD_LIVE, port, dpi, gothic) == 0
    } else {
        httpd_acquire(HTTPD_SHARE, port, dpi, gothic) == 0
    };
    s(a).started = started;
    app_watch_server(a);
    s(a).last_count = httpd_requests_served();
}

// A fresh invitation for the card (pair.h): at the open, when the one shown is used up or about
// to run out, and after five wrong codes (pair_redeem made a new one).
fn invite(a: &mut App) {
    pair_begin(PAIR_LIVE, (now_ms() / 1000) as i64);
    let st = s(a);
    st.code.set(&pair_code());
    st.code_at = now_ms();
}

/// switch to SCR_SHARE (returns to prev_scr)
pub fn share_open(a: &mut App) {
    let scr = a.scr;
    let st = s(a);
    st.ret = scr;
    st.live = false;
    app_switch(a, ScreenId::Share);
}

/// the same, as Live view's pairing card
pub fn share_open_live(a: &mut App) {
    s(a).pairings = httpd_live_pairings();
    s(a).viewers = live_viewers();
    invite(a);
    if a.scr != ScreenId::Share {
        let scr = a.scr;
        let st = s(a);
        st.ret = scr;
        st.live = true;
        app_switch(a, ScreenId::Share);
        return;
    }
    // From Share's own button: the same screen as the card, drawn again in place (a leave and an
    // enter would take it for the card closing). Share's downloads stop answering with it.
    if !s(a).live {
        httpd_release(HTTPD_SHARE);
    }
    s(a).live = true;
    share_enter(a);
    app_invalidate_all(a, RefreshMode::Ui);
}

/// Live view's pairing card is the screen
pub fn share_live_card_up(a: &App) -> bool {
    a.scr == ScreenId::Share && a.share.as_ref().is_some_and(|st| st.live)
}

pub fn share_init(a: &mut App) {
    a.share = Some(Box::default());
}
pub fn share_free(a: &mut App) {
    httpd_stop();
    a.share = None;
}

pub fn share_enter(a: &mut App) {
    if a.relayout && httpd_running() {
        qlog!("share: laid out again (turned or woken); the server carries on");
    }
    if s(a).live {
        live_set_state(LIVE_PAIRING); // (before the code is drawn: it is never streamed)
    }
    start(a); // a server kept across a relayout goes on as it is
    draw(a);
}

// Asleep on this screen nothing accepts a connection (app_tick runs no screen's tick while
// asleep), so one waiting at the server would wake input_poll at once, again and again, and the
// main loop would spin at full speed until the wake. It waits in the server's queue instead, and
// the wake's relayout watches the server again (start, from share_enter).
/// asleep on Share: the server waits unwatched
pub fn share_sleep(a: &mut App) {
    let _ = a;
    input_watch_fd(-1);
}

// The sleep screen while this screen is up and no page is shown in its place: the card without
// the code and the addresses, since nothing answers while the Kindle sleeps (share_sleep), and a
// sleeping Kindle need not show its address. The wake lays the screen out again (share_enter).
/// Share's sleep screen when no page is shown
pub fn share_draw_asleep(a: &mut App) {
    let (w, h) = (a.w, a.h);
    let live = s(a).live;
    canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w, h }, 255);
    let m = ui_px(70.0);
    let card = Rect { x: m, y: ui_px(260.0), w: w - 2 * m, h: h - ui_px(520.0) };
    let in_ = ui_sheet(&mut a.screen, card, if live { tr("Live view") } else { tr("Share over Wi-Fi") });
    ui_icon(
        &mut a.screen,
        if live { IconKind::Live } else { IconKind::Share },
        Rect { x: in_.x + in_.w / 2 - ui_px(130.0), y: in_.y + ui_px(40.0), w: ui_px(260.0), h: ui_px(260.0) },
        0,
    );
    let y = in_.y + ui_px(360.0);
    // TRANSLATORS: the Share screen as the Kindle's sleep screen.
    draw_line(&mut a.screen, FontFace::Bold, ui_title_size(), Rect { x: in_.x, y, w: in_.w, h: ui_px(100.0) }, tr("Asleep"), 0);
    if live {
        return;
    }
    draw_line(
        &mut a.screen,
        FontFace::Regular,
        ui_text_size(),
        Rect { x: in_.x, y: y + ui_px(110.0), w: in_.w, h: ui_px(80.0) },
        tr("Wake the Kindle to share again."),
        60,
    );
}

pub fn share_leave(a: &mut App) {
    // Turning the Kindle and waking it lay this screen out again: keep the server (and its
    // downloads, connections and queue) for the enter that follows.
    if a.relayout {
        return;
    }
    if s(a).live {
        // The card goes: its code with it (a new one each time it opens), and the stream goes on.
        pair_forget_invitation();
        if live_on() {
            live_set_state(LIVE_AWAKE);
        }
        s(a).live = false;
        app_watch_server(a);
        return;
    }
    httpd_release(HTTPD_SHARE);
    app_watch_server(a);
}

pub fn share_event(a: &mut App, ev: &InEvent) {
    httpd_poll();
    if app_pointer_phase(ev) != 2 {
        return;
    }
    let Some(h) = hits_find(&s(a).hits, ev.x, ev.y).copied() else { return };
    let back = {
        let ret = s(a).ret;
        if ret == ScreenId::Share {
            ScreenId::Library
        } else {
            ret
        }
    };
    if h.id == H_DONE {
        app_switch(a, back);
    } else if h.id == H_LIVE {
        live_view_open(a);
    } else if h.id == H_STOP {
        app_switch(a, back);
        live_view_stop(a, true);
    } else if h.id == H_RETRY {
        start(a);
        draw(a);
        app_invalidate_all(a, RefreshMode::Ui);
    }
}

pub fn share_tick(a: &mut App, now: u32) {
    httpd_poll();
    if s(a).live {
        if httpd_live_pairings() != s(a).pairings {
            // A browser paired: the card's work is done (app.c says who is watching once its stream
            // starts).
            let ret = s(a).ret;
            app_switch(a, if ret == ScreenId::Share { ScreenId::Library } else { ret });
            return;
        }
        // A new code once five wrong ones used this one up, or a while before it runs out.
        let stale = now.wrapping_sub(s(a).code_at) >= ((PAIR_LIFETIME_S - 20) * 1000) as u32;
        if stale {
            invite(a);
        }
        if stale || s(a).code.as_str() != pair_code() {
            s(a).code.set(&pair_code());
            draw(a);
            app_invalidate_all(a, RefreshMode::Ui);
            return;
        }
        let v = live_viewers();
        if v != s(a).viewers {
            s(a).viewers = v;
            draw_counter(a);
            let counter = s(a).counter;
            app_invalidate(a, counter, RefreshMode::Ui);
        }
        return;
    }
    let n = httpd_requests_served();
    if n != s(a).last_count && now.wrapping_sub(s(a).last_paint) > 2000 {
        let st = s(a);
        st.last_count = n;
        st.last_paint = now;
        draw_counter(a);
        let counter = s(a).counter;
        app_invalidate(a, counter, RefreshMode::Ui);
    }
}
