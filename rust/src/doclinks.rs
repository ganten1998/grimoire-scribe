//! doclinks.ini: the links a document brings with it (a planner PDF's tabs: KOReader's `getPageLinks`, the
//! PDF's own link annotations), kept apart from the user's own links (links.ini) so that neither file's caps
//! or readers ever meet the other's (DESIGN.md 3.10, D15).
//!
//! The file is `notebooks/<id>/doclinks.ini`, in links.ini's grammar (`link.`, `area.`, `outline.`), every
//! target a page of the same notebook (`p<page id>`). Lines a version does not know are carried. A 2.0 build
//! never reads it.
//!
//! In memory a link is 24 bytes (a page id, a seed, a target page id, an area in four i16 and an index into a
//! side list of outlines), the links sorted by source page id and, inside a page, in the order the document
//! drew them (annotation order: of two equal boxes the later one wins a tap, as a PDF viewer picks it).
//!
//! Who reads it: the editor (taps, the eye, the Links bar's row, the card), Copy page and Copy to notebook,
//! the notebook PDF and raw zips. Who never does: backlinks, libindex and `graph_build` (so a planner never
//! fills the Graph), the lasso and Select, Clear page, the clipboard, and the 64-a-page and 4,000-a-notebook
//! checks of links.ini.
//!
//! Who writes it: pdflinks.rs's fills (the book import at commit; a PDF import's writer), and Copy page and
//! Copy to notebook. Never a page turn, a Clear or a lasso.

use crate::prelude::*;

/// links in a file
pub const DOCLINKS_MAX: usize = 16_000;
/// the file's size
pub const DOCLINKS_FILE_MAX: usize = 3 << 19; // 1.5 MiB
/// links on a page
pub const DOCLINKS_PER_PAGE: usize = 1024;
/// carried lines
pub const DOCLINKS_UNKNOWN_MAX: usize = 16 * 1024;
/// the water level: the share of both caps a fill leaves itself, in percent (the rest is room for Copy
/// page and later fills)
pub const DOCLINKS_WATER_PCT: usize = 90;

const NO_OUTLINE: u32 = u32::MAX;
const HEADER: &str = "# Grimoire: links a document brought (docs/FORMAT.md, doclinks.ini)\n";

/// One document link, 24 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocLink {
    pub pid: u32,
    pub seed: u32,
    /// the target page's id (a page of the same notebook)
    pub target: u32,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    outline: u32,
}

impl DocLink {
    /// A link with this area (1 to NAV_AREA_MAX px a side, on the page side of it: area_valid) and no outline.
    pub fn new(pid: u32, seed: u32, target: u32, area: Rect) -> Option<DocLink> {
        if pid == 0 || target == 0 || !nav_area_valid(area) {
            return None;
        }
        Some(DocLink { pid, seed, target, x: area.x as i16, y: area.y as i16, w: area.w as i16, h: area.h as i16, outline: NO_OUTLINE })
    }
    pub fn area(&self) -> Rect {
        Rect { x: self.x as i32, y: self.y as i32, w: self.w as i32, h: self.h as i32 }
    }
    pub fn has_outline(&self) -> bool {
        self.outline != NO_OUTLINE
    }
}

#[derive(Clone, Debug)]
pub struct DocLinks {
    /// sorted by page id (stable: inside a page, annotation order)
    pub links: Vec<DocLink>,
    outlines: Vec<LinkOutline>,
    /// doclinks.ini's lines this version does not read, carried as links.ini's are
    pub unknown: Vec<u8>,
    /// false after a file that did not read: never overwrite it
    pub writable: bool,
    /// doclinks.ini is there (else it is written only once there is a link)
    pub file: bool,
}

impl Default for DocLinks {
    fn default() -> Self {
        DocLinks { links: Vec::new(), outlines: Vec::new(), unknown: Vec::new(), writable: true, file: false }
    }
}

impl DocLinks {
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
    /// The outline link l has, if it has one.
    pub fn outline_of(&self, l: &DocLink) -> Option<&LinkOutline> {
        self.outlines.get(l.outline as usize)
    }
    /// Adds a link at the end of its page's, with its outline when it has one (None, or one that does not
    /// lie within the link's area, for none). Keeps the order; false for no memory.
    pub fn push(&mut self, mut l: DocLink, outline: Option<LinkOutline>) -> bool {
        if self.links.try_reserve(1).is_err() {
            return false;
        }
        l.outline = NO_OUTLINE;
        if let Some(o) = outline.filter(|o| o.n > 0 && o.within(l.area())) {
            if self.outlines.try_reserve(1).is_err() {
                return false;
            }
            l.outline = self.outlines.len() as u32;
            self.outlines.push(o);
        }
        let at = self.links.partition_point(|k| k.pid <= l.pid);
        self.links.insert(at, l);
        true
    }
    /// Keeps the links f says yes to, and the outlines they use.
    pub fn retain(&mut self, mut f: impl FnMut(&DocLink) -> bool) {
        let before = self.links.len();
        self.links.retain(|l| f(l));
        if self.links.len() != before || self.outlines.len() > self.links.iter().filter(|l| l.has_outline()).count() {
            self.compact_outlines();
        }
    }
    fn compact_outlines(&mut self) {
        let old = std::mem::take(&mut self.outlines);
        for l in self.links.iter_mut() {
            if l.has_outline() {
                let o = old[l.outline as usize];
                l.outline = self.outlines.len() as u32;
                self.outlines.push(o);
            }
        }
    }
}

/// The links of page pid, in annotation order.
pub fn doclinks_of(dl: &DocLinks, pid: u32) -> &[DocLink] {
    let a = dl.links.partition_point(|l| l.pid < pid);
    let b = dl.links.partition_point(|l| l.pid <= pid);
    &dl.links[a..b]
}

/// Whether page pid has a document link with this seed.
pub fn doclinks_has_seed(dl: &DocLinks, pid: u32, seed: u32) -> bool {
    doclinks_of(dl, pid).iter().any(|l| l.seed == seed)
}

// ---- reading ----

fn line_end(text: &[u8], at: usize) -> (usize, usize) {
    match text[at..].iter().position(|&c| c == b'\n') {
        Some(k) => (at + k, at + k + 1),
        None => (text.len(), text.len()),
    }
}

fn carry_line(buf: &mut Vec<u8>, line: &[u8]) -> bool {
    if line.len() > 255 || buf.len() + line.len() + 1 > DOCLINKS_UNKNOWN_MAX || buf.try_reserve(line.len() + 1).is_err() {
        return false;
    }
    buf.extend_from_slice(line);
    buf.push(b'\n');
    true
}

/// A doclinks.ini's text read strictly: None when it holds more links than the cap, a carried line past its
/// bounds, or runs out of memory (a file that does not read is never written over). A link line whose target
/// is not a page of this notebook (a future version's) and its area and outline lines are carried.
pub fn doclinks_parse(text: &[u8]) -> Option<DocLinks> {
    let mut dl = DocLinks::default();
    // (page, seed) -> index into dl.links: lines come in any order
    let mut at: std::collections::HashMap<(u32, u32), usize> = std::collections::HashMap::new();
    let mut carried: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    let mut lines: Vec<&[u8]> = Vec::new();
    let mut p = 0;
    while p < text.len() {
        let (e, next) = line_end(text, p);
        let mut line = &text[p..e];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        p = next;
        if !line.is_empty() && line[0] != b'#' {
            lines.push(line);
        }
    }
    // Links first (a link's own order is its annotation order), then their areas and outlines.
    for line in lines.iter() {
        if let Some((pid, seed, target)) = link_parse(line) {
            let to = target_page(target.as_bytes());
            if to == 0 {
                carried.insert((pid, seed));
                if !carry_line(&mut dl.unknown, line) {
                    return None;
                }
                continue;
            }
            if let Some(&k) = at.get(&(pid, seed)) {
                dl.links[k].target = to; // (a later line for the same link wins)
                continue;
            }
            if dl.links.len() >= DOCLINKS_MAX {
                return None;
            }
            if dl.links.try_reserve(1).is_err() {
                return None;
            }
            at.insert((pid, seed), dl.links.len());
            dl.links.push(DocLink { pid, seed, target: to, x: 0, y: 0, w: 0, h: 0, outline: NO_OUTLINE });
        } else if link_key(line).is_some() {
            // a link line of a form this version cannot read (a bad seed or target): carried whole
            if let Some((pid, seed, _)) = link_key(line) {
                carried.insert((pid, seed));
            }
            if !carry_line(&mut dl.unknown, line) {
                return None;
            }
        } else if area_parse(line).is_none() && outline_parse(line).is_none() && !carry_line(&mut dl.unknown, line) {
            // a line of a form this version does not know (a future version's): carried as it is
            return None;
        }
    }
    let mut has_area = vec![false; dl.links.len()];
    for line in &lines {
        if let Some((pid, seed, r)) = area_parse(line) {
            if let Some(&k) = at.get(&(pid, seed)) {
                let l = &mut dl.links[k];
                l.x = r.x as i16;
                l.y = r.y as i16;
                l.w = r.w as i16;
                l.h = r.h as i16;
                has_area[k] = true;
            } else if carried.contains(&(pid, seed)) && !carry_line(&mut dl.unknown, line) {
                return None;
            }
        }
    }
    for line in &lines {
        if let Some((pid, seed, o)) = outline_parse(line) {
            if let Some(&k) = at.get(&(pid, seed)) {
                if has_area[k] && o.within(dl.links[k].area()) {
                    if dl.outlines.try_reserve(1).is_err() {
                        return None;
                    }
                    dl.links[k].outline = dl.outlines.len() as u32;
                    dl.outlines.push(o);
                } else if !carry_line(&mut dl.unknown, line) {
                    return None;
                }
            } else if carried.contains(&(pid, seed)) && !carry_line(&mut dl.unknown, line) {
                return None;
            }
        }
    }
    // A link with no area is no document link (it has nowhere to be tapped): left out. Its link line is carried
    // with its area line (one a later version wrote in a form this one cannot read), so that the next write
    // does not take the link from the version that wrote it.
    if has_area.iter().any(|&h| !h) {
        for (k, l) in dl.links.iter().enumerate() {
            if !has_area[k] {
                if let Some(line) = lines.iter().find(|ln| link_parse(ln).is_some_and(|(p, s, _)| p == l.pid && s == l.seed)) {
                    if !carry_line(&mut dl.unknown, line) {
                        return None;
                    }
                }
            }
        }
        let mut k = 0;
        dl.links.retain(|_| {
            k += 1;
            has_area[k - 1]
        });
        dl.compact_outlines();
    }
    // Annotation order is file order; sorting by page keeps it.
    dl.links.sort_by_key(|l| l.pid);
    dl.compact_outlines();
    let mut run = 0usize;
    let mut last = 0u32;
    for l in &dl.links {
        if l.pid == 0 {
            return None;
        }
        // (a page past its cap does not read, as FORMAT.md says: a file that could never be written again is
        // not shown as if it could)
        run = if l.pid == last { run + 1 } else { 1 };
        last = l.pid;
        if run > DOCLINKS_PER_PAGE {
            return None;
        }
    }
    Some(dl)
}

fn is_unreadable() -> DocLinks {
    DocLinks { writable: false, file: true, ..Default::default() }
}

/// Notebook id's doclinks.ini. Missing is an empty writable set. One that is there and does not read (not a
/// regular file, over the cap, a carried line too long, more links than the cap) reads as none and refuses
/// writes, so it is never written over.
pub fn doclinks_load(id: &str) -> DocLinks {
    let Some(path) = path_for(id, "doclinks.ini") else { return is_unreadable() };
    match read_sidecar(&path, DOCLINKS_FILE_MAX) {
        Ok(None) => DocLinks::default(),
        Ok(Some(text)) => match doclinks_parse(&text) {
            Some(mut dl) => {
                dl.file = true;
                dl
            }
            None => {
                qlog!("doclinks: doclinks.ini could not be read; left unchanged");
                is_unreadable()
            }
        },
        Err(()) => {
            qlog!("doclinks: doclinks.ini could not be read; left unchanged");
            is_unreadable()
        }
    }
}

/// Drops the links of pages that are no longer in nb and those going to such pages, from memory (a page
/// deleted in the Pages view leaves them in doclinks.ini until some later write). Returns how many went. The
/// file is not touched.
pub fn doclinks_prune(dl: &mut DocLinks, nb: &NotebookInfo) -> usize {
    let before = dl.links.len();
    dl.retain(|l| has_page(nb, l.pid) && has_page(nb, l.target));
    before - dl.links.len()
}

// ---- writing ----

/// The bytes link l takes in the file (its link line, its area line and its outline line).
pub fn doclinks_link_bytes(dl: &DocLinks, l: &DocLink) -> usize {
    link_bytes(l, dl.outline_of(l))
}

/// The bytes link l takes with this outline (a link not in a set yet has no index to ask for it by).
fn link_bytes(l: &DocLink, outline: Option<&LinkOutline>) -> usize {
    let mut n = format!("link.{}={:08x},p{}\n", l.pid, l.seed, l.target).len();
    let r = l.area();
    n += format!("area.{}={:08x},{},{},{},{}\n", l.pid, l.seed, r.x, r.y, r.w, r.h).len();
    if let Some(o) = outline {
        n += format!("outline.{}={:08x},{}\n", l.pid, l.seed, o.text()).len();
    }
    n
}

/// The file's size as it would be written for the links now (kept ones only when nb is given).
pub fn doclinks_bytes(dl: &DocLinks) -> usize {
    HEADER.len() + dl.unknown.len() + dl.links.iter().map(|l| doclinks_link_bytes(dl, l)).sum::<usize>()
}

/// The text of the file: every link of a page that is in nb and goes to a page that is (all of them without nb).
pub fn doclinks_text(dl: &DocLinks, nb: Option<&NotebookInfo>) -> Vec<u8> {
    let mut text = String::with_capacity(HEADER.len() + dl.links.len() * 64);
    text.push_str(HEADER);
    for l in &dl.links {
        if nb.is_some_and(|nb| !has_page(nb, l.pid) || !has_page(nb, l.target)) {
            continue;
        }
        let r = l.area();
        text.push_str(&format!("link.{}={:08x},p{}\n", l.pid, l.seed, l.target));
        text.push_str(&format!("area.{}={:08x},{},{},{},{}\n", l.pid, l.seed, r.x, r.y, r.w, r.h));
        if let Some(o) = dl.outline_of(l) {
            text.push_str(&format!("outline.{}={:08x},{}\n", l.pid, l.seed, o.text()));
        }
    }
    let mut bytes = text.into_bytes();
    bytes.extend_from_slice(&dl.unknown);
    bytes
}

/// Writes doclinks.ini (atomic), leaving out the links of pages no longer in nb and those going to such
/// pages (they are dropped from memory after the write, as links.ini's are). -1 when not writable, over a
/// cap (links, bytes, or links on a page) or the write failed: nothing is left out then.
pub fn doclinks_save(id: &str, dl: &mut DocLinks, nb: &NotebookInfo) -> i32 {
    if id.as_bytes() != nb.id.as_bytes() {
        return -1;
    }
    let Some(path) = path_for(id, "doclinks.ini") else { return -1 };
    doclinks_save_to(&path, dl, nb)
}

/// doclinks_save into the notebook folder `dir`, which is not the store's yet (a notebook being built in a work
/// folder, renamed into place once whole).
pub fn doclinks_save_in(dir: &str, dl: &mut DocLinks, nb: &NotebookInfo) -> i32 {
    doclinks_save_to(&format!("{dir}/doclinks.ini"), dl, nb)
}

fn doclinks_save_to(path: &str, dl: &mut DocLinks, nb: &NotebookInfo) -> i32 {
    if !dl.writable || nb.npages() < 0 || nb.npages() > STORE_MAX_PAGES as i32 {
        return -1;
    }
    let kept = |l: &DocLink| has_page(nb, l.pid) && has_page(nb, l.target);
    let n = dl.links.iter().filter(|l| kept(l)).count();
    if !dl.file && n == 0 && dl.unknown.is_empty() {
        return 0;
    }
    if n > DOCLINKS_MAX {
        return -1;
    }
    let mut run = 0usize;
    let mut last = 0u32;
    for l in dl.links.iter().filter(|l| kept(l)) {
        run = if l.pid == last { run + 1 } else { 1 };
        last = l.pid;
        if run > DOCLINKS_PER_PAGE {
            return -1;
        }
    }
    let text = doclinks_text(dl, Some(nb));
    if text.len() > DOCLINKS_FILE_MAX {
        return -1;
    }
    let rc = write_file_atomic(path, &text);
    if rc == 0 {
        dl.file = true;
        dl.retain(|l| kept(l));
    }
    rc
}

// ---- reading a page's links ----

// The area on page p as stored, clamped to the page; None when nothing is left on it.
fn area_on(l: &DocLink, p: &Page) -> Option<Rect> {
    let r = l.area();
    let (x0, y0) = (r.x.clamp(0, p.w.max(0)), r.y.clamp(0, p.h.max(0)));
    let (x1, y1) = ((r.x + r.w).clamp(0, p.w.max(0)), (r.y + r.h).clamp(0, p.h.max(0)));
    (x1 > x0 && y1 > y0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// The exact areas of page pid's document links on page p, in annotation order (what the eye draws).
pub fn doclinks_boxes(dl: &DocLinks, p: &Page, pid: u32) -> Vec<Rect> {
    doclinks_of(dl, pid).iter().filter_map(|l| area_on(l, p)).collect()
}

/// Page pid's document links on page p as (target page id, exact area), annotation order: what the Links bar's
/// "From the PDF" row groups.
pub fn doclinks_targets(dl: &DocLinks, p: &Page, pid: u32) -> Vec<(u32, Rect)> {
    doclinks_of(dl, pid).iter().filter_map(|l| area_on(l, p).map(|r| (l.target, r))).collect()
}

/// A box made at least min_px a side around its centre, rounded outward (nav's padding).
fn grown(r: Rect, min_px: f32) -> Rect {
    let (mut x0, mut y0, mut x1, mut y1) = (r.x as f32, r.y as f32, (r.x + r.w) as f32, (r.y + r.h) as f32);
    let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    if x1 - x0 < min_px {
        x0 = cx - min_px * 0.5;
        x1 = cx + min_px * 0.5;
    }
    if y1 - y0 < min_px {
        y0 = cy - min_px * 0.5;
        y1 = cy + min_px * 0.5;
    }
    let (ix, iy) = (x0.floor() as i32, y0.floor() as i32);
    Rect { x: ix, y: iy, w: x1.ceil() as i32 - ix, h: y1.ceil() as i32 - iy }
}

fn holds(b: Rect, x: f32, y: f32) -> bool {
    x >= b.x as f32 && x < (b.x + b.w) as f32 && y >= b.y as f32 && y < (b.y + b.h) as f32
}

/// The document link a tap at (x, y) on page p (page pid) follows (D9): first the link whose exact area holds
/// the point, the smallest; failing that, the one that reaches it only through the growth to min_px (a
/// finger's 7 mm), the smallest of those. Of equal boxes the one drawn later wins (annotation order). The
/// user's own links come first: the caller asks this only when nav_link_at found none.
pub fn doclinks_at(dl: &DocLinks, p: &Page, pid: u32, x: f32, y: f32, min_px: f32) -> Option<DocLink> {
    let mine = doclinks_of(dl, pid);
    for pass in 0..2 {
        let mut best: Option<(usize, i64)> = None;
        for (i, l) in mine.iter().enumerate() {
            let Some(exact) = area_on(l, p) else { continue };
            let b = if pass == 0 { exact } else { grown(exact, min_px) };
            if !holds(b, x, y) {
                continue;
            }
            if pass == 0 {
                if let Some(o) = dl.outline_of(l).filter(|o| o.n > 0) {
                    if !o.hit(l.area(), b, x, y, min_px) {
                        continue;
                    }
                }
            }
            let size = b.w as i64 * b.h as i64;
            // (<=: of two equal boxes the later annotation)
            if best.is_none_or(|(_, s)| size <= s) {
                best = Some((i, size));
            }
        }
        if let Some((i, _)) = best {
            return Some(mine[i]);
        }
    }
    None
}

// ---- carrying them ----

/// Copy page, inside one notebook: every document link of page `from` onto page `to` (same seeds, targets,
/// areas and outlines; a link to the copied page itself keeps going to the original). Appended after what `to`
/// has. Past the water level (links or bytes) nothing is copied. Returns how many were copied, or -1 when
/// nothing could be (not writable, past the water level, no memory).
pub fn doclinks_copy_page(dl: &mut DocLinks, from: u32, to: u32) -> i32 {
    if !dl.writable || from == to || to == 0 {
        return -1;
    }
    let src: Vec<(DocLink, Option<LinkOutline>)> = doclinks_of(dl, from).iter().map(|l| (*l, dl.outline_of(l).copied())).collect();
    if src.is_empty() {
        return 0;
    }
    let have = doclinks_of(dl, to).len();
    let new_bytes: usize = src.iter().map(|(l, _)| doclinks_link_bytes(dl, &DocLink { pid: to, ..*l })).sum();
    // (the caps themselves: the water level's 10% is kept for exactly this)
    if dl.links.len() + src.len() > DOCLINKS_MAX || have + src.len() > DOCLINKS_PER_PAGE || doclinks_bytes(dl) + new_bytes > DOCLINKS_FILE_MAX {
        return -1;
    }
    let mut n = 0;
    for (l, o) in src {
        if doclinks_has_seed(dl, to, l.seed) {
            continue;
        }
        if !dl.push(DocLink { pid: to, ..l }, o) {
            return -1;
        }
        n += 1;
    }
    n
}

/// Copy to notebook...: the document links of the pages copied (`copies`, from store_copy_pages_to) written
/// into notebook dst's doclinks.ini. A link is carried only when its target was copied in the same operation
/// (a page taken out of a planner loses the tabs that point to pages left behind: a document link never points
/// outside its notebook). The areas scale by sx, sy when the two notebooks' pages differ in size. Returns how
/// many were carried, -1 when dst's doclinks.ini does not read, is past the water level or did not write
/// (the pages are copied without their document links).
pub fn doclinks_copy_across(src: &DocLinks, copies: &[PageCopy], dst: &NotebookInfo, sx: f32, sy: f32) -> i32 {
    let mut carry: Vec<(DocLink, Option<LinkOutline>)> = Vec::new();
    for c in copies {
        for l in doclinks_of(src, c.from) {
            let Some(t) = copies.iter().find(|x| x.from == l.target) else { continue };
            let r = l.area();
            let sc = |v: i32, f: f32| ((v as f32 * f).round() as i32).clamp(0, NAV_AREA_MAX);
            let (x0, y0) = (sc(r.x, sx), sc(r.y, sy));
            let (x1, y1) = (sc(r.x + r.w, sx).max(x0 + 1), sc(r.y + r.h, sy).max(y0 + 1));
            let area = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
            let Some(nl) = DocLink::new(c.to, l.seed, t.to, area) else { continue };
            let outline = src.outline_of(l).copied().map(|mut o| {
                for q in o.xy[..o.n as usize].iter_mut() {
                    q.0 = ((q.0 as f32 * sx).round() as i32).clamp(0, NAV_AREA_MAX) as i16;
                    q.1 = ((q.1 as f32 * sy).round() as i32).clamp(0, NAV_AREA_MAX) as i16;
                }
                o
            });
            carry.push((nl, outline));
        }
    }
    if carry.is_empty() {
        return 0;
    }
    let mut out = doclinks_load(dst.id.as_str());
    if !out.writable {
        return -1;
    }
    // (with their outline lines: the new links have no outline index in `out` yet)
    let add_bytes: usize = carry.iter().map(|(l, o)| link_bytes(l, o.as_ref())).sum();
    if out.links.len() + carry.len() > DOCLINKS_MAX || doclinks_bytes(&out) + add_bytes > DOCLINKS_FILE_MAX {
        return -1;
    }
    let n = carry.len() as i32;
    for (l, o) in carry {
        if doclinks_of(&out, l.pid).len() >= DOCLINKS_PER_PAGE || !out.push(l, o) {
            return -1;
        }
    }
    if doclinks_save(dst.id.as_str(), &mut out, dst) != 0 {
        return -1;
    }
    n
}
