// power.h: the power button and the Kindle's power service as pure logic (unit-tested on the
// host): one press of the power key followed to its release or its 4 s hold (input_evdev.c), the
// power service's state, which app.c reads before its sleep and wake fallbacks act, and the
// decisions app.c takes on them.
#pragma once
#include "input.h"

// One press of the power key, as the platform layer follows it.
typedef struct {
    bool down;          // pressed, and no release seen yet
    bool held_sent;     // IN_POWER_HELD went out for this press
    uint32_t down_at;   // when it went down (now_ms()'s clock)
} PowerKey;

// A KEY_POWER event (value 1 press, 0 release, 2 autorepeat) stamped `t`: when the kernel saw it,
// in now_ms()'s clock, where the platform can tell (so a release read late, after a busy moment,
// is not a longer hold), else when it was read. Returns IN_POWER for a press, IN_POWER_UP for the
// release of a press it saw (*held_ms = how long the key was down), else IN_NONE (autorepeat, or a
// release whose press it never saw).
InType powerkey_event(PowerKey *k, int value, uint32_t t, int *held_ms);
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

// What the release of a power button press asks of Grimoire (app.c arms the matching fallback).
// `followed`: Grimoire saw the press go down (and has not spent it on a hold); `answered`: a sleep
// or wake announcement came while it was down; `down_asleep`, `asleep`: whether Grimoire showed its
// sleep screen when the press went down, and now; `held_ms`: IN_POWER_UP's value (-1 unknown).
typedef enum {
    POWER_UP_NOTHING,     // not followed, answered already, or Grimoire slept or woke meanwhile
    POWER_UP_SLEEP_WAIT,  // a short press while awake: wait 2.5 s for goingToScreenSaver
    POWER_UP_WAKE_WAIT,   // a short press on the sleep screen: wait 3 s for outOfScreenSaver
    POWER_UP_LEAVE,       // held INPUT_POWER_HOLD_MS, but the hold never reached Grimoire: leave
} PowerUpAction;
PowerUpAction power_up_action(bool followed, bool answered, bool down_asleep, bool asleep, int held_ms);

// The wake fallback: a short press on the sleep screen, and no outOfScreenSaver 3 s later. With the
// power service awake (or unreadable), wake. Still in its screensaver or deeper, the Kindle is
// locked (its passcode) or asleep: stay on the sleep screen, and the second time in a row leave
// Grimoire, so the Kindle gets its screen, pen and touch back even on a device whose power key
// cannot report a 4 s hold. `*locked` counts those in a row; a wake (any) resets it.
typedef enum { WAKE_FALLBACK_WAKE, WAKE_FALLBACK_STAY, WAKE_FALLBACK_LEAVE } WakeFallback;
WakeFallback power_wake_fallback(PowerdState st, int *locked);

// Input Grimoire ignores while it shows its sleep screen (or the Kindle's): pen and touch, and the
// start of a side button press or of a pinch (three clicks cleared the page, a pinch zoomed it).
// Their ends still get through, so one that began before the sleep finishes: a button left down
// would clear the page by its hold right after the wake.
bool power_asleep_drops(const InEvent *ev);
