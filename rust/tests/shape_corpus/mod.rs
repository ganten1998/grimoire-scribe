// shape_corpus: synthetic strokes for test_shape, made deterministically (a seeded PRNG), in page
// mm at the Scribe's 300 dpi and zoom 1. Paths are built in mm as polylines, sampled evenly at the
// pen's spacing (PEN_STEP_MM along the path, a sample every PEN_STEP_MM / PEN_SPEED_MM_S at a
// steady speed), then given a hand's faults: a wobble (two sines whose wavelengths are true
// lengths along the path, an amplitude the caller picks from the shape's size), per-sample jitter,
// a pressure ramp at each end and a tilt that drifts a little. Closed shapes start anywhere on
// their outline, go either way and stop short of or past their start. A stroke the hold
// evaluates ends in a rest: the nib jittering in place (with_hold) or wandering within the
// trigger's 1 mm (with_wander); held picks one at random.
//
// For scale, the owner's own strokes (assets/eraser-sample.qpg, written slowly) have samples
// about 0.07 mm apart and a second-difference jitter of about 0.018 mm (median; 0.029 mm at the
// 90th percentile): Noise::Steady's 0.03 mm is already a little rougher than that pen,
// Noise::Rough's 0.08 mm several times rougher, Noise::Slow puts that jitter on samples 0.1 mm
// apart, and Noise::Tremor gives the rough hand's wobble a tremor's short waves.
#![allow(dead_code)]

use grimoire::prelude::*;
use std::f64::consts::PI;

pub const DPI: f32 = 300.0;
/// The pen's sample spacing along the path (mm) and the speed that spacing is drawn at.
pub const PEN_STEP_MM: f64 = 0.25;
pub const PEN_SPEED_MM_S: f64 = 80.0;

/// Page px per mm at DPI.
pub fn px(mm: f64) -> f64 {
    mm * DPI as f64 / 25.4
}

/// splitmix64: small, fast, the same everywhere.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// [0, 1)
    pub fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
    pub fn int(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn coin(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// a standard normal (Box-Muller)
    pub fn gauss(&mut self) -> f64 {
        let u = self.f().max(1e-12);
        let v = self.f();
        (-2.0 * u.ln()).sqrt() * (2.0 * PI * v).cos()
    }
}

/// How unsteady the hand is.
#[derive(Clone, Copy)]
pub struct Hand {
    /// the slow wobble's amplitude (mm)
    pub wobble: f64,
    /// per-sample jitter (mm, one sigma)
    pub jitter: f64,
    /// corners rounded over about this many mm
    pub round: f64,
    /// the wobble's wavelengths are from this to twice this (mm), lengths along the path
    pub wave: f64,
    /// the samples' spacing along the path (mm): PEN_STEP_MM, or less for a slow pen
    pub step: f64,
}

pub const CLEAN: Hand = Hand { wobble: 0.0, jitter: 0.0, round: 0.0, wave: 20.0, step: PEN_STEP_MM };

pub type Pt = (f64, f64);

/// The polyline's points every `step` mm (the vertices kept).
pub fn densify(path: &[Pt], step: f64) -> Vec<Pt> {
    let mut out = vec![path[0]];
    for w in path.windows(2) {
        let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let l = (dx * dx + dy * dy).sqrt();
        let k = ((l / step).ceil() as usize).max(1);
        for s in 1..=k {
            let t = s as f64 / k as f64;
            out.push((w[0].0 + dx * t, w[0].1 + dy * t));
        }
    }
    out
}

/// A moving average over `half` samples each side, its window narrowed near the ends so it stays
/// centred: the ends stay where they are (a one-sided window would pull them in, shortening every
/// line by about the corner rounding).
fn smooth(p: &[Pt], half: usize) -> Vec<Pt> {
    if half == 0 || p.len() < 3 {
        return p.to_vec();
    }
    let n = p.len();
    (0..n)
        .map(|i| {
            let h = half.min(i).min(n - 1 - i);
            let (a, b) = (i - h, i + h);
            let mut s = (0.0, 0.0);
            for q in &p[a..=b] {
                s.0 += q.0;
                s.1 += q.1;
            }
            let k = (b - a + 1) as f64;
            (s.0 / k, s.1 / k)
        })
        .collect()
}

/// The polyline sampled every `step` mm along its length (its first and last points kept; the
/// vertices between are cut across, as a pen sampling at a steady rate cuts them).
pub fn even(path: &[Pt], step: f64) -> Vec<Pt> {
    let mut out = vec![path[0]];
    let mut prev = path[0];
    let mut acc = 0.0;
    for &cur in &path[1..] {
        let mut d = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
        while d > 0.0 && acc + d >= step {
            let t = (step - acc) / d;
            let np = (prev.0 + (cur.0 - prev.0) * t, prev.1 + (cur.1 - prev.1) * t);
            out.push(np);
            prev = np;
            d = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
            acc = 0.0;
        }
        acc += d;
        prev = cur;
    }
    let last = path[path.len() - 1];
    if out[out.len() - 1] != last {
        out.push(last);
    }
    out
}

/// The path as pen samples (page px), with the hand's faults. The wobble's phase and the samples'
/// times follow the true length along the path.
pub fn pen(path: &[Pt], hand: Hand, rng: &mut Rng) -> Vec<PenSample> {
    let step = hand.step;
    let mut d = even(path, step);
    d = smooth(&d, (hand.round / step / 2.0).round() as usize);
    let n = d.len();
    let (l1, l2) = (rng.range(hand.wave, 2.0 * hand.wave), rng.range(hand.wave, 2.0 * hand.wave));
    let (f1, f2, f3, f4) = (rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI));
    let mut out = Vec::with_capacity(n);
    let (tx, ty) = (rng.range(15.0, 40.0), rng.range(10.0, 30.0));
    let mut s = 0.0;
    for (i, &(x, y)) in d.iter().enumerate() {
        if i > 0 {
            s += ((x - d[i - 1].0).powi(2) + (y - d[i - 1].1).powi(2)).sqrt();
        }
        let wx = hand.wobble * ((2.0 * PI * s / l1 + f1).sin() * 0.7 + (2.0 * PI * s / l2 + f2).sin() * 0.3);
        let wy = hand.wobble * ((2.0 * PI * s / l2 + f3).sin() * 0.7 + (2.0 * PI * s / l1 + f4).sin() * 0.3);
        let (jx, jy) = (hand.jitter * rng.gauss(), hand.jitter * rng.gauss());
        let f = i as f64 / (n - 1).max(1) as f64;
        let ramp = (f / 0.06).min((1.0 - f) / 0.06).min(1.0);
        let pressure = (0.15 + 0.4 * ramp + 0.02 * rng.gauss()).clamp(0.01, 1.0);
        out.push(PenSample {
            x: px(x + wx + jx) as f32,
            y: px(y + wy + jy) as f32,
            pressure: pressure as f32,
            tilt_x: (tx + 0.5 * rng.gauss()) as f32,
            tilt_y: (ty + 0.5 * rng.gauss()) as f32,
            t_ms: 1_000_000 + (s / PEN_SPEED_MM_S * 1000.0).round() as u32,
        });
    }
    out
}

/// The stroke as the hold hands it over: `n` more samples at its last point, each jittered by
/// `sigma` mm, a frame (PEN_STEP_MM at PEN_SPEED_MM_S) apart, the pressure held.
pub fn with_hold(mut s: Vec<PenSample>, n: usize, sigma: f64, rng: &mut Rng) -> Vec<PenSample> {
    let Some(&last) = s.last() else {
        return s;
    };
    let frame = (PEN_STEP_MM / PEN_SPEED_MM_S * 1000.0).max(1.0);
    for k in 1..=n {
        s.push(PenSample {
            x: last.x + px(sigma * rng.gauss()) as f32,
            y: last.y + px(sigma * rng.gauss()) as f32,
            t_ms: last.t_ms.wrapping_add((k as f64 * frame).round() as u32),
            ..last
        });
    }
    s
}

/// Turns and moves a path.
pub fn place(path: &[Pt], rot: f64, cx: f64, cy: f64) -> Vec<Pt> {
    let (c, s) = (rot.cos(), rot.sin());
    path.iter().map(|&(x, y)| (cx + x * c - y * s, cy + x * s + y * c)).collect()
}

/// A closed polygon traced from `start` (a fraction of the perimeter), forwards or backwards, for
/// (1 + extra) of the perimeter (extra < 0 leaves a gap, > 0 overshoots).
pub fn trace_polygon(v: &[Pt], start: f64, forward: bool, extra: f64) -> Vec<Pt> {
    let mut v = v.to_vec();
    if !forward {
        v.reverse();
    }
    let n = v.len();
    let lens: Vec<f64> = (0..n)
        .map(|k| {
            let (a, b) = (v[k], v[(k + 1) % n]);
            ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
        })
        .collect();
    let per: f64 = lens.iter().sum();
    let at = |s: f64| -> (Pt, usize) {
        let mut s = s.rem_euclid(per);
        for k in 0..n {
            if s <= lens[k] {
                let t = s / lens[k];
                let (a, b) = (v[k], v[(k + 1) % n]);
                return ((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), k);
            }
            s -= lens[k];
        }
        (v[0], 0)
    };
    // (sampled every 0.1 mm: a corner is missed by at most that)
    let s0 = start * per;
    let total = per * (1.0 + extra);
    let k = (total / 0.1).ceil() as usize;
    (0..=k).map(|i| at(s0 + total * i as f64 / k as f64).0).collect()
}

/// An ellipse traced from angle `start`, either way, for (1 + extra) turns.
pub fn trace_ellipse(a: f64, b: f64, start: f64, forward: bool, extra: f64) -> Vec<Pt> {
    let n = 400;
    let span = 2.0 * PI * (1.0 + extra) * if forward { 1.0 } else { -1.0 };
    (0..=n)
        .map(|k| {
            let t = start + span * k as f64 / n as f64;
            (a * t.cos(), b * t.sin())
        })
        .collect()
}

/// The kinds the positives come in, with what each must be recognized as.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pos {
    Line,
    Rectangle,
    Square,
    Quad,
    Triangle,
    Ellipse,
    Circle,
}

pub const POS_ALL: [Pos; 7] = [Pos::Line, Pos::Rectangle, Pos::Square, Pos::Quad, Pos::Triangle, Pos::Ellipse, Pos::Circle];

pub fn expected(k: Pos) -> ShapeKind {
    match k {
        Pos::Line => ShapeKind::Line,
        Pos::Rectangle => ShapeKind::Rectangle,
        Pos::Square => ShapeKind::Square,
        Pos::Quad => ShapeKind::Quad,
        Pos::Triangle => ShapeKind::Triangle,
        Pos::Ellipse => ShapeKind::Ellipse,
        Pos::Circle => ShapeKind::Circle,
    }
}

/// How unsteady the hand of a positive is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Noise {
    /// no faults at all, no gap or overshoot
    Clean,
    /// a steady hand: a slow wobble of about 1% of the shape's small side (0.05-0.5 mm) over
    /// wavelengths of half to twice its size, 0.03 mm of jitter
    Steady,
    /// a rough one: a wobble of 0.5-1% of the shape's size (0.1-0.8 mm) over wavelengths of half
    /// to twice its size, corners rounded over up to 1.2 mm, and 0.08 mm of jitter
    Rough,
    /// the rough hand on a slow pen: its samples 0.1 mm apart, about as far as they jitter, so
    /// the raw path zigzags
    Slow,
    /// the rough hand with a tremor: the same wobble over wavelengths of a sixth to a third of
    /// the shape's size (at least 3 mm). It bends a polygon's edges to about the 0.93 straightness
    /// the plan allows them, so by the plan's own numbers some of its polygons are not
    /// straight-edged: a stress level, below the others' rate.
    Tremor,
}

/// The noisy levels the rate test sweeps.
pub const NOISE_ALL: [Noise; 4] = [Noise::Steady, Noise::Rough, Noise::Slow, Noise::Tremor];

/// A hand for a shape whose smallest feature is `minor` mm and largest `major`.
pub fn hand_for(minor: f64, major: f64, noise: Noise, rng: &mut Rng) -> Hand {
    match noise {
        Noise::Clean => CLEAN,
        Noise::Steady => Hand { wobble: (minor * rng.range(0.006, 0.014)).clamp(0.05, 0.5), jitter: 0.03, round: rng.range(0.3, 1.0), wave: major * rng.range(0.5, 1.0), step: PEN_STEP_MM },
        Noise::Rough => Hand { wobble: (major * rng.range(0.005, 0.01)).clamp(0.1, 0.8), jitter: 0.08, round: rng.range(0.3, 1.2), wave: major * rng.range(0.5, 1.0), step: PEN_STEP_MM },
        Noise::Slow => Hand { step: 0.1, ..hand_for(minor, major, Noise::Rough, rng) },
        Noise::Tremor => Hand { wave: (major * rng.range(1.0 / 6.0, 1.0 / 3.0)).max(3.0), ..hand_for(minor, major, Noise::Rough, rng) },
    }
}

/// The stroke as the hold hands it over when the nib wanders as it rests: `n` frames of a random
/// walk (0.05 mm steps) kept within `radius` mm of the last point, the pressure held. The trigger
/// fires on any rest within its 1 mm, so up to that this is a rest too.
pub fn with_wander(mut s: Vec<PenSample>, n: usize, radius: f64, rng: &mut Rng) -> Vec<PenSample> {
    let Some(&last) = s.last() else {
        return s;
    };
    let frame = (PEN_STEP_MM / PEN_SPEED_MM_S * 1000.0).max(1.0);
    let (mut dx, mut dy) = (0.0f64, 0.0f64);
    for k in 1..=n {
        dx += 0.05 * rng.gauss();
        dy += 0.05 * rng.gauss();
        let d = (dx * dx + dy * dy).sqrt();
        if d > radius {
            dx *= radius / d;
            dy *= radius / d;
        }
        s.push(PenSample {
            x: last.x + px(dx) as f32,
            y: last.y + px(dy) as f32,
            t_ms: last.t_ms.wrapping_add((k as f64 * frame).round() as u32),
            ..last
        });
    }
    s
}

/// The stroke as the hold hands it over, at random: 100-200 frames of rest, the nib jittering
/// 0.02-0.1 mm (one sigma) about its last point, or wandering within 0.3-0.9 mm of it.
pub fn held(s: Vec<PenSample>, rng: &mut Rng) -> Vec<PenSample> {
    let n = 100 + rng.int(101);
    if rng.coin() {
        let sigma = rng.range(0.02, 0.1);
        with_hold(s, n, sigma, rng)
    } else {
        let radius = rng.range(0.3, 0.9);
        with_wander(s, n, radius, rng)
    }
}

/// One positive of kind k, its size drawn from 12-80 mm (the line from 12, its gate, plus 0.2 mm;
/// a rectangle from 4.5 mm thick, its gate 4), any turn, any start, either way, and unless clean
/// a gap of up to 5% of the perimeter or an overshoot of up to 12%.
pub fn positive(k: Pos, rng: &mut Rng, noise: Noise) -> Vec<PenSample> {
    let hand = |minor: f64, major: f64, rng: &mut Rng| hand_for(minor, major, noise, rng);
    positive_with(k, rng, noise != Noise::Clean, &hand)
}

/// positive() with the hand made by `hand` from the shape's (minor, major) sizes.
pub fn positive_with(k: Pos, rng: &mut Rng, gaps: bool, hand: &dyn Fn(f64, f64, &mut Rng) -> Hand) -> Vec<PenSample> {
    let rot = rng.range(0.0, 2.0 * PI);
    let (cx, cy) = (rng.range(60.0, 90.0), rng.range(60.0, 140.0));
    let start = rng.f();
    let fwd = rng.coin();
    let extra = if gaps { rng.range(-0.05, 0.12) } else { 0.0 };
    let (path, minor, major) = match k {
        Pos::Line => {
            let l = rng.range(12.2, 80.0);
            (place(&[(-l / 2.0, 0.0), (l / 2.0, 0.0)], rot, cx, cy), l, l)
        }
        Pos::Rectangle => {
            let w = rng.range(14.0, 80.0);
            let h = (w * rng.range(0.3, 0.78)).max(4.5);
            let v = [(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)];
            (place(&trace_polygon(&v, start, fwd, extra), rot, cx, cy), h, w)
        }
        Pos::Square => {
            let s = rng.range(12.0, 80.0);
            let h = s / 2.0;
            let v = [(-h, -h), (h, -h), (h, h), (-h, h)];
            (place(&trace_polygon(&v, start, fwd, extra), rot, cx, cy), s, s)
        }
        Pos::Quad => {
            // a diamond whose corners are far from square (48-68 and 112-132 degrees)
            let p = rng.range(16.0, 80.0);
            let q = p * rng.range(0.45, 0.68);
            let v = [(-p / 2.0, 0.0), (0.0, -q / 2.0), (p / 2.0, 0.0), (0.0, q / 2.0)];
            (place(&trace_polygon(&v, start, fwd, extra), rot, cx, cy), q, p)
        }
        Pos::Triangle => {
            let s = rng.range(16.0, 80.0);
            let v: Vec<Pt> = match rng.int(3) {
                0 => (0..3).map(|i| {
                    let t = 2.0 * PI * i as f64 / 3.0;
                    (s / 3f64.sqrt() * t.cos(), s / 3f64.sqrt() * t.sin())
                })
                .collect(),
                1 => {
                    let h = s * rng.range(0.5, 1.0);
                    vec![(0.0, 0.0), (s, 0.0), (0.0, h)]
                }
                _ => {
                    // scalene: angles about 45, 65 and 70 degrees
                    let b = s;
                    let (a1, a2) = (45f64.to_radians(), 65f64.to_radians());
                    let apex_x = b * a2.tan() / (a1.tan() + a2.tan());
                    vec![(0.0, 0.0), (b, 0.0), (apex_x, -apex_x * a1.tan())]
                }
            };
            let minor = s * 0.5;
            (place(&trace_polygon(&v, start, fwd, extra), rot, cx, cy), minor, s)
        }
        Pos::Ellipse => {
            let a = rng.range(14.0, 80.0) / 2.0;
            let b = (a * rng.range(0.35, 0.75)).max(3.5);
            (place(&trace_ellipse(a, b, start * 2.0 * PI, fwd, extra), rot, cx, cy), b * 2.0, a * 2.0)
        }
        Pos::Circle => {
            let r = rng.range(12.0, 80.0) / 2.0;
            (place(&trace_ellipse(r, r, start * 2.0 * PI, fwd, extra), rot, cx, cy), r * 2.0, r * 2.0)
        }
    };
    let h = hand(minor, major, rng);
    pen(&path, h, rng)
}

// ---- negatives: writing at written size, scribbles, small shapes ----

/// A loop glyph (e, l): the prolate cycloid x = c t - b sin t, y = -k b (1 - cos t), one turn.
fn loop_glyph(x0: f64, w: f64, h: f64) -> Vec<Pt> {
    let c = w / (2.0 * PI);
    let b = w * 0.32;
    let k = h / (2.0 * b);
    (0..=40)
        .map(|i| {
            let t = 2.0 * PI * i as f64 / 40.0;
            (x0 + c * t - b * t.sin(), -k * b * (1.0 - t.cos()))
        })
        .collect()
}

/// An o (or a, with its down stroke) and the joining tail.
fn o_glyph(x0: f64, h: f64, with_stem: bool) -> Vec<Pt> {
    let r = h / 2.0;
    let mut v: Vec<Pt> = (0..=36)
        .map(|i| {
            let t = PI / 2.0 - 2.0 * PI * i as f64 / 36.0;
            (x0 + r + r * 0.8 * t.cos(), -r - r * t.sin())
        })
        .collect();
    v.reverse();
    if with_stem {
        v.push((x0 + 1.8 * r, -h));
        v.push((x0 + 1.8 * r, 0.0));
    }
    v.push((x0 + 2.2 * r, -0.3 * h));
    v
}

/// Humps (n, m, u).
fn hump_glyph(x0: f64, w: f64, h: f64, humps: usize, up: bool) -> Vec<Pt> {
    let mut v = Vec::new();
    for k in 0..humps {
        for i in 0..=12 {
            let t = PI * i as f64 / 12.0;
            let y = if up { -h * t.sin() } else { -h * (1.0 - t.sin()) };
            v.push((x0 + w * (k as f64 + i as f64 / 12.0), y));
        }
    }
    v
}

/// A cursive word: 3-8 letters of height h (3-7 mm), joined, as one stroke.
pub fn cursive_word(rng: &mut Rng) -> Vec<PenSample> {
    let h = rng.range(3.0, 7.0);
    let n = 3 + rng.int(6);
    let mut path: Vec<Pt> = vec![(0.0, 0.0)];
    let mut x = 0.0;
    for _ in 0..n {
        let g = match rng.int(6) {
            0 => {
                let w = h * rng.range(0.6, 0.9);
                let v = loop_glyph(x, w, h * 0.6);
                x += w;
                v
            }
            1 => {
                let w = h * rng.range(0.4, 0.6);
                let v = loop_glyph(x, w, h * 1.6);
                x += w;
                v
            }
            2 => {
                let v = o_glyph(x, h * 0.6, false);
                x += h * 0.7;
                v
            }
            3 => {
                let v = o_glyph(x, h * 0.6, true);
                x += h * 0.7;
                v
            }
            4 => {
                let w = h * 0.45;
                let v = hump_glyph(x, w, h * 0.6, 1 + rng.int(2), true);
                x += w * 2.0;
                v
            }
            _ => {
                let w = h * 0.45;
                let v = hump_glyph(x, w, h * 0.6, 2, false);
                x += w * 2.0;
                v
            }
        };
        path.extend(g);
    }
    let rot = rng.range(-0.08, 0.08);
    let p = place(&path, rot, 30.0, 80.0);
    pen(&p, Hand { wobble: 0.05, jitter: 0.03, round: 0.2, wave: 20.0, step: PEN_STEP_MM }, rng)
}

/// A single letter at written size (3-8 mm): e, l, o with its tail, a, and the like.
pub fn letter(rng: &mut Rng) -> Vec<PenSample> {
    let h = rng.range(3.0, 8.0);
    let path = match rng.int(5) {
        0 => loop_glyph(0.0, h * 0.8, h * 0.7),
        1 => loop_glyph(0.0, h * 0.45, h * 1.5),
        2 => o_glyph(0.0, h * 0.8, false),
        3 => o_glyph(0.0, h * 0.8, true),
        _ => hump_glyph(0.0, h * 0.5, h * 0.7, 2, true),
    };
    let p = place(&path, rng.range(-0.15, 0.15), 50.0, 50.0);
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.2, wave: 20.0, step: PEN_STEP_MM }, rng)
}

/// A digit, 0-9, at written size (height 4-9 mm), as one stroke the way they are written.
pub fn digit(d: usize, rng: &mut Rng) -> Vec<PenSample> {
    let h = rng.range(4.0, 9.0);
    let w = h * rng.range(0.5, 0.65);
    let arc = |cx: f64, cy: f64, rx: f64, ry: f64, a0: f64, a1: f64, n: usize| -> Vec<Pt> {
        (0..=n)
            .map(|i| {
                let t = a0 + (a1 - a0) * i as f64 / n as f64;
                (cx + rx * t.cos(), cy + ry * t.sin())
            })
            .collect()
    };
    let path: Vec<Pt> = match d {
        // 0: an ellipse, started at the top, with a little overshoot
        0 => arc(0.0, -h / 2.0, w / 2.0, h / 2.0, -PI / 2.0, -PI / 2.0 - 2.1 * PI, 60),
        1 => vec![(-w * 0.3, -h * 0.75), (0.0, -h), (0.0, 0.0)],
        2 => {
            let mut v = arc(0.0, -h * 0.72, w / 2.0, h * 0.28, PI, 2.3 * PI, 30);
            v.push((-w / 2.0, 0.0));
            v.push((w / 2.0, 0.0));
            v
        }
        3 => {
            let mut v = arc(0.0, -h * 0.75, w / 2.0, h * 0.25, -PI * 0.9, PI * 0.5, 24);
            v.extend(arc(0.0, -h * 0.25, w / 2.0, h * 0.25, -PI * 0.5, PI * 0.9, 24));
            v
        }
        4 => vec![(w * 0.2, -h), (-w / 2.0, -h * 0.35), (w / 2.0, -h * 0.35), (w * 0.25, -h * 0.6), (w * 0.25, 0.0)],
        5 => {
            let mut v = vec![(w / 2.0, -h), (-w / 2.0, -h), (-w / 2.0, -h * 0.55)];
            v.extend(arc(0.0, -h * 0.3, w / 2.0, h * 0.3, -PI * 0.8, PI * 0.8, 30));
            v
        }
        6 => {
            let mut v = arc(w * 0.6, -h * 0.4, w * 1.1, h * 0.6, -PI * 0.7, -PI, 12);
            v.extend(arc(0.0, -h * 0.28, w / 2.0, h * 0.28, PI, 3.0 * PI, 36));
            v
        }
        7 => vec![(-w / 2.0, -h), (w / 2.0, -h), (-w * 0.1, 0.0)],
        // 8: a figure eight, one stroke (the lemniscate of Gerono, upright)
        8 => (0..=80)
            .map(|i| {
                let t = 2.0 * PI * i as f64 / 80.0 + PI / 2.0;
                (w / 2.0 * (2.0 * t).sin() * 0.9, -h / 2.0 - h / 2.0 * t.sin())
            })
            .collect(),
        _ => {
            let mut v = arc(0.0, -h * 0.72, w / 2.0, h * 0.28, 0.0, 2.0 * PI, 36);
            v.push((w / 2.0, 0.0));
            v
        }
    };
    let p = place(&path, rng.range(-0.15, 0.15), 50.0, 60.0);
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.2, wave: 20.0, step: PEN_STEP_MM }, rng)
}

/// A CJK-like stroke at written size (a character 6-10 mm): a level stroke with a hook, a
/// vertical with a hook, a turn (heng zhe), the one-stroke box, the zigzag, a sweeping pie.
pub fn cjk_stroke(rng: &mut Rng) -> Vec<PenSample> {
    let s = rng.range(6.0, 10.0);
    let path: Vec<Pt> = match rng.int(6) {
        0 => vec![(0.0, 0.0), (s * 0.95, -s * 0.03), (s * 0.85, s * 0.1)],
        1 => vec![(0.0, 0.0), (0.0, s), (-s * 0.15, s * 0.9)],
        2 => vec![(0.0, 0.0), (s * 0.8, 0.0), (s * 0.75, s * 0.9)],
        3 => vec![(0.0, 0.0), (s * 0.8, 0.0), (s * 0.78, s * 0.8), (0.05 * s, s * 0.8)],
        4 => vec![(0.0, 0.0), (s * 0.7, 0.0), (0.0, s * 0.6), (s * 0.8, s * 0.7)],
        _ => (0..=20)
            .map(|i| {
                let t = i as f64 / 20.0;
                (s * 0.5 - s * 0.6 * t * t, s * t)
            })
            .collect(),
    };
    let p = place(&path, rng.range(-0.05, 0.05), 40.0, 40.0);
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.2, wave: 20.0, step: PEN_STEP_MM }, rng)
}

/// A scribble 15-40 mm: a random walk with momentum, hatching, a round scribble of several turns,
/// or a cross-out zigzag.
pub fn scribble(rng: &mut Rng) -> Vec<PenSample> {
    let size = rng.range(15.0, 40.0);
    let path: Vec<Pt> = match rng.int(4) {
        0 => {
            let mut v = vec![(0.0, 0.0)];
            let (mut x, mut y, mut a) = (0.0f64, 0.0f64, rng.range(0.0, 2.0 * PI));
            for _ in 0..150 {
                a += rng.gauss() * 0.9;
                x = (x + a.cos() * size * 0.05).clamp(-size / 2.0, size / 2.0);
                y = (y + a.sin() * size * 0.05).clamp(-size / 2.0, size / 2.0);
                v.push((x, y));
            }
            v
        }
        1 => (0..=12)
            .map(|i| {
                let x = size * i as f64 / 12.0;
                (x, if i % 2 == 0 { 0.0 } else { size * 0.5 })
            })
            .collect(),
        2 => {
            let turns = rng.range(2.6, 5.0);
            (0..=300)
                .map(|i| {
                    let t = 2.0 * PI * turns * i as f64 / 300.0;
                    let r = size / 2.0 * (0.75 + 0.25 * (t * 0.37).sin());
                    (r * t.cos() + size * 0.05 * t, r * t.sin())
                })
                .collect()
        }
        _ => (0..=16)
            .map(|i| {
                let x = size * i as f64 / 16.0;
                (x, if i % 2 == 0 { 0.0 } else { size * 0.25 } + rng.range(-0.5, 0.5))
            })
            .collect(),
    };
    let p = place(&path, rng.range(0.0, 2.0 * PI), 80.0, 80.0);
    pen(&p, Hand { wobble: 0.1, jitter: 0.04, round: 0.3, wave: 20.0, step: PEN_STEP_MM }, rng)
}

/// Which crossing doodle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Doodle {
    /// an alpha: the nodal cubic (t^2 - 1, t (t^2 - 1)), its teardrop loop and two tails that
    /// cross at the node and fly apart
    Alpha,
    /// a fish: a tail in, once round a circle, and a tail out from where the tail came in, the
    /// two spreading apart like a fish's
    Fish,
    /// a ribbon: a tail up, round a tall loop, and down again across the first tail
    Ribbon,
}

pub const DOODLE_ALL: [Doodle; 3] = [Doodle::Alpha, Doodle::Fish, Doodle::Ribbon];

/// A stroke that crosses itself near both ends, its ends far apart, 15-40 mm: no shape, however
/// round its loop (cut at both ends it would close where it crosses).
pub fn crossing_doodle(k: Doodle, rng: &mut Rng) -> Vec<PenSample> {
    let size = rng.range(15.0, 40.0);
    let path: Vec<Pt> = match k {
        Doodle::Alpha => {
            // the tails reach t = +-tt; at s = 1 the curve is tt^2 wide and twice the larger of
            // the loop's half height (0.385) and the tails' (tt (tt^2 - 1)) tall
            let tt = rng.range(1.15, 1.45);
            let s = size / (tt * tt).max(2.0 * (tt * (tt * tt - 1.0)).max(0.385));
            (0..=240)
                .map(|i| {
                    let t = -tt + 2.0 * tt * i as f64 / 240.0;
                    (s * (t * t - 1.0), s * t * (t * t - 1.0))
                })
                .collect()
        }
        Doodle::Fish => {
            let r = size / 2.0;
            // in along a chord below the circle, then round from its lowest left point, then out
            // across the way in
            let a0 = PI * rng.range(0.6, 0.8);
            let tail = r * rng.range(0.5, 1.0);
            let p0 = (r * a0.cos(), r * a0.sin());
            let mut v = vec![(p0.0 - tail * 0.8, p0.1 + tail * 0.6)];
            for i in 0..=120 {
                let t = a0 - 2.0 * PI * i as f64 / 120.0;
                v.push((r * t.cos(), r * t.sin()));
            }
            v.push((p0.0 + tail * 1.6, p0.1 + tail * 0.9));
            v
        }
        Doodle::Ribbon => {
            let (a, b) = (size * 0.375, size * 0.5);
            let below = b + size * rng.range(0.15, 0.3);
            let leg = size * rng.range(0.5, 0.9);
            let mut v = vec![(-a * 0.8, below + leg), (0.0, below)];
            for i in 0..=120 {
                let t = PI / 3.0 - (2.0 * PI - PI / 3.0) * i as f64 / 120.0;
                v.push((a * t.cos(), b * t.sin()));
            }
            v.push((0.0, below));
            v.push((a * 0.8, below + leg));
            v
        }
    };
    let p = place(&path, rng.range(0.0, 2.0 * PI), 80.0, 80.0);
    pen(&p, Hand { wobble: 0.05, jitter: 0.03, round: 0.3, wave: 20.0, step: PEN_STEP_MM }, rng)
}

/// A shape at written size: a circle, square or triangle under 9.5 mm, a line under 11.5 mm.
pub fn small_shape(rng: &mut Rng) -> Vec<PenSample> {
    let s = rng.range(4.0, 9.5);
    let rot = rng.range(0.0, 2.0 * PI);
    let path = match rng.int(4) {
        0 => trace_ellipse(s / 2.0, s / 2.0, rng.range(0.0, 6.0), rng.coin(), 0.03),
        1 => {
            let h = s / 2.0;
            trace_polygon(&[(-h, -h), (h, -h), (h, h), (-h, h)], rng.f(), rng.coin(), 0.03)
        }
        2 => trace_polygon(&[(0.0, 0.0), (s, 0.0), (s / 2.0, -s * 0.85)], rng.f(), rng.coin(), 0.03),
        _ => {
            let l = rng.range(5.0, 11.5);
            vec![(-l / 2.0, 0.0), (l / 2.0, 0.0)]
        }
    };
    let p = place(&path, rot, 60.0, 60.0);
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.3, wave: 20.0, step: PEN_STEP_MM }, rng)
}

// ---- v1.5's kinds (ShapeParams::extras) ----

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extra {
    Arrow,
    Polyline,
    Arc,
}

pub const EXTRA_ALL: [Extra; 3] = [Extra::Arrow, Extra::Polyline, Extra::Arc];

pub fn extra_expected(k: Extra) -> ShapeKind {
    match k {
        Extra::Arrow => ShapeKind::Arrow,
        Extra::Polyline => ShapeKind::Polyline,
        Extra::Arc => ShapeKind::Arc,
    }
}

/// One positive of a v1.5 kind: an arrow (a 20-80 mm shaft, one or two barbs 22-40% of it at
/// 25-55 degrees, drawn tail, tip, barb, back to the tip, barb), an open polyline (2-4 pieces of
/// 15-40 mm turning 90 degrees or 50-150, 50-105 for two pieces), or an arc (radius 10-50 mm, sweeping 90-270 degrees).
pub fn extra_positive(k: Extra, rng: &mut Rng, noisy: bool) -> Vec<PenSample> {
    let rot = rng.range(0.0, 2.0 * PI);
    let (cx, cy) = (rng.range(60.0, 90.0), rng.range(60.0, 140.0));
    let (path, minor, major): (Vec<Pt>, f64, f64) = match k {
        Extra::Arrow => {
            let l = rng.range(20.0, 80.0);
            let tip = (l / 2.0, 0.0);
            let barb = |side: f64, rng: &mut Rng| -> Pt {
                let a = rng.range(25.0f64, 55.0).to_radians() * side;
                let bl = l * rng.range(0.22, 0.4);
                (tip.0 - bl * a.cos(), tip.1 - bl * a.sin())
            };
            let mut v = vec![(-l / 2.0, 0.0), tip, barb(1.0, rng)];
            if rng.coin() {
                v.push(tip);
                v.push(barb(-1.0, rng));
            }
            (place(&v, rot, cx, cy), l * 0.2, l)
        }
        Extra::Polyline => {
            // (drawn again until it is open: ends at least half its box's diagonal apart, or it
            // is a closed shape and rightly recognized as one)
            let v = loop {
                let n = 2 + rng.int(3);
                let mut v = vec![(0.0, 0.0)];
                let mut a = 0.0f64;
                let mut turn = if rng.coin() { 1.0 } else { -1.0 };
                for i in 0..n {
                    if i > 0 {
                        // (two pieces turning sharply back are a one-barbed arrow: kept apart)
                        let hi = if n == 2 { 105.0 } else { 150.0 };
                        let t = if rng.coin() { 90.0f64 } else { rng.range(50.0, hi) };
                        // a zigzag turns both ways, a staircase alternates
                        if rng.coin() {
                            turn = -turn;
                        }
                        a += turn * t.to_radians();
                    }
                    let l = rng.range(15.0, 40.0);
                    let last = v[v.len() - 1];
                    v.push((last.0 + l * a.cos(), last.1 + l * a.sin()));
                }
                let (x0, x1) = v.iter().fold((f64::MAX, f64::MIN), |m, q| (m.0.min(q.0), m.1.max(q.0)));
                let (y0, y1) = v.iter().fold((f64::MAX, f64::MIN), |m, q| (m.0.min(q.1), m.1.max(q.1)));
                let diag = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
                // (no point of its first fifth near one of its last fifth either)
                let d = densify(&v, 0.5);
                let k = d.len() / 5;
                let mut near = f64::MAX;
                for a in &d[..=k] {
                    for b in &d[d.len() - 1 - k..] {
                        near = near.min(((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt());
                    }
                }
                if near >= 0.4 * diag {
                    break v;
                }
            };
            (place(&v, rot, cx, cy), 15.0, 40.0)
        }
        Extra::Arc => {
            let r = rng.range(10.0, 50.0);
            let sweep = rng.range(90.0f64, 270.0).to_radians() * if rng.coin() { 1.0 } else { -1.0 };
            let a0 = rng.range(0.0, 2.0 * PI);
            let v: Vec<Pt> = (0..=200)
                .map(|i| {
                    let t = a0 + sweep * i as f64 / 200.0;
                    (r * t.cos(), r * t.sin())
                })
                .collect();
            (place(&v, rot, cx, cy), r, 2.0 * r)
        }
    };
    let hand = hand_for(minor, major, if noisy { Noise::Steady } else { Noise::Clean }, rng);
    pen(&path, hand, rng)
}
