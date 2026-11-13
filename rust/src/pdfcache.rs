//! The notebook PDF's page streams, kept so that a changed notebook renders only its changed pages
//! (docs/FORMAT.md, "grimoire/.cache/pdf/"). Each page's compressed image stream lives in
//! `grimoire/.cache/pdf/<notebook id>/p<page id>.z` with the stamp it was made under: the page file's
//! identity (size, modification time, inode), the renderer's version and the ink renderer's fingerprint,
//! the dpi, the notebook's page size and paper, and the page's picture file as it was (its size, its
//! modification time, its channel count). A record that does not read, whose checksum is wrong, or whose
//! stamp is not the page's today is never used: the page is drawn again (full render on any doubt). A page
//! drawn without its picture (not made yet, or not decoded) is never kept.
//!
//! Every cache is disposable: deleting this folder loses nothing. Only background jobs write it (the
//! mirror's child); the share server's render children read it and never write. Writes are best
//! effort: the first one that fails (a full disk) turns the cache off for that run, and the PDF is
//! made as if there were none. The cache holds 256 MB in all: past it whole notebooks go, the one used
//! longest ago first, and a notebook that alone is over it keeps its first pages' records and writes no
//! more, so an unchanged notebook draws the same pages at every close and writes nothing.

use crate::prelude::*;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// 256 MB in all.
pub const PDF_CACHE_MAX: u64 = 256 << 20;
/// Bumped whenever a change to the PDF's own drawing (the colour, the compression, the record) would make
/// an old record wrong. Changes to the ink and paper renderer change its fingerprint, which is in the
/// stamp too (page_render_fingerprint, as the kept drawings have it).
const PDF_RENDER_VER: u32 = 2;
/// A record's header, in bytes.
const HEAD: usize = 112;
/// The most a record's stream may be (a page is one image of at most 6000 x 6000 pixels).
const Z_MAX: usize = 128 << 20;
/// How long a mirror waits for another mirror of the same notebook to finish (the lock) before it goes
/// on beside it: each writes a temporary file of its own, so the worst then is the older state last.
const LOCK_WAIT_MS: u32 = 15 * 60 * 1000;

/// Pages the PDF writer has drawn (not found in the cache) in this process: tests count them.
static RENDERED: AtomicU32 = AtomicU32::new(0);
/// Tests: make every cache write fail as a full disk does.
static FULL: AtomicBool = AtomicBool::new(false);
/// The cap (tests set another).
static CAP: AtomicU64 = AtomicU64::new(PDF_CACHE_MAX);

pub fn pdf_pages_rendered() -> u32 {
    RENDERED.load(Ordering::Relaxed)
}
pub(crate) fn pdf_page_rendered() {
    RENDERED.fetch_add(1, Ordering::Relaxed);
}
/// Tests: while set, the cache's writes fail (a full disk).
pub fn pdf_cache_inject_full(on: bool) {
    FULL.store(on, Ordering::Relaxed);
}
/// Tests: the cap the mirror keeps the cache to (PDF_CACHE_MAX until set).
pub fn pdf_cache_set_cap(max: u64) {
    CAP.store(max, Ordering::Relaxed);
}

/// How an export uses the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfCacheMode {
    /// every page drawn, nothing read or kept
    Off,
    /// records used when they fit, never written (the share server's children)
    Read,
    /// records used, and the pages drawn kept (the mirror job's child)
    ReadWrite,
}

/// What a record was made under, besides its page's picture (checked apart: the page's paper is in the
/// record).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PdfStamp {
    render: u32,
    version: u64,
    dpi: u32,
    size: u64,
    ino: u64,
    sec: i64,
    nsec: u32,
    nbw: i32,
    nbh: i32,
    /// the notebook's paper: what a missing page file is drawn on
    nbpaper: u32,
}

/// A page's picture file as it was when the page was drawn: its channels by its PNG header, its size and
/// modification time. All zero for a page on no picture; size 0 for a picture that is not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PicStamp {
    ch: u32,
    size: u64,
    sec: i64,
    nsec: u32,
}

/// One page's image, as it goes into the PDF.
pub struct PageImage {
    /// pixels
    pub w: i32,
    pub h: i32,
    /// 1 (DeviceGray) or 3 (DeviceRGB)
    pub ch: u8,
    /// the zlib stream of the rows: a record's bytes, or the compressor's own block (never copied)
    z: Box<dyn AsRef<[u8]>>,
}
impl PageImage {
    pub fn new(w: i32, h: i32, ch: u8, z: impl AsRef<[u8]> + 'static) -> PageImage {
        PageImage { w, h, ch, z: Box::new(z) }
    }
    /// The zlib stream.
    pub fn stream(&self) -> &[u8] {
        (*self.z).as_ref()
    }
}

/// A record read back: the image, and what it was made from.
struct Record {
    stamp: PdfStamp,
    paper: u32,
    pic: PicStamp,
    img: PageImage,
}

pub struct PdfCache {
    dir: String,
    write: bool,
    /// a write failed: no more are tried in this run
    off: bool,
    stamp: PdfStamp,
    /// picture stamps by paper id, each taken once a run, before any page on it is drawn
    pics: std::collections::HashMap<u32, PicStamp>,
    /// the bytes of this notebook's records used or written in this run, in page order: past the cap no
    /// more are written
    kept: u64,
    cap: u64,
}

fn fnv(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// grimoire/.cache/pdf, the cache's folder (a notebook's records are in its id's folder).
pub fn pdf_cache_root() -> String {
    format!("{}/.cache/pdf", store_root())
}

/// Whether `dir` and the folder it is in are real folders (never links). The cache is pruned with remove_dir_all
/// and written as root: a linked .cache, .cache/pdf or notebook folder of it would be pruned and written in its
/// target, anywhere on the device (security review 2026-11). A folder that is not there yet passes (mkdir_p
/// makes it); a link never does.
fn real_dirs(dir: &str) -> bool {
    let ok = |p: &str| match std::fs::symlink_metadata(p) {
        Ok(m) => m.file_type().is_dir(),
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    };
    let parent = dir.rsplit_once('/').map_or("", |x| x.0);
    ok(dir) && (parent.is_empty() || ok(parent))
}

/// A file of the cache created afresh, never through a link (O_EXCL after any stale one goes).
fn create_fresh(path: &str) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let _ = std::fs::remove_file(path);
    std::fs::OpenOptions::new().write(true).create_new(true).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC).open(path)
}

fn page_file(nb: &NotebookInfo, pid: u32) -> String {
    let mut dir = String::new();
    store_notebook_dir(nb.id.as_str(), &mut dir, 700);
    format!("{dir}/p{pid}.qpg")
}

/// A notebook id that names a folder of the cache (and of notebooks/).
fn good_id(id: &str) -> bool {
    !id.is_empty() && !id.contains(['/', '\\']) && !id.starts_with('.')
}

fn pic_stamp(nb: &NotebookInfo, k: PaperKind) -> PicStamp {
    use std::os::unix::fs::MetadataExt;
    if !paper_is_image(k) {
        return PicStamp::default();
    }
    let mut dir = String::new();
    let mut path = String::new();
    if bgimage_dir(nb.id.as_str(), &mut dir, 700) != 0 {
        return PicStamp { ch: 1, ..PicStamp::default() };
    }
    bgimage_file(&dir, paper_image_id(k), &mut path, 800);
    let Ok(m) = std::fs::metadata(&path) else { return PicStamp { ch: 1, ..PicStamp::default() } };
    PicStamp { ch: bgimage_channels(nb.id.as_str(), k) as u32, size: m.len(), sec: m.mtime(), nsec: m.mtime_nsec() as u32 }
}

impl PdfCache {
    /// None for Off, and for a notebook whose folder is no good name.
    pub fn open(nb: &NotebookInfo, dpi: f32, mode: PdfCacheMode) -> Option<PdfCache> {
        if mode == PdfCacheMode::Off || !good_id(nb.id.as_str()) {
            return None;
        }
        let (mut w, mut h) = (0, 0);
        store_notebook_page_size(nb, &mut w, &mut h);
        let version = fnv(GRIMOIRE_VERSION) ^ page_render_fingerprint();
        Some(PdfCache {
            dir: format!("{}/{}", pdf_cache_root(), nb.id.as_str()),
            write: mode == PdfCacheMode::ReadWrite,
            off: false,
            stamp: PdfStamp { render: PDF_RENDER_VER, version, dpi: dpi.to_bits(), nbw: w, nbh: h, nbpaper: nb.paper.0, ..PdfStamp::default() },
            pics: std::collections::HashMap::new(),
            kept: 0,
            cap: CAP.load(Ordering::Relaxed),
        })
    }

    /// Whether this run keeps the pages it draws.
    pub fn writes(&self) -> bool {
        self.write && !self.off
    }

    /// What a record of the page must have been made under, as the page file is now: taken before the
    /// page is read, so a save that comes while it is drawn leaves the record stale, never trusted.
    pub fn stamp_of(&self, nb: &NotebookInfo, pid: u32) -> PdfStamp {
        use std::os::unix::fs::MetadataExt;
        let mut s = self.stamp;
        if let Ok(m) = std::fs::metadata(page_file(nb, pid)) {
            s.size = m.len();
            s.ino = m.ino();
            s.sec = m.mtime();
            s.nsec = m.mtime_nsec() as u32;
        }
        s
    }

    /// The stamp of image paper `paper`'s picture file in this run: taken the first time it is asked for,
    /// which is before any page on it is drawn (a picture made or replaced later in the run leaves the
    /// records stale, never trusted).
    pub fn picture(&mut self, nb: &NotebookInfo, paper: u32) -> PicStamp {
        *self.pics.entry(paper).or_insert_with(|| pic_stamp(nb, PaperKind(paper)))
    }

    fn record_path(&self, pid: u32) -> String {
        format!("{}/p{}.z", self.dir, pid)
    }

    /// The page's image when a record of it was made from the page as it is now.
    pub fn get(&mut self, nb: &NotebookInfo, pid: u32, stamp: &PdfStamp) -> Option<PageImage> {
        let data = std::fs::read(self.record_path(pid)).ok()?;
        let len = data.len() as u64;
        let rec = parse(data)?;
        if rec.stamp != *stamp || rec.pic != self.picture(nb, rec.paper) {
            return None;
        }
        self.kept += len;
        Some(rec.img)
    }

    /// Keeps the image just drawn for the page (best effort). `paper` is the page's paper kind and `pic`
    /// its picture's stamp, both as they were before the page was drawn. Past the cap nothing is written
    /// (and the page's old record goes): the records kept are the first pages'.
    pub fn put(&mut self, pid: u32, stamp: &PdfStamp, paper: u32, pic: PicStamp, img: &PageImage) {
        if !self.write || self.off {
            return;
        }
        let z = img.stream();
        let len = (HEAD + z.len()) as u64;
        if self.kept + len > self.cap {
            let _ = std::fs::remove_file(self.record_path(pid));
            return;
        }
        let head = serialize_head(stamp, paper, pic, img, z.len(), crc32(z));
        if !self.write_record(pid, &head, z) {
            self.off = true;
            qlog!("pdf cache: a write failed; this run goes on without it");
            return;
        }
        self.kept += len;
    }

    fn write_record(&self, pid: u32, head: &[u8], z: &[u8]) -> bool {
        if FULL.load(Ordering::Relaxed) || !real_dirs(&self.dir) || !real_dirs(&pdf_cache_root()) || mkdir_p(&self.dir) != 0 {
            return false;
        }
        let path = self.record_path(pid);
        let tmp = format!("{path}.{}.tmp", std::process::id());
        let ok = (|| -> std::io::Result<()> {
            let mut f = create_fresh(&tmp)?;
            f.write_all(head)?;
            f.write_all(z)?;
            f.flush()
        })()
        .is_ok();
        if !ok || std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return false;
        }
        true
    }
}

fn crc32(p: &[u8]) -> u32 {
    let mut c = 0xFFFFFFFFu32;
    for &b in p {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn serialize_head(st: &PdfStamp, paper: u32, pic: PicStamp, img: &PageImage, zlen: usize, crc: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(HEAD);
    v.extend_from_slice(b"GPZ2");
    v.extend_from_slice(&st.render.to_le_bytes());
    v.extend_from_slice(&st.version.to_le_bytes());
    v.extend_from_slice(&st.dpi.to_le_bytes());
    v.extend_from_slice(&st.size.to_le_bytes());
    v.extend_from_slice(&st.sec.to_le_bytes());
    v.extend_from_slice(&st.nsec.to_le_bytes());
    v.extend_from_slice(&st.nbw.to_le_bytes());
    v.extend_from_slice(&st.nbh.to_le_bytes());
    v.extend_from_slice(&paper.to_le_bytes());
    v.extend_from_slice(&pic.ch.to_le_bytes());
    v.extend_from_slice(&((img.ch as u32) << 24).to_le_bytes());
    v.extend_from_slice(&img.w.to_le_bytes());
    v.extend_from_slice(&img.h.to_le_bytes());
    v.extend_from_slice(&(zlen as u32).to_le_bytes());
    v.extend_from_slice(&crc.to_le_bytes());
    v.extend_from_slice(&st.ino.to_le_bytes());
    v.extend_from_slice(&st.nbpaper.to_le_bytes());
    v.extend_from_slice(&pic.size.to_le_bytes());
    v.extend_from_slice(&pic.sec.to_le_bytes());
    v.extend_from_slice(&pic.nsec.to_le_bytes());
    while v.len() < HEAD {
        v.push(0);
    }
    v
}

fn parse(mut d: Vec<u8>) -> Option<Record> {
    if d.len() < HEAD || &d[..4] != b"GPZ2" {
        return None;
    }
    let mut at = 4;
    let mut take = |n: usize| {
        let s = &d[at..at + n];
        at += n;
        s
    };
    let u32_ = |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    let u64_ = |s: &[u8]| u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]);
    let render = u32_(take(4));
    let version = u64_(take(8));
    let dpi = u32_(take(4));
    let size = u64_(take(8));
    let sec = u64_(take(8)) as i64;
    let nsec = u32_(take(4));
    let nbw = u32_(take(4)) as i32;
    let nbh = u32_(take(4)) as i32;
    let paper = u32_(take(4));
    let pic_ch = u32_(take(4));
    let ch = (u32_(take(4)) >> 24) as u8;
    let w = u32_(take(4)) as i32;
    let h = u32_(take(4)) as i32;
    let zlen = u32_(take(4)) as usize;
    let crc = u32_(take(4));
    let ino = u64_(take(8));
    let nbpaper = u32_(take(4));
    let pic = PicStamp { ch: pic_ch, size: u64_(take(8)), sec: u64_(take(8)) as i64, nsec: u32_(take(4)) };
    let stamp = PdfStamp { render, version, dpi, size, ino, sec, nsec, nbw, nbh, nbpaper };
    if (ch != 1 && ch != 3) || w < 1 || h < 1 || zlen > Z_MAX || d.len() != HEAD + zlen {
        return None;
    }
    if crc32(&d[HEAD..]) != crc {
        return None;
    }
    d.drain(..HEAD); // (the stream, without a second copy of it)
    Some(Record { stamp, paper, pic, img: PageImage::new(w, h, ch, d) })
}

/// A held lock on one notebook's mirror (`.cache/pdf/<id>.lock`): let go when dropped.
pub struct MirrorLock {
    _f: std::fs::File,
}

/// Takes notebook `id`'s mirror lock, waiting (up to LOCK_WAIT_MS) while another mirror of it runs: a
/// mirror an earlier run of Grimoire left running beside the one this run queued, which then writes
/// after it, the newer state last. None when the lock cannot be had (a full disk, a folder that cannot
/// be made, the wait run out): the mirror goes on without it, its temporary file its own.
pub fn pdf_mirror_lock(id: &str) -> Option<MirrorLock> {
    use std::os::fd::AsRawFd;
    if !good_id(id) {
        return None;
    }
    let root = pdf_cache_root();
    if !real_dirs(&root) || mkdir_p(&root) != 0 {
        return None;
    }
    use std::os::unix::fs::OpenOptionsExt;
    let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK).open(format!("{root}/{id}.lock")).ok()?;
    let t0 = now_ms();
    let mut said = false;
    loop {
        // SAFETY: flock only takes the open descriptor.
        if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Some(MirrorLock { _f: f });
        }
        let e = errno();
        if e != libc::EWOULDBLOCK && e != libc::EINTR {
            return None; // (a file system without locks: each mirror's temporary file is its own)
        }
        if !said {
            said = true;
            qlog!("bg: export of %s waits for the one running", id);
        }
        if now_ms().wrapping_sub(t0) > LOCK_WAIT_MS {
            qlog!("bg: export of %s goes on beside the one running", id);
            return None;
        }
        sleep_ms(100);
    }
}

/// A picture job for notebook `id` ended (in its child): the notebook's records go, as its page previews
/// are made again. (Each record names its picture's file as it was, and a page drawn without its picture
/// is never kept: this is the last of three guards.)
pub fn pdf_cache_forget(id: &str) {
    if good_id(id) && real_dirs(&pdf_cache_root()) {
        let _ = std::fs::remove_dir_all(format!("{}/{}", pdf_cache_root(), id));
    }
}

/// The page ids a notebook's records may name, and the records of every other: after a mirror, the
/// pages deleted from the notebook, the notebooks deleted, and then, while the folder holds more than
/// the cap, whole notebooks, the one used longest ago first (never this one), then this one's records of
/// its last pages. Best effort; never touches anything outside the cache.
pub fn pdf_cache_prune(nb: &NotebookInfo) {
    prune_in(&pdf_cache_root(), Some(nb), CAP.load(Ordering::Relaxed));
}

/// pdf_cache_prune with another size cap (tests).
pub fn pdf_cache_prune_to(nb: &NotebookInfo, max: u64) {
    prune_in(&pdf_cache_root(), Some(nb), max);
}

/// The prune with no notebook in hand, at most once a day (any background job's child calls it, so a
/// cache whose mirror is off still loses the records of deleted notebooks and stays within its cap).
pub fn pdf_cache_sweep_due() {
    let root = pdf_cache_root();
    let stamp = format!("{root}/.swept");
    let due = std::fs::metadata(&stamp).ok().and_then(|m| m.modified().ok()).is_none_or(|t| std::time::SystemTime::now().duration_since(t).is_ok_and(|d| d.as_secs() >= 86400));
    if !due || !std::path::Path::new(&root).is_dir() || !real_dirs(&root) {
        return;
    }
    if create_fresh(&stamp).is_ok() {
        prune_in(&root, None, CAP.load(Ordering::Relaxed));
    }
}

/// Sets a folder's modification time to now (its notebook's last use).
fn touch(path: &str) {
    if let Ok(c) = std::ffi::CString::new(path) {
        // SAFETY: utimensat reads the NUL-terminated path; a null times sets both to now.
        unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), std::ptr::null(), 0) };
    }
}

pub(crate) fn prune_in(root: &str, nb: Option<&NotebookInfo>, max: u64) {
    if !real_dirs(root) {
        return; // (a linked cache is never pruned: it would be pruned in its target)
    }
    let Ok(rd) = std::fs::read_dir(root) else { return };
    let notebooks = format!("{}/notebooks", store_root());
    let gone = |id: &str| !std::path::Path::new(&format!("{notebooks}/{id}")).is_dir();
    let mine_id = nb.map(|n| n.id.as_str().to_string()).unwrap_or_default();
    if nb.is_some() && good_id(&mine_id) {
        touch(&format!("{root}/{mine_id}")); // (this notebook's last use: now)
    }
    let now = std::time::SystemTime::now();
    // the other notebooks' folders (last use, bytes), and this one's records (page index, bytes, path)
    let mut others: Vec<(std::time::SystemTime, u64, std::path::PathBuf)> = Vec::new();
    let mut mine: Vec<(usize, u64, std::path::PathBuf)> = Vec::new();
    let mut total: u64 = 0;
    for e in rd.flatten() {
        let path = e.path();
        let Some(name) = e.file_name().to_str().map(|s| s.to_string()) else { continue };
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_file() {
            let old = |hours: u64| std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()).is_some_and(|t| now.duration_since(t).is_ok_and(|d| d.as_secs() > hours * 3600));
            if name.ends_with(".spool") && old(1) {
                // (a share child's spool that was left behind by a kill: gone after an hour)
                let _ = std::fs::remove_file(&path);
            } else if name.strip_suffix(".lock").is_some_and(|id| good_id(id) && gone(id)) {
                let _ = std::fs::remove_file(&path); // (a deleted notebook's mirror lock: no mirror of it can start)
            }
            continue;
        }
        if !ft.is_dir() {
            continue;
        }
        if !good_id(&name) || gone(&name) {
            let _ = std::fs::remove_dir_all(&path); // (a notebook that is gone)
            continue;
        }
        let Ok(rd2) = std::fs::read_dir(&path) else { continue };
        let is_mine = nb.is_some() && name == mine_id;
        let mut bytes = 0u64;
        for f in rd2.flatten() {
            let fp = f.path();
            let Some(fname) = f.file_name().to_str().map(|s| s.to_string()) else { continue };
            let Ok(m) = f.metadata() else { continue };
            if !m.is_file() {
                continue;
            }
            if is_mine {
                let at = fname.strip_prefix('p').and_then(|r| r.strip_suffix(".z")).and_then(|n| n.parse::<u32>().ok()).and_then(|pid| nb.and_then(|n| n.page_ids.iter().position(|&p| p == pid)));
                let Some(at) = at else {
                    let _ = std::fs::remove_file(&fp); // (a deleted page's record, or a writer's leftover)
                    continue;
                };
                mine.push((at, m.len(), fp));
            }
            bytes += m.len();
        }
        total += bytes;
        if !is_mine {
            let used = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()).unwrap_or(now);
            others.push((used, bytes, path));
        }
    }
    if total <= max {
        return;
    }
    others.sort();
    for (_, bytes, path) in others {
        if total <= max {
            return;
        }
        if std::fs::remove_dir_all(&path).is_ok() {
            total -= bytes;
        }
    }
    mine.sort();
    while total > max {
        let Some((_, len, path)) = mine.pop() else { break };
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
        }
    }
}
