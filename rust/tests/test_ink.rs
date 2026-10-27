// test_ink.c: canvas, paper, brush, page, history.
// (the C literals, 6.2831853f and 3.14f, are kept as they are: they are part of the recorded input)
#![allow(clippy::needless_range_loop, clippy::too_many_arguments, clippy::approx_constant)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const DPI: f32 = 300.0;
const W: i32 = 1860;
const H: i32 = 2480;
const FULL: Rect = Rect { x: 0, y: 0, w: W, h: H };

// srand and rand, the C library's: the same sequences the C test draws its random cases from.
fn srand(seed: u32) {
    // SAFETY: srand only sets the C library's generator state.
    unsafe { libc::srand(seed) }
}
fn rand() -> i32 {
    // SAFETY: rand only reads and advances the C library's generator state.
    unsafe { libc::rand() }
}

fn out_png(name: &str, c: &Canvas) {
    let dir = cformat!("%s/../out", tl_tmp().as_str());
    mkdir_p(&dir);
    let path = cformat!("%s/%s", dir.as_str(), name);
    let Ok(cpath) = std::ffi::CString::new(path) else { return };
    // SAFETY: stbi_write_png reads c.h rows of c.w bytes at c.stride from c.px, which holds them
    // (w, h and stride are the canvas's own), and writes the NUL-terminated path we own.
    unsafe {
        grimoire::ffi_stb::stbi_write_png(cpath.as_ptr(), c.w, c.h, 1, c.px.as_ptr() as *const libc::c_void, c.stride);
    }
}

fn px(c: &Canvas, x: i32, y: i32) -> u8 {
    c.px[(y * c.stride + x) as usize]
}

fn rects() {
    let a = Rect { x: 0, y: 0, w: 10, h: 10 };
    let b = Rect { x: 5, y: 5, w: 10, h: 10 };
    let u = rect_union(a, b);
    let i = rect_intersect(a, b);
    check!(u.x == 0 && u.y == 0 && u.w == 15 && u.h == 15);
    check!(i.x == 5 && i.y == 5 && i.w == 5 && i.h == 5);
    check!(rect_is_empty(rect_intersect(a, Rect { x: 20, y: 20, w: 5, h: 5 })));
    check!(rect_union(RECT_EMPTY, b).w == 10);
    check!(rect_contains(a, 9, 9) && !rect_contains(a, 10, 9));
}

fn canvas_basics() {
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 100, 80) == 0);
    canvas_set_clip(&mut c, Rect { x: 10, y: 10, w: 20, h: 20 });
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 100, h: 80 }, 0);
    check!(c.px[0] == 255 && c.px[15 * 100 + 15] == 0 && c.px[40 * 100 + 40] == 255);
    canvas_reset_clip(&mut c);
    // deposit only darkens
    c.px[50 * 100 + 50] = 100;
    canvas_deposit_px(&mut c, 50, 50, 200, 1.0);
    check!(c.px[50 * 100 + 50] == 100);
    canvas_deposit_px(&mut c, 50, 50, 0, 0.5);
    check!(c.px[50 * 100 + 50] == 50);
    // downsample averages
    let mut d = Canvas::default();
    canvas_init(&mut d, 50, 40);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 100, h: 80 }, 255);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 100, h: 80 }, 255);
    for y in 0..80 {
        for x in 0..100 {
            c.px[y * 100 + x] = if (x & 1) != 0 { 0 } else { 200 };
        }
    }
    canvas_downsample(&mut d, &c);
    check!(d.px[0] == 100);
    canvas_free(&mut d);
    canvas_free(&mut c);
}

// The original per-pixel rounded rect: the fast one must match it exactly.
fn ref_round_rect(c: &mut Canvas, r: Rect, mut radius: i32, gray: u8, filled: bool, t: i32) {
    if rect_is_empty(r) {
        return;
    }
    let maxr = (if r.w < r.h { r.w } else { r.h }) / 2;
    if radius > maxr {
        radius = maxr;
    }
    if radius < 0 {
        radius = 0;
    }
    let b = rect_intersect(r, c.clip);
    let rr = radius as f32;
    for y in b.y..b.y + b.h {
        let py = y as f32 + 0.5;
        for x in b.x..b.x + b.w {
            let pxx = x as f32 + 0.5;
            let cx = clampf(pxx, r.x as f32 + rr, (r.x + r.w) as f32 - rr);
            let cy = clampf(py, r.y as f32 + rr, (r.y + r.h) as f32 - rr);
            let dx = pxx - cx;
            let dy = py - cy;
            let d = if dx != 0.0 || dy != 0.0 {
                fmla(dx, dx, dy * dy).sqrt() - rr
            } else {
                -((pxx - r.x as f32).min((r.x + r.w) as f32 - pxx).min((py - r.y as f32).min((r.y + r.h) as f32 - py)))
            };
            let mut a = clampf(0.5 - d, 0.0, 1.0);
            if !filled {
                a = a.min(clampf(0.5 - (-(d + t as f32)), 0.0, 1.0));
            }
            if a <= 0.0 {
                continue;
            }
            let at = (y * c.stride + x) as usize;
            let p = c.px[at];
            c.px[at] = if a >= 1.0 { gray } else { (fmla((gray as i32 - p as i32) as f32, a, p as f32) + 0.5) as u8 };
        }
    }
}

fn round_rect_fast_matches_reference() {
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    canvas_init(&mut a, 400, 300);
    canvas_init(&mut b, 400, 300);
    srand(7);
    for it in 0..600 {
        let r = Rect { x: rand() % 380 - 40, y: rand() % 280 - 40, w: 1 + rand() % 300, h: 1 + rand() % 250 };
        let rad = rand() % 60;
        let t = 1 + rand() % 12;
        let filled = (rand() & 1) != 0;
        let g = (rand() % 256) as u8;
        let clip = if it % 3 != 0 {
            Rect { x: 0, y: 0, w: 400, h: 300 }
        } else {
            Rect { x: rand() % 200, y: rand() % 150, w: 50 + rand() % 200, h: 50 + rand() % 150 }
        };
        canvas_set_clip(&mut a, clip);
        canvas_set_clip(&mut b, clip);
        canvas_round_rect(&mut a, r, rad, g, filled, t);
        ref_round_rect(&mut b, r, rad, g, filled, t);
    }
    check!(a.px[..(a.stride * 300) as usize] == b.px[..(a.stride * 300) as usize]);
    canvas_free(&mut a);
    canvas_free(&mut b);
}

fn paper_subregion_matches_full() {
    let mut full = Canvas::default();
    let mut part = Canvas::default();
    canvas_init(&mut full, W, H);
    canvas_init(&mut part, W, H);
    let mix = MusicLayout { systems: 3, n: 5, part: [PART_TREBLE, PART_BASS, PART_TAB6, PART_LYRICS, PART_TAB4, PART_NONE] };
    let clefs = MusicLayout { systems: 2, n: 4, part: [PART_ALTO, PART_TENOR, PART_PERC, PART_TAB7, PART_NONE, PART_NONE] };
    for i in 0..PAPER_COUNT + 2 {
        let k = if i < PAPER_COUNT { PaperKind(i as u32) } else { music_encode(if i == PAPER_COUNT { &mix } else { &clefs }) };
        paper_render(&mut full, k, FULL, DPI);
        canvas_fill(&mut part, FULL, 77);
        let tiles = [
            Rect { x: 0, y: 0, w: 700, h: 900 },
            Rect { x: 700, y: 0, w: W - 700, h: 900 },
            Rect { x: 0, y: 900, w: 333, h: H - 900 },
            Rect { x: 333, y: 900, w: W - 333, h: H - 900 },
        ];
        for t in tiles {
            paper_render(&mut part, k, t, DPI);
        }
        check!(full.px[..(W * H) as usize] == part.px[..(W * H) as usize]);
        let name = cformat!("paper-%d.png", i);
        out_png(&name, &full);
    }
    // Pitch for medium rule: find consecutive rule rows in a column away from the margin.
    paper_render(&mut full, PAPER_MEDIUM, FULL, DPI);
    let mut rows = [0i32; 64];
    let mut n = 0usize;
    let mut y = 1;
    while y < H && n < 64 {
        if full.px[(y * W + 1000) as usize] < 230 && full.px[((y - 1) * W + 1000) as usize] >= 230 {
            rows[n] = y;
            n += 1;
        }
        y += 1;
    }
    check!(n > 20);
    if n > 3 {
        check_near!(rows[3] - rows[2], mm_to_px(7.1, DPI), 1.0);
    }
    canvas_free(&mut full);
    canvas_free(&mut part);
}

fn pressure_tilt_mapping() {
    for t in 0..TOOL_COUNT as i32 {
        let mut p = ToolParams::default();
        tool_params_default(ToolKind(t), &mut p);
        check!(brush_pressure(&p, 0.0) == 0.0);
        check_near!(brush_pressure(&p, 1.0), 1.0, 1e-6);
        let mut prev = -1.0f32;
        for i in 0..=100 {
            let v = brush_pressure(&p, i as f32 / 100.0);
            check!(v >= prev);
            prev = v;
        }
        check!(brush_tilt(&p, 0.0, 0.0) == 0.0);
        // Input caps tilt at 89 degrees; a 90-degree full-tilt setting approaches its maximum.
        let flat = brush_tilt(&p, 89.0, 0.0);
        check!(flat > 0.999 && flat <= 1.0);
        if p.tilt_max_deg <= 89.0 {
            check_near!(flat, 1.0, 1e-6);
        }
        let mut reachable = p;
        reachable.tilt_max_deg = p.tilt_max_deg.min(88.0);
        check_near!(brush_tilt(&reachable, 89.0, 0.0), 1.0, 1e-6);
        let pi = tool_param_table();
        for i in 0..pi.len() {
            let v = *tool_param_ptr(&mut p, &pi[i]);
            check!(v >= pi[i].min && v <= pi[i].max);
        }
    }
    check!(tool_param_table().len() == 13);
}

fn make_stroke(t: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, steps: i32, p0: f32, p1: f32, tilt: f32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    let mut s = stroke_new(t, &prm, 1234u32.wrapping_add(fmla(x0, 7.0, y0) as u32));
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let ps = PenSample {
            x: lerpf(x0, x1, f),
            y: fmla(25.0, (f * 9.0).sin(), lerpf(y0, y1, f)),
            pressure: lerpf(p0, p1, f),
            tilt_x: tilt,
            tilt_y: 0.0,
            t_ms: i as u32 * 5,
        };
        stroke_append(&mut s, &ps, DPI);
    }
    s
}

fn live_equals_rerender() {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    let mut live = Canvas::default();
    let mut re = Canvas::default();
    let mut chunk = Canvas::default();
    canvas_init(&mut live, W, H);
    canvas_init(&mut re, W, H);
    canvas_init(&mut chunk, W, H);
    paper_render(&mut live, pg.paper, FULL, DPI);
    for t in 0..3 {
        let s = make_stroke(ToolKind(t), 200.0, 400.0 + t as f32 * 300.0, 1600.0, 500.0 + t as f32 * 300.0, 90, 0.1, 1.0, t as f32 * 20.0);
        let mut b = BrushState::default();
        brush_begin(&mut b, s.tool, &s.params, s.seed, DPI);
        for i in 0..s.pts.len() {
            brush_add(&mut b, &mut live, &s.pts[i]);
        }
        page_add(&mut pg, s);
    }
    page_render(&mut re, &pg, FULL, DPI);
    check!(live.px[..(W * H) as usize] == re.px[..(W * H) as usize]);
    // Region re-render matches too.
    for i in 0..6 {
        page_render(&mut chunk, &pg, Rect { x: i * 310, y: 0, w: 310, h: H }, DPI);
    }
    check!(live.px[..(W * H) as usize] == chunk.px[..(W * H) as usize]);
    // Some ink was actually put down.
    let mut dark = 0;
    for i in 0..(W * H) as usize {
        dark += (live.px[i] < 100) as i32;
    }
    check!(dark > 2000);
    canvas_free(&mut live);
    canvas_free(&mut re);
    canvas_free(&mut chunk);
    page_free(pg);
}

// A straight stroke straight onto a white canvas; returns how many pixels it inked (< 128).
fn ink_line(c: &mut Canvas, t: ToolKind, prm: &ToolParams, x0: f32, y0: f32, x1: f32, y1: f32, p: f32, tx: f32, ty: f32) -> i32 {
    let mut b = BrushState::default();
    brush_begin(&mut b, t, prm, 99, DPI);
    for i in 0..=80 {
        let f = i as f32 / 80.0;
        let s = PenSample { x: lerpf(x0, x1, f), y: lerpf(y0, y1, f), pressure: p, tilt_x: tx, tilt_y: ty, t_ms: i as u32 * 5 };
        brush_add(&mut b, c, &s);
    }
    let mut n = 0;
    for i in 0..(c.w * c.h) as usize {
        n += (c.px[i] < 128) as i32;
    }
    n
}

fn ink_on_fresh(t: ToolKind, prm: &ToolParams, x0: f32, y0: f32, x1: f32, y1: f32, p: f32, tx: f32, ty: f32) -> i32 {
    let mut c = Canvas::default();
    canvas_init(&mut c, 1000, 1000);
    let n = ink_line(&mut c, t, prm, x0, y0, x1, y1, p, tx, ty);
    canvas_free(&mut c);
    n
}

// The calligraphy nib's broad edge sits at 90 degrees to the pen's lean: a right hand leaning
// toward the bottom right makes strokes toward the bottom right broad and strokes up to the
// right hairlines; a left hand mirrors that; an upright pen gets the 45 degree default.
fn calligraphy_nib() {
    let mut p = ToolParams::default();
    tool_params_default(TOOL_CALLIGRAPHY, &mut p);
    // Test the fully-following nib geometry independently of the chosen default tilt range.
    p.tilt_min_deg = 10.0;
    p.tilt_max_deg = 25.0;
    check_near!(brush_nib_angle(&p, 0.0, 0.0), 45.0, 0.01);
    check_near!(brush_nib_angle(&p, 40.0, 46.0), 39.02, 0.05); // edge rises right
    check_near!(brush_nib_angle(&p, -40.0, 46.0), 140.98, 0.05); // edge falls right
    check_near!(brush_nib_angle(&p, 3.0, 3.0), 45.0, 0.01); // too upright to trust the lean
    let down_r = ink_on_fresh(TOOL_CALLIGRAPHY, &p, 300.0, 300.0, 700.0, 700.0, 0.7, 40.0, 46.0);
    let up_r = ink_on_fresh(TOOL_CALLIGRAPHY, &p, 300.0, 700.0, 700.0, 300.0, 0.7, 40.0, 46.0);
    check!(down_r > 4 * up_r);
    let down_l = ink_on_fresh(TOOL_CALLIGRAPHY, &p, 300.0, 300.0, 700.0, 700.0, 0.7, -40.0, 46.0);
    let up_l = ink_on_fresh(TOOL_CALLIGRAPHY, &p, 300.0, 700.0, 700.0, 300.0, 0.7, -40.0, 46.0);
    check!(up_l > 4 * down_l);
    check!((down_r - up_l).abs() < down_r / 10); // the mirror image, near enough
    check!(
        ink_on_fresh(TOOL_CALLIGRAPHY, &p, 300.0, 300.0, 700.0, 700.0, 0.7, 0.0, 0.0)
            > 4 * ink_on_fresh(TOOL_CALLIGRAPHY, &p, 300.0, 700.0, 700.0, 300.0, 0.7, 0.0, 0.0)
    );
    let mut fountain = ToolParams::default();
    tool_params_default(TOOL_FOUNTAIN, &mut fountain);
    // The fountain pen keeps its fixed 40 degree italic nib whatever the lean.
    check!(
        ink_on_fresh(TOOL_FOUNTAIN, &fountain, 300.0, 300.0, 700.0, 700.0, 0.7, 40.0, 46.0)
            == ink_on_fresh(TOOL_FOUNTAIN, &fountain, 300.0, 300.0, 700.0, 700.0, 0.7, -40.0, 46.0)
    );
}

// ToolParams from its 13 floats, in field order (the C's initializer).
fn tp(v: [f32; 13]) -> ToolParams {
    let mut p = ToolParams::default();
    for (i, x) in v.iter().enumerate() {
        *p.get_mut(i) = *x;
    }
    p
}

// Charcoal: paper shows through even when pressed, harder pressure is darker, and laid on its
// side it is broader and lighter.
fn charcoal_texture() {
    // A fixed wide shading profile, so this renderer check does not depend on later tuning.
    let p = tp([0.80, 2.40, 0.30, 1.00, 0.03, 0.85, 1.00, 45.0, 75.0, 2.0, 0.35, 0.85, 0.15]);
    let mut c = Canvas::default();
    canvas_init(&mut c, 1000, 400);
    ink_line(&mut c, TOOL_CHARCOAL, &p, 100.0, 200.0, 900.0, 200.0, 0.6, 0.0, 0.0);
    let mut light = 0;
    let mut total = 0;
    for y in 195..=205 {
        for x in 300..700 {
            light += (px(&c, x, y) > 200) as i32;
            total += 1;
        }
    }
    check!(light * 50 > total); // > 2% of the core is bare paper
    check!(light * 2 < total); // but most of it is charcoal
    canvas_free(&mut c);
    let mut soft: i64 = 0;
    let mut hard: i64 = 0;
    for pass in 0..2 {
        canvas_init(&mut c, 1000, 400);
        ink_line(&mut c, TOOL_CHARCOAL, &p, 100.0, 200.0, 900.0, 200.0, if pass != 0 { 1.0 } else { 0.35 }, 0.0, 0.0);
        let mut sum: i64 = 0;
        for y in 190..=210 {
            for x in 300..700 {
                sum += 255 - px(&c, x, y) as i64;
            }
        }
        if pass != 0 {
            hard = sum;
        } else {
            soft = sum;
        }
        canvas_free(&mut c);
    }
    check!(hard > soft * 3 / 2);
    let upright = ink_on_fresh(TOOL_CHARCOAL, &p, 100.0, 500.0, 900.0, 500.0, 0.8, 0.0, 0.0);
    let mut side = Canvas::default();
    canvas_init(&mut side, 1000, 1000);
    ink_line(&mut side, TOOL_CHARCOAL, &p, 100.0, 500.0, 900.0, 500.0, 0.8, 75.0, 0.0);
    let mut rows = 0; // rows touched by the side stroke vs the upright one
    for y in 0..side.h {
        let mut any = false;
        for x in 400..600 {
            any |= px(&side, x, y) < 200;
        }
        rows += any as i32;
    }
    check!(rows > 50); // about 3x the 2 mm upright width
    check!(upright > 0);
    canvas_free(&mut side);
}

// Every tool's ink stays inside the stroke's bbox (what redraws and erasing rely on), including
// charcoal's rough edge and dust and its broad side.
fn ink_inside_bbox() {
    for t in 0..TOOL_COUNT as i32 {
        if ToolKind(t) == TOOL_ERASER {
            continue;
        }
        for side in 0..2 {
            let mut prm = ToolParams::default();
            tool_params_default(ToolKind(t), &mut prm);
            prm.grain = if side != 0 { 1.0 } else { prm.grain }; // the roughest edge it can have
            let mut s = stroke_new(ToolKind(t), &prm, 4242u32 + t as u32);
            for i in 0..=60 {
                let f = i as f32 / 60.0;
                let q = PenSample {
                    x: fmla(f, 1200.0, 300.0),
                    y: fmla(200.0, (f * 5.0).sin(), 700.0),
                    pressure: 1.0,
                    tilt_x: if side != 0 { 80.0 } else { 30.0 },
                    tilt_y: if side != 0 { 10.0 } else { 30.0 },
                    t_ms: i as u32 * 5,
                };
                stroke_append(&mut s, &q, DPI);
            }
            let mut c = Canvas::default();
            canvas_init(&mut c, W, H);
            let mut b = BrushState::default();
            brush_begin(&mut b, s.tool, &s.params, s.seed, DPI);
            for i in 0..s.pts.len() {
                brush_add(&mut b, &mut c, &s.pts[i]);
            }
            let mut outside = 0;
            let mut inked = 0;
            for y in 0..H {
                for x in 0..W {
                    if c.px[(y * W + x) as usize] < 255 {
                        inked += 1;
                        outside += !rect_contains(s.bbox, x, y) as i32;
                    }
                }
            }
            check!(inked > 0);
            check!(outside == 0);
            if outside != 0 {
                eprintln!("  tool {} side {}: {} inked px outside the bbox", t, side, outside);
            }
            canvas_free(&mut c);
            stroke_free(s);
        }
    }
}

// The new pens re-render exactly as they were drawn live (the calligraphy nib's smoothing and
// charcoal's dust are part of the deterministic state), in full and in pieces.
fn new_pens_live_equals_rerender() {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    let mut live = Canvas::default();
    let mut re = Canvas::default();
    let mut chunk = Canvas::default();
    canvas_init(&mut live, W, H);
    canvas_init(&mut re, W, H);
    canvas_init(&mut chunk, W, H);
    paper_render(&mut live, pg.paper, FULL, DPI);
    const TS: [ToolKind; 3] = [TOOL_CHARCOAL, TOOL_CALLIGRAPHY, TOOL_CHARCOAL];
    const TILTS: [[f32; 2]; 3] = [[30.0, 30.0], [40.0, 46.0], [70.0, 20.0]];
    for k in 0..3 {
        let mut prm = ToolParams::default();
        tool_params_default(TS[k], &mut prm);
        let mut s = stroke_new(TS[k], &prm, 77u32 + k as u32);
        for i in 0..=120 {
            let f = i as f32 / 120.0;
            let ang = f * 4.0 * 6.2831853;
            // Wobbling tilt, so the nib keeps turning and smoothing.
            let q = PenSample {
                x: fmla(60.0, ang.cos(), fmla(f, 1200.0, 300.0)),
                y: fmla(60.0, ang.sin(), fmla(k as f32, 600.0, 500.0)),
                pressure: fmla(0.5, (f * 3.0).sin(), 0.4),
                tilt_x: fmla(8.0, (f * 11.0).sin(), TILTS[k][0]),
                tilt_y: fmla(-6.0, (f * 7.0).cos(), TILTS[k][1]),
                t_ms: i as u32 * 5,
            };
            stroke_append(&mut s, &q, DPI);
        }
        let mut b = BrushState::default();
        brush_begin(&mut b, s.tool, &s.params, s.seed, DPI);
        for i in 0..s.pts.len() {
            brush_add(&mut b, &mut live, &s.pts[i]);
        }
        page_add(&mut pg, s);
    }
    page_render(&mut re, &pg, FULL, DPI);
    check!(live.px[..(W * H) as usize] == re.px[..(W * H) as usize]);
    for i in 0..6 {
        page_render(&mut chunk, &pg, Rect { x: i * 310, y: 0, w: 310, h: H }, DPI);
    }
    check!(live.px[..(W * H) as usize] == chunk.px[..(W * H) as usize]);
    canvas_free(&mut live);
    canvas_free(&mut re);
    canvas_free(&mut chunk);
    page_free(pg);
}

fn hit_test() {
    let s = make_stroke(TOOL_BALLPOINT, 100.0, 100.0, 500.0, 100.0, 20, 0.5, 0.5, 0.0);
    check!(stroke_hit(&s, 300.0, fmla(25.0, (0.5f32 * 9.0).sin(), 100.0), 3.0, DPI));
    check!(!stroke_hit(&s, 300.0, 400.0, 3.0, DPI));
    check!(stroke_hit_segment(&s, 300.0, 0.0, 300.0, 300.0, 2.0, DPI));
    stroke_free(s);
}

fn history_roundtrip() {
    let mut p = page_new(W, H, PAPER_WIDE);
    let mut h = History::default();
    history_init(&mut h);
    let a = make_stroke(TOOL_PENCIL, 0.0, 0.0, 100.0, 100.0, 5, 0.5, 0.5, 0.0);
    let b = make_stroke(TOOL_PENCIL, 0.0, 200.0, 100.0, 300.0, 5, 0.5, 0.5, 0.0);
    let c = make_stroke(TOOL_PENCIL, 0.0, 400.0, 100.0, 500.0, 5, 0.5, 0.5, 0.0);
    // (the strokes' identities: the boxes move into the page, their addresses do not)
    let (ia, ib, ic) = (stroke_id(&a), stroke_id(&b), stroke_id(&c));
    history_push_add(&mut h, 0, &a);
    page_add(&mut p, a);
    history_push_add(&mut h, 1, &b);
    page_add(&mut p, b);
    history_push_add(&mut h, 2, &c);
    page_add(&mut p, c);
    // erase a and c in one gesture
    let idx = [0, 2];
    let rm1 = page_remove(&mut p, 2);
    let rm0 = page_remove(&mut p, 0);
    history_push_erase(&mut h, &idx, vec![rm0, rm1]);
    check!(p.strokes.len() == 1 && stroke_id(&p.strokes[0]) == ib);
    let mut d = Rect::default();
    check!(history_undo(&mut h, &mut p, &mut d));
    check!(p.strokes.len() == 3 && stroke_id(&p.strokes[0]) == ia && stroke_id(&p.strokes[1]) == ib && stroke_id(&p.strokes[2]) == ic);
    check!(history_redo(&mut h, &mut p, &mut d));
    check!(p.strokes.len() == 1);
    check!(history_undo(&mut h, &mut p, &mut d));
    check!(history_undo(&mut h, &mut p, &mut d)); // undo add c
    check!(p.strokes.len() == 2);
    history_push_paper(&mut h, PAPER_WIDE, PAPER_MUSIC); // clears redo (frees c)
    p.paper = PAPER_MUSIC;
    check!(!history_can_redo(&h));
    check!(history_undo(&mut h, &mut p, &mut d) && p.paper == PAPER_WIDE && d.w == W);
    history_push_clear(&mut h, &mut p);
    check!(p.strokes.is_empty());
    check!(history_undo(&mut h, &mut p, &mut d) && p.strokes.len() == 2);
    // limit trimming
    h.limit = 3;
    for _ in 0..10 {
        let s = make_stroke(TOOL_BALLPOINT, 0.0, 0.0, 10.0, 10.0, 2, 0.5, 0.5, 0.0);
        history_push_add(&mut h, p.strokes.len() as i32, &s);
        page_add(&mut p, s);
    }
    check!(h.undo.len() == 3);
    history_free(&mut h);
    page_free(p);
}

// A sampler sheet for eyeballing the tools.
fn tool_sampler() {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    let mut y: f32 = 340.0;
    for t in 0..3 {
        for k in 0..6 {
            // pressure ramps
            let pr = fmla(0.17, k as f32, 0.15);
            page_add(&mut pg, make_stroke(ToolKind(t), (330 + k * 250) as f32, y, (330 + k * 250 + 200) as f32, y + 40.0, 40, pr, pr, 0.0));
        }
        page_add(&mut pg, make_stroke(ToolKind(t), 330.0, y + 170.0, 1700.0, y + 170.0, 160, 0.05, 1.0, 0.0));
        for k in 0..3 {
            // tilt 0 / 35 / 65
            page_add(&mut pg, make_stroke(ToolKind(t), (330 + k * 480) as f32, y + 330.0, (330 + k * 480 + 400) as f32, y + 330.0, 60, 0.6, 0.6, k as f32 * 32.0));
        }
        y += 700.0;
    }
    let mut c = Canvas::default();
    canvas_init(&mut c, W, H);
    page_render(&mut c, &pg, FULL, DPI);
    if font_init() == 0 {
        for t in 0..3 {
            font_draw(&mut c, FontFace::Bold, 60.0, 330, 300 + t * 700, tool_name(ToolKind(t)), 0);
        }
    }
    out_png("tools.png", &c);
    canvas_free(&mut c);
    page_free(pg);
    // The newer pens: a handwriting-like loop line, a pressure ramp, and a figure that shows the
    // fountain pen's nib angle (thin one way, thick the other).
    const MORE: [ToolKind; 3] = [TOOL_FOUNTAIN, TOOL_FINELINER, TOOL_MARKER];
    let mut p2 = page_new(W, H, PAPER_MEDIUM);
    for i in 0..3 {
        let y0 = (380 + i * 700) as f32;
        let mut prm = ToolParams::default();
        tool_params_default(MORE[i as usize], &mut prm);
        let mut lp = stroke_new(MORE[i as usize], &prm, 7u32 + i as u32);
        for k in 0..=220 {
            let f = k as f32 / 220.0;
            let ang = f * 6.0 * 6.2831853;
            let q = PenSample {
                x: fmla(60.0, ang.cos(), fmla(f, 1300.0, 330.0)),
                y: fmla(60.0, ang.sin(), y0),
                pressure: fmla(0.3, (f * 3.14).sin(), 0.55),
                tilt_x: 0.0,
                tilt_y: 0.0,
                t_ms: k as u32 * 5,
            };
            stroke_append(&mut lp, &q, DPI);
        }
        page_add(&mut p2, lp);
        page_add(&mut p2, make_stroke(MORE[i as usize], 330.0, y0 + 220.0, 1700.0, y0 + 220.0, 160, 0.05, 1.0, 0.0));
        page_add(&mut p2, make_stroke(MORE[i as usize], 330.0, y0 + 330.0, 900.0, y0 + 420.0, 60, 0.7, 0.7, 0.0));
        page_add(&mut p2, make_stroke(MORE[i as usize], 1000.0, y0 + 420.0, 1600.0, y0 + 330.0, 60, 0.7, 0.7, 0.0));
    }
    canvas_init(&mut c, W, H);
    page_render(&mut c, &p2, FULL, DPI);
    for i in 0..3 {
        font_draw(&mut c, FontFace::Bold, 60.0, 330, 280 + i * 700, tool_name(MORE[i as usize]), 0);
    }
    out_png("tools2.png", &c);
    canvas_free(&mut c);
    page_free(p2);
}

fn highlighter_vertical_tip() {
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    check!(canvas_init(&mut a, 200, 200) == 0 && canvas_init(&mut b, 200, 200) == 0);
    let mut p = ToolParams::default();
    tool_params_default(TOOL_HIGHLIGHTER, &mut p);
    let mut brush = BrushState::default();
    let mut q = PenSample { x: 100.0, y: 100.0, pressure: 0.8, tilt_x: 0.0, tilt_y: 0.0, t_ms: 0 };
    brush_begin(&mut brush, TOOL_HIGHLIGHTER, &p, 1, DPI);
    brush_add(&mut brush, &mut a, &q);
    let (mut x0, mut y0, mut x1, mut y1) = (200, 200, -1, -1);
    for y in 0..200 {
        for x in 0..200 {
            if px(&a, x, y) < 255 {
                x0 = if x < x0 { x } else { x0 };
                x1 = if x > x1 { x } else { x1 };
                y0 = if y < y0 { y } else { y0 };
                y1 = if y > y1 { y } else { y1 };
            }
        }
    }
    check!(y1 - y0 > 6 * (x1 - x0) && x1 > x0);
    check!(px(&a, 100, 88) < 255 && px(&a, 88, 100) == 255);
    q.tilt_x = 60.0;
    q.tilt_y = -20.0;
    brush_begin(&mut brush, TOOL_HIGHLIGHTER, &p, 1, DPI);
    brush_add(&mut brush, &mut b, &q);
    check!(a.px[..(a.stride * a.h) as usize] == b.px[..(a.stride * a.h) as usize]);
    canvas_free(&mut a);
    canvas_free(&mut b);
}

#[test]
fn test_ink() {
    font_init(); // some papers draw labels and clefs
    run!(rects);
    run!(canvas_basics);
    run!(round_rect_fast_matches_reference);
    run!(paper_subregion_matches_full);
    run!(pressure_tilt_mapping);
    run!(live_equals_rerender);
    run!(hit_test);
    run!(history_roundtrip);
    run!(tool_sampler);
    run!(calligraphy_nib);
    run!(charcoal_texture);
    run!(ink_inside_bbox);
    run!(new_pens_live_equals_rerender);
    run!(highlighter_vertical_tip);
    done();
}
