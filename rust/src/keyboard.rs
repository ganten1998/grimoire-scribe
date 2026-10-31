//! keyboard.h: on-screen keyboard modal for naming notebooks and folders.
//! QWERTY + numbers row, shift, space, backspace, Cancel / Done, - ' and . beside the space bar, and
//! a key each to a page of symbols and a page of accented letters (and back). Pen or finger taps. The text is UTF-8: a key types a whole character and
//! backspace takes a whole character away.

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
pub const KB_KEY_BKSP: u32 = 8;
pub const KB_KEY_DONE: u32 = 13;
pub const KB_KEY_CANCEL: u32 = 27;

#[derive(Clone, Copy, Debug, Default)]
pub struct Keyboard {
    pub open: bool,
    pub title: FixedStr<64>,
    pub text: FixedStr<KB_TEXT_MAX>,
    pub shift: bool,
    /// 0 letters, 1 accents, 2 number pad, 3 symbols
    pub page: i32,
    /// where it was last drawn (whole modal)
    pub area: Rect,
    pub press: Press,
    /// -1 if none
    pub pressed_key: i32,
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
    let h = ui_px(170.0) + 5 * ui_px(150.0) + ui_px(60.0);
    let y = if dock_y > 0 { screen.y + dock_y } else { screen.y + screen.h - h - ui_px(20.0) };
    Rect { x: screen.x + ui_px(20.0), y, w: screen.w - ui_px(40.0), h }
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

fn layout(area: Rect, page: i32, keys: &mut [Key; MAX_KEYS]) -> usize {
    if page == 2 {
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
        keys[n] = Key { r: Rect { x, y: y + h + gap, w: area.w - ui_px(60.0), h }, code: KB_KEY_DONE };
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
    for code in ['-' as u32, KB_KEY_SYMBOLS, KB_KEY_PAGE] {
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
    copy_text(&mut kb.title, title.as_bytes());
    copy_text(&mut kb.text, initial);
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
        KB_KEY_CANCEL => tr("Cancel"),
        // The two page keys: each to its page, and on it back to the letters.
        KB_KEY_PAGE => {
            if kb.page == 1 {
                "abc"
            } else {
                "\u{e1}\u{e9}" // a and e acute
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
            let mut enc = [0u8; 4];
            let n = utf8_put(if kb.shift { keyboard_upper(code) } else { code }, &mut enc);
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
    let area = modal_rect(screen, kb.dock_y);
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
    let mut at = 0;
    while at < text.len() && font_measure(FontFace::Regular, ts * 1.1, &text[at..]) > tr.w - ui_px(20.0) {
        at = utf8_next(text.as_bytes(), at);
    }
    let shown = &text[at..];
    let w = font_measure(FontFace::Regular, ts * 1.1, shown);
    font_draw_left(c, FontFace::Regular, ts * 1.1, tr, shown, 0);
    canvas_fill(c, Rect { x: tr.x + w + ui_px(4.0), y: f.y + ui_px(24.0), w: ui_px(5.0), h: f.h - ui_px(48.0) }, 0);
    let mut keys = [Key::default(); MAX_KEYS];
    let n = layout(area, kb.page, &mut keys);
    for (i, key) in keys.iter().enumerate().take(n) {
        let mut ch = [0u8; 8];
        let lab = key_label(kb, key.code, &mut ch);
        let st = if i as i32 == kb.pressed_key || (key.code == KB_KEY_SHIFT && kb.shift) {
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
    }
    area
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

// Appends the key's character (its capital while shift is on) if it fits; shift is one-shot.
fn type_key(kb: &mut Keyboard, code: u32, dirty: &mut Rect) {
    let ch = if kb.shift { keyboard_upper(code) } else { code };
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
    if kb.shift && ch != code {
        kb.shift = false;
        *dirty = kb.area;
    }
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
        let c = keys[kb.pressed_key as usize].code;
        if c == KB_KEY_SHIFT || c == KB_KEY_PAGE || c == KB_KEY_SYMBOLS || c == KB_KEY_DONE || c == KB_KEY_CANCEL {
            return KbResult::None;
        }
        kb.typed = true; // type now: the lift only releases the key
    } else if phase == 1 {
        press_move(&mut kb.press, x, y);
        return KbResult::None;
    } else {
        let k = kb.pressed_key;
        kb.pressed_key = -1;
        if k < 0 || k >= n {
            return KbResult::None;
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
    let mut len = kb.text.len();
    match keys[k as usize].code {
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
            kb.shift = !kb.shift;
            *dirty = kb.area;
            return KbResult::Changed;
        }
        KB_KEY_PAGE | KB_KEY_SYMBOLS => {
            let to = if keys[k as usize].code == KB_KEY_PAGE { 1 } else { 3 };
            kb.page = if kb.page == to { 0 } else { to };
            *dirty = kb.area;
            return KbResult::Changed;
        }
        KB_KEY_BKSP => {
            // A whole character: back over its continuation bytes to its first byte.
            let b = kb.text.as_bytes();
            while len > 1 && (b[len - 1] & 0xC0) == 0x80 {
                len -= 1;
            }
            if len != 0 {
                let mut buf = [0u8; KB_TEXT_MAX];
                buf[..len - 1].copy_from_slice(&b[..len - 1]);
                kb.text.set_bytes(&buf[..len - 1]);
            }
        }
        code => type_key(kb, code, dirty),
    }
    kb.field_dirty = field_rect(kb.area);
    KbResult::Changed
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(kb.open && kb.pressed_key == -1 && kb.page == 0 && !kb.shift);
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
}
