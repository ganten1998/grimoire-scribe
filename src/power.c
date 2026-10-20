// power.c: the power key and power service logic (see power.h).
#include "power.h"

#include <ctype.h>
#include <string.h>
#include <strings.h>

InType powerkey_event(PowerKey *k, int value, uint32_t t, int *held_ms) {
    *held_ms = 0;
    if (value == 1) {
        // A press, even over one whose release never came: follow the new one.
        k->down = true;
        k->held_sent = false;
        k->down_at = t;
        return IN_POWER;
    }
    if (value == 0 && k->down) {
        k->down = false;
        *held_ms = (int)(t - k->down_at);
        return IN_POWER_UP;
    }
    return IN_NONE;
}

InType powerkey_tick(PowerKey *k, uint32_t now, int (*still_down)(void), int *held_ms) {
    *held_ms = 0;
    if (!k->down || k->held_sent || (int32_t)(now - k->down_at) < INPUT_POWER_HOLD_MS) return IN_NONE;
    // The events say it is still down: make sure before leaving Grimoire on it. If the device
    // cannot tell, trust the events (a key device does not drop them).
    if (still_down && still_down() == 0) {
        k->down = false;
        *held_ms = -1;
        return IN_POWER_UP;
    }
    k->held_sent = true;
    *held_ms = (int)(now - k->down_at);
    return IN_POWER_HELD;
}

PowerdState powerd_parse_state(const char *s) {
    static const struct {
        const char *name;
        PowerdState state;
    } k_names[] = {{"active", POWERD_ACTIVE},
                   {"screenSaver", POWERD_SCREENSAVER},
                   {"readyToSuspend", POWERD_READY_TO_SUSPEND},
                   {"suspended", POWERD_SUSPENDED}};
    if (!s) return POWERD_UNKNOWN;
    while (*s && (isspace((unsigned char)*s) || *s == '"')) s++;
    size_t n = strlen(s);
    while (n && (isspace((unsigned char)s[n - 1]) || s[n - 1] == '"')) n--;
    for (size_t i = 0; i < sizeof k_names / sizeof k_names[0]; i++)
        if (strlen(k_names[i].name) == n && !strncasecmp(s, k_names[i].name, n)) return k_names[i].state;
    return POWERD_UNKNOWN;
}

bool powerd_dozing(PowerdState s) {
    return s == POWERD_SCREENSAVER || s == POWERD_READY_TO_SUSPEND || s == POWERD_SUSPENDED;
}

PowerUpAction power_up_action(bool followed, bool answered, bool down_asleep, bool asleep, int held_ms) {
    if (!followed) return POWER_UP_NOTHING;
    // Held past the 4 s mark while Grimoire was busy (its IN_POWER_HELD never came): the release is
    // the hold, answered or not, as the hold itself would have been.
    if (held_ms >= INPUT_POWER_HOLD_MS) return POWER_UP_LEAVE;
    // The power service answered (a wake announced while the key was down must not turn the release
    // into a sleep request), or Grimoire slept or woke on its own while the key was down.
    if (answered || down_asleep != asleep) return POWER_UP_NOTHING;
    return asleep ? POWER_UP_WAKE_WAIT : POWER_UP_SLEEP_WAIT;
}

bool power_asleep_drops(const InEvent *ev) {
    switch (ev->type) {
    case IN_PEN_DOWN:
    case IN_PEN_MOVE:
    case IN_PEN_UP:
    case IN_TOUCH_DOWN:
    case IN_TOUCH_MOVE:
    case IN_TOUCH_UP:
    case IN_PINCH_BEGIN:
    case IN_PINCH_MOVE: return true;
    case IN_PEN_BUTTON: return ev->side_button;  // the press; its release gets through
    default: return false;
    }
}
