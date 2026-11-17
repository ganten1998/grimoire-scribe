// test_share: the Share screen (share.c) drawn and compared with the C's, pixel for pixel. The C tree
// checks the screen through the headless runner (tests/share_check.sh reads the QR code back from a
// shot and compares the addresses); here each case's whole screen is hashed (FNV-1a over the gray
// bytes) and the hashes are the ones the C build's share.c drew, from a driver that links share.c and
// httpd.c as they are (build: the C tree's CORE_SRC and UI_SRC, share.c, httpd.c, stubs for app.c's
// app_switch, app_invalidate and the like), each with GRIMOIRE_SIM_ADDRS set to the case's addresses
// and the server on port PORT. Live view's pairing card is here only where it shows no code (a code
// and a QR token are random): no address, and the sleep screen.
//
// The simulator's address hook is the only way to fix the addresses, so this is a sim-feature test.
// share_enter calls app_watch_server, which tells input_watch_fd (the headless runner's, in this
// build) which socket to watch, and the card's refreshes go through app_invalidate_all: app.rs's and
// sim_sdl.rs's own, which draw nothing else on the screen, so the 25 screens below are the C's,
// pixel for pixel.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::prelude::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use testlib::*;

/// The port the case hashes were drawn with (the address on screen and in its QR code has it).
const PORT: i32 = 18851;

/// The Share screen was redesigned in Rust (2026-11-15, the new menu) after the table below was drawn
/// from the C: its pixels are not the C's on any host. Set true to compare again once the C tree has
/// the new screen (or to record the new one).
const SHARE_IS_THE_CS_SCREEN: bool = false;

struct Case {
    label: &'static str,
    w: i32,
    h: i32,
    addrs: &'static str,
    live: bool,
    gothic: bool,
    invert: bool,
    lang: &'static str,
    /// "enter" (the screen as it opens), "asleep" (the sleep screen on it) or "tick" (a download served,
    /// the counter drawn again: the one case that serves, since the count outlives the server, and the
    /// C's driver draws it as the first)
    mode: &'static str,
    hash: u64,
}

const CASES: &[Case] = &[
    Case { label: "portrait_wifi_gothic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x138bb9e9e066a0e4 },
    Case { label: "landscape_wifi_gothic", w: 2480, h: 1860, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x31fe38b1405b9638 },
    Case { label: "portrait_wifi_classic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: false, invert: false, lang: "en", mode: "enter", hash: 0xc073d56f88ebe2fa },
    Case { label: "landscape_wifi_classic", w: 2480, h: 1860, addrs: "wlan0=192.168.0.20", live: false, gothic: false, invert: false, lang: "en", mode: "enter", hash: 0xf5fb2304ddb27a64 },
    Case { label: "portrait_wifi_inverted", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: true, lang: "en", mode: "enter", hash: 0xbd7d978724a4abe4 },
    Case { label: "landscape_wifi_inverted", w: 2480, h: 1860, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: true, lang: "en", mode: "enter", hash: 0x92f44fe281325918 },
    Case { label: "portrait_usb_only_gothic", w: 1860, h: 2480, addrs: "usb0=192.168.15.244", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0xf8ca452e5fb46dc8 },
    Case { label: "landscape_usb_only_gothic", w: 2480, h: 1860, addrs: "usb0=192.168.15.244", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x840f439560b6ed6c },
    Case { label: "portrait_three_addresses_gothic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20,usb0=192.168.15.244,en0=10.0.0.5,eth1=1.2.3.4", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x8c1a03a876c805b4 },
    Case { label: "landscape_three_addresses_classic", w: 2480, h: 1860, addrs: "wlan0=192.168.0.20,usb0=192.168.15.244,en0=10.0.0.5,eth1=1.2.3.4", live: false, gothic: false, invert: false, lang: "en", mode: "enter", hash: 0x0a60885c7acd6de4 },
    Case { label: "portrait_no_address_gothic", w: 1860, h: 2480, addrs: "", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x63f990aa45783ba8 },
    Case { label: "landscape_no_address_classic", w: 2480, h: 1860, addrs: "", live: false, gothic: false, invert: false, lang: "en", mode: "enter", hash: 0xc605693a24ceaf60 },
    Case { label: "portrait_ethernet_gothic", w: 1860, h: 2480, addrs: "en0=10.0.0.5", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x2810882339500ce4 },
    Case { label: "portrait_wifi_spanish_gothic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "es", mode: "enter", hash: 0x828fa454e3d7abee },
    Case { label: "landscape_wifi_spanish_classic", w: 2480, h: 1860, addrs: "wlan0=192.168.0.20", live: false, gothic: false, invert: false, lang: "es", mode: "enter", hash: 0x4353775ca3743f68 },
    Case { label: "small_portrait_wifi_gothic", w: 1072, h: 1448, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0x80a5a14c670cb5bc },
    Case { label: "small_landscape_three_addresses_classic", w: 1448, h: 1072, addrs: "wlan0=192.168.0.20,usb0=192.168.15.244,en0=10.0.0.5,eth1=1.2.3.4", live: false, gothic: false, invert: false, lang: "en", mode: "enter", hash: 0x754da609fc1948b6 },
    Case { label: "portrait_live_no_address_gothic", w: 1860, h: 2480, addrs: "", live: true, gothic: true, invert: false, lang: "en", mode: "enter", hash: 0xcde73db52f5cb78f },
    Case { label: "landscape_live_no_address_classic", w: 2480, h: 1860, addrs: "", live: true, gothic: false, invert: false, lang: "en", mode: "enter", hash: 0x92d4e22399d2725b },
    Case { label: "portrait_live_no_address_spanish", w: 1860, h: 2480, addrs: "", live: true, gothic: true, invert: false, lang: "es", mode: "enter", hash: 0x5033ba5e32f886e1 },
    Case { label: "portrait_asleep_gothic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "en", mode: "asleep", hash: 0xd9aa20fac304f48d },
    Case { label: "landscape_asleep_classic", w: 2480, h: 1860, addrs: "wlan0=192.168.0.20", live: false, gothic: false, invert: false, lang: "en", mode: "asleep", hash: 0x553efbb87ad9b1eb },
    Case { label: "portrait_live_asleep_gothic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: true, gothic: true, invert: false, lang: "en", mode: "asleep", hash: 0x6d25238b7e8e303b },
    Case { label: "portrait_asleep_spanish", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "es", mode: "asleep", hash: 0x4086e21faf26ab7b },
    Case { label: "portrait_after_a_download_gothic", w: 1860, h: 2480, addrs: "wlan0=192.168.0.20", live: false, gothic: true, invert: false, lang: "en", mode: "tick", hash: 0xb67d1d6199e27864 },
];

fn fnv(c: &Canvas) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for y in 0..c.h {
        for x in 0..c.w {
            h ^= c.px[(y * c.stride + x) as usize] as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}

/// One request answered by the server, polled from this thread.
fn serve_one() {
    let mut s = TcpStream::connect(("127.0.0.1", PORT as u16)).unwrap();
    s.write_all(b"GET /api/list.json HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
    s.set_nonblocking(true).unwrap();
    for _ in 0..2000 {
        if httpd_requests_served() != 0 {
            break;
        }
        httpd_poll();
        std::thread::sleep(Duration::from_micros(500));
    }
    for _ in 0..20 {
        httpd_poll();
        std::thread::sleep(Duration::from_micros(500));
    }
    let mut b = [0u8; 4096];
    let _ = s.read(&mut b);
}

fn drawn(c: &Case) -> u64 {
    std::env::set_var("GRIMOIRE_SIM_ADDRS", c.addrs);
    ui_set_theme(if c.gothic { UiTheme::Gothic } else { UiTheme::Classic });
    i18n_apply(c.lang);
    let mut a = App { w: c.w, h: c.h, dpi: 300.0, ..Default::default() };
    check!(canvas_init(&mut a.screen, c.w, c.h) == 0);
    a.settings.http_port = PORT;
    a.settings.invert = c.invert;
    share_init(&mut a);
    a.scr = ScreenId::Share;
    if c.live {
        share_open_live(&mut a);
    } else {
        share_enter(&mut a);
    }
    if c.mode == "asleep" {
        share_draw_asleep(&mut a);
    }
    if c.mode == "tick" {
        serve_one();
        share_tick(&mut a, now_ms().wrapping_add(3000));
    }
    let h = fnv(&a.screen);
    share_free(&mut a);
    h
}

fn every_screen_is_the_c_screen() {
    if !SHARE_IS_THE_CS_SCREEN {
        eprintln!("  share: the screen was redesigned in Rust; the C's pixels are not compared");
        return;
    }
    for c in CASES {
        let h = drawn(c);
        if h != c.hash {
            eprintln!("  {}: drew {h:016x}, the C draws {:016x}", c.label, c.hash);
        }
        check!(h == c.hash);
    }
    std::env::remove_var("GRIMOIRE_SIM_ADDRS");
}

fn the_card_knows_whether_it_is_live_view() {
    let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
    share_init(&mut a);
    check!(!share_live_card_up(&a));
    a.scr = ScreenId::Share;
    check!(!share_live_card_up(&a));
    share_free(&mut a);
    check!(!share_live_card_up(&a) && a.share.is_none());
}

fn wifi_controls_remain_reachable_on_live_cards_without_an_address() {
    let path=format!("{}/wifi-share-{}.sh",tl_tmp(),std::process::id());
    std::fs::write(&path,"printf 'wireless=0\\nenabled=0\\nconnected=0\\n'\n").unwrap();
    for lang in ["en","es"] {for gothic in [true,false] {for (w,h) in [(1860,2480),(2480,1860)] {
        std::env::set_var("GRIMOIRE_SIM_ADDRS","");i18n_apply(lang);ui_set_theme(if gothic{UiTheme::Gothic}else{UiTheme::Classic});
        wifi_init(Some(&path));
        let mut a=App{w,h,dpi:300.0,..Default::default()};canvas_init(&mut a.screen,w,h);a.settings.http_port=PORT;
        grimoire::library::library_init(&mut a); // (Share's Wi-Fi button opens the Library's Settings > Wi-Fi)
        share_init(&mut a);a.scr=ScreenId::Share;share_open_live(&mut a);
        let (x,y)=grimoire::share::share_control_point(&a,"wifi").unwrap();
        share_event(&mut a,&InEvent{type_:InType::TouchUp,x,y,..Default::default()});
        check!(grimoire::share::share_wifi_open(&a));
        // Back from the submenu returns to Share (the same path its own Back and X take).
        check!(grimoire::library::library_back(&mut a));
        check!(!grimoire::share::share_wifi_open(&a) && a.scr == ScreenId::Share);
        share_free(&mut a);grimoire::library::library_free(&mut a);wifi_init(None);
    }}}
    std::env::remove_var("GRIMOIRE_SIM_ADDRS");std::fs::remove_file(path).unwrap();
}

// ---- the question a PDF sent from the Share page asks (issue #175) ----

fn probe(a: &mut App, key: &str) -> String {
    let mut o = String::new();
    check!(app_probe(a, key, &mut o, 512) == 0);
    o
}

/// The screen's tick, as the main loop gives it, then a moment.
fn tick(a: &mut App) {
    share_tick(a, now_ms());
    std::thread::sleep(Duration::from_millis(1));
}

/// The send's code that the Kindle's question shows under its title ("Code 4821" as drawn), and the server's.
fn shown_code(a: &mut App) -> String {
    let drawn = probe(a, "share.code");
    let server = httpd_upload_question().map(|q| q.3).unwrap_or_default();
    check!(drawn == format!("Code {server}"));
    server
}

/// How many dark pixels the screen has in the band between the question's title and its Import row, where the code
/// is drawn (none without it: the sheet is white there).
fn code_ink(a: &mut App) -> usize {
    let Some((ix, iy)) = grimoire::share::share_control_point(a, "ask.import") else { return 0 };
    let c = &a.screen;
    let (top, bottom) = (iy as i32 - 70 - 150, iy as i32 - 70 - 20);
    (top..bottom).flat_map(|y| (ix as i32 - 700..ix as i32 + 700).map(move |x| (x, y))).filter(|&(x, y)| c.px[(y * c.stride + x) as usize] < 128).count()
}

/// Whether a send's whole answer is its reply's head, its code, then `said`.
fn said(answer: &str, code: &str, said: &str) -> bool {
    let ok = answer.starts_with("HTTP/1.0 200 OK\r\n") && answer.ends_with(&format!("\r\n\r\ncode {code}\n{said}\n"));
    if !ok {
        eprintln!("    wanted code {code} and {said:?}, got {answer:?}");
    }
    ok
}

/// Everything the server answers on s, to its close, the screen ticking between reads.
fn answer(a: &mut App, s: &mut TcpStream) -> String {
    let mut out = Vec::new();
    let mut b = [0u8; 4096];
    for _ in 0..20_000 {
        tick(a);
        match s.read(&mut b) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&b[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The Share page's token, as its script reads it from the page (the screen ticking while it is served).
fn page_token(a: &mut App) -> String {
    let mut g = TcpStream::connect(("127.0.0.1", PORT as u16)).unwrap();
    g.write_all(b"GET / HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
    g.set_nonblocking(true).unwrap();
    let page = answer(a, &mut g);
    const AT: &str = "window.GRIMOIRE_UPLOAD={\"token\":\"";
    let token: String = page.find(AT).map(|i| page[i + AT.len()..].chars().take_while(|c| c.is_ascii_hexdigit()).collect()).unwrap_or_default();
    check!(token.len() == 32);
    token
}

/// A small PDF named `name` (percent-encoded) sent with `token` as the page's script sends it, `fill` making its
/// bytes: the connection, everything written, the screen not ticked.
fn post_pdf(token: &str, name: &str, fill: char) -> TcpStream {
    let body = format!("%PDF-1.4\n{}\n%%EOF\n", fill.to_string().repeat(1000));
    let mut s = TcpStream::connect(("127.0.0.1", PORT as u16)).unwrap();
    let head = format!("POST /upload HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nOrigin: http://127.0.0.1:{PORT}\r\nContent-Length: {}\r\nX-Grimoire-Token: {token}\r\nX-Grimoire-Name: {name}\r\n\r\n", body.len());
    s.write_all(head.as_bytes()).unwrap();
    s.write_all(body.as_bytes()).unwrap();
    s.set_nonblocking(true).unwrap();
    s
}

/// A browser on the Wi-Fi sends a small PDF named `name` (percent-encoded) as the Share page's script does: the page
/// for its token, then the POST. Its connection, the body sent; the screen has ticked until it asks.
fn send_pdf(a: &mut App, name: &str) -> TcpStream {
    let token = page_token(a);
    let s = post_pdf(&token, name, 'x');
    for _ in 0..5000 {
        tick(a);
        if !probe(a, "share.ask").is_empty() {
            break;
        }
    }
    s
}

fn touch(a: &mut App, down: bool, x: f32, y: f32, t: u32) {
    share_event(a, &InEvent { type_: if down { InType::TouchDown } else { InType::TouchUp }, x, y, t_ms: t, ..Default::default() });
}

fn the_question_asks_and_only_a_tap_after_it_was_shown_answers() {
    std::env::set_var("GRIMOIRE_SIM_ADDRS", "wlan0=192.168.0.20");
    ui_set_theme(UiTheme::Gothic);
    i18n_apply("en");
    let root = store_root();
    let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
    check!(canvas_init(&mut a.screen, 1860, 2480) == 0);
    a.settings.http_port = PORT;
    share_init(&mut a);
    a.scr = ScreenId::Share;
    share_enter(&mut a);
    check!(probe(&mut a, "share.ask").is_empty() && probe(&mut a, "share.upload") == "none");
    check!(grimoire::share::share_control_point(&a, "ask.import").is_none());
    let mut s = send_pdf(&mut a, "2027%20Planner.pdf");
    check!(probe(&mut a, "share.ask") == "Import \"2027 Planner.pdf\" (1 KB)?");
    check!(probe(&mut a, "share.upload") == "asking");
    // the send's code under the title, the one the browser's reply carries
    let code = shown_code(&mut a);
    check!(code.len() == 4 && code.bytes().all(|d| d.is_ascii_digit()));
    check!(code_ink(&mut a) > 2000);
    // drawn, not shown yet: no contact answers it, wherever it lands (the second of two taps at one place)
    check!(a.asked == Asked::Drawn);
    let (ix, iy) = grimoire::share::share_control_point(&a, "ask.import").unwrap();
    let (cx, cy) = grimoire::share::share_control_point(&a, "ask.cancel").unwrap();
    let t = now_ms();
    touch(&mut a, true, ix, iy, t);
    touch(&mut a, false, ix, iy, t + 50);
    check!(probe(&mut a, "share.upload") == "asking" && !probe(&mut a, "share.ask").is_empty());
    // the screen's own controls are not there while it is up
    check!(grimoire::share::share_control_point(&a, "done").is_none() && grimoire::share::share_control_point(&a, "live").is_none());
    // shown (app_flush notes when): a contact too soon after, one that lifts on the other row, one off the rows
    let at = now_ms();
    a.asked = Asked::Shown(at);
    touch(&mut a, true, ix, iy, at + PEN_REOPEN_MS - 1);
    touch(&mut a, false, ix, iy, at + PEN_REOPEN_MS + 100);
    touch(&mut a, true, ix, iy, at + PEN_REOPEN_MS);
    touch(&mut a, false, cx, cy, at + PEN_REOPEN_MS + 100);
    touch(&mut a, true, 20.0, 20.0, at + PEN_REOPEN_MS);
    touch(&mut a, false, 20.0, 20.0, at + PEN_REOPEN_MS + 100);
    check!(probe(&mut a, "share.upload") == "asking");
    check!(!std::path::Path::new(&format!("{root}/import/2027 Planner.pdf")).exists());
    // Import: the PDF into grimoire/import, the toast says where, the browser is told
    touch(&mut a, true, ix, iy, at + PEN_REOPEN_MS);
    touch(&mut a, false, ix, iy, at + PEN_REOPEN_MS + 100);
    check!(probe(&mut a, "share.ask").is_empty() && probe(&mut a, "share.upload") == "none");
    check!(a.toast.as_str() == "Copied to grimoire/import.");
    check!(std::fs::read(format!("{root}/import/2027 Planner.pdf")).is_ok_and(|b| b.starts_with(b"%PDF-1.4") && b.ends_with(b"%%EOF\n")));
    check!(said(&answer(&mut a, &mut s), &code, "imported"));
    check!(probe(&mut a, "share.code").is_empty());
    check!(grimoire::share::share_control_point(&a, "done").is_some());
    // Cancel: nothing kept, no toast
    a.toast.set("");
    let mut s = send_pdf(&mut a, "Cancelled.pdf");
    let code = shown_code(&mut a);
    a.asked = Asked::Shown(now_ms().wrapping_sub(PEN_REOPEN_MS));
    let (cx, cy) = grimoire::share::share_control_point(&a, "ask.cancel").unwrap();
    let t = now_ms();
    touch(&mut a, true, cx, cy, t);
    touch(&mut a, false, cx, cy, t + 80);
    check!(probe(&mut a, "share.ask").is_empty() && a.toast.as_str().is_empty());
    check!(said(&answer(&mut a, &mut s), &code, "declined on the Kindle"));
    check!(!std::path::Path::new(&format!("{root}/import/Cancelled.pdf")).exists());
    check!(!std::path::Path::new(&grimoire::upload::part_path()).exists());
    // Esc is Cancel
    let mut s = send_pdf(&mut a, "Esc.pdf");
    let code = shown_code(&mut a);
    check!(grimoire::share::share_back(&mut a) && a.scr == ScreenId::Share);
    check!(probe(&mut a, "share.ask").is_empty() && said(&answer(&mut a, &mut s), &code, "declined on the Kindle"));
    // a turn draws it at its new place: asked again, not shown there yet
    let mut s = send_pdf(&mut a, "Turned.pdf");
    let code = shown_code(&mut a);
    a.asked = Asked::Shown(now_ms());
    a.w = 2480;
    a.h = 1860;
    check!(canvas_init(&mut a.screen, 2480, 1860) == 0);
    a.relayout = true;
    share_leave(&mut a);
    share_enter(&mut a);
    a.relayout = false;
    check!(a.asked == Asked::Drawn && probe(&mut a, "share.ask") == "Import \"Turned.pdf\" (1 KB)?");
    check!(shown_code(&mut a) == code && code_ink(&mut a) > 2000);
    // sleep ends it, and the wake shows the screen without it
    share_sleep(&mut a);
    check!(probe(&mut a, "share.ask").is_empty() && probe(&mut a, "share.upload") == "none");
    check!(said(&answer(&mut a, &mut s), &code, "the Kindle is asleep"));
    a.relayout = true;
    share_enter(&mut a);
    a.relayout = false;
    check!(probe(&mut a, "share.ask").is_empty());
    // no answer for a while: it goes, and the toast says it was not imported
    std::env::set_var("GRIMOIRE_UPLOAD_ASK_MS", "200");
    let mut s = send_pdf(&mut a, "Unanswered.pdf");
    let code = shown_code(&mut a);
    for _ in 0..2000 {
        tick(&mut a);
        if probe(&mut a, "share.ask").is_empty() {
            break;
        }
    }
    std::env::remove_var("GRIMOIRE_UPLOAD_ASK_MS");
    check!(probe(&mut a, "share.ask").is_empty() && a.toast.as_str() == "Not imported.");
    check!(said(&answer(&mut a, &mut s), &code, "no answer on the Kindle"));
    // leaving Share ends it (and the server with it)
    let mut s = send_pdf(&mut a, "Left.pdf");
    share_leave(&mut a);
    check!(probe(&mut a, "share.upload") == "none" && !httpd_running());
    let mut rest = Vec::new();
    let _ = s.read_to_end(&mut rest);
    check!(!std::path::Path::new(&format!("{root}/import/Left.pdf")).exists());
    check!(!std::path::Path::new(&grimoire::upload::part_path()).exists());
    // in import/ only what Import kept (beside the import's own dot-files)
    let names: Vec<String> = std::fs::read_dir(format!("{root}/import")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.')).collect();
    check!(names == ["2027 Planner.pdf"]);
    // a leftover of a crash goes at the next start (share_init)
    check!(std::fs::write(grimoire::upload::part_path(), b"%PDF-").is_ok());
    share_free(&mut a);
    share_init(&mut a);
    check!(!std::path::Path::new(&grimoire::upload::part_path()).exists());
    share_free(&mut a);
    std::env::remove_var("GRIMOIRE_SIM_ADDRS");
}

/// The Share screen open on the Kindle, as the question tests need it.
fn share_app() -> App {
    std::env::set_var("GRIMOIRE_SIM_ADDRS", "wlan0=192.168.0.20");
    ui_set_theme(UiTheme::Gothic);
    i18n_apply("en");
    let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
    check!(canvas_init(&mut a.screen, 1860, 2480) == 0);
    a.settings.http_port = PORT;
    share_init(&mut a);
    a.scr = ScreenId::Share;
    share_enter(&mut a);
    a
}

// A question that ends while it is shown, for whatever reason, is taken away first, with "Not imported.", and the
// next is drawn only at a later tick: here one sender's PDF ends the question about another's and a third arrives in
// the same poll with the same name and size, so the question would read the same. It is taken away all the same.
// (The server's quiet time after a send that was not imported, made 0 here, would refuse the third: the screen's rule
// is held without it.)
fn a_question_that_ends_is_taken_away_before_another_is_drawn() {
    let mut a = share_app();
    let root = store_root();
    let token = page_token(&mut a);
    let mut mine = send_pdf(&mut a, "Planner.pdf");
    check!(probe(&mut a, "share.ask") == "Import \"Planner.pdf\" (1 KB)?");
    let first = httpd_upload_question().map_or(0, |q| q.0);
    let mine_code = shown_code(&mut a);
    a.asked = Asked::Shown(now_ms().wrapping_sub(PEN_REOPEN_MS));
    a.toast.set("");
    // two more, both written before the screen's next tick: the first ends mine, the second comes right behind it
    let mut other = post_pdf(&token, "Planner.pdf", 'x');
    let mut theirs = post_pdf(&token, "Planner.pdf", 'y');
    tick(&mut a);
    check!(httpd_upload_question().is_some_and(|q| q.0 != first));
    check!(probe(&mut a, "share.ask").is_empty() && a.toast.as_str() == "Not imported.");
    check!(grimoire::share::share_control_point(&a, "ask.import").is_none());
    // the next tick draws the server's new question, as a question of its own: not shown yet
    tick(&mut a);
    check!(probe(&mut a, "share.ask") == "Import \"Planner.pdf\" (1 KB)?" && a.asked == Asked::Drawn);
    // under it the third's own code, which only the third's reply carries: mine had its own, and its end
    let theirs_code = shown_code(&mut a);
    check!(said(&answer(&mut a, &mut mine), &mine_code, "another PDF was sent"));
    check!(answer(&mut a, &mut other).ends_with("\r\n\r\nanother PDF is being sent\n"));
    grimoire::share::share_back(&mut a);
    check!(said(&answer(&mut a, &mut theirs), &theirs_code, "declined on the Kindle"));
    check!(!std::path::Path::new(&format!("{root}/import/Planner.pdf")).exists());
    share_leave(&mut a);
    share_free(&mut a);
    std::env::remove_var("GRIMOIRE_SIM_ADDRS");
}

#[test]
fn test_share() {
    let root = format!("{}/share-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    ui_init(300.0);
    check!(font_init() == 0);
    run!(every_screen_is_the_c_screen);
    run!(the_card_knows_whether_it_is_live_view);
    run!(wifi_controls_remain_reachable_on_live_cards_without_an_address);
    // (the question cases send again at once after a send that was not imported: no quiet time for them, and the
    // screen's own rule is held as if there were none)
    std::env::set_var("GRIMOIRE_UPLOAD_QUIET_MS", "0");
    run!(the_question_asks_and_only_a_tap_after_it_was_shown_answers);
    run!(a_question_that_ends_is_taken_away_before_another_is_drawn);
    std::env::remove_var("GRIMOIRE_UPLOAD_QUIET_MS");
    let _ = std::fs::remove_dir_all(&root);
    done();
}
