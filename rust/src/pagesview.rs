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
//!
//! Since 2.0.0-beta.2 a held tile moved without lifting is dragged to a gap (PageDrag), and the bottom
//! row offers Undo right after a page move (move_undo): both go through the editor's Move path.
//! New page held (or tapped with a keyboard's modifier key held) opens the count pad for a batch
//! of new pages (PvResult::AddPages), and Undo takes the batch back while its pages are still blank
//! (add_undo). A tap on New page is a batch of one (owner, 2026-11-03: the page is not opened; the view
//! stays, its tile marked "(new)" while Undo would take it back). The `undo` and `redo` actions (keys, a
//! controller, gestures) are that Undo and its Redo while the view shows its grid (the owner, 2026-11-03):
//! the Pages view's history (one step, `move_undo` or `add_undo`, and `redo`) stays apart from a page's own.
//! The `select-all` action ticks every page (pagesview_select_all).

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
const PG_CONTEXT_MOVE: i32 = 24;
const PG_CONTEXT_COPY_TO: i32 = 25;
const PG_MOVE: i32 = 26;
const PG_COPY_TO: i32 = 27;
const PG_UNDO_MOVE: i32 = 28;
const PG_UNDO_ADD: i32 = 29;
const PG_REDO: i32 = 30;

/// How long New page is held to open the batch's count pad: a held tile's (its card) time. The editor's
/// toolbar + (the next-page button on the last page) takes the same hold for its own count pad.
pub const ADD_HOLD_MS: u32 = 600;
/// A tap on New page with any of these held on a keyboard (KEYMOD_*, which input_evdev puts on each pen
/// and touch event) opens it too (and the toolbar's + its pad).
pub const ADD_MODS: u32 = (KEYMOD_SHIFT | KEYMOD_CTRL | KEYMOD_ALT | KEYMOD_META) as u32;
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
    // (4, PV_NEW, a tap on New page that added a page and opened it: since 2.0.0-beta.2 a tap is a batch of
    // one, PV_ADD_PAGES with add_count 1, and the view stays)
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
    /// Move to...'s number pad said where: the editor moves the ticked pages, together and in their
    /// order, before page `target` (npages: at the end), and the view stays
    Move = 14,
    /// Copy to notebook...: the editor opens the notebook picker for the ticked pages
    CopyTo = 15,
    /// the bottom row's Undo (in Move to...'s place, nothing ticked, right after a move): the editor puts
    /// the pages back in the order `move_undo` had before it (store_undo_move_pages)
    UndoMove = 16,
    /// New page's count pad said how many, or New page was tapped (`add_count` 1): the editor adds
    /// `add_count` blank pages at index `target` (after the last page ticked, or after the open page: never
    /// first), in one store operation, and the view stays on the first of them
    AddPages = 17,
    /// the bottom row's Undo right after a batch (add_undo): the editor takes those pages back
    UndoAdd = 18,
    /// the bottom row's Redo (in Copy to notebook...'s place, nothing ticked, right after an Undo): the editor
    /// does again what the Undo took back (`redo`)
    Redo = 19,
}

/// What the Pages view's Redo does again (2.0.0-beta.2, the owner's ruling of 2026-11-03): the move or the
/// batch an Undo just took back, by the record that Undo used. One step: an Undo sets it, a Redo uses it (and
/// gives the Undo its record back), a new move or batch drops it, and it applies only while the notebook's
/// pages are still the list the Undo left (PageRedo::applies), as Undo applies only while they are the list
/// the move or the batch left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageRedo {
    /// store_redo_move_pages: the pages in the exact order the move left
    Move(PageMoveUndo),
    /// store_redo_add_pages: the same blank pages again, with the same ids, at the same places
    Add(PageAddUndo),
}

impl PageRedo {
    /// Whether it still redoes: the same notebook, its pages still the list from before the move or batch.
    pub fn applies(&self, nb: &NotebookInfo) -> bool {
        let (id, before) = match self {
            PageRedo::Move(u) => (&u.notebook, &u.before),
            PageRedo::Add(u) => (&u.notebook, &u.before),
        };
        id.as_str() == nb.id.as_str() && *before == nb.page_ids
    }
}

/// A held tile picked up and moved (2.0.0-beta.2): the pages it carries follow the finger (or pen), a black
/// bar shows the gap they land in (nothing else moves on the screen: the owner, 2026-11-03, "no need to have
/// the page outline show when dragging, just show the black bar"), and the lift drops them there
/// (PvResult::Move).
#[derive(Clone, Debug, Default)]
struct PageDrag {
    /// the pages carried (indices, in order): the ticked ones when the held tile is ticked, else it alone
    pages: Vec<i32>,
    /// the contact's place now
    pos: (f32, f32),
    /// the gap it lands in: before page `to` (npages: the end); None off the grid
    to: Option<i32>,
    /// what the bar covers on the screen now (restored from `under` as it moves)
    shown: Rect,
    /// over the bar or the bottom row for a while: the next screen (true) or the previous, since when
    edge: Option<(bool, u32)>,
    /// the view as drawn under it
    under: Canvas,
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
    /// after a drag's PvResult::Move: the pages it carried (indices, in order), which the editor moves in
    /// place of the ticked ones (Move to...'s keypad leaves it empty). The ticks are not touched, so a move
    /// that fails leaves them as they were.
    pub carry: Vec<i32>,
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
    /// the number pad up is Move to...'s (where the ticked pages go), not Go to page's
    pub moving: bool,
    /// the notebook's last page move (Move to... or a drag), offered as Undo in the bottom row while it
    /// applies (PageMoveUndo::applies: no page added, deleted, copied or moved since); the editor drops it
    /// as the notebook closes. A rename keeps it: names go by page id.
    pub move_undo: Option<PageMoveUndo>,
    /// the number pad up is New page's count pad (how many pages a batch adds), PvResult::AddPages's
    pub adding: bool,
    /// after PvResult::AddPages: how many pages
    pub add_count: i32,
    /// the notebook's last batch of new pages, offered as Undo in the bottom row while it applies
    /// (PageAddUndo::applies), none of its pages was saved since (pagesview_page_saved) and the open page
    /// is not one of them; the editor drops it as the notebook closes
    pub add_undo: Option<PageAddUndo>,
    /// the move or batch the last Undo took back, offered as Redo in the bottom row (Copy to notebook...'s
    /// place, nothing ticked) and to the `redo` action while it applies (PageRedo::applies); the editor drops
    /// it at a new move or batch and as the notebook closes
    pub redo: Option<PageRedo>,
    /// when the editor finished writing a batch (now_ms): a contact that came down by then was made while
    /// the pad still showed and the pages were being written, and is let go, down to its lift, so it never
    /// lands on the grid or the bottom row drawn after; cleared by the first contact after it
    pub quiet_until: Option<u32>,
    /// the contact under way is such a one
    pub quiet_contact: bool,
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
    drag: Option<PageDrag>,
    /// the modifier keys held as the contact under way came down
    press_mods: u32,
    view_size: (i32,i32),
    previews: PagePreviewRequests,
    cache: PreviewCache,
}


/// (cols, rows, cw, ch, th)
fn geom(a: &App, v: &PagesView) -> (i32, i32, i32, i32, i32) {
    geom_at((a.w, a.h), v)
}
// The same on a screen of w x h (the view's last: view_size).
fn geom_at((w, h): (i32, i32), v: &PagesView) -> (i32, i32, i32, i32, i32) {
    if v.unified && !v.picking {
        let gap=ui_px(24.0);
        let cw=(w-gap*4)/3;
        let ch=(h-ui_px(190.0)-ui_px(170.0)-gap*4)/3;
        return (3,3,cw,ch,ch-ui_px(58.0));
    }
    let cols = if w > h { 6 } else { 4 };
    let m = ui_px(50.0);
    let cw = (w - m * (cols + 1)) / cols;
    let th = ui_px(200.0);
    let ch = th + ui_px(90.0);
    let mut rows = (h - ui_px(190.0) - ui_px(170.0) - m) / (ch + m);
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
    v.moving = false;
    v.adding = false;
    v.cache.sync(nb,true); // (a page saved since, here or elsewhere, is decoded again)
    v.previews=PagePreviewRequests::default();
    v.context = None;
    v.held = false;
    v.drag = None;
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
    // (a page of the last batch written: no longer the blank page Undo would take back)
    if v.add_undo.as_ref().is_some_and(|u|u.added.contains(&pid)) {v.add_undo=None;}
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
    v.moving = false;
    v.adding = false;
    v.overlay = false;
    v.press.down = false;
    v.context = None;
    v.drag = None;
    v.last_tap = None;
    v.picking = false;
    v.quiet_until = None;
    v.quiet_contact = false;
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
    v.move_undo = None;
    v.add_undo = None;
    v.redo = None;
    v.drag = None;
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
    let fmt = if i == v.current {
        // TRANSLATORS: under each page's picture in the Pages view (436 px at 44 px sideways), the
        // open page in bold.
        tr("Page %d (open)")
    } else if add_marked(v, nb, nav, nb.page_ids[i as usize]) {
        // TRANSLATORS: under a page's picture in the Pages view (436 px at 44 px sideways): a page New page
        // (or its batch) just added, which the bottom row's Undo would take back.
        tr("Page %d (new)")
    } else {
        tr("Page %d")
    };
    csnprintf!(lab, cap, fmt, i + 1);
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

// Whether page `pid` is one of the last batch's (or New page's one) that the bottom row's Undo would take
// back now: its tile says "(new)" (drawn with the grid, no refresh of its own; gone with the Undo).
fn add_marked(v: &PagesView, nb: &NotebookInfo, nav: &Nav, pid: u32) -> bool {
    v.add_undo.as_ref().is_some_and(|u| u.added.contains(&pid)) && undo_id(v, nb, nav, 0) == Some(PG_UNDO_ADD)
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
    if v.unified {
        // The screenful's missing previews asked for first, so its tiles show the loading mark until they
        // come. (The open page is left out when its tile is made from its canvas, as the view opened, or its
        // preview is current: so asked, the batch is the idle warm-up's (pagesview_prefetch), and one that is
        // queued already is the one waited on.)
        let first=(v.screen*per).max(0) as usize;let last=(first+per as usize).min(nb.page_ids.len());
        let open=nb.page_ids.get(v.current.max(0) as usize).copied()
            .filter(|&pid|v.cache.has(pid)||store_page_preview_fresh(nb,pid).is_some());
        bg_request_page_previews_but(&mut v.previews,nb,first,last,a.dpi,true,open);
        draw_tiles(v,a,nb,nav);
    }
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
        let undo=undo_id(v,nb,nav,k);
        let redo=redo_id(v,nb,k);
        draw_footer(v,a,screens,k,all,undo,redo);
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
    if v.drag.is_some() {
        drag_under(v, a, nb);
    }
}

/// Whether a page of a batch is in use: named, or linked (a link on it, or one to it). Its Undo is then
/// neither offered (undo_id) nor done (the editor's pages_undo_add).
pub fn pagesview_add_in_use(u: &PageAddUndo, nav: &Nav) -> bool {
    u.added.iter().any(|&pid| {
        let target = format!("p{pid}");
        !nav_name(nav, pid).is_empty() || nav.links.iter().any(|l| l.pid == pid || l.target.as_str() == target)
    })
}

// Whether the bottom row offers Undo in Move to...'s place: nothing ticked, and the last move still the
// newest change to the notebook's pages.
fn undo_offered(v: &PagesView, nb: &NotebookInfo, nav: &Nav, k: i32) -> bool {
    undo_id(v, nb, nav, k).is_some()
}

// Which Undo it is: the last batch of new pages (PG_UNDO_ADD) while it is the newest change, none of its
// pages saved, named or linked since and the open page not one of them, else the last move (PG_UNDO_MOVE).
fn undo_id(v: &PagesView, nb: &NotebookInfo, nav: &Nav, k: i32) -> Option<i32> {
    if k != 0 {
        return None;
    }
    let open = nb.page_ids.get(v.current.max(0) as usize).copied().unwrap_or(0);
    if v.add_undo.as_ref().is_some_and(|u| u.applies(nb) && !u.added.contains(&open) && !pagesview_add_in_use(u, nav)) {
        return Some(PG_UNDO_ADD);
    }
    v.move_undo.as_ref().is_some_and(|u| u.applies(nb)).then_some(PG_UNDO_MOVE)
}

/// Which Undo the `undo` action does now (ticks or not: the bottom row hides it only for Move to...'s
/// place): PvResult::UndoAdd, PvResult::UndoMove, or None when nothing is offered.
pub fn pagesview_undo_result(v: &PagesView, nb: &NotebookInfo, nav: &Nav) -> Option<PvResult> {
    undo_id(v, nb, nav, 0).map(|id| if id == PG_UNDO_ADD { PvResult::UndoAdd } else { PvResult::UndoMove })
}

/// Whether nothing is under way on the grid: no held tile's card and no drag (the `undo` and `redo` actions
/// wait for those to end).
pub fn pagesview_quiet(v: &PagesView) -> bool {
    v.context.is_none() && v.drag.is_none()
}

impl PagesView {
    /// No drag under way.
    pub fn drag_free(&self) -> bool {
        self.drag.is_none()
    }
}

/// Whether the grid is up and takes what a key asks of it (`new-page`'s count pad, `select-all`, `go-to-page`'s
/// Find over it): the Pages view open on its tiles, not picking or naming, no pad, card or drag over them.
pub fn pagesview_grid_ready(v: &PagesView) -> bool {
    v.open && v.unified && !v.picking && !v.index_tab && !v.overlay && !v.name_only && !v.kb.open && pagesview_quiet(v)
}

/// `select-all` (Ctrl+A): every page ticked, as the Select all button
/// leaves them (Multi-select on, the button reading Select none); already all ticked, it stays so. No undo step.
/// False when the grid is not up to take it.
pub fn pagesview_select_all(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> bool {
    if !pagesview_grid_ready(v) {
        return false;
    }
    let n = (nb.npages().max(0) as usize).min(v.sel.len());
    if v.multi_select && v.sel[..n].iter().all(|&t| t) {
        return true; // (as it was: nothing to draw)
    }
    v.multi_select = true;
    v.sel[..n].fill(true);
    v.last_tap = None;
    redraw(v, a, nb, nav, RefreshMode::Ui);
    true
}

/// New page's count pad over the grid, its count at 1 (`new-page` and `new-pages` while the view is up: Ctrl+N):
/// Enter or its Done adds that many pages after the open page (or the last page ticked). False when the grid is not up to take it.
pub fn pagesview_open_add(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> bool {
    if !pagesview_grid_ready(v) {
        return false;
    }
    open_add_pad(v, a, nb, nav);
    v.held = false; // (no press of its own to let go: a key opened it)
    true
}

/// Whether a drag (a held tile moved) is under way: what Esc cancels (pagesview_cancel_drag).
pub fn pagesview_dragging(v: &PagesView) -> bool {
    v.open && v.drag.is_some()
}

/// Esc during a drag: the move is dropped with the pages where they were; the view is drawn again, and the rest
/// of the contact does nothing (down to its lift). False with no drag under way.
pub fn pagesview_cancel_drag(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> bool {
    if !pagesview_dragging(v) {
        return false;
    }
    v.drag = None;
    v.held = true; // (its lift lets go)
    v.press.down = false;
    redraw(v, a, nb, nav, RefreshMode::Ui);
    true
}

/// The ticks clear (Esc's `back` with pages ticked): false when none were.
pub fn pagesview_clear_ticks(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> bool {
    if !v.open || ticked_in(v, nb.npages()) == 0 {
        return false;
    }
    let before = v.sel.clone();
    v.sel.fill(false);
    v.last_tap = None;
    if v.unified && !v.index_tab && !v.overlay && !v.kb.open && v.context.is_none() {
        repaint_ticks(v, a, nb, nav, &before);
    } else {
        redraw(v, a, nb, nav, RefreshMode::Ui);
    }
    true
}

/// The held tile's card closes (Esc's `back`): false when none was up.
pub fn pagesview_close_card(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> bool {
    if !v.open || v.context.is_none() || v.drag.is_some() {
        return false;
    }
    v.context = None;
    redraw(v, a, nb, nav, RefreshMode::Ui);
    true
}

/// Whether there is a Redo to do now (the `redo` action, and the bottom row's with nothing ticked).
pub fn pagesview_redo_offered(v: &PagesView, nb: &NotebookInfo) -> bool {
    v.redo.as_ref().is_some_and(|r| r.applies(nb))
}

// Which Redo the bottom row offers in Copy to notebook...'s place: nothing ticked and a Redo to do.
fn redo_id(v: &PagesView, nb: &NotebookInfo, k: i32) -> Option<i32> {
    (k == 0 && pagesview_redo_offered(v, nb)).then_some(PG_REDO)
}

// The unified view's bottom row (its buttons follow the ticks: Copy page for one; a held tile's card
// has Delete).
fn draw_footer(v:&mut PagesView,a:&mut App,screens:i32,k:i32,all:bool,undo:Option<i32>,redo:Option<i32>) {
    let (aw,ah)=(a.w,a.h);
    let y=ah-ui_px(170.0);
    let c=&mut a.screen;
    canvas_fill(c,Rect::new(0,y,aw,ah-y),255);canvas_hline(c,0,aw,y,ui_px(3.0),0);
    v.hits.h.retain(|h|h.r.y<y);
    let side=ui_px(120.0);let gap=ui_px(12.0);
    // (no arrows since 2.0.0-rc.12: one finger flicked sideways, or the page-turn action, turns the screen)
    // Move to... (the grip) and Copy to notebook... (the notebook) act on the ticked pages. With nothing
    // ticked, right after a move (Move to... or a drag), the grip's place is Undo, which puts them back,
    // and right after an Undo the notebook's place, beside it, is Redo, which does it again.
    let widths=[side,side,side,side,side,ui_px(260.0),ui_px(260.0)];
    let widths=widths.map(|w|w.min((aw-ui_px(40.0)-6*gap)/7+ui_px(60.0)));
    let total: i32=widths.iter().sum::<i32>()+6*gap;
    let mut x=(aw-total)/2;
    for (i,id) in [PG_NEW,PG_COPY,undo.unwrap_or(PG_MOVE),redo.unwrap_or(PG_COPY_TO),PG_JUMP,PG_MULTI,PG_ALL].into_iter().enumerate(){
        let enabled=match id{PG_PREV=>v.screen>0,PG_NEXT=>v.screen<screens-1,PG_COPY=>k==1,PG_MOVE|PG_COPY_TO=>k>0,_=>true};
        let r=Rect::new(x,y+ui_px(15.0),widths[i],ui_px(140.0));x+=widths[i]+gap;
        let (icon,label)=match id {PG_PREV=>(IconKind::Prev,""),PG_NEW=>(IconKind::Plus,""),PG_COPY=>(IconKind::Copy,""),PG_MOVE=>(IconKind::Grip,""),PG_UNDO_MOVE|PG_UNDO_ADD=>(IconKind::Undo,""),PG_REDO=>(IconKind::Redo,""),PG_COPY_TO=>(IconKind::Notebook,""),PG_NEXT=>(IconKind::Next,""),PG_MULTI=>(IconKind::None,tr("Multi-select")),PG_ALL=>(IconKind::None,if all{tr("Select none")}else{tr("Select all")}),_=>(IconKind::Search,"")};
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
    // (noted only when it is drawn whole: under the Find sheet's clip a tile is drawn again later)
    let whole=rect_contains_rect(a.screen.clip,image);
    match v.cache.tile(nb,pid,&mut v.previews,whole) {
        Tile::Preview(preview)=>canvas_blit_fit(&mut a.screen,preview,image),
        Tile::Loading=>ui_tile_loading(&mut a.screen,image),
        Tile::Number=>font_draw_centered(&mut a.screen,FontFace::Bold,ui_title_size(),image,&format!("{}",i+1),0),
    }
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

// The screenful's pages: the first one's index, and their ids.
fn screen_pages<'a>(v:&PagesView,a:&App,nb:&'a NotebookInfo)->(usize,&'a [u32]) {
    let (cols,rows,..)=geom(a,v);
    let first=((v.screen*cols*rows).max(0) as usize).min(nb.page_ids.len());
    let end=(first+(cols*rows) as usize).min(nb.page_ids.len());
    (first,&nb.page_ids[first..end])
}

// Only the screenful's decoded previews stay, and the open page's (made from its canvas as the view
// opened: its preview on disk may be older, and no job makes it while it is open).
fn retain_screen(v:&mut PagesView,a:&App,nb:&NotebookInfo) {
    let mut keep=screen_pages(v,a,nb).1.to_vec();
    if let Some(&open)=nb.page_ids.get(v.current.max(0) as usize) {keep.push(open);}
    v.cache.retain(&keep);
}

/// New previews came while the grid is up, or a batch of them ended: only the tiles that show something
/// else now are drawn again (a preview in a loading mark's place, a loading mark whose batch could not make
/// its preview giving way to the page's number), and the caller refreshes the rect returned. RECT_EMPTY when
/// none did, or when no grid shows (a keyboard, the card over it): it is drawn whole when that goes.
pub fn pagesview_repaint_tiles(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav)->Rect {
    if !v.open || !v.unified || v.index_tab || v.overlay || v.kb.open || v.context.is_some() || v.drag.is_some() || v.view_size!=(a.w,a.h) {
        return RECT_EMPTY;
    }
    let (first,pages)=screen_pages(v,a,nb);
    let changed=v.cache.changed(nb,pages,&mut v.previews);
    let only:Vec<i32>=(0..pages.len()).filter(|&k|changed.contains(&pages[k])).map(|k|(first+k) as i32).collect();
    let r=if only.is_empty() {RECT_EMPTY} else {redraw_tiles(v,a,nb,nav,Some(&only))};
    retain_screen(v,a,nb);
    r
}

// A tap changed the ticks (sel as it was before): the tiles that changed, and the bottom row when its
// buttons did, are drawn again and refreshed on their own, not the whole view.
fn repaint_ticks(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,before:&[bool]) {
    let npages=nb.npages();
    let changed:Vec<i32>=(0..npages).filter(|&i|before.get(i as usize)!=v.sel.get(i as usize)).collect();
    let k_was=before.iter().take(npages as usize).filter(|&&t|t).count() as i32;
    let r=redraw_tiles(v,a,nb,nav,Some(&changed));
    if !rect_is_empty(r) {app_invalidate(a,r,RefreshMode::Ui);}
    footer_after_ticks(v,a,nb,nav,k_was);
}

// The bottom row drawn again and refreshed on its own when the ticks changed what it offers (Copy page for one,
// Select none for all, Move to... and Copy to notebook... for some, Undo and Redo for none).
fn footer_after_ticks(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,k_was:i32) {
    let npages=nb.npages();
    let k=ticked_in(v,npages);
    if (k_was==1)!=(k==1) || (k_was==0)!=(k==0) || (k_was==npages)!=(k==npages) {
        // (on its own: a tile near the top and the bottom row never merge into one refresh of the screen)
        app_flush(a);
        let (cols,rows,..)=geom(a,v);
        let screens=((npages+cols*rows-1)/(cols*rows)).max(1);
        let undo=undo_id(v,nb,nav,k);
        let redo=redo_id(v,nb,k);
        draw_footer(v,a,screens,k,k==npages,undo,redo);
        let y=a.h-ui_px(170.0);
        app_invalidate(a,Rect::new(0,y,a.w,a.h-y),RefreshMode::Ui);
    }
}

// A held tile's card: Open, Add a link, Rename (its naming keyboard), Move to..., Copy to notebook...,
// Delete (those three: the ticked pages when it is one of them, else it alone) and Cancel.
fn draw_context(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav,page:i32) {
    hits_clear(&mut v.hits);
    let card=card_rect(a,v);let w=card.w;
    ui_frame(&mut a.screen,card,ui_px(18.0),255,true,0);
    ui_frame(&mut a.screen,card,ui_px(18.0),0,false,ui_px(3.0));
    // Under its title, how many pages link to it, here and in other notebooks (2.0.0-beta.2; the Links bar lists them).
    let from=context_from(nb,nav,page);
    let title_h=if from>0 {ui_px(72.0)} else {ui_px(100.0)};
    font_draw_centered(&mut a.screen,FontFace::Bold,ui_title_size(),Rect::new(card.x,card.y,w,title_h),&cformat!(tr("Page %d"),page+1),0);
    if from>0 {
        // TRANSLATORS: under a held page's title in the Pages view (about 900 px at 34 px): how many pages, in this
        // notebook or others, have links to it.
        let line=cformat!(ng_("Linked from %d page","Linked from %d pages",from as i64),from);
        font_draw_centered(&mut a.screen,FontFace::Regular,ui_small_size(),Rect::new(card.x,card.y+ui_px(62.0),w,ui_px(44.0)),&line,0);
    }
    let k=ticked_in(v,nb.npages());
    let del=if k>1 && pagesview_ticked(v,page) {cformat!(tr("Delete (%d)"),k)}else{tr("Delete").to_string()};
    // TRANSLATORS: a held page's card in the Pages view, under its icon (950 px at 34 px): give the page
    // a name (its naming keyboard, titled Page name).
    let rename=tr("Rename");
    // TRANSLATORS: a held page's card in the Pages view, under its icon (950 px at 34 px): move the page
    // (or the ticked pages, with how many) to another place in the notebook, on the Go to page number pad.
    let moving=if k>1 && pagesview_ticked(v,page) {cformat!(ng_("Move %d page to...","Move %d pages to...",k as i64),k)}else{tr("Move to...").to_string()};
    // TRANSLATORS: the same card: copy the page (or the ticked pages) into another notebook, picked from
    // the notebooks and folders.
    let copying=if k>1 && pagesview_ticked(v,page) {cformat!(ng_("Copy %d page to notebook...","Copy %d pages to notebook...",k as i64),k)}else{tr("Copy to notebook...").to_string()};
    for (row,id,icon,label) in [(0,PG_CONTEXT_OPEN,IconKind::None,tr("Open")),(1,PG_CONTEXT_LINK,IconKind::LinkPlus,tr("Add a link")),
        (2,PG_CONTEXT_RENAME,IconKind::Rename,rename),(3,PG_CONTEXT_MOVE,IconKind::Grip,moving.as_str()),
        (4,PG_CONTEXT_COPY_TO,IconKind::Notebook,copying.as_str()),(5,PG_CONTEXT_DELETE,IconKind::Trash,del.as_str()),(6,PG_CONTEXT_CANCEL,IconKind::None,tr("Cancel"))] {
        let rect=Rect::new(card.x+ui_px(24.0),card.y+ui_px(110.0)+row*ui_px(130.0),w-ui_px(48.0),ui_px(120.0));
        let enabled=nb.page_ids.get(page as usize).is_some() && (id!=PG_CONTEXT_RENAME || nav.writable);
        ui_button(&mut a.screen,rect,icon,label,if enabled{ButtonStyle::Plain}else{ButtonStyle::Disabled});
        if enabled{hits_add(&mut v.hits,rect,id,0);}
    }
    a.toast_floor=card.y;
}

// Where a held tile's card is drawn: centred.
fn card_rect(a:&App,_v:&PagesView)->Rect {
    card_rect_at((a.w,a.h))
}
fn card_rect_at((aw,ah):(i32,i32))->Rect {
    let w=ui_px(1000.0).min(aw-ui_px(60.0));let h=ui_px(1040.0);
    Rect::new((aw-w)/2,(ah-h)/2,w,h)
}

// How many pages link to page index `page` of nb: its own pages (nav) and other notebooks' (the backlinks cache).
fn context_from(nb:&NotebookInfo,nav:&Nav,page:i32)->i32 {
    nb.page_ids.get(page.max(0) as usize).filter(|_|page>=0&&nav.links_writable).map_or(0,|&pid|backlinks_for_page(nb,nav,pid).len() as i32)
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
    // A contact made while a batch was being written (quiet_until): let go, down to its lift.
    if phase == 0 {
        v.quiet_contact = v.quiet_until.is_some_and(|until| until.wrapping_sub(ev.t_ms) as i32 >= 0);
        if !v.quiet_contact {
            v.quiet_until = None;
        }
    }
    if v.quiet_contact {
        if phase == 2 {
            v.quiet_contact = false;
        }
        return PvResult::None;
    }
    if v.kb.open && v.adding {
        let r = app_keyboard_pointer(a, &mut v.kb, phase, ev);
        return add_pad_result(v, a, nb, nav, r);
    }
    if v.kb.open {
        // A key costs what it does on the Library's keyboard (app_keyboard_pointer): what it changed,
        // the text line and then the key, each on its own fast refresh, and nothing for a sample that
        // changed nothing. The view under the keys is not drawn again until the keyboard closes.
        let r = app_keyboard_pointer(a, &mut v.kb, phase, ev);
        if r == KbResult::Done {
            if v.naming {
                return PvResult::Name;
            }
            if v.moving {
                // Before page N, or N = npages + 1 (an empty pad too): at the end.
                let text = v.kb.text.as_bytes();
                let (page, end) = strtol10(text);
                let n = nb.npages() as i64;
                if text.is_empty() || (end == text.len() && page >= 1 && page <= n + 1) {
                    v.target = if text.is_empty() { n as i32 } else { page as i32 - 1 };
                    v.moving = false;
                    keyboard_close(&mut v.kb);
                    app_toast_dismiss(a); // (its hint goes with it: the move says nothing once done)
                    return PvResult::Move;
                }
                v.kb.open = true;
                move_hint(a, nb);
                let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
                keyboard_draw(&mut v.kb, &mut a.screen, screen);
                app_invalidate(a, v.kb.area, RefreshMode::Fast);
                return PvResult::None;
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
            if std::mem::take(&mut v.moving) {
                app_toast_dismiss(a); // (its hint goes with it)
            }
            redraw(v, a, nb, nav, RefreshMode::Ui);
        }
        return PvResult::None;
    }
    if phase == 0 {
        v.held = false;
        v.down_k = in_sample_ms(ev);
        v.press_mods = ev.key_mods;
        press_begin(&mut v.press, ev.x, ev.y, ev.t_ms);
        if let Some(&h) = hits_find(&v.hits, ev.x, ev.y).filter(|h| h.id != PG_OPEN && h.id != PG_TOGGLE) {
            app_press_feedback(a, h.r);
            v.pressed = Some(h.r);
        }
        return PvResult::None;
    }
    let was_moved = v.press.moved;
    press_move(&mut v.press, ev.x, ev.y);
    if phase == 1 {
        if v.drag.is_some() {
            drag_step(v, a, nb, ev);
        } else if v.press.moved && !was_moved {
            drag_begin(v, a, nb, nav, ev);
        }
        return PvResult::None;
    }
    v.press.down = false;
    let back = app_press_release(a, &mut v.pressed);
    if !rect_is_empty(back) {
        app_invalidate(a, back, RefreshMode::Ui);
    }
    if let Some(d) = v.drag.take() {
        return drag_drop(v, a, nb, nav, d, ev);
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
    // New page held (the tick may not have seen it yet), or tapped with a keyboard's modifier key held: the batch's pad.
    if !v.picking && hit.id==PG_NEW && (ev.key_mods&ADD_MODS!=0 || in_sample_ms(ev).wrapping_sub(v.down_k)>=ADD_HOLD_MS) {
        open_add_pad(v,a,nb,nav);return PvResult::None;
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
        // Move to... and Copy to notebook...: the same pages as Delete's.
        PG_CONTEXT_MOVE | PG_CONTEXT_COPY_TO => {
            if let Some(page)=v.context.take() {
                if !pagesview_ticked(v,page) {v.sel.fill(false);if let Some(t)=v.sel.get_mut(page as usize){*t=true;}}
                if hit.id==PG_CONTEXT_COPY_TO {return PvResult::CopyTo;}
                open_move_pad(v,a,nb,nav);
            }
        }
        PG_MOVE if ticked_in(v,nb.npages())>0 => open_move_pad(v,a,nb,nav),
        PG_UNDO_MOVE => return PvResult::UndoMove,
        PG_UNDO_ADD => return PvResult::UndoAdd,
        PG_REDO => return PvResult::Redo,
        PG_COPY_TO if ticked_in(v,nb.npages())>0 => return PvResult::CopyTo,
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
        // A tap: one page at once, as a batch of one (its toast, its Undo); the view stays.
        PG_NEW => {
            v.target = new_page_at(v, nb);
            v.add_count = 1;
            return PvResult::AddPages;
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

// ---- a held tile dragged to a gap (2.0.0-beta.2) ----

/// How long the contact stays over the top bar or the bottom row before the drag turns the screen.
const DRAG_TURN_MS: u32 = 700;

// The grid's band: between the top bar and the bottom row. The gap's bar stays inside it.
fn grid_band(a: &App) -> Rect {
    let top = ui_px(190.0);
    Rect::new(0, top, a.w, a.h - top - ui_px(170.0))
}

// Where pages dropped at (x, y) go: before page `to` (npages: at the end), and the bar that shows that
// gap (between two tiles, or after the last). None off the grid's band.
fn drop_at(v: &PagesView, a: &App, nb: &NotebookInfo, x: f32, y: f32) -> Option<(i32, Rect)> {
    let band = grid_band(a);
    if !rect_contains(band, x as i32, y as i32) || nb.npages() < 1 {
        return None;
    }
    let (cols, rows, cw, ch, _) = geom(a, v);
    let gap = ui_px(24.0);
    let col = ((x as i32 - gap / 2) / (cw + gap)).clamp(0, cols - 1);
    let row = ((y as i32 - band.y - gap / 2) / (ch + gap)).clamp(0, rows - 1);
    let per = cols * rows;
    let first = v.screen * per;
    let n = nb.npages();
    let mut slot = row * cols + col;
    let (to, right) = if first + slot >= n {
        slot = n - 1 - first; // (past the last tile: after it)
        (n, true)
    } else {
        let (cell, ..) = tile_rects(v, a, nb, slot);
        if (x as i32) < cell.x + cell.w / 2 { (first + slot, false) } else { (first + slot + 1, true) }
    };
    if slot < 0 {
        return None;
    }
    let (cell, ..) = tile_rects(v, a, nb, slot);
    let bw = ui_px(12.0);
    let bx = if right { cell.x + cell.w + gap / 2 } else { cell.x - gap / 2 } - bw / 2;
    Some((to, Rect::new(bx, cell.y, bw, cell.h)))
}

// The gap's bar drawn over the view at the contact's place, clipped to the grid's band (never over the top
// bar or the bottom row): where it is (`shown`, empty off the grid) and the gap (`to`) set.
fn drag_paint(v: &mut PagesView, a: &mut App, nb: &NotebookInfo) -> Rect {
    let Some(mut d) = v.drag.take() else { return RECT_EMPTY };
    let band = grid_band(a);
    let at = drop_at(v, a, nb, d.pos.0, d.pos.1);
    d.to = at.map(|(to, _)| to);
    let bar = at.map_or(RECT_EMPTY, |(_, bar)| rect_intersect(bar, band));
    if !rect_is_empty(bar) {
        canvas_fill(&mut a.screen, bar, 0);
    }
    d.shown = bar;
    v.drag = Some(d);
    bar
}

// The view was drawn again under a drag (it began, the screen turned, or the editor drew it whole): kept
// as what the bar moves over, and the bar drawn on it. A view that cannot be kept ends the drag.
fn drag_under(v: &mut PagesView, a: &mut App, nb: &NotebookInfo) {
    let Some(d) = v.drag.as_mut() else { return };
    if d.under.w != a.w || d.under.h != a.h {
        if canvas_init(&mut d.under, a.w, a.h) != 0 {
            v.drag = None;
            return;
        }
    }
    canvas_blit(&mut d.under, &a.screen, Rect::new(0, 0, a.w, a.h), 0, 0);
    d.shown = RECT_EMPTY;
    drag_paint(v, a, nb);
}

// A hold on a tile (its card up, or held 600 ms and the card not drawn yet) moved past the slop: the tile
// is picked up, the ticked pages with it when it is one of them. The card goes in the same refresh.
fn drag_begin(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, ev: &InEvent) {
    if !v.open || !v.unified || v.picking || v.index_tab || v.overlay || v.kb.open || touch_cancelled(ev) {
        return;
    }
    let page = match v.context {
        Some(p) if v.held => p,
        Some(_) => return, // (a card left up: this contact is on it)
        None if in_sample_ms(ev).wrapping_sub(v.down_k) >= 600 => {
            match hits_find(&v.hits, v.press.x0, v.press.y0).filter(|h| h.id == PG_OPEN) {
                Some(h) => h.arg,
                None => return,
            }
        }
        None => return,
    };
    let (cols, rows, ..) = geom(a, v);
    let slot = page - v.screen * cols * rows;
    if page < 0 || page >= nb.npages() || slot < 0 || slot >= cols * rows {
        return;
    }
    let card = if v.context.is_some() { card_rect(a, v) } else { RECT_EMPTY };
    let pages: Vec<i32> = if pagesview_ticked(v, page) { (0..nb.npages()).filter(|&i| pagesview_ticked(v, i)).collect() } else { vec![page] };
    v.context = None;
    v.held = false;
    v.last_tap = None;
    v.drag = Some(PageDrag {
        pages,
        pos: (ev.x, ev.y),
        ..Default::default()
    });
    pagesview_draw(v, a, nb, nav); // (the card gone; drag_under keeps the view and draws the bar)
    let shown = v.drag.as_ref().map_or(RECT_EMPTY, |d| d.shown);
    app_invalidate(a, rect_union(card, shown), if rect_is_empty(card) { RefreshMode::Fast } else { RefreshMode::Ui });
}

// The contact moved while dragging: the bar moves when the gap changes (only the bar's old and new places are
// refreshed, fast, each on its own: two bars far apart never make one refresh of what lies between; the same
// gap refreshes nothing), and over the top bar or the bottom row the screen turns after a while
// (pagesview_tick).
fn drag_step(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, ev: &InEvent) {
    let band = grid_band(a);
    let Some(d) = v.drag.as_mut() else { return };
    let edge = if ev.y < band.y as f32 {
        Some(false)
    } else if ev.y >= (band.y + band.h) as f32 {
        Some(true)
    } else {
        None
    };
    if edge != d.edge.map(|e| e.0) {
        d.edge = edge.map(|e| (e, now_ms()));
    }
    let step = ui_px(4.0) as f32;
    if (ev.x - d.pos.0).abs() < step && (ev.y - d.pos.1).abs() < step {
        return;
    }
    d.pos = (ev.x, ev.y);
    let old = d.shown;
    let bar = drop_at(v, a, nb, ev.x, ev.y).map_or(RECT_EMPTY, |(_, bar)| rect_intersect(bar, band));
    if bar == old {
        // (the same gap: the bar stays; `to` is that gap's already)
        return;
    }
    if let Some(d) = v.drag.as_ref() {
        canvas_blit(&mut a.screen, &d.under, old, old.x, old.y);
    }
    let drawn = drag_paint(v, a, nb);
    if !rect_is_empty(old) && !rect_is_empty(drawn) && rect_is_empty(rect_intersect(old, drawn)) {
        app_invalidate(a, old, RefreshMode::Fast);
        app_flush(a);
        app_invalidate(a, drawn, RefreshMode::Fast);
    } else {
        app_invalidate(a, rect_union(old, drawn), RefreshMode::Fast);
    }
}

// The drag's lift: dropped in a gap that changes the order, the editor moves the pages carried (`carry`,
// PvResult::Move, one undo step, the view drawn once; drawn again too if the move fails). The gap is the
// one the bar shows (`to`, as drawn: drag_step skips moves under 4 px, so the lift's own place can be a
// few pixels past it). Lifted off the grid, back in their own place, or cancelled (a second finger, a
// lost lift): nothing moves, the view is drawn again.
fn drag_drop(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, d: PageDrag, ev: &InEvent) -> PvResult {
    v.held = false;
    let at = if touch_cancelled(ev) { None } else { d.to };
    if let Some(to) = at {
        if store_move_order(&nb.page_ids, &d.pages, to).is_some_and(|o| o != nb.page_ids) {
            v.carry = d.pages;
            v.target = to;
            return PvResult::Move;
        }
    }
    redraw(v, a, nb, nav, RefreshMode::Ui);
    PvResult::None
}

// While dragging over the top bar or the bottom row: the previous or next screen after DRAG_TURN_MS, and
// again each DRAG_TURN_MS it stays (only the grid's band is refreshed: the bars do not change).
fn drag_tick(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, now: u32) {
    let Some((next, since)) = v.drag.as_ref().and_then(|d| d.edge) else { return };
    if now.wrapping_sub(since) < DRAG_TURN_MS {
        return;
    }
    if let Some(d) = v.drag.as_mut() {
        d.edge = Some((next, now));
    }
    let (cols, rows, ..) = geom(a, v);
    let screens = ((nb.npages() + cols * rows - 1) / (cols * rows)).max(1);
    let to = v.screen + if next { 1 } else { -1 };
    if to < 0 || to >= screens {
        return;
    }
    v.screen = to;
    pagesview_draw(v, a, nb, nav);
    app_invalidate(a, grid_band(a), RefreshMode::Ui);
}

// Move to...: Go to page's number pad, for the page the ticked pages go before.
fn open_move_pad(v:&mut PagesView,a:&mut App,nb:&NotebookInfo,nav:&Nav) {
    v.naming=false;v.moving=true;
    // TRANSLATORS: the title in Move to...'s number-pad field (bold, 44 px, about 700 px): the page
    // number typed is the page the moved pages go before.
    keyboard_open(&mut v.kb,tr("Move before page"),b"");
    v.kb.page=2;
    redraw(v,a,nb,nav,RefreshMode::Ui);
    move_hint(a,nb);
}

// What Move to...'s pad takes, as a toast above its keys.
fn move_hint(a:&mut App,nb:&NotebookInfo) {
    let n=nb.npages();
    // TRANSLATORS: a toast over Move to...'s number pad (about 1700 px at 44 px in bold, or two lines):
    // the first %d is the notebook's page count, the second one more, which puts the pages at the end.
    app_toast(a,&cformat!(tr("Enter a page from 1 to %d, or %d for the end."),n,n+1),4000);
}

// Where New page puts a page (and a batch its first): after the last page ticked, else after the open
// page. Never first, so the Library's card stays the same page.
fn new_page_at(v: &PagesView, nb: &NotebookInfo) -> i32 {
    let after = last_ticked(v, nb);
    (if after >= 0 { after } else { v.current }) + 1
}

/// The count pad's Done: "Add N pages" for the count typed (the toolbar's count pad's too).
pub fn add_label(n: i32) -> String {
    // TRANSLATORS: the confirm button at the bottom of New page's count pad (bold, 44 px, about 1700 px):
    // %d is how many blank pages it adds at once.
    cformat!(ng_("Add %d page", "Add %d pages", n as i64), n)
}

// New page held (or tapped with a keyboard's modifier key held): the count pad, for a batch of new pages. The press that
// opened it ends here (its lift does nothing). A full notebook says so instead.
fn open_add_pad(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) {
    v.press.down = false;
    v.pressed = None;
    v.held = true;
    v.last_tap = None;
    let room = store_room_for_pages(nb);
    if room < 1 {
        redraw(v, a, nb, nav, RefreshMode::Ui);
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): New page held, in a notebook
        // that already has the most pages a notebook can have (%d).
        app_toast(a, &cformat!(tr("Notebook full with %d pages."), STORE_MAX_PAGES as i32), 3000);
        return;
    }
    v.naming = false;
    v.moving = false;
    v.adding = true;
    // TRANSLATORS: the title in New page's count pad (bold, 44 px, about 700 px): how many blank pages to
    // add at once, typed on the number pad or changed by one with its - and + keys.
    keyboard_open_count(&mut v.kb, tr("Pages to add"), room, add_label);
    redraw(v, a, nb, nav, RefreshMode::Ui);
}

// What a key of the count pad did: Done adds the count (PvResult::AddPages, the pad closed), Cancel closes
// it with nothing added. A count held at the most the notebook can take shows as that count (no toast: the
// owner, 2026-11-04).
fn add_pad_result(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, r: KbResult) -> PvResult {
    match r {
        KbResult::Done => {
            v.add_count = keyboard_count(&v.kb);
            v.adding = false;
            keyboard_close(&mut v.kb);
            v.target = new_page_at(v, nb);
            PvResult::AddPages
        }
        KbResult::Cancel => {
            v.adding = false;
            app_toast_dismiss(a);
            redraw(v, a, nb, nav, RefreshMode::Ui);
            PvResult::None
        }
        _ => PvResult::None,
    }
}

/// A hardware keyboard's key (Linux evdev code) while New page's count pad is up: the digits (the top row's
/// and the keypad's) type, Backspace deletes, - and + (the keypad's too) change the count by one, Enter is
/// the pad's Done and Esc its Cancel; Up and Down do nothing. Some(result) while the pad is up, whatever the key (no
/// other key reaches what is under it); None when it is not up.
pub fn pagesview_key(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, code: i32) -> Option<PvResult> {
    if !(v.open && v.kb.open && v.adding) {
        return None;
    }
    let Some(key) = count_pad_key(code) else { return Some(PvResult::None) };
    let r = app_keyboard_press(a, &mut v.kb, key);
    Some(add_pad_result(v, a, nb, nav, r))
}

/// Esc (`back`) in the view, before it closes: a drag is cancelled, else a held tile's card closes, else the ticks
/// clear. True when it ate the press (the view stays); false with none of them, and the view closes.
pub fn pagesview_back(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> bool {
    pagesview_cancel_drag(v, a, nb, nav) || pagesview_close_card(v, a, nb, nav) || pagesview_clear_ticks(v, a, nb, nav)
}

/// A count pad's key for a hardware keyboard's (Linux evdev code): the digits (the top row's and the keypad's),
/// Backspace, - and + (the keypad's too), Enter (Done) and Esc (Cancel); None for any other. Up and Down no
/// longer step it (2.0.0-beta.2: they do nothing but in the Graph). The toolbar's count pad (editor.rs) reads
/// keys through it too.
pub fn count_pad_key(code: i32) -> Option<u32> {
    Some(match code {
        2..=10 => '1' as u32 + (code - 2) as u32,
        11 | 82 => '0' as u32,
        79..=81 => '1' as u32 + (code - 79) as u32,
        75..=77 => '4' as u32 + (code - 75) as u32,
        71..=73 => '7' as u32 + (code - 71) as u32,
        14 => KB_KEY_BKSP,
        28 | 96 => KB_KEY_DONE,
        1 => KB_KEY_CANCEL,
        12 | 74 => KB_KEY_DEC,
        13 | 78 => KB_KEY_INC,
        _ => return None,
    })
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
    if v.open && !v.picking && !v.kb.open && v.context.is_none() && v.drag.is_none() && v.press.down && !v.press.moved
        && now.wrapping_sub(v.press.t0)>=ADD_HOLD_MS
        && hits_find(&v.hits,v.press.x0,v.press.y0).is_some_and(|h|h.id==PG_NEW) {
        open_add_pad(v,a,nb,nav);
    }
    if v.open && v.unified && !v.picking && !v.kb.open && v.context.is_none() && v.press.down && !v.press.moved
        && now.wrapping_sub(v.press.t0)>=600 {
        if let Some(hit)=hits_find(&v.hits,v.press.x0,v.press.y0).copied().filter(|h|h.id==PG_OPEN) {
            v.context=Some(hit.arg);v.held=true;v.last_tap=None;redraw(v,a,nb,nav,RefreshMode::Ui);
        }
    }
    if v.open && v.drag.is_some() {
        drag_tick(v, a, nb, nav, now);
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
    if key=="context.from" {csnprintf!(out,cap,"%d",v.context.map_or(0,|p|context_from(nb,nav,p)));return 0;}
    if key=="overlay" {csnprintf!(out,cap,"%d",v.overlay as i32);return 0;}
    if key=="multi" {csnprintf!(out,cap,"%d",v.multi_select as i32);return 0;}
    if key=="cached" {csnprintf!(out,cap,"%d",v.cache.len());return 0;}
    // What the screenful's tiles show, one letter each: p a preview, l the loading mark, n the number alone
    // (nine tiles a screenful in the Pages view, more when picking a link's page).
    if key=="shows" {
        out.clear();
        if v.open && v.unified && !v.index_tab {
            let (cols,rows,..)=geom_at(v.view_size,v);
            let per=(cols*rows).max(1) as usize;let first=((v.screen.max(0) as usize)*per).min(nb.page_ids.len());
            let end=(first+per).min(nb.page_ids.len());
            csnprintf!(out,cap,"%s",&v.cache.shows(&nb.page_ids[first..end]));
        }
        return 0;
    }
    // The ticked pages (from 1, joined by ","; empty for none).
    if key=="ticks" {
        let t:Vec<String>=(0..nb.npages()).filter(|&i|pagesview_ticked(v,i)).map(|i|(i+1).to_string()).collect();
        csnprintf!(out,cap,"%s",&t.join(","));return 0;
    }
    // The held tile's card, "x y w h" (empty while none is up).
    if key=="card" {
        out.clear();
        let r=v.context.map(|_|card_rect_at(v.view_size));
        if let Some(r)=r {csnprintf!(out,cap,"%d %d %d %d",r.x,r.y,r.w,r.h);}
        return 0;
    }
    // A drag: the pages carried (from 1, joined by ","), then the gap ("before N", "end", or "off" off the grid).
    if key=="drag" {
        out.clear();
        if let Some(d)=&v.drag {
            let pages:Vec<String>=d.pages.iter().map(|p|(p+1).to_string()).collect();
            let gap=match d.to {Some(t) if t>=nb.npages()=>"end".to_string(),Some(t)=>format!("before {}",t+1),None=>"off".to_string()};
            csnprintf!(out,cap,"%s %s",&pages.join(","),&gap);
        }
        return 0;
    }
    // Whether the bottom row offers Undo for the last page move (nothing ticked, the move still the newest change).
    if key=="undo" {csnprintf!(out,cap,"%d",undo_offered(v,nb,nav,ticked_in(v,nb.npages())) as i32);return 0;}
    // Which: "add" (the last batch of new pages), "move", or empty.
    if key=="undo.kind" {
        let kind=match undo_id(v,nb,nav,ticked_in(v,nb.npages())) {Some(PG_UNDO_ADD)=>"add",Some(_)=>"move",None=>""};
        csnprintf!(out,cap,"%s",kind);return 0;
    }
    // Whether there is a Redo (the `redo` action's, and the bottom row's with nothing ticked): 1 or 0; and which,
    // "move", "add" or empty.
    if key=="redo" {csnprintf!(out,cap,"%d",pagesview_redo_offered(v,nb) as i32);return 0;}
    if key=="redo.kind" {
        let kind=match &v.redo {Some(r) if r.applies(nb)=>match r {PageRedo::Move(_)=>"move",PageRedo::Add(_)=>"add"},_=>""};
        csnprintf!(out,cap,"%s",kind);return 0;
    }
    // New page's count pad: 1 while it is up; "add.done" its Done's label (the count in it), "add.done.on" 1 when
    // Done can be pressed (a count of 1 or more).
    // Where `tap panel pages.NAME` taps, "X Y" (empty for none): a script holds New page there.
    if let Some(name)=key.strip_prefix("at.") {
        out.clear();
        if let Some((x,y))=pagesview_control_point(v,name) {csnprintf!(out,cap,"%d %d",x as i32,y as i32);}
        return 0;
    }
    if key=="adding" {csnprintf!(out,cap,"%d",(v.open&&v.kb.open&&v.adding) as i32);return 0;}
    if key=="add.done" || key=="add.done.on" {
        out.clear();
        if v.open && v.kb.open && v.adding {
            let n=keyboard_count(&v.kb);
            if key=="add.done" {csnprintf!(out,cap,"%s",&add_label(n));} else {csnprintf!(out,cap,"%d",(n>=1) as i32);}
        }
        return 0;
    }
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
    // Its keyboard's keys while it is up (the number pad's: key.0 to key.9, key.done, key.del, key.cancel).
    if let Some(k)=name.strip_prefix("key.") {
        if !v.kb.open {return None;}
        let code=match k {"done"=>KB_KEY_DONE,"del"=>KB_KEY_BKSP,"cancel"=>KB_KEY_CANCEL,"dec"=>KB_KEY_DEC,"inc"=>KB_KEY_INC,c if c.chars().count()==1=>c.chars().next()? as u32,_=>return None};
        let mut r=RECT_EMPTY;
        return keyboard_key_rect(&v.kb,code,&mut r).then(||((r.x+r.w/2) as f32,(r.y+r.h/2) as f32));
    }
    let (id,arg)=match name {"link"=>(PG_CONTEXT_LINK,0),"open"=>(PG_CONTEXT_OPEN,0),"cancel"=>(PG_CONTEXT_CANCEL,0),"back"=>(PG_BACK,0),"graph"=>(PG_GRAPH,0),"list"=>(PG_LIST,0),"pages"=>(PG_TAB,0),"new"=>(PG_NEW,0),"copy"=>(PG_COPY,0),"all"=>(PG_ALL,0),"multi"=>(PG_MULTI,0),"goto"=>(PG_JUMP,0),"prev"=>(PG_PREV,0),"next"=>(PG_NEXT,0),"rename"=>(PG_CONTEXT_RENAME,0),"delete"=>(PG_CONTEXT_DELETE,0),"move-to"=>(PG_CONTEXT_MOVE,0),"copy-to"=>(PG_CONTEXT_COPY_TO,0),"move"=>(PG_MOVE,0),"copy-to-notebook"=>(PG_COPY_TO,0),"undo-move"=>(PG_UNDO_MOVE,0),"undo-add"=>(PG_UNDO_ADD,0),"redo"=>(PG_REDO,0),n if n.starts_with("page.")=>(PG_OPEN,n[5..].parse::<i32>().ok()?-1),_=>return None};
    v.hits.h.iter().find(|h|h.id==id&&h.arg==arg).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32))
}

pub fn pagesview_grid(v:&PagesView,a:&App)->(i32,i32) {let (cols,rows,..)=geom(a,v);(cols,rows)}
