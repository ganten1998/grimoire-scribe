//! musicbuild.h: the "Custom music" sheet. Stack staves (with clefs), tabs and lyric lines
//! into a system, choose how many systems go on a page, see the page as it will look.
//! A modal hosted by the editor (paper for this page) and the library (new notebook).

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MbResult {
    #[default]
    None = 0,  // MB_NONE
    Changed = 1,  // MB_CHANGED
    Cancel = 2,  // MB_CANCEL
    Done = 3,  // MB_DONE
}

#[derive(Clone, Debug, Default)]
pub struct MusicBuild {
    pub open: bool,
    pub rect: Rect,
    pub hits: HitList,
    pub m: MusicLayout,
    pub press: Press,
    /// page preview at 1/4 size, rebuilt when the layout changes
    pub prev: Canvas,
    pub prev_k: PaperKind,
    /// the page the layout is for: how many systems fit, the preview's size
    pub page_w: i32,
    pub page_h: i32,
}

/// Starts from `start` if it is a custom layout, else from the last one built (settings), for a
/// new notebook's page (the device's page size).
pub fn musicbuild_open(b: &mut MusicBuild, a: &mut App, start: PaperKind) {
    todo!()
}
/// The same for a page of page_w x page_h (an open notebook's, which may be another screen's size).
pub fn musicbuild_open_page(b: &mut MusicBuild, a: &mut App, start: PaperKind, page_w: i32, page_h: i32) {
    todo!()
}
/// draws into a->screen, returns its rect
pub fn musicbuild_draw(b: &mut MusicBuild, a: &mut App) -> Rect {
    todo!()
}
/// Pointer phase 0 down / 1 move / 2 up. MB_DONE: read musicbuild_result (already saved as
/// the last layout); MB_DONE and MB_CANCEL close the sheet (the host recomposes b->rect).
pub fn musicbuild_pointer(b: &mut MusicBuild, a: &mut App, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> MbResult {
    todo!()
}
pub fn musicbuild_result(b: &MusicBuild) -> PaperKind {
    todo!()
}
pub fn musicbuild_free(b: &mut MusicBuild) {
    todo!()
}
