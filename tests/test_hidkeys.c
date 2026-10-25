#include "hidkeys.h"
#include "shortcuts.h"
#include "testlib.h"
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
TEST(presses_and_modifiers) {
    HidKeys s; HidPress p; hidkeys_sync(&s, NULL, 0, 0);
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 29, 1, &p));
    CHECK(hidkeys_event(&s, HID_KEYBOARD, 1, 44, 1, &p) && p.code == 44 && p.mods == KEYMOD_CTRL);
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 44, 1, &p)); // duplicate down
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 44, 2, &p)); // auto-repeat
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 44, 0, &p));
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 97, 1, &p));
    CHECK(!hidkeys_event(&s, HID_KEYBOARD, 1, 29, 0, &p)); // right Ctrl remains held
    CHECK(hidkeys_event(&s, HID_KEYBOARD, 1, 44, 1, &p) && p.mods == KEYMOD_CTRL);
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 116, 1, &p)); // external power never quits
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 768, 1, &p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 114, 1, &p) && !p.gamepad); // mixed HID media key
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, &p) && p.gamepad);
}
TEST(hats_drop_and_pin_resync) {
    HidKeys s; HidPress p; hidkeys_sync(&s, NULL, 0, 0);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, -1, &p) && p.code == 546);
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, -1, &p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, 0, &p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, 1, &p) && p.code == 547);
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 17, -1, &p) && p.code == 544);
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 0, 200, &p)); // sticks are not hats
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, 2, &p)); // invalid hat value
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 0, 3, 0, &p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, &p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 0, 0, 0, &p)); // SYN_REPORT alone cannot trust stale state
    uint8_t held[HID_KEY_BYTES] = {0}; add(held, 304);
    hidkeys_sync(&s, held, 1, 0); // held while PIN was up, or after a lost frame
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, &p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, 1, &p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 1, 304, 0, &p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 1, 304, 1, &p));
    CHECK(!hidkeys_event(&s, HID_GAMEPAD, 3, 16, 0, &p));
    CHECK(hidkeys_event(&s, HID_GAMEPAD, 3, 16, 1, &p));
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
int main(void) {
    RUN(capabilities); RUN(presses_and_modifiers); RUN(hats_drop_and_pin_resync); RUN(power_key_on_firmware_5_20);
    return tl_done();
}
