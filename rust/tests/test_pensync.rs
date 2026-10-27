// test_pensync.c: the pen after the kernel dropped its events (SYN_DROPPED), the kernel's sample
// times in now_ms()'s clock, and the raw digitizer's frame each pen frame takes pressure and tilt from. The shapes are the ones the Scribe 2's pen service sends: frames of X
// and Y alone (an axis that did not change is left out), about 2.8 ms apart, and the buttons only
// when they change. p28 (2026-10-26) lost 220-270 ms of path to a 330-370 ms stall, and the stroke
// jumped to the device's present position, then traced the 40 frames left in the queue back.
// Index loops and range comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

// (The C's Stream keeps 1024 events and a count; the Vec's length is the count.)
#[derive(Default)]
struct Stream {
    ev: Vec<PenEv>,
    t: u64,
}

impl Stream {
    fn n(&self) -> i32 {
        self.ev.len() as i32
    }
}

fn put(s: &mut Stream, type_: u16, code: u16, value: i32) {
    let t_us = s.t;
    s.ev.push(PenEv { t_us, type_, code, value });
}
fn drop_(s: &mut Stream) {
    put(s, PS_EV_SYN, PS_SYN_DROPPED, 0);
}
fn report(s: &mut Stream) {
    put(s, PS_EV_SYN, PS_SYN_REPORT, 0);
    s.t += 2800; // the pen service's frames are about 2.8 ms apart
}
fn key(s: &mut Stream, code: u16, value: i32) {
    put(s, PS_EV_KEY, code, value);
}
fn xy(s: &mut Stream, x: i32, y: i32) {
    put(s, PS_EV_ABS, PS_ABS_X, x);
    put(s, PS_EV_ABS, PS_ABS_Y, y);
    report(s);
}

const WRITING: PenState = PenState { tool_pen: true, tool_rubber: false, touching: true, side: false, x: 674, y: 433, p: 0, tx: 0, ty: 0 };
const HOVERING: PenState = PenState { tool_pen: true, tool_rubber: false, touching: false, side: false, x: 100, y: 100, p: 0, tx: 0, ty: 0 };

fn tail_of(s: &Stream, good: PenState, now: PenState) -> PenTail<'_> {
    PenTail { good, ev: &s.ev, now, now_ok: PS_F_ALL, touch_btn: true, p_min: 0 }
}
fn contact(f: &PenFrame) -> bool {
    pensync_contact(&f.s, true, 0)
}
// pensync_tail with the C's NULL info.
fn tail_no_info(t: &PenTail, out: &mut [PenFrame]) -> i32 {
    let mut info = PenTailInfo::default();
    pensync_tail(t, out, &mut info)
}

// A drop, the rest of the frame it cut (Y alone), and the tail.
fn cut(s: &mut Stream, y: i32) {
    s.t = 7857000000; // the kernel's time, in us
    drop_(s);
    put(s, PS_EV_ABS, PS_ABS_Y, y);
    report(s);
}

fn the_p28_shape_reports_the_tail_in_order_and_never_jumps_ahead() {
    let mut s = Stream::default();
    cut(&mut s, 438);
    for i in 0..40 {
        xy(&mut s, 700 + i, 440 + i / 2); // ends at 739, 459
    }
    let mut now = WRITING;
    now.x = 739;
    now.y = 459; // the device holds the tail's end
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 1024 + 3];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info) as usize;
    check!(n == 40 && info.frames == 40 && info.skipped == 1 && info.used == s.n());
    check!(!info.lifted && !info.keys);
    check!(out[0].gap && out[0].moved);
    // The first point is the oldest frame's, joined from the last good point; not the device's now.
    check!(out[0].s.x == 700 && out[0].s.y == 440);
    let (mut in_order, mut timed, mut writing) = (true, true, true);
    for i in 0..n {
        in_order &= out[i].s.x == 700 + i as i32 && out[i].s.y == 440 + i as i32 / 2;
        writing &= contact(&out[i]);
        if i != 0 {
            timed &= out[i].t_us == out[i - 1].t_us + 2800 && !out[i].gap && out[i].moved;
        }
        if i < n - 1 {
            check!(out[i].s.x != now.x); // the present position comes last, in its turn
        }
    }
    check!(in_order && timed && writing);
    check!(info.x == 739 && info.y == 459 && info.first_us == out[0].t_us && info.last_us == out[39].t_us);
    // Their times for the stroke keep the frames' spacing (the old code stamped all 40 alike).
    let read_at = (info.last_us / 1000) as u32 + 40;
    let a = pensync_ms(out[0].t_us, true, read_at);
    let b = pensync_ms(out[39].t_us, true, read_at);
    check!(b.wrapping_sub(a) >= 109 && b.wrapping_sub(a) <= 110); // 39 frames of 2.8 ms, in whole ms
    check!(pensync_ms(out[1].t_us, true, read_at).wrapping_sub(a) >= 2);
}

fn an_axis_missing_from_the_first_frame_takes_its_first_value_in_the_tail() {
    let mut s = Stream::default();
    cut(&mut s, 499);
    put(&mut s, PS_EV_ABS, PS_ABS_Y, 500); // X did not change in this frame: the pen service leaves it out
    report(&mut s);
    xy(&mut s, 131, 502);
    xy(&mut s, 132, 504);
    let mut good = WRITING;
    let mut now = WRITING;
    good.x = 100;
    good.y = 400; // where the pen was before the lost stretch
    now.x = 132;
    now.y = 504;
    let t = tail_of(&s, good, now);
    let mut out = [PenFrame::default(); 16];
    let n = tail_no_info(&t, &mut out);
    check!(n == 3);
    // Neither the stale X from before the drop (a corner) nor the device's present one.
    check!(out[0].s.x == 131 && out[0].s.y == 500);
    check!(out[1].s.x == 131 && out[2].s.x == 132 && out[2].s.y == 504);

    // A tail that never carries X: the pen kept the X the device holds now, all along.
    let mut v = Stream::default();
    cut(&mut v, 499);
    for i in 0..5 {
        put(&mut v, PS_EV_ABS, PS_ABS_Y, 500 + i);
        report(&mut v);
    }
    now.x = 140;
    now.y = 504;
    let mut t = tail_of(&v, good, now);
    let n = tail_no_info(&t, &mut out);
    check!(n == 5);
    for i in 0..n as usize {
        check!(out[i].s.x == 140 && out[i].s.y == 500 + i as i32);
    }
    // And with the device not answering, the X from before the drop is all there is.
    t.now_ok = 0;
    let n = tail_no_info(&t, &mut out);
    check!(n == 5 && out[0].s.x == 100 && out[0].s.y == 500 && contact(&out[0]));
}

fn a_lift_inside_the_tail_lifts_there() {
    let mut s = Stream::default();
    cut(&mut s, 100);
    for i in 0..10 {
        xy(&mut s, 110 + i, 100);
    }
    key(&mut s, PS_BTN_TOUCH, 0); // lifted at frame 11
    xy(&mut s, 120, 100);
    for i in 0..4 {
        xy(&mut s, 121 + i, 102);
    }
    key(&mut s, PS_BTN_TOUCH, 1); // and down again at frame 16
    xy(&mut s, 125, 104);
    for i in 0..4 {
        xy(&mut s, 126 + i, 104);
    }
    let mut now = WRITING;
    now.x = 129;
    now.y = 104;
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 64];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 20 && info.keys && !info.lifted);
    for i in 0..n as usize {
        check!(contact(&out[i]) == (i < 10 || i >= 15));
    }
    check!(out[10].s.x == 120 && out[15].s.x == 125); // the lift and the new stroke where they were
}

fn a_lift_after_the_tails_writing_keeps_the_writing() {
    // The pen is up when the device is asked, but its lift is the tail's last frame: everything
    // before it was ink. (The old code asked first and lifted at once, at the tail's end, and the
    // 40 frames were reported as hovering: lost ink.)
    let mut s = Stream::default();
    cut(&mut s, 438);
    for i in 0..40 {
        xy(&mut s, 700 + i, 440);
    }
    key(&mut s, PS_BTN_TOUCH, 0);
    report(&mut s);
    let mut now = WRITING;
    now.touching = false;
    now.x = 739;
    now.y = 440;
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 64];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 41 && !info.lifted);
    for i in 0..40 {
        check!(contact(&out[i]) && out[i].s.x == 700 + i as i32);
    }
    check!(!contact(&out[40]) && out[40].s.x == 739 && !out[40].moved);
}

fn a_lift_in_the_lost_stretch_ends_the_stroke_at_the_last_good_point() {
    // The tail has the pen hovering, then leaving; no lift in it: the lift was thrown away. The
    // stroke ends where it was last seen, not where the tail finds the pen.
    let mut s = Stream::default();
    cut(&mut s, 120);
    for i in 0..5 {
        xy(&mut s, 800 + i, 120);
    }
    key(&mut s, PS_BTN_TOOL_PEN, 0);
    report(&mut s);
    let now = PenState { x: 804, y: 120, ..Default::default() };
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 16];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 7 && info.lifted);
    check!(!contact(&out[0]) && out[0].s.x == WRITING.x && out[0].s.y == WRITING.y && out[0].s.tool_pen);
    check!(out[1].gap && !contact(&out[1]) && out[1].s.x == 800 && out[1].s.tool_pen);
    check!(!out[6].s.tool_pen && out[6].s.x == 804);
}

fn a_touch_at_the_first_frame_means_a_lift_in_the_lost_stretch() {
    // The first frame brings BTN_TOUCH 1: the input core passes only changes, so the pen was up just
    // before it, and the stroke at the last good point ended in the lost stretch. One word's end is
    // not joined to the next word's start.
    let mut s = Stream::default();
    cut(&mut s, 438);
    key(&mut s, PS_BTN_TOUCH, 1);
    xy(&mut s, 900, 500);
    for i in 1..5 {
        xy(&mut s, 900 + i, 500);
    }
    let mut now = WRITING;
    now.x = 904;
    now.y = 500;
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 16];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 6 && info.lifted && info.frames == 5);
    check!(!contact(&out[0]) && out[0].s.x == WRITING.x && out[0].s.y == WRITING.y && !out[0].gap);
    check!(contact(&out[1]) && out[1].gap && out[1].s.x == 900 && out[1].s.y == 500);
    for i in 2..n as usize {
        check!(contact(&out[i]) && out[i].s.x == 899 + i as i32);
    }
}

fn an_eraser_flip_at_the_first_frame_or_in_the_lost_stretch_ends_the_pen_stroke() {
    // The eraser end touches down in the tail's first frame: the pen's stroke ended before it, and
    // the eraser starts its own (the editor picks ink or eraser where a stroke begins).
    let mut s = Stream::default();
    cut(&mut s, 438);
    key(&mut s, PS_BTN_TOOL_PEN, 0);
    key(&mut s, PS_BTN_TOOL_RUBBER, 1);
    key(&mut s, PS_BTN_TOUCH, 1);
    xy(&mut s, 900, 500);
    for i in 1..5 {
        xy(&mut s, 900 + i, 500);
    }
    let now = PenState { tool_rubber: true, touching: true, x: 904, y: 500, ..Default::default() };
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 16];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 6 && info.lifted);
    check!(!contact(&out[0]) && out[0].s.tool_pen && !out[0].s.tool_rubber && out[0].s.x == WRITING.x);
    check!(contact(&out[1]) && out[1].gap && out[1].s.tool_rubber && !out[1].s.tool_pen && out[1].s.x == 900);
    for i in 1..n as usize {
        check!(contact(&out[i]) && out[i].s.tool_rubber);
    }

    // The whole flip in the lost stretch: the tail carries no buttons, only the eraser's moves, and
    // the device says the eraser end touches. Still not the pen's stroke going on.
    let mut v = Stream::default();
    cut(&mut v, 438);
    for i in 0..5 {
        xy(&mut v, 900 + i, 500);
    }
    let t = tail_of(&v, WRITING, now);
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 6 && info.lifted && !info.keys);
    check!(!contact(&out[0]) && out[0].s.tool_pen && out[0].s.x == WRITING.x);
    check!(contact(&out[1]) && out[1].gap && out[1].s.tool_rubber && out[1].s.x == 900);
}

fn an_eraser_flip_in_the_tail_flips_where_it_happened() {
    let mut s = Stream::default();
    cut(&mut s, 100);
    for i in 0..5 {
        xy(&mut s, 101 + i, 100);
    }
    key(&mut s, PS_BTN_TOOL_PEN, 0);
    report(&mut s);
    key(&mut s, PS_BTN_TOOL_RUBBER, 1);
    xy(&mut s, 300, 300);
    key(&mut s, PS_BTN_TOUCH, 1);
    xy(&mut s, 301, 300);
    for i in 0..4 {
        xy(&mut s, 302 + i, 300);
    }
    let now = PenState { tool_rubber: true, touching: true, x: 305, y: 300, ..Default::default() };
    let t = tail_of(&s, HOVERING, now);
    let mut out = [PenFrame::default(); 32];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 12 && !info.lifted);
    for i in 0..5 {
        check!(out[i].s.tool_pen && !out[i].s.tool_rubber && !contact(&out[i]));
    }
    check!(!out[5].s.tool_pen && !out[5].s.tool_rubber);
    check!(out[6].s.tool_rubber && !contact(&out[6]) && out[6].s.x == 300);
    // The eraser touches down at its own first point, not at the device's present position.
    check!(contact(&out[7]) && out[7].s.x == 301);
    for i in 7..n as usize {
        check!(contact(&out[i]) && out[i].s.tool_rubber);
    }
}

fn a_button_changed_after_the_tail_was_read_comes_last() {
    // The pen came down in the tail; by the time the device was asked it was up again, and
    // EVIOCGKEY took that lift out of the queue: it is reported after the tail, at its end.
    let mut s = Stream::default();
    cut(&mut s, 100);
    key(&mut s, PS_BTN_TOUCH, 1);
    xy(&mut s, 200, 200);
    for i in 1..5 {
        xy(&mut s, 200 + i, 200);
    }
    let now = PenState { tool_pen: true, x: 204, y: 200, ..Default::default() };
    let t = tail_of(&s, HOVERING, now);
    let mut out = [PenFrame::default(); 16];
    let n = tail_no_info(&t, &mut out);
    check!(n == 6);
    for i in 0..5 {
        check!(contact(&out[i]));
    }
    check!(!contact(&out[5]) && out[5].s.x == 204 && !out[5].moved && out[5].t_us == out[4].t_us);
}

fn no_frame_after_the_partial_one_reports_the_device_now() {
    let mut s = Stream::default();
    cut(&mut s, 160);
    let mut now = WRITING;
    now.x = 150;
    now.y = 160;
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 8];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 1 && info.frames == 0 && out[0].gap && out[0].s.x == 150 && contact(&out[0]));
    check!(out[0].t_us == s.ev[2].t_us); // the partial frame's SYN_REPORT
    // Up by now: the stroke ends at the last good point first.
    now.touching = false;
    let t = tail_of(&s, WRITING, now);
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 2 && info.lifted && out[0].s.x == WRITING.x && !contact(&out[0]) && out[1].s.x == 150);
}

fn a_second_drop_and_an_unfinished_frame() {
    let mut s = Stream::default();
    cut(&mut s, 100);
    for i in 0..3 {
        xy(&mut s, 400 + i, 100);
    }
    drop_(&mut s); // overflowed again: what came before is gone too
    put(&mut s, PS_EV_ABS, PS_ABS_X, 450);
    report(&mut s);
    xy(&mut s, 460, 110);
    xy(&mut s, 461, 111);
    put(&mut s, PS_EV_ABS, PS_ABS_X, 462); // a frame still arriving
    let mut now = WRITING;
    now.x = 462;
    now.y = 111;
    let t = tail_of(&s, WRITING, now);
    let mut out = [PenFrame::default(); 16];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 2 && info.frames == 2 && info.skipped == 1);
    check!(out[0].gap && out[0].s.x == 460 && out[1].s.x == 461);
    check!(info.used == s.n() - 1); // the unfinished frame is read on as usual
    // Never more frames than room.
    check!(tail_no_info(&t, &mut out[..1]) == 1);
}

fn contact_without_a_touch_button() {
    // A pen with no BTN_TOUCH (the raw digitizer as the pen): pressure while near.
    let mut s = PenState { tool_pen: true, p: 12, ..Default::default() };
    check!(pensync_contact(&s, false, 0) && !pensync_contact(&s, false, 12) && !pensync_contact(&s, true, 0));
    s.tool_pen = false;
    check!(!pensync_contact(&s, false, 0));
    s.tool_rubber = true;
    check!(pensync_contact(&s, false, 0));
    // Lifted in the lost stretch: the lift at the last good point carries no pressure.
    let mut v = Stream::default();
    cut(&mut v, 100);
    xy(&mut v, 500, 500);
    let good = PenState { tool_pen: true, p: 900, x: 10, y: 10, ..Default::default() };
    let now = PenState { tool_pen: true, p: 0, x: 500, y: 500, ..Default::default() };
    let t = PenTail { good, ev: &v.ev, now, now_ok: PS_F_ALL, touch_btn: false, p_min: 0 };
    let mut out = [PenFrame::default(); 8];
    let mut info = PenTailInfo::default();
    let n = pensync_tail(&t, &mut out, &mut info);
    check!(n == 2 && info.lifted && out[0].s.p == 0 && out[0].s.x == 10 && !pensync_contact(&out[1].s, false, 0));
}

fn kernel_times_in_now_ms_clock() {
    // As now_ms() truncates CLOCK_MONOTONIC: sec * 1000 + nsec / 1000000, in a uint32.
    let mut sec = 0u64;
    while sec < 9000000 {
        let mut ns = 0u64;
        while ns < 1000000000 {
            let now_style = (sec * 1000 + ns / 1000000) as u32;
            check!(pensync_ms(sec * 1000000 + ns / 1000, true, now_style) == now_style);
            ns += 123456789;
        }
        sec += 777777;
    }
    check!(pensync_ms(7856999123, true, 7857330) == 7856999); // a sample 331 ms before it was read
    // Across the clock's wrap (49.7 days of uptime), as now_ms() wraps.
    check!(pensync_ms(4294968000 * 1000 + 500, true, 900) == 704);
    // Falls back to when it was read: not CLOCK_MONOTONIC, no time, from the future, or too old.
    check!(pensync_ms(7856999123, false, 7857330) == 7857330);
    check!(pensync_ms(0, true, 5000) == 5000);
    check!(pensync_ms(5001000, true, 5000) == 5000);
    check!(pensync_ms(1000000, true, 1000 + PENSYNC_K_MAX_AGE_MS) == 1000);
    check!(pensync_ms(1000000, true, 1001 + PENSYNC_K_MAX_AGE_MS) == 1001 + PENSYNC_K_MAX_AGE_MS);
    // Realtime stamps (the clock switch failed unnoticed) are decades off: never used.
    check!(pensync_ms(1793000000 * 1000000, true, 7857330) == 7857330);
}

// The raw digitizer's frames, about 3 ms apart, each relayed as a pen frame a little later. The
// pressure and tilt rise frame by frame, so a pen frame that took another frame's values shows.
const RAW_T0: u64 = 7857000000;
fn raw_frame(i: i32) -> PenRaw {
    PenRaw { t_us: RAW_T0 + 3000 * i as u64, p: 100 + 10 * i, tx: i, ty: -i }
}

fn each_pen_frame_takes_the_pressure_and_tilt_of_its_own_moment() {
    let mut g = PenRawRing::default();
    pensync_raw_clear(&mut g);
    let mut got = PenRaw::default();
    check!(!pensync_raw_at(&g, RAW_T0, &mut got)); // nothing read yet: the newest values, as before
    // Read in turns, a frame of each: the relay's hop from 0.1 ms up to just under a frame.
    const HOPS: [u64; 4] = [0, 100, 1000, 2900];
    for i in 0..20 {
        let r = raw_frame(i);
        pensync_raw_push(&mut g, r.t_us, r.p, r.tx, r.ty);
        for h in 0..4 {
            check!(pensync_raw_at(&g, r.t_us + HOPS[h], &mut got));
            check!(got.p == r.p && got.tx == r.tx && got.ty == r.ty && got.t_us == r.t_us);
        }
    }
    // A wait of 100 ms (the main loop held up): 34 frames of each are read at once, the raw ones
    // first. Each pen frame still takes its own, not the newest: no flat stretch.
    pensync_raw_clear(&mut g);
    for i in 0..34 {
        let r = raw_frame(i);
        pensync_raw_push(&mut g, r.t_us, r.p, r.tx, r.ty);
    }
    let mut distinct = 0;
    let mut last = -1;
    for i in 0..34 {
        check!(pensync_raw_at(&g, raw_frame(i).t_us + 100, &mut got));
        check!(got.p == raw_frame(i).p && got.tx == i && got.ty == -i);
        distinct += (got.p != last) as i32;
        last = got.p;
    }
    check!(distinct == 34);
    // Older than the oldest kept: none (the caller takes the newest values).
    check!(!pensync_raw_at(&g, RAW_T0 - 1, &mut got));
    // A pen frame newer than every raw frame (the pen held still: the input core sends no unchanged
    // values) takes the newest, which is what the digitizer still holds.
    check!(pensync_raw_at(&g, RAW_T0 + 10000000, &mut got) && got.p == raw_frame(33).p);
    // Equal times: the later pushed (a resync's values over the frame it came with).
    pensync_raw_push(&mut g, raw_frame(33).t_us + 3000, 999, 9, 9);
    pensync_raw_push(&mut g, raw_frame(33).t_us + 3000, 1000, 10, 10);
    check!(pensync_raw_at(&g, raw_frame(33).t_us + 3000, &mut got) && got.p == 1000);
}

// A drop of the raw digitizer's events forgets what was kept: the frames after the gap no longer meet
// it. Its resync asks the device and pushes that at the time it asked; pen frames from the lost
// stretch, older than that, take the newest values (the resync's), as before there was a ring.
fn a_raw_drop_forgets_and_the_resync_starts_again() {
    let mut g = PenRawRing::default();
    pensync_raw_clear(&mut g);
    for i in 0..10 {
        pensync_raw_push(&mut g, raw_frame(i).t_us, raw_frame(i).p, i, -i);
    }
    pensync_raw_clear(&mut g); // SYN_DROPPED
    let mut got = PenRaw::default();
    check!(!pensync_raw_at(&g, raw_frame(5).t_us, &mut got));
    let asked = raw_frame(40).t_us;
    pensync_raw_push(&mut g, asked, 777, 7, -7);
    check!(!pensync_raw_at(&g, raw_frame(30).t_us, &mut got)); // in the lost stretch
    check!(pensync_raw_at(&g, asked + 500, &mut got) && got.p == 777 && got.tx == 7 && got.ty == -7);
    pensync_raw_push(&mut g, raw_frame(41).t_us, raw_frame(41).p, 41, -41);
    check!(pensync_raw_at(&g, raw_frame(41).t_us + 100, &mut got) && got.p == raw_frame(41).p);
}

// The ring keeps the last PENSYNC_RAW_N frames (about 0.8 s): a pen frame older than those finds none.
fn the_ring_keeps_the_newest_frames() {
    let mut g = PenRawRing::default();
    pensync_raw_clear(&mut g);
    let n = PENSYNC_RAW_N as i32;
    for i in 0..n + 44 {
        pensync_raw_push(&mut g, raw_frame(i).t_us, raw_frame(i).p, i, -i);
    }
    let mut got = PenRaw::default();
    check!(g.count == n);
    check!(!pensync_raw_at(&g, raw_frame(43).t_us + 100, &mut got));
    check!(pensync_raw_at(&g, raw_frame(44).t_us + 100, &mut got) && got.p == raw_frame(44).p);
    check!(pensync_raw_at(&g, raw_frame(n + 43).t_us, &mut got) && got.tx == n + 43);
}

// The Scribe 2's ranges (grimoire.log, 2026-10-26): the raw digitizer 8.4 units to each pixel.
const HV_RMIN: [i32; 2] = [0, 0];
const HV_RMAX: [i32; 2] = [15624, 20832];
const HV_SMIN: [i32; 2] = [0, 0];
const HV_SMAX: [i32; 2] = [1860, 2480];

// Contact frame i of a stroke that wanders over the whole screen: where the scaled pen is, and the
// raw digitizer's place for it under orientation o (bit 0: x reversed, bit 1: y reversed), a pixel or
// two off as the pen service rounds. Returns (rx, ry, sx, sy).
fn hover_frame(i: i32, o: i32) -> (i32, i32, i32, i32) {
    let sx = (i * 397) % 1861;
    let sy = (i * 911) % 2481;
    let ux = if o & 1 != 0 { 1860 - sx } else { sx };
    let uy = if o & 2 != 0 { 2480 - sy } else { sy };
    // (ux * 8.4f + 0.5f: one fused multiply-add in the C, as clang compiles it)
    let mut rx = fmla(ux as f32, 8.4, 0.5) as i32 + (i % 3 - 1) * 9; // (about a pixel either way)
    let mut ry = fmla(uy as f32, 8.4, 0.5) as i32 - (i % 5 - 2) * 6;
    if rx < 0 {
        rx = 0;
    }
    if ry < 0 {
        ry = 0;
    }
    (rx, ry, sx, sy)
}

fn near(a: f32, b: f32, tol: f32) -> bool {
    a - b <= tol && b - a <= tol
}

fn the_hovering_pen_comes_from_the_raw_digitizer_in_the_scaled_pens_units() {
    let mut h = PenHover::default();
    pensync_hover_init(&mut h, &HV_RMIN, &HV_RMAX, &HV_SMIN, &HV_SMAX);
    let (mut x, mut y) = (0f32, 0f32);
    // Before any contact: both axes as they are, scaled onto the screen.
    check!(pensync_hover_orientation(&h) == 0);
    check!(pensync_hover_place(&h, 840, 1680, &mut x, &mut y) && near(x, 100.0, 0.01) && near(y, 200.0, 0.01));
    check!(pensync_hover_place(&h, 15624, 20832, &mut x, &mut y) && near(x, 1860.0, 0.01) && near(y, 2480.0, 0.01));
    check!(pensync_hover_place(&h, -50, 99999, &mut x, &mut y) && near(x, 0.0, 0.01) && near(y, 2480.0, 0.01)); // (cut)
    // A pen service that keeps both axes as they are: so it stays, a pixel or so from the pen.
    for i in 0..200 {
        let (rx, ry, sx, sy) = hover_frame(i, 0);
        pensync_hover_learn(&mut h, rx, ry, sx, sy);
    }
    check!(pensync_hover_orientation(&h) == 0 && pensync_hover_mean(&h, 0) < 2.0);
    check!(pensync_hover_mean(&h, 1) > 20.0 && pensync_hover_mean(&h, 2) > 20.0 && pensync_hover_mean(&h, 3) > 20.0);
    // One frame far off (a stale raw place) weighs at most PENSYNC_HOVER_CAP: nothing changes.
    pensync_hover_learn(&mut h, 0, 0, 1800, 2400);
    check!(pensync_hover_orientation(&h) == 0);
}

fn a_reversed_axis_is_found_after_enough_contact_frames() {
    for o in 1..4 {
        let mut h = PenHover::default();
        pensync_hover_init(&mut h, &HV_RMIN, &HV_RMAX, &HV_SMIN, &HV_SMAX);
        for i in 0..PENSYNC_HOVER_FRAMES - 1 {
            let (rx, ry, sx, sy) = hover_frame(i, o);
            pensync_hover_learn(&mut h, rx, ry, sx, sy);
        }
        check!(pensync_hover_orientation(&h) == 0); // (not yet: as it is until they decide)
        let (rx, ry, sx, sy) = hover_frame(PENSYNC_HOVER_FRAMES - 1, o);
        pensync_hover_learn(&mut h, rx, ry, sx, sy);
        check!(pensync_hover_orientation(&h) == o);
        let (mut x, mut y) = (0f32, 0f32);
        check!(pensync_hover_place(&h, 840, 1680, &mut x, &mut y));
        check!(near(x, if o & 1 != 0 { 1760.0 } else { 100.0 }, 0.01) && near(y, if o & 2 != 0 { 2280.0 } else { 200.0 }, 0.01));
    }
}

fn no_hover_when_the_raw_place_is_far_from_the_pen_in_every_orientation() {
    let mut h = PenHover::default();
    pensync_hover_init(&mut h, &HV_RMIN, &HV_RMAX, &HV_SMIN, &HV_SMAX);
    for i in 0..PENSYNC_HOVER_FRAMES {
        let (rx, ry, sx, sy) = hover_frame(i, 0);
        pensync_hover_learn(&mut h, rx, ry, if sx > 930 { sx - 40 } else { sx + 40 }, sy); // 40 px off, whichever way
    }
    let (mut x, mut y) = (-1f32, -1f32);
    check!(pensync_hover_orientation(&h) == -1 && !pensync_hover_place(&h, 840, 1680, &mut x, &mut y) && x == -1.0);
    // It keeps comparing: frames that agree bring it back once the mean is near again.
    for i in 0..400 {
        let (rx, ry, sx, sy) = hover_frame(i, 0);
        pensync_hover_learn(&mut h, rx, ry, sx, sy);
    }
    check!(pensync_hover_orientation(&h) == 0 && pensync_hover_place(&h, 840, 1680, &mut x, &mut y));
    // A long session halves what it has compared, so it never overflows and still follows.
    for i in 0..9000 {
        let (rx, ry, sx, sy) = hover_frame(i, 0);
        pensync_hover_learn(&mut h, rx, ry, sx, sy);
    }
    check!(h.n < 4096 && h.n >= 2048 && pensync_hover_orientation(&h) == 0);
    // A digitizer without ranges: no place at all.
    let zero = [0, 0];
    pensync_hover_init(&mut h, &zero, &zero, &HV_SMIN, &HV_SMAX);
    check!(!pensync_hover_place(&h, 840, 1680, &mut x, &mut y));
}

#[test]
fn test_pensync() {
    run!(the_p28_shape_reports_the_tail_in_order_and_never_jumps_ahead);
    run!(an_axis_missing_from_the_first_frame_takes_its_first_value_in_the_tail);
    run!(a_lift_inside_the_tail_lifts_there);
    run!(a_lift_after_the_tails_writing_keeps_the_writing);
    run!(a_lift_in_the_lost_stretch_ends_the_stroke_at_the_last_good_point);
    run!(a_touch_at_the_first_frame_means_a_lift_in_the_lost_stretch);
    run!(an_eraser_flip_at_the_first_frame_or_in_the_lost_stretch_ends_the_pen_stroke);
    run!(an_eraser_flip_in_the_tail_flips_where_it_happened);
    run!(a_button_changed_after_the_tail_was_read_comes_last);
    run!(no_frame_after_the_partial_one_reports_the_device_now);
    run!(a_second_drop_and_an_unfinished_frame);
    run!(contact_without_a_touch_button);
    run!(kernel_times_in_now_ms_clock);
    run!(each_pen_frame_takes_the_pressure_and_tilt_of_its_own_moment);
    run!(a_raw_drop_forgets_and_the_resync_starts_again);
    run!(the_ring_keeps_the_newest_frames);
    run!(the_hovering_pen_comes_from_the_raw_digitizer_in_the_scaled_pens_units);
    run!(a_reversed_axis_is_found_after_enough_contact_frames);
    run!(no_hover_when_the_raw_place_is_far_from_the_pen_in_every_orientation);
    done();
}
