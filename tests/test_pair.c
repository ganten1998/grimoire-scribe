#include "pair.h"
#include "testlib.h"

static void cookie(const char *tok, char out[128]) {
    snprintf(out, 128, PAIR_COOKIE_NAME "=%s", tok);
}

TEST(invitation_is_single_use_and_scoped) {
    pair_reset();
    CHECK(!*pair_code() && !*pair_qr_token());
    CHECK(pair_begin(PAIR_LIVE, 100) == 0);
    CHECK(strlen(pair_code()) == 6 && strspn(pair_code(), "0123456789") == 6);
    CHECK(strlen(pair_qr_token()) == PAIR_TOKEN_LEN);
    char code[7], qr[33], tok[33], hdr[128];
    strcpy(code, pair_code());
    strcpy(qr, pair_qr_token());
    CHECK(pair_redeem(code, 101, tok) == PAIR_OK);
    CHECK(strlen(tok) == PAIR_TOKEN_LEN && strcmp(tok, qr));
    cookie(tok, hdr);
    CHECK(pair_cookie_scopes(hdr) == PAIR_LIVE);
    CHECK(!*pair_code() && !*pair_qr_token());
    CHECK(pair_redeem(code, 102, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_redeem(qr, 102, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_begin(PAIR_SEND, 103) == 0);
    strcpy(qr, pair_qr_token());
    CHECK(pair_redeem(qr, 104, tok) == PAIR_OK);
    CHECK(pair_cookie_scopes(hdr) == PAIR_LIVE);  // starting another invitation keeps sessions
    cookie(tok, hdr);
    CHECK(pair_cookie_scopes(hdr) == PAIR_SEND);
    pair_forget(PAIR_LIVE);
    CHECK(pair_cookie_scopes(hdr) == PAIR_SEND);
    pair_reset();
    CHECK(pair_cookie_scopes(hdr) == 0);
}

TEST(expiry_clock_and_invalid_input) {
    pair_reset();
    char code[7], tok[33];
    CHECK(pair_begin(0, 0) != 0 && !*pair_code());
    CHECK(pair_begin(4, 0) != 0 && !*pair_code());
    CHECK(pair_begin(PAIR_LIVE, -1) != 0);
    CHECK(pair_begin(PAIR_LIVE, INT64_MAX) != 0);
    CHECK(pair_begin(PAIR_LIVE, 1000) == 0);
    strcpy(code, pair_code());
    CHECK(pair_redeem(code, 999, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_redeem(code, 1300, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_redeem(code, INT64_MAX, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_begin(PAIR_LIVE, 1400) == 0);
    strcpy(code, pair_code());
    CHECK(pair_redeem(code, 1699, tok) == PAIR_OK);
    CHECK(pair_redeem(NULL, 1699, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_redeem(NULL, 1699, NULL) == PAIR_WRONG);
}

TEST(five_misses_rotate_and_lock) {
    pair_reset();
    CHECK(pair_begin(PAIR_LIVE, 100) == 0);
    char code[7], qr[33], tok[33];
    strcpy(code, pair_code());
    strcpy(qr, pair_qr_token());
    for (int i = 0; i < 4; i++) CHECK(pair_redeem("bad", 101 + i, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_redeem("bad", 105, tok) == PAIR_LOCKED && !*tok);
    CHECK(strcmp(code, pair_code()) && strcmp(qr, pair_qr_token()));
    CHECK(pair_redeem(pair_code(), 134, tok) == PAIR_LOCKED && !*tok);
    // A refresh of the pairing card must not bypass the lockout.
    strcpy(code, pair_code());
    CHECK(pair_begin(PAIR_SEND, 110) == 0);
    CHECK(pair_redeem(pair_code(), 134, tok) == PAIR_LOCKED && !*tok);
    CHECK(pair_redeem(code, 135, tok) == PAIR_WRONG && !*tok);
    CHECK(pair_redeem(qr, 135, tok) == PAIR_WRONG && !*tok);
    char fresh[7], hdr[128];
    strcpy(fresh, pair_code());
    CHECK(pair_redeem(fresh, 135, tok) == PAIR_OK);
    cookie(tok, hdr);
    CHECK(pair_cookie_scopes(hdr) == PAIR_SEND);
}

TEST(cookies_are_exact_and_unambiguous) {
    pair_reset();
    CHECK(pair_begin(PAIR_LIVE | PAIR_SEND, 0) == 0);
    char code[7], tok[33], hdr[128], input[512];
    strcpy(code, pair_code());
    CHECK(pair_redeem(code, 1, tok) == PAIR_OK);
    cookie(tok, hdr);
    CHECK(pair_cookie_scopes(hdr) == (PAIR_LIVE | PAIR_SEND));
    snprintf(input, sizeof input, "other=a;\t%s; another=b", hdr);
    CHECK(pair_cookie_scopes(input) == (PAIR_LIVE | PAIR_SEND));
    snprintf(input, sizeof input, "%s  ;", hdr);
    CHECK(pair_cookie_scopes(input) == (PAIR_LIVE | PAIR_SEND));
    snprintf(input, sizeof input, "%s; %s", hdr, hdr);
    CHECK(pair_cookie_scopes(input) == 0);
    snprintf(input, sizeof input, "%s; " PAIR_COOKIE_NAME "=bad", hdr);
    CHECK(pair_cookie_scopes(input) == 0);
    snprintf(input, sizeof input, "x%s", hdr);
    CHECK(pair_cookie_scopes(input) == 0);
    snprintf(input, sizeof input, "%sx", hdr);
    CHECK(pair_cookie_scopes(input) == 0);
    snprintf(input, sizeof input, "%s\r\nX-Extra: bad", hdr);
    CHECK(pair_cookie_scopes(input) == 0);
    CHECK(pair_cookie_scopes(NULL) == 0 && pair_cookie_scopes("") == 0);
    // A mismatch at every position must refuse authentication.
    for (int i = 0; i < PAIR_TOKEN_LEN; i++) {
        char changed[33];
        strcpy(changed, tok);
        changed[i] = changed[i] == '0' ? '1' : '0';
        cookie(changed, input);
        CHECK(pair_cookie_scopes(input) == 0);
    }
    char large[4200];
    memset(large, 'x', sizeof large - 1);
    large[sizeof large - 1] = 0;
    CHECK(pair_cookie_scopes(large) == 0);
    CHECK(pair_cookie_header(tok, input, sizeof input) == 0);
    CHECK(strstr(input, "HttpOnly") && strstr(input, "SameSite=Strict") && strstr(input, "Path=/"));
    CHECK(pair_cookie_header(tok, input, 10) != 0 && !*input);
    CHECK(pair_cookie_header("bad", input, sizeof input) != 0 && !*input);
    pair_forget(PAIR_LIVE);
    CHECK(pair_cookie_scopes(hdr) == PAIR_SEND);
    pair_forget(PAIR_SEND);
    CHECK(pair_cookie_scopes(hdr) == 0);
}

TEST(session_limit_and_revocation) {
    pair_reset();
    char saved[PAIR_SESSIONS + 1][33], hdr[128];
    for (int i = 0; i <= PAIR_SESSIONS; i++) {
        CHECK(pair_begin(PAIR_LIVE | PAIR_SEND, i * 2) == 0);
        char code[7];
        strcpy(code, pair_code());
        CHECK(pair_redeem(code, i * 2 + 1, saved[i]) == PAIR_OK);
    }
    cookie(saved[0], hdr);
    CHECK(pair_cookie_scopes(hdr) == 0);
    for (int i = 1; i <= PAIR_SESSIONS; i++) {
        cookie(saved[i], hdr);
        CHECK(pair_cookie_scopes(hdr) == (PAIR_LIVE | PAIR_SEND));
    }
    CHECK(pair_begin(PAIR_LIVE, 20) == 0);
    pair_forget(PAIR_LIVE);
    CHECK(!*pair_code() && !*pair_qr_token());
    CHECK(pair_cookie_scopes(hdr) == PAIR_SEND);
    pair_reset();
    CHECK(pair_cookie_scopes(hdr) == 0);
}

int main(void) {
    RUN(invitation_is_single_use_and_scoped);
    RUN(expiry_clock_and_invalid_input);
    RUN(five_misses_rotate_and_lock);
    RUN(cookies_are_exact_and_unambiguous);
    RUN(session_limit_and_revocation);
    return tl_done();
}
