// The popout's place (src/popout.rs, popout_place): beside the anchor, off a side panel, never over the anchor
// or an avoided rect when any place avoids them; the link card's cases (moved from test_linkcard.rs), avoid
// lists, sideways, a host with no panel, and the link card placed exactly as before the popout.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const UP: Rect = Rect { x: 0, y: 0, w: 1860, h: 2480 };
const SIDE: Rect = Rect { x: 0, y: 0, w: 2480, h: 1860 };
// the clock's corner zone at rest, top left (status_zone: 165 px)
const CORNER: Rect = Rect { x: 0, y: 0, w: 165, h: 165 };

fn inside(r: Rect, screen: Rect) -> bool {
    r.x >= screen.x + 20 && r.y >= screen.y + 20 && r.x + r.w <= screen.x + screen.w - 20 && r.y + r.h <= screen.y + screen.h - 20
}

// Right of the box when it fits; left when the box is at the right edge; below or above when neither side has
// room; never over the box when any place avoids it, never over the corner.
fn the_card_goes_beside_the_box() {
    let (w, h) = (780, 666);
    let none = RECT_EMPTY;
    // a box on the left: right of it, its top level with the box's
    let a = Rect { x: 200, y: 900, w: 600, h: 300 };
    let r = popout_place(w, h, a, UP, &[none, CORNER]);
    check!(r == Rect { x: 824, y: 900, w, h });
    // at the right edge: left of it
    let a = Rect { x: 1100, y: 900, w: 600, h: 300 };
    let r = popout_place(w, h, a, UP, &[none, CORNER]);
    check!(r == Rect { x: 1100 - 24 - 780, y: 900, w, h });
    // a box low on the page: clamped up into the screen on its free axis
    let a = Rect { x: 200, y: 2300, w: 600, h: 150 };
    let r = popout_place(w, h, a, UP, &[none, CORNER]);
    check!(r == Rect { x: 824, y: 2480 - 20 - h, w, h });
    // as wide as the page from side to side: below it, or above it near the bottom
    let a = Rect { x: 40, y: 300, w: 1780, h: 200 };
    let r = popout_place(w, h, a, UP, &[none, CORNER]);
    check!(r == Rect { x: 40, y: 524, w, h });
    let a = Rect { x: 40, y: 1900, w: 1780, h: 300 };
    let r = popout_place(w, h, a, UP, &[none, CORNER]);
    check!(r == Rect { x: 40, y: 1900 - 24 - h, w, h });
    for a in [
        Rect { x: 200, y: 900, w: 600, h: 300 },
        Rect { x: 1100, y: 100, w: 600, h: 300 },
        Rect { x: 40, y: 1900, w: 1780, h: 300 },
        Rect { x: 600, y: 20, w: 700, h: 200 },
    ] {
        let r = popout_place(w, h, a, UP, &[none, CORNER]);
        check!(inside(r, UP) && !rect_overlaps(r, a) && !rect_overlaps(r, CORNER));
    }
}

// The panel: a card that would meet it slides to the nearer side of it that stays on the screen; when no
// place is clear of it, the card draws over it rather than over the box.
fn the_card_keeps_off_the_panel() {
    let (w, h) = (780, 666);
    let panel = Rect { x: 1600, y: 220, w: 232, h: 1400 };
    let a = Rect { x: 200, y: 900, w: 600, h: 300 };
    let r = popout_place(w, h, a, UP, &[panel, CORNER]);
    check!(r == Rect { x: 800, y: 900, w, h }); // (824 + 780 = 1604 meets it: slid left of it, just clear of the box)
    check!(!rect_overlaps(r, a) && !rect_overlaps(r, panel));
    // a narrow box: the right candidate slides left of the panel and is clear of everything
    let a = Rect { x: 100, y: 900, w: 300, h: 300 };
    let r = popout_place(w, h, a, UP, &[panel, CORNER]);
    check!(!rect_overlaps(r, panel) && !rect_overlaps(r, a) && inside(r, UP));
    check!(r == Rect { x: 424, y: 900, w, h });
    // a panel on the left: the card stays right of the box
    let panel = Rect { x: 28, y: 220, w: 232, h: 1400 };
    let a = Rect { x: 400, y: 900, w: 300, h: 300 };
    let r = popout_place(w, h, a, UP, &[panel, CORNER]);
    check!(r == Rect { x: 724, y: 900, w, h });
}

// The corner is never met, a grown one included; sideways the same rules hold; a box that covers most of the
// screen gets the fallback (the right place clamped, moved below the corner).
fn the_card_never_meets_the_corner() {
    let (w, h) = (780, 666);
    // a box at the top right with the corner there grown (the indicator's box beside it)
    let corner = Rect { x: 1400, y: 0, w: 460, h: 200 };
    for a in [Rect { x: 100, y: 40, w: 500, h: 150 }, Rect { x: 1000, y: 210, w: 600, h: 120 }, Rect { x: 600, y: 30, w: 400, h: 100 }] {
        let r = popout_place(w, h, a, UP, &[RECT_EMPTY, corner]);
        check!(inside(r, UP) && !rect_overlaps(r, corner) && !rect_overlaps(r, a));
    }
    // sideways
    let a = Rect { x: 300, y: 500, w: 900, h: 400 };
    let r = popout_place(w, h, a, SIDE, &[RECT_EMPTY, CORNER]);
    check!(r == Rect { x: 1224, y: 500, w, h });
    let a = Rect { x: 1500, y: 1300, w: 900, h: 400 };
    let r = popout_place(w, h, a, SIDE, &[RECT_EMPTY, CORNER]);
    check!(inside(r, SIDE) && !rect_overlaps(r, a) && !rect_overlaps(r, CORNER));
    // most of the screen: nothing avoids the box; the right place clamped, and below the corner
    let a = Rect { x: 10, y: 10, w: 1840, h: 2460 };
    let r = popout_place(w, h, a, UP, &[RECT_EMPTY, CORNER]);
    check!(inside(r, UP) && !rect_overlaps(r, CORNER));
    check!(r.x == 1860 - 20 - w);
    let corner = Rect { x: 1100, y: 0, w: 760, h: 200 };
    let r = popout_place(w, h, a, UP, &[RECT_EMPTY, corner]);
    check!(r.y == 208 && !rect_overlaps(r, corner));
}

// No avoid list (the Library's item cards in 2.2): the card may go where the corner is, and the last resort is
// the right place clamped, moved nowhere.
fn a_card_that_avoids_nothing() {
    let (w, h) = (780, 666);
    // a box at the top left: right of it, its top level with the box's, over the corner's zone
    let a = Rect { x: 40, y: 40, w: 100, h: 100 };
    let r = popout_place(w, h, a, UP, &[]);
    check!(r == Rect { x: 164, y: 40, w, h });
    check!(rect_overlaps(r, CORNER));
    // the link card's corner moves it: right of the box still, below the corner it cannot meet
    let r = popout_place(w, h, a, UP, &[RECT_EMPTY, CORNER]);
    check!(!rect_overlaps(r, CORNER) && !rect_overlaps(r, a) && inside(r, UP));
    // most of the screen: the right place clamped, and nothing moves it
    let a = Rect { x: 10, y: 10, w: 1840, h: 2460 };
    let r = popout_place(w, h, a, UP, &[]);
    check!(r == Rect { x: 1860 - 20 - w, y: 20, w, h });
}

// More than the corner to avoid: none of them is covered when a place avoids them all; a panel slides the card
// off it sideways, upright and sideways.
fn a_card_avoids_every_rect_listed() {
    let (w, h) = (780, 666);
    let panel = Rect { x: 1600, y: 220, w: 232, h: 1400 };
    let band = Rect { x: 0, y: 1800, w: 1860, h: 400 };
    for a in [Rect { x: 200, y: 900, w: 600, h: 300 }, Rect { x: 900, y: 1500, w: 500, h: 200 }, Rect { x: 100, y: 300, w: 300, h: 120 }] {
        let r = popout_place(w, h, a, UP, &[panel, CORNER, band]);
        check!(inside(r, UP) && !rect_overlaps(r, a) && !rect_overlaps(r, panel));
        check!(!rect_overlaps(r, CORNER) && !rect_overlaps(r, band));
    }
    // no room on the right: left of the box, over the band; with the band avoided, above the box
    let a = Rect { x: 1000, y: 1300, w: 300, h: 300 };
    let r = popout_place(w, h, a, UP, &[RECT_EMPTY, CORNER]);
    check!(r == Rect { x: 1000 - 24 - w, y: 1300, w, h } && rect_overlaps(r, band));
    let r = popout_place(w, h, a, UP, &[RECT_EMPTY, CORNER, band]);
    check!(r == Rect { x: 1000, y: 1300 - 24 - h, w, h });
    // sideways, a panel on the right that the right place meets: slid left of it, just clear of the box
    let panel = Rect { x: 2000, y: 220, w: 232, h: 1400 };
    let a = Rect { x: 300, y: 500, w: 900, h: 400 };
    let r = popout_place(w, h, a, SIDE, &[panel, CORNER]);
    check!(r == Rect { x: 2000 - 20 - w, y: 500, w, h });
    check!(!rect_overlaps(r, panel) && !rect_overlaps(r, a) && inside(r, SIDE));
    // sideways with no panel: right of the box
    let r = popout_place(w, h, a, SIDE, &[RECT_EMPTY, CORNER]);
    check!(r == Rect { x: 1224, y: 500, w, h });
}

// The link card in its host (the editor: [panel, corner]) places exactly as linkcard_place did at 033dc1e4,
// kept here as it was: every anchor of a grid over both screens, with and without a panel on either side, the
// corner at rest and grown, each card height.
fn linkcard_place_033dc1e4(w: i32, h: i32, anchor: Rect, screen: Rect, panel: Rect, corner: Rect) -> Rect {
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
    let clear = |r: Rect| on_screen(r) && !rect_overlaps(r, anchor) && !rect_overlaps(r, corner);
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
    if rect_overlaps(r, corner) {
        r.y = corner.y + corner.h + ui_px(8.0);
    }
    r
}

fn the_link_card_places_as_it_did() {
    let mut n = 0;
    for screen in [UP, SIDE, Rect { x: 0, y: 160, w: 1860, h: 2320 }] {
        let panels = [
            RECT_EMPTY,
            Rect { x: screen.w - 260, y: 220, w: 232, h: 1400 },
            Rect { x: 28, y: 220, w: 232, h: 1400 },
            Rect { x: 200, y: screen.h - 300, w: 1400, h: 232 },
        ];
        let corners = [CORNER, Rect { x: screen.w - 460, y: 0, w: 460, h: 200 }, Rect { x: 0, y: screen.h - 165, w: 165, h: 165 }];
        for panel in panels {
            for corner in corners {
                for h in [218, 478, 666] {
                    for x in (-40..screen.w).step_by(97) {
                        for y in (-40..screen.h).step_by(113) {
                            for (aw, ah) in [(40, 36), (600, 300), (1780, 200), (300, 1400)] {
                                let a = Rect { x, y, w: aw, h: ah };
                                let old = linkcard_place_033dc1e4(780, h, a, screen, panel, corner);
                                let new = popout_place(780, h, a, screen, &[panel, corner]);
                                if old != new {
                                    eprintln!("  {a:?} {screen:?} {panel:?} {corner:?}: {old:?} then, {new:?} now");
                                }
                                check!(old == new);
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    check!(n > 100_000);
}

#[test]
fn test_popout() {
    ui_init(300.0);
    run!(the_card_goes_beside_the_box);
    run!(the_card_keeps_off_the_panel);
    run!(the_card_never_meets_the_corner);
    run!(a_card_that_avoids_nothing);
    run!(a_card_avoids_every_rect_listed);
    run!(the_link_card_places_as_it_did);
    done();
}
