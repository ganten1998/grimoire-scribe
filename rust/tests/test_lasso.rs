// Not a port of a C test: lasso select, copy, cut, paste and move, the pure part (lasso.rs,
// clipboard.rs, history.rs ActKind::Lasso, nav.rs nav_target_across; docs/beta2/LASSO-DESIGN.md 10).
//
// The polygon against an exact test; enclosure on handwriting-like strokes (dots and crossbars come,
// an underline through the lasso does not, a stroke whose tail is erased is judged by what shows); link
// selection; which erase strokes go with the ink; seeds; targets across notebooks; where a paste goes;
// layers (a history's too, and the refusal at 255); placing equals drawing the page whole, plain and
// scoped; the move oracle; the clipboard file; lasso undo and redo; and the time a move and a paste take
// on a 1,000-stroke page (printed: cargo test --release --test test_lasso -- --nocapture).
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use testlib::*;

const DPI: f32 = 300.0;

// What the code under test logs (grimoire.log is its stderr), from log_capture to log_captured (as
// test_annotate.rs has it).
static G_LOG_SAVED: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

fn g_log_path() -> String {
    format!("{}/lasso-captured.log", tl_tmp())
}

fn log_capture() {
    // (stderr is not buffered here: the lines of earlier checks are out already)
    // SAFETY: dup, open and dup2 on descriptor 2 and one this function opens, which it closes again.
    unsafe {
        G_LOG_SAVED.store(libc::dup(2), Ordering::Relaxed);
        let path = std::ffi::CString::new(g_log_path()).unwrap();
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
}

fn log_captured() -> Option<String> {
    let saved = G_LOG_SAVED.swap(-1, Ordering::Relaxed);
    if saved >= 0 {
        // SAFETY: puts descriptor 2 back from the copy log_capture made, and closes the copy.
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
    let t = std::fs::read(g_log_path()).ok().map(|t| String::from_utf8_lossy(&t).into_owned());
    if let Some(t) = &t {
        eprint!("{}", t);
    }
    t
}

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
fn canvas(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::default();
    assert!(canvas_init(&mut c, w, h) == 0);
    c
}
fn render(p: &Page) -> Canvas {
    let mut c = canvas(p.w, p.h);
    page_render(&mut c, p, whole(p), DPI);
    c
}
fn paper_of(p: &Page) -> Rc<Vec<u8>> {
    let mut c = canvas(p.w, p.h);
    page_paper_render(&mut c, p, whole(p), DPI);
    Rc::new(c.px)
}

fn poly(t: ToolKind, pts: &[(f32, f32)], size_mm: f32, seed: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    prm.size_min_mm = size_mm;
    prm.size_max_mm = size_mm;
    let mut s = stroke_new(t, &prm, seed);
    let mut k = 0;
    for w in pts.windows(2) {
        for j in 0..8 {
            let f = j as f32 / 8.0;
            stroke_append(&mut s, &PenSample { x: w[0].0 + (w[1].0 - w[0].0) * f, y: w[0].1 + (w[1].1 - w[0].1) * f, pressure: 0.8, t_ms: k * 8, ..Default::default() }, DPI);
            k += 1;
        }
    }
    let l = pts[pts.len() - 1];
    stroke_append(&mut s, &PenSample { x: l.0, y: l.1, pressure: 0.8, t_ms: k * 8, ..Default::default() }, DPI);
    s
}
fn ink(pts: &[(f32, f32)], seed: u32) -> Box<Stroke> {
    poly(TOOL_BALLPOINT, pts, 0.6, seed)
}
fn eraser(pts: &[(f32, f32)], size_mm: f32) -> Box<Stroke> {
    poly(TOOL_ERASER, pts, size_mm, erase_seed(TOOL_ERASER, 3))
}
fn rect_lasso(x0: f32, y0: f32, x1: f32, y1: f32) -> Lasso {
    Lasso::new(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]).unwrap()
}

// ---- 1. the polygon ----

fn exact_inside(pts: &[(f32, f32)], x: f32, y: f32) -> bool {
    // non-zero winding, exactly
    let mut w = 0;
    let n = pts.len();
    for k in 0..n {
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        if a.1 <= y {
            if b.1 > y && (b.0 - a.0) * (y - a.1) - (x - a.0) * (b.1 - a.1) > 0.0 {
                w += 1;
            }
        } else if b.1 <= y && (b.0 - a.0) * (y - a.1) - (x - a.0) * (b.1 - a.1) < 0.0 {
            w -= 1;
        }
    }
    w != 0
}

fn polygon_matches_exact_test() {
    SEED.store(0x1A55, Ordering::Relaxed);
    let mut bad = 0;
    for _ in 0..40 {
        let n = 3 + rnd() % 30;
        let pts: Vec<(f32, f32)> = (0..n).map(|_| (frand(0.0, 400.0), frand(0.0, 400.0))).collect();
        let l = Lasso::new(&pts).unwrap();
        for _ in 0..400 {
            // at row middles, as the table samples
            let (x, y) = (frand(-10.0, 410.0), (rnd() % 420) as f32 - 10.0 + 0.5);
            if l.inside(x, y) != exact_inside(&pts, x, y) {
                bad += 1;
            }
        }
    }
    check!(bad == 0);
    // a figure 8: both loops are inside, the crossing point region too
    let eight = [(0.0, 0.0), (100.0, 100.0), (100.0, 0.0), (0.0, 100.0)];
    let l = Lasso::new(&eight).unwrap();
    check!(l.inside(20.0, 50.5) && l.inside(80.0, 50.5));
    check!(!l.inside(50.0, 10.5) && !l.inside(150.0, 50.5));
    check!(Lasso::new(&[(0.0, 0.0), (1.0, 1.0)]).is_none());
}

// ---- 2. enclosure ----

fn enclosure_on_handwriting() {
    let mut p = page_new(800, 600, PAPER_NARROW);
    // a word: a stroke, an i-dot, a crossbar
    page_add(&mut p, ink(&[(100.0, 200.0), (130.0, 150.0), (160.0, 200.0), (190.0, 150.0), (220.0, 200.0)], 1));
    page_add(&mut p, ink(&[(150.0, 130.0), (151.0, 131.0)], 2));
    page_add(&mut p, ink(&[(170.0, 165.0), (200.0, 165.0)], 3));
    // a long underline through the lasso: most of it outside
    page_add(&mut p, ink(&[(50.0, 215.0), (700.0, 215.0)], 4));
    // a word beside, outside the lasso
    page_add(&mut p, ink(&[(400.0, 200.0), (450.0, 150.0)], 5));
    let l = rect_lasso(90.0, 120.0, 240.0, 225.0);
    let sel = lasso_select(&p, &[], &l, DPI);
    check!(sel.strokes == vec![0, 1, 2]);
    // a stroke whose long tail (outside the lasso) is erased: judged by what shows, inside
    let mut q = page_new(800, 600, PAPER_NARROW);
    page_add(&mut q, ink(&[(100.0, 300.0), (200.0, 300.0), (600.0, 300.0)], 7));
    page_add(&mut q, eraser(&[(210.0, 300.0), (620.0, 300.0)], 3.0));
    let l2 = rect_lasso(80.0, 270.0, 205.0, 330.0);
    check!(lasso_select(&q, &[], &l2, DPI).strokes == vec![0]);
    // the same without the eraser: most of it is outside
    let mut r = page_new(800, 600, PAPER_NARROW);
    page_add(&mut r, ink(&[(100.0, 300.0), (200.0, 300.0), (600.0, 300.0)], 7));
    check!(lasso_select(&r, &[], &l2, DPI).strokes.is_empty());
    // erase strokes are never selected, alone or with ink
    let mut w = page_new(800, 600, PAPER_NARROW);
    page_add(&mut w, poly(TOOL_ERASER, &[(100.0, 100.0), (200.0, 100.0)], 2.0, erase_seed(TOOL_PENCIL, 9)));
    check!(lasso_select(&w, &[], &rect_lasso(50.0, 50.0, 300.0, 150.0), DPI) == Selection::default());
    check!(stroke_visible(&q, 0, DPI));
    let mut gone = page_new(400, 400, PAPER_BLANK);
    page_add(&mut gone, ink(&[(100.0, 100.0), (200.0, 100.0)], 1));
    page_add(&mut gone, eraser(&[(90.0, 100.0), (210.0, 100.0)], 4.0));
    check!(!stroke_visible(&gone, 0, DPI));
}

// ---- 3. links ----

fn link(pid: u32, seed: u32, target: &str, area: Option<Rect>) -> NavLink {
    let mut l = NavLink { pid, seed, ..Default::default() };
    l.target.set(target);
    if let Some(a) = area {
        l.area = a;
        l.has_area = true;
    }
    l
}

fn links_come_along() {
    let mut p = page_new(800, 600, PAPER_BLANK);
    page_add(&mut p, ink(&[(100.0, 100.0), (150.0, 120.0)], 10)); // in area A
    page_add(&mut p, ink(&[(400.0, 100.0), (450.0, 120.0)], 0x55)); // beta.5 link's ink, part 1
    page_add(&mut p, ink(&[(470.0, 100.0), (520.0, 120.0)], 0x55)); // part 2
    let links = [
        link(1, 0xA, "p2", Some(Rect { x: 90, y: 90, w: 70, h: 40 })),     // area half inside
        link(1, 0xB, "p3", Some(Rect { x: 600, y: 400, w: 50, h: 50 })),   // far away
        link(1, 0x55, "index", None),                                      // beta.5: ink only
    ];
    // a lasso around area A only
    let s = lasso_select(&p, &links, &rect_lasso(80.0, 80.0, 140.0, 140.0), DPI);
    check!(s.links == vec![0]);
    // a lasso around most of the beta.5 ink (part 1 and half of part 2): all of it joins
    let s = lasso_select(&p, &links, &rect_lasso(380.0, 80.0, 500.0, 140.0), DPI);
    check!(s.links == vec![2]);
    check!(s.strokes == vec![1, 2]);
    // a lasso around only a small part: none of it
    let s = lasso_select(&p, &links, &rect_lasso(380.0, 80.0, 415.0, 140.0), DPI);
    check!(s.links.is_empty() && s.strokes.is_empty());
}

// ---- 4. carrying ----

fn erase_strokes_go_with_their_ink() {
    let mut p = page_new(600, 400, PAPER_NARROW);
    page_add(&mut p, eraser(&[(100.0, 100.0), (200.0, 100.0)], 2.0)); // 0: before the ink: not carried
    page_add(&mut p, ink(&[(100.0, 100.0), (200.0, 100.0)], 1)); // 1: selected
    page_add(&mut p, ink(&[(100.0, 300.0), (200.0, 300.0)], 2)); // 2: not selected
    page_add(&mut p, eraser(&[(150.0, 90.0), (150.0, 110.0)], 2.0)); // 3: over 1: carried
    page_add(&mut p, eraser(&[(150.0, 290.0), (150.0, 310.0)], 2.0)); // 4: over 2 only: not carried
    page_add(&mut p, eraser(&[(150.0, 80.0), (150.0, 320.0)], 2.0)); // 5: crosses both: carried (and stays)
    check!(lasso_carried(&p, &[1], DPI) == vec![3, 5]);
    // scoped at the source to a floor after the ink: it never touched it
    let mut q = page_new(600, 400, PAPER_NARROW);
    page_add(&mut q, ink(&[(100.0, 100.0), (200.0, 100.0)], 1));
    page_add(&mut q, floor_marker_new(4));
    let mut e = eraser(&[(150.0, 90.0), (150.0, 110.0)], 2.0);
    e.ext[1] = 4;
    page_add(&mut q, e);
    check!(lasso_carried(&q, &[0], DPI).is_empty());
    // the group: floor 0 at its start, the erase strokes scoped to it
    let g = lasso_lift(&p, &[1], Vec::new(), DPI);
    check!(g.strokes.len() == 4 && is_floor_marker(&g.strokes[0]) && g.strokes[0].ext[1] == 1);
    check!(stroke_is_ink(&g.strokes[1]) && g.strokes[2].ext[1] == 1 && g.strokes[3].ext[1] == 1);
    check!(g.carried == 2 && g.ink == 1);
}

// ---- 5. seeds ----

fn seeds_kept_when_free() {
    let mut src = page_new(400, 400, PAPER_BLANK);
    page_add(&mut src, ink(&[(10.0, 10.0), (50.0, 10.0)], 0x111));
    page_add(&mut src, ink(&[(10.0, 20.0), (50.0, 20.0)], 0x222));
    page_add(&mut src, ink(&[(60.0, 20.0), (90.0, 20.0)], 0x222)); // shares its seed
    page_add(&mut src, ink(&[(10.0, 30.0), (50.0, 30.0)], 0x333)); // a link's ink whose link stays
    let g = lasso_lift(&src, &[0, 1, 2, 3], vec![link(1, 0x222, "p4", None)], DPI);
    let mut dst = page_new(400, 400, PAPER_BLANK);
    page_add(&mut dst, ink(&[(10.0, 10.0), (50.0, 10.0)], 0x111)); // clash
    let src_links: HashSet<u32> = [0x222, 0x333].into_iter().collect();
    let (inkm, linkm) = lasso_seeds(&g, &dst, &[link(9, 0x444, "p1", None)], &[], &src_links, false);
    check!(inkm[&0x111] != 0x111); // clash: fresh
    check!(linkm[&0x222] != 0x222 && inkm[&0x222] == linkm[&0x222]); // paste: the link's fresh seed, both strokes
    check!(inkm[&0x333] != 0x333); // leaves its link
    let all: HashSet<u32> = inkm.values().chain(linkm.values()).copied().collect();
    check!(!all.contains(&0x444) && !all.contains(&0x111));
    // a move on its own page: links keep their seeds, ink keeps its own (the originals skipped)
    let (inkm, linkm) = lasso_seeds(&g, &src, &[link(1, 0x222, "p4", None)], &[0, 1, 2, 3], &src_links, true);
    check!(linkm[&0x222] == 0x222 && inkm[&0x222] == 0x222 && inkm[&0x111] == 0x111);
    check!(inkm[&0x333] != 0x333);
}

// ---- 6. targets, links, spots ----

fn targets_across_notebooks() {
    check!(nav_target_across("p12", "AAA", "BBB").as_deref() == Some("n:AAA:p12"));
    check!(nav_target_across("n:BBB:p7", "AAA", "BBB").as_deref() == Some("p7"));
    check!(nav_target_across("n:CCC:p7", "AAA", "BBB").as_deref() == Some("n:CCC:p7"));
    check!(nav_target_across("index", "AAA", "BBB").is_none());
    check!(nav_target_across("zz9", "AAA", "BBB").as_deref() == Some("zz9"));
    // a placed link: moved, scaled, clamped; an outline moves with it
    let mut l = link(3, 5, "p2", Some(Rect { x: 100, y: 100, w: 50, h: 40 }));
    l.outline = LinkOutline::from_points(&[(100.0, 100.0), (150.0, 100.0), (150.0, 140.0), (100.0, 140.0)], true, 1000, 1000);
    let m = link_placed(&l, Spot { dx: 30.0, dy: -20.0, scale: 1.0 }, 9, 77, "n:X:p2", 1000, 1000).unwrap();
    check!(m.pid == 9 && m.seed == 77 && m.target == "n:X:p2");
    check!(m.area == Rect { x: 130, y: 80, w: 50, h: 40 } && m.outline.n == l.outline.n && m.outline.xy[0] == (130, 80));
    let half = link_placed(&l, Spot { dx: 0.0, dy: 0.0, scale: 0.5 }, 9, 77, "p2", 1000, 1000).unwrap();
    check!(half.area == Rect { x: 50, y: 50, w: 25, h: 20 });
    check!(link_placed(&l, Spot { dx: 5000.0, dy: 0.0, scale: 1.0 }, 9, 77, "p2", 1000, 1000).is_none());
    // where a paste goes
    let view = Rect { x: 0, y: 0, w: 1860, h: 2480 };
    let b = Rect { x: 100, y: 200, w: 300, h: 100 };
    check!(paste_spot(b, view, 1860, 2480, false, None, DPI) == Spot { dx: 0.0, dy: 0.0, scale: 1.0 });
    let s = paste_spot(b, view, 1860, 2480, true, None, DPI);
    check!(s.dx == 35.0 && s.dy == 35.0); // 3 mm
    let s = paste_spot(b, Rect { x: 0, y: 1000, w: 1860, h: 1000 }, 1860, 2480, false, None, DPI);
    check!(s.dx == (930 - 150 - 100) as f32 && s.dy == (1500 - 50 - 200) as f32);
    let s = paste_spot(b, view, 1860, 2480, false, Some((1850, 10)), DPI);
    check!(s.dx == (1860 - 300 - 100) as f32 && s.dy == -200.0);
    let big = Rect { x: 0, y: 0, w: 1986, h: 2648 };
    let s = paste_spot(big, view, 1860, 2480, false, None, DPI);
    check!(s.scale < 1.0 && (1986.0 * s.scale).ceil() as i32 <= 1860 && (2648.0 * s.scale).ceil() as i32 <= 2480);
}

// ---- 7. layers ----

fn layers_skip_the_page_and_its_history() {
    let mut p = page_new(300, 300, PAPER_BLANK);
    let mut e = eraser(&[(10.0, 10.0), (20.0, 10.0)], 2.0);
    e.ext[1] = 1;
    page_add(&mut p, e);
    let mut busy = [false; FLOOR_LAYERS];
    layers_in_use(&p, &mut busy);
    check!(busy[0] && busy[1] && !busy[2]);
    let mut h = History::default();
    history_init(&mut h);
    let mut off = eraser(&[(10.0, 10.0), (20.0, 10.0)], 2.0);
    off.ext[1] = 2;
    history_push_erase(&mut h, &[5], vec![off]);
    history_layers_in_use(&h, &mut busy);
    check!(busy[2] && !busy[3]);
    // a placement that needs a layer when all are taken is refused
    let mut dst = page_new(300, 300, PAPER_BLANK);
    page_add(&mut dst, ink(&[(50.0, 150.0), (250.0, 150.0)], 1));
    let mut src = page_new(300, 300, PAPER_BLANK);
    page_add(&mut src, ink(&[(50.0, 150.0), (250.0, 150.0)], 2));
    page_add(&mut src, eraser(&[(150.0, 140.0), (150.0, 160.0)], 3.0));
    let g = lasso_lift(&src, &[0], Vec::new(), DPI);
    let mut c = render(&dst);
    let pr = paper_of(&dst);
    let full = [true; FLOOR_LAYERS];
    check!(lasso_place(&mut c, &g, Spot { dx: 0.0, dy: 0.0, scale: 1.0 }, &HashMap::new(), Some((&pr, dst.w)), &full, DPI).err() == Some(NoLayer { drawn: RECT_EMPTY }));
    check!(c.px == render(&dst).px);
    // a group whose ink comes before its floor (a re-lifted group with a nested floor): the refusal names
    // the rect that ink was already drawn in, so the caller can draw it again from the page
    let first = ink(&[(40.0, 60.0), (120.0, 80.0)], 9);
    let first_box = first.bbox;
    let mut later = eraser(&[(150.0, 140.0), (150.0, 160.0)], 3.0);
    later.ext[1] = 4;
    let g2 = Group { w: 300, h: 300, strokes: vec![first, floor_marker_new(4), later], box_: first_box, ink: 1, carried: 1, ..Default::default() };
    let mut c = render(&dst);
    let e = lasso_place(&mut c, &g2, Spot { dx: 0.0, dy: 0.0, scale: 1.0 }, &HashMap::new(), Some((&pr, dst.w)), &full, DPI).err();
    check!(e.as_ref().is_some_and(|e| e.drawn == rect_intersect(first_box, whole(&dst))));
    check!(c.px != render(&dst).px); // (drawn: what the caller must take back)
}

// ---- 8. placing ----

fn place_and_append(dst: &mut Page, g: &Group, spot: Spot) -> (Canvas, Placed) {
    let mut c = render(dst);
    let pr = paper_of(dst);
    let mut busy = [false; FLOOR_LAYERS];
    layers_in_use(dst, &mut busy);
    let placed = lasso_place(&mut c, g, spot, &HashMap::new(), Some((&pr, dst.w)), &busy, DPI).unwrap();
    for s in &placed.strokes {
        page_add(dst, Box::new((**s).clone()));
    }
    (c, placed)
}

fn placing_equals_drawing_whole() {
    for k in 0..12u32 {
        SEED.store(0x9100 + k, Ordering::Relaxed);
        let paper = [PAPER_BLANK, PAPER_NARROW, PAPER_DOTS, PAPER_GRID][k as usize % 4];
        // the source: a word partly erased with the precise eraser and white pencil, and a neighbour
        let mut src = page_new(500, 500, paper);
        page_add(&mut src, ink(&[(100.0, 100.0), (140.0, 60.0), (180.0, 100.0), (220.0, 60.0)], 1));
        page_add(&mut src, poly(TOOL_PENCIL, &[(100.0, 120.0), (230.0, 120.0)], 0.8, 2));
        page_add(&mut src, ink(&[(260.0, 100.0), (300.0, 60.0)], 3));
        page_add(&mut src, eraser(&[(160.0, 50.0), (160.0, 130.0)], 1.5));
        page_add(&mut src, poly(TOOL_ERASER, &[(200.0, 40.0), (280.0, 110.0)], 1.0, erase_seed(TOOL_PENCIL, 5)));
        let g = lasso_lift(&src, &[0, 1], Vec::new(), DPI);
        check!(g.carried == 2);
        // into space: plain floors, no marker, exact in every version
        let mut dst = page_new(500, 500, paper);
        page_add(&mut dst, ink(&[(10.0, 400.0), (80.0, 450.0)], 9));
        let (c, placed) = place_and_append(&mut dst, &g, Spot { dx: 100.0, dy: 250.0, scale: 1.0 });
        check!(placed.scoped == 0 && placed.plain == 1 && !page_layered(&dst));
        check!(c.px == render(&dst).px);
        // over ink: scoped, and the destination ink is never lightened
        let mut over = page_new(500, 500, paper);
        page_add(&mut over, poly(TOOL_MARKER, &[(60.0, 80.0), (320.0, 80.0)], 3.0, 4));
        page_add(&mut over, poly(TOOL_MARKER, &[(160.0, 30.0), (160.0, 150.0)], 3.0, 5));
        let before = render(&over);
        let n0 = over.strokes.len();
        let (c, placed) = place_and_append(&mut over, &g, Spot { dx: 0.0, dy: 0.0, scale: 1.0 });
        check!(placed.scoped == 1 && page_layered(&over));
        let full = render(&over);
        check!(c.px == full.px);
        // never lighter than the page was, anywhere
        check!(full.px.iter().zip(&before.px).all(|(a, b)| a <= b));
        // tiles of the new strokes drawn again equal the whole drawing
        let mut m = tile_mask_new(over.w, over.h).unwrap();
        for s in &over.strokes[n0..] {
            stroke_mark_tiles(&mut m, s, DPI);
        }
        tile_mask_seal(&mut m);
        let mut t = before.clone();
        page_render_tiles(&mut t, &over, whole(&over), &Rc::new(m), DPI);
        check!(t.px == full.px);
        // page_render_from equals it too
        let mut f = before.clone();
        let pr = paper_of(&over);
        check!(page_render_from(&mut f, &over, whole(&over), DPI, n0, Some((&pr, over.w))));
        check!(f.px == full.px);
    }
}

// The move oracle: Blank paper, an offset of a multiple of 1024 px (the tooth's tile is 512), points on
// 1/16 px so translation is exact. The destination over empty paper equals the group drawn alone,
// translated; the source spot equals the page without the selection; where the group draws nothing,
// pixels are unchanged.
fn move_oracle() {
    let q = |x: f32| (x * 16.0).round() / 16.0;
    let mut p = page_new(1400, 600, PAPER_BLANK);
    page_add(&mut p, poly(TOOL_PENCIL, &[(q(60.3), q(100.1)), (q(140.7), q(160.2)), (q(220.5), q(90.9))], 0.9, 1));
    page_add(&mut p, ink(&[(q(80.0), q(200.0)), (q(240.0), q(210.0))], 2));
    page_add(&mut p, ink(&[(q(300.0), q(100.0)), (q(330.0), q(220.0))], 3)); // not selected
    page_add(&mut p, eraser(&[(q(150.0), q(80.0)), (q(150.0), q(230.0))], 1.5));
    page_add(&mut p, poly(TOOL_ERASER, &[(q(100.0), q(150.0)), (q(320.0), q(150.0))], 1.2, erase_seed(TOOL_PENCIL, 8)));
    let sel = [0usize, 1];
    let g = lasso_lift(&p, &sel, Vec::new(), DPI);
    // the group alone over blank paper, untranslated
    let mut alone = page_new(p.w, p.h, PAPER_BLANK);
    for s in &g.strokes {
        page_add(&mut alone, Box::new((**s).clone()));
    }
    let alone_c = render(&alone);
    // the move: take the selection off, draw its spot again, place at +1024
    let mut moved = page_new(p.w, p.h, PAPER_BLANK);
    for (i, s) in p.strokes.iter().enumerate() {
        if !sel.contains(&i) {
            page_add(&mut moved, Box::new((**s).clone()));
        }
    }
    let without = render(&moved);
    let (c, placed) = place_and_append(&mut moved, &g, Spot { dx: 1024.0, dy: 0.0, scale: 1.0 });
    check!(placed.plain == 1 && placed.scoped == 0);
    let after = render(&moved);
    check!(c.px == after.px);
    let mut same_dst = true;
    let mut same_src = true;
    let mut untouched = true;
    for y in 0..p.h {
        for x in 0..376 {
            let a = (y * p.w + x) as usize;
            let b = (y * p.w + x + 1024) as usize;
            same_dst &= after.px[b] == alone_c.px[a];
            same_src &= after.px[a] == without.px[a];
            if alone_c.px[a] == 255 {
                untouched &= after.px[b] == without.px[b];
            }
        }
    }
    // (dab positions between points are computed in f32: past a power of two a rare pixel may differ by
    // one gray level, LASSO-DESIGN 2; counted, never more than one level)
    let (mut n, mut maxd) = (0, 0);
    for y in 0..p.h {
        for x in 0..376 {
            let a = (y * p.w + x) as usize;
            let b = (y * p.w + x + 1024) as usize;
            if after.px[b] != alone_c.px[a] {
                n += 1;
                maxd = maxd.max((after.px[b] as i32 - alone_c.px[a] as i32).abs());
            }
        }
    }
    eprintln!("  move oracle: {n} pixels one gray level off (max {maxd})");
    check!(same_dst || (maxd <= 1 && n <= 16));
    check!(same_src);
    check!(untouched);
}

// ---- 9. the clipboard ----

fn clipboard_round_trip_and_refusals() {
    let mut p = page_new(600, 400, PAPER_NARROW);
    p.nb.set("NB1");
    page_add(&mut p, ink(&[(100.0, 100.0), (200.0, 100.0)], 1));
    page_add(&mut p, eraser(&[(150.0, 90.0), (150.0, 110.0)], 2.0));
    let mut l = link(4, 0x77, "p9", Some(Rect { x: 90, y: 80, w: 130, h: 40 }));
    l.outline = LinkOutline::from_points(&[(90.0, 80.0), (220.0, 80.0), (220.0, 120.0)], true, 600, 400);
    let mut g = lasso_lift(&p, &[0], vec![l], DPI);
    g.src_pid = 4;
    let bytes = clipboard_bytes(&g).unwrap();
    let back = clipboard_parse(&bytes).unwrap();
    check!(back.w == 600 && back.h == 400 && back.src_nb == "NB1" && back.src_pid == 4 && back.box_ == g.box_);
    check!(back.strokes.len() == g.strokes.len());
    check!(back.strokes.iter().zip(&g.strokes).all(|(a, b)| a.tool == b.tool && a.seed == b.seed && a.ext == b.ext && a.pts.len() == b.pts.len()));
    check!(back.links.len() == 1 && back.links[0].seed == 0x77 && back.links[0].target == "p9" && back.links[0].area == l.area && back.links[0].outline.n == l.outline.n);
    // cut short, a bad chunk, a bad format byte, trailing bytes, no chunk: refused
    check!(clipboard_parse(&bytes[..bytes.len() - 3]).is_none());
    let mut bad = bytes.clone();
    let at = bytes.windows(4).rposition(|w| w == b"CLIP").unwrap();
    bad[at] = b'X';
    check!(clipboard_parse(&bad).is_none());
    let mut bad = bytes.clone();
    bad[at + 8] = 2;
    check!(clipboard_parse(&bad).is_none());
    let mut bad = bytes.clone();
    bad.push(0);
    check!(clipboard_parse(&bad).is_none());
    check!(clipboard_parse(&bytes[..at]).is_none());
    check!(clipboard_parse(b"QPG1").is_none());
    // the file: written atomically, read back (by this session: page turns and notebook switches read the
    // same file; a start of Grimoire deletes it, below)
    let root = format!("{}/lasso-clip", tl_tmp());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    check!(!clipboard_has() && clipboard_read().is_none());
    check!(clipboard_write(&g) == 0);
    check!(clipboard_has());
    check!(clipboard_read().is_some_and(|b| b.strokes.len() == g.strokes.len()));
    check!(!std::path::Path::new(&format!("{root}/clipboard.qpg.tmp")).exists());
    std::fs::write(clipboard_path(), &bytes[..40]).unwrap();
    check!(clipboard_read().is_none());
    // a Cut that could not take its selection off puts the clipboard back as it was: bytes, or none
    check!(clipboard_write(&g) == 0);
    let before = clipboard_snapshot();
    check!(before.as_deref() == Some(&bytes[..]));
    let mut other = g.clone();
    other.src_pid = 9;
    check!(clipboard_write(&other) == 0);
    check!(clipboard_read().is_some_and(|b| b.src_pid == 9));
    check!(clipboard_restore(before.as_deref()) == 0);
    check!(clipboard_read().is_some_and(|b| b.src_pid == 4));
    check!(clipboard_restore(None) == 0 && !clipboard_has());
    check!(clipboard_snapshot().is_none() && clipboard_restore(None) == 0);
    // The clipboard lasts one session (the owner, 2026-11-04): a start of Grimoire (app_init) deletes the one an
    // earlier session left, saying so in one log line; a start with none says nothing.
    check!(clipboard_write(&g) == 0 && clipboard_has());
    log_capture();
    clipboard_clear_at_start();
    let log = log_captured().unwrap_or_default();
    check!(!clipboard_has() && clipboard_read().is_none());
    check!(log.lines().filter(|l| l.contains("clipboard:")).count() == 1);
    check!(log.contains("the lasso's clipboard from the last session is deleted"));
    log_capture();
    clipboard_clear_at_start();
    check!(log_captured().is_some_and(|l| !l.contains("clipboard:")));
    check!(!clipboard_has());
    // Read quietly: its CLIP chunk, critical on purpose, logs no "read-only page" line as a paste reads it,
    // while a notebook's page with a chunk this version does not know still does.
    check!(clipboard_write(&g) == 0);
    log_capture();
    check!(clipboard_read().is_some());
    check!(log_captured().is_some_and(|l| !l.contains("read-only page")));
    log_capture();
    let mut as_page = None;
    check!(page_deserialize(&bytes, &mut as_page) == 0 && as_page.as_ref().is_some_and(|p| p.readonly));
    check!(log_captured().is_some_and(|l| l.contains("store: read-only page: a chunk this version does not know (CLIP)")));
    check!(clipboard_restore(None) == 0);
    // its own page's originals: there (even after the file's round trip), then gone (a Cut) or moved
    check!(group_originals_on(&p, &g) && group_originals_on(&p, &back));
    let mut moved = page_new(600, 400, PAPER_NARROW);
    let mut m = ink(&[(100.0, 100.0), (200.0, 100.0)], 1);
    for q in m.pts.iter_mut() {
        q.y += 50.0;
    }
    page_add(&mut moved, m);
    page_add(&mut moved, ink(&[(100.0, 100.0), (200.0, 100.0)], 2)); // (another seed)
    check!(!group_originals_on(&moved, &g));
    check!(!group_originals_on(&page_new(600, 400, PAPER_NARROW), &g));
}

// ---- 10. undo and redo ----

fn ids(p: &Page) -> Vec<StrokeId> {
    p.strokes.iter().map(|s| stroke_id(s)).collect()
}

fn lasso_undo_redo() {
    let mut p = page_new(500, 500, PAPER_BLANK);
    for k in 0..6 {
        page_add(&mut p, ink(&[(20.0 + 50.0 * k as f32, 100.0), (60.0 + 50.0 * k as f32, 120.0)], k + 1));
    }
    let start = ids(&p);
    let mut h = History::default();
    history_init(&mut h);
    // a move of strokes 1 and 3: taken off, two new ones appended
    let g = lasso_lift(&p, &[1, 3], Vec::new(), DPI);
    let removed = vec![page_remove(&mut p, 3), page_remove(&mut p, 1)];
    let removed: Vec<Box<Stroke>> = removed.into_iter().rev().collect();
    let mut c = render(&p);
    let placed = lasso_place(&mut c, &g, Spot { dx: 0.0, dy: 200.0, scale: 1.0 }, &HashMap::new(), None, &[false; FLOOR_LAYERS], DPI).unwrap();
    let n_added = placed.strokes.len();
    for s in placed.strokes {
        page_add(&mut p, s);
    }
    let done = ids(&p);
    history_push_lasso(&mut h, &p, &[1, 3], removed, n_added, None);
    check!(history_undo_kind(&h) == Some(ActKind::Lasso));
    let mut d = RECT_EMPTY;
    check!(history_undo(&mut h, &mut p, &mut d));
    check!(ids(&p) == start);
    check!(d.h > 0 && d.y <= 100 && d.y + d.h >= 320);
    check!(history_redo(&mut h, &mut p, &mut d));
    check!(ids(&p) == done);
    check!(history_undo(&mut h, &mut p, &mut d) && ids(&p) == start);
    // a paste: nothing taken off
    let mut c = render(&p);
    let placed = lasso_place(&mut c, &g, Spot { dx: 300.0, dy: 300.0, scale: 1.0 }, &HashMap::new(), None, &[false; FLOOR_LAYERS], DPI).unwrap();
    let n = placed.strokes.len();
    for s in placed.strokes {
        page_add(&mut p, s);
    }
    history_push_lasso(&mut h, &p, &[], Vec::new(), n, None);
    check!(!history_can_redo(&h)); // (a push clears redo)
    check!(history_undo(&mut h, &mut p, &mut d) && ids(&p) == start);
    // a cut: nothing added
    let taken = vec![page_remove(&mut p, 0)];
    history_push_lasso(&mut h, &p, &[0], taken, 0, None);
    check!(history_undo(&mut h, &mut p, &mut d) && ids(&p) == start);
    check!(history_redo(&mut h, &mut p, &mut d) && p.strokes.len() == 5);
    // nothing at all: no step
    let pushes = h.pushes;
    history_push_lasso(&mut h, &p, &[], Vec::new(), 0, None);
    check!(h.pushes == pushes);
    // a scoped paste (its floor marker sits off the page): undo and redo dirty only the ink's place, never
    // a rect from the marker's corner (a full refresh and a whole-page redraw on the Kindle)
    let ink_box = {
        let a = ink(&[(200.0, 300.0), (260.0, 330.0)], 40);
        let b = a.bbox;
        page_add(&mut p, floor_marker_new(7));
        page_add(&mut p, a);
        b
    };
    history_push_lasso(&mut h, &p, &[], Vec::new(), 2, None);
    let mut d = RECT_EMPTY;
    check!(history_undo(&mut h, &mut p, &mut d));
    check!(d == ink_box);
    check!(history_redo(&mut h, &mut p, &mut d));
    check!(d == ink_box);
    // a move of that group: its marker taken off and put back too
    let n = p.strokes.len() as i32;
    let taken: Vec<Box<Stroke>> = vec![page_remove(&mut p, n - 2), page_remove(&mut p, n - 2)];
    history_push_lasso(&mut h, &p, &[n - 2, n - 1], taken, 0, None);
    check!(history_undo(&mut h, &mut p, &mut d) && d == ink_box);
    check!(history_redo(&mut h, &mut p, &mut d) && d == ink_box);
}

// ---- 11. a 1,000-stroke page ----

fn wander(w: i32, h: i32, t: ToolKind) -> Box<Stroke> {
    let erase = t == TOOL_ERASER;
    let texture = if erase { ToolKind((rnd() % TOOL_COUNT as u32) as i32) } else { t };
    let mut prm = ToolParams::default();
    tool_params_default(texture, &mut prm);
    let seed = if erase { erase_seed(texture, rnd()) } else { rnd() & 0xFFFFFF };
    let mut s = stroke_new(t, &prm, seed);
    let (mut x, mut y) = (frand(0.0, w as f32), frand(0.0, h as f32));
    let mut ang = frand(0.0, 6.28);
    let n = 10 + rnd() % 120;
    for k in 0..n {
        stroke_append(&mut s, &PenSample { x, y, pressure: frand(0.3, 1.0), t_ms: k * 6, ..Default::default() }, DPI);
        ang += frand(-0.4, 0.4);
        x += ang.cos() * 4.0;
        y += ang.sin() * 4.0;
    }
    s
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn dense_page_timing() {
    SEED.store(0xD0D0, Ordering::Relaxed);
    let (w, h) = (1860, 2480);
    let mut p = page_new(w, h, PAPER_NARROW);
    for _ in 0..1000 {
        let t = if rnd() % 10 == 0 { TOOL_ERASER } else { [TOOL_BALLPOINT, TOOL_PENCIL, TOOL_FINELINER, TOOL_MARKER][(rnd() % 4) as usize] };
        page_add(&mut p, wander(w, h, t));
    }
    let npts: usize = p.strokes.iter().map(|s| s.pts.len()).sum();
    let t0 = Instant::now();
    let mut c = render(&p);
    let full = ms(t0);
    // a word-sized lasso in the middle
    let l = rect_lasso(800.0, 1100.0, 1100.0, 1300.0);
    let t0 = Instant::now();
    let sel = lasso_select(&p, &[], &l, DPI);
    let select_ms = ms(t0);
    let t0 = Instant::now();
    let g = lasso_lift(&p, &sel.strokes, Vec::new(), DPI);
    let lift_ms = ms(t0);
    // the move: take off, draw the old place again in the tiles they drew into, place at the end
    let t0 = Instant::now();
    let mut m = tile_mask_new(w, h).unwrap();
    let mut removed = Vec::new();
    for &i in sel.strokes.iter().rev() {
        let s = page_remove(&mut p, i as i32);
        stroke_mark_tiles(&mut m, &s, DPI);
        removed.push(s);
    }
    tile_mask_seal(&mut m);
    let src = tile_mask_bounds(&m);
    page_render_tiles(&mut c, &p, src, &Rc::new(m), DPI);
    let old_place = ms(t0);
    let pr = paper_of(&p);
    let mut busy = [false; FLOOR_LAYERS];
    layers_in_use(&p, &mut busy);
    let t0 = Instant::now();
    let placed = lasso_place(&mut c, &g, Spot { dx: 200.0, dy: 300.0, scale: 1.0 }, &HashMap::new(), Some((&pr, w)), &busy, DPI).unwrap();
    let place_ms = ms(t0);
    let n0 = p.strokes.len();
    for s in &placed.strokes {
        page_add(&mut p, Box::new((**s).clone()));
    }
    let full_after = render(&p);
    check!(c.px == full_after.px);
    // a paste of the same group elsewhere (the canvas as it is)
    let mut busy = [false; FLOOR_LAYERS];
    layers_in_use(&p, &mut busy);
    let t0 = Instant::now();
    let pasted = lasso_place(&mut c, &g, Spot { dx: -400.0, dy: 600.0, scale: 1.0 }, &HashMap::new(), Some((&pr, w)), &busy, DPI).unwrap();
    let paste_ms = ms(t0);
    for s in &pasted.strokes {
        page_add(&mut p, Box::new((**s).clone()));
    }
    check!(c.px == render(&p).px);
    let _ = n0;
    eprintln!(
        "  1,000-stroke page ({npts} points): full draw {full:.1} ms; lasso select {} strokes {select_ms:.1} ms; lift (carried {}) {lift_ms:.1} ms; move: old place {old_place:.1} ms + place {place_ms:.1} ms (scoped {} plain {}); paste {paste_ms:.1} ms (scoped {} plain {})",
        sel.strokes.len(), g.carried, placed.scoped, placed.plain, pasted.scoped, pasted.plain
    );
    check!(!sel.strokes.is_empty());
}

#[test]
fn test_lasso() {
    run!(polygon_matches_exact_test);
    run!(enclosure_on_handwriting);
    run!(links_come_along);
    run!(erase_strokes_go_with_their_ink);
    run!(seeds_kept_when_free);
    run!(targets_across_notebooks);
    run!(layers_skip_the_page_and_its_history);
    run!(placing_equals_drawing_whole);
    run!(move_oracle);
    run!(clipboard_round_trip_and_refusals);
    run!(lasso_undo_redo);
    run!(dense_page_timing);
    done();
}
