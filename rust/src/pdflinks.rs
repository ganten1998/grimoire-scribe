//! Document links from any source into doclinks.ini (DESIGN.md 3.10): validation, seeds, fills with a 2 px
//! tolerance, the water level. A source (KOReader's `getPageLinks` here; the PDF's own link annotations, lane
//! L6) only produces `DocRecord`s: a source page, a target page, a rectangle in the notebook's page pixels
//! (the same transform as the page picture), an optional quad polygon and the annotation's index in its page.
//!
//! This file never parses PDF, font or picture syntax. KOReader's route reads a small text file the plugin
//! wrote (`import/.book/links.txt`, docs/FORMAT.md, "Links from KOReader"), strictly.

use crate::prelude::*;

/// A link a document brought, in the notebook's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocRecord {
    /// the page the link is on, and the page it goes to (page ids of the notebook)
    pub src: u32,
    pub dst: u32,
    /// its area in the notebook's page pixels
    pub rect: Rect,
    /// a quad polygon (four corners, page px) inside it, when the source drew one
    pub quad: Option<[(f32, f32); 4]>,
    /// the annotation's index in its page (the order the document drew them: later is on top)
    pub index: u32,
}

/// What a fill did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FillStats {
    pub added: usize,
    /// already there (the same target, the area within 2 px)
    pub same: usize,
    /// refused by validation
    pub invalid: usize,
    /// left out: the page already has 1,024 (the water level trims the rest)
    pub full: usize,
}

/// How far (px) the areas of two links to one target may differ and still be one link: KOReader's rects and
/// the PDF's may differ by a pixel.
pub const PDFLINKS_TOLERANCE_PX: i32 = 2;

/// The seed wanted for annotation `index` of page `src`: a hash of both, so a fill finds the same seeds.
pub fn pdflinks_seed(src: u32, index: u32) -> u32 {
    let mut h = src.wrapping_mul(0x9e37_79b1) ^ index.wrapping_mul(0x85eb_ca6b).wrapping_add(0x27d4_eb2f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    h
}

/// Whether two areas are the same link's, within the tolerance on each edge.
fn same_area(a: Rect, b: Rect) -> bool {
    let t = PDFLINKS_TOLERANCE_PX;
    (a.x - b.x).abs() <= t && (a.y - b.y).abs() <= t && ((a.x + a.w) - (b.x + b.w)).abs() <= t && ((a.y + a.h) - (b.y + b.h)).abs() <= t
}

/// Validation: both pages in the notebook, not a link to its own page, the area clamped to the page (pw x ph)
/// and at least 1 px a side after that. The area and, for a quad, its outline (None when it would not lie within
/// the area). None for a record that is refused.
pub fn pdflinks_valid(rec: &DocRecord, pw: i32, ph: i32, exists: &dyn Fn(u32) -> bool) -> Option<(Rect, Option<LinkOutline>)> {
    if rec.src == 0 || rec.dst == 0 || rec.src == rec.dst || !exists(rec.src) || !exists(rec.dst) {
        return None;
    }
    let (pw, ph) = (pw.min(NAV_AREA_MAX), ph.min(NAV_AREA_MAX));
    if pw < 1 || ph < 1 {
        return None;
    }
    let r = rec.rect;
    let x0 = (r.x as i64).clamp(0, pw as i64) as i32;
    let y0 = (r.y as i64).clamp(0, ph as i64) as i32;
    let x1 = (r.x as i64 + r.w as i64).clamp(0, pw as i64) as i32;
    let y1 = (r.y as i64 + r.h as i64).clamp(0, ph as i64) as i32;
    if x1 - x0 < 1 || y1 - y0 < 1 {
        return None;
    }
    let area = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    let outline = rec.quad.and_then(|q| {
        let mut pts = q.to_vec();
        pts.push(q[0]);
        let o = LinkOutline::from_points(&pts, true, pw, ph);
        (o.n > 0 && o.within(area)).then_some(o)
    });
    Some((area, outline))
}

/// Puts records into dl (the notebook's page size pw x ph; `exists` says whether a page id is one of the
/// notebook's; `taken(page, seed)` says whether the page's ink or a user's link already has that seed). In
/// annotation order inside a page: links come after what the page has. Idempotent: a record whose target
/// matches and whose area is within 2 px of a link the page has is that link (counted `same`, nothing
/// changed). A page takes 1,024; the rest are `full`. Call pdflinks_water after, then doclinks_save.
pub fn pdflinks_fill(
    dl: &mut DocLinks,
    recs: &[DocRecord],
    pw: i32,
    ph: i32,
    exists: &dyn Fn(u32) -> bool,
    taken: &dyn Fn(u32, u32) -> bool,
) -> FillStats {
    let mut st = FillStats::default();
    let mut order: Vec<usize> = (0..recs.len()).collect();
    order.sort_by_key(|&i| (recs[i].src, recs[i].index, i));
    for i in order {
        let rec = &recs[i];
        let Some((area, outline)) = pdflinks_valid(rec, pw, ph, exists) else {
            st.invalid += 1;
            continue;
        };
        let mine = doclinks_of(dl, rec.src);
        if mine.iter().any(|l| l.target == rec.dst && same_area(l.area(), area)) {
            st.same += 1;
            continue;
        }
        if mine.len() >= DOCLINKS_PER_PAGE {
            st.full += 1;
            continue;
        }
        let mut seed = pdflinks_seed(rec.src, rec.index);
        for _ in 0..64 {
            if !doclinks_has_seed(dl, rec.src, seed) && !taken(rec.src, seed) {
                break;
            }
            seed = seed.wrapping_mul(2654435761).wrapping_add(0x9e3779b9);
        }
        let Some(l) = DocLink::new(rec.src, seed, rec.dst, area) else {
            st.invalid += 1;
            continue;
        };
        if !dl.push(l, outline) {
            st.invalid += 1;
            continue;
        }
        st.added += 1;
    }
    st
}

/// The water level (DESIGN 3.10): trims dl to the most links under 90% of both caps, counted in links and in
/// the bytes the file would have (outline lines included), the densest pages first, each page keeping its first
/// links in annotation order. The 10% left is room for Copy page and later fills. Returns how many links went.
pub fn pdflinks_water(dl: &mut DocLinks) -> usize {
    let max_n = DOCLINKS_MAX * DOCLINKS_WATER_PCT / 100;
    let max_b = DOCLINKS_FILE_MAX * DOCLINKS_WATER_PCT / 100;
    let total_b = doclinks_bytes(dl);
    if dl.links.len() <= max_n && total_b <= max_b {
        return 0;
    }
    let base = total_b - dl.links.iter().map(|l| doclinks_link_bytes(dl, l)).sum::<usize>();
    // rank of each link in its page, and its bytes
    let mut rank = Vec::with_capacity(dl.links.len());
    let mut cost = Vec::with_capacity(dl.links.len());
    let (mut last, mut run) = (0u32, 0usize);
    for l in &dl.links {
        run = if l.pid == last { run + 1 } else { 1 };
        last = l.pid;
        rank.push(run);
        cost.push(doclinks_link_bytes(dl, l));
    }
    let widest = rank.iter().copied().max().unwrap_or(0);
    let level = |lv: usize| -> (usize, usize) {
        let mut n = 0;
        let mut b = base;
        for i in 0..rank.len() {
            if rank[i] <= lv {
                n += 1;
                b += cost[i];
            }
        }
        (n, b)
    };
    // the largest per-page count that fits both
    let (mut lo, mut hi) = (0usize, widest);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let (n, b) = level(mid);
        if n <= max_n && b <= max_b {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let (mut n, mut b) = level(lo);
    // the pages above the level take one more each, by page, while the room lasts
    let mut keep: Vec<bool> = rank.iter().map(|&r| r <= lo).collect();
    for i in 0..rank.len() {
        if rank[i] == lo + 1 {
            if n < max_n && b + cost[i] <= max_b {
                keep[i] = true;
                n += 1;
                b += cost[i];
            }
        }
    }
    let removed = keep.iter().filter(|&&k| !k).count();
    let mut k = 0;
    dl.retain(|_| {
        k += 1;
        keep[k - 1]
    });
    removed
}

// ---- KOReader's links.txt ----

/// The most a links.txt is read at, and its link lines.
pub const BOOK_LINKS_FILE_MAX: usize = 8 << 20;
pub const BOOK_LINKS_MAX: usize = 200_000;

/// What import/.book/links.txt holds: the size of the pictures the rectangles are drawn in, and each link as
/// book page numbers (from 1) with its rectangle in those pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookLinks {
    pub pic_w: i32,
    pub pic_h: i32,
    pub links: Vec<(i32, i32, Rect)>,
}

/// links.txt, strictly: `P <w> <h>` first (the pictures' size), then `L <src> <dst> <x> <y> <w> <h>` lines (book
/// pages from 1, whole pixels), `#` lines and blank ones skipped. None when anything else is in it, a number is
/// out of range or there are more links than BOOK_LINKS_MAX (a file is taken whole or not at all).
pub fn pdflinks_parse_book(text: &[u8]) -> Option<BookLinks> {
    let text = std::str::from_utf8(text).ok()?;
    let mut out = BookLinks::default();
    let mut have_size = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut f = line.split(' ');
        let kind = f.next()?;
        let mut num = || -> Option<i64> {
            let s = f.next()?;
            if s.is_empty() || s.len() > 9 || !s.bytes().all(|c| c.is_ascii_digit() || c == b'-') {
                return None;
            }
            s.parse::<i64>().ok()
        };
        match kind {
            "P" => {
                let (w, h) = (num()?, num()?);
                if have_size || !(1..=NAV_AREA_MAX as i64).contains(&w) || !(1..=NAV_AREA_MAX as i64).contains(&h) {
                    return None;
                }
                out.pic_w = w as i32;
                out.pic_h = h as i32;
                have_size = true;
            }
            "L" => {
                if !have_size || out.links.len() >= BOOK_LINKS_MAX {
                    return None;
                }
                let (s, d, x, y, w, h) = (num()?, num()?, num()?, num()?, num()?, num()?);
                if !(1..=STORE_MAX_PAGES as i64).contains(&s) || !(1..=STORE_MAX_PAGES as i64).contains(&d) || w < 1 || h < 1 {
                    return None;
                }
                if !(-100_000..=100_000).contains(&x) || !(-100_000..=100_000).contains(&y) || w > 100_000 || h > 100_000 {
                    return None;
                }
                out.links.try_reserve(1).ok()?;
                out.links.push((s as i32, d as i32, Rect { x: x as i32, y: y as i32, w: w as i32, h: h as i32 }));
            }
            _ => return None,
        }
        if f.next().is_some() {
            return None;
        }
    }
    have_size.then_some(out)
}

/// A rectangle drawn in a pic_w x pic_h picture, in the page pixels (pw x ph) the picture is fitted into
/// (aspect kept, centred: the page picture's own fit, so the box lands on what it covers).
pub fn pdflinks_to_page(r: Rect, pic_w: i32, pic_h: i32, pw: i32, ph: i32) -> Rect {
    if pic_w == pw && pic_h == ph {
        return r;
    }
    let fit = fit_box(pic_w.max(1), pic_h.max(1), pw, ph);
    let sx = fit.w as f32 / pic_w.max(1) as f32;
    let sy = fit.h as f32 / pic_h.max(1) as f32;
    let x0 = fit.x as f32 + r.x as f32 * sx;
    let y0 = fit.y as f32 + r.y as f32 * sy;
    let x1 = fit.x as f32 + (r.x + r.w) as f32 * sx;
    let y1 = fit.y as f32 + (r.y + r.h) as f32 * sy;
    let (ix, iy) = (x0.floor() as i32, y0.floor() as i32);
    Rect { x: ix, y: iy, w: (x1.ceil() as i32 - ix).max(1), h: (y1.ceil() as i32 - iy).max(1) }
}
