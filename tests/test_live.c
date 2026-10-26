// test_live.c: Live view's tile marks and runs, its frames, and a viewer child end to end over a
// socket pair: the hello, the tiles (PNGs read back against the screen), only what changed after.
#include "live.h"
#include "testlib.h"

#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

#include "stb_image.h"

TEST(marks_cover_exactly_the_tiles_a_rect_touches) {
    uint32_t w[LIVE_WORDS] = {0};
    live_mark(w, 1860, 2480, (Rect){0, 0, 1, 1});
    CHECK(w[0] == 1u);
    memset(w, 0, sizeof w);
    live_mark(w, 1860, 2480, (Rect){63, 63, 2, 2});  // the corner of four tiles
    int across = live_tiles_across(1860);
    CHECK(across == 30);
    CHECK(w[0] == (3u | 3u << 30));  // tiles 0 and 1, and 30 and 31 (the next row's first two)
    memset(w, 0, sizeof w);
    live_mark(w, 1860, 2480, (Rect){-50, -50, 5000, 5000});  // cut to the screen: every tile, no more
    int tiles = 30 * 39, set = 0;
    for (int i = 0; i < LIVE_WORDS * 32; i++) set += (w[i / 32] >> (i % 32)) & 1u;
    CHECK(set == tiles);
    memset(w, 0, sizeof w);
    live_mark(w, 1860, 2480, (Rect){1859, 2479, 1, 1});  // the last pixel: the last tile
    CHECK((w[(tiles - 1) / 32] >> ((tiles - 1) % 32) & 1u) == 1u);
    memset(w, 0, sizeof w);
    live_mark(w, 2648, 1986, (Rect){0, 0, 2648, 1986});  // the bigger Scribe, sideways: still fits
    set = 0;
    for (int i = 0; i < LIVE_WORDS * 32; i++) set += (w[i / 32] >> (i % 32)) & 1u;
    CHECK(set == live_tiles_across(2648) * 32);
}

TEST(runs_join_neighbours_in_a_row_and_stay_on_screen) {
    uint32_t w[LIVE_WORDS] = {0};
    live_mark(w, 1860, 2480, (Rect){100, 100, 200, 10});  // tiles 1..4 of row 1
    live_mark(w, 1860, 2480, (Rect){1800, 2470, 60, 10});  // the last tile
    Rect r[8];
    int n = live_runs(w, 1860, 2480, r, 8);
    CHECK(n == 2);
    CHECK(r[0].x == 64 && r[0].y == 64 && r[0].w == 256 && r[0].h == 64);
    CHECK(r[1].x == 1792 && r[1].y == 2432 && r[1].w == 68 && r[1].h == 48);  // cut at the edges
    CHECK(live_runs(w, 1860, 2480, r, 1) == 1);  // at most cap
}

TEST(frame_headers_are_little_endian) {
    uint8_t h[5];
    live_frame_header(h, 0x010203, LIVE_TILE_FRAME);
    CHECK(h[0] == 0x04 && h[1] == 0x02 && h[2] == 0x01 && h[3] == 0 && h[4] == LIVE_TILE_FRAME);
}

// Reads one frame from fd: its type, its payload (malloc'd) and length. False at the end.
static bool read_frame(int fd, uint8_t *type, uint8_t **body, uint32_t *len) {
    uint8_t h[5];
    size_t got = 0;
    while (got < 5) {
        ssize_t r = read(fd, h + got, 5 - got);
        if (r <= 0) return false;
        got += (size_t)r;
    }
    *len = (uint32_t)h[0] | (uint32_t)h[1] << 8 | (uint32_t)h[2] << 16 | (uint32_t)h[3] << 24;
    if (*len < 1) return false;
    *len -= 1;
    *type = h[4];
    *body = malloc(*len ? *len : 1);
    for (got = 0; got < *len;) {
        ssize_t r = read(fd, *body + got, *len - got);
        if (r <= 0) {
            free(*body);
            return false;
        }
        got += (size_t)r;
    }
    return true;
}

static unsigned u16(const uint8_t *p) { return (unsigned)p[0] | (unsigned)p[1] << 8; }

// Skips the HTTP head: everything up to the blank line.
static bool skip_head(int fd) {
    char c, last[4] = {0};
    while (read(fd, &c, 1) == 1) {
        memmove(last, last + 1, 3);
        last[3] = c;
        if (!memcmp(last, "\r\n\r\n", 4)) return true;
    }
    return false;
}

// Reads frames up to the next batch end, drawing the tiles into screen (w x h); returns how many
// tiles came, or -1 when the stream ended first.
static int read_batch(int fd, uint8_t *screen, int w, int h, int *hellos) {
    int tiles = 0;
    for (;;) {
        uint8_t type, *body;
        uint32_t len;
        if (!read_frame(fd, &type, &body, &len)) return -1;
        if (type == LIVE_HELLO) {
            CHECK(len == 12 && u16(body) == 1 && u16(body + 2) == (unsigned)w && u16(body + 4) == (unsigned)h &&
                  u16(body + 6) == LIVE_TILE);
            (*hellos)++;
        } else if (type == LIVE_TILE_FRAME) {
            int x = (int)u16(body), y = (int)u16(body + 2), tw = (int)u16(body + 4), th = (int)u16(body + 6);
            int pw, ph, ch;
            uint8_t *px = stbi_load_from_memory(body + 8, (int)len - 8, &pw, &ph, &ch, 1);
            CHECK(px && pw == tw && ph == th && x + tw <= w && y + th <= h);
            for (int r = 0; px && r < th; r++) memcpy(&screen[(size_t)(y + r) * (size_t)w + (size_t)x], &px[(size_t)r * (size_t)tw], (size_t)tw);
            stbi_image_free(px);
            tiles++;
        } else if (type == LIVE_BATCH) {
            free(body);
            return tiles;
        }
        free(body);
    }
}

TEST(a_viewer_sends_the_screen_then_what_changed) {
    const int w = 300, h = 200;
    Canvas c;
    CHECK(canvas_init(&c, w, h) == 0);
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) c.px[(size_t)y * (size_t)c.stride + (size_t)x] = (uint8_t)((x * 7 + y * 3) & 0xFF);
    CHECK(live_start(w, h) == 0 && live_on());
    live_push(&c, (Rect){0, 0, w, h});
    int sv[2];
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    CHECK(live_take_viewer(sv[0]));
    close(sv[0]);
    CHECK(live_viewers() == 1);
    CHECK(skip_head(sv[1]));
    uint8_t type, *body;
    uint32_t len;
    // The state first (awake, the colors as drawn), then the hello and the screen.
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_STATE && len == 2 && body[0] == LIVE_AWAKE &&
          body[1] == 0);
    free(body);
    uint8_t *seen = calloc((size_t)w * (size_t)h, 1);
    int hellos = 0;
    int first = read_batch(sv[1], seen, w, h, &hellos);
    CHECK(hellos == 1 && first > 0);
    bool same = true;
    for (int y = 0; y < h; y++) same &= !memcmp(&seen[(size_t)y * (size_t)w], &c.px[(size_t)y * (size_t)c.stride], (size_t)w);
    CHECK(same);
    // A stroke's worth: one tile changes, one tile comes.
    canvas_fill(&c, (Rect){70, 70, 20, 20}, 0);
    live_push(&c, (Rect){70, 70, 20, 20});
    CHECK(read_batch(sv[1], seen, w, h, &hellos) == 1);
    CHECK(seen[80 * w + 80] == 0 && hellos == 1);
    // A push that changes nothing sends nothing; the next real change comes alone.
    live_push(&c, (Rect){200, 100, 50, 50});
    canvas_fill(&c, (Rect){5, 150, 10, 10}, 255);
    live_push(&c, (Rect){5, 150, 10, 10});
    CHECK(read_batch(sv[1], seen, w, h, &hellos) == 1);
    CHECK(seen[155 * w + 8] == 255);
    // Turned: a new hello, the whole screen again.
    live_resize(h, w);
    Canvas t;
    CHECK(canvas_init(&t, h, w) == 0);
    canvas_fill(&t, (Rect){0, 0, h, w}, 40);
    live_push(&t, (Rect){0, 0, h, w});
    uint8_t *turned = calloc((size_t)w * (size_t)h, 1);
    while (hellos < 2 && read_batch(sv[1], turned, h, w, &hellos) >= 0) {
    }
    CHECK(hellos == 2);
    // Bye: the viewer says so and ends; nothing is left running.
    live_stop();
    bool bye = false;
    while (read_frame(sv[1], &type, &body, &len)) {
        if (type == LIVE_STATE && len == 2 && body[0] == LIVE_BYE) bye = true;
        free(body);
    }
    CHECK(bye && !live_on() && live_viewers() == 0);
    close(sv[1]);
    free(seen);
    free(turned);
    canvas_free(&t);
    canvas_free(&c);
}

TEST(nothing_is_sent_while_the_pairing_card_is_up) {
    const int w = 128, h = 128;
    Canvas c;
    CHECK(canvas_init(&c, w, h) == 0);
    canvas_fill(&c, (Rect){0, 0, w, h}, 200);
    CHECK(live_start(w, h) == 0);
    live_push(&c, (Rect){0, 0, w, h});
    int sv[2];
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    CHECK(live_take_viewer(sv[0]));
    close(sv[0]);
    CHECK(skip_head(sv[1]));
    uint8_t *seen = calloc((size_t)w * (size_t)h, 1);
    int hellos = 0;
    CHECK(read_batch(sv[1], seen, w, h, &hellos) > 0);
    live_set_state(LIVE_PAIRING);
    uint8_t type, *body;
    uint32_t len;
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_STATE && len == 2 && body[0] == LIVE_PAIRING);
    free(body);
    canvas_fill(&c, (Rect){0, 0, 64, 64}, 0);  // the code on the card
    live_push(&c, (Rect){0, 0, 64, 64});
    canvas_fill(&c, (Rect){0, 0, 64, 64}, 200);  // the card gone, the screen as it was
    live_push(&c, (Rect){0, 0, 64, 64});
    live_set_state(LIVE_AWAKE);
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_STATE && len == 2 && body[0] == LIVE_AWAKE);
    free(body);
    // Every tile looked at again, none of the card's sent: the next batch holds the one real change.
    canvas_fill(&c, (Rect){100, 100, 10, 10}, 0);
    live_push(&c, (Rect){100, 100, 10, 10});
    CHECK(read_batch(sv[1], seen, w, h, &hellos) == 1);
    CHECK(seen[0] == 200 && seen[105 * w + 105] == 0);
    live_stop();
    close(sv[1]);
    free(seen);
    canvas_free(&c);
}

// The next frame that is not a tile or a batch end.
static bool next_state(int fd, uint8_t out[2]) {
    uint8_t type, *body;
    uint32_t len;
    while (read_frame(fd, &type, &body, &len)) {
        bool state = type == LIVE_STATE && len == 2;
        if (state) memcpy(out, body, 2);
        free(body);
        if (state) return true;
    }
    return false;
}

TEST(the_browser_is_told_when_the_colors_invert) {
    const int w = 128, h = 128;
    Canvas c;
    CHECK(canvas_init(&c, w, h) == 0);
    canvas_fill(&c, (Rect){0, 0, w, h}, 230);
    CHECK(live_start(w, h) == 0);
    live_push(&c, (Rect){0, 0, w, h});
    live_set_invert(true);  // inverted before the browser came: its first frame says so
    int sv[2];
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    CHECK(live_take_viewer(sv[0]));
    close(sv[0]);
    CHECK(skip_head(sv[1]));
    uint8_t st[2];
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE && st[1] == 1);
    uint8_t *seen = calloc((size_t)w * (size_t)h, 1);
    int hellos = 0;
    CHECK(read_batch(sv[1], seen, w, h, &hellos) > 0 && seen[0] == 230);  // (the frame as drawn)
    live_set_invert(false);
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE && st[1] == 0);
    live_set_invert(false);  // no change, nothing told: the next frames are the next change's
    canvas_fill(&c, (Rect){10, 10, 10, 10}, 0);
    live_push(&c, (Rect){10, 10, 10, 10});
    uint8_t type, *body;
    uint32_t len;
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_TILE_FRAME);
    free(body);
    live_set_invert(true);
    live_stop();  // bye, with the colors as they are (told apart first, or only with it)
    bool bye = false;
    while (next_state(sv[1], st))
        if (st[0] == LIVE_BYE) bye = st[1] == 1;
    CHECK(bye);
    close(sv[1]);
    free(seen);
    canvas_free(&c);
}

TEST(no_viewer_for_ten_minutes_turns_it_off) {
    CHECK(live_start(64, 64) == 0);
    bool idle = true;
    uint32_t t = now_ms();
    CHECK(live_poll(t, &idle) == 0 && !idle);
    CHECK(live_poll(t + LIVE_IDLE_OFF_MS - 1000, &idle) == 0 && !idle);
    CHECK(live_poll(t + LIVE_IDLE_OFF_MS + 1000, &idle) == 0 && idle);
    live_stop();
    CHECK(live_poll(t + LIVE_IDLE_OFF_MS + 1000, &idle) == 0 && !idle);  // off: nothing to turn off
}

int main(void) {
    RUN(marks_cover_exactly_the_tiles_a_rect_touches);
    RUN(runs_join_neighbours_in_a_row_and_stay_on_screen);
    RUN(frame_headers_are_little_endian);
    RUN(a_viewer_sends_the_screen_then_what_changed);
    RUN(nothing_is_sent_while_the_pairing_card_is_up);
    RUN(the_browser_is_told_when_the_colors_invert);
    RUN(no_viewer_for_ten_minutes_turns_it_off);
    return tl_done();
}
