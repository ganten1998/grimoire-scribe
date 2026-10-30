// Exercise the real evdev state machine on the host. These tests use synthetic events and pipe
// descriptors only; they never open input hardware. The normal host platform remains unchanged.
#![allow(dead_code)]
pub use grimoire::*;
#[path = "../src/input_evdev.rs"]
mod input_evdev;
