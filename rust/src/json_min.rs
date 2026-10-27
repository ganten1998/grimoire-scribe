//! json_min.h: a small, bounded JSON reader for answers Grimoire asks for (the Bluetooth helper's
//! local API, bluetooth.h). It tokenizes into a caller's array, never allocates, and refuses what it
//! cannot hold (too deep, too many tokens, a broken document) as a whole: a refusal reads as no
//! answer, never as a half one.

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum JsonType {
    #[default]
    None = 0,  // JS_NONE
    Object = 1,  // JS_OBJECT
    Array = 2,  // JS_ARRAY
    String = 3,  // JS_STRING
    Number = 4,  // JS_NUMBER
    True = 5,  // JS_TRUE
    False = 6,  // JS_FALSE
    Null = 7,  // JS_NULL
}
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonTok {
    pub type_: JsonType,
    /// the value's bytes in the text: [start, end); a string's without its quotes
    pub start: i32,
    pub end: i32,
    /// an object's keys, an array's items
    pub size: i32,
    /// the token after this value and all it holds
    pub next: i32,
}

pub const JSON_DEPTH: usize = 16;

struct P<'a> {
    s: &'a [u8],
    at: usize,
    toks: &'a mut [JsonTok],
    cap: i32,
    n: i32,
}

impl P<'_> {
    fn len(&self) -> usize {
        self.s.len()
    }
}

fn ws(p: &mut P) {
    while p.at < p.len() && (p.s[p.at] == b' ' || p.s[p.at] == b'\t' || p.s[p.at] == b'\n' || p.s[p.at] == b'\r') {
        p.at += 1;
    }
}

fn add(p: &mut P, t: JsonType, start: i32) -> i32 {
    if p.n >= p.cap {
        return -1;
    }
    p.toks[p.n as usize] = JsonTok { type_: t, start, end: start, size: 0, next: 0 };
    let i = p.n;
    p.n += 1;
    i
}

fn hexval(c: u8) -> i32 {
    match c {
        b'0'..=b'9' => (c - b'0') as i32,
        b'a'..=b'f' => (c - b'a') as i32 + 10,
        b'A'..=b'F' => (c - b'A') as i32 + 10,
        _ => -1,
    }
}

/// A string from the opening quote: its token (without the quotes), or -1.
fn string(p: &mut P) -> i32 {
    let t = add(p, JsonType::String, p.at as i32 + 1);
    if t < 0 {
        return -1;
    }
    p.at += 1;
    while p.at < p.len() {
        let c = p.s[p.at];
        if c == b'"' {
            p.toks[t as usize].end = p.at as i32;
            p.at += 1;
            return t;
        }
        if c < 0x20 {
            return -1;
        }
        if c == b'\\' {
            p.at += 1;
            if p.at >= p.len() {
                return -1;
            }
            let e = p.s[p.at];
            // (The C asks strchr whether e is a valid escape, and strchr finds the string's own end
            // for a NUL byte: a backslash and a NUL byte pass. Ported as it is.)
            if e == b'u' {
                for k in 1..=4 {
                    if p.at + k >= p.len() || hexval(p.s[p.at + k]) < 0 {
                        return -1;
                    }
                }
                p.at += 4;
            } else if e != 0 && !b"\"\\/bfnrt".contains(&e) {
                return -1;
            }
        }
        p.at += 1;
    }
    -1
}

fn container(p: &mut P, depth: usize, object: bool) -> i32 {
    if depth >= JSON_DEPTH {
        return -1;
    }
    let t = add(p, if object { JsonType::Object } else { JsonType::Array }, p.at as i32);
    if t < 0 {
        return -1;
    }
    p.at += 1;
    ws(p);
    let close = if object { b'}' } else { b']' };
    if p.at < p.len() && p.s[p.at] == close {
        p.at += 1;
    } else {
        loop {
            ws(p);
            if object {
                if p.at >= p.len() || p.s[p.at] != b'"' || string(p) < 0 {
                    return -1;
                }
                ws(p);
                if p.at >= p.len() {
                    return -1;
                }
                let colon = p.s[p.at];
                p.at += 1;
                if colon != b':' {
                    return -1;
                }
            }
            if value(p, depth + 1) < 0 {
                return -1;
            }
            p.toks[t as usize].size += 1;
            ws(p);
            if p.at >= p.len() {
                return -1;
            }
            let c = p.s[p.at];
            p.at += 1;
            if c == b',' {
                continue;
            }
            if c == close {
                break;
            }
            return -1;
        }
    }
    p.toks[t as usize].end = p.at as i32;
    p.toks[t as usize].next = p.n;
    t
}

fn literal(p: &mut P, word: &[u8], type_: JsonType) -> i32 {
    let n = word.len();
    if p.at + n > p.len() || &p.s[p.at..p.at + n] != word {
        return -1;
    }
    let t = add(p, type_, p.at as i32);
    if t < 0 {
        return -1;
    }
    p.at += n;
    p.toks[t as usize].end = p.at as i32;
    p.toks[t as usize].next = p.n;
    t
}

fn digits(p: &mut P) {
    while p.at < p.len() && p.s[p.at].is_ascii_digit() {
        p.at += 1;
    }
}

fn number(p: &mut P) -> i32 {
    let t = add(p, JsonType::Number, p.at as i32);
    if t < 0 {
        return -1;
    }
    if p.at < p.len() && p.s[p.at] == b'-' {
        p.at += 1;
    }
    let d = p.at;
    digits(p);
    if p.at == d {
        return -1;
    }
    if p.at < p.len() && p.s[p.at] == b'.' {
        p.at += 1;
        let f = p.at;
        digits(p);
        if p.at == f {
            return -1;
        }
    }
    if p.at < p.len() && (p.s[p.at] == b'e' || p.s[p.at] == b'E') {
        p.at += 1;
        if p.at < p.len() && (p.s[p.at] == b'+' || p.s[p.at] == b'-') {
            p.at += 1;
        }
        let e = p.at;
        digits(p);
        if p.at == e {
            return -1;
        }
    }
    p.toks[t as usize].end = p.at as i32;
    p.toks[t as usize].next = p.n;
    t
}

fn value(p: &mut P, depth: usize) -> i32 {
    ws(p);
    if p.at >= p.len() {
        return -1;
    }
    let c = p.s[p.at];
    if c == b'{' || c == b'[' {
        return container(p, depth, c == b'{');
    }
    if c == b'"' {
        let t = string(p);
        if t >= 0 {
            p.toks[t as usize].next = p.n;
        }
        return t;
    }
    if c == b't' {
        return literal(p, b"true", JsonType::True);
    }
    if c == b'f' {
        return literal(p, b"false", JsonType::False);
    }
    if c == b'n' {
        return literal(p, b"null", JsonType::Null);
    }
    number(p)
}

/// Tokenizes text (len bytes): an object's tokens are key, value, key, value... after it, an array's
/// items after it. Returns how many tokens, or -1 (not one whole JSON value, or more than cap
/// tokens, or nested deeper than JSON_DEPTH).
pub fn json_parse(text: &[u8], toks: &mut [JsonTok]) -> i32 {
    let cap = toks.len().min(i32::MAX as usize) as i32;
    if cap <= 0 || text.len() > (i32::MAX / 2) as usize {
        return -1;
    }
    let len = text.len();
    let mut p = P { s: text, at: 0, toks, cap, n: 0 };
    if value(&mut p, 0) < 0 {
        return -1;
    }
    ws(&mut p);
    if p.at == len {
        p.n
    } else {
        -1
    }
}

/// The value of key in the object at token obj, or -1 (no such key, obj not an object).
pub fn json_get(text: &[u8], toks: &[JsonTok], obj: i32, key: &str) -> i32 {
    let ntok = toks.len() as i32;
    if obj < 0 || obj >= ntok || toks[obj as usize].type_ != JsonType::Object {
        return -1;
    }
    let key = key.as_bytes();
    let kl = key.len();
    let mut at = obj + 1;
    let mut i = 0;
    while i < toks[obj as usize].size && at + 1 < ntok {
        let k = &toks[at as usize];
        // (A key with escapes never matches: the keys asked for are plain.)
        if (k.end - k.start) as usize == kl && &text[k.start as usize..k.start as usize + kl] == key {
            return at + 1;
        }
        at = toks[(at + 1) as usize].next;
        i += 1;
    }
    -1
}

/// Item i of the array at token arr, or -1.
pub fn json_item(toks: &[JsonTok], arr: i32, i: i32) -> i32 {
    let ntok = toks.len() as i32;
    if arr < 0 || arr >= ntok || toks[arr as usize].type_ != JsonType::Array || i < 0 || i >= toks[arr as usize].size {
        return -1;
    }
    let mut at = arr + 1;
    let mut k = 0;
    while k < i && at < ntok {
        at = toks[at as usize].next;
        k += 1;
    }
    if at < ntok {
        at
    } else {
        -1
    }
}

/// UTF-8 for cp at out[at..], or the bytes it takes (0 when it does not fit whole below cap).
fn put_utf8(out: &mut Vec<u8>, at: usize, cap: usize, cp: u32) -> usize {
    let mut b = [0u8; 4];
    let n;
    if cp < 0x80 {
        b[0] = cp as u8;
        n = 1;
    } else if cp < 0x800 {
        b[0] = (0xC0 | cp >> 6) as u8;
        b[1] = (0x80 | (cp & 63)) as u8;
        n = 2;
    } else if cp < 0x10000 {
        b[0] = (0xE0 | cp >> 12) as u8;
        b[1] = (0x80 | (cp >> 6 & 63)) as u8;
        b[2] = (0x80 | (cp & 63)) as u8;
        n = 3;
    } else {
        b[0] = (0xF0 | cp >> 18) as u8;
        b[1] = (0x80 | (cp >> 12 & 63)) as u8;
        b[2] = (0x80 | (cp >> 6 & 63)) as u8;
        b[3] = (0x80 | (cp & 63)) as u8;
        n = 4;
    }
    if at + n >= cap {
        return 0; // (no room for it whole: the text ends before it)
    }
    out.extend_from_slice(&b[..n]);
    n
}

fn hex4(s: &[u8]) -> u32 {
    (hexval(s[0]) << 12 | hexval(s[1]) << 8 | hexval(s[2]) << 4 | hexval(s[3])) as u32
}

/// A string token's text, with its escapes decoded (\uXXXX as UTF-8, a lone surrogate as U+FFFD),
/// cut at cap - 1 bytes on a character's boundary. False when it is not a string.
///
/// The text is bytes, not a String: a name that came over the radio need not be UTF-8, and the C
/// copies a character's bytes as they are (btapi.c makes everything but printable ASCII safe). A
/// \u0000 ends the text, as it ends the C string.
pub fn json_string(text: &[u8], t: &JsonTok, out: &mut Vec<u8>, cap: usize) -> bool {
    if t.type_ != JsonType::String || cap == 0 {
        return false;
    }
    let mut o: Vec<u8> = Vec::new();
    let mut i = t.start;
    while i < t.end {
        let mut cp: u32;
        if text[i as usize] != b'\\' {
            // A raw byte: copied, but a multi-byte character only whole.
            let c = text[i as usize];
            let n: i32 = if c < 0x80 {
                1
            } else if c >= 0xF0 {
                4
            } else if c >= 0xE0 {
                3
            } else if c >= 0xC0 {
                2
            } else {
                1
            };
            if i + n > t.end || o.len() + n as usize >= cap {
                break;
            }
            o.extend_from_slice(&text[i as usize..(i + n) as usize]);
            i += n;
            continue;
        }
        let e = text[i as usize + 1];
        i += 2;
        if e == b'u' {
            cp = hex4(&text[i as usize..]);
            i += 4;
            if (0xD800..0xDC00).contains(&cp) {
                if i + 6 <= t.end && text[i as usize] == b'\\' && text[i as usize + 1] == b'u' {
                    let lo = hex4(&text[i as usize + 2..]);
                    if (0xDC00..0xE000).contains(&lo) {
                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                        i += 6;
                    } else {
                        cp = 0xFFFD;
                    }
                } else {
                    cp = 0xFFFD;
                }
            } else if (0xDC00..0xE000).contains(&cp) {
                cp = 0xFFFD;
            }
        } else {
            cp = match e {
                b'b' => 8,
                b'f' => 12,
                b'n' => b'\n' as u32,
                b'r' => b'\r' as u32,
                b't' => b'\t' as u32,
                _ => e as u32,
            };
        }
        let at = o.len();
        if put_utf8(&mut o, at, cap, cp) == 0 {
            break;
        }
    }
    if let Some(nul) = o.iter().position(|&b| b == 0) {
        o.truncate(nul);
    }
    *out = o;
    true
}

/// A true or false token: *v. False when it is neither.
pub fn json_bool(t: &JsonTok, v: &mut bool) -> bool {
    if t.type_ != JsonType::True && t.type_ != JsonType::False {
        return false;
    }
    *v = t.type_ == JsonType::True;
    true
}

/// A number token as an int (its integer part, clamped to int's range): *v. False when not a number.
pub fn json_int(text: &[u8], t: &JsonTok, v: &mut i32) -> bool {
    if t.type_ != JsonType::Number {
        return false;
    }
    let mut n: i64 = 0;
    let mut i = t.start as usize;
    let neg = text[i] == b'-';
    if neg {
        i += 1;
    }
    while i < t.end as usize && text[i].is_ascii_digit() {
        n = n * 10 + (text[i] - b'0') as i64;
        if n > i32::MAX as i64 {
            n = i32::MAX as i64 + 1;
        }
        i += 1;
    }
    if neg {
        n = -n;
    }
    *v = if n > i32::MAX as i64 {
        i32::MAX
    } else if n < i32::MIN as i64 {
        i32::MIN
    } else {
        n as i32
    };
    true
}
