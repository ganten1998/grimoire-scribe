//! Optional keyboard/gamepad mappings. Linux evdev key numbers are stable across Kindle models.

use crate::prelude::*;

/// 2.0.0-beta.2: one key may have one action in each place, so a file keeps every line (192 before).
pub const SHORTCUT_MAX: usize = 256;
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
/// The clock and battery corner on or off (status.rs; not in the C tree): it acts on every screen, and shows
/// on a notebook page.
pub const SHORTCUT_STATUS: i32 = 47;
/// The Index's actions (2.0.0-beta.5, Rust only): the Links bar (two fingers up open it, gesturemap.rs's
/// built-in), the Find sheet (Go to page or name), back to the page before, the dotted link areas, and
/// the whole page sideways. Menus open and close with their own button; the others act at once.
pub const SHORTCUT_LINKS: i32 = 48;
pub const SHORTCUT_GO_TO_PAGE: i32 = 49;
pub const SHORTCUT_GO_BACK: i32 = 50;
pub const SHORTCUT_LINK_AREAS: i32 = 51;
pub const SHORTCUT_LANDSCAPE_FIT: i32 = 52;
/// The clock and battery corner's place, the next of the page's four corners (status.rs; Rust only).
pub const SHORTCUT_STATUS_CORNER: i32 = 53;
/// Two fingers left on the last page add a page, or do not (settings.ini gesture_new_page; Rust only).
pub const SHORTCUT_GESTURE_NEW_PAGE: i32 = 54;
pub const SHORTCUT_STATUS_MODE: i32 = 55;
/// The highest action id in use (it follows the highest; ids between with no action have no name, which the
/// loops over 1..=SHORTCUT_LAST skip).
pub const SHORTCUT_LAST: i32 = SHORTCUT_PAN_DOWN;
/// Where an action came from: a key, a controller's button, a touch gesture (gestures.ini), and
/// later a finger shape or the pen's button. A page turned by a gesture adds a page on the last one unless settings.ini gesture_new_page is off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionSrc {
    #[default]
    Key,
    Pad,
    Touch,
    Shape,
    Pen,
}
impl ActionSrc {
    pub const fn mask(self) -> u8 {
        match self {
            ActionSrc::Key => SRC_KEY,
            ActionSrc::Pad => SRC_PAD,
            ActionSrc::Touch => SRC_TOUCH,
            ActionSrc::Shape => SRC_SHAPE,
            ActionSrc::Pen => SRC_PEN,
        }
    }
}
impl From<bool> for ActionSrc {
    fn from(gamepad: bool) -> Self {
        if gamepad {
            ActionSrc::Pad
        } else {
            ActionSrc::Key
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub code: i32,
    pub mods: i32,
    pub action: i32,
    pub src: ActionSrc,
}
#[derive(Clone, Debug)]
pub struct Shortcuts {
    pub bindings: [ShortcutBinding; SHORTCUT_MAX],
    pub count: i32,
    pub rejected: i32,
}
impl Default for Shortcuts {
    fn default() -> Self {
        Shortcuts { bindings: [ShortcutBinding::default(); SHORTCUT_MAX], count: 0, rejected: 0 ,
        }
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

// The keyboard scheme of 2.0.0-beta.2, for a keyboard paired with the Kindle: in pieces, as concat! reads only
// literals.
macro_rules! scheme_everywhere {
    () => {
        "# Everywhere\nkey.Escape=back\nkey.Ctrl+s=save\n"
    };
}
macro_rules! scheme_library_and_page {
    () => {
        concat!(
            "# The Library\n",
            "key.Ctrl+n=new-notebook\nkey.Ctrl+f=new-folder\nkey.s=share\nkey.o=settings\n",
            "# A notebook page. 1 to 9: the tool picker's pens in its order, left to right and down; 0 and E: the eraser.\n",
            "key.1=tool:pencil\nkey.2=tool:mech\nkey.3=tool:charcoal\nkey.4=tool:ballpoint\n",
            "key.5=tool:fineliner\nkey.6=tool:marker\nkey.7=tool:fountain\nkey.8=tool:calligraphy\n",
            "key.9=tool:highlighter\nkey.0=tool:eraser\nkey.e=tool:eraser\nkey.w=white\n",
            "key.Ctrl+z=undo\nkey.Ctrl+Shift+z=redo\nkey.Ctrl+y=redo\n",
            "key.Left=previous-page\nkey.Right=next-page\nkey.PageUp=previous-page\nkey.PageDown=next-page\n",
            "key.Home=first-page\nkey.End=last-page\n",
            "# [ and ]: the previous and the next tool.\n",
            "key.code26=previous-tool\nkey.code27=next-tool\n",
            "key.Ctrl+n=new-page\nkey.Ctrl+f=go-to-page\nkey.Ctrl+l=add-link\nkey.r=name-page\nkey.Space=two-finger-up\n",
            "key.g=index-map\nkey.l=links\nkey.p=pages\nkey.t=tool-picker\nkey.s=select\nkey.x=hold-eraser\n",
        )
    };
}
macro_rules! scheme_zoom_pages_graph {
    () => {
        concat!(
            "# - and = (+), Shift+=, the keypad's - and +, and with Ctrl: zoom out and in; Ctrl+0 the whole page.\n",
            "key.code12=zoom-out\nkey.code13=zoom-in\nkey.Shift+code13=zoom-in\nkey.code74=zoom-out\nkey.code78=zoom-in\n",
            "key.Ctrl+code12=zoom-out\nkey.Ctrl+code13=zoom-in\nkey.Ctrl+0=fit-page\n",
            "# The Pages view and the Graph\n",
            "key.Ctrl+a=select-all\n",
            "# The Graph: the arrows move it; V its view.\n",
            "key.Left=pan-left\nkey.Right=pan-right\nkey.Up=pan-up\nkey.Down=pan-down\nkey.v=graph-view\n",
        )
    };
}
// The starter map below its first line, which names the menu (a macro, as concat! reads only literals).
macro_rules! starter_body {
    () => {
        concat!(
            "# key.NAME or key.Ctrl+NAME=ACTION (an Apple keyboard's Cmd is Ctrl here); codeN is a Linux evdev key code;\n",
            "# pad.NUMBER is a Linux evdev gamepad button code.\n",
            "# A key may have one action in each place (the Library, a notebook page, the Pages view, the Graph,\n",
            "# Pen feel): each action acts in its own places (docs/SHORTCUTS.md).\n",
            scheme_everywhere!(),
            scheme_library_and_page!(),
            scheme_zoom_pages_graph!(),
            "# Generic evdev gamepad defaults; controllers may report different codes.\n",
            "pad.304=undo\npad.305=tool:eraser\npad.307=redo\npad.308=white\n",
            "pad.310=previous-tool\npad.311=next-tool\npad.314=previous-page\npad.315=next-page\n",
            "# Digital D-pad buttons and standard HAT0 directions share these bindings.\n",
            dpad_lines!(),
        )
    };
}
/// The Kindle's starter shortcuts.ini (written when shortcuts are first enabled, and read in memory with no
/// file): the scheme for a paired keyboard, and the controller's lines. Its first line names the menu as
/// 2.0.0-beta.2 has it, Settings > Shortcuts (the owner's ruling of 2026-11-04).
const DEFAULTS: &str = concat!(
    "# Grimoire shortcuts. Enable in Settings > Shortcuts; after editing, Reload shortcuts.ini there.\n",
    starter_body!(),
);

/// The starter map: what a missing shortcuts.ini reads as, and what Enable shortcuts writes.
pub fn shortcuts_starter() -> &'static str {
    DEFAULTS
}

/// Every starter shortcuts.ini an earlier Grimoire wrote, byte for byte (from the history of src/shortcuts.c
/// and rust/src/shortcuts.rs): the Kindle's of 1.1.0, of 1.2.0 (the D-pad), of 2.0.0 (Settings >
/// Controller, the picker's order) and of the 2.0.0-beta.2 builds before Settings > Shortcuts (the keyboard
/// scheme, its first line naming "Settings > Controller and keys"). Such a file nobody edited is replaced by
/// this build's starter map at the next start (shortcuts_upgrade_starter); any other is kept.
const OLD_STARTERS: [&str; 4] = [
    concat!(
        "# Grimoire shortcuts. Enable in Settings > Shortcuts. Reload there after editing.\n",
        "# key.NAME or key.Ctrl+NAME; pad.NUMBER is a Linux evdev gamepad button code.\n",
        "key.1=tool:pencil\nkey.2=tool:mech\nkey.3=tool:ballpoint\nkey.4=tool:fountain\n",
        "key.5=tool:fineliner\nkey.6=tool:marker\nkey.7=tool:charcoal\nkey.8=tool:calligraphy\n",
        "key.9=tool:highlighter\nkey.e=tool:eraser\nkey.w=white\nkey.Ctrl+z=undo\nkey.Ctrl+y=redo\n",
        "key.Left=previous-page\nkey.Right=next-page\nkey.PageUp=previous-tool\nkey.PageDown=next-tool\n",
        "# Generic evdev gamepad defaults; controllers may report different codes.\n",
        "pad.304=undo\npad.305=tool:eraser\npad.307=redo\npad.308=white\n",
        "pad.310=previous-tool\npad.311=next-tool\npad.314=previous-page\npad.315=next-page\n",
    ),
    concat!(
        "# Grimoire shortcuts. Enable in Settings > Shortcuts. Reload there after editing.\n",
        "# key.NAME or key.Ctrl+NAME; pad.NUMBER is a Linux evdev gamepad button code.\n",
        "key.1=tool:pencil\nkey.2=tool:mech\nkey.3=tool:ballpoint\nkey.4=tool:fountain\n",
        "key.5=tool:fineliner\nkey.6=tool:marker\nkey.7=tool:charcoal\nkey.8=tool:calligraphy\n",
        "key.9=tool:highlighter\nkey.e=tool:eraser\nkey.w=white\nkey.Ctrl+z=undo\nkey.Ctrl+y=redo\n",
        "key.Left=previous-page\nkey.Right=next-page\nkey.PageUp=previous-tool\nkey.PageDown=next-tool\n",
        "# Generic evdev gamepad defaults; controllers may report different codes.\n",
        "pad.304=undo\npad.305=tool:eraser\npad.307=redo\npad.308=white\n",
        "pad.310=previous-tool\npad.311=next-tool\npad.314=previous-page\npad.315=next-page\n",
        "# Digital D-pad buttons and standard HAT0 directions share these bindings.\n",
        dpad_lines!(),
    ),
    OLD_KINDLE_STARTER_2_0,
    concat!(
        "# Grimoire shortcuts. Enable in Settings > Controller and keys; after editing, Reload shortcuts.ini there.\n",
        starter_body!(),
    ),
];
/// The Kindle's starter map from 2.0.0-beta.1 until this build (live-keys.ini's earlier starter came from it).
pub(crate) const OLD_KINDLE_STARTER_2_0: &str = concat!(
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

/// Whether text is a starter map some Grimoire wrote (byte for byte; a copy with CRLF line ends counts),
/// earlier ones and this build's.
pub fn shortcuts_is_starter(text: &[u8]) -> bool {
    let lf: Vec<u8>;
    let text = if text.contains(&b'\r') {
        lf = String::from_utf8_lossy(text).replace("\r\n", "\n").into_bytes();
        if lf.contains(&b'\r') {
            return false;
        }
        &lf[..]
    } else {
        text
    };
    OLD_STARTERS.iter().chain([DEFAULTS].iter()).any(|t| t.as_bytes() == text)
}

/// At the start: a shortcuts.ini that is byte for byte a starter map
/// some Grimoire wrote, and not this build's, is replaced by this build's starter map, atomically. True when
/// it was (the caller says so once: a log line and a toast). A file someone edited, a missing one, or one that
/// cannot be read is left as it is.
pub fn shortcuts_upgrade_starter(path: &str) -> bool {
    let Ok(Some(text)) = ini_read_bounded(path, 65536) else { return false };
    if text == shortcuts_starter().as_bytes() || !shortcuts_is_starter(&text) {
        return false;
    }
    write_file_atomic(path, shortcuts_starter().as_bytes()) == 0
}

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

// Linux input-event-codes.h, not SDL's unrelated USB scan code numbering.
const KEY_NAMES: [(&str, i32); 54] = [
    ("1", 2), ("2", 3), ("3", 4), ("4", 5), ("5", 6), ("6", 7), ("7", 8), ("8", 9), ("9", 10), ("0", 11),
    ("q", 16), ("w", 17), ("e", 18), ("r", 19), ("t", 20), ("y", 21), ("u", 22), ("i", 23), ("o", 24), ("p", 25),
    ("a", 30), ("s", 31), ("d", 32), ("f", 33), ("g", 34), ("h", 35), ("j", 36), ("k", 37), ("l", 38),
    ("z", 44), ("x", 45), ("c", 46), ("v", 47), ("b", 48), ("n", 49), ("m", 50),
    ("Left", 105), ("Right", 106), ("Up", 103), ("Down", 108), ("PageUp", 104), ("PageDown", 109),
    ("Space", 57), ("Enter", 28), ("Tab", 15), ("Escape", 1),
    ("VolumeUp", 115), ("VolumeDown", 114), ("PlayPause", 164), ("NextSong", 163), ("PreviousSong", 165),
    ("Home", 102), ("End", 107), ("Back", 158),
];

pub fn shortcut_keycode(name: &str) -> i32 {
    for (key, code) in KEY_NAMES {
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

/// Gamepad buttons by name: the Linux gamepad convention's, the D-pad (HAT0 decodes to these), and
/// the analog triggers (hidkeys_trigger_event reports them as these).
const K_BUTTONS: [(&str, i32); 16] = [
    ("Up", 544), ("Down", 545), ("Left", 546), ("Right", 547),
    ("South", 304), ("East", 305), ("North", 307), ("West", 308),
    ("L1", 310), ("R1", 311), ("L2", 312), ("R2", 313), ("Select", 314), ("Start", 315),
    ("LT", HIDKEYS_LT), ("RT", HIDKEYS_RT),
];

pub fn shortcut_action(name: &str) -> i32 {
    action_by_name(name) }
fn binding(m: &mut Shortcuts, key: &[u8], value: &[u8], allow_key: bool, allow_pad: bool) -> bool {
    // (Text from a file is bytes. A key or value that is not UTF-8 matches no name, number or
    // modifier, so the C refuses its line once, as here.)
    let (Ok(key), Ok(value)) = (std::str::from_utf8(key), std::str::from_utf8(value)) else {
        m.rejected += 1;
        return false;
    };
    let mut b = ShortcutBinding { code: -1, action: shortcut_action(value), ..Default::default() };
    if let Some(rest) = key.strip_prefix("pad.") {
        b.src = ActionSrc::Pad;
        if !allow_pad {
            m.rejected += 1;
            return false;
        }
        let (n, end) = strtol10(rest.as_bytes());
        if end != 0 && end == rest.len() && (256..=767).contains(&n) {
            b.code = n as i32;
        }
        for (button, code) in K_BUTTONS {
            if rest.eq_ignore_ascii_case(button) {
                b.code = code;
            }
        }
        } else if let Some(mut p) = key.strip_prefix("key.") {
        b.src = ActionSrc::Key;
        if !allow_key {
            m.rejected += 1;
            return false;
        }
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
    if b.code < 0 || b.action == 0 || action_sources(b.action) & b.src.mask() == 0 {
        m.rejected += 1;
        return false;
    }
    // Every line is kept (one key may have an action in each place: shortcuts_lookup_at); the same key with
    // the same action again moves to the end, so a later line still decides, as before.
    if let Some(i) = (0..m.count as usize).find(|&i| {
        let o = &m.bindings[i];
        o.code == b.code && o.mods == b.mods && o.src == b.src && o.action == b.action
    }) {
        m.bindings.copy_within(i + 1..m.count as usize, i);
        m.bindings[m.count as usize - 1] = b;
        return true;
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
fn parse(map: &mut Shortcuts, text: &[u8], allow_key: bool, allow_pad: bool) {
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
        binding(map, trim(&line[..eq]), trim(&line[eq + 1..]),
            allow_key,
            allow_pad,
        );
    }
}
/// A small file of lines read whole (shortcuts.ini, controller.ini, gestures.ini): None when it is
/// not there; Err when it is not a file, is over cap bytes, or cannot be read; else its text up to
/// its first NUL.
pub(crate) fn ini_read_bounded(path: &str, cap: u64) -> Result<Option<Vec<u8>>, ()> {
    match std::fs::metadata(path) {
        Ok(st) if st.is_file() && st.len() <= cap => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // A dangling symlink is an existing unreadable mapping, not a missing file.
            return match std::fs::symlink_metadata(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(()),
            };
        }
        _ => return Err(()),
    }
    // Bound the read too, including a file changed since stat.
    let Ok(f) = std::fs::File::open(path) else { return Err(()) ;
    };
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
    shortcuts_load_sources(map, path, true, true);
}

fn shortcuts_load_sources(map: &mut Shortcuts, path: &str, allow_key: bool, allow_pad: bool) {
    *map = Shortcuts::default();
    let text = match ini_read_bounded(path, 65536) {
        Ok(Some(text)) => text,
        Ok(None) => DEFAULTS.as_bytes().to_vec(),
        Err(()) => {
            map.rejected += 1;
            return;
        }
    };
    parse(map, &text, allow_key, allow_pad);
}

/// A starter map's text read as shortcuts.ini is (live-keys.ini's starter is made from it).
pub(crate) fn shortcuts_parse_text(text: &str) -> Shortcuts {
    let mut m = Shortcuts::default();
    parse(&mut m, text.as_bytes(), true, true);
    m
}

/// access(path, F_OK) == 0
fn access_ok(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}
/// The action of a key (or a controller's button) in any place: the last line for it whose action acts
/// somewhere on this build; SHORTCUT_NONE for none.
pub fn shortcuts_lookup(map: &Shortcuts, src: impl Into<ActionSrc>, code: i32, mods: i32) -> i32 {
    shortcuts_lookup_at(map, src, code, mods, PL_ALL)
}

/// The action of a key (or a controller's button) pressed in place (app_place's PL_* bit): the last line for
/// it whose action acts there; SHORTCUT_NONE for none. An Apple keyboard's Cmd reports as Meta: a key with Meta
/// held and no line for it is looked up again with Ctrl in Meta's place, so Cmd+Z is Undo too.
pub fn shortcuts_lookup_at(map: &Shortcuts, src: impl Into<ActionSrc>, code: i32, mods: i32, place: u8) -> i32 {
    let src = src.into();
    let find = |mods: i32| {
        map.bindings[..map.count as usize]
            .iter()
            .rev()
            .find(|b| b.src == src && b.code == code && b.mods == mods && action_places(b.action) & place != 0)
            .map_or(SHORTCUT_NONE, |b| b.action)
    };
    let action = find(mods);
    if action == SHORTCUT_NONE && src == ActionSrc::Key && mods & KEYMOD_META != 0 {
        return find((mods & !KEYMOD_META) | KEYMOD_CTRL);
    }
    action
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
        if map.bindings[i].src == ActionSrc::Pad
            && map.bindings[i].code >= 544 && map.bindings[i].code <= 547 {
            return;
        }
    }
    let mut dpad = Shortcuts::default();
    parse(&mut dpad, DPAD_LINES.as_bytes(), false, true);
    let mut i = 0;
    while i < dpad.count && map.count < SHORTCUT_MAX as i32 {
        map.bindings[map.count as usize] = dpad.bindings[i as usize];
        map.count += 1;
        i += 1;
    }
}
/// The Graph's starter keys: Left, Right, Up and Down pan, V switches its view.
const GRAPH_KEYS: [(i32, i32); 5] = [
    (105, SHORTCUT_PAN_LEFT), (106, SHORTCUT_PAN_RIGHT), (103, SHORTCUT_PAN_UP), (108, SHORTCUT_PAN_DOWN), (47, SHORTCUT_GRAPH_VIEW),
];
/// The key's action in the Graph: the last line for it whose action acts there
/// (shortcuts_lookup_at with PL_GRAPH). The Graph's actions (graph-view and the four pans) came after every
/// shortcuts.ini written before 2.0.0-beta.2: a starter map nobody edited becomes this build's at the start
/// (shortcuts_upgrade_starter), and an edited one that binds none of the five, and is not empty, has GRAPH_KEYS
/// there too for a key with no line in the Graph, as an old file has the D-pad's lines (add_dpad): its own lines
/// keep their meaning everywhere, nothing is added to the map or the file, and a file that binds any of the five
/// is taken as written.
pub fn shortcuts_lookup_graph(map: &Shortcuts, src: impl Into<ActionSrc>, code: i32, mods: i32) -> i32 {
    let src = src.into();
    let action = shortcuts_lookup_at(map, src, code, mods, PL_GRAPH);
    let graph = |id: i32| (SHORTCUT_GRAPH_VIEW..=SHORTCUT_PAN_DOWN).contains(&id);
    if action != SHORTCUT_NONE || src != ActionSrc::Key || mods != 0 || map.count == 0
        || map.bindings[..map.count as usize].iter().any(|b| graph(b.action)) {
        return action;
    }
    GRAPH_KEYS.iter().find(|k| k.0 == code).map_or(SHORTCUT_NONE, |k| k.1)
}
/// controller.ini (Settings > Shortcuts > Buttons): the gamepad buttons learned there. While it is
/// there they come from it alone (shortcuts.ini's pad.* lines are set aside); key.* lines are
/// always shortcuts.ini's. Grimoire writes it whole, atomically; a line it cannot read is rejected
/// and counted, as in shortcuts.ini. A shortcuts.ini that binds no D-pad direction (one written
/// before 2.0.0-beta.5) gets the D-pad's starter lines in memory; the file is not changed.
pub fn shortcuts_load_all(map: &mut Shortcuts, shortcuts_path: &str, controller_path: &str) {
    shortcuts_load_sources(map, shortcuts_path, true, true);
    if map.count != 0 {
        add_dpad(map);
    }
    if !access_ok(controller_path) {
        return;
    }
    let mut pads = Shortcuts::default();
    shortcuts_load_sources(&mut pads, controller_path, false, true);
    let mut kept = 0usize;
    for i in 0..map.count as usize {
        if map.bindings[i].src != ActionSrc::Pad {
            map.bindings[kept] = map.bindings[i];
            kept += 1;
        }
    }
    map.count = kept as i32;
    map.rejected += pads.rejected;
    for i in 0..pads.count as usize {
        if pads.bindings[i].src != ActionSrc::Pad || map.count == SHORTCUT_MAX as i32 {
            map.rejected += 1; // (a key there, or no room)
            continue;
        }
        map.bindings[map.count as usize] = pads.bindings[i];
        map.count += 1;
    }
}

/// controller.ini (or a device profile's <slug>.controller.ini, shortcut_profiles.rs) with every gamepad
/// binding of map, written whole (atomically); -1 when it could not be.
fn write_pads(next: &Shortcuts, controller_path: &str) -> i32 {
    const HEADER: &str = "# Grimoire: controller buttons, learned in Settings > Shortcuts > Buttons.\n\
         # While this file is here, the pad. lines of the shortcuts file beside it are set aside.\n";
    let mut longest_name = 0usize;
    for id in (SHORTCUT_TOOL_BASE..SHORTCUT_TOOL_BASE + TOOL_COUNT as i32)
        .chain(K_ACTIONS.iter().map(|(_, id)| *id))
    {
        if let Some(name) = shortcut_action_name(id) {
            longest_name = longest_name.max(name.len());
        }
    }
    // `pad.767=` is eight bytes; each line ends in LF.
    let text_cap = HEADER.len() + SHORTCUT_MAX * (8 + longest_name + 1);
    let mut text = String::new();
    if text.try_reserve_exact(text_cap).is_err() {
        return -1;
    }
    text.push_str(HEADER);
    for i in 0..next.count as usize {
        let b = &next.bindings[i];
        if b.src != ActionSrc::Pad {
            continue;
        }
        let mut pad = String::new();
        shortcut_pad_name(b.code, &mut pad, 16);
        // (an action with no name prints "(null)", as the C library's %s does)
        let line = cformat!(
            "pad.%s=%s\n",
            &pad,
            shortcut_action_name(b.action).unwrap_or("(null)")
        );
        if line.len() > text_cap.saturating_sub(text.len()) {
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
pub fn shortcuts_learn(
    map: &mut Shortcuts,
    controller_path: &str,
    src: impl Into<ActionSrc>,
    code: i32,
    action: i32,
) -> i32 {
    let src = src.into();
    if src != ActionSrc::Pad
        || shortcut_action_name(action).is_none()
        || action_sources(action) & src.mask() == 0
        || !(256..=767).contains(&code)
    {
        return -1;
    }
    let mut next = map.clone();
    let mut kept = 0usize;
    for i in 0..next.count as usize {
        let b = next.bindings[i];
        if b.src == src && (b.code == code || b.action == action) {
            continue;
        }
        next.bindings[kept] = b;
        kept += 1;
    }
    next.count = kept as i32;
    if next.count == SHORTCUT_MAX as i32 {
        return -1;
    }
    next.bindings[next.count as usize] = ShortcutBinding {
        code,
        action,
        src,
        ..Default::default()
    };
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
pub fn shortcuts_forget(
    map: &mut Shortcuts,
    controller_path: &str,
    src: impl Into<ActionSrc>,
    action: i32,
    action2: i32,
) -> i32 {
    let src = src.into();
    if src != ActionSrc::Pad {
        return -1;
    }
    let mut next = map.clone();
    let mut kept = 0usize;
    for i in 0..next.count as usize {
        let b = next.bindings[i];
        if b.src == src && (b.action == action || (action2 != 0 && b.action == action2)) {
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
pub fn shortcuts_pads_for(
    map: &Shortcuts,
    src: impl Into<ActionSrc>,
    action: i32,
    codes: &mut [i32],
) -> i32 {
    let src = src.into();
    let mut n = 0i32;
    for i in 0..map.count as usize {
        let b = &map.bindings[i];
        if b.src != src || b.action != action {
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
    const K_SWITCH: [&str; 14] = [
        "B", "A", "Y", "X", "L", "R", "ZL", "ZR", "-", "+", "LS", "RS", "Home", "Capture",
    ];
    if switch_layout && code >= 304 && code < 304 + K_SWITCH.len() as i32 {
        csnprintf!(out, cap, "%s", K_SWITCH[(code - 304) as usize]);
    } else {
        shortcut_pad_name(code, out, cap);
    }
}
/// An action's name in the files ("undo", "tool:eraser"), or None.
pub fn shortcut_action_name(action: i32) -> Option<&'static str> {
    action_info(action ) .map(|a| a .name)}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str, text: Option<&str>) -> Shortcuts {
        let dir = std::env::temp_dir().join(format!("grimoire-shortcuts-{}-{name}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("shortcuts.ini");
        let _ = std::fs::remove_file(&path);
        if let Some(text) = text {
            std::fs::write(&path, text).unwrap();
        }
        let mut map = Shortcuts::default();
        shortcuts_load_all(&mut map, path.to_str().unwrap(), dir.join("none.ini").to_str().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
        map
    }
    const LEFT: i32 = 105;
    const UP: i32 = 103;
    const V: i32 = 47;

    // The Graph's actions are names the files can use, by keys and by a controller's buttons.
    #[test]
    fn the_graphs_actions_have_names() {
        for (name, id) in [("graph-view", SHORTCUT_GRAPH_VIEW), ("pan-left", SHORTCUT_PAN_LEFT), ("pan-right", SHORTCUT_PAN_RIGHT),
            ("pan-up", SHORTCUT_PAN_UP), ("pan-down", SHORTCUT_PAN_DOWN)] {
            assert_eq!(shortcut_action(name), id);
            assert_eq!(shortcut_action_name(id), Some(name));
            assert_eq!(action_sources(id), SRC_KEY | SRC_PAD | SRC_PEN);
            assert!(!action_label(id).is_empty() && action_label(id) != "?");
        }
        assert!(SHORTCUT_LAST >= SHORTCUT_PAN_DOWN, "the loops over 1..=SHORTCUT_LAST reach them");
    }

    // The Graph's keys by place: the starter map's arrows turn pages on a page and pan in
    // the Graph, V switches its view. A file written before the Graph's actions and edited (so never upgraded)
    // that binds none of them has the arrows and V in the Graph too, its own lines keeping their meaning; a file
    // that binds any of them is taken as written; an empty file keeps nothing. Nothing is added to the map.
    #[test]
    fn an_older_file_has_the_graphs_keys_in_the_graph_only() {
        let look = |m: &Shortcuts, src: ActionSrc, code: i32, mods: i32| shortcuts_lookup_graph(m, src, code, mods);
        let map = load("defaults", None);
        assert_eq!(map.rejected, 0);
        assert_eq!(shortcuts_lookup_at(&map, ActionSrc::Key, LEFT, 0, PL_PAGE), SHORTCUT_PREV_PAGE);
        assert_eq!(look(&map, ActionSrc::Key, LEFT, 0), SHORTCUT_PAN_LEFT);
        assert_eq!(look(&map, ActionSrc::Key, UP, 0), SHORTCUT_PAN_UP);
        assert_eq!(look(&map, ActionSrc::Key, V, 0), SHORTCUT_GRAPH_VIEW);
        assert_eq!(look(&map, ActionSrc::Key, V, KEYMOD_CTRL), SHORTCUT_NONE);
        // Ctrl+= and Ctrl+A act in the Graph as written.
        assert_eq!(look(&map, ActionSrc::Key, 13, KEYMOD_CTRL), SHORTCUT_ZOOM_IN);
        assert_eq!(look(&map, ActionSrc::Key, 30, KEYMOD_CTRL), SHORTCUT_SELECT_ALL);
        // An older file someone edited: its lines keep their meaning on a page, and the Graph has its keys.
        let old = load("old", Some("key.Left=previous-page\nkey.Up=previous-tool\nkey.Ctrl+z=undo\n"));
        assert_eq!(old.rejected, 0);
        assert_eq!(shortcuts_lookup_at(&old, ActionSrc::Key, LEFT, 0, PL_PAGE), SHORTCUT_PREV_PAGE);
        assert_eq!(shortcuts_lookup_at(&old, ActionSrc::Key, UP, 0, PL_PAGE), SHORTCUT_PREV_TOOL);
        assert_eq!(look(&old, ActionSrc::Key, LEFT, 0), SHORTCUT_PAN_LEFT);
        assert_eq!(look(&old, ActionSrc::Key, UP, 0), SHORTCUT_PAN_UP);
        assert_eq!(look(&old, ActionSrc::Key, V, 0), SHORTCUT_GRAPH_VIEW);
        // (a controller's D-pad is not the keyboard: it keeps its page turns, which act on a page)
        assert_eq!(look(&old, ActionSrc::Pad, 546, 0), SHORTCUT_NONE);
        // A file naming one of them: no starter key.
        let own = load("own", Some("key.Left=previous-page\nkey.g=graph-view\n"));
        assert_eq!(look(&own, ActionSrc::Key, LEFT, 0), SHORTCUT_NONE);
        assert_eq!(look(&own, ActionSrc::Key, V, 0), SHORTCUT_NONE);
        assert_eq!(look(&own, ActionSrc::Key, 34, 0), SHORTCUT_GRAPH_VIEW);
        let empty = load("empty", Some(""));
        assert_eq!(look(&empty, ActionSrc::Key, LEFT, 0), SHORTCUT_NONE);
        // The last line for the key that acts in the Graph wins there.
        let both = load("both", Some("key.Left=pan-right\nkey.Left=pan-left\n"));
        assert_eq!(look(&both, ActionSrc::Key, LEFT, 0), SHORTCUT_PAN_LEFT);
        // One action in each place: a page turn then a pan for Left both act, each in its own place.
        let turn_then_pan = load("turn-then-pan", Some("key.Left=previous-page\nkey.Left=pan-left\n"));
        assert_eq!(shortcuts_lookup_at(&turn_then_pan, ActionSrc::Key, LEFT, 0, PL_PAGE), SHORTCUT_PREV_PAGE);
        assert_eq!(look(&turn_then_pan, ActionSrc::Key, LEFT, 0), SHORTCUT_PAN_LEFT);
    }
}
