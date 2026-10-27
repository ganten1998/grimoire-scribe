//! json_min.h: a small, bounded JSON reader for answers Grimoire asks for (the Bluetooth helper's
//! local API, bluetooth.h). It tokenizes into a caller's array, never allocates, and refuses what it
//! cannot hold (too deep, too many tokens, a broken document) as a whole: a refusal reads as no
//! answer, never as a half one.

use crate::prelude::*;

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

/// Tokenizes text (len bytes): an object's tokens are key, value, key, value... after it, an array's
/// items after it. Returns how many tokens, or -1 (not one whole JSON value, or more than cap
/// tokens, or nested deeper than JSON_DEPTH).
pub fn json_parse(text: &[u8], toks: &mut [JsonTok]) -> i32 {
    todo!()
}
/// The value of key in the object at token obj, or -1 (no such key, obj not an object).
pub fn json_get(text: &[u8], toks: &[JsonTok], obj: i32, key: &str) -> i32 {
    todo!()
}
/// Item i of the array at token arr, or -1.
pub fn json_item(toks: &[JsonTok], arr: i32, i: i32) -> i32 {
    todo!()
}
/// A string token's text, with its escapes decoded (\uXXXX as UTF-8, a lone surrogate as U+FFFD),
/// cut at cap - 1 bytes on a character's boundary. False when it is not a string.
pub fn json_string(text: &[u8], t: &JsonTok, out: &mut String, cap: usize) -> bool {
    todo!()
}
/// A true or false token: *v. False when it is neither.
pub fn json_bool(t: &JsonTok, v: &mut bool) -> bool {
    todo!()
}
/// A number token as an int (its integer part, clamped to int's range): *v. False when not a number.
pub fn json_int(text: &[u8], t: &JsonTok, v: &mut i32) -> bool {
    todo!()
}
