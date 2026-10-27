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

/// the requests' folder, in the data root
pub const ANNOTATE_DIR: &str = "import/.annotate";
/// the request, in it
pub const ANNOTATE_REQUEST: &str = "request.ini";
/// a request older (or newer) than this is not opened
pub const ANNOTATE_STALE_S: i64 = 300;
/// the most of a title a request is read with
pub const ANNOTATE_TITLE_LEN: usize = 512;

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
    pub picture: FixedStr<64>,
    /// when it was written (unix seconds); 0 when not given
    pub time: i64,
}

/// Reads a request's text. 0, or -1 and why (for the log: never the title).
/// (text: the file's bytes; a title need not be UTF-8, and annotate_name mends it.)
pub fn annotate_parse(text: &[u8], r: &mut AnnotateRequest, why: &mut &'static str) -> i32 {
    todo!()
}
/// Whether a request is too old (or too new: the clock was set back) to open at `now`: its time, or
/// file_time (the file's own) when it gives none, more than ANNOTATE_STALE_S away.
pub fn annotate_too_old(r: &AnnotateRequest, now: i64, file_time: i64) -> bool {
    todo!()
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

/// Reads source.ini's text into m (NULL: an empty map). -1 out of memory.
pub fn annotate_map_parse(text: Option<&[u8]>, m: &mut AnnotateMap) -> i32 {
    todo!()
}
/// The text to write: entries for pages no longer in nb (deleted) are left out. Caller frees.
pub fn annotate_map_text(m: &AnnotateMap, nb: &NotebookInfo) -> Option<Vec<u8>> {
    todo!()
}
pub fn annotate_map_free(m: &mut AnnotateMap) {
    todo!()
}
/// Notebook id's source.ini into m (none: an empty map). -1 when it is there but does not read, or
/// out of memory (m is then empty too).
pub fn annotate_map_load(id: &str, m: &mut AnnotateMap) -> i32 {
    todo!()
}
/// The page that holds book page `page` of `pages` (an id in nb's page list), or 0.
pub fn annotate_map_find(m: &AnnotateMap, nb: &NotebookInfo, page: i32, pages: i32) -> u32 {
    todo!()
}
pub fn annotate_map_add(m: &mut AnnotateMap, page: i32, pages: i32, pid: u32) -> i32 {
    todo!()
}
/// Where a new page for book page `page` of `pages` goes in nb's page list: just before the first
/// page that holds a later book page (by its place in the book: page 12 of 300 before page 50 of
/// 400), so the pages written after an annotated page (notes on it) stay with it; at the end when no
/// page holds a later one.
pub fn annotate_insert_at(nb: &NotebookInfo, m: &AnnotateMap, page: i32, pages: i32) -> i32 {
    todo!()
}

/// A new notebook's name: the title on one line (control characters as spaces, one space between two
/// words, none first or last), without broken UTF-8, cut to the name limit between two characters,
/// never inside one. Every other character stays, also one Grimoire's fonts cannot draw (Chinese,
/// Japanese, Korean): the screens leave it out where they draw the name (ui_name_label), and the
/// PDF's file name keeps it. Empty when nothing is left (the caller names it Untitled).
pub fn annotate_name(title: &[u8], out: &mut String, cap: usize) {
    todo!()
}

/// Whether data is a PNG whose chunks are all there, in order, and whose critical ones (IHDR, PLTE,
/// IDAT, IEND) check out against their CRCs, with nothing after IEND: a file cut short or damaged is
/// caught before anything is made of it. 0 with its size, or -1 and why.
pub fn annotate_png_check(data: &[u8], w: &mut i32, h: &mut i32, why: &mut &'static str) -> i32 {
    todo!()
}

// ---- at startup ----

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnnotateOutcome {
    /// no request
    #[default]
    None = 0,  // ANNOTATE_NONE
    /// a request this start was not asked to open, or too old: deleted unopened
    Stale = 1,  // ANNOTATE_STALE
    /// the page was there already
    Opened = 2,  // ANNOTATE_OPENED
    /// a page was added, in a notebook made for the book or in its own
    Made = 3,  // ANNOTATE_MADE
    /// nothing was made (the notebook stays as it was); msg says why
    Failed = 4,  // ANNOTATE_FAILED
}

/// Why nothing was made (FAILED), for the toast the caller shows.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnnotateWhy {
    /// see grimoire.log (the request does not read, a notebook's files do not)
    #[default]
    Log = 0,  // ANNOTATE_WHY_LOG
    /// the picture is missing, cut short, damaged, or no picture Grimoire takes
    Picture = 1,  // ANNOTATE_WHY_PICTURE
    /// a file could not be written: the storage is probably full
    Storage = 2,  // ANNOTATE_WHY_STORAGE
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
    todo!()
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
pub const BOOK_DIR: &str = "import/.book";
/// the request, in it
pub const BOOK_REQUEST: &str = "request.ini";

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

/// Reads a request's text. 0, or -1 and why (never the title). book_request_free frees it either way.
pub fn book_parse(text: &[u8], r: &mut BookRequest, why: &mut &'static str) -> i32 {
    todo!()
}
pub fn book_request_free(r: &mut BookRequest) {
    todo!()
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BookState {
    /// no request
    #[default]
    None = 0,  // BOOK_NONE
    /// not asked for, or too old: deleted unopened
    Stale = 1,  // BOOK_STALE
    /// pages are being added: call book_step until it is DONE or FAILED
    Running = 2,  // BOOK_RUNNING
    /// the notebook holds the book's pages: open nb at pid
    Done = 3,  // BOOK_DONE
    /// nothing changed (the notebook stays as it was); why says why
    Failed = 4,  // BOOK_FAILED
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

/// The request in grimoire/import/.book/, if any, at startup (after annotate_take). asked: KOReader's
/// plugin started Grimoire to open it (GRIMOIRE_BOOK=1, hosted); else, or too old, it is deleted
/// unopened. RUNNING: the book's notebook found (its compaction stopped) or a new one begun.
pub fn book_start(asked: bool, st: &mut BookStatus) {
    todo!()
}
/// Adds pages for about budget_ms; then st says how far it got. At DONE or FAILED the request's
/// folder is emptied.
pub fn book_step(budget_ms: u32, st: &mut BookStatus) {
    todo!()
}
/// Pictures of another size than their notebook's pages (a notebook from the other Scribe size) are
/// fitted in the background, one queued at a time while the job queue is idle: call it often.
pub fn book_fits_tick() {
    todo!()
}

/// The headless runner's annotate.* keys (app_probe forwards the ones it does not answer itself):
/// "result" (none, stale, opened, made, failed: what this start's request came to), "leftover"
/// (files left in the requests' folder), "book_page" ("12/300": the book page held by page index
/// open_page of the notebook this start's request opened; "" for a page that holds none, or no such
/// notebook). 0, or -1 for a key it lacks.
pub fn annotate_probe(key: &str, open_page: i32, out: &mut String, cap: usize) -> i32 {
    todo!()
}
