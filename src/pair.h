// pair.h: a short, one-use invitation exchanges a code or QR token for a browser session.
// Memory only: stopping an activity revokes its scope; stopping the server resets everything.
// Times are monotonic seconds, supplied by the caller. Nothing here logs a credential.
#pragma once
#include <stddef.h>
#include <stdint.h>

enum { PAIR_LIVE = 1, PAIR_SEND = 2 };
enum { PAIR_OK = 0, PAIR_WRONG = -1, PAIR_LOCKED = -2, PAIR_NO_ENTROPY = -3 };
#define PAIR_COOKIE_NAME "grimoire_session"
#define PAIR_TOKEN_LEN 32
#define PAIR_SESSIONS 4
#define PAIR_LIFETIME_S 300
#define PAIR_LOCK_S 30

// Returns 0, or -1 if the scopes/time are invalid or secure randomness is unavailable.
// Existing sessions survive a new invitation. A new invitation does not clear a lockout.
int pair_begin(unsigned scopes, int64_t now);
const char *pair_code(void);       // six digits, or "" when no invitation is active
const char *pair_qr_token(void);   // 128 random bits as lowercase hex, or ""
// Single use, at most five misses before rotating the invitation and locking for 30 seconds.
// tok is empty on failure. A fifth session replaces the oldest of the four sessions.
int pair_redeem(const char *credential, int64_t now, char tok[PAIR_TOKEN_LEN + 1]);
// The Cookie header's value, not a complete header line. Ambiguous/duplicate session cookies
// and malformed tokens refuse authentication. Comparisons inspect every token byte.
unsigned pair_cookie_scopes(const char *cookie);
// A complete Set-Cookie value: HttpOnly; SameSite=Strict; Path=/ (covers /live and /send).
int pair_cookie_header(const char *tok, char *out, size_t cap);
void pair_forget(unsigned scopes);
void pair_reset(void);
