//! weblinks.ini: the web links a document brought (2.2, issue #174). A PDF's link annotation whose action is a
//! URI with an address the rules take (weblinks_address_ok: http or https, printable ASCII, at most 200 bytes) is
//! kept here, with its area on the page; a finger's tap on it shows a card with the address and its QR code, for
//! a phone to open. Nothing on the Kindle opens, fetches or runs an address: the card shows the stored text and a
//! code made from it.
//!
//! The file is `notebooks/<id>/weblinks.ini`, one line a link:
//!
//!   web.<page id>=<seed>,<x>,<y>,<w>,<h>,<address>
//!
//! It is a file of its own, never part of doclinks.ini, so that a version before 2.2 (which never reads or writes
//! it) opens the notebook as before, its document links and its pages untouched, the web links simply absent
//! there; and a 2.1.0 doclinks.ini never carries lines it would count against its 16 KiB of carried lines. Lines
//! this version does not read are carried, as doclinks.ini's are.
//!
//! Who reads it: the editor (taps, the eye, the card), Copy page and Copy to notebook, raw zips. Who never does:
//! the Links bar, backlinks, libindex and the Graph, the Index and Find, the notebook PDF, the lasso and Select,
//! Clear page and the clipboard.
//!
//! Who writes it: the PDF import's last slice (a new notebook's), a fill's commit (the web links of the pages it
//! adds), Copy page and Copy to notebook. Never a page turn, a Clear or a lasso.

use crate::prelude::*;

/// The file's name in the notebook's folder.
pub const WEBLINKS_FILE: &str = "weblinks.ini";
/// links in a file
pub const WEBLINKS_MAX: usize = 4000;
/// links on a page
pub const WEBLINKS_PER_PAGE: usize = 256;
/// the file's size: every line at its longest, the most links and the carried lines all fit (a test holds it)
pub const WEBLINKS_FILE_MAX: usize = 1 << 20;
/// carried lines
pub const WEBLINKS_UNKNOWN_MAX: usize = 16 * 1024;
/// An address's bytes at most: under the 213 bytes the largest code qr.rs draws holds at level M (version 10), so
/// every address kept has its code.
pub const WEBLINK_ADDRESS_MAX: usize = 200;

const HEADER: &str = "# Grimoire: web links a document brought (docs/FORMAT.md, weblinks.ini)\n";

/// Whether an address may be kept as a web link (the worker sends only these, and the writer and the reader check
/// again): `http://` or `https://` (the scheme in either case) followed by a byte that starts a host (not `/`, `?`
/// or `#`); every byte printable ASCII other than a space (0x21 to 0x7E: no control byte, no space, nothing
/// outside ASCII); at most WEBLINK_ADDRESS_MAX bytes in all.
pub fn weblinks_address_ok(b: &[u8]) -> bool {
    if b.len() > WEBLINK_ADDRESS_MAX || !b.iter().all(|&c| (0x21..=0x7e).contains(&c)) {
        return false;
    }
    let rest = if b.len() >= 7 && b[..7].eq_ignore_ascii_case(b"http://") {
        &b[7..]
    } else if b.len() >= 8 && b[..8].eq_ignore_ascii_case(b"https://") {
        &b[8..]
    } else {
        return false;
    };
    rest.first().is_some_and(|c| !matches!(c, b'/' | b'?' | b'#'))
}

/// A URI action's string as the worker takes it: its ends trimmed of spaces, tabs, line breaks and NULs (some
/// producers leave them), then by weblinks_address_ok as it is. None for an address the rules refuse.
pub fn weblinks_address_from(raw: &[u8]) -> Option<String> {
    let pad = |c: &u8| matches!(c, b' ' | b'\t' | b'\r' | b'\n' | 0);
    let s = raw.iter().position(|c| !pad(c)).unwrap_or(raw.len());
    let e = raw.iter().rposition(|c| !pad(c)).map_or(s, |e| e + 1);
    let b = &raw[s..e.max(s)];
    if !weblinks_address_ok(b) {
        return None;
    }
    String::from_utf8(b.to_vec()).ok()
}

/// The host an address names, what the card's title shows: after `://`, up to the first `/`, `?` or `#`, after
/// the last `@` (a `user@` part is not the host). The whole address when that leaves nothing.
pub fn weblinks_host(a: &str) -> &str {
    let Some(i) = a.find("://") else { return a };
    let rest = &a[i + 3..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let auth = &rest[..end];
    let h = auth.rsplit('@').next().unwrap_or(auth);
    if h.is_empty() {
        a
    } else {
        h
    }
}

/// One web link: its page, its seed (it names the link inside this file; ink and the other link files never meet
/// it), its area in page pixels (links.ini's rules) and its address as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebLink {
    pub pid: u32,
    pub seed: u32,
    pub area: Rect,
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct WebLinks {
    /// sorted by page id (stable: inside a page, the document's order)
    pub links: Vec<WebLink>,
    /// the lines this version does not read, carried
    pub unknown: Vec<u8>,
    /// false after a file that did not read: never overwrite it
    pub writable: bool,
    /// weblinks.ini is there (else it is written only once there is a link)
    pub file: bool,
}

impl Default for WebLinks {
    fn default() -> Self {
        WebLinks { links: Vec::new(), unknown: Vec::new(), writable: true, file: false }
    }
}

impl WebLinks {
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
    /// Adds a link at the end of its page's; false for no memory.
    pub fn push(&mut self, l: WebLink) -> bool {
        if self.links.try_reserve(1).is_err() {
            return false;
        }
        let at = self.links.partition_point(|k| k.pid <= l.pid);
        self.links.insert(at, l);
        true
    }
}

/// The web links of page pid, in the document's order.
pub fn weblinks_of(wl: &WebLinks, pid: u32) -> &[WebLink] {
    let a = wl.links.partition_point(|l| l.pid < pid);
    let b = wl.links.partition_point(|l| l.pid <= pid);
    &wl.links[a..b]
}

// ---- reading ----

fn carry_line(buf: &mut Vec<u8>, line: &[u8]) -> bool {
    if line.len() > 255 || buf.len() + line.len() + 1 > WEBLINKS_UNKNOWN_MAX || buf.try_reserve(line.len() + 1).is_err() {
        return false;
    }
    buf.extend_from_slice(line);
    buf.push(b'\n');
    true
}

/// web.<page id>=<seed, 1 to 8 hex digits>,<x>,<y>,<w>,<h>,<address>: the page id and seed as a link line's, the
/// area as an area line's (1 to 4 digits each, not empty, on the page side of 6000), the address the rest of the
/// line, by weblinks_address_ok.
fn web_parse(line: &[u8]) -> Option<WebLink> {
    let rest = line.strip_prefix(b"web.")?;
    let mut key = b"link.".to_vec();
    key.extend_from_slice(rest);
    let (pid, seed, mut t) = link_key(&key)?;
    let mut v = [0i32; 4];
    for n in &mut v {
        let comma = t.iter().position(|&c| c == b',')?;
        let f = &t[..comma];
        if f.is_empty() || f.len() > 4 || !f.iter().all(u8::is_ascii_digit) {
            return None;
        }
        for &c in f {
            *n = *n * 10 + (c - b'0') as i32;
        }
        t = &t[comma + 1..];
    }
    let area = Rect { x: v[0], y: v[1], w: v[2], h: v[3] };
    if !nav_area_valid(area) || !weblinks_address_ok(t) {
        return None;
    }
    Some(WebLink { pid, seed, area, address: String::from_utf8(t.to_vec()).ok()? })
}

/// A weblinks.ini's text read strictly: None when it holds more links than the cap (a page more than its 256
/// too), a carried line past its bounds, or runs out of memory (a file that does not read is never written over).
/// A line this version cannot read (another version's, or a `web.` line whose area or address it refuses) is
/// carried as it is; the last line for a page id and seed wins.
pub fn weblinks_parse(text: &[u8]) -> Option<WebLinks> {
    let mut wl = WebLinks::default();
    let mut at: std::collections::HashMap<(u32, u32), usize> = std::collections::HashMap::new();
    for raw in text.split(|&c| c == b'\n') {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        let Some(l) = web_parse(line) else {
            if !carry_line(&mut wl.unknown, line) {
                return None;
            }
            continue;
        };
        if let Some(&k) = at.get(&(l.pid, l.seed)) {
            wl.links[k] = l;
            continue;
        }
        if wl.links.len() >= WEBLINKS_MAX || wl.links.try_reserve(1).is_err() {
            return None;
        }
        at.insert((l.pid, l.seed), wl.links.len());
        wl.links.push(l);
    }
    // (the file's order inside a page is the document's; sorting by page keeps it)
    wl.links.sort_by_key(|l| l.pid);
    let (mut run, mut last) = (0usize, 0u32);
    for l in &wl.links {
        run = if l.pid == last { run + 1 } else { 1 };
        last = l.pid;
        if run > WEBLINKS_PER_PAGE {
            return None;
        }
    }
    Some(wl)
}

fn unreadable() -> WebLinks {
    WebLinks { writable: false, file: true, ..Default::default() }
}

/// Notebook id's weblinks.ini. Missing is an empty writable set. One that is there and does not read (no plain
/// file at its name, over the cap, a carried line too long, more links than the cap) reads as none and refuses
/// writes, so it is never written over.
pub fn weblinks_load(id: &str) -> WebLinks {
    let Some(path) = path_for(id, WEBLINKS_FILE) else { return unreadable() };
    match read_sidecar(&path, WEBLINKS_FILE_MAX) {
        Ok(None) => WebLinks::default(),
        Ok(Some(text)) => match weblinks_parse(&text) {
            Some(mut wl) => {
                wl.file = true;
                wl
            }
            None => {
                qlog!("weblinks: weblinks.ini could not be read; left unchanged");
                unreadable()
            }
        },
        Err(()) => {
            qlog!("weblinks: weblinks.ini could not be read; left unchanged");
            unreadable()
        }
    }
}

/// Drops the links of pages no longer in nb, from memory (the file loses them at its next write). Returns how
/// many went.
pub fn weblinks_prune(wl: &mut WebLinks, nb: &NotebookInfo) -> usize {
    let before = wl.links.len();
    wl.links.retain(|l| has_page(nb, l.pid));
    before - wl.links.len()
}

// ---- writing ----

fn line_of(l: &WebLink) -> String {
    let r = l.area;
    format!("web.{}={:08x},{},{},{},{},{}\n", l.pid, l.seed, r.x, r.y, r.w, r.h, l.address)
}

/// The text of the file: every link of a page that is in nb (all of them without nb), then the carried lines.
pub fn weblinks_text(wl: &WebLinks, nb: Option<&NotebookInfo>) -> Vec<u8> {
    let mut text = String::with_capacity(HEADER.len() + wl.links.len() * 64);
    text.push_str(HEADER);
    for l in &wl.links {
        if nb.is_some_and(|nb| !has_page(nb, l.pid)) {
            continue;
        }
        text.push_str(&line_of(l));
    }
    let mut bytes = text.into_bytes();
    bytes.extend_from_slice(&wl.unknown);
    bytes
}

/// Writes weblinks.ini (atomic), leaving out the links of pages no longer in nb (dropped from memory after the
/// write). Nothing is written while there is no file and no link (a notebook without web links never has one).
/// -1 when not writable, over a cap (links, links on a page, bytes) or the write failed.
pub fn weblinks_save(id: &str, wl: &mut WebLinks, nb: &NotebookInfo) -> i32 {
    if id.as_bytes() != nb.id.as_bytes() {
        return -1;
    }
    let Some(path) = path_for(id, WEBLINKS_FILE) else { return -1 };
    save_to(&path, wl, nb)
}

/// weblinks_save into the notebook folder `dir`, which is not the store's yet (a notebook being built in a work
/// folder, renamed into place once whole), or is the store's own (a fill's commit).
pub fn weblinks_save_in(dir: &str, wl: &mut WebLinks, nb: &NotebookInfo) -> i32 {
    save_to(&format!("{dir}/{WEBLINKS_FILE}"), wl, nb)
}

fn save_to(path: &str, wl: &mut WebLinks, nb: &NotebookInfo) -> i32 {
    if !wl.writable || nb.npages() < 0 || nb.npages() > STORE_MAX_PAGES as i32 {
        return -1;
    }
    let kept = |l: &WebLink| has_page(nb, l.pid);
    let n = wl.links.iter().filter(|l| kept(l)).count();
    if !wl.file && n == 0 && wl.unknown.is_empty() {
        return 0;
    }
    if n > WEBLINKS_MAX {
        return -1;
    }
    let (mut run, mut last) = (0usize, 0u32);
    for l in wl.links.iter().filter(|l| kept(l)) {
        run = if l.pid == last { run + 1 } else { 1 };
        last = l.pid;
        if run > WEBLINKS_PER_PAGE {
            return -1;
        }
    }
    let text = weblinks_text(wl, Some(nb));
    if text.len() > WEBLINKS_FILE_MAX {
        return -1;
    }
    let rc = write_file_atomic(path, &text);
    if rc == 0 {
        wl.file = true;
        wl.links.retain(kept);
    }
    rc
}

// ---- a page's links ----

// The area as stored, clamped to page p; None when nothing is left on it.
fn area_on(r: Rect, p: &Page) -> Option<Rect> {
    let (x0, y0) = (r.x.clamp(0, p.w.max(0)), r.y.clamp(0, p.h.max(0)));
    let (x1, y1) = ((r.x + r.w).clamp(0, p.w.max(0)), (r.y + r.h).clamp(0, p.h.max(0)));
    (x1 > x0 && y1 > y0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// Link l's exact area on page p (clamped to it); None when nothing of it is on the page.
pub fn weblinks_box(l: &WebLink, p: &Page) -> Option<Rect> {
    area_on(l.area, p)
}

/// The exact areas of page pid's web links on page p, in order (what the eye draws, with the document links').
pub fn weblinks_boxes(wl: &WebLinks, p: &Page, pid: u32) -> Vec<Rect> {
    weblinks_of(wl, pid).iter().filter_map(|l| area_on(l.area, p)).collect()
}

/// The web link a tap at (x, y) on page p (page pid) opens, as doclinks_at picks a document link: first the link
/// whose exact area holds the point, the smallest; failing that, the one that reaches it only through the growth
/// to min_px (a finger's 7 mm), the smallest of those; of equal boxes the later one. With it, whether its exact
/// area holds the point (a document link that reaches the point only through its growth gives way to it).
pub fn weblinks_at(wl: &WebLinks, p: &Page, pid: u32, x: f32, y: f32, min_px: f32) -> Option<(WebLink, bool)> {
    let mine = weblinks_of(wl, pid);
    for pass in 0..2 {
        let mut best: Option<(usize, i64)> = None;
        for (i, l) in mine.iter().enumerate() {
            let Some(exact) = area_on(l.area, p) else { continue };
            let b = if pass == 0 { exact } else { crate::doclinks::grown(exact, min_px) };
            if !crate::doclinks::holds(b, x, y) {
                continue;
            }
            let size = b.w as i64 * b.h as i64;
            if best.is_none_or(|(_, s)| size <= s) {
                best = Some((i, size));
            }
        }
        if let Some((i, _)) = best {
            return Some((mine[i].clone(), pass == 0));
        }
    }
    None
}

// ---- filling it (the PDF import's writer) ----

/// A web link a document brought, in the notebook's terms: its page id, its rectangle in page pixels, its place
/// among the page's web links (the order the document drew them) and its address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebRecord {
    pub src: u32,
    pub rect: Rect,
    pub index: u32,
    pub address: String,
}

/// The seed wanted for web link `index` of page `src`: a hash of both, so a fill finds the same seeds (apart from
/// the document links' by the top bit of the index).
fn web_seed(src: u32, index: u32) -> u32 {
    pdflinks_seed(src, index | 0x8000_0000)
}

/// Puts records into wl (the notebook's page size pw x ph; `exists` says whether a page id is one of the
/// notebook's). Validation: the page in the notebook, the address by weblinks_address_ok, the area clamped to the
/// page and at least 1 px a side. In the document's order inside a page, after what the page has. Idempotent: a
/// record whose address is a link's of the page and whose area is within 2 px of it is that link (`same`). A page
/// takes WEBLINKS_PER_PAGE; the rest are `full`. Call weblinks_water after, then save.
pub fn weblinks_fill(wl: &mut WebLinks, recs: &[WebRecord], pw: i32, ph: i32, exists: &dyn Fn(u32) -> bool) -> FillStats {
    let mut st = FillStats::default();
    let mut order: Vec<usize> = (0..recs.len()).collect();
    order.sort_by_key(|&i| (recs[i].src, recs[i].index, i));
    for i in order {
        let rec = &recs[i];
        let area = if rec.src != 0 && exists(rec.src) && weblinks_address_ok(rec.address.as_bytes()) { page_area(rec.rect, pw, ph) } else { None };
        let Some(area) = area else {
            st.invalid += 1;
            continue;
        };
        let mine = weblinks_of(wl, rec.src);
        if mine.iter().any(|l| l.address == rec.address && same_area(l.area, area)) {
            st.same += 1;
            continue;
        }
        if mine.len() >= WEBLINKS_PER_PAGE {
            st.full += 1;
            continue;
        }
        let mut seed = web_seed(rec.src, rec.index);
        for _ in 0..64 {
            if !mine.iter().any(|l| l.seed == seed) {
                break;
            }
            seed = seed.wrapping_mul(2654435761).wrapping_add(0x9e3779b9);
        }
        if mine.iter().any(|l| l.seed == seed) || !wl.push(WebLink { pid: rec.src, seed, area, address: rec.address.clone() }) {
            st.invalid += 1;
            continue;
        }
        st.added += 1;
    }
    st
}

/// The water level (as doclinks.ini's, counted in links: the bytes never bind, WEBLINKS_FILE_MAX holds the most
/// links at their longest): trims wl to at most 90% of WEBLINKS_MAX, the densest pages first, each page keeping
/// its first links. The links `fixed` says yes to (the ones a notebook had before a fill) are never trimmed and
/// count as they are. Returns how many went.
pub fn weblinks_water(wl: &mut WebLinks, fixed: &dyn Fn(&WebLink) -> bool) -> usize {
    let max_n = WEBLINKS_MAX * DOCLINKS_WATER_PCT / 100;
    if wl.links.len() <= max_n {
        return 0;
    }
    // rank of each link among its page's links that may go (0 for a link held)
    let mut rank = Vec::with_capacity(wl.links.len());
    let (mut last, mut run, mut held) = (0u32, 0usize, 0usize);
    for l in &wl.links {
        if l.pid != last {
            run = 0;
            last = l.pid;
        }
        if fixed(l) {
            rank.push(0);
            held += 1;
            continue;
        }
        run += 1;
        rank.push(run);
    }
    let level = |lv: usize| held + rank.iter().filter(|&&r| r != 0 && r <= lv).count();
    // the largest per-page count that fits
    let (mut lo, mut hi) = (0usize, rank.iter().copied().max().unwrap_or(0));
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if level(mid) <= max_n {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    // the pages above the level take one more each, by page, while the room lasts
    let mut n = level(lo);
    let keep: Vec<bool> = rank
        .iter()
        .map(|&r| {
            if r <= lo {
                return true;
            }
            if r == lo + 1 && n < max_n {
                n += 1;
                return true;
            }
            false
        })
        .collect();
    let removed = keep.iter().filter(|&&k| !k).count();
    let mut k = 0;
    wl.links.retain(|_| {
        k += 1;
        keep[k - 1]
    });
    removed
}

// ---- carrying them ----

/// Copy page, inside one notebook: every web link of page `from` onto page `to` (same seeds, areas and
/// addresses), after what `to` has. Returns how many were copied, or -1 when nothing could be (not writable, the
/// caps, no memory).
pub fn weblinks_copy_page(wl: &mut WebLinks, from: u32, to: u32) -> i32 {
    if !wl.writable || from == to || to == 0 {
        return -1;
    }
    let src: Vec<WebLink> = weblinks_of(wl, from).to_vec();
    if src.is_empty() {
        return 0;
    }
    let have = weblinks_of(wl, to).len();
    if wl.links.len() + src.len() > WEBLINKS_MAX || have + src.len() > WEBLINKS_PER_PAGE {
        return -1;
    }
    let mut n = 0;
    for l in src {
        if weblinks_of(wl, to).iter().any(|k| k.seed == l.seed) {
            continue;
        }
        if !wl.push(WebLink { pid: to, ..l }) {
            return -1;
        }
        n += 1;
    }
    n
}

/// Copy to notebook...: the web links of the pages copied (`copies`, from store_copy_pages_to) written into
/// notebook dst's weblinks.ini, the areas scaled by sx, sy when the two notebooks' pages differ in size. Returns how
/// many were carried, -1 when dst's weblinks.ini does not read, is past a cap or did not write (the pages are
/// copied without them).
pub fn weblinks_copy_across(src: &WebLinks, copies: &[PageCopy], dst: &NotebookInfo, sx: f32, sy: f32) -> i32 {
    let mut carry: Vec<WebLink> = Vec::new();
    for c in copies {
        for l in weblinks_of(src, c.from) {
            let r = l.area;
            let sc = |v: i32, f: f32| ((v as f32 * f).round() as i32).clamp(0, NAV_AREA_MAX);
            let (x0, y0) = (sc(r.x, sx), sc(r.y, sy));
            let (x1, y1) = (sc(r.x + r.w, sx).max(x0 + 1), sc(r.y + r.h, sy).max(y0 + 1));
            let area = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
            if c.to == 0 || !nav_area_valid(area) {
                continue;
            }
            if carry.try_reserve(1).is_err() {
                return -1;
            }
            carry.push(WebLink { pid: c.to, seed: l.seed, area, address: l.address.clone() });
        }
    }
    if carry.is_empty() {
        return 0;
    }
    let mut out = weblinks_load(dst.id.as_str());
    if !out.writable || out.links.len() + carry.len() > WEBLINKS_MAX {
        return -1;
    }
    let n = carry.len() as i32;
    for l in carry {
        if weblinks_of(&out, l.pid).len() >= WEBLINKS_PER_PAGE || !out.push(l) {
            return -1;
        }
    }
    if weblinks_save(dst.id.as_str(), &mut out, dst) != 0 {
        return -1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(pid: u32, seed: u32, x: i32, addr: &str) -> WebLink {
        WebLink { pid, seed, area: Rect::new(x, 10, 40, 20), address: addr.to_string() }
    }

    #[test]
    fn only_http_and_https_addresses_of_printable_ascii_are_kept() {
        for ok in ["http://example.org", "https://example.org/a?b=c#d", "HTTPS://Example.ORG", "http://u@h", "https://x"] {
            assert!(weblinks_address_ok(ok.as_bytes()), "{ok}");
        }
        let long_ok = format!("https://e.org/{}", "a".repeat(WEBLINK_ADDRESS_MAX - 14));
        assert_eq!(long_ok.len(), WEBLINK_ADDRESS_MAX);
        assert!(weblinks_address_ok(long_ok.as_bytes()));
        let too_long = format!("{long_ok}a");
        for bad in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "mailto:a@b.c",
            "ftp://example.org",
            "data:text/html,x",
            "http:/example.org",
            "http://",
            "https://",
            "http:///etc",
            "https://?x",
            "https://#x",
            "//example.org",
            "example.org",
            "http://exa mple.org",
            "http://example.org/\u{7f}",
            "http://example.org/\t",
            "http://example.org/\n",
            "http://ex\u{e9}mple.org",
            "",
            too_long.as_str(),
        ] {
            assert!(!weblinks_address_ok(bad.as_bytes()), "{bad:?}");
        }
        assert!(!weblinks_address_ok(b"http://a\x00b"));
        assert!(!weblinks_address_ok(b"http://a\x1bb"));
    }

    #[test]
    fn the_worker_trims_an_addresss_ends_and_nothing_inside() {
        assert_eq!(weblinks_address_from(b"  https://example.org/x\r\n\x00").as_deref(), Some("https://example.org/x"));
        assert_eq!(weblinks_address_from(b"\x00\x00"), None);
        assert_eq!(weblinks_address_from(b"https://exa mple.org"), None);
        assert_eq!(weblinks_address_from(b" javascript:x "), None);
        assert_eq!(weblinks_address_from(b"https://e.org/\x00x"), None);
    }

    #[test]
    fn the_title_is_the_host() {
        assert_eq!(weblinks_host("https://example.org/a/b?c"), "example.org");
        assert_eq!(weblinks_host("http://user:pw@shop.example:8080/x"), "shop.example:8080");
        assert_eq!(weblinks_host("https://google.com@evil.example/"), "evil.example");
        assert_eq!(weblinks_host("https://h?q"), "h");
        assert_eq!(weblinks_host("https://@/"), "https://@/");
    }

    #[test]
    fn a_file_reads_back_as_written_and_carries_what_it_does_not_read() {
        let mut wl = WebLinks::default();
        assert!(wl.push(link(3, 0xd00d, 5, "https://b.example/2")));
        assert!(wl.push(link(1, 7, 5, "https://a.example/1")));
        assert!(wl.push(link(3, 0x1234_5678, 50, "http://c.example/,x,y")));
        wl.unknown = b"gopher.1=whatever\n".to_vec();
        let text = weblinks_text(&wl, None);
        let s = String::from_utf8(text.clone()).unwrap();
        assert!(s.starts_with(HEADER));
        assert!(s.contains("web.1=00000007,5,10,40,20,https://a.example/1\n"));
        // (an address may hold commas: it is the rest of the line)
        assert!(s.contains("web.3=12345678,50,10,40,20,http://c.example/,x,y\n"));
        let back = weblinks_parse(&text).unwrap();
        assert_eq!(back.links, wl.links);
        assert_eq!(back.unknown, wl.unknown);
        assert_eq!(weblinks_text(&back, None), text);
        // a web line this version refuses (an address of another kind, an area past the page, a seed of nine
        // digits) is carried as it is, and the last line for a link wins
        let text = b"web.2=1,0,0,10,10,javascript:alert(1)\nweb.2=2,5990,0,20,10,https://x.example\nweb.2=123456789,0,0,1,1,https://y\nweb.2=3,0,0,9,9,https://old\nweb.2=3,1,1,9,9,https://new\n";
        let wl = weblinks_parse(text).unwrap();
        assert_eq!(wl.links.len(), 1);
        assert_eq!(wl.links[0].address, "https://new");
        assert_eq!(wl.links[0].area, Rect::new(1, 1, 9, 9));
        assert_eq!(wl.unknown.split(|&c| c == b'\n').filter(|l| !l.is_empty()).count(), 3);
    }

    #[test]
    fn a_file_past_a_cap_does_not_read() {
        // the links of a notebook
        let mut t = String::new();
        for k in 0..=WEBLINKS_MAX {
            t.push_str(&format!("web.{}={:08x},0,0,10,10,https://e.org\n", 1 + k / 200, k));
        }
        assert!(weblinks_parse(t.as_bytes()).is_none());
        // a page's
        let mut t = String::new();
        for k in 0..=WEBLINKS_PER_PAGE {
            t.push_str(&format!("web.1={k:08x},0,0,10,10,https://e.org\n"));
        }
        assert!(weblinks_parse(t.as_bytes()).is_none());
        // a carried line over 255 bytes, and carried lines over 16 KiB
        assert!(weblinks_parse(format!("x.1={}\n", "a".repeat(260)).as_bytes()).is_none());
        let many = (0..200).map(|k| format!("future.{k}={}\n", "b".repeat(90))).collect::<String>();
        assert!(weblinks_parse(many.as_bytes()).is_none());
    }

    #[test]
    fn the_most_links_at_their_longest_fit_the_file() {
        let addr = format!("https://e.org/{}", "a".repeat(WEBLINK_ADDRESS_MAX - 14));
        let l = WebLink { pid: u32::MAX, seed: u32::MAX, area: Rect::new(1000, 1000, 5000, 5000), address: addr };
        let longest = line_of(&l).len();
        assert!(longest <= 255, "a web line fits a carried line of an older reader of this file: {longest}");
        assert!(HEADER.len() + WEBLINKS_MAX * longest + WEBLINKS_UNKNOWN_MAX <= WEBLINKS_FILE_MAX);
        assert!(weblinks_address_ok(l.address.as_bytes()) && nav_area_valid(Rect::new(1000, 1000, 5000, 5000)));
    }

    #[test]
    fn a_fill_validates_keeps_one_of_each_and_caps_a_page() {
        let exists = |p: u32| (1..=3).contains(&p);
        let rec = |src: u32, x: i32, i: u32, a: &str| WebRecord { src, rect: Rect::new(x, 10, 40, 20), index: i, address: a.to_string() };
        let recs = vec![
            rec(1, 10, 0, "https://a.example"),
            rec(1, 100, 1, "https://b.example"),
            rec(9, 10, 0, "https://gone.example"),     // no such page
            rec(2, 10, 0, "javascript:alert(1)"),      // refused address
            rec(2, -500, 1, "https://off.example"),    // off the page
            rec(2, 590, 2, "https://edge.example"),    // clamped to the page's 10 px
        ];
        let mut wl = WebLinks::default();
        let st = weblinks_fill(&mut wl, &recs, 600, 800, &exists);
        assert_eq!((st.added, st.same, st.invalid, st.full), (3, 0, 3, 0));
        assert_eq!(weblinks_of(&wl, 2)[0].area, Rect::new(590, 10, 10, 20));
        // again, the areas a pixel off: nothing added
        let again: Vec<WebRecord> = recs.iter().map(|r| WebRecord { rect: Rect { x: r.rect.x + 1, ..r.rect }, ..r.clone() }).collect();
        let st = weblinks_fill(&mut wl, &again, 600, 800, &exists);
        assert_eq!((st.added, st.same), (0, 3));
        // the same area with another address is another link
        let st = weblinks_fill(&mut wl, &[rec(1, 10, 5, "https://c.example")], 600, 800, &exists);
        assert_eq!(st.added, 1);
        let seeds: std::collections::HashSet<u32> = weblinks_of(&wl, 1).iter().map(|l| l.seed).collect();
        assert_eq!(seeds.len(), 3, "a page's seeds are its own");
        // a page takes 256
        let many: Vec<WebRecord> = (0..300).map(|k| rec(3, k % 500, k as u32, &format!("https://e.org/{k}"))).collect();
        let st = weblinks_fill(&mut wl, &many, 600, 800, &exists);
        assert_eq!((st.added, st.full), (WEBLINKS_PER_PAGE, 300 - WEBLINKS_PER_PAGE));
        assert_eq!(weblinks_of(&wl, 3)[0].address, "https://e.org/0", "the document's order");
    }

    #[test]
    fn the_water_level_trims_the_densest_pages_and_keeps_what_a_notebook_had() {
        let max_n = WEBLINKS_MAX * DOCLINKS_WATER_PCT / 100;
        let mut wl = WebLinks::default();
        // a page of 3,000 and twenty pages of 40: 3,800, over the level
        for k in 0..3000u32 {
            assert!(wl.push(link(1, k, 0, "https://a.example")));
        }
        for p in 2..=21u32 {
            for k in 0..40u32 {
                assert!(wl.push(link(p, k, 0, "https://b.example")));
            }
        }
        let gone = weblinks_water(&mut wl, &|_| false);
        assert_eq!(wl.links.len(), max_n);
        assert_eq!(gone, 3800 - max_n);
        assert!((2..=21).all(|p| weblinks_of(&wl, p).len() == 40), "the small pages keep theirs");
        assert_eq!(weblinks_of(&wl, 1).len(), max_n - 800);
        assert_eq!(weblinks_of(&wl, 1).last().unwrap().seed, (max_n - 800 - 1) as u32, "a page keeps its first");
        // a fill: what the notebook had stays whatever the level
        let mut wl = WebLinks::default();
        for k in 0..3700u32 {
            assert!(wl.push(link(1 + k / 250, k, 0, "https://had.example")));
        }
        for k in 0..100u32 {
            assert!(wl.push(link(100, k, 0, "https://new.example")));
        }
        let gone = weblinks_water(&mut wl, &|l| l.address == "https://had.example");
        assert_eq!(gone, 100);
        assert!(weblinks_of(&wl, 100).is_empty());
        assert_eq!(wl.links.len(), 3700);
    }

    #[test]
    fn copy_page_copies_a_pages_links_within_the_caps() {
        let mut wl = WebLinks::default();
        assert!(wl.push(link(1, 1, 0, "https://a.example")));
        assert!(wl.push(link(1, 2, 50, "https://b.example")));
        assert!(wl.push(link(2, 9, 0, "https://c.example")));
        assert_eq!(weblinks_copy_page(&mut wl, 1, 5), 2);
        assert_eq!(weblinks_of(&wl, 5).iter().map(|l| (l.seed, l.address.as_str())).collect::<Vec<_>>(), vec![(1, "https://a.example"), (2, "https://b.example")]);
        assert_eq!(weblinks_copy_page(&mut wl, 3, 6), 0);
        assert_eq!(weblinks_copy_page(&mut wl, 1, 1), -1);
        wl.writable = false;
        assert_eq!(weblinks_copy_page(&mut wl, 2, 7), -1);
    }

    #[test]
    fn a_tap_finds_the_exact_area_first_then_the_grown_one() {
        let mut p = Page::default();
        p.w = 600;
        p.h = 800;
        let mut wl = WebLinks::default();
        assert!(wl.push(WebLink { pid: 1, seed: 1, area: Rect::new(100, 100, 200, 100), address: "https://big.example".into() }));
        assert!(wl.push(WebLink { pid: 1, seed: 2, area: Rect::new(150, 120, 10, 10), address: "https://small.example".into() }));
        let at = |x: f32, y: f32| weblinks_at(&wl, &p, 1, x, y, 80.0).map(|(l, exact)| (l.address, exact));
        assert_eq!(at(155.0, 125.0), Some(("https://small.example".into(), true)));
        assert_eq!(at(250.0, 150.0), Some(("https://big.example".into(), true)));
        assert_eq!(at(320.0, 150.0), None);
        // below both: only the small one's growth to 80 px reaches it
        assert_eq!(at(155.0, 190.0).map(|x| x.0), Some("https://big.example".into()));
        assert_eq!(at(155.0, 205.0), None);
        assert_eq!(at(100.0, 50.0), None);
        // above the big one: the small one's growth reaches it, not exactly
        assert_eq!(at(155.0, 95.0), Some(("https://small.example".into(), false)));
        assert!(weblinks_at(&wl, &p, 2, 155.0, 125.0, 80.0).is_none());
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("weblinks.rs").contains('\u{2014}'));
    }
}
