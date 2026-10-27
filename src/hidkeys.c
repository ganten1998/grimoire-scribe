#include "hidkeys.h"
#include "shortcuts.h"
#include <errno.h>
#include <string.h>

static bool bit(const uint8_t *bytes, int code) { return (bytes[code / 8] >> (code % 8)) & 1; }
static void set(uint8_t *bytes, int code, bool value) {
    if (value) bytes[code / 8] |= (uint8_t)(1u << (code % 8));
    else bytes[code / 8] &= (uint8_t)~(1u << (code % 8));
}
static bool power(int code) { return code == 116 || code == 142 || code == 143; }
// BTN_TOOL_PEN, BTN_TOOL_RUBBER, BTN_TOOL_FINGER, BTN_TOUCH or ABS_MT_POSITION_X.
static bool digitizer(const uint8_t *keys, const uint8_t *axes) {
    return bit(keys, 320) || bit(keys, 321) || bit(keys, 325) || bit(keys, 330) || bit(axes, 53);
}
HidKind hidkeys_kind(const uint8_t *keys, const uint8_t *axes) {
    // Reject pen/touch interfaces before inspecting their buttons. An accelerometer's ABS_X/Y
    // is not a joystick; a mouse's BTN_LEFT/RIGHT is not a controller either.
    if (digitizer(keys, axes)) return HID_NONE;
    for (int code = 288; code <= 319; code++) if (bit(keys, code)) return HID_GAMEPAD;
    for (int code = 544; code <= 547; code++) if (bit(keys, code)) return HID_GAMEPAD;
    if (bit(axes, 16) || bit(axes, 17)) return HID_GAMEPAD;
    // Includes sparse page turners and media remotes, not just full A/Z/Space keyboards.
    for (int code = 1; code < 256; code++) if (!power(code) && bit(keys, code)) return HID_KEYBOARD;
    return HID_NONE;
}
bool hidkeys_proxy(const char *name) { return name && !strcmp(name, "kindle-input-pt"); }
HidKind hidkeys_optional(const char *name, const uint8_t *keys, const uint8_t *axes) {
    return hidkeys_proxy(name) ? HID_NONE : hidkeys_kind(keys, axes);
}
int hidkeys_power(const char *name, const uint8_t *keys, const uint8_t *axes) {
    // The Scribe's own key (bd71828-pwrkey) has KEY_POWER alone; the proxy has all 767 codes.
    if (hidkeys_proxy(name) || !bit(keys, 116) || digitizer(keys, axes)) return -1;
    int n = 0;
    for (int code = 1; code < HID_KEY_BYTES * 8; code++) n += bit(keys, code);
    return n <= 16 ? n : -1;
}
bool hidkeys_held(int grab_result, int grab_errno) { return grab_result != 0 && grab_errno == EBUSY; }
void hidkeys_name(char *out, size_t cap, const char *raw) {
    if (!cap) return;
    size_t n = 0, max = cap - 1 < 40 ? cap - 1 : 40;
    bool other = false;  // inside a run of bytes that are not printable ASCII
    const char *p = raw ? raw : "";
    while (*p == ' ') p++;
    for (; *p && n < max; p++) {
        unsigned char c = (unsigned char)*p;
        if (c >= 0x20 && c < 0x7f) out[n++] = (char)c, other = false;
        else if (!other) out[n++] = '?', other = true;
    }
    while (n && out[n - 1] == ' ') n--;
    if (!n && max) out[n++] = '?';
    out[n] = 0;
}
bool hidkeys_due(uint32_t *last, uint32_t now, uint32_t gap) {
    if (*last && now - *last < gap) return false;
    *last = now ? now : 1;
    return true;
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
static bool modifier(int code) {
    return code == 29 || code == 97 || code == 42 || code == 54 || code == 56 || code == 100 || code == 125 || code == 126;
}
static HidPress key(const HidKeys *state, HidKind kind, int code, bool up) {
    return (HidPress){.code = code, .mods = mods(state), .gamepad = kind == HID_GAMEPAD && code >= 256, .up = up};
}
// A hat direction as the digital D-pads' evdev codes: UP/DOWN/LEFT/RIGHT.
static HidPress hat(int axis, int value, bool up) {
    return (HidPress){.code = axis ? (value < 0 ? 544 : 545) : (value < 0 ? 546 : 547), .gamepad = true, .up = up};
}
int hidkeys_resync(HidKeys *state, HidKind kind, const uint8_t *held, int hat_x, int hat_y, HidPress *ups, int cap) {
    int n = 0, now[2] = {hat_x, hat_y};
    for (int code = 1; code < HID_KEY_BYTES * 8 && n < cap; code++)
        if (bit(state->held, code) && !(held && bit(held, code)) && !modifier(code) && !power(code))
            ups[n++] = key(state, kind, code, true);
    for (int axis = 0; axis < 2; axis++) {
        int v = now[axis] >= -1 && now[axis] <= 1 ? now[axis] : 0;
        if (kind == HID_GAMEPAD && state->hat[axis] && state->hat[axis] != v && n < cap)
            ups[n++] = hat(axis, state->hat[axis], true);
    }
    hidkeys_sync(state, held, hat_x, hat_y);
    return n;
}
int hidkeys_event(HidKeys *state, HidKind kind, int type, int code, int value, HidPress out[2]) {
    if (type == 0 && code == 3) { // EV_SYN / SYN_DROPPED: caller must obtain a fresh device state
        state->dropped = true;
        return 0;
    }
    if (state->dropped || kind == HID_NONE) return 0;
    if (type == 3 && kind == HID_GAMEPAD && (code == 16 || code == 17)) {
        if (value < -1 || value > 1) return 0;
        int axis = code - 16, previous = state->hat[axis], n = 0;
        state->hat[axis] = value;
        if (value == previous) return 0;
        if (previous) out[n++] = hat(axis, previous, true);
        if (value) out[n++] = hat(axis, value, false);
        return n;
    }
    if (type != 1 || code <= 0 || code >= HID_KEY_BYTES * 8 || power(code)) return 0;
    if (!value) {
        bool was = bit(state->held, code);
        set(state->held, code, false);
        if (!was || modifier(code)) return 0;
        out[0] = key(state, kind, code, true);
        return 1;
    }
    if (value != 1 || bit(state->held, code)) return 0;
    set(state->held, code, true);
    if (modifier(code)) return 0;
    out[0] = key(state, kind, code, false);
    return 1;
}
