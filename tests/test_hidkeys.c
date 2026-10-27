#include "hidkeys.h"
#include "shortcuts.h"
#include "testlib.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>
static void add(uint8_t *bits, int code) { bits[code / 8] |= (uint8_t)(1u << (code % 8)); }
TEST(capabilities) {
    uint8_t keys[HID_KEY_BYTES] = {0}, axes[HID_ABS_BYTES] = {0};
    add(keys, 116); CHECK(hidkeys_kind(keys, axes) == HID_NONE); // native power
    add(keys, 114); CHECK(hidkeys_kind(keys, axes) == HID_KEYBOARD); // sparse media remote
    add(keys, 320); CHECK(hidkeys_kind(keys, axes) == HID_NONE); // never a pen, even with keys
    memset(keys, 0, sizeof keys); add(keys, 308);
    CHECK(hidkeys_kind(keys, axes) == HID_GAMEPAD); // not every controller advertises button 304
    add(axes, 53); CHECK(hidkeys_kind(keys, axes) == HID_NONE); // touch interface
    memset(keys, 0, sizeof keys); memset(axes, 0, sizeof axes);
    add(axes, 0); add(axes, 1); CHECK(hidkeys_kind(keys, axes) == HID_NONE); // accelerometer
    add(keys, 272); CHECK(hidkeys_kind(keys, axes) == HID_NONE); // mouse
    add(axes, 16); CHECK(hidkeys_kind(keys, axes) == HID_GAMEPAD);
}
// A Switch-mode controller through Linux's generic driver: "Pro Controller", sixteen buttons from
// BTN_A (BTN_C and BTN_Z among them). Another pad with the same buttons, or a Pro Controller under
// Nintendo's driver (the convention's buttons, no BTN_C or BTN_Z), is not one.
TEST(switch_layout) {
    uint8_t generic[HID_KEY_BYTES] = {0}, nintendo[HID_KEY_BYTES] = {0};
    for (int code = 304; code <= 319; code++) add(generic, code);
    CHECK(hidkeys_switch("Pro Controller", generic));
    CHECK(!hidkeys_switch("Xbox Wireless Controller", generic));
    CHECK(!hidkeys_switch(NULL, generic));
    for (int code = 304; code <= 317; code++) if (code != 306 && code != 309) add(nintendo, code);
    CHECK(!hidkeys_switch("Nintendo Switch Pro Controller", nintendo));
}
TEST(presses_and_modifiers) {
    HidKeys s; HidPress p[2]; hidkeys_sync(&s, NULL, 0, 0);
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 29, 1, p));
    CHECK(hidkeys_event(&s, HID_KEYBOARD, 1, 44, 1, p) == 1 && p[0].code == 44 && p[0].mods == KEYMOD_CTRL && !p[0].up);
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 44, 1, p)); // duplicate down
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 44, 2, p)); // auto-repeat
    CHECK(hidkeys_event(&s, HID_KEYBOARD, 1, 44, 0, p) == 1 && p[0].code == 44 && p[0].up); // its release
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 44, 0, p)); // released again: nothing
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 97, 1, p));
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 29, 0, p)); // right Ctrl remains held; no modifier's release
    CHECK(hidkeys_event(&s, HID_KEYBOARD, 1, 44, 1, p) == 1 && p[0].mods == KEYMOD_CTRL);
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 116, 1, p)); // external power never quits
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 116, 0, p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 768, 1, p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 114, 1, p) == 1 && !p[0].gamepad); // mixed HID media key
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, p) == 1 && p[0].gamepad && !p[0].up);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 0, p) == 1 && p[0].code == 304 && p[0].gamepad && p[0].up);
}
TEST(hats_drop_and_pin_resync) {
    HidKeys s; HidPress p[2]; hidkeys_sync(&s, NULL, 0, 0);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, -1, p) == 1 && p[0].code == 546 && !p[0].up);
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, -1, p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, 0, p) == 1 && p[0].code == 546 && p[0].up); // back to the middle
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, 1, p) == 1 && p[0].code == 547);
    // Straight across, with no middle between: the release of the one, then the press of the other.
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, -1, p) == 2 && p[0].code == 547 && p[0].up && p[1].code == 546 && !p[1].up);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 17, -1, p) == 1 && p[0].code == 544);
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 0, 200, p)); // sticks are not hats
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, 2, p)); // invalid hat value
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 0, 3, 0, p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 0, 0, 0, p)); // SYN_REPORT alone cannot trust stale state
    uint8_t held[HID_KEY_BYTES] = {0}; add(held, 304);
    hidkeys_sync(&s, held, 1, 0); // held while PIN was up, or after a lost frame
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, 1, p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 0, p) == 1 && p[0].up); // its release is one (nothing waits for it)
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, p) == 1 && !p[0].up);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, 0, p) == 1 && p[0].code == 547 && p[0].up);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, 1, p) == 1 && !p[0].up);
}
TEST(resync_releases_what_went) {
    HidKeys s; HidPress p[2], ups[8]; hidkeys_sync(&s, NULL, 0, 0);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 312, 1, p) == 1);  // L2 held (Erase while held, say)
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, p) == 1);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 29, 1, p) == 0);   // a modifier: never released into ups
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 17, 1, p) == 1 && p[0].code == 545);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 0, 3, 0, p) == 0);   // SYN_DROPPED: the frames in between are lost
    // A fresh state with South still held, L2 and the hat let go: their releases, South's later.
    uint8_t held[HID_KEY_BYTES] = {0}; add(held, 304);
    int n = hidkeys_resync(&s, HID_GAMEPAD, held, 0, 0, ups, 8);
    CHECK(n == 2 && ups[0].code == 312 && ups[0].up && ups[0].gamepad && ups[1].code == 545 && ups[1].up);
    CHECK(!s.dropped);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 312, 0, p) == 0);  // gone already
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 0, p) == 1 && p[0].code == 304 && p[0].up);
    // Native PIN input up (held NULL): everything held is let go; cap bounds how many.
    CHECK(hidkeys_event(&s, HID_KEYBOARD, 1, 30, 1, p) == 1 && hidkeys_event(&s, HID_KEYBOARD, 1, 31, 1, p) == 1);
    n = hidkeys_resync(&s, HID_KEYBOARD, NULL, 0, 0, ups, 1);
    CHECK(n == 1 && ups[0].code == 30 && ups[0].up && !ups[0].gamepad);
    CHECK(hidkeys_resync(&s, HID_KEYBOARD, NULL, 0, 0, ups, 8) == 0);  // (state already empty)
    // Held at attach (the first sync): no release on that sync.
    hidkeys_sync(&s, NULL, 0, 0);
    CHECK(hidkeys_resync(&s, HID_GAMEPAD, held, -1, 0, ups, 8) == 0);
    CHECK(hidkeys_resync(&s, HID_GAMEPAD, held, 7, 0, ups, 8) == 1 && ups[0].code == 546);  // (7 reads as 0)
}
// A sysfs capability bitmap (/sys/class/input/eventN/device/capabilities/key or abs) as the
// Kindle prints it: hex words of 32 bits (an armhf long), the highest first.
static void sysfs_bits(const char *text, uint8_t *bits, size_t nbytes) {
    unsigned long words[64]; int n = 0;
    memset(bits, 0, nbytes);
    for (const char *p = text; *p && n < 64;) {
        char *end; words[n] = strtoul(p, &end, 16);
        if (end == p) break;
        n++; p = end;
    }
    for (int w = 0; w < n; w++)
        for (int b = 0; b < 32; b++)
            if ((words[n - 1 - w] >> b) & 1ul && (size_t)(w * 32 + b) / 8 < nbytes) add(bits, w * 32 + b);
}
// The Kindle Scribe 2024 on firmware 5.20.1.0.2 (KindleLab evidence dpre-2026-10-25, input-caps.txt).
#define EVERY_KEY "ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff " \
                  "ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff " \
                  "ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff ffffffff fffffffe"
static const struct { const char *name, *key, *abs; } k_scribe_520[] = {
    {"bd71828-pwrkey", "100000 0 0 0", "0"},
    {"bma4xy_acc", "0", "100 7"},
    {"bma4xy_feature", "0", "1000000"},
    {"WacomDigitizer", "1c03 0 0 0 0 0 0 0 0 0 0", "f000003"},
    {"pt_mt", "0", "ee18000 0"},
    {"kindle-input-pt", EVERY_KEY, "0"},
    {"stylus-custom", "1c03 0 0 0 0 0 0 0 0 0 0", "f000003"},
};
TEST(power_key_on_firmware_5_20) {
    uint8_t keys[HID_KEY_BYTES], axes[HID_ABS_BYTES];
    int powers = 0, best = -1, rank = 0;
    // input_open meets them in readdir's order, the proxy (event5) before the power key (event0).
    for (int i = (int)(sizeof k_scribe_520 / sizeof *k_scribe_520) - 1; i >= 0; i--) {
        sysfs_bits(k_scribe_520[i].key, keys, sizeof keys);
        sysfs_bits(k_scribe_520[i].abs, axes, sizeof axes);
        int r = hidkeys_power(k_scribe_520[i].name, keys, axes);
        if (r >= 0) { powers++; if (best < 0 || r < rank) best = i, rank = r; }
        // No device of the Kindle's own is an optional keyboard or controller.
        CHECK(hidkeys_optional(k_scribe_520[i].name, keys, axes) == HID_NONE);
    }
    CHECK(powers == 1 && best == 0 && rank == 1);
    sysfs_bits(EVERY_KEY, keys, sizeof keys); memset(axes, 0, sizeof axes);
    CHECK(hidkeys_kind(keys, axes) == HID_NONE); // BTN_TOOL_PEN and BTN_TOUCH are among every key
    // A proxy without the pen and touch bits, as a later firmware may make it, is still nothing.
    keys[320 / 8] = 0; keys[330 / 8] = 0;
    CHECK(hidkeys_kind(keys, axes) == HID_GAMEPAD); // by its bits alone: the gamepad buttons
    CHECK(hidkeys_optional("kindle-input-pt", keys, axes) == HID_NONE);
    CHECK(hidkeys_power("kindle-input-pt", keys, axes) == -1);
    // Under any other name, a device with every key is never a power key.
    CHECK(hidkeys_power("virtual-keys", keys, axes) == -1);
    // A remote with a power key loses to the Kindle's own, which has fewer keys.
    memset(keys, 0, sizeof keys);
    add(keys, 116); add(keys, 114); add(keys, 115); add(keys, 163); add(keys, 165);
    CHECK(hidkeys_power("BT remote", keys, axes) == 5);
    CHECK(hidkeys_optional("BT remote", keys, axes) == HID_KEYBOARD);
    CHECK(hidkeys_power(NULL, keys, axes) == 5 && hidkeys_optional(NULL, keys, axes) == HID_KEYBOARD);
}
TEST(held_names_and_rate) {
    CHECK(hidkeys_held(-1, EBUSY));  // a button mapper's grab
    CHECK(!hidkeys_held(0, EBUSY) && !hidkeys_held(-1, EINVAL) && !hidkeys_held(-1, ENODEV));
    char n[41];
    hidkeys_name(n, sizeof n, "Pro Controller"); CHECK(!strcmp(n, "Pro Controller"));
    hidkeys_name(n, sizeof n, "  8BitDo\x01\x02\xc3\xa9 Stick\n "); CHECK(!strcmp(n, "8BitDo? Stick?"));
    hidkeys_name(n, sizeof n, "0123456789012345678901234567890123456789012345");
    CHECK(strlen(n) == 40 && !strncmp(n, "0123456789", 10));
    hidkeys_name(n, sizeof n, ""); CHECK(!strcmp(n, "?"));
    hidkeys_name(n, sizeof n, NULL); CHECK(!strcmp(n, "?"));
    char small[5]; hidkeys_name(small, sizeof small, "Keyboard"); CHECK(!strcmp(small, "Keyb"));
    uint32_t last = 0;
    CHECK(hidkeys_due(&last, 1000, 30000)); // the first one
    CHECK(!hidkeys_due(&last, 1500, 30000) && !hidkeys_due(&last, 30999, 30000));
    CHECK(hidkeys_due(&last, 31000, 30000) && last == 31000);
    last = 0; CHECK(hidkeys_due(&last, 0, 30000) && last != 0); // at time 0, still remembered
    last = 0xfffff000u; CHECK(hidkeys_due(&last, 0x00010000u, 30000)); // across the clock's wrap
}
// The scan for keyboards and controllers that come held the thread 320-370 ms, every 5 s: long
// enough for the kernel to drop pen events mid-stroke (p28, 2026-10-26).
TEST(the_scan_waits_for_the_pen_and_fingers) {
    CHECK(hidkeys_scan_due(6000, 1000, false, 0));
    CHECK(!hidkeys_scan_due(5999, 1000, false, 0));  // every 5 s, as before
    CHECK(!hidkeys_scan_due(60000, 1000, true, 0));   // never while the pen or a finger is in use
    CHECK(!hidkeys_scan_due(6000, 1000, false, 5000)); // nor until the pen has been gone 1.5 s
    CHECK(hidkeys_scan_due(6500, 1000, false, 5000));
    // Writing from 5.9 s to 9.2 s with polls every 30 ms: no scan mid-stroke, nor while the pen is
    // near; the scan put off runs at the first idle poll, 1.5 s after the pen left.
    uint32_t last = 1000, left = 0, first = 0;
    int during = 0;
    for (uint32_t now = 5900; now < 20000; now += 30) {
        bool busy = now < 9200;
        if (now == 9200) left = now;
        if (!hidkeys_scan_due(now, last, busy, left)) continue;
        if (busy || now - left < 1500) during++;
        if (!first) first = now;
        last = now;
    }
    CHECK(during == 0 && first == 10700);
    // Across the clock's wrap.
    CHECK(hidkeys_scan_due(0x00001000u, 0xfffff000u, false, 0xffff0000u));
    CHECK(!hidkeys_scan_due(0x00000100u, 0xfffff000u, false, 0xffff0000u));
}
// The Scribe 2 on firmware 5.20.1: event0 the power key, 1 the accelerometer, 2 its orientation
// codes, 3 the raw digitizer, 4 touch, 5 kindle-input-pt, 6 the scaled pen (major 13, minors 64..70).
static HidNode scribe_node(int i) { return (HidNode){(13u << 8) | (unsigned)(64 + i), 100u + (unsigned)i, 1761500000, 5}; }
TEST(the_scan_never_opens_what_grimoire_holds_or_knows) {
    static HidNoneCache c;
    memset(&c, 0, sizeof c);
    uint64_t held[] = {scribe_node(6).rdev, scribe_node(3).rdev, scribe_node(4).rdev, scribe_node(0).rdev,
                       scribe_node(2).rdev, 0};  // pen, pressure source, touch, power, orientation, a closed fd
    int nheld = (int)(sizeof held / sizeof held[0]);
    char path[40];
    // input_open looked at the accelerometer and kindle-input-pt, and left them: no keyboards.
    HidNode acc = scribe_node(1), proxy = scribe_node(5);
    hidkeys_node_none(&c, "/dev/input/event1", &acc);
    hidkeys_node_none(&c, "/dev/input/event5", &proxy);
    int opened = 0;
    for (int i = 0; i < 7; i++) {
        snprintf(path, sizeof path, "/dev/input/event%d", i);
        HidNode n = scribe_node(i);
        HidNodeVerdict v = hidkeys_node(&c, held, nheld, path, &n);
        opened += v == HIDNODE_PROBE;
        CHECK(v == (i == 1 || i == 5 ? HIDNODE_NONE : HIDNODE_HELD));
    }
    CHECK(opened == 0);  // the old scan opened all seven, every 5 s
    // A controller that comes is opened once; found to be none, it is not opened again.
    HidNode pad = {(13u << 8) | 71u, 200, 1761500100, 7};
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event7", &pad) == HIDNODE_PROBE);
    hidkeys_node_none(&c, "/dev/input/event7", &pad);
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event7", &pad) == HIDNODE_NONE);
    // The node made again (a new device at the same path): a new inode, or a new change time.
    HidNode again = pad;
    again.ino = 201;
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event7", &again) == HIDNODE_PROBE);
    again = pad;
    again.ctime_ns = 8;
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event7", &again) == HIDNODE_PROBE);
    int had = c.n;
    hidkeys_node_none(&c, "/dev/input/event7", &again);
    CHECK(c.n == had && hidkeys_node(&c, held, nheld, "/dev/input/event7", &again) == HIDNODE_NONE);
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event7", &pad) == HIDNODE_PROBE);
    // A device number of 0 (a closed fd's place) holds nothing.
    HidNode zero = {0, 300, 0, 0};
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event9", &zero) == HIDNODE_PROBE);
    // A path too long to keep is never kept: always opened.
    char longp[80];
    snprintf(longp, sizeof longp, "/dev/input/by-a-name-longer-than-a-node-path/event%d", 12);
    hidkeys_node_none(&c, longp, &pad);
    CHECK(hidkeys_node(&c, held, nheld, longp, &pad) == HIDNODE_PROBE);
    // Full: the oldest goes.
    for (int i = 0; i < HIDKEYS_NONE_MAX; i++) {
        snprintf(path, sizeof path, "/dev/input/event%d", 20 + i);
        HidNode n = {(uint64_t)(1000 + i), (uint64_t)i, 0, 0};
        hidkeys_node_none(&c, path, &n);
    }
    CHECK(c.n == HIDKEYS_NONE_MAX);
    CHECK(hidkeys_node(&c, held, nheld, "/dev/input/event1", &acc) == HIDNODE_PROBE);
    HidNode last = {1000 + HIDKEYS_NONE_MAX - 1, HIDKEYS_NONE_MAX - 1, 0, 0};
    snprintf(path, sizeof path, "/dev/input/event%d", 20 + HIDKEYS_NONE_MAX - 1);
    CHECK(hidkeys_node(&c, held, nheld, path, &last) == HIDNODE_NONE);
}
int main(void) {
    RUN(capabilities); RUN(presses_and_modifiers); RUN(hats_drop_and_pin_resync); RUN(resync_releases_what_went);
    RUN(power_key_on_firmware_5_20);
    RUN(held_names_and_rate);
    RUN(switch_layout);
    RUN(the_scan_waits_for_the_pen_and_fingers);
    RUN(the_scan_never_opens_what_grimoire_holds_or_knows);
    return tl_done();
}
