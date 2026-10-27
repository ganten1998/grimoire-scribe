//! bluetooth.h: the Bluetooth helper (kindle-hid-passthrough; docs/SHORTCUTS.md, Bluetooth) as Grimoire
//! runs it. With Settings > Controller's Bluetooth on, Grimoire starts the helper after its first
//! frame (kindle/app/bluetooth.sh start: only one of its own, never someone else's) and stops it when
//! that is turned off; the launcher's restore stops it as Grimoire leaves, crash or not, and gives
//! the Kindle its own Bluetooth jobs back. Pairing goes through the helper's local API
//! (127.0.0.1:8321). Every script run and request is a forked child: bt_tick reaps them, so the UI
//! never waits. Names and addresses from the helper are made safe first (btapi.h).

use crate::prelude::*;


#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BtState {
    /// not started by Grimoire (or stopped)
    #[default]
    Stopped = 0,  // BT_STOPPED
    Starting = 1,  // BT_STARTING
    /// started by Grimoire, answering
    Owned = 2,  // BT_OWNED
    /// another one runs (KOReader's, say): Grimoire uses it, never stops it
    External = 3,  // BT_EXTERNAL
    /// it would not start (bluetooth.sh's log says why)
    Failed = 4,  // BT_FAILED
    Stopping = 5,  // BT_STOPPING
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BtEvent {
    #[default]
    None = 0,  // BT_EV_NONE
    State = 1,  // BT_EV_STATE
    Reply = 2,  // BT_EV_REPLY
    Failed = 3,  // BT_EV_FAILED
}

/// root: Grimoire's data folder. script: kindle/app/bluetooth.sh (the launcher says where), or NULL.
/// standin: "host:port" of a stand-in helper that runs already (tests/fake_bthelper.py), or NULL.
pub fn bt_init(root: &str, script: Option<&str>, standin: Option<&str>) {
    todo!()
}
/// A helper Grimoire could start: unpacked in <root>/bluetooth/, or KHP's /mnt/us/kindle_hid_passthrough.
pub fn bt_present() -> bool {
    todo!()
}
pub fn bt_state() -> BtState {
    todo!()
}
pub fn bt_start() {
    todo!()
}
pub fn bt_stop() {
    todo!()
}
/// A GET of path (with its query) to the helper's API: false while one is under way, or with no API.
pub fn bt_request(path: &str) -> bool {
    todo!()
}
pub fn bt_busy() -> bool {
    todo!()
}
/// Reaps what finished: BT_EV_STATE (bt_state changed), BT_EV_REPLY (*reply: the request's answer)
/// or BT_EV_FAILED (the request got none: the helper is gone or did not answer in 3 s).
pub fn bt_tick(reply: &mut BtReply) -> BtEvent {
    todo!()
}
