// test_fingershape: the recorded one-finger shape gestures' pure module (docs/index-hook/
// index-hook-shape-gestures-2026-10-27.json, section 10.1): the capture state machine, every gate on
// both sides of its edge, the integer $P matcher's invariances and exact scores, the decision
// (threshold, margin, structure guard), the record checks (consistency, separation, the slip bank)
// and the gestures.ini shape value. The shapes come from fingershape_synth, which draws them with
// integer arithmetic and a fixed-seed PRNG: every score asserted here is the same on every
// architecture by construction, so the numbers are exact, not ranges.
#![allow(clippy::needless_range_loop)]
mod fingershape_synth;
mod testlib;
use fingershape_synth::*;
use grimoire::prelude::*;
use testlib::*;

fn none() {}

/// A shape recorded the way the record card records it: three recording-like draws through the
/// real capture and gates, then fs_prepare.
fn rec(name: &str, seed: u32) -> FsGesture {
    let mut r = Rng::new(seed);
    let mut ss = Vec::new();
    for k in 0..3 {
        let dv = vary_record(&mut r, seed * 10 + k);
        let mut s = FsSample::default();
        let l = capture(&shape_frames(name, &dv), 1000, FsMode::Record, &mut s);
        check!(l == FsLift::Shape);
        ss.push(s);
    }
    let mut g = FsGesture::default();
    fs_prepare(&mut g, &ss);
    g
}

/// Frames along a polyline at constant speed, landing at t = 0 and lifting at ms, with every vertex
/// a frame and the legs cut into pieces of about step px.
fn fpath(v: &[(i32, i32)], ms: u32, step: i32) -> Vec<Frame> {
    let len = |a: (i32, i32), b: (i32, i32)| -> i64 { isqrt(((b.0 - a.0) as i64).pow(2) + ((b.1 - a.1) as i64).pow(2)) };
    let total: i64 = v.windows(2).map(|w| len(w[0], w[1])).sum();
    let mut out = vec![Frame { x: v[0].0, y: v[0].1, t: 0 }];
    let mut acc = 0i64;
    for w in v.windows(2) {
        let l = len(w[0], w[1]);
        let n = (l / step as i64).max(1);
        for k in 1..=n {
            let x = w[0].0 + ((w[1].0 - w[0].0) as i64 * k / n) as i32;
            let y = w[0].1 + ((w[1].1 - w[0].1) as i64 * k / n) as i32;
            let t = ((acc + l * k / n) * ms as i64 / total.max(1)) as u32;
            out.push(Frame { x, y, t });
        }
        acc += l;
    }
    out
}

fn moved(v: &[(i32, i32)], dx: i32, dy: i32) -> Vec<(i32, i32)> {
    v.iter().map(|&(x, y)| (x + dx, y + dy)).collect()
}

fn lift_of(fr: &[Frame]) -> FsLift {
    let mut s = FsSample::default();
    capture(fr, 1000, FsMode::Runtime, &mut s)
}

// ---- the shared table ----

fn shared_gates() {
    // One table for the page and the record card (section 3): the same object, not a copy.
    check!(std::ptr::eq(fs_gates(FsMode::Runtime), fs_gates(FsMode::Record)));
    check!(std::ptr::eq(fs_gates(FsMode::Runtime), &FS_GATES));
    let g = FS_GATES;
    check!(g.slop_mm10 == 60 && g.trail_ms == 150 && g.lead_ms == 400 && g.dur_max_ms == 2500);
    check!(g.pause_mm10 == 15 && g.pause_ms == 500 && g.frames_min == 8);
    check!(g.box_mm10 == 200 && g.path_mm10 == 300 && g.speed_mm_s == 50);
    check!(g.chord_pct == 15 && g.tangle_x10 == 50);
    check!(g.swipe_mm10 == 300 && g.swipe_ms == 600 && g.swipe_tan_x10000 == 5774);
    // And the same verdict and sample for the same touch, shape or slip, in either mode.
    let mut r = Rng::new(4242);
    let mut differ = 0;
    for k in 0..60u32 {
        let fr = if k % 2 == 0 {
            shape_frames(SHAPES[k as usize % SHAPES.len()], &vary_use(&mut r, k))
        } else {
            let kind = SLIPS[k as usize % SLIPS.len()];
            frames_of(&place(&slip_path(kind, &mut r), false, &vary_use(&mut r, k)), &vary_use(&mut r, k))
        };
        let (mut a, mut b) = (FsSample::default(), FsSample::default());
        let la = capture(&fr, 1000, FsMode::Runtime, &mut a);
        let lb = capture(&fr, 1000, FsMode::Record, &mut b);
        if la != lb || a != b {
            differ += 1;
        }
    }
    check!(differ == 0);
}

fn mm_px() {
    // The design's numbers at the Scribe's 11.81 px/mm.
    check!(fs_mm_px(60, PX100) == 71);
    check!(fs_mm_px(200, PX100) == 236);
    check!(fs_mm_px(300, PX100) == 354);
    check!(fs_mm_px(15, PX100) == 18);
    check!(fs_mm_px(55, PX100) == 65);
    check!(fs_mm_px(65, PX100) == 77);
}

fn sine_table() {
    check!(fs_sin_q14(0) == 0 && fs_sin_q14(90) == 16384 && fs_sin_q14(180) == 0 && fs_sin_q14(270) == -16384);
    check!(fs_sin_q14(30) == 8192 && fs_sin_q14(-30) == -8192 && fs_sin_q14(390) == 8192);
    check!(fs_cos_q14(10) == 16135 && fs_sin_q14(10) == 2845 && fs_cos_q14(-10) == 16135);
    check!(fs_cos_q14(120) == -8192 && fs_cos_q14(240) == -8192 && fs_cos_q14(300) == 8192);
}

// ---- capture (section 2) ----

fn capture_slop() {
    let mut s = FsStroke::default();
    // 5.5 mm (65 px) never leaves the slop: a tap's band, nothing changes
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_add(&mut s, 1065, 1000, 100, false) == FsState::Pending);
    check!(fs_tick(&mut s, 300) == FsState::Pending);
    let mut out = FsSample::default();
    check!(fs_finish(&mut s, 320, FsMode::Runtime, &mut out) == FsLift::Nothing);
    check!(s.state == FsState::Idle);
    // 6.5 mm (77 px) does
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_add(&mut s, 1077, 1000, 100, false) == FsState::Pending);
    check!(fs_tick(&mut s, 160) == FsState::Drawing);
}

fn capture_trail_delay() {
    let mut s = FsStroke::default();
    // out of the slop at 50 ms, but a shape only from 150 ms: a late second finger never shows a trail
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_add(&mut s, 1100, 1000, 50, false) == FsState::Pending);
    check!(fs_tick(&mut s, 140) == FsState::Pending);
    check!(fs_tick(&mut s, 160) == FsState::Drawing);
    // the same by a frame
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_add(&mut s, 1100, 1000, 50, false) == FsState::Pending);
    check!(fs_add(&mut s, 1110, 1000, 140, false) == FsState::Pending);
    check!(fs_add(&mut s, 1120, 1000, 160, false) == FsState::Drawing);
}

fn capture_lead() {
    let mut s = FsStroke::default();
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_add(&mut s, 1077, 1000, 390, false) == FsState::Drawing);
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_add(&mut s, 1077, 1000, 410, false) == FsState::Dead);
    check!(s.dead == FsReason::LateStart);
    // dead for the rest of the touch: further moves are ignored, the lift reports why
    let f = s.frames;
    check!(fs_add(&mut s, 1500, 1000, 450, false) == FsState::Dead);
    check!(s.frames == f);
    let mut out = FsSample::default();
    check!(fs_finish(&mut s, 500, FsMode::Record, &mut out) == FsLift::Refused(FsReason::LateStart));
}

/// A finger that left the slop and kept moving 30 px every 20 ms until 300 ms.
fn drawing(s: &mut FsStroke, t0: u32) {
    fs_begin(s, 1000, 1000, t0, PX100);
    for k in 1..=15u32 {
        fs_add(s, 1000 + 30 * k as i32, 1000, t0.wrapping_add(20 * k), false);
    }
}

fn capture_pause() {
    let mut s = FsStroke::default();
    // a rest of 490 ms, then on: still a shape
    drawing(&mut s, 0);
    check!(s.state == FsState::Drawing);
    check!(fs_add(&mut s, 1450, 1100, 300 + 490, false) == FsState::Drawing);
    // a rest of 510 ms: "hold still to cancel"
    drawing(&mut s, 0);
    check!(fs_add(&mut s, 1450, 1100, 300 + 510, false) == FsState::Dead);
    check!(s.dead == FsReason::Paused);
    // the same on wall time (editor_tick), and a drift within 1.5 mm (18 px) is still a rest
    drawing(&mut s, 0);
    check!(fs_add(&mut s, 1460, 1000, 400, false) == FsState::Drawing);
    check!(fs_add(&mut s, 1450, 1010, 600, false) == FsState::Drawing);
    check!(fs_tick(&mut s, 300 + 490) == FsState::Drawing);
    check!(fs_tick(&mut s, 300 + 500) == FsState::Dead);
    check!(s.dead == FsReason::Paused);
    let mut out = FsSample::default();
    check!(fs_finish(&mut s, 900, FsMode::Runtime, &mut out) == FsLift::Refused(FsReason::Paused));
}

fn capture_overtime() {
    let mut s = FsStroke::default();
    drawing(&mut s, 0);
    for k in 1..=105u32 {
        // moving on (no pause) in a circle of 100 px
        let a = (k * 20) as i32;
        fs_add(&mut s, 1450 + fs_cos_q14(a) * 100 / 16384, 1000 + fs_sin_q14(a) * 100 / 16384, 300 + 20 * k, false);
    }
    check!(s.state == FsState::Drawing);
    check!(fs_tick(&mut s, 2490) == FsState::Drawing);
    check!(fs_tick(&mut s, 2510) == FsState::Dead);
    check!(s.dead == FsReason::TooSlow);
}

fn capture_cancels() {
    let mut s = FsStroke::default();
    let mut out = FsSample::default();
    // palm-sized at any frame
    drawing(&mut s, 0);
    check!(fs_add(&mut s, 1460, 1000, 320, true) == FsState::Dead);
    check!(fs_finish(&mut s, 400, FsMode::Runtime, &mut out) == FsLift::Refused(FsReason::Broad));
    // the off-screen cancel: drawing reports a cancel (the trail goes), pending nothing
    drawing(&mut s, 0);
    check!(fs_abort(&mut s) == FsLift::Refused(FsReason::Cancelled));
    check!(s.state == FsState::Idle);
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    check!(fs_abort(&mut s) == FsLift::Nothing);
    // the link hold fired
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    fs_kill(&mut s, FsReason::Held);
    check!(s.state == FsState::Dead);
    check!(fs_finish(&mut s, 700, FsMode::Runtime, &mut out) == FsLift::Refused(FsReason::Held));
    // a lift before any of it: a tap
    fs_begin(&mut s, 1000, 1000, 0, PX100);
    fs_add(&mut s, 1010, 1000, 30, false);
    check!(fs_finish(&mut s, 60, FsMode::Runtime, &mut out) == FsLift::Nothing);
    // nothing happens to an idle stroke
    let mut i = FsStroke::default();
    check!(fs_add(&mut i, 5, 5, 5, false) == FsState::Idle);
    check!(fs_points(&i).is_empty());
    check!(fs_box(&i) == RECT_EMPTY);
}

fn capture_clock_wrap() {
    // now_ms() wraps: a shape drawn across the wrap is the same shape
    let fr = shape_frames("Z", &Draw::default());
    let (mut a, mut b) = (FsSample::default(), FsSample::default());
    check!(capture(&fr, 1000, FsMode::Runtime, &mut a) == FsLift::Shape);
    check!(capture(&fr, u32::MAX - 300, FsMode::Runtime, &mut b) == FsLift::Shape);
    check!(a == b);
}

fn capture_decimation() {
    // 600 frames (2.4 s at 4 ms): the buffer halves itself and keeps the whole extent
    let mut v = Vec::new();
    for k in 0..=360 {
        v.push((1000 + 236 * fs_cos_q14(k) / 16384, 1200 + 236 * fs_sin_q14(k) / 16384));
    }
    let fr = fpath(&v, 2400, 2);
    let mut s = FsStroke::default();
    let mut out = FsSample::default();
    let l = capture_into(&mut s, &fr, 1000, FsMode::Runtime, &mut out);
    check!(l == FsLift::Shape);
    check!(s.frames as usize == fr.len());
    check!(fr.len() > 2 * FS_RAW_MAX);
    let pts = fs_points(&s);
    check!(pts.len() < FS_RAW_MAX);
    check!(pts[0] == FsPt { x: fr[0].x, y: fr[0].y });
    check!(*pts.last().unwrap() == FsPt { x: fr[fr.len() - 1].x, y: fr[fr.len() - 1].y });
    check!(fs_box(&s) == Rect { x: 764, y: 964, w: 473, h: 473 });
    let d = fs_distance(&out, &ideal("circle", &Draw::default()));
    eprintln!("  decimated circle vs ideal circle: {d}");
    check!(d == DECIMATED_CIRCLE);
}
const DECIMATED_CIRCLE: i32 = 1;

// ---- the gates (section 3), both sides of every edge ----

/// N in a box of b px: never a line, never tangled.
fn n_shape(b: i32) -> Vec<(i32, i32)> {
    moved(&[(0, b), (0, 0), (b, b), (b, 0)], 800, 1000)
}

fn gate_frames() {
    // 7 frames are refused, 8 pass (an N of 300 px, 100 ms apart)
    let v7 = moved(&[(0, 300), (0, 150), (0, 0), (150, 150), (300, 300), (300, 150), (300, 0)], 800, 1000);
    let fr: Vec<Frame> = v7.iter().enumerate().map(|(k, &(x, y))| Frame { x, y, t: 100 * k as u32 }).collect();
    check!(lift_of(&fr) == FsLift::Refused(FsReason::Frames));
    let v8 = moved(&[(0, 300), (0, 150), (0, 0), (150, 150), (300, 300), (300, 150), (300, 75), (300, 0)], 800, 1000);
    let fr: Vec<Frame> = v8.iter().enumerate().map(|(k, &(x, y))| Frame { x, y, t: 85 * k as u32 }).collect();
    check!(lift_of(&fr) == FsLift::Shape);
}

fn gate_box() {
    // 19 mm (224 px) and 21 mm (248 px)
    check!(lift_of(&fpath(&n_shape(224), 800, 8)) == FsLift::Refused(FsReason::TooSmall));
    check!(lift_of(&fpath(&n_shape(248), 800, 8)) == FsLift::Shape);
}

fn gate_path() {
    // a V 21 mm wide: legs to 118 px down make a 29 mm path (342 px), to 135 px a 31 mm one (366 px)
    let v = |h: i32| moved(&[(0, 0), (124, h), (248, 0)], 800, 1000);
    check!(lift_of(&fpath(&v(118), 600, 8)) == FsLift::Refused(FsReason::TooSmall));
    check!(lift_of(&fpath(&v(135), 600, 8)) == FsLift::Shape);
}

fn gate_duration() {
    // an N of 60 mm (708 px) drawn at an even pace: 2490 ms passes, 2510 ms is too slow
    check!(lift_of(&fpath(&n_shape(708), 2490, 8)) == FsLift::Shape);
    check!(lift_of(&fpath(&n_shape(708), 2510, 8)) == FsLift::Refused(FsReason::TooSlow));
}

fn gate_speed() {
    // An N of 30 mm: a 102.3 mm path (1209 px), out of the slop 72 px in. At an even pace the speed
    // from the exit to the lift is 102.3 mm x (1 - 72/1209) / T: 45 mm/s at 2418 ms, 55 at 1978.
    check!(lift_of(&fpath(&n_shape(354), 2418, 8)) == FsLift::Refused(FsReason::TooSlow));
    check!(lift_of(&fpath(&n_shape(354), 1978, 8)) == FsLift::Shape);
}

fn gate_chord() {
    // A chevron on a 400 px chord: 56 px off it is 14% (a line), 64 px is 16% (a shape).
    let c = |h: i32| moved(&[(0, 0), (200, h), (400, 0)], 800, 1000);
    // quick and level: the line is a one-finger swipe
    check!(lift_of(&fpath(&c(56), 500, 8)) == FsLift::Swipe(FsSwipe::Right));
    check!(lift_of(&fpath(&c(64), 500, 8)) == FsLift::Shape);
    // slower than a swipe: too straight
    check!(lift_of(&fpath(&c(56), 800, 8)) == FsLift::Refused(FsReason::Straight));
}

fn gate_closed() {
    // A closed shape's chord is short: the distance from the start stands in, so a circle is no line.
    let mut v = Vec::new();
    for k in 0..=72 {
        v.push((1000 + 300 * fs_cos_q14(5 * k) / 16384, 1000 + 300 * fs_sin_q14(5 * k) / 16384));
    }
    check!(lift_of(&fpath(&v, 900, 8)) == FsLift::Shape);
}

fn gate_tangle() {
    // A 300 px square traced on past one lap: 2079 px is 4.9 times the diagonal (424 px), 2164 px
    // is 5.1 times.
    let lap = [(0, 0), (300, 0), (300, 300), (0, 300), (0, 0), (300, 0), (300, 300)];
    let mut a = lap.to_vec();
    a.push((21, 300));
    let mut b = lap.to_vec();
    b.push((0, 300));
    b.push((0, 236));
    check!(lift_of(&fpath(&moved(&a, 800, 1000), 1500, 8)) == FsLift::Shape);
    check!(lift_of(&fpath(&moved(&b, 800, 1000), 1500, 8)) == FsLift::Refused(FsReason::Tangled));
    // and a scribble is refused by the gates whatever is recorded
    let scrub = moved(&[(0, 0), (400, 30), (0, 60), (400, 20), (0, 50), (400, 10), (0, 40), (400, 30), (0, 20)], 800, 1000);
    check!(lift_of(&fpath(&scrub, 1200, 8)) == FsLift::Refused(FsReason::Tangled));
}

fn gate_swipe() {
    // 40 mm (472 px) lines: within 30 degrees of an axis and 600 ms they are swipes, else too straight
    let line = |deg: i32, ms: u32| {
        let v = [(1200, 1200), (1200 + 472 * fs_cos_q14(deg) / 16384, 1200 + 472 * fs_sin_q14(deg) / 16384)];
        lift_of(&fpath(&v, ms, 8))
    };
    check!(line(0, 400) == FsLift::Swipe(FsSwipe::Right));
    check!(line(90, 400) == FsLift::Swipe(FsSwipe::Down));
    check!(line(180, 400) == FsLift::Swipe(FsSwipe::Left));
    check!(line(270, 400) == FsLift::Swipe(FsSwipe::Up));
    check!(line(29, 400) == FsLift::Swipe(FsSwipe::Right));
    check!(line(31, 400) == FsLift::Refused(FsReason::Straight));
    check!(line(-119, 400) == FsLift::Swipe(FsSwipe::Up));
    check!(line(0, 600) == FsLift::Swipe(FsSwipe::Right));
    check!(line(0, 610) == FsLift::Refused(FsReason::Straight));
    // 29 mm is not enough for a swipe (and too small for a shape: a line's box is its length)
    let v = [(1200, 1200), (1200 + 342, 1200)];
    check!(lift_of(&fpath(&v, 400, 8)) == FsLift::Refused(FsReason::TooSmall));
}

fn two_strokes() {
    let st = FsLift::Refused(FsReason::Straight);
    check!(fs_two_strokes(st, 1000, 1700));
    check!(!fs_two_strokes(st, 1000, 1710));
    check!(!fs_two_strokes(FsLift::Refused(FsReason::TooSmall), 1000, 1100));
    check!(fs_two_strokes(st, u32::MAX - 100, 500));
}

// ---- normalization and the $P distance (section 4) ----

fn normalize() {
    // The larger side spans 0-255, the smaller is centred, the order is kept.
    let mut s = FsSample::default();
    fs_sample_of(&[FsPt { x: 100, y: 500 }, FsPt { x: 900, y: 500 }], &mut s);
    check!(s.xy[0] == 0 && s.xy[1] == 127 && s.xy[62] == 255 && s.xy[63] == 127);
    check!(s.xy[2] == 8 && s.xy[4] == 16);
    let z = ideal("Z", &Draw::default());
    check!(z.xy[0] == 0 && z.xy[1] == 0 && z.xy[62] == 255 && z.xy[63] == 255);
    // a lone point and an empty path give the centre
    fs_sample_of(&[FsPt { x: 7, y: 7 }], &mut s);
    check!(s == FsSample::default());
    fs_sample_of(&[], &mut s);
    check!(s == FsSample::default());
}

fn invariances() {
    let d = Draw::default();
    // translation: the very same sample anywhere on the page
    let a = ideal("Z", &Draw { size_mm: 20, ..d });
    let c = ideal("Z", &Draw { size_mm: 20, cx: 300, cy: 2000, ..d });
    check!(a == c);
    // scale x5
    let b = ideal("Z", &Draw { size_mm: 100, ..d });
    check!(fs_distance(&a, &b) == 0);
    // direction and start point: a circle drawn either way, from anywhere, is one gesture
    let c0 = ideal("circle", &d);
    check!(fs_distance(&c0, &ideal("circle", &Draw { reverse: true, ..d })) == 0);
    check!(fs_distance(&c0, &ideal("circle", &Draw { shift: 25, ..d })) == 26);
    check!(fs_distance(&c0, &ideal("circle", &Draw { shift: 50, ..d })) == 34);
    check!(fs_distance(&c0, &ideal("circle", &Draw { reverse: true, shift: 75, ..d })) == 28);
    check!(fs_distance(&c0, &ideal("circle", &Draw { shift: 13, ..d })) == 13);
    let circle = rec("circle", 11);
    for (rev, shift) in [(false, 0), (true, 0), (false, 25), (false, 50), (true, 75)] {
        let s = ideal("circle", &Draw { reverse: rev, shift, ..d });
        check!(fs_match(&s, &[circle], &mut none).fired == Some(0));
    }
    // a square started at each corner
    let sq = ideal("square", &d);
    let mut ds = Vec::new();
    for shift in [25, 50, 75] {
        ds.push(fs_distance(&sq, &ideal("square", &Draw { shift, ..d })));
    }
    eprintln!("  square from other corners: {ds:?}");
    check!(ds == SQUARE_SHIFTS);
    // symmetric
    let v = ideal("V", &d);
    check!(fs_distance(&v, &c0) == fs_distance(&c0, &v));
}

fn v_and_caret() {
    // V and ^ are not one shape turned: 264 per mille apart, over 2 x T_MAX.
    let d = Draw::default();
    let v = ideal("V", &d);
    let c = ideal("caret", &d);
    check!(fs_distance(&v, &v) == 0);
    check!(fs_distance(&v, &c) == 264);
    check!(fs_distance(&c, &v) == 264);
    check!(fs_distance(&v, &c) >= 2 * FS_T_MAX);
    // Recorded for two actions, each draw fires its own, never the other.
    let gv = rec("V", 21);
    let gc = rec("caret", 22);
    eprintln!("  V: spread {} thr {} ratio {}; caret: spread {} thr {} ratio {}", gv.spread, gv.thr, gv.ratio, gc.spread, gc.thr, gc.ratio);
    let gs = [gv, gc];
    let mut r = Rng::new(23);
    let (mut fv, mut fc, mut wrong) = (0, 0, 0);
    for k in 0..30 {
        for (i, name) in ["V", "caret"].iter().enumerate() {
            let mut dv = vary_use(&mut r, 100 + k);
            dv.rot = dv.rot.clamp(-8, 8);
            let mut s = FsSample::default();
            if capture(&shape_frames(name, &dv), 1000, FsMode::Runtime, &mut s) != FsLift::Shape {
                continue;
            }
            match fs_match(&s, &gs, &mut none).fired {
                Some(j) if j == i => {
                    if i == 0 {
                        fv += 1
                    } else {
                        fc += 1
                    }
                }
                Some(_) => wrong += 1,
                None => {}
            }
        }
    }
    eprintln!("  V fired {fv}/30, caret fired {fc}/30, wrong {wrong}");
    check!(wrong == 0);
    check!(fv == V_FIRED && fc == CARET_FIRED);
    // each candidate's exact distances to both
    let mv = fs_match(&ideal("V", &d), &gs, &mut none);
    let mc = fs_match(&ideal("caret", &d), &gs, &mut none);
    eprintln!("  ideal V: {mv:?}\n  ideal caret: {mc:?}");
    check!(mv.fired == Some(0) && mc.fired == Some(1));
    check!(mv.d1 == V_D1 && mv.d2 == FS_FAR + 1 && mc.d1 == CARET_D1 && mc.d2 == FS_FAR + 1);
}
const V_FIRED: i32 = 30;
const CARET_FIRED: i32 = 30;
const V_D1: i32 = 22;
const CARET_D1: i32 = 14;

fn rotation() {
    // Z tolerates +-9 degrees (the -10/0/+10 tries) and not +-25.
    let z = rec("Z", 5);
    check!(z.spread == 37 && z.thr == 78 && z.ratio == 2334);
    let d = Draw::default();
    let at = |rot: i32| {
        let s = ideal("Z", &Draw { rot, ..d });
        (fs_gesture_distance(&z, &s), fs_match(&s, &[z], &mut none))
    };
    let expect = [(-25, 178, false), (-15, 121, false), (-9, 35, true), (0, 22, true), (9, 37, true), (15, 64, true), (25, 152, false)];
    for (rot, dist, fires) in expect {
        let (dd, m) = at(rot);
        check!(dd == dist);
        check!(m.fired.is_some() == fires);
        if !fires {
            check!(m.why == FsReason::NoMatch);
        }
    }
}

fn thresholds() {
    check!(fs_threshold(0) == 70);
    check!(fs_threshold(31) == 70);
    check!(fs_threshold(32) == 72);
    check!(fs_threshold(40) == 82);
    check!(fs_threshold(50) == 95);
    check!(fs_threshold(60) == 95);
    check!(fs_threshold(200) == 95);
    // derived from the samples: spread, T_g, the ratio, and the medoid
    let g = rec("S", 31);
    eprintln!("  S: spread {} thr {} ratio {} medoid {}", g.spread, g.thr, g.ratio, fs_medoid(&g));
    check!(g.n == 3);
    let mut sp = 0;
    for i in 0..3 {
        for j in i + 1..3 {
            sp = sp.max(fs_distance(&g.samples[i], &g.samples[j]));
        }
    }
    check!(g.spread == sp);
    check!(g.thr == fs_threshold(g.spread));
    check!((g.spread, g.thr, g.ratio, fs_medoid(&g)) == S_PREPARED);
    // two samples work too; more than three are not used
    let mut g2 = FsGesture::default();
    fs_prepare(&mut g2, &g.samples[..2]);
    check!(g2.n == 2 && g2.spread == fs_distance(&g.samples[0], &g.samples[1]));
    let four = [g.samples[0], g.samples[1], g.samples[2], ideal("V", &Draw::default())];
    let mut g4 = FsGesture::default();
    fs_prepare(&mut g4, &four);
    check!(g4.n == 3 && g4.spread == g.spread);
}
const S_PREPARED: (i32, i32, i32, usize) = (28, 70, 1926, 1);

// ---- the decision ----

fn unknown_shapes() {
    // N, Z, S and a circle recorded: no other shape fires anything.
    let gs = [rec("N", 41), rec("Z", 42), rec("S", 43), rec("circle", 44)];
    let d = Draw::default();
    let mut fired = Vec::new();
    for name in SHAPES {
        let m = fs_match(&ideal(name, &d), &gs, &mut none);
        eprintln!("  {name:9} {m:?}");
        if let Some(i) = m.fired {
            fired.push((*name, i));
        }
    }
    check!(fired == [("N", 0), ("Z", 1), ("S", 2), ("circle", 3)]);
    let sp = fs_match(&ideal("star", &d), &gs, &mut none);
    check!(sp.fired.is_none() && sp.why == FsReason::NoMatch);
    // nothing recorded: nothing fires
    let m = fs_match(&ideal("Z", &d), &[], &mut none);
    check!(m.fired.is_none() && m.why == FsReason::NoMatch && m.best == -1 && m.d1 == FS_FAR + 1);
}

fn margin() {
    // Two actions whose shapes look alike as clouds (a circle, and one drawn twice over): a circle
    // is as near to both, so nothing fires.
    let gs = [rec("circle", 51), rec("dcircle", 52)];
    let m = fs_match(&ideal("circle", &Draw::default()), &gs, &mut none);
    eprintln!("  margin: {m:?}");
    check!(m.fired.is_none() && m.why == FsReason::Unclear);
    check!((m.best, m.d1, m.thr, m.d2) == MARGIN);
    check!(m.d2 - m.d1 < FS_MARGIN_MIN);
}
const MARGIN: (i32, i32, i32, i32) = (0, 16, 72, 16);
const SQUARE_SHIFTS: [i32; 3] = [30, 43, 30];
const ALPHA_N: (i32, i32, i32) = (112, 95, 81);

fn structure_guard() {
    // A circle recorded alone. A spiral of two tight turns and a circle drawn twice are near it as
    // clouds, but their path is 1.8-2x as long for the size: the guard refuses them.
    let gs = [rec("circle", 61)];
    let d = Draw::default();
    let ts = fs_match(&ideal("tspiral", &d), &gs, &mut none);
    let dc = fs_match(&ideal("dcircle", &d), &gs, &mut none);
    let c = fs_match(&ideal("circle", &d), &gs, &mut none);
    eprintln!("  guard: tspiral {ts:?}\n  dcircle {dc:?}\n  circle {c:?}; circle ratio {}", gs[0].ratio);
    check!(ts.fired.is_none() && ts.why == FsReason::Structure && ts.d1 <= ts.thr);
    check!(dc.fired.is_none() && dc.why == FsReason::Structure && dc.d1 <= dc.thr);
    check!(c.fired == Some(0));
    check!((ts.d1, dc.d1, c.d1) == GUARD_D1);
}
const GUARD_D1: (i32, i32, i32) = (44, 5, 5);

fn pump() {
    // work_pump every 4 samples: 8 shapes of 3 samples are 24, so 6 calls
    let mut gs = Vec::new();
    for (k, name) in ["N", "Z", "S", "circle", "alpha", "M", "e", "two"].iter().enumerate() {
        gs.push(rec(name, 70 + k as u32));
    }
    let mut n = 0;
    fs_match(&ideal("Z", &Draw::default()), &gs, &mut || n += 1);
    check!(n == 6);
}

// ---- recording (sections 5 and 6.3) ----

fn record_first() {
    // Sample 1's quick slip check at T_MIN refuses the one-turn shapes at once.
    let mut r = Rng::new(81);
    for (name, slip) in [
        ("V", true),
        ("caret", true),
        ("lt", true),
        ("gt", true),
        ("check", true),
        ("L", true),
        ("U", true),
        ("C", true),
        ("N", false),
        ("Z", false),
        ("S", false),
        ("circle", false),
    ] {
        let dv = vary_record(&mut r, 1);
        let mut s = FsSample::default();
        check!(capture(&shape_frames(name, &dv), 1000, FsMode::Record, &mut s) == FsLift::Shape);
        let res = fs_record_first(&s, &mut none);
        if res != if slip { Err(FsReason::Slip) } else { Ok(()) } {
            eprintln!("  FIRST {name}: {res:?}");
        }
        check!(res == if slip { Err(FsReason::Slip) } else { Ok(()) });
    }
}

fn record_bank() {
    // The full check before Use: one-turn shapes are refused, two turns or a loop pass.
    let mut got = Vec::new();
    for (k, name) in ["V", "caret", "check", "lt", "gt", "L", "U", "C", "N", "Z", "S", "circle"].iter().enumerate() {
        let g = rec(name, 90 + k as u32);
        let mut pumps = 0;
        let hit = fs_bank_hit(&g, g.thr, &mut || pumps += 1);
        check!(fs_record_bank(&g, &mut none) == if hit.is_some() { Err(FsReason::Slip) } else { Ok(()) });
        eprintln!("  bank {name}: thr {} hit {hit:?} pumps {pumps}", g.thr);
        got.push(hit);
        if hit.is_none() {
            // the whole bank was walked, pumped every 64 paths
            check!(pumps == (fs_bank_len() - 1) / FS_BANK_PUMP);
        }
    }
    check!(got == BANK_HITS);
    // the bank itself: 1620 paths, generated on the fly
    check!(fs_bank_len() == 1620);
    let mut p = [FsPt::default(); FS_BANK_PTS];
    check!(fs_bank_path(0, &mut p) == 3);
    eprintln!("  bank path 0: {:?}", &p[..3]);
    check!(p[..3] == [FsPt { x: 0, y: 0 }, FsPt { x: 800, y: 0 }, FsPt { x: 3571, y: 1600 }]);
    check!(fs_bank_path(1619, &mut p) == 45);
    check!(fs_bank_path(1620, &mut p) == 0);
}
const BANK_HITS: [Option<usize>; 12] =
    [Some(761), Some(772), Some(240), Some(676), Some(757), Some(297), Some(1574), Some(1615), None, None, None, None];

fn record_consistent() {
    // A later sample within 60 per mille of an earlier one is kept; 65 is "not like the first one".
    let z0 = ideal("Z", &Draw::default());
    let near = Draw { size_mm: 70, rot: 5, aspect: 92, reverse: false, shift: 16, ms: 641, jitter: 8, wobble: 32, cx: 988, cy: 1351, seed: 22 };
    let far = Draw { size_mm: 84, rot: -6, aspect: 91, reverse: false, shift: 47, ms: 670, jitter: 3, wobble: 21, cx: 838, cy: 1292, seed: 11 };
    let (mut a, mut b) = (FsSample::default(), FsSample::default());
    check!(capture(&shape_frames("Z", &near), 1000, FsMode::Record, &mut a) == FsLift::Shape);
    check!(capture(&shape_frames("Z", &far), 1000, FsMode::Record, &mut b) == FsLift::Shape);
    check!(fs_distance(&z0, &a) == 56);
    check!(fs_distance(&z0, &b) == 65);
    check!(fs_record_consistent(&a, &[z0]).is_ok());
    check!(fs_record_consistent(&b, &[z0]) == Err(FsReason::Inconsistent));
    // within 60 of any earlier sample is enough
    check!(fs_record_consistent(&b, &[z0, a]) == if fs_distance(&a, &b) <= 60 { Ok(()) } else { Err(FsReason::Inconsistent) });
    check!(fs_record_consistent(&b, &[]).is_ok());
}

fn record_separation() {
    let z_undo = rec("Z", 101);
    let z_redo = rec("Z", 102);
    let n = rec("N", 103);
    let s = rec("S", 104);
    let circle = rec("circle", 105);
    let e = rec("e", 106);
    // the same shape for another action: "That is the shape for Undo. Use moves it here."
    check!(fs_record_separation(&z_redo, &[n, z_undo]) == FsSeparation::SameAs(1));
    // an e is near a circle as a cloud (a loop with a bar): within the circle's own threshold
    let sep = fs_record_separation(&e, &[n, s, circle]);
    eprintln!("  e against circle: {sep:?}, D {} thr {}", fs_gesture_distance(&circle, &e.samples[0]), circle.thr);
    check!(sep == FsSeparation::SameAs(2));
    // near enough to be confused: an alpha after an N (d over T_N, under max(T_N, T_alpha) + 20)
    let n2 = rec("N", 200);
    let alpha = rec("alpha", 204);
    let mut d = 1000;
    for i in 0..3 {
        d = d.min(fs_gesture_distance(&n2, &alpha.samples[i]));
    }
    eprintln!("  alpha against N: D {d}, T_N {}, T_alpha {}", n2.thr, alpha.thr);
    check!(fs_record_separation(&alpha, &[s, n2]) == FsSeparation::TooClose(1));
    check!(d > n2.thr && d < n2.thr.max(alpha.thr) + FS_SEPARATE_PAD);
    check!((d, n2.thr, alpha.thr) == ALPHA_N);
    // clear
    check!(fs_record_separation(&n, &[s, circle, z_undo]) == FsSeparation::Clear);
    check!(fs_record_separation(&n, &[]) == FsSeparation::Clear);
    // room for 32 shapes
    check!(fs_record_room(31).is_ok());
    check!(fs_record_room(32) == Err(FsReason::Full));
}

// ---- the gestures.ini value (section 8) ----

/// The design's own example: a real Z from the prototype, 46 raw frames resampled to 32.
const DESIGN_Z: &str = "1607180331034c00660581079b0ab609d00cea04ec17d829cb3fba53a66592777c86689855ab44bf34d41ee312f62cf447f362f57cf995ffb0fdc7fae0fbedf7";

fn codec() {
    let g = rec("Z", 5);
    let mut v = String::new();
    fs_encode(&g.samples[..3], &mut v);
    check!(v.len() == 3 + 3 * 129);
    check!(v.starts_with("1,3,"));
    let mut out = [FsSample::default(); 3];
    check!(fs_decode(&v, &mut out) == Ok(3));
    check!(out == g.samples);
    // a decoded shape is the very shape: it matches with the same score
    let mut g2 = FsGesture::default();
    fs_prepare(&mut g2, &out);
    check!(g2.spread == g.spread && g2.thr == g.thr && g2.ratio == g.ratio);
    // case
    check!(fs_decode(&v.to_ascii_uppercase(), &mut out) == Ok(3));
    check!(out == g.samples);
    // two samples
    fs_encode(&g.samples[..2], &mut v);
    check!(fs_decode(&v, &mut out) == Ok(2));
    let h0 = v.split(',').nth(2).unwrap().to_string();
    // malformed: count, hex, length, extra or missing fields, a degenerate sample
    for bad in [
        format!("1,1,{h0}"),
        format!("1,4,{h0},{h0},{h0},{h0}"),
        format!("1,2,{h0}"),
        format!("1,2,{h0},{h0},{h0}"),
        format!("1,2,{h0},{}", &h0[..127]),
        format!("1,2,{h0},{}g", &h0[..127]),
        format!("1,2,{h0},{}", "80".repeat(64)),
        format!("1,2,{h0}, {}", &h0[1..]),
        format!("0,2,{h0},{h0}"),
        format!("x,2,{h0},{h0}"),
        String::new(),
        "1".into(),
    ] {
        check!(fs_decode(&bad, &mut out) == Err(FsCodec::Malformed));
    }
    // another format: carried
    check!(fs_decode("2,whatever,the,newer,one,writes", &mut out) == Err(FsCodec::Newer));
    check!(fs_decode("17", &mut out) == Err(FsCodec::Newer));
    // the design's example decodes and is a Z to our Z gesture
    let d = format!("1,2,{DESIGN_Z},{DESIGN_Z}");
    check!(fs_decode(&d, &mut out) == Ok(2));
    check!(fs_gesture_distance(&g, &out[0]) == 30);
    check!(fs_match(&out[0], &[g], &mut none).fired == Some(0));
}

// ---- the corpus (10.2), reduced to what a debug test run affords ----

fn corpus() {
    // Record rich shapes through the whole pipeline; keep those the bank and separation accept.
    let mut kept: Vec<(&str, FsGesture)> = Vec::new();
    for (k, name) in ["N", "Z", "S", "circle", "alpha", "two", "M", "e", "triangle", "W", "spiral", "square"].iter().enumerate() {
        let g = rec(name, 200 + k as u32);
        let others: Vec<FsGesture> = kept.iter().map(|k| k.1).collect();
        let sep = fs_record_separation(&g, &others);
        let bank = fs_record_bank(&g, &mut none);
        eprintln!("  record {name}: thr {} sep {sep:?} bank {bank:?}", g.thr);
        if sep == FsSeparation::Clear && bank.is_ok() {
            kept.push((name, g));
        }
    }
    let names: Vec<&str> = kept.iter().map(|k| k.0).collect();
    eprintln!("  kept {names:?}");
    check!(names == CORPUS_KEPT);
    let gs: Vec<FsGesture> = kept.iter().map(|k| k.1).collect();
    // Use-like and recording-like draws of each kept shape.
    let mut r = Rng::new(300);
    let (mut tries, mut ok_use, mut ok_rec, mut wrong) = (0, 0, 0, 0);
    for (i, name) in names.iter().enumerate() {
        for k in 0..20u32 {
            for (regime, ok) in [(0, &mut ok_use), (1, &mut ok_rec)] {
                let dv = if regime == 0 { vary_use(&mut r, k) } else { vary_record(&mut r, k) };
                let mut s = FsSample::default();
                if regime == 0 {
                    tries += 1;
                }
                if capture(&shape_frames(name, &dv), 1000, FsMode::Runtime, &mut s) != FsLift::Shape {
                    continue;
                }
                match fs_match(&s, &gs, &mut none).fired {
                    Some(j) if j == i => *ok += 1,
                    Some(_) => wrong += 1,
                    None => {}
                }
            }
        }
    }
    eprintln!("  use: {ok_use}/{tries} fired, recording: {ok_rec}/{tries}, wrong {wrong}");
    check!(wrong == 0);
    check!((ok_use, ok_rec, tries) == CORPUS_ACCEPT);
    // Slips: none may fire.
    let mut r = Rng::new(301);
    for kind in SLIPS {
        let (mut shapes, mut fired) = (0, 0);
        for k in 0..200u32 {
            let path = slip_path(kind, &mut r);
            let mut dv = vary_use(&mut r, 1000 + k);
            dv.rot = r.range(0, 359);
            dv.reverse = false;
            dv.ms = r.range(250, 900) as u32;
            let mut s = FsSample::default();
            if capture(&frames_of(&place(&path, false, &dv), &dv), 1000, FsMode::Runtime, &mut s) != FsLift::Shape {
                continue;
            }
            shapes += 1;
            if fs_match(&s, &gs, &mut none).fired.is_some() {
                fired += 1;
            }
        }
        eprintln!("  slips {kind}: {shapes}/200 past the gates, {fired} fired");
        // (a deliberate random scribble is the hard case: the design's prototype fired 86 in 12,000)
        check!(fired == if *kind == "scribble" { 3 } else { 0 });
    }
}
const CORPUS_KEPT: [&str; 6] = ["N", "Z", "S", "circle", "M", "triangle"];
const CORPUS_ACCEPT: (i32, i32, i32) = (116, 120, 120);

fn bench() {
    // The 32-shape ceiling, 3 samples each: the host time per match. The design's ceiling is 2.5 ms
    // in an optimized host build; this is the test profile, so the assertion is the device budget
    // (25 ms), and the time is printed.
    let mut gs = Vec::new();
    for k in 0..FS_SHAPES_MAX {
        gs.push(rec(SHAPES[k % SHAPES.len()], 400 + k as u32));
    }
    let z = ideal("Z", &Draw::default());
    let t = std::time::Instant::now();
    let runs = 20;
    for _ in 0..runs {
        std::hint::black_box(fs_match(&z, &gs, &mut none));
    }
    let per = t.elapsed() / runs;
    eprintln!("  32 x 3 match: {per:?}");
    check!(per.as_micros() < 25_000);
    let t = std::time::Instant::now();
    let g = rec("N", 5);
    let _ = std::hint::black_box(fs_record_bank(&g, &mut none));
    eprintln!("  full slip bank for N: {:?}", t.elapsed());
}

#[test]
fn test_fingershape() {
    run!(shared_gates);
    run!(mm_px);
    run!(sine_table);
    run!(capture_slop);
    run!(capture_trail_delay);
    run!(capture_lead);
    run!(capture_pause);
    run!(capture_overtime);
    run!(capture_cancels);
    run!(capture_clock_wrap);
    run!(capture_decimation);
    run!(gate_frames);
    run!(gate_box);
    run!(gate_path);
    run!(gate_duration);
    run!(gate_speed);
    run!(gate_chord);
    run!(gate_closed);
    run!(gate_tangle);
    run!(gate_swipe);
    run!(two_strokes);
    run!(normalize);
    run!(invariances);
    run!(v_and_caret);
    run!(rotation);
    run!(thresholds);
    run!(unknown_shapes);
    run!(margin);
    run!(structure_guard);
    run!(pump);
    run!(record_first);
    run!(record_bank);
    run!(record_consistent);
    run!(record_separation);
    run!(codec);
    run!(corpus);
    run!(bench);
    done();
}
