//! The PDF worker and its protocol (2.1, DESIGN.md 3.7 and 3.9; areas/pdf.md 3 and 4.10). Two halves of one
//! file, so they agree:
//!
//! - the worker side (`worker_run`): hayro reads the PDF from fd 3 (mapped private and read only, or read
//!   whole when the device refuses the mapping), takes commands on fd 5 and answers on fd 4 with records and
//!   pixels. It runs under sandbox.rs's steps, on one thread, and never logs.
//! - the writer side (`Session`): starts a worker, speaks the grammar, validates every record and every
//!   pixel band strictly (a worker that lies can only lie inside it), and times each page.
//!
//! Commands (writer to worker, one line each):
//!   `PROBE`                         the page count, then the links and outline (lane L6: a stub today)
//!   `R <page> <w> <h> <bands>`      page n (from 1) fitted and centred on a w x h white page, in 1 band
//!                                   (whole) or in that many bands
//!   `H <hook> <page>`               a test hook (headless and test builds only: `crash`, `hang`, `alloc`, `trap`)
//!   `Q`                             end
//!
//! Records (worker to writer, text lines of a fixed grammar, at most 4 MiB a job besides the pixels):
//!   `B <version> <protocol>`        the build, first line
//!   `S <0|1>`                       whether the sandbox steps ran
//!   `P <pages>`                     the probe's page count
//!   `K`                             the probe is done
//!   `G <page> <y0> <w> <h> <ch>`    then w*h*ch bytes: a band of a page (rows y0 .. y0+h), 1 channel when
//!                                   every pixel of the band has r = g = b, else 3 (RGB), top row first
//!   `D <page>`                      the page is done
//!   `E <code> [<detail>]`           an error: password, invalid, empty, render, panic, sandbox <step>,
//!                                   sigsys <call>, protocol
//!   (`L`, `Q`, `O` and `F` records, the links, outline and names of lane L6, are not in the grammar yet: the
//!   writer refuses a line it does not know.)

use crate::sandbox::*;

/// A page's pixels are at most this many on a side (what bgimage takes of a page picture).
pub const PAGE_PX_MAX: i32 = 6000;

/// How long a worker may go without its next record before it is killed (3.7, Timers): the table's number, or
/// (headless and test builds) GRIMOIRE_PDF_WATCHDOG_MS.
pub fn page_watchdog_ms() -> u64 {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_PDF_WATCHDOG_MS").ok().and_then(|v| v.parse::<u64>().ok()).filter(|&v| v > 0) {
        return v;
    }
    s1::PAGE_WATCHDOG_MS as u64
}

/// The records of one job, besides pixels.
pub const RECORDS_MAX_BYTES: usize = 4 << 20;

/// A page is stored gray when no pixel's channels differ by more than this (8-bit levels), colour (8-bit
/// RGB) otherwise. Planners drawn in black and gray, and scans with a faint cast, stay gray (a third of the
/// bytes, and what the Scribe shows); any real colour (a highlighter, a blue heading) makes the page RGB.
/// Chosen by looking at hayro's anti-aliasing of gray content, which keeps r = g = b, and at JPEG noise in
/// gray scans, which stays within 3 or 4 levels: 6 leaves room for both and is far below any colour a person
/// would call one (a pale yellow paper, 255 250 235, spreads 20).
pub const GRAY_TOLERANCE: u8 = 6;

// ---- the fit (areas/pdf.md 4.10) ----

/// How a PDF page of `pw` x `ph` points (its render dimensions: crop box within media box, turned by
/// /Rotate) lands on a `page_w` x `page_h` Scribe page: scale s = min(page_w / pw, page_h / ph), the rendered
/// size trunc(pw s) x trunc(ph s), centred by integer offsets, as KOReader's draw_whole_page centres it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub scale: f64,
    pub ox: i32,
    pub oy: i32,
    pub rw: i32,
    pub rh: i32,
}

pub fn fit_page(pw: f64, ph: f64, page_w: i32, page_h: i32) -> Fit {
    let (pw, ph) = (pw.max(1.0), ph.max(1.0));
    let scale = (page_w as f64 / pw).min(page_h as f64 / ph);
    let rw = ((pw * scale) as i32).clamp(1, page_w);
    let rh = ((ph * scale) as i32).clamp(1, page_h);
    Fit { scale, ox: (page_w - rw) / 2, oy: (page_h - rh) / 2, rw, rh }
}

// ---- the grammar (shared) ----

/// Why the writer refuses what a worker sent: the worker lied or broke the grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation(pub &'static str);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrCode {
    Password,
    Invalid,
    Empty,
    Render,
    Panic,
    Sandbox(String),
    Sigsys(u32),
    Protocol,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rec {
    Build(String),
    Sandboxed(bool),
    Pages(u32),
    ProbeDone,
    Band { page: u32, y0: u32, w: u32, h: u32, ch: u32 },
    PageDone(u32),
    Err(ErrCode),
}

fn number(t: &str) -> Option<u32> {
    if t.is_empty() || t.len() > 9 || !t.bytes().all(|c| c.is_ascii_digit()) || (t.len() > 1 && t.starts_with('0')) {
        return None;
    }
    t.parse().ok()
}

/// Parses one record line strictly: ASCII, single spaces, decimal numbers without sign or leading zero.
pub fn parse_record(line: &[u8]) -> Result<Rec, Violation> {
    let bad = Violation("record");
    if !line.iter().all(|&c| (0x20..0x7f).contains(&c)) {
        return Err(bad);
    }
    let s = std::str::from_utf8(line).map_err(|_| Violation("record"))?;
    let t: Vec<&str> = s.split(' ').collect();
    let num = |i: usize| t.get(i).and_then(|x| number(x)).ok_or(Violation("record"));
    match (t[0], t.len()) {
        ("B", 3) => Ok(Rec::Build(format!("{} {}", t[1], t[2]))),
        ("S", 2) => match t[1] {
            "0" => Ok(Rec::Sandboxed(false)),
            "1" => Ok(Rec::Sandboxed(true)),
            _ => Err(bad),
        },
        ("P", 2) => Ok(Rec::Pages(num(1)?)),
        ("K", 1) => Ok(Rec::ProbeDone),
        ("G", 6) => Ok(Rec::Band { page: num(1)?, y0: num(2)?, w: num(3)?, h: num(4)?, ch: num(5)? }),
        ("D", 2) => Ok(Rec::PageDone(num(1)?)),
        ("E", 2 | 3) => Ok(Rec::Err(match (t[1], t.get(2)) {
            ("password", None) => ErrCode::Password,
            ("invalid", None) => ErrCode::Invalid,
            ("empty", None) => ErrCode::Empty,
            ("render", None) => ErrCode::Render,
            ("panic", None) => ErrCode::Panic,
            ("protocol", None) => ErrCode::Protocol,
            ("sandbox", Some(step)) if step.len() <= 16 && step.bytes().all(|c| c.is_ascii_lowercase()) => ErrCode::Sandbox(step.to_string()),
            ("sigsys", Some(n)) => ErrCode::Sigsys(number(n).ok_or(bad)?),
            _ => return Err(Violation("record")),
        })),
        _ => Err(bad),
    }
}

// ---- gray or colour ----

/// A band's pixels, RGBA premultiplied over white (so opaque): true when every pixel has r = g = b.
fn exact_gray(rgba: &[u8]) -> bool {
    rgba.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2])
}

/// RGB bytes whose pixels all differ by no more than `tol` between channels, as luma.
pub fn settle_gray(rgb: &[u8], tol: u8) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.try_reserve_exact(rgb.len() / 3).ok()?;
    for p in rgb.chunks_exact(3) {
        let (mx, mn) = (p[0].max(p[1]).max(p[2]), p[0].min(p[1]).min(p[2]));
        if mx - mn > tol {
            return None;
        }
        out.push(luma(p[0], p[1], p[2]));
    }
    Some(out)
}

/// The luma of a pixel (the weights hayro's own gray conversion in the S1 probe used).
pub fn luma(r: u8, g: u8, b: u8) -> u8 {
    ((r as u32 * 77 + g as u32 * 151 + b as u32 * 28) >> 8) as u8
}

// ====================================================================================================
// The worker side
// ====================================================================================================

struct Mapped {
    ptr: *const u8,
    len: usize,
}
// SAFETY: a read-only private mapping that lives until the process ends and is never written.
unsafe impl Send for Mapped {}
unsafe impl Sync for Mapped {}
impl AsRef<[u8]> for Mapped {
    fn as_ref(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        // SAFETY: ptr..ptr+len is the mapping made in map_source, read only, never unmapped.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

fn say(s: &str) {
    let b = s.as_bytes();
    write_all(FD_OUT, b);
}

fn write_all(fd: i32, mut b: &[u8]) {
    while !b.is_empty() {
        // SAFETY: write from a slice we own.
        let n = unsafe { libc::write(fd, b.as_ptr() as *const libc::c_void, b.len()) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            // the writer is gone: the parent-death signal is on its way
            unsafe { libc::_exit(6) };
        }
        b = &b[n as usize..];
    }
}

fn read_line_fd(fd: i32) -> Option<String> {
    let mut line = Vec::new();
    loop {
        let mut b = [0u8; 1];
        // SAFETY: read one byte into a buffer we own.
        let n = unsafe { libc::read(fd, b.as_mut_ptr() as *mut libc::c_void, 1) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            return None;
        }
        if b[0] == b'\n' {
            return String::from_utf8(line).ok();
        }
        if line.len() > 200 {
            return None;
        }
        line.push(b[0]);
    }
}

/// The PDF's bytes: mapped private and read only when the writer says so (and the map works), else read whole
/// through fd 3 (read(2) is on the allow-list).
fn map_source(l: &Limits, file_len: usize) -> Option<hayro::hayro_syntax::PdfData> {
    if l.mapped && file_len > 0 {
        // SAFETY: a private, read-only mapping of the descriptor the writer gave us, of the length it gave us
        // (a file truncated under it raises SIGBUS here, in the worker only: a failed unit).
        let p = unsafe { libc::mmap(std::ptr::null_mut(), file_len, libc::PROT_READ, libc::MAP_PRIVATE, FD_SOURCE, 0) };
        if p != libc::MAP_FAILED {
            return Some(std::sync::Arc::new(Mapped { ptr: p as *const u8, len: file_len }).into());
        }
    }
    if file_len as u64 > s1::PDF_MAX_READ_WHOLE && !l.mapped {
        return None;
    }
    let mut v = Vec::new();
    v.try_reserve_exact(file_len).ok()?;
    v.resize(file_len, 0);
    let mut got = 0;
    while got < file_len {
        // SAFETY: read into the part of v not yet filled.
        let n = unsafe { libc::read(FD_SOURCE, v[got..].as_mut_ptr() as *mut libc::c_void, file_len - got) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            break;
        }
        got += n as usize;
    }
    v.truncate(got);
    Some(v.into())
}

// The rendering state a worker keeps between pages (a context and a pixmap, grown and reused).
struct Renderer {
    ctx: hayro::vello_cpu::RenderContext,
    pix: hayro::vello_cpu::Pixmap,
    res: hayro::vello_cpu::Resources,
}

impl Renderer {
    fn new() -> Self {
        Renderer { ctx: hayro::vello_cpu::RenderContext::new(8, 8), pix: hayro::vello_cpu::Pixmap::new(8, 8), res: hayro::vello_cpu::Resources::default() }
    }
}

/// What a worker does on a hook page (headless and test builds only; the Kindle build has none).
#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "kindle", allow(dead_code))]
struct Hooks {
    crash: u32,
    hang: u32,
    alloc: u32,
    /// makes a call outside the allow-list (getpid), to see the sandbox kill it
    trap: u32,
}

/// Warms up what would touch the filesystem or entropy after the chroot (3.7, step 1): std's CPU feature
/// detection, cached on first use.
pub fn warm_up() {
    #[cfg(target_arch = "x86_64")]
    let _ = std::is_x86_feature_detected!("avx2");
    #[cfg(target_arch = "aarch64")]
    let _ = std::arch::is_aarch64_feature_detected!("neon");
}

fn white_gray_rows(w: usize, rows: usize) -> Vec<u8> {
    vec![255u8; w * rows]
}

/// Renders page `n` of the PDF (fitted into w x h) in `bands` bands, answering G records.
fn render_page<'a>(
    pdf_pages: &'a [hayro::hayro_syntax::page::Page<'a>],
    cache: &hayro::RenderCache<'a>,
    rr: &mut Renderer,
    n: usize,
    w: i32,
    h: i32,
    bands: u32,
) -> bool {
    use hayro::hayro_interpret::util::TransformExt;
    use hayro::kurbo::Affine;
    use hayro::vello_cpu::{color::palette::css::WHITE, RasterizerSettings, TargetInit};
    let Some(page) = pdf_pages.get(n.wrapping_sub(1)) else {
        return false;
    };
    let (pw, ph) = page.render_dimensions();
    let f = fit_page(pw as f64, ph as f64, w, h);
    let fit = Affine::translate((f.ox as f64, f.oy as f64)) * Affine::scale(f.scale) * page.initial_transform(true).to_kurbo();
    let settings = hayro::hayro_interpret::InterpreterSettings::default();
    let band_h = (h as u32).div_ceil(bands.max(1));
    let mut y0 = 0u32;
    while y0 < h as u32 {
        let bh = band_h.min(h as u32 - y0);
        let tr = Affine::translate((0.0, -(y0 as f64))) * fit;
        rr.ctx.reset_and_resize(w as u16, bh as u16);
        rr.pix.resize(w as u16, bh as u16);
        hayro::render_into(page, cache, &settings, &hayro::RenderSettings::default(), &mut rr.ctx, tr);
        rr.ctx.flush();
        rr.ctx.render_with(&mut rr.pix, &mut rr.res, RasterizerSettings { target_init: TargetInit::Clear(WHITE), ..Default::default() });
        let src = rr.pix.data_as_u8_slice();
        let gray = exact_gray(src);
        let ch = if gray { 1 } else { 3 };
        say(&format!("G {} {} {} {} {}\n", n, y0, w, bh, ch));
        // rows in chunks, so the worker holds no copy of the band beside the pixmap
        let row = w as usize * 4;
        let chunk_rows = 64usize;
        let mut out: Vec<u8> = Vec::with_capacity(chunk_rows * w as usize * ch);
        for rows in src.chunks(row * chunk_rows) {
            out.clear();
            if gray {
                out.extend(rows.chunks_exact(4).map(|p| p[0]));
            } else {
                for p in rows.chunks_exact(4) {
                    out.extend_from_slice(&p[..3]);
                }
            }
            write_all(FD_OUT, &out);
        }
        y0 += bh;
    }
    true
}

/// The worker's loop after the sandbox: map the PDF, answer commands. Returns the exit status.
pub fn worker_run(l: &Limits) -> i32 {
    let Some(data) = map_source(l, l.file_len as usize) else {
        say("E invalid\n");
        return 1;
    };
    let pdf = match hayro::hayro_syntax::Pdf::new(data) {
        Ok(p) => p,
        Err(hayro::hayro_syntax::LoadPdfError::Decryption(hayro::hayro_syntax::DecryptionError::PasswordProtected)) => {
            say("E password\n");
            return 1;
        }
        Err(_) => {
            say("E invalid\n");
            return 1;
        }
    };
    let pages = pdf.pages();
    let cache = hayro::RenderCache::new();
    let mut rr = Renderer::new();
    #[cfg_attr(feature = "kindle", allow(unused_mut))]
    let mut hooks = Hooks::default();
    while let Some(cmd) = read_line_fd(FD_IN) {
        let t: Vec<&str> = cmd.split(' ').collect();
        match t.as_slice() {
            ["PROBE"] => {
                if pages.is_empty() {
                    say("E empty\n");
                    return 1;
                }
                say(&format!("P {}\n", pages.len()));
                probe_links(&pdf, pages);
                say("K\n");
            }
            ["R", p, w, h, b] => {
                let (Some(p), Some(w), Some(h), Some(b)) = (number(p), number(w), number(h), number(b)) else {
                    say("E protocol\n");
                    return 1;
                };
                if w == 0 || h == 0 || w as i32 > PAGE_PX_MAX || h as i32 > PAGE_PX_MAX || b == 0 || b > 64 {
                    say("E protocol\n");
                    return 1;
                }
                run_hooks(&hooks, p);
                if !render_page(pages, &cache, &mut rr, p as usize, w as i32, h as i32, b) {
                    say("E render\n");
                    return 1;
                }
                say(&format!("D {p}\n"));
            }
            #[cfg(not(feature = "kindle"))]
            ["H", hook, p] => {
                let Some(p) = number(p) else {
                    say("E protocol\n");
                    return 1;
                };
                match *hook {
                    "crash" => hooks.crash = p,
                    "hang" => hooks.hang = p,
                    "alloc" => hooks.alloc = p,
                    "trap" => hooks.trap = p,
                    _ => {
                        say("E protocol\n");
                        return 1;
                    }
                }
            }
            ["Q"] => return 0,
            _ => {
                say("E protocol\n");
                return 1;
            }
        }
    }
    0
}

#[cfg(not(feature = "kindle"))]
fn run_hooks(h: &Hooks, page: u32) {
    if h.crash == page {
        std::process::abort();
    }
    if h.hang == page {
        // a busy loop: a sleeping call is not on the allow-list
        let mut x = 0u64;
        loop {
            x = std::hint::black_box(x.wrapping_add(1));
        }
    }
    if h.trap == page {
        // SAFETY: getpid takes no arguments; it is not on the allow-list, so under the filter it never returns.
        unsafe { libc::getpid() };
    }
    if h.alloc == page {
        // more than any address-space cap: the allocation fails and the worker aborts
        let v: Vec<u8> = Vec::with_capacity(1 << 31);
        std::hint::black_box(&v);
        let mut big = Vec::new();
        for _ in 0..64 {
            big.push(vec![1u8; 64 << 20]);
        }
        std::hint::black_box(&big);
    }
}
#[cfg(feature = "kindle")]
fn run_hooks(_h: &Hooks, _page: u32) {}

/// The links, outline and names of the PDF (lane L6). MERGE-ME(L6): a stub that records nothing. L6 fills it:
/// it writes `L`, `Q`, `O` and `F` records here (and adds them to `parse_record`), in the probe's turn, so the
/// writer (pdfimport.rs `slice0_records`) can keep them in the work folder's `links.part` and `names.part`.
fn probe_links(_pdf: &hayro::hayro_syntax::Pdf, _pages: &[hayro::hayro_syntax::page::Page<'_>]) {}

// ====================================================================================================
// The writer's side
// ====================================================================================================

/// How a page ended.
#[derive(Debug)]
pub enum PageOutcome {
    /// its pixels: `ch` 1 (gray) or 3 (RGB), rows of w * ch bytes
    Page(PagePixels),
    /// the worker died or went silent on this page: counts a strike against the page
    Died(Death),
    /// the worker named a PDF problem (it cannot render this page)
    Failed(ErrCode),
    /// the worker lied: refused
    Lied(&'static str),
}

#[derive(Debug)]
pub struct PagePixels {
    pub w: i32,
    pub h: i32,
    pub ch: i32,
    pub px: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Death {
    /// no record for PAGE_WATCHDOG_MS
    Timeout,
    /// the pipe ended: exit status, as waitpid gave it
    Gone(i32),
    /// it said which call killed it (the seccomp reporter)
    Sigsys(u32),
    Panic,
}

/// Why a session could not start or probe.
#[derive(Debug, PartialEq, Eq)]
pub enum StartErr {
    /// the sandbox refused a step (the name): not the file's fault
    Sandbox(String),
    /// a worker that could not start, or answered another build or protocol: also not the file's fault
    Worker(&'static str),
    Password,
    /// no page tree, or nothing parsed
    Invalid,
    Empty,
    /// a worker lie, a death or a stall while the file was being opened: counts as a failed try
    Failed(&'static str),
}

pub struct Session {
    w: Worker,
    rec_bytes: usize,
    pub sandboxed: bool,
    pub pages: u32,
}

impl Session {
    /// Starts a worker on the PDF and checks its handshake: the build line, then the sandbox line or
    /// `E sandbox <step>`.
    pub fn start(source_fd: i32, file_len: u64, limits: &Limits) -> Result<Session, StartErr> {
        let limits = Limits { file_len, ..limits.clone() };
        let mut w = match Worker::spawn(WorkKind::Pdf, Some(source_fd), &limits) {
            Ok(w) => w,
            Err(SpawnErr::Sandbox(step)) => return Err(StartErr::Sandbox(step.to_string())),
            Err(SpawnErr::Start) => return Err(StartErr::Worker("start")),
        };
        let deadline = mono_ms() + 15_000;
        let sandboxed;
        // first record: the build
        let line = w.read_line(512, deadline).map_err(|_| StartErr::Worker("no handshake"))?;
        match parse_record(&line) {
            Ok(Rec::Build(b)) if b == build_id() => {}
            Ok(Rec::Build(_)) => return Err(StartErr::Worker("build mismatch")),
            _ => return Err(StartErr::Worker("handshake")),
        }
        let line = w.read_line(512, deadline).map_err(|_| StartErr::Worker("no sandbox line"))?;
        match parse_record(&line) {
            Ok(Rec::Sandboxed(on)) => sandboxed = on,
            Ok(Rec::Err(ErrCode::Sandbox(step))) => {
                w.kill_and_reap();
                return Err(StartErr::Sandbox(step));
            }
            _ => return Err(StartErr::Worker("handshake")),
        }
        Ok(Session { w, rec_bytes: 0, sandboxed, pages: 0 })
    }

    /// The probe: the page count (the writer checks it against the caps).
    pub fn probe(&mut self) -> Result<u32, StartErr> {
        if !self.w.send("PROBE\n") {
            return Err(StartErr::Failed("send"));
        }
        let mut pages = None;
        loop {
            let deadline = mono_ms() + page_watchdog_ms();
            let line = match self.w.read_line(4096, deadline) {
                Ok(l) => l,
                Err(_) => {
                    self.w.kill_and_reap();
                    return Err(StartErr::Failed("the worker died or stalled opening the file"));
                }
            };
            self.rec_bytes += line.len() + 1;
            if self.rec_bytes > RECORDS_MAX_BYTES {
                self.w.kill_and_reap();
                return Err(StartErr::Failed("too many records"));
            }
            match parse_record(&line) {
                Ok(Rec::Pages(n)) if pages.is_none() => pages = Some(n),
                Ok(Rec::ProbeDone) => {
                    let Some(n) = pages else { return Err(StartErr::Failed("no page count")) };
                    self.pages = n;
                    return Ok(n);
                }
                Ok(Rec::Err(ErrCode::Password)) => return Err(StartErr::Password),
                Ok(Rec::Err(ErrCode::Invalid)) => return Err(StartErr::Invalid),
                Ok(Rec::Err(ErrCode::Empty)) => return Err(StartErr::Empty),
                Ok(Rec::Err(ErrCode::Sandbox(s))) => return Err(StartErr::Sandbox(s)),
                _ => {
                    self.w.kill_and_reap();
                    return Err(StartErr::Failed("unexpected record"));
                }
            }
        }
    }

    /// Test hooks (headless and test builds): forwarded to the worker before its first page.
    pub fn hook(&mut self, name: &str, page: u32) -> bool {
        self.w.send(&format!("H {name} {page}\n"))
    }

    /// Renders page `page` (from 1) fitted into w x h in `bands` bands, and assembles its pixels. The
    /// band records are checked as they come: the page, the next row, the width, the channels.
    pub fn render(&mut self, page: u32, w: i32, h: i32, bands: u32) -> PageOutcome {
        if !self.w.send(&format!("R {page} {w} {h} {bands}\n")) {
            let st = self.w.kill_and_reap();
            return PageOutcome::Died(Death::Gone(st));
        }
        let (wu, hu) = (w as usize, h as usize);
        let mut px: Vec<u8> = Vec::new();
        let mut ch = 1usize;
        if px.try_reserve_exact(wu * hu).is_err() {
            self.w.kill_and_reap();
            return PageOutcome::Lied("out of memory");
        }
        px.resize(wu * hu, 255);
        let mut next_y = 0usize;
        loop {
            let deadline = mono_ms() + page_watchdog_ms();
            let line = match self.w.read_line(512, deadline) {
                Ok(l) => l,
                Err(ReadErr::Timeout) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Died(Death::Timeout);
                }
                Err(ReadErr::End) => {
                    let st = self.w.kill_and_reap();
                    return PageOutcome::Died(Death::Gone(st));
                }
                Err(ReadErr::TooLong) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Lied("a line too long");
                }
            };
            self.rec_bytes += line.len() + 1;
            if self.rec_bytes > RECORDS_MAX_BYTES {
                self.w.kill_and_reap();
                return PageOutcome::Lied("too many records");
            }
            match parse_record(&line) {
                Ok(Rec::Band { page: p, y0, w: bw, h: bh, ch: bch }) => {
                    let ok = p == page
                        && y0 as usize == next_y
                        && bw as i32 == w
                        && bh >= 1
                        && next_y + bh as usize <= hu
                        && (bch == 1 || bch == 3);
                    if !ok {
                        self.w.kill_and_reap();
                        return PageOutcome::Lied("a band outside its page");
                    }
                    let n = wu * bh as usize * bch as usize;
                    let mut band = Vec::new();
                    if band.try_reserve_exact(n).is_err() {
                        self.w.kill_and_reap();
                        return PageOutcome::Lied("out of memory");
                    }
                    band.resize(n, 0u8);
                    match self.w.read_exact(&mut band, deadline) {
                        Ok(()) => {}
                        Err(ReadErr::Timeout) => {
                            self.w.kill_and_reap();
                            return PageOutcome::Died(Death::Timeout);
                        }
                        Err(_) => {
                            let st = self.w.kill_and_reap();
                            return PageOutcome::Died(Death::Gone(st));
                        }
                    }
                    if bch == 3 && ch == 1 {
                        // the first coloured band: the page becomes RGB (what came before, gray, is copied out)
                        let mut rgb = Vec::new();
                        if rgb.try_reserve_exact(wu * hu * 3).is_err() {
                            self.w.kill_and_reap();
                            return PageOutcome::Lied("out of memory");
                        }
                        rgb.resize(wu * hu * 3, 255u8);
                        for (i, &g) in px[..next_y * wu].iter().enumerate() {
                            rgb[3 * i] = g;
                            rgb[3 * i + 1] = g;
                            rgb[3 * i + 2] = g;
                        }
                        px = rgb;
                        ch = 3;
                    }
                    let at = next_y * wu;
                    if ch == 3 && bch == 1 {
                        for (i, &g) in band.iter().enumerate() {
                            px[3 * (at + i)] = g;
                            px[3 * (at + i) + 1] = g;
                            px[3 * (at + i) + 2] = g;
                        }
                    } else {
                        px[at * ch..at * ch + n].copy_from_slice(&band);
                    }
                    next_y += bh as usize;
                }
                Ok(Rec::PageDone(p)) if p == page => {
                    if next_y != hu {
                        self.w.kill_and_reap();
                        return PageOutcome::Lied("a page with rows missing");
                    }
                    let (px, ch) = if ch == 3 {
                        match settle_gray(&px, GRAY_TOLERANCE) {
                            Some(g) => (g, 1),
                            None => (px, 3),
                        }
                    } else {
                        (px, 1)
                    };
                    return PageOutcome::Page(PagePixels { w, h, ch: ch as i32, px });
                }
                Ok(Rec::Err(ErrCode::Sigsys(n))) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Died(Death::Sigsys(n));
                }
                Ok(Rec::Err(ErrCode::Panic)) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Died(Death::Panic);
                }
                Ok(Rec::Err(e)) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Failed(e);
                }
                _ => {
                    self.w.kill_and_reap();
                    return PageOutcome::Lied("an unexpected record");
                }
            }
        }
    }

    /// Ends the worker (Q), killing it if it lingers.
    pub fn end(mut self) {
        let _ = self.w.send("Q\n");
        self.w.finish(2000);
    }

    /// Kills the worker now.
    pub fn kill(&mut self) -> i32 {
        self.w.kill_and_reap()
    }
}

/// A page that is all white, gray (a page whose worker died twice, or which could not be read).
pub fn blank_page(w: i32, h: i32) -> PagePixels {
    PagePixels { w, h, ch: 1, px: white_gray_rows(w as usize, h as usize) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_is_koreaders_centred_fit() {
        // an A4 portrait page (595.28 x 841.89 pt) on a 1860 x 2480 Scribe page: the height decides
        let f = fit_page(595.28, 841.89, 1860, 2480);
        assert!((f.scale - 2480.0 / 841.89).abs() < 1e-9);
        assert_eq!((f.rh, f.rw), (2480, (595.28 * f.scale) as i32));
        assert_eq!(f.oy, 0);
        assert_eq!(f.ox, (1860 - f.rw) / 2);
        // landscape: fitted as it is (letterboxed), never turned
        let f = fit_page(842.0, 595.0, 1860, 2480);
        assert_eq!(f.rw, 1860);
        assert!(f.rh < 2480 && f.oy == (2480 - f.rh) / 2);
        // no area counts as 1 x 1
        let f = fit_page(0.0, 0.0, 100, 100);
        assert_eq!((f.rw, f.rh), (100, 100));
    }

    #[test]
    fn records_parse_strictly() {
        assert_eq!(parse_record(b"B 2.1.0 1"), Ok(Rec::Build("2.1.0 1".into())));
        assert_eq!(parse_record(b"P 380"), Ok(Rec::Pages(380)));
        assert_eq!(parse_record(b"G 3 0 1860 620 3"), Ok(Rec::Band { page: 3, y0: 0, w: 1860, h: 620, ch: 3 }));
        assert_eq!(parse_record(b"D 3"), Ok(Rec::PageDone(3)));
        assert_eq!(parse_record(b"E sandbox chroot"), Ok(Rec::Err(ErrCode::Sandbox("chroot".into()))));
        assert_eq!(parse_record(b"E sigsys 5"), Ok(Rec::Err(ErrCode::Sigsys(5))));
        assert_eq!(parse_record(b"E password"), Ok(Rec::Err(ErrCode::Password)));
        for bad in [
            &b"P -1"[..],
            b"P 01",
            b"P 1 2",
            b"P  1",
            b"P 1234567890",
            b"G 1 0 10 10",
            b"X 1",
            b"L 1 2 3 4 5 6",
            b"E sandbox",
            b"E sandbox Chroot",
            b"E sigsys x",
            b"D \x01",
            b"",
            "P \u{e9}".as_bytes(),
        ] {
            assert!(parse_record(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn gray_pages_settle_to_gray_and_colour_pages_stay() {
        // all within the tolerance: luma
        let mut px = Vec::new();
        for i in 0..100u8 {
            px.extend_from_slice(&[i, i + GRAY_TOLERANCE.min(255 - i), i]);
        }
        let g = settle_gray(&px, GRAY_TOLERANCE).expect("within tolerance");
        assert_eq!(g.len(), 100);
        // one pixel one level past it: colour
        let mut px2 = px.clone();
        px2[0] = 0;
        px2[1] = GRAY_TOLERANCE + 1;
        px2[2] = 0;
        assert!(settle_gray(&px2, GRAY_TOLERANCE).is_none());
        // white and black exactly
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(0, 0, 0), 0);
        assert_eq!(luma(100, 100, 100), 100);
        // a pale yellow paper is colour, not gray
        assert!(settle_gray(&[255, 250, 235], GRAY_TOLERANCE).is_none());
        // a faint JPEG cast is gray
        assert!(settle_gray(&[200, 203, 201], GRAY_TOLERANCE).is_some());
    }

    #[test]
    fn exact_gray_means_equal_channels() {
        assert!(exact_gray(&[1, 1, 1, 255, 9, 9, 9, 255]));
        assert!(!exact_gray(&[1, 1, 2, 255]));
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("pdfwork.rs").contains('\u{2014}'));
    }
}
