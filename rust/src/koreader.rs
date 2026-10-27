//! KOReader discovery and optional integration files. No KOReader settings are executed.

use crate::prelude::*;

pub fn koreader_find(out: &mut String, cap: usize) -> bool {
    todo!()
}
/// 0 plugin, 1 screenshot gestures, 2 square corners, 3 Terminess menus.
pub fn koreader_item_name(item: i32) -> &'static str {
    todo!()
}
pub fn koreader_item_installed(item: i32) -> bool {
    todo!()
}
/// Installs only our files. A different file at the same path is kept (returns -1).
pub fn koreader_item_set(item: i32, enabled: bool) -> i32 {
    todo!()
}
