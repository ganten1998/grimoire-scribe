// Not a port of a C test: the stroke eraser draws the page again only in the tiles the strokes it
// took off drew into (page.rs page_render_tiles, canvas.rs TileMask, brush.rs BrushState.mask). On
// random dense pages of every tool, white ink and the precise eraser, over blank, ruled, dotted, music
// and graph paper: a canvas of the page with them, drawn again in their tiles, is byte for byte a fresh
// drawing of the page without them; nothing outside the tiles of the region is touched; a masked brush
// draws exactly its dabs' pixels in marked tiles and none elsewhere, with the erase fast path on and
// off.
//
// dense_erase_bench (ignored) times it on a real page (the owner's 1,090-stroke one is never
// committed): copy its notebook folder somewhere first, never timing the original, then
//   GRIMOIRE_DENSE_PAGE=/path/to/copy/p72.qpg cargo test --release --test test_erase_tiles -- \
//     --ignored --nocapture dense_erase_bench
// It prints the first full draw, each tool's share of it, single strokes and eraser sweeps taken off
// (the box drawn again, as before 2.0.0-rc.12, and the tiles), each checked against a fresh drawing.
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use testlib::*;

const DPI: f32 = 300.0;

static SEED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
fn rnd() -> u32 {
    let s = SEED.load(Ordering::Relaxed).wrapping_mul(1664525).wrapping_add(1013904223);
    SEED.store(s, Ordering::Relaxed);
    s >> 8
}
fn frand(lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rnd() % 10001) as f32 / 10000.0
}

fn whole(p: &Page) -> Rect {
    Rect { x: 0, y: 0, w: p.w, h: p.h }
}

// A stroke of tool t (TOOL_ERASER: an erase stroke of a random texture) wandering from a random start,
// long and across the page at times, with pressure and tilt changing as it goes.
fn wander(w: i32, h: i32, t: ToolKind) -> Box<Stroke> {
    let erase = t == TOOL_ERASER;
    let texture = if erase { ToolKind((rnd() % TOOL_COUNT as u32) as i32) } else { t };
    let mut prm = ToolParams::default();
    tool_params_default(texture, &mut prm);
    if rnd() % 4 == 0 {
        prm.grain = frand(0.0, 1.0);
    }
    if rnd() % 4 == 0 {
        prm.tilt_size = frand(0.0, 2.0);
    }
    let seed = if erase { erase_seed(texture, rnd()) } else { rnd() & 0xFFFFFF };
    let mut s = stroke_new(t, &prm, seed);
    let (mut x, mut y) = (frand(-40.0, w as f32 + 40.0), frand(-40.0, h as f32 + 40.0));
    let mut ang = frand(0.0, 6.28);
    let step = frand(2.0, 24.0);
    let mut pr = frand(0.1, 1.0);
    let n = 2 + rnd() % if rnd() % 3 == 0 { 220 } else { 50 };
    for k in 0..n {
        let q = PenSample { x, y, pressure: pr, tilt_x: frand(-60.0, 60.0), tilt_y: frand(-60.0, 60.0), t_ms: k * 6 };
        stroke_append(&mut s, &q, DPI);
        ang += frand(-0.5, 0.5);
        x = fmla(ang.cos(), step, x);
        y = fmla(ang.sin(), step, y);
        pr = clampf(pr + frand(-0.08, 0.08), 0.0, 1.0);
    }
    s
}

// A dense page: n strokes of every drawing tool, one in eight an erase stroke.
fn dense_page(w: i32, h: i32, paper: PaperKind, n: usize) -> Box<Page> {
    let mut p = page_new(w, h, paper);
    for _ in 0..n {
        let t = if rnd() % 8 == 0 { TOOL_ERASER } else { ToolKind((rnd() % TOOL_COUNT as u32) as i32) };
        let t = if t == TOOL_ERASER && rnd() % 8 != 0 { TOOL_MARKER } else { t };
        page_add(&mut p, wander(w, h, t));
    }
    p
}

fn copy_stroke(s: &Stroke) -> Box<Stroke> {
    let mut c = stroke_new(s.tool, &s.params, s.seed);
    c.ext = s.ext;
    for q in &s.pts {
        stroke_append(&mut c, q, DPI);
    }
    c
}

fn page_without(p: &Page, gone: &[usize]) -> Box<Page> {
    let mut q = page_new(p.w, p.h, p.paper);
    for (i, s) in p.strokes.iter().enumerate() {
        if !gone.contains(&i) {
            page_add(&mut q, copy_stroke(s));
        }
    }
    q
}

// The tiles strokes gone of p drew into, and the union of their boxes.
fn mask_of(p: &Page, gone: &[usize]) -> (Rc<TileMask>, Rect) {
    let mut m = tile_mask_new(p.w, p.h).unwrap();
    let mut d = RECT_EMPTY;
    for &i in gone {
        stroke_mark_tiles(&mut m, &p.strokes[i], DPI);
        d = rect_union(d, p.strokes[i].bbox);
    }
    tile_mask_seal(&mut m);
    (Rc::new(m), d)
}

fn in_tiles(m: &TileMask, r: Rect, x: i32, y: i32) -> bool {
    rect_contains(r, x, y) && tile_mask_hits(m, Rect { x, y, w: 1, h: 1 })
}

// Taking strokes off (one, a few, an erase stroke among them, all of one tool) and drawing their tiles
// again over the page with them gives the page without them, byte for byte; a canvas of something else
// keeps it outside the tiles of the region, and gets the fresh drawing inside them.
fn tiles_draw_the_page_without_them() {
    let (w, h) = (420, 560);
    for (k, paper) in [PAPER_BLANK, PAPER_NARROW, PAPER_DOTS, PAPER_MUSIC, PAPER_GRID].into_iter().enumerate() {
        SEED.store(0x7113 + k as u32, Ordering::Relaxed);
        let p = dense_page(w, h, paper, 90);
        let mut with = Canvas::default();
        check!(canvas_init(&mut with, w, h) == 0);
        page_render(&mut with, &p, whole(&p), DPI);
        let n = p.strokes.len();
        let mut cases: Vec<Vec<usize>> = (0..10).map(|_| vec![rnd() as usize % n]).collect();
        cases.push((0..5).map(|_| rnd() as usize % n).collect());
        cases.push((0..n).filter(|&i| stroke_is_erase(&p.strokes[i])).take(2).collect());
        cases.push((0..n).filter(|&i| p.strokes[i].tool == TOOL_CHARCOAL).collect());
        cases.push(vec![n - 1, 0]);
        for gone in &cases {
            let q = page_without(&p, gone);
            let mut fresh = Canvas::default();
            check!(canvas_init(&mut fresh, w, h) == 0);
            page_render(&mut fresh, &q, whole(&q), DPI);
            let (m, d) = mask_of(&p, gone);
            let mut c = with.clone();
            page_render_tiles(&mut c, &q, d, &m, DPI);
            check!(c.px == fresh.px);
            // the same with the erase fast path off
            BRUSH_ERASE_SPANS.store(false, Ordering::Relaxed);
            let mut c = with.clone();
            page_render_tiles(&mut c, &q, d, &m, DPI);
            BRUSH_ERASE_SPANS.store(true, Ordering::Relaxed);
            check!(c.px == fresh.px);
            // only the tiles of the region: over a canvas of one gray
            let mut g = Canvas::default();
            check!(canvas_init(&mut g, w, h) == 0);
            canvas_fill(&mut g, whole(&p), 0x5A);
            page_render_tiles(&mut g, &q, d, &m, DPI);
            let mut wrong = 0;
            for y in 0..h {
                for x in 0..w {
                    let at = (y * w + x) as usize;
                    let want = if in_tiles(&m, d, x, y) { fresh.px[at] } else { 0x5A };
                    wrong += (g.px[at] != want) as i32;
                }
            }
            check!(wrong == 0);
        }
    }
}

// A brush with a mask draws, in the marked tiles, exactly what it draws without one, and nothing
// elsewhere; it moves on the same (the rects it returns are the same): every tool, ink and white over
// ruled paper, the erase fast path on and off, a checkerboard of tiles and a band.
fn masked_dabs_are_the_dabs() {
    let (w, h) = (300, 240);
    let mut paper = Canvas::default();
    check!(canvas_init(&mut paper, w, h) == 0);
    paper_render(&mut paper, PAPER_NARROW, Rect { x: 0, y: 0, w, h }, DPI);
    let pp = Rc::new(paper.px.clone());
    let mut checker = tile_mask_new(w, h).unwrap();
    for ty in 0..(h + MASK_TILE - 1) / MASK_TILE {
        for tx in 0..(w + MASK_TILE - 1) / MASK_TILE {
            if (tx + ty) % 2 == 0 {
                tile_mask_mark(&mut checker, Rect { x: tx * MASK_TILE, y: ty * MASK_TILE, w: 1, h: 1 });
            }
        }
    }
    tile_mask_seal(&mut checker);
    let mut band = tile_mask_new(w, h).unwrap();
    tile_mask_mark(&mut band, Rect { x: 0, y: 100, w, h: 20 });
    tile_mask_seal(&mut band);
    SEED.store(0xDAB5, Ordering::Relaxed);
    for spans in [true, false] {
        BRUSH_ERASE_SPANS.store(spans, Ordering::Relaxed);
        for t in 0..TOOL_COUNT as i32 {
            for mask in [&checker, &band] {
                let s = wander(w, h, ToolKind(t));
                // over ink, so white ink has something to take away
                let mut under = paper.clone();
                let ink = wander(w, h, TOOL_MARKER);
                let mut b = BrushState::default();
                stroke_brush_begin(&mut b, &ink, DPI, None, 0, 0, 0);
                for q in &ink.pts {
                    brush_add(&mut b, &mut under, q);
                }
                let mut all = under.clone();
                let mut some = under.clone();
                let mut ba = BrushState::default();
                let mut bs = BrushState::default();
                stroke_brush_begin(&mut ba, &s, DPI, Some(pp.clone()), 0, 0, w);
                stroke_brush_begin(&mut bs, &s, DPI, Some(pp.clone()), 0, 0, w);
                bs.mask = Some(Rc::new(mask.clone()));
                let mut same_course = true;
                for q in &s.pts {
                    same_course &= brush_add(&mut ba, &mut all, q) == brush_add(&mut bs, &mut some, q);
                }
                check!(same_course);
                let mut wrong = 0;
                for y in 0..h {
                    for x in 0..w {
                        let at = (y * w + x) as usize;
                        let want = if tile_mask_hits(mask, Rect { x, y, w: 1, h: 1 }) { all.px[at] } else { under.px[at] };
                        wrong += (some.px[at] != want) as i32;
                    }
                }
                check!(wrong == 0);
            }
        }
    }
    BRUSH_ERASE_SPANS.store(true, Ordering::Relaxed);
}

// A mask of a canvas whose sides are not whole tiles: marks clip to the canvas, runs stop at its edge
// and at the bounds of the region asked about, counts are the tiles a rect meets.
fn masks_count_and_run() {
    let mut m = tile_mask_new(100, 70).unwrap();
    tile_mask_seal(&mut m);
    check!(rect_is_empty(tile_mask_bounds(&m)) && !tile_mask_hits(&m, Rect { x: 0, y: 0, w: 100, h: 70 }));
    check!(tile_mask_run(&m, 5, 0, 100).is_none());
    tile_mask_mark(&mut m, Rect { x: -30, y: -30, w: 35, h: 35 }); // the corner tile only
    tile_mask_mark(&mut m, Rect { x: 97, y: 65, w: 50, h: 50 }); // the last, a part tile
    tile_mask_mark(&mut m, Rect { x: 40, y: 20, w: 1, h: 1 });
    tile_mask_mark(&mut m, Rect { x: 200, y: 200, w: 5, h: 5 }); // off the canvas: nothing
    tile_mask_seal(&mut m);
    let t = MASK_TILE;
    check!(tile_mask_bounds(&m) == Rect { x: 0, y: 0, w: 100, h: 70 });
    check!(tile_mask_count(&m, Rect { x: 0, y: 0, w: 100, h: 70 }) == 3);
    check!(tile_mask_count(&m, Rect { x: 0, y: 0, w: 1, h: 1 }) == 1);
    check!(!tile_mask_hits(&m, Rect { x: t, y: 0, w: t, h: t }));
    check!(tile_mask_hits(&m, Rect { x: 40, y: 20, w: 1, h: 1 }));
    check!(tile_mask_hits(&m, Rect { x: 99, y: 69, w: 30, h: 30 }));
    check!(tile_mask_run(&m, 3, 0, 100) == Some((0, t)));
    check!(tile_mask_run(&m, 3, 2, 100) == Some((2, t)));
    check!(tile_mask_run(&m, 3, t, 100).is_none());
    let (x0, y0) = (40 / t * t, 20 / t * t);
    check!(tile_mask_run(&m, y0, 0, 100) == Some((x0, x0 + t)));
    check!(tile_mask_run(&m, y0, x0 + 3, x0 + 5) == Some((x0 + 3, x0 + 5)));
    check!(tile_mask_run(&m, 69, 0, 100) == Some((99 / t * t, 100)));
    // two marked tiles side by side are one run
    tile_mask_mark(&mut m, Rect { x: x0 + t, y: y0, w: 1, h: 1 });
    tile_mask_seal(&mut m);
    check!(tile_mask_run(&m, y0 + 1, 0, 100) == Some((x0, x0 + 2 * t)));
}

#[test]
fn test_erase_tiles() {
    run!(masks_count_and_run);
    run!(masked_dabs_are_the_dabs);
    run!(tiles_draw_the_page_without_them);
    done();
}

// ---- the bench (ignored) ----

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn area(r: Rect) -> i64 {
    r.w as i64 * r.h as i64
}

// A stroke eraser sweep as erase_segment does it: every pen step takes off the page the strokes it
// touches, and the page is drawn again where they were (their boxes, or their tiles there).
fn sweep(p: &Page, with: &Canvas, path: &[(f32, f32)], r: f32, tiles: bool) -> (f64, usize, Canvas, Box<Page>) {
    let mut q = page_without(p, &[]);
    let mut c = with.clone();
    let mut gone = 0;
    let t = Instant::now();
    for w in path.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        let mut d = RECT_EMPTY;
        let mut off: Vec<Box<Stroke>> = Vec::new();
        for i in (0..q.strokes.len()).rev() {
            let st = &q.strokes[i];
            if stroke_is_erase(st) || !stroke_hit_segment(st, x0, y0, x1, y1, r, DPI) {
                continue;
            }
            let st = page_remove(&mut q, i as i32);
            d = rect_union(d, st.bbox);
            off.push(st);
        }
        if rect_is_empty(d) {
            continue;
        }
        gone += off.len();
        if tiles {
            let mut m = tile_mask_new(q.w, q.h).unwrap();
            for st in &off {
                stroke_mark_tiles(&mut m, st, DPI);
            }
            tile_mask_seal(&mut m);
            page_render_tiles(&mut c, &q, d, &Rc::new(m), DPI);
        } else {
            page_render(&mut c, &q, d, DPI);
        }
    }
    (ms(t), gone, c, q)
}

#[test]
#[ignore]
fn dense_erase_bench() {
    let Ok(path) = std::env::var("GRIMOIRE_DENSE_PAGE") else {
        eprintln!("GRIMOIRE_DENSE_PAGE names no page: nothing to time");
        return;
    };
    store_set_device(DPI, 1860, 2480);
    let buf = std::fs::read(&path).expect("the page file");
    let mut out = None;
    assert!(page_deserialize(&buf, &mut out) == 0);
    let p = out.unwrap();
    let npts: usize = p.strokes.iter().map(|s| s.pts.len()).sum();
    eprintln!("{}: {}x{}, {} strokes, {} points", path, p.w, p.h, p.strokes.len(), npts);
    let mut with = Canvas::default();
    assert!(canvas_init(&mut with, p.w, p.h) == 0);
    let t = Instant::now();
    page_render(&mut with, &p, whole(&p), DPI);
    let full = ms(t);
    eprintln!("first full draw {full:.1} ms");
    // Each tool's strokes alone over the page, and their brushes' course alone (dry: no pixel).
    for t in 0..TOOL_COUNT as i32 {
        let mine: Vec<usize> = (0..p.strokes.len()).filter(|&i| p.strokes[i].tool.0 == t).collect();
        if mine.is_empty() {
            continue;
        }
        let others: Vec<usize> = (0..p.strokes.len()).filter(|&i| p.strokes[i].tool.0 != t).collect();
        let q = page_without(&p, &others);
        let pts: usize = q.strokes.iter().map(|s| s.pts.len()).sum();
        let mut c = Canvas::default();
        assert!(canvas_init(&mut c, p.w, p.h) == 0);
        let t0 = Instant::now();
        page_render(&mut c, &q, whole(&q), DPI);
        let alone = ms(t0);
        let mut none = Canvas { px: Vec::new(), w: p.w, h: p.h, stride: p.w, clip: whole(&p) };
        let t0 = Instant::now();
        for s in &q.strokes {
            let mut b = BrushState::default();
            stroke_brush_begin(&mut b, s, DPI, None, 0, 0, 0);
            b.dry = true;
            for pt in &s.pts {
                brush_add(&mut b, &mut none, pt);
            }
        }
        let dry = ms(t0);
        eprintln!("  {:<18} {:5} strokes {:7} points: {:8.1} ms alone ({:.3} ms a stroke, {:.4} a point), course {:.1} ms",
            tool_name(ToolKind(t)), mine.len(), pts, alone, alone / mine.len() as f64, alone / pts as f64, dry);
    }
    // Single strokes: a small pencil stroke, a median marker stroke, the largest marker stroke, the
    // longest charcoal stroke.
    let of = |t: ToolKind| -> Vec<usize> { (0..p.strokes.len()).filter(|&i| p.strokes[i].tool == t).collect() };
    let mut mech = of(TOOL_MECH_PENCIL);
    mech.sort_by_key(|&i| area(p.strokes[i].bbox));
    let mut mk = of(TOOL_MARKER);
    mk.sort_by_key(|&i| area(p.strokes[i].bbox));
    let mut ch = of(TOOL_CHARCOAL);
    ch.sort_by_key(|&i| p.strokes[i].pts.len());
    let mut cases: Vec<(&str, usize)> = Vec::new();
    if !mech.is_empty() {
        cases.push(("a small pencil stroke", mech[mech.len() / 2]));
    }
    if !mk.is_empty() {
        cases.push(("a median marker stroke", mk[mk.len() / 2]));
        cases.push(("the largest marker stroke", mk[mk.len() - 1]));
    }
    if !ch.is_empty() {
        cases.push(("the longest charcoal stroke", ch[ch.len() - 1]));
    }
    for (name, i) in cases {
        let q = page_without(&p, &[i]);
        let mut fresh = Canvas::default();
        assert!(canvas_init(&mut fresh, p.w, p.h) == 0);
        page_render(&mut fresh, &q, whole(&q), DPI);
        let d = p.strokes[i].bbox;
        let mut c = with.clone();
        let t0 = Instant::now();
        page_render(&mut c, &q, d, DPI);
        let boxed = ms(t0);
        let box_ok = c.px == fresh.px;
        let mut c = with.clone();
        let t0 = Instant::now();
        let (m, _) = mask_of(&p, &[i]);
        page_render_tiles(&mut c, &q, d, &m, DPI);
        let tiled = ms(t0);
        let s = &p.strokes[i];
        eprintln!("{name} ({}, {} points, box {}x{}): box {boxed:.1} ms (exact {box_ok}), tiles {tiled:.1} ms (exact {}), {:.1}x",
            tool_name(s.tool), s.pts.len(), d.w, d.h, c.px == fresh.px, boxed / tiled);
    }
    // Sweeps of the eraser at its default size, half pressure (4.75 mm across), a pen sample every 8 px.
    let r = mm_to_px(4.75, DPI) * 0.5;
    let lines: [(&str, (f32, f32), (f32, f32)); 3] =
        [("a short sweep", (300.0, 1560.0), (420.0, 1560.0)), ("across the page", (500.0, 1300.0), (1500.0, 1300.0)), ("down the page", (930.0, 200.0), (930.0, 2300.0))];
    for (name, a, b) in lines {
        let n = (((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() / 8.0) as usize;
        let path: Vec<(f32, f32)> = (0..=n).map(|k| (a.0 + (b.0 - a.0) * k as f32 / n as f32, a.1 + (b.1 - a.1) * k as f32 / n as f32)).collect();
        let (boxed, gone, c_box, q) = sweep(&p, &with, &path, r, false);
        let (tiled, _, c_tiles, _) = sweep(&p, &with, &path, r, true);
        let mut fresh = Canvas::default();
        assert!(canvas_init(&mut fresh, p.w, p.h) == 0);
        page_render(&mut fresh, &q, whole(&q), DPI);
        // (the least a sweep could cost: all it took off drawn again once, at its end)
        let mut c = with.clone();
        let t0 = Instant::now();
        let mut m = tile_mask_new(p.w, p.h).unwrap();
        let mut d = RECT_EMPTY;
        for s in &p.strokes {
            if !q.strokes.iter().any(|k| k.seed == s.seed && k.pts.len() == s.pts.len() && k.tool == s.tool) {
                stroke_mark_tiles(&mut m, s, DPI);
                d = rect_union(d, s.bbox);
            }
        }
        tile_mask_seal(&mut m);
        page_render_tiles(&mut c, &q, d, &Rc::new(m), DPI);
        let once = ms(t0);
        eprintln!("{name}: {} steps, {gone} strokes off: box {boxed:.1} ms (exact {}), tiles {tiled:.1} ms (exact {}), {:.1}x; all at once {once:.1} ms (exact {})",
            path.len() - 1, c_box.px == fresh.px, c_tiles.px == fresh.px, boxed / tiled, c.px == fresh.px);
    }
}
