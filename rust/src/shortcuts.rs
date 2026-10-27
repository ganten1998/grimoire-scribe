//! Optional keyboard/gamepad mappings. Linux evdev key numbers are stable across Kindle models.

use crate::prelude::*;

pub const SHORTCUT_MAX: usize = 64;
pub const KEYMOD_CTRL: i32 = 1;
pub const KEYMOD_SHIFT: i32 = 2;
pub const KEYMOD_ALT: i32 = 4;
pub const KEYMOD_META: i32 = 8;
/// plus ToolKind (0..9)
pub const SHORTCUT_NONE: i32 = 0;
pub const SHORTCUT_TOOL_BASE: i32 = 1;
pub const SHORTCUT_UNDO: i32 = 20;
pub const SHORTCUT_REDO: i32 = 21;
pub const SHORTCUT_PREV_PAGE: i32 = 22;
pub const SHORTCUT_NEXT_PAGE: i32 = 23;
pub const SHORTCUT_PREV_TOOL: i32 = 24;
pub const SHORTCUT_NEXT_TOOL: i32 = 25;
pub const SHORTCUT_WHITE: i32 = 26;
pub const SHORTCUT_INDEX: i32 = 27;
/// 2.0.0-beta.5 (the files name actions; these numbers stay in memory). Erase while held acts
/// from its press to its release; a menu's own button closes it again (docs/SHORTCUTS.md).
pub const SHORTCUT_HOLD_ERASER: i32 = 28;
pub const SHORTCUT_ERASER_MODE: i32 = 29;
pub const SHORTCUT_CLEAR_PAGE: i32 = 30;
pub const SHORTCUT_FIRST_PAGE: i32 = 31;
pub const SHORTCUT_LAST_PAGE: i32 = 32;
pub const SHORTCUT_NEW_PAGE: i32 = 33;
pub const SHORTCUT_FIT_PAGE: i32 = 34;
pub const SHORTCUT_PAGES: i32 = 35;
pub const SHORTCUT_TOOL_PICKER: i32 = 36;
pub const SHORTCUT_PAPER: i32 = 37;
pub const SHORTCUT_DISPLAY: i32 = 38;
pub const SHORTCUT_PEN_FEEL: i32 = 39;
pub const SHORTCUT_FOLD_PANEL: i32 = 40;
pub const SHORTCUT_LIBRARY: i32 = 41;
pub const SHORTCUT_CLOSE_MENU: i32 = 42;
/// Next and previous in the tool picker: its choice while it is open, else the tool, through its
/// nine pens (next and previous tool go through the panel's).
pub const SHORTCUT_PICKER_PREV: i32 = 43;
pub const SHORTCUT_PICKER_NEXT: i32 = 44;
/// From the page itself: the Index's Add a link without the Pages view (the next ink stroke is its
/// box, then the Pages view picks where it goes), and its naming keyboard for the open page, which
/// goes back to the page.
pub const SHORTCUT_ADD_LINK: i32 = 45;
pub const SHORTCUT_NAME_PAGE: i32 = 46;
pub const SHORTCUT_LAST: i32 = 46;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub code: i32,
    pub mods: i32,
    pub action: i32,
    pub gamepad: bool,
}
#[derive(Clone, Debug)]
pub struct Shortcuts {
    pub bindings: [ShortcutBinding; SHORTCUT_MAX],
    pub count: i32,
    pub rejected: i32,
}
impl Default for Shortcuts {
    fn default() -> Self {
        Shortcuts { bindings: [ShortcutBinding::default(); SHORTCUT_MAX], count: 0, rejected: 0 }
    }
}
pub fn shortcut_keycode(name: &str) -> i32 {
    todo!()
}
pub fn shortcut_action(name: &str) -> i32 {
    todo!()
}
/// Missing file: defaults. Existing empty file: no bindings. Never rewrites user mappings.
pub fn shortcuts_load(map: &mut Shortcuts, path: &str) {
    todo!()
}
pub fn shortcuts_lookup(map: &Shortcuts, gamepad: bool, code: i32, mods: i32) -> i32 {
    todo!()
}
/// creates only if absent
pub fn shortcuts_write_defaults(path: &str) -> i32 {
    todo!()
}
/// controller.ini (Settings > Controller > Buttons): the gamepad buttons learned there. While it is
/// there they come from it alone (shortcuts.ini's pad.* lines are set aside); key.* lines are
/// always shortcuts.ini's. Grimoire writes it whole, atomically; a line it cannot read is rejected
/// and counted, as in shortcuts.ini. A shortcuts.ini that binds no D-pad direction (one written
/// before 2.0.0-beta.5) gets the D-pad's starter lines in memory; the file is not changed.
pub fn shortcuts_load_all(map: &mut Shortcuts, shortcuts_path: &str, controller_path: &str) {
    todo!()
}
/// Gamepad button code does action from now on, as its only button: the button's old action and the
/// action's other buttons lose theirs. Writes controller.ini with every gamepad binding; -1 when it
/// could not (map unchanged).
pub fn shortcuts_learn(map: &mut Shortcuts, controller_path: &str, code: i32, action: i32) -> i32 {
    todo!()
}
/// The gamepad buttons for action, and for action2 (0: none), do nothing from now on (a Buttons sheet
/// row held): controller.ini is written with every other gamepad binding, as shortcuts_learn writes
/// it, so shortcuts.ini's pad. lines (and the D-pad's starter lines) stay set aside. Nothing is
/// written when neither had a button. -1 when it could not be written (map unchanged).
pub fn shortcuts_forget(map: &mut Shortcuts, controller_path: &str, action: i32, action2: i32) -> i32 {
    todo!()
}
/// The gamepad buttons for action: how many there are, the first cap of them in codes.
pub fn shortcuts_pads_for(map: &Shortcuts, action: i32, codes: &mut [i32]) -> i32 {
    todo!()
}
/// A gamepad button as the files name it: "South", "Up", or its number ("316").
pub fn shortcut_pad_name(code: i32, out: &mut String, cap: usize) {
    todo!()
}
/// A gamepad button as the controller labels it, for the Buttons sheet: a Switch-mode one's (B, A, Y,
/// X, L, R, ZL, ZR, -, +, LS, RS, Home, Capture: hidkeys_switch), else as the files name it.
pub fn shortcut_pad_label(code: i32, switch_layout: bool, out: &mut String, cap: usize) {
    todo!()
}
/// An action's name in the files ("undo", "tool:eraser"), or None.
pub fn shortcut_action_name(action: i32) -> Option<&'static str> {
    todo!()
}
