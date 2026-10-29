// fingershape_synth: the synthetic finger shapes and slips the fingershape tests draw (the design's
// fingershape_synth, section 10.2). Everything is integer arithmetic with a fixed-seed PRNG and the
// module's own sine table, so a test's asserted scores are the same on every architecture: a float
// sin() here would round differently on another libm and move a point by a pixel.
//
// A shape is an ideal path in a 0-1000 unit box (y grows downward, as on the screen). A Draw says
// how a finger draws it: its size, turn, aspect, start point and direction, its speed profile (a
// minimum-jerk curve, so it starts and ends slowly as a hand does), its jitter and a slow wobble.
// frames() turns that into the touch frames a finger sends (80 Hz), which go through the real
// capture (fs_begin, fs_add, fs_finish), gates included.
#![allow(dead_code)]

use crate::prelude::*;

/// The Scribe's scale: 11.81 px/mm (ui_px(1181.0) at 300 dpi).
pub const PX100: i32 = 1181;

/// xorshift32: a fixed, portable sequence for every seed.
pub struct Rng(pub u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(seed.wrapping_mul(2654435761).wrapping_add(0x9e37_79b9) | 1)
    }
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// A whole number in lo..=hi.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next() % (hi - lo + 1) as u32) as i32
    }
}

fn rdiv(a: i64, b: i64) -> i64 {
    if a >= 0 {
        (a + b / 2) / b
    } else {
        -((-a + b / 2) / b)
    }
}

/// floor(sqrt(v)), in integers.
pub fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut r = 0i64;
    let mut bit = 1i64 << 62;
    let mut op = v;
    while bit > op {
        bit >>= 2;
    }
    while bit != 0 {
        if op >= r + bit {
            op -= r + bit;
            r = (r >> 1) + bit;
        } else {
            r >>= 1;
        }
        bit >>= 2;
    }
    r
}

type P = (i32, i32);

fn line(out: &mut Vec<P>, a: P, b: P) {
    // (a dense line: every 10 units)
    let len = isqrt(((b.0 - a.0) as i64).pow(2) + ((b.1 - a.1) as i64).pow(2)).max(1);
    let steps = (len / 10).max(1);
    for k in 0..=steps {
        if k == 0 && out.last() == Some(&a) {
            continue;
        }
        out.push((
            a.0 + rdiv((b.0 - a.0) as i64 * k, steps) as i32,
            a.1 + rdiv((b.1 - a.1) as i64 * k, steps) as i32,
        ));
    }
}

fn poly(out: &mut Vec<P>, v: &[P]) {
    for w in v.windows(2) {
        line(out, w[0], w[1]);
    }
}

/// An arc from angle a0 to a1 (degrees, either way; 0 is right, 90 is down), radius from r0 to r1.
fn arc(out: &mut Vec<P>, c: P, r0: i32, r1: i32, a0: i32, a1: i32) {
    let n = (a1 - a0).abs().max(1);
    let dir = if a1 >= a0 { 1 } else { -1 };
    for k in 0..=n {
        let a = a0 + dir * k;
        let r = r0 as i64 + rdiv((r1 - r0) as i64 * k as i64, n as i64);
        out.push((
            c.0 + rdiv(r * fs_cos_q14(a) as i64, 16384) as i32,
            c.1 + rdiv(r * fs_sin_q14(a) as i64, 16384) as i32,
        ));
    }
}

/// Every shape the corpus knows.
pub const SHAPES: &[&str] = &[
    "V", "caret", "lt", "gt", "check", "L", "U", "C", "seven", "N", "Z", "S", "circle", "square", "triangle",
    "zigzag", "W", "M", "alpha", "two", "spiral", "star", "e", "question", "dcircle", "tspiral",
];

/// Closed shapes: a finger may start them anywhere along the way.
pub fn closed(name: &str) -> bool {
    matches!(name, "circle" | "square" | "triangle" | "dcircle")
}

/// The ideal path of a shape, in a 0-1000 box.
pub fn shape_path(name: &str) -> Vec<P> {
    let mut o = Vec::new();
    match name {
        "V" => poly(&mut o, &[(0, 0), (500, 1000), (1000, 0)]),
        "caret" => poly(&mut o, &[(0, 1000), (500, 0), (1000, 1000)]),
        "lt" => poly(&mut o, &[(1000, 0), (0, 500), (1000, 1000)]),
        "gt" => poly(&mut o, &[(0, 0), (1000, 500), (0, 1000)]),
        "check" => poly(&mut o, &[(0, 550), (330, 1000), (1000, 0)]),
        "L" => poly(&mut o, &[(0, 0), (0, 1000), (700, 1000)]),
        "U" => {
            // (a round U: short legs into a half circle, as a finger draws one)
            line(&mut o, (0, 0), (0, 200));
            arc(&mut o, (500, 200), 500, 500, 180, 0);
            line(&mut o, (1000, 200), (1000, 0));
        }
        "C" => arc(&mut o, (500, 500), 500, 500, -45, -315),
        "seven" => poly(&mut o, &[(0, 0), (1000, 0), (350, 1000)]),
        "N" => poly(&mut o, &[(0, 1000), (0, 0), (1000, 1000), (1000, 0)]),
        "Z" => poly(&mut o, &[(0, 0), (1000, 0), (0, 1000), (1000, 1000)]),
        "S" => {
            arc(&mut o, (500, 250), 250, 250, -20, -270);
            arc(&mut o, (500, 750), 250, 250, -90, 160);
        }
        "circle" => arc(&mut o, (500, 500), 500, 500, 0, 360),
        "dcircle" => arc(&mut o, (500, 500), 500, 500, 0, 720),
        "square" => poly(&mut o, &[(0, 0), (1000, 0), (1000, 1000), (0, 1000), (0, 0)]),
        "triangle" => poly(&mut o, &[(500, 0), (1000, 1000), (0, 1000), (500, 0)]),
        "zigzag" => poly(&mut o, &[(0, 0), (1000, 250), (0, 500), (1000, 750), (0, 1000)]),
        "W" => poly(&mut o, &[(0, 0), (250, 1000), (500, 300), (750, 1000), (1000, 0)]),
        "M" => poly(&mut o, &[(0, 1000), (250, 0), (500, 700), (750, 0), (1000, 1000)]),
        "alpha" => {
            line(&mut o, (1000, 150), (600, 500));
            arc(&mut o, (350, 500), 250, 250, 0, -360);
            line(&mut o, (600, 500), (1000, 850));
        }
        "two" => {
            arc(&mut o, (500, 300), 300, 300, 180, 400);
            let last = *o.last().unwrap();
            poly(&mut o, &[last, (0, 1000), (1000, 1000)]);
        }
        "spiral" => arc(&mut o, (500, 500), 100, 500, 0, 720),
        "tspiral" => arc(&mut o, (500, 500), 375, 500, 0, 720),
        "star" => {
            let mut v = Vec::new();
            for k in 0..6 {
                let a = -90 + 144 * k;
                v.push((500 + rdiv(500 * fs_cos_q14(a) as i64, 16384) as i32, 500 + rdiv(500 * fs_sin_q14(a) as i64, 16384) as i32));
            }
            poly(&mut o, &v);
        }
        "e" => {
            line(&mut o, (100, 500), (900, 500));
            arc(&mut o, (500, 500), 400, 400, 0, -300);
        }
        "question" => {
            arc(&mut o, (500, 300), 300, 300, 180, 450);
            line(&mut o, (500, 600), (500, 1000));
        }
        _ => panic!("unknown shape {name}"),
    }
    o
}

/// The kinds of accident the gates and the bank must stand against (10.2).
pub const SLIPS: &[&str] = &["swipe", "thumbarc", "slide", "hairpin", "drift", "scrub", "scribble"];

/// A random slip path in 0-1000ish units.
pub fn slip_path(kind: &str, r: &mut Rng) -> Vec<P> {
    let mut o = Vec::new();
    match kind {
        // a swipe that hooks at its end
        "swipe" => {
            let hook = r.range(40, 120) * if r.next() & 1 == 0 { 1 } else { -1 };
            let len = r.range(120, 250);
            line(&mut o, (0, 500), (1000, 500));
            o.pop();
            arc(&mut o, (1000, 500 + len * hook.signum()), len, len, -90 * hook.signum(), -90 * hook.signum() + hook);
        }
        // a thumb arcing while it scrolls
        "thumbarc" => {
            let sweep = r.range(30, 100);
            arc(&mut o, (500, 1500), 1000, 1000, -90 - sweep / 2, -90 + sweep / 2);
        }
        // a hand that skids and turns once
        "slide" => {
            let turn = r.range(30, 170) * if r.next() & 1 == 0 { 1 } else { -1 };
            let corner = r.range(20, 80) as i64;
            let h = r.range(0, 359);
            let p1 = (rdiv(10 * corner * fs_cos_q14(h) as i64, 16384) as i32, rdiv(10 * corner * fs_sin_q14(h) as i64, 16384) as i32);
            let rest = 10 * (100 - corner);
            let p2 = (p1.0 + rdiv(rest * fs_cos_q14(h + turn) as i64, 16384) as i32, p1.1 + rdiv(rest * fs_sin_q14(h + turn) as i64, 16384) as i32);
            poly(&mut o, &[(0, 0), p1, p2]);
        }
        // out and back: a slide that turns all the way round, with a rounded tip
        "hairpin" => {
            let r0 = r.range(20, 90);
            let back = r.range(55, 100);
            let lean = r.range(-15, 15);
            line(&mut o, (0, 500), (1000, 500));
            o.pop();
            let side = if r.next() & 1 == 0 { 1 } else { -1 };
            arc(&mut o, (1000, 500 + side * r0), r0, r0, -90 * side, 90 * side + lean * side);
            let end = *o.last().unwrap();
            let h = 180 + lean * side;
            let len = 10 * back as i64;
            let tip = (end.0 + rdiv(len * fs_cos_q14(h) as i64, 16384) as i32, end.1 + rdiv(len * fs_sin_q14(h) as i64, 16384) as i32);
            line(&mut o, end, tip);
        }
        // a palm-like drift: the hand slides on, its heading wandering a little at a time
        "drift" => {
            // (headings in 1/16 degree: a bend of up to 6 degrees a step, wandering by 1)
            let mut h = r.range(0, 359) * 16;
            let mut bend = r.range(-48, 48);
            let (mut x, mut y) = (0i64, 0i64);
            o.push((0, 0));
            for _ in 0..40 {
                bend = (bend + r.range(-16, 16)).clamp(-96, 96);
                h += bend;
                x += rdiv(25 * fs_cos_q14(h / 16) as i64, 16384);
                y += rdiv(25 * fs_sin_q14(h / 16) as i64, 16384);
                let p = (x as i32, y as i32);
                let last = *o.last().unwrap();
                line(&mut o, last, p);
            }
        }
        // scrubbing back and forth
        "scrub" => {
            let n = r.range(6, 10);
            let mut v = vec![(0, 500)];
            for k in 0..n {
                v.push((if k % 2 == 0 { 1000 } else { 0 }, 500 + r.range(-60, 60)));
            }
            poly(&mut o, &v);
        }
        // a deliberate random scribble: the hard case
        "scribble" => {
            let n = r.range(5, 9);
            let mut v = Vec::new();
            for _ in 0..n {
                v.push((r.range(0, 1000), r.range(0, 1000)));
            }
            poly(&mut o, &v);
        }
        _ => panic!("unknown slip {kind}"),
    }
    o
}

/// How a finger draws one path.
#[derive(Clone, Copy, Debug)]
pub struct Draw {
    /// the larger box side on the screen, mm
    pub size_mm: i32,
    /// turned by this, degrees
    pub rot: i32,
    /// x stretched by this % (aspect)
    pub aspect: i32,
    /// drawn backward
    pub reverse: bool,
    /// a closed shape started this % of the way along
    pub shift: i32,
    /// landing to lift, ms
    pub ms: u32,
    /// frame-to-frame jitter, +- px
    pub jitter: i32,
    /// slow wobble amplitude, px
    pub wobble: i32,
    /// corners rounded over this much of the path, tenths of a mm (a finger cuts corners)
    pub round: i32,
    /// a hook at the end as the finger lifts: its length in tenths of a mm, turned by hook_turn
    /// degrees from the way the path was going
    pub hook: i32,
    pub hook_turn: i32,
    /// the box centre on the screen
    pub cx: i32,
    pub cy: i32,
    pub seed: u32,
}

impl Default for Draw {
    fn default() -> Draw {
        Draw {
            size_mm: 40,
            rot: 0,
            aspect: 100,
            reverse: false,
            shift: 0,
            ms: 900,
            jitter: 0,
            wobble: 0,
            round: 0,
            hook: 0,
            hook_turn: 0,
            cx: 900,
            cy: 1200,
            seed: 1,
        }
    }
}

fn len_of(a: P, b: P) -> i64 {
    isqrt(((b.0 - a.0) as i64).pow(2) + ((b.1 - a.1) as i64).pow(2))
}

/// What a finger does to a placed path (screen px): corners rounded by a moving average over
/// `round` of the path (the window shrinks near the ends, so the ends stay put), then an end hook.
pub fn finger(pts: &[P], d: &Draw) -> Vec<P> {
    let mut v = pts.to_vec();
    if v.len() < 2 {
        return v;
    }
    if d.round > 0 {
        let half = (d.round as i64 * PX100 as i64 / 1000 / 2).max(1);
        let mut cum = vec![0i64];
        for w in v.windows(2) {
            cum.push(cum.last().unwrap() + len_of(w[0], w[1]));
        }
        let total = *cum.last().unwrap();
        // (prefix sums, and a window [lo, hi) whose two edges only move forward: both
        // cum - h and cum + h grow along the path, h shrinking only near the ends)
        let (mut px, mut py) = (vec![0i64], vec![0i64]);
        for p in &v {
            px.push(px.last().unwrap() + p.0 as i64);
            py.push(py.last().unwrap() + p.1 as i64);
        }
        let (mut lo, mut hi) = (0usize, 0usize);
        let mut out = Vec::with_capacity(v.len());
        for i in 0..v.len() {
            let h = half.min(cum[i]).min(total - cum[i]);
            while cum[lo] < cum[i] - h {
                lo += 1;
            }
            while hi < v.len() && cum[hi] <= cum[i] + h {
                hi += 1;
            }
            let n = (hi - lo) as i64;
            out.push((rdiv(px[hi] - px[lo], n) as i32, rdiv(py[hi] - py[lo], n) as i32));
        }
        v = out;
    }
    if d.hook > 0 {
        // the way the path was going over its last 3 mm
        let end = *v.last().unwrap();
        let back = 3 * PX100 as i64 / 100;
        let mut k = v.len() - 1;
        while k > 0 && len_of(v[k], end) < back {
            k -= 1;
        }
        let (dx, dy) = ((end.0 - v[k].0) as i64, (end.1 - v[k].1) as i64);
        let l = isqrt(dx * dx + dy * dy).max(1);
        let (cs, sn) = (fs_cos_q14(d.hook_turn) as i64, fs_sin_q14(d.hook_turn) as i64);
        let (hx, hy) = (rdiv(dx * cs - dy * sn, 16384), rdiv(dx * sn + dy * cs, 16384));
        let hook_px = d.hook as i64 * PX100 as i64 / 1000;
        let tip = (end.0 + rdiv(hx * hook_px, l) as i32, end.1 + rdiv(hy * hook_px, l) as i32);
        let steps = (hook_px / 4).max(1);
        for s in 1..=steps {
            v.push((end.0 + rdiv((tip.0 - end.0) as i64 * s, steps) as i32, end.1 + rdiv((tip.1 - end.1) as i64 * s, steps) as i32));
        }
    }
    v
}

/// The path placed on the screen as Draw says (no timing): reversed, shifted, stretched, turned,
/// scaled and moved.
pub fn place(path: &[P], closed_: bool, d: &Draw) -> Vec<P> {
    let mut v: Vec<P> = path.to_vec();
    if closed_ && d.shift != 0 {
        // (a closed path's last point repeats its first: drop it, turn the ring, close it again)
        v.pop();
        let k = (v.len() as i64 * d.shift as i64 / 100) as usize % v.len().max(1);
        v.rotate_left(k);
        let first = v[0];
        v.push(first);
    }
    if d.reverse {
        v.reverse();
    }
    let (cs, sn) = (fs_cos_q14(d.rot) as i64, fs_sin_q14(d.rot) as i64);
    // 1000 units become size_mm on the screen
    let px = d.size_mm as i64 * PX100 as i64 / 100;
    v.iter()
        .map(|&(x, y)| {
            let x = (x as i64 - 500) * d.aspect as i64 / 100;
            let y = y as i64 - 500;
            let rx = rdiv(x * cs - y * sn, 16384);
            let ry = rdiv(x * sn + y * cs, 16384);
            (d.cx + rdiv(rx * px, 1000) as i32, d.cy + rdiv(ry * px, 1000) as i32)
        })
        .collect()
}

/// The minimum-jerk progress for a time fraction, both in Q16: 10t^3 - 15t^4 + 6t^5.
fn minjerk(t: i64) -> i64 {
    let t2 = (t * t) >> 16;
    let t3 = (t2 * t) >> 16;
    let t4 = (t3 * t) >> 16;
    let t5 = (t4 * t) >> 16;
    (10 * t3 - 15 * t4 + 6 * t5).clamp(0, 65536)
}

/// One touch frame: screen px and ms from the landing (t0 is added by the caller).
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub t: u32,
}

/// The frames a finger sends drawing pts in d.ms: the corners rounded and the end hooked as d
/// says (finger), then 80 Hz (12 and 13 ms apart in turn), along the path at minimum-jerk speed,
/// with jitter and a slow wobble.
pub fn frames_of(pts: &[P], d: &Draw) -> Vec<Frame> {
    let pts = &finger(pts, d)[..];
    let mut cum = vec![0i64];
    for w in pts.windows(2) {
        let l = isqrt(((w[1].0 - w[0].0) as i64).pow(2) + ((w[1].1 - w[0].1) as i64).pow(2));
        cum.push(cum.last().unwrap() + l);
    }
    let total = *cum.last().unwrap();
    let mut r = Rng::new(d.seed);
    let phase_x = r.range(0, 359);
    let phase_y = r.range(0, 359);
    let mut out = Vec::new();
    let mut k = 0u32;
    let mut seg = 0usize;
    loop {
        let t = k * 25 / 2;
        let t = t.min(d.ms);
        let tq = (t as i64 * 65536) / d.ms.max(1) as i64;
        let s = (minjerk(tq) * total) >> 16;
        while seg + 1 < cum.len() - 1 && cum[seg + 1] < s {
            seg += 1;
        }
        let (a, b) = if pts.len() > 1 { (pts[seg], pts[seg + 1]) } else { (pts[0], pts[0]) };
        let sl = (cum.get(seg + 1).copied().unwrap_or(0) - cum[seg]).max(1);
        let f = s - cum[seg];
        let mut x = a.0 + rdiv((b.0 - a.0) as i64 * f, sl) as i32;
        let mut y = a.1 + rdiv((b.1 - a.1) as i64 * f, sl) as i32;
        if d.wobble > 0 {
            let ang = ((360 * tq) >> 16) as i32;
            x += rdiv(d.wobble as i64 * fs_sin_q14(ang + phase_x) as i64, 16384) as i32;
            y += rdiv(d.wobble as i64 * fs_sin_q14(ang * 3 / 2 + phase_y) as i64, 16384) as i32;
        }
        if d.jitter > 0 {
            x += r.range(-d.jitter, d.jitter);
            y += r.range(-d.jitter, d.jitter);
        }
        out.push(Frame { x, y, t });
        if t >= d.ms {
            break;
        }
        k += 1;
    }
    out
}

/// The frames for a named shape.
pub fn shape_frames(name: &str, d: &Draw) -> Vec<Frame> {
    frames_of(&place(&shape_path(name), closed(name), d), d)
}

/// Runs frames through the real capture from t0 and lifts at the last frame: what fs_finish made
/// of it, and its sample.
pub fn capture(fr: &[Frame], t0: u32, mode: FsMode, out: &mut FsSample) -> FsLift {
    let mut s = FsStroke::default();
    capture_into(&mut s, fr, t0, mode, out)
}

pub fn capture_into(s: &mut FsStroke, fr: &[Frame], t0: u32, mode: FsMode, out: &mut FsSample) -> FsLift {
    fs_begin(s, fr[0].x, fr[0].y, t0.wrapping_add(fr[0].t), PX100);
    for f in &fr[1..] {
        fs_add(s, f.x, f.y, t0.wrapping_add(f.t), false);
    }
    fs_finish(s, t0.wrapping_add(fr[fr.len() - 1].t), mode, out)
}

/// The sample of a shape drawn exactly (no finger model): the matcher's own tests use these.
pub fn ideal(name: &str, d: &Draw) -> FsSample {
    let pts: Vec<FsPt> = place(&shape_path(name), closed(name), d).iter().map(|&(x, y)| FsPt { x, y }).collect();
    let mut s = FsSample::default();
    fs_sample_of(&pts, &mut s);
    s
}

/// A recording-like variation of a Draw: small turn, size and speed changes, light jitter.
pub fn vary_record(r: &mut Rng, seed: u32) -> Draw {
    Draw {
        size_mm: r.range(30, 60),
        rot: r.range(-3, 3),
        aspect: r.range(92, 108),
        reverse: false,
        shift: 0,
        ms: r.range(700, 1100) as u32,
        jitter: r.range(2, 4),
        wobble: r.range(0, 6),
        cx: r.range(700, 1100),
        cy: r.range(900, 1500),
        seed,
        ..Draw::default()
    }
}

/// A use-like variation, the design's harsh one (10.2 and section 5's results): any size from 20
/// to 100 mm, +-12 degrees, aspect 0.8-1.25, jitter 0.3-0.8 mm (4-9 px), wobble, either
/// direction, any start on a closed shape, corners rounded over up to 3 mm, and an end hook of up
/// to 3 mm in two draws of three.
pub fn vary_use(r: &mut Rng, seed: u32) -> Draw {
    let hooked = r.range(0, 2) > 0;
    let hook_turn = r.range(40, 120) * if r.next() & 1 == 0 { 1 } else { -1 };
    Draw {
        size_mm: r.range(20, 100),
        rot: r.range(-12, 12),
        aspect: r.range(80, 125),
        reverse: r.next() & 1 == 1,
        shift: r.range(0, 99),
        ms: r.range(600, 1400) as u32,
        jitter: r.range(4, 9),
        wobble: r.range(0, 14),
        round: r.range(0, 30),
        hook: if hooked { r.range(5, 30) } else { 0 },
        hook_turn,
        cx: r.range(700, 1100),
        cy: r.range(900, 1500),
        seed,
    }
}
