#include "hidkeys.h"
#include "shortcuts.h"
#include "testlib.h"
#include <errno.h>
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
int main(void) {
    RUN(capabilities); RUN(presses_and_modifiers); RUN(hats_drop_and_pin_resync); RUN(resync_releases_what_went);
    RUN(power_key_on_firmware_5_20);
    RUN(held_names_and_rate);
    RUN(switch_layout);
    return tl_done();
}
