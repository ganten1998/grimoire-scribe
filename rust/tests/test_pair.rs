// test_pair.c: invitations, browser sessions, the lockout and the Cookie header.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn cookie(tok: &str) -> String {
    format!("{}={}", PAIR_COOKIE_NAME, tok)
}

fn invitation_is_single_use_and_scoped() {
    pair_reset();
    check!(pair_code().is_empty() && pair_qr_token().is_empty());
    check!(pair_begin(PAIR_LIVE, 100) == 0);
    check!(pair_code().len() == 6 && pair_code().bytes().all(|b| b.is_ascii_digit()));
    check!(pair_qr_token().len() == PAIR_TOKEN_LEN);
    let code = pair_code();
    let qr = pair_qr_token();
    let mut tok = String::new();
    check!(pair_redeem(&code, 101, &mut tok) == PAIR_OK);
    check!(tok.len() == PAIR_TOKEN_LEN && tok != qr);
    let mut hdr = cookie(&tok);
    check!(pair_cookie_scopes(&hdr) == PAIR_LIVE);
    check!(pair_code().is_empty() && pair_qr_token().is_empty());
    check!(pair_redeem(&code, 102, &mut tok) == PAIR_WRONG && tok.is_empty());
    check!(pair_redeem(&qr, 102, &mut tok) == PAIR_WRONG && tok.is_empty());
    check!(pair_begin(PAIR_SEND, 103) == 0);
    let qr = pair_qr_token();
    check!(pair_redeem(&qr, 104, &mut tok) == PAIR_OK);
    check!(pair_cookie_scopes(&hdr) == PAIR_LIVE); // starting another invitation keeps sessions
    hdr = cookie(&tok);
    check!(pair_cookie_scopes(&hdr) == PAIR_SEND);
    pair_forget(PAIR_LIVE);
    check!(pair_cookie_scopes(&hdr) == PAIR_SEND);
    pair_reset();
    check!(pair_cookie_scopes(&hdr) == 0);
}

fn expiry_clock_and_invalid_input() {
    pair_reset();
    let mut tok = String::new();
    check!(pair_begin(0, 0) != 0 && pair_code().is_empty());
    check!(pair_begin(4, 0) != 0 && pair_code().is_empty());
    check!(pair_begin(PAIR_LIVE, -1) != 0);
    check!(pair_begin(PAIR_LIVE, i64::MAX) != 0);
    check!(pair_begin(PAIR_LIVE, 1000) == 0);
    let code = pair_code();
    check!(pair_redeem(&code, 999, &mut tok) == PAIR_WRONG && tok.is_empty());
    check!(pair_redeem(&code, 1300, &mut tok) == PAIR_WRONG && tok.is_empty());
    check!(pair_redeem(&code, i64::MAX, &mut tok) == PAIR_WRONG && tok.is_empty());
    check!(pair_begin(PAIR_LIVE, 1400) == 0);
    let code = pair_code();
    check!(pair_redeem(&code, 1699, &mut tok) == PAIR_OK);
    // (The C also passes NULL as the credential and as the token: Rust has no NULL string, so the
    // missing credential is the empty one, and there is no missing token to refuse.)
    check!(pair_redeem("", 1699, &mut tok) == PAIR_WRONG && tok.is_empty());
}

fn five_misses_rotate_and_lock() {
    pair_reset();
    check!(pair_begin(PAIR_LIVE, 100) == 0);
    let mut tok = String::new();
    let mut code = pair_code();
    let qr = pair_qr_token();
    for i in 0..4 {
        check!(pair_redeem("bad", 101 + i, &mut tok) == PAIR_WRONG && tok.is_empty());
    }
    check!(pair_redeem("bad", 105, &mut tok) == PAIR_LOCKED && tok.is_empty());
    check!(code != pair_code() && qr != pair_qr_token());
    check!(pair_redeem(&pair_code(), 134, &mut tok) == PAIR_LOCKED && tok.is_empty());
    // A refresh of the pairing card must not bypass the lockout.
    code = pair_code();
    check!(pair_begin(PAIR_SEND, 110) == 0);
    check!(pair_redeem(&pair_code(), 134, &mut tok) == PAIR_LOCKED && tok.is_empty());
    check!(pair_redeem(&code, 135, &mut tok) == PAIR_WRONG && tok.is_empty());
    check!(pair_redeem(&qr, 135, &mut tok) == PAIR_WRONG && tok.is_empty());
    let fresh = pair_code();
    check!(pair_redeem(&fresh, 135, &mut tok) == PAIR_OK);
    let hdr = cookie(&tok);
    check!(pair_cookie_scopes(&hdr) == PAIR_SEND);
}

fn cookies_are_exact_and_unambiguous() {
    pair_reset();
    check!(pair_begin(PAIR_LIVE | PAIR_SEND, 0) == 0);
    let mut tok = String::new();
    let code = pair_code();
    check!(pair_redeem(&code, 1, &mut tok) == PAIR_OK);
    let hdr = cookie(&tok);
    check!(pair_cookie_scopes(&hdr) == (PAIR_LIVE | PAIR_SEND));
    let mut input = format!("other=a;\t{}; another=b", hdr);
    check!(pair_cookie_scopes(&input) == (PAIR_LIVE | PAIR_SEND));
    input = format!("{}  ;", hdr);
    check!(pair_cookie_scopes(&input) == (PAIR_LIVE | PAIR_SEND));
    input = format!("{}; {}", hdr, hdr);
    check!(pair_cookie_scopes(&input) == 0);
    input = format!("{}; {}=bad", hdr, PAIR_COOKIE_NAME);
    check!(pair_cookie_scopes(&input) == 0);
    input = format!("x{}", hdr);
    check!(pair_cookie_scopes(&input) == 0);
    input = format!("{}x", hdr);
    check!(pair_cookie_scopes(&input) == 0);
    input = format!("{}\r\nX-Extra: bad", hdr);
    check!(pair_cookie_scopes(&input) == 0);
    // (The C also asks with NULL: Rust has no NULL string.)
    check!(pair_cookie_scopes("") == 0);
    // A mismatch at every position must refuse authentication.
    for i in 0..PAIR_TOKEN_LEN {
        let mut changed = tok.clone().into_bytes();
        changed[i] = if changed[i] == b'0' { b'1' } else { b'0' };
        input = cookie(&String::from_utf8(changed).unwrap());
        check!(pair_cookie_scopes(&input) == 0);
    }
    let large = "x".repeat(4199);
    check!(pair_cookie_scopes(&large) == 0);
    check!(pair_cookie_header(&tok, &mut input, 512) == 0);
    check!(input.contains("HttpOnly") && input.contains("SameSite=Strict") && input.contains("Path=/"));
    check!(pair_cookie_header(&tok, &mut input, 10) != 0 && input.is_empty());
    check!(pair_cookie_header("bad", &mut input, 512) != 0 && input.is_empty());
    pair_forget(PAIR_LIVE);
    check!(pair_cookie_scopes(&hdr) == PAIR_SEND);
    pair_forget(PAIR_SEND);
    check!(pair_cookie_scopes(&hdr) == 0);
}

fn session_limit_and_revocation() {
    pair_reset();
    let mut saved = vec![String::new(); PAIR_SESSIONS + 1];
    for i in 0..=PAIR_SESSIONS {
        check!(pair_begin(PAIR_LIVE | PAIR_SEND, i as i64 * 2) == 0);
        let code = pair_code();
        check!(pair_redeem(&code, i as i64 * 2 + 1, &mut saved[i]) == PAIR_OK);
    }
    let mut hdr = cookie(&saved[0]);
    check!(pair_cookie_scopes(&hdr) == 0);
    for i in 1..=PAIR_SESSIONS {
        hdr = cookie(&saved[i]);
        check!(pair_cookie_scopes(&hdr) == (PAIR_LIVE | PAIR_SEND));
    }
    check!(pair_begin(PAIR_LIVE, 20) == 0);
    pair_forget(PAIR_LIVE);
    check!(pair_code().is_empty() && pair_qr_token().is_empty());
    check!(pair_cookie_scopes(&hdr) == PAIR_SEND);
    pair_reset();
    check!(pair_cookie_scopes(&hdr) == 0);
}

#[test]
fn test_pair() {
    run!(invitation_is_single_use_and_scoped);
    run!(expiry_clock_and_invalid_input);
    run!(five_misses_rotate_and_lock);
    run!(cookies_are_exact_and_unambiguous);
    run!(session_limit_and_revocation);
    done();
}
