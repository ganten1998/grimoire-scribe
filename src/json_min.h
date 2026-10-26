// json_min.h: a small, bounded JSON reader for answers Grimoire asks for (the Bluetooth helper's
// local API, bluetooth.h). It tokenizes into a caller's array, never allocates, and refuses what it
// cannot hold (too deep, too many tokens, a broken document) as a whole: a refusal reads as no
// answer, never as a half one.
#pragma once
#include <stdbool.h>
#include <stddef.h>

typedef enum { JS_NONE = 0, JS_OBJECT, JS_ARRAY, JS_STRING, JS_NUMBER, JS_TRUE, JS_FALSE, JS_NULL } JsonType;
typedef struct {
    JsonType type;
    int start, end;  // the value's bytes in the text: [start, end); a string's without its quotes
    int size;        // an object's keys, an array's items
    int next;        // the token after this value and all it holds
} JsonTok;

#define JSON_DEPTH 16

// Tokenizes text (len bytes): an object's tokens are key, value, key, value... after it, an array's
// items after it. Returns how many tokens, or -1 (not one whole JSON value, or more than cap
// tokens, or nested deeper than JSON_DEPTH).
int json_parse(const char *text, size_t len, JsonTok *toks, int cap);
// The value of key in the object at token obj, or -1 (no such key, obj not an object).
int json_get(const char *text, const JsonTok *toks, int ntok, int obj, const char *key);
// Item i of the array at token arr, or -1.
int json_item(const JsonTok *toks, int ntok, int arr, int i);
// A string token's text, with its escapes decoded (\uXXXX as UTF-8, a lone surrogate as U+FFFD),
// cut at cap - 1 bytes on a character's boundary. False when it is not a string.
bool json_string(const char *text, const JsonTok *t, char *out, size_t cap);
// A true or false token: *v. False when it is neither.
bool json_bool(const JsonTok *t, bool *v);
// A number token as an int (its integer part, clamped to int's range): *v. False when not a number.
bool json_int(const char *text, const JsonTok *t, int *v);
