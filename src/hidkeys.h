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
typedef struct { int code, mods; bool gamepad, up; } HidPress;  // up: a release
HidKind hidkeys_kind(const uint8_t keys[HID_KEY_BYTES], const uint8_t axes[HID_ABS_BYTES]);
// Amazon's virtual input proxy (kindle-input-pt, firmware 5.20+) advertises every key code and
// delivers no presses of its own: never a keyboard, a controller or the power key.
bool hidkeys_proxy(const char *name);
// An optional keyboard or controller (hidkeys_kind), never the proxy.
HidKind hidkeys_optional(const char *name, const uint8_t keys[HID_KEY_BYTES], const uint8_t axes[HID_ABS_BYTES]);
// A Switch-mode controller read through Linux's generic HID driver, as 8BitDo's Switch mode and a Pro
// Controller come over Bluetooth ("Pro Controller"): its buttons are numbered in the Pro Controller's
// simple report order from BTN_A (B, A, Y, X, L, R, ZL, ZR, -, +, the sticks, Home, Capture), so
// BTN_C and BTN_Z are among them. Nintendo's own driver numbers them by the convention instead, and
// advertises neither.
bool hidkeys_switch(const char *name, const uint8_t keys[HID_KEY_BYTES]);
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
// The same after a lost frame, or with native PIN input up (held NULL: nothing held now): the keys,
// buttons and hat directions held before and not now are released into ups, at most cap of them;
// how many. One still held is released later, as any is.
int hidkeys_resync(HidKeys *state, HidKind kind, const uint8_t held[HID_KEY_BYTES], int hat_x, int hat_y,
                   HidPress *ups, int cap);
// Linux evdev event numbers: what one event does, in out, 0 to 2 of them. A new actionable press,
// or the release of a key or button held (a hat back to its middle releases its direction; turned
// straight across it releases it, then presses the other). Repeats, modifiers and power do nothing.
int hidkeys_event(HidKeys *state, HidKind kind, int type, int code, int value, HidPress out[2]);

// The scan for keyboards and controllers that come (input_evdev.c's key_scan) runs while shortcuts
// are on, every HIDKEYS_SCAN_MS, but never while the pen or a finger is in use (busy: the pen near
// or touching, a finger down) nor until the pen has been gone HIDKEYS_SCAN_QUIET_MS: nothing reads
// the pen while it runs. A scan put off runs at the first idle poll after.
#define HIDKEYS_SCAN_MS 5000u
#define HIDKEYS_SCAN_QUIET_MS 1500u
bool hidkeys_scan_due(uint32_t now, uint32_t last_scan, bool busy, uint32_t pen_left);
// A /dev/input node as stat() sees it, without opening it: closing an evdev node waits out an RCU
// grace period (22-96 ms on the Scribe's Linux 4.9), so the scan opens only nodes it has to.
typedef struct { uint64_t rdev, ino; int64_t ctime_s; long ctime_ns; } HidNode;
// Nodes the scan (or input_open) found to be no keyboard or controller, by path and identity: a
// node made again (a new inode or change time) is looked at again. The oldest goes when it is full.
#define HIDKEYS_NONE_MAX 32
typedef struct {
    struct { char path[40]; HidNode node; } e[HIDKEYS_NONE_MAX];
    int n, next;
} HidNoneCache;
typedef enum { HIDNODE_PROBE, HIDNODE_HELD, HIDNODE_NONE } HidNodeVerdict;
// Whether the scan opens the node at path: never one Grimoire holds open (its device number is in
// held: the pen, the pressure source, touch, power, orientation, the keyboards; 0s are ignored), nor
// one found no keyboard as it is now.
HidNodeVerdict hidkeys_node(const HidNoneCache *c, const uint64_t *held, int nheld, const char *path, const HidNode *n);
// Remembers that the node at path is no keyboard or controller (a path too long to keep is not kept).
void hidkeys_node_none(HidNoneCache *c, const char *path, const HidNode *n);
