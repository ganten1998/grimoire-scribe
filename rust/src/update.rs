//! update.h: Settings > Updates (docs/KPM.md, Updates). Grimoire's updates come through KPM, the
//! Kindle's package manager, from Grimoire's own feed: kindle/app/update.sh adds the chosen channel's
//! feed to KPM (and takes the other channel's away: KPM installs the newest Grimoire any feed it knows
//! lists), refreshes KPM's index and lists what it offers. KPM does the downloading (Grimoire has no
//! HTTPS of its own). An update itself cannot run inside Grimoire (KPM's hooks refuse while it runs):
//! Grimoire writes the request and leaves, and the launcher hands it to update.sh, which installs the
//! newest Grimoire with KPM once the launcher is gone, and starts it again.

use crate::prelude::*;

pub const UPDATE_FEED_PREVIEW: &str = "https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json";
pub const UPDATE_FEED_STABLE: &str = "https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/stable/manifest.json";
pub const UPDATE_VERSION_LEN: usize = 24;

// ---- pure parts (tests/test_update.c) ----

/// A Grimoire version as KPM orders it (tools/kpm-package.py): "2.0.0-beta.5" is 2.0.1005,
/// "2.0.0-rc.1" 2.0.5001, "2.0.0" 2.0.9999, "2.0.1" 2.0.19999. False for anything else.
pub fn update_version_key(version: &str, key: &mut [u32; 3]) -> bool {
    todo!()
}
/// <0, 0, >0 as version a is older than, the same as, or newer than b; unreadable ones are oldest.
pub fn update_version_cmp(a: &str, b: &str) -> i32 {
    todo!()
}
/// A prerelease (beta or rc): its updates come from the preview channel unless told otherwise.
pub fn update_is_prerelease(version: &str) -> bool {
    todo!()
}
/// The newest Grimoire in `kpm search grimoire`'s output (each package a line "- grimoire (Grimoire):
/// <description>", the description ending in its version; colour codes and other packages skipped).
/// False when it lists none.
pub fn update_latest(search_output: &str, version: &mut String /* at most UPDATE_VERSION_LEN - 1 bytes */) -> bool {
    todo!()
}

// ---- on the Kindle ----

/// script: kindle/app/update.sh (the launcher says where), or NULL: no updates here. request: the file
/// "Update now" writes for the launcher (it says where too; without a launcher, the simulator's, the
/// caller passes one in the data folder).
pub fn update_init(script: Option<&str>, request: Option<&str>) {
    todo!()
}
pub fn update_request_path() -> String {
    todo!()
}
pub fn update_available() -> bool {
    todo!()
}
/// Checks the channel (preview or stable) in a forked child; false while one is under way.
pub fn update_check(preview: bool) -> bool {
    todo!()
}
pub fn update_checking() -> bool {
    todo!()
}
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UpdateEvent {
    #[default]
    None = 0,  // UPDATE_EV_NONE
    Done = 1,  // UPDATE_EV_DONE
    Failed = 2,  // UPDATE_EV_FAILED
}
/// Reaps the check: UPDATE_EV_DONE with the newest version it offers in *latest ("" when it offers
/// none), UPDATE_EV_FAILED when the check did not work (no network, no KPM: *why says which).
pub fn update_tick(latest: &mut String /* at most UPDATE_VERSION_LEN - 1 bytes */, why: &mut String, why_cap: usize) -> UpdateEvent {
    todo!()
}
/// Writes the request ("preview" or "stable") for the launcher. 0, or -1 when it could not.
pub fn update_request(preview: bool) -> i32 {
    todo!()
}
