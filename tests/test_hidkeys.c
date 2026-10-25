#include "hidkeys.h"
#include "shortcuts.h"
#include "testlib.h"
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
int main(void) { RUN(capabilities); RUN(presses_and_modifiers); RUN(hats_drop_and_pin_resync); return tl_done(); }
