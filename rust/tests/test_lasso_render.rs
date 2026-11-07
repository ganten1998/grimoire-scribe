// Not a port of a C test: floors in the renderer (page.rs render_from, docs/FORMAT.md "Scoped erase
// strokes and floor markers", docs/beta2/LASSO-DESIGN.md 5 and 6.5).
//
// - Frozen-copy identity: a copy of render_layers as it was before floors (kept here, below) and the
//   current one draw random unlayered pages (ink, precise and white erase strokes, every paper) byte for
//   byte the same: whole, tile-masked and the ink layer only. Dead markers (layer 0, as a 1.0.0-beta
//   re-save leaves them) change nothing either. The renderer's fingerprint is unchanged.
// - Forced floor path: the same pages with a marker of layer 1 at index 0 and every erase stroke scoped
//   to it draw the same bytes (the floor at the page start is its paper), so the generalisation itself
//   is proven, not only bypassed.
// - Scoped semantics: over ink, a scoped erase stroke never lightens a pixel past its floor, and lands
//   exactly on it where it covers fully; over blank paper a scoped group draws what an ordinary one does.
// - page_render_from equals a whole drawing; the marker robustness rules (missing, duplicate, after).
#![allow(clippy::needless_range_loop)]
mod testlib;
mod beta_format;
use grimoire::prelude::*;
use std::rc::Rc;
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
fn whole(p: &Page) -> Rect {
    Rect { x: 0, y: 0, w: p.w, h: p.h }
}

fn wander(w: i32, h: i32, t: ToolKind) -> Box<Stroke> {
    let erase = t == TOOL_ERASER;
    let texture = if erase { ToolKind((rnd() % TOOL_COUNT as u32) as i32) } else { t };
    let mut prm = ToolParams::default();
    tool_params_default(texture, &mut prm);
    if erase && texture == TOOL_ERASER {
        prm.size_min_mm = prm.size_max_mm.max(1.0); // (the precise eraser: big enough to cover ink)
    }
    let seed = if erase { erase_seed(texture, rnd()) } else { rnd() & 0xFFFFFF };
    let mut s = stroke_new(t, &prm, seed);
    let (mut x, mut y) = (frand(0.0, w as f32), frand(0.0, h as f32));
    let mut ang = frand(0.0, 6.28);
    let step = frand(2.0, 14.0);
    let n = 2 + rnd() % 40;
    for k in 0..n {
        let q = PenSample { x, y, pressure: frand(0.2, 1.0), tilt_x: frand(-40.0, 40.0), tilt_y: frand(-40.0, 40.0), t_ms: k * 6 };
        stroke_append(&mut s, &q, DPI);
        ang += frand(-0.5, 0.5);
        x = fmla(ang.cos(), step, x);
        y = fmla(ang.sin(), step, y);
    }
    s
}

fn random_page(w: i32, h: i32, paper: PaperKind, n: usize) -> Box<Page> {
    let mut p = page_new(w, h, paper);
    for _ in 0..n {
        let t = if rnd() % 4 == 0 { TOOL_ERASER } else { ToolKind((rnd() % TOOL_COUNT as u32) as i32) };
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
fn copy_page(p: &Page) -> Box<Page> {
    let mut q = page_new(p.w, p.h, p.paper);
    for s in &p.strokes {
        page_add(&mut q, copy_stroke(s));
    }
    q
}

fn canvas(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::default();
    assert!(canvas_init(&mut c, w, h) == 0);
    c
}

// ---- render_layers as it was before floors (2.0.0-beta.1), without the compaction pieces ----
fn frozen(c: &mut Canvas, p: &Page, region: Rect, dpi: f32, layers: i32, mask: Option<&Rc<TileMask>>) {
    let saved = c.clip;
    let r = rect_intersect(region, saved);
    if rect_is_empty(r) {
        return;
    }
    let n = p.strokes.len();
    let meets = |s: &Stroke| mask.is_none_or(|m| tile_mask_hits(m, rect_intersect(s.bbox, r)));
    let mut u = RECT_EMPTY;
    if (layers & PAGE_LAYER_PAPER) != 0 && (layers & PAGE_LAYER_INK) != 0 {
        for i in 0..n {
            if stroke_is_erase(&p.strokes[i]) && meets(&p.strokes[i]) {
                u = rect_union(u, rect_intersect(p.strokes[i].bbox, r));
            }
        }
    }
    let mut copy: Option<Vec<u8>> = None;
    if !rect_is_empty(u) {
        copy = Some(Vec::with_capacity(u.w as usize * u.h as usize));
    }
    if (layers & PAGE_LAYER_PAPER) != 0 {
        match mask {
            None => page_paper_render(c, p, r, dpi),
            Some(m) => {
                let mut y = r.y;
                while y < r.y + r.h {
                    let h = (y / MASK_TILE + 1) * MASK_TILE - y;
                    let h = h.min(r.y + r.h - y);
                    let mut from = r.x;
                    while let Some((x0, x1)) = tile_mask_run(m, y, from, r.x + r.w) {
                        page_paper_render(c, p, Rect { x: x0, y, w: x1 - x0, h }, dpi);
                        from = x1;
                    }
                    y += h;
                }
            }
        }
    }
    if (layers & PAGE_LAYER_INK) == 0 {
        return;
    }
    let copy: Option<Rc<Vec<u8>>> = copy.map(|mut room| {
        for y in 0..u.h {
            let at = ((u.y + y) * c.stride + u.x) as usize;
            room.extend_from_slice(&c.px[at..at + u.w as usize]);
        }
        Rc::new(room)
    });
    let mut b = BrushState::default();
    for i in 0..n {
        let s: &Stroke = &p.strokes[i];
        if (s.tool.0 as u32) >= TOOL_COUNT as u32 || !rect_overlaps(s.bbox, r) || !meets(s) {
            continue;
        }
        canvas_set_clip(c, if copy.is_some() && stroke_is_erase(s) { u } else { r });
        stroke_brush_begin(&mut b, s, dpi, copy.clone(), u.x, u.y, u.w);
        b.mask = mask.cloned();
        for k in 0..s.pts.len() {
            brush_add(&mut b, c, &s.pts[k]);
        }
    }
    c.clip = saved;
}

fn mask_of(p: &Page, which: &[usize]) -> Rc<TileMask> {
    let mut m = tile_mask_new(p.w, p.h).unwrap();
    for &i in which {
        stroke_mark_tiles(&mut m, &p.strokes[i], DPI);
    }
    tile_mask_seal(&mut m);
    Rc::new(m)
}

// The three ways a page is drawn, by the current renderer and by `frozen`: (whole, masked over a
// canvas of something else, ink layer only).
fn draws(p: &Page, mask_from: &Page, which: &[usize], old: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (w, h) = (p.w, p.h);
    let mut a = canvas(w, h);
    if old { frozen(&mut a, p, whole(p), DPI, PAGE_LAYER_ALL, None) } else { page_render(&mut a, p, whole(p), DPI) }
    let m = mask_of(mask_from, which);
    let mut b = canvas(w, h);
    canvas_fill(&mut b, whole(p), 0x55);
    if old { frozen(&mut b, p, whole(p), DPI, PAGE_LAYER_ALL, Some(&m)) } else { page_render_tiles(&mut b, p, whole(p), &m, DPI) }
    let mut c = canvas(w, h);
    if old { frozen(&mut c, p, whole(p), DPI, PAGE_LAYER_INK, None) } else { page_render_layers(&mut c, p, whole(p), DPI, PAGE_LAYER_INK) }
    (a.px, b.px, c.px)
}

const PAPERS: [PaperKind; 5] = [PAPER_BLANK, PAPER_NARROW, PAPER_DOTS, PAPER_MUSIC, PAPER_GRID];

fn unlayered_pages_draw_as_before() {
    let mut same = 0;
    for k in 0..200u32 {
        SEED.store(0xF100 + k, Ordering::Relaxed);
        let paper = PAPERS[k as usize % PAPERS.len()];
        let mut p = random_page(160, 200, paper, 6 + (rnd() % 30) as usize);
        if k % 10 == 9 {
            // dead markers (layer 0: a 1.0.0-beta re-save) anywhere
            let at = rnd() as i32 % (p.strokes.len() as i32 + 1);
            page_insert(&mut p, at, floor_marker_new(0));
            p.strokes[at as usize].ext[1] = 0;
        }
        let which: Vec<usize> = (0..p.strokes.len()).filter(|_| rnd() % 5 == 0).collect();
        let now = draws(&p, &p, &which, false);
        let old = draws(&p, &p, &which, true);
        if now == old {
            same += 1;
        } else {
            eprintln!("  page {k} draws differently");
        }
    }
    check!(same == 200);
}

// The renderer's fingerprint stays what 2.0.0-beta.1's is: no kept drawing or preview is retired by it. (2.0.2
// sets 2.0.0's and 2.0.1's kept drawings aside by their version instead, store.rs RENDER_TAG_VERSION, and knows
// them by this fingerprint: store_retire_page_render.)
fn fingerprint_unchanged() {
    // (recorded from 2.0.0-beta.1's renderer on this processor family: the fingerprint is the hash of a
    // fixed page; floors never run on it, which the identity test above already proves)
    let mut p = page_new(240, 160, PAPER_NARROW);
    for t in 0..TOOL_COUNT as i32 {
        let mut prm = ToolParams::default();
        tool_params_default(ToolKind(t), &mut prm);
        let mut s = stroke_new(ToolKind(t), &prm, 0x5EED_0000 ^ (t as u32).wrapping_mul(2654435761));
        for k in 0..24 {
            let f = k as f32 / 23.0;
            let pt = PenSample { x: 12.0 + 216.0 * f, y: 10.0 + 14.0 * t as f32 + 6.0 * (f * 9.0).sin(),
                pressure: 0.2 + 0.7 * f, tilt_x: 30.0 * f - 15.0, tilt_y: 10.0, t_ms: k * 8 };
            stroke_append(&mut s, &pt, 300.0);
        }
        page_add(&mut p, s);
    }
    let mut c = canvas(p.w, p.h);
    frozen(&mut c, &p, whole(&p), 300.0, PAGE_LAYER_ALL, None);
    let fp = c.px.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    check!(page_render_fingerprint() == fp);
}

// A marker of layer 1 at index 0 and every erase stroke scoped to it: the floor is the paper, so the
// page draws as it does unscoped.
fn forced_floor_path_draws_the_same() {
    let mut same = 0;
    for k in 0..120u32 {
        SEED.store(0xF700 + k, Ordering::Relaxed);
        let paper = PAPERS[k as usize % PAPERS.len()];
        let p = random_page(160, 200, paper, 6 + (rnd() % 30) as usize);
        let mut q = copy_page(&p);
        page_insert(&mut q, 0, floor_marker_new(1));
        for s in q.strokes.iter_mut().skip(1) {
            if stroke_is_erase(s) {
                s.ext[1] = 1;
            }
        }
        check!(page_layered(&q));
        let which: Vec<usize> = (0..p.strokes.len()).filter(|_| rnd() % 5 == 0).collect();
        let which_q: Vec<usize> = which.iter().map(|i| i + 1).collect();
        let a = draws(&p, &p, &which, false);
        let b = draws(&q, &q, &which_q, false);
        if a == b {
            same += 1;
        } else {
            eprintln!("  page {k}: the forced floor path draws differently");
        }
    }
    check!(same == 120);
}

fn line(t: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, size_mm: f32, seed: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    prm.size_min_mm = size_mm;
    prm.size_max_mm = size_mm;
    let mut s = stroke_new(t, &prm, seed);
    for k in 0..=20 {
        let f = k as f32 / 20.0;
        stroke_append(&mut s, &PenSample { x: x0 + (x1 - x0) * f, y: y0 + (y1 - y0) * f, pressure: 0.8, t_ms: k * 8, ..Default::default() }, DPI);
    }
    s
}

// Ink A (the destination's), a marker of layer 5, ink B (the group), then a precise eraser over both
// scoped to 5: every pixel is at least as dark as A alone draws it, and where the eraser covers fully it
// is exactly A alone. Unscoped, the same eraser takes A away too (the difference is real).
fn scoped_erase_stops_at_its_floor() {
    for paper in [PAPER_BLANK, PAPER_NARROW] {
        let mut p = page_new(240, 200, paper);
        page_add(&mut p, line(TOOL_MARKER, 30.0, 100.0, 210.0, 100.0, 3.0, 11));
        let floor_n = p.strokes.len();
        page_add(&mut p, floor_marker_new(5));
        page_add(&mut p, line(TOOL_BALLPOINT, 120.0, 30.0, 120.0, 170.0, 1.5, 12));
        let mut e = line(TOOL_ERASER, 60.0, 100.0, 180.0, 100.0, 4.0, erase_seed(TOOL_ERASER, 7));
        e.ext[1] = 5;
        page_add(&mut p, e);
        let mut floor = page_new(p.w, p.h, paper);
        page_add(&mut floor, copy_stroke(&p.strokes[0]));
        check!(floor_n == 1);
        let mut c = canvas(p.w, p.h);
        page_render(&mut c, &p, whole(&p), DPI);
        let mut f = canvas(p.w, p.h);
        page_render(&mut f, &floor, whole(&floor), DPI);
        let lighter = c.px.iter().zip(&f.px).filter(|(a, b)| a > b).count();
        check!(lighter == 0);
        // the eraser's core, around (120, 100): exactly the floor there
        let mut core_same = true;
        for y in 97..104 {
            for x in 110..131 {
                let at = (y * p.w + x) as usize;
                core_same &= c.px[at] == f.px[at];
            }
        }
        check!(core_same);
        // unscoped: the destination ink under it is taken away
        let mut q = copy_page(&p);
        q.strokes[3].ext[1] = 0;
        let mut d = canvas(p.w, p.h);
        page_render(&mut d, &q, whole(&q), DPI);
        let at = (100 * p.w + 120) as usize;
        check!(d.px[at] > f.px[at]);
    }
}

// Over blank paper a scoped group draws exactly what the unscoped one does.
fn scoped_over_paper_is_plain() {
    SEED.store(0xAB, Ordering::Relaxed);
    for paper in PAPERS {
        let mut p = page_new(200, 200, paper);
        page_add(&mut p, line(TOOL_FINELINER, 20.0, 40.0, 180.0, 160.0, 0.8, 3));
        let mut q = copy_page(&p);
        page_insert(&mut q, 0, floor_marker_new(9));
        let e = line(TOOL_ERASER, 40.0, 100.0, 160.0, 100.0, 2.0, erase_seed(TOOL_PENCIL, 99));
        let mut e2 = copy_stroke(&e);
        e2.ext[1] = 9;
        page_add(&mut p, e);
        page_add(&mut q, e2);
        let mut a = canvas(200, 200);
        let mut b = canvas(200, 200);
        page_render(&mut a, &p, whole(&p), DPI);
        page_render(&mut b, &q, whole(&q), DPI);
        check!(a.px == b.px);
    }
}

// Drawing the appended strokes onto a canvas of the page before them gives the whole drawing (with
// floors of the appended group, and ordinary erase strokes in it restoring toward the paper given).
fn render_from_equals_whole() {
    for k in 0..30u32 {
        SEED.store(0x5150 + k, Ordering::Relaxed);
        let paper = PAPERS[k as usize % PAPERS.len()];
        let base = random_page(200, 240, paper, 12 + (rnd() % 20) as usize);
        let mut p = copy_page(&base);
        let n0 = p.strokes.len();
        let g = 1 + (rnd() % 200) as u8;
        page_add(&mut p, wander(200, 240, TOOL_PENCIL));
        page_add(&mut p, floor_marker_new(g));
        for _ in 0..6 {
            let t = if rnd() % 3 == 0 { TOOL_ERASER } else { ToolKind((rnd() % TOOL_COUNT as u32) as i32) };
            let mut s = wander(200, 240, t);
            if stroke_is_erase(&s) && rnd() % 2 == 0 {
                s.ext[1] = g;
            }
            page_add(&mut p, s);
        }
        let mut full = canvas(p.w, p.h);
        page_render(&mut full, &p, whole(&p), DPI);
        let mut c = canvas(p.w, p.h);
        page_render(&mut c, &base, whole(&base), DPI);
        let mut paper_c = canvas(p.w, p.h);
        page_paper_render(&mut paper_c, &p, whole(&p), DPI);
        let pr = Rc::new(paper_c.px.clone());
        check!(page_render_from(&mut c, &p, whole(&p), DPI, n0, Some((&pr, p.w))));
        check!(c.px == full.px);
        // tile-masked equals whole on a layered page too
        let which: Vec<usize> = (n0..p.strokes.len()).collect();
        let m = mask_of(&p, &which);
        let mut t = canvas(p.w, p.h);
        page_render(&mut t, &base, whole(&base), DPI);
        page_render_tiles(&mut t, &p, whole(&p), &m, DPI);
        check!(t.px == full.px);
    }
    // a floor before `first`: refused, nothing drawn
    let mut p = page_new(100, 100, PAPER_BLANK);
    page_add(&mut p, floor_marker_new(3));
    page_add(&mut p, line(TOOL_BALLPOINT, 10.0, 10.0, 90.0, 90.0, 1.0, 1));
    let mut e = line(TOOL_ERASER, 10.0, 50.0, 90.0, 50.0, 2.0, 0);
    e.ext[1] = 3;
    page_add(&mut p, e);
    let mut c = canvas(100, 100);
    let before = c.px.clone();
    check!(!page_render_from(&mut c, &p, whole(&p), DPI, 2, None));
    check!(c.px == before);
}

// A scoped stroke with no marker of its layer is ordinary; a duplicate marker is ignored (the first wins);
// a scoped stroke before its marker is ordinary; a marker scaled or translated stays off the page.
fn marker_robustness() {
    let ink = || line(TOOL_MARKER, 20.0, 50.0, 180.0, 50.0, 3.0, 5);
    let er = |g: u8| {
        let mut e = line(TOOL_ERASER, 60.0, 50.0, 140.0, 50.0, 4.0, 0);
        e.ext[1] = g;
        e
    };
    let render = |p: &Page| {
        let mut c = canvas(p.w, p.h);
        page_render(&mut c, p, whole(p), DPI);
        c.px
    };
    let mut plain = page_new(200, 100, PAPER_NARROW);
    page_add(&mut plain, ink());
    page_add(&mut plain, er(0));
    let plain_px = render(&plain);
    // missing marker
    let mut a = page_new(200, 100, PAPER_NARROW);
    page_add(&mut a, ink());
    page_add(&mut a, er(4));
    check!(render(&a) == plain_px);
    // scoped before its marker
    let mut b = page_new(200, 100, PAPER_NARROW);
    page_add(&mut b, ink());
    page_add(&mut b, er(4));
    page_add(&mut b, floor_marker_new(4));
    check!(render(&b) == plain_px);
    // duplicate: the first (before the ink) wins, so the eraser takes the ink; a second after the ink
    // would keep it
    let mut c = page_new(200, 100, PAPER_NARROW);
    page_add(&mut c, floor_marker_new(4));
    page_add(&mut c, ink());
    page_add(&mut c, floor_marker_new(4));
    page_add(&mut c, er(4));
    check!(render(&c) == plain_px);
    let mut d = page_new(200, 100, PAPER_NARROW);
    page_add(&mut d, ink());
    page_add(&mut d, floor_marker_new(4));
    page_add(&mut d, er(4));
    check!(render(&d) != plain_px);
    // the marker's shape
    let m = floor_marker_new(7);
    check!(is_floor_marker(&m) && stroke_is_erase(&m) && stroke_layer(&m) == 7 && m.seed == 0);
    check!(m.pts.len() == 1 && m.pts[0].x == FLOOR_MARK_AT && m.pts[0].pressure == 0.0);
    check!(!rect_overlaps(m.bbox, Rect { x: 0, y: 0, w: 1986, h: 2648 }));
}

// A layered page round-trips byte for byte through the reader and writer, opens writable, and keeps every
// marker and layer byte; a page of another size is scaled with its markers left off the page; compaction
// leaves a layered page as it is (its file unchanged, it stays listed).
fn layered_pages_in_the_store() {
    let mut p = page_new(300, 300, PAPER_NARROW);
    page_add(&mut p, line(TOOL_BALLPOINT, 20.0, 20.0, 280.0, 280.0, 1.0, 9));
    page_add(&mut p, floor_marker_new(12));
    let mut e = line(TOOL_ERASER, 20.0, 150.0, 280.0, 150.0, 2.0, erase_seed(TOOL_PENCIL, 4));
    e.ext[1] = 12;
    page_add(&mut p, e);
    let mut a = Vec::new();
    check!(page_serialize(&p, &mut a) == 0);
    let mut back = None;
    check!(page_deserialize(&a, &mut back) == 0);
    let back = back.unwrap();
    check!(!back.readonly && page_layered(&back) && back.strokes[2].ext[1] == 12 && is_floor_marker(&back.strokes[1]));
    let mut b = Vec::new();
    check!(page_serialize(&back, &mut b) == 0);
    check!(a == b);
    // through the released 1.0.0-beta (design 10, row 14): it reads the page, and its re-save keeps every
    // stroke, seed and point (the marker's off-page point included) with the pad bytes zeroed; read back
    // here, the marker is a dead stroke and the scoped stroke an ordinary one: no layer left, nothing lost
    let mut bp = beta_format::BetaPage::default();
    check!(beta_format::beta_page_deserialize(&a, &mut bp) == 0 && bp.strokes.len() == 3);
    let mut r = Vec::new();
    check!(beta_format::beta_resave(&a, &mut r) == 0);
    let mut old = None;
    check!(page_deserialize(&r, &mut old) == 0);
    if let Some(old) = old {
        check!(!old.readonly && !page_layered(&old) && old.strokes.len() == 3);
        check!(old.strokes.iter().all(|s| s.ext == [0, 0, 0]));
        check!(old.strokes.iter().zip(&p.strokes).all(|(x, y)| x.tool == y.tool && x.seed == y.seed && x.pts.len() == y.pts.len()
            && x.pts.iter().zip(&y.pts).all(|(u, v)| u.x == v.x && u.y == v.y)));
        check!(is_floor_marker(&old.strokes[1]) && old.strokes[1].pts[0].x == FLOOR_MARK_AT);
    }
    // the store: scaled to the notebook's size, markers stay off the page
    let root = format!("{}/lasso-store-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    store_set_device(DPI, 1860, 2480);
    let mut nb = NotebookInfo::default();
    check!(store_create("Layers", "", PAPER_NARROW, &mut nb) == 0);
    let mut big = page_new(1986, 2648, PAPER_NARROW);
    for s in &p.strokes {
        page_add(&mut big, copy_stroke(s));
    }
    let dir = format!("{root}/notebooks/{}", nb.id.as_str());
    check!(store_write_page_in(&dir, nb.page_ids[0], &big) == 0);
    let mut out = None;
    check!(store_load_page(&nb, 0, 1860, 2480, &mut out) == 0);
    let got = out.unwrap();
    check!(got.w == 1860 && got.strokes.len() == 3);
    check!(is_floor_marker(&got.strokes[1]) && got.strokes[1].pts[0].x == FLOOR_MARK_AT && got.strokes[1].ext[1] == 12);
    check!(got.strokes[0].pts[0].x != 20.0); // (the ink was scaled)
    // compaction leaves it as it is
    let mut at_size = page_new(1860, 2480, PAPER_NARROW);
    for s in &p.strokes {
        page_add(&mut at_size, copy_stroke(s));
    }
    check!(store_save_page(&nb, 0, &at_size) == 0);
    let file = format!("{dir}/p{}.qpg", nb.page_ids[0]);
    let before = std::fs::read(&file).unwrap();
    check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
    bg_compact_pages(nb.id.as_str(), DPI);
    check!(std::fs::read(&file).unwrap() == before);
    check!(compact_listed(nb.id.as_str()));
}

#[test]
fn test_lasso_render() {
    run!(unlayered_pages_draw_as_before);
    run!(fingerprint_unchanged);
    run!(forced_floor_path_draws_the_same);
    run!(scoped_erase_stops_at_its_floor);
    run!(scoped_over_paper_is_plain);
    run!(render_from_equals_whole);
    run!(marker_robustness);
    run!(layered_pages_in_the_store);
    done();
}
