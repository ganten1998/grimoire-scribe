//! PDFs as templates (2.2, issue #172; docs/DESIGN.md, "PDFs as templates"): a PDF in grimoire/templates (copied in
//! over USB, or by Use as template in Files) gives one template a page. Its pages are drawn once, by the sandboxed
//! PDF worker an import uses (pdfwork.rs: the same exec, filter and limits), into the form a picture template takes:
//! each page a picture of the Kindle's page size, with its preview and its key beside it, in the templates folder's
//! own cache. From there a page is a template as any picture is (bgwork_template.rs: template_install fits it to a
//! notebook's page in the background, through the picture worker, and that copy is the notebook's own), so drawing
//! a page, picking one or opening a notebook on one never parses the PDF again, and nothing but the worker ever
//! parses it.
//!
//! - The cache (docs/FORMAT.md, "Template PDFs"): beside the previews of the folder's pictures, under the key theirs
//!   have (`<hash>`, the FNV-1a hash of the PDF's name, size and time), `<hash>.pages` (its page count, written once
//!   the worker counted them) and for page n `<hash>-p<nnnn>.png` (the page), `.key` (the hash of that PNG's bytes,
//!   and its size) and `.pgm` (its preview), written in that order; `<hash>-p<nnnn>.bad` for a page that cannot be
//!   drawn, `<hash>.bad` for a PDF that is no template. Disposable: deleting it costs a new drawing of the pages.
//! - The job: BgKind::Template with no notebook and the PDF's path, queued by the scan that looks at the folder
//!   (template_scan, as the Templates sheet opens), ranked behind imports and font checks (bgjob.rs `rank`). It runs
//!   in slices as an import does (at most SLICE_PAGES pages or SLICE_WALL_MS of a slice), each one but the last
//!   ending "more to do" (CODE_MORE), queued again behind whatever ranks higher.
//! - Limits: the templates folder's 16 MB (TEMPLATE_MAX_BYTES, as a picture); TEMPLATE_PDF_MAX_PAGES pages (a longer
//!   PDF is no template, as a whole: it is never cut to its first pages); the import's worker limits (address
//!   space, CPU, the page watchdog, band mode under little memory); the free-space floor before every page.
//! - What a failure costs: a PDF the worker refuses (no PDF, a password, no page), one past the limits, one whose
//!   worker lies, one that ends or stalls its worker twice while it is opened, and one with no page that draws, is
//!   marked bad as a whole, as a picture is. A page the worker cannot draw is marked alone, and so is one whose
//!   worker ends or stalls twice on it; DEAD_PAGES_MAX such pages mark the whole PDF. No worker to be had (a sandbox
//!   step failed, none starts), no room, a file that changes while its pages are made: nothing is marked, and the
//!   next look tries again.
//!
//! grimoire.log never names the file here: page numbers, counts, sizes and reasons.

use crate::bgwork_template::TEMPLATE_MAX_BYTES;
use crate::pdfwork::{Death, PageOutcome, PagePixels, Session, StartErr};
use crate::prelude::*;
use crate::sandbox::{mono_ms, s1, worker_as_bytes, Limits};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// A template PDF has at most this many pages. One with more is no template (marked bad as a whole, its count in the
/// log), never cut to its first pages: a sheet of 50 tiles is three pages of the Templates sheet, a planner of a year
/// is a notebook (an import).
pub const TEMPLATE_PDF_MAX_PAGES: u32 = 50;

/// How many PDFs one look at the folder queues (each its own job; the queue holds 16).
pub const TEMPLATE_PDF_QUEUE_MAX: usize = 8;

/// A page whose worker ends or stalls this many times is marked, and so is a PDF whose worker does so while it is
/// opened.
const TRIES: u32 = 2;

/// A PDF with this many pages marked so (their workers ended or stalled twice each) is no template at all.
pub const DEAD_PAGES_MAX: usize = 2;

/// A slice that ends with pages left: bgjob.rs queues the job again (its exit status).
pub const CODE_MORE: i32 = 3;

/// What a page's `.bad` says when its worker ended or stalled on it twice (a page the worker could not draw says
/// nothing).
const DIED: &[u8] = b"died\n";

/// A `.pages` file is read up to this many bytes (its count and a line break).
const PAGES_FILE_MAX: usize = 16;

/// The room a page needs on the storage before its worker is asked: its picture (a planner's is tens of KB, a scan's
/// a few MB) and its preview. Checked again with the picture's real size before anything is written.
const PAGE_ROOM: u64 = 4 << 20;

/// What the cache says of a PDF in the templates folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfPages {
    /// nothing yet: its pages are still to be counted
    Unknown,
    /// no template (`<hash>.bad`)
    Bad,
    /// this many pages (`<hash>.pages`), each made, marked or still to come
    Known(u32),
}

/// Whether a file's name is a PDF's: `.pdf` in any case, and no dot-file.
pub fn is_pdf_name(n: &str) -> bool {
    !n.starts_with('.') && crate::browser::is_pdf(n.as_bytes())
}

fn templates_dir() -> String {
    format!("{}/templates", store_root())
}

/// The 16 hexadecimal digits a file's cache entries begin with.
pub fn hash_hex(h: u64) -> String {
    format!("{h:016x}")
}

fn pages_file(cache: &str, hash: &str) -> String {
    format!("{cache}/{hash}.pages")
}

fn bad_file(cache: &str, hash: &str) -> String {
    format!("{cache}/{hash}.bad")
}

/// The cache file of page `page` (from 1) of the PDF whose entries begin with `hash`, with this extension (png, key,
/// pgm, bad).
pub fn page_file(cache: &str, hash: &str, page: u32, ext: &str) -> String {
    format!("{cache}/{hash}-p{page:04}.{ext}")
}

/// The count a `.pages` file holds: its text exactly "<n>\n", n from 1 to TEMPLATE_PDF_MAX_PAGES in decimal without
/// a sign or a leading zero. None for anything else (the pages are then counted again).
pub fn parse_pages(t: &[u8]) -> Option<u32> {
    let d = t.strip_suffix(b"\n")?;
    if d.is_empty() || d.len() > 3 || !d.iter().all(|c| c.is_ascii_digit()) || d[0] == b'0' {
        return None;
    }
    let n: u32 = std::str::from_utf8(d).ok()?.parse().ok()?;
    (1..=TEMPLATE_PDF_MAX_PAGES).contains(&n).then_some(n)
}

/// What the cache folder `cache` says of the PDF whose entries begin with `hash`.
pub fn pdf_state(cache: &str, hash: &str) -> PdfPages {
    if Path::new(&bad_file(cache, hash)).exists() {
        return PdfPages::Bad;
    }
    match read_file_max(&pages_file(cache, hash), PAGES_FILE_MAX).as_deref().and_then(parse_pages) {
        Some(n) => PdfPages::Known(n),
        None => PdfPages::Unknown,
    }
}

fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

/// Whether page `page` is made (its picture, key and preview there) or marked.
fn page_done(cache: &str, hash: &str, page: u32) -> bool {
    let f = |ext| page_file(cache, hash, page, ext);
    (exists(&f("pgm")) && exists(&f("key")) && exists(&f("png"))) || exists(&f("bad"))
}

/// For the picture of a template PDF's page (`<templates>/.cache/<16 hex>-p<4 digits>.png`), its stem (the path
/// without `.png`): its preview, key and mark are beside it, at that stem with their own extensions. None for any
/// other path.
pub fn page_stem(path: &str) -> Option<&str> {
    let dir = templates_dir();
    let rest = path.strip_prefix(dir.as_str())?.strip_prefix("/.cache/")?;
    let b = rest.as_bytes();
    // "<16 hex>-p<4 digits>.png": 26 bytes
    let ok = b.len() == 26
        && b[..16].iter().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
        && &b[16..18] == b"-p"
        && b[18..22].iter().all(|c| c.is_ascii_digit())
        && &b[22..] == b".png";
    ok.then(|| &path[..path.len() - 4])
}

/// A PDF page's tile name: the file's name (cut short at a character so the page's part fits) and the page's part.
pub fn page_name(base: &str, page: u32) -> String {
    let suffix = crate::tplsheet::pdf_page_suffix(page);
    let room = (STORE_NAME_LEN - 1).saturating_sub(suffix.len());
    format!("{}{suffix}", cut_at_bytes(base, room))
}

/// The scan's part for a PDF of the templates folder (`dir`/`name`, at `path`; `base` its name without `.pdf`): its
/// tiles into `out` as the cache has them (one tile a page once its pages are counted, each ready, bad or still to
/// come; until then one tile for the whole PDF, still to come; one bad tile for a PDF that is no template). True when
/// its job has work to do.
pub fn scan_pdf(dir: &str, name: &str, path: &str, base: &str, out: &mut Vec<Template>) -> bool {
    let Ok(st) = std::fs::metadata(path) else { return false };
    let hash = hash_hex(crate::bgwork_template::preview_hash(name, st.len(), st.mtime()));
    let cache = format!("{dir}/.cache");
    let whole = |bad: bool| {
        let mut t = Template { bad, ..Default::default() };
        t.path.set(path);
        t.name.set_cut(base);
        t
    };
    match pdf_state(&cache, &hash) {
        PdfPages::Bad => {
            out.push(whole(true));
            false
        }
        PdfPages::Unknown => {
            out.push(whole(false));
            true
        }
        PdfPages::Known(n) => {
            let mut work = false;
            for k in 1..=n {
                let f = |ext| page_file(&cache, &hash, k, ext);
                let mut t = Template { page: k, ..Default::default() };
                t.path.set(&f("png"));
                t.name.set_cut(&page_name(base, k));
                if exists(&f("pgm")) && exists(&f("png")) {
                    t.ready = true;
                    work |= !exists(&f("key"));
                } else if exists(&f("bad")) {
                    t.bad = true;
                } else {
                    work = true;
                }
                out.push(t);
            }
            work
        }
    }
}

/// Queues the jobs of the PDFs a look at the folder found work for (at most TEMPLATE_PDF_QUEUE_MAX, none queued or
/// running already), unless no worker can be had in this session.
pub fn queue_jobs(paths: &[String], dpi: f32) {
    if crate::sandbox::session_broken() {
        return;
    }
    for p in paths.iter().take(TEMPLATE_PDF_QUEUE_MAX) {
        if !bg_job_pending(BgKind::Template, "", p) {
            bg_queue(BgKind::Template, Some(""), Some(p), dpi, false);
        }
    }
}

/// Whether one of these templates is a whole PDF still waiting for its pages to be counted whose pages are counted
/// now (or which is no template now): a sheet showing it reads the folder again.
pub fn pdf_pages_counted(t: &[Template]) -> bool {
    t.iter().any(|t| {
        if t.bad || t.page != 0 || !is_pdf_name(t.path.as_str().rsplit('/').next().unwrap_or("")) {
            return false;
        }
        let path = t.path.as_str();
        let (Some(slash), Ok(st)) = (path.rfind('/'), std::fs::metadata(path)) else { return false };
        let hash = hash_hex(crate::bgwork_template::preview_hash(&path[slash + 1..], st.len(), st.mtime()));
        pdf_state(&format!("{}/.cache", &path[..slash]), &hash) != PdfPages::Unknown
    })
}

// ---- the job ----

// (tests) What runs in this thread right after a slice's page count (with 0) and after each page a job made (with the
// page's number): a test changes the file, or the room, there.
#[cfg(feature = "sim")]
type AfterPage = Option<Box<dyn FnMut(u32)>>;
#[cfg(feature = "sim")]
thread_local! {
    static AFTER_PAGE: std::cell::RefCell<AfterPage> = const { std::cell::RefCell::new(None) };
}

/// (tests) `f` runs in this thread right after the next jobs' page counts (with 0) and after each page they make (with
/// its number), until `sim_after_page_clear`.
#[cfg(feature = "sim")]
pub fn sim_after_page(f: impl FnMut(u32) + 'static) {
    AFTER_PAGE.with(|a| *a.borrow_mut() = Some(Box::new(f)));
}

#[cfg(feature = "sim")]
pub fn sim_after_page_clear() {
    AFTER_PAGE.with(|a| *a.borrow_mut() = None);
}

fn after_page(_page: u32) {
    #[cfg(feature = "sim")]
    AFTER_PAGE.with(|a| {
        if let Some(f) = a.borrow_mut().as_mut() {
            f(_page);
        }
    });
}

/// A sandbox step refused: not the file's fault, and this process asks for no more workers (as the pictures' pool and
/// the font check do).
fn sandbox_refused(step: &str) -> Stop {
    crate::sandbox::session_set_broken();
    Stop::Later(format!("the sandbox refused its step \"{step}\""))
}

/// How a page ended, for the slice.
enum PageEnd {
    Made,
    /// marked, with what its mark says
    Marked(&'static [u8]),
}

/// Why a slice stops without its pages done.
enum Stop {
    /// the PDF is no template: marked as a whole, with the log's reason
    Bad(String),
    /// not the file's fault (no worker, no room, a write that failed, the file changed): nothing marked, the log's
    /// reason; the next look tries again
    Later(String),
    /// the file changed while its pages were made: what was made of it goes
    Changed,
}

struct Run<'a> {
    src: &'a str,
    cache: String,
    hash: String,
    file: std::fs::File,
    md: std::fs::Metadata,
    limits: Limits,
    bands: u32,
    pw: i32,
    ph: i32,
    sess: Option<Session>,
}

/// BgKind::Template with no notebook and a PDF's path (`src`, directly in grimoire/templates), in the background
/// child: a slice of the PDF's pages made into templates. 0 when the PDF is done (every page made or marked, or the
/// PDF marked: nothing more to do), CODE_MORE when pages are left for the next slice, -1 when nothing could be done
/// now (the log says why; nothing is marked).
pub fn bg_template_pdf(src: &str) -> i32 {
    // (a worker that ended by itself has closed its pipe: a write to it must fail, never end this child)
    // SAFETY: setting a signal's disposition to ignore.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
    let dir = templates_dir();
    let Some(name) = src.strip_prefix(dir.as_str()).and_then(|r| r.strip_prefix('/')).filter(|n| !n.contains('/') && is_pdf_name(n)) else {
        qlog!("templates: a PDF's job for a file that is no PDF of the templates folder; nothing made");
        return -1;
    };
    let cache = format!("{dir}/.cache");
    // (written as root: never through a planted link, and the folder made where it is missing)
    if !no_link_below(&store_root(), &cache) || mkdir_p(&cache) != 0 {
        qlog!("templates: the cache folder cannot be made; no PDF's pages made");
        return -1;
    }
    // The file, opened once: a plain file, no link. Its cache entries are keyed by the file opened, and the worker
    // reads this very file.
    let Ok((file, md)) = open_read_nofollow(src) else {
        qlog!("templates: a PDF is gone, or is no plain file; nothing made");
        return -1;
    };
    let hash = hash_hex(crate::bgwork_template::preview_hash(name, md.len(), md.mtime()));
    let (mut pw, mut ph) = (0, 0);
    store_page_size(&mut pw, &mut ph);
    let mut run = Run { src, cache, hash, file, md, limits: Limits { as_bytes: 0, cpu_secs: 0, mapped: false, file_len: 0 }, bands: 1, pw, ph, sess: None };
    let t0 = real_now_ms();
    let r = run.slice();
    if let Some(s) = run.sess.take() {
        s.end();
    }
    match r {
        Ok(rc) => rc,
        Err(Stop::Bad(why)) => {
            qlog!("templates: a PDF is no template Grimoire can use (%s); marked", why.as_str());
            run.remove_entries();
            if crate::bgwork_template::write_quiet(&bad_file(&run.cache, &run.hash), "") != 0 {
                return -1;
            }
            crate::bgwork_template::templates_cache_sweep();
            0
        }
        Err(Stop::Later(why)) => {
            qlog!("templates: a PDF's pages not made now (%s); the next look tries again (%d ms)", why.as_str(), real_now_ms().wrapping_sub(t0) as i32);
            -1
        }
        Err(Stop::Changed) => {
            qlog!("templates: a PDF changed while its pages were made; what was made of it is removed, and the next look makes them from the file as it is");
            run.remove_entries();
            -1
        }
    }
}

impl Run<'_> {
    fn slice(&mut self) -> Result<i32, Stop> {
        let known = match pdf_state(&self.cache, &self.hash) {
            PdfPages::Bad => return Ok(0),
            PdfPages::Known(n) => Some(n),
            PdfPages::Unknown => None,
        };
        // (all made or marked: no worker is started)
        if let Some(n) = known {
            if (1..=n).all(|k| page_done(&self.cache, &self.hash, k)) {
                return Ok(0);
            }
        }
        if crate::sandbox::session_broken() {
            return Err(Stop::Later("no sandboxed worker can be had".into()));
        }
        let size = self.md.len();
        if size == 0 {
            return Err(Stop::Bad("an empty file".into()));
        }
        if size > TEMPLATE_MAX_BYTES as u64 {
            return Err(Stop::Bad(format!("a PDF over {} MB", TEMPLATE_MAX_BYTES >> 20)));
        }
        self.room(PAGE_ROOM)?;
        let mapped = crate::pdfimport::can_map(self.file.as_raw_fd(), size);
        let band_mode = crate::pdfimport::read_mem_available().is_some_and(|a| a < s1::BAND_MODE_BELOW_AVAILABLE);
        self.bands = if band_mode { s1::BANDS } else { 1 };
        self.limits = Limits { as_bytes: worker_as_bytes(size, band_mode), cpu_secs: s1::SLICE_CPU_SECS, mapped, file_len: size };
        // the page count: the worker's probe, which every slice asks again (the slice's first worker)
        let n = self.count()?;
        after_page(0);
        if n > TEMPLATE_PDF_MAX_PAGES {
            return Err(Stop::Bad(format!("{n} pages, more than the {TEMPLATE_PDF_MAX_PAGES} a template PDF may have")));
        }
        if !self.same() {
            return Err(Stop::Changed);
        }
        if known != Some(n) {
            // (a count of another file, or none: the pages made under it are not this file's)
            self.remove_entries();
            if crate::bgwork_template::write_quiet(&pages_file(&self.cache, &self.hash), &format!("{n}\n")) != 0 {
                return Err(Stop::Later("the cache could not be written".into()));
            }
            qlog!("templates: a PDF of %d page(s) (%d KB): its pages are made", n as i32, (size >> 10) as i32);
        }
        let t0 = mono_ms();
        let max = crate::pdfimport::slice_pages();
        let (mut made, mut marked) = (0usize, 0usize);
        for k in 1..=n {
            if page_done(&self.cache, &self.hash, k) {
                continue;
            }
            // (a slice's limits are looked at before a page, never between the tries of one: a page whose worker
            // ends is tried again at once, so no slice ends on a strike a later slice would forget)
            if made + marked >= max || mono_ms().saturating_sub(t0) >= s1::SLICE_WALL_MS as u64 {
                qlog!("templates: a PDF's slice made %d page(s) and marked %d; the rest in the next", made as i32, marked as i32);
                return Ok(CODE_MORE);
            }
            // (the floor before each page's worker is asked; its picture's own size is asked again before it is kept)
            self.room(PAGE_ROOM)?;
            match self.page(k)? {
                PageEnd::Made => made += 1,
                PageEnd::Marked(why) => {
                    marked += 1;
                    if crate::bgwork_template::write_quiet(&page_file(&self.cache, &self.hash, k, "bad"), std::str::from_utf8(why).unwrap_or("")) != 0 {
                        return Err(Stop::Later("the cache could not be written".into()));
                    }
                    if why == DIED && self.dead_pages(n) >= DEAD_PAGES_MAX {
                        return Err(Stop::Bad(format!("{DEAD_PAGES_MAX} of its pages ended or stalled their worker twice")));
                    }
                }
            }
            after_page(k);
        }
        // every page made or marked: a PDF none of whose pages draws is no template
        if !(1..=n).any(|k| exists(&page_file(&self.cache, &self.hash, k, "pgm"))) {
            return Err(Stop::Bad("none of its pages could be drawn".into()));
        }
        qlog!("templates: a PDF's pages are done (%d made now, %d marked now)", made as i32, marked as i32);
        crate::bgwork_template::templates_cache_sweep();
        Ok(0)
    }

    /// Room on the storage for `need` bytes above the floor (fsview_room_for); unknown room is room.
    fn room(&self, need: u64) -> Result<(), Stop> {
        match crate::pdfimport::free_bytes() {
            Some(free) if !crate::fsview::fsview_room_for(free, need) => Err(Stop::Later("not enough room on the storage".into())),
            _ => Ok(()),
        }
    }

    /// Whether the file opened is unchanged and still the plain file at its name (a file written over, or another
    /// one put in its place, is not).
    fn same(&self) -> bool {
        let Ok(now) = self.file.metadata() else { return false };
        let same = |a: &std::fs::Metadata, b: &std::fs::Metadata| a.len() == b.len() && a.mtime() == b.mtime() && a.mtime_nsec() == b.mtime_nsec();
        same(&now, &self.md) && std::fs::symlink_metadata(self.src).is_ok_and(|at| at.is_file() && at.dev() == now.dev() && at.ino() == now.ino() && same(&at, &now))
    }

    /// Removes every cache entry of this PDF's (its count, its pages, its mark).
    fn remove_entries(&self) {
        let Ok(rd) = std::fs::read_dir(&self.cache) else { return };
        let (pages, count, bad) = (format!("{}-p", self.hash), format!("{}.pages", self.hash), format!("{}.bad", self.hash));
        for e in rd.flatten() {
            let n = e.file_name();
            let Some(n) = n.to_str() else { continue };
            if n.starts_with(&pages) || n == count || n == bad {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    /// How many of the n pages are marked because their workers ended or stalled on them.
    fn dead_pages(&self, n: u32) -> usize {
        (1..=n).filter(|&k| read_file_max(&page_file(&self.cache, &self.hash, k, "bad"), 16).is_some_and(|t| t == DIED)).count()
    }

    /// A worker on the file, and its page count.
    fn start(&mut self) -> Result<u32, StartErr> {
        // Every worker of the slice shares this open file and its offset: one that reads it whole starts from the
        // beginning, wherever the one before it stopped.
        // SAFETY: lseek on the descriptor this job opened.
        unsafe { libc::lseek(self.file.as_raw_fd(), 0, libc::SEEK_SET) };
        let mut s = Session::start(self.file.as_raw_fd(), self.md.len(), &self.limits)?;
        // (the import's test hooks, in headless and test builds: a page that hangs or crashes its worker)
        for (h, p) in crate::pdfimport::test_hooks() {
            s.hook(h, p);
        }
        // (what the sandbox holds, once a process, as the import logs it: the device check reads it)
        static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            qlog!("%s", &crate::sandbox::sandboxed_line());
        }
        let n = s.probe()?;
        self.sess = Some(s);
        Ok(n)
    }

    /// The page count, by a worker that stays for the pages: a worker that cannot be had is no try; one the file
    /// ends or stalls while it is opened is tried once more, and twice makes it no template.
    fn count(&mut self) -> Result<u32, Stop> {
        let mut tries = 0;
        loop {
            match self.start() {
                Ok(n) => return Ok(n),
                Err(StartErr::Sandbox(step)) => return Err(sandbox_refused(&step)),
                Err(StartErr::Worker(why)) => return Err(Stop::Later(format!("no worker ({why})"))),
                Err(StartErr::Password) => return Err(Stop::Bad("it needs a password".into())),
                Err(StartErr::Invalid | StartErr::Empty) => return Err(Stop::Bad("no PDF Grimoire can read".into())),
                Err(StartErr::Failed(why)) => {
                    tries += 1;
                    qlog!("templates: a PDF's worker could not open it (%s, %d of %d)", why, tries as i32, TRIES as i32);
                    if tries >= TRIES {
                        return Err(Stop::Bad("its worker ended or stalled opening it twice".into()));
                    }
                }
            }
        }
    }

    /// Page k drawn by the worker and stored, or marked: a page the worker cannot draw at once, one whose worker
    /// ends or stalls twice (a stall right after the Kindle slept is tried again with no strike, as in an import).
    fn page(&mut self, k: u32) -> Result<PageEnd, Stop> {
        let mut strikes = 0;
        let mut woke = false;
        loop {
            if self.sess.is_none() {
                match self.start() {
                    Ok(n) if Some(n) == self.counted() => {}
                    // (the file's count changed under the job: it is another file now)
                    Ok(_) => return Err(Stop::Changed),
                    Err(StartErr::Sandbox(step)) => return Err(sandbox_refused(&step)),
                    Err(StartErr::Worker(why)) => return Err(Stop::Later(format!("no worker ({why})"))),
                    Err(StartErr::Password) | Err(StartErr::Invalid) | Err(StartErr::Empty) => return Err(Stop::Changed),
                    Err(StartErr::Failed(why)) => {
                        strikes += 1;
                        qlog!("templates: page %d of a PDF: a worker could not open the file (%s, strike %d)", k as i32, why, strikes as i32);
                        if strikes >= TRIES {
                            return Ok(PageEnd::Marked(DIED));
                        }
                        continue;
                    }
                }
            }
            let out = match self.sess.as_mut() {
                Some(s) => s.render(k, self.pw, self.ph, self.bands),
                None => return Err(Stop::Later("no worker".into())),
            };
            match out {
                PageOutcome::Page(p) => {
                    // (nothing of a file that changed while its worker read it is kept)
                    if !self.same() {
                        return Err(Stop::Changed);
                    }
                    return self.store(k, p);
                }
                PageOutcome::Died(Death::TimeoutAfterWake) if !woke => {
                    self.sess = None;
                    woke = true;
                    qlog!("templates: page %d of a PDF: the worker stalled right after a wake; tried again (no strike)", k as i32);
                }
                PageOutcome::Died(d) => {
                    self.sess = None;
                    strikes += 1;
                    qlog!("templates: page %d of a PDF: the worker %s (strike %d)", k as i32, crate::pdfimport::death_text(&d).as_str(), strikes as i32);
                    if strikes >= TRIES {
                        return Ok(PageEnd::Marked(DIED));
                    }
                }
                PageOutcome::NoMemory => {
                    self.sess = None;
                    strikes += 1;
                    qlog!("templates: page %d of a PDF left the writer without memory for its pixels (strike %d)", k as i32, strikes as i32);
                    if strikes >= TRIES {
                        return Ok(PageEnd::Marked(DIED));
                    }
                }
                PageOutcome::Failed(e) => {
                    self.sess = None;
                    qlog!("templates: page %d of a PDF cannot be drawn (%s); marked", k as i32, &format!("{e:?}"));
                    return Ok(PageEnd::Marked(b""));
                }
                PageOutcome::Lied(why) => {
                    self.sess = None;
                    return Err(Stop::Bad(format!("its worker sent what the protocol does not allow ({why})")));
                }
            }
        }
    }

    /// The page count this file's cache holds.
    fn counted(&self) -> Option<u32> {
        match pdf_state(&self.cache, &self.hash) {
            PdfPages::Known(n) => Some(n),
            _ => None,
        }
    }

    /// Page k's pixels stored as a picture template's are: the page (PNG, gray or colour as it came), its key, its
    /// preview, in that order, above the floor. A page whose picture is over the folder's limit is marked.
    fn store(&self, k: u32, p: PagePixels) -> Result<PageEnd, Stop> {
        let pic = BgPicture { px: p.px, w: p.w, h: p.h, ch: p.ch };
        let Some(png) = crate::pdfimport::encode_png(&pic) else { return Err(Stop::Later("out of memory".into())) };
        if png.len() > TEMPLATE_MAX_BYTES {
            qlog!("templates: page %d of a PDF makes a picture over %d MB; marked", k as i32, (TEMPLATE_MAX_BYTES >> 20) as i32);
            return Ok(PageEnd::Marked(b""));
        }
        let (mut full, mut small) = (Canvas::default(), Canvas::default());
        let div = crate::bgwork_template::THUMB_DIV;
        if bgimage_luma(&pic, &mut full) != 0 || canvas_init(&mut small, self.pw / div, self.ph / div) != 0 {
            canvas_free(&mut full);
            return Err(Stop::Later("out of memory".into()));
        }
        canvas_downsample_keep_lines(&mut small, &full, 0.6);
        canvas_free(&mut full);
        let need = png.len() as u64 + (small.w as u64 * small.h as u64) + 4096;
        if let Err(e) = self.room(need) {
            canvas_free(&mut small);
            return Err(e);
        }
        let f = |ext| page_file(&self.cache, &self.hash, k, ext);
        let key = cformat!("%016llx %d %d\n", bgimage_content_hash(&png), pic.w, pic.h);
        // (the picture, then its key, then the preview that says the page is made)
        let wrote = write_file_atomic(&f("png"), &png) == 0 && crate::bgwork_template::write_quiet(&f("key"), &key) == 0 && store_write_pgm(&f("pgm"), &small) == 0;
        canvas_free(&mut small);
        if !wrote {
            for ext in ["pgm", "key", "png"] {
                let _ = std::fs::remove_file(f(ext));
            }
            return Err(Stop::Later("the cache could not be written".into()));
        }
        Ok(PageEnd::Made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A .pages file is its count and a line break, and nothing else: 1 to the cap, no sign, no leading zero.
    #[test]
    fn the_page_count_is_read_strictly() {
        assert_eq!(parse_pages(b"1\n"), Some(1));
        assert_eq!(parse_pages(b"7\n"), Some(7));
        assert_eq!(parse_pages(format!("{TEMPLATE_PDF_MAX_PAGES}\n").as_bytes()), Some(TEMPLATE_PDF_MAX_PAGES));
        for bad in [&b""[..], b"\n", b"7", b"07\n", b"0\n", b"-1\n", b"+7\n", b" 7\n", b"7 \n", b"7\n\n", b"7\r\n", b"1000\n", b"x\n"] {
            assert_eq!(parse_pages(bad), None, "{:?}", String::from_utf8_lossy(bad));
        }
        assert_eq!(parse_pages(format!("{}\n", TEMPLATE_PDF_MAX_PAGES + 1).as_bytes()), None);
    }

    // A page's tile name keeps its number whole, however long the file's name: the name is cut at a character.
    #[test]
    fn a_page_name_keeps_its_number() {
        crate::i18n::i18n_set("en");
        assert_eq!(page_name("Planner", 3), "Planner p. 3");
        let long = "\u{e9}".repeat(80);
        let n = page_name(&long, 12);
        assert!(n.ends_with(" p. 12") && n.len() < STORE_NAME_LEN, "{n}");
        assert!(n.trim_end_matches(" p. 12").chars().all(|c| c == '\u{e9}'));
    }
}
