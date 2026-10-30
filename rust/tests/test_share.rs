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
        share_init(&mut a);a.scr=ScreenId::Share;share_open_live(&mut a);
        let (x,y)=grimoire::share::share_control_point(&a,"wifi").unwrap();
        share_event(&mut a,&InEvent{type_:InType::TouchUp,x,y,..Default::default()});
        check!(grimoire::share::share_wifi_open(&a));
        let (x,y)=grimoire::share::share_control_point(&a,"wifi.done").unwrap();
        share_event(&mut a,&InEvent{type_:InType::TouchUp,x,y,..Default::default()});
        check!(!grimoire::share::share_wifi_open(&a));share_free(&mut a);wifi_init(None);
    }}}
    std::env::remove_var("GRIMOIRE_SIM_ADDRS");std::fs::remove_file(path).unwrap();
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
    let _ = std::fs::remove_dir_all(&root);
    done();
}
