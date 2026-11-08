// Not a port of a C test: the exact-area lasso's cut (lasso.rs stroke_dabs, stroke_cut,
// Lasso::rect_inside; docs/beta2/PIXEL-LASSO.md 1 and 4, tests 1 to 4): the dab log is the real draw's
// dabs; a stroke cut along a lasso draws, piece after piece, what the stroke drew (every tool and tip,
// over paper: the counted rare one-level f32 pixel aside, and the per-dab jitter of pencil, mechanical
// pencil and charcoal pinned; their ends are the dabs' own edges, unchanged); pieces saved and read back
// draw the same bytes; rect_inside agrees with a brute-force test.
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::sync::atomic::Ordering;
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

fn canvas(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::default();
    assert!(canvas_init(&mut c, w, h) == 0);
    c
}
fn render(p: &Page) -> Canvas {
    let mut c = canvas(p.w, p.h);
    page_render(&mut c, p, Rect { x: 0, y: 0, w: p.w, h: p.h }, DPI);
    c
}

/// A wandering stroke of tool t (tip `tip`), its points stored as the page file stores them.
fn stroke(t: ToolKind, tip: u8, w: f32, h: f32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default_pre21(t, &mut prm); // (the tolerances below were measured at these sizes)
    if rnd() % 3 == 0 {
        prm.tilt_size = frand(0.0, 1.5);
    }
    let mut s = stroke_new(t, &prm, rnd() & 0xFFFFFF);
    s.ext[2] = tip;
    let (mut x, mut y) = (frand(w * 0.3, w * 0.7), frand(h * 0.3, h * 0.7));
    let mut ang = frand(0.0, 6.28);
    let n = 2 + rnd() % 60;
    let step = frand(2.0, 14.0);
    for k in 0..n {
        let mut q = PenSample { x, y, pressure: frand(0.2, 1.0), tilt_x: frand(-50.0, 50.0), tilt_y: frand(-50.0, 50.0), t_ms: k * 7 };
        sample_as_stored(&mut q);
        stroke_append(&mut s, &q, DPI);
        ang += frand(-0.6, 0.6);
        x += ang.cos() * step;
        y += ang.sin() * step;
    }
    s
}

fn random_lasso(w: f32, h: f32) -> Lasso {
    loop {
        let (cx, cy) = (frand(w * 0.3, w * 0.7), frand(h * 0.3, h * 0.7));
        let n = 3 + rnd() % 12;
        let pts: Vec<(f32, f32)> = (0..n)
            .map(|k| {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let r = frand(20.0, w * 0.4);
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        if let Some(l) = Lasso::new(&pts) {
            return l;
        }
    }
}

// ---- 1. the dab log ----

fn dab_log_is_the_draw() {
    SEED.store(0xDAB1, Ordering::Relaxed);
    let mut bad = 0;
    for t in 0..TOOL_COUNT as i32 {
        let t = ToolKind(t);
        if t == TOOL_ERASER {
            continue;
        }
        for tip in [TIP_FIRST, TIP_HIGHLIGHTER_VERTICAL] {
            if !brush_tip_known(t, tip) {
                continue;
            }
            for _ in 0..20 {
                let s = stroke(t, tip, 600.0, 600.0);
                let dabs = stroke_dabs(&s, DPI);
                // a real draw stamps as many dabs (dab_index counts them)
                let mut c = canvas(600, 600);
                let mut b = BrushState::default();
                stroke_brush_begin(&mut b, &s, DPI, None, 0, 0, 0);
                for q in &s.pts {
                    brush_add(&mut b, &mut c, q);
                }
                if b.dab_index as usize != dabs.len() || dabs.is_empty() {
                    bad += 1;
                }
                // in order along the points, each radius the brush's own for its sample
                if dabs.windows(2).any(|w| w[1].seg < w[0].seg) || dabs[0].seg != 0 {
                    bad += 1;
                }
                for d in &dabs {
                    let (mut r, mut o) = (0.0, 0.0);
                    brush_dab(stroke_render_tool(&s), &s.params, &d.m, DPI, &mut r, &mut o);
                    bad += (r != d.r) as i32;
                }
            }
        }
    }
    check!(bad == 0);
    // a one-point stroke: one dab; a corrupt 30,000 px segment: no dab bridges it
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut prm);
    let mut s = stroke_new(TOOL_BALLPOINT, &prm, 1);
    stroke_append(&mut s, &PenSample { x: 100.0, y: 100.0, pressure: 0.5, ..Default::default() }, DPI);
    check!(stroke_dabs(&s, DPI).len() == 1);
    stroke_append(&mut s, &PenSample { x: 30100.0, y: 100.0, pressure: 0.5, t_ms: 5, ..Default::default() }, DPI);
    check!(stroke_dabs(&s, DPI).len() == 1);
}

// ---- 2. the cut draws the stroke ----

/// (differing pixels, the largest difference, the mean difference over the stroke's box)
fn compare(a: &Canvas, b: &Canvas, r: Rect) -> (usize, i32, f64) {
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h });
    let (mut n, mut m, mut sum) = (0usize, 0i32, 0i64);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            let i = (y * a.stride + x) as usize;
            let d = (a.px[i] as i32 - b.px[i] as i32).abs();
            if d > 0 {
                n += 1;
                m = m.max(d);
                sum += d as i64;
            }
        }
    }
    (n, m, sum as f64 / (r.w.max(1) as f64 * r.h.max(1) as f64))
}

fn cut_draws_the_stroke() {
    SEED.store(0xC07, Ordering::Relaxed);
    let (w, h) = (500, 500);
    for t in 0..TOOL_COUNT as i32 {
        let t = ToolKind(t);
        if t == TOOL_ERASER {
            continue;
        }
        let jitter = t == TOOL_PENCIL || t == TOOL_MECH_PENCIL || t == TOOL_CHARCOAL;
        for tip in [TIP_FIRST, TIP_HIGHLIGHTER_VERTICAL] {
            if !brush_tip_known(t, tip) {
                continue;
            }
            let (mut cuts, mut px_off, mut px_all, mut worst, mut worst_mean) = (0, 0usize, 0usize, 0, 0.0f64);
            let mut prefix_off = 0usize;
            for _ in 0..30 {
                let s = stroke(t, tip, w as f32, h as f32);
                if !stroke_cuttable(&s) {
                    // (a calligraphy stroke whose tilt widens or lightens it: judged whole)
                    check!(t == TOOL_CALLIGRAPHY && s.params.tilt_size != 0.0);
                    continue;
                }
                let l = random_lasso(w as f32, h as f32);
                let Cut::Pieces(pieces) = stroke_cut(&s, DPI, &|x, y| l.inside(x, y)) else { continue };
                cuts += 1;
                check!(pieces.len() >= 2);
                check!(pieces.windows(2).all(|p| p[0].1 != p[1].1));
                for (pc, _) in &pieces {
                    check!(pc.tool == s.tool && pc.seed == s.seed && pc.ext == s.ext && pc.params.size_max_mm == s.params.size_max_mm);
                }
                let mut a = page_new(w, h, PAPER_NARROW);
                page_add(&mut a, Box::new((*s).clone()));
                let mut b = page_new(w, h, PAPER_NARROW);
                for (pc, _) in &pieces {
                    page_add(&mut b, pc.clone());
                }
                let (ca, cb) = (render(&a), render(&b));
                let (n, m, mean) = compare(&ca, &cb, s.bbox);
                px_off += n;
                px_all += (s.bbox.w * s.bbox.h) as usize;
                worst = worst.max(m);
                worst_mean = worst_mean.max(mean);
                // the prefix piece alone: its own dabs are the stroke's first dabs; drawn over the stroke
                // drawn whole, it may only differ where the stroke went on (none of its pixels lighter)
                let mut pre = page_new(w, h, PAPER_NARROW);
                page_add(&mut pre, pieces[0].0.clone());
                let cp = render(&pre);
                for i in 0..ca.px.len() {
                    prefix_off += (cp.px[i] < ca.px[i].saturating_sub(1)) as usize;
                }
            }
            check!(cuts >= 8);
            check!(prefix_off == 0);
            if jitter {
                // the per-dab opacity jitter re-rolls after a cut, a grain change in single pixels: pinned
                // (measured 2026-11-03: largest 54, worst mean 0.39 of a level over the stroke's box)
                check!(worst <= 64);
                check!(worst_mean <= 0.5);
            } else if t == TOOL_CALLIGRAPHY {
                // the nib seeded at the cut (brush_tilt_for_nib): its 0.01 degree tilt (measured: largest 6)
                check!(worst <= 8);
                check!(worst_mean <= 0.05);
                check!(px_off * 500 <= px_all);
            } else {
                // the rare one-level f32 pixel, counted
                check!(worst <= 2);
                check!(px_off * 2000 <= px_all);
            }
            eprintln!(
                "  {:<18} tip {tip}: {cuts} cuts, {px_off} of {px_all} px differ, largest {worst}, worst mean {worst_mean:.4}",
                tool_name(t)
            );
        }
    }
}

// ---- 3. pieces saved and read back ----

fn pieces_round_trip() {
    SEED.store(0x5A7E, Ordering::Relaxed);
    let (w, h) = (500, 500);
    let mut bad = 0;
    for k in 0..40 {
        let t = [TOOL_BALLPOINT, TOOL_FOUNTAIN, TOOL_CALLIGRAPHY, TOOL_PENCIL, TOOL_HIGHLIGHTER][k % 5];
        let s = stroke(t, TIP_FIRST, w as f32, h as f32);
        let l = random_lasso(w as f32, h as f32);
        let Cut::Pieces(pieces) = stroke_cut(&s, DPI, &|x, y| l.inside(x, y)) else { continue };
        let mut p = page_new(w, h, PAPER_BLANK);
        for (pc, _) in pieces {
            page_add(&mut p, pc);
        }
        let before = render(&p);
        let mut buf = Vec::new();
        check!(page_serialize(&p, &mut buf) == 0);
        let mut out = None;
        check!(page_deserialize(&buf, &mut out) == 0);
        let q = out.unwrap();
        bad += (render(&q).px != before.px) as i32;
        // and the bytes written again are the same
        let mut buf2 = Vec::new();
        page_serialize(&q, &mut buf2);
        bad += (buf2 != buf) as i32;
    }
    check!(bad == 0);
}

// ---- 4. rect_inside ----

fn rect_inside_matches_brute_force() {
    SEED.store(0x4EC7, Ordering::Relaxed);
    let (mut wrong, mut decided) = (0, 0);
    for _ in 0..60 {
        let n = 3 + rnd() % 25;
        let pts: Vec<(f32, f32)> = (0..n).map(|_| (frand(0.0, 300.0), frand(0.0, 300.0))).collect();
        let Some(l) = Lasso::new(&pts) else { continue };
        for _ in 0..200 {
            let r = Rect { x: (rnd() % 320) as i32 - 10, y: (rnd() % 320) as i32 - 10, w: 1 + (rnd() % 40) as i32, h: 1 + (rnd() % 40) as i32 };
            let got = l.rect_inside(r);
            // brute force: every point of r on a fine grid, edges included
            let mut all_in = true;
            let mut all_out = true;
            for gy in 0..=r.h * 4 {
                for gx in 0..=r.w * 4 {
                    let (x, y) = (r.x as f32 + gx as f32 / 4.0, r.y as f32 + gy as f32 / 4.0);
                    let i = l.inside(x, y);
                    all_in &= i;
                    all_out &= !i;
                }
            }
            match got {
                Some(true) => wrong += (!all_in) as i32,
                Some(false) => wrong += (!all_out) as i32,
                None => {}
            }
            decided += got.is_some() as i32;
        }
    }
    check!(wrong == 0);
    check!(decided > 2000);
}


// ---- 5. the cut plan ----

fn line(t: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, seed: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    let mut s = stroke_new(t, &prm, seed);
    for k in 0..=10 {
        let f = k as f32 / 10.0;
        stroke_append(&mut s, &PenSample { x: x0 + (x1 - x0) * f, y: y0 + (y1 - y0) * f, pressure: 0.6, t_ms: k * 8, ..Default::default() }, DPI);
    }
    s
}
fn link(seed: u32, area: Option<Rect>) -> NavLink {
    let mut l = NavLink { pid: 1, seed, ..Default::default() };
    l.target.set("p2");
    if let Some(a) = area {
        l.area = a;
        l.has_area = true;
    }
    l
}
fn ids(p: &Page) -> Vec<StrokeId> {
    p.strokes.iter().map(|s| stroke_id(s)).collect()
}

fn cut_plan_rules() {
    let mut p = page_new(800, 600, PAPER_BLANK);
    page_add(&mut p, line(TOOL_BALLPOINT, 100.0, 100.0, 400.0, 100.0, 1)); // 0: crossed by the lasso's edge at 250
    page_add(&mut p, line(TOOL_BALLPOINT, 260.0, 150.0, 300.0, 150.0, 2)); // 1: inside whole
    page_add(&mut p, line(TOOL_BALLPOINT, 50.0, 200.0, 400.0, 200.0, 0x55)); // 2: a beta.5 link's ink, crossed
    page_add(&mut p, line(TOOL_ERASER, 100.0, 150.0, 400.0, 150.0, erase_seed(TOOL_ERASER, 3))); // 3: never cut
    page_add(&mut p, line(TOOL_BALLPOINT, 600.0, 100.0, 700.0, 100.0, 4)); // 4: outside
    // a link area round the right half of stroke 0
    let links = [link(0x55, None), link(0xA, Some(Rect { x: 300, y: 80, w: 120, h: 40 }))];
    let l = Lasso::new(&[(250.0, 50.0), (500.0, 50.0), (500.0, 250.0), (250.0, 250.0)]).unwrap();
    let plan = lasso_cut_plan(&p, &links, &l, DPI, lasso_enclosed(&p, &l, DPI));
    check!(plan.whole == vec![1]); // (the link's ink is half inside by length: left out whole, not cut)
    check!(plan.cut.len() == 1 && plan.cut[0].0 == 0);
    check!(plan.cut[0].1.len() == 2 && !plan.cut[0].1[0].1 && plan.cut[0].1[1].1);
    // the area's middle box: the inside piece, taken: the link comes along
    check!(plan.links == vec![1]);
    check!(plan.taken() == 2);
    // a lasso round most of the link's ink: all of it, whole
    let l2 = Lasso::new(&[(150.0, 180.0), (500.0, 180.0), (500.0, 220.0), (150.0, 220.0)]).unwrap();
    let plan2 = lasso_cut_plan(&p, &links, &l2, DPI, lasso_enclosed(&p, &l2, DPI));
    check!(plan2.whole == vec![2] && plan2.cut.is_empty() && plan2.links == vec![0]);
    // a stroke grazed by one dab: a one-dab piece
    let mut q = page_new(800, 600, PAPER_BLANK);
    page_add(&mut q, line(TOOL_FINELINER, 100.0, 300.0, 400.0, 300.0, 9));
    let dabs = stroke_dabs(&q.strokes[0], DPI);
    let last = dabs.last().unwrap().m;
    let edge = (last.x + dabs[dabs.len() - 2].m.x) / 2.0;
    let l3 = Lasso::new(&[(edge, 250.0), (600.0, 250.0), (600.0, 350.0), (edge, 350.0)]).unwrap();
    let plan3 = lasso_cut_plan(&q, &[], &l3, DPI, lasso_enclosed(&q, &l3, DPI));
    check!(plan3.cut.len() == 1);
    if let Some((_, v)) = plan3.cut.first() {
        let inside = &v[1].0;
        check!(stroke_dabs(inside, DPI).len() == 1);
    }
    // splice and splice back: the page's strokes and their identities as they were
    let before = ids(&p);
    let plan = lasso_cut_plan(&p, &links, &l, DPI, lasso_enclosed(&p, &l, DPI));
    let sp = plan_splice(&mut p, &plan.whole, plan.cut);
    check!(p.strokes.len() == before.len() + 1);
    check!(sp.sel == vec![1, 2] && sp.outside == vec![0]);
    let back = plan_unsplice(&mut p, sp);
    check!(ids(&p) == before);
    check!(back.len() == 1 && back[0].0 == 0 && back[0].1.len() == 2);
}

// ---- 6. one step: an exact move, cut and delete undone and redone ----

fn exact_history() {
    let (w, h) = (800, 600);
    let mut p = page_new(w, h, PAPER_NARROW);
    page_add(&mut p, line(TOOL_BALLPOINT, 100.0, 100.0, 400.0, 100.0, 1));
    page_add(&mut p, line(TOOL_PENCIL, 100.0, 140.0, 400.0, 160.0, 2));
    page_add(&mut p, line(TOOL_BALLPOINT, 260.0, 120.0, 300.0, 130.0, 3));
    page_add(&mut p, line(TOOL_MARKER, 600.0, 400.0, 700.0, 450.0, 4));
    let start = ids(&p);
    let start_px = render(&p).px;
    let l = Lasso::new(&[(250.0, 50.0), (500.0, 50.0), (500.0, 250.0), (250.0, 250.0)]).unwrap();
    for gesture in 0..2 {
        let mut hist = History::default();
        history_init(&mut hist);
        let plan = lasso_cut_plan(&p, &[], &l, DPI, lasso_enclosed(&p, &l, DPI));
        check!(plan.cut.len() == 2 && plan.whole == vec![2]);
        if plan.cut.len() != 2 {
            return;
        }
        let whole = plan.whole.clone();
        let sp = plan_splice(&mut p, &whole, plan.cut);
        let (mut sp_originals, sp_whole) = (sp.originals, sp.whole);
        let (sp_sel, sp_outside) = (sp.sel, sp.outside);
        // the taken ink off the spliced page
        let mut taken = Vec::new();
        for &i in sp_sel.iter().rev() {
            taken.push((i, page_remove(&mut p, i as i32)));
        }
        let outside_final: Vec<usize> = sp_outside.iter().map(|&o| o - sp_sel.iter().filter(|&&i| i < o).count()).collect();
        // the move puts the taken ink back at the end, moved (a stand-in for lasso_place)
        let mut added_at = outside_final.clone();
        if gesture == 0 {
            for (_, s) in taken.iter().rev() {
                let mut c = Box::new((**s).clone());
                for q in c.pts.iter_mut() {
                    q.y += 300.0;
                }
                c.bbox.y += 300;
                page_add(&mut p, c);
                added_at.push(p.strokes.len() - 1);
            }
        }
        // the history: the originals and the whole ink, at their places before (inside pieces dropped)
        let mut removed: Vec<(usize, Box<Stroke>)> = std::mem::take(&mut sp_originals);
        for (i, s) in taken {
            if let Some(&(old, _)) = sp_whole.iter().find(|w| w.1 == i) {
                removed.push((old, s));
            }
        }
        removed.sort_by_key(|r| r.0);
        let idx: Vec<i32> = removed.iter().map(|r| r.0 as i32).collect();
        let done = ids(&p);
        check!(added_at.windows(2).all(|w| w[0] < w[1]));
        history_push_lasso_at(&mut hist, &p, &idx, removed.into_iter().map(|r| r.1).collect(), &added_at, None);
        check!(hist.pushes == 1);
        let mut d = RECT_EMPTY;
        check!(history_undo(&mut hist, &mut p, &mut d));
        check!(ids(&p) == start);
        check!(render(&p).px == start_px);
        check!(history_redo(&mut hist, &mut p, &mut d));
        check!(ids(&p) == done);
        check!(history_undo(&mut hist, &mut p, &mut d) && ids(&p) == start);
    }
}

// ---- the dense page ----

/// The exact-area lift on the dense page (the owner's with GRIMOIRE_DENSE_PAGE=a copy of p72.qpg, else a
/// generated 1,090-stroke one): printed with --nocapture.
fn dense_page_exact_cost() {
    let p = if let Ok(path) = std::env::var("GRIMOIRE_DENSE_PAGE") {
        store_set_device(DPI, 1860, 2480);
        let buf = std::fs::read(&path).expect("the page file");
        let mut out = None;
        assert!(page_deserialize(&buf, &mut out) == 0);
        *out.unwrap()
    } else {
        SEED.store(0xD0D0, Ordering::Relaxed);
        let mut p = page_new(1860, 2480, PAPER_NARROW);
        for _ in 0..1090 {
            let t = [TOOL_BALLPOINT, TOOL_PENCIL, TOOL_FINELINER, TOOL_MARKER][(rnd() % 4) as usize];
            page_add(&mut p, stroke(t, TIP_FIRST, 1860.0, 2480.0));
        }
        *p
    };
    let (w, h) = (p.w as f32, p.h as f32);
    let l = Lasso::new(&[(w * 0.2, h * 0.3), (w * 0.8, h * 0.3 + 10.0), (w * 0.8 - 5.0, h * 0.55), (w * 0.2 + 8.0, h * 0.55 - 4.0)]).unwrap();
    let t0 = std::time::Instant::now();
    let on = lasso_enclosed(&p, &l, DPI);
    let whole_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t0 = std::time::Instant::now();
    let plan = lasso_cut_plan(&p, &[], &l, DPI, on);
    let plan_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let pieces: usize = plan.cut.iter().map(|c| c.1.len()).sum();
    let (nw, nc) = (plan.whole.len(), plan.cut.len());
    let mut q = p.clone();
    let before: Vec<StrokeId> = q.strokes.iter().map(|s| stroke_id(s)).collect();
    let t0 = std::time::Instant::now();
    let sp = plan_splice(&mut q, &plan.whole, plan.cut);
    let back = plan_unsplice(&mut q, sp);
    let splice_ms = t0.elapsed().as_secs_f64() * 1000.0;
    check!(q.strokes.iter().map(|s| stroke_id(s)).collect::<Vec<_>>() == before);
    check!(back.len() == nc);
    eprintln!("  dense page ({} strokes): whole-strokes enclosure {whole_ms:.1} ms; exact plan {plan_ms:.1} ms: {nw} whole, {nc} cut into {pieces} pieces; splice and back {splice_ms:.2} ms", p.strokes.len());
}

#[test]
fn test_lasso_cut() {
    run!(dab_log_is_the_draw);
    run!(cut_draws_the_stroke);
    run!(pieces_round_trip);
    run!(rect_inside_matches_brute_force);
    run!(cut_plan_rules);
    run!(exact_history);
    run!(dense_page_exact_cost);
    done();
}
