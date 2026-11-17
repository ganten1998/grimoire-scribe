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
const H_WIFI: i32 = 5;
const H_WIFI_ON: i32 = 6;
const H_WIFI_OFF: i32 = 7;
const H_WIFI_CHECK: i32 = 8;
const H_WIFI_DONE: i32 = 9;
/// the question a PDF sent from the Share page asks (issue #175): its two rows
const H_ASK_IMPORT: i32 = 120;
const H_ASK_CANCEL: i32 = 121;
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
    wifi_open: bool,
    wifi_paint: Option<(Option<WifiStatus>, bool, bool)>,
    wifi_check_at: u32,
    wifi_watch_at: u32,
    address_check_at: u32,
    address_key: String,
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
    /// the question a PDF sent from the Share page asks, while the server waits for its answer (issue #175)
    ask: Option<Ask>,
}

/// "Import "name" (size)?" over the Share screen, the Library's question sheet with the send's code under its title,
/// then Import and Cancel: the server's number for it, what it shows, where it was drawn and its rows. A contact
/// answers it only once it has been shown (app_ask, app_asked_seen), and only a tap that comes down and lifts on one
/// row.
#[derive(Default)]
struct Ask {
    serial: u32,
    name: String,
    size: u64,
    /// the send's code, which the page that sent it shows too (never logged or stored; gone with the question)
    code: String,
    rect: Rect,
    hits: HitList,
    /// the contact under way came down before the question was shown, or off its rows: its lift answers nothing
    swallow: bool,
    /// the row the contact under way came down on
    down: Option<i32>,
}

/// The screen's state, which share_init has made (the C's `S`, a->share).
fn s(a: &mut App) -> &mut ShareState {
    a.share.as_deref_mut().unwrap()
}

// The screen checks its own layout as it draws, and logs a "share: layout:" line for anything that
// does not fit (tests/share_check.sh fails on any): a line too wide for its column (which
// font_draw_centered would cut short), or a part of the screen outside the card or over another.

// One line of text centered in r. Returns the bottom of r. (On the Kindle the log leaves the
// line's text out: it may be the address, and grimoire.log goes to public bug reports.)
fn draw_line(c: &mut Canvas, f: FontFace, px: f32, r: Rect, s: &str, gray: u8) -> i32 {
    let w = font_measure(f, px, s);
    #[cfg(feature = "sim")]
    if w > r.w {
        qlog!(
            "share: layout: '%s' is cut to fit (%d px wide, room for %d)",
            s,
            w,
            r.w
        );
    }
    #[cfg(not(feature = "sim"))]
    if w > r.w {
        qlog!(
            "share: layout: a line is cut to fit (%d px wide, room for %d)",
            w,
            r.w
        );
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
        csnprintf!(
            &mut text,
            96,
            tr("Downloads served: %d"),
            httpd_requests_served()
        );
    } else {
        // TRANSLATORS: Live view's pairing card: how many computers watch the screen now.
        csnprintf!(
            &mut text,
            96,
            ng_(
                "%d computer watching",
                "%d computers watching",
                viewers as i64
            ),
            viewers
        );
    }
    canvas_fill(&mut a.screen, counter, 255);
    draw_line(
        &mut a.screen,
        FontFace::Regular,
        ui_text_size(),
        counter,
        &text,
        0,
    );
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
    csnprintf!(
        out,
        cap,
        "http://%s:%d%s",
        &ad.ip,
        a.settings.http_port,
        if live { "/live" } else { "" }
    );
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
    ui_px(110.0) + ui_px(150.0) + (n - 1) * ui_px(70.0) + ui_px(30.0)
}

// The text block from y down, centered in the column [x, x + w): the caption, url (the first
// address, the very string in the code) in big type, a line per other address, the help. Returns
// the bottom of its last line.
fn draw_text(
    a: &mut App,
    x: i32,
    w: i32,
    y: i32,
    url: &str,
    ad: &[NetAddr],
    n: i32,
    qr: bool,
) -> i32 {
    let mut y = y;
    let mut other = String::new();
    let mut text = String::new();
    // TRANSLATORS: over the Share screen's address (1640 px at 44 px upright, 1578 sideways beside
    // the QR code), with its QR code, or without one (no Wi-Fi address).
    let mut caption = tr("Scan with your phone, or open this in a browser:");
    if !qr {
        caption = tr("On your computer, open this in a browser:");
    }
    draw_line(
        &mut a.screen,
        FontFace::Regular,
        ui_text_size(),
        Rect {
            x,
            y,
            w,
            h: ui_px(80.0),
        },
        caption,
        0,
    );
    y += ui_px(110.0);
    draw_line(
        &mut a.screen,
        FontFace::Bold,
        ui_title_size() * 1.5f32,
        Rect {
            x,
            y,
            w,
            h: ui_px(140.0),
        },
        url,
        0,
    );
    y += ui_px(150.0);
    for i in 1..listed(a, n) {
        url_of(a, &ad[i as usize], &mut other, 64);
        // TRANSLATORS: another address the Kindle answers at, and its network interface ("usb0").
        csnprintf!(
            &mut text,
            160,
            tr("Also: %s (%s)"),
            &other,
            &ad[i as usize].ifname
        );
        draw_line(
            &mut a.screen,
            FontFace::Regular,
            ui_small_size(),
            Rect {
                x,
                y,
                w,
                h: ui_px(64.0),
            },
            &text,
            0,
        );
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
            Rect {
                x,
                y,
                w,
                h: ui_px(80.0),
            },
            // TRANSLATORS: Live view's pairing card, over the code a computer types (1640 px at 44 px
            // upright, 1578 sideways).
            tr("Then type this code:"),
            0,
        );
        y += ui_px(80.0);
        code_shown(&pair_code(), &mut shown, 16);
        end = draw_line(
            &mut a.screen,
            FontFace::Bold,
            ui_title_size() * 1.8f32,
            Rect {
                x,
                y,
                w,
                h: ui_px(160.0),
            },
            &shown,
            0,
        );
        y += ui_px(170.0);
        for i in 0..N_LIVE_HELP {
            end = draw_line(
                &mut a.screen,
                FontFace::Regular,
                ui_small_size(),
                Rect {
                    x,
                    y,
                    w,
                    h: ui_px(64.0),
                },
                tr(K_LIVE_HELP[i]),
                60,
            );
            y += ui_px(70.0);
        }
        return end;
    }
    end
}

/// The box side that draws `data` dark modules at the same size on Share and in Live view, even
/// though their addresses encode as different QR versions. The quiet zone is included in the box.
fn qr_side(across: i32, data: i32, target: i32, room: i32) -> i32 {
    if across <= 0 || data <= 0 {
        return 0;
    }
    let m = ((target + data / 2) / data).max(1).min(room / across);
    if m < 1 {
        0
    } else {
        across * m
    }
}

// The QR code of the first address when a phone can reach it (the Wi-Fi icon when none can, say
// over USBNetwork only), then the text. Portrait stacks them above the counter; landscape puts
// the code on the left and the text and counter on the right.
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
    let qn = if phone_reachable(ad[0].ifname.as_str()) {
        ui_qr_modules(&qr)
    } else {
        0
    }; // modules across, 0: no code
    let qn_data = if qn != 0 {
        crate::qr::ui_qr_data_modules(&qr)
    } else {
        0
    };
    // The dark part of the code is this size on both Share and Live view; the quiet zone around it
    // makes the drawing box a little larger for the shorter address.
    let qr_target = ui_px(666.0);
       // grimoire.log goes to public bug reports (kindle/README.txt asks for it): it names the
       // interface and the port, never the address. Only the simulator logs the address too, for
       // tests/share_check.sh, which reads the code back from a screenshot and compares.
    if qn != 0 {
        qlog!(
            "share: QR code for the %s address, port %d",
            &ad[0].ifname,
            a.settings.http_port
        );
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
            if qn != 0 {
                "the code holds"
            } else {
                "the address shown is"
            },
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
            let room = (bottom - top - gap - text_h).min(in_.w);
            let side = qr_side(qn, qn_data, qr_target, room);
            let sym = ui_qr(
                &mut a.screen,
                Rect {
                    x: in_.x,
                    y: top,
                    w: in_.w,
                    h: side,
                },
                &qr,
                negative,
            );
            placed(a, "the code", sym);
            if !rect_is_empty(sym) {
                y = sym.y + sym.h + gap;
            }
        } else {
            let icon = Rect {
                x: in_.x + in_.w / 2 - ui_px(130.0),
                y: in_.y + ui_px(40.0),
                w: ui_px(260.0),
                h: ui_px(260.0),
            };
            ui_icon(&mut a.screen, IconKind::Share, icon, 0);
            placed(a, "the icon", icon);
        }
        let end = draw_text(a, in_.x, in_.w, y, &url, ad, n, qn > 0);
        placed(
            a,
            "the text",
            Rect {
                x: in_.x,
                y,
                w: in_.w,
                h: end - y,
            },
        );
        return;
    }
    let lx = in_.x + ui_px(30.0);
    let room = (bottom - top).min(in_.w * 45 / 100);
    let side = if qn != 0 {
        qr_side(qn, qn_data, qr_target, room)
    } else {
        0
    };
    let left = if qn != 0 { side } else { ui_px(260.0) };
    let col = lx + left + ui_px(60.0);
    let col_w = in_.x + in_.w - col;
    if qn != 0 {
        let sym = ui_qr(
            &mut a.screen,
            Rect {
                x: lx,
                y: top,
                w: left,
                h: left,
            },
            &qr,
            negative,
        );
        placed(a, "the code", sym);
    } else {
        let icon = Rect {
            x: lx,
            y: in_.y + ui_px(40.0),
            w: left,
            h: left,
        };
        ui_icon(&mut a.screen, IconKind::Share, icon, 0);
        placed(a, "the icon", icon);
    }
    let st = s(a);
    st.counter = Rect {
        x: col,
        y: st.counter.y,
        w: col_w,
        h: st.counter.h,
    };
    let end = draw_text(a, col, col_w, top, &url, ad, n, qn > 0);
    placed(
        a,
        "the text",
        Rect {
            x: col,
            y: top,
            w: col_w,
            h: end - top,
        },
    );
}

fn draw_wifi(a: &mut App) {
    hits_clear(&mut s(a).hits);
    let page = crate::library::settings_page(&mut a.screen, a.w, a.h, tr("Wi-Fi"), false, false);
    hits_add(&mut s(a).hits, page.close, H_WIFI_DONE, 0);
    let in_ = page.content;
    let state = wifi_status();
    let busy = wifi_changing() || (wifi_busy() && state.is_none());
    let failed = wifi_failed();
    let text = if busy {
        tr("Checking Wi-Fi…")
    } else if failed {
        tr("Could not read or change Wi-Fi.")
    } else {
        match state {
            Some(st) if !st.wireless || !st.enabled => tr("Wi-Fi is off"),
            Some(st) if st.connected == Some(true) => tr("Connected to Wi-Fi"),
            Some(st) if st.connected == Some(false) => tr("Wi-Fi is on; no connection yet"),
            Some(_) => tr("Wi-Fi is on; connection unknown"),
            None => tr("Wi-Fi unavailable"),
        }
    };
    let status = Rect::new(in_.x, in_.y, in_.w, ui_px(170.0));
    crate::library::settings_card(&mut a.screen, status, ButtonStyle::Plain);
    let isz = ui_px(70.0);
    let icon = Rect::new(
        status.x + crate::library::settings_pad(),
        status.y + (status.h - isz) / 2,
        isz,
        isz,
    );
    ui_icon(&mut a.screen, IconKind::Share, icon, 0);
    font_draw_left(
        &mut a.screen,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(
            icon.x + icon.w + crate::library::settings_gap(),
            status.y,
            status.x + status.w
                - icon.x
                - icon.w
                - crate::library::settings_gap()
                - crate::library::settings_pad(),
            status.h,
        ),
        text,
        0,
    );
    let help = Rect::new(
        in_.x,
        status.y + status.h + crate::library::settings_gap(),
        in_.w,
        ui_px(280.0),
    );
    crate::library::settings_card(&mut a.screen, help, ButtonStyle::Plain);
    for (i, line) in [
        tr("Connects to saved networks."),
        tr("Add new networks in Kindle settings or KOReader."),
        tr("Turning Wi-Fi off disconnects SSH and sharing."),
    ]
    .into_iter()
    .enumerate()
    {
        font_draw_left(
            &mut a.screen,
            FontFace::Regular,
            ui_small_size(),
            Rect::new(
                help.x + crate::library::settings_pad(),
                help.y + crate::library::settings_pad() + i as i32 * ui_px(70.0),
                help.w - 2 * crate::library::settings_pad(),
                ui_px(70.0),
            ),
            line,
            0,
        );
    }
    let bh = ui_px(150.0);
    let gap = crate::library::settings_gap();
    let bw = ((in_.w - 2 * gap) / 3).min(ui_px(520.0));
    let by = help.y + help.h + gap;
    let x0 = in_.x + (in_.w - (3 * bw + 2 * gap)) / 2;
    for (i, id, label, enabled) in [
        (0, H_WIFI_ON, tr("Turn on Wi-Fi"), !busy && state.is_some()),
        (
            1,
            H_WIFI_OFF,
            tr("Turn off Wi-Fi"),
            !busy && state.is_some(),
        ),
        (2, H_WIFI_CHECK, tr("Retry"), !busy),
    ] {
        let hit = Rect::new(x0 + i * (bw + gap), by, bw, bh);
        ui_button(
            &mut a.screen,
            hit,
            IconKind::None,
            label,
            if enabled {
                ButtonStyle::Plain
            } else {
                ButtonStyle::Disabled
            },
        );
        if enabled {
            hits_add(&mut s(a).hits, hit, id, 0);
        }
    }
    s(a).wifi_paint = Some((state, busy, failed));
}
fn draw(a: &mut App) {
    if s(a).wifi_open {
        draw_wifi(a);
        return;
    }

    hits_clear(&mut s(a).hits);
    s(a).parts.clear();
    let (live, started) = {
        let st = s(a);
        (st.live, st.started)
    };
    // Share and Live view's pairing card are the same Settings frame: the title and X above the
    // rule, the code and address on the page, and one row of actions at the foot.
    let page = crate::library::settings_page(
        &mut a.screen,
        a.w,
        a.h,
        if live {
            tr("Live view")
        } else {
            tr("Share over Wi-Fi")
        },
        false,
        false,
    );
    hits_add(&mut s(a).hits, page.close, H_DONE, 0);
    let in_ = page.content;
    let bh = ui_px(150.0);
    let gap = crate::library::settings_gap();
    let by = in_.y + in_.h - bh;
    let mut ad = [NetAddr::default(); MAX_SHOWN];
    let n = if started { net_ipv4_list(&mut ad) } else { 0 };
    let has_code = n > 0 && phone_reachable(ad[0].ifname.as_str());
    let hero_h = if n == 0 {
        ui_px(760.0)
    } else if has_code {
        if a.w <= a.h {
            ui_px(1680.0)
        } else {
            ui_px(1080.0)
        }
    } else if a.w <= a.h {
        ui_px(1280.0)
    } else {
        ui_px(1200.0)
    };
    let hero = Rect::new(in_.x, in_.y, in_.w, hero_h.min(in_.h - bh - gap).max(0));
    let hero_in = rect_inflate(hero, -crate::library::settings_pad());
    s(a).counter = Rect {
        x: hero_in.x,
        y: by - gap - ui_px(80.0),
        w: hero_in.w,
        h: ui_px(80.0),
    };
    if n > 0 {
        draw_addresses(a, hero_in, &ad, n);
    } else {
        let mut y = hero_in.y + ui_px(40.0);
        let icon = Rect {
            x: hero_in.x + hero_in.w / 2 - ui_px(130.0),
            y,
            w: ui_px(260.0),
            h: ui_px(260.0),
        };
        ui_icon(
            &mut a.screen,
            if live {
                IconKind::Live
            } else {
                IconKind::Share
            },
            icon,
            0,
        );
        placed(a, "the icon", icon);
        y += ui_px(320.0);
        let y0 = y;
        // TRANSLATORS: the Share screen without an address (1640 px at 64 px), and what to do (at
        // 44 px); Retry is the button's name.
        let what = if started {
            if wifi_available() {
                tr("No Wi-Fi connection yet")
            } else {
                tr("Turn on Wi-Fi on the Kindle first")
            }
        } else {
            tr("Could not start the server")
        };
        let then = if started {
            if wifi_available() {
                tr("Tap Wi-Fi to connect to a saved network.")
            } else {
                tr("Then tap Retry.")
            }
        } else {
            tr("Is another app using the port? Try again.")
        };
        draw_line(
            &mut a.screen,
            FontFace::Bold,
            ui_title_size(),
            Rect {
                x: hero_in.x,
                y,
                w: hero_in.w,
                h: ui_px(100.0),
            },
            what,
            0,
        );
        y += ui_px(110.0);
        let end = draw_line(
            &mut a.screen,
            FontFace::Regular,
            ui_text_size(),
            Rect {
                x: hero_in.x,
                y,
                w: hero_in.w,
                h: ui_px(80.0),
            },
            then,
            60,
        );
        placed(
            a,
            "the text",
            Rect {
                x: hero_in.x,
                y: y0,
                w: hero_in.w,
                h: end - y0,
            },
        );
    }
    if started {
        draw_counter(a);
        let counter = s(a).counter;
        placed(a, "the counter", counter);
    }
    // The actions are Settings cards too: the primary one, Wi-Fi, and Retry or Stop beside it.
    let mut actions: Vec<(i32, IconKind, &'static str)> = Vec::new();
    if live {
        // TRANSLATORS: Live view's pairing card: turns Live view off.
        actions.push((H_STOP, IconKind::Close, tr("Stop")));
        if n == 0 {
            // TRANSLATORS: starts the share server again.
            actions.push((H_RETRY, IconKind::None, tr("Retry")));
        }
    } else if n > 0 {
        // TRANSLATORS: the Share screen's button that turns Live view on (or shows its card while it
        // is on).
        actions.push((H_LIVE, IconKind::Live, tr("Live view")));
    } else {
        // TRANSLATORS: starts the share server again.
        actions.push((H_RETRY, IconKind::None, tr("Retry")));
    }
    if wifi_available() {
        actions.push((H_WIFI, IconKind::Share, tr("Wi-Fi")));
    }
    let nacts = actions.len() as i32;
    let bw = if nacts > 0 {
        ((in_.w - gap * (nacts - 1)) / nacts).min(ui_px(520.0))
    } else {
        0
    };
    let ax = in_.x + (in_.w - (nacts * bw + (nacts - 1) * gap)) / 2;
    for (i, (id, icon, label)) in actions.into_iter().enumerate() {
        let r = Rect::new(ax + i as i32 * (bw + gap), by, bw, bh);
        ui_button(&mut a.screen, r, icon, label, ButtonStyle::Plain);
        hits_add(&mut s(a).hits, r, id, 0);
        placed(a, label, r);
    }
    check_layout(a, in_);
    draw_ask(a);
}

/// The question's title for a sheet `w` wide: the name shortened with "..." until it fits, as the Library's Delete
/// question does.
fn ask_title(name: &str, size: u64, w: i32) -> String {
    let size = crate::browser::size_label(size);
    // TRANSLATORS: the question on the Kindle's Share screen when a PDF was sent from its page in a browser: its
    // file's name (shortened with "..." when long) and its size ("12.3 MB"); Import keeps it, Cancel does not.
    let title = |n: &str| cformat!(tr("Import \"%s\" (%s)?"), n, size.as_str());
    let wide = |t: &str| font_measure(FontFace::Title, ui_title_size(), t).max(font_measure(FontFace::Bold, ui_title_size(), t));
    let mut t = title(name);
    let mut n = name.chars().count();
    while n > 1 && wide(&t) > w {
        n -= 1;
        t = title(&format!("{}...", name.chars().take(n).collect::<String>()));
    }
    t
}

/// The send's code as the question shows it, big under its title ("Code 4821").
fn code_line(code: &str) -> String {
    // TRANSLATORS: the Kindle's question for a PDF sent from the Share page, big under its title: the send's code
    // (%s: four digits, "4821"), which the page that sent it shows too, so the user can see it is that PDF.
    cformat!(tr("Code %s"), code)
}

/// The question over the screen, when one is asked: the Library's question sheet (its title, the send's code in big
/// type, a Primary row and a plain one), drawn as Files draws its Delete question. Drawn for the first time or at a
/// new place (the screen turned), it has not been shown yet (app_ask): every draw of it is invalidated with the
/// screen's.
fn draw_ask(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let Some(k) = s(a).ask.as_ref() else { return };
    let (name, size, code, was) = (k.name.clone(), k.size, code_line(&k.code), k.rect);
    let (rh, gap, ch) = (ui_px(140.0), ui_px(20.0), ui_px(150.0));
    let rows = [(H_ASK_IMPORT, IconKind::Pdf, tr("Import")), (H_ASK_CANCEL, IconKind::Close, tr("Cancel"))];
    let rect = crate::library::sheet_rect(aw, ah, ui_px(120.0) + ui_px(72.0) + ch + rows.len() as i32 * (rh + gap));
    if rect != was {
        app_ask(a);
    }
    let title = ask_title(&name, size, rect.w - ui_px(160.0));
    let c = &mut a.screen;
    let inner = ui_sheet(c, rect, &title);
    // (half again the title's size: read from arm's length on the Kindle, as the page shows it big)
    font_draw_centered(c, FontFace::Bold, ui_title_size() * 1.5, Rect::new(inner.x, inner.y, inner.w, ch - gap), &code, 0);
    let mut hits = HitList::default();
    for (i, (id, icon, label)) in rows.iter().enumerate() {
        let r = Rect::new(inner.x, inner.y + ch + i as i32 * (rh + gap), inner.w, rh);
        ui_frame(c, r, ui_px(16.0), 255, true, 0);
        ui_frame(c, r, ui_px(16.0), 0, false, if *id == H_ASK_IMPORT { ui_px(6.0) } else { ui_px(3.0) });
        ui_icon(c, *icon, Rect::new(r.x + ui_px(20.0), r.y + ui_px(15.0), rh - ui_px(30.0), rh - ui_px(30.0)), 0);
        font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(r.x + rh + ui_px(10.0), r.y, r.w - rh - ui_px(30.0), r.h), label, 0);
        hits_add(&mut hits, r, *id, 0);
    }
    if let Some(k) = s(a).ask.as_mut() {
        k.rect = rect;
        k.hits = hits;
    }
}

/// The server's question as the screen shows it. A question shown that the server no longer asks, for whatever
/// reason (no answer in two minutes, a second PDF sent, the browser gone), or asks another in its place, is taken
/// away first, with "Not imported."; a new one is drawn (over the screen, its sheet refreshed) only when none is
/// shown, so never in the same call: a question is never replaced under the user's eyes, even by one that reads
/// the same.
fn ask_sync(a: &mut App) {
    let now = httpd_upload_question();
    let shown = s(a).ask.as_ref().map(|k| (k.serial, k.rect));
    match (now, shown) {
        (now, Some((serial, old))) if now.as_ref().map(|q| q.0) != Some(serial) => {
            s(a).ask = None;
            draw(a);
            app_invalidate(a, old, RefreshMode::Ui);
            app_toast(a, tr("Not imported."), 3000);
        }
        (Some((serial, name, size, code)), None) => {
            s(a).ask = Some(Ask { serial, name, size, code, ..Ask::default() });
            draw(a);
            let r = s(a).ask.as_ref().map_or(RECT_EMPTY, |k| k.rect);
            app_invalidate(a, r, RefreshMode::Ui);
        }
        _ => {}
    }
}

/// The user's answer to the question shown: Import (the PDF into grimoire/import, where the Library's import takes
/// it) or Cancel (it goes). The screen under the sheet is drawn again, and Import's toast says where it went.
fn ask_answer(a: &mut App, import: bool) {
    let Some(k) = s(a).ask.take() else { return };
    let r = httpd_upload_answer(k.serial, import);
    draw(a);
    app_invalidate(a, k.rect, RefreshMode::Ui);
    match r {
        Some(Ok(_)) => app_toast(a, &cformat!(tr("Copied to %s."), "grimoire/import"), 3000),
        Some(Err(_)) => app_toast(a, tr("Couldn't import PDF. Is the storage full?"), 4000),
        None => {}
    }
}

/// A contact while the question is up: only a tap that came down after the question was shown and lifts on the row
/// it came down on answers; a lift on Share's X, outside the sheet, is the question's Cancel and closes Share;
/// anything else, on the sheet or off it, does nothing.
fn ask_event(a: &mut App, ev: &InEvent) {
    let ph = app_pointer_phase(ev);
    let seen = app_asked_seen(a, ev);
    let on_x = hits_find(&s(a).hits, ev.x, ev.y).is_some_and(|h| h.id == H_DONE);
    let Some(k) = s(a).ask.as_mut() else { return };
    let on_x = on_x && !rect_contains(k.rect, ev.x as i32, ev.y as i32);
    let at = hits_find(&k.hits, ev.x, ev.y).map(|h| h.id);
    if ph == 0 {
        k.swallow = !seen;
        k.down = at;
        return;
    }
    if ph != 2 {
        return;
    }
    let (swallow, down) = (k.swallow, k.down.take());
    k.swallow = false;
    if on_x && down.is_none() {
        // Share's X stays live: Cancel, then Share closes. A refusal, so it waits for nothing (a tap that came down as
        // the question appeared counts): a question that comes again after every answer cannot keep Share open.
        ask_answer(a, false);
        let back = back_screen(a);
        return app_switch(a, back);
    }
    if swallow || at.is_none() || at != down {
        return;
    }
    ask_answer(a, at == Some(H_ASK_IMPORT));
}

/// Where Share's X and Esc go: the screen Share was opened from, or the Library.
fn back_screen(a: &mut App) -> ScreenId {
    let ret = s(a).ret;
    if ret == ScreenId::Share {
        ScreenId::Library
    } else {
        ret
    }
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

/// switch to SCR_SHARE (returns to prev_scr). While Live view is on, Share opens its pairing card
/// directly, the same card the Live view button shows.
pub fn share_open(a: &mut App) {
    if live_on() {
        s(a).wifi_open = false;
        share_open_live(a);
        return;
    }
    let scr = a.scr;
    let st = s(a);
    st.ret = scr;
    st.live = false;
    st.wifi_open = false;
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
        s(a).ask = None; // (and a PDF sent from its page goes)
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
    // (a PDF sent from the Share page that a crash left waiting for its answer: gone, issue #175)
    if crate::upload::upload_sweep() {
        qlog!("share: a sent PDF the last run left unanswered was removed");
    }
}
pub fn share_free(a: &mut App) {
    wifi_free();
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
    input_watch_fd(-1);
    // A PDF sent from the page and not answered yet goes: nobody answers a sleeping Kindle (issue #175).
    httpd_upload_discard(UploadEnd::Asleep);
    s(a).ask = None;
}

// The sleep screen while this screen is up and no page is shown in its place: the card without
// the code and the addresses, since nothing answers while the Kindle sleeps (share_sleep), and a
// sleeping Kindle need not show its address. The wake lays the screen out again (share_enter).
/// Share's sleep screen when no page is shown
pub fn share_draw_asleep(a: &mut App) {
    let live = s(a).live;
    let page = crate::library::settings_page(
        &mut a.screen,
        a.w,
        a.h,
        if live {
            tr("Live view")
        } else {
            tr("Share over Wi-Fi")
        },
        false,
        false,
    );
    let in_ = page.content;
    let card = Rect::new(in_.x, in_.y, in_.w, ui_px(640.0));
    ui_icon(
        &mut a.screen,
        if live {
            IconKind::Live
        } else {
            IconKind::Share
        },
        Rect {
            x: in_.x + in_.w / 2 - ui_px(130.0),
            y: card.y + ui_px(50.0),
            w: ui_px(260.0),
            h: ui_px(260.0),
        },
        0,
    );
    let y = card.y + ui_px(350.0);
    // TRANSLATORS: the Share screen as the Kindle's sleep screen.
    draw_line(
        &mut a.screen,
        FontFace::Bold,
        ui_title_size(),
        Rect {
            x: in_.x,
            y,
            w: in_.w,
            h: ui_px(100.0),
        },
        tr("Asleep"),
        0,
    );
    if live {
        return;
    }
    draw_line(
        &mut a.screen,
        FontFace::Regular,
        ui_text_size(),
        Rect {
            x: in_.x,
            y: y + ui_px(110.0),
            w: in_.w,
            h: ui_px(80.0),
        },
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
    s(a).ask = None; // (its upload goes with Share: httpd_release)
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
    if s(a).ask.is_some() {
        // (the question is the screen's only control while it is up, with Share's X)
        return ask_event(a, ev);
    }
    if app_pointer_phase(ev) != 2 {
        return;
    }
    let Some(h) = hits_find(&s(a).hits, ev.x, ev.y).copied() else {
        return;
    };
    let back = {
        let ret = s(a).ret;
        if ret == ScreenId::Share {
            ScreenId::Library
        } else {
            ret
        }
    };
    if h.id == H_WIFI {
        // The same Wi-Fi submenu Settings has, not a second, smaller copy: Back and its X return
        // here when it was opened from Share.
        crate::library::library_open_wifi_from_share(a);
    } else if h.id == H_WIFI_ON || h.id == H_WIFI_OFF || h.id == H_WIFI_CHECK {
        let action = if h.id == H_WIFI_ON {
            WifiAction::On
        } else if h.id == H_WIFI_OFF {
            WifiAction::Off
        } else {
            WifiAction::Status
        };
        wifi_request(action);
        s(a).wifi_check_at = now_ms();
        s(a).wifi_watch_at = now_ms();
        draw(a);
        app_invalidate_all(a, RefreshMode::Ui);
    } else if h.id == H_WIFI_DONE {
        s(a).wifi_open = false;
        if !s(a).started {
            start(a);
        }
        draw(a);
        app_invalidate_all(a, RefreshMode::Ui);
    } else if h.id == H_DONE {
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

/// Esc (`back`) on the Share screen: its Wi-Fi card closes, else back to where Share was opened from (its
/// Done). True when it did either.
pub fn share_back(a: &mut App) -> bool {
    if a.scr != ScreenId::Share {
        return false;
    }
    if s(a).ask.is_some() {
        // (Esc on the question is its Cancel, then what Esc does without one: Share closes, as its X does)
        ask_answer(a, false);
    }
    if s(a).wifi_open {
        s(a).wifi_open = false;
        if !s(a).started {
            start(a);
        }
        draw(a);
        app_invalidate_all(a, RefreshMode::Ui);
        return true;
    }
    let ret = s(a).ret;
    app_switch(
        a,
        if ret == ScreenId::Share {
            ScreenId::Library
        } else {
            ret
        },
    );
    true
}

pub fn share_tick(a: &mut App, now: u32) {
    httpd_poll();
    ask_sync(a);
    if s(a).wifi_open {
        if !wifi_busy()
            && !wifi_failed()
            && now.wrapping_sub(s(a).wifi_check_at) > 2000
            && now.wrapping_sub(s(a).wifi_watch_at) < 45000
            && wifi_status()
                .is_some_and(|st| st.wireless && st.enabled && st.connected != Some(true))
        {
            s(a).wifi_check_at = now;
            wifi_request(WifiAction::Status);
        }
        if s(a).wifi_paint
            != Some((
                wifi_status(),
                wifi_changing() || (wifi_busy() && wifi_status().is_none()),
                wifi_failed(),
            ))
        {
            draw(a);
            app_invalidate_all(a, RefreshMode::Ui);
        }
        return;
    }
    if wifi_available() && now.wrapping_sub(s(a).address_check_at) > 2000 {
        s(a).address_check_at = now;
        let mut addresses = Vec::new();
        net_ipv4_addrs(&mut addresses, MAX_SHOWN as i32);
        let key = addresses.join(",");
        if key != s(a).address_key {
            s(a).address_key = key;
            draw(a);
            app_invalidate_all(a, RefreshMode::Ui);
        }
    }
    if s(a).live {
        if httpd_live_pairings() != s(a).pairings {
            // A browser paired: the card's work is done (app.c says who is watching once its stream
            // starts).
            let ret = s(a).ret;
            app_switch(
                a,
                if ret == ScreenId::Share {
                    ScreenId::Library
                } else {
                    ret
                },
            );
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

/// Test selectors resolve only visible controls, just like the screen's pointer handler.
pub fn share_control_point(a: &App, name: &str) -> Option<(f32, f32)> {
    // the question's rows (issue #175), while it is up; of the screen's other controls only its X answers then
    if let Some(k) = a.share.as_ref()?.ask.as_ref().filter(|_| name != "done") {
        let id = match name {
            "ask.import" => H_ASK_IMPORT,
            "ask.cancel" => H_ASK_CANCEL,
            _ => return None,
        };
        return k.hits.h.iter().find(|h| h.id == id).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32));
    }
    let id = match name {
        "wifi" => H_WIFI,
        "wifi.on" => H_WIFI_ON,
        "wifi.off" => H_WIFI_OFF,
        "wifi.retry" => H_WIFI_CHECK,
        "wifi.done" => H_WIFI_DONE,
        "done" => H_DONE,
        "live" => H_LIVE,
        "stop" => H_STOP,
        "retry" => H_RETRY,
        _ => return None,
    };
    a.share
        .as_ref()?
        .hits
        .h
        .iter()
        .find(|h| h.id == id)
        .map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32))
}
pub fn share_wifi_open(a: &App) -> bool {
    crate::library::library_wifi_from_share(a)
}

/// The headless runner's checks (app_probe's "share." keys): "ask", the question's title as drawn ("" with none);
/// "code", the send's code under it as drawn ("Code 4821", "" with none); "upload", where the server's upload is
/// (none, receiving or asking).
pub fn share_probe(a: &App, key: &str, out: &mut String, cap: usize) -> i32 {
    match key {
        "ask" => {
            let t = a.share.as_ref().and_then(|st| st.ask.as_ref()).map_or(String::new(), |k| ask_title(&k.name, k.size, k.rect.w - ui_px(160.0)));
            csnprintf!(out, cap, "%s", &t);
        }
        "code" => {
            let t = a.share.as_ref().and_then(|st| st.ask.as_ref()).map_or(String::new(), |k| code_line(&k.code));
            csnprintf!(out, cap, "%s", &t);
        }
        "upload" => {
            let st = match httpd_upload_state() {
                UploadState::None => "none",
                UploadState::Receiving => "receiving",
                UploadState::Asking => "asking",
            };
            csnprintf!(out, cap, "%s", st);
        }
        _ => return -1,
    }
    0
}
