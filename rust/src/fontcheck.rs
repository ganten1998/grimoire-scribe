//! Your own fonts, read safely (2.1.0-beta.2, lane L10; DESIGN.md "Your own fonts", 3.7, 4.2 and 4.3). Rust only.
//!
//! A font file someone copied into grimoire/fonts is untrusted bytes. stb_truetype, which draws the embedded
//! fonts, "does no range checking of the offsets found in the file" (its own header), so it never opens such a
//! file. Three parts here keep it so:
//!
//! - the safe face (`Outliner`, `rasterize`, `SafeFace`): skrifa, a parser without unsafe code, reads the
//!   tables; each glyph's outline comes out in font units as stb's own vertex list, under the caps (points, box,
//!   draw memory), and stb's rasterizer alone draws it: the anti-aliasing of the built-in fonts, and no byte of
//!   the file ever reaches stb's parser. The check and font.rs draw through the very same functions.
//! - the worker's side (`worker_run`, `WorkKind::Font` of sandbox.rs): the check. It takes one font's bytes on
//!   fd 5 and answers on fd 4: what the face is (metrics, weight, names), then every glyph the cmap maps,
//!   outlined at each location the UI uses and drawn at the UI's smallest and largest sizes, in slices of CPU
//!   time resumed by glyph index, then a preview of the font's name in itself. It never logs.
//! - the writer's side (`check_font`): one font through as many workers as it takes slices, every record
//!   validated, into a verdict (`Verdict`, the `.font` file's text) or a reason to mark the file.
//!
//! Commands (writer to worker, one line; `F` is followed by `len` bytes):
//!   `F <len> <start> <px> <slice ms> <scale>`   check the font from glyph index `start`; `px` is the preview's
//!                                       size; `scale` is 0 for the first slice, which settles the font's
//!                                       scale, and that scale for the slices after it
//!   `H <hook> <n>`                      a test hook (headless and test builds only: `crash`, `hang`, `trap` at
//!                                       glyph index n)
//!   `Q`                                 end
//!
//! Records (worker to writer, text lines of a fixed grammar; numbers are decimal, without sign):
//!   `B <version> <protocol>`, `S <0|1>`   the handshake, as every worker's
//!   `F face <upem> <ascent> <descent> <scale> <weight> <italic> <mapped>`   (descent as a magnitude; scale in
//!                                       thousandths)
//!   `F wght <min> <default> <max> <bold at>`   only for a font with a weight axis
//!   `F cover <6 counts>`, `F lacks <bits>`, `F family <hex>`, `F style <hex>`
//!   `K <glyph>`                         a glyph that failed: it comes from Terminess
//!   `G <next>`                          still working (the watchdog's sign of life)
//!   `N <next> <cpu ms>`                 the slice is over, more to do
//!   `P <w> <h>` then w * h bytes        the preview, top row first
//!   `D <cpu ms>`                        done
//!   `E bad <n>`                         the font is at fault (n: `Refuse`)
//!   `E mem`, `E protocol`, `E sandbox <step>`, `E sigsys <call>`, `E panic`

use crate::ffi_stb::*;
use crate::prelude::*;
use crate::sandbox::*;
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::pen::PathStyle;
use skrifa::outline::{DrawSettings, OutlineGlyphCollection, OutlineGlyphFormat, OutlinePen};
use skrifa::raw::types::Tag;
use skrifa::raw::FontRef;
use skrifa::string::StringId;
use skrifa::{GlyphId, MetadataProvider};
use std::rc::Rc;

// ---- the caps (DESIGN.md 4.3, one table) ----

/// A font file bigger than this is not taken ("Too big (over 24 MB).").
pub const FONT_FILE_MAX: usize = 24 << 20;
/// The font files the UI holds at once, all together.
pub const FONT_LOADED_MAX: usize = 32 << 20;
/// Files of the fonts folder looked at (one folder level).
pub const FONT_FILES_MAX: usize = 200;
/// A glyph with more points than this is not drawn.
pub const FONT_POINTS_MAX: usize = 4096;
/// A glyph's box is at most this many line heights each way, and as close to the origin.
pub const FONT_BOX_LINES: i32 = 4;
/// skrifa's memory for one outline (its composite stack, its deltas): a glyph that needs more is not drawn.
pub const FONT_DRAW_MEMORY: usize = 256 << 10;
/// The smallest and the largest line height the UI draws a custom font at; outside them a glyph comes from
/// Terminess. The check draws every glyph at both.
pub const FONT_PX_MIN: f32 = 8.0;
pub const FONT_PX_MAX: f32 = 160.0;
/// The most glyphs of a font that may fail and come from Terminess: 1% of the 65,535 a font can hold.
pub const FONT_SKIP_MAX: usize = 656;
/// Characters of a family or style name kept.
pub const FONT_NAME_CHARS: usize = 64;
/// The line a font's own em of kana and kanji is drawn at: DotGothic16's em over its line, so Japanese takes
/// the room it takes today whatever face draws it.
pub const FONT_CJK_EM: f32 = 0.691;
/// Terminess's capital height over its line: a custom font's letters are drawn about this big (its scale).
pub const FONT_CAP_OF_LINE: f32 = 0.592;
/// The preview's width, in text sizes.
const PREVIEW_EMS: f32 = 20.0;

// ---- outlines, under the caps ----

/// Why a glyph is not drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphErr {
    /// the font has no such outline, or skrifa could not read it
    Draw,
    /// more points than FONT_POINTS_MAX
    Points,
    /// a point outside the box
    Box,
    /// it needs more draw memory than FONT_DRAW_MEMORY
    Memory,
}

/// An outline's box in font units, over every point and control point (so no ink is outside it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnitBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// The pen skrifa draws into: stb's vertices, in whole font units. A TrueType outline at its default location
/// comes out as stb's own stbtt_GetGlyphShape makes it (the contour's start as hb-draw and stb find it, an
/// implied point between two control points rounded down, a closing line where the contour does not end at
/// its start), so stb's rasterizer draws the very bitmap it draws for an embedded font. Any other outline
/// (CFF, a variable font away from its default) is rounded to the nearest unit.
struct CapPen<'a> {
    v: &'a mut Vec<stbtt_vertex>,
    points: usize,
    limit: f32,
    floor: bool,
    err: Option<GlyphErr>,
    start: (i16, i16),
    at: (i16, i16),
    open: bool,
    b: UnitBox,
    any: bool,
}

impl CapPen<'_> {
    fn unit(&mut self, v: f32) -> i16 {
        if !v.is_finite() {
            self.err.get_or_insert(GlyphErr::Box);
            return 0;
        }
        let r = if self.floor { v.floor() } else { v.round_ties_even() };
        if r.abs() > self.limit {
            self.err.get_or_insert(GlyphErr::Box);
            return 0;
        }
        r as i16
    }
    fn grow(&mut self, x: i16, y: i16) {
        let (x, y) = (x as i32, y as i32);
        if !self.any {
            self.b = UnitBox { x0: x, y0: y, x1: x, y1: y };
            self.any = true;
            return;
        }
        self.b.x0 = self.b.x0.min(x);
        self.b.y0 = self.b.y0.min(y);
        self.b.x1 = self.b.x1.max(x);
        self.b.y1 = self.b.y1.max(y);
    }
    fn push(&mut self, kind: u8, pts: [(f32, f32); 3], n: usize) {
        if self.err.is_some() {
            return;
        }
        self.points += n;
        if self.points > FONT_POINTS_MAX {
            self.err = Some(GlyphErr::Points);
            return;
        }
        let mut u = [(0i16, 0i16); 3];
        for i in 0..n {
            u[i] = (self.unit(pts[i].0), self.unit(pts[i].1));
            self.grow(u[i].0, u[i].1);
        }
        if self.err.is_some() {
            return;
        }
        // (the end point is the last of pts; the control points before it)
        let end = u[n - 1];
        let (c0, c1) = match n {
            2 => (u[0], (0, 0)),
            3 => (u[0], u[1]),
            _ => ((0, 0), (0, 0)),
        };
        self.v.push(stbtt_vertex { x: end.0, y: end.1, cx: c0.0, cy: c0.1, cx1: c1.0, cy1: c1.1, kind, padding: 0 });
        self.at = end;
    }
    // A contour that does not end where it started gets the closing line stb's own shapes carry.
    fn close_contour(&mut self) {
        if self.open && self.at != self.start && self.err.is_none() {
            let s = self.start;
            self.push(STBTT_VLINE, [(s.0 as f32, s.1 as f32), (0.0, 0.0), (0.0, 0.0)], 1);
        }
        self.open = false;
    }
}

impl OutlinePen for CapPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close_contour();
        self.push(STBTT_VMOVE, [(x, y), (0.0, 0.0), (0.0, 0.0)], 1);
        self.start = self.at;
        self.open = true;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(STBTT_VLINE, [(x, y), (0.0, 0.0), (0.0, 0.0)], 1);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.push(STBTT_VCURVE, [(cx0, cy0), (x, y), (0.0, 0.0)], 2);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push(STBTT_VCUBIC, [(cx0, cy0), (cx1, cy1), (x, y)], 3);
    }
    fn close(&mut self) {
        self.close_contour();
    }
}

/// What turns a glyph into stb's vertices: skrifa's draw memory (one buffer, reused) and the vertex list.
pub struct Outliner {
    mem: Vec<u8>,
    pub verts: Vec<stbtt_vertex>,
}

impl Outliner {
    /// None without the memory for it.
    pub fn new() -> Option<Outliner> {
        let mut mem = Vec::new();
        mem.try_reserve_exact(FONT_DRAW_MEMORY).ok()?;
        mem.resize(FONT_DRAW_MEMORY, 0);
        let mut verts = Vec::new();
        verts.try_reserve_exact(FONT_POINTS_MAX + 64).ok()?;
        Some(Outliner { mem, verts })
    }

    /// Glyph `gid`'s outline at `coords` into `self.verts`, in font units, under the caps for a font whose
    /// line is `line` units high: its box, None for a glyph with no ink (a space), or why it is not drawn.
    pub fn outline(&mut self, glyphs: &OutlineGlyphCollection, gid: u32, coords: &[NormalizedCoord], line: i32) -> Result<Option<UnitBox>, GlyphErr> {
        self.verts.clear();
        let Some(glyph) = glyphs.get(GlyphId::new(gid)) else { return Err(GlyphErr::Draw) };
        if glyph.draw_memory_size(skrifa::outline::Hinting::None) > FONT_DRAW_MEMORY {
            return Err(GlyphErr::Memory);
        }
        let default = coords.iter().all(|c| *c == NormalizedCoord::ZERO);
        let limit = ((FONT_BOX_LINES as i64 * line.max(1) as i64).min(i16::MAX as i64)) as f32;
        let mut pen = CapPen {
            v: &mut self.verts,
            points: 0,
            limit,
            floor: default && glyph.format() == OutlineGlyphFormat::Glyf,
            err: None,
            start: (0, 0),
            at: (0, 0),
            open: false,
            b: UnitBox::default(),
            any: false,
        };
        let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::new(coords))
            .with_memory(Some(&mut self.mem[..]))
            .with_path_style(PathStyle::HarfBuzz);
        let drawn = glyph.draw(settings, &mut pen);
        pen.close_contour();
        let (err, any, b) = (pen.err, pen.any, pen.b);
        if let Some(e) = err {
            return Err(e);
        }
        if let Err(e) = drawn {
            return Err(if matches!(e, skrifa::outline::DrawError::InsufficientMemory) { GlyphErr::Memory } else { GlyphErr::Draw });
        }
        if !any {
            return Ok(None);
        }
        let span = FONT_BOX_LINES as i64 * line.max(1) as i64;
        if (b.x1 - b.x0) as i64 > span || (b.y1 - b.y0) as i64 > span {
            return Err(GlyphErr::Box);
        }
        Ok(Some(b))
    }
}

/// A glyph's 8-bit coverage, top row first, and where its corner goes from the pen on the baseline.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bitmap {
    pub px: Vec<u8>,
    pub w: i32,
    pub h: i32,
    pub xoff: i32,
    pub yoff: i32,
}

/// The vertices (font units, box `b`) drawn at `scale` px a unit by stb's own rasterizer, as
/// stbtt_GetGlyphBitmap draws an embedded font's glyph: the same flatness, the box rounded out to whole
/// pixels. None for a scale or a size no font of ours has (the caps keep every real call far inside them),
/// or without the memory.
pub fn rasterize(verts: &mut [stbtt_vertex], b: UnitBox, scale: f32) -> Option<Bitmap> {
    if !(scale.is_finite() && scale > 0.0) || verts.is_empty() || verts.len() > FONT_POINTS_MAX + 64 {
        return None;
    }
    let ix0 = (b.x0 as f32 * scale + 0.0).floor();
    let iy0 = (-(b.y1 as f32) * scale + 0.0).floor();
    let ix1 = (b.x1 as f32 * scale + 0.0).ceil();
    let iy1 = (-(b.y0 as f32) * scale + 0.0).ceil();
    let side = 8.0 * FONT_PX_MAX;
    if !(ix0.abs() <= side && iy0.abs() <= side && ix1.abs() <= side && iy1.abs() <= side) {
        return None;
    }
    let (ix0, iy0, ix1, iy1) = (ix0 as i32, iy0 as i32, ix1 as i32, iy1 as i32);
    let (w, h) = (ix1 - ix0, iy1 - iy0);
    let mut out = Bitmap { px: Vec::new(), w, h, xoff: ix0, yoff: iy0 };
    if w <= 0 || h <= 0 {
        out.w = w.max(0);
        out.h = h.max(0);
        return Some(out);
    }
    let n = w as usize * h as usize;
    out.px.try_reserve_exact(n).ok()?;
    out.px.resize(n, 0);
    let mut bm = stbtt__bitmap { w, h, stride: w, pixels: out.px.as_mut_ptr() };
    // SAFETY: bm describes out.px, w * h bytes we own, which stb fills row by row and never grows; verts is our
    // own slice of plain vertices (stb reads it, and keeps no pointer past the call); no font data is involved.
    unsafe {
        stbtt_Rasterize(&mut bm, 0.35, verts.as_mut_ptr(), verts.len() as libc::c_int, scale, scale, 0.0, 0.0, ix0, iy0, 1, std::ptr::null_mut());
    }
    Some(out)
}

// ---- what the check learns of a face ----

/// What the check learned of a font that passed, and what font.rs draws it by: the `.font` verdict's lines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FaceFacts {
    /// font units per em
    pub upem: i32,
    /// the line box, font units: above the baseline, and below it (0 or negative). Sanitized by the check.
    pub ascent: i32,
    pub descent: i32,
    /// how much bigger than asked its text is drawn, in thousandths (850 to 1250)
    pub scale_pm: i32,
    /// its weight class (100 to 1000), and whether it is italic or oblique
    pub weight: i32,
    pub italic: bool,
    /// its weight axis, if it has one: least, default, most
    pub wght: Option<(i32, i32, i32)>,
    /// the weight its bold is drawn at through the axis (0: none)
    pub bold_at: i32,
    /// glyphs the cmap maps, and how many of them failed
    pub mapped: i32,
    /// the glyphs that failed, in order: they come from Terminess
    pub skip: Vec<u16>,
    pub family: String,
    pub style: String,
    /// bit i: the strings of language i (i18n_language_code) need letters this font lacks
    pub lacks: u32,
    /// how many characters it has of Latin-1, Latin Extended-A, Greek, Cyrillic, kana and kanji
    pub cover: [i32; 6],
}

impl FaceFacts {
    /// The line's height in font units (ascent to descent).
    pub fn line(&self) -> i32 {
        self.ascent - self.descent
    }
    /// Whether the numbers are ones a font can have: the UI checks them again before it draws by them.
    pub fn sane(&self) -> bool {
        (16..=16384).contains(&self.upem)
            && self.ascent > 0
            && self.descent <= 0
            && (self.upem / 8..=self.upem * 8).contains(&self.line())
            && (850..=1250).contains(&self.scale_pm)
            && self.skip.len() <= FONT_SKIP_MAX
            && self.skip.windows(2).all(|w| w[0] < w[1])
            && (0..=1000).contains(&self.bold_at)
    }
}

/// Why a font is refused, as the worker's `E bad <n>` numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refuse {
    /// not a font skrifa reads, no outlines, letters that draw nothing, too many broken glyphs
    Unreadable = 0,
    /// no Unicode cmap, a symbol font, a printable ASCII character missing
    NoLatin = 1,
    /// a WOFF or WOFF2 file
    Format = 2,
}

/// The normalized location of the axes for weight `wght` (every other axis at its default).
fn location_for(font: &FontRef, wght: f32) -> Vec<NormalizedCoord> {
    font.axes().location([("wght", wght)]).coords().to_vec()
}

/// A parsed face with what the walk needs.
struct Opened<'a> {
    font: FontRef<'a>,
    glyphs: OutlineGlyphCollection<'a>,
    /// the locations the UI uses: the default, then the bold's (a weight axis only)
    locations: Vec<Vec<NormalizedCoord>>,
    facts: FaceFacts,
    /// one bit per glyph id: the cmap maps to it
    mapped: Vec<u64>,
}

fn bit(set: &[u64], i: u32) -> bool {
    set.get((i / 64) as usize).is_some_and(|w| w >> (i % 64) & 1 == 1)
}

/// The ink a bitmap has inside the line box, in 255ths of a pixel of coverage.
fn ink_in_line(bm: &Bitmap, asc_px: i32, desc_px: i32) -> u64 {
    let mut sum = 0u64;
    for r in 0..bm.h {
        let y = bm.yoff + r;
        if y < -asc_px || y >= desc_px {
            continue;
        }
        let row = &bm.px[(r * bm.w) as usize..((r + 1) * bm.w) as usize];
        sum += row.iter().map(|&p| p as u64).sum::<u64>();
    }
    sum
}

fn name_of(font: &FontRef, ids: [StringId; 2]) -> String {
    for id in ids {
        if let Some(s) = font.localized_strings(id).english_or_first() {
            let t: String = s.chars().filter(|c| !c.is_control()).take(FONT_NAME_CHARS).collect();
            let t = t.trim().to_string();
            if !t.is_empty() {
                return t;
            }
        }
    }
    String::new()
}

/// The first part of the check: the face read, its letters looked at, its line and scale settled. Every
/// slice runs it (it is quick), and the first one reports it.
fn open_face<'a>(data: &'a [u8], out: &mut Outliner, scale_pm: i32) -> Result<Opened<'a>, Refuse> {
    if data.len() >= 4 && (&data[..4] == b"wOFF" || &data[..4] == b"wOF2") {
        return Err(Refuse::Format);
    }
    // (an sfnt, or the first face of a collection)
    let font = FontRef::from_index(data, 0).map_err(|_| Refuse::Unreadable)?;
    let cmap = font.charmap();
    if !cmap.has_map() || cmap.is_symbol() {
        return Err(Refuse::NoLatin);
    }
    let glyphs = font.outline_glyphs();
    if glyphs.format().is_none() {
        return Err(Refuse::Unreadable);
    }
    let m = font.metrics(Size::unscaled(), LocationRef::default());
    let upem = m.units_per_em as i32;
    if !(16..=16384).contains(&upem) {
        return Err(Refuse::Unreadable);
    }
    let nglyphs = (m.glyph_count as u32).max(1);
    // The letters: all of printable ASCII, each a glyph of the font.
    let mut ascii = [0u32; 0x7F];
    for cp in 0x20u32..0x7F {
        match cmap.map(cp).map(|g| g.to_u32()) {
            Some(g) if g != 0 && g < nglyphs => ascii[cp as usize] = g,
            _ => return Err(Refuse::NoLatin),
        }
    }
    // The line: the font's own, unless it is far from what its letters reach (a font whose tables say ten
    // lines of air, or none): then what they reach. Above: the capitals, the digits, the tall small letters,
    // and the accented capitals it has; below: g j p q y and the cedillas.
    let limit_line = upem * 8;
    let reach = |out: &mut Outliner, glyph: u32, need: bool, top: &mut i32, bottom: &mut i32| -> Result<(), Refuse> {
        match out.outline(&glyphs, glyph, &[], limit_line) {
            Ok(Some(b)) => {
                *top = (*top).max(b.y1);
                *bottom = (*bottom).min(b.y0);
            }
            Ok(None) => {}
            // (a letter that does not draw refuses the font; an accented one is only left out of the measure)
            Err(_) if need => return Err(Refuse::Unreadable),
            Err(_) => {}
        }
        Ok(())
    };
    let (mut top, mut bottom, mut unused) = (0, 0, 0);
    for &c in b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789" {
        reach(out, ascii[c as usize], true, &mut top, &mut unused)?;
    }
    if top <= 0 {
        return Err(Refuse::Unreadable); // capitals that reach nowhere
    }
    let caps = top;
    for &c in b"bdfhkl" {
        reach(out, ascii[c as usize], true, &mut top, &mut unused)?;
    }
    for &c in b"gjpqy" {
        reach(out, ascii[c as usize], true, &mut unused, &mut bottom)?;
    }
    let extra = |cp: u32| cmap.map(cp).map(|g| g.to_u32()).filter(|&g| g != 0 && g < nglyphs);
    for g in (0xC0u32..=0xDE).filter_map(extra) {
        reach(out, g, false, &mut top, &mut unused)?;
    }
    for g in [0xC7u32, 0xE7].into_iter().filter_map(extra) {
        reach(out, g, false, &mut unused, &mut bottom)?;
    }
    let fin = |v: f32| if v.is_finite() { v.round_ties_even().clamp(-1.0e6, 1.0e6) as i32 } else { 0 };
    let (mut ascent, mut descent) = (fin(m.ascent), fin(m.descent).min(0));
    let far = |decl: i32, real: i32| decl <= 0 || decl as i64 * 2 > real as i64 * 3 || decl as i64 * 3 < real as i64 * 2;
    if far(ascent, top) {
        ascent = top;
    }
    if bottom < 0 {
        if far(-descent, -bottom) {
            descent = bottom;
        }
    } else if -(descent as i64) > caps as i64 {
        // (letters without a tail, and a descent deeper than the capitals are tall: a quarter of them)
        descent = -(caps / 4);
    }
    let line = ascent - descent;
    if !(upem / 8..=upem * 8).contains(&line) {
        return Err(Refuse::Unreadable);
    }
    // The locations the UI uses: the default, and the bold's when the font has a weight axis that reaches a
    // bolder weight (700, or its heaviest when that is lighter, if it is at least 100 above the default).
    let mut locations: Vec<Vec<NormalizedCoord>> = vec![Vec::new()];
    let (mut wght, mut bold_at) = (None, 0);
    if let Some(ax) = font.axes().get_by_tag(Tag::new(b"wght")) {
        let (lo, def, hi) = (fin(ax.min_value()), fin(ax.default_value()), fin(ax.max_value()));
        if lo <= def && def <= hi && (1..=1000).contains(&lo) && hi <= 1000 {
            wght = Some((lo, def, hi));
            let at = hi.min(700);
            if at >= def + 100 {
                bold_at = at;
                locations.push(location_for(&font, at as f32));
            }
        }
    }
    let attrs = font.attributes();
    let weight = fin(attrs.weight.value()).clamp(1, 1000);
    let italic = !matches!(attrs.style, skrifa::attribute::Style::Normal);
    // Every letter and digit must put ink inside the line box at the smallest and the largest size: a font
    // whose letters are there and draw nothing never reaches a screen. (An eighth of a pixel of coverage at the
    // smallest size, which the thinnest stem of a real font passes, and sixteen pixels at the largest.)
    for cp in (b'A'..=b'Z').chain(b'a'..=b'z').chain(b'0'..=b'9') {
        for loc in &locations {
            let Ok(Some(b)) = out.outline(&glyphs, ascii[cp as usize], loc, line) else { return Err(Refuse::Unreadable) };
            for (px, need) in [(FONT_PX_MIN, 32u64), (FONT_PX_MAX, 16 * 255)] {
                let scale = px / line as f32;
                let (asc_px, desc_px) = ((ascent as f32 * scale).ceil() as i32, (-descent as f32 * scale).ceil() as i32);
                match rasterize(&mut out.verts, b, scale) {
                    Some(bm) if ink_in_line(&bm, asc_px, desc_px) >= need => {}
                    _ => return Err(Refuse::Unreadable),
                }
            }
        }
    }
    // The glyphs the cmap maps (a glyph nothing maps to is never drawn: Grimoire shapes nothing), and what
    // the font covers.
    let mut mapped = vec![0u64; nglyphs.div_ceil(64) as usize];
    let mut count = 0;
    let mut cover = [0i32; 6];
    // (each character through `map`, as the UI asks: on cmap groups that overlap, the group a walk of `mappings`
    // names can differ from the glyph `map` finds, and the UI would be the first to outline that glyph)
    for (cp, _) in cmap.mappings() {
        let Some(g) = cmap.map(cp) else { continue };
        let g = g.to_u32();
        if g == 0 || g >= nglyphs {
            continue;
        }
        let w = &mut mapped[(g / 64) as usize];
        if *w >> (g % 64) & 1 == 0 {
            *w |= 1 << (g % 64);
            count += 1;
        }
        let k = match cp {
            0xA0..=0xFF => 0,
            0x100..=0x17F => 1,
            0x370..=0x3FF => 2,
            0x400..=0x45F => 3,
            0x3040..=0x30FF => 4,
            0x4E00..=0x9FFF => 5,
            _ => continue,
        };
        cover[k] = cover[k].saturating_add(1);
    }
    let family = name_of(&font, [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]);
    let style = name_of(&font, [StringId::TYPOGRAPHIC_SUBFAMILY_NAME, StringId::SUBFAMILY_NAME]);
    let mut facts = FaceFacts { upem, ascent, descent, scale_pm: 1000, weight, italic, wght, bold_at, mapped: count, family, style, cover, ..Default::default() };
    // Its scale: capitals as tall as Terminess's, then smaller until its text is no wider than Terminess's.
    // (The first slice settles it; the ones after it are told.)
    if scale_pm == 0 {
        let cap = out.outline(&glyphs, ascii[b'H' as usize], &[], line).ok().flatten().map_or(caps, |b| b.y1).max(1);
        let k = (FONT_CAP_OF_LINE * line as f32 / cap as f32).clamp(0.85, 1.25);
        let (pm, lacks) = fit_strings(&font, &facts, (k * 1000.0).round() as i32);
        facts.scale_pm = pm;
        facts.lacks = lacks;
    } else {
        facts.scale_pm = scale_pm.clamp(850, 1250);
    }
    Ok(Opened { font, glyphs, locations, facts, mapped })
}

/// A custom font's text against Terminess's, over every string of every language Grimoire speaks: the scale
/// (thousandths, from `start` down to 850) at which 95 of 100 strings are no wider than Terminess draws them,
/// and which languages need letters the font lacks (Japanese's kana and kanji never count: every font takes
/// them from the Japanese face). Advances only, as font.rs measures: no glyph is drawn.
fn fit_strings(font: &FontRef, facts: &FaceFacts, start: i32) -> (i32, u32) {
    let cmap = font.charmap();
    let gm = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let line = facts.line() as f32;
    // (a size at which a pixel is a thousandth of a line: widths in whole units either way)
    let px = 1000.0;
    let _ = font_init();
    let mut own = std::collections::HashMap::<u32, Option<f32>>::new();
    let mut advance = |cp: u32| -> Option<f32> {
        *own.entry(cp).or_insert_with(|| {
            let g = cmap.map(cp).filter(|g| g.to_u32() != 0)?;
            let a = gm.advance_width(g)?;
            (a.is_finite() && a >= 0.0 && a <= FONT_BOX_LINES as f32 * line).then_some(a * px / line)
        })
    };
    // (what Terminess, or the Japanese face behind it, takes for a character, as font.rs lays it out)
    let mut theirs = std::collections::HashMap::<u32, f32>::new();
    let mut terminess = |ch: char| -> f32 {
        *theirs.entry(ch as u32).or_insert_with(|| {
            let mut buf = [0u8; 4];
            font_measure(FontFace::Fixed, px, ch.encode_utf8(&mut buf)) as f32
        })
    };
    // each string: what the font's own glyphs take at scale 1, what the borrowed ones take, Terminess's width
    // (character by character, so a string drawn all in borrowed glyphs is exactly as wide either way)
    let mut rows: Vec<(f32, f32, f32)> = Vec::new();
    let mut lacks = 0u32;
    let mut one = |s: &str, lang: usize, rows: &mut Vec<(f32, f32, f32)>| {
        if s.chars().count() < 3 {
            return;
        }
        let (mut a, mut b, mut t) = (0.0f32, 0.0f32, 0.0f32);
        for ch in s.chars() {
            let cp = ch as u32;
            if cp < 0x20 {
                continue;
            }
            let w = terminess(ch);
            t += w;
            match advance(cp).filter(|_| !crate::font::cjk(cp)) {
                Some(own) => a += own,
                None => {
                    b += w;
                    if cp > 0x20 && !crate::font::cjk(cp) {
                        lacks |= 1 << lang;
                    }
                }
            }
        }
        if t > 0.0 {
            rows.push((a, b, t));
        }
    };
    for lang in 0..i18n_language_count().min(16) {
        let Some(code) = i18n_language_code(lang) else { continue };
        if lang == 0 {
            // the English: every catalog's msgids (the same in each), from the first that reads
            for other in 1..i18n_language_count() {
                let Some(oc) = i18n_language_code(other) else { continue };
                if i18n_walk(oc, &mut |e: &I18nEntry| {
                    one(e.id, 0, &mut rows);
                    if let Some(p) = e.id_plural {
                        one(p, 0, &mut rows);
                    }
                }) >= 0
                {
                    break;
                }
            }
            continue;
        }
        i18n_walk(code, &mut |e: &I18nEntry| {
            for form in e.str.split('\0') {
                one(form, lang as usize, &mut rows);
            }
        });
    }
    let mut pm = start.clamp(850, 1250);
    if !rows.is_empty() {
        while pm > 850 {
            let k = pm as f32 / 1000.0;
            let fit = rows.iter().filter(|(a, b, t)| a * k + b <= *t).count();
            if fit * 100 >= rows.len() * 95 {
                break;
            }
            pm -= 10;
        }
    }
    (pm.max(850), lacks)
}

/// The check's reading of a font, in this very process: for the tests (the headless and host builds only; the
/// Kindle's binary has no such function). Grimoire itself never reads a file of the owner's this way: the
/// worker does (check_font). The glyph walk is left to `glyph_fails`.
#[cfg(not(feature = "kindle"))]
pub fn facts_in_process(data: &[u8]) -> Result<FaceFacts, Refuse> {
    let mut out = Outliner::new().ok_or(Refuse::Unreadable)?;
    Ok(open_face(data, &mut out, 0)?.facts)
}

/// The glyphs the cmap maps that fail the walk, in this very process: for the tests, as facts_in_process.
#[cfg(not(feature = "kindle"))]
pub fn glyphs_failing_in_process(data: &[u8]) -> Result<Vec<u32>, Refuse> {
    let mut out = Outliner::new().ok_or(Refuse::Unreadable)?;
    let o = open_face(data, &mut out, 0)?;
    let n = o.mapped.len() as u32 * 64;
    Ok((0..n).filter(|&g| bit(&o.mapped, g) && !glyph_draws(&o, &mut out, g)).collect())
}

// ---- the worker's side ----

fn say(s: &str) {
    write_all(FD_OUT, s.as_bytes());
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
            // SAFETY: _exit ends the process at once.
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

fn read_bytes_fd(fd: i32, len: usize) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    v.try_reserve_exact(len).ok()?;
    v.resize(len, 0);
    let mut got = 0;
    while got < len {
        // SAFETY: read into the part of v not yet filled.
        let n = unsafe { libc::read(fd, v[got..].as_mut_ptr() as *mut libc::c_void, len - got) };
        if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        if n <= 0 {
            return None;
        }
        got += n as usize;
    }
    Some(v)
}

/// A record's number: decimal digits, no sign, no leading zero, at most nine of them.
fn number(t: &str) -> Option<usize> {
    if t.is_empty() || t.len() > 9 || !t.bytes().all(|c| c.is_ascii_digit()) || (t.len() > 1 && t.starts_with('0')) {
        return None;
    }
    t.parse().ok()
}

/// This process's CPU time so far, in milliseconds (clock_gettime is on the worker's allow-list).
fn cpu_ms() -> u64 {
    // SAFETY: ts is written by clock_gettime before it is read.
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    ts.tv_sec as u64 * 1000 + ts.tv_nsec as u64 / 1_000_000
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "kindle", allow(dead_code))]
struct Hooks {
    crash: Option<usize>,
    hang: Option<usize>,
    trap: Option<usize>,
}

#[cfg(not(feature = "kindle"))]
fn run_hooks(h: &Hooks, n: usize) {
    if h.crash == Some(n) {
        std::process::abort();
    }
    if h.hang == Some(n) {
        // a busy loop: a sleeping call is not on the allow-list
        let mut x = 0u64;
        loop {
            x = std::hint::black_box(x.wrapping_add(1));
        }
    }
    if h.trap == Some(n) {
        // SAFETY: getpid takes no arguments; it is not on the allow-list, so under the filter it never returns.
        unsafe { libc::getpid() };
    }
}
#[cfg(feature = "kindle")]
fn run_hooks(_h: &Hooks, _n: usize) {}

fn hex(s: &str) -> String {
    if s.is_empty() {
        return "-".to_string();
    }
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

/// One glyph at every location the UI uses, outlined under the caps and drawn at the smallest and the largest
/// size: whether all of it worked.
fn glyph_draws(o: &Opened, out: &mut Outliner, gid: u32) -> bool {
    let line = o.facts.line();
    for loc in &o.locations {
        match out.outline(&o.glyphs, gid, loc, line) {
            Ok(None) => {}
            Ok(Some(b)) => {
                for px in [FONT_PX_MIN, FONT_PX_MAX] {
                    if rasterize(&mut out.verts, b, px / line as f32).is_none() {
                        return false;
                    }
                }
            }
            Err(_) => return false,
        }
    }
    // (its advance is what the UI measures by: a number a line can hold)
    let adv = o.font.glyph_metrics(Size::unscaled(), LocationRef::default()).advance_width(GlyphId::new(gid));
    matches!(adv, Some(a) if a.is_finite() && a >= 0.0 && a <= (FONT_BOX_LINES * line) as f32)
}

/// The preview: the font's family name drawn in the font itself at `px` (as text of that size is drawn, at
/// the font's scale), with its style when that is not plain Regular (the Bold and the Italic of one family
/// tell apart), black on white, cut at the last whole letter that fits. A name with no letter of the font in
/// it gives the alphabet's start instead.
fn preview(o: &Opened, out: &mut Outliner, px: f32) -> Option<(Vec<u8>, i32, i32)> {
    let f = &o.facts;
    let line = f.line() as f32;
    let gpx = (px * f.scale_pm as f32 / 1000.0).clamp(FONT_PX_MIN, FONT_PX_MAX);
    let scale = gpx / line;
    let h = (px * 1.25).ceil() as i32 + 8;
    let wmax = (px * PREVIEW_EMS) as i32;
    let asc = (f.ascent as f32 * scale).round_ties_even() as i32;
    let desc = (-f.descent as f32 * scale).round_ties_even() as i32;
    let base = (h - (asc + desc)) / 2 + asc;
    let cmap = o.font.charmap();
    let gm = o.font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let mut px_out: Vec<u8> = Vec::new();
    px_out.try_reserve_exact(wmax as usize * h as usize).ok()?;
    px_out.resize(wmax as usize * h as usize, 255);
    let mut draw = |text: &str, px_out: &mut Vec<u8>| -> i32 {
        let mut pen = 0.0f32;
        let mut right = 0;
        for ch in text.chars() {
            let Some(g) = cmap.map(ch as u32).map(|g| g.to_u32()).filter(|&g| g != 0 && bit(&o.mapped, g)) else { continue };
            let Some(adv) = gm.advance_width(GlyphId::new(g)).filter(|a| a.is_finite() && *a >= 0.0) else { continue };
            let next = pen + adv * scale;
            if next.ceil() as i32 > wmax {
                break;
            }
            if let Ok(Some(b)) = out.outline(&o.glyphs, g, &[], f.line()) {
                if let Some(bm) = rasterize(&mut out.verts, b, scale) {
                    let (gx, gy) = (pen.round_ties_even() as i32 + bm.xoff, base + bm.yoff);
                    for r in 0..bm.h {
                        let y = gy + r;
                        if y < 0 || y >= h {
                            continue;
                        }
                        for c in 0..bm.w {
                            let x = gx + c;
                            if x < 0 || x >= wmax {
                                continue;
                            }
                            let a = bm.px[(r * bm.w + c) as usize] as i32;
                            let d = &mut px_out[(y * wmax + x) as usize];
                            *d = (*d as i32 - (*d as i32 * a + 127) / 255) as u8;
                            right = right.max(x + 1);
                        }
                    }
                }
            }
            pen = next;
        }
        right
    };
    let named = if f.style.is_empty() || f.style.eq_ignore_ascii_case("regular") { f.family.clone() } else { format!("{} {}", f.family, f.style) };
    let mut right = draw(&named, &mut px_out);
    if right == 0 {
        right = draw("AaBbCc 123", &mut px_out);
    }
    // (cut to its ink, with a little air)
    let w = (right + 4).clamp(8, wmax);
    let mut cut = Vec::new();
    cut.try_reserve_exact(w as usize * h as usize).ok()?;
    for y in 0..h {
        cut.extend_from_slice(&px_out[(y * wmax) as usize..(y * wmax + w) as usize]);
    }
    Some((cut, w, h))
}

/// The check of one font from glyph index `start`: the records it sends.
fn check_one(data: &[u8], start: usize, px: f32, slice_ms: u64, scale_pm: i32, hooks: &Hooks) {
    let t0 = cpu_ms();
    let Some(mut out) = Outliner::new() else {
        say("E mem\n");
        return;
    };
    let o = match open_face(data, &mut out, if start == 0 { 0 } else { scale_pm }) {
        Ok(o) => o,
        Err(why) => {
            say(&format!("E bad {}\n", why as i32));
            return;
        }
    };
    let f = &o.facts;
    if start == 0 {
        say(&format!("F face {} {} {} {} {} {} {}\n", f.upem, f.ascent, -f.descent, f.scale_pm, f.weight, f.italic as i32, f.mapped));
        if let Some((lo, def, hi)) = f.wght {
            say(&format!("F wght {lo} {def} {hi} {}\n", f.bold_at));
        }
        say(&format!("F cover {} {} {} {} {} {}\n", f.cover[0], f.cover[1], f.cover[2], f.cover[3], f.cover[4], f.cover[5]));
        say(&format!("F lacks {}\n", f.lacks));
        say(&format!("F family {}\n", hex(&f.family)));
        say(&format!("F style {}\n", hex(&f.style)));
    }
    let nglyphs = o.mapped.len() as u32 * 64;
    let mut failed = 0usize;
    let mut beat = cpu_ms();
    let mut gid = start as u32;
    while gid < nglyphs {
        if bit(&o.mapped, gid) {
            run_hooks(hooks, gid as usize);
            if !glyph_draws(&o, &mut out, gid) {
                failed += 1;
                if failed > FONT_SKIP_MAX {
                    say("E bad 0\n");
                    return;
                }
                say(&format!("K {gid}\n"));
            }
        }
        gid += 1;
        if gid.is_multiple_of(16) {
            let now = cpu_ms();
            if now.saturating_sub(t0) >= slice_ms && gid < nglyphs {
                say(&format!("N {gid} {}\n", now.saturating_sub(t0)));
                return;
            }
            if now.saturating_sub(beat) >= 1000 {
                beat = now;
                say(&format!("G {gid}\n"));
            }
        }
    }
    match preview(&o, &mut out, px) {
        Some((pixels, w, h)) => {
            say(&format!("P {w} {h}\n"));
            write_all(FD_OUT, &pixels);
        }
        None => {
            say("E mem\n");
            return;
        }
    }
    say(&format!("D {}\n", cpu_ms().saturating_sub(t0)));
}

/// The worker's loop (after sandbox.rs's steps): one font, in as many `F` commands as the writer sends.
pub fn worker_run(_l: &Limits) -> i32 {
    #[cfg_attr(feature = "kindle", allow(unused_mut))]
    let mut hooks = Hooks::default();
    while let Some(cmd) = read_line_fd(FD_IN) {
        let t: Vec<&str> = cmd.split(' ').collect();
        match t.as_slice() {
            ["F", len, start, px, slice, scale] => {
                let (Some(len), Some(start), Some(px), Some(slice), Some(scale)) = (number(len), number(start), number(px), number(slice), number(scale)) else {
                    say("E protocol\n");
                    return 1;
                };
                let scale_ok = if start == 0 { scale == 0 } else { (850..=1250).contains(&scale) };
                if len == 0 || len > FONT_FILE_MAX || start > 65_535 || !(8..=400).contains(&px) || slice == 0 || !scale_ok {
                    say("E protocol\n");
                    return 1;
                }
                let Some(data) = read_bytes_fd(FD_IN, len) else {
                    return 1;
                };
                check_one(&data, start, px as f32, slice as u64, scale as i32, &hooks);
            }
            #[cfg(not(feature = "kindle"))]
            ["H", hook, n] => {
                let Some(n) = number(n) else {
                    say("E protocol\n");
                    return 1;
                };
                match *hook {
                    "crash" => hooks.crash = Some(n),
                    "hang" => hooks.hang = Some(n),
                    "trap" => hooks.trap = Some(n),
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

/// What the worker warms up before the sandbox closes (sandbox.rs's step 1): the embedded fonts' tables, which
/// the string fit reads through font.rs.
pub fn warm_up() {
    let _ = font_init();
}

// ---- the writer's side ----

/// What a font came to.
#[derive(Debug)]
pub enum FontOut {
    /// it passed: what the check learned, and the preview (w x h gray pixels, top row first)
    Passed { facts: FaceFacts, preview: Vec<u8>, w: i32, h: i32 },
    /// the font is at fault: the same bytes give the same answer
    Refused(Refuse),
    /// it took longer than a font may ("Took too long to read.")
    Slow,
    /// the worker died or broke the protocol on it: it is marked bad
    Dead(&'static str),
    /// the memory was not there: another time it may be; nothing is marked
    Retry(&'static str),
    /// no worker can be had (a sandbox step failed, or it did not start): not the font's fault, nothing is
    /// marked, and the session checks no more fonts
    NoWorker(&'static str),
}

/// How long a worker may go without a record before it is killed: the table's number, or (headless and test
/// builds) GRIMOIRE_FONT_WATCHDOG_MS.
fn watchdog_ms() -> u64 {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_FONT_WATCHDOG_MS").ok().and_then(|v| v.parse::<u64>().ok()).filter(|&v| v > 0) {
        return v;
    }
    s1::FONT_WATCHDOG_MS as u64
}

/// A slice's CPU time and the whole check's: the table's numbers, or (headless and test builds)
/// GRIMOIRE_FONT_SLICE_MS and GRIMOIRE_FONT_JOB_MS.
fn budget_ms() -> (u64, u64) {
    #[allow(unused_mut)]
    let (mut slice, mut job) = (s1::FONT_SLICE_CPU_SECS * 1000, s1::FONT_JOB_CPU_SECS * 1000);
    #[cfg(not(feature = "kindle"))]
    {
        let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<u64>().ok()).filter(|&v| v > 0);
        if let Some(v) = env("GRIMOIRE_FONT_SLICE_MS") {
            slice = v;
        }
        if let Some(v) = env("GRIMOIRE_FONT_JOB_MS") {
            job = v;
        }
    }
    (slice, job)
}

/// The limits of a font worker for a file of `len` bytes (4.3: its image, the file, 64 MiB).
pub fn limits(len: usize) -> Limits {
    Limits { as_bytes: s1::WORKER_IMAGE + len as u64 + s1::FONT_AS_MARGIN, cpu_secs: s1::FONT_CPU_SECS, mapped: false, file_len: 0 }
}

static SANDBOX_LINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

enum Hello {
    Ready(Worker),
    No(&'static str),
}

fn start_worker(len: usize) -> Hello {
    if session_broken() {
        return Hello::No("an earlier worker's sandbox step failed");
    }
    // a worker that is gone must never kill the writer with SIGPIPE
    // SAFETY: setting a signal's disposition to ignore.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
    let mut w = match Worker::spawn(WorkKind::Font, None, &limits(len)) {
        Ok(w) => w,
        Err(SpawnErr::Sandbox(_)) => {
            session_set_broken();
            return Hello::No("a sandbox step failed");
        }
        Err(SpawnErr::Start) => return Hello::No("the worker did not start"),
    };
    let deadline = mono_ms() + 15_000;
    let hello = (|| {
        let line = w.read_line(512, deadline).ok()?;
        if line != format!("B {}", build_id()).as_bytes() {
            return Some("the worker is another build");
        }
        let line = w.read_line(512, deadline).ok()?;
        let s = String::from_utf8_lossy(&line).into_owned();
        if s == "S 0" || s == "S 1" {
            return Some("");
        }
        if let Some(step) = s.strip_prefix("E sandbox ") {
            if step.len() < 32 && step.bytes().all(|c| c.is_ascii_lowercase()) {
                qlog!("work: a font's worker could not close its sandbox (step %s)", step);
                session_set_broken();
                return Some("a sandbox step failed");
            }
        }
        Some("no handshake")
    })();
    match hello {
        Some("") => {}
        Some(why) => {
            w.kill_and_reap();
            return Hello::No(why);
        }
        None => {
            w.kill_and_reap();
            return Hello::No("the worker did not answer");
        }
    }
    if !SANDBOX_LINE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        qlog!("%s", &sandboxed_line());
    }
    #[cfg(not(feature = "kindle"))]
    for (var, hook) in [("GRIMOIRE_FONT_TEST_CRASH", "crash"), ("GRIMOIRE_FONT_TEST_HANG", "hang"), ("GRIMOIRE_FONT_TEST_TRAP", "trap")] {
        if let Some(n) = std::env::var(var).ok().and_then(|v| v.parse::<usize>().ok()) {
            w.send(&format!("H {hook} {n}\n"));
        }
    }
    Hello::Ready(w)
}

fn unhex(t: &str) -> Option<String> {
    if t == "-" {
        return Some(String::new());
    }
    if !t.len().is_multiple_of(2) || t.len() > FONT_NAME_CHARS * 8 || !t.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return None;
    }
    let bytes: Vec<u8> = (0..t.len() / 2).map(|i| u8::from_str_radix(&t[i * 2..i * 2 + 2], 16).unwrap_or(0)).collect();
    // (cleaned as a page's name is: text, no control characters, cut to the cap)
    let s = String::from_utf8(bytes).ok()?;
    Some(s.chars().filter(|c| !c.is_control()).take(FONT_NAME_CHARS).collect::<String>().trim().to_string())
}

/// Checks one font (its whole bytes, at most FONT_FILE_MAX) through the sandboxed worker, a slice of CPU time
/// a worker, each a fresh exec that gets the bytes again and goes on from the glyph the one before reached.
/// `px` is the size the preview is drawn for (the UI's text size on this screen). Every record is validated:
/// a worker can only lie inside the grammar.
pub fn check_font(data: &[u8], px: i32) -> FontOut {
    if data.is_empty() || data.len() > FONT_FILE_MAX {
        return FontOut::Refused(Refuse::Unreadable);
    }
    let px = px.clamp(8, 400);
    let (slice_ms, job_ms) = budget_ms();
    let mut facts: Option<FaceFacts> = None;
    let mut skip: Vec<u16> = Vec::new();
    let mut next = 0usize;
    let mut spent = 0u64;
    let mut records = 0usize;
    // The writer bounds the check itself (security review 2026-11): the worker's own word for its CPU time, and a
    // watchdog that a record restarts, let a worker the font took over run slices for weeks (futex sleeps cost no
    // CPU). A check is at most the slices its budget holds, and a slice at most what RLIMIT_CPU lets a worker run
    // and one watchdog; the CPU the kernel counted for a slice's worker counts when it is more than it claimed.
    let max_slices = job_ms / slice_ms.max(1) + 2;
    let mut slices = 0u64;
    loop {
        slices += 1;
        if slices > max_slices {
            return FontOut::Slow;
        }
        let cpu0 = children_cpu_ms();
        let mut w = match start_worker(data.len()) {
            Hello::Ready(w) => w,
            Hello::No(why) => return FontOut::NoWorker(why),
        };
        let slice_end = mono_ms() + slice_ms.max(s1::FONT_CPU_SECS * 1000) + watchdog_ms();
        let mut deadline = (mono_ms() + watchdog_ms()).min(slice_end);
        let scale = facts.as_ref().map_or(0, |f| f.scale_pm);
        if !(w.send(&format!("F {} {} {} {} {}\n", data.len(), next, px, slice_ms, scale)) && w.send_bytes(data, deadline)) {
            let stalled = mono_ms() >= deadline;
            w.kill_and_reap();
            return if stalled { FontOut::Slow } else { FontOut::Dead("the worker ended") };
        }
        let started = next;
        // one slice's records (it ends at its N, with the CPU time the worker says it used)
        let claimed = loop {
            let line = match w.read_line(1024, deadline) {
                Ok(l) => l,
                Err(ReadErr::Timeout) => {
                    w.kill_and_reap();
                    return FontOut::Slow;
                }
                Err(_) => {
                    // (ended by its CPU limit, whose signal ends a process that does not catch it: a glyph
                    // that never ends is a slow font, not a broken worker)
                    let st = w.kill_and_reap();
                    let cpu = libc::WIFSIGNALED(st) && libc::WTERMSIG(st) == libc::SIGXCPU;
                    return if cpu { FontOut::Slow } else { FontOut::Dead("the worker ended") };
                }
            };
            deadline = (mono_ms() + watchdog_ms()).min(slice_end);
            records += 1;
            // (4 MiB of records a job at most, DESIGN.md 4.3: a font has 65,535 glyphs, so far fewer)
            if records > 200_000 || !line.iter().all(|&c| (0x20..0x7f).contains(&c)) {
                w.kill_and_reap();
                return FontOut::Dead("the worker broke the protocol");
            }
            let s = String::from_utf8_lossy(&line).into_owned();
            let t: Vec<&str> = s.split(' ').collect();
            let num = |i: usize| t.get(i).and_then(|x| number(x)).map(|n| n as i64);
            let bad = |w: &mut Worker| {
                w.kill_and_reap();
                FontOut::Dead("the worker broke the protocol")
            };
            match (t[0], t.get(1).copied().unwrap_or(""), t.len()) {
                ("F", "face", 9) if started == 0 && facts.is_none() => {
                    let v: Vec<Option<i64>> = (2..9).map(num).collect();
                    if v.iter().any(|x| x.is_none()) {
                        return bad(&mut w);
                    }
                    let v: Vec<i64> = v.into_iter().flatten().collect();
                    let f = FaceFacts {
                        upem: v[0] as i32,
                        ascent: v[1] as i32,
                        descent: -(v[2] as i32),
                        scale_pm: v[3] as i32,
                        weight: (v[4] as i32).clamp(1, 1000),
                        italic: v[5] == 1,
                        mapped: (v[6] as i32).min(65_535),
                        ..Default::default()
                    };
                    if !f.sane() || v[5] > 1 || f.mapped < 1 {
                        return bad(&mut w);
                    }
                    facts = Some(f);
                }
                ("F", "wght", 6) => {
                    let (Some(f), Some(lo), Some(def), Some(hi), Some(at)) = (facts.as_mut(), num(2), num(3), num(4), num(5)) else { return bad(&mut w) };
                    if !(1 <= lo && lo <= def && def <= hi && hi <= 1000) || !(at == 0 || (def < at && at <= hi)) || f.wght.is_some() {
                        return bad(&mut w);
                    }
                    f.wght = Some((lo as i32, def as i32, hi as i32));
                    f.bold_at = at as i32;
                }
                ("F", "cover", 8) => {
                    let Some(f) = facts.as_mut() else { return bad(&mut w) };
                    for i in 0..6 {
                        let Some(n) = num(2 + i) else { return bad(&mut w) };
                        f.cover[i] = n.min(100_000) as i32;
                    }
                }
                ("F", "lacks", 3) => {
                    let (Some(f), Some(bits)) = (facts.as_mut(), num(2)) else { return bad(&mut w) };
                    f.lacks = (bits as u32) & 0xFFFF;
                }
                ("F", "family", 3) | ("F", "style", 3) => {
                    let (Some(f), Some(text)) = (facts.as_mut(), unhex(t[2])) else { return bad(&mut w) };
                    if t[1] == "family" {
                        f.family = text;
                    } else {
                        f.style = text;
                    }
                }
                ("K", _, 2) => {
                    let Some(g) = num(1).filter(|&g| (next as i64..=65_535).contains(&g)) else { return bad(&mut w) };
                    if facts.is_none() || skip.last().is_some_and(|&l| l as i64 >= g) || skip.len() >= FONT_SKIP_MAX {
                        return bad(&mut w);
                    }
                    skip.push(g as u16);
                }
                ("G", _, 2) => {
                    if num(1).is_none() {
                        return bad(&mut w);
                    }
                }
                ("N", _, 3) => {
                    let (Some(n), Some(ms)) = (num(1), num(2)) else { return bad(&mut w) };
                    if facts.is_none() || n as usize <= next || n > 65_535 {
                        return bad(&mut w);
                    }
                    next = n as usize;
                    break (ms as u64).max(1);
                }
                ("P", _, 3) => {
                    let (Some(f), Some(pw), Some(ph)) = (facts.as_ref(), num(1), num(2)) else { return bad(&mut w) };
                    let hmax = (px as f32 * 1.25).ceil() as i64 + 8;
                    if !(8..=(px as f32 * PREVIEW_EMS) as i64).contains(&pw) || ph != hmax {
                        return bad(&mut w);
                    }
                    let n = pw as usize * ph as usize;
                    let mut pixels: Vec<u8> = Vec::new();
                    if pixels.try_reserve_exact(n).is_err() {
                        w.kill_and_reap();
                        return FontOut::Retry("out of memory");
                    }
                    pixels.resize(n, 0);
                    if w.read_exact(&mut pixels, mono_ms() + watchdog_ms()).is_err() {
                        w.kill_and_reap();
                        return FontOut::Dead("the worker ended while it sent the preview");
                    }
                    // then its last line, and it is done
                    let done = w.read_line(64, mono_ms() + watchdog_ms()).ok().map(|l| String::from_utf8_lossy(&l).into_owned());
                    let fine = done.as_deref().and_then(|d| d.strip_prefix("D ")).and_then(number).is_some();
                    w.send("Q\n");
                    w.finish(500);
                    if !fine {
                        return FontOut::Dead("the worker broke the protocol");
                    }
                    let mut f = f.clone();
                    // More than one glyph in a hundred broken: not a font to read by.
                    if skip.len() * 100 > f.mapped.max(1) as usize {
                        return FontOut::Refused(Refuse::Unreadable);
                    }
                    f.skip = skip;
                    return FontOut::Passed { facts: f, preview: pixels, w: pw as i32, h: ph as i32 };
                }
                ("E", "bad", 3) => {
                    let why = match num(2) {
                        Some(0) => Refuse::Unreadable,
                        Some(1) => Refuse::NoLatin,
                        Some(2) => Refuse::Format,
                        _ => return bad(&mut w),
                    };
                    w.send("Q\n");
                    w.finish(500);
                    return FontOut::Refused(why);
                }
                ("E", "mem", 2) => {
                    w.kill_and_reap();
                    return FontOut::Retry("out of memory");
                }
                ("E", "sigsys", 3) => {
                    let n = num(2).unwrap_or(-1);
                    w.kill_and_reap();
                    qlog!("work: a font's worker made call %d, which the sandbox does not allow", n as i32);
                    return FontOut::Dead("the worker made a call the sandbox does not allow");
                }
                _ => return bad(&mut w),
            }
        };
        w.send("Q\n");
        w.finish(500);
        spent += claimed.max(children_cpu_ms().saturating_sub(cpu0));
        if spent >= job_ms {
            return FontOut::Slow;
        }
    }
}

/// The CPU time of this process's reaped children (the workers), in ms.
fn children_cpu_ms() -> u64 {
    // SAFETY: ru is written by getrusage before it is read.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut ru) };
    (ru.ru_utime.tv_sec as u64 + ru.ru_stime.tv_sec as u64) * 1000 + (ru.ru_utime.tv_usec as u64 + ru.ru_stime.tv_usec as u64) / 1000
}

// ---- the verdict: fonts/.cache/<key>.font ----

/// What the `.font` file says of a font file: the whole file's hash and size, and what the check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// FNV-1a 64 of the whole file, and its size: the UI loads a font only when the bytes it read are these
    pub hash: u64,
    pub size: u64,
    pub what: VerdictKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerdictKind {
    Passed(FaceFacts),
    Refused(Refuse),
    Slow,
}

/// A `.font` file is at most this long (656 glyph numbers, two names, a dozen short lines).
pub const VERDICT_MAX: usize = 8 << 10;

/// FNV-1a 64 over a font's whole bytes.
pub fn font_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for &b in data {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    h
}

impl Verdict {
    /// The file's text.
    pub fn to_text(&self) -> String {
        let mut s = format!("grimoire-font 1\nhash={:016x}\nsize={}\n", self.hash, self.size);
        match &self.what {
            VerdictKind::Refused(r) => {
                s.push_str(match r {
                    Refuse::Unreadable => "why=unreadable\n",
                    Refuse::NoLatin => "why=nolatin\n",
                    Refuse::Format => "why=format\n",
                });
            }
            VerdictKind::Slow => s.push_str("why=slow\n"),
            VerdictKind::Passed(f) => {
                s.push_str(&format!(
                    "ok=1\nupem={}\nascent={}\ndescent={}\nscale={}\nweight={}\nitalic={}\nmapped={}\n",
                    f.upem, f.ascent, -f.descent, f.scale_pm, f.weight, f.italic as i32, f.mapped
                ));
                if let Some((lo, def, hi)) = f.wght {
                    s.push_str(&format!("wght={lo} {def} {hi}\nbold={}\n", f.bold_at));
                }
                s.push_str(&format!("cover={} {} {} {} {} {}\n", f.cover[0], f.cover[1], f.cover[2], f.cover[3], f.cover[4], f.cover[5]));
                s.push_str(&format!("lacks={}\n", f.lacks));
                s.push_str(&format!("family={}\nstyle={}\n", f.family, f.style));
                if !f.skip.is_empty() {
                    s.push_str("skip=");
                    s.push_str(&f.skip.iter().map(|g| g.to_string()).collect::<Vec<_>>().join(","));
                    s.push('\n');
                }
            }
        }
        s
    }

    /// Reads a `.font` file's bytes, strictly: every line known, every number in range, or None (the font is
    /// then checked again). Nothing here trusts the file: it sits on storage a computer can write.
    pub fn parse(text: &[u8]) -> Option<Verdict> {
        if text.len() > VERDICT_MAX {
            return None;
        }
        let text = std::str::from_utf8(text).ok()?;
        let mut lines = text.split('\n');
        if lines.next()? != "grimoire-font 1" {
            return None;
        }
        let (mut hash, mut size, mut why, mut ok) = (None, None, None, false);
        let mut f = FaceFacts::default();
        let int = |v: &str| number(v).map(|n| n as i32);
        for l in lines {
            if l.is_empty() {
                continue;
            }
            let (k, v) = l.split_once('=')?;
            match k {
                "hash" if v.len() == 16 && v.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) => hash = u64::from_str_radix(v, 16).ok(),
                "size" => size = number(v).map(|n| n as u64),
                "why" => {
                    why = Some(match v {
                        "unreadable" => VerdictKind::Refused(Refuse::Unreadable),
                        "nolatin" => VerdictKind::Refused(Refuse::NoLatin),
                        "format" => VerdictKind::Refused(Refuse::Format),
                        "slow" => VerdictKind::Slow,
                        _ => return None,
                    })
                }
                "ok" if v == "1" => ok = true,
                "upem" => f.upem = int(v)?,
                "ascent" => f.ascent = int(v)?,
                "descent" => f.descent = -int(v)?,
                "scale" => f.scale_pm = int(v)?,
                "weight" => f.weight = int(v)?.clamp(1, 1000),
                "italic" => f.italic = int(v)? == 1,
                "mapped" => f.mapped = int(v)?,
                "wght" => {
                    let p: Vec<i32> = v.split(' ').map(int).collect::<Option<Vec<_>>>()?;
                    if p.len() != 3 || !(1 <= p[0] && p[0] <= p[1] && p[1] <= p[2] && p[2] <= 1000) {
                        return None;
                    }
                    f.wght = Some((p[0], p[1], p[2]));
                }
                "bold" => f.bold_at = int(v)?,
                "cover" => {
                    let p: Vec<i32> = v.split(' ').map(int).collect::<Option<Vec<_>>>()?;
                    if p.len() != 6 {
                        return None;
                    }
                    f.cover.copy_from_slice(&p);
                }
                "lacks" => f.lacks = int(v)? as u32 & 0xFFFF,
                "family" => f.family = v.chars().filter(|c| !c.is_control()).take(FONT_NAME_CHARS).collect(),
                "style" => f.style = v.chars().filter(|c| !c.is_control()).take(FONT_NAME_CHARS).collect(),
                "skip" => {
                    for g in v.split(',') {
                        let g = number(g).filter(|&g| g <= 65_535)? as u16;
                        f.skip.push(g);
                    }
                }
                _ => return None,
            }
        }
        let (hash, size) = (hash?, size?);
        if size == 0 || size > FONT_FILE_MAX as u64 {
            return None;
        }
        let what = match (ok, why) {
            (false, Some(w)) => w,
            (true, None) => {
                let axis_ok = match f.wght {
                    Some((_, def, hi)) => f.bold_at == 0 || (def < f.bold_at && f.bold_at <= hi),
                    None => f.bold_at == 0,
                };
                if !f.sane() || !axis_ok {
                    return None;
                }
                VerdictKind::Passed(f)
            }
            _ => return None,
        };
        Some(Verdict { hash, size, what })
    }
}

// ---- the face font.rs draws by ----

/// A checked font's face in the UI: the file's bytes (they passed the check: their hash is the verdict's),
/// read by skrifa for each glyph, with the caps applied again to every outline before stb draws it.
pub struct SafeFace {
    data: Rc<Vec<u8>>,
    coords: Vec<NormalizedCoord>,
    facts: Rc<FaceFacts>,
    /// how many glyphs the font says it has: the check walked no glyph past them, so none is drawn
    nglyphs: u32,
    /// code point to (glyph, advance in font units); (0, 0) for one the face does not draw
    look: std::collections::HashMap<u32, (u32, i32)>,
}

/// Lookups kept a face (a screen uses a few hundred characters; a name in kanji a few more).
const LOOKUPS_MAX: usize = 4096;

impl SafeFace {
    /// The face of `data` (the default location, or its bold through the weight axis), None when skrifa no
    /// longer reads the bytes as the check did or the facts are not a font's.
    pub fn new(data: Rc<Vec<u8>>, facts: Rc<FaceFacts>, bold: bool) -> Option<SafeFace> {
        if !facts.sane() || data.len() > FONT_FILE_MAX || (bold && facts.bold_at == 0) {
            return None;
        }
        let font = FontRef::from_index(&data, 0).ok()?;
        font.outline_glyphs().format()?;
        let coords = if bold { location_for(&font, facts.bold_at as f32) } else { Vec::new() };
        let nglyphs = (font.metrics(Size::unscaled(), LocationRef::default()).glyph_count as u32).max(1);
        Some(SafeFace { data, coords, facts, nglyphs, look: std::collections::HashMap::new() })
    }

    pub fn facts(&self) -> &FaceFacts {
        &self.facts
    }

    /// The bytes held (for the loaded-fonts cap; a bold through the axis shares its regular's).
    pub fn data(&self) -> &Rc<Vec<u8>> {
        &self.data
    }

    /// The glyph and advance (font units) of `cp`, None for a character this face does not draw: one the
    /// font lacks, or one whose glyph failed the check. Kept, so measuring never reads the cmap twice.
    pub fn glyph(&mut self, cp: u32) -> Option<(u32, i32)> {
        if let Some(&(g, a)) = self.look.get(&cp) {
            return (g != 0).then_some((g, a));
        }
        let found = self.find(cp);
        if self.look.len() >= LOOKUPS_MAX {
            self.look.clear();
        }
        self.look.insert(cp, found.unwrap_or((0, 0)));
        found
    }

    fn find(&self, cp: u32) -> Option<(u32, i32)> {
        let font = FontRef::from_index(&self.data, 0).ok()?;
        let g = font.charmap().map(cp)?.to_u32();
        if g == 0 || g >= self.nglyphs || g > 65_535 || self.facts.skip.binary_search(&(g as u16)).is_ok() {
            return None;
        }
        let adv = font.glyph_metrics(Size::unscaled(), LocationRef::new(&self.coords)).advance_width(GlyphId::new(g))?;
        let line = self.facts.line();
        if !(adv.is_finite() && adv >= 0.0 && adv <= (FONT_BOX_LINES * line) as f32) {
            return None;
        }
        Some((g, adv.round_ties_even() as i32))
    }

    /// Glyph `gid` at a line height of `px`, outlined through `out` (the one outline buffer the UI keeps): its
    /// bitmap (empty for a glyph with no ink), None when the size is outside what the check drew or the outline
    /// does not pass the caps (nothing is drawn then).
    pub fn bitmap(&self, out: &mut Outliner, gid: u32, px: f32) -> Option<Bitmap> {
        if !(FONT_PX_MIN..=FONT_PX_MAX).contains(&px) {
            return None;
        }
        let line = self.facts.line();
        let font = FontRef::from_index(&self.data, 0).ok()?;
        let glyphs = font.outline_glyphs();
        match out.outline(&glyphs, gid, &self.coords, line) {
            Ok(None) => Some(Bitmap::default()),
            Ok(Some(b)) => rasterize(&mut out.verts, b, px / line as f32),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts_round_trip_and_parse_strictly() {
        let f = FaceFacts {
            upem: 1000,
            ascent: 950,
            descent: -250,
            scale_pm: 1040,
            weight: 400,
            italic: false,
            wght: Some((100, 400, 900)),
            bold_at: 700,
            mapped: 2400,
            skip: vec![3, 77, 4000],
            family: "Some Family".into(),
            style: "Regular".into(),
            lacks: 4,
            cover: [96, 128, 0, 0, 0, 0],
        };
        let v = Verdict { hash: 0x0123_4567_89ab_cdef, size: 54321, what: VerdictKind::Passed(f) };
        let text = v.to_text();
        assert_eq!(Verdict::parse(text.as_bytes()), Some(v.clone()));
        for (kind, word) in [(VerdictKind::Refused(Refuse::NoLatin), "nolatin"), (VerdictKind::Slow, "slow")] {
            let r = Verdict { hash: 1, size: 2, what: kind };
            assert!(r.to_text().contains(word));
            assert_eq!(Verdict::parse(r.to_text().as_bytes()), Some(r));
        }
        // a line this version does not know, a number out of range, another header: not a verdict
        for bad in [
            text.replace("upem=1000", "upem=1000\nfuture=1"),
            text.replace("upem=1000", "upem=3"),
            text.replace("scale=1040", "scale=5000"),
            text.replace("skip=3,77,4000", "skip=77,3"),
            text.replace("skip=3,77,4000", "skip=70000"),
            text.replace("bold=700", "bold=300"),
            text.replace("grimoire-font 1", "grimoire-font 2"),
            text.replace("ok=1\n", ""),
            text.replace("hash=0123456789abcdef", "hash=xyz"),
            text.replace("size=54321", "size=0"),
            text.replace("ascent=950", "ascent=-950"),
            format!("{text}why=slow\n"),
        ] {
            assert_eq!(Verdict::parse(bad.as_bytes()), None, "{bad}");
        }
        assert_eq!(Verdict::parse(&vec![b'a'; VERDICT_MAX + 1]), None);
        assert_eq!(Verdict::parse(b""), None);
    }

    #[test]
    fn records_numbers_are_strict() {
        assert_eq!(number("0"), Some(0));
        assert_eq!(number("650"), Some(650));
        for bad in ["", "-1", "+1", "01", "1 ", "1e3", "9999999999"] {
            assert_eq!(number(bad), None, "{bad}");
        }
        assert_eq!(unhex("-").as_deref(), Some(""));
        assert_eq!(unhex("4e6f746f").as_deref(), Some("Noto"));
        assert_eq!(unhex("4E6F"), None);
        assert_eq!(unhex("4e6"), None);
        assert_eq!(unhex("ff"), None); // not UTF-8
        assert_eq!(unhex(&hex("Li\u{7}ne\n")).as_deref(), Some("Line"));
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("fontcheck.rs").contains('\u{2014}'));
    }
}
