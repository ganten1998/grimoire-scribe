//! export.h: turning notebooks into files people can use elsewhere. Pure (no UI);
//! shared by the app, the share server, and the host-side tools/grimoire-export CLI.
//! Compression uses stbi_zlib_compress from third_party/stb_image_write.h.

use crate::prelude::*;

// ---- Buf ----

#[derive(Clone, Debug, Default)]
pub struct Buf {
    /// the bytes (the C's data, len, cap)
    pub data: Vec<u8>,
    /// sticky: an append ran out of memory, the contents are incomplete
    pub err: bool,
}
pub fn buf_free(b: &mut Buf) {
    *b = Buf::default();
}
/// -1 (and err, sticky) when the memory is not there, as the C's realloc failing.
pub fn buf_append(b: &mut Buf, p: &[u8]) -> i32 {
    if b.err {
        return -1;
    }
    if b.data.try_reserve(p.len()).is_err() {
        b.err = true;
        return -1;
    }
    b.data.extend_from_slice(p);
    0
}
/// printf into the buffer (cfmt.rs): `buf_printf!(b, "%s", x)`.
pub fn buf_printf(b: &mut Buf, fmt: &str, args: &[Arg]) -> i32 {
    let t = crate::cfmt::cformat(fmt, args);
    buf_append(b, t.as_bytes())
}

#[macro_export]
macro_rules! buf_printf {
    ($b:expr, $fmt:expr $(, $a:expr)* $(,)?) => {
        $crate::export::buf_printf($b, $fmt, &[$($crate::cfmt::Arg::from($a)),*])
    };
}

// ---- Sink: where a streamed file goes ----

/// Somewhere bytes go as they are made: a Buf, a file, the connection of a download, or nothing (a
/// count). false when they could not be taken (the memory or the disk is full, the reader is gone);
/// the writer then stops.
pub trait Sink {
    fn put(&mut self, b: &[u8]) -> bool;
}

/// The bytes into a Buf, after what it holds.
pub struct BufSink<'a>(pub &'a mut Buf);
impl Sink for BufSink<'_> {
    fn put(&mut self, b: &[u8]) -> bool {
        buf_append(self.0, b) == 0
    }
}

/// The bytes only counted: what a download's Content-Length is.
#[derive(Default)]
pub struct CountSink(pub u64);
impl Sink for CountSink {
    fn put(&mut self, b: &[u8]) -> bool {
        self.0 += b.len() as u64;
        true
    }
}

/// The bytes into a file, written as they come.
pub struct FileSink {
    f: std::fs::File,
    len: u64,
    failed: bool,
}
impl FileSink {
    /// Creates (truncating) the file. None when it cannot be (the simulator's full storage too).
    pub fn create(path: &str) -> Option<FileSink> {
        #[cfg(feature = "sim")]
        if crate::util::SIM_STORAGE_FULL.load(std::sync::atomic::Ordering::Relaxed) {
            qlog!("write %s: %s (simulated: storage full)", path, &strerror(libc::ENOSPC));
            return None;
        }
        match std::fs::File::create(path) {
            Ok(f) => Some(FileSink { f, len: 0, failed: false }),
            Err(e) => {
                qlog!("write %s: %s", path, &strerror(e.raw_os_error().unwrap_or(0)));
                None
            }
        }
    }
    /// A file to be made and read back (a spool): the caller unlinks it when it likes.
    pub fn create_spool(path: &str) -> Option<FileSink> {
        #[cfg(feature = "sim")]
        if crate::util::SIM_STORAGE_FULL.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path).ok()?;
        Some(FileSink { f, len: 0, failed: false })
    }
    /// A write failed (the disk is full): the file is not whole.
    pub fn failed(&self) -> bool {
        self.failed
    }
    /// Everything written, into `to` in blocks of 64 KB. false when `to` stops taking it.
    pub fn send_all(&self, to: &mut dyn Sink) -> bool {
        use std::os::unix::fs::FileExt;
        let mut block = vec![0u8; 1 << 16];
        let mut at = 0u64;
        while at < self.len {
            let want = block.len().min((self.len - at) as usize);
            match self.f.read_at(&mut block[..want], at) {
                Ok(0) | Err(_) => return false,
                Ok(n) => {
                    if !to.put(&block[..n]) {
                        return false;
                    }
                    at += n as u64;
                }
            }
        }
        true
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Makes the data durable, as write_file_atomic does before its rename. false on an error.
    pub fn sync(&self) -> bool {
        self.f.sync_all().is_ok()
    }
}
impl Sink for FileSink {
    fn put(&mut self, b: &[u8]) -> bool {
        use std::io::Write;
        if self.failed || self.f.write_all(b).is_err() {
            self.failed = true;
            return false;
        }
        self.len += b.len() as u64;
        true
    }
}

/// A C string for a path or a name, cut where the C would see its NUL.
fn cstr(s: &str) -> std::ffi::CString {
    let end = s.bytes().position(|b| b == 0).unwrap_or(s.len());
    std::ffi::CString::new(&s.as_bytes()[..end]).unwrap_or_default()
}

/// File-name-safe version of a notebook/folder name ("/" ":" etc. replaced, never empty).
pub fn export_safe_name(in_: &str, out: &mut String, cap: usize) {
    // Bytes, as the C works: a cut at cap - 1 can fall inside a character, and a String cannot hold
    // that, so the partial character is dropped at the very end (PORTING.md, Text).
    let mut o: Vec<u8> = Vec::new();
    for &ch in in_.as_bytes() {
        if ch == 0 || o.len() + 1 >= cap {
            break;
        }
        o.push(if ch < 32 || b"/\\:*?\"<>|".contains(&ch) { b'_' } else { ch });
    }
    // Trim leading/trailing spaces and dots (hidden files, Windows quirks).
    let mut s = 0;
    while s < o.len() && (o[s] == b' ' || o[s] == b'.') {
        s += 1;
    }
    o.drain(..s);
    while let Some(&last) = o.last() {
        if last == b' ' || last == b'.' {
            o.pop();
        } else {
            break;
        }
    }
    if o.is_empty() {
        strlcpy_q(out, "Untitled", cap);
        return;
    }
    *out = match String::from_utf8(o) {
        Ok(t) => t,
        Err(e) => {
            let good = e.utf8_error().valid_up_to();
            let mut b = e.into_bytes();
            b.truncate(good);
            String::from_utf8(b).unwrap_or_default()
        }
    };
}

// ---- stb ----

/// A block stb allocated with malloc, freed with free.
struct StbBytes {
    p: *mut u8,
    len: usize,
}
impl StbBytes {
    fn as_slice(&self) -> &[u8] {
        // SAFETY: p points at len initialized bytes that stb wrote and nothing else touches; they
        // stay allocated until this value drops.
        unsafe { std::slice::from_raw_parts(self.p, self.len) }
    }
}
impl Drop for StbBytes {
    fn drop(&mut self) {
        // SAFETY: p came from stb's malloc (STBIW_MALLOC is the C library's) and is freed once.
        unsafe { libc::free(self.p as *mut libc::c_void) }
    }
}

/// stbi_write_png_to_mem on one gray plane (the canvas's pixels, its stride).
fn png_to_mem(c: &Canvas) -> Option<StbBytes> {
    let mut len: libc::c_int = 0;
    // SAFETY: px holds h rows of stride bytes (canvas_init makes w * h with stride w), which stb
    // reads for w x h one-component pixels; len receives the size of the block it returns.
    let p = unsafe { crate::ffi_stb::stbi_write_png_to_mem(c.px.as_ptr(), c.stride, c.w, c.h, 1, &mut len) };
    if p.is_null() {
        return None;
    }
    Some(StbBytes { p, len: len as usize })
}

/// stbi_zlib_compress of data at the quality the exports use (8).
fn zlib_compress(data: &[u8]) -> Option<StbBytes> {
    let mut zl: libc::c_int = 0;
    // SAFETY: stb reads data_len bytes of data (the parameter is not const, but stb never writes
    // through it); zl receives the size of the block it returns.
    let z = unsafe { crate::ffi_stb::stbi_zlib_compress(data.as_ptr() as *mut u8, data.len() as libc::c_int, &mut zl, 8) };
    if z.is_null() {
        return None;
    }
    Some(StbBytes { p: z, len: zl as usize })
}

// ---- rendering helpers ----

// Link boxes found on a rendered page (render_page_links).
struct PageLinks<'a> {
    nav: &'a Nav,
    /// the notebook's document links (doclinks.ini), and the open page's, as (target page id, exact area):
    /// a vector beside the 64-entry arrays, since a planner's month page has 365
    docs: &'a DocLinks,
    doc: Vec<(u32, Rect)>,
    links: [NavLink; NAV_LINKS_PER_PAGE as usize],
    areas: [Rect; NAV_LINKS_PER_PAGE as usize],
    n: i32,
}

/// One page drawn into c (its paper kind in `paper`, its links in `links`). With `color` asked for (the
/// notebook PDF, all layers), a page whose picture has colour also gets `color`: the page as 8-bit RGB,
/// the picture fitted to the page and the ink drawn over it as it darkens or lightens the gray paper
/// under it; any other page leaves it None, and the gray canvas is the page.
#[allow(clippy::too_many_arguments)]
fn render_page_links(nb: &NotebookInfo, index: i32, dpi: f32, layers: i32, c: &mut Canvas, paper: Option<&mut u32>, links: Option<&mut PageLinks>, color: Option<&mut Option<Vec<u8>>>) -> i32 {
    // Rendered at the notebook's page size, as the editor has the page: one an older version scaled
    // to another screen goes back to that size, and a missing or damaged page is blank paper of it.
    let mut p: Option<Box<Page>> = None;
    if store_read_page(nb, index, &mut p) != 0 {
        return -1;
    }
    // A page bigger than any e-ink screen is damaged or hostile: do not try to allocate it.
    if p.as_ref().is_some_and(|p| p.w > 6000 || p.h > 6000) {
        if let Some(p) = p {
            page_free(p);
        }
        return -1;
    }
    let p = match p {
        Some(p) if canvas_init(c, p.w, p.h) == 0 => p,
        other => {
            if let Some(p) = other {
                page_free(p);
            }
            return -1;
        }
    };
    page_render_layers(c, &p, Rect { x: 0, y: 0, w: p.w, h: p.h }, dpi, layers);
    if let Some(paper) = paper {
        *paper = p.paper.0;
    }
    if let Some(links) = links {
        links.n = nav_link_areas(links.nav, &p, nb.page_ids[index as usize], NAV_LINK_MIN_MM * dpi / 25.4, &mut links.links, &mut links.areas);
        links.doc = doclinks_targets(links.docs, &p, nb.page_ids[index as usize]);
    }
    if let Some(color) = color.filter(|_| layers == PAGE_LAYER_ALL && paper_is_image(p.paper)) {
        *color = None;
        if let Some(mut pic) = bgimage_render_color(nb.id.as_str(), p.paper, p.w, p.h) {
            // The paper alone, as the gray page has it under its ink: where the page is the paper, the
            // picture shows as it is; where ink changed it, the picture takes the same change.
            let mut cp = Canvas::default();
            if canvas_init(&mut cp, p.w, p.h) == 0 {
                page_render_layers(&mut cp, &p, Rect { x: 0, y: 0, w: p.w, h: p.h }, dpi, PAGE_LAYER_PAPER);
                ink_over_colour(&mut pic.px, &cp.px, &c.px);
                *color = Some(std::mem::take(&mut pic.px));
            }
            canvas_free(&mut cp);
        }
    }
    page_free(p);
    0
}

fn render_page(nb: &NotebookInfo, index: i32, dpi: f32, layers: i32, c: &mut Canvas, paper: Option<&mut u32>) -> i32 {
    render_page_links(nb, index, dpi, layers, c, paper, None, None)
}

/// One page rendered at full size (paper + ink) as an 8-bit grayscale PNG.
pub fn export_page_png(nb: &NotebookInfo, index: i32, dpi: f32, out: &mut Buf) -> i32 {
    export_page_png_ex(nb, index, dpi, PAGE_LAYER_ALL, 1, None, out)
}

// The box of the pixels darker than 250 (the studio's "ink" threshold too), empty if none.
fn dark_box(c: &Canvas) -> Rect {
    let (mut x0, mut y0, mut x1, mut y1) = (c.w, c.h, -1, -1);
    for y in 0..c.h {
        let row = &c.px[(y * c.stride) as usize..];
        let (mut first, mut last) = (-1, -1);
        for x in 0..c.w {
            if row[x as usize] < 250 {
                if first < 0 {
                    first = x;
                }
                last = x;
            }
        }
        if first < 0 {
            continue;
        }
        if first < x0 {
            x0 = first;
        }
        if last > x1 {
            x1 = last;
        }
        if y < y0 {
            y0 = y;
        }
        y1 = y;
    }
    if x1 < 0 {
        Rect { x: 0, y: 0, w: 0, h: 0 }
    } else {
        Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 }
    }
}

/// What export_page_png_ex drew.
#[derive(Clone, Copy, Debug, Default)]
pub struct PageRender {
    /// the page's full size, px
    pub w: i32,
    pub h: i32,
    /// its PaperKind
    pub paper: u32,
    /// ink-only renders: box of the pixels darker than 250 (empty if none)
    pub ink: Rect,
}
/// The same with only some layers (PAGE_LAYER_* from page.h; a left-out layer is white), shrunk
/// `shrink` times by averaging whole blocks (1 = full size). info may be NULL.
pub fn export_page_png_ex(nb: &NotebookInfo, index: i32, dpi: f32, layers: i32, shrink: i32, info: Option<&mut PageRender>, out: &mut Buf) -> i32 {
    let mut c = Canvas::default();
    let mut paper: u32 = 0;
    if index < 0 || index >= nb.npages() || shrink < 1 || render_page(nb, index, dpi, layers, &mut c, Some(&mut paper)) != 0 {
        return -1;
    }
    if let Some(info) = info {
        *info = PageRender { w: c.w, h: c.h, paper, ink: if layers == PAGE_LAYER_INK { dark_box(&c) } else { Rect { x: 0, y: 0, w: 0, h: 0 } } };
    }
    if shrink > 1 {
        // whole shrink x shrink blocks from the top left; leftover edge pixels drop
        let mut s = Canvas::default();
        if c.w / shrink < 1 || c.h / shrink < 1 || canvas_init(&mut s, c.w / shrink, c.h / shrink) != 0 {
            canvas_free(&mut c);
            return -1;
        }
        let n = shrink * shrink;
        for y in 0..s.h {
            for x in 0..s.w {
                let mut sum: i32 = 0;
                for j in 0..shrink {
                    let at = ((y * shrink + j) * c.stride + x * shrink) as usize;
                    for i in 0..shrink {
                        sum += c.px[at + i as usize] as i32;
                    }
                }
                s.px[(y * s.stride + x) as usize] = ((sum + n / 2) / n) as u8;
            }
        }
        canvas_free(&mut c);
        c = s;
    }
    let png = png_to_mem(&c);
    canvas_free(&mut c);
    let Some(png) = png else { return -1 };
    buf_append(out, png.as_slice())
}

// ---- PDF ----

// PDF text string as UTF-16BE hex with BOM (handles any UTF-8 name).
fn pdf_text(b: &mut Buf, s: &[u8]) {
    buf_printf!(b, "<FEFF");
    let mut i = 0;
    while i < s.len() {
        let mut c = s[i] as u32;
        i += 1;
        let mut extra: i32 = if c >= 0xF0 {
            3
        } else if c >= 0xE0 {
            2
        } else if c >= 0xC0 {
            1
        } else {
            0
        };
        if extra != 0 {
            c &= 0x3F >> extra;
        }
        while extra > 0 && i < s.len() && (s[i] & 0xC0) == 0x80 {
            extra -= 1;
            c = (c << 6) | (s[i] as u32 & 0x3F);
            i += 1;
        }
        if c >= 0x10000 {
            c -= 0x10000;
            buf_printf!(b, "%04X%04X", 0xD800 + (c >> 10), 0xDC00 + (c & 0x3FF));
        } else {
            buf_printf!(b, "%04X", c);
        }
    }
    buf_printf!(b, ">");
}

/// Whole notebook as a PDF: one page per notebook page, each a Flate-compressed DeviceGray
/// image at the page's physical size (w/dpi inches), or DeviceRGB for a page whose picture has
/// colour (the ink drawn over it). Title = notebook name. The section names (nav.ini) are its
/// outline, and the links drawn to pages (links.ini) and the document's own (doclinks.ini) are
/// links on its pages. It is made in a Buf here; `export_notebook_pdf_to` writes it to any Sink,
/// object by object, one page in memory at a time.
pub fn export_notebook_pdf(nb: &NotebookInfo, dpi: f32, out: &mut Buf) -> i32 {
    export_notebook_pdf_progress(nb, dpi, out, None)
}

/// The PDF's writer: objects go to a scratch buffer, which is handed to the sink at the end of each
/// page (and before a page's image, which goes to the sink straight), keeping the offsets the xref wants.
struct PdfOut<'a> {
    sink: &'a mut dyn Sink,
    buf: Buf,
    /// bytes the sink has taken
    pos: usize,
    off: Vec<usize>,
}
impl PdfOut<'_> {
    fn here(&self) -> usize {
        self.pos + self.buf.data.len()
    }
    /// The C's OBJ(i): notes where object i starts and opens it.
    fn obj(&mut self, i: i32) {
        let at = self.here();
        self.off[i as usize] = at;
        buf_printf!(&mut self.buf, "%d 0 obj\n", i);
    }
    fn flush(&mut self) {
        if self.buf.data.is_empty() {
            return;
        }
        if self.buf.err || !self.sink.put(&self.buf.data) {
            self.buf.err = true;
        }
        self.pos += self.buf.data.len();
        self.buf.data.clear();
    }
    /// A big block, straight to the sink.
    fn big(&mut self, b: &[u8]) {
        self.flush();
        if self.buf.err || !self.sink.put(b) {
            self.buf.err = true;
        }
        self.pos += b.len();
    }
}

/// Optional page-completion callback, including (0, total) before rendering. No global state;
/// the Share child uses it to report preparation without changing the PDF bytes.
pub fn export_notebook_pdf_progress(nb: &NotebookInfo, dpi: f32, out: &mut Buf, progress: Option<&mut dyn FnMut(i32, i32)>) -> i32 {
    let mut sink = BufSink(out);
    export_notebook_pdf_to(nb, dpi, &mut sink, PdfCacheMode::Read, progress)
}

/// The notebook's PDF into a sink, a page at a time. `mode` says how the page-stream cache is used
/// (pdfcache.rs): the bytes are the same in every mode, a page being drawn or found.
pub fn export_notebook_pdf_to(nb: &NotebookInfo, dpi: f32, sink: &mut dyn Sink, mode: PdfCacheMode, mut progress: Option<&mut dyn FnMut(i32, i32)>) -> i32 {
    let np = nb.npages();
    if let Some(f) = progress.as_deref_mut() {
        f(0, np);
    }
    // The section names make the outline (a viewer's table of contents), and the links drawn on the
    // pages links to their pages (one to the Index has no place to go in the PDF).
    let mut named = vec![0i32; if np > 0 { np as usize } else { 1 }];
    let mut nav = Nav::default();
    nav_init(&mut nav);
    nav_load(nb.id.as_str(), &mut nav); // what does not read is left out
    let nnamed = nav_index(&nav, nb, &mut named[..np as usize]);
    let dl = doclinks_load(nb.id.as_str()); // what does not read is left out
    let page_at: std::collections::HashMap<u32, i32> = nb.page_ids.iter().enumerate().map(|(i, &p)| (p, i as i32)).collect();
    let mut pl = PageLinks {
        nav: &nav,
        docs: &dl,
        doc: Vec::new(),
        links: [NavLink::default(); NAV_LINKS_PER_PAGE as usize],
        areas: [Rect::default(); NAV_LINKS_PER_PAGE as usize],
        n: 0,
    };
    let mut cache = PdfCache::open(nb, dpi, mode);
    // Objects: 1 catalog, 2 pages, 3 info, then 3 per page (page, content, image), then the outline
    // (its root, then one per section) when there are names, then each link as its page comes.
    let outline = 4 + 3 * np;
    let mut nobj = 3 + 3 * np + if nnamed != 0 { 1 + nnamed } else { 0 };
    let mut cap = nobj + 16;
    let mut out = PdfOut { sink, buf: Buf::default(), pos: 0, off: vec![0; cap as usize + 1] };
    let mut rc = 0;
    // The comment's four bytes above 127 mark the file as binary for the tools that move it. They are
    // not UTF-8, so the line cannot be a Rust format string: the C's buf_printf copies them as they
    // are, and so does this.
    buf_append(&mut out.buf, b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    out.obj(1);
    if nnamed != 0 {
        buf_printf!(&mut out.buf, "<< /Type /Catalog /Pages 2 0 R /Outlines %d 0 R /PageMode /UseOutlines >>\nendobj\n", outline);
    } else {
        buf_printf!(&mut out.buf, "<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    }
    out.obj(2);
    buf_printf!(&mut out.buf, "<< /Type /Pages /Count %d /Kids [", np);
    for i in 0..np {
        buf_printf!(&mut out.buf, " %d 0 R", 4 + 3 * i);
    }
    buf_printf!(&mut out.buf, " ] >>\nendobj\n");
    out.obj(3);
    buf_printf!(&mut out.buf, "<< /Title ");
    pdf_text(&mut out.buf, nb.name.as_bytes());
    buf_printf!(&mut out.buf, " /Creator (Grimoire %s) /Producer (Grimoire) >>\nendobj\n", GRIMOIRE_VERSION);
    for i in 0..np {
        if rc != 0 {
            break;
        }
        let Some(img) = page_image(nb, i, dpi, &mut cache, &mut pl) else {
            rc = -1;
            break;
        };
        let pw = (img.w as f32) * 72.0 / dpi;
        let ph = (img.h as f32) * 72.0 / dpi;
        let pobj = 4 + 3 * i;
        let cobj = pobj + 1;
        let iobj = pobj + 2;
        // This page's links that go to a page: their objects come next.
        let mut to = [0i32; NAV_LINKS_PER_PAGE as usize];
        let mut nannot: i32 = 0;
        let first_annot = nobj + 1;
        for k in 0..pl.n as usize {
            let at = nav_link_page(&pl.links[k], nb);
            if at >= 0 {
                pl.areas[nannot as usize] = pl.areas[k];
                to[nannot as usize] = at;
                nannot += 1;
            }
        }
        // The page's document links follow the user's, each at its exact area (a viewer's tap box).
        let mut areas: Vec<Rect> = pl.areas[..nannot as usize].to_vec();
        let mut tos: Vec<i32> = to[..nannot as usize].to_vec();
        for (t, r) in &pl.doc {
            if let Some(&at) = page_at.get(t) {
                areas.push(*r);
                tos.push(at);
            }
        }
        let nannot = areas.len() as i32;
        if nobj + nannot > cap {
            let grown = (nobj + nannot) * 2;
            if out.off.try_reserve_exact((grown as usize + 1).saturating_sub(out.off.len())).is_err() {
                rc = -1;
                break;
            }
            out.off.resize(grown as usize + 1, 0);
            cap = grown;
        }
        nobj += nannot;
        out.obj(pobj);
        buf_printf!(
            &mut out.buf,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %.2f %.2f] /Contents %d 0 R /Resources << /XObject << /Im0 %d 0 R >> >>",
            pw,
            ph,
            cobj,
            iobj
        );
        if nannot != 0 {
            buf_printf!(&mut out.buf, " /Annots [");
            for k in 0..nannot {
                buf_printf!(&mut out.buf, " %d 0 R", first_annot + k);
            }
            buf_printf!(&mut out.buf, " ]");
        }
        buf_printf!(&mut out.buf, " >>\nendobj\n");
        for k in 0..nannot {
            let r = areas[k as usize];
            let s = 72.0f32 / dpi;
            out.obj(first_annot + k);
            buf_printf!(
                &mut out.buf,
                "<< /Type /Annot /Subtype /Link /Rect [%.2f %.2f %.2f %.2f] /Border [0 0 0] /Dest [%d 0 R /Fit] >>\nendobj\n",
                (r.x as f32) * s,
                fmla(-((r.y + r.h) as f32), s, ph),
                ((r.x + r.w) as f32) * s,
                fmla(-(r.y as f32), s, ph),
                4 + 3 * tos[k as usize]
            );
        }
        let mut content = String::new();
        let cl = csnprintf!(&mut content, 128, "q %.2f 0 0 %.2f 0 0 cm /Im0 Do Q\n", pw, ph);
        out.obj(cobj);
        buf_printf!(&mut out.buf, "<< /Length %d >>\nstream\n%s\nendstream\nendobj\n", cl, &content);
        out.obj(iobj);
        buf_printf!(
            &mut out.buf,
            "<< /Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /%s /BitsPerComponent 8 /Filter /FlateDecode /Length %d >>\nstream\n",
            (pw * dpi / 72.0 + 0.5) as i32,
            (ph * dpi / 72.0 + 0.5) as i32,
            if img.ch == 3 { "DeviceRGB" } else { "DeviceGray" },
            img.z.len() as i32
        );
        out.big(&img.z);
        drop(img);
        buf_printf!(&mut out.buf, "\nendstream\nendobj\n");
        out.flush();
        if out.buf.err {
            rc = -1;
        }
        if rc == 0 {
            if let Some(f) = progress.as_deref_mut() {
                f(i + 1, np);
            }
        }
    }
    if rc == 0 && nnamed != 0 {
        out.obj(outline);
        buf_printf!(&mut out.buf, "<< /Type /Outlines /First %d 0 R /Last %d 0 R /Count %d >>\nendobj\n", outline + 1, outline + nnamed, nnamed);
        for k in 0..nnamed {
            out.obj(outline + 1 + k);
            buf_printf!(&mut out.buf, "<< /Title ");
            pdf_text(&mut out.buf, nav_name(&nav, nb.page_ids[named[k as usize] as usize]).as_bytes());
            buf_printf!(&mut out.buf, " /Parent %d 0 R", outline);
            if k != 0 {
                buf_printf!(&mut out.buf, " /Prev %d 0 R", outline + k);
            }
            if k + 1 < nnamed {
                buf_printf!(&mut out.buf, " /Next %d 0 R", outline + 2 + k);
            }
            buf_printf!(&mut out.buf, " /Dest [%d 0 R /Fit] >>\nendobj\n", 4 + 3 * named[k as usize]);
        }
    }
    nav_free(&mut nav);
    if rc == 0 {
        let xref = out.here();
        buf_printf!(&mut out.buf, "xref\n0 %d\n0000000000 65535 f \n", nobj + 1);
        for i in 1..=nobj {
            buf_printf!(&mut out.buf, "%010zu 00000 n \n", out.off[i as usize]);
        }
        buf_printf!(&mut out.buf, "trailer\n<< /Size %d /Root 1 0 R /Info 3 0 R >>\nstartxref\n%zu\n%%%%EOF\n", nobj + 1, xref);
    }
    out.flush();
    if rc != 0 || out.buf.err {
        -1
    } else {
        0
    }
}

/// Page `index`'s image for the PDF, and its links in `pl`: the cache's record when it fits the page
/// as it is, else the page drawn (and kept, when the cache is written). None when it cannot be drawn.
fn page_image(nb: &NotebookInfo, index: i32, dpi: f32, cache: &mut Option<PdfCache>, pl: &mut PageLinks) -> Option<PageImage> {
    let pid = nb.page_ids[index as usize];
    if let Some(img) = cache.as_mut().and_then(|c| c.get(nb, pid)) {
        // A page with no links needs no reading: its image is all the PDF takes from it.
        let mut first = 0;
        pl.n = 0;
        pl.doc.clear();
        if nav_links_of(pl.nav, pid, &mut first) > 0 || !doclinks_of(pl.docs, pid).is_empty() {
            let mut p: Option<Box<Page>> = None;
            if store_read_page(nb, index, &mut p) != 0 {
                return None;
            }
            let p = p?;
            pl.n = nav_link_areas(pl.nav, &p, pid, NAV_LINK_MIN_MM * dpi / 25.4, &mut pl.links, &mut pl.areas);
            pl.doc = doclinks_targets(pl.docs, &p, pid);
            page_free(p);
        }
        return Some(img);
    }
    pdf_page_rendered();
    let mut c = Canvas::default();
    let mut paper = 0u32;
    let mut color: Option<Vec<u8>> = None;
    if render_page_links(nb, index, dpi, PAGE_LAYER_ALL, &mut c, Some(&mut paper), Some(pl), Some(&mut color)) != 0 {
        return None;
    }
    let (w, h) = (c.w, c.h);
    let plane: &[u8] = match &color {
        Some(rgb) => rgb,
        None => &c.px[..(w * h) as usize],
    };
    let z = zlib_compress(plane);
    let img = z.map(|z| PageImage { w, h, ch: if color.is_some() { 3 } else { 1 }, z: z.as_slice().to_vec() });
    canvas_free(&mut c);
    let img = img?;
    if let Some(cache) = cache.as_mut() {
        cache.put(nb, pid, paper, &img);
    }
    Some(img)
}

/// `rgb` (3 bytes a pixel) given the change from the gray paper to the gray page: a darker pixel
/// scales each channel by the same ratio, a lighter one (white ink) moves each toward white by the
/// same share. Untouched pixels stay the picture's own.
fn ink_over_colour(rgb: &mut [u8], paper: &[u8], page: &[u8]) {
    for (i, (&g0, &g1)) in paper.iter().zip(page.iter()).enumerate() {
        if g0 == g1 {
            continue;
        }
        let px = &mut rgb[3 * i..3 * i + 3];
        if g1 < g0 {
            for v in px.iter_mut() {
                *v = ((*v as u32 * g1 as u32 + (g0 as u32) / 2) / g0 as u32) as u8;
            }
        } else {
            let (num, den) = ((g1 - g0) as u32, (255 - g0) as u32);
            for v in px.iter_mut() {
                *v = (*v as u32 + ((255 - *v as u32) * num + den / 2) / den) as u8;
            }
        }
    }
}

// ---- zip ----

const CRC_TABLE: [u32; 256] = {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB88320u32 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
};
fn crc32_of(p: &[u8]) -> u32 {
    let mut c = 0xFFFFFFFFu32;
    for &b in p {
        c = CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFFFFFFu32
}

fn le16(b: &mut Buf, v: u16) {
    buf_append(b, &[v as u8, (v >> 8) as u8]);
}
fn le32(b: &mut Buf, v: u32) {
    buf_append(b, &[v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]);
}

fn dos_time() -> (u16, u16) {
    let tm = local_tm(wall_time());
    let t = ((tm.tm_hour << 11) | (tm.tm_min << 5) | (tm.tm_sec / 2)) as u16;
    let d = (((tm.tm_year - 80) << 9) | ((tm.tm_mon + 1) << 5) | tm.tm_mday) as u16;
    (t, d)
}

/// A zip (stored or deflated entries, CRC32, no zip64) streaming builder: with a sink, each entry goes
/// out as it is finished and `out` is only the scratch it is built in.
pub struct ZipWriter<'a> {
    pub out: &'a mut Buf,
    pub central: Buf,
    pub count: i32,
    sink: Option<&'a mut dyn Sink>,
    /// bytes already handed to the sink
    flushed: u64,
}
/// (The C's zip_begin(z, out) fills a ZipWriter in place; here it makes one.)
pub fn zip_begin(out: &mut Buf) -> ZipWriter<'_> {
    ZipWriter { out, central: Buf::default(), count: 0, sink: None, flushed: 0 }
}
/// A zip made into a sink as it goes, `scratch` holding one entry at a time.
pub fn zip_begin_to<'a>(scratch: &'a mut Buf, sink: &'a mut dyn Sink) -> ZipWriter<'a> {
    ZipWriter { out: scratch, central: Buf::default(), count: 0, sink: Some(sink), flushed: 0 }
}
fn zip_pos(z: &ZipWriter) -> u64 {
    z.flushed + z.out.data.len() as u64
}
/// Hands what is built to the sink (when there is one).
fn zip_spill(z: &mut ZipWriter) {
    if let Some(s) = z.sink.as_deref_mut() {
        if z.out.data.is_empty() {
            return;
        }
        if z.out.err || !s.put(&z.out.data) {
            z.out.err = true;
        }
        z.flushed += z.out.data.len() as u64;
        z.out.data.clear();
    }
}

#[allow(clippy::too_many_arguments)]
fn zip_central(z: &mut ZipWriter, name: &[u8], flags: u16, method: u16, crc: u32, csize: u32, usize_: u32, local_off: u32) {
    let (t, d) = dos_time();
    let c = &mut z.central;
    le32(c, 0x02014b50);
    le16(c, 0x0314); // made by: unix, 2.0
    le16(c, 20);
    le16(c, flags);
    le16(c, method);
    le16(c, t);
    le16(c, d);
    le32(c, crc);
    le32(c, csize);
    le32(c, usize_);
    le16(c, name.len() as u16);
    le16(c, 0);
    le16(c, 0);
    le16(c, 0);
    le16(c, 0);
    le32(c, 0o100644u32 << 16);
    le32(c, local_off);
    buf_append(c, name);
    z.count += 1;
}

pub fn zip_add(z: &mut ZipWriter, name: &str, data: &[u8], deflate: bool) -> i32 {
    let len = data.len();
    if len > 0xFFFFFFF0 || zip_pos(z) > 0xFFFFFFF0 || z.count >= 0xFFFF {
        return -1; // no zip64
    }
    let crc = crc32_of(data);
    let mut payload: &[u8] = data;
    let mut method: u16 = 0;
    let zbuf = if deflate && len > 64 { zlib_compress(data) } else { None };
    if let Some(zb) = &zbuf {
        // Zip wants raw deflate: drop the 2-byte zlib header and 4-byte adler32 trailer.
        let zl = zb.len;
        if zl > 6 && zl - 6 < len {
            payload = &zb.as_slice()[2..zl - 4];
            method = 8;
        }
    }
    let plen = payload.len();
    let (t, d) = dos_time();
    let nl = name.len() as u16;
    let name_bytes = &name.as_bytes()[..nl as usize];
    let local_off = zip_pos(z) as u32;
    let o = &mut *z.out;
    le32(o, 0x04034b50);
    le16(o, 20);
    le16(o, 0x0800); // UTF-8 names
    le16(o, method);
    le16(o, t);
    le16(o, d);
    le32(o, crc);
    le32(o, plen as u32);
    le32(o, len as u32);
    le16(o, nl);
    le16(o, 0);
    buf_append(o, name_bytes);
    let rc = buf_append(o, payload);
    zip_central(z, name_bytes, 0x0800, method, crc, plen as u32, len as u32, local_off);
    zip_spill(z);
    rc
}

/// The CRC-32 running state after p (start from 0xFFFFFFFF, and xor the end with it).
fn crc32_run(mut c: u32, p: &[u8]) -> u32 {
    for &b in p {
        c = CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c
}

/// An entry's bytes as they are made: counted, checksummed, and written as the stored blocks of a
/// deflate stream (65535 bytes at most each), the entry's sizes being known only at its end.
struct EntrySink<'b, 'a> {
    z: &'b mut ZipWriter<'a>,
    block: Vec<u8>,
    crc: u32,
    size: u64,
    packed: u64,
}
impl EntrySink<'_, '_> {
    fn emit(&mut self, last: bool) {
        let n = self.block.len() as u16;
        let o = &mut *self.z.out;
        buf_append(o, &[last as u8]);
        le16(o, n);
        le16(o, !n);
        buf_append(o, &self.block);
        self.packed += 5 + self.block.len() as u64;
        self.block.clear();
        if self.z.out.data.len() >= 1 << 16 {
            zip_spill(self.z);
        }
    }
}
impl Sink for EntrySink<'_, '_> {
    fn put(&mut self, mut b: &[u8]) -> bool {
        self.crc = crc32_run(self.crc, b);
        self.size += b.len() as u64;
        while !b.is_empty() {
            let room = 65535 - self.block.len();
            let n = room.min(b.len());
            self.block.extend_from_slice(&b[..n]);
            b = &b[n..];
            if self.block.len() == 65535 {
                self.emit(false);
            }
        }
        !self.z.out.err
    }
}

/// An entry whose bytes `fill` makes into the sink it is given, written as they come: a local header
/// that leaves the sizes to the data descriptor after the data (flag bit 3), the data as stored
/// deflate blocks, then the descriptor; the central directory has the true sizes. Memory: one block.
/// -1 when `fill` fails (the zip is then broken: the caller stops) or the zip is full.
pub fn zip_add_stream(z: &mut ZipWriter, name: &str, fill: &mut dyn FnMut(&mut dyn Sink) -> i32) -> i32 {
    if zip_pos(z) > 0xFFFFFFF0 || z.count >= 0xFFFF {
        return -1;
    }
    let (t, d) = dos_time();
    let nl = name.len() as u16;
    let name_bytes = &name.as_bytes()[..nl as usize];
    let local_off = zip_pos(z) as u32;
    {
        let o = &mut *z.out;
        le32(o, 0x04034b50);
        le16(o, 20);
        le16(o, 0x0808); // UTF-8 names, sizes after the data
        le16(o, 8);
        le16(o, t);
        le16(o, d);
        le32(o, 0);
        le32(o, 0);
        le32(o, 0);
        le16(o, nl);
        le16(o, 0);
        buf_append(o, name_bytes);
    }
    let mut es = EntrySink { z: &mut *z, block: Vec::new(), crc: 0xFFFFFFFF, size: 0, packed: 0 };
    let rc = fill(&mut es);
    es.emit(true); // (the last block, empty or not)
    let (crc, size, packed) = (es.crc ^ 0xFFFFFFFF, es.size, es.packed);
    drop(es);
    if rc != 0 || size > 0xFFFFFFF0 || packed > 0xFFFFFFF0 {
        return -1;
    }
    let o = &mut *z.out;
    le32(o, 0x08074b50);
    le32(o, crc);
    le32(o, packed as u32);
    le32(o, size as u32);
    zip_central(z, name_bytes, 0x0808, 8, crc, packed as u32, size as u32, local_off);
    zip_spill(z);
    if z.out.err {
        -1
    } else {
        0
    }
}

pub fn zip_end(z: &mut ZipWriter) -> i32 {
    let cd_off = zip_pos(z) as u32;
    buf_append(z.out, &z.central.data);
    let o = &mut *z.out;
    le32(o, 0x06054b50);
    le16(o, 0);
    le16(o, 0);
    le16(o, z.count as u16);
    le16(o, z.count as u16);
    le32(o, z.central.data.len() as u32);
    le32(o, cd_off);
    let rc = buf_append(o, b"\0\0");
    zip_spill(z);
    let bad = z.central.err || z.out.err;
    buf_free(&mut z.central);
    if rc != 0 || bad {
        -1
    } else {
        0
    }
}

// The notebook's image papers (docs/FORMAT.md): raw/bg/<id>.png, index.ini and pages.ini, so a raw
// copy of the notebook draws its pages as the Kindle does. PNGs are stored, not deflated again.
fn add_raw_pictures(dir: &str, z: &mut ZipWriter, dirprefix: &str) -> i32 {
    let mut bg = String::new();
    csnprintf!(&mut bg, 760, "%s/bg", dir);
    let Ok(dd) = std::fs::read_dir(&bg) else { return 0 };
    let mut names: Vec<String> = Vec::new();
    for e in dd.flatten() {
        // (A name that is not UTF-8 is none of the three kinds below.)
        let Some(f) = e.file_name().to_str().map(|s| s.to_string()) else { continue };
        // <6 hex digits>.png
        let fb = f.as_bytes();
        let pic = fb.len() == 10 && &fb[6..] == b".png" && fb[..6].iter().all(|&c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        if !pic && f != "index.ini" && f != "pages.ini" {
            continue;
        }
        names.push(f);
    }
    names.sort();
    for nm in &names {
        let mut path = String::new();
        let mut name = String::new();
        csnprintf!(&mut path, 1000, "%s/%s", &bg, nm);
        if let Some(d) = read_file(&path) {
            csnprintf!(&mut name, 700, "%sraw/bg/%s", dirprefix, nm);
            zip_add(z, &name, &d, !nm.contains(".png"));
        }
    }
    0
}

fn add_raw_files(nb: &NotebookInfo, z: &mut ZipWriter, dirprefix: &str) -> i32 {
    let mut dir = String::new();
    store_notebook_dir(nb.id.as_str(), &mut dir, 700);
    let mut path = String::new();
    let mut name = String::new();
    // Notebook metadata, the book-page map and named sections: a notebook put back from the zip
    // keeps its navigation as well as its ink and pictures.
    const K_INFO: [&str; 5] = ["notebook.ini", "source.ini", "nav.ini", "links.ini", "doclinks.ini"];
    for k in K_INFO {
        csnprintf!(&mut path, 900, "%s/%s", &dir, k);
        let Some(d) = read_file(&path) else { continue };
        csnprintf!(&mut name, 600, "%sraw/%s", dirprefix, k);
        zip_add(z, &name, &d, true);
    }
    for i in 0..nb.page_ids.len() {
        csnprintf!(&mut path, 900, "%s/p%u.qpg", &dir, nb.page_ids[i]);
        let Some(d) = read_file(&path) else { continue };
        csnprintf!(&mut name, 600, "%sraw/p%u.qpg", dirprefix, nb.page_ids[i]);
        zip_add(z, &name, &d, true);
    }
    add_raw_pictures(&dir, z, dirprefix)
}

fn notebook_into_zip(nb: &NotebookInfo, dpi: f32, z: &mut ZipWriter, prefix: &str, dirname_: Option<&str>, pngs: bool) -> i32 {
    let mut safe = String::new();
    export_safe_name(nb.name.as_str(), &mut safe, STORE_NAME_LEN);
    let mut dirp = String::new();
    let mut name = String::new();
    csnprintf!(&mut dirp, 400, "%s%s/", prefix, dirname_.unwrap_or(&safe));
    // The PDF is made into the zip entry as its pages come (a data descriptor ends it): never whole in memory.
    csnprintf!(&mut name, 600, "%s%s.pdf", &dirp, &safe);
    if zip_add_stream(z, &name, &mut |s| export_notebook_pdf_to(nb, dpi, s, PdfCacheMode::Read, None)) != 0 {
        return -1;
    }
    if pngs {
        for i in 0..nb.npages() {
            let mut png = Buf::default();
            if export_page_png(nb, i, dpi, &mut png) == 0 {
                csnprintf!(&mut name, 600, "%spage-%03d.png", &dirp, i + 1);
                zip_add(z, &name, &png.data, false);
            }
            buf_free(&mut png);
        }
    }
    add_raw_files(nb, z, &dirp)
}

/// Notebook zip: "<name>/<name>.pdf", "<name>/page-001.png"..., "<name>/raw/" (notebook.ini + .qpg,
/// nav.ini section names, links.ini links, and bg/ with the notebook's image papers: <id>.png,
/// index.ini, pages.ini).
pub fn export_notebook_zip(nb: &NotebookInfo, dpi: f32, z: &mut ZipWriter, prefix: &str) -> i32 {
    notebook_into_zip(nb, dpi, z, prefix, None, true)
}

/// Sanitize each logical component, retaining hierarchy in folder ZIPs.
fn export_folder_components(path: &str) -> String {
    let mut result = String::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        let mut safe = String::new();
        export_safe_name(part, &mut safe, STORE_NAME_LEN);
        if !result.is_empty() {
            result.push('/');
        }
        result.push_str(&safe);
    }
    result
}

/// Every notebook in folder ("" = all notebooks, grouped "<folder>/<name>/..."). To bound memory
/// on the device, folder/all zips carry each notebook's PDF + raw files, not per-page PNGs.
pub fn export_folder_zip(folder: &str, dpi: f32, out: &mut Buf) -> i32 {
    let mut z = zip_begin(out);
    let rc = export_folder_zip_into(folder, dpi, &mut z);
    if zip_end(&mut z) != 0 {
        -1
    } else {
        rc
    }
}

/// export_folder_zip into a sink, an entry at a time.
pub fn export_folder_zip_to(folder: &str, dpi: f32, sink: &mut dyn Sink) -> i32 {
    let mut scratch = Buf::default();
    let mut z = zip_begin_to(&mut scratch, sink);
    let rc = export_folder_zip_into(folder, dpi, &mut z);
    if zip_end(&mut z) != 0 {
        -1
    } else {
        rc
    }
}

/// export_notebook_zip into a sink, an entry at a time.
pub fn export_notebook_zip_to(nb: &NotebookInfo, dpi: f32, sink: &mut dyn Sink) -> i32 {
    let mut scratch = Buf::default();
    let mut z = zip_begin_to(&mut scratch, sink);
    let rc = export_notebook_zip(nb, dpi, &mut z, "");
    if zip_end(&mut z) != 0 {
        -1
    } else {
        rc
    }
}

fn export_folder_zip_into(folder: &str, dpi: f32, z: &mut ZipWriter) -> i32 {
    let mut nbs: Vec<NotebookInfo> = Vec::new();
    if store_list(&mut nbs) != 0 {
        return -1;
    }
    // Track used "<folder>/<name>" paths to disambiguate duplicates.
    let mut used: Vec<String> = Vec::new();
    for nb in &nbs {
        if !store_folder_contains(folder, nb.folder.as_str()) {
            continue;
        }
        let sf;
        let mut sn = String::new();
        let mut prefix = String::new();
        let mut dir = String::new();
        let relative = if folder.is_empty() {
            nb.folder.as_str()
        } else {
            nb.folder.as_str().strip_prefix(folder).unwrap_or("").trim_start_matches('/')
        };
        if !relative.is_empty() {
            sf = export_folder_components(relative);
            csnprintf!(&mut prefix, 400, "%s/", &sf);
        }
        export_safe_name(nb.name.as_str(), &mut sn, STORE_NAME_LEN);
        strlcpy_q(&mut dir, &sn, 400);
        let mut k = 2;
        loop {
            let mut key = String::new();
            csnprintf!(&mut key, 400, "%s%s", &prefix, &dir);
            if !used.contains(&key) {
                let mut u = String::new();
                strlcpy_q(&mut u, &key, 400);
                used.push(u);
                break;
            }
            csnprintf!(&mut dir, 400, "%s (%d)", &sn, k);
            k += 1;
        }
        notebook_into_zip(nb, dpi, z, &prefix, Some(&dir), false);
    }
    0
}

/// Where export_mirror_pdf puts (or put) this notebook's PDF.
pub fn export_mirror_path(nb: &NotebookInfo, mirror_dir: &str, buf: &mut String, cap: usize) {
    let mut sf = String::new();
    let mut sn = String::new();
    export_safe_name(nb.name.as_str(), &mut sn, STORE_NAME_LEN);
    if !nb.folder.is_empty() {
        export_safe_name(nb.folder.as_str(), &mut sf, STORE_NAME_LEN);
        csnprintf!(buf, cap, "%s/%s/%s.pdf", mirror_dir, &sf, &sn);
    } else {
        csnprintf!(buf, cap, "%s/%s.pdf", mirror_dir, &sn);
    }
}

/// After removing/renaming a logical folder: remove only its empty mirror directory, without
/// following symlinks or removing a directory another folder uses. 1 removed, 0 kept, -1 error.
pub fn export_mirror_prune_folder(folder: &str, mirror_dir: &str) -> i32 {
    if folder.is_empty() {
        return 0; // never remove the mirror root
    }
    let mut name = String::new();
    export_safe_name(folder, &mut name, STORE_NAME_LEN);
    let cdir = cstr(mirror_dir);
    // SAFETY: open reads the NUL-terminated path we own.
    let fd = unsafe { libc::open(cdir.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        return if errno() == libc::ENOENT { 0 } else { -1 };
    }
    // SAFETY: stat is plain data; zeroed is a valid value, and fstatat fills it.
    let mut target: libc::stat = unsafe { std::mem::zeroed() };
    let cname = cstr(&name);
    // SAFETY: fstatat reads the NUL-terminated name relative to the open directory fd, and writes
    // one stat into our local.
    let rc = unsafe { libc::fstatat(fd, cname.as_ptr(), &mut target, libc::AT_SYMLINK_NOFOLLOW) };
    if rc != 0 || (target.st_mode & libc::S_IFMT) != libc::S_IFDIR {
        // SAFETY: fd is the descriptor opened above, closed once.
        unsafe { libc::close(fd) };
        return 0;
    }
    // Sanitized names may collide; case-folding filesystems and aliases may share an inode too.
    // Keep the directory if any remaining logical folder still uses it.
    let mut folders: Vec<String> = Vec::new();
    if store_list_folders(&mut folders) != 0 {
        // SAFETY: fd is the descriptor opened above, closed once.
        unsafe { libc::close(fd) };
        return -1;
    }
    let mut used = false;
    for f in &folders {
        if used {
            break;
        }
        let mut safe = String::new();
        export_safe_name(f, &mut safe, STORE_NAME_LEN);
        used = name == safe || {
            let csafe = cstr(&safe);
            // SAFETY: stat is plain data; zeroed is a valid value, and fstatat fills it.
            let mut other: libc::stat = unsafe { std::mem::zeroed() };
            // SAFETY: as the fstatat above, following symlinks this time.
            unsafe { libc::fstatat(fd, csafe.as_ptr(), &mut other, 0) == 0 && target.st_dev == other.st_dev && target.st_ino == other.st_ino }
        };
    }
    let mut rc = 0;
    if !used {
        // SAFETY: unlinkat removes the empty directory named relative to the open directory fd.
        if unsafe { libc::unlinkat(fd, cname.as_ptr(), libc::AT_REMOVEDIR) } == 0 {
            rc = 1;
        } else {
            let e = errno();
            if e != libc::ENOENT && e != libc::ENOTEMPTY && e != libc::EEXIST {
                rc = -1;
            }
        }
    }
    // A nonempty directory, including one with KOReader sidecars, is deliberately left alone.
    // SAFETY: fd is the descriptor opened above, closed once.
    unsafe { libc::close(fd) };
    rc
}

/// Writes the mirror <mirror_dir>/<folder>/<name>.pdf atomically. 0 on success. It is made into
/// `<name>.pdf.tmp` as its pages are drawn (or found in the page-stream cache, pdfcache.rs, which a
/// finished mirror brings up to date and prunes), then synced and renamed over the old one.
pub fn export_mirror_pdf(nb: &NotebookInfo, dpi: f32, mirror_dir: &str) -> i32 {
    let mut sf = String::new();
    let mut sn = String::new();
    let mut dir = String::new();
    let mut path = String::new();
    export_safe_name(nb.name.as_str(), &mut sn, STORE_NAME_LEN);
    if !nb.folder.is_empty() {
        export_safe_name(nb.folder.as_str(), &mut sf, STORE_NAME_LEN);
        csnprintf!(&mut dir, 800, "%s/%s", mirror_dir, &sf);
    } else {
        csnprintf!(&mut dir, 800, "%s", mirror_dir);
    }
    if mkdir_p(&dir) != 0 {
        return -1;
    }
    csnprintf!(&mut path, 1000, "%s/%s.pdf", &dir, &sn);
    let tmp = format!("{path}.tmp");
    let Some(mut sink) = FileSink::create(&tmp) else { return -1 };
    let rc = export_notebook_pdf_to(nb, dpi, &mut sink, PdfCacheMode::ReadWrite, None);
    let ok = rc == 0 && sink.sync();
    drop(sink);
    if !ok || std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return -1;
    }
    pdf_cache_prune(nb);
    0
}
