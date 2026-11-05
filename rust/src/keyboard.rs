//! keyboard.h: on-screen keyboard modal for naming notebooks and folders.
//! QWERTY + numbers row, shift, space, backspace, Cancel / Done, - ' and . beside the space bar, and
//! a key each to a page of symbols and a page of accented letters (and back). Shift tapped once gives
//! the next letter's capital, twice caps on, a third time off (KbShift). Pen or finger taps. The text is UTF-8: a key types a whole character and
//! backspace takes a whole character away. In Japanese the accents key is a kana key: a flick
//! keypad, twelve keys in three columns, where a tap types the row's first kana and a swipe left,
//! up, right or down the rest of the row (a mobile phone's kana pad), with ー beside it, one
//! modifier that cycles the kana typed last (゛小: か to が, は to ば to ぱ, つ to づ to っ), カ for
//! katakana and 、 for 、。？！.

use crate::prelude::*;

pub const KB_TEXT_MAX: usize = 96;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KbResult {
    #[default]
    None = 0,  // KB_NONE
    Changed = 1,  // KB_CHANGED
    Done = 2,  // KB_DONE
    Cancel = 3,  // KB_CANCEL
}

/// Key codes: a character key's code is its Unicode code point as typed without shift; the
/// other keys have codes below 32.
pub const KB_KEY_SHIFT: u32 = 1;
pub const KB_KEY_PAGE: u32 = 2;
pub const KB_KEY_SYMBOLS: u32 = 3;
/// the kana page (Japanese: the letters page's accents key becomes this)
pub const KB_KEY_KANA: u32 = 4;
/// ゛小: the kana typed last in its next form, cycling (か to が and back; は to ば to ぱ to は;
/// つ to づ to っ to つ; や to ゃ to や)
pub const KB_KEY_KANA_MOD: u32 = 5;
/// カ: the kana page in katakana (カ キ ク), or back to hiragana
pub const KB_KEY_KATA: u32 = 7;
/// + row: the kana page's own keys, whose swipe picks one of the row's five kana
pub const KB_KEY_FLICK: u32 = 16;
pub const KB_KEY_BKSP: u32 = 8;
pub const KB_KEY_DONE: u32 = 13;
pub const KB_KEY_CANCEL: u32 = 27;
/// The count pad's (page 4) - and +: the count one less, one more.
pub const KB_KEY_DEC: u32 = 14;
pub const KB_KEY_INC: u32 = 15;

/// The shift key's three states, a tap each in this order and round again (the owner, 2026-11-05: "next
/// toggle shift in typing toggles off shift for next char then shift on which makes longer caps strings
/// possible"). The letter keys show their capitals in both on-states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KbShift {
    /// small letters
    #[default]
    Off,
    /// the next letter a capital, then off; a key with no capital (a digit, a symbol, the space bar)
    /// leaves it on, waiting for a letter. Drawn as the pressed key.
    Once,
    /// caps on: every letter a capital until shift is tapped again or the keyboard closes; digits,
    /// symbols, the space bar, delete and the page keys leave it on. Drawn as the pressed key with a bar
    /// under its label.
    Caps,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Keyboard {
    pub open: bool,
    pub title: FixedStr<64>,
    pub text: FixedStr<KB_TEXT_MAX>,
    /// the shift key's state (keyboard_open starts every keyboard with it off)
    pub shift: KbShift,
    /// the kana page in katakana (カ toggles it): the pad types カ キ ク, not か き く
    pub kata: bool,
    /// 0 letters, 1 accents, 2 number pad, 3 symbols, 4 count pad (keyboard_open_count), 5 kana
    pub page: i32,
    /// where it was last drawn (whole modal)
    pub area: Rect,
    pub press: Press,
    /// -1 if none
    pub pressed_key: i32,
    /// on the kana page, the way the pressed key was swiped: 0 while still on it (a tap), 1 left,
    /// 2 up, 3 right, 4 down. Its kana is typed on the lift.
    pub flick: i32,
    /// the pressed key already typed on touch-down
    pub typed: bool,
    /// set by keyboard_pointer when the text field changed (refresh it first)
    pub field_dirty: Rect,
    /// when a pause in typing gets the keys their grayscale refresh
    /// (app_keyboard_tick), 0 for none due
    pub settle_at: u32,
    /// 0 docks the modal at the bottom of the screen (every keyboard until a caller says otherwise);
    /// otherwise the modal's top y on the screen, for a keyboard that must keep clear of something low on
    /// the page (a link's area being given a target)
    pub dock_y: i32,
    /// The count pad's (page 4): the most it takes (a digit or + past it leaves the count there, which the
    /// pad shows), and its Done's label for a count (Done disabled at 0)
    pub count_max: i32,
    pub count_label: Option<fn(i32) -> String>,
    /// set with field_dirty on the count pad: its Done, whose label shows the count (refreshed on its own)
    pub also_dirty: Rect,
    /// The count pad's count is still the 1 it opened with (the owner: "1 will be the default"): drawn as any
    /// count is but with no caret after it, and the first digit typed replaces it rather than following it.
    pub count_fresh: bool,
}

// keyboard.c: on-screen keyboard modal. Layout is computed from the area it was drawn in,
// so hit testing and drawing always agree. keyboard_draw paints the whole modal (cheap);
// keyboard_pointer reports only the rect that needs refreshing. The text is UTF-8 throughout:
// keys carry code points, a key appends a whole character and backspace removes one.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default)]
struct Key {
    r: Rect,
    /// a code point, or a KB_KEY_* code
    code: u32,
}

const MAX_KEYS: usize = 48;

const KEY_SPACE: u32 = ' ' as u32;

// A row of the tables below, padded with the zeros that end it.
const fn row(keys: &[u16]) -> [u16; 11] {
    let mut r = [0u16; 11];
    let mut i = 0;
    while i < keys.len() {
        r[i] = keys[i];
        i += 1;
    }
    r
}

/// The character keys of each page, row by row (zero-terminated). Rows 0-2 span the keyboard;
/// row 3 sits between shift and delete.
const K_ROWS: [[[u16; 11]; 4]; 3] = [
    [
        row(&['1' as u16, '2' as u16, '3' as u16, '4' as u16, '5' as u16, '6' as u16, '7' as u16, '8' as u16, '9' as u16, '0' as u16]),
        row(&['q' as u16, 'w' as u16, 'e' as u16, 'r' as u16, 't' as u16, 'y' as u16, 'u' as u16, 'i' as u16, 'o' as u16, 'p' as u16]),
        row(&['a' as u16, 's' as u16, 'd' as u16, 'f' as u16, 'g' as u16, 'h' as u16, 'j' as u16, 'k' as u16, 'l' as u16]),
        row(&['z' as u16, 'x' as u16, 'c' as u16, 'v' as u16, 'b' as u16, 'n' as u16, 'm' as u16]),
    ],
    // Accents, enough for Spanish, Portuguese, French, Italian, German, Dutch, Catalan, Danish,
    // Norwegian, Swedish and Finnish (shift gives the capitals): a e i o u with an acute, then
    // a grave; with a circumflex, then a diaeresis; n tilde, c cedilla, a and o tilde, sharp s,
    // oe, ae, o slash, a ring; the inverted question and exclamation marks, the angle quotes and
    // the middle dot Catalan puts between two l's.
    [
        row(&[0xE1, 0xE9, 0xED, 0xF3, 0xFA, 0xE0, 0xE8, 0xEC, 0xF2, 0xF9]),
        row(&[0xE2, 0xEA, 0xEE, 0xF4, 0xFB, 0xE4, 0xEB, 0xEF, 0xF6, 0xFC]),
        row(&[0xF1, 0xE7, 0xE3, 0xF5, 0xDF, 0x153, 0xE6, 0xF8, 0xE5]),
        row(&[0xBF, 0xA1, 0xAB, 0xBB, 0xB7]),
    ],
    // Symbols: the digits again (for "(2)" without a trip back), under them what shift gives on a
    // US keyboard's digits, then the rest of the punctuation names and searches take, brackets, the
    // euro, the pound and the degree sign. (- ' and . are beside the space bar on every page.)
    [
        row(&['1' as u16, '2' as u16, '3' as u16, '4' as u16, '5' as u16, '6' as u16, '7' as u16, '8' as u16, '9' as u16, '0' as u16]),
        row(&['!' as u16, '@' as u16, '#' as u16, '$' as u16, '%' as u16, '^' as u16, '&' as u16, '*' as u16, '(' as u16, ')' as u16]),
        row(&['?' as u16, ',' as u16, '"' as u16, ':' as u16, ';' as u16, '_' as u16, '+' as u16, '=' as u16, '/' as u16, '~' as u16]),
        row(&['[' as u16, ']' as u16, '<' as u16, '>' as u16, 0x20AC, 0xA3, 0xB0]),
    ],
];

/// In Spanish the letters page's middle row ends with n tilde, as on Spanish keyboards (to the right
/// of l): a letter of the alphabet there, not an accent to fetch from the other page.
const K_ROW_ES: [u16; 11] = row(&['a' as u16, 's' as u16, 'd' as u16, 'f' as u16, 'g' as u16, 'h' as u16, 'j' as u16, 'k' as u16, 'l' as u16, 0xF1]);

/// The kana page's eleventh and twelfth cells: the modifier and ー, after the ten kana rows.
const K_KANA_MOD_CELL: usize = 10;
const KANA_CHOON: u32 = 0x30FC; // ー, the long vowel mark
/// The punctuation key, which the bottom row holds: K_KANA's last row.
pub const KB_KEY_PUNCT: u32 = KB_KEY_FLICK + 10;

/// What each kana key types, in the order a finger swipes it: tap, left, up, right, down (0 where
/// the row has no such kana). あ is the gojuon's first row, わ its last; the last row is the
/// punctuation key's. A phone's kana pad puts や with its brackets on the sides and ゆ above, よ
/// below; 、。？！ are on the punctuation key's flicks.
const K_KANA: [[u16; 5]; 11] = [
    [0x3042, 0x3044, 0x3046, 0x3048, 0x304A], // あ い う え お
    [0x304B, 0x304D, 0x304F, 0x3051, 0x3053], // か き く け こ
    [0x3055, 0x3057, 0x3059, 0x305B, 0x305D], // さ し す せ そ
    [0x305F, 0x3061, 0x3064, 0x3066, 0x3068], // た ち つ て と
    [0x306A, 0x306B, 0x306C, 0x306D, 0x306E], // な に ぬ ね の
    [0x306F, 0x3072, 0x3075, 0x3078, 0x307B], // は ひ ふ へ ほ
    [0x307E, 0x307F, 0x3080, 0x3081, 0x3082], // ま み む め も
    [0x3084, 0x300C, 0x3086, 0x300D, 0x3088], // や 「 ゆ 」 よ
    [0x3089, 0x308A, 0x308B, 0x308C, 0x308D], // ら り る れ ろ
    [0x308F, 0x3092, 0x3093, 0, 0], // わ を ん
    [0x3001, 0x3002, 0xFF1F, 0xFF01, 0x30FB], // 、 。 ？ ！ ・
];

fn is_flick(code: u32) -> bool {
    (KB_KEY_FLICK..KB_KEY_FLICK + 11).contains(&code)
}

/// The voiced kana of one that has a voiced form (か to が, う to ゔ, つ to づ, は to ば); anything
/// else comes back as it is.
fn kana_dakuten(cp: u32) -> u32 {
    match cp {
        0x3046 => 0x3094, // う to ゔ
        0x304B | 0x304D | 0x304F | 0x3051 | 0x3053 // か row
        | 0x3055 | 0x3057 | 0x3059 | 0x305B | 0x305D // さ row
        | 0x305F | 0x3061 | 0x3064 | 0x3066 | 0x3068 // た row
        | 0x306F | 0x3072 | 0x3075 | 0x3078 | 0x307B => cp + 1, // は row
        _ => cp,
    }
}

/// The half-voiced kana of one that has that form (は to ぱ); anything else comes back as it is.
fn kana_handakuten(cp: u32) -> u32 {
    match cp {
        0x306F | 0x3072 | 0x3075 | 0x3078 | 0x307B => cp + 2, // は row to ぱ row
        _ => cp,
    }
}

/// The voiced or half-voiced kana an unvoiced one becomes it (が to か, ぱ to は, ゔ to う), or cp
/// when it has neither form: the kana a cycle starts from.
fn kana_unvoice(cp: u32) -> u32 {
    if cp == 0x3094 {
        return 0x3046; // ゔ
    }
    if kana_small(cp + 1) == cp {
        return cp + 1; // っ to つ, ゃ to や, ぁ to あ
    }
    for back in [1u32, 2] {
        if cp > back {
            let b = cp - back;
            if kana_dakuten(b) == cp || kana_handakuten(b) == cp {
                return b;
            }
        }
    }
    cp
}

/// The small form of a kana that has one (あ to ぁ, つ to っ, や to ゃ, わ to ゎ); anything else
/// comes back as it is.
fn kana_small(cp: u32) -> u32 {
    match cp {
        0x3042 | 0x3044 | 0x3046 | 0x3048 | 0x304A // あ い う え お
        | 0x3064 // つ
        | 0x3084 | 0x3086 | 0x3088 // や ゆ よ
        | 0x308F => cp - 1, // わ
        _ => cp,
    }
}

/// The katakana of a hiragana, and anything else (ー, 、, Latin) as it is: the kana page in
/// katakana types the same rows as katakana.
fn kana_katakana(cp: u32) -> u32 {
    if (0x3041..=0x3096).contains(&cp) {
        cp + 0x60
    } else {
        cp
    }
}

/// The next form of a kana, for the pad's one modifier key: its unvoiced form, then voiced, then
/// half-voiced, then small, then round again (か が か; は ば ぱ は; つ づ っ つ; や ゃ や). ー and
/// the punctuation stay as they are; katakana cycles in its own script.
fn kana_cycle(cp: u32) -> u32 {
    let kata = (0x30A1..=0x30F6).contains(&cp);
    let h = if kata { cp - 0x60 } else { cp };
    let base = kana_unvoice(h);
    let mut forms = [base; 4];
    let mut n = 1;
    for f in [kana_dakuten(base), kana_handakuten(base), kana_small(base)] {
        if f != base && !forms[..n].contains(&f) {
            forms[n] = f;
            n += 1;
        }
    }
    let mut next = h;
    for i in 0..n {
        if forms[i] == h {
            next = forms[(i + 1) % n];
        }
    }
    if kata {
        next + 0x60
    } else {
        next
    }
}

/// Which way a finger was swiped from where it went down: 0 for none (a tap, or a wobble no bigger
/// than slop), 1 left, 2 up, 3 right, 4 down.
fn flick_dir(x0: f32, y0: f32, x: f32, y: f32) -> i32 {
    let (dx, dy) = (x - x0, y - y0);
    let slop = ui_px(40.0) as f32;
    if dx.abs().max(dy.abs()) < slop {
        return 0;
    }
    if dx.abs() >= dy.abs() {
        if dx < 0.0 { 1 } else { 3 }
    } else if dy < 0.0 {
        2
    } else {
        4
    }
}

/// Replaces the last character of the text with f(that character): the kana page's modifier takes
/// it to its next form. No character, or one with no such form, leaves the text as it is.
fn change_last(kb: &mut Keyboard, f: fn(u32) -> u32) {
    let b = kb.text.as_bytes();
    let mut start = b.len();
    while start > 0 && (b[start - 1] & 0xC0) == 0x80 {
        start -= 1;
    }
    if start == 0 {
        return;
    }
    let start = start - 1;
    let Some((_, cp)) = utf8_get(&b[start..]) else {
        return;
    };
    let ch = f(cp);
    if ch == cp {
        return;
    }
    let mut enc = [0u8; 4];
    let n = utf8_put(ch, &mut enc);
    if start + n < KB_TEXT_MAX {
        let mut buf = [0u8; KB_TEXT_MAX];
        buf[..start].copy_from_slice(&b[..start]);
        buf[start..start + n].copy_from_slice(&enc[..n]);
        kb.text.set_bytes(&buf[..start + n]);
    }
}

fn row_len(row: &[u16; 11]) -> usize {
    let mut n = 0;
    while n < 11 && row[n] != 0 {
        n += 1;
    }
    n
}

// ---- UTF-8 ----

// Encodes cp into buf (room for 4 bytes); returns the length.
fn utf8_put(cp: u32, buf: &mut [u8; 4]) -> usize {
    if cp < 0x80 {
        buf[0] = cp as u8;
        return 1;
    }
    if cp < 0x800 {
        buf[0] = (0xC0 | cp >> 6) as u8;
        buf[1] = (0x80 | (cp & 0x3F)) as u8;
        return 2;
    }
    if cp < 0x10000 {
        buf[0] = (0xE0 | cp >> 12) as u8;
        buf[1] = (0x80 | (cp >> 6 & 0x3F)) as u8;
        buf[2] = (0x80 | (cp & 0x3F)) as u8;
        return 3;
    }
    buf[0] = (0xF0 | cp >> 18) as u8;
    buf[1] = (0x80 | (cp >> 12 & 0x3F)) as u8;
    buf[2] = (0x80 | (cp >> 6 & 0x3F)) as u8;
    buf[3] = (0x80 | (cp & 0x3F)) as u8;
    4
}

// Length of the character at the start of s (1-4) with its code point, or None if the bytes there
// are not valid UTF-8: a stray or missing continuation byte, an overlong form, a surrogate, or a
// code point past U+10FFFF. (The end of s is the C string's terminator.)
fn utf8_get(s: &[u8]) -> Option<(usize, u32)> {
    let p = |i: usize| s.get(i).copied().unwrap_or(0);
    if p(0) < 0x80 {
        return Some((1, p(0) as u32));
    }
    let len: usize = if p(0) >= 0xF0 {
        4
    } else if p(0) >= 0xE0 {
        3
    } else if p(0) >= 0xC0 {
        2
    } else {
        0
    };
    if len == 0 || p(0) > 0xF4 {
        return None;
    }
    let mut c = p(0) as u32 & (0x7F >> len);
    for i in 1..len {
        if (p(i) & 0xC0) != 0x80 {
            return None; // also stops at the terminator
        }
        c = c << 6 | (p(i) as u32 & 0x3F);
    }
    const LEAST: [u32; 5] = [0, 0, 0x80, 0x800, 0x10000];
    if c < LEAST[len] || c > 0x10FFFF || (0xD800..=0xDFFF).contains(&c) {
        return None;
    }
    Some((len, c))
}

// Where the character after the one at off starts (off is not at the terminator).
fn utf8_next(s: &[u8], off: usize) -> usize {
    let mut off = off + 1;
    while off < s.len() && (s[off] & 0xC0) == 0x80 {
        off += 1;
    }
    off
}

// Characters (not bytes) in s.
fn utf8_count(s: &str) -> usize {
    s.bytes().filter(|&b| (b & 0xC0) != 0x80).count()
}

// Copies the valid characters of src into dst (cap bytes with the terminator), dropping control
// characters and broken UTF-8 and stopping before a character that would not fit.
fn copy_text<const N: usize>(dst: &mut FixedStr<N>, src: &[u8]) {
    let mut buf = [0u8; N];
    let mut n = 0usize;
    // (src is a C string: it ends at the first NUL)
    let mut s = &src[..src.iter().position(|&b| b == 0).unwrap_or(src.len())];
    while !s.is_empty() {
        let Some((len, cp)) = utf8_get(s) else {
            s = &s[1..]; // not UTF-8: drop the byte
            continue;
        };
        if cp >= 32 && !(0x7F..0xA0).contains(&cp) {
            if n + len >= N {
                break;
            }
            buf[n..n + len].copy_from_slice(&s[..len]);
            n += len;
        }
        s = &s[len..];
    }
    dst.set_bytes(&buf[..n]);
}

/// The capital shift types for a letter (Latin-1 and Latin Extended-A); anything without one
/// comes back as it is.
pub fn keyboard_upper(c: u32) -> u32 {
    if c >= 'a' as u32 && c <= 'z' as u32 {
        return c - 32;
    }
    if (0xE0..=0xFE).contains(&c) && c != 0xF7 {
        return c - 32; // Latin-1 (not the division sign)
    }
    if c == 0xFF {
        return 0x178; // y diaeresis
    }
    if c == 0x131 {
        return 'I' as u32; // dotless i
    }
    // Latin Extended-A pairs each capital with its small letter: the capital at the even code
    // point, except in two runs where it takes the odd one.
    if (0x100..=0x137).contains(&c) || (0x14A..=0x177).contains(&c) {
        return c & !1u32;
    }
    if (0x139..=0x148).contains(&c) || (0x179..=0x17E).contains(&c) {
        return if c & 1 != 0 { c } else { c - 1 };
    }
    c
}

// ---- layout ----

/// Where keyboard_draw puts the keyboard on this screen: along the bottom, 20 px in from the sides and above the
/// edge (a screen that lays out around it knows before it is drawn).
pub fn keyboard_modal_rect(screen: Rect) -> Rect {
    modal_rect(screen, 0)
}

/// keyboard_modal_rect for a keyboard docked with keyboard_set_dock: dock_y 0 is the bottom, as above;
/// otherwise the modal's top sits dock_y below the screen's top.
pub fn keyboard_modal_rect_docked(screen: Rect, dock_y: i32) -> Rect {
    modal_rect(screen, dock_y)
}

fn modal_rect(screen: Rect, dock_y: i32) -> Rect {
    modal_rect_page(screen, dock_y, 0)
}

// The count pad (page 4) has a row more than the others: its - and +.
fn modal_rect_page(screen: Rect, dock_y: i32, page: i32) -> Rect {
    let h = ui_px(170.0) + 5 * ui_px(150.0) + ui_px(60.0) + if page == 4 { ui_px(170.0) } else { 0 };
    let y = if dock_y > 0 { screen.y + dock_y } else { screen.y + screen.h - h - ui_px(20.0) };
    Rect { x: screen.x + ui_px(20.0), y, w: screen.w - ui_px(40.0), h }
}

/// Where keyboard_draw puts this keyboard on this screen (its page and dock: the count pad is a row taller),
/// for a caller that composes the screen under it before it is drawn.
pub fn keyboard_area(kb: &Keyboard, screen: Rect) -> Rect {
    modal_rect_page(screen, kb.dock_y, kb.page)
}

/// Docks an open keyboard: top_y 0 keeps it at the bottom; otherwise the modal's top is at top_y (a caller
/// passes the line below the clock corner, so the keyboard never covers it). keyboard_open resets the dock to
/// the bottom, so a caller that docks calls this after it.
pub fn keyboard_set_dock(kb: &mut Keyboard, top_y: i32) {
    kb.dock_y = top_y.max(0);
}

fn field_rect(area: Rect) -> Rect {
    Rect { x: area.x + ui_px(30.0), y: area.y + ui_px(30.0), w: area.w - ui_px(60.0), h: ui_px(120.0) }
}

/// The text line of a keyboard drawn in area: no key is there, so a contact on it presses nothing
/// (a hardware key's input taps it to reach the keyboard's owner, app_type_text).
pub fn keyboard_field_rect(area: Rect) -> Rect {
    field_rect(area)
}

/// What a hardware key does to an open on-screen keyboard (app_type_text): Esc is its Cancel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KbInput {
    Cancel,
}

// Backspace: a whole character, back over its continuation bytes to its first byte.
fn backspace(kb: &mut Keyboard) {
    let b = kb.text.as_bytes();
    let mut len = b.len();
    while len > 1 && (b[len - 1] & 0xC0) == 0x80 {
        len -= 1;
    }
    if len != 0 {
        let mut buf = [0u8; KB_TEXT_MAX];
        buf[..len - 1].copy_from_slice(&b[..len - 1]);
        kb.text.set_bytes(&buf[..len - 1]);
    }
}

/// A hardware key on an open on-screen keyboard: the result a tap on the same key would give (Cancel closes it;
/// the count pad's Cancel as keyboard_press does it), with *dirty as keyboard_pointer sets it.
pub fn keyboard_input(kb: &mut Keyboard, input: &KbInput, dirty: &mut Rect) -> KbResult {
    *dirty = RECT_EMPTY;
    if !kb.open {
        return KbResult::None;
    }
    kb.field_dirty = RECT_EMPTY;
    match input {
        KbInput::Cancel if kb.page == 4 => keyboard_press(kb, KB_KEY_CANCEL, dirty),
        KbInput::Cancel => {
            kb.open = false;
            *dirty = kb.area;
            KbResult::Cancel
        }
    }
}

/// The kana pad's page (Japanese): 4 is the count pad's (keyboard_open_count), so the kana pad is 5.
pub const KANA_PAGE: i32 = 5;

fn layout(area: Rect, page: i32, keys: &mut [Key; MAX_KEYS]) -> usize {
    if page == 2 || page == 4 {
        let mut n = 0;
        let gap = ui_px(16.0);
        let h = ui_px(150.0);
        let w = (area.w - ui_px(60.0) - 2 * gap) / 3;
        let x = area.x + ui_px(30.0);
        let mut y = area.y + ui_px(170.0);
        for i in 0..9 {
            keys[n] = Key { r: Rect { x: x + (i % 3) * (w + gap), y: y + (i / 3) * (h + gap), w, h }, code: '1' as u32 + i as u32 };
            n += 1;
        }
        y += 3 * (h + gap);
        keys[n] = Key { r: Rect { x, y, w, h }, code: KB_KEY_CANCEL };
        n += 1;
        keys[n] = Key { r: Rect { x: x + w + gap, y, w, h }, code: '0' as u32 };
        n += 1;
        keys[n] = Key { r: Rect { x: x + 2 * (w + gap), y, w, h }, code: KB_KEY_BKSP };
        n += 1;
        if page == 4 {
            // The count pad: - and + under the digits, each half the row.
            y += h + gap;
            let half = (area.w - ui_px(60.0) - gap) / 2;
            keys[n] = Key { r: Rect { x, y, w: half, h }, code: KB_KEY_DEC };
            n += 1;
            keys[n] = Key { r: Rect { x: x + half + gap, y, w: area.w - ui_px(60.0) - half - gap, h }, code: KB_KEY_INC };
            n += 1;
        }
        keys[n] = Key { r: Rect { x, y: y + h + gap, w: area.w - ui_px(60.0), h }, code: KB_KEY_DONE };
        n += 1;
        return n;
    }
    if page == KANA_PAGE {
        // The kana pad: three columns of four keys, あ か さ / た な は / ま や ら / わ ゛小 ー,
        // then cancel, カ, 、, abc, space, delete and done.
        let mut n = 0;
        let kh = ui_px(150.0);
        let gap = ui_px(10.0);
        let top = area.y + ui_px(170.0);
        let inner_x = area.x + ui_px(30.0);
        let inner_w = area.w - ui_px(60.0);
        let kw = (inner_w - 2 * gap) / 3;
        for i in 0..12 {
            let code = if i < K_KANA_MOD_CELL as i32 {
                KB_KEY_FLICK + i as u32
            } else if i as usize == K_KANA_MOD_CELL {
                KB_KEY_KANA_MOD
            } else {
                KANA_CHOON
            };
            keys[n] = Key { r: Rect { x: inner_x + (i % 3) * (kw + gap), y: top + (i / 3) * (kh + gap), w: kw, h: kh }, code };
            n += 1;
        }
        // (the bottom row's keys are the letters page's sizes, one tenth of the width each, as
        // the grid's three columns are far wider)
        let sk = (inner_w - 9 * gap) / 10;
        let wide = sk + sk / 2;
        let y = top + 4 * (kh + gap);
        keys[n] = Key { r: Rect { x: inner_x, y, w: wide, h: kh }, code: KB_KEY_CANCEL };
        n += 1;
        let mut x = inner_x + wide + gap;
        for code in [KB_KEY_KATA, KB_KEY_PUNCT, KB_KEY_PAGE] {
            keys[n] = Key { r: Rect { x, y, w: sk, h: kh }, code };
            n += 1;
            x += sk + gap;
        }
        let done_x = inner_x + inner_w - wide;
        let bx = done_x - sk - gap;
        keys[n] = Key { r: Rect { x, y, w: bx - gap - x, h: kh }, code: KEY_SPACE };
        n += 1;
        keys[n] = Key { r: Rect { x: bx, y, w: sk, h: kh }, code: KB_KEY_BKSP };
        n += 1;
        keys[n] = Key { r: Rect { x: done_x, y, w: wide, h: kh }, code: KB_KEY_DONE };
        n += 1;
        return n;
    }
    let rows = &K_ROWS[match page {
        0 => 0,
        3 => 2,
        _ => 1,
    }];
    let mut n = 0;
    let kh = ui_px(150.0);
    let gap = ui_px(10.0);
    let top = area.y + ui_px(170.0);
    let inner_x = area.x + ui_px(30.0);
    let inner_w = area.w - ui_px(60.0);
    let kw = (inner_w - 9 * gap) / 10;
    let enye = page == 0 && i18n_current() == "es";
    for r in 0..3 {
        let row = if enye && r == 2 { &K_ROW_ES } else { &rows[r as usize] };
        let cnt = row_len(row) as i32;
        let rowx = inner_x + (inner_w - (cnt * kw + (cnt - 1) * gap)) / 2;
        for i in 0..cnt {
            keys[n] = Key { r: Rect { x: rowx + i * (kw + gap), y: top + r * (kh + gap), w: kw, h: kh }, code: row[i as usize] as u32 };
            n += 1;
        }
    }
    // Row 4: shift, up to seven keys (centered between), backspace.
    let mut y = top + 3 * (kh + gap);
    let wide = kw + kw / 2;
    keys[n] = Key { r: Rect { x: inner_x, y, w: wide, h: kh }, code: KB_KEY_SHIFT };
    n += 1;
    let zx = inner_x + wide + gap;
    let bx = zx + 7 * (kw + gap);
    let cnt = row_len(&rows[3]) as i32;
    let rowx = zx + (bx - gap - zx - (cnt * kw + (cnt - 1) * gap)) / 2;
    for i in 0..cnt {
        keys[n] = Key { r: Rect { x: rowx + i * (kw + gap), y, w: kw, h: kh }, code: rows[3][i as usize] as u32 };
        n += 1;
    }
    keys[n] = Key { r: Rect { x: bx, y, w: inner_x + inner_w - bx, h: kh }, code: KB_KEY_BKSP };
    n += 1;
    // Row 5: cancel (as wide as shift), -, the symbols page, the accents page, space, ', ., done (as
    // wide as shift).
    y += kh + gap;
    keys[n] = Key { r: Rect { x: inner_x, y, w: wide, h: kh }, code: KB_KEY_CANCEL };
    n += 1;
    let mut x = inner_x + wide + gap;
    // In Japanese the accents key is the kana key (the accents page has nothing a Japanese name
    // needs); every other language keeps áé there.
    let page_key = if page == 0 && i18n_current() == "ja" { KB_KEY_KANA } else { KB_KEY_PAGE };
    for code in ['-' as u32, KB_KEY_SYMBOLS, page_key] {
        keys[n] = Key { r: Rect { x, y, w: kw, h: kh }, code };
        n += 1;
        x += kw + gap;
    }
    let done_x = inner_x + inner_w - wide;
    let ax = done_x - 2 * (kw + gap);
    keys[n] = Key { r: Rect { x, y, w: ax - gap - x, h: kh }, code: KEY_SPACE };
    n += 1;
    keys[n] = Key { r: Rect { x: ax, y, w: kw, h: kh }, code: '\'' as u32 };
    n += 1;
    keys[n] = Key { r: Rect { x: ax + kw + gap, y, w: kw, h: kh }, code: '.' as u32 };
    n += 1;
    keys[n] = Key { r: Rect { x: done_x, y, w: wide, h: kh }, code: KB_KEY_DONE };
    n += 1;
    n
}

/// Starts on the letters page with initial as the text: every valid character is kept (accents
/// included); control characters and broken UTF-8 are dropped, and the text is cut, if it must
/// be, between characters.
pub fn keyboard_open(kb: &mut Keyboard, title: &str, initial: &[u8]) {
    *kb = Keyboard::default();
    kb.open = true;
    kb.pressed_key = -1;
    kb.flick = 0;
    copy_text(&mut kb.title, title.as_bytes());
    copy_text(&mut kb.text, initial);
}

/// The count pad (page 4): the number pad's digits, Cancel and del, then a row with - and + (the count
/// one less, down to 1, or one more), and Done, whose label `label` gives for the count (disabled at 0).
/// The text is the count in digits (empty for 0); a digit or + never takes it past `max`: it stays at
/// `max`, which the pad shows (no toast: the owner, 2026-11-04). It opens at 1 (`count_fresh`, drawn plain
/// with no caret), so Done (or Enter) at once adds one; the first digit typed replaces that 1, del
/// clears it, and - and + count from it.
pub fn keyboard_open_count(kb: &mut Keyboard, title: &str, max: i32, label: fn(i32) -> String) {
    let max = max.max(0);
    keyboard_open(kb, title, if max >= 1 { b"1" } else { b"" });
    kb.page = 4;
    kb.count_max = max;
    kb.count_label = Some(label);
    kb.count_fresh = max >= 1;
}

/// The count pad's count: its digits as a number (0 for none).
pub fn keyboard_count(kb: &Keyboard) -> i32 {
    kb.text.as_str().bytes().filter(u8::is_ascii_digit).fold(0i32, |v, d| v.saturating_mul(10).saturating_add((d - b'0') as i32))
}

fn set_count(kb: &mut Keyboard, n: i32) {
    let n = if n > kb.count_max { kb.count_max } else { n.max(0) };
    let digits = if n > 0 { n.to_string() } else { String::new() };
    kb.text.set_bytes(digits.as_bytes());
}

pub fn keyboard_close(kb: &mut Keyboard) {
    kb.open = false;
}

// What a key shows: a name for the named keys, else its character as it would type now (in
// ch, room for 5 bytes).
fn key_label<'a>(kb: &Keyboard, code: u32, ch: &'a mut [u8; 8]) -> &'a str {
    match code {
        // TRANSLATORS: the keyboard's named keys, in bold at 44 px: shift (250 px wide), delete
        // (261), the space bar (355), and Cancel and Done (250).
        KB_KEY_SHIFT => c_("key", "shift"),
        KB_KEY_BKSP => c_("key", "del"),
        KB_KEY_DONE => tr("Done"),
        KB_KEY_DEC => "\u{2212}", // minus sign
        KB_KEY_INC => "+",
        KB_KEY_CANCEL => tr("Cancel"),
        // TRANSLATORS: the kana page's named keys, in bold at 44 px: ゛小 (one tenth of the
        // modal wide), カ, 、, the space bar, and あ, the letters page's way to the kana page (as
        // wide as shift).
        KB_KEY_KANA => "\u{3042}",              // あ
        KB_KEY_KANA_MOD => "\u{309B}\u{5C0F}",  // ゛小: the kana typed last, in its next form
        KB_KEY_KATA => {
            if kb.kata {
                "\u{3042}" // あ: back to hiragana
            } else {
                "\u{30AB}" // カ: katakana
            }
        }
        // The page keys: each to its page, and on it back to the letters.
        KB_KEY_PAGE => {
            if kb.page == 0 {
                "\u{e1}\u{e9}" // a and e acute
            } else {
                "abc"
            }
        }
        KB_KEY_SYMBOLS => {
            if kb.page == 3 {
                "abc"
            } else {
                "#+="
            }
        }
        KEY_SPACE => c_("key", "space"),
        _ => {
            // A kana key shows the kana the swipe would type (its first while the finger is still
            // on it).
            let code = if is_flick(code) {
                let row = &K_KANA[(code - KB_KEY_FLICK) as usize];
                let at = if kb.flick > 0 { kb.flick as usize } else { 0 };
                let kana = if row[at] != 0 { row[at] as u32 } else { row[0] as u32 };
                if kb.kata {
                    kana_katakana(kana)
                } else {
                    kana
                }
            } else {
                code
            };
            let mut enc = [0u8; 4];
            let n = utf8_put(if kb.shift != KbShift::Off { keyboard_upper(code) } else { code }, &mut enc);
            ch[..n].copy_from_slice(&enc[..n]);
            ch[n] = 0;
            std::str::from_utf8(&ch[..n]).unwrap_or("")
        }
    }
}

thread_local! {
    /// keyboard_draws_take
    static G_DRAWS: Cell<u32> = const { Cell::new(0) };
}

/// How many times keyboard_draw drew a keyboard since this was last called (the headless runner's
/// keyboard.draws check, docs/SIM.md): every draw of the whole modal is work on the main thread, and a
/// key press should cost one.
pub fn keyboard_draws_take() -> u32 {
    G_DRAWS.with(|g| g.replace(0))
}

/// Draws the modal (text field at top, keys below) anchored to the bottom of screen, or at its dock
/// (keyboard_set_dock).
/// Returns the rect it covered (to invalidate).
pub fn keyboard_draw(kb: &mut Keyboard, c: &mut Canvas, screen: Rect) -> Rect {
    G_DRAWS.with(|g| g.set(g.get().wrapping_add(1)));
    let area = modal_rect_page(screen, kb.dock_y, kb.page);
    kb.area = area;
    ui_frame(c, area, ui_px(24.0), 255, true, 0);
    ui_frame(c, area, ui_px(24.0), 0, false, ui_px(6.0));
    let f = field_rect(area);
    ui_frame(c, f, ui_px(12.0), 255, true, 0);
    ui_frame(c, f, ui_px(12.0), 0, false, ui_px(3.0));
    let ts = ui_text_size();
    let title = kb.title.as_str();
    let tw = if !title.is_empty() { font_measure(FontFace::Bold, ts, title) + ui_px(30.0) } else { 0 };
    if tw != 0 {
        font_draw_left(c, FontFace::Bold, ts, Rect { x: f.x + ui_px(24.0), y: f.y, w: tw, h: f.h }, title, 0);
    }
    // Show the tail of long text so the caret stays visible.
    let tr = Rect { x: f.x + ui_px(24.0) + tw, y: f.y, w: f.w - ui_px(48.0) - tw, h: f.h };
    let text = kb.text.as_str();
    // (the count pad's count in bold, at the size of the title beside it: at ui_title_size it was large and
    // intrusive, the owner's Scribe feedback of 2026-11-04)
    let (face, size) = if kb.page == 4 { (FontFace::Bold, ts) } else { (FontFace::Regular, ts * 1.1) };
    let mut at = 0;
    while at < text.len() && font_measure(face, size, &text[at..]) > tr.w - ui_px(20.0) {
        at = utf8_next(text.as_bytes(), at);
    }
    let shown = &text[at..];
    let w = font_measure(face, size, shown);
    font_draw_left(c, face, size, tr, shown, 0);
    // The count pad's opening 1 (count_fresh) is plain black text like any count, with no caret after it: the
    // first digit typed replaces it rather than following it. (It was a white 1 in a black box, as selected
    // text is drawn, which read as a 0 on the Scribe: the owner, 2026-11-04, "Batch page adding starts the
    // counter at 0 - and is displayed at a large, intrusive amount".)
    if !(kb.page == 4 && kb.count_fresh) {
        canvas_fill(c, Rect { x: tr.x + w + ui_px(4.0), y: f.y + ui_px(24.0), w: ui_px(5.0), h: f.h - ui_px(48.0) }, 0);
    }
    let mut keys = [Key::default(); MAX_KEYS];
    let n = layout(area, kb.page, &mut keys);
    for (i, key) in keys.iter().enumerate().take(n) {
        let mut ch = [0u8; 8];
        let counted;
        let lab = match kb.count_label {
            Some(f) if kb.page == 4 && key.code == KB_KEY_DONE => {
                counted = f(keyboard_count(kb));
                counted.as_str()
            }
            _ => key_label(kb, key.code, &mut ch),
        };
        if count_done_off(kb, key.code) {
            ui_button(c, key.r, IconKind::None, lab, ButtonStyle::Disabled);
            continue;
        }
        let st = if i as i32 == kb.pressed_key || (key.code == KB_KEY_SHIFT && kb.shift != KbShift::Off) {
            ButtonStyle::Selected
        } else if key.code == KB_KEY_DONE {
            ButtonStyle::Primary
        } else {
            ButtonStyle::Plain
        };
        ui_frame(c, key.r, ui_px(12.0), if st == ButtonStyle::Selected { 0 } else { 255 }, true, 0);
        ui_frame(c, key.r, ui_px(12.0), 0, false, if st == ButtonStyle::Primary { ui_px(6.0) } else { ui_px(3.0) });
        let ks = if utf8_count(lab) > 1 { ui_text_size() } else { ui_title_size() };
        font_draw_centered(c, FontFace::Bold, ks, key.r, lab, if st == ButtonStyle::Selected { 255 } else { 0 });
        if key.code == KB_KEY_SHIFT && kb.shift == KbShift::Caps {
            caps_bar(c, key.r, ks, lab);
        }
    }
    area
}

// Caps on's cue: a white bar under the shift key's white label on the pressed key's black, as wide as the
// label. Shift for the next letter is the pressed key alone, so the two read apart at a glance, in both
// themes and sideways, with no word of their own to translate.
fn caps_bar(c: &mut Canvas, r: Rect, size: f32, lab: &str) {
    let w = font_measure(FontFace::Bold, size, lab).min(r.w - ui_px(40.0)).max(1);
    let (asc, desc) = (font_ascent(FontFace::Bold, size), font_descent(FontFace::Bold, size));
    // (the label's bottom, where font_draw_centered sets it)
    let bottom = r.y + (r.h - (asc + desc)) / 2 + asc + desc;
    let h = ui_px(12.0);
    let y = (bottom + ui_px(6.0)).min(r.y + r.h - ui_px(12.0) - h);
    canvas_fill(c, Rect { x: r.x + (r.w - w) / 2, y, w, h }, 255);
}

// The count pad's Done at a count of 0: drawn disabled, and it does nothing.
fn count_done_off(kb: &Keyboard, code: u32) -> bool {
    kb.page == 4 && code == KB_KEY_DONE && keyboard_count(kb) < 1
}

fn key_at(kb: &Keyboard, x: f32, y: f32) -> i32 {
    let mut keys = [Key::default(); MAX_KEYS];
    let n = layout(kb.area, kb.page, &mut keys);
    for (i, key) in keys.iter().enumerate().take(n) {
        if rect_contains(rect_inflate(key.r, ui_px(5.0)), x as i32, y as i32) {
            return i as i32;
        }
    }
    -1
}

// Appends the key's character (its capital while shift is on) if it fits. Shift for the next letter lets go
// once it gave a capital (every letter key's label changes back: the keyboard's refresh); caps on stays.
fn type_key(kb: &mut Keyboard, code: u32, dirty: &mut Rect) {
    let ch = if kb.shift != KbShift::Off { keyboard_upper(code) } else { code };
    let mut enc = [0u8; 4];
    let len = kb.text.len();
    let n = utf8_put(ch, &mut enc);
    if len + n < KB_TEXT_MAX {
        // a character that does not fit is not typed
        let mut buf = [0u8; KB_TEXT_MAX];
        buf[..len].copy_from_slice(kb.text.as_bytes());
        buf[len..len + n].copy_from_slice(&enc[..n]);
        kb.text.set_bytes(&buf[..len + n]);
    }
    if kb.shift == KbShift::Once && ch != code {
        kb.shift = KbShift::Off;
        *dirty = kb.area;
    }
}

// Whether shift shows on the page's other keys: a key whose character has a capital shows it while shift is
// on (key_label). The letters and accents pages have them; the symbols page has none (its digits and signs
// have no capitals), so a shift tap there changes the shift key alone. (The named keys' codes are below 32,
// the kana pad's flick keys among them; neither changes with shift.)
fn shift_shows_on_keys(kb: &Keyboard) -> bool {
    let mut keys = [Key::default(); MAX_KEYS];
    let n = layout(kb.area, kb.page, &mut keys);
    keys.iter().take(n).any(|k| k.code >= 32 && keyboard_upper(k.code) != k.code)
}

/// Where the key with this code is on the page showing now (as last drawn); false if that page
/// has no such key. For tests and scripted runs.
pub fn keyboard_key_rect(kb: &Keyboard, code: u32, r: &mut Rect) -> bool {
    let mut keys = [Key::default(); MAX_KEYS];
    let n = layout(kb.area, kb.page, &mut keys);
    for key in keys.iter().take(n) {
        if key.code == code {
            *r = key.r;
            return true;
        }
    }
    false
}

/// Pointer events only (pen or touch, already unified by the caller as down/move/up).
/// *dirty receives the rect needing redraw (may be empty). Characters and backspace type on
/// touch-down (no waiting for the lift); Shift, the page keys, Cancel and Done act on release.
/// phase: 0 down, 1 move, 2 up.
pub fn keyboard_pointer(kb: &mut Keyboard, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> KbResult {
    *dirty = RECT_EMPTY;
    if !kb.open {
        return KbResult::None;
    }
    kb.field_dirty = RECT_EMPTY;
    kb.also_dirty = RECT_EMPTY;
    let mut keys = [Key::default(); MAX_KEYS];
    let n = layout(kb.area, kb.page, &mut keys) as i32;
    if phase == 0 {
        press_begin(&mut kb.press, x, y, t);
        kb.pressed_key = key_at(kb, x, y);
        kb.typed = false;
        if kb.pressed_key < 0 {
            return KbResult::None;
        }
        *dirty = keys[kb.pressed_key as usize].r;
        kb.flick = 0;
        let c = keys[kb.pressed_key as usize].code;
        // (a kana key waits for the lift: which kana it types is the swipe, and the modifier and
        // カ act on the character before them, or on the page)
        if c == KB_KEY_SHIFT
            || c == KB_KEY_PAGE
            || c == KB_KEY_SYMBOLS
            || c == KB_KEY_KANA
            || c == KB_KEY_DONE
            || c == KB_KEY_CANCEL
            || c == KB_KEY_KANA_MOD
            || c == KB_KEY_KATA
            || is_flick(c)
        {
            return KbResult::None;
        }
        kb.typed = true; // type now: the lift only releases the key
    } else if phase == 1 {
        press_move(&mut kb.press, x, y);
        // A swipe on a kana key shows the kana it would type there, once for each way it turns.
        let k = kb.pressed_key;
        if k >= 0 && k < n && is_flick(keys[k as usize].code) {
            let dir = flick_dir(kb.press.x0, kb.press.y0, x, y);
            if dir != kb.flick {
                kb.flick = dir;
                *dirty = keys[k as usize].r;
            }
        }
        return KbResult::None;
    } else {
        let k = kb.pressed_key;
        kb.pressed_key = -1;
        if k < 0 || k >= n {
            kb.flick = 0;
            return KbResult::None;
        }
        if is_flick(keys[k as usize].code) {
            // The kana the swipe picked: the pen may end well off the key by the lift.
            let row = &K_KANA[(keys[k as usize].code - KB_KEY_FLICK) as usize];
            let at = if kb.flick > 0 { kb.flick as usize } else { 0 };
            let kana = if row[at] != 0 { row[at] as u32 } else { row[0] as u32 };
            let kana = if kb.kata { kana_katakana(kana) } else { kana };
            kb.flick = 0;
            *dirty = keys[k as usize].r;
            type_key(kb, kana, dirty);
            kb.field_dirty = field_rect(kb.area);
            return KbResult::Changed;
        }
        *dirty = keys[k as usize].r;
        if kb.typed {
            return KbResult::None;
        }
        if key_at(kb, x, y) != k {
            return KbResult::None; // slid off the key: no input
        }
    }
    let k = if kb.pressed_key >= 0 { kb.pressed_key } else { key_at(kb, x, y) };
    if k < 0 || k >= n {
        return KbResult::None;
    }
    keyboard_press(kb, keys[k as usize].code, dirty)
}

/// What a key of the page showing does, as keyboard_pointer does it for a tap on that key (a keyboard's
/// key typed, or a test's): `code` a character or a KB_KEY_* code. *dirty and field_dirty as there; on
/// the count pad also_dirty is its Done when the count changed.
pub fn keyboard_press(kb: &mut Keyboard, code: u32, dirty: &mut Rect) -> KbResult {
    kb.field_dirty = RECT_EMPTY;
    kb.also_dirty = RECT_EMPTY;
    if !kb.open || count_done_off(kb, code) {
        return KbResult::None;
    }
    if kb.page == 4 && code != KB_KEY_CANCEL && code != KB_KEY_DONE {
        // The count pad: digits, del, - and + change the count; it stays a number from 0 to count_max. While
        // the count is the 1 it opened with (count_fresh), a digit replaces it and del clears it.
        let n = keyboard_count(kb);
        let fresh = kb.count_fresh;
        let to = match code {
            KB_KEY_DEC => if n > 1 { n - 1 } else { n },
            KB_KEY_INC => n.saturating_add(1),
            KB_KEY_BKSP if fresh => 0,
            KB_KEY_BKSP => n / 10,
            c if (b'0' as u32..=b'9' as u32).contains(&c) && fresh => (c - b'0' as u32) as i32,
            c if (b'0' as u32..=b'9' as u32).contains(&c) => n.saturating_mul(10).saturating_add((c - b'0' as u32) as i32),
            _ => return KbResult::None,
        };
        kb.count_fresh = false;
        set_count(kb, to);
        if keyboard_count(kb) == n && !fresh {
            return KbResult::None; // (as it was: nothing to draw)
        }
        let mut r = RECT_EMPTY;
        if keyboard_key_rect(kb, KB_KEY_DONE, &mut r) {
            kb.also_dirty = r;
        }
        kb.field_dirty = field_rect(kb.area);
        return KbResult::Changed;
    }
    match code {
        KB_KEY_CANCEL => {
            kb.open = false;
            *dirty = kb.area;
            return KbResult::Cancel;
        }
        KB_KEY_DONE => {
            kb.open = false;
            *dirty = kb.area;
            return KbResult::Done;
        }
        KB_KEY_SHIFT => {
            // Off, the next letter, caps on, off again (KbShift). Off to the next letter and caps on to off
            // change every letter key's label (capitals while shift is on): the keyboard's refresh, as
            // before. The next letter to caps on changes the shift key alone (its bar): its own refresh, and
            // so does every tap on a page with no letter keys (the symbols page: no label there changes case).
            kb.shift = match kb.shift {
                KbShift::Off => KbShift::Once,
                KbShift::Once => KbShift::Caps,
                KbShift::Caps => KbShift::Off,
            };
            let only_key = kb.shift == KbShift::Caps || !shift_shows_on_keys(kb);
            let mut r = RECT_EMPTY;
            *dirty = if only_key && keyboard_key_rect(kb, KB_KEY_SHIFT, &mut r) { r } else { kb.area };
            return KbResult::Changed;
        }
        KB_KEY_PAGE | KB_KEY_SYMBOLS | KB_KEY_KANA => {
            // abc is the way back from the accents and kana pages; áé goes to the accents page
            // (the symbols page's page key goes there too).
            kb.page = match code {
                KB_KEY_PAGE => {
                    if kb.page == 1 || kb.page == KANA_PAGE { 0 } else { 1 }
                }
                KB_KEY_KANA => {
                    if kb.page == KANA_PAGE { 0 } else { KANA_PAGE }
                }
                _ => {
                    if kb.page == 3 { 0 } else { 3 }
                }
            };
            *dirty = kb.area;
            return KbResult::Changed;
        }
        KB_KEY_KANA_MOD => change_last(kb, kana_cycle),
        KB_KEY_KATA => {
            kb.kata = !kb.kata;
            *dirty = kb.area;
            return KbResult::Changed;
        }
        KB_KEY_BKSP => backspace(kb),
        code => type_key(kb, code, dirty),
    }
    kb.field_dirty = field_rect(kb.area);
    KbResult::Changed
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every named key's code is its own: the kana pad's ゛小 and the count pad's minus once both took 5,
    // so the kana key showed a minus sign.
    #[test]
    fn named_key_codes_are_distinct() {
        let src = include_str!("keyboard.rs");
        let mut seen: Vec<(String, String)> = Vec::new();
        for line in src.lines() {
            let Some(rest) = line.strip_prefix("pub const KB_KEY_") else { continue };
            let Some((name, value)) = rest.split_once(": u32 = ") else { continue };
            let v = value.trim_end_matches(';').trim().to_string();
            if let Some((_, other)) = seen.iter().find(|(w, _)| *w == v) {
                panic!("KB_KEY_{name} and KB_KEY_{other} are both {v}");
            }
            seen.push((v, name.to_string()));
        }
        let flick: Vec<u32> = (KB_KEY_FLICK..KB_KEY_FLICK + 11).collect();
        for c in [KB_KEY_SHIFT, KB_KEY_PAGE, KB_KEY_SYMBOLS, KB_KEY_KANA, KB_KEY_KANA_MOD, KB_KEY_KATA, KB_KEY_BKSP, KB_KEY_DONE, KB_KEY_CANCEL, KB_KEY_DEC, KB_KEY_INC] {
            assert!(!flick.contains(&c), "a named key inside the flick keys: {c}");
            assert!(c < 32, "a named key code that a typed character can take: {c}");
        }
    }

    fn opened(initial: &[u8]) -> Keyboard {
        let mut kb = Keyboard::default();
        keyboard_open(&mut kb, "Ti\u{1}tle \u{e9}", initial);
        kb
    }

    #[test]
    fn shift_gives_the_capital() {
        assert_eq!(keyboard_upper('a' as u32), 'A' as u32);
        assert_eq!(keyboard_upper('z' as u32), 'Z' as u32);
        assert_eq!(keyboard_upper(0xE9), 0xC9); // e acute
        assert_eq!(keyboard_upper(0xF7), 0xF7); // the division sign has none
        assert_eq!(keyboard_upper(0xFF), 0x178); // y diaeresis
        assert_eq!(keyboard_upper(0x131), 'I' as u32); // dotless i
        assert_eq!(keyboard_upper(0x101), 0x100); // Latin Extended-A: the even code point is the capital
        assert_eq!(keyboard_upper(0x13A), 0x139); // ... except in two runs, where it is the odd one
        assert_eq!(keyboard_upper(0x153), 0x152); // oe
        assert_eq!(keyboard_upper(0x149), 0x149);
        assert_eq!(keyboard_upper('1' as u32), '1' as u32);
        assert_eq!(keyboard_upper(0x20AC), 0x20AC);
    }

    #[test]
    fn opening_keeps_valid_characters_only() {
        let kb = opened(b"Hi");
        assert!(kb.open && kb.pressed_key == -1 && kb.page == 0 && kb.shift == KbShift::Off);
        assert_eq!((kb.title.as_str(), kb.text.as_str()), ("Title \u{e9}", "Hi"));
        assert_eq!(opened("caf\u{e9} \u{20ac} \u{1f600}".as_bytes()).text.as_str(), "caf\u{e9} \u{20ac} \u{1f600}");
        // Control characters (C0, DEL, C1) and broken UTF-8 are dropped; the no-break space stays.
        assert_eq!(opened(b"a\x01b\x7fc\xc2\x80d\xc2\xa0e").text.as_str(), "abcd\u{a0}e");
        assert_eq!(opened(b"bad\xffx\xc3y\xe0\x80\x80z").text.as_str(), "badxyz");
        assert_eq!(opened(b"\xed\xa0\x80 surrogate").text.as_str(), " surrogate");
        assert_eq!(opened(b"\xf4\x90\x80\x80 big").text.as_str(), " big");
        assert_eq!(opened(b"\xc0\xaf overlong").text.as_str(), " overlong");
        assert_eq!(opened(b"\xc3").text.as_str(), "");
        assert_eq!(opened(b"x\xf0\x9f\x98").text.as_str(), "x");
        assert_eq!(opened(b"ab\0cd").text.as_str(), "ab"); // (a C string ends at its NUL)
    }

    #[test]
    fn opening_cuts_between_characters() {
        let digits: String = (0..120).map(|i| char::from(b'0' + (i % 10) as u8)).collect();
        assert_eq!(opened(digits.as_bytes()).text.len(), KB_TEXT_MAX - 1);
        // A character that would not fit ends the text, even if a later one would.
        let mut two = digits[..90].to_string();
        two.push_str("\u{e9}\u{e9}"); // 90 + 2 + 2 = 94 bytes: fits
        assert_eq!(opened(two.as_bytes()).text.as_str(), two);
        let mut tight = digits[..94].to_string();
        tight.push('\u{20ac}'); // 94 + 3 = 97 > 95
        tight.push('Z');
        assert_eq!(opened(tight.as_bytes()).text.as_str(), &digits[..94]);
        let mut title = Keyboard::default();
        keyboard_open(&mut title, &"T".repeat(199), b"");
        assert_eq!(title.title.len(), 63);
    }

    #[test]
    fn closing_and_counting_draws() {
        let mut kb = opened(b"x");
        keyboard_close(&mut kb);
        assert!(!kb.open);
        // A closed keyboard takes no pointer events.
        let mut d = Rect { x: 1, y: 2, w: 3, h: 4 };
        assert_eq!(keyboard_pointer(&mut kb, 0, 10.0, 10.0, 0, &mut d), KbResult::None);
        assert_eq!(d, RECT_EMPTY);
        assert_eq!(keyboard_draws_take(), 0);
    }

    #[test]
    fn a_hardware_key_cancels_an_open_keyboard() {
        let mut d = RECT_EMPTY;
        let mut kb = opened(b"ab");
        kb.area = Rect { x: 0, y: 1000, w: 1800, h: 1200 };
        assert_eq!(keyboard_input(&mut kb, &KbInput::Cancel, &mut d), KbResult::Cancel);
        assert!(!kb.open && d == kb.area);
        assert_eq!(keyboard_input(&mut kb, &KbInput::Cancel, &mut d), KbResult::None);
    }

    #[test]
    fn utf8_helpers() {
        assert_eq!(utf8_get(b"a"), Some((1, 0x61)));
        assert_eq!(utf8_get("\u{20ac}".as_bytes()), Some((3, 0x20AC)));
        assert_eq!(utf8_get(b"\xc3"), None); // cut short by the terminator
        assert_eq!(utf8_get(b"\x80"), None);
        assert_eq!(utf8_get(b"\xf5\x80\x80\x80"), None);
        let mut buf = [0u8; 4];
        assert_eq!(utf8_put(0x1F600, &mut buf), 4);
        assert_eq!(&buf, "\u{1f600}".as_bytes());
        assert_eq!(utf8_next("a\u{e9}b".as_bytes(), 1), 3);
        assert_eq!(utf8_count("a\u{e9}\u{20ac}"), 3);
    }

    fn label(n: i32) -> String {
        format!("Add {n}")
    }

    /// New page's count pad (2.0.0-beta.2): digits, del, - and + keep a count from 0 to its most, a count past
    /// it stays at the most and says so, - stops at 1, and Done does nothing at 0.
    #[test]
    fn the_count_pad_keeps_its_count_in_bounds() {
        let mut kb = Keyboard::default();
        keyboard_open_count(&mut kb, "Pages to add", 25, label);
        let mut c = Canvas::default();
        canvas_init(&mut c, 600, 1600);
        keyboard_draw(&mut kb, &mut c, Rect { x: 0, y: 0, w: 600, h: 1600 });
        let mut d = RECT_EMPTY;
        let mut press = |kb: &mut Keyboard, code: u32| keyboard_press(kb, code, &mut d);
        // It opens at 1 (count_fresh); del clears that 1 (the count 0: Done off).
        assert_eq!((kb.page, keyboard_count(&kb), kb.count_fresh), (4, 1, true));
        assert_eq!(press(&mut kb, KB_KEY_BKSP), KbResult::Changed);
        assert_eq!((keyboard_count(&kb), kb.count_fresh), (0, false));
        assert_eq!(press(&mut kb, KB_KEY_DONE), KbResult::None, "Done is off at 0");
        assert!(kb.open);
        assert_eq!(press(&mut kb, KB_KEY_DEC), KbResult::None);
        assert_eq!(press(&mut kb, '0' as u32), KbResult::None, "no leading zero");
        assert_eq!(press(&mut kb, '1' as u32), KbResult::Changed);
        assert!(!rect_is_empty(kb.field_dirty) && !rect_is_empty(kb.also_dirty), "the count line and Done");
        assert_eq!(press(&mut kb, '2' as u32), KbResult::Changed);
        assert_eq!(kb.text.as_str(), "12");
        assert_eq!(press(&mut kb, '0' as u32), KbResult::Changed);
        assert_eq!(kb.text.as_str(), "25", "120 is past 25: 25");
        assert_eq!(press(&mut kb, KB_KEY_INC), KbResult::None);
        assert_eq!(kb.text.as_str(), "25", "+ at 25 stays at 25");
        assert_eq!(press(&mut kb, KB_KEY_BKSP), KbResult::Changed);
        assert_eq!(kb.text.as_str(), "2");
        assert_eq!(press(&mut kb, KB_KEY_DEC), KbResult::Changed);
        assert_eq!(press(&mut kb, KB_KEY_DEC), KbResult::None, "- stops at 1");
        assert_eq!(kb.text.as_str(), "1");
        assert_eq!(press(&mut kb, KB_KEY_INC), KbResult::Changed);
        assert_eq!(press(&mut kb, KB_KEY_DONE), KbResult::Done);
        assert_eq!((keyboard_count(&kb), kb.open), (2, false));
        // The pad has a row more than the number pad, and its Done is under - and +.
        let (mut done, mut dec) = (RECT_EMPTY, RECT_EMPTY);
        assert!(keyboard_key_rect(&kb, KB_KEY_DONE, &mut done) && keyboard_key_rect(&kb, KB_KEY_DEC, &mut dec));
        assert!(done.y > dec.y + dec.h);
        assert!(kb.area.y + kb.area.h <= 1600 && done.y + done.h <= kb.area.y + kb.area.h);
    }

    /// The count pad opens at 1 (the owner: "if enter is pressed or confirm via touch, 1 will be the default"): Done at once is 1; the first digit replaces the 1, the next ones follow it; - and +
    /// count from it; a typed 1 is still a change to draw (its caret shows). A notebook with no
    /// room opens it empty.
    #[test]
    fn the_count_pad_opens_at_one_and_the_first_digit_replaces_it() {
        let mut d = RECT_EMPTY;
        let mut kb = Keyboard::default();
        keyboard_open_count(&mut kb, "Pages to add", 25, label);
        assert_eq!((kb.text.as_str(), kb.count_fresh), ("1", true));
        assert_eq!(keyboard_press(&mut kb, KB_KEY_DONE, &mut d), KbResult::Done);
        assert_eq!(keyboard_count(&kb), 1, "Done at once adds one");
        keyboard_open_count(&mut kb, "Pages to add", 25, label);
        assert_eq!(keyboard_press(&mut kb, '7' as u32, &mut d), KbResult::Changed);
        assert_eq!((kb.text.as_str(), kb.count_fresh), ("7", false), "7, not 17");
        assert_eq!(keyboard_press(&mut kb, '2' as u32, &mut d), KbResult::Changed);
        assert_eq!(kb.text.as_str(), "25", "72 is past 25: 25");
        keyboard_open_count(&mut kb, "Pages to add", 25, label);
        assert_eq!(keyboard_press(&mut kb, '1' as u32, &mut d), KbResult::Changed, "its caret shows");
        assert_eq!((kb.text.as_str(), kb.count_fresh), ("1", false));
        assert_eq!(keyboard_press(&mut kb, '2' as u32, &mut d), KbResult::Changed);
        assert_eq!(kb.text.as_str(), "12");
        keyboard_open_count(&mut kb, "Pages to add", 25, label);
        assert_eq!(keyboard_press(&mut kb, KB_KEY_INC, &mut d), KbResult::Changed);
        assert_eq!((kb.text.as_str(), kb.count_fresh), ("2", false), "+ from 1");
        keyboard_open_count(&mut kb, "Pages to add", 25, label);
        assert_eq!(keyboard_press(&mut kb, KB_KEY_DEC, &mut d), KbResult::Changed, "- stays at 1, its caret shows");
        assert_eq!((kb.text.as_str(), kb.count_fresh), ("1", false));
        assert_eq!(keyboard_press(&mut kb, KB_KEY_DEC, &mut d), KbResult::None);
        keyboard_open_count(&mut kb, "Pages to add", 0, label);
        assert_eq!((kb.text.as_str(), kb.count_fresh), ("", false), "no room: empty");
    }

    /// The count pad's opening 1 is plain black text, drawn as its title is (bold, at the title's size), with no
    /// caret after it; a count typed has its caret (2.0.0-beta.2, the owner's Scribe feedback of 2026-11-04: the
    /// white 1 in a black box, at the title size, "starts the counter at 0 - and is displayed at a large,
    /// intrusive amount"). The pad is titled "1" here, so the title's glyph and the count's are the same.
    #[test]
    fn the_count_pads_opening_one_is_plain_text() {
        assert_eq!(font_init(), 0);
        let screen = Rect { x: 0, y: 0, w: 1000, h: 1600 };
        let draw = |fresh: bool| {
            let mut kb = Keyboard::default();
            keyboard_open_count(&mut kb, "1", 25, label);
            kb.count_fresh = fresh;
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, screen.w, screen.h), 0);
            keyboard_draw(&mut kb, &mut c, screen);
            (c, field_rect(kb.area))
        };
        // The black pixels' bounds (x0, y0, x1, y1) inside r, and how many.
        let ink = |c: &Canvas, r: Rect| {
            let (mut b, mut n) = ((i32::MAX, i32::MAX, i32::MIN, i32::MIN), 0);
            for y in r.y..r.y + r.h {
                for x in r.x..r.x + r.w {
                    if c.px[(y * c.stride + x) as usize] < 128 {
                        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
                        n += 1;
                    }
                }
            }
            (b, n)
        };
        let (fresh, f) = draw(true);
        let (typed, _) = draw(false);
        // inside the field's frame: the title from its left margin, the count after the title's width
        let inner = Rect { x: f.x + ui_px(12.0), y: f.y + ui_px(12.0), w: f.w - ui_px(24.0), h: f.h - ui_px(24.0) };
        let count_x = f.x + ui_px(24.0) + font_measure(FontFace::Bold, ui_text_size(), "1") + ui_px(30.0);
        let title_r = Rect { w: count_x - inner.x, ..inner };
        let count_r = Rect { x: count_x, w: inner.x + inner.w - count_x, ..inner };
        let (tb, tn) = ink(&fresh, title_r);
        let (cb, cn) = ink(&fresh, count_r);
        assert!(tn > 0 && cn > 0, "the title and the count are drawn");
        assert_eq!((cb.1, cb.3), (tb.1, tb.3), "the count's 1 as tall as the title's, on its line");
        assert_eq!((cb.2 - cb.0, cn), (tb.2 - tb.0, tn), "the same glyph, black on white: no box, no caret");
        // Typed, the same 1 and a caret after it, and nothing else differs.
        let (yb, yn) = ink(&typed, count_r);
        assert!(yn > cn && yb.0 == cb.0 && yb.2 > cb.2, "a typed count has its caret");
        for y in count_r.y..count_r.y + count_r.h {
            for x in count_r.x..=cb.2 {
                let i = (y * fresh.stride + x) as usize;
                assert_eq!(fresh.px[i], typed.px[i], "the 1 itself drawn the same at ({x}, {y})");
            }
        }
    }

    // A keyboard open on the letters page, drawn on this screen (its keys where keyboard_key_rect finds them).
    fn drawn(screen: Rect) -> (Keyboard, Canvas) {
        let mut kb = opened(b"");
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, screen.w, screen.h), 0);
        keyboard_draw(&mut kb, &mut c, screen);
        (kb, c)
    }

    // A key typed (keyboard_press), which must change something: what it refreshes beyond the text line.
    fn tap(kb: &mut Keyboard, code: u32) -> Rect {
        let mut d = RECT_EMPTY;
        assert_eq!(keyboard_press(kb, code, &mut d), KbResult::Changed, "key {code}");
        d
    }

    fn type_str(kb: &mut Keyboard, s: &str) {
        for ch in s.chars() {
            tap(kb, ch as u32);
        }
    }

    // The key with this code tapped as a finger does, down and up at its middle (keyboard_pointer): what the lift
    // refreshes.
    fn tap_at(kb: &mut Keyboard, code: u32) -> Rect {
        let mut r = RECT_EMPTY;
        assert!(keyboard_key_rect(kb, code, &mut r), "key {code} on the page");
        let (x, y) = ((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32);
        let mut d = RECT_EMPTY;
        keyboard_pointer(kb, 0, x, y, 0, &mut d);
        keyboard_pointer(kb, 2, x, y, 40, &mut d);
        d
    }

    /// Shift tapped once gives the next letter its capital and lets go, as it always has; a key with no capital
    /// (a digit, a symbol, the space bar) leaves it waiting for a letter.
    #[test]
    fn shift_once_gives_the_next_letter_only() {
        let mut kb = opened(b"");
        tap(&mut kb, KB_KEY_SHIFT);
        assert_eq!(kb.shift, KbShift::Once);
        type_str(&mut kb, "abc");
        assert_eq!((kb.text.as_str(), kb.shift), ("Abc", KbShift::Off));
        tap(&mut kb, KB_KEY_SHIFT);
        type_str(&mut kb, "1- &");
        assert_eq!((kb.text.as_str(), kb.shift), ("Abc1- &", KbShift::Once), "no capital yet: it waits for a letter");
        type_str(&mut kb, "\u{e9}e");
        assert_eq!((kb.text.as_str(), kb.shift), ("Abc1- &\u{c9}e", KbShift::Off));
    }

    /// Shift tapped twice is caps on (2.0.0-beta.2, the owner, 2026-11-05: "next toggle shift in typing toggles off
    /// shift for next char then shift on which makes longer caps strings possible"): every letter a capital through
    /// digits, the space bar, delete and the symbols and accents pages, until a third tap turns it off.
    #[test]
    fn shift_twice_is_caps_on_until_a_third_tap() {
        let mut kb = opened(b"");
        tap(&mut kb, KB_KEY_SHIFT);
        tap(&mut kb, KB_KEY_SHIFT);
        assert_eq!(kb.shift, KbShift::Caps);
        type_str(&mut kb, "nasa 2026");
        assert_eq!(kb.text.as_str(), "NASA 2026");
        tap(&mut kb, KB_KEY_BKSP);
        tap(&mut kb, KB_KEY_BKSP);
        type_str(&mut kb, "x ");
        assert_eq!(kb.text.as_str(), "NASA 20X ");
        tap(&mut kb, KB_KEY_SYMBOLS);
        assert_eq!(kb.page, 3);
        type_str(&mut kb, "!&");
        tap(&mut kb, KB_KEY_PAGE); // the symbols page's accents key
        assert_eq!(kb.page, 1);
        type_str(&mut kb, "\u{e9}\u{f1}\u{153}");
        tap(&mut kb, KB_KEY_PAGE); // abc
        assert_eq!(kb.page, 0);
        type_str(&mut kb, "ab");
        assert_eq!((kb.text.as_str(), kb.shift), ("NASA 20X !&\u{c9}\u{d1}\u{152}AB", KbShift::Caps));
        tap(&mut kb, KB_KEY_SHIFT);
        assert_eq!(kb.shift, KbShift::Off, "a third tap: off");
        type_str(&mut kb, "cd");
        assert!(kb.text.as_str().ends_with("ABcd"));
        tap(&mut kb, KB_KEY_SHIFT);
        assert_eq!(kb.shift, KbShift::Once, "and round again");
    }

    /// Caps on ends with the keyboard: closed by Done, Cancel or a hardware Esc, the next keyboard opens with shift
    /// off (every keyboard_open starts there, as before).
    #[test]
    fn caps_on_ends_when_the_keyboard_closes() {
        let mut d = RECT_EMPTY;
        for close in [Some(KB_KEY_DONE), Some(KB_KEY_CANCEL), None] {
            let mut kb = opened(b"x");
            tap(&mut kb, KB_KEY_SHIFT);
            tap(&mut kb, KB_KEY_SHIFT);
            assert_eq!(kb.shift, KbShift::Caps);
            match close {
                Some(code) => assert_ne!(keyboard_press(&mut kb, code, &mut d), KbResult::Changed),
                None => assert_eq!(keyboard_input(&mut kb, &KbInput::Cancel, &mut d), KbResult::Cancel),
            }
            assert!(!kb.open);
            keyboard_open(&mut kb, "Name", b"");
            assert_eq!(kb.shift, KbShift::Off);
            type_str(&mut kb, "ab");
            assert_eq!(kb.text.as_str(), "ab");
        }
    }

    /// What a shift tap refreshes: off to once and caps on to off change every letter key's label (capitals while
    /// shift is on), so the keyboard, as before; once to caps on changes only the shift key (its bar), so only it.
    /// A letter typed with caps on refreshes what any letter does (its key, then the text line, keyboard_pointer);
    /// the capital of shift once takes the letter keys back to small letters. On the symbols page no label changes
    /// case, so each of the three taps refreshes the shift key alone.
    #[test]
    fn a_shift_tap_refreshes_only_what_it_changed() {
        let (mut kb, _c) = drawn(Rect { x: 0, y: 0, w: 1860, h: 2480 });
        let mut key = RECT_EMPTY;
        assert!(keyboard_key_rect(&kb, KB_KEY_SHIFT, &mut key));
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), kb.area, "off to once");
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), key, "once to caps on: the shift key alone");
        assert_eq!(kb.shift, KbShift::Caps);
        assert_eq!(tap(&mut kb, 'a' as u32), RECT_EMPTY, "a letter with caps on: no more than any letter");
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), kb.area, "caps on to off");
        tap_at(&mut kb, KB_KEY_SHIFT);
        assert_eq!(tap(&mut kb, '1' as u32), RECT_EMPTY, "a digit: shift once still waits");
        assert_eq!(tap(&mut kb, 'b' as u32), kb.area, "shift once let go");
        assert_eq!((kb.text.as_str(), kb.shift), ("A1B", KbShift::Off));
        // The symbols page: no key's label there changes case, so every tap refreshes the shift key alone.
        tap_at(&mut kb, KB_KEY_SYMBOLS);
        assert_eq!(kb.page, 3);
        assert!(keyboard_key_rect(&kb, KB_KEY_SHIFT, &mut key));
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), key, "symbols page, off to once");
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), key, "symbols page, once to caps on");
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), key, "symbols page, caps on to off");
        // The accents page's letters show their capitals: the keyboard again.
        tap_at(&mut kb, KB_KEY_PAGE);
        assert_eq!(kb.page, 1);
        assert_eq!(tap_at(&mut kb, KB_KEY_SHIFT), kb.area, "accents page, off to once");
    }

    /// The two on-states read apart on the e-ink screen: shift once is the pressed key, as shift always was (black,
    /// its label white); caps on is that key with a white bar under its label, as wide as the label. Nothing else on
    /// the keyboard differs between the two (the letter keys show their capitals in both). Gothic and Classic,
    /// upright and sideways.
    #[test]
    fn caps_on_draws_a_bar_under_the_shift_label() {
        assert_eq!(font_init(), 0);
        for th in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(th);
            for screen in [Rect { x: 0, y: 0, w: 1860, h: 2480 }, Rect { x: 0, y: 0, w: 2480, h: 1860 }] {
                let shot = |shift: KbShift, pressed: bool| {
                    let (mut kb, mut c) = drawn(screen);
                    let mut r = RECT_EMPTY;
                    assert!(keyboard_key_rect(&kb, KB_KEY_SHIFT, &mut r));
                    kb.shift = shift;
                    if pressed {
                        kb.pressed_key = key_at(&kb, (r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32);
                    }
                    keyboard_draw(&mut kb, &mut c, screen);
                    (c, r)
                };
                let (pressed, _) = shot(KbShift::Off, true);
                let (once, r) = shot(KbShift::Once, false);
                let (caps, _) = shot(KbShift::Caps, false);
                let px = |c: &Canvas, x: i32, y: i32| c.px[(y * c.stride + x) as usize];
                let inside = |x: i32, y: i32| rect_contains(r, x, y);
                let (mut label_bottom, mut bar) = (i32::MIN, (i32::MAX, i32::MAX, i32::MIN, i32::MIN));
                for y in 0..screen.h {
                    for x in 0..screen.w {
                        let (o, k) = (px(&once, x, y), px(&caps, x, y));
                        if !inside(x, y) {
                            assert_eq!(o, k, "{th:?} {}x{}: only the shift key differs, not ({x}, {y})", screen.w, screen.h);
                            continue;
                        }
                        assert_eq!(o, px(&pressed, x, y), "{th:?}: shift once is the pressed key, at ({x}, {y})");
                        if o > 128 && rect_contains(rect_inflate(r, -ui_px(16.0)), x, y) {
                            label_bottom = label_bottom.max(y);
                        }
                        if o != k {
                            assert_eq!((o, k), (0, 255), "{th:?}: caps on adds white to the black key, at ({x}, {y})");
                            bar = (bar.0.min(x), bar.1.min(y), bar.2.max(x), bar.3.max(y));
                        }
                    }
                }
                let (bw, bh) = (bar.2 - bar.0 + 1, bar.3 - bar.1 + 1);
                assert_eq!(bh, ui_px(12.0), "{th:?} {}x{}: a bar", screen.w, screen.h);
                assert!(bw >= ui_px(60.0) && bw < r.w - ui_px(30.0), "{th:?}: as wide as the label, inside the key: {bw}");
                assert!((bar.0 + bar.2 + 1 - (2 * r.x + r.w)).abs() <= 1, "{th:?}: centered under the label");
                assert!(bar.1 > label_bottom && bar.3 < r.y + r.h - ui_px(10.0), "{th:?}: under the label, in the key");
                let ink = (r.y..r.y + r.h).flat_map(|y| (r.x..r.x + r.w).map(move |x| (x, y))).filter(|&(x, y)| px(&caps, x, y) < 128).count();
                assert!(ink * 2 > (r.w * r.h) as usize, "{th:?}: caps on is still the black key");
            }
        }
        ui_set_theme(UiTheme::Gothic);
    }
}
