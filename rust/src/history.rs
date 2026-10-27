//! history.h: per-page undo/redo. Strokes removed from the page are owned by the history.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActKind {
    #[default]
    Add = 0,  // ACT_ADD
    Erase = 1,  // ACT_ERASE
    Paper = 2,  // ACT_PAPER
    Clear = 3,  // ACT_CLEAR
}

#[derive(Debug, Default)]
pub struct Action {
    pub kind: ActKind,
    /// page indices, ascending (ADD: 1 entry; ERASE/CLEAR: removed ones). Their count is the C's count.
    pub indices: Vec<i32>,
    /// The C's strokes[], by identity (stroke_id), always.
    pub ids: Vec<StrokeId>,
    /// The strokes themselves while this action owns them (they are NOT on the page): the C's
    /// strokes[] with owns set. Empty while they are on the page.
    pub owned: Vec<Box<Stroke>>,
    /// whether strokes[] are currently owned by this action
    pub owns: bool,
    pub paper_old: PaperKind,
    pub paper_new: PaperKind,
}

#[derive(Debug, Default)]
pub struct History {
    /// the C's undo, nundo, capundo
    pub undo: Vec<Action>,
    /// the C's redo, nredo, capredo
    pub redo: Vec<Action>,
    /// max undo depth (oldest dropped), default 500
    pub limit: i32,
}

pub fn history_init(h: &mut History) {
    todo!()
}
/// frees owned strokes
pub fn history_free(h: &mut History) {
    todo!()
}
pub fn history_clear(h: &mut History) {
    todo!()
}

/// Record an action that has ALREADY been applied to the page.
/// s stays owned by page
pub fn history_push_add(h: &mut History, index: i32, s: &Stroke) {
    todo!()
}
/// takes ownership of removed
pub fn history_push_erase(h: &mut History, indices: &[i32], removed: Vec<Box<Stroke>>) {
    todo!()
}
pub fn history_push_paper(h: &mut History, old_k: PaperKind, new_k: PaperKind) {
    todo!()
}
/// call BEFORE clearing: it takes all strokes out
pub fn history_push_clear(h: &mut History, p: &mut Page) {
    todo!()
}

pub fn history_can_undo(h: &History) -> bool {
    todo!()
}
pub fn history_can_redo(h: &History) -> bool {
    todo!()
}
/// Apply undo/redo to the page. *dirty receives the page area that must be re-rendered
/// (the full page for paper changes). Returns false if nothing to do.
pub fn history_undo(h: &mut History, p: &mut Page, dirty: &mut Rect) -> bool {
    todo!()
}
pub fn history_redo(h: &mut History, p: &mut Page, dirty: &mut Rect) -> bool {
    todo!()
}
