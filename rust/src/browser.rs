//! The file picker (2.1.0-beta.2, lane L8; DESIGN.md 2.2, 2.8, 3.2, 3.6; areas/browser.md Phase 1): Library > New >
//! From a file... opens Files in pick mode, hosted by the Library as a mode the way it hosts the Graph. It shows
//! a root of four places (Import, Documents, Downloads, All storage), the folders under them and, in them, the
//! PDFs and folders a task can use: a tap on a folder enters it, a tap on a PDF opens its card (the one place a
//! tap offers choices: an import can be 400 pages, and the card is the confirmation), a hold on a folder (or its
//! `...`) opens the card that imports its pictures. The Import place lists `grimoire/import` with a status on
//! each row and a Look again row.
//!
//! Everything it reads of the storage goes through fsview.rs: the walk (no link anywhere, one device, Grimoire's
//! notebooks, PDFs and trash refused by identity), listings of names and kinds only in slices of at most 20 ms,
//! stats and book keys only for the rows on screen. The storage outside Grimoire's folders is read only, here and
//! in the jobs that follow a pick. No file or folder name reaches grimoire.log.
//!
//! The model (`Browser`'s rows, listing and statuses) needs no screen: the tests drive it with `list_now`.

use crate::fsview::*;
use crate::prelude::*;

// Hit ids.
const B_BACK: i32 = 1;
const B_ROW: i32 = 2;
const B_MORE: i32 = 3;
const B_PREV: i32 = 4;
const B_NEXT: i32 = 5;
const B_CARD_IMPORT: i32 = 6;
const B_CARD_OPEN: i32 = 7;
const B_CARD_PICTURES: i32 = 8;
const B_CARD_NOTEBOOK: i32 = 9;
const B_CARD_CANCEL: i32 = 10;

/// What a task can pick (browser.md 4.3: 2.2 adds Picture and Font).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Pdf,
    PictureFolder,
}

/// What the picker was asked for.
#[derive(Clone, Copy, Debug)]
pub struct PickSpec {
    pub kinds: &'static [PickKind],
}

/// What a step of the picker says to its host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserResult {
    None,
    /// Back at the picker's root: nothing picked
    Cancel,
    /// Import as notebook, on this PDF
    Pdf(StoragePath),
    /// Import pictures as notebook, on this folder
    Pictures(StoragePath),
    /// Open its notebook (the PDF's notes): this notebook's id
    OpenNotebook(String),
    /// the Import place's Look again row
    LookAgain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// a root row: Import, Documents, Downloads, All storage
    Place,
    Dir,
    Pdf,
    /// the Import place's last row
    Look,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceKind {
    Import,
    Documents,
    Downloads,
    All,
}

/// What an Import place row says (its probe name in parentheses).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowStatus {
    None,
    /// ("waiting")
    Waiting,
    /// ("importing")
    Importing(u32, u32),
    /// ("failed")
    Failed,
    /// ("imported")
    Imported,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub kind: RowKind,
    /// the name as shown (UTF-8: a name that is not is never listed)
    pub name: String,
    pub path: StoragePath,
    place: Option<PlaceKind>,
    /// its place in the listing's entries (for its stat)
    ix: usize,
    pub stat: Option<FsStat>,
    /// its stat was asked for (a file gone since the listing never gets one, and is not asked again at every tick)
    stat_tried: bool,
    /// a folder's pictures: None until counted
    pub pictures: Option<usize>,
    /// a PDF's book key: None until read; Some("") when it could not be
    key: Option<String>,
    /// the notebook that holds this PDF (its id), once the key is known
    pub notebook: Option<String>,
    pub status: RowStatus,
}

impl Row {
    fn new(kind: RowKind, name: &str, path: StoragePath) -> Row {
        Row { kind, name: name.to_string(), path, place: None, ix: usize::MAX, stat: None, stat_tried: false, pictures: None, key: None, notebook: None, status: RowStatus::None }
    }
}

/// The item card: what a PDF or a folder offers (popout.rs places it beside its row).
#[derive(Clone, Debug, Default)]
struct Card {
    /// the row (an index into rows) it belongs to
    row: usize,
    rect: Rect,
    hits: HitList,
    /// the pointer went down inside it
    inside: bool,
    pressed: Option<Rect>,
}

#[derive(Default)]
pub struct Browser {
    pub open: bool,
    kinds: Vec<PickKind>,
    /// None: the root's four rows; else the folder shown
    at: Option<StoragePath>,
    /// the folder the way in started at (a root row's), whose Back is the root's rows
    place_root: Option<StoragePath>,
    pub rows: Vec<Row>,
    storage: Option<FsStorage>,
    listing: Option<FsListing>,
    /// the listing is read, and the rows built
    ready: bool,
    capped: bool,
    pub too_long: usize,
    pub hidden: usize,
    /// listing began (now_ms), and the Loading notice asked
    loading_since: u32,
    loading_notice: bool,
    screen: usize,
    hits: HitList,
    press: Press,
    pressed: Option<Hit>,
    pressed_rect: Option<Rect>,
    pressed_at: u32,
    card: Option<Card>,
    free: Option<(u64, u64)>,
    /// the notebooks, for "has notes"
    books: Vec<NotebookInfo>,
    /// their book keys (a notebook's `source=`, else its source.ini's book), read once for the picker's session
    book_keys: Option<Vec<(String, String)>>,
    /// the Import place's statuses are due (its rows were just built), and when they were last looked at: the
    /// PDF rows' when the import's progress changes, every row's every few seconds
    status_due: bool,
    status_at: u32,
    status_stamp: u32,
    /// how far the listing's entries of no known kind have been stat-ed (in slices, before the rows are built)
    stat_from: usize,
    /// the lift of a contact that opened the card by a hold is not a tap on anything
    swallow: bool,
    /// where each row on screen was last drawn (a status or count that arrives redraws only that row)
    row_rects: Vec<(usize, Rect)>,
    /// something to say to the user once drawn (a toast)
    pub toast: Option<&'static str>,
}

fn root_dir() -> Option<(String, String)> {
    let root = crate::pdfimport::storage_root()?;
    Some((root, store_root()))
}

fn is_pdf(n: &[u8]) -> bool {
    n.len() > 4 && n[n.len() - 4..].eq_ignore_ascii_case(b".pdf")
}

fn is_picture(n: &[u8]) -> bool {
    [b".png".as_slice(), b".jpg", b".jpeg", b".pgm"].iter().any(|e| n.len() > e.len() && n[n.len() - e.len()..].eq_ignore_ascii_case(e))
}

/// A size as the rows show it: "%d KB", "%.1f MB" or "%.1f GB".
pub fn size_label(b: u64) -> String {
    if b < 1 << 20 {
        // TRANSLATORS: a file's size in kilobytes, in the file picker's rows and cards (small text).
        cformat!(tr("%d KB"), ((b + 1023) >> 10).max(1) as i32)
    } else if b < 1 << 30 {
        // TRANSLATORS: a file's size in megabytes with one decimal ("14.2 MB"), in the file picker's rows and cards.
        cformat!(tr("%.1f MB"), b as f64 / (1u64 << 20) as f64)
    } else {
        // TRANSLATORS: a size in gigabytes with one decimal ("9.8 GB"), in the file picker's rows, cards and free space line.
        cformat!(tr("%.1f GB"), b as f64 / (1u64 << 30) as f64)
    }
}

fn u(v: f32) -> i32 {
    ui_px(v)
}

impl Browser {
    // ---- opening and listing ----

    /// Opens the picker at its root. False when there is no storage to browse.
    pub fn open_pick(&mut self, spec: PickSpec) -> bool {
        *self = Browser::default();
        let Some((root, grimoire)) = root_dir() else { return false };
        let Ok(st) = FsStorage::open(&root, &grimoire) else { return false };
        self.free = st.free_bytes().zip(st.total_bytes());
        self.storage = Some(st);
        self.kinds = spec.kinds.to_vec();
        self.open = true;
        store_list(&mut self.books);
        self.show_root();
        true
    }

    pub fn close(&mut self) {
        self.listing = None;
        self.storage = None;
        self.card = None;
        self.open = false;
        self.rows = Vec::new();
    }

    fn import_path(&self) -> Option<StoragePath> {
        StoragePath::new(format!("{}/import", self.storage.as_ref()?.grimoire_rel()?.to_string_lossy()).as_bytes())
    }

    /// Enters the folder `p` as a tap on it would (the tests'): false, with the toast, when the walk refuses it.
    pub fn go(&mut self, p: StoragePath) -> bool {
        let ok = self.enter(p, false, now_ms());
        self.list_now();
        ok
    }

    /// Reads the folder shown again (Look again: the scan may have moved files).
    pub fn relist(&mut self, now: u32) {
        self.card = None;
        match self.at.clone() {
            Some(p) => {
                let keep = self.screen;
                self.enter(p, false, now);
                self.list_now();
                self.screen = keep;
            }
            None => self.show_root(),
        }
    }

    fn show_root(&mut self) {
        self.at = None;
        self.place_root = None;
        self.listing = None;
        self.card = None;
        self.screen = 0;
        self.capped = false;
        self.too_long = 0;
        self.hidden = 0;
        let mut rows = Vec::new();
        let Some(st) = self.storage.as_ref() else { return };
        let import = self.import_path();
        if let Some(p) = import {
            let mut r = Row::new(RowKind::Place, "Import", p);
            r.place = Some(PlaceKind::Import);
            rows.push(r);
        }
        for (name, kind) in [("documents", PlaceKind::Documents), ("Downloads", PlaceKind::Downloads)] {
            let Some(p) = StoragePath::new(name.as_bytes()) else { continue };
            if st.stat(&p, true).is_ok_and(|s| s.mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32) {
                let mut r = Row::new(RowKind::Place, name, p);
                r.place = Some(kind);
                rows.push(r);
            }
        }
        let mut r = Row::new(RowKind::Place, "All storage", StoragePath::root());
        r.place = Some(PlaceKind::All);
        rows.push(r);
        self.rows = rows;
        self.ready = true;
        self.free = st.free_bytes().zip(st.total_bytes());
    }

    /// Enters folder `p` (a root row's place when `from_root`). False, with the toast "Couldn't open folder.", when
    /// the walk refuses it (a link, another device, one of Grimoire's own folders, gone).
    fn enter(&mut self, p: StoragePath, from_root: bool, now: u32) -> bool {
        let Some(st) = self.storage.as_ref() else { return false };
        match FsListing::start(st, &p, true) {
            Ok(l) => {
                if from_root {
                    self.place_root = Some(p.clone());
                }
                self.listing = Some(l);
                self.at = Some(p);
                self.rows = Vec::new();
                self.ready = false;
                self.card = None;
                self.screen = 0;
                self.capped = false;
                self.too_long = 0;
                self.hidden = 0;
                self.loading_since = now;
                self.stat_from = 0;
                self.step_listing(FS_SLICE_MS);
                true
            }
            Err(_) => {
                self.toast = Some(tr("Couldn't open folder."));
                false
            }
        }
    }

    /// Reads the listing for at most `budget_ms`; the rows are built once it is read. An entry whose kind the
    /// directory did not say (d_type unknown) is its stat's, within the same budget a tick: thousands of them never
    /// hold the UI past the 20 ms rule.
    fn step_listing(&mut self, budget_ms: u32) {
        let Some(l) = self.listing.as_mut() else { return };
        let t0 = real_now_ms();
        if !l.step(budget_ms) {
            return;
        }
        while self.stat_from < l.entries.len() {
            if real_now_ms().wrapping_sub(t0) >= budget_ms {
                return;
            }
            let i = self.stat_from;
            if l.entries[i].kind == FsKind::Unknown {
                l.stat(i..i + 1, u32::MAX);
            }
            self.stat_from += 1;
        }
        self.build_rows();
    }

    /// The tests': the folder read to its end at once.
    pub fn list_now(&mut self) {
        while self.listing.is_some() && !self.ready {
            self.step_listing(FS_SLICE_MS * 50);
        }
    }

    fn build_rows(&mut self) {
        let (Some(l), Some(at)) = (self.listing.as_mut(), self.at.clone()) else { return };
        self.capped = l.capped;
        self.hidden = l.hidden;
        self.too_long = l.too_long;
        // (a kind d_type left unknown was its stat's: step_listing)
        let mut dirs: Vec<Row> = Vec::new();
        let mut pdfs: Vec<Row> = Vec::new();
        let mut odd = 0;
        for (ix, e) in l.entries.iter().enumerate() {
            let Ok(name) = std::str::from_utf8(&e.name) else {
                odd += 1;
                continue;
            };
            let Some(path) = at.join(&e.name) else { continue };
            match e.kind {
                FsKind::Dir => {
                    let mut r = Row::new(RowKind::Dir, name, path);
                    r.ix = ix;
                    r.stat = e.stat;
                    dirs.push(r);
                }
                FsKind::File if self.kinds.contains(&PickKind::Pdf) && is_pdf(&e.name) => {
                    let mut r = Row::new(RowKind::Pdf, name, path);
                    r.ix = ix;
                    r.stat = e.stat;
                    pdfs.push(r);
                }
                _ => {}
            }
        }
        dirs.sort_by(|a, b| name_cmp(&a.name, &b.name));
        pdfs.sort_by(|a, b| name_cmp(&a.name, &b.name));
        if odd > 0 || l.hidden > 0 {
            qlog!("files: %d names left out (links, refused folders, odd names, long paths: %d, %d of them odd)", (odd + l.hidden) as i32, l.hidden as i32, odd as i32);
        }
        if l.too_long > 0 {
            // TRANSLATORS: a toast in the file picker: some names in this folder are left out because their path
            // would be too long for Grimoire to hand to its import (one line).
            self.toast = Some(tr("Path too long."));
        }
        self.rows = dirs;
        self.rows.extend(pdfs);
        if self.in_import_place() {
            self.rows.push(Row::new(RowKind::Look, "Look again", at));
        }
        self.ready = true;
        self.status_due = true;
    }

    fn in_import_place(&self) -> bool {
        self.at.as_ref().is_some_and(|p| self.import_path().is_some_and(|i| *p == i))
    }

    /// Under grimoire/import (the Import place or a folder in it), where rows carry statuses.
    fn under_import(&self) -> bool {
        match (self.at.as_ref(), self.import_path()) {
            (Some(p), Some(i)) => {
                let (pb, ib) = (p.as_bytes(), i.as_bytes());
                pb.len() >= ib.len() && pb[..ib.len()].eq_ignore_ascii_case(ib) && (pb.len() == ib.len() || pb[ib.len()] == b'/')
            }
            _ => false,
        }
    }

    /// Whether the Import place (not a folder of it) is the screen: the staying notice stays off it, as its rows say it.
    pub fn import_place_up(&self) -> bool {
        self.open && self.in_import_place()
    }

    /// Up one level: false at the root rows (the host cancels).
    pub fn up(&mut self, now: u32) -> bool {
        if self.card.is_some() {
            self.card = None;
            return true;
        }
        let Some(mut at) = self.at.clone() else { return false };
        self.listing = None;
        // up to the first folder above that still opens (one renamed or deleted over USB is passed by), else the
        // root's rows: Back and Esc always leave
        loop {
            if self.place_root.as_ref() == Some(&at) {
                self.show_root();
                return true;
            }
            match at.split_last() {
                Some((parent, _)) => {
                    if self.enter(parent.clone(), false, now) {
                        return true;
                    }
                    at = parent;
                }
                None => {
                    self.show_root();
                    return true;
                }
            }
        }
    }

    // ---- what a row says ----

    fn per_page(&self, a_h: i32) -> usize {
        let top = u(190.0) + u(40.0) + u(100.0) + u(10.0);
        let room = a_h - top - u(170.0) - u(20.0);
        (room / u(150.0)).max(1) as usize
    }

    pub fn pages(&self, a_h: i32) -> usize {
        self.rows.len().div_ceil(self.per_page(a_h)).max(1)
    }

    fn place_label(k: PlaceKind) -> &'static str {
        match k {
            // TRANSLATORS: the file picker's first row, the folder Grimoire watches (grimoire/import): PDFs and pictures
            // dropped in it become notebooks. Also the first word of its path line (a row of 150 px at 44 px).
            PlaceKind::Import => tr("Import"),
            // TRANSLATORS: the file picker's row for the Kindle's documents folder (44 px in a row).
            PlaceKind::Documents => tr("Documents"),
            // TRANSLATORS: the file picker's row for the Kindle's Downloads folder (44 px in a row).
            PlaceKind::Downloads => tr("Downloads"),
            // TRANSLATORS: the file picker's row for the whole Kindle storage, less Grimoire's own notebooks (44 px in a row).
            PlaceKind::All => tr("All storage"),
        }
    }

    /// A row's name as it is drawn.
    pub fn row_label(&self, r: &Row) -> String {
        match (r.kind, r.place) {
            (RowKind::Place, Some(k)) => Self::place_label(k).to_string(),
            // TRANSLATORS: the Import place's last row: it looks in grimoire/import for new files now (44 px in a row).
            (RowKind::Look, _) => tr("Look again").to_string(),
            _ => r.name.clone(),
        }
    }

    /// The second line of a row: a place's real path, a PDF's size and date, a folder's pictures, an Import row's status.
    pub fn row_sub(&self, r: &Row) -> String {
        match r.kind {
            RowKind::Place => match r.place {
                Some(PlaceKind::Import) => format!("{}", String::from_utf8_lossy(r.path.as_bytes())),
                Some(PlaceKind::All) => String::new(),
                _ => String::from_utf8_lossy(r.path.as_bytes()).into_owned(),
            },
            RowKind::Look => String::new(),
            RowKind::Pdf | RowKind::Dir if self.under_import() && r.status != RowStatus::None => self.status_text(&r.status),
            RowKind::Pdf => {
                let Some(s) = r.stat else { return String::new() };
                let mut day = String::new();
                format_date_short(s.mtime, &mut day, 64);
                format!("{}, {}", size_label(s.size), day)
            }
            RowKind::Dir => match r.pictures {
                Some(n) if n > 0 => {
                    // TRANSLATORS: a folder in the file picker that holds pictures; %d is how many (small text under its name).
                    cformat!(ng_("%d picture", "%d pictures", n as i64), n as i32)
                }
                _ => String::new(),
            },
        }
    }

    fn status_text(&self, s: &RowStatus) -> String {
        match s {
            RowStatus::None => String::new(),
            // TRANSLATORS: a row of the Import place: this file is not imported yet (small text under its name).
            RowStatus::Waiting => tr("Waiting").to_string(),
            // TRANSLATORS: a row of the Import place while its PDF becomes a notebook: the page it is on, of how many.
            RowStatus::Importing(p, n) => cformat!(tr("Importing: page %d of %d"), *p as i32, *n as i32),
            // TRANSLATORS: a row of the Import place: this PDF could not become a notebook; %s is the log that says why.
            RowStatus::Failed => cformat!(tr("Couldn't import PDF (see %s)."), "grimoire.log"),
            // TRANSLATORS: a row of the Import place: this file became a notebook.
            RowStatus::Imported => tr("Imported").to_string(),
        }
    }

    fn status_key(s: &RowStatus) -> &'static str {
        match s {
            RowStatus::None => "",
            RowStatus::Waiting => "waiting",
            RowStatus::Importing(..) => "importing",
            RowStatus::Failed => "failed",
            RowStatus::Imported => "imported",
        }
    }

    /// The path line: Files, the place, then the folders below it.
    fn crumbs(&self) -> String {
        let Some(at) = self.at.as_ref() else { return String::new() };
        let files = tr("Files");
        let comps: Vec<String> = at.components().map(|c| String::from_utf8_lossy(c).into_owned()).collect();
        let import = self.import_path();
        let mut skip = 0;
        let place = if let Some(i) = import.as_ref().filter(|i| at.as_bytes().len() >= i.as_bytes().len() && at.as_bytes()[..i.as_bytes().len()].eq_ignore_ascii_case(i.as_bytes())) {
            skip = i.components().count();
            Some(PlaceKind::Import)
        } else if comps.first().is_some_and(|c| c.eq_ignore_ascii_case("documents")) {
            skip = 1;
            Some(PlaceKind::Documents)
        } else if comps.first().is_some_and(|c| c.eq_ignore_ascii_case("Downloads")) {
            skip = 1;
            Some(PlaceKind::Downloads)
        } else {
            Some(PlaceKind::All)
        };
        let mut out = files.to_string();
        if let Some(k) = place {
            out.push_str(" > ");
            out.push_str(Self::place_label(k));
        }
        for c in comps.iter().skip(skip) {
            out.push_str(" > ");
            out.push_str(c);
        }
        out
    }

    // ---- the idle tick: stats, counts, keys and statuses for the rows on screen ----

    /// Rows on the current screen.
    fn visible(&self, a_h: i32) -> std::ops::Range<usize> {
        let per = self.per_page(a_h);
        let first = (self.screen * per).min(self.rows.len());
        first..(first + per).min(self.rows.len())
    }

    /// One tick of work: the listing read on, then (once it is read) one row's stat, picture count or book key,
    /// each at most a slice of 20 ms, the rows on screen only. Returns the rows (indices) that changed.
    pub fn work(&mut self, a_h: i32, now: u32) -> Vec<usize> {
        let mut changed = Vec::new();
        if !self.ready {
            self.step_listing(FS_SLICE_MS);
            return changed;
        }
        let vis = self.visible(a_h);
        // stats first (they give a PDF its size and date), in one slice
        if let Some(l) = self.listing.as_mut() {
            let t0 = real_now_ms();
            for i in vis.clone() {
                let r = &mut self.rows[i];
                if r.kind == RowKind::Place || r.kind == RowKind::Look || r.stat.is_some() || r.stat_tried || r.ix == usize::MAX {
                    continue;
                }
                if !l.stat(r.ix..r.ix + 1, FS_SLICE_MS) {
                    return changed; // (out of time: this row's stat at the next tick)
                }
                r.stat = l.entries.get(r.ix).and_then(|e| e.stat);
                r.stat_tried = true;
                changed.push(i);
                if real_now_ms().wrapping_sub(t0) >= FS_SLICE_MS {
                    return changed;
                }
            }
        }
        // picture counts, one folder a tick
        if self.kinds.contains(&PickKind::PictureFolder) {
            for i in vis.clone() {
                if self.rows[i].kind == RowKind::Dir && self.rows[i].pictures.is_none() {
                    let p = self.rows[i].path.clone();
                    let n = self.count_pictures(&p);
                    self.rows[i].pictures = Some(n);
                    if n > 0 {
                        changed.push(i);
                    }
                    return changed;
                }
            }
        }
        // book keys, one PDF a tick
        for i in vis {
            if self.rows[i].kind == RowKind::Pdf && self.rows[i].key.is_none() && self.rows[i].stat.is_some() {
                let p = self.rows[i].path.clone();
                let key = self.book_key(&p);
                let nb = self.notebook_of_key(&key);
                let r = &mut self.rows[i];
                r.notebook = nb;
                r.key = Some(key);
                if r.notebook.is_some() {
                    changed.push(i);
                }
                return changed;
            }
        }
        // the Import place's statuses: every row's when the rows are first known and then every 5 s (a folder's
        // lists it), the PDF rows' as soon as the import's progress changes (looked at at most once a second)
        if self.under_import() && (self.status_due || now.wrapping_sub(self.status_at) >= 1000) {
            let stamp = crate::pdfimport::import_progress_stamp();
            let all = self.status_due || now.wrapping_sub(self.status_at) >= 5000;
            if !all && stamp == self.status_stamp {
                return changed;
            }
            self.status_due = false;
            self.status_stamp = stamp;
            if all {
                self.status_at = now;
            }
            for i in self.visible(a_h) {
                if !all && self.rows[i].kind != RowKind::Pdf {
                    continue;
                }
                let s = self.status_of(i);
                if self.rows[i].status != s {
                    self.rows[i].status = s;
                    changed.push(i);
                }
            }
        }
        changed
    }

    /// The pictures a folder holds (counted through the walk).
    fn count_pictures(&self, p: &StoragePath) -> usize {
        let Some(st) = self.storage.as_ref() else { return 0 };
        let Ok(mut l) = FsListing::start(st, p, true) else { return 0 };
        let t0 = real_now_ms();
        while !l.step(FS_SLICE_MS) {
            if real_now_ms().wrapping_sub(t0) > 200 {
                break;
            }
        }
        l.entries.iter().filter(|e| matches!(e.kind, FsKind::File | FsKind::Unknown) && is_picture(&e.name)).count().min(STORE_MAX_PAGES + 1)
    }

    /// The notebook that holds a book key ("has notes"): the notebooks' keys are read once for the picker's session
    /// (a notebook's `source=`, else the book its source.ini names: one small read each), not once a row.
    fn notebook_of_key(&mut self, key: &str) -> Option<String> {
        if key.is_empty() {
            return None;
        }
        if self.book_keys.is_none() {
            let keys = self
                .books
                .iter()
                .filter_map(|nb| {
                    if !nb.source.is_empty() {
                        return Some((nb.source.as_str().to_string(), nb.id.as_str().to_string()));
                    }
                    let mut m = AnnotateMap::default();
                    let k = (annotate_map_load(nb.id.as_str(), &mut m) == 0 && !m.book.is_empty()).then(|| m.book.as_str().to_string());
                    annotate_map_free(&mut m);
                    k.map(|k| (k, nb.id.as_str().to_string()))
                })
                .collect();
            self.book_keys = Some(keys);
        }
        self.book_keys.as_ref().and_then(|v| v.iter().find(|(k, _)| k == key).map(|(_, id)| id.clone()))
    }

    /// A PDF's book key (KOReader's partial MD5, 12 reads of 1 KiB), "" when the file cannot be read.
    fn book_key(&self, p: &StoragePath) -> String {
        let Some(st) = self.storage.as_ref() else { return String::new() };
        match st.open_file(p, true) {
            Ok(fd) => {
                use std::os::fd::AsRawFd;
                crate::md5::partial_md5_fd(fd.as_raw_fd())
            }
            Err(_) => String::new(),
        }
    }

    /// What the Import place says of row `i`, from the marks, the queue and the attempts.
    fn status_of(&self, i: usize) -> RowStatus {
        let r = &self.rows[i];
        let in_import = self.in_import_place();
        let done_dir = self.at.as_ref().is_some_and(|p| {
            self.import_path().is_some_and(|ip| {
                let done = format!("{}/done", String::from_utf8_lossy(ip.as_bytes()));
                let pb = p.as_bytes();
                pb.len() >= done.len() && pb[..done.len()].eq_ignore_ascii_case(done.as_bytes()) && (pb.len() == done.len() || pb[done.len()] == b'/')
            })
        });
        match r.kind {
            RowKind::Pdf => {
                if done_dir {
                    return RowStatus::Imported;
                }
                let Some(s) = r.stat else { return RowStatus::None };
                use crate::pdfimport::ImportStatus as S;
                match crate::pdfimport::import_status(&String::from_utf8_lossy(r.path.as_bytes()), s.size, s.mtime) {
                    S::Waiting => RowStatus::Waiting,
                    S::Importing(p, n) => RowStatus::Importing(p, n),
                    S::Failed => RowStatus::Failed,
                    S::Imported => RowStatus::Imported,
                }
            }
            RowKind::Dir if in_import => {
                if r.name.eq_ignore_ascii_case("done") {
                    return RowStatus::Imported;
                }
                match r.pictures {
                    Some(n) if n > 0 => {
                        let abs = root_dir().map(|(root, _)| format!("{}/{}", root.trim_end_matches('/'), String::from_utf8_lossy(r.path.as_bytes())));
                        if abs.is_some_and(|p| crate::bgwork_template::folder_failed(&p)) {
                            RowStatus::Failed
                        } else {
                            RowStatus::Waiting
                        }
                    }
                    _ => RowStatus::None,
                }
            }
            _ => RowStatus::None,
        }
    }

    // ---- the item card ----

    fn open_card(&mut self, row: usize) {
        let Some(r) = self.rows.get(row) else { return };
        if !matches!(r.kind, RowKind::Pdf | RowKind::Dir) {
            return;
        }
        // a PDF's book key is read now if the tick has not yet
        if r.kind == RowKind::Pdf && r.key.is_none() {
            let p = r.path.clone();
            let key = self.book_key(&p);
            let nb = self.notebook_of_key(&key);
            let r = &mut self.rows[row];
            r.notebook = nb;
            r.key = Some(key);
        }
        if self.rows[row].kind == RowKind::Dir && self.rows[row].pictures.is_none() && self.kinds.contains(&PickKind::PictureFolder) {
            let p = self.rows[row].path.clone();
            let n = self.count_pictures(&p);
            self.rows[row].pictures = Some(n);
        }
        self.card = Some(Card { row, ..Default::default() });
    }

    /// The card's lines: details, then its rows (hit id, icon, label).
    fn card_lines(&self, row: usize) -> (String, Vec<(i32, IconKind, String)>) {
        let r = &self.rows[row];
        let mut rows: Vec<(i32, IconKind, String)> = Vec::new();
        let details;
        match r.kind {
            RowKind::Pdf => {
                details = self.row_sub_plain(r);
                if r.notebook.is_some() {
                    // TRANSLATORS: a PDF's card in the file picker: a notebook already holds this book, and this opens it
                    // (a row of about 800 px at 44 px).
                    rows.push((B_CARD_NOTEBOOK, IconKind::Notebook, tr("Open its notebook").to_string()));
                } else {
                    // TRANSLATORS: a PDF's card in the file picker: this makes a notebook of its pages (a row of about 800 px at 44 px).
                    rows.push((B_CARD_IMPORT, IconKind::Download, tr("Import as notebook").to_string()));
                }
            }
            _ => {
                details = r.name.clone();
                rows.push((B_CARD_OPEN, IconKind::Folder, tr("Open").to_string()));
                let n = r.pictures.unwrap_or(0);
                if n > 0 {
                    // TRANSLATORS: a folder's card in the file picker: this makes one notebook of the %d pictures in it,
                    // in their order (a row of about 800 px at 44 px).
                    rows.push((B_CARD_PICTURES, IconKind::Download, cformat!(ng_("Import %d picture as notebook", "Import %d pictures as notebook", n as i64), n as i32)));
                }
            }
        }
        rows.push((B_CARD_CANCEL, IconKind::Close, tr("Cancel").to_string()));
        (details, rows)
    }

    fn row_sub_plain(&self, r: &Row) -> String {
        let Some(s) = r.stat else { return String::new() };
        let mut day = String::new();
        format_date_short(s.mtime, &mut day, 64);
        format!("{}, {}", size_label(s.size), day)
    }

    /// Checks the row again before acting on it ("File changed. Try again."): its stat now against the listing's.
    fn unchanged(&mut self, row: usize) -> bool {
        let (Some(st), Some(r)) = (self.storage.as_ref(), self.rows.get(row)) else { return false };
        match (r.stat, st.stat(&r.path, true)) {
            (Some(was), Ok(now)) if r.kind == RowKind::Pdf => was.same(&now),
            (_, Ok(now)) => now.mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32,
            _ => false,
        }
    }

    // ---- drawing ----

    /// The screen: header, path line, rows, pager, and the card when one is open.
    pub fn draw(&mut self, a: &mut App) {
        hits_clear(&mut self.hits);
        self.row_rects.clear();
        let (aw, ah) = (a.w, a.h);
        let per = self.per_page(ah);
        self.screen = self.screen.min(self.rows.len().saturating_sub(1) / per);
        let c = &mut a.screen;
        canvas_fill(c, Rect::new(0, 0, aw, ah), 255);
        let th = u(190.0);
        let m = u(50.0);
        let bh = u(150.0);
        let by = (th - bh) / 2;
        let back = Rect::new(m, by, bh, bh);
        ui_button(c, back, IconKind::Back, "", ButtonStyle::Plain);
        hits_add(&mut self.hits, back, B_BACK, 0);
        // TRANSLATORS: the title of the file picker opened by New notebook's From a file... (the Library's header,
        // about 1400 px at 56 px).
        let title = tr("Import a PDF or pictures");
        let tx = back.x + back.w + u(40.0);
        ui_text(c, FontFace::Title, ui_title_size() * 1.2, ui_small_size(), Rect::new(tx, 0, aw - tx - m, th), title, 2, false, 0);
        ui_rule(c, 0, aw, th, u(4.0), 0);
        ui_frieze(c, 0, aw, th + u(9.0), u(28.0), 0);
        // the band: Files and the free space at the root, else the path
        let band = Rect::new(m, th + u(40.0), aw - 2 * m, u(100.0));
        if self.at.is_none() {
            let free = self.free;
            let low = free.is_some_and(|(f, _)| fsview_nearly_full(f));
            let left = if low {
                // TRANSLATORS: the file picker's top line when the Kindle has under 200 MB free (one line, about 800 px at 44 px).
                tr("Kindle storage nearly full.")
            } else {
                // TRANSLATORS: the file picker's name, the first word of its path line (small, in a band of 100 px).
                tr("Files")
            };
            font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(band.x, band.y, band.w / 2, band.h), left, 0);
            if let Some((f, t)) = free {
                // TRANSLATORS: the file picker's free space line: free of the whole storage ("Free: 9.8 GB of 14.5 GB"), at 44 px, right-aligned.
                let line = cformat!(tr("Free: %s of %s"), size_label(f).as_str(), size_label(t).as_str());
                let w = font_measure(FontFace::Regular, ui_text_size(), &line).min(band.w - u(40.0));
                font_draw_left(c, FontFace::Regular, ui_text_size(), Rect::new(band.x + band.w - w, band.y, w, band.h), &line, 0);
            }
        } else {
            let crumbs = self.crumbs();
            let cw = if self.capped { band.w * 55 / 100 } else { band.w };
            font_draw_left_name(c, FontFace::Bold, ui_text_size(), Rect::new(band.x, band.y, cw, band.h), &crumbs, 0);
            if self.capped {
                // TRANSLATORS: a folder in the file picker held more than it lists; %d is how many it shows (5000), at 34 px, right of the path.
                let line = cformat!(tr("Showing the first %d."), FS_LIST_MAX as i32);
                let r = Rect::new(band.x + cw, band.y, band.w - cw, band.h);
                ui_text(c, FontFace::Regular, ui_small_size(), ui_small_size() * 0.8, r, &line, 2, false, 0);
            }
        }
        // rows
        let first = self.screen * per;
        let y0 = band.y + band.h + u(10.0);
        let c = &mut a.screen;
        if self.rows.is_empty() && self.ready {
            // TRANSLATORS: a folder in the file picker with nothing in it a PDF import can use (one line, 44 px, centred).
            font_draw_centered(c, FontFace::Bold, ui_text_size(), Rect::new(m, y0 + u(120.0), aw - 2 * m, u(100.0)), tr("Nothing here to use."), 0);
        }
        for k in 0..per {
            let i = first + k;
            if i >= self.rows.len() {
                break;
            }
            let r = Rect::new(m, y0 + k as i32 * u(150.0), aw - 2 * m, u(150.0) - u(12.0));
            self.draw_row(&mut a.screen, r, i);
            self.row_rects.push((i, r));
            hits_add(&mut self.hits, r, B_ROW, i as i32);
            if self.has_card(i) {
                let mr = Rect::new(r.x + r.w - u(130.0), r.y + u(14.0), u(110.0), r.h - u(28.0));
                hits_add(&mut self.hits, mr, B_MORE, i as i32);
            }
        }
        // pager
        let pages = self.pages(ah);
        let y = ah - u(170.0);
        let c = &mut a.screen;
        ui_rule(c, 0, aw, y - u(10.0), u(3.0), 0);
        let pr = Rect::new(m, y + u(15.0), u(220.0), u(170.0) - u(30.0));
        let nr = Rect::new(aw - m - u(220.0), pr.y, u(220.0), pr.h);
        ui_button(c, pr, IconKind::Prev, "", if self.screen > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        ui_button(c, nr, IconKind::Next, "", if self.screen + 1 < pages { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        if self.screen > 0 {
            hits_add(&mut self.hits, pr, B_PREV, 0);
        }
        if self.screen + 1 < pages {
            hits_add(&mut self.hits, nr, B_NEXT, 0);
        }
        // TRANSLATORS: between the Library's page buttons (1320 px at 44 px).
        let pl = cformat!(tr("Page %d of %d"), self.screen as i32 + 1, pages as i32);
        font_draw_centered(c, FontFace::Bold, ui_text_size(), Rect::new(pr.x + pr.w, pr.y, nr.x - pr.x - pr.w, pr.h), &pl, 0);
        if self.card.is_some() {
            self.draw_card(a);
        }
        a.toast_floor = 0;
    }

    fn has_card(&self, i: usize) -> bool {
        self.rows.get(i).is_some_and(|r| matches!(r.kind, RowKind::Pdf | RowKind::Dir))
    }

    fn draw_row(&self, c: &mut Canvas, r: Rect, i: usize) {
        let row = &self.rows[i];
        canvas_fill(c, Rect::new(r.x - u(4.0), r.y - u(4.0), r.w + u(8.0), r.h + u(8.0)), 255);
        ui_frame(c, r, u(12.0), 0, false, u(3.0));
        let icon = match (row.kind, row.place) {
            (RowKind::Place, Some(PlaceKind::Import)) => IconKind::Download,
            (RowKind::Place, Some(PlaceKind::Documents)) => IconKind::Folder,
            (RowKind::Place, Some(PlaceKind::Downloads)) => IconKind::Download,
            (RowKind::Place, _) => IconKind::Library,
            (RowKind::Dir, _) => IconKind::Folder,
            (RowKind::Pdf, _) => IconKind::Paper,
            (RowKind::Look, _) => IconKind::Update,
        };
        ui_icon(c, icon, Rect::new(r.x + u(20.0), r.y + u(20.0), u(90.0), r.h - u(40.0)), 0);
        let right = if self.has_card(i) { u(150.0) } else { u(30.0) };
        let tx = r.x + u(140.0);
        let tw = r.w - u(140.0) - right;
        let label = self.row_label(row);
        let sub = self.row_sub(row);
        let notes = row.kind == RowKind::Pdf && row.notebook.is_some();
        let (name_r, sub_r) = if sub.is_empty() && !notes {
            (Rect::new(tx, r.y, tw, r.h), RECT_EMPTY)
        } else {
            (Rect::new(tx, r.y + u(10.0), tw, r.h * 55 / 100), Rect::new(tx, r.y + r.h * 55 / 100, tw, r.h * 40 / 100))
        };
        if matches!(row.kind, RowKind::Place | RowKind::Look) {
            font_draw_left(c, FontFace::Bold, ui_text_size(), name_r, &label, 0);
        } else {
            font_draw_left_name(c, FontFace::Bold, ui_text_size(), name_r, &label, 0);
        }
        if !rect_is_empty(sub_r) {
            let mut sw = sub_r.w;
            if notes {
                // TRANSLATORS: a PDF in the file picker that a notebook already holds (its notes are there); a tag at the
                // right of the row under its name (about 260 px at 34 px).
                let t = tr("has notes");
                let w = (font_measure(FontFace::Bold, ui_small_size(), t) + u(20.0)).min(sub_r.w / 2);
                font_draw_left(c, FontFace::Bold, ui_small_size(), Rect::new(sub_r.x + sub_r.w - w, sub_r.y, w, sub_r.h), t, 0);
                sw = sub_r.w - w - u(20.0);
            }
            let r2 = Rect::new(sub_r.x, sub_r.y, sw, sub_r.h);
            if row.kind == RowKind::Place || row.kind == RowKind::Dir {
                font_draw_left_name(c, FontFace::Regular, ui_small_size(), r2, &sub, 0);
            } else {
                font_draw_left(c, FontFace::Regular, ui_small_size(), r2, &sub, 0);
            }
        }
        if self.has_card(i) {
            let mr = Rect::new(r.x + r.w - u(130.0), r.y + u(14.0), u(110.0), r.h - u(28.0));
            ui_button(c, mr, IconKind::More, "", ButtonStyle::Plain);
        }
    }

    /// Row `i` again, alone (a count, status or key arrived): the rect to refresh, empty when it is not on screen.
    pub fn redraw_row(&mut self, a: &mut App, i: usize) -> Rect {
        let Some(&(_, r)) = self.row_rects.iter().find(|(k, _)| *k == i) else { return RECT_EMPTY };
        if self.card.is_some() {
            return RECT_EMPTY; // (the card covers it: the next draw has it right)
        }
        self.draw_row(&mut a.screen, r, i);
        rect_inflate(r, u(4.0))
    }

    fn draw_card(&mut self, a: &mut App) {
        let Some(card) = self.card.take() else { return };
        let (details, lines) = self.card_lines(card.row);
        let (aw, ah) = (a.w, a.h);
        let cw = (aw - 2 * u(40.0)).min(u(980.0));
        let rh = u(130.0);
        let gap = u(12.0);
        let ch = u(24.0) * 2 + u(80.0) + lines.len() as i32 * (rh + gap) - gap;
        let anchor = self.row_rects.iter().find(|(k, _)| *k == card.row).map_or(Rect::new(aw / 4, ah / 3, aw / 2, u(138.0)), |x| x.1);
        let rect = popout_place(cw, ch, anchor, Rect::new(0, 0, aw, ah), &[]);
        let c = &mut a.screen;
        ui_frame(c, rect, u(24.0), 255, true, 0);
        ui_frame(c, rect, u(24.0), 0, false, u(6.0));
        let inner = rect_inflate(rect, -u(24.0));
        let mut hits = HitList::default();
        font_draw_left_name(c, FontFace::Regular, ui_small_size(), Rect::new(inner.x + u(8.0), inner.y, inner.w - u(16.0), u(80.0)), &details, 0);
        let mut y = inner.y + u(80.0);
        for (id, icon, label) in &lines {
            let r = Rect::new(inner.x, y, inner.w, rh);
            let primary = *id != B_CARD_CANCEL && *id != B_CARD_OPEN;
            ui_frame(c, r, u(16.0), 255, true, 0);
            ui_frame(c, r, u(16.0), 0, false, if primary { u(6.0) } else { u(3.0) });
            ui_icon(c, *icon, Rect::new(r.x + u(20.0), r.y + u(15.0), rh - u(30.0), rh - u(30.0)), 0);
            ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), Rect::new(r.x + rh + u(10.0), r.y, r.w - rh - u(30.0), r.h), label, 2, false, 0);
            hits_add(&mut hits, r, *id, 0);
            y += rh + gap;
        }
        self.card = Some(Card { rect, hits, ..card });
    }

    /// Where the card is (to refresh it alone), empty with none.
    pub fn card_rect(&self) -> Rect {
        self.card.as_ref().map_or(RECT_EMPTY, |c| c.rect)
    }

    // ---- contacts ----

    /// A pen or finger on the screen (phase 0 down, 1 move, 2 up).
    pub fn pointer(&mut self, a: &mut App, ph: i32, ev: &InEvent) -> BrowserResult {
        if self.card.is_some() {
            return self.card_pointer(a, ph, ev);
        }
        if ph == 0 {
            self.swallow = false;
            press_begin(&mut self.press, ev.x, ev.y, ev.t_ms);
            self.pressed_at = now_ms();
            self.pressed = hits_find(&self.hits, ev.x, ev.y).copied();
            self.pressed_rect = None;
            if let Some(h) = self.pressed {
                app_press_feedback(a, h.r);
                self.pressed_rect = Some(h.r);
            }
            return BrowserResult::None;
        }
        press_move(&mut self.press, ev.x, ev.y);
        if ph != 2 {
            return BrowserResult::None;
        }
        // (the pressed look comes off before what the tap does draws)
        let rect = app_press_release(a, &mut self.pressed_rect);
        if !rect_is_empty(rect) {
            app_invalidate(a, rect, RefreshMode::Ui);
        }
        let Some(h) = self.pressed.take() else { return BrowserResult::None };
        if !(press_is_tap(&self.press, h.r) || rect_contains(h.r, ev.x as i32, ev.y as i32)) {
            return BrowserResult::None;
        }
        self.act(a, h)
    }

    fn act(&mut self, a: &mut App, h: Hit) -> BrowserResult {
        let now = now_ms();
        let ah = a.h;
        match h.id {
            B_BACK => {
                if !self.up(now) {
                    return BrowserResult::Cancel;
                }
                self.redraw_full(a);
            }
            B_PREV | B_NEXT => {
                self.turn(h.id == B_NEXT, ah);
                self.redraw(a, RefreshMode::Ui);
            }
            B_ROW | B_MORE => {
                let i = h.arg as usize;
                let Some(row) = self.rows.get(i).cloned() else { return BrowserResult::None };
                match row.kind {
                    RowKind::Place => {
                        if self.enter(row.path.clone(), true, now) {
                            self.redraw_full(a);
                        } else {
                            self.flush_toast(a);
                        }
                    }
                    RowKind::Dir if h.id == B_ROW => {
                        if self.enter(row.path.clone(), false, now) {
                            self.redraw_full(a);
                        } else {
                            self.flush_toast(a);
                        }
                    }
                    // a PDF's tap, and any `...`: the card (the exception to "a tap acts": two taps to import)
                    RowKind::Pdf | RowKind::Dir => {
                        self.open_card(i);
                        self.show_card(a);
                    }
                    RowKind::Look => {
                        self.status_at = 0;
                        return BrowserResult::LookAgain;
                    }
                }
            }
            _ => {}
        }
        BrowserResult::None
    }

    /// The card opened or changed: the screen drawn into the panel's buffer, and the card's rect alone refreshed.
    fn show_card(&mut self, a: &mut App) {
        self.draw(a);
        let r = self.card_rect();
        app_invalidate(a, r, RefreshMode::Ui);
    }

    fn card_pointer(&mut self, a: &mut App, ph: i32, ev: &InEvent) -> BrowserResult {
        if self.swallow && ph >= 0 {
            // (the contact that held a row open the card: its lift is nothing)
            if ph == 2 {
                self.swallow = false;
            }
            return BrowserResult::None;
        }
        let Some(card) = self.card.as_mut() else { return BrowserResult::None };
        if ph == 0 {
            press_begin(&mut self.press, ev.x, ev.y, ev.t_ms);
            card.inside = rect_contains(card.rect, ev.x as i32, ev.y as i32);
            card.pressed = None;
            if let Some(h) = hits_find(&card.hits, ev.x, ev.y).copied() {
                app_press_feedback(a, h.r);
                if let Some(c) = self.card.as_mut() {
                    c.pressed = Some(h.r);
                }
            }
            return BrowserResult::None;
        }
        press_move(&mut self.press, ev.x, ev.y);
        if ph != 2 {
            return BrowserResult::None;
        }
        let was_inside = card.inside;
        let rect = {
            let c = self.card.as_mut().unwrap();
            let mut p = c.pressed;
            let r = app_press_release(a, &mut p);
            c.pressed = None;
            r
        };
        if !rect_is_empty(rect) {
            app_invalidate(a, rect, RefreshMode::Ui);
        }
        if !was_inside {
            // outside the card a lift closes it and does nothing else
            return self.close_card(a);
        }
        let card = self.card.as_ref().unwrap();
        let Some(h) = hits_find(&card.hits, self.press.x0, self.press.y0).copied() else { return BrowserResult::None };
        if !(press_is_tap(&self.press, h.r) || rect_contains(h.r, ev.x as i32, ev.y as i32)) {
            return BrowserResult::None;
        }
        let row = card.row;
        match h.id {
            B_CARD_CANCEL => self.close_card(a),
            B_CARD_OPEN => {
                let Some(r) = self.rows.get(row).cloned() else { return self.close_card(a) };
                self.card = None;
                if self.enter(r.path.clone(), false, now_ms()) {
                    self.redraw_full(a);
                } else {
                    self.flush_toast(a);
                }
                BrowserResult::None
            }
            B_CARD_IMPORT | B_CARD_PICTURES | B_CARD_NOTEBOOK => {
                if !self.unchanged(row) {
                    // the file changed since it was listed: the folder is read again
                    self.card = None;
                    self.toast = Some(tr("File changed. Try again."));
                    let at = self.at.clone();
                    if let Some(p) = at {
                        self.enter(p, false, now_ms());
                    }
                    self.redraw_full(a);
                    self.flush_toast(a);
                    return BrowserResult::None;
                }
                let r = &self.rows[row];
                let res = match h.id {
                    B_CARD_IMPORT => BrowserResult::Pdf(r.path.clone()),
                    B_CARD_PICTURES => BrowserResult::Pictures(r.path.clone()),
                    _ => r.notebook.clone().map_or(BrowserResult::None, BrowserResult::OpenNotebook),
                };
                self.card = None;
                res
            }
            _ => BrowserResult::None,
        }
    }

    fn close_card(&mut self, a: &mut App) -> BrowserResult {
        let r = self.card_rect();
        self.card = None;
        self.draw(a);
        app_invalidate(a, r, RefreshMode::Ui);
        BrowserResult::None
    }

    /// A finger held on a row (HOLD_MS, not moved): a folder's card, as its `...`. Called by the tick.
    pub fn hold_check(&mut self, a: &mut App, now: u32) -> bool {
        let Some(h) = self.pressed else { return false };
        if self.card.is_some() || h.id != B_ROW || self.press.moved || now.wrapping_sub(self.pressed_at) < HOLD_MS {
            return false;
        }
        let i = h.arg as usize;
        if !self.has_card(i) {
            return false;
        }
        // (the lift after a hold never acts: the press is let go now)
        self.pressed = None;
        self.swallow = true;
        let rect = app_press_release(a, &mut self.pressed_rect);
        if !rect_is_empty(rect) {
            app_invalidate(a, rect, RefreshMode::Ui);
        }
        self.open_card(i);
        self.show_card(a);
        true
    }

    fn turn(&mut self, next: bool, ah: i32) {
        let pages = self.pages(ah);
        self.card = None;
        if next && self.screen + 1 < pages {
            self.screen += 1;
        } else if !next && self.screen > 0 {
            self.screen -= 1;
        }
    }

    /// Left and Right (the pager's keys).
    pub fn turn_key(&mut self, a: &mut App, next: bool) {
        let before = self.screen;
        self.turn(next, a.h);
        if self.screen != before {
            self.redraw(a, RefreshMode::Ui);
        }
    }

    fn redraw(&mut self, a: &mut App, m: RefreshMode) {
        self.draw(a);
        app_invalidate_all(a, m);
    }

    fn redraw_full(&mut self, a: &mut App) {
        self.redraw(a, RefreshMode::Full);
    }

    fn flush_toast(&mut self, a: &mut App) {
        if let Some(t) = self.toast.take() {
            app_toast(a, t, 3000);
        }
    }

    /// Esc: the card, else up one level. False at the root rows (the host cancels).
    pub fn back(&mut self, a: &mut App) -> bool {
        if self.card.is_some() {
            self.close_card(a);
            return true;
        }
        if !self.up(now_ms()) {
            return false;
        }
        self.redraw_full(a);
        true
    }

    /// The idle tick: the listing read on, the rows on screen filled in, statuses, the Loading notice, a hold.
    pub fn tick(&mut self, a: &mut App, now: u32) {
        if !self.open {
            return;
        }
        self.hold_check(a, now);
        let was_ready = self.ready;
        let changed = self.work(a.h, now);
        if self.toast.is_some() {
            self.flush_toast(a);
        }
        // (the Loading notice goes once a folder is read, whichever: Back during a slow listing shows another)
        if self.ready && self.loading_notice {
            self.loading_notice = false;
            app_notice_clear(a, NoticeKey::Loading);
        }
        if !was_ready && self.ready {
            self.redraw_full(a);
            return;
        }
        if !self.ready {
            if now.wrapping_sub(self.loading_since) >= 1000 && !self.loading_notice {
                self.loading_notice = true;
                // TRANSLATORS: the staying notice while the file picker reads a big folder (one line, about 600 px at 44 px).
                app_notice(a, NoticeKey::Loading, tr("Loading..."));
            }
            return;
        }
        let mut d = RECT_EMPTY;
        for i in changed {
            d = rect_union(d, self.redraw_row(a, i));
        }
        if !rect_is_empty(d) {
            app_invalidate(a, d, RefreshMode::Ui);
        }
    }

    // ---- the runner's probes ----

    /// The runner's "files.*" keys (docs/SIM.md), the key without "files.".
    pub fn probe(&self, a_h: i32, key: &str, out: &mut String) -> bool {
        let per = self.per_page(a_h);
        let vis = self.visible(a_h);
        let nth = |n: &str| -> Option<&Row> {
            let n: usize = n.parse().ok()?;
            (n >= 1 && n <= vis.len()).then(|| &self.rows[vis.start + n - 1])
        };
        match key {
            "open" => out.push_str(if self.open { "1" } else { "0" }),
            "place" => out.push_str(&match &self.at {
                None => "root".to_string(),
                Some(p) => String::from_utf8_lossy(p.as_bytes()).into_owned(),
            }),
            "ready" => out.push_str(if self.ready { "1" } else { "0" }),
            "rows" => out.push_str(&self.rows.len().to_string()),
            "page" => out.push_str(&(self.screen + 1).to_string()),
            "pages" => out.push_str(&self.pages_for(per).to_string()),
            "per_page" => out.push_str(&per.to_string()),
            "hidden" => out.push_str(&self.hidden.to_string()),
            "capped" => out.push_str(if self.capped { "1" } else { "0" }),
            "crumbs" => out.push_str(&self.crumbs()),
            "free" => out.push_str(&self.free.map_or(0, |(f, _)| f).to_string()),
            "card" => out.push_str(&match &self.card {
                None => String::new(),
                Some(c) => match self.rows[c.row].kind {
                    RowKind::Pdf => "pdf".to_string(),
                    _ => "folder".to_string(),
                },
            }),
            "card.details" => {
                if let Some(c) = &self.card {
                    out.push_str(&self.card_lines(c.row).0);
                }
            }
            "card.rows" => {
                if let Some(c) = &self.card {
                    out.push_str(&self.card_lines(c.row).1.iter().map(|l| l.2.clone()).collect::<Vec<_>>().join("|"));
                }
            }
            _ => {
                let Some((what, n)) = key.split_once('.') else { return false };
                if !matches!(what, "row" | "sub" | "status" | "notes" | "kind" | "item" | "pictures") {
                    return false;
                }
                let r = if what == "item" { self.rows.iter().find(|r| r.name == n || r.name == n.replace('_', " ")) } else { nth(n) };
                let Some(r) = r else { return true };
                match what {
                    "row" | "item" => out.push_str(&self.row_label(r)),
                    "sub" => out.push_str(&self.row_sub(r)),
                    "status" => out.push_str(Self::status_key(&r.status)),
                    "notes" => out.push_str(if r.notebook.is_some() { "1" } else { "0" }),
                    "pictures" => out.push_str(&r.pictures.map_or(String::new(), |n| n.to_string())),
                    _ => out.push_str(match r.kind {
                        RowKind::Place => "place",
                        RowKind::Dir => "folder",
                        RowKind::Pdf => "pdf",
                        RowKind::Look => "look",
                    }),
                }
            }
        }
        true
    }

    fn pages_for(&self, per: usize) -> usize {
        self.rows.len().div_ceil(per).max(1)
    }

    /// The point a runner tap on a control named so lands on (docs/SIM.md): back, prev, next, row.N, more.N,
    /// item.NAME, and the card's import, open, pictures, notebook and cancel.
    pub fn point(&self, name: &str) -> Option<(f32, f32)> {
        let centre = |h: &Hit| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32);
        if let Some(c) = &self.card {
            let id = match name {
                "card.import" => B_CARD_IMPORT,
                "card.open" => B_CARD_OPEN,
                "card.pictures" => B_CARD_PICTURES,
                "card.notebook" => B_CARD_NOTEBOOK,
                "card.cancel" => B_CARD_CANCEL,
                _ => 0,
            };
            if id != 0 {
                return c.hits.h.iter().find(|h| h.id == id).map(centre);
            }
        }
        let find = |id: i32, arg: i32| self.hits.h.iter().find(|h| h.id == id && h.arg == arg).map(centre);
        match name {
            "back" => find(B_BACK, 0),
            "prev" => find(B_PREV, 0),
            "next" => find(B_NEXT, 0),
            _ => {
                let (what, n) = name.split_once('.')?;
                // the rows on screen, in order: their B_ROW hits
                let rows: Vec<&Hit> = self.hits.h.iter().filter(|h| h.id == B_ROW).collect();
                let i = match what {
                    "row" | "more" => rows.get(n.parse::<usize>().ok()?.checked_sub(1)?)?.arg,
                    "item" => self.rows.iter().position(|r| r.name == n || r.name == n.replace('_', " "))? as i32,
                    _ => return None,
                };
                find(if what == "more" { B_MORE } else { B_ROW }, i)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdfimport::{import_status, ImportStatus};

    // A storage root with Grimoire's folder in it, set as the app's (the process's: one test at a time).
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn storage(name: &str) -> (String, std::sync::MutexGuard<'static, ()>) {
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let base = std::env::temp_dir().join(format!("grimoire-browser-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("grimoire");
        std::fs::create_dir_all(root.join("import")).unwrap();
        std::env::set_var("GRIMOIRE_STORAGE", &base);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        crate::pdfimport::ui_reset();
        (base.to_str().unwrap().to_string(), g)
    }

    // What an Import row says: Waiting for a file nothing knows, Waiting for a pick that waits for the lock,
    // Imported for a done mark, Failed once its attempts are used up and it has not changed.
    #[test]
    fn an_import_row_says_waiting_imported_or_failed() {
        let (base, _g) = storage("status");
        let f = format!("{base}/grimoire/import/A.pdf");
        std::fs::write(&f, b"%PDF-1.4\n").unwrap();
        let src = "grimoire/import/A.pdf";
        let md = std::fs::metadata(&f).unwrap();
        use std::os::unix::fs::MetadataExt;
        let (size, mtime) = (md.len(), md.mtime());
        assert_eq!(import_status(src, size, mtime), ImportStatus::Waiting);
        let m = |state| crate::pdfimport::Mark { state, src: src.into(), pick: false, at: 1, size, mtime, ..Default::default() };
        assert!(crate::pdfimport::mark_write_public(&m(crate::pdfimport::MarkState::Waiting)));
        assert_eq!(import_status(src, size, mtime), ImportStatus::Waiting);
        assert!(crate::pdfimport::mark_write_public(&m(crate::pdfimport::MarkState::Done)));
        assert_eq!(import_status(src, size, mtime), ImportStatus::Imported);
        // a done mark is that file's: another file of the name since is not imported
        assert_eq!(import_status(src, size + 1, mtime), ImportStatus::Waiting);
        crate::pdfimport::mark_drop(src);
        // three tries with these very contents: Failed, until the file changes
        let h = {
            let mut h: u64 = 14695981039346656037;
            for b in size.to_ne_bytes().iter().chain(mtime.to_ne_bytes().iter()) {
                h = (h ^ *b as u64).wrapping_mul(1099511628211);
            }
            h
        };
        for _ in 0..3 {
            crate::bgwork_template::attempts_note_public(src, h, 0);
        }
        assert_eq!(import_status(src, size, mtime), ImportStatus::Failed);
        assert_eq!(import_status(src, size + 1, mtime), ImportStatus::Waiting);
    }

    // The Import place is the screen only at grimoire/import itself (the staying notice stays off it there).
    #[test]
    fn the_notice_stays_off_the_import_place_only() {
        let (base, _g) = storage("place");
        std::fs::create_dir_all(format!("{base}/grimoire/import/Sub")).unwrap();
        std::fs::create_dir_all(format!("{base}/documents")).unwrap();
        let mut b = Browser::default();
        assert!(b.open_pick(PickSpec { kinds: &[PickKind::Pdf, PickKind::PictureFolder] }));
        assert!(!b.import_place_up());
        assert!(b.go(StoragePath::new(b"grimoire/import").unwrap()));
        assert!(b.import_place_up());
        assert!(b.go(StoragePath::new(b"grimoire/import/Sub").unwrap()));
        assert!(!b.import_place_up());
        assert!(b.go(StoragePath::new(b"documents").unwrap()));
        assert!(!b.import_place_up());
    }

    #[test]
    fn sizes_read_in_kb_mb_and_gb() {
        i18n_set("en");
        assert_eq!(size_label(0), "1 KB");
        assert_eq!(size_label(1500), "2 KB");
        assert_eq!(size_label(14_900_000), "14.2 MB");
        assert_eq!(size_label(5 << 30), "5.0 GB");
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("browser.rs").contains('\u{2014}'));
    }
}
