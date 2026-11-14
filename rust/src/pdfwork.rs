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
//!   `PROBE`                         the page count
//!   `LINKS <w> <h>`                 the links, outline names and the left-out counts of the PDF, the links in the
//!                                   pixels of a w x h Scribe page (the same fit as `R`); asked once, after PROBE
//!   `R <page> <w> <h> <bands>`      page n (from 1) fitted and centred on a w x h white page, in 1 band
//!                                   (whole) or in that many bands
//!   `H <hook> <page>`               a test hook (headless and test builds only: `crash`, `hang`, `alloc`, `trap`)
//!   `Q`                             end
//!
//! Records (worker to writer, text lines of a fixed grammar, at most 4 MiB a job besides the pixels):
//!   `B <version> <protocol>`        the build, first line
//!   `S <0|1>`                       1 when this build has the sandbox steps (each held, where the sandbox is
//!                                   required); the writer takes 0 only where the sandbox is not required
//!   `P <pages>`                     the probe's page count
//!   `K`                             the probe is done
//!   `G <page> <y0> <w> <h> <ch>`    then w*h*ch bytes: a band of a page (rows y0 .. y0+h), 1 channel when
//!                                   every pixel of the band has r = g = b, else 3 (RGB), top row first
//!   `D <page>`                      the page is done
//!   `E <code> [<detail>]`           an error: password, invalid, empty, render, panic, sandbox <step>,
//!                                   sigsys <call>, protocol
//!   `L <src> <dst> <x> <y> <w> <h> <i>`  a link: pages from 1, its area in page pixels (signed, w and h at least
//!                                   1), i its place among the page's links (the order the PDF drew them)
//!   `Q <x0> <y0> .. <x3> <y3>`      the quad of the link just before it, corners in 1/16 px, signed
//!   `O <page> <hex>`                an outline title (UTF-8, hex) that names this page: at most one a page
//!   `F <uri> <other> <dead> <capped>`  what the walk left out: web links, other actions, links without a
//!                                   place to go or an area, links past the worker's cap

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
    // trunc, as KOReader's integer dimensions, after a hair of slack: the side that decides the scale comes
    // out at exactly page_w or page_h (842 pt x 1860 / 842 is 1859.9999... in floating point)
    let rw = ((pw * scale + 1e-6) as i32).clamp(1, page_w);
    let rh = ((ph * scale + 1e-6) as i32).clamp(1, page_h);
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
    Link(WireLink),
    Quad([i32; 8]),
    Name { page: u32, text: Vec<u8> },
    Left(LeftOut),
}

/// A link as the worker sends it (L record): its pages, area (page pixels) and place among the page's links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireLink {
    pub src: u32,
    pub dst: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub index: u32,
}

/// What the link walk left out (F record), for the log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LeftOut {
    pub uri: u32,
    pub other: u32,
    pub dead: u32,
    pub capped: u32,
}

/// The most link records a worker sends (the notebook keeps at most 14,400 in the end).
pub const LINKS_MAX: usize = 60_000;
/// Corners and areas stay within this many pixels of the page (a hostile file's numbers go no further).
pub const LINK_COORD_MAX: i32 = 200_000;

fn number(t: &str) -> Option<u32> {
    if t.is_empty() || t.len() > 9 || !t.bytes().all(|c| c.is_ascii_digit()) || (t.len() > 1 && t.starts_with('0')) {
        return None;
    }
    t.parse().ok()
}

/// A signed decimal: at most 7 digits, no leading zero, no "-0".
fn snumber(t: &str) -> Option<i32> {
    let (neg, d) = match t.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, t),
    };
    if d.is_empty() || d.len() > 7 || !d.bytes().all(|c| c.is_ascii_digit()) || (d.len() > 1 && d.starts_with('0')) || (neg && d == "0") {
        return None;
    }
    let v: i32 = d.parse().ok()?;
    Some(if neg { -v } else { v })
}

fn unhex(t: &str) -> Option<Vec<u8>> {
    if t.is_empty() || t.len() % 2 != 0 || t.len() > 2 * 95 {
        return None;
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    t.as_bytes().chunks(2).map(|p| Some(nib(p[0])? << 4 | nib(p[1])?)).collect()
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
        ("L", 8) => {
            let sn = |i: usize| t.get(i).and_then(|x| snumber(x)).ok_or(Violation("record"));
            let (w, h) = (sn(5)?, sn(6)?);
            if w < 1 || h < 1 {
                return Err(bad);
            }
            Ok(Rec::Link(WireLink { src: num(1)?, dst: num(2)?, x: sn(3)?, y: sn(4)?, w, h, index: num(7)? }))
        }
        ("Q", 9) => {
            let mut q = [0i32; 8];
            for (i, c) in q.iter_mut().enumerate() {
                *c = t.get(1 + i).and_then(|x| snumber(x)).ok_or(Violation("record"))?;
            }
            Ok(Rec::Quad(q))
        }
        ("O", 3) => Ok(Rec::Name { page: num(1)?, text: unhex(t[2]).ok_or(bad)? }),
        ("F", 5) => Ok(Rec::Left(LeftOut { uri: num(1)?, other: num(2)?, dead: num(3)?, capped: num(4)? })),
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

// ---- what the link walk found, as the writer keeps it ----

/// The links, names and counts of a walk, every record checked as it is taken (a worker that lies can only lie
/// inside these bounds). It is also what the work folder's `links.part` and `names.part` hold, in the
/// records' own grammar, so the last slice reads them back through the same checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collected {
    /// each link with its quad (corners in 1/16 px) when it has one
    pub links: Vec<(WireLink, Option<[i32; 8]>)>,
    /// page from 1 and its name (UTF-8, at most 95 bytes)
    pub names: Vec<(u32, Vec<u8>)>,
    pub left: LeftOut,
    seen_left: bool,
}

impl Collected {
    /// Takes one record of a walk for a PDF of `pages` pages; Err for what the grammar does not allow there.
    pub fn take(&mut self, rec: Rec, pages: u32) -> Result<(), Violation> {
        let bad = |why| Err(Violation(why));
        match rec {
            Rec::Link(l) => {
                let off = l.x.abs().max(l.y.abs()) > LINK_COORD_MAX || l.w > 2 * LINK_COORD_MAX || l.h > 2 * LINK_COORD_MAX;
                if l.src < 1 || l.src > pages || l.dst < 1 || l.dst > pages || off {
                    return bad("a link outside its pages");
                }
                if self.links.len() >= LINKS_MAX {
                    return bad("too many links");
                }
                self.links.try_reserve(1).map_err(|_| Violation("out of memory"))?;
                self.links.push((l, None));
                Ok(())
            }
            Rec::Quad(q) => match self.links.last_mut() {
                Some((_, none @ None)) if q.iter().all(|c| c.abs() <= LINK_COORD_MAX * 16) => {
                    *none = Some(q);
                    Ok(())
                }
                _ => bad("a quad without its link"),
            },
            Rec::Name { page, text } => {
                if page < 1 || page > pages || text.is_empty() || text.len() > 95 || std::str::from_utf8(&text).is_err() || self.names.iter().any(|n| n.0 == page) {
                    return bad("a name that does not fit");
                }
                self.names.push((page, text));
                Ok(())
            }
            Rec::Left(l) if !self.seen_left => {
                self.left = l;
                self.seen_left = true;
                Ok(())
            }
            _ => bad("an unexpected record"),
        }
    }

    /// links.part: the L and Q records, then F.
    pub fn links_text(&self) -> String {
        let mut t = String::new();
        for (l, q) in &self.links {
            t.push_str(&format!("L {} {} {} {} {} {} {}\n", l.src, l.dst, l.x, l.y, l.w, l.h, l.index));
            if let Some(q) = q {
                t.push_str(&format!("Q {}\n", q.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")));
            }
        }
        let l = self.left;
        t.push_str(&format!("F {} {} {} {}\n", l.uri, l.other, l.dead, l.capped));
        t
    }

    /// names.part: the O records.
    pub fn names_text(&self) -> String {
        self.names.iter().map(|(p, n)| format!("O {p} {}\n", n.iter().map(|b| format!("{b:02x}")).collect::<String>())).collect()
    }

    /// Reads the two parts back, strictly: None for any line the grammar or the page count refuses.
    pub fn from_parts(links: &[u8], names: &[u8], pages: u32) -> Option<Collected> {
        let mut c = Collected::default();
        for text in [links, names] {
            for line in text.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
                c.take(parse_record(line).ok()?, pages).ok()?;
            }
        }
        Some(c)
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
    // read whole (the writer said so, or the mapping it found to work failed here): never past the read-whole cap
    if file_len as u64 > s1::PDF_MAX_READ_WHOLE {
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
    /// dies (1) or spins (2) when asked for the links
    links: u32,
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

/// Where a PDF page's user space lands on the w x h Scribe page: the render's own transform, which the link
/// areas share, so a box lands on what it covers.
fn page_fit(page: &hayro::hayro_syntax::page::Page<'_>, w: i32, h: i32) -> hayro::kurbo::Affine {
    use hayro::hayro_interpret::util::TransformExt;
    use hayro::kurbo::Affine;
    let (pw, ph) = page.render_dimensions();
    let f = fit_page(pw as f64, ph as f64, w, h);
    Affine::translate((f.ox as f64, f.oy as f64)) * Affine::scale(f.scale) * page.initial_transform(true).to_kurbo()
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
    use hayro::kurbo::Affine;
    use hayro::vello_cpu::{color::palette::css::WHITE, RasterizerSettings, TargetInit};
    let Some(page) = pdf_pages.get(n.wrapping_sub(1)) else {
        return false;
    };
    let fit = page_fit(page, w, h);
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
                say("K\n");
            }
            ["LINKS", w, h] => {
                let (Some(w), Some(h)) = (number(w), number(h)) else {
                    say("E protocol\n");
                    return 1;
                };
                if w == 0 || h == 0 || w as i32 > PAGE_PX_MAX || h as i32 > PAGE_PX_MAX {
                    say("E protocol\n");
                    return 1;
                }
                #[cfg(not(feature = "kindle"))]
                run_links_hook(&hooks);
                links::walk(&pdf, pages, w as i32, h as i32);
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
                    "links" => hooks.links = p,
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
fn run_links_hook(h: &Hooks) {
    match h.links {
        1 => std::process::abort(),
        2 => {
            let mut x = 0u64;
            loop {
                x = std::hint::black_box(x.wrapping_add(1));
            }
        }
        _ => {}
    }
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

// ====================================================================================================
// The link walk (areas/pdf.md 5.1; the walk caps are 4.3's: depth 32, 100,000 entries, a visited set)
// ====================================================================================================

mod links {
    use super::*;
    use hayro::hayro_syntax::object::{Array, Dict, MaybeRef, Name, Object, ObjectIdentifier};
    use hayro::hayro_syntax::page::Page;
    use hayro::hayro_syntax::Pdf;
    use hayro::kurbo::{Affine, Point};
    use std::collections::{HashMap, HashSet};

    /// How deep a name tree or the outline may nest, and how many entries (and nodes) either may hold.
    const WALK_DEPTH: usize = 32;
    const WALK_ENTRIES: usize = 100_000;
    /// The annotations looked at on one page, and the quads of one link.
    const ANNOTS_PER_PAGE: usize = 20_000;
    const QUADS_MAX: usize = 8;

    /// Records leave in blocks, so a planner's thousands of links are not thousands of writes.
    struct Out {
        buf: String,
        /// bytes sent so far (the job's records have 4 MiB in all: links stop at 3)
        bytes: usize,
    }
    impl Out {
        fn put(&mut self, s: &str) {
            self.bytes += s.len();
            self.buf.push_str(s);
            if self.buf.len() > 32 << 10 {
                self.flush();
            }
        }
        fn flush(&mut self) {
            if !self.buf.is_empty() {
                say(&self.buf);
                self.buf.clear();
            }
        }
    }

    struct Walk<'a> {
        pdf: &'a Pdf,
        pages: &'a [Page<'a>],
        page_of: HashMap<ObjectIdentifier, u32>,
        /// the catalog's name tree of destinations and its PDF 1.1 /Dests dictionary, read when a link first
        /// names a destination (name -> page from 1)
        tree: Option<HashMap<Vec<u8>, u32>>,
        dests: Option<HashMap<Vec<u8>, u32>>,
        left: LeftOut,
    }

    fn name_is(n: &Name<'_>, s: &[u8]) -> bool {
        let b: &[u8] = n.as_ref();
        b == s
    }

    impl<'a> Walk<'a> {
        fn catalog(&self) -> Option<Dict<'a>> {
            let x = self.pdf.xref();
            x.get::<Dict<'a>>(x.root_id())
        }

        fn resolve(&self, m: MaybeRef<Object<'a>>) -> Option<Object<'a>> {
            match m {
                MaybeRef::Ref(r) => self.pdf.xref().get::<Object<'a>>(r.into()),
                MaybeRef::NotRef(o) => Some(o),
            }
        }

        /// The page an explicit destination array names: by reference, or (out of spec, seen in the wild) by
        /// a 0-based number inside the page count.
        fn array_page(&self, a: &Array<'a>) -> Option<u32> {
            match a.raw_iter().next()? {
                MaybeRef::Ref(r) => self.page_of.get(&ObjectIdentifier::from(r)).copied(),
                MaybeRef::NotRef(Object::Number(n)) => {
                    let i = n.as_i64();
                    (0..self.pages.len() as i64).contains(&i).then(|| i as u32 + 1)
                }
                _ => None,
            }
        }

        /// The page a destination object goes to: an array, a dictionary with /D (a /Dests entry), or (when
        /// `named`) a name or string looked up in both tables, as producers mix them.
        fn dest_page(&mut self, o: &Object<'a>, named: bool) -> Option<u32> {
            match o {
                Object::Array(a) => self.array_page(a),
                Object::Dict(d) => {
                    let a = d.get::<Array<'a>>("D")?;
                    self.array_page(&a)
                }
                Object::Name(n) if named => {
                    let k: &[u8] = n.as_ref();
                    self.lookup(k)
                }
                Object::String(s) if named => self.lookup(s.as_bytes()),
                _ => None,
            }
        }

        fn lookup(&mut self, key: &[u8]) -> Option<u32> {
            if self.tree.is_none() {
                self.tree = Some(self.build_tree());
            }
            if let Some(p) = self.tree.as_ref().and_then(|m| m.get(key)) {
                return Some(*p);
            }
            if self.dests.is_none() {
                self.dests = Some(self.build_dests());
            }
            self.dests.as_ref().and_then(|m| m.get(key)).copied()
        }

        /// Catalog /Names /Dests: a name tree, walked with a visited set under the caps.
        fn build_tree(&mut self) -> HashMap<Vec<u8>, u32> {
            let mut map = HashMap::new();
            let Some(root) = self.catalog().and_then(|c| c.get::<Dict<'a>>("Names")).and_then(|n| n.get::<Dict<'a>>("Dests")) else {
                return map;
            };
            let mut seen: HashSet<ObjectIdentifier> = HashSet::new();
            let mut stack = vec![(root, 1usize)];
            let (mut entries, mut nodes) = (0usize, 0usize);
            while let Some((node, depth)) = stack.pop() {
                nodes += 1;
                if nodes > WALK_ENTRIES {
                    break;
                }
                if let Some(id) = node.obj_id() {
                    if !seen.insert(id) {
                        continue;
                    }
                }
                if let Some(arr) = node.get::<Array<'a>>("Names") {
                    let mut it = arr.raw_iter();
                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                        if entries >= WALK_ENTRIES {
                            break;
                        }
                        entries += 1;
                        let Some(Object::String(key)) = self.resolve(k) else { continue };
                        let Some(val) = self.resolve(v) else { continue };
                        if let Some(p) = self.dest_page(&val, false) {
                            map.entry(key.as_bytes().to_vec()).or_insert(p);
                        }
                    }
                }
                if entries >= WALK_ENTRIES {
                    break;
                }
                if depth < WALK_DEPTH {
                    if let Some(kids) = node.get::<Array<'a>>("Kids") {
                        for k in kids.raw_iter().take(WALK_ENTRIES) {
                            if let Some(Object::Dict(d)) = self.resolve(k) {
                                stack.push((d, depth + 1));
                            }
                        }
                    }
                }
            }
            map
        }

        /// Catalog /Dests: the PDF 1.1 dictionary of names.
        fn build_dests(&mut self) -> HashMap<Vec<u8>, u32> {
            let mut map = HashMap::new();
            let Some(d) = self.catalog().and_then(|c| c.get::<Dict<'a>>("Dests")) else { return map };
            for key in d.keys().take(WALK_ENTRIES) {
                let k: &[u8] = key.as_ref();
                if let Some(o) = d.get::<Object<'a>>(k) {
                    if let Some(p) = self.dest_page(&o, false) {
                        map.entry(k.to_vec()).or_insert(p);
                    }
                }
            }
            map
        }

        /// Where an annotation or outline item goes: /A (GoTo, Named) first, else /Dest. `Err(())` for an
        /// action Grimoire leaves out (counted by the caller), `Ok(None)` for none that resolves.
        fn target(&mut self, d: &Dict<'a>, src: u32) -> Result<Option<u32>, Kind> {
            if let Some(act) = d.get::<Dict<'a>>("A") {
                let Some(s) = act.get::<Name<'a>>("S") else { return Err(Kind::Other) };
                if name_is(&s, b"GoTo") {
                    let o = act.get::<Object<'a>>("D");
                    return Ok(o.and_then(|o| self.dest_page(&o, true)));
                }
                if name_is(&s, b"Named") {
                    let n = self.pages.len() as u32;
                    let Some(nm) = act.get::<Name<'a>>("N") else { return Err(Kind::Other) };
                    return match nm.as_ref() as &[u8] {
                        b"NextPage" => Ok((src < n).then(|| src + 1)),
                        b"PrevPage" => Ok((src > 1).then(|| src - 1)),
                        b"FirstPage" => Ok(Some(1)),
                        b"LastPage" => Ok(Some(n)),
                        _ => Err(Kind::Other),
                    };
                }
                if name_is(&s, b"URI") {
                    return Err(Kind::Uri);
                }
                return Err(Kind::Other);
            }
            match d.get::<Object<'a>>("Dest") {
                Some(o) => Ok(self.dest_page(&o, true)),
                None => Err(Kind::Other),
            }
        }
    }

    enum Kind {
        Uri,
        Other,
    }

    fn pt(fit: Affine, x: f64, y: f64) -> Option<(f64, f64)> {
        let p = fit * Point::new(x, y);
        (p.x.is_finite() && p.y.is_finite()).then_some((p.x, p.y))
    }

    fn clamp_px(v: f64) -> i32 {
        v.clamp(-(LINK_COORD_MAX as f64), LINK_COORD_MAX as f64) as i32
    }

    /// The box of corners in page pixels: floor of the minimum, ceiling of the maximum, at least 1 px.
    fn box_of(p: &[(f64, f64)]) -> (i32, i32, i32, i32) {
        let x0 = p.iter().map(|c| c.0).fold(f64::MAX, f64::min);
        let y0 = p.iter().map(|c| c.1).fold(f64::MAX, f64::min);
        let x1 = p.iter().map(|c| c.0).fold(f64::MIN, f64::max);
        let y1 = p.iter().map(|c| c.1).fold(f64::MIN, f64::max);
        let (x, y) = (clamp_px(x0.floor()), clamp_px(y0.floor()));
        ((x), (y), (clamp_px(x1.ceil()) - x).max(1), (clamp_px(y1.ceil()) - y).max(1))
    }

    /// The four corners of a quad in order around it, and whether they make an axis-aligned rectangle (no
    /// polygon is then needed: its box is the link's area).
    fn ring(mut p: [(f64, f64); 4]) -> ([(f64, f64); 4], bool) {
        let cx = p.iter().map(|c| c.0).sum::<f64>() / 4.0;
        let cy = p.iter().map(|c| c.1).sum::<f64>() / 4.0;
        p.sort_by(|a, b| (a.1 - cy).atan2(a.0 - cx).total_cmp(&(b.1 - cy).atan2(b.0 - cx)));
        let flat = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 0.5 || (a.1 - b.1).abs() < 0.5;
        let rect = (0..4).all(|i| flat(p[i], p[(i + 1) % 4])) && ((p[0].0 - p[1].0).abs() < 0.5) != ((p[1].0 - p[2].0).abs() < 0.5);
        (p, rect)
    }

    /// Titles: UTF-16BE or LE with a BOM, UTF-8 with a BOM, else PDFDocEncoding (7.9.2.2). Controls become
    /// spaces, the ends are trimmed, and the text is cut at 95 bytes between characters.
    pub(super) fn title_text(b: &[u8]) -> String {
        let s: String = if let Some(r) = b.strip_prefix(&[0xFE, 0xFF]) {
            char::decode_utf16(r.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]]))).map(|c| c.unwrap_or('\u{FFFD}')).collect()
        } else if let Some(r) = b.strip_prefix(&[0xFF, 0xFE]) {
            char::decode_utf16(r.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]))).map(|c| c.unwrap_or('\u{FFFD}')).collect()
        } else if let Some(r) = b.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
            String::from_utf8_lossy(r).into_owned()
        } else {
            b.iter().map(|&c| pdfdoc(c)).collect()
        };
        let s: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        let s = s.trim();
        let mut end = s.len().min(95);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s[..end].trim_end().to_string()
    }

    fn pdfdoc(c: u8) -> char {
        const HIGH: [u32; 31] = [
            0x2022, 0x2020, 0x2021, 0x2026, 0x2014, 0x2013, 0x0192, 0x2044, 0x2039, 0x203A, 0x2212, 0x2030, 0x201E, 0x201C, 0x201D, 0x2018, 0x2019,
            0x201A, 0x2122, 0xFB01, 0xFB02, 0x0141, 0x0152, 0x0160, 0x0178, 0x017D, 0x0131, 0x0142, 0x0153, 0x0161, 0x017E,
        ];
        const LOW: [u32; 8] = [0x02D8, 0x02C7, 0x02C6, 0x02D9, 0x02DD, 0x02DB, 0x02DA, 0x02DC];
        let u = match c {
            0x18..=0x1F => LOW[(c - 0x18) as usize],
            0x80..=0x9E => HIGH[(c - 0x80) as usize],
            0xA0 => 0x20AC,
            0xAD | 0x7F | 0x9F => 0x20,
            c => c as u32,
        };
        char::from_u32(u).unwrap_or(' ')
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    /// Answers the LINKS command: L (and Q) records page by page, O records, then F.
    pub(super) fn walk(pdf: &Pdf, pages: &[Page<'_>], w: i32, h: i32) {
        let mut page_of = HashMap::new();
        for (i, p) in pages.iter().enumerate() {
            if let Some(id) = p.raw().obj_id() {
                page_of.entry(id).or_insert(i as u32 + 1);
            }
        }
        let mut wk = Walk { pdf, pages, page_of, tree: None, dests: None, left: LeftOut::default() };
        let mut out = Out { buf: String::new(), bytes: 0 };
        let mut sent = 0usize;
        for (i, page) in pages.iter().enumerate() {
            let src = i as u32 + 1;
            let Some(annots) = page.raw().get::<Array<'_>>("Annots") else { continue };
            let fit = page_fit(page, w, h);
            let mut index = 0u32;
            for item in annots.raw_iter().take(ANNOTS_PER_PAGE) {
                let Some(Object::Dict(a)) = wk.resolve(item) else { continue };
                match a.get::<Name<'_>>("Subtype") {
                    Some(n) if name_is(&n, b"Link") => {}
                    _ => continue,
                }
                // hidden and not-viewed annotations are not on the paper
                if a.get::<i32>("F").is_some_and(|f| f & (2 | 32) != 0) {
                    continue;
                }
                let dst = match wk.target(&a, src) {
                    Ok(Some(d)) if d != src => d,
                    // a link to its own page goes nowhere
                    Ok(_) => {
                        wk.left.dead += 1;
                        continue;
                    }
                    Err(Kind::Uri) => {
                        wk.left.uri += 1;
                        continue;
                    }
                    Err(Kind::Other) => {
                        wk.left.other += 1;
                        continue;
                    }
                };
                // the areas: one per quad (up to 8), else the rectangle
                let quads: Vec<f64> = a.get::<Array<'_>>("QuadPoints").map(|q| q.iter::<f64>().take(QUADS_MAX * 8).collect()).unwrap_or_default();
                let rect = a.get::<hayro::hayro_syntax::object::Rect>("Rect");
                let mut shapes: Vec<([(f64, f64); 4], bool)> = Vec::new();
                // A quad is taken only inside the link's Rect, as the spec asks (a point within 1 pt of it), and only
                // with an area: an all-zero quad, or one outside the Rect, is left out, and with none left the Rect
                // is the link's area.
                let inside = |q: &[f64]| -> bool {
                    let Some(r) = rect else { return true };
                    let (x0, x1) = (r.x0.min(r.x1) - 1.0, r.x0.max(r.x1) + 1.0);
                    let (y0, y1) = (r.y0.min(r.y1) - 1.0, r.y0.max(r.y1) + 1.0);
                    (0..4).all(|k| q[2 * k] >= x0 && q[2 * k] <= x1 && q[2 * k + 1] >= y0 && q[2 * k + 1] <= y1)
                };
                let has_area = |q: &[f64]| -> bool {
                    let xs = [q[0], q[2], q[4], q[6]];
                    let ys = [q[1], q[3], q[5], q[7]];
                    let w = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
                    let h = ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min);
                    w > 0.0 && h > 0.0
                };
                let quads: Vec<f64> = quads.chunks_exact(8).filter(|q| inside(q) && has_area(q)).flatten().copied().collect();
                if quads.len() >= 8 {
                    for q in quads.chunks_exact(8) {
                        let mut c = [(0.0, 0.0); 4];
                        let mut ok = true;
                        for k in 0..4 {
                            match pt(fit, q[2 * k], q[2 * k + 1]) {
                                Some(p) => c[k] = p,
                                None => ok = false,
                            }
                        }
                        if ok {
                            let (ring, flat) = ring(c);
                            shapes.push((ring, !flat));
                        }
                    }
                } else if let Some(r) = rect {
                    let (x0, x1) = (r.x0.min(r.x1), r.x0.max(r.x1));
                    let (y0, y1) = (r.y0.min(r.y1), r.y0.max(r.y1));
                    if x1 - x0 > 0.0 && y1 - y0 > 0.0 {
                        let cs = [pt(fit, x0, y0), pt(fit, x1, y0), pt(fit, x1, y1), pt(fit, x0, y1)];
                        if let [Some(a0), Some(a1), Some(a2), Some(a3)] = cs {
                            shapes.push(([a0, a1, a2, a3], false));
                        }
                    }
                }
                if shapes.is_empty() {
                    wk.left.dead += 1;
                    continue;
                }
                for (corners, polygon) in shapes {
                    if sent >= LINKS_MAX || out.bytes > 3 << 20 {
                        wk.left.capped += 1;
                        continue;
                    }
                    let (x, y, bw, bh) = box_of(&corners);
                    out.put(&format!("L {src} {dst} {x} {y} {bw} {bh} {index}\n"));
                    if polygon {
                        let q: Vec<String> = corners.iter().flat_map(|c| [clamp_px(c.0 * 16.0).to_string(), clamp_px(c.1 * 16.0).to_string()]).collect();
                        out.put(&format!("Q {}\n", q.join(" ")));
                    }
                    index += 1;
                    sent += 1;
                }
            }
        }
        // the outline names pages that have no name: depth first, the first item pointing at a page
        let mut named = vec![false; pages.len() + 1];
        let first = wk.catalog().and_then(|c| c.get::<Dict<'_>>("Outlines")).and_then(|o| o.get::<Dict<'_>>("First"));
        if let Some(first) = first {
            let mut seen: HashSet<ObjectIdentifier> = HashSet::new();
            let mut stack = vec![(first, 1usize)];
            let mut n = 0usize;
            while let Some((item, depth)) = stack.pop() {
                n += 1;
                if n > WALK_ENTRIES {
                    break;
                }
                if let Some(id) = item.obj_id() {
                    if !seen.insert(id) {
                        continue;
                    }
                }
                if let Some(next) = item.get::<Dict<'_>>("Next") {
                    stack.push((next, depth));
                }
                if depth < WALK_DEPTH {
                    if let Some(child) = item.get::<Dict<'_>>("First") {
                        stack.push((child, depth + 1));
                    }
                }
                let Ok(Some(page)) = wk.target(&item, 0) else { continue };
                if page == 0 || page as usize > pages.len() || named[page as usize] {
                    continue;
                }
                let Some(t) = item.get::<hayro::hayro_syntax::object::String<'_>>("Title") else { continue };
                let text = title_text(t.as_bytes());
                if text.is_empty() {
                    continue;
                }
                named[page as usize] = true;
                out.put(&format!("O {page} {}\n", hex(text.as_bytes())));
            }
        }
        let l = wk.left;
        out.put(&format!("F {} {} {} {}\n", l.uri, l.other, l.dead, l.capped));
        out.flush();
    }
}

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
    /// the writer had no memory for the page's pixels: the worker did nothing wrong (a strike, as a death)
    NoMemory,
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
    /// the same, the Kindle having slept while the page was waited for: retried once without a strike (3.7)
    TimeoutAfterWake,
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
        // A worker that ended by itself (`E password`) has closed its pipe: the writer's next line must fail, not
        // end the writer with SIGPIPE before it reads why (the worker inherits this, and exits on a failed write).
        // SAFETY: setting a signal's disposition to ignore.
        unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
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
            Ok(Rec::Sandboxed(on)) if sandbox_line_ok(on) => sandboxed = on,
            // (`S 0` where the sandbox is required: no worker, before a command or a byte of the file is sent)
            Ok(Rec::Sandboxed(_)) => {
                w.kill_and_reap();
                return Err(StartErr::Worker("the worker is not sandboxed"));
            }
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
        // (a send that fails is read past: a worker that ended by itself said why before it went)
        let _ = self.w.send("PROBE\n");
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
                Ok(Rec::Pages(n)) if pages.is_none() && n > 0 => pages = Some(n),
                Ok(Rec::ProbeDone) => {
                    let Some(n) = pages else { return Err(StartErr::Failed("no page count")) };
                    self.pages = n;
                    return Ok(n);
                }
                Ok(Rec::Err(ErrCode::Password)) => return Err(StartErr::Password),
                Ok(Rec::Err(ErrCode::Invalid)) => return Err(StartErr::Invalid),
                Ok(Rec::Err(ErrCode::Empty)) => return Err(StartErr::Empty),
                // A step can only fail before the handshake's S line; here the worker has parsed the PDF, so the
                // line is a lie (a worker the PDF took over would mark the session broken and count no try).
                Ok(Rec::Err(ErrCode::Sandbox(_))) => {
                    self.w.kill_and_reap();
                    return Err(StartErr::Failed("a sandbox record after the handshake"));
                }
                _ => {
                    self.w.kill_and_reap();
                    return Err(StartErr::Failed("unexpected record"));
                }
            }
        }
    }

    /// The links of the PDF in the pixels of a w x h Scribe page, its outline names and what was left out
    /// (LINKS, asked after PROBE). A worker that dies, stalls or lies here is killed and costs the notebook its
    /// links only: the pages do not depend on them.
    pub fn links(&mut self, w: i32, h: i32) -> Result<Collected, &'static str> {
        if !self.w.send(&format!("LINKS {w} {h}\n")) {
            return Err("the worker could not be asked");
        }
        let mut got = Collected::default();
        loop {
            let deadline = mono_ms() + page_watchdog_ms();
            let line = match self.w.read_line(512, deadline) {
                Ok(l) => l,
                Err(_) => {
                    self.w.kill_and_reap();
                    return Err("the worker died or stalled reading the links");
                }
            };
            self.rec_bytes += line.len() + 1;
            if self.rec_bytes > RECORDS_MAX_BYTES {
                self.w.kill_and_reap();
                return Err("too many records");
            }
            match parse_record(&line) {
                Ok(Rec::ProbeDone) => return Ok(got),
                Ok(r @ (Rec::Link(_) | Rec::Quad(_) | Rec::Name { .. } | Rec::Left(_))) => {
                    if let Err(v) = got.take(r, self.pages) {
                        self.w.kill_and_reap();
                        return Err(v.0);
                    }
                }
                _ => {
                    self.w.kill_and_reap();
                    return Err("an unexpected record");
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
        // (a send that fails is read past: the worker's last record, or the end of its pipe, says what became of it)
        let _ = self.w.send(&format!("R {page} {w} {h} {bands}\n"));
        let slept0 = slept_ms();
        let (wu, hu) = (w as usize, h as usize);
        let mut px: Vec<u8> = Vec::new();
        let mut ch = 1usize;
        if px.try_reserve_exact(wu * hu).is_err() {
            self.w.kill_and_reap();
            return PageOutcome::NoMemory;
        }
        px.resize(wu * hu, 255);
        let mut next_y = 0usize;
        // The worker cuts a page into exactly `bands` bands of ceil(h / bands) rows (render_page), and each comes
        // within the watchdog: a page is never more than that, so a worker that drips one-row bands (asleep in
        // futex, no CPU spent) cannot hold the writer and the job queue for hours (security review 2026-11).
        let band_h = (h as u32).div_ceil(bands.max(1)) as usize;
        let page_end = mono_ms() + page_watchdog_ms().saturating_mul(bands.max(1) as u64 + 1);
        loop {
            let deadline = (mono_ms() + page_watchdog_ms()).min(page_end);
            let line = match self.w.read_line(512, deadline) {
                Ok(l) => l,
                Err(ReadErr::Timeout) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Died(timeout_since(slept0));
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
                        && bh as usize == band_h.min(hu - next_y)
                        && (bch == 1 || bch == 3);
                    if !ok {
                        self.w.kill_and_reap();
                        return PageOutcome::Lied("a band outside its page");
                    }
                    let n = wu * bh as usize * bch as usize;
                    let mut band = Vec::new();
                    if band.try_reserve_exact(n).is_err() {
                        self.w.kill_and_reap();
                        return PageOutcome::NoMemory;
                    }
                    band.resize(n, 0u8);
                    match self.w.read_exact(&mut band, deadline) {
                        Ok(()) => {}
                        Err(ReadErr::Timeout) => {
                            self.w.kill_and_reap();
                            return PageOutcome::Died(timeout_since(slept0));
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
                            return PageOutcome::NoMemory;
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
                Ok(Rec::Err(ErrCode::Sandbox(_))) => {
                    self.w.kill_and_reap();
                    return PageOutcome::Lied("a sandbox record after the handshake");
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

/// What the Kindle has slept since it started, in ms: CLOCK_BOOTTIME (which counts a suspend) less
/// CLOCK_MONOTONIC (which does not, as the watchdogs do not). It grows only while asleep. 0 where there is no
/// such clock (a wake is then never seen), or, in headless and test builds, with GRIMOIRE_PDF_TEST_WOKE an hour
/// more at every look.
fn slept_ms() -> i64 {
    #[cfg(not(feature = "kindle"))]
    if std::env::var_os("GRIMOIRE_PDF_TEST_WOKE").is_some() {
        static N: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
        return N.fetch_add(3_600_000, std::sync::atomic::Ordering::Relaxed);
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: both are written by clock_gettime before they are read.
        let (mut b, mut m): (libc::timespec, libc::timespec) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
        unsafe {
            libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut b);
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut m);
        }
        (b.tv_sec as i64 - m.tv_sec as i64) * 1000 + (b.tv_nsec as i64 - m.tv_nsec as i64) / 1_000_000
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

/// A page's watchdog ran out: right after a wake (the Kindle slept for more than a second while the page was
/// waited for) or not.
fn timeout_since(slept0: i64) -> Death {
    if slept_ms() - slept0 > 1000 {
        Death::TimeoutAfterWake
    } else {
        Death::Timeout
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
        assert_eq!(f.rw, 1860); // the deciding side fills the page exactly, never a pixel short
        assert_eq!(f.ox, 0);
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
    fn link_records_parse_strictly() {
        assert_eq!(parse_record(b"L 1 2 -5 10 40 20 3"), Ok(Rec::Link(WireLink { src: 1, dst: 2, x: -5, y: 10, w: 40, h: 20, index: 3 })));
        assert_eq!(parse_record(b"Q 0 0 160 -16 160 160 0 160"), Ok(Rec::Quad([0, 0, 160, -16, 160, 160, 0, 160])));
        assert_eq!(parse_record(b"O 3 436166c3a9"), Ok(Rec::Name { page: 3, text: "Caf\u{e9}".as_bytes().to_vec() }));
        assert_eq!(parse_record(b"F 1 2 3 4"), Ok(Rec::Left(LeftOut { uri: 1, other: 2, dead: 3, capped: 4 })));
        for bad in [
            &b"L 1 2 3 4 0 5 0"[..], // no width
            b"L 1 2 3 4 5 -6 0",
            b"L 1 2 -0 4 5 6 0",
            b"L 1 2 03 4 5 6 0",
            b"L 1 2 3 4 5 6",
            b"L 1 2 3 4 5 6 7 8",
            b"L 1 2 12345678 4 5 6 0",
            b"L 1 2 +3 4 5 6 0",
            b"Q 1 2 3 4 5 6 7",
            b"O 1 4",
            b"O 1 436166C3A9",
            b"O 1 zz",
            b"O 1 ",
            b"F 1 2 3",
            b"F -1 2 3 4",
        ] {
            assert!(parse_record(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
        }
        // a name is at most 95 bytes: 190 hex digits
        let long = format!("O 1 {}", "61".repeat(96));
        assert!(parse_record(long.as_bytes()).is_err());
        let ok = format!("O 1 {}", "61".repeat(95));
        assert!(parse_record(ok.as_bytes()).is_ok());
    }

    #[test]
    fn titles_decode_by_their_marker_and_are_cut_between_characters() {
        let t = links::title_text;
        assert_eq!(t(&[0xFE, 0xFF, 0x00, 0x41, 0x26, 0x3A]), "A\u{263a}");
        assert_eq!(t(&[0xFF, 0xFE, 0x41, 0x00]), "A");
        assert_eq!(t(b"\xEF\xBB\xBFcaf\xC3\xA9"), "caf\u{e9}");
        assert_eq!(t(b"caf\xE9"), "caf\u{e9}");
        assert_eq!(t(b"\x80\x84\xA0"), "\u{2022}\u{2014}\u{20ac}");
        assert_eq!(t(b"  a\tb\r\nc  "), "a b  c");
        // a lone surrogate is a replacement character, not a refusal
        assert_eq!(t(&[0xFE, 0xFF, 0xD8, 0x00, 0x00, 0x41]), "\u{fffd}A");
        // 95 bytes at most, never through a character (e acute is two bytes)
        let s = t(&utf16be_for_test(&"\u{e9}".repeat(60)));
        assert_eq!(s.len(), 94);
        assert_eq!(t(&utf16be_for_test(&"a".repeat(200))).len(), 95);
        assert_eq!(t(b""), "");
        assert_eq!(t(b"\x01\x02"), "");
    }

    fn utf16be_for_test(s: &str) -> Vec<u8> {
        let mut v = vec![0xFE, 0xFF];
        for u in s.encode_utf16() {
            v.extend_from_slice(&u.to_be_bytes());
        }
        v
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
