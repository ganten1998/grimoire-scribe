//! libindex: every notebook's pages, names and links, for the Graph of all notebooks (Rust only,
//! 2.0.0-beta.2; docs/beta2/GRAPH-ALL.md). Reading every notebook's notebook.ini, nav.ini and
//! links.ini when the Graph opens would be 1500 file opens for 500 notebooks on the Kindle's FUSE
//! storage, so this module keeps what they say in root/graph-index.ini:
//!
//! ```text
//! # Grimoire: every notebook's pages, names and links, for the Graph (a cache: deleting it loses nothing)
//! nb=20261010-143205-a3f1,9f3c22d05b1e4471,1b2c3d4e5f607182,77aa66bb55cc44dd
//! name=Lecture notes
//! folder=School/2026
//! omit=1
//! pages=1,2,5,7,8
//! pname=5,Thermodynamics
//! link=5,p7
//! link=7,n:20261012-090000-b2c4:p3
//! link=8,index
//! ```
//!
//! `nb=<id>,<notebook.ini>,<nav.ini>,<links.ini>` names a notebook and the stamps of its three files
//! when they were read (backlinks' file stamps, 16 hex digits; 0 for a file that is not there), then
//! its name, folder, whether it is left out of the Graph (`omit=1`, notebook.ini's graph=0; no line when
//! it is not), page order, page names and links (targets as links.ini writes them, links from
//! pages not in `pages` left out). A notebook whose notebook.ini does not read, or that was left out
//! at LIB_LINKS_MAX, is `nb=<id>,0,0,0` alone, read again at every check. It is a cache, never data:
//! deleting it, or a file that does not read, loses nothing, and no other version reads it.
//!
//! It is kept current as backlinks.ini is. Where this version writes a notebook's files, its entry is
//! updated in memory at once with the new file's stamp (libindex_note_info from notebook.ini's
//! writer, libindex_note_names from nav_save, libindex_note_links from nav_save_links, and
//! libindex_note_gone when a notebook goes to the trash). And on idle ticks (libindex_step) every
//! notebook's three stamps are checked, LIB_STATS_PER_STEP notebooks a tick, and one notebook whose
//! files changed (another version, a copy, a notebook brought back) is read again, one a tick, with
//! the pens pumped between its files: the main loop goes round every tick. A notebook not there any
//! more loses its entry once a check of every notebook ends. The file is read on the first idle tick
//! (or when the Graph opens before it), and written after a check, or as a notebook closes, only when
//! what it holds changed.
//!
//! The module is pure apart from its files: no screen (it times its reads, for the log). Its state is per
//! thread (the store's root is), and starts again when the root changes (the tests' roots).

use crate::prelude::*;
use std::cell::RefCell;

/// graph-index.ini is read up to this size; a bigger one is not read (and rebuilt).
pub const LIB_FILE_MAX: usize = 4 * 1024 * 1024;
/// The most links the index keeps, every notebook together: the Graph of all notebooks' cap
/// (GRAPH_ALL_EDGES_MAX). A notebook whose links would pass it is left out (libindex_left_out).
pub const LIB_LINKS_MAX: usize = GRAPH_ALL_EDGES_MAX;
/// A tick checks this many notebooks' stamps (three files each), and reads at most one notebook.
pub const LIB_STATS_PER_STEP: usize = 8;

const HEADER: &[u8] = b"# Grimoire: every notebook's pages, names and links, for the Graph (a cache: deleting it loses nothing)\n";

/// One notebook in the index: its files' stamps when they were read, and what they said.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LibEntry {
    /// notebook.ini's, nav.ini's and links.ini's stamps (0: not there); all 0 for a notebook that is
    /// read again at every check (its notebook.ini did not read, or it was left out)
    pub stamps: [u64; 3],
    /// left out at LIB_LINKS_MAX (until there is room for it)
    pub left_out: bool,
    pub book: GraphBook,
}

impl LibEntry {
    /// Its notebook.ini read (a notebook with pages, as far as the index knows).
    pub fn readable(&self) -> bool {
        self.stamps[0] != 0
    }
}

#[derive(Default)]
struct Scan {
    ids: Vec<FixedStr<STORE_ID_LEN>>,
    at: usize,
}

#[derive(Default)]
struct Lib {
    root: String,
    loaded: bool,
    /// the file was not there (or did not read) when it was loaded: the first check builds it
    building: bool,
    /// sorted by id
    entries: Vec<LibEntry>,
    dirty: bool,
    scan: Option<Scan>,
    complete: bool,
    retry: bool,
    /// changes whenever what a notebook says changes: the Graph knows to build again
    generation: u32,
    /// how many notebooks have been read from their files (a probe of the work the stamps save)
    reads: u32,
    /// graph-omit.ini's folders, read when first asked and again after libindex_note_omit
    omit_folders: Option<Vec<String>>,
    /// this check's notebook reads and the time they took (logged when it ends: what the device's storage
    /// costs)
    check_reads: u32,
    check_us: u64,
    /// notebook.ini writes and trashings before the index was read (the Library's menus before any Graph or
    /// notebook opened), by id, the last one each: put over the file's entries as it loads, so a Graph opened
    /// at once draws what the files say now (a notebook just left out, moved into a left-out folder, trashed)
    pending: Vec<(FixedStr<STORE_ID_LEN>, Option<Noted>)>,
}

/// What a notebook.ini write said of its notebook (libindex_note_info), with the new file's stamp.
#[derive(Clone)]
struct Noted {
    stamp: u64,
    name: String,
    folder: String,
    omit: bool,
    pages: Vec<u32>,
}

thread_local! {
    static LIB: RefCell<Lib> = RefCell::new(Lib::default());
}

fn path() -> String {
    format!("{}/graph-index.ini", store_root())
}

fn with<R>(f: impl FnOnce(&mut Lib) -> R) -> R {
    LIB.with(|l| {
        let mut l = l.borrow_mut();
        let root = store_root();
        if l.root != root {
            *l = Lib { root, ..Lib::default() };
        }
        f(&mut l)
    })
}

/// The three files' stamps of notebook id now (backlinks' file stamps; 0 for a file not there).
pub fn libindex_stamps(id: &str) -> [u64; 3] {
    [backlinks_file_stamp(id, "notebook.ini"), backlinks_file_stamp(id, "nav.ini"), backlinks_file_stamp(id, "links.ini")]
}

fn total(l: &Lib) -> usize {
    l.entries.iter().map(|e| e.book.links.len()).sum()
}

fn find(l: &Lib, id: &str) -> Result<usize, usize> {
    l.entries.binary_search_by(|e| e.book.id.as_bytes().cmp(id.as_bytes()))
}

// Puts an entry in (replacing the one for its id), past LIB_LINKS_MAX left out: no stamps, no pages,
// names or links. Marks it dirty, and moves the generation on when what it says changed.
fn put(l: &mut Lib, mut e: LibEntry) {
    let at = find(l, e.book.id.as_str());
    let others = total(l) - at.map_or(0, |i| l.entries[i].book.links.len());
    e.left_out = false;
    if others + e.book.links.len() > LIB_LINKS_MAX {
        e = LibEntry { stamps: [0; 3], left_out: true, book: GraphBook { id: e.book.id, name: e.book.name, ..GraphBook::default() } };
    }
    let before = total(l);
    match at {
        Ok(i) => {
            if l.entries[i] != e {
                if l.entries[i].book != e.book || l.entries[i].left_out != e.left_out {
                    l.generation = l.generation.wrapping_add(1);
                }
                l.entries[i] = e;
                l.dirty = true;
            }
        }
        Err(i) => {
            if l.entries.try_reserve(1).is_err() {
                return;
            }
            l.entries.insert(i, e);
            l.generation = l.generation.wrapping_add(1);
            l.dirty = true;
        }
    }
    // Room made where a notebook was left out: a check soon, to take it in.
    if total(l) < before && l.entries.iter().any(|e| e.left_out) {
        l.complete = false;
        l.retry = true;
    }
}

fn remove(l: &mut Lib, id: &str) {
    if let Ok(i) = find(l, id) {
        let had = l.entries[i].left_out || !l.entries[i].book.links.is_empty();
        l.entries.remove(i);
        l.dirty = true;
        l.generation = l.generation.wrapping_add(1);
        if had && l.entries.iter().any(|e| e.left_out) {
            l.complete = false;
            l.retry = true;
        }
    }
}

// ---- the file ----

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

fn text_of(v: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(v).ok()?;
    (!s.contains('\r') && !s.contains('\0')).then(|| s.to_string())
}

/// graph-index.ini's text read: its notebooks, sorted by id, or None when it does not read (a line
/// that does not parse, a notebook twice, a line before the first notebook, past LIB_LINKS_MAX). A
/// cache that does not read is built again.
pub fn libindex_parse(text: &[u8]) -> Option<Vec<LibEntry>> {
    let mut out: Vec<LibEntry> = Vec::new();
    let mut links = 0usize;
    for line in text.split(|&c| c == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        if let Some(rest) = line.strip_prefix(b"nb=") {
            let mut f = rest.split(|&c| c == b',');
            let id = id_of(f.next()?)?;
            let mut stamps = [0u64; 3];
            for s in &mut stamps {
                *s = hex16(f.next()?)?;
            }
            if f.next().is_some() {
                return None;
            }
            out.try_reserve(1).ok()?;
            out.push(LibEntry { stamps, left_out: false, book: GraphBook { id, ..GraphBook::default() } });
            continue;
        }
        let e = out.last_mut()?;
        // (an entry read again at every check holds nothing)
        if !e.readable() {
            return None;
        }
        let b = &mut e.book;
        if let Some(rest) = line.strip_prefix(b"name=") {
            b.name = text_of(rest)?;
        } else if let Some(rest) = line.strip_prefix(b"folder=") {
            b.folder = text_of(rest)?;
        } else if line == b"omit=1" {
            b.omit = true;
        } else if let Some(rest) = line.strip_prefix(b"pages=") {
            if !b.pages.is_empty() {
                return None;
            }
            if !rest.is_empty() {
                for p in rest.split(|&c| c == b',') {
                    if b.pages.len() >= STORE_MAX_PAGES {
                        return None;
                    }
                    b.pages.try_reserve(1).ok()?;
                    b.pages.push(dec(p)?);
                }
            }
        } else if let Some(rest) = line.strip_prefix(b"pname=") {
            let comma = rest.iter().position(|&c| c == b',')?;
            let pid = dec(&rest[..comma])?;
            let name = text_of(&rest[comma + 1..])?;
            if name.is_empty() {
                return None;
            }
            b.names.try_reserve(1).ok()?;
            b.names.push((pid, name));
        } else if let Some(rest) = line.strip_prefix(b"link=") {
            let comma = rest.iter().position(|&c| c == b',')?;
            let src = dec(&rest[..comma])?;
            let target = std::str::from_utf8(&rest[comma + 1..]).ok()?;
            let to = graph_to_of(b.id.as_str(), target)?;
            links += 1;
            if links > LIB_LINKS_MAX {
                return None;
            }
            b.links.try_reserve(1).ok()?;
            b.links.push(GraphLink { src, to });
        } else {
            return None;
        }
    }
    for e in &mut out {
        e.book.names.sort_by_key(|n| n.0);
        e.book.names.dedup_by_key(|n| n.0);
        e.book.links.sort_unstable();
        e.book.links.dedup();
    }
    out.sort_by(|a, b| a.book.id.as_bytes().cmp(b.book.id.as_bytes()));
    if out.windows(2).any(|w| w[0].book.id == w[1].book.id) {
        return None;
    }
    Some(out)
}

/// The text graph-index.ini holds for these notebooks.
pub fn libindex_text(entries: &[LibEntry]) -> Vec<u8> {
    let mut t = HEADER.to_vec();
    for e in entries {
        let b = &e.book;
        t.extend_from_slice(format!("nb={},{:016x},{:016x},{:016x}\n", b.id.as_str(), e.stamps[0], e.stamps[1], e.stamps[2]).as_bytes());
        if !e.readable() {
            continue;
        }
        t.extend_from_slice(format!("name={}\nfolder={}\n{}pages=", b.name, b.folder, if b.omit { "omit=1\n" } else { "" }).as_bytes());
        for (k, p) in b.pages.iter().enumerate() {
            if k > 0 {
                t.push(b',');
            }
            t.extend_from_slice(p.to_string().as_bytes());
        }
        t.push(b'\n');
        for (pid, name) in &b.names {
            t.extend_from_slice(format!("pname={pid},{name}\n").as_bytes());
        }
        for l in &b.links {
            t.extend_from_slice(format!("link={},{}\n", l.src, graph_to_text(&l.to)).as_bytes());
        }
    }
    t
}

fn load(l: &mut Lib) {
    if l.loaded {
        return;
    }
    l.loaded = true;
    let p = path();
    let Ok(md) = std::fs::symlink_metadata(&p) else {
        begin_building(l);
        return;
    };
    if !md.is_file() || md.len() as usize > LIB_FILE_MAX {
        begin_building(l); // (written afresh at the end of the check)
        return;
    }
    match read_file(&p).and_then(|t| libindex_parse(&t)) {
        Some(entries) => {
            l.entries = entries;
            l.generation = l.generation.wrapping_add(1);
            // (what was written since the file was: newer than it)
            for (id, n) in std::mem::take(&mut l.pending) {
                match n {
                    Some(n) => noted(l, id.as_str(), n),
                    None => remove(l, id.as_str()),
                }
            }
        }
        None => {
            qlog!("libindex: the cache does not read; building it again");
            begin_building(l);
        }
    }
}

// Kept for the load: the last note of each notebook (none after a build, which reads every notebook's files).
fn note_pending(l: &mut Lib, id: FixedStr<STORE_ID_LEN>, n: Option<Noted>) {
    if let Some(i) = l.pending.iter().position(|(p, _)| *p == id) {
        l.pending[i].1 = n;
    } else if l.pending.try_reserve(1).is_ok() {
        l.pending.push((id, n));
    }
}

// No index to start from: the check that builds it begins now, its notebooks listed (one directory read), so
// "Reading notebooks: 0 of 50" can say how many there are from the start.
fn begin_building(l: &mut Lib) {
    l.pending = Vec::new();
    l.building = true;
    l.dirty = true;
    l.complete = false;
    l.scan = Some(Scan { ids: list_ids(), at: 0 });
}

fn flush(l: &mut Lib) -> bool {
    if !l.dirty {
        return true;
    }
    if write_file_atomic(&path(), &libindex_text(&l.entries)) == 0 {
        l.dirty = false;
        return true;
    }
    false
}

// ---- kept current on save ----

/// notebook.ini was written for nb (create, a page added, moved or deleted, a rename, a move to
/// another folder, Move to... and Copy to notebook...): its pages, name and folder in the index now,
/// with the new file's stamp. Before the index is read, kept for its load (Leave out of the Graph in the
/// Library's menu, then the Graph at once: it must not draw the file's older word).
pub fn libindex_note_info(nb: &NotebookInfo) {
    let n = Noted {
        stamp: backlinks_file_stamp(nb.id.as_str(), "notebook.ini"),
        name: nb.name.as_str().to_string(),
        folder: nb.folder.as_str().to_string(),
        omit: nb.graph_omit,
        pages: nb.page_ids.clone(),
    };
    with(|l| {
        if l.loaded {
            noted(l, nb.id.as_str(), n);
        } else {
            note_pending(l, nb.id, Some(n));
        }
    });
}

fn noted(l: &mut Lib, id: &str, n: Noted) {
    let mut e = match find(l, id) {
        Ok(i) if !l.entries[i].left_out => l.entries[i].clone(),
        // (new to the index, or one it left out or could not read: its other files are read at the
        // next check, their stamps being unknown here)
        _ => LibEntry { stamps: [0, 0, 0], left_out: false, book: GraphBook { id: FixedStr::from_str(id), ..GraphBook::default() } },
    };
    if e.stamps[0] == 0 {
        e.stamps = [0, 0, 0];
        l.complete = false;
    }
    e.stamps[0] = n.stamp;
    e.book.name = n.name;
    e.book.folder = n.folder;
    e.book.omit = n.omit;
    e.book.pages = n.pages;
    put(l, e);
}

/// nav.ini was written for notebook nb (nav_save): its page names in the index now.
pub fn libindex_note_names(nb: &NotebookInfo, nav: &Nav) {
    if !with(|l| l.loaded) {
        return;
    }
    let id = nb.id.as_str();
    let stamp = backlinks_file_stamp(id, "nav.ini");
    with(|l| {
        let Ok(i) = find(l, id) else { l.complete = false; return };
        if !l.entries[i].readable() || l.entries[i].left_out {
            l.complete = false;
            return;
        }
        let mut e = l.entries[i].clone();
        e.stamps[1] = stamp;
        e.book.names = graph_names_of(&nav.names, &nb.page_ids);
        put(l, e);
    });
}

/// links.ini was written for notebook id with these links (nav_save_links, which has left out the
/// links of pages no longer in the notebook): its links in the index now.
pub fn libindex_note_links(id: &str, links: &[NavLink]) {
    if !with(|l| l.loaded) {
        return;
    }
    let stamp = backlinks_file_stamp(id, "links.ini");
    with(|l| {
        let Ok(i) = find(l, id) else { l.complete = false; return };
        if !l.entries[i].readable() || l.entries[i].left_out {
            l.complete = false;
            return;
        }
        let mut e = l.entries[i].clone();
        e.stamps[2] = stamp;
        e.book.links = graph_links_of(id, links, &e.book.pages);
        put(l, e);
    });
}

/// Notebook id went to the trash: out of the index now.
pub fn libindex_note_gone(id: &str) {
    with(|l| {
        if l.loaded {
            remove(l, id);
        } else {
            note_pending(l, FixedStr::from_str(id), None);
        }
    });
}

/// graph-omit.ini was written (a folder left out of the Graph, or shown again, renamed or removed): read
/// again when next asked, and the Graph builds again.
pub fn libindex_note_omit() {
    with(|l| {
        l.omit_folders = None;
        l.generation = l.generation.wrapping_add(1);
    });
}

fn omit_folders(l: &mut Lib) -> &[String] {
    l.omit_folders.get_or_insert_with(store_graph_omit_folders)
}

/// Whether the Graph of all notebooks leaves notebook `book` out (graph_omits, with graph-omit.ini's folders).
pub fn libindex_omits(book: &GraphBook) -> bool {
    with(|l| graph_omits(book, omit_folders(l)))
}

/// Every notebook is to be checked again (a notebook opens: another may have changed since).
pub fn libindex_revalidate() {
    with(|l| {
        l.complete = false;
        l.scan = None;
    });
}

// ---- the idle check ----

// Notebook id read again from its files (stamps: theirs now), the pens pumped between them.
fn reread(l: &mut Lib, id: &FixedStr<STORE_ID_LEN>, stamps: [u64; 3]) {
    let t = std::time::Instant::now();
    reread_files(l, id, stamps);
    l.check_reads += 1;
    l.check_us += t.elapsed().as_micros() as u64;
}

fn reread_files(l: &mut Lib, id: &FixedStr<STORE_ID_LEN>, stamps: [u64; 3]) {
    l.reads = l.reads.wrapping_add(1);
    let mut nb = NotebookInfo::default();
    if store_load_info(id.as_str(), &mut nb) != 0 {
        // (read again at every check, until it reads)
        put(l, LibEntry { stamps: [0; 3], left_out: false, book: GraphBook { id: *id, ..GraphBook::default() } });
        return;
    }
    work_pump();
    // (each of nav_load_names and nav_load_links replaces the Nav it is given: the names are read into one,
    // the links into another, and put together)
    let mut nav = Nav::default();
    nav_init(&mut nav);
    nav_load_names(id.as_str(), &mut nav);
    work_pump();
    let mut links = Nav::default();
    nav_init(&mut links);
    nav_load_links(id.as_str(), &mut links);
    nav.links = std::mem::take(&mut links.links);
    work_pump();
    put(l, LibEntry { stamps, left_out: false, book: graph_book_of(&nb, &nav) });
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

/// One idle tick's work: the index read, or the notebooks listed, or up to LIB_STATS_PER_STEP
/// notebooks' stamps checked with at most one notebook read again; at the end of a check, notebooks no
/// longer there dropped and the file written if it changed. True while there is more to do.
pub fn libindex_step() -> bool {
    with(|l| {
        if !l.loaded {
            load(l);
            return true;
        }
        if l.complete {
            return false;
        }
        if l.scan.is_none() {
            l.scan = Some(Scan { ids: list_ids(), at: 0 });
            return true;
        }
        let mut stats = 0;
        loop {
            let Some(scan) = l.scan.as_mut() else { return false };
            if scan.at >= scan.ids.len() || stats >= LIB_STATS_PER_STEP {
                break;
            }
            let id = scan.ids[scan.at];
            scan.at += 1;
            stats += 1;
            let stamps = libindex_stamps(id.as_str());
            let known = find(l, id.as_str()).ok().map(|i| l.entries[i].stamps);
            if known == Some(stamps) && stamps[0] != 0 {
                continue;
            }
            reread(l, &id, stamps);
            return true; // (a notebook read is the tick's work)
        }
        let Some(scan) = l.scan.as_ref() else { return false };
        if scan.at < scan.ids.len() {
            return true;
        }
        let ids = std::mem::take(&mut l.scan).map(|s| s.ids).unwrap_or_default();
        let before = l.entries.len();
        l.entries.retain(|e| ids.binary_search(&e.book.id).is_ok());
        if l.entries.len() != before {
            l.dirty = true;
            l.generation = l.generation.wrapping_add(1);
        }
        l.complete = !std::mem::take(&mut l.retry);
        if l.complete {
            l.building = false;
        }
        if l.check_reads > 0 {
            qlog!("libindex: %d of %d notebooks read again, %u ms", l.check_reads as i32, ids.len() as i32, (l.check_us / 1000) as u32);
        }
        l.check_reads = 0;
        l.check_us = 0;
        flush(l);
        !l.complete
    })
}

/// The index read now if it was not yet (the Graph opened before the first idle tick): one file.
pub fn libindex_load() {
    with(load);
}

/// The index written now if it changed (a notebook closing; before a layout child reads it). False
/// when the write failed.
pub fn libindex_flush() -> bool {
    with(|l| !l.loaded || flush(l))
}

/// Changes whenever what the index knows of any notebook changes.
pub fn libindex_generation() -> u32 {
    with(|l| l.generation)
}

/// Every notebook has been checked since the last libindex_revalidate.
pub fn libindex_complete() -> bool {
    with(|l| l.complete)
}

/// The index is being built for the first time (no file, or one that did not read): how many
/// notebooks of how many the check has gone through, for "Reading notebooks: 12 of 50". None once a
/// check has ended, or while there was a file.
pub fn libindex_building() -> Option<(usize, usize)> {
    with(|l| {
        if !l.building || l.complete {
            return None;
        }
        Some(l.scan.as_ref().map_or((0, 0), |s| (s.at, s.ids.len())))
    })
}

/// How many notebooks are left out at LIB_LINKS_MAX now.
pub fn libindex_left_out() -> usize {
    with(|l| l.entries.iter().filter(|e| e.left_out).count())
}

/// How many times a notebook has been read from its files.
pub fn libindex_reads() -> u32 {
    with(|l| l.reads)
}

/// Every page and every notebook the index knows: (pages, notebooks), the readable ones the Graph of all
/// notebooks shows (those left out of it, libindex_omits, are not counted).
pub fn libindex_page_totals() -> (usize, usize) {
    with(|l| {
        let mut pages = 0;
        let mut books = 0;
        let folders = omit_folders(l).to_vec();
        for e in l.entries.iter().filter(|e| e.readable() && !graph_omits(&e.book, &folders)) {
            pages += e.book.pages.len();
            books += 1;
        }
        (pages, books)
    })
}

/// f over the index's entries as they are now, sorted by notebook id (nothing is read).
pub fn libindex_with<R>(f: impl FnOnce(&[LibEntry]) -> R) -> R {
    with(|l| f(&l.entries))
}

/// The Graph of all notebooks from the index (graph_build_all): every notebook the index has read, the
/// editor's own replaced by `live` (its live NotebookInfo and Nav, unsaved links included) when given,
/// with the open page. Notebooks the index could not read or left out, and those left out of the Graph
/// (libindex_omits: the live one too), are skipped (links to them are neither edges nor broken).
/// graph_build_all's result.
pub fn libindex_graph(g: &mut GraphModel, live: Option<&GraphBook>, open: Option<(&str, u32)>) -> i32 {
    with(|l| {
        let folders = omit_folders(l).to_vec();
        let mut books: Vec<&GraphBook> = Vec::new();
        let mut skip: Vec<FixedStr<STORE_ID_LEN>> = Vec::new();
        if books.try_reserve(l.entries.len() + 1).is_err() {
            return -1;
        }
        for e in &l.entries {
            if live.is_some_and(|b| b.id == e.book.id) {
                continue;
            }
            if e.readable() && !e.left_out && !graph_omits(&e.book, &folders) {
                books.push(&e.book);
            } else if skip.try_reserve(1).is_ok() {
                skip.push(e.book.id);
            }
        }
        if let Some(b) = live {
            if !graph_omits(b, &folders) {
                books.push(b);
            } else if skip.try_reserve(1).is_ok() {
                skip.push(b.id);
            }
        }
        // Until a check has gone through every notebook, a link to a notebook the index has not read yet is
        // not known to be broken: it is left out, as one to a notebook it could not read is.
        if !l.complete {
            for b in &books {
                for link in &b.links {
                    if let GraphTo::Other(id, _) = link.to {
                        let known = live.is_some_and(|lb| lb.id == id) || find(l, id.as_str()).is_ok();
                        if !known && !skip.contains(&id) && skip.try_reserve(1).is_ok() {
                            skip.push(id);
                        }
                    }
                }
            }
        }
        graph_build_all(g, &books, &skip, open)
    })
}

/// What the index has for notebook id: its name, page order and names (None when it has nothing).
pub fn libindex_book(id: &str) -> Option<GraphBook> {
    with(|l| find(l, id).ok().map(|i| l.entries[i].book.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(tag: &str) -> std::path::PathBuf {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let r = base.join(format!("grimoire-libindex-{tag}-{}", std::process::id()));
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
    fn link(nb: &NotebookInfo, from: usize, target: &str, seed: u32) {
        let mut nav = Nav::default();
        nav_init(&mut nav);
        nav_load(nb.id.as_str(), &mut nav);
        assert_eq!(nav_link_set(&mut nav, nb.page_ids[from], seed, target), 0);
        assert_eq!(nav_save_links(nb.id.as_str(), &mut nav, nb), 0);
    }
    fn settle() -> usize {
        let mut n = 0;
        while libindex_step() {
            n += 1;
            assert!(n < 100_000, "the check never ends");
        }
        n
    }
    fn entry(id: &str) -> Option<LibEntry> {
        libindex_with(|e| e.iter().find(|e| e.book.id == id).cloned())
    }
    // What the index says of a notebook is what its files say.
    fn matches_files(nb: &NotebookInfo) -> bool {
        let mut info = NotebookInfo::default();
        let mut nav = Nav::default();
        nav_init(&mut nav);
        assert_eq!(store_load_info(nb.id.as_str(), &mut info), 0);
        nav_load(nb.id.as_str(), &mut nav);
        entry(nb.id.as_str()).is_some_and(|e| e.book == graph_book_of(&info, &nav) && e.stamps == libindex_stamps(nb.id.as_str()))
    }

    // A notebook or a folder left out of the Graph (2.0.0-beta.2, the owner's ruling of 2026-11-03): no node of
    // it in All, and a link to it no edge (the page with it linked only if it has other links); a folder covers
    // the folders inside it and notebooks moved in, follows a rename and goes with the folder; the cache keeps
    // the setting and says so in graph-index.ini.
    #[test]
    fn a_notebook_or_folder_left_out_of_the_graph_is_no_node_and_no_edge() {
        let r = root("omit");
        let (a, b, c) = (book("A", 3), book("B", 3), book("C", 2));
        link(&a, 0, &format!("n:{}:p{}", b.id.as_str(), b.page_ids[1]), 1); // A1 -> B2
        link(&a, 1, &format!("n:{}:p{}", c.id.as_str(), c.page_ids[0]), 2); // A2 -> C1 (only link of A2)
        link(&b, 2, "p1", 3); // B3 -> B1
        settle();
        let nodes = || {
            let mut g = GraphModel::default();
            assert_eq!(libindex_graph(&mut g, None, None), 0);
            let mut ids: Vec<String> = g.nodes.iter().map(|n| format!("{}:{}", g.books[n.book as usize].as_str(), n.pid)).collect();
            ids.sort();
            (ids, g.edges.len(), g.nodes.iter().map(|n| n.broken.max(0)).sum::<i32>())
        };
        let id = |nb: &NotebookInfo, k: usize| format!("{}:{}", nb.id.as_str(), nb.page_ids[k]);
        let all = nodes();
        assert_eq!(all.0.len(), 6, "{:?}", all.0);
        assert_eq!(all.1, 3);
        // C left out by its own setting: no C node, A2 (linked only with C) no node, nothing broken.
        assert_eq!(store_set_graph_omit(c.id.as_str(), true), 0);
        assert!(entry(c.id.as_str()).unwrap().book.omit, "notebook.ini's writer tells the index at once");
        let mut info = NotebookInfo::default();
        assert_eq!(store_load_info(c.id.as_str(), &mut info), 0);
        assert!(info.graph_omit);
        let (ids, edges, broken) = nodes();
        assert!(!ids.iter().any(|n| n.starts_with(c.id.as_str())), "{ids:?}");
        assert!(!ids.contains(&id(&a, 1)), "A2's only link went to C: it is not linked in All");
        assert!(ids.contains(&id(&a, 0)) && ids.contains(&id(&b, 1)));
        assert_eq!((edges, broken), (2, 0));
        assert_eq!(libindex_page_totals(), (6, 2), "a notebook left out is not counted");
        // The cache says so, and reads it back.
        assert!(flush_now());
        let text = std::fs::read(r.join("graph-index.ini")).unwrap();
        assert!(String::from_utf8_lossy(&text).contains("omit=1\n"));
        assert_eq!(libindex_parse(&text).unwrap(), libindex_with(|e| e.to_vec()));
        assert_eq!(store_set_graph_omit(c.id.as_str(), false), 0);
        assert_eq!(nodes().0.len(), 6);
        // A folder: B moved into School/2025, School left out, B gone with it (nested and moved in).
        assert_eq!(store_add_folder("School"), 0);
        assert_eq!(store_set_folder(b.id.as_str(), "School/2025"), 0);
        let gen = libindex_generation();
        assert_eq!(store_set_folder_graph_omit("School", true), 0);
        assert_ne!(libindex_generation(), gen, "the Graph builds again");
        assert_eq!(store_graph_omit_folders(), vec!["School".to_string()]);
        let (ids, _, _) = nodes();
        assert!(!ids.iter().any(|n| n.starts_with(b.id.as_str())), "{ids:?}");
        assert!(!ids.contains(&id(&a, 0)), "A1's only link went to B");
        // C moved into the nested folder: left out too.
        assert_eq!(store_set_folder(c.id.as_str(), "School/2025/Old"), 0);
        assert!(nodes().0.is_empty());
        // The folder renamed: its line follows; removed lines name no folder.
        assert_eq!(store_rename_folder("School", "Archive"), 0);
        assert_eq!(store_graph_omit_folders(), vec!["Archive".to_string()]);
        assert!(nodes().0.is_empty());
        assert_eq!(store_set_folder_graph_omit("Archive", false), 0);
        assert!(!r.join("graph-omit.ini").exists(), "nothing left out: no file");
        assert_eq!(nodes().0.len(), 6);
        // An empty folder left out, then removed: its line goes with it.
        assert_eq!(store_add_folder("Empty"), 0);
        assert_eq!(store_set_folder_graph_omit("Empty", true), 0);
        assert_eq!(store_remove_folder("Empty"), 0);
        assert!(store_graph_omit_folders().is_empty());
        std::fs::remove_dir_all(r).unwrap();
    }
    fn flush_now() -> bool {
        libindex_flush()
    }

    // Leave out of the Graph (or a move, or the trash) in the Library before anything read the index: the Graph
    // opened at once (mapview's libindex_load) draws what the files say now, not the cache's older word.
    #[test]
    fn a_change_before_the_index_is_read_shows_in_the_first_graph() {
        let r = root("pending");
        let (a, b, c) = (book("A", 2), book("B", 2), book("C", 2));
        link(&a, 0, &format!("n:{}:p{}", b.id.as_str(), b.page_ids[1]), 1); // A1 -> B2
        link(&a, 1, &format!("n:{}:p{}", c.id.as_str(), c.page_ids[0]), 2); // A2 -> C1
        settle();
        assert!(flush_now());
        // A new session: the index not read yet.
        let other = r.with_extension("other");
        assert_eq!(store_init(other.to_str().unwrap()), 0);
        assert_eq!(libindex_generation(), 0, "(a fresh state)");
        assert_eq!(store_init(r.to_str().unwrap()), 0);
        assert!(libindex_building().is_none(), "nothing read yet");
        assert_eq!(store_set_graph_omit(b.id.as_str(), true), 0);
        assert_eq!(store_delete(c.id.as_str()), 0);
        assert!(libindex_building().is_none(), "a note reads nothing");
        libindex_load();
        assert!(entry(b.id.as_str()).unwrap().book.omit, "the note made before the load");
        assert!(entry(c.id.as_str()).is_none(), "trashed before the load");
        let mut g = GraphModel::default();
        assert_eq!(libindex_graph(&mut g, None, None), 0);
        assert!(g.nodes.is_empty(), "A's links went to B (left out) and C (trashed)");
        let _ = std::fs::remove_dir_all(&r);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn built_from_nothing_then_read_back_without_reading_a_notebook() {
        let r = root("build");
        let (a, b) = (book("A", 3), book("B", 2));
        link(&a, 0, "p2", 1);
        link(&a, 1, &format!("n:{}:p{}", b.id.as_str(), b.page_ids[1]), 2);
        link(&b, 0, "index", 3);
        // (page names too: a notebook read from its files keeps its names beside its links)
        for (nb, k, text) in [(&a, 2, "Loose"), (&b, 0, "First")] {
            let mut nav = Nav::default();
            nav_init(&mut nav);
            nav_load(nb.id.as_str(), &mut nav);
            assert_eq!(nav_set_name(&mut nav, nb.page_ids[k], text), 0);
            assert_eq!(nav_save(nb.id.as_str(), &mut nav, nb), 0);
        }
        assert!(libindex_building().is_none(), "nothing known before the first tick");
        assert!(libindex_step(), "the first tick loads (no file: building)");
        assert_eq!(libindex_building(), Some((0, 2)), "the notebooks are listed at once");
        settle();
        assert!(libindex_complete() && libindex_building().is_none());
        assert!(matches_files(&a) && matches_files(&b));
        assert_eq!(entry(a.id.as_str()).unwrap().book.names, vec![(a.page_ids[2], "Loose".to_string())]);
        assert_eq!(entry(b.id.as_str()).unwrap().book.links.len(), 1);
        assert_eq!(libindex_page_totals(), (5, 2));
        let text = std::fs::read(r.join("graph-index.ini")).unwrap();
        assert_eq!(libindex_parse(&text).unwrap(), libindex_with(|e| e.to_vec()));
        // A new session reads the file and, its stamps unchanged, no notebook's files.
        let reads = libindex_reads();
        let other = r.with_extension("other");
        assert_eq!(store_init(other.to_str().unwrap()), 0);
        assert_eq!(libindex_generation(), 0);
        assert_eq!(store_init(r.to_str().unwrap()), 0);
        settle();
        assert_eq!(libindex_reads(), 0, "(a fresh state: {reads} reads before)");
        assert!(matches_files(&a) && matches_files(&b));
        let _ = std::fs::remove_dir_all(&r);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn a_save_needs_no_read_and_another_versions_change_is_read_once() {
        let r = root("save");
        let (a, b) = (book("A", 2), book("B", 2));
        settle();
        let reads = libindex_reads();
        // This version's saves: the entry is current at once, with the new files' stamps.
        let gen = libindex_generation();
        link(&a, 0, "p2", 1);
        let mut nav = Nav::default();
        nav_init(&mut nav);
        nav_load(a.id.as_str(), &mut nav);
        assert_eq!(nav_set_name(&mut nav, a.page_ids[1], "Second"), 0);
        assert_eq!(nav_save(a.id.as_str(), &mut nav, &a), 0);
        let mut a2 = a.clone();
        assert_eq!(store_add_page(&mut a2, 2, PAPER_BLANK), 0);
        assert_ne!(libindex_generation(), gen);
        assert!(matches_files(&a2));
        libindex_revalidate();
        settle();
        assert_eq!(libindex_reads(), reads, "a save's stamps need no read");
        // Another version writes B's links.ini behind this one's back: one read of B, at the next check.
        let line = format!("link.{}=00000001,n:{}:p{}\n", b.page_ids[0], a.id.as_str(), a.page_ids[0]);
        std::fs::write(r.join(format!("notebooks/{}/links.ini", b.id.as_str())), line).unwrap();
        libindex_revalidate();
        settle();
        assert_eq!(libindex_reads(), reads + 1);
        assert!(matches_files(&b));
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_notebook_deleted_and_restored() {
        let r = root("trash");
        let (a, b) = (book("A", 2), book("B", 2));
        settle();
        assert!(entry(b.id.as_str()).is_some());
        // Its own delete takes it out at once.
        let gen = libindex_generation();
        assert_eq!(store_delete(b.id.as_str()), 0);
        assert!(entry(b.id.as_str()).is_none());
        assert_ne!(libindex_generation(), gen);
        // Brought back behind the index's back (another version's restore): the next check reads it.
        let trash = r.join("trash");
        let back = std::fs::read_dir(&trash).unwrap().flatten().find(|e| e.file_name().to_string_lossy().contains(b.id.as_str())).unwrap();
        std::fs::rename(back.path(), r.join(format!("notebooks/{}", b.id.as_str()))).unwrap();
        libindex_revalidate();
        settle();
        assert!(matches_files(&b) && matches_files(&a));
        // Gone behind its back: dropped when a check ends.
        std::fs::remove_dir_all(r.join(format!("notebooks/{}", b.id.as_str()))).unwrap();
        libindex_revalidate();
        settle();
        assert!(entry(b.id.as_str()).is_none());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn the_link_cap_leaves_a_notebook_out_until_there_is_room() {
        let r = root("cap");
        let (a, b) = (book("A", 2), book("B", 2));
        settle();
        // B's links pass the cap with A's: B is left out, A kept.
        let fill = |id: &FixedStr<STORE_ID_LEN>, n: usize| {
            let mut s = String::from("# Grimoire: links drawn on pages\n");
            for k in 0..n {
                s.push_str(&format!("link.1={:08x},p2\n", k + 1));
            }
            std::fs::write(r.join(format!("notebooks/{}/links.ini", id.as_str())), s).unwrap();
        };
        fill(&a.id, LIB_LINKS_MAX - 10);
        fill(&b.id, 20);
        libindex_revalidate();
        settle();
        // (one link each: page 1 to page 2 once, whatever the seed)
        assert_eq!(libindex_left_out(), 0);
        let many = |id: &FixedStr<STORE_ID_LEN>, n: usize| {
            let mut e = entry(id.as_str()).unwrap();
            e.book.links = (0..n as u32).map(|k| GraphLink { src: 1, to: GraphTo::Page(k + 2) }).collect();
            with(|l| put(l, e));
        };
        many(&a.id, LIB_LINKS_MAX - 10);
        many(&b.id, 20);
        assert_eq!(libindex_left_out(), 1);
        let left = entry(b.id.as_str()).unwrap();
        assert!(left.left_out && left.stamps == [0; 3] && left.book.links.is_empty());
        let text = libindex_text(&libindex_with(|e| e.to_vec()));
        let line = format!("nb={},0000000000000000,0000000000000000,0000000000000000\n", b.id.as_str());
        assert!(String::from_utf8_lossy(&text).contains(&line));
        // A frees room: B is taken in at the next check.
        many(&a.id, 5);
        assert!(!libindex_complete());
        settle();
        assert_eq!(libindex_left_out(), 0);
        assert!(matches_files(&b));
        // The file itself refuses more than the cap.
        let mut big = format!("nb={},1,1,1\npages=1,2\n", a.id.as_str()).into_bytes();
        for _ in 0..=LIB_LINKS_MAX {
            big.extend_from_slice(b"link=1,p2\n");
        }
        assert!(libindex_parse(&big).is_none());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_corrupt_index_is_built_again() {
        assert!(libindex_parse(b"name=X\n").is_none(), "a line before any notebook");
        assert!(libindex_parse(b"nb=A,1,1\n").is_none(), "two stamps");
        assert!(libindex_parse(b"nb=A,1,1,1\nnb=A,2,2,2\n").is_none(), "a notebook twice");
        assert!(libindex_parse(b"nb=../x,1,1,1\n").is_none(), "an id that is no notebook's");
        assert!(libindex_parse(b"nb=A,1,1,1\npages=1,02\n").is_none(), "a leading zero");
        assert!(libindex_parse(b"nb=A,1,1,1\nfuture=1\n").is_none(), "a line it does not know");
        assert!(libindex_parse(b"nb=A,1,1,1\nlink=1,somewhere\n").is_none(), "a target it does not know");
        assert!(libindex_parse(b"nb=A,0,0,0\npages=1\n").is_none(), "lines under an entry read again");
        let e = libindex_parse(b"# c\r\nnb=B,ff,0,1\r\nname=Bee\npages=3,1\npname=3,Third\nlink=3,n:A:p1\nlink=3,n:B:p1\nnb=A,0,0,0\n").unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].book.id.as_str(), "A");
        assert!(!e[0].readable());
        assert_eq!(e[1].book.pages, vec![3, 1]);
        assert_eq!(e[1].book.names, vec![(3, "Third".to_string())]);
        // (a link to its own notebook written as another's is a page of its own)
        assert_eq!(e[1].book.links, vec![GraphLink { src: 3, to: GraphTo::Page(1) }, GraphLink { src: 3, to: GraphTo::Other(FixedStr::from_str("A"), 1) }]);
        let r = root("corrupt");
        let a = book("A", 2);
        std::fs::write(r.join("graph-index.ini"), b"nb=garbage\n").unwrap();
        settle();
        assert!(matches_files(&a));
        assert!(libindex_parse(&std::fs::read(r.join("graph-index.ini")).unwrap()).is_some());
        let _ = std::fs::remove_dir_all(&r);
    }

    // A link to a notebook the index has not read yet is not broken (the first check is still going); once every
    // notebook is checked, a link to one that is gone is.
    #[test]
    fn a_link_to_a_notebook_not_read_yet_is_not_broken() {
        let r = root("unknown");
        let (a, b) = (book("A", 2), book("B", 2));
        // (B's folder renamed so it sorts after A's: the check reads A first)
        let late = "zzzzzzzz-late";
        std::fs::rename(r.join(format!("notebooks/{}", b.id.as_str())), r.join(format!("notebooks/{late}"))).unwrap();
        link(&a, 0, &format!("n:{late}:p{}", b.page_ids[1]), 1);
        link(&a, 0, "p2", 2);
        libindex_step(); // (load: no file, the notebooks listed)
        while entry(a.id.as_str()).is_none() {
            libindex_step();
        }
        assert!(entry(late).is_none());
        let mut g = GraphModel::default();
        assert_eq!(libindex_graph(&mut g, None, None), 0);
        assert!(g.nodes.iter().all(|n| n.broken == 0), "B is not known to be gone");
        settle();
        assert!(entry(late).is_some());
        assert_eq!(store_delete(late), 0);
        libindex_revalidate();
        settle();
        assert_eq!(libindex_graph(&mut g, None, None), 0);
        assert_eq!(g.nodes.iter().map(|n| n.broken).sum::<i32>(), 1);
        // (and before a check after a restart: B's link target unknown again until the check ends)
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn a_tick_reads_at_most_one_notebook() {
        let r = root("bounded");
        let mut ids = Vec::new();
        for k in 0..20 {
            ids.push(book(&format!("N{k}"), 1).id);
        }
        libindex_step(); // (load)
        libindex_step(); // (list)
        let mut ticks = 0;
        loop {
            let before = libindex_reads();
            let more = libindex_step();
            assert!(libindex_reads() - before <= 1, "one notebook a tick");
            ticks += 1;
            if let Some((at, of)) = libindex_building() {
                assert!(at <= of && of == 20);
            }
            if !more {
                break;
            }
        }
        assert_eq!(libindex_reads(), 20);
        assert!(ticks >= 20);
        // A check over an index that is current reads nothing, LIB_STATS_PER_STEP notebooks a tick.
        libindex_revalidate();
        let n = settle();
        assert_eq!(libindex_reads(), 20);
        assert!(n >= 20 / LIB_STATS_PER_STEP, "{n} ticks");
        let _ = std::fs::remove_dir_all(&r);
    }
}
