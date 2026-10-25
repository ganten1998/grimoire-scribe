// Portable external-HID capability/press decoder. No device opens, grabs or key logging.
#pragma once
#include <stdbool.h>
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
// Synchronize after attachment, native PIN return or SYN_DROPPED. Held controls do not fire.
void hidkeys_sync(HidKeys *state, const uint8_t held[HID_KEY_BYTES], int hat_x, int hat_y);
// Linux evdev event numbers. True only for a new actionable press; repeats/releases are ignored.
bool hidkeys_event(HidKeys *state, HidKind kind, int type, int code, int value, HidPress *press);
