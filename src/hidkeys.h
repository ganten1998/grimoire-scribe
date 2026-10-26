// Portable external-HID capability/press decoder. No device opens, grabs or key logging.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define HID_KEY_BYTES 96
#define HID_ABS_BYTES 8
typedef enum { HID_NONE, HID_KEYBOARD, HID_GAMEPAD } HidKind;
typedef struct {
    uint8_t held[HID_KEY_BYTES];
    int hat[2];
    bool dropped;
} HidKeys;
typedef struct { int code, mods; bool gamepad; } HidPress;
HidKind hidkeys_kind(const uint8_t keys[HID_KEY_BYTES], const uint8_t axes[HID_ABS_BYTES]);
// Amazon's virtual input proxy (kindle-input-pt, firmware 5.20+) advertises every key code and
// delivers no presses of its own: never a keyboard, a controller or the power key.
bool hidkeys_proxy(const char *name);
// An optional keyboard or controller (hidkeys_kind), never the proxy.
HidKind hidkeys_optional(const char *name, const uint8_t keys[HID_KEY_BYTES], const uint8_t axes[HID_ABS_BYTES]);
// The power key: KEY_POWER on a device that is no pen or touch panel and not the proxy, with few
// other keys, so neither a keyboard nor the proxy can take its place. Decided apart from
// hidkeys_kind. -1 when it is not one, else how many keys it has: input_open watches the fewest.
int hidkeys_power(const char *name, const uint8_t keys[HID_KEY_BYTES], const uint8_t axes[HID_ABS_BYTES]);
// The attach probe: EVIOCGRAB(1)'s result and errno. True when another app (a button mapper)
// holds the device exclusively, so its presses never reach Grimoire.
bool hidkeys_held(int grab_result, int grab_errno);
// A device's name, which can come over the radio, made safe to show: printable ASCII only (a run of
// other bytes becomes one '?'), spaces trimmed, at most 40 bytes, "?" for none.
void hidkeys_name(char *out, size_t cap, const char *raw);
// A rate limit: true, and *last set to now, when *last is 0 (never) or at least gap ms ago.
bool hidkeys_due(uint32_t *last, uint32_t now, uint32_t gap);
// Synchronize after attachment, native PIN return or SYN_DROPPED. Held controls do not fire.
void hidkeys_sync(HidKeys *state, const uint8_t held[HID_KEY_BYTES], int hat_x, int hat_y);
// Linux evdev event numbers. True only for a new actionable press; repeats/releases are ignored.
bool hidkeys_event(HidKeys *state, HidKind kind, int type, int code, int value, HidPress *press);
