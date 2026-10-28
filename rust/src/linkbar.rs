//! The Links bar (Rust only, the Index's phase 1): the links on the open page as one row per place they
//! go (Index entry, "x2" when two go to one), numbered in reading order; peek at one without leaving the
//! page, stay there with a tap on the page, or go back. An editor-hosted sheet like dispmenu.rs: it draws
//! itself and takes the taps, and says what was asked (LbResult); the editor does it. Its functions never
//! reach back into the editor's state (PORTING.md, Sheets and keyboards).
//!
//! This file begins with the part the eye needs too: how the page's links group into entries and in what
//! order they are numbered, so the number on a box on the page and the number of its row in the bar are
//! one and the same.

use crate::prelude::*;

/// The most links a page has (nav.h), so the most rows and boxes.
pub const LB_MAX: usize = NAV_LINKS_PER_PAGE as usize;

/// Boxes whose tops are within this many mm are one line of the page, read left to right.
const LINE_MM: f32 = 10.0;

/// One row of the bar: a place the page's links go to.
#[derive(Clone, Copy, Debug, Default)]
pub struct LbEntry {
    /// "p<page id>" or "index" (or another version's)
    pub target: FixedStr<NAV_TARGET_LEN>,
    /// the page's index, NAV_TO_INDEX, or -1 for a page that is gone or a target this version does not know
    pub page: i32,
    /// links on the page to it (D1: shown once, x N)
    pub count: i32,
    /// the first of its boxes in reading order, page px
    pub first_box: Rect,
}

/// The links of one page (each with its box, page px) grouped by where they go: entries in the reading
/// order of each entry's first box, numbered from 1; `box_entry[i]` is the number of links[i]'s entry.
/// Boxes whose tops are within 10 mm count as one line, read left to right. Returns how many entries.
pub fn linkbar_group(
    nb: &NotebookInfo,
    links: &[NavLink],
    areas: &[Rect],
    dpi: f32,
    entries: &mut [LbEntry; LB_MAX],
    box_entry: &mut [u8; LB_MAX],
) -> usize {
    let n = links.len().min(areas.len()).min(LB_MAX);
    *box_entry = [0; LB_MAX];
    // Reading order: down the page, boxes within a line side by side.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (areas[i].y, areas[i].x, i));
    let line = mm_to_px(LINE_MM, dpi) as i32;
    let mut read: Vec<usize> = Vec::with_capacity(n);
    let mut at = 0;
    while at < order.len() {
        let top = areas[order[at]].y;
        let mut end = at;
        while end < order.len() && areas[order[end]].y - top <= line {
            end += 1;
        }
        let mut row: Vec<usize> = order[at..end].to_vec();
        row.sort_by_key(|&i| (areas[i].x, areas[i].y, i));
        read.extend(row);
        at = end;
    }
    let mut count = 0usize;
    for &i in &read {
        let target = links[i].target;
        let found = (0..count).find(|&e| entries[e].target.as_bytes() == target.as_bytes());
        let e = match found {
            Some(e) => e,
            None => {
                entries[count] = LbEntry { target, page: nav_link_page(&links[i], nb), count: 0, first_box: areas[i] };
                count += 1;
                count - 1
            }
        };
        entries[e].count += 1;
        box_entry[i] = (e + 1) as u8;
    }
    count
}
