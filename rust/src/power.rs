//! power.h: the power button and the Kindle's power service as pure logic (unit-tested on the
//! host): one press of the power key followed to its release or its 4 s hold (input_evdev.c), the
//! power service's state, which app.c reads before its sleep and wake fallbacks act, and the
//! decisions app.c takes on them.

use crate::prelude::*;

/// One press of the power key, as the platform layer follows it.
#[derive(Clone, Copy, Debug, Default)]
pub struct PowerKey {
    /// pressed, and no release seen yet
    pub down: bool,
    /// IN_POWER_HELD went out for this press
    pub held_sent: bool,
    /// when it went down (now_ms()'s clock)
    pub down_at: u32,
}

/// A KEY_POWER event (value 1 press, 0 release, 2 autorepeat) stamped `t`: when the kernel saw it,
/// in now_ms()'s clock, where the platform can tell (so a release read late, after a busy moment,
/// is not a longer hold), else when it was read. Returns IN_POWER for a press, IN_POWER_UP for the
/// release of a press it saw (*held_ms = how long the key was down), else IN_NONE (autorepeat, or a
/// release whose press it never saw).
pub fn powerkey_event(k: &mut PowerKey, value: i32, t: u32, held_ms: &mut i32) -> InType {
    *held_ms = 0;
    if value == 1 {
        // A press, even over one whose release never came: follow the new one.
        k.down = true;
        k.held_sent = false;
        k.down_at = t;
        return InType::Power;
    }
    if value == 0 && k.down {
        k.down = false;
        *held_ms = t.wrapping_sub(k.down_at) as i32;
        return InType::PowerUp;
    }
    InType::None
}
/// Called at every input poll. Once the key has been down INPUT_POWER_HOLD_MS it asks still_down()
/// (EVIOCGKEY: 1 down, 0 up, -1 cannot tell) and returns IN_POWER_HELD, once per press, unless the
/// key is found up: then its release was lost, and it returns IN_POWER_UP with *held_ms = -1.
/// Otherwise IN_NONE; still_down is only called at that moment.
pub fn powerkey_tick(k: &mut PowerKey, now: u32, still_down: &mut dyn FnMut() -> i32, held_ms: &mut i32) -> InType {
    *held_ms = 0;
    if !k.down || k.held_sent || (now.wrapping_sub(k.down_at) as i32) < INPUT_POWER_HOLD_MS {
        return InType::None;
    }
    // The events say it is still down: make sure before leaving Grimoire on it. If the device
    // cannot tell, trust the events (a key device does not drop them).
    if still_down() == 0 {
        k.down = false;
        *held_ms = -1;
        return InType::PowerUp;
    }
    k.held_sent = true;
    *held_ms = now.wrapping_sub(k.down_at) as i32;
    InType::PowerHeld
}

/// The power service's state (`lipc-get-prop -s com.lab126.powerd state`).
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PowerdState {
    /// unreadable, or a name this build does not know
    #[default]
    Unknown = 0,  // POWERD_UNKNOWN
    /// awake
    Active = 1,  // POWERD_ACTIVE
    /// its screensaver is up and input is locked (a passcode waits here)
    Screensaver = 2,  // POWERD_SCREENSAVER
    /// about to suspend
    ReadyToSuspend = 3,  // POWERD_READY_TO_SUSPEND
    Suspended = 4,  // POWERD_SUSPENDED
}

/// Parses the property's text: case, surrounding spaces and quotes are ignored; NULL, an empty
/// string or any other name give POWERD_UNKNOWN.
pub fn powerd_parse_state(s: Option<&str>) -> PowerdState {
    const K_NAMES: [(&str, PowerdState); 4] = [
        ("active", PowerdState::Active),
        ("screenSaver", PowerdState::Screensaver),
        ("readyToSuspend", PowerdState::ReadyToSuspend),
        ("suspended", PowerdState::Suspended),
    ];
    let Some(s) = s else {
        return PowerdState::Unknown;
    };
    // (a C string ends at its first NUL)
    let b = s.as_bytes();
    let b = &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())];
    let mut start = 0;
    while start < b.len() && (isspace(b[start]) || b[start] == b'"') {
        start += 1;
    }
    let mut n = b.len();
    while n > start && (isspace(b[n - 1]) || b[n - 1] == b'"') {
        n -= 1;
    }
    let s = &b[start..n];
    for (name, state) in K_NAMES {
        if name.len() == s.len() && name.as_bytes().eq_ignore_ascii_case(s) {
            return state;
        }
    }
    PowerdState::Unknown
}

/// isspace() in the C locale: space, \t, \n, \v, \f, \r (u8::is_ascii_whitespace leaves out \v).
fn isspace(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}
/// The screensaver or deeper: the Kindle is not awake and may be waiting for its passcode, so
/// Grimoire must neither draw over it nor send it a power button (that would wake it).
pub fn powerd_dozing(s: PowerdState) -> bool {
    s == PowerdState::Screensaver || s == PowerdState::ReadyToSuspend || s == PowerdState::Suspended
}

/// What the release of a power button press asks of Grimoire (app.c arms the matching fallback).
/// `followed`: Grimoire saw the press go down (and has not spent it on a hold); `answered`: a sleep
/// or wake announcement came while it was down; `down_asleep`, `asleep`: whether Grimoire showed its
/// sleep screen when the press went down, and now; `held_ms`: IN_POWER_UP's value (-1 unknown).
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PowerUpAction {
    /// not followed, answered already, or Grimoire slept or woke meanwhile
    #[default]
    Nothing = 0,  // POWER_UP_NOTHING
    /// a short press while awake: wait 2.5 s for goingToScreenSaver
    SleepWait = 1,  // POWER_UP_SLEEP_WAIT
    /// a short press on the sleep screen: wait 3 s for outOfScreenSaver
    WakeWait = 2,  // POWER_UP_WAKE_WAIT
    /// held INPUT_POWER_HOLD_MS, but the hold never reached Grimoire: leave
    Leave = 3,  // POWER_UP_LEAVE
}
pub fn power_up_action(followed: bool, answered: bool, down_asleep: bool, asleep: bool, held_ms: i32) -> PowerUpAction {
    if !followed {
        return PowerUpAction::Nothing;
    }
    // Held past the 4 s mark while Grimoire was busy (its IN_POWER_HELD never came): the release is
    // the hold, answered or not, as the hold itself would have been.
    if held_ms >= INPUT_POWER_HOLD_MS {
        return PowerUpAction::Leave;
    }
    // The power service answered (a wake announced while the key was down must not turn the release
    // into a sleep request), or Grimoire slept or woke on its own while the key was down.
    if answered || down_asleep != asleep {
        return PowerUpAction::Nothing;
    }
    if asleep {
        PowerUpAction::WakeWait
    } else {
        PowerUpAction::SleepWait
    }
}

/// The wake fallback: a short press on the sleep screen, and no outOfScreenSaver 3 s later. With the
/// power service awake (or unreadable), wake. Still in its screensaver or deeper, the Kindle is
/// locked (its passcode) or asleep: stay on the sleep screen, and the second time in a row leave
/// Grimoire, so the Kindle gets its screen, pen and touch back even on a device whose power key
/// cannot report a 4 s hold. `*locked` counts those in a row; a wake (any) resets it.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WakeFallback {
    #[default]
    Wake = 0,  // WAKE_FALLBACK_WAKE
    Stay = 1,  // WAKE_FALLBACK_STAY
    Leave = 2,  // WAKE_FALLBACK_LEAVE
}
pub fn power_wake_fallback(st: PowerdState, locked: &mut i32) -> WakeFallback {
    if !powerd_dozing(st) {
        *locked = 0;
        return WakeFallback::Wake;
    }
    *locked += 1;
    if *locked >= 2 {
        WakeFallback::Leave
    } else {
        WakeFallback::Stay
    }
}

/// Input Grimoire ignores while it shows its sleep screen (or the Kindle's): pen and touch, and the
/// start of a side button press or of a pinch (three clicks cleared the page, a pinch zoomed it).
/// Their ends still get through, so one that began before the sleep finishes: a button left down
/// would clear the page by its hold right after the wake.
pub fn power_asleep_drops(ev: &InEvent) -> bool {
    match ev.type_ {
        InType::Key
        | InType::PenDown
        | InType::PenMove
        | InType::PenUp
        | InType::TouchDown
        | InType::TouchMove
        | InType::TouchUp
        | InType::PinchBegin
        | InType::PinchMove => true,
        InType::PenButton => ev.side_button, // the press; its release gets through
        _ => false,
    }
}
