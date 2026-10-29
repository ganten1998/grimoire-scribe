//! gesturemap: which action each gesture does. Built-in gestures, never written to a file, and the
//! owner's `<root>/gestures.ini` over them (docs/FORMAT.md): shortcuts.ini's grammar ('#' and ';'
//! comments, trimmed key=value lines, a later duplicate wins, keys in any case), read through
//! shortcuts.rs's bounded reader. Pure apart from that one read: no App, no UI.
//!
//! One-finger directions, multi-finger patterns and recorded shapes share this atomic sidecar.
//! Unknown versions and future actions are carried within fixed bounds. A failed read prevents
//! writes until Reset, and callers save an edited copy before publishing it to the App.

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

/// Stable registry actions, assignable only to a live pen button's drag slot.
pub const GESTURE_PAN: i32 = SHORTCUT_PEN_PAN;
pub const GESTURE_ZOOM: i32 = SHORTCUT_PEN_ZOOM;

/// gestures.ini is read up to this size (shortcuts.ini's cap).
pub const GESTURES_CAP: u64 = 65536;
/// Lines carried for a later version: each at most CARRY_LINE_MAX bytes, CARRY_MAX in all.
const CARRY_LINE_MAX: usize = 1024;
const CARRY_MAX: usize = 16384;

const K_TOUCH_KINDS: [(&str, TouchKind); 5] =
    [("up", TouchKind::Up), ("down", TouchKind::Down), ("left", TouchKind::Left), ("right", TouchKind::Right), ("tap", TouchKind::Tap),
];
const K_PEN_BTNS: [(&str, PenBtn); 3] = [("Button", PenBtn::Button), ("Button2", PenBtn::Button2), ("Eraser", PenBtn::Eraser),
];
const K_PEN_KINDS: [(&str, PenKind); 5] =
    [("tap", PenKind::Tap), ("double", PenKind::Double), ("triple", PenKind::Triple), ("hold", PenKind::Hold), ("drag", PenKind::Drag),
];
/// The order a touch gesture is looked for by action (gesturemap_key_for): the built-ins' order.
const K_TOUCH_ORDER: [TouchKind; 5] = [TouchKind::Down, TouchKind::Up, TouchKind::Left, TouchKind::Right, TouchKind::Tap,
];

#[derive(Clone, Debug, Default)]
pub struct GestureMap {
    /// [fingers - 2][kind]: the action (0 nothing), and whether a line of the file set it
    touch: [[i32; 5]; 2],
    touch1: [i32; 4],
    touch1_set: [bool; 4],
    shapes: Vec<(i32, FsGesture)>,
    /// False after an unreadable file; Reset is the explicit recovery path.
    pub writable: bool,
    touch_set: [[bool; 5]; 2],
    /// [button][kind], the same
    pen: [[i32; 5]; 3],
    pen_set: [[bool; 5]; 3],
    /// Hardware capability, set before load and retained by defaults and Reset.
    pen_button2: bool,
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
    let pen_button2 = m.pen_button2;
    *m = GestureMap::default();
    m.pen_button2 = pen_button2;
    m.writable = true;
    m.touch[0][tk(TouchKind::Down)] = SHORTCUT_PAGES;
    m.touch[0][tk(TouchKind::Up)] = SHORTCUT_LINKS;
    m.touch[0][tk(TouchKind::Left)] = SHORTCUT_NEXT_PAGE;
    m.touch[0][tk(TouchKind::Right)] = SHORTCUT_PREV_PAGE;
    m.pen[PenBtn::Button as usize][PenKind::Triple as usize] = SHORTCUT_CLEAR_PAGE;
    m.pen[PenBtn::Button as usize][PenKind::Hold as usize] = SHORTCUT_CLEAR_PAGE;
}

/// Set the hardware capability before loading the map. Reload to activate carried Button2 lines.
pub fn gesturemap_pen_button2(m: &mut GestureMap, enabled: bool) {
    m.pen_button2 = enabled;
}

fn pen_live(m: &GestureMap, button: PenBtn) -> bool {
    button == PenBtn::Button || (button == PenBtn::Button2 && m.pen_button2)
}

/// Clicks wait only where a live double or triple binding needs disambiguation.
pub fn gesturemap_pen_config(m: &GestureMap) -> PbConfig {
    let mut c = PbConfig { waits: [false; 3], ..PbConfig::default() };
    for &(_, button) in &K_PEN_BTNS {
        c.waits[button as usize] = [PenKind::Double, PenKind::Triple].iter().any(|&kind|
            gesturemap_action(m, GestureKey::Pen { button, kind }) != 0);
    }
    c
}

fn pen_action_valid(kind: PenKind, action: i32) -> bool {
    action == 0 || (action_sources(action) & SRC_PEN != 0 && match action {
        GESTURE_PAN | GESTURE_ZOOM => kind == PenKind::Drag,
        SHORTCUT_HOLD_ERASER => matches!(kind, PenKind::Hold | PenKind::Drag),
        _ => true,
    })
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
    Some((a.to_ascii_lowercase(), b.to_ascii_lowercase(), c.map(|c| c.to_ascii_lowercase()),
    ))
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
        let replaced: usize = self.carried
            .split_inclusive(|&b| b == b'\n')
            .filter(|l| {
                let end = l.iter().position(|&b| b == b'=').unwrap_or(l.len());
                trim(&l[..end]).eq_ignore_ascii_case(key)
            })
            .map(|l| l.len())
            .sum();
        if self.carried.len() - replaced + line.len() + 1 > CARRY_MAX {
            return false;
        }
        if self.carried.try_reserve_exact(line.len() + 1).is_err() {
            return false;
        }
        self.uncarry(key);
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

    fn put_pen(&mut self, button: PenBtn, kind: PenKind, action: i32) {
        self.pen[button as usize][kind as usize] = action;
        self.pen_set[button as usize][kind as usize] = true;
        let mut key = String::new();
        gesturemap_key_name(GestureKey::Pen { button, kind }, &mut key);
        self.uncarry(key.as_bytes());
    }

    fn set_pen(&mut self, button: PenBtn, kind: PenKind, action: i32, reading: bool) {
        for &(_, other_button) in &K_PEN_BTNS {
            if !pen_live(self, other_button) { continue; }
            for &(_, other_kind) in &K_PEN_KINDS {
                if (button, kind) == (other_button, other_kind) { continue; }
                // The two historical Clear page defaults are the sole duplicate exception on read.
                let builtin_pair = reading && action == SHORTCUT_CLEAR_PAGE
                    && button == PenBtn::Button && other_button == PenBtn::Button
                    && matches!(kind, PenKind::Triple | PenKind::Hold)
                    && matches!(other_kind, PenKind::Triple | PenKind::Hold);
                if action != 0 && !builtin_pair
                    && self.pen[other_button as usize][other_kind as usize] == action {
                    self.put_pen(other_button, other_kind, 0);
                }
            }
        }
        self.put_pen(button, kind, action);
    }

    /// One raw `key=value` line: false when refused; trim only the parsed key and value.
    fn line(&mut self, line: &[u8]) -> bool {
        if !self.writable { return false; }
        let Some(eq) = line.iter().position(|&c| c == b'=') else { return false ;
        };
        let (key, value) = (trim(&line[..eq]), trim(&line[eq + 1..]));
        let Some((head, a, b)) = parts(key) else { return false ;
        };
        match (head.as_str(), b) {
            ("touch", Some(kind)) => {
                let fingers = match a.as_str() {
                    "1" => 1,
                    "2" => 2,
                    "3" => 3,
                    _ => return false, };
                let Some(&(_, kind)) = K_TOUCH_KINDS.iter().find(|(n, _)| *n == kind) else { return false ;
                };
                let Some(action) = value_action(value) else { return false ;
                };
                if fingers == 1 && kind == TouchKind::Tap {
                    return false;
                }
                if action != 0 && action_sources(action) & SRC_TOUCH == 0 {
                    return false;
                }
                gesturemap_set(self, GestureKey::Touch { fingers, kind}, action)>= 0
            }
            ("pen", Some(kind)) => {
                let btn = K_PEN_BTNS.iter().find(|(n, _)| n.eq_ignore_ascii_case(&a)).map(|&(_, b)| b);
                let kind = K_PEN_KINDS.iter().find(|(n, _)| *n == kind).map(|&(_, k)| k);
                let (Some(btn), Some(kind)) = (btn, kind) else {
                    // a pen slot a later version has: carried
                    return name_like(value) && self.carry(key, line);
                };
                if !pen_live(self, btn) {
                    return name_like(value) && self.carry(key, line);
                }
                match value_action(value) {
                    Some(action) => {
                        if !pen_action_valid(kind, action) { return false; }
                        self.set_pen(btn, kind, action, true);
                        true
                    }
                    None if name_like(value) => {
                        // Commit the fallback only after carrying succeeded: refusal is atomic.
                        if !self.carry(key, line) { return false; }
                        let built_in = if btn == PenBtn::Button
                            && matches!(kind, PenKind::Triple | PenKind::Hold) {
                            SHORTCUT_CLEAR_PAGE
                        } else { 0 };
                        self.pen[btn as usize][kind as usize] = built_in;
                        self.pen_set[btn as usize][kind as usize] = false;
                        true
                    }
                    None => false,
                }
            }
            ("shape", None) => {
                if !name_like(a.as_bytes()) {
                    return false;
                }
                let Ok(value) = std::str::from_utf8(value) else {
                    return false;
                };
                let action = action_by_name(&a);
                if action != 0 && action_sources(action) &SRC_SHAPE == 0 {
                    return false;
                }
                let mut samples = [FsSample::default(); FS_SAMPLES_MAX];
                match fs_decode(value, & mut samples) {
                    Err(FsCodec::Malformed) => false,
                    Err(FsCodec::Newer) => {
                        if !self.carry(key, line) {
                            return false;
                        }
                        self.shapes.retain(|(a, _)| *a != action);
                        true
                    }
                    Ok(_) if action == 0 => self.carry(key, line),
            Ok(n) => {
                        let at = self.shapes.binary_search_by_key(&action, |(a, _)| *a);
                        if at.is_err()
                            && (self.shapes.len() >= FS_SHAPES_MAX
                                || self.shapes.try_reserve_exact(1).is_err())
                        {
                            return false;
                        }
                        let mut g = FsGesture::default();
                        fs_prepare(&mut g, &samples[..n]);
                        self.uncarry(key);
                        match at {
                            Ok(i) => self.shapes[i].1 = g,
                            Err(i) => self.shapes.insert(i, (action, g)),
                        }
                        true
                    }
                }
            }
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
        let trimmed = trim(line);
        if trimmed.is_empty() || trimmed[0] == b'#' || trimmed[0] == b';' {
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
        Err(()) => {
            m.writable = false;
            -1}
    }
}

/// The action a gesture does, 0 for nothing.
pub fn gesturemap_action(m: &GestureMap, k: GestureKey) -> i32 {
    match k {
        GestureKey::Touch { fingers: 1, kind } if kind != TouchKind::Tap => m.touch1[tk(kind)],
        GestureKey::Touch {
            fingers: f @ 2..=3, kind ,
        } => m.touch[(f - 2) as usize][tk(kind)],
        GestureKey::Touch { .. } => 0,
        GestureKey::Pen { button, kind } if pen_live(m, button) => m.pen[button as usize][kind as usize],
        GestureKey::Pen { .. } => 0,
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
            for fingers in [2, 3, 1] {
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
    const KINDS: [TouchKind; 5] = [TouchKind::Up, TouchKind::Down, TouchKind::Left, TouchKind::Right, TouchKind::Tap,
    ];
    if !(G_2_UP..=G_3_TAP).contains(&id) {
        return None;
    }
    let i = (id - G_2_UP) as usize;
    Some(GestureKey::Touch { fingers: 2 + (i / 5) as u8, kind: KINDS[i % 5] ,
    })
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
        GestureKey::Touch { fingers: 1, kind } if kind != TouchKind::Tap => m.touch1_set[tk(kind)],
        GestureKey::Touch {
            fingers: f @ 2..=3, kind ,
        } => m.touch_set[(f - 2) as usize][tk(kind)],
        GestureKey::Touch { .. } => false,
        GestureKey::Pen { button, kind } => m.pen_set[button as usize][kind as usize],
    }
}

/// The lines carried for a later version (shape lines among them), as they were, each ended with a
/// newline.
pub fn gesturemap_carried(m: &GestureMap) -> &[u8] {
    &m.carried
}
pub const GM_SHAPES_MAX: usize = FS_SHAPES_MAX;
const HEADER: &str = concat!(
    "# Grimoire: gestures (Settings > Gestures). Built in unless a line says otherwise:\n",
    "# touch.2.down=pages, touch.2.up=links, touch.2.left=next-page, touch.2.right=previous-page, pen.Button.triple=clear-page, pen.Button.hold=clear-page\n",
    "# shape.ACTION=1,N,S1,S2[,S3]: a finger shape you drew; format 1, N samples; each sample is 32 points in drawing order, x then y as two hex digits, the larger side spanning 00-ff.\n",
);

pub fn gesturemap_path() -> String {
    cformat!("%s/gestures.ini", &store_root())
}

fn put_touch(m: &mut GestureMap, fingers: u8, kind: TouchKind, action: i32) {
    if fingers == 1 {
        m.touch1[tk(kind)] = action;
        m.touch1_set[tk(kind)] = true;
    } else {
        m.touch[(fingers - 2) as usize][tk(kind)] = action;
        m.touch_set[(fingers - 2) as usize][tk(kind)] = true;
    }
}

/// Move this gesture to an action; each source has at most one edited binding per action.
/// Shapes are separate. Pen slots must be live and support the action's release requirements.
pub fn gesturemap_set(m: &mut GestureMap, k: GestureKey, action: i32) -> i32 {
    if !m.writable {
        return -1;
    }
    let (fingers, kind) = match k {
        GestureKey::Pen { button, kind } => {
            if !pen_live(m, button) || !pen_action_valid(kind, action) { return -1; }
            let lost = gesturemap_action(m, k);
            m.set_pen(button, kind, action, false);
            return if lost != action { lost } else { 0 };
        }
        GestureKey::Touch { fingers, kind } => (fingers, kind),
    };
    if !(1..=3).contains(&fingers)
        || (fingers == 1 && kind == TouchKind::Tap)
        || (action != 0 && action_sources(action) & SRC_TOUCH == 0)
    {
        return -1;
    }
    let lost = gesturemap_action(m, k);
    if action != 0 {
        for f in 1..=3 {
            for &(_, t) in &K_TOUCH_KINDS {
                if f == 1 && t == TouchKind::Tap {
                    continue;
                }
                let other = GestureKey::Touch {
                    fingers: f,
                    kind: t,
                };
                if other != k && gesturemap_action(m, other) == action {
                    put_touch(m, f, t, 0);
                }
            }
        }
    }
    put_touch(m, fingers, kind, action);
    if lost != action {
        lost
    } else {
        0
    }
}

pub fn gesturemap_clear_action(m: &mut GestureMap, action: i32) {
    if !m.writable || action == 0 {
        return;
    }
    for f in 1..=3 {
        for &(_, kind) in &K_TOUCH_KINDS {
            if f == 1 && kind == TouchKind::Tap {
                continue;
            }
            if gesturemap_action(m, GestureKey::Touch { fingers: f, kind }) == action {
                put_touch(m, f, kind, 0);
            }
        }
    }
    for &(_, button) in &K_PEN_BTNS {
        if !pen_live(m, button) { continue; }
        for &(_, kind) in &K_PEN_KINDS {
            if gesturemap_action(m, GestureKey::Pen { button, kind }) == action {
                m.put_pen(button, kind, 0);
            }
        }
    }
    m.shapes.retain(|(a, _)| *a != action);
    m.uncarry(format!("shape.{}", shortcut_action_name(action).unwrap_or("")).as_bytes());
}

pub fn gesturemap_shapes(m: &GestureMap) -> &[(i32, FsGesture)] {
    &m.shapes
}
pub fn gesturemap_shape_of(m: &GestureMap, action: i32) -> Option<&FsGesture> {
    m.shapes
        .binary_search_by_key(&action, |(a, _)| *a)
        .ok()
        .map(|i| &m.shapes[i].1)
}
pub fn gesturemap_shape_newer(m: &GestureMap, action: i32) -> bool {
    let key = format!("shape.{}", shortcut_action_name(action).unwrap_or(""));
    m.carried.split(|&b| b == b'\n').any(|l| {
        l.iter()
            .position(|&b| b == b'=')
            .is_some_and(|i| trim(&l[..i]).eq_ignore_ascii_case(key.as_bytes()))
    })
}
pub fn gesturemap_has_onefinger(m: &GestureMap) -> bool {
    m.touch1.iter().any(|&a| a != 0)
}

/// Install a checked recording; an existing action's same shape moves here, leaving touch alone.
pub fn gesturemap_shape_set(m: &mut GestureMap, action: i32, samples: &[FsSample]) -> i32 {
    let mut encoded = [0u8; FS_VALUE_MAX];
    if !m.writable
        || action_sources(action) & SRC_SHAPE == 0
        || fs_encode(samples, &mut encoded).is_err()
    {
        return -1;
    }
    let mut g = FsGesture::default();
    fs_prepare(&mut g, samples);
    let mut bank = [FsGesture::default(); FS_SHAPES_MAX];
    let mut actions = [0; FS_SHAPES_MAX];
    let mut n = 0;
    for &(a, h) in &m.shapes {
        if a != action {
            bank[n] = h;
            actions[n] = a;
            n += 1;
        }
    }
    let lost = match fs_record_separation(&g, &bank[..n]) {
        FsSeparation::Clear => 0,
        FsSeparation::SameAs(i) => actions[i],
        FsSeparation::TooClose(_) => return -1,
    };
    let exists = gesturemap_shape_of(m, action).is_some();
    if !exists && lost == 0 && m.shapes.len() >= FS_SHAPES_MAX {
        return -1;
    }
    if !exists && lost == 0 && m.shapes.try_reserve_exact(1).is_err() {
        return -1;
    }
    m.shapes.retain(|(a, _)| *a != lost && *a != action);
    let at = m
        .shapes
        .binary_search_by_key(&action, |(a, _)| *a)
        .unwrap_err();
    m.shapes.insert(at, (action, g));
    m.uncarry(format!("shape.{}", shortcut_action_name(action).unwrap_or("")).as_bytes());
    if lost != 0 {
        m.uncarry(format!("shape.{}", shortcut_action_name(lost).unwrap_or("")).as_bytes());
    }
    lost
}

/// Pure canonical writer. Built-ins are implicit; unknown lines retain their read order and bytes.
pub fn gesturemap_format(m: &GestureMap, out: &mut Vec<u8>) -> bool {
    out.clear();
    if !m.writable || out.try_reserve_exact(GESTURES_CAP as usize).is_err() {
        return false;
    }
    out.extend_from_slice(HEADER.as_bytes());
    let mut built_in = GestureMap::default();
    gesturemap_defaults(&mut built_in);
    let mut key = String::new();
    for f in 1..=3 {
        for &(_, kind) in &K_TOUCH_KINDS {
            if f == 1 && kind == TouchKind::Tap {
                continue;
            }
            let k = GestureKey::Touch { fingers: f, kind };
            let action = gesturemap_action(m, k);
            if action != gesturemap_action(&built_in, k) {
                gesturemap_key_name(k, &mut key);
                let value = if action == 0 {
                    "nothing"
                } else {
                    shortcut_action_name(action).unwrap_or("")
                };
                out.extend_from_slice(format!("{key}={value}\n").as_bytes());
            }
        }
    }
    for &(_, button) in &K_PEN_BTNS {
        for &(_, kind) in &K_PEN_KINDS {
            let k = GestureKey::Pen { button, kind };
            if !pen_live(m, button) || !gesturemap_is_set(m, k) {
                continue;
            }
            gesturemap_key_name(k, &mut key);
            let value = match gesturemap_action(m, k) {
                0 => "nothing",
                GESTURE_PAN => "pan",
                GESTURE_ZOOM => "zoom",
                a => shortcut_action_name(a).unwrap_or(""),
            };
            out.extend_from_slice(format!("{key}={value}\n").as_bytes());
        }
    }
    let mut encoded = [0u8; FS_VALUE_MAX];
    for &(action, g) in &m.shapes {
        let Ok(value) = fs_encode(&g.samples[..g.n], &mut encoded) else {
            return false;
        };
        out.extend_from_slice(
            format!(
                "shape.{}={value}\n",
                shortcut_action_name(action).unwrap_or("")
            )
            .as_bytes(),
        );
    }
    if out.len() + m.carried.len() > GESTURES_CAP as usize {
        return false;
    }
    out.extend_from_slice(&m.carried);
    true
}

/// Atomic; an empty overlay removes the file. Refuse an unreadable source until explicit Reset.
pub fn gesturemap_save(m: &GestureMap, path: &str) -> i32 {
    let mut text = Vec::new();
    if !gesturemap_format(m, &mut text) {
        return -1;
    }
    if text.len() == HEADER.len() {
        return match std::fs::remove_file(path) {
            Ok(()) => 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 1,
            Err(_) => -1,
        };
    }
    write_file_atomic(path, &text)
}

pub fn gesturemap_reset(m: &mut GestureMap, path: &str) -> i32 {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return -1,
    }
    gesturemap_defaults(m);
    0
}
