//! annotate.h: "Annotate this page in Grimoire", from KOReader (docs/FORMAT.md, Annotated pages).
//! KOReader's Grimoire plugin (kindle/koreader/grimoire.koplugin) draws the page its reader is on
//! into a picture, writes it with a request into grimoire/import/.annotate/ (a dot-folder the folder
//! import skips), and hands over to Grimoire with --annotate, which kindle/app/grimoire.sh passes on
//! (GRIMOIRE_ANNOTATE=1) only when KOReader hosts Grimoire. At startup Grimoire finds the notebook
//! for that book (notebook.ini source=), or makes one named after the book's title, and the page for
//! that book page (the notebook's source.ini), or adds one: a document page whose paper is the
//! picture, among the book's other pages in book order. The editor opens on it, and the request goes.
//!
//! grimoire.log goes into public bug reports: the book's title, its file and its key are never
//! logged here, only notebook ids, page numbers and counts.

use crate::prelude::*;

use std::cell::RefCell;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

// The folders' and files' names, as macros: the C joins them to its format strings by adjacent
// string literals ("%s/" ANNOTATE_DIR), which only a literal can do here (concat!).
macro_rules! annotate_dir {
    () => {
        "import/.annotate"
    };
}
macro_rules! annotate_request {
    () => {
        "request.ini"
    };
}
macro_rules! book_dir_name {
    () => {
        "import/.book"
    };
}
macro_rules! book_request {
    () => {
        "request.ini"
    };
}
macro_rules! work_prefix {
    () => {
        ".annotate-"
    };
}

/// the requests' folder, in the data root
pub const ANNOTATE_DIR: &str = annotate_dir!();
/// the request, in it
pub const ANNOTATE_REQUEST: &str = annotate_request!();
/// a request older (or newer) than this is not opened
pub const ANNOTATE_STALE_S: i64 = 300;
/// the most of a title a request is read with
pub const ANNOTATE_TITLE_LEN: usize = 512;

/// a request is a few lines
const REQUEST_MAX: usize = 4096;
/// the most of a page picture read: a template's, which the fitting job reads at most (no page's is
/// near it)
const PICTURE_MAX: usize = 16 << 20;
/// notebooks/.annotate-<id>/: a notebook being made
const WORK_PREFIX: &str = work_prefix!();
/// a picture's file name is shorter than this (AnnotateRequest.picture)
const PICTURE_LEN: usize = 64;

/// A request (request.ini), as annotate_parse reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnnotateRequest {
    /// 1
    pub version: i32,
    /// the book's key: KOReader's partial MD5 of its file
    pub book: FixedStr<STORE_SOURCE_LEN>,
    /// the book's title, for a new notebook's name (never logged)
    pub title: FixedStr<ANNOTATE_TITLE_LEN>,
    /// the page the reader is on, from 1, and how many there are
    pub page: i32,
    pub pages: i32,
    /// "paged" (PDF, DjVu, comics) or "reflowable" (EPUB, FB2, TXT)
    pub kind: FixedStr<16>,
    /// the picture's file name, in the requests' folder
    pub picture: FixedStr<PICTURE_LEN>,
    /// when it was written (unix seconds); 0 when not given
    pub time: i64,
}

// ---- the request ----

/// The text up to its first NUL: the C reads a request as a string, and stops there.
fn cstr(text: &[u8]) -> &[u8] {
    match text.iter().position(|&c| c == 0) {
        Some(n) => &text[..n],
        None => text,
    }
}

/// The length of the line at the start of text (without its newline), and how far to step to the
/// next one (past the newline, if there is one).
fn next_line(text: &[u8]) -> (usize, usize) {
    match text.iter().position(|&c| c == b'\n') {
        Some(n) => (n, n + 1),
        None => (text.len(), text.len()),
    }
}

/// A whole number of at least lo, written as digits only; -1 for anything else.
fn number(v: &[u8], lo: i64, hi: i64) -> i64 {
    if v.is_empty() {
        return -1;
    }
    let mut x: i64 = 0;
    for &c in v {
        if !c.is_ascii_digit() {
            return -1;
        }
        x = x * 10 + (c - b'0') as i64;
        if x > hi {
            return -1;
        }
    }
    if x < lo {
        -1
    } else {
        x
    }
}

/// store_source_ok on bytes that need not be text: a key is letters, digits, '-' and '_', so bytes
/// that are not UTF-8 are none.
fn book_key_ok(v: &[u8]) -> bool {
    std::str::from_utf8(v).is_ok_and(store_source_ok)
}

/// A picture's name in the requests' folder: a plain file name ending in .png (letters, digits, '-',
/// '_' and '.', not starting with a dot), so it cannot name anything outside the folder.
fn picture_name_ok(s: &[u8]) -> bool {
    let n = s.len();
    if !(5..PICTURE_LEN).contains(&n) || s[0] == b'.' || &s[n - 4..] != b".png" {
        return false;
    }
    s.iter().all(|&c| c.is_ascii_digit() || c.is_ascii_lowercase() || c.is_ascii_uppercase() || c == b'-' || c == b'_' || c == b'.')
}

fn word_ok(s: &[u8], cap: usize) -> bool {
    let n = s.len();
    if n == 0 || n >= cap {
        return false;
    }
    s.iter().all(|&c| c.is_ascii_lowercase())
}

/// a request's line is read up to this many bytes (a longer one: its start)
const LINE_MAX: usize = ANNOTATE_TITLE_LEN + 16;

/// Reads a request's text. 0, or -1 and why (for the log: never the title).
/// (text: the file's bytes; a title need not be UTF-8, and annotate_name mends it.)
pub fn annotate_parse(text: &[u8], r: &mut AnnotateRequest, why: &mut &'static str) -> i32 {
    *r = AnnotateRequest::default();
    r.version = 1; // (a request without the line: a hand-made one)
    *why = "no request";
    let text = cstr(text);
    let (mut page, mut pages): (i64, i64) = (-1, -1);
    let mut p = 0;
    while p < text.len() {
        let (n, step) = next_line(&text[p..]);
        let k = n.min(LINE_MAX - 1); // (a longer line: its start)
        let mut line = &text[p..p + k];
        if let Some((&b'\r', head)) = line.split_last() {
            line = head;
        }
        p += step;
        let Some(eq) = line.iter().position(|&c| c == b'=') else { continue };
        if line[0] == b'#' {
            continue;
        }
        let (key, v) = (&line[..eq], &line[eq + 1..]);
        if key == b"version" {
            r.version = number(v, 0, 1000000) as i32;
        } else if key == b"book" {
            if !book_key_ok(v) {
                *why = "its book key is not one";
                return -1;
            }
            r.book.set_bytes(v);
        } else if key == b"title" {
            r.title.set_bytes(v); // (cut between characters later: annotate_name)
        } else if key == b"page" {
            page = number(v, 1, 10000000);
        } else if key == b"pages" {
            pages = number(v, 1, 10000000);
        } else if key == b"kind" {
            if word_ok(v, FixedStr::<16>::CAP) {
                r.kind.set_bytes(v);
            }
        } else if key == b"picture" {
            if !picture_name_ok(v) {
                *why = "its picture's name is not a plain .png file name";
                return -1;
            }
            r.picture.set_bytes(v);
        } else if key == b"time" {
            let t = number(v, 0, 1i64 << 40);
            r.time = if t > 0 { t } else { 0 };
        } // (a key this version does not know: a newer plugin's, left to it)
    }
    if r.version != 1 {
        *why = "a version of request this Grimoire does not know (from a newer plugin)";
        return -1;
    }
    if r.book.is_empty() || r.picture.is_empty() {
        *why = if !r.book.is_empty() { "no picture named" } else { "no book key" };
        return -1;
    }
    if page < 1 || pages < page {
        *why = if page < 1 { "no page number" } else { "no page count, or one under the page number" };
        return -1;
    }
    r.page = page as i32;
    r.pages = pages as i32;
    *why = "";
    0
}

/// Whether a request is too old (or too new: the clock was set back) to open at `now`: its time, or
/// file_time (the file's own) when it gives none, more than ANNOTATE_STALE_S away.
pub fn annotate_too_old(r: &AnnotateRequest, now: i64, file_time: i64) -> bool {
    let t = if r.time > 0 { r.time } else { file_time };
    t <= 0 || now - t > ANNOTATE_STALE_S || t - now > ANNOTATE_STALE_S
}

// ---- a notebook's source.ini: the book it annotates, and which page holds which book page ----

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnnotatePage {
    /// the book page, of how many the book had then (a reflowable book's change with its layout:
    /// page 12 of 300 and page 12 of 340 are two pages)
    pub page: i32,
    pub pages: i32,
    /// the notebook's page that holds it
    pub pid: u32,
}

#[derive(Clone, Debug, Default)]
pub struct AnnotateMap {
    pub book: FixedStr<STORE_SOURCE_LEN>,
    pub kind: FixedStr<16>,
    /// the C's e, n, cap
    pub e: Vec<AnnotatePage>,
    /// lines this version does not know, written back as they were
    pub extra: FixedStr<1024>,
}

/// Whether nb lists page pid, and where (its index in the page list).
fn in_notebook(nb: &NotebookInfo, pid: u32, index: Option<&mut i32>) -> bool {
    match nb.page_ids.iter().position(|&id| id == pid) {
        Some(i) => {
            if let Some(index) = index {
                *index = i as i32;
            }
            true
        }
        None => false,
    }
}

pub fn annotate_map_add(m: &mut AnnotateMap, page: i32, pages: i32, pid: u32) -> i32 {
    for e in m.e.iter_mut() {
        if e.page == page && e.pages == pages {
            e.pid = pid;
            return 0;
        }
    }
    if m.e.try_reserve(1).is_err() {
        return -1;
    }
    m.e.push(AnnotatePage { page, pages, pid });
    0
}

/// Lines of a form this version does not read, kept to be written back (whole lines, while they fit).
fn keep_line(m: &mut AnnotateMap, line: &[u8]) {
    let have = m.extra.len();
    let need = line.len() + 1;
    if have + need < FixedStr::<1024>::CAP {
        let mut t = m.extra.as_bytes().to_vec();
        t.extend_from_slice(line);
        t.push(b'\n');
        m.extra.set_bytes(&t);
    }
}

/// Reads source.ini's text into m (NULL: an empty map). -1 out of memory.
pub fn annotate_map_parse(text: Option<&[u8]>, m: &mut AnnotateMap) -> i32 {
    *m = AnnotateMap::default();
    let text = cstr(text.unwrap_or(&[]));
    let mut p = 0;
    while p < text.len() {
        let (n, step) = next_line(&text[p..]);
        let whole = n < 256; // char line[256]
        let k = if whole { n } else { 255 };
        let mut line = &text[p..p + k];
        if let Some((&b'\r', head)) = line.split_last() {
            line = head;
        }
        p += step;
        if line.is_empty() || line[0] == b'#' || !whole {
            continue;
        }
        let eq = line.iter().position(|&c| c == b'=');
        let (mut page, mut pages, mut pid): (i64, i64, i64) = (-1, -1, -1);
        if let Some(eq) = eq {
            if line.starts_with(b"page.") {
                // page.<book page>.<page count>=<page id>
                let head = &line[5..eq];
                if let Some(dot) = head.iter().position(|&c| c == b'.') {
                    page = number(&head[..dot], 1, 10000000);
                    pages = number(&head[dot + 1..], 1, 10000000);
                }
                pid = number(&line[eq + 1..], 1, 0x7ffffffe);
            }
        }
        if eq.is_some() && line.starts_with(b"book=") && book_key_ok(&line[5..]) {
            m.book.set_bytes(&line[5..]);
        } else if eq.is_some() && line.starts_with(b"kind=") && word_ok(&line[5..], FixedStr::<16>::CAP) {
            m.kind.set_bytes(&line[5..]);
        } else if page >= 1 && pages >= page && pid >= 1 {
            if annotate_map_add(m, page as i32, pages as i32, pid as u32) != 0 {
                annotate_map_free(m);
                return -1;
            }
        } else {
            keep_line(m, line);
        }
    }
    0
}

pub fn annotate_map_free(m: &mut AnnotateMap) {
    m.e = Vec::new();
}

/// Book order: by the place in the book (page - 1 out of pages), then the page number, then the count.
fn book_cmp(pa: i32, na: i32, pb: i32, nb: i32) -> i32 {
    let x = (pa - 1) as i64 * nb as i64;
    let y = (pb - 1) as i64 * na as i64;
    if x != y {
        return if x < y { -1 } else { 1 };
    }
    if pa != pb {
        return if pa < pb { -1 } else { 1 };
    }
    if na < nb {
        -1
    } else {
        (na > nb) as i32
    }
}

fn cmp_entry(x: &AnnotatePage, y: &AnnotatePage) -> std::cmp::Ordering {
    book_cmp(x.page, x.pages, y.page, y.pages).cmp(&0)
}

/// The text to write: entries for pages no longer in nb (deleted) are left out. Caller frees.
pub fn annotate_map_text(m: &AnnotateMap, nb: &NotebookInfo) -> Option<Vec<u8>> {
    let mut e: Vec<AnnotatePage> = Vec::new();
    let cap = 256 + m.e.len() * 48 + FixedStr::<1024>::CAP;
    let mut t: Vec<u8> = Vec::new();
    if e.try_reserve_exact(m.e.len().max(1)).is_err() || t.try_reserve_exact(cap).is_err() {
        return None;
    }
    for x in &m.e {
        if in_notebook(nb, x.pid, None) {
            e.push(*x); // a deleted page's goes
        }
    }
    // (the entries are one per book page: no two compare equal, so the sort's stability is moot)
    e.sort_by(cmp_entry);
    t.extend_from_slice(b"# Grimoire: the book this notebook annotates, from KOReader (docs/FORMAT.md)\n");
    if !m.book.is_empty() {
        t.extend_from_slice(b"book=");
        t.extend_from_slice(m.book.as_bytes());
        t.push(b'\n');
    }
    if !m.kind.is_empty() {
        t.extend_from_slice(b"kind=");
        t.extend_from_slice(m.kind.as_bytes());
        t.push(b'\n');
    }
    for x in &e {
        t.extend_from_slice(cformat!("page.%d.%d=%u\n", x.page, x.pages, x.pid).as_bytes());
    }
    t.extend_from_slice(m.extra.as_bytes());
    Some(t)
}

/// The page that holds book page `page` of `pages` (an id in nb's page list), or 0.
pub fn annotate_map_find(m: &AnnotateMap, nb: &NotebookInfo, page: i32, pages: i32) -> u32 {
    for x in &m.e {
        if x.page == page && x.pages == pages && in_notebook(nb, x.pid, None) {
            return x.pid;
        }
    }
    0
}

/// Where a new page for book page `page` of `pages` goes in nb's page list: just before the first
/// page that holds a later book page (by its place in the book: page 12 of 300 before page 50 of
/// 400), so the pages written after an annotated page (notes on it) stay with it; at the end when no
/// page holds a later one.
pub fn annotate_insert_at(nb: &NotebookInfo, m: &AnnotateMap, page: i32, pages: i32) -> i32 {
    for (i, id) in nb.page_ids.iter().enumerate() {
        for x in &m.e {
            if x.pid == *id && book_cmp(x.page, x.pages, page, pages) > 0 {
                return i as i32;
            }
        }
    }
    nb.npages()
}

fn map_path(nbdir: &str, buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, "%s/source.ini", nbdir);
}

/// A notebook's source.ini (none: an empty map). -1 when it is there but does not read: then
/// nothing may be written over it (its pages would lose their book pages).
fn map_load(nbdir: &str, m: &mut AnnotateMap) -> i32 {
    let mut path = String::new();
    map_path(nbdir, &mut path, 800);
    let t = read_file(&path);
    if t.is_none() && exists(&path) {
        return -1;
    }
    annotate_map_parse(t.as_deref(), m)
}

/// Notebook id's source.ini into m (none: an empty map). -1 when it is there but does not read, or
/// out of memory (m is then empty too).
pub fn annotate_map_load(id: &str, m: &mut AnnotateMap) -> i32 {
    let mut dir = String::new();
    *m = AnnotateMap::default();
    if store_notebook_dir(id, &mut dir, 600) != 0 {
        return -1;
    }
    map_load(&dir, m)
}

fn map_save(nbdir: &str, m: &AnnotateMap, nb: &NotebookInfo) -> i32 {
    let mut path = String::new();
    map_path(nbdir, &mut path, 800);
    match annotate_map_text(m, nb) {
        Some(t) => write_file_atomic(&path, &t),
        None => -1,
    }
}

// ---- names ----

/// The length of the UTF-8 character at s (1 to 4 bytes), or 0 for a byte that starts none: a stray
/// continuation byte, an overlong form, a surrogate, past U+10FFFF, or one cut short.
fn utf8_len(s: &[u8]) -> usize {
    let c = s[0];
    if c < 0x80 {
        return 1;
    }
    let n = if (0xF0..=0xF4).contains(&c) {
        4
    } else if c >= 0xE0 {
        3
    } else if (0xC2..=0xDF).contains(&c) {
        2
    } else {
        0
    };
    if n == 0 || (c >= 0xF0 && n != 4) {
        return 0;
    }
    for i in 1..n {
        // (the C string's end is a 0 byte, which is no continuation either)
        if s.get(i).is_none_or(|&b| (b & 0xC0) != 0x80) {
            return 0;
        }
    }
    if n == 3 && ((c == 0xE0 && s[1] < 0xA0) || (c == 0xED && s[1] >= 0xA0)) {
        return 0; // overlong, surrogate
    }
    if n == 4 && ((c == 0xF0 && s[1] < 0x90) || (c == 0xF4 && s[1] >= 0x90)) {
        return 0; // overlong, too big
    }
    n
}

/// A new notebook's name: the title on one line (control characters as spaces, one space between two
/// words, none first or last), without broken UTF-8, cut to the name limit between two characters,
/// never inside one. Every other character stays, also one Grimoire's fonts cannot draw (Chinese,
/// Japanese, Korean): the screens leave it out where they draw the name (ui_name_label), and the
/// PDF's file name keeps it. Empty when nothing is left (the caller names it Untitled).
// Every character of the title is kept, also those Grimoire's fonts cannot draw (Chinese, Japanese,
// Korean, most Vietnamese letters): the name is also the notebook's PDF's in grimoire/PDFs, which
// KOReader lists. Only the screens leave those out as they draw it (ui_name_label).
pub fn annotate_name(title: &[u8], out: &mut String, cap: usize) {
    if cap == 0 {
        return;
    }
    let title = cstr(title);
    let mut buf: Vec<u8> = Vec::new();
    let mut s = 0;
    while s < title.len() {
        let n = utf8_len(&title[s..]);
        if n == 0 {
            // broken: left out
            s += 1;
            continue;
        }
        // Spaces and control characters: one space between two words (none first or last).
        let space = if n == 1 { title[s] <= b' ' || title[s] == 0x7F } else { n == 2 && title[s] == 0xC2 && title[s + 1] < 0xA0 };
        if space {
            if !buf.is_empty() && buf[buf.len() - 1] != b' ' && buf.len() + 1 < cap {
                buf.push(b' ');
            }
        } else {
            if buf.len() + n >= cap {
                break; // the next character would not fit whole
            }
            buf.extend_from_slice(&title[s..s + n]);
        }
        s += n;
    }
    while buf.last() == Some(&b' ') {
        buf.pop();
    }
    // (whole characters only went in, each checked by utf8_len)
    *out = String::from_utf8(buf).unwrap_or_default();
}

/// The name with " 2", " 3"... added when a notebook in all has it already, as the Library names new
/// notebooks: two notebooks of one name would share their PDF in grimoire/PDFs.
fn unique_name(base: &str, all: &[NotebookInfo], out: &mut String, cap: usize) {
    let mut cut = String::new();
    strlcpy_q(out, base, cap);
    for k in 2..1000 {
        let taken = all.iter().any(|nb| nb.name.as_bytes() == out.as_bytes());
        if !taken {
            return;
        }
        let mut tail = String::new();
        let tl = csnprintf!(&mut tail, 16, " %d", k) as usize;
        annotate_name(base.as_bytes(), &mut cut, if cap - tl < STORE_NAME_LEN { cap - tl } else { STORE_NAME_LEN });
        csnprintf!(out, cap, "%s%s", &cut, &tail);
    }
}

// ---- the picture ----

fn be32(p: &[u8]) -> u32 {
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | p[3] as u32
}

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB88320u32 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

fn crc32_png(p: &[u8]) -> u32 {
    let mut c = 0xFFFFFFFFu32;
    for &b in p {
        c = CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFFFFFFu32
}

/// Whether data is a PNG whose chunks are all there, in order, and whose critical ones (IHDR, PLTE,
/// IDAT, IEND) check out against their CRCs, with nothing after IEND: a file cut short or damaged is
/// caught before anything is made of it. 0 with its size, or -1 and why.
pub fn annotate_png_check(data: &[u8], w: &mut i32, h: &mut i32, why: &mut &'static str) -> i32 {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    let len = data.len();
    *why = "not a PNG";
    if len < 8 || data[..8] != SIG {
        return -1;
    }
    let (mut idat, mut iend) = (false, false);
    let mut chunks = 0;
    let mut at = 8;
    while at < len && !iend {
        *why = "cut short";
        if len - at < 12 {
            return -1;
        }
        let n = be32(&data[at..]) as usize;
        let type_ = &data[at + 4..at + 8];
        if n > len - at - 12 {
            return -1;
        }
        if !type_.iter().all(|&c| c.is_ascii_uppercase() || c.is_ascii_lowercase()) {
            *why = "damaged (a chunk that is none)";
            return -1;
        }
        let critical = type_[0] & 0x20 == 0;
        if critical && crc32_png(&data[at + 4..at + 8 + n]) != be32(&data[at + 8 + n..]) {
            *why = "damaged (a checksum does not match)";
            return -1;
        }
        if chunks == 0 && (type_ != b"IHDR" || n != 13) {
            *why = "damaged (no header first)";
            return -1;
        }
        idat |= type_ == b"IDAT";
        iend = type_ == b"IEND";
        at += 12 + n;
        chunks += 1;
    }
    if !iend || !idat {
        *why = "cut short";
        return -1;
    }
    if at != len {
        // (more after its end: two writes of the file mixed, say)
        *why = "damaged (something after its end)";
        return -1;
    }
    bgimage_check(data, len, w, h, why) // a picture Grimoire takes: its size, what it takes to decode
}

// ---- at startup ----

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnnotateOutcome {
    /// no request
    #[default]
    None = 0, // ANNOTATE_NONE
    /// a request this start was not asked to open, or too old: deleted unopened
    Stale = 1, // ANNOTATE_STALE
    /// the page was there already
    Opened = 2, // ANNOTATE_OPENED
    /// a page was added, in a notebook made for the book or in its own
    Made = 3, // ANNOTATE_MADE
    /// nothing was made (the notebook stays as it was); msg says why
    Failed = 4, // ANNOTATE_FAILED
}

/// Why nothing was made (FAILED), for the toast the caller shows.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnnotateWhy {
    /// see grimoire.log (the request does not read, a notebook's files do not)
    #[default]
    Log = 0, // ANNOTATE_WHY_LOG
    /// the picture is missing, cut short, damaged, or no picture Grimoire takes
    Picture = 1, // ANNOTATE_WHY_PICTURE
    /// a file could not be written: the storage is probably full
    Storage = 2, // ANNOTATE_WHY_STORAGE
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AnnotateResult {
    pub outcome: AnnotateOutcome,
    /// OPENED, MADE: the notebook to open, and its page that holds the book page (its last_page
    /// says so too)
    pub nb: FixedStr<STORE_ID_LEN>,
    pub pid: u32,
    /// FAILED
    pub why: AnnotateWhy,
    /// the book page asked for (OPENED, MADE, FAILED)
    pub page: i32,
    pub pages: i32,
}

// ---- files ----

/// access(path, F_OK) == 0
fn exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

/// lstat succeeds and the path is a file of its own (never a link to one elsewhere)
fn is_own_file(path: &str) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

fn fsync_file(path: &str) {
    let Ok(f) = fs::File::open(path) else { return };
    // SAFETY: fsync only takes a descriptor, which f owns and keeps open for the call.
    unsafe { libc::fsync(f.as_raw_fd()) };
}

fn sync_dir(dir: &str) {
    let Ok(f) = fs::File::open(dir) else { return };
    // SAFETY: fsync only takes a descriptor, which f owns and keeps open for the call.
    unsafe { libc::fsync(f.as_raw_fd()) };
}

/// errno = v, so that a failed call's errno can be told from an old one (bgimage_read leaves EFBIG).
fn set_errno(v: i32) {
    // SAFETY: the C library's errno slot belongs to the calling thread and lives as long as it does.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        *libc::__errno_location() = v;
    }
    // SAFETY: the same.
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe {
        *libc::__error() = v;
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos", target_os = "ios", target_os = "freebsd")))]
    let _ = v;
}

/// <bgdir>/<id, 6 hex digits>.source.png: a picture of another size than its notebook's pages,
/// waiting for BG_TEMPLATE to fit it (bgwork_template.c removes it then).
fn source_file(bgdir: &str, id: u32, buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, "%s/%06x.source.png", bgdir, id & 0xFFFFFF);
}

/// Puts the request's picture (at src, the file's bytes in png, w x h) where it becomes picture id
/// of a notebook whose bg folder is bgdir and whose pages are pw x ph: as it is when it is of that
/// size and drawn as it is (bgimage_drawable), else as its source, to be fitted in the background
/// (*fit). Nothing to do when the picture is there already. *placed: a file this call put there,
/// for the caller to take back if what follows fails. 0, or -1 (errno).
#[allow(clippy::too_many_arguments)]
fn place_picture(
    bgdir: &str,
    id: u32,
    src: &str,
    png: &[u8],
    w: i32,
    h: i32,
    pw: i32,
    ph: i32,
    fit: &mut bool,
    placed: &mut String,
    cap: usize,
) -> i32 {
    let mut file = String::new();
    *fit = false;
    placed.clear();
    bgimage_file(bgdir, id, &mut file, 900);
    if exists(&file) {
        return 0;
    }
    let (mut dw, mut dh) = (0, 0);
    let mut why: &'static str = "";
    if !(w == pw && h == ph && bgimage_drawable(png, &mut dw, &mut dh, &mut why) == 0) {
        source_file(bgdir, id, &mut file, 900);
        *fit = true;
        if exists(&file) {
            return 0; // (waiting already)
        }
    }
    if fs::rename(src, &file).is_err() {
        return -1;
    }
    fsync_file(&file); // (KOReader wrote it without)
    sync_dir(bgdir);
    strlcpy_q(placed, &file, cap);
    0
}

// ---- the work ----

/// The state of this start's request and book (the C's file-level statics).
#[derive(Default)]
struct State {
    /// the notebook this start's request (or book) opened (g_last_nb)
    last_nb: FixedStr<STORE_ID_LEN>,
    outcome: AnnotateOutcome,
    /// what this start's book came to (book_start, book_step)
    book: BookState,
    book_added: i32,
    /// the job (J). book_start and book_step take it out of here while they work, and put it back,
    /// so no borrow of the state is held across a call into another module.
    j: Job,
    /// the pictures left to fit after a job (F)
    f: Fits,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

fn fail(out: &mut AnnotateResult, why: AnnotateWhy) {
    out.outcome = AnnotateOutcome::Failed;
    out.why = why;
}

fn request_dir(buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, concat!("%s/", annotate_dir!()), &store_root());
}

/// Empties a request's folder (and removes it): everything in it was one request's. How many files went.
fn empty_folder(dir: &str) -> i32 {
    let Ok(d) = fs::read_dir(dir) else { return 0 };
    let mut n = 0;
    for e in d.flatten() {
        n += fs::remove_file(e.path()).is_ok() as i32;
    }
    let _ = fs::remove_dir(dir);
    n
}

fn empty_requests() -> i32 {
    let mut dir = String::new();
    request_dir(&mut dir, 700);
    empty_folder(&dir)
}

/// Removes a folder a new notebook was being made in (its files, its bg/ and source.ini).
fn remove_work(work: &Path) {
    for pass in 0..2 {
        // (work + "/bg" by its bytes: the C joins them as strings, so an empty work is "/bg")
        let mut dir = work.as_os_str().as_bytes().to_vec();
        if pass == 0 {
            dir.extend_from_slice(b"/bg");
        }
        let dir = Path::new(std::ffi::OsStr::from_bytes(&dir));
        if let Ok(d) = fs::read_dir(dir) {
            for e in d.flatten() {
                let _ = fs::remove_file(e.path());
            }
        }
        let _ = fs::remove_dir(dir);
    }
}

/// What a start cut off while it made a notebook left (notebooks/.annotate-<id>/): never whole,
/// since it becomes the notebook by its last step, a rename.
fn remove_leftovers() {
    let mut dir = String::new();
    csnprintf!(&mut dir, 600, "%s/notebooks", &store_root());
    let Ok(d) = fs::read_dir(&dir) else { return };
    for e in d.flatten() {
        let name = e.file_name();
        if !name.as_bytes().starts_with(WORK_PREFIX.as_bytes()) {
            continue;
        }
        remove_work(&e.path());
        // (a name of ours is ASCII: the id it was made with)
        qlog!(
            "annotate: removed a notebook a cut-off start was making (%s)",
            &*String::from_utf8_lossy(&name.as_bytes()[WORK_PREFIX.len()..])
        );
    }
}

/// Whether notebook id's source.ini says it annotates book: the link, once an older version (which
/// drops notebook.ini's source= line on its next save) has lost it there.
fn map_names(id: &str, book: &str) -> bool {
    let mut dir = String::new();
    let mut m = AnnotateMap::default();
    if store_notebook_dir(id, &mut dir, 600) != 0 || map_load(&dir, &mut m) != 0 {
        return false;
    }
    let yes = m.book.as_bytes() == book.as_bytes();
    annotate_map_free(&mut m);
    yes
}

/// The notebook of book in all (newest first): notebook.ini's source=, else source.ini's book= in a
/// notebook with no source= line. Its index.
fn find_notebook(all: &[NotebookInfo], book: &str) -> Option<usize> {
    for (i, nb) in all.iter().enumerate() {
        if nb.source.as_bytes() == book.as_bytes() {
            return Some(i);
        }
    }
    for (i, nb) in all.iter().enumerate() {
        if nb.source.is_empty() && map_names(nb.id.as_str(), book) {
            return Some(i);
        }
    }
    None
}

/// The id a new page of nb (in folder dir) gets: its next_page_id, stepping over a page file that is
/// there already (one its page list does not name), as store_add_page does. 0 when none is free.
fn free_page_id(dir: &str, nb: &NotebookInfo) -> u32 {
    let mut id = if nb.next_page_id != 0 { nb.next_page_id } else { 1 };
    for _ in 0..64 {
        let mut path = String::new();
        csnprintf!(&mut path, 800, "%s/p%u.qpg", dir, id);
        if !exists(&path) {
            return id;
        }
        id = id.wrapping_add(1);
    }
    0
}

/// The request's page in notebook nb (loaded, its compaction stopped): the page that holds it, or a
/// new one. Picture: the request's file (src), its bytes and size.
fn into_notebook(nb: &mut NotebookInfo, r: &AnnotateRequest, src: &str, png: &[u8], w: i32, h: i32, out: &mut AnnotateResult) {
    let (mut dir, mut bgdir, mut placed) = (String::new(), String::new(), String::new());
    let mut m = AnnotateMap::default();
    if store_notebook_dir(nb.id.as_str(), &mut dir, 600) != 0 || bgimage_dir(nb.id.as_str(), &mut bgdir, 700) != 0 {
        fail(out, AnnotateWhy::Log);
        return;
    }
    if map_load(&dir, &mut m) != 0 {
        qlog!("annotate: %s: its source.ini does not read; left as it is, nothing opened", &nb.id);
        fail(out, AnnotateWhy::Log);
        return;
    }
    if nb.source.is_empty() {
        qlog!("annotate: %s: its link to the book was lost (an older version saved it); linked again", &nb.id);
    }
    nb.source.set_bytes(r.book.as_bytes());
    if m.book.is_empty() {
        m.book.set_bytes(r.book.as_bytes());
    }
    if m.kind.is_empty() {
        m.kind.set_bytes(r.kind.as_bytes());
    }
    let (mut pw, mut ph) = (0, 0);
    store_notebook_page_size(nb, &mut pw, &mut ph);
    let key = bgimage_source_key(png, pw, ph);
    let mut ix = BgIndex::default();
    let mut pic = 0u32;
    let mut pid = annotate_map_find(&m, nb, r.page, r.pages);
    let (mut added, mut fit) = (false, false);
    if pid != 0 {
        // There already: opened as it is. Its picture, should it be missing (its fitting never ran),
        // comes from this one when this one is the same.
        let mut index = 0;
        in_notebook(nb, pid, Some(&mut index));
        let mut file = String::new();
        let mut pg: Option<Box<Page>> = None;
        let mut paper = PaperKind::default();
        if store_read_page(nb, index, &mut pg) == 0
            && pg.as_deref().is_some_and(|p| {
                paper = p.paper;
                paper_is_image(p.paper)
            })
            && mkdir_p(&bgdir) == 0
            && bgindex_load(&bgdir, &mut ix) == 0
        {
            let want = paper_image_id(paper);
            bgimage_file(&bgdir, want, &mut file, 900);
            if !exists(&file)
                && bgindex_find(&ix, key, &mut pic)
                && pic == want
                && place_picture(&bgdir, pic, src, png, w, h, pw, ph, &mut fit, &mut placed, 900) == 0
            {
                qlog!("annotate: %s: page %d of %d is there; its missing picture is made again", &nb.id, r.page, r.pages);
                if fit {
                    source_file(&bgdir, pic, &mut file, 900);
                    bg_queue(BgKind::Template, Some(nb.id.as_str()), Some(&file), store_dpi(), false);
                }
            }
            bgindex_free(&mut ix);
        }
        drop(pg);
        nb.last_page = index;
        if store_write_info_in(&dir, nb) != 0 {
            qlog!("annotate: %s: could not note the page to open", &nb.id);
        }
        qlog!("annotate: %s: page %d of %d is its page %d", &nb.id, r.page, r.pages, index + 1);
        out.outcome = AnnotateOutcome::Opened;
        out.pid = pid;
        annotate_map_free(&mut m);
        return;
    }
    if nb.npages() >= STORE_MAX_PAGES as i32 {
        qlog!("annotate: %s: it has %d pages, the most a notebook has; nothing added", &nb.id, nb.npages());
        fail(out, AnnotateWhy::Log);
        annotate_map_free(&mut m);
        return;
    }
    // A new page, in book order, on a picture of its own (or one of its pages' that is the same).
    if mkdir_p(&bgdir) != 0 || bgindex_load(&bgdir, &mut ix) != 0 {
        qlog!("annotate: %s: its bg folder cannot be written, or bg/index.ini does not read; nothing added", &nb.id);
        fail(out, AnnotateWhy::Storage);
        annotate_map_free(&mut m);
        return;
    }
    let mut rc = bgindex_claim(&mut ix, &bgdir, key, &mut pic, &mut added);
    if rc == 0 && added {
        rc = bgindex_save(&bgdir, &ix);
    }
    bgindex_free(&mut ix);
    if rc == 0 {
        rc = place_picture(&bgdir, pic, src, png, w, h, pw, ph, &mut fit, &mut placed, 900);
    }
    let at = annotate_insert_at(nb, &m, r.page, r.pages);
    let paper = paper_image(PAPER_ROLE_DOCUMENT, pic);
    // The page is listed last. Its id is taken first (notebook.ini's next_page_id, nothing listed
    // yet); then its file on its own paper, its line in bg/pages.ini and its line in source.ini are
    // written; then notebook.ini lists it. A start cut off on the way leaves files that name an id
    // no version gives again (1.0.0-beta takes next_page_id as it is, never stepping over a page
    // file), so they never become a page, nor tell another page's picture or book page.
    let id = if rc == 0 { free_page_id(&dir, nb) } else { 0 };
    let mut pagefile = String::new();
    let (mut taken, mut written) = (false, false);
    if id != 0 {
        nb.next_page_id = id + 1;
        taken = store_write_info_in(&dir, nb) == 0; // (nothing else changes, its modified time neither)
    }
    let p = if taken { Some(page_new(pw, ph, paper)) } else { None };
    if let Some(mut p) = p {
        p.nb.set_bytes(nb.id.as_bytes());
        nb.page_ids.insert(at as usize, id);
        // (in memory, until notebook.ini lists it below)
        csnprintf!(&mut pagefile, 800, "%s/p%u.qpg", &dir, id);
        written = store_save_page(nb, at, &p) == 0; // its file, and its line in bg/pages.ini
    }
    nb.last_page = at;
    let listed = written && annotate_map_add(&mut m, r.page, r.pages, id) == 0 && map_save(&dir, &m, nb) == 0 && store_save_info(nb) == 0;
    if !listed {
        let e = errno();
        if written {
            let _ = fs::remove_file(&pagefile); // (never listed, and its id is not given again)
        }
        if !placed.is_empty() {
            let _ = fs::remove_file(&placed);
        }
        qlog!("annotate: %s: page %d of %d could not be added (%s); the notebook stays as it was", &nb.id, r.page, r.pages, &strerror(e));
        fail(out, AnnotateWhy::Storage);
        annotate_map_free(&mut m);
        return;
    }
    pid = id;
    // A new first page: the Library's card shows it once the editor has drawn it (it makes the
    // notebook's thumbnail as it leaves the page); the thumbnail of the page that was first goes.
    if at == 0 {
        let mut thumb = String::new();
        csnprintf!(&mut thumb, 700, "%s/thumb.pgm", &dir);
        let _ = fs::remove_file(&thumb);
    }
    if fit {
        let mut file = String::new();
        source_file(&bgdir, pic, &mut file, 900);
        bg_queue(BgKind::Template, Some(nb.id.as_str()), Some(&file), store_dpi(), false);
    }
    qlog!(
        "annotate: %s: page %d of %d added as its page %d of %d, picture bg/%06x.png%s",
        &nb.id,
        r.page,
        r.pages,
        at + 1,
        nb.npages(),
        pic,
        if fit { " (fitted to its pages in the background)" } else { "" }
    );
    out.outcome = AnnotateOutcome::Made;
    out.pid = pid;
    annotate_map_free(&mut m);
}

/// A new notebook for the book, its one page the request's: made in notebooks/.annotate-<id>/ (a
/// dot-folder the Library and every older version skip) and renamed into place once whole, so a
/// failure or a start cut off leaves no notebook half made.
fn new_notebook(r: &AnnotateRequest, all: &[NotebookInfo], src: &str, png: &[u8], w: i32, h: i32, out: &mut AnnotateResult) {
    let mut nb = Box::new(NotebookInfo::default());
    let (mut work, mut final_, mut bgdir, mut path, mut placed) =
        (String::new(), String::new(), String::new(), String::new(), String::new());
    let mut tries = 0;
    loop {
        // (a notebook made the same second with the same random part)
        let mut id = String::new();
        store_make_id(&mut id);
        nb.id.set(&id);
        if store_notebook_dir(nb.id.as_str(), &mut final_, 600) != 0 || tries == 8 {
            fail(out, AnnotateWhy::Log);
            return;
        }
        if !exists(&final_) {
            break;
        }
        tries += 1;
    }
    csnprintf!(&mut work, 700, concat!("%s/notebooks/", work_prefix!(), "%s"), &store_root(), &nb.id);
    csnprintf!(&mut bgdir, 800, "%s/bg", &work);
    let (mut pw, mut ph) = (0, 0);
    store_page_size(&mut pw, &mut ph);
    let key = bgimage_source_key(png, pw, ph);
    let mut ix = BgIndex::default();
    let mut pic = 0u32;
    let (mut added, mut fit) = (false, false);
    remove_work(Path::new(&work));
    let mut rc = if mkdir_p(&bgdir) == 0 && bgindex_claim(&mut ix, &bgdir, key, &mut pic, &mut added) == 0 && bgindex_save(&bgdir, &ix) == 0
    {
        0
    } else {
        -1
    };
    bgindex_free(&mut ix);
    if rc == 0 {
        rc = place_picture(&bgdir, pic, src, png, w, h, pw, ph, &mut fit, &mut placed, 900);
    }
    let paper = paper_image(PAPER_ROLE_DOCUMENT, pic);
    if rc == 0 {
        let p = page_new(pw, ph, paper);
        let one = 1u32;
        rc = if store_write_page_in(&work, 1, &p) == 0 && store_write_pages_list_in(&bgdir, &[one], &[paper]) == 0 { 0 } else { -1 };
    }
    if rc == 0 {
        let mut m = AnnotateMap::default();
        m.book.set_bytes(r.book.as_bytes());
        m.kind.set_bytes(r.kind.as_bytes());
        let mut t: Option<Vec<u8>> = None;
        nb.page_ids = vec![1];
        if annotate_map_add(&mut m, r.page, r.pages, 1) == 0 {
            t = annotate_map_text(&m, &nb);
        }
        csnprintf!(&mut path, 900, "%s/source.ini", &work);
        rc = if t.as_ref().is_some_and(|t| write_file_atomic(&path, t) == 0) { 0 } else { -1 };
        annotate_map_free(&mut m);
    }
    if rc == 0 {
        // Named after the book, its title whole (with no title: Untitled, numbered as any name;
        // the screens leave out what the fonts cannot draw). Its own paper is Blank, as an imported
        // notebook's: what a page added to it gets, and what a page that does not read shows.
        let mut name = String::new();
        annotate_name(r.title.as_bytes(), &mut name, STORE_NAME_LEN);
        let mut unique = String::new();
        // TRANSLATORS: the name of a notebook that has none (it is saved with it, as a name given).
        let base = if name.is_empty() { tr("Untitled") } else { name.as_str() };
        unique_name(base, all, &mut unique, STORE_NAME_LEN);
        nb.name.set_bytes(unique.as_bytes());
        nb.paper = PAPER_BLANK;
        nb.created = wall_time();
        nb.modified = nb.created;
        nb.next_page_id = 2;
        nb.page_w = pw;
        nb.page_h = ph;
        nb.source.set_bytes(r.book.as_bytes());
        // notebook.ini last: then the folder becomes the notebook.
        rc = if store_write_info_in(&work, &nb) == 0 { 0 } else { -1 };
    }
    if rc == 0 {
        sync_dir(&bgdir);
        sync_dir(&work);
        rc = if fs::rename(&work, &final_).is_ok() { 0 } else { -1 };
    }
    if rc != 0 {
        qlog!("annotate: page %d of %d: a notebook for it could not be made (%s); nothing made", r.page, r.pages, &strerror(errno()));
        remove_work(Path::new(&work));
        fail(out, AnnotateWhy::Storage);
        return;
    }
    let mut notebooks = String::new();
    csnprintf!(&mut notebooks, 600, "%s/notebooks", &store_root());
    sync_dir(&notebooks);
    if fit {
        let (mut file, mut fbg) = (String::new(), String::new());
        bgimage_dir(nb.id.as_str(), &mut fbg, 700);
        source_file(&fbg, pic, &mut file, 900);
        bg_queue(BgKind::Template, Some(nb.id.as_str()), Some(&file), store_dpi(), false);
    }
    qlog!(
        "annotate: notebook %s made for a %s book, page %d of %d, picture bg/%06x.png%s",
        &nb.id,
        if !r.kind.is_empty() { r.kind.as_str() } else { "(kind not given)" },
        r.page,
        r.pages,
        pic,
        if fit { " (fitted to its pages in the background)" } else { "" }
    );
    out.nb.set_bytes(nb.id.as_bytes());
    out.outcome = AnnotateOutcome::Made;
    out.pid = 1;
}

/// The request read and fresh: its picture checked, then its page found or made.
fn handle(r: &AnnotateRequest, out: &mut AnnotateResult) {
    let (mut dir, mut src) = (String::new(), String::new());
    request_dir(&mut dir, 700);
    csnprintf!(&mut src, 800, "%s/%s", &dir, &r.picture);
    set_errno(0);
    // (A file of its own, as the plugin writes it: never a link to one elsewhere, moved into bg/.)
    let png = if is_own_file(&src) { bgimage_read(&src, PICTURE_MAX) } else { None };
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = if errno() == libc::EFBIG { "bigger than a page picture can be" } else { "missing or unreadable" };
    let png = match png {
        Some(png) if annotate_png_check(&png, &mut w, &mut h, &mut why) == 0 => png,
        _ => {
            qlog!("annotate: page %d of %d: its picture is %s; nothing opened", r.page, r.pages, why);
            fail(out, AnnotateWhy::Picture);
            return;
        }
    };
    let mut all: Vec<NotebookInfo> = Vec::new();
    if store_list(&mut all) != 0 {
        qlog!("annotate: the notebooks do not list; nothing opened");
        fail(out, AnnotateWhy::Log);
        return;
    }
    if let Some(at) = find_notebook(&all, r.book.as_str()) {
        // Its compaction (queued at this start for what an earlier one left) stops before its page
        // list changes, as when a notebook opens.
        bg_cancel_notebook(all[at].id.as_str());
        let mut nb = Box::new(NotebookInfo::default());
        if store_load_info(all[at].id.as_str(), &mut nb) == 0 {
            out.nb.set_bytes(nb.id.as_bytes());
            into_notebook(&mut nb, r, &src, &png, w, h, out);
        } else {
            fail(out, AnnotateWhy::Log);
        }
    } else {
        new_notebook(r, &all, &src, &png, w, h, out);
    }
}

/// The request in grimoire/import/.annotate/, if any, at startup, before anything opens a notebook.
/// asked: KOReader's plugin started Grimoire to open it (GRIMOIRE_ANNOTATE=1, hosted). A request
/// this start was not asked for (Grimoire started from the Library, or by Open Grimoire, after a
/// hand-over that never reached it), or one too old, is deleted unopened, with one log line. Either
/// way the folder is emptied after. The book's notebook has its compaction stopped first, as when a
/// notebook opens. A page picture of the size of its notebook's pages (KOReader draws them at the
/// Scribe's) becomes the page's picture as it is; one of another size (a notebook from a Scribe of the
/// other size, a screenshot taken sideways) is fitted to the page in the background (BG_TEMPLATE),
/// and the page draws white until then. Opens nothing itself: the caller opens the page given.
pub fn annotate_take(asked: bool, out: &mut AnnotateResult) {
    *out = AnnotateResult::default();
    remove_leftovers();
    let (mut dir, mut path) = (String::new(), String::new());
    request_dir(&mut dir, 700);
    csnprintf!(&mut path, 800, concat!("%s/", annotate_request!()), &dir);
    let st = fs::metadata(&path).ok().filter(|m| m.is_file());
    let have = st.is_some();
    let mut r = AnnotateRequest::default();
    let mut why: &'static str = "it does not read";
    let size = st.as_ref().map_or(0, |m| m.len());
    let text = if have && size <= REQUEST_MAX as u64 { read_file(&path) } else { None };
    let parsed = text.as_deref().is_some_and(|t| annotate_parse(t, &mut r, &mut why) == 0);
    if !have {
        if asked {
            qlog!(concat!("annotate: KOReader asked for a page, but there is no request in ", annotate_dir!()));
            fail(out, AnnotateWhy::Log);
        }
    } else if !asked {
        qlog!(concat!(
            "annotate: a request from KOReader that this start was not asked to open (Grimoire started from the ",
            "Library, or by Open Grimoire); deleted unopened"
        ));
        out.outcome = AnnotateOutcome::Stale;
    } else if !parsed {
        qlog!("annotate: the request does not read (%s); nothing opened", if size > REQUEST_MAX as u64 { "too big" } else { why });
        fail(out, AnnotateWhy::Log);
    } else if annotate_too_old(&r, wall_time(), st.as_ref().map_or(0, |m| m.mtime())) {
        let age = wall_time() - (if r.time > 0 { r.time } else { st.as_ref().map_or(0, |m| m.mtime()) });
        qlog!(
            "annotate: a request from %lld s %s; deleted unopened",
            if age < 0 { -age } else { age },
            if age >= 0 { "ago" } else { "ahead (the clock was set back?)" }
        );
        out.outcome = AnnotateOutcome::Stale;
    } else {
        out.page = r.page;
        out.pages = r.pages;
        handle(&r, out);
    }
    let gone = empty_requests();
    if gone > 2 {
        qlog!(concat!("annotate: %d files in ", annotate_dir!(), " deleted"), gone);
    }
    with(|s| {
        s.outcome = out.outcome;
        if out.outcome == AnnotateOutcome::Opened || out.outcome == AnnotateOutcome::Made {
            s.last_nb = out.nb;
        } else {
            s.last_nb.clear();
        }
    });
}

// ---- a whole book: "Open this book in Grimoire" ----
//
// KOReader's plugin draws every page of a paged document (PDF, DjVu, comics) that the book's
// notebook does not hold yet into grimoire/import/.book/page-<n>.png, writes a request naming them
// (book.ini rules as request.ini's, docs/FORMAT.md, Whole books), and hands over with --book
// (GRIMOIRE_BOOK=1). Grimoire then adds those pages to the book's notebook (one made for the book
// when there is none), each in book order among the pages there (notes stay after their page), all
// listed at once at the end: a failure on the way takes back every file it wrote. The work runs in
// slices on the UI thread (book_step), each page a picture moved into place and a small page file,
// so the launcher's heartbeat keeps going for a book of any size; the editor then opens on the page
// the reader was on.

/// the request's folder, in the data root
pub const BOOK_DIR: &str = book_dir_name!();
/// the request, in it
pub const BOOK_REQUEST: &str = book_request!();

#[derive(Clone, Debug, Default)]
pub struct BookRequest {
    /// 1
    pub version: i32,
    /// as AnnotateRequest's
    pub book: FixedStr<STORE_SOURCE_LEN>,
    pub title: FixedStr<ANNOTATE_TITLE_LEN>,
    /// the page to open (the reader's), and how many the book has
    pub page: i32,
    pub pages: i32,
    pub kind: FixedStr<16>,
    pub time: i64,
    /// pictures sent: page-<n>.png for each of these book pages, ascending, each once
    /// (picture.<n>=page-<n>.png lines): the C's n and sent
    pub sent: Vec<i32>,
}

/// (the lines of up to 2000 pictures)
const BOOK_REQUEST_MAX: usize = 128 << 10;

fn book_dir(buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, concat!("%s/", book_dir_name!()), &store_root());
}

pub fn book_request_free(r: &mut BookRequest) {
    r.sent = Vec::new();
}

/// Reads a request's text. 0, or -1 and why (never the title). book_request_free frees it either way.
pub fn book_parse(text: &[u8], r: &mut BookRequest, why: &mut &'static str) -> i32 {
    *r = BookRequest::default();
    r.version = 1;
    *why = "no request";
    let text = cstr(text);
    let (mut page, mut pages): (i64, i64) = (-1, -1);
    let mut p = 0;
    while p < text.len() {
        let (n, step) = next_line(&text[p..]);
        let k = n.min(LINE_MAX - 1);
        let mut line = &text[p..p + k];
        if let Some((&b'\r', head)) = line.split_last() {
            line = head;
        }
        p += step;
        let Some(eq) = line.iter().position(|&c| c == b'=') else { continue };
        if line[0] == b'#' {
            continue;
        }
        let (key, v) = (&line[..eq], &line[eq + 1..]);
        if key == b"version" {
            r.version = number(v, 0, 1000000) as i32;
        } else if key == b"book" {
            if !book_key_ok(v) {
                *why = "its book key is not one";
                return -1;
            }
            r.book.set_bytes(v);
        } else if key == b"title" {
            r.title.set_bytes(v);
        } else if key == b"page" {
            page = number(v, 1, 10000000);
        } else if key == b"pages" {
            pages = number(v, 1, 10000000);
        } else if key == b"kind" {
            if word_ok(v, FixedStr::<16>::CAP) {
                r.kind.set_bytes(v);
            }
        } else if key == b"time" {
            let t = number(v, 0, 1i64 << 40);
            r.time = if t > 0 { t } else { 0 };
        } else if key.starts_with(b"picture.") {
            // picture.<n>=page-<n>.png: that name and no other, so it names a file in the folder.
            let b = number(&key[8..], 1, STORE_MAX_PAGES as i64);
            let mut want = String::new();
            csnprintf!(&mut want, 32, "page-%lld.png", b);
            if b < 1 || v != want.as_bytes() {
                *why = "a picture line is not picture.<n>=page-<n>.png for a page of a notebook";
                return -1;
            }
            if r.sent.try_reserve(1).is_err() {
                *why = "out of memory";
                return -1;
            }
            r.sent.push(b as i32);
        } // (a key this version does not know: a newer plugin's, left to it)
    }
    if r.version != 1 {
        *why = "a version of request this Grimoire does not know (from a newer plugin)";
        return -1;
    }
    if r.book.is_empty() {
        *why = "no book key";
        return -1;
    }
    if pages < 1 || page < 1 || page > pages {
        *why = if pages < 1 { "no page count" } else { "no page to open, or one past the last page" };
        return -1;
    }
    r.page = page as i32;
    r.pages = pages as i32;
    r.sent.sort();
    for i in 0..r.sent.len() {
        if r.sent[i] > r.pages || (i > 0 && r.sent[i] == r.sent[i - 1]) {
            *why = if r.sent[i] > r.pages { "a picture of a page past the last page" } else { "a page's picture named twice" };
            return -1;
        }
    }
    *why = "";
    0
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BookState {
    /// no request
    #[default]
    None = 0, // BOOK_NONE
    /// not asked for, or too old: deleted unopened
    Stale = 1, // BOOK_STALE
    /// pages are being added: call book_step until it is DONE or FAILED
    Running = 2, // BOOK_RUNNING
    /// the notebook holds the book's pages: open nb at pid
    Done = 3, // BOOK_DONE
    /// nothing changed (the notebook stays as it was); why says why
    Failed = 4, // BOOK_FAILED
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BookStatus {
    pub state: BookState,
    /// DONE: the notebook, and the page to open
    pub nb: FixedStr<STORE_ID_LEN>,
    pub pid: u32,
    /// DONE: the notebook was made for the book
    pub made: bool,
    /// pages added so far, of how many to add
    pub added: i32,
    pub total: i32,
    /// FAILED
    pub why: AnnotateWhy,
    /// FAILED: the book has more pages than a notebook holds
    pub too_many: bool,
}

/// The job: one at a time, from book_start to DONE or FAILED.
#[derive(Default)]
struct Job {
    on: bool,
    r: BookRequest,
    /// the book's notebook (a new one's info, until it is in place)
    nb: NotebookInfo,
    /// a new notebook, being built in work
    made: bool,
    /// its folder (a new one's work folder until the end), and its bg/
    dir: String,
    /// a new one's folder to be
    final_: String,
    bgdir: String,
    pw: i32,
    ph: i32,
    m: AnnotateMap,
    ix: BgIndex,
    /// the next page id to give (reserved in notebook.ini at the start)
    next_id: u32,
    /// index into r.sent of the next page to add
    next: usize,
    /// pages added so far: ids, papers (the C's nids is their length)
    ids: Vec<u32>,
    papers: Vec<PaperKind>,
    /// files put into bg/ so far, to take back on a failure (an existing notebook)
    placed: Vec<String>,
    /// pictures to fit in the background, once the notebook is done
    fits: Vec<u32>,
}

/// Pictures left to fit after a job (book_fits_tick), in their notebook.
#[derive(Default)]
struct Fits {
    nb: FixedStr<STORE_ID_LEN>,
    ids: Vec<u32>,
    next: usize,
}

fn job_free(j: &mut Job) {
    *j = Job::default();
}

fn job_end(j: &mut Job, st: &mut BookStatus) {
    with(|s| s.book = st.state);
    let mut dir = String::new();
    book_dir(&mut dir, 700);
    let gone = empty_folder(&dir);
    if gone > 2 && st.state != BookState::Done {
        qlog!(concat!("book: %d files in ", book_dir_name!(), " deleted"), gone);
    }
    job_free(j);
}

fn job_fail(j: &mut Job, st: &mut BookStatus, why: AnnotateWhy) {
    if j.made {
        remove_work(Path::new(&j.dir));
    } else {
        // The page files it wrote (never listed; their ids are not given again) and the pictures it
        // put into bg/ go; notebook.ini, bg/pages.ini and source.ini were not written yet.
        for id in &j.ids {
            let mut path = String::new();
            csnprintf!(&mut path, 800, "%s/p%u.qpg", &j.dir, *id);
            let _ = fs::remove_file(&path);
        }
        for placed in &j.placed {
            let _ = fs::remove_file(placed);
        }
    }
    st.state = BookState::Failed;
    st.why = why;
    job_end(j, st);
}

fn note_placed(j: &mut Job, file: &str) -> bool {
    if j.placed.try_reserve(1).is_err() {
        return false;
    }
    j.placed.push(file.to_string());
    true
}

/// The request in grimoire/import/.book/, if any, at startup (after annotate_take). asked: KOReader's
/// plugin started Grimoire to open it (GRIMOIRE_BOOK=1, hosted); else, or too old, it is deleted
/// unopened. RUNNING: the book's notebook found (its compaction stopped) or a new one begun.
pub fn book_start(asked: bool, st: &mut BookStatus) {
    *st = BookStatus::default();
    // (job_free: whatever an earlier job held goes)
    let mut j = Job::default();
    start(&mut j, asked, st);
    with(|s| s.j = j);
}

fn start(j: &mut Job, asked: bool, st: &mut BookStatus) {
    let (mut dir, mut path) = (String::new(), String::new());
    book_dir(&mut dir, 700);
    csnprintf!(&mut path, 800, concat!("%s/", book_request!()), &dir);
    let Some(rs) = fs::metadata(&path).ok().filter(|m| m.is_file()) else {
        if asked {
            qlog!(concat!("book: KOReader asked to open a book, but there is no request in ", book_dir_name!()));
            st.state = BookState::Failed;
            st.why = AnnotateWhy::Log;
        }
        empty_folder(&dir); // (pictures of a request that never came whole)
        return;
    };
    let mut why: &'static str = "it does not read";
    let size = rs.len();
    let text = if size <= BOOK_REQUEST_MAX as u64 { read_file(&path) } else { None };
    let parsed = text.as_deref().is_some_and(|t| book_parse(t, &mut j.r, &mut why) == 0);
    let age = AnnotateRequest { time: j.r.time, ..AnnotateRequest::default() };
    if !asked {
        qlog!("book: a request from KOReader that this start was not asked to open; deleted unopened");
        st.state = BookState::Stale;
    } else if !parsed {
        qlog!("book: the request does not read (%s); nothing opened", if size > BOOK_REQUEST_MAX as u64 { "too big" } else { why });
        st.state = BookState::Failed;
        st.why = AnnotateWhy::Log;
    } else if annotate_too_old(&age, wall_time(), rs.mtime()) {
        qlog!("book: a request too old (or from the future); deleted unopened");
        st.state = BookState::Stale;
    }
    if st.state != BookState::None {
        job_end(j, st);
        return;
    }
    if j.r.pages > STORE_MAX_PAGES as i32 {
        qlog!("book: a book of %d pages, more than a notebook holds (%d); nothing opened", j.r.pages, STORE_MAX_PAGES as i32);
        st.too_many = true;
        job_fail(j, st, AnnotateWhy::Log);
        return;
    }
    let mut all: Vec<NotebookInfo> = Vec::new();
    if store_list(&mut all) != 0 {
        qlog!("book: the notebooks do not list; nothing opened");
        job_fail(j, st, AnnotateWhy::Log);
        return;
    }
    let room = j.r.sent.len() + 1;
    if j.ids.try_reserve_exact(room).is_err() || j.papers.try_reserve_exact(room).is_err() || j.fits.try_reserve_exact(room).is_err() {
        job_fail(j, st, AnnotateWhy::Log);
        return;
    }
    if let Some(at) = find_notebook(&all, j.r.book.as_str()) {
        bg_cancel_notebook(all[at].id.as_str()); // (its page list is about to change, as when it opens)
        if store_load_info(all[at].id.as_str(), &mut j.nb) != 0
            || store_notebook_dir(j.nb.id.as_str(), &mut j.dir, 600) != 0
            || bgimage_dir(j.nb.id.as_str(), &mut j.bgdir, 700) != 0
        {
            job_fail(j, st, AnnotateWhy::Log);
            return;
        }
        if map_load(&j.dir, &mut j.m) != 0 {
            qlog!("book: %s: its source.ini does not read; left as it is, nothing opened", &j.nb.id);
            job_fail(j, st, AnnotateWhy::Log);
            return;
        }
        store_notebook_page_size(&j.nb, &mut j.pw, &mut j.ph);
        // Ids for every page it may add, taken now (notebook.ini, nothing listed yet), so a start cut
        // off on the way leaves page files no version gives an id to again.
        let first = free_page_id(&j.dir, &j.nb);
        j.next_id = first;
        j.nb.next_page_id = first.wrapping_add(j.r.sent.len() as u32);
        let mut missing = 0;
        for i in 0..j.r.sent.len() {
            missing += (annotate_map_find(&j.m, &j.nb, j.r.sent[i], j.r.pages) == 0) as i32;
        }
        if j.nb.npages() + missing > STORE_MAX_PAGES as i32 {
            qlog!("book: %s: %d pages to add to its %d, more than a notebook holds; nothing added", &j.nb.id, missing, j.nb.npages());
            st.too_many = true;
            job_fail(j, st, AnnotateWhy::Log);
            return;
        }
        if first == 0
            || (!j.r.sent.is_empty() && store_write_info_in(&j.dir, &j.nb) != 0)
            || mkdir_p(&j.bgdir) != 0
            || bgindex_load(&j.bgdir, &mut j.ix) != 0
        {
            qlog!("book: %s: its files cannot be written; nothing added", &j.nb.id);
            job_fail(j, st, AnnotateWhy::Storage);
            return;
        }
        if j.nb.source.is_empty() {
            qlog!("book: %s: its link to the book was lost (an older version saved it); linked again", &j.nb.id);
        }
    } else {
        // A new notebook, built in notebooks/.annotate-<id>/ and renamed into place once whole.
        j.made = true;
        let mut tries = 0;
        loop {
            let mut id = String::new();
            store_make_id(&mut id);
            j.nb.id.set(&id);
            if store_notebook_dir(j.nb.id.as_str(), &mut j.final_, 600) != 0 || tries == 8 {
                job_fail(j, st, AnnotateWhy::Log);
                return;
            }
            if !exists(&j.final_) {
                break;
            }
            tries += 1;
        }
        csnprintf!(&mut j.dir, 600, concat!("%s/notebooks/", work_prefix!(), "%s"), &store_root(), &j.nb.id);
        csnprintf!(&mut j.bgdir, 700, "%s/bg", &j.dir);
        remove_work(Path::new(&j.dir));
        store_page_size(&mut j.pw, &mut j.ph);
        j.next_id = 1;
        if j.r.sent.is_empty() || mkdir_p(&j.bgdir) != 0 {
            qlog!(
                "book: %s; nothing made",
                if !j.r.sent.is_empty() { "notebooks/ cannot be written" } else { "no pages came for a new notebook" }
            );
            job_fail(j, st, if !j.r.sent.is_empty() { AnnotateWhy::Storage } else { AnnotateWhy::Log });
            return;
        }
        let mut name = String::new();
        annotate_name(j.r.title.as_bytes(), &mut name, STORE_NAME_LEN);
        let mut unique = String::new();
        // TRANSLATORS: the name of a notebook that has none (it is saved with it, as a name given).
        let base = if name.is_empty() { tr("Untitled") } else { name.as_str() };
        unique_name(base, &all, &mut unique, STORE_NAME_LEN);
        j.nb.name.set_bytes(unique.as_bytes());
    }
    j.nb.source.set_bytes(j.r.book.as_bytes());
    if j.m.book.is_empty() {
        j.m.book.set_bytes(j.r.book.as_bytes());
    }
    if j.m.kind.is_empty() {
        j.m.kind.set_bytes(if !j.r.kind.is_empty() { j.r.kind.as_bytes() } else { b"paged" });
    }
    j.on = true;
    st.nb.set_bytes(j.nb.id.as_bytes());
    st.made = j.made;
    st.total = j.r.sent.len() as i32;
    st.state = BookState::Running;
    qlog!(
        "book: %s: %d page(s) of %d to add, in a %s notebook",
        &j.nb.id,
        j.r.sent.len() as i32,
        j.r.pages,
        if j.made { "new" } else { "existing" }
    );
}

/// One page: its picture into bg/, its page file, its place in the page list and source.ini (in
/// memory until the end). 0, or the AnnotateWhy to fail with.
// (The C returns ANNOTATE_WHY_LOG, which is 0, on three of its failures below: the caller takes 0 for
// success. They are kept as they are: see the report.)
fn add_one(j: &mut Job, bpage: i32, failed_picture: &mut bool) -> i32 {
    *failed_picture = false;
    let (mut dir, mut src) = (String::new(), String::new());
    book_dir(&mut dir, 700);
    csnprintf!(&mut src, 800, "%s/page-%d.png", &dir, bpage);
    if !j.made && annotate_map_find(&j.m, &j.nb, bpage, j.r.pages) != 0 {
        let _ = fs::remove_file(&src); // there already (a newer page there than the plugin knew of): kept as it is
        return 0;
    }
    set_errno(0);
    let png = if is_own_file(&src) { bgimage_read(&src, PICTURE_MAX) } else { None };
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = if errno() == libc::EFBIG { "bigger than a page picture can be" } else { "missing or unreadable" };
    let png = match png {
        Some(png) if annotate_png_check(&png, &mut w, &mut h, &mut why) == 0 => png,
        _ => {
            qlog!("book: page %d of %d: its picture is %s; nothing added", bpage, j.r.pages, why);
            *failed_picture = true;
            return AnnotateWhy::Picture as i32;
        }
    };
    let key = bgimage_source_key(&png, j.pw, j.ph);
    let mut pic = 0u32;
    let (mut added, mut fit) = (false, false);
    let mut placed = String::new();
    let mut rc = bgindex_claim(&mut j.ix, &j.bgdir, key, &mut pic, &mut added);
    if rc == 0 {
        rc = place_picture(&j.bgdir, pic, &src, &png, w, h, j.pw, j.ph, &mut fit, &mut placed, 900);
    }
    drop(png);
    if rc != 0 {
        return AnnotateWhy::Storage as i32;
    }
    if !placed.is_empty() && !j.made && !note_placed(j, &placed) {
        let _ = fs::remove_file(&placed);
        return AnnotateWhy::Log as i32;
    }
    if fit {
        let mut queued = false;
        for f in &j.fits {
            queued |= *f == pic;
        }
        if !queued {
            j.fits.push(pic);
        }
    }
    let mut id = j.next_id;
    if !j.made {
        // stepping over a page file that is there already, as free_page_id does
        while id < j.nb.next_page_id {
            let mut path = String::new();
            csnprintf!(&mut path, 800, "%s/p%u.qpg", &j.dir, id);
            if !exists(&path) {
                break;
            }
            id += 1;
        }
        if id >= j.nb.next_page_id {
            return AnnotateWhy::Log as i32; // (none left of those it took)
        }
    }
    j.next_id = id + 1;
    let paper = paper_image(PAPER_ROLE_DOCUMENT, pic);
    let p = page_new(j.pw, j.ph, paper);
    if store_write_page_in(&j.dir, id, &p) != 0 {
        return AnnotateWhy::Storage as i32;
    }
    drop(p);
    j.ids.push(id);
    j.papers.push(paper);
    let at = if j.made { j.nb.npages() } else { annotate_insert_at(&j.nb, &j.m, bpage, j.r.pages) };
    j.nb.page_ids.insert(at as usize, id);
    if annotate_map_add(&mut j.m, bpage, j.r.pages, id) == 0 {
        0
    } else {
        AnnotateWhy::Log as i32
    }
}

/// Every page added: the lists written, notebook.ini last (then the pages are the notebook's).
fn commit(j: &mut Job, st: &mut BookStatus) -> i32 {
    if !j.ids.is_empty() && bgindex_save(&j.bgdir, &j.ix) != 0 {
        return -1;
    }
    let mut open = annotate_map_find(&j.m, &j.nb, j.r.page, j.r.pages);
    let mut open_at = 0;
    if open == 0 && !j.ids.is_empty() {
        open = j.ids[0]; // (the reader's page is not in the book's notebook: its first new one)
    }
    if open == 0 && j.nb.npages() != 0 {
        open = j.nb.page_ids[(if j.nb.last_page < j.nb.npages() { j.nb.last_page } else { 0 }) as usize];
    }
    in_notebook(&j.nb, open, Some(&mut open_at));
    j.nb.last_page = open_at;
    if j.made {
        let mut path = String::new();
        let t = annotate_map_text(&j.m, &j.nb);
        csnprintf!(&mut path, 800, "%s/source.ini", &j.dir);
        let rc = if t.as_ref().is_some_and(|t| write_file_atomic(&path, t) == 0)
            && store_write_pages_list_in(&j.bgdir, &j.ids, &j.papers) == 0
        {
            0
        } else {
            -1
        };
        if rc != 0 {
            return -1;
        }
        j.nb.paper = PAPER_BLANK; // (what a page added to it gets, as an imported notebook's)
        j.nb.created = wall_time();
        j.nb.modified = j.nb.created;
        j.nb.next_page_id = j.next_id;
        j.nb.page_w = j.pw;
        j.nb.page_h = j.ph;
        if store_write_info_in(&j.dir, &j.nb) != 0 {
            return -1;
        }
        sync_dir(&j.bgdir);
        sync_dir(&j.dir);
        if fs::rename(&j.dir, &j.final_).is_err() {
            return -1;
        }
        let mut notebooks = String::new();
        csnprintf!(&mut notebooks, 600, "%s/notebooks", &store_root());
        sync_dir(&notebooks);
    } else if !j.ids.is_empty() {
        let mut first_new = false;
        for id in &j.ids {
            first_new |= j.nb.page_ids[0] == *id;
        }
        if store_note_papers_in(&j.bgdir, &j.ids, &j.papers) != 0 || map_save(&j.dir, &j.m, &j.nb) != 0 || store_save_info(&mut j.nb) != 0 {
            return -1;
        }
        if first_new {
            // a new first page: the Library's card shows it once the editor drew it
            let mut thumb = String::new();
            csnprintf!(&mut thumb, 700, "%s/thumb.pgm", &j.dir);
            let _ = fs::remove_file(&thumb);
        }
    } else if store_write_info_in(&j.dir, &j.nb) != 0 {
        return -1; // (nothing added: only the page to open)
    }
    if !j.fits.is_empty() {
        // fitted in the background, one at a time (book_fits_tick)
        with(|s| s.f = Fits { nb: j.nb.id, ids: j.fits.clone(), next: 0 });
    }
    st.nb.set_bytes(j.nb.id.as_bytes());
    st.pid = open;
    st.made = j.made;
    st.added = j.ids.len() as i32;
    st.state = BookState::Done;
    with(|s| {
        s.last_nb = j.nb.id;
        s.book_added = j.ids.len() as i32;
    });
    qlog!(
        "book: %s: %d page(s) added (%d now), opens at its page %d%s",
        &j.nb.id,
        j.ids.len() as i32,
        j.nb.npages(),
        open_at + 1,
        if !j.fits.is_empty() { " (pictures fitted to its pages in the background)" } else { "" }
    );
    0
}

/// Adds pages for about budget_ms; then st says how far it got. At DONE or FAILED the request's
/// folder is emptied.
pub fn book_step(budget_ms: u32, st: &mut BookStatus) {
    if st.state != BookState::Running {
        return; // (what step says first, without taking the job out for nothing)
    }
    let mut j = with(|s| std::mem::take(&mut s.j));
    step(&mut j, budget_ms, st);
    with(|s| s.j = j);
}

fn step(j: &mut Job, budget_ms: u32, st: &mut BookStatus) {
    if !j.on || st.state != BookState::Running {
        return;
    }
    let t0 = now_ms();
    while j.next < j.r.sent.len() && now_ms().wrapping_sub(t0) < budget_ms {
        let mut bad_picture = false;
        let bpage = j.r.sent[j.next];
        let why = add_one(j, bpage, &mut bad_picture);
        if why != 0 {
            if !bad_picture {
                qlog!("book: page %d of %d could not be added (%s); nothing added", bpage, j.r.pages, &strerror(errno()));
            }
            // (add_one answers 0 for no failure, so only 1 and 2 get here)
            job_fail(j, st, if why == AnnotateWhy::Picture as i32 { AnnotateWhy::Picture } else { AnnotateWhy::Storage });
            return;
        }
        j.next += 1;
        st.added = j.ids.len() as i32;
    }
    if j.next < j.r.sent.len() {
        return;
    }
    if commit(j, st) != 0 {
        qlog!("book: %s: its page list could not be written (%s); nothing added", &j.nb.id, &strerror(errno()));
        job_fail(j, st, AnnotateWhy::Storage);
        return;
    }
    job_end(j, st);
}

/// Pictures of another size than their notebook's pages (a notebook from the other Scribe size) are
/// fitted in the background, one queued at a time while the job queue is idle: call it often.
pub fn book_fits_tick() {
    let Some((nb, id)) = with(|s| if s.f.next >= s.f.ids.len() { None } else { Some((s.f.nb, s.f.ids[s.f.next])) }) else { return };
    if bg_busy() {
        return;
    }
    let (mut bgdir, mut file) = (String::new(), String::new());
    if bgimage_dir(nb.as_str(), &mut bgdir, 700) == 0 {
        source_file(&bgdir, id, &mut file, 900);
        bg_queue(BgKind::Template, Some(nb.as_str()), Some(&file), store_dpi(), false);
    }
    with(|s| {
        s.f.next += 1;
        if s.f.next >= s.f.ids.len() {
            s.f = Fits::default();
        }
    });
}

// ---- probes ----

/// How many files a folder holds.
fn count_files(dir: &str) -> i32 {
    fs::read_dir(dir).map_or(0, |d| d.flatten().count() as i32)
}

/// The headless runner's annotate.* keys (app_probe forwards the ones it does not answer itself):
/// "result" (none, stale, opened, made, failed: what this start's request came to), "leftover"
/// (files left in the requests' folder), "book_page" ("12/300": the book page held by page index
/// open_page of the notebook this start's request opened; "" for a page that holds none, or no such
/// notebook). 0, or -1 for a key it lacks.
pub fn annotate_probe(key: &str, open_page: i32, out: &mut String, cap: usize) -> i32 {
    const OUTCOMES: [&str; 5] = ["none", "stale", "opened", "made", "failed"];
    const BOOKS: [&str; 5] = ["none", "stale", "running", "done", "failed"];
    match key {
        "result" => {
            let o = with(|s| s.outcome);
            csnprintf!(out, cap, "%s", OUTCOMES[o as usize]);
        }
        "book" => {
            let (on, book) = with(|s| (s.j.on, s.book));
            csnprintf!(out, cap, "%s", BOOKS[(if on { BookState::Running } else { book }) as usize]);
        }
        "book_added" => {
            let n = with(|s| s.book_added);
            csnprintf!(out, cap, "%d", n);
        }
        "book_leftover" => {
            let mut dir = String::new();
            book_dir(&mut dir, 700);
            csnprintf!(out, cap, "%d", count_files(&dir));
        }
        "leftover" => {
            let mut dir = String::new();
            request_dir(&mut dir, 700);
            csnprintf!(out, cap, "%d", count_files(&dir));
        }
        "book_page" => {
            out.clear();
            let last = with(|s| s.last_nb);
            let mut nb = NotebookInfo::default();
            let mut dir = String::new();
            let mut m = AnnotateMap::default();
            if !last.is_empty()
                && store_load_info(last.as_str(), &mut nb) == 0
                && open_page >= 0
                && open_page < nb.npages()
                && store_notebook_dir(nb.id.as_str(), &mut dir, 600) == 0
                && map_load(&dir, &mut m) == 0
            {
                for e in &m.e {
                    if e.pid == nb.page_ids[open_page as usize] {
                        csnprintf!(out, cap, "%d/%d", e.page, e.pages);
                    }
                }
                annotate_map_free(&mut m);
            }
        }
        _ => return -1,
    }
    0
}
