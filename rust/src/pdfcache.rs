//! The notebook PDF's page streams, kept so that a changed notebook renders only its changed pages
//! (docs/FORMAT.md, "grimoire/.cache/pdf/"). Each page's compressed image stream lives in
//! `grimoire/.cache/pdf/<notebook id>/p<page id>.z` with the stamp it was made under: the page file's
//! identity (size and modification time), the renderer's version, the dpi, the notebook's page size, and
//! the channel count of the page's picture. A record that does not read, whose checksum is wrong, or
//! whose stamp is not the page's today is never used: the page is drawn again (full render on any doubt).
//!
//! Every cache is disposable: deleting this folder loses nothing. Only background jobs write it (the
//! mirror's child); the share server's render children read it and never write. Writes are best
//! effort: the first one that fails (a full disk) turns the cache off for that run, and the PDF is
//! made as if there were none.

use crate::prelude::*;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// 256 MB in all: past it the oldest records go.
pub const PDF_CACHE_MAX: u64 = 256 << 20;
/// Bumped whenever a change to drawing would make an old record wrong.
const PDF_RENDER_VER: u32 = 1;
/// A record's header, in bytes.
const HEAD: usize = 80;
/// The most a record's stream may be (a page is one image of at most 6000 x 6000 pixels).
const Z_MAX: usize = 128 << 20;

/// Pages the PDF writer has drawn (not found in the cache) in this process: tests count them.
static RENDERED: AtomicU32 = AtomicU32::new(0);
/// Tests: make every cache write fail as a full disk does.
static FULL: AtomicBool = AtomicBool::new(false);

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

/// What a record was made under, besides the picture's channels (checked apart: the page's paper is
/// in the record).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Stamp {
    render: u32,
    version: u64,
    dpi: u32,
    size: u64,
    sec: i64,
    nsec: u32,
    nbw: i32,
    nbh: i32,
}

/// One page's image, as it goes into the PDF.
pub struct PageImage {
    /// pixels
    pub w: i32,
    pub h: i32,
    /// 1 (DeviceGray) or 3 (DeviceRGB)
    pub ch: u8,
    /// the zlib stream of the rows
    pub z: Vec<u8>,
}

/// A record read back: the image, and what it was made from.
struct Record {
    stamp: Stamp,
    paper: u32,
    key_ch: u32,
    img: PageImage,
}

pub struct PdfCache {
    dir: String,
    write: bool,
    /// a write failed: no more are tried in this run
    off: bool,
    stamp: Stamp,
    /// picture channels by paper id (one header read a picture in a run)
    chans: std::collections::HashMap<u32, u32>,
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

fn page_file(nb: &NotebookInfo, pid: u32) -> String {
    let mut dir = String::new();
    store_notebook_dir(nb.id.as_str(), &mut dir, 700);
    format!("{dir}/p{pid}.qpg")
}

impl PdfCache {
    /// None for Off, and for a notebook whose folder is no good name.
    pub fn open(nb: &NotebookInfo, dpi: f32, mode: PdfCacheMode) -> Option<PdfCache> {
        if mode == PdfCacheMode::Off || nb.id.is_empty() || nb.id.as_str().contains(['/', '\\']) || nb.id.as_str().starts_with('.') {
            return None;
        }
        let (mut w, mut h) = (0, 0);
        store_notebook_page_size(nb, &mut w, &mut h);
        Some(PdfCache {
            dir: format!("{}/{}", pdf_cache_root(), nb.id.as_str()),
            write: mode == PdfCacheMode::ReadWrite,
            off: false,
            stamp: Stamp { render: PDF_RENDER_VER, version: fnv(GRIMOIRE_VERSION), dpi: dpi.to_bits(), nbw: w, nbh: h, ..Stamp::default() },
            chans: std::collections::HashMap::new(),
        })
    }

    fn stamp_of(&self, nb: &NotebookInfo, pid: u32) -> Stamp {
        use std::os::unix::fs::MetadataExt;
        let mut s = self.stamp;
        if let Ok(m) = std::fs::metadata(page_file(nb, pid)) {
            s.size = m.len();
            s.sec = m.mtime();
            s.nsec = m.mtime_nsec() as u32;
        }
        s
    }

    fn channels(&mut self, nb: &NotebookInfo, paper: u32) -> u32 {
        *self.chans.entry(paper).or_insert_with(|| bgimage_channels(nb.id.as_str(), PaperKind(paper)) as u32)
    }

    fn record_path(&self, pid: u32) -> String {
        format!("{}/p{}.z", self.dir, pid)
    }

    /// The page's image when a record of it was made from the page as it is now.
    pub fn get(&mut self, nb: &NotebookInfo, pid: u32) -> Option<PageImage> {
        let data = std::fs::read(self.record_path(pid)).ok()?;
        let rec = parse(&data)?;
        if rec.stamp != self.stamp_of(nb, pid) || rec.key_ch != self.channels(nb, rec.paper) {
            return None;
        }
        Some(rec.img)
    }

    /// Keeps the image just drawn for the page (best effort). `paper` is the page's paper kind.
    pub fn put(&mut self, nb: &NotebookInfo, pid: u32, paper: u32, img: &PageImage) {
        if !self.write || self.off {
            return;
        }
        let rec = Record { stamp: self.stamp_of(nb, pid), paper, key_ch: self.channels(nb, paper), img: PageImage { w: img.w, h: img.h, ch: img.ch, z: Vec::new() } };
        let head = serialize_head(&rec, img.z.len(), crc32(&img.z));
        if !self.write_record(pid, &head, &img.z) {
            self.off = true;
            qlog!("pdf cache: a write failed; this run goes on without it");
        }
    }

    fn write_record(&self, pid: u32, head: &[u8], z: &[u8]) -> bool {
        if FULL.load(Ordering::Relaxed) || mkdir_p(&self.dir) != 0 {
            return false;
        }
        let path = self.record_path(pid);
        let tmp = format!("{path}.tmp");
        let ok = (|| -> std::io::Result<()> {
            let mut f = std::fs::File::create(&tmp)?;
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

fn serialize_head(r: &Record, zlen: usize, crc: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(HEAD);
    v.extend_from_slice(b"GPZ1");
    v.extend_from_slice(&r.stamp.render.to_le_bytes());
    v.extend_from_slice(&r.stamp.version.to_le_bytes());
    v.extend_from_slice(&r.stamp.dpi.to_le_bytes());
    v.extend_from_slice(&r.stamp.size.to_le_bytes());
    v.extend_from_slice(&r.stamp.sec.to_le_bytes());
    v.extend_from_slice(&r.stamp.nsec.to_le_bytes());
    v.extend_from_slice(&r.stamp.nbw.to_le_bytes());
    v.extend_from_slice(&r.stamp.nbh.to_le_bytes());
    v.extend_from_slice(&r.paper.to_le_bytes());
    v.extend_from_slice(&r.key_ch.to_le_bytes());
    v.extend_from_slice(&((r.img.ch as u32) << 24).to_le_bytes());
    v.extend_from_slice(&r.img.w.to_le_bytes());
    v.extend_from_slice(&r.img.h.to_le_bytes());
    v.extend_from_slice(&(zlen as u32).to_le_bytes());
    v.extend_from_slice(&crc.to_le_bytes());
    while v.len() < HEAD {
        v.push(0);
    }
    v
}

fn parse(d: &[u8]) -> Option<Record> {
    if d.len() < HEAD || &d[..4] != b"GPZ1" {
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
    let stamp = Stamp {
        render: u32_(take(4)),
        version: u64_(take(8)),
        dpi: u32_(take(4)),
        size: u64_(take(8)),
        sec: u64_(take(8)) as i64,
        nsec: u32_(take(4)),
        nbw: u32_(take(4)) as i32,
        nbh: u32_(take(4)) as i32,
    };
    let paper = u32_(take(4));
    let key_ch = u32_(take(4));
    let ch = (u32_(take(4)) >> 24) as u8;
    let w = u32_(take(4)) as i32;
    let h = u32_(take(4)) as i32;
    let zlen = u32_(take(4)) as usize;
    let crc = u32_(take(4));
    if (ch != 1 && ch != 3) || w < 1 || h < 1 || zlen > Z_MAX || d.len() != HEAD + zlen {
        return None;
    }
    let z = &d[HEAD..];
    if crc32(z) != crc {
        return None;
    }
    Some(Record { stamp, paper, key_ch, img: PageImage { w, h, ch, z: z.to_vec() } })
}

/// The page ids a notebook's records may name, and the records of every other: after a mirror, the
/// pages deleted from the notebook, the notebooks deleted, and then the oldest records while the
/// folder holds more than PDF_CACHE_MAX. Best effort; never touches anything outside the cache.
pub fn pdf_cache_prune(nb: &NotebookInfo) {
    prune_in(&pdf_cache_root(), nb, PDF_CACHE_MAX);
}

pub(crate) fn prune_in(root: &str, nb: &NotebookInfo, max: u64) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    let notebooks = format!("{}/notebooks", store_root());
    let mut files: Vec<(std::time::SystemTime, u64, std::path::PathBuf)> = Vec::new();
    let now = std::time::SystemTime::now();
    for e in rd.flatten() {
        let path = e.path();
        let Some(name) = e.file_name().to_str().map(|s| s.to_string()) else { continue };
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_file() {
            // (a share child's spool that was left behind by a kill: gone after an hour)
            if name.ends_with(".spool") && std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()).is_some_and(|t| now.duration_since(t).is_ok_and(|d| d.as_secs() > 3600)) {
                let _ = std::fs::remove_file(&path);
            }
            continue;
        }
        if !ft.is_dir() {
            continue;
        }
        if !std::path::Path::new(&format!("{notebooks}/{name}")).is_dir() {
            let _ = std::fs::remove_dir_all(&path); // (a notebook that is gone)
            continue;
        }
        let mine = name == nb.id.as_str();
        let Ok(rd2) = std::fs::read_dir(&path) else { continue };
        for f in rd2.flatten() {
            let fp = f.path();
            let Some(fname) = f.file_name().to_str().map(|s| s.to_string()) else { continue };
            let live = mine.then(|| fname.strip_prefix('p').and_then(|r| r.strip_suffix(".z")).and_then(|n| n.parse::<u32>().ok()).is_some_and(|pid| nb.page_ids.contains(&pid)));
            if live == Some(false) {
                let _ = std::fs::remove_file(&fp);
                continue;
            }
            if let Ok(m) = f.metadata() {
                if m.is_file() {
                    files.push((m.modified().unwrap_or(now), m.len(), fp));
                }
            }
        }
    }
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max {
        return;
    }
    files.sort();
    for (_, len, path) in files {
        if total <= max {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
        }
    }
}
