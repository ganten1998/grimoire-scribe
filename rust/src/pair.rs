//! pair.h: a short, one-use invitation exchanges a code or QR token for a browser session.
//! Memory only: stopping an activity revokes its scope; stopping the server resets everything.
//! Times are monotonic seconds, supplied by the caller. Nothing here logs a credential.

use crate::prelude::*;
use std::cell::RefCell;

pub const PAIR_LIVE: u32 = 1;
pub const PAIR_SEND: u32 = 2;
pub const PAIR_OK: i32 = 0;
pub const PAIR_WRONG: i32 = -1;
pub const PAIR_LOCKED: i32 = -2;
pub const PAIR_NO_ENTROPY: i32 = -3;
pub const PAIR_COOKIE_NAME: &str = "grimoire_session";
pub const PAIR_TOKEN_LEN: usize = 32;
pub const PAIR_SESSIONS: usize = 4;
pub const PAIR_LIFETIME_S: i64 = 300;
pub const PAIR_LOCK_S: i64 = 30;

// pair.c: invitations and browser sessions. The HTTP parent owns this state, before any fork.
#[derive(Clone, Copy)]
struct Session {
    token: [u8; PAIR_TOKEN_LEN + 1],
    scopes: u32,
}

impl Default for Session {
    fn default() -> Session {
        Session { token: [0; PAIR_TOKEN_LEN + 1], scopes: 0 }
    }
}

#[derive(Clone, Copy)]
struct State {
    scopes: u32,
    misses: u32,
    next: u32,
    issued: i64,
    expires: i64,
    locked_until: i64,
    code: [u8; 7],
    qr: [u8; PAIR_TOKEN_LEN + 1],
    sessions: [Session; PAIR_SESSIONS],
}

impl Default for State {
    fn default() -> State {
        State {
            scopes: 0,
            misses: 0,
            next: 0,
            issued: 0,
            expires: 0,
            locked_until: 0,
            code: [0; 7],
            qr: [0; PAIR_TOKEN_LEN + 1],
            sessions: [Session::default(); PAIR_SESSIONS],
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

/// Fills out from /dev/urandom; false if it cannot (nothing is used then).
fn random_bytes(out: &mut [u8]) -> bool {
    use std::io::Read;
    // (std opens it close-on-exec, as the C asks with O_CLOEXEC; read_exact retries EINTR and
    // fails on a short read or an error, as the C's loop does.)
    match std::fs::File::open("/dev/urandom") {
        Ok(mut f) => f.read_exact(out).is_ok(),
        Err(_) => false,
    }
}

fn random_token(out: &mut [u8; PAIR_TOKEN_LEN + 1]) -> bool {
    let mut bytes = [0u8; 16];
    if !random_bytes(&mut bytes) {
        return false;
    }
    let hex = b"0123456789abcdef";
    for i in 0..16 {
        out[i * 2] = hex[(bytes[i] >> 4) as usize];
        out[i * 2 + 1] = hex[(bytes[i] & 15) as usize];
    }
    out[PAIR_TOKEN_LEN] = 0;
    true
}

/// strnlen(s, max) of a Rust string (a C string ends at its first NUL).
fn strnlen(s: &[u8], max: usize) -> usize {
    s.iter().take(max).position(|&b| b == 0).unwrap_or(s.len().min(max))
}

fn token_ok(s: &[u8]) -> bool {
    if strnlen(s, PAIR_TOKEN_LEN + 1) != PAIR_TOKEN_LEN {
        return false;
    }
    s[..PAIR_TOKEN_LEN].iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
}

/// Constant time: every one of the n bytes is looked at, whatever the first difference.
fn equal(a: &[u8], b: &[u8], n: usize) -> bool {
    let mut difference: u32 = 0;
    for i in 0..n {
        difference = std::hint::black_box(difference | (a[i] ^ b[i]) as u32);
    }
    difference == 0
}

fn begin(s: &mut State, scopes: u32, now: i64) -> i32 {
    let old = s.code;
    s.scopes = 0;
    s.code[0] = 0;
    s.qr[0] = 0;
    if scopes == 0 || (scopes & !(PAIR_LIVE | PAIR_SEND)) != 0 || !(0..=i64::MAX - PAIR_LIFETIME_S).contains(&now) {
        return -1;
    }
    let mut qr = [0u8; PAIR_TOKEN_LEN + 1];
    let mut code;
    if !random_token(&mut qr) {
        return -1;
    }
    // Rejection sampling gives each of the million codes the same chance.
    loop {
        let mut value;
        loop {
            let mut b = [0u8; 4];
            if !random_bytes(&mut b) {
                return -1;
            }
            value = u32::from_ne_bytes(b);
            if value < u32::MAX - u32::MAX % 1000000 {
                break;
            }
        }
        code = [0u8; 7];
        let digits = cformat!("%06u", value % 1000000);
        code[..6].copy_from_slice(digits.as_bytes());
        if code != old {
            break; // rotation must really change the code
        }
    }
    s.code = code;
    s.qr = qr;
    s.scopes = scopes;
    s.issued = now;
    s.expires = now + PAIR_LIFETIME_S;
    0
}

/// Returns 0, or -1 if the scopes/time are invalid or secure randomness is unavailable.
/// Existing sessions survive a new invitation. A new invitation does not clear a lockout.
pub fn pair_begin(scopes: u32, now: i64) -> i32 {
    with(|s| begin(s, scopes, now))
}

/// six digits, or "" when no invitation is active
pub fn pair_code() -> String {
    with(|s| if s.scopes != 0 { text(&s.code) } else { String::new() })
}

/// 128 random bits as lowercase hex, or ""
pub fn pair_qr_token() -> String {
    with(|s| if s.scopes != 0 { text(&s.qr) } else { String::new() })
}

/// A C string in a buffer, as a String (the codes and tokens are ASCII).
fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]).into_owned()
}

/// Single use, at most five misses before rotating the invitation and locking for 30 seconds.
/// tok is empty on failure. A fifth session replaces the oldest of the four sessions.
pub fn pair_redeem(credential: &str, now: i64, tok: &mut String /* PAIR_TOKEN_LEN chars */) -> i32 {
    tok.clear();
    with(|s| {
        if !(0..=i64::MAX - PAIR_LIFETIME_S).contains(&now) {
            return PAIR_WRONG;
        }
        if now < s.locked_until {
            return PAIR_LOCKED;
        }
        if s.scopes == 0 || now < s.issued || now >= s.expires {
            return PAIR_WRONG;
        }
        let c = credential.as_bytes();
        let n = strnlen(c, PAIR_TOKEN_LEN + 1);
        let matches = if n == 6 {
            equal(c, &s.code, 6)
        } else {
            n == PAIR_TOKEN_LEN && equal(c, &s.qr, PAIR_TOKEN_LEN)
        };
        if !matches {
            s.misses += 1;
            if s.misses >= 5 {
                let scopes = s.scopes;
                s.locked_until = now + PAIR_LOCK_S;
                s.misses = 0;
                if begin(s, scopes, now) != 0 {
                    return PAIR_NO_ENTROPY;
                }
                return PAIR_LOCKED;
            }
            return PAIR_WRONG;
        }
        let mut t = [0u8; PAIR_TOKEN_LEN + 1];
        if !random_token(&mut t) {
            return PAIR_NO_ENTROPY;
        }
        *tok = text(&t);
        let i = (s.next % PAIR_SESSIONS as u32) as usize;
        s.next = s.next.wrapping_add(1);
        s.sessions[i].token = t;
        s.sessions[i].scopes = s.scopes;
        s.scopes = 0;
        s.misses = 0;
        s.code[0] = 0;
        s.qr[0] = 0;
        PAIR_OK
    })
}

/// The Cookie header's value, not a complete header line. Ambiguous/duplicate session cookies
/// and malformed tokens refuse authentication. Comparisons inspect every token byte.
pub fn pair_cookie_scopes(cookie: &str) -> u32 {
    let cookie = cookie.as_bytes();
    let cookie = &cookie[..cookie.iter().position(|&b| b == 0).unwrap_or(cookie.len())];
    if cookie.len() > 4096 || cookie.iter().any(|&b| b == b'\r' || b == b'\n') {
        return 0;
    }
    let mut token: Option<&[u8]> = None;
    let key = PAIR_COOKIE_NAME.as_bytes();
    let keylen = key.len();
    let mut p = 0usize;
    while p < cookie.len() {
        while p < cookie.len() && (cookie[p] == b' ' || cookie[p] == b'\t' || cookie[p] == b';') {
            p += 1;
        }
        let mut end = cookie[p..].iter().position(|&b| b == b';').map_or(cookie.len(), |x| p + x);
        let eq = cookie[p..end].iter().position(|&b| b == b'=').map(|x| p + x);
        if let Some(eq) = eq {
            if eq - p == keylen && &cookie[p..p + keylen] == key {
                while end > eq + 1 && (cookie[end - 1] == b' ' || cookie[end - 1] == b'\t') {
                    end -= 1;
                }
                if token.is_some() || end - eq - 1 != PAIR_TOKEN_LEN {
                    return 0;
                }
                token = Some(&cookie[eq + 1..end]);
            }
        }
        match cookie[p..].iter().position(|&b| b == b';') {
            Some(x) => p += x + 1,
            None => break,
        }
    }
    let Some(token) = token else { return 0 };
    let mut copy = [0u8; PAIR_TOKEN_LEN + 1];
    copy[..PAIR_TOKEN_LEN].copy_from_slice(token);
    if !token_ok(&copy) {
        return 0;
    }
    with(|s| {
        let mut scopes = 0u32;
        for i in 0..PAIR_SESSIONS {
            scopes |= s.sessions[i].scopes * equal(&copy, &s.sessions[i].token, PAIR_TOKEN_LEN) as u32;
        }
        scopes
    })
}

/// A complete Set-Cookie value: HttpOnly; SameSite=Strict; Path=/ (covers /live and /send).
pub fn pair_cookie_header(tok: &str, out: &mut String, cap: usize) -> i32 {
    if cap == 0 {
        return -1;
    }
    out.clear();
    if !token_ok(tok.as_bytes()) {
        return -1;
    }
    // (The C writes PAIR_COOKIE_NAME into the format; the text is the same.)
    let n = csnprintf!(out, cap, "%s=%s; HttpOnly; SameSite=Strict; Path=/", PAIR_COOKIE_NAME, tok);
    if n < 0 || n as usize >= cap {
        out.clear();
        return -1;
    }
    0
}

pub fn pair_forget(scopes: u32) {
    with(|s| {
        s.scopes &= !scopes;
        if s.scopes == 0 {
            s.code[0] = 0;
            s.qr[0] = 0;
        }
        for i in 0..PAIR_SESSIONS {
            s.sessions[i].scopes &= !scopes;
            if s.sessions[i].scopes == 0 {
                s.sessions[i].token = [0; PAIR_TOKEN_LEN + 1];
            }
        }
    })
}

/// Drops the invitation shown (the pairing card closed), keeping the sessions and any lockout.
pub fn pair_forget_invitation() {
    with(|s| {
        s.scopes = 0;
        s.misses = 0;
        s.code[0] = 0;
        s.qr[0] = 0;
    })
}

pub fn pair_reset() {
    with(|s| *s = State::default())
}
