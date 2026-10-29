//! The long-press card (Rust only, the Index's phase 2): a finger held on a link, or on a row of the Links bar,
//! shows a heavy dashed border around the link's area and, beside it, a card of what can be done with it:
//! Rename in the Index (or Add page %d to the Index...), Change where it goes..., Redraw the area and Delete
//! link. The page stays in view (R2). An editor-hosted sheet like linkbar.rs: it draws itself and takes the taps,
//! and says what was asked (LcResult); the editor does it. Its functions never reach back into the editor's
//! state (PORTING.md, Sheets and keyboards): the editor takes the card out of its state for each call that also
//! needs the App, and hands it the notebook and its Nav.
//!
//! Layout (logical px at 300 dpi, scaled with ui_px): 780 wide, from the top 24 px padding, a 110 px title row
//! (the target's name in 44 px bold, at the right 'p. %d' at 34 px and '×%d' for a bar row of N links), the
//! rows (732 x 118 each, 12 px apart), for N links the line 'To change one, hold it on the page.' (60 px, 12 px
//! above it), 24 px padding: 666 px with four rows, 536 with three, 608 or 478 for N links, 218 read-only.
//!
//! Where it goes (linkcard_place) is a pure function, tested: beside the border, never over it when any place
//! avoids it, never over the clock's corner, and off the panel when it can be.

use crate::prelude::*;

// Hit ids.
const LC_ROW: i32 = 1;

/// What a row of the card does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardRow {
    /// Rename in the Index (D3: the target page's Index name, everywhere), or Add page %d to the Index...
    #[default]
    Rename,
    /// Change where it goes... (Change where they go... for N links)
    Retarget,
    /// Redraw the area (one link only, D1)
    Redraw,
    /// Delete link (Delete %d links)
    Delete,
}

/// What a press on the card asks the editor to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LcResult {
    #[default]
    None,
    /// a row tapped
    Row(CardRow),
    /// the pressed row was let go without a tap: its rect is to be refreshed
    Changed,
}

/// The card's state while it is up (memory only), and the border, which stays while a retarget or a redraw
/// it began goes on.
#[derive(Clone, Debug, Default)]
pub struct LinkCard {
    pub open: bool,
    /// the page the links are on (the open page) and their seeds: 1 from the page, N from a bar row
    pub pid: u32,
    pub seeds: Vec<u32>,
    /// where they go ("p<page id>", "index", or another version's)
    pub target: FixedStr<NAV_TARGET_LEN>,
    /// nav_link_page: a page index, NAV_TO_INDEX, or -1 (a page gone, another version's target)
    pub page: i32,
    pub readonly: bool,
    pub from_bar: bool,
    pub rows: [CardRow; 4],
    pub nrows: usize,
    pub enabled: [bool; 4],
    /// the title as drawn ("Árbol", "Page 2", "The Index", "Page not here")
    pub title: String,
    /// each row's label, as drawn
    pub labels: [String; 4],
    /// page px: the padded boxes the heavy border goes around; `border` while it is drawn (also during a
    /// retarget or a redraw the card began)
    pub boxes: Vec<Rect>,
    pub border: bool,
    /// screen px: everything the border covers (for closing), and the card
    pub band: Rect,
    pub rect: Rect,
    /// Rename's keyboard
    pub kb: Keyboard,
    pub hits: HitList,
    pub press: Press,
    pub pressed: Hit,
    pub has_pressed: bool,
}

fn u(v: f32) -> i32 {
    ui_px(v)
}

/// The card's height: `nrows` rows, the ×N line under them for a bar row of several links, or the read-only
/// line in place of rows.
pub fn linkcard_height(nrows: usize, xn: bool, readonly: bool) -> i32 {
    if readonly {
        return u(24.0) + u(110.0) + u(60.0) + u(24.0);
    }
    let n = nrows as i32;
    let rows = n * u(118.0) + (n - 1).max(0) * u(12.0);
    u(24.0) + u(110.0) + rows + if xn { u(12.0) + u(60.0) } else { 0 } + u(24.0)
}

/// The card's size as linkcard_set left it: 780 wide, as tall as its rows need (the same sideways).
pub fn linkcard_size(c: &LinkCard) -> (i32, i32) {
    (u(780.0), linkcard_height(c.nrows, c.seeds.len() > 1, c.readonly))
}

/// The card for the links `seeds` on page pid (one from the page, N from a bar row, all to one target): its
/// title and its rows (PHASE2 8.3). Rename is left out for the Index (and for N links to a page gone), and
/// disabled for a page gone or while nav.ini cannot be saved; N links have no Redraw; a read-only page has no
/// rows. The boxes, the band and where the card goes are the editor's (it knows the page and the view).
pub fn linkcard_set(c: &mut LinkCard, pid: u32, seeds: &[u32], nb: &NotebookInfo, nav: &Nav, readonly: bool, from_bar: bool) {
    let kb = std::mem::take(&mut c.kb);
    let boxes = std::mem::take(&mut c.boxes);
    *c = LinkCard { open: true, pid, readonly, from_bar, kb, boxes, ..Default::default() };
    c.boxes.clear();
    // (a handful of seeds: with no room the card acts on none, and offers nothing)
    if c.seeds.try_reserve_exact(seeds.len()).is_ok() {
        c.seeds.extend_from_slice(seeds);
    }
    let l = seeds.first().and_then(|&s| nav_link_get(nav, pid, s));
    if let Some(l) = l {
        c.target = l.target;
        c.page = nav_link_page(&l, nb);
    } else {
        c.page = -1;
    }
    let name = if c.page >= 0 && (c.page as usize) < nb.page_ids.len() { nav_name(nav, nb.page_ids[c.page as usize]) } else { "" };
    c.title = if c.page == NAV_TO_INDEX {
        tr("The Index").to_string()
    } else if c.page < 0 {
        tr("Page not here").to_string()
    } else if name.is_empty() {
        cformat!(tr("Page %d"), c.page + 1)
    } else {
        name.to_string()
    };
    if readonly {
        return;
    }
    let many = c.seeds.len() > 1;
    let add = |c: &mut LinkCard, r: CardRow, on: bool, label: String| {
        c.rows[c.nrows] = r;
        c.enabled[c.nrows] = on;
        c.labels[c.nrows] = label;
        c.nrows += 1;
    };
    // Rename: the page's Index name (D3), or the page added to the Index; none for the Index.
    if c.page != NAV_TO_INDEX && !(many && c.page < 0) {
        let label = if c.page >= 0 && name.is_empty() {
            // TRANSLATORS: a row of the card a finger held on a link opens, for a link to a page with no name in
            // the Index: %d is that page's number; the row names it (about 700 px at 44 px).
            cformat!(tr("Add page %d to the Index..."), c.page + 1)
        } else {
            // TRANSLATORS: a row of the long-press card of a link: rename the page it goes to, in the Index
            // (everywhere the page is listed; about 700 px at 44 px).
            tr("Rename in the Index").to_string()
        };
        let on = c.page >= 0 && nav.writable;
        add(c, CardRow::Rename, on, label);
    }
    let retarget = if many {
        // TRANSLATORS: a row of the long-press card of a Links bar row of several links: change where all of them
        // go (about 700 px at 44 px).
        tr("Change where they go...")
    } else {
        // TRANSLATORS: a row of the long-press card of a link: pick another place for it to go (about 700 px at
        // 44 px).
        tr("Change where it goes...")
    };
    add(c, CardRow::Retarget, true, retarget.to_string());
    if !many {
        // TRANSLATORS: a row of the long-press card of a link: draw a new area for it with the pen (about 700 px
        // at 44 px).
        add(c, CardRow::Redraw, true, tr("Redraw the area").to_string());
    }
    let delete = if many {
        let n = c.seeds.len() as i32;
        // TRANSLATORS: a row of the long-press card of a Links bar row: delete all its links on this page, %d
        // of them (about 700 px at 44 px). The ink around them stays.
        cformat!(ng_("Delete %d link", "Delete %d links", n as i64), n)
    } else {
        // TRANSLATORS: a row of the long-press card of a link: delete the link (the ink around it stays; about
        // 700 px at 44 px).
        tr("Delete link").to_string()
    };
    add(c, CardRow::Delete, true, delete);
}

/// The card's place on screen `screen` beside `anchor` (the border's outer rect), PHASE2 8.4: right of it, then
/// left, below, above; each clamped into the screen 20 px in on its free axis, and slid sideways off the panel
/// when it meets it. The first that is on the screen, does not cover the anchor, the panel or `corner` (the
/// clock's corner zone) wins; then the first that covers neither the anchor nor the corner (it draws over the
/// panel); else the right one clamped to the screen, moved below the corner when it meets it.
pub fn linkcard_place(w: i32, h: i32, anchor: Rect, screen: Rect, panel: Rect, corner: Rect) -> Rect {
    let m = u(20.0);
    let gap = u(24.0);
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
        r.y = corner.y + corner.h + u(8.0);
    }
    r
}

/// The card from a Links bar row (PHASE2 8.10): above the bar, 16 px over it, in the held chip's column,
/// kept 20 px in from the screen's sides.
pub fn linkcard_place_above(w: i32, h: i32, chip: Rect, bar: Rect, screen: Rect) -> Rect {
    let m = u(20.0);
    let x = clampi(chip.x, screen.x + m, screen.x + screen.w - w - m);
    Rect { x, y: bar.y - u(16.0) - h, w, h }
}

/// The heavy border's dashes and halo: 5 px dashes, 20 on and about 14 off, a 3 px white halo.
fn border_pen() -> (i32, i32, i32, i32) {
    (u(5.0), u(20.0), u(14.0), u(3.0))
}

/// The heavy border around r (screen px): a dashed rectangle with a white halo, and solid 24 px corner Ls of
/// the same thickness, so the link held reads apart from the eye's lighter boxes. 1-bit (0 and 255 only): a DU
/// shows it exactly. Clipped to the canvas's clip.
pub fn linkcard_border(c: &mut Canvas, r: Rect) {
    let (t, on, off, halo) = border_pen();
    canvas_dash_rect(c, r, t, on, off, halo);
    let t = t.min(r.w).min(r.h);
    let l = u(24.0).min(r.w).min(r.h);
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    for (x, y, w, h) in [
        (x0, y0, l, t),
        (x0, y0, t, l),
        (x1 - l, y0, l, t),
        (x1 - t, y0, t, l),
        (x0, y1 - t, l, t),
        (x0, y1 - l, t, l),
        (x1 - l, y1 - t, l, t),
        (x1 - t, y1 - l, t, l),
    ] {
        canvas_fill(c, Rect { x, y, w, h }, 0);
    }
}

/// The four strips linkcard_border(r) paints, without overlaps: the top and the bottom the full width (the halo
/// included), the left and the right between them. Their union is exactly what it paints, so four DU of them
/// show the border with nothing of the gray ink inside presented on the 1-bit waveform.
pub fn linkcard_strips(r: Rect) -> [Rect; 4] {
    let (t, _, _, halo) = border_pen();
    let t = t.min(r.w).min(r.h);
    let o = rect_inflate(r, halo);
    let s = t + 2 * halo; // (a strip's thickness: the dash and its halo on both sides)
    let top = Rect { x: o.x, y: o.y, w: o.w, h: s.min(o.h) };
    let bottom = Rect { x: o.x, y: (o.y + o.h - s).max(top.y + top.h), w: o.w, h: (o.y + o.h - (top.y + top.h)).clamp(0, s) };
    let mid_y = top.y + top.h;
    let mid_h = (bottom.y - mid_y).max(0);
    let left = Rect { x: o.x, y: mid_y, w: s.min(o.w), h: mid_h };
    let right = Rect { x: (o.x + o.w - s).max(left.x + left.w), y: mid_y, w: (o.x + o.w - (left.x + left.w)).clamp(0, s), h: mid_h };
    [top, bottom, left, right]
}

/// The outer rect of the border around r: the four strips' bounds.
pub fn linkcard_border_reach(r: Rect) -> Rect {
    let st = linkcard_strips(r);
    st.iter().fold(RECT_EMPTY, |u, &s| rect_union(u, s))
}

// Where row k is on the card.
fn row_rect(c: &LinkCard, k: usize) -> Rect {
    let y = c.rect.y + u(24.0) + u(110.0) + k as i32 * (u(118.0) + u(12.0));
    Rect { x: c.rect.x + u(24.0), y, w: c.rect.w - 2 * u(24.0), h: u(118.0) }
}

fn row_icon(r: CardRow) -> IconKind {
    match r {
        CardRow::Rename => IconKind::Rename,
        CardRow::Retarget => IconKind::Link,
        CardRow::Redraw => IconKind::Pencil,
        CardRow::Delete => IconKind::Trash,
    }
}

/// Draws the card at c.rect into a.screen and registers the hits of its enabled rows; returns its rect.
pub fn linkcard_draw(c: &mut LinkCard, a: &mut App, _nb: &NotebookInfo, _nav: &Nav) -> Rect {
    hits_clear(&mut c.hits);
    let card = c.rect;
    let s = &mut a.screen;
    ui_frame(s, card, u(24.0), 255, true, 0);
    ui_frame(s, card, u(24.0), 0, false, u(6.0));
    let inn = Rect { x: card.x + u(24.0), y: card.y + u(24.0), w: card.w - 2 * u(24.0), h: card.h - 2 * u(24.0) };
    // The title row: the name, and at the right where it goes and how many links.
    let col_w = u(150.0);
    let title = Rect { x: inn.x + u(8.0), y: inn.y, w: inn.w - col_w - u(24.0), h: u(110.0) };
    let ts = ui_text_size();
    font_draw_left_name(s, FontFace::Bold, ts, title, &c.title, 0);
    let mut lines: Vec<String> = Vec::new();
    if c.page >= 0 {
        lines.push(cformat!(c_("bar", "p. %d"), c.page + 1));
    }
    if c.seeds.len() > 1 {
        lines.push(cformat!(tr("×%d"), c.seeds.len() as i32));
    }
    let col = Rect { x: inn.x + inn.w - col_w, y: inn.y, w: col_w, h: u(110.0) };
    let lh = u(46.0);
    let top = col.y + (col.h - lh * lines.len() as i32) / 2;
    for (k, l) in lines.iter().enumerate() {
        font_draw_centered(s, FontFace::Regular, ui_small_size(), Rect { x: col.x, y: top + k as i32 * lh, w: col.w, h: lh }, l, 0);
    }
    if c.readonly {
        let r = Rect { x: inn.x + u(8.0), y: inn.y + u(110.0), w: inn.w - u(16.0), h: u(60.0) };
        ui_text(s, FontFace::Regular, ui_small_size(), ui_small_size() * 0.88, r, tr("This page is read-only."), 1, false, 0);
        return card;
    }
    for k in 0..c.nrows {
        let r = row_rect(c, k);
        let on = c.enabled[k];
        let st = if on { ButtonStyle::Plain } else { ButtonStyle::Disabled };
        ui_button(s, r, IconKind::None, "", st);
        let fg = if on { 0 } else { 170 };
        let d = u(64.0);
        ui_icon(s, row_icon(c.rows[k]), Rect { x: r.x + u(28.0), y: r.y + (r.h - d) / 2, w: d, h: d }, fg);
        let lr = Rect { x: r.x + u(120.0), y: r.y, w: r.w - u(120.0) - u(20.0), h: r.h };
        // (a long translation takes a smaller size on one line, down to the small one, then two lines)
        ui_text(s, FontFace::Regular, ts, ui_small_size(), lr, &c.labels[k], 2, false, fg);
        if on {
            hits_add(&mut c.hits, r, LC_ROW, k as i32);
        }
    }
    if c.seeds.len() > 1 {
        let y = row_rect(c, c.nrows.max(1) - 1).y + u(118.0) + u(12.0);
        let r = Rect { x: inn.x + u(8.0), y, w: inn.w - u(16.0), h: u(60.0) };
        // TRANSLATORS: the last line of the long-press card of a Links bar row of several links (34 px, about
        // 700 px): its rows act on all of them; one is changed alone by holding it on the page.
        ui_text(s, FontFace::Regular, ui_small_size(), ui_small_size() * 0.88, r, tr("To change one, hold it on the page."), 1, false, 0);
    }
    card
}

/// A pen or finger on the card (phase 0 down, 1 move, 2 up): a row inverts at once, on a DU of its own
/// (app_press_feedback), and acts at the lift if that is a tap. `dirty` gets the pressed row's rect to refresh
/// when it is let go.
pub fn linkcard_pointer(c: &mut LinkCard, a: &mut App, ph: i32, ev: &InEvent, dirty: &mut Rect) -> LcResult {
    *dirty = RECT_EMPTY;
    if ph == 0 {
        press_begin(&mut c.press, ev.x, ev.y, ev.t_ms);
        let h = hits_find(&c.hits, ev.x, ev.y).copied();
        c.has_pressed = h.is_some();
        if let Some(h) = h {
            c.pressed = h;
            app_press_feedback(a, h.r);
        }
        return LcResult::None;
    }
    if !c.has_pressed {
        return LcResult::None;
    }
    press_move(&mut c.press, ev.x, ev.y);
    if ph == 1 {
        return LcResult::None;
    }
    c.has_pressed = false;
    let h = c.pressed;
    canvas_invert(&mut a.screen, h.r);
    *dirty = h.r;
    if !press_is_tap(&c.press, h.r) || h.id != LC_ROW || h.arg < 0 || h.arg as usize >= c.nrows {
        return LcResult::Changed;
    }
    LcResult::Row(c.rows[h.arg as usize])
}

/// The finger that pressed a row is a pinch now: the row is drawn as it was. The rect to refresh (empty when no
/// row was pressed).
pub fn linkcard_cancel_press(c: &mut LinkCard, a: &mut App) -> Rect {
    let was = c.has_pressed;
    c.has_pressed = false;
    if !was {
        return RECT_EMPTY;
    }
    canvas_invert(&mut a.screen, c.pressed.r);
    c.pressed.r
}

fn row_name(r: CardRow) -> &'static str {
    match r {
        CardRow::Rename => "rename",
        CardRow::Retarget => "retarget",
        CardRow::Redraw => "redraw",
        CardRow::Delete => "delete",
    }
}

/// The name of the card's row at screen point (x, y), for the runner's editor.ui_at: `card.rename`,
/// `card.retarget`, `card.redraw`, `card.delete`; empty for none (a disabled row has no hit).
pub fn linkcard_hit_name(c: &LinkCard, x: i32, y: i32) -> String {
    if !c.open || c.kb.open {
        return String::new();
    }
    match hits_find(&c.hits, x as f32, y as f32) {
        Some(h) if h.id == LC_ROW && h.arg >= 0 && (h.arg as usize) < c.nrows => cformat!("card.%s", row_name(c.rows[h.arg as usize])),
        _ => String::new(),
    }
}

/// The runner's keys for the card, without "editor.link_card.": "rows" (the row names in order, a disabled one
/// with a leading `-`; `readonly` for the read-only card), "title" (as drawn), "rect" (`x y w h`, empty
/// closed), "links" (how many it acts on) and "border" (how many boxes carry the heavy border now). false for
/// another key.
pub fn linkcard_probe(c: &LinkCard, key: &str, out: &mut String, cap: usize) -> bool {
    if key == "rows" {
        let mut v = String::new();
        if c.open && c.readonly {
            v.push_str("readonly");
        } else if c.open {
            for k in 0..c.nrows {
                if !v.is_empty() {
                    v.push(' ');
                }
                if !c.enabled[k] {
                    v.push('-');
                }
                v.push_str(row_name(c.rows[k]));
            }
        }
        csnprintf!(out, cap, "%s", &v);
    } else if key == "title" {
        csnprintf!(out, cap, "%s", if c.open { c.title.as_str() } else { "" });
    } else if key == "rect" {
        if c.open {
            csnprintf!(out, cap, "%d %d %d %d", c.rect.x, c.rect.y, c.rect.w, c.rect.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "links" {
        csnprintf!(out, cap, "%d", if c.open { c.seeds.len() as i32 } else { 0 });
    } else if key == "border" {
        csnprintf!(out, cap, "%d", if c.border { c.boxes.len() as i32 } else { 0 });
    } else {
        return false;
    }
    true
}
