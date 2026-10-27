//! tplsheet.h: the Templates sheet, behind the "More papers..." tile of the editor's Paper sheet and
//! of New notebook: the pictures in grimoire/templates/ (templates.h) as tiles, a page of them at a
//! time, to pick one as paper. A tile's preview is read only while the tile is on screen, and a
//! tile whose preview the background job has not made yet shows a placeholder until it has (one
//! the job finds it cannot use keeps its place, crossed out, until the sheet opens again: no tile
//! moves under the pen). Also the layout the paper pickers share: their grid of tiles and the
//! labels under them.
//!
//! A modal hosted by the editor (in the Paper sheet's place) and the library (over New notebook).
//! It never shows or logs more of a template than its name on screen: names are the owner's.

use crate::prelude::*;

/// The paper pickers' tiles: the fixed papers in paper_at() order, then Custom music..., then More
/// papers... (this sheet); the editor's Paper sheet also has Photos... (this sheet on the photos).
pub const PAPER_SHEET_TILES: i32 = PAPER_PICKER_COUNT + 1;
pub const PAPER_SHEET_EDITOR_TILES: i32 = PAPER_SHEET_TILES + 1;

/// A grid of n tiles (each 3:4 with label_h px for its label under it, gap px apart) in a box
/// avail_w wide and avail_h tall: the number of columns, at least min_cols, that makes the tiles
/// biggest (no wider than max_w), and of those that make them as big, the one that leaves the
/// fewest places empty on the last row. The paper pickers take at least 6, as they always had:
/// upright 6 (the papers where they were, More papers... on a row of its own), sideways 7, which
/// keeps them as big as 6 were before More papers... came.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaperGrid {
    pub cols: i32,
    pub rows: i32,
    /// a tile
    pub tw: i32,
    pub th: i32,
    /// a row: a tile and its label
    pub cell_h: i32,
}
pub fn paper_grid(n: i32, avail_w: i32, avail_h: i32, gap: i32, label_h: i32, max_w: i32, min_cols: i32) -> PaperGrid {
    todo!()
}
pub const PAPER_GRID_MIN_COLS: i32 = 6;
/// The editor's Paper sheet on this screen, and the grid of its PAPER_SHEET_EDITOR_TILES tiles (the
/// Templates sheet takes the same place and tiles).
pub fn paper_sheet_rect(a: &App, g: &mut PaperGrid) -> Rect {
    todo!()
}
/// A paper tile's label, under the tile (tile: its rect; gap: the grid's): on one line, centred,
/// or on two lines no wider than the tile when it is wider than the tile (a break after a comma
/// preferred: "Narrow," over "no margin"), so that labels side by side never run into each other.
/// In bold for the selected tile, on as many lines as it has in regular.
pub fn paper_tile_label(c: &mut Canvas, tile: Rect, gap: i32, label: &str, selected: bool) {
    todo!()
}
/// The More papers... tile's picture when it has none to show (the paper is no picture): a blank
/// page with the sign for more.
pub fn paper_tile_more(c: &mut Canvas, tile: Rect) {
    todo!()
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TsResult {
    #[default]
    None = 0,  // TS_NONE
    Changed = 1,  // TS_CHANGED
    Picked = 2,  // TS_PICKED
    Back = 3,  // TS_BACK
    Cancel = 4,  // TS_CANCEL
}
/// The hits it registers as it draws (tests find its buttons by them): a tile (arg: the template, an
/// index into shown), the page buttons, Back.
pub const TS_HIT_TILE: i32 = 1;
pub const TS_HIT_PREV: i32 = 2;
pub const TS_HIT_NEXT: i32 = 3;
pub const TS_HIT_BACK: i32 = 4;

#[derive(Clone, Debug, Default)]
pub struct TplSheet {
    pub open: bool,
    /// on screen, as last drawn
    pub rect: Rect,
    pub hits: HitList,
    pub press: Press,
    /// every template of the folder (template_scan), bad ones too
    pub t: Vec<Template>,
    /// its tiles, indices into t: the ones not bad as it opened (one the previews job finds bad
    /// since keeps its tile, offered no more). The C's shown and n.
    pub shown: Vec<i32>,
    /// a template to keep in view, an index into shown: the first of the
    /// page last paged to (the selected one's, at first)
    pub anchor: i32,
    /// as last laid out
    pub page: i32,
    pub pages: i32,
    pub per: i32,
    /// the one that is the paper now (index into shown), or -1
    pub selected: i32,
    /// after TS_PICKED: the one tapped (index into shown)
    pub picked: i32,
    /// some offered template has no preview yet
    pub pending: bool,
    /// bgimage_generation when the list was last brought up to date
    pub gen: u32,
    /// when to look again for previews made since
    pub next_check: u32,
    /// the photos (templates.h), as a page's own picture: tplsheet_open_photos
    pub photos: bool,
    /// What the paper now is: in the editor a picture of nb's (current), in New notebook a file.
    /// (The C keeps a pointer to the editor's notebook; this is a copy made at tplsheet_open: the
    /// notebook does not change while the sheet is open. None in New notebook.)
    pub nb: Option<Box<NotebookInfo>>,
    pub ix: BgIndex,
    pub current: PaperKind,
    pub current_path: FixedStr<TEMPLATE_PATH_MAX>,
}

/// Opens it on the templates as they are now (the previews job queued for those that have none):
/// on the page with the template that is the paper now, which is selected. In the editor that is
/// the open page's paper `current` (an image paper of notebook nb's); in New notebook (nb NULL) the
/// file current_path (NULL or "" for none).
pub fn tplsheet_open(s: &mut TplSheet, a: &mut App, nb: Option<&NotebookInfo>, current: PaperKind, current_path: Option<&str>) {
    todo!()
}
/// The same on the photos in grimoire/photos/ (the editor's Photos...): the one that is the open page's
/// paper now is selected; the host installs the one picked with photo_install.
pub fn tplsheet_open_photos(s: &mut TplSheet, a: &mut App, nb: &NotebookInfo, current: PaperKind) {
    todo!()
}
/// draws into a->screen, returns its rect
pub fn tplsheet_draw(s: &mut TplSheet, a: &mut App) -> Rect {
    todo!()
}
/// Pointer phase 0 down / 1 move / 2 up. TS_PICKED (a template tapped), TS_BACK (its Back button)
/// and TS_CANCEL (a tap outside it) close it: the host recomposes *dirty, and after TS_PICKED reads
/// tplsheet_path (and tplsheet_preview), then calls tplsheet_close. TS_CHANGED: another page of
/// tiles, drawn; refresh *dirty.
pub fn tplsheet_pointer(s: &mut TplSheet, a: &mut App, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> TsResult {
    todo!()
}
/// From the host's tick while it is open: previews made since (looked for every 1.5 s while a
/// background job runs, and when one ends) are drawn in their tiles. True with *dirty to refresh.
pub fn tplsheet_tick(s: &mut TplSheet, a: &mut App, now: u32, dirty: &mut Rect) -> bool {
    todo!()
}
/// After TS_PICKED, until tplsheet_close: the picked template's file, and its preview.
pub fn tplsheet_path(s: &TplSheet) -> &str {
    todo!()
}
pub fn tplsheet_preview(s: &mut TplSheet) -> Option<&Canvas> {
    todo!()
}
/// Closes it and lets go of its list (also when it is closed already).
pub fn tplsheet_close(s: &mut TplSheet) {
    todo!()
}
/// The headless runner's "templates.*" keys (key without the prefix; docs/SIM.md): count (the
/// templates it offers), ready (how many of those have their previews), page and pages (of tiles,
/// from 1), selected (the selected one's name). Empty values while it is closed. 0, -1 for a key
/// it does not know.
pub fn tplsheet_probe(s: &TplSheet, key: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
