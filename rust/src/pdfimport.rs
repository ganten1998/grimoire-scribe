//! Importing PDFs without KOReader (2.1.0-beta.2; DESIGN.md 3.8, 3.9, 3.11; areas/pdf.md 3.3, 3.4 and 4).
//!
//! A PDF dropped in `grimoire/import/` (or picked anywhere, read in place) becomes a notebook of its pages,
//! each a document page on the PDF's page fitted and centred on white, as KOReader would draw it. The job
//! (`BgKind::PdfImport`) runs in slices of at most 25 pages or 20 s, each a bgjob child that is the writer
//! of sandbox.rs's picture: it opens the file through fsview, owns the work folder
//! `notebooks/.import-<id>/`, starts a worker (a fresh sandboxed exec) for the pages and validates all it
//! sends. Slice 0 probes (the page count and the caps), the next slices render, the last commits.
//!
//! - State lives in `notebooks/.import-<id>/pdf-import.ini` (the source, its size and mtime, the key, the next
//!   page, the pictures made, the blank pages and the strikes, a pick's name and folder), so a power cut
//!   resumes and a rescan resumes. A resume first checks the files the state names, and makes again any page
//!   whose picture or page file is gone.
//! - Only a file that lies in `grimoire/import/` ever moves (to `import/done/`), decided by where it is: a
//!   file picked anywhere else is read in place and never moved, whatever its mark says.
//! - What counts: a try per import of a file of import/ (`import/.attempts`, keyed by its path there; three,
//!   or a refusal, and it waits until it changes). No room on the storage, a sandbox step that fails and a
//!   worker that cannot start count nothing. An import that stops without finishing (a writer cut off, the
//!   CPU budget) keeps its pages and goes on later, three times at most.
//! - One owner per import: `import/.lock` (flock), taken by the UI without blocking before it queues any
//!   import and held while any is queued or running, through one handle; the bgjob child and the detached
//!   child keep the same descriptor (`lock_fd`), and no job ever opens or waits for it.
//! - Per-page isolation (the owner's change to D37): a page whose worker died or stalled twice becomes a
//!   blank document page and the import goes on; the end says so once. A PDF that fails before any page
//!   (no page tree, a password, over the caps, the sandbox refusing) fails whole.
//! - Colour (the owner's change to D18): a page with colour is stored as an 8-bit RGB PNG, a page that is
//!   gray within `pdfwork::GRAY_TOLERANCE` as an 8-bit gray one. The Scribe shows both in gray (bgimage
//!   decodes either to its luma at load and keeps only that), so the UI never holds the 13.8 MB RGB copy.
//! - No file name, title or key reaches grimoire.log: ids, page numbers, counts, timings and reasons.

use crate::fsview::*;
use crate::md5::*;
use crate::pdfwork::*;
use crate::prelude::*;
use crate::sandbox::*;
use std::cell::{Cell, RefCell};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

// ---- numbers ----

/// How often a scan may run (the Library's return, the start): at most every 10 s.
pub const SCAN_EVERY_MS: u32 = 10_000;
/// A file is taken when its size and mtime held across two looks at least this far apart ...
pub const STABLE_GAP_MS: u32 = 2_000;
/// ... and its mtime is at least this old.
pub const STABLE_AGE_S: i64 = 5;
/// While a file waits to be stable the Library looks again after this.
pub const WAIT_RESCAN_MS: u32 = 5_000;
/// Imports queued at once.
pub const QUEUE_MAX: usize = 8;
/// Tries before a file waits until it changes.
pub const PDF_TRIES: i32 = 3;
/// A PDF's pages (STORE_MAX_PAGES).
pub const PDF_MAX_PAGES: u32 = STORE_MAX_PAGES as u32;
/// An import that stopped this many times without finishing, keeping its work folder (a writer cut off, the job
/// over its CPU budget), fails whole at its next slice.
pub const STOPS_MAX: u32 = 3;
/// Workers a later slice starts to open the PDF before it gives up on the file (the probe's failure is the
/// slice's, never a page's).
const PROBE_TRIES: u32 = 3;
/// A later slice checks the storage floor again every this many pages.
const FLOOR_EVERY: usize = 5;
/// The first estimate of a page's bytes, before any page is made (later ones come from the pages written).
const PAGE_GUESS: u64 = 60 * 1024;
/// A page file besides its picture (a document page: its paper, no ink).
const PAGE_FILE_BYTES: u64 = 1024;
/// A file without its end marker that has not changed for this long is taken anyway (the renderer repairs
/// what it can, and a file it cannot read fails as any other does).
pub const INCOMPLETE_GIVE_UP_MS: u32 = 60_000;
/// The files the scan remembers seeing (one entry each); a file past this waits for a later scan.
pub const SEEN_MAX: usize = 4096;
/// After a PDF found no room, the scan takes it again once this much more is free.
const ROOM_MARGIN: u64 = 16 << 20;
/// The mark of a file waiting for the lock or being made lives in import/.marks/<hash>.
const MARKS_DIR: &str = "import/.marks";
const STATE_FILE: &str = "pdf-import.ini";
const STATE_MAX: usize = 64 << 10;
const MARK_MAX: usize = 4 << 10;
const WORK_PREFIX: &str = ".import-";
const FNV0: u64 = 14695981039346656037;

fn fnv(mut h: u64, d: &[u8]) -> u64 {
    for &b in d {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    h
}

/// The slice length: the table's, or (headless and test builds) GRIMOIRE_PDF_SLICE_PAGES.
fn slice_pages() -> usize {
    #[cfg(not(feature = "kindle"))]
    if let Some(n) = std::env::var("GRIMOIRE_PDF_SLICE_PAGES").ok().and_then(|v| v.parse::<usize>().ok()).filter(|&n| n > 0) {
        return n;
    }
    s1::SLICE_PAGES
}

// ---- the shared progress record ----

/// One page of shared memory (MAP_SHARED | MAP_ANON, as httpd.rs does): made by the UI before the fork,
/// written by the writer, validated by the UI anyway. The worker never sees it.
#[repr(C)]
struct Progress {
    page: AtomicU32,
    pages: AtomicU32,
    /// the job the result below is for (a hash of its id)
    result_for: AtomicU32,
    reason: AtomicU32,
    arg: AtomicU32,
    blank: AtomicU32,
    sandbox_logged: AtomicU32,
    /// a worker reported a failed sandbox step, or could not be started (the log says which)
    broken: AtomicU32,
    /// the job `page` and `pages` are for (a hash of its id), stored at the start of every slice
    page_for: AtomicU32,
}

// the reasons a job ends with
const R_NONE: u32 = 0;
const R_FAILED: u32 = 1;
const R_PASSWORD: u32 = 2;
const R_TOO_LONG: u32 = 3;
const R_TOO_BIG: u32 = 4;
const R_ALREADY: u32 = 5;
const R_STORAGE: u32 = 6;
const R_DONE: u32 = 7;
/// a copy of this PDF is being imported: this one waits for that notebook (and then says "already")
const R_DEFERRED: u32 = 8;

thread_local! {
    static PROGRESS: Cell<usize> = const { Cell::new(0) };
}

/// The record; made on first use (the UI asks before it forks anything).
fn progress() -> &'static Progress {
    let at = PROGRESS.with(|p| {
        if p.get() == 0 {
            // SAFETY: an anonymous shared mapping of one page, zero-filled by the kernel, that lives as long as
            // the process (and its children, which inherit it).
            let m = unsafe {
                libc::mmap(std::ptr::null_mut(), 4096, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED | libc::MAP_ANON, -1, 0)
            };
            if m != libc::MAP_FAILED {
                p.set(m as usize);
            } else {
                // no shared page: a private one (the UI then sees no progress, which is harmless)
                let b: &'static mut [u64; 512] = Box::leak(Box::new([0u64; 512]));
                p.set(b.as_mut_ptr() as usize);
            }
        }
        p.get()
    });
    // SAFETY: at is a page-aligned, zeroed, live mapping at least 32 bytes long; Progress is atomics only.
    unsafe { &*(at as *const Progress) }
}

fn id_hash(id: &str) -> u32 {
    fnv(FNV0, id.as_bytes()) as u32
}

fn set_result(id: &str, reason: u32, arg: u32, blank: u32) {
    let p = progress();
    p.reason.store(reason, Ordering::Relaxed);
    p.arg.store(arg, Ordering::Relaxed);
    p.blank.store(blank, Ordering::Relaxed);
    p.result_for.store(id_hash(id), Ordering::Release);
}

/// The page and count the notice shows, and the job they are for: stored when a slice starts (and as pages are
/// made), so an import that goes on after a restart, or two taking turns, each show their own.
fn set_progress(id: &str, page: u32, pages: u32) {
    let p = progress();
    p.page.store(page, Ordering::Relaxed);
    p.pages.store(pages, Ordering::Relaxed);
    p.page_for.store(id_hash(id), Ordering::Release);
}

// ---- paths ----

fn root_path(sub: &str) -> String {
    format!("{}/{}", store_root(), sub)
}

/// The storage root a StoragePath is relative to: the Kindle's /mnt/us, else GRIMOIRE_STORAGE (the
/// simulator's and tests' hook), else (headless builds only) the folder around Grimoire's own.
pub(crate) fn storage_root() -> Option<String> {
    if let Some(r) = fsview_root() {
        return Some(r);
    }
    #[cfg(not(feature = "kindle"))]
    {
        let r = store_root();
        let p = Path::new(&r).parent()?.to_str()?.to_string();
        return (!p.is_empty() && p != "/").then_some(p);
    }
    #[allow(unreachable_code)]
    None
}

/// Grimoire's folder relative to the storage root ("grimoire" on the Kindle).
pub(crate) fn grimoire_rel() -> Option<String> {
    let root = storage_root()?;
    let g = store_root();
    g.strip_prefix(root.trim_end_matches('/')).map(|r| r.trim_matches('/').to_string()).filter(|r| !r.is_empty())
}

/// The StoragePath of `rel` inside Grimoire's import folder.
pub(crate) fn import_storage_path(rel: &str) -> Option<StoragePath> {
    StoragePath::new(format!("{}/import/{}", grimoire_rel()?, rel).as_bytes())
}

/// The path of a source under Grimoire's import folder (`Book.pdf`, `Work/Book.pdf`), None for a source
/// anywhere else and for `import/done/` (compared without case, as the storage is).
pub(crate) fn under_import(src: &str) -> Option<&str> {
    let g = grimoire_rel()?;
    let pre = format!("{}/import/", g.trim_matches('/'));
    let b = src.as_bytes();
    if b.len() <= pre.len() || !b[..pre.len()].eq_ignore_ascii_case(pre.as_bytes()) {
        return None;
    }
    let rest = src.get(pre.len()..)?;
    let first = rest.split('/').next().unwrap_or("");
    (!first.eq_ignore_ascii_case("done") && !rest.is_empty()).then_some(rest)
}

/// Whether a file may be moved (to import/done) or written beside: only when it lies in Grimoire's import folder.
/// Decided by where it is, never by a flag (a lost or torn mark cannot make a picked file a drop: D26, 4.2.5).
pub fn in_import(src: &str) -> bool {
    under_import(src).is_some()
}

/// The key of a file's record in import/.attempts: a hash of its path under import/ (a folder's is its name,
/// as before), so `import/A.pdf` and `import/Work/A.pdf` have one each. None for a file picked elsewhere: a
/// pick is the user's own asking, never taken again by a scan, and keeps no record.
pub(crate) fn attempt_key(src: &str) -> Option<u64> {
    under_import(src).map(|r| fnv(FNV0, r.as_bytes()))
}

fn abs_of(sp: &StoragePath) -> Option<String> {
    Some(format!("{}/{}", storage_root()?.trim_end_matches('/'), std::str::from_utf8(sp.as_bytes()).ok()?))
}

fn work_dir(id: &str) -> String {
    format!("{}/notebooks/{}{}", store_root(), WORK_PREFIX, id)
}

fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

/// A name for the log that is not the file's: the first 8 hex digits of a hash of it.
#[allow(dead_code)]
fn ghost(s: &str) -> String {
    format!("{:08x}", fnv(FNV0, s.as_bytes()) as u32)
}

// ---- the lock ----

thread_local! {
    static LOCK: Cell<i32> = const { Cell::new(-1) };
}

/// The result of asking for the import lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lock {
    /// taken now: leftovers of a cut-off import are no one's, and may be dealt with
    Fresh,
    /// this process holds it already (an import is queued or running)
    Held,
    /// another process holds it (a previous run's detached child still importing), or it cannot be opened
    Busy(i32),
}

/// Takes `import/.lock` without blocking, once, and keeps the handle (O_CLOEXEC, so no script and no worker
/// inherits it) while any import is queued or running. Never blocks.
pub fn lock_take() -> Lock {
    if LOCK.with(|l| l.get()) >= 0 {
        return Lock::Held;
    }
    let path = root_path("import/.lock");
    let Ok(c) = std::ffi::CString::new(path) else { return Lock::Busy(libc::ENOENT) };
    // SAFETY: open reads the NUL-terminated path we own; the descriptor it returns is ours alone.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC, 0o644 as libc::c_uint) };
    if fd < 0 {
        return Lock::Busy(std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO));
    }
    // SAFETY: flock only takes the open descriptor.
    if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO);
        // SAFETY: closing the descriptor opened above.
        unsafe { libc::close(fd) };
        return Lock::Busy(e);
    }
    LOCK.with(|l| l.set(fd));
    Lock::Fresh
}

/// The lock's descriptor, or -1: what `child_setup` keeps open in a bgjob child and the detached child, so
/// they hold the very lock the UI holds (the same open file description).
pub fn lock_fd() -> i32 {
    LOCK.with(|l| l.get())
}

/// Lets go of the lock when nothing is queued or running (pictures and PDFs).
pub fn lock_release_if_idle() {
    let fd = lock_fd();
    if fd < 0 || pending_count() > 0 || crate::bgwork_template::pictures_pending() > 0 {
        return;
    }
    // SAFETY: closing the descriptor lock_take opened; the lock goes with it.
    unsafe { libc::close(fd) };
    LOCK.with(|l| l.set(-1));
}

/// The share server's and Live view's children close their copy of the descriptor (the lock is not theirs).
pub fn lock_close_in_child() {
    let fd = lock_fd();
    if fd >= 0 {
        // SAFETY: closing this process's copy; the parent's lock holds.
        unsafe { libc::close(fd) };
        LOCK.with(|l| l.set(-1));
    }
}

// ---- marks: import/.marks/<hash> ----

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MarkState {
    /// the notebook is being made
    #[default]
    Making,
    /// a pick that waits for the lock
    Waiting,
    /// imported (the id of its notebook)
    Done,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mark {
    pub state: MarkState,
    pub id: String,
    pub src: String,
    pub pick: bool,
    pub name: String,
    pub folder: String,
    pub page: u32,
    pub pages: u32,
    pub at: i64,
    /// a done mark's file: its size and mtime (another file of that name later is a new file)
    pub size: u64,
    pub mtime: i64,
}

fn mark_hash(src: &str) -> String {
    format!("{:016x}", fnv(FNV0, src.as_bytes()))
}

fn mark_path(src: &str) -> String {
    root_path(&format!("{}/{}", MARKS_DIR, mark_hash(src)))
}

fn clean_field(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

pub fn mark_text(m: &Mark) -> String {
    format!(
        "# Grimoire: a PDF being imported (docs/FORMAT.md)\nstate={}\nid={}\nsrc={}\npick={}\nname={}\nfolder={}\npage={}\npages={}\nat={}\nsize={}\nmtime={}\n",
        match m.state {
            MarkState::Making => "making",
            MarkState::Waiting => "waiting",
            MarkState::Done => "done",
        },
        clean_field(&m.id),
        clean_field(&m.src),
        m.pick as u8,
        clean_field(&m.name),
        clean_field(&m.folder),
        m.page,
        m.pages,
        m.at,
        m.size,
        m.mtime
    )
}

/// Reads a mark strictly (at most 4 KiB, known keys, an id the store takes).
pub fn mark_parse(t: &[u8]) -> Option<Mark> {
    if t.len() > MARK_MAX {
        return None;
    }
    let t = std::str::from_utf8(t).ok()?;
    let mut m = Mark::default();
    let mut have_state = false;
    for line in t.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once('=')?;
        match k {
            "state" => {
                have_state = true;
                m.state = match v {
                    "making" => MarkState::Making,
                    "waiting" => MarkState::Waiting,
                    "done" => MarkState::Done,
                    _ => return None,
                }
            }
            "id" => {
                if !v.is_empty() {
                    let mut d = String::new();
                    if v.len() >= STORE_ID_LEN || store_notebook_dir(v, &mut d, 700) != 0 {
                        return None;
                    }
                }
                m.id = v.to_string();
            }
            "src" => m.src = v.to_string(),
            "pick" => m.pick = v == "1",
            "name" => m.name = v.to_string(),
            "folder" => m.folder = v.to_string(),
            "page" => m.page = v.parse().ok()?,
            "pages" => m.pages = v.parse().ok()?,
            "at" => m.at = v.parse().ok()?,
            "size" => m.size = v.parse().ok()?,
            "mtime" => m.mtime = v.parse().ok()?,
            _ => {}
        }
    }
    have_state.then_some(m)
}

pub(crate) fn mark_read(src: &str) -> Option<Mark> {
    let t = read_file(&mark_path(src))?;
    mark_parse(&t)
}

/// (the tests write a mark the way the queue does)
pub fn mark_write_public(m: &Mark) -> bool {
    mark_write(m)
}

/// Whether a worker in this process's progress record reported a failed sandbox step.
pub fn sandbox_broken_in_progress() -> bool {
    progress().broken.load(Ordering::Relaxed) != 0
}

fn mark_write(m: &Mark) -> bool {
    let dir = root_path(MARKS_DIR);
    if mkdir_p(&dir) != 0 {
        return false;
    }
    write_file_atomic(&mark_path(&m.src), mark_text(m).as_bytes()) == 0
}

pub(crate) fn mark_drop(src: &str) {
    let _ = std::fs::remove_file(mark_path(src));
}

// ---- the work folder's state ----

/// A PDF import in progress (notebooks/.import-<id>/pdf-import.ini), written by the writer as each slice starts
/// and ends (and after a strike, the picture index always saved before it) and read by the next one, and by the
/// UI to decide whether a cut-off work folder is resumable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub src: String,
    pub size: u64,
    pub mtime: i64,
    pub key: String,
    /// 0 until the probe has run
    pub pages: u32,
    /// the next page to make, from 1; pages + 1 when all are made
    pub next: u32,
    /// the picture ids of pages 1 .. next - 1
    pub pics: Vec<u32>,
    /// pages that became blank (their worker died or stalled twice)
    pub blank: Vec<u32>,
    /// strikes per page (a death or a timeout each), for the pages with any
    pub strikes: Vec<(u32, u32)>,
    /// the CPU its workers used, counted from the probe on (each slice adds its own)
    pub cpu_ms: u64,
    /// a file chosen by hand, the name typed and the Library folder: kept here as well as in the mark, so a lost
    /// or torn mark never loses them (whether the file may move is decided by where it lies: `in_import`)
    pub pick: bool,
    pub name: String,
    pub folder: String,
    /// what the pages made so far take on the storage (pictures and page files): the floor's estimate
    pub bytes: u64,
    /// a slice is running: set as it starts, cleared as it ends, so a slice that finds it set knows the last
    /// writer was cut off
    pub open: bool,
    /// how often the import stopped without finishing and kept its folder (a writer cut off, the CPU budget)
    pub stops: u32,
}

impl State {
    pub fn strikes_of(&self, page: u32) -> u32 {
        self.strikes.iter().find(|s| s.0 == page).map_or(0, |s| s.1)
    }
    pub fn strike(&mut self, page: u32) -> u32 {
        match self.strikes.iter_mut().find(|s| s.0 == page) {
            Some(s) => {
                s.1 += 1;
                s.1
            }
            None => {
                self.strikes.push((page, 1));
                1
            }
        }
    }
    pub fn text(&self) -> String {
        let join = |v: &[u32], hex: bool| v.iter().map(|x| if hex { format!("{x:x}") } else { x.to_string() }).collect::<Vec<_>>().join(",");
        let strikes = self.strikes.iter().map(|(p, n)| format!("{p}:{n}")).collect::<Vec<_>>().join(",");
        format!(
            "# Grimoire: a PDF import in progress (docs/FORMAT.md)\nsrc={}\nsize={}\nmtime={}\nkey={}\npages={}\nnext={}\npics={}\nblank={}\nstrikes={}\ncpu_ms={}\npick={}\nname={}\nfolder={}\nbytes={}\nopen={}\nstops={}\n",
            clean_field(&self.src),
            self.size,
            self.mtime,
            self.key,
            self.pages,
            self.next,
            join(&self.pics, true),
            join(&self.blank, false),
            strikes,
            self.cpu_ms,
            self.pick as u8,
            clean_field(&self.name),
            clean_field(&self.folder),
            self.bytes,
            self.open as u8,
            self.stops
        )
    }
    /// Reads it strictly: bounded, known keys, every list within the page count.
    pub fn parse(t: &[u8]) -> Option<State> {
        if t.len() > STATE_MAX {
            return None;
        }
        let t = std::str::from_utf8(t).ok()?;
        let mut s = State::default();
        let mut have = 0;
        let list = |v: &str, radix: u32| -> Option<Vec<u32>> {
            if v.is_empty() {
                return Some(Vec::new());
            }
            v.split(',').map(|x| u32::from_str_radix(x, radix).ok()).collect()
        };
        for line in t.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let (k, v) = line.split_once('=')?;
            match k {
                "src" => {
                    s.src = v.to_string();
                    have |= 1;
                }
                "size" => s.size = v.parse().ok()?,
                "mtime" => s.mtime = v.parse().ok()?,
                "key" => {
                    if !(v.is_empty() || (v.len() == 32 && v.bytes().all(|c| c.is_ascii_hexdigit()))) {
                        return None;
                    }
                    s.key = v.to_string();
                }
                "pages" => s.pages = v.parse().ok().filter(|&n| n <= PDF_MAX_PAGES)?,
                "next" => {
                    s.next = v.parse().ok()?;
                    have |= 2;
                }
                "pics" => s.pics = list(v, 16)?,
                "blank" => s.blank = list(v, 10)?,
                "strikes" => {
                    if !v.is_empty() {
                        for e in v.split(',') {
                            let (p, n) = e.split_once(':')?;
                            s.strikes.push((p.parse().ok()?, n.parse().ok()?));
                        }
                    }
                }
                "cpu_ms" => s.cpu_ms = v.parse().ok()?,
                "pick" => s.pick = v == "1",
                "name" => s.name = v.to_string(),
                "folder" => s.folder = v.to_string(),
                "bytes" => s.bytes = v.parse().ok()?,
                "open" => s.open = v == "1",
                "stops" => s.stops = v.parse().ok()?,
                _ => {}
            }
        }
        let ok = have == 3
            && s.next >= 1
            && s.next <= s.pages + 1
            && s.pics.len() as u32 == s.next - 1
            && s.blank.iter().all(|&p| p >= 1 && p <= s.pages)
            && s.strikes.iter().all(|&(p, _)| p >= 1 && p <= s.pages)
            && s.pics.iter().all(|&p| p >= 1 && p <= 0xFFFFFF);
        ok.then_some(s)
    }
}

fn state_path(id: &str) -> String {
    format!("{}/{}", work_dir(id), STATE_FILE)
}

fn state_read(id: &str) -> Option<State> {
    State::parse(&read_file(&state_path(id))?)
}

fn state_write(id: &str, s: &State) -> bool {
    write_file_atomic(&state_path(id), s.text().as_bytes()) == 0
}

// ---- stability (3.8) ----

/// What the scan has seen of a file: its size and mtime, and when they were first seen so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub key: u64,
    pub size: u64,
    pub mtime: i64,
    pub first_ms: u32,
}

/// Whether a file has held still long enough: unchanged across two looks at least `gap_ms` apart, and its
/// mtime at least `age_s` old (`wall_s` is the clock now; a stamp ahead of the clock, which a Kindle whose clock
/// is behind sees, counts as old, the two looks still holding). The first look, or a change, only records. No
/// entry is ever pushed out for another (the scan keeps only the files it still lists): past SEEN_MAX a new
/// file is not recorded and waits for a later scan.
pub fn time_stable(seen: &mut Vec<Seen>, key: u64, size: u64, mtime: i64, now: u32, wall_s: i64, gap_ms: u32, age_s: i64) -> bool {
    let i = seen.iter().position(|s| s.key == key);
    let steady = match i {
        Some(i) if seen[i].size == size && seen[i].mtime == mtime => true,
        Some(i) => {
            seen[i] = Seen { key, size, mtime, first_ms: now };
            false
        }
        None => {
            if seen.len() >= SEEN_MAX {
                return false;
            }
            seen.push(Seen { key, size, mtime, first_ms: now });
            false
        }
    };
    let first = seen.iter().find(|s| s.key == key).map_or(now, |s| s.first_ms);
    if gap_ms == 0 && age_s == 0 {
        return true; // (the headless runner's fast mode)
    }
    let age = wall_s - mtime;
    steady && now.wrapping_sub(first) >= gap_ms && (age >= age_s || age < 0)
}

/// How long a file has looked the same to the scan (0 for one it has not seen).
fn unchanged_ms(seen: &[Seen], key: u64, now: u32) -> u32 {
    seen.iter().find(|s| s.key == key).map_or(0, |s| now.wrapping_sub(s.first_ms))
}

/// `%PDF-` in the first KiB and `%%EOF` (white space after it allowed) in the last KiB: a copy has ended.
pub fn looks_complete(head: &[u8], tail: &[u8]) -> bool {
    let has = |h: &[u8], n: &[u8]| h.windows(n.len()).any(|w| w == n);
    if !has(head, b"%PDF-") {
        return false;
    }
    let end = tail.iter().rposition(|c| !matches!(c, b' ' | b'\t' | b'\r' | b'\n' | 0 | 0x0c)).map_or(0, |i| i + 1);
    tail[..end].ends_with(b"%%EOF") || has(&tail[end.saturating_sub(1024)..end], b"%%EOF") && tail[end.saturating_sub(8)..end].ends_with(b"EOF")
}

fn read_ends(path: &str, size: u64) -> Option<(Vec<u8>, Vec<u8>)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = vec![0u8; 1024.min(size as usize)];
    f.read_exact(&mut head).ok()?;
    let from = size.saturating_sub(1024);
    f.seek(SeekFrom::Start(from)).ok()?;
    let mut tail = vec![0u8; (size - from) as usize];
    f.read_exact(&mut tail).ok()?;
    Some((head, tail))
}

// ---- the UI's side: what is queued, what is seen ----

#[derive(Default)]
struct Ui {
    /// the notebook ids of PDF imports queued or running
    pending: Vec<String>,
    /// their sources (storage paths), for the scan not to queue them again
    queued: Vec<String>,
    seen: Vec<Seen>,
    /// the last scan (now_ms), and a look due sooner because a file waits to be stable
    last_scan: Option<u32>,
    rescan_at: Option<u32>,
    /// a previous run's detached child holds the lock: the notice reads the coarse progress in .marks
    detached: bool,
    detached_read: u32,
    /// when the tick last tried the lock while `detached` (every 10 s, so the end of that import is seen)
    detached_tried: u32,
    /// the notice's last text came from this key
    notice_on: bool,
    /// a page folder waits for the PDFs in it: a scan is due as soon as they are done
    pictures_wait: bool,
    /// copies of a PDF being imported, which wait for its notebook: not queued again while imports run
    deferred: Vec<String>,
    /// PDFs that found no room this session, and the bytes free then: taken again once more is free
    room_wait: Vec<(String, u64)>,
}

thread_local! {
    static UI: RefCell<Ui> = RefCell::new(Ui::default());
}

fn ui<R>(f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|u| f(&mut u.borrow_mut()))
}

pub fn pending_count() -> usize {
    ui(|u| u.pending.len())
}

/// Whether any PDF import (queued or running) belongs to `src`.
pub fn is_queued(src: &str) -> bool {
    ui(|u| u.queued.iter().any(|q| q == src))
}

/// Resets what the UI side remembers (tests), the shared record with it.
pub fn ui_reset() {
    ui(|u| *u = Ui::default());
    let p = progress();
    for a in [&p.page, &p.pages, &p.result_for, &p.reason, &p.arg, &p.blank, &p.broken, &p.page_for] {
        a.store(0, Ordering::Relaxed);
    }
}

/// Whether `id` is a PDF import this process has queued or running (the picture imports' ids are checked
/// against these, and these against theirs: one id never names two work folders).
pub(crate) fn is_pending_id(id: &str) -> bool {
    ui(|u| u.pending.iter().any(|p| p == id))
}

fn fresh_id() -> String {
    let mut id = String::new();
    let mut dir = String::new();
    for _ in 0..8 {
        store_make_id(&mut id);
        let queued = is_pending_id(&id) || crate::bgwork_template::is_pending_id(&id);
        if !queued && !(store_notebook_dir(&id, &mut dir, 600) == 0 && exists(&dir)) && !exists(&work_dir(&id)) {
            return id;
        }
    }
    id
}

/// A scan left a folder's pictures waiting for its PDFs: look again when they are done.
pub fn pictures_wait(on: bool) {
    ui(|u| u.pictures_wait = on);
}

/// What a scan found to do.
#[derive(Default, Debug)]
pub struct Found {
    /// loose files and those of `import/<Folder>/`: (path under import/, folder, size, mtime)
    pub files: Vec<(String, String, u64, i64)>,
    /// folders (names) that hold PDFs: their pictures wait until the PDFs are done
    pub pdf_folders: Vec<String>,
    /// work folders a cut-off import left that can go on (their ids)
    pub resumable: Vec<String>,
    pub skipped: usize,
}

fn is_pdf_name(n: &str) -> bool {
    n.len() > 4 && n[n.len() - 4..].eq_ignore_ascii_case(".pdf") && !n.starts_with('.')
}

/// The PDFs of import/ (top level, and one folder down), no links, regular files only, sorted by the
/// comparator the browser and the font list use.
pub fn list_pdfs() -> Found {
    let mut f = Found::default();
    let dir = root_path("import");
    let Ok(rd) = std::fs::read_dir(&dir) else { return f };
    let mut top: Vec<(String, std::fs::Metadata)> = Vec::new();
    for e in rd.flatten() {
        let Ok(name) = e.file_name().into_string() else {
            f.skipped += 1;
            continue;
        };
        if name.starts_with('.') || name == "done" {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            f.skipped += 1;
            continue;
        }
        let Ok(md) = e.metadata() else { continue };
        top.push((name, md));
    }
    top.sort_by(|a, b| name_cmp(&a.0, &b.0));
    for (name, md) in top {
        if md.is_file() && is_pdf_name(&name) {
            f.files.push((name, String::new(), md.len(), md.mtime()));
        } else if md.is_dir() {
            let Ok(rd) = std::fs::read_dir(format!("{dir}/{name}")) else { continue };
            let mut inner: Vec<(String, std::fs::Metadata)> = Vec::new();
            for e in rd.flatten().take(1000) {
                let Ok(n) = e.file_name().into_string() else {
                    f.skipped += 1;
                    continue;
                };
                if !is_pdf_name(&n) {
                    continue;
                }
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_symlink() || !ft.is_file() {
                    f.skipped += 1;
                    continue;
                }
                if let Ok(md) = e.metadata() {
                    inner.push((n, md));
                }
            }
            if !inner.is_empty() {
                f.pdf_folders.push(name.clone());
            }
            inner.sort_by(|a, b| name_cmp(&a.0, &b.0));
            for (n, md) in inner {
                f.files.push((format!("{name}/{n}"), name.clone(), md.len(), md.mtime()));
            }
        }
    }
    f
}

/// Whether a folder of import/ still holds PDFs that can be imported (its pictures wait for them). A PDF that
/// was refused or failed its tries and has not changed since, or one imported already that could not move,
/// holds nothing back: the pictures then import as they did before 2.1.
pub(crate) fn folder_holds_pdfs(name: &str, attempts: &[crate::bgwork_template::Attempt]) -> bool {
    let Ok(rd) = std::fs::read_dir(root_path(&format!("import/{name}"))) else { return false };
    rd.flatten().take(1000).any(|e| {
        let Some(n) = e.file_name().to_str().map(str::to_string) else { return false };
        if !is_pdf_name(&n) || !e.file_type().is_ok_and(|t| t.is_file()) {
            return false;
        }
        let Ok(md) = e.metadata() else { return false };
        let Some(src) = import_storage_path(&format!("{name}/{n}")).and_then(|sp| String::from_utf8(sp.as_bytes().to_vec()).ok()) else { return false };
        let stable_hash = fnv(fnv(FNV0, &md.len().to_ne_bytes()), &md.mtime().to_ne_bytes());
        if attempt_key(&src).is_some_and(|k| crate::bgwork_template::attempts_waiting_key(k, stable_hash, attempts, PDF_TRIES)) {
            return false;
        }
        !mark_read(&src).is_some_and(|m| m.state == MarkState::Done && m.size == md.len() && m.mtime == md.mtime())
    })
}

/// Whether a cut-off work folder can go on: it has a readable state whose source is still there, unchanged.
pub fn keep_work_folder(id: &str) -> bool {
    let Some(s) = state_read(id) else { return false };
    let Ok(sp) = StoragePath::new(s.src.as_bytes()).ok_or(()) else { return false };
    let Some(abs) = abs_of(&sp) else { return false };
    match std::fs::symlink_metadata(&abs) {
        Ok(md) => md.is_file() && md.len() == s.size && md.mtime() == s.mtime,
        Err(_) => false,
    }
}

fn fast_stable() -> bool {
    #[cfg(not(feature = "kindle"))]
    {
        std::env::var("GRIMOIRE_IMPORT_STABLE").is_ok_and(|v| v == "fast")
    }
    #[cfg(feature = "kindle")]
    {
        false
    }
}

/// Whether the lock is held by another process: the Import place says Waiting.
pub fn detached_running() -> bool {
    ui(|u| u.detached)
}

/// What a scan queues (called by `import_scan` once it holds the lock): the PDFs that have held still, the
/// work folders to resume. Returns how many were queued.
pub fn scan_queue(found: &Found, dpi: f32, lock_busy: bool) -> i32 {
    let now = now_ms();
    let wall = wall_time();
    progress(); // (the shared record is made before any fork)
    let mut queued = 0;
    let (mut waiting, mut tried_out, mut already_made, mut skipped_busy, mut no_room) = (0, 0, 0, 0, 0);
    ui(|u| u.rescan_at = None);
    if session_broken() {
        qlog!("import: PDFs are not imported this session (the sandbox refused a step; see the log above)");
        return 0;
    }
    let attempts = crate::bgwork_template::attempts_snapshot();
    // the table of files seen keeps this scan's files only, so no file still listed is ever pushed out
    let listed: std::collections::HashSet<u64> = found
        .files
        .iter()
        .filter_map(|f| import_storage_path(&f.0))
        .map(|sp| fnv(FNV0, sp.as_bytes()))
        .collect();
    ui(|u| u.seen.retain(|s| listed.contains(&s.key)));
    // the imports a cut-off run left go on first (a copy of one waits for its notebook, never the other way)
    for id in &found.resumable {
        if ui(|u| u.pending.len()) >= QUEUE_MAX || is_pending_id(id) {
            continue;
        }
        let Some(s) = state_read(id) else { continue };
        if lock_busy || is_queued(&s.src) {
            continue;
        }
        ui(|u| {
            u.pending.push(id.clone());
            u.queued.push(s.src.clone());
        });
        bg_queue(BgKind::PdfImport, Some(id), Some(&s.src), dpi, false);
        qlog!("import: PDF import %s goes on from page %d of %d", id, s.next as i32, s.pages as i32);
        queued += 1;
    }
    let free = if ui(|u| u.room_wait.is_empty()) { None } else { free_bytes() };
    for (rel, folder, size, mtime) in &found.files {
        let Some(sp) = import_storage_path(rel) else { continue };
        let Ok(src) = String::from_utf8(sp.as_bytes().to_vec()) else { continue };
        if src.len() >= FS_PATH_MAX || is_queued(&src) {
            continue;
        }
        if ui(|u| u.deferred.iter().any(|d| *d == src)) {
            continue; // (a copy of a PDF being imported: after that import, it says "already")
        }
        if let Some(at) = ui(|u| u.room_wait.iter().find(|r| r.0 == src).map(|r| r.1)) {
            if free.is_some_and(|f| f < at.saturating_add(ROOM_MARGIN)) {
                no_room += 1;
                continue;
            }
            ui(|u| u.room_wait.retain(|r| r.0 != src));
        }
        if lock_busy {
            skipped_busy += 1;
            continue;
        }
        if ui(|u| u.pending.len()) >= QUEUE_MAX {
            break;
        }
        if let Some(m) = mark_read(&src) {
            if m.state == MarkState::Done {
                if m.size == *size && m.mtime == *mtime {
                    // imported, its file could not move: nothing to do while it is that very file
                    already_made += 1;
                    continue;
                }
                mark_drop(&src); // (another file of that name since: a new file)
            }
        }
        let key = fnv(FNV0, src.as_bytes());
        let abs = format!("{}/import/{}", store_root(), rel);
        let stable_hash = fnv(fnv(FNV0, &size.to_ne_bytes()), &mtime.to_ne_bytes());
        if attempt_key(&src).is_some_and(|k| crate::bgwork_template::attempts_waiting_key(k, stable_hash, &attempts, PDF_TRIES)) {
            tried_out += 1;
            continue;
        }
        let steady = ui(|u| {
            let (g, a) = if fast_stable() { (0, 0) } else { (STABLE_GAP_MS, STABLE_AGE_S) };
            time_stable(&mut u.seen, key, *size, *mtime, now, wall, g, a)
        });
        if !steady {
            waiting += 1;
            continue;
        }
        let ok = read_ends(&abs, *size).is_some_and(|(h, t)| looks_complete(&h, &t));
        if !ok {
            if ui(|u| unchanged_ms(&u.seen, key, now)) < INCOMPLETE_GIVE_UP_MS {
                waiting += 1;
                continue;
            }
            qlog!("import: a PDF without its end marker has not changed for a minute; it is imported as it is");
        }
        if queue_one(&src, "", folder, false, dpi) {
            queued += 1;
        }
    }
    if waiting > 0 {
        ui(|u| u.rescan_at = Some(now.wrapping_add(WAIT_RESCAN_MS)));
    }
    if queued != 0 || waiting != 0 || tried_out != 0 || found.skipped != 0 || already_made != 0 || skipped_busy != 0 || no_room != 0 {
        qlog!(
            "import: %d PDF(s) queued; left as they are: %d not yet steady, %d that failed and have not changed since, %d imported already, %d skipped (links or odd names), %d while the lock is held elsewhere, %d waiting for room",
            queued,
            waiting,
            tried_out,
            already_made,
            found.skipped as i32,
            skipped_busy,
            no_room
        );
    }
    queued
}

/// Queues one PDF import: its mark first (the notebook id it will be), then the job. `pick`: a file chosen by
/// hand (its name and folder kept for the notebook); whether the file may move is still where it lies.
fn queue_one(src: &str, name: &str, folder: &str, pick: bool, dpi: f32) -> bool {
    let id = fresh_id();
    let prior = mark_read(src);
    // a mark left by a run that was cut off names the notebook this file was becoming: that id again
    let id = match prior {
        Some(m) if m.state == MarkState::Making && !m.id.is_empty() && exists(&work_dir(&m.id)) => m.id,
        _ => id,
    };
    let m = Mark { state: MarkState::Making, id: id.clone(), src: src.to_string(), pick, name: name.to_string(), folder: folder.to_string(), at: wall_time(), ..Default::default() };
    if !mark_write(&m) {
        qlog!("import: a PDF's mark could not be written; left as it is");
        return false;
    }
    progress(); // (the shared record is made before the fork)
    ui(|u| {
        u.pending.push(id.clone());
        u.queued.push(src.to_string());
    });
    bg_queue(BgKind::PdfImport, Some(&id), Some(src), dpi, false);
    true
}

/// The picks that had to wait for the lock (import/.marks, `state=waiting`): any there? (A scan with nothing else
/// to look at still takes the lock for them.)
pub fn marks_waiting() -> bool {
    waiting_marks().next().is_some()
}

fn waiting_marks() -> impl Iterator<Item = Mark> {
    std::fs::read_dir(root_path(MARKS_DIR))
        .into_iter()
        .flat_map(|rd| rd.flatten().take(256))
        .filter_map(|e| read_file(&e.path().to_string_lossy()))
        .filter_map(|t| mark_parse(&t))
        .filter(|m| m.state == MarkState::Waiting)
}

/// When the UI takes the lock afresh: the picks that waited for it are queued now (a PDF as a PDF import, a
/// folder as a picture import), with the name and folder they were given. Returns how many.
pub fn queue_waiting_marks(dpi: f32) -> i32 {
    let mut n = 0;
    let marks: Vec<Mark> = waiting_marks().collect();
    for m in marks {
        if ui(|u| u.pending.len()) + crate::bgwork_template::pictures_pending() >= QUEUE_MAX {
            break;
        }
        let Some(sp) = StoragePath::new(m.src.as_bytes()) else {
            mark_drop(&m.src);
            continue;
        };
        let Some(abs) = abs_of(&sp) else { continue };
        let md = std::fs::symlink_metadata(&abs);
        let ok = match md {
            Ok(md) if md.is_dir() => crate::bgwork_template::queue_folder_pick(&m.src, &m.name, &m.folder, dpi).is_some(),
            Ok(md) if md.is_file() => !is_queued(&m.src) && queue_one(&m.src, &m.name, &m.folder, true, dpi),
            _ => {
                // (gone, or a link: nothing to import)
                mark_drop(&m.src);
                false
            }
        };
        if ok {
            n += 1;
        }
    }
    if n > 0 {
        qlog!("import: %d pick(s) that waited for the lock queued", n);
    }
    n
}

/// Imports this process holds as queued or running that the job queue no longer knows (a full queue dropped
/// one, or its fork failed: bgjob.rs): taken off the list, so the lock and the notice are let go, and queued
/// again when they can go on. A work folder with its state is resumed now; a pick dropped before its first
/// slice waits for the lock as a pick does; a file of import/ is taken by the next scan. Returns how many.
pub fn reconcile(dpi: f32) -> usize {
    let lost: Vec<(String, String)> = ui(|u| u.pending.iter().cloned().zip(u.queued.iter().cloned()).filter(|(id, _)| !bg_pending(BgKind::PdfImport, id)).collect());
    for (id, src) in &lost {
        ui(|u| {
            if let Some(i) = u.pending.iter().position(|p| p == id) {
                u.pending.remove(i);
                if i < u.queued.len() {
                    u.queued.remove(i);
                }
            }
        });
        if keep_work_folder(id) {
            ui(|u| {
                u.pending.push(id.clone());
                u.queued.push(src.clone());
            });
            bg_queue(BgKind::PdfImport, Some(id), Some(src), dpi, false);
            qlog!("import: PDF import %s was dropped from the queue; queued again", id);
        } else if let Some(m) = mark_read(src).filter(|m| m.pick && m.state == MarkState::Making) {
            mark_write(&Mark { state: MarkState::Waiting, id: String::new(), at: wall_time(), ..m });
            qlog!("import: PDF import %s was dropped from the queue; it waits to be queued again", id);
        } else {
            qlog!("import: PDF import %s was dropped from the queue; the next scan takes its file again", id);
        }
    }
    lost.len() + crate::bgwork_template::reconcile_pictures()
}

/// What `pdf_import_queue` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Queued {
    /// queued (the notebook it will be)
    Started(String),
    /// another process holds the import lock: the file is recorded as waiting in import/.marks, and queued once
    /// the lock is taken afresh (the Library's next scan after that import ends)
    Waiting,
    /// this very file is queued or running already
    AlreadyRunning,
    /// not a file this can import
    Refused(&'static str),
}

/// The picker's entry point (lane L8): imports the PDF at `path` (read in place; moved to import/done only
/// when it lies in import/ itself), named `name` (else the file's name) into Library folder `folder` (else the
/// top). Takes the import lock first; when another process holds it the file is recorded as waiting, and
/// queued as soon as a scan takes the lock afresh.
pub fn pdf_import_queue(path: &StoragePath, name: Option<&str>, folder: Option<&str>, dpi: f32) -> Queued {
    let Ok(src) = String::from_utf8(path.as_bytes().to_vec()) else {
        qlog!("import: a picked PDF's path is not UTF-8; refused");
        return Queued::Refused("name");
    };
    if session_broken() {
        qlog!("import: a picked PDF is not imported this session (the sandbox refused a step; see the log above)");
        return Queued::Refused("sandbox");
    }
    if is_queued(&src) {
        return Queued::AlreadyRunning;
    }
    // the folders of Grimoire's own are refused by identity when the job opens it (fsview), and here by
    // name, so a refusal is quick
    let comps: Vec<&[u8]> = path.components().collect();
    if let Some(g) = grimoire_rel() {
        let gc: Vec<&str> = g.split('/').collect();
        if comps.len() > gc.len() && gc.iter().zip(&comps).all(|(a, b)| a.as_bytes().eq_ignore_ascii_case(b)) {
            let n = comps[gc.len()];
            if [&b"notebooks"[..], b"PDFs", b"trash"].iter().any(|r| n.eq_ignore_ascii_case(r)) {
                qlog!("import: a picked PDF lies in one of Grimoire's own folders; refused");
                return Queued::Refused("own");
            }
        }
    }
    match lock_take() {
        Lock::Busy(_) => {
            let m = Mark { state: MarkState::Waiting, src: src.clone(), pick: true, name: name.unwrap_or("").to_string(), folder: folder.unwrap_or("").to_string(), at: wall_time(), ..Default::default() };
            if !mark_write(&m) {
                qlog!("import: a picked PDF's mark could not be written; refused");
                return Queued::Refused("mark");
            }
            ui(|u| u.detached = true);
            qlog!("import: another process holds the import lock; a picked PDF waits for it");
            return Queued::Waiting;
        }
        Lock::Fresh => {
            crate::bgwork_template::import_leftovers_public(true);
            ui(|u| u.detached = false);
            queue_waiting_marks(dpi);
            if is_queued(&src) {
                return Queued::Started(ui(|u| u.queued.iter().position(|q| *q == src).map(|i| u.pending[i].clone()).unwrap_or_default()));
            }
        }
        Lock::Held => {}
    }
    if !queue_one(&src, name.unwrap_or(""), folder.unwrap_or(""), true, dpi) {
        lock_release_if_idle();
        return Queued::Refused("mark");
    }
    Queued::Started(ui(|u| u.pending.last().cloned().unwrap_or_default()))
}

// ---- what the Import place says of a row (lane L8) ----

/// What a row of the browser's Import place says of a file in `grimoire/import` (a PDF; a folder of pictures
/// has `folder_status` in bgwork_template.rs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportStatus {
    /// not taken yet (it has to hold still first), queued behind another, or the lock is held elsewhere
    Waiting,
    /// the page it is on, of how many
    Importing(u32, u32),
    /// tried and failed; it waits until the file changes
    Failed,
    /// made a notebook (its file moved to import/done, or could not and is marked)
    Imported,
}

/// The shared progress record's page and count, and the import they are for, while that import is one this
/// process has queued or running (validated anyway: the writer is a child that may be wrong).
fn live_progress() -> Option<(String, u32, u32)> {
    let p = progress();
    let at = p.page_for.load(Ordering::Acquire);
    let (page, pages) = (p.page.load(Ordering::Relaxed), p.pages.load(Ordering::Relaxed));
    if pages == 0 || pages > PDF_MAX_PAGES {
        return None;
    }
    let id = ui(|u| u.pending.iter().find(|id| id_hash(id) == at).cloned())?;
    Some((id, page.clamp(1, pages), pages))
}

/// The status of the PDF at `src` (its StoragePath as text; `size` and `mtime` as listed). Reads its mark and
/// the attempts: small files, and only for the rows on screen.
pub fn import_status(src: &str, size: u64, mtime: i64) -> ImportStatus {
    if is_queued(src) {
        if let Some((id, p, n)) = live_progress() {
            if ui(|u| u.pending.iter().position(|q| *q == id).and_then(|i| u.queued.get(i)).is_some_and(|q| q == src)) {
                return ImportStatus::Importing(p, n);
            }
        }
        return ImportStatus::Waiting;
    }
    if let Some(m) = mark_read(src) {
        match m.state {
            MarkState::Done if m.size == size && m.mtime == mtime => return ImportStatus::Imported,
            MarkState::Done => {}
            MarkState::Waiting => return ImportStatus::Waiting,
            MarkState::Making => {
                if ui(|u| u.detached) && m.pages > 0 && m.pages <= PDF_MAX_PAGES && !m.id.is_empty() && exists(&work_dir(&m.id)) {
                    return ImportStatus::Importing(m.page.clamp(1, m.pages), m.pages);
                }
            }
        }
    }
    let stable_hash = fnv(fnv(FNV0, &size.to_ne_bytes()), &mtime.to_ne_bytes());
    if attempt_key(src).is_some_and(|k| crate::bgwork_template::attempts_waiting_key(k, stable_hash, &crate::bgwork_template::attempts_snapshot(), PDF_TRIES)) {
        return ImportStatus::Failed;
    }
    ImportStatus::Waiting
}

/// Whether anything of this process imports now (the Import place re-reads its rows' statuses while so).
pub fn import_busy() -> bool {
    ui(|u| !u.pending.is_empty() || u.detached)
}

/// A hash of what changes a row's status text (the browser redraws a row when it changes).
pub fn import_progress_stamp() -> u32 {
    live_progress().map_or(0, |(id, p, n)| p.wrapping_mul(4099) ^ n ^ id_hash(&id)) ^ (pending_count() as u32) << 28
}

/// The UI's tick: the staying notice's text from the shared record, for the import it names (or, while a
/// previous run's detached child imports, from the coarse progress in .marks).
pub fn progress_text(now: u32) -> Option<(u32, u32)> {
    let pending = ui(|u| !u.pending.is_empty());
    if pending {
        return live_progress().map(|(_, p, n)| (p, n));
    }
    if ui(|u| u.detached) && now.wrapping_sub(ui(|u| u.detached_read)) >= 2000 {
        ui(|u| u.detached_read = now);
        return detached_progress();
    }
    None
}

fn detached_progress() -> Option<(u32, u32)> {
    let rd = std::fs::read_dir(root_path(MARKS_DIR)).ok()?;
    let mut best: Option<Mark> = None;
    for e in rd.flatten().take(256) {
        let Some(t) = read_file(&e.path().to_string_lossy()) else { continue };
        let Some(m) = mark_parse(&t) else { continue };
        if m.state == MarkState::Making && m.pages > 0 && m.pages <= PDF_MAX_PAGES && !m.id.is_empty() && exists(&work_dir(&m.id)) && best.as_ref().is_none_or(|b| m.at > b.at) {
            best = Some(m);
        }
    }
    best.map(|m| (m.page.clamp(1, m.pages), m.pages))
}

/// Whether the previous run's detached child is still going (the lock is held elsewhere): checked by trying
/// the lock now and then. Returns true while it holds it.
pub fn detached_check() -> bool {
    if lock_fd() >= 0 {
        ui(|u| u.detached = false);
        return false;
    }
    let busy = match lock_take() {
        Lock::Busy(_) => true,
        Lock::Fresh => {
            lock_release_if_idle();
            false
        }
        Lock::Held => false,
    };
    ui(|u| u.detached = busy);
    busy
}

/// The lock is this process's: no other process imports, whatever an earlier look said.
pub fn detached_clear() {
    ui(|u| u.detached = false);
}

/// The lock was taken afresh (no import runs): copies that waited for another's notebook may go again (any import
/// they waited on is resumed by the same scan, ahead of them).
pub fn deferred_clear() {
    ui(|u| u.deferred.clear());
}

/// The tick, while a previous run's detached child holds the lock: the lock is tried every 10 s, and once it is
/// free the scan is due at once (true: the Library's list is read again, for the notebook that child made).
pub fn detached_poll(now: u32) -> bool {
    if !ui(|u| u.detached) || now.wrapping_sub(ui(|u| u.detached_tried)) < SCAN_EVERY_MS {
        return false;
    }
    ui(|u| u.detached_tried = now);
    if detached_check() {
        return false;
    }
    ui(|u| u.rescan_at = Some(now));
    qlog!("import: the import from an earlier run has ended");
    true
}

/// A scan begins: a look that was due is being taken now (so one that finds nothing to do, or the lock held
/// elsewhere, is not due again at every tick).
pub fn rescan_clear() {
    ui(|u| u.rescan_at = None);
}

/// The scan is due now: at most every 10 s, or sooner when a file was waiting to be stable.
pub fn scan_due(now: u32, entering: bool) -> bool {
    ui(|u| {
        if let Some(at) = u.rescan_at {
            if now.wrapping_sub(at) as i32 >= 0 {
                return true;
            }
        }
        entering && u.last_scan.is_none_or(|l| now.wrapping_sub(l) >= SCAN_EVERY_MS)
    })
}

pub fn notice_on() -> bool {
    ui(|u| u.notice_on)
}
pub fn notice_set(on: bool) {
    ui(|u| u.notice_on = on);
}

pub fn scan_noted(now: u32) {
    ui(|u| u.last_scan = Some(now));
}

/// What the end of a job says to the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finished {
    /// not one of ours (or already told)
    Nothing,
    /// a notebook of this many pages, of which `blank` could not be read, and `trimmed` of whose document links
    /// were left out (the water level, a page's cap or the worker's)
    Imported { pages: u32, blank: u32, trimmed: u32 },
    Failed(FailWhy),
    /// the PDF was imported before
    Already,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailWhy {
    Generic,
    Password,
    TooLong(u32),
    TooBig(u32),
    Storage,
}

/// The result record of import `id` as its job left it: (reason, arg, blank pages), None when the record is
/// another job's. For a finished import `arg` is how many document links it left out.
pub fn import_result(id: &str) -> Option<(u32, u32, u32)> {
    let p = progress();
    (p.result_for.load(Ordering::Acquire) == id_hash(id)).then(|| (p.reason.load(Ordering::Relaxed), p.arg.load(Ordering::Relaxed), p.blank.load(Ordering::Relaxed)))
}

/// A PDF import job ended (`ok` and `code` as bgjob says): what to tell the user. Lets go of the lock when no
/// import is left.
pub fn finished(id: &str, ok: bool) -> Finished {
    // (pending and queued are pushed together, so a job's place in one is its place in the other)
    let src = ui(|u| match u.pending.iter().position(|p| p == id) {
        Some(i) => {
            u.pending.remove(i);
            (i < u.queued.len()).then(|| u.queued.remove(i))
        }
        None => None,
    });
    let Some(src) = src else { return Finished::Nothing };
    let p = progress();
    let mine = p.result_for.load(Ordering::Acquire) == id_hash(id);
    let (reason, arg, blank) = if mine {
        (p.reason.load(Ordering::Relaxed), p.arg.load(Ordering::Relaxed), p.blank.load(Ordering::Relaxed))
    } else {
        (R_NONE, 0, 0)
    };
    if p.broken.load(Ordering::Relaxed) != 0 {
        session_set_broken();
    }
    if p.page_for.load(Ordering::Acquire) == id_hash(id) {
        p.page.store(0, Ordering::Relaxed);
        p.pages.store(0, Ordering::Relaxed);
    }
    if mine && reason == R_STORAGE {
        // (no room is not the file's fault and counts nothing: the scan takes it again once more is free)
        let free = free_bytes().unwrap_or(0);
        ui(|u| {
            u.room_wait.retain(|r| r.0 != src);
            u.room_wait.push((src.clone(), free));
        });
    }
    let deferred = mine && reason == R_DEFERRED;
    if deferred {
        ui(|u| u.deferred.push(src.clone()));
    }
    lock_release_if_idle();
    if pending_count() == 0 {
        // pictures that waited for this folder's PDFs, or copies that waited for a notebook: the scan is due now.
        // (A copy that waited with nothing else running waits on a work folder no job runs, a cut-off import's:
        // it goes again once the lock is taken afresh, which resumes that import first.)
        ui(|u| {
            if u.pictures_wait || (!u.deferred.is_empty() && !deferred) {
                u.pictures_wait = false;
                if !deferred {
                    u.deferred.clear();
                }
                u.rescan_at = Some(now_ms());
            }
        });
    }
    if mine && reason == R_DEFERRED {
        return Finished::Nothing;
    }
    if !ok {
        return Finished::Failed(match reason {
            R_PASSWORD => FailWhy::Password,
            R_TOO_LONG => FailWhy::TooLong(arg),
            R_TOO_BIG => FailWhy::TooBig(arg),
            R_STORAGE => FailWhy::Storage,
            _ => FailWhy::Generic,
        });
    }
    if reason == R_ALREADY {
        return Finished::Already;
    }
    let mut nb = NotebookInfo::default();
    let pages = if store_load_info(id, &mut nb) == 0 { nb.npages() as u32 } else { 0 };
    if pages == 0 {
        return Finished::Nothing;
    }
    // (a finished import's `arg` is the number of links it left out)
    Finished::Imported { pages, blank: if reason == R_DONE { blank } else { 0 }, trimmed: if reason == R_DONE || reason == R_NONE { arg } else { 0 } }
}

// ====================================================================================================
// The job (the writer, in a bgjob child)
// ====================================================================================================

/// How a slice ended.
#[derive(Debug, PartialEq, Eq)]
pub enum SliceEnd {
    /// the notebook is made: its page count and how many pages are blank
    Done { pages: u32, blank: u32 },
    /// the PDF was imported before (this one only needs to move)
    Already,
    /// more slices to run
    More,
    Failed(u32, u32),
    /// a sandbox step refused, or no worker could be started: nothing is counted
    Sandbox,
    /// a copy of this PDF is being imported: this one ended, counting nothing, and is taken again later
    Deferred,
}

// what attempts_note records (bgwork_template.rs)
const A_TRY: i32 = 0;
const A_REFUSED: i32 = 1;
const A_DONE: i32 = 2;
const A_UNTRY: i32 = 3;

pub const CODE_DONE: i32 = 0;
pub const CODE_FAILED: i32 = 1;
pub const CODE_BLANKS: i32 = 2;
pub const CODE_MORE: i32 = 3;

fn mono_ms_now() -> u64 {
    mono_ms()
}

fn read_mem_available() -> Option<u64> {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_PDF_BANDS").ok().and_then(|v| v.parse::<u64>().ok()) {
        // (the runner asks for band mode: report little memory)
        return Some(if v > 1 { 1 << 20 } else { u64::MAX });
    }
    let t = std::fs::read_to_string("/proc/meminfo").ok()?;
    let l = t.lines().find(|l| l.starts_with("MemAvailable:"))?;
    Some(l.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024)
}

fn free_bytes() -> Option<u64> {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_PDF_FREE_BYTES").ok().and_then(|v| v.parse::<u64>().ok()) {
        return Some(v);
    }
    let c = std::ffi::CString::new(store_root()).ok()?;
    // SAFETY: sv is written by statvfs before it is read; c is NUL-terminated.
    let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statvfs(c.as_ptr(), &mut sv) } == 0).then(|| sv.f_bavail as u64 * sv.f_frsize as u64)
}

struct Src {
    fd: std::os::fd::OwnedFd,
    size: u64,
    mtime: i64,
}

fn open_source(src: &str) -> Result<Src, &'static str> {
    use std::os::fd::AsRawFd;
    let root = storage_root().ok_or("no storage")?;
    let st = FsStorage::open(&root, &store_root()).map_err(|_| "storage")?;
    let sp = StoragePath::new(src.as_bytes()).ok_or("path")?;
    let fd = st.open_file(&sp, true).map_err(|e| match e {
        FsErr::NotFound => "gone",
        FsErr::Refused => "refused",
        FsErr::Link => "link",
        _ => "open",
    })?;
    // SAFETY: md is written by fstat before it is read.
    let mut md: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd.as_raw_fd(), &mut md) } != 0 {
        return Err("stat");
    }
    Ok(Src { fd, size: md.st_size as u64, mtime: md.st_mtime as i64 })
}

/// Whether the private read-only mapping the worker needs works on this storage (the S1 spike's question).
fn can_map(fd: i32, len: u64) -> bool {
    if len == 0 {
        return false;
    }
    #[cfg(not(feature = "kindle"))]
    if std::env::var("GRIMOIRE_PDF_NO_MAP").is_ok() {
        return false;
    }
    // One page answers the question (does this storage allow a private mapping): the whole file would ask
    // this process's address space instead, which the writer's RLIMIT_AS (childcaps.rs) keeps small, and
    // refuse every PDF over the read-whole cap. The worker maps the whole file in its own fresh process.
    let n = (len as usize).min(4096);
    // SAFETY: a private, read-only mapping, unmapped at once.
    unsafe {
        let p = libc::mmap(std::ptr::null_mut(), n, libc::PROT_READ, libc::MAP_PRIVATE, fd, 0);
        if p == libc::MAP_FAILED {
            return false;
        }
        libc::munmap(p, n);
    }
    true
}

fn children_cpu_ms() -> u64 {
    // SAFETY: ru is written by getrusage before it is read.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut ru) };
    (ru.ru_utime.tv_sec as u64 + ru.ru_stime.tv_sec as u64) * 1000 + (ru.ru_utime.tv_usec as u64 + ru.ru_stime.tv_usec as u64) / 1000
}

/// The notebook's name: a name given, else the file's name without `.pdf`, cleaned as every name is, and
/// numbered when another notebook has it (two would share a mirror PDF).
pub fn import_name(given: &str, src: &str, all: &[NotebookInfo]) -> String {
    let mut clean = String::new();
    annotate_name(given.as_bytes(), &mut clean, STORE_NAME_LEN);
    if clean.is_empty() {
        // no name given, or one that cleans to nothing (spaces): the file's (D28)
        let f = src.rsplit('/').next().unwrap_or(src);
        let base = f.strip_suffix(".pdf").or_else(|| f.strip_suffix(".PDF")).or_else(|| (f.len() > 4 && f[f.len() - 4..].eq_ignore_ascii_case(".pdf")).then(|| &f[..f.len() - 4])).unwrap_or(f);
        annotate_name(base.as_bytes(), &mut clean, STORE_NAME_LEN);
    }
    unique_import_name(&clean, all)
}

/// The name with " 2", " 3" ... when a notebook has it already (the picture import uses the same rule, so two
/// folders of one name never share a mirror PDF).
pub fn unique_import_name(base: &str, all: &[NotebookInfo]) -> String {
    // TRANSLATORS: the name of a notebook that has none (it is saved with it, as a name given).
    let base = if base.is_empty() { tr("Untitled") } else { base };
    let taken = |n: &str| all.iter().any(|nb| nb.name.as_bytes().eq_ignore_ascii_case(n.as_bytes()));
    if !taken(base) {
        return base.to_string();
    }
    for k in 2..1000 {
        let tail = format!(" {k}");
        let mut cut = String::new();
        annotate_name(base.as_bytes(), &mut cut, STORE_NAME_LEN - tail.len());
        let n = format!("{cut}{tail}");
        if !taken(&n) {
            return n;
        }
    }
    base.to_string()
}

/// A notebook already holds this book key (its `source=`, or a `source.ini` that names it).
fn notebook_with_key(key: &str, all: &[NotebookInfo]) -> bool {
    notebook_id_with_key(key, all).is_some()
}

/// The notebook that holds this book key, when one does (the picker's "has notes" and Open its notebook).
pub fn notebook_id_with_key(key: &str, all: &[NotebookInfo]) -> Option<String> {
    all.iter().find(|nb| {
        nb.source.as_bytes() == key.as_bytes() || (nb.source.is_empty() && {
            let mut m = AnnotateMap::default();
            let r = annotate_map_load(nb.id.as_str(), &mut m) == 0 && m.book.as_bytes() == key.as_bytes();
            annotate_map_free(&mut m);
            r
        })
    }).map(|nb| nb.id.as_str().to_string())
}

/// Another import in flight (another work folder) holds this key.
fn in_flight_with_key(key: &str, my_id: &str) -> bool {
    let Ok(rd) = std::fs::read_dir(root_path("notebooks")) else { return false };
    rd.flatten().any(|e| {
        let n = e.file_name();
        let Some(n) = n.to_str() else { return false };
        n.starts_with(WORK_PREFIX) && &n[WORK_PREFIX.len()..] != my_id && state_read(&n[WORK_PREFIX.len()..]).is_some_and(|s| s.key == key)
    })
}

fn remove_work_folder(id: &str) {
    crate::bgwork_template::remove_work_public(work_dir(id));
}

/// The PDF encoded as a PNG (8-bit gray or RGB, the png crate at Balanced with adaptive filtering).
pub fn encode_png(p: &BgPicture) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.try_reserve(p.px.len() / 4 + 4096).ok()?;
    {
        let mut e = png::Encoder::new(&mut out, p.w as u32, p.h as u32);
        e.set_color(if p.ch == 3 { png::ColorType::Rgb } else { png::ColorType::Grayscale });
        e.set_depth(png::BitDepth::Eight);
        e.set_compression(png::Compression::Balanced);
        e.set_filter(png::Filter::Adaptive);
        let mut w = e.write_header().ok()?;
        w.write_image_data(&p.px).ok()?;
    }
    Some(out)
}

/// Stores one page: its picture (shared by content), its page file, and, for page 1, the card's thumbnail.
/// Returns the picture's id and about what the page took on the storage (its picture when new, its page file).
fn store_one_page(work: &str, bgdir: &str, ix: &mut BgIndex, page_no: u32, px: PagePixels, pw: i32, ph: i32) -> Result<(u32, u64), &'static str> {
    let pic = BgPicture { px: px.px, w: px.w, h: px.h, ch: px.ch };
    let key = bgimage_key(&pic);
    let mut id = 0u32;
    let mut added = false;
    if bgindex_claim(ix, bgdir, key, &mut id, &mut added) != 0 {
        return Err("out of memory");
    }
    let mut bytes = PAGE_FILE_BYTES;
    if added {
        let png = encode_png(&pic).ok_or("a picture could not be encoded")?;
        let mut file = String::new();
        bgimage_file(bgdir, id, &mut file, 900);
        if write_file_atomic(&file, &png) != 0 {
            return Err("a picture could not be written");
        }
        bytes += png.len() as u64;
    }
    if page_no == 1 {
        let mut full = Canvas::default();
        let mut thumb = Canvas::default();
        if bgimage_luma(&pic, &mut full) == 0 && canvas_init(&mut thumb, pw / 3, ph / 3) == 0 {
            canvas_downsample(&mut thumb, &full);
            store_write_pgm(&format!("{work}/thumb.pgm"), &thumb);
        }
        canvas_free(&mut full);
        canvas_free(&mut thumb);
    }
    let pg = page_new(pw, ph, paper_image(PAPER_ROLE_DOCUMENT, id));
    let rc = store_write_page_in(work, page_no, &pg);
    page_free(pg);
    if rc != 0 {
        return Err("a page could not be written");
    }
    Ok((id, bytes))
}

/// The names in a folder (none when it does not read).
fn names_in(dir: &str) -> Vec<String> {
    std::fs::read_dir(dir).map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect()).unwrap_or_default()
}

/// What a slice cut off, or an older build's sweep cut off, left of the pages made (H7), checked as a later slice
/// starts: the index forgets the pictures whose file is gone, `next` goes back to the first page whose picture or
/// page file is missing (from there the pages are made again), and the pictures no index entry names go. One
/// listing of each folder, no file opened. Returns the page the import goes back to, when it does.
fn check_made_pages(work: &str, bgdir: &str, ix: &mut BgIndex, state: &mut State) -> Option<u32> {
    let pngs: std::collections::HashSet<u32> = names_in(bgdir)
        .iter()
        .filter_map(|n| n.strip_suffix(".png").filter(|h| h.len() == 6).and_then(|h| u32::from_str_radix(h, 16).ok()))
        .collect();
    let pagefiles: std::collections::HashSet<u32> = names_in(work).iter().filter_map(|n| n.strip_prefix('p')?.strip_suffix(".qpg")?.parse().ok()).collect();
    ix.e.retain(|x| pngs.contains(&x.id));
    let mut good = 1;
    while good < state.next {
        let pic = state.pics[good as usize - 1];
        if !ix.e.iter().any(|x| x.id == pic) || !pagefiles.contains(&good) {
            break;
        }
        good += 1;
    }
    // bg/<id>.png files that no index entry names (a slice that was cut off wrote them) go
    for id in &pngs {
        if !ix.e.iter().any(|x| x.id == *id) {
            let mut file = String::new();
            bgimage_file(bgdir, *id, &mut file, 900);
            let _ = std::fs::remove_file(&file);
        }
    }
    if good >= state.next {
        return None;
    }
    let made = (state.next - 1) as u64;
    state.bytes = state.bytes.saturating_mul(good as u64 - 1) / made.max(1);
    state.next = good;
    state.pics.truncate(good as usize - 1);
    Some(good)
}

/// Moves a PDF that became a notebook to import/done/<Name>-<YYYYMMDD-HHMMSS>.pdf; one that cannot move is
/// marked done in place, with its size and mtime, so no later start imports that very file again. Only a file
/// that lies in import/ ever moves: any other stays where it is, whatever a mark or a state says (H6).
fn finish_pdf(abs: &str, src: &str, id: &str, size: u64, mtime: i64) {
    if !in_import(src) {
        mark_drop(src);
        return;
    }
    let done = root_path("import/done");
    let stem = Path::new(abs).file_stem().and_then(|s| s.to_str()).unwrap_or("import").to_string();
    let tm = local_tm(wall_time());
    let stamp = format!("{:04}{:02}{:02}-{:02}{:02}{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec);
    let mut moved = false;
    if mkdir_p(&done) == 0 {
        for k in 1..100 {
            let to = if k == 1 { format!("{done}/{stem}-{stamp}.pdf") } else { format!("{done}/{stem}-{stamp}-{k}.pdf") };
            if exists(&to) {
                continue;
            }
            if std::fs::rename(abs, &to).is_ok() {
                moved = true;
            }
            break;
        }
    }
    if moved {
        crate::bgwork_template::sync_dir_public(&done);
        crate::bgwork_template::sync_dir_public(&root_path("import"));
        // (the mark would only ever say "done" for a file that is not there)
        mark_drop(src);
        return;
    }
    let m = Mark { state: MarkState::Done, id: id.to_string(), src: src.to_string(), size, mtime, ..Default::default() };
    mark_write(&m);
    qlog!("import: %s: its PDF could not move to import/done; marked as imported", id);
}

fn fail(id: &str, reason: u32, arg: u32) -> SliceEnd {
    set_result(id, reason, arg, 0);
    SliceEnd::Failed(reason, arg)
}

/// The first estimate of the storage a PDF takes, before any page is made: 60 KB a page and 2 MB, doubled as a
/// margin. Later slices recount from what the pages made really took (`room_for_rest`).
fn need_bytes(pages: u32) -> u64 {
    (PAGE_GUESS * pages as u64 + (2 << 20)) * 2
}

/// Whether the pages left fit above the floor (D29: 100 MB left after a big job), estimated from the pages made
/// so far (a scanned or colour page is megabytes, a planner's a few KB), or 60 KB a page before any.
fn room_for_rest(state: &State) -> bool {
    let Some(free) = free_bytes() else { return true };
    let made = state.next.saturating_sub(1) as u64;
    let left = (state.pages + 1).saturating_sub(state.next) as u64;
    let per = if made > 0 { (state.bytes / made).max(PAGE_FILE_BYTES) } else { PAGE_GUESS };
    fsview_room_for(free, per.saturating_mul(left) + (2 << 20))
}

/// The record of tries for a file of import/ (none for a pick elsewhere): `what` as attempts_note takes it.
fn note_try(src: &str, stable_hash: u64, what: i32) {
    if let Some(k) = attempt_key(src) {
        crate::bgwork_template::attempts_note_key(k, stable_hash, what);
    }
}

/// A SIGPIPE must never end the writer: a worker that ended by itself (`E password`) has closed its end, and
/// the writer's next line would kill it before it read why (M3). A write then fails and is read past.
fn ignore_sigpipe() {
    // SAFETY: setting a signal's disposition to ignore.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
}

#[cfg(not(feature = "kindle"))]
fn test_hooks() -> Vec<(&'static str, u32)> {
    let mut v = Vec::new();
    for (name, var) in [
        ("crash", "GRIMOIRE_PDF_TEST_CRASH_PAGE"),
        ("hang", "GRIMOIRE_PDF_TEST_HANG_PAGE"),
        ("alloc", "GRIMOIRE_PDF_TEST_ALLOC_PAGE"),
        ("trap", "GRIMOIRE_PDF_TEST_TRAP_PAGE"),
        ("links", "GRIMOIRE_PDF_TEST_LINKS"),
    ] {
        if let Some(p) = std::env::var(var).ok().and_then(|x| x.parse::<u32>().ok()) {
            v.push((name, p));
        }
    }
    v
}
#[cfg(feature = "kindle")]
fn test_hooks() -> Vec<(&'static str, u32)> {
    Vec::new()
}

fn start_session(src: &Src, limits: &Limits, id: &str) -> Result<Session, StartErr> {
    use std::os::fd::AsRawFd;
    // Every worker of a slice shares this open file (and its offset): one that reads it whole must start from the
    // beginning, wherever the one before it stopped (M1).
    // SAFETY: lseek on the descriptor this slice opened.
    unsafe { libc::lseek(src.fd.as_raw_fd(), 0, libc::SEEK_SET) };
    let mut s = Session::start(src.fd.as_raw_fd(), src.size, limits)?;
    for (n, p) in test_hooks() {
        s.hook(n, p);
    }
    let pr = progress();
    if pr.sandbox_logged.swap(1, Ordering::Relaxed) == 0 {
        qlog!("%s", &sandboxed_line());
    }
    let _ = id;
    Ok(s)
}

fn log_sandbox_failure(id: &str, step: &str) {
    progress().broken.store(1, Ordering::Relaxed);
    qlog!("import: %s: the sandbox refused its step \"%s\": no PDF is read and no try is counted (this session imports no more PDFs)", id, step);
}

/// A worker that cannot be started (no exec, no handshake, another build) is not the file's fault either: it is
/// treated as a sandbox failure is (DESIGN.md 3.7).
fn log_worker_failure(id: &str, why: &str) {
    progress().broken.store(1, Ordering::Relaxed);
    qlog!("import: %s: a worker could not be started (%s): no try is counted (this session imports no more PDFs)", id, why);
}

/// One slice of a PDF import, in the writer. Slice 0 (no state yet) probes; the others render pages; the last
/// commits.
pub fn run_slice(id: &str, src: &str) -> SliceEnd {
    ignore_sigpipe();
    let mark = mark_read(src);
    let Some(abs) = StoragePath::new(src.as_bytes()).and_then(|sp| abs_of(&sp)) else { return fail(id, R_FAILED, 0) };
    let source = match open_source(src) {
        Ok(s) => s,
        Err(why) => {
            qlog!("import: %s: the PDF cannot be opened (%s)", id, why);
            remove_work_folder(id);
            mark_drop(src);
            return fail(id, R_FAILED, 0);
        }
    };
    match state_read(id) {
        None => first_slice(id, src, &abs, &source, mark.as_ref()),
        Some(state) => later_slice(id, src, &abs, &source, state),
    }
}

/// Slice 0: the checks, then the probe (the page count, the links and names). Its end is the state that makes the
/// import resumable.
fn first_slice(id: &str, src: &str, abs: &str, source: &Src, mark: Option<&Mark>) -> SliceEnd {
    let work = work_dir(id);
    let bgdir = format!("{work}/bg");
    let stable_hash = fnv(fnv(FNV0, &source.size.to_ne_bytes()), &source.mtime.to_ne_bytes());
    let (pick, name, folder) = mark.map_or((false, String::new(), String::new()), |m| (m.pick, m.name.clone(), m.folder.clone()));
    if is_dir_path(&work) {
        remove_work_folder(id); // (a folder without a readable state is no one's)
    }
    note_try(src, stable_hash, A_TRY); // counted first: a try cut off counts too
    // an end that is the file's: a refusal, recorded, waits until the file changes
    let refuse = |reason: u32, arg: u32| {
        note_try(src, stable_hash, A_REFUSED);
        mark_drop(src);
        fail(id, reason, arg)
    };
    // an end that is not the file's (no room on the storage: H1) counts nothing
    let no_room = || {
        note_try(src, stable_hash, A_UNTRY);
        remove_work_folder(id);
        mark_drop(src);
        fail(id, R_STORAGE, 0)
    };
    if source.size == 0 {
        qlog!("import: %s: an empty file; nothing imported", id);
        return refuse(R_FAILED, 0);
    }
    let mapped = can_map(std::os::fd::AsRawFd::as_raw_fd(&source.fd), source.size);
    if !mapped {
        qlog!("import: %s: the device refuses the private mapping of the PDF; the worker reads it whole (%d MB at most)", id, (s1::PDF_MAX_READ_WHOLE >> 20) as i32);
    }
    let cap = if mapped { s1::PDF_MAX_MAPPED } else { s1::PDF_MAX_READ_WHOLE };
    if source.size > cap {
        qlog!("import: %s: a PDF over %d MB; nothing imported%s", id, (cap >> 20) as i32, if mapped { "" } else { " (the device refuses the private mapping)" });
        return refuse(R_TOO_BIG, (cap >> 20) as u32);
    }
    let key = partial_md5_fd(std::os::fd::AsRawFd::as_raw_fd(&source.fd));
    let mut all = Vec::new();
    store_list(&mut all);
    if notebook_with_key(&key, &all) {
        qlog!("import: %s: this PDF was imported before; nothing made", id);
        finish_pdf(abs, src, id, source.size, source.mtime); // (only a file of import/ moves)
        note_try(src, stable_hash, A_DONE);
        set_result(id, R_ALREADY, 0, 0);
        return SliceEnd::Already;
    }
    if in_flight_with_key(&key, id) {
        // A copy of this PDF is being imported. Waiting for its notebook here would hold this job (and, in the
        // detached child, the jobs behind it) for as long as that import takes: this one ends now, counting
        // nothing, and is taken again once the imports are done, when it is "already imported" (H9).
        note_try(src, stable_hash, A_UNTRY);
        if pick {
            mark_write(&Mark { state: MarkState::Waiting, src: src.to_string(), pick, name, folder, at: wall_time(), ..Default::default() });
        } else {
            mark_drop(src);
        }
        qlog!("import: %s: a copy of this PDF is being imported; this one waits for its notebook", id);
        set_result(id, R_DEFERRED, 0, 0);
        return SliceEnd::Deferred;
    }
    if let Some(free) = free_bytes() {
        if !fsview_room_for(free, need_bytes(0)) {
            qlog!("import: %s: not enough room on the storage; nothing imported", id);
            return no_room();
        }
    }
    if mkdir_p(&bgdir) != 0 {
        qlog!("import: %s: notebooks/ cannot be written (%s)", id, &strerror(errno()));
        return no_room();
    }
    let mut st = State { src: src.to_string(), size: source.size, mtime: source.mtime, key, pages: 0, next: 1, pick, name, folder, ..Default::default() };
    let limits = Limits { as_bytes: worker_as_bytes(source.size, false), cpu_secs: s1::SLICE_CPU_SECS, mapped, file_len: source.size };
    let (spw, sph) = {
        let (mut w, mut h) = (0, 0);
        store_page_size(&mut w, &mut h);
        (w, h)
    };
    let mut found: Option<Collected> = None;
    // (the job's CPU is what its own workers use: in the detached child the imports before it are not this one's)
    let cpu0 = children_cpu_ms();
    let pages = match start_session(source, &limits, id) {
        Ok(mut s) => {
            let r = s.probe();
            // the links and names, in the same worker: a failure costs the notebook them, not its pages
            if matches!(r, Ok(n) if n <= PDF_MAX_PAGES) {
                match s.links(spw, sph) {
                    Ok(c) => found = Some(c),
                    Err(why) => qlog!("import: %s: the links and names could not be read (%s); the notebook has none", id, why),
                }
            }
            s.end();
            r
        }
        Err(e) => Err(e),
    };
    let pages = match pages {
        Ok(n) => n,
        Err(e @ (StartErr::Sandbox(_) | StartErr::Worker(_))) => {
            match e {
                StartErr::Sandbox(step) => log_sandbox_failure(id, &step),
                StartErr::Worker(why) => log_worker_failure(id, why),
                _ => {}
            }
            note_try(src, stable_hash, A_UNTRY);
            remove_work_folder(id);
            mark_drop(src);
            set_result(id, R_FAILED, 0, 0);
            return SliceEnd::Sandbox;
        }
        Err(StartErr::Password) => {
            qlog!("import: %s: the PDF needs a password; nothing imported", id);
            remove_work_folder(id);
            return refuse(R_PASSWORD, 0);
        }
        Err(StartErr::Invalid) | Err(StartErr::Empty) => {
            qlog!("import: %s: the file is no PDF Grimoire can read; nothing imported", id);
            remove_work_folder(id);
            return refuse(R_FAILED, 0);
        }
        Err(e) => {
            qlog!("import: %s: the PDF could not be opened (%s); nothing imported", id, &format!("{e:?}"));
            remove_work_folder(id);
            mark_drop(src);
            return fail(id, R_FAILED, 0);
        }
    };
    if pages > PDF_MAX_PAGES {
        qlog!("import: %s: %d pages; nothing imported (a notebook holds %d)", id, pages as i32, PDF_MAX_PAGES as i32);
        remove_work_folder(id);
        return refuse(R_TOO_LONG, PDF_MAX_PAGES);
    }
    if let Some(free) = free_bytes() {
        if !fsview_room_for(free, need_bytes(pages)) {
            qlog!("import: %s: not enough room for %d pages; nothing imported", id, pages as i32);
            return no_room();
        }
    }
    if let Some(c) = &found {
        let w1 = write_file_atomic(&format!("{work}/links.part"), c.links_text().as_bytes());
        let w2 = write_file_atomic(&format!("{work}/names.part"), c.names_text().as_bytes());
        if w1 != 0 || w2 != 0 {
            qlog!("import: %s: the links and names could not be kept (%s); the notebook has none", id, &strerror(errno()));
            let _ = std::fs::remove_file(format!("{work}/links.part"));
            let _ = std::fs::remove_file(format!("{work}/names.part"));
        }
    }
    st.pages = pages;
    st.cpu_ms = children_cpu_ms().saturating_sub(cpu0);
    if !state_write(id, &st) {
        qlog!("import: %s: its state could not be written", id);
        return no_room();
    }
    qlog!("import: %s: %d page(s), the PDF is %d KB", id, pages as i32, (source.size >> 10) as i32);
    set_progress(id, 0, pages);
    write_progress_mark(src, mark, &st, id, 0, pages, true);
    SliceEnd::More
}

/// Ends a slice that keeps its work folder for a later run: the pictures' index first, then the state (whose
/// pictures it must name: H7), with the slice no longer running.
fn keep_slice(id: &str, bgdir: &str, ix: &BgIndex, state: &mut State) -> bool {
    state.open = false;
    bgindex_save(bgdir, ix) == 0 && state_write(id, state)
}

/// A later slice: pages from `next`, at most a slice's worth; the last commits.
fn later_slice(id: &str, src: &str, abs: &str, source: &Src, mut state: State) -> SliceEnd {
    let work = work_dir(id);
    let bgdir = format!("{work}/bg");
    let stable_hash = fnv(fnv(FNV0, &source.size.to_ne_bytes()), &source.mtime.to_ne_bytes());
    // the file is the one the first slice saw
    if source.size != state.size || source.mtime != state.mtime {
        qlog!("import: %s: the PDF changed while it was imported; nothing imported (it is tried again once it holds still)", id);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_FAILED, 0);
    }
    // a slice that found the last one still running: that writer was cut off (a restart, the kernel, a crash,
    // a SIGPIPE): one of the stops, and a few of them end the import (M11)
    if state.open {
        state.stops += 1;
        qlog!("import: %s: its last slice was cut off (%d of %d)", id, state.stops as i32, STOPS_MAX as i32);
    }
    if state.stops >= STOPS_MAX {
        qlog!("import: %s: it stopped %d times without finishing; nothing imported", id, state.stops as i32);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_FAILED, 0);
    }
    // what pages took so far: the pages left must fit above the floor (M2)
    if !room_for_rest(&state) {
        qlog!("import: %s: not enough room for the %d page(s) left; nothing imported", id, (state.pages + 1 - state.next) as i32);
        note_try(src, stable_hash, A_UNTRY);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_STORAGE, 0);
    }
    let mut ix = BgIndex::default();
    if bgindex_load(&bgdir, &mut ix) != 0 {
        qlog!("import: %s: its picture index does not read", id);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_FAILED, 0);
    }
    if let Some(back) = check_made_pages(&work, &bgdir, &mut ix, &mut state) {
        qlog!("import: %s: the files of the pages from %d are gone; they are made again", id, back as i32);
    }
    // this slice runs (a slice that finds this still set knows it was cut off)
    state.open = true;
    if !(bgindex_save(&bgdir, &ix) == 0 && state_write(id, &state)) {
        qlog!("import: %s: its state could not be written; nothing imported", id);
        bgindex_free(&mut ix);
        note_try(src, stable_hash, A_UNTRY);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_STORAGE, 0);
    }
    set_progress(id, state.next.saturating_sub(1), state.pages);
    let cpu_start = children_cpu_ms();
    let (pw, ph) = {
        let (mut w, mut h) = (0, 0);
        store_page_size(&mut w, &mut h);
        (w, h)
    };
    let band_mode = read_mem_available().is_some_and(|a| a < s1::BAND_MODE_BELOW_AVAILABLE);
    let bands = if band_mode { s1::BANDS } else { 1 };
    let mapped = can_map(std::os::fd::AsRawFd::as_raw_fd(&source.fd), source.size);
    let limits = Limits { as_bytes: worker_as_bytes(source.size, band_mode), cpu_secs: s1::SLICE_CPU_SECS, mapped, file_len: source.size };
    let mut sess: Option<Session> = None;
    let t0 = mono_ms_now();
    let mut made = 0usize;
    let mut probes_failed = 0u32;
    let max_pages = slice_pages();
    let mut end: Option<SliceEnd> = None;
    while state.next <= state.pages && made < max_pages && mono_ms_now() - t0 < s1::SLICE_WALL_MS as u64 {
        let page = state.next;
        let px = loop {
            if state.blank.contains(&page) || state.strikes_of(page) >= 2 {
                if !state.blank.contains(&page) {
                    state.blank.push(page);
                    qlog!("import: %s: page %d of %d could not be read (its worker died or stalled twice); it is blank", id, page as i32, state.pages as i32);
                }
                break blank_page(pw, ph);
            }
            if sess.is_none() {
                match start_session(source, &limits, id) {
                    Ok(mut s) => {
                        // the worker's page count is the probe's: a PDF that changed its count is refused
                        match s.probe() {
                            Ok(n) if n == state.pages => sess = Some(s),
                            Ok(_) => {
                                qlog!("import: %s: the PDF's page count changed; nothing imported", id);
                                end = Some(fail(id, R_FAILED, 0));
                                break blank_page(pw, ph);
                            }
                            Err(StartErr::Sandbox(step)) => {
                                log_sandbox_failure(id, &step);
                                end = Some(SliceEnd::Sandbox);
                                break blank_page(pw, ph);
                            }
                            Err(e) => {
                                // A worker that cannot open the file is the file's (or the memory's) trouble, never
                                // the page's: tried again a few times, then the import fails whole (M1).
                                probes_failed += 1;
                                qlog!("import: %s: a worker could not open the PDF (%s, %d of %d)", id, &format!("{e:?}"), probes_failed as i32, PROBE_TRIES as i32);
                                if probes_failed >= PROBE_TRIES {
                                    end = Some(fail(id, R_FAILED, 0));
                                    break blank_page(pw, ph);
                                }
                                continue;
                            }
                        }
                    }
                    Err(StartErr::Sandbox(step)) => {
                        log_sandbox_failure(id, &step);
                        end = Some(SliceEnd::Sandbox);
                        break blank_page(pw, ph);
                    }
                    Err(StartErr::Worker(why)) => {
                        log_worker_failure(id, why);
                        end = Some(SliceEnd::Sandbox);
                        break blank_page(pw, ph);
                    }
                    Err(e) => {
                        probes_failed += 1;
                        qlog!("import: %s: a worker could not open the PDF (%s, %d of %d)", id, &format!("{e:?}"), probes_failed as i32, PROBE_TRIES as i32);
                        if probes_failed >= PROBE_TRIES {
                            end = Some(fail(id, R_FAILED, 0));
                            break blank_page(pw, ph);
                        }
                        continue;
                    }
                }
            }
            let outcome = sess.as_mut().map(|s| s.render(page, pw, ph, bands));
            let why = match outcome {
                Some(PageOutcome::Page(p)) => break p,
                Some(PageOutcome::Died(d)) => death_text(&d),
                Some(PageOutcome::Failed(e)) => format!("could not render it ({e:?})"),
                Some(PageOutcome::NoMemory) => "left the writer without memory for its pixels".to_string(),
                Some(PageOutcome::Lied(why)) => {
                    sess = None;
                    qlog!("import: %s: page %d of %d: the worker sent what the protocol does not allow (%s); nothing imported", id, page as i32, state.pages as i32, why);
                    end = Some(fail(id, R_FAILED, 0));
                    break blank_page(pw, ph);
                }
                None => {
                    end = Some(fail(id, R_FAILED, 0));
                    break blank_page(pw, ph);
                }
            };
            sess = None;
            let n = state.strike(page);
            qlog!("import: %s: page %d of %d: the worker %s (strike %d)", id, page as i32, state.pages as i32, &why, n as i32);
            // (the index first: the state must never name a picture the saved index does not: H7)
            if bgindex_save(&bgdir, &ix) != 0 || !state_write(id, &state) {
                qlog!("import: %s: its state could not be written; nothing imported", id);
                end = Some(fail(id, R_STORAGE, 0));
                break blank_page(pw, ph);
            }
        };
        if end.is_some() {
            break;
        }
        match store_one_page(&work, &bgdir, &mut ix, page, px, pw, ph) {
            Ok((pic, bytes)) => {
                state.pics.push(pic);
                state.bytes += bytes;
            }
            Err(why) => {
                qlog!("import: %s: page %d of %d: %s; nothing imported", id, page as i32, state.pages as i32, why);
                let low = free_bytes().is_some_and(|f| !fsview_room_for(f, 0));
                end = Some(fail(id, if low { R_STORAGE } else { R_FAILED }, 0));
                break;
            }
        }
        state.next += 1;
        made += 1;
        progress().page.store(page, Ordering::Relaxed);
        if made % 10 == 0 {
            qlog!("import: %s: %d of %d pages", id, page as i32, state.pages as i32);
        }
        if made % FLOOR_EVERY == 0 && state.next <= state.pages && !room_for_rest(&state) {
            qlog!("import: %s: not enough room for the %d page(s) left; nothing imported", id, (state.pages + 1 - state.next) as i32);
            end = Some(fail(id, R_STORAGE, 0));
            break;
        }
    }
    if let Some(s) = sess.take() {
        s.end();
    }
    state.cpu_ms += children_cpu_ms().saturating_sub(cpu_start);
    if let Some(e) = end {
        match e {
            SliceEnd::Sandbox => {
                // not the file's fault: nothing counted, and the pages made are kept for the next start (M6)
                if !keep_slice(id, &bgdir, &ix, &mut state) {
                    remove_work_folder(id);
                    mark_drop(src);
                }
                set_result(id, R_FAILED, 0, 0);
            }
            SliceEnd::Failed(R_STORAGE, _) => {
                note_try(src, stable_hash, A_UNTRY);
                remove_work_folder(id);
                mark_drop(src);
            }
            SliceEnd::Failed(..) => {
                remove_work_folder(id);
                mark_drop(src);
            }
            _ => {}
        }
        bgindex_free(&mut ix);
        return e;
    }
    let cap_s = s1::JOB_CPU_BASE_SECS + s1::JOB_CPU_PER_PAGE_SECS * state.pages as u64;
    if state.cpu_ms / 1000 > cap_s {
        // Over the job's CPU budget (s1's, a placeholder until the Scribe is measured): it stops, keeping every page
        // made, and a later scan goes on with a budget of its own; the stops end it (M10).
        state.stops += 1;
        state.cpu_ms = 0;
        qlog!("import: %s: the job used more CPU than %d s; it stops here (%d of %d) and goes on later", id, cap_s as i32, state.stops as i32, STOPS_MAX as i32);
        let kept = keep_slice(id, &bgdir, &ix, &mut state);
        bgindex_free(&mut ix);
        if !kept {
            remove_work_folder(id);
            mark_drop(src);
        }
        return fail(id, R_FAILED, 0);
    }
    if !keep_slice(id, &bgdir, &ix, &mut state) {
        qlog!("import: %s: the slice's files could not be written; nothing imported", id);
        bgindex_free(&mut ix);
        note_try(src, stable_hash, A_UNTRY);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_STORAGE, 0);
    }
    bgindex_free(&mut ix);
    let last = state.next > state.pages;
    write_progress_mark(src, None, &state, id, state.next.saturating_sub(1), state.pages, last);
    if !last {
        return SliceEnd::More;
    }
    commit(id, src, abs, &state, source, pw, ph)
}

fn death_text(d: &Death) -> String {
    match d {
        Death::Timeout => "stalled".to_string(),
        Death::Gone(st) => {
            if libc::WIFSIGNALED(*st) {
                format!("died of signal {}", libc::WTERMSIG(*st))
            } else {
                format!("ended with status {}", libc::WEXITSTATUS(*st))
            }
        }
        Death::Sigsys(n) => format!("made call {n}, which the sandbox does not allow"),
        Death::Panic => "panicked".to_string(),
    }
}

fn is_dir_path(p: &str) -> bool {
    Path::new(p).is_dir()
}

/// The coarse progress in the mark (what the notice reads when it is a previous run's detached child that
/// imports), rewritten at most every 20 s. A mark that is gone or does not read is made again from the state,
/// which keeps a pick a pick, with its name and folder.
fn write_progress_mark(src: &str, prior: Option<&Mark>, state: &State, id: &str, page: u32, pages: u32, force: bool) {
    let now = wall_time();
    let prior = prior.cloned().or_else(|| mark_read(src));
    if !force && prior.as_ref().is_some_and(|m| now - m.at < 20 && m.pages == pages) {
        return;
    }
    let m = Mark {
        state: MarkState::Making,
        id: id.to_string(),
        src: src.to_string(),
        pick: state.pick,
        name: state.name.clone(),
        folder: state.folder.clone(),
        page,
        pages,
        at: now,
        ..Default::default()
    };
    mark_write(&m);
}

/// Folds the first slice's links and names (the work folder's links.part and names.part) into the notebook being
/// made: doclinks.ini (every record validated and put through pdflinks.rs, the water level trimming the densest
/// pages first) and nav.ini (the outline's names, for the pages that have none: all of them, here). Both are
/// optional: a part that is missing or does not read, or a file that cannot be written, leaves the notebook
/// without them and says so in the log. Returns whether doclinks.ini was written, and how many links were left out
/// for room (the toast says so).
fn fold_links(id: &str, work: &str, nb: &NotebookInfo, pw: i32, ph: i32) -> (bool, u32) {
    let read = |n: &str| std::fs::read(format!("{work}/{n}")).ok().filter(|b| b.len() <= 2 * RECORDS_MAX_BYTES);
    let (Some(lt), Some(nt)) = (read("links.part"), read("names.part")) else { return (false, 0) };
    let pages = nb.npages().max(0) as u32;
    let Some(c) = Collected::from_parts(&lt, &nt, pages) else {
        qlog!("import: %s: its links and names do not read; the notebook has none", id);
        return (false, 0);
    };
    // the names
    let mut nav = Nav::default();
    nav_init(&mut nav);
    for (page, text) in &c.names {
        if let Some(&pid) = nb.page_ids.get(*page as usize - 1) {
            nav_set_name(&mut nav, pid, &String::from_utf8_lossy(text));
        }
    }
    let mut ini: Vec<u8> = b"# Grimoire: page names\n".to_vec();
    let mut named = 0;
    for &pid in &nb.page_ids {
        let n = nav_name(&nav, pid);
        if !n.is_empty() {
            ini.extend_from_slice(format!("name.{pid}={n}\n").as_bytes());
            named += 1;
        }
    }
    nav_free(&mut nav);
    if named > 0 && write_file_atomic(&format!("{work}/nav.ini"), &ini) != 0 {
        qlog!("import: %s: its page names could not be written; left out", id);
        named = 0;
    }
    // the links
    let recs: Vec<DocRecord> = c
        .links
        .iter()
        .map(|(l, q)| DocRecord {
            src: nb.page_ids[l.src as usize - 1],
            dst: nb.page_ids[l.dst as usize - 1],
            rect: Rect { x: l.x, y: l.y, w: l.w, h: l.h },
            quad: q.map(|q| {
                let f = |i: usize| q[i] as f32 / 16.0;
                [(f(0), f(1)), (f(2), f(3)), (f(4), f(5)), (f(6), f(7))]
            }),
            index: l.index,
        })
        .collect();
    let mut dl = DocLinks::default();
    let have = |p: u32| nb.page_ids.contains(&p);
    let st = pdflinks_fill(&mut dl, &recs, pw, ph, &have, &|_, _| false);
    let water = pdflinks_water(&mut dl);
    let kept = dl.links.len();
    if doclinks_save_in(work, &mut dl, nb) != 0 {
        qlog!("import: %s: its document links could not be written; left out", id);
        return (false, 0);
    }
    qlog!(
        "import: %s: %d document link(s) and %d page name(s); left out: %d web, %d other action(s), %d without a place to go, %d refused, %d for room",
        id,
        kept as i32,
        named as i32,
        c.left.uri as i32,
        c.left.other as i32,
        c.left.dead as i32,
        st.invalid as i32,
        (water + st.full + c.left.capped as usize) as i32
    );
    (true, (water + st.full + c.left.capped as usize) as u32)
}

fn commit(id: &str, src: &str, abs: &str, state: &State, source: &Src, pw: i32, ph: i32) -> SliceEnd {
    let work = work_dir(id);
    let bgdir = format!("{work}/bg");
    // the file must be the one the slices read
    let again = open_source(src);
    if again.as_ref().map_or(true, |s| s.size != state.size || s.mtime != state.mtime) || source.size != state.size {
        qlog!("import: %s: the PDF changed under the import; nothing imported", id);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_FAILED, 0);
    }
    // the good pages are kept (D37), but a PDF with none is not a notebook of blank pages: it fails whole
    if state.pages == 0 || state.blank.len() as u32 >= state.pages {
        qlog!("import: %s: no page of %d could be read; nothing imported", id, state.pages as i32);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_FAILED, 0);
    }
    let mut final_ = String::new();
    if store_notebook_dir(id, &mut final_, 600) != 0 {
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_FAILED, 0);
    }
    if is_dir_path(&final_) {
        // made by a run cut off before its file moved
        qlog!("import: %s: made by an earlier run; its PDF moves now", id);
        remove_work_folder(id);
        finish_pdf(abs, src, id, source.size, source.mtime); // (only a file of import/ moves)
        note_try(src, 0, A_DONE);
        return SliceEnd::Done { pages: state.pages, blank: state.blank.len() as u32 };
    }
    let n = state.pages as usize;
    let pids: Vec<u32> = (1..=state.pages).collect();
    let papers: Vec<PaperKind> = state.pics.iter().map(|&p| paper_image(PAPER_ROLE_DOCUMENT, p)).collect();
    let mut all = Vec::new();
    store_list(&mut all);
    let mut nb = NotebookInfo::default();
    nb.id.set(id);
    nb.name.set_cut(&import_name(&state.name, src, &all));
    // (the Library folder: the notebook names it, which lists it; folders.ini is the UI's to write, as the import
    // ends: app.rs)
    if !state.folder.is_empty() && store_folder_valid(&state.folder) {
        nb.folder.set_cut(&state.folder);
    }
    nb.paper = PAPER_BLANK;
    nb.created = wall_time();
    nb.modified = nb.created;
    nb.page_ids = pids.clone();
    nb.next_page_id = state.pages + 1;
    nb.page_w = pw;
    nb.page_h = ph;
    nb.source.set(&state.key);
    // the links and names the first slice read (links.part, names.part): doclinks.ini and nav.ini, before
    // notebook.ini is written
    let (links_written, trimmed) = fold_links(id, &work, &nb, pw, ph);
    let mut m = AnnotateMap::default();
    m.book.set(&state.key);
    m.kind.set("paged");
    // (the book's links are in doclinks.ini already: KOReader's Open this book does not ask for them again,
    // which could add each a second time where the two mappers differ: M22)
    m.links = links_written;
    for p in 1..=state.pages {
        annotate_map_add(&mut m, p as i32, state.pages as i32, p);
    }
    let text = annotate_map_text(&m, &nb);
    annotate_map_free(&mut m);
    let wrote = text.as_ref().is_some_and(|t| write_file_atomic(&format!("{work}/source.ini"), t) == 0)
        && store_write_pages_list_in(&bgdir, &pids, &papers) == 0
        && {
            let _ = std::fs::remove_file(state_path(id)); // (a finished notebook carries no import state)
            let _ = std::fs::remove_file(format!("{work}/links.part"));
            let _ = std::fs::remove_file(format!("{work}/names.part"));
            store_write_info_in(&work, &nb) == 0 // notebook.ini last: a folder holding it is whole
        };
    if !wrote {
        qlog!("import: %s: its notebook files could not be written; nothing imported", id);
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_STORAGE, 0);
    }
    crate::bgwork_template::sync_dir_public(&bgdir);
    crate::bgwork_template::sync_dir_public(&work);
    if let Err(e) = std::fs::rename(&work, &final_) {
        qlog!("import: %s: could not move the notebook into place (%s); nothing imported", id, &strerror(e.raw_os_error().unwrap_or(0)));
        remove_work_folder(id);
        mark_drop(src);
        return fail(id, R_STORAGE, 0);
    }
    crate::bgwork_template::sync_dir_public(&root_path("notebooks"));
    let blank = state.blank.len() as u32;
    qlog!("import: notebook %s made of %d page(s), %d picture(s), %d blank", id, n as i32, state.pics.iter().collect::<std::collections::HashSet<_>>().len() as i32, blank as i32);
    // a file of import/ goes to import/done, whoever chose it (M5); any other stays where it is (H6)
    finish_pdf(abs, src, id, source.size, source.mtime);
    note_try(src, 0, A_DONE);
    set_result(id, if blank > 0 { R_DONE } else { R_NONE }, trimmed, blank);
    SliceEnd::Done { pages: state.pages, blank }
}

/// bgjob.h: one slice of a PDF import (BG_PDF_IMPORT), in the child: 0 = done (or nothing to do: imported before,
/// or a copy that waits for another's notebook), 2 = done with unreadable pages, 3 = more slices to run,
/// anything else failed. The job never takes the import lock: the UI holds it.
pub fn bg_pdf_import(id: &str, src: &str) -> i32 {
    match run_slice(id, src) {
        SliceEnd::Done { blank, .. } => {
            if blank > 0 {
                CODE_BLANKS
            } else {
                CODE_DONE
            }
        }
        SliceEnd::Already | SliceEnd::Deferred => CODE_DONE,
        SliceEnd::More => CODE_MORE,
        SliceEnd::Failed(..) | SliceEnd::Sandbox => CODE_FAILED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_taken_after_two_steady_looks_and_five_seconds() {
        let mut seen = Vec::new();
        // first look: only recorded
        assert!(!time_stable(&mut seen, 1, 100, 1000, 10_000, 1100, STABLE_GAP_MS, STABLE_AGE_S));
        // a second look 1 s on: too soon
        assert!(!time_stable(&mut seen, 1, 100, 1000, 11_000, 1101, STABLE_GAP_MS, STABLE_AGE_S));
        // 2 s on, but the file's mtime is only 3 s old
        assert!(!time_stable(&mut seen, 1, 100, 1000, 12_000, 1003, STABLE_GAP_MS, STABLE_AGE_S));
        // 2 s on and 5 s old: taken
        assert!(time_stable(&mut seen, 1, 100, 1000, 12_100, 1005, STABLE_GAP_MS, STABLE_AGE_S));
    }

    #[test]
    fn a_file_that_grows_between_looks_is_not_taken() {
        let mut seen = Vec::new();
        assert!(!time_stable(&mut seen, 7, 100, 1000, 0, 2000, STABLE_GAP_MS, STABLE_AGE_S));
        assert!(!time_stable(&mut seen, 7, 100, 1000, 2500, 2000, STABLE_GAP_MS, STABLE_AGE_S) == false);
        // it grew: the clock starts again
        assert!(!time_stable(&mut seen, 7, 900, 2000, 3000, 2010, STABLE_GAP_MS, STABLE_AGE_S));
        assert!(!time_stable(&mut seen, 7, 900, 2000, 4000, 2020, STABLE_GAP_MS, STABLE_AGE_S));
        assert!(time_stable(&mut seen, 7, 900, 2000, 5100, 2030, STABLE_GAP_MS, STABLE_AGE_S));
    }

    #[test]
    fn a_pdf_without_its_end_marker_waits() {
        let head = b"%PDF-1.7\n%\xe2\xe3";
        assert!(looks_complete(head, b"...\nstartxref\n123\n%%EOF\n"));
        assert!(looks_complete(head, b"%%EOF"));
        assert!(looks_complete(head, b"%%EOF \r\n\r\n "));
        assert!(!looks_complete(head, b"...\nstartxref\n123\n"));
        assert!(!looks_complete(b"GIF89a", b"%%EOF"));
        assert!(!looks_complete(head, b""));
        assert!(!looks_complete(b"", b"%%EOF"));
    }

    #[test]
    fn marks_round_trip_and_refuse_junk() {
        let m = Mark { state: MarkState::Making, id: "20261109-101112-ab12".into(), src: "grimoire/import/A.pdf".into(), pick: true, name: "My book".into(), folder: "Work".into(), page: 3, pages: 40, at: 99, size: 5000, mtime: 7 };
        let r = Mark { ..m.clone() };
        // (the id must be one the store takes)
        let t = mark_text(&m);
        let p = mark_parse(t.as_bytes());
        assert_eq!(p, Some(r));
        assert!(mark_parse(b"state=zzz\n").is_none());
        assert!(mark_parse(b"id=x\n").is_none());
        assert!(mark_parse(&vec![b'a'; 5000]).is_none());
        assert!(mark_parse(b"state=making\nid=../../etc\n").is_none());
        assert!(mark_parse(b"state=making\npage=x\n").is_none());
    }

    fn sample_state() -> State {
        State {
            src: "grimoire/import/A.pdf".into(),
            size: 5000,
            mtime: 7,
            key: "0123456789abcdef0123456789abcdef".into(),
            pages: 6,
            next: 4,
            pics: vec![0x1a, 0x2b, 0x1a],
            blank: vec![2],
            strikes: vec![(2, 2), (5, 1)],
            cpu_ms: 1200,
            pick: true,
            name: "My = book".into(),
            folder: "Work/2027".into(),
            bytes: 123_456,
            open: true,
            stops: 2,
        }
    }

    #[test]
    fn state_round_trips_and_keeps_the_blank_pages() {
        let s = sample_state();
        let back = State::parse(s.text().as_bytes()).expect("parses");
        assert_eq!(back, s);
        assert_eq!(back.blank, vec![2]);
        assert_eq!(back.strikes_of(2), 2);
        assert_eq!(back.strikes_of(9), 0);
    }

    #[test]
    fn state_refuses_what_does_not_hold_together() {
        let ok = sample_state().text();
        assert!(State::parse(ok.as_bytes()).is_some());
        // pictures that do not match the next page
        assert!(State::parse(ok.replace("next=4", "next=5").as_bytes()).is_none());
        // more pages than a notebook holds
        assert!(State::parse(ok.replace("pages=6", "pages=2001").as_bytes()).is_none());
        // a blank page outside the PDF
        assert!(State::parse(ok.replace("blank=2", "blank=9").as_bytes()).is_none());
        // a key that is no MD5
        assert!(State::parse(ok.replace("0123456789abcdef0123456789abcdef", "xyz").as_bytes()).is_none());
        assert!(State::parse(&vec![b'#'; STATE_MAX + 1]).is_none());
        assert!(State::parse(b"src=x\n").is_none());
    }

    #[test]
    fn a_strike_counts_per_page() {
        let mut s = State::default();
        assert_eq!(s.strike(3), 1);
        assert_eq!(s.strike(3), 2);
        assert_eq!(s.strike(4), 1);
        assert_eq!(s.strikes_of(3), 2);
    }

    #[test]
    fn names_come_from_the_file_and_never_repeat() {
        let mut a = NotebookInfo::default();
        a.name.set("Planner");
        let all = vec![a];
        assert_eq!(import_name("", "grimoire/import/Diary.pdf", &all), "Diary");
        assert_eq!(import_name("", "grimoire/import/Planner.PDF", &all), "Planner 2");
        assert_eq!(import_name("Mine", "grimoire/import/Planner.pdf", &all), "Mine");
        assert_eq!(import_name("", "grimoire/import/Planner.pdf", &[]), "Planner");
        // a name is never a path
        assert_eq!(import_name("", "grimoire/import/Folder/Sub.pdf", &[]), "Sub");
    }

    #[test]
    fn png_encoding_keeps_gray_gray_and_colour_colour() {
        let g = BgPicture { px: vec![0, 64, 128, 255], w: 2, h: 2, ch: 1 };
        let c = BgPicture { px: vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 9, 9, 9], w: 2, h: 2, ch: 3 };
        for (p, ct) in [(&g, png::ColorType::Grayscale), (&c, png::ColorType::Rgb)] {
            let bytes = encode_png(p).unwrap();
            let dec = png::Decoder::new(std::io::Cursor::new(bytes));
            let mut r = dec.read_info().unwrap();
            let info = r.info();
            assert_eq!((info.color_type, info.bit_depth), (ct, png::BitDepth::Eight));
            let mut buf = vec![0u8; r.output_buffer_size().unwrap()];
            let f = r.next_frame(&mut buf).unwrap();
            assert_eq!(&buf[..f.buffer_size()], &p.px[..]);
        }
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("pdfimport.rs").contains('\u{2014}'));
    }
}
