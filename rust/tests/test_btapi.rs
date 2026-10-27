// test_btapi.c: the bounded JSON reader, the Bluetooth helper's answers read from it (a /status the
// helper gave on the Kindle, its address made up), and the client against tests/fake_bthelper.py.
mod testlib;
use grimoire::prelude::*;
use std::ffi::CString;
use testlib::*;

fn json_reads_what_it_should_and_refuses_the_rest() {
    let s = r#"{"ok": true, "version": "3.18.0", "n": -42, "devices": [{"address": "AA:BB", "name": "Pro \u00e9\ud83c\udfb5"}, {"address": "CC", "name": null}], "empty": {}, "arr": []}"#;
    let text = s.as_bytes();
    let mut all = [JsonTok::default(); 64];
    let n = json_parse(text, &mut all);
    check!(n > 0 && all[0].type_ == JsonType::Object && all[0].size == 6);
    let n = n.max(0) as usize;
    let t = &all[..n];
    let n = n as i32;
    let mut b = false;
    let mut v = 0;
    let mut out: Vec<u8> = Vec::new();
    // (The C indexes with whatever json_get returns; a token that is not there would be -1 in both.)
    let tok = |i: i32| &t[i as usize];
    check!(json_bool(tok(json_get(text, t, 0, "ok")), &mut b) && b);
    check!(json_string(text, tok(json_get(text, t, 0, "version")), &mut out, 64) && out == b"3.18.0");
    check!(json_int(text, tok(json_get(text, t, 0, "n")), &mut v) && v == -42);
    let devs = json_get(text, t, 0, "devices");
    check!(devs > 0 && tok(devs).size == 2);
    let d0 = json_item(t, devs, 0);
    let d1 = json_item(t, devs, 1);
    check!(json_string(text, tok(json_get(text, t, d0, "name")), &mut out, 64) && out == "Pro \u{e9}\u{1f3b5}".as_bytes());
    check!(json_string(text, tok(json_get(text, t, d0, "name")), &mut out, 7) && out == "Pro \u{e9}".as_bytes()); // cut whole
    check!(tok(json_get(text, t, d1, "name")).type_ == JsonType::Null);
    check!(json_get(text, t, 0, "missing") == -1 && json_item(t, devs, 2) == -1);
    check!(json_get(text, t, json_get(text, t, 0, "arr"), "x") == -1);
    let bad: [&[u8]; 13] = [
        b"", b"{", b"{\"a\":}", b"[1,]", b"{\"a\" 1}", b"\"\\x\"", b"tru", b"01x", b"{\"a\":1} 2", b"[\"\x01\"]", b"-", b"1.", b"1e",
    ];
    for b in bad {
        check!(json_parse(b, &mut all) < 0);
    }
    let mut deep = [0u8; 40];
    deep[..20].fill(b'[');
    deep[20..].fill(b']');
    check!(json_parse(&deep, &mut all) < 0); // deeper than JSON_DEPTH
    check!(json_parse(text, &mut all[..5]) < 0); // more tokens than room
    let _ = n;
}

fn helper_answers_are_read_safely() {
    let status = read_file(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/bluetooth/status.json"));
    check!(status.is_some());
    let Some(status) = status else { return };
    let mut r = BtReply::default();
    check!(btapi_parse(&status, &mut r) == 0 && r.ok && r.running && !r.scanning && r.ndev == 1);
    check!(r.dev[0].address == "02:00:5E:10:20:30" && r.dev[0].name == "Pro Controller" && r.dev[0].classic && r.dev[0].connected);
    check!(r.version == "3.18.0-ea011ea");
    let scan = r#"{"ok": true, "scanning": true, "devices": [{"address": "aa:bb:cc:dd:ee:ff", "name": "  8BitDo\u0007\u0008 Stick  ", "protocol": "ble"}, {"address": "bad"}, {"address": "AA:BB:CC:DD:EE:F", "name": "y"}]}"#;
    check!(btapi_parse(scan.as_bytes(), &mut r) == 0 && r.ok && r.scanning && r.ndev == 1 && !r.dev[0].classic);
    check!(r.dev[0].name == "8BitDo? Stick"); // radio names: printable ASCII, trimmed
    let err = r#"{"ok": false, "error": "No scan in progress"}"#;
    check!(btapi_parse(err.as_bytes(), &mut r) == 0 && !r.ok && r.error == "No scan in progress");
    check!(btapi_parse(b"[1]", &mut r) < 0 && btapi_parse(b"{", &mut r) < 0);
    check!(
        btapi_address_ok("AA:bb:01:23:45:67")
            && !btapi_address_ok("AA:bb:01:23:45:6")
            && !btapi_address_ok("AA-bb-01-23-45-67")
            && !btapi_address_ok("AA:bb:01:23:45:6g")
    );
}

// (By the clock, not by turns: on a loaded machine the turns run long. A request's child ends within
// 4 s, its own alarm.)
fn wait_event(r: &mut BtReply) -> BtEvent {
    let t0 = now_ms();
    while now_ms().wrapping_sub(t0) < 20000 {
        let e = bt_tick(r);
        if e != BtEvent::None {
            return e;
        }
        sleep_ms(10);
    }
    BtEvent::None
}

fn the_client_asks_one_question_at_a_time() {
    let named = format!("{}/grimoire-bthelper-{}.port", tl_tmp(), std::process::id());
    let _ = std::fs::remove_file(&named);
    let script = CString::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fake_bthelper.py")).unwrap();
    let cnamed = CString::new(named.as_str()).unwrap();
    // SAFETY: fork in the test; the child only execs python3 (arguments made before the fork) or _exits.
    let fake = unsafe { libc::fork() };
    if fake == 0 {
        // SAFETY: execlp gets NUL-terminated strings and a NULL-terminated list; _exit if it fails.
        unsafe {
            libc::execlp(
                c"python3".as_ptr(),
                c"python3".as_ptr(),
                c"-I".as_ptr(),
                script.as_ptr(),
                c"0".as_ptr(),
                cnamed.as_ptr(),
                std::ptr::null::<libc::c_char>(),
            );
            libc::_exit(127);
        }
    }
    check!(fake > 0);
    // The stand-in names its port once it listens (a minute at most, on a loaded machine).
    let mut port = 0;
    let t0 = now_ms();
    while port == 0 && now_ms().wrapping_sub(t0) < 60000 {
        if let Some(text) = read_file(&named) {
            port = String::from_utf8_lossy(&text).trim().parse::<i32>().unwrap_or(0);
        }
        if port == 0 {
            sleep_ms(50);
        }
    }
    let _ = std::fs::remove_file(&named);
    check!(port > 0);
    let api = format!("127.0.0.1:{}", port);
    bt_init(&tl_tmp(), None, Some(&api));
    let mut r = BtReply::default();
    check!(bt_present());
    bt_start();
    check!(bt_state() == BtState::External); // a stand-in that runs already: Grimoire's to use, not to stop
    check!(bt_request("/status") && wait_event(&mut r) == BtEvent::Reply && r.ok && r.ndev == 0);
    check!(bt_request("/scan") && !bt_request("/status") && bt_busy()); // one at a time
    check!(wait_event(&mut r) == BtEvent::Reply && r.ok);
    sleep_ms(1100);
    check!(bt_request("/scan-status") && wait_event(&mut r) == BtEvent::Reply && r.ndev == 2);
    check!(r.dev[1].name == "Stick? ?");
    check!(bt_request("/pair?addr=02:00:5E:10:20:30&protocol=classic") && wait_event(&mut r) == BtEvent::Reply);
    sleep_ms(900);
    check!(bt_request("/pair-status") && wait_event(&mut r) == BtEvent::Reply && r.ok);
    check!(bt_request("/status") && wait_event(&mut r) == BtEvent::Reply && r.ndev == 1 && r.dev[0].connected);
    check!(!bt_request("/a b") && !bt_request("status")); // one plain line, a path
    // SAFETY: kill and waitpid on the stand-in forked above.
    unsafe {
        libc::kill(fake, libc::SIGTERM);
        libc::waitpid(fake, std::ptr::null_mut(), 0);
    }
    check!(bt_request("/status") && wait_event(&mut r) == BtEvent::Failed); // gone: no answer
    bt_init(&tl_tmp(), None, None);
    check!(!bt_present() && !bt_request("/status")); // no script, no stand-in: no helper
}

#[test]
fn test_btapi() {
    run!(json_reads_what_it_should_and_refuses_the_rest);
    run!(helper_answers_are_read_safely);
    run!(the_client_asks_one_question_at_a_time);
    done();
}
