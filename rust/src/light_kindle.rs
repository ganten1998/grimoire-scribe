//! light_kindle.c: front light through lipc (com.lab126.powerd), as the stock UI and KOReader do.

use crate::prelude::*;

// ---- light.h (light.rs forwards here on the Kindle) ----

pub fn light_available() -> bool {
    todo!()
}
pub fn light_refresh() {
    todo!()
}
pub fn light_get(intensity: &mut i32, warmth: &mut i32) {
    todo!()
}
pub fn light_set_intensity(level: i32) {
    todo!()
}
pub fn light_set_warmth(level: i32) {
    todo!()
}
pub fn light_warmth() -> i32 {
    todo!()
}
pub fn sys_request_sleep() {
    todo!()
}
pub fn sys_powerd_state(out: &mut String, cap: usize) -> i32 {
    todo!()
}
pub fn sys_kindle_ui(run: bool) -> bool {
    todo!()
}
pub fn sys_prevent_sleep(prevent: bool) {
    todo!()
}
pub fn sys_activity() {
    todo!()
}
