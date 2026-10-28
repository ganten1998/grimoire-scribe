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
/// The clock and battery corner on or off, on any screen (status.rs; not in the C tree).
pub const SHORTCUT_STATUS: i32 = 47;
pub const SHORTCUT_LAST: i32 = 47;
/// The Links bar: two fingers up open it (gesturemap.rs's built-in). Not a name in the files yet.
pub const SHORTCUT_LINKS: i32 = 48;
/// Where an action came from: a key, a controller's button, a touch gesture (gestures.ini), and
/// later a finger shape or the pen's button. Page turns from a gesture never add a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionSrc {
    Key,
    Pad,
    Touch,
    Shape,
    Pen,
}
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

/// The D-pad's starter lines: in a new shortcuts.ini, and in memory for one that binds none of its
/// directions (shortcuts_load_all). (A macro, as concat! reads only literals.)
macro_rules! dpad_lines {
    () => {
        "pad.Up=previous-tool\npad.Down=next-tool\npad.Left=previous-page\npad.Right=next-page\n"
    };
}
const DPAD_LINES: &str = dpad_lines!();
const DEFAULTS: &str = concat!(
    "# Grimoire shortcuts. Enable in Settings > Controller. Reload there after editing.\n",
    "# key.NAME or key.Ctrl+NAME; pad.NUMBER is a Linux evdev gamepad button code.\n",
    "# 1 to 9: the tool picker's pens in its order, left to right and down.\n",
    "key.1=tool:pencil\nkey.2=tool:mech\nkey.3=tool:charcoal\nkey.4=tool:ballpoint\n",
    "key.5=tool:fineliner\nkey.6=tool:marker\nkey.7=tool:fountain\nkey.8=tool:calligraphy\n",
    "key.9=tool:highlighter\nkey.e=tool:eraser\nkey.w=white\nkey.Ctrl+z=undo\nkey.Ctrl+y=redo\n",
    "key.Left=previous-page\nkey.Right=next-page\nkey.PageUp=previous-tool\nkey.PageDown=next-tool\n",
    "# Generic evdev gamepad defaults; controllers may report different codes.\n",
    "pad.304=undo\npad.305=tool:eraser\npad.307=redo\npad.308=white\n",
    "pad.310=previous-tool\npad.311=next-tool\npad.314=previous-page\npad.315=next-page\n",
    "# Digital D-pad buttons and standard HAT0 directions share these bindings.\n",
    dpad_lines!(),
);

/// isspace() in the C locale: space, \t, \n, \v, \f, \r (u8::is_ascii_whitespace leaves out \v).
fn isspace(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}

/// strtol(s, &end, 10): the value (saturating) and where it ended; 0 consumed when there are no
/// digits (the C's end is then s itself).
fn strtol10(s: &[u8]) -> (i64, usize) {
    let mut i = 0;
    while i < s.len() && isspace(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let digits = i;
    let mut v = 0i64;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == digits {
        return (0, 0);
    }
    (if neg { -v } else { v }, i)
}

pub fn shortcut_keycode(name: &str) -> i32 {
    // Linux input-event-codes.h, not SDL's unrelated USB scan code numbering.
    const KEYS: [(&str, i32); 54] = [
        ("1", 2), ("2", 3), ("3", 4), ("4", 5), ("5", 6), ("6", 7), ("7", 8), ("8", 9), ("9", 10), ("0", 11),
        ("q", 16), ("w", 17), ("e", 18), ("r", 19), ("t", 20), ("y", 21), ("u", 22), ("i", 23), ("o", 24), ("p", 25),
        ("a", 30), ("s", 31), ("d", 32), ("f", 33), ("g", 34), ("h", 35), ("j", 36), ("k", 37), ("l", 38),
        ("z", 44), ("x", 45), ("c", 46), ("v", 47), ("b", 48), ("n", 49), ("m", 50),
        ("Left", 105), ("Right", 106), ("Up", 103), ("Down", 108), ("PageUp", 104), ("PageDown", 109),
        ("Space", 57), ("Enter", 28), ("Tab", 15), ("Escape", 1),
        ("VolumeUp", 115), ("VolumeDown", 114), ("PlayPause", 164), ("NextSong", 163), ("PreviousSong", 165),
        ("Home", 102), ("End", 107), ("Back", 158),
    ];
    for (key, code) in KEYS {
        if name.eq_ignore_ascii_case(key) {
            return code;
        }
    }
    let b = name.as_bytes();
    if b.len() >= 4 && b[..4].eq_ignore_ascii_case(b"code") {
        let (n, end) = strtol10(&b[4..]);
        if end != 0 && end == b.len() - 4 && n > 0 && n <= 767 {
            return n as i32;
        }
    }
    -1
}

const K_TOOLS: [&str; TOOL_COUNT] =
    ["pencil", "mech", "ballpoint", "eraser", "highlighter", "fountain", "fineliner", "marker", "charcoal", "calligraphy"];
// (K_TOOLS with "tool:" before each: shortcut_action_name hands out &'static str, where the C
// formats into a static buffer.)
const K_TOOL_ACTIONS: [&str; TOOL_COUNT] = [
    "tool:pencil",
    "tool:mech",
    "tool:ballpoint",
    "tool:eraser",
    "tool:highlighter",
    "tool:fountain",
    "tool:fineliner",
    "tool:marker",
    "tool:charcoal",
    "tool:calligraphy",
];
const K_ACTIONS: [(&str, i32); 28] = [
    ("undo", SHORTCUT_UNDO), ("redo", SHORTCUT_REDO), ("previous-page", SHORTCUT_PREV_PAGE),
    ("next-page", SHORTCUT_NEXT_PAGE), ("previous-tool", SHORTCUT_PREV_TOOL), ("next-tool", SHORTCUT_NEXT_TOOL),
    ("white", SHORTCUT_WHITE), ("index", SHORTCUT_INDEX), ("hold-eraser", SHORTCUT_HOLD_ERASER),
    ("eraser-mode", SHORTCUT_ERASER_MODE), ("clear-page", SHORTCUT_CLEAR_PAGE), ("first-page", SHORTCUT_FIRST_PAGE),
    ("last-page", SHORTCUT_LAST_PAGE), ("new-page", SHORTCUT_NEW_PAGE), ("fit-page", SHORTCUT_FIT_PAGE),
    ("pages", SHORTCUT_PAGES), ("tool-picker", SHORTCUT_TOOL_PICKER), ("paper", SHORTCUT_PAPER),
    ("display", SHORTCUT_DISPLAY), ("pen-feel", SHORTCUT_PEN_FEEL), ("fold-panel", SHORTCUT_FOLD_PANEL),
    ("library", SHORTCUT_LIBRARY), ("close-menu", SHORTCUT_CLOSE_MENU), ("picker-previous", SHORTCUT_PICKER_PREV),
    ("picker-next", SHORTCUT_PICKER_NEXT), ("add-link", SHORTCUT_ADD_LINK), ("name-page", SHORTCUT_NAME_PAGE),
    ("status-toggle", SHORTCUT_STATUS),
];
/// Gamepad buttons by name: the Linux gamepad convention's, the D-pad (HAT0 decodes to these), and
/// the analog triggers (hidkeys_trigger_event reports them as these).
const K_BUTTONS: [(&str, i32); 16] = [
    ("Up", 544), ("Down", 545), ("Left", 546), ("Right", 547),
    ("South", 304), ("East", 305), ("North", 307), ("West", 308),
    ("L1", 310), ("R1", 311), ("L2", 312), ("R2", 313), ("Select", 314), ("Start", 315),
    ("LT", HIDKEYS_LT), ("RT", HIDKEYS_RT),
];

pub fn shortcut_action(name: &str) -> i32 {
    if let Some(rest) = name.strip_prefix("tool:") {
        for (i, tool) in K_TOOLS.iter().enumerate() {
            if rest == *tool {
                return SHORTCUT_TOOL_BASE + i as i32;
            }
        }
        return SHORTCUT_NONE;
    }
    for (action_name, action) in K_ACTIONS {
        if name == action_name {
            return action;
        }
    }
    SHORTCUT_NONE
}
fn binding(m: &mut Shortcuts, key: &[u8], value: &[u8]) -> bool {
    // (Text from a file is bytes. A key or value that is not UTF-8 matches no name, number or
    // modifier, so the C refuses its line once, as here.)
    let (Ok(key), Ok(value)) = (std::str::from_utf8(key), std::str::from_utf8(value)) else {
        m.rejected += 1;
        return false;
    };
    let mut b = ShortcutBinding { code: -1, action: shortcut_action(value), ..Default::default() };
    if let Some(rest) = key.strip_prefix("pad.") {
        let (n, end) = strtol10(rest.as_bytes());
        if end != 0 && end == rest.len() && (256..=767).contains(&n) {
            b.code = n as i32;
        }
        for (button, code) in K_BUTTONS {
            if rest.eq_ignore_ascii_case(button) {
                b.code = code;
            }
        }
        b.gamepad = true;
    } else if let Some(mut p) = key.strip_prefix("key.") {
        while let Some(plus) = p.find('+') {
            let part = &p[..plus];
            if part.eq_ignore_ascii_case("Ctrl") {
                b.mods |= KEYMOD_CTRL;
            } else if part.eq_ignore_ascii_case("Shift") {
                b.mods |= KEYMOD_SHIFT;
            } else if part.eq_ignore_ascii_case("Alt") {
                b.mods |= KEYMOD_ALT;
            } else if part.eq_ignore_ascii_case("Meta") {
                b.mods |= KEYMOD_META;
            } else {
                m.rejected += 1;
                return false;
            }
            p = &p[plus + 1..];
        }
        b.code = shortcut_keycode(p);
    }
    if b.code < 0 || b.action == 0 {
        m.rejected += 1;
        return false;
    }
    for i in 0..m.count as usize {
        let old = &mut m.bindings[i];
        if old.code == b.code && old.mods == b.mods && old.gamepad == b.gamepad {
            *old = b;
            return true;
        }
    }
    if m.count == SHORTCUT_MAX as i32 {
        m.rejected += 1;
        return false;
    }
    m.bindings[m.count as usize] = b;
    m.count += 1;
    true
}
fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut n = s.len();
    while a < n && isspace(s[a]) {
        a += 1;
    }
    while n > a && isspace(s[n - 1]) {
        n -= 1;
    }
    &s[a..n]
}
fn parse(map: &mut Shortcuts, text: &[u8]) {
    // (strtok_r with "\n": the empty lines between two newlines are skipped.)
    for line in text.split(|&c| c == b'\n').filter(|l| !l.is_empty()) {
        let line = trim(line);
        if line.is_empty() || line[0] == b'#' || line[0] == b';' {
            continue;
        }
        let Some(eq) = line.iter().position(|&c| c == b'=') else {
            map.rejected += 1;
            continue;
        };
        binding(map, trim(&line[..eq]), trim(&line[eq + 1..]));
    }
}
/// A small file of lines read whole (shortcuts.ini, controller.ini, gestures.ini): None when it is
/// not there; Err when it is not a file, is over cap bytes, or cannot be read; else its text up to
/// its first NUL.
pub(crate) fn ini_read_bounded(path: &str, cap: u64) -> Result<Option<Vec<u8>>, ()> {
    if !access_ok(path) {
        return Ok(None);
    }
    match std::fs::metadata(path) {
        Ok(st) if st.is_file() && st.len() <= cap => {}
        _ => return Err(()),
    }
    // Bound the read too, including a file changed since stat.
    let Ok(f) = std::fs::File::open(path) else { return Err(()) };
    let mut text = Vec::new();
    let failed = std::io::Read::read_to_end(&mut std::io::Read::take(f, cap + 1), &mut text).is_err() || text.len() as u64 > cap;
    if failed {
        return Err(());
    }
    // (the C keeps the text as a string: it ends at a NUL)
    text.truncate(text.iter().position(|&c| c == 0).unwrap_or(text.len()));
    Ok(Some(text))
}

/// Missing file: defaults. Existing empty file: no bindings. Never rewrites user mappings.
pub fn shortcuts_load(map: &mut Shortcuts, path: &str) {
    *map = Shortcuts::default();
    let text = match ini_read_bounded(path, 65536) {
        Ok(Some(text)) => text,
        Ok(None) => DEFAULTS.as_bytes().to_vec(),
        Err(()) => {
            map.rejected += 1;
            return;
        }
    };
    parse(map, &text);
}

/// access(path, F_OK) == 0
fn access_ok(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}
pub fn shortcuts_lookup(map: &Shortcuts, gamepad: bool, code: i32, mods: i32) -> i32 {
    for i in 0..map.count as usize {
        let b = &map.bindings[i];
        if b.gamepad == gamepad && b.code == code && b.mods == mods {
            return b.action;
        }
    }
    SHORTCUT_NONE
}
/// creates only if absent
pub fn shortcuts_write_defaults(path: &str) -> i32 {
    if access_ok(path) {
        return 0;
    }
    write_file_atomic(path, DEFAULTS.as_bytes())
}

/// No version before 2.0.0-beta.5 read a D-pad or a hat, so a shortcuts.ini written then binds none of
/// its directions: such a file gets the D-pad's starter lines too, in memory (as many as there is room
/// for, none counted as refused). An empty file keeps no mappings.
fn add_dpad(map: &mut Shortcuts) {
    for i in 0..map.count as usize {
        if map.bindings[i].gamepad && map.bindings[i].code >= 544 && map.bindings[i].code <= 547 {
            return;
        }
    }
    let mut dpad = Shortcuts::default();
    parse(&mut dpad, DPAD_LINES.as_bytes());
    let mut i = 0;
    while i < dpad.count && map.count < SHORTCUT_MAX as i32 {
        map.bindings[map.count as usize] = dpad.bindings[i as usize];
        map.count += 1;
        i += 1;
    }
}
/// controller.ini (Settings > Controller > Buttons): the gamepad buttons learned there. While it is
/// there they come from it alone (shortcuts.ini's pad.* lines are set aside); key.* lines are
/// always shortcuts.ini's. Grimoire writes it whole, atomically; a line it cannot read is rejected
/// and counted, as in shortcuts.ini. A shortcuts.ini that binds no D-pad direction (one written
/// before 2.0.0-beta.5) gets the D-pad's starter lines in memory; the file is not changed.
pub fn shortcuts_load_all(map: &mut Shortcuts, shortcuts_path: &str, controller_path: &str) {
    shortcuts_load(map, shortcuts_path);
    if map.count != 0 {
        add_dpad(map);
    }
    if !access_ok(controller_path) {
        return;
    }
    let mut pads = Shortcuts::default();
    shortcuts_load(&mut pads, controller_path);
    let mut kept = 0usize;
    for i in 0..map.count as usize {
        if !map.bindings[i].gamepad {
            map.bindings[kept] = map.bindings[i];
            kept += 1;
        }
    }
    map.count = kept as i32;
    map.rejected += pads.rejected;
    for i in 0..pads.count as usize {
        if !pads.bindings[i].gamepad || map.count == SHORTCUT_MAX as i32 {
            map.rejected += 1; // (a key there, or no room)
            continue;
        }
        map.bindings[map.count as usize] = pads.bindings[i];
        map.count += 1;
    }
}

/// controller.ini with every gamepad binding of map, written whole (atomically); -1 when it could not be.
fn write_pads(next: &Shortcuts, controller_path: &str) -> i32 {
    // (the C builds it in a char[4096])
    const TEXT_CAP: usize = 4096;
    let mut text = String::from(
        "# Grimoire: controller buttons, learned in Settings > Controller > Buttons.\n\
         # While this file is here, shortcuts.ini's pad. lines are set aside.\n",
    );
    for i in 0..next.count as usize {
        let b = &next.bindings[i];
        if !b.gamepad {
            continue;
        }
        let mut pad = String::new();
        shortcut_pad_name(b.code, &mut pad, 16);
        // (an action with no name prints "(null)", as the C library's %s does)
        let line = cformat!("pad.%s=%s\n", &pad, shortcut_action_name(b.action).unwrap_or("(null)"));
        if line.len() >= TEXT_CAP - text.len() {
            return -1;
        }
        text.push_str(&line);
    }
    if write_file_atomic(controller_path, text.as_bytes()) != 0 {
        -1
    } else {
        0
    }
}
/// Gamepad button code does action from now on, as its only button: the button's old action and the
/// action's other buttons lose theirs. Writes controller.ini with every gamepad binding; -1 when it
/// could not (map unchanged).
pub fn shortcuts_learn(map: &mut Shortcuts, controller_path: &str, code: i32, action: i32) -> i32 {
    if shortcut_action_name(action).is_none() || !(256..=767).contains(&code) {
        return -1;
    }
    let mut next = map.clone();
    let mut kept = 0usize;
    for i in 0..next.count as usize {
        let b = next.bindings[i];
        if b.gamepad && (b.code == code || b.action == action) {
            continue;
        }
        next.bindings[kept] = b;
        kept += 1;
    }
    next.count = kept as i32;
    if next.count == SHORTCUT_MAX as i32 {
        return -1;
    }
    next.bindings[next.count as usize] = ShortcutBinding { code, action, gamepad: true, ..Default::default() };
    next.count += 1;
    if write_pads(&next, controller_path) != 0 {
        return -1;
    }
    *map = next;
    0
}
/// The gamepad buttons for action, and for action2 (0: none), do nothing from now on (a Buttons sheet
/// row held): controller.ini is written with every other gamepad binding, as shortcuts_learn writes
/// it, so shortcuts.ini's pad. lines (and the D-pad's starter lines) stay set aside. Nothing is
/// written when neither had a button. -1 when it could not be written (map unchanged).
pub fn shortcuts_forget(map: &mut Shortcuts, controller_path: &str, action: i32, action2: i32) -> i32 {
    let mut next = map.clone();
    let mut kept = 0usize;
    for i in 0..next.count as usize {
        let b = next.bindings[i];
        if b.gamepad && (b.action == action || (action2 != 0 && b.action == action2)) {
            continue;
        }
        next.bindings[kept] = b;
        kept += 1;
    }
    if kept as i32 == next.count {
        return 0; // (neither had a button: nothing to write)
    }
    next.count = kept as i32;
    if write_pads(&next, controller_path) != 0 {
        return -1;
    }
    *map = next;
    0
}
/// The gamepad buttons for action: how many there are, the first cap of them in codes.
pub fn shortcuts_pads_for(map: &Shortcuts, action: i32, codes: &mut [i32]) -> i32 {
    let mut n = 0i32;
    for i in 0..map.count as usize {
        let b = &map.bindings[i];
        if !b.gamepad || b.action != action {
            continue;
        }
        if (n as usize) < codes.len() {
            codes[n as usize] = b.code;
        }
        n += 1;
    }
    n
}
/// A gamepad button as the files name it: "South", "Up", or its number ("316").
pub fn shortcut_pad_name(code: i32, out: &mut String, cap: usize) {
    for (name, button) in K_BUTTONS {
        if button == code {
            csnprintf!(out, cap, "%s", name);
            return;
        }
    }
    csnprintf!(out, cap, "%d", code);
}
/// A gamepad button as the controller labels it, for the Buttons sheet: a Switch-mode one's (B, A, Y,
/// X, L, R, ZL, ZR, -, +, LS, RS, Home, Capture: hidkeys_switch), else as the files name it.
pub fn shortcut_pad_label(code: i32, switch_layout: bool, out: &mut String, cap: usize) {
    // The Pro Controller's simple report, in its order from BTN_A (hidkeys_switch).
    const K_SWITCH: [&str; 14] = ["B", "A", "Y", "X", "L", "R", "ZL", "ZR", "-", "+", "LS", "RS", "Home", "Capture"];
    if switch_layout && code >= 304 && code < 304 + K_SWITCH.len() as i32 {
        csnprintf!(out, cap, "%s", K_SWITCH[(code - 304) as usize]);
    } else {
        shortcut_pad_name(code, out, cap);
    }
}
/// An action's name in the files ("undo", "tool:eraser"), or None.
pub fn shortcut_action_name(action: i32) -> Option<&'static str> {
    if action >= SHORTCUT_TOOL_BASE && action < SHORTCUT_TOOL_BASE + TOOL_COUNT as i32 {
        return Some(K_TOOL_ACTIONS[(action - SHORTCUT_TOOL_BASE) as usize]);
    }
    for (name, a) in K_ACTIONS {
        if a == action {
            return Some(name);
        }
    }
    None
}
