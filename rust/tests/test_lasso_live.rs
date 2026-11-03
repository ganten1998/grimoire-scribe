// Not a port of a C test: the lasso's live selection preview (lasso_live.rs, docs/beta2/PIXEL-LASSO.md 2
// and 4, tests 7 and 8): folded in batches of any size, the preview's answer equals the lift's
// (lasso_enclosed, lasso_select) for random lassos (loops, figure 8s, back-tracking, points off the page,
// a lasso past LASSO_MAX that drops every other point) on pages with erased ink, floors and beta.5 ink
// links; a stroke exactly half inside is decided the same both ways; and the preview's cost on a dense
// page (printed: cargo test --release --test test_lasso_live -- --nocapture; the owner's 1,090-stroke page
// with GRIMOIRE_DENSE_PAGE=/path/to/a/copy/p72.qpg, never the original).
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::sync::atomic::Ordering;
use std::time::Instant;
use testlib::*;

const DPI: f32 = 300.0;
/// The editor's LASSO_MAX (editor.rs): past it the lasso drops every other point.
const LASSO_MAX: usize = 4096;

static SEED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
fn rnd() -> u32 {
    let s = SEED.load(Ordering::Relaxed).wrapping_mul(1664525).wrapping_add(1013904223);
    SEED.store(s, Ordering::Relaxed);
    s >> 8
}
fn frand(lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rnd() % 10001) as f32 / 10000.0
}

fn wander(w: i32, h: i32, t: ToolKind) -> Box<Stroke> {
    let erase = t == TOOL_ERASER;
    let texture = if erase { ToolKind((rnd() % TOOL_COUNT as u32) as i32) } else { t };
    let mut prm = ToolParams::default();
    tool_params_default(texture, &mut prm);
    let seed = if erase { erase_seed(texture, rnd()) } else { rnd() & 0xFFFFFF };
    let mut s = stroke_new(t, &prm, seed);
    let (mut x, mut y) = (frand(-30.0, w as f32 + 30.0), frand(-30.0, h as f32 + 30.0));
    let mut ang = frand(0.0, 6.28);
    // dots, short marks and long wandering lines
    let n = match rnd() % 4 {
        0 => 1 + rnd() % 3,
        1 => 3 + rnd() % 10,
        _ => 10 + rnd() % 120,
    };
    let step = frand(1.0, 6.0);
    for k in 0..n {
        stroke_append(&mut s, &PenSample { x, y, pressure: frand(0.3, 1.0), t_ms: k * 6, ..Default::default() }, DPI);
        ang += frand(-0.5, 0.5);
        x += ang.cos() * step;
        y += ang.sin() * step;
    }
    s
}

fn link(pid: u32, seed: u32, area: Option<Rect>) -> NavLink {
    let mut l = NavLink { pid, seed, ..Default::default() };
    l.target.set("p2");
    if let Some(a) = area {
        l.area = a;
        l.has_area = true;
    }
    l
}

/// A page of handwriting-like ink with erase strokes, a scoped floor and beta.5 links (ink sharing a
/// seed), and its links.
fn fixture(w: i32, h: i32, n: usize) -> (Page, Vec<NavLink>) {
    let mut p = page_new(w, h, PAPER_NARROW);
    for k in 0..n {
        let t = if rnd() % 8 == 0 { TOOL_ERASER } else { [TOOL_BALLPOINT, TOOL_PENCIL, TOOL_FINELINER, TOOL_MARKER, TOOL_CALLIGRAPHY][(rnd() % 5) as usize] };
        if k == n / 2 {
            // a floor: a marker and erase strokes scoped to it
            page_add(&mut p, floor_marker_new(5));
        }
        let mut s = wander(w, h, t);
        if k > n / 2 && t == TOOL_ERASER && rnd() % 2 == 0 {
            s.ext[1] = 5;
        }
        page_add(&mut p, s);
    }
    // beta.5 links: two or three ink strokes share a link's seed
    let mut links = Vec::new();
    for g in 0..3u32 {
        let seed = 0xBE7A00 + g;
        let mut got = 0;
        for i in 0..p.strokes.len() {
            if got < 2 + g as usize && stroke_is_ink(&p.strokes[i]) && rnd() % 7 == 0 {
                p.strokes[i].seed = seed;
                got += 1;
            }
        }
        if got > 0 {
            links.push(link(1, seed, None));
        }
    }
    links.push(link(1, 0xA4EA, Some(Rect { x: w / 4, y: h / 4, w: w / 5, h: h / 6 })));
    (*p, links)
}

/// A random lasso: a loop, a figure 8, a back-tracking scribble, or one wandering off the page.
fn lasso_path(w: i32, h: i32) -> Vec<(f32, f32)> {
    let kind = rnd() % 4;
    let (cx, cy) = (frand(0.0, w as f32), frand(0.0, h as f32));
    let (rx, ry) = (frand(10.0, w as f32 * 0.6), frand(10.0, h as f32 * 0.6));
    let n = 8 + rnd() % 300;
    let mut out = Vec::with_capacity(n as usize);
    let mut ang = frand(0.0, 6.28);
    let (mut x, mut y) = (cx, cy);
    for k in 0..n {
        let f = k as f32 / n as f32 * std::f32::consts::TAU;
        let pt = match kind {
            0 => (cx + rx * f.cos() + frand(-3.0, 3.0), cy + ry * f.sin() + frand(-3.0, 3.0)),
            1 => (cx + rx * f.sin(), cy + ry * (2.0 * f).sin() / 2.0),
            2 => {
                ang += frand(-1.2, 1.2);
                x += ang.cos() * frand(2.0, 40.0);
                y += ang.sin() * frand(2.0, 40.0);
                (x, y)
            }
            _ => (cx + rx * 1.6 * f.cos(), cy + ry * 1.6 * f.sin()),
        };
        out.push(pt);
    }
    out
}

/// The preview fed path in batches of random size (one point at a time sometimes), dropping every other
/// point as the editor's lasso does past `max`: (its final answer, the lasso it answers for, batches
/// whose answer did not equal the lift's for the lasso as folded so far).
fn fold(p: &Page, links: &[NavLink], path: &[(f32, f32)], max: usize, check_each: bool) -> (Vec<bool>, Vec<(f32, f32)>, u32) {
    let mut live = LassoLive::new(p, links, DPI, path[0]);
    let mut pts: Vec<(f32, f32)> = vec![path[0]];
    let mut from = 1;
    let mut bad = 0;
    let mut k = 1;
    while k < path.len() {
        let take = if rnd() % 3 == 0 { 1 } else { 1 + (rnd() % 40) as usize };
        let mut halved = false;
        for &q in &path[k..(k + take).min(path.len())] {
            if pts.len() >= max {
                let mut c = 0;
                pts.retain(|_| {
                    c += 1;
                    c % 2 == 1
                });
                halved = true;
            }
            pts.push(q);
        }
        k += take;
        if halved {
            live.rebuild(p, &pts);
        } else {
            live.extend(p, &pts, from);
        }
        from = pts.len();
        if check_each {
            if let Some(l) = Lasso::new(&pts) {
                if live.enclosed() != lasso_enclosed(p, &l, DPI) || live.windings_off(&pts) != 0 {
                    bad += 1;
                }
            }
        }
    }
    (live.enclosed(), pts, bad)
}

fn preview_equals_lift() {
    SEED.store(0x11FE, Ordering::Relaxed);
    let mut bad_final = 0;
    let mut bad_each = 0;
    let mut bad_sel = 0;
    let mut selected = 0;
    for round in 0..500 {
        if round % 50 == 0 {
            // (a new page every 50 lassos)
        }
        let (w, h) = (900, 1100);
        SEED.store(0x5EED + round / 50, Ordering::Relaxed);
        let (p, links) = fixture(w, h, 220);
        SEED.store(0x1A55 + round * 7919, Ordering::Relaxed);
        let path = lasso_path(w, h);
        let (got, pts, bad) = fold(&p, &links, &path, LASSO_MAX, round % 10 == 0);
        bad_each += bad;
        let Some(l) = Lasso::new(&pts) else { continue };
        let want = lasso_enclosed(&p, &l, DPI);
        if got != want {
            bad_final += 1;
            eprintln!("  preview != lift: round {round}");
        }
        selected += want.iter().filter(|&&b| b).count();
        if lasso_select_from(&p, &links, &l, got) != lasso_select(&p, &links, &l, DPI) {
            bad_sel += 1;
        }
    }
    check!(bad_final == 0);
    check!(bad_each == 0);
    check!(bad_sel == 0);
    check!(selected > 500); // (the lassos do select things)
    // a lasso past LASSO_MAX: a small maximum makes it drop every other point many times
    let mut bad = 0;
    for round in 0..40 {
        SEED.store(0x8A1F + round, Ordering::Relaxed);
        let (p, links) = fixture(900, 1100, 150);
        let path = lasso_path(900, 1100);
        let (got, pts, b) = fold(&p, &links, &path, 32, true);
        bad += b;
        if let Some(l) = Lasso::new(&pts) {
            bad += (got != lasso_enclosed(&p, &l, DPI)) as u32;
        }
    }
    check!(bad == 0);
}

/// A removed closing edge takes away exactly what it added: an edge whose crossing of a row, computed
/// from its other end, lands an ulp apart, and a sample exactly at the larger of the two. Folding a point
/// after it (the edge goes) leaves every winding the table's.
fn removed_edge_subtracts_exactly() {
    SEED.store(0xE06E, Ordering::Relaxed);
    let mut found = 0;
    let mut off = 0;
    for _ in 0..20000 {
        let a = (frand(100.0, 700.0), frand(100.0, 700.0));
        let b = (frand(100.0, 700.0), frand(100.0, 700.0));
        let Some((first, last, _)) = edge_rows(a, b) else { continue };
        if last < first {
            continue;
        }
        let r = first + (rnd() as i32 % (last - first + 1));
        let (Some(xf), Some(xr)) = (edge_cross(a, b, r), edge_cross(b, a, r)) else { continue };
        if xf == xr {
            continue;
        }
        // a straight 128 px stroke of 11 one-mm pieces whose middle piece's sample sits exactly at x
        let x = xf.max(xr);
        let y = r as f32 + 0.25;
        let mut p = page_new(900, 900, PAPER_BLANK);
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_FINELINER, &mut prm);
        let mut st = stroke_new(TOOL_FINELINER, &prm, 1);
        stroke_append(&mut st, &PenSample { x: x - 64.0, y, pressure: 0.6, ..Default::default() }, DPI);
        stroke_append(&mut st, &PenSample { x: x + 64.0, y, pressure: 0.6, t_ms: 8, ..Default::default() }, DPI);
        page_add(&mut p, st);
        let j = judged_samples(&p, None, 0, DPI, &[]).unwrap();
        if !j.samples.iter().any(|q| q.0 == x && q.1.floor() as i32 == r) {
            continue;
        }
        found += 1;
        // the lasso: from b round to a (its closing edge a -> b), then on to a third point
        let c = (a.0 + (b.1 - a.1), a.1 - (b.0 - a.0));
        let mut live = LassoLive::new(&p, &[], DPI, b);
        let mut pts = vec![b, ((a.0 + b.0) / 2.0 + 300.0, (a.1 + b.1) / 2.0), a];
        live.extend(&p, &pts, 1);
        off += live.windings_off(&pts);
        pts.push(c);
        live.extend(&p, &pts, 3);
        off += live.windings_off(&pts);
        if found >= 200 {
            break;
        }
    }
    check!(found >= 50);
    check!(off == 0);
}

fn shown_follows_links_and_flips() {
    // a beta.5 link's ink: all of it shows once half its length is inside, none before
    let mut p = page_new(800, 600, PAPER_BLANK);
    let mk = |pts: &[(f32, f32)], seed: u32| {
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_BALLPOINT, &mut prm);
        let mut s = stroke_new(TOOL_BALLPOINT, &prm, seed);
        for (k, &(x, y)) in pts.iter().enumerate() {
            stroke_append(&mut s, &PenSample { x, y, pressure: 0.8, t_ms: k as u32 * 8, ..Default::default() }, DPI);
        }
        s
    };
    page_add(&mut p, mk(&[(100.0, 100.0), (180.0, 100.0)], 0x55));
    page_add(&mut p, mk(&[(300.0, 100.0), (380.0, 100.0)], 0x55));
    page_add(&mut p, mk(&[(100.0, 300.0), (180.0, 300.0)], 7));
    let links = [link(1, 0x55, None)];
    // a lasso drawn around the first stroke, then on around the third
    let path = [(80.0, 80.0), (200.0, 80.0), (200.0, 120.0), (80.0, 120.0), (80.0, 200.0), (80.0, 320.0), (200.0, 320.0), (200.0, 280.0), (90.0, 280.0)];
    let mut live = LassoLive::new(&p, &links, DPI, path[0]);
    let mut pts = vec![path[0]];
    let mut flips_at = Vec::new();
    for &q in &path[1..] {
        pts.push(q);
        let f = live.extend(&p, &pts, pts.len() - 1);
        flips_at.push(f);
    }
    // the first stroke is half of its link's length: both strokes show together
    check!(live.shown(0) && live.shown(1));
    check!(!live.enclosed()[1]);
    check!(live.shown(2));
    check!(flips_at.iter().any(|f| f.contains(&0) && f.contains(&1)));
    let l = Lasso::new(&pts).unwrap();
    check!(lasso_select(&p, &links, &l, DPI).strokes == live.shown_list());
    // a stroke taken in and left out again: shown, then not
    let mut q = page_new(800, 600, PAPER_BLANK);
    page_add(&mut q, mk(&[(300.0, 300.0), (340.0, 300.0)], 9));
    let mut live = LassoLive::new(&q, &[], DPI, (250.0, 250.0));
    let mut pts = vec![(250.0, 250.0), (400.0, 250.0), (400.0, 350.0), (250.0, 350.0)];
    let f1 = live.extend(&q, &pts, 1);
    check!(f1 == vec![0] && live.shown(0));
    // on round a pocket that opens to the left, past the lasso's start: the stroke is outside again
    pts.extend_from_slice(&[(250.0, 330.0), (380.0, 330.0), (380.0, 270.0), (250.0, 270.0)]);
    let f2 = live.extend(&q, &pts, 4);
    let l = Lasso::new(&pts).unwrap();
    check!(live.enclosed() == lasso_enclosed(&q, &l, DPI));
    check!(f2 == vec![0] && !live.shown(0));
    // an edit to the page makes the preview stale
    check!(live.holds(&q));
    page_add(&mut q, mk(&[(10.0, 10.0), (20.0, 10.0)], 3));
    check!(!live.holds(&q));
}

fn half_inside_decided_alike() {
    // a straight stroke split into an even number of 1 mm pieces, a lasso edge exactly between its two
    // halves: inside == half the total, selected (at least half), by the lift and the preview alike
    let mut p = page_new(800, 600, PAPER_BLANK);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_FINELINER, &mut prm);
    let mut s = stroke_new(TOOL_FINELINER, &prm, 4);
    let step = mm_to_px(1.0, DPI);
    let len = step * 8.0;
    stroke_append(&mut s, &PenSample { x: 100.0, y: 200.0, pressure: 0.6, ..Default::default() }, DPI);
    stroke_append(&mut s, &PenSample { x: 100.0 + len, y: 200.0, pressure: 0.6, t_ms: 10, ..Default::default() }, DPI);
    page_add(&mut p, s);
    let mid = 100.0 + len / 2.0;
    let pts = [(mid, 150.0), (mid + 400.0, 150.0), (mid + 400.0, 250.0), (mid, 250.0)];
    let l = Lasso::new(&pts).unwrap();
    let e = lasso_enclosed(&p, &l, DPI);
    check!(e == vec![true]);
    let mut live = LassoLive::new(&p, &[], DPI, pts[0]);
    live.extend(&p, &pts, 1);
    check!(live.enclosed() == e);
    // one piece fewer inside: not selected either way
    let e = mid + step / 2.0 + 1.0;
    let pts2 = [(e, 150.0), (mid + 400.0, 150.0), (mid + 400.0, 250.0), (e, 250.0)];
    let l2 = Lasso::new(&pts2).unwrap();
    let e2 = lasso_enclosed(&p, &l2, DPI);
    check!(e2 == vec![false]);
    let mut live = LassoLive::new(&p, &[], DPI, pts2[0]);
    live.extend(&p, &pts2, 1);
    check!(live.enclosed() == e2);
}

/// Exact area: a short stroke (an i-dot, an accent: under LASSO_SHORT_MM of what shows) is judged whole
/// by the preview's halo (by the middle of what shows), and the lift (lasso_cut_plan) takes exactly what
/// the halo showed: never a piece of a dot with no halo, never part of a dot whose whole halo showed.
fn exact_short_strokes_lift_as_shown() {
    let dot = |x0: f32, y: f32, len: f32, seed: u32| {
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_BALLPOINT, &mut prm);
        let mut s = stroke_new(TOOL_BALLPOINT, &prm, seed);
        stroke_append(&mut s, &PenSample { x: x0, y, pressure: 0.6, ..Default::default() }, DPI);
        stroke_append(&mut s, &PenSample { x: x0 + len, y, pressure: 0.6, t_ms: 8, ..Default::default() }, DPI);
        s
    };
    // the owner's case: a dot 12 px (about 1 mm) long, the lasso's right edge swept across it
    let mut p = page_new(800, 600, PAPER_BLANK);
    page_add(&mut p, dot(500.0, 300.0, 12.0, 1));
    check!(judged_samples(&p, None, 0, DPI, &[]).is_some_and(|j| !j.long));
    let mut bad = 0;
    for e in 490..=525 {
        let e = e as f32;
        let pts = [(e - 200.0, 200.0), (e, 200.0), (e, 400.0), (e - 200.0, 400.0)];
        let l = Lasso::new(&pts).unwrap();
        let mut live = LassoLive::new(&p, &[], DPI, pts[0]);
        live.set_exact(true);
        live.extend(&p, &pts, 1);
        let halo = live.halo_list();
        let plan = lasso_cut_plan(&p, &[], &l, DPI, live.enclosed());
        let shown_whole = halo.iter().any(|h| h.0 == 0 && h.1.is_none());
        let shown_runs = halo.iter().any(|h| h.0 == 0 && h.1.is_some());
        if shown_runs || !plan.cut.is_empty() || shown_whole != (plan.whole == vec![0]) {
            bad += 1;
        }
    }
    check!(bad == 0);
    // random short marks and random lassos: every short stroke lifted as its halo showed
    SEED.store(0xD07, Ordering::Relaxed);
    let mut q = page_new(800, 600, PAPER_BLANK);
    for k in 0..300 {
        let t = [TOOL_BALLPOINT, TOOL_FINELINER, TOOL_MARKER, TOOL_PENCIL][(rnd() % 4) as usize];
        let mut prm = ToolParams::default();
        tool_params_default(t, &mut prm);
        let mut s = stroke_new(t, &prm, k + 10);
        let (x, y) = (frand(20.0, 780.0), frand(20.0, 580.0));
        let (dx, dy) = (frand(-10.0, 10.0), frand(-10.0, 10.0));
        for j in 0..=(rnd() % 3) {
            let f = j as f32 / 2.0;
            stroke_append(&mut s, &PenSample { x: x + dx * f, y: y + dy * f, pressure: frand(0.3, 1.0), t_ms: j * 6, ..Default::default() }, DPI);
        }
        page_add(&mut q, s);
    }
    let erasers = page_erasers(&q);
    let short: Vec<bool> = (0..q.strokes.len()).map(|i| judged_samples(&q, None, i, DPI, &erasers).is_some_and(|j| !j.long)).collect();
    check!(short.iter().filter(|&&s| s).count() >= 100);
    let (mut bad, mut seen) = (0, 0);
    for _ in 0..40 {
        let path = lasso_path(800, 600);
        let mut live = LassoLive::new(&q, &[], DPI, path[0]);
        live.set_exact(true);
        live.extend(&q, &path, 1);
        let Some(l) = Lasso::new(&path) else { continue };
        let halo = live.halo_list();
        let plan = lasso_cut_plan(&q, &[], &l, DPI, live.enclosed());
        for i in (0..q.strokes.len()).filter(|&i| short[i]) {
            let whole = halo.iter().any(|h| h.0 == i && h.1.is_none());
            seen += whole as u32;
            if halo.iter().any(|h| h.0 == i && h.1.is_some()) || plan.cut.iter().any(|c| c.0 == i) || whole != plan.whole.contains(&i) {
                bad += 1;
            }
        }
    }
    check!(seen >= 20);
    check!(bad == 0);
}

/// A page whose ink lies wholly left of (or above) the grid's sane limit (a corrupt or crafted page loads:
/// only non-finite coordinates are refused) makes a preview with no grid, not a panic.
fn far_off_ink_makes_no_grid() {
    for (x0, y0) in [(-100000.0f32, 100.0f32), (100.0, -100000.0), (-100000.0, -100000.0)] {
        let mut p = page_new(800, 600, PAPER_BLANK);
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_BALLPOINT, &mut prm);
        let mut s = stroke_new(TOOL_BALLPOINT, &prm, 1);
        stroke_append(&mut s, &PenSample { x: x0, y: y0, pressure: 0.6, ..Default::default() }, DPI);
        stroke_append(&mut s, &PenSample { x: x0 + 100.0, y: y0, pressure: 0.6, t_ms: 8, ..Default::default() }, DPI);
        page_add(&mut p, s);
        let pts = [(100.0, 100.0), (300.0, 100.0), (300.0, 300.0), (100.0, 300.0)];
        let mut live = LassoLive::new(&p, &[], DPI, pts[0]);
        live.extend(&p, &pts, 1);
        check!(live.enclosed() == vec![false]);
        check!(live.halo_list().is_empty());
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// The dense page: the owner's (GRIMOIRE_DENSE_PAGE, a copy) or a generated one of 1,090 strokes.
fn dense_page() -> (Page, String) {
    if let Ok(path) = std::env::var("GRIMOIRE_DENSE_PAGE") {
        store_set_device(DPI, 1860, 2480);
        let buf = std::fs::read(&path).expect("the page file");
        let mut out = None;
        if page_deserialize(&buf, &mut out) == 0 {
            if let Some(p) = out {
                return (*p, path);
            }
        }
        eprintln!("  (GRIMOIRE_DENSE_PAGE {path} did not read: a generated page instead)");
    }
    SEED.store(0xD0D0, Ordering::Relaxed);
    let (w, h) = (1860, 2480);
    let mut p = page_new(w, h, PAPER_NARROW);
    for _ in 0..1090 {
        let t = if rnd() % 10 == 0 { TOOL_ERASER } else { [TOOL_BALLPOINT, TOOL_PENCIL, TOOL_FINELINER, TOOL_MARKER][(rnd() % 4) as usize] };
        page_add(&mut p, wander(w, h, t));
    }
    (*p, "generated".into())
}

fn dense_page_preview_cost() {
    let (p, what) = dense_page();
    let npts: usize = p.strokes.iter().map(|s| s.pts.len()).sum();
    // a lasso around a paragraph, drawn as a pen does (a point every 2 px), folded every 24 points (a
    // batch of 120 ms at a quick 400 px a second... and more)
    let (w, h) = (p.w as f32, p.h as f32);
    let (x0, y0, x1, y1) = (w * 0.2, h * 0.3, w * 0.8, h * 0.55);
    let corners = [(x0, y0), (x1, y0 + 10.0), (x1 - 5.0, y1), (x0 + 8.0, y1 - 4.0), (x0, y0 + 2.0)];
    let mut path = Vec::new();
    for c in corners.windows(2) {
        let (a, b) = (c[0], c[1]);
        let n = (((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() / 2.0) as usize;
        for k in 0..n {
            let f = k as f32 / n as f32;
            path.push((a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f));
        }
    }
    let t0 = Instant::now();
    let mut live = LassoLive::new(&p, &[], DPI, path[0]);
    let setup = ms(t0);
    let mut worst = 0.0f64;
    let mut all = 0.0f64;
    let mut pts = vec![path[0]];
    let mut from = 1;
    for chunk in path[1..].chunks(24) {
        pts.extend_from_slice(chunk);
        let t = Instant::now();
        live.extend(&p, &pts, from);
        let d = ms(t);
        worst = worst.max(d);
        all += d;
        from = pts.len();
    }
    let l = Lasso::new(&pts).unwrap();
    let t = Instant::now();
    let want = lasso_enclosed(&p, &l, DPI);
    let lift = ms(t);
    check!(live.enclosed() == want);
    let n = want.iter().filter(|&&b| b).count();
    eprintln!(
        "  dense page ({what}: {} strokes, {npts} points): preview setup {setup:.2} ms; {} batches, {:.2} ms in all, worst {worst:.2} ms, mean {:.3} ms; {} samples tested, {} strokes made; the lift's own select {lift:.1} ms; {n} selected",
        p.strokes.len(),
        live.batches,
        all,
        all / live.batches.max(1) as f64,
        live.tests,
        live.made
    );
}

#[test]
fn test_lasso_live() {
    run!(preview_equals_lift);
    run!(removed_edge_subtracts_exactly);
    run!(shown_follows_links_and_flips);
    run!(half_inside_decided_alike);
    run!(exact_short_strokes_lift_as_shown);
    run!(far_off_ink_makes_no_grid);
    run!(dense_page_preview_cost);
    done();
}
