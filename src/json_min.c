// json_min.c: see json_min.h.
#include "json_min.h"

#include <limits.h>
#include <string.h>

typedef struct {
    const char *s;
    size_t len, at;
    JsonTok *toks;
    int cap, n;
} P;

static void ws(P *p) {
    while (p->at < p->len && (p->s[p->at] == ' ' || p->s[p->at] == '\t' || p->s[p->at] == '\n' || p->s[p->at] == '\r'))
        p->at++;
}

static int add(P *p, JsonType t, int start) {
    if (p->n >= p->cap) return -1;
    p->toks[p->n] = (JsonTok){t, start, start, 0, 0};
    return p->n++;
}

static int hexval(char c) {
    return c >= '0' && c <= '9' ? c - '0' : c >= 'a' && c <= 'f' ? c - 'a' + 10 : c >= 'A' && c <= 'F' ? c - 'A' + 10 : -1;
}

// A string from the opening quote: its token (without the quotes), or -1.
static int string(P *p) {
    int t = add(p, JS_STRING, (int)p->at + 1);
    if (t < 0) return -1;
    for (p->at++; p->at < p->len; p->at++) {
        unsigned char c = (unsigned char)p->s[p->at];
        if (c == '"') {
            p->toks[t].end = (int)p->at++;
            return t;
        }
        if (c < 0x20) return -1;
        if (c == '\\') {
            if (++p->at >= p->len) return -1;
            char e = p->s[p->at];
            if (e == 'u') {
                for (int k = 1; k <= 4; k++)
                    if (p->at + k >= p->len || hexval(p->s[p->at + k]) < 0) return -1;
                p->at += 4;
            } else if (!strchr("\"\\/bfnrt", e)) {
                return -1;
            }
        }
    }
    return -1;
}

static int value(P *p, int depth);

static int container(P *p, int depth, bool object) {
    if (depth >= JSON_DEPTH) return -1;
    int t = add(p, object ? JS_OBJECT : JS_ARRAY, (int)p->at);
    if (t < 0) return -1;
    p->at++;
    ws(p);
    if (p->at < p->len && p->s[p->at] == (object ? '}' : ']')) {
        p->at++;
    } else {
        for (;;) {
            ws(p);
            if (object) {
                if (p->at >= p->len || p->s[p->at] != '"' || string(p) < 0) return -1;
                ws(p);
                if (p->at >= p->len || p->s[p->at++] != ':') return -1;
            }
            if (value(p, depth + 1) < 0) return -1;
            p->toks[t].size++;
            ws(p);
            if (p->at >= p->len) return -1;
            char c = p->s[p->at++];
            if (c == ',') continue;
            if (c == (object ? '}' : ']')) break;
            return -1;
        }
    }
    p->toks[t].end = (int)p->at;
    p->toks[t].next = p->n;
    return t;
}

static int literal(P *p, const char *word, JsonType type) {
    size_t n = strlen(word);
    if (p->at + n > p->len || memcmp(p->s + p->at, word, n)) return -1;
    int t = add(p, type, (int)p->at);
    if (t < 0) return -1;
    p->at += n;
    p->toks[t].end = (int)p->at;
    p->toks[t].next = p->n;
    return t;
}

static int number(P *p) {
    int t = add(p, JS_NUMBER, (int)p->at);
    if (t < 0) return -1;
    size_t start = p->at;
    if (p->at < p->len && p->s[p->at] == '-') p->at++;
    size_t digits = p->at;
    while (p->at < p->len && p->s[p->at] >= '0' && p->s[p->at] <= '9') p->at++;
    if (p->at == digits) return -1;
    if (p->at < p->len && p->s[p->at] == '.') {
        p->at++;
        size_t f = p->at;
        while (p->at < p->len && p->s[p->at] >= '0' && p->s[p->at] <= '9') p->at++;
        if (p->at == f) return -1;
    }
    if (p->at < p->len && (p->s[p->at] == 'e' || p->s[p->at] == 'E')) {
        p->at++;
        if (p->at < p->len && (p->s[p->at] == '+' || p->s[p->at] == '-')) p->at++;
        size_t e = p->at;
        while (p->at < p->len && p->s[p->at] >= '0' && p->s[p->at] <= '9') p->at++;
        if (p->at == e) return -1;
    }
    (void)start;
    p->toks[t].end = (int)p->at;
    p->toks[t].next = p->n;
    return t;
}

static int value(P *p, int depth) {
    ws(p);
    if (p->at >= p->len) return -1;
    char c = p->s[p->at];
    int t;
    if (c == '{' || c == '[') return container(p, depth, c == '{');
    if (c == '"') {
        t = string(p);
        if (t >= 0) p->toks[t].next = p->n;
        return t;
    }
    if (c == 't') return literal(p, "true", JS_TRUE);
    if (c == 'f') return literal(p, "false", JS_FALSE);
    if (c == 'n') return literal(p, "null", JS_NULL);
    return number(p);
}

int json_parse(const char *text, size_t len, JsonTok *toks, int cap) {
    if (!text || !toks || cap <= 0 || len > INT_MAX / 2) return -1;
    P p = {text, len, 0, toks, cap, 0};
    if (value(&p, 0) < 0) return -1;
    ws(&p);
    return p.at == len ? p.n : -1;
}

int json_get(const char *text, const JsonTok *toks, int ntok, int obj, const char *key) {
    if (obj < 0 || obj >= ntok || toks[obj].type != JS_OBJECT) return -1;
    size_t kl = strlen(key);
    int at = obj + 1;
    for (int i = 0; i < toks[obj].size && at + 1 < ntok; i++) {
        const JsonTok *k = &toks[at];
        // (A key with escapes never matches: the keys asked for are plain.)
        if ((size_t)(k->end - k->start) == kl && !memcmp(text + k->start, key, kl)) return at + 1;
        at = toks[at + 1].next;
    }
    return -1;
}

int json_item(const JsonTok *toks, int ntok, int arr, int i) {
    if (arr < 0 || arr >= ntok || toks[arr].type != JS_ARRAY || i < 0 || i >= toks[arr].size) return -1;
    int at = arr + 1;
    for (int k = 0; k < i && at < ntok; k++) at = toks[at].next;
    return at < ntok ? at : -1;
}

static size_t put_utf8(char *out, size_t at, size_t cap, unsigned cp) {
    char b[4];
    size_t n;
    if (cp < 0x80) { b[0] = (char)cp; n = 1; }
    else if (cp < 0x800) { b[0] = (char)(0xC0 | cp >> 6); b[1] = (char)(0x80 | (cp & 63)); n = 2; }
    else if (cp < 0x10000) { b[0] = (char)(0xE0 | cp >> 12); b[1] = (char)(0x80 | (cp >> 6 & 63)); b[2] = (char)(0x80 | (cp & 63)); n = 3; }
    else { b[0] = (char)(0xF0 | cp >> 18); b[1] = (char)(0x80 | (cp >> 12 & 63)); b[2] = (char)(0x80 | (cp >> 6 & 63)); b[3] = (char)(0x80 | (cp & 63)); n = 4; }
    if (at + n >= cap) return 0;  // (no room for it whole: the text ends before it)
    memcpy(out + at, b, n);
    return n;
}

static unsigned hex4(const char *s) {
    return (unsigned)(hexval(s[0]) << 12 | hexval(s[1]) << 8 | hexval(s[2]) << 4 | hexval(s[3]));
}

bool json_string(const char *text, const JsonTok *t, char *out, size_t cap) {
    if (!t || t->type != JS_STRING || !cap) return false;
    size_t o = 0;
    for (int i = t->start; i < t->end;) {
        unsigned cp;
        if (text[i] != '\\') {
            // A raw byte: copied, but a multi-byte character only whole.
            unsigned char c = (unsigned char)text[i];
            int n = c < 0x80 ? 1 : c >= 0xF0 ? 4 : c >= 0xE0 ? 3 : c >= 0xC0 ? 2 : 1;
            if (i + n > t->end || o + (size_t)n >= cap) break;
            memcpy(out + o, text + i, (size_t)n);
            o += (size_t)n;
            i += n;
            continue;
        }
        char e = text[i + 1];
        i += 2;
        if (e == 'u') {
            cp = hex4(text + i);
            i += 4;
            if (cp >= 0xD800 && cp < 0xDC00) {
                if (i + 6 <= t->end && text[i] == '\\' && text[i + 1] == 'u') {
                    unsigned lo = hex4(text + i + 2);
                    if (lo >= 0xDC00 && lo < 0xE000) {
                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                        i += 6;
                    } else {
                        cp = 0xFFFD;
                    }
                } else {
                    cp = 0xFFFD;
                }
            } else if (cp >= 0xDC00 && cp < 0xE000) {
                cp = 0xFFFD;
            }
        } else {
            cp = e == 'b' ? '\b' : e == 'f' ? '\f' : e == 'n' ? '\n' : e == 'r' ? '\r' : e == 't' ? '\t' : (unsigned char)e;
        }
        size_t n = put_utf8(out, o, cap, cp);
        if (!n) break;
        o += n;
    }
    out[o] = 0;
    return true;
}

bool json_bool(const JsonTok *t, bool *v) {
    if (!t || (t->type != JS_TRUE && t->type != JS_FALSE)) return false;
    *v = t->type == JS_TRUE;
    return true;
}

bool json_int(const char *text, const JsonTok *t, int *v) {
    if (!t || t->type != JS_NUMBER) return false;
    long long n = 0;
    int i = t->start;
    bool neg = text[i] == '-';
    if (neg) i++;
    for (; i < t->end && text[i] >= '0' && text[i] <= '9'; i++) {
        n = n * 10 + (text[i] - '0');
        if (n > INT_MAX) n = (long long)INT_MAX + 1;
    }
    if (neg) n = -n;
    *v = n > INT_MAX ? INT_MAX : n < INT_MIN ? INT_MIN : (int)n;
    return true;
}
