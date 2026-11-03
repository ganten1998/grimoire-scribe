//! lasso.rs: lasso select, copy, cut, paste and move, the pure part (docs/beta2/LASSO-DESIGN.md):
//! the lasso's polygon, which ink and links it encloses, which erase strokes go with that ink, the
//! group lifted from a page (floors encoded with markers, page.rs), the seeds it gets on its new page,
//! and placing it at the end of a page's stroke order, drawn onto the page canvas as it goes.
//!
//! Nothing here touches the editor, the store or a file: the editor (Select mode) and clipboard.rs
//! call it.

use crate::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// ---- the polygon ----

/// A closed lasso as a crossing table: for each pixel row its box meets, the x of every edge crossing
/// at the row's middle with the winding there after it. Inside = a non-zero winding (forgiving for a
/// loopy lasso: a figure 8's both loops are inside).
pub struct Lasso {
    y0: i32,
    rows: Vec<Vec<(f32, i32)>>,
    /// the points' box, whole px around them
    pub bounds: Rect,
}

/// Rows a lasso's table may have (a page is at most 2648 tall; points off the page are clamped).
pub const LASSO_ROWS_MAX: i32 = 8192;

/// The pixel rows edge a -> b may cross at their middles (first, last, unclamped) and its direction
/// (1 down, -1 up); None for a level edge. Lasso::new and the live preview (lasso_live.rs) both walk an
/// edge with this and edge_cross, so a winding the preview sums edge by edge is the table's, contribution
/// for contribution.
#[inline]
pub fn edge_rows(a: (f32, f32), b: (f32, f32)) -> Option<(i32, i32, i32)> {
    if a.1 == b.1 {
        return None;
    }
    let dir = if b.1 > a.1 { 1 } else { -1 };
    let (lo, hi) = (a.1.min(b.1), a.1.max(b.1));
    Some(((lo - 0.5).ceil() as i32, (hi - 0.5).floor() as i32, dir))
}

/// Where edge a -> b crosses the middle of pixel row r, or None when it does not (half-open in y, so a
/// vertex is crossed once). One f32 expression, the only one: see edge_rows.
#[inline]
pub fn edge_cross(a: (f32, f32), b: (f32, f32), r: i32) -> Option<f32> {
    let yr = r as f32 + 0.5;
    if (a.1 <= yr) == (b.1 <= yr) {
        return None;
    }
    Some(a.0 + (yr - a.1) * (b.0 - a.0) / (b.1 - a.1))
}

impl Lasso {
    /// The table of a lasso (page px), closed from its last point to its first. None for fewer than
    /// three points or no area.
    pub fn new(pts: &[(f32, f32)]) -> Option<Lasso> {
        if pts.len() < 3 || pts.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let ry0 = (y0.floor() as i32).max(-LASSO_ROWS_MAX);
        let ry1 = (y1.ceil() as i32).min(ry0 + LASSO_ROWS_MAX);
        if ry1 <= ry0 || x1 <= x0 {
            return None;
        }
        let mut rows: Vec<Vec<(f32, i32)>> = vec![Vec::new(); (ry1 - ry0) as usize];
        let n = pts.len();
        for k in 0..n {
            let (a, b) = (pts[k], pts[(k + 1) % n]);
            let Some((first, last, dir)) = edge_rows(a, b) else { continue };
            for r in first.max(ry0)..=last.min(ry1 - 1) {
                if let Some(x) = edge_cross(a, b, r) {
                    rows[(r - ry0) as usize].push((x, dir));
                }
            }
        }
        for row in rows.iter_mut() {
            row.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut w = 0;
            for c in row.iter_mut() {
                w += c.1;
                c.1 = w;
            }
        }
        let bounds = Rect { x: x0.floor() as i32, y: ry0, w: x1.ceil() as i32 - x0.floor() as i32, h: ry1 - ry0 };
        Some(Lasso { y0: ry0, rows, bounds })
    }
    /// The winding number at (x, y): one binary search in its row.
    pub fn winding(&self, x: f32, y: f32) -> i32 {
        if !x.is_finite() || !y.is_finite() {
            return 0;
        }
        let r = y.floor() as i64 - self.y0 as i64;
        if r < 0 || r >= self.rows.len() as i64 {
            return 0;
        }
        let row = &self.rows[r as usize];
        let k = row.partition_point(|c| c.0 < x);
        if k == 0 {
            0
        } else {
            row[k - 1].1
        }
    }
    pub fn inside(&self, x: f32, y: f32) -> bool {
        self.winding(x, y) != 0
    }
}

// ---- floors at the source ----

/// The index of the floor erase stroke i restores toward (its layer's marker), or -1 for the page start
/// (an ordinary erase stroke, and ink).
fn floor_index(p: &Page, f: Option<&FloorTable>, i: usize) -> i64 {
    match floor_of(p, f, i) {
        0 => -1,
        g => f.map_or(-1, |t| t.at[g as usize] as i64),
    }
}

/// Whether s is ink (tools 0 to 9 but the eraser): the strokes a lasso selects.
pub fn stroke_is_ink(s: &Stroke) -> bool {
    !stroke_is_erase(s) && (s.tool.0 as u32) < TOOL_COUNT as u32
}

// ---- enclosure: the visible spine ----

/// One sample of a stroke's spine: where, how much spine length it stands for, and the ink's radius.
#[derive(Clone, Copy)]
struct Sample {
    x: f32,
    y: f32,
    len: f32,
    r: f32,
}

pub fn dab_radius(s: &Stroke, q: &PenSample, dpi: f32) -> f32 {
    let (mut r, mut o) = (0.0, 0.0);
    brush_dab(stroke_render_tool(s), &s.params, q, dpi, &mut r, &mut o);
    r
}

/// A stroke's spine: its points (no length), and the middles of pieces at most 1 mm long along each
/// segment (each standing for its piece's length).
fn spine(s: &Stroke, dpi: f32) -> Vec<Sample> {
    let step = mm_to_px(1.0, dpi).max(1.0);
    let mut out = Vec::with_capacity(s.pts.len() * 2);
    for (k, q) in s.pts.iter().enumerate() {
        let r = dab_radius(s, q, dpi);
        out.push(Sample { x: q.x, y: q.y, len: 0.0, r });
        if let Some(b) = s.pts.get(k + 1) {
            let (dx, dy) = (b.x - q.x, b.y - q.y);
            let l = (dx * dx + dy * dy).sqrt();
            if l > 0.0 {
                let n = (l / step).ceil().clamp(1.0, 4096.0) as i32;
                for j in 0..n {
                    let t = (j as f32 + 0.5) / n as f32;
                    out.push(Sample { x: q.x + dx * t, y: q.y + dy * t, len: l / n as f32, r });
                }
            }
        }
    }
    out
}

fn seg_dist2(px: f32, py: f32, a: &PenSample, b: &PenSample) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((px - a.x) * dx + (py - a.y) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let (ex, ey) = (px - (a.x + dx * t), py - (a.y + dy * t));
    ex * ex + ey * ey
}

/// Whether erase stroke e hides sample q of an ink stroke entirely: its hard core (its dab radius less
/// the rim that fades out, less half a pixel) passes within q's ink radius of it. An estimate for
/// selection only, never for drawing.
fn hides(e: &Stroke, q: &Sample, dpi: f32) -> bool {
    if !rect_contains(rect_inflate(e.bbox, 1), q.x as i32, q.y as i32) {
        return false;
    }
    let rim = brush_rim(stroke_render_tool(e));
    let n = e.pts.len();
    for k in 0..n {
        let a = &e.pts[k];
        let b = &e.pts[if k + 1 < n { k + 1 } else { k }];
        let core = dab_radius(e, a, dpi) * (1.0 - rim) - 0.5 - q.r;
        if core <= 0.0 {
            continue;
        }
        if seg_dist2(q.x, q.y, a, b) <= core * core {
            return true;
        }
    }
    false
}

/// The spine length under which a stroke is judged by the middle of what shows (i-dots, accents, short
/// crossbars), mm.
pub const LASSO_SHORT_MM: f32 = 1.5;
/// The share of a stroke's visible spine that must be inside the lasso.
pub const LASSO_SHARE: f32 = 0.5;

/// The samples a stroke is judged by (enclosure, at the lift and in the live preview alike): what shows of
/// it (its samples no later erase stroke in scope covers with its core), or its whole spine when nothing
/// shows. A long one (at least LASSO_SHORT_MM of judged spine) by its samples' lengths, fixed point
/// (1/LEN_ONE px), so "at least half inside" is decided on integers the same way everywhere; a short one by
/// one sample at the middle of its judged samples.
pub struct Judged {
    /// (x, y, length in 1/LEN_ONE px): the samples that carry length (a long stroke), or the one middle
    pub samples: Vec<(f32, f32, u32)>,
    /// the lengths' sum (a long stroke), 1 for a short one
    pub total: u64,
    pub long: bool,
}

/// Fixed-point lengths: px * LEN_ONE.
pub const LEN_ONE: f32 = 64.0;

impl Judged {
    /// Whether a stroke whose samples inside the lasso carry `inside` (Judged's units) is enclosed.
    #[inline]
    pub fn decide(&self, inside: u64) -> bool {
        if self.long {
            inside * 2 >= self.total
        } else {
            inside > 0
        }
    }
}

/// The erase strokes of p that may hide ink (markers aside), ascending.
pub fn page_erasers(p: &Page) -> Vec<usize> {
    (0..p.strokes.len()).filter(|&j| stroke_is_erase(&p.strokes[j]) && !is_floor_marker(&p.strokes[j])).collect()
}

/// Ink stroke i of p as the lasso judges it (Judged); None when it has no spine.
pub fn judged_samples(p: &Page, f: Option<&FloorTable>, i: usize, dpi: f32, erasers: &[usize]) -> Option<Judged> {
    let s = &p.strokes[i];
    let all = spine(s, dpi);
    if all.is_empty() {
        return None;
    }
    // the erase strokes after it that reach it (and whose floor is before it)
    let near: Vec<&Stroke> = erasers
        .iter()
        .filter(|&&j| j > i && rect_overlaps(p.strokes[j].bbox, s.bbox) && floor_index(p, f, j) < i as i64)
        .map(|&j| &*p.strokes[j])
        .collect();
    let vis: Vec<Sample> = if near.is_empty() { all.clone() } else { all.iter().copied().filter(|q| !near.iter().any(|e| hides(e, q, dpi))).collect() };
    let judged = if vis.is_empty() { &all } else { &vis };
    let fx = |len: f32| (len * LEN_ONE).round() as u32;
    let total: u64 = judged.iter().map(|q| fx(q.len) as u64).sum();
    if total >= (mm_to_px(LASSO_SHORT_MM, dpi) * LEN_ONE).ceil() as u64 {
        let samples = judged.iter().filter(|q| fx(q.len) > 0).map(|q| (q.x, q.y, fx(q.len))).collect();
        return Some(Judged { samples, total, long: true });
    }
    let n = judged.len() as f32;
    let (cx, cy) = judged.iter().fold((0.0f32, 0.0f32), |a, q| (a.0 + q.x / n, a.1 + q.y / n));
    Some(Judged { samples: vec![(cx, cy, 1)], total: 1, long: false })
}

/// Whether the lasso encloses ink stroke i of p: at least half its judged spine inside; a short one (under
/// LASSO_SHORT_MM visible) by the middle of its judged samples (judged_samples).
fn encloses(p: &Page, f: Option<&FloorTable>, i: usize, l: &Lasso, dpi: f32, erasers: &[usize]) -> bool {
    let Some(j) = judged_samples(p, f, i, dpi, erasers) else { return false };
    let inside: u64 = j.samples.iter().filter(|q| l.inside(q.0, q.1)).map(|q| q.2 as u64).sum();
    j.decide(inside)
}

pub fn stroke_length(s: &Stroke) -> f32 {
    s.pts.windows(2).map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt()).sum()
}

/// What a lasso selected on a page: ink strokes (ascending indices) and links (indices into the link
/// list given to lasso_select).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    pub strokes: Vec<usize>,
    pub links: Vec<usize>,
}

/// The ink and links lasso l selects on page p, whose links (page pid's) are `links`. A link comes
/// along when half of an 8 x 8 grid over its area (inside its outline, if it is a closed one) is in the
/// lasso, or when every ink stroke whose box's middle is in its area is selected (at least one), or,
/// for a link with no area (its ink's own box), when at least half its ink's length is: then all of its
/// strokes join the selection.
pub fn lasso_select(p: &Page, links: &[NavLink], l: &Lasso, dpi: f32) -> Selection {
    lasso_select_from(p, links, l, lasso_enclosed(p, l, dpi))
}

/// Which ink strokes of p lasso l encloses, before the links' rules (one flag per stroke).
pub fn lasso_enclosed(p: &Page, l: &Lasso, dpi: f32) -> Vec<bool> {
    let floors = page_floors(p);
    let f = floors.as_deref();
    let erasers = page_erasers(p);
    let mut on = vec![false; p.strokes.len()];
    for i in 0..p.strokes.len() {
        let s = &p.strokes[i];
        if stroke_is_ink(s) && !s.pts.is_empty() && rect_overlaps(s.bbox, l.bounds) && encloses(p, f, i, l, dpi, &erasers) {
            on[i] = true;
        }
    }
    on
}

/// lasso_select given the enclosed ink (lasso_enclosed, or the live preview's same answer): the links'
/// rules, then the selection.
pub fn lasso_select_from(p: &Page, links: &[NavLink], l: &Lasso, mut on: Vec<bool>) -> Selection {
    on.resize(p.strokes.len(), false);
    let mut sel_links = Vec::new();
    for (k, link) in links.iter().enumerate() {
        let carried = if link.has_area {
            let a = link.area;
            let pts = link.outline.points(a, a);
            let closed = link.outline.n > 0 && link.outline.closed;
            let (mut n, mut inside) = (0, 0);
            for gy in 0..8 {
                for gx in 0..8 {
                    let x = a.x as f32 + (gx as f32 + 0.5) * a.w as f32 / 8.0;
                    let y = a.y as f32 + (gy as f32 + 0.5) * a.h as f32 / 8.0;
                    if closed && pts.len() >= 3 && !link.outline.hit(a, a, x, y, 0.0) {
                        continue;
                    }
                    n += 1;
                    inside += l.inside(x, y) as i32;
                }
            }
            let by_area = n > 0 && inside * 2 >= n;
            let mut any = false;
            let mut all = true;
            for i in 0..p.strokes.len() {
                let s = &p.strokes[i];
                if stroke_is_ink(s) && rect_contains(a, s.bbox.x + s.bbox.w / 2, s.bbox.y + s.bbox.h / 2) {
                    any = true;
                    all &= on[i];
                }
            }
            by_area || (any && all)
        } else {
            let mine: Vec<usize> = (0..p.strokes.len()).filter(|&i| stroke_is_ink(&p.strokes[i]) && p.strokes[i].seed == link.seed).collect();
            let total: f32 = mine.iter().map(|&i| stroke_length(&p.strokes[i]).max(1.0)).sum();
            let picked: f32 = mine.iter().filter(|&&i| on[i]).map(|&i| stroke_length(&p.strokes[i]).max(1.0)).sum();
            let go = !mine.is_empty() && picked * 2.0 >= total;
            // all or nothing
            for &i in &mine {
                on[i] = go;
            }
            go
        };
        if carried {
            sel_links.push(k);
        }
    }
    Selection { strokes: (0..on.len()).filter(|&i| on[i]).collect(), links: sel_links }
}

/// Whether any ink stroke of p is visible at all under its erase strokes (for Copy and Cut, which leave
/// out of the clipboard selected ink that is entirely erased: erased content should not travel). By the
/// same estimate as the selection: conservative, a stroke counts as erased only when every sample of its
/// spine is under some erase stroke's hard core.
pub fn stroke_visible(p: &Page, i: usize, dpi: f32) -> bool {
    let floors = page_floors(p);
    let f = floors.as_deref();
    let s = &p.strokes[i];
    let near: Vec<&Stroke> = (i + 1..p.strokes.len())
        .filter(|&j| stroke_is_erase(&p.strokes[j]) && !is_floor_marker(&p.strokes[j]) && rect_overlaps(p.strokes[j].bbox, s.bbox) && floor_index(p, f, j) < i as i64)
        .map(|j| &*p.strokes[j])
        .collect();
    near.is_empty() || spine(s, dpi).iter().any(|q| !near.iter().any(|e| hides(e, q, dpi)))
}

// ---- which erase strokes go with the ink ----

/// The tiles (MASK_TILE px, numbered row by row on a w x h page) a dry run of s's brush draws into.
pub fn stroke_tiles(s: &Stroke, dpi: f32, w: i32, h: i32) -> Vec<u32> {
    let page = Rect { x: 0, y: 0, w, h };
    if (s.tool.0 as u32) >= TOOL_COUNT as u32 || w <= 0 || h <= 0 || !rect_overlaps(s.bbox, page) {
        return Vec::new();
    }
    let mut none = Canvas { px: Vec::new(), w, h, stride: w, clip: page };
    let mut b = BrushState::default();
    stroke_brush_begin(&mut b, s, dpi, None, 0, 0, 0);
    b.dry = true;
    let cols = (w + MASK_TILE - 1) / MASK_TILE;
    let mut v: Vec<u32> = Vec::new();
    for q in &s.pts {
        let r = rect_intersect(brush_add(&mut b, &mut none, q), page);
        if rect_is_empty(r) {
            continue;
        }
        for ty in r.y / MASK_TILE..=(r.y + r.h - 1) / MASK_TILE {
            for tx in r.x / MASK_TILE..=(r.x + r.w - 1) / MASK_TILE {
                v.push((ty * cols + tx) as u32);
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

fn meet(a: &[u32], b: &[u32]) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

/// The erase strokes that go with selected ink `sel` (ascending): every one (markers aside) that came
/// after some selected stroke whose index is after its floor, whose box meets that stroke's, and whose
/// dabs' tiles meet its tiles. Carrying one too many is harmless: over no group ink it restores pixels
/// already at its floor.
pub fn lasso_carried(p: &Page, sel: &[usize], dpi: f32) -> Vec<usize> {
    let Some(&first) = sel.first() else { return Vec::new() };
    let floors = page_floors(p);
    let f = floors.as_deref();
    let mut tiles: HashMap<usize, Vec<u32>> = HashMap::new();
    let mut out = Vec::new();
    for e in first + 1..p.strokes.len() {
        let es = &p.strokes[e];
        if !stroke_is_erase(es) || is_floor_marker(es) {
            continue;
        }
        let fe = floor_index(p, f, e);
        let mut et: Option<Vec<u32>> = None;
        let mut go = false;
        for &s in sel {
            if s >= e {
                break;
            }
            if (s as i64) <= fe || !rect_overlaps(es.bbox, p.strokes[s].bbox) {
                continue;
            }
            let st = tiles.entry(s).or_insert_with(|| stroke_tiles(&p.strokes[s], dpi, p.w, p.h));
            let et = et.get_or_insert_with(|| stroke_tiles(es, dpi, p.w, p.h));
            if meet(st, et) {
                go = true;
                break;
            }
        }
        if go {
            out.push(e);
        }
    }
    out
}

// ---- the group ----

/// A group lifted from a page: its strokes in their page order, at the page's coordinates, with the
/// erase strokes that go with them scoped to floors exactly as on a page (a marker before each floor's
/// place in the group, its layer on its erase strokes: group-local numbers from 1), its carried links,
/// and its box (ink and link areas).
#[derive(Clone, Debug, Default)]
pub struct Group {
    /// the source page's size
    pub w: i32,
    pub h: i32,
    pub strokes: Vec<Box<Stroke>>,
    /// carried links, at the source's coordinates, with their own seeds (pid: the source page)
    pub links: Vec<NavLink>,
    /// the box of its ink and link areas, on the source page
    pub box_: Rect,
    /// where it came from: the notebook's id and the page id (for cross-notebook targets and the
    /// offset of a paste on its own page)
    pub src_nb: String,
    pub src_pid: u32,
    /// how many of its strokes are ink, and erase strokes carried with it
    pub ink: usize,
    pub carried: usize,
}

fn copy_stroke(s: &Stroke) -> Box<Stroke> {
    Box::new(s.clone())
}

/// Lifts selected ink `sel` (ascending) of p with the erase strokes that go with it (lasso_carried), and
/// the links given (already chosen by lasso_select, at p's coordinates). Each distinct floor of the
/// carried erase strokes (the page start for ordinary ones) is one floor of the group, placed before the
/// first group stroke that came after it; so each carried erase stroke takes away, in the group, exactly
/// the group's ink it took away on the page.
pub fn lasso_lift(p: &Page, sel: &[usize], links: Vec<NavLink>, dpi: f32) -> Group {
    let carried = lasso_carried(p, sel, dpi);
    let floors = page_floors(p);
    let f = floors.as_deref();
    let mut order: Vec<usize> = sel.iter().chain(carried.iter()).copied().collect();
    order.sort_unstable();
    order.dedup();
    // the floors (source indices, -1 the page start), in order, each with its group-local layer
    let mut fl: Vec<i64> = carried.iter().map(|&e| floor_index(p, f, e)).collect();
    fl.sort_unstable();
    fl.dedup();
    let layer_of = |fe: i64| -> u8 { (fl.iter().position(|&x| x == fe).unwrap_or(0) + 1).min(255) as u8 };
    let mut strokes: Vec<Box<Stroke>> = Vec::with_capacity(order.len() + fl.len());
    let mut next_floor = 0;
    for &i in &order {
        while next_floor < fl.len() && fl[next_floor] < i as i64 {
            strokes.push(floor_marker_new(layer_of(fl[next_floor])));
            next_floor += 1;
        }
        let mut c = copy_stroke(&p.strokes[i]);
        if stroke_is_erase(&c) {
            c.ext[1] = layer_of(floor_index(p, f, i));
        } else {
            c.ext[1] = 0;
        }
        strokes.push(c);
    }
    let page = Rect { x: 0, y: 0, w: p.w, h: p.h };
    let mut box_ = RECT_EMPTY;
    for &i in sel {
        box_ = rect_union(box_, p.strokes[i].bbox);
    }
    for l in &links {
        if l.has_area {
            box_ = rect_union(box_, l.area);
        }
    }
    Group {
        w: p.w,
        h: p.h,
        strokes,
        links,
        box_: rect_intersect(box_, page),
        src_nb: p.nb.as_str().to_string(),
        src_pid: 0,
        ink: sel.len(),
        carried: carried.len(),
    }
}

// ---- seeds ----

/// The next seed after `want` that `taken` does not hold, by nav_new_seed's step.
pub fn fresh_seed(taken: &HashSet<u32>, mut want: u32) -> u32 {
    for _ in 0..4096 {
        want = want.wrapping_mul(2654435761).wrapping_add(0x9e3779b9);
        if !taken.contains(&want) {
            break;
        }
    }
    want
}

/// The seeds a group's ink and links take on page dst, whose links are dst_links: (ink seed map, link
/// seed map), each from the group's seed to the new one.
///
/// - Links: a move on its own page keeps each link's seed (`keep_links`); a paste gives each a fresh one,
///   unique on the page and in the paste.
/// - Ink: a stroke with a carried link's seed takes that link's new seed (a 2.0.0-beta.5 ink link keeps
///   finding its box); one with the seed of a link on the source that does not travel (`src_link_seeds`)
///   gets a fresh one (it leaves that link); any other keeps its seed when no ink stroke on dst (but
///   those in `skip`, the originals of a move) and no link there has it, else a fresh one. Strokes that
///   shared a seed keep sharing one. Erase strokes and markers keep their seeds (their texture byte).
pub fn lasso_seeds(g: &Group, dst: &Page, dst_links: &[NavLink], skip: &[usize], src_link_seeds: &HashSet<u32>, keep_links: bool) -> (HashMap<u32, u32>, HashMap<u32, u32>) {
    let mut taken: HashSet<u32> = HashSet::new();
    for (i, s) in dst.strokes.iter().enumerate() {
        if stroke_is_ink(s) && !skip.contains(&i) {
            taken.insert(s.seed);
        }
    }
    for l in dst_links {
        taken.insert(l.seed);
    }
    let mut links = HashMap::new();
    for l in &g.links {
        let new = if keep_links {
            l.seed
        } else {
            let n = fresh_seed(&taken, l.seed);
            taken.insert(n);
            n
        };
        taken.insert(new);
        links.insert(l.seed, new);
    }
    let mut ink = HashMap::new();
    for s in &g.strokes {
        if !stroke_is_ink(s) || ink.contains_key(&s.seed) {
            continue;
        }
        let new = if let Some(&n) = links.get(&s.seed) {
            n
        } else if !src_link_seeds.contains(&s.seed) && !taken.contains(&s.seed) {
            s.seed
        } else {
            fresh_seed(&taken, s.seed)
        };
        taken.insert(new);
        ink.insert(s.seed, new);
    }
    (ink, links)
}

// ---- where a paste goes ----

/// How a group is put down: a point (x, y) goes to (x * scale + dx, y * scale + dy).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
}

/// Where a group with box `b` (on its source page) goes on a page pw x ph seen through `view` (page px):
/// 1:1, scaled down uniformly only if it is larger than the page. Centred on `at` (a long press), else
/// where it was if that spot is in the view and on the page (3 mm right and down when the originals are
/// still there, `offset`), else in the middle of the view; then its box is clamped onto the page. Whole
/// pixels when it is not scaled.
pub fn paste_spot(b: Rect, view: Rect, pw: i32, ph: i32, offset: bool, at: Option<(i32, i32)>, dpi: f32) -> Spot {
    if rect_is_empty(b) || pw <= 0 || ph <= 0 {
        return Spot { dx: 0.0, dy: 0.0, scale: 1.0 };
    }
    let scale = if b.w > pw || b.h > ph { (pw as f32 / b.w as f32).min(ph as f32 / b.h as f32) } else { 1.0 };
    let (w, h) = ((b.w as f32 * scale).ceil() as i32, (b.h as f32 * scale).ceil() as i32);
    let page = Rect { x: 0, y: 0, w: pw, h: ph };
    let (mut x, mut y) = if let Some((cx, cy)) = at {
        (cx - w / 2, cy - h / 2)
    } else if scale == 1.0 && rect_contains_rect(page, b) && rect_contains_rect(view, b) {
        let o = if offset { mm_to_px(3.0, dpi).round() as i32 } else { 0 };
        (b.x + o, b.y + o)
    } else {
        let v = if rect_is_empty(rect_intersect(view, page)) { page } else { rect_intersect(view, page) };
        (v.x + (v.w - w) / 2, v.y + (v.h - h) / 2)
    };
    x = x.clamp(0, (pw - w).max(0));
    y = y.clamp(0, (ph - h).max(0));
    if scale == 1.0 {
        Spot { dx: (x - b.x) as f32, dy: (y - b.y) as f32, scale }
    } else {
        Spot { dx: x as f32 - b.x as f32 * scale, dy: y as f32 - b.y as f32 * scale, scale }
    }
}

/// A carried link put down at `spot` on a pw x ph page, with its new seed and target: its area (and
/// outline) moved and scaled, rounded outward, clamped to the page and to NAV_AREA_MAX. None when what is
/// left is not a valid area.
pub fn link_placed(l: &NavLink, spot: Spot, pid: u32, seed: u32, target: &str, pw: i32, ph: i32) -> Option<NavLink> {
    let mut out = *l;
    out.pid = pid;
    out.seed = seed;
    out.target.set(target);
    if !l.has_area {
        return Some(out);
    }
    let lim_w = pw.min(NAV_AREA_MAX);
    let lim_h = ph.min(NAV_AREA_MAX);
    let mx = |x: f32| x * spot.scale + spot.dx;
    let my = |y: f32| y * spot.scale + spot.dy;
    let a = l.area;
    let x0 = (mx(a.x as f32).floor() as i32).clamp(0, lim_w);
    let y0 = (my(a.y as f32).floor() as i32).clamp(0, lim_h);
    let x1 = (mx((a.x + a.w) as f32).ceil() as i32).clamp(0, lim_w);
    let y1 = (my((a.y + a.h) as f32).ceil() as i32).clamp(0, lim_h);
    out.area = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    if !nav_area_valid(out.area) {
        return None;
    }
    if l.outline.n > 0 {
        let mut o = l.outline;
        for k in 0..o.n as usize {
            let (x, y) = o.xy[k];
            let nx = (mx(x as f32).round() as i32).clamp(out.area.x, out.area.x + out.area.w);
            let ny = (my(y as f32).round() as i32).clamp(out.area.y, out.area.y + out.area.h);
            o.xy[k] = (nx as i16, ny as i16);
        }
        out.outline = if o.within(out.area) { o } else { LinkOutline::default() };
    }
    Some(out)
}

// ---- placing a group ----

/// What lasso_place did.
#[derive(Debug, Default)]
pub struct Placed {
    /// the strokes to append to the page, in order (markers of the floors that were scoped)
    pub strokes: Vec<Box<Stroke>>,
    /// the page rect they drew in
    pub drawn: Rect,
    /// floors of the group that were scoped (needed a layer), and those left plain
    pub scoped: usize,
    pub plain: usize,
}

/// lasso_place could not: the page has no free layer for a floor that needs one. `drawn` is the page rect
/// the group's strokes before that floor were already drawn onto the canvas in: the caller draws it again
/// from the page, or that ink would stay on screen without being on the page.
#[derive(Debug, PartialEq, Eq)]
pub struct NoLayer {
    pub drawn: Rect,
}

/// The layers of 1 to 255 a page uses (on any stroke), joined to `busy` (those its history owns).
pub fn layers_in_use(p: &Page, busy: &mut [bool; FLOOR_LAYERS]) {
    for s in &p.strokes {
        busy[s.ext[1] as usize] = true;
    }
    busy[0] = true;
}

fn translated(s: &Stroke, spot: Spot, dpi: f32) -> Box<Stroke> {
    let mut c = Box::new(Stroke { tool: s.tool, params: s.params, seed: s.seed, ext: s.ext, pts: Vec::new(), bbox: RECT_EMPTY });
    let _ = c.pts.try_reserve_exact(s.pts.len());
    let marker = is_floor_marker(s);
    for q in &s.pts {
        let mut q = *q;
        if !marker {
            if spot.scale != 1.0 {
                q.x *= spot.scale;
                q.y *= spot.scale;
            }
            q.x += spot.dx;
            q.y += spot.dy;
        }
        stroke_append(&mut c, &q, dpi);
    }
    c
}

/// Draws s onto c (a canvas of the page), as the renderer does: ink clipped to the canvas's clip, an
/// erase stroke restoring toward `rest` (its buffer and the rect it covers, which it is clipped to).
fn draw_stroke(c: &mut Canvas, s: &Stroke, dpi: f32, rest: Option<(Rc<Vec<u8>>, Rect, i32)>) {
    let saved = c.clip;
    if (s.tool.0 as u32) >= TOOL_COUNT as u32 || !rect_overlaps(s.bbox, saved) {
        return;
    }
    let mut b = BrushState::default();
    match (&rest, stroke_is_erase(s)) {
        (Some((buf, r, stride)), true) => {
            let clip = rect_intersect(*r, saved);
            if rect_is_empty(clip) {
                return;
            }
            canvas_set_clip(c, clip);
            stroke_brush_begin(&mut b, s, dpi, Some(buf.clone()), r.x, r.y, *stride);
        }
        _ => stroke_brush_begin(&mut b, s, dpi, None, 0, 0, 0),
    }
    for k in 0..s.pts.len() {
        brush_add(&mut b, c, &s.pts[k]);
        if (k & 127) == 127 {
            work_pump();
        }
    }
    work_pump();
    c.clip = saved;
}

/// Puts group g down at the end of page dst, drawn onto c (a canvas of dst as it is now, at page size)
/// as it goes: its points moved by `spot` (markers stay where they are), its ink's seeds by `seeds`
/// (lasso_seeds), and each of its floors decided. A floor whose snapshot is the destination's paper under
/// every tile its erase strokes draw in is plain: its marker is left out and its strokes become ordinary
/// erase strokes (exact in every version since 1.1). Any other is scoped: a free layer (not in `busy`,
/// which layers_in_use fills) for its marker and strokes, so they never lighten the ink already there.
/// `paper` is dst's paper at page size and its row stride; None when it cannot be compared (a picture
/// still missing): every floor is scoped then. Nothing is changed on dst: the caller appends the strokes.
pub fn lasso_place(c: &mut Canvas, g: &Group, spot: Spot, seeds: &HashMap<u32, u32>, paper: Option<(&Rc<Vec<u8>>, i32)>, busy: &[bool; FLOOR_LAYERS], dpi: f32) -> Result<Placed, NoLayer> {
    let (w, h) = (c.w, c.h);
    let page = rect_intersect(Rect { x: 0, y: 0, w, h }, c.clip);
    let mut t: Vec<Box<Stroke>> = g.strokes.iter().map(|s| translated(s, spot, dpi)).collect();
    for s in t.iter_mut() {
        if stroke_is_ink(s) {
            if let Some(&n) = seeds.get(&s.seed) {
                s.seed = n;
            }
        }
    }
    // each local layer's first marker in the group (a later duplicate is ignored), and its strokes' rect
    let mut first_marker = [usize::MAX; FLOOR_LAYERS];
    for (k, s) in t.iter().enumerate() {
        if is_floor_marker(s) && s.ext[1] != 0 && first_marker[s.ext[1] as usize] == usize::MAX {
            first_marker[s.ext[1] as usize] = k;
        }
    }
    let scoped_to = |t: &[Box<Stroke>], k: usize| -> u8 {
        let s = &t[k];
        if !stroke_is_erase(s) || is_floor_marker(s) || s.ext[1] == 0 {
            return 0;
        }
        let m = first_marker[s.ext[1] as usize];
        if m != usize::MAX && m < k {
            s.ext[1]
        } else {
            0
        }
    };
    let mut used = *busy;
    used[0] = true;
    let mut out = Placed::default();
    // per local layer: the page layer it got (0: plain), and its snapshot
    let mut new_layer = [0u8; FLOOR_LAYERS];
    let mut snap: Vec<Option<(Rc<Vec<u8>>, Rect, i32)>> = (0..FLOOR_LAYERS).map(|_| None).collect();
    let paper_rest = paper.map(|(b, st)| (b.clone(), Rect { x: 0, y: 0, w, h }, st));
    let mut keep = vec![true; t.len()];
    for k in 0..t.len() {
        if is_floor_marker(&t[k]) {
            let lg = t[k].ext[1] as usize;
            keep[k] = false;
            if lg == 0 || first_marker[lg] != k {
                continue;
            }
            let mine: Vec<usize> = (k + 1..t.len()).filter(|&j| scoped_to(&t, j) as usize == lg).collect();
            let mut r = RECT_EMPTY;
            for &j in &mine {
                r = rect_union(r, rect_intersect(t[j].bbox, page));
            }
            if mine.is_empty() || rect_is_empty(r) {
                // (nothing of it on the page: its strokes draw nothing; plain)
                out.plain += 1;
                continue;
            }
            let plain = match paper {
                None => false,
                Some((pb, st)) => {
                    let cols = (w + MASK_TILE - 1) / MASK_TILE;
                    let mut tiles: Vec<u32> = mine.iter().flat_map(|&j| stroke_tiles(&t[j], dpi, w, h)).collect();
                    tiles.sort_unstable();
                    tiles.dedup();
                    tiles.iter().all(|&tn| {
                        let tr = rect_intersect(Rect { x: (tn as i32 % cols) * MASK_TILE, y: (tn as i32 / cols) * MASK_TILE, w: MASK_TILE, h: MASK_TILE }, r);
                        (tr.y..tr.y + tr.h).all(|y| {
                            let a = (y * c.stride + tr.x) as usize;
                            let b = (y * st + tr.x) as usize;
                            c.px[a..a + tr.w as usize] == pb[b..b + tr.w as usize]
                        })
                    })
                }
            };
            if plain {
                out.plain += 1;
                continue;
            }
            let Some(gl) = (1..FLOOR_LAYERS).find(|&x| !used[x]) else { return Err(NoLayer { drawn: out.drawn }) };
            used[gl] = true;
            new_layer[lg] = gl as u8;
            keep[k] = true;
            let mut room = Vec::with_capacity((r.w * r.h) as usize);
            for y in r.y..r.y + r.h {
                let a = (y * c.stride + r.x) as usize;
                room.extend_from_slice(&c.px[a..a + r.w as usize]);
            }
            snap[lg] = Some((Rc::new(room), r, r.w));
            out.scoped += 1;
            continue;
        }
        let lg = scoped_to(&t, k) as usize;
        if lg != 0 && new_layer[lg] != 0 {
            draw_stroke(c, &t[k], dpi, snap[lg].clone());
        } else {
            draw_stroke(c, &t[k], dpi, paper_rest.clone());
        }
        out.drawn = rect_union(out.drawn, rect_intersect(t[k].bbox, page));
    }
    for (k, mut s) in t.into_iter().enumerate() {
        if !keep[k] {
            continue;
        }
        if stroke_is_erase(&s) {
            let lg = s.ext[1] as usize;
            s.ext[1] = new_layer[lg]; // (plain floors, and scoped strokes with no marker: 0)
        }
        out.strokes.push(s);
    }
    Ok(out)
}

/// Whether any of group g's ink is still on page p where it was lifted (the same tool, seed and point
/// positions: f32 in the file, so exact after a clipboard round trip, where pressure is rounded):
/// a paste on the page it was copied from offsets itself only then (design 6.7), so after a Cut, or once
/// the originals moved away, it lands where the group was.
pub fn group_originals_on(p: &Page, g: &Group) -> bool {
    let twin = |a: &Stroke, b: &Stroke| a.tool == b.tool && a.seed == b.seed && a.pts.len() == b.pts.len()
        && a.pts.iter().zip(&b.pts).all(|(x, y)| x.x == y.x && x.y == y.y);
    g.strokes.iter().filter(|s| stroke_is_ink(s)).any(|s| p.strokes.iter().any(|q| stroke_is_ink(q) && twin(s, q)))
}

// ---- exact area: a stroke cut at its own dabs (docs/beta2/PIXEL-LASSO.md 1) ----

/// One dab of a stroke's walk: the sample the brush stamped, its radius, and the index of the point
/// whose brush_add stamped it (0: the first point's own dab; k: on the segment from point k - 1 to k).
#[derive(Clone, Copy, Debug)]
pub struct Dab {
    pub m: PenSample,
    pub r: f32,
    pub seg: usize,
    /// the calligraphy nib after it (BrushState.nib_c, nib_s)
    pub nib: (f32, f32),
}

/// The dabs a stroke's brush stamps, in order: a dry run of the very walk that draws it (brush.rs
/// BrushState.dab_log), so a cut made at them cannot drift from what is drawn.
pub fn stroke_dabs(s: &Stroke, dpi: f32) -> Vec<Dab> {
    if (s.tool.0 as u32) >= TOOL_COUNT as u32 || s.pts.is_empty() {
        return Vec::new();
    }
    let big = 1 << 22;
    let mut none = Canvas { px: Vec::new(), w: big, h: big, stride: big, clip: Rect { x: -big, y: -big, w: 2 * big, h: 2 * big } };
    let mut b = BrushState::default();
    stroke_brush_begin(&mut b, s, dpi, None, 0, 0, 0);
    b.dry = true;
    b.dab_log = Some(Vec::new());
    let mut out = Vec::new();
    for (k, q) in s.pts.iter().enumerate() {
        brush_add(&mut b, &mut none, q);
        for (m, r, nc, ns) in b.dab_log.as_mut().unwrap().drain(..) {
            out.push(Dab { m, r, seg: k, nib: (nc, ns) });
        }
    }
    out
}

/// Whether the exact-area lasso may cut s: ink, but not a calligraphy stroke whose tilt widens or lightens
/// it (its nib's smoothing restarts at a cut, and only where the tilt changes nothing but the nib can a
/// piece start it where it was: stroke_cut). Such a stroke is judged whole, as in whole-strokes mode.
pub fn stroke_cuttable(s: &Stroke) -> bool {
    stroke_is_ink(s) && !s.pts.is_empty() && (stroke_render_tool(s) != TOOL_CALLIGRAPHY || (s.params.tilt_size == 0.0 && s.params.tilt_opacity == 0.0))
}

/// What cutting a stroke along a lasso gives.
#[derive(Debug)]
pub enum Cut {
    /// every dab inside (or outside): the stroke whole
    Inside,
    Outside,
    /// the pieces in the stroke's order, each with whether it is inside
    Pieces(Vec<(Box<Stroke>, bool)>),
}

/// A point as the page file stores it (pressure in 1/65535, tilt in 0.01 degree): a piece drawn live,
/// saved and read back is the same bytes.
fn stored(mut q: PenSample) -> PenSample {
    sample_as_stored(&mut q);
    q
}

fn dist(a: &PenSample, b: &PenSample) -> f32 {
    ((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)).sqrt()
}

/// The pieces of stroke s by its dabs' classes (`inside` judges a dab's centre): runs of equal class.
/// A piece after the first starts at its first dab's own sample (so its first dab is the original's,
/// and its spacing from there walks the original's line); a piece before the last ends half a spacing
/// past its last dab (the next dab of the original is a whole spacing on, so no dab lands there). Same
/// tool, parameters, ext and seed as s; new points stored as the page file stores them.
pub fn stroke_cut(s: &Stroke, dpi: f32, inside: &dyn Fn(f32, f32) -> bool) -> Cut {
    let dabs = stroke_dabs(s, dpi);
    if dabs.is_empty() {
        return Cut::Outside;
    }
    let class: Vec<bool> = dabs.iter().map(|d| inside(d.m.x, d.m.y)).collect();
    if class.iter().all(|&c| c) {
        return Cut::Inside;
    }
    if class.iter().all(|&c| !c) {
        return Cut::Outside;
    }
    let n = dabs.len();
    // (only where the tilt changes nothing but the nib: calligraphy's defaults)
    let seed_nib = stroke_render_tool(s) == TOOL_CALLIGRAPHY && !stroke_is_erase(s) && s.params.tilt_size == 0.0 && s.params.tilt_opacity == 0.0;
    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    let mut j0 = 0;
    for j in 1..=n {
        if j == n || class[j] != class[j0] {
            runs.push((j0, j, class[j0]));
            j0 = j;
        }
    }
    let mut out = Vec::with_capacity(runs.len());
    for &(j0, j1, cl) in &runs {
        let mut pts: Vec<PenSample> = Vec::new();
        // the start, and the first point after it
        let mut k = if j0 == 0 {
            pts.push(s.pts[0]);
            1
        } else {
            let m = stored(dabs[j0].m);
            if seed_nib {
                // a broad edge whose tilt changes only its nib: the first point leans so the piece's nib
                // starts where the stroke's smoothed one was, and a second point at the same place
                // (no dab: no length) carries the pen's own tilt on from there
                let (tx, ty) = brush_tilt_for_nib(&s.params, dabs[j0].nib.0, dabs[j0].nib.1);
                pts.push(stored(PenSample { tilt_x: tx, tilt_y: ty, ..m }));
            }
            pts.push(m);
            dabs[j0].seg
        };
        if j1 == n {
            pts.extend_from_slice(&s.pts[k.min(s.pts.len())..]);
        } else {
            // half a spacing on from the last dab, along the stroke's points
            let last = dabs[j1 - 1];
            let mut h = (0.15 * last.r).max(0.5) / 2.0;
            let mut from = last.m;
            // the points between the start and the last dab
            while k < last.seg {
                pts.push(s.pts[k]);
                k += 1;
            }
            k = last.seg.max(1).max(k);
            loop {
                let Some(next) = s.pts.get(k) else {
                    pts.push(stored(from));
                    break;
                };
                let d = dist(&from, next);
                if d > 20000.0 {
                    // (corrupt data the brush does not bridge: the piece ends where its ink did)
                    break;
                }
                if h <= d && d > 0.0 {
                    let t = h / d;
                    let e = PenSample {
                        x: from.x + (next.x - from.x) * t,
                        y: from.y + (next.y - from.y) * t,
                        pressure: from.pressure + (next.pressure - from.pressure) * t,
                        tilt_x: from.tilt_x + (next.tilt_x - from.tilt_x) * t,
                        tilt_y: from.tilt_y + (next.tilt_y - from.tilt_y) * t,
                        t_ms: ((from.t_ms as i64) + (((next.t_ms as i64 - from.t_ms as i64) as f64) * t as f64) as i64) as u32,
                    };
                    pts.push(stored(e));
                    break;
                }
                h -= d;
                pts.push(*next);
                from = *next;
                k += 1;
            }
        }
        let mut c = Box::new(Stroke { tool: s.tool, params: s.params, seed: s.seed, ext: s.ext, pts: Vec::new(), bbox: RECT_EMPTY });
        let _ = c.pts.try_reserve_exact(pts.len());
        for q in &pts {
            stroke_append(&mut c, q, dpi);
        }
        out.push((c, cl));
    }
    Cut::Pieces(out)
}

impl Lasso {
    /// Whether rect r (page px, whole) is wholly inside the lasso (Some(true)), wholly outside
    /// (Some(false)), or crosses its line (None): no crossing strictly within r's x span on any of its
    /// rows, and the winding there non-zero or zero on every row alike.
    pub fn rect_inside(&self, r: Rect) -> Option<bool> {
        if rect_is_empty(r) {
            return Some(false);
        }
        let (x0, x1) = (r.x as f32, (r.x + r.w) as f32);
        let mut seen: Option<bool> = None;
        // (rows and x inclusive of r's far edges: a point on them is judged by the row and span it is in)
        for y in r.y..=r.y + r.h {
            let row = y as i64 - self.y0 as i64;
            let inside = if row < 0 || row >= self.rows.len() as i64 {
                false
            } else {
                let row = &self.rows[row as usize];
                let k = row.partition_point(|c| c.0 < x0);
                // a crossing at x0 itself, or inside the span, changes the winding across r
                if k < row.len() && row[k].0 <= x1 {
                    return None;
                }
                k > 0 && row[k - 1].1 != 0
            };
            match seen {
                None => seen = Some(inside),
                Some(s) if s != inside => return None,
                _ => {}
            }
        }
        seen
    }
}

// ---- exact area: the cut plan ----

/// What an exact-area lasso selects on a page: ink taken whole (ascending indices), the strokes it
/// crosses with their pieces (ascending by index; each piece with whether it is inside), and links
/// (indices into the link list given). Nothing on the page changes until a gesture splices it.
#[derive(Debug, Default)]
pub struct CutPlan {
    pub whole: Vec<usize>,
    pub cut: Vec<(usize, Vec<(Box<Stroke>, bool)>)>,
    pub links: Vec<usize>,
}

impl CutPlan {
    pub fn is_empty(&self) -> bool {
        self.whole.is_empty() && self.cut.is_empty() && self.links.is_empty()
    }
    /// How many strokes it takes (whole ones and inside pieces).
    pub fn taken(&self) -> usize {
        self.whole.len() + self.cut.iter().map(|(_, v)| v.iter().filter(|q| q.1).count()).sum::<usize>()
    }
}

/// The exact-area selection of lasso l on page p (links `links`), given the whole-strokes enclosure
/// `on_whole` (lasso_enclosed, or the live preview's same answer). Ink wholly inside the lasso (its box,
/// or every dab) is taken whole; ink it crosses is cut at its dabs (stroke_cut), but a short stroke (under
/// LASSO_SHORT_MM of what shows: i-dots, accents) is taken whole or not by `on_whole`. Ink of a 2.0.0-beta.5
/// ink link (its seed a link's with no area) is never cut: the shipped all-or-nothing rule decides it, as
/// it decides those links; so is a stroke that cannot be cut faithfully (stroke_cuttable). A link with an
/// area comes along when half of its 8 x 8 grid is in the lasso, or when every ink stroke (or piece)
/// whose box's middle is in its area is taken.
pub fn lasso_cut_plan(p: &Page, links: &[NavLink], l: &Lasso, dpi: f32, on_whole: Vec<bool>) -> CutPlan {
    let ws = lasso_select_from(p, links, l, on_whole);
    let linked: HashSet<u32> = links.iter().filter(|k| !k.has_area).map(|k| k.seed).collect();
    let mut plan = CutPlan::default();
    let in_ws: HashSet<usize> = ws.strokes.iter().copied().collect();
    // (made at the first stroke the lasso's line crosses)
    let mut judging: Option<(Option<Box<FloorTable>>, Vec<usize>)> = None;
    let mut short = |i: usize| {
        let (floors, erasers) = judging.get_or_insert_with(|| (page_floors(p), page_erasers(p)));
        judged_samples(p, floors.as_deref(), i, dpi, erasers).is_some_and(|j| !j.long)
    };
    for i in 0..p.strokes.len() {
        let s = &p.strokes[i];
        if !stroke_is_ink(s) || s.pts.is_empty() {
            continue;
        }
        if linked.contains(&s.seed) || !stroke_cuttable(s) {
            if in_ws.contains(&i) {
                plan.whole.push(i);
            }
            continue;
        }
        if !rect_overlaps(s.bbox, l.bounds) {
            continue;
        }
        match l.rect_inside(s.bbox) {
            Some(true) => plan.whole.push(i),
            Some(false) => {}
            // a short stroke (an i-dot, an accent: under LASSO_SHORT_MM of what shows) is never cut: the
            // whole-strokes rule, by the middle of what shows, decides it, as the live preview shows it
            // (lasso_live.rs halo_list), so the lift takes exactly what the halo showed
            None if short(i) => {
                if in_ws.contains(&i) {
                    plan.whole.push(i);
                }
            }
            None => match stroke_cut(s, dpi, &|x, y| l.inside(x, y)) {
                Cut::Inside => plan.whole.push(i),
                Cut::Outside => {}
                Cut::Pieces(v) => plan.cut.push((i, v)),
            },
        }
    }
    // the links: those with no area as the whole-strokes rule decided; areas on the cut view
    let whole: HashSet<usize> = plan.whole.iter().copied().collect();
    let cut_at: HashMap<usize, usize> = plan.cut.iter().enumerate().map(|(k, c)| (c.0, k)).collect();
    for (k, link) in links.iter().enumerate() {
        if !link.has_area {
            if ws.links.contains(&k) {
                plan.links.push(k);
            }
            continue;
        }
        let a = link.area;
        let pts = link.outline.points(a, a);
        let closed = link.outline.n > 0 && link.outline.closed;
        let (mut n, mut inside) = (0, 0);
        for gy in 0..8 {
            for gx in 0..8 {
                let x = a.x as f32 + (gx as f32 + 0.5) * a.w as f32 / 8.0;
                let y = a.y as f32 + (gy as f32 + 0.5) * a.h as f32 / 8.0;
                if closed && pts.len() >= 3 && !link.outline.hit(a, a, x, y, 0.0) {
                    continue;
                }
                n += 1;
                inside += l.inside(x, y) as i32;
            }
        }
        let by_area = n > 0 && inside * 2 >= n;
        let mid = |b: Rect| rect_contains(a, b.x + b.w / 2, b.y + b.h / 2);
        let (mut any, mut all) = (false, true);
        for i in 0..p.strokes.len() {
            let s = &p.strokes[i];
            if !stroke_is_ink(s) {
                continue;
            }
            match cut_at.get(&i) {
                Some(&c) => {
                    for (pc, cl) in &plan.cut[c].1 {
                        if mid(pc.bbox) {
                            any = true;
                            all &= *cl;
                        }
                    }
                }
                None => {
                    if mid(s.bbox) {
                        any = true;
                        all &= whole.contains(&i);
                    }
                }
            }
        }
        if by_area || (any && all) {
            plan.links.push(k);
        }
    }
    plan
}

/// A cut plan spliced into its page: each crossing stroke replaced, at its place, by its pieces in their
/// order (every other stroke keeps its relative order).
#[derive(Debug, Default)]
pub struct Spliced {
    /// the crossing strokes taken off, at their indices before the splice (ascending)
    pub originals: Vec<(usize, Box<Stroke>)>,
    /// the taken ink on the spliced page: whole strokes and inside pieces (ascending)
    pub sel: Vec<usize>,
    /// the outside pieces on the spliced page (ascending)
    pub outside: Vec<usize>,
    /// the ink taken whole: (its index before, its index on the spliced page), ascending
    pub whole: Vec<(usize, usize)>,
    /// per crossing stroke, its pieces' indices on the spliced page (to splice it back)
    pieces_at: Vec<(usize, usize)>,
}

/// Splices plan `cut` (with the ink taken whole, `whole`, at its indices before) into p.
pub fn plan_splice(p: &mut Page, whole: &[usize], cut: Vec<(usize, Vec<(Box<Stroke>, bool)>)>) -> Spliced {
    let mut out = Spliced::default();
    // new index of an old one: old + the pieces added before it (each cut adds k - 1)
    let mut shift: Vec<(usize, i64)> = Vec::with_capacity(cut.len());
    let mut acc: i64 = 0;
    for (i, v) in &cut {
        shift.push((*i, acc));
        acc += v.len() as i64 - 1;
    }
    // (the cuts before an old index each moved it on by their pieces less one)
    let new_index = |old: usize| -> usize {
        let k = shift.partition_point(|s| s.0 < old);
        (old as i64 + if k < shift.len() { shift[k].1 } else { acc }) as usize
    };
    out.whole = whole.iter().map(|&i| (i, new_index(i))).collect();
    let mut sel: Vec<usize> = out.whole.iter().map(|w| w.1).collect();
    // (back to front: the cuts before one are not spliced yet, so it is still at its own index; its pieces
    // end up after the earlier cuts' pieces)
    for (k, (i, v)) in cut.into_iter().enumerate().rev() {
        let at = (i as i64 + shift[k].1) as usize;
        let orig = page_remove(p, i as i32);
        let n = v.len();
        for (j, (pc, cl)) in v.into_iter().enumerate() {
            page_insert(p, (i + j) as i32, pc);
            if cl {
                sel.push(at + j);
            } else {
                out.outside.push(at + j);
            }
        }
        out.pieces_at.push((at, n));
        out.originals.push((i, orig));
    }
    out.originals.reverse();
    out.pieces_at.reverse();
    sel.sort_unstable();
    out.outside.sort_unstable();
    out.sel = sel;
    out
}

/// Undoes plan_splice on p (nothing else changed it since): the pieces off, the originals back where they
/// were; the pieces, in the plan's form, returned.
pub fn plan_unsplice(p: &mut Page, sp: Spliced) -> Vec<(usize, Vec<(Box<Stroke>, bool)>)> {
    let inside: HashSet<usize> = sp.sel.iter().copied().collect();
    let mut back = Vec::with_capacity(sp.originals.len());
    for ((i, orig), (at, n)) in sp.originals.into_iter().zip(sp.pieces_at).rev() {
        let mut v = Vec::with_capacity(n);
        for j in (0..n).rev() {
            let pc = page_remove(p, (at + j) as i32);
            v.push((pc, inside.contains(&(at + j))));
        }
        v.reverse();
        page_insert(p, at as i32, orig);
        back.push((i, v));
    }
    back.reverse();
    back
}
