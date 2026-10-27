// test_pensync.c: the pen after the kernel dropped its events (SYN_DROPPED), the kernel's sample
// times in now_ms()'s clock, and the raw digitizer's frame each pen frame takes pressure and tilt from. The shapes are the ones the Scribe 2's pen service sends: frames of X
// and Y alone (an axis that did not change is left out), about 2.8 ms apart, and the buttons only
// when they change. p28 (2026-10-26) lost 220-270 ms of path to a 330-370 ms stall, and the stroke
// jumped to the device's present position, then traced the 40 frames left in the queue back.
#include "pensync.h"
#include "testlib.h"

typedef struct {
    PenEv ev[1024];
    int n;
    uint64_t t;
} Stream;

static void put(Stream *s, int type, int code, int value) { s->ev[s->n++] = (PenEv){s->t, (uint16_t)type, (uint16_t)code, value}; }
static void drop(Stream *s) { put(s, PS_EV_SYN, PS_SYN_DROPPED, 0); }
static void report(Stream *s) {
    put(s, PS_EV_SYN, PS_SYN_REPORT, 0);
    s->t += 2800;  // the pen service's frames are about 2.8 ms apart
}
static void key(Stream *s, int code, int value) { put(s, PS_EV_KEY, code, value); }
static void xy(Stream *s, int x, int y) {
    put(s, PS_EV_ABS, PS_ABS_X, x);
    put(s, PS_EV_ABS, PS_ABS_Y, y);
    report(s);
}

static const PenState WRITING = {.tool_pen = true, .touching = true, .x = 674, .y = 433};
static const PenState HOVERING = {.tool_pen = true, .x = 100, .y = 100};

static PenTail tail_of(const Stream *s, PenState good, PenState now) {
    return (PenTail){.good = good, .ev = s->ev, .n = s->n, .now = now, .now_ok = PS_F_ALL, .touch_btn = true};
}
static bool contact(const PenFrame *f) { return pensync_contact(&f->s, true, 0); }

// A drop, the rest of the frame it cut (Y alone), and the tail.
static void cut(Stream *s, int y) {
    s->t = 7857000000ull;  // the kernel's time, in us
    drop(s);
    put(s, PS_EV_ABS, PS_ABS_Y, y);
    report(s);
}

TEST(the_p28_shape_reports_the_tail_in_order_and_never_jumps_ahead) {
    static Stream s;
    cut(&s, 438);
    for (int i = 0; i < 40; i++) xy(&s, 700 + i, 440 + i / 2);  // ends at 739, 459
    PenState now = WRITING;
    now.x = 739, now.y = 459;  // the device holds the tail's end
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[1024 + 3];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 1024 + 3, &info);
    CHECK(n == 40 && info.frames == 40 && info.skipped == 1 && info.used == s.n);
    CHECK(!info.lifted && !info.keys);
    CHECK(out[0].gap && out[0].moved);
    // The first point is the oldest frame's, joined from the last good point; not the device's now.
    CHECK(out[0].s.x == 700 && out[0].s.y == 440);
    bool in_order = true, timed = true, writing = true;
    for (int i = 0; i < n; i++) {
        in_order &= out[i].s.x == 700 + i && out[i].s.y == 440 + i / 2;
        writing &= contact(&out[i]);
        if (i) timed &= out[i].t_us == out[i - 1].t_us + 2800 && !out[i].gap && out[i].moved;
        if (i < n - 1) CHECK(out[i].s.x != now.x);  // the present position comes last, in its turn
    }
    CHECK(in_order && timed && writing);
    CHECK(info.x == 739 && info.y == 459 && info.first_us == out[0].t_us && info.last_us == out[39].t_us);
    // Their times for the stroke keep the frames' spacing (the old code stamped all 40 alike).
    uint32_t read_at = (uint32_t)(info.last_us / 1000u) + 40;
    uint32_t a = pensync_ms(out[0].t_us, true, read_at), b = pensync_ms(out[39].t_us, true, read_at);
    CHECK(b - a >= 109 && b - a <= 110);  // 39 frames of 2.8 ms, in whole ms
    CHECK(pensync_ms(out[1].t_us, true, read_at) - a >= 2);
}

TEST(an_axis_missing_from_the_first_frame_takes_its_first_value_in_the_tail) {
    static Stream s;
    cut(&s, 499);
    put(&s, PS_EV_ABS, PS_ABS_Y, 500);  // X did not change in this frame: the pen service leaves it out
    report(&s);
    xy(&s, 131, 502);
    xy(&s, 132, 504);
    PenState good = WRITING, now = WRITING;
    good.x = 100, good.y = 400;  // where the pen was before the lost stretch
    now.x = 132, now.y = 504;
    PenTail t = tail_of(&s, good, now);
    PenFrame out[16];
    int n = pensync_tail(&t, out, 16, NULL);
    CHECK(n == 3);
    // Neither the stale X from before the drop (a corner) nor the device's present one.
    CHECK(out[0].s.x == 131 && out[0].s.y == 500);
    CHECK(out[1].s.x == 131 && out[2].s.x == 132 && out[2].s.y == 504);

    // A tail that never carries X: the pen kept the X the device holds now, all along.
    static Stream v;
    cut(&v, 499);
    for (int i = 0; i < 5; i++) {
        put(&v, PS_EV_ABS, PS_ABS_Y, 500 + i);
        report(&v);
    }
    now.x = 140, now.y = 504;
    t = tail_of(&v, good, now);
    n = pensync_tail(&t, out, 16, NULL);
    CHECK(n == 5);
    for (int i = 0; i < n; i++) CHECK(out[i].s.x == 140 && out[i].s.y == 500 + i);
    // And with the device not answering, the X from before the drop is all there is.
    t.now_ok = 0;
    n = pensync_tail(&t, out, 16, NULL);
    CHECK(n == 5 && out[0].s.x == 100 && out[0].s.y == 500 && contact(&out[0]));
}

TEST(a_lift_inside_the_tail_lifts_there) {
    static Stream s;
    cut(&s, 100);
    for (int i = 0; i < 10; i++) xy(&s, 110 + i, 100);
    key(&s, PS_BTN_TOUCH, 0);  // lifted at frame 11
    xy(&s, 120, 100);
    for (int i = 0; i < 4; i++) xy(&s, 121 + i, 102);
    key(&s, PS_BTN_TOUCH, 1);  // and down again at frame 16
    xy(&s, 125, 104);
    for (int i = 0; i < 4; i++) xy(&s, 126 + i, 104);
    PenState now = WRITING;
    now.x = 129, now.y = 104;
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[64];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 64, &info);
    CHECK(n == 20 && info.keys && !info.lifted);
    for (int i = 0; i < n; i++) CHECK(contact(&out[i]) == (i < 10 || i >= 15));
    CHECK(out[10].s.x == 120 && out[15].s.x == 125);  // the lift and the new stroke where they were
}

TEST(a_lift_after_the_tails_writing_keeps_the_writing) {
    // The pen is up when the device is asked, but its lift is the tail's last frame: everything
    // before it was ink. (The old code asked first and lifted at once, at the tail's end, and the
    // 40 frames were reported as hovering: lost ink.)
    static Stream s;
    cut(&s, 438);
    for (int i = 0; i < 40; i++) xy(&s, 700 + i, 440);
    key(&s, PS_BTN_TOUCH, 0);
    report(&s);
    PenState now = WRITING;
    now.touching = false, now.x = 739, now.y = 440;
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[64];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 64, &info);
    CHECK(n == 41 && !info.lifted);
    for (int i = 0; i < 40; i++) CHECK(contact(&out[i]) && out[i].s.x == 700 + i);
    CHECK(!contact(&out[40]) && out[40].s.x == 739 && !out[40].moved);
}

TEST(a_lift_in_the_lost_stretch_ends_the_stroke_at_the_last_good_point) {
    // The tail has the pen hovering, then leaving; no lift in it: the lift was thrown away. The
    // stroke ends where it was last seen, not where the tail finds the pen.
    static Stream s;
    cut(&s, 120);
    for (int i = 0; i < 5; i++) xy(&s, 800 + i, 120);
    key(&s, PS_BTN_TOOL_PEN, 0);
    report(&s);
    PenState now = {.x = 804, .y = 120};
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[16];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 16, &info);
    CHECK(n == 7 && info.lifted);
    CHECK(!contact(&out[0]) && out[0].s.x == WRITING.x && out[0].s.y == WRITING.y && out[0].s.tool_pen);
    CHECK(out[1].gap && !contact(&out[1]) && out[1].s.x == 800 && out[1].s.tool_pen);
    CHECK(!out[6].s.tool_pen && out[6].s.x == 804);
}

TEST(a_touch_at_the_first_frame_means_a_lift_in_the_lost_stretch) {
    // The first frame brings BTN_TOUCH 1: the input core passes only changes, so the pen was up just
    // before it, and the stroke at the last good point ended in the lost stretch. One word's end is
    // not joined to the next word's start.
    static Stream s;
    cut(&s, 438);
    key(&s, PS_BTN_TOUCH, 1);
    xy(&s, 900, 500);
    for (int i = 1; i < 5; i++) xy(&s, 900 + i, 500);
    PenState now = WRITING;
    now.x = 904, now.y = 500;
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[16];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 16, &info);
    CHECK(n == 6 && info.lifted && info.frames == 5);
    CHECK(!contact(&out[0]) && out[0].s.x == WRITING.x && out[0].s.y == WRITING.y && !out[0].gap);
    CHECK(contact(&out[1]) && out[1].gap && out[1].s.x == 900 && out[1].s.y == 500);
    for (int i = 2; i < n; i++) CHECK(contact(&out[i]) && out[i].s.x == 899 + i);
}

TEST(an_eraser_flip_at_the_first_frame_or_in_the_lost_stretch_ends_the_pen_stroke) {
    // The eraser end touches down in the tail's first frame: the pen's stroke ended before it, and
    // the eraser starts its own (the editor picks ink or eraser where a stroke begins).
    static Stream s;
    cut(&s, 438);
    key(&s, PS_BTN_TOOL_PEN, 0);
    key(&s, PS_BTN_TOOL_RUBBER, 1);
    key(&s, PS_BTN_TOUCH, 1);
    xy(&s, 900, 500);
    for (int i = 1; i < 5; i++) xy(&s, 900 + i, 500);
    PenState now = {.tool_rubber = true, .touching = true, .x = 904, .y = 500};
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[16];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 16, &info);
    CHECK(n == 6 && info.lifted);
    CHECK(!contact(&out[0]) && out[0].s.tool_pen && !out[0].s.tool_rubber && out[0].s.x == WRITING.x);
    CHECK(contact(&out[1]) && out[1].gap && out[1].s.tool_rubber && !out[1].s.tool_pen && out[1].s.x == 900);
    for (int i = 1; i < n; i++) CHECK(contact(&out[i]) && out[i].s.tool_rubber);

    // The whole flip in the lost stretch: the tail carries no buttons, only the eraser's moves, and
    // the device says the eraser end touches. Still not the pen's stroke going on.
    static Stream v;
    cut(&v, 438);
    for (int i = 0; i < 5; i++) xy(&v, 900 + i, 500);
    t = tail_of(&v, WRITING, now);
    n = pensync_tail(&t, out, 16, &info);
    CHECK(n == 6 && info.lifted && !info.keys);
    CHECK(!contact(&out[0]) && out[0].s.tool_pen && out[0].s.x == WRITING.x);
    CHECK(contact(&out[1]) && out[1].gap && out[1].s.tool_rubber && out[1].s.x == 900);
}

TEST(an_eraser_flip_in_the_tail_flips_where_it_happened) {
    static Stream s;
    cut(&s, 100);
    for (int i = 0; i < 5; i++) xy(&s, 101 + i, 100);
    key(&s, PS_BTN_TOOL_PEN, 0);
    report(&s);
    key(&s, PS_BTN_TOOL_RUBBER, 1);
    xy(&s, 300, 300);
    key(&s, PS_BTN_TOUCH, 1);
    xy(&s, 301, 300);
    for (int i = 0; i < 4; i++) xy(&s, 302 + i, 300);
    PenState now = {.tool_rubber = true, .touching = true, .x = 305, .y = 300};
    PenTail t = tail_of(&s, HOVERING, now);
    PenFrame out[32];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 32, &info);
    CHECK(n == 12 && !info.lifted);
    for (int i = 0; i < 5; i++) CHECK(out[i].s.tool_pen && !out[i].s.tool_rubber && !contact(&out[i]));
    CHECK(!out[5].s.tool_pen && !out[5].s.tool_rubber);
    CHECK(out[6].s.tool_rubber && !contact(&out[6]) && out[6].s.x == 300);
    // The eraser touches down at its own first point, not at the device's present position.
    CHECK(contact(&out[7]) && out[7].s.x == 301);
    for (int i = 7; i < n; i++) CHECK(contact(&out[i]) && out[i].s.tool_rubber);
}

TEST(a_button_changed_after_the_tail_was_read_comes_last) {
    // The pen came down in the tail; by the time the device was asked it was up again, and
    // EVIOCGKEY took that lift out of the queue: it is reported after the tail, at its end.
    static Stream s;
    cut(&s, 100);
    key(&s, PS_BTN_TOUCH, 1);
    xy(&s, 200, 200);
    for (int i = 1; i < 5; i++) xy(&s, 200 + i, 200);
    PenState now = {.tool_pen = true, .x = 204, .y = 200};
    PenTail t = tail_of(&s, HOVERING, now);
    PenFrame out[16];
    int n = pensync_tail(&t, out, 16, NULL);
    CHECK(n == 6);
    for (int i = 0; i < 5; i++) CHECK(contact(&out[i]));
    CHECK(!contact(&out[5]) && out[5].s.x == 204 && !out[5].moved && out[5].t_us == out[4].t_us);
}

TEST(no_frame_after_the_partial_one_reports_the_device_now) {
    static Stream s;
    cut(&s, 160);
    PenState now = WRITING;
    now.x = 150, now.y = 160;
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[8];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 8, &info);
    CHECK(n == 1 && info.frames == 0 && out[0].gap && out[0].s.x == 150 && contact(&out[0]));
    CHECK(out[0].t_us == s.ev[2].t_us);  // the partial frame's SYN_REPORT
    // Up by now: the stroke ends at the last good point first.
    now.touching = false;
    t = tail_of(&s, WRITING, now);
    n = pensync_tail(&t, out, 8, &info);
    CHECK(n == 2 && info.lifted && out[0].s.x == WRITING.x && !contact(&out[0]) && out[1].s.x == 150);
}

TEST(a_second_drop_and_an_unfinished_frame) {
    static Stream s;
    cut(&s, 100);
    for (int i = 0; i < 3; i++) xy(&s, 400 + i, 100);
    drop(&s);  // overflowed again: what came before is gone too
    put(&s, PS_EV_ABS, PS_ABS_X, 450);
    report(&s);
    xy(&s, 460, 110);
    xy(&s, 461, 111);
    put(&s, PS_EV_ABS, PS_ABS_X, 462);  // a frame still arriving
    PenState now = WRITING;
    now.x = 462, now.y = 111;
    PenTail t = tail_of(&s, WRITING, now);
    PenFrame out[16];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 16, &info);
    CHECK(n == 2 && info.frames == 2 && info.skipped == 1);
    CHECK(out[0].gap && out[0].s.x == 460 && out[1].s.x == 461);
    CHECK(info.used == s.n - 1);  // the unfinished frame is read on as usual
    // Never more frames than room.
    CHECK(pensync_tail(&t, out, 1, NULL) == 1);
}

TEST(contact_without_a_touch_button) {
    // A pen with no BTN_TOUCH (the raw digitizer as the pen): pressure while near.
    PenState s = {.tool_pen = true, .p = 12};
    CHECK(pensync_contact(&s, false, 0) && !pensync_contact(&s, false, 12) && !pensync_contact(&s, true, 0));
    s.tool_pen = false;
    CHECK(!pensync_contact(&s, false, 0));
    s.tool_rubber = true;
    CHECK(pensync_contact(&s, false, 0));
    // Lifted in the lost stretch: the lift at the last good point carries no pressure.
    static Stream v;
    cut(&v, 100);
    xy(&v, 500, 500);
    PenState good = {.tool_pen = true, .p = 900, .x = 10, .y = 10}, now = {.tool_pen = true, .p = 0, .x = 500, .y = 500};
    PenTail t = {.good = good, .ev = v.ev, .n = v.n, .now = now, .now_ok = PS_F_ALL, .touch_btn = false, .p_min = 0};
    PenFrame out[8];
    PenTailInfo info;
    int n = pensync_tail(&t, out, 8, &info);
    CHECK(n == 2 && info.lifted && out[0].s.p == 0 && out[0].s.x == 10 && !pensync_contact(&out[1].s, false, 0));
}

TEST(kernel_times_in_now_ms_clock) {
    // As now_ms() truncates CLOCK_MONOTONIC: sec * 1000 + nsec / 1000000, in a uint32.
    for (uint64_t sec = 0; sec < 9000000; sec += 777777)
        for (uint64_t ns = 0; ns < 1000000000u; ns += 123456789u) {
            uint32_t now_style = (uint32_t)(sec * 1000u + ns / 1000000u);
            CHECK(pensync_ms(sec * 1000000u + ns / 1000u, true, now_style) == now_style);
        }
    CHECK(pensync_ms(7856999123ull, true, 7857330) == 7856999);  // a sample 331 ms before it was read
    // Across the clock's wrap (49.7 days of uptime), as now_ms() wraps.
    CHECK(pensync_ms(4294968000ull * 1000u + 500u, true, 900) == 704);
    // Falls back to when it was read: not CLOCK_MONOTONIC, no time, from the future, or too old.
    CHECK(pensync_ms(7856999123ull, false, 7857330) == 7857330);
    CHECK(pensync_ms(0, true, 5000) == 5000);
    CHECK(pensync_ms(5001000ull, true, 5000) == 5000);
    CHECK(pensync_ms(1000000ull, true, 1000 + PENSYNC_K_MAX_AGE_MS) == 1000);
    CHECK(pensync_ms(1000000ull, true, 1001 + PENSYNC_K_MAX_AGE_MS) == 1001 + PENSYNC_K_MAX_AGE_MS);
    // Realtime stamps (the clock switch failed unnoticed) are decades off: never used.
    CHECK(pensync_ms(1793000000ull * 1000000u, true, 7857330) == 7857330);
}

// The raw digitizer's frames, about 3 ms apart, each relayed as a pen frame a little later. The
// pressure and tilt rise frame by frame, so a pen frame that took another frame's values shows.
#define RAW_T0 7857000000ull
static PenRaw raw_frame(int i) { return (PenRaw){RAW_T0 + 3000u * (uint64_t)i, 100 + 10 * i, i, -i}; }

TEST(each_pen_frame_takes_the_pressure_and_tilt_of_its_own_moment) {
    static PenRawRing g;
    pensync_raw_clear(&g);
    PenRaw got;
    CHECK(!pensync_raw_at(&g, RAW_T0, &got));  // nothing read yet: the newest values, as before
    // Read in turns, a frame of each: the relay's hop from 0.1 ms up to just under a frame.
    static const uint64_t hops[] = {0, 100, 1000, 2900};
    for (int i = 0; i < 20; i++) {
        PenRaw r = raw_frame(i);
        pensync_raw_push(&g, r.t_us, r.p, r.tx, r.ty);
        for (int h = 0; h < 4; h++) {
            CHECK(pensync_raw_at(&g, r.t_us + hops[h], &got));
            CHECK(got.p == r.p && got.tx == r.tx && got.ty == r.ty && got.t_us == r.t_us);
        }
    }
    // A wait of 100 ms (the main loop held up): 34 frames of each are read at once, the raw ones
    // first. Each pen frame still takes its own, not the newest: no flat stretch.
    pensync_raw_clear(&g);
    for (int i = 0; i < 34; i++) {
        PenRaw r = raw_frame(i);
        pensync_raw_push(&g, r.t_us, r.p, r.tx, r.ty);
    }
    int distinct = 0, last = -1;
    for (int i = 0; i < 34; i++) {
        CHECK(pensync_raw_at(&g, raw_frame(i).t_us + 100, &got));
        CHECK(got.p == raw_frame(i).p && got.tx == i && got.ty == -i);
        distinct += got.p != last;
        last = got.p;
    }
    CHECK(distinct == 34);
    // Older than the oldest kept: none (the caller takes the newest values).
    CHECK(!pensync_raw_at(&g, RAW_T0 - 1, &got));
    // A pen frame newer than every raw frame (the pen held still: the input core sends no unchanged
    // values) takes the newest, which is what the digitizer still holds.
    CHECK(pensync_raw_at(&g, RAW_T0 + 10000000u, &got) && got.p == raw_frame(33).p);
    // Equal times: the later pushed (a resync's values over the frame it came with).
    pensync_raw_push(&g, raw_frame(33).t_us + 3000, 999, 9, 9);
    pensync_raw_push(&g, raw_frame(33).t_us + 3000, 1000, 10, 10);
    CHECK(pensync_raw_at(&g, raw_frame(33).t_us + 3000, &got) && got.p == 1000);
}

// A drop of the raw digitizer's events forgets what was kept: the frames after the gap no longer meet
// it. Its resync asks the device and pushes that at the time it asked; pen frames from the lost
// stretch, older than that, take the newest values (the resync's), as before there was a ring.
TEST(a_raw_drop_forgets_and_the_resync_starts_again) {
    static PenRawRing g;
    pensync_raw_clear(&g);
    for (int i = 0; i < 10; i++) pensync_raw_push(&g, raw_frame(i).t_us, raw_frame(i).p, i, -i);
    pensync_raw_clear(&g);  // SYN_DROPPED
    PenRaw got;
    CHECK(!pensync_raw_at(&g, raw_frame(5).t_us, &got));
    uint64_t asked = raw_frame(40).t_us;
    pensync_raw_push(&g, asked, 777, 7, -7);
    CHECK(!pensync_raw_at(&g, raw_frame(30).t_us, &got));  // in the lost stretch
    CHECK(pensync_raw_at(&g, asked + 500, &got) && got.p == 777 && got.tx == 7 && got.ty == -7);
    pensync_raw_push(&g, raw_frame(41).t_us, raw_frame(41).p, 41, -41);
    CHECK(pensync_raw_at(&g, raw_frame(41).t_us + 100, &got) && got.p == raw_frame(41).p);
}

// The ring keeps the last PENSYNC_RAW_N frames (about 0.8 s): a pen frame older than those finds none.
TEST(the_ring_keeps_the_newest_frames) {
    static PenRawRing g;
    pensync_raw_clear(&g);
    for (int i = 0; i < PENSYNC_RAW_N + 44; i++) pensync_raw_push(&g, raw_frame(i).t_us, raw_frame(i).p, i, -i);
    PenRaw got;
    CHECK(g.count == PENSYNC_RAW_N);
    CHECK(!pensync_raw_at(&g, raw_frame(43).t_us + 100, &got));
    CHECK(pensync_raw_at(&g, raw_frame(44).t_us + 100, &got) && got.p == raw_frame(44).p);
    CHECK(pensync_raw_at(&g, raw_frame(PENSYNC_RAW_N + 43).t_us, &got) && got.tx == PENSYNC_RAW_N + 43);
}

// The Scribe 2's ranges (grimoire.log, 2026-10-26): the raw digitizer 8.4 units to each pixel.
static const int HV_RMIN[2] = {0, 0}, HV_RMAX[2] = {15624, 20832}, HV_SMIN[2] = {0, 0}, HV_SMAX[2] = {1860, 2480};

// Contact frame i of a stroke that wanders over the whole screen: where the scaled pen is, and the
// raw digitizer's place for it under orientation o (bit 0: x reversed, bit 1: y reversed), a pixel or
// two off as the pen service rounds.
static void hover_frame(int i, int o, int *rx, int *ry, int *sx, int *sy) {
    *sx = (i * 397) % 1861;
    *sy = (i * 911) % 2481;
    int ux = o & 1 ? 1860 - *sx : *sx, uy = o & 2 ? 2480 - *sy : *sy;
    *rx = (int)((float)ux * 8.4f + 0.5f) + (i % 3 - 1) * 9;  // (about a pixel either way)
    *ry = (int)((float)uy * 8.4f + 0.5f) - (i % 5 - 2) * 6;
    if (*rx < 0) *rx = 0;
    if (*ry < 0) *ry = 0;
}

static bool near(float a, float b, float tol) { return a - b <= tol && b - a <= tol; }

TEST(the_hovering_pen_comes_from_the_raw_digitizer_in_the_scaled_pens_units) {
    PenHover h;
    pensync_hover_init(&h, HV_RMIN, HV_RMAX, HV_SMIN, HV_SMAX);
    float x, y;
    // Before any contact: both axes as they are, scaled onto the screen.
    CHECK(pensync_hover_orientation(&h) == 0);
    CHECK(pensync_hover_place(&h, 840, 1680, &x, &y) && near(x, 100.f, 0.01f) && near(y, 200.f, 0.01f));
    CHECK(pensync_hover_place(&h, 15624, 20832, &x, &y) && near(x, 1860.f, 0.01f) && near(y, 2480.f, 0.01f));
    CHECK(pensync_hover_place(&h, -50, 99999, &x, &y) && near(x, 0.f, 0.01f) && near(y, 2480.f, 0.01f));  // (cut)
    // A pen service that keeps both axes as they are: so it stays, a pixel or so from the pen.
    int rx, ry, sx, sy;
    for (int i = 0; i < 200; i++) {
        hover_frame(i, 0, &rx, &ry, &sx, &sy);
        pensync_hover_learn(&h, rx, ry, sx, sy);
    }
    CHECK(pensync_hover_orientation(&h) == 0 && pensync_hover_mean(&h, 0) < 2.f);
    CHECK(pensync_hover_mean(&h, 1) > 20.f && pensync_hover_mean(&h, 2) > 20.f && pensync_hover_mean(&h, 3) > 20.f);
    // One frame far off (a stale raw place) weighs at most PENSYNC_HOVER_CAP: nothing changes.
    pensync_hover_learn(&h, 0, 0, 1800, 2400);
    CHECK(pensync_hover_orientation(&h) == 0);
}

TEST(a_reversed_axis_is_found_after_enough_contact_frames) {
    for (int o = 1; o < 4; o++) {
        PenHover h;
        pensync_hover_init(&h, HV_RMIN, HV_RMAX, HV_SMIN, HV_SMAX);
        int rx, ry, sx, sy;
        for (int i = 0; i < PENSYNC_HOVER_FRAMES - 1; i++) {
            hover_frame(i, o, &rx, &ry, &sx, &sy);
            pensync_hover_learn(&h, rx, ry, sx, sy);
        }
        CHECK(pensync_hover_orientation(&h) == 0);  // (not yet: as it is until they decide)
        hover_frame(PENSYNC_HOVER_FRAMES - 1, o, &rx, &ry, &sx, &sy);
        pensync_hover_learn(&h, rx, ry, sx, sy);
        CHECK(pensync_hover_orientation(&h) == o);
        float x, y;
        CHECK(pensync_hover_place(&h, 840, 1680, &x, &y));
        CHECK(near(x, o & 1 ? 1760.f : 100.f, 0.01f) && near(y, o & 2 ? 2280.f : 200.f, 0.01f));
    }
}

TEST(no_hover_when_the_raw_place_is_far_from_the_pen_in_every_orientation) {
    PenHover h;
    pensync_hover_init(&h, HV_RMIN, HV_RMAX, HV_SMIN, HV_SMAX);
    int rx, ry, sx, sy;
    for (int i = 0; i < PENSYNC_HOVER_FRAMES; i++) {
        hover_frame(i, 0, &rx, &ry, &sx, &sy);
        pensync_hover_learn(&h, rx, ry, sx > 930 ? sx - 40 : sx + 40, sy);  // 40 px off, whichever way
    }
    float x = -1.f, y = -1.f;
    CHECK(pensync_hover_orientation(&h) == -1 && !pensync_hover_place(&h, 840, 1680, &x, &y) && x == -1.f);
    // It keeps comparing: frames that agree bring it back once the mean is near again.
    for (int i = 0; i < 400; i++) {
        hover_frame(i, 0, &rx, &ry, &sx, &sy);
        pensync_hover_learn(&h, rx, ry, sx, sy);
    }
    CHECK(pensync_hover_orientation(&h) == 0 && pensync_hover_place(&h, 840, 1680, &x, &y));
    // A long session halves what it has compared, so it never overflows and still follows.
    for (int i = 0; i < 9000; i++) {
        hover_frame(i, 0, &rx, &ry, &sx, &sy);
        pensync_hover_learn(&h, rx, ry, sx, sy);
    }
    CHECK(h.n < 4096 && h.n >= 2048 && pensync_hover_orientation(&h) == 0);
    // A digitizer without ranges: no place at all.
    const int zero[2] = {0, 0};
    pensync_hover_init(&h, zero, zero, HV_SMIN, HV_SMAX);
    CHECK(!pensync_hover_place(&h, 840, 1680, &x, &y));
}

int main(void) {
    RUN(the_p28_shape_reports_the_tail_in_order_and_never_jumps_ahead);
    RUN(an_axis_missing_from_the_first_frame_takes_its_first_value_in_the_tail);
    RUN(a_lift_inside_the_tail_lifts_there);
    RUN(a_lift_after_the_tails_writing_keeps_the_writing);
    RUN(a_lift_in_the_lost_stretch_ends_the_stroke_at_the_last_good_point);
    RUN(a_touch_at_the_first_frame_means_a_lift_in_the_lost_stretch);
    RUN(an_eraser_flip_at_the_first_frame_or_in_the_lost_stretch_ends_the_pen_stroke);
    RUN(an_eraser_flip_in_the_tail_flips_where_it_happened);
    RUN(a_button_changed_after_the_tail_was_read_comes_last);
    RUN(no_frame_after_the_partial_one_reports_the_device_now);
    RUN(a_second_drop_and_an_unfinished_frame);
    RUN(contact_without_a_touch_button);
    RUN(kernel_times_in_now_ms_clock);
    RUN(each_pen_frame_takes_the_pressure_and_tilt_of_its_own_moment);
    RUN(a_raw_drop_forgets_and_the_resync_starts_again);
    RUN(the_ring_keeps_the_newest_frames);
    RUN(the_hovering_pen_comes_from_the_raw_digitizer_in_the_scaled_pens_units);
    RUN(a_reversed_axis_is_found_after_enough_contact_frames);
    RUN(no_hover_when_the_raw_place_is_far_from_the_pen_in_every_orientation);
    return tl_done();
}
