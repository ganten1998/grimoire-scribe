//! pagesview.h: the editor's Pages view, which covers the screen: numbered tiles a screenful at a
//! time. The UI reads only small cached previews; bounded background jobs render missing ones,
//! ticks for Delete (a held tile's card: Open, Add a link, Rename, Delete), a number pad for Go to
//! page..., New page and Copy page. A page that holds a
//! book page (a notebook annotated from KOReader, annotate.h) says which under its number ("p. 12"),
//! from the notebook's source.ini, read as the view opens.
//!
//! A modal hosted by the editor, which keeps the notebook: the view draws itself and takes the taps,
//! and says what was asked (PvResult); the editor does it (opens a page, deletes the ticked ones).
//!
//! It also picks where a new link goes (pagesview_pick, Link to...'s Choose from the pages...): then a
//! tile or an Index row picks that page, and there are no ticks, nor New page, Copy page or Go to page...
//! (Link to... has the Index and Search for those). And it opens with
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
const PG_GRAPH: i32 = 16;
const PG_LIST: i32 = 17;
const PG_CONTEXT_LINK: i32 = 18;
const PG_CONTEXT_OPEN: i32 = 19;
const PG_CONTEXT_CANCEL: i32 = 20;
const PG_MULTI: i32 = 21;
const PG_CONTEXT_RENAME: i32 = 22;
const PG_CONTEXT_DELETE: i32 = 23;
// (14 and 15 were the picking header's The Index and Remove link, retired with the Pages view's hold path:
// a link held on opens the long-press card over the page instead)

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PvResult {
    #[default]
    None = 0, // PV_NONE
    /// its Back button: the view closes on the page that was open
    Close = 1, // PV_CLOSE
    /// a tile tapped, or a page number entered: the view closes on page `target`
    Open = 2, // PV_OPEN
    /// Delete: the editor deletes the ticked pages (pagesview_ticked), the view stays (the held page's
    /// Delete ticks only that page first, unless it is one of the ticked ones)
    Delete = 3, // PV_DELETE
    /// New page: the editor adds a page at index `target` (after the last page ticked, or after
    /// the open page: never first) and opens it
    New = 4, // PV_NEW
    /// Copy page (one page ticked): the editor copies page `target` right after it
    Copy = 5, // PV_COPY
    /// save kb.text as page `name_page`'s section name (the open page's, or the held page's after its
    /// Rename); empty removes it (then, when name_only, the view closes on the page, as on PV_CLOSE)
    Name = 6, // PV_NAME
    /// Add a link (the Index tab): the editor waits for the link's box on the open page
    Link = 7, // PV_LINK
    // While picking (pagesview_pick); its Back button says PV_CLOSE:
    /// the link goes to page `target`
    Pick = 8, // PV_PICK
    // (9 and 10, PV_PICK_INDEX and PV_UNLINK, went with the Pages view's hold path: the other values stay)
    /// Go to page...: the view closes and the Find sheet opens over the page (Rust only: the C's number pad
    /// stays behind jump_pad, for the test that holds the view to the C build)
    GoTo = 11,
    Graph = 12,
    List = 13,
}

/// (The C keeps pointers to the editor's notebook and Nav, nb and nav; here the editor passes them
/// to each call that reads them: pagesview_draw, pagesview_pointer, pagesview_probe. PORTING.md,
/// Ownership.)
#[derive(Clone, Debug, Default)]
pub struct PagesView {
    pub open: bool,
    /// the control pressed (not a tile: a tile's tap redraws it at once), shown pressed until the lift
    pub pressed: Option<Rect>,
    pub unified: bool,
    /// the open page's index; the editor keeps it up to date
    pub current: i32,
    /// Go to page...'s number pad (only with jump_pad; else the Find sheet is Go to page...), and the
    /// naming keyboard
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
    /// the page the naming keyboard names, PV_NAME's (the open page, or a held tile's after Rename)
    pub name_page: i32,
    /// source.ini: which page holds which book page (empty for most)
    pub book: AnnotateMap,
    pub index_tab: bool,
    pub tab_initialized: bool,
    pub naming: bool,
    pub index_screen: i32,
    /// picking where a new link goes (pagesview_pick)
    pub picking: bool,
    /// opened to name the open page (pagesview_name): its keyboard's
    /// Cancel says PV_CLOSE, and after its Done the editor closes the view
    pub name_only: bool,
    /// Naming or a numeric page jump drawn over the notebook, without the management grid.
    pub overlay: bool,
    pub multi_select: bool,
    /// ... index_tab and tab_initialized as it found them, given back as it closes
    pub name_back_tab: bool,
    pub name_back_init: bool,
    /// The C's `v->nav != NULL`: an open (or pick, or name) set it, pagesview_free clears it.
    /// pagesview_probe's index_rows answers 0 until then (the probe gets the editor's Nav on every
    /// call here, where the C's view had none before it was first opened).
    pub nav_set: bool,
    context: Option<i32>,
    held: bool,
    last_tap: Option<(i32, u32)>,
    /// the press's down, at its kernel time (in_sample_ms): how long a tile was held
    down_k: u32,
    view_size: (i32,i32),
    previews: PagePreviewRequests,
    cache: PreviewCache,
}


/// (cols, rows, cw, ch, th)
fn geom(a: &App, v: &PagesView) -> (i32, i32, i32, i32, i32) {
    if v.unified && !v.picking {
        let gap=ui_px(24.0);
        let cw=(a.w-gap*4)/3;
        let ch=(a.h-ui_px(190.0)-ui_px(170.0)-gap*4)/3;
        return (3,3,cw,ch,ch-ui_px(58.0));
    }
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
    v.overlay = false;
    v.multi_select = false;
    v.cache.sync(nb,true); // (a page saved since, here or elsewhere, is decoded again)
    v.previews=PagePreviewRequests::default();
    v.context = None;
    v.held = false;
    v.last_tap = None;
    v.picking = false;
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
    let (cols, rows, ..) = geom(a,v);
    v.screen = current / (cols * rows);
    true
}

/// The same to pick where a new link on the open page goes (Choose from the pages...).
pub fn pagesview_pick(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32) -> bool {
    if !pagesview_open(v, a, nb, nav, current) {
        return false;
    }
    if nav_index(nav, nb, &mut []) == 0 {
        v.index_tab = false; // an Index with nothing to pick
    }
    v.picking = true;
    let (cols,rows,..)=geom(a,v);v.screen=current/(cols*rows);
    true
}

// The naming keyboard for page `page` (the open one, or a held tile's Rename), holding its name (Name
// this page, pagesview_name); its Done names that page (name_page).
fn open_naming(v: &mut PagesView, nb: &NotebookInfo, nav: &Nav, page: i32) {
    v.naming = true;
    v.name_page = page;
    // TRANSLATORS: the naming keyboard's title, 96 px text-field header.
    keyboard_open(&mut v.kb, tr("Page name"), nav_name(nav, nb.page_ids[page as usize]).as_bytes());
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
    v.overlay = v.unified;
    let current = v.current;
    open_naming(v, nb, nav, current);
    true
}

pub fn pagesview_draw_overlay(v:&mut PagesView,a:&mut App) {
    hits_clear(&mut v.hits);
    keyboard_draw(&mut v.kb,&mut a.screen,Rect::new(0,0,a.w,a.h));
    a.toast_floor=v.kb.area.y;
}

/// Warm the next management screen while the notebook is idle, rather than on its first open.
pub fn pagesview_prefetch(v:&mut PagesView,nb:&NotebookInfo,current:i32,dpi:f32) {
    let first=(current.max(0) as usize/9)*9;
    // (the open page's own preview is made from its canvas: editor save_page)
    let open=nb.page_ids.get(current.max(0) as usize).copied();
    bg_request_page_previews_but(&mut v.previews,nb,first,first+9,dpi,false,open);
}

/// The open page was saved: its tile is decoded again, and its preview may be made once more.
pub fn pagesview_page_saved(v:&mut PagesView,pid:u32) {
    v.cache.forget(pid);
    bg_preview_retry(&mut v.previews,pid);
}

/// The open page's tile from its canvas in memory, as the view opens on it: the page was just saved,
/// and its preview on disk waits for the background job. Nothing when that one is current.
pub fn pagesview_seed(v:&mut PagesView,nb:&NotebookInfo,pid:u32,canvas:&Canvas) {
    if v.cache.has(pid) || store_page_preview_fresh(nb,pid).is_some() {return;}
    let Some(identity)=store_page_source_identity(nb,pid) else{return};
    let mut small=Canvas::default();
    if store_page_preview_image(canvas,&mut small) {v.cache.put(pid,identity,small);}
}

/// The previews generation the view's tiles were drawn at (0 before the first).
pub fn pagesview_previews_generation(v:&PagesView)->u64 {v.cache.generation()}

/// Closes it (and its number pad). Its ticks' memory stays for the next open.
pub fn pagesview_close(v: &mut PagesView) {
    v.open = false;
    v.overlay = false;
    v.press.down = false;
    v.context = None;
    v.last_tap = None;
    v.picking = false;
    end_name_only(v);
    keyboard_close(&mut v.kb);
    annotate_map_free(&mut v.book);
}

pub fn pagesview_free(v: &mut PagesView) {
    annotate_map_free(&mut v.book);
    v.sel = Vec::new();
    v.tab_initialized = false;
    v.nav_set = false;
    v.cache = PreviewCache::default();
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
    // One traversal of the links, outside the visible-row loop. Page IDs need not be in notebook order.
    let mut incoming = [0i32; STORE_MAX_PAGES];
    let mut ids = Vec::new();
    if ids.try_reserve_exact(nb.page_ids.len()).is_ok() {
        ids.extend(nb.page_ids.iter().enumerate().map(|(i, &pid)| (pid, i)));
        ids.sort_unstable_by_key(|p| p.0);
        for l in &nav.links {
            if let Some(pid) = l.target.as_str().strip_prefix('p').and_then(|t| t.parse::<u32>().ok()) {
                if ids.binary_search_by_key(&l.pid, |p| p.0).is_ok() {
                    if let Ok(at) = ids.binary_search_by_key(&pid, |p| p.0) {
                        incoming[ids[at].1] += 1;
                    }
                }
            }
        }
    }
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
            Rect { x: r.x + ui_px(30.0), y: r.y, w: r.w - ui_px(720.0), h: r.h },
            title,
            0,
        );
        let count = incoming[i as usize];
        let text = cformat!(ng_("%d link here", "%d links here", count as i64), count);
        font_draw_left(
            c,
            FontFace::Regular,
            ui_small_size(),
            Rect { x: r.x + r.w - ui_px(670.0), y: r.y, w: ui_px(440.0), h: r.h },
            &text,
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
    v.pressed = None;
    if v.overlay {pagesview_draw_overlay(v,a);return;}
    if v.view_size!=(a.w,a.h) {v.view_size=(a.w,a.h);v.press=Press::default();v.last_tap=None;}
    if v.unified {v.index_tab=false;}
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
        // Nothing beside the tabs while picking: Link to..., under the view, has The Index as a chip.
    } else if v.index_tab {
        let name = Rect { x: rall.x, y: by, w: ui_px(420.0), h: bh };
        let graph = Rect { x: rdel.x + rdel.w - ui_px(300.0), y: by, w: ui_px(300.0), h: bh };
        ui_button(c, graph, IconKind::Bubbles, tr("Graph"), ButtonStyle::Plain);
        hits_add(&mut v.hits, graph, PG_GRAPH, 0);
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
    if v.unified {
        canvas_fill(c,Rect::new(0,0,aw,th0),255);hits_clear(&mut v.hits);
        if v.picking {
            ui_button(c,back,IconKind::Back,"",ButtonStyle::Plain);hits_add(&mut v.hits,back,PG_BACK,0);
            font_draw_left_name(c,FontFace::Bold,ui_title_size(),Rect::new(ui_px(230.0),by,aw-ui_px(280.0),bh),nb.name.as_str(),0);
        } else {
            let tabs=index_navigation(c,aw,2);
            for (i,id) in [PG_BACK,PG_GRAPH,PG_LIST,PG_TAB].into_iter().enumerate(){hits_add(&mut v.hits,tabs[i],id,0);}
        }
    }

    let (cols, rows, cw, ch, th) = geom(a,v);
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
    let mut n = 0;
    if v.unified { draw_tiles(v,a,nb,nav); }
    let c=&mut a.screen;
    while !v.unified && !v.index_tab && n < per {
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
    if v.unified && !v.index_tab {
        let first=(v.screen*per).max(0) as usize;let last=(first+per as usize).min(nb.page_ids.len());
        bg_request_page_previews(&mut v.previews,nb,first,last,a.dpi,true);
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
    // (not while picking: Link to...'s Search finds a page by number or name)
    if !v.picking {
        ui_button(c, jump, IconKind::None, tr("Go to page..."), ButtonStyle::Plain);
        hits_add(&mut v.hits, jump, PG_JUMP, 0);
    }
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
    if v.unified && !v.picking {
        draw_footer(v,a,screens,k,all);
    }
    if let Some(page)=v.context { draw_context(v,a,nb,nav,page); }
    let c=&mut a.screen;
    // A toast stays above the keys, as over the Library's keyboard: Go to page's "Enter a page number"
    // used to cover its 0 and del just as the number was put right, and a key pressed under a toast
    // refreshes the toast's width with it.
    if v.kb.open {
        keyboard_draw(&mut v.kb, c, Rect { x: 0, y: 0, w: aw, h: ah });
        a.toast_floor = v.kb.area.y;
    }
}

// The unified view's bottom row (its buttons follow the ticks: Copy page for one; a held tile's card
// has Delete).
fn draw_footer(v:&mut PagesView,a:&mut App,screens:i32,k:i32,all:bool) {
    let (aw,ah)=(a.w,a.h);
    let y=ah-ui_px(170.0);
    let c=&mut a.screen;
    canvas_fill(c,Rect::new(0,y,aw,ah-y),255);canvas_hline(c,0,aw,y,ui_px(3.0),0);
    v.hits.h.retain(|h|h.r.y<y);
    let side=ui_px(120.0);let gap=ui_px(12.0);
    // (no arrows since 2.0.0-rc.12: one finger flicked sideways, or the page-turn action, turns the screen)
    let widths=[side,side,side,ui_px(260.0),ui_px(260.0)];
    let widths=widths.map(|w|w.min((aw-ui_px(40.0)-4*gap)/5+ui_px(80.0)));
    let total: i32=widths.iter().sum::<i32>()+4*gap;
    let mut x=(aw-total)/2;
    for (i,id) in [PG_NEW,PG_COPY,PG_JUMP,PG_MULTI,PG_ALL].into_iter().enumerate(){
        let enabled=match id{PG_PREV=>v.screen>0,PG_NEXT=>v.screen<screens-1,PG_COPY=>k==1,_=>true};
        let r=Rect::new(x,y+ui_px(15.0),widths[i],ui_px(140.0));x+=widths[i]+gap;
        let (icon,label)=match id {PG_PREV=>(IconKind::Prev,""),PG_NEW=>(IconKind::Plus,""),PG_COPY=>(IconKind::Copy,""),PG_NEXT=>(IconKind::Next,""),PG_MULTI=>(IconKind::None,tr("Multi-select")),PG_ALL=>(IconKind::None,if all{tr("Select none")}else{tr("Select all")}),_=>(IconKind::Search,"")};
        ui_button(c,r,icon,label,if !enabled{ButtonStyle::Disabled}else if id==PG_MULTI&&v.multi_select{ButtonStyle::Selected}else{ButtonStyle::Plain});
        if enabled {hits_add(&mut v.hits,r,id,0);}
    }
}

// Tile `slot` of the screenful: its cell (preview and label, what a tap opens), its preview's place, and
// that place's height.
fn tile_rects(v:&PagesView,a:&App,nb:&NotebookInfo,slot:i32)->(Rect,Rect,i32) {
    let (cols,_,cw,ch,th)=geom(a,v);
    let gap=ui_px(if v.picking {50.0}else{24.0});
    let (mut pw,mut ph)=(0,0);store_notebook_page_size(nb,&mut pw,&mut ph);
    let scale=(cw as f32/pw.max(1) as f32).min(th as f32/ph.max(1) as f32);
    let (w,h)=((pw as f32*scale)as i32,(ph as f32*scale)as i32);
    let cell=Rect::new(gap+(slot%cols)*(cw+gap),ui_px(190.0)+gap+(slot/cols)*(ch+gap),cw,ch);
    (cell,Rect::new(cell.x+(cw-w)/2,cell.y,w,h),h)
}

// Page i's tile, over what is there (the caller clears its cell when drawing it again).
fn draw_tile(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,slot:i32,i:i32,pid:u32)->Rect {
    let (cell,image,h)=tile_rects(v,a,nb,slot);
    let w=image.w;
    if let Some(preview)=v.cache.get(nb,pid){canvas_blit_fit(&mut a.screen,preview,image);}
    else {font_draw_centered(&mut a.screen,FontFace::Bold,ui_title_size(),image,&format!("{}",i+1),0);}
    ui_frame(&mut a.screen,image,0,0,false,ui_px(2.0));
    if !v.picking && v.sel.get(i as usize)==Some(&true) {
        let inner=rect_inflate(image,-ui_px(7.0));
        ui_frame(&mut a.screen,inner,0,0,false,ui_px(4.0));
    }
    let name=nav_name(nav,pid);
    let mut label=if name.is_empty(){format!("{}",i+1)}else{format!("{} · {}",i+1,name)};
    let bp=book_page(v,pid);if bp!=0 {label.push_str(" · ");label.push_str(&cformat!(tr("p. %d"),bp));}
    font_draw_left_name(&mut a.screen,if i==v.current{FontFace::Bold}else{FontFace::Regular},ui_small_size(),Rect::new(image.x,image.y+h+ui_px(6.0),w,ui_px(46.0)),&label,0);
    cell
}

// The tiles of pages `only` (page indexes; None: the whole screenful) drawn again in their cells, and
// where they are. Hits stay as they were: the cells do not move.
fn redraw_tiles(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,only:Option<&[i32]>)->Rect {
    let (cols,rows,..)=geom(a,v);
    let mut dirty=RECT_EMPTY;
    for slot in 0..cols*rows {
        let i=v.screen*(cols*rows)+slot;
        let Some(&pid)=nb.page_ids.get(i as usize)else{break};
        if only.is_some_and(|o|!o.contains(&i)) {continue;}
        let (cell,..)=tile_rects(v,a,nb,slot);
        canvas_fill(&mut a.screen,cell,255);
        dirty=rect_union(dirty,draw_tile(v,a,nb,nav,slot,i,pid));
    }
    dirty
}

// Nine full-page previews. Cache decoded images while open; the disk cache survives app restarts.
fn draw_tiles(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav) {
    v.cache.sync(nb,false);
    let (cols,rows,..)=geom(a,v);
    for slot in 0..cols*rows {
        let i=v.screen*(cols*rows)+slot;
        let Some(&pid)=nb.page_ids.get(i as usize)else{break};
        let cell=draw_tile(v,a,nb,nav,slot,i,pid);
        hits_add(&mut v.hits,cell,PG_OPEN,i);
    }
    retain_screen(v,a,nb);
}

// Only the screenful's decoded previews stay.
fn retain_screen(v:&mut PagesView,a:&App,nb:&NotebookInfo) {
    let (cols,rows,..)=geom(a,v);
    let first=((v.screen*cols*rows).max(0) as usize).min(nb.page_ids.len());
    let end=(first+(cols*rows) as usize).min(nb.page_ids.len());
    v.cache.retain(&nb.page_ids[first..end]);
}

/// New previews came while the grid is up: only its tiles are drawn again (the caller refreshes the
/// rect returned). RECT_EMPTY when no grid shows (a keyboard, the card over it): it is drawn whole when
/// that goes.
pub fn pagesview_repaint_tiles(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav)->Rect {
    if !v.open || !v.unified || v.index_tab || v.overlay || v.kb.open || v.context.is_some() || v.view_size!=(a.w,a.h) {
        return RECT_EMPTY;
    }
    v.cache.sync(nb,false);
    let r=redraw_tiles(v,a,nb,nav,None);
    retain_screen(v,a,nb);
    r
}

// A tap changed the ticks (sel as it was before): the tiles that changed, and the bottom row when its
// buttons did, are drawn again and refreshed on their own, not the whole view.
fn repaint_ticks(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,before:&[bool]) {
    let npages=nb.npages();
    let changed:Vec<i32>=(0..npages).filter(|&i|before.get(i as usize)!=v.sel.get(i as usize)).collect();
    let k_was=before.iter().take(npages as usize).filter(|&&t|t).count() as i32;
    let k=ticked_in(v,npages);
    let r=redraw_tiles(v,a,nb,nav,Some(&changed));
    if !rect_is_empty(r) {app_invalidate(a,r,RefreshMode::Ui);}
    if (k_was==1)!=(k==1) || (k_was==npages)!=(k==npages) {
        // (on its own: a tile near the top and the bottom row never merge into one refresh of the screen)
        app_flush(a);
        let (cols,rows,..)=geom(a,v);
        let screens=((npages+cols*rows-1)/(cols*rows)).max(1);
        draw_footer(v,a,screens,k,k==npages);
        let y=a.h-ui_px(170.0);
        app_invalidate(a,Rect::new(0,y,a.w,a.h-y),RefreshMode::Ui);
    }
}

// A held tile's card: Open, Add a link, Rename (its naming keyboard), Delete (the ticked pages when it
// is one of them, else it alone) and Cancel.
fn draw_context(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,page:i32) {
    hits_clear(&mut v.hits);
    let w=ui_px(1000.0).min(a.w-ui_px(60.0));let h=ui_px(780.0);
    let card=Rect::new((a.w-w)/2,(a.h-h)/2,w,h);
    ui_frame(&mut a.screen,card,ui_px(18.0),255,true,0);
    ui_frame(&mut a.screen,card,ui_px(18.0),0,false,ui_px(3.0));
    font_draw_centered(&mut a.screen,FontFace::Bold,ui_title_size(),Rect::new(card.x,card.y,w,ui_px(100.0)),&cformat!(tr("Page %d"),page+1),0);
    let k=ticked_in(v,nb.npages());
    let del=if k>1 && pagesview_ticked(v,page) {cformat!(tr("Delete (%d)"),k)}else{tr("Delete").to_string()};
    // TRANSLATORS: a held page's card in the Pages view, under its icon (950 px at 34 px): give the page
    // a name (its naming keyboard, titled Page name).
    let rename=tr("Rename");
    for (row,id,icon,label) in [(0,PG_CONTEXT_OPEN,IconKind::None,tr("Open")),(1,PG_CONTEXT_LINK,IconKind::LinkPlus,tr("Add a link")),
        (2,PG_CONTEXT_RENAME,IconKind::Rename,rename),(3,PG_CONTEXT_DELETE,IconKind::Trash,del.as_str()),(4,PG_CONTEXT_CANCEL,IconKind::None,tr("Cancel"))] {
        let rect=Rect::new(card.x+ui_px(24.0),card.y+ui_px(110.0)+row*ui_px(130.0),w-ui_px(48.0),ui_px(120.0));
        let enabled=nb.page_ids.get(page as usize).is_some() && (id!=PG_CONTEXT_RENAME || nav.writable);
        ui_button(&mut a.screen,rect,icon,label,if enabled{ButtonStyle::Plain}else{ButtonStyle::Disabled});
        if enabled{hits_add(&mut v.hits,rect,id,0);}
    }
    a.toast_floor=card.y;
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
                return PvResult::Open; // (the number pad never opens while picking: no Go to page... then)
            }
            // Not a page of this notebook: the number pad stays, its Done let go.
            v.kb.open = true;
            app_toast(a, tr("Enter a page number in this notebook."), 2500);
            let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
            keyboard_draw(&mut v.kb, &mut a.screen, screen);
            app_invalidate(a, v.kb.area, RefreshMode::Fast);
        }
        if r == KbResult::Cancel && (v.name_only || v.overlay) {
            return PvResult::Close; // (back to the page it was asked on)
        }
        if r == KbResult::Cancel {
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        return PvResult::None;
    }
    if phase == 0 {
        v.held = false;
        v.down_k = in_sample_ms(ev);
        press_begin(&mut v.press, ev.x, ev.y, ev.t_ms);
        if let Some(&h) = hits_find(&v.hits, ev.x, ev.y).filter(|h| h.id != PG_OPEN && h.id != PG_TOGGLE) {
            app_press_feedback(a, h.r);
            v.pressed = Some(h.r);
        }
        return PvResult::None;
    }
    press_move(&mut v.press, ev.x, ev.y);
    if phase == 1 {
        return PvResult::None;
    }
    v.press.down = false;
    let back = app_press_release(a, &mut v.pressed);
    if !rect_is_empty(back) {
        app_invalidate(a, back, RefreshMode::Ui);
    }
    if v.held {v.held=false;return PvResult::None;}
    if v.unified && !touch_cancelled(ev) {
        if let Some(next) = flick(v, ev) {
            pagesview_turn(v, a, nb, nav, next);
            return PvResult::None;
        }
    }
    let Some(&hit) = hits_find(&v.hits, v.press.x0, v.press.y0) else {
        if v.context.take().is_some(){redraw(v,a,nb,nav,RefreshMode::Ui);}
        return PvResult::None;
    };
    if !press_is_tap(&v.press, hit.r) {
        return PvResult::None;
    }
    if v.unified && !v.picking && hit.id==PG_OPEN && in_sample_ms(ev).wrapping_sub(v.down_k)>=600 {
        v.context=Some(hit.arg);v.last_tap=None;redraw(v,a,nb,nav,RefreshMode::Ui);return PvResult::None;
    }
    match hit.id {
        PG_CONTEXT_CANCEL => {v.context=None;redraw(v,a,nb,nav,RefreshMode::Ui);}
        PG_CONTEXT_LINK | PG_CONTEXT_OPEN => {
            if let Some(page)=v.context.take(){v.target=page;return if hit.id==PG_CONTEXT_LINK {PvResult::Link}else{PvResult::Open};}
        }
        // Rename: the naming keyboard over the grid, for the held page; its Cancel gives the grid back.
        PG_CONTEXT_RENAME => {
            if let Some(page)=v.context.take(){open_naming(v,nb,nav,page);redraw(v,a,nb,nav,RefreshMode::Ui);}
        }
        // Delete: the ticked pages when the held one is ticked, else that page alone (as if it were the one tick).
        PG_CONTEXT_DELETE => {
            if let Some(page)=v.context.take() {
                if !pagesview_ticked(v,page) {v.sel.fill(false);if let Some(t)=v.sel.get_mut(page as usize){*t=true;}}
                return PvResult::Delete;
            }
        }
        PG_TAB => {
            v.index_tab = hit.arg != 0;
            if v.index_tab {
                pagesview_untick(v);
            }
            redraw(v, a, nb, nav, RefreshMode::Full);
        }
        PG_GRAPH => return PvResult::Graph,
        PG_LIST => return PvResult::List,
        PG_NAME => {
            let current = v.current;
            open_naming(v, nb, nav, current);
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        // Go to page... leaves for the Find sheet (page number or name); it is not there while picking.
        PG_JUMP if !v.jump_pad => return PvResult::GoTo,
        PG_JUMP => {
            v.naming = false;
            keyboard_open(&mut v.kb, tr("Go to page"), b"");
            v.kb.page = 2;
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        PG_BACK => return PvResult::Close,
        PG_OPEN => {
            v.target = hit.arg;
            if v.picking {return PvResult::Pick;}
            if !v.unified {return PvResult::Open;}
            // (the taps' own times, the kernel's: a second tap read late, behind a refresh, still opens)
            let at_k=in_sample_ms(ev);
            if v.last_tap.is_some_and(|(page,at)|page==hit.arg && at_k.wrapping_sub(at)<=450) {
                v.last_tap=None;return PvResult::Open;
            }
            v.last_tap=Some((hit.arg,at_k));
            let before=v.sel.clone();
            if !v.multi_select {v.sel.fill(false);}
            if let Some(selected)=v.sel.get_mut(hit.arg as usize){*selected=!*selected;}
            repaint_ticks(v,a,nb,nav,&before);
        }
        PG_LINK => return PvResult::Link,
        PG_TOGGLE => {
            if (hit.arg as usize) < v.sel.len() {
                v.sel[hit.arg as usize] = !v.sel[hit.arg as usize];
            }
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        PG_ALL => {
            v.multi_select=true;
            let all = ticked_in(v, nb.npages()) == nb.npages();
            let mut i = 0;
            while i < nb.npages() && (i as usize) < v.sel.len() {
                v.sel[i as usize] = !all;
                i += 1;
            }
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        PG_MULTI => {
            v.multi_select=!v.multi_select;
            if !v.multi_select {v.sel.fill(false);}
            v.last_tap=None;
            redraw(v,a,nb,nav,RefreshMode::Ui);
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

/// One finger flicked sideways over the grid (at least 12 mm, mostly sideways, within 800 ms): Some(true)
/// for the next screen (to the left), Some(false) for the previous, as on the notebook's page.
fn flick(v: &PagesView, ev: &InEvent) -> Option<bool> {
    let (dx, dy) = (ev.x - v.press.x0, ev.y - v.press.y0);
    let quick = ev.t_ms.wrapping_sub(v.press.t0) < 800;
    (quick && dx.abs() >= ui_px(140.0) as f32 && dx.abs() > 1.5 * dy.abs()).then_some(dx < 0.0)
}

/// The grid's next (or previous) screen in the tab in front (the pages, or the Index's list): a flick, or
/// the page-turn actions while the view is up (keys, a controller, a gesture). False at either end.
pub fn pagesview_turn(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, next: bool) -> bool {
    let screens = if v.index_tab {
        (nav_index(nav, nb, &mut []) + index_rows(a) - 1) / index_rows(a)
    } else {
        let (cols, rows, ..) = geom(a, v);
        (nb.npages() + cols * rows - 1) / (cols * rows)
    }
    .max(1);
    let cur = if v.index_tab { v.index_screen } else { v.screen };
    let to = cur + if next { 1 } else { -1 };
    if to < 0 || to >= screens {
        return false;
    }
    if v.index_tab { v.index_screen = to } else { v.screen = to }
    v.context = None;
    redraw(v, a, nb, nav, RefreshMode::Full);
    true
}

/// Held management tiles open their action card; paused typing gets its grayscale refresh.
pub fn pagesview_tick(v: &mut PagesView, a: &mut App, nb:&NotebookInfo, nav:&Nav, now: u32) {
    if v.open && v.unified && !v.picking && !v.kb.open && v.context.is_none() && v.press.down && !v.press.moved
        && now.wrapping_sub(v.press.t0)>=600 {
        if let Some(hit)=hits_find(&v.hits,v.press.x0,v.press.y0).copied().filter(|h|h.id==PG_OPEN) {
            v.context=Some(hit.arg);v.held=true;v.last_tap=None;redraw(v,a,nb,nav,RefreshMode::Ui);
        }
    }
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
/// is closed. "picking": "new" while it picks where a new link goes, else empty. "keyboard":
/// its keyboard's title while that is up ("Page name", "Go to page"), else empty; "text" what it
/// holds. 0, or -1 for a key it does not know.
pub fn pagesview_probe(v: &PagesView, nb: &NotebookInfo, nav: &Nav, key: &str, out: &mut String, cap: usize) -> i32 {
    if key=="context" {csnprintf!(out,cap,"%d",v.context.map_or(0,|p|p+1));return 0;}
    if key=="overlay" {csnprintf!(out,cap,"%d",v.overlay as i32);return 0;}
    if key=="multi" {csnprintf!(out,cap,"%d",v.multi_select as i32);return 0;}
    if key=="cached" {csnprintf!(out,cap,"%d",v.cache.len());return 0;}
    out.clear();
    if key == "pages_tab" {
        csnprintf!(out, cap, "%s", if v.index_tab { "index" } else { "pages" });
    } else if key == "picking" {
        csnprintf!(out, cap, "%s", if !v.open || !v.picking { "" } else { "new" });
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

pub fn pagesview_control_point(v:&PagesView,name:&str)->Option<(f32,f32)> {
    let (id,arg)=match name {"link"=>(PG_CONTEXT_LINK,0),"open"=>(PG_CONTEXT_OPEN,0),"cancel"=>(PG_CONTEXT_CANCEL,0),"back"=>(PG_BACK,0),"graph"=>(PG_GRAPH,0),"list"=>(PG_LIST,0),"pages"=>(PG_TAB,0),"new"=>(PG_NEW,0),"copy"=>(PG_COPY,0),"all"=>(PG_ALL,0),"multi"=>(PG_MULTI,0),"goto"=>(PG_JUMP,0),"prev"=>(PG_PREV,0),"next"=>(PG_NEXT,0),"rename"=>(PG_CONTEXT_RENAME,0),"delete"=>(PG_CONTEXT_DELETE,0),n if n.starts_with("page.")=>(PG_OPEN,n[5..].parse::<i32>().ok()?-1),_=>return None};
    v.hits.h.iter().find(|h|h.id==id&&h.arg==arg).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32))
}

pub fn pagesview_grid(v:&PagesView,a:&App)->(i32,i32) {let (cols,rows,..)=geom(a,v);(cols,rows)}
