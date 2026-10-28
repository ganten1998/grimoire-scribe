//! gesturemap: which action each gesture does. Built-in gestures, never written to a file, and the
//! owner's `<root>/gestures.ini` over them (docs/FORMAT.md): shortcuts.ini's grammar ('#' and ';'
//! comments, trimmed key=value lines, a later duplicate wins, keys in any case), read through
//! shortcuts.rs's bounded reader. Pure apart from that one read: no App, no UI.
//!
//! Keys: `touch.<2|3>.<up|down|left|right|tap>` (an action's file name, or `nothing` to turn a
//! built-in off; `touch.1.*` is kept for one-finger taps and links, and refused), `pen.<Button|
//! Button2|Eraser>.<tap|double|triple|hold|drag>` (parsed and held: the pen's button keeps its own
//! clicks for now) and `shape.<action>` (a finger shape someone recorded: carried as it is, for a
//! version that draws them). A line this version cannot use but a newer one wrote (an action it does
//! not know on a pen or shape line, a pen slot it does not know) is carried as it was; a line that
//! cannot be read is refused and counted. Nothing here writes the file.

use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchKind {
    Up,
    Down,
    Left,
    Right,
    Tap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PenBtn {
    Button,
    Button2,
    Eraser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PenKind {
    Tap,
    Double,
    Triple,
    Hold,
    Drag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKey {
    Touch { fingers: u8, kind: TouchKind },
    Pen { button: PenBtn, kind: PenKind },
}

/// A pen drag slot's own values besides actions: it pans or zooms the page.
pub const GESTURE_PAN: i32 = -1;
pub const GESTURE_ZOOM: i32 = -2;

/// gestures.ini is read up to this size (shortcuts.ini's cap).
const GESTURES_CAP: u64 = 65536;
/// Lines carried for a later version: each at most CARRY_LINE_MAX bytes, CARRY_MAX in all.
const CARRY_LINE_MAX: usize = 1024;
const CARRY_MAX: usize = 16384;

const K_TOUCH_KINDS: [(&str, TouchKind); 5] =
    [("up", TouchKind::Up), ("down", TouchKind::Down), ("left", TouchKind::Left), ("right", TouchKind::Right), ("tap", TouchKind::Tap)];
const K_PEN_BTNS: [(&str, PenBtn); 3] = [("Button", PenBtn::Button), ("Button2", PenBtn::Button2), ("Eraser", PenBtn::Eraser)];
const K_PEN_KINDS: [(&str, PenKind); 5] =
    [("tap", PenKind::Tap), ("double", PenKind::Double), ("triple", PenKind::Triple), ("hold", PenKind::Hold), ("drag", PenKind::Drag)];
/// The order a touch gesture is looked for by action (gesturemap_key_for): the built-ins' order.
const K_TOUCH_ORDER: [TouchKind; 5] = [TouchKind::Down, TouchKind::Up, TouchKind::Left, TouchKind::Right, TouchKind::Tap];

#[derive(Clone, Debug, Default)]
pub struct GestureMap {
    /// [fingers - 2][kind]: the action (0 nothing), and whether a line of the file set it
    touch: [[i32; 5]; 2],
    touch_set: [[bool; 5]; 2],
    /// [button][kind], the same
    pen: [[i32; 5]; 3],
    pen_set: [[bool; 5]; 3],
    /// the lines carried for a later version, as they were, each ended with a newline
    carried: Vec<u8>,
    /// lines refused, since the last gesturemap_defaults
    pub refused: i32,
}

fn tk(k: TouchKind) -> usize {
    k as usize
}

/// The built-ins only: two fingers down open Pages, up the Links bar, left and right turn the page;
/// three quick clicks of the pen's button, or holding it, clear the page (as the pen's button does).
/// Two- and three-finger taps and one finger's gestures do nothing.
pub fn gesturemap_defaults(m: &mut GestureMap) {
    *m = GestureMap::default();
    m.touch[0][tk(TouchKind::Down)] = SHORTCUT_PAGES;
    m.touch[0][tk(TouchKind::Up)] = SHORTCUT_LINKS;
    m.touch[0][tk(TouchKind::Left)] = SHORTCUT_NEXT_PAGE;
    m.touch[0][tk(TouchKind::Right)] = SHORTCUT_PREV_PAGE;
    m.pen[PenBtn::Button as usize][PenKind::Triple as usize] = SHORTCUT_CLEAR_PAGE;
    m.pen[PenBtn::Button as usize][PenKind::Hold as usize] = SHORTCUT_CLEAR_PAGE;
}

/// isspace() in the C locale, as shortcuts.ini's reader has it.
fn isspace(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
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

/// An action's file name as a newer version may write one: letters, digits, ':', '_', '-', '.'.
fn name_like(s: &[u8]) -> bool {
    !s.is_empty() && s.iter().all(|&c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'-' | b'.'))
}

/// The key's dot-separated parts, lower-cased: the head and up to two more (None past three or for
/// an empty part).
fn parts(key: &[u8]) -> Option<(String, String, Option<String>)> {
    let s = std::str::from_utf8(key).ok()?;
    let mut it = s.split('.');
    let (a, b, c) = (it.next()?, it.next()?, it.next());
    if it.next().is_some() || a.is_empty() || b.is_empty() || c.is_some_and(|c| c.is_empty()) {
        return None;
    }
    Some((a.to_ascii_lowercase(), b.to_ascii_lowercase(), c.map(|c| c.to_ascii_lowercase())))
}

/// The action a value names: Some(0) for `nothing`, Some(action) for an action this version knows.
fn value_action(v: &[u8]) -> Option<i32> {
    if v == b"nothing" {
        return Some(0);
    }
    let a = shortcut_action(std::str::from_utf8(v).ok()?);
    (a != SHORTCUT_NONE).then_some(a)
}

impl GestureMap {
    /// Carries a line for a later version, in place of an earlier one with the same key: false when
    /// it does not fit.
    fn carry(&mut self, key: &[u8], line: &[u8]) -> bool {
        if line.len() > CARRY_LINE_MAX {
            return false;
        }
        self.uncarry(key);
        if self.carried.len() + line.len() + 1 > CARRY_MAX {
            return false;
        }
        self.carried.extend_from_slice(line);
        self.carried.push(b'\n');
        true
    }

    /// Drops a carried line with this key (a later line wins).
    fn uncarry(&mut self, key: &[u8]) {
        let mut at = 0;
        while at < self.carried.len() {
            let end = at + self.carried[at..].iter().position(|&c| c == b'\n').unwrap_or(self.carried.len() - at);
            let line = &self.carried[at..end];
            let k = trim(&line[..line.iter().position(|&c| c == b'=').unwrap_or(line.len())]);
            if k.eq_ignore_ascii_case(key) {
                self.carried.drain(at..(end + 1).min(self.carried.len()));
            } else {
                at = end + 1;
            }
        }
    }

    /// One trimmed `key=value` line: false when it is refused.
    fn line(&mut self, line: &[u8]) -> bool {
        let Some(eq) = line.iter().position(|&c| c == b'=') else { return false };
        let (key, value) = (trim(&line[..eq]), trim(&line[eq + 1..]));
        let Some((head, a, b)) = parts(key) else { return false };
        match (head.as_str(), b) {
            ("touch", Some(kind)) => {
                let fingers = match a.as_str() {
                    "2" => 0,
                    "3" => 1,
                    _ => return false, // touch.1.* is one finger's (taps and links), and kept so
                };
                let Some(&(_, kind)) = K_TOUCH_KINDS.iter().find(|(n, _)| *n == kind) else { return false };
                let Some(action) = value_action(value) else { return false };
                self.touch[fingers][tk(kind)] = action;
                self.touch_set[fingers][tk(kind)] = true;
                true
            }
            ("pen", Some(kind)) => {
                let btn = K_PEN_BTNS.iter().find(|(n, _)| n.eq_ignore_ascii_case(&a)).map(|&(_, b)| b);
                let kind = K_PEN_KINDS.iter().find(|(n, _)| *n == kind).map(|&(_, k)| k);
                let (Some(btn), Some(kind)) = (btn, kind) else {
                    // a pen slot a later version has: carried
                    return name_like(value) && self.carry(key, line);
                };
                let (bi, ki) = (btn as usize, kind as usize);
                let action = match value {
                    b"pan" if kind == PenKind::Drag => Some(GESTURE_PAN),
                    b"zoom" if kind == PenKind::Drag => Some(GESTURE_ZOOM),
                    _ => value_action(value),
                };
                match action {
                    Some(action) => {
                        self.uncarry(key);
                        self.pen[bi][ki] = action;
                        self.pen_set[bi][ki] = true;
                        true
                    }
                    None if name_like(value) => {
                        // An action a later version has: carried, and the slot keeps its built-in.
                        let mut built_in = GestureMap::default();
                        gesturemap_defaults(&mut built_in);
                        self.pen[bi][ki] = built_in.pen[bi][ki];
                        self.pen_set[bi][ki] = false;
                        self.carry(key, line)
                    }
                    None => false,
                }
            }
            // A recorded finger shape (format 1): carried as it is for a version that draws them.
            ("shape", None) => name_like(a.as_bytes()) && value.starts_with(b"1,") && self.carry(key, line),
            _ => false,
        }
    }
}

/// gestures.ini's lines over what m holds (gesturemap_defaults first for the file alone): how many
/// lines were refused (also added to m.refused).
pub fn gesturemap_parse(m: &mut GestureMap, text: &[u8]) -> i32 {
    let mut refused = 0;
    // (as shortcuts.ini's: the empty lines between two newlines are skipped)
    for line in text.split(|&c| c == b'\n').filter(|l| !l.is_empty()) {
        let line = trim(line);
        if line.is_empty() || line[0] == b'#' || line[0] == b';' {
            continue;
        }
        if !m.line(line) {
            refused += 1;
        }
    }
    m.refused += refused;
    refused
}

/// The built-ins, then the file at path over them: the lines refused, or -1 when the file is there
/// but cannot be read (not a file, over 64 KiB, a read error), which leaves the built-ins alone. A
/// missing file is the built-ins (0).
pub fn gesturemap_load(m: &mut GestureMap, path: &str) -> i32 {
    gesturemap_defaults(m);
    match crate::shortcuts::ini_read_bounded(path, GESTURES_CAP) {
        Ok(Some(text)) => gesturemap_parse(m, &text),
        Ok(None) => 0,
        Err(()) => -1,
    }
}

/// The action a gesture does, 0 for nothing.
pub fn gesturemap_action(m: &GestureMap, k: GestureKey) -> i32 {
    match k {
        GestureKey::Touch { fingers: f @ 2..=3, kind } => m.touch[(f - 2) as usize][tk(kind)],
        GestureKey::Touch { .. } => 0,
        GestureKey::Pen { button, kind } => m.pen[button as usize][kind as usize],
    }
}

/// The first gesture of src's kind that does action: for touch in the order 2.down, 2.up, 2.left,
/// 2.right, 2.tap, then the same with three fingers; None for none (or for a source with no
/// gestures here).
pub fn gesturemap_key_for(m: &GestureMap, action: i32, src: ActionSrc) -> Option<GestureKey> {
    if action == SHORTCUT_NONE {
        return None;
    }
    match src {
        ActionSrc::Touch => {
            for fingers in 2..=3u8 {
                for kind in K_TOUCH_ORDER {
                    let k = GestureKey::Touch { fingers, kind };
                    if gesturemap_action(m, k) == action {
                        return Some(k);
                    }
                }
            }
            None
        }
        ActionSrc::Pen => {
            for &(_, button) in &K_PEN_BTNS {
                for &(_, kind) in &K_PEN_KINDS {
                    let k = GestureKey::Pen { button, kind };
                    if gesturemap_action(m, k) == action {
                        return Some(k);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// The same gesture the other way: up and down, left and right; a tap, and the pen's, stay.
pub fn gesturemap_mirror(k: GestureKey) -> GestureKey {
    match k {
        GestureKey::Touch { fingers, kind } => GestureKey::Touch {
            fingers,
            kind: match kind {
                TouchKind::Up => TouchKind::Down,
                TouchKind::Down => TouchKind::Up,
                TouchKind::Left => TouchKind::Right,
                TouchKind::Right => TouchKind::Left,
                TouchKind::Tap => TouchKind::Tap,
            },
        },
        pen => pen,
    }
}

/// touchgest's gesture (InEvent.value of an InType::Gesture) as a key.
pub fn gesturemap_key_of_id(id: i32) -> Option<GestureKey> {
    const KINDS: [TouchKind; 5] = [TouchKind::Up, TouchKind::Down, TouchKind::Left, TouchKind::Right, TouchKind::Tap];
    if !(G_2_UP..=G_3_TAP).contains(&id) {
        return None;
    }
    let i = (id - G_2_UP) as usize;
    Some(GestureKey::Touch { fingers: 2 + (i / 5) as u8, kind: KINDS[i % 5] })
}

/// A key as gestures.ini names it: "touch.2.down", "pen.Button.triple".
pub fn gesturemap_key_name(k: GestureKey, out: &mut String) {
    out.clear();
    match k {
        GestureKey::Touch { fingers, kind } => {
            out.push_str(&cformat!("touch.%d.%s", fingers as i32, K_TOUCH_KINDS[tk(kind)].0));
        }
        GestureKey::Pen { button, kind } => {
            out.push_str(&cformat!("pen.%s.%s", K_PEN_BTNS[button as usize].0, K_PEN_KINDS[kind as usize].0));
        }
    }
}

/// Whether a line of gestures.ini set this gesture (a built-in is never written to the file).
pub fn gesturemap_is_set(m: &GestureMap, k: GestureKey) -> bool {
    match k {
        GestureKey::Touch { fingers: f @ 2..=3, kind } => m.touch_set[(f - 2) as usize][tk(kind)],
        GestureKey::Touch { .. } => false,
        GestureKey::Pen { button, kind } => m.pen_set[button as usize][kind as usize],
    }
}

/// The lines carried for a later version (shape lines among them), as they were, each ended with a
/// newline.
pub fn gesturemap_carried(m: &GestureMap) -> &[u8] {
    &m.carried
}
