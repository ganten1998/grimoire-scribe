//! bgimage.h: image papers, pictures as page backgrounds (docs/FORMAT.md, Image papers). Each one
//! lives in its notebook's folder as bg/<id>.png (the id in 6 hex digits), made once from a source
//! picture fitted to the notebook's page size, and never changed after: a page drawn today and in
//! any later version looks the same. Drawing one copies it from a small cache of decoded pictures,
//! so a region drawn alone gets exactly the pixels it gets in a whole page.
//!
//! Making a picture (decoding a big JPEG, fitting it, encoding the PNG) takes seconds on the
//! Kindle: that runs in a forked child (bgjob.h, src/bgwork_template.c), never on the UI thread.
//! Drawing one decodes a page-size PNG once (it stays cached), which the UI thread can afford; a
//! file there that is no PNG, or bigger than a page picture can be, is not decoded (it draws white).
//!
//! Pictures are decoded by stb_image (third_party, compiled by build.rs as rust/csrc/stb_image_impl.c):
//! PNG, JPEG and PNM only, at most 8192 px a side, no SIMD (every build decodes a picture to the same
//! bytes), and every byte it allocates counted against a budget (below).

use crate::ffi_stb;
use crate::prelude::*;
use std::cell::RefCell;
use std::ffi::{c_int, c_void, CStr, CString};
use std::io::Read;
use std::rc::Rc;

// What a picture drawn on the UI thread may take: a notebook's own picture, a PNG of about a page
// (an RGB 1860x2480 one takes about 30 MB to decode, one of noise at 1986x2648 about 65 MB). One
// past these is no picture Grimoire made.
const BG_UI_FILE_MAX: usize = 32 << 20;
const BG_UI_DECODE_BUDGET: usize = 96 << 20;

// ---- drawing (any process: the editor, a share server child, the PDF mirror's child) ----

/// Draws image paper k of notebook nb (its id) into region of c (the canvas is the whole page),
/// within c's clip: white where the picture does not reach. A picture of another size than the
/// canvas is fitted inside it (aspect kept, centred) by the filter bgimage_normalize uses, pixel by
/// pixel from page coordinates. A missing or unreadable file draws white, is remembered (it is not
/// read again until bgimage_forget or bgimage_retry_missing), and raises bgimage_take_missing's flag.
/// The cache holds pictures by notebook and id: the same id in two notebooks is two pictures.
pub fn bgimage_render(c: &mut Canvas, nb: &str, k: PaperKind, region: Rect) {
    let r = rect_intersect(region, c.clip);
    if rect_is_empty(r) {
        return;
    }
    let img = if paper_is_image(k) && !nb.is_empty() { picture(nb, paper_image_id(k)) } else { None };
    let Some(img) = img else {
        canvas_fill(c, r, 255);
        draw(|d| d.missing = true);
        return;
    };
    // The pens are read every 64 rows of a whole page's picture, and after a fitted one (work_pump,
    // common.h).
    if img.w == c.w && img.h == c.h {
        for y in r.y..r.y + r.h {
            let (rx, rw) = (r.x as usize, r.w as usize);
            let dst = y as usize * c.stride as usize + rx;
            let src = y as usize * img.stride as usize + rx;
            c.px[dst..dst + rw].copy_from_slice(&img.px[src..src + rw]);
            if ((y - r.y) & 63) == 63 {
                work_pump();
            }
        }
        return;
    }
    // A picture made for a page of another size (a notebook an older version moved to another
    // screen): fitted as it was made, from page coordinates; white if there is no memory for it.
    let (cw, ch, cstride) = (c.w, c.h, c.stride as usize);
    if fit_draw(&img.px, img.w, img.h, img.stride as usize, 1, 0, &mut c.px, cstride, 1, cw, ch, r) != 0 {
        canvas_fill(c, r, 255);
    }
    work_pump();
}
/// Whether a page drew white since the last call because its picture was missing or unreadable:
/// for the editor to say so (renderers in forked children have nobody to tell).
pub fn bgimage_take_missing() -> bool {
    draw(|d| std::mem::replace(&mut d.missing, false))
}
/// Forgets the decoded pictures and remembered failures of notebook nb (NULL: of all of them).
pub fn bgimage_forget(nb: Option<&str>) {
    draw(|d| {
        for s in d.slot.iter_mut() {
            if s.img.is_some() && nb.is_none_or(|nb| s.nb.as_bytes() == nb.as_bytes()) {
                s.img = None;
            }
        }
        let Some(nb) = nb else {
            d.nfailed = 0;
            d.failed_next = 0;
            return;
        };
        let mut m = 0;
        for i in 0..d.nfailed {
            if d.failed[i].nb.as_bytes() != nb.as_bytes() {
                d.failed[m] = d.failed[i];
                m += 1;
            }
        }
        d.nfailed = m;
        d.failed_next = m % BG_FAILED;
    })
}
/// Forgets only the remembered failures (a job may have written the file since).
pub fn bgimage_retry_missing() {
    draw(|d| {
        d.nfailed = 0;
        d.failed_next = 0;
        d.generation = d.generation.wrapping_add(1);
    })
}
/// How many times bgimage_retry_missing ran: a screen that drew a picture white (its file was not
/// made yet: bgimage_take_missing said so) draws it again once this changes.
pub fn bgimage_generation() -> u32 {
    draw(|d| d.generation)
}
/// Whether bgimage_render draws data, a whole file of len bytes, as one of a notebook's pictures:
/// a PNG of at most 6000 px a side, within what a picture drawn on the UI thread may take (the
/// file's size, the memory to decode it). A page KOReader sent (annotate.h) is kept as it is when it
/// is, and of its notebook's page size. 0 with its size, or -1 and why.
/// (What load_picture asks of a file before it decodes it.)
pub fn bgimage_drawable(data: &[u8], w: &mut i32, h: &mut i32, why: &mut &'static str) -> i32 {
    let len = data.len();
    let mut cost = 0usize;
    *why = "bigger than a page picture can be";
    if len > BG_UI_FILE_MAX {
        return -1;
    }
    *why = "not a PNG";
    if len < 8 || data[..8] != PNG_SIG {
        return -1;
    }
    if check_within(data, len, BG_UI_DECODE_BUDGET, w, h, &mut cost, why) != 0 {
        return -1;
    }
    *why = "bigger than any page";
    if *w > 6000 || *h > 6000 {
        -1
    } else {
        0
    }
}
/// Whether notebook nb's picture id has its file but is not decoded here: no page drawn since the
/// file came was drawn with it (a picture drawn stays decoded while it is among the last few drawn).
/// Once a picture job is done, the editor draws its page again only then (editor_picture_made).
pub fn bgimage_undrawn(nb: &str, id: u32) -> bool {
    let mut dir = String::new();
    let mut path = String::new();
    if bgimage_dir(nb, &mut dir, 700) != 0 {
        return false;
    }
    if draw(|d| d.slot.iter().any(|s| s.img.is_some() && s.id == id && s.nb.as_bytes() == nb.as_bytes())) {
        return false;
    }
    bgimage_file(&dir, id, &mut path, 800);
    file_exists(&path)
}

// ---- stb_image's memory (bgimage.c's stb_malloc, stb_realloc, stb_free) ----
//
// Each block stb_image holds is listed with its size while a decode runs (stb_begin .. stb_end),
// and one that would take what it holds past the decode's budget is refused: stb_image then gives
// up ("outofmem"), and g_stb_over says the budget, not the system, ran out. Outside a decode
// nothing is listed (the picture a decode returns is an ordinary malloc block, freed with free).
// stb_image's STBI_MALLOC, STBI_REALLOC and STBI_FREE reach these through ffi_stb.rs. They hand
// out libc::malloc blocks (stb_image frees what it allocates, and the C frees the decoded picture
// with free), so they deal in raw pointers: unsafe only at the libc calls, with SAFETY comments.

const STB_BLOCKS: usize = 64; // stb_image holds a dozen at most (a JPEG: 3 a component, and its state)

/// bgimage.c's g_blk, g_stb_used, g_stb_peak, g_stb_last_peak, g_stb_budget and g_stb_over.
struct StbState {
    /// the address of a block (0: a free entry) and its size
    blk: [(usize, usize); STB_BLOCKS],
    used: usize,
    peak: usize,
    last_peak: usize,
    budget: usize,
    over: bool,
}

impl StbState {
    const fn new() -> Self {
        StbState { blk: [(0, 0); STB_BLOCKS], used: 0, peak: 0, last_peak: 0, budget: usize::MAX, over: false }
    }

    fn begin(&mut self, budget: usize) {
        self.blk = [(0, 0); STB_BLOCKS];
        self.used = 0;
        self.peak = 0;
        self.budget = budget;
        self.over = false;
    }

    fn end(&mut self) {
        self.last_peak = self.peak;
        self.blk = [(0, 0); STB_BLOCKS];
        self.used = 0;
        self.budget = usize::MAX;
    }

    fn find(&self, p: usize) -> Option<usize> {
        self.blk.iter().position(|b| b.0 == p)
    }

    /// Whether `more` bytes fit in the budget beside what is held; if not, the decode is over it.
    fn room(&mut self, more: usize) -> bool {
        if more <= self.budget - self.used {
            return true;
        }
        self.over = true;
        false
    }

    fn hold(&mut self, i: usize, p: usize, n: usize) {
        self.blk[i] = (p, n);
        self.used += n;
        if self.used > self.peak {
            self.peak = self.used;
        }
    }
}

thread_local! {
    static STB: RefCell<StbState> = const { RefCell::new(StbState::new()) };
}

fn stb<R>(f: impl FnOnce(&mut StbState) -> R) -> R {
    STB.with(|s| f(&mut s.borrow_mut()))
}

fn stb_begin(budget: usize) {
    stb(|s| s.begin(budget))
}

fn stb_end() {
    stb(|s| s.end())
}

pub(crate) fn stb_malloc(n: usize) -> *mut libc::c_void {
    let slot = stb(|s| match s.find(0) {
        Some(i) if s.room(n) => Some(i),
        _ => {
            s.over = true;
            None
        }
    });
    let Some(i) = slot else { return std::ptr::null_mut() };
    // SAFETY: malloc of at least one byte; the block goes to stb_image, which frees it through
    // stb_free (or the caller frees a decoded picture), so it is never freed twice.
    let p = unsafe { libc::malloc(if n != 0 { n } else { 1 }) };
    if !p.is_null() {
        stb(|s| s.hold(i, p as usize, n));
    }
    p
}

pub(crate) fn stb_realloc(p: *mut libc::c_void, n: usize) -> *mut libc::c_void {
    if p.is_null() {
        return stb_malloc(n);
    }
    // (never missing: stb_image reallocates only what it allocated)
    let Some((i, old)) = stb(|s| match s.find(p as usize) {
        Some(i) => Some((i, s.blk[i].1)),
        None => {
            s.over = true;
            None
        }
    }) else {
        return std::ptr::null_mut();
    };
    if n > old && !stb(|s| s.room(n - old)) {
        return std::ptr::null_mut();
    }
    // SAFETY: p is a malloc block this module handed out (it is in the list), realloc'ed to at
    // least one byte; on failure it is left as it was.
    let q = unsafe { libc::realloc(p, if n != 0 { n } else { 1 }) };
    if q.is_null() {
        return std::ptr::null_mut();
    }
    stb(|s| {
        s.used -= old;
        s.hold(i, q as usize, n);
    });
    q
}

pub(crate) fn stb_free(p: *mut libc::c_void) {
    if p.is_null() {
        return;
    }
    stb(|s| {
        if let Some(i) = s.find(p as usize) {
            s.used -= s.blk[i].1;
            s.blk[i] = (0, 0);
        }
    });
    // SAFETY: p came from stb_malloc or stb_realloc (libc::malloc), and is freed once.
    unsafe { libc::free(p) };
}

/// The most memory the last decode (bgimage_decode, or a picture drawn) held at once through
/// stb_image: for the tests, which hold it to bgimage_decode_cost.
pub fn bgimage_last_decode_peak() -> usize {
    stb(|s| s.last_peak)
}

// stb_image, called safely: each of these is one call into csrc/stb_image_impl.c.

/// stbi_info_from_memory: whether stb_image reads data's header, and its width, height and channels.
/// (data's length fits an int: the callers check it.)
fn stbi_info(data: &[u8]) -> (bool, i32, i32, i32) {
    let (mut x, mut y, mut n) = (0 as c_int, 0 as c_int, 0 as c_int);
    // SAFETY: stb_image reads at most data.len() bytes at data.as_ptr() (a live slice whose length
    // the caller has checked fits an int) and writes three ints into our locals.
    let ok = unsafe { ffi_stb::stbi_info_from_memory(data.as_ptr(), data.len() as c_int, &mut x, &mut y, &mut n) } != 0;
    (ok, x, y, n)
}

/// stbi_is_16_bit_from_memory
fn stbi_is_16(data: &[u8]) -> bool {
    // SAFETY: as in stbi_info: a read of the slice's own bytes.
    unsafe { ffi_stb::stbi_is_16_bit_from_memory(data.as_ptr(), data.len() as c_int) != 0 }
}

/// The pixels stbi_load_from_memory returns (a libc::malloc block from stb_malloc above), freed with
/// stbi_image_free as the C frees them.
struct StbPixels {
    p: *mut u8,
    len: usize,
}

impl StbPixels {
    fn bytes(&mut self) -> &mut [u8] {
        // SAFETY: p points at len bytes (width * height * channels, what stbi_load_from_memory
        // returns), owned by this value, and not aliased while the borrow lasts.
        unsafe { std::slice::from_raw_parts_mut(self.p, self.len) }
    }
}

impl Drop for StbPixels {
    fn drop(&mut self) {
        // SAFETY: p came from stbi_load_from_memory and is freed here once.
        unsafe { ffi_stb::stbi_image_free(self.p as *mut c_void) };
    }
}

/// stbi_load_from_memory(data, ..., 0): the pixels and their width, height and channels, or None.
fn stbi_load(data: &[u8]) -> Option<(StbPixels, i32, i32, i32)> {
    let (mut x, mut y, mut n) = (0 as c_int, 0 as c_int, 0 as c_int);
    // SAFETY: as in stbi_info; the pixels it returns are a malloc block of x * y * n bytes.
    let p = unsafe { ffi_stb::stbi_load_from_memory(data.as_ptr(), data.len() as c_int, &mut x, &mut y, &mut n, 0) };
    if p.is_null() {
        return None;
    }
    let len = (x as usize) * (y as usize) * (n as usize);
    Some((StbPixels { p, len }, x, y, n))
}

/// Whether stbi_failure_reason says "outofmem".
fn stbi_reason_is_outofmem() -> bool {
    // SAFETY: stbi_failure_reason returns NULL or a pointer to a static NUL-terminated string.
    let r = unsafe { ffi_stb::stbi_failure_reason() };
    // SAFETY: r is non-NULL (just checked) and NUL-terminated; it is read at once.
    !r.is_null() && unsafe { CStr::from_ptr(r) }.to_bytes() == b"outofmem"
}

// ---- files ----

/// root/notebooks/<nb>/bg (-1 for an id that is no notebook's).
pub fn bgimage_dir(nb: &str, buf: &mut String, cap: usize) -> i32 {
    let mut dir = String::new();
    if store_notebook_dir(nb, &mut dir, 600) != 0 {
        return -1;
    }
    csnprintf!(buf, cap, "%s/bg", &dir);
    0
}
/// <bgdir>/<id, 6 hex digits>.png
pub fn bgimage_file(bgdir: &str, id: u32, buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, "%s/%06x.png", bgdir, id & 0xFFFFFFu32);
}

/// access(path, F_OK) == 0
fn file_exists(path: &str) -> bool {
    let Ok(c) = CString::new(path) else { return false };
    // SAFETY: access reads the NUL-terminated path we own.
    unsafe { libc::access(c.as_ptr(), libc::F_OK) == 0 }
}

/// errno = e, for bgimage_read's callers, which read it (util::errno) after a NULL.
fn set_errno(e: i32) {
    // SAFETY: the C library's errno location is valid for the calling thread.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        *libc::__errno_location() = e;
    }
    // SAFETY: as above.
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe {
        *libc::__error() = e;
    }
}

/// A Vec of n copies of v, or None when the memory is not there (the C's malloc returning NULL).
fn try_vec<T: Clone>(n: usize, v: T) -> Option<Vec<T>> {
    let mut x = Vec::new();
    x.try_reserve_exact(n).ok()?;
    x.resize(n, v);
    Some(x)
}

pub fn bgpicture_free(p: &mut BgPicture) {
    *p = BgPicture::default();
}

// ---- reading ----

/// A whole file of at most max bytes, read into a buffer of its size (caller frees), or NULL:
/// missing or no regular file, or bigger than max (errno EFBIG), or unreadable.
/// (None leaves errno as the C does: ENOENT, EFBIG, EIO, ENOMEM or what open and read set.)
pub fn bgimage_read(path: &str, max: usize) -> Option<Vec<u8>> {
    let Ok(cpath) = CString::new(path) else {
        set_errno(libc::ENOENT);
        return None;
    };
    // SAFETY: open reads the NUL-terminated path we own.
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return None;
    }
    // SAFETY: stat is plain data; fstat fills it from our open fd.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let mut buf: Option<Vec<u8>> = None;
    let mut err = 0;
    // SAFETY: fstat writes one stat into our local through a valid pointer.
    if unsafe { libc::fstat(fd, &mut st) } != 0 || (st.st_mode & libc::S_IFMT) != libc::S_IFREG || st.st_size < 0 {
        err = libc::ENOENT;
    } else if st.st_size as u64 > max as u64 {
        err = libc::EFBIG;
    } else {
        let n = st.st_size as usize;
        let mut v: Vec<u8> = Vec::new();
        if v.try_reserve_exact(if n != 0 { n } else { 1 }).is_ok() {
            v.resize(n, 0);
            let mut got = 0usize;
            let mut ok = true;
            while got < n {
                // SAFETY: reads at most n - got bytes into v[got..], which is that long.
                let r = unsafe { libc::read(fd, v[got..].as_mut_ptr() as *mut c_void, n - got) };
                if r < 0 && errno() == libc::EINTR {
                    continue;
                }
                if r <= 0 {
                    // an error, or a file shorter than it said (it changed)
                    err = if r < 0 { errno() } else { libc::EIO };
                    ok = false;
                    break;
                }
                got += r as usize;
            }
            if ok {
                buf = Some(v);
            }
        }
        if buf.is_none() && err == 0 {
            err = libc::ENOMEM;
        }
    }
    // SAFETY: fd is ours and open.
    unsafe { libc::close(fd) };
    if buf.is_none() {
        set_errno(err);
    }
    buf
}

// ---- what a decode takes ----

fn max_z(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

// A PNG's peak by stb_image's steps (stb_image.h, stbi__parse_png_file): the compressed data
// gathered (a buffer that doubles: up to twice the file), then inflated (one filter byte a row; an
// interlaced picture's passes have more rows than stb_image guesses, so its buffer doubles), then
// unfiltered into the picture at its own depth (two rows of workspace, and for an interlaced one
// each pass's picture beside it), its palette expanded while the inflated data is still held,
// then 16 bits cut to 8 beside the picture. n: the channels stbi_info gives (a palette's 3 or 4;
// one more with a tRNS chunk).
fn png_cost(d: &[u8], file_len: usize, w: usize, h: usize, n: i32) -> usize {
    let len = d.len();
    let mut at: usize = 8; // the first chunk: IHDR, or an iPhone PNG's CgBI and then IHDR
    if len >= at + 8 && &d[at + 4..at + 8] == b"CgBI" {
        let chunk =
            (d[at] as usize) << 24 | (d[at + 1] as usize) << 16 | (d[at + 2] as usize) << 8 | d[at + 3] as usize;
        at = at.wrapping_add(12usize.wrapping_add(chunk));
    }
    if at > len || len - at < 21 || &d[at + 4..at + 8] != b"IHDR" {
        return 0;
    }
    let ih = &d[at + 8..]; // width, height, then depth, colour type, compression, filter, interlace
    let (depth, color, interlace) = (ih[8] as usize, ih[9] as i32, ih[12]);
    let palette = color == 3;
    let in_n: usize =
        if palette { 1 } else { (if color & 2 != 0 { 3 } else { 1 }) + (if color & 4 != 0 { 1 } else { 0 }) };
    let bytes: usize = if depth == 16 { 2 } else { 1 };
    let out_n: usize = if palette { 1 } else { n as usize };
    let row = (w * in_n * depth).div_ceil(8);
    let raw = (row + 1) * h;
    let out = w * h * out_n * bytes;
    let inflated = if interlace != 0 { 2 * raw } else { raw };
    let mut peak = 2 * (if file_len < (1usize << 30) { file_len } else { 1usize << 30 }) + inflated;
    peak = max_z(peak, inflated + out + 2 * row + if interlace != 0 { out } else { 0 });
    if palette {
        peak = max_z(peak, inflated + out + w * h * n as usize);
    }
    if bytes == 2 {
        peak = max_z(peak, out + w * h * out_n);
    }
    peak
}

// A JPEG's: each component's plane padded to whole blocks of its sampling (stb_image.h,
// stbi__process_frame_header), for a progressive one its coefficients too (2 bytes each), a line
// buffer each, and the picture (gray or RGB), from its frame header (SOFn).
fn jpeg_cost(d: &[u8], w: usize, h: usize) -> usize {
    let len = d.len();
    let mut i: usize = 2;
    while i + 4 <= len {
        if d[i] != 0xFF {
            // padding after a segment: stb_image scans past it too
            i += 1;
            continue;
        }
        let m = d[i + 1];
        if m == 0xFF {
            // fill
            i += 1;
            continue;
        }
        if m == 0x01 || (0xD0..=0xD8).contains(&m) {
            // markers without a length
            i += 2;
            continue;
        }
        let seg = (d[i + 2] as usize) << 8 | d[i + 3] as usize;
        if seg < 2 {
            return 0;
        }
        if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
            let f = &d[i + 4..]; // precision, height, width, components, then 3 bytes each
            let nc = if i + 10 <= len { f[5] as usize } else { 0 };
            if !(1..=4).contains(&nc) || seg < 8 + 3 * nc || i + 2 + seg > len {
                return 0;
            }
            let progressive = m == 0xC2 || m == 0xC6 || m == 0xCA || m == 0xCE;
            let (mut hmax, mut vmax) = (1usize, 1usize);
            for c in 0..nc {
                hmax = max_z(hmax, (f[7 + 3 * c] >> 4) as usize);
                vmax = max_z(vmax, (f[7 + 3 * c] & 15) as usize);
            }
            let mx = w.div_ceil(8 * hmax);
            let my = h.div_ceil(8 * vmax);
            let mut total = 0usize;
            for c in 0..nc {
                let hs = max_z((f[7 + 3 * c] >> 4) as usize, 1);
                let vs = max_z((f[7 + 3 * c] & 15) as usize, 1);
                let plane = mx * hs * 8 * my * vs * 8 + 15;
                total += plane + (if progressive { 2 * plane } else { 0 }) + w + 3;
            }
            return total + w * h * (if nc >= 3 { 3 } else { 1 }) + ffi_stb_jpeg_size();
        }
        i += 2 + seg;
    }
    0
}

/// sizeof(stbi__jpeg), the decoder's own state (csrc/stb_image_impl.c).
fn ffi_stb_jpeg_size() -> usize {
    // SAFETY: a sizeof, no arguments.
    unsafe { ffi_stb::grimoire_stbi_jpeg_size() }
}

/// What decoding a picture takes at most by its header (as above; the file's own bytes not
/// counted), or 0 when the header does not tell.
pub fn bgimage_decode_cost(data: &[u8], file_len: usize) -> usize {
    let len = data.len();
    if len < 8 || len > i32::MAX as usize {
        return 0;
    }
    stb_begin(usize::MAX);
    let (info, x, y, n) = stbi_info(data);
    let is16 = info && stbi_is_16(data);
    stb_end();
    if !info || x < 1 || y < 1 {
        return 0;
    }
    let (w, h) = (x as usize, y as usize);
    let cost = if data[0] == 0x89 && data[1] == b'P' {
        png_cost(data, file_len, w, h, n)
    } else if data[0] == 0xFF && data[1] == 0xD8 {
        jpeg_cost(data, w, h)
    } else {
        w * h * (n as usize) * (if is16 { 3 } else { 1 }) // PNM: the picture, and cut to 8 bits beside it
    };
    if cost != 0 {
        cost + (64usize << 10) // (stb_image's own state)
    } else {
        0
    }
}

fn check_within(
    data: &[u8],
    file_len: usize,
    budget: usize,
    w: &mut i32,
    h: &mut i32,
    cost: &mut usize,
    why: &mut &'static str,
) -> i32 {
    let len = data.len();
    // (stb_image tries every format it has, so its reason is the last one's: not worth giving.)
    *why = "not a PNG, JPEG or PNM picture of at most 8192 pixels a side";
    if len == 0 || len > i32::MAX as usize {
        return -1;
    }
    stb_begin(usize::MAX);
    let (info, x, y, _comp) = stbi_info(data);
    stb_end();
    if !info {
        return -1;
    }
    if x < 1 || y < 1 || x > STBI_MAX_DIMENSIONS || y > STBI_MAX_DIMENSIONS {
        *why = "more than 8192 pixels a side";
        return -1;
    }
    if (x as i64) * (y as i64) > BG_MAX_PIXELS {
        *why = "more than 40 megapixels";
        return -1;
    }
    *cost = if file_len <= budget { bgimage_decode_cost(data, file_len) } else { 0 };
    if *cost == 0 {
        return -1; // (a header stb_image takes and this does not read: none is known)
    }
    if *cost > budget - file_len {
        *why = "too big to decode in the memory Grimoire gives a picture (a 16-bit or progressive one this size)";
        return -1;
    }
    *w = x;
    *h = y;
    0
}

/// STBI_MAX_DIMENSIONS (csrc/stb_image_impl.c)
const STBI_MAX_DIMENSIONS: i32 = 8192;

/// A PNG file's first 8 bytes.
const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

// ---- making pictures (slow: forked children) ----

/// A picture: w x h pixels of ch channels (1 gray, 3 RGB), rows of w * ch bytes, top row first.
#[derive(Clone, Debug, Default)]
pub struct BgPicture {
    pub px: Vec<u8>,
    pub w: i32,
    pub h: i32,
    pub ch: i32,
}

/// The biggest picture accepted: about 40 megapixels (a 12 MP phone photo is fine), at most 8192 px
/// a side.
pub const BG_MAX_PIXELS: i64 = 40000000;
/// The memory a picture may take to decode, its file's bytes included: what its header says
/// stb_image needs (a 16-bit PNG, a palette or a progressive JPEG take several bytes a pixel more
/// than the picture itself), so no picture can take more of the Kindle's 1 GB than this. Each
/// allocation stb_image makes is counted: one whose data inflates past what its header says (a
/// "zlib bomb") fails as it reaches twice that. A 16-bit colour PNG of more than about 20 MP is
/// refused, and a progressive JPEG of more than about 35 MP (22 MP when its colour is kept at full
/// resolution, 4:4:4).
pub const BG_DECODE_BUDGET: usize = 256 << 20;
/// The biggest source picture file read (a template's is smaller still: templates.h).
pub const BG_SOURCE_MAX_BYTES: usize = 64 << 20;

/// Whether data, the first len bytes of a picture file of file_len bytes (all of it, or enough for
/// its header), is a PNG, JPEG or PNM (PGM, PPM) picture Grimoire takes: no bigger than
/// BG_MAX_PIXELS, at most 8192 px a side, decoding within BG_DECODE_BUDGET. 0 with its size, or -1
/// and why.
pub fn bgimage_check(data: &[u8], file_len: usize, w: &mut i32, h: &mut i32, why: &mut &'static str) -> i32 {
    let mut cost = 0usize;
    check_within(data, file_len, BG_DECODE_BUDGET, w, h, &mut cost, why)
}

// A channel over white by its alpha.
#[inline]
fn over_white(v: i32, a: i32) -> u8 {
    ((v * a + 255 * (255 - a) + 127) / 255) as u8
}

// bgimage_decode within budget bytes, the data's own included.
fn decode_within(data: &[u8], budget: usize, out: &mut BgPicture, why: &mut &'static str) -> i32 {
    let len = data.len();
    *out = BgPicture::default();
    let (mut w, mut h) = (0, 0);
    let mut cost = 0usize;
    if check_within(data, len, budget, &mut w, &mut h, &mut cost, why) != 0 {
        return -1;
    }
    // stb_image may hold twice what the header says it needs (that is already the most it takes),
    // never more than the budget leaves beside the data: a picture whose data inflates past that
    // (a "zlib bomb") fails as it gets there.
    let room = budget - len; // (check_within: cost + len <= budget)
    let want = 2 * cost + (1usize << 20);
    stb_begin(if want < room { want } else { room });
    let loaded = stbi_load(data);
    let over = stb(|s| s.over);
    stb_end();
    let Some((mut px, w, h, n)) = loaded else {
        if over {
            *why = "a damaged picture (it takes more memory to decode than its size says)";
            return -1;
        }
        if stbi_reason_is_outofmem() {
            *why = "out of memory";
            return -2;
        }
        *why = "a damaged or unsupported picture";
        return -1;
    };
    let np = (w as usize) * (h as usize);
    let mut ch = if n >= 3 { 3 } else { 1 };
    {
        let px = px.bytes();
        // Transparency over white, in place (each pixel moves to a lower or the same offset).
        if n == 2 {
            for i in 0..np {
                px[i] = over_white(px[2 * i] as i32, px[2 * i + 1] as i32);
            }
        } else if n == 4 {
            for i in 0..np {
                let a = px[4 * i + 3] as i32;
                for k in 0..3 {
                    px[3 * i + k] = over_white(px[4 * i + k] as i32, a);
                }
            }
        }
        if ch == 3 {
            let mut gray = true;
            let mut i = 0;
            while i < np && gray {
                gray = px[3 * i] == px[3 * i + 1] && px[3 * i + 1] == px[3 * i + 2];
                i += 1;
            }
            if gray {
                for i in 0..np {
                    px[i] = px[3 * i];
                }
                ch = 1;
            }
        }
    }
    // (The C shrinks the block with realloc; the Rust keeps the picture in a Vec, copied from it.)
    let keep = np * ch as usize;
    let mut v: Vec<u8> = Vec::new();
    if v.try_reserve_exact(keep).is_err() {
        *why = "out of memory";
        return -2;
    }
    v.extend_from_slice(&px.bytes()[..keep]);
    *out = BgPicture { px: v, w, h, ch };
    0
}

/// Decodes one: transparency over white (8 bits a channel; 16-bit PNGs are cut to 8), and gray
/// (ch 1) when no pixel has colour, else RGB. 0; -1 and why when the picture is at fault (damaged,
/// unsupported, too big), so trying again gives the same; -2 when the memory it needed was not
/// there (another time it may be).
pub fn bgimage_decode(data: &[u8], out: &mut BgPicture, why: &mut &'static str) -> i32 {
    decode_within(data, BG_DECODE_BUDGET, out, why)
}

// ---- fitting: the area filter ----

// Where a sw x sh picture goes on a dw x dh page: fitted inside it, aspect kept (rounded to whole
// pixels), centred.
fn fit_box(sw: i32, sh: i32, dw: i32, dh: i32) -> Rect {
    let (mut fw, mut fh);
    if (dw as i64) * (sh as i64) <= (dh as i64) * (sw as i64) {
        // as wide as the page
        fw = dw;
        fh = (((sh as i64) * (dw as i64) * 2 + sw as i64) / ((sw as i64) * 2)) as i32;
    } else {
        // as tall as the page
        fh = dh;
        fw = (((sw as i64) * (dh as i64) * 2 + sh as i64) / ((sh as i64) * 2)) as i32;
    }
    fw = clampi(fw, 1, dw);
    fh = clampi(fh, 1, dh);
    Rect { x: (dw - fw) / 2, y: (dh - fh) / 2, w: fw, h: fh }
}

// Destination pixel u of n covers source pixels [u * s / n, (u + 1) * s / n) of s, each by the part
// of it inside: taps first..first + count - 1, their weights at wt[at] on. The weights are 12-bit
// fixed point adding up to 4096 exactly, rounded from the running total, so a pixel's weights
// depend on u, s and n alone.
#[derive(Clone, Copy, Default)]
struct Taps {
    first: i32,
    count: i32,
    at: usize,
}
const TAP_ONE: u32 = 4096;

/// Fills tp for the destination pixels u0..u1 and returns their weights, or None out of memory.
fn taps_make(s: i32, n: i32, u0: i32, u1: i32, tp: &mut [Taps]) -> Option<Vec<u16>> {
    let mut total = 0usize;
    for u in u0..u1 {
        let lo = (u as i64) * (s as i64);
        let hi = lo + s as i64;
        total += ((hi - 1) / n as i64 - lo / n as i64 + 1) as usize;
    }
    let mut wt: Vec<u16> = Vec::new();
    wt.try_reserve_exact(if total != 0 { total } else { 1 }).ok()?;
    let mut at = 0usize;
    for u in u0..u1 {
        let lo = (u as i64) * (s as i64);
        let hi = lo + s as i64;
        let i0 = (lo / n as i64) as i32;
        let i1 = ((hi - 1) / n as i64) as i32;
        tp[(u - u0) as usize] = Taps { first: i0, count: i1 - i0 + 1, at };
        let mut cum: i64 = 0;
        let mut prev: i32 = 0;
        for i in i0..=i1 {
            let a = if (i as i64) * (n as i64) > lo { (i as i64) * (n as i64) } else { lo };
            let b = if ((i + 1) as i64) * (n as i64) < hi { ((i + 1) as i64) * (n as i64) } else { hi };
            cum += b - a;
            let q = ((cum * TAP_ONE as i64 + (s / 2) as i64) / s as i64) as i32;
            wt.push((q - prev) as u16);
            at += 1;
            prev = q;
        }
    }
    Some(wt)
}

// Draws channel c of src (sw x sh, sch channels a pixel, sstride bytes a row) fitted inside a
// dw x dh page into rect r of that page in dst (dch channels a pixel, channel c, dstride bytes a
// row, dst at the page's top left): white outside the picture. Each pixel depends on its page
// coordinates alone, so a region gets exactly what the whole page gets there. 0, or -1 out of
// memory.
// (The loops index by position, as the C does; and the C's long argument list stays.)
#[allow(clippy::needless_range_loop, clippy::too_many_arguments)]
fn fit_draw(
    src: &[u8],
    sw: i32,
    sh: i32,
    sstride: usize,
    sch: usize,
    c: usize,
    dst: &mut [u8],
    dstride: usize,
    dch: usize,
    dw: i32,
    dh: i32,
    r: Rect,
) -> i32 {
    let bx = fit_box(sw, sh, dw, dh);
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: dw, h: dh });
    for y in r.y..r.y + r.h {
        let row = y as usize * dstride + c;
        let band = y < bx.y || y >= bx.y + bx.h;
        for x in r.x..r.x + r.w {
            if band || x < bx.x || x >= bx.x + bx.w {
                dst[row + x as usize * dch] = 255;
            }
        }
    }
    let inr = rect_intersect(r, bx);
    if rect_is_empty(inr) {
        return 0;
    }
    let (u0, v0) = (inr.x - bx.x, inr.y - bx.y);
    let (Some(mut tx), Some(mut ty), Some(mut acc)) = (
        try_vec(inr.w as usize, Taps::default()),
        try_vec(inr.h as usize, Taps::default()),
        try_vec(inr.w as usize, 0u32),
    ) else {
        return -1;
    };
    let Some(wx) = taps_make(sw, bx.w, u0, u0 + inr.w, &mut tx) else { return -1 };
    let Some(wy) = taps_make(sh, bx.h, v0, v0 + inr.h, &mut ty) else { return -1 };
    for v in 0..inr.h as usize {
        acc.fill(0);
        let t = ty[v];
        for k in 0..t.count as usize {
            let a = wy[t.at + k] as u32;
            if a == 0 {
                continue;
            }
            let row = &src[(t.first as usize + k) * sstride + c..];
            for u in 0..inr.w as usize {
                let x = tx[u];
                let w = &wx[x.at..x.at + x.count as usize];
                let base = x.first as usize * sch;
                let mut hsum: u32 = 0; // at most 255 * 4096
                for (m, &wm) in w.iter().enumerate() {
                    hsum += wm as u32 * row[base + m * sch] as u32;
                }
                acc[u] += a * hsum; // in all at most 255 * 4096 * 4096 < 2^32
            }
        }
        let out = (inr.y as usize + v) * dstride + inr.x as usize * dch + c;
        for u in 0..inr.w as usize {
            dst[out + u * dch] = ((acc[u] + (1u32 << 23)) >> 24) as u8;
        }
    }
    0
}

/// The picture as it is kept for a pw x ph page: fitted inside the page with its aspect kept,
/// centred on white, by an area filter (each page pixel averages the part of the picture it covers,
/// in 12-bit fixed point, so every build gets the same bytes). Its channels stay as they are.
pub fn bgimage_normalize(src: &BgPicture, pw: i32, ph: i32, out: &mut BgPicture) -> i32 {
    *out = BgPicture::default();
    if src.px.is_empty() || src.w < 1 || src.h < 1 || (src.ch != 1 && src.ch != 3) || pw < 1 || ph < 1 {
        return -1;
    }
    let ch = src.ch as usize;
    let stride = pw as usize * ch;
    let Some(mut px) = try_vec(stride * ph as usize, 0u8) else { return -1 };
    for c in 0..ch {
        if fit_draw(
            &src.px,
            src.w,
            src.h,
            src.w as usize * ch,
            ch,
            c,
            &mut px,
            stride,
            ch,
            pw,
            ph,
            Rect { x: 0, y: 0, w: pw, h: ph },
        ) != 0
        {
            return -1;
        }
    }
    *out = BgPicture { px, w: pw, h: ph, ch: src.ch };
    0
}

/// What a gray screen shows of it: its luma (Rec. 709: 0.2126 R + 0.7152 G + 0.0722 B), w x h.
pub fn bgimage_luma(p: &BgPicture, out: &mut Canvas) -> i32 {
    if canvas_init(out, p.w, p.h) != 0 {
        return -1;
    }
    let np = p.w as usize * p.h as usize;
    if p.ch == 1 {
        out.px[..np].copy_from_slice(&p.px[..np]);
        return 0;
    }
    for i in 0..np {
        let q = &p.px[3 * i..3 * i + 3];
        out.px[i] = ((2126u32 * q[0] as u32 + 7152u32 * q[1] as u32 + 722u32 * q[2] as u32 + 5000u32) / 10000u32) as u8;
    }
    0
}

/// Writes it as a PNG (gray or RGB), atomically. 0 on success.
pub fn bgimage_write_png(path: &str, p: &BgPicture) -> i32 {
    let mut len: c_int = 0;
    // SAFETY: stb_image_write reads w * ch * h bytes from p.px (a picture holds that many; the
    // stride is a row's bytes) and returns a malloc block of len bytes, or NULL.
    let png = unsafe { ffi_stb::stbi_write_png_to_mem(p.px.as_ptr(), p.w * p.ch, p.w, p.h, p.ch, &mut len) };
    if png.is_null() {
        return -1;
    }
    // SAFETY: png points at len bytes of the block just returned, which stays valid until the free
    // below; write_file_atomic only reads the slice and has returned by then.
    let rc = write_file_atomic(path, unsafe { std::slice::from_raw_parts(png, len as usize) });
    // SAFETY: png is stb_image_write's malloc block, freed once.
    unsafe { libc::free(png as *mut c_void) };
    rc
}

// ---- content keys ----

fn mix(mut h: u64, v: u64) -> u64 {
    h ^= v;
    h = h.wrapping_mul(0x9E3779B97F4A7C15);
    h ^ (h >> 29)
}

fn hash_bytes(mut h: u64, d: &[u8]) -> u64 {
    let mut words = d.chunks_exact(8);
    for w in words.by_ref() {
        // little-endian, on any CPU
        h = mix(h, u64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]));
    }
    let rest = words.remainder();
    let mut v = (rest.len() as u64) << 56;
    for (i, &b) in rest.iter().enumerate() {
        v |= (b as u64) << (8 * i);
    }
    mix(h, v)
}

fn finish(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51AFD7ED558CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CEB9FE1A85EC53);
    h ^ (h >> 33)
}

fn seed(what: &str) -> u64 {
    hash_bytes(0x243F6A8885A308D3, what.as_bytes())
}

/// Content keys (64 bits): a kept picture's (its size, channels and pixels), and a template's
/// before it is made (a hash of the source file's bytes, with their length and the page size it is
/// made for). docs/FORMAT.md says how ids come from them.
pub fn bgimage_key(p: &BgPicture) -> u64 {
    let mut h = seed("grimoire picture 1");
    h = mix(mix(mix(h, p.w as u64), p.h as u64), p.ch as u64);
    let n = p.w as usize * p.h as usize * p.ch as usize;
    finish(hash_bytes(h, &p.px[..n]))
}
/// A file's bytes as one 64-bit hash: in memory, or read a piece at a time (never all of it in
/// memory: 0 with *hash and *len, or -1 when it does not read or changed while it was read).
pub fn bgimage_content_hash(data: &[u8]) -> u64 {
    finish(hash_bytes(mix(seed("grimoire file 1"), data.len() as u64), data))
}

/// fread: fills buf from f unless the file ends first; the count, and whether a read failed.
fn fread(f: &mut std::fs::File, buf: &mut [u8]) -> (usize, bool) {
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return (n, true),
        }
    }
    (n, false)
}

pub fn bgimage_content_hash_file(path: &str, hash: &mut u64, len: &mut usize) -> i32 {
    let Ok(mut f) = std::fs::File::open(path) else { return -1 };
    let Ok(md) = f.metadata() else { return -1 };
    if !md.is_file() {
        return -1;
    }
    let total = md.len() as usize;
    let mut got = 0usize;
    let mut h = mix(seed("grimoire file 1"), total as u64);
    const PIECE: usize = 1 << 16; // a multiple of 8: whole words until the last piece
    let mut buf = vec![0u8; PIECE];
    let (mut n, mut failed);
    loop {
        (n, failed) = fread(&mut f, &mut buf);
        if failed || n != PIECE {
            break;
        }
        for w in buf.chunks_exact(8) {
            h = mix(h, u64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]));
        }
        got += PIECE;
    }
    let ok = !failed && got + n == total;
    if ok {
        *hash = finish(hash_bytes(h, &buf[..n])); // the rest: its whole words, then the tail
        *len = total;
    }
    if ok {
        0
    } else {
        -1
    }
}
/// A template's key from its file's hash and length, for a pw x ph page.
pub fn bgimage_template_key(content: u64, len: usize, pw: i32, ph: i32) -> u64 {
    let h = seed("grimoire template 2");
    finish(mix(mix(mix(mix(h, pw as u64), ph as u64), len as u64), content))
}
/// The same from the file's bytes.
pub fn bgimage_source_key(data: &[u8], pw: i32, ph: i32) -> u64 {
    bgimage_template_key(bgimage_content_hash(data), data.len(), pw, ph)
}
/// A photo's key (templates.h, photos), as a template's but of its own kind: a photo is turned
/// upright before it is fitted, so the same file as a template and as a photo makes two pictures.
pub fn bgimage_photo_key(content: u64, len: usize, pw: i32, ph: i32) -> u64 {
    let h = seed("grimoire photo 1");
    finish(mix(mix(mix(mix(h, pw as u64), ph as u64), len as u64), content))
}

// ---- EXIF orientation ----

fn rd16(p: &[u8], le: bool) -> u32 {
    if le {
        p[0] as u32 | (p[1] as u32) << 8
    } else {
        (p[0] as u32) << 8 | p[1] as u32
    }
}

fn rd32(p: &[u8], le: bool) -> u32 {
    if le {
        p[0] as u32 | (p[1] as u32) << 8 | (p[2] as u32) << 16 | (p[3] as u32) << 24
    } else {
        (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | p[3] as u32
    }
}

// The orientation in an Exif block's TIFF data (t, n bytes): IFD0's tag 0x0112, a SHORT. 1 without it.
fn tiff_orientation(t: &[u8]) -> i32 {
    let n = t.len();
    if n < 8 {
        return 1;
    }
    let le = t[0] == b'I' && t[1] == b'I';
    if !le && !(t[0] == b'M' && t[1] == b'M') {
        return 1;
    }
    if rd16(&t[2..], le) != 42 {
        return 1;
    }
    let ifd = rd32(&t[4..], le);
    if ifd as usize > n - 2 {
        return 1;
    }
    let count = rd16(&t[ifd as usize..], le);
    let mut i = 0u32;
    while i < count && i < 512 {
        let e = ifd as usize + 2 + i as usize * 12;
        if e + 12 > n {
            return 1;
        }
        if rd16(&t[e..], le) != 0x0112 {
            i += 1;
            continue;
        }
        if rd16(&t[e + 2..], le) != 3 || rd32(&t[e + 4..], le) != 1 {
            return 1; // a SHORT, one of it
        }
        let v = rd16(&t[e + 8..], le);
        return if (1..=8).contains(&v) { v as i32 } else { 1 };
    }
    1
}

/// The EXIF orientation of a JPEG photo (tag 0x0112 of its APP1 Exif block, as a phone writes it): 1 to
/// 8, or 1 when it has none (no JPEG, no Exif block, a damaged one, a value out of range). The parser
/// reads only within the file's first 64 KB and the block's own length.
pub fn bgimage_exif_orientation(data: &[u8]) -> i32 {
    let mut n = data.len();
    if n > (64usize << 10) {
        n = 64usize << 10; // (the Exif block comes first, after the start)
    }
    let d = data;
    if n < 4 || d[0] != 0xFF || d[1] != 0xD8 {
        return 1;
    }
    let mut at = 2usize;
    while at + 4 <= n {
        if d[at] != 0xFF {
            return 1;
        }
        let marker = d[at + 1];
        if marker == 0xFF {
            // (fill bytes)
            at += 1;
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            return 1; // the picture's data, or its end: no Exif
        }
        let len = rd16(&d[at + 2..], false) as usize;
        if len < 2 || at + 2 + len > n {
            return 1;
        }
        if marker == 0xE1 && len >= 8 && &d[at + 4..at + 10] == b"Exif\0\0" {
            return tiff_orientation(&d[at + 10..at + 10 + (len - 8)]);
        }
        at += 2 + len;
    }
    1
}
/// p turned and mirrored as EXIF orientation o (2 to 8) says, so that it shows upright: 3 turns it a
/// half turn, 6 a quarter turn clockwise, 8 a quarter turn the other way, 2, 4, 5 and 7 mirror it
/// first. 0 (1 and values out of range change nothing), or -1 out of memory (p as it was).
pub fn bgimage_orient(p: &mut BgPicture, o: i32) -> i32 {
    if !(2..=8).contains(&o) || p.px.is_empty() {
        return 0;
    }
    let swap = o >= 5;
    let (w, h) = (p.w, p.h);
    let (ow, oh) = if swap { (h, w) } else { (w, h) };
    let ch = p.ch as usize;
    let Some(mut out) = try_vec(ow as usize * oh as usize * ch, 0u8) else { return -1 };
    for y in 0..oh {
        for x in 0..ow {
            // the stored pixel that shows at (x, y) upright
            let (sx, sy) = match o {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (y, h - 1 - x),
                7 => (w - 1 - y, h - 1 - x),
                _ => (w - 1 - y, x), // 8
            };
            let d = (y as usize * ow as usize + x as usize) * ch;
            let s = (sy as usize * w as usize + sx as usize) * ch;
            out[d..d + ch].copy_from_slice(&p.px[s..s + ch]);
        }
    }
    p.px = out;
    p.w = ow;
    p.h = oh;
    0
}

// ---- bg/index.ini: which id holds which key in a notebook's bg folder ----

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BgIndexEntry {
    pub id: u32,
    pub key: u64,
}
#[derive(Clone, Debug, Default)]
pub struct BgIndex {
    /// the C's e, n, cap
    pub e: Vec<BgIndexEntry>,
}

fn index_path(bgdir: &str, buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, "%s/index.ini", bgdir);
}

fn index_add(ix: &mut BgIndex, id: u32, key: u64) -> i32 {
    if ix.e.try_reserve(1).is_err() {
        return -1;
    }
    ix.e.push(BgIndexEntry { id, key });
    0
}

/// strtoul / strtoull in base 16: the value (what a number out of range or negative gives, cut to
/// max bits as the C's unsigned long is) and where the number ends in s, 0 when there is none (the
/// C's end pointer is then its first argument).
fn c_strtou_hex(s: &[u8], max: u64) -> (u64, usize) {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || (9..=13).contains(&s[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    // (a 0x prefix counts only when a digit follows it)
    if s.get(i) == Some(&b'0')
        && matches!(s.get(i + 1), Some(b'x' | b'X'))
        && s.get(i + 2).is_some_and(|c| c.is_ascii_hexdigit())
    {
        i += 2;
    }
    let start = i;
    let mut v: u64 = 0;
    let mut over = false;
    while let Some(d) = s.get(i).and_then(|&c| (c as char).to_digit(16)) {
        let d = d as u64;
        if v > (max - d) / 16 {
            over = true;
        } else {
            v = v * 16 + d;
        }
        i += 1;
    }
    if i == start {
        return (0, 0);
    }
    if over {
        return (max, i);
    }
    (if neg { v.wrapping_neg() & max } else { v }, i)
}

/// Reads <bgdir>/index.ini (none: empty). 0, or -1: out of memory, or the file is there but does
/// not read (then nothing may be saved over it: it would lose the ids it lists).
pub fn bgindex_load(bgdir: &str, ix: &mut BgIndex) -> i32 {
    *ix = BgIndex::default();
    let mut path = String::new();
    index_path(bgdir, &mut path, 700);
    let Some(txt) = read_file(&path) else {
        return if file_exists(&path) { -1 } else { 0 }; // there but unreadable: not to be saved over
    };
    // (the text is a C string: it ends at a NUL)
    let txt = &txt[..txt.iter().position(|&b| b == 0).unwrap_or(txt.len())];
    // unsigned long: 32 bits on the Kindle
    let ulong_max = if cfg!(target_pointer_width = "32") { u32::MAX as u64 } else { u64::MAX };
    let mut rc = 0;
    for line in txt.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        if rc != 0 {
            break;
        }
        if line[0] == b'#' {
            continue;
        }
        let Some(eq) = line.iter().position(|&b| b == b'=') else { continue };
        let (id, end) = c_strtou_hex(line, ulong_max);
        if end != eq || end == 0 || id == 0 || id > 0xFFFFFF {
            continue;
        }
        let (key, kend) = c_strtou_hex(&line[eq + 1..], u64::MAX);
        if kend == 0 {
            continue;
        }
        let mut have = 0;
        if bgindex_find(ix, key, &mut have) {
            continue; // (the first line for a key counts)
        }
        let taken = ix.e.iter().any(|e| e.id == id as u32);
        if !taken {
            rc = index_add(ix, id as u32, key);
        }
    }
    if rc != 0 {
        bgindex_free(ix);
    }
    rc
}
/// atomically, sorted by id
pub fn bgindex_save(bgdir: &str, ix: &BgIndex) -> i32 {
    let mut e: Vec<BgIndexEntry> = Vec::new();
    if e.try_reserve_exact(ix.e.len()).is_err() {
        return -1;
    }
    e.extend_from_slice(&ix.e);
    e.sort_by_key(|a| a.id);
    let mut text = cformat!("# Grimoire background pictures: id=key (docs/FORMAT.md)\n");
    for x in &e {
        text.push_str(&cformat!("%06x=%016llx\n", x.id, x.key));
    }
    let mut path = String::new();
    index_path(bgdir, &mut path, 700);
    write_file_atomic(&path, text.as_bytes())
}
pub fn bgindex_free(ix: &mut BgIndex) {
    *ix = BgIndex::default();
}
pub fn bgindex_find(ix: &BgIndex, key: u64, id: &mut u32) -> bool {
    for e in &ix.e {
        if e.key == key {
            *id = e.id;
            return true;
        }
    }
    false
}
/// The id for key: the one the index gives it, else a new one, added to ix (not yet saved) and
/// *added set: the key's low 24 bits, stepping to the next id past one that holds another key or
/// that has a file in bgdir with no entry (whatever it holds). 0 is never given. 0 on success.
pub fn bgindex_claim(ix: &mut BgIndex, bgdir: &str, key: u64, id: &mut u32, added: &mut bool) -> i32 {
    *added = false;
    if bgindex_find(ix, key, id) {
        return 0;
    }
    let mut cand = (key as u32) & 0xFFFFFF;
    let mut tries = 0u32;
    while tries < 0xFFFFFF {
        if cand == 0 {
            cand = 1;
        }
        let taken = ix.e.iter().any(|e| e.id == cand);
        if !taken {
            let mut path = String::new();
            bgimage_file(bgdir, cand, &mut path, 700);
            if !file_exists(&path) {
                // (else a picture no entry names: whatever it holds, not ours)
                if index_add(ix, cand, key) != 0 {
                    return -1;
                }
                *id = cand;
                *added = true;
                return 0;
            }
        }
        tries += 1;
        cand = (cand + 1) & 0xFFFFFF;
    }
    -1
}

// ---- drawing ----

const BG_SLOTS: usize = 3; // decoded pictures kept: about 4.6 MB each at 1860x2480
const BG_FAILED: usize = 16; // pictures remembered as missing or unreadable

#[derive(Default)]
struct Slot {
    nb: FixedStr<PAGE_NB_LEN>,
    id: u32,
    /// its luma; None: a free slot. (Shared, so that a draw keeps its picture while the pens are
    /// read between its rows, work_pump, whatever happens to the cache then.)
    img: Option<Rc<Canvas>>,
    /// for the least recently used
    used: u32,
}

#[derive(Clone, Copy, Default)]
struct Failed {
    nb: FixedStr<PAGE_NB_LEN>,
    id: u32,
}

/// bgimage.c's g_slot, g_clock, g_failed, g_nfailed, g_failed_next, g_missing and g_generation.
#[derive(Default)]
struct Draw {
    slot: [Slot; BG_SLOTS],
    clock: u32,
    failed: [Failed; BG_FAILED],
    nfailed: usize,
    failed_next: usize,
    missing: bool,
    generation: u32,
}

thread_local! {
    static DRAW: RefCell<Draw> = RefCell::new(Draw::default());
}

fn draw<R>(f: impl FnOnce(&mut Draw) -> R) -> R {
    DRAW.with(|d| f(&mut d.borrow_mut()))
}

fn remember_failure(nb: &str, id: u32) {
    draw(|d| {
        let at = d.failed_next;
        d.failed[at].nb.set(nb);
        d.failed[at].id = id;
        d.failed_next = (d.failed_next + 1) % BG_FAILED;
        if d.nfailed < BG_FAILED {
            d.nfailed += 1;
        }
    })
}

// Reads notebook nb's picture id as its luma. A kept picture is a PNG of about a page: a file that
// is no PNG (Grimoire writes no other), or one bigger than a page picture can be, is not decoded on
// the UI thread.
fn load_picture(nb: &str, id: u32, out: &mut Canvas, why: &mut &'static str) -> i32 {
    let mut dir = String::new();
    let mut path = String::new();
    *why = "not in this notebook";
    if bgimage_dir(nb, &mut dir, 700) != 0 {
        return -1;
    }
    bgimage_file(&dir, id, &mut path, 800);
    let Some(data) = bgimage_read(&path, BG_UI_FILE_MAX) else {
        *why = if errno() == libc::EFBIG { "bigger than a page picture can be" } else { "missing" };
        return -1;
    };
    let (mut w, mut h) = (0, 0);
    let mut cost = 0usize;
    let mut p = BgPicture::default();
    let mut rc = -1;
    if data.len() < 8 || data[..8] != PNG_SIG {
        *why = "not a PNG";
    } else if check_within(&data, data.len(), BG_UI_DECODE_BUDGET, &mut w, &mut h, &mut cost, why) != 0 {
        // (*why says what)
    } else if w > 6000 || h > 6000 {
        *why = "bigger than any page";
    } else if decode_within(&data, BG_UI_DECODE_BUDGET, &mut p, why) == 0 {
        rc = bgimage_luma(&p, out);
        if rc != 0 {
            *why = "out of memory";
        }
        bgpicture_free(&mut p);
    }
    rc
}

// Notebook nb's picture id, decoded, or NULL (missing or unreadable: remembered).
fn picture(nb: &str, id: u32) -> Option<Rc<Canvas>> {
    enum Look {
        Cached(Rc<Canvas>),
        Failed,
        Load(usize),
    }
    let look = draw(|d| {
        for i in 0..BG_SLOTS {
            if d.slot[i].img.is_some() && d.slot[i].id == id && d.slot[i].nb.as_bytes() == nb.as_bytes() {
                d.clock = d.clock.wrapping_add(1);
                d.slot[i].used = d.clock;
                return Look::Cached(d.slot[i].img.clone().unwrap());
            }
        }
        for i in 0..d.nfailed {
            if d.failed[i].id == id && d.failed[i].nb.as_bytes() == nb.as_bytes() {
                return Look::Failed;
            }
        }
        let mut s = 0; // a free slot, else the least recently used
        for i in 0..BG_SLOTS {
            if d.slot[i].img.is_none() {
                s = i;
                break;
            }
            if d.slot[i].used < d.slot[s].used {
                s = i;
            }
        }
        d.slot[s].img = None; // (freed before the next is decoded: the memory is bounded)
        Look::Load(s)
    });
    let s = match look {
        Look::Cached(img) => return Some(img),
        Look::Failed => return None,
        Look::Load(s) => s,
    };
    let mut img = Canvas::default();
    let mut why = "";
    if load_picture(nb, id, &mut img, &mut why) != 0 {
        qlog!("bg: %s: bg/%06x.png is %s; its pages show white paper", nb, id, why);
        remember_failure(nb, id);
        return None;
    }
    let img = Rc::new(img);
    draw(|d| {
        d.slot[s].nb.set(nb);
        d.slot[s].id = id;
        d.slot[s].img = Some(img.clone());
        d.clock = d.clock.wrapping_add(1);
        d.slot[s].used = d.clock;
    });
    Some(img)
}
