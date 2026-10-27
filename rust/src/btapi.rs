//! btapi.h: the Bluetooth helper's answers (kindle-hid-passthrough's local API, bluetooth.h), read
//! into fixed records: /status, /scan-status and /pair-status. Pure, for tests/test_btapi.c. What
//! comes from the helper (names come over the radio) is kept only as it is safe to show and to put
//! back in a request: names printable ASCII, addresses six hex pairs.

use crate::prelude::*;
use std::cell::RefCell;

const TOKENS: usize = 512;

pub const BT_NAME_LEN: usize = 41;
/// "AA:BB:CC:DD:EE:FF" and its end
pub const BT_ADDR_LEN: usize = 18;
pub const BT_DEVICES: usize = 8;

#[derive(Clone, Copy, Debug, Default)]
pub struct BtDevice {
    pub address: FixedStr<BT_ADDR_LEN>,
    pub name: FixedStr<BT_NAME_LEN>,
    /// protocol "classic" (else BLE)
    pub classic: bool,
    /// in /status: a live session with its input node made (hid_ready)
    pub connected: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BtReply {
    /// the helper said ok
    pub ok: bool,
    /// its error, when not ok (printable ASCII)
    pub error: FixedStr<80>,
    pub running: bool,
    pub scanning: bool,
    pub pairing: bool,
    pub version: FixedStr<24>,
    /// /status: the paired ones (connected marked); /scan-status: found
    pub dev: [BtDevice; BT_DEVICES],
    pub ndev: i32,
}

/// True for "AA:BB:CC:DD:EE:FF" (hex digits, either case).
pub fn btapi_address_ok(addr: &str) -> bool {
    // (A C string ends at its first NUL.)
    let a = addr.as_bytes();
    let a = &a[..a.iter().position(|&b| b == 0).unwrap_or(a.len())];
    if a.len() != 17 {
        return false;
    }
    for (i, &c) in a.iter().enumerate() {
        let hex = c.is_ascii_hexdigit();
        if if i % 3 == 2 { c != b':' } else { !hex } {
            return false;
        }
    }
    true
}

/// A string field, made printable ASCII (a run of anything else one '?'), trimmed; "" for none.
/// N is the C buffer's size: at most N - 1 bytes are kept.
fn text_of<const N: usize>(json: &[u8], t: &[JsonTok], obj: i32, key: &str) -> FixedStr<N> {
    let cap = N;
    let mut raw: Vec<u8> = Vec::new();
    let v = json_get(json, t, obj, key);
    if v < 0 || !json_string(json, &t[v as usize], &mut raw, 256) {
        return FixedStr::default();
    }
    let mut out: Vec<u8> = Vec::new();
    let mut other = false;
    for &p in &raw {
        if p == 0 || out.len() + 1 >= cap {
            break;
        }
        if (0x20..0x7F).contains(&p) {
            out.push(p);
            other = false;
        } else if !other {
            out.push(b'?');
            other = true;
        }
    }
    while out.last() == Some(&b' ') {
        out.pop();
    }
    let lead = out.iter().take_while(|&&b| b == b' ').count();
    let mut f = FixedStr::<N>::default();
    f.set_bytes(&out[lead..]);
    f
}

fn flag(json: &[u8], t: &[JsonTok], obj: i32, key: &str) -> bool {
    let mut v = false;
    let at = json_get(json, t, obj, key);
    at >= 0 && json_bool(&t[at as usize], &mut v) && v
}

/// The devices of array key: those with a sound address, at most BT_DEVICES.
fn devices(json: &[u8], t: &[JsonTok], obj: i32, key: &str, out: &mut BtReply) {
    let arr = json_get(json, t, obj, key);
    let mut i = 0;
    while arr >= 0 && (out.ndev as usize) < BT_DEVICES {
        let d = json_item(t, arr, i);
        i += 1;
        if d < 0 {
            break;
        }
        if t[d as usize].type_ != JsonType::Object {
            continue;
        }
        let address = text_of::<BT_ADDR_LEN>(json, t, d, "address");
        if !btapi_address_ok(address.as_str()) {
            continue;
        }
        let name = text_of::<BT_NAME_LEN>(json, t, d, "name");
        let proto = text_of::<16>(json, t, d, "protocol");
        out.dev[out.ndev as usize] = BtDevice { address, name, classic: proto == "classic", connected: false };
        out.ndev += 1;
    }
}

thread_local! {
    // (The UI thread's alone.)
    static TOKS: RefCell<[JsonTok; TOKENS]> = const { RefCell::new([JsonTok { type_: JsonType::None, start: 0, end: 0, size: 0, next: 0 }; TOKENS]) };
}

/// A reply's JSON (len bytes): 0, or -1 when it is not one the helper sends (then *out is empty).
pub fn btapi_parse(json: &[u8], out: &mut BtReply) -> i32 {
    TOKS.with(|t| {
        let mut t = t.borrow_mut();
        *out = BtReply::default();
        let n = json_parse(json, &mut t[..]);
        if n <= 0 || t[0].type_ != JsonType::Object {
            return -1;
        }
        let t = &t[..n as usize];
        out.ok = flag(json, t, 0, "ok");
        out.error = text_of::<80>(json, t, 0, "error");
        out.version = text_of::<24>(json, t, 0, "version");
        out.running = flag(json, t, 0, "daemon_running");
        out.scanning = flag(json, t, 0, "scanning");
        out.pairing = flag(json, t, 0, "pairing");
        devices(json, t, 0, "devices", out);
        // Live sessions mark the paired devices they are (and add one not listed, as connected).
        let conns = json_get(json, t, 0, "connections");
        if conns >= 0 {
            let mut i = 0;
            loop {
                let c = json_item(t, conns, i);
                i += 1;
                if c < 0 {
                    break;
                }
                let addr = text_of::<BT_ADDR_LEN>(json, t, c, "address");
                if !btapi_address_ok(addr.as_str()) || !flag(json, t, c, "hid_ready") {
                    continue;
                }
                let mut k = 0usize;
                while k < out.ndev as usize && !out.dev[k].address.as_bytes().eq_ignore_ascii_case(addr.as_bytes()) {
                    k += 1;
                }
                if k == out.ndev as usize {
                    if k >= BT_DEVICES {
                        continue;
                    }
                    let name = text_of::<BT_NAME_LEN>(json, t, c, "name");
                    out.dev[k] = BtDevice { address: addr, name, classic: false, connected: false };
                    out.ndev += 1;
                }
                out.dev[k].connected = true;
            }
        }
        0
    })
}
