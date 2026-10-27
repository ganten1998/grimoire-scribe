// test_power.c: the power key followed to its release or its 4 s hold, the power service's state
// as the sleep and wake fallbacks read it, and what Grimoire decides from them.
//
// The C's tests give powerkey_tick a function pointer, or NULL for "no way to ask". Here the device
// is a closure; one that answers -1 ("cannot tell") stands for NULL: both leave the events trusted.
mod testlib;
use grimoire::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};
use testlib::*;

static G_DOWN: AtomicI32 = AtomicI32::new(1);
static G_ASKED: AtomicI32 = AtomicI32::new(0);
fn mock_down() -> i32 {
    G_ASKED.fetch_add(1, Ordering::Relaxed);
    G_DOWN.load(Ordering::Relaxed)
}
fn g_asked() -> i32 {
    G_ASKED.load(Ordering::Relaxed)
}

fn press_release() {
    let mut k = PowerKey::default();
    let mut held = -5;
    check!(powerkey_event(&mut k, 1, 1000, &mut held) == InType::Power);
    check!(k.down);
    check!(powerkey_event(&mut k, 2, 1100, &mut held) == InType::None); // autorepeat changes nothing
    check!(k.down);
    check!(powerkey_event(&mut k, 0, 1300, &mut held) == InType::PowerUp);
    check!(held == 300);
    check!(!k.down);
    // A release whose press was never seen sends nothing.
    check!(powerkey_event(&mut k, 0, 1400, &mut held) == InType::None);
}

fn hold_once() {
    let mut k = PowerKey::default();
    let mut held = 0;
    G_DOWN.store(1, Ordering::Relaxed);
    G_ASKED.store(0, Ordering::Relaxed);
    powerkey_event(&mut k, 1, 5000, &mut held);
    check!(powerkey_tick(&mut k, 5000 + INPUT_POWER_HOLD_MS as u32 - 1, &mut mock_down, &mut held) == InType::None);
    check!(g_asked() == 0); // the device is only asked at the 4 s mark
    check!(powerkey_tick(&mut k, 5000 + INPUT_POWER_HOLD_MS as u32, &mut mock_down, &mut held) == InType::PowerHeld);
    check!(held == INPUT_POWER_HOLD_MS);
    check!(g_asked() == 1);
    check!(powerkey_tick(&mut k, 5000 + INPUT_POWER_HOLD_MS as u32 + 30, &mut mock_down, &mut held) == InType::None); // once per press
    check!(g_asked() == 1);
    check!(powerkey_event(&mut k, 0, 5000 + 4500, &mut held) == InType::PowerUp);
    check!(held == 4500);
    // A new press is followed afresh, hold included.
    powerkey_event(&mut k, 1, 20000, &mut held);
    check!(powerkey_tick(&mut k, 24000, &mut mock_down, &mut held) == InType::PowerHeld);
}

fn hold_needs_the_key_down() {
    let mut k = PowerKey::default();
    let mut held = 0;
    // The device says the key is up: its release was lost, so no leaving, just a late release.
    G_DOWN.store(0, Ordering::Relaxed);
    powerkey_event(&mut k, 1, 100, &mut held);
    check!(powerkey_tick(&mut k, 100 + INPUT_POWER_HOLD_MS as u32, &mut mock_down, &mut held) == InType::PowerUp);
    check!(held == -1);
    check!(!k.down);
    check!(powerkey_tick(&mut k, 100 + 2 * INPUT_POWER_HOLD_MS as u32, &mut mock_down, &mut held) == InType::None);
    check!(powerkey_event(&mut k, 0, 9000, &mut held) == InType::None);
    // The device cannot tell (or there is no way to ask): the events are trusted.
    G_DOWN.store(-1, Ordering::Relaxed);
    powerkey_event(&mut k, 1, 10000, &mut held);
    check!(powerkey_tick(&mut k, 10000 + INPUT_POWER_HOLD_MS as u32, &mut mock_down, &mut held) == InType::PowerHeld);
    powerkey_event(&mut k, 0, 15000, &mut held);
    powerkey_event(&mut k, 1, 20000, &mut held);
    check!(powerkey_tick(&mut k, 20000 + INPUT_POWER_HOLD_MS as u32, &mut || -1, &mut held) == InType::PowerHeld); // (NULL)
    G_DOWN.store(1, Ordering::Relaxed);
}

fn press_over_a_lost_release() {
    let mut k = PowerKey::default();
    let mut held = 0;
    G_DOWN.store(1, Ordering::Relaxed);
    powerkey_event(&mut k, 1, 1000, &mut held);
    // A second press with no release between: the old one is forgotten, the hold counts anew.
    check!(powerkey_event(&mut k, 1, 4000, &mut held) == InType::Power);
    check!(powerkey_tick(&mut k, 1000 + INPUT_POWER_HOLD_MS as u32, &mut mock_down, &mut held) == InType::None);
    check!(powerkey_tick(&mut k, 4000 + INPUT_POWER_HOLD_MS as u32, &mut mock_down, &mut held) == InType::PowerHeld);
}

fn clock_wraps() {
    let mut k = PowerKey::default();
    let mut held = 0;
    G_DOWN.store(1, Ordering::Relaxed);
    let t0: u32 = 0xFFFFF000;
    powerkey_event(&mut k, 1, t0, &mut held);
    check!(powerkey_tick(&mut k, t0.wrapping_add(3000), &mut mock_down, &mut held) == InType::None);
    check!(powerkey_tick(&mut k, t0.wrapping_add(4096), &mut mock_down, &mut held) == InType::PowerHeld); // past the wrap
    check!(held == 4096);
    check!(powerkey_event(&mut k, 0, t0.wrapping_add(5000), &mut held) == InType::PowerUp);
    check!(held == 5000);
}

fn powerd_states() {
    check!(powerd_parse_state(Some("active")) == PowerdState::Active);
    check!(powerd_parse_state(Some("screenSaver\n")) == PowerdState::Screensaver);
    check!(powerd_parse_state(Some("  readyToSuspend ")) == PowerdState::ReadyToSuspend);
    check!(powerd_parse_state(Some("\"suspended\"")) == PowerdState::Suspended);
    check!(powerd_parse_state(Some("SCREENSAVER")) == PowerdState::Screensaver);
    check!(powerd_parse_state(Some("")) == PowerdState::Unknown);
    check!(powerd_parse_state(None) == PowerdState::Unknown);
    check!(powerd_parse_state(Some("screen")) == PowerdState::Unknown);
    check!(powerd_parse_state(Some("activeX")) == PowerdState::Unknown);
    check!(powerd_parse_state(Some("Failed to get property")) == PowerdState::Unknown);
    // Only the screensaver and deeper keep Grimoire asleep (and off the power button); an awake or
    // unreadable state keeps the old behavior (wake, or ask for sleep).
    check!(!powerd_dozing(PowerdState::Active));
    check!(!powerd_dozing(PowerdState::Unknown));
    check!(powerd_dozing(PowerdState::Screensaver));
    check!(powerd_dozing(PowerdState::ReadyToSuspend));
    check!(powerd_dozing(PowerdState::Suspended));
}

// A release read long after the key went up must be timed by the event stamps, not the read: the
// platform passes the kernel's times, so a short press stays short and a real hold stays a hold.
fn timed_by_event_stamps() {
    let mut k = PowerKey::default();
    let mut held = 0;
    G_DOWN.store(1, Ordering::Relaxed);
    powerkey_event(&mut k, 1, 1000, &mut held); // the press, stamped 1000
    check!(powerkey_event(&mut k, 0, 1200, &mut held) == InType::PowerUp); // its release, stamped 1200 (read at 6000)
    check!(held == 200);
    check!(power_up_action(true, false, false, false, held) == PowerUpAction::SleepWait);
    // A real 4.5 s hold whose IN_POWER_HELD never went out (the loop was busy at the 4 s mark):
    // both stamps arrive in one read, and the release is the hold.
    powerkey_event(&mut k, 1, 20000, &mut held);
    check!(powerkey_event(&mut k, 0, 24500, &mut held) == InType::PowerUp);
    check!(held == 4500);
    check!(power_up_action(true, false, false, false, held) == PowerUpAction::Leave);
}

fn release_decisions() {
    // A short press while awake waits for goingToScreenSaver; on the sleep screen, for outOfScreenSaver.
    check!(power_up_action(true, false, false, false, 150) == PowerUpAction::SleepWait);
    check!(power_up_action(true, false, true, true, 150) == PowerUpAction::WakeWait);
    // Held 3 s, short of the hold: still a short press (its fallback starts at the release).
    check!(power_up_action(true, false, false, false, 3000) == PowerUpAction::SleepWait);
    check!(power_up_action(true, false, false, false, INPUT_POWER_HOLD_MS - 1) == PowerUpAction::SleepWait);
    // A release whose time is unknown (it was lost, and found up at the 4 s check) is a short press.
    check!(power_up_action(true, false, false, false, -1) == PowerUpAction::SleepWait);
    check!(power_up_action(true, false, true, true, -1) == PowerUpAction::WakeWait);
    // Answered while the key was down: a wake announced mid-press asks for no sleep afterwards, on
    // either screen (the ping-pong), and a sleep announced mid-press asks for no wake.
    check!(power_up_action(true, true, false, false, 150) == PowerUpAction::Nothing);
    check!(power_up_action(true, true, true, false, 150) == PowerUpAction::Nothing);
    check!(power_up_action(true, true, true, true, 150) == PowerUpAction::Nothing);
    check!(power_up_action(true, true, false, true, 150) == PowerUpAction::Nothing);
    // Grimoire slept or woke on its own while the key was down: the release is judged by the
    // screen its press went down on, so it asks for nothing.
    check!(power_up_action(true, false, false, true, 150) == PowerUpAction::Nothing);
    check!(power_up_action(true, false, true, false, 150) == PowerUpAction::Nothing);
    // Held 4 s but the hold never reached Grimoire: the release leaves, whatever happened meanwhile.
    check!(power_up_action(true, false, false, false, INPUT_POWER_HOLD_MS) == PowerUpAction::Leave);
    check!(power_up_action(true, true, true, false, 9000) == PowerUpAction::Leave);
    check!(power_up_action(true, false, true, true, 5000) == PowerUpAction::Leave);
    // A release with no press followed (the hold already spent it, or it was never seen): nothing,
    // however long it claims.
    check!(power_up_action(false, false, false, false, 150) == PowerUpAction::Nothing);
    check!(power_up_action(false, false, false, false, INPUT_POWER_HOLD_MS + 200) == PowerUpAction::Nothing);
    check!(power_up_action(false, false, true, true, 150) == PowerUpAction::Nothing);
}

fn wake_fallback_locked_twice() {
    let mut locked = 0;
    // Awake or unreadable: wake, and nothing is counted.
    check!(power_wake_fallback(PowerdState::Active, &mut locked) == WakeFallback::Wake);
    check!(power_wake_fallback(PowerdState::Unknown, &mut locked) == WakeFallback::Wake);
    check!(locked == 0);
    // Still locked: stay on the sleep screen once, leave the second time in a row.
    check!(power_wake_fallback(PowerdState::Screensaver, &mut locked) == WakeFallback::Stay);
    check!(locked == 1);
    check!(power_wake_fallback(PowerdState::Screensaver, &mut locked) == WakeFallback::Leave);
    // A wake in between starts the count again (app_wake also resets it, for outOfScreenSaver).
    locked = 0;
    check!(power_wake_fallback(PowerdState::ReadyToSuspend, &mut locked) == WakeFallback::Stay);
    check!(power_wake_fallback(PowerdState::Active, &mut locked) == WakeFallback::Wake);
    check!(locked == 0);
    check!(power_wake_fallback(PowerdState::Suspended, &mut locked) == WakeFallback::Stay);
    check!(power_wake_fallback(PowerdState::Screensaver, &mut locked) == WakeFallback::Leave);
}

fn ev_of(t: InType, side: bool) -> InEvent {
    InEvent { type_: t, side_button: side, ..Default::default() }
}

fn asleep_drops() {
    // Pen and touch, the side button's press and a pinch's start and moves are ignored on the
    // sleep screen (three clicks cleared the page, a pinch zoomed it behind the sleep screen).
    const DROPPED: [InType; 9] = [
        InType::PenDown,
        InType::PenMove,
        InType::PenUp,
        InType::TouchDown,
        InType::TouchMove,
        InType::TouchUp,
        InType::PinchBegin,
        InType::PinchMove,
        InType::Key,
    ];
    for t in DROPPED {
        let e = ev_of(t, false);
        check!(power_asleep_drops(&e));
    }
    let press = ev_of(InType::PenButton, true);
    let release = ev_of(InType::PenButton, false);
    check!(power_asleep_drops(&press));
    // The ends get through: a button that went down before the sleep must come up (its hold would
    // clear the page after the wake), and a pinch begun before it must finish.
    check!(!power_asleep_drops(&release));
    let end = ev_of(InType::PinchEnd, false);
    check!(!power_asleep_drops(&end));
    let key_up = ev_of(InType::KeyUp, false); // (a controller's Erase while held ends with it)
    check!(!power_asleep_drops(&key_up));
    // So does everything the sleep and wake logic itself needs.
    const KEPT: [InType; 9] = [
        InType::Power,
        InType::PowerUp,
        InType::PowerHeld,
        InType::Sleep,
        InType::Wake,
        InType::Quit,
        InType::Orientation,
        InType::PenHover,
        InType::PenLeave,
    ];
    for t in KEPT {
        let e = ev_of(t, false);
        check!(!power_asleep_drops(&e));
    }
}

#[test]
fn test_power() {
    run!(press_release);
    run!(hold_once);
    run!(hold_needs_the_key_down);
    run!(press_over_a_lost_release);
    run!(clock_wraps);
    run!(powerd_states);
    run!(timed_by_event_stamps);
    run!(release_decisions);
    run!(wake_fallback_locked_twice);
    run!(asleep_drops);
    done();
}
