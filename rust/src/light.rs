//! light.h: the Kindle's front light (brightness + warmth), through the system power service so
//! the stock UI stays in sync (same lipc properties KOReader uses: com.lab126.powerd flIntensity
//! and currentAmberLevel). Implemented by light_kindle.c and the simulator.

use crate::prelude::*;

// light.h is implemented by light_kindle.c on the Kindle and by the simulator (sim_sdl.c); host
// builds link none (platform_none.rs). These forward to it.
#[cfg(feature = "kindle")]
use crate::light_kindle as platform;
#[cfg(not(any(feature = "kindle", feature = "sim")))]
use crate::platform_none as platform;
#[cfg(feature = "sim")]
use crate::sim_sdl as platform;

pub const LIGHT_MAX: i32 = 24;

#[inline]
pub fn light_available() -> bool {
    platform::light_available()
}
/// Asks the power service for the levels (blocks: two lipc runs, 95-191 ms on the Scribe 2). Once, at
/// the start, before the pens are opened; light_get asks again only while the brightness is unknown.
#[inline]
pub fn light_refresh() {
    platform::light_refresh()
}
/// Current levels (0..LIGHT_MAX) as last read or set; -1 where unknown.
#[inline]
pub fn light_get(intensity: &mut i32, warmth: &mut i32) {
    platform::light_get(intensity, warmth)
}
#[inline]
pub fn light_set_intensity(level: i32) {
    platform::light_set_intensity(level)
}
#[inline]
pub fn light_set_warmth(level: i32) {
    platform::light_set_warmth(level)
}
/// The warmth as last read (light_get) or set, without asking the power service: -1 before either,
/// or on a Kindle without warm light.
#[inline]
pub fn light_warmth() -> i32 {
    platform::light_warmth()
}

/// Other bits of the Kindle system Grimoire drives.
/// Ask the power service to go to sleep, exactly as a power button press would. It turns
/// preventScreenSaver off first, in the same shell (the button is ignored while it is on), so a
/// "Sleep after: never" policy has to be applied again afterwards.
#[inline]
pub fn sys_request_sleep() {
    platform::sys_request_sleep()
}
/// The power service's state, best effort (`lipc-get-prop -s com.lab126.powerd state`): 0 with e.g.
/// active, screenSaver, readyToSuspend or suspended in `out`, or -1 if it cannot be read. Blocks
/// (lipc can take half a second; at most about 1.5 s): only for the rare sleep and wake fallbacks,
/// never on the drawing path.
#[inline]
pub fn sys_powerd_state(out: &mut String, cap: usize) -> i32 {
    platform::sys_powerd_state(out, cap)
}
/// Resume / pause the Kindle's window manager (only if the launcher paused it): the Kindle's own
/// screensaver can only show while it runs.
#[inline]
pub fn sys_kindle_ui(run: bool) -> bool {
    platform::sys_kindle_ui(run)
}
/// Keep the power service from sleeping on its own (only for "Sleep after: never").
#[inline]
pub fn sys_prevent_sleep(prevent: bool) {
    platform::sys_prevent_sleep(prevent)
}
/// Tell the power service the user is active (Grimoire grabs the pen and touch, so it cannot see).
#[inline]
pub fn sys_activity() {
    platform::sys_activity()
}
