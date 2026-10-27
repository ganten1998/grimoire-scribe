//! hidkeys.h: portable external-HID capability/press decoder. No device opens, grabs or key logging.

use crate::prelude::*;

pub const HID_KEY_BYTES: usize = 96;
pub const HID_ABS_BYTES: usize = 8;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HidKind {
    #[default]
    None = 0, // HID_NONE
    Keyboard = 1, // HID_KEYBOARD
    Gamepad = 2,  // HID_GAMEPAD
}

#[derive(Clone, Copy, Debug)]
pub struct HidKeys {
    pub held: [u8; HID_KEY_BYTES],
    pub hat: [i32; 2],
    pub dropped: bool,
}

impl Default for HidKeys {
    fn default() -> Self {
        HidKeys { held: [0; HID_KEY_BYTES], hat: [0; 2], dropped: false }
    }
}

/// up: a release
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HidPress {
    pub code: i32,
    pub mods: i32,
    pub gamepad: bool,
    pub up: bool,
}

pub fn hidkeys_kind(keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) -> HidKind {
    todo!()
}
/// Amazon's virtual input proxy (kindle-input-pt, firmware 5.20+) advertises every key code and
/// delivers no presses of its own: never a keyboard, a controller or the power key.
pub fn hidkeys_proxy(name: &str) -> bool {
    todo!()
}
/// An optional keyboard or controller (hidkeys_kind), never the proxy.
pub fn hidkeys_optional(name: &str, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) -> HidKind {
    todo!()
}
/// A Switch-mode controller read through Linux's generic HID driver, as 8BitDo's Switch mode and a Pro
/// Controller come over Bluetooth ("Pro Controller"): its buttons are numbered in the Pro Controller's
/// simple report order from BTN_A (B, A, Y, X, L, R, ZL, ZR, -, +, the sticks, Home, Capture), so
/// BTN_C and BTN_Z are among them. Nintendo's own driver numbers them by the convention instead, and
/// advertises neither.
pub fn hidkeys_switch(name: &str, keys: &[u8; HID_KEY_BYTES]) -> bool {
    todo!()
}
/// The power key: KEY_POWER on a device that is no pen or touch panel and not the proxy, with few
/// other keys, so neither a keyboard nor the proxy can take its place. Decided apart from
/// hidkeys_kind. -1 when it is not one, else how many keys it has: input_open watches the fewest.
pub fn hidkeys_power(name: &str, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) -> i32 {
    todo!()
}
/// The attach probe: EVIOCGRAB(1)'s result and errno. True when another app (a button mapper)
/// holds the device exclusively, so its presses never reach Grimoire.
pub fn hidkeys_held(grab_result: i32, grab_errno: i32) -> bool {
    todo!()
}
/// A device's name, which can come over the radio, made safe to show: printable ASCII only (a run of
/// other bytes becomes one '?'), spaces trimmed, at most 40 bytes, "?" for none. (raw is bytes: a
/// name that comes over the radio need not be UTF-8.)
pub fn hidkeys_name(out: &mut String, cap: usize, raw: &[u8]) {
    todo!()
}
/// A rate limit: true, and *last set to now, when *last is 0 (never) or at least gap ms ago.
pub fn hidkeys_due(last: &mut u32, now: u32, gap: u32) -> bool {
    todo!()
}
/// Synchronize after attachment, native PIN return or SYN_DROPPED. Held controls do not fire.
/// (held None: nothing held, as the C's NULL.)
pub fn hidkeys_sync(state: &mut HidKeys, held: Option<&[u8; HID_KEY_BYTES]>, hat_x: i32, hat_y: i32) {
    todo!()
}
/// The same after a lost frame, or with native PIN input up (held None: nothing held now): the keys,
/// buttons and hat directions held before and not now are released into ups, at most ups.len() of
/// them; how many. One still held is released later, as any is.
pub fn hidkeys_resync(state: &mut HidKeys, kind: HidKind, held: Option<&[u8; HID_KEY_BYTES]>, hat_x: i32, hat_y: i32, ups: &mut [HidPress]) -> i32 {
    todo!()
}
/// Linux evdev event numbers: what one event does, in out, 0 to 2 of them. A new actionable press,
/// or the release of a key or button held (a hat back to its middle releases its direction; turned
/// straight across it releases it, then presses the other). Repeats, modifiers and power do nothing.
pub fn hidkeys_event(state: &mut HidKeys, kind: HidKind, type_: i32, code: i32, value: i32, out: &mut [HidPress; 2]) -> i32 {
    todo!()
}

/// Analog triggers. Every controller the Kindle's Bluetooth helper makes comes through Linux's
/// generic HID driver (vendor and product 0), where an Xbox-layout one's LT and RT are axes, not
/// buttons: ABS_Z and ABS_RZ beside a right stick on ABS_RX/RY (an Xbox One S's first Bluetooth
/// report, which 8BitDo's Xbox mode copies), or ABS_BRAKE and ABS_GAS (its later report, and pads
/// whose right stick is on Z/RZ). Each is read as a button with hysteresis: pressed past half its
/// range, released at a quarter or under. They are reported as HIDKEYS_LT and HIDKEYS_RT, the last
/// two key codes, which Linux leaves unassigned (its last, BTN_TRIGGER_HAPPY40, is 0x2e7): the
/// generic driver numbers a pad's buttons from BTN_A, so its BTN_TL2 and BTN_TR2 are its ninth and
/// tenth buttons, never its triggers (that 8BitDo's stick presses, a Switch-mode pad's - and +).
pub const HIDKEYS_LT: i32 = 0x2fe;
pub const HIDKEYS_RT: i32 = 0x2ff;
/// ABS_X (0) to ABS_BRAKE (10): the axes the verdict reads.
pub const HIDKEYS_AXES: usize = 11;
/// EVIOCGABS's
#[derive(Clone, Copy, Debug, Default)]
pub struct HidAxis {
    pub minimum: i32,
    pub maximum: i32,
    pub value: i32,
}
/// LT's and RT's axes (0, ABS_X, which never is one: none) and the values past which each is
/// pressed, and at or under which it is released.
#[derive(Clone, Copy, Debug, Default)]
pub struct HidTriggers {
    pub axis: [i32; 2],
    pub press: [i32; 2],
    pub release: [i32; 2],
}
/// A controller's triggers, from the axes it has and their ranges and values now (info, by ABS code;
/// only those it has are read): BRAKE and GAS when it has both; else Z and RZ when it has X, Y, RX
/// and RY too and they are no right stick: narrower than RX and RY, which match X and Y (an Xbox One
/// S: 10-bit triggers beside 16-bit sticks), unless both rest in their middle half; or, the ranges
/// alike, resting outside their middle half while RX and RY rest inside theirs. A trigger whose code
/// is among the controller's own keys is none. False when the values cannot tell yet (the ranges
/// alike, and no report has come: every axis reads 0 until one does, a range's minimum when it
/// starts at 0): ask again after the controller's first report.
pub fn hidkeys_triggers(t: &mut HidTriggers, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES], info: &[HidAxis; HIDKEYS_AXES]) -> bool {
    todo!()
}
/// A resync's fresh state (held: EVIOCGKEY's), with the triggers held now marked in it, as their
/// axes read now (value: LT's, RT's): past the press point, or past the release point while state
/// held them. hidkeys_resync then releases one let go, and holds one pulled without a press.
pub fn hidkeys_trigger_sync(state: &HidKeys, t: &HidTriggers, value: &[i32; 2], held: &mut [u8; HID_KEY_BYTES]) {
    todo!()
}
/// An EV_ABS event: a trigger's press or release in out (0 or 1 of them), as hidkeys_event reports
/// a button's. Any other axis does nothing.
pub fn hidkeys_trigger_event(state: &mut HidKeys, t: &HidTriggers, code: i32, value: i32, out: &mut HidPress) -> i32 {
    todo!()
}

/// The scan for keyboards and controllers that come (input_evdev.c's key_scan) runs while shortcuts
/// are on, every HIDKEYS_SCAN_MS, but never while the pen or a finger is in use (busy: the pen near
/// or touching, a finger down) nor until the pen has been gone HIDKEYS_SCAN_QUIET_MS: nothing reads
/// the pen while it runs. A scan put off runs at the first idle poll after.
pub const HIDKEYS_SCAN_MS: u32 = 5000;
pub const HIDKEYS_SCAN_QUIET_MS: u32 = 1500;
pub fn hidkeys_scan_due(now: u32, last_scan: u32, busy: bool, pen_left: u32) -> bool {
    todo!()
}
/// A /dev/input node as stat() sees it, without opening it: closing an evdev node waits out an RCU
/// grace period (22-96 ms on the Scribe's Linux 4.9), so the scan opens only nodes it has to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HidNode {
    pub rdev: u64,
    pub ino: u64,
    pub ctime_s: i64,
    pub ctime_ns: i64,
}
/// Nodes the scan (or input_open) found to be no keyboard or controller, by path and identity: a
/// node made again (a new inode or change time) is looked at again. The oldest goes when it is full.
pub const HIDKEYS_NONE_MAX: usize = 32;
#[derive(Clone, Copy, Debug, Default)]
pub struct HidNoneEntry {
    pub path: FixedStr<40>,
    pub node: HidNode,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct HidNoneCache {
    pub e: [HidNoneEntry; HIDKEYS_NONE_MAX],
    pub n: i32,
    pub next: i32,
}
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HidNodeVerdict {
    #[default]
    Probe = 0, // HIDNODE_PROBE
    Held = 1, // HIDNODE_HELD
    None = 2, // HIDNODE_NONE
}
/// Whether the scan opens the node at path: never one Grimoire holds open (its device number is in
/// held: the pen, the pressure source, touch, power, orientation, the keyboards; 0s are ignored), nor
/// one found no keyboard as it is now.
pub fn hidkeys_node(c: &HidNoneCache, held: &[u64], path: &str, n: &HidNode) -> HidNodeVerdict {
    todo!()
}
/// Remembers that the node at path is no keyboard or controller (a path too long to keep is not kept).
pub fn hidkeys_node_none(c: &mut HidNoneCache, path: &str, n: &HidNode) {
    todo!()
}
