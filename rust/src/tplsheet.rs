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

/// how often a sheet waiting for previews looks for them, while a job runs
const CHECK_MS: u32 = 1500;

// ---- the paper pickers' layout ----

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
pub fn paper_grid(mut n: i32, avail_w: i32, avail_h: i32, gap: i32, label_h: i32, max_w: i32, min_cols: i32) -> PaperGrid {
    let mut best = PaperGrid::default();
    let mut best_empty = 0;
    if n < 1 {
        n = 1;
    }
    let mut cols = clampi(min_cols, 1, n);
    while cols <= n {
        let rows = (n + cols - 1) / cols;
        let mut tw = (avail_w - (cols - 1) * gap) / cols;
        if tw > max_w {
            tw = max_w;
        }
        let max_th = avail_h / rows - label_h; // a tile's height, its label under it
        if tw * 4 / 3 > max_th {
            tw = max_th * 3 / 4;
        }
        let empty = rows * cols - n;
        if tw < 1 || (best.tw != 0 && (tw < best.tw || (tw == best.tw && empty >= best_empty))) {
            cols += 1;
            continue;
        }
        best = PaperGrid { cols, rows, tw, th: tw * 4 / 3, cell_h: tw * 4 / 3 + label_h };
        best_empty = empty;
        cols += 1;
    }
    if best.tw == 0 {
        best = PaperGrid { cols: n, rows: 1, tw: 1, th: 1, cell_h: 1 + label_h }; // (a box with no room at all)
    }
    best
}
pub const PAPER_GRID_MIN_COLS: i32 = 6;
/// The editor's Paper sheet on this screen, and the grid of its PAPER_SHEET_EDITOR_TILES tiles (the
/// Templates sheet takes the same place and tiles).
pub fn paper_sheet_rect(a: &App, g: &mut PaperGrid) -> Rect {
    let m = ui_px(70.0);
    let gap = ui_px(30.0);
    // Beside the tiles: the title (120), the sheet's insets (72), its buttons (190), a margin (80).
    let avail_w = a.w - 2 * m - ui_px(72.0);
    let avail_h = a.h - ui_px(120.0) - ui_px(72.0) - ui_px(190.0) - ui_px(80.0);
    *g = paper_grid(PAPER_SHEET_EDITOR_TILES, avail_w, avail_h, gap, ui_px(100.0), a.w / 6, PAPER_GRID_MIN_COLS);
    let sheet_h = ui_px(120.0) + ui_px(72.0) + g.rows * g.cell_h + ui_px(190.0);
    Rect { x: m, y: (a.h - sheet_h) / 3, w: a.w - 2 * m, h: sheet_h }
}
/// A paper tile's label, under the tile (tile: its rect; gap: the grid's): on one line, centred,
/// or on two lines no wider than the tile when it is wider than the tile (a break after a comma
/// preferred: "Narrow," over "no margin"), so that labels side by side never run into each other.
/// In bold for the selected tile, on as many lines as it has in regular.
pub fn paper_tile_label(c: &mut Canvas, tile: Rect, gap: i32, label: &str, selected: bool) {
    let tr = tile;
    let lf = if selected { FontFace::Bold } else { FontFace::Regular };
    let ls = ui_small_size() * 0.9;
    // By its regular width, so that a label keeps its lines when its tile is selected (in bold).
    if font_measure(FontFace::Regular, ls, label) > tr.w {
        font_draw_wrapped(
            c,
            lf,
            ls,
            Rect { x: tr.x, y: tr.y + tr.h + ui_px(8.0), w: tr.w, h: ui_px(80.0) },
            label,
            2,
            ui_px(40.0),
            true,
            0,
        );
    } else {
        font_draw_centered(c, lf, ls, Rect { x: tr.x - gap / 2, y: tr.y + tr.h + ui_px(16.0), w: tr.w + gap, h: ui_px(64.0) }, label, 0);
    }
}
/// The More papers... tile's picture when it has none to show (the paper is no picture): a blank
/// page with the sign for more.
pub fn paper_tile_more(c: &mut Canvas, tile: Rect) {
    let tr = tile;
    canvas_fill(c, tr, 255);
    let s = tr.w * 2 / 5;
    ui_icon(c, IconKind::More, Rect { x: tr.x + (tr.w - s) / 2, y: tr.y + (tr.h - s) / 2, w: s, h: s }, 0);
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
    /// the button or tile pressed, shown pressed until the lift (app_press_feedback)
    pub pressed: Option<Rect>,
}

// ---- the sheet ----

// The templates the sheet offers as it opens (every one not bad), and whether any of them waits for
// a preview.
fn rebuild_shown(s: &mut TplSheet) {
    s.shown.clear();
    s.pending = false;
    for i in 0..s.t.len() {
        if s.t[i].bad {
            continue;
        }
        s.shown.push(i as i32);
        if !s.t[i].ready {
            s.pending = true;
        }
    }
}

// A shown template the previews job found it cannot use since the sheet opened keeps its tile, so
// that no tile moves under the pen, until the sheet opens again. How many it offers: the others.
fn usable(s: &TplSheet) -> i32 {
    let mut u = 0;
    for k in 0..s.shown.len() {
        u += !s.t[s.shown[k] as usize].bad as i32;
    }
    u
}

// Whether one of those still waits for its preview.
fn waiting(s: &TplSheet) -> bool {
    for k in 0..s.shown.len() {
        let t = &s.t[s.shown[k] as usize];
        if !t.bad && !t.ready {
            return true;
        }
    }
    false
}

// The offered template that is the paper now (an index into shown), or -1.
fn find_selected(s: &TplSheet) -> i32 {
    let by_paper = s.current_path.is_empty() && s.nb.is_some() && paper_is_image(s.current) && paper_is_document(s.current) == s.photos;
    for k in 0..s.shown.len() {
        let t = &s.t[s.shown[k] as usize];
        let mut kind = PaperKind::default();
        let found = match s.nb.as_deref() {
            Some(nb) if by_paper => {
                if s.photos {
                    photo_paper(t, nb, &s.ix, &mut kind)
                } else {
                    template_paper(t, nb, &s.ix, &mut kind)
                }
            }
            _ => -1,
        };
        let is_it = if !s.current_path.is_empty() { t.path.as_bytes() == s.current_path.as_bytes() } else { found == 0 && kind == s.current };
        if is_it {
            return k as i32;
        }
    }
    -1
}

/// Closes it and lets go of its list (also when it is closed already).
pub fn tplsheet_close(s: &mut TplSheet) {
    template_list_free(&mut s.t);
    s.shown = Vec::new();
    bgindex_free(&mut s.ix);
    s.open = false;
    s.selected = -1;
    s.picked = -1;
}

fn open_folder(s: &mut TplSheet, a: &mut App, nb: Option<&NotebookInfo>, current: PaperKind, current_path: Option<&str>, photos: bool) {
    tplsheet_close(s);
    *s = TplSheet {
        open: true,
        photos,
        nb: nb.map(|n| Box::new(n.clone())),
        current,
        selected: -1,
        picked: -1,
        gen: bgimage_generation(),
        ..Default::default()
    };
    if let Some(p) = current_path {
        s.current_path.set(p);
    }
    if (if photos { photos_scan(&mut s.t, a.dpi) } else { template_scan(&mut s.t, a.dpi) }) != 0 {
        s.t = Vec::new(); // (no folder: nothing to offer)
    }
    let mut shown = Vec::new();
    if shown.try_reserve_exact(s.t.len().max(1)).is_err() {
        // (no memory: nothing offered, rather than every file called unusable)
        template_list_free(&mut s.t);
        s.t = Vec::new();
    } else {
        s.shown = shown;
    }
    // In the editor, which template the page's paper is comes from the notebook's index of its
    // pictures (the page's paper is in the notebook, its file in the templates folder).
    let mut bgdir = String::new();
    if let Some(nb) = nb {
        if paper_is_image(current) && bgimage_dir(nb.id.as_str(), &mut bgdir, 700) == 0 && bgindex_load(&bgdir, &mut s.ix) != 0 {
            qlog!("templates: %s: bg/index.ini does not read; no template shows as this page's", &nb.id);
        }
    }
    rebuild_shown(s);
    s.selected = find_selected(s);
    s.anchor = if s.selected > 0 { s.selected } else { 0 };
    s.next_check = now_ms().wrapping_add(CHECK_MS);
}

/// Opens it on the templates as they are now (the previews job queued for those that have none):
/// on the page with the template that is the paper now, which is selected. In the editor that is
/// the open page's paper `current` (an image paper of notebook nb's); in New notebook (nb NULL) the
/// file current_path (NULL or "" for none).
pub fn tplsheet_open(s: &mut TplSheet, a: &mut App, nb: Option<&NotebookInfo>, current: PaperKind, current_path: Option<&str>) {
    open_folder(s, a, nb, current, current_path, false);
}

/// The same on the photos in grimoire/photos/ (the editor's Photos...): the one that is the open page's
/// paper now is selected; the host installs the one picked with photo_install.
pub fn tplsheet_open_photos(s: &mut TplSheet, a: &mut App, nb: &NotebookInfo, current: PaperKind) {
    open_folder(s, a, Some(nb), current, None, true);
}

// A template's name under its tile, on one line no wider than the tile: centred, or cut short as
// names are (the fit check does not hear of a name, which is the owner's).
fn tile_name(c: &mut Canvas, tr: Rect, name: &str, selected: bool) {
    let f = if selected && ui_theme() != UiTheme::Gothic { FontFace::Bold } else { FontFace::Regular }; // (as the cards' names)
    let px = ui_small_size() * 0.9;
    let mut lr = Rect { x: tr.x, y: tr.y + tr.h + ui_px(16.0), w: tr.w, h: ui_px(64.0) };
    let w = font_measure(f, px, name);
    if w < lr.w {
        lr = Rect { x: lr.x + (lr.w - w) / 2, y: lr.y, w: w + 1, h: lr.h };
    }
    font_draw_left_name(c, f, px, lr, name, 0);
}

// A tile whose preview is not made yet: the sign for more, in gray, until it is.
fn tile_waiting(c: &mut Canvas, tr: Rect) {
    let s = tr.w / 4;
    ui_icon(c, IconKind::More, Rect { x: tr.x + (tr.w - s) / 2, y: tr.y + (tr.h - s) / 2, w: s, h: s }, 170);
}

// A tile whose template the previews job found it cannot use since the sheet opened: a cross, in
// gray. It cannot be picked, and is gone when the sheet opens again.
fn tile_unusable(c: &mut Canvas, tr: Rect) {
    let s = tr.w / 4;
    ui_icon(c, IconKind::Close, Rect { x: tr.x + (tr.w - s) / 2, y: tr.y + (tr.h - s) / 2, w: s, h: s }, 170);
}

// A paragraph of the empty state at y, centred, on the lines it takes (at most max, line px
// apart): the y under it, so that every paragraph starts as far under the one before it.
fn paragraph(c: &mut Canvas, px: f32, w: Rect, y: i32, text: &str, max: i32, line: i32) -> i32 {
    let n = clampi(font_wrap_lines(FontFace::Regular, px, text, w.w, None), 1, max);
    font_draw_wrapped(c, FontFace::Regular, px, Rect { x: w.x, y, w: w.w, h: n * line }, text, max, line, true, 0);
    y + n * line
}

// No template to offer: how to add some.
fn draw_empty(s: &TplSheet, a: &mut App, r: Rect) {
    let c = &mut a.screen;
    let pad = ui_px(40.0);
    let mut y = r.y + ui_px(40.0);
    let w = Rect { x: r.x + pad, y: 0, w: r.w - 2 * pad, h: 0 };
    // TRANSLATORS: the heading of the Templates sheet when grimoire/templates has no picture to
    // offer (1560 px at 64 px in bold, sideways 2180).
    let mut heading = tr("No templates yet");
    if s.photos {
        // TRANSLATORS: the same for the Photos sheet, when grimoire/photos has no photo (as above).
        heading = tr("No photos yet");
    }
    font_draw_centered(c, FontFace::Bold, ui_title_size(), Rect { x: w.x, y, w: w.w, h: ui_px(100.0) }, heading, 0);
    y += ui_px(150.0);
    let mut text = String::new();
    if s.photos {
        // TRANSLATORS: under it, what to do, centred (up to 4 lines of 1560 px at 44 px, sideways
        // 2180); %s is the folder on the Kindle's storage.
        csnprintf!(&mut text, 640, tr("Copy JPG or PNG photos into the %s folder over USB while Grimoire is closed."), "grimoire/photos");
    } else {
        // TRANSLATORS: under it, what to do, centred (up to 4 lines of 1560 px at 44 px, sideways
        // 2180); %s is the folder on the Kindle's storage.
        csnprintf!(&mut text, 640, tr("Copy PNG or JPG pictures into the %s folder over USB while Grimoire is closed."), "grimoire/templates");
    }
    let ts = ui_text_size();
    let line = ui_px(62.0);
    y = paragraph(c, ts, w, y, &text, 4, line) + ui_px(40.0);
    // TRANSLATORS: the paragraph after it, centred too (up to 4 lines of 1560 px at 44 px, sideways
    // 2180); README.txt is the file Grimoire wrote in that folder.
    let mut more = tr("Upright pictures in the shape of a page (3:4) work best, ideally 1860 x 2480 pixels. The \
README.txt there says more.");
    // TRANSLATORS: the same for the Photos sheet (as above): a photo picked becomes the open page's
    // paper. "Pages" is the editor's Pages view, as translated there.
    if s.photos {
        more = tr("A photo becomes the open page's paper, upright and fitted to the page. For a page of its own, add \
a new page in Pages first.");
    }
    y = paragraph(c, ts, w, y, more, 4, line) + ui_px(40.0);
    let mut bad = 0;
    for i in 0..s.t.len() {
        bad += s.t[i].bad as i32;
    }
    if bad != 0 {
        csnprintf!(
            &mut text,
            640,
            // TRANSLATORS: under them, when the folder holds files Grimoire cannot use (a picture that
            // does not read, or too big): how many; %s is the log file, which says which (by number). Up
            // to 2 lines of 1560 px at 34 px, sideways 2180.
            ng_("Grimoire cannot use %d file there (see %s).", "Grimoire cannot use %d files there (see %s).", bad as i64),
            bad,
            "grimoire.log"
        );
        paragraph(c, ui_small_size(), w, y, &text, 2, line);
    }
}

/// draws into a->screen, returns its rect
pub fn tplsheet_draw(s: &mut TplSheet, a: &mut App) -> Rect {
    hits_clear(&mut s.hits);
    s.pressed = None;
    let mut g = PaperGrid::default();
    s.rect = paper_sheet_rect(a, &mut g);
    let c = &mut a.screen;
    // TRANSLATORS: the title of the sheet of templates (pictures copied into grimoire/templates, to
    // write on as paper), which More papers... opens (1560 px at 64 px).
    let mut title = tr("Templates");
    if s.photos {
        // TRANSLATORS: the title of the sheet of photos (copied into grimoire/photos), which Photos...
        // opens (1560 px at 64 px).
        title = tr("Photos");
    }
    let in_ = ui_sheet(c, s.rect, title);
    let gap = ui_px(30.0);
    let bh = ui_px(140.0);
    let offered = usable(s);
    s.per = g.cols * g.rows;
    s.pages = if offered != 0 { (s.shown.len() as i32 + s.per - 1) / s.per } else { 1 };
    s.page = clampi(s.anchor / s.per, 0, s.pages - 1);
    let back = Rect { x: in_.x, y: in_.y + in_.h - bh, w: ui_px(380.0), h: bh };
    // TRANSLATORS: the Templates sheet's button back to the papers, under its icon (348 px at
    // 34 px).
    ui_button(c, back, IconKind::Back, tr("Back"), ButtonStyle::Plain);
    hits_add(&mut s.hits, back, TS_HIT_BACK, 0);
    // Only the previews of the tiles on screen stay in memory (each about 130 KB).
    let first = s.page * s.per;
    for k in 0..s.shown.len() as i32 {
        if k < first || k >= first + s.per {
            canvas_free(&mut s.t[s.shown[k as usize] as usize].thumb);
        }
    }
    if offered == 0 {
        draw_empty(s, a, Rect { x: in_.x, y: in_.y, w: in_.w, h: back.y - in_.y });
        return s.rect;
    }
    let gridw = g.cols * g.tw + (g.cols - 1) * gap;
    let mut k = 0;
    while k < s.per && first + k < s.shown.len() as i32 {
        let i = first + k;
        let t = &mut s.t[s.shown[i as usize] as usize];
        let tr = Rect { x: in_.x + (in_.w - gridw) / 2 + (k % g.cols) * (g.tw + gap), y: in_.y + (k / g.cols) * g.cell_h, w: g.tw, h: g.th };
        canvas_fill(c, tr, 255);
        let pv = if t.bad { None } else { template_thumb(t) };
        let have = pv.is_some();
        if let Some(pv) = pv {
            canvas_blit_fit(c, pv, tr);
        } else if t.bad {
            tile_unusable(c, tr);
        } else {
            tile_waiting(c, tr);
            s.pending = true; // (also a preview gone since the scan: made again, then shown)
        }
        let sel = i == s.selected;
        ui_frame(c, rect_inflate(tr, if sel { ui_px(10.0) } else { 0 }), ui_px(12.0), 0, false, if sel { ui_px(9.0) } else { ui_px(3.0) });
        tile_name(c, tr, t.name.as_str(), sel);
        if have {
            hits_add(&mut s.hits, Rect { x: tr.x, y: tr.y, w: tr.w, h: tr.h + ui_px(90.0) }, TS_HIT_TILE, i); // (none waiting, unusable)
        }
        k += 1;
    }
    if s.pages > 1 {
        let nr = Rect { x: in_.x + in_.w - ui_px(220.0), y: back.y, w: ui_px(220.0), h: bh };
        let lr = Rect { x: nr.x - ui_px(440.0), y: back.y, w: ui_px(440.0), h: bh };
        let pr = Rect { x: lr.x - ui_px(220.0), y: back.y, w: ui_px(220.0), h: bh };
        ui_button(c, pr, IconKind::Prev, "", if s.page > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        ui_button(c, nr, IconKind::Next, "", if s.page < s.pages - 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        if s.page > 0 {
            hits_add(&mut s.hits, pr, TS_HIT_PREV, 0);
        }
        if s.page < s.pages - 1 {
            hits_add(&mut s.hits, nr, TS_HIT_NEXT, 0);
        }
        let mut pl = String::new();
        // TRANSLATORS: (also) between the Templates sheet's page buttons, in bold: 440 px at 44 px.
        csnprintf!(&mut pl, 96, tr("Page %d of %d"), s.page + 1, s.pages);
        font_draw_centered(c, FontFace::Bold, ui_text_size(), lr, &pl, 0);
    }
    s.rect
}

/// Pointer phase 0 down / 1 move / 2 up. TS_PICKED (a template tapped), TS_BACK (its Back button)
/// and TS_CANCEL (a tap outside it) close it: the host recomposes *dirty, and after TS_PICKED reads
/// tplsheet_path (and tplsheet_preview), then calls tplsheet_close. TS_CHANGED: another page of
/// tiles, drawn; refresh *dirty.
pub fn tplsheet_pointer(s: &mut TplSheet, a: &mut App, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> TsResult {
    *dirty = RECT_EMPTY;
    if !s.open {
        return TsResult::None;
    }
    if phase == 0 {
        press_begin(&mut s.press, x, y, t);
        if let Some(&h) = hits_find(&s.hits, x, y).filter(|_| rect_contains(s.rect, x as i32, y as i32)) {
            app_press_feedback(a, h.r);
            s.pressed = Some(h.r);
        }
        return TsResult::None;
    }
    press_move(&mut s.press, x, y);
    if phase != 2 {
        return TsResult::None;
    }
    *dirty = app_press_release(a, &mut s.pressed);
    if !rect_contains(s.rect, s.press.x0 as i32, s.press.y0 as i32) {
        *dirty = s.rect; // a tap outside the sheet cancels
        tplsheet_close(s);
        return TsResult::Cancel;
    }
    let Some(&h) = hits_find(&s.hits, s.press.x0, s.press.y0) else {
        return TsResult::None;
    };
    if !press_is_tap(&s.press, h.r) {
        return TsResult::None;
    }
    match h.id {
        TS_HIT_BACK => {
            *dirty = s.rect;
            tplsheet_close(s);
            TsResult::Back
        }
        TS_HIT_TILE => {
            if h.arg < 0 || h.arg >= s.shown.len() as i32 {
                return TsResult::None;
            }
            s.picked = h.arg;
            s.open = false; // (the list stays for tplsheet_path until tplsheet_close)
            *dirty = s.rect;
            TsResult::Picked
        }
        TS_HIT_PREV | TS_HIT_NEXT => {
            s.anchor = clampi(s.page + if h.id == TS_HIT_NEXT { 1 } else { -1 }, 0, s.pages - 1) * s.per;
            tplsheet_draw(s, a);
            *dirty = s.rect;
            TsResult::Changed
        }
        _ => TsResult::None,
    }
}

/// From the host's tick while it is open: previews made since (looked for every 1.5 s while a
/// background job runs, and when one ends) are drawn in their tiles. True with *dirty to refresh.
pub fn tplsheet_tick(s: &mut TplSheet, a: &mut App, now: u32, dirty: &mut Rect) -> bool {
    *dirty = RECT_EMPTY;
    if !s.open || !s.pending {
        return false;
    }
    // A background job that ended (the previews job among them) says so by bgimage_generation;
    // while one runs, the previews it made so far are looked for now and then.
    let ended = s.gen != bgimage_generation();
    if !ended && !(bg_busy() && (now.wrapping_sub(s.next_check) as i32) >= 0) {
        return false;
    }
    s.gen = bgimage_generation();
    s.next_check = now.wrapping_add(CHECK_MS);
    if template_refresh(&mut s.t) == 0 {
        return false;
    }
    s.pending = waiting(s); // (a template found unusable keeps its tile: usable)
    s.selected = find_selected(s);
    tplsheet_draw(s, a);
    *dirty = s.rect;
    true
}

/// After TS_PICKED, until tplsheet_close: the picked template's file, and its preview. (The C's
/// NULL, when nothing is picked, is the empty string: no template has an empty path, and the hosts
/// read a NULL as "".)
pub fn tplsheet_path(s: &TplSheet) -> &str {
    if !s.t.is_empty() && s.picked >= 0 && (s.picked as usize) < s.shown.len() {
        s.t[s.shown[s.picked as usize] as usize].path.as_str()
    } else {
        ""
    }
}
pub fn tplsheet_preview(s: &mut TplSheet) -> Option<&Canvas> {
    if !s.t.is_empty() && s.picked >= 0 && (s.picked as usize) < s.shown.len() {
        template_thumb(&mut s.t[s.shown[s.picked as usize] as usize])
    } else {
        None
    }
}

/// The headless runner's "templates.*" keys (key without the prefix; docs/SIM.md): count (the
/// templates it offers), ready (how many of those have their previews), page and pages (of tiles,
/// from 1), selected (the selected one's name). Empty values while it is closed. 0, -1 for a key
/// it does not know.
pub fn tplsheet_probe(s: &TplSheet, key: &str, out: &mut String, cap: usize) -> i32 {
    let open = s.open;
    let mut ready = 0;
    if open {
        for k in 0..s.shown.len() {
            ready += s.t[s.shown[k] as usize].ready as i32;
        }
    }
    match key {
        "count" => {
            if open {
                csnprintf!(out, cap, "%d", usable(s));
            } else {
                csnprintf!(out, cap, "%s", "");
            }
        }
        "ready" => {
            if open {
                csnprintf!(out, cap, "%d", ready);
            } else {
                csnprintf!(out, cap, "%s", "");
            }
        }
        "page" => {
            if open {
                csnprintf!(out, cap, "%d", s.page + 1);
            } else {
                csnprintf!(out, cap, "%s", "");
            }
        }
        "pages" => {
            if open {
                csnprintf!(out, cap, "%d", s.pages);
            } else {
                csnprintf!(out, cap, "%s", "");
            }
        }
        "selected" => {
            csnprintf!(out, cap, "%s", if open && s.selected >= 0 { s.t[s.shown[s.selected as usize] as usize].name.as_str() } else { "" });
        }
        _ => return -1,
    }
    0
}
