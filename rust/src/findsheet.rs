//! The Find sheet (Rust only, the Index's phase 1): go to a page by its number or by a name in the Index. An
//! editor-hosted sheet like dispmenu.rs and linkbar.rs: a keyboard along the bottom ("Page number or name"), and
//! above it a card of up to five rows (three sideways) that follow what is typed:
//!
//!   - digits: "Go to page 12" when the notebook has that page, else "There is no page 12 (5 pages)." (not a
//!     button);
//!   - letters: the named pages whose names the text finds (i18n_name_match: the beginning of the name, then of a
//!     word, then anywhere; accents and capitals ignored), best first, then in page order; or "No page names
//!     match." (not a button).
//!
//! In pick mode (PHASE2 7.2, Link to...'s Search) the same sheet finds where a new link goes: a digits row reads
//! "Link to page 12", and its keyboard may dock at the top of the screen, below the clock corner, with the rows
//! under it, so neither covers the link's area low on the page.
//!
//! A tap on a row, or the keyboard's Done on the first row that is a button, goes there; Cancel, a tap outside it
//! and the menu's own action close it. It draws itself and takes the taps and says what was asked (FsResult); the
//! editor does it. Its functions never reach back into the editor's state (PORTING.md, Sheets and keyboards).
//!
//! Typing refreshes three things, each on its own fast refresh: the keyboard's text line (the split keyboard host
//! does it), then this card, then the key (app_keyboard_pointer_split): the card between them is why the host was
//! split, as a union from the card down to the key would be the overlapping refresh the host avoids.

use crate::prelude::*;

/// The most rows upright (sideways three).
pub const FS_ROWS: usize = 5;

const FS_ROW: i32 = 1;

/// Go to a page (Go), or pick the page a new link goes to (Pick).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FindMode {
    #[default]
    Go,
    Pick,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FindRow {
    /// the page's index; -1 for a line that says why there is nothing (not a button)
    pub page: i32,
    /// a page found by its name (drawn in bold with its number at the right), not by its number
    pub named: bool,
    pub label: FixedStr<128>,
}

/// What a tap on the sheet asks the editor to do.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FsResult {
    #[default]
    None,
    /// go to this page (an index)
    Go(i32),
    /// Cancel on the keyboard: close it
    Close,
    /// the text changed (the rows were made again; see FindSheet.rows_dirty) or the keyboard did
    Changed,
}

#[derive(Clone, Debug, Default)]
pub struct FindSheet {
    pub open: bool,
    pub mode: FindMode,
    pub kb: Keyboard,
    pub rows: [FindRow; FS_ROWS],
    pub n: usize,
    /// the rows' card (empty while there are none)
    pub rect: Rect,
    pub hits: HitList,
    pub press: Press,
    pub pressed: Hit,
    pub has_pressed: bool,
    /// what the last typing changed of the card: the old and the new card together, for the editor to draw again
    /// and refresh (empty when the rows stayed as they were)
    pub rows_dirty: Rect,
    /// the contact in progress is the keyboard's
    on_keys: bool,
    /// the most rows there is room for
    max_rows: usize,
}

fn u(v: f32) -> i32 {
    ui_px(v)
}

/// Opens the sheet with an empty field: the letters page. The editor draws it (findsheet_draw) and refreshes it.
pub fn findsheet_open(f: &mut FindSheet, a: &App) {
    *f = FindSheet { open: true, max_rows: if a.w > a.h { 3 } else { FS_ROWS }, ..Default::default() };
    // TRANSLATORS: the title of the keyboard of the Find sheet, in its text line (about 560 px at 44 px in bold):
    // go to a page by typing its number or a name of the Index.
    keyboard_open(&mut f.kb, tr("Page number or name"), b"");
    f.kb.area = keyboard_modal_rect(Rect { x: 0, y: 0, w: a.w, h: a.h });
    layout(f);
}

/// Opens the sheet in pick mode, for Link to...'s Search: its keyboard docked with its top at top_y (0: at the
/// bottom, as in Go mode), the rows under it when it is at the top.
pub fn findsheet_open_pick(f: &mut FindSheet, a: &App, top_y: i32) {
    findsheet_open(f, a);
    f.mode = FindMode::Pick;
    keyboard_set_dock(&mut f.kb, top_y);
    f.kb.area = keyboard_modal_rect_docked(Rect { x: 0, y: 0, w: a.w, h: a.h }, f.kb.dock_y);
    layout(f);
}

// The card above the keyboard: its rows 124 px tall with 16 px between and 16 px of padding, the bottom 12 px
// above the keyboard, its top never above 8 px from the screen's (the clock and battery is not shown while it is up).
// Under a keyboard docked at the top, the card's top is 16 px below it.
fn layout(f: &mut FindSheet) {
    hits_clear(&mut f.hits);
    if f.n == 0 {
        f.rect = RECT_EMPTY;
        return;
    }
    let (rh, gap, pad) = (u(124.0), u(16.0), u(16.0));
    let h = 2 * pad + f.n as i32 * rh + (f.n as i32 - 1) * gap;
    if f.kb.dock_y > 0 {
        f.rect = Rect { x: f.kb.area.x, y: f.kb.area.y + f.kb.area.h + u(16.0), w: f.kb.area.w, h };
    } else {
        let floor = u(8.0);
        let bottom = f.kb.area.y - u(12.0);
        let top = (bottom - h).max(floor);
        f.rect = Rect { x: f.kb.area.x, y: top, w: f.kb.area.w, h: bottom - top };
    }
    for i in 0..f.n {
        if f.rows[i].page >= 0 {
            let r = row_rect(f, i);
            hits_add(&mut f.hits, r, FS_ROW, i as i32);
        }
    }
}

fn row_rect(f: &FindSheet, i: usize) -> Rect {
    let (rh, gap, pad) = (u(124.0), u(16.0), u(16.0));
    // (the last row sits on the card's bottom padding)
    let y = f.rect.y + f.rect.h - pad - (f.n - i) as i32 * rh - (f.n - 1 - i) as i32 * gap;
    Rect { x: f.rect.x + pad, y, w: f.rect.w - 2 * pad, h: rh }
}

/// The rows for what the field holds now, made again (the card moved by layout). Returns whether they changed.
pub fn findsheet_update(f: &mut FindSheet, _a: &App, nb: &NotebookInfo, nav: &Nav) -> bool {
    let before: Vec<(i32, String)> = f.rows[..f.n].iter().map(|r| (r.page, r.label.as_str().to_string())).collect();
    let old = f.rect;
    let text = f.kb.text.as_str().trim().to_string();
    let mut rows: Vec<(i32, String, bool)> = Vec::new();
    if text.is_empty() {
        // (nothing typed: no rows)
    } else if text.bytes().all(|b| b.is_ascii_digit()) {
        // A page number: the page, or why not.
        let npages = nb.npages() as i64;
        let n = text.bytes().fold(0i64, |v, b| (v * 10 + (b - b'0') as i64).min(1 << 40));
        if n >= 1 && n <= npages {
            // TRANSLATORS: a row of the Find sheet for a page number typed, which a tap goes to (about 1300 px at 44 px):
            // %d is the number.
            let row = if f.mode == FindMode::Pick {
                // TRANSLATORS: a row of the Find sheet while it picks where a new link goes (Link to...'s Search),
                // for a page number typed, which a tap links to (about 1300 px at 44 px): %d is the number.
                cformat!(tr("Link to page %d"), n as i32)
            } else {
                cformat!(tr("Go to page %d"), n as i32)
            };
            rows.push((n as i32 - 1, row, false));
        } else {
            // TRANSLATORS: a line of the Find sheet for a page number the notebook does not have (not a button, about
            // 1300 px at 44 px): the first %d is the number typed, the second how many pages it has.
            let msg = ng_("There is no page %d (%d page).", "There is no page %d (%d pages).", npages);
            rows.push((-1, cformat!(msg, n.min(i32::MAX as i64) as i32, npages as i32), false));
        }
    } else {
        // The named pages the text finds, best first and in page order among equals.
        let mut idx = vec![0i32; nb.page_ids.len()];
        let named = nav_index(nav, nb, &mut idx).max(0) as usize;
        let mut found: Vec<(i32, i32)> = Vec::new();
        for &i in idx.iter().take(named.min(idx.len())) {
            let rank = i18n_name_match(nav_name(nav, nb.page_ids[i as usize]), &text);
            if rank > 0 {
                found.push((rank, i));
            }
        }
        found.sort_by_key(|&(rank, i)| (-rank, i));
        for (_, i) in found.into_iter().take(f.max_rows.max(1)) {
            rows.push((i, nav_name(nav, nb.page_ids[i as usize]).to_string(), true));
        }
        if rows.is_empty() {
            // TRANSLATORS: a line of the Find sheet when no page name has what was typed (not a button, about
            // 1300 px at 44 px).
            rows.push((-1, tr("No page names match.").to_string(), false));
        }
    }
    rows.truncate(f.max_rows.max(1));
    f.n = rows.len();
    for (i, (page, label, named)) in rows.iter().enumerate() {
        f.rows[i] = FindRow { page: *page, named: *named, label: FixedStr::default() };
        f.rows[i].label.set_cut(label);
    }
    layout(f);
    let after: Vec<(i32, String)> = f.rows[..f.n].iter().map(|r| (r.page, r.label.as_str().to_string())).collect();
    let changed = before != after || old != f.rect;
    f.rows_dirty = if changed { rect_union(old, f.rect) } else { RECT_EMPTY };
    changed
}

/// Draws the card of rows (`f.rect`): a row each, its label at the left (a name in bold), a page's number at the
/// right; a line that is no button is lighter.
fn draw_rows(f: &FindSheet, a: &mut App) {
    if f.n == 0 {
        return;
    }
    let c = &mut a.screen;
    ui_frame(c, f.rect, u(24.0), 255, true, 0);
    ui_frame(c, f.rect, u(24.0), 0, false, u(6.0));
    let ts = ui_text_size();
    for i in 0..f.n {
        let r = row_rect(f, i);
        let row = &f.rows[i];
        if row.page >= 0 {
            ui_frame(c, r, u(16.0), 255, true, 0);
            ui_frame(c, r, u(16.0), 0, false, u(3.0));
            let col = u(150.0);
            let text = Rect { x: r.x + u(30.0), y: r.y, w: r.w - u(30.0) - col - u(14.0), h: r.h };
            if row.named {
                font_draw_left_name(c, FontFace::Bold, ts, text, row.label.as_str(), 0);
                // TRANSLATORS: a row of the Find sheet, in its right column at 34 px (about 150 px): the page a name is on, %d its number ("p. 12").
                let at = cformat!(c_("bar", "p. %d"), row.page + 1);
                font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect { x: r.x + r.w - col - u(8.0), y: r.y, w: col, h: r.h }, &at, 0);
            } else {
                font_draw_left(c, FontFace::Regular, ts, text, row.label.as_str(), 0);
            }
        } else {
            // (a line that says why there is nothing: no frame, lighter)
            font_draw_left(c, FontFace::Regular, ts, Rect { x: r.x + u(30.0), y: r.y, w: r.w - u(60.0), h: r.h }, row.label.as_str(), 150);
        }
    }
}

/// Draws the sheet where it meets `r`: the card of rows and the keyboard, each only if r reaches it. Returns the
/// card and the keyboard together (what a refresh of the sheet covers). The toast keeps above the keyboard.
pub fn findsheet_draw(f: &mut FindSheet, a: &mut App, nb: &NotebookInfo, nav: &Nav, r: Rect) -> Rect {
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let kb_area = keyboard_modal_rect_docked(screen, f.kb.dock_y);
    // (a turn changes how many rows there is room for: the rows are made again for it)
    let cap = if a.w > a.h { 3 } else { FS_ROWS };
    if cap != f.max_rows {
        f.max_rows = cap;
        f.kb.area = kb_area;
        findsheet_update(f, a, nb, nav);
    }
    if rect_overlaps(r, kb_area) {
        keyboard_draw(&mut f.kb, &mut a.screen, screen);
    } else {
        f.kb.area = kb_area;
    }
    layout(f);
    if rect_overlaps(r, f.rect) {
        draw_rows(f, a);
    }
    if f.kb.dock_y == 0 {
        a.toast_floor = f.kb.area.y; // (docked at the top, the toast keeps its place)
    }
    rect_union(f.rect, f.kb.area)
}

/// Draws the card of rows alone, where typing changed it (the editor draws the page under it again first).
pub fn findsheet_draw_rows(f: &mut FindSheet, a: &mut App) -> Rect {
    layout(f);
    draw_rows(f, a);
    f.rect
}

/// A pen or finger on the sheet (phase 0 down, 1 move, 2 up). A contact that began on the keyboard is the
/// keyboard's: through the split host, which refreshes the text line on its own; then, when what was typed
/// changed the rows, `rows_dirty` says what the editor draws again and refreshes before the key
/// (the key's rect is in `dirty`, to be invalidated on the fast waveform after it). One that began on a row
/// inverts it at once on a DU of its own, is drawn as it was at the lift (a Ui) and goes there if that was a tap.
pub fn findsheet_pointer(f: &mut FindSheet, a: &mut App, nb: &NotebookInfo, nav: &Nav, ph: i32, ev: &InEvent, dirty: &mut Rect) -> FsResult {
    *dirty = RECT_EMPTY;
    f.rows_dirty = RECT_EMPTY;
    if ph == 0 {
        f.on_keys = rect_contains(f.kb.area, ev.x as i32, ev.y as i32);
        f.has_pressed = false;
        if !f.on_keys {
            press_begin(&mut f.press, ev.x, ev.y, ev.t_ms);
            if let Some(h) = hits_find(&f.hits, ev.x, ev.y).copied() {
                f.pressed = h;
                f.has_pressed = true;
                app_press_feedback(a, h.r);
            }
            return FsResult::None;
        }
    } else if !f.on_keys {
        if !f.has_pressed {
            return FsResult::None;
        }
        press_move(&mut f.press, ev.x, ev.y);
        if ph == 1 {
            return FsResult::None;
        }
        // The lift: the row is drawn as it was, and a tap on it goes there.
        f.has_pressed = false;
        let h = f.pressed;
        canvas_invert(&mut a.screen, h.r);
        // (a press a second finger took over, the cancel, is let go with a Fast refresh as it was pressed)
        app_invalidate(a, h.r, if touch_cancelled(ev) { RefreshMode::Fast } else { RefreshMode::Ui });
        if press_is_tap(&f.press, h.r) && h.arg >= 0 && (h.arg as usize) < f.n && f.rows[h.arg as usize].page >= 0 {
            return FsResult::Go(f.rows[h.arg as usize].page);
        }
        return FsResult::None;
    }
    // The keyboard's.
    let (r, d) = app_keyboard_pointer_split(a, &mut f.kb, ph, ev);
    match r {
        KbResult::Cancel => FsResult::Close,
        KbResult::Done => {
            // The first row that is a button; with none the field stays as it was (the rows already say why).
            if let Some(row) = f.rows[..f.n].iter().find(|r| r.page >= 0) {
                return FsResult::Go(row.page);
            }
            f.kb.open = true;
            keyboard_draw(&mut f.kb, &mut a.screen, Rect { x: 0, y: 0, w: a.w, h: a.h });
            app_invalidate(a, f.kb.area, RefreshMode::Fast);
            FsResult::None
        }
        KbResult::Changed => {
            if !rect_is_empty(f.kb.field_dirty) {
                findsheet_update(f, a, nb, nav);
            }
            *dirty = d;
            FsResult::Changed
        }
        KbResult::None => {
            *dirty = d;
            FsResult::None
        }
    }
}

/// The finger that pressed a row is a pinch now: the row is drawn as it was. The rect to refresh (empty if none).
pub fn findsheet_cancel_press(f: &mut FindSheet, a: &mut App) -> Rect {
    let was = f.has_pressed;
    f.has_pressed = false;
    f.on_keys = false;
    if !was {
        return RECT_EMPTY;
    }
    canvas_invert(&mut a.screen, f.pressed.r);
    f.pressed.r
}

/// From the editor's tick while the sheet is open: once typing pauses the keyboard's grayscale refresh
/// (app_keyboard_tick), with the card of rows in it, so it too is cleaned of what the fast ones left.
pub fn findsheet_tick(f: &mut FindSheet, a: &mut App, now: u32) {
    if !f.open || !f.kb.open {
        return;
    }
    let due = f.kb.settle_at != 0 && (now.wrapping_sub(f.kb.settle_at) as i32) >= 0 && f.kb.pressed_key < 0;
    app_keyboard_tick(a, &mut f.kb, now);
    if due {
        app_invalidate(a, f.rect, RefreshMode::Ui);
    }
}

/// The runner's keys for the sheet, without "editor.find.": "rows" (how many rows it shows) and "N" (row N from
/// 1, as drawn: the name alone, or "Go to page 3"; empty past the last). false for another key.
pub fn findsheet_probe(f: &FindSheet, key: &str, out: &mut String, cap: usize) -> bool {
    if key == "rows" {
        csnprintf!(out, cap, "%d", if f.open { f.n as i32 } else { 0 });
    } else if !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()) {
        let i = key.parse::<usize>().unwrap_or(0);
        if f.open && i >= 1 && i <= f.n {
            csnprintf!(out, cap, "%s", f.rows[i - 1].label.as_str());
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else {
        return false;
    }
    true
}
