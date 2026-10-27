// test_live.c: Live view's tile marks and runs, its frames, and a viewer child end to end over a
// socket pair: the hello, the tiles (PNGs read back against the screen), only what changed after,
// and the pen's place after them.
#include "live.h"
#include "testlib.h"

#include <math.h>
#include <poll.h>
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
    // The state first (awake, the colors as drawn, no warmth known), then the hello and the screen.
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_STATE && len == 3 && body[0] == LIVE_AWAKE &&
          body[1] == 0 && body[2] == LIVE_NO_WARMTH);
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
        if (type == LIVE_STATE && len == 3 && body[0] == LIVE_BYE) bye = true;
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
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_STATE && len == 3 && body[0] == LIVE_PAIRING);
    free(body);
    canvas_fill(&c, (Rect){0, 0, 64, 64}, 0);  // the code on the card
    live_push(&c, (Rect){0, 0, 64, 64});
    canvas_fill(&c, (Rect){0, 0, 64, 64}, 200);  // the card gone, the screen as it was
    live_push(&c, (Rect){0, 0, 64, 64});
    live_set_state(LIVE_AWAKE);
    CHECK(read_frame(sv[1], &type, &body, &len) && type == LIVE_STATE && len == 3 && body[0] == LIVE_AWAKE);
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
static bool next_state(int fd, uint8_t out[3]) {
    uint8_t type, *body;
    uint32_t len;
    while (read_frame(fd, &type, &body, &len)) {
        bool state = type == LIVE_STATE && len == 3;
        if (state) memcpy(out, body, 3);
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
    uint8_t st[3];
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

TEST(the_browser_is_told_the_front_light_warmth) {
    const int w = 64, h = 64;
    Canvas c;
    CHECK(canvas_init(&c, w, h) == 0);
    canvas_fill(&c, (Rect){0, 0, w, h}, 230);
    CHECK(live_start(w, h) == 0);
    live_push(&c, (Rect){0, 0, w, h});
    live_set_warmth(10);  // set before the browser came: its first frame says so
    int sv[2];
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    CHECK(live_take_viewer(sv[0]));
    close(sv[0]);
    CHECK(skip_head(sv[1]));
    uint8_t st[3];
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE && st[1] == 0 && st[2] == 10);
    uint8_t *seen = calloc((size_t)w * (size_t)h, 1);
    int hellos = 0;
    CHECK(read_batch(sv[1], seen, w, h, &hellos) > 0);
    live_set_warmth(10);  // no change: nothing told
    live_set_warmth(24);
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE && st[2] == 24);
    live_set_warmth(300);  // past what a byte holds: the most the frame can say short of none
    CHECK(next_state(sv[1], st) && st[2] == LIVE_NO_WARMTH - 1);
    live_set_warmth(-1);  // no warm light (or not known)
    CHECK(next_state(sv[1], st) && st[2] == LIVE_NO_WARMTH);
    live_set_warmth(3);
    live_stop();  // bye, with the warmth as it is
    bool bye = false;
    while (next_state(sv[1], st))
        if (st[0] == LIVE_BYE) bye = st[2] == 3;
    CHECK(bye);
    close(sv[1]);
    free(seen);
    canvas_free(&c);
}

TEST(the_pen_word_packs_cuts_and_reads_back) {
    CHECK(live_pen_word(LIVE_PEN_HOVER, 10.4f, 20.6f, 300, 200) == (1u << 30 | 21u << 15 | 10u));
    CHECK(live_pen_word(LIVE_PEN_AWAY, 10.f, 20.f, 300, 200) == 0);
    CHECK(live_pen_word(7, 10.f, 20.f, 300, 200) == 0);  // (not a state: away)
    // Past the edges (a pen calibrated a little past the glass), and a NaN: cut to the screen.
    CHECK(live_pen_word(LIVE_PEN_TOUCH, -40.f, 9999.f, 300, 200) == (2u << 30 | 199u << 15));
    CHECK(live_pen_word(LIVE_PEN_TOUCH, NAN, 5.f, 300, 200) == (2u << 30 | 5u << 15));
    uint8_t p[5];
    live_pen_payload(p, live_pen_word(LIVE_PEN_TOUCH, 1859.f, 2479.f, 1860, 2480));  // the far corner
    CHECK(p[0] == 0x43 && p[1] == 0x07 && p[2] == 0xAF && p[3] == 0x09 && p[4] == LIVE_PEN_TOUCH);
    live_pen_payload(p, 0);
    CHECK(!p[0] && !p[1] && !p[2] && !p[3] && p[4] == LIVE_PEN_AWAY);
}

// Nothing to read on fd for ms.
static bool quiet(int fd, int ms) {
    struct pollfd p = {fd, POLLIN, 0};
    return poll(&p, 1, ms) == 0;
}

// The next pen frame (x, y, state into out), the frames before it skipped: the tiles and batch ends
// among them counted where asked.
static bool next_pen(int fd, unsigned out[3], int *tiles, int *batches) {
    uint8_t type, *body;
    uint32_t len;
    while (read_frame(fd, &type, &body, &len)) {
        bool pen = type == LIVE_PEN_FRAME && len == 5;
        if (pen) out[0] = u16(body), out[1] = u16(body + 2), out[2] = body[4];
        if (tiles && type == LIVE_TILE_FRAME) (*tiles)++;
        if (batches && type == LIVE_BATCH) (*batches)++;
        free(body);
        if (pen) return true;
    }
    return false;
}

TEST(a_viewer_sends_the_pen_after_the_tiles_and_only_its_newest_place) {
    const int w = 300, h = 200;
    Canvas c;
    CHECK(canvas_init(&c, w, h) == 0);
    canvas_fill(&c, (Rect){0, 0, w, h}, 230);
    CHECK(live_start(w, h) == 0);
    live_push(&c, (Rect){0, 0, w, h});
    int sv[2];
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    CHECK(live_take_viewer(sv[0]));
    close(sv[0]);
    CHECK(skip_head(sv[1]));
    uint8_t st[3];
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE);
    uint8_t *seen = calloc((size_t)w * (size_t)h, 1);
    int hellos = 0;
    CHECK(read_batch(sv[1], seen, w, h, &hellos) > 0);
    CHECK(quiet(sv[1], 300));  // no pen yet: nothing (the browser starts with none)
    // Coming into range wakes the viewer, idle in its one-second wait, at once.
    uint32_t t0 = now_ms();
    live_pen(LIVE_PEN_HOVER, 10.f, 20.f);
    unsigned pen[3];
    CHECK(next_pen(sv[1], pen, NULL, NULL) && pen[0] == 10 && pen[1] == 20 && pen[2] == LIVE_PEN_HOVER);
    CHECK(now_ms() - t0 < 600);
    // A stroke's tile and the pen's place after it: the tile's batch first, then the pen.
    canvas_fill(&c, (Rect){70, 70, 20, 20}, 0);
    live_push(&c, (Rect){70, 70, 20, 20});
    live_pen(LIVE_PEN_TOUCH, 80.f, 80.f);
    int tiles = 0, batches = 0;
    CHECK(next_pen(sv[1], pen, &tiles, &batches) && pen[0] == 80 && pen[1] == 80 && pen[2] == LIVE_PEN_TOUCH);
    CHECK(tiles == 1 && batches == 1);
    // Fifty places at once: the newest comes, with at most one between.
    for (int i = 0; i < 50; i++) live_pen(LIVE_PEN_HOVER, 100.f + (float)i, 50.f);
    int frames = 0;
    do CHECK(next_pen(sv[1], pen, NULL, NULL) && ++frames <= 2);
    while (pen[0] != 149 && frames <= 2);
    CHECK(pen[0] == 149 && pen[1] == 50 && pen[2] == LIVE_PEN_HOVER);
    // Moving for 300 ms, a place every 5 ms: about 30 a second, never more, the last one last. The
    // bound is from the time it took, not the 300 ms asked for: under a loaded make -j the sleeps
    // run long, and every frame was sent between t0 and the moment the last one was read.
    t0 = now_ms();
    for (int i = 0; i < 60; i++) {
        live_pen(LIVE_PEN_HOVER, 10.f + (float)i, 150.f);
        sleep_ms(5);
    }
    frames = 0;
    uint32_t last_read = t0;
    while (!quiet(sv[1], 150) && next_pen(sv[1], pen, NULL, NULL)) {
        frames++;
        last_read = now_ms();
    }
    CHECK(frames >= 2 && frames <= (int)((last_read - t0) / LIVE_PEN_MS) + 2);
    CHECK(frames < 60);  // (60 places: fewer frames than that, whatever the load)
    CHECK(pen[0] == 69 && pen[1] == 150);
    // A pen that stays where it is is not told again; gone, it is told once.
    live_pen(LIVE_PEN_HOVER, 69.f, 150.f);
    CHECK(quiet(sv[1], 200));
    live_pen(LIVE_PEN_AWAY, 0.f, 0.f);
    CHECK(next_pen(sv[1], pen, NULL, NULL) && pen[2] == LIVE_PEN_AWAY);
    CHECK(quiet(sv[1], 300));
    live_stop();
    close(sv[1]);
    free(seen);
    canvas_free(&c);
}

TEST(the_pen_is_never_sent_asleep_and_a_turn_takes_it_away) {
    const int w = 128, h = 96;
    Canvas c;
    CHECK(canvas_init(&c, w, h) == 0);
    canvas_fill(&c, (Rect){0, 0, w, h}, 230);
    CHECK(live_start(w, h) == 0);
    live_push(&c, (Rect){0, 0, w, h});
    live_pen(LIVE_PEN_HOVER, 5.f, 6.f);  // in range as the browser comes: told after its first batch
    int sv[2];
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    CHECK(live_take_viewer(sv[0]));
    close(sv[0]);
    CHECK(skip_head(sv[1]));
    unsigned pen[3];
    int tiles = 0, batches = 0;
    CHECK(next_pen(sv[1], pen, &tiles, &batches) && pen[0] == 5 && pen[1] == 6 && pen[2] == LIVE_PEN_HOVER);
    CHECK(tiles > 0 && batches == 1);
    // Asleep: the state, and the pen is gone with it (the browser takes it away); awake again, it
    // is not told until it moves.
    live_set_state(LIVE_ASLEEP);
    uint8_t st[3];
    CHECK(next_state(sv[1], st) && st[0] == LIVE_ASLEEP);
    CHECK(quiet(sv[1], 200));
    live_set_state(LIVE_AWAKE);
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE);
    CHECK(quiet(sv[1], 200));
    live_pen(LIVE_PEN_HOVER, 7.f, 8.f);
    CHECK(next_pen(sv[1], pen, NULL, NULL) && pen[0] == 7 && pen[1] == 8);
    // The pairing card takes it away the same way.
    live_set_state(LIVE_PAIRING);
    CHECK(next_state(sv[1], st) && st[0] == LIVE_PAIRING);
    live_set_state(LIVE_AWAKE);
    CHECK(next_state(sv[1], st) && st[0] == LIVE_AWAKE);
    CHECK(quiet(sv[1], 200));
    // Turned: the hello, and no pen until it is told on the turned screen (96 wide now).
    live_pen(LIVE_PEN_HOVER, 100.f, 10.f);
    CHECK(next_pen(sv[1], pen, NULL, NULL) && pen[0] == 100);
    live_resize(h, w);
    Canvas t;
    CHECK(canvas_init(&t, h, w) == 0);
    canvas_fill(&t, (Rect){0, 0, h, w}, 40);
    live_push(&t, (Rect){0, 0, h, w});
    // Every frame until the turned screen is all sent: one batch, or two when the viewer took its
    // snapshot while live_push was still copying (seen with a dozen of these tests at once). One
    // hello among them and no pen.
    int hellos = 0, pens = 0;
    uint8_t type, *body;
    uint32_t len;
    while (!quiet(sv[1], 200) && read_frame(sv[1], &type, &body, &len)) {
        hellos += type == LIVE_HELLO;
        pens += type == LIVE_PEN_FRAME;
        free(body);
    }
    CHECK(hellos == 1 && pens == 0);
    live_pen(LIVE_PEN_HOVER, 100.f, 120.f);
    CHECK(next_pen(sv[1], pen, NULL, NULL) && pen[0] == 95 && pen[1] == 120);  // (x cut to the new width)
    live_stop();
    close(sv[1]);
    canvas_free(&t);
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
    RUN(the_browser_is_told_the_front_light_warmth);
    RUN(the_pen_word_packs_cuts_and_reads_back);
    RUN(a_viewer_sends_the_pen_after_the_tiles_and_only_its_newest_place);
    RUN(the_pen_is_never_sent_asleep_and_a_turn_takes_it_away);
    RUN(no_viewer_for_ten_minutes_turns_it_off);
    return tl_done();
}
