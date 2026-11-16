//! The file browser (the picker since 2.1.0-beta.2, Files since 2.2, issue #124; DESIGN.md 2.2, 2.8, 3.2 and
//! 3.6), hosted by the Library as a mode the way it hosts the Graph. Two modes. Pick mode: Library > New > From a
//! file... asks for a PDF or a folder of pictures; its root has four places (Import, Documents, Downloads, All
//! storage), the folders under them and, in them, the PDFs and folders a task can use: a tap on a folder enters
//! it, a tap on a PDF opens its card (the one place a tap offers choices: an import can be 400 pages, and the card
//! is the confirmation), a hold on a folder (or its `...`) opens the card that imports its pictures. The Import
//! place lists `grimoire/import` with a status on each row and a Look again row. Place mode (2.2): Files, opened
//! from the Library's Files card or the `files` action, where the user browses: its root adds Grimoire's own
//! places (Templates, Photos, Fonts, PDF copies, Trash) and the Kindle's fonts, every one read only here. A picture's
//! card copies it into Grimoire's templates or photos (Use as template, Use as photo: the copy job, bgwork_copy.rs),
//! and the Templates and Photos sheets' From a file... tile opens the picture pick (2.2, issue #146): pictures only,
//! its card offering that sheet's copy.
//!
//! Everything it reads of the storage goes through fsview.rs: the walk (no link anywhere, one device, Grimoire's
//! notebooks refused by identity, and in pick mode its PDFs and trash too), listings of names and kinds only in
//! slices of at most 20 ms, stats and book keys only for the rows on screen. The storage outside Grimoire's folders
//! is read only, here and in the jobs that follow a pick. No file or folder name reaches grimoire.log.
//!
//! The model (`Browser`'s rows, listing and statuses) needs no screen: the tests drive it with `list_now`.

use crate::bgwork_copy::{copy_refusal, copy_request, CopyFail, CopyTo};
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
// Files (2.2): the path bar's crumbs (arg: the crumb's index in crumb_list), its Sort and Show chips, and a row of
// the Go to sheet (arg: its index in goto).
const B_CRUMB: i32 = 11;
const B_SORT: i32 = 12;
const B_SHOW: i32 = 13;
const B_GOTO: i32 = 14;
// A picture's card (#146): Use as template, Use as photo. (The ids are used by name only, here and by the runner,
// which taps `card.template` and `card.photo`: to move them, change the one number. No two ids may be equal.)
const B_CARD_TEMPLATE: i32 = 15;
const B_CARD_PHOTO: i32 = B_CARD_TEMPLATE + 1;

/// What a task can pick, or Files lists as usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Pdf,
    PictureFolder,
    /// a picture file (.png, .jpg, .jpeg, .pgm, as templates take them)
    Picture,
    /// a font file (.ttf, .otf)
    Font,
}

/// Pick mode (a task asks for a file) or place mode (Files: the user browses).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowseMode {
    #[default]
    Pick,
    Place,
}

/// What the browser was asked for: the kinds it offers, its title (translated by the caller), the folder it opens
/// at (None: its root), its mode, and for the picture pick the folder a picture picked is copied into (its card's
/// one copy).
#[derive(Clone, Debug)]
pub struct PickSpec {
    pub kinds: &'static [PickKind],
    pub title: &'static str,
    pub start: Option<StoragePath>,
    pub mode: BrowseMode,
    pub to: Option<CopyTo>,
}

/// What the Library's settings say of Files (settings.ini files_card, files_sort, files_show), handed to the
/// browser as it opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilesPrefs {
    /// the Files card is in the Library (the root then has no "Show Files in Library" row)
    pub card: bool,
    /// FILES_SORT_NAMES's index
    pub sort: i32,
    /// FILES_SHOW_NAMES's index: 1 every file
    pub show: i32,
}

impl PickSpec {
    /// New notebook's From a file...: a PDF or a folder of pictures to import.
    pub fn import() -> PickSpec {
        PickSpec {
            kinds: &[PickKind::Pdf, PickKind::PictureFolder],
            // TRANSLATORS: the title of the file picker opened by New notebook's From a file... (the Library's header,
            // about 1400 px at 56 px).
            title: tr("Import a PDF or pictures"),
            start: None,
            mode: BrowseMode::Pick,
            to: None,
        }
    }

    /// Files, the place (2.2): every kind Grimoire can use, from its root.
    pub fn place() -> PickSpec {
        PickSpec {
            kinds: &[PickKind::Pdf, PickKind::PictureFolder, PickKind::Picture, PickKind::Font],
            title: tr("Files"),
            start: None,
            mode: BrowseMode::Place,
            to: None,
        }
    }

    /// The Templates or Photos sheet's From a file... (2.2, #146): a picture, copied into grimoire/templates or
    /// grimoire/photos.
    pub fn picture(to: CopyTo) -> PickSpec {
        PickSpec {
            kinds: &[PickKind::Picture],
            // TRANSLATORS: (also) the title of the file picker the Templates and Photos sheets' From a file... tile
            // opens, to pick a picture to copy in (the Library's header, about 1400 px at 56 px).
            title: tr("From a file..."),
            start: None,
            mode: BrowseMode::Pick,
            to: Some(to),
        }
    }
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
    /// the root's "Show Files in Library" row: the host puts the Files card in the Library (files_card=1)
    ShowInLibrary,
    /// Files' Sort or Show chip changed `prefs`: the host saves them (files_sort, files_show)
    Prefs,
    /// A picture's Use as template or Use as photo (#146): the copy job was asked to copy it into this folder (the
    /// app's tick says when it ends); the picture pick is done
    Copying(CopyTo),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// a root row: Import, Documents, Downloads, All storage
    Place,
    Dir,
    Pdf,
    /// the Import place's last row
    Look,
    /// the root's last row while the Files card is not in the Library: "Show Files in Library"
    ShowCard,
    /// Files (2.2): a picture, a font, any other file (Show all), and a link (Show all: "link, not followed", never
    /// entered or opened)
    Picture,
    Font,
    Other,
    Link,
    /// an entry of Grimoire's trash, listed read only by the time in its name (a notebook by its name): never
    /// entered, no card yet (Restore comes later, with the storage's writes)
    Trashed,
}

/// A root row's place. Pick mode's root has Import, Documents, Downloads and All storage; place mode's has them all
/// (Grimoire's and the Kindle's folders only when they are there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceKind {
    Import,
    Documents,
    Downloads,
    Templates,
    Photos,
    Fonts,
    KindleFonts,
    PdfCopies,
    Trash,
    All,
}

/// The root's places in order, each with its folder: in Grimoire's folder (`true`) or at the storage root.
const PLACES: [(PlaceKind, bool, &str); 10] = [
    (PlaceKind::Import, true, "import"),
    (PlaceKind::Documents, false, "documents"),
    (PlaceKind::Downloads, false, "Downloads"),
    (PlaceKind::Templates, true, "templates"),
    (PlaceKind::Photos, true, "photos"),
    (PlaceKind::Fonts, true, "fonts"),
    (PlaceKind::KindleFonts, false, "fonts"),
    (PlaceKind::PdfCopies, true, "PDFs"),
    (PlaceKind::Trash, true, "trash"),
    (PlaceKind::All, false, ""),
];

/// A root row's name for the runner's `files.item.NAME` (the label drawn is translated): English, and the Kindle's
/// folders by their own names.
fn place_name(k: PlaceKind) -> &'static str {
    match k {
        PlaceKind::Import => "Import",
        PlaceKind::Documents => "documents",
        PlaceKind::Downloads => "Downloads",
        PlaceKind::Templates => "Templates",
        PlaceKind::Photos => "Photos",
        PlaceKind::Fonts => "Fonts",
        PlaceKind::KindleFonts => "Kindle fonts",
        PlaceKind::PdfCopies => "PDF copies",
        PlaceKind::Trash => "Trash",
        PlaceKind::All => "All storage",
    }
}

/// Whether `p` is `base` or inside it, comparing names without regard to ASCII case (as a case-insensitive
/// storage finds them), name by name.
fn path_under(p: &StoragePath, base: &StoragePath) -> bool {
    let (pb, bb) = (p.as_bytes(), base.as_bytes());
    bb.is_empty() || (pb.len() >= bb.len() && pb[..bb.len()].eq_ignore_ascii_case(bb) && (pb.len() == bb.len() || pb[bb.len()] == b'/'))
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
    /// that notebook was found by a record of this file's size (not by the key alone): only the whole file can
    /// say it is the same, so the card offers Import as notebook as well, and the import gives the answer
    pub also_import: bool,
    pub status: RowStatus,
    /// Files (2.2): the name a row shows in place of its file's (a trashed notebook's own, a trashed file's from its
    /// record; None until looked up, "" when there is none), the time in a trash entry's name, whether the entry is a
    /// folder, and what fsview's trash listing read from its name
    title: Option<String>,
    when: i64,
    dir: bool,
    trash: Option<TrashEntry>,
    /// what the card's details say of the file beyond its size and date, once known without reading it here (a
    /// font's family, from its check's verdict): None until looked, "" for nothing
    detail: Option<String>,
}

impl Row {
    fn new(kind: RowKind, name: &str, path: StoragePath) -> Row {
        Row {
            kind,
            name: name.to_string(),
            path,
            place: None,
            ix: usize::MAX,
            stat: None,
            stat_tried: false,
            pictures: None,
            key: None,
            notebook: None,
            also_import: false,
            status: RowStatus::None,
            title: None,
            when: 0,
            dir: kind == RowKind::Dir,
            trash: None,
            detail: None,
        }
    }
}

/// The item card: what a PDF or a folder offers (popout.rs places it beside its row).
#[derive(Clone, Debug, Default)]
struct Card {
    /// the row (an index into rows) it is drawn beside, and the file it was opened for: its path and its stat then.
    /// When the rows are made again (a new order, Show) the card follows that file to its row, or closes; what it
    /// acts on is that file, checked against that stat.
    row: usize,
    path: StoragePath,
    stat: Option<FsStat>,
    /// Files' Go to sheet (2.2) in place of a row's card
    goto: bool,
    rect: Rect,
    hits: HitList,
    /// the pointer went down inside it
    inside: bool,
    pressed: Option<Rect>,
}

#[derive(Default)]
pub struct Browser {
    pub open: bool,
    /// pick mode or place mode (Files), and the header's title
    pub mode: BrowseMode,
    title: &'static str,
    kinds: Vec<PickKind>,
    /// the picture pick's folder (PickSpec.to)
    to: Option<CopyTo>,
    /// the Library's settings for Files, as it opened (and as changed since)
    pub prefs: FilesPrefs,
    /// the rows were made from the listing (and, once ready, are the rows shown)
    built: bool,
    /// Files (2.2): the rows being made ready, stat-ed in slices for Sort by newest or size and then sorted, while
    /// the rows on screen stay as they are; how far their stats are
    next: Option<Vec<Row>>,
    next_from: usize,
    /// the Go to sheet's rows: label, folder, icon, and whether it is a place of the root
    goto: Vec<(String, StoragePath, IconKind, bool)>,
    /// where the path bar was drawn (the Go to sheet hangs from it), and the rows' names as last drawn (a probe's)
    path_bar: Rect,
    drawn: Vec<String>,
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
    /// rows a screen, as last drawn (a card that follows its file goes to that file's screen)
    per_last: usize,
    hits: HitList,
    press: Press,
    pressed: Option<Hit>,
    pressed_rect: Option<Rect>,
    pressed_at: u32,
    card: Option<Card>,
    free: Option<(u64, u64)>,
    /// the notebooks, for "has notes"
    books: Vec<NotebookInfo>,
    /// the keys they hold and the records of their files (pdfimport::key_holders), read once for the picker's
    /// session
    book_keys: Option<Vec<crate::pdfimport::KeyHolder>>,
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

fn is_font(n: &[u8]) -> bool {
    [b".ttf".as_slice(), b".otf"].iter().any(|e| n.len() > e.len() && n[n.len() - e.len()..].eq_ignore_ascii_case(e))
}

/// Files' order (2.2) of two rows (is a folder, name, stat): folders before files; then by name
/// (name_cmp: accents folded, digit runs read as numbers), the newest first (sort 1), or the largest first (sort 2;
/// folders by name); ties by name. A row with no stat (a link, a file gone since its folder was read) comes after
/// those with one.
pub fn files_cmp(a: (bool, &str, Option<FsStat>), b: (bool, &str, Option<FsStat>), sort: i32) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if a.0 != b.0 {
        return if a.0 { Ordering::Less } else { Ordering::Greater };
    }
    let key = match sort {
        1 => b.2.map(|s| s.mtime).cmp(&a.2.map(|s| s.mtime)),
        2 if !a.0 => b.2.map(|s| s.size).cmp(&a.2.map(|s| s.size)),
        _ => Ordering::Equal,
    };
    key.then_with(|| name_cmp(a.1, b.1))
}

/// Sorts rows: `how` 0 by name, 1 and 2 as files_cmp (the newest or the largest first). Every row sort goes through
/// here: one sort in the binary (the Trash place keeps the order of fsview's trash listing).
fn sort_rows(rows: &mut [Row], how: i32) {
    rows.sort_by(|a, b| match how {
        0 => name_cmp(&a.name, &b.name),
        _ => files_cmp((a.kind == RowKind::Dir, &a.name, a.stat), (b.kind == RowKind::Dir, &b.name, b.stat), how),
    });
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

/// The free space line: free of the whole storage.
fn free_line(free: u64, total: u64) -> String {
    // TRANSLATORS: the file picker's free space line: free of the whole storage ("Free: 9.8 GB of 14.5 GB"), at 44 px, right-aligned.
    cformat!(tr("Free: %s of %s"), size_label(free).as_str(), size_label(total).as_str())
}

/// The longest the Go to sheet's tap waits for the storage root's folders.
const GOTO_MS: u32 = 200;

/// The storage root for the Go to sheet, read at once (the tap waits for it) and all of it within GOTO_MS by
/// `clock`: names and kinds, and a stat only for an entry whose kind the folder did not say. A root that takes
/// longer gives the folders read by then (All storage lists them all).
fn root_listing(st: &FsStorage, pick: bool, clock: &mut dyn FnMut() -> u32) -> Option<FsListing> {
    let mut l = FsListing::start(st, &StoragePath::root(), pick).ok()?;
    let t0 = clock();
    loop {
        let left = GOTO_MS.saturating_sub(clock().wrapping_sub(t0));
        if left == 0 || l.step_clocked(FS_SLICE_MS.min(left), clock) {
            break;
        }
    }
    for i in 0..l.entries.len() {
        if l.entries[i].kind == FsKind::Unknown {
            let left = GOTO_MS.saturating_sub(clock().wrapping_sub(t0));
            if !l.stat_clocked(i..i + 1, left, clock) {
                break;
            }
        }
    }
    Some(l)
}

// (how many times this thread asked for the Files card's free space: a test's)
#[cfg(test)]
thread_local! {
    pub static FREE_LINE_READS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The storage's free space as Files' root says it, for the Library's Files card (2.2); None with no storage to
/// tell (one statvfs of the storage root: nothing is listed).
pub fn files_free_line() -> Option<String> {
    #[cfg(test)]
    FREE_LINE_READS.with(|n| n.set(n.get() + 1));
    let (root, _) = root_dir()?;
    let c = std::ffi::CString::new(root).ok()?;
    // SAFETY: sv is written by statvfs before it is read; c is NUL-terminated.
    let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut sv) } != 0 {
        return None;
    }
    Some(free_line(sv.f_bavail as u64 * sv.f_frsize as u64, sv.f_blocks as u64 * sv.f_frsize as u64))
}

impl Browser {
    // ---- opening and listing ----

    /// Opens the browser at its root with the Files card not in the Library (the tests'): open's.
    pub fn open_pick(&mut self, spec: PickSpec) -> bool {
        self.open(spec, FilesPrefs::default())
    }

    /// Opens the browser at its root, or at the spec's start when that opens, with the Library's settings for
    /// Files. False when there is no storage to browse.
    pub fn open(&mut self, spec: PickSpec, prefs: FilesPrefs) -> bool {
        *self = Browser::default();
        self.prefs = prefs;
        let Some((root, grimoire)) = root_dir() else { return false };
        let Ok(st) = FsStorage::open(&root, &grimoire) else { return false };
        self.free = st.free_bytes().zip(st.total_bytes());
        self.storage = Some(st);
        self.kinds = spec.kinds.to_vec();
        self.mode = spec.mode;
        self.title = spec.title;
        self.to = spec.to;
        self.open = true;
        store_list(&mut self.books);
        self.show_root();
        if let Some(p) = spec.start {
            let from_root = self.rows.iter().any(|r| r.kind == RowKind::Place && r.path == p);
            if !self.enter(p, from_root, now_ms()) {
                self.toast = None; // (a start that does not open is the root, said by nothing)
            }
        }
        true
    }

    /// The walk's pick flag: pick mode also refuses Grimoire's PDFs and trash, which Files shows read only.
    fn pick(&self) -> bool {
        self.mode == BrowseMode::Pick
    }

    pub fn close(&mut self) {
        self.listing = None;
        self.storage = None;
        self.card = None;
        self.open = false;
        self.rows = Vec::new();
    }

    fn import_path(&self) -> Option<StoragePath> {
        self.place_path(PlaceKind::Import)
    }

    /// A place's folder: in Grimoire's folder (when it is inside the storage) or at the storage root.
    fn place_path(&self, k: PlaceKind) -> Option<StoragePath> {
        let &(_, own, name) = PLACES.iter().find(|p| p.0 == k)?;
        if !own {
            return StoragePath::new(name.as_bytes());
        }
        StoragePath::new(format!("{}/{name}", self.storage.as_ref()?.grimoire_rel()?.to_string_lossy()).as_bytes())
    }

    /// Whether this browser offers what an import takes (a PDF, a folder of pictures): the picture pick does not.
    fn imports(&self) -> bool {
        self.kinds.contains(&PickKind::Pdf) || self.kinds.contains(&PickKind::PictureFolder)
    }

    /// The root's places in this mode, in order. (The picture pick has no Import: that place is the imports', its rows
    /// their statuses and Look again.)
    fn mode_places(&self) -> &'static [PlaceKind] {
        match self.mode {
            BrowseMode::Pick if !self.imports() => &[PlaceKind::Documents, PlaceKind::Downloads, PlaceKind::All],
            BrowseMode::Pick => &[PlaceKind::Import, PlaceKind::Documents, PlaceKind::Downloads, PlaceKind::All],
            BrowseMode::Place => &[
                PlaceKind::Import,
                PlaceKind::Documents,
                PlaceKind::Downloads,
                PlaceKind::Templates,
                PlaceKind::Photos,
                PlaceKind::Fonts,
                PlaceKind::KindleFonts,
                PlaceKind::PdfCopies,
                PlaceKind::Trash,
                PlaceKind::All,
            ],
        }
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
        self.next = None;
        self.built = true;
        self.card = None;
        self.drop_press();
        self.screen = 0;
        self.capped = false;
        self.too_long = 0;
        self.hidden = 0;
        let mut rows = Vec::new();
        let pick = self.pick();
        let Some(st) = self.storage.as_ref() else { return };
        // Import and All storage always; every other place when its folder is there (and opens: a link is no place)
        for &k in self.mode_places() {
            let Some(p) = self.place_path(k) else { continue };
            let always = matches!(k, PlaceKind::Import | PlaceKind::All);
            if always || st.stat(&p, pick).is_ok_and(|s| s.mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32) {
                let mut r = Row::new(RowKind::Place, place_name(k), p);
                r.place = Some(k);
                rows.push(r);
            }
        }
        // (the way to the Files card, hidden at first: in either mode, while it is not in the Library)
        if !self.prefs.card {
            rows.push(Row::new(RowKind::ShowCard, "Show Files in Library", StoragePath::root()));
        }
        self.rows = rows;
        self.ready = true;
        self.free = st.free_bytes().zip(st.total_bytes());
    }

    /// Enters folder `p` (a root row's place when `from_root`). False, with the toast "Couldn't open folder.", when
    /// the walk refuses it (a link, another device, one of Grimoire's own folders, gone).
    fn enter(&mut self, p: StoragePath, from_root: bool, now: u32) -> bool {
        let pick = self.pick();
        let Some(st) = self.storage.as_ref() else { return false };
        match FsListing::start(st, &p, pick) {
            Ok(l) => {
                if from_root {
                    self.place_root = Some(p.clone());
                }
                self.listing = Some(l);
                self.at = Some(p);
                self.rows = Vec::new();
                self.ready = false;
                self.built = false;
                self.next = None;
                self.card = None;
                self.drop_press();
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
                // TRANSLATORS: a toast in the file picker: the folder tapped could not be opened (gone since it was
                // listed, or one Grimoire never opens). One line of about 1000 px at 44 px.
                self.toast = Some(tr("Couldn't open folder."));
                false
            }
        }
    }

    /// Reads the listing for at most `budget_ms`; the rows are built once it is read. An entry whose kind the
    /// directory did not say (d_type unknown) is its stat's, within the same budget a tick: thousands of them never
    /// hold the UI past the 20 ms rule.
    fn step_listing(&mut self, budget_ms: u32) {
        if self.built {
            return;
        }
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
        while self.listing.is_some() && !self.built {
            self.step_listing(FS_SLICE_MS * 50);
        }
        while self.next.is_some() {
            self.sort_step(FS_SLICE_MS * 50);
        }
    }

    fn build_rows(&mut self) {
        let place = self.mode == BrowseMode::Place;
        let show_all = place && self.prefs.show == 1;
        let pdfs = self.kinds.contains(&PickKind::Pdf);
        let pictures = self.kinds.contains(&PickKind::Picture);
        let (Some(l), Some(at)) = (self.listing.as_mut(), self.at.clone()) else { return };
        self.capped = l.capped;
        self.hidden = l.hidden;
        self.too_long = l.too_long;
        // (a kind d_type left unknown was its stat's: step_listing)
        let mut dirs: Vec<Row> = Vec::new();
        let mut files: Vec<Row> = Vec::new();
        let mut odd = 0;
        for (ix, e) in l.entries.iter().enumerate() {
            let Ok(name) = std::str::from_utf8(&e.name) else {
                odd += 1;
                continue;
            };
            // pick mode: folders and its kinds (PDFs, or pictures); Files: pictures and fonts too, and with Show all
            // every file and link
            let kind = match e.kind {
                FsKind::Dir => RowKind::Dir,
                FsKind::File if pdfs && is_pdf(&e.name) => RowKind::Pdf,
                FsKind::File if pictures && is_picture(&e.name) => RowKind::Picture,
                FsKind::File if place && is_font(&e.name) => RowKind::Font,
                FsKind::File if show_all => RowKind::Other,
                FsKind::Link if show_all => RowKind::Link,
                _ => continue,
            };
            // (a row whose path an import cannot keep, pdfimport::name_storable, is no row either: counted with the
            // odd names)
            if !crate::pdfimport::name_storable(name) {
                odd += 1;
                continue;
            }
            let Some(path) = at.join(&e.name) else { continue };
            let mut r = Row::new(kind, name, path);
            r.ix = ix;
            r.stat = e.stat;
            if kind == RowKind::Dir {
                dirs.push(r);
            } else {
                files.push(r);
            }
        }
        // (the links the listing left out, by name: Show all lists them, never entered or opened, within the listing's
        // cap of 5,000 rows; a full list of links may have left more out: "Showing the first 5000." either way)
        if show_all {
            self.capped |= l.links.len() >= FS_LIST_MAX;
            for n in &l.links {
                if dirs.len() + files.len() >= FS_LIST_MAX {
                    self.capped = true;
                    break;
                }
                let (Ok(name), Some(path)) = (std::str::from_utf8(n), at.join(n)) else { continue };
                if crate::pdfimport::name_storable(name) {
                    files.push(Row::new(RowKind::Link, name, path));
                }
            }
        }
        sort_rows(&mut dirs, 0);
        sort_rows(&mut files, 0);
        if odd > 0 || l.hidden > 0 {
            qlog!("files: %d names left out (links, refused folders, odd names, long paths: %d, %d of them odd)", (odd + l.hidden) as i32, l.hidden as i32, odd as i32);
        }
        if l.too_long > 0 {
            // TRANSLATORS: a toast in the file picker: some names in this folder are left out because their path
            // would be too long for Grimoire to hand to its import (one line).
            self.toast = Some(tr("Path too long."));
        }
        dirs.extend(files);
        self.built = true;
        if place && self.in_trash_place() {
            // Grimoire's trash, read only, in the order of fsview's trash listing (no stat)
            let rows = self.trash_rows();
            self.set_rows(rows);
            return;
        }
        if place && self.prefs.sort != 0 && !dirs.is_empty() {
            // (by newest or size: every row's stat first, in slices; the rows shown stay as they are meanwhile)
            self.next = Some(dirs);
            self.next_from = 0;
        } else {
            self.set_rows(dirs);
        }
    }

    /// Whether the folder shown is Grimoire's trash itself (Files lists its entries read only).
    fn in_trash_place(&self) -> bool {
        self.at.as_ref().is_some_and(|p| self.place_path(PlaceKind::Trash).is_some_and(|t| *p == t))
    }

    /// The trash's entries as rows, read and ordered by fsview's trash listing (`fsview_trash_entries`: the newest
    /// first by the time in each name, a name of no known form last): a file or folder of Grimoire's folders
    /// (`files-<stamp>-<4 hex>`, beside its record), a notebook (`<id>-<time>`) and a page (`<id>-p<page>-<time>`),
    /// each with the time in its name. A name of another form is listed as it is, with no time.
    fn trash_rows(&self) -> Vec<Row> {
        let (Some(l), Some(at)) = (self.listing.as_ref(), self.at.as_ref()) else { return Vec::new() };
        let ix: std::collections::HashMap<&[u8], usize> = l.entries.iter().enumerate().map(|(i, e)| (e.name.as_slice(), i)).collect();
        let mut rows = Vec::new();
        for t in fsview_trash_entries(&l.entries) {
            let Some(&i) = ix.get(t.name.as_slice()) else { continue };
            let e = &l.entries[i];
            if !matches!(e.kind, FsKind::Dir | FsKind::File) {
                continue;
            }
            let (Ok(name), Some(path)) = (std::str::from_utf8(&e.name), at.join(&e.name)) else { continue };
            if !crate::pdfimport::name_storable(name) {
                continue;
            }
            let mut r = Row::new(RowKind::Trashed, name, path);
            r.ix = i;
            r.dir = e.kind == FsKind::Dir;
            r.when = t.time;
            r.trash = Some(t);
            rows.push(r);
        }
        rows
    }

    /// A trash entry's name as Files shows it: a file's or folder's own name (the last part of where its record says
    /// it was, `FsStorage::trash_record`, read for the rows on screen only), a notebook's own name (its notebook.ini in
    /// the trash, Grimoire's own small file, opened through the walk as the listing was: no link on the way, a plain
    /// file, never waited on; the name cut as the store cuts one), a page's notebook's name, else "" (the entry's
    /// name shows).
    fn trash_title(&self, r: &Row) -> String {
        use std::io::Read;
        let Some(t) = r.trash.as_ref() else { return String::new() };
        match t.kind {
            TrashKind::Page => {
                let nb = self.books.iter().find(|b| b.id.as_str() == t.id.as_str());
                return nb.map_or(String::new(), |b| b.name.as_str().to_string());
            }
            TrashKind::File => {
                let rec = self.storage.as_ref().and_then(|st| st.trash_record(&t.name).ok());
                let name = rec.and_then(|rec| rec.from.split_last().map(|(_, n)| String::from_utf8_lossy(n).into_owned()));
                return name.filter(|n| crate::pdfimport::name_storable(n)).unwrap_or_default();
            }
            TrashKind::Other => return String::new(),
            TrashKind::Notebook => {}
        }
        let ini = r.path.join(b"notebook.ini").filter(|_| r.dir);
        let fd = ini.and_then(|p| self.storage.as_ref()?.open_file(&p, self.pick()).ok());
        let Some(fd) = fd else { return String::new() };
        let mut text = Vec::new();
        if std::fs::File::from(fd).take(16 << 10).read_to_end(&mut text).is_err() {
            return String::new();
        }
        String::from_utf8_lossy(&text)
            .lines()
            .find_map(|l| l.strip_prefix("name="))
            .map(|n| cut_at_bytes(n.trim(), STORE_NAME_LEN - 1).to_string())
            .unwrap_or_default()
    }

    /// The rows shown from now on (the Import place's Look again after them), from the first screen. A press on the
    /// rows before is dropped (its row number names another file now); an open card follows the file it was opened
    /// for to its row (and its screen), with what was read of it since, or closes when that file is not listed now.
    fn set_rows(&mut self, mut rows: Vec<Row>) {
        if self.in_import_place() && self.imports() {
            if let Some(at) = self.at.clone() {
                rows.push(Row::new(RowKind::Look, "Look again", at));
            }
        }
        self.drop_press();
        self.screen = 0;
        if let Some(mut c) = self.card.take() {
            let old = self.card_row(&c).cloned();
            let same = |r: &Row| r.path == c.path && c.stat.zip(r.stat).is_none_or(|(a, b)| (a.dev, a.ino) == (b.dev, b.ino));
            match rows.iter().position(same) {
                _ if c.goto => self.card = Some(c),
                Some(j) => {
                    if let Some(o) = old {
                        let stat = rows[j].stat.or(o.stat);
                        rows[j] = Row { stat, ..o };
                    }
                    c.row = j;
                    self.screen = j / self.per_last.max(1);
                    self.card = Some(c);
                }
                None => {}
            }
        }
        self.rows = rows;
        self.ready = true;
        self.status_due = true;
    }

    /// A press under way is let go of without acting (the rows under it changed).
    fn drop_press(&mut self) {
        self.pressed = None;
        self.pressed_rect = None;
    }

    /// The row the open card is for, while that row is still its file's.
    fn card_row(&self, c: &Card) -> Option<&Row> {
        self.rows.get(c.row).filter(|r| !c.goto && r.path == c.path)
    }

    /// One slice of stats for the rows being made ready (Sort by newest or size, at most `budget_ms`); once each has
    /// its stat they are sorted and become the rows shown. True when they did.
    fn sort_step(&mut self, budget_ms: u32) -> bool {
        let t0 = real_now_ms();
        if let (Some(next), Some(l)) = (self.next.as_mut(), self.listing.as_mut()) {
            while self.next_from < next.len() {
                let r = &mut next[self.next_from];
                if r.ix != usize::MAX && r.stat.is_none() && !r.stat_tried {
                    if !l.stat(r.ix..r.ix + 1, budget_ms) {
                        return false; // (out of time: this row's stat at the next tick)
                    }
                    r.stat = l.entries.get(r.ix).and_then(|e| e.stat);
                    r.stat_tried = true;
                }
                self.next_from += 1;
                if real_now_ms().wrapping_sub(t0) >= budget_ms {
                    return false;
                }
            }
        }
        let Some(mut rows) = self.next.take() else { return false };
        let sort = self.prefs.sort;
        sort_rows(&mut rows, sort.clamp(0, 2));
        qlog!("files: %d rows sorted (%d) in %u ms", rows.len() as i32, sort, real_now_ms().wrapping_sub(t0));
        self.set_rows(rows);
        true
    }

    /// Sort or Show changed: the rows made again from the folder's listing (nothing is read again), and sorted.
    fn rebuild(&mut self) {
        if self.at.is_none() || !self.built {
            return; // (the root has no listing; a listing still being read builds with the new settings)
        }
        self.next = None;
        self.built = false;
        self.build_rows();
    }

    fn in_import_place(&self) -> bool {
        self.at.as_ref().is_some_and(|p| self.import_path().is_some_and(|i| *p == i))
    }

    /// Under grimoire/import (the Import place or a folder in it), where rows carry statuses.
    fn under_import(&self) -> bool {
        match (self.at.as_ref(), self.import_path()) {
            (Some(p), Some(i)) => path_under(p, &i),
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

    /// The band under the header: the path line (100 px), in Files the path bar (120 px).
    fn band_h(&self) -> i32 {
        if self.mode == BrowseMode::Place {
            u(120.0)
        } else {
            u(100.0)
        }
    }

    fn per_page(&self, a_h: i32) -> usize {
        let top = u(190.0) + u(40.0) + self.band_h() + u(10.0);
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
            PlaceKind::Templates => tr("Templates"),
            PlaceKind::Photos => tr("Photos"),
            // TRANSLATORS: Files' row for grimoire/fonts, the fonts copied for Classic's text (44 px in a row of 150 px).
            PlaceKind::Fonts => tr("Fonts"),
            // TRANSLATORS: Files' row for the Kindle's own fonts folder (the fonts its reader uses), which Grimoire
            // only looks in (44 px in a row of 150 px).
            PlaceKind::KindleFonts => tr("Kindle fonts"),
            PlaceKind::PdfCopies => tr("PDF copies"),
            // TRANSLATORS: Files' row for grimoire/trash, where deleted notebooks and pages go (44 px in a row of 150 px).
            PlaceKind::Trash => tr("Trash"),
        }
    }

    /// A row's name as it is drawn.
    pub fn row_label(&self, r: &Row) -> String {
        match (r.kind, r.place) {
            (RowKind::Place, Some(k)) => Self::place_label(k).to_string(),
            // TRANSLATORS: the Import place's last row: it looks in grimoire/import for new files now (44 px in a row).
            (RowKind::Look, _) => tr("Look again").to_string(),
            // TRANSLATORS: the last row of the file picker's first screen (and of Files'): puts a Files card at the end
            // of the Library, which opens these places to browse (44 px in a row of about 1500 px).
            (RowKind::ShowCard, _) => tr("Show Files in Library").to_string(),
            (RowKind::Trashed, _) => r.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| r.name.clone()),
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
            RowKind::Look | RowKind::ShowCard => String::new(),
            RowKind::Pdf | RowKind::Dir if self.under_import() && r.status != RowStatus::None => self.status_text(&r.status),
            RowKind::Pdf | RowKind::Picture | RowKind::Font | RowKind::Other => self.row_sub_plain(r),
            // TRANSLATORS: Files with Show all: the second line of a row that is a link, which Grimoire never follows
            // (34 px, under the link's name).
            RowKind::Link => tr("link, not followed").to_string(),
            RowKind::Trashed if r.when > 0 => {
                let mut day = String::new();
                format_date_short(r.when, &mut day, 64);
                day
            }
            RowKind::Trashed => String::new(),
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

    /// The path's parts, each with the folder a tap on it opens: Files (its root: None), the place the folder is in,
    /// then the folders below it.
    fn crumb_list(&self) -> Vec<(String, Option<StoragePath>)> {
        let Some(at) = self.at.as_ref() else { return Vec::new() };
        let mut out = vec![(tr("Files").to_string(), None)];
        // the place: in Files the one the way in started at (Back goes up to it), else the one of this mode whose
        // folder holds this one, the nearest (All storage holds every one); the picker's path line always the nearest,
        // as before Files
        let places: Vec<(PlaceKind, StoragePath)> = self.mode_places().iter().filter_map(|&k| self.place_path(k).map(|p| (k, p))).collect();
        let started = self.place_root.as_ref().filter(|r| !self.pick() && path_under(at, r));
        let started = started.and_then(|r| places.iter().find(|(_, p)| p == r).cloned());
        let (k, base) = started
            .or_else(|| places.iter().filter(|(_, p)| path_under(at, p)).max_by_key(|(_, p)| p.as_bytes().len()).cloned())
            .unwrap_or((PlaceKind::All, StoragePath::root()));
        out.push((Self::place_label(k).to_string(), Some(base.clone())));
        let mut p = base;
        for c in at.components().skip(p.components().count()) {
            let Some(next) = p.join(c) else { break };
            out.push((String::from_utf8_lossy(c).into_owned(), Some(next.clone())));
            p = next;
        }
        out
    }

    /// The path line: Files, the place, then the folders below it.
    fn crumbs(&self) -> String {
        self.crumb_list().into_iter().map(|c| c.0).collect::<Vec<_>>().join(" > ")
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
        if self.next.is_some() {
            self.sort_step(FS_SLICE_MS);
            return changed;
        }
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
                if matches!(r.kind, RowKind::Place | RowKind::Look | RowKind::ShowCard) || r.stat.is_some() || r.stat_tried || r.ix == usize::MAX {
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
        // Files: a trash entry's name, a font's family (from its check's verdict), one a tick
        for i in vis.clone() {
            if self.rows[i].kind == RowKind::Trashed && self.rows[i].title.is_none() {
                let t = self.trash_title(&self.rows[i]);
                self.rows[i].title = Some(t);
                changed.push(i);
                return changed;
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
                let (key, size) = self.book_key(&p);
                let (nb, also) = self.notebook_of_key(&key, size);
                let r = &mut self.rows[i];
                r.notebook = nb;
                r.also_import = also;
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
        let Ok(mut l) = FsListing::start(st, p, self.pick()) else { return 0 };
        let t0 = real_now_ms();
        while !l.step(FS_SLICE_MS) {
            if real_now_ms().wrapping_sub(t0) > 200 {
                break;
            }
        }
        l.entries.iter().filter(|e| matches!(e.kind, FsKind::File | FsKind::Unknown) && is_picture(&e.name)).count().min(STORE_MAX_PAGES + 1)
    }

    /// The notebook that holds a PDF of this key and size ("has notes"), and whether it was found by a record of
    /// that size. The notebooks' keys and records are read once for the picker's session (one small read each),
    /// not once a row, and no file is read whole here: a notebook whose record has this size is the PDF's and the
    /// card also offers the import (which compares the whole file); one without a record goes by the key, as
    /// before; one whose record is of another size was made from another file and does not count.
    fn notebook_of_key(&mut self, key: &str, size: u64) -> (Option<String>, bool) {
        use crate::pdfimport::{key_holders, key_match, KeyHolder, KeyMatch};
        if key.is_empty() {
            return (None, false);
        }
        let all = self.book_keys.get_or_insert_with(|| key_holders(&self.books, None));
        let of_key: Vec<KeyHolder> = all.iter().filter(|h| h.key == key).cloned().collect();
        match key_match(&of_key, size, None) {
            KeyMatch::SizeOnly(id) => (Some(id), true),
            KeyMatch::KeyOnly(id) | KeyMatch::Same(id) => (Some(id), false),
            KeyMatch::None => (None, false),
        }
    }

    /// A PDF's book key (KOReader's partial MD5, 12 reads of 1 KiB) and its size, from one descriptor; "" when
    /// the file cannot be read.
    fn book_key(&self, p: &StoragePath) -> (String, u64) {
        let Some(st) = self.storage.as_ref() else { return (String::new(), 0) };
        let Ok(fd) = st.open_file(p, self.pick()) else { return (String::new(), 0) };
        use std::os::fd::AsRawFd;
        // SAFETY: md is written by fstat before it is read; the descriptor is this function's.
        let mut md: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd.as_raw_fd(), &mut md) } != 0 {
            return (String::new(), 0);
        }
        (crate::md5::partial_md5_fd(fd.as_raw_fd()).unwrap_or_default(), md.st_size as u64)
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

    /// A row's card, as a tap on a PDF or a hold on a folder opens it (the tests' too: it needs no screen).
    pub fn open_card(&mut self, row: usize) {
        if !self.has_card(row) {
            return;
        }
        let Some(r) = self.rows.get(row) else { return };
        // a PDF's book key is read now if the tick has not yet
        if r.kind == RowKind::Pdf && r.key.is_none() {
            let p = r.path.clone();
            let (key, size) = self.book_key(&p);
            let (nb, also) = self.notebook_of_key(&key, size);
            let r = &mut self.rows[row];
            r.notebook = nb;
            r.also_import = also;
            r.key = Some(key);
        }
        if self.rows[row].kind == RowKind::Dir && self.rows[row].pictures.is_none() && self.kinds.contains(&PickKind::PictureFolder) {
            let p = self.rows[row].path.clone();
            let n = self.count_pictures(&p);
            self.rows[row].pictures = Some(n);
        }
        // the row's stat, if the tick has not read it yet (its size and date are the details' first words)
        if self.rows[row].stat.is_none() && self.rows[row].ix != usize::MAX {
            if let Some(l) = self.listing.as_mut() {
                let ix = self.rows[row].ix;
                l.stat(ix..ix + 1, FS_SLICE_MS);
                self.rows[row].stat = l.entries.get(ix).and_then(|e| e.stat);
                self.rows[row].stat_tried = true;
            }
        }
        if self.mode == BrowseMode::Place && self.rows[row].detail.is_none() {
            let d = self.detail_of(row);
            self.rows[row].detail = Some(d);
        }
        // a PDF of the PDF copies: the notebook it was written from
        if self.rows[row].kind == RowKind::Pdf && self.rows[row].notebook.is_none() {
            if let Some(id) = self.mirror_notebook(row) {
                self.rows[row].notebook = Some(id);
            }
        }
        let r = &self.rows[row];
        self.card = Some(Card { row, path: r.path.clone(), stat: r.stat, ..Default::default() });
    }

    /// Files' Go to sheet (2.2): every place of the root that is there, then the storage's other top folders, by
    /// name (each a row that opens it), then Cancel.
    fn open_goto(&mut self) {
        let mut rows: Vec<(String, StoragePath, IconKind, bool)> = Vec::new();
        let pick = self.pick();
        let Some(st) = self.storage.as_ref() else { return };
        let mut taken: Vec<Vec<u8>> = Vec::new();
        for &k in self.mode_places() {
            let Some(p) = self.place_path(k) else { continue };
            if let Some(first) = p.components().next() {
                taken.push(first.to_ascii_lowercase());
            }
            let always = matches!(k, PlaceKind::Import | PlaceKind::All);
            if always || st.stat(&p, pick).is_ok_and(|s| s.mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32) {
                rows.push((Self::place_label(k).to_string(), p, Self::place_icon(k), true));
            }
        }
        // the storage's top folders that are no place, but a name a row would not take either (a line break)
        if let Some(l) = root_listing(st, pick, &mut real_now_ms) {
            let mut tops: Vec<(String, StoragePath)> = l
                .entries
                .iter()
                .filter(|e| e.kind == FsKind::Dir && !taken.contains(&e.name.to_ascii_lowercase()))
                .filter_map(|e| {
                    let n = std::str::from_utf8(&e.name).ok().filter(|n| crate::pdfimport::name_storable(n))?;
                    Some((n.to_string(), StoragePath::new(&e.name)?))
                })
                .collect();
            // (name_cmp is a total order: the unstable sort gives the same order, in less code)
            tops.sort_unstable_by(|a, b| name_cmp(&a.0, &b.0));
            rows.extend(tops.into_iter().map(|(n, p)| (n, p, IconKind::Folder, false)));
        }
        self.goto = rows;
        self.card = Some(Card { goto: true, ..Default::default() });
    }

    fn place_icon(k: PlaceKind) -> IconKind {
        match k {
            PlaceKind::Import | PlaceKind::Downloads => IconKind::Download,
            PlaceKind::Documents => IconKind::Folder,
            PlaceKind::Templates => IconKind::Paper,
            PlaceKind::Photos => IconKind::Image,
            PlaceKind::Fonts | PlaceKind::KindleFonts => IconKind::Font,
            PlaceKind::PdfCopies => IconKind::Pdf,
            PlaceKind::Trash => IconKind::Trash,
            PlaceKind::All => IconKind::Files,
        }
    }

    /// The card's lines: details (the Go to sheet's title), then its rows (hit id, its arg, icon, label).
    fn card_lines(&self, card: &Card) -> (String, Vec<(i32, i32, IconKind, String)>) {
        if card.goto {
            let mut rows: Vec<(i32, i32, IconKind, String)> = self.goto.iter().enumerate().map(|(i, g)| (B_GOTO, i as i32, g.2, g.0.clone())).collect();
            rows.push((B_CARD_CANCEL, 0, IconKind::Close, tr("Cancel").to_string()));
            // TRANSLATORS: the title of Files' Go to sheet, which a tap on the folder shown in its path bar opens: its
            // places and the Kindle's top folders, one a row (34 px, at the sheet's top).
            return (tr("Go to").to_string(), rows);
        }
        let Some(r) = self.card_row(card) else { return (String::new(), Vec::new()) };
        let mut rows: Vec<(i32, i32, IconKind, String)> = Vec::new();
        let details;
        // (Files: no import from Grimoire's PDF copies or trash, as the picker never offers them)
        let importable = self.pick() || self.storage.as_ref().is_some_and(|st| st.zone(&r.path, true) != FsZone::Refused);
        match r.kind {
            // a picture (#146): its copies into Grimoire's templates and photos (the picture pick's one, first)
            RowKind::Picture => {
                details = if self.pick() { self.row_sub_plain(r) } else { self.card_details(r) };
                for to in [CopyTo::Templates, CopyTo::Photos] {
                    if importable && self.offers_copy(r, to) {
                        let (id, icon, label) = if to == CopyTo::Templates {
                            // TRANSLATORS: a picture's card in Files (and in the picker the Templates sheet's From a
                            // file... opens): this copies it into grimoire/templates, to write on as paper (a row of
                            // about 800 px at 44 px).
                            (B_CARD_TEMPLATE, IconKind::Paper, tr("Use as template"))
                        } else {
                            // TRANSLATORS: a picture's card in Files (and in the picker the Photos sheet's From a
                            // file... opens): this copies it into grimoire/photos, to be a page's own picture (a row
                            // of about 800 px at 44 px).
                            (B_CARD_PHOTO, IconKind::Image, tr("Use as photo"))
                        };
                        rows.push((id, 0, icon, label.to_string()));
                    }
                }
            }
            // Files (2.2): what the file is, its size and date (and its family for a font of Grimoire's whose check
            // passed); Add to fonts waits for the Font sheet's Add a font... (#111)
            RowKind::Font | RowKind::Other => {
                details = self.card_details(r);
            }
            RowKind::Pdf => {
                details = if self.pick() { self.row_sub_plain(r) } else { self.card_details(r) };
                if r.notebook.is_some() {
                    // TRANSLATORS: a PDF's card in the file picker: a notebook already holds this book, and this opens it
                    // (a row of about 800 px at 44 px).
                    rows.push((B_CARD_NOTEBOOK, 0, IconKind::Notebook, tr("Open its notebook").to_string()));
                }
                // (with a notebook found by its record's size, both: the import says "PDF already imported." of the
                // same file, and makes a notebook of another)
                if importable && !self.is_mirror(r) && (r.notebook.is_none() || r.also_import) {
                    // TRANSLATORS: a PDF's card in the file picker: this makes a notebook of its pages (a row of about 800 px at 44 px).
                    rows.push((B_CARD_IMPORT, 0, IconKind::Download, tr("Import as notebook").to_string()));
                }
            }
            _ => {
                details = r.name.clone();
                rows.push((B_CARD_OPEN, 0, IconKind::Folder, tr("Open").to_string()));
                let n = r.pictures.unwrap_or(0);
                if n > 0 && importable {
                    // TRANSLATORS: a folder's card in the file picker: this makes one notebook of the %d pictures in it,
                    // in their order (a row of about 800 px at 44 px).
                    rows.push((B_CARD_PICTURES, 0, IconKind::Download, cformat!(ng_("Import %d picture as notebook", "Import %d pictures as notebook", n as i64), n as i32)));
                }
            }
        }
        rows.push((B_CARD_CANCEL, 0, IconKind::Close, tr("Cancel").to_string()));
        (details, rows)
    }

    /// Whether a picture's card offers its copy into `to` (#146): in the picture pick only the pick's own, and never
    /// into the folder the picture is in already (there it is a template, or a photo, as it is).
    fn offers_copy(&self, r: &Row, to: CopyTo) -> bool {
        let place = if to == CopyTo::Templates { PlaceKind::Templates } else { PlaceKind::Photos };
        let there = match (r.path.split_last(), self.place_path(place)) {
            (Some((dir, _)), Some(p)) => dir.as_bytes().eq_ignore_ascii_case(p.as_bytes()),
            _ => false,
        };
        self.to.is_none_or(|t| t == to) && !there
    }

    /// A Files card's details: what the file is, then its size and date, and what is known of it without reading
    /// it here ("PDF, 14.2 MB, Oct 30"; "Font, Inter, 120 KB, Sep 9").
    fn card_details(&self, r: &Row) -> String {
        let kind = match r.kind {
            RowKind::Pdf => "PDF",
            RowKind::Picture => tr("Picture"),
            RowKind::Font => tr("Font"),
            // TRANSLATORS: a Files card's first line for a file Grimoire cannot use, before its size and date ("File,
            // 2 KB, Oct 3"; 34 px).
            _ => tr("File"),
        };
        let mut parts = vec![kind.to_string()];
        if let Some(d) = r.detail.as_ref().filter(|d| !d.is_empty()) {
            parts.push(d.clone());
        }
        let plain = self.row_sub_plain(r);
        if !plain.is_empty() {
            parts.push(plain);
        }
        parts.join(", ")
    }

    /// What is known of a file without reading it in the UI (never a user's file parsed here: the worker or
    /// nothing): a font of grimoire/fonts whose check passed, its family; else "".
    fn detail_of(&self, row: usize) -> String {
        let r = &self.rows[row];
        let (Some(fonts), Some(s)) = (self.place_path(PlaceKind::Fonts), r.stat) else { return String::new() };
        if r.kind != RowKind::Font || !path_under(&r.path, &fonts) {
            return String::new();
        }
        let rel = String::from_utf8_lossy(&r.path.as_bytes()[fonts.as_bytes().len()..]).trim_start_matches('/').to_string();
        crate::userfonts::userfont_family(&rel, s.size, s.mtime).unwrap_or_default()
    }

    /// Whether a row is in Grimoire's PDF copies (a notebook's mirror PDF: opened as its notebook, never imported).
    fn is_mirror(&self, r: &Row) -> bool {
        self.place_path(PlaceKind::PdfCopies).is_some_and(|m| path_under(&r.path, &m))
    }

    /// The notebook a mirror PDF was written from: the one whose mirror path (export.rs) is this file's.
    fn mirror_notebook(&self, row: usize) -> Option<String> {
        let r = &self.rows[row];
        if !self.is_mirror(r) {
            return None;
        }
        let (root, _) = root_dir()?;
        let abs = format!("{}/{}", root.trim_end_matches('/'), r.path.to_string_lossy());
        let dir = crate::app::app_mirror_dir();
        self.books
            .iter()
            .find(|nb| {
                let mut p = String::new();
                crate::export::export_mirror_path(nb, &dir, &mut p, 1200);
                p == abs
            })
            .map(|nb| nb.id.as_str().to_string())
    }

    fn row_sub_plain(&self, r: &Row) -> String {
        let Some(s) = r.stat else { return String::new() };
        let mut day = String::new();
        format_date_short(s.mtime, &mut day, 64);
        format!("{}, {}", size_label(s.size), day)
    }

    /// Checks the file a card was opened for again before acting on it ("File changed. Try again."): its stat now
    /// against the one it had then (false once its row is not listed).
    fn unchanged(&self, c: &Card) -> bool {
        let (Some(st), Some(r)) = (self.storage.as_ref(), self.card_row(c)) else { return false };
        match (c.stat, st.stat(&c.path, self.pick())) {
            (Some(was), Ok(now)) if r.kind != RowKind::Dir => was.same(&now),
            (_, Ok(now)) => now.mode & libc::S_IFMT as u32 == libc::S_IFDIR as u32,
            _ => false,
        }
    }

    // ---- drawing ----

    /// The screen: header, path line, rows, pager, and the card when one is open.
    pub fn draw(&mut self, a: &mut App) {
        hits_clear(&mut self.hits);
        self.row_rects.clear();
        self.drawn.clear();
        let (aw, ah) = (a.w, a.h);
        let per = self.per_page(ah);
        self.per_last = per;
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
        let title = self.title;
        let tx = back.x + back.w + u(40.0);
        ui_text(c, FontFace::Title, ui_title_size() * 1.2, ui_small_size(), Rect::new(tx, 0, aw - tx - m, th), title, 2, false, 0);
        ui_rule(c, 0, aw, th, u(4.0), 0);
        ui_frieze(c, 0, aw, th + u(9.0), u(28.0), 0);
        // the band: Files and the free space at the root, else the path (in Files the path bar)
        let band = Rect::new(m, th + u(40.0), aw - 2 * m, self.band_h());
        self.path_bar = band;
        if self.at.is_some() && self.mode == BrowseMode::Place {
            self.draw_path_bar(c, band);
        } else if self.at.is_none() {
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
                let line = free_line(f, t);
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
            self.drawn.push(self.rows[i].name.clone());
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

    /// Files' path bar (2.2): the path's parts on the left, each a target the bar's height (a tap on one opens it;
    /// on the last, the folder shown, the Go to sheet), and the Sort and Show chips on the right. A path too long for
    /// its room loses the parts after Files first, to one "...", which opens the last of them.
    fn draw_path_bar(&mut self, c: &mut Canvas, band: Rect) {
        let fs = ui_text_size();
        let small = ui_small_size();
        // the chips, right to left: Show, then Sort
        let chip = |c: &mut Canvas, x_right: i32, icon: IconKind, label: &str| -> Rect {
            let iw = u(70.0);
            let w = (u(30.0) + iw + u(14.0) + font_measure(FontFace::Bold, small, label) + u(30.0)).clamp(u(220.0), band.w / 3);
            let r = Rect::new(x_right - w, band.y + u(10.0), w, band.h - u(20.0));
            ui_frame(c, r, u(16.0), 255, true, 0);
            ui_frame(c, r, u(16.0), 0, false, u(3.0));
            ui_icon(c, icon, Rect::new(r.x + u(24.0), r.y + (r.h - iw) / 2, iw, iw), 0);
            let tx = r.x + u(24.0) + iw + u(14.0);
            // (a word, not a name: the fit check hears of one cut short)
            font_draw_left(c, FontFace::Bold, small, Rect::new(tx, r.y, r.x + r.w - tx - u(20.0), r.h), label, 0);
            r
        };
        // (none in the trash: it keeps its own order, by the time in each name, and lists all it holds)
        let room_end = if self.in_trash_place() {
            band.x + band.w
        } else {
            let show = chip(c, band.x + band.w, IconKind::Eye, self.show_label());
            let sort = chip(c, show.x - u(20.0), IconKind::Sort, self.sort_label());
            hits_add(&mut self.hits, show, B_SHOW, 0);
            hits_add(&mut self.hits, sort, B_SORT, 0);
            sort.x - u(30.0)
        };
        // the path's parts in the room left
        let room = Rect::new(band.x, band.y, room_end - band.x, band.h);
        let parts = self.crumb_list();
        if parts.is_empty() || room.w <= 0 {
            return;
        }
        let sep = " > ";
        let sep_w = font_measure(FontFace::Regular, fs, sep);
        let cap = (room.w * 2 / 5).max(u(200.0));
        let widths: Vec<i32> = parts.iter().map(|p| font_measure(FontFace::Bold, fs, &p.0).min(cap)).collect();
        let more_w = font_measure(FontFace::Bold, fs, "...");
        // all of them, else Files, "..." (for the parts from 1 to from - 1: it opens the last of them) and the parts
        // from `from` on; the folder shown is always there
        let n = parts.len();
        let width_of = |from: usize| -> i32 {
            if from == 0 {
                widths.iter().sum::<i32>() + sep_w * (n as i32 - 1)
            } else {
                widths[0] + more_w + widths[from..].iter().sum::<i32>() + sep_w * (n - from + 1) as i32
            }
        };
        let mut from = 0;
        while width_of(from) > room.w {
            let next = if from == 0 { 2 } else { from + 1 };
            if next > n - 1 {
                break;
            }
            from = next;
        }
        let mut shown: Vec<(usize, i32, String)> = Vec::new();
        if from == 0 {
            shown.extend(parts.iter().enumerate().map(|(i, p)| (i, widths[i], p.0.clone())));
        } else {
            shown.push((0, widths[0], parts[0].0.clone()));
            shown.push((from - 1, more_w, "...".to_string()));
            shown.extend(parts.iter().enumerate().skip(from).map(|(i, p)| (i, widths[i], p.0.clone())));
        }
        let mut x = room.x;
        let last = shown.len() - 1;
        for (k, (i, w, label)) in shown.into_iter().enumerate() {
            let w = w.min(room.x + room.w - x).max(0);
            font_draw_left_name(c, FontFace::Bold, fs, Rect::new(x, band.y, w, band.h), &label, 0);
            // (its target reaches half way into the separators either side, the bar's height)
            let pad = if k == 0 { 0 } else { sep_w / 2 };
            let hit_w = w + pad + if k == last { 0 } else { sep_w / 2 };
            hits_add(&mut self.hits, Rect::new(x - pad, band.y, hit_w, band.h), B_CRUMB, i as i32);
            x += w;
            if k != last {
                font_draw_left(c, FontFace::Regular, fs, Rect::new(x, band.y, sep_w, band.h), sep, 0);
                x += sep_w;
            }
        }
        if self.capped {
            let line = cformat!(tr("Showing the first %d."), FS_LIST_MAX as i32);
            font_draw_left(c, FontFace::Regular, small * 0.8, Rect::new(room.x, band.y + band.h - u(36.0), room.w, u(36.0)), &line, 0);
        }
    }

    /// The Sort chip's word: the order in use (a tap takes the next: name, newest, size).
    fn sort_label(&self) -> &'static str {
        match self.prefs.sort {
            // TRANSLATORS: Files' Sort chip on its path bar while the newest files come first (one word of about
            // 220 px at 34 px in bold; a tap sorts by size, the largest first).
            1 => tr("Newest"),
            2 => tr("Size"),
            // TRANSLATORS: Files' Sort chip on its path bar while the rows go by name (one word of about 220 px at
            // 34 px in bold; a tap puts the newest first).
            _ => tr("Name"),
        }
    }

    fn show_label(&self) -> &'static str {
        if self.prefs.show == 1 {
            // TRANSLATORS: Files' Show chip on its path bar: every file is listed (others greyed, never opened), at
            // 34 px in bold, about 360 px. A tap shows only the files Grimoire can use.
            tr("All files")
        } else {
            // TRANSLATORS: Files' Show chip on its path bar: only the files Grimoire can use are listed, at 34 px in
            // bold, about 360 px. A tap lists every file.
            tr("PDFs, pictures, fonts")
        }
    }

    fn has_card(&self, i: usize) -> bool {
        self.rows.get(i).is_some_and(|r| matches!(r.kind, RowKind::Pdf | RowKind::Dir | RowKind::Picture | RowKind::Font | RowKind::Other))
    }

    fn draw_row(&self, c: &mut Canvas, r: Rect, i: usize) {
        let row = &self.rows[i];
        canvas_fill(c, Rect::new(r.x - u(4.0), r.y - u(4.0), r.w + u(8.0), r.h + u(8.0)), 255);
        ui_frame(c, r, u(12.0), 0, false, u(3.0));
        let icon = match (row.kind, row.place) {
            (RowKind::Place, Some(PlaceKind::Import)) => IconKind::Download,
            (RowKind::Place, Some(PlaceKind::Documents)) => IconKind::Folder,
            (RowKind::Place, Some(PlaceKind::Downloads)) => IconKind::Download,
            (RowKind::Place, Some(PlaceKind::Templates)) => IconKind::Paper,
            (RowKind::Place, Some(PlaceKind::Photos)) => IconKind::Image,
            (RowKind::Place, Some(PlaceKind::Fonts | PlaceKind::KindleFonts)) => IconKind::Font,
            (RowKind::Place, Some(PlaceKind::PdfCopies)) => IconKind::Pdf,
            (RowKind::Place, Some(PlaceKind::Trash)) => IconKind::Trash,
            (RowKind::Place, _) => IconKind::Files,
            (RowKind::Dir, _) => IconKind::Folder,
            (RowKind::Pdf, _) => IconKind::Pdf,
            (RowKind::Look, _) => IconKind::Update,
            (RowKind::ShowCard, _) => IconKind::Pin,
            (RowKind::Picture, _) => IconKind::Image,
            (RowKind::Font, _) => IconKind::Font,
            (RowKind::Other, _) => IconKind::FileOther,
            (RowKind::Link, _) => IconKind::LinkOff,
            (RowKind::Trashed, _) if row.trash.as_ref().is_some_and(|t| t.kind == TrashKind::File) => IconKind::FileOther,
            (RowKind::Trashed, _) if row.dir => IconKind::Notebook,
            (RowKind::Trashed, _) => IconKind::Paper,
        };
        // (a file Grimoire cannot use, and a link, greyed: Show all lists them so the folder is seen whole)
        let ink = if matches!(row.kind, RowKind::Other | RowKind::Link) { 110 } else { 0 };
        ui_icon(c, icon, Rect::new(r.x + u(20.0), r.y + u(20.0), u(90.0), r.h - u(40.0)), ink);
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
        if matches!(row.kind, RowKind::Place | RowKind::Look | RowKind::ShowCard) {
            font_draw_left(c, FontFace::Bold, ui_text_size(), name_r, &label, 0);
        } else {
            font_draw_left_name(c, FontFace::Bold, ui_text_size(), name_r, &label, ink);
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
                font_draw_left(c, FontFace::Regular, ui_small_size(), r2, &sub, ink);
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
        if !card.goto && self.card_row(&card).is_none() {
            return; // (its file is not listed now: no card)
        }
        let (details, mut lines) = self.card_lines(&card);
        let (aw, ah) = (a.w, a.h);
        let cw = (aw - 2 * u(40.0)).min(u(980.0));
        let rh = u(130.0);
        let gap = u(12.0);
        // (the Go to sheet's rows that do not fit the screen are left out, Cancel kept: All storage reaches them)
        let fit = ((ah - 2 * u(40.0) - 2 * u(24.0) - u(80.0) + gap) / (rh + gap)).max(2) as usize;
        if lines.len() > fit {
            let cancel = lines.pop();
            lines.truncate(fit - 1);
            lines.extend(cancel);
        }
        let ch = u(24.0) * 2 + u(80.0) + lines.len() as i32 * (rh + gap) - gap;
        let anchor = if card.goto {
            Rect::new(self.path_bar.x, self.path_bar.y, self.path_bar.w, self.path_bar.h)
        } else {
            self.row_rects.iter().find(|(k, _)| *k == card.row).map_or(Rect::new(aw / 4, ah / 3, aw / 2, u(138.0)), |x| x.1)
        };
        let rect = popout_place(cw, ch, anchor, Rect::new(0, 0, aw, ah), &[]);
        let c = &mut a.screen;
        ui_frame(c, rect, u(24.0), 255, true, 0);
        ui_frame(c, rect, u(24.0), 0, false, u(6.0));
        let inner = rect_inflate(rect, -u(24.0));
        let mut hits = HitList::default();
        font_draw_left_name(c, FontFace::Regular, ui_small_size(), Rect::new(inner.x + u(8.0), inner.y, inner.w - u(16.0), u(80.0)), &details, 0);
        let mut y = inner.y + u(80.0);
        for (id, arg, icon, label) in &lines {
            let r = Rect::new(inner.x, y, inner.w, rh);
            // (a picture's copy is the task's action in the picture pick; in Files neither of its two is first)
            let primary = matches!(*id, B_CARD_IMPORT | B_CARD_PICTURES | B_CARD_NOTEBOOK) || (self.pick() && matches!(*id, B_CARD_TEMPLATE | B_CARD_PHOTO));
            ui_frame(c, r, u(16.0), 255, true, 0);
            ui_frame(c, r, u(16.0), 0, false, if primary { u(6.0) } else { u(3.0) });
            ui_icon(c, *icon, Rect::new(r.x + u(20.0), r.y + u(15.0), rh - u(30.0), rh - u(30.0)), 0);
            ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), Rect::new(r.x + rh + u(10.0), r.y, r.w - rh - u(30.0), r.h), label, 2, false, 0);
            hits_add(&mut hits, r, *id, *arg);
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
            // the path bar: a part opens its folder (Files, the root), the folder shown the Go to sheet
            B_CRUMB => {
                let parts = self.crumb_list();
                let i = h.arg.max(0) as usize;
                if i + 1 >= parts.len() {
                    self.open_goto();
                    self.show_card(a);
                    return BrowserResult::None;
                }
                match parts[i].1.clone() {
                    None => {
                        self.show_root();
                        self.redraw_full(a);
                    }
                    Some(p) => {
                        if self.enter(p, i == 1, now) {
                            self.redraw_full(a);
                        } else {
                            self.flush_toast(a);
                        }
                    }
                }
            }
            B_SORT | B_SHOW => {
                if h.id == B_SORT {
                    self.prefs.sort = (self.prefs.sort.clamp(0, 2) + 1) % 3;
                } else {
                    self.prefs.show = if self.prefs.show == 1 { 0 } else { 1 };
                }
                self.card = None;
                self.loading_since = now;
                self.rebuild();
                // (the chip shows its new word now; rows sorted by newest or size come once their stats are in)
                self.redraw(a, RefreshMode::Ui);
                return BrowserResult::Prefs;
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
                    // (the row goes once the card is in the Library: the host saves the setting)
                    RowKind::ShowCard => {
                        self.prefs.card = true;
                        self.show_root();
                        self.redraw(a, RefreshMode::Ui);
                        return BrowserResult::ShowInLibrary;
                    }
                    // a picture's, a font's or another file's card (Files): what it is, and what can be done today
                    RowKind::Picture | RowKind::Font | RowKind::Other => {
                        self.open_card(i);
                        self.show_card(a);
                    }
                    RowKind::Link | RowKind::Trashed => {}
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
        let path = card.path.clone();
        match h.id {
            B_CARD_CANCEL => self.close_card(a),
            B_GOTO => {
                let Some((_, p, _, place)) = self.goto.get(h.arg.max(0) as usize).cloned() else { return self.close_card(a) };
                self.card = None;
                if self.enter(p, place, now_ms()) {
                    self.redraw_full(a);
                } else {
                    self.flush_toast(a);
                }
                BrowserResult::None
            }
            B_CARD_OPEN => {
                self.card = None;
                if self.enter(path, false, now_ms()) {
                    self.redraw_full(a);
                } else {
                    self.flush_toast(a);
                }
                BrowserResult::None
            }
            B_CARD_IMPORT | B_CARD_PICTURES | B_CARD_NOTEBOOK | B_CARD_TEMPLATE | B_CARD_PHOTO => {
                let Some(c) = self.card.take() else { return BrowserResult::None };
                if !self.unchanged(&c) {
                    return self.changed_since(a);
                }
                match h.id {
                    B_CARD_IMPORT => BrowserResult::Pdf(path),
                    B_CARD_PICTURES => BrowserResult::Pictures(path),
                    B_CARD_TEMPLATE => self.copy_in(a, &c, CopyTo::Templates),
                    B_CARD_PHOTO => self.copy_in(a, &c, CopyTo::Photos),
                    _ => self.card_row(&c).and_then(|r| r.notebook.clone()).map_or(BrowserResult::None, BrowserResult::OpenNotebook),
                }
            }
            _ => BrowserResult::None,
        }
    }

    /// The file a card acts on changed since its folder was listed: said, and the folder read again.
    fn changed_since(&mut self, a: &mut App) -> BrowserResult {
        // TRANSLATORS: a toast in the file picker: the file tapped was changed or replaced since its
        // folder was listed; the folder is read again. One line of about 1000 px at 44 px.
        self.toast = Some(tr("File changed. Try again."));
        let at = self.at.clone();
        if let Some(p) = at {
            self.enter(p, false, now_ms());
        }
        self.redraw_full(a);
        self.flush_toast(a);
        BrowserResult::None
    }

    /// Use as template, Use as photo (#146): the file card `c` was opened for, checked again, asked of the copy job
    /// into Grimoire's templates or photos (`copy_request`: the walk, the file as listed, its size, the free-space
    /// floor). The card is closed; a refusal is said now (`copy_refusal`: nothing was copied, and no job runs), the
    /// copy's end by the app's tick when the job ends.
    fn copy_in(&mut self, a: &mut App, c: &Card, to: CopyTo) -> BrowserResult {
        let asked = match (self.storage.as_ref(), c.stat) {
            (Some(st), Some(listed)) => copy_request(st, &c.path, &listed, to),
            _ => Err(FsWriteErr::Changed),
        };
        match asked {
            Ok(()) => {
                // (the picture pick ends here: its host closes it; Files stays, the card gone)
                if !self.pick() {
                    self.draw(a);
                    app_invalidate(a, c.rect, RefreshMode::Ui);
                }
                BrowserResult::Copying(to)
            }
            // (changed since it was listed: said, and the folder read again; every other refusal by its own sentence)
            Err(FsWriteErr::Changed) => self.changed_since(a),
            Err(e) => {
                let why = match e {
                    FsWriteErr::NoRoom => "no room: the storage would keep less than 100 MB".to_string(),
                    FsWriteErr::Refused => "a place it is not taken from".to_string(),
                    FsWriteErr::TooLong => "its path is too long for a job".to_string(),
                    FsWriteErr::TooBig => format!("it is over {} MB", to.max_bytes() >> 20),
                    FsWriteErr::Io(n) => strerror(n),
                    other => format!("{other:?}"),
                };
                qlog!("files: a copy into %s was refused: %s", to.folder(), &why);
                self.draw(a);
                app_invalidate(a, c.rect, RefreshMode::Ui);
                app_toast(a, &copy_refusal(CopyFail::of_request(e), to), 3000);
                BrowserResult::None
            }
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

    /// The `files` action while the browser is up (2.2): its root, the card closed.
    pub fn to_root(&mut self, a: &mut App) {
        self.card = None;
        self.show_root();
        self.redraw_full(a);
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
        let was_sorting = self.next.is_some();
        let changed = self.work(a.h, now);
        if self.toast.is_some() {
            self.flush_toast(a);
        }
        // (busy: a folder being read, or Files' rows being sorted by newest or size)
        let busy = !self.ready || self.next.is_some();
        // (the Loading notice goes once a folder is read, whichever: Back during a slow listing shows another)
        if !busy && self.loading_notice {
            self.loading_notice = false;
            app_notice_clear(a, NoticeKey::Loading);
        }
        if !was_ready && self.ready {
            self.redraw_full(a);
            return;
        }
        // (the rows on screen stayed while a new order's stats came in: they change at once now)
        if was_sorting && !busy {
            self.redraw(a, RefreshMode::Ui);
            return;
        }
        if busy {
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
            "mode" => out.push_str(if self.mode == BrowseMode::Place { "place" } else { "pick" }),
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
                Some(c) if c.goto => "goto".to_string(),
                Some(c) => match self.card_row(c).map(|r| r.kind) {
                    None => String::new(),
                    Some(RowKind::Pdf) => "pdf".to_string(),
                    Some(RowKind::Picture) => "picture".to_string(),
                    Some(RowKind::Font) => "font".to_string(),
                    Some(RowKind::Other) => "other".to_string(),
                    _ => "folder".to_string(),
                },
            }),
            "card.details" => {
                if let Some(c) = &self.card {
                    out.push_str(&self.card_lines(c).0);
                }
            }
            "card.rows" => {
                if let Some(c) = &self.card {
                    out.push_str(&self.card_lines(c).1.iter().map(|l| l.3.clone()).collect::<Vec<_>>().join("|"));
                }
            }
            "sort" => out.push_str(FILES_SORT_NAMES[self.prefs.sort.clamp(0, 2) as usize]),
            "show" => out.push_str(FILES_SHOW_NAMES[self.prefs.show.clamp(0, 1) as usize]),
            "sorting" => out.push_str(if self.next.is_some() { "1" } else { "0" }),
            "order" => out.push_str(&self.rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join("|")),
            // (the rows as last drawn on the screen, by name: Sort's rows change on screen once, when ready)
            "drawn" => out.push_str(&self.drawn.join("|")),
            _ => {
                let Some((what, n)) = key.split_once('.') else { return false };
                if !matches!(what, "row" | "sub" | "status" | "notes" | "keyed" | "kind" | "item" | "pictures") {
                    return false;
                }
                let r = if what == "item" { self.rows.iter().find(|r| r.name == n || r.name == n.replace('_', " ")) } else { nth(n) };
                let Some(r) = r else { return true };
                match what {
                    "row" | "item" => out.push_str(&self.row_label(r)),
                    "sub" => out.push_str(&self.row_sub(r)),
                    "status" => out.push_str(Self::status_key(&r.status)),
                    "notes" => out.push_str(if r.notebook.is_some() { "1" } else { "0" }),
                    "keyed" => out.push_str(if r.key.is_some() { "1" } else { "0" }),
                    "pictures" => out.push_str(&r.pictures.map_or(String::new(), |n| n.to_string())),
                    _ => out.push_str(match r.kind {
                        RowKind::Place => "place",
                        RowKind::Dir => "folder",
                        RowKind::Pdf => "pdf",
                        RowKind::Look => "look",
                        RowKind::ShowCard => "show",
                        RowKind::Picture => "picture",
                        RowKind::Font => "font",
                        RowKind::Other => "other",
                        RowKind::Link => "link",
                        RowKind::Trashed => "trashed",
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
    /// item.NAME, and the card's import, open, pictures, notebook, template, photo and cancel.
    pub fn point(&self, name: &str) -> Option<(f32, f32)> {
        let centre = |h: &Hit| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32);
        if let Some(c) = &self.card {
            let id = match name {
                "card.import" => B_CARD_IMPORT,
                "card.open" => B_CARD_OPEN,
                "card.pictures" => B_CARD_PICTURES,
                "card.notebook" => B_CARD_NOTEBOOK,
                "card.template" => B_CARD_TEMPLATE,
                "card.photo" => B_CARD_PHOTO,
                "card.cancel" => B_CARD_CANCEL,
                _ => 0,
            };
            if id != 0 {
                return c.hits.h.iter().find(|h| h.id == id).map(centre);
            }
            if let Some(n) = name.strip_prefix("goto.") {
                let i = n.parse::<i32>().ok()? - 1;
                return c.hits.h.iter().find(|h| h.id == B_GOTO && h.arg == i).map(centre);
            }
        }
        let find = |id: i32, arg: i32| self.hits.h.iter().find(|h| h.id == id && h.arg == arg).map(centre);
        match name {
            "back" => find(B_BACK, 0),
            "prev" => find(B_PREV, 0),
            "next" => find(B_NEXT, 0),
            "sort" => find(B_SORT, 0),
            "show" => find(B_SHOW, 0),
            _ => {
                let (what, n) = name.split_once('.')?;
                // the rows on screen, in order: their B_ROW hits
                let rows: Vec<&Hit> = self.hits.h.iter().filter(|h| h.id == B_ROW).collect();
                if what == "crumb" {
                    // (the Nth part of the path, from 1: Files, the place, then its folders)
                    return find(B_CRUMB, n.parse::<i32>().ok()? - 1);
                }
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
        assert!(b.open_pick(PickSpec::import()));
        assert!(!b.import_place_up());
        assert!(b.go(StoragePath::new(b"grimoire/import").unwrap()));
        assert!(b.import_place_up());
        assert!(b.go(StoragePath::new(b"grimoire/import/Sub").unwrap()));
        assert!(!b.import_place_up());
        assert!(b.go(StoragePath::new(b"documents").unwrap()));
        assert!(!b.import_place_up());
    }

    fn sp(s: &str) -> StoragePath {
        StoragePath::new(s.as_bytes()).unwrap()
    }

    fn row_names(b: &Browser) -> Vec<String> {
        b.rows.iter().map(|r| r.name.clone()).collect()
    }

    // Files' root (2.2): place mode lists Import, the Kindle's folders, Grimoire's places and the Kindle's fonts that
    // are there, and All storage, in that order; pick mode keeps its own four. Grimoire's PDFs and trash open in
    // place mode (read only), never in pick mode; the path line names the place.
    #[test]
    fn the_place_root_lists_grimoire_places_and_the_kindles_folders() {
        let (base, _g) = storage("placeroot");
        for d in ["documents/Books", "grimoire/templates", "grimoire/photos", "grimoire/fonts", "fonts", "grimoire/PDFs/School", "grimoire/trash"] {
            std::fs::create_dir_all(format!("{base}/{d}")).unwrap();
        }
        std::fs::write(format!("{base}/grimoire/PDFs/School/x.pdf"), b"%PDF-1.4\n").unwrap();
        i18n_set("en");
        let card = FilesPrefs { card: true, ..Default::default() };
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), card));
        assert_eq!(b.mode, BrowseMode::Place);
        assert_eq!(row_names(&b), ["Import", "documents", "Templates", "Photos", "Fonts", "Kindle fonts", "PDF copies", "Trash", "All storage"]);
        let labels: Vec<String> = b.rows.iter().map(|r| b.row_label(r)).collect();
        assert_eq!(labels[5], "Kindle fonts");
        assert_eq!(b.row_sub(&b.rows[5]), "fonts");
        assert_eq!(b.row_sub(&b.rows[4]), "grimoire/fonts");
        assert!(b.go(sp("grimoire/PDFs/School")));
        assert_eq!(row_names(&b), ["x.pdf"]);
        assert_eq!(b.crumbs(), "Files > PDF copies > School");
        assert!(b.go(sp("documents/Books")));
        assert_eq!(b.crumbs(), "Files > Documents > Books");
        assert!(b.go(sp("grimoire/trash")));
        let mut p = Browser::default();
        assert!(p.open(PickSpec::import(), card));
        assert_eq!(p.mode, BrowseMode::Pick);
        assert_eq!(row_names(&p), ["Import", "documents", "All storage"]);
        assert!(!p.go(sp("grimoire/PDFs/School")));
        assert!(!p.go(sp("grimoire/trash")));
    }

    // The way to the Files card is the root's last row, in either mode, while the card is not in the Library;
    // once it is, the row is not there.
    #[test]
    fn show_files_in_library_is_the_roots_last_row_until_the_card_is_shown() {
        let (_base, _g) = storage("showcard");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::import(), FilesPrefs::default()));
        assert_eq!(row_names(&b), ["Import", "All storage", "Show Files in Library"]);
        assert_eq!(b.rows.last().map(|r| r.kind), Some(RowKind::ShowCard));
        assert!(b.open(PickSpec::place(), FilesPrefs::default()));
        assert_eq!(b.rows.last().map(|r| r.kind), Some(RowKind::ShowCard));
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        assert!(b.rows.iter().all(|r| r.kind != RowKind::ShowCard));
        assert_eq!(b.rows.last().map(|r| r.name.as_str()), Some("All storage"));
    }

    // Files' order (2.2): folders first; by name (digit runs as numbers, accents folded), by the newest, by the
    // largest (folders by name); ties by name; a row with no stat (a link) after those with one.
    #[test]
    fn files_sort_by_name_newest_and_size() {
        let st = |mtime: i64, size: u64| Some(FsStat { mtime, size, ..Default::default() });
        let items = [
            (false, "b.pdf", st(900, 3000)),
            (true, "Sub", st(995, 0)),
            (false, "a 10.pdf", st(700, 1000)),
            (false, "a 2.pdf", st(700, 2000)),
            (false, "\u{c1}caro.png", st(950, 2000)),
            (false, "link.pdf", None),
            (true, "Archive", st(100, 0)),
        ];
        let order = |sort: i32| -> Vec<&str> {
            let mut v: Vec<usize> = (0..items.len()).collect();
            v.sort_by(|&x, &y| files_cmp(items[x], items[y], sort));
            v.into_iter().map(|i| items[i].1).collect()
        };
        assert_eq!(order(0), ["Archive", "Sub", "a 2.pdf", "a 10.pdf", "\u{c1}caro.png", "b.pdf", "link.pdf"]);
        assert_eq!(order(1), ["Sub", "Archive", "\u{c1}caro.png", "b.pdf", "a 2.pdf", "a 10.pdf", "link.pdf"]);
        assert_eq!(order(2), ["Archive", "Sub", "b.pdf", "a 2.pdf", "\u{c1}caro.png", "a 10.pdf", "link.pdf"]);
    }

    // Files (2.2) in a folder: pictures and fonts listed beside PDFs and folders; Sort by newest or size stats every
    // row first (the rows shown stay until then, and a folder entered so shows no row before); Show all adds the
    // files Grimoire cannot use and the links, "link, not followed". The path's parts open their folders; the Go to
    // sheet has the places and the storage's other top folders.
    #[test]
    fn files_lists_every_kind_sorts_shows_links_and_goes_to() {
        let (base, _g) = storage("sorted");
        let d = format!("{base}/documents/Sorted");
        std::fs::create_dir_all(format!("{d}/Sub")).unwrap();
        std::fs::create_dir_all(format!("{base}/audible")).unwrap();
        let put = |n: &str, size: usize, age: u64| {
            let p = format!("{d}/{n}");
            std::fs::write(&p, vec![b'x'; size]).unwrap();
            let f = std::fs::File::options().write(true).open(&p).unwrap();
            f.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000 - age)).unwrap();
        };
        put("a.pdf", 1000, 300);
        put("b.pdf", 3000, 100);
        put("c.png", 2000, 50);
        put("d.ttf", 500, 200);
        put("e.txt", 4000, 10);
        std::os::unix::fs::symlink("a.pdf", format!("{d}/f.pdf")).unwrap();
        i18n_set("en");
        let kinds = |b: &Browser| -> Vec<RowKind> { b.rows.iter().map(|r| r.kind).collect() };
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        assert!(b.go(sp("documents/Sorted")));
        assert_eq!(row_names(&b), ["Sub", "a.pdf", "b.pdf", "c.png", "d.ttf"]);
        assert_eq!(kinds(&b), [RowKind::Dir, RowKind::Pdf, RowKind::Pdf, RowKind::Picture, RowKind::Font]);
        b.prefs.sort = 1;
        b.rebuild();
        assert!(b.next.is_some() && b.ready, "the rows shown stay while the stats come in");
        assert_eq!(row_names(&b), ["Sub", "a.pdf", "b.pdf", "c.png", "d.ttf"]);
        b.list_now();
        assert_eq!(row_names(&b), ["Sub", "c.png", "b.pdf", "d.ttf", "a.pdf"]);
        b.prefs.sort = 2;
        b.rebuild();
        b.list_now();
        assert_eq!(row_names(&b), ["Sub", "b.pdf", "c.png", "a.pdf", "d.ttf"]);
        b.prefs.sort = 0;
        b.prefs.show = 1;
        b.rebuild();
        assert_eq!(row_names(&b), ["Sub", "a.pdf", "b.pdf", "c.png", "d.ttf", "e.txt", "f.pdf"]);
        assert_eq!(kinds(&b)[5..], [RowKind::Other, RowKind::Link]);
        assert_eq!(b.row_sub(&b.rows[6]), "link, not followed");
        // entered by newest: no row shows until every stat is in
        let mut e = Browser::default();
        assert!(e.open(PickSpec::place(), FilesPrefs { card: true, sort: 1, show: 0 }));
        assert!(e.enter(sp("documents/Sorted"), false, now_ms()));
        while e.next.is_none() && !e.ready {
            e.step_listing(FS_SLICE_MS);
        }
        assert!(!e.ready && e.rows.is_empty());
        e.list_now();
        assert_eq!(row_names(&e), ["Sub", "c.png", "b.pdf", "d.ttf", "a.pdf"]);
        // the path's parts and where each opens
        assert!(b.go(sp("documents/Sorted/Sub")));
        let parts: Vec<(String, Option<Vec<u8>>)> = b.crumb_list().into_iter().map(|(l, p)| (l, p.map(|p| p.as_bytes().to_vec()))).collect();
        assert_eq!(
            parts,
            [
                ("Files".to_string(), None),
                ("Documents".to_string(), Some(b"documents".to_vec())),
                ("Sorted".to_string(), Some(b"documents/Sorted".to_vec())),
                ("Sub".to_string(), Some(b"documents/Sorted/Sub".to_vec())),
            ]
        );
        b.open_goto();
        let labels: Vec<&str> = b.goto.iter().map(|g| g.0.as_str()).collect();
        assert_eq!(labels, ["Import", "Documents", "Trash", "All storage", "audible"]);
        assert!(b.goto[4].1 == sp("audible") && !b.goto[4].3 && b.goto[1].3);
    }

    // Files' item cards (2.2): a picture, a font and another file say what they are, with their size and date; a
    // picture offers Use as template and Use as photo (#146), a font and another file nothing yet but Cancel; a PDF
    // offers its import; a PDF of the PDF copies opens the notebook it was written from and is never imported; a link
    // has no card. The trash lists its entries read only, the newest first, a notebook by its own name and a page by
    // its notebook's.
    #[test]
    fn files_cards_say_what_a_file_is_and_the_trash_is_read_only() {
        let (base, _g) = storage("cards");
        let d = format!("{base}/documents/Sorted");
        std::fs::create_dir_all(&d).unwrap();
        for (n, size) in [("a.pdf", 1000), ("c.png", 2000), ("d.ttf", 500), ("e.txt", 4000)] {
            std::fs::write(format!("{d}/{n}"), vec![b'x'; size]).unwrap();
        }
        std::os::unix::fs::symlink("a.pdf", format!("{d}/f.pdf")).unwrap();
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Gamma", "School", PAPER_BLANK, &mut nb), 0);
        let mut mirror = String::new();
        crate::export::export_mirror_path(&nb, &crate::app::app_mirror_dir(), &mut mirror, 1200);
        std::fs::create_dir_all(std::path::Path::new(&mirror).parent().unwrap()).unwrap();
        std::fs::write(&mirror, b"%PDF-1.4\n").unwrap();
        for (entry, name) in [("20261101-000000-0001-1700000000", "Old"), ("20261101-000000-0002-1790000000", "Newer")] {
            std::fs::create_dir_all(format!("{base}/grimoire/trash/{entry}")).unwrap();
            std::fs::write(format!("{base}/grimoire/trash/{entry}/notebook.ini"), format!("name={name}\nfolder=\n")).unwrap();
        }
        std::fs::write(format!("{base}/grimoire/trash/{}-p3-1750000000", nb.id.as_str()), b"").unwrap();
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, sort: 0, show: 1 }));
        assert!(b.go(sp("documents/Sorted")));
        assert_eq!(row_names(&b), ["a.pdf", "c.png", "d.ttf", "e.txt", "f.pdf"]);
        let card = |b: &mut Browser, i: usize| -> (String, Vec<String>) {
            b.open_card(i);
            let c = b.card.clone().expect("a card");
            let (d, rows) = b.card_lines(&c);
            (d, rows.into_iter().map(|r| r.3).collect())
        };
        let (d0, r0) = card(&mut b, 0);
        assert!(d0.starts_with("PDF, 1 KB, "), "{d0}");
        assert_eq!(r0, ["Import as notebook", "Cancel"]);
        let (d1, r1) = card(&mut b, 1);
        assert!(d1.starts_with("Picture, 2 KB, "), "{d1}");
        assert_eq!(r1, ["Use as template", "Use as photo", "Cancel"]);
        let (d2, r2) = card(&mut b, 2);
        assert!(d2.starts_with("Font, 1 KB, "), "{d2}");
        assert_eq!(r2, ["Cancel"]);
        let (d3, _) = card(&mut b, 3);
        assert!(d3.starts_with("File, 4 KB, "), "{d3}");
        b.card = None;
        assert!(!b.has_card(4));
        b.open_card(4);
        assert!(b.card.is_none(), "a link has no card");
        assert!(b.go(sp("grimoire/PDFs/School")));
        assert_eq!(row_names(&b), ["Gamma.pdf"]);
        let (_, rm) = card(&mut b, 0);
        assert_eq!(rm, ["Open its notebook", "Cancel"]);
        assert_eq!(b.rows[0].notebook.as_deref(), Some(nb.id.as_str()));
        b.card = None;
        assert!(b.go(sp("grimoire/trash")));
        for _ in 0..8 {
            b.work(2480, now_ms());
        }
        let labels: Vec<String> = b.rows.iter().map(|r| b.row_label(r)).collect();
        assert_eq!(labels, ["Newer", "Gamma", "Old"]);
        assert!(b.rows.iter().all(|r| r.kind == RowKind::Trashed && !b.has_card(0)));
        assert!(b.rows[0].dir && !b.rows[1].dir);
    }

    // A screen to draw the browser on (1860 x 2480, upright), for the tests that draw.
    fn screen_app() -> App {
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        a
    }

    fn put_files(d: &str, files: &[(&str, usize)]) {
        std::fs::create_dir_all(d).unwrap();
        for (n, size) in files {
            std::fs::write(format!("{d}/{n}"), [b"%PDF-1.4\n".to_vec(), vec![b'x'; *size]].concat()).unwrap();
        }
    }

    // A card opened on a row while a new order is being made (Sort by size: the rows on screen stay, and can be
    // tapped, while the stats come in) stays on the file it was opened for when the new order takes their place: it
    // follows that file to its new row, says the same, and checks and acts on that file, against its stat when the
    // card opened. A press on the old rows is dropped.
    #[test]
    fn a_card_opened_while_sorting_follows_its_file() {
        let (base, _g) = storage("sortcard");
        let d = format!("{base}/documents/S");
        put_files(&d, &[("a.pdf", 1000), ("b.pdf", 3000), ("c.pdf", 2000)]);
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, sort: 0, show: 0 }));
        assert!(b.go(sp("documents/S")));
        assert_eq!(row_names(&b), ["a.pdf", "b.pdf", "c.pdf"]);
        b.prefs.sort = 2;
        b.rebuild();
        assert!(b.next.is_some(), "the new order is being made");
        b.open_card(0);
        let labels = |b: &Browser, c: &Card| -> (String, Vec<String>) {
            let (d, rows) = b.card_lines(c);
            (d, rows.into_iter().map(|r| r.3).collect())
        };
        let before = labels(&b, &b.card.clone().unwrap());
        assert!(before.0.starts_with("PDF, 1 KB, "), "{before:?}");
        b.pressed = Some(Hit { id: B_ROW, arg: 1, ..Default::default() });
        b.list_now();
        assert_eq!(row_names(&b), ["b.pdf", "c.pdf", "a.pdf"]);
        let c = b.card.clone().expect("the card stays open");
        assert_eq!(b.rows[c.row].name, "a.pdf", "the card follows its file");
        assert_eq!(labels(&b, &c), before);
        assert!(b.pressed.is_none(), "a press on the old rows is dropped");
        assert!(c.path == sp("documents/S/a.pdf") && b.unchanged(&c));
        // (the check is against the file the card was opened for: a.pdf changed, nothing acts)
        std::fs::write(format!("{d}/a.pdf"), b"%PDF-1.4\nlonger than it was").unwrap();
        assert!(!b.unchanged(&c));
    }

    // The same while Show narrows the rows (every file to the usable ones) under a card on a file that is not listed
    // then: the card closes, and nothing reads past the rows (drawing the card read them by its number, past the end).
    #[test]
    fn a_card_whose_file_is_no_longer_listed_closes() {
        let (base, _g) = storage("showcard2");
        put_files(&format!("{base}/documents/S"), &[("a.pdf", 1000), ("b.pdf", 3000), ("e.txt", 4000), ("f.txt", 10)]);
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, sort: 0, show: 1 }));
        assert!(b.go(sp("documents/S")));
        assert_eq!(row_names(&b), ["a.pdf", "b.pdf", "e.txt", "f.txt"]);
        b.prefs.sort = 2;
        b.rebuild();
        b.prefs.show = 0;
        b.rebuild();
        assert!(b.next.is_some());
        b.open_card(3);
        assert!(b.card.is_some());
        b.list_now();
        assert_eq!(row_names(&b), ["b.pdf", "a.pdf"]);
        let mut out = String::new();
        assert!(b.probe(2480, "card", &mut out) && out.is_empty(), "{out}");
        assert!(b.card.is_none(), "its file is not listed now");
        let mut a = screen_app();
        b.draw(&mut a);
        canvas_free(&mut a.screen);
    }

    // The Go to sheet reads the storage root at once on the tap, and all of it within GOTO_MS (by a clock that runs
    // a millisecond a call: names and kinds, a stat only where the kind is not known), and it leaves out a top folder
    // whose name holds a line break, as the rows do.
    #[test]
    fn go_to_reads_the_root_within_its_time_and_leaves_out_odd_names() {
        let (base, _g) = storage("gotoroot");
        for i in 0..400 {
            std::fs::create_dir_all(format!("{base}/F{i:03}")).unwrap();
        }
        std::fs::create_dir_all(format!("{base}/Top\nX")).unwrap();
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        let mut t = 0u32;
        let mut clock = || {
            t += 1;
            t
        };
        let l = root_listing(b.storage.as_ref().unwrap(), false, &mut clock).unwrap();
        assert!(!l.entries.is_empty());
        assert!(t <= GOTO_MS + 5, "the tap waited {t} ms");
        b.open_goto();
        let labels: Vec<&str> = b.goto.iter().map(|g| g.0.as_str()).collect();
        assert!(labels.contains(&"F000") && labels.contains(&"F399"), "{labels:?}");
        assert!(!labels.iter().any(|l| l.contains('\n')), "a name with a line break is no row");
    }

    // The Trash place reads a notebook's name through the walk: an entry that was a folder when listed and is a link
    // to another folder when its name is read gives no name (the entry's own shows), and a name is cut as the store
    // cuts one. A file's entry with no record beside it shows its own name, with the time in it.
    #[test]
    fn trash_names_are_read_through_the_walk() {
        let (base, _g) = storage("trashwalk");
        let t = format!("{base}/grimoire/trash");
        let other = format!("{base}/elsewhere");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(format!("{other}/notebook.ini"), "name=FROM-ELSEWHERE\n").unwrap();
        let swapped = "20261101-000000-0007-1790000700";
        let long = "20261101-000000-0006-1790000600";
        for (entry, name) in [(swapped, "Real".to_string()), (long, "N".repeat(300))] {
            std::fs::create_dir_all(format!("{t}/{entry}")).unwrap();
            std::fs::write(format!("{t}/{entry}/notebook.ini"), format!("name={name}\n")).unwrap();
        }
        std::fs::create_dir_all(format!("{t}/files-20261115-120000-1234")).unwrap();
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        assert!(b.go(sp("grimoire/trash")));
        assert_eq!(row_names(&b), ["files-20261115-120000-1234", swapped, long]);
        // (after the listing, the folder is swapped for a link to another)
        std::fs::remove_dir_all(format!("{t}/{swapped}")).unwrap();
        std::os::unix::fs::symlink(&other, format!("{t}/{swapped}")).unwrap();
        for _ in 0..8 {
            b.work(2480, now_ms());
        }
        let labels: Vec<String> = b.rows.iter().map(|r| b.row_label(r)).collect();
        assert_eq!(labels[0], "files-20261115-120000-1234", "no record: the entry's own name");
        assert_eq!(labels[1], swapped, "nothing read through the link");
        assert_eq!(labels[2], "N".repeat(STORE_NAME_LEN - 1));
        assert!(b.rows[0].when > 0 && !b.row_sub(&b.rows[0]).is_empty());
    }

    // The Trash place reads the trash through fsview's trash listing (`fsview_trash_entries`), as the storage writes
    // define it: a trash holding both forms side by side, a file trashed now by fsview's own trash and one trashed
    // earlier (`files-<stamp>-<4 hex>`, each beside its record), a `files-` entry with no record, a notebook
    // (`<id>-<time>`), a page (`<id>-p<page>-<time>`) and a name of another form. The rows are the listing's entries in
    // its order (the newest first by the time in each name, the other form last), with its times; a file shows the
    // name its record holds, a notebook its own, a page its notebook's; the entry with no record and the other form
    // show their own names, and only the other form has no date.
    #[test]
    fn the_trash_place_lists_both_forms_through_fsviews_listing() {
        let (base, _g) = storage("trashforms");
        let t = format!("{base}/grimoire/trash");
        std::fs::create_dir_all(format!("{base}/grimoire/templates")).unwrap();
        std::fs::write(format!("{base}/grimoire/templates/Grid.png"), b"a template").unwrap();
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Gamma", "", PAPER_BLANK, &mut nb), 0);
        // (a file trashed now, by the storage writes' own trash)
        let st = FsStorage::open(&base, &format!("{base}/grimoire")).unwrap();
        let grid = sp("grimoire/templates/Grid.png");
        let listed = st.stat(&grid, false).unwrap();
        let made = String::from_utf8(st.trash(&grid, &listed).unwrap()).unwrap();
        assert!(made.starts_with("files-") && !std::path::Path::new(&format!("{base}/grimoire/templates/Grid.png")).exists());
        // (the older entries: a notebook of 2026-10-26, a file's of 2026-10-20 with its record, one of 2026-10-10
        // with none, a page of 2026-09-21, and a name of another form)
        let kept = "20261001-000000-0001-1793000000";
        std::fs::create_dir_all(format!("{t}/{kept}")).unwrap();
        std::fs::write(format!("{t}/{kept}/notebook.ini"), "name=Kept\n").unwrap();
        let beach = "files-20261020-090000-00ff";
        std::fs::create_dir_all(format!("{t}/{beach}")).unwrap();
        std::fs::write(format!("{t}/{beach}/beach.jpg"), b"a photo").unwrap();
        std::fs::write(format!("{t}/{beach}/{TRASH_RECORD}"), "# a record\nfrom=grimoire/photos/beach.jpg\ntime=1792483200\n").unwrap();
        let bare = "files-20261010-080000-0abc";
        std::fs::create_dir_all(format!("{t}/{bare}")).unwrap();
        let page = format!("{}-p3-1790000500", nb.id.as_str());
        std::fs::write(format!("{t}/{page}"), b"").unwrap();
        std::fs::write(format!("{t}/notes"), b"").unwrap();
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        assert!(b.go(sp("grimoire/trash")));
        let mut l = FsListing::start(&st, &st.trash_dir().unwrap(), false).unwrap();
        while !l.step(FS_SLICE_MS) {}
        let listing: Vec<(String, i64)> =
            fsview_trash_entries(&l.entries).into_iter().map(|e| (String::from_utf8(e.name).unwrap(), e.time)).collect();
        let rows: Vec<(String, i64)> = b.rows.iter().map(|r| (r.name.clone(), r.when)).collect();
        assert_eq!(rows, listing, "the rows are fsview's listing, in its order, with its times");
        assert_eq!(row_names(&b), [made.as_str(), kept, beach, bare, page.as_str(), "notes"]);
        for _ in 0..16 {
            b.work(2480, now_ms());
        }
        let labels: Vec<String> = b.rows.iter().map(|r| b.row_label(r)).collect();
        assert_eq!(labels, ["Grid.png", "Kept", "beach.jpg", bare, "Gamma", "notes"]);
        assert!(b.rows[..5].iter().all(|r| r.when > 0 && !b.row_sub(r).is_empty()));
        assert!(b.rows[5].when == 0 && b.row_sub(&b.rows[5]).is_empty());
        assert!(b.rows.iter().enumerate().all(|(i, r)| r.kind == RowKind::Trashed && !b.has_card(i)));
    }

    // Show all in a folder of 6,000 links (the listing keeps 5,000 of their names): no more than 5,000 rows, and
    // "Showing the first 5000." is said, as for 5,000 entries. With the usable files shown, the links are not rows
    // and nothing is said.
    #[test]
    fn show_all_past_5000_links_says_so_and_keeps_the_cap() {
        let (base, _g) = storage("links6000");
        let d = format!("{base}/documents/Links");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(format!("{d}/a.pdf"), b"%PDF-1.4\n").unwrap();
        for i in 0..6000 {
            std::os::unix::fs::symlink("a.pdf", format!("{d}/l{i}")).unwrap();
        }
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, sort: 0, show: 1 }));
        assert!(b.go(sp("documents/Links")));
        assert_eq!(b.rows.len(), FS_LIST_MAX);
        assert!(b.capped, "Showing the first 5000.");
        b.prefs.show = 0;
        b.rebuild();
        assert!(b.rows.len() == 1 && !b.capped);
    }

    // The picker's path line names a folder by the place that holds it however it was reached (All storage, then
    // documents/Books: "Files > Documents > Books", as before Files); Files' path bar names the place the way in
    // started at, as its Back goes up to it.
    #[test]
    fn the_pickers_path_line_names_the_place_that_holds_the_folder() {
        let (base, _g) = storage("pickcrumbs");
        std::fs::create_dir_all(format!("{base}/documents/Books")).unwrap();
        i18n_set("en");
        let cases = [(PickSpec::import(), "Files > Documents > Books"), (PickSpec::place(), "Files > All storage > documents > Books")];
        for (spec, want) in cases {
            let mut b = Browser::default();
            assert!(b.open(spec, FilesPrefs { card: true, ..Default::default() }));
            assert!(b.enter(StoragePath::root(), true, now_ms()));
            b.list_now();
            assert!(b.go(sp("documents/Books")));
            assert_eq!(b.crumbs(), want);
        }
    }

    // The Trash place has no Sort or Show chip on its path bar (its order is the time in each name, and it lists all
    // it holds); a folder elsewhere in Files has both.
    #[test]
    fn the_trash_place_has_no_sort_or_show_chip() {
        let (base, _g) = storage("trashchips");
        std::fs::create_dir_all(format!("{base}/grimoire/trash/20261101-000000-0001-1790000000")).unwrap();
        std::fs::create_dir_all(format!("{base}/documents")).unwrap();
        i18n_set("en");
        let mut a = screen_app();
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        assert!(b.go(sp("documents")));
        b.draw(&mut a);
        assert!(b.point("sort").is_some() && b.point("show").is_some());
        assert!(b.go(sp("grimoire/trash")));
        b.draw(&mut a);
        assert!(b.point("sort").is_none() && b.point("show").is_none());
        assert!(b.point("crumb.2").is_some(), "the path's parts are there");
        canvas_free(&mut a.screen);
    }

    // A picture's card copies it into Grimoire's templates or photos (2.2, #146). In Files: Use as template and Use as
    // photo, but never the copy into the folder the picture is already in, and neither in a place a pick never takes
    // from (Grimoire's PDF copies). In the picture pick a Templates or Photos sheet's From a file... opens: that sheet's
    // copy alone, then Cancel.
    #[test]
    fn a_pictures_card_offers_the_copies_into_templates_and_photos() {
        let (base, _g) = storage("useas");
        for d in ["documents/Pics", "grimoire/templates", "grimoire/photos", "grimoire/PDFs"] {
            std::fs::create_dir_all(format!("{base}/{d}")).unwrap();
        }
        for f in ["documents/Pics/a.png", "grimoire/templates/t.png", "grimoire/photos/p.jpg", "grimoire/PDFs/odd.png"] {
            std::fs::write(format!("{base}/{f}"), b"a picture").unwrap();
        }
        i18n_set("en");
        let rows = |b: &mut Browser, i: usize| -> Vec<String> {
            b.open_card(i);
            let c = b.card.clone().expect("a card");
            b.card_lines(&c).1.into_iter().map(|r| r.3).collect()
        };
        let mut b = Browser::default();
        assert!(b.open(PickSpec::place(), FilesPrefs { card: true, ..Default::default() }));
        assert!(b.go(sp("documents/Pics")));
        assert_eq!(rows(&mut b, 0), ["Use as template", "Use as photo", "Cancel"]);
        assert!(b.go(sp("grimoire/templates")));
        assert_eq!(rows(&mut b, 0), ["Use as photo", "Cancel"]);
        assert!(b.go(sp("grimoire/photos")));
        assert_eq!(rows(&mut b, 0), ["Use as template", "Cancel"]);
        assert!(b.go(sp("grimoire/PDFs")));
        assert_eq!(rows(&mut b, 0), ["Cancel"]);
        for (to, want) in [(CopyTo::Templates, "Use as template"), (CopyTo::Photos, "Use as photo")] {
            let mut p = Browser::default();
            assert!(p.open(PickSpec::picture(to), FilesPrefs { card: true, ..Default::default() }));
            assert!(p.go(sp("documents/Pics")));
            assert_eq!(rows(&mut p, 0), [want, "Cancel"]);
        }
    }

    // The picture pick (2.2, #146): its root has no Import (that place is the imports', with their statuses and Look
    // again), and a folder lists its folders and the pictures the template job takes (.png, .jpg, .jpeg, .pgm, in any
    // case), nothing else.
    #[test]
    fn the_picture_pick_lists_folders_and_pictures() {
        let (base, _g) = storage("picpick");
        let d = format!("{base}/documents/Pics");
        std::fs::create_dir_all(format!("{d}/Sub")).unwrap();
        for n in ["a.png", "B.JPG", "c.jpeg", "d.pgm", "e.pdf", "f.ttf", "g.txt"] {
            std::fs::write(format!("{d}/{n}"), b"x").unwrap();
        }
        i18n_set("en");
        let mut b = Browser::default();
        assert!(b.open(PickSpec::picture(CopyTo::Photos), FilesPrefs { card: true, ..Default::default() }));
        assert_eq!(b.mode, BrowseMode::Pick);
        assert_eq!(row_names(&b), ["documents", "All storage"]);
        assert!(b.go(sp("documents/Pics")));
        assert_eq!(row_names(&b), ["Sub", "a.png", "B.JPG", "c.jpeg", "d.pgm"]);
        assert!(b.rows[1..].iter().all(|r| r.kind == RowKind::Picture));
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
