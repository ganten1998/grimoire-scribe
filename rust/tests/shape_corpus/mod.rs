// shape_corpus: synthetic strokes for test_shape, made deterministically (a seeded PRNG), in page
// mm at the Scribe's 300 dpi and zoom 1. Paths are built in mm as polylines, densified to the pen's
// spacing (about 0.25 mm, a 3 ms frame at a brisk 80 mm/s), then given a hand's faults: a slow
// wobble (two sines, an amplitude the caller picks from the shape's size), per-sample jitter, a
// pressure ramp at each end and a tilt that drifts a little. Closed shapes start anywhere on
// their outline, go either way and stop short of or past their start.
#![allow(dead_code)]

use grimoire::prelude::*;
use std::f64::consts::PI;

pub const DPI: f32 = 300.0;

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
    /// the wobble's wavelengths are from this to twice this (mm): a hand drifts over the whole
    /// shape, not in ripples
    pub wave: f64,
}

pub const CLEAN: Hand = Hand { wobble: 0.0, jitter: 0.0, round: 0.0, wave: 20.0 };

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

fn smooth(p: &[Pt], half: usize) -> Vec<Pt> {
    if half == 0 || p.len() < 3 {
        return p.to_vec();
    }
    let n = p.len();
    (0..n)
        .map(|i| {
            let a = i.saturating_sub(half);
            let b = (i + half).min(n - 1);
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

/// The path as pen samples (page px), with the hand's faults.
pub fn pen(path: &[Pt], hand: Hand, rng: &mut Rng) -> Vec<PenSample> {
    let step = 0.25;
    let mut d = densify(path, step);
    d = smooth(&d, (hand.round / step / 2.0).round() as usize);
    let n = d.len();
    let (l1, l2) = (rng.range(hand.wave, 2.0 * hand.wave), rng.range(hand.wave, 2.0 * hand.wave));
    let (f1, f2, f3, f4) = (rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI), rng.range(0.0, 2.0 * PI));
    let mut out = Vec::with_capacity(n);
    let (tx, ty) = (rng.range(15.0, 40.0), rng.range(10.0, 30.0));
    for (i, &(x, y)) in d.iter().enumerate() {
        let s = i as f64 * step;
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
            t_ms: 1_000_000 + 3 * i as u32,
        });
    }
    out
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

/// A hand for a shape whose smallest feature is `minor` mm and largest `major`: steadier for small
/// shapes, as a hand drawing small is.
pub fn hand_for(minor: f64, major: f64, rng: &mut Rng) -> Hand {
    Hand { wobble: (minor * rng.range(0.006, 0.014)).clamp(0.05, 0.5), jitter: 0.04, round: rng.range(0.3, 1.0), wave: major * rng.range(0.5, 1.0) }
}

/// One positive of kind k, its size drawn from 12-80 mm (the line from 13: its gate is 12), any
/// turn, any start, either way, a gap of up to 5% of the perimeter or an overshoot of up to 12%.
pub fn positive(k: Pos, rng: &mut Rng, noisy: bool) -> Vec<PenSample> {
    let rot = rng.range(0.0, 2.0 * PI);
    let (cx, cy) = (rng.range(60.0, 90.0), rng.range(60.0, 140.0));
    let start = rng.f();
    let fwd = rng.coin();
    let extra = if noisy { rng.range(-0.05, 0.12) } else { 0.0 };
    let (path, minor, major) = match k {
        Pos::Line => {
            let l = rng.range(13.0, 80.0);
            (place(&[(-l / 2.0, 0.0), (l / 2.0, 0.0)], rot, cx, cy), l, l)
        }
        Pos::Rectangle => {
            let w = rng.range(14.0, 80.0);
            let h = (w * rng.range(0.3, 0.78)).max(7.0);
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
    let hand = if noisy { hand_for(minor, major, rng) } else { CLEAN };
    pen(&path, hand, rng)
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
    pen(&p, Hand { wobble: 0.05, jitter: 0.03, round: 0.2, wave: 20.0 }, rng)
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
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.2, wave: 20.0 }, rng)
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
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.2, wave: 20.0 }, rng)
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
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.2, wave: 20.0 }, rng)
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
    pen(&p, Hand { wobble: 0.1, jitter: 0.04, round: 0.3, wave: 20.0 }, rng)
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
    pen(&p, Hand { wobble: 0.04, jitter: 0.03, round: 0.3, wave: 20.0 }, rng)
}
