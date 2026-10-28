// test_shape: the smart-shape recognizer (src/shape.rs) against synthetic positives and negatives
// (tests/shape_corpus) and the owner's own handwriting (assets/eraser-sample.qpg, the negative
// corpus the plan names: every stroke there must give no shape at written size, and at twice the
// size only lines). The rates of the noisy positives and the false snaps are printed for the
// record; the plan's targets are at least 94% per kind and none.
#![allow(clippy::needless_range_loop)]
mod shape_corpus;
mod testlib;
use grimoire::prelude::*;
use shape_corpus::*;
use testlib::*;

/// The corpus's seed; SHAPE_SEED=n in the environment draws another corpus (to check the rates are
/// not one lucky draw).
fn seed(default: u64) -> u64 {
    std::env::var("SHAPE_SEED").ok().and_then(|s| s.parse::<u64>().ok()).map_or(default, |s| s ^ default)
}

fn prm() -> ShapeParams {
    shape_params(DPI, 1.0, PAPER_BLANK, 1860)
}

fn rec(pts: &[PenSample]) -> ShapeResult {
    shape_recognize(pts, &prm())
}

/// A clean stroke along the polyline (mm), one sample every 0.25 mm, 3 ms apart.
fn clean(path: &[Pt]) -> Vec<PenSample> {
    let mut rng = Rng::new(1);
    pen(path, CLEAN, &mut rng)
}

fn mm_of(px_: f32) -> f64 {
    px_ as f64 * 25.4 / DPI as f64
}

fn closes_exactly(r: &ShapeResult) -> bool {
    let (a, b) = (r.pts[0], r.pts[r.pts.len() - 1]);
    a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
}

fn area2(v: &[PenSample]) -> f64 {
    let mut s = 0.0;
    for k in 0..v.len() {
        let (a, b) = (v[k], v[(k + 1) % v.len()]);
        s += a.x as f64 * b.y as f64 - a.y as f64 * b.x as f64;
    }
    s
}

fn d_px(a: &PenSample, b: &PenSample) -> f64 {
    (((a.x - b.x) as f64).powi(2) + ((a.y - b.y) as f64).powi(2)).sqrt()
}

// ---- positives ----

fn clean_positives_snap_to_their_kind() {
    let mut rng = Rng::new(7);
    for k in POS_ALL {
        for i in 0..40 {
            let s = positive(k, &mut rng, false);
            let r = rec(&s);
            if r.kind != expected(k) {
                eprintln!("  clean {k:?} #{i}: {:?} {:?} {:?}", r.kind, r.reason, r.features);
            }
            check!(r.kind == expected(k));
        }
    }
}

fn noisy_positives_reach_the_rate() {
    let mut rng = Rng::new(seed(20261027));
    for k in POS_ALL {
        let mut ok = 0;
        let n = 200;
        let mut wrong: Vec<String> = Vec::new();
        for _ in 0..n {
            let s = positive(k, &mut rng, true);
            let r = rec(&s);
            if r.kind == expected(k) {
                ok += 1;
            } else if wrong.len() < 6 {
                wrong.push(format!("d{:.0} {:?}/{:?} c{} w{:.2} t{:.2} g{:.2} rms{:.3} max{:.3} ax{:.2}", r.features.diag_mm, r.kind, r.reason, r.features.corners, r.features.winding, r.features.turning, r.features.gap, r.features.rms, r.features.max_err, r.features.axis_ratio));
            }
        }
        let rate = ok as f64 / n as f64 * 100.0;
        eprintln!("  noisy {k:?}: {ok}/{n} = {rate:.1}%");
        for w in &wrong {
            eprintln!("    {w}");
        }
        check!(rate >= 94.0);
    }
}

// ---- negatives ----

fn writing_and_scribbles_never_snap() {
    let mut rng = Rng::new(seed(99));
    let mut total = 0;
    let mut snapped = 0;
    let reasons = std::cell::RefCell::new([0i32; 10]);
    let tally = |name: &str, s: &[PenSample], total: &mut i32, snapped: &mut i32| {
        let r = rec(s);
        reasons.borrow_mut()[r.reason as usize] += 1;
        *total += 1;
        if r.kind != ShapeKind::None {
            *snapped += 1;
            eprintln!("  false snap: {name} -> {:?} {:?}", r.kind, r.features);
        }
    };
    for _ in 0..300 {
        let s = cursive_word(&mut rng);
        tally("cursive", &s, &mut total, &mut snapped);
    }
    for _ in 0..300 {
        let s = letter(&mut rng);
        tally("letter", &s, &mut total, &mut snapped);
    }
    for d in 0..10 {
        for _ in 0..40 {
            let s = digit(d, &mut rng);
            tally("digit", &s, &mut total, &mut snapped);
        }
    }
    for _ in 0..300 {
        let s = cjk_stroke(&mut rng);
        tally("cjk", &s, &mut total, &mut snapped);
    }
    for _ in 0..300 {
        let s = scribble(&mut rng);
        tally("scribble", &s, &mut total, &mut snapped);
    }
    for _ in 0..300 {
        let s = small_shape(&mut rng);
        tally("small", &s, &mut total, &mut snapped);
    }
    eprintln!("  negatives: {snapped} false snaps of {total}; refused as (by ShapeReason value) {:?}", reasons.borrow());
    check!(snapped == 0);
}

fn the_owners_strokes_never_snap_at_written_size() {
    let mut out = None;
    check!(page_deserialize(GRIMOIRE_ERASER_SAMPLE, &mut out) == 0);
    let Some(page) = out else {
        return;
    };
    check!(page.strokes.len() == 32);
    let mut at1 = 0;
    let (mut lines2, mut others2) = (0, 0);
    for s in &page.strokes {
        let r = rec(&s.pts);
        if r.kind != ShapeKind::None {
            at1 += 1;
        }
        // twice the size: zoom 2 (screen mm = page mm x zoom), the gates see the stroke doubled
        let r2 = shape_recognize(&s.pts, &shape_params(DPI, 2.0, PAPER_BLANK, page.w));
        match r2.kind {
            ShapeKind::None => {}
            ShapeKind::Line => lines2 += 1,
            k => {
                others2 += 1;
                eprintln!("  at 2x: {k:?} {:?}", r2.features);
            }
        }
    }
    eprintln!("  owner's strokes: {at1} snaps at written size; at 2x {lines2} lines, {others2} other shapes");
    check!(at1 == 0);
    check!(others2 == 0);
}

// ---- exact outputs ----

fn exact_point_counts() {
    // A level 41 mm line: ceil(41 / 2) = 21 pieces, 22 points.
    let r = rec(&clean(&[(20.0, 50.0), (61.0, 50.0)]));
    check!(r.kind == ShapeKind::Line);
    check!(r.pts.len() == 22);
    check!(r.pts[0].y == r.pts[21].y);
    // A level 31 mm square started at its corner: 16 pieces an edge, back to the start: 65.
    let r = rec(&clean(&trace_polygon(&[(20.0, 20.0), (51.0, 20.0), (51.0, 51.0), (20.0, 51.0)], 0.0, true, 0.0)));
    check!(r.kind == ShapeKind::Square);
    check!(r.pts.len() == 65);
    if r.kind == ShapeKind::Square {
        check_near!(mm_of(r.pts[0].x), 20.0, 0.3);
        check_near!(mm_of(r.pts[0].y), 20.0, 0.3);
        check_near!(d_px(&r.pts[0], &r.pts[16]), px(31.0), px(0.3));
        check!(r.pts[16].y == r.pts[0].y); // level: on the page's axes
    }
    // A 20 mm radius circle: N = ceil(2 pi / acos(1 - 0.25 / 236.22)) = 137 vertices, then the first.
    let c: Vec<Pt> = trace_ellipse(20.0, 20.0, 0.0, true, 0.0).iter().map(|&(x, y)| (x + 60.0, y + 60.0)).collect();
    let r = rec(&clean(&c));
    check!(r.kind == ShapeKind::Circle);
    check!(r.pts.len() == 138);
}

fn the_vertex_count_formula() {
    let f = |r: f64| -> i32 {
        let n = (2.0 * std::f64::consts::PI / (1.0 - 0.25 / r).clamp(-1.0, 1.0).acos()).ceil();
        (n as i32).clamp(24, 720)
    };
    check!(shape_ellipse_vertices(1.0) == 24);
    check!(shape_ellipse_vertices(10.0) == 29);
    check!(shape_ellipse_vertices(100.0) == 89);
    check!(shape_ellipse_vertices(1000.0) == 281);
    check!(shape_ellipse_vertices(5000.0) == 629);
    check!(shape_ellipse_vertices(10000.0) == 720);
    check!(shape_ellipse_vertices(0.0) == 24);
    check!(shape_ellipse_vertices(-3.0) == 24);
    check!(shape_ellipse_vertices(f32::NAN) == 24);
    check!(shape_ellipse_vertices(f32::INFINITY) == 720);
    for r in [2.5f64, 7.0, 33.3, 77.0, 150.0, 236.22, 400.0, 999.0, 3000.0, 6000.0] {
        check!(shape_ellipse_vertices(r as f32) == f(r as f32 as f64));
    }
}

fn the_gates_scale_with_the_zoom() {
    // 8 page mm of line: too small at zoom 1, a 16 mm line at zoom 2 (the hand moved 16 mm).
    let s = clean(&[(30.0, 30.0), (38.0, 30.0)]);
    check!(rec(&s).reason == ShapeReason::TooSmall);
    let r2 = shape_recognize(&s, &shape_params(DPI, 2.0, PAPER_BLANK, 1860));
    check!(r2.kind == ShapeKind::Line);
    // 11 page mm (screen mm = page mm x zoom): a line too short at zoom 1, nothing at zoom 0.9
    // (9.9 mm), a line at 1.2 (13.2 mm).
    let s = clean(&[(30.0, 30.0), (41.0, 30.0)]);
    check!(rec(&s).reason == ShapeReason::LineTooShort);
    check!(shape_recognize(&s, &shape_params(DPI, 0.9, PAPER_BLANK, 1860)).reason == ShapeReason::TooSmall);
    check!(shape_recognize(&s, &shape_params(DPI, 1.2, PAPER_BLANK, 1860)).kind == ShapeKind::Line);
    // A 24 page mm line at zoom 0.4 is 9.6 mm on the glass: too small.
    let s = clean(&[(30.0, 30.0), (54.0, 30.0)]);
    check!(shape_recognize(&s, &shape_params(DPI, 0.4, PAPER_BLANK, 1860)).reason == ShapeReason::TooSmall);
    // An 8 mm circle (its box's diagonal 11.3 mm): too narrow at zoom 1, a circle at zoom 2.
    let c: Vec<Pt> = trace_ellipse(4.0, 4.0, 0.0, true, 0.0).iter().map(|&(x, y)| (x + 60.0, y + 60.0)).collect();
    let s = clean(&c);
    check!(rec(&s).reason == ShapeReason::TooNarrow);
    check!(shape_recognize(&s, &shape_params(DPI, 2.0, PAPER_BLANK, 1860)).kind == ShapeKind::Circle);
    // The snapped points are page geometry: the same 16 page mm circle at zoom 1 and 2 gives the
    // same vertex count (r in page px).
    let c: Vec<Pt> = trace_ellipse(8.0, 8.0, 0.0, true, 0.0).iter().map(|&(x, y)| (x + 60.0, y + 60.0)).collect();
    let s = clean(&c);
    let (a, b) = (rec(&s), shape_recognize(&s, &shape_params(DPI, 2.0, PAPER_BLANK, 1860)));
    check!(a.kind == ShapeKind::Circle && b.kind == ShapeKind::Circle && a.pts.len() == b.pts.len());
}

fn the_grid_snaps_within_a_millimetre() {
    let pitch = mm_to_px(5.0, DPI) as f64;
    // Graph paper: lines at lrintf(pitch * i). Ends 0.6 mm off nodes land on them.
    let g = shape_params(DPI, 1.0, PAPER_GRID, 1860);
    let s = clean(&[(20.0 + 0.5, 30.0 - 0.3), (60.0 - 0.4, 30.0 + 0.2)]);
    let r = shape_recognize(&s, &g);
    check!(r.kind == ShapeKind::Line);
    let (a, b) = (r.pts[0], r.pts[r.pts.len() - 1]);
    let node = |i: i32| (pitch * i as f64).round_ties_even() as f32;
    check!(a.x == node(4) && a.y == node(6));
    check!(b.x == node(12) && b.y == node(6));
    // 1.5 mm off: stays off the grid (and level, by the angle snap).
    let s = clean(&[(20.0 + 1.5, 31.5), (60.0 - 1.5, 31.5)]);
    let r = shape_recognize(&s, &g);
    check!(r.kind == ShapeKind::Line);
    check!((mm_of(r.pts[0].x) - 21.5).abs() < 0.2 && (mm_of(r.pts[0].y) - 31.5).abs() < 0.2);
    // A level rectangle's edges: 0.7 mm off each.
    let v = [(20.7, 40.6), (50.6, 40.6), (50.6, 60.7), (20.7, 60.7)];
    let r = shape_recognize(&clean(&trace_polygon(&v, 0.0, true, 0.0)), &g);
    check!(r.kind == ShapeKind::Rectangle);
    let xs: Vec<f32> = r.pts.iter().map(|q| q.x).collect();
    let ys: Vec<f32> = r.pts.iter().map(|q| q.y).collect();
    let (x0, x1) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
    let (y0, y1) = (ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max));
    check!(x0 == node(4) && x1 == node(10) && y0 == node(8) && y1 == node(12));
    // The same rectangle on blank paper keeps its edges.
    let r = rec(&clean(&trace_polygon(&v, 0.0, true, 0.0)));
    check!(r.kind == ShapeKind::Rectangle);
    check!((mm_of(r.pts.iter().map(|q| q.x).fold(f32::MAX, f32::min)) - 20.7).abs() < 0.15);
    // Dot grid: the columns centred on the page, the rows from 10 mm (paper.rs dots()).
    let w = 1860;
    let d = shape_params(DPI, 1.0, PAPER_DOTS, w);
    let p32 = mm_to_px(5.0, DPI);
    let ox = ((w as f32 % p32) / 2.0 + p32 / 2.0) as f64;
    let oy = mm_to_px(10.0, DPI) as f64;
    let (nx, ny) = (ox + 6.0 * pitch, oy + 4.0 * pitch);
    let start = (mm_of(nx as f32) + 0.5, mm_of(ny as f32) - 0.4);
    let s = clean(&[start, (start.0 + 30.0, start.1 + 0.1)]);
    let r = shape_recognize(&s, &d);
    check!(r.kind == ShapeKind::Line);
    check_near!(r.pts[0].x, nx, 0.01);
    check_near!(r.pts[0].y, ny, 0.01);
    check_near!(r.pts[r.pts.len() - 1].x, ox + 12.0 * pitch, 0.01);
}

fn ruled_paper_makes_lines_level() {
    // 7 degrees: level on ruled paper, kept on blank paper (outside the 4 degree snap).
    let a = 7f64.to_radians();
    let s = clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]);
    for k in [PAPER_NARROW, PAPER_MEDIUM, PAPER_WIDE, PAPER_NARROW_NM, PAPER_MEDIUM_NM, PAPER_WIDE_NM, PAPER_CORNELL] {
        let r = shape_recognize(&s, &shape_params(DPI, 1.0, k, 1860));
        check!(r.kind == ShapeKind::Line && r.pts[0].y == r.pts[r.pts.len() - 1].y);
    }
    let r = rec(&s);
    check!(r.kind == ShapeKind::Line);
    let (p0, p1) = (r.pts[0], r.pts[r.pts.len() - 1]);
    let ang = ((p1.y - p0.y) as f64).atan2((p1.x - p0.x) as f64).to_degrees();
    check_near!(ang, 7.0, 0.3);
    // Music paper is not ruled: kept too.
    let r = shape_recognize(&s, &shape_params(DPI, 1.0, PAPER_MUSIC, 1860));
    check!(r.kind == ShapeKind::Line && r.pts[0].y != r.pts[r.pts.len() - 1].y);
    // 3 degrees snaps level anywhere; 44 degrees snaps to 45.
    let a = 3f64.to_radians();
    let r = rec(&clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]));
    check!(r.kind == ShapeKind::Line && r.pts[0].y == r.pts[r.pts.len() - 1].y);
    let a = 44f64.to_radians();
    let r = rec(&clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]));
    let (p0, p1) = (r.pts[0], r.pts[r.pts.len() - 1]);
    check!(r.kind == ShapeKind::Line);
    check_near!(((p1.y - p0.y) as f64).atan2((p1.x - p0.x) as f64).to_degrees(), 45.0, 0.01);
    // A ruled page's 15 degree line stays slanted.
    let a = 15f64.to_radians();
    let r = shape_recognize(&clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]), &shape_params(DPI, 1.0, PAPER_MEDIUM, 1860));
    check!(r.kind == ShapeKind::Line && r.pts[0].y != r.pts[r.pts.len() - 1].y);
}

fn closed_shapes_close_exactly_and_keep_start_and_direction() {
    let mut rng = Rng::new(5);
    for k in [Pos::Rectangle, Pos::Square, Pos::Quad, Pos::Triangle, Pos::Ellipse, Pos::Circle] {
        for _ in 0..30 {
            let s = positive(k, &mut rng, true);
            let r = rec(&s);
            if r.kind == ShapeKind::None {
                continue;
            }
            check!(closes_exactly(&r));
            // the start: near the stroke's first point (on the outline, or its vertex 2 mm away)
            let d = d_px(&r.pts[0], &s[0]);
            if d > px(3.0) {
                eprintln!("  start {k:?} {:.2} mm away", mm_of(d as f32));
            }
            check!(d <= px(3.0));
            // the direction: the same turn as the stroke's
            check!(area2(&r.pts) * area2(&s) > 0.0);
        }
    }
    // A line drawn right to left runs right to left.
    let r = rec(&clean(&[(60.0, 40.0), (20.0, 40.5)]));
    check!(r.kind == ShapeKind::Line && r.pts[0].x > r.pts[r.pts.len() - 1].x);
    check!(d_px(&r.pts[0], &clean(&[(60.0, 40.0)])[0]) < px(0.5));
}

fn the_samples_carry_pressure_tilt_and_time() {
    // pressure: 0.1 in the first and last tenth, 0.6 between (the median of the middle 80%)
    let mut s = clean(&[(20.0, 40.0), (60.0, 40.0)]);
    let n = s.len();
    for (i, q) in s.iter_mut().enumerate() {
        let f = i as f32 / (n - 1) as f32;
        q.pressure = if !(0.08..=0.92).contains(&f) { 0.1 } else { 0.6 };
        q.tilt_x = if i % 3 == 0 { 50.0 } else { 20.0 };
        q.tilt_y = -10.0;
        q.t_ms = 4_294_967_000u32.wrapping_add(4 * i as u32); // wraps past 2^32 mid-stroke
    }
    let r = rec(&s);
    check!(r.kind == ShapeKind::Line);
    check!(r.pts.iter().all(|q| q.pressure == 0.6 && q.tilt_x == 20.0 && q.tilt_y == -10.0));
    check!(r.pts[0].t_ms == s[0].t_ms);
    check!(r.pts[r.pts.len() - 1].t_ms == s[n - 1].t_ms);
    let mut ok = true;
    for w in r.pts.windows(2) {
        ok &= w[1].t_ms.wrapping_sub(w[0].t_ms) < 1000;
    }
    check!(ok);
    // the points are evenly spaced, so point k is k / (len - 1) of the way in time too
    let span = s[n - 1].t_ms.wrapping_sub(s[0].t_ms) as f64;
    let m = r.pts.len();
    let mut ok = true;
    for k in 0..m {
        let want = span * k as f64 / (m - 1) as f64;
        ok &= (r.pts[k].t_ms.wrapping_sub(s[0].t_ms) as f64 - want).abs() <= 4.5;
    }
    check!(ok);
}

fn the_snapped_stroke_keeps_tool_params_seed_and_ext() {
    let pts = clean(&trace_polygon(&[(20.0, 20.0), (50.0, 20.0), (50.0, 40.0), (20.0, 40.0)], 0.3, false, 0.04));
    let mut tp = ToolParams::default();
    tool_params_default(TOOL_FOUNTAIN, &mut tp);
    let mut raw = stroke_new(TOOL_FOUNTAIN, &tp, 0xC0FFEE);
    raw.ext = [3, 1, 0x12];
    for q in &pts {
        stroke_append(&mut raw, q, DPI);
    }
    let r = rec(&raw.pts);
    check!(r.kind == ShapeKind::Rectangle);
    let Some(s) = shape_stroke(&raw, &r, DPI) else {
        check!(false);
        return;
    };
    check!(s.tool == raw.tool && s.seed == raw.seed && s.ext == raw.ext);
    check!(s.params == raw.params);
    check!(s.pts == r.pts);
    check!(!rect_is_empty(s.bbox));
    check!(s.pts.iter().all(|q| rect_contains(s.bbox, q.x as i32, q.y as i32)));
    // no shape, no stroke
    let none = rec(&clean(&[(20.0, 20.0), (22.0, 21.0)]));
    check!(none.kind == ShapeKind::None && shape_stroke(&raw, &none, DPI).is_none());
}

fn the_burst_gate_narrows_the_tolerances() {
    // A 40 mm line bowed 1.5 mm: inside the offset allowance (1.6 + 0.5 mm), outside 0.7x of it.
    let path: Vec<Pt> = (0..=40).map(|i| (20.0 + i as f64, 40.0 + 1.5 * (std::f64::consts::PI * i as f64 / 40.0).sin())).collect();
    let s = clean(&path);
    check!(rec(&s).kind == ShapeKind::Line);
    let mut b = prm();
    b.burst = true;
    let r = shape_recognize(&s, &b);
    check!(r.kind == ShapeKind::None && r.reason != ShapeReason::Snapped);
    // A clean square passes the narrower gates too.
    let sq = clean(&trace_polygon(&[(20.0, 20.0), (50.0, 20.0), (50.0, 50.0), (20.0, 50.0)], 0.1, true, 0.02));
    check!(shape_recognize(&sq, &b).kind == ShapeKind::Square);
    // A circle 8% out of round (its RMS error about 5.7% of the radius): a circle-ish ellipse
    // alone, nothing in a burst.
    let blob: Vec<Pt> = (0..=400)
        .map(|i| {
            let t = 2.0 * std::f64::consts::PI * i as f64 / 400.0;
            let r = 20.0 * (1.0 + 0.08 * (3.0 * t).sin());
            (60.0 + r * t.cos(), 60.0 + r * t.sin())
        })
        .collect();
    let s = clean(&blob);
    let r1 = rec(&s);
    let r2 = shape_recognize(&s, &b);
    eprintln!("  wobbly circle: {:?} rms {:.3}; in a burst {:?} {:?}", r1.kind, r1.features.rms, r2.kind, r2.reason);
    check!(r1.kind == ShapeKind::Circle);
    check!(r2.kind == ShapeKind::None);
    // the gate itself
    let lift = PenSample { x: px(50.0) as f32, y: px(50.0) as f32, t_ms: 10_000, ..Default::default() };
    let at = |dx: f64, dt: u32| PenSample { x: px(50.0 + dx) as f32, y: px(50.0) as f32, t_ms: 10_000u32.wrapping_add(dt), ..Default::default() };
    check!(shape_in_burst(Some(&lift), &at(5.0, 500), DPI, 1.0));
    check!(shape_in_burst(Some(&lift), &at(7.9, 600), DPI, 1.0));
    check!(!shape_in_burst(Some(&lift), &at(5.0, 601), DPI, 1.0));
    check!(!shape_in_burst(Some(&lift), &at(8.5, 100), DPI, 1.0));
    // screen mm: 5 page mm is 10 on the glass at zoom 2
    check!(!shape_in_burst(Some(&lift), &at(5.0, 100), DPI, 2.0));
    check!(!shape_in_burst(None, &at(1.0, 100), DPI, 1.0));
    // across the u32 wrap
    let late = PenSample { t_ms: u32::MAX - 100, ..lift };
    let next = PenSample { t_ms: 300, ..lift };
    check!(shape_in_burst(Some(&late), &next, DPI, 1.0));
    check!(SHAPE_BURST_TOL == 0.7 && SHAPE_BURST_HOLD == 1.5 && SHAPE_BURST_MS == 600 && SHAPE_BURST_MM == 8.0);
}

fn the_kinds_and_their_snaps() {
    // a level rectangle 3 degrees off stays exactly level; a 20 degree one keeps its turn
    let turned = |deg: f64, w: f64, h: f64| -> Vec<PenSample> {
        let v = [(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)];
        clean(&place(&trace_polygon(&v, 0.1, true, 0.03), deg.to_radians(), 60.0, 60.0))
    };
    let on_axes = |r: &ShapeResult| -> bool {
        let x0 = r.pts.iter().fold(f32::MAX, |m, q| m.min(q.x));
        let x1 = r.pts.iter().fold(f32::MIN, |m, q| m.max(q.x));
        let y0 = r.pts.iter().fold(f32::MAX, |m, q| m.min(q.y));
        let y1 = r.pts.iter().fold(f32::MIN, |m, q| m.max(q.y));
        r.pts.iter().all(|q| q.x == x0 || q.x == x1 || q.y == y0 || q.y == y1)
    };
    let r = rec(&turned(3.0, 40.0, 20.0));
    check!(r.kind == ShapeKind::Rectangle && on_axes(&r));
    let r = rec(&turned(20.0, 40.0, 20.0));
    check!(r.kind == ShapeKind::Rectangle && !on_axes(&r));
    if r.kind == ShapeKind::Rectangle {
        // its corners square, its turn kept
        let v = corners_of(&r.pts);
        check!(v.len() == 4);
        if v.len() == 4 {
            let (ux, uy) = ((v[1].x - v[0].x) as f64, (v[1].y - v[0].y) as f64);
            let (wx, wy) = ((v[2].x - v[1].x) as f64, (v[2].y - v[1].y) as f64);
            check!((ux * wx + uy * wy).abs() / ((ux * ux + uy * uy).sqrt() * (wx * wx + wy * wy).sqrt()) < 1e-4);
            let ang = uy.atan2(ux).to_degrees().rem_euclid(90.0);
            check_near!(ang, 20.0, 1.0);
        }
    }
    // a square 9% off is a square, 15% off a rectangle
    check!(rec(&turned(0.0, 30.0, 27.5)).kind == ShapeKind::Square);
    check!(rec(&turned(0.0, 30.0, 25.0)).kind == ShapeKind::Rectangle);
    // a square on its point is still a square; a diamond with 53 and 127 degree corners is a quad
    check!(rec(&turned(45.0, 30.0, 30.0)).kind == ShapeKind::Square);
    let dia = clean(&place(&trace_polygon(&[(-20.0, 0.0), (0.0, -10.0), (20.0, 0.0), (0.0, 10.0)], 0.0, true, 0.0), 0.0, 60.0, 60.0));
    check!(rec(&dia).kind == ShapeKind::Quad);
    // an equilateral triangle 4 degrees off is made equilateral
    let tri = clean(&trace_polygon(&[(30.0, 60.0), (60.0, 60.0), (46.5, 34.0)], 0.0, true, 0.0));
    let r = rec(&tri);
    check!(r.kind == ShapeKind::Triangle);
    if r.kind == ShapeKind::Triangle {
        // its sides: every vertex the same distance from the next
        let v: Vec<PenSample> = corners_of(&r.pts);
        check!(v.len() == 3);
        if v.len() == 3 {
            let (a, b, c) = (d_px(&v[0], &v[1]), d_px(&v[1], &v[2]), d_px(&v[2], &v[0]));
            check!((a - b).abs() < 0.05 && (b - c).abs() < 0.05);
        }
    }
    // a right triangle 5 degrees off is made square at its corner
    let rt = clean(&trace_polygon(&[(30.0, 30.0), (70.0, 31.5), (31.0, 55.0)], 0.0, true, 0.0));
    let r = rec(&rt);
    check!(r.kind == ShapeKind::Triangle);
    if r.kind == ShapeKind::Triangle {
        let v = corners_of(&r.pts);
        check!(v.len() == 3);
        if v.len() == 3 {
            let mut right = false;
            for k in 0..3 {
                let (a, b, c) = (v[k], v[(k + 1) % 3], v[(k + 2) % 3]);
                let (ux, uy, wx, wy) = ((b.x - a.x) as f64, (b.y - a.y) as f64, (c.x - a.x) as f64, (c.y - a.y) as f64);
                let cos = (ux * wx + uy * wy) / ((ux * ux + uy * uy).sqrt() * (wx * wx + wy * wy).sqrt());
                right |= cos.abs() < 1e-4;
            }
            check!(right);
        }
    }
    // an ellipse 4 degrees off its axes is put on them; 15 degrees off it is not
    let el = |deg: f64| clean(&place(&trace_ellipse(30.0, 15.0, 0.4, true, 0.0), deg.to_radians(), 70.0, 70.0));
    let r = rec(&el(4.0));
    check!(r.kind == ShapeKind::Ellipse);
    let top = r.pts.iter().cloned().fold(f32::MAX, |m, q| m.min(q.y));
    let ext_x = r.pts.iter().cloned().fold(f32::MIN, |m, q| m.max(q.x)) - r.pts.iter().cloned().fold(f32::MAX, |m, q| m.min(q.x));
    check_near!(mm_of(ext_x), 60.0, 0.5);
    check_near!(mm_of(top), 55.0, 0.5);
    let r = rec(&el(15.0));
    check!(r.kind == ShapeKind::Ellipse);
    // a nearly round ellipse (0.9) is a circle
    check!(rec(&clean(&place(&trace_ellipse(20.0, 18.0, 0.0, false, 0.0), 0.3, 60.0, 60.0))).kind == ShapeKind::Circle);
}

/// The polygon's vertices among the snapped points: where the direction turns.
fn corners_of(p: &[PenSample]) -> Vec<PenSample> {
    let n = p.len() - 1; // closed: the last is the first
    let mut out = Vec::new();
    for k in 0..n {
        let (a, b, c) = (p[(k + n - 1) % n], p[k], p[(k + 1) % n]);
        let (ux, uy, wx, wy) = ((b.x - a.x) as f64, (b.y - a.y) as f64, (c.x - b.x) as f64, (c.y - b.y) as f64);
        let cr = ux * wy - uy * wx;
        if cr.abs() / ((ux * ux + uy * uy).sqrt() * (wx * wx + wy * wy).sqrt()) > 0.05 {
            out.push(b);
        }
    }
    out
}

fn degenerate_input_is_refused() {
    check!(rec(&[]).reason == ShapeReason::Degenerate);
    let one = clean(&[(10.0, 10.0)]);
    check!(rec(&one).reason == ShapeReason::Degenerate);
    let mut s = clean(&[(20.0, 20.0), (60.0, 20.0)]);
    s[3].x = f32::NAN;
    check!(rec(&s).reason == ShapeReason::Degenerate);
    let s = clean(&[(20.0, 20.0), (60.0, 20.0)]);
    check!(shape_recognize(&s, &shape_params(0.0, 1.0, PAPER_BLANK, 0)).reason == ShapeReason::Degenerate);
    check!(shape_recognize(&s, &shape_params(DPI, 0.0, PAPER_BLANK, 0)).reason == ShapeReason::Degenerate);
    // the same point a thousand times
    let s = vec![PenSample { x: 500.0, y: 500.0, ..Default::default() }; 1000];
    check!(rec(&s).reason == ShapeReason::Degenerate);
    // an open arc is not closed; a spiral of two turns does not wind once; a long scribble
    let arc: Vec<Pt> = (0..=100).map(|i| {
        let t = std::f64::consts::PI * 1.2 * i as f64 / 100.0;
        (60.0 + 20.0 * t.cos(), 60.0 + 20.0 * t.sin())
    }).collect();
    check!(rec(&clean(&arc)).reason == ShapeReason::NotClosed);
    let spiral: Vec<Pt> = (0..=400).map(|i| {
        let t = 4.0 * std::f64::consts::PI * i as f64 / 400.0;
        let r = 20.0 - 0.3 * t;
        (60.0 + r * t.cos(), 60.0 + r * t.sin())
    }).collect();
    let r = rec(&clean(&spiral));
    check!(r.kind == ShapeKind::None);
}

/// Every kind at 12, 20, 40 and 80 mm, turned 0, 30, 45 and 77 degrees, both ways round, clean.
fn every_kind_at_every_size_turn_and_direction() {
    let mut bad = 0;
    for size in [12.0, 20.0, 40.0, 80.0] {
        for deg in [0.0f64, 30.0, 45.0, 77.0] {
            for fwd in [true, false] {
                let rot = deg.to_radians();
                let h = size / 2.0;
                let cases: Vec<(ShapeKind, Vec<Pt>)> = vec![
                    (ShapeKind::Line, place(&[(-h - 1.0, 0.0), (h + 1.0, 0.0)], rot, 80.0, 80.0)),
                    (ShapeKind::Rectangle, place(&trace_polygon(&[(-h, -h * 0.55), (h, -h * 0.55), (h, h * 0.55), (-h, h * 0.55)], 0.3, fwd, 0.05), rot, 80.0, 80.0)),
                    (ShapeKind::Square, place(&trace_polygon(&[(-h, -h), (h, -h), (h, h), (-h, h)], 0.6, fwd, 0.05), rot, 80.0, 80.0)),
                    (ShapeKind::Quad, place(&trace_polygon(&[(-h - 2.0, 0.0), (0.0, -h * 0.6), (h + 2.0, 0.0), (0.0, h * 0.6)], 0.1, fwd, 0.05), rot, 80.0, 80.0)),
                    (ShapeKind::Triangle, place(&trace_polygon(&[(-h - 2.0, h), (h + 2.0, h), (0.0, -h)], 0.5, fwd, 0.05), rot, 80.0, 80.0)),
                    (ShapeKind::Ellipse, place(&trace_ellipse(h + 2.0, (h + 2.0) * 0.5, 1.0, fwd, 0.05), rot, 80.0, 80.0)),
                    (ShapeKind::Circle, place(&trace_ellipse(h, h, 2.0, fwd, 0.05), rot, 80.0, 80.0)),
                ];
                for (want, path) in cases {
                    let r = rec(&clean(&path));
                    if r.kind != want {
                        bad += 1;
                        eprintln!("  {size} mm {deg} deg {fwd}: {want:?} -> {:?} {:?}", r.kind, r.reason);
                    }
                }
            }
        }
    }
    check!(bad == 0);
}

/// The shape-log's reasons, each from a stroke that fails at its gate.
fn each_gate_says_why() {
    let circle = |r: f64, turns: f64| -> Vec<Pt> {
        (0..=(400.0 * turns) as usize).map(|i| {
            let t = 2.0 * std::f64::consts::PI * i as f64 / 400.0;
            (60.0 + r * t.cos(), 60.0 + r * t.sin())
        }).collect()
    };
    check!(rec(&clean(&[(20.0, 20.0), (25.0, 23.0)])).reason == ShapeReason::TooSmall);
    check!(rec(&clean(&[(20.0, 20.0), (31.0, 20.0)])).reason == ShapeReason::LineTooShort);
    check!(rec(&clean(&circle(20.0, 0.6))).reason == ShapeReason::NotClosed);
    // a long, thin loop: 30 mm by 3 mm
    check!(rec(&clean(&trace_ellipse(15.0, 1.5, 0.0, true, 0.0).iter().map(|&(x, y)| (x + 60.0, y + 60.0)).collect::<Vec<_>>())).reason == ShapeReason::TooThin);
    // twice round
    check!(rec(&clean(&circle(20.0, 2.0))).reason == ShapeReason::Winding);
    // a figure eight 30 mm tall: it crosses itself at its centroid, so around the centroid it
    // seems to wind once (each lobe half a turn), and the turning is within budget; the fits
    // refuse it
    let eight: Vec<Pt> = (0..=400).map(|i| {
        let t = 2.0 * std::f64::consts::PI * i as f64 / 400.0;
        (60.0 + 10.0 * (2.0 * t).sin(), 60.0 + 15.0 * t.sin())
    }).collect();
    let r8 = rec(&clean(&eight));
    check!(r8.kind == ShapeKind::None && r8.reason == ShapeReason::NoFit);
    // once round with a zigzag on the way: it turns far more than once
    let zig: Vec<Pt> = (0..=400).map(|i| {
        let t = 2.0 * std::f64::consts::PI * i as f64 / 400.0;
        let r = 20.0 + if i % 10 < 5 { 1.5 } else { -1.5 };
        (60.0 + r * t.cos(), 60.0 + r * t.sin())
    }).collect();
    check!(rec(&clean(&zig)).reason == ShapeReason::Scribble);
    // a closed blob: a circle with a bite out of it is neither a polygon nor an ellipse
    let blob: Vec<Pt> = (0..=400).map(|i| {
        let t = 2.0 * std::f64::consts::PI * i as f64 / 400.0;
        let r = 20.0 * (1.0 - 0.35 * (-((t - 3.0) * 3.0).powi(2)).exp());
        (60.0 + r * t.cos(), 60.0 + r * t.sin())
    }).collect();
    let r = rec(&clean(&blob));
    check!(r.reason == ShapeReason::NoFit && r.kind == ShapeKind::None && r.pts.is_empty());
    // a snap says Snapped
    check!(rec(&clean(&circle(20.0, 1.0))).reason == ShapeReason::Snapped);
}

#[test]
fn test_shape() {
    run!(clean_positives_snap_to_their_kind);
    run!(noisy_positives_reach_the_rate);
    run!(writing_and_scribbles_never_snap);
    run!(the_owners_strokes_never_snap_at_written_size);
    run!(exact_point_counts);
    run!(the_vertex_count_formula);
    run!(the_gates_scale_with_the_zoom);
    run!(the_grid_snaps_within_a_millimetre);
    run!(ruled_paper_makes_lines_level);
    run!(closed_shapes_close_exactly_and_keep_start_and_direction);
    run!(the_samples_carry_pressure_tilt_and_time);
    run!(the_snapped_stroke_keeps_tool_params_seed_and_ext);
    run!(the_burst_gate_narrows_the_tolerances);
    run!(the_kinds_and_their_snaps);
    run!(degenerate_input_is_refused);
    run!(every_kind_at_every_size_turn_and_direction);
    run!(each_gate_says_why);
    done();
}

