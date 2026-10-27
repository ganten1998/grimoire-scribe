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

fn bit(bytes: &[u8], code: i32) -> bool {
    (bytes[(code / 8) as usize] >> (code % 8)) & 1 != 0
}
fn set(bytes: &mut [u8], code: i32, value: bool) {
    if value {
        bytes[(code / 8) as usize] |= 1u8 << (code % 8);
    } else {
        bytes[(code / 8) as usize] &= !(1u8 << (code % 8));
    }
}
fn power(code: i32) -> bool {
    code == 116 || code == 142 || code == 143
}
/// BTN_TOOL_PEN, BTN_TOOL_RUBBER, BTN_TOOL_FINGER, BTN_TOUCH or ABS_MT_POSITION_X.
fn digitizer(keys: &[u8], axes: &[u8]) -> bool {
    bit(keys, 320) || bit(keys, 321) || bit(keys, 325) || bit(keys, 330) || bit(axes, 53)
}
pub fn hidkeys_kind(keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) -> HidKind {
    // Reject pen/touch interfaces before inspecting their buttons. An accelerometer's ABS_X/Y
    // is not a joystick; a mouse's BTN_LEFT/RIGHT is not a controller either.
    if digitizer(keys, axes) {
        return HidKind::None;
    }
    for code in 288..=319 {
        if bit(keys, code) {
            return HidKind::Gamepad;
        }
    }
    for code in 544..=547 {
        if bit(keys, code) {
            return HidKind::Gamepad;
        }
    }
    if bit(axes, 16) || bit(axes, 17) {
        return HidKind::Gamepad;
    }
    // Includes sparse page turners and media remotes, not just full A/Z/Space keyboards.
    for code in 1..256 {
        if !power(code) && bit(keys, code) {
            return HidKind::Keyboard;
        }
    }
    HidKind::None
}
/// Amazon's virtual input proxy (kindle-input-pt, firmware 5.20+) advertises every key code and
/// delivers no presses of its own: never a keyboard, a controller or the power key.
pub fn hidkeys_proxy(name: &str) -> bool {
    name == "kindle-input-pt"
}
/// An optional keyboard or controller (hidkeys_kind), never the proxy.
pub fn hidkeys_optional(name: &str, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) -> HidKind {
    if hidkeys_proxy(name) {
        HidKind::None
    } else {
        hidkeys_kind(keys, axes)
    }
}
/// A Switch-mode controller read through Linux's generic HID driver, as 8BitDo's Switch mode and a Pro
/// Controller come over Bluetooth ("Pro Controller"): its buttons are numbered in the Pro Controller's
/// simple report order from BTN_A (B, A, Y, X, L, R, ZL, ZR, -, +, the sticks, Home, Capture), so
/// BTN_C and BTN_Z are among them. Nintendo's own driver numbers them by the convention instead, and
/// advertises neither.
pub fn hidkeys_switch(name: &str, keys: &[u8; HID_KEY_BYTES]) -> bool {
    name.contains("Pro Controller") && bit(keys, 306) && bit(keys, 309)
}
/// The power key: KEY_POWER on a device that is no pen or touch panel and not the proxy, with few
/// other keys, so neither a keyboard nor the proxy can take its place. Decided apart from
/// hidkeys_kind. -1 when it is not one, else how many keys it has: input_open watches the fewest.
pub fn hidkeys_power(name: &str, keys: &[u8; HID_KEY_BYTES], axes: &[u8; HID_ABS_BYTES]) -> i32 {
    // The Scribe's own key (bd71828-pwrkey) has KEY_POWER alone; the proxy has all 767 codes.
    if hidkeys_proxy(name) || !bit(keys, 116) || digitizer(keys, axes) {
        return -1;
    }
    let mut n = 0;
    for code in 1..(HID_KEY_BYTES * 8) as i32 {
        n += bit(keys, code) as i32;
    }
    if n <= 16 {
        n
    } else {
        -1
    }
}
/// The attach probe: EVIOCGRAB(1)'s result and errno. True when another app (a button mapper)
/// holds the device exclusively, so its presses never reach Grimoire.
pub fn hidkeys_held(grab_result: i32, grab_errno: i32) -> bool {
    grab_result != 0 && grab_errno == libc::EBUSY
}
/// A device's name, which can come over the radio, made safe to show: printable ASCII only (a run of
/// other bytes becomes one '?'), spaces trimmed, at most 40 bytes, "?" for none. (raw is bytes: a
/// name that comes over the radio need not be UTF-8.)
pub fn hidkeys_name(out: &mut String, cap: usize, raw: &[u8]) {
    if cap == 0 {
        return;
    }
    out.clear();
    let max = if cap - 1 < 40 { cap - 1 } else { 40 };
    let mut other = false; // inside a run of bytes that are not printable ASCII
    // (a C string ends at its first NUL)
    let raw = &raw[..raw.iter().position(|&b| b == 0).unwrap_or(raw.len())];
    let mut p = 0;
    while p < raw.len() && raw[p] == b' ' {
        p += 1;
    }
    while p < raw.len() && out.len() < max {
        let c = raw[p];
        if (0x20..0x7f).contains(&c) {
            out.push(c as char);
            other = false;
        } else if !other {
            out.push('?');
            other = true;
        }
        p += 1;
    }
    while out.ends_with(' ') {
        out.pop();
    }
    if out.is_empty() && max != 0 {
        out.push('?');
    }
}
/// A rate limit: true, and *last set to now, when *last is 0 (never) or at least gap ms ago.
pub fn hidkeys_due(last: &mut u32, now: u32, gap: u32) -> bool {
    if *last != 0 && now.wrapping_sub(*last) < gap {
        return false;
    }
    *last = if now != 0 { now } else { 1 };
    true
}
/// Synchronize after attachment, native PIN return or SYN_DROPPED. Held controls do not fire.
/// (held None: nothing held, as the C's NULL.)
pub fn hidkeys_sync(state: &mut HidKeys, held: Option<&[u8; HID_KEY_BYTES]>, hat_x: i32, hat_y: i32) {
    *state = HidKeys::default();
    if let Some(h) = held {
        state.held = *h;
    }
    state.hat[0] = if (-1..=1).contains(&hat_x) { hat_x } else { 0 };
    state.hat[1] = if (-1..=1).contains(&hat_y) { hat_y } else { 0 };
}
fn mods(state: &HidKeys) -> i32 {
    (if bit(&state.held, 29) || bit(&state.held, 97) { KEYMOD_CTRL } else { 0 })
        | (if bit(&state.held, 42) || bit(&state.held, 54) { KEYMOD_SHIFT } else { 0 })
        | (if bit(&state.held, 56) || bit(&state.held, 100) { KEYMOD_ALT } else { 0 })
        | (if bit(&state.held, 125) || bit(&state.held, 126) { KEYMOD_META } else { 0 })
}
fn modifier(code: i32) -> bool {
    code == 29 || code == 97 || code == 42 || code == 54 || code == 56 || code == 100 || code == 125 || code == 126
}
fn key(state: &HidKeys, kind: HidKind, code: i32, up: bool) -> HidPress {
    HidPress { code, mods: mods(state), gamepad: kind == HidKind::Gamepad && code >= 256, up }
}
/// A hat direction as the digital D-pads' evdev codes: UP/DOWN/LEFT/RIGHT.
fn hat(axis: i32, value: i32, up: bool) -> HidPress {
    let code = if axis != 0 {
        if value < 0 { 544 } else { 545 }
    } else if value < 0 {
        546
    } else {
        547
    };
    HidPress { code, mods: 0, gamepad: true, up }
}
/// The same after a lost frame, or with native PIN input up (held None: nothing held now): the keys,
/// buttons and hat directions held before and not now are released into ups, at most ups.len() of
/// them; how many. One still held is released later, as any is.
pub fn hidkeys_resync(state: &mut HidKeys, kind: HidKind, held: Option<&[u8; HID_KEY_BYTES]>, hat_x: i32, hat_y: i32, ups: &mut [HidPress]) -> i32 {
    let cap = ups.len();
    let mut n = 0usize;
    let now = [hat_x, hat_y];
    for code in 1..(HID_KEY_BYTES * 8) as i32 {
        if n >= cap {
            break;
        }
        if bit(&state.held, code) && !held.is_some_and(|h| bit(h, code)) && !modifier(code) && !power(code) {
            ups[n] = key(state, kind, code, true);
            n += 1;
        }
    }
    for (axis, &hat_now) in now.iter().enumerate() {
        let v = if (-1..=1).contains(&hat_now) { hat_now } else { 0 };
        if kind == HidKind::Gamepad && state.hat[axis] != 0 && state.hat[axis] != v && n < cap {
            ups[n] = hat(axis as i32, state.hat[axis], true);
            n += 1;
        }
    }
    hidkeys_sync(state, held, hat_x, hat_y);
    n as i32
}
/// Linux evdev event numbers: what one event does, in out, 0 to 2 of them. A new actionable press,
/// or the release of a key or button held (a hat back to its middle releases its direction; turned
/// straight across it releases it, then presses the other). Repeats, modifiers and power do nothing.
pub fn hidkeys_event(state: &mut HidKeys, kind: HidKind, type_: i32, code: i32, value: i32, out: &mut [HidPress; 2]) -> i32 {
    if type_ == 0 && code == 3 {
        // EV_SYN / SYN_DROPPED: caller must obtain a fresh device state
        state.dropped = true;
        return 0;
    }
    if state.dropped || kind == HidKind::None {
        return 0;
    }
    if type_ == 3 && kind == HidKind::Gamepad && (code == 16 || code == 17) {
        if !(-1..=1).contains(&value) {
            return 0;
        }
        let axis = code - 16;
        let previous = state.hat[axis as usize];
        let mut n = 0usize;
        state.hat[axis as usize] = value;
        if value == previous {
            return 0;
        }
        if previous != 0 {
            out[n] = hat(axis, previous, true);
            n += 1;
        }
        if value != 0 {
            out[n] = hat(axis, value, false);
            n += 1;
        }
        return n as i32;
    }
    if type_ != 1 || code <= 0 || code >= (HID_KEY_BYTES * 8) as i32 || power(code) {
        return 0;
    }
    if value == 0 {
        let was = bit(&state.held, code);
        set(&mut state.held, code, false);
        if !was || modifier(code) {
            return 0;
        }
        out[0] = key(state, kind, code, true);
        return 1;
    }
    if value != 1 || bit(&state.held, code) {
        return 0;
    }
    set(&mut state.held, code, true);
    if modifier(code) {
        return 0;
    }
    out[0] = key(state, kind, code, false);
    1
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
    *t = HidTriggers::default();
    let mut codes = [0usize; 2];
    let mut decided = true;
    if bit(axes, 10) && bit(axes, 9) {
        // ABS_BRAKE (LT), ABS_GAS (RT)
        codes = [10, 9];
    } else if bit(axes, 0) && bit(axes, 1) && bit(axes, 2) && bit(axes, 3) && bit(axes, 4) && bit(axes, 5) {
        let (x, y, z, rx, ry, rz) = (&info[0], &info[1], &info[2], &info[3], &info[4], &info[5]);
        let reported = x.value != x.minimum || y.value != y.minimum;
        let narrower = span(z) < span(rx) && span(rz) < span(ry) && span(rx) == span(x) && span(ry) == span(y);
        let triggers = if narrower {
            !(reported && middle(z) && middle(rz))
        } else {
            reported && middle(rx) && middle(ry) && !middle(z) && !middle(rz)
        };
        if triggers {
            codes = [2, 5]; // ABS_Z (LT), ABS_RZ (RT)
        }
        decided = narrower || reported;
    }
    for i in 0..2usize {
        let a = &info[codes[i]];
        if codes[i] == 0 || span(a) <= 0 || bit(keys, HIDKEYS_LT + i as i32) {
            continue;
        }
        t.axis[i] = codes[i] as i32;
        t.press[i] = (a.minimum as i64 + span(a) / 2) as i32;
        t.release[i] = (a.minimum as i64 + span(a) / 4) as i32;
    }
    decided
}
fn span(a: &HidAxis) -> i64 {
    a.maximum as i64 - a.minimum as i64
}
/// In the middle half of its range: where a stick rests, and a trigger never does.
fn middle(a: &HidAxis) -> bool {
    let s = span(a);
    let v = a.value as i64 - a.minimum as i64;
    s > 0 && v >= s / 4 && v <= s - s / 4
}
fn trigger_held(t: &HidTriggers, i: usize, value: i32, was: bool) -> bool {
    value > if was { t.release[i] } else { t.press[i] }
}
/// A resync's fresh state (held: EVIOCGKEY's), with the triggers held now marked in it, as their
/// axes read now (value: LT's, RT's): past the press point, or past the release point while state
/// held them. hidkeys_resync then releases one let go, and holds one pulled without a press.
pub fn hidkeys_trigger_sync(state: &HidKeys, t: &HidTriggers, value: &[i32; 2], held: &mut [u8; HID_KEY_BYTES]) {
    for (i, &v) in value.iter().enumerate() {
        if t.axis[i] != 0 && trigger_held(t, i, v, bit(&state.held, HIDKEYS_LT + i as i32)) {
            set(held, HIDKEYS_LT + i as i32, true);
        }
    }
}
/// An EV_ABS event: a trigger's press or release in out (0 or 1 of them), as hidkeys_event reports
/// a button's. Any other axis does nothing.
pub fn hidkeys_trigger_event(state: &mut HidKeys, t: &HidTriggers, code: i32, value: i32, out: &mut HidPress) -> i32 {
    if state.dropped {
        return 0;
    }
    for i in 0..2usize {
        if t.axis[i] == 0 || code != t.axis[i] {
            continue;
        }
        let c = HIDKEYS_LT + i as i32;
        let was = bit(&state.held, c);
        let now = trigger_held(t, i, value, was);
        if now == was {
            return 0;
        }
        set(&mut state.held, c, now);
        *out = key(state, HidKind::Gamepad, c, !now);
        return 1;
    }
    0
}

/// The scan for keyboards and controllers that come (input_evdev.c's key_scan) runs while shortcuts
/// are on, every HIDKEYS_SCAN_MS, but never while the pen or a finger is in use (busy: the pen near
/// or touching, a finger down) nor until the pen has been gone HIDKEYS_SCAN_QUIET_MS: nothing reads
/// the pen while it runs. A scan put off runs at the first idle poll after.
pub const HIDKEYS_SCAN_MS: u32 = 5000;
pub const HIDKEYS_SCAN_QUIET_MS: u32 = 1500;
pub fn hidkeys_scan_due(now: u32, last_scan: u32, busy: bool, pen_left: u32) -> bool {
    now.wrapping_sub(last_scan) >= HIDKEYS_SCAN_MS && !busy && now.wrapping_sub(pen_left) >= HIDKEYS_SCAN_QUIET_MS
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
    for &h in held {
        if h != 0 && h == n.rdev {
            return HidNodeVerdict::Held;
        }
    }
    for i in 0..c.n as usize {
        if c.e[i].path.as_bytes() == path.as_bytes() {
            // (same_node: the identity stat() gives: device number, inode, change time)
            return if c.e[i].node == *n { HidNodeVerdict::None } else { HidNodeVerdict::Probe };
        }
    }
    HidNodeVerdict::Probe
}
/// Remembers that the node at path is no keyboard or controller (a path too long to keep is not kept).
pub fn hidkeys_node_none(c: &mut HidNoneCache, path: &str, n: &HidNode) {
    if path.len() >= FixedStr::<40>::CAP {
        return;
    }
    let mut i = 0i32;
    while i < c.n && c.e[i as usize].path.as_bytes() != path.as_bytes() {
        i += 1; // a node made again replaces its entry
    }
    if i == c.n {
        if (c.n as usize) < HIDKEYS_NONE_MAX {
            i = c.n;
            c.n += 1;
        } else {
            i = c.next;
            c.next = (c.next + 1) % HIDKEYS_NONE_MAX as i32;
        }
    }
    c.e[i as usize].path.set(path);
    c.e[i as usize].node = *n;
}
