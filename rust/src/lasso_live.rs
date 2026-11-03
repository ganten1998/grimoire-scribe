//! lasso_live.rs: the live selection preview (docs/beta2/PIXEL-LASSO.md 2): while a lasso is drawn,
//! which ink the lasso closed from the pen back to its start would select now, kept up to date in small
//! batches and giving exactly the answer the lift gives (lasso.rs lasso_enclosed), so the lift reuses it.
//!
//! Going from points p0..pn to p0..pm, the closed polygon loses the edge pn -> p0 and gains the chain
//! pn -> ... -> pm -> p0: the winding anywhere changes by the winding of the small closed polygon
//! D = (p0, pn, ..., pm), a thin fan. Each sample of a stroke keeps its winding as the integer sum of its
//! edges' contributions (edge_rows and edge_cross, the expressions Lasso::new uses), so a removed edge
//! subtracts exactly what it once added. Only samples inside D's rows, between the first and the last of
//! D's crossings in their row, can change; a grid of 64 px cells finds the strokes there, and a stroke's
//! samples (lasso.rs judged_samples) are made the first time D reaches it.
//!
//! Pure: the editor (select.rs) feeds it points and draws what it says.

use crate::prelude::*;

/// The grid's cell, page px (also the band of rows D's crossings are gathered by).
const CELL: i32 = 64;
/// Samples per chunk: a chunk's box skips its samples at once.
const CHUNK: usize = 32;

/// One ink stroke's state, made the first time a batch reaches it.
struct Track {
    j: Judged,
    /// each sample's winding: the sum of the lasso's edges' contributions
    w: Vec<i32>,
    /// each chunk's box (x0, y0, x1, y1)
    chunks: Vec<(f32, f32, f32, f32)>,
    /// the length of the samples whose winding is not 0
    inside: u64,
}

/// The ink of a 2.0.0-beta.5 link with no area (its seed): all or nothing, by length (lasso_select_from).
struct Group {
    members: Vec<usize>,
    lens: Vec<f32>,
    total: f32,
}

/// The live preview of one lasso on one page.
pub struct LassoLive {
    dpi: f32,
    /// the page it was made on: its strokes by identity (an edit in the meantime makes it stale)
    ids: Vec<StrokeId>,
    erasers: Vec<usize>,
    floors: Option<Box<FloorTable>>,
    /// the grid: its first cell (cells), size (cells), and the ink strokes whose box meets each cell
    gx0: i32,
    gy0: i32,
    gw: i32,
    gh: i32,
    cells: Vec<Vec<u32>>,
    tracks: Vec<Option<Box<Track>>>,
    /// enclosed (before the link rule), and shown (after it)
    raw: Vec<bool>,
    shown: Vec<bool>,
    groups: Vec<Group>,
    group_of: Vec<u32>,
    stamp: Vec<u32>,
    cur: u32,
    /// the lasso's first point and the last one folded in, and how many points it has folded
    p0: (f32, f32),
    last: (f32, f32),
    /// the points' extent (whole rows: Lasso::new's table limits)
    y_lo: f32,
    y_hi: f32,
    /// past the table's LASSO_ROWS_MAX rows the lift clamps them: the preview answers from a table then
    clamped: bool,
    /// batches folded, and samples tested (for the log and the probes)
    pub batches: u32,
    pub tests: u64,
    pub made: u32,
    /// exact area: a stroke whose inside samples changed is reported even when its enclosure did not
    /// (its halo covers its inside samples' runs)
    exact: bool,
    /// the strokes the exact-area lift judges whole (beta.5 links' ink, strokes it cannot cut)
    whole_only: Vec<bool>,
}

const NO_GROUP: u32 = u32::MAX;

impl LassoLive {
    /// The preview of a lasso starting at page point p0 on page p, whose links are `links` (page px).
    pub fn new(p: &Page, links: &[NavLink], dpi: f32, p0: (f32, f32)) -> LassoLive {
        let n = p.strokes.len();
        let ink: Vec<usize> = (0..n).filter(|&i| stroke_is_ink(&p.strokes[i]) && !p.strokes[i].pts.is_empty()).collect();
        // the grid over the ink's boxes (a point off the page is still on its stroke's box)
        let mut u = RECT_EMPTY;
        for &i in &ink {
            u = rect_union(u, rect_inflate(p.strokes[i].bbox, 1));
        }
        let (gx0, gy0, gw, gh) = if rect_is_empty(u) {
            (0, 0, 0, 0)
        } else {
            let (x0, y0) = (u.x.div_euclid(CELL), u.y.div_euclid(CELL));
            let (x1, y1) = ((u.x + u.w - 1).div_euclid(CELL), (u.y + u.h - 1).div_euclid(CELL));
            // (boxes are clamped to a sane page: a corrupt one cannot make a huge grid)
            let x0 = x0.max(-512);
            let y0 = y0.max(-512);
            let (gw, gh) = ((x1 - x0 + 1).clamp(0, 1024), (y1 - y0 + 1).clamp(0, 1024));
            // (ink wholly left of or above that sane page: no grid, as for no ink; the cells below
            // clamp to gw - 1 and gh - 1)
            if gw == 0 || gh == 0 { (0, 0, 0, 0) } else { (x0, y0, gw, gh) }
        };
        let mut cells: Vec<Vec<u32>> = vec![Vec::new(); (gw * gh) as usize];
        for &i in ink.iter().filter(|_| gw > 0) {
            let b = rect_inflate(p.strokes[i].bbox, 1);
            let cx0 = (b.x.div_euclid(CELL) - gx0).clamp(0, gw - 1);
            let cy0 = (b.y.div_euclid(CELL) - gy0).clamp(0, gh - 1);
            let cx1 = ((b.x + b.w - 1).div_euclid(CELL) - gx0).clamp(0, gw - 1);
            let cy1 = ((b.y + b.h - 1).div_euclid(CELL) - gy0).clamp(0, gh - 1);
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    cells[(cy * gw + cx) as usize].push(i as u32);
                }
            }
        }
        // the beta.5 links' ink: all or nothing (as lasso_select_from judges them, in its order)
        let mut groups = Vec::new();
        let mut group_of = vec![NO_GROUP; n];
        for l in links.iter().filter(|l| !l.has_area) {
            let members: Vec<usize> = (0..n).filter(|&i| stroke_is_ink(&p.strokes[i]) && p.strokes[i].seed == l.seed).collect();
            if members.is_empty() || members.iter().any(|&i| group_of[i] != NO_GROUP) {
                continue;
            }
            let lens: Vec<f32> = members.iter().map(|&i| stroke_length(&p.strokes[i]).max(1.0)).collect();
            let total: f32 = lens.iter().sum();
            for &i in &members {
                group_of[i] = groups.len() as u32;
            }
            groups.push(Group { members, lens, total });
        }
        LassoLive {
            dpi,
            ids: p.strokes.iter().map(|s| stroke_id(s)).collect(),
            erasers: page_erasers(p),
            floors: page_floors(p),
            gx0,
            gy0,
            gw,
            gh,
            cells,
            tracks: (0..n).map(|_| None).collect(),
            raw: vec![false; n],
            shown: vec![false; n],
            groups,
            group_of,
            stamp: vec![0; n],
            cur: 0,
            p0,
            last: p0,
            y_lo: p0.1,
            y_hi: p0.1,
            clamped: false,
            batches: 0,
            tests: 0,
            made: 0,
            exact: false,
            whole_only: (0..n).map(|i| p.strokes[i].pts.is_empty() || !stroke_cuttable(&p.strokes[i]) || links.iter().any(|l| !l.has_area && l.seed == p.strokes[i].seed)).collect(),
        }
    }

    /// Exact area (or whole strokes): what the preview reports and shows (inside_runs).
    pub fn set_exact(&mut self, on: bool) {
        self.exact = on;
    }

    /// Exact area: stroke i's judged samples (x, y, inside) when its halo covers the runs inside the
    /// lasso, the cut ones; None when it shows whole or not at all (shown()).
    pub fn inside_runs(&self, i: usize) -> Option<Vec<(f32, f32, bool)>> {
        if !self.exact || self.whole_only.get(i).copied().unwrap_or(true) {
            return None;
        }
        let t = self.tracks.get(i)?.as_ref()?;
        if !t.j.long || t.inside == 0 || t.inside == t.j.total {
            return None;
        }
        Some(t.j.samples.iter().zip(&t.w).map(|(q, &w)| (q.0, q.1, w != 0)).collect())
    }

    /// The strokes with a halo now, ascending: whole (None), or exact area's inside runs (inside_runs).
    pub fn halo_list(&self) -> Vec<(usize, Option<Vec<(f32, f32, bool)>>)> {
        let mut out = Vec::new();
        for i in 0..self.shown.len() {
            if !self.exact || self.whole_only[i] {
                if self.shown[i] {
                    out.push((i, None));
                }
                continue;
            }
            let Some(t) = self.tracks[i].as_ref() else { continue };
            if !t.j.long {
                if self.shown[i] {
                    out.push((i, None));
                }
            } else if t.inside == t.j.total && t.inside > 0 {
                out.push((i, None));
            } else if t.inside > 0 {
                out.push((i, self.inside_runs(i)));
            }
        }
        out
    }

    /// Whether p is still the page it was made on (no stroke added, taken off or replaced since).
    pub fn holds(&self, p: &Page) -> bool {
        p.strokes.len() == self.ids.len() && p.strokes.iter().zip(&self.ids).all(|(s, &id)| stroke_id(s) == id)
    }

    /// Whether stroke i shows as selected now.
    pub fn shown(&self, i: usize) -> bool {
        self.shown.get(i).copied().unwrap_or(false)
    }

    /// The strokes shown as selected now, ascending.
    pub fn shown_list(&self) -> Vec<usize> {
        (0..self.shown.len()).filter(|&i| self.shown[i]).collect()
    }

    /// The ink the lasso encloses now, before the links' rules: lasso_enclosed's answer for the lasso as
    /// folded so far (closed from its last folded point to its first).
    pub fn enclosed(&self) -> Vec<bool> {
        self.raw.clone()
    }

    /// For tests: how many samples made so far have a winding other than the lift's table gives for
    /// lasso `pts` (the points folded so far): 0 when the preview is exact.
    pub fn windings_off(&self, pts: &[(f32, f32)]) -> usize {
        let l = Lasso::new(pts);
        let mut off = 0;
        for t in self.tracks.iter().flatten() {
            for (q, &(x, y, _)) in t.j.samples.iter().enumerate() {
                let want = l.as_ref().map_or(0, |l| l.winding(x, y));
                off += (t.w[q] != want) as usize;
            }
        }
        off
    }

    fn track(&mut self, p: &Page, i: usize) {
        if self.tracks[i].is_some() {
            return;
        }
        let Some(j) = judged_samples(p, self.floors.as_deref(), i, self.dpi, &self.erasers) else {
            self.tracks[i] = Some(Box::new(Track { j: Judged { samples: Vec::new(), total: 1, long: false }, w: Vec::new(), chunks: Vec::new(), inside: 0 }));
            return;
        };
        let chunks = j
            .samples
            .chunks(CHUNK)
            .map(|c| c.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, q| (b.0.min(q.0), b.1.min(q.1), b.2.max(q.0), b.3.max(q.1))))
            .collect();
        let w = vec![0; j.samples.len()];
        self.tracks[i] = Some(Box::new(Track { j, w, chunks, inside: 0 }));
        self.made += 1;
    }

    /// Stroke i's enclosure after its windings changed; its shown flag (and its link's ink's) after that.
    /// Pushes the strokes whose shown flag flipped.
    fn settle(&mut self, i: usize, flips: &mut Vec<usize>) {
        let t = self.tracks[i].as_ref().unwrap();
        let on = !t.j.samples.is_empty() && t.j.decide(t.inside);
        if self.exact && !self.whole_only[i] && t.j.long {
            // (its inside runs changed: drawn again whatever its enclosure says)
            flips.push(i);
        }
        if on == self.raw[i] {
            return;
        }
        self.raw[i] = on;
        let g = self.group_of[i];
        if g == NO_GROUP {
            self.shown[i] = on;
            flips.push(i);
            return;
        }
        let gr = &self.groups[g as usize];
        let picked: f32 = gr.members.iter().zip(&gr.lens).filter(|(&m, _)| self.raw[m]).map(|(_, &l)| l).sum();
        let go = picked * 2.0 >= gr.total;
        for &m in &gr.members {
            if self.shown[m] != go {
                self.shown[m] = go;
                flips.push(m);
            }
        }
    }

    fn extent(&mut self, pts: &[(f32, f32)]) {
        for &(_, y) in pts {
            self.y_lo = self.y_lo.min(y);
            self.y_hi = self.y_hi.max(y);
        }
        let ry0 = self.y_lo.floor() as i64;
        let ry1 = self.y_hi.ceil() as i64;
        self.clamped = ry0 < -(LASSO_ROWS_MAX as i64) || ry1 - ry0.max(-(LASSO_ROWS_MAX as i64)) > LASSO_ROWS_MAX as i64;
    }

    /// Folds in points pts[from..] (pts: the whole lasso so far, pts[0] its first point; nothing before
    /// `from` changed since the last fold). Returns the strokes whose shown flag flipped.
    pub fn extend(&mut self, p: &Page, pts: &[(f32, f32)], from: usize) -> Vec<usize> {
        let new = &pts[from.min(pts.len())..];
        let mut flips = Vec::new();
        if new.is_empty() || pts.is_empty() {
            return flips;
        }
        self.batches += 1;
        let (p0, pn, pm) = (self.p0, self.last, *new.last().unwrap());
        self.last = pm;
        self.extent(new);
        if self.clamped {
            self.answer_from_table(p, pts, &mut flips);
            return flips;
        }
        // D's crossings, row by row: the old closing edge taken away, the new stretch and closing edge added
        let mut ylo = p0.1.min(pn.1);
        let mut yhi = p0.1.max(pn.1);
        for &(_, y) in new {
            ylo = ylo.min(y);
            yhi = yhi.max(y);
        }
        let r0 = (ylo - 0.5).ceil() as i32;
        let r1 = (yhi - 0.5).floor() as i32;
        if r1 < r0 {
            return flips;
        }
        let mut rows: Vec<Vec<(f32, i32)>> = vec![Vec::new(); (r1 - r0 + 1) as usize];
        let mut add = |a: (f32, f32), b: (f32, f32), sign: i32| {
            let Some((first, last, dir)) = edge_rows(a, b) else { return };
            for r in first.max(r0)..=last.min(r1) {
                if let Some(x) = edge_cross(a, b, r) {
                    rows[(r - r0) as usize].push((x, sign * dir));
                }
            }
        };
        add(pn, p0, -1);
        let mut a = pn;
        for &b in new {
            add(a, b, 1);
            a = b;
        }
        add(pm, p0, 1);
        for row in rows.iter_mut() {
            row.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut w = 0;
            for c in row.iter_mut() {
                w += c.1;
                c.1 = w;
            }
        }
        // band by band (CELL rows): the x range between D's first and last crossings there
        let b0 = r0.div_euclid(CELL);
        let b1 = r1.div_euclid(CELL);
        for band in b0..=b1 {
            let (y0, y1) = (band * CELL, band * CELL + CELL);
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for r in y0.max(r0)..y1.min(r1 + 1) {
                let row = &rows[(r - r0) as usize];
                if let (Some(f), Some(l)) = (row.first(), row.last()) {
                    lo = lo.min(f.0);
                    hi = hi.max(l.0);
                }
            }
            if lo > hi || self.gw == 0 {
                continue;
            }
            let cy = band - self.gy0;
            if cy < 0 || cy >= self.gh {
                continue;
            }
            let cx0 = ((lo.floor() as i32).div_euclid(CELL) - self.gx0).clamp(0, self.gw - 1);
            let cx1 = ((hi.ceil() as i32).div_euclid(CELL) - self.gx0).clamp(0, self.gw - 1);
            self.cur = self.cur.wrapping_add(1);
            if self.cur == 0 {
                self.stamp.iter_mut().for_each(|s| *s = 0);
                self.cur = 1;
            }
            for cx in cx0..=cx1 {
                let cell = std::mem::take(&mut self.cells[(cy * self.gw + cx) as usize]);
                for &si in &cell {
                    let i = si as usize;
                    if self.stamp[i] == self.cur {
                        continue;
                    }
                    self.stamp[i] = self.cur;
                    self.track(p, i);
                    let t = self.tracks[i].as_mut().unwrap();
                    let (fy0, fy1) = (y0 as f32, y1 as f32);
                    let mut changed = false;
                    for (k, c) in t.chunks.iter().enumerate() {
                        if c.3 < fy0 || c.1 >= fy1 || c.2 <= lo || c.0 > hi {
                            continue;
                        }
                        let base = k * CHUNK;
                        let end = (base + CHUNK).min(t.j.samples.len());
                        for q in base..end {
                            let (x, y, len) = t.j.samples[q];
                            if x <= lo || x > hi {
                                continue;
                            }
                            let r = y.floor() as i32;
                            if r < y0 || r >= y1 || r < r0 || r > r1 {
                                continue;
                            }
                            self.tests += 1;
                            let row = &rows[(r - r0) as usize];
                            let at = row.partition_point(|c| c.0 < x);
                            if at == 0 {
                                continue;
                            }
                            let d = row[at - 1].1;
                            if d == 0 {
                                continue;
                            }
                            let before = t.w[q] != 0;
                            t.w[q] += d;
                            let after = t.w[q] != 0;
                            if before != after {
                                if after {
                                    t.inside += len as u64;
                                } else {
                                    t.inside -= len as u64;
                                }
                                changed = true;
                            }
                        }
                    }
                    if changed {
                        self.settle(i, &mut flips);
                    }
                }
                self.cells[(cy * self.gw + cx) as usize] = cell;
            }
        }
        flips
    }

    /// The lasso changed everywhere (a full lasso dropped every other point): every sample's winding from a
    /// fresh crossing table, the lift's own (strokes not reached yet that the new polygon reaches are made
    /// now). Returns the strokes whose shown flag flipped.
    pub fn rebuild(&mut self, p: &Page, pts: &[(f32, f32)]) -> Vec<usize> {
        let mut flips = Vec::new();
        if pts.is_empty() {
            return flips;
        }
        self.batches += 1;
        self.p0 = pts[0];
        self.last = *pts.last().unwrap();
        self.y_lo = pts[0].1;
        self.y_hi = pts[0].1;
        self.extent(pts);
        self.answer_from_table(p, pts, &mut flips);
        flips
    }

    fn answer_from_table(&mut self, p: &Page, pts: &[(f32, f32)], flips: &mut Vec<usize>) {
        let l = Lasso::new(pts);
        for i in 0..self.tracks.len() {
            let reach = l.as_ref().is_some_and(|l| stroke_is_ink(&p.strokes[i]) && !p.strokes[i].pts.is_empty() && rect_overlaps(p.strokes[i].bbox, l.bounds));
            if reach {
                self.track(p, i);
            }
            let Some(t) = self.tracks[i].as_mut() else { continue };
            t.inside = 0;
            for q in 0..t.j.samples.len() {
                let (x, y, len) = t.j.samples[q];
                t.w[q] = match (&l, reach) {
                    (Some(l), true) => l.winding(x, y),
                    _ => 0,
                };
                if t.w[q] != 0 {
                    t.inside += len as u64;
                }
            }
            self.tests += t.j.samples.len() as u64;
            self.settle(i, flips);
        }
    }
}
