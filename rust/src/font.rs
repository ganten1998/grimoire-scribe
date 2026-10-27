//! font.h: TrueType text via stb_truetype, fonts embedded at build time
//! (build/gen/fonts.c, from assets/*.ttf via xxd -i). Glyph bitmaps are cached per size.

use crate::assets::{FONT_JACQUARD24, FONT_JERSEY10, FONT_TERMINESS_BOLD, FONT_TERMINESS_REGULAR};
use crate::ffi_stb::*;
use crate::prelude::*;
use std::cell::RefCell;

/// What the text is for; the theme picks the typeface. Classic: all Terminess. Gothic: pixel
/// faces, Jersey 10 for text and labels (overprinted one design pixel for bold, except where
/// that would fill in an accent or turn a letter into another), Jacquard 24 (pixel blackletter)
/// for titles. FIXED faces are Terminess in every theme: paper labels.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFace {
    #[default]
    Regular = 0,  // FONT_REGULAR
    Bold = 1,  // FONT_BOLD
    Title = 2,  // FONT_TITLE
    Caps = 3,  // FONT_CAPS
    Fixed = 4,  // FONT_FIXED
    FixedBold = 5,  // FONT_FIXED_BOLD
}
impl FontFace {
    /// FONT_FACE_COUNT
    pub const COUNT: usize = 6;
}

// font.c: TrueType text through stb_truetype with a small glyph cache. Pixel fonts (drawn on
// a grid of `unit` font units) are rendered one bitmap pixel per design pixel, without
// smoothing, then blown up to whole k x k blocks, so they stay razor sharp at any size.

// The embedded TrueType files. Terminess (with the Nerd Font icons) is always there; the
// pixel faces belong to the gothic theme: Jacquard 24 (pixel blackletter) for titles, Jersey
// 10 for text and labels.
const TTF_TERM: usize = 0;
const TTF_TERM_BOLD: usize = 1;
const TTF_JACQUARD: usize = 2;
const TTF_JERSEY: usize = 3;
const TTF_COUNT: usize = 4;

/// Font units per design pixel (0 = an outline font, drawn smooth). Measured: every outline
/// point of these files sits on this grid.
const K_UNIT: [i32; TTF_COUNT] = {
    let mut k = [0; TTF_COUNT];
    k[TTF_JACQUARD] = 30;
    k[TTF_JERSEY] = 75;
    k
};

/// What a FontFace means under the current theme: which file, how much bigger than asked, and
/// whether to embolden by overprinting (one design pixel for pixel faces).
#[derive(Clone, Copy)]
struct FaceMap {
    ttf: usize,
    scale: f32,
    embolden: bool,
}

const fn fm(ttf: usize, scale: f32, embolden: bool) -> FaceMap {
    FaceMap { ttf, scale, embolden }
}

// The glyph cache: one slot per (face, size, code point), probed from a hash.
#[derive(Default)]
struct Glyph {
    used: bool,
    /// TTF_* index
    face: u8,
    /// size * 4, rounded; pixel faces: -k
    size4: i32,
    cp: u32,
    w: i32,
    h: i32,
    xoff: i32,
    yoff: i32,
    /// unscaled advance (font units)
    advance: i32,
    /// empty where the C has NULL (a glyph with no ink, a space)
    bmp: Vec<u8>,
    /// pixel glyphs above U+007F: the bold weight, w + k wide (see pixel_bold); empty until made
    bold: Vec<u8>,
}

const CACHE_SLOTS: usize = 1024;

struct State {
    /// one stbtt_fontinfo per embedded file, boxed so it keeps its address once stbtt_InitFont has
    /// filled it
    font: [Box<stbtt_fontinfo>; TTF_COUNT],
    ready: bool,
    map: [FaceMap; FontFace::COUNT],
    gothic: bool,
    cache: Vec<Glyph>,
    cache_used: i32,
    overflow: Option<FontOverflowHook>,
}

impl State {
    fn new() -> State {
        State {
            font: std::array::from_fn(|_| Box::new(stbtt_fontinfo::default())),
            ready: false,
            map: [fm(0, 0.0, false); FontFace::COUNT],
            gothic: false,
            cache: (0..CACHE_SLOTS).map(|_| Glyph::default()).collect(),
            cache_used: 0,
            overflow: None,
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::new());
}
// Each borrow is one primitive's: nothing here calls the overflow hook, another module or itself
// while holding it.
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// ---- stb_truetype, one safe function per call ----

fn init_font(info: &mut stbtt_fontinfo, data: &'static [u8]) -> bool {
    // SAFETY: data is one of the embedded TrueType files, 'static, so the pointer stb keeps in info
    // stays valid; info is our own boxed storage, larger than stb's struct (ffi_stb.rs).
    unsafe { stbtt_InitFont(info, data.as_ptr(), stbtt_GetFontOffsetForIndex(data.as_ptr(), 0)) != 0 }
}

fn vmetrics(fi: &stbtt_fontinfo) -> (i32, i32, i32) {
    let (mut a, mut d, mut l) = (0, 0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the three outputs are our locals.
    unsafe { stbtt_GetFontVMetrics(fi, &mut a, &mut d, &mut l) };
    (a, d, l)
}

fn scale_for_pixel_height(fi: &stbtt_fontinfo, px: f32) -> f32 {
    // SAFETY: fi was filled by stbtt_InitFont.
    unsafe { stbtt_ScaleForPixelHeight(fi, px) }
}

fn find_glyph_index(fi: &stbtt_fontinfo, cp: u32) -> i32 {
    // SAFETY: fi was filled by stbtt_InitFont.
    unsafe { stbtt_FindGlyphIndex(fi, cp as i32) }
}

/// (advance, left side bearing) in font units
fn hmetrics(fi: &stbtt_fontinfo, cp: u32) -> (i32, i32) {
    let (mut adv, mut lsb) = (0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the outputs are our locals.
    unsafe { stbtt_GetCodepointHMetrics(fi, cp as i32, &mut adv, &mut lsb) };
    (adv, lsb)
}

fn kern_advance(fi: &stbtt_fontinfo, a: u32, b: u32) -> i32 {
    // SAFETY: fi was filled by stbtt_InitFont.
    unsafe { stbtt_GetCodepointKernAdvance(fi, a as i32, b as i32) }
}

/// The glyph's ink box (x0, y0, x1, y1) in font units, None if the font has no such glyph.
fn codepoint_box(fi: &stbtt_fontinfo, cp: u32) -> Option<(i32, i32, i32, i32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (0, 0, 0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the outputs are our locals.
    let ok = unsafe { stbtt_GetCodepointBox(fi, cp as i32, &mut x0, &mut y0, &mut x1, &mut y1) };
    if ok == 0 {
        None
    } else {
        Some((x0, y0, x1, y1))
    }
}

/// stbtt_GetCodepointBitmap: the 8-bit coverage (empty where stb returns NULL: a glyph with no
/// ink), and its width, height and offsets. The bitmap stb allocated is copied and freed here, so
/// no raw pointer outlives this call.
fn codepoint_bitmap(fi: &stbtt_fontinfo, sx: f32, sy: f32, cp: u32) -> (Vec<u8>, i32, i32, i32, i32) {
    let (mut w, mut h, mut xo, mut yo) = (0, 0, 0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the four outputs are our locals.
    let p = unsafe { stbtt_GetCodepointBitmap(fi, sx, sy, cp as i32, &mut w, &mut h, &mut xo, &mut yo) };
    let mut bmp = Vec::new();
    if !p.is_null() {
        if w > 0 && h > 0 {
            // SAFETY: on success stb returns w * h bytes (it returns NULL when either is 0).
            bmp.extend_from_slice(unsafe { std::slice::from_raw_parts(p, w as usize * h as usize) });
        }
        // SAFETY: p came from stbtt_GetCodepointBitmap (malloc), is not used again, and no
        // userdata was given to the allocator.
        unsafe { stbtt_FreeBitmap(p, std::ptr::null_mut()) };
    }
    (bmp, w, h, xo, yo)
}

// ---- faces ----

fn map_faces(st: &mut State, gothic: bool) {
    const CLASSIC: [FaceMap; FontFace::COUNT] = [
        fm(TTF_TERM, 1.0, false),      // FONT_REGULAR
        fm(TTF_TERM_BOLD, 1.0, false), // FONT_BOLD
        fm(TTF_TERM_BOLD, 1.0, false), // FONT_TITLE
        fm(TTF_TERM_BOLD, 1.0, false), // FONT_CAPS
        fm(TTF_TERM, 1.0, false),      // FONT_FIXED
        fm(TTF_TERM_BOLD, 1.0, false), // FONT_FIXED_BOLD
    ];
    const GOTH: [FaceMap; FontFace::COUNT] = [
        fm(TTF_JERSEY, 1.15, false),   // FONT_REGULAR
        fm(TTF_JERSEY, 1.15, true),    // FONT_BOLD
        fm(TTF_JACQUARD, 1.1, false),  // FONT_TITLE
        fm(TTF_JERSEY, 1.15, true),    // FONT_CAPS
        fm(TTF_TERM, 1.0, false),      // FONT_FIXED
        fm(TTF_TERM_BOLD, 1.0, false), // FONT_FIXED_BOLD
    ];
    st.map = if gothic { GOTH } else { CLASSIC };
}

/// Pixel faces: how many screen pixels one design pixel becomes at a requested line height.
fn pixel_k(st: &State, ttf: usize, px: f32) -> i32 {
    let (a, d, _l) = vmetrics(&st.font[ttf]);
    let line = (a - d) as f32 / K_UNIT[ttf] as f32;
    let k = (px / line).round_ties_even() as i32;
    if k < 1 {
        1
    } else {
        k
    }
}

fn cache_flush(st: &mut State) {
    // (the bitmaps are Vecs: dropping a slot frees what stbtt_FreeBitmap and free free in the C)
    for g in st.cache.iter_mut() {
        *g = Glyph::default();
    }
    st.cache_used = 0;
}

/// 0 on success
pub fn font_init() -> i32 {
    let failed = with(|st| {
        if st.ready {
            return None;
        }
        let data: [&'static [u8]; TTF_COUNT] = [FONT_TERMINESS_REGULAR, FONT_TERMINESS_BOLD, FONT_JACQUARD24, FONT_JERSEY10];
        for (i, d) in data.iter().enumerate() {
            if !init_font(&mut st.font[i], d) {
                return Some(i);
            }
        }
        let gothic = st.gothic;
        map_faces(st, gothic);
        st.ready = true;
        None
    });
    match failed {
        Some(i) => {
            qlog!("font: init failed for face %d", i as i32);
            -1
        }
        None => 0,
    }
}

pub fn font_set_gothic(gothic: bool) {
    with(|st| {
        st.gothic = gothic;
        map_faces(st, gothic);
    });
}

pub fn font_gothic() -> bool {
    with(|st| st.gothic)
}

pub fn font_shutdown() {
    with(|st| {
        cache_flush(st);
        st.ready = false;
    });
}

/// Design pixels a pixel face's glyph is set below where the font has it. Jersey 10 draws the
/// inverted exclamation mark exactly as its i, a dot over a stem standing on the baseline, so a
/// Spanish exclamation would open with what reads as an i ("iHola!"). Lowered by the depth of
/// the face's descenders (g j p q y reach two design pixels below the baseline), it hangs as type
/// usually sets it: the dot at the x-height, the stem below the baseline.
fn pixel_drop(ttf: usize, cp: u32) -> i32 {
    if ttf == TTF_JERSEY && cp == 0xA1 {
        2
    } else {
        0
    }
}

/// A pixel face's glyph at k screen pixels per design pixel: hard-edged k x k blocks. Returns the
/// bitmap (empty for none) and the glyph's w, h, xoff, yoff.
fn make_pixel_glyph(st: &State, ttf: usize, k: i32, cp: u32) -> (Vec<u8>, i32, i32, i32, i32) {
    let s = 1.0 / K_UNIT[ttf] as f32;
    let (small, w, h, xo, yo) = codepoint_bitmap(&st.font[ttf], s, s, cp);
    let gw = w * k;
    let gh = h * k;
    let mut bmp = Vec::new();
    if !small.is_empty() && w > 0 && h > 0 {
        bmp = vec![0u8; gw as usize * gh as usize];
        for y in 0..gh {
            for x in 0..gw {
                bmp[(y * gw + x) as usize] = if small[((y / k) * w + x / k) as usize] >= 128 { 255 } else { 0 };
            }
        }
    }
    (bmp, gw, gh, xo * k, (yo + pixel_drop(ttf, cp)) * k)
}

/// The cache slot of the glyph, made on first use.
fn glyph_get(st: &mut State, ttf: usize, px: f32, cp: u32) -> usize {
    let size4 = if K_UNIT[ttf] != 0 { -pixel_k(st, ttf, px) } else { (px * 4.0).round_ties_even() as i32 };
    let hsh = cp.wrapping_mul(2654435761) ^ (size4 as u32).wrapping_mul(40503) ^ (ttf as u32).wrapping_mul(977);
    for probe in 0..16u32 {
        let slot = (hsh.wrapping_add(probe) % CACHE_SLOTS as u32) as usize;
        let g = &st.cache[slot];
        if g.used && g.cp == cp && g.size4 == size4 && g.face as usize == ttf {
            return slot;
        }
        if !g.used {
            if st.cache_used > (CACHE_SLOTS * 3 / 4) as i32 {
                cache_flush(st);
                return glyph_get(st, ttf, px, cp);
            }
            let (adv, _lsb) = hmetrics(&st.font[ttf], cp);
            let (bmp, w, h, xoff, yoff);
            if size4 < 0 {
                (bmp, w, h, xoff, yoff) = make_pixel_glyph(st, ttf, -size4, cp);
            } else {
                let scale = scale_for_pixel_height(&st.font[ttf], size4 as f32 / 4.0);
                (bmp, w, h, xoff, yoff) = codepoint_bitmap(&st.font[ttf], scale, scale, cp);
            }
            st.cache[slot] =
                Glyph { used: true, face: ttf as u8, size4, cp, w, h, xoff, yoff, advance: adv, bmp, bold: Vec::new() };
            st.cache_used += 1;
            return slot;
        }
    }
    cache_flush(st);
    glyph_get(st, ttf, px, cp)
}

fn face_of(st: &State, f: FontFace) -> FaceMap {
    // (the C clamps an out-of-range face to FONT_REGULAR; the enum cannot be out of range)
    st.map[f as usize]
}

// ---- text as the C reads it: bytes up to a NUL ----

/// The string a C function sees: the text up to a NUL byte, if it has one.
fn cstr(s: &str) -> &str {
    match s.find('\0') {
        Some(n) => &s[..n],
        None => s,
    }
}

/// Reads s[i] as C reads a terminated string: the byte past the end is the NUL.
#[inline]
fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// Decodes one UTF-8 code point at s[*i..]; advances *i. Invalid bytes become U+FFFD.
fn utf8_next(s: &[u8], i: &mut usize) -> u32 {
    let mut p = *i;
    let mut c = at(s, p) as u32;
    p += 1;
    let mut extra = 0;
    if (0xF0..0xF8).contains(&c) {
        c &= 0x07;
        extra = 3;
    } else if c >= 0xE0 {
        c &= 0x0F;
        extra = 2;
    } else if c >= 0xC0 {
        c &= 0x1F;
        extra = 1;
    } else if c >= 0x80 {
        *i = p;
        return 0xFFFD;
    }
    while extra > 0 {
        extra -= 1;
        if (at(s, p) & 0xC0) != 0x80 {
            *i = p;
            return 0xFFFD;
        }
        c = (c << 6) | (at(s, p) & 0x3F) as u32;
        p += 1;
    }
    *i = p;
    c
}

fn blit(c: &mut Canvas, bmp: &[u8], w: i32, h: i32, gx: i32, gy: i32, gray: u8) {
    let gr = rect_intersect(Rect { x: gx, y: gy, w, h }, c.clip);
    for yy in gr.y..gr.y + gr.h {
        let src = ((yy - gy) * w) as usize;
        let dst = (yy * c.stride) as usize;
        for xx in gr.x..gr.x + gr.w {
            let a = bmp[src + (xx - gx) as usize] as i32;
            if a != 0 {
                let d = &mut c.px[dst + xx as usize];
                *d = (*d as i32 + ((gray as i32 - *d as i32) * a + 127) / 255) as u8;
            }
        }
    }
}

fn blit_glyph(c: &mut Canvas, g: &Glyph, gx: i32, gy: i32, gray: u8) {
    blit(c, &g.bmp, g.w, g.h, gx, gy, gray);
}

/// Letters the overprint would turn into others: sharp s (U+00DF) closes into a B, the notch
/// between its stem and its bowls being all that tells them apart, and o slash (U+00F8, U+00D8)
/// fills its counter around the slash until it reads as a theta.
fn bold_keeps_gaps(cp: u32) -> bool {
    cp == 0xDF || cp == 0xF8 || cp == 0xD8
}

/// The bold weight of a pixel glyph above U+007F, made on first use. Bold is the glyph printed
/// again one design pixel to the right, which closes every one-pixel gap: the dots of u-umlaut
/// run together into a macron, the tilde and the circumflex fill in. So here the letter takes the
/// overprint as before, but a detached mark (a component wholly above or below the letter's
/// largest part: an accent, a cedilla that does not touch) is drawn as designed when the
/// overprint would close a one-pixel gap in it. Marks it cannot harm (an acute, the dot of an
/// inverted exclamation mark) keep the overprint. In the letters of bold_keeps_gaps a one-pixel
/// gap moves one design pixel right instead of closing: the stroke on its left takes the
/// overprint, the one on its right keeps its width. ASCII never comes here: it draws exactly as
/// it always has. Returns whether g.bold holds the bold weight (false: plain overprint).
fn pixel_bold(g: &mut Glyph) -> bool {
    if !g.bold.is_empty() || g.bmp.is_empty() || g.size4 >= 0 {
        return !g.bold.is_empty();
    }
    let k = -g.size4;
    let (w, h) = (g.w / k, g.h / k);
    let n = (w * h) as usize;
    // Per design pixel: its component (0 = paper), then a flood-fill stack; per component: its
    // first and last row, its size and its zone (0 the letter, 1 a mark above, 2 a mark below).
    let mut mem = vec![0i32; n * 6];
    let mut out = vec![0u8; (w as usize + 1) * h as usize];
    let (lab, rest) = mem.split_at_mut(n);
    let (stack, rest) = rest.split_at_mut(n);
    let (top, rest) = rest.split_at_mut(n);
    let (bot, rest) = rest.split_at_mut(n);
    let (size, zone) = rest.split_at_mut(n);
    let gw = g.w;
    let bmp = &g.bmp;
    let ink = |x: i32, y: i32| bmp[(y * k * gw + x * k) as usize] != 0;
    let mut nc = 0usize;
    let mut big = 0usize;
    for i in 0..n {
        if lab[i] != 0 || !ink(i as i32 % w, i as i32 / w) {
            continue;
        }
        let id = nc;
        nc += 1;
        let mut sp = 0usize;
        top[id] = h;
        bot[id] = -1;
        lab[i] = id as i32 + 1;
        stack[sp] = i as i32;
        sp += 1;
        while sp != 0 {
            sp -= 1;
            let p = stack[sp];
            let (px, py) = (p % w, p / w);
            size[id] += 1;
            if py < top[id] {
                top[id] = py;
            }
            if py > bot[id] {
                bot[id] = py;
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (qx, qy) = (px + dx, py + dy);
                    if qx < 0 || qy < 0 || qx >= w || qy >= h {
                        continue;
                    }
                    let q = (qy * w + qx) as usize;
                    if lab[q] == 0 && ink(qx, qy) {
                        lab[q] = id as i32 + 1;
                        stack[sp] = q as i32;
                        sp += 1;
                    }
                }
            }
        }
        if size[id] > size[big] {
            big = id;
        }
    }
    for i in 0..nc {
        zone[i] = if bot[i] < top[big] {
            1
        } else if top[i] > bot[big] {
            2
        } else {
            0
        };
    }
    // Marks (above, below) keep their designed shape if the overprint would fill a gap one
    // design pixel wide in them: paper between two inked pixels of a row.
    let mut plain = [false; 3];
    for y in 0..h {
        for x in 0..(w - 2).max(0) {
            let l = lab[(y * w + x) as usize];
            if l != 0 && zone[l as usize - 1] != 0 && lab[(y * w + x + 1) as usize] == 0 && lab[(y * w + x + 2) as usize] != 0 {
                plain[zone[l as usize - 1] as usize] = true;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let l = lab[(y * w + x) as usize];
            if l == 0 {
                continue;
            }
            out[(y * (w + 1) + x) as usize] = 1;
            if !plain[zone[l as usize - 1] as usize] {
                out[(y * (w + 1) + x + 1) as usize] = 1;
            }
        }
    }
    if bold_keeps_gaps(g.cp) {
        for y in 0..h {
            for x in 1..(w - 1).max(1) {
                if lab[(y * w + x) as usize] == 0 && lab[(y * w + x - 1) as usize] != 0 && lab[(y * w + x + 1) as usize] != 0 {
                    out[(y * (w + 1) + x + 1) as usize] = 0;
                }
            }
        }
    }
    let bw = (w + 1) * k;
    let mut bold = vec![0u8; bw as usize * g.h as usize];
    for y in 0..g.h {
        for x in 0..bw {
            bold[(y * bw + x) as usize] = if out[((y / k) * (w + 1) + x / k) as usize] != 0 { 255 } else { 0 };
        }
    }
    g.bold = bold;
    true
}

/// Horizontal font units -> screen pixels for a face at a size (pixel faces: whole design
/// pixels, so every glyph starts on the grid).
fn units_to_px(st: &State, ttf: usize, gpx: f32, units: i32) -> f32 {
    if K_UNIT[ttf] != 0 {
        return (((units as f32) / (K_UNIT[ttf] as f32)).round_ties_even() as i64 * pixel_k(st, ttf, gpx) as i64) as f32;
    }
    scale_for_pixel_height(&st.font[ttf], gpx) * units as f32
}

/// Lays out (and with a canvas, draws) the text from s[from] up to s[end] (None: its terminator).
/// Measuring (no canvas) reads the advances straight from the font: no glyph is rendered or cached
/// for it.
#[allow(clippy::too_many_arguments)]
fn layout(
    st: &mut State,
    mut c: Option<&mut Canvas>,
    f: FontFace,
    px: f32,
    x: i32,
    baseline: i32,
    s: &[u8],
    from: usize,
    end: Option<usize>,
    gray: u8,
) -> i32 {
    if !st.ready {
        return 0;
    }
    let m = face_of(st, f);
    let fpx = px * m.scale;
    // Overprinting a second copy to the right reads as bold on e-ink: by one design pixel for a
    // pixel face, by a size-relative amount for an outline face.
    let bold = if !m.embolden {
        0
    } else if K_UNIT[m.ttf] != 0 {
        pixel_k(st, m.ttf, fpx)
    } else {
        (1.0f32).max((fpx / 34.0).round_ties_even() as i64 as f32) as i32
    };
    let mut pen = x as f32;
    let mut prev = 0u32;
    let mut prev_ttf: i32 = -1;
    let mut i = from;
    while at(s, i) != 0 && end.is_none_or(|e| i < e) {
        let cp = utf8_next(s, &mut i);
        // Glyphs a theme face lacks (symbols, arrows) come from Terminess at the asked size.
        let mut ttf = m.ttf;
        let mut gpx = fpx;
        if ttf != TTF_TERM && ttf != TTF_TERM_BOLD && cp > 0x7F && find_glyph_index(&st.font[ttf], cp) == 0 {
            ttf = if m.embolden { TTF_TERM_BOLD } else { TTF_TERM };
            gpx = px;
        }
        if prev != 0 && prev_ttf == ttf as i32 {
            pen += units_to_px(st, ttf, gpx, kern_advance(&st.font[ttf], prev, cp));
        }
        let advance;
        if let Some(cv) = c.as_deref_mut() {
            let gi = glyph_get(st, ttf, gpx, cp);
            let g = &mut st.cache[gi];
            advance = g.advance;
            if !g.bmp.is_empty() {
                let gx = pen.round_ties_even() as i32 + g.xoff;
                let gy = baseline + g.yoff;
                // Accented letters in a pixel face's bold spare their marks (pixel_bold); that
                // bitmap already holds the overprint of one design pixel (bold == k px).
                let bb = bold != 0 && cp > 0x7F && K_UNIT[ttf] != 0 && bold == -g.size4 && pixel_bold(g);
                if bb {
                    blit(cv, &g.bold, g.w + bold, g.h, gx, gy, gray);
                } else {
                    blit_glyph(cv, g, gx, gy, gray);
                    for b in 1..=bold {
                        blit_glyph(cv, g, gx + b, gy, gray);
                    }
                }
            }
        } else {
            advance = hmetrics(&st.font[ttf], cp).0;
        }
        pen += units_to_px(st, ttf, gpx, advance) + bold as f32;
        prev = cp;
        prev_ttf = ttf as i32;
    }
    pen.round_ties_even() as i32 - x
}

/// The width of s[from..end] (end: None, its terminator) at px, drawing nothing.
fn measure_n(f: FontFace, px: f32, s: &[u8], from: usize, end: Option<usize>) -> i32 {
    with(|st| layout(st, None, f, px, 0, 0, s, from, end, 0))
}

fn measure(f: FontFace, px: f32, s: &[u8]) -> i32 {
    measure_n(f, px, s, 0, None)
}

#[allow(clippy::too_many_arguments)]
fn draw(c: &mut Canvas, f: FontFace, px: f32, x: i32, baseline_y: i32, s: &[u8], from: usize, end: Option<usize>, gray: u8) -> i32 {
    with(|st| layout(st, Some(c), f, px, x, baseline_y, s, from, end, gray))
}

/// Draws UTF-8 text with its baseline at y. Returns advance width in px.
pub fn font_draw(c: &mut Canvas, f: FontFace, px: f32, x: i32, baseline_y: i32, utf8: &str, gray: u8) -> i32 {
    draw(c, f, px, x, baseline_y, cstr(utf8).as_bytes(), 0, None, gray)
}

pub fn font_measure(f: FontFace, px: f32, utf8: &str) -> i32 {
    measure(f, px, cstr(utf8).as_bytes())
}

/// Whether the face (as the current theme maps it) has its own glyph for cp, not Terminess's.
pub fn font_has_glyph(f: FontFace, cp: u32) -> bool {
    with(|st| st.ready && find_glyph_index(&st.font[face_of(st, f).ttf], cp) != 0)
}

/// utf8 without the characters no face draws (every face takes what it lacks from Terminess, so one
/// Terminess lacks too would show as a box: Chinese, Japanese, Korean, most Vietnamese letters), and
/// without the spaces that leaves doubled, first or last. Returns how many characters it left out:
/// 0, and out is utf8 as it is (cut to cap between characters). Before font_init, none.
pub fn font_drawable_copy(utf8: &str, out: &mut String, cap: usize) -> i32 {
    if cap == 0 {
        return 0;
    }
    let s = cstr(utf8).as_bytes();
    let mut buf: Vec<u8> = Vec::new();
    let mut left_out = 0;
    let mut i = 0;
    while at(s, i) != 0 {
        let c = i;
        let cp = utf8_next(s, &mut i);
        let n = i - c;
        if cp > 0x7F && with(|st| st.ready && find_glyph_index(&st.font[TTF_TERM], cp) == 0) {
            left_out += 1;
            continue;
        }
        if left_out != 0 && s[c] == b' ' && (buf.is_empty() || buf[buf.len() - 1] == b' ') {
            continue;
        }
        if buf.len() + n >= cap {
            break; // the next character would not fit whole
        }
        buf.extend_from_slice(&s[c..c + n]);
    }
    while left_out != 0 && !buf.is_empty() && buf[buf.len() - 1] == b' ' {
        buf.pop();
    }
    // (every piece is a whole character of a &str: the bytes are UTF-8)
    *out = String::from_utf8(buf).unwrap_or_default();
    left_out
}

/// px above baseline
pub fn font_ascent(f: FontFace, px: f32) -> i32 {
    with(|st| {
        if !st.ready {
            return px as i32;
        }
        let m = face_of(st, f);
        let (a, _d, _l) = vmetrics(&st.font[m.ttf]);
        if K_UNIT[m.ttf] != 0 {
            return ((a as f32 / K_UNIT[m.ttf] as f32).round_ties_even() as i32) * pixel_k(st, m.ttf, px * m.scale);
        }
        (a as f32 * scale_for_pixel_height(&st.font[m.ttf], px * m.scale)).round_ties_even() as i32
    })
}

/// px below baseline (positive)
pub fn font_descent(f: FontFace, px: f32) -> i32 {
    with(|st| {
        if !st.ready {
            return 0;
        }
        let m = face_of(st, f);
        let (_a, d, _l) = vmetrics(&st.font[m.ttf]);
        if K_UNIT[m.ttf] != 0 {
            return (((-d) as f32 / K_UNIT[m.ttf] as f32).round_ties_even() as i32) * pixel_k(st, m.ttf, px * m.scale);
        }
        ((-d) as f32 * scale_for_pixel_height(&st.font[m.ttf], px * m.scale)).round_ties_even() as i32
    })
}

/// Whenever text is cut short to fit (font_draw_centered, font_draw_left, font_draw_wrapped), or
/// font_draw_wrapped breaks a word between its letters, the hook hears of it: the text as given,
/// the width it needs (for wrapped text, its last line's, or the broken word's) and the width it
/// has, in px. NULL takes it away. The headless runner's fit check uses it.
pub type FontOverflowHook = fn(utf8: &str, need: i32, have: i32);
pub fn font_set_overflow_hook(hook: Option<FontOverflowHook>) {
    with(|st| st.overflow = hook);
}

/// Tells the overflow hook, if there is one, outside the state's borrow: the hook may measure.
fn overflow(utf8: &str, need: i32, have: i32) {
    if let Some(hook) = with(|st| st.overflow) {
        hook(utf8, need, have);
    }
}

/// Text s (bytes up to a NUL), truncated with "..." so it measures <= maxw: None when it fits as
/// it is, else the shortened copy (the C's buf; empty where nothing fits). With report (the
/// original text), tells the overflow hook.
fn fit(f: FontFace, px: f32, s: &[u8], maxw: i32, report: Option<&str>) -> Option<Vec<u8>> {
    let w = measure(f, px, s);
    if w <= maxw {
        return None;
    }
    if let Some(text) = report {
        overflow(text, w, maxw);
    }
    // strlcpy_q(buf, s, cap - 4) with the C's 256-byte buffer: the first 251 bytes, cut wherever.
    let orig: Vec<u8> = s.iter().copied().take_while(|&b| b != 0).take(256 - 4 - 1).collect();
    let mut n = orig.len();
    while n > 0 {
        // Step back over a whole UTF-8 sequence.
        loop {
            n -= 1;
            if !(n > 0 && (orig[n] & 0xC0) == 0x80) {
                break;
            }
        }
        let mut buf = orig[..n].to_vec();
        buf.extend_from_slice(b"...");
        if measure(f, px, &buf) <= maxw {
            return Some(buf);
        }
    }
    Some(Vec::new())
}

/// Centered in r both ways; text longer than r is truncated with an ellipsis.
pub fn font_draw_centered(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    let t = cstr(utf8);
    let fitted = fit(f, px, t.as_bytes(), r.w, Some(t));
    let s: &[u8] = fitted.as_deref().unwrap_or(t.as_bytes());
    let w = measure(f, px, s);
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let base = r.y + (r.h - (asc + desc)) / 2 + asc;
    draw(c, f, px, r.x + (r.w - w) / 2, base, s, 0, None, gray);
}

fn draw_left(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8, report: bool) {
    let t = cstr(utf8);
    let fitted = fit(f, px, t.as_bytes(), r.w, if report { Some(t) } else { None });
    let s: &[u8] = fitted.as_deref().unwrap_or(t.as_bytes());
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let base = r.y + (r.h - (asc + desc)) / 2 + asc;
    draw(c, f, px, r.x, base, s, 0, None, gray);
}

/// Left-aligned, vertically centered in r, truncated with ellipsis.
pub fn font_draw_left(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    draw_left(c, f, px, r, utf8, gray, true);
}

/// The same for a name someone gave (a notebook's, a folder's), which "..." shortens by design:
/// the overflow hook (below) does not hear of it.
pub fn font_draw_left_name(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    draw_left(c, f, px, r, utf8, gray, false);
}

// ---- several lines ----

const WRAP_MAX: usize = 16;

/// A line: the bytes [s, e) of the text.
#[derive(Clone, Copy, Default)]
struct Span {
    s: usize,
    e: usize,
}

fn skip_spaces(t: &[u8], mut s: usize) -> usize {
    while at(t, s) == b' ' {
        s += 1;
    }
    s
}

fn next_char(t: &[u8], mut s: usize) -> usize {
    if at(t, s) != 0 {
        s += 1;
    }
    while (at(t, s) & 0xC0) == 0x80 {
        s += 1;
    }
    s
}

/// As many whole words on each line as fit w; a word wider than w on its own is broken between its
/// characters (at least one to a line), and *broken gets the widest such word's width (else 0).
/// Keeps up to max lines; returns how many the text needs.
fn wrap_greedy(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span], max: usize, broken: &mut i32) -> usize {
    let mut n = 0;
    *broken = 0;
    let mut s = skip_spaces(t, 0);
    while at(t, s) != 0 {
        let mut fits: Option<usize> = None; // the end of the longest run of words that fits
        let mut e = s;
        while at(t, skip_spaces(t, e)) != 0 {
            let mut we = skip_spaces(t, e);
            while at(t, we) != 0 && at(t, we) != b' ' {
                we += 1;
            }
            if measure_n(f, px, t, s, Some(we)) > w {
                break;
            }
            fits = Some(we);
            e = we;
        }
        let fits = match fits {
            Some(x) => x,
            None => {
                let mut we = s;
                while at(t, we) != 0 && at(t, we) != b' ' {
                    we += 1;
                }
                let ww = measure_n(f, px, t, s, Some(we));
                if ww > *broken {
                    *broken = ww;
                }
                let mut fits = next_char(t, s);
                while at(t, fits) != 0 && at(t, fits) != b' ' && measure_n(f, px, t, s, Some(next_char(t, fits))) <= w {
                    fits = next_char(t, fits);
                }
                fits
            }
        };
        if n < max {
            out[n] = Span { s, e: fits };
        }
        n += 1;
        s = skip_spaces(t, fits);
    }
    n
}

/// The two-line break that keeps the longer line shortest, or one right after punctuation, where a
/// clause ends ("Narrow," over "no margin"), whose longer line is at most half again as long: a
/// first clause much shorter than the rest stays on the line with it. False if no break makes two
/// lines that fit.
fn wrap_two(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span]) -> bool {
    let s = skip_spaces(t, 0);
    let mut end = t.len(); // s + strlen(s)
    while end > s && t[end - 1] == b' ' {
        end -= 1;
    }
    let mut best = 0.0f32;
    let mut found = false;
    let mut b = s;
    while b < end {
        if at(t, b) != b' ' || t[b - 1] == b' ' {
            b += 1;
            continue;
        }
        let s2 = skip_spaces(t, b);
        let w1 = measure_n(f, px, t, s, Some(b));
        let w2 = measure_n(f, px, t, s2, Some(end));
        if w1 > w || w2 > w {
            b += 1;
            continue;
        }
        let score = (if w1 > w2 { w1 } else { w2 }) as f32 * (if b",.;:!?)".contains(&t[b - 1]) { 2.0f32 / 3.0 } else { 1.0 });
        if !found || score < best {
            best = score;
            out[0] = Span { s, e: b };
            out[1] = Span { s: s2, e: end };
            found = true;
        }
        b += 1;
    }
    found
}

/// The lines font_draw_wrapped breaks text into for width w: how many it needs, up to max kept;
/// *broken as for wrap_greedy (two lines broken at a space never break a word: each fits w).
fn wrap(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span], max: usize, broken: &mut i32) -> usize {
    let n = wrap_greedy(f, px, t, w, out, max, broken);
    if n == 2 && max >= 2 {
        wrap_two(f, px, t, w, out);
    }
    n
}

/// How many lines font_draw_wrapped needs for the text in width w (it draws at most its
/// max_lines), and the width of the widest of them in *widest (may be NULL).
pub fn font_wrap_lines(f: FontFace, px: f32, utf8: &str, w: i32, widest: Option<&mut i32>) -> i32 {
    // (the C writes through *widest only if it is not NULL; a local takes the writes otherwise)
    let mut unused = 0;
    let widest = widest.unwrap_or(&mut unused);
    *widest = 0;
    if !with(|st| st.ready) {
        return 0;
    }
    let t = cstr(utf8).as_bytes();
    let one = measure(f, px, t);
    if one <= w {
        *widest = one;
        return if at(t, skip_spaces(t, 0)) != 0 { 1 } else { 0 };
    }
    let mut ln = [Span::default(); WRAP_MAX];
    let mut broken = 0;
    let n = wrap(f, px, t, w, &mut ln, WRAP_MAX, &mut broken);
    for l in ln.iter().take(n.min(WRAP_MAX)) {
        let lw = measure_n(f, px, t, l.s, Some(l.e));
        if lw > *widest {
            *widest = lw;
        }
    }
    n as i32
}

/// Up to max_lines lines broken at spaces (a word wider than r on its own is broken between its
/// characters), each centered in r or left-aligned, in slots line_h px apart (0: the face's own
/// height) with the block centered vertically in r. Text that fits on one line is drawn exactly as
/// font_draw_centered or font_draw_left draws it. Two lines are balanced: the break that keeps the
/// longer line shortest, unless one right after punctuation makes it at most half again as long
/// ("Wide, no margin" breaks after its comma). Only text that still does not fit ends in "...".
/// Returns the lines drawn.
pub fn font_draw_wrapped(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, max_lines: i32, line_h: i32, center: bool, gray: u8) -> i32 {
    if !with(|st| st.ready) {
        return 0;
    }
    let text = cstr(utf8);
    let t = text.as_bytes();
    if max_lines <= 1 || measure(f, px, t) <= r.w {
        if center {
            font_draw_centered(c, f, px, r, utf8, gray);
        } else {
            font_draw_left(c, f, px, r, utf8, gray);
        }
        return 1;
    }
    let max_lines = if max_lines > WRAP_MAX as i32 { WRAP_MAX as i32 } else { max_lines };
    let mut ln = [Span::default(); WRAP_MAX];
    let mut broken = 0;
    let n = wrap(f, px, t, r.w, &mut ln, WRAP_MAX, &mut broken) as i32;
    let shown = if n < max_lines { n } else { max_lines };
    // A word wider than the line, split between two of its letters with no hyphen: the hook hears
    // of it as of a cut (a translation should take a shorter word, or two).
    if broken != 0 {
        overflow(text, broken, r.w);
    }
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let lh = if line_h > 0 { line_h } else { asc + desc };
    let top = r.y + (r.h - shown * lh) / 2;
    for i in 0..shown {
        let base = top + i * lh + (lh - (asc + desc)) / 2 + asc;
        let (s, mut e) = (ln[i as usize].s, Some(ln[i as usize].e));
        let mut fitted: Option<Vec<u8>> = None;
        if n > max_lines && i == shown - 1 {
            // The last line holds the rest of the text, cut short: the hook hears of the whole.
            overflow(text, measure_n(f, px, t, s, None), r.w);
            fitted = fit(f, px, &t[s..], r.w, None);
            e = None;
        }
        // (the line's text: the original from s, or the shortened copy)
        let (line, from): (&[u8], usize) = match (&fitted, e) {
            (Some(buf), _) => (buf, 0),
            (None, None) => (&t[s..], 0),
            (None, Some(_)) => (t, s),
        };
        let w = measure_n(f, px, line, from, e);
        draw(c, f, px, if center { r.x + (r.w - w) / 2 } else { r.x }, base, line, from, e, gray);
    }
    shown
}

/// The largest size from px down to min_px at which the text fits maxw on one line, among the
/// sizes the face really has: a pixel face changes only at whole design pixels (Jersey 10 looks the
/// same from 27 to 43 px), an outline face in quarter pixels. px if it fits as it is; 0 if even
/// the smallest does not.
pub fn font_fit_px(f: FontFace, px: f32, min_px: f32, utf8: &str, maxw: i32) -> f32 {
    if font_measure(f, px, utf8) <= maxw {
        return px;
    }
    if !with(|st| st.ready) || min_px >= px {
        return 0.0;
    }
    let m = with(|st| face_of(st, f));
    if K_UNIT[m.ttf] != 0 {
        // Each smaller whole step, at the size it is drawn for, or at min_px inside it.
        let (a, d, _l) = with(|st| vmetrics(&st.font[m.ttf]));
        let line = (a - d) as f32 / K_UNIT[m.ttf] as f32;
        let mut k = with(|st| pixel_k(st, m.ttf, px * m.scale)) - 1;
        while k >= 1 {
            let size = (k as f32 * line / m.scale).max(min_px);
            if with(|st| pixel_k(st, m.ttf, size * m.scale)) != k {
                break; // min_px lies above this step
            }
            if font_measure(f, size, utf8) <= maxw {
                return size;
            }
            k -= 1;
        }
        return 0.0;
    }
    // An outline face: the largest quarter pixel that fits.
    let mut lo = (min_px * 4.0).ceil() / 4.0;
    let mut hi = px;
    if lo >= hi || font_measure(f, lo, utf8) > maxw {
        return 0.0;
    }
    loop {
        let mid = ((lo + hi) * 2.0).floor() / 4.0;
        if mid <= lo || mid >= hi {
            break;
        }
        if font_measure(f, mid, utf8) <= maxw {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Draws one glyph (e.g. a Nerd Font icon code point) scaled so its ink box fits r, centered.
pub fn font_draw_icon(c: &mut Canvas, f: FontFace, cp: u32, r: Rect, gray: u8) {
    if !with(|st| st.ready) || rect_is_empty(r) {
        return;
    }
    // Icons are Terminess Nerd Font glyphs in every theme.
    let ttf = if f == FontFace::Bold || f == FontFace::FixedBold { TTF_TERM_BOLD } else { TTF_TERM };
    with(|st| {
        let fi = &st.font[ttf];
        let Some((x0, y0, x1, y1)) = codepoint_box(fi, cp) else { return };
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        // Pick the pixel height whose scale fits the glyph's ink box into r.
        let unit = scale_for_pixel_height(fi, 100.0) / 100.0; // scale per px of height
        let sx = r.w as f32 / ((x1 - x0) as f32 * unit);
        let sy = r.h as f32 / ((y1 - y0) as f32 * unit);
        let px = (sx.min(sy) * 4.0).floor() / 4.0;
        let gi = glyph_get(st, ttf, px, cp);
        let g = &st.cache[gi];
        if g.bmp.is_empty() {
            return;
        }
        blit_glyph(c, g, r.x + (r.w - g.w) / 2, r.y + (r.h - g.h) / 2, gray);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn up() {
        assert_eq!(font_init(), 0);
        ui_set_theme(UiTheme::Classic);
    }

    thread_local! {
        static SEEN: RefCell<Vec<(String, i32, i32)>> = const { RefCell::new(Vec::new()) };
    }
    fn hear(text: &str, need: i32, have: i32) {
        SEEN.with(|s| s.borrow_mut().push((text.to_string(), need, have)));
    }

    #[test]
    fn broken_utf8_decodes_to_the_replacement_character() {
        let mut i = 0;
        assert_eq!(utf8_next(b"A", &mut i), 0x41);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(utf8_next(b"\xc3\xb1", &mut i), 0xF1);
        assert_eq!(i, 2);
        // a lead byte with nothing after it, a stray continuation byte, a cut sequence
        let mut i = 0;
        assert_eq!(utf8_next(b"\xc3", &mut i), 0xFFFD);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(utf8_next(b"\x80x", &mut i), 0xFFFD);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(utf8_next(b"\xe2\x82", &mut i), 0xFFFD);
        assert_eq!(i, 2);
        let mut i = 0;
        assert_eq!(utf8_next(b"\xf0\x9f\x98\x80", &mut i), 0x1F600);
        assert_eq!(i, 4);
    }

    #[test]
    fn nothing_is_measured_before_font_init() {
        font_shutdown();
        assert_eq!(font_measure(FontFace::Regular, 44.0, "abc"), 0);
        assert_eq!(font_ascent(FontFace::Regular, 44.4), 44);
        assert_eq!(font_descent(FontFace::Regular, 44.0), 0);
        assert!(!font_has_glyph(FontFace::Regular, 0x41));
        assert_eq!(font_wrap_lines(FontFace::Regular, 44.0, "abc", 1, None), 0);
        let mut out = String::new();
        assert_eq!(font_drawable_copy("a\u{65e5}b", &mut out, 20), 0);
        assert_eq!(out, "a\u{65e5}b");
    }

    #[test]
    fn drawable_copy_leaves_out_what_no_face_draws() {
        up();
        let mut out = String::new();
        assert_eq!(font_drawable_copy("Hello", &mut out, 20), 0);
        assert_eq!(out, "Hello");
        // Japanese is in no face; the spaces it leaves doubled, first or last go with it
        assert_eq!(font_drawable_copy("\u{65e5}\u{672c} a \u{8a9e} b \u{3042}", &mut out, 40), 4);
        assert_eq!(out, "a b");
        // cut between two characters, never inside one
        assert_eq!(font_drawable_copy("\u{f1}\u{f1}\u{f1}", &mut out, 5), 0);
        assert_eq!(out, "\u{f1}\u{f1}");
        assert_eq!(font_drawable_copy("abc", &mut out, 1), 0);
        assert_eq!(out, "");
        // (cap 0 touches nothing)
        out = "kept".to_string();
        assert_eq!(font_drawable_copy("abc", &mut out, 0), 0);
        assert_eq!(out, "kept");
    }

    #[test]
    fn text_cut_short_ends_in_dots_and_the_hook_hears_of_it() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 400, 100);
        let text = "A notebook with a name too long for the box";
        let need = font_measure(FontFace::Regular, 34.0, text);
        SEEN.with(|s| s.borrow_mut().clear());
        font_set_overflow_hook(Some(hear));
        font_draw_centered(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: 200, h: 50 }, text, 0);
        font_draw_left(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 50, w: 200, h: 50 }, text, 0);
        // a name someone gave is shortened by design: the hook does not hear of it
        font_draw_left_name(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 50, w: 200, h: 50 }, text, 0);
        font_set_overflow_hook(None);
        font_draw_centered(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: 200, h: 50 }, text, 0);
        SEEN.with(|s| {
            let s = s.borrow();
            assert_eq!(*s, vec![(text.to_string(), need, 200), (text.to_string(), need, 200)]);
        });
        let cut = fit(FontFace::Regular, 34.0, text.as_bytes(), 200, None).unwrap();
        assert!(cut.ends_with(b"..."));
        assert!(measure(FontFace::Regular, 34.0, &cut) <= 200);
        assert!(fit(FontFace::Regular, 34.0, b"short", 200, None).is_none());
        assert_eq!(fit(FontFace::Regular, 34.0, b"abc", 1, None).unwrap(), b"");
    }

    #[test]
    fn two_lines_break_after_the_comma() {
        up();
        // Terminess is monospaced: "Wide, no" is 8 cells, "no margin" 9
        let text = "Wide, no margin";
        let w = font_measure(FontFace::Regular, 34.0, "no margin");
        let mut widest = 0;
        assert_eq!(font_wrap_lines(FontFace::Regular, 34.0, text, w, Some(&mut widest)), 2);
        assert_eq!(widest, w);
        let mut ln = [Span::default(); WRAP_MAX];
        let mut broken = 0;
        assert_eq!(wrap(FontFace::Regular, 34.0, text.as_bytes(), w, &mut ln, WRAP_MAX, &mut broken), 2);
        assert_eq!((ln[0].s, ln[0].e, ln[1].s, ln[1].e), (0, 5, 6, 15));
        assert_eq!(broken, 0);
        // a word wider than the line is broken between its letters
        let n = wrap(FontFace::Regular, 34.0, b"abcdefghij k", font_measure(FontFace::Regular, 34.0, "abcd"), &mut ln, WRAP_MAX, &mut broken);
        assert!(n >= 3);
        assert_eq!(broken, font_measure(FontFace::Regular, 34.0, "abcdefghij"));
        assert_eq!((ln[0].s, ln[0].e), (0, 4));
        // text that fits is one line; nothing but spaces is none
        assert_eq!(font_wrap_lines(FontFace::Regular, 34.0, text, 10000, Some(&mut widest)), 1);
        assert_eq!(font_wrap_lines(FontFace::Regular, 34.0, "   ", 10000, None), 0);
    }

    #[test]
    fn wrapped_text_draws_the_lines_it_returns() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 300, 300);
        let text = "one two three four five six seven eight nine ten";
        let r = Rect { x: 0, y: 0, w: 200, h: 200 };
        let one = font_draw_wrapped(&mut c, FontFace::Regular, 34.0, r, text, 1, 0, false, 0);
        assert_eq!(one, 1);
        let shown = font_draw_wrapped(&mut c, FontFace::Regular, 34.0, r, text, 3, 0, true, 0);
        assert_eq!(shown, 3);
        let needed = font_wrap_lines(FontFace::Regular, 34.0, text, 200, None);
        assert!(needed > 3);
        let all = font_draw_wrapped(&mut c, FontFace::Regular, 34.0, r, text, 99, 40, false, 0);
        assert_eq!(all, needed.min(WRAP_MAX as i32));
    }

    #[test]
    fn fit_px_finds_the_largest_size_that_fits() {
        up();
        let text = "Rename the notebook";
        let full = font_measure(FontFace::Regular, 44.0, text);
        assert_eq!(font_fit_px(FontFace::Regular, 44.0, 20.0, text, full), 44.0);
        let px = font_fit_px(FontFace::Regular, 44.0, 20.0, text, full - 40);
        assert!(px < 44.0 && px >= 20.0);
        assert!(font_measure(FontFace::Regular, px, text) <= full - 40);
        // quarter pixels in an outline face
        assert_eq!(px * 4.0, (px * 4.0).floor());
        assert_eq!(font_fit_px(FontFace::Regular, 44.0, 20.0, text, 10), 0.0);
        // a pixel face changes only at whole design pixels
        ui_set_theme(UiTheme::Gothic);
        let g_full = font_measure(FontFace::Regular, 44.0, text);
        let g = font_fit_px(FontFace::Regular, 44.0, 20.0, text, g_full - 30);
        assert!(g > 0.0 && g < 44.0);
        assert!(font_measure(FontFace::Regular, g, text) <= g_full - 30);
    }

    #[test]
    fn glyph_cache_keeps_working_past_its_size() {
        up();
        // every code point of a block at several sizes: more than the cache holds, so it flushes
        let mut c = Canvas::default();
        canvas_init(&mut c, 120, 120);
        for size in [20.0f32, 21.0, 22.0, 23.0] {
            let mut cp = 0x20u32;
            while cp < 0x2200 {
                if let Some(ch) = char::from_u32(cp) {
                    font_draw(&mut c, FontFace::Regular, size, 10, 60, &ch.to_string(), 0);
                }
                cp += 1;
            }
        }
        assert!(font_measure(FontFace::Regular, 20.0, "Hello") > 0);
        font_shutdown();
        assert_eq!(font_measure(FontFace::Regular, 20.0, "Hello"), 0);
        assert_eq!(font_init(), 0);
        assert!(font_measure(FontFace::Regular, 20.0, "Hello") > 0);
    }

    #[test]
    fn icons_fit_their_box() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 100, 100);
        let r = Rect { x: 20, y: 20, w: 60, h: 60 };
        font_draw_icon(&mut c, FontFace::Regular, 0xF064F, r, 0);
        let mut inked = Rect::default();
        for y in 0..100 {
            for x in 0..100 {
                if c.px[(y * c.stride + x) as usize] < 255 {
                    inked = rect_union(inked, Rect { x, y, w: 1, h: 1 });
                }
            }
        }
        assert!(!rect_is_empty(inked));
        // (stb rounds the bitmap out to whole pixels: a pixel over at most)
        assert!(rect_contains_rect(rect_inflate(r, 1), inked));
        assert!(inked.w >= r.w - 2 && inked.h >= r.h - 2);
        // an empty box: nothing drawn
        let mut d = Canvas::default();
        canvas_init(&mut d, 100, 100);
        font_draw_icon(&mut d, FontFace::Regular, 0xF064F, Rect { x: 0, y: 0, w: 0, h: 10 }, 0);
        assert!(d.px.iter().all(|&p| p == 255));
    }
}
