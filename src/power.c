// power.c: the power key and power service logic (see power.h).
#include "power.h"

#include <ctype.h>
#include <string.h>
#include <strings.h>

InType powerkey_event(PowerKey *k, int value, uint32_t now, int *held_ms) {
    *held_ms = 0;
    if (value == 1) {
        // A press, even over one whose release never came: follow the new one.
        k->down = true;
        k->held_sent = false;
        k->down_at = now;
        return IN_POWER;
    }
    if (value == 0 && k->down) {
        k->down = false;
        *held_ms = (int)(now - k->down_at);
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
