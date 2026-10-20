// power.h: the power button and the Kindle's power service as pure logic (unit-tested on the
// host): one press of the power key followed to its release or its 4 s hold (input_evdev.c), and
// the power service's state, which app.c reads before its sleep and wake fallbacks act.
#pragma once
#include "input.h"

// One press of the power key, as the platform layer follows it.
typedef struct {
    bool down;          // pressed, and no release seen yet
    bool held_sent;     // IN_POWER_HELD went out for this press
    uint32_t down_at;   // now_ms() of the press
} PowerKey;

// A KEY_POWER event (value 1 press, 0 release, 2 autorepeat) seen at `now`. Returns IN_POWER for a
// press, IN_POWER_UP for the release of a press it saw (*held_ms = how long the key was down), else
// IN_NONE (autorepeat, or a release whose press it never saw).
InType powerkey_event(PowerKey *k, int value, uint32_t now, int *held_ms);
// Called at every input poll. Once the key has been down INPUT_POWER_HOLD_MS it asks still_down()
// (EVIOCGKEY: 1 down, 0 up, -1 cannot tell) and returns IN_POWER_HELD, once per press, unless the
// key is found up: then its release was lost, and it returns IN_POWER_UP with *held_ms = -1.
// Otherwise IN_NONE; still_down is only called at that moment.
InType powerkey_tick(PowerKey *k, uint32_t now, int (*still_down)(void), int *held_ms);

// The power service's state (`lipc-get-prop -s com.lab126.powerd state`).
typedef enum {
    POWERD_UNKNOWN = 0,       // unreadable, or a name this build does not know
    POWERD_ACTIVE,            // awake
    POWERD_SCREENSAVER,       // its screensaver is up and input is locked (a passcode waits here)
    POWERD_READY_TO_SUSPEND,  // about to suspend
    POWERD_SUSPENDED,
} PowerdState;

// Parses the property's text: case, surrounding spaces and quotes are ignored; NULL, an empty
// string or any other name give POWERD_UNKNOWN.
PowerdState powerd_parse_state(const char *s);
// The screensaver or deeper: the Kindle is not awake and may be waiting for its passcode, so
// Grimoire must neither draw over it nor send it a power button (that would wake it).
bool powerd_dozing(PowerdState s);
