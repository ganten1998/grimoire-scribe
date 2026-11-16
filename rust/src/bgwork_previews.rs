//! Files' previews (2.2, issue #145; docs/DESIGN.md, "Files: previews and the Covers view"): a small picture of a file
//! that Files' Covers view shows, made by the sandboxed worker and kept in a cache of its own.
//!
//! - What reaches the worker: only the PDFs, pictures and fonts of the screen the Covers view shows (the folder on
//!   screen, its screenful, in the listed order), each asked for once a visit. Nothing is asked for in the list, in
//!   pick mode, at Files' root or in the trash; a screen turned away from takes its queued jobs out of the queue.
//! - The request (UI thread, `preview_request`, from the Covers view): a file the cache has nothing for is queued as
//!   a job of its own (BgKind::FilePreviews, as one the user waits on; path: the file as listed,
//!   `<dev>.<ino>.<size>.<mtime>:` and its storage path, as the copy job's).
//! - The job (a background child, `bg_file_preview`): opens the file again through fsview's walk (Files' place mode:
//!   no link, one device, never the notebooks), checks it is still the file listed, and has one worker make the
//!   preview, 300 x 400 and gray: a PDF's first page fitted (pdfwork.rs `R`), a picture fitted and turned upright
//!   (picwork.rs, a pool that never decodes in the writer), a font's fixed sample (fontcheck.rs `T`). Each worker has
//!   its kind's limits and watchdog, and the writer validates every record, as the import and the font check do. A
//!   file the worker refuses, takes too long on, or ends on gets a "no preview" mark, kept, so that it is not tried at
//!   every visit. No worker to be had, no memory, a file changed or gone: nothing is written (the next visit asks).
//! - The cache: grimoire/.cache/files/<key>.pgm and <key>.info (docs/FORMAT.md), <key> the FNV-1a 64 hash of the
//!   file's storage path, its size and its time, so a changed file gets a new preview. Written atomically by the job
//!   alone (the picture first, then the .info that says it is whole), read by the UI with the plain-file read and a
//!   size limit, the .info parsed strictly and the picture held to it. The job prunes it past 2,000 files or 64 MB,
//!   the oldest first. Deleting the folder loses nothing.
//!
//! grimoire.log never has a file's name here: kinds, counts, sizes, timings and reasons.

use crate::fsview::*;
use crate::prelude::*;
use std::io::Read;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

/// A preview's size: a Scribe page's shape, at most this big (browser.md 9).
pub const PREVIEW_W: i32 = 300;
pub const PREVIEW_H: i32 = 400;
/// The cache is pruned past this many files ...
pub const CACHE_MAX_FILES: usize = 2000;
/// ... or this many bytes, the oldest previews first, down to three quarters of each.
pub const CACHE_MAX_BYTES: u64 = 64 << 20;
/// A `.info` file is at most this long.
const INFO_MAX: usize = 128;
/// A preview's picture file: its P5 header and its pixels, at most.
const PGM_MAX: usize = 32 + (PREVIEW_W * PREVIEW_H) as usize;

/// The kinds of file that have a preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewKind {
    Pdf,
    Picture,
    Font,
}

impl PreviewKind {
    /// The kind a file's name says, as Files lists it (`.pdf`; `.png`, `.jpg`, `.jpeg`, `.pgm`; `.ttf`, `.otf`), None for
    /// one that has no preview.
    pub fn of_name(name: &[u8]) -> Option<PreviewKind> {
        if crate::browser::is_pdf(name) {
            Some(PreviewKind::Pdf)
        } else if crate::browser::is_picture(name) {
            Some(PreviewKind::Picture)
        } else if crate::browser::is_font(name) {
            Some(PreviewKind::Font)
        } else {
            None
        }
    }
    fn word(self) -> &'static str {
        match self {
            PreviewKind::Pdf => "pdf",
            PreviewKind::Picture => "picture",
            PreviewKind::Font => "font",
        }
    }
    fn from_word(w: &str) -> Option<PreviewKind> {
        [PreviewKind::Pdf, PreviewKind::Picture, PreviewKind::Font].into_iter().find(|k| k.word() == w)
    }
}

/// Why a file has no preview (its "no preview" mark): the worker refused it (no file of its kind Grimoire reads, or
/// too big for its worker), took too long on it, or ended on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoPreview {
    Refused,
    Slow,
    Ended,
}

impl NoPreview {
    fn word(self) -> &'static str {
        match self {
            NoPreview::Refused => "refused",
            NoPreview::Slow => "slow",
            NoPreview::Ended => "ended",
        }
    }
    fn from_word(w: &str) -> Option<NoPreview> {
        [NoPreview::Refused, NoPreview::Slow, NoPreview::Ended].into_iter().find(|n| n.word() == w)
    }
}

/// What a `.info` says: the file it is of (its size and time), its kind, and the preview's size or why there is none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewInfo {
    pub size: u64,
    pub mtime: i64,
    pub kind: PreviewKind,
    pub made: Result<(i32, i32), NoPreview>,
}

impl PreviewInfo {
    /// The file's text: four lines (docs/FORMAT.md).
    pub fn to_text(&self) -> String {
        let last = match self.made {
            Ok((w, h)) => format!("made {w} {h}"),
            Err(n) => format!("none {}", n.word()),
        };
        format!("grimoire-preview 1\nof {} {}\nkind {}\n{last}\n", self.size, self.mtime, self.kind.word())
    }

    /// The text read strictly: at most INFO_MAX bytes of printable ASCII, exactly the four lines in their order, each
    /// ended by a line break, single spaces, decimal numbers without a sign (but a time's minus) or a leading zero, a
    /// preview at most PREVIEW_W x PREVIEW_H. Anything else is None (the entry is made again).
    pub fn parse(text: &[u8]) -> Option<PreviewInfo> {
        if text.len() > INFO_MAX || !text.iter().all(|&c| c == b'\n' || (0x20..0x7f).contains(&c)) {
            return None;
        }
        let s = std::str::from_utf8(text).ok()?.strip_suffix('\n')?;
        let lines: Vec<&str> = s.split('\n').collect();
        let [head, of, kind, last] = lines[..] else { return None };
        if head != "grimoire-preview 1" {
            return None;
        }
        let of: Vec<&str> = of.split(' ').collect();
        let ["of", size, mtime] = of[..] else { return None };
        let (size, mtime) = (num_u64(size)?, num_i64(mtime)?);
        let kind = PreviewKind::from_word(kind.strip_prefix("kind ")?)?;
        let t: Vec<&str> = last.split(' ').collect();
        let made = match t[..] {
            ["made", w, h] => {
                let (w, h) = (num_u64(w)?, num_u64(h)?);
                if !(1..=PREVIEW_W as u64).contains(&w) || !(1..=PREVIEW_H as u64).contains(&h) {
                    return None;
                }
                Ok((w as i32, h as i32))
            }
            ["none", why] => Err(NoPreview::from_word(why)?),
            _ => return None,
        };
        Some(PreviewInfo { size, mtime, kind, made })
    }
}

/// A decimal number: digits only, no sign, no leading zero, at most 19 of them.
fn num_u64(t: &str) -> Option<u64> {
    if t.is_empty() || t.len() > 19 || !t.bytes().all(|c| c.is_ascii_digit()) || (t.len() > 1 && t.starts_with('0')) {
        return None;
    }
    t.parse().ok()
}

/// A time: num_u64's form, or a minus and a number other than 0.
fn num_i64(t: &str) -> Option<i64> {
    match t.strip_prefix('-') {
        Some(r) => i64::try_from(num_u64(r).filter(|&v| v != 0)?).ok().map(|v| -v),
        None => i64::try_from(num_u64(t)?).ok(),
    }
}

/// A file's cache key: the FNV-1a 64 hash of its storage path, then `/<size>/<mtime>` (as a template's preview is
/// keyed by its name, size and time), so a changed or replaced file has another.
pub fn preview_key(path: &[u8], size: u64, mtime: i64) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in path.iter().copied().chain(format!("/{size}/{mtime}").bytes()) {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    h
}

fn cache_dir() -> String {
    format!("{}/.cache/files", store_root())
}

/// The cache file of `key` with this extension (`pgm`, `info`).
pub fn preview_file(key: u64, ext: &str) -> String {
    format!("{}/{key:016x}.{ext}", cache_dir())
}

/// What the cache holds for a file as listed.
pub enum Cached {
    /// nothing (not made yet, or an entry that does not read: it is made again)
    Nothing,
    /// its "no preview" mark
    NoPreview,
    /// its preview, decoded
    Preview(Canvas),
}

/// The `.info` of the file at `path` as listed (`st`): read with the plain-file read (at most INFO_MAX bytes), parsed
/// strictly, and only when it is of that file (its size, time and kind).
pub fn preview_info(path: &StoragePath, st: &FsStat) -> Option<PreviewInfo> {
    let kind = PreviewKind::of_name(path.split_last()?.1)?;
    let key = preview_key(path.as_bytes(), st.size, st.mtime);
    let info = PreviewInfo::parse(&read_file_max(&preview_file(key, "info"), INFO_MAX)?)?;
    (info.size == st.size && info.mtime == st.mtime && info.kind == kind).then_some(info)
}

/// What the cache holds for the file at `path` as listed (`st`), for the UI: its `.info`, then its picture, read with
/// the plain-file read and a size limit, the picture's header and length exactly what the `.info` says.
pub fn preview_cached(path: &StoragePath, st: &FsStat) -> Cached {
    let Some(info) = preview_info(path, st) else { return Cached::Nothing };
    let Ok((w, h)) = info.made else { return Cached::NoPreview };
    let key = preview_key(path.as_bytes(), st.size, st.mtime);
    let Some(d) = read_file_max(&preview_file(key, "pgm"), PGM_MAX) else { return Cached::Nothing };
    let head = format!("P5\n{w} {h}\n255\n");
    let n = (w * h) as usize;
    if d.len() != head.len() + n || !d.starts_with(head.as_bytes()) {
        return Cached::Nothing;
    }
    let mut c = Canvas::default();
    if canvas_init(&mut c, w, h) != 0 {
        return Cached::Nothing;
    }
    for y in 0..h as usize {
        let at = y * c.stride as usize;
        c.px[at..at + w as usize].copy_from_slice(&d[head.len() + y * w as usize..head.len() + (y + 1) * w as usize]);
    }
    Cached::Preview(c)
}

// ---- the job ----

/// Asks for the preview of the file at `path` as listed (`st`): one job, queued as one the user waits on (unless that
/// very job is queued or running). Its job's path, or None for a file of no kind with a preview, or whose path does
/// not fit a job.
pub fn preview_request(path: &StoragePath, st: &FsStat) -> Option<String> {
    PreviewKind::of_name(path.split_last()?.1)?;
    let jp = crate::bgwork_copy::job_path(path, st)?;
    // (one queued or running already is the one: a queued duplicate would be merged, a running one would be run twice)
    if !bg_job_pending(BgKind::FilePreviews, "", &jp) {
        bg_queue(BgKind::FilePreviews, None, Some(&jp), 0.0, true);
    }
    Some(jp)
}

/// How a file's preview ended: made, marked as having none, or already there.
enum Outcome {
    Made,
    Marked(NoPreview),
    There,
}

/// Why no preview came from a worker: one the file earns a mark for, or one that is not the file's (nothing written).
enum Fail {
    Mark(NoPreview),
    Later(&'static str),
}

/// BgKind::FilePreviews, in the background child: the preview of the file `path` names (the job's path), or its "no
/// preview" mark, in the cache; then the cache's prune. 0 when the cache holds one or the other for it now, -1 when
/// nothing was written (the log says why).
pub fn bg_file_preview(path: &str) -> i32 {
    let t0 = real_now_ms();
    let r = make(path);
    let ms = real_now_ms().wrapping_sub(t0);
    let rc = match &r {
        Ok((kind, Outcome::Made)) => {
            qlog!("file previews: a %s made in %u ms (the worker's peak %d kB, this job's %d kB)", kind.word(), ms, workers_peak_kb(), own_peak_kb());
            0
        }
        Ok((kind, Outcome::Marked(why))) => {
            qlog!("file previews: a %s has no preview (%s), marked, in %u ms", kind.word(), why.word(), ms);
            0
        }
        Ok((_, Outcome::There)) => 0,
        Err(why) => {
            qlog!("file previews: nothing made (%s)", why.as_str());
            -1
        }
    };
    if matches!(r, Ok((_, Outcome::Made | Outcome::Marked(_)))) {
        prune(&cache_dir());
    }
    rc
}

fn walk_why(e: FsErr) -> String {
    match e {
        FsErr::Link => "a link on its way".into(),
        FsErr::OtherDevice => "another device".into(),
        FsErr::Refused => "a place Files does not read".into(),
        FsErr::NotFound => "it is not there".into(),
        FsErr::NotDir => "a folder on its way is not one".into(),
        FsErr::NotFile => "it is no plain file".into(),
        FsErr::Io(n) => strerror(n),
    }
}

fn fd_stat(fd: &OwnedFd) -> Option<FsStat> {
    // SAFETY: md is written by fstat before it is read; the descriptor is open.
    let mut md: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd.as_raw_fd(), &mut md) } != 0 {
        return None;
    }
    Some(FsStat { dev: md.st_dev as u64, ino: md.st_ino as u64, size: md.st_size as u64, mtime: md.st_mtime as i64, mode: md.st_mode as u32 })
}

fn make(path: &str) -> Result<(PreviewKind, Outcome), String> {
    let (listed, src) = crate::bgwork_copy::job_parse(path).ok_or("a request that does not read")?;
    let kind = src.split_last().and_then(|(_, n)| PreviewKind::of_name(n)).ok_or("a kind with no preview")?;
    if preview_info(&src, &listed).is_some() {
        return Ok((kind, Outcome::There)); // (asked twice, and made by the first)
    }
    let dir = cache_dir();
    // (written as root: never through a planted link, and the folder made where it is missing)
    if !no_link_below(&store_root(), &dir) || mkdir_p(&dir) != 0 {
        return Err("the cache folder cannot be made".into());
    }
    let root = crate::pdfimport::storage_root().ok_or("no storage")?;
    let st = FsStorage::open(&root, &store_root()).map_err(walk_why)?;
    // (place mode, as Files lists it: Grimoire's PDF copies and trash are read, its notebooks never)
    let fd = st.open_file(&src, false).map_err(walk_why)?;
    let at = fd_stat(&fd).ok_or("it does not stat")?;
    if !at.same(&listed) {
        return Err("it changed since it was listed".into());
    }
    let r = match kind {
        PreviewKind::Pdf => pdf_preview(&fd, at.size),
        PreviewKind::Picture => read_whole(&fd, at.size, BG_SOURCE_MAX_BYTES as u64, &listed).and_then(|d| picture_preview(&d)),
        PreviewKind::Font => read_whole(&fd, at.size, crate::fontcheck::FONT_FILE_MAX as u64, &listed).and_then(|d| font_preview(&d)),
    };
    let key = preview_key(src.as_bytes(), listed.size, listed.mtime);
    let info = |made| PreviewInfo { size: listed.size, mtime: listed.mtime, kind, made };
    match r {
        Ok(c) => {
            // (the picture first: a .info that says "made" has one beside it, whole)
            let ok = store_write_pgm(&preview_file(key, "pgm"), &c) == 0 && write_file_atomic(&preview_file(key, "info"), info(Ok((c.w, c.h))).to_text().as_bytes()) == 0;
            if !ok {
                return Err("the cache could not be written".into());
            }
            Ok((kind, Outcome::Made))
        }
        Err(Fail::Mark(why)) => {
            let _ = std::fs::remove_file(preview_file(key, "pgm"));
            if write_file_atomic(&preview_file(key, "info"), info(Err(why)).to_text().as_bytes()) != 0 {
                return Err("the cache could not be written".into());
            }
            Ok((kind, Outcome::Marked(why)))
        }
        Err(Fail::Later(why)) => Err(why.into()),
    }
}

/// The whole file, at most `max` bytes (a bigger one has no preview: its worker would not take it), still the file
/// listed when read.
fn read_whole(fd: &OwnedFd, size: u64, max: u64, listed: &FsStat) -> Result<Vec<u8>, Fail> {
    if size == 0 || size > max {
        return Err(Fail::Mark(NoPreview::Refused));
    }
    let mut f = std::fs::File::from(fd.try_clone().map_err(|_| Fail::Later("the file did not read"))?);
    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve_exact(size as usize + 1).is_err() {
        return Err(Fail::Later("out of memory"));
    }
    if f.by_ref().take(size + 1).read_to_end(&mut buf).is_err() || buf.len() as u64 != size {
        return Err(Fail::Later("the file did not read whole"));
    }
    if !fd_stat(fd).is_some_and(|now| now.same(listed)) {
        return Err(Fail::Later("it changed while it was read"));
    }
    Ok(buf)
}

/// A gray canvas of a worker's pixels (`ch` 1 or 3, rows of w * ch bytes).
fn gray_of(px: &[u8], w: i32, h: i32, ch: i32) -> Result<Canvas, Fail> {
    let mut c = Canvas::default();
    if canvas_init(&mut c, w, h) != 0 {
        return Err(Fail::Later("out of memory"));
    }
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = (y * w as usize + x) * ch as usize;
            c.px[y * c.stride as usize + x] = if ch == 3 { crate::pdfwork::luma(px[i], px[i + 1], px[i + 2]) } else { px[i] };
        }
    }
    Ok(c)
}

/// A PDF's first page fitted onto PREVIEW_W x PREVIEW_H, by one worker with the import's limits (the PDF mapped where
/// the storage allows it, else read whole under its cap) and its watchdog.
fn pdf_preview(fd: &OwnedFd, size: u64) -> Result<Canvas, Fail> {
    use crate::pdfwork::*;
    use crate::sandbox::*;
    let raw = fd.as_raw_fd();
    let mapped = crate::pdfimport::can_map(raw, size);
    if size == 0 || size > if mapped { s1::PDF_MAX_MAPPED } else { s1::PDF_MAX_READ_WHOLE } {
        return Err(Fail::Mark(NoPreview::Refused));
    }
    let limits = Limits { as_bytes: worker_as_bytes(size, false), cpu_secs: s1::SLICE_CPU_SECS, mapped, file_len: size };
    let start_err = |e: StartErr| match e {
        StartErr::Sandbox(_) | StartErr::Worker(_) => Fail::Later("no sandboxed worker can be had"),
        StartErr::Password | StartErr::Invalid | StartErr::Empty => Fail::Mark(NoPreview::Refused),
        StartErr::Failed(_) => Fail::Mark(NoPreview::Ended),
    };
    let mut s = Session::start(raw, size, &limits).map_err(start_err)?;
    s.probe().map_err(start_err)?;
    match s.render(1, PREVIEW_W, PREVIEW_H, 1) {
        PageOutcome::Page(p) => {
            s.end();
            gray_of(&p.px, p.w, p.h, p.ch)
        }
        PageOutcome::Died(Death::Timeout) => Err(Fail::Mark(NoPreview::Slow)),
        // (the Kindle slept while the page was waited for: not the file's fault)
        PageOutcome::Died(Death::TimeoutAfterWake) => Err(Fail::Later("the Kindle slept while it was made")),
        PageOutcome::Died(_) | PageOutcome::Lied(_) => Err(Fail::Mark(NoPreview::Ended)),
        PageOutcome::Failed(_) => Err(Fail::Mark(NoPreview::Refused)),
        PageOutcome::NoMemory => Err(Fail::Later("out of memory")),
    }
}

/// A picture fitted onto PREVIEW_W x PREVIEW_H and turned upright, by the picture worker (never in the writer).
fn picture_preview(data: &[u8]) -> Result<Canvas, Fail> {
    use crate::picwork::*;
    let mut pool = PicPool::worker_only();
    match pool.decode(data, true, PREVIEW_W, PREVIEW_H) {
        PicOut::Pic { fit, .. } => {
            let mut c = Canvas::default();
            if bgimage_luma(&fit, &mut c) != 0 {
                return Err(Fail::Later("out of memory"));
            }
            Ok(c)
        }
        PicOut::Bad(_) => Err(Fail::Mark(NoPreview::Refused)),
        PicOut::Dead("the worker stalled") => Err(Fail::Mark(NoPreview::Slow)),
        PicOut::Dead(_) => Err(Fail::Mark(NoPreview::Ended)),
        PicOut::Retry(_) | PicOut::Info { .. } => Err(Fail::Later("out of memory")),
        PicOut::NoWorker(_) => Err(Fail::Later("no sandboxed worker can be had")),
    }
}

/// A font's fixed sample on PREVIEW_W x PREVIEW_H, by the font worker.
fn font_preview(data: &[u8]) -> Result<Canvas, Fail> {
    use crate::fontcheck::*;
    match font_sample(data, PREVIEW_W, PREVIEW_H) {
        Ok(px) => gray_of(&px, PREVIEW_W, PREVIEW_H, 1),
        Err(FontOut::Refused(_)) => Err(Fail::Mark(NoPreview::Refused)),
        Err(FontOut::Slow) => Err(Fail::Mark(NoPreview::Slow)),
        Err(FontOut::Dead(_)) => Err(Fail::Mark(NoPreview::Ended)),
        Err(FontOut::NoWorker(_)) => Err(Fail::Later("no sandboxed worker can be had")),
        Err(_) => Err(Fail::Later("out of memory")),
    }
}

/// The largest peak resident memory of this process's reaped children (the worker), in kB.
fn workers_peak_kb() -> i32 {
    // SAFETY: ru is written by getrusage before it is read.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut ru) };
    // (Linux counts kB; macOS bytes)
    (if cfg!(target_os = "linux") { ru.ru_maxrss } else { ru.ru_maxrss / 1024 }) as i32
}

/// This process's own peak resident memory, in kB.
fn own_peak_kb() -> i32 {
    // SAFETY: as workers_peak_kb.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    (if cfg!(target_os = "linux") { ru.ru_maxrss } else { ru.ru_maxrss / 1024 }) as i32
}

// ---- the prune ----

/// One preview's files in the cache: its key (the names' stem), how many files and bytes, and the newest time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheEntry {
    pub stem: String,
    pub files: usize,
    pub bytes: u64,
    pub mtime: i64,
}

/// The stems to remove so that the cache holds at most `max_files` files and `max_bytes` bytes: none while it is
/// within both, else the oldest first (ties by stem) until it is within three quarters of each, so the jobs after a
/// prune do not prune again at once.
pub fn prune_plan(entries: &[CacheEntry], max_files: usize, max_bytes: u64) -> Vec<String> {
    let (mut files, mut bytes) = (entries.iter().map(|e| e.files).sum::<usize>(), entries.iter().map(|e| e.bytes).sum::<u64>());
    if files <= max_files && bytes <= max_bytes {
        return Vec::new();
    }
    let (to_files, to_bytes) = (max_files / 4 * 3, max_bytes / 4 * 3);
    let mut order: Vec<&CacheEntry> = entries.iter().collect();
    order.sort_by(|a, b| a.mtime.cmp(&b.mtime).then_with(|| a.stem.cmp(&b.stem)));
    let mut out = Vec::new();
    for e in order {
        if files <= to_files && bytes <= to_bytes {
            break;
        }
        out.push(e.stem.clone());
        files -= e.files;
        bytes -= e.bytes;
    }
    out
}

/// The cache's limits: CACHE_MAX_FILES and CACHE_MAX_BYTES, or (tests, never the Kindle's build)
/// GRIMOIRE_PREVIEWS_MAX as `<files>,<bytes>`.
fn prune_limits() -> (usize, u64) {
    #[cfg(not(feature = "kindle"))]
    if let Some((f, b)) = std::env::var("GRIMOIRE_PREVIEWS_MAX").ok().as_deref().and_then(|v| v.split_once(',')).and_then(|(f, b)| Some((f.parse().ok()?, b.parse().ok()?))) {
        return (f, b);
    }
    (CACHE_MAX_FILES, CACHE_MAX_BYTES)
}

/// The prune, in the job's child: the names alone first (no stat), and while they are within both limits by the most
/// their files can hold, nothing more is read; else every file's size and time, and prune_plan's stems go, each with
/// all of its files. Entries that are no plain file are left to themselves (never followed).
fn prune(dir: &str) {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    let (max_files, max_bytes) = prune_limits();
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let names: Vec<std::ffi::OsString> = rd.flatten().map(|e| e.file_name()).filter(|n| n.as_bytes().first() != Some(&b'.')).collect();
    let most: u64 = names.iter().map(|n| if n.as_bytes().ends_with(b".info") { INFO_MAX as u64 } else { PGM_MAX as u64 }).sum();
    if names.len() <= max_files && most <= max_bytes {
        return;
    }
    let mut entries: Vec<CacheEntry> = Vec::new();
    for n in &names {
        let Ok(m) = std::fs::symlink_metadata(Path::new(dir).join(n)) else { continue };
        if !m.is_file() {
            continue;
        }
        let stem = String::from_utf8_lossy(n.as_bytes().split(|&c| c == b'.').next().unwrap_or(&[])).into_owned();
        match entries.iter_mut().find(|e| e.stem == stem) {
            Some(e) => {
                e.files += 1;
                e.bytes += m.len();
                e.mtime = e.mtime.max(m.mtime());
            }
            None => entries.push(CacheEntry { stem, files: 1, bytes: m.len(), mtime: m.mtime() }),
        }
    }
    let gone = prune_plan(&entries, max_files, max_bytes);
    if gone.is_empty() {
        return;
    }
    let (mut files, mut bytes) = (0usize, 0u64);
    for n in &names {
        let stem = n.as_bytes().split(|&c| c == b'.').next().unwrap_or(&[]);
        if gone.iter().any(|g| g.as_bytes() == stem) {
            let p = Path::new(dir).join(n);
            let len = std::fs::symlink_metadata(&p).map_or(0, |m| m.len());
            if std::fs::remove_file(&p).is_ok() {
                files += 1;
                bytes += len;
            }
        }
    }
    qlog!("file previews: the cache pruned: %d file(s) (%d kB) removed of %d", files as i32, (bytes >> 10) as i32, names.len() as i32);
}

#[cfg(test)]
mod tests {
    use super::*;

    // The key is the path's, the size's and the time's: any one of them changed is another key; the same three are
    // the same key, whatever else differs.
    #[test]
    fn the_key_is_the_path_size_and_time() {
        let k = preview_key(b"documents/a.pdf", 1000, 1_700_000_000);
        assert_eq!(k, preview_key(b"documents/a.pdf", 1000, 1_700_000_000));
        assert_ne!(k, preview_key(b"documents/b.pdf", 1000, 1_700_000_000));
        assert_ne!(k, preview_key(b"documents/a.pdf", 1001, 1_700_000_000));
        assert_ne!(k, preview_key(b"documents/a.pdf", 1000, 1_700_000_001));
        assert_ne!(k, preview_key(b"Documents/a.pdf", 1000, 1_700_000_000));
        // (FNV-1a 64 of the bytes "x/1/2", by hand: the form is the path, then /size/mtime)
        let mut h: u64 = 14695981039346656037;
        for b in b"x/1/2" {
            h = (h ^ *b as u64).wrapping_mul(1099511628211);
        }
        assert_eq!(preview_key(b"x", 1, 2), h);
        assert!(preview_file(k, "pgm").ends_with(&format!("/.cache/files/{k:016x}.pgm")));
    }

    fn info(made: Result<(i32, i32), NoPreview>) -> PreviewInfo {
        PreviewInfo { size: 1234, mtime: 1_700_000_000, kind: PreviewKind::Pdf, made }
    }

    // A .info is read back as written, and anything else is refused: each line's word and order, the numbers' form,
    // the preview's bounds, the length, a byte outside printable ASCII, a missing or extra line break.
    #[test]
    fn the_info_is_parsed_strictly() {
        for i in [info(Ok((300, 400))), info(Ok((1, 1))), info(Err(NoPreview::Refused)), info(Err(NoPreview::Slow)), info(Err(NoPreview::Ended))] {
            assert_eq!(PreviewInfo::parse(i.to_text().as_bytes()), Some(i));
        }
        let neg = PreviewInfo { mtime: -5, kind: PreviewKind::Font, ..info(Ok((300, 400))) };
        assert_eq!(PreviewInfo::parse(neg.to_text().as_bytes()), Some(neg));
        let good = "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\n";
        assert!(PreviewInfo::parse(good.as_bytes()).is_some());
        for bad in [
            "",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\n\n",
            "grimoire-preview 2\nof 1234 1700000000\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nkind picture\nof 1234 1700000000\nmade 300 400\n",
            "grimoire-preview 1\nof 01234 1700000000\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nof +1234 1700000000\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nof 1234 -0\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nof 1234  1700000000\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind photo\nmade 300 400\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 301 400\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 401\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 0 400\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nnone tired\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nnone\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\nmore\n",
            "grimoire-preview 1\r\nof 1234 1700000000\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nof 99999999999999999999 1700000000\nkind picture\nmade 300 400\n",
        ] {
            assert_eq!(PreviewInfo::parse(bad.as_bytes()), None, "{bad:?}");
        }
        let mut long = good.to_string();
        long.insert_str(good.len() - 1, &" ".repeat(INFO_MAX));
        assert_eq!(PreviewInfo::parse(long.as_bytes()), None);
        assert_eq!(PreviewInfo::parse("grimoire-preview 1\nof 1 2\nkind pdf\nmade 3 4\u{e9}\n".as_bytes()), None);
    }

    fn entry(stem: &str, files: usize, bytes: u64, mtime: i64) -> CacheEntry {
        CacheEntry { stem: stem.to_string(), files, bytes, mtime }
    }

    // Within both limits nothing goes; past either, the oldest go first until the cache is within three quarters of
    // each; ties of time by the stem.
    #[test]
    fn the_prune_takes_the_oldest_down_to_three_quarters() {
        let e = [entry("c", 2, 100, 30), entry("a", 2, 100, 10), entry("b", 2, 100, 20), entry("d", 2, 100, 40)];
        assert!(prune_plan(&e, 8, 400).is_empty());
        // past the files (10 > 8): down to 6 files
        let mut more = e.to_vec();
        more.push(entry("e", 2, 100, 50));
        assert_eq!(prune_plan(&more, 8, 10_000), ["a", "b"]);
        // past the bytes (500 > 400): down to 300 bytes
        assert_eq!(prune_plan(&more, 100, 400), ["a", "b"]);
        // past both
        assert_eq!(prune_plan(&more, 4, 200), ["a", "b", "c", "d"]);
        // a tie of time: by the stem (1,200 bytes past 1,000: one goes, down to 600)
        let tie = [entry("y", 1, 600, 5), entry("x", 1, 600, 5)];
        assert_eq!(prune_plan(&tie, 100, 1000), ["x"]);
        // the real limits: 2,001 files of a byte each go down to 1,500
        let many: Vec<CacheEntry> = (0..2001).map(|i| entry(&format!("{i:016x}"), 1, 1, i)).collect();
        let gone = prune_plan(&many, CACHE_MAX_FILES, CACHE_MAX_BYTES);
        assert_eq!(gone.len(), 501);
        assert_eq!(gone[0], format!("{:016x}", 0));
    }
}
