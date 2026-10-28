// test_erase.c: erase strokes, the white-ink engine (docs/FORMAT.md, Erase strokes). White ink and
// the precise eraser restore the paper under them: live drawing equals every re-render, the
// eraser's core lands exactly on the paper with its lines, ink drawn later is untouched, the fast
// path is the per-pixel path byte for byte, compaction drops only what changes nothing, and
// 1.0.0-beta still opens every page with erase strokes.
// (the C literal 6.28f and its `% 3 == 0` tests are kept as they are)
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::approx_constant,
    clippy::manual_is_multiple_of,
    clippy::if_same_then_else,
    clippy::vec_init_then_push
)]
mod beta_format;
mod testlib;
use grimoire::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
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

// ToolParams from its 13 floats, in field order (the C's initializer).
fn tp(v: [f32; 13]) -> ToolParams {
    let mut p = ToolParams::default();
    for (i, x) in v.iter().enumerate() {
        *p.get_mut(i) = *x;
    }
    p
}

// Profiles used to record the 1.1 erase renders. A stroke stores these values; current pen
// defaults may change, but these profiles and the recorded CRCs must stay frozen.
fn k_legacy_params() -> [ToolParams; TOOL_COUNT] {
    [
        tp([0.20, 0.45, 0.30, 0.95, 0.03, 0.85, 1.25, 15.0, 60.0, 1.6, 0.45, 0.65, 0.10]),
        tp([0.26, 0.36, 0.38, 0.85, 0.03, 0.80, 1.10, 25.0, 70.0, 0.35, 0.10, 0.50, 0.15]),
        tp([0.30, 0.48, 0.85, 1.00, 0.02, 0.55, 0.60, 30.0, 75.0, 0.10, 0.00, 0.06, 0.35]),
        tp([2.5, 7.0, 1.0, 1.0, 0.0, 0.60, 1.00, 30.0, 75.0, 0.0, 0.0, 0.0, 0.0]),
        tp([4.0, 5.0, 0.55, 0.65, 0.02, 0.50, 0.80, 30.0, 70.0, 0.2, 0.0, 0.0, 0.40]),
        tp([0.35, 1.10, 0.90, 1.00, 0.02, 0.60, 0.80, 30.0, 75.0, 0.0, 0.0, 0.0, 0.30]),
        tp([0.28, 0.34, 1.00, 1.00, 0.01, 0.30, 1.00, 30.0, 75.0, 0.0, 0.0, 0.0, 0.30]),
        tp([1.50, 2.00, 0.92, 1.00, 0.01, 0.40, 0.70, 30.0, 75.0, 0.3, 0.0, 0.04, 0.30]),
        tp([0.80, 2.40, 0.30, 1.00, 0.03, 0.85, 1.00, 45.0, 75.0, 2.0, 0.35, 0.85, 0.15]),
        tp([0.70, 1.40, 0.92, 1.00, 0.02, 0.60, 0.80, 10.0, 25.0, 0.0, 0.0, 0.0, 0.30]),
    ]
}
fn legacy(t: ToolKind) -> ToolParams {
    k_legacy_params()[t.0 as usize]
}

fn set_erase_spans(on: bool) {
    BRUSH_ERASE_SPANS.store(on, Ordering::Relaxed);
}

// ---- building strokes ----

// Fills s with a wavy line from (x0, y0) to (x1, y1): pressure p0..p1, tilt (tx, ty) wobbling.
fn wave_points(s: &mut Stroke, x0: f32, y0: f32, x1: f32, y1: f32, amp: f32, p0: f32, p1: f32, tx: f32, ty: f32) {
    let steps = (fmla(x1 - x0, x1 - x0, (y1 - y0) * (y1 - y0)).sqrt() / 12.0) as i32 + 8;
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let q = PenSample {
            x: lerpf(x0, x1, f),
            y: fmla(amp, fmla(f, 9.0, (s.seed & 7) as f32).sin(), lerpf(y0, y1, f)),
            pressure: lerpf(p0, p1, f),
            tilt_x: fmla(6.0, (f * 11.0).sin(), tx),
            tilt_y: fmla(-5.0, (f * 7.0).cos(), ty),
            t_ms: i as u32 * 5,
        };
        stroke_append(s, &q, DPI);
    }
}

fn ink(t: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, amp: f32, p0: f32, p1: f32, tx: f32, ty: f32, seed: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    let mut s = stroke_new(t, &prm, seed);
    wave_points(&mut s, x0, y0, x1, y1, amp, p0, p1, tx, ty);
    s
}

// An erase stroke with a texture (TOOL_ERASER: the round tip) and that tool's default settings.
fn white(texture: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, amp: f32, p0: f32, p1: f32, tx: f32, ty: f32, hash: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(texture, &mut prm);
    let mut s = stroke_new(TOOL_ERASER, &prm, erase_seed(texture, hash));
    wave_points(&mut s, x0, y0, x1, y1, amp, p0, p1, tx, ty);
    s
}

// The round eraser along a straight line, upright, at pressure p (1 = its 7 mm).
fn rubber(x0: f32, y0: f32, x1: f32, y1: f32, p: f32, hash: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_ERASER, &mut prm);
    let mut s = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_ERASER, hash));
    let steps = (fmla(x1 - x0, x1 - x0, (y1 - y0) * (y1 - y0)).sqrt() / 10.0) as i32 + 1;
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let q = PenSample { x: lerpf(x0, x1, f), y: lerpf(y0, y1, f), pressure: p, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 5 };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// The paper beside a live canvas, as the brushes take it: shared bytes (the C hands them the
// canvas's own).
fn paper_bytes(c: &Canvas) -> Rc<Vec<u8>> {
    Rc::new(c.px.clone())
}

// Draws s the way the editor does: a new stroke, then stroke_append and brush_add sample by
// sample over c, with the page's paper beside it (None = white). Returns the new stroke.
fn draw_live(c: &mut Canvas, paper: Option<&Canvas>, s: &Stroke, restored: Option<&mut bool>) -> Box<Stroke> {
    let mut n = stroke_new(s.tool, &s.params, s.seed);
    let mut b = BrushState::default();
    stroke_brush_begin(&mut b, &n, DPI, paper.map(paper_bytes), 0, 0, paper.map_or(0, |p| p.stride));
    for k in 0..s.pts.len() {
        stroke_append(&mut n, &s.pts[k], DPI);
        brush_add(&mut b, c, &s.pts[k]);
    }
    if let Some(r) = restored {
        *r = b.restored;
    }
    n
}

fn diff_px(a: &Canvas, b: &Canvas, r: Rect) -> i64 {
    let mut n = 0;
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h });
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            n += (a.px[(y * a.stride + x) as usize] != b.px[(y * b.stride + x) as usize]) as i64;
        }
    }
    n
}

fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let l2 = fmla(dx, dx, dy * dy);
    let t = if l2 > 0.0 { clampf(fmla(px - ax, dx, (py - ay) * dy) / l2, 0.0, 1.0) } else { 0.0 };
    let ex = px - fmla(dx, t, ax);
    let ey = py - fmla(dy, t, ay);
    fmla(ex, ex, ey * ey).sqrt()
}

// Distance from a pixel's centre to a stroke's polyline.
fn poly_dist(s: &Stroke, x: i32, y: i32) -> f32 {
    let mut m = 1e9f32;
    for k in 0..s.pts.len().saturating_sub(1) {
        m = m.min(seg_dist(x as f32 + 0.5, y as f32 + 0.5, s.pts[k].x, s.pts[k].y, s.pts[k + 1].x, s.pts[k + 1].y));
    }
    m
}

// The same for a straight stroke (rubber): the segment from its first point to its last.
fn line_dist(s: &Stroke, x: i32, y: i32) -> f32 {
    let last = s.pts.len() - 1;
    seg_dist(x as f32 + 0.5, y as f32 + 0.5, s.pts[0].x, s.pts[0].y, s.pts[last].x, s.pts[last].y)
}

// The round eraser at full pressure: its core (full coverage) reaches r_in from each dab; a pixel
// this close to its line is that close to a dab too (dabs are 6 px apart, the last one at most a
// spacing short of the end).
fn rubber_core() -> f32 {
    let r = mm_to_px(7.0, DPI) * 0.5;
    fmla(r, 1.0f32 - 0.08f32, -0.5) - 8.0
}

// A view of p without the strokes marked in drop. (The C's borrows p's strokes; a stroke has one
// owner here, so the view holds copies of them: it renders the same.)
fn page_without(p: &Page, drop: &[bool]) -> Box<Page> {
    let mut v = page_new(p.w, p.h, p.paper);
    for i in 0..p.strokes.len() {
        if !drop[i] {
            v.strokes.push(p.strokes[i].clone());
        }
    }
    v
}

// Frees a page_without view.
fn view_free(v: Box<Page>) {
    page_free(v);
}

// A page that has it all, drawn live over `live` (the paper beside it in `paper`): ink with every
// tool, white with every texture and the round tip at several pressures, a soft eraser (no fast
// path), strokes running off all four edges, ink drawn after the erasing, then more erasing.
fn busy_page(k: PaperKind, live: &mut Canvas, paper: &mut Canvas) -> Box<Page> {
    let mut pg = page_new(W, H, k);
    paper_render(paper, k, FULL, DPI);
    canvas_blit(live, paper, FULL, 0, 0);
    let mut tpl: Vec<Box<Stroke>> = Vec::new();
    let mut seed: u32 = 1;
    let mut next = || {
        seed += 1;
        seed - 1
    };
    for t in 0..TOOL_COUNT as i32 {
        if ToolKind(t) != TOOL_ERASER {
            tpl.push(ink(ToolKind(t), -40.0, (140 + t * 250) as f32, 1900.0, (160 + t * 250) as f32, 70.0, 0.1, 1.0, 35.0, 30.0, next()));
        }
    }
    for t in 0..TOOL_COUNT as i32 {
        // every texture (TOOL_ERASER: the round tip), off the top and bottom
        tpl.push(white(ToolKind(t), (90 + t * 175) as f32, -40.0, (150 + t * 175) as f32, 2520.0, 40.0, 0.2, 1.0, 40.0, 46.0, next()));
    }
    tpl.push(rubber(-60.0, 900.0, 1920.0, 1010.0, 0.0, next())); // the thinnest round tip, off both sides
    tpl.push(rubber(200.0, 1500.0, 1700.0, 2300.0, 0.5, next()));
    let mut soft = white(TOOL_ERASER, 300.0, 300.0, 1500.0, 2100.0, 300.0, 0.3, 1.0, 0.0, 0.0, next());
    soft.params.opacity_min = 0.5;
    soft.params.opacity_max = 0.5; // half strength: no fast path
    tpl.push(soft);
    for t in 0..TOOL_COUNT as i32 {
        if ToolKind(t) != TOOL_ERASER {
            tpl.push(ink(ToolKind(t), -30.0, (220 + t * 230) as f32, 1890.0, (300 + t * 230) as f32, 250.0, 0.2, 0.9, -30.0, 40.0, next()));
        }
    }
    tpl.push(white(TOOL_CALLIGRAPHY, -50.0, 1500.0, 1910.0, 1600.0, 900.0, 0.4, 1.0, 40.0, 46.0, next()));
    tpl.push(white(TOOL_CHARCOAL, -50.0, 2000.0, 1910.0, 1700.0, 300.0, 0.5, 1.0, 75.0, 10.0, next())); // on its side
    tpl.push(rubber(-80.0, 2470.0, 1940.0, 2490.0, 1.0, next())); // off the bottom edge
    tpl.push(rubber(930.0, -60.0, 940.0, 320.0, 1.0, next())); // off the top edge
    for s in tpl {
        page_add(&mut pg, draw_live(live, Some(paper), &s, None));
        stroke_free(s);
    }
    pg
}

// ---- tests ----

fn restore_blend() {
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 64, 64) == 0);
    let at = 10 * 64 + 10;
    c.px[at] = 200;
    canvas_restore_px(&mut c, 10, 10, 150, 1.0); // already lighter than the paper: kept
    check!(c.px[at] == 200);
    c.px[at] = 100;
    canvas_restore_px(&mut c, 10, 10, 200, 0.5);
    check!(c.px[at] == 150);
    canvas_restore_px(&mut c, 10, 10, 200, 1.0);
    check!(c.px[at] == 200);
    c.px[at] = 254;
    canvas_restore_px(&mut c, 10, 10, 255, 0.4); // a partial share stops a level short...
    check!(c.px[at] == 254);
    canvas_restore_px(&mut c, 10, 10, 255, 0.999); // ...full coverage lands on the paper
    check!(c.px[at] == 255);
    c.px[at] = 37;
    canvas_restore_px(&mut c, 10, 10, 180, 0.9995);
    check!(c.px[at] == 180);
    // Never past the paper, never darker, whatever the alpha.
    srand(3);
    for _ in 0..20000 {
        let v = (rand() & 255) as u8;
        let paper = (rand() & 255) as u8;
        let a = (rand() % 1500) as f32 / 1000.0;
        c.px[at] = v;
        canvas_restore_px(&mut c, 10, 10, paper, a);
        check!(c.px[at] >= v && (c.px[at] <= paper || c.px[at] == v));
    }
    // The clip holds.
    canvas_set_clip(&mut c, Rect { x: 20, y: 20, w: 10, h: 10 });
    c.px[at] = 0;
    canvas_restore_px(&mut c, 10, 10, 255, 1.0);
    check!(c.px[at] == 0);
    canvas_free(&mut c);
}

// Live drawing over the editor's page canvas, with the paper beside it, gives exactly what every
// re-render gives: the full page, column tiles, 131x97 tiles and random rects in any order; the
// ink layer too, in full and in tiles; and no pixel ends up lighter than the paper.
fn live_equals_rerender() {
    const KINDS: [PaperKind; 2] = [PAPER_MEDIUM, PAPER_GRAND];
    for ki in 0..2 {
        let mut live = Canvas::default();
        let mut paper = Canvas::default();
        let mut re = Canvas::default();
        let mut part = Canvas::default();
        canvas_init(&mut live, W, H);
        canvas_init(&mut paper, W, H);
        canvas_init(&mut re, W, H);
        canvas_init(&mut part, W, H);
        let pg = busy_page(KINDS[ki], &mut live, &mut paper);
        page_render(&mut re, &pg, FULL, DPI);
        check!(diff_px(&live, &re, FULL) == 0);
        canvas_fill(&mut part, FULL, 77);
        for i in 0..6 {
            page_render(&mut part, &pg, Rect { x: i * 310, y: 0, w: 310, h: H }, DPI);
        }
        check!(diff_px(&live, &part, FULL) == 0);
        if ki == 0 {
            // (small tiles replay every long stroke over and over: one paper is enough)
            canvas_fill(&mut part, FULL, 77);
            let mut y = 0;
            while y < H {
                let mut x = 0;
                while x < W {
                    page_render(&mut part, &pg, Rect { x, y, w: 131, h: 97 }, DPI);
                    x += 131;
                }
                y += 97;
            }
            check!(diff_px(&live, &part, FULL) == 0);
        }
        // Random overlapping rects in a random order, as undo and the stroke eraser redraw.
        canvas_blit(&mut part, &re, FULL, 0, 0);
        srand(7 + ki as u32);
        for _ in 0..(if ki != 0 { 100 } else { 300 }) {
            let r = Rect { x: rand() % W - 50, y: rand() % H - 50, w: 1 + rand() % 400, h: 1 + rand() % 400 };
            page_render(&mut part, &pg, r, DPI);
        }
        check!(diff_px(&live, &part, FULL) == 0);
        let mut lighter: i64 = 0;
        let mut erased: i64 = 0;
        for i in 0..(W * H) as usize {
            lighter += (re.px[i] > paper.px[i]) as i64;
        }
        check!(lighter == 0);
        // The ink layer (?layer=ink): erase strokes restore toward white there, full or in tiles.
        canvas_fill(&mut re, FULL, 255);
        page_render_layers(&mut re, &pg, FULL, DPI, PAGE_LAYER_INK);
        canvas_fill(&mut part, FULL, 255);
        for i in 0..6 {
            page_render_layers(&mut part, &pg, Rect { x: i * 310, y: 0, w: 310, h: H }, DPI, PAGE_LAYER_INK);
        }
        check!(diff_px(&re, &part, FULL) == 0);
        // The erasing did something: without the erase strokes, much more ink.
        let drop: Vec<bool> = pg.strokes.iter().map(|s| stroke_is_erase(s)).collect();
        let inkonly = page_without(&pg, &drop);
        canvas_fill(&mut part, FULL, 255);
        page_render_layers(&mut part, &inkonly, FULL, DPI, PAGE_LAYER_INK);
        for i in 0..(W * H) as usize {
            erased += (part.px[i] < 128 && re.px[i] >= 250) as i64;
        }
        check!(erased > 10000);
        view_free(inkonly);
        canvas_free(&mut live);
        canvas_free(&mut paper);
        canvas_free(&mut re);
        canvas_free(&mut part);
        page_free(pg);
    }
}

// The fast path (a round dab's core restored a row at a time) is the per-pixel path, byte for
// byte: single dabs of every size up to the largest at 300 dpi and past the fast path's limit, at
// any centre, over any content (pixels lighter than their paper too) and any clip, toward white or
// a random paper; and whole pages.
fn span_fast_path_is_the_per_pixel_path() {
    const N: i32 = 1000;
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    let mut paper = Canvas::default();
    canvas_init(&mut a, N, N);
    canvas_init(&mut b, N, N);
    canvas_init(&mut paper, N, N);
    srand(11);
    for i in 0..(N * N) as usize {
        paper.px[i] = (rand() & 255) as u8;
    }
    let paper_rc = paper_bytes(&paper);
    const DPIS: [f32; 3] = [300.0, 433.0, 600.0];
    const ROUND: [ToolKind; 3] = [TOOL_ERASER, TOOL_FINELINER, TOOL_BALLPOINT];
    let mut eligible = 0;
    let mut bad: i64 = 0;
    for it in 0..1500 {
        if it % 8 == 0 {
            // fresh content: random gray blocks on both canvases
            for _ in 0..12 {
                let r = Rect { x: rand() % N - 100, y: rand() % N - 100, w: 50 + rand() % 400, h: 50 + rand() % 400 };
                let g = (rand() & 255) as u8;
                canvas_fill(&mut a, r, g);
                canvas_fill(&mut b, r, g);
            }
        }
        let mut clip = Rect { x: 0, y: 0, w: N, h: N };
        if it % 3 == 0 {
            clip = Rect { x: rand() % 600, y: rand() % 600, w: 50 + rand() % 500, h: 50 + rand() % 500 };
        }
        canvas_set_clip(&mut a, clip);
        canvas_set_clip(&mut b, clip);
        let dpi = DPIS[(rand() % 3) as usize];
        let t = ROUND[(rand() % 3) as usize];
        let mut prm = ToolParams::default();
        tool_params_default(t, &mut prm);
        prm.size_min_mm = 0.1 + (rand() % 1190) as f32 / 100.0;
        prm.size_max_mm = prm.size_min_mm;
        prm.tilt_size = (rand() % 5) as f32;
        prm.opacity_min = 1.0;
        prm.opacity_max = 1.0;
        prm.grain = 0.0;
        let sx = (rand() % 1300) as f32 - 150.0 + (rand() % 1000) as f32 / 1000.0;
        let sy = (rand() % 1300) as f32 - 150.0 + (rand() % 1000) as f32 / 1000.0;
        let spr = (rand() % 1001) as f32 / 1000.0;
        let stx = (rand() % 179) as f32 - 89.0;
        let sty = (rand() % 179) as f32 - 89.0;
        let s = PenSample { x: sx, y: sy, pressure: spr, tilt_x: stx, tilt_y: sty, t_ms: 0 };
        let pp = if rand() % 4 != 0 { Some(paper_rc.clone()) } else { None };
        let mut ba = BrushState::default();
        brush_begin(&mut ba, t, &prm, it as u32, dpi);
        brush_set_erase(&mut ba, pp, 0, 0, N);
        let mut bb = ba.clone();
        set_erase_spans(true);
        let da = brush_add(&mut ba, &mut a, &s);
        set_erase_spans(false);
        let db = brush_add(&mut bb, &mut b, &s);
        set_erase_spans(true);
        check!(da.x == db.x && da.y == db.y && da.w == db.w && da.h == db.h);
        check!(ba.restored == bb.restored);
        let d = diff_px(&a, &b, da);
        bad += d;
        if d != 0 {
            eprintln!("  dab {} (dpi {:.0}, tool {}, {:.2} mm): {} px differ", it, dpi, t.0, prm.size_max_mm, d);
        }
        let mut r = 0.0f32;
        let mut op = 0.0f32;
        brush_dab(t, &prm, &s, ba.dpi, &mut r, &mut op);
        let r_in = fmla(r, 1.0 - (if t == TOOL_ERASER { 0.08f32 } else if t == TOOL_FINELINER { 0.08 } else { 0.10 }), -0.5);
        eligible += (r_in > 2.5 && r_in <= 512.0 && !rect_is_empty(da)) as i32;
    }
    check!(bad == 0);
    check!(eligible > 600); // most dabs took the fast path
    canvas_free(&mut a);
    canvas_free(&mut b);
    canvas_free(&mut paper);
    // Whole pages, and their ink layers.
    let mut live = Canvas::default();
    let mut pap = Canvas::default();
    let mut on = Canvas::default();
    let mut off = Canvas::default();
    canvas_init(&mut live, W, H);
    canvas_init(&mut pap, W, H);
    canvas_init(&mut on, W, H);
    canvas_init(&mut off, W, H);
    let pg = busy_page(PAPER_DOTS, &mut live, &mut pap);
    page_render(&mut on, &pg, FULL, DPI);
    set_erase_spans(false);
    page_render(&mut off, &pg, FULL, DPI);
    check!(diff_px(&on, &off, FULL) == 0);
    canvas_fill(&mut off, FULL, 255);
    page_render_layers(&mut off, &pg, FULL, DPI, PAGE_LAYER_INK);
    set_erase_spans(true);
    canvas_fill(&mut on, FULL, 255);
    page_render_layers(&mut on, &pg, FULL, DPI, PAGE_LAYER_INK);
    check!(diff_px(&on, &off, FULL) == 0);
    canvas_free(&mut live);
    canvas_free(&mut pap);
    canvas_free(&mut on);
    canvas_free(&mut off);
    page_free(pg);
}

// The round eraser's core goes back exactly to the paper, keeping every line of it, on ruled,
// music, graph, dotted, grand staff and custom music paper (ruled once more without the fast
// path); and erasing never makes a pixel lighter than its paper.
fn eraser_core_returns_to_paper() {
    let m = MusicLayout { systems: 3, n: 4, part: [PART_TREBLE, PART_BASS, PART_TAB6, PART_LYRICS, PART_NONE, PART_NONE] };
    let kinds = [PAPER_MEDIUM, PAPER_MUSIC, PAPER_GRID, PAPER_DOTS, PAPER_GRAND, music_encode(&m), PAPER_MEDIUM];
    let lim = rubber_core();
    let mut re = Canvas::default();
    let mut before = Canvas::default();
    let mut paper = Canvas::default();
    canvas_init(&mut re, W, H);
    canvas_init(&mut before, W, H);
    canvas_init(&mut paper, W, H);
    for ki in 0..7 {
        set_erase_spans(ki < 6);
        let mut pg = page_new(W, H, kinds[ki]);
        for i in 0..27 {
            let t = ToolKind(if i % 9 >= TOOL_ERASER.0 { i % 9 + 1 } else { i % 9 });
            let fi = i as f32;
            page_add(&mut pg, ink(t, 100.0, fmla(fi, 40.0, 700.0), 1760.0, fmla(fi, 40.0, 760.0), 30.0, 0.4, 1.0, 20.0, 20.0, 1 + i as u32));
        }
        page_render(&mut before, &pg, FULL, DPI);
        let er = rubber(300.0, 600.0, 1500.0, 1900.0, 1.0, 77);
        page_add(&mut pg, er.clone());
        page_render(&mut re, &pg, FULL, DPI);
        paper_render(&mut paper, kinds[ki], FULL, DPI);
        let (mut core, mut off, mut inked, mut lines, mut kept, mut lighter) = (0i64, 0i64, 0i64, 0i64, 0i64, 0i64);
        let bb = rect_intersect(er.bbox, FULL);
        for y in bb.y..bb.y + bb.h {
            for x in bb.x..bb.x + bb.w {
                if line_dist(&er, x, y) > lim {
                    continue;
                }
                let i = (y * W + x) as usize;
                core += 1;
                off += (re.px[i] != paper.px[i]) as i64;
                inked += (before.px[i] < paper.px[i]) as i64;
                if paper.px[i] < 255 {
                    lines += 1;
                    kept += (re.px[i] == paper.px[i]) as i64;
                }
            }
        }
        for i in 0..(W * H) as usize {
            lighter += (re.px[i] > paper.px[i]) as i64;
        }
        check!(core > 80000 && inked > 20000); // the eraser had plenty to erase
        check!(off == 0);
        check!(lines > 20 && kept == lines);
        check!(lighter == 0);
        if off != 0 || kept != lines || lighter != 0 {
            eprintln!("  paper {}: core {} px, {} off; lines {} px, {} kept; {} lighter", ki, core, off, lines, kept, lighter);
        }
        page_free(pg);
    }
    set_erase_spans(true);
    canvas_free(&mut re);
    canvas_free(&mut before);
    canvas_free(&mut paper);
}

// Ink drawn after an erase stroke is untouched by it: inside the eraser's core it is exactly the
// same ink drawn alone on fresh paper.
fn ink_after_erasing_is_untouched() {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    let mut later = page_new(W, H, PAPER_MEDIUM);
    for i in 0..8 {
        let fi = i as f32;
        page_add(&mut pg, ink(TOOL_MARKER, 500.0, fmla(fi, 220.0, 400.0), 1300.0, fmla(fi, 220.0, 420.0), 60.0, 1.0, 1.0, 0.0, 0.0, 3 + i as u32));
    }
    let er = rubber(900.0, 300.0, 905.0, 2200.0, 1.0, 9);
    page_add(&mut pg, er.clone());
    let mut row = 0;
    for t in 0..TOOL_COUNT as i32 {
        if ToolKind(t) == TOOL_ERASER {
            continue;
        }
        let y = fmla(row as f32, 220.0, 380.0);
        page_add(&mut pg, ink(ToolKind(t), 650.0, y, 1150.0, y + 140.0, 50.0, 0.3, 1.0, 30.0, 30.0, 50 + t as u32));
        page_add(&mut later, ink(ToolKind(t), 650.0, y, 1150.0, y + 140.0, 50.0, 0.3, 1.0, 30.0, 30.0, 50 + t as u32));
        row += 1;
    }
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    canvas_init(&mut a, W, H);
    canvas_init(&mut b, W, H);
    page_render(&mut a, &pg, FULL, DPI);
    page_render(&mut b, &later, FULL, DPI);
    let lim = rubber_core();
    let (mut core, mut differ, mut dark) = (0i64, 0i64, 0i64);
    let bb = rect_intersect(er.bbox, FULL);
    for y in bb.y..bb.y + bb.h {
        for x in bb.x..bb.x + bb.w {
            if line_dist(&er, x, y) > lim {
                continue;
            }
            core += 1;
            differ += (a.px[(y * W + x) as usize] != b.px[(y * W + x) as usize]) as i64;
            dark += (b.px[(y * W + x) as usize] < 100) as i64;
        }
    }
    check!(core > 50000 && dark > 1500 && differ == 0);
    canvas_free(&mut a);
    canvas_free(&mut b);
    page_free(pg);
    page_free(later);
}

// Every white texture renders the same every time, live or replayed, and hashes with the seed's
// low 24 bits (its top byte is the texture). The grainy ones leave the paper's tooth; the round tip
// leaves nothing.
fn white_textures_are_deterministic() {
    const PW: i32 = 900;
    const PH: i32 = 700;
    let all = Rect { x: 0, y: 0, w: PW, h: PH };
    let mut paper = Canvas::default();
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    let mut live = Canvas::default();
    canvas_init(&mut paper, PW, PH);
    canvas_init(&mut a, PW, PH);
    canvas_init(&mut b, PW, PH);
    canvas_init(&mut live, PW, PH);
    for t in 0..TOOL_COUNT as i32 {
        let mut pg = page_new(PW, PH, PAPER_MEDIUM);
        for i in 0..6 {
            let fi = i as f32;
            page_add(&mut pg, ink(TOOL_MARKER, 60.0, fmla(fi, 50.0, 200.0), 840.0, fmla(fi, 50.0, 210.0), 10.0, 1.0, 1.0, 0.0, 0.0, 1 + i as u32));
        }
        paper_render(&mut paper, PAPER_MEDIUM, all, DPI);
        canvas_blit(&mut live, &paper, all, 0, 0);
        for i in 0..pg.strokes.len() {
            stroke_free(draw_live(&mut live, Some(&paper), &pg.strokes[i], None));
        }
        let w = white(ToolKind(t), 100.0, 330.0, 800.0, 420.0, 60.0, 0.6, 1.0, 30.0, 25.0, 0x00C0FFEE + t as u32);
        check!(stroke_render_tool(&w) == ToolKind(t));
        check!(w.seed >> 24 == if ToolKind(t) == TOOL_ERASER { 0 } else { t as u32 + 1 });
        let mut restored = false;
        stroke_free(draw_live(&mut live, Some(&paper), &w, Some(&mut restored)));
        check!(restored);
        page_add(&mut pg, w.clone());
        page_render(&mut a, &pg, all, DPI);
        canvas_fill(&mut b, all, 0);
        page_render(&mut b, &pg, all, DPI);
        check!(diff_px(&a, &b, all) == 0);
        check!(diff_px(&a, &live, all) == 0);
        // Tooth: marker pixels near the white stroke's middle that stay well below the paper.
        let mut tooth = 0;
        for y in 340..410 {
            for x in 300..600 {
                if poly_dist(&w, x, y) < 2.0 {
                    tooth += (a.px[(y * PW + x) as usize] as i32 + 40 < paper.px[(y * PW + x) as usize] as i32) as i32;
                }
            }
        }
        let tk = ToolKind(t);
        let grainy = tk == TOOL_PENCIL || tk == TOOL_MECH_PENCIL || tk == TOOL_CHARCOAL;
        check!(if grainy { tooth > 0 } else { tk != TOOL_ERASER || tooth == 0 });
        page_free(pg);
    }
    // The brush hashes the low 24 bits: the same as a charcoal brush begun with them, by hand.
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_CHARCOAL, &mut prm);
    let mut s = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_CHARCOAL, 0xAB123456));
    check!(s.seed == ((TOOL_CHARCOAL.0 as u32 + 1) << 24 | 0x123456));
    wave_points(&mut s, 100.0, 300.0, 800.0, 400.0, 80.0, 0.5, 1.0, 60.0, 20.0);
    canvas_fill(&mut a, all, 0);
    canvas_fill(&mut b, all, 0);
    stroke_free(draw_live(&mut a, None, &s, None));
    let mut hand = BrushState::default();
    brush_begin(&mut hand, TOOL_CHARCOAL, &prm, 0x123456, DPI);
    brush_set_erase(&mut hand, None, 0, 0, 0);
    for k in 0..s.pts.len() {
        brush_add(&mut hand, &mut b, &s.pts[k]);
    }
    check!(diff_px(&a, &b, all) == 0);
    // Other low bits: other grain and specks.
    let mut s2 = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_CHARCOAL, 0x654321));
    wave_points(&mut s2, 100.0, 300.0, 800.0, 400.0, 80.0, 0.5, 1.0, 60.0, 20.0);
    canvas_fill(&mut b, all, 0);
    stroke_free(draw_live(&mut b, None, &s2, None));
    check!(diff_px(&a, &b, all) > 100);
    stroke_free(s);
    stroke_free(s2);
    canvas_free(&mut paper);
    canvas_free(&mut a);
    canvas_free(&mut b);
    canvas_free(&mut live);
}

// A texture byte that is not a drawing tool (4, the eraser itself, or a tool this version lacks)
// draws the round tip; erase_seed writes the bytes FORMAT.md gives.
fn other_texture_bytes_draw_the_round_tip() {
    check!(erase_seed(TOOL_ERASER, 0xFF123456) == 0x00123456);
    check!(erase_seed(TOOL_PENCIL, 7) == 0x01000007);
    check!(erase_seed(TOOL_CALLIGRAPHY, 7) == 0x0A000007);
    check!(erase_seed(ToolKind(TOOL_COUNT as i32), 7) == 7 && erase_seed(ToolKind(200), 7) == 7);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_ERASER, &mut prm);
    let ink_s = stroke_new(TOOL_PENCIL, &prm, 0x05000000);
    check!(stroke_render_tool(&ink_s) == TOOL_PENCIL && !stroke_is_erase(&ink_s)); // ink keeps its seed whole
    stroke_free(ink_s);
    const TOPS: [u32; 5] = [0, 4, 11, 200, 255];
    let all = Rect { x: 0, y: 0, w: 600, h: 400 };
    let mut reference = Canvas::default();
    let mut c = Canvas::default();
    canvas_init(&mut reference, 600, 400);
    canvas_init(&mut c, 600, 400);
    for i in 0..5 {
        let mut s = stroke_new(TOOL_ERASER, &prm, TOPS[i] << 24 | 0x00BEEF);
        check!(stroke_render_tool(&s) == TOOL_ERASER);
        wave_points(&mut s, 50.0, 200.0, 550.0, 220.0, 60.0, 0.0, 1.0, 0.0, 0.0);
        let dst = if i != 0 { &mut c } else { &mut reference };
        canvas_fill(dst, all, 0);
        stroke_free(draw_live(dst, None, &s, None));
        if i != 0 {
            check!(diff_px(&reference, &c, all) == 0);
        }
        stroke_free(s);
    }
    canvas_free(&mut reference);
    canvas_free(&mut c);
}

// Every texture stays inside its stroke's bbox (what region re-renders and the paper copy rely
// on), with its roughest grain and laid on its side: the bbox is padded by the texture's reach.
fn erase_inside_bbox() {
    for t in 0..TOOL_COUNT as i32 {
        for side in 0..2 {
            let mut prm = ToolParams::default();
            tool_params_default(ToolKind(t), &mut prm);
            prm.grain = if side != 0 { 1.0 } else { prm.grain };
            let mut s = stroke_new(TOOL_ERASER, &prm, erase_seed(ToolKind(t), 4242 + t as u32));
            for i in 0..=60 {
                let f = i as f32 / 60.0;
                let tx = if side != 0 { 80.0 } else { 30.0 };
                let ty = if side != 0 { 10.0 } else { 30.0 };
                let q = PenSample { x: fmla(f, 1200.0, 300.0), y: fmla(200.0, (f * 5.0).sin(), 700.0), pressure: 1.0, tilt_x: tx, tilt_y: ty, t_ms: i as u32 * 5 };
                stroke_append(&mut s, &q, DPI);
            }
            let mut c = Canvas::default();
            canvas_init(&mut c, W, H);
            canvas_fill(&mut c, FULL, 0);
            stroke_free(draw_live(&mut c, None, &s, None));
            let (mut touched, mut outside) = (0i64, 0i64);
            for y in 0..H {
                for x in 0..W {
                    if c.px[(y * W + x) as usize] > 0 {
                        touched += 1;
                        outside += !rect_contains(s.bbox, x, y) as i64;
                    }
                }
            }
            check!(touched > 0 && outside == 0);
            if outside != 0 {
                eprintln!("  texture {} side {}: {} px outside the bbox", t, side, outside);
            }
            canvas_free(&mut c);
            stroke_free(s);
        }
    }
}

// A stroke erased whole leaves an all-white ink layer and the bare paper, and compaction drops it
// with its eraser.
fn fully_erased_stroke_leaves_nothing() {
    let mut pg = page_new(W, H, PAPER_GRAND);
    let s = ink(TOOL_FINELINER, 300.0, 1200.0, 1500.0, 1260.0, 40.0, 0.5, 1.0, 0.0, 0.0, 21);
    let spts = s.pts.clone();
    page_add(&mut pg, s);
    // The round eraser along the same points, from 60 px before the first to 60 px past the last.
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_ERASER, &mut prm);
    let mut e = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_ERASER, 22));
    let mut q = spts[0];
    q.x -= 60.0;
    q.pressure = 1.0;
    stroke_append(&mut e, &q, DPI);
    for k in 0..spts.len() {
        q = spts[k];
        q.pressure = 1.0;
        stroke_append(&mut e, &q, DPI);
    }
    q.x += 60.0;
    stroke_append(&mut e, &q, DPI);
    page_add(&mut pg, e);
    let mut c = Canvas::default();
    let mut paper = Canvas::default();
    canvas_init(&mut c, W, H);
    canvas_init(&mut paper, W, H);
    page_render_layers(&mut c, &pg, FULL, DPI, PAGE_LAYER_INK);
    let mut ink_left: i64 = 0;
    for i in 0..(W * H) as usize {
        ink_left += (c.px[i] != 255) as i64;
    }
    check!(ink_left == 0);
    page_render(&mut c, &pg, FULL, DPI);
    paper_render(&mut paper, PAPER_GRAND, FULL, DPI);
    check!(diff_px(&c, &paper, FULL) == 0);
    let mut drop = [false; 2];
    check!(page_droppable(&pg, DPI, &mut drop) == 2 && drop[0] && drop[1]);
    canvas_free(&mut c);
    canvas_free(&mut paper);
    page_free(pg);
}

// A reworked page: words erased whole, half, by two erasers together, a white pencil over a word,
// erasers on bare paper, and words written after the erasing.
const RW_A: usize = 0;
const RW_EA: usize = 1;
const RW_B: usize = 2;
const RW_EB: usize = 3;
const RW_C: usize = 4;
const RW_EC1: usize = 5;
const RW_EC2: usize = 6;
const RW_D: usize = 7;
const RW_ED: usize = 8;
const RW_E: usize = 9;
const RW_WP: usize = 10;
const RW_NA: usize = 11;
const RW_NB: usize = 12;
const RW_F: usize = 13;
const RW_G: usize = 14;
const RW_H: usize = 15;
const RW_COUNT: usize = 16;
fn reworked_page(k: PaperKind) -> Box<Page> {
    let mut pg = page_new(W, H, k);
    let mut s: Vec<Box<Stroke>> = Vec::new();
    s.push(ink(TOOL_PENCIL, 200.0, 400.0, 500.0, 400.0, 12.0, 0.6, 0.9, 0.0, 0.0, 1)); // RW_A
    s.push(rubber(170.0, 400.0, 530.0, 400.0, 1.0, 2)); // RW_EA
    s.push(ink(TOOL_BALLPOINT, 200.0, 600.0, 500.0, 600.0, 12.0, 0.6, 0.9, 0.0, 0.0, 3)); // RW_B
    s.push(rubber(170.0, 600.0, 530.0, 600.0, 1.0, 4)); // RW_EB
    s.push(ink(TOOL_HIGHLIGHTER, 200.0, 800.0, 500.0, 800.0, 0.0, 0.7, 0.7, 0.0, 0.0, 5)); // RW_C: too wide for one eraser
    s.push(rubber(150.0, 780.0, 550.0, 780.0, 1.0, 6)); // RW_EC1
    s.push(rubber(150.0, 820.0, 550.0, 820.0, 1.0, 7)); // RW_EC2
    s.push(ink(TOOL_FOUNTAIN, 200.0, 1000.0, 500.0, 1000.0, 12.0, 0.6, 0.9, 0.0, 0.0, 8)); // RW_D
    s.push(rubber(170.0, 1000.0, 350.0, 1000.0, 1.0, 9)); // RW_ED: half of it
    s.push(ink(TOOL_CHARCOAL, 200.0, 1200.0, 500.0, 1200.0, 12.0, 0.8, 1.0, 0.0, 0.0, 10)); // RW_E
    s.push(white(TOOL_PENCIL, 180.0, 1200.0, 520.0, 1200.0, 10.0, 0.9, 0.9, 20.0, 10.0, 11)); // RW_WP: leaves tooth
    s.push(rubber(900.0, 1400.0, 1500.0, 1450.0, 1.0, 12)); // RW_NA: bare paper
    s.push(white(TOOL_CHARCOAL, 900.0, 1600.0, 1500.0, 1650.0, 30.0, 0.8, 1.0, 30.0, 30.0, 13)); // RW_NB
    s.push(ink(TOOL_MARKER, 220.0, 400.0, 480.0, 410.0, 10.0, 0.8, 0.8, 0.0, 0.0, 14)); // RW_F: over the erased word A
    s.push(ink(TOOL_PENCIL, 900.0, 2000.0, 1500.0, 2000.0, 10.0, 0.0, 0.0, 0.0, 0.0, 15)); // RW_G: below the deadzone, no eraser near
    s.push(ink(TOOL_PENCIL, 950.0, 1420.0, 1450.0, 1430.0, 10.0, 0.0, 0.0, 0.0, 0.0, 16)); // RW_H: the same, over the earlier RW_NA
    for st in s {
        page_add(&mut pg, st);
    }
    pg
}

// A staff line of music paper (drawn into scratch) at x = 1000: the centre of its rows, which are
// darker than the highlighter's ink, and how many rows thick it is.
fn staff_line(scratch: &mut Canvas, thickness: &mut i32) -> f32 {
    paper_render(scratch, PAPER_MUSIC, FULL, DPI);
    let mut y0 = 0;
    let mut lt = 0;
    let mut y = 0;
    while y < H && lt == 0 {
        if scratch.px[(y * W + 1000) as usize] <= 110 {
            y0 = y;
            while scratch.px[((y0 + lt) * W + 1000) as usize] <= 110 {
                lt += 1;
            }
        }
        y += 1;
    }
    *thickness = lt;
    fmla(lt as f32, 0.5, y0 as f32)
}

// Compaction lists exactly what can go: the full render and the ink layer of the page without
// those strokes are byte for byte the page's own, and the page is left as it was.
fn compaction_drops_only_what_changes_nothing() {
    const KINDS: [PaperKind; 2] = [PAPER_MEDIUM, PAPER_GRAND];
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    canvas_init(&mut a, W, H);
    canvas_init(&mut b, W, H);
    for ki in 0..2 {
        let pg = reworked_page(KINDS[ki]);
        let mut before = [0usize; RW_COUNT];
        let mut boxes = [Rect::default(); RW_COUNT];
        for i in 0..RW_COUNT {
            before[i] = stroke_id(&pg.strokes[i]);
            boxes[i] = pg.strokes[i].bbox;
        }
        let mut drop = [false; RW_COUNT];
        let n = page_droppable(&pg, DPI, &mut drop);
        check!(n > 0);
        let mut marked = 0;
        for i in 0..RW_COUNT {
            marked += drop[i] as i32;
        }
        check!(marked == n);
        // The page is untouched.
        check!(pg.strokes.len() == RW_COUNT);
        for i in 0..RW_COUNT.min(pg.strokes.len()) {
            check!(stroke_id(&pg.strokes[i]) == before[i]);
            check!(pg.strokes[i].bbox == boxes[i]);
        }
        let v = page_without(&pg, &drop);
        page_render(&mut a, &pg, FULL, DPI);
        page_render(&mut b, &v, FULL, DPI);
        check!(diff_px(&a, &b, FULL) == 0);
        canvas_fill(&mut a, FULL, 255);
        canvas_fill(&mut b, FULL, 255);
        page_render_layers(&mut a, &pg, FULL, DPI, PAGE_LAYER_INK);
        page_render_layers(&mut b, &v, FULL, DPI, PAGE_LAYER_INK);
        check!(diff_px(&a, &b, FULL) == 0);
        view_free(v);
        if KINDS[ki] == PAPER_MEDIUM {
            // Gone: the words erased whole and their erasers, the erasers on bare paper. Kept: what
            // still shows, and ink no later eraser reaches even when it draws nothing (not erasing's
            // to drop), also over an earlier eraser.
            const GONE: [usize; 9] = [RW_A, RW_EA, RW_B, RW_EB, RW_C, RW_EC1, RW_EC2, RW_NA, RW_NB];
            const KEPT: [usize; 7] = [RW_D, RW_ED, RW_E, RW_WP, RW_F, RW_G, RW_H];
            for i in 0..GONE.len() {
                check!(drop[GONE[i]]);
            }
            for i in 0..KEPT.len() {
                check!(!drop[KEPT[i]]);
            }
            check!(n == GONE.len() as i32);
        }
        page_free(pg);
    }
    // A thin highlighter lying on a staff line darker than its ink draws nothing on the page, but
    // the ink layer shows it: it stays, though the page alone would let it go. An eraser just
    // below makes it a candidate (their boxes meet, their pixels do not).
    {
        let mut mp = page_new(W, H, PAPER_MUSIC);
        let mut lt = 0;
        let yc = staff_line(&mut a, &mut lt);
        check!((2..=4).contains(&lt));
        let mut hp = ToolParams::default();
        tool_params_default(TOOL_HIGHLIGHTER, &mut hp);
        hp.size_min_mm = 0.1;
        hp.size_max_mm = 0.1;
        let mut hl = stroke_new(TOOL_HIGHLIGHTER, &hp, 31);
        for i in 0..=50 {
            let q = PenSample { x: fmla(i as f32, 10.0, 700.0), y: yc, pressure: 0.7, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 5 };
            stroke_append(&mut hl, &q, DPI);
        }
        page_add(&mut mp, hl);
        page_add(&mut mp, rubber(650.0, yc + 46.0, 1250.0, yc + 46.0, 1.0, 32));
        check!(rect_overlaps(mp.strokes[0].bbox, mp.strokes[1].bbox));
        let none = [false, false];
        let first = [true, false];
        let mut md = [false; 2];
        let with = page_without(&mp, &none);
        let without = page_without(&mp, &first);
        page_render(&mut a, &with, FULL, DPI);
        page_render(&mut b, &without, FULL, DPI);
        check!(diff_px(&a, &b, FULL) == 0); // nothing on the page...
        canvas_fill(&mut a, FULL, 255);
        canvas_fill(&mut b, FULL, 255);
        page_render_layers(&mut a, &with, FULL, DPI, PAGE_LAYER_INK);
        page_render_layers(&mut b, &without, FULL, DPI, PAGE_LAYER_INK);
        check!(diff_px(&a, &b, FULL) > 500); // ...but a line on the ink layer
        check!(page_droppable(&mp, DPI, &mut md) >= 0 && !md[0]);
        let v = page_without(&mp, &md);
        view_free(with);
        canvas_fill(&mut b, FULL, 255);
        page_render_layers(&mut b, &v, FULL, DPI, PAGE_LAYER_INK);
        check!(diff_px(&a, &b, FULL) == 0);
        view_free(v);
        view_free(without);
        page_free(mp);
    }
    // A page without erase strokes has nothing to drop (and is never rendered for it).
    let mut plain = page_new(W, H, PAPER_MEDIUM);
    page_add(&mut plain, ink(TOOL_PENCIL, 100.0, 100.0, 300.0, 100.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1)); // below the deadzone: draws nothing
    page_add(&mut plain, ink(TOOL_PENCIL, 100.0, 100.0, 300.0, 100.0, 0.0, 0.8, 0.8, 0.0, 0.0, 2));
    let mut d2 = [true, true];
    check!(page_droppable(&plain, DPI, &mut d2) == 0 && !d2[0] && !d2[1]);
    page_free(plain);
    canvas_free(&mut a);
    canvas_free(&mut b);
}

// The compaction rule the slow way: in stroke order, ink first, then erase strokes, a stroke on the
// page is marked when the page and its ink layer, drawn whole without it and the strokes marked
// before it, are what they are without those alone. Only ink a later erase stroke's bbox reaches
// is a candidate.
fn droppable_slowly(p: &Page, drop: &mut [bool]) -> i32 {
    let mut n = 0;
    let mut any = false;
    for i in 0..p.strokes.len() {
        drop[i] = false;
        any = any || stroke_is_erase(&p.strokes[i]);
    }
    if !any {
        return 0;
    }
    let all = Rect { x: 0, y: 0, w: p.w, h: p.h };
    // the page and its ink layer, without the marked strokes / and stroke i
    let mut with = [Canvas::default(), Canvas::default()];
    let mut without = [Canvas::default(), Canvas::default()];
    for l in 0..2 {
        canvas_init(&mut with[l], p.w, p.h);
        canvas_init(&mut without[l], p.w, p.h);
    }
    page_render(&mut with[0], p, all, DPI);
    page_render_layers(&mut with[1], p, all, DPI, PAGE_LAYER_INK);
    for pass in 0..2 {
        for i in 0..p.strokes.len() {
            let s = &p.strokes[i];
            if (s.tool.0 as u32) >= TOOL_COUNT as u32 || stroke_is_erase(s) != (pass == 1) {
                continue;
            }
            let mut later = false;
            for j in i + 1..p.strokes.len() {
                later = later || (stroke_is_erase(&p.strokes[j]) && rect_overlaps(p.strokes[j].bbox, s.bbox));
            }
            if (pass == 0 && !later) || rect_is_empty(rect_intersect(s.bbox, all)) {
                continue;
            }
            drop[i] = true;
            let v = page_without(p, drop);
            page_render(&mut without[0], &v, all, DPI);
            canvas_fill(&mut without[1], all, 255);
            page_render_layers(&mut without[1], &v, all, DPI, PAGE_LAYER_INK);
            view_free(v);
            drop[i] = diff_px(&with[0], &without[0], all) == 0 && diff_px(&with[1], &without[1], all) == 0;
            n += drop[i] as i32;
        }
    }
    for l in 0..2 {
        canvas_free(&mut with[l]);
        canvas_free(&mut without[l]);
    }
    n
}

thread_local! {
    static G_RAND: Cell<u32> = const { Cell::new(1) };
}
fn rnd() -> u32 {
    G_RAND.with(|g| {
        let mut r = g.get();
        r ^= r << 13;
        r ^= r >> 17;
        r ^= r << 5;
        g.set(r);
        r
    })
}
fn frand(lo: f32, hi: f32) -> f32 {
    lo + ((hi - lo) * (rnd() % 10001) as f32) / 10000.0
}

// A random stroke over (and off) a w x h page: ink of any tool, or an erase stroke of any texture
// (soft, grainy and laid on its side at times), wandering from a random start.
fn random_stroke(w: i32, h: i32, erase: bool) -> Box<Stroke> {
    let mut t = ToolKind((rnd() % TOOL_COUNT as u32) as i32);
    if !erase && t == TOOL_ERASER {
        t = TOOL_MARKER;
    }
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    if rnd() % 3 == 0 {
        prm.size_min_mm = frand(0.1, 8.0);
        prm.size_max_mm = prm.size_min_mm;
    }
    if rnd() % 5 == 0 {
        prm.opacity_min = frand(0.2, 1.0);
        prm.opacity_max = prm.opacity_min;
    }
    if rnd() % 5 == 0 {
        prm.grain = frand(0.0, 1.0);
    }
    let seed = if erase { erase_seed(t, rnd()) } else { rnd() & 0xFFFFFF };
    let mut s = stroke_new(if erase { TOOL_ERASER } else { t }, &prm, seed);
    let mut x = frand(-60.0, w as f32 + 60.0);
    let mut y = frand(-60.0, h as f32 + 60.0);
    let mut ang = frand(0.0, 6.28);
    let step = frand(3.0, 30.0);
    let mut pr = frand(0.0, 1.0);
    let tx = frand(-70.0, 70.0);
    let ty = frand(-70.0, 70.0);
    let first = rnd();
    let n = 1 + (first % (if rnd() % 4 != 0 { 40 } else { 160 })) as i32;
    for k in 0..n {
        let q = PenSample { x, y, pressure: pr, tilt_x: tx, tilt_y: ty, t_ms: k as u32 * 6 };
        stroke_append(&mut s, &q, DPI);
        ang += frand(-0.7, 0.7);
        x = fmla(ang.cos(), step, x);
        y = fmla(ang.sin(), step, y);
        pr = clampf(pr + frand(-0.1, 0.1), 0.0, 1.0);
    }
    s
}

// Compaction marks exactly what the rule done the slow way marks (so the page without the marked
// strokes renders as before), on random pages of ink and erase strokes of every kind, on blank,
// ruled, dotted, music and graph paper. The fast way draws only the pieces of strokes near what it
// checks, and the ink layer only where the paper has marks: this is what says that changes nothing.
fn compaction_is_the_rule() {
    const KINDS: [PaperKind; 5] = [PAPER_BLANK, PAPER_MEDIUM, PAPER_DOTS, PAPER_MUSIC, PAPER_GRID];
    let mut marked: i64 = 0;
    for it in 0..16u32 {
        G_RAND.with(|g| g.set(0x9E3779B9u32.wrapping_add(it.wrapping_mul(2654435761))));
        let w = 250 + (rnd() % 300) as i32;
        let h = 250 + (rnd() % 400) as i32;
        let mut p = page_new(w, h, KINDS[(it % 5) as usize]);
        let rounds = 1 + (rnd() % 3) as i32;
        for _round in 0..rounds {
            let ni = 3 + (rnd() % 12) as i32;
            for _ in 0..ni {
                page_add(&mut p, random_stroke(w, h, false));
            }
            let ne = 1 + (rnd() % 6) as i32;
            for _ in 0..ne {
                page_add(&mut p, random_stroke(w, h, true));
            }
        }
        let mut fast = vec![false; p.strokes.len()];
        let mut slow = vec![false; p.strokes.len()];
        let nf = page_droppable(&p, DPI, &mut fast);
        let ns = droppable_slowly(&p, &mut slow);
        check!(nf == ns && fast == slow);
        if nf != ns {
            eprintln!("  page {}: {} marked, the slow way {}", it, nf, ns);
        }
        marked += nf as i64;
        page_free(p);
    }
    check!(marked > 20); // the pages gave it plenty to mark
}

// Ink the page shows and the ink layer does not: a light pencil line along a staff line, under a
// highlighter at full strength. On the ink layer the highlighter buries the pencil (both settle on
// the same gray), but on the page the staff line is darker than the highlighter's ink, which leaves
// it alone, so there the pencil still darkens the line. An eraser after them reaches into the
// pencil's box without touching it, which makes the pencil a candidate: only the page's check
// keeps it.
fn compaction_checks_the_page_too() {
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    canvas_init(&mut a, W, H);
    canvas_init(&mut b, W, H);
    let mut lt = 0;
    let yc = staff_line(&mut a, &mut lt);
    check!((2..=4).contains(&lt));
    // This contrast needs the original light pencil, not a later darker writing profile.
    let pp = legacy(TOOL_PENCIL);
    let mut hp = legacy(TOOL_HIGHLIGHTER);
    hp.opacity_min = 1.0;
    hp.opacity_max = 1.0;
    let mut pencil = stroke_new(TOOL_PENCIL, &pp, 41);
    let mut hl = stroke_new(TOOL_HIGHLIGHTER, &hp, 42);
    for i in 0..=50 {
        let q = PenSample { x: fmla(i as f32, 10.0, 700.0), y: yc, pressure: 0.3, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 5 };
        stroke_append(&mut pencil, &q, DPI);
    }
    // The narrow vertical tip needs repeated passes to bury the light pencil completely.
    for _pass in 0..3 {
        for i in 0..=80 {
            let q = PenSample { x: fmla(i as f32, 10.0, 600.0), y: yc, pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 5 };
            stroke_append(&mut hl, &q, DPI);
        }
    }
    let mut mp = page_new(W, H, PAPER_MUSIC);
    page_add(&mut mp, pencil);
    page_add(&mut mp, hl);
    page_add(&mut mp, rubber(1250.0, yc, 1500.0, yc, 1.0, 43));
    check!(rect_overlaps(mp.strokes[0].bbox, mp.strokes[2].bbox));
    // Without the pencil: another page, the same ink layer.
    let none = [false, false, false];
    let first = [true, false, false];
    let mut md = [false; 3];
    let with = page_without(&mp, &none);
    let without = page_without(&mp, &first);
    page_render(&mut a, &with, FULL, DPI);
    page_render(&mut b, &without, FULL, DPI);
    check!(diff_px(&a, &b, FULL) > 20);
    canvas_fill(&mut a, FULL, 255);
    canvas_fill(&mut b, FULL, 255);
    page_render_layers(&mut a, &with, FULL, DPI, PAGE_LAYER_INK);
    page_render_layers(&mut b, &without, FULL, DPI, PAGE_LAYER_INK);
    check!(diff_px(&a, &b, FULL) == 0);
    check!(page_droppable(&mp, DPI, &mut md) == 0 && !md[0] && !md[1] && !md[2]);
    view_free(with);
    view_free(without);
    page_free(mp);
    canvas_free(&mut a);
    canvas_free(&mut b);
}

// BrushState.restored tells whether a stroke changed the canvas it drew on, the page. An eraser
// over ink the paper hides there (a thin highlighter on a staff line darker than its ink) reports
// no change, yet it changes the ink layer: page_droppable, which checks both, keeps it. It, not
// `restored`, decides what a page may leave out.
fn restored_is_the_page_only() {
    let mut paper = Canvas::default();
    let mut live = Canvas::default();
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    canvas_init(&mut paper, W, H);
    canvas_init(&mut live, W, H);
    canvas_init(&mut a, W, H);
    canvas_init(&mut b, W, H);
    let mut lt = 0;
    let yc = staff_line(&mut paper, &mut lt);
    check!((2..=4).contains(&lt));
    canvas_blit(&mut live, &paper, FULL, 0, 0);
    let mut hp = ToolParams::default();
    let mut ep = ToolParams::default();
    tool_params_default(TOOL_HIGHLIGHTER, &mut hp);
    hp.size_min_mm = 0.1;
    hp.size_max_mm = 0.1;
    tool_params_default(TOOL_ERASER, &mut ep);
    ep.size_min_mm = 0.1;
    ep.size_max_mm = 0.1; // the precise eraser at its smallest, along the same points
    let mut hl = stroke_new(TOOL_HIGHLIGHTER, &hp, 51);
    let mut er = stroke_new(TOOL_ERASER, &ep, erase_seed(TOOL_ERASER, 52));
    for i in 0..=50 {
        let q = PenSample { x: fmla(i as f32, 10.0, 700.0), y: yc, pressure: 0.7, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 5 };
        stroke_append(&mut hl, &q, DPI);
        stroke_append(&mut er, &q, DPI);
    }
    let mut mp = page_new(W, H, PAPER_MUSIC);
    let mut restored = true;
    let no_eraser = [false, true];
    let mut md = [false; 2];
    page_add(&mut mp, draw_live(&mut live, Some(&paper), &hl, None));
    page_add(&mut mp, draw_live(&mut live, Some(&paper), &er, Some(&mut restored)));
    check!(!restored);
    check!(diff_px(&live, &paper, FULL) == 0); // nothing changed on the page...
    canvas_fill(&mut a, FULL, 255);
    page_render_layers(&mut a, &mp, FULL, DPI, PAGE_LAYER_INK);
    let v = page_without(&mp, &no_eraser);
    canvas_fill(&mut b, FULL, 255);
    page_render_layers(&mut b, &v, FULL, DPI, PAGE_LAYER_INK);
    check!(diff_px(&a, &b, FULL) > 100); // ...but the eraser changed the ink layer
    check!(page_droppable(&mp, DPI, &mut md) == 0 && !md[0] && !md[1]);
    view_free(v);
    page_free(mp);
    stroke_free(hl);
    stroke_free(er);
    canvas_free(&mut paper);
    canvas_free(&mut live);
    canvas_free(&mut a);
    canvas_free(&mut b);
}

// The page and its ink layer must be drawn in one call: the ink layer alone restores toward white,
// so drawing it over the paper in a second pass wipes the lines under every erase stroke.
fn one_call_keeps_the_lines() {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    for i in 0..5 {
        let fi = i as f32;
        page_add(&mut pg, ink(TOOL_BALLPOINT, 200.0, fmla(fi, 90.0, 500.0), 1600.0, fmla(fi, 90.0, 520.0), 30.0, 0.8, 0.8, 0.0, 0.0, 1 + i as u32));
    }
    let er = rubber(800.0, 400.0, 820.0, 1000.0, 1.0, 9);
    page_add(&mut pg, er.clone());
    let mut one = Canvas::default();
    let mut two = Canvas::default();
    let mut paper = Canvas::default();
    canvas_init(&mut one, W, H);
    canvas_init(&mut two, W, H);
    canvas_init(&mut paper, W, H);
    page_render(&mut one, &pg, FULL, DPI);
    page_render_layers(&mut two, &pg, FULL, DPI, PAGE_LAYER_PAPER);
    page_render_layers(&mut two, &pg, FULL, DPI, PAGE_LAYER_INK);
    paper_render(&mut paper, PAPER_MEDIUM, FULL, DPI);
    let (mut lines, mut kept, mut wiped) = (0i64, 0i64, 0i64);
    let lim = rubber_core();
    for y in 400..1000 {
        for x in 760..860 {
            if line_dist(&er, x, y) > lim || paper.px[(y * W + x) as usize] == 255 {
                continue;
            }
            lines += 1;
            kept += (one.px[(y * W + x) as usize] == paper.px[(y * W + x) as usize]) as i64;
            wiped += (two.px[(y * W + x) as usize] == 255) as i64;
        }
    }
    check!(lines > 100 && kept == lines && wiped == lines);
    canvas_free(&mut one);
    canvas_free(&mut two);
    canvas_free(&mut paper);
    page_free(pg);
}

// BrushState.restored says whether an erase stroke changed the canvas it drew on: never on bare
// paper, always over ink, with or without the fast path.
fn restored_tells_a_no_op() {
    const PW: i32 = 600;
    const PH: i32 = 400;
    let all = Rect { x: 0, y: 0, w: PW, h: PH };
    let mut paper = Canvas::default();
    let mut c = Canvas::default();
    canvas_init(&mut paper, PW, PH);
    canvas_init(&mut c, PW, PH);
    for fast in 0..2 {
        set_erase_spans(fast != 0);
        for t in 0..TOOL_COUNT as i32 {
            paper_render(&mut paper, PAPER_MEDIUM, all, DPI);
            canvas_blit(&mut c, &paper, all, 0, 0);
            let w = white(ToolKind(t), 50.0, 200.0, 550.0, 210.0, 40.0, 0.0, 1.0, 30.0, 30.0, 5);
            let mut restored = true;
            stroke_free(draw_live(&mut c, Some(&paper), &w, Some(&mut restored)));
            check!(!restored);
            check!(diff_px(&c, &paper, all) == 0);
            let m = ink(TOOL_MARKER, 50.0, 200.0, 550.0, 210.0, 40.0, 1.0, 1.0, 0.0, 0.0, 6);
            stroke_free(draw_live(&mut c, Some(&paper), &m, None));
            stroke_free(draw_live(&mut c, Some(&paper), &w, Some(&mut restored)));
            check!(restored);
            stroke_free(m);
            stroke_free(w);
        }
    }
    set_erase_spans(true);
    canvas_free(&mut paper);
    canvas_free(&mut c);
}

// An erase stroke is an ordinary stroke to undo and redo: undo gives back the page as it was
// before it, redo the page with it.
fn undo_and_redo_an_erase_stroke() {
    let mut p = page_new(W, H, PAPER_MEDIUM);
    let mut h = History::default();
    history_init(&mut h);
    for i in 0..4 {
        let y = 600.0 + i as f32 * 100.0;
        // Broad ink gives the erase/undo comparison plenty of changed pixels.
        let mut s = stroke_new(TOOL_MARKER, &legacy(TOOL_MARKER), 1 + i as u32);
        wave_points(&mut s, 300.0, y, 1500.0, y + 40.0, 30.0, 0.6, 1.0, 30.0, 20.0);
        history_push_add(&mut h, p.strokes.len() as i32, &s);
        page_add(&mut p, s);
    }
    let mut before = Canvas::default();
    let mut after = Canvas::default();
    let mut cur = Canvas::default();
    canvas_init(&mut before, W, H);
    canvas_init(&mut after, W, H);
    canvas_init(&mut cur, W, H);
    page_render(&mut before, &p, FULL, DPI);
    // A fixed broad erase profile makes this an undo test independent of fresh defaults.
    let mut e = stroke_new(TOOL_ERASER, &legacy(TOOL_CHARCOAL), erase_seed(TOOL_CHARCOAL, 5));
    wave_points(&mut e, 600.0, 500.0, 1100.0, 1100.0, 80.0, 0.8, 1.0, 30.0, 30.0);
    history_push_add(&mut h, p.strokes.len() as i32, &e);
    page_add(&mut p, e);
    page_render(&mut after, &p, FULL, DPI);
    check!(diff_px(&before, &after, FULL) > 1000);
    canvas_blit(&mut cur, &after, FULL, 0, 0);
    let mut d = Rect::default();
    check!(history_undo(&mut h, &mut p, &mut d));
    page_render(&mut cur, &p, d, DPI);
    check!(diff_px(&cur, &before, FULL) == 0);
    check!(history_redo(&mut h, &mut p, &mut d));
    page_render(&mut cur, &p, d, DPI);
    check!(diff_px(&cur, &after, FULL) == 0);
    history_free(&mut h);
    page_free(p);
    canvas_free(&mut before);
    canvas_free(&mut after);
    canvas_free(&mut cur);
}

// The frozen 1.0.0-beta reader opens every page with erase strokes (version 1, or 2 for custom
// music), keeps tool 3 and every seed byte for byte, and its re-save brings each texture back here
// as it was. It draws erase strokes as nothing, so there the erased ink shows again.
fn beta_opens_erase_strokes() {
    let m = MusicLayout { systems: 2, n: 2, part: [PART_TREBLE, PART_BASS, PART_NONE, PART_NONE, PART_NONE, PART_NONE] };
    let kinds = [PAPER_MEDIUM, music_encode(&m)];
    let mut ours = Canvas::default();
    let mut back = Canvas::default();
    canvas_init(&mut ours, W, H);
    canvas_init(&mut back, W, H);
    for ki in 0..2 {
        let mut pg = page_new(W, H, kinds[ki]);
        for t in 0..TOOL_COUNT as i32 {
            if ToolKind(t) != TOOL_ERASER {
                let ft = t as f32;
                page_add(&mut pg, ink(ToolKind(t), 100.0, fmla(ft, 240.0, 200.0), 1700.0, fmla(ft, 240.0, 260.0), 60.0, 0.3, 1.0, 30.0, 20.0, 1 + t as u32));
            }
        }
        for t in 0..TOOL_COUNT as i32 {
            page_add(&mut pg, white(ToolKind(t), (150 + t * 170) as f32, 100.0, (200 + t * 170) as f32, 2400.0, 30.0, 0.5, 1.0, 35.0, 30.0, 0xF000 + t as u32));
        }
        let mut d: Vec<u8> = Vec::new();
        check!(page_serialize(&pg, &mut d) == 0 && !d.is_empty());
        if d.is_empty() {
            continue;
        }
        check!((d[4] as i32 | (d[5] as i32) << 8) == if ki != 0 { 2 } else { 1 });
        let mut bp = beta_format::BetaPage::default();
        check!(beta_format::beta_page_deserialize(&d, &mut bp) == 0);
        check!(bp.strokes.len() == pg.strokes.len());
        for i in 0..bp.strokes.len().min(pg.strokes.len()) {
            check!(bp.strokes[i].tool == pg.strokes[i].tool.0 as u8 && bp.strokes[i].seed == pg.strokes[i].seed);
        }
        let mut d2: Vec<u8> = Vec::new();
        check!(beta_format::beta_resave(&d, &mut d2) == 0 && !d2.is_empty());
        let mut q: Option<Box<Page>> = None;
        let mut q0: Option<Box<Page>> = None;
        check!(!d2.is_empty() && page_deserialize(&d2, &mut q) == 0 && q.as_ref().is_some_and(|q| q.strokes.len() == pg.strokes.len()));
        check!(page_deserialize(&d, &mut q0) == 0 && q0.is_some()); // ours, read back (a save rounds pressure and tilt)
        if let Some(q0) = q0 {
            page_render(&mut ours, &q0, FULL, DPI);
            page_free(q0);
        }
        if let Some(q) = q {
            for i in 0..q.strokes.len().min(pg.strokes.len()) {
                check!(q.strokes[i].seed == pg.strokes[i].seed);
                check!(stroke_render_tool(&q.strokes[i]) == stroke_render_tool(&pg.strokes[i]));
            }
            page_render(&mut back, &q, FULL, DPI);
            check!(diff_px(&ours, &back, FULL) == 0);
            page_free(q);
        }
        // As 1.0.0-beta shows it: the erase strokes draw nothing (its brush_add returns at once
        // for tool 3; test_golden holds its ink), so the erased ink is back.
        let drop: Vec<bool> = pg.strokes.iter().map(|s| stroke_is_erase(s)).collect();
        let beta_view = page_without(&pg, &drop);
        page_render(&mut back, &beta_view, FULL, DPI);
        check!(diff_px(&ours, &back, FULL) > 10000);
        view_free(beta_view);
        page_free(pg);
    }
    canvas_free(&mut ours);
    canvas_free(&mut back);
}

// The texture is known before the first point everywhere a stroke is built: the reader gives every
// stroke the bbox it was drawn with, and a page opened at another size (scale_page) pads each
// erase stroke by its texture's reach.
fn reader_and_rescale_know_the_texture() {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    for t in 0..TOOL_COUNT as i32 {
        // Fixed historical widths establish that charcoal can reach beyond the round eraser.
        let mut s = stroke_new(TOOL_ERASER, &legacy(ToolKind(t)), erase_seed(ToolKind(t), 9 + t as u32));
        wave_points(&mut s, (150 + t * 170) as f32, 300.0, (200 + t * 170) as f32, 2000.0, 30.0, 0.5, 1.0, 80.0, 10.0);
        page_add(&mut pg, s);
    }
    let mut d: Vec<u8> = Vec::new();
    check!(page_serialize(&pg, &mut d) == 0);
    let mut q: Option<Box<Page>> = None;
    check!(!d.is_empty() && page_deserialize(&d, &mut q) == 0 && q.as_ref().is_some_and(|q| q.strokes.len() == pg.strokes.len()));
    if let Some(q) = &q {
        for i in 0..q.strokes.len().min(pg.strokes.len()) {
            check!(q.strokes[i].bbox == pg.strokes[i].bbox);
        }
    }
    // White charcoal laid on its side reaches further than the round tip: padded by charcoal.
    check!(pg.strokes[TOOL_CHARCOAL.0 as usize].bbox.w > pg.strokes[TOOL_ERASER.0 as usize].bbox.w + 20);
    if let Some(q) = q {
        page_free(q);
    }
    page_free(pg);

    let root = cformat!("%s/erase-%d", tl_tmp().as_str(), std::process::id() as i32);
    // (the C's rm -rf, which succeeds when there is nothing to remove)
    check!(std::fs::remove_dir_all(&root).is_ok() || std::fs::symlink_metadata(&root).is_err());
    check!(store_init(&root) == 0);
    store_set_device(DPI, W, H);
    let mut nb = NotebookInfo::default();
    check!(store_create("Erase strokes", "", PAPER_MEDIUM, &mut nb) == 0);
    let mut half = page_new(W / 2, H / 2, PAPER_MEDIUM);
    for t in 0..TOOL_COUNT as i32 {
        let mut prm = ToolParams::default();
        tool_params_default(ToolKind(t), &mut prm);
        prm.grain = 1.0;
        let mut s = stroke_new(TOOL_ERASER, &prm, erase_seed(ToolKind(t), 3 + t as u32));
        wave_points(&mut s, (60 + t * 85) as f32, 100.0, (90 + t * 85) as f32, 1100.0, 20.0, 0.5, 1.0, 80.0, 10.0);
        page_add(&mut half, s);
    }
    check!(store_save_page(&nb, 0, &half) == 0);
    let mut big: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 0, W, H, &mut big) == 0 && big.as_ref().is_some_and(|b| b.strokes.len() == TOOL_COUNT && b.w == W));
    let mut c = Canvas::default();
    canvas_init(&mut c, W, H);
    if let Some(big) = &big {
        for i in 0..big.strokes.len() {
            let s = &big.strokes[i];
            // The same bbox as a stroke built at the new size from the start.
            let mut fresh = stroke_new(s.tool, &s.params, s.seed);
            for k in 0..s.pts.len() {
                stroke_append(&mut fresh, &s.pts[k], DPI);
            }
            check!(fresh.bbox == s.bbox);
            stroke_free(fresh);
            canvas_fill(&mut c, FULL, 0);
            stroke_free(draw_live(&mut c, None, s, None));
            let (mut touched, mut outside) = (0i64, 0i64);
            for y in 0..H {
                for x in 0..W {
                    if c.px[(y * W + x) as usize] > 0 {
                        touched += 1;
                        outside += !rect_contains(s.bbox, x, y) as i64;
                    }
                }
            }
            check!(touched > 0 && outside == 0);
        }
    }
    canvas_free(&mut c);
    if let Some(big) = big {
        page_free(big);
    }
    page_free(half);
    check!(std::fs::remove_dir_all(&root).is_ok());
}

// ---- the look of erase strokes, recorded ----

// GRIMOIRE_GOLDEN_PRINT=1 prints the table below instead of checking it (that is how it was made),
// as in test_golden.c. FORMAT.md freezes how an erase stroke draws from 1.1 on, and until a
// released version has drawn one, this table is what keeps a change to it from passing unnoticed:
// re-record it only for a change meant to alter that look, before 1.1 ships; never afterwards, and
// never to make another change pass. The values hold for the host build this suite runs on (clang
// on arm64 macOS); elsewhere floats may round differently, so there the comparison is skipped.
const GOLDEN_HOST: bool = cfg!(all(target_arch = "aarch64", target_os = "macos"));

// Recorded on 2026-10-20 from the 1.1 engine (lane inkcore).
const K_LOOK: [(&str, u32); 48] = [
    ("medium rule, white pencil", 0x807def5d),
    ("medium rule, white pencil, ink", 0x0ca259ea),
    ("medium rule, white mechanical pencil", 0x20505b6e),
    ("medium rule, white mechanical pencil, ink", 0xc3f29e67),
    ("medium rule, white ballpoint", 0xfa3c4357),
    ("medium rule, white ballpoint, ink", 0x140df6f3),
    ("medium rule, round tip", 0x7de3880b),
    ("medium rule, round tip, ink", 0x6533900b),
    ("medium rule, white highlighter", 0x800f1d04),
    ("medium rule, white highlighter, ink", 0x2b56527c),
    ("medium rule, white fountain pen", 0x17a61725),
    ("medium rule, white fountain pen, ink", 0x32a41ca7),
    ("medium rule, white fineliner", 0x73b1784e),
    ("medium rule, white fineliner, ink", 0x1754e09f),
    ("medium rule, white marker", 0xbab3dfee),
    ("medium rule, white marker, ink", 0x3c47f3e8),
    ("medium rule, white charcoal", 0x9f243fbd),
    ("medium rule, white charcoal, ink", 0xb3961079),
    ("medium rule, white calligraphy pen", 0xb1fc2850),
    ("medium rule, white calligraphy pen, ink", 0x5cec3bd2),
    ("medium rule, soft round tip", 0x621d8e19),
    ("medium rule, soft round tip, ink", 0x36779c1e),
    ("medium rule, fine round tip", 0xf477bc76),
    ("medium rule, fine round tip, ink", 0xacafd213),
    ("grand staff, white pencil", 0x5ab58951),
    ("grand staff, white pencil, ink", 0x0ca259ea),
    ("grand staff, white mechanical pencil", 0x639afd54),
    ("grand staff, white mechanical pencil, ink", 0xc3f29e67),
    ("grand staff, white ballpoint", 0xc2109e2a),
    ("grand staff, white ballpoint, ink", 0x140df6f3),
    ("grand staff, round tip", 0xd7a8a412),
    ("grand staff, round tip, ink", 0x6533900b),
    ("grand staff, white highlighter", 0x76ea94e2),
    ("grand staff, white highlighter, ink", 0x2b56527c),
    ("grand staff, white fountain pen", 0x00c8770c),
    ("grand staff, white fountain pen, ink", 0x32a41ca7),
    ("grand staff, white fineliner", 0x3e3fa675),
    ("grand staff, white fineliner, ink", 0x1754e09f),
    ("grand staff, white marker", 0x96a6610f),
    ("grand staff, white marker, ink", 0x3c47f3e8),
    ("grand staff, white charcoal", 0x8dde2f93),
    ("grand staff, white charcoal, ink", 0xb3961079),
    ("grand staff, white calligraphy pen", 0x5443ff59),
    ("grand staff, white calligraphy pen, ink", 0x5cec3bd2),
    ("grand staff, soft round tip", 0xa8a7d6c1),
    ("grand staff, soft round tip, ink", 0x36779c1e),
    ("grand staff, fine round tip", 0x18e2011b),
    ("grand staff, fine round tip, ink", 0xacafd213),
];

fn g_print() -> bool {
    std::env::var_os("GRIMOIRE_GOLDEN_PRINT").is_some()
}

fn crc32_bytes(crc: u32, p: &[u8]) -> u32 {
    static CRC_TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = CRC_TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for i in 0..256u32 {
            let mut c = i;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
            }
            t[i as usize] = c;
        }
        t
    });
    let mut crc = !crc;
    for &b in p {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

fn crc_rect(c: &Canvas, r: Rect) -> u32 {
    let mut crc = 0;
    for y in r.y..r.y + r.h {
        let at = (y * c.stride + r.x) as usize;
        crc = crc32_bytes(crc, &c.px[at..at + r.w as usize]);
    }
    crc
}

fn look(name: &str, crc: u32) {
    if g_print() {
        println!("    (\"{}\", 0x{:08x}),", name, crc);
        return;
    }
    if !GOLDEN_HOST {
        return;
    }
    // White ink in the highlighter's texture drawn with the vertical tip (TIP_HIGHLIGHTER_VERTICAL), as
    // strokes drawn since 2.0.0-beta.5 record it: what 2.0.0-beta.2 to beta.4 drew for every such
    // stroke (issue #5). The table above is the round tip, which older strokes (no tip recorded) get.
    const VERTICAL: [(&str, u32); 4] = [
            ("medium rule, white highlighter, vertical tip", 0x1d8309ca),
            ("medium rule, white highlighter, vertical tip, ink", 0x14d61db0),
            ("grand staff, white highlighter, vertical tip", 0x0a4bc45d),
            ("grand staff, white highlighter, vertical tip, ink", 0x14d61db0),
        ];
    for &(n, expected) in VERTICAL.iter().chain(K_LOOK.iter()) {
        if n != name {
            continue;
        }
        check!(expected == crc);
        if expected != crc {
            eprintln!("  {}: 0x{:08x}, recorded 0x{:08x}", name, crc, expected);
        }
        return;
    }
    eprintln!("  {}: not in the table", name);
    check!(false); // every render has a recorded value
}

// test_golden.c's stroke: a zigzag whose pressure starts below every deadzone, swells to full and
// fades again, the pen leaning from left to right and nodding. Plain arithmetic only.
fn zigzag(tool: ToolKind, prm: &ToolParams, seed: u32, y0: f32) -> Box<Stroke> {
    let mut s = stroke_new(tool, prm, seed);
    const N: i32 = 161;
    for i in 0..N {
        let f = i as f32 / (N - 1) as f32;
        let tri = ((f * 6.0) % 2.0 - 1.0).abs();
        let bell = fmla(-fmla(2.0, f, -1.0), fmla(2.0, f, -1.0), 1.0);
        let q = PenSample {
            x: fmla(1420.0, f, 220.0),
            y: fmla(60.0 * f, f, fmla(170.0, tri, y0) - 85.0),
            pressure: if i < 3 { 0.0 } else { fmla(0.96, bell, 0.04) },
            tilt_x: fmla(120.0, f, -60.0),
            tilt_y: fmla(50.0, tri, -15.0),
            t_ms: i as u32 * 7,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// The erase stroke across that ink: a slower wave the other way, pressing hardest in the middle,
// the pen leaning from right to left (so the calligraphy nib turns and charcoal lies on its side).
fn wave(prm: &ToolParams, seed: u32, y0: f32) -> Box<Stroke> {
    let mut s = stroke_new(TOOL_ERASER, prm, seed);
    const N: i32 = 121;
    for i in 0..N {
        let f = i as f32 / (N - 1) as f32;
        let bell = fmla(-fmla(2.0, f, -1.0), fmla(2.0, f, -1.0), 1.0);
        let q = PenSample {
            x: fmla(-1460.0, f, 1660.0),
            y: fmla(110.0, (f * 7.0).sin(), y0),
            pressure: if i < 2 { 0.0 } else { fmla(0.9, bell, 0.1) },
            tilt_x: fmla(-130.0, f, 65.0),
            tilt_y: fmla(-(40.0 * f), f, 20.0),
            t_ms: i as u32 * 9,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// The erase strokes recorded: every texture (TOOL_ERASER: the round tip, with its default 2.5 to
// 7 mm), the round tip at half strength (the per-pixel path) and at its smallest.
const LOOK_SOFT: usize = TOOL_COUNT;
const LOOK_FINE: usize = TOOL_COUNT + 1;
const LOOK_COUNT: usize = TOOL_COUNT + 2;
const K_LOOK_KEY: [&str; LOOK_COUNT] = [
    "white pencil",
    "white mechanical pencil",
    "white ballpoint",
    "round tip",
    "white highlighter",
    "white fountain pen",
    "white fineliner",
    "white marker",
    "white charcoal",
    "white calligraphy pen",
    "soft round tip",
    "fine round tip",
];

// How erase strokes draw: each one over ink of five tools, on ruled paper and a grand staff, the
// page and its ink layer, compared with the recorded table (and each one erases plenty).
fn erase_look_is_recorded() {
    const PAPERS: [PaperKind; 2] = [PAPER_MEDIUM, PAPER_GRAND];
    const UNDER: [ToolKind; 5] = [TOOL_MARKER, TOOL_PENCIL, TOOL_CHARCOAL, TOOL_FOUNTAIN, TOOL_BALLPOINT];
    let y0: f32 = 1240.0;
    let band = Rect { x: 0, y: 980, w: W, h: 520 };
    let mut c = Canvas::default();
    let mut inked = Canvas::default();
    check!(canvas_init(&mut c, W, H) == 0);
    check!(canvas_init(&mut inked, W, H) == 0);
    for pi in 0..2 {
        for v in 0..LOOK_COUNT {
            let mut pg = page_new(W, H, PAPERS[pi]);
            for u in 0..5 {
                let ip = legacy(UNDER[u]);
                page_add(&mut pg, zigzag(UNDER[u], &ip, 0x00A5F00D + u as u32 * 40503, fmla(30.0, u as f32, y0 - 60.0)));
            }
            if v == 0 && !g_print() {
                page_render(&mut inked, &pg, band, DPI); // the ink before any erasing
            }
            let tex = if v < TOOL_COUNT { ToolKind(v as i32) } else { TOOL_ERASER };
            let mut prm = legacy(tex);
            if v == LOOK_SOFT {
                prm.opacity_min = 0.5;
                prm.opacity_max = 0.5;
            }
            if v == LOOK_FINE {
                prm.size_min_mm = 0.10;
                prm.size_max_mm = 0.10;
            }
            page_add(&mut pg, wave(&prm, erase_seed(tex, 0x00C0DE00 + v as u32), y0));
            let paper = if PAPERS[pi] == PAPER_MEDIUM { "medium rule" } else { "grand staff" };
            page_render(&mut c, &pg, band, DPI);
            look(&cformat!("%s, %s", paper, K_LOOK_KEY[v]), crc_rect(&c, band));
            if !g_print() {
                check!(diff_px(&c, &inked, band) > 1000);
            }
            canvas_fill(&mut c, band, 255);
            page_render_layers(&mut c, &pg, band, DPI, PAGE_LAYER_INK);
            look(&cformat!("%s, %s, ink", paper, K_LOOK_KEY[v]), crc_rect(&c, band));
            if tex == TOOL_HIGHLIGHTER && !g_print() {
                // the same white stroke drawn since 2.0.0-beta.5
                let last = pg.strokes.len() - 1;
                stroke_mark_tip(&mut pg.strokes[last]);
                check!(stroke_tip(&pg.strokes[last]) == TIP_HIGHLIGHTER_VERTICAL);
                page_render(&mut c, &pg, band, DPI);
                look(&cformat!("%s, white highlighter, vertical tip", paper), crc_rect(&c, band));
                canvas_fill(&mut c, band, 255);
                page_render_layers(&mut c, &pg, band, DPI, PAGE_LAYER_INK);
                look(&cformat!("%s, white highlighter, vertical tip, ink", paper), crc_rect(&c, band));
            }
            page_free(pg);
        }
    }
    canvas_free(&mut c);
    canvas_free(&mut inked);
}

#[test]
fn test_erase() {
    font_init(); // some papers draw labels and clefs
    if g_print() {
        // the table only
        println!("const K_LOOK: [(&str, u32); N] = [");
        run!(erase_look_is_recorded);
        println!("];");
        return;
    }
    if !GOLDEN_HOST {
        eprintln!("  erase look: not the recording host (clang, arm64 macOS): renders not compared");
    }
    run!(erase_look_is_recorded);
    run!(restore_blend);
    run!(live_equals_rerender);
    run!(span_fast_path_is_the_per_pixel_path);
    run!(eraser_core_returns_to_paper);
    run!(ink_after_erasing_is_untouched);
    run!(white_textures_are_deterministic);
    run!(other_texture_bytes_draw_the_round_tip);
    run!(erase_inside_bbox);
    run!(fully_erased_stroke_leaves_nothing);
    run!(compaction_drops_only_what_changes_nothing);
    run!(compaction_is_the_rule);
    run!(compaction_checks_the_page_too);
    run!(restored_is_the_page_only);
    run!(one_call_keeps_the_lines);
    run!(restored_tells_a_no_op);
    run!(undo_and_redo_an_erase_stroke);
    run!(beta_opens_erase_strokes);
    run!(reader_and_rescale_know_the_texture);
    done();
}
