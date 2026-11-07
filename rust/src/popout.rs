//! One popout (the 2.1 plan, DESIGN.md "One popout"): a card anchored beside what was held. Placement only
//! for now (lane L0): the link card's place, which the quick size card and the picker's item cards will share.
//! Its frame, rows, chips, contacts and keys come with the first card that needs them (lane L1).
//!
//! Where it goes (popout_place) is a pure function, tested (tests/test_popout.rs): beside the anchor, never over
//! it when any place avoids it, never over an avoided rect, and slid off a side panel when it can be.

use crate::prelude::*;

/// The card's place on screen `screen` beside `anchor` (for the link card, its border's outer rect), PHASE2 8.4:
/// right of it, then left, below, above; each clamped into the screen 20 px in on its free axis.
///
/// `avoid` is what the card keeps clear of, in order: the first is a side panel (the editor's tool panel), the
/// rest are never covered when any place avoids them (the clock's corner). A candidate that meets the panel is
/// slid sideways to the nearer side of it that stays on the screen. The first candidate that is on the screen
/// and covers neither the anchor nor anything avoided wins; then the first that covers neither the anchor nor
/// the rest (it draws over the panel); else the right one clamped to the screen, moved below each of the rest it
/// meets. The link card passes [panel, corner], a host with no panel [RECT_EMPTY, corner], and a card that
/// avoids nothing (the Library's item cards in 2.2) an empty list.
pub fn popout_place(w: i32, h: i32, anchor: Rect, screen: Rect, avoid: &[Rect]) -> Rect {
    let panel = avoid.first().copied().unwrap_or(RECT_EMPTY);
    let rest = avoid.get(1..).unwrap_or(&[]);
    let m = ui_px(20.0);
    let gap = ui_px(24.0);
    let x_lo = screen.x + m;
    let x_hi = screen.x + screen.w - m - w;
    let y_lo = screen.y + m;
    let y_hi = screen.y + screen.h - m - h;
    let on_screen = |r: Rect| r.x >= x_lo && r.x <= x_hi && r.y >= y_lo && r.y <= y_hi;
    let cy = clampi(anchor.y, y_lo, y_hi);
    let cx = clampi(anchor.x, x_lo, x_hi);
    let cands = [
        Rect { x: anchor.x + anchor.w + gap, y: cy, w, h },
        Rect { x: anchor.x - gap - w, y: cy, w, h },
        Rect { x: cx, y: anchor.y + anchor.h + gap, w, h },
        Rect { x: cx, y: anchor.y - gap - h, w, h },
    ];
    // Off the panel: to the nearer side of it that stays on the screen.
    let slide = |r: Rect| -> Rect {
        if !rect_overlaps(r, panel) {
            return r;
        }
        let left = Rect { x: panel.x - m - w, ..r };
        let right = Rect { x: panel.x + panel.w + m, ..r };
        let (near, far) = if (r.x - left.x).abs() <= (right.x - r.x).abs() { (left, right) } else { (right, left) };
        if on_screen(near) {
            near
        } else if on_screen(far) {
            far
        } else {
            r
        }
    };
    let clear = |r: Rect| on_screen(r) && !rect_overlaps(r, anchor) && !rest.iter().any(|&k| rect_overlaps(r, k));
    for &c in &cands {
        let r = slide(c);
        if clear(r) && !rect_overlaps(r, panel) {
            return r;
        }
    }
    for &c in &cands {
        for r in [slide(c), c] {
            if clear(r) {
                return r;
            }
        }
    }
    let mut r = Rect { x: clampi(cands[0].x, x_lo, x_hi), y: clampi(cands[0].y, y_lo, y_hi), w, h };
    for &k in rest {
        if rect_overlaps(r, k) {
            r.y = k.y + k.h + ui_px(8.0);
        }
    }
    r
}
