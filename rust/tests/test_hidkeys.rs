// test_hidkeys.c: the external keyboard/controller decoder (hidkeys.c): capabilities, presses, hats,
// resyncs, the power key's device, the analog triggers, and the scan's node bookkeeping.
//
// The C passes NULL for a name in three places (hidkeys_switch, hidkeys_power, hidkeys_optional) and
// for hidkeys_name's raw text; the Rust names are &str and bytes, so an empty one stands in: it is no
// "Pro Controller" and no "kindle-input-pt" either, and prints as "?".
// Index loops and range comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn add(bits: &mut [u8], code: i32) {
    bits[(code / 8) as usize] |= 1u8 << (code % 8);
}
fn capabilities() {
    let mut keys = [0u8; HID_KEY_BYTES];
    let mut axes = [0u8; HID_ABS_BYTES];
    add(&mut keys, 116);
    check!(hidkeys_kind(&keys, &axes) == HidKind::None); // native power
    add(&mut keys, 114);
    check!(hidkeys_kind(&keys, &axes) == HidKind::Keyboard); // sparse media remote
    add(&mut keys, 320);
    check!(hidkeys_kind(&keys, &axes) == HidKind::None); // never a pen, even with keys
    keys = [0; HID_KEY_BYTES];
    add(&mut keys, 308);
    check!(hidkeys_kind(&keys, &axes) == HidKind::Gamepad); // not every controller advertises button 304
    add(&mut axes, 53);
    check!(hidkeys_kind(&keys, &axes) == HidKind::None); // touch interface
    keys = [0; HID_KEY_BYTES];
    axes = [0; HID_ABS_BYTES];
    add(&mut axes, 0);
    add(&mut axes, 1);
    check!(hidkeys_kind(&keys, &axes) == HidKind::None); // accelerometer
    add(&mut keys, 272);
    check!(hidkeys_kind(&keys, &axes) == HidKind::None); // mouse
    add(&mut axes, 16);
    check!(hidkeys_kind(&keys, &axes) == HidKind::Gamepad);
}
// A Switch-mode controller through Linux's generic driver: "Pro Controller", sixteen buttons from
// BTN_A (BTN_C and BTN_Z among them). Another pad with the same buttons, or a Pro Controller under
// Nintendo's driver (the convention's buttons, no BTN_C or BTN_Z), is not one.
fn switch_layout() {
    let mut generic = [0u8; HID_KEY_BYTES];
    let mut nintendo = [0u8; HID_KEY_BYTES];
    for code in 304..=319 {
        add(&mut generic, code);
    }
    check!(hidkeys_switch("Pro Controller", &generic));
    check!(!hidkeys_switch("Xbox Wireless Controller", &generic));
    check!(!hidkeys_switch("", &generic)); // (NULL)
    for code in 304..=317 {
        if code != 306 && code != 309 {
            add(&mut nintendo, code);
        }
    }
    check!(!hidkeys_switch("Nintendo Switch Pro Controller", &nintendo));
}
fn presses_and_modifiers() {
    let mut s = HidKeys::default();
    let mut p = [HidPress::default(); 2];
    hidkeys_sync(&mut s, None, 0, 0);
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 29, 1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 44, 1, &mut p) == 1 && p[0].code == 44 && p[0].mods == KEYMOD_CTRL && !p[0].up);
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 44, 1, &mut p) == 0); // duplicate down
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 44, 2, &mut p) == 0); // auto-repeat
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 44, 0, &mut p) == 1 && p[0].code == 44 && p[0].up); // its release
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 44, 0, &mut p) == 0); // released again: nothing
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 97, 1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 29, 0, &mut p) == 0); // right Ctrl remains held; no modifier's release
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 44, 1, &mut p) == 1 && p[0].mods == KEYMOD_CTRL);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 116, 1, &mut p) == 0); // external power never quits
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 116, 0, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 768, 1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 114, 1, &mut p) == 1 && !p[0].gamepad); // mixed HID media key
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 1, &mut p) == 1 && p[0].gamepad && !p[0].up);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 0, &mut p) == 1 && p[0].code == 304 && p[0].gamepad && p[0].up);
}
fn hats_drop_and_pin_resync() {
    let mut s = HidKeys::default();
    let mut p = [HidPress::default(); 2];
    hidkeys_sync(&mut s, None, 0, 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, -1, &mut p) == 1 && p[0].code == 546 && !p[0].up);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, -1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, 0, &mut p) == 1 && p[0].code == 546 && p[0].up); // back to the middle
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, 1, &mut p) == 1 && p[0].code == 547);
    // Straight across, with no middle between: the release of the one, then the press of the other.
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, -1, &mut p) == 2 && p[0].code == 547 && p[0].up && p[1].code == 546 && !p[1].up);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 17, -1, &mut p) == 1 && p[0].code == 544);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 0, 200, &mut p) == 0); // sticks are not hats
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, 2, &mut p) == 0); // invalid hat value
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 0, 3, 0, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 0, 0, 0, &mut p) == 0); // SYN_REPORT alone cannot trust stale state
    let mut held = [0u8; HID_KEY_BYTES];
    add(&mut held, 304);
    hidkeys_sync(&mut s, Some(&held), 1, 0); // held while PIN was up, or after a lost frame
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, 1, &mut p) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 0, &mut p) == 1 && p[0].up); // its release is one (nothing waits for it)
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 1, &mut p) == 1 && !p[0].up);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, 0, &mut p) == 1 && p[0].code == 547 && p[0].up);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 16, 1, &mut p) == 1 && !p[0].up);
}
fn resync_releases_what_went() {
    let mut s = HidKeys::default();
    let mut p = [HidPress::default(); 2];
    let mut ups = [HidPress::default(); 8];
    hidkeys_sync(&mut s, None, 0, 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 312, 1, &mut p) == 1); // L2 held (Erase while held, say)
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 1, &mut p) == 1);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 29, 1, &mut p) == 0); // a modifier: never released into ups
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 17, 1, &mut p) == 1 && p[0].code == 545);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 0, 3, 0, &mut p) == 0); // SYN_DROPPED: the frames in between are lost
    // A fresh state with South still held, L2 and the hat let go: their releases, South's later.
    let mut held = [0u8; HID_KEY_BYTES];
    add(&mut held, 304);
    let n = hidkeys_resync(&mut s, HidKind::Gamepad, Some(&held), 0, 0, &mut ups[..8]);
    check!(n == 2 && ups[0].code == 312 && ups[0].up && ups[0].gamepad && ups[1].code == 545 && ups[1].up);
    check!(!s.dropped);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 312, 0, &mut p) == 0); // gone already
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 304, 0, &mut p) == 1 && p[0].code == 304 && p[0].up);
    // Native PIN input up (held NULL): everything held is let go; cap bounds how many.
    check!(hidkeys_event(&mut s, HidKind::Keyboard, 1, 30, 1, &mut p) == 1 && hidkeys_event(&mut s, HidKind::Keyboard, 1, 31, 1, &mut p) == 1);
    let n = hidkeys_resync(&mut s, HidKind::Keyboard, None, 0, 0, &mut ups[..1]);
    check!(n == 1 && ups[0].code == 30 && ups[0].up && !ups[0].gamepad);
    check!(hidkeys_resync(&mut s, HidKind::Keyboard, None, 0, 0, &mut ups[..8]) == 0); // (state already empty)
    // Held at attach (the first sync): no release on that sync.
    hidkeys_sync(&mut s, None, 0, 0);
    check!(hidkeys_resync(&mut s, HidKind::Gamepad, Some(&held), -1, 0, &mut ups[..8]) == 0);
    check!(hidkeys_resync(&mut s, HidKind::Gamepad, Some(&held), 7, 0, &mut ups[..8]) == 1 && ups[0].code == 546); // (7 reads as 0)
}
// A sysfs capability bitmap (/sys/class/input/eventN/device/capabilities/key or abs) as the
// Kindle prints it: hex words of 32 bits (an armhf long), the highest first.
fn sysfs_bits(text: &str, bits: &mut [u8]) {
    let nbytes = bits.len();
    let mut words: Vec<u64> = Vec::new();
    bits.fill(0);
    // strtoul(p, &end, 16) over the text: words until one does not parse (at most 64)
    let t = text.as_bytes();
    let mut p = 0;
    while p < t.len() && words.len() < 64 {
        let mut q = p;
        while q < t.len() && t[q].is_ascii_whitespace() {
            q += 1;
        }
        let digits = q;
        let mut v = 0u64;
        while q < t.len() && t[q].is_ascii_hexdigit() {
            v = v * 16 + (t[q] as char).to_digit(16).unwrap() as u64;
            q += 1;
        }
        if q == digits {
            break;
        }
        words.push(v);
        p = q;
    }
    let n = words.len();
    for w in 0..n {
        for b in 0..32 {
            if (words[n - 1 - w] >> b) & 1 != 0 && (w * 32 + b) / 8 < nbytes {
                add(bits, (w * 32 + b) as i32);
            }
        }
    }
}
// The Kindle Scribe 2024 on firmware 5.20.1.0.2 (KindleLab evidence dpre-2026-10-25, input-caps.txt).
const EVERY_KEY: &str = concat!(
    "ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ",
    "ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ",
    "ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff fffffffe"
);
const K_SCRIBE_520: [(&str, &str, &str); 7] = [
    ("bd71828-pwrkey", "100000 0 0 0", "0"),
    ("bma4xy_acc", "0", "100 7"),
    ("bma4xy_feature", "0", "1000000"),
    ("WacomDigitizer", "1c03 0 0 0 0 0 0 0 0 0 0", "f000003"),
    ("pt_mt", "0", "ee18000 0"),
    ("kindle-input-pt", EVERY_KEY, "0"),
    ("stylus-custom", "1c03 0 0 0 0 0 0 0 0 0 0", "f000003"),
];
fn power_key_on_firmware_5_20() {
    let mut keys = [0u8; HID_KEY_BYTES];
    let mut axes = [0u8; HID_ABS_BYTES];
    let (mut powers, mut best, mut rank) = (0, -1i32, 0);
    // input_open meets them in readdir's order, the proxy (event5) before the power key (event0).
    for i in (0..K_SCRIBE_520.len() as i32).rev() {
        let (name, key, abs) = K_SCRIBE_520[i as usize];
        sysfs_bits(key, &mut keys);
        sysfs_bits(abs, &mut axes);
        let r = hidkeys_power(name, &keys, &axes);
        if r >= 0 {
            powers += 1;
            if best < 0 || r < rank {
                best = i;
                rank = r;
            }
        }
        // No device of the Kindle's own is an optional keyboard or controller.
        check!(hidkeys_optional(name, &keys, &axes) == HidKind::None);
    }
    check!(powers == 1 && best == 0 && rank == 1);
    sysfs_bits(EVERY_KEY, &mut keys);
    axes = [0; HID_ABS_BYTES];
    check!(hidkeys_kind(&keys, &axes) == HidKind::None); // BTN_TOOL_PEN and BTN_TOUCH are among every key
    // A proxy without the pen and touch bits, as a later firmware may make it, is still nothing.
    keys[320 / 8] = 0;
    keys[330 / 8] = 0;
    check!(hidkeys_kind(&keys, &axes) == HidKind::Gamepad); // by its bits alone: the gamepad buttons
    check!(hidkeys_optional("kindle-input-pt", &keys, &axes) == HidKind::None);
    check!(hidkeys_power("kindle-input-pt", &keys, &axes) == -1);
    // Under any other name, a device with every key is never a power key.
    check!(hidkeys_power("virtual-keys", &keys, &axes) == -1);
    // A remote with a power key loses to the Kindle's own, which has fewer keys.
    keys = [0; HID_KEY_BYTES];
    add(&mut keys, 116);
    add(&mut keys, 114);
    add(&mut keys, 115);
    add(&mut keys, 163);
    add(&mut keys, 165);
    check!(hidkeys_power("BT remote", &keys, &axes) == 5);
    check!(hidkeys_optional("BT remote", &keys, &axes) == HidKind::Keyboard);
    check!(hidkeys_power("", &keys, &axes) == 5 && hidkeys_optional("", &keys, &axes) == HidKind::Keyboard); // (NULL)
}
fn held_names_and_rate() {
    check!(hidkeys_held(-1, libc::EBUSY)); // a button mapper's grab
    check!(!hidkeys_held(0, libc::EBUSY) && !hidkeys_held(-1, libc::EINVAL) && !hidkeys_held(-1, libc::ENODEV));
    let mut n = String::new();
    hidkeys_name(&mut n, 41, b"Pro Controller");
    check!(n == "Pro Controller");
    hidkeys_name(&mut n, 41, b"  8BitDo\x01\x02\xc3\xa9 Stick\n ");
    check!(n == "8BitDo? Stick?");
    hidkeys_name(&mut n, 41, b"0123456789012345678901234567890123456789012345");
    check!(n.len() == 40 && n.starts_with("0123456789"));
    hidkeys_name(&mut n, 41, b"");
    check!(n == "?");
    hidkeys_name(&mut n, 41, b""); // (NULL)
    check!(n == "?");
    let mut small = String::new();
    hidkeys_name(&mut small, 5, b"Keyboard");
    check!(small == "Keyb");
    let mut last = 0u32;
    check!(hidkeys_due(&mut last, 1000, 30000)); // the first one
    check!(!hidkeys_due(&mut last, 1500, 30000) && !hidkeys_due(&mut last, 30999, 30000));
    check!(hidkeys_due(&mut last, 31000, 30000) && last == 31000);
    last = 0;
    check!(hidkeys_due(&mut last, 0, 30000) && last != 0); // at time 0, still remembered
    last = 0xfffff000;
    check!(hidkeys_due(&mut last, 0x00010000, 30000)); // across the clock's wrap
}
// The scan for keyboards and controllers that come held the thread 320-370 ms, every 5 s: long
// enough for the kernel to drop pen events mid-stroke (p28, 2026-10-26).
fn the_scan_waits_for_the_pen_and_fingers() {
    check!(hidkeys_scan_due(6000, 1000, false, 0));
    check!(!hidkeys_scan_due(5999, 1000, false, 0)); // every 5 s, as before
    check!(!hidkeys_scan_due(60000, 1000, true, 0)); // never while the pen or a finger is in use
    check!(!hidkeys_scan_due(6000, 1000, false, 5000)); // nor until the pen has been gone 1.5 s
    check!(hidkeys_scan_due(6500, 1000, false, 5000));
    // Writing from 5.9 s to 9.2 s with polls every 30 ms: no scan mid-stroke, nor while the pen is
    // near; the scan put off runs at the first idle poll, 1.5 s after the pen left.
    let (mut last, mut left, mut first) = (1000u32, 0u32, 0u32);
    let mut during = 0;
    let mut now = 5900u32;
    while now < 20000 {
        let busy = now < 9200;
        if now == 9200 {
            left = now;
        }
        if hidkeys_scan_due(now, last, busy, left) {
            if busy || now.wrapping_sub(left) < 1500 {
                during += 1;
            }
            if first == 0 {
                first = now;
            }
            last = now;
        }
        now += 30;
    }
    check!(during == 0 && first == 10700);
    // Across the clock's wrap.
    check!(hidkeys_scan_due(0x00001000, 0xfffff000, false, 0xffff0000));
    check!(!hidkeys_scan_due(0x00000100, 0xfffff000, false, 0xffff0000));
}
// The Scribe 2 on firmware 5.20.1: event0 the power key, 1 the accelerometer, 2 its orientation
// codes, 3 the raw digitizer, 4 touch, 5 kindle-input-pt, 6 the scaled pen (major 13, minors 64..70).
fn scribe_node(i: u64) -> HidNode {
    HidNode { rdev: (13 << 8) | (64 + i), ino: 100 + i, ctime_s: 1761500000, ctime_ns: 5 }
}
fn the_scan_never_opens_what_grimoire_holds_or_knows() {
    let mut c = HidNoneCache::default();
    let held: [u64; 6] = [scribe_node(6).rdev, scribe_node(3).rdev, scribe_node(4).rdev, scribe_node(0).rdev, scribe_node(2).rdev, 0]; // pen, pressure source, touch, power, orientation, a closed fd
    // input_open looked at the accelerometer and kindle-input-pt, and left them: no keyboards.
    let acc = scribe_node(1);
    let proxy = scribe_node(5);
    hidkeys_node_none(&mut c, "/dev/input/event1", &acc);
    hidkeys_node_none(&mut c, "/dev/input/event5", &proxy);
    let mut opened = 0;
    for i in 0..7u64 {
        let path = format!("/dev/input/event{}", i);
        let n = scribe_node(i);
        let v = hidkeys_node(&c, &held, &path, &n);
        opened += (v == HidNodeVerdict::Probe) as i32;
        check!(v == if i == 1 || i == 5 { HidNodeVerdict::None } else { HidNodeVerdict::Held });
    }
    check!(opened == 0); // the old scan opened all seven, every 5 s
    // A controller that comes is opened once; found to be none, it is not opened again.
    let pad = HidNode { rdev: (13 << 8) | 71, ino: 200, ctime_s: 1761500100, ctime_ns: 7 };
    check!(hidkeys_node(&c, &held, "/dev/input/event7", &pad) == HidNodeVerdict::Probe);
    hidkeys_node_none(&mut c, "/dev/input/event7", &pad);
    check!(hidkeys_node(&c, &held, "/dev/input/event7", &pad) == HidNodeVerdict::None);
    // The node made again (a new device at the same path): a new inode, or a new change time.
    let mut again = pad;
    again.ino = 201;
    check!(hidkeys_node(&c, &held, "/dev/input/event7", &again) == HidNodeVerdict::Probe);
    again = pad;
    again.ctime_ns = 8;
    check!(hidkeys_node(&c, &held, "/dev/input/event7", &again) == HidNodeVerdict::Probe);
    let had = c.n;
    hidkeys_node_none(&mut c, "/dev/input/event7", &again);
    check!(c.n == had && hidkeys_node(&c, &held, "/dev/input/event7", &again) == HidNodeVerdict::None);
    check!(hidkeys_node(&c, &held, "/dev/input/event7", &pad) == HidNodeVerdict::Probe);
    // A device number of 0 (a closed fd's place) holds nothing.
    let zero = HidNode { rdev: 0, ino: 300, ctime_s: 0, ctime_ns: 0 };
    check!(hidkeys_node(&c, &held, "/dev/input/event9", &zero) == HidNodeVerdict::Probe);
    // A path too long to keep is never kept: always opened.
    let longp = format!("/dev/input/by-a-name-longer-than-a-node-path/event{}", 12);
    hidkeys_node_none(&mut c, &longp, &pad);
    check!(hidkeys_node(&c, &held, &longp, &pad) == HidNodeVerdict::Probe);
    // Full: the oldest goes.
    for i in 0..HIDKEYS_NONE_MAX {
        let path = format!("/dev/input/event{}", 20 + i);
        let n = HidNode { rdev: (1000 + i) as u64, ino: i as u64, ctime_s: 0, ctime_ns: 0 };
        hidkeys_node_none(&mut c, &path, &n);
    }
    check!(c.n == HIDKEYS_NONE_MAX as i32);
    check!(hidkeys_node(&c, &held, "/dev/input/event1", &acc) == HidNodeVerdict::Probe);
    let last = HidNode { rdev: (1000 + HIDKEYS_NONE_MAX - 1) as u64, ino: (HIDKEYS_NONE_MAX - 1) as u64, ctime_s: 0, ctime_ns: 0 };
    let path = format!("/dev/input/event{}", 20 + HIDKEYS_NONE_MAX - 1);
    check!(hidkeys_node(&c, &held, &path, &last) == HidNodeVerdict::None);
}
// The controller in KindleLab evidence controller-20261027-014113: an 8BitDo in Xbox mode through
// the Kindle's Bluetooth helper and Linux's generic driver ("Xbox Bluetooth Gamepad", vendor and
// product 0). Ten buttons from BTN_A (BTN_TL2 and BTN_TR2 its ninth and tenth: its stick presses,
// not its triggers), KEY_MENU; axes X, Y, Z, RX, RY, RZ, the hat and ABS_MISC. LT and RT did nothing.
const K_8BITDO_XBOX_KEY: &str = "3ff0000 0 0 0 0 800 0 0 0 0";
const K_8BITDO_XBOX_ABS: &str = "100 3003f";
fn axis(info: &mut [HidAxis; HIDKEYS_AXES], code: usize, minimum: i32, maximum: i32, value: i32) {
    info[code] = HidAxis { minimum, maximum, value };
}
fn bit_of(bytes: &[u8], code: i32) -> bool {
    (bytes[(code / 8) as usize] >> (code % 8)) & 1 != 0
}
fn analog_triggers_are_buttons() {
    let mut keys = [0u8; HID_KEY_BYTES];
    let mut axes = [0u8; HID_ABS_BYTES];
    sysfs_bits(K_8BITDO_XBOX_KEY, &mut keys);
    sysfs_bits(K_8BITDO_XBOX_ABS, &mut axes);
    check!(hidkeys_optional("Xbox Bluetooth Gamepad", &keys, &axes) == HidKind::Gamepad);
    check!(!hidkeys_switch("Xbox Bluetooth Gamepad", &keys));
    check!(bit_of(&keys, 312) && bit_of(&keys, 313)); // its own BTN_TL2 and BTN_TR2
    // An Xbox One S's first Bluetooth report: 16-bit sticks, 10-bit triggers. Attached before its
    // first report (every axis 0): the ranges alone tell.
    let mut info = [HidAxis::default(); HIDKEYS_AXES];
    for c in 0..=5usize {
        axis(&mut info, c, 0, if c == 2 || c == 5 { 1023 } else { 65535 }, 0);
    }
    let mut t = HidTriggers::default();
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info));
    check!(t.axis[0] == 2 && t.axis[1] == 5 && t.press[0] == 511 && t.release[0] == 255);
    // Its sticks at rest, the triggers too: the same.
    for c in 0..=5usize {
        if c != 2 && c != 5 {
            info[c].value = 32768;
        }
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 2 && t.axis[1] == 5);
    // A pull past half is a press, a let-go to a quarter its release; in between it stays as it was.
    let mut s = HidKeys::default();
    let mut p = [HidPress::default(); 2];
    hidkeys_sync(&mut s, None, 0, 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 300, &mut p[0]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 511, &mut p[0]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 512, &mut p[0]) == 1 && p[0].code == HIDKEYS_LT && p[0].gamepad && !p[0].up);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 1023, &mut p[0]) == 0); // held: once
    check!(hidkeys_trigger_event(&mut s, &t, 2, 300, &mut p[0]) == 0); // a hand that eases off a little: still held
    check!(hidkeys_trigger_event(&mut s, &t, 2, 700, &mut p[0]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 255, &mut p[0]) == 1 && p[0].code == HIDKEYS_LT && p[0].gamepad && p[0].up);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 0, &mut p[0]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 400, &mut p[0]) == 0); // under half again: nothing
    check!(hidkeys_trigger_event(&mut s, &t, 5, 1023, &mut p[0]) == 1 && p[0].code == HIDKEYS_RT && !p[0].up);
    // The sticks and the hat are no triggers, and hidkeys_event leaves the trigger axes alone.
    check!(hidkeys_trigger_event(&mut s, &t, 3, 65535, &mut p[0]) == 0 && hidkeys_trigger_event(&mut s, &t, 0, 0, &mut p[0]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 16, 1, &mut p[0]) == 0);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 3, 2, 1023, &mut p) == 0);
    // Its own buttons 312 and 313 are buttons apart from the triggers.
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 312, 1, &mut p) == 1 && p[0].code == 312 && !p[0].up);
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 1, 312, 0, &mut p) == 1 && p[0].code == 312 && p[0].up);
    // A lost frame: RT let go meanwhile is released, LT pulled meanwhile is held with no press.
    check!(hidkeys_event(&mut s, HidKind::Gamepad, 0, 3, 0, &mut p) == 0 && s.dropped);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 1023, &mut p[0]) == 0); // (nothing until the resync)
    let mut held = [0u8; HID_KEY_BYTES];
    let mut ups = [HidPress::default(); 4];
    let mut now = [1023, 0];
    hidkeys_trigger_sync(&s, &t, &now, &mut held);
    check!(hidkeys_resync(&mut s, HidKind::Gamepad, Some(&held), 0, 0, &mut ups[..4]) == 1 && ups[0].code == HIDKEYS_RT && ups[0].up && ups[0].gamepad);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 900, &mut p[0]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 0, &mut p[0]) == 1 && p[0].code == HIDKEYS_LT && p[0].up);
    // A resync keeps one held between the two points, and holds none there that was not.
    check!(hidkeys_trigger_event(&mut s, &t, 5, 600, &mut p[0]) == 1 && !p[0].up);
    held = [0; HID_KEY_BYTES];
    now[0] = 400;
    now[1] = 400;
    hidkeys_trigger_sync(&s, &t, &now, &mut held);
    check!(!bit_of(&held, HIDKEYS_LT) && bit_of(&held, HIDKEYS_RT));
    check!(hidkeys_resync(&mut s, HidKind::Gamepad, Some(&held), 0, 0, &mut ups[..4]) == 0);
    // Native PIN input up (held NULL): a trigger held is let go with the rest.
    check!(hidkeys_resync(&mut s, HidKind::Gamepad, None, 0, 0, &mut ups[..4]) == 1 && ups[0].code == HIDKEYS_RT && ups[0].up);
    // Pulled at attach: held, no press; its let-go is a release (as a button held then).
    hidkeys_sync(&mut s, None, 0, 0);
    held = [0; HID_KEY_BYTES];
    now[0] = 1023;
    now[1] = 0;
    hidkeys_trigger_sync(&s, &t, &now, &mut held);
    check!(hidkeys_resync(&mut s, HidKind::Gamepad, Some(&held), 0, 0, &mut ups[..4]) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 0, &mut p[0]) == 1 && p[0].code == HIDKEYS_LT && p[0].up);
    // The names the files and the Buttons sheet give them.
    let mut name = String::new();
    shortcut_pad_name(HIDKEYS_LT, &mut name, 16);
    check!(name == "LT");
    shortcut_pad_label(HIDKEYS_RT, false, &mut name, 16);
    check!(name == "RT");
}
// A right stick is never a trigger: its middle, where it rests, is past a trigger's press point.
fn a_right_stick_is_no_trigger() {
    let mut keys = [0u8; HID_KEY_BYTES];
    let mut axes = [0u8; HID_ABS_BYTES];
    for c in 304..=317 {
        add(&mut keys, c);
    }
    for c in 0..=5 {
        add(&mut axes, c);
    }
    add(&mut axes, 16);
    add(&mut axes, 17);
    let mut info = [HidAxis::default(); HIDKEYS_AXES];
    let mut t = HidTriggers::default();
    let mut s = HidKeys::default();
    let mut p = HidPress::default();
    hidkeys_sync(&mut s, None, 0, 0);
    // A DualShock 4 through the generic driver: X, Y, Z, RZ its sticks, RX and RY its L2 and R2
    // (buttons too), all 0 to 255.
    for c in 0..=5usize {
        axis(&mut info, c, 0, 255, if c == 3 || c == 4 { 0 } else { 128 });
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 0 && t.axis[1] == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 128, &mut p) == 0 && hidkeys_trigger_event(&mut s, &t, 5, 255, &mut p) == 0);
    // Before its first report every axis reads 0: no verdict yet, and no triggers meanwhile.
    for c in 0..=5usize {
        info[c].value = 0;
    }
    check!(!hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 0 && t.axis[1] == 0);
    // The same ranges once a report came, Z and RZ at their minimum and RX and RY in the middle: an
    // Xbox layout's triggers, also with one pulled (outside the middle at its end).
    for c in 0..=5usize {
        axis(&mut info, c, 0, 255, if c == 2 || c == 5 { 0 } else { 128 });
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 2 && t.axis[1] == 5 && t.press[0] == 127 && t.release[0] == 63);
    info[2].value = 255;
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 2);
    // Signed sticks (at rest at 0, their middle, which no report has to set) beside 0 to 255 triggers.
    for c in 0..=5usize {
        let trig = c == 2 || c == 5;
        axis(&mut info, c, if trig { 0 } else { -32768 }, if trig { 255 } else { 32767 }, 0);
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 2 && t.axis[1] == 5);
    // Narrower Z and RZ resting in the middle once reported: a stick after all.
    for c in 0..=5usize {
        let trig = c == 2 || c == 5;
        axis(&mut info, c, 0, if trig { 255 } else { 65535 }, if trig { 128 } else { 32768 });
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 0 && t.axis[1] == 0);
    // Narrower Z and RZ, but RX and RY unlike X and Y (10-bit triggers there): no verdict from the ranges.
    for c in 0..=5usize {
        let rxy = c == 3 || c == 4;
        axis(&mut info, c, 0, if rxy { 1023 } else { 255 }, if rxy { 0 } else { 128 });
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 0 && t.axis[1] == 0);
    // Z and RZ without RX and RY (a DirectInput pad's right stick): never triggers.
    axes = [0; HID_ABS_BYTES];
    add(&mut axes, 0);
    add(&mut axes, 1);
    add(&mut axes, 2);
    add(&mut axes, 5);
    add(&mut axes, 16);
    add(&mut axes, 17);
    for c in 0..=5usize {
        axis(&mut info, c, 0, 255, if c == 2 || c == 5 { 0 } else { 128 });
    }
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 0 && t.axis[1] == 0);
    // BRAKE and GAS (an Xbox's later Bluetooth report, 8BitDo's D-input mode): those, LT the brake;
    // the right stick on Z and RZ stays a stick.
    add(&mut axes, 9);
    add(&mut axes, 10);
    axis(&mut info, 9, 0, 1023, 0);
    axis(&mut info, 10, 0, 1023, 0);
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 10 && t.axis[1] == 9);
    check!(hidkeys_trigger_event(&mut s, &t, 2, 255, &mut p) == 0 && hidkeys_trigger_event(&mut s, &t, 5, 0, &mut p) == 0);
    check!(hidkeys_trigger_event(&mut s, &t, 10, 1000, &mut p) == 1 && p.code == HIDKEYS_LT && !p.up);
    check!(hidkeys_trigger_event(&mut s, &t, 9, 1000, &mut p) == 1 && p.code == HIDKEYS_RT && !p.up);
    // A range of one step (0 to 1): pressed at 1, released at 0. None (min = max): no trigger.
    axis(&mut info, 10, 0, 1, 0);
    axis(&mut info, 9, 5, 5, 5);
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 10 && t.axis[1] == 0);
    hidkeys_sync(&mut s, None, 0, 0);
    check!(hidkeys_trigger_event(&mut s, &t, 10, 1, &mut p) == 1 && !p.up && hidkeys_trigger_event(&mut s, &t, 10, 0, &mut p) == 1 && p.up);
    // A controller with a key of that code of its own: no trigger there (no driver has one).
    axis(&mut info, 9, 0, 1023, 0);
    add(&mut keys, HIDKEYS_RT);
    check!(hidkeys_triggers(&mut t, &keys, &axes, &info) && t.axis[0] == 10 && t.axis[1] == 0);
    // A zeroed HidTriggers (a keyboard's, or before the verdict) has none.
    let none = HidTriggers::default();
    check!(hidkeys_trigger_event(&mut s, &none, 0, 1000, &mut p) == 0 && hidkeys_trigger_event(&mut s, &none, 2, 1000, &mut p) == 0);
}
#[test]
fn test_hidkeys() {
    run!(capabilities);
    run!(presses_and_modifiers);
    run!(hats_drop_and_pin_resync);
    run!(resync_releases_what_went);
    run!(power_key_on_firmware_5_20);
    run!(held_names_and_rate);
    run!(switch_layout);
    run!(the_scan_waits_for_the_pen_and_fingers);
    run!(the_scan_never_opens_what_grimoire_holds_or_knows);
    run!(analog_triggers_are_buttons);
    run!(a_right_stick_is_no_trigger);
    done();
}
