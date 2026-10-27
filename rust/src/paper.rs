//! paper.h: page backgrounds. The numeric values are stored on disk: never reorder.

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaperKind(pub u32);
/// 5.5 mm rule
pub const PAPER_NARROW: PaperKind = PaperKind(0);
/// 7.1 mm rule
pub const PAPER_MEDIUM: PaperKind = PaperKind(1);
/// 9.0 mm rule
pub const PAPER_WIDE: PaperKind = PaperKind(2);
/// 8 roomy five-line staves
pub const PAPER_MUSIC: PaperKind = PaperKind(3);
pub const PAPER_BLANK: PaperKind = PaperKind(4);
/// the three rules again, without the margin line
pub const PAPER_NARROW_NM: PaperKind = PaperKind(5);
pub const PAPER_MEDIUM_NM: PaperKind = PaperKind(6);
pub const PAPER_WIDE_NM: PaperKind = PaperKind(7);
/// piano grand staff: treble + bass systems with clefs and a brace
pub const PAPER_GRAND: PaperKind = PaperKind(8);
/// guitar tablature: six-line staves
pub const PAPER_TAB: PaperKind = PaperKind(9);
/// 5 mm dot grid
pub const PAPER_DOTS: PaperKind = PaperKind(10);
/// 5 mm graph paper
pub const PAPER_GRID: PaperKind = PaperKind(11);
/// Cornell notes: cue column, notes, summary
pub const PAPER_CORNELL: PaperKind = PaperKind(12);
/// bass tablature, 4 / 5 / 6 / 7 strings
pub const PAPER_BASS4: PaperKind = PaperKind(13);
pub const PAPER_BASS5: PaperKind = PaperKind(14);
pub const PAPER_BASS6: PaperKind = PaperKind(15);
pub const PAPER_BASS7: PaperKind = PaperKind(16);
/// fixed kinds end here
pub const PAPER_COUNT: i32 = 17;
/// A custom music layout (see MusicLayout): this flag | systems << 24 | part i << 4*i.
/// The top four bits name the namespace: 0x5 is image papers (below), 0x6 is reserved for a
/// ruled-paper builder (docs/FORMAT.md).
pub const PAPER_CUSTOM_FLAG: u32 = 0x40000000;

/// An image paper: PAPER_IMAGE_FLAG | role << 24 | id, a picture kept in the page's notebook as
/// bg/<id, 6 hex digits>.png (src/bgimage.h) and drawn by page_render (paper_render alone draws it
/// blank: it has no notebook). Role 0 is a template (a page added after it gets it too), 1 a document
/// page (an imported page image: a page added after it gets the notebook's paper). Bits 26-27 are
/// kept for colour and are 0: a value with them set, or with role 2 or 3, is a newer version's.
pub const PAPER_IMAGE_FLAG: u32 = 0x50000000;
pub const PAPER_ROLE_TEMPLATE: i32 = 0;
pub const PAPER_ROLE_DOCUMENT: i32 = 1;
/// a well-formed image paper
pub fn paper_is_image(k: PaperKind) -> bool {
    let v = k.0;
    // The namespace, then bits 24-27: the role (0 or 1) and the colour bits (0).
    (v & 0xF0000000) == PAPER_IMAGE_FLAG && (v >> 24 & 15) <= PAPER_ROLE_DOCUMENT as u32
}
/// an image paper of the document page role
pub fn paper_is_document(k: PaperKind) -> bool {
    paper_is_image(k) && (k.0 >> 24 & 3) == PAPER_ROLE_DOCUMENT as u32
}
/// id: the low 24 bits are used
pub fn paper_image(role: i32, id: u32) -> PaperKind {
    PaperKind(PAPER_IMAGE_FLAG | ((role as u32) & 3) << 24 | (id & 0xFFFFFF))
}
#[inline]
pub fn paper_image_id(k: PaperKind) -> u32 {
    k.0 & 0xFFFFFF
}

/// The parts a custom music system stacks, top to bottom. Stored on disk: never reorder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MusicPart(pub i32);
pub const PART_NONE: MusicPart = MusicPart(0);
pub const PART_TREBLE: MusicPart = MusicPart(1);
pub const PART_BASS: MusicPart = MusicPart(2);
pub const PART_ALTO: MusicPart = MusicPart(3);
pub const PART_TENOR: MusicPart = MusicPart(4);
pub const PART_PERC: MusicPart = MusicPart(5);
pub const PART_STAFF: MusicPart = MusicPart(6);
pub const PART_TAB4: MusicPart = MusicPart(7);
pub const PART_TAB5: MusicPart = MusicPart(8);
pub const PART_TAB6: MusicPart = MusicPart(9);
pub const PART_TAB7: MusicPart = MusicPart(10);
pub const PART_LYRICS: MusicPart = MusicPart(11);
pub const PART_COUNT: i32 = 12;

pub const MUSIC_MAX_PARTS: usize = 6;
pub const MUSIC_MAX_SYSTEMS: i32 = 12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MusicLayout {
    /// systems per page, 1..MUSIC_MAX_SYSTEMS
    pub systems: i32,
    /// parts per system, 1..MUSIC_MAX_PARTS
    pub n: i32,
    pub part: [MusicPart; MUSIC_MAX_PARTS],
}


/// in the custom music namespace (well-formed or not)
pub fn paper_is_custom(k: PaperKind) -> bool {
    (k.0 & 0xF0000000) == PAPER_CUSTOM_FLAG
}
/// A fixed kind, a well-formed custom layout or a well-formed image paper. Anything else is a paper
/// from a newer version: pages keep it, draw it blank and call it "Newer paper".
pub fn paper_is_known(k: PaperKind) -> bool {
    k.0 < PAPER_COUNT as u32 || decode(k).is_some() || paper_is_image(k)
}
/// A kind the pickers offer (a fixed kind or a well-formed custom layout) as it is; anything else
/// becomes `fallback`, an image paper too: it needs its notebook's picture (a new notebook, the
/// music layout setting).
pub fn paper_sanitize(v: u32, fallback: PaperKind) -> PaperKind {
    if v < PAPER_COUNT as u32 || decode(PaperKind(v)).is_some() {
        PaperKind(v)
    } else {
        fallback
    }
}
pub fn music_encode(m: &MusicLayout) -> PaperKind {
    let mut v = PAPER_CUSTOM_FLAG | (clampi(m.systems, 1, MUSIC_MAX_SYSTEMS) as u32) << 24;
    for i in 0..m.n.min(MUSIC_MAX_PARTS as i32) {
        v |= ((m.part[i as usize].0 & 15) as u32) << (4 * i);
    }
    PaperKind(v)
}

// music_decode, with the layout as its result: the C lets its caller pass NULL where only the
// answer matters (paper_is_known, paper_sanitize).
fn decode(k: PaperKind) -> Option<MusicLayout> {
    if !paper_is_custom(k) {
        return None;
    }
    let v = k.0;
    let mut r = MusicLayout { systems: ((v >> 24) & 15) as i32, ..Default::default() };
    for i in 0..MUSIC_MAX_PARTS {
        let p = ((v >> (4 * i)) & 15) as i32;
        if p == PART_NONE.0 {
            break;
        }
        if p >= PART_COUNT {
            return None;
        }
        r.part[r.n as usize] = MusicPart(p);
        r.n += 1;
    }
    if r.n == 0 || r.systems < 1 || r.systems > MUSIC_MAX_SYSTEMS {
        return None;
    }
    Some(r)
}

/// false if k is not a custom layout
pub fn music_decode(k: PaperKind, m: &mut MusicLayout) -> bool {
    match decode(k) {
        Some(r) => {
            *m = r;
            true
        }
        None => false,
    }
}

/// "Treble staff", "Tab, 6 lines", ...
pub fn music_part_name(p: MusicPart) -> &'static str {
    match p {
        // TRANSLATORS: the parts a custom music layout stacks: on the Custom music sheet's buttons
        // (280 px at 44 px in Gothic, 298 in Classic, or two lines at 34 px) and in its list of the
        // layout's parts.
        PART_TREBLE => nc_("music part", "Treble staff"),
        PART_BASS => nc_("music part", "Bass staff"),
        PART_ALTO => nc_("music part", "Alto staff"),
        PART_TENOR => nc_("music part", "Tenor staff"),
        PART_PERC => nc_("music part", "Drum staff"),
        PART_STAFF => nc_("music part", "Plain staff"),
        PART_TAB4 => nc_("music part", "Tab, 4 lines"),
        PART_TAB5 => nc_("music part", "Tab, 5 lines"),
        PART_TAB6 => nc_("music part", "Tab, 6 lines"),
        PART_TAB7 => nc_("music part", "Tab, 7 lines"),
        PART_LYRICS => nc_("music part", "Lyric line"),
        _ => "",
    }
}
/// the same in the language in use, for the screens
pub fn part_label(p: MusicPart) -> &'static str {
    c_("music part", music_part_name(p))
}

fn part_lines(p: MusicPart) -> i32 {
    if p >= PART_TAB4 && p <= PART_TAB7 {
        return 4 + (p.0 - PART_TAB4.0);
    }
    if p == PART_LYRICS {
        1
    } else {
        5
    }
}

// Room in staff spaces: a part's own height, and the gap above part i of a system
// (staves get room for ledger lines; a lyric line sits close to what it sings).
fn part_height(p: MusicPart) -> f32 {
    (part_lines(p) - 1) as f32
}
fn gap_above(m: &MusicLayout, i: usize) -> f32 {
    if i == 0 {
        return 0.0;
    }
    if m.part[i] == PART_LYRICS || m.part[i - 1] == PART_LYRICS {
        return 3.5;
    }
    4.5
}
const SYSTEM_GAP: f32 = 6.0;
const MUSIC_TOP_MM: f32 = 14.0;
const MUSIC_BOT_MM: f32 = 12.0;
/// smaller than this is no longer comfortable to write in
const MIN_SPACE_MM: f32 = 1.5;
/// larger looks like a children's primer
const MAX_SPACE_MM: f32 = 3.0;

fn system_units(m: &MusicLayout) -> f32 {
    let mut u = 0.0f32;
    for i in 0..m.n as usize {
        u += gap_above(m, i) + part_height(m.part[i]);
    }
    u
}

/// The most systems that fit on a page page_h px tall (at dpi) with staff spaces still big enough
/// to write in. A layout keeps its systems on a page of another height: paper_render spaces them
/// to fit (with smaller staff spaces on a shorter page), it never drops one.
pub fn music_max_systems(m: &MusicLayout, page_h: i32, dpi: f32) -> i32 {
    // The page's height in mm less the margins music_custom keeps: a 1986x2648 page has room for
    // more systems than an 1860x2480 one (whose answers are the same as when 2480 px was assumed).
    let page_mm = page_h as f32 / dpi * 25.4;
    let avail = page_mm - MUSIC_TOP_MM - MUSIC_BOT_MM;
    let su = system_units(m);
    let mut best = 1;
    for s in 1..=MUSIC_MAX_SYSTEMS {
        // (clang fuses the first product of a sum of two: the C's -ffp-contract=on, PORTING.md)
        if avail / (s as f32).mul_add(su, ((s - 1) as f32) * SYSTEM_GAP) >= MIN_SPACE_MM {
            best = s;
        }
    }
    best
}

/// treble staff over 6-line tab, 5 systems
pub fn music_default() -> PaperKind {
    let mut m = MusicLayout { systems: 5, n: 2, ..Default::default() };
    m.part[0] = PART_TREBLE;
    m.part[1] = PART_TAB6;
    music_encode(&m)
}

/// Pickers offer the fixed kinds in paper_at() order, then one "Custom music" tile.
pub const PAPER_PICKER_COUNT: i32 = PAPER_COUNT + 1;

// The English names are identifiers too: /api/list.json gives them to scripts (httpd.c), so they
// never change with the language; paper_label is what the screens and the share pages show.
/// e.g. "Narrow rule", "Narrow, no margin", "Template", "Imported page", "Newer paper"
pub fn paper_name(k: PaperKind) -> &'static str {
    // A picture: a template the owner copied in, or a page of an imported document.
    // TRANSLATORS: papers that are pictures: a template copied into grimoire/templates, and a page
    // of a document imported as pictures (named as the other papers are, below).
    if paper_is_image(k) {
        return if paper_is_document(k) { nc_("paper", "Imported page") } else { nc_("paper", "Template") };
    }
    match k {
        // TRANSLATORS: papers, named under each paper's picture in New notebook and in the editor's
        // paper sheet: 192 px at 31 px in New notebook sideways (222 upright; Classic 4 px more), on
        // one line or two (a break after a comma is preferred: "Narrow," over "no margin"). Also on the
        // share pages.
        PAPER_NARROW => nc_("paper", "Narrow rule"),
        PAPER_MEDIUM => nc_("paper", "Medium rule"),
        PAPER_WIDE => nc_("paper", "Wide rule"),
        PAPER_MUSIC => nc_("paper", "Sheet music"),
        PAPER_BLANK => nc_("paper", "Blank"),
        PAPER_NARROW_NM => nc_("paper", "Narrow, no margin"),
        PAPER_MEDIUM_NM => nc_("paper", "Medium, no margin"),
        PAPER_WIDE_NM => nc_("paper", "Wide, no margin"),
        PAPER_GRAND => nc_("paper", "Grand staff"),
        PAPER_TAB => nc_("paper", "Guitar tab"),
        PAPER_DOTS => nc_("paper", "Dot grid"),
        PAPER_GRID => nc_("paper", "Graph paper"),
        PAPER_CORNELL => nc_("paper", "Cornell notes"),
        PAPER_BASS4 => nc_("paper", "4-string bass"),
        PAPER_BASS5 => nc_("paper", "5-string bass"),
        PAPER_BASS6 => nc_("paper", "6-string bass"),
        PAPER_BASS7 => nc_("paper", "7-string bass"),
        _ => {
            if paper_is_known(k) {
                return nc_("paper", "Custom music");
            }
            // TRANSLATORS: a paper made by a newer Grimoire, which this one draws blank.
            nc_("paper", "Newer paper")
        }
    }
}

/// paper_name in the language in use: what the screens show (paper_name stays English, an
/// identifier for /api/list.json).
pub fn paper_label(k: PaperKind) -> &'static str {
    c_("paper", paper_name(k))
}

/// The order papers are offered in pickers (index 0..PAPER_COUNT-1 -> kind).
pub fn paper_at(index: i32) -> PaperKind {
    const ORDER: [PaperKind; PAPER_COUNT as usize] = [
        PAPER_NARROW,
        PAPER_MEDIUM,
        PAPER_WIDE,
        PAPER_NARROW_NM,
        PAPER_MEDIUM_NM,
        PAPER_WIDE_NM,
        PAPER_CORNELL,
        PAPER_DOTS,
        PAPER_GRID,
        PAPER_MUSIC,
        PAPER_GRAND,
        PAPER_TAB,
        PAPER_BASS4,
        PAPER_BASS5,
        PAPER_BASS6,
        PAPER_BASS7,
        PAPER_BLANK,
    ];
    ORDER[clampi(index, 0, PAPER_COUNT - 1) as usize]
}

const NARROW_MM: f32 = 5.5;
const MEDIUM_MM: f32 = 7.1;
const WIDE_MM: f32 = 9.0;

// The C's (int)lrintf: round to nearest, ties to even (the default rounding mode).
fn lrintf(x: f32) -> i32 {
    x.round_ties_even() as i32
}

fn px_mm(mm: f32, dpi: f32) -> i32 {
    lrintf(mm_to_px(mm, dpi))
}
fn thick(px300: f32, dpi: f32) -> i32 {
    let t = lrintf(px300 * dpi / 300.0);
    if t < 1 {
        1
    } else {
        t
    }
}

// (Where the C has `a * b + c` in one expression, clang's default -ffp-contract=on makes it one
// fused multiply-add and so does `a.mul_add(b, c)` here: PORTING.md, Floating point. Of a sum of two
// products clang fuses the left one; a difference `c - a * b` is `(-a).mul_add(b, c)`.)

fn ruled(c: &mut Canvas, pitch_mm: f32, dpi: f32, margin: bool) {
    let w = c.w;
    let h = c.h;
    let line_t = thick(2.0, dpi);
    let pitch = mm_to_px(pitch_mm, dpi);
    let first = mm_to_px(22.0, dpi);
    let bottom = h - px_mm(10.0, dpi);
    // Header rule: slightly heavier.
    canvas_hline(c, 0, w, lrintf(first), thick(3.0, dpi), 150);
    let mut i = 1;
    loop {
        let y = lrintf(pitch.mul_add(i as f32, first));
        if y > bottom {
            break;
        }
        canvas_hline(c, 0, w, y, line_t, 175);
        i += 1;
    }
    if !margin {
        return;
    }
    // Double hairline margin.
    let mx = px_mm(25.0, dpi);
    canvas_vline(c, mx, 0, h, line_t, 140);
    canvas_vline(c, mx + thick(7.0, dpi), 0, h, line_t, 140);
}

fn music(c: &mut Canvas, dpi: f32) {
    // 8 staves with roomy 2.7 mm staff spaces: written notes need far more room than printed.
    const STAVES: i32 = 8;
    // blank space between staves, in staff spaces
    const GAP_SPACES: f32 = 5.0;
    let w = c.w;
    let h = c.h;
    let top = mm_to_px(14.0, dpi);
    let bot = mm_to_px(12.0, dpi);
    let space = (h as f32 - top - bot) / (STAVES as f32).mul_add(4.0, ((STAVES - 1) as f32) * GAP_SPACES);
    let lt = thick(2.0, dpi);
    let x0 = px_mm(10.0, dpi);
    let x1 = w - px_mm(10.0, dpi);
    for s in 0..STAVES {
        let y0 = ((s as f32) * space).mul_add(4.0 + GAP_SPACES, top);
        let ytop = lrintf(y0);
        let ybot = lrintf(4.0f32.mul_add(space, y0));
        for l in 0..5 {
            canvas_hline(c, x0, x1, lrintf((l as f32).mul_add(space, y0)), lt, 90);
        }
        canvas_vline(c, x0, ytop, ybot + lt, lt, 90);
        canvas_vline(c, x1 - lt, ytop, ybot + lt, lt, 90);
    }
}

// Piano grand staff: 5 systems of a treble and a bass staff joined by a brace and a system
// line, with clefs. Staff spaces come out around 2 mm: room to write.
fn grand(c: &mut Canvas, dpi: f32) {
    const SYSTEMS: i32 = 5;
    // gaps in staff spaces: treble->bass, system->system
    const INNER: f32 = 5.0;
    const BETWEEN: f32 = 6.0;
    let w = c.w;
    let h = c.h;
    let top = mm_to_px(16.0, dpi);
    let bot = mm_to_px(12.0, dpi);
    let space = (h as f32 - top - bot) / (SYSTEMS as f32).mul_add(8.0 + INNER, ((SYSTEMS - 1) as f32) * BETWEEN);
    let lt = thick(2.0, dpi);
    let x0 = px_mm(16.0, dpi);
    let x1 = w - px_mm(10.0, dpi);
    for sys in 0..SYSTEMS {
        let y0 = ((sys as f32) * space).mul_add(8.0 + INNER + BETWEEN, top);
        // bass staff top line
        let yb = space.mul_add(4.0 + INNER, y0);
        for l in 0..5 {
            canvas_hline(c, x0, x1, lrintf((l as f32).mul_add(space, y0)), lt, 90);
            canvas_hline(c, x0, x1, lrintf((l as f32).mul_add(space, yb)), lt, 90);
        }
        let ytop = lrintf(y0);
        let ybot = lrintf(4.0f32.mul_add(space, yb));
        // system line joins both staves
        canvas_vline(c, x0, ytop, ybot + lt, thick(3.0, dpi), 70);
        canvas_vline(c, x1 - lt, ytop, lrintf(4.0f32.mul_add(space, y0)) + lt, lt, 90);
        canvas_vline(c, x1 - lt, lrintf(yb), ybot + lt, lt, 90);
        // Brace: a bold bracket with curled ends left of the system line.
        let bx = x0 as f32 - mm_to_px(3.0, dpi);
        canvas_line_aa(c, bx, ytop as f32 + space, bx, ybot as f32 - space, thick(5.0, dpi) as f32, 60);
        canvas_line_aa(c, bx, ytop as f32 + space, bx + mm_to_px(2.0, dpi), ytop as f32, thick(4.0, dpi) as f32, 60);
        canvas_line_aa(c, bx, ybot as f32 - space, bx + mm_to_px(2.0, dpi), ybot as f32, thick(4.0, dpi) as f32, 60);
        // Clefs (Material Design music glyphs from the UI font).
        let cw = lrintf(space * 3.2);
        font_draw_icon(c, FontFace::Regular, 0xF0F70, Rect { x: x0 + space as i32, y: lrintf((-space).mul_add(1.6, y0)), w: cw, h: lrintf(space * 7.2) }, 110);
        font_draw_icon(c, FontFace::Regular, 0xF0F6F, Rect { x: x0 + space as i32, y: lrintf((-space).mul_add(0.2, yb)), w: cw, h: lrintf(space * 3.6) }, 110);
    }
}

// Guitar tablature: six-line staves with "TAB" at the start.
fn tab(c: &mut Canvas, dpi: f32) {
    const STAVES: i32 = 9;
    const GAP: f32 = 4.0;
    let w = c.w;
    let h = c.h;
    let top = mm_to_px(14.0, dpi);
    let bot = mm_to_px(12.0, dpi);
    let space = (h as f32 - top - bot) / (STAVES as f32).mul_add(5.0, ((STAVES - 1) as f32) * GAP);
    let lt = thick(2.0, dpi);
    let x0 = px_mm(10.0, dpi);
    let x1 = w - px_mm(10.0, dpi);
    for s in 0..STAVES {
        let y0 = ((s as f32) * space).mul_add(5.0 + GAP, top);
        for l in 0..6 {
            canvas_hline(c, x0, x1, lrintf((l as f32).mul_add(space, y0)), lt, 100);
        }
        canvas_vline(c, x0, lrintf(y0), lrintf(5.0f32.mul_add(space, y0)) + lt, lt, 100);
        canvas_vline(c, x1 - lt, lrintf(y0), lrintf(5.0f32.mul_add(space, y0)) + lt, lt, 100);
        let fs = space * 1.75;
        const LETTERS: [&str; 3] = ["T", "A", "B"];
        for i in 0..3 {
            font_draw(
                c,
                FontFace::FixedBold,
                fs,
                x0 + lrintf(space * 0.6),
                lrintf(space.mul_add(1.6f32.mul_add(i as f32, 1.6), y0)),
                LETTERS[i],
                60,
            );
        }
    }
}

// ---- custom music layouts ----

fn clef_glyph(c: &mut Canvas, cp: u32, x: i32, y: f32, w: f32, h: f32) {
    font_draw_icon(c, FontFace::Regular, cp, Rect { x, y: lrintf(y), w: lrintf(w), h: lrintf(h) }, 110);
}

fn draw_part(c: &mut Canvas, p: MusicPart, y0: f32, space: f32, x0: i32, x1: i32, dpi: f32) {
    let lt = thick(2.0, dpi);
    let lines = part_lines(p);
    if p == PART_LYRICS {
        canvas_hline(c, x0, x1, lrintf(y0), lt, 170);
        return;
    }
    let tab = p >= PART_TAB4 && p <= PART_TAB7;
    let g: u8 = if tab { 100 } else { 90 };
    let h = space * (lines - 1) as f32;
    for l in 0..lines {
        canvas_hline(c, x0, x1, lrintf((l as f32).mul_add(space, y0)), lt, g);
    }
    canvas_vline(c, x0, lrintf(y0), lrintf(y0 + h) + lt, lt, g);
    canvas_vline(c, x1 - lt, lrintf(y0), lrintf(y0 + h) + lt, lt, g);
    let cx = x0 + space as i32;
    match p {
        PART_TREBLE => clef_glyph(c, 0xF0F70, cx, (-space).mul_add(1.6, y0), space * 3.2, space * 7.2),
        PART_BASS => clef_glyph(c, 0xF0F6F, cx, (-space).mul_add(0.2, y0), space * 3.2, space * 3.6),
        // The C clef centers on its line: the middle line (alto), the second from the top (tenor).
        PART_ALTO => clef_glyph(c, 0xF0F6E, cx, (-space).mul_add(0.2, y0), space * 4.4, space * 4.4),
        PART_TENOR => clef_glyph(c, 0xF0F6E, cx, (-space).mul_add(1.2, y0), space * 4.4, space * 4.4),
        PART_PERC => {
            // The neutral clef: two thick bars across the middle spaces.
            let bw = lrintf(space * 0.4);
            for b in 0..2 {
                canvas_fill(
                    c,
                    Rect { x: cx + lrintf(space * 0.9f32.mul_add(b as f32, 0.6)), y: lrintf(y0 + space), w: bw, h: lrintf(space * 2.0) },
                    80,
                );
            }
        }
        _ => {
            if tab {
                // "TAB" spread down the lines, shrinking to fit a 4-line tab.
                let fs = (space * 1.75).min(h / 3.0 * 1.05);
                const LETTERS: [&str; 3] = ["T", "A", "B"];
                for i in 0..3 {
                    let cy = y0 + h * (i as f32 + 0.5) / 3.0;
                    font_draw(c, FontFace::FixedBold, fs, x0 + lrintf(space * 0.6), lrintf(fs.mul_add(0.34, cy)), LETTERS[i], 60);
                }
            }
        }
    }
}

fn music_custom(c: &mut Canvas, m: &MusicLayout, dpi: f32) {
    let top = mm_to_px(MUSIC_TOP_MM, dpi);
    let bot = mm_to_px(MUSIC_BOT_MM, dpi);
    let avail = c.h as f32 - top - bot;
    let su = system_units(m);
    let mut space = avail / (m.systems as f32).mul_add(su, ((m.systems - 1) as f32) * SYSTEM_GAP);
    let mut sgap = SYSTEM_GAP;
    let maxs = mm_to_px(MAX_SPACE_MM, dpi);
    if space > maxs {
        // Few systems: keep the staff size sane and spread the systems down the page instead.
        space = maxs;
        if m.systems > 1 {
            sgap = (-(m.systems as f32)).mul_add(su, avail / space) / (m.systems - 1) as f32;
        }
    }
    let mut staffy = 0;
    for i in 0..m.n as usize {
        staffy += (m.part[i] != PART_LYRICS) as i32;
    }
    let joined = staffy > 1;
    let x0 = px_mm(if joined { 16.0 } else { 10.0 }, dpi);
    let x1 = c.w - px_mm(10.0, dpi);
    let mut y = top;
    for _s in 0..m.systems {
        let mut first = -1.0f32;
        let mut last = -1.0f32;
        let mut py = [0.0f32; MUSIC_MAX_PARTS];
        for i in 0..m.n as usize {
            y = gap_above(m, i).mul_add(space, y);
            py[i] = y;
            if m.part[i] != PART_LYRICS {
                if first < 0.0 {
                    first = y;
                }
                last = part_height(m.part[i]).mul_add(space, y);
            }
            draw_part(c, m.part[i], y, space, x0, x1, dpi);
            y = part_height(m.part[i]).mul_add(space, y);
        }
        if joined {
            // The system line joins every staff and tab of the system.
            canvas_vline(c, x0, lrintf(first), lrintf(last) + thick(2.0, dpi), thick(3.0, dpi), 70);
            // A brace for each treble + bass pair (a piano part).
            for i in 0..(m.n - 1).max(0) as usize {
                if m.part[i] != PART_TREBLE || m.part[i + 1] != PART_BASS {
                    continue;
                }
                let yt = py[i];
                let yb = 4.0f32.mul_add(space, py[i + 1]);
                let bx = x0 as f32 - mm_to_px(3.0, dpi);
                canvas_line_aa(c, bx, yt + space, bx, yb - space, thick(5.0, dpi) as f32, 60);
                canvas_line_aa(c, bx, yt + space, bx + mm_to_px(2.0, dpi), yt, thick(4.0, dpi) as f32, 60);
                canvas_line_aa(c, bx, yb - space, bx + mm_to_px(2.0, dpi), yb, thick(4.0, dpi) as f32, 60);
            }
        }
        y = sgap.mul_add(space, y);
    }
}

// Bass tab pages are custom layouts with a single tab part; the counts keep about 2 mm
// between lines, like the guitar tab page.
fn bass_tab(c: &mut Canvas, strings: i32, dpi: f32) {
    const SYSTEMS: [i32; 4] = [10, 10, 9, 8];
    let mut m = MusicLayout { n: 1, systems: SYSTEMS[clampi(strings - 4, 0, 3) as usize], ..Default::default() };
    m.part[0] = MusicPart(PART_TAB4.0 + strings - 4);
    music_custom(c, &m, dpi);
}

// 5 mm dot grid. Only the dots inside the clip are visited.
fn dots(c: &mut Canvas, dpi: f32) {
    let pitch = mm_to_px(5.0, dpi);
    let r = thick(3.0, dpi) as f32 * 0.8;
    let ox = (c.w as f32 % pitch) / 2.0 + pitch / 2.0;
    let oy = mm_to_px(10.0, dpi);
    let cl = c.clip;
    let i0 = (((cl.x as f32 - ox - r) / pitch).floor()) as i32;
    let i1 = ((((cl.x + cl.w) as f32 - ox + r) / pitch).ceil()) as i32;
    let j0 = (((cl.y as f32 - oy - r) / pitch).floor()) as i32;
    let j1 = ((((cl.y + cl.h) as f32 - oy + r) / pitch).ceil()) as i32;
    let i0 = i0.max(0);
    let j0 = j0.max(0);
    for j in j0..=j1 {
        let y = (j as f32).mul_add(pitch, oy);
        if y > c.h as f32 - mm_to_px(8.0, dpi) {
            break;
        }
        for i in i0..=i1 {
            let x = (i as f32).mul_add(pitch, ox);
            if x > c.w as f32 - pitch / 2.0 {
                break;
            }
            canvas_disc_aa(c, x, y, r, 90, 1.0);
        }
    }
}

// 5 mm graph paper, every fifth line a little stronger.
fn grid(c: &mut Canvas, dpi: f32) {
    let pitch = mm_to_px(5.0, dpi);
    let lt = thick(1.0, dpi);
    let lt5 = thick(2.0, dpi);
    let mut i = 0;
    loop {
        let x = lrintf(pitch * i as f32);
        if x >= c.w {
            break;
        }
        let h = c.h;
        canvas_vline(c, x, 0, h, if i % 5 != 0 { lt } else { lt5 }, if i % 5 != 0 { 200 } else { 170 });
        i += 1;
    }
    let mut j = 0;
    loop {
        let y = lrintf(pitch * j as f32);
        if y >= c.h {
            break;
        }
        let w = c.w;
        canvas_hline(c, 0, w, y, if j % 5 != 0 { lt } else { lt5 }, if j % 5 != 0 { 200 } else { 170 });
        j += 1;
    }
}

// Cornell notes: medium rule, a cue column on the left, a summary block at the bottom.
fn cornell(c: &mut Canvas, dpi: f32) {
    ruled(c, MEDIUM_MM, dpi, false);
    let lt = thick(3.0, dpi);
    let head = lrintf(mm_to_px(22.0, dpi));
    let sum = c.h - px_mm(52.0, dpi);
    let cue = px_mm(55.0, dpi);
    canvas_vline(c, cue, head, sum, lt, 120);
    let w = c.w;
    canvas_hline(c, 0, w, sum, lt, 120);
    let fs = mm_to_px(2.6, dpi);
    font_draw(c, FontFace::Fixed, fs, px_mm(4.0, dpi), head - px_mm(3.0, dpi), "Cues", 150);
    font_draw(c, FontFace::Fixed, fs, cue + px_mm(4.0, dpi), head - px_mm(3.0, dpi), "Notes", 150);
    font_draw(c, FontFace::Fixed, fs, px_mm(4.0, dpi), sum + px_mm(6.0, dpi), "Summary", 150);
}

// paper_render given an image paper: a caller that should have taken the page's paper from
// page_paper_render (page.h), which draws the picture. Counted for the tests, logged once.
thread_local! {
    static G_IMAGE_BLANK: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
}

/// How many times paper_render drew an image paper blank since the last call (the first is logged):
/// a caller that should have used page_paper_render. The tests hold page rendering to 0.
pub fn paper_take_image_blank_count() -> i32 {
    G_IMAGE_BLANK.with(|g| g.replace(0))
}

/// Renders the background for `region` of a full page canvas (canvas size = page size).
/// Must be a pure function of (kind, canvas w/h, dpi, pixel position) so any sub-region
/// re-render matches a full render exactly. A kind this build does not know is blank paper, and so
/// is an image paper here: page_paper_render (page.h) draws it from its notebook's picture, and is
/// what every page's paper must come from (the ink layer's erase strokes, drawn live, too).
pub fn paper_render(c: &mut Canvas, k: PaperKind, region: Rect, dpi: f32) {
    let saved = c.clip;
    canvas_set_clip(c, rect_intersect(region, saved));
    if rect_is_empty(c.clip) {
        c.clip = saved;
        return;
    }
    let clip = c.clip;
    canvas_fill(c, clip, 255);
    if paper_is_image(k) {
        // (g_image_blank++ == 0: counted every time, logged the first)
        let n = G_IMAGE_BLANK.with(|g| g.replace(g.get() + 1));
        if n == 0 {
            qlog!("paper: an image paper drawn as blank paper, without its notebook's picture (page_paper_render draws it)");
        }
    }
    match k {
        // Spread ~30% apart so the three are clearly different on screen (the stationery values,
        // 6.35 / 7.14 / 8.73 mm, looked nearly identical between narrow and medium).
        PAPER_NARROW => ruled(c, NARROW_MM, dpi, true),
        PAPER_MEDIUM => ruled(c, MEDIUM_MM, dpi, true),
        PAPER_WIDE => ruled(c, WIDE_MM, dpi, true),
        PAPER_NARROW_NM => ruled(c, NARROW_MM, dpi, false),
        PAPER_MEDIUM_NM => ruled(c, MEDIUM_MM, dpi, false),
        PAPER_WIDE_NM => ruled(c, WIDE_MM, dpi, false),
        PAPER_MUSIC => music(c, dpi),
        PAPER_GRAND => grand(c, dpi),
        PAPER_TAB => tab(c, dpi),
        PAPER_DOTS => dots(c, dpi),
        PAPER_GRID => grid(c, dpi),
        PAPER_CORNELL => cornell(c, dpi),
        PAPER_BASS4 => bass_tab(c, 4, dpi),
        PAPER_BASS5 => bass_tab(c, 5, dpi),
        PAPER_BASS6 => bass_tab(c, 6, dpi),
        PAPER_BASS7 => bass_tab(c, 7, dpi),
        _ => {
            // anything else (a newer version's paper) stays blank
            if let Some(m) = decode(k) {
                music_custom(c, &m, dpi);
            }
        }
    }
    c.clip = saved;
    // A whole page of paper is 10-40 ms on the Kindle, and the Paper sheet draws a dozen: the pens
    // are read after each (work_pump, common.h).
    work_pump();
}

#[cfg(test)]
mod tests {
    use super::*;

    // (the language is thread state: a test that sets one ends in English)
    #[test]
    fn the_pickers_offer_every_fixed_kind_once() {
        let mut seen = [false; PAPER_COUNT as usize];
        for i in 0..PAPER_COUNT {
            let k = paper_at(i);
            assert!(k.0 < PAPER_COUNT as u32 && !seen[k.0 as usize]);
            seen[k.0 as usize] = true;
            assert!(paper_is_known(k) && paper_sanitize(k.0, PAPER_BLANK) == k);
        }
        assert_eq!((paper_at(-5), paper_at(99)), (PAPER_NARROW, PAPER_BLANK));
        assert_eq!(PAPER_PICKER_COUNT, 18);
    }

    #[test]
    fn names_are_english_identifiers() {
        assert_eq!(paper_name(PAPER_NARROW), "Narrow rule");
        assert_eq!(paper_name(PAPER_WIDE_NM), "Wide, no margin");
        assert_eq!(paper_name(PAPER_BASS7), "7-string bass");
        assert_eq!(paper_name(music_default()), "Custom music");
        assert_eq!(paper_name(paper_image(PAPER_ROLE_TEMPLATE, 7)), "Template");
        assert_eq!(paper_name(paper_image(PAPER_ROLE_DOCUMENT, 7)), "Imported page");
        assert_eq!(paper_name(PaperKind(0x60000001)), "Newer paper");
        assert_eq!(paper_name(PaperKind(PAPER_COUNT as u32)), "Newer paper");
        assert_eq!(music_part_name(PART_TAB6), "Tab, 6 lines");
        assert_eq!(music_part_name(PART_NONE), "");
        // The labels follow the language in use; the names do not.
        i18n_set("es");
        assert_eq!((paper_label(PAPER_BLANK), paper_name(PAPER_BLANK)), ("En blanco", "Blank"));
        assert_eq!(part_label(PART_TREBLE), c_("music part", "Treble staff"));
        assert_ne!(part_label(PART_TREBLE), "Treble staff");
        i18n_set("en");
        assert_eq!(paper_label(PAPER_BLANK), "Blank");
    }

    #[test]
    fn custom_layouts_round_trip() {
        let d = music_default();
        assert_eq!(d, PaperKind(0x45000091));
        let mut m = MusicLayout::default();
        assert!(paper_is_custom(d) && music_decode(d, &mut m));
        assert_eq!((m.systems, m.n, m.part[0], m.part[1], m.part[2]), (5, 2, PART_TREBLE, PART_TAB6, PART_NONE));
        assert_eq!(music_encode(&m), d);
        // Out of range counts are clamped on the way in.
        m.systems = 99;
        assert_eq!(music_encode(&m).0 >> 24, 0x4c);
        m.systems = -3;
        assert_eq!(music_encode(&m).0 >> 24, 0x41);
        // Malformed layouts decode to nothing, and leave what the caller passed.
        let mut keep = MusicLayout { systems: 7, ..Default::default() };
        for bad in [0x40000000, 0x41000000, 0x40000001, 0x4d000001, 0x410000c1, 0x41000fff, 0x4fffffff, 0x50000000, 3] {
            assert!(!music_decode(PaperKind(bad), &mut keep), "{bad:x}");
            assert_eq!(keep.systems, 7);
            assert!(!paper_is_known(PaperKind(bad)) || bad == 0x50000000 || bad == 3);
        }
        assert!(paper_is_custom(PaperKind(0x40000000)) && !paper_is_custom(PaperKind(0x50000000)));
        assert_eq!(paper_sanitize(0x410000c1, PAPER_MEDIUM), PAPER_MEDIUM);
        assert!(music_decode(PaperKind(0x41000c01), &mut keep)); // (a part after a gap is not read)
        assert_eq!(paper_sanitize(0x41000009, PAPER_MEDIUM), PaperKind(0x41000009));
        assert_eq!(paper_sanitize(0x50000001, PAPER_MEDIUM), PAPER_MEDIUM); // an image needs its notebook
    }

    #[test]
    fn image_papers() {
        let t = paper_image(PAPER_ROLE_TEMPLATE, 0x123456);
        let d = paper_image(PAPER_ROLE_DOCUMENT, 0xffabcdef);
        assert_eq!((t.0, d.0), (0x50123456, 0x51abcdef));
        assert!(paper_is_image(t) && !paper_is_document(t) && paper_is_image(d) && paper_is_document(d));
        assert_eq!((paper_image_id(t), paper_image_id(d)), (0x123456, 0xabcdef));
        // Roles 2 and 3, and the colour bits, are a newer version's.
        for newer in [0x52000000, 0x53000000, 0x54000000, 0x58000000] {
            assert!(!paper_is_image(PaperKind(newer)) && !paper_is_known(PaperKind(newer)), "{newer:x}");
        }
        assert_eq!(paper_image(5, 7).0, 0x51000007); // (the role is two bits)
    }

    #[test]
    fn how_many_systems_fit() {
        let mut m = MusicLayout::default();
        assert!(music_decode(music_default(), &mut m));
        assert_eq!((music_max_systems(&m, 1000, 300.0), music_max_systems(&m, 2480, 300.0), music_max_systems(&m, 3500, 300.0)), (2, 6, 9));
        // More staff spaces fit at a lower resolution.
        assert_eq!((music_max_systems(&m, 1000, 226.0), music_max_systems(&m, 3500, 226.0)), (3, 12));
    }

    #[test]
    fn an_empty_region_draws_nothing() {
        paper_take_image_blank_count();
        // A region with no area draws nothing, counts nothing and leaves the clip as it was (drawing
        // itself is checked against the C, call for call, where the canvas is implemented).
        let mut c = Canvas { px: vec![], w: 100, h: 100, stride: 100, clip: Rect::new(0, 0, 100, 100) };
        paper_render(&mut c, paper_image(PAPER_ROLE_TEMPLATE, 1), Rect::new(5, 5, 0, 0), 300.0);
        assert_eq!(paper_take_image_blank_count(), 0);
        assert_eq!(c.clip, Rect::new(0, 0, 100, 100));
    }
}
