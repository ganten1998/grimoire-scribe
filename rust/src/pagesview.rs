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
}

/// (The C keeps pointers to the editor's notebook and Nav, nb and nav; here the editor passes them
/// to each call that reads them: pagesview_draw, pagesview_pointer, pagesview_probe. PORTING.md,
/// Ownership.)
#[derive(Clone, Debug, Default)]
pub struct PagesView {
    pub open: bool,
    /// the open page's index; the editor keeps it up to date
    pub current: i32,
    /// Go to page...'s number pad
    pub kb: Keyboard,
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
}

/// Opens it on notebook nb, on the screenful with page `current`, nothing ticked. False when there
/// is no memory for its ticks (it stays closed).
pub fn pagesview_open(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32) -> bool {
    todo!()
}
/// The same to pick where a link on the open page goes: had, a link it had (else one drawn now).
pub fn pagesview_pick(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32, had: bool) -> bool {
    todo!()
}
/// The same on its Index tab with the naming keyboard up for the open page, as Name this page opens
/// it, to go back to the page after it (name_only). False also when the names cannot be changed (a
/// nav.ini that does not read: that button is greyed out then).
pub fn pagesview_name(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, current: i32) -> bool {
    todo!()
}
/// Draws the whole screen into a->screen (the caller refreshes it).
pub fn pagesview_draw(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav) {
    todo!()
}
/// Pointer phase 0 down / 1 move / 2 up. What it draws itself (a tick, another screenful, the
/// number pad) it refreshes itself; for the rest, see PvResult. While its keyboard is up a key draws
/// and refreshes only the keyboard, as the Library's does (app_keyboard_pointer).
pub fn pagesview_pointer(v: &mut PagesView, a: &mut App, nb: &NotebookInfo, nav: &Nav, phase: i32, ev: &InEvent) -> PvResult {
    todo!()
}
/// From the editor's tick: once typing on its keyboard pauses, the keyboard's grayscale refresh
/// (app_keyboard_tick). Nothing else.
pub fn pagesview_tick(v: &mut PagesView, a: &mut App, now: u32) {
    todo!()
}
pub fn pagesview_ticked_count(v: &PagesView) -> i32 {
    todo!()
}
pub fn pagesview_ticked(v: &PagesView, index: i32) -> bool {
    todo!()
}
pub fn pagesview_untick(v: &mut PagesView) {
    todo!()
}
/// Closes it (and its number pad). Its ticks' memory stays for the next open.
pub fn pagesview_close(v: &mut PagesView) {
    todo!()
}
pub fn pagesview_free(v: &mut PagesView) {
    todo!()
}
/// The headless runner's keys for it, without "editor." (docs/SIM.md): "tile.N", the label under
/// page N's tile (from 1) as drawn, its two lines joined by " | " ("Page 3 | p. 12"); empty while it
/// is closed. "picking": "new" or "had" while it picks where a link goes, else empty. "keyboard":
/// its keyboard's title while that is up ("Page name", "Go to page"), else empty; "text" what it
/// holds. 0, or -1 for a key it does not know.
pub fn pagesview_probe(v: &PagesView, nb: &NotebookInfo, nav: &Nav, key: &str, out: &mut String, cap: usize) -> i32 {
    todo!()
}
