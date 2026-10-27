//! btapi.h: the Bluetooth helper's answers (kindle-hid-passthrough's local API, bluetooth.h), read
//! into fixed records: /status, /scan-status and /pair-status. Pure, for tests/test_btapi.c. What
//! comes from the helper (names come over the radio) is kept only as it is safe to show and to put
//! back in a request: names printable ASCII, addresses six hex pairs.

use crate::prelude::*;

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

/// A reply's JSON (len bytes): 0, or -1 when it is not one the helper sends (then *out is empty).
pub fn btapi_parse(json: &[u8], out: &mut BtReply) -> i32 {
    todo!()
}
/// True for "AA:BB:CC:DD:EE:FF" (hex digits, either case).
pub fn btapi_address_ok(addr: &str) -> bool {
    todo!()
}
