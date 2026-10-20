// test_power.c: the power key followed to its release or its 4 s hold, the power service's state
// as the sleep and wake fallbacks read it, and what Grimoire decides from them.
#include "power.h"
#include "testlib.h"

static int g_down = 1, g_asked;
static int mock_down(void) {
    g_asked++;
    return g_down;
}

TEST(press_release) {
    PowerKey k = {0};
    int held = -5;
    CHECK(powerkey_event(&k, 1, 1000, &held) == IN_POWER);
    CHECK(k.down);
    CHECK(powerkey_event(&k, 2, 1100, &held) == IN_NONE);  // autorepeat changes nothing
    CHECK(k.down);
    CHECK(powerkey_event(&k, 0, 1300, &held) == IN_POWER_UP);
    CHECK(held == 300);
    CHECK(!k.down);
    // A release whose press was never seen sends nothing.
    CHECK(powerkey_event(&k, 0, 1400, &held) == IN_NONE);
}

TEST(hold_once) {
    PowerKey k = {0};
    int held = 0;
    g_down = 1;
    g_asked = 0;
    powerkey_event(&k, 1, 5000, &held);
    CHECK(powerkey_tick(&k, 5000 + INPUT_POWER_HOLD_MS - 1, mock_down, &held) == IN_NONE);
    CHECK(g_asked == 0);  // the device is only asked at the 4 s mark
    CHECK(powerkey_tick(&k, 5000 + INPUT_POWER_HOLD_MS, mock_down, &held) == IN_POWER_HELD);
    CHECK(held == INPUT_POWER_HOLD_MS);
    CHECK(g_asked == 1);
    CHECK(powerkey_tick(&k, 5000 + INPUT_POWER_HOLD_MS + 30, mock_down, &held) == IN_NONE);  // once per press
    CHECK(g_asked == 1);
    CHECK(powerkey_event(&k, 0, 5000 + 4500, &held) == IN_POWER_UP);
    CHECK(held == 4500);
    // A new press is followed afresh, hold included.
    powerkey_event(&k, 1, 20000, &held);
    CHECK(powerkey_tick(&k, 24000, mock_down, &held) == IN_POWER_HELD);
}

TEST(hold_needs_the_key_down) {
    PowerKey k = {0};
    int held = 0;
    // The device says the key is up: its release was lost, so no leaving, just a late release.
    g_down = 0;
    powerkey_event(&k, 1, 100, &held);
    CHECK(powerkey_tick(&k, 100 + INPUT_POWER_HOLD_MS, mock_down, &held) == IN_POWER_UP);
    CHECK(held == -1);
    CHECK(!k.down);
    CHECK(powerkey_tick(&k, 100 + 2 * INPUT_POWER_HOLD_MS, mock_down, &held) == IN_NONE);
    CHECK(powerkey_event(&k, 0, 9000, &held) == IN_NONE);
    // The device cannot tell (or there is no way to ask): the events are trusted.
    g_down = -1;
    powerkey_event(&k, 1, 10000, &held);
    CHECK(powerkey_tick(&k, 10000 + INPUT_POWER_HOLD_MS, mock_down, &held) == IN_POWER_HELD);
    powerkey_event(&k, 0, 15000, &held);
    powerkey_event(&k, 1, 20000, &held);
    CHECK(powerkey_tick(&k, 20000 + INPUT_POWER_HOLD_MS, NULL, &held) == IN_POWER_HELD);
    g_down = 1;
}

TEST(press_over_a_lost_release) {
    PowerKey k = {0};
    int held = 0;
    g_down = 1;
    powerkey_event(&k, 1, 1000, &held);
    // A second press with no release between: the old one is forgotten, the hold counts anew.
    CHECK(powerkey_event(&k, 1, 4000, &held) == IN_POWER);
    CHECK(powerkey_tick(&k, 1000 + INPUT_POWER_HOLD_MS, mock_down, &held) == IN_NONE);
    CHECK(powerkey_tick(&k, 4000 + INPUT_POWER_HOLD_MS, mock_down, &held) == IN_POWER_HELD);
}

TEST(clock_wraps) {
    PowerKey k = {0};
    int held = 0;
    g_down = 1;
    uint32_t t0 = 0xFFFFF000u;
    powerkey_event(&k, 1, t0, &held);
    CHECK(powerkey_tick(&k, t0 + 3000u, mock_down, &held) == IN_NONE);
    CHECK(powerkey_tick(&k, t0 + 4096u, mock_down, &held) == IN_POWER_HELD);  // past the wrap
    CHECK(held == 4096);
    CHECK(powerkey_event(&k, 0, t0 + 5000u, &held) == IN_POWER_UP);
    CHECK(held == 5000);
}

TEST(powerd_states) {
    CHECK(powerd_parse_state("active") == POWERD_ACTIVE);
    CHECK(powerd_parse_state("screenSaver\n") == POWERD_SCREENSAVER);
    CHECK(powerd_parse_state("  readyToSuspend ") == POWERD_READY_TO_SUSPEND);
    CHECK(powerd_parse_state("\"suspended\"") == POWERD_SUSPENDED);
    CHECK(powerd_parse_state("SCREENSAVER") == POWERD_SCREENSAVER);
    CHECK(powerd_parse_state("") == POWERD_UNKNOWN);
    CHECK(powerd_parse_state(NULL) == POWERD_UNKNOWN);
    CHECK(powerd_parse_state("screen") == POWERD_UNKNOWN);
    CHECK(powerd_parse_state("activeX") == POWERD_UNKNOWN);
    CHECK(powerd_parse_state("Failed to get property") == POWERD_UNKNOWN);
    // Only the screensaver and deeper keep Grimoire asleep (and off the power button); an awake or
    // unreadable state keeps the old behavior (wake, or ask for sleep).
    CHECK(!powerd_dozing(POWERD_ACTIVE));
    CHECK(!powerd_dozing(POWERD_UNKNOWN));
    CHECK(powerd_dozing(POWERD_SCREENSAVER));
    CHECK(powerd_dozing(POWERD_READY_TO_SUSPEND));
    CHECK(powerd_dozing(POWERD_SUSPENDED));
}

// A release read long after the key went up must be timed by the event stamps, not the read: the
// platform passes the kernel's times, so a short press stays short and a real hold stays a hold.
TEST(timed_by_event_stamps) {
    PowerKey k = {0};
    int held = 0;
    g_down = 1;
    powerkey_event(&k, 1, 1000, &held);              // the press, stamped 1000
    CHECK(powerkey_event(&k, 0, 1200, &held) == IN_POWER_UP);  // its release, stamped 1200 (read at 6000)
    CHECK(held == 200);
    CHECK(power_up_action(true, false, false, false, held) == POWER_UP_SLEEP_WAIT);
    // A real 4.5 s hold whose IN_POWER_HELD never went out (the loop was busy at the 4 s mark):
    // both stamps arrive in one read, and the release is the hold.
    powerkey_event(&k, 1, 20000, &held);
    CHECK(powerkey_event(&k, 0, 24500, &held) == IN_POWER_UP);
    CHECK(held == 4500);
    CHECK(power_up_action(true, false, false, false, held) == POWER_UP_LEAVE);
}

TEST(release_decisions) {
    // A short press while awake waits for goingToScreenSaver; on the sleep screen, for outOfScreenSaver.
    CHECK(power_up_action(true, false, false, false, 150) == POWER_UP_SLEEP_WAIT);
    CHECK(power_up_action(true, false, true, true, 150) == POWER_UP_WAKE_WAIT);
    // Held 3 s, short of the hold: still a short press (its fallback starts at the release).
    CHECK(power_up_action(true, false, false, false, 3000) == POWER_UP_SLEEP_WAIT);
    CHECK(power_up_action(true, false, false, false, INPUT_POWER_HOLD_MS - 1) == POWER_UP_SLEEP_WAIT);
    // A release whose time is unknown (it was lost, and found up at the 4 s check) is a short press.
    CHECK(power_up_action(true, false, false, false, -1) == POWER_UP_SLEEP_WAIT);
    CHECK(power_up_action(true, false, true, true, -1) == POWER_UP_WAKE_WAIT);
    // Answered while the key was down: a wake announced mid-press asks for no sleep afterwards, on
    // either screen (the ping-pong), and a sleep announced mid-press asks for no wake.
    CHECK(power_up_action(true, true, false, false, 150) == POWER_UP_NOTHING);
    CHECK(power_up_action(true, true, true, false, 150) == POWER_UP_NOTHING);
    CHECK(power_up_action(true, true, true, true, 150) == POWER_UP_NOTHING);
    CHECK(power_up_action(true, true, false, true, 150) == POWER_UP_NOTHING);
    // Grimoire slept or woke on its own while the key was down: the release is judged by the
    // screen its press went down on, so it asks for nothing.
    CHECK(power_up_action(true, false, false, true, 150) == POWER_UP_NOTHING);
    CHECK(power_up_action(true, false, true, false, 150) == POWER_UP_NOTHING);
    // Held 4 s but the hold never reached Grimoire: the release leaves, whatever happened meanwhile.
    CHECK(power_up_action(true, false, false, false, INPUT_POWER_HOLD_MS) == POWER_UP_LEAVE);
    CHECK(power_up_action(true, true, true, false, 9000) == POWER_UP_LEAVE);
    CHECK(power_up_action(true, false, true, true, 5000) == POWER_UP_LEAVE);
    // A release with no press followed (the hold already spent it, or it was never seen): nothing,
    // however long it claims.
    CHECK(power_up_action(false, false, false, false, 150) == POWER_UP_NOTHING);
    CHECK(power_up_action(false, false, false, false, INPUT_POWER_HOLD_MS + 200) == POWER_UP_NOTHING);
    CHECK(power_up_action(false, false, true, true, 150) == POWER_UP_NOTHING);
}

TEST(wake_fallback_locked_twice) {
    int locked = 0;
    // Awake or unreadable: wake, and nothing is counted.
    CHECK(power_wake_fallback(POWERD_ACTIVE, &locked) == WAKE_FALLBACK_WAKE);
    CHECK(power_wake_fallback(POWERD_UNKNOWN, &locked) == WAKE_FALLBACK_WAKE);
    CHECK(locked == 0);
    // Still locked: stay on the sleep screen once, leave the second time in a row.
    CHECK(power_wake_fallback(POWERD_SCREENSAVER, &locked) == WAKE_FALLBACK_STAY);
    CHECK(locked == 1);
    CHECK(power_wake_fallback(POWERD_SCREENSAVER, &locked) == WAKE_FALLBACK_LEAVE);
    // A wake in between starts the count again (app_wake also resets it, for outOfScreenSaver).
    locked = 0;
    CHECK(power_wake_fallback(POWERD_READY_TO_SUSPEND, &locked) == WAKE_FALLBACK_STAY);
    CHECK(power_wake_fallback(POWERD_ACTIVE, &locked) == WAKE_FALLBACK_WAKE);
    CHECK(locked == 0);
    CHECK(power_wake_fallback(POWERD_SUSPENDED, &locked) == WAKE_FALLBACK_STAY);
    CHECK(power_wake_fallback(POWERD_SCREENSAVER, &locked) == WAKE_FALLBACK_LEAVE);
}

static InEvent ev_of(InType t, bool side) {
    InEvent e = {0};
    e.type = t;
    e.side_button = side;
    return e;
}

TEST(asleep_drops) {
    // Pen and touch, the side button's press and a pinch's start and moves are ignored on the
    // sleep screen (three clicks cleared the page, a pinch zoomed it behind the sleep screen).
    static const InType dropped[] = {IN_PEN_DOWN,   IN_PEN_MOVE,   IN_PEN_UP,      IN_TOUCH_DOWN,
                                     IN_TOUCH_MOVE, IN_TOUCH_UP,   IN_PINCH_BEGIN, IN_PINCH_MOVE};
    for (size_t i = 0; i < sizeof dropped / sizeof dropped[0]; i++) {
        InEvent e = ev_of(dropped[i], false);
        CHECK(power_asleep_drops(&e));
    }
    InEvent press = ev_of(IN_PEN_BUTTON, true), release = ev_of(IN_PEN_BUTTON, false);
    CHECK(power_asleep_drops(&press));
    // The ends get through: a button that went down before the sleep must come up (its hold would
    // clear the page after the wake), and a pinch begun before it must finish.
    CHECK(!power_asleep_drops(&release));
    InEvent end = ev_of(IN_PINCH_END, false);
    CHECK(!power_asleep_drops(&end));
    // So does everything the sleep and wake logic itself needs.
    static const InType kept[] = {IN_POWER, IN_POWER_UP, IN_POWER_HELD, IN_SLEEP, IN_WAKE,
                                  IN_QUIT,  IN_ORIENTATION, IN_PEN_HOVER, IN_PEN_LEAVE};
    for (size_t i = 0; i < sizeof kept / sizeof kept[0]; i++) {
        InEvent e = ev_of(kept[i], false);
        CHECK(!power_asleep_drops(&e));
    }
}

int main(void) {
    RUN(press_release);
    RUN(hold_once);
    RUN(hold_needs_the_key_down);
    RUN(press_over_a_lost_release);
    RUN(clock_wraps);
    RUN(powerd_states);
    RUN(timed_by_event_stamps);
    RUN(release_decisions);
    RUN(wake_fallback_locked_twice);
    RUN(asleep_drops);
    return tl_done();
}
