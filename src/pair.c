// pair.c: invitations and browser sessions. The HTTP parent owns this state, before any fork.
#include "pair.h"

#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static struct {
    unsigned scopes, misses, next;
    int64_t issued, expires, locked_until;
    char code[7], qr[PAIR_TOKEN_LEN + 1];
    struct {
        char token[PAIR_TOKEN_LEN + 1];
        unsigned scopes;
    } sessions[PAIR_SESSIONS];
} S;

static bool random_bytes(void *out, size_t n) {
    int fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    unsigned char *p = out;
    size_t left = n;
    while (left) {
        ssize_t got = read(fd, p, left);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) { close(fd); return false; }
        p += got;
        left -= (size_t)got;
    }
    close(fd);
    return true;
}

static bool random_token(char out[PAIR_TOKEN_LEN + 1]) {
    unsigned char bytes[16];
    if (!random_bytes(bytes, sizeof bytes)) return false;
    const char *hex = "0123456789abcdef";
    for (int i = 0; i < 16; i++) {
        out[i * 2] = hex[bytes[i] >> 4];
        out[i * 2 + 1] = hex[bytes[i] & 15];
    }
    out[PAIR_TOKEN_LEN] = 0;
    return true;
}

static bool token_ok(const char *s) {
    if (!s || strnlen(s, PAIR_TOKEN_LEN + 1) != PAIR_TOKEN_LEN) return false;
    for (int i = 0; i < PAIR_TOKEN_LEN; i++)
        if (!(s[i] >= '0' && s[i] <= '9') && !(s[i] >= 'a' && s[i] <= 'f')) return false;
    return true;
}

static bool equal(const char *a, const char *b, size_t n) {
    volatile unsigned difference = 0;
    for (size_t i = 0; i < n; i++) difference |= (unsigned char)a[i] ^ (unsigned char)b[i];
    return difference == 0;
}

int pair_begin(unsigned scopes, int64_t now) {
    char old[7];
    memcpy(old, S.code, sizeof old);
    S.scopes = 0;
    S.code[0] = S.qr[0] = 0;
    if (!scopes || (scopes & ~(PAIR_LIVE | PAIR_SEND)) || now < 0 || now > INT64_MAX - PAIR_LIFETIME_S) return -1;
    char qr[PAIR_TOKEN_LEN + 1], code[7];
    uint32_t value;
    if (!random_token(qr)) return -1;
    // Rejection sampling gives each of the million codes the same chance.
    do {
        do { if (!random_bytes(&value, sizeof value)) return -1; } while (value >= UINT32_MAX - UINT32_MAX % 1000000u);
        snprintf(code, sizeof code, "%06u", value % 1000000u);
    } while (!memcmp(code, old, sizeof code));  // rotation must really change the code
    memcpy(S.code, code, sizeof code);
    memcpy(S.qr, qr, sizeof qr);
    S.scopes = scopes;
    S.issued = now;
    S.expires = now + PAIR_LIFETIME_S;
    return 0;
}

const char *pair_code(void) { return S.scopes ? S.code : ""; }
const char *pair_qr_token(void) { return S.scopes ? S.qr : ""; }

int pair_redeem(const char *credential, int64_t now, char tok[PAIR_TOKEN_LEN + 1]) {
    if (!tok) return PAIR_WRONG;
    tok[0] = 0;
    if (now < 0 || now > INT64_MAX - PAIR_LIFETIME_S) return PAIR_WRONG;
    if (now < S.locked_until) return PAIR_LOCKED;
    if (!S.scopes || now < S.issued || now >= S.expires) return PAIR_WRONG;
    size_t n = credential ? strnlen(credential, PAIR_TOKEN_LEN + 1) : 0;
    bool matches = n == 6 ? equal(credential, S.code, 6) : n == PAIR_TOKEN_LEN && equal(credential, S.qr, PAIR_TOKEN_LEN);
    if (!matches) {
        if (++S.misses >= 5) {
            unsigned scopes = S.scopes;
            S.locked_until = now + PAIR_LOCK_S;
            S.misses = 0;
            if (pair_begin(scopes, now) != 0) return PAIR_NO_ENTROPY;
            return PAIR_LOCKED;
        }
        return PAIR_WRONG;
    }
    if (!random_token(tok)) return PAIR_NO_ENTROPY;
    unsigned i = S.next++ % PAIR_SESSIONS;
    memcpy(S.sessions[i].token, tok, PAIR_TOKEN_LEN + 1);
    S.sessions[i].scopes = S.scopes;
    S.scopes = S.misses = 0;
    S.code[0] = S.qr[0] = 0;
    return PAIR_OK;
}

unsigned pair_cookie_scopes(const char *cookie) {
    if (!cookie || strnlen(cookie, 4097) > 4096 || strpbrk(cookie, "\r\n")) return 0;
    const char *token = NULL;
    size_t keylen = sizeof PAIR_COOKIE_NAME - 1;
    for (const char *p = cookie; *p;) {
        while (*p == ' ' || *p == '\t' || *p == ';') p++;
        const char *end = strchr(p, ';');
        if (!end) end = p + strlen(p);
        const char *eq = memchr(p, '=', (size_t)(end - p));
        if (eq && (size_t)(eq - p) == keylen && !memcmp(p, PAIR_COOKIE_NAME, keylen)) {
            while (end > eq + 1 && (end[-1] == ' ' || end[-1] == '\t')) end--;
            if (token || end - eq - 1 != PAIR_TOKEN_LEN) return 0;
            token = eq + 1;
        }
        p = strchr(p, ';');
        if (!p) break;
        p++;
    }
    if (!token) return 0;
    char copy[PAIR_TOKEN_LEN + 1];
    memcpy(copy, token, PAIR_TOKEN_LEN);
    copy[PAIR_TOKEN_LEN] = 0;
    if (!token_ok(copy)) return 0;
    unsigned scopes = 0;
    for (unsigned i = 0; i < PAIR_SESSIONS; i++)
        scopes |= S.sessions[i].scopes * (unsigned)equal(copy, S.sessions[i].token, PAIR_TOKEN_LEN);
    return scopes;
}

int pair_cookie_header(const char *tok, char *out, size_t cap) {
    if (!out || !cap) return -1;
    out[0] = 0;
    if (!token_ok(tok)) return -1;
    int n = snprintf(out, cap, PAIR_COOKIE_NAME "=%s; HttpOnly; SameSite=Strict; Path=/", tok);
    if (n < 0 || (size_t)n >= cap) { out[0] = 0; return -1; }
    return 0;
}

void pair_forget(unsigned scopes) {
    S.scopes &= ~scopes;
    if (!S.scopes) S.code[0] = S.qr[0] = 0;
    for (unsigned i = 0; i < PAIR_SESSIONS; i++) {
        S.sessions[i].scopes &= ~scopes;
        if (!S.sessions[i].scopes) memset(S.sessions[i].token, 0, sizeof S.sessions[i].token);
    }
}

void pair_reset(void) { memset(&S, 0, sizeof S); }
