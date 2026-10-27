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
    todo!()
}
/// an image paper of the document page role
pub fn paper_is_document(k: PaperKind) -> bool {
    todo!()
}
/// id: the low 24 bits are used
pub fn paper_image(role: i32, id: u32) -> PaperKind {
    todo!()
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
    todo!()
}
/// A fixed kind, a well-formed custom layout or a well-formed image paper. Anything else is a paper
/// from a newer version: pages keep it, draw it blank and call it "Newer paper".
pub fn paper_is_known(k: PaperKind) -> bool {
    todo!()
}
/// A kind the pickers offer (a fixed kind or a well-formed custom layout) as it is; anything else
/// becomes `fallback`, an image paper too: it needs its notebook's picture (a new notebook, the
/// music layout setting).
pub fn paper_sanitize(v: u32, fallback: PaperKind) -> PaperKind {
    todo!()
}
pub fn music_encode(m: &MusicLayout) -> PaperKind {
    todo!()
}
/// false if k is not a custom layout
pub fn music_decode(k: PaperKind, m: &mut MusicLayout) -> bool {
    todo!()
}
/// "Treble staff", "Tab, 6 lines", ...
pub fn music_part_name(p: MusicPart) -> &'static str {
    todo!()
}
/// the same in the language in use, for the screens
pub fn part_label(p: MusicPart) -> &'static str {
    todo!()
}
/// The most systems that fit on a page page_h px tall (at dpi) with staff spaces still big enough
/// to write in. A layout keeps its systems on a page of another height: paper_render spaces them
/// to fit (with smaller staff spaces on a shorter page), it never drops one.
pub fn music_max_systems(m: &MusicLayout, page_h: i32, dpi: f32) -> i32 {
    todo!()
}
/// treble staff over 6-line tab, 5 systems
pub fn music_default() -> PaperKind {
    todo!()
}

/// Pickers offer the fixed kinds in paper_at() order, then one "Custom music" tile.
pub const PAPER_PICKER_COUNT: i32 = PAPER_COUNT + 1;

/// e.g. "Narrow rule", "Narrow, no margin", "Template", "Imported page", "Newer paper"
pub fn paper_name(k: PaperKind) -> &'static str {
    todo!()
}
/// paper_name in the language in use: what the screens show (paper_name stays English, an
/// identifier for /api/list.json).
pub fn paper_label(k: PaperKind) -> &'static str {
    todo!()
}
/// The order papers are offered in pickers (index 0..PAPER_COUNT-1 -> kind).
pub fn paper_at(index: i32) -> PaperKind {
    todo!()
}

/// Renders the background for `region` of a full page canvas (canvas size = page size).
/// Must be a pure function of (kind, canvas w/h, dpi, pixel position) so any sub-region
/// re-render matches a full render exactly. A kind this build does not know is blank paper, and so
/// is an image paper here: page_paper_render (page.h) draws it from its notebook's picture, and is
/// what every page's paper must come from (the ink layer's erase strokes, drawn live, too).
pub fn paper_render(c: &mut Canvas, k: PaperKind, region: Rect, dpi: f32) {
    todo!()
}
/// How many times paper_render drew an image paper blank since the last call (the first is logged):
/// a caller that should have used page_paper_render. The tests hold page rendering to 0.
pub fn paper_take_image_blank_count() -> i32 {
    todo!()
}
