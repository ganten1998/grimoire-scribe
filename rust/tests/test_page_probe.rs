// Not a port of a C test: page.rs's hit testing against the C build, as test_brush.rs checks the
// brush. tests/fixtures/page_probe.c is a C program (it links ../src/page.c, brush.c, canvas.c and
// util.c, built with clang -O1 -ffp-contract=on, the Makefile's test flags) that makes strokes of
// every tool and asks stroke_hit_segment and stroke_hit about thousands of segments and points:
// random ones, ones along the stroke's own segments (the cross products in segs_intersect are
// rounding noise there, which a fused multiply-add changes) and ones from a stroke's point; it
// prints one hash of the answers and the count of hits for each stroke. fixtures/page_probe_c.txt
// is what it printed. This test builds the same strokes and queries (integer arithmetic, exact
// float conversions and double arithmetic on values that are exact there, so the inputs are the
// same) and compares line by line.
//
// To record the fixture again (the machine's libm decides powf in brush_dab, so the answers are
// the recording machine's: Apple's on the owner's Mac):
//   clang -O1 -ffp-contract=on -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -DGRIMOIRE_VERSION='"x"' \
//     rust/tests/fixtures/page_probe.c <a two-line i18n stub> src/page.c src/brush.c src/canvas.c src/util.c -lm
// (the stub defines i18n_get and i18n_getc to return their msgid).
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const H0: u64 = 14695981039346656037;

fn fnv_u32(mut h: u64, v: u32) -> u64 {
    for i in 0..4 {
        h ^= (v >> (8 * i)) as u8 as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn rng(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s >> 8
}

const TOOLS: [ToolKind; 10] = [
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
];
const NS: usize = 8;
const NQ: usize = 3000;

// A coordinate on a 400 px square from the base, in 1/1000 px (not exact in a float).
fn fx(r: &mut u32, base: f32) -> f32 {
    base + (rng(r) % 400001) as f32 / 1000.0
}

// a + (b - a) * t, in double, for values that make every step exact there; to float once.
fn along(a: f32, b: f32, t: f32) -> f32 {
    ((a as f64) + ((b as f64) - (a as f64)) * (t as f64)) as f32
}

fn page_matches_the_c_build() {
    let mut out = String::new();
    for ti in 0..TOOLS.len() {
        for si in 0..NS {
            let mut r = 0x1234u32 + ti as u32 * 131 + si as u32 * 7919;
            let mut prm = ToolParams::default();
            tool_params_default(TOOLS[ti], &mut prm);
            let mut s = stroke_new(TOOLS[ti], &prm, 77 + ti as u32);
            let n = if si == 0 {
                1
            } else if si == 1 {
                2
            } else {
                1 + (rng(&mut r) % 40) as usize
            };
            let bx = (rng(&mut r) % 1200) as f32;
            let by = (rng(&mut r) % 1600) as f32;
            for i in 0..n {
                let x = fx(&mut r, bx);
                let y = fx(&mut r, by);
                let pressure = (rng(&mut r) % 1001) as f32 / 1000.0;
                let tilt_x = ((rng(&mut r) % 141) as i32 - 70) as f32;
                let tilt_y = ((rng(&mut r) % 141) as i32 - 70) as f32;
                stroke_append(&mut s, &PenSample { x, y, pressure, tilt_x, tilt_y, t_ms: i as u32 * 7 }, 300.0);
            }
            let mut h = H0;
            let mut hits = 0;
            for qi in 0..NQ {
                let mut rad = (rng(&mut r) % 20001) as f32 / 1000.0;
                let j = (rng(&mut r) % n as u32) as usize;
                let a = s.pts[j];
                let b = s.pts[if j + 1 < n { j + 1 } else { j }];
                let (x0, y0, x1, y1);
                match qi % 4 {
                    0 => {
                        // anywhere near
                        x0 = fx(&mut r, bx);
                        y0 = fx(&mut r, by);
                        x1 = fx(&mut r, bx);
                        y1 = fx(&mut r, by);
                    }
                    1 => {
                        // along the stroke's own segment, past both ends
                        let t0 = ((rng(&mut r) % 3073) as i32 - 1024) as f32 / 1024.0;
                        let t1 = ((rng(&mut r) % 3073) as i32 - 1024) as f32 / 1024.0;
                        x0 = along(a.x, b.x, t0);
                        y0 = along(a.y, b.y, t0);
                        x1 = along(a.x, b.x, t1);
                        y1 = along(a.y, b.y, t1);
                        rad = (rng(&mut r) % 3) as f32 * 0.25;
                    }
                    2 => {
                        // from a point of the stroke
                        x0 = a.x;
                        y0 = a.y;
                        x1 = fx(&mut r, bx);
                        y1 = fx(&mut r, by);
                        rad = 0.0;
                    }
                    _ => {
                        // a point (stroke_hit)
                        x0 = fx(&mut r, bx);
                        x1 = x0;
                        y0 = fx(&mut r, by);
                        y1 = y0;
                    }
                }
                let hit = if qi % 4 == 3 { stroke_hit(&s, x0, y0, rad, 300.0) } else { stroke_hit_segment(&s, x0, y0, x1, y1, rad, 300.0) };
                h = fnv_u32(h, hit as u32);
                hits += hit as i32;
            }
            out.push_str(&format!("hit t{} s{} {:016x} {}\n", ti, si, h, hits));
            stroke_free(s);
        }
    }
    if !cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        eprintln!("  page probe: not the recording host (arm64 macOS): not compared");
        return;
    }
    let want = include_str!("fixtures/page_probe_c.txt");
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

#[test]
fn test_page_probe() {
    run!(page_matches_the_c_build);
    done();
}
