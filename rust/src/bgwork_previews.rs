//! Files' previews (2.2, issue #145; docs/DESIGN.md, "Files: previews and the Covers view"): a small picture of a file
//! that Files' Covers view shows, made by the sandboxed worker and kept in a cache of its own.
//!
//! - What reaches the worker: only the PDFs, pictures and fonts of the screen the Covers view shows (the folder on
//!   screen, its screenful, in the listed order). Nothing is asked for in the list, in pick mode, at Files' root or in
//!   the trash; a screen turned away from, another folder, the list and Files closed take the jobs still queued out
//!   of the queue (the one running runs on: it is short, PREVIEW_TILES_MAX below).
//! - The request (UI thread, `preview_request`, from the Covers view): a file the cache has nothing for is queued as
//!   a job of its own (BgKind::FilePreviews; path: the file as listed, `<dev>.<ino>.<size>.<mtime>:` and its storage
//!   path, as the copy job's). It is work nobody asked for by name, so it ranks behind an export, a template's
//!   picture, the PDF mirror, an import's slices and a font check (bgjob.rs `rank`), and ahead only of compaction and
//!   the idle warm-ups.
//! - The job (a background child, `bg_file_preview`): opens the file again through fsview's walk (Files' place mode:
//!   no link, one device, never the notebooks), checks it is still the file listed, and has ONE worker run make the
//!   preview, 300 x 400 and gray: a PDF's first page fitted (pdfwork.rs `R`), a picture fitted and turned upright
//!   (picwork.rs, a pool that never decodes in the writer and gives a picture one run), a font's fixed sample
//!   (fontcheck.rs `T`, its glyphs held to the check's glyph limit; a sample is no check, and no verdict comes of
//!   it). Each worker has its kind's limits and watchdog, the writer validates every record, as the import and the
//!   font check do, and the whole of it, from the worker's start to its last record, is held to the preview's own
//!   time (`preview_limit_ms`), a tenth of what its kind's import allows one record.
//! - What a job ends with (the import's rule for what counts as a try, pdfimport.rs): a preview; a "no preview"
//!   mark at once for a file its worker refuses (the same bytes give the same answer); a try on record (`once`) the
//!   first time a worker ends, lies or runs out of time on it, and the mark only the second time in a row, at
//!   another visit; or nothing at all when no worker can be had (a sandbox step failed, none started or answered),
//!   memory or room is short, the Kindle slept, or the file changed or is gone (the next visit asks).
//! - The cache: grimoire/.cache/files/<key>.pgm and <key>.info (docs/FORMAT.md), <key> the FNV-1a 64 hash of the
//!   file's storage path, its size and its time, so a changed file gets a new preview. Written atomically by the job
//!   alone (the picture first, then the .info that says it is whole), read by the UI with the plain-file read and a
//!   size limit, the .info parsed strictly and held to the file (its path, size, time and kind), the picture to the
//!   .info. After a job that wrote, the job tidies it (`tidy`): once a day, and whenever it is past 2,000 files or
//!   64 MB, the entries whose file is gone or changed go; past a limit the oldest go too. Deleting the folder loses
//!   nothing.
//!
//! grimoire.log never has a file's name here: kinds, counts, sizes, timings and reasons.

use crate::fsview::*;
use crate::prelude::*;
use crate::sandbox::{mono_ms, s1};
use std::io::Read;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

/// A preview's size: a Scribe page's shape, at most 300 x 400.
pub const PREVIEW_W: i32 = 300;
pub const PREVIEW_H: i32 = 400;
/// The cache is pruned past this many files ...
pub const CACHE_MAX_FILES: usize = 2000;
/// ... or this many bytes, the oldest previews first, down to three quarters of each.
pub const CACHE_MAX_BYTES: u64 = 64 << 20;
/// A `.info` file is at most this long: four short lines and the file's storage path.
const INFO_MAX: usize = 160 + FS_PATH_MAX;
/// A preview's picture file: its P5 header and its pixels, at most.
const PGM_MAX: usize = 32 + (PREVIEW_W * PREVIEW_H) as usize;
/// The number in a `.info`'s first line. It stands for the text and for the rules a mark was made by (the time a
/// preview may take, what the workers refuse): an entry of another number is no entry, and its file is asked for
/// again. Bump it whenever the text changes or the rules do.
pub const INFO_NUMBER: u32 = 1;
/// The most tiles a Covers screen holds on a Scribe (ten sideways, nine upright): what a preview's time is derived
/// from (`preview_limit_ms`).
pub const PREVIEW_TILES_MAX: u32 = 10;

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

/// The time one preview may take, in ms, from its worker's start to its last record (the monotonic clock, which
/// stops in sleep): its kind's import watchdog (sandbox.rs `s1`: what an import waits for one record of a page, a
/// picture or a font) divided by PREVIEW_TILES_MAX, 3 s for a PDF's page, 6 s for a picture, 3 s for a font's
/// sample. So a whole screen of files that each run to their limit holds the one background child no longer than one
/// record of an import may, and a job that outranks previews (all the others that matter) waits behind a running
/// preview for this long at most. A file that needs longer has no preview. Derived, not yet measured on the Scribe
/// (docs/TESTING.md asks for the times). Headless and test builds take GRIMOIRE_PREVIEW_MS for every kind.
pub fn preview_limit_ms(kind: PreviewKind) -> u64 {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_PREVIEW_MS").ok().and_then(|v| v.parse::<u64>().ok()).filter(|&v| v > 0) {
        return v;
    }
    let watchdog = match kind {
        PreviewKind::Pdf => s1::PAGE_WATCHDOG_MS,
        PreviewKind::Picture => s1::PICTURE_WATCHDOG_MS,
        PreviewKind::Font => s1::FONT_WATCHDOG_MS,
    };
    (watchdog / PREVIEW_TILES_MAX) as u64
}

/// Why a file has no preview: the worker refused it (no file of its kind Grimoire reads, or too big for its worker),
/// took too long on it, or ended on it.
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

/// What a `.info` says came of its file's preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Made {
    /// the picture beside it, this big
    Preview(i32, i32),
    /// the "no preview" mark, kept so the file is not tried at every visit, and why
    None(NoPreview),
    /// a worker ended or ran out of time on it once (never `Refused`): no mark yet. Another visit's job tries once
    /// more, and a second time in a row is the mark
    Once(NoPreview),
}

/// What a `.info` says: the file it is of (its storage path, size and time), its kind, and what came of its preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewInfo {
    pub size: u64,
    pub mtime: i64,
    pub kind: PreviewKind,
    pub made: Made,
    pub path: StoragePath,
}

impl PreviewInfo {
    /// The file's text: five lines (docs/FORMAT.md), None for a path that is no text or holds a control character
    /// (such a file is never listed).
    pub fn to_text(&self) -> Option<String> {
        let path = std::str::from_utf8(self.path.as_bytes()).ok().filter(|p| !p.is_empty() && !p.chars().any(|c| c.is_control()))?;
        let made = match self.made {
            Made::Preview(w, h) => format!("made {w} {h}"),
            Made::None(n) => format!("none {}", n.word()),
            Made::Once(n) => format!("once {}", n.word()),
        };
        Some(format!("grimoire-preview {INFO_NUMBER}\nof {} {}\nkind {}\n{made}\npath {path}\n", self.size, self.mtime, self.kind.word()))
    }

    /// The text read strictly: at most INFO_MAX bytes of UTF-8 without a control character but its line breaks,
    /// exactly the five lines in their order, each ended by a line break, single spaces, decimal numbers without a
    /// sign (but a time's minus) or a leading zero, a preview at most PREVIEW_W x PREVIEW_H, `once` never `refused`,
    /// a path that is a storage path. And only the very bytes `to_text` writes for what they say. Anything else is
    /// None (the entry is made again).
    pub fn parse(text: &[u8]) -> Option<PreviewInfo> {
        if text.len() > INFO_MAX {
            return None;
        }
        let whole = std::str::from_utf8(text).ok()?;
        let s = whole.strip_suffix('\n')?;
        let lines: Vec<&str> = s.split('\n').collect();
        let [head, of, kind, last, path] = lines[..] else { return None };
        if head != format!("grimoire-preview {INFO_NUMBER}") {
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
                Made::Preview(w as i32, h as i32)
            }
            ["none", why] => Made::None(NoPreview::from_word(why)?),
            ["once", why] => Made::Once(NoPreview::from_word(why).filter(|n| *n != NoPreview::Refused)?),
            _ => return None,
        };
        let path = StoragePath::new(path.strip_prefix("path ")?.as_bytes())?;
        let info = PreviewInfo { size, mtime, kind, made, path };
        (info.to_text().as_deref() == Some(whole)).then_some(info)
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

/// What the cache holds for a file as listed, for the Covers view.
pub enum Cached {
    /// nothing to show: not made yet, an entry that does not read (it is made again), or one try on record
    Nothing,
    /// its "no preview" mark
    NoPreview,
    /// its preview, decoded
    Preview(Canvas),
}

/// The `.info` of the file at `path` as listed (`st`): read with the plain-file read (at most INFO_MAX bytes), parsed
/// strictly, and only when it is of that very file: its path (two paths whose keys meet never share an entry), its
/// size, its time and its kind.
pub fn preview_info(path: &StoragePath, st: &FsStat) -> Option<PreviewInfo> {
    let kind = PreviewKind::of_name(path.split_last()?.1)?;
    let key = preview_key(path.as_bytes(), st.size, st.mtime);
    let info = PreviewInfo::parse(&read_file_max(&preview_file(key, "info"), INFO_MAX)?)?;
    (info.size == st.size && info.mtime == st.mtime && info.kind == kind && info.path == *path).then_some(info)
}

/// What the cache holds for the file at `path` as listed (`st`): its `.info`, then its picture, read with the
/// plain-file read and a size limit, the picture's header and length exactly what the `.info` says. The UI's read,
/// and the job's own test of "it is there".
pub fn preview_cached(path: &StoragePath, st: &FsStat) -> Cached {
    let Some(info) = preview_info(path, st) else { return Cached::Nothing };
    let (w, h) = match info.made {
        Made::Preview(w, h) => (w, h),
        Made::None(_) => return Cached::NoPreview,
        Made::Once(_) => return Cached::Nothing,
    };
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

/// Asks for the preview of the file at `path` as listed (`st`): one job, queued as work nobody waits on by name
/// (bgjob.rs ranks it behind exports, the PDF mirror, imports and font checks), unless that very job is queued or
/// running. Its job's path, or None: a file of no kind with a preview, a path that does not fit a job, or a session
/// in which a sandbox step already failed (no worker can be had: nothing is forked for it).
pub fn preview_request(path: &StoragePath, st: &FsStat) -> Option<String> {
    PreviewKind::of_name(path.split_last()?.1)?;
    if crate::sandbox::session_broken() {
        return None;
    }
    let jp = crate::bgwork_copy::job_path(path, st)?;
    // (one queued or running already is the one: a queued duplicate would be merged, a running one would be run twice)
    if !bg_job_pending(BgKind::FilePreviews, "", &jp) {
        bg_queue(BgKind::FilePreviews, None, Some(&jp), 0.0, false);
    }
    Some(jp)
}

/// How a file's preview ended: made, marked as having none, one try on record, or already there.
enum Outcome {
    Made,
    Marked(NoPreview),
    Once(NoPreview),
    There,
}

/// Why no preview came from a worker.
enum Fail {
    /// the worker says the file is at fault: the mark at once (the same bytes give the same answer)
    Refused,
    /// the worker ended, lied or ran out of time on it: a try, as a page's strike in an import. The first is put on
    /// record; the second in a row is the mark
    Strike(NoPreview),
    /// not the file's: nothing is written, and the next visit asks again
    Later(&'static str),
}

/// BgKind::FilePreviews, in the background child: the preview of the file `path` names (the job's path), its "no
/// preview" mark or its first try's record, in the cache; then the cache tidied. 0 when the job wrote one of them or
/// found the preview or the mark there, -1 when nothing was written (the log says why).
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
        Ok((kind, Outcome::Once(why))) => {
            qlog!("file previews: a %s gave no preview (%s) in %u ms; one more try at another visit", kind.word(), why.word(), ms);
            0
        }
        Ok((_, Outcome::There)) => 0,
        Err(why) => {
            qlog!("file previews: nothing made (%s)", why.as_str());
            -1
        }
    };
    if matches!(r, Ok((_, Outcome::Made | Outcome::Marked(_) | Outcome::Once(_)))) {
        tidy(&cache_dir());
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
    let info = |made| PreviewInfo { size: listed.size, mtime: listed.mtime, kind, made, path: src.clone() };
    if info(Made::None(NoPreview::Refused)).to_text().is_none() {
        return Err("a path the cache cannot record".into());
    }
    let dir = cache_dir();
    // (read and written as root: never through a planted link, and the folder made where it is missing)
    if !no_link_below(&store_root(), &dir) || mkdir_p(&dir) != 0 {
        return Err("the cache folder cannot be made".into());
    }
    // What the cache holds for it, by the UI's own test: a mark, or a preview whose picture reads, is there (asked
    // twice, and made by the first); one try on record makes this the second; anything else is made (again).
    let struck = match preview_info(&src, &listed).map(|i| i.made) {
        Some(Made::None(_)) => return Ok((kind, Outcome::There)),
        Some(Made::Preview(..)) if matches!(preview_cached(&src, &listed), Cached::Preview(_)) => return Ok((kind, Outcome::There)),
        Some(Made::Once(_)) => true,
        _ => false,
    };
    let root = crate::pdfimport::storage_root().ok_or("no storage")?;
    let st = FsStorage::open(&root, &store_root()).map_err(walk_why)?;
    // (place mode, as Files lists it: Grimoire's PDF copies and trash are read, its notebooks never)
    let fd = st.open_file(&src, false).map_err(walk_why)?;
    let at = fd_stat(&fd).ok_or("it does not stat")?;
    if !at.same(&listed) {
        return Err("it changed since it was listed".into());
    }
    // (a preview never takes the room the notebooks need: under the free floor none is made)
    if crate::bgwork_copy::free_bytes(&st).is_some_and(|f| !fsview_room_for(f, PGM_MAX as u64)) {
        return Err("the storage is nearly full".into());
    }
    let r = match kind {
        PreviewKind::Pdf => pdf_preview(&fd, at.size, &listed),
        PreviewKind::Picture => read_whole(&fd, at.size, BG_SOURCE_MAX_BYTES as u64, &listed).and_then(|d| picture_preview(&d)),
        PreviewKind::Font => read_whole(&fd, at.size, crate::fontcheck::FONT_FILE_MAX as u64, &listed).and_then(|d| font_preview(&d)),
    };
    let key = preview_key(src.as_bytes(), listed.size, listed.mtime);
    let write = |made| info(made).to_text().is_some_and(|t| write_file_atomic(&preview_file(key, "info"), t.as_bytes()) == 0);
    // (no picture beside a mark or a try's record)
    let no_picture = || {
        let _ = std::fs::remove_file(preview_file(key, "pgm"));
    };
    let failed = || -> Result<(PreviewKind, Outcome), String> { Err("the cache could not be written".into()) };
    let mark = |why: NoPreview| {
        no_picture();
        if !write(Made::None(why)) {
            return failed();
        }
        Ok((kind, Outcome::Marked(why)))
    };
    match r {
        Ok(c) => {
            // (the picture first: a .info that says "made" has one beside it, whole)
            if store_write_pgm(&preview_file(key, "pgm"), &c) != 0 || !write(Made::Preview(c.w, c.h)) {
                return failed();
            }
            Ok((kind, Outcome::Made))
        }
        Err(Fail::Later(why)) => Err(why.into()),
        Err(Fail::Strike(why)) if !struck => {
            no_picture();
            if !write(Made::Once(why)) {
                return failed();
            }
            Ok((kind, Outcome::Once(why)))
        }
        Err(Fail::Refused) => mark(NoPreview::Refused),
        Err(Fail::Strike(why)) => mark(why),
    }
}

/// The whole file, at most `max` bytes (a bigger one has no preview: its worker would not take it), still the file
/// listed when read.
fn read_whole(fd: &OwnedFd, size: u64, max: u64, listed: &FsStat) -> Result<Vec<u8>, Fail> {
    if size == 0 || size > max {
        return Err(Fail::Refused);
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

const NO_WORKER: &str = "no sandboxed worker can be had";

/// A PDF's first page fitted onto PREVIEW_W x PREVIEW_H, by one worker with the import's address space (the PDF
/// mapped where the storage allows it, else read whole under its cap), held to the preview's time (its CPU limit
/// the same, in whole seconds), and still the file listed once the worker is done.
fn pdf_preview(fd: &OwnedFd, size: u64, listed: &FsStat) -> Result<Canvas, Fail> {
    use crate::pdfwork::*;
    use crate::sandbox::*;
    let raw = fd.as_raw_fd();
    let mapped = crate::pdfimport::can_map(raw, size);
    if size == 0 || size > if mapped { s1::PDF_MAX_MAPPED } else { s1::PDF_MAX_READ_WHOLE } {
        return Err(Fail::Refused);
    }
    let limit = preview_limit_ms(PreviewKind::Pdf);
    let until = mono_ms() + limit;
    let limits = Limits { as_bytes: worker_as_bytes(size, false), cpu_secs: limit.div_ceil(1000).clamp(1, s1::SLICE_CPU_SECS), mapped, file_len: size };
    // The import's rule: a step the sandbox refuses, a worker that cannot be started or does not answer, and one
    // the filter ends before it was asked anything are no try (not the file's); a worker that ends, lies or stalls
    // once it opens the file is a try.
    let start_err = |e: StartErr| match e {
        StartErr::Sandbox(_) | StartErr::Worker(_) => Fail::Later(NO_WORKER),
        StartErr::Password | StartErr::Invalid | StartErr::Empty => Fail::Refused,
        StartErr::Failed(_) => Fail::Strike(if mono_ms() >= until { NoPreview::Slow } else { NoPreview::Ended }),
    };
    let mut s = Session::start_until(raw, size, &limits, until).map_err(start_err)?;
    // (the import's test hooks, in headless and test builds: a page that hangs or crashes its worker)
    for (name, page) in crate::pdfimport::test_hooks() {
        s.hook(name, page);
    }
    s.probe().map_err(start_err)?;
    let page = match s.render(1, PREVIEW_W, PREVIEW_H, 1) {
        PageOutcome::Page(p) => {
            s.end();
            p
        }
        PageOutcome::Died(Death::Timeout) => return Err(Fail::Strike(NoPreview::Slow)),
        // (the Kindle slept while the page was waited for: not the file's fault, and no try, as in an import)
        PageOutcome::Died(Death::TimeoutAfterWake) => return Err(Fail::Later("the Kindle slept while it was made")),
        PageOutcome::Died(_) | PageOutcome::Lied(_) => return Err(Fail::Strike(NoPreview::Ended)),
        PageOutcome::Failed(_) => return Err(Fail::Refused),
        PageOutcome::NoMemory => return Err(Fail::Later("out of memory")),
    };
    // (a PDF written over while its worker read it is not cached under the listing it no longer is)
    if !fd_stat(fd).is_some_and(|now| now.same(listed)) {
        return Err(Fail::Later("it changed while it was read"));
    }
    gray_of(&page.px, page.w, page.h, page.ch)
}

/// A picture fitted onto PREVIEW_W x PREVIEW_H and turned upright, by one run of the picture worker (never in the
/// writer, and no second worker), held to the preview's time.
fn picture_preview(data: &[u8]) -> Result<Canvas, Fail> {
    use crate::picwork::*;
    let mut pool = PicPool::for_preview(mono_ms() + preview_limit_ms(PreviewKind::Picture));
    match pool.decode(data, true, PREVIEW_W, PREVIEW_H) {
        PicOut::Pic { fit, .. } => {
            let mut c = Canvas::default();
            if bgimage_luma(&fit, &mut c) != 0 {
                return Err(Fail::Later("out of memory"));
            }
            Ok(c)
        }
        PicOut::Bad(_) => Err(Fail::Refused),
        PicOut::Dead(why) if why == STALLED => Err(Fail::Strike(NoPreview::Slow)),
        PicOut::Dead(_) => Err(Fail::Strike(NoPreview::Ended)),
        PicOut::Retry(_) | PicOut::Info { .. } => Err(Fail::Later("out of memory")),
        PicOut::NoWorker(_) => Err(Fail::Later(NO_WORKER)),
    }
}

/// A font's fixed sample on PREVIEW_W x PREVIEW_H, by one run of the font worker, held to the preview's time. A
/// sample is no check of the font: nothing here reads or writes a verdict.
fn font_preview(data: &[u8]) -> Result<Canvas, Fail> {
    use crate::fontcheck::*;
    match font_sample(data, PREVIEW_W, PREVIEW_H, mono_ms() + preview_limit_ms(PreviewKind::Font)) {
        Ok(px) => gray_of(&px, PREVIEW_W, PREVIEW_H, 1),
        Err(FontOut::Refused(_)) => Err(Fail::Refused),
        // (its time ran out, or a glyph of the sample is over the check's glyph limit: measured, so a second try
        // must agree, as the check's own)
        Err(FontOut::Slow | FontOut::SlowGlyphs { .. }) => Err(Fail::Strike(NoPreview::Slow)),
        Err(FontOut::Dead(_)) => Err(Fail::Strike(NoPreview::Ended)),
        Err(FontOut::NoWorker(_)) => Err(Fail::Later(NO_WORKER)),
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

// ---- the cache tidied: the files that are gone, and the prune ----

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

/// The names the tidying reads of the cache at most (a cache within its limits holds far fewer).
const NAMES_MAX: usize = 4 * CACHE_MAX_FILES;
/// The sweep's stamp in the cache folder: empty when the last sweep went through (its time is the stamp's), else
/// the stem that sweep stopped after.
const SWEPT: &str = ".swept";
/// The sweep for files that are gone runs once this long ...
const SWEEP_EVERY_SECS: i64 = 86_400;
/// ... and for this long at most in one job; what is left is the next writing job's, from where this one stopped.
const SWEEP_MS: u32 = 2_000;

/// The stem of a cache file's name: the part before its first dot.
fn stem_of(name: &[u8]) -> &[u8] {
    name.split(|&c| c == b'.').next().unwrap_or(&[])
}

/// Whether the sweep is due, and from where: None when the last one went through less than a day ago (or the stamp
/// is no plain file: not ours to judge by), Some("") for a whole one, Some(stem) to go on after that stem.
fn sweep_from(dir: &str) -> Option<String> {
    let stamp = format!("{dir}/{SWEPT}");
    let Ok(m) = std::fs::symlink_metadata(&stamp) else { return Some(String::new()) };
    if !m.is_file() {
        return None;
    }
    use std::os::unix::fs::MetadataExt;
    let at = read_file_max(&stamp, 32).and_then(|b| String::from_utf8(b).ok()).unwrap_or_default();
    let at = at.trim_end();
    if at.len() == 16 && at.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Some(at.to_string());
    }
    let age = wall_time() - m.mtime();
    (!(0..SWEEP_EVERY_SECS).contains(&age)).then(String::new)
}

/// The sweep: each entry's `.info` read (in the stems' order, after `from`), and the entry dropped when the file it
/// names is no longer there as it was (gone, renamed, trashed, replaced, or no longer reached by Files' walk); an
/// entry whose `.info` does not read is left to be made again, and to the prune. At most SWEEP_MS: the stamp then
/// says where it stopped. The names that are left.
fn sweep(dir: &str, names: Vec<std::ffi::OsString>, from: &str) -> Vec<std::ffi::OsString> {
    use std::os::unix::ffi::OsStrExt;
    let Some(st) = crate::pdfimport::storage_root().and_then(|root| FsStorage::open(&root, &store_root()).ok()) else { return names };
    let mut stems: Vec<&[u8]> = names.iter().map(|n| n.as_bytes()).filter(|n| n.ends_with(b".info")).map(stem_of).filter(|s| *s > from.as_bytes()).collect();
    stems.sort_unstable();
    let t0 = real_now_ms();
    let (mut gone, mut stopped): (Vec<Vec<u8>>, Option<&[u8]>) = (Vec::new(), None);
    for (i, stem) in stems.iter().enumerate() {
        if i > 0 && real_now_ms().wrapping_sub(t0) >= SWEEP_MS {
            stopped = Some(stems[i - 1]);
            break;
        }
        let file = |ext: &str| format!("{dir}/{}.{ext}", String::from_utf8_lossy(stem));
        let Some(info) = read_file_max(&file("info"), INFO_MAX).and_then(|b| PreviewInfo::parse(&b)) else { continue };
        let there = match st.stat(&info.path, false) {
            Ok(s) => s.mode & libc::S_IFMT as u32 == libc::S_IFREG as u32 && s.size == info.size && s.mtime == info.mtime,
            // (the storage did not answer: no reason to think the file gone)
            Err(FsErr::Io(_)) => true,
            Err(_) => false,
        };
        if !there {
            let _ = std::fs::remove_file(file("pgm"));
            if std::fs::remove_file(file("info")).is_ok() {
                gone.push(stem.to_vec());
            }
        }
    }
    // (the stamp made afresh, never through what stands at its name: a link planted there is unlinked, not followed)
    if let Ok(mut f) = create_fresh_nofollow(&format!("{dir}/{SWEPT}")) {
        use std::io::Write;
        let _ = f.write_all(stopped.unwrap_or(&[]));
    }
    if !gone.is_empty() {
        qlog!("file previews: %d preview(s) of files that are gone or changed dropped from the cache", gone.len() as i32);
    }
    if gone.is_empty() {
        return names;
    }
    names.into_iter().filter(|n| !gone.iter().any(|g| g.as_slice() == stem_of(n.as_bytes()))).collect()
}

/// The cache tidied, in the job's child, after a job that wrote. The names alone first (plain files by the
/// listing's own word, no stat): while they are within both limits by the most their files can hold and no sweep is
/// due, nothing more is read. The sweep, once a day and whenever the cache is past a limit, drops the entries whose
/// file is gone or changed. Past a limit still, every file's size and time is read, and prune_plan's stems go, each
/// with all of its files. Entries that are no plain file are left to themselves (never followed, never counted).
fn tidy(dir: &str) {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    let (max_files, max_bytes) = prune_limits();
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut names: Vec<std::ffi::OsString> = Vec::new();
    let mut more = 0usize;
    for e in rd.flatten() {
        let n = e.file_name();
        if n.as_bytes().first() == Some(&b'.') || !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        if names.len() >= NAMES_MAX {
            more += 1;
            continue;
        }
        names.push(n);
    }
    if more > 0 {
        qlog!("file previews: the cache holds %d file(s) more than are looked at", more as i32);
    }
    let past = |names: &[std::ffi::OsString]| {
        let most: u64 = names.iter().map(|n| if n.as_bytes().ends_with(b".info") { INFO_MAX as u64 } else { PGM_MAX as u64 }).sum();
        names.len() > max_files || most > max_bytes
    };
    let over = past(&names);
    match sweep_from(dir) {
        Some(from) => names = sweep(dir, names, if over { "" } else { &from }),
        None if over => names = sweep(dir, names, ""),
        None => {}
    }
    if !past(&names) {
        return;
    }
    let mut entries: std::collections::HashMap<Vec<u8>, CacheEntry> = std::collections::HashMap::new();
    for n in &names {
        let Ok(m) = std::fs::symlink_metadata(Path::new(dir).join(n)) else { continue };
        if !m.is_file() {
            continue;
        }
        let stem = stem_of(n.as_bytes());
        let e = entries.entry(stem.to_vec()).or_insert_with(|| CacheEntry { stem: String::from_utf8_lossy(stem).into_owned(), files: 0, bytes: 0, mtime: i64::MIN });
        e.files += 1;
        e.bytes += m.len();
        e.mtime = e.mtime.max(m.mtime());
    }
    let entries: Vec<CacheEntry> = entries.into_values().collect();
    let gone: std::collections::HashSet<String> = prune_plan(&entries, max_files, max_bytes).into_iter().collect();
    if gone.is_empty() {
        return;
    }
    let (mut files, mut bytes) = (0usize, 0u64);
    for n in &names {
        if gone.contains(String::from_utf8_lossy(stem_of(n.as_bytes())).as_ref()) {
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

// ---- the Covers view's tiles (UI thread) ----

/// What a Covers tile shows: the file's preview, the hourglass while it is to come, or its kind's icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileShow {
    Preview,
    Loading,
    Icon,
}

impl TileShow {
    /// Its letter for the runner's files.tiles: `p`, `l`, `i`.
    pub fn letter(self) -> char {
        match self {
            TileShow::Preview => 'p',
            TileShow::Loading => 'l',
            TileShow::Icon => 'i',
        }
    }
}

/// Where a file of the Covers screen is with its preview.
enum TileState {
    /// to be looked up in the cache (`CoverTiles::step`, a slice a tick)
    Look,
    /// its job is queued or running (the job's path)
    Job(String),
    /// decoded
    Preview(Canvas),
    /// its mark, or nothing to be had this visit: its kind's icon
    Icon,
}

struct Tile {
    path: StoragePath,
    stat: FsStat,
    state: TileState,
}

/// The Covers view's previews (2.2), on the UI thread: the files of its screen, each looked up in the cache when it
/// comes onto the screen (again each time it comes back: a preview decoded is dropped when its file leaves), a few a
/// tick; a file the cache has nothing for asked for once a visit, as a job of its own; and what each tile was drawn
/// showing, so that the tiles that changed are drawn together.
#[derive(Default)]
pub struct CoverTiles {
    /// the files of the screen that have a kind with a preview, in the listed order
    on: Vec<Tile>,
    /// what each tile on screen was drawn showing, by its file
    shown: Vec<(StoragePath, TileShow)>,
    /// the files looked at this visit (the runner's files.asked): one whose queued job was taken back is taken out
    asked: Vec<(StoragePath, FsStat)>,
    /// the files whose job ended this visit with no preview and no mark (no worker, no memory, a first try that
    /// failed): not asked for again this visit
    nothing: Vec<(StoragePath, FsStat)>,
}

impl CoverTiles {
    /// A visit ends (another folder, the list, Files closed): the jobs still queued for it are taken out of the queue
    /// (a running one runs on: one preview's time at most), and nothing of it is kept.
    pub fn forget(&mut self) {
        for t in &self.on {
            if let TileState::Job(j) = &t.state {
                bg_unqueue(BgKind::FilePreviews, "", j);
            }
        }
        *self = CoverTiles::default();
    }

    /// The screen shows these files now (`on`: the files of its tiles that have a kind with a preview, each with its
    /// stat, in the listed order). The jobs still queued for files no longer on it are taken out of the queue, and the
    /// previews decoded of files gone from it are dropped; a file that is new to the screen, or back on it, is to be
    /// looked up in the cache (`step`). Nothing is read here.
    pub fn screen(&mut self, on: &[(StoragePath, FsStat)]) {
        if on.len() == self.on.len() && on.iter().zip(&self.on).all(|((p, s), t)| *p == t.path && s.same(&t.stat)) {
            return; // (the same screen as at the last look)
        }
        let mut old = std::mem::take(&mut self.on);
        for (p, s) in on {
            let state = match old.iter().position(|t| t.path == *p && t.stat.same(s)) {
                Some(i) => old.swap_remove(i).state,
                None => TileState::Look,
            };
            if self.on.try_reserve(1).is_ok() {
                self.on.push(Tile { path: p.clone(), stat: *s, state });
            }
        }
        for t in old {
            if let TileState::Job(j) = &t.state {
                if bg_unqueue(BgKind::FilePreviews, "", j) {
                    // (not made, and no longer asked for: asked for again when it is back on screen)
                    self.asked.retain(|(p, s)| !(*p == t.path && s.same(&t.stat)));
                }
            }
        }
    }

    /// One slice of the screen's lookups, at most `budget_ms` (one file at least): a file to be looked up, or whose
    /// job ended, is read from the cache: its preview is decoded, a mark is taken as it is, and with nothing there a
    /// job is queued, once a visit (a file whose job ended with nothing keeps its icon for the visit). A cache folder
    /// that is a link is not read. True when a tile's state changed.
    pub fn step(&mut self, budget_ms: u32) -> bool {
        let t0 = real_now_ms();
        let mut changed = false;
        let mut linked: Option<bool> = None;
        for i in 0..self.on.len() {
            let ended = match &self.on[i].state {
                TileState::Look => false,
                TileState::Job(j) if !bg_job_pending(BgKind::FilePreviews, "", j) => true,
                _ => continue,
            };
            let (p, s) = (self.on[i].path.clone(), self.on[i].stat);
            let here = |v: &[(StoragePath, FsStat)]| v.iter().any(|(q, t)| *q == p && t.same(&s));
            let linked = *linked.get_or_insert_with(|| !no_link_below(&store_root(), &cache_dir()));
            let cached = if linked { Cached::Nothing } else { preview_cached(&p, &s) };
            let state = match cached {
                Cached::Preview(c) => TileState::Preview(c),
                Cached::NoPreview => TileState::Icon,
                // (its job ended and left nothing to show, or it did so earlier this visit)
                Cached::Nothing if ended || here(&self.nothing) => {
                    if ended && !here(&self.nothing) && self.nothing.try_reserve(1).is_ok() {
                        self.nothing.push((p.clone(), s));
                    }
                    TileState::Icon
                }
                Cached::Nothing => preview_request(&p, &s).map_or(TileState::Icon, TileState::Job),
            };
            if !here(&self.asked) && self.asked.try_reserve(1).is_ok() {
                self.asked.push((p, s));
            }
            self.on[i].state = state;
            changed = true;
            if real_now_ms().wrapping_sub(t0) >= budget_ms {
                break;
            }
        }
        changed
    }

    fn tile(&self, p: &StoragePath, s: &FsStat) -> Option<&Tile> {
        self.on.iter().find(|t| t.path == *p && t.stat.same(s))
    }

    /// What the tile of the file at `p` (as listed: `s`) shows now: its preview once decoded; its kind's icon for a
    /// mark or a job that ended without a preview; else the hourglass (to be looked up, or its job queued or
    /// running).
    pub fn show(&self, p: &StoragePath, s: &FsStat) -> TileShow {
        match self.tile(p, s).map(|t| &t.state) {
            Some(TileState::Preview(_)) => TileShow::Preview,
            Some(TileState::Icon) => TileShow::Icon,
            _ => TileShow::Loading,
        }
    }

    /// The decoded preview of the file at `p` (as listed: `s`).
    pub fn preview(&self, p: &StoragePath, s: &FsStat) -> Option<&Canvas> {
        match self.tile(p, s).map(|t| &t.state) {
            Some(TileState::Preview(c)) => Some(c),
            _ => None,
        }
    }

    /// What the tiles were drawn showing, taken (a whole draw notes each tile anew).
    pub fn take_shown(&mut self) -> Vec<(StoragePath, TileShow)> {
        std::mem::take(&mut self.shown)
    }

    /// The tile of `p` was drawn showing `show`.
    pub fn note(&mut self, p: &StoragePath, show: TileShow) {
        match self.shown.iter_mut().find(|(q, _)| q == p) {
            Some(s) => s.1 = show,
            None => {
                if self.shown.try_reserve(1).is_ok() {
                    self.shown.push((p.clone(), show));
                }
            }
        }
    }

    /// What the tile of `p` was last drawn showing.
    pub fn shown(&self, p: &StoragePath) -> Option<TileShow> {
        self.shown.iter().find(|(q, _)| q == p).map(|s| s.1)
    }

    /// Whether the screen's set is not complete yet: a file of it is still to be looked up, or its job is queued or
    /// running (or ended and not looked at yet).
    pub fn pending(&self) -> bool {
        self.on.iter().any(|t| matches!(t.state, TileState::Look | TileState::Job(_)))
    }

    /// Whether a job the screen asked for is queued or running (the notice "Making previews..." stays meanwhile).
    pub fn making(&self) -> bool {
        self.on.iter().any(|t| matches!(&t.state, TileState::Job(j) if bg_job_pending(BgKind::FilePreviews, "", j)))
    }

    /// How many files were looked at this visit (the runner's files.asked).
    pub fn asked(&self) -> usize {
        self.asked.len()
    }
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

    fn info(made: Made) -> PreviewInfo {
        PreviewInfo { size: 1234, mtime: 1_700_000_000, kind: PreviewKind::Pdf, made, path: StoragePath::new(b"documents/a b.pdf").unwrap() }
    }

    // A .info is read back as written, and anything else is refused: each line's word and order, the numbers' form,
    // the preview's bounds, a first try that says "refused", the path's form, the length, a control character, a
    // missing or extra line break.
    #[test]
    fn the_info_is_parsed_strictly() {
        for i in [info(Made::Preview(300, 400)), info(Made::Preview(1, 1)), info(Made::None(NoPreview::Refused)), info(Made::None(NoPreview::Slow)), info(Made::None(NoPreview::Ended)), info(Made::Once(NoPreview::Slow)), info(Made::Once(NoPreview::Ended))] {
            assert_eq!(PreviewInfo::parse(i.to_text().unwrap().as_bytes()), Some(i));
        }
        let neg = PreviewInfo { mtime: -5, kind: PreviewKind::Font, path: StoragePath::new("fonts/\u{e9}t\u{e9}.ttf".as_bytes()).unwrap(), ..info(Made::Preview(300, 400)) };
        assert_eq!(PreviewInfo::parse(neg.to_text().unwrap().as_bytes()), Some(neg));
        let good = "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n";
        assert_eq!(PreviewInfo::parse(good.as_bytes()).map(|i| i.path), StoragePath::new(b"photos/p.png"));
        assert_eq!(info(Made::Preview(300, 400)).to_text().as_deref(), Some("grimoire-preview 1\nof 1234 1700000000\nkind pdf\nmade 300 400\npath documents/a b.pdf\n"));
        for bad in [
            "",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n\n",
            "grimoire-preview 2\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 01\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nkind picture\nof 1234 1700000000\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 01234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof +1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 -0\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234  1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind photo\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 301 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 401\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 0 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nnone tired\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nnone\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nonce refused\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nonce\npath photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\nmore\n",
            "grimoire-preview 1\r\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            "grimoire-preview 1\nof 99999999999999999999 1700000000\nkind picture\nmade 300 400\npath photos/p.png\n",
            // the path: there, a storage path (no empty part, no "..", no slash first), no control character
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath \n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath /photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/../p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos//p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p\t.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\nfile photos/p.png\n",
            "grimoire-preview 1\nof 1234 1700000000\nkind picture\nmade 300 400\npath photos/p.png \nx\n",
        ] {
            assert_eq!(PreviewInfo::parse(bad.as_bytes()), None, "{bad:?}");
        }
        // too long; bytes that are no text
        let long = good.replace("p.png", &format!("{}.png", "p".repeat(INFO_MAX)));
        assert_eq!(PreviewInfo::parse(long.as_bytes()), None);
        assert_eq!(PreviewInfo::parse(b"grimoire-preview 1\nof 1 2\nkind pdf\nmade 3 4\npath a\xff.pdf\n"), None);
        // a path the cache cannot record has no text
        let odd = PreviewInfo { path: StoragePath::new(b"documents/a\nb.pdf").unwrap(), ..info(Made::Preview(300, 400)) };
        assert_eq!(odd.to_text(), None);
        // the longest storage path fits
        let deep = PreviewInfo { size: 9_999_999_999_999_999_999, mtime: i64::MIN + 1, path: StoragePath::new(format!("d/{}.pdf", "n".repeat(FS_PATH_MAX - 8)).as_bytes()).unwrap(), ..info(Made::None(NoPreview::Refused)) };
        assert!(deep.to_text().is_some_and(|t| t.len() <= INFO_MAX && PreviewInfo::parse(t.as_bytes()) == Some(deep.clone())));
    }

    // A preview's time is its kind's import watchdog over the most tiles a screen holds: 3 s for a PDF's page, 6 s
    // for a picture, 3 s for a font's sample, each tighter than the import's own. They are rules a "no preview"
    // mark is made by, and the .info's number stands for them: whoever changes a time (here or in sandbox.rs's
    // table) bumps the number, so that the files an older time marked are asked for again.
    #[test]
    fn a_previews_time_is_a_tenth_of_its_imports_and_the_infos_number_follows_it() {
        std::env::remove_var("GRIMOIRE_PREVIEW_MS");
        let ms = [PreviewKind::Pdf, PreviewKind::Picture, PreviewKind::Font].map(preview_limit_ms);
        assert_eq!(
            (INFO_NUMBER, PREVIEW_TILES_MAX, ms),
            (1, 10, [3000, 6000, 3000]),
            "a preview's time (preview_limit_ms, sandbox.rs s1's watchdogs) and the .info's number (INFO_NUMBER) move together"
        );
        assert!(ms[0] < s1::PAGE_WATCHDOG_MS as u64 && ms[1] < s1::PICTURE_WATCHDOG_MS as u64 && ms[2] < s1::FONT_WATCHDOG_MS as u64);
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
