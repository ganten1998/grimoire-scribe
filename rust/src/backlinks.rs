//! backlinks: which pages link to a page from other notebooks (Rust only, 2.0.0-beta.2). A notebook's
//! links.ini says where its own links go; what links INTO a notebook is spread over every other
//! notebook's links.ini. This module keeps that turned around, in root/backlinks.ini:
//!
//! ```text
//! # Grimoire: links between notebooks (a cache: deleting it loses nothing)
//! src=A,9f3c22d05b1e4471
//! to=1,B,2
//! to=3,B,2
//! src=C,0
//! ```
//!
//! `src=<notebook id>,<stamp>` names a notebook and the stamp of its links.ini when it was read (16 hex
//! digits: the file's identity, size and times hashed); each `to=<page id>,<notebook id>,<page id>` line
//! after it is one of that notebook's pages with a link to a page of another notebook, once per pair of
//! pages. It is a cache, never data: deleting it, or a file that does not read, loses nothing, and a
//! version that does not know it never reads it.
//!
//! It is kept current two ways. A links.ini this version writes (nav_save_links) updates its notebook's
//! lines in memory at once (backlinks_note_saved), with the stamp of the file just written. And on the
//! editor's idle ticks (backlinks_step) every notebook's links.ini is checked against its stamp, a few a
//! tick, and one that changed (another version, a copy, a notebook deleted or brought back) is read again,
//! one a tick: the editor is never held up for more than one links.ini (512 KiB at most). A notebook not
//! there any more loses its lines once a check of every notebook ends. The file is written after such a
//! check, or as a notebook closes, only when what it holds changed. The file itself is read on the first
//! idle tick: a query (a draw) only ever sees what is in memory. A notebook left out at BL_LINKS_MAX is
//! kept as `src=<id>,0` with no lines, read again at every check.
//!
//! It also keeps, between reads, other notebooks' notebook.ini and page names (backlinks_book): what the
//! Links bar's Linked from rows and the Graph's anchors label their pages with, read again only when one
//! of those two files changed (never their links.ini).
//!
//! The module is pure apart from its files: no screen, no clock. Its state is per thread (the store's
//! root is), and starts again when the root changes (the tests' roots).

use crate::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// backlinks.ini is read up to this size; a bigger one is not read (and rebuilt).
pub const BL_FILE_MAX: usize = 2 * 1024 * 1024;
/// The most links between notebooks the cache keeps, all notebooks together: ten notebooks' worth of
/// NAV_LINKS_MAX. A notebook whose links would go past it is left out (backlinks_partial).
pub const BL_LINKS_MAX: usize = 10 * NAV_LINKS_MAX as usize;
/// A tick checks this many notebooks' stamps, and reads at most one links.ini.
pub const BL_STATS_PER_STEP: usize = 8;
/// The most other notebooks whose page names are kept between reads (backlinks_book).
pub const BL_BOOKS_MAX: usize = 16;

const HEADER: &[u8] = b"# Grimoire: links between notebooks (a cache: deleting it loses nothing)\n";

/// A link from page `src` of a notebook to page `pid` of notebook `to`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlLink {
    pub src: u32,
    pub to: FixedStr<STORE_ID_LEN>,
    pub pid: u32,
}

/// One notebook's links to other notebooks, as its links.ini had them at `stamp`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlSource {
    pub id: FixedStr<STORE_ID_LEN>,
    pub stamp: u64,
    /// sorted, one per pair of pages
    pub links: Vec<BlLink>,
}

/// A page that links to a page (backlinks_to): in notebook `id` (this one's own pages too), page `pid`,
/// with `count` links to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlFrom {
    pub id: FixedStr<STORE_ID_LEN>,
    pub pid: u32,
    pub count: i32,
}

/// Another notebook as its pages are labelled: its notebook.ini and its page names (nav.ini alone, never
/// its links.ini).
#[derive(Debug, Default)]
pub struct BlBook {
    pub info: NotebookInfo,
    pub names: Nav,
}

struct BookSlot {
    id: FixedStr<STORE_ID_LEN>,
    /// notebook.ini's and nav.ini's stamps when they were read
    stamps: (u64, u64),
    book: Option<Rc<BlBook>>,
    used: u64,
}

#[derive(Default)]
struct Scan {
    ids: Vec<FixedStr<STORE_ID_LEN>>,
    at: usize,
}

#[derive(Default)]
struct Backlinks {
    root: String,
    loaded: bool,
    /// sorted by id
    sources: Vec<BlSource>,
    /// what is in memory is not what the file holds
    dirty: bool,
    scan: Option<Scan>,
    /// every notebook was checked since the last backlinks_revalidate
    complete: bool,
    /// room was made where a notebook is left out: the check goes round again
    retry: bool,
    /// changes whenever a notebook's links change: what shows them knows to build again
    generation: u32,
    /// other notebooks' names (backlinks_book), the most recently used last read
    books: Vec<BookSlot>,
    clock: u64,
    /// how many times backlinks_book read a notebook's files (a probe of the work it saves)
    books_read: u32,
}

thread_local! {
    static BL: RefCell<Backlinks> = RefCell::new(Backlinks::default());
}

fn path() -> String {
    format!("{}/backlinks.ini", store_root())
}

// The state for the store's root now: started again when the root changed.
fn with<R>(f: impl FnOnce(&mut Backlinks) -> R) -> R {
    BL.with(|b| {
        let mut b = b.borrow_mut();
        let root = store_root();
        if b.root != root {
            *b = Backlinks { root, ..Backlinks::default() };
        }
        f(&mut b)
    })
}

/// links.ini's stamp now: 0 when there is none, else a number that changes whenever it is written
/// again (a write is a new file renamed into place: its inode, size and times).
pub fn backlinks_stamp(id: &str) -> u64 {
    file_stamp(id, "links.ini")
}

/// The stamp of file `file` in notebook id's folder (0 when it is not there): what backlinks.ini and the
/// Graph of all notebooks' index (libindex) tell a changed file by.
pub fn backlinks_file_stamp(id: &str, file: &str) -> u64 {
    file_stamp(id, file)
}

fn file_stamp(id: &str, file: &str) -> u64 {
    use std::os::unix::fs::MetadataExt;
    let mut dir = String::new();
    if store_notebook_dir(id, &mut dir, 600) != 0 {
        return 0;
    }
    let Ok(st) = std::fs::symlink_metadata(format!("{dir}/{file}")) else { return 0 };
    let v = [st.dev(), st.ino(), st.size(), st.mtime() as u64, st.mtime_nsec() as u64, st.ctime() as u64];
    let mut h: u64 = 1469598103934665603; // FNV-1a
    for x in v {
        for b in (0..64).step_by(8) {
            h = (h ^ ((x >> b) & 0xFF)).wrapping_mul(1099511628211);
        }
    }
    h | 1
}

/// A notebook's links to pages of other notebooks: one per pair of pages, sorted. Links from pages
/// not in `page_ids` (when given) are left out, as nav_save_links leaves them out.
pub fn backlinks_of_links(id: &str, links: &[NavLink], page_ids: Option<&[u32]>) -> Vec<BlLink> {
    let mut out: Vec<BlLink> = Vec::new();
    for l in links {
        let Some((to, pid)) = nav_external_target(l.target.as_str()) else { continue };
        if to == id || page_ids.is_some_and(|p| !p.contains(&l.pid)) {
            continue;
        }
        if out.try_reserve(1).is_err() {
            break;
        }
        out.push(BlLink { src: l.pid, to: FixedStr::from_str(to), pid });
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn total(b: &Backlinks) -> usize {
    b.sources.iter().map(|s| s.links.len()).sum()
}

// Puts notebook id's links at stamp into the cache (none and no stamp: out of it). Marks it dirty and
// moves the generation on when anything changed. Past BL_LINKS_MAX the notebook is left out: kept with
// no links and stamp 0, "not read", so every check tries it again (backlinks_partial while one is).
fn put(b: &mut Backlinks, id: &str, stamp: u64, links: Vec<BlLink>) {
    let before = total(b);
    put_in(b, id, stamp, links);
    // Room made where a notebook was left out: a check soon, to take it in.
    if total(b) < before && b.sources.iter().any(|s| s.stamp == 0) {
        b.complete = false;
        b.retry = true;
    }
}

fn put_in(b: &mut Backlinks, id: &str, stamp: u64, links: Vec<BlLink>) {
    let at = b.sources.binary_search_by(|s| s.id.as_bytes().cmp(id.as_bytes()));
    if stamp == 0 && links.is_empty() {
        if let Ok(i) = at {
            let had = !b.sources[i].links.is_empty();
            b.sources.remove(i);
            b.dirty = true;
            if had {
                b.generation = b.generation.wrapping_add(1);
            }
        }
        return;
    }
    let others = total(b) - at.map_or(0, |i| b.sources[i].links.len());
    let (stamp, links) = if others + links.len() > BL_LINKS_MAX { (0, Vec::new()) } else { (stamp, links) };
    match at {
        Ok(i) => {
            let s = &mut b.sources[i];
            if s.links != links {
                s.links = links;
                b.generation = b.generation.wrapping_add(1);
                b.dirty = true;
            }
            if s.stamp != stamp {
                s.stamp = stamp;
                b.dirty = true;
            }
        }
        Err(i) => {
            if b.sources.try_reserve(1).is_err() {
                return;
            }
            if !links.is_empty() {
                b.generation = b.generation.wrapping_add(1);
            }
            b.sources.insert(i, BlSource { id: FixedStr::from_str(id), stamp, links });
            b.dirty = true;
        }
    }
}

fn hex16(v: &[u8]) -> Option<u64> {
    if v.is_empty() || v.len() > 16 {
        return None;
    }
    let mut x: u64 = 0;
    for &c in v {
        x = x << 4 | (c as char).to_digit(16)? as u64;
    }
    Some(x)
}

fn dec(v: &[u8]) -> Option<u32> {
    if v.is_empty() || v.len() > 10 || v[0] == b'0' || !v.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(v).ok()?.parse::<u32>().ok()
}

fn id_of(v: &[u8]) -> Option<FixedStr<STORE_ID_LEN>> {
    let s = std::str::from_utf8(v).ok()?;
    (!s.is_empty() && s.len() < STORE_ID_LEN && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'))
        .then(|| FixedStr::from_str(s))
}

/// backlinks.ini's text read: its notebooks, sorted, or None when it does not read (a line that does
/// not parse, a notebook twice, past the limits). A cache that does not read is built again.
pub fn backlinks_parse(text: &[u8]) -> Option<Vec<BlSource>> {
    let mut out: Vec<BlSource> = Vec::new();
    let mut n = 0usize;
    for line in text.split(|&c| c == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        if let Some(rest) = line.strip_prefix(b"src=") {
            let comma = rest.iter().position(|&c| c == b',')?;
            let id = id_of(&rest[..comma])?;
            let stamp = hex16(&rest[comma + 1..])?;
            out.try_reserve(1).ok()?;
            out.push(BlSource { id, stamp, links: Vec::new() });
        } else if let Some(rest) = line.strip_prefix(b"to=") {
            let mut f = rest.split(|&c| c == b',');
            let (src, to, pid) = (dec(f.next()?)?, id_of(f.next()?)?, dec(f.next()?)?);
            if f.next().is_some() {
                return None;
            }
            let s = out.last_mut()?;
            if to == s.id {
                return None;
            }
            n += 1;
            if n > BL_LINKS_MAX {
                return None;
            }
            s.links.try_reserve(1).ok()?;
            s.links.push(BlLink { src, to, pid });
        } else {
            return None;
        }
    }
    for s in &mut out {
        s.links.sort_unstable();
        s.links.dedup();
    }
    out.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    if out.windows(2).any(|w| w[0].id == w[1].id) {
        return None;
    }
    Some(out)
}

/// The text backlinks.ini holds for these notebooks.
pub fn backlinks_text(sources: &[BlSource]) -> Vec<u8> {
    let mut t = HEADER.to_vec();
    for s in sources {
        t.extend_from_slice(format!("src={},{:016x}\n", s.id.as_str(), s.stamp).as_bytes());
        for l in &s.links {
            t.extend_from_slice(format!("to={},{},{}\n", l.src, l.to.as_str(), l.pid).as_bytes());
        }
    }
    t
}

fn load(b: &mut Backlinks) {
    if b.loaded {
        return;
    }
    b.loaded = true;
    let p = path();
    let Ok(md) = std::fs::symlink_metadata(&p) else { return };
    if !md.is_file() || md.len() as usize > BL_FILE_MAX {
        b.dirty = true; // (written afresh at the end of the check)
        return;
    }
    match read_file(&p).and_then(|t| backlinks_parse(&t)) {
        Some(sources) => {
            b.sources = sources;
            b.generation = b.generation.wrapping_add(1);
        }
        None => {
            qlog!("backlinks: the cache does not read; building it again");
            b.dirty = true;
        }
    }
}

fn flush(b: &mut Backlinks) -> bool {
    if !b.dirty {
        return true;
    }
    if write_file_atomic(&path(), &backlinks_text(&b.sources)) == 0 {
        b.dirty = false;
        return true;
    }
    false
}

/// nav_save_links wrote notebook id's links.ini with these links: its lines in the cache now. (Before the
/// cache is read, on the first idle tick, nothing: that tick's check finds the new stamp and reads it.)
pub fn backlinks_note_saved(id: &str, links: &[NavLink]) {
    if !with(|b| b.loaded) {
        return;
    }
    let stamp = backlinks_stamp(id);
    let ext = backlinks_of_links(id, links, None);
    with(|b| put(b, id, stamp, ext));
}

/// Every notebook is to be checked again (a notebook opens: another may have changed since).
pub fn backlinks_revalidate() {
    with(|b| {
        b.complete = false;
        b.scan = None;
    });
}

// Notebook id read again from its files: its links.ini (stamp is its stamp now) and its page list.
fn reread(b: &mut Backlinks, id: &str, stamp: u64) {
    let mut nav = Nav::default();
    nav_init(&mut nav);
    let mut nb = NotebookInfo::default();
    if nav_load_links(id, &mut nav) != 0 || store_load_info(id, &mut nb) != 0 {
        // (unreadable now: what was known of it stays, and it is tried again at the next check)
        return;
    }
    let ext = backlinks_of_links(id, &nav.links, Some(&nb.page_ids));
    put(b, id, stamp, ext);
}

fn list_ids() -> Vec<FixedStr<STORE_ID_LEN>> {
    let mut ids = Vec::new();
    if let Ok(rd) = std::fs::read_dir(format!("{}/notebooks", store_root())) {
        for e in rd.flatten() {
            let name = e.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with('.') {
                continue;
            }
            if let Some(id) = id_of(name.as_bytes()) {
                if ids.try_reserve(1).is_err() {
                    break;
                }
                ids.push(id);
            }
        }
    }
    ids.sort();
    ids
}

/// One idle tick's work: the cache read, or the notebooks listed, or up to BL_STATS_PER_STEP stamps
/// checked with at most one links.ini read again; at the end of a check, notebooks no longer there
/// dropped and the file written if it changed. True while there is more to do.
pub fn backlinks_step() -> bool {
    with(|b| {
        if !b.loaded {
            load(b);
            return true;
        }
        if b.complete {
            return false;
        }
        if b.scan.is_none() {
            b.scan = Some(Scan { ids: list_ids(), at: 0 });
            return true;
        }
        let mut stats = 0;
        loop {
            let Some(scan) = b.scan.as_mut() else { return false };
            if scan.at >= scan.ids.len() || stats >= BL_STATS_PER_STEP {
                break;
            }
            let id = scan.ids[scan.at];
            scan.at += 1;
            stats += 1;
            let stamp = backlinks_stamp(id.as_str());
            let known = b.sources.binary_search_by(|s| s.id.as_bytes().cmp(id.as_bytes())).ok().map(|i| b.sources[i].stamp);
            if stamp == 0 {
                // (no links.ini: nothing of it in the cache, a notebook left out at the limit included)
                if known.is_some() {
                    put(b, id.as_str(), 0, Vec::new());
                }
                continue;
            }
            if known == Some(stamp) {
                continue;
            }
            reread(b, id.as_str(), stamp);
            return true; // (a links.ini read is the tick's work)
        }
        let Some(scan) = b.scan.as_ref() else { return false };
        if scan.at < scan.ids.len() {
            return true;
        }
        let ids = std::mem::take(&mut b.scan).map(|s| s.ids).unwrap_or_default();
        let before = b.sources.len();
        let lost = b.sources.iter().any(|s| ids.binary_search(&s.id).is_err() && !s.links.is_empty());
        b.sources.retain(|s| ids.binary_search(&s.id).is_ok());
        if b.sources.len() != before {
            b.dirty = true;
            if lost {
                b.generation = b.generation.wrapping_add(1);
            }
        }
        b.complete = !std::mem::take(&mut b.retry);
        flush(b);
        !b.complete
    })
}

/// The cache written now if it changed (a notebook closing). False when the write failed.
pub fn backlinks_flush() -> bool {
    with(|b| !b.loaded || flush(b))
}

/// Changes whenever the links between notebooks the cache knows change.
pub fn backlinks_generation() -> u32 {
    with(|b| b.generation)
}

/// Every notebook has been checked since the last backlinks_revalidate.
pub fn backlinks_complete() -> bool {
    with(|b| b.complete)
}

/// A notebook is left out of the cache at BL_LINKS_MAX now (until there is room for it).
pub fn backlinks_partial() -> bool {
    with(|b| b.sources.iter().any(|s| s.stamp == 0))
}

/// Another notebook's notebook.ini and page names, as the Links bar's Linked from rows and the Graph's
/// anchors label its pages; None when its notebook.ini does not read. Kept between calls (the most recently
/// used BL_BOOKS_MAX) and read again only when notebook.ini or nav.ini changed: two stats, not its files.
pub fn backlinks_book(id: &str) -> Option<Rc<BlBook>> {
    let stamps = (file_stamp(id, "notebook.ini"), file_stamp(id, "nav.ini"));
    let hit = with(|b| {
        b.clock += 1;
        let clock = b.clock;
        let s = b.books.iter_mut().find(|s| s.id == id && s.stamps == stamps && stamps.0 != 0)?;
        s.used = clock;
        Some(s.book.clone())
    });
    if let Some(book) = hit {
        return book;
    }
    let mut info = NotebookInfo::default();
    let book = (store_load_info(id, &mut info) == 0).then(|| {
        let mut names = Nav::default();
        nav_init(&mut names);
        nav_load_names(id, &mut names);
        Rc::new(BlBook { info, names })
    });
    with(|b| {
        b.books_read = b.books_read.wrapping_add(1);
        let used = b.clock;
        b.books.retain(|s| s.id != id);
        if b.books.len() >= BL_BOOKS_MAX {
            if let Some(i) = (0..b.books.len()).min_by_key(|&i| b.books[i].used) {
                b.books.remove(i);
            }
        }
        if b.books.try_reserve(1).is_ok() {
            b.books.push(BookSlot { id: FixedStr::from_str(id), stamps, book: book.clone(), used });
        }
    });
    book
}

/// How many times backlinks_book has read a notebook's files.
pub fn backlinks_books_read() -> u32 {
    with(|b| b.books_read)
}

/// The pages of OTHER notebooks with links to page pid of notebook id, in notebook-id then page-id
/// order, one each (count 1: the cache keeps one line per pair of pages). What the cache knows now: it is
/// read on the first idle tick (backlinks_step), never by a query, so a draw never waits for the file.
pub fn backlinks_to(id: &str, pid: u32) -> Vec<BlFrom> {
    with(|b| {
        let mut out = Vec::new();
        for s in &b.sources {
            if s.id == id {
                continue;
            }
            for l in &s.links {
                if l.pid == pid && l.to == id && out.try_reserve(1).is_ok() {
                    out.push(BlFrom { id: s.id, pid: l.src, count: 1 });
                }
            }
        }
        out
    })
}

/// Every link into notebook id from other notebooks: (its page here, the other notebook, its page),
/// sorted by the other notebook, its page, then the page here.
pub fn backlinks_into(id: &str) -> Vec<(u32, FixedStr<STORE_ID_LEN>, u32)> {
    with(|b| {
        let mut out = Vec::new();
        for s in &b.sources {
            if s.id == id {
                continue;
            }
            for l in &s.links {
                if l.to == id && out.try_reserve(1).is_ok() {
                    out.push((l.pid, s.id, l.src));
                }
            }
        }
        out.sort_by(|a, b| (a.1, a.2, a.0).cmp(&(b.1, b.2, b.0)));
        out
    })
}

/// The pages linking to page pid of notebook nb (nav its links): its own pages first, in notebook
/// order (links from the page to itself left out), then other notebooks' from the cache. count is
/// how many links come from that page (always 1 for another notebook's page: the cache keeps pairs).
pub fn backlinks_for_page(nb: &NotebookInfo, nav: &Nav, pid: u32) -> Vec<BlFrom> {
    let mut out: Vec<BlFrom> = Vec::new();
    let here = format!("p{pid}");
    let own = nav_notebook_target(nb.id.as_str(), pid).unwrap_or_default();
    for &src in &nb.page_ids {
        if src == pid || out.iter().any(|f| f.pid == src) {
            continue;
        }
        let mut first = 0;
        let n = nav_links_of(nav, src, &mut first);
        let count = nav.links[first as usize..(first + n) as usize].iter().filter(|l| l.target == here.as_str() || l.target == own.as_str()).count();
        if count > 0 && out.try_reserve(1).is_ok() {
            out.push(BlFrom { id: nb.id, pid: src, count: count as i32 });
        }
    }
    let others = backlinks_to(nb.id.as_str(), pid);
    if out.try_reserve(others.len()).is_ok() {
        out.extend(others);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(tag: &str) -> std::path::PathBuf {
        let r = std::env::temp_dir().join(format!("grimoire-backlinks-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        assert_eq!(store_init(r.to_str().unwrap()), 0);
        r
    }
    fn book(name: &str, pages: i32) -> NotebookInfo {
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create(name, "", PAPER_BLANK, &mut nb), 0);
        while nb.npages() < pages {
            let n = nb.npages();
            assert_eq!(store_add_page(&mut nb, n, PAPER_BLANK), 0);
        }
        nb
    }
    fn link(nb: &NotebookInfo, from: usize, to: &NotebookInfo, page: usize, seed: u32) {
        let mut nav = Nav::default();
        nav_init(&mut nav);
        nav_load(nb.id.as_str(), &mut nav);
        let t = nav_notebook_target(to.id.as_str(), to.page_ids[page]).unwrap();
        assert_eq!(nav_link_set(&mut nav, nb.page_ids[from], seed, &t), 0);
        assert_eq!(nav_save_links(nb.id.as_str(), &mut nav, nb), 0);
    }
    fn settle() {
        let mut n = 0;
        while backlinks_step() {
            n += 1;
            assert!(n < 10_000, "the check never ends");
        }
    }

    #[test]
    fn a_save_updates_the_cache_at_once_and_the_file_after_a_check() {
        let r = root("save");
        let (a, b) = (book("A", 2), book("B", 3));
        // Before the first idle tick the cache is not read: a query sees nothing yet and never reads it.
        assert!(backlinks_to(b.id.as_str(), b.page_ids[2]).is_empty());
        assert!(!with(|b| b.loaded));
        assert!(backlinks_step(), "the first tick reads the cache");
        link(&a, 0, &b, 2, 1);
        let to = backlinks_to(b.id.as_str(), b.page_ids[2]);
        assert_eq!(to, vec![BlFrom { id: a.id, pid: a.page_ids[0], count: 1 }]);
        assert!(backlinks_to(b.id.as_str(), b.page_ids[0]).is_empty());
        // A link inside B to its own page is not another notebook's.
        link(&b, 0, &b, 2, 2);
        assert_eq!(backlinks_to(b.id.as_str(), b.page_ids[2]).len(), 1);
        settle();
        let text = std::fs::read(r.join("backlinks.ini")).unwrap();
        let parsed = backlinks_parse(&text).unwrap();
        assert!(parsed.iter().any(|s| s.id == a.id && s.links.len() == 1));
        // A save that changes nothing between notebooks writes nothing more.
        let gen = backlinks_generation();
        link(&a, 1, &a, 0, 3);
        assert_eq!(backlinks_generation(), gen);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_lazy_check_finds_what_another_version_wrote_and_deleted_notebooks() {
        let r = root("lazy");
        let (a, b, c) = (book("A", 2), book("B", 2), book("C", 1));
        // Written behind this version's back (no nav_save_links), as a copy or another version would.
        let line = format!("link.{}=00000001,n:{}:p{}\n", a.page_ids[1], b.id.as_str(), b.page_ids[0]);
        std::fs::write(r.join(format!("notebooks/{}/links.ini", a.id.as_str())), &line).unwrap();
        let line = format!("link.{}=00000001,n:{}:p{}\n", c.page_ids[0], b.id.as_str(), b.page_ids[0]);
        std::fs::write(r.join(format!("notebooks/{}/links.ini", c.id.as_str())), &line).unwrap();
        settle();
        assert!(backlinks_complete());
        let to = backlinks_to(b.id.as_str(), b.page_ids[0]);
        assert_eq!(to.len(), 2, "{to:?}");
        // C is deleted: the next check drops it.
        assert_eq!(store_delete(c.id.as_str()), 0);
        let gen = backlinks_generation();
        backlinks_revalidate();
        settle();
        assert_ne!(backlinks_generation(), gen);
        assert_eq!(backlinks_to(b.id.as_str(), b.page_ids[0]), vec![BlFrom { id: a.id, pid: a.page_ids[1], count: 1 }]);
        // A link from a page A no longer lists is not a link.
        let line = format!("link.999=00000001,n:{}:p{}\n", b.id.as_str(), b.page_ids[1]);
        std::fs::write(r.join(format!("notebooks/{}/links.ini", a.id.as_str())), &line).unwrap();
        backlinks_revalidate();
        settle();
        assert!(backlinks_to(b.id.as_str(), b.page_ids[1]).is_empty());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn the_cache_is_read_back_and_an_unchanged_notebook_is_not_read_again() {
        let r = root("reread");
        let (a, b) = (book("A", 1), book("B", 1));
        link(&a, 0, &b, 0, 1);
        settle();
        assert!(backlinks_flush());
        // A new session (another root and back: the state starts again) reads the file.
        let other = std::env::temp_dir().join(format!("grimoire-backlinks-other-{}", std::process::id()));
        assert_eq!(store_init(other.to_str().unwrap()), 0);
        assert!(backlinks_to("X", 1).is_empty());
        assert_eq!(store_init(r.to_str().unwrap()), 0);
        // The cache read back (on the first tick) says A links to B, and a check finds A's stamp unchanged.
        assert!(backlinks_to(b.id.as_str(), b.page_ids[0]).is_empty(), "a query never reads the file");
        backlinks_step();
        assert_eq!(backlinks_to(b.id.as_str(), b.page_ids[0]).len(), 1);
        settle();
        assert_eq!(backlinks_to(b.id.as_str(), b.page_ids[0]).len(), 1);
        let _ = std::fs::remove_dir_all(&r);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn a_cache_that_does_not_read_is_built_again() {
        assert!(backlinks_parse(b"to=1,B,2\n").is_none(), "a to line before any src");
        assert!(backlinks_parse(b"src=A,1\nto=1,A,2\n").is_none(), "a notebook to itself");
        assert!(backlinks_parse(b"src=A,1\nsrc=A,2\n").is_none(), "a notebook twice");
        assert!(backlinks_parse(b"src=A,1\nto=01,B,2\n").is_none(), "a leading zero");
        assert!(backlinks_parse(b"src=../x,1\n").is_none(), "an id that is no notebook's");
        assert!(backlinks_parse(b"src=A,1\nfuture=1\n").is_none());
        let s = backlinks_parse(b"# c\r\nsrc=B,ff\r\nto=3,A,1\r\nto=3,A,1\nsrc=A,0\n").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id.as_str(), "A");
        assert_eq!(s[1].links.len(), 1);
        assert_eq!(backlinks_parse(&backlinks_text(&s)).unwrap(), s);
        let r = root("bad");
        let (a, b) = (book("A", 1), book("B", 1));
        std::fs::write(r.join("backlinks.ini"), b"garbage\n").unwrap();
        let line = format!("link.{}=00000001,n:{}:p{}\n", a.page_ids[0], b.id.as_str(), b.page_ids[0]);
        std::fs::write(r.join(format!("notebooks/{}/links.ini", a.id.as_str())), &line).unwrap();
        settle();
        assert_eq!(backlinks_to(b.id.as_str(), b.page_ids[0]).len(), 1);
        assert!(backlinks_parse(&std::fs::read(r.join("backlinks.ini")).unwrap()).is_some());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_notebook_with_4000_links_to_another_fits_and_the_limit_holds() {
        let r = root("limits");
        let (a, b) = (book("A", 1), book("B", 1));
        let mut text = String::new();
        // 4000 links from 63 pages, 64 a page at most: page ids A does not list are left out, so A
        // lists them all here (store_add_page is slow for 63: the ini is written by hand).
        let pids: Vec<u32> = (1..=63).collect();
        let ini = r.join(format!("notebooks/{}/notebook.ini", a.id.as_str()));
        let old = std::fs::read_to_string(&ini).unwrap();
        let mut new: String = old.lines().filter(|l| !l.starts_with("pages=") && !l.starts_with("next_page_id=")).map(|l| format!("{l}\n")).collect();
        new.push_str(&format!("next_page_id=64\npages={}\n", pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",")));
        std::fs::write(&ini, new).unwrap();
        for k in 0..NAV_LINKS_MAX as u32 {
            text.push_str(&format!("link.{}={:08x},n:{}:p{}\n", 1 + k / 64, k, b.id.as_str(), 1 + k));
        }
        std::fs::write(r.join(format!("notebooks/{}/links.ini", a.id.as_str())), &text).unwrap();
        settle();
        assert_eq!(backlinks_into(b.id.as_str()).len(), NAV_LINKS_MAX as usize);
        assert!(!backlinks_partial());
        // Past BL_LINKS_MAX all together, a notebook is left out (and says so), never a crash.
        let mut many = vec![BlSource { id: FixedStr::from_str("Z"), stamp: 1, links: Vec::new() }];
        for k in 0..BL_LINKS_MAX as u32 {
            many[0].links.push(BlLink { src: 1, to: FixedStr::from_str("Y"), pid: k + 1 });
        }
        with(|bl| {
            bl.sources = many;
            put(bl, "Q", 5, vec![BlLink { src: 1, to: FixedStr::from_str("Y"), pid: 1 }]);
            let q = bl.sources.iter().find(|s| s.id == "Q").unwrap();
            assert!(q.links.is_empty());
            assert_eq!(q.stamp, 0, "left out: kept as not read, to be read again");
        });
        assert!(backlinks_partial());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_notebook_left_out_is_taken_in_once_another_makes_room() {
        let r = root("room");
        let (q, y, z) = (book("Q", 1), book("Y", 1), book("Z", 1));
        let line = format!("link.{}=00000001,n:{}:p{}\n", q.page_ids[0], y.id.as_str(), y.page_ids[0]);
        std::fs::write(r.join(format!("notebooks/{}/links.ini", q.id.as_str())), &line).unwrap();
        settle();
        assert_eq!(backlinks_to(y.id.as_str(), y.page_ids[0]).len(), 1);
        // Z fills the cache to the limit (put as a save would, with its real stamp); Q, read again, is left out.
        let full: Vec<BlLink> = (0..BL_LINKS_MAX as u32).map(|k| BlLink { src: z.page_ids[0], to: y.id, pid: k + 2 }).collect();
        std::fs::write(r.join(format!("notebooks/{}/links.ini", z.id.as_str())), "# z\n").unwrap();
        let zs = backlinks_stamp(z.id.as_str());
        with(|b| {
            let qi = b.sources.iter().position(|s| s.id == q.id).unwrap();
            b.sources.remove(qi);
            put(b, z.id.as_str(), zs, full);
        });
        std::fs::write(r.join(format!("notebooks/{}/links.ini", q.id.as_str())), format!("{line}# again\n")).unwrap();
        backlinks_revalidate();
        settle();
        assert!(backlinks_partial());
        assert!(backlinks_to(y.id.as_str(), y.page_ids[0]).is_empty());
        // A check with nothing new reads Q again (stamp 0) and still has no room: it stays out, and ends.
        backlinks_revalidate();
        settle();
        assert!(backlinks_partial());
        // Z deletes its links (a save): room again, so a check runs by itself and takes Q in.
        backlinks_note_saved(z.id.as_str(), &[]);
        assert!(!backlinks_complete(), "room made starts a check");
        settle();
        assert!(!backlinks_partial());
        assert_eq!(backlinks_to(y.id.as_str(), y.page_ids[0]).len(), 1);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn another_notebooks_names_are_read_once_and_again_only_when_they_change() {
        let r = root("books");
        let b = book("B", 2);
        // links.ini that does not read (past its 512 KiB): a names-only read never looks at it.
        std::fs::write(r.join(format!("notebooks/{}/links.ini", b.id.as_str())), "# x\n".repeat(200_000)).unwrap();
        std::fs::write(r.join(format!("notebooks/{}/nav.ini", b.id.as_str())), format!("name.{}=Second\n", b.page_ids[1])).unwrap();
        let mut n = Nav::default();
        nav_init(&mut n);
        assert_eq!(nav_load_names(b.id.as_str(), &mut n), 0);
        assert_eq!(nav_name(&n, b.page_ids[1]), "Second");
        assert!(n.links.is_empty() && !n.links_writable);
        let mut full = Nav::default();
        nav_init(&mut full);
        assert_eq!(nav_load(b.id.as_str(), &mut full), -1, "nav_load does read links.ini");
        let read = backlinks_books_read();
        let first = backlinks_book(b.id.as_str()).unwrap();
        assert_eq!(nav_name(&first.names, b.page_ids[1]), "Second");
        for _ in 0..5 {
            assert!(backlinks_book(b.id.as_str()).is_some());
        }
        assert_eq!(backlinks_books_read() - read, 1);
        // nav.ini written again (a rename): read again once.
        std::fs::write(r.join(format!("notebooks/{}/nav.ini", b.id.as_str())), format!("name.{}=Other\n", b.page_ids[1])).unwrap();
        assert_eq!(nav_name(&backlinks_book(b.id.as_str()).unwrap().names, b.page_ids[1]), "Other");
        assert!(backlinks_book(b.id.as_str()).is_some());
        assert_eq!(backlinks_books_read() - read, 2);
        // A notebook that is gone has no book.
        assert_eq!(store_delete(b.id.as_str()), 0);
        assert!(backlinks_book(b.id.as_str()).is_none());
        // At most BL_BOOKS_MAX are kept.
        for k in 0..BL_BOOKS_MAX + 4 {
            let nb = book(&format!("N{k}"), 1);
            assert!(backlinks_book(nb.id.as_str()).is_some());
        }
        assert!(with(|b| b.books.len()) <= BL_BOOKS_MAX);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn own_pages_come_first_then_other_notebooks() {
        let r = root("forpage");
        backlinks_step(); // (the cache read: the first idle tick)
        let (a, b) = (book("A", 3), book("B", 1));
        let mut nav = Nav::default();
        nav_init(&mut nav);
        let to = format!("p{}", a.page_ids[0]);
        assert_eq!(nav_link_set(&mut nav, a.page_ids[2], 1, &to), 0);
        assert_eq!(nav_link_set(&mut nav, a.page_ids[2], 2, &to), 0);
        assert_eq!(nav_link_set(&mut nav, a.page_ids[1], 3, &to), 0);
        assert_eq!(nav_link_set(&mut nav, a.page_ids[0], 4, &to), 0); // (to itself: not listed)
        assert_eq!(nav_save_links(a.id.as_str(), &mut nav, &a), 0);
        link(&b, 0, &a, 0, 9);
        let from = backlinks_for_page(&a, &nav, a.page_ids[0]);
        assert_eq!(from.len(), 3, "{from:?}");
        assert_eq!((from[0].pid, from[0].count), (a.page_ids[1], 1));
        assert_eq!((from[1].pid, from[1].count), (a.page_ids[2], 2));
        assert_eq!(from[2].id, b.id);
        let _ = std::fs::remove_dir_all(&r);
    }
}
