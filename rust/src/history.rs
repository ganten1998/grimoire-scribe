//! history.h: per-page undo/redo. Strokes removed from the page are owned by the history.
//!
//! (history.c) Ownership: an action owns its strokes exactly when they are NOT on the page.
//!   ADD    on page after push/redo (not owned); undo removes it (owned).
//!   ERASE  removed on push/redo (owned); undo re-inserts them (not owned).
//!   CLEAR  same as ERASE for every stroke.
//! Strokes are located by pointer on undo/redo; the stored index is the insert position. In Rust
//! the pointer is the boxed stroke's address (page.rs: stroke_id), `ids`; the strokes themselves
//! are in `owned` while the action owns them.
//!   LINK   (phase 2) a step of link changes on the page's links (links.ini, not the page file): it
//!          moves between the stacks like any other, so ink and links keep one order, and leaves
//!          the page untouched; the editor applies its records to the Nav (history_moved_link).

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActKind {
    #[default]
    Add = 0,  // ACT_ADD
    Erase = 1,  // ACT_ERASE
    Paper = 2,  // ACT_PAPER
    Clear = 3,  // ACT_CLEAR
    /// A step of link edits (`Action.link`): the page itself is untouched.
    Link = 4,
}

/// One link as an undo step keeps it: what nav_link_put needs to restore it exactly (its page is the
/// step's). The editor converts (link_rec, rec_link), so this file stays free of the Nav.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinkRec {
    pub seed: u32,
    pub target: FixedStr<NAV_TARGET_LEN>,
    pub area: Rect,
    pub has_area: bool,
}

/// One undo step's link changes on page `pid`: each record is (before, after) for one seed, None for
/// no link. A group from the bar (xN) is one step of N records.
#[derive(Clone, Debug, Default)]
pub struct LinkEdit {
    pub pid: u32,
    pub records: Vec<(Option<LinkRec>, Option<LinkRec>)>,
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
    /// ActKind::Link: its records (None for every other kind). Boxed: most actions are ink.
    pub link: Option<Box<LinkEdit>>,
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
    *h = History::default();
    h.limit = 500;
}

// (action_free: an action that owns its strokes drops them with it, the Vec's drop)

fn clear_redo(h: &mut History) {
    h.redo.clear();
}

pub fn history_clear(h: &mut History) {
    h.undo.clear();
    clear_redo(h);
}

/// frees owned strokes
pub fn history_free(h: &mut History) {
    history_clear(h);
    *h = History::default();
}

fn push_undo(h: &mut History, a: Action) {
    clear_redo(h);
    if h.limit > 0 && h.undo.len() as i32 >= h.limit {
        h.undo.remove(0);
    }
    h.undo.push(a);
}

/// Record an action that has ALREADY been applied to the page.
/// s stays owned by page
pub fn history_push_add(h: &mut History, index: i32, s: &Stroke) {
    let a = Action { kind: ActKind::Add, indices: vec![index], ids: vec![stroke_id(s)], ..Default::default() };
    push_undo(h, a);
}

/// takes ownership of removed
pub fn history_push_erase(h: &mut History, indices: &[i32], removed: Vec<Box<Stroke>>) {
    if indices.is_empty() {
        return;
    }
    debug_assert_eq!(indices.len(), removed.len(), "one index for each stroke removed");
    let a = Action {
        kind: ActKind::Erase,
        indices: indices.to_vec(),
        ids: removed.iter().map(|s| stroke_id(s)).collect(),
        owned: removed,
        owns: true,
        ..Default::default()
    };
    push_undo(h, a);
}

pub fn history_push_paper(h: &mut History, old_k: PaperKind, new_k: PaperKind) {
    if old_k == new_k {
        return;
    }
    push_undo(h, Action { kind: ActKind::Paper, paper_old: old_k, paper_new: new_k, ..Default::default() });
}

/// call BEFORE clearing: it takes all strokes out
pub fn history_push_clear(h: &mut History, p: &mut Page) {
    if p.strokes.is_empty() {
        return;
    }
    // strokes now belong to the action
    let owned = std::mem::take(&mut p.strokes);
    let a = Action {
        kind: ActKind::Clear,
        indices: (0..owned.len() as i32).collect(),
        ids: owned.iter().map(|s| stroke_id(s)).collect(),
        owned,
        owns: true,
        ..Default::default()
    };
    push_undo(h, a);
}

/// Records a link step ALREADY applied to the Nav and saved (push only after nav_save_links returned
/// 0). No records: no step. Like every push it clears redo, and the limit drops the oldest step,
/// a link step's records with it.
pub fn history_push_link(h: &mut History, e: LinkEdit) {
    if e.records.is_empty() {
        return;
    }
    push_undo(h, Action { kind: ActKind::Link, link: Some(Box::new(e)), ..Default::default() });
}

/// The kind of the step an undo would apply, None for none (the editor applies a Link step's records).
pub fn history_undo_kind(h: &History) -> Option<ActKind> {
    h.undo.last().map(|a| a.kind)
}
/// The kind of the step a redo would apply, None for none.
pub fn history_redo_kind(h: &History) -> Option<ActKind> {
    h.redo.last().map(|a| a.kind)
}

/// The link records of the step history_undo (`undone`) or history_redo just moved: the top of the
/// redo stack after an undo, of the undo stack after a redo. None if that step is not a Link step.
pub fn history_moved_link(h: &History, undone: bool) -> Option<&LinkEdit> {
    let top = if undone { h.redo.last() } else { h.undo.last() };
    top.and_then(|a| a.link.as_deref())
}

/// Drops the step on top of the redo stack: a link step an undo moved whose save then failed (the
/// links are back to the file, so the step no longer describes them).
pub fn history_drop_last_redo(h: &mut History) {
    h.redo.pop();
}
/// Drops the step on top of the undo stack (a redone link step whose save failed).
pub fn history_drop_last_undo(h: &mut History) {
    h.undo.pop();
}

pub fn history_can_undo(h: &History) -> bool {
    !h.undo.is_empty()
}
pub fn history_can_redo(h: &History) -> bool {
    !h.redo.is_empty()
}

fn find_stroke(p: &Page, id: StrokeId, hint: i32) -> i32 {
    if hint >= 0 && (hint as usize) < p.strokes.len() && stroke_id(&p.strokes[hint as usize]) == id {
        return hint;
    }
    for i in (0..p.strokes.len()).rev() {
        if stroke_id(&p.strokes[i]) == id {
            return i as i32;
        }
    }
    -1
}

// The boxes of the action's strokes, wherever they are: in the action, or on the page (the C reads
// them through the pointers, which are good either way). A stroke that was dropped (id 0: put_back's
// insert failed and freed it) or is nowhere is left out.
fn bbox_of(a: &Action, p: &Page) -> Rect {
    let mut r = RECT_EMPTY;
    if a.owns {
        let mut owned = a.owned.iter();
        for i in 0..a.ids.len() {
            if a.ids[i] != 0 {
                if let Some(s) = owned.next() {
                    r = rect_union(r, s.bbox);
                }
            }
        }
        return r;
    }
    for i in 0..a.ids.len() {
        if a.ids[i] == 0 {
            continue;
        }
        let at = find_stroke(p, a.ids[i], a.indices[i]);
        if at >= 0 {
            r = rect_union(r, p.strokes[at as usize].bbox);
        }
    }
    r
}

// Takes the action's strokes off the page (becomes owner).
fn take_off(a: &mut Action, p: &mut Page) {
    // (back to front, so each index is still good; `owned` is in action order, so the strokes
    // taken are reversed at the end, and a stroke not found on the page gets no place in it)
    let mut taken: Vec<Box<Stroke>> = Vec::with_capacity(a.ids.len());
    for i in (0..a.ids.len()).rev() {
        let at = find_stroke(p, a.ids[i], a.indices[i]);
        if at >= 0 {
            taken.push(page_remove(p, at));
        } else {
            a.ids[i] = 0;
        }
    }
    taken.reverse();
    a.owned = taken;
    a.owns = true;
}

// Puts the action's strokes back at their indices (page becomes owner).
fn put_back(a: &mut Action, p: &mut Page) {
    let mut owned = std::mem::take(&mut a.owned).into_iter();
    for i in 0..a.ids.len() {
        if a.ids[i] == 0 {
            continue;
        }
        let Some(s) = owned.next() else { break };
        let before = p.strokes.len();
        page_insert(p, a.indices[i], s);
        if p.strokes.len() == before {
            a.ids[i] = 0; // insert failed and freed it
        }
    }
    a.owns = false;
}

fn apply(a: &mut Action, p: &mut Page, dirty: &mut Rect, undo: bool) -> bool {
    let full = Rect { x: 0, y: 0, w: p.w, h: p.h };
    match a.kind {
        ActKind::Add => {
            *dirty = bbox_of(a, p);
            if undo {
                take_off(a, p);
            } else {
                put_back(a, p);
            }
        }
        ActKind::Erase => {
            *dirty = bbox_of(a, p);
            if undo {
                put_back(a, p);
            } else {
                take_off(a, p);
            }
        }
        ActKind::Clear => {
            *dirty = full;
            if undo {
                put_back(a, p);
            } else {
                take_off(a, p);
            }
        }
        ActKind::Paper => {
            *dirty = full;
            p.paper = if undo { a.paper_old } else { a.paper_new };
        }
        ActKind::Link => {
            // (the page is untouched: the editor applies the records to the Nav)
            *dirty = RECT_EMPTY;
        }
    }
    true
}

/// Apply undo/redo to the page. *dirty receives the page area that must be re-rendered
/// (the full page for paper changes). Returns false if nothing to do.
pub fn history_undo(h: &mut History, p: &mut Page, dirty: &mut Rect) -> bool {
    let mut d = RECT_EMPTY;
    let Some(mut a) = h.undo.pop() else { return false };
    apply(&mut a, p, &mut d, true);
    h.redo.push(a);
    *dirty = d;
    true
}
pub fn history_redo(h: &mut History, p: &mut Page, dirty: &mut Rect) -> bool {
    let mut d = RECT_EMPTY;
    let Some(mut a) = h.redo.pop() else { return false };
    apply(&mut a, p, &mut d, false);
    h.undo.push(a);
    *dirty = d;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // A stroke of one point at (x, y).
    fn dot(x: f32, y: f32) -> Box<Stroke> {
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_BALLPOINT, &mut prm);
        let mut s = stroke_new(TOOL_BALLPOINT, &prm, 1);
        stroke_append(&mut s, &PenSample { x, y, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: 0 }, 300.0);
        s
    }
    fn ids(p: &Page) -> Vec<StrokeId> {
        p.strokes.iter().map(|s| stroke_id(s)).collect()
    }

    // An action owns its strokes exactly when they are not on the page, and the dirty rect is
    // the strokes' own boxes (history.c's rule, header of this file).
    #[test]
    fn ownership_follows_the_page() {
        let mut p = page_new(1000, 1000, PAPER_BLANK);
        let mut h = History::default();
        history_init(&mut h);
        assert_eq!(h.limit, 500);
        let (a, b, c) = (dot(100.0, 100.0), dot(300.0, 300.0), dot(500.0, 500.0));
        let (ba, bb, bc) = (a.bbox, b.bbox, c.bbox);
        let before = vec![stroke_id(&a), stroke_id(&b), stroke_id(&c)];
        for (i, s) in [a, b, c].into_iter().enumerate() {
            history_push_add(&mut h, i as i32, &s);
            page_add(&mut p, s);
        }
        let mut d = RECT_EMPTY;
        // undo the add of c: it leaves the page, the action owns it, the dirty rect is its box
        assert!(history_undo(&mut h, &mut p, &mut d));
        assert_eq!(d, bc);
        assert_eq!(ids(&p), before[..2].to_vec());
        assert!(h.redo[0].owns && h.redo[0].owned.len() == 1 && stroke_id(&h.redo[0].owned[0]) == before[2]);
        assert!(history_redo(&mut h, &mut p, &mut d));
        assert_eq!(d, bc);
        assert_eq!(ids(&p), before);
        assert!(!h.undo[2].owns && h.undo[2].owned.is_empty());
        // erase a and c in one gesture; b moves to index 0 and a stroke added at the front makes
        // every stored index stale: undo finds strokes by identity, puts them back at their indices
        let rm_c = page_remove(&mut p, 2);
        let rm_a = page_remove(&mut p, 0);
        history_push_erase(&mut h, &[0, 2], vec![rm_a, rm_c]);
        assert!(h.undo[3].owns && h.undo[3].owned.len() == 2 && h.redo.is_empty());
        assert!(history_undo(&mut h, &mut p, &mut d));
        assert_eq!(d, rect_union(ba, bc));
        assert_eq!(ids(&p), before);
        assert!(!h.redo[0].owns && h.redo[0].owned.is_empty());
        let front = dot(700.0, 700.0);
        let front_id = stroke_id(&front);
        page_insert(&mut p, 0, front);
        assert!(history_redo(&mut h, &mut p, &mut d));
        assert_eq!(d, rect_union(ba, bc));
        assert_eq!(ids(&p), vec![front_id, before[1]]);
        assert!(h.undo[3].owns);
        // a clear takes every stroke, and gives them back in order
        history_push_clear(&mut h, &mut p);
        assert!(p.strokes.is_empty());
        assert!(history_undo(&mut h, &mut p, &mut d));
        assert_eq!(d, Rect { x: 0, y: 0, w: 1000, h: 1000 });
        assert_eq!(ids(&p), vec![front_id, before[1]]);
        assert_eq!(bb, p.strokes[1].bbox);
        // a paper change; no change is no step
        let n = h.undo.len();
        history_push_paper(&mut h, PAPER_BLANK, PAPER_BLANK);
        assert_eq!(h.undo.len(), n);
        history_push_paper(&mut h, PAPER_BLANK, PAPER_GRID);
        p.paper = PAPER_GRID;
        assert!(history_undo(&mut h, &mut p, &mut d) && p.paper == PAPER_BLANK && d.w == 1000);
        assert!(history_redo(&mut h, &mut p, &mut d) && p.paper == PAPER_GRID);
        assert!(!history_redo(&mut h, &mut p, &mut d));
        // the limit drops the oldest (and frees what it owned), one at each push; an action clears redo
        history_clear(&mut h);
        h.limit = 2;
        for i in 0..5 {
            let s = dot(10.0 * i as f32, 0.0);
            history_push_add(&mut h, 0, &s);
            page_add(&mut p, s);
        }
        assert_eq!(h.undo.len(), 2);
        assert!(history_undo(&mut h, &mut p, &mut d));
        assert!(history_can_redo(&h));
        history_push_paper(&mut h, PAPER_GRID, PAPER_DOTS);
        assert!(!history_can_redo(&h) && history_can_undo(&h));
        history_free(&mut h);
        assert!(h.undo.is_empty() && h.redo.is_empty());
        page_free(p);
    }
}
