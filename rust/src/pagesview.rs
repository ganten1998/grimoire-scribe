//! pagesview.h: the editor's Pages view, which covers the screen: numbered tiles a screenful at a
//! time (it never reads or draws another page, so it opens at once in a notebook of any length),
//! ticks for Delete, a number pad for Go to page..., New page and Copy page. A page that holds a
//! book page (a notebook annotated from KOReader, annotate.h) says which under its number ("p. 12"),
//! from the notebook's source.ini, read as the view opens.
//!
//! A modal hosted by the editor, which keeps the notebook: the view draws itself and takes the taps,
//! and says what was asked (PvResult); the editor does it (opens a page, deletes the ticked ones).
//!
//! It also picks where a link drawn on a page goes (pagesview_pick): then a tile, an Index row or a
//! page number picks that page, and there are no ticks, nor New page or Copy page. And it opens with
//! its naming keyboard up for the open page (pagesview_name), for a shortcut pressed on the page.

use crate::prelude::*;

const PG_BACK: i32 = 1;
const PG_DELETE: i32 = 2;
const PG_ALL: i32 = 3;
const PG_TOGGLE: i32 = 4;
const PG_OPEN: i32 = 5;
const PG_PREV: i32 = 6;
const PG_NEXT: i32 = 7;
const PG_JUMP: i32 = 8;
const PG_NEW: i32 = 9;
const PG_COPY: i32 = 10;
const PG_TAB: i32 = 11;
const PG_NAME: i32 = 12;
const PG_LINK: i32 = 13;
const PG_TO_INDEX: i32 = 14;
const PG_UNLINK: i32 = 15;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PvResult {
    #[default]
    None = 0, // PV_NONE
    /// its Back button: the view closes on the page that was open
    Close = 1, // PV_CLOSE
    /// a tile tapped, or a page number entered: the view closes on page `target`
    Open = 2, // PV_OPEN
    /// Delete: the editor deletes the ticked pages (pagesview_ticked), the view stays
    Delete = 3, // PV_DELETE
    /// New page: the editor adds a page at index `target` (after the last page ticked, or after
    /// the open page: never first) and opens it
    New = 4, // PV_NEW
    /// Copy page (one page ticked): the editor copies page `target` right after it
    Copy = 5, // PV_COPY
    /// save kb.text as the open page's section name; empty removes it (then, when name_only, the
    /// view closes on the page, as on PV_CLOSE)
    Name = 6, // PV_NAME
    /// Add a link (the Index tab): the editor waits for the link's box on the open page
    Link = 7, // PV_LINK
    // While picking (pagesview_pick); its Back button says PV_CLOSE:
    /// the link goes to page `target`
    Pick = 8, // PV_PICK
    /// ... to the Index
    PickIndex = 9, // PV_PICK_INDEX
    /// Remove link (a link the page had)
    Unlink = 10, // PV_UNLINK
    /// Go to page...: the view closes and the Find sheet opens over the page (Rust only: the number pad it used
    /// to open is kept for picking a link's page, where Find has no mode yet)
    GoTo = 11,
}

/// (The C keeps pointers to the editor's notebook and Nav, nb and nav; here the editor passes them
/// to each call that reads them: pagesview_draw, pagesview_pointer, pagesview_probe. PORTING.md,
/// Ownership.)
#[derive(Clone, Debug, Default)]
pub struct PagesView {
    pub open: bool,
    /// the open page's index; the editor keeps it up to date
    pub current: i32,
    /// Go to page...'s number pad (while a link's page is picked; else the Find sheet is Go to page...)
    pub kb: Keyboard,
    /// Go to page... opens the number pad as the C build's does, instead of leaving for the Find sheet: the app
    /// never sets it; the test that holds the view to the C build's recorded behaviour does
    pub jump_pad: bool,
    /// which screenful of tiles
    pub screen: i32,
    /// ticked, per page index (the C's sel and sel_cap)
    pub sel: Vec<bool>,
    pub hits: HitList,
    pub press: Press,
    /// after PV_OPEN, PV_NEW, PV_COPY: a page index (see PvResult)
    pub target: i32,
    /// source.ini: which page holds which book page (empty for most)
    pub book: AnnotateMap,
    pub index_tab: bool,
    pub tab_initialized: bool,
    pub naming: bool,
    pub index_screen: i32,
    /// picking where a link goes (pagesview_pick)
    pub picking: bool,
    /// ... a link the page had (it can be removed), not one drawn just now
    pub pick_had: bool,
    /// opened to name the open page (pagesview_name): its keyboard's
    /// Cancel says PV_CLOSE, and after its Done the editor closes the view
    pub name_only: bool,
    /// ... index_tab and tab_initialized as it found them, given back as it closes
    pub name_back_tab: bool,
    pub name_back_init: bool,
    /// The C's `v->nav != NULL`: an open (or pick, or name) set it, pagesview_free clears it.
    /// pagesview_probe's index_rows answers 0 until then (the probe gets the editor's Nav on every
    /// call here, where the C's view had none before it was first opened).
    pub nav_set: bool,
}


/// (cols, rows, cw, ch, th)
fn geom(a: &App) -> (i32, i32, i32, i32, i32) {
    let cols = if a.w > a.h { 6 } else { 4 };
    let m = ui_px(50.0);
    let cw = (a.w - m * (cols + 1)) / cols;
    let th = ui_px(200.0);
    let ch = th + ui_px(90.0);
    let mut rows = (a.h - ui_px(190.0) - ui_px(170.0) - m) / (ch + m);
    if rows < 1 {
        rows = 1;
    }
    (cols, rows, cw, ch, th)
}

// How many of the first npages pages are ticked. (The C's pagesview_ticked_count reads the editor's
// live nb->npages; the view draws and takes taps with nb in hand, so those use this one.)
fn ticked_in(v: &PagesView, npages: i32) -> i32 {
    let mut k = 0;
    let mut i = 0;
    while i < npages && (i as usize) < v.sel.len() {
        k += v.sel[i as usize] as i32;
        i += 1;
    }
    k
}

/// (The C counts the first nb->npages entries, and the editor's nb is not kept here. Every entry
/// past the notebook's pages is false: a tick is only set on a page tile or by Select all, and
/// untick clears all of them after the pages deleted, so counting every entry is the same number.)
pub fn pagesview_ticked_count(v: &PagesView) -> i32 {
    ticked_in(v, v.sel.len() as i32)
}

pub fn pagesview_ticked(v: &PagesView, index: i32) -> bool {
    index >= 0 && (index as usize) < v.sel.len() && v.sel[index as usize]
}

pub fn pagesview_untick(v: &mut PagesView) {
    v.sel.fill(false);
}

// A view opened to name the open page (name_only) gives back the tab it found: the Index under its
// keys was not picked there, so the Pages button and the pages shortcut open where they did before.
fn end_name_only(v: &mut PagesView) {
    if v.name_only {
        v.index_tab = v.name_back_tab;
        v.tab_initialized = v.name_back_init;
    }
    v.name_only = false;
}

/// Opens it on notebook nb, on the screenful with page `current`, nothing ticked. False when there
/// is no memory for its ticks (it stays closed).
pub fn pagesview_open(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32) -> bool {
    v.nav_set = true;
    end_name_only(v);
    if !v.tab_initialized {
        v.index_tab = nav_index(nav, nb, &mut []) > 0;
        v.index_screen = 0;
        v.tab_initialized = true;
    }
    v.current = current;
    v.picking = false;
    v.pick_had = false;
    let npages = nb.npages() as usize;
    if v.sel.len() < npages {
        if v.sel.try_reserve_exact(npages - v.sel.len()).is_err() {
            return false;
        }
        v.sel.resize(npages, false);
    }
    pagesview_untick(v);
    keyboard_close(&mut v.kb);
    annotate_map_free(&mut v.book);
    annotate_map_load(nb.id.as_str(), &mut v.book); // (no labels if it is not there or does not read)
    v.open = true;
    let (cols, rows, ..) = geom(a);
    v.screen = current / (cols * rows);
    true
}

/// The same to pick where a link on the open page goes: had, a link it had (else one drawn now).
pub fn pagesview_pick(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32, had: bool) -> bool {
    if !pagesview_open(v, a, nb, nav, current) {
        return false;
    }
    if nav_index(nav, nb, &mut []) == 0 {
        v.index_tab = false; // an Index with nothing to pick
    }
    v.picking = true;
    v.pick_had = had;
    true
}

// The naming keyboard for the open page, holding its name (Name this page, pagesview_name).
fn open_naming(v: &mut PagesView, nb: &NotebookInfo, nav: &Nav) {
    v.naming = true;
    // TRANSLATORS: the naming keyboard's title, 96 px text-field header.
    keyboard_open(&mut v.kb, tr("Page name"), nav_name(nav, nb.page_ids[v.current as usize]).as_bytes());
}

/// The same on its Index tab with the naming keyboard up for the open page, as Name this page opens
/// it, to go back to the page after it (name_only). False also when the names cannot be changed (a
/// nav.ini that does not read: that button is greyed out then).
pub fn pagesview_name(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32) -> bool {
    if !nav.writable {
        return false;
    }
    let (back_tab, back_init) = (v.index_tab, v.tab_initialized); // (before an opening picks one)
    if !pagesview_open(v, a, nb, nav, current) {
        return false;
    }
    v.name_back_tab = back_tab;
    v.name_back_init = back_init;
    v.index_tab = true; // (Name this page's tab: its names above the keys)
    v.tab_initialized = true;
    v.name_only = true;
    open_naming(v, nb, nav);
    true
}

/// Closes it (and its number pad). Its ticks' memory stays for the next open.
pub fn pagesview_close(v: &mut PagesView) {
    v.open = false;
    v.picking = false;
    v.pick_had = false;
    end_name_only(v);
    keyboard_close(&mut v.kb);
    annotate_map_free(&mut v.book);
}

pub fn pagesview_free(v: &mut PagesView) {
    annotate_map_free(&mut v.book);
    v.sel = Vec::new();
    v.tab_initialized = false;
    v.nav_set = false;
}

// The book page page pid holds, or 0.
fn book_page(v: &PagesView, pid: u32) -> i32 {
    for e in &v.book.e {
        if e.pid == pid {
            return e.page;
        }
    }
    0
}

// Tile i's label: its page number (and that it is the open page), and on a second line (book, ""
// for none) the book page it holds.
fn tile_label(v: &PagesView, nb: &NotebookInfo, nav: &Nav, i: i32, lab: &mut String, cap: usize, book: &mut String, bcap: usize) {
    // TRANSLATORS: under each page's picture in the Pages view (436 px at 44 px sideways), the
    // open page in bold.
    csnprintf!(lab, cap, if i == v.current { tr("Page %d (open)") } else { tr("Page %d") }, i + 1);
    let name = nav_name(nav, nb.page_ids[i as usize]);
    if !name.is_empty() {
        i18n_copy(lab, name, cap);
    }
    let bp = book_page(v, nb.page_ids[i as usize]);
    if bp != 0 {
        // TRANSLATORS: under a page's label in the Pages view, the page of the book it holds (a page
        // annotated from KOReader): "p. 12". 355 px at 34 px.
        csnprintf!(book, bcap, tr("p. %d"), bp);
    } else {
        book.clear();
    }
}

// The last page ticked, or -1.
fn last_ticked(v: &PagesView, nb: &NotebookInfo) -> i32 {
    let mut last = -1;
    let mut i = 0;
    while i < nb.npages() && (i as usize) < v.sel.len() {
        if v.sel[i as usize] {
            last = i;
        }
        i += 1;
    }
    last
}

fn index_rows(a: &App) -> i32 {
    let rows = (a.h - ui_px(410.0)) / ui_px(140.0);
    if rows > 0 {
        rows
    } else {
        1
    }
}

fn draw_index(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, total: i32) {
    if total == 0 {
        // TRANSLATORS: empty Index tab, body text with nearly the screen's width.
        let r = Rect { x: ui_px(80.0), y: ui_px(350.0), w: a.w - ui_px(160.0), h: ui_px(200.0) };
        ui_text(
            &mut a.screen,
            FontFace::Regular,
            ui_text_size(),
            ui_small_size(),
            r,
            if nav.writable { tr("Name a page to list it here.") } else { tr("Page names could not be read.") },
            3,
            true,
            0,
        );
        return;
    }
    let mut indices = [0i32; STORE_MAX_PAGES];
    nav_index(nav, nb, &mut indices);
    let per = index_rows(a);
    let section = nav_section_of(nav, nb, v.current);
    let mut row = 0;
    while row < per && v.index_screen * per + row < total {
        let i = indices[(v.index_screen * per + row) as usize];
        let r = Rect { x: ui_px(50.0), y: ui_px(240.0) + row * ui_px(140.0), w: a.w - ui_px(100.0), h: ui_px(128.0) };
        let c = &mut a.screen;
        if i == section {
            canvas_fill(c, Rect { x: r.x, y: r.y + ui_px(12.0), w: ui_px(10.0), h: r.h - ui_px(24.0) }, 0);
        }
        let mut name = String::new();
        let title = ui_name_label(nav_name(nav, nb.page_ids[i as usize]), &mut name, 160);
        font_draw_left_name(
            c,
            if i == section { FontFace::Bold } else { FontFace::Regular },
            ui_text_size(),
            Rect { x: r.x + ui_px(30.0), y: r.y, w: r.w - ui_px(250.0), h: r.h },
            title,
            0,
        );
        let mut number = String::new();
        csnprintf!(&mut number, 32, "%d", i + 1);
        font_draw_centered(c, FontFace::Bold, ui_text_size(), Rect { x: r.x + r.w - ui_px(200.0), y: r.y, w: ui_px(180.0), h: r.h }, &number, 0);
        canvas_hline(c, r.x + ui_px(30.0), r.w - ui_px(30.0), r.y + r.h, ui_px(2.0), 160);
        hits_add(&mut v.hits, r, PG_OPEN, i);
        row += 1;
    }
}

/// Draws the whole screen into a->screen (the caller refreshes it).
pub fn pagesview_draw(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) {
    hits_clear(&mut v.hits);
    let (aw, ah) = (a.w, a.h);
    let c = &mut a.screen;
    canvas_fill(c, Rect { x: 0, y: 0, w: aw, h: ah }, 255);
    let m = ui_px(50.0);
    let th0 = ui_px(190.0);
    let bh = ui_px(150.0);
    let by = (th0 - bh) / 2;
    let back = Rect { x: m, y: by, w: bh, h: bh };
    ui_button(c, back, IconKind::Back, "", ButtonStyle::Plain);
    hits_add(&mut v.hits, back, PG_BACK, 0);
    let k = ticked_in(v, nb.npages());
    let mut del = String::new();
    if k != 0 {
        // TRANSLATORS: the Pages view's buttons, each under its icon (322 px at 34 px in Gothic, 344 in
        // Classic): Delete, with how many pages are ticked; tick all of them, or none.
        csnprintf!(&mut del, 64, tr("Delete (%d)"), k);
    } else {
        csnprintf!(&mut del, 64, "%s", tr("Delete"));
    }
    let rdel = Rect { x: aw - m - ui_px(360.0), y: by, w: ui_px(360.0), h: bh };
    let rall = Rect { x: rdel.x - ui_px(20.0) - ui_px(360.0), y: by, w: ui_px(360.0), h: bh };
    let all = k == nb.npages();
    // TRANSLATORS: the Pages view's button that ticks every page, or none (328 px at 34 px in
    // Gothic, 344 in Classic).
    if v.picking {
        // Where a link goes: the Index, or (one the page had) nowhere any more.
        let to = if v.pick_had { rall } else { Rect { x: rall.x, y: by, w: rdel.x + rdel.w - rall.x, h: bh } };
        // TRANSLATORS: while picking where a link drawn on a page goes, the button that sends it to
        // the notebook's Index (the list of named pages): 322 px at 34 px under its icon, or about
        // 740 px when Remove link is not beside it.
        ui_button(c, to, IconKind::Link, tr("The Index"), ButtonStyle::Plain);
        hits_add(&mut v.hits, to, PG_TO_INDEX, 0);
        if v.pick_had {
            // TRANSLATORS: while picking where a link goes, the button that removes the link (the
            // ink drawn around it stays): 322 px at 34 px under its icon.
            ui_button(c, rdel, IconKind::Trash, tr("Remove link"), ButtonStyle::Plain);
            hits_add(&mut v.hits, rdel, PG_UNLINK, 0);
        }
    } else if v.index_tab {
        let name = Rect { x: rall.x, y: by, w: rdel.x + rdel.w - rall.x, h: bh };
        // TRANSLATORS: Index tab header button (about 740 px wide, 44 px text).
        ui_button(
            c,
            name,
            IconKind::None,
            if !nav_name(nav, nb.page_ids[v.current as usize]).is_empty() { tr("Rename this page") } else { tr("Name this page") },
            if nav.writable { ButtonStyle::Plain } else { ButtonStyle::Disabled },
        );
        if nav.writable {
            hits_add(&mut v.hits, name, PG_NAME, 0);
        }
    } else {
        ui_button(c, rdel, IconKind::Trash, &del, if k != 0 { ButtonStyle::Primary } else { ButtonStyle::Disabled });
        if k != 0 {
            hits_add(&mut v.hits, rdel, PG_DELETE, 0);
        }
        ui_button(c, rall, IconKind::Check, if all { tr("Select none") } else { tr("Select all") }, ButtonStyle::Plain);
        hits_add(&mut v.hits, rall, PG_ALL, 0);
    }
    let mut title = String::new();
    let tx = back.x + bh + ui_px(30.0);
    let gap = ui_px(16.0);
    let tw = (rall.x - tx - ui_px(20.0) - gap) / 2;
    let pages_tab = Rect { x: tx, y: by, w: tw, h: bh };
    let index_tab = Rect { x: tx + tw + gap, y: by, w: tw, h: bh };
    // TRANSLATORS: Pages tab, with the notebook's page count (about 400 px wide, 44 px text).
    csnprintf!(&mut title, 64, tr("Pages (%d)"), nb.npages());
    ui_button(c, pages_tab, IconKind::None, &title, if v.index_tab { ButtonStyle::Plain } else { ButtonStyle::Selected });
    let named = nav_index(nav, nb, &mut []);
    // TRANSLATORS: Index tab, with the number of named sections (about 400 px, 44 px text).
    csnprintf!(&mut title, 64, tr("Index (%d)"), named);
    ui_button(c, index_tab, IconKind::None, &title, if v.index_tab { ButtonStyle::Selected } else { ButtonStyle::Plain });
    hits_add(&mut v.hits, pages_tab, PG_TAB, 0);
    hits_add(&mut v.hits, index_tab, PG_TAB, 1);
    ui_rule(c, 0, aw, th0, ui_px(4.0), 0);
    let (cols, rows, cw, ch, th) = geom(a);
    let per = cols * rows;
    let mut screens = (nb.npages() + per - 1) / per;
    if v.index_tab {
        screens = (named + index_rows(a) - 1) / index_rows(a);
        if screens < 1 {
            screens = 1;
        }
        v.index_screen = clampi(v.index_screen, 0, screens - 1);
        draw_index(v, a, nb, nav, named);
    }
    if screens < 1 {
        screens = 1;
    }
    if !v.index_tab {
        v.screen = clampi(v.screen, 0, screens - 1);
    }
    let c = &mut a.screen;
    let mut n = 0;
    while !v.index_tab && n < per {
        let i = v.screen * per + n;
        if i >= nb.npages() {
            break;
        }
        let tr_ = Rect { x: m + (n % cols) * (cw + m), y: th0 + m + (n / cols) * (ch + m), w: cw, h: th };
        let mut number = String::new();
        csnprintf!(&mut number, 24, "%d", i + 1);
        font_draw_centered(c, FontFace::Bold, ui_title_size() * 1.4, tr_, &number, 0);
        let cur = i == v.current;
        let sel = (i as usize) < v.sel.len() && v.sel[i as usize];
        ui_frame(c, rect_inflate(tr_, if cur { ui_px(8.0) } else { 0 }), ui_px(12.0), 0, false, if cur { ui_px(8.0) } else { ui_px(3.0) });
        let cb = Rect { x: tr_.x + tr_.w - ui_px(96.0), y: tr_.y + ui_px(16.0), w: ui_px(80.0), h: ui_px(80.0) };
        if !v.picking {
            ui_frame(c, cb, ui_px(12.0), if sel { 0 } else { 255 }, true, 0);
            ui_frame(c, cb, ui_px(12.0), 0, false, ui_px(5.0));
            if sel {
                ui_icon(c, IconKind::Check, cb, 255);
            }
        }
        let (mut lab, mut book) = (String::new(), String::new());
        tile_label(v, nb, nav, i, &mut lab, NAV_NAME_LEN, &mut book, 32);
        let name = nav_name(nav, nb.page_ids[i as usize]);
        if !name.is_empty() {
            let mut display = String::new();
            font_draw_left_name(
                c,
                if cur { FontFace::Bold } else { FontFace::Regular },
                ui_text_size(),
                Rect { x: tr_.x, y: tr_.y + th + ui_px(12.0), w: tr_.w, h: ui_px(70.0) },
                ui_name_label(name, &mut display, 160),
                0,
            );
            if !book.is_empty() {
                font_draw_left(
                    c,
                    FontFace::Regular,
                    ui_small_size(),
                    Rect { x: tr_.x, y: tr_.y + th + ui_px(64.0), w: tr_.w, h: ui_px(44.0) },
                    &book,
                    0,
                );
            }
        } else if !book.is_empty() {
            // A book page's: under its label, on a line of its own, the book page it holds.
            ui_text(
                c,
                if cur { FontFace::Bold } else { FontFace::Regular },
                ui_text_size(),
                ui_small_size(),
                Rect { x: tr_.x, y: tr_.y + th + ui_px(12.0), w: tr_.w, h: ui_px(52.0) },
                &lab,
                1,
                true,
                0,
            );
            ui_text(
                c,
                FontFace::Regular,
                ui_small_size(),
                ui_small_size(),
                Rect { x: tr_.x, y: tr_.y + th + ui_px(64.0), w: tr_.w, h: ui_px(44.0) },
                &book,
                1,
                true,
                0,
            );
        } else {
            ui_text(
                c,
                if cur { FontFace::Bold } else { FontFace::Regular },
                ui_text_size(),
                ui_small_size(),
                Rect { x: tr_.x, y: tr_.y + th + ui_px(12.0), w: tr_.w, h: ui_px(70.0) },
                &lab,
                2,
                true,
                0,
            );
        }
        hits_add(&mut v.hits, tr_, PG_OPEN, i);
        if !v.picking {
            hits_add(&mut v.hits, rect_inflate(cb, ui_px(30.0)), PG_TOGGLE, i);
        }
        n += 1;
    }
    let y = ah - ui_px(170.0);
    a.toast_floor = y; // keep a Copy page notice above the bottom controls when Pages reopens
    let c = &mut a.screen;
    canvas_hline(c, 0, aw, y, ui_px(3.0), 0);
    let pr = Rect { x: m, y: y + ui_px(15.0), w: ui_px(220.0), h: ui_px(140.0) };
    let nr = Rect { x: aw - m - ui_px(220.0), y: y + ui_px(15.0), w: ui_px(220.0), h: ui_px(140.0) };
    let screen = if v.index_tab { v.index_screen } else { v.screen };
    ui_button(c, pr, IconKind::Prev, "", if screen > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    ui_button(c, nr, IconKind::Next, "", if screen < screens - 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if screen > 0 {
        hits_add(&mut v.hits, pr, PG_PREV, 0);
    }
    if screen < screens - 1 {
        hits_add(&mut v.hits, nr, PG_NEXT, 0);
    }
    let jump = Rect { x: aw / 2 - ui_px(300.0), y: pr.y, w: ui_px(600.0), h: pr.h };
    ui_button(c, jump, IconKind::None, tr("Go to page..."), ButtonStyle::Plain);
    hits_add(&mut v.hits, jump, PG_JUMP, 0);
    // New page before Go to page..., Copy page after it, each up to 400 px of the room beside it.
    let g = ui_px(20.0);
    let mut bw = jump.x - g - (pr.x + pr.w + g);
    if bw > ui_px(400.0) {
        bw = ui_px(400.0);
    }
    let rnew = Rect { x: jump.x - g - bw, y: pr.y, w: bw, h: pr.h };
    let rcopy = Rect { x: jump.x + jump.w + g, y: pr.y, w: bw, h: pr.h };
    // TRANSLATORS: the Pages view's bottom buttons, each under its icon (290 px at 34 px): a new
    // blank page after the open page (or after the last page ticked), and a copy of the one page
    // ticked, right after it.
    if !v.picking {
        ui_button(c, rnew, IconKind::Plus, tr("New page"), ButtonStyle::Plain);
        hits_add(&mut v.hits, rnew, PG_NEW, 0);
        if v.index_tab {
            // In the Index, where nothing is ticked: a link drawn on the open page.
            let can = nav.links_writable;
            // TRANSLATORS: the Index tab's bottom button, under its icon (290 px at 34 px): draw a
            // link on the open page to another page (or to the Index).
            ui_button(c, rcopy, IconKind::Link, tr("Add a link"), if can { ButtonStyle::Plain } else { ButtonStyle::Disabled });
            if can {
                hits_add(&mut v.hits, rcopy, PG_LINK, 0);
            }
        } else {
            ui_button(c, rcopy, IconKind::Copy, tr("Copy page"), if k == 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
            if k == 1 {
                hits_add(&mut v.hits, rcopy, PG_COPY, 0);
            }
        }
    }
    // A toast stays above the keys, as over the Library's keyboard: Go to page's "Enter a page number"
    // used to cover its 0 and del just as the number was put right, and a key pressed under a toast
    // refreshes the toast's width with it.
    if v.kb.open {
        keyboard_draw(&mut v.kb, c, Rect { x: 0, y: 0, w: aw, h: ah });
        a.toast_floor = v.kb.area.y;
    }
}

// The whole view drawn again and refreshed (what the editor's compose_all does while it is open).
fn redraw(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, m: RefreshMode) {
    pagesview_draw(v, a, nb, nav);
    app_invalidate_all(a, m);
}

// strtol(s, &end, 10): the value (saturating, as strtol does) and where the number ended in s
// (0 when there is none to read).
fn strtol10(s: &[u8]) -> (i64, usize) {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return (0, 0);
    }
    (if neg { -v } else { v }, i)
}

/// Pointer phase 0 down / 1 move / 2 up. What it draws itself (a tick, another screenful, the
/// number pad) it refreshes itself; for the rest, see PvResult. While its keyboard is up a key draws
/// and refreshes only the keyboard, as the Library's does (app_keyboard_pointer).
pub fn pagesview_pointer(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, phase: i32, ev: &InEvent) -> PvResult {
    if v.kb.open {
        // A key costs what it does on the Library's keyboard (app_keyboard_pointer): what it changed,
        // the text line and then the key, each on its own fast refresh, and nothing for a sample that
        // changed nothing. The view under the keys is not drawn again until the keyboard closes.
        let r = app_keyboard_pointer(a, &mut v.kb, phase, ev);
        if r == KbResult::Done {
            if v.naming {
                return PvResult::Name;
            }
            let text = v.kb.text.as_bytes();
            let (page, end) = strtol10(text);
            // (strtol's end is at the text's start when it read nothing: then *end is the text's first
            // byte, which is not the NUL unless the text is empty.)
            if !text.is_empty() && end == text.len() && page >= 1 && page <= nb.npages() as i64 {
                v.target = page as i32 - 1;
                return if v.picking { PvResult::Pick } else { PvResult::Open };
            }
            // Not a page of this notebook: the number pad stays, its Done let go.
            v.kb.open = true;
            app_toast(a, tr("Enter a page number in this notebook."), 2500);
            let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
            keyboard_draw(&mut v.kb, &mut a.screen, screen);
            app_invalidate(a, v.kb.area, RefreshMode::Fast);
        }
        if r == KbResult::Cancel && v.name_only {
            return PvResult::Close; // (back to the page it was asked on)
        }
        if r == KbResult::Cancel {
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        return PvResult::None;
    }
    if phase == 0 {
        press_begin(&mut v.press, ev.x, ev.y, ev.t_ms);
        return PvResult::None;
    }
    press_move(&mut v.press, ev.x, ev.y);
    if phase == 1 {
        return PvResult::None;
    }
    let Some(&hit) = hits_find(&v.hits, v.press.x0, v.press.y0) else {
        return PvResult::None;
    };
    if !press_is_tap(&v.press, hit.r) {
        return PvResult::None;
    }
    match hit.id {
        PG_TAB => {
            v.index_tab = hit.arg != 0;
            if v.index_tab {
                pagesview_untick(v);
            }
            redraw(v, a, nb, nav, RefreshMode::Full);
        }
        PG_NAME => {
            open_naming(v, nb, nav);
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        // Go to page... leaves for the Find sheet (page number or name) while the view is for pages; a link's
        // page being picked keeps the number pad, which is how it picks by number.
        PG_JUMP if !v.picking && !v.jump_pad => return PvResult::GoTo,
        PG_JUMP => {
            v.naming = false;
            keyboard_open(&mut v.kb, tr("Go to page"), b"");
            v.kb.page = 2;
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        PG_BACK => return PvResult::Close,
        PG_OPEN => {
            v.target = hit.arg;
            return if v.picking { PvResult::Pick } else { PvResult::Open };
        }
        PG_LINK => return PvResult::Link,
        PG_TO_INDEX => return PvResult::PickIndex,
        PG_UNLINK => return PvResult::Unlink,
        PG_TOGGLE => {
            if (hit.arg as usize) < v.sel.len() {
                v.sel[hit.arg as usize] = !v.sel[hit.arg as usize];
            }
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        PG_ALL => {
            let all = ticked_in(v, nb.npages()) == nb.npages();
            let mut i = 0;
            while i < nb.npages() && (i as usize) < v.sel.len() {
                v.sel[i as usize] = !all;
                i += 1;
            }
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        PG_DELETE => return PvResult::Delete,
        PG_NEW => {
            let after = last_ticked(v, nb);
            v.target = (if after >= 0 { after } else { v.current }) + 1;
            return PvResult::New;
        }
        PG_COPY => {
            if ticked_in(v, nb.npages()) != 1 {
                return PvResult::None;
            }
            v.target = last_ticked(v, nb);
            return PvResult::Copy;
        }
        PG_PREV | PG_NEXT => {
            if v.index_tab {
                v.index_screen += if hit.id == PG_NEXT { 1 } else { -1 };
            } else {
                v.screen += if hit.id == PG_NEXT { 1 } else { -1 };
            }
            redraw(v, a, nb, nav, RefreshMode::Full);
        }
        _ => {}
    }
    PvResult::None
}

/// From the editor's tick: once typing on its keyboard pauses, the keyboard's grayscale refresh
/// (app_keyboard_tick). Nothing else.
pub fn pagesview_tick(v: &mut PagesView, a: &mut App, now: u32) {
    if v.open && v.kb.open {
        app_keyboard_tick(a, &mut v.kb, now);
    }
}

// atoi(s): strtol's number as an int.
fn atoi(s: &str) -> i32 {
    let (v, _) = strtol10(s.as_bytes());
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// The headless runner's keys for it, without "editor." (docs/SIM.md): "tile.N", the label under
/// page N's tile (from 1) as drawn, its two lines joined by " | " ("Page 3 | p. 12"); empty while it
/// is closed. "picking": "new" or "had" while it picks where a link goes, else empty. "keyboard":
/// its keyboard's title while that is up ("Page name", "Go to page"), else empty; "text" what it
/// holds. 0, or -1 for a key it does not know.
pub fn pagesview_probe(v: &PagesView, nb: &NotebookInfo, nav: &Nav, key: &str, out: &mut String, cap: usize) -> i32 {
    out.clear();
    if key == "pages_tab" {
        csnprintf!(out, cap, "%s", if v.index_tab { "index" } else { "pages" });
    } else if key == "picking" {
        csnprintf!(out, cap, "%s", if !v.open || !v.picking { "" } else if v.pick_had { "had" } else { "new" });
    } else if key == "keyboard" {
        csnprintf!(out, cap, "%s", if v.open && v.kb.open { v.kb.title.as_str() } else { "" });
    } else if key == "text" {
        csnprintf!(out, cap, "%s", if v.open && v.kb.open { v.kb.text.as_str() } else { "" });
    } else if key == "index_rows" {
        csnprintf!(out, cap, "%d", if v.nav_set { nav_index(nav, nb, &mut []) } else { 0 });
    } else if let Some(rest) = key.strip_prefix("tile.") {
        let i = atoi(rest).wrapping_sub(1);
        let (mut lab, mut book) = (String::new(), String::new());
        if !v.open || i < 0 || i >= nb.npages() {
            return 0;
        }
        tile_label(v, nb, nav, i, &mut lab, NAV_NAME_LEN, &mut book, 32);
        csnprintf!(out, cap, "%s%s%s", &lab, if !book.is_empty() { " | " } else { "" }, &book);
    } else {
        return -1;
    }
    0
}
