//! pair.h: a short, one-use invitation exchanges a code or QR token for a browser session.
//! Memory only: stopping an activity revokes its scope; stopping the server resets everything.
//! Times are monotonic seconds, supplied by the caller. Nothing here logs a credential.

use crate::prelude::*;

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

/// Returns 0, or -1 if the scopes/time are invalid or secure randomness is unavailable.
/// Existing sessions survive a new invitation. A new invitation does not clear a lockout.
pub fn pair_begin(scopes: u32, now: i64) -> i32 {
    todo!()
}
/// six digits, or "" when no invitation is active
pub fn pair_code() -> String {
    todo!()
}
/// 128 random bits as lowercase hex, or ""
pub fn pair_qr_token() -> String {
    todo!()
}
/// Single use, at most five misses before rotating the invitation and locking for 30 seconds.
/// tok is empty on failure. A fifth session replaces the oldest of the four sessions.
pub fn pair_redeem(credential: &str, now: i64, tok: &mut String /* PAIR_TOKEN_LEN chars */) -> i32 {
    todo!()
}
/// The Cookie header's value, not a complete header line. Ambiguous/duplicate session cookies
/// and malformed tokens refuse authentication. Comparisons inspect every token byte.
pub fn pair_cookie_scopes(cookie: &str) -> u32 {
    todo!()
}
/// A complete Set-Cookie value: HttpOnly; SameSite=Strict; Path=/ (covers /live and /send).
pub fn pair_cookie_header(tok: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
pub fn pair_forget(scopes: u32) {
    todo!()
}
/// Drops the invitation shown (the pairing card closed), keeping the sessions and any lockout.
pub fn pair_forget_invitation() {
    todo!()
}
pub fn pair_reset() {
    todo!()
}
