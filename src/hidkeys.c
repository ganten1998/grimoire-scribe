#include "hidkeys.h"
#include "shortcuts.h"
#include <string.h>

static bool bit(const uint8_t *bytes, int code) { return (bytes[code / 8] >> (code % 8)) & 1; }
static void set(uint8_t *bytes, int code, bool value) {
    if (value) bytes[code / 8] |= (uint8_t)(1u << (code % 8));
    else bytes[code / 8] &= (uint8_t)~(1u << (code % 8));
}
static bool power(int code) { return code == 116 || code == 142 || code == 143; }
HidKind hidkeys_kind(const uint8_t *keys, const uint8_t *axes) {
    // Reject pen/touch interfaces before inspecting their buttons. An accelerometer's ABS_X/Y
    // is not a joystick; a mouse's BTN_LEFT/RIGHT is not a controller either.
    if (bit(keys, 320) || bit(keys, 321) || bit(keys, 325) || bit(keys, 330) || bit(axes, 53)) return HID_NONE;
    for (int code = 288; code <= 319; code++) if (bit(keys, code)) return HID_GAMEPAD;
    for (int code = 544; code <= 547; code++) if (bit(keys, code)) return HID_GAMEPAD;
    if (bit(axes, 16) || bit(axes, 17)) return HID_GAMEPAD;
    // Includes sparse page turners and media remotes, not just full A/Z/Space keyboards.
    for (int code = 1; code < 256; code++) if (!power(code) && bit(keys, code)) return HID_KEYBOARD;
    return HID_NONE;
}
void hidkeys_sync(HidKeys *state, const uint8_t *held, int hat_x, int hat_y) {
    memset(state, 0, sizeof *state);
    if (held) memcpy(state->held, held, sizeof state->held);
    state->hat[0] = hat_x >= -1 && hat_x <= 1 ? hat_x : 0;
    state->hat[1] = hat_y >= -1 && hat_y <= 1 ? hat_y : 0;
}
static int mods(const HidKeys *state) {
    return ((bit(state->held, 29) || bit(state->held, 97)) ? KEYMOD_CTRL : 0) |
           ((bit(state->held, 42) || bit(state->held, 54)) ? KEYMOD_SHIFT : 0) |
           ((bit(state->held, 56) || bit(state->held, 100)) ? KEYMOD_ALT : 0) |
           ((bit(state->held, 125) || bit(state->held, 126)) ? KEYMOD_META : 0);
}
bool hidkeys_event(HidKeys *state, HidKind kind, int type, int code, int value, HidPress *press) {
    if (type == 0 && code == 3) { // EV_SYN / SYN_DROPPED: caller must obtain a fresh device state
        state->dropped = true;
        return false;
    }
    if (state->dropped || kind == HID_NONE) return false;
    if (type == 3 && kind == HID_GAMEPAD && (code == 16 || code == 17)) {
        if (value < -1 || value > 1) return false;
        int axis = code - 16, previous = state->hat[axis];
        state->hat[axis] = value;
        if (!value || value == previous) return false;
        // Use the same evdev codes as digital D-pads: UP/DOWN/LEFT/RIGHT.
        *press = (HidPress){.code = axis ? (value < 0 ? 544 : 545) : (value < 0 ? 546 : 547), .gamepad = true};
        return true;
    }
    if (type != 1 || code <= 0 || code >= HID_KEY_BYTES * 8 || power(code)) return false;
    if (!value) { set(state->held, code, false); return false; }
    if (value != 1 || bit(state->held, code)) return false;
    set(state->held, code, true);
    if (code == 29 || code == 97 || code == 42 || code == 54 || code == 56 || code == 100 || code == 125 || code == 126) return false;
    *press = (HidPress){.code = code, .mods = mods(state), .gamepad = kind == HID_GAMEPAD && code >= 256};
    return true;
}
