// test_shape: the smart-shape recognizer (src/shape.rs) against synthetic positives and negatives
// (tests/shape_corpus) and the owner's own handwriting (assets/eraser-sample.qpg, the negative
// corpus the plan names: every stroke there must give no shape at written size, and at twice the
// size only lines). The rates of the noisy positives (at three noise levels, each stroke ending in
// the rest the hold leaves) and the false snaps are printed for the record; the plan's targets are
// at least 94% per kind and none.
#![allow(clippy::needless_range_loop)]
mod shape_corpus;
mod testlib;
use grimoire::prelude::*;
use shape_corpus::*;
use testlib::*;
use std::f64::consts::PI;

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

/// A clean stroke along the polyline (mm), one sample every 0.25 mm.
fn clean(path: &[Pt]) -> Vec<PenSample> {
    let mut rng = Rng::new(1);
    pen(path, CLEAN, &mut rng)
}

/// The same with a hold's rest at its end: 150 frames jittering 0.05 mm, always the same.
fn clean_held(path: &[Pt]) -> Vec<PenSample> {
    with_hold(clean(path), 150, 0.05, &mut Rng::new(3))
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

/// The distance of a sample from the polyline through the snapped points.
fn d_outline(q: &PenSample, v: &[PenSample]) -> f64 {
    let mut best = f64::MAX;
    for w in v.windows(2) {
        let (ax, ay, bx, by) = (w[0].x as f64, w[0].y as f64, w[1].x as f64, w[1].y as f64);
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 { (((q.x as f64 - ax) * dx + (q.y as f64 - ay) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        best = best.min(((q.x as f64 - ax - dx * t).powi(2) + (q.y as f64 - ay - dy * t).powi(2)).sqrt());
    }
    best
}

fn bbox(r: &ShapeResult) -> (f32, f32, f32, f32) {
    let x0 = r.pts.iter().fold(f32::MAX, |m, q| m.min(q.x));
    let x1 = r.pts.iter().fold(f32::MIN, |m, q| m.max(q.x));
    let y0 = r.pts.iter().fold(f32::MAX, |m, q| m.min(q.y));
    let y1 = r.pts.iter().fold(f32::MIN, |m, q| m.max(q.y));
    (x0, y0, x1, y1)
}

/// The angle (degrees) from a snapped line's first point to its last.
fn line_deg(r: &ShapeResult) -> f64 {
    let (p0, p1) = (r.pts[0], r.pts[r.pts.len() - 1]);
    ((p1.y - p0.y) as f64).atan2((p1.x - p0.x) as f64).to_degrees()
}

// ---- positives ----

fn clean_positives_snap_to_their_kind() {
    let mut rng = Rng::new(7);
    for k in POS_ALL {
        for i in 0..40 {
            let s = positive(k, &mut rng, Noise::Clean);
            let r = rec(&s);
            if r.kind != expected(k) {
                eprintln!("  clean {k:?} #{i}: {:?} {:?} {:?}", r.kind, r.reason, r.features);
            }
            check!(r.kind == expected(k));
        }
    }
}

/// 200 positives per kind at each noise level (a steady hand, a rough one, the rough one on a
/// slow, dense pen, the rough one with a tremor), every one handed over as the hold does, with its
/// rest at the end. The plan's 94% holds for the first three; the tremor bends polygon edges to
/// the plan's own straightness limit, so it only has to stay clear of a collapse.
fn noisy_positives_reach_the_rate() {
    for noise in NOISE_ALL {
        let need = if noise == Noise::Tremor { 90.0 } else { 94.0 };
        let mut rng = Rng::new(seed(20261027));
        for k in POS_ALL {
            let mut ok = 0;
            let n = 200;
            let mut wrong: Vec<String> = Vec::new();
            for _ in 0..n {
                let s = held(positive(k, &mut rng, noise), &mut rng);
                let r = rec(&s);
                if r.kind == expected(k) {
                    ok += 1;
                } else if wrong.len() < 4 {
                    wrong.push(format!("d{:.0} {:?}/{:?} c{} w{:.2} t{:.2} g{:.2} rms{:.3} max{:.3} ax{:.2}", r.features.diag_mm, r.kind, r.reason, r.features.corners, r.features.winding, r.features.turning, r.features.gap, r.features.rms, r.features.max_err, r.features.axis_ratio));
                }
            }
            let rate = ok as f64 / n as f64 * 100.0;
            eprintln!("  {noise:?} {k:?}: {ok}/{n} = {rate:.1}%");
            for w in &wrong {
                eprintln!("    {w}");
            }
            check!(rate >= need);
        }
    }
}

// ---- negatives ----

fn writing_and_scribbles_never_snap() {
    let mut rng = Rng::new(seed(99));
    let mut total = 0;
    let mut snapped = 0;
    let reasons = std::cell::RefCell::new([0i32; 10]);
    // each as written, and again with a hold's rest at its end (the accident the gates are for)
    let mut tally = |name: &str, s: Vec<PenSample>, rng: &mut Rng| {
        let h = held(s.clone(), rng);
        for s in [s, h] {
            let r = rec(&s);
            reasons.borrow_mut()[r.reason as usize] += 1;
            total += 1;
            if r.kind != ShapeKind::None {
                snapped += 1;
                eprintln!("  false snap: {name} -> {:?} {:?}", r.kind, r.features);
            }
        }
    };
    for _ in 0..300 {
        let s = cursive_word(&mut rng);
        tally("cursive", s, &mut rng);
    }
    for _ in 0..300 {
        let s = letter(&mut rng);
        tally("letter", s, &mut rng);
    }
    for d in 0..10 {
        for _ in 0..40 {
            let s = digit(d, &mut rng);
            tally("digit", s, &mut rng);
        }
    }
    for _ in 0..300 {
        let s = cjk_stroke(&mut rng);
        tally("cjk", s, &mut rng);
    }
    for _ in 0..300 {
        let s = scribble(&mut rng);
        tally("scribble", s, &mut rng);
    }
    for _ in 0..300 {
        let s = small_shape(&mut rng);
        tally("small", s, &mut rng);
    }
    for k in DOODLE_ALL {
        for _ in 0..100 {
            let s = crossing_doodle(k, &mut rng);
            tally("doodle", s, &mut rng);
        }
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
    let mut rng = Rng::new(5);
    let mut at1 = 0;
    let (mut lines2, mut others2) = (0, 0);
    for s in &page.strokes {
        // as written, and held at its end
        let h = with_hold(s.pts.clone(), 150, 0.05, &mut rng);
        for pts in [&s.pts, &h] {
            if rec(pts).kind != ShapeKind::None {
                at1 += 1;
            }
            let mut configured=shape_params(DPI,1.0,PAPER_BLANK,page.w);
            configured.quad_mode=1;configured.circle_mode=1;
            check!(shape_recognize(pts,&configured).kind==ShapeKind::None);
            // twice the size: zoom 2 (screen mm = page mm x zoom), the gates see the stroke doubled
            let r2 = shape_recognize(pts, &shape_params(DPI, 2.0, PAPER_BLANK, page.w));
            match r2.kind {
                ShapeKind::None => {}
                ShapeKind::Line => lines2 += 1,
                k => {
                    others2 += 1;
                    eprintln!("  at 2x: {k:?} {:?}", r2.features);
                }
            }
        }
    }
    eprintln!("  owner's strokes, written and held: {at1} snaps at written size; at 2x {lines2} lines, {others2} other shapes");
    check!(at1 == 0);
    check!(others2 == 0);
}

// ---- exact outputs ----

fn exact_point_counts() {
    // Each with and without a hold's rest at its end: the rest changes nothing.
    for hold in [false, true] {
        let mk = |path: &[Pt]| if hold { clean_held(path) } else { clean(path) };
        // A level 41 mm line: ceil(41 / 2) = 21 pieces, 22 points.
        let r = rec(&mk(&[(20.0, 50.0), (61.0, 50.0)]));
        check!(r.kind == ShapeKind::Line);
        check!(r.pts.len() == 22);
        check!(r.pts[0].y == r.pts[r.pts.len() - 1].y);
        // A level 31 mm square started at its corner: 16 pieces an edge, back to the start: 65.
        let r = rec(&mk(&trace_polygon(&[(20.0, 20.0), (51.0, 20.0), (51.0, 51.0), (20.0, 51.0)], 0.0, true, 0.0)));
        check!(r.kind == ShapeKind::Square);
        check!(r.pts.len() == 65);
        if r.kind == ShapeKind::Square {
            check_near!(mm_of(r.pts[0].x), 20.0, 0.3);
            check_near!(mm_of(r.pts[0].y), 20.0, 0.3);
            check_near!(d_px(&r.pts[0], &r.pts[16]), px(31.0), px(0.3));
            check!(r.pts[16].y == r.pts[0].y); // level: on the page's axes
        }
        // A 20 mm radius circle: N = ceil(2 pi / acos(1 - 0.25 / 236.22)) = 137 vertices, then the
        // first.
        let c: Vec<Pt> = trace_ellipse(20.0, 20.0, 0.0, true, 0.0).iter().map(|&(x, y)| (x + 60.0, y + 60.0)).collect();
        let r = rec(&mk(&c));
        check!(r.kind == ShapeKind::Circle);
        check!(r.pts.len() == 138);
    }
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
    // The line gate's edge at zoom 2: 6.1 page mm is 12.2 on the glass, a line; 5.9 is 11.8, too
    // short.
    check!(shape_recognize(&clean(&[(30.0, 30.0), (36.1, 30.0)]), &shape_params(DPI, 2.0, PAPER_BLANK, 1860)).kind == ShapeKind::Line);
    check!(shape_recognize(&clean(&[(30.0, 30.0), (35.9, 30.0)]), &shape_params(DPI, 2.0, PAPER_BLANK, 1860)).reason == ShapeReason::LineTooShort);
    // and the thickness gate's: a 30 x 2.25 page mm rectangle is 4.5 thick at zoom 2, 1.75 too thin
    let thin = |h: f64| clean(&trace_polygon(&[(20.0, 20.0), (50.0, 20.0), (50.0, 20.0 + h), (20.0, 20.0 + h)], 0.2, true, 0.03));
    check!(shape_recognize(&thin(2.25), &shape_params(DPI, 2.0, PAPER_BLANK, 1860)).kind == ShapeKind::Rectangle);
    check!(shape_recognize(&thin(1.75), &shape_params(DPI, 2.0, PAPER_BLANK, 1860)).reason == ShapeReason::TooThin);
}

/// The sizes at the gates' edges: lines just over 12 mm, rectangles 4.5-6 mm thick (the gate is 4).
fn the_gate_edges() {
    let mut bad = 0;
    for l in [12.1f64, 12.5, 13.0] {
        for deg in [0.0f64, 30.0, 45.0, 77.0, 160.0] {
            let (c, s) = (deg.to_radians().cos(), deg.to_radians().sin());
            let (a, b) = ((60.0 - l / 2.0 * c, 60.0 - l / 2.0 * s), (60.0 + l / 2.0 * c, 60.0 + l / 2.0 * s));
            for (p, q) in [(a, b), (b, a)] {
                let r = rec(&clean_held(&[p, q]));
                if r.kind != ShapeKind::Line {
                    bad += 1;
                    eprintln!("  {l} mm line at {deg}: {:?} {:?}", r.kind, r.reason);
                }
            }
        }
    }
    check!(bad == 0);
    check!(rec(&clean(&[(30.0, 30.0), (41.8, 30.0)])).reason == ShapeReason::LineTooShort);
    // noisy lines 12.2-13 mm, held: at the rate. The line ends where the rest does, so one whose
    // rest moved its end under the gate is rightly too short and is not counted.
    let mut rng = Rng::new(seed(12));
    let (mut ok, mut n) = (0, 0);
    while n < 200 {
        let l = rng.range(12.2, 13.0);
        let rot = rng.range(0.0, std::f64::consts::TAU);
        let path = place(&[(-l / 2.0, 0.0), (l / 2.0, 0.0)], rot, 70.0, 70.0);
        let h = hand_for(l, l, Noise::Steady, &mut rng);
        let s = held(pen(&path, h, &mut rng), &mut rng);
        if d_px(&s[0], &s[s.len() - 1]) < px(12.1) {
            continue;
        }
        n += 1;
        let r = rec(&s);
        if r.kind == ShapeKind::Line {
            ok += 1;
        } else {
            eprintln!("    {l:.2} mm: {:?} straight {:.3} offset {:.2}", r.reason, r.features.straight, r.features.offset_mm);
        }
    }
    eprintln!("  12-13 mm lines, steady and held: {ok}/{n}");
    check!(ok >= 188);
    // rectangles 4.5-6 mm thick, 20-60 mm long: clean ones all, steady held ones at the rate
    let mut bad = 0;
    for h in [4.5f64, 5.0, 6.0] {
        for w in [20.0f64, 40.0, 60.0] {
            for deg in [0.0f64, 20.0, 90.0] {
                let v = [(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)];
                let r = rec(&clean_held(&place(&trace_polygon(&v, 0.15, true, 0.04), deg.to_radians(), 70.0, 70.0)));
                if r.kind != ShapeKind::Rectangle {
                    bad += 1;
                    eprintln!("  {w} x {h} at {deg}: {:?} {:?} c{}", r.kind, r.reason, r.features.corners);
                }
            }
        }
    }
    check!(bad == 0);
    let mut ok = 0;
    for _ in 0..200 {
        let (w, h) = (rng.range(20.0, 60.0), rng.range(4.5, 6.0));
        let v = [(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)];
        let path = place(&trace_polygon(&v, rng.f(), rng.coin(), rng.range(-0.03, 0.08)), rng.range(0.0, std::f64::consts::TAU), 70.0, 70.0);
        let hd = hand_for(h, w, Noise::Steady, &mut rng);
        let s = held(pen(&path, hd, &mut rng), &mut rng);
        ok += (rec(&s).kind == ShapeKind::Rectangle) as i32;
    }
    eprintln!("  4.5-6 mm thick rectangles, steady and held: {ok}/200");
    check!(ok >= 188);
    // 3.5 mm thick is under the gate
    let v = [(20.0, 20.0), (50.0, 20.0), (50.0, 23.5), (20.0, 23.5)];
    check!(rec(&clean(&trace_polygon(&v, 0.2, true, 0.03))).reason == ShapeReason::TooThin);
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
    // A level rectangle's corners 0.85-0.99 mm off their nodes: all four land on them.
    let v = [(20.7, 40.6), (50.6, 40.6), (50.6, 60.7), (20.7, 60.7)];
    let r = shape_recognize(&clean(&trace_polygon(&v, 0.0, true, 0.0)), &g);
    check!(r.kind == ShapeKind::Rectangle);
    let (x0, y0, x1, y1) = bbox(&r);
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

/// A line the angle snap made level, upright or diagonal keeps that angle on the grid, even when
/// only one end is near a node: the line moves whole onto that node, and the other end slides
/// along it onto a node only if one is on the line.
fn the_grid_keeps_a_snapped_lines_angle() {
    let pitch = mm_to_px(5.0, DPI) as f64;
    let node = |i: i32| (pitch * i as f64).round_ties_even();
    let w = 1860;
    let p32 = mm_to_px(5.0, DPI);
    let (ox, oy) = (((w as f32 % p32) / 2.0 + p32 / 2.0) as f64, mm_to_px(10.0, DPI) as f64);
    for paper in [PAPER_GRID, PAPER_DOTS] {
        let g = shape_params(DPI, 1.0, paper, w);
        // the node (i, j) in page px, and in mm
        let at = |i: i32, j: i32| -> (f64, f64) {
            if paper == PAPER_GRID {
                (node(i), node(j))
            } else {
                (ox + i as f64 * pitch, oy + j as f64 * pitch)
            }
        };
        let mm = |v: (f64, f64)| (mm_of(v.0 as f32), mm_of(v.1 as f32));
        // level: the start 0.5, 0.5 mm off node (4, 6), the end 2 mm from any (the reviewer's case)
        let n = mm(at(4, 6));
        let r = shape_recognize(&clean(&[(n.0 + 0.5, n.1 + 0.5), (n.0 + 22.5, n.1 + 0.5)]), &g);
        check!(r.kind == ShapeKind::Line);
        check_near!(r.pts[0].x, at(4, 6).0, 0.01);
        check_near!(r.pts[0].y, at(4, 6).1, 0.01);
        check!(r.pts[0].y == r.pts[r.pts.len() - 1].y);
        check_near!(mm_of(r.pts[r.pts.len() - 1].x), n.0 + 22.0, 0.05);
        // the same with the end near its node and the start far: the end carries the line
        let r = shape_recognize(&clean(&[(n.0 - 1.5, n.1 + 0.6), (n.0 + 20.3, n.1 + 0.6)]), &g);
        check!(r.kind == ShapeKind::Line);
        let e = r.pts[r.pts.len() - 1];
        check_near!(e.x, at(8, 6).0, 0.01);
        check_near!(e.y, at(8, 6).1, 0.01);
        check!(r.pts[0].y == e.y);
        // upright: the start 0.4, 0.5 mm off node (6, 4), the end off its nodes
        let n = mm(at(6, 4));
        let r = shape_recognize(&clean(&[(n.0 + 0.4, n.1 + 0.5), (n.0 + 0.4, n.1 + 22.5)]), &g);
        check!(r.kind == ShapeKind::Line);
        check_near!(r.pts[0].x, at(6, 4).0, 0.01);
        check!(r.pts[0].x == r.pts[r.pts.len() - 1].x);
        // diagonal, 1 degree off 45: the start 0.6, 0.3 mm off node (4, 6), the end 2.8 mm from
        // any node
        let n = mm(at(4, 6));
        let a = 46f64.to_radians();
        let l = 17.0 * 2f64.sqrt();
        let r = shape_recognize(&clean(&[(n.0 + 0.6, n.1 + 0.3), (n.0 + 0.6 + l * a.cos(), n.1 + 0.3 + l * a.sin())]), &g);
        check!(r.kind == ShapeKind::Line);
        check_near!(r.pts[0].x, at(4, 6).0, 0.01);
        check_near!(r.pts[0].y, at(4, 6).1, 0.01);
        check_near!(line_deg(&r), 45.0, 1e-4);
        // diagonal with the end near the node on its line (8, 10): it slides onto the line's
        // point level with that node (on graph paper the node can be a px off the exact 45)
        let r = shape_recognize(&clean(&[(n.0 + 0.6, n.1 + 0.3), (n.0 + 20.3, n.1 + 20.2)]), &g);
        check!(r.kind == ShapeKind::Line);
        check_near!(line_deg(&r), 45.0, 1e-4);
        let e = r.pts[r.pts.len() - 1];
        let m = at(8, 10);
        check!(((e.x as f64 - m.0).powi(2) + (e.y as f64 - m.1).powi(2)).sqrt() <= 1.0);
        // a slanted line (20 degrees, no snap) has no angle to keep: each end to its own node
        let a = 20f64.to_radians();
        let (sx, sy) = (n.0 + 0.3, n.1 + 0.3);
        let r = shape_recognize(&clean(&[(sx, sy), (sx + 30.0 * a.cos(), sy + 30.0 * a.sin())]), &g);
        check!(r.kind == ShapeKind::Line);
        check_near!(r.pts[0].x, at(4, 6).0, 0.01);
        check_near!(r.pts[0].y, at(4, 6).1, 0.01);
    }
}

/// The grid's window is SHAPE_GRID_SNAP_MM on the glass, measured from the node: 0.9 mm snaps,
/// 1.1 does not, at zoom 1 and zoom 2 (where it is half that on the page), on both grid papers.
fn the_grid_window_is_screen_mm() {
    let pitch = mm_to_px(5.0, DPI) as f64;
    let w = 1860;
    let p32 = mm_to_px(5.0, DPI);
    let (ox, oy) = (((w as f32 % p32) / 2.0 + p32 / 2.0) as f64, mm_to_px(10.0, DPI) as f64);
    for paper in [PAPER_GRID, PAPER_DOTS] {
        let at = |i: i32, j: i32| -> (f64, f64) {
            if paper == PAPER_GRID {
                ((pitch * i as f64).round_ties_even(), (pitch * j as f64).round_ties_even())
            } else {
                (ox + i as f64 * pitch, oy + j as f64 * pitch)
            }
        };
        for zoom in [1.0f32, 2.0] {
            let g = shape_params(DPI, zoom, paper, w);
            for d in [0.9f64, 1.1] {
                // off the node along its diagonal by d screen mm (d / zoom page mm)
                let off = d / zoom as f64 / 2f64.sqrt();
                let n = at(4, 6);
                let (nx, ny) = (mm_of(n.0 as f32), mm_of(n.1 as f32));
                let start = (nx + off, ny + off);
                // a level line on to 2.5 mm from the nodes
                let line = shape_recognize(&clean(&[start, (start.0 + 22.5 - off, start.1)]), &g);
                check!(line.kind == ShapeKind::Line);
                let p0 = line.pts[0];
                let on = (p0.x as f64 - n.0).abs() < 0.01 && (p0.y as f64 - n.1).abs() < 0.01;
                check!(on == (d < 1.0));
                if d > 1.0 {
                    check_near!(mm_of(p0.x), start.0, 0.05);
                }
                // a level rectangle whose top left corner is that far off the node, the others
                // 2.5 mm from any
                let v = [start, (nx + 32.5, start.1), (nx + 32.5, ny + 17.5), (start.0, ny + 17.5)];
                let r = shape_recognize(&clean(&trace_polygon(&v, 0.3, true, 0.03)), &g);
                check!(r.kind == ShapeKind::Rectangle);
                let (x0, y0, _, _) = bbox(&r);
                let on = (x0 as f64 - n.0).abs() < 0.01 && (y0 as f64 - n.1).abs() < 0.01;
                check!(on == (d < 1.0));
            }
        }
    }
}

/// A level rectangle's corners snap by their distance to a node, and move their two edges with
/// them; a square moves whole and keeps its sides equal.
fn the_grid_snaps_rectangle_corners() {
    let pitch = mm_to_px(5.0, DPI) as f64;
    let node = |i: i32| (pitch * i as f64).round_ties_even() as f32;
    let g = shape_params(DPI, 1.0, PAPER_GRID, 1860);
    let rect = |v: [Pt; 4], p: &ShapeParams| shape_recognize(&clean(&trace_polygon(&v, 0.3, true, 0.03)), p);
    // edges at x = 20.5 and 52.5, y = 42.5 and 62.5: each edge near a grid line, but every corner
    // 2.55 mm from a node, so nothing moves (edge by edge, the left edge would have)
    let r = rect([(20.5, 42.5), (52.5, 42.5), (52.5, 62.5), (20.5, 62.5)], &g);
    check!(r.kind == ShapeKind::Rectangle);
    let (x0, y0, x1, y1) = bbox(&r);
    check_near!(mm_of(x0), 20.5, 0.1);
    check_near!(mm_of(y0), 42.5, 0.1);
    check_near!(mm_of(x1), 52.5, 0.1);
    check_near!(mm_of(y1), 62.5, 0.1);
    // every corner 0.9 mm off in x and in y: 1.27 mm from its node, outside the window
    let r = rect([(20.9, 40.9), (50.9, 40.9), (50.9, 60.9), (20.9, 60.9)], &g);
    check!(r.kind == ShapeKind::Rectangle);
    let (x0, y0, _, _) = bbox(&r);
    check_near!(mm_of(x0), 20.9, 0.1);
    check_near!(mm_of(y0), 40.9, 0.1);
    // one corner 0.5 mm from its node: it and its two edges go there, the far edges stay
    let r = rect([(20.4, 40.3), (52.5, 40.3), (52.5, 62.5), (20.4, 62.5)], &g);
    check!(r.kind == ShapeKind::Rectangle);
    let (x0, y0, x1, y1) = bbox(&r);
    check!(x0 == node(4) && y0 == node(8));
    check_near!(mm_of(x1), 52.5, 0.1);
    check_near!(mm_of(y1), 62.5, 0.1);
    // a square: its top left corner 0.5 mm off a node, its side 30.4 mm; it moves onto the node
    // and its side becomes 30 mm, the corner beside it being 0.4 mm from its node
    let r = rect([(20.3, 40.4), (50.7, 40.4), (50.7, 70.8), (20.3, 70.8)], &g);
    check!(r.kind == ShapeKind::Square);
    let (x0, y0, x1, y1) = bbox(&r);
    check!(x0 == node(4) && y0 == node(8) && x1 == node(10));
    check!(x1 - x0 == y1 - y0);
    // Dot grid: the corners 0.4-0.6 mm off four dots all land on them.
    let w = 1860;
    let d = shape_params(DPI, 1.0, PAPER_DOTS, w);
    let p32 = mm_to_px(5.0, DPI);
    let (ox, oy) = (((w as f32 % p32) / 2.0 + p32 / 2.0) as f64, mm_to_px(10.0, DPI) as f64);
    let dot = |i: i32, j: i32| (mm_of((ox + i as f64 * pitch) as f32), mm_of((oy + j as f64 * pitch) as f32));
    let (a, b) = (dot(2, 3), dot(8, 7));
    let r = rect([(a.0 + 0.4, a.1 - 0.3), (b.0 - 0.3, a.1 - 0.3), (b.0 - 0.3, b.1 + 0.4), (a.0 + 0.4, b.1 + 0.4)], &d);
    check!(r.kind == ShapeKind::Rectangle);
    let (x0, y0, x1, y1) = bbox(&r);
    check_near!(x0, ox + 2.0 * pitch, 0.01);
    check_near!(y0, oy + 3.0 * pitch, 0.01);
    check_near!(x1, ox + 8.0 * pitch, 0.01);
    check_near!(y1, oy + 7.0 * pitch, 0.01);
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
    check_near!(line_deg(&r), 7.0, 0.3);
    // Music paper is not ruled: kept too.
    let r = shape_recognize(&s, &shape_params(DPI, 1.0, PAPER_MUSIC, 1860));
    check!(r.kind == ShapeKind::Line && r.pts[0].y != r.pts[r.pts.len() - 1].y);
    // 3 degrees snaps level anywhere; 44 degrees snaps to 45.
    let a = 3f64.to_radians();
    let r = rec(&clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]));
    check!(r.kind == ShapeKind::Line && r.pts[0].y == r.pts[r.pts.len() - 1].y);
    let a = 44f64.to_radians();
    let r = rec(&clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]));
    check!(r.kind == ShapeKind::Line);
    check_near!(line_deg(&r), 45.0, 0.01);
    // A ruled page's 15 degree line stays slanted.
    let a = 15f64.to_radians();
    let r = shape_recognize(&clean(&[(20.0, 50.0), (20.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())]), &shape_params(DPI, 1.0, PAPER_MEDIUM, 1860));
    check!(r.kind == ShapeKind::Line && r.pts[0].y != r.pts[r.pts.len() - 1].y);
}

fn closed_shapes_close_exactly_and_keep_start_and_direction() {
    let mut rng = Rng::new(5);
    for k in [Pos::Rectangle, Pos::Square, Pos::Quad, Pos::Triangle, Pos::Ellipse, Pos::Circle] {
        let mut worst = 0.0f64;
        for _ in 0..30 {
            let s = held(positive(k, &mut rng, Noise::Steady), &mut rng);
            let r = rec(&s);
            if r.kind == ShapeKind::None {
                continue;
            }
            check!(closes_exactly(&r));
            // The start: the outline's point nearest the stroke's first point, or for a polygon
            // the vertex within a point step (2 page mm) of that.
            let near = d_outline(&s[0], &r.pts);
            let excess = d_px(&r.pts[0], &s[0]) - near;
            let allow = if matches!(k, Pos::Ellipse | Pos::Circle) { px(0.05) } else { px(SHAPE_POINT_MM as f64) + 0.5 };
            worst = worst.max(excess);
            check!(excess <= allow);
            // the direction: the same turn as the stroke's
            check!(area2(&r.pts) * area2(&s) > 0.0);
        }
        eprintln!("  {k:?}: the start at most {:.2} mm farther from the first point than the outline is", mm_of(worst as f32));
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
    // With a hold's rest after it (its samples at the last one's light pressure), the pressure is
    // still the drawing's, every point but the last is timed within the drawing, and the last
    // keeps the rest's end: the time the stroke really ended.
    let h = with_hold(s.clone(), 150, 0.05, &mut Rng::new(9));
    let rh = rec(&h);
    check!(rh.kind == ShapeKind::Line);
    check!(rh.pts.iter().all(|q| q.pressure == 0.6));
    let mh = rh.pts.len();
    check!(rh.pts[mh - 1].t_ms == h[h.len() - 1].t_ms);
    let mut ok = true;
    for w in rh.pts[..mh - 1].windows(2) {
        ok &= w[1].t_ms.wrapping_sub(w[0].t_ms) < 1000;
    }
    ok &= rh.pts[mh - 2].t_ms.wrapping_sub(s[0].t_ms) <= span as u32;
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

/// The rest the hold leaves at the end of every stroke it hands over is cut before any gate: it
/// adds no path, no turns and no straightness loss. The reviewer's probe: a clean 15 mm line or
/// square, or a 40 mm circle, with 130 frames of rest jittering up to 0.1 mm; and the same with
/// the nib wandering as far as the trigger lets it (within 0.9 mm), which the thinning alone
/// would keep as a zigzag.
fn the_hold_is_cut() {
    let mut rng = Rng::new(11);
    let line = clean(&[(20.0, 40.0), (35.0, 40.0)]);
    let sq = clean(&trace_polygon(&[(20.0, 20.0), (35.0, 20.0), (35.0, 35.0), (20.0, 35.0)], 0.1, true, 0.03));
    let circ: Vec<Pt> = trace_ellipse(20.0, 20.0, 0.3, true, 0.03).iter().map(|&(x, y)| (x + 60.0, y + 60.0)).collect();
    let circ = clean(&circ);
    let mut bad = 0;
    for rest in 0..6 {
        for _ in 0..10 {
            for (want, s) in [(ShapeKind::Line, &line), (ShapeKind::Square, &sq), (ShapeKind::Circle, &circ)] {
                let h = match rest {
                    0 => with_hold(s.clone(), 130, 0.02, &mut rng),
                    1 => with_hold(s.clone(), 130, 0.05, &mut rng),
                    2 => with_hold(s.clone(), 130, 0.1, &mut rng),
                    3 => with_wander(s.clone(), 130, 0.3, &mut rng),
                    4 => with_wander(s.clone(), 130, 0.6, &mut rng),
                    _ => with_wander(s.clone(), 200, 0.9, &mut rng),
                };
                let r = rec(&h);
                if r.kind != want {
                    bad += 1;
                    eprintln!("  held {want:?} rest {rest}: {:?} {:?} t{:.2}", r.kind, r.reason, r.features.turning);
                }
            }
        }
    }
    check!(bad == 0);
    // the features see none of it
    let (a, b) = (rec(&line), rec(&with_wander(line.clone(), 200, 0.9, &mut rng)));
    check!(b.kind == ShapeKind::Line && (a.features.straight - b.features.straight).abs() < 0.005);
    let (a, b) = (rec(&sq), rec(&with_wander(sq.clone(), 200, 0.9, &mut rng)));
    check!(b.kind == ShapeKind::Square && (a.features.turning - b.features.turning).abs() < 0.1);
    check!(SHAPE_HOLD_MM == 1.0);
}

/// The closure cuts one end only (an overshoot or a lead-in), and what it cuts must lie on the
/// shape: a stroke whose ends cross and fly apart, or a shape with a stick, is no shape.
fn the_closure_cuts_one_end() {
    // still closed: an overshoot of 12% along the start, a run 3 mm on past the start corner, a
    // 2 mm lead-in to the corner
    let sq = [(20.0, 20.0), (50.0, 20.0), (50.0, 50.0), (20.0, 50.0)];
    check!(rec(&clean(&trace_polygon(&sq, 0.1, true, 0.12))).kind == ShapeKind::Square);
    check!(rec(&clean(&[(20.0, 20.0), (50.0, 20.0), (50.0, 50.0), (20.0, 50.0), (20.0, 20.0), (20.0, 17.0)])).kind == ShapeKind::Square);
    check!(rec(&clean(&[(18.6, 21.4), (20.0, 20.0), (50.0, 20.0), (50.0, 50.0), (20.0, 50.0), (20.0, 20.5)])).kind == ShapeKind::Square);
    let circle = |a0: f64, turns: f64| -> Vec<Pt> {
        (0..=200).map(|i| {
            let t = a0 - std::f64::consts::TAU * turns * i as f64 / 200.0;
            (60.0 + 15.0 * t.cos(), 90.0 - 15.0 * t.sin())
        }).collect()
    };
    check!(rec(&clean(&circle(1.0, 1.12))).kind == ShapeKind::Circle);
    // The reviewer's fish: a tail in from (48, 108) mm, once round a 15 mm circle, a tail out to
    // (72, 108): its ends half its diagonal apart. Not a circle, and the log says how far apart.
    let mut fish = vec![(48.0, 108.0)];
    fish.extend(circle(-0.75 * std::f64::consts::PI, 1.0));
    fish.push((72.0, 108.0));
    let r = rec(&clean(&fish));
    eprintln!("  the fish: gap {:.3} of the diagonal after the cut, its ends {:.3} apart", r.features.gap, r.features.end_gap);
    check!(r.kind == ShapeKind::None);
    check!(r.features.end_gap > 0.4);
    // (one end cut, the gap is 0.19 of the diagonal; cut at both, nearer where its tails meet, it
    // would be 0.12)
    check!(r.features.gap > 0.15);
    // A balloon on its string, drawn string last (a 14 mm tail out: its ends 26% of the diagonal
    // apart, so only a cut closes it) and string first: the string is not on the circle.
    let mut balloon = circle(-std::f64::consts::FRAC_PI_2, 1.0);
    balloon.push((60.0, 105.0 + 14.0));
    check!(rec(&clean(&balloon)).kind == ShapeKind::None);
    balloon.reverse();
    check!(rec(&clean(&balloon)).kind == ShapeKind::None);
    // A sign: a 20 mm post drawn first, then the 30 mm square on it; a polygon, but the post is
    // far from the square's outline.
    let sign = [(35.0, 70.0), (35.0, 50.0), (50.0, 50.0), (50.0, 20.0), (20.0, 20.0), (20.0, 50.0), (35.0, 50.0)];
    let r = rec(&clean(&sign));
    check!(r.kind == ShapeKind::None);
    // Where it closes the loop must not double back: a 12 mm diamond turned 45 degrees and drawn
    // backwards, sampled at its trace's own points (0.1 mm apart), keeps an end half a step past
    // its start, which read as a corner in mid-edge.
    let h = 6.0;
    let dia = place(&trace_polygon(&[(-h - 2.0, 0.0), (0.0, -h * 0.6), (h + 2.0, 0.0), (0.0, h * 0.6)], 0.1, false, 0.05), std::f64::consts::FRAC_PI_4, 80.0, 80.0);
    let s: Vec<PenSample> = dia.iter().enumerate().map(|(k, &(x, y))| PenSample { x: px(x) as f32, y: px(y) as f32, pressure: 0.5, t_ms: 3 * k as u32, ..Default::default() }).collect();
    check!(rec(&s).kind == ShapeKind::Quad);
    // The crossing doodles (alpha, fish, ribbon), 15-40 mm: never a shape, their end gap logged.
    let mut rng = Rng::new(seed(77));
    let mut bad = 0;
    for k in DOODLE_ALL {
        for _ in 0..40 {
            let r = rec(&crossing_doodle(k, &mut rng));
            if r.kind != ShapeKind::None {
                bad += 1;
                eprintln!("  {k:?}: {:?} {:?}", r.kind, r.features);
            }
            check!(r.features.end_gap >= r.features.gap);
        }
    }
    check!(bad == 0);
}

fn the_kinds_and_their_snaps() {
    // a level rectangle 3 degrees off stays exactly level; a 20 degree one keeps its turn
    let turned = |deg: f64, w: f64, h: f64| -> Vec<PenSample> {
        let v = [(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)];
        clean(&place(&trace_polygon(&v, 0.1, true, 0.03), deg.to_radians(), 60.0, 60.0))
    };
    let on_axes = |r: &ShapeResult| -> bool {
        let (x0, y0, x1, y1) = bbox(r);
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
    let (x0, y0, x1, _) = bbox(&r);
    check_near!(mm_of(x1 - x0), 60.0, 0.5);
    check_near!(mm_of(y0), 55.0, 0.5);
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
    // a held dot: all of it within the hold's radius
    let s = with_hold(vec![PenSample { x: 500.0, y: 500.0, ..Default::default() }], 300, 0.1, &mut Rng::new(2));
    check!(rec(&s).kind == ShapeKind::None);
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

/// Every kind at 12, 20, 40 and 80 mm, turned 0, 30, 45 and 77 degrees, both ways round, clean,
/// from a pen sampling every 0.25 mm and from a slow one sampling every 0.1 mm (where the samples
/// fall decides which point the closure keeps, and a clean shape must not depend on it).
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
                    for step in [PEN_STEP_MM, 0.1] {
                        let r = rec(&pen(&path, Hand { step, ..CLEAN }, &mut Rng::new(1)));
                        if r.kind != want {
                            bad += 1;
                            eprintln!("  {size} mm {deg} deg {fwd}, samples {step} mm apart: {want:?} -> {:?} {:?}", r.kind, r.reason);
                        }
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

fn with_extras() -> ShapeParams {
    let mut p = prm();
    p.extras = true;
    p
}

/// v1.5's arrows, polylines and arcs (ShapeParams::extras): clean ones all, noisy ones at the
/// rate, and none of them with the flag off (it is off by default).
fn v15_arrows_polylines_and_arcs() {
    let mut rng = Rng::new(seed(15));
    for k in EXTRA_ALL {
        let (mut clean_ok, mut ok, mut off) = (0, 0, 0);
        let n = 200;
        let mut wrong: Vec<String> = Vec::new();
        for _ in 0..40 {
            let s = extra_positive(k, &mut rng, false);
            let r = shape_recognize(&s, &with_extras());
            if r.kind == extra_expected(k) {
                clean_ok += 1;
            } else {
                eprintln!("  clean {k:?}: {:?} {:?} c{} rms{:.3}", r.kind, r.reason, r.features.corners, r.features.rms);
            }
        }
        for _ in 0..n {
            let s = held(extra_positive(k, &mut rng, true), &mut rng);
            let r = shape_recognize(&s, &with_extras());
            if r.kind == extra_expected(k) {
                ok += 1;
            } else if wrong.len() < 6 {
                wrong.push(format!("{:?} {:?} c{} rms{:.3}", r.kind, r.reason, r.features.corners, r.features.rms));
            }
            if matches!(rec(&s).kind, ShapeKind::Arrow | ShapeKind::Polyline | ShapeKind::Arc) {
                off += 1;
            }
        }
        let rate = ok as f64 / n as f64 * 100.0;
        eprintln!("  v1.5 {k:?}: clean {clean_ok}/40, noisy and held {ok}/{n} = {rate:.1}%");
        for w in &wrong {
            eprintln!("    {w}");
        }
        check!(clean_ok == 40);
        check!(rate >= 94.0);
        check!(off == 0);
    }
    // the arrow's chain: tail, tip, barb, tip, barb, the shaft level (1 degree off)
    let s = clean(&[(20.0, 50.0), (60.0, 50.7), (52.0, 45.0), (60.0, 50.7), (51.0, 56.0)]);
    let r = shape_recognize(&s, &with_extras());
    check!(r.kind == ShapeKind::Arrow);
    if r.kind == ShapeKind::Arrow {
        let tipx = r.pts.iter().fold(f32::MIN, |m, q| m.max(q.x));
        let tip = r.pts.iter().find(|q| q.x == tipx).copied().unwrap_or_default();
        check!(r.pts[0].y == tip.y);
        check_near!(mm_of(r.pts[0].x), 20.0, 0.3);
        // the barbs mirror each other about the shaft
        let ends: Vec<PenSample> = r.pts.iter().filter(|q| mm_of(q.x) < 53.5 && mm_of(q.x) > 50.0 && (q.y - tip.y).abs() > px(4.0) as f32).copied().collect();
        check!(ends.len() >= 2);
    }
    // a polyline turning 88 degrees is made square
    let s = clean(&[(20.0, 20.0), (50.0, 20.5), (51.5, 50.0)]);
    let r = shape_recognize(&s, &with_extras());
    check!(r.kind == ShapeKind::Polyline);
    if r.kind == ShapeKind::Polyline {
        let c = corners_open(&r.pts);
        check!(c.len() == 1);
        if c.len() == 1 {
            let (a, b, e) = (r.pts[0], r.pts[c[0]], r.pts[r.pts.len() - 1]);
            check!(a.y == b.y && b.x == e.x);
        }
    }
    // the arc's point count follows the vertex formula: half of 137 vertices, ceil(68.5) = 69
    // pieces, 70 points
    let arc: Vec<Pt> = (0..=200)
        .map(|i| {
            let t = std::f64::consts::PI * i as f64 / 200.0;
            (60.0 + 20.0 * t.cos(), 60.0 + 20.0 * t.sin())
        })
        .collect();
    let r = shape_recognize(&clean(&arc), &with_extras());
    check!(r.kind == ShapeKind::Arc);
    check!(r.pts.len() == 70);
    check!(d_px(&r.pts[0], &clean(&arc)[0]) < px(0.2));
}

/// The indices where an open chain of points turns.
fn corners_open(p: &[PenSample]) -> Vec<usize> {
    let mut out = Vec::new();
    for k in 1..p.len() - 1 {
        let (a, b, c) = (p[k - 1], p[k], p[k + 1]);
        let (ux, uy, wx, wy) = ((b.x - a.x) as f64, (b.y - a.y) as f64, (c.x - b.x) as f64, (c.y - b.y) as f64);
        let cr = ux * wy - uy * wx;
        if cr.abs() / ((ux * ux + uy * uy).sqrt() * (wx * wx + wy * wy).sqrt()) > 0.05 {
            out.push(k);
        }
    }
    out
}

/// With v1.5's kinds on, writing, scribbles, doodles and the owner's strokes still give no shape
/// at written size: the plan's gate for shipping them (step 22). At twice the size the owner's
/// strokes give no closed shape; the open kinds they give (a 'v' with its hook is an arrow by the
/// rule's own numbers) are printed: v1.5 stays off by default, and this says why.
fn v15_negatives_still_pass() {
    let mut rng = Rng::new(seed(99));
    let mut snapped = 0;
    let mut total = 0;
    let mut one = |s: &[PenSample], name: &str| {
        let r = shape_recognize(s, &with_extras());
        total += 1;
        if r.kind != ShapeKind::None {
            snapped += 1;
            eprintln!("  v1.5 false snap: {name} -> {:?} {:?}", r.kind, r.features);
        }
    };
    for _ in 0..300 {
        one(&cursive_word(&mut rng), "cursive");
    }
    for _ in 0..300 {
        one(&letter(&mut rng), "letter");
    }
    for d in 0..10 {
        for _ in 0..40 {
            one(&digit(d, &mut rng), "digit");
        }
    }
    for _ in 0..300 {
        one(&cjk_stroke(&mut rng), "cjk");
    }
    for _ in 0..300 {
        one(&scribble(&mut rng), "scribble");
    }
    for _ in 0..300 {
        one(&small_shape(&mut rng), "small");
    }
    for k in DOODLE_ALL {
        for _ in 0..100 {
            one(&crossing_doodle(k, &mut rng), "doodle");
        }
    }
    let mut out = None;
    check!(page_deserialize(GRIMOIRE_ERASER_SAMPLE, &mut out) == 0);
    let (mut closed2, mut open2) = (0, Vec::new());
    if let Some(page) = out {
        for s in &page.strokes {
            one(&s.pts, "owner");
            one(&with_hold(s.pts.clone(), 150, 0.05, &mut rng), "owner, held");
        }
        for s in &page.strokes {
            let mut p2 = with_extras();
            p2.zoom = 2.0;
            match shape_recognize(&s.pts, &p2).kind {
                ShapeKind::None | ShapeKind::Line => {}
                k @ (ShapeKind::Arrow | ShapeKind::Polyline | ShapeKind::Arc) => open2.push(k),
                k => {
                    closed2 += 1;
                    eprintln!("  v1.5 at 2x: {k:?}");
                }
            }
        }
    }
    eprintln!("  v1.5 negatives: {snapped} false snaps of {total}; the owner's at 2x: {closed2} closed shapes, open ones {open2:?}");
    check!(snapped == 0);
    check!(closed2 == 0);
}

// ---- 2.0.0-rc.12: long ellipses as drawn, uneven quads, Corner roundness ----

fn with_modes(quads: i32, round: i32) -> ShapeParams {
    let mut p = prm();
    p.quad_mode = quads;
    p.circle_mode = round;
    p
}

/// A hand whose wobble grows with the shape's size (`major` mm), as a real one's does.
fn sized_hand(major: f64, rng: &mut Rng) -> Hand {
    Hand { wobble: (major * rng.range(0.004, 0.01)).clamp(0.1, 0.8), jitter: 0.06, round: rng.range(0.3, 1.2), wave: major * rng.range(0.5, 1.0), step: PEN_STEP_MM }
}

/// Ellipses a quarter to four fifths as thick as long, turned any way and drawn as a hand draws
/// them (an egg of up to 3%, ends pointed or sides flatter, wavering with their size, gaps and
/// overshoots, held): Round shapes: As drawn snaps them as ellipses. Their errors used to be held
/// to the minor semi-axis alone, and the long ones failed (of these strokes, 28 in 100 snapped a
/// quarter across, 75 at 0.3).
/// Clean ones keep their axes and their turn; Prefer circles and Circles only map them as before.
fn long_ellipses_snap_as_drawn() {
    let mut rng = Rng::new(seed(1212));
    for aspect in [0.25, 0.3, 0.4, 0.5, 0.65, 0.8] {
        let n = 100;
        let mut ok = 0;
        let mut wrong: Vec<String> = Vec::new();
        for _ in 0..n {
            let a = rng.range(14.0, 45.0);
            let oval = trace_oval(a, a * aspect, rng.range(0.0, 2.0 * PI), rng.coin(), rng.range(-0.04, 0.12), rng.range(0.0, 0.03), rng.range(1.7, 2.4));
            let path = place(&oval, rng.range(0.0, PI), rng.range(60.0, 90.0), rng.range(60.0, 140.0));
            let h = sized_hand(2.0 * a, &mut rng);
            let r = rec(&held(pen(&path, h, &mut rng), &mut rng));
            if r.kind == ShapeKind::Ellipse {
                ok += 1;
            } else if wrong.len() < 3 {
                wrong.push(format!("a{a:.0} {:?}/{:?} c{} rms{:.3} max{:.3} ax{:.2}", r.kind, r.reason, r.features.corners, r.features.rms, r.features.max_err, r.features.axis_ratio));
            }
        }
        eprintln!("  hand-drawn ellipses {aspect:.2} across: {ok}/{n}");
        for w in &wrong {
            eprintln!("    {w}");
        }
        check!(ok >= if aspect < 0.3 { 90 } else { 95 });
    }
    // Clean, at every turn: its semi-axes as drawn, its long axis turned as drawn (or onto the
    // page's axes within 6 degrees).
    for aspect in [0.25, 0.4, 0.6] {
        for deg in (0..180).step_by(25) {
            let th = (deg as f64).to_radians();
            let r = rec(&clean(&place(&trace_ellipse(30.0, 30.0 * aspect, 0.4, deg % 2 == 0, 0.0), th, 70.0, 70.0)));
            check!(r.kind == ShapeKind::Ellipse);
            if r.kind != ShapeKind::Ellipse {
                continue;
            }
            let (cx, cy) = (px(70.0), px(70.0));
            let d = |q: &PenSample| mm_of(1.0) * (q.x as f64 - cx).hypot(q.y as f64 - cy);
            check_near!(r.pts.iter().map(d).fold(0.0, f64::max), 30.0, 0.2);
            check_near!(r.pts.iter().map(d).fold(f64::MAX, f64::min), 30.0 * aspect, 0.2);
            // (its long axis from the outline's second moments: its points are even in the
            // eccentric angle, so they are exact)
            let q = &r.pts[..r.pts.len() - 1];
            let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
            for p in q {
                let (x, y) = (p.x as f64 - cx, p.y as f64 - cy);
                sxx += x * x;
                syy += y * y;
                sxy += x * y;
            }
            let got = (0.5 * (2.0 * sxy).atan2(sxx - syy)).to_degrees().rem_euclid(180.0);
            let want = if deg <= 6 || deg >= 174 { 0.0 } else { deg as f64 };
            check!((got - want).abs().min(180.0 - (got - want).abs()) < 1.0);
        }
    }
    // The modes: 0.8 across is an ellipse as drawn, a circle when circles are preferred (0.72); a
    // third across is an ellipse in both, and a circle only in Circles only.
    let round = clean(&place(&trace_ellipse(30.0, 24.0, 0.3, true, 0.0), 0.5, 70.0, 70.0));
    let long = clean(&place(&trace_ellipse(36.0, 12.0, 0.3, true, 0.0), 0.5, 70.0, 70.0));
    check!(shape_recognize(&round, &with_modes(0, 0)).kind == ShapeKind::Ellipse);
    check!(shape_recognize(&round, &with_modes(0, 1)).kind == ShapeKind::Circle);
    check!(shape_recognize(&long, &with_modes(0, 0)).kind == ShapeKind::Ellipse);
    check!(shape_recognize(&long, &with_modes(0, 1)).kind == ShapeKind::Ellipse);
    check!(shape_recognize(&long, &with_modes(0, 2)).kind == ShapeKind::Circle);
}

/// Rectangles a fifth to nine tenths as thick as long, their corners rounded by the hand up to
/// 7 mm, are never ellipses (nor circles): a box leaves errors about three times the ellipse's gate
/// whatever its aspect, against the longer yardstick too.
fn sloppy_rectangles_are_no_ellipses() {
    let mut rng = Rng::new(seed(4512));
    let mut round = 0;
    for aspect in [0.2, 0.3, 0.45, 0.6, 0.9] {
        for corner in [0.5, 2.0, 4.0, 7.0] {
            for _ in 0..15 {
                let w = rng.range(30.0, 80.0);
                let h = (w * aspect).max(5.0);
                let v = place(&[(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)], rng.range(0.0, PI), 90.0, 110.0);
                let path = trace_polygon(&v, rng.f(), rng.coin(), rng.range(-0.04, 0.10));
                let hand = Hand { round: corner, ..sized_hand(w, &mut rng) };
                let r = rec(&held(pen(&path, hand, &mut rng), &mut rng));
                if matches!(r.kind, ShapeKind::Ellipse | ShapeKind::Circle) {
                    round += 1;
                    eprintln!("  a {aspect} rectangle, corners {corner} mm: {:?} rms{:.3}", r.kind, r.features.rms);
                }
            }
        }
    }
    check!(round == 0);
}

/// The snapped quad's four corners and edges (None unless it has exactly four).
fn quad_corners(r: &ShapeResult) -> Option<Vec<PenSample>> {
    Some(corners_of(&r.pts)).filter(|c| c.len() == 4)
}

/// Whether edges k and k + 2 of the quad are parallel (to a ten-thousandth of a radian).
fn parallel(c: &[PenSample], k: usize) -> bool {
    let e = |k: usize| ((c[(k + 1) % 4].x - c[k].x) as f64, (c[(k + 1) % 4].y - c[k].y) as f64);
    let (a, b) = (e(k), e(k + 2));
    (a.0 * b.1 - a.1 * b.0).abs() / (a.0.hypot(a.1) * b.0.hypot(b.1)) < 1e-4
}

/// The farthest any snapped corner is from the nearest drawn one (mm).
fn corners_off(c: &[PenSample], v: &[Pt]) -> f64 {
    c.iter().map(|q| v.iter().map(|t| mm_of(1.0) * (q.x as f64 - px(t.0)).hypot(q.y as f64 - px(t.1))).fold(f64::MAX, f64::min)).fold(0.0, f64::max)
}

/// Four sides: Uneven allowed keeps a quad that is no rectangle with straight edges and its corners
/// where they were drawn, opposite edges within 10 degrees of parallel made parallel: every
/// parallelogram (slanted either way, turned, thin or tall, to 30 and 150 degree corners, whose
/// obtuse corners turned too little for the corner finder before: none of these at 60 degrees
/// snapped, 44 in 60 at -50) comes out an exact one; a
/// trapezium has its one pair made parallel; a kite stays as drawn. A quad near square in every
/// corner is a rectangle in every mode (a square in Squares only), as a parallelogram is in the
/// two strict modes.
fn uneven_quads_keep_their_corners() {
    let mut rng = Rng::new(seed(2612));
    for slant in [-50.0, -35.0, -20.0, 20.0, 30.0, 45.0, 60.0] {
        let (mut ok, n) = (0, 60);
        for i in 0..n {
            let w = rng.range(25.0, 70.0);
            let h = w * [0.3, 0.6, 1.0][i % 3];
            let v = place(&parallelogram(w, h, slant), rng.range(0.0, 2.0 * PI), 90.0, 110.0);
            let path = trace_polygon(&v, rng.f(), rng.coin(), rng.range(-0.04, 0.10));
            let hand = hand_for(h.min(w), w.max(h), if i % 2 == 0 { Noise::Steady } else { Noise::Rough }, &mut rng);
            let r = rec(&held(pen(&path, hand, &mut rng), &mut rng));
            if r.kind == ShapeKind::Quad {
                if let Some(c) = quad_corners(&r) {
                    ok += (parallel(&c, 0) && parallel(&c, 1)) as i32;
                }
            } else {
                eprintln!("  parallelogram {slant} degrees, {:.0} by {:.0}: {:?} {:?} c{}", w, h, r.kind, r.reason, r.features.corners);
            }
        }
        eprintln!("  parallelograms slanted {slant} degrees: {ok}/{n} exact");
        check!(ok >= 57);
    }
    // Clean and turned off the page's axes, the corners are where they were drawn.
    for (slant, rot) in [(30.0, 20.0), (-45.0, 110.0), (60.0, 200.0), (25.0, 320.0)] {
        let v = place(&parallelogram(50.0, 25.0, slant), (rot as f64).to_radians(), 80.0, 90.0);
        let r = rec(&clean(&trace_polygon(&v, 0.3, true, 0.03)));
        check!(r.kind == ShapeKind::Quad);
        let c = quad_corners(&r);
        check!(c.as_ref().is_some_and(|c| parallel(c, 0) && parallel(c, 1) && corners_off(c, &v) < 0.5));
    }
    // Drawn 3 degrees off level, its base is made level: the quad turned about its middle, so its
    // corners move by the turn (3 degrees, 30 mm out).
    let v = place(&parallelogram(50.0, 25.0, 30.0), 3f64.to_radians(), 80.0, 90.0);
    let r = rec(&clean(&trace_polygon(&v, 0.3, true, 0.03)));
    check!(r.kind == ShapeKind::Quad);
    let c = quad_corners(&r);
    check!(c.as_ref().is_some_and(|c| (0..4).any(|k| c[k].y == c[(k + 1) % 4].y) && corners_off(c, &v) < 2.0));
    // A trapezium keeps its corners, its bases parallel and its legs not; one whose bases are 6
    // degrees apart has them made parallel, each turned 3 degrees about its middle (the long one's
    // ends move 1.6 mm).
    for (tilt, rot) in [(0.0, 25.0), (6.0, 25.0), (0.0, 140.0)] {
        let mut v: Vec<Pt> = vec![(-30.0, 15.0), (-14.0, -15.0), (14.0, -15.0), (30.0, 15.0)];
        // (the top base turned by `tilt` about its middle)
        let t = (tilt as f64).to_radians();
        for k in [1, 2] {
            let x = v[k].0;
            v[k] = (x * t.cos(), -15.0 + x * t.sin());
        }
        let v = place(&v, (rot as f64).to_radians(), 80.0, 90.0);
        let r = rec(&clean(&trace_polygon(&v, 0.2, false, 0.03)));
        check!(r.kind == ShapeKind::Quad);
        let c = quad_corners(&r);
        check!(c.as_ref().is_some_and(|c| (parallel(c, 0) ^ parallel(c, 1)) && corners_off(c, &v) < if tilt > 0.0 { 2.0 } else { 0.5 }));
    }
    // A kite, no two sides near parallel: as drawn.
    let v = place(&[(-30.0, 0.0), (0.0, -17.0), (17.0, 0.0), (0.0, 17.0)], 0.4, 80.0, 90.0);
    let r = rec(&clean(&trace_polygon(&v, 0.1, true, 0.03)));
    check!(r.kind == ShapeKind::Quad);
    let c = quad_corners(&r);
    check!(c.as_ref().is_some_and(|c| !parallel(c, 0) && !parallel(c, 1) && corners_off(c, &v) < 0.5));
    // In each mode: a quad 5 to 7 degrees off square in its corners, and a parallelogram.
    let near: Vec<Pt> = vec![(30.0, 30.0), (64.0, 32.0), (63.0, 55.0), (30.5, 53.0)];
    let near = clean(&trace_polygon(&near, 0.05, true, 0.03));
    let para = clean(&trace_polygon(&place(&parallelogram(40.0, 20.0, 30.0), 0.0, 60.0, 60.0), 0.1, true, 0.03));
    for (mode, a, b) in [(0, ShapeKind::Rectangle, ShapeKind::Quad), (1, ShapeKind::Rectangle, ShapeKind::Rectangle), (2, ShapeKind::Square, ShapeKind::Square)] {
        check!(shape_recognize(&near, &with_modes(mode, 0)).kind == a);
        check!(shape_recognize(&para, &with_modes(mode, 0)).kind == b);
    }
}

/// Corner roundness: a snapped polygon's corners become arcs of one radius, that fraction of the
/// largest that fits, in the stroke's own points (smooth: a circle's spacing of that radius, the
/// edges' 2 mm between); the stroke still closes exactly, starts where the drawing did and goes its
/// way; at the most a square is a circle and an equilateral triangle its inscribed circle; arcs never
/// run into each other; lines, ellipses and circles are no polygons and keep their points.
fn corner_roundness_rounds_polygons() {
    let at = |f: f32| {
        let mut p = prm();
        p.corner = f;
        p
    };
    // A level 60 by 30 mm rectangle at a half: the most is half its short side, 15 mm, so 7.5 mm.
    let rect = clean_held(&trace_polygon(&[(20.0, 20.0), (80.0, 20.0), (80.0, 50.0), (20.0, 50.0)], 0.1, true, 0.03));
    let sharp = shape_recognize(&rect, &at(0.0));
    let r = shape_recognize(&rect, &at(0.5));
    check!(sharp.kind == ShapeKind::Rectangle && r.kind == ShapeKind::Rectangle);
    if r.kind == ShapeKind::Rectangle && sharp.kind == ShapeKind::Rectangle {
        check!(closes_exactly(&r));
        // the edges where they were (the box the same), the corner cut by r (sqrt 2 - 1)
        let (b0, b1) = (bbox(&sharp), bbox(&r));
        check!((b0.0 - b1.0).abs() < 0.01 && (b0.1 - b1.1).abs() < 0.01 && (b0.2 - b1.2).abs() < 0.01 && (b0.3 - b1.3).abs() < 0.01);
        let cut = r.pts.iter().map(|q| mm_of(1.0) * ((q.x - b1.0) as f64).hypot((q.y - b1.1) as f64)).fold(f64::MAX, f64::min);
        check_near!(cut, 7.5 * (2f64.sqrt() - 1.0), 0.05);
        // smooth, and not excessive: no turn over 6 degrees, no piece over 2 mm, under 200 points
        let n = r.pts.len() - 1;
        let mut most: f64 = 0.0;
        for k in 0..n {
            let (a, b, c) = (r.pts[(k + n - 1) % n], r.pts[k], r.pts[(k + 1) % n]);
            let (u, w) = (((b.x - a.x) as f64, (b.y - a.y) as f64), ((c.x - b.x) as f64, (c.y - b.y) as f64));
            most = most.max((u.0 * w.1 - u.1 * w.0).atan2(u.0 * w.0 + u.1 * w.1).abs().to_degrees());
            check!(mm_of(1.0) * u.0.hypot(u.1) <= 2.0 + 1e-6);
        }
        check!(most < 6.0 && r.pts.len() < 200);
        // its start where the sharp one starts (the first point on an edge, a step from a corner),
        // the same way round
        check!(d_px(&r.pts[0], &sharp.pts[0]) < 0.01);
        check!((area2(&r.pts) > 0.0) == (area2(&sharp.pts) > 0.0));
    }
    // At the most: a square is a circle, an equilateral triangle its inscribed circle.
    let sq = shape_recognize(&clean(&trace_polygon(&[(30.0, 30.0), (60.0, 30.0), (60.0, 60.0), (30.0, 60.0)], 0.2, true, 0.03)), &at(1.0));
    check!(sq.kind == ShapeKind::Square);
    check!(sq.pts.iter().all(|q| (mm_of(1.0) * ((q.x as f64 - px(45.0)).hypot(q.y as f64 - px(45.0))) - 15.0).abs() < 0.02));
    let s = 40.0;
    let tri: Vec<Pt> = vec![(30.0, 70.0), (30.0 + s, 70.0), (30.0 + s / 2.0, 70.0 - s * 3f64.sqrt() / 2.0)];
    let t = shape_recognize(&clean(&trace_polygon(&tri, 0.1, true, 0.03)), &at(1.0));
    check!(t.kind == ShapeKind::Triangle);
    if t.kind == ShapeKind::Triangle {
        // (the snapped triangle is made exactly equilateral about its own centre)
        let (cx, cy) = (t.pts.iter().map(|q| q.x as f64).sum::<f64>() / t.pts.len() as f64, t.pts.iter().map(|q| q.y as f64).sum::<f64>() / t.pts.len() as f64);
        let d: Vec<f64> = t.pts.iter().map(|q| (q.x as f64 - cx).hypot(q.y as f64 - cy)).collect();
        let (lo, hi) = (d.iter().cloned().fold(f64::MAX, f64::min), d.iter().cloned().fold(0.0, f64::max));
        check!(mm_of(1.0) * (hi - lo) < 0.3);
        check_near!(mm_of(1.0) * hi, s / (2.0 * 3f64.sqrt()), 0.3);
    }
    // A parallelogram, a trapezium, a kite and a right triangle at every roundness: one turn in all,
    // every turn the same way (no arc runs back into the last).
    let shapes: Vec<Vec<Pt>> = vec![
        place(&parallelogram(50.0, 20.0, 40.0), 0.3, 70.0, 70.0),
        vec![(40.0, 80.0), (55.0, 50.0), (75.0, 50.0), (100.0, 80.0)],
        place(&[(-30.0, 0.0), (0.0, -17.0), (17.0, 0.0), (0.0, 17.0)], 0.4, 80.0, 90.0),
        vec![(30.0, 30.0), (80.0, 30.0), (30.0, 60.0)],
    ];
    for v in &shapes {
        let s = clean(&trace_polygon(v, 0.15, true, 0.03));
        for f in [0.2, 0.6, 1.0] {
            let r = shape_recognize(&s, &at(f));
            check!(matches!(r.kind, ShapeKind::Quad | ShapeKind::Triangle));
            let n = r.pts.len() - 1;
            let (mut sum, mut pos, mut neg) = (0.0, 0, 0);
            for k in 0..n {
                let (a, b, c) = (r.pts[(k + n - 1) % n], r.pts[k], r.pts[(k + 1) % n]);
                let (u, w) = (((b.x - a.x) as f64, (b.y - a.y) as f64), ((c.x - b.x) as f64, (c.y - b.y) as f64));
                let turn = (u.0 * w.1 - u.1 * w.0).atan2(u.0 * w.0 + u.1 * w.1);
                sum += turn;
                // (a straight edge's points turn by float noise, a millionth of a radian)
                if turn > 1e-4 {
                    pos += 1;
                } else if turn < -1e-4 {
                    neg += 1;
                }
            }
            check!((sum.abs() - 2.0 * PI).abs() < 1e-3);
            check!(pos == 0 || neg == 0);
            check!(closes_exactly(&r));
        }
    }
    // Not polygons: the same points at any roundness.
    for path in [vec![(20.0, 20.0), (70.0, 40.0)], trace_ellipse(30.0, 15.0, 0.4, true, 0.0), trace_ellipse(20.0, 20.0, 0.4, true, 0.0)] {
        let path = if path.len() == 2 { path } else { place(&path, 0.3, 70.0, 70.0) };
        let s = clean(&path);
        let (a, b) = (shape_recognize(&s, &at(0.0)), shape_recognize(&s, &at(1.0)));
        check!(a.kind != ShapeKind::None && a.kind == b.kind && a.pts == b.pts);
    }
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
    run!(the_gate_edges);
    run!(the_grid_snaps_within_a_millimetre);
    run!(the_grid_keeps_a_snapped_lines_angle);
    run!(the_grid_window_is_screen_mm);
    run!(the_grid_snaps_rectangle_corners);
    run!(ruled_paper_makes_lines_level);
    run!(closed_shapes_close_exactly_and_keep_start_and_direction);
    run!(the_samples_carry_pressure_tilt_and_time);
    run!(the_snapped_stroke_keeps_tool_params_seed_and_ext);
    run!(the_burst_gate_narrows_the_tolerances);
    run!(the_hold_is_cut);
    run!(the_closure_cuts_one_end);
    run!(the_kinds_and_their_snaps);
    run!(degenerate_input_is_refused);
    run!(every_kind_at_every_size_turn_and_direction);
    run!(each_gate_says_why);
    run!(v15_arrows_polylines_and_arcs);
    run!(v15_negatives_still_pass);
    run!(long_ellipses_snap_as_drawn);
    run!(sloppy_rectangles_are_no_ellipses);
    run!(uneven_quads_keep_their_corners);
    run!(corner_roundness_rounds_polygons);
    done();
}
