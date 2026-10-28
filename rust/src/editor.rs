//! editor.c: the notebook editor. Live ink, stroke eraser, the floating panel, page
//! navigation, paper changes, autosave.
//!
//! Pixels: page_canvas holds paper + ink at page resolution: the notebook's page size, the screen
//! it was made on (a notebook from another Scribe keeps its own). The view maps page -> screen
//! (zoom + pan): the page's width fits the screen's (100% upright for a notebook made on this
//! screen), pinch to zoom up to 4x that. a->screen = the viewed page + panel (+ paper sheet).
//! compose(r) rebuilds a screen rect from those layers; show(page rect) is the page-side entry.

use crate::prelude::*;
use std::cell::Cell;
#[cfg(feature = "sim")]
use std::cell::RefCell;
use std::rc::Rc;

// The grayscale settle (see EditorState.settle): this long after a stroke with the pen still near,
// or once the pen has been gone this long.
const SETTLE_NEAR_MS: u32 = 1500;
const SETTLE_LEFT_MS: i32 = 150;
// A finger held this long on a link (without moving) opens it to change or remove; a shorter tap
// follows it.
const LINK_HOLD_MS: u32 = 600;
// A link's box drawn smaller than this both ways is no box: it is taken back.
const LINK_BOX_MIN_MM: f32 = 5.0;

// The hits' ids (the panel's, the paper sheet's and the tool picker's).
#[allow(dead_code)]
const P_NONE: i32 = 0;
const P_TOOL: i32 = 1;
const P_UNDO: i32 = 2;
const P_REDO: i32 = 3;
const P_PREV: i32 = 4;
const P_NEXT: i32 = 5;
const P_PAPER: i32 = 6;
const P_PENSET: i32 = 7;
const P_LIBRARY: i32 = 8;
const P_COLLAPSE: i32 = 9;
const P_GRIP: i32 = 10;
const P_EXPAND: i32 = 11;
const P_PICK_PAPER: i32 = 12;
const P_PICK_CUSTOM: i32 = 13;
const P_PICK_CANCEL: i32 = 14;
const P_CLEAR: i32 = 15;
const P_DISPLAY: i32 = 16;
const P_PAGES: i32 = 17;
const P_TOOLPICK: i32 = 18;
const P_PICK_TOOL: i32 = 19;
const P_PIN_MODE: i32 = 20;
const P_PICK_INK: i32 = 21;
const P_PICK_MORE: i32 = 22;
const P_PICK_PHOTOS: i32 = 23;

/// Which input started the current gesture, and what it does.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Owner {
    #[default]
    None = 0, // OWN_NONE
    Draw = 1,      // OWN_DRAW
    Erase = 2,     // OWN_ERASE
    Panel = 3,     // OWN_PANEL
    Modal = 4,     // OWN_MODAL
    Display = 5,   // OWN_DISPLAY
    Pages = 6,     // OWN_PAGES
    Tools = 7,     // OWN_TOOLS
    Build = 8,     // OWN_BUILD
    Templates = 9, // OWN_TEMPLATES
    Link = 10,     // OWN_LINK
}

// Every page visited while a notebook is open stays in memory with its history: undo
// actions point at the page's live Stroke objects, so a page must never be freed and
// reloaded from disk while its history exists.
struct PageHistory {
    page_id: u32,
    h: History,
    /// None until loaded
    page: Option<Box<Page>>,
    /// it got an erase stroke this session that erased.txt may not list yet (compact.h)
    to_list: bool,
}

/// The page's paper alone, at page size (EditorState.paper): the C's Canvas, whose pixels the brush
/// of an erase stroke points into while it draws. Here the brush shares them (BrushState.paper,
/// PORTING.md, Ownership): drawn into a canvas of their own, then held shared, so a stroke keeps the
/// paper it began with whatever is drawn here after.
#[derive(Clone, Default)]
struct SharedPaper {
    /// None: not made (the C's px NULL)
    px: Option<Rc<Vec<u8>>>,
    w: i32,
    h: i32,
    stride: i32,
}

/// The screen's state (editor.c's, which app.h keeps opaque): editor_init makes it, in a.ed.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct EditorState {
    open: bool,
    // Pen side button gestures (when not drawing): 3 quick clicks, or a ~1.2 s hold while
    // hovering, clears the page (undoable). Holding it while drawing picks the button's tool.
    btn_down: bool,
    btn_used_for_ink: bool,
    btn_hold_fired: bool,
    btn_down_at: u32,
    // A controller's Erase while held is down (app.c ends it at the release): a stroke that starts
    // now erases as the pen's eraser end does.
    hold_eraser: bool,
    clicks: [u32; 3],
    nclicks: i32,
    nb: NotebookInfo,
    index: i32,
    /// The open page (the C's Page *page, which lives in the session cache, hist): the page id of its
    /// cache entry, None for none. open_page() finds it.
    page: Option<u32>,
    page_canvas: Canvas,
    page_dirty: bool,
    info_dirty: bool,
    changed: bool,
    /// page 1 changed: library thumbnail to write when leaving it
    thumb_dirty: bool,
    // Grays settle in one GL16 refresh once the writer stops (not after every stroke): when the pen
    // leaves the page, or after SETTLE_NEAR_MS with it still near. While the pen stays near only
    // fast DU updates go out, nothing blocks, and the next word never meets a settle still on the
    // panel (a stroke that does keeps plain DU instead of the pen mode, display_fbink.c).
    /// page rect still showing dithered live ink
    settle: Rect,
    settle_at: u32,
    /// the pen is near the page (hovering or touching)
    pen_near: bool,
    /// when it last left
    pen_left: u32,
    /// Undo/Redo/Clear availability changed: repaint at the settle
    panel_stale: bool,
    last_up: u32,

    /// the C's hist, nhist
    hist: Vec<PageHistory>,

    // Pointer ownership: which input started the current gesture and what it does.
    owner: Owner,
    owner_is_pen: bool,

    // Drawing
    cur: Option<Box<Stroke>>,
    bs: BrushState,
    sm: Smoother,
    // White ink and the precise eraser (erase strokes) restore the page's paper: this is it alone,
    // at page size, for their strokes drawn live (page_render draws them over the same paper, so
    // live equals re-render). Made at the first erase stroke, drawn again when one starts on a page
    // of another paper or size than it holds, freed with the notebook.
    paper: SharedPaper,
    /// the paper it holds (while paper.px)
    paper_k: PaperKind,
    /// drawn again in place at its next use (the next erase stroke's start; never freed then, a
    /// stroke reads it): the page was drawn whole on an image paper since, or its picture came
    /// after it
    paper_stale: bool,
    // The open notebook's pages listed for compaction this session (compact_note_page); a page
    // with an erase stroke its file does not have yet is listed once the page is saved (to_list).
    /// the C's noted, nnoted, capnoted
    noted: Vec<u32>,

    // Erasing: one gesture = one undo step.
    /// page stroke list at gesture start (to recover original indices): the strokes by identity
    /// (stroke_id), the C's snap and nsnap
    snap: Vec<StrokeId>,
    /// the C's removed, nremoved, capremoved (boxed: a stroke keeps its address, which the history
    /// knows it by)
    #[allow(clippy::vec_box)]
    removed: Vec<Box<Stroke>>,
    ex: f32,
    ey: f32,
    erase_dirty: Rect,

    // Panel
    panel: Rect,
    collapsed: bool,
    /// as last drawn: the pen in use shows that it draws white ink
    white_mark: bool,
    /// as last drawn: the tool button's pen, TB_GRID, or TB_NONE collapsed
    tool_button_shows: i32,
    dragging: bool,
    grab_dx: f32,
    grab_dy: f32,
    drag_start_rect: Rect,
    last_drag_paint: u32,
    hits: HitList,
    press: Press,
    pressed: Hit,
    has_pressed: bool,

    // View (page -> screen): screen = (page - o) * zoom.
    /// page size
    pgw: i32,
    pgh: i32,
    zoom: f32,
    ox: f32,
    oy: f32,
    zoom_default: f32,
    zoom_min: f32,
    /// screen size the view was set up for
    view_w: i32,
    view_h: i32,
    /// which page page_canvas currently shows: its Page's identity (page_ref: the boxed page's
    /// address, compared, never dereferenced, as the C compares its pointer), 0 for none
    canvas_page: usize,
    /// committed once the fingers clearly spread or move
    pinching: bool,
    pinch_committed: bool,
    /// start spread/zoom, page point under midpoint
    pinch_d0: f32,
    pinch_z0: f32,
    pinch_px: f32,
    pinch_py: f32,
    pinch_mx0: f32,
    pinch_my0: f32,
    last_pinch_paint: u32,

    /// Display sheet (front light, invert)
    dm: DispMenu,

    // Tool picker (opened from the toolbar's tool button). In Pin mode a tap pins a pen to the
    // panel, or unpins it.
    tools_open: bool,
    pin_mode: bool,
    /// where its top is meant to be: the panel's, as it opened (it stays there while pins move the
    /// panel; laid out again at a turn)
    tools_y: i32,
    tools_rect: Rect,
    tools_hits: HitList,
    tools_press: Press,

    /// the Pages view (pagesview.h)
    pv: PagesView,
    nav: Nav,
    // Links (nav.h). Add a link (the Index, or the add-link shortcut) waits for the next ink stroke
    // on the open page, its box (link_mode); then the Pages view picks where it goes. A finger's
    // tap on a link follows it; one held on it (LINK_HOLD_MS) opens the Pages view to change where
    // it goes, or remove it.
    link_mode: bool,
    /// the link the Pages view picks for: its box's seed
    link_seed: u32,
    /// a finger on the page (OWN_LINK)
    link_press: Press,
    link_down_at: u32,
    link_held: bool,

    // Paper sheet
    paper_open: bool,
    paper_rect: Rect,
    modal_hits: HitList,
    paper_prev: [Canvas; PAPER_COUNT as usize],
    /// the Custom music tile's preview (one layout at a time)
    custom_prev: Canvas,
    custom_prev_k: PaperKind,
    /// an image paper's preview: one of this notebook's pictures at a time
    image_prev: Canvas,
    image_prev_k: PaperKind,
    image_prev_nb: FixedStr<STORE_ID_LEN>,
    /// the Custom music sheet
    mb: MusicBuild,
    /// the Templates sheet (More papers...), in the Paper sheet's place
    ts: TplSheet,
    // The open page's picture (an image paper's) was not there when the page canvas was drawn (a
    // template's, made in the background): it drew white, and is drawn again once a job has ended
    // since (picture_gen: bgimage_generation then).
    picture_missing: bool,
    picture_gen: u32,
    // A cache drew a picture white the same way (the paper erase strokes restore toward, the More
    // papers tile's preview, a Pages preview): they are all dropped once a job has ended since the
    // first such draw (white_gen), whichever page is open by then.
    white_caches: bool,
    white_gen: u32,
}

// (The C's S, a->ed.)
fn ed(a: &mut App) -> &mut EditorState {
    a.ed.as_deref_mut().unwrap()
}
fn ed_ref(a: &App) -> &EditorState {
    a.ed.as_deref().unwrap()
}

/// A page's identity while it is boxed in the session cache: its address, as the C compares Page
/// pointers (canvas_page). Compared, never dereferenced.
fn page_ref(p: &Page) -> usize {
    p as *const Page as usize
}

impl EditorState {
    /// The cache entry of the open page (S->page), when there is one.
    fn page_ix(&self) -> Option<usize> {
        let pid = self.page?;
        self.hist.iter().position(|e| e.page_id == pid && e.page.is_some())
    }
    fn open_page(&self) -> Option<&Page> {
        let i = self.page_ix()?;
        self.hist[i].page.as_deref()
    }
    fn open_page_mut(&mut self) -> Option<&mut Page> {
        let i = self.page_ix()?;
        self.hist[i].page.as_deref_mut()
    }
    /// page_ref of the open page, 0 for none (the C's S->page as a pointer value).
    fn open_page_ref(&self) -> usize {
        self.open_page().map_or(0, page_ref)
    }
    /// Whether entry i holds the open page (the C's e->page == S->page).
    fn is_open_page(&self, i: usize) -> bool {
        let e = &self.hist[i];
        match self.page {
            None => e.page.is_none(),
            Some(pid) => pid == e.page_id && e.page.is_some(),
        }
    }
}

// The history at entry ih and the page at entry ip at once (one entry, or two).
fn hist_page(hist: &mut [PageHistory], ih: usize, ip: usize) -> (&mut History, &mut Page) {
    if ih == ip {
        let e = &mut hist[ih];
        (&mut e.h, e.page.as_deref_mut().unwrap())
    } else if ih < ip {
        let (l, r) = hist.split_at_mut(ip);
        (&mut l[ih].h, r[0].page.as_deref_mut().unwrap())
    } else {
        let (l, r) = hist.split_at_mut(ih);
        (&mut r[0].h, l[ip].page.as_deref_mut().unwrap())
    }
}

// A modal stored in the editor's state (the Templates sheet, Display, Custom music), taken out for
// a call that also needs the App, then put back (PORTING.md, The App). Its own functions never
// reach back into it.
fn with_modal<T: Default, R>(
    a: &mut App,
    field: fn(&mut EditorState) -> &mut T,
    f: impl FnOnce(&mut T, &mut App) -> R,
) -> R {
    let mut m = std::mem::take(field(ed(a)));
    let r = f(&mut m, a);
    *field(ed(a)) = m;
    r
}

// The same for the Pages view, with the notebook and its Nav that it reads (the C keeps pointers
// to them in it; here they are passed to each call: PORTING.md, Ownership). Nothing the view calls
// reads them from the editor while they are out: of the editor's state, the App's own work reads
// only the page canvas (editor_page_canvas), which stays.
fn with_pv<R>(a: &mut App, f: impl FnOnce(&mut PagesView, &mut App, &NotebookInfo, &Nav) -> R) -> R {
    let s = ed(a);
    let mut pv = std::mem::take(&mut s.pv);
    let nb = std::mem::take(&mut s.nb);
    let nav = std::mem::take(&mut s.nav);
    let r = f(&mut pv, a, &nb, &nav);
    let s = ed(a);
    s.pv = pv;
    s.nb = nb;
    s.nav = nav;
    r
}

// ---- layout ----

// Panel metrics shrink (g_fit, panel.h) when the screen is too short for the full panel: the
// Kindle turned sideways, with pens pinned.
thread_local! {
    static G_FIT: Cell<PanelFit> = const { Cell::new(PanelFit { scale: 1.0, space: 1.0 }) };
}
fn g_fit() -> PanelFit {
    G_FIT.with(|f| f.get())
}
fn sc(v: f32) -> i32 {
    ui_px(v * g_fit().scale)
}
/// expanded width (icons only)
fn pw() -> i32 {
    panel_w()
}
fn btn_h() -> i32 {
    panel_btn_h(g_fit())
}
fn gap() -> i32 {
    panel_gap(g_fit())
}
fn div_h() -> i32 {
    panel_div_h(g_fit())
}
fn head_h() -> i32 {
    panel_head_h(g_fit())
}
/// collapsed square
fn box_() -> i32 {
    panel_box()
}
fn label_h() -> i32 {
    panel_label_h(g_fit())
}
fn npins(st: &Settings) -> i32 {
    clampi(st.npins, 0, PANEL_PINS_MAX as i32)
}

fn fit_panel_to(st: &Settings, screen_h: i32) {
    let f = panel_fit(screen_h, npins(st));
    G_FIT.with(|g| g.set(f));
}

fn panel_rect_at(a: &App, x: i32, y: i32) -> Rect {
    let s = ed_ref(a);
    let w = if s.collapsed { box_() } else { pw() };
    let h = if s.collapsed { box_() } else { panel_height(g_fit(), npins(&a.settings)) };
    // Its margins above and below stay free wherever it goes (fit_panel_to leaves room for them).
    let m = if h + 2 * ui_px(PANEL_MARGIN as f32) <= a.h { ui_px(PANEL_MARGIN as f32) } else { 0 };
    let x = clampi(x, 0, a.w - w);
    let y = clampi(y, m, a.h - h - m);
    Rect { x, y, w, h }
}

// ---- histories ----

impl EditorState {
    // (None where the C's realloc fails.)
    fn entry_for(&mut self, pid: u32) -> Option<usize> {
        if let Some(i) = self.hist.iter().position(|e| e.page_id == pid) {
            return Some(i);
        }
        if self.hist.try_reserve(1).is_err() {
            return None;
        }
        let mut h = History::default();
        history_init(&mut h);
        self.hist.push(PageHistory { page_id: pid, h, page: None, to_list: false });
        Some(self.hist.len() - 1)
    }

    fn cur_hist(&mut self) -> Option<usize> {
        let pid = self.nb.page_ids[self.index as usize];
        self.entry_for(pid)
    }

    // Page `index` from disk, at the notebook's page size (S->pgw x S->pgh, use_page_size): every
    // page the editor reads comes through here, so none can come at the screen's size instead.
    fn load_page(&self, index: i32, out: &mut Option<Box<Page>>) -> i32 {
        store_load_page(&self.nb, index, self.pgw, self.pgh, out)
    }

    // The page at index, from the session cache or (first visit) from disk: its page id (the C's
    // Page pointer into the cache), None when it could not be read.
    fn page_at(&mut self, index: i32) -> Option<u32> {
        let pid = self.nb.page_ids[index as usize];
        let e = self.entry_for(pid)?;
        if self.hist[e].page.is_none() {
            let mut p = None;
            if self.load_page(index, &mut p) != 0 {
                return None;
            }
            self.hist[e].page = p;
        }
        Some(pid)
    }

    fn free_histories(&mut self) {
        // (each entry's history and page go with it)
        self.hist.clear();
        self.hist.shrink_to_fit();
        self.page = None; // it lived in the cache
    }
}

// ---- drawing the panel ----

fn tool_style(st: &Settings, t: ToolKind) -> ButtonStyle {
    if st.current_tool == t {
        ButtonStyle::Selected
    } else {
        ButtonStyle::Plain
    }
}

// The drawing tool the tool button shows, and where a tap on the eraser in use goes back to
// (panel.h, beside the steps of next and previous tool that use them).
fn button_tool(st: &Settings) -> ToolKind {
    panel_button_pen(st)
}
fn eraser_back(st: &Settings) -> ToolKind {
    panel_eraser_back(st)
}

// What the tool button showed as the panel was last drawn (editor.tool_button): its pen, or these.
const TB_GRID: i32 = -1;
const TB_NONE: i32 = -2;

// While the drawing tools draw white ink, the pen in use shows its icon hollow: white on white
// paper shows nothing, and without a mark the pen would seem broken.
fn draws_white(st: &Settings, t: ToolKind) -> bool {
    st.white && t == st.current_tool && (t.0 as u32) < TOOL_COUNT as u32 && t != TOOL_ERASER
}

// A pen's button on the panel (the tool button's, or a pinned one's), selected while it is the pen
// in use, hollow while that draws white.
fn draw_pen_button(c: &mut Canvas, st: &Settings, s: &mut EditorState, r: Rect, t: ToolKind) {
    if draws_white(st, t) {
        ui_button_hollow(c, r, ui_tool_icon(t.0), ButtonStyle::Selected);
        s.white_mark = true;
    } else {
        ui_button(c, r, ui_tool_icon(t.0), "", tool_style(st, t));
    }
}

fn draw_panel(a: &mut App) {
    let App { screen: c, settings: st, ed, .. } = a;
    let s = ed.as_deref_mut().unwrap();
    let p = s.panel;
    hits_clear(&mut s.hits);
    s.white_mark = false;
    s.tool_button_shows = TB_NONE;
    if s.collapsed {
        ui_frame(c, p, ui_px(22.0), 255, true, 0);
        ui_frame(c, p, ui_px(22.0), 0, false, ui_px(6.0));
        let mut ir = rect_inflate(p, -ui_px(20.0));
        ir.h -= ui_px(12.0);
        if draws_white(st, st.current_tool) {
            ui_icon_hollow(c, ui_tool_icon(st.current_tool.0), ir, 0, 255);
            s.white_mark = true;
        } else {
            ui_icon(c, ui_tool_icon(st.current_tool.0), ir, 0);
        }
        // Expand hint in the corner.
        ui_icon(
            c,
            IconKind::Expand,
            Rect { x: p.x + p.w - ui_px(40.0), y: p.y + p.h - ui_px(40.0), w: ui_px(30.0), h: ui_px(30.0) },
            0,
        );
        hits_add(&mut s.hits, p, P_EXPAND, 0);
        return;
    }
    ui_frame(c, p, ui_px(22.0), 255, true, 0);
    ui_frame(c, p, ui_px(22.0), 0, false, ui_px(6.0));
    let x = p.x + ui_px(14.0);
    let w = p.w - ui_px(28.0);
    // Header: grip (drag) + collapse.
    let grip = Rect { x: p.x, y: p.y, w: p.w - ui_px(62.0), h: head_h() };
    let col = Rect { x: p.x + p.w - ui_px(64.0), y: p.y + sc(12.0), w: ui_px(54.0), h: sc(56.0) };
    ui_icon(c, IconKind::Grip, Rect { x: p.x + ui_px(10.0), y: p.y + sc(14.0), w: ui_px(52.0), h: sc(52.0) }, 0);
    ui_icon(c, IconKind::Collapse, col, 0);
    hits_add(&mut s.hits, grip, P_GRIP, 0);
    hits_add(&mut s.hits, rect_inflate(col, ui_px(10.0)), P_COLLAPSE, 0);
    let mut y = p.y + head_h();
    canvas_hline(c, p.x + ui_px(6.0), p.x + p.w - ui_px(6.0), y - ui_px(4.0), ui_px(3.0), 0);
    y += gap();
    // The drawing tool (tap: the picker), the pens pinned under it, and the eraser. When the
    // drawing tool is one of the pins, its pin shows it: the tool button shows a grid instead.
    let dt = button_tool(st);
    let dt_pinned = settings_pinned(st, dt);
    let dt_sel = !dt_pinned && st.current_tool == dt;
    let rt = Rect { x, y, w, h: btn_h() };
    if dt_pinned {
        ui_button(c, rt, IconKind::Grid, "", ButtonStyle::Plain);
        s.tool_button_shows = TB_GRID;
    } else {
        draw_pen_button(c, st, s, rt, dt);
        s.tool_button_shows = dt.0;
    }
    // A small corner mark says "more tools here" (the grid says it by itself).
    if !dt_pinned {
        canvas_disc_aa(
            c,
            (rt.x + rt.w - ui_px(16.0)) as f32,
            (rt.y + rt.h - ui_px(16.0)) as f32,
            ui_px(7.0) as f32,
            if dt_sel { 255 } else { 0 },
            1.0,
        );
    }
    hits_add(&mut s.hits, rt, P_TOOLPICK, 0);
    y += btn_h() + gap();
    for i in 0..npins(st) {
        let rp = Rect { x, y, w, h: btn_h() };
        let t = st.pins[i as usize];
        draw_pen_button(c, st, s, rp, t);
        hits_add(&mut s.hits, rp, P_TOOL, t.0);
        y += btn_h() + gap();
    }
    let re = Rect { x, y, w, h: btn_h() };
    ui_button(c, re, IconKind::Eraser, "", tool_style(st, TOOL_ERASER));
    hits_add(&mut s.hits, re, P_TOOL, TOOL_ERASER.0);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6.0), ui_px(3.0), 160);
    y += div_h();
    let h = if s.open { s.cur_hist() } else { None };
    let cu = h.is_some_and(|i| history_can_undo(&s.hist[i].h));
    let cr = h.is_some_and(|i| history_can_redo(&s.hist[i].h));
    let ru = Rect { x, y, w, h: btn_h() };
    ui_button(c, ru, IconKind::Undo, "", if cu { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if cu {
        hits_add(&mut s.hits, ru, P_UNDO, 0);
    }
    y += btn_h() + gap();
    let rr = Rect { x, y, w, h: btn_h() };
    ui_button(c, rr, IconKind::Redo, "", if cr { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if cr {
        hits_add(&mut s.hits, rr, P_REDO, 0);
    }
    y += btn_h() + gap();
    let rc = Rect { x, y, w, h: btn_h() }; // clear the page right away (no confirmation)
    let can_clear = s.open_page().is_some_and(|p| !p.strokes.is_empty());
    ui_button(c, rc, IconKind::Broom, "", if can_clear { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if can_clear {
        hits_add(&mut s.hits, rc, P_CLEAR, 0);
    }
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6.0), ui_px(3.0), 160);
    y += div_h();
    let rp = Rect { x, y, w, h: btn_h() };
    ui_button(c, rp, IconKind::Prev, "", if s.index > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if s.index > 0 {
        hits_add(&mut s.hits, rp, P_PREV, 0);
    }
    y += btn_h() + gap();
    let mut lab = String::new();
    csnprintf!(&mut lab, 32, "%d / %d", s.index + 1, s.nb.npages());
    let lr = Rect { x: x - ui_px(6.0), y, w: w + ui_px(12.0), h: label_h() };
    ui_frame(c, lr, ui_px(10.0), 0, false, ui_px(2.0)); // a tappable chip: opens Pages
    let fit = g_fit();
    font_draw_centered(c, FontFace::Bold, ui_small_size() * (if fit.scale < 1.0 { fit.scale } else { 1.0 }), lr, &lab, 0);
    hits_add(&mut s.hits, lr, P_PAGES, 0);
    y += label_h() + gap();
    let last = s.index >= s.nb.npages() - 1;
    let rn = Rect { x, y, w, h: btn_h() };
    ui_button(c, rn, if last { IconKind::Plus } else { IconKind::Next }, "", ButtonStyle::Plain);
    hits_add(&mut s.hits, rn, P_NEXT, 0);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6.0), ui_px(3.0), 160);
    y += div_h();
    // Paper, Display, Pen feel, Library (icons only, like the rest of the panel).
    let rest: [(IconKind, i32); 4] = [
        (IconKind::Paper, P_PAPER),
        (IconKind::Brightness, P_DISPLAY),
        (IconKind::Tune, P_PENSET),
        (IconKind::Library, P_LIBRARY),
    ];
    for (icon, id) in rest {
        let r = Rect { x, y, w, h: btn_h() };
        ui_button(c, r, icon, "", ButtonStyle::Plain);
        hits_add(&mut s.hits, r, id, 0);
        y += btn_h() + gap();
    }
}

// A cache was just drawn with a picture white in it (bgimage_take_missing; at generation gen):
// it is dropped once a background job has ended since (pictures_arrived).
fn note_white(s: &mut EditorState, gen: u32) {
    if !s.white_caches || (gen.wrapping_sub(s.white_gen) as i32) < 0 {
        s.white_gen = gen;
    }
    s.white_caches = true;
}

// Which of the previews a paper's is (paper_preview).
#[derive(Clone, Copy)]
enum Prev {
    Custom,
    Image,
    Paper(usize),
}
fn prev_canvas(s: &mut EditorState, which: Prev) -> &mut Canvas {
    match which {
        Prev::Custom => &mut s.custom_prev,
        Prev::Image => &mut s.image_prev,
        Prev::Paper(i) => &mut s.paper_prev[i],
    }
}

fn paper_preview(s: &mut EditorState, dpi: f32, k: PaperKind) -> Option<&Canvas> {
    let mut k = k;
    if !paper_is_known(k) {
        k = PAPER_BLANK; // a newer version's paper: blank, as it renders here
    }
    let which;
    if paper_is_custom(k) {
        which = Prev::Custom;
        if !s.custom_prev.px.is_empty() && s.custom_prev_k != k {
            canvas_free(&mut s.custom_prev);
        }
        s.custom_prev_k = k;
    } else if paper_is_image(k) {
        // A picture of this notebook's (the same value in another notebook can be another one).
        which = Prev::Image;
        let c = &s.image_prev;
        if !c.px.is_empty()
            && (s.image_prev_k != k || s.image_prev_nb.as_bytes() != s.nb.id.as_bytes() || c.w != s.pgw / 6 || c.h != s.pgh / 6)
        {
            canvas_free(&mut s.image_prev);
        }
        s.image_prev_k = k;
        s.image_prev_nb = s.nb.id;
    } else {
        which = Prev::Paper(clampi(k.0 as i32, 0, PAPER_COUNT - 1) as usize);
    }
    if prev_canvas(s, which).px.is_empty() {
        let mut full = Canvas::default();
        if canvas_init(&mut full, s.pgw, s.pgh) != 0 {
            return None;
        }
        if paper_is_image(k) {
            bgimage_take_missing(); // (what drew white before)
            bgimage_render(&mut full, s.nb.id.as_str(), k, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh });
            if bgimage_take_missing() {
                note_white(s, bgimage_generation());
            }
        } else {
            paper_render(&mut full, k, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh }, dpi);
        }
        let (w, h) = (s.pgw / 6, s.pgh / 6);
        let c = prev_canvas(s, which);
        if canvas_init(c, w, h) == 0 {
            canvas_downsample_keep_lines(c, &full, 0.6);
        }
    }
    let c = prev_canvas(s, which);
    if !c.px.is_empty() {
        Some(c)
    } else {
        None
    }
}

fn draw_paper_sheet(a: &mut App) {
    let mut g = PaperGrid::default();
    let paper_rect = paper_sheet_rect(a, &mut g);
    let dpi = a.dpi;
    let App { screen: c, settings: st, ed, .. } = a;
    let s = ed.as_deref_mut().unwrap();
    hits_clear(&mut s.modal_hits);
    s.paper_rect = paper_rect;
    let gap_ = ui_px(30.0);
    let gridw = g.cols * g.tw + (g.cols - 1) * gap_;
    // TRANSLATORS: the title of the editor's paper sheet, which changes the open page's paper.
    let in_ = ui_sheet(c, s.paper_rect, tr("Paper for this page"));
    let page_k = s.open_page().map(|p| p.paper);
    let page_custom = page_k.is_some_and(|k| paper_is_known(k) && paper_is_custom(k));
    let page_picture = page_k.is_some_and(paper_is_image);
    for i in 0..PAPER_SHEET_EDITOR_TILES {
        // The last three tiles. Custom music shows this page's layout, or the last one built (also
        // for a newer layout this build cannot draw, which is then not selected either). More
        // papers opens the Templates sheet: it shows this page's picture when the page is on one.
        // Photos... opens it on the photos (the sign for more: a book page is a page's own picture
        // too, so a photo's is never told from it here).
        let custom = i == PAPER_COUNT;
        let more = i == PAPER_COUNT + 1;
        let photos = i == PAPER_COUNT + 2;
        if photos {
            let tr_ = Rect {
                x: in_.x + (in_.w - gridw) / 2 + (i % g.cols) * (g.tw + gap_),
                y: in_.y + (i / g.cols) * g.cell_h,
                w: g.tw,
                h: g.th,
            };
            canvas_fill(c, tr_, 255);
            paper_tile_more(c, tr_);
            ui_frame(c, tr_, ui_px(12.0), 0, false, ui_px(3.0));
            // TRANSLATORS: the editor's Paper sheet's tile that opens the Photos sheet (photos copied into
            // grimoire/photos), under its tile: as Custom music... and More papers... (their room).
            paper_tile_label(c, tr_, gap_, tr("Photos..."), false);
            hits_add(&mut s.modal_hits, Rect { x: tr_.x, y: tr_.y, w: g.tw, h: g.th + ui_px(90.0) }, P_PICK_PHOTOS, 0);
            continue;
        }
        let k = if more {
            if page_picture {
                page_k.unwrap()
            } else {
                PAPER_BLANK
            }
        } else if custom {
            if page_custom {
                page_k.unwrap()
            } else {
                st.music_layout
            }
        } else {
            paper_at(i)
        };
        let tr_ = Rect {
            x: in_.x + (in_.w - gridw) / 2 + (i % g.cols) * (g.tw + gap_),
            y: in_.y + (i / g.cols) * g.cell_h,
            w: g.tw,
            h: g.th,
        };
        canvas_fill(c, tr_, 255);
        let pv = if more && !page_picture { None } else { paper_preview(s, dpi, k) };
        if let Some(pv) = pv {
            canvas_blit_fit(c, pv, tr_);
        } else if more {
            paper_tile_more(c, tr_);
        }
        let sel = if more {
            page_picture
        } else if custom {
            page_custom
        } else {
            page_k == Some(k)
        };
        ui_frame(
            c,
            rect_inflate(tr_, if sel { ui_px(10.0) } else { 0 }),
            ui_px(12.0),
            0,
            false,
            if sel { ui_px(9.0) } else { ui_px(3.0) },
        );
        // (Custom music... and More papers... as in New notebook, which says their room.)
        let label = if more {
            tr("More papers...")
        } else if custom {
            tr("Custom music...")
        } else {
            paper_label(k)
        };
        paper_tile_label(c, tr_, gap_, label, sel);
        hits_add(
            &mut s.modal_hits,
            Rect { x: tr_.x, y: tr_.y, w: g.tw, h: g.th + ui_px(90.0) },
            if more {
                P_PICK_MORE
            } else if custom {
                P_PICK_CUSTOM
            } else {
                P_PICK_PAPER
            },
            k.0 as i32,
        );
    }
    let cancel = Rect { x: in_.x + in_.w - ui_px(380.0), y: in_.y + in_.h - ui_px(140.0), w: ui_px(380.0), h: ui_px(140.0) };
    ui_button(c, cancel, IconKind::None, tr("Cancel"), ButtonStyle::Plain);
    hits_add(&mut s.modal_hits, cancel, P_PICK_CANCEL, 0);
}

// ---- view ----

fn view_identity(a: &App) -> bool {
    let s = ed_ref(a);
    s.zoom == 1.0 && s.ox == 0.0 && s.oy == 0.0 && s.pgw == a.w && s.pgh == a.h
}

fn view_clamp(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let s = ed(a);
    s.zoom = clampf(s.zoom, s.zoom_min, s.zoom_default * 4.0);
    let mut vw = aw as f32 / s.zoom; // visible page area
    let mut vh = ah as f32 / s.zoom;
    // A page fitted to a screen of another size (a notebook from another Scribe) spans it but for
    // float rounding: exactly, so the fitted view sits at no offset (the fitted view).
    if (vw - s.pgw as f32).abs() < 0.01 {
        vw = s.pgw as f32;
    }
    if (vh - s.pgh as f32).abs() < 0.01 {
        vh = s.pgh as f32;
    }
    s.ox = if vw >= s.pgw as f32 { -(vw - s.pgw as f32) / 2.0 } else { clampf(s.ox, 0.0, s.pgw as f32 - vw) };
    s.oy = if vh >= s.pgh as f32 { -(vh - s.pgh as f32) / 2.0 } else { clampf(s.oy, 0.0, s.pgh as f32 - vh) };
    // Snap to exactly 100% / no offset in portrait so the fast identity path is used.
    if (s.zoom - 1.0).abs() < 0.01 && s.pgw == aw && s.pgh == ah {
        s.zoom = 1.0;
        s.ox = 0.0;
        s.oy = 0.0;
    }
}

// The page fills the screen's width: 100% upright for a page of this screen's size, the whole page
// for a notebook from another Scribe (theirs are 3:4 too), and sideways the width (pan with two
// fingers; pinching out shows the whole page).
fn view_reset(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let s = ed(a);
    s.zoom_min = (aw as f32 / s.pgw as f32).min(ah as f32 / s.pgh as f32);
    s.zoom_default = aw as f32 / s.pgw as f32;
    s.zoom = s.zoom_default;
    s.ox = 0.0;
    s.oy = 0.0;
    s.view_w = aw;
    s.view_h = ah;
    view_clamp(a);
}

fn to_page(s: &EditorState, sx: f32, sy: f32, px: &mut f32, py: &mut f32) {
    *px = sx / s.zoom + s.ox;
    *py = sy / s.zoom + s.oy;
}

fn page_to_screen(a: &App, p: Rect) -> Rect {
    if rect_is_empty(p) {
        return RECT_EMPTY;
    }
    let s = ed_ref(a);
    let x0 = (p.x as f32 - s.ox) * s.zoom;
    let y0 = (p.y as f32 - s.oy) * s.zoom;
    let x1 = ((p.x + p.w) as f32 - s.ox) * s.zoom;
    let y1 = ((p.y + p.h) as f32 - s.oy) * s.zoom;
    let r = Rect {
        x: x0.floor() as i32 - 2,
        y: y0.floor() as i32 - 2,
        w: (x1 - x0).ceil() as i32 + 5,
        h: (y1 - y0).ceil() as i32 + 5,
    };
    rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h })
}

fn zoomed(a: &App) -> bool {
    let s = ed_ref(a);
    (s.zoom - s.zoom_default).abs() > 0.01 || s.ox != 0.0 || s.oy > 0.5
}

// Draws the visible page into a screen rect: bilinear sampling, a gray desk outside the page. The
// whole screen of it (a pinch, a zoomed page) is about 100 ms on the Kindle: the pens are read every
// 64 rows (work_pump, common.h).
fn blit_view(a: &mut App, r: Rect) {
    let App { screen, ed, .. } = a;
    let s = ed.as_deref().unwrap();
    let pc = &s.page_canvas;
    // (the C's static ix[8192], wx[8192])
    let mut ix = [0i32; 8192];
    let mut wx = [0u8; 8192];
    let mut r = r;
    if r.w > 8192 {
        r.w = 8192;
    }
    for i in 0..r.w {
        let fx = ((r.x + i) as f32 + 0.5) / s.zoom + s.ox - 0.5;
        let x = fx.floor() as i32;
        ix[i as usize] = x;
        wx[i as usize] = ((fx - x as f32) * 255.0) as u8;
    }
    for y in r.y..r.y + r.h {
        let dst = y as usize * screen.stride as usize;
        let fy = (y as f32 + 0.5) / s.zoom + s.oy - 0.5;
        let py = fy.floor() as i32;
        let wy = ((fy - py as f32) * 255.0) as u32;
        let py0 = clampi(py, 0, pc.h - 1);
        let py1 = clampi(py + 1, 0, pc.h - 1);
        let r0 = py0 as usize * pc.stride as usize;
        let r1 = py1 as usize * pc.stride as usize;
        let row_out = py < -1 || py >= pc.h;
        for i in 0..r.w {
            let x = ix[i as usize];
            if row_out || x < -1 || x >= pc.w {
                screen.px[dst + (r.x + i) as usize] = 205;
                continue;
            }
            let x0 = clampi(x, 0, pc.w - 1) as usize;
            let x1 = clampi(x + 1, 0, pc.w - 1) as usize;
            let w = wx[i as usize] as u32;
            let top = pc.px[r0 + x0] as u32 * (255 - w) + pc.px[r0 + x1] as u32 * w;
            let bot = pc.px[r1 + x0] as u32 * (255 - w) + pc.px[r1 + x1] as u32 * w;
            screen.px[dst + (r.x + i) as usize] = ((top * (255 - wy) + bot * wy + 32512) / 65025) as u8;
        }
        if ((y - r.y) & 63) == 63 {
            work_pump();
        }
    }
}

// ---- tool picker ----

// Its pens: tool_order's first TOOL_PENS (brush.h), three to a row, which the picker's next and
// previous go through in the same order, 1 to 9 (panel.h, picker_step).
const N_DRAW_TOOLS: usize = TOOL_PENS;

// The ink row under the pens: one choice each, the pens drawing black ink or white (erase strokes:
// docs/FORMAT.md). A later colour adds its choices to this control.
struct Ink {
    /// looked up where drawn, C_("ink", ...)
    label: &'static str,
    white: bool,
}
const K_INKS: [Ink; 2] = [
    // TRANSLATORS: the tool picker's ink row under the pens, two choices side by side; this one:
    // the pens draw black ink. 426 px at 44 px, or two lines at 34 px.
    Ink { label: nc_("ink", "Black"), white: false },
    // TRANSLATORS: the other choice of the tool picker's ink row: the pens draw white ink, which
    // takes away the ink under it and keeps the paper's lines (426 px at 44 px).
    Ink { label: nc_("ink", "White"), white: true },
];
const N_INKS: i32 = K_INKS.len() as i32;

fn draw_tools(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let App { screen: c, settings: st, ed, .. } = a;
    let s = ed.as_deref_mut().unwrap();
    hits_clear(&mut s.tools_hits);
    let bw = ui_px(300.0);
    let bh = ui_px(150.0);
    let g = ui_px(16.0);
    let cols = 3;
    let rows = (N_DRAW_TOOLS as i32 + cols - 1) / cols;
    let gridw = cols * bw + (cols - 1) * g;
    let inkh = ui_px(120.0);
    let w = gridw + 2 * ui_px(30.0);
    let h = ui_px(110.0) + rows * bh + (rows - 1) * g + g + inkh + ui_px(40.0);
    let mut x = s.panel.x - w - ui_px(20.0);
    if x < ui_px(10.0) {
        x = s.panel.x + s.panel.w + ui_px(20.0);
    }
    // Neither side has room for it beside a panel dragged to the middle: it goes over the panel
    // then, whole on the screen (while it is open it takes every tap).
    x = clampi(x, ui_px(10.0), aw - w - ui_px(10.0));
    let y = clampi(s.tools_y, ui_px(10.0), ah - h - ui_px(10.0));
    s.tools_rect = Rect { x, y, w, h };
    ui_frame(c, s.tools_rect, ui_px(22.0), 255, true, 0);
    ui_frame(c, s.tools_rect, ui_px(22.0), 0, false, ui_px(6.0));
    // The title row: at its right Pin, which turns the grid into the choice of the pens pinned to
    // the panel, and there Done, which turns it back. Its width holds either word.
    // TRANSLATORS: the tool picker's button at the top right that starts choosing the pens pinned
    // to the panel, beside a pin (116 px at 34 px; the button widens for a longer word).
    let pin = c_("panel", "Pin");
    // TRANSLATORS: the same button while the pens are being chosen, beside a check mark: it ends
    // that (116 px at 34 px, as Pin).
    let done = tr("Done");
    let hs = ui_small_size();
    let hh = ui_px(78.0);
    let hi = hh - ui_px(24.0);
    let lx = ui_px(14.0) + hi + ui_px(10.0);
    let pad = ui_px(16.0);
    let lw = font_measure(FontFace::Caps, hs, pin);
    let dw = font_measure(FontFace::Caps, hs, done);
    let hw = clampi(lx + (if lw > dw { lw } else { dw }) + pad, ui_px(210.0), w / 2);
    let hb = Rect { x: x + w - ui_px(30.0) - hw, y: y + ui_px(14.0), w: hw, h: hh };
    ui_button(c, hb, IconKind::None, "", if s.pin_mode { ButtonStyle::Primary } else { ButtonStyle::Plain });
    ui_icon(
        c,
        if s.pin_mode { IconKind::Check } else { IconKind::Pin },
        Rect { x: hb.x + ui_px(14.0), y: hb.y + (hh - hi) / 2, w: hi, h: hi },
        0,
    );
    ui_text(
        c,
        FontFace::Caps,
        hs,
        hs,
        Rect { x: hb.x + lx, y: hb.y, w: hb.w - lx - pad, h: hh },
        if s.pin_mode { done } else { pin },
        1,
        true,
        0,
    );
    hits_add(&mut s.tools_hits, hb, P_PIN_MODE, 0);
    let mut title = String::new();
    if s.pin_mode {
        // TRANSLATORS: the tool picker's title while it chooses the pens pinned to the panel under the
        // tool button: a tap pins a pen or unpins it; %d is how many can be (4). About 700 px at 44 px
        // in bold, or two lines at 34 px. Normally the title is Tools.
        csnprintf!(&mut title, 160, tr("Pin to the panel (up to %d)"), PANEL_PINS_MAX as i32);
    } else {
        // TRANSLATORS: the title of the editor's tool picker.
        csnprintf!(&mut title, 160, "%s", tr("Tools"));
    }
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size(),
        ui_small_size(),
        Rect { x: x + ui_px(30.0), y, w: hb.x - x - ui_px(50.0), h: ui_px(100.0) },
        &title,
        2,
        false,
        0,
    );
    for i in 0..N_DRAW_TOOLS as i32 {
        let t = TOOL_ORDER[i as usize];
        let r = Rect {
            x: x + ui_px(30.0) + (i % cols) * (bw + g),
            y: y + ui_px(100.0) + (i / cols) * (bh + g),
            w: bw,
            h: bh,
        };
        // TRANSLATORS: the tool picker's shorter names for the Mechanical pencil and the
        // Calligraphy pen (268 px at 34 px in Gothic, 284 in Classic, as the other tools' names).
        let name = if t == TOOL_MECH_PENCIL { c_("tool", "Mechanical") }
                   else if t == TOOL_CALLIGRAPHY { c_("tool", "Calligraphy") }
                   else { tool_label(t) };
        // Selected: the pen in use, or in Pin mode a pen pinned.
        let sel = if s.pin_mode { settings_pinned(st, t) } else { st.current_tool == t };
        ui_button(c, r, ui_tool_icon(t.0), name, if sel { ButtonStyle::Selected } else { ButtonStyle::Plain });
        hits_add(&mut s.tools_hits, r, P_PICK_TOOL, t.0);
    }
    // The ink row, out of use in Pin mode (the pins are pens, whatever they draw).
    let iy = y + ui_px(100.0) + rows * bh + (rows - 1) * g + g;
    let iw = (gridw - (N_INKS - 1) * g) / N_INKS;
    for i in 0..N_INKS {
        let r = Rect { x: x + ui_px(30.0) + i * (iw + g), y: iy, w: iw, h: inkh };
        let ink = &K_INKS[i as usize];
        let stl = if s.pin_mode {
            ButtonStyle::Disabled
        } else if st.white == ink.white {
            ButtonStyle::Selected
        } else {
            ButtonStyle::Plain
        };
        ui_button(c, r, IconKind::None, c_("ink", ink.label), stl);
        if !s.pin_mode {
            hits_add(&mut s.tools_hits, r, P_PICK_INK, i);
        }
    }
}

// Rebuild a screen rect from the layers.
fn compose(a: &mut App, r: Rect) {
    if ed(a).pv.open {
        // the Pages view covers everything
        with_pv(a, pagesview_draw);
        return;
    }
    a.toast_floor = 0;
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h });
    if rect_is_empty(r) {
        return;
    }
    if view_identity(a) {
        let App { screen, ed, .. } = a;
        canvas_blit(screen, &ed.as_deref().unwrap().page_canvas, r, r.x, r.y);
    } else {
        blit_view(a, r);
    }
    if rect_overlaps(r, ed(a).panel) {
        draw_panel(a);
    }
    let s = ed(a);
    if s.paper_open && rect_overlaps(r, s.paper_rect) {
        draw_paper_sheet(a);
    }
    let s = ed(a);
    if s.ts.open && rect_overlaps(r, s.ts.rect) {
        with_modal(a, |s| &mut s.ts, tplsheet_draw);
    }
    let s = ed(a);
    if s.dm.open && rect_overlaps(r, s.dm.rect) {
        with_modal(a, |s| &mut s.dm, dispmenu_draw);
    }
    let s = ed(a);
    if s.mb.open && rect_overlaps(r, s.mb.rect) {
        with_modal(a, |s| &mut s.mb, musicbuild_draw);
    }
    let s = ed(a);
    if s.tools_open && rect_overlaps(r, s.tools_rect) {
        draw_tools(a);
    }
}

// A page-space change: recompose what it covers on screen and refresh it.
fn show(a: &mut App, page_rect: Rect, m: RefreshMode) {
    let r = page_to_screen(a, page_rect);
    if rect_is_empty(r) {
        return;
    }
    compose(a, r);
    app_invalidate(a, r, m);
}

fn compose_all(a: &mut App, m: RefreshMode) {
    compose(a, Rect { x: 0, y: 0, w: a.w, h: a.h });
    app_invalidate_all(a, m);
}

fn repaint_panel(a: &mut App, m: RefreshMode) {
    draw_panel(a);
    let p = ed(a).panel;
    app_invalidate(a, p, m);
}

// ---- pages ----

fn save_thumb_from_canvas(s: &EditorState) {
    let mut t = Canvas::default();
    if canvas_init(&mut t, s.pgw / 3, s.pgh / 3) != 0 {
        return;
    }
    canvas_downsample_keep_lines(&mut t, &s.page_canvas, 0.35);
    store_save_thumb(s.nb.id.as_str(), &t);
}

// Page `pid` was written with an erase stroke it did not have: its file keeps the ink under it
// until compaction takes that out when the notebook closes (compact.h). Listed once per page and
// session; false if the list could not be written.
fn note_for_compaction(s: &mut EditorState, pid: u32) -> bool {
    if s.noted.contains(&pid) {
        return true;
    }
    if compact_note_page(s.nb.id.as_str(), pid) != 0 {
        return false;
    }
    if s.noted.len() == s.noted.capacity() {
        let nc = if s.noted.capacity() > 0 { s.noted.capacity() * 2 } else { 16 };
        if s.noted.try_reserve_exact(nc - s.noted.len()).is_err() {
            return true; // (listed: only noted again next time)
        }
    }
    s.noted.push(pid);
    true
}

// The pages that got an erase stroke this session, listed for compaction once their files have it:
// at a save, not as the pen lifts, so the list's write never holds up the next stroke. A page the
// list could not take (it could not be written, or did not read) is tried again at the next save.
fn list_erased_pages(s: &mut EditorState) {
    for i in 0..s.hist.len() {
        let pid = s.hist[i].page_id;
        if s.hist[i].to_list && !(s.is_open_page(i) && s.page_dirty) && note_for_compaction(s, pid) {
            s.hist[i].to_list = false;
        }
    }
}

// False if the page could not be written; it then stays dirty (and in memory), so callers
// that would drop it (turning the page, closing) must stay put instead.
fn save_page(a: &mut App) -> bool {
    let s = ed(a);
    if !s.open {
        return true;
    }
    if s.page_dirty {
        // A read-only page (from a newer Grimoire) is never written: its file stays as it is.
        let failed = match s.open_page() {
            Some(p) => !p.readonly && store_save_page(&s.nb, s.index, p) != 0,
            None => false,
        };
        if failed {
            app_toast(a, tr("Could not save this page. Is the storage full?"), 5000);
            return false;
        }
        if s.index == 0 {
            s.thumb_dirty = true; // written when leaving the page (it is slow)
        }
        s.page_dirty = false;
        s.info_dirty = true;
    }
    list_erased_pages(s); // their files have the erase strokes now (and the ink under them)
    if s.info_dirty {
        if store_save_info(&mut s.nb) != 0 {
            return false;
        }
        s.info_dirty = false;
    }
    true
}

// Draws r of the open page into the page canvas. An image paper's picture that is not there (a
// template's, made in the background) draws white: noted, to be drawn once it is (editor_tick).
fn draw_page(a: &mut App, r: Rect) {
    let dpi = a.dpi;
    let s = ed(a);
    bgimage_take_missing(); // (what drew white before)
    let ip = s.page_ix().unwrap();
    page_render(&mut s.page_canvas, s.hist[ip].page.as_deref().unwrap(), r, dpi);
    let whole = r.x <= 0 && r.y <= 0 && r.x + r.w >= s.pgw && r.y + r.h >= s.pgh;
    if bgimage_take_missing() {
        s.picture_missing = true;
        s.picture_gen = bgimage_generation();
    } else if whole {
        s.picture_missing = false;
    }
}

fn render_page_full(a: &mut App) {
    let s = ed(a);
    let r = Rect { x: 0, y: 0, w: s.pgw, h: s.pgh };
    draw_page(a, r);
    let s = ed(a);
    s.canvas_page = s.open_page_ref();
    // An image paper's picture can have come since its paper alone was drawn (S->paper: a picture
    // made in the background draws white until it is there), and the page now shows it.
    if paper_is_image(s.open_page().unwrap().paper) {
        s.paper_stale = true;
    }
}

// A page from a newer Grimoire (or one this build cannot fully read) is shown as far as it can
// be and never changed: writing, erasing, clearing and paper changes are refused with this.
// TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines).
const K_READONLY_MSG: &str = n_("This page was made by a newer Grimoire. It is read-only here.");

fn refuse_readonly(a: &mut App) -> bool {
    if !ed(a).open_page().is_some_and(|p| p.readonly) {
        return false;
    }
    app_toast(a, tr(K_READONLY_MSG), 3000);
    true
}

// On arriving at a page: a read-only page says so, and any other page takes that message down if
// it is still up from the page before, so it never seems to be about this one.
fn arrive_readonly(a: &mut App) {
    if refuse_readonly(a) || !a.toast_visible || a.toast.as_bytes() != tr(K_READONLY_MSG).as_bytes() {
        return;
    }
    a.toast_visible = false;
    let r = a.toast_rect;
    app_invalidate(a, r, RefreshMode::Ui); // (already inside the arriving page's full repaint)
}

fn go_to_page(a: &mut App, index: i32) -> bool {
    let s = ed(a);
    s.link_mode = false; // (a link's box is drawn on the page Add a link was asked on)
    s.settle = RECT_EMPTY; // the whole screen is about to be redrawn anyway
    s.panel_stale = false;
    if !save_page(a) {
        compose_all(a, RefreshMode::Full);
        return false;
    }
    write_thumb_if_needed(a);
    let s = ed(a);
    let index = clampi(index, 0, s.nb.npages() - 1);
    let Some(np) = s.page_at(index) else {
        compose_all(a, RefreshMode::Full);
        app_toast(a, tr("Could not open that page"), 3000);
        return false;
    };
    live_reset();
    a.live_processed = 0;
    a.live_processed_ms = 0;
    let s = ed(a);
    s.page = Some(np);
    s.index = index;
    s.nb.last_page = index;
    s.info_dirty = true;
    render_page_full(a);
    view_reset(a);
    compose_all(a, RefreshMode::Full);
    arrive_readonly(a);
    true
}

// The open notebook's page size (it keeps the one it was made with, whatever the screen). The page
// canvas and the paper previews are made at it: when it changes they go, to be made again.
fn use_page_size(s: &mut EditorState) {
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(&s.nb, &mut w, &mut h);
    if w == s.pgw && h == s.pgh {
        return;
    }
    s.pgw = w;
    s.pgh = h;
    canvas_free(&mut s.page_canvas);
    s.canvas_page = 0;
    for k in 0..PAPER_COUNT as usize {
        canvas_free(&mut s.paper_prev[k]);
    }
    canvas_free(&mut s.custom_prev);
    s.paper = SharedPaper::default();
}

// Tests (editor.matches_closed): a hash of the open page drawn from its file as the notebook closed
// to be compacted, so the page drawn from its file when the notebook opens again (compacted by
// then, or not) can be held to it. (From its file, not the page canvas: a file keeps pressure in
// 16 bits, so a page read back can differ from the one drawn live by a level here and there.)
#[cfg(feature = "sim")]
#[derive(Default)]
struct Closed {
    id: FixedStr<STORE_ID_LEN>,
    page: u32,
    hash: u64,
}
#[cfg(feature = "sim")]
thread_local! {
    static G_CLOSED: RefCell<Closed> = RefCell::new(Closed::default());
}

#[cfg(feature = "sim")]
fn canvas_hash(c: &Canvas) -> u64 {
    let mut h: u64 = 1469598103934665603; // FNV-1a
    for y in 0..c.h {
        let row = y as usize * c.stride as usize;
        for x in 0..c.w as usize {
            h = (h ^ c.px[row + x] as u64).wrapping_mul(1099511628211);
        }
    }
    h
}

#[cfg(feature = "sim")]
fn closed_record(a: &mut App) {
    G_CLOSED.with(|g| g.borrow_mut().id.clear());
    let dpi = a.dpi;
    let s = ed(a);
    let mut p: Option<Box<Page>> = None;
    let mut c = Canvas::default();
    if store_read_page(&s.nb, s.index, &mut p) != 0 || canvas_init(&mut c, s.pgw, s.pgh) != 0 {
        return; // (page_free(p): dropped)
    }
    page_render(&mut c, p.as_deref().unwrap(), Rect { x: 0, y: 0, w: s.pgw, h: s.pgh }, dpi);
    let hash = canvas_hash(&c);
    let (id, page) = (s.nb.id, s.nb.page_ids[s.index as usize]);
    G_CLOSED.with(|g| {
        let mut g = g.borrow_mut();
        g.id = id;
        g.page = page;
        g.hash = hash;
    });
}
#[cfg(not(feature = "sim"))]
fn closed_record(_a: &mut App) {}

fn close_notebook(a: &mut App) {
    let s = ed(a);
    s.hold_eraser = false;
    if !s.open {
        return;
    }
    s.link_mode = false;
    // Pages an older version had saved on blank paper got their pictures back as they opened: the
    // thumbnail and the PDF it drew of them are drawn again.
    if store_take_restored_count() > 0 {
        s.changed = true;
        if s.index == 0 {
            s.thumb_dirty = true;
        }
    }
    save_page(a);
    write_thumb_if_needed(a);
    let dpi = a.dpi;
    let s = ed(a);
    if s.index != 0 && s.changed {
        // Keep the library thumbnail (page 1) in sync even if we never visited it.
        let mut p0: Option<Box<Page>> = None;
        if s.load_page(0, &mut p0) == 0 {
            let mut t = Canvas::default();
            if canvas_init(&mut t, s.pgw / 3, s.pgh / 3) == 0 {
                page_render_thumb(&mut t, p0.as_deref().unwrap(), dpi);
                store_save_thumb(s.nb.id.as_str(), &t);
            }
        }
    }
    if a.settings.mirror_pdf && ed(a).changed {
        let id = ed(a).nb.id;
        bg_mirror_pdf(id.as_str(), dpi, &app_mirror_dir(), false);
    }
    // The ink erase strokes took away entirely, out of the page files (compact.h): in the
    // background, as it can take long on the Kindle, and after the PDF (which it does not change).
    // Not while Grimoire leaves: the compaction would go with it, so the pages stay listed for the
    // next start.
    let id = ed(a).nb.id;
    if compact_listed(id.as_str()) {
        if !a.running {
            qlog!("compact: %s left listed for the next start (Grimoire is leaving)", id.as_str());
        } else {
            closed_record(a);
            bg_queue(BgKind::CompactPages, Some(id.as_str()), None, dpi, false);
        }
    }
    let s = ed(a);
    let unlisted = s.hist.iter().filter(|e| e.to_list).count() as i32;
    if unlisted != 0 {
        qlog!(
            "compact: %s: %d page(s) with erased ink could not be listed; their files keep it until listed again",
            id.as_str(),
            unlisted
        );
    }
    s.free_histories();
    s.paper = SharedPaper::default();
    canvas_free(&mut s.image_prev); // (it may have drawn a picture white: made again from the file)
    tplsheet_close(&mut s.ts);
    s.picture_missing = false;
    s.white_caches = false;
    s.noted.clear();
    pagesview_close(&mut s.pv);
    s.pv.tab_initialized = false;
    nav_free(&mut s.nav);
    nav_init(&mut s.nav);
    s.open = false;
    store_take_restored_count(); // (page 1, read for the thumbnail above, was drawn as it is now)
    bgimage_forget(None); // the notebook's pictures (up to 3 pages' worth) and its failures
}

/// loads, switches to SCR_EDITOR
pub fn editor_open_notebook(a: &mut App, id: &str) {
    close_notebook(a);
    // Every way into a notebook comes here (the Library's card and its menu, a new notebook): its
    // compaction, queued or running, is stopped before anything here reads or writes its pages (a
    // stopped compaction leaves each page file as it was; the list keeps the pages for later).
    bg_cancel_notebook(id);
    if store_load_info(id, &mut ed(a).nb) != 0 {
        app_toast(a, tr("Could not open that notebook"), 3000);
        return;
    }
    let s = ed(a);
    store_recover_pages(&s.nb); // pages an older version set aside that open now
    // Pages an older version saved on blank paper get their pictures back (bg/pages.ini), all of
    // them now: the thumbnail and PDF it drew are drawn again when the notebook closes.
    store_restore_listed_pages(&s.nb);
    bgimage_retry_missing(); // (a picture the Library found missing may be there now)
    use_page_size(s);
    s.index = clampi(s.nb.last_page, 0, s.nb.npages() - 1);
    s.page = s.page_at(s.index);
    if s.page.is_none() {
        s.free_histories();
        app_toast(a, tr("Could not open that notebook"), 3000);
        return;
    }
    s.open = true;
    nav_load(s.nb.id.as_str(), &mut s.nav);
    s.page_dirty = false;
    s.info_dirty = false;
    s.changed = false;
    if s.page_canvas.px.is_empty() && canvas_init(&mut s.page_canvas, s.pgw, s.pgh) != 0 {
        return;
    }
    render_page_full(a);
    ed(a).view_w = 0; // set the view up afresh on enter
    app_switch(a, ScreenId::Editor);
    arrive_readonly(a);
}

/// the same, at that page
// A page KOReader sent (annotate.h): notebook id open at its page page_id, or where
// editor_open_notebook opens it should it have no such page. added: the page was added to the
// notebook (or the notebook made) at this start, a change like any other: the notebook's PDF is
// made again as it closes, and the Library's card too when the page is its first.
pub fn editor_open_notebook_at(a: &mut App, id: &str, page_id: u32, added: bool) {
    editor_open_notebook(a, id);
    let s = ed(a);
    if !s.open || s.nb.id.as_bytes() != id.as_bytes() {
        return;
    }
    if let Some(i) = s.nb.page_ids.iter().position(|&p| p == page_id) {
        if i as i32 != s.index {
            go_to_page(a, i as i32);
        }
    }
    if !added {
        return;
    }
    let s = ed(a);
    s.changed = true;
    if s.index == 0 {
        s.thumb_dirty = true;
    }
}

/// a picture job is done: the open page drawn again
// A picture made in the background (a template's, a page KOReader sent: bgimage.h) may be the open
// page's paper, drawn white while its file was missing: the page is drawn again with it, now, or once
// the stroke or gesture under way is over (editor_tick), or as the editor comes back in front. Only
// such a page: one that shows its picture already (the job was another notebook's, or the templates'
// previews), or whose picture is still to come, stays as it is.
pub fn editor_picture_made(a: &mut App) {
    let Some(s) = a.ed.as_deref_mut() else { return };
    let Some(paper) = s.open_page().filter(|_| s.open).map(|p| p.paper) else { return };
    if !paper_is_image(paper) {
        return;
    }
    if !bgimage_undrawn(s.nb.id.as_str(), paper_image_id(paper)) {
        return;
    }
    s.canvas_page = 0;
    s.paper_stale = true; // (what erase strokes restore toward: drawn white too)
    if a.scr == ScreenId::Editor && !a.asleep && !editor_busy(a) && !ed_ref(a).page_canvas.px.is_empty() {
        render_page_full(a);
        compose_all(a, RefreshMode::Ui);
    }
}

pub fn editor_save_now(a: &mut App) -> bool {
    save_page(a)
}

// ---- ink ----

fn mark_changed(s: &mut EditorState) {
    s.page_dirty = true;
    s.changed = true;
    s.last_up = now_ms();
}

fn sample_of(s: &EditorState, ev: &InEvent) -> PenSample {
    let mut p = PenSample { x: 0.0, y: 0.0, pressure: ev.pressure, tilt_x: ev.tilt_x, tilt_y: ev.tilt_y, t_ms: in_sample_ms(ev) };
    to_page(s, ev.x, ev.y, &mut p.x, &mut p.y);
    p
}

// The open page's paper alone (S->paper), drawn again first if it holds another paper or size, or
// the page was drawn whole on an image paper since (paper_stale): what an erase stroke restores
// toward live, as page_render draws it (an image paper's picture too). False when there is no memory
// for it (the C's NULL); else it is in s.paper.
fn page_paper(s: &mut EditorState, dpi: f32) -> bool {
    if s.paper.px.is_some() && (s.paper.w != s.pgw || s.paper.h != s.pgh) {
        s.paper = SharedPaper::default(); // canvas_free
    }
    let fresh = s.paper.px.is_none();
    let k = s.open_page().unwrap().paper;
    if fresh || s.paper_k != k || s.paper_stale {
        // (Held shared, where the C draws in place: no stroke is drawing toward it now, and the
        // last one's brush lets go of it, as it is begun afresh for the next. Its pixels are drawn
        // again where they are, as the C's are, when nothing else holds them: a page-size canvas
        // made and copied anew would be megabytes of allocation and copying at a pen-down.)
        s.bs.paper = None;
        let kept = s.paper.px.take().and_then(|px| Rc::try_unwrap(px).ok());
        s.paper = SharedPaper::default();
        let mut c = Canvas::default();
        match kept {
            // (the same size: another one freed it above; canvas_init's stride is the width)
            Some(px) if px.len() == (s.pgw.max(0) as usize) * (s.pgh.max(0) as usize) => {
                c = Canvas { px, w: s.pgw, h: s.pgh, stride: s.pgw, clip: Rect { x: 0, y: 0, w: s.pgw, h: s.pgh } };
            }
            _ => {
                if canvas_init(&mut c, s.pgw, s.pgh) != 0 {
                    return false;
                }
            }
        }
        s.paper_k = k;
        s.paper_stale = false;
        bgimage_take_missing();
        let whole = Rect { x: 0, y: 0, w: c.w, h: c.h };
        page_paper_render(&mut c, s.open_page().unwrap(), whole, dpi); // (an image paper's picture too)
        if bgimage_take_missing() {
            note_white(s, bgimage_generation());
        }
        s.paper = SharedPaper { w: c.w, h: c.h, stride: c.stride, px: Some(Rc::new(c.px)) };
    }
    true
}

// Whether stroke s, drawn after Add a link, is too small to be the link's box both ways.
fn link_box_too_small(dpi: f32, s: &Stroke) -> bool {
    let (mut x0, mut y0, mut x1, mut y1) = (1e9f32, 1e9f32, -1e9f32, -1e9f32);
    for p in &s.pts {
        x0 = x0.min(p.x);
        x1 = x1.max(p.x);
        y0 = y0.min(p.y);
        y1 = y1.max(p.y);
    }
    let min_px = LINK_BOX_MIN_MM * dpi / 25.4;
    x1 - x0 < min_px && y1 - y0 < min_px
}

fn draw_begin(a: &mut App, ev: &InEvent) {
    if refuse_readonly(a) {
        // neither ink nor the eraser (tool, eraser end or side button)
        let s = ed(a);
        if s.btn_down {
            s.btn_used_for_ink = true;
        }
        s.owner = Owner::Modal; // swallow the rest of this stroke
        s.has_pressed = false;
        return;
    }
    let dpi = a.dpi;
    let App { settings: st, ed, .. } = a;
    let s = ed.as_deref_mut().unwrap();
    let mut ps = pen_stroke(st, ev.eraser || s.hold_eraser, ev.side_button);
    // The pen's button drawing white while the eraser is in use: with the pen a tap on the eraser
    // goes back to, a pin too (pen_drawing_tool knows only the tool button's).
    if ps.act == PenAct::White && st.current_tool == TOOL_ERASER {
        ps.tool = eraser_back(st);
    }
    if ps.act == PenAct::Whole {
        s.owner = Owner::Erase;
        s.snap = s.open_page().unwrap().strokes.iter().map(|x| stroke_id(x)).collect();
        s.removed.clear();
        let (mut ex, mut ey) = (0.0, 0.0);
        to_page(s, ev.x, ev.y, &mut ex, &mut ey);
        s.ex = ex;
        s.ey = ey;
        s.erase_dirty = RECT_EMPTY;
        if s.btn_down {
            s.btn_used_for_ink = true;
        }
        display_live_ink(true); // a pen stroke too: its frames never wait (erase_end repaints them)
        return;
    }
    if s.btn_down {
        s.btn_used_for_ink = true;
    }
    s.owner = Owner::Draw;
    let n = s.open_page().unwrap().strokes.len() as u32;
    let mut seed = ev.t_ms.wrapping_mul(2654435761) ^ n.wrapping_mul(40503) ^ (ev.x * 97.0) as u32;
    // White ink and the precise eraser are erase strokes (the same draw path as ink: its dabs,
    // dirty rects, refreshes, settle, zoom, rotation and undo): white with the pen's own settings
    // and texture, the eraser with its round tip, at the size on screen it has at any zoom.
    let erase = ps.act != PenAct::Ink;
    let paper = erase && page_paper(s, dpi);
    if erase && !paper {
        qlog!("editor: no memory for the page's paper; the erase stroke is not drawn");
        s.owner = Owner::Modal; // swallow the rest of this stroke
        s.has_pressed = false;
        return;
    }
    let prm = if ps.act == PenAct::Precise {
        eraser_at_zoom(&st.tools[TOOL_ERASER.0 as usize], s.zoom)
    } else {
        st.tools[ps.tool.0 as usize]
    };
    // Ink never shares a seed with a stroke or a link of the page: a link's box is its seed's strokes.
    if !erase {
        let pid = s.nb.page_ids[s.index as usize];
        seed = nav_new_seed(&s.nav, s.open_page().unwrap(), pid, seed);
    }
    // (stroke_new cannot fail here: the C's NULL check has nothing to catch)
    let mut cur = if erase { stroke_new(TOOL_ERASER, &prm, erase_seed(ps.tool, seed)) } else { stroke_new(ps.tool, &prm, seed) };
    stroke_mark_tip(&mut cur); // live and replayed, it keeps the tip it is drawn with now
    let (ppx, pstride) = if paper { (s.paper.px.clone(), s.paper.stride) } else { (None, 0) };
    stroke_brush_begin(&mut s.bs, &cur, dpi, ppx, 0, 0, pstride);
    s.cur = Some(cur);
    smoother_reset(&mut s.sm);
    // Ink asks for the pen mode (where it is on); an erase stroke is live ink without it, as the
    // stroke eraser is (the driver's pen path is for ink going down).
    if erase {
        display_live_ink(true);
    } else {
        display_pen_mode(true);
    }
}

fn draw_sample(a: &mut App, ev: &InEvent) {
    let dpi = a.dpi;
    let s = ed(a);
    let mut p = sample_of(s, ev);
    // The pen's samples between the last one and this were lost (the kernel dropped them): a straight
    // line to where the pen is, not one eased from where the smoothing had got to.
    if ev.gap {
        smoother_reset(&mut s.sm);
    }
    let cur = s.cur.as_deref_mut().unwrap();
    smoother_apply(&mut s.sm, cur.params.smoothing, &mut p);
    stroke_append(cur, &p, dpi);
    let d = brush_add(&mut s.bs, &mut s.page_canvas, &p);
    if !rect_is_empty(d) {
        show(a, d, RefreshMode::Fast);
    }
}

fn draw_end(a: &mut App) {
    let dpi = a.dpi;
    let s = ed(a);
    let st = s.cur.take();
    s.owner = Owner::None;
    display_pen_mode(false);
    let Some(st) = st else { return };
    if st.pts.is_empty() {
        stroke_free(st);
        return;
    }
    // Settle later, once the writer stops (see settle_now).
    s.settle = rect_union(s.settle, st.bbox);
    s.settle_at = now_ms().wrapping_add(SETTLE_NEAR_MS);
    if stroke_is_erase(&st) && !s.bs.restored {
        // An erase stroke that changed no pixel of the page (over blank paper, or ink the eraser
        // had taken already): not kept, and no undo step that would do nothing. Its fast frames
        // still settle with the rest.
        stroke_free(st);
        return;
    }
    if s.link_mode && !stroke_is_erase(&st) && link_box_too_small(dpi, &st) {
        // A dot or a tick for a link's box: taken back (no undo step), and the next stroke is tried.
        let d = st.bbox;
        stroke_free(st);
        draw_page(a, d);
        show(a, d, RefreshMode::Ui);
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): after Add a link, the
        // stroke drawn was too small to be the link's box.
        app_toast(a, tr("Too small for a link: draw around what links."), 3000);
        return;
    }
    let h = s.cur_hist();
    let had_undo = h.is_some_and(|i| history_can_undo(&s.hist[i].h));
    let had_redo = h.is_some_and(|i| history_can_redo(&s.hist[i].h));
    let ip = s.page_ix().unwrap();
    let was_empty = s.hist[ip].page.as_deref().unwrap().strokes.is_empty();
    let erase = stroke_is_erase(&st);
    let seed = st.seed;
    // (the stroke's identity survives the move into the page: PORTING.md, wave 2's conventions)
    let sid = stroke_id(&st);
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    page_add(page, st);
    let kept = page.strokes.last().is_some_and(|x| stroke_id(x) == sid);
    if let (Some(ih), true) = (h, kept) {
        let (hh, page) = hist_page(&mut s.hist, ih, ip);
        let n = page.strokes.len();
        history_push_add(hh, n as i32 - 1, &page.strokes[n - 1]);
    }
    let e = if erase && kept {
        let pid = s.nb.page_ids[s.index as usize];
        s.entry_for(pid)
    } else {
        None
    };
    if let Some(e) = e {
        s.hist[e].to_list = true; // listed for compaction once the page is saved
    }
    mark_changed(s);
    // Only touch the panel when Undo/Redo availability actually changed.
    if !had_undo || had_redo || was_empty {
        s.panel_stale = true;
    }
    if s.link_mode && !erase && kept {
        link_box_drawn(a, seed);
    }
}

fn dist2_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    // (contracted as clang contracts the C: PORTING.md, Floating point)
    let dx = bx - ax;
    let dy = by - ay;
    let l2 = fmla(dx, dx, dy * dy);
    let t = if l2 > 0.0 { clampf(fmla(px - ax, dx, (py - ay) * dy) / l2, 0.0, 1.0) } else { 0.0 };
    let ex = px - fmla(dx, t, ax);
    let ey = py - fmla(dy, t, ay);
    fmla(ex, ex, ey * ey)
}

// A pixel shows ink when the page there is darker than its paper by more than this: a level or
// two left by a dab's rounding is no ink anyone can see.
const INK_SHOWS: i32 = 6;

// Whether the page is darker than its paper by more than INK_SHOWS anywhere over R (page px) within
// r px of the segment (x0, y0)-(x1, y1) and within stroke s's reach of its line (as far as its bbox
// reaches): the only pixels there where its ink can show. The page canvas may still show strokes
// taken off the page in this sweep, which only makes more of it dark.
fn dark_near(s_ed: &EditorState, dpi: f32, s: &Stroke, paper: &SharedPaper, rr: Rect, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let pc = &s_ed.page_canvas;
    let ppx = paper.px.as_deref().unwrap();
    let reach = brush_max_radius(s.tool, &s.params, dpi) + 2.0;
    let n = s.pts.len();
    for j in 0..n {
        let pa = &s.pts[j];
        let pb = &s.pts[if j + 1 < n { j + 1 } else { j }];
        let sr = rect_intersect(rect_from_points(pa.x, pa.y, pb.x, pb.y, reach + 1.0), rr);
        for y in sr.y..sr.y + sr.h {
            let row = y as usize * pc.stride as usize;
            let pr = y as usize * paper.stride as usize;
            for x in sr.x..sr.x + sr.w {
                if ppx[pr + x as usize] as i32 - pc.px[row + x as usize] as i32 <= INK_SHOWS {
                    continue;
                }
                let qx = x as f32 + 0.5;
                let qy = y as f32 + 0.5;
                if dist2_to_segment(qx, qy, x0, y0, x1, y1) <= r * r
                    && dist2_to_segment(qx, qy, pa.x, pa.y, pb.x, pb.y) <= reach * reach
                {
                    return true;
                }
            }
        }
    }
    false
}

// Whether stroke i's own ink shows over R (page px, on the page and in its bbox) within r px of the
// segment (x0, y0)-(x1, y1): the page drawn there without it is lighter than with it by more than
// INK_SHOWS somewhere in that reach, as page_droppable tells ink that matters. Both are drawn afresh
// from the page as it is now (without the strokes this sweep took off, which the page canvas may
// still show), over the page canvas, which gets its own pixels back. 1 or 0; -1 out of memory.
fn own_ink_shows(a: &mut App, i: i32, rr: Rect, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> i32 {
    let dpi = a.dpi;
    let s = ed(a);
    let mut keep = Canvas::default();
    let mut with = Canvas::default();
    if canvas_init(&mut keep, rr.w, rr.h) != 0 {
        return -1;
    }
    if canvas_init(&mut with, rr.w, rr.h) != 0 {
        return -1;
    }
    let ip = s.page_ix().unwrap();
    let pc = &mut s.page_canvas;
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    canvas_blit(&mut keep, pc, rr, 0, 0); // (put back below)
    page_render(pc, page, rr, dpi);
    canvas_blit(&mut with, pc, rr, 0, 0);
    // Without it: taken off the list and put back in its place (the list then has room for it, so
    // page_insert cannot fail).
    let st = page_remove(page, i);
    page_render(pc, page, rr, dpi);
    page_insert(page, i, st);
    let mut shows = false;
    let mut y = 0;
    while y < rr.h && !shows {
        let without = (rr.y + y) as usize * pc.stride as usize + rr.x as usize;
        let drawn = y as usize * with.stride as usize;
        let qy = (rr.y + y) as f32 + 0.5;
        let mut x = 0;
        while x < rr.w && !shows {
            shows = pc.px[without + x as usize] as i32 - with.px[drawn + x as usize] as i32 > INK_SHOWS
                && dist2_to_segment((rr.x + x) as f32 + 0.5, qy, x0, y0, x1, y1) <= r * r;
            x += 1;
        }
        y += 1;
    }
    canvas_blit(pc, &keep, Rect { x: 0, y: 0, w: rr.w, h: rr.h }, rr.x, rr.y);
    shows as i32
}

// Whether the stroke eraser's segment (x0, y0)-(x1, y1), r wide each side, touches ink of stroke i
// (page px) that shows. A stroke no erase stroke drawn after it reaches shows wherever it is, as
// before 1.1. One that white ink or the precise eraser drawn after it reaches shows only where its
// own ink is on the page (own_ink_shows): not where they took it away, though the rest of it shows,
// nor where all there is to see is ink drawn later over that place, or a paper darker than it. That
// takes drawing the page there twice, so first the page canvas is held to the paper (S->paper):
// where nothing in reach is darker than the paper, nothing of it shows (scrubbing over white ink).
fn touches_visible_ink(a: &mut App, i: i32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let dpi = a.dpi;
    let s = ed(a);
    let page = s.open_page().unwrap();
    let st = &page.strokes[i as usize];
    let mut covered = false;
    let mut k = i as usize + 1;
    while k < page.strokes.len() && !covered {
        covered = stroke_is_erase(&page.strokes[k]) && rect_overlaps(page.strokes[k].bbox, st.bbox);
        k += 1;
    }
    if !covered {
        return true;
    }
    let pc = &s.page_canvas;
    if s.canvas_page != s.open_page_ref() || pc.px.is_empty() {
        return true; // nothing to tell by: as before
    }
    let rr = rect_intersect(
        rect_intersect(rect_from_points(x0, y0, x1, y1, r + 1.0), st.bbox),
        Rect { x: 0, y: 0, w: pc.w, h: pc.h },
    );
    if rect_is_empty(rr) {
        return false;
    }
    // A pixel the eraser touches anywhere has its centre within half its diagonal more (so even the
    // smallest eraser, zoomed in, touches a pixel wherever it is set down).
    let rp = r + 0.71;
    let paper = page_paper(s, dpi);
    let s = ed(a);
    let pc = &s.page_canvas;
    if paper
        && s.paper.w == pc.w
        && s.paper.h == pc.h
        && !dark_near(s, dpi, &s.open_page().unwrap().strokes[i as usize], &s.paper, rr, x0, y0, x1, y1, rp)
    {
        return false;
    }
    own_ink_shows(a, i, rr, x0, y0, x1, y1, rp) != 0 // (out of memory: as before)
}

fn erase_segment(a: &mut App, sx: f32, sy: f32, pressure: f32) {
    let dpi = a.dpi;
    let ep = a.settings.tools[TOOL_ERASER.0 as usize];
    let p = brush_pressure(&ep, pressure);
    let s = ed(a);
    // The eraser keeps its size on screen whatever the zoom.
    let r = mm_to_px(lerpf(ep.size_min_mm, ep.size_max_mm, p), dpi) * 0.5 / s.zoom;
    let (mut x, mut y) = (0.0, 0.0);
    to_page(s, sx, sy, &mut x, &mut y);
    let mut d = RECT_EMPTY;
    // (the open page's cache entry: nothing below adds or drops one)
    let ip = s.page_ix().unwrap();
    let n = s.hist[ip].page.as_deref().unwrap().strokes.len() as i32;
    for i in (0..n).rev() {
        let s = ed(a);
        let (ex, ey) = (s.ex, s.ey);
        let st = &s.hist[ip].page.as_deref().unwrap().strokes[i as usize];
        // Erase strokes stay: taking one away would bring back the ink it covers. Ink they took
        // away is not there to touch (touches_visible_ink).
        let passed = stroke_is_erase(st) || !stroke_hit_segment(st, ex, ey, x, y, r, dpi);
        if passed || !touches_visible_ink(a, i, ex, ey, x, y, r) {
            continue;
        }
        let s = ed(a);
        let st = page_remove(s.hist[ip].page.as_deref_mut().unwrap(), i);
        if s.removed.len() == s.removed.capacity() {
            let nc = if s.removed.capacity() > 0 { s.removed.capacity() * 2 } else { 16 };
            if s.removed.try_reserve_exact(nc - s.removed.len()).is_err() {
                page_insert(s.hist[ip].page.as_deref_mut().unwrap(), i, st); // could not track it: put it back
                continue;
            }
        }
        d = rect_union(d, st.bbox);
        s.removed.push(st);
    }
    let s = ed(a);
    s.ex = x;
    s.ey = y;
    if !rect_is_empty(d) {
        draw_page(a, d);
        show(a, d, RefreshMode::Fast);
        let s = ed(a);
        s.erase_dirty = rect_union(s.erase_dirty, d);
    }
}

fn erase_end(a: &mut App) {
    let s = ed(a);
    s.owner = Owner::None;
    display_live_ink(false);
    if !s.removed.is_empty() {
        // History wants original indices ascending, paired with their strokes.
        let mut removed: Vec<Option<Box<Stroke>>> = std::mem::take(&mut s.removed).into_iter().map(Some).collect();
        let mut idx: Vec<i32> = Vec::with_capacity(removed.len());
        let mut st: Vec<Box<Stroke>> = Vec::with_capacity(removed.len());
        for (i, &sid) in s.snap.iter().enumerate() {
            for slot in removed.iter_mut() {
                if slot.as_deref().is_some_and(|x| stroke_id(x) == sid) {
                    idx.push(i as i32);
                    st.push(slot.take().unwrap());
                }
            }
        }
        // Strokes not found in the snapshot cannot happen (erase only touches the page),
        // but never leak them. (Those found were taken out above: the ones left are these.)
        for x in removed.into_iter().flatten() {
            stroke_free(x);
        }
        match s.cur_hist() {
            Some(h) => history_push_erase(&mut s.hist[h].h, &idx, st),
            None => st.into_iter().for_each(stroke_free),
        }
        let d = s.erase_dirty;
        show(a, d, RefreshMode::Ui);
        mark_changed(ed(a));
        repaint_panel(a, RefreshMode::Ui);
    }
    let s = ed(a);
    s.snap = Vec::new();
}

fn clear_page(a: &mut App) {
    if refuse_readonly(a) {
        return;
    }
    if ed(a).open_page().is_none_or(|p| p.strokes.is_empty()) {
        app_toast(a, tr("This page is already empty"), 1500);
        return;
    }
    let s = ed(a);
    let Some(h) = s.cur_hist() else { return };
    let ip = s.page_ix().unwrap();
    let (hh, page) = hist_page(&mut s.hist, h, ip);
    history_push_clear(hh, page); // takes every stroke off the page, undoable
    render_page_full(a);
    compose_all(a, RefreshMode::Full);
    mark_changed(ed(a));
    repaint_panel(a, RefreshMode::Full);
    app_toast(a, tr("Page cleared"), 1500);
}

// Whether a sheet is in front of the page: the Pages view, the tool picker, Custom music, Display
// or the paper sheet. The pen's button clears no page from under one.
fn sheet_open(s: &EditorState) -> bool {
    s.pv.open || s.tools_open || s.mb.open || s.dm.open || s.paper_open || s.ts.open
}

/// A cheap snapshot for the early input hook. UI gestures never paint provisional ink. The
/// real brush remains authoritative: textures, nibs and paper restoration arrive in the tiles.
pub fn editor_live_style(a: &App, ev: &InEvent, out: &mut LiveStyle) {
    let Some(s) = a.ed.as_deref() else { return };
    let Some(page) = s.open_page() else { return };
    if !s.open || page.readonly || sheet_open(s) || s.pinching || s.link_mode || (s.owner != Owner::None && s.owner != Owner::Draw && s.owner != Owner::Erase) {
        return;
    }
    let st = &a.settings;
    let mut ps = pen_stroke(st, ev.eraser || s.hold_eraser, ev.side_button);
    if ps.act == PenAct::White && st.current_tool == TOOL_ERASER {
        ps.tool = eraser_back(st);
    }
    let mut prm = if ps.act == PenAct::Precise { eraser_at_zoom(&st.tools[TOOL_ERASER.0 as usize], s.zoom) } else { st.tools[ps.tool.0 as usize] };
    if s.owner == Owner::Draw {
        if let Some(cur) = s.cur.as_deref() {
            ps.tool = stroke_render_tool(cur);
            ps.act = if stroke_is_erase(cur) { PenAct::White } else { PenAct::Ink };
            prm = cur.params;
        }
    }
    out.enabled = 1;
    out.tool = ps.tool.0 as u8;
    out.white = (ps.act != PenAct::Ink) as u8;
    out.whole = (ps.act == PenAct::Whole) as u8;
    out.width_min = prm.size_min_mm * a.dpi / 25.4 * s.zoom;
    out.width_max = prm.size_max_mm * a.dpi / 25.4 * s.zoom;
    out.pressure_floor = prm.pressure_floor;
    out.pressure_ceiling = prm.pressure_ceiling;
    out.pressure_gamma = prm.pressure_gamma;
    out.opacity_min = prm.opacity_min;
    out.opacity_max = prm.opacity_max;
    out.clip = rect_intersect(page_to_screen(a, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh }), Rect { x: 0, y: 0, w: a.w, h: a.h });
    out.exclude = s.panel;
}

// Side button: clicks and holds that happen while not drawing. A press used for something else (a
// stroke, or the pen on a button or a sheet while it is held) is neither a click nor a hold.
fn pen_button(a: &mut App, ev: &InEvent) {
    let t = ev.t_ms;
    let s = ed(a);
    if ev.side_button {
        s.btn_down = true;
        s.btn_down_at = t;
        s.btn_used_for_ink = s.owner != Owner::None || sheet_open(s);
        s.btn_hold_fired = false;
        return;
    }
    if !s.btn_down {
        return;
    }
    s.btn_down = false;
    if s.btn_used_for_ink || s.btn_hold_fired || t.wrapping_sub(s.btn_down_at) > 450 {
        return;
    }
    // A click: keep the last three; three within 1.2 s clear the page.
    if s.nclicks == 3 {
        s.clicks[0] = s.clicks[1];
        s.clicks[1] = s.clicks[2];
        s.nclicks = 2;
    }
    s.clicks[s.nclicks as usize] = t;
    s.nclicks += 1;
    if s.nclicks == 3 && s.clicks[2].wrapping_sub(s.clicks[0]) <= 1200 {
        s.nclicks = 0;
        if s.owner == Owner::None && !sheet_open(s) {
            clear_page(a);
        }
    }
}

fn undo_redo(a: &mut App, undo: bool) {
    settle_now(a);
    if refuse_readonly(a) {
        return;
    }
    let s = ed(a);
    let Some(h) = s.cur_hist() else { return };
    let Some(ip) = s.page_ix() else { return };
    let mut d = RECT_EMPTY;
    let (hh, page) = hist_page(&mut s.hist, h, ip);
    if !(if undo { history_undo(hh, page, &mut d) } else { history_redo(hh, page, &mut d) }) {
        return;
    }
    draw_page(a, d); // (back to a template whose picture is still being made: white until it is)
    let s = ed(a);
    let full = d.w >= s.pgw && d.h >= s.pgh;
    show(a, d, if full { RefreshMode::Full } else { RefreshMode::Ui });
    mark_changed(ed(a));
    repaint_panel(a, RefreshMode::Ui);
}

// ---- panel interaction ----

// ---- pages view: open, act, delete ----

fn pages_show(a: &mut App) {
    ed(a).link_mode = false;
    save_page(a);
    let index = ed(a).index;
    if !with_pv(a, |pv, a, nb, nav| pagesview_open(pv, a, nb, nav, index)) {
        return;
    }
    compose_all(a, RefreshMode::Full);
}

// name-page: the Pages view's naming keyboard for the open page, over its Index, as Name this page
// there opens it; its Done or Cancel goes back to the page (pages_event). Nothing while the names
// cannot be changed (that button is greyed out then).
fn pages_name(a: &mut App) {
    if !ed(a).nav.writable {
        return;
    }
    ed(a).link_mode = false;
    save_page(a);
    let index = ed(a).index;
    if !with_pv(a, |pv, a, nb, nav| pagesview_name(pv, a, nb, nav, index)) {
        return;
    }
    compose_all(a, RefreshMode::Full);
}

fn pages_hide(a: &mut App, open_index: i32) {
    pagesview_close(&mut ed(a).pv);
    if open_index >= 0 && open_index != ed(a).index {
        go_to_page(a, open_index);
    } else {
        compose_all(a, RefreshMode::Full);
    }
}

// Removes a page's in-memory copy and history (the page is being deleted).
fn forget_page(s: &mut EditorState, pid: u32) {
    let Some(i) = s.hist.iter().position(|e| e.page_id == pid) else { return };
    if s.is_open_page(i) {
        s.page = None;
    }
    // (its history and page go with the entry; the last entry takes its place, as in the C)
    s.hist.swap_remove(i);
}

fn pages_delete(a: &mut App) {
    let s = ed(a);
    let n = s.nb.npages();
    let k = pagesview_ticked_count(&s.pv);
    if k == 0 {
        return;
    }
    if k == n {
        // A notebook keeps at least one page: add a fresh blank one to stay behind.
        let paper = s.open_page().map_or(s.nb.paper, |p| p.paper);
        if store_add_page(&mut s.nb, n, paper) != 0 {
            app_toast(a, tr("Could not delete those pages"), 2500);
            return;
        }
    }
    let cur = s.index;
    let mut deleted_before = 0;
    for i in (0..n).rev() {
        if !pagesview_ticked(&s.pv, i) {
            continue;
        }
        let pid = s.nb.page_ids[i as usize];
        if store_delete_page(&mut s.nb, i) != 0 {
            continue; // the file goes to grimoire/trash
        }
        forget_page(s, pid);
        if i < cur {
            deleted_before += 1;
        }
    }
    pagesview_untick(&mut s.pv);
    s.index = clampi(cur - deleted_before, 0, s.nb.npages() - 1);
    s.pv.current = s.index;
    s.nb.last_page = s.index;
    s.page = s.page_at(s.index);
    if s.page.is_some() {
        render_page_full(a);
    }
    let s = ed(a);
    s.changed = true;
    s.page_dirty = s.index == 0; // refreshes the library thumbnail when page 1 changed
    s.info_dirty = true;
    save_page(a);
    let s = ed(a);
    if s.nav.writable && nav_save(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
        qlog!("index: deleted-page names left for a later save");
    }
    if s.nav.links_writable && nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
        qlog!("index: deleted pages' links left for a later save");
    }
    let mut msg = String::new();
    // TRANSLATORS: deleted pages go to the trash folder on the Kindle's storage (%s).
    csnprintf!(&mut msg, 256, ng_("Deleted %d page (kept in %s)", "Deleted %d pages (kept in %s)", k as i64), k, "grimoire/trash");
    compose_all(a, RefreshMode::Full);
    app_toast(a, &msg, 2500);
}

// A page added (or copied) at index `at` this session: the open page moves along if it was there or
// after, and the new page opens (the Pages view closes).
fn pages_added(a: &mut App, at: i32) {
    let s = ed(a);
    if at <= s.index {
        s.index += 1;
    }
    s.nb.last_page = s.index;
    s.pv.current = s.index;
    s.changed = true;
    pages_hide(a, at);
}

// New page: the notebook's own paper (Blank in a book's notebook: annotate.h), at the index the
// Pages view gives, never the first, so the Library's card stays the same page. A page that holds
// no book page (in no line of source.ini) is a note page: a book page annotated later still goes
// before the first page that holds a later book page, after the notes on the one before.
fn pages_new(a: &mut App, at: i32) {
    if !save_page(a) {
        return;
    }
    let s = ed(a);
    let paper = s.nb.paper;
    if at < 1 || store_add_page(&mut s.nb, at, paper) != 0 {
        app_toast(a, tr("Could not add a page"), 3000);
        return;
    }
    pages_added(a, at);
}

// Copy page: page `index` copied right after it (store_duplicate_page: its ink and its paper, a
// picture shared), which opens. The copy is a note page; one with erase strokes is listed for
// compaction, as the page it copies may be.
fn pages_copy(a: &mut App, index: i32) {
    if !save_page(a) {
        return;
    }
    if store_duplicate_page(&mut ed(a).nb, index, index + 1) != 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Copy page in the
        // Pages view could not copy the page (the storage is full, or the notebook has 2000 pages).
        app_toast(a, tr("Could not copy that page"), 3000);
        return;
    }
    pages_added(a, index + 1);
    let s = ed(a);
    let erased = s.open_page().is_some_and(|p| p.strokes.iter().any(|x| stroke_is_erase(x)));
    if erased && s.index == index + 1 {
        let pid = s.nb.page_ids[s.index as usize];
        note_for_compaction(s, pid);
    }
    let mut msg = String::new();
    // TRANSLATORS: a toast as the copy Copy page made opens (about 1700 px at 44 px in bold, or two
    // lines): %d is the page it copies.
    csnprintf!(&mut msg, 160, tr("This page is a copy of page %d."), index + 1);
    app_toast(a, &msg, 2500);
}

// Add a link, from the Index or the add-link shortcut on the page: the next ink stroke on the open
// page is the link's box (link_box_drawn). Neither on a read-only page nor while links.ini does not
// read (the Index's button is greyed out then).
fn link_start(a: &mut App) {
    if refuse_readonly(a) || !ed(a).nav.links_writable {
        return;
    }
    ed(a).link_mode = true;
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): after Add a link, draw
    // a box or a ring around what links with the pen; the Pages view then asks where it goes.
    app_toast(a, tr("Draw around what links. Then tap where it goes."), 4000);
}

// A link still waiting for its box is not made (Close menu, or add-link pressed again).
fn link_stop(a: &mut App) {
    ed(a).link_mode = false;
    app_toast(a, tr("No link made."), 2500);
}

// After Add a link: stroke s (seed: its seed), just drawn on the open page, is the link's box. The
// Pages view picks where it goes.
fn link_box_drawn(a: &mut App, seed: u32) {
    let s = ed(a);
    s.link_mode = false;
    s.link_seed = seed;
    save_page(a);
    let index = ed(a).index;
    if !with_pv(a, |pv, a, nb, nav| pagesview_pick(pv, a, nb, nav, index, false)) {
        return;
    }
    compose_all(a, RefreshMode::Full);
    // TRANSLATORS: a toast over the Pages view (about 1700 px at 44 px in bold, or two lines): the
    // box of a new link is drawn; tap the page (or the Index) it goes to.
    app_toast(a, tr("Where does the link go? Tap its page."), 3000);
}

// The link the Pages view picked for goes to target ("index" or "p<page id>"), and the view closes on
// the open page.
fn link_to(a: &mut App, target: &str, done: &str) {
    let s = ed(a);
    let pid = s.nb.page_ids[s.index as usize];
    pages_hide(a, -1);
    let s = ed(a);
    let seed = s.link_seed;
    if nav_link_set(&mut s.nav, pid, seed, target) != 0 || nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
        nav_load(s.nb.id.as_str(), &mut s.nav); // (back to the links as saved)
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines).
        app_toast(a, tr("Could not save the link."), 3000);
        return;
    }
    s.changed = true;
    app_toast(a, done, 2500);
}

// Back while the Pages view picks for a link drawn just now: its box is taken back (an undo step,
// the box's own: nothing else happened since).
fn link_cancel(a: &mut App) {
    let had = ed(a).pv.pick_had;
    pages_hide(a, -1);
    if had {
        return;
    }
    let s = ed(a);
    let h = s.cur_hist();
    let can = h.is_some_and(|i| history_can_undo(&s.hist[i].h));
    let seed = s.link_seed;
    if can && s.open_page().is_some_and(|p| p.strokes.last().is_some_and(|x| x.seed == seed)) {
        undo_redo(a, true);
    }
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Back while picking where a
    // new link goes took its box back.
    app_toast(a, tr("No link made."), 2500);
}

fn pages_event(a: &mut App, ph: i32, ev: &InEvent) {
    match with_pv(a, |pv, a, nb, nav| pagesview_pointer(pv, a, nb, nav, ph, ev)) {
        PvResult::Close => {
            if ed(a).pv.picking {
                link_cancel(a);
            } else {
                pages_hide(a, -1);
            }
        }
        PvResult::Link => {
            pages_hide(a, -1);
            link_start(a);
        }
        PvResult::Pick => {
            let s = ed(a);
            let to = s.pv.target;
            let mut target = String::new();
            let mut done = String::new();
            csnprintf!(&mut target, NAV_TARGET_LEN, "p%u", s.nb.page_ids[to as usize]);
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the link drawn on the
            // page now goes to page %d (a tap on it with a finger turns there).
            csnprintf!(&mut done, 160, tr("The link goes to page %d."), to + 1);
            link_to(a, &target, &done);
        }
        PvResult::PickIndex => {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the link drawn on the
            // page now opens the Index (the list of named pages).
            link_to(a, "index", tr("The link goes to the Index."));
        }
        PvResult::Unlink => {
            pages_hide(a, -1);
            let s = ed(a);
            let (pid, seed) = (s.nb.page_ids[s.index as usize], s.link_seed);
            nav_link_remove(&mut s.nav, pid, seed);
            if nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
                nav_load(s.nb.id.as_str(), &mut s.nav);
                app_toast(a, tr("Could not save the link."), 3000);
                return;
            }
            s.changed = true;
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Remove link took the
            // link away; the ink drawn around it is still on the page.
            app_toast(a, tr("Link removed. Its ink stays on the page."), 2500);
        }
        PvResult::Open => {
            let to = ed(a).pv.target;
            pages_hide(a, to);
        }
        PvResult::Delete => pages_delete(a),
        PvResult::New => {
            let to = ed(a).pv.target;
            pages_new(a, to);
        }
        PvResult::Copy => {
            let to = ed(a).pv.target;
            pages_copy(a, to);
        }
        PvResult::Name => {
            let s = ed(a);
            let pid = s.nb.page_ids[s.index as usize];
            let mut old = String::new();
            i18n_copy(&mut old, nav_name(&s.nav, pid), NAV_NAME_LEN);
            let text = s.pv.kb.text;
            if nav_set_name(&mut s.nav, pid, text.as_str()) != 0 || nav_save(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
                nav_set_name(&mut s.nav, pid, &old);
                app_toast(a, tr("Could not save the page name."), 3000);
            } else {
                s.changed = true;
            }
            // Named from the page (name-page): back to it, as its Cancel goes back; else the Index again.
            if ed(a).pv.name_only {
                pages_hide(a, -1);
            } else {
                compose_all(a, RefreshMode::Ui);
            }
        }
        _ => {}
    }
}

// The link under (x, y) on screen, on the open page: false for none.
fn link_under(a: &App, x: f32, y: f32, out: &mut NavLink) -> bool {
    let s = ed_ref(a);
    let Some(page) = s.open_page() else { return false };
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, x, y, &mut px, &mut py);
    nav_link_at(&s.nav, page, s.nb.page_ids[s.index as usize], px, py, NAV_LINK_MIN_MM * a.dpi / 25.4, out)
}

// A finger's tap at (x, y): a link there goes to its page, or opens the Index.
fn link_follow(a: &mut App, x: f32, y: f32) {
    let mut l = NavLink::default();
    if !link_under(a, x, y, &mut l) {
        return;
    }
    let to = nav_link_page(&l, &ed(a).nb);
    if to >= 0 {
        go_to_page(a, to);
    } else if to == NAV_TO_INDEX {
        pages_show(a);
        ed(a).pv.index_tab = true;
        compose_all(a, RefreshMode::Full);
    } else {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a finger tapped a link
        // whose page was deleted (or that a newer Grimoire made, to a place this one cannot open).
        app_toast(a, tr("That link's page is not in this notebook."), 3000);
    }
}

// A finger held still on a link: the Pages view opens to change where it goes, or remove it.
fn link_hold(a: &mut App) {
    ed(a).link_held = true;
    let mut l = NavLink::default();
    let (x0, y0) = (ed(a).link_press.x0, ed(a).link_press.y0);
    if !link_under(a, x0, y0, &mut l) {
        return;
    }
    if refuse_readonly(a) {
        return;
    }
    if !ed(a).nav.links_writable {
        return;
    }
    ed(a).link_seed = l.seed;
    save_page(a);
    ed(a).link_mode = false;
    let index = ed(a).index;
    if !with_pv(a, |pv, a, nb, nav| pagesview_pick(pv, a, nb, nav, index, true)) {
        return;
    }
    compose_all(a, RefreshMode::Full);
    let mut msg = String::new();
    let to = nav_link_page(&l, &ed(a).nb);
    if to >= 0 {
        // TRANSLATORS: a toast over the Pages view (about 1700 px at 44 px in bold, or two lines): a
        // finger held on a link; it goes to page %d now, and a tap on another page changes that.
        csnprintf!(&mut msg, 160, tr("The link goes to page %d. Tap another to change it."), to + 1);
    } else if to == NAV_TO_INDEX {
        // TRANSLATORS: the same toast for a link to the Index.
        csnprintf!(&mut msg, 160, "%s", tr("The link goes to the Index. Tap a page to change it."));
    } else {
        // TRANSLATORS: the same toast for a link whose page is gone.
        csnprintf!(&mut msg, 160, "%s", tr("The link's page is not in this notebook. Tap a page to change it."));
    }
    app_toast(a, &msg, 3500);
}

fn move_panel_to(a: &mut App, x: i32, y: i32, m: RefreshMode) {
    let old = ed(a).panel;
    ed(a).panel = panel_rect_at(a, x, y);
    compose(a, old);
    draw_panel(a);
    let p = ed(a).panel;
    app_invalidate(a, rect_union(old, p), m);
}

fn persist_panel(a: &mut App) {
    let s = ed_ref(a);
    let (p, collapsed) = (s.panel, s.collapsed);
    a.settings.panel_x = p.x;
    a.settings.panel_y = p.y;
    a.settings.panel_collapsed = collapsed;
    settings_save(&a.settings);
}

fn set_collapsed(a: &mut App, c: bool) {
    let old = ed(a).panel;
    ed(a).collapsed = c;
    // Collapse toward the top-right corner of where the panel was (and expand from there).
    let x = if c { old.x + old.w - box_() } else { old.x + old.w - pw() };
    ed(a).panel = panel_rect_at(a, x, old.y);
    compose(a, old);
    draw_panel(a);
    let p = ed(a).panel;
    app_invalidate(a, rect_union(old, p), RefreshMode::Ui);
    persist_panel(a);
}

// ---- the tool picker's taps ----

// The tool picker closed (a pen picked, or a tap outside it), and its Pin mode with it.
fn close_tools(a: &mut App) {
    let s = ed(a);
    s.tools_open = false;
    s.pin_mode = false;
    let r = s.tools_rect;
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Ui);
    repaint_panel(a, RefreshMode::Ui);
}

// The panel's buttons changed (a pen pinned or unpinned): it is fitted again where it is, and drawn
// with the picker beside it; where they no longer reach, the page shows again.
fn panel_changed(a: &mut App) {
    let s = ed_ref(a);
    let old = rect_union(s.panel, if s.tools_open { s.tools_rect } else { RECT_EMPTY });
    fit_panel_to(&a.settings, a.h);
    let p = ed(a).panel;
    ed(a).panel = panel_rect_at(a, p.x, p.y);
    compose(a, old); // (draws the panel and the picker whole, where they are now)
    let s = ed_ref(a);
    let now = rect_union(s.panel, if s.tools_open { s.tools_rect } else { RECT_EMPTY });
    app_invalidate(a, rect_union(old, now), RefreshMode::Ui);
}

// Pin mode: a pen pinned to the panel (after the others), or unpinned. Unpinned while in use, it
// goes to the tool button, so the panel still shows the pen that draws.
fn toggle_pin(a: &mut App, t: ToolKind) {
    let st = &mut a.settings;
    let was = settings_pinned(st, t);
    if !settings_toggle_pin(st, t) {
        let mut msg = String::new();
        // TRANSLATORS: a toast when the tool picker's Pin mode is asked for one more pen than the
        // panel holds: %d is how many it holds, 4 in this version. About 1700 px at 44 px in bold,
        // or two lines.
        csnprintf!(&mut msg, 200, ng_("The panel holds up to %d pen. Unpin one first.",
                                      "The panel holds up to %d pens. Unpin one first.", PANEL_PINS_MAX as i64),
                   PANEL_PINS_MAX as i32);
        app_toast(a, &msg, 2500);
        return;
    }
    if was && st.current_tool == t {
        st.draw_tool = t;
    }
    settings_save(st);
    panel_changed(a);
}

// A tap on the tool picker (the press is S->tools_press): a pen picked, which closes it, or in
// Pin mode pinned or unpinned; Pin or Done, which turn Pin mode on or off; an ink chosen. A tap
// outside it closes it; one inside, between its buttons, does nothing.
fn tools_tap(a: &mut App) {
    let s = ed(a);
    let p = s.tools_press;
    let Some(h) = hits_find(&s.tools_hits, p.x0, p.y0).copied() else {
        if !rect_contains(s.tools_rect, p.x0 as i32, p.y0 as i32) {
            close_tools(a);
        }
        return;
    };
    if !press_is_tap(&p, h.r) {
        return;
    }
    match h.id {
        P_PIN_MODE => {
            let s = ed(a);
            s.pin_mode = !s.pin_mode;
            draw_tools(a);
            let r = ed(a).tools_rect;
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_PICK_INK => {
            let st = &mut a.settings;
            st.white = K_INKS[clampi(h.arg, 0, N_INKS - 1) as usize].white;
            settings_save(st);
            draw_panel(a); // the pen in use shows whether it draws white
            draw_tools(a); // (over the panel where it covers it)
            let s = ed(a);
            let r = rect_union(s.panel, s.tools_rect);
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_PICK_TOOL => {
            let t = ToolKind(h.arg);
            if ed(a).pin_mode {
                toggle_pin(a, t); // (the panel and the picker drawn again, or a toast at the cap)
                return;
            }
            // A pinned pen picked is the one in use; the tool button keeps the pen it shows.
            let st = &mut a.settings;
            st.current_tool = t;
            if !settings_pinned(st, t) {
                st.draw_tool = t;
            }
            settings_save(st);
            close_tools(a);
        }
        _ => {}
    }
}

// The tool picker opens beside the panel, out of Pin mode.
fn open_tools(a: &mut App) {
    let s = ed(a);
    s.tools_open = true;
    s.pin_mode = false;
    s.tools_y = s.panel.y;
    draw_tools(a);
    let r = ed(a).tools_rect;
    app_invalidate(a, r, RefreshMode::Ui);
}

fn panel_act(a: &mut App, h: Hit) {
    match h.id {
        P_TOOLPICK => {
            // From any other tool, including the fixed eraser, the tool button's pen is one tap away; a tap
            // while that pen is in use opens the picker. So does every tap on the grid (its pen pinned).
            let cur = a.settings.current_tool;
            let dt = button_tool(&a.settings);
            if cur != dt && !settings_pinned(&a.settings, dt) {
                a.settings.current_tool = dt;
                settings_save(&a.settings);
                repaint_panel(a, RefreshMode::Ui);
                return;
            }
            open_tools(a);
        }
        P_TOOL => {
            let t = ToolKind(h.arg);
            let cur = a.settings.current_tool;
            if t == cur && t != TOOL_ERASER {
                return; // the pen in use already
            }
            if t == cur {
                // Tapping the active eraser again goes back to the pen used just before it (a pinned
                // one, or the tool button's), or to the tool button's if that one is off the panel now.
                a.settings.current_tool = eraser_back(&a.settings);
            } else {
                if t == TOOL_ERASER {
                    a.settings.before_eraser = cur.0;
                }
                a.settings.current_tool = t;
            }
            settings_save(&a.settings);
            repaint_panel(a, RefreshMode::Ui);
        }
        P_UNDO => undo_redo(a, true),
        P_REDO => undo_redo(a, false),
        P_PREV => {
            let i = ed(a).index;
            go_to_page(a, i - 1);
        }
        P_NEXT => {
            // On the last page (its button a +), a new page on this page's paper (store_new_page_paper).
            let s = ed(a);
            if s.index >= s.nb.npages() - 1 {
                if !save_page(a) {
                    return;
                }
                let s = ed(a);
                let paper = s.open_page().map_or(s.nb.paper, |p| p.paper);
                let n = s.nb.npages();
                if store_add_page(&mut s.nb, n, paper) != 0 {
                    app_toast(a, tr("Could not add a page"), 3000);
                    return;
                }
                s.changed = true;
            }
            let i = ed(a).index;
            go_to_page(a, i + 1);
        }
        P_PAPER => {
            if refuse_readonly(a) {
                return;
            }
            ed(a).paper_open = true;
            draw_paper_sheet(a);
            let r = ed(a).paper_rect;
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_CLEAR => clear_page(a),
        P_PAGES => pages_show(a),
        P_DISPLAY => {
            with_modal(a, |s| &mut s.dm, dispmenu_open);
            let r = with_modal(a, |s| &mut s.dm, dispmenu_draw);
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_PENSET => {
            let t = a.settings.current_tool;
            pensettings_open(a, t);
        }
        P_LIBRARY => {
            app_toast(a, tr("Saving..."), 8000);
            app_flush(a);
            if !save_page(a) {
                return; // stay with the unsaved page rather than drop it
            }
            close_notebook(a);
            app_switch(a, ScreenId::Library);
            app_toast(a, tr("Saved"), 1200);
        }
        P_COLLAPSE => set_collapsed(a, true),
        P_EXPAND => set_collapsed(a, false),
        _ => {}
    }
}

// The menu in front of the page, as the action of its button: the tool picker, Paper (the paper
// sheet, its Templates sheet and Custom music), Display, or Pages or Index (the Pages view on that
// tab); 0 with none.
fn front_menu(s: &EditorState) -> i32 {
    if s.pv.open {
        return if s.pv.index_tab { SHORTCUT_INDEX } else { SHORTCUT_PAGES };
    }
    if s.tools_open {
        return SHORTCUT_TOOL_PICKER;
    }
    if s.mb.open || s.ts.open || s.paper_open {
        return SHORTCUT_PAPER;
    }
    if s.dm.open {
        return SHORTCUT_DISPLAY;
    }
    0
}

// A menu's button (Pen feel and the Library leave the editor for their screens), or Close menu.
fn menu_action(action: i32) -> bool {
    action == SHORTCUT_PAGES
        || action == SHORTCUT_INDEX
        || action == SHORTCUT_TOOL_PICKER
        || action == SHORTCUT_PAPER
        || action == SHORTCUT_DISPLAY
        || action == SHORTCUT_PEN_FEEL
        || action == SHORTCUT_LIBRARY
        || action == SHORTCUT_CLOSE_MENU
}

// The menu in front closes, as its Back or a tap outside it closes it (a link being placed is taken
// back); with draw false the screen is about to be drawn whole anyway.
fn close_front(a: &mut App, draw: bool) {
    let s = ed(a);
    if s.pv.open {
        if s.pv.picking {
            link_cancel(a);
        } else if draw {
            pages_hide(a, -1);
        } else {
            pagesview_close(&mut s.pv);
        }
        return;
    }
    let mut r = if s.tools_open { rect_union(s.tools_rect, s.panel) } else { RECT_EMPTY };
    if s.paper_open {
        r = rect_union(r, s.paper_rect);
    }
    if s.ts.open {
        r = rect_union(r, s.ts.rect);
    }
    if s.dm.open {
        r = rect_union(r, s.dm.rect);
    }
    if s.mb.open {
        r = rect_union(r, s.mb.rect);
    }
    close_sheets(s);
    if !draw {
        return;
    }
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Ui);
}

// A menu's button with nothing in front: its menu opens, as the panel's button for it opens it.
fn open_menu(a: &mut App, action: i32) {
    match action {
        SHORTCUT_PAGES | SHORTCUT_INDEX => {
            let s = ed(a);
            s.pv.index_tab = action == SHORTCUT_INDEX;
            s.pv.tab_initialized = true;
            pages_show(a);
        }
        SHORTCUT_TOOL_PICKER => open_tools(a),
        SHORTCUT_PAPER => panel_act(a, Hit { id: P_PAPER, ..Default::default() }),
        SHORTCUT_DISPLAY => panel_act(a, Hit { id: P_DISPLAY, ..Default::default() }),
        SHORTCUT_PEN_FEEL => panel_act(a, Hit { id: P_PENSET, ..Default::default() }),
        SHORTCUT_LIBRARY => panel_act(a, Hit { id: P_LIBRARY, ..Default::default() }),
        // Nothing in front: a link waiting for its box is not made.
        SHORTCUT_CLOSE_MENU if ed(a).link_mode => link_stop(a),
        _ => {}
    }
}

/// Erase while held's button let go (false)
pub fn editor_hold_eraser(a: &mut App, held: bool) {
    if let Some(s) = a.ed.as_deref_mut() {
        s.hold_eraser = held; // (a stroke under way keeps what it does)
    }
}

/// optional key/button action, idle editor only
// Optional external buttons act only in the idle editor, never through an open sheet, but for
// these: Erase while held counts from its press, whenever it comes (from the next stroke); a menu's
// button closes its own menu, or the one in front and opens its own (never over the Pages view's
// keyboard: typing is never thrown away), and Close menu closes any; the picker's next and previous
// move the open tool picker's choice. Next and previous tool go through the panel's pens alone, so
// they leave the picker as it is: pressed again and again they never move the highlight from the
// panel to the picker and back.
pub fn editor_shortcut(a: &mut App, action: i32) -> bool {
    let asleep = a.asleep;
    let Some(s) = a.ed.as_deref_mut() else { return false };
    if !s.open || asleep || action == 0 {
        return false;
    }
    if action == SHORTCUT_HOLD_ERASER {
        s.hold_eraser = true;
        return true;
    }
    if s.owner != Owner::None || s.pinching {
        return false;
    }
    let mut action = action;
    let front = front_menu(s);
    if front != 0 && menu_action(action) {
        if s.pv.open && s.pv.kb.open {
            return false;
        }
        if (action == SHORTCUT_PAGES || action == SHORTCUT_INDEX) && action != front && s.pv.open {
            s.pv.index_tab = action == SHORTCUT_INDEX; // the other tab, as a tap on it
            if s.pv.index_tab {
                pagesview_untick(&mut s.pv);
            }
            compose_all(a, RefreshMode::Full);
            return true;
        }
        let leaves = action == SHORTCUT_PEN_FEEL || action == SHORTCUT_LIBRARY;
        close_front(a, !leaves);
        if action != front && action != SHORTCUT_CLOSE_MENU {
            open_menu(a, action);
        }
        // The Library stays with a page it could not save: the page again, where the menu was.
        if leaves && a.scr == ScreenId::Editor {
            compose_all(a, RefreshMode::Full);
        }
        return true;
    }
    let picker_move = action == SHORTCUT_PICKER_NEXT || action == SHORTCUT_PICKER_PREV;
    if picker_move && s.tools_open && !s.pv.open {
        // In the open picker the step is its choice, as a tap on the pen is but for closing it.
        if s.pin_mode {
            return false;
        }
        let st = &mut a.settings;
        let t = picker_step(st, action == SHORTCUT_PICKER_NEXT);
        st.current_tool = t;
        if !settings_pinned(st, t) {
            st.draw_tool = t;
        }
        settings_save(st);
        draw_panel(a);
        draw_tools(a); // (over the panel where it covers it)
        let s = ed(a);
        let r = rect_union(s.panel, s.tools_rect);
        app_invalidate(a, r, RefreshMode::Ui);
        return true;
    }
    if sheet_open(s) {
        return false;
    }
    if menu_action(action) {
        open_menu(a, action);
        return true;
    }
    // With the picker closed its next and previous switch through its nine pens, and next and
    // previous tool through the panel's (or the nine, with fewer than two there): either picks as
    // its pen's button would, a pinned pen lighting its pin, another going to the tool button.
    if action == SHORTCUT_NEXT_TOOL || action == SHORTCUT_PREV_TOOL {
        action = SHORTCUT_TOOL_BASE + panel_step(&a.settings, action == SHORTCUT_NEXT_TOOL).0;
    } else if picker_move {
        action = SHORTCUT_TOOL_BASE + picker_step(&a.settings, action == SHORTCUT_PICKER_NEXT).0;
    }
    if action >= SHORTCUT_TOOL_BASE && action < SHORTCUT_TOOL_BASE + TOOL_COUNT as i32 {
        let t = ToolKind(action - SHORTCUT_TOOL_BASE);
        if t != TOOL_ERASER && !settings_pinned(&a.settings, t) {
            a.settings.draw_tool = t;
        }
        panel_act(a, Hit { id: P_TOOL, arg: t.0, ..Default::default() });
    } else if action == SHORTCUT_UNDO || action == SHORTCUT_REDO {
        panel_act(a, Hit { id: if action == SHORTCUT_UNDO { P_UNDO } else { P_REDO }, ..Default::default() });
    } else if action == SHORTCUT_PREV_PAGE || action == SHORTCUT_NEXT_PAGE {
        // The panel's arrows: next page on the last page adds one, as its + does.
        panel_act(a, Hit { id: if action == SHORTCUT_NEXT_PAGE { P_NEXT } else { P_PREV }, ..Default::default() });
    } else if action == SHORTCUT_FIRST_PAGE || action == SHORTCUT_LAST_PAGE {
        let s = ed(a);
        let to = if action == SHORTCUT_FIRST_PAGE { 0 } else { s.nb.npages() - 1 };
        if to != s.index {
            go_to_page(a, to);
        }
    } else if action == SHORTCUT_ADD_LINK {
        // The Index's Add a link from the page; pressed again while the link waits for its box, it
        // is not made, as a menu's own button closes it.
        if ed(a).link_mode {
            link_stop(a);
        } else {
            link_start(a);
        }
    } else if action == SHORTCUT_NAME_PAGE {
        pages_name(a);
    } else if action == SHORTCUT_NEW_PAGE {
        let i = ed(a).index;
        pages_new(a, i + 1); // the Pages view's New page, after the open one
    } else if action == SHORTCUT_CLEAR_PAGE {
        clear_page(a); // the panel's broom: no question, one undo step
    } else if action == SHORTCUT_FIT_PAGE {
        if !zoomed(a) {
            return true;
        }
        view_reset(a);
        compose_all(a, RefreshMode::Full);
    } else if action == SHORTCUT_FOLD_PANEL {
        let c = ed(a).collapsed;
        set_collapsed(a, !c);
    } else if action == SHORTCUT_ERASER_MODE {
        a.settings.eraser_mode = if a.settings.eraser_mode == 1 { 0 } else { 1 };
        settings_save(&a.settings);
        let mut msg = String::new();
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a controller's button
        // changed the eraser to the one named, as Pen feel names it: Whole strokes or Precise.
        csnprintf!(&mut msg, 160, tr("Eraser: %s"),
                   if a.settings.eraser_mode == 1 { c_("eraser", "Precise") } else { c_("eraser", "Whole strokes") });
        app_toast(a, &msg, 1500);
    } else if action == SHORTCUT_WHITE {
        a.settings.white = !a.settings.white;
        settings_save(&a.settings);
        repaint_panel(a, RefreshMode::Ui);
    } else {
        return false;
    }
    true
}

fn apply_paper(a: &mut App, k: PaperKind) {
    if refuse_readonly(a) {
        return;
    }
    let s = ed(a);
    let old = s.open_page().unwrap().paper;
    if let Some(hi) = s.cur_hist() {
        history_push_paper(&mut s.hist[hi].h, old, k);
    }
    s.open_page_mut().unwrap().paper = k;
    render_page_full(a);
    compose_all(a, RefreshMode::Full);
    mark_changed(ed(a));
}

// A template picked on the Templates sheet becomes this page's paper, as any paper does (one undo
// step): template_install gives its paper value at once, and the page draws white until the
// background job has made its picture for this notebook (editor_tick then draws it). One that
// cannot be installed changes nothing.
fn apply_template(a: &mut App, path: &str) {
    let mut k = PaperKind::default();
    let mut why: &'static str = "";
    let s = ed(a);
    if template_install(&s.nb, path, &mut k, &mut why) != 0 {
        qlog!("editor: %s: a template could not be used: %s", s.nb.id.as_str(), why);
        let mut msg = String::new();
        // TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines): a template picked
        // on the Templates sheet could not become the page's paper (or the new notebook's); %s is
        // the log file that says why.
        csnprintf!(&mut msg, 256, tr("Could not use that template (see %s)."), "grimoire.log");
        app_toast(a, &msg, 4000);
        return;
    }
    if s.open_page().unwrap().paper != k {
        apply_paper(a, k);
    }
}

// A photo picked on the Photos sheet becomes the open page's paper, as a template does (templates.h:
// a document page on its own picture, made upright in the background; Undo puts the old paper back).
fn apply_photo(a: &mut App, path: &str) {
    let mut k = PaperKind::default();
    let mut why: &'static str = "";
    let s = ed(a);
    if photo_install(&s.nb, path, &mut k, &mut why) != 0 {
        qlog!("editor: %s: a photo could not be used: %s", s.nb.id.as_str(), why);
        let mut msg = String::new();
        // TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines): a photo picked on the
        // Photos sheet could not become the page's paper; %s is the log file that says why.
        csnprintf!(&mut msg, 256, tr("Could not use that photo (see %s)."), "grimoire.log");
        app_toast(a, &msg, 4000);
        return;
    }
    if s.open_page().unwrap().paper != k {
        apply_paper(a, k);
    }
}

fn modal_act(a: &mut App, h: Hit) {
    let s = ed(a);
    let sheet = s.paper_rect;
    s.paper_open = false;
    let paper = s.open_page().unwrap().paper;
    if h.id == P_PICK_PAPER && paper != PaperKind(h.arg as u32) {
        apply_paper(a, PaperKind(h.arg as u32));
        return;
    }
    if h.id == P_PICK_MORE || h.id == P_PICK_PHOTOS {
        compose(a, sheet); // (the Templates sheet takes the Paper sheet's place)
        // (the sheet keeps a copy of the notebook it is given: tplsheet_open)
        let nb = ed(a).nb.clone();
        if h.id == P_PICK_PHOTOS {
            with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_open_photos(ts, a, &nb, paper));
        } else {
            with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_open(ts, a, Some(&nb), paper, None));
        }
        let r = with_modal(a, |s| &mut s.ts, tplsheet_draw);
        app_invalidate(a, rect_union(sheet, r), RefreshMode::Ui);
        return;
    }
    if h.id == P_PICK_CUSTOM {
        compose(a, sheet);
        let (pgw, pgh) = (ed(a).pgw, ed(a).pgh);
        with_modal(a, |s| &mut s.mb, |mb, a| musicbuild_open_page(mb, a, paper, pgw, pgh));
        let r = rect_union(sheet, with_modal(a, |s| &mut s.mb, musicbuild_draw));
        app_invalidate(a, r, RefreshMode::Ui);
        return;
    }
    compose(a, sheet);
    app_invalidate(a, sheet, RefreshMode::Ui);
}

fn ui_pointer(a: &mut App, ph: i32, ev: &InEvent, modal: bool) {
    let s = ed(a);
    if ph == 0 {
        press_begin(&mut s.press, ev.x, ev.y, ev.t_ms);
        let hl = if modal { &s.modal_hits } else { &s.hits };
        let h = hits_find(hl, ev.x, ev.y).copied();
        s.has_pressed = h.is_some();
        s.dragging = false;
        if let Some(h) = h {
            s.pressed = h;
            s.grab_dx = ev.x - s.panel.x as f32;
            s.grab_dy = ev.y - s.panel.y as f32;
            s.drag_start_rect = s.panel;
            // Drag handles do not invert; buttons do.
            if h.id != P_GRIP && h.id != P_EXPAND {
                app_press_feedback(a, h.r);
            }
        }
        return;
    }
    if !s.has_pressed {
        return;
    }
    let draggable = !modal && (s.pressed.id == P_GRIP || s.pressed.id == P_EXPAND);
    if ph == 1 {
        press_move(&mut s.press, ev.x, ev.y);
        if draggable && s.press.moved {
            s.dragging = true;
            if ev.t_ms.wrapping_sub(s.last_drag_paint) > 70 {
                s.last_drag_paint = ev.t_ms;
                let (x, y) = ((ev.x - s.grab_dx) as i32, (ev.y - s.grab_dy) as i32);
                move_panel_to(a, x, y, RefreshMode::Fast);
            }
        }
        return;
    }
    // Release.
    press_move(&mut s.press, ev.x, ev.y);
    s.has_pressed = false;
    if s.dragging {
        s.dragging = false;
        let (x, y) = ((ev.x - s.grab_dx) as i32, (ev.y - s.grab_dy) as i32);
        move_panel_to(a, x, y, RefreshMode::Ui);
        // Also clean up wherever the panel passed through.
        let s = ed(a);
        let trail = rect_union(s.drag_start_rect, s.panel);
        compose(a, trail);
        app_invalidate(a, trail, RefreshMode::Ui);
        persist_panel(a);
        return;
    }
    let h = s.pressed;
    let p = s.press;
    if h.id != P_GRIP && h.id != P_EXPAND {
        canvas_invert(&mut a.screen, h.r);
        app_invalidate(a, h.r, RefreshMode::Ui);
    }
    if !press_is_tap(&p, h.r) && !rect_contains(h.r, ev.x as i32, ev.y as i32) {
        return;
    }
    if modal {
        modal_act(a, h);
    } else {
        panel_act(a, h);
    }
}

// ---- screen interface ----

pub fn editor_init(a: &mut App) {
    // (calloc: every field zero, as EditorState::default())
    a.ed = Some(Box::<EditorState>::default());
    let s = ed(a);
    nav_init(&mut s.nav);
    store_page_size(&mut s.pgw, &mut s.pgh);
    s.zoom = 1.0;
    s.zoom_default = 1.0;
    s.zoom_min = 1.0;
    s.tool_button_shows = TB_NONE; // (no panel drawn yet)
    // Erased ink an earlier run left in page files (it left with the notebook open, or a compaction
    // was stopped): taken out now, in the background, unless the notebook is opened first.
    compact_queue_listed(a.dpi);
}

pub fn editor_free(a: &mut App) {
    if a.ed.is_none() {
        return;
    }
    close_notebook(a);
    let s = ed(a);
    if let Some(cur) = s.cur.take() {
        stroke_free(cur);
    }
    s.snap = Vec::new();
    for st in std::mem::take(&mut s.removed) {
        stroke_free(st);
    }
    pagesview_free(&mut s.pv);
    nav_free(&mut s.nav);
    canvas_free(&mut s.page_canvas);
    for k in 0..PAPER_COUNT as usize {
        canvas_free(&mut s.paper_prev[k]);
    }
    canvas_free(&mut s.custom_prev);
    canvas_free(&mut s.image_prev);
    s.paper = SharedPaper::default();
    s.noted = Vec::new();
    musicbuild_free(&mut s.mb);
    tplsheet_close(&mut s.ts);
    a.ed = None;
}

// Every sheet over the page closed: the paper sheet, Display, Custom music, the tool picker (and its
// Pin mode). A sheet closed here stays open through a turn or a wake (editor_enter), laid out again
// by compose for the new size, as each of these lays itself out where it is drawn. (The Pages view
// closes with its notebook, and stays open through those too.)
fn close_sheets(s: &mut EditorState) {
    s.paper_open = false;
    s.dm.open = false;
    s.mb.open = false;
    s.tools_open = false;
    s.pin_mode = false;
    tplsheet_close(&mut s.ts);
}

pub fn editor_enter(a: &mut App) {
    let s = ed(a);
    if s.page_canvas.px.is_empty() && canvas_init(&mut s.page_canvas, s.pgw, s.pgh) != 0 {
        return;
    }
    if s.open && s.page.is_some() && s.canvas_page != s.open_page_ref() {
        render_page_full(a);
    }
    let s = ed_ref(a);
    if s.view_w != a.w || s.view_h != a.h {
        view_reset(a); // new notebook or turned
    }
    ed(a).pinching = false;
    fit_panel_to(&a.settings, a.h);
    ed(a).collapsed = a.settings.panel_collapsed;
    let mut x = a.settings.panel_x;
    let mut y = a.settings.panel_y;
    if x < 0 || y < 0 {
        x = a.w - (if ed(a).collapsed { box_() } else { pw() }) - ui_px(28.0);
        y = ui_px(220.0);
    }
    let p = panel_rect_at(a, x, y);
    let s = ed(a);
    s.panel = p;
    s.tools_y = s.panel.y;
    s.owner = Owner::None;
    // The pen in use always has its button on the panel. One that has none (a pin a version without
    // pins dropped, or one taken out of settings.ini by hand) goes to the tool button.
    let st = &mut a.settings;
    if (st.current_tool.0 as u32) < TOOL_COUNT as u32
        && st.current_tool != TOOL_ERASER
        && st.current_tool != button_tool(st)
        && !settings_pinned(st, st.current_tool)
    {
        st.draw_tool = st.current_tool;
        settings_save(st);
    }
    // A turn, or the redraw after waking (a->relayout), lays out the same screen again: the sheet
    // that was open stays open. Coming in from another screen, none is.
    if !a.relayout {
        close_sheets(ed(a));
    }
    compose(a, Rect { x: 0, y: 0, w: a.w, h: a.h });
}

/// a stroke / drag / pinch is in progress
pub fn editor_busy(a: &App) -> bool {
    a.ed.as_deref().is_some_and(|s| s.owner != Owner::None || s.pinching)
}

/// the open page (paper + ink), or None
pub fn editor_page_canvas(a: &App) -> Option<&Canvas> {
    match a.ed.as_deref() {
        Some(s) if s.open && !s.page_canvas.px.is_empty() => Some(&s.page_canvas),
        _ => None,
    }
}

/// saves the open page as the sleep screen
pub fn editor_pin_sleep_page(a: &mut App) -> bool {
    editor_page_canvas(a).is_some_and(|c| store_save_sleep_image(c) == 0)
}

// Two fingers: zoom around the midpoint and pan with it.
fn pinch(a: &mut App, ev: &InEvent) {
    let mx = (ev.x + ev.x2) * 0.5;
    let my = (ev.y + ev.y2) * 0.5;
    let d = (ev.x2 - ev.x).hypot(ev.y2 - ev.y);
    let s = ed(a);
    if ev.type_ == InType::PinchBegin {
        if s.owner == Owner::Link {
            s.owner = Owner::None; // the first finger turns into the pinch
        }
        if s.owner == Owner::Panel && !s.owner_is_pen {
            // The first finger had pressed a panel button: that press is cancelled.
            if s.has_pressed && s.pressed.id != P_GRIP && s.pressed.id != P_EXPAND {
                let r = s.pressed.r;
                canvas_invert(&mut a.screen, r);
                app_invalidate(a, r, RefreshMode::Ui);
            }
            let s = ed(a);
            s.has_pressed = false;
            s.dragging = false;
            s.owner = Owner::None;
        }
        let pinch_zoom = a.settings.pinch_zoom;
        let s = ed(a);
        if s.owner != Owner::None || s.paper_open || s.mb.open || s.ts.open || !pinch_zoom {
            return;
        }
        s.pinching = true;
        s.pinch_committed = false;
        s.pinch_mx0 = mx;
        s.pinch_my0 = my;
        s.pinch_d0 = if d > 10.0 { d } else { 10.0 };
        s.pinch_z0 = s.zoom;
        let (mut px, mut py) = (0.0, 0.0);
        to_page(s, mx, my, &mut px, &mut py);
        s.pinch_px = px;
        s.pinch_py = py;
        s.last_pinch_paint = 0;
        return;
    }
    if !s.pinching {
        return;
    }
    if ev.type_ == InType::PinchMove {
        // Nothing happens until the fingers clearly spread/close or travel: resting fingers
        // (or a hand) must not zoom or flash the screen.
        if !s.pinch_committed {
            let spread = (d / s.pinch_d0 - 1.0).abs();
            let travel = (mx - s.pinch_mx0).hypot(my - s.pinch_my0);
            if spread < 0.10 && travel < ui_px(70.0) as f32 {
                return;
            }
            s.pinch_committed = true;
            // Re-anchor at the commit point so the view does not jump by the dead zone.
            s.pinch_d0 = if d > 10.0 { d } else { 10.0 };
            s.pinch_z0 = s.zoom;
            let (mut px, mut py) = (0.0, 0.0);
            to_page(s, mx, my, &mut px, &mut py);
            s.pinch_px = px;
            s.pinch_py = py;
        }
        s.zoom = s.pinch_z0 * d / s.pinch_d0;
        s.zoom = clampf(s.zoom, s.zoom_min, s.zoom_default * 4.0);
        s.ox = s.pinch_px - mx / s.zoom; // keep the pinched spot under the fingers
        s.oy = s.pinch_py - my / s.zoom;
        view_clamp(a);
        let s = ed(a);
        if ev.t_ms.wrapping_sub(s.last_pinch_paint) > 180 {
            s.last_pinch_paint = ev.t_ms;
            compose_all(a, RefreshMode::Fast);
        }
        return;
    }
    s.pinching = false; // IN_PINCH_END
    // Settle without a flash, and only if the view actually changed.
    if s.pinch_committed {
        compose_all(a, RefreshMode::Ui);
    }
}

pub fn editor_leave(a: &mut App) {
    // Leaving for pen settings / calibration keeps the notebook open; just persist.
    if ed(a).owner == Owner::Draw {
        draw_end(a);
    }
    if ed(a).owner == Owner::Erase {
        erase_end(a);
    }
    save_page(a);
    write_thumb_if_needed(a);
}

/// flush pending page/info saves; false if they failed
pub fn editor_suspend(a: &mut App) -> bool {
    match a.ed.as_deref() {
        Some(s) if s.open => {}
        _ => return true,
    }
    // Native unlock will consume the pen/finger releases. Commit live ink before yielding input.
    if ed(a).owner == Owner::Draw {
        draw_end(a);
    }
    if ed(a).owner == Owner::Erase {
        erase_end(a);
    }
    let s = ed(a);
    s.owner = Owner::None;
    s.has_pressed = false;
    s.dragging = false;
    s.pinching = false;
    s.btn_down = false;
    s.nclicks = 0;
    s.hold_eraser = false; // (its release may never come: the Kindle sleeps, native PIN input takes the keys)
    save_page(a)
}

/// Whether the editor has a control at (x, y) for a finger: a sheet, a menu or the Pages view that is
/// up (each takes every tap), the panel, or a link on the page. The page itself is not one. The
/// clock and battery corner leaves a tap on a control to it.
pub fn editor_control_at(a: &App, x: f32, y: f32) -> bool {
    let s = ed_ref(a);
    if !s.open || s.pv.open || s.tools_open || s.mb.open || s.dm.open || s.ts.open || s.paper_open {
        return true;
    }
    if rect_contains(s.panel, x as i32, y as i32) {
        return true;
    }
    let mut l = NavLink::default();
    link_under(a, x, y, &mut l)
}

pub fn editor_event(a: &mut App, ev: &InEvent) {
    let s = ed(a);
    if !s.open {
        return;
    }
    if ev.type_ == InType::PenLeave {
        s.pen_near = false;
        s.pen_left = ev.t_ms;
    } else if ev.type_ == InType::PenDown || ev.type_ == InType::PenMove || ev.type_ == InType::PenHover {
        s.pen_near = true;
    }
    if ev.type_ == InType::PenButton {
        pen_button(a, ev);
        return;
    }
    if ev.type_ == InType::PinchBegin || ev.type_ == InType::PinchMove || ev.type_ == InType::PinchEnd {
        if ev.type_ == InType::PinchBegin {
            live_reset();
        }
        if !s.pv.open {
            pinch(a, ev);
        }
        return;
    }
    let pen = ev.type_ == InType::PenDown || ev.type_ == InType::PenMove || ev.type_ == InType::PenUp || ev.type_ == InType::PenLeave;
    // A pen leaving proximity mid-gesture ends it.
    let type_ = if ev.type_ == InType::PenLeave { InType::PenUp } else { ev.type_ };
    let ph = if type_ == InType::PenUp { 2 } else { app_pointer_phase(ev) };
    if ph < 0 {
        return;
    }
    if ph == 0 {
        if pen && s.btn_down {
            s.btn_used_for_ink = true; // (pen_button: no click, no hold)
        }
        if s.owner != Owner::None || s.pinching {
            return; // one gesture at a time
        }
        s.owner_is_pen = pen;
        if s.pv.open {
            s.owner = Owner::Pages;
            pages_event(a, 0, ev);
            return;
        }
        if s.tools_open {
            s.owner = Owner::Tools;
            press_begin(&mut s.tools_press, ev.x, ev.y, ev.t_ms);
            return;
        }
        if s.mb.open {
            s.owner = Owner::Build;
            let mut d = RECT_EMPTY;
            with_modal(a, |s| &mut s.mb, |mb, a| musicbuild_pointer(mb, a, 0, ev.x, ev.y, ev.t_ms, &mut d));
            return;
        }
        if s.dm.open {
            s.owner = Owner::Display;
            let mut d = RECT_EMPTY;
            with_modal(a, |s| &mut s.dm, |dm, a| dispmenu_pointer(dm, a, 0, ev.x, ev.y, ev.t_ms, &mut d));
            return;
        }
        if s.ts.open {
            s.owner = Owner::Templates;
            let mut d = RECT_EMPTY;
            with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_pointer(ts, a, 0, ev.x, ev.y, ev.t_ms, &mut d));
            return;
        }
        if s.paper_open {
            if !rect_contains(s.paper_rect, ev.x as i32, ev.y as i32) {
                // Tapping outside the sheet dismisses it.
                s.paper_open = false;
                let r = s.paper_rect;
                compose(a, r);
                app_invalidate(a, r, RefreshMode::Ui);
                let s = ed(a);
                s.owner = Owner::Modal; // swallow the rest of this gesture
                s.has_pressed = false;
                return;
            }
            s.owner = Owner::Modal;
            ui_pointer(a, 0, ev, true);
        } else if rect_contains(s.panel, ev.x as i32, ev.y as i32) {
            s.owner = Owner::Panel;
            // A stroke just ended: Undo/Clear may have become available, but the panel (and its
            // hit list) is only rebuilt at the settle. Settle now so the tap finds the live buttons.
            if s.panel_stale {
                settle_now(a);
            }
            ui_pointer(a, 0, ev, false);
        } else if pen {
            draw_begin(a, ev);
            if ed(a).owner == Owner::Draw {
                draw_sample(a, ev);
            }
        } else {
            // A finger on the page: a tap follows a link there, a hold opens it (editor_tick).
            s.owner = Owner::Link;
            press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
            s.link_down_at = now_ms();
            s.link_held = false;
        }
        return;
    }
    if s.owner == Owner::None || pen != s.owner_is_pen {
        return;
    }
    match s.owner {
        Owner::Draw => {
            if ph == 1 {
                draw_sample(a, ev);
            } else {
                draw_end(a);
            }
        }
        Owner::Erase => {
            erase_segment(a, ev.x, ev.y, ev.pressure);
            if ph == 2 {
                erase_end(a);
            }
        }
        Owner::Pages => {
            pages_event(a, ph, ev);
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
        }
        Owner::Link => {
            press_move(&mut s.link_press, ev.x, ev.y);
            if ph != 2 {
                return;
            }
            s.owner = Owner::None;
            if !s.link_held && !s.link_press.moved {
                let (x0, y0) = (s.link_press.x0, s.link_press.y0);
                link_follow(a, x0, y0);
            }
        }
        Owner::Tools => {
            press_move(&mut s.tools_press, ev.x, ev.y);
            if ph != 2 {
                return;
            }
            s.owner = Owner::None;
            tools_tap(a);
        }
        Owner::Build => {
            let mut d = RECT_EMPTY;
            let r = with_modal(a, |s| &mut s.mb, |mb, a| musicbuild_pointer(mb, a, ph, ev.x, ev.y, ev.t_ms, &mut d));
            if r == MbResult::Done {
                compose(a, d);
                let k = musicbuild_result(&ed(a).mb);
                if ed(a).open_page().unwrap().paper != k {
                    apply_paper(a, k);
                } else {
                    app_invalidate(a, d, RefreshMode::Ui);
                }
            } else if r == MbResult::Cancel {
                compose(a, d);
                app_invalidate(a, d, RefreshMode::Ui);
            } else if !rect_is_empty(d) {
                app_invalidate(a, d, RefreshMode::Ui);
            }
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
        }
        Owner::Templates => {
            let mut d = RECT_EMPTY;
            let r = with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_pointer(ts, a, ph, ev.x, ev.y, ev.t_ms, &mut d));
            if r == TsResult::Picked {
                let s = ed(a);
                let mut path = String::new();
                strlcpy_q(&mut path, tplsheet_path(&s.ts), TEMPLATE_PATH_MAX);
                let photo = s.ts.photos;
                tplsheet_close(&mut s.ts);
                compose(a, d);
                app_invalidate(a, d, RefreshMode::Ui);
                if !path.is_empty() && photo {
                    apply_photo(a, &path);
                } else if !path.is_empty() {
                    apply_template(a, &path);
                }
            } else if r == TsResult::Back {
                // Back to the Paper sheet, in the same place.
                compose(a, d);
                ed(a).paper_open = true;
                draw_paper_sheet(a);
                let p = ed(a).paper_rect;
                app_invalidate(a, rect_union(d, p), RefreshMode::Ui);
            } else if r == TsResult::Cancel {
                compose(a, d);
                app_invalidate(a, d, RefreshMode::Ui);
            } else if !rect_is_empty(d) {
                app_invalidate(a, d, RefreshMode::Ui);
            }
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
        }
        Owner::Display => {
            let mut d = RECT_EMPTY;
            let r = with_modal(a, |s| &mut s.dm, |dm, a| dispmenu_pointer(dm, a, ph, ev.x, ev.y, ev.t_ms, &mut d));
            if r == DmResult::Inverted {
                compose_all(a, RefreshMode::Full);
            } else if r == DmResult::Closed {
                compose(a, d);
                app_invalidate(a, d, RefreshMode::Ui);
            } else if !rect_is_empty(d) {
                app_invalidate(a, d, if ph == 2 { RefreshMode::Ui } else { RefreshMode::Fast });
            }
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
        }
        Owner::Panel | Owner::Modal => {
            let modal = s.owner == Owner::Modal;
            ui_pointer(a, ph, ev, modal);
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
        }
        _ => {}
    }
}

// The deferred settle: one GL16 refresh for everything written since the last pause.
fn settle_now(a: &mut App) {
    // Under the Pages view the page is not on screen: the view's opening refreshed the whole screen
    // and its closing composes it all again, so nothing is due. Settling now would draw the whole view
    // again (compose) under its keyboard, and the panel over both.
    let s = ed(a);
    if s.pv.open {
        s.settle = RECT_EMPTY;
        s.panel_stale = false;
        return;
    }
    if !rect_is_empty(s.settle) {
        let r = s.settle;
        show(a, r, RefreshMode::Ui);
    }
    ed(a).settle = RECT_EMPTY;
    if ed(a).panel_stale {
        repaint_panel(a, RefreshMode::Ui);
    }
    ed(a).panel_stale = false;
}

fn write_thumb_if_needed(a: &mut App) {
    let s = ed(a);
    if !s.thumb_dirty || s.index != 0 {
        return;
    }
    save_thumb_from_canvas(s);
    s.thumb_dirty = false;
}

// A background job ended since the open page (page), or a cache (caches: note_white), drew a
// picture white (a template's, being made for this notebook): they are drawn again, with the
// picture if it is there now, and so is what shows them.
fn pictures_arrived(a: &mut App, page: bool, caches: bool) {
    if caches {
        let s = ed(a);
        s.white_caches = false;
        s.paper = SharedPaper::default();
        canvas_free(&mut s.image_prev);
    }
    if page {
        render_page_full(a);
    }
    let s = ed(a);
    if page && !s.picture_missing {
        // (still missing: another job ended, this one is to come)
        if s.index == 0 {
            s.thumb_dirty = true; // the Library's card of the notebook
        }
        compose_all(a, RefreshMode::Full);
    } else if caches && s.pv.open {
        compose_all(a, RefreshMode::Ui);
    } else if caches && s.paper_open {
        draw_paper_sheet(a);
        let r = ed(a).paper_rect;
        app_invalidate(a, r, RefreshMode::Ui);
    }
}

pub fn editor_tick(a: &mut App, now: u32) {
    let s = ed(a);
    // The pen gone for SETTLE_LEFT_MS (not just lifted high for a moment), or near for too long.
    let stopped = (now.wrapping_sub(s.settle_at) as i32) >= 0 || (!s.pen_near && (now.wrapping_sub(s.pen_left) as i32) >= SETTLE_LEFT_MS);
    if s.open && s.owner == Owner::None && (!rect_is_empty(s.settle) || s.panel_stale) && stopped {
        settle_now(a);
    }
    // (Never under a stroke: an erase stroke restores toward S->paper as it goes.)
    let s = ed(a);
    if s.open && s.owner == Owner::None && !s.pinching {
        let page = s.picture_missing && s.picture_gen != bgimage_generation();
        let caches = s.white_caches && s.white_gen != bgimage_generation();
        if page || caches {
            pictures_arrived(a, page, caches);
        }
    }
    let mut d = RECT_EMPTY;
    let s = ed(a);
    if s.open && s.owner == Owner::None && with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_tick(ts, a, now, &mut d)) {
        app_invalidate(a, d, RefreshMode::Ui);
    }
    if ed(a).open {
        with_modal(a, |s| &mut s.pv, |pv, a| pagesview_tick(pv, a, now));
    }
    let s = ed(a);
    if s.open && s.owner == Owner::Link && !s.link_held && !s.link_press.moved && now.wrapping_sub(s.link_down_at) >= LINK_HOLD_MS {
        link_hold(a);
    }
    // Holding the button ~1.2 s while hovering (no ink) clears the page once: not a hold during
    // which a sheet was in front of the page, nor one the pen pressed anything during.
    let s = ed(a);
    if s.btn_down && sheet_open(s) {
        s.btn_used_for_ink = true;
    }
    if s.open
        && s.btn_down
        && !s.btn_used_for_ink
        && !s.btn_hold_fired
        && s.owner == Owner::None
        && !sheet_open(s)
        && now.wrapping_sub(s.btn_down_at) >= 1200
    {
        s.btn_hold_fired = true;
        s.nclicks = 0;
        clear_page(a);
    }
    // Autosave after a real pause (and never while the pen is down). One that fails (a full store)
    // tries again 2 s later, not at every tick: each try says so with a toast.
    let s = ed(a);
    if s.open && s.page_dirty && s.owner == Owner::None && now.wrapping_sub(s.last_up) > 2000 && !save_page(a) {
        ed(a).last_up = now;
    }
    // A page whose picture was made while a stroke or gesture was under way (editor_picture_made).
    let s = ed(a);
    if s.open && s.page.is_some() && s.canvas_page != s.open_page_ref() && !s.page_canvas.px.is_empty() && s.owner == Owner::None && !s.pinching {
        render_page_full(a);
        compose_all(a, RefreshMode::Ui);
    }
}

// ---- headless checks ----

// atoi: the leading number of s (white space and a sign first), 0 for none.
fn atoi(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (9..=13).contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut v: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((b[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

// The C's memcmp of two ToolParams: the same bytes (every field's bits; -0 is not 0, a NaN is
// itself).
fn params_same(x: &ToolParams, y: &ToolParams) -> bool {
    (0..ToolParams::COUNT).all(|i| x.get(i).to_bits() == y.get(i).to_bits())
}

// The open page's newest stroke, for the stroke_* keys (the C's st).
fn newest(s: &EditorState) -> Option<&Stroke> {
    s.open_page()?.strokes.last().map(|x| &**x)
}

/// The headless runner's "editor.*" keys, asked without the prefix (app_probe's contract);
/// "templates.*" keys reach it whole (docs/SIM.md).
pub fn editor_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    let dpi = a.dpi;
    let Some(s) = a.ed.as_deref() else { return -1 };
    let st = &a.settings;
    let has_page = s.open && s.page.is_some();
    if key == "page" {
        csnprintf!(out, cap, "%d", if s.open { s.index + 1 } else { 0 });
    } else if key == "loaded_pages" {
        csnprintf!(out, cap, "%d", if s.open { s.hist.len() as i32 } else { 0 });
    } else if key == "pages" {
        csnprintf!(out, cap, "%d", if s.open { s.nb.npages() } else { 0 });
    } else if key == "notebook_last_page" {
        // (app_probe's "notebook.last_page": what a reopen lands on, kept apart from the open page)
        csnprintf!(out, cap, "%d", if s.open { s.nb.last_page + 1 } else { 0 });
    } else if key == "page_name" {
        csnprintf!(out, cap, "%s", if has_page { nav_name(&s.nav, s.nb.page_ids[s.index as usize]) } else { "" });
    } else if key == "index_rows" {
        csnprintf!(out, cap, "%d", if has_page { nav_index(&s.nav, &s.nb, &mut []) } else { 0 });
    } else if key == "link_mode" {
        csnprintf!(out, cap, "%d", if s.open && s.link_mode { 1 } else { 0 });
    } else if key == "links" {
        // The open page's links: how many it has, and how many of them have their box on it.
        let pid = if has_page { s.nb.page_ids[s.index as usize] } else { 0 };
        let boxed = if has_page {
            let mut links = [NavLink::default(); NAV_LINKS_PER_PAGE as usize];
            let mut areas = [RECT_EMPTY; NAV_LINKS_PER_PAGE as usize];
            nav_link_areas(&s.nav, s.open_page().unwrap(), pid, 1.0, &mut links, &mut areas)
        } else {
            0
        };
        let mut first = 0;
        csnprintf!(out, cap, "%d %d", if has_page { nav_links_of(&s.nav, pid, &mut first) } else { 0 }, boxed);
    } else if let Some(n) = key.strip_prefix("link.") {
        // The open page's link N (from 1, in the order links.ini has them): where it goes.
        let mut first = 0;
        let n_links = if has_page { nav_links_of(&s.nav, s.nb.page_ids[s.index as usize], &mut first) } else { 0 };
        let i = atoi(n) - 1;
        csnprintf!(out, cap, "%s", if i >= 0 && i < n_links { s.nav.links[(first + i) as usize].target.as_str() } else { "" });
    } else if key == "tool" {
        let name = app_probe_tool(st.current_tool); // a tool it lacks: its number
        match name {
            Some(name) => csnprintf!(out, cap, "%s", name),
            None => csnprintf!(out, cap, "%d", st.current_tool.0),
        };
    } else if key == "zoom" {
        csnprintf!(out, cap, "%.2f", s.zoom / s.zoom_default); // 1.00 = fitted page view
    } else if key == "page_size" {
        // The open page's own size: its notebook's, whatever the screen (a page that came at the
        // screen's size instead would show here).
        if has_page {
            let p = s.open_page().unwrap();
            csnprintf!(out, cap, "%dx%d", p.w, p.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "scale" {
        csnprintf!(out, cap, "%.2f", if s.open { s.zoom } else { 0.0 }); // screen px per page px
    } else if key == "zoomed" {
        let z = s.open && zoomed(a);
        csnprintf!(out, cap, "%d", if z { 1 } else { 0 });
    } else if key == "stroke_start" {
        // Where the page's newest stroke starts, in page px: ink lands under the pen through the view.
        let p = if has_page { s.open_page().and_then(|p| p.strokes.last()) } else { None };
        match p {
            Some(x) if !x.pts.is_empty() => csnprintf!(out, cap, "%.0f %.0f", x.pts[0].x, x.pts[0].y),
            _ => csnprintf!(out, cap, "%s", ""),
        };
    } else if key == "music_max" {
        // The most systems the Custom music sheet's layout fits on the page it is for.
        if s.mb.open {
            csnprintf!(out, cap, "%d", music_max_systems(&s.mb.m, s.mb.page_h, dpi));
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "hold_eraser" {
        csnprintf!(out, cap, "%d", if s.hold_eraser { 1 } else { 0 }); // Erase while held is down
    } else if key == "modal" {
        // What is in front (input goes to the first one open, in this order).
        let m = if s.pv.open {
            "pages"
        } else if s.tools_open {
            "tools"
        } else if s.mb.open {
            "music"
        } else if s.dm.open {
            "display"
        } else if s.ts.open {
            "templates"
        } else if s.paper_open {
            "paper"
        } else {
            "none"
        };
        csnprintf!(out, cap, "%s", m);
    } else if key == "strokes" {
        csnprintf!(out, cap, "%d", if has_page { s.open_page().unwrap().strokes.len() as i32 } else { 0 });
    } else if key == "stroke_points" {
        let x = if has_page { newest(s) } else { None };
        csnprintf!(out, cap, "%d", x.map_or(0, |x| x.pts.len() as i32));
    } else if key == "erase_strokes" {
        let n = if has_page { s.open_page().unwrap().strokes.iter().filter(|x| stroke_is_erase(x)).count() as i32 } else { 0 };
        csnprintf!(out, cap, "%d", n);
    } else if key == "live_matches_render" {
        // The open page drawn afresh, pixel for pixel what drawing, erasing and undoing left in the
        // page canvas: 1 when they are the same.
        let mut same = false;
        let mut c = Canvas::default();
        if has_page && s.canvas_page == s.open_page_ref() && !s.page_canvas.px.is_empty() && canvas_init(&mut c, s.pgw, s.pgh) == 0 {
            page_render(&mut c, s.open_page().unwrap(), Rect { x: 0, y: 0, w: s.pgw, h: s.pgh }, dpi);
            let pc = &s.page_canvas;
            same = pc.w == c.w
                && pc.h == c.h
                && pc.stride == c.stride
                && c.px[..c.stride as usize * c.h as usize] == pc.px[..c.stride as usize * c.h as usize];
        }
        csnprintf!(out, cap, "%d", if same { 1 } else { 0 });
    } else if key == "stroke_screen_mm" {
        // The newest stroke's widest, as the screen shows it at the zoom now (the precise eraser's
        // keeps its setting at any zoom); empty with no stroke.
        match if has_page { newest(s) } else { None } {
            Some(x) => csnprintf!(out, cap, "%.2f", x.params.size_max_mm * s.zoom),
            None => csnprintf!(out, cap, "%s", ""),
        };
    } else if key == "stroke_look" {
        // The newest stroke's look: its tool for ink ("pencil"), "white" and its texture for white
        // ink ("white charcoal"), "round tip" for the precise eraser; empty with no stroke.
        let x = if has_page { newest(s) } else { None };
        let t = x.map_or(TOOL_PENCIL, stroke_render_tool);
        let name = app_probe_tool(t);
        match x {
            None => csnprintf!(out, cap, "%s", ""),
            Some(x) if !stroke_is_erase(x) => csnprintf!(out, cap, "%s", name.unwrap_or("?")),
            Some(_) if t == TOOL_ERASER => csnprintf!(out, cap, "%s", "round tip"),
            Some(_) => csnprintf!(out, cap, "white %s", name.unwrap_or("?")),
        };
    } else if key == "stroke_tip" {
        // The newest stroke's tip variant (brush.h: 0 a tool's first tip, 1 the vertical highlighter);
        // empty with no stroke.
        match if has_page { newest(s) } else { None } {
            Some(x) => csnprintf!(out, cap, "%d", stroke_tip(x) as i32),
            None => csnprintf!(out, cap, "%s", ""),
        };
    } else if key == "stroke_settings" {
        // Whose pen feel the newest stroke was drawn with: the tool whose settings its parameters
        // are (its look's tool tried first), "eraser" also for the eraser's at the zoom now (the
        // precise eraser's widths are divided by it), "other" for none; empty with no stroke.
        let x = if has_page { newest(s) } else { None };
        let mut who: Option<&str> = None;
        let look = x.map_or(TOOL_PENCIL, stroke_render_tool);
        if let Some(x) = x {
            let mut k = -1;
            while who.is_none() && k < TOOL_COUNT as i32 {
                let t = if k < 0 { look } else { ToolKind(k) };
                // (a tool this build lacks reads past the table in the C: here it stops the runner)
                if params_same(&x.params, &st.tools[t.0 as usize]) {
                    who = app_probe_tool(t);
                }
                k += 1;
            }
        }
        let ez = eraser_at_zoom(&st.tools[TOOL_ERASER.0 as usize], s.zoom);
        if let Some(x) = x {
            if who.is_none() && params_same(&x.params, &ez) {
                who = app_probe_tool(TOOL_ERASER);
            }
        }
        csnprintf!(out, cap, "%s", if x.is_none() { "" } else { who.unwrap_or("other") });
    } else if key == "compact_listed" {
        // 1 while the open notebook's erased.txt lists pages for compaction
        csnprintf!(out, cap, "%d", if s.open && compact_listed(s.nb.id.as_str()) { 1 } else { 0 });
    } else if key == "paper_cache" {
        csnprintf!(out, cap, "%d", if s.paper.px.is_some() { 1 } else { 0 }); // the paper erase strokes restore toward
    } else if key == "matches_closed" {
        // The open page shows as its file did when its notebook last closed to be compacted (1 the
        // same, 0 not); empty when that was another notebook or page.
        #[cfg(feature = "sim")]
        {
            let (gid, gpage, ghash) = G_CLOSED.with(|g| {
                let g = g.borrow();
                (g.id, g.page, g.hash)
            });
            let known = has_page
                && s.canvas_page == s.open_page_ref()
                && !s.page_canvas.px.is_empty()
                && gid.as_bytes() == s.nb.id.as_bytes()
                && gpage == s.nb.page_ids[s.index as usize];
            if known {
                csnprintf!(out, cap, "%d", if canvas_hash(&s.page_canvas) == ghash { 1 } else { 0 });
            } else {
                csnprintf!(out, cap, "%s", "");
            }
        }
        #[cfg(not(feature = "sim"))]
        {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "unsaved" {
        csnprintf!(out, cap, "%d", if s.open && (s.page_dirty || s.info_dirty) { 1 } else { 0 });
    } else if key == "ticked" {
        csnprintf!(out, cap, "%d", if s.pv.open { pagesview_ticked_count(&s.pv) } else { 0 });
    } else if key == "paper" {
        if !has_page {
            csnprintf!(out, cap, "none");
        } else if paper_is_custom(s.open_page().unwrap().paper) {
            csnprintf!(out, cap, "custom");
        } else {
            csnprintf!(out, cap, "%d", s.open_page().unwrap().paper.0 as i32);
        }
    } else if key == "paper_hex" {
        // The page's paper value as 8 hex digits (an image paper: 0x50 for a template, 0x51 for an
        // imported page, then its picture's id), empty with no notebook open.
        if has_page {
            csnprintf!(out, cap, "0x%08x", s.open_page().unwrap().paper.0);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "picture" {
        // Whether the page's picture (an image paper's) shows: "shown", or "missing" while the page
        // draws white for it (a template's picture not made yet); empty for any other paper.
        if has_page && paper_is_image(s.open_page().unwrap().paper) {
            csnprintf!(out, cap, "%s", if s.picture_missing { "missing" } else { "shown" });
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "pages_tab" || key == "picking" || key == "keyboard" || key == "text" || key.starts_with("tile.") {
        return pagesview_probe(&s.pv, &s.nb, &s.nav, key, out, cap); // the Pages view's
    } else if let Some(k) = key.strip_prefix("templates.") {
        return tplsheet_probe(&s.ts, k, out, cap); // the Templates sheet over the page
    } else if key == "panel" {
        csnprintf!(out, cap, "%s", if s.collapsed { "collapsed" } else { "expanded" });
    } else if key == "panel_rect" {
        csnprintf!(out, cap, "%d %d %d %d", s.panel.x, s.panel.y, s.panel.w, s.panel.h); // as toast.rect
    } else if key == "panel_button" {
        csnprintf!(out, cap, "%d", btn_h()); // an expanded panel's buttons' height, px
    } else if key == "panel_fits" {
        // 1 while the panel is inside the screen with its margins free above and below it, its
        // buttons at full size (as without pins, upright or sideways)
        let p = s.panel;
        let m = ui_px(PANEL_MARGIN as f32);
        let in_ = p.x >= 0 && p.x + p.w <= a.w && p.y >= m && p.y + p.h <= a.h - m;
        csnprintf!(out, cap, "%d", if in_ && btn_h() == ui_px(100.0) { 1 } else { 0 });
    } else if key == "pins" {
        settings_pins_text(st, out, cap); // the pinned pens in pin order, as settings.ini names them
    } else if key == "draw_tool" {
        let name = app_probe_tool(button_tool(st)); // the tool button's pen
        csnprintf!(out, cap, "%s", name.unwrap_or("?"));
    } else if key == "tool_button" {
        // What the tool button showed as the panel was last drawn: its pen, "grid" while that pen
        // is pinned, empty while the panel is collapsed.
        let t = s.tool_button_shows;
        let name = if t >= 0 { app_probe_tool(ToolKind(t)) } else { None };
        csnprintf!(out, cap, "%s", if t == TB_GRID { "grid" } else if t == TB_NONE { "" } else { name.unwrap_or("?") });
    } else if key == "pin_mode" {
        csnprintf!(out, cap, "%d", if s.tools_open && s.pin_mode { 1 } else { 0 });
    } else if key == "tools_rect" {
        // Where the tool picker is while it is open, "x y w h" (as toast.rect), else empty.
        let r = s.tools_rect;
        if s.tools_open {
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "before_eraser" {
        // The pen used before the eraser (settings.ini before_eraser: where a tap on the eraser in
        // use goes back to, if it is still on the panel), empty for none.
        let b = st.before_eraser;
        let name = if b >= 0 && b < TOOL_COUNT as i32 { app_probe_tool(ToolKind(b)) } else { None };
        csnprintf!(out, cap, "%s", name.unwrap_or(""));
    } else if key == "white_mark" {
        csnprintf!(out, cap, "%d", if s.white_mark { 1 } else { 0 }); // the panel as drawn last shows the pen draws white
    } else if key == "music" {
        // The layout on the Custom music sheet while it is open, else the page's custom paper.
        let mut m = MusicLayout::default();
        if s.mb.open {
            app_probe_music(Some(&s.mb.m), out, cap);
        } else {
            let decoded = has_page && music_decode(s.open_page().unwrap().paper, &mut m);
            app_probe_music(if decoded { Some(&m) } else { None }, out, cap);
        }
    } else {
        return -1;
    }
    0
}

// Checks of the helpers that need no screen but this module's: the view's arithmetic, the bilinear
// blit, the panel's layout against panel.h's height, and the session cache's lookups. (The editor
// as a whole is the scenarios' to check: tests/scenarios/*.qs, make rust-scenarios.)
#[cfg(test)]
mod tests {
    use super::*;

    // An App with the editor's state on a w x h screen, for a page of pw x ph, at 300 dpi.
    fn app(w: i32, h: i32, pw: i32, ph: i32) -> App {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let mut a = App { w, h, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, w, h), 0);
        a.ed = Some(Box::default());
        let s = ed(&mut a);
        s.pgw = pw;
        s.pgh = ph;
        a
    }

    #[test]
    fn the_view_fits_the_pages_width() {
        // Upright, a page of the screen's size: 100% and no offset, the identity.
        let mut a = app(1860, 2480, 1860, 2480);
        view_reset(&mut a);
        assert!(view_identity(&a) && !zoomed(&a));
        let (mut x, mut y) = (0.0, 0.0);
        to_page(ed_ref(&a), 100.5, 200.25, &mut x, &mut y);
        assert_eq!((x, y), (100.5, 200.25));
        assert_eq!(page_to_screen(&a, Rect::new(10, 10, 5, 5)), Rect::new(8, 8, 10, 10));
        assert_eq!(page_to_screen(&a, RECT_EMPTY), RECT_EMPTY);
        // Sideways: the page's width across the screen, from its top; the whole page at the least.
        let mut a = app(2480, 1860, 1860, 2480);
        view_reset(&mut a);
        let s = ed_ref(&a);
        assert_eq!(s.zoom, 2480.0 / 1860.0);
        assert_eq!(s.zoom_min, 1860.0 / 2480.0);
        assert_eq!((s.ox, s.oy), (0.0, 0.0));
        assert!(!view_identity(&a) && !zoomed(&a));
        // Pinched out past the whole page: held to it, centred across.
        let s = ed(&mut a);
        s.zoom = 0.1;
        s.oy = 300.0;
        view_clamp(&mut a);
        let s = ed_ref(&a);
        assert_eq!(s.zoom, 0.75);
        assert_eq!(s.oy, 0.0);
        assert_eq!(s.ox, -(2480.0f32 / 0.75 - 1860.0) / 2.0);
        assert!(zoomed(&a));
        // A notebook from a larger Scribe: its whole page fits this screen's width.
        let mut a = app(1860, 2480, 1986, 2648);
        view_reset(&mut a);
        let s = ed_ref(&a);
        assert_eq!(s.zoom, 1860.0 / 1986.0);
        assert_eq!((s.ox, s.oy), (0.0, 0.0));
        assert!(!zoomed(&a));
        // Nearly 100% upright snaps to the identity.
        let mut a = app(1860, 2480, 1860, 2480);
        view_reset(&mut a);
        ed(&mut a).zoom = 1.005;
        view_clamp(&mut a);
        assert!(view_identity(&a));
    }

    #[test]
    fn the_blit_at_100_percent_is_the_page() {
        let mut a = app(64, 48, 64, 48);
        let s = ed(&mut a);
        assert_eq!(canvas_init(&mut s.page_canvas, 64, 48), 0);
        for (i, p) in s.page_canvas.px.iter_mut().enumerate() {
            *p = (i * 37 % 251) as u8;
        }
        view_reset(&mut a);
        blit_view(&mut a, Rect::new(0, 0, 64, 48));
        assert_eq!(a.screen.px, ed_ref(&a).page_canvas.px);
        // Zoomed out to half: the desk around the page, the page sampled between its pixels.
        let s = ed(&mut a);
        s.zoom = 0.5;
        s.ox = -32.0;
        s.oy = -24.0;
        blit_view(&mut a, Rect::new(0, 0, 64, 48));
        assert_eq!(a.screen.px[0], 205);
        let pc = &ed_ref(&a).page_canvas;
        // Screen (32, 24) samples page (32.5, 24.5): its four pixels, half each way.
        let w = (0.5f32 * 255.0) as u32;
        let at = |x: usize, y: usize| pc.px[y * 64 + x] as u32;
        let top = at(32, 24) * (255 - w) + at(33, 24) * w;
        let bot = at(32, 25) * (255 - w) + at(33, 25) * w;
        assert_eq!(a.screen.px[24 * 64 + 32] as u32, (top * (255 - w) + bot * w + 32512) / 65025);
    }

    #[test]
    fn the_panel_is_as_tall_as_panel_h_says() {
        for (w, h, pins) in [(1860, 2480, 0), (2480, 1860, 0), (2480, 1860, 4), (1860, 2480, 4)] {
            let mut a = app(w, h, 1860, 2480);
            for i in 0..pins {
                a.settings.pins[i] = TOOL_ORDER[i + 1];
            }
            a.settings.npins = pins as i32;
            fit_panel_to(&a.settings, a.h);
            let p = panel_rect_at(&a, a.w - pw() - ui_px(28.0), ui_px(220.0));
            ed(&mut a).panel = p;
            draw_panel(&mut a);
            let s = ed_ref(&a);
            // Inside the screen, its margins free.
            let m = ui_px(PANEL_MARGIN as f32);
            assert!(p.x >= 0 && p.x + p.w <= w && p.y >= m && p.y + p.h <= h - m);
            assert_eq!(p.h, panel_height(g_fit(), pins as i32));
            // The buttons from the top: grip, collapse, the tool button, the pins, the eraser, (undo,
            // redo, clear and previous greyed out), the page chip, next, paper, display, pen feel,
            // the library, whose gap under it ends the panel but for its last 6 px.
            let ids: Vec<i32> = s.hits.h.iter().map(|h| h.id).collect();
            let mut want = vec![P_GRIP, P_COLLAPSE, P_TOOLPICK];
            want.extend(std::iter::repeat_n(P_TOOL, pins));
            want.extend([P_TOOL, P_PAGES, P_NEXT, P_PAPER, P_DISPLAY, P_PENSET, P_LIBRARY]);
            assert_eq!(ids, want);
            let last = s.hits.h.last().unwrap().r;
            assert_eq!(last.y + last.h + gap(), p.y + p.h - ui_px(6.0));
            assert_eq!(s.tool_button_shows, TOOL_PENCIL.0);
            assert!(!s.white_mark);
        }
        // Collapsed: the square is the one button.
        let mut a = app(1860, 2480, 1860, 2480);
        ed(&mut a).collapsed = true;
        let p = panel_rect_at(&a, 5000, -50);
        assert_eq!(p, Rect::new(1860 - panel_box(), ui_px(PANEL_MARGIN as f32), panel_box(), panel_box()));
        ed(&mut a).panel = p;
        draw_panel(&mut a);
        let s = ed_ref(&a);
        assert_eq!(s.hits.h.len(), 1);
        assert_eq!((s.hits.h[0].id, s.hits.h[0].r), (P_EXPAND, p));
        assert_eq!(s.tool_button_shows, TB_NONE);
    }

    #[test]
    fn the_session_cache_finds_the_open_page() {
        let mut s = EditorState::default();
        for pid in [7u32, 9, 11] {
            let i = s.entry_for(pid).unwrap();
            assert_eq!(s.hist[i].h.limit, 500);
            s.hist[i].page = Some(page_new(10, 10, PAPER_BLANK));
        }
        assert_eq!(s.entry_for(9), Some(1));
        assert_eq!(s.open_page_ref(), 0);
        assert!(!s.is_open_page(1) && s.open_page().is_none());
        s.page = Some(9);
        assert_eq!(s.page_ix(), Some(1));
        assert_eq!(s.open_page_ref(), page_ref(s.hist[1].page.as_deref().unwrap()));
        assert!(s.is_open_page(1) && !s.is_open_page(0));
        // A page deleted: the last entry takes its place, as in the C; the open one is forgotten.
        forget_page(&mut s, 7);
        assert_eq!(s.hist.iter().map(|e| e.page_id).collect::<Vec<_>>(), [11, 9]);
        assert_eq!(s.page_ix(), Some(1));
        forget_page(&mut s, 9);
        assert_eq!(s.page, None);
        // The history of one entry and the page of another, at once.
        s.entry_for(5).unwrap();
        let (h, p) = hist_page(&mut s.hist, 1, 0);
        assert_eq!((h.limit, p.w), (500, 10));
        let (h, p) = hist_page(&mut s.hist, 0, 0);
        assert_eq!((h.limit, p.w), (500, 10));
    }

    #[test]
    fn probes_read_atoi_as_c_does() {
        assert_eq!(atoi("12"), 12);
        assert_eq!(atoi(" -3x"), -3);
        assert_eq!(atoi("x"), 0);
        assert_eq!(atoi(""), 0);
    }
}
