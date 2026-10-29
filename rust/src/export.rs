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
    links: [NavLink; NAV_LINKS_PER_PAGE as usize],
    areas: [Rect; NAV_LINKS_PER_PAGE as usize],
    n: i32,
}

fn render_page_links(nb: &NotebookInfo, index: i32, dpi: f32, layers: i32, c: &mut Canvas, paper: Option<&mut u32>, links: Option<&mut PageLinks>) -> i32 {
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
    }
    page_free(p);
    0
}

fn render_page(nb: &NotebookInfo, index: i32, dpi: f32, layers: i32, c: &mut Canvas, paper: Option<&mut u32>) -> i32 {
    render_page_links(nb, index, dpi, layers, c, paper, None)
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
/// image at the page's physical size (w/dpi inches). Title = notebook name. The section names
/// (nav.ini) are its outline, and the links drawn to pages (links.ini) are links on its pages.
pub fn export_notebook_pdf(nb: &NotebookInfo, dpi: f32, out: &mut Buf) -> i32 {
    export_notebook_pdf_progress(nb, dpi, out, None)
}

// The C's OBJ(i): notes where object i starts (from the PDF's first byte) and opens it.
fn obj(out: &mut Buf, off: &mut [usize], base: usize, i: i32) {
    off[i as usize] = out.data.len() - base;
    buf_printf!(out, "%d 0 obj\n", i);
}

/// Optional page-completion callback, including (0, total) before rendering. No global state;
/// the Share child uses it to report preparation without changing the PDF bytes.
pub fn export_notebook_pdf_progress(nb: &NotebookInfo, dpi: f32, out: &mut Buf, mut progress: Option<&mut dyn FnMut(i32, i32)>) -> i32 {
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
    let mut pl = PageLinks {
        nav: &nav,
        links: [NavLink::default(); NAV_LINKS_PER_PAGE as usize],
        areas: [Rect::default(); NAV_LINKS_PER_PAGE as usize],
        n: 0,
    };
    // Objects: 1 catalog, 2 pages, 3 info, then 3 per page (page, content, image), then the outline
    // (its root, then one per section) when there are names, then each link as its page comes.
    let outline = 4 + 3 * np;
    let mut nobj = 3 + 3 * np + if nnamed != 0 { 1 + nnamed } else { 0 };
    let mut cap = nobj + 16;
    let mut off: Vec<usize> = vec![0; cap as usize + 1];
    let base = out.data.len();
    let mut rc = 0;
    // The comment's four bytes above 127 mark the file as binary for the tools that move it. They are
    // not UTF-8, so the line cannot be a Rust format string: the C's buf_printf copies them as they
    // are, and so does this.
    buf_append(out, b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    obj(out, &mut off, base, 1);
    if nnamed != 0 {
        buf_printf!(out, "<< /Type /Catalog /Pages 2 0 R /Outlines %d 0 R /PageMode /UseOutlines >>\nendobj\n", outline);
    } else {
        buf_printf!(out, "<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    }
    obj(out, &mut off, base, 2);
    buf_printf!(out, "<< /Type /Pages /Count %d /Kids [", np);
    for i in 0..np {
        buf_printf!(out, " %d 0 R", 4 + 3 * i);
    }
    buf_printf!(out, " ] >>\nendobj\n");
    obj(out, &mut off, base, 3);
    buf_printf!(out, "<< /Title ");
    pdf_text(out, nb.name.as_bytes());
    buf_printf!(out, " /Creator (Grimoire %s) /Producer (Grimoire) >>\nendobj\n", GRIMOIRE_VERSION);
    for i in 0..np {
        if rc != 0 {
            break;
        }
        let mut c = Canvas::default();
        if render_page_links(nb, i, dpi, PAGE_LAYER_ALL, &mut c, None, Some(&mut pl)) != 0 {
            rc = -1;
            break;
        }
        let pw = (c.w as f32) * 72.0 / dpi;
        let ph = (c.h as f32) * 72.0 / dpi;
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
        if nobj + nannot > cap {
            let grown = (nobj + nannot) * 2;
            if off.try_reserve_exact((grown as usize + 1).saturating_sub(off.len())).is_err() {
                canvas_free(&mut c);
                rc = -1;
                break;
            }
            off.resize(grown as usize + 1, 0);
            cap = grown;
        }
        nobj += nannot;
        obj(out, &mut off, base, pobj);
        buf_printf!(
            out,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %.2f %.2f] /Contents %d 0 R /Resources << /XObject << /Im0 %d 0 R >> >>",
            pw,
            ph,
            cobj,
            iobj
        );
        if nannot != 0 {
            buf_printf!(out, " /Annots [");
            for k in 0..nannot {
                buf_printf!(out, " %d 0 R", first_annot + k);
            }
            buf_printf!(out, " ]");
        }
        buf_printf!(out, " >>\nendobj\n");
        for k in 0..nannot {
            let r = pl.areas[k as usize];
            let s = 72.0f32 / dpi;
            obj(out, &mut off, base, first_annot + k);
            buf_printf!(
                out,
                "<< /Type /Annot /Subtype /Link /Rect [%.2f %.2f %.2f %.2f] /Border [0 0 0] /Dest [%d 0 R /Fit] >>\nendobj\n",
                (r.x as f32) * s,
                fmla(-((r.y + r.h) as f32), s, ph),
                ((r.x + r.w) as f32) * s,
                fmla(-(r.y as f32), s, ph),
                4 + 3 * to[k as usize]
            );
        }
        let mut content = String::new();
        let cl = csnprintf!(&mut content, 128, "q %.2f 0 0 %.2f 0 0 cm /Im0 Do Q\n", pw, ph);
        obj(out, &mut off, base, cobj);
        buf_printf!(out, "<< /Length %d >>\nstream\n%s\nendstream\nendobj\n", cl, &content);
        let z = zlib_compress(&c.px[..(c.w * c.h) as usize]);
        canvas_free(&mut c);
        let Some(z) = z else {
            rc = -1;
            break;
        };
        let zl = z.len as i32;
        obj(out, &mut off, base, iobj);
        buf_printf!(
            out,
            "<< /Type /XObject /Subtype /Image /Width %d /Height %d /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length %d >>\nstream\n",
            (pw * dpi / 72.0 + 0.5) as i32,
            (ph * dpi / 72.0 + 0.5) as i32,
            zl
        );
        buf_append(out, z.as_slice());
        buf_printf!(out, "\nendstream\nendobj\n");
        drop(z);
        if out.err {
            rc = -1;
        }
        if rc == 0 {
            if let Some(f) = progress.as_deref_mut() {
                f(i + 1, np);
            }
        }
    }
    if rc == 0 && nnamed != 0 {
        obj(out, &mut off, base, outline);
        buf_printf!(out, "<< /Type /Outlines /First %d 0 R /Last %d 0 R /Count %d >>\nendobj\n", outline + 1, outline + nnamed, nnamed);
        for k in 0..nnamed {
            obj(out, &mut off, base, outline + 1 + k);
            buf_printf!(out, "<< /Title ");
            pdf_text(out, nav_name(&nav, nb.page_ids[named[k as usize] as usize]).as_bytes());
            buf_printf!(out, " /Parent %d 0 R", outline);
            if k != 0 {
                buf_printf!(out, " /Prev %d 0 R", outline + k);
            }
            if k + 1 < nnamed {
                buf_printf!(out, " /Next %d 0 R", outline + 2 + k);
            }
            buf_printf!(out, " /Dest [%d 0 R /Fit] >>\nendobj\n", 4 + 3 * named[k as usize]);
        }
    }
    nav_free(&mut nav);
    if rc == 0 {
        let xref = out.data.len() - base;
        buf_printf!(out, "xref\n0 %d\n0000000000 65535 f \n", nobj + 1);
        for i in 1..=nobj {
            buf_printf!(out, "%010zu 00000 n \n", off[i as usize]);
        }
        buf_printf!(out, "trailer\n<< /Size %d /Root 1 0 R /Info 3 0 R >>\nstartxref\n%zu\n%%%%EOF\n", nobj + 1, xref);
    }
    if rc != 0 || out.err {
        -1
    } else {
        0
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

/// A zip (stored or deflated entries, CRC32, no zip64) streaming builder.
#[derive(Debug)]
pub struct ZipWriter<'a> {
    pub out: &'a mut Buf,
    pub central: Buf,
    pub count: i32,
}
/// (The C's zip_begin(z, out) fills a ZipWriter in place; here it makes one.)
pub fn zip_begin(out: &mut Buf) -> ZipWriter<'_> {
    ZipWriter { out, central: Buf::default(), count: 0 }
}

pub fn zip_add(z: &mut ZipWriter, name: &str, data: &[u8], deflate: bool) -> i32 {
    let len = data.len();
    if len > 0xFFFFFFF0 || z.out.data.len() > 0xFFFFFFF0 || z.count >= 0xFFFF {
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
    let local_off = z.out.data.len() as u32;
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
    let c = &mut z.central;
    le32(c, 0x02014b50);
    le16(c, 0x0314); // made by: unix, 2.0
    le16(c, 20);
    le16(c, 0x0800);
    le16(c, method);
    le16(c, t);
    le16(c, d);
    le32(c, crc);
    le32(c, plen as u32);
    le32(c, len as u32);
    le16(c, nl);
    le16(c, 0);
    le16(c, 0);
    le16(c, 0);
    le16(c, 0);
    le32(c, 0o100644u32 << 16);
    le32(c, local_off);
    buf_append(c, name_bytes);
    z.count += 1;
    rc
}

pub fn zip_end(z: &mut ZipWriter) -> i32 {
    let cd_off = z.out.data.len() as u32;
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
    let bad = z.central.err || o.err;
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
    const K_INFO: [&str; 4] = ["notebook.ini", "source.ini", "nav.ini", "links.ini"];
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
    let mut pdf = Buf::default();
    if export_notebook_pdf(nb, dpi, &mut pdf) != 0 {
        buf_free(&mut pdf);
        return -1;
    }
    csnprintf!(&mut name, 600, "%s%s.pdf", &dirp, &safe);
    zip_add(z, &name, &pdf.data, false);
    buf_free(&mut pdf);
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
    let mut nbs: Vec<NotebookInfo> = Vec::new();
    if store_list(&mut nbs) != 0 {
        return -1;
    }
    let mut z = zip_begin(out);
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
        notebook_into_zip(nb, dpi, &mut z, &prefix, Some(&dir), false);
    }
    zip_end(&mut z)
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

/// Writes the mirror <mirror_dir>/<folder>/<name>.pdf atomically. 0 on success.
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
    let mut pdf = Buf::default();
    let mut rc = export_notebook_pdf(nb, dpi, &mut pdf);
    if rc == 0 {
        rc = write_file_atomic(&path, &pdf.data);
    }
    buf_free(&mut pdf);
    rc
}
