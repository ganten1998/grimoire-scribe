// test_brush.rs: the brush's byte-exactness against the C build. Every page render replays brush_add,
// so its pixels must match the C's to the byte (PORTING.md, Floating point), and the C tests that
// check it (test_golden, test_ink, test_erase, test_pagesize) belong to later waves. This port of
// the C brush was checked meanwhile against the C itself: tests/fixtures/brush_probe.c is a C
// program (it links ../src/brush.c, canvas.c and util.c, built with clang -O1 -ffp-contract=on, the
// Makefile's test flags) that renders 528 strokes (every tool, three parameter sets, two densities,
// ink / erase / erase over paper with and without the span fast path / dry / clipped) and prints
// one hash of each result; fixtures/brush_probe_c.txt is what it printed. This test builds the same
// scenarios (integer arithmetic and exact float conversions only, so the inputs are the same) and
// compares line by line.
//
// To record the fixture again (the machine's libm decides sinf and powf, so the hashes are the
// recording machine's: Apple's on the owner's Mac):
//   clang -O1 -ffp-contract=on -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -DGRIMOIRE_VERSION='"x"' \
//     rust/tests/fixtures/brush_probe.c <a two-line i18n stub> src/brush.c src/canvas.c src/util.c -lm
// (the stub defines i18n_get and i18n_getc to return their msgid).
// (the C tests index by position, and so do these ports of them)
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]
mod testlib;
use grimoire::prelude::*;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use testlib::*;

const H0: u64 = 14695981039346656037;

fn fnv(h: u64, b: &[u8]) -> u64 {
    b.iter().fold(h, |h, &x| (h ^ x as u64).wrapping_mul(1099511628211))
}
fn fnv_u32(h: u64, v: u32) -> u64 {
    fnv(h, &v.to_le_bytes())
}
fn fnv_f(h: u64, f: f32) -> u64 {
    fnv_u32(h, f.to_bits())
}

fn rng(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s >> 8
}

const N: usize = 48;
const CW: i32 = 320;
const CH: i32 = 240;

fn make_stroke(n: usize, seed: u32) -> Vec<PenSample> {
    let mut r = seed;
    let mut xi = 600 + (rng(&mut r) % 1200) as i32;
    let mut yi = 400 + (rng(&mut r) % 900) as i32;
    let mut out = Vec::new();
    for i in 0..n {
        xi += (rng(&mut r) % 161) as i32 - 60;
        yi += (rng(&mut r) % 161) as i32 - 80;
        if xi < 160 {
            xi = 160 + (rng(&mut r) % 40) as i32;
        }
        if xi > 2400 {
            xi = 2400 - (rng(&mut r) % 40) as i32;
        }
        if yi < 160 {
            yi = 160 + (rng(&mut r) % 40) as i32;
        }
        if yi > 1760 {
            yi = 1760 - (rng(&mut r) % 40) as i32;
        }
        let x = xi as f32 / 8.0;
        let y = yi as f32 / 8.0;
        let pressure = (rng(&mut r) % 1001) as f32 / 1000.0;
        let tilt_x = ((rng(&mut r) % 141) as i32 - 70) as f32;
        let tilt_y = ((rng(&mut r) % 141) as i32 - 70) as f32;
        out.push(PenSample { x, y, pressure, tilt_x, tilt_y, t_ms: 1000 + i as u32 * 7 });
    }
    out
}

fn variant(t: ToolKind, var: i32) -> ToolParams {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    if var == 1 {
        p.grain = 0.9;
        p.tilt_size = 2.0;
        p.tilt_opacity = 0.5;
        p.smoothing = 0.0;
    } else if var == 2 {
        p.pressure_gamma = 0.6;
        p.pressure_floor = 0.1;
        p.pressure_ceiling = 0.8;
        p.size_max_mm *= 3.0;
        p.grain = 0.4;
    }
    p
}

const TOOLS: [ToolKind; 11] = [
    TOOL_PENCIL,
    TOOL_MECH_PENCIL,
    TOOL_BALLPOINT,
    TOOL_ERASER,
    TOOL_HIGHLIGHTER,
    TOOL_FOUNTAIN,
    TOOL_FINELINER,
    TOOL_MARKER,
    TOOL_CHARCOAL,
    TOOL_CALLIGRAPHY,
    ToolKind(12),
];

fn gradient(px: &mut [u8]) {
    for y in 0..CH {
        for x in 0..CW {
            px[(y * CW + x) as usize] = ((x * 3 + y * 5 + ((x * y) >> 5)) & 255) as u8;
        }
    }
}

fn make_paper() -> Vec<u8> {
    let mut paper = vec![0u8; (CW * CH) as usize];
    for y in 0..CH {
        for x in 0..CW {
            paper[(y * CW + x) as usize] = (if y % 17 == 0 { 90 } else { 235 + (x + y) % 21 }) as u8;
        }
    }
    paper
}

fn scenario(out: &mut String, ti: usize, var: i32, dpi: f32, mode: i32, rep: u32) {
    let t = TOOLS[ti];
    let p = variant(t, var);
    let st = make_stroke(N, (77 + (ti as i32 * 31 + var * 7 + mode) as u32).wrapping_add(rep.wrapping_mul(1000003)));
    let mut c = Canvas::default();
    if mode == 5 {
        c = Canvas { px: Vec::new(), w: CW, h: CH, stride: CW, clip: Rect::new(0, 0, CW, CH) };
    } else {
        check!(canvas_init(&mut c, CW, CH) == 0);
        if mode != 0 {
            gradient(&mut c.px);
        }
        if mode == 6 || mode == 7 {
            c.clip = Rect::new(40, 30, 200, 150);
        }
    }
    let paper: Rc<Vec<u8>> = Rc::new(make_paper());
    let mut b = BrushState::default();
    brush_begin(&mut b, t, &p, 0xC0FFEE + ti as u32 + rep, dpi);
    if mode == 2 {
        brush_set_erase(&mut b, None, 0, 0, 0);
    }
    if mode == 3 || mode == 4 || mode == 7 {
        brush_set_erase(&mut b, Some(paper.clone()), 0, 0, CW);
    }
    if mode == 5 {
        b.dry = true;
    }
    BRUSH_ERASE_SPANS.store(mode != 4, Ordering::Relaxed);
    let mut u = RECT_EMPTY;
    let mut sm = Smoother::default();
    smoother_reset(&mut sm);
    for i in 0..N {
        let mut s = st[i];
        if var == 2 {
            smoother_apply(&mut sm, p.smoothing + 0.5, &mut s);
        }
        u = rect_union(u, brush_add(&mut b, &mut c, &s));
    }
    BRUSH_ERASE_SPANS.store(true, Ordering::Relaxed);
    let mut h = H0;
    if !c.px.is_empty() {
        h = fnv(h, &c.px);
    }
    h = fnv_u32(h, u.x as u32);
    h = fnv_u32(h, u.y as u32);
    h = fnv_u32(h, u.w as u32);
    h = fnv_u32(h, u.h as u32);
    h = fnv_u32(h, b.restored as u32);
    h = fnv_f(h, b.carry);
    h = fnv_u32(h, b.dab_index);
    h = fnv_f(h, b.nib_c);
    h = fnv_f(h, b.nib_s);
    if rep != 0 {
        out.push_str(&format!("stroke t{} v{} dpi{} m{} r{} {:016x}\n", t.0, var, dpi as i32, mode, rep, h));
    } else {
        out.push_str(&format!("stroke t{} v{} dpi{} m{} {:016x}\n", t.0, var, dpi as i32, mode, h));
    }
}

fn pure(out: &mut String) {
    for ti in 0..TOOLS.len() {
        for var in 0..3 {
            let t = TOOLS[ti];
            let p = variant(t, var);
            let mut h = H0;
            for i in 0..=20 {
                h = fnv_f(h, brush_pressure(&p, i as f32 / 20.0));
            }
            h = fnv_f(h, brush_pressure(&p, f32::NAN));
            h = fnv_f(h, brush_pressure(&p, -3.0));
            h = fnv_f(h, brush_pressure(&p, 7.0));
            let mut tx = -90;
            while tx <= 90 {
                let mut ty = -90;
                while ty <= 90 {
                    h = fnv_f(h, brush_tilt(&p, tx as f32, ty as f32));
                    h = fnv_f(h, brush_nib_angle(&p, tx as f32, ty as f32));
                    ty += 15;
                }
                tx += 15;
            }
            h = fnv_f(h, brush_tilt(&p, f32::NAN, 5.0));
            h = fnv_f(h, brush_nib_angle(&p, 0.0, 0.0));
            let st = make_stroke(N, 900 + (ti as i32 * 3 + var) as u32);
            for i in 0..N {
                let (mut r, mut op) = (0.0f32, 0.0f32);
                brush_dab(t, &p, &st[i], 226.0 + i as f32, &mut r, &mut op);
                h = fnv_f(h, r);
                h = fnv_f(h, op);
            }
            let mut d = 100;
            while d <= 600 {
                h = fnv_f(h, brush_max_radius(t, &p, d as f32));
                d += 125;
            }
            out.push_str(&format!("pure t{} v{} {:016x}\n", t.0, var, h));
        }
    }
    // the param table, the defaults, the names and the tips
    let mut h = H0;
    let tab = tool_param_table();
    h = fnv_u32(h, tab.len() as u32);
    for pi in tab {
        h = fnv(h, pi.key.as_bytes());
        h = fnv(h, pi.label.as_bytes());
        h = fnv(h, pi.unit.as_bytes());
        h = fnv_u32(h, pi.field as u32);
        h = fnv_f(h, pi.min);
        h = fnv_f(h, pi.max);
        h = fnv_f(h, pi.step);
        h = fnv_u32(h, pi.eraser_uses as u32);
    }
    for t in TOOLS {
        let mut p = ToolParams::default();
        tool_params_default(t, &mut p);
        for i in 0..ToolParams::COUNT {
            h = fnv_f(h, p.get(i));
        }
        h = fnv(h, tool_name(t).as_bytes());
        h = fnv_u32(h, brush_tip_current(t) as u32);
        for v in 0..3u8 {
            h = fnv_u32(h, brush_tip_known(t, v) as u32);
        }
    }
    for i in 0..TOOL_COUNT {
        h = fnv_u32(h, TOOL_ORDER[i].0 as u32);
    }
    out.push_str(&format!("table {:016x}\n", h));
}

fn smoothing(out: &mut String) {
    for a in 0..5 {
        let amount = a as f32 * 0.25;
        let mut sm = Smoother::default();
        smoother_reset(&mut sm);
        let st = make_stroke(N, 5150 + a as u32);
        let mut h = H0;
        for i in 0..N {
            let mut s = st[i];
            smoother_apply(&mut sm, amount, &mut s);
            h = fnv_f(h, s.x);
            h = fnv_f(h, s.y);
            h = fnv_f(h, s.pressure);
        }
        out.push_str(&format!("smooth a{} {:016x}\n", a, h));
    }
}

// a corrupt jump is not bridged; a stroke of one sample is one dab; times running backwards
fn odd(out: &mut String) {
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, CW, CH) == 0);
    let mut p = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut p);
    let mut b = BrushState::default();
    brush_begin(&mut b, TOOL_PENCIL, &p, 5, 300.0);
    let s = PenSample { x: 100.0, y: 100.0, pressure: 0.7, tilt_x: 10.0, tilt_y: 20.0, t_ms: 5000 };
    brush_add(&mut b, &mut c, &s);
    let j = PenSample { x: 30000.0, y: 100.0, pressure: 0.7, tilt_x: 10.0, tilt_y: 20.0, t_ms: 100 };
    let d = brush_add(&mut b, &mut c, &j);
    let k = PenSample { x: 150.0, y: 120.0, pressure: 0.2, tilt_x: 0.0, tilt_y: 0.0, t_ms: 20 };
    let e = brush_add(&mut b, &mut c, &k);
    let mut h = fnv(H0, &c.px);
    h = fnv_u32(h, d.w as u32);
    h = fnv_u32(h, e.x as u32);
    h = fnv_u32(h, e.w as u32);
    h = fnv_f(h, b.carry);
    h = fnv_u32(h, b.prev.t_ms);
    out.push_str(&format!("odd {:016x}\n", h));
}

// The deterministic renderer, hashed scenario by scenario against what the C build drew.
fn brush_matches_the_c_build() {
    let mut out = String::new();
    pure(&mut out);
    smoothing(&mut out);
    odd(&mut out);
    let dpis = [300.0f32, 226.0];
    // (BRUSH_PROBE_REPS=n and BRUSH_PROBE_FIXTURE=<file the C probe printed with -DREPS=n>: a
    // wider sweep than the committed fixture, to look for a float operation the fixture's few
    // strokes do not reach)
    let reps: u32 = std::env::var("BRUSH_PROBE_REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    for rep in 0..reps {
        for ti in 0..TOOLS.len() {
            for var in 0..3 {
                for di in 0..2 {
                    for mode in 0..8 {
                        scenario(&mut out, ti, var, dpis[di], mode, rep);
                    }
                }
            }
        }
    }
    let wide = std::env::var("BRUSH_PROBE_FIXTURE").ok().map(|f| std::fs::read_to_string(f).expect("BRUSH_PROBE_FIXTURE"));
    let want: &str = wide.as_deref().unwrap_or(include_str!("fixtures/brush_probe_c.txt"));
    let (got_lines, want_lines): (Vec<&str>, Vec<&str>) = (out.lines().collect(), want.lines().collect());
    check!(got_lines.len() == want_lines.len());
    let mut bad = 0;
    for (g, w) in got_lines.iter().zip(want_lines.iter()) {
        check!(g == w);
        if g != w {
            bad += 1;
            if bad <= 12 {
                eprintln!("  rust {g}\n  c    {w}");
            }
        }
    }
}

// Pen feel's table: field i is the ToolParams field named by the key (the C's offsetof).
fn the_param_table_names_its_fields() {
    let tab = tool_param_table();
    check!(tab.len() == ToolParams::COUNT);
    for (i, pi) in tab.iter().enumerate() {
        let mut p = ToolParams::default();
        *tool_param_ptr(&mut p, pi) = 7.0 + i as f32;
        let named = match pi.key {
            "size_min_mm" => p.size_min_mm,
            "size_max_mm" => p.size_max_mm,
            "opacity_min" => p.opacity_min,
            "opacity_max" => p.opacity_max,
            "pressure_floor" => p.pressure_floor,
            "pressure_ceiling" => p.pressure_ceiling,
            "pressure_gamma" => p.pressure_gamma,
            "tilt_min_deg" => p.tilt_min_deg,
            "tilt_max_deg" => p.tilt_max_deg,
            "tilt_size" => p.tilt_size,
            "tilt_opacity" => p.tilt_opacity,
            "grain" => p.grain,
            "smoothing" => p.smoothing,
            _ => -1.0,
        };
        check!(named == 7.0 + i as f32);
        check!(p.get(pi.field) == 7.0 + i as f32);
    }
}

#[test]
fn test_brush() {
    run!(brush_matches_the_c_build);
    run!(the_param_table_names_its_fields);
    done();
}
