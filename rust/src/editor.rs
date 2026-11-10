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
// The band of the page an erased stroke counts as toward DU wear (stroke_wear_area): its ink and the
// paper right around it, about the height of a small handwritten letter.
const WEAR_BAND_MM: f32 = 4.0;
/// A pen that left proximity less than this before a finger came down still counts as in range for that
/// finger: its hold arms no lasso (the writing hand, a grip thumb; select.rs sel_finger_arm).
const FHOLD_PEN_MS: u32 = 500;
// Migrated link areas wait this long after the first migration before links.ini is written (the idle
// ticks migrate a page each, so the notebook's pages are saved once); a failed write is tried again
// this long after.
const LINKS_FLUSH_MS: u32 = 2000;
// A lasso smaller than this both ways draws no link's area: the strip stays for another try.
const LINK_BOX_MIN_MM: f32 = 5.0;
// The lasso keeps at most this many points: past it every other point goes and the spacing a new point
// needs doubles, so a long scribble around an area costs a bounded memory and compose.
mod select;
use select::*;
use std::collections::HashSet;

const LASSO_MAX: usize = 4096;
/// The most targets kept for Link to...'s Recent targets (it offers four of them).
const RECENT_MAX: usize = 8;

// settings.hints' bits (docs/FORMAT.md): the one-time tips shown. Bits 2 (the link areas tip), 5 (the first
// shape), 6 (the pan hint) and 8 (the sideways hint) were tips 2.0.0-beta.2 left out (the owner's toast pass,
// 2026-11-04): no longer written, a settings.ini may still have them set.
/// the first notebook that opened after the update said what two fingers up and down do now
const HINT_FIRST_OPEN: i32 = 1 << 0;
/// the first link made said that a finger's tap follows it and a hold opens its card
const HINT_FIRST_LINK: i32 = 1 << 1;
/// the first unrecorded finger drawing faded away with its one-time explanation
const HINT_FSHAPE: i32 = 1 << 7;
/// the quick sizes tip was shown (Pen feel opened from the toolbar the first time), or the card was used (2.1)
const HINT_QUICK_SIZE: i32 = 1 << 9;
/// settings.hints' bits 3 and 4: how many times the Links bar showed its second header line (0 to 3)
const HINT_BAR_SHIFT: i32 = 3;
const HINT_BAR_MASK: i32 = 3;
/// The back list holds this many pages: the oldest is dropped past it.
const BACK_MAX: usize = 32;

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
const P_LINKS: i32 = 24;
// Stable registry id after phase 4's three actions; moved into actions.rs on integration.
const P_SHAPES: i32 = 25;
const P_SHAPE_HOLD: i32 = 26;
const P_SHAPE_QUADS: i32 = 27;
const P_SHAPE_ROUND: i32 = 28;
const P_SHAPE_LINKS: i32 = 29;
const P_SHAPE_MIN: i32 = 64;
const P_SHAPE_CORNER: i32 = 65;
const P_SHAPE_DONE: i32 = 30;
const P_SETTINGS: i32 = 31;
/// the tool picker's Select (Select mode, select.rs)
const P_PICK_SELECT: i32 = 66;

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
    /// a pointer that began on the Links bar (its chips, tool row, grip or header)
    Bar = 11,
    /// a pointer that began outside the bar (or Find), the panel included: swallowed, decided at the lift
    Outside = 12,
    /// a pointer that began on the Find sheet (its rows or its keyboard)
    Find = 13,
    /// the pen drawing the dashed lasso around a new link's area (never ink)
    Lasso = 14,
    /// a pointer that began on the long-press card (its rows) or on Rename's keyboard
    Card = 15,
    /// a pointer that began on Link to... (its chips and tool row) or on New page...'s keyboard
    LinkTo = 16,
    /// a pointer that began on the link-making strip (its Cancel, or the strip around it)
    Strip = 17,
    Map = 18,
    /// A bound side-button contact, swallowed without ink.
    PenDrag = 19,
    /// Select mode: the pen drawing its lasso, a drag of the selection, its bar (select.rs)
    Select = 20,
    /// a pointer that began on the toolbar's count pad, or one let go while its batch was written
    AddPad = 21,
    /// the contact that opened the quick sizes card (a pen button held HOLD_MS), which may slide to a chip, or one
    /// that began on the card (quicksize.rs)
    QuickSize = 22,
}

/// The owner's name in grimoire.log and the `editor.owner` probe.
fn owner_name(o: Owner) -> &'static str {
    const K: [&str; 23] = [
        "none", "draw", "erase", "panel", "modal", "display", "pages", "tools", "build", "templates", "link", "bar",
        "outside", "find", "lasso", "card", "linkto", "strip", "map", "pen-drag", "select", "add-pad", "quick-size",
    ];
    K[o as usize]
}

/// The editor's contact and link-making lines in grimoire.log: DIAG_LINES a minute, then one line saying the
/// rest are dropped, until a minute from the window's first line has passed (the next line renews it).
#[derive(Clone, Copy, Debug, Default)]
struct DiagBudget {
    start: Option<u32>,
    lines: u16,
}
const DIAG_WINDOW_MS: u32 = 60_000;
const DIAG_LINES: u16 = 40;

impl DiagBudget {
    /// Whether a line may go to the log at now (ms); the first refused one of a window says so itself.
    fn admit(&mut self, now: u32) -> bool {
        if self.start.is_none_or(|t| now.wrapping_sub(t) >= DIAG_WINDOW_MS) {
            self.start = Some(now);
            self.lines = 0;
        }
        self.lines = self.lines.saturating_add(1);
        if self.lines == DIAG_LINES + 1 {
            qlog!("editor: %d contact and link lines this minute: the rest of it is not logged", DIAG_LINES as i32);
        }
        self.lines <= DIAG_LINES
    }
}

/// Making a link without ink (PHASE2 6): the strip is up and the next pen contact on the page is the lasso
/// (Armed), the lasso is being drawn (Lasso), its area waits for where the link goes (Pick: Link to..., or
/// the Pages view under Choose from the pages...), or New page...'s keyboard names the page it will go to
/// (Naming).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LmState {
    #[default]
    Off,
    Armed,
    Lasso,
    Pick,
    Naming,
}

/// The link being made: the strip, the lasso and the area. Screen-only: nothing here touches the page,
/// page_canvas, the history or the PDF until the pick saves the link.
#[derive(Clone, Debug, Default)]
struct LinkMaking {
    state: LmState,
    /// the lasso, page px, at most LASSO_MAX points (reserved at its start; with no room none are kept and
    /// the lasso is only drawn as it goes)
    pts: Vec<(f32, f32)>,
    shape_preview: Option<Vec<(f32,f32)>>,
    shape_anchor: (f32,f32),
    shape_anchor_k: u32,
    shape_tried: bool,
    shape_kind: ShapeKind,
    outline: LinkOutline,
    /// the last point the lasso went to, page px (the next segment starts there)
    last: (f32, f32),
    /// how far (screen px) a new point must be from the last one to be kept; doubles as the lasso fills
    spacing: f32,
    /// the dash phase at the last point, so the lasso drawn piece by piece matches the one compose draws whole
    phase: i32,
    /// the box of every point the pen gave, page px (the area: decimation never shrinks it)
    bx0: f32,
    by0: f32,
    bx1: f32,
    by1: f32,
    /// screen px: what the lasso has drawn so far
    lasso_rect: Rect,
    /// page px: the area, from the lift until the pick
    area: Rect,
    /// the strip, where compose last laid it out
    strip: Rect,
    /// a press on the strip (its Cancel inverts at the down)
    press: Press,
    cancel_down: bool,
    /// came from the Links bar's New link (or a card over the bar): the bar comes back when the making ends
    from_bar: bool,
    /// New page...'s naming keyboard
    kb: Keyboard,
    /// the card's Redraw the area (8.8): the lasso gives this link (its seed) a new area instead of making one
    redraw: Option<u32>,
    /// the card's Change where it goes (8.7): the links (seeds, N from a bar row) Link to... gives a new
    /// target; empty while a link is made
    retarget: Vec<u32>,
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

/// The open page's link boxes for the eye and the bar: page px, padded to a finger's size (what a tap hits,
/// nav_link_areas at link_under's minimum), each with the number of the bar's entry it belongs to
/// (linkbar_group). Memory only. It is made when the eye draws or the bar opens, not at every change
/// (nav_link_areas walks the strokes once per link), so it is stale from any change to the page, its ink or
/// its links until then.
struct LinkBoxes {
    n: usize,
    link: [NavLink; LB_MAX],
    area: [Rect; LB_MAX],
    /// its bar entry's number, from 1
    entry: [u8; LB_MAX],
    stale: bool,
    /// the page's document links' exact areas, page px (the eye draws them dotted and unnumbered; no row of
    /// the bar stands for one of them: they fold into its "From the PDF" row)
    doc: Vec<Rect>,
    /// how many boxes the eye's last paint drew (the scripts' editor.eye_drawn: 0 when it painted none)
    drawn: usize,
    /// how many document-link boxes it drew (editor.eye_docs)
    doc_drawn: usize,
}
// (arrays derive Default only up to 32 elements: EditorState keeps its derive)
impl Default for LinkBoxes {
    fn default() -> Self {
        LinkBoxes { n: 0, link: [NavLink::default(); LB_MAX], area: [RECT_EMPTY; LB_MAX], entry: [0; LB_MAX], stale: true, doc: Vec::new(), drawn: 0, doc_drawn: 0,
        }
    }
}

/// The page the Links bar shows above itself while an entry is looked at (D2): a display state only. The open
/// page (s.index, s.page, its history, the link boxes, s.zoom/ox/oy) and page_canvas stay the origin's until a
/// commit, so autosave, undo or a link tap can never act on the peeked page.
#[derive(Clone, Copy, Debug, Default)]
struct Peek {
    active: bool,
    /// the peeked page's index and id
    index: i32,
    pid: u32,
    /// the canvas cache slot it is shown from
    slot: usize,
    /// pages looked at since the bar opened: three or more leave GL16 ghosting, so a cancel then is a Full
    count: i32,
    /// it is the back list's newest page ("Back to ..."): staying there pops the list instead of pushing
    from_back: bool,
}

/// A page canvas the bar keeps for a peek (the origin's is page_canvas, so three in all): rendered ahead of the
/// tap in bands, or at the tap. Keyed by page id, page size and bgimage_generation(); freed when the bar closes.
#[derive(Default)]
struct CacheEntry {
    /// 0: the slot is free
    pid: u32,
    w: i32,
    h: i32,
    pic_gen: u32,
    canvas: Canvas,
    /// the page it was drawn from when the session cache (EditorState.hist) did not have it: read by the
    /// pumped reader, moved into the session cache at a commit
    page: Option<Box<Page>>,
    /// rows of the canvas drawn so far (all of them: ready)
    rows_done: i32,
    from_hist: bool,
    /// a picture an image paper shows was not there when it was drawn: a commit draws the page again
    white: bool,
    /// when it was last shown (cache_tick), for the least recently used
    used: u32,
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
struct ShapeHold {
    anchor: (f32, f32), anchor_k: u32, still_fired: bool, burst: bool,
    preview: Option<ShapePreview>, last_lift: Option<PenSample>,
    last_kind: ShapeKind, last_reason: ShapeReason, kept: u32,
}
struct ShapePreview { saved: Canvas, region: Rect, raw_n: usize, stroke: Box<Stroke> }

#[derive(Default)]
pub struct EditorState {
    /// when the open page's Pages preview was last made from its canvas (save_page), and whether the
    /// save under way is the idle autosave (which makes it at most every 30 s)
    preview_saved_ms: u32,
    /// The open page's file was written, and its preview or kept drawing is not made yet (or not made again):
    /// editor_tick makes them at a quiet moment (caches_due). Never set while nothing can be made.
    caches_pending: bool,
    /// The page canvas as it was before the step under way, a copy of it between steps (base_follow):
    /// what differs from it when a step ends is that step's patch (history.rs Patch), which undo and
    /// redo swap back. `base_page` is the page it mirrors (canvas_page's), 0 for none; `base_hold`
    /// keeps it as it was through an eraser sweep, which draws the page again as it goes.
    base: Canvas,
    base_page: usize,
    base_hold: bool,
    step_pushes: u64,
    /// undos and redos done by swapping a patch back (a probe, editor.undo_patched)
    undo_patched: u32,
    /// the stroke eraser's tiles (MASK_TILE) drawn again, and those the boxes of what it took off
    /// cover (a probe, editor.erase_tiles)
    erase_tiles: (u32, u32),
    /// the open page (its id) and how long its last whole drawing took, ms (u32::MAX when it came from its
    /// kept drawing): a page that took store_keep_render_ms() or longer keeps its drawing at each save
    full_ms: (u32, u32),
    /// whether the open page came up from its kept drawing (a probe, editor.page_kept)
    page_kept: bool,
    /// how many times "Loading page..." went up before a slow page was drawn (a probe, editor.loading_toasts)
    loading_toasts: u32,
    /// how many times "Adding N pages..." reached the panel before a batch of new pages was written, or a
    /// batch's Redo wrote them again (a probe, editor.add_notices)
    add_notices: u32,
    /// Select mode (select.rs), and the last lasso move's or paste's times, ms (old place, new place)
    sel: SelMode,
    last_lasso_ms: (u32, u32),
    map: MapView,
    map_kb: Keyboard,
    /// The toolbar's count pad (owner, 2026-11-03): the + (next page on the last page) held, or clicked with a
    /// modifier key, or the new-pages action: how many blank pages to add after the open page, which stays.
    add_kb: Keyboard,
    /// when the toolbar's batch was written (now_ms): a contact that came down by then is let go to its lift,
    /// as the Pages view's quiet_until (it was aimed at the pad, never at the page); cleared by the next one
    add_quiet_until: Option<u32>,
    /// the sample time (in_sample_ms) of the panel press under way: the + held that long at its lift (the
    /// tick may not have seen it yet) opens the count pad
    panel_down_k: u32,
    map_rename_pid: u32,
    sh: ShapeHold,
    open: bool,
    /// compose leaves the panel out (editor_live_under_panel: Live view's page under it)
    live_no_panel: bool,
    pb: PenButtons,
    pb_pen: PbPen,
    pb_buttons: u8,
    pb_erasers: u8,
    pb_point: Option<(f32, f32)>,
    pb_last: Option<(PenBtn, PenKind)>,
    pb_drag: Option<PenDrag>,
    hold_eraser: bool,
    hold_eraser_key: bool,
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
    /// grimoire.log's contact and link-making lines (diag): their budget, and the link-making state last said
    diag: DiagBudget,
    lm_said: LmState,

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
    /// every place a panel drag drew the panel by fast refreshes (screen px): its release cleans all of it
    drag_trail: Rect,
    last_drag_paint: u32,
    hits: HitList,
    /// every button the expanded panel draws, greyed out ones too: what panel_tile_hit tiles
    panel_cells: Vec<Rect>,
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
    /// the resting zoom: the page's width across the screen, or sideways with landscape_fit the whole page
    zoom_default: f32,
    zoom_min: f32,
    /// pinched in at most this far: four times the page's width across the screen, however it rests
    zoom_max: f32,
    /// the offset the resting view has across (the whole page sideways is centred: negative)
    rest_ox: f32,
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
    /// the two fingers are a session touchgest classified (PinchBegin.value 1): its moves say zoom (1),
    /// pan (2) or swipe (3); the legacy pinch (the runner's `pinch`) decides itself
    pinch_classified: bool,
    /// The pages the open notebook was left from by a link, the Pages view or Find, newest last, at most
    /// BACK_MAX (page ids): Back to the page before pops it. Memory only; a page turn does not push.
    back: Vec<u32>,
    /// the open page's link boxes (the eye, the bar's numbers)
    boxes: LinkBoxes,
    // beta.5's links (an inked lasso, no area) migrate to areas, the box of their ink, as their pages are
    // read and on idle ticks (PHASE2 5): an invisible rewrite, never an undo step, and never of the ink.
    /// migrated areas are in the Nav and not yet in links.ini
    links_dirty: bool,
    /// page ids tried this session (migrated, dormant only, read-only, or unreadable): never tried twice
    migrate_done: Vec<u32>,
    /// when the next links.ini flush may be tried (a pause after a migration, or 2 s after a failed save)
    links_try_at: u32,
    /// "link areas left for a later save" was logged (once per notebook opened)
    links_fail_logged: bool,
    /// the Links bar (linkbar.rs), over the page while it is open
    lb: LinkBar,
    /// the Find sheet (findsheet.rs), over the page while it is open
    find: FindSheet,
    /// Link to... (linkbar.rs in pick mode): where a new link goes, while its area waits (lm Pick)
    lt: LinkBar,
    /// the long-press card (linkcard.rs): a link held on the page, or a row held on the bar
    card: LinkCard,
    /// the quick sizes card (quicksize.rs): a pen button on the toolbar held HOLD_MS
    qs: QuickSize,
    /// a size picked, a pen the hold made the pen in use, or the tip's bit is not in settings.ini yet: 1 when it
    /// was asked for, 2 once a pass of the loop has presented the screen (editor_tick writes it then, so the
    /// card's close never waits for the storage)
    qs_save: u8,
    /// the targets ("p<page id>") links were made to this session, newest last, at most RECENT_MAX: Link to...'s
    /// Recent targets. Memory only; the notebook's closing clears it.
    recent: Vec<FixedStr<NAV_TARGET_LEN>>,
    /// the page it shows above itself, if it does
    peek: Peek,
    /// two page canvases besides page_canvas for the peeks, and what they did since the bar opened
    ccache: [CacheEntry; 2],
    cache_hits: i32,
    cache_misses: i32,
    cache_tick: u32,

    /// Display sheet (front light, invert)
    dm: DispMenu,

    // Tool picker (opened from the toolbar's tool button). In Pin mode a tap pins a pen to the
    // panel, or unpins it.
    tools_open: bool,
    shapes_menu: bool,
    /// the ink the picker last drew selected (white or not): the probe's, tools.ink
    tools_ink_drawn: Option<bool>,
    pin_mode: bool,
    /// where its top is meant to be: the panel's, as it opened (it stays there while pins move the
    /// panel; laid out again at a turn)
    tools_y: i32,
    tools_rect: Rect,
    tools_hits: HitList,
    tools_press: Press,
    /// the picker's button pressed, shown pressed until the lift (app_press_feedback)
    tools_pressed: Option<Rect>,
    /// a press on a Smart shapes slider has come over its track: the knob follows it to the lift
    tools_slider: bool,

    /// the Pages view (pagesview.h)
    pv: PagesView,
    destination: NotebookPicker,
    /// the pages Copy to notebook... copies (ids, in notebook order) while its notebook picker is up
    copy_pids: Vec<u32>,
    /// the notebooks a link was followed out of, newest last: Back returns to each one's page and view
    notebook_back: Vec<NotebookBack>,
    /// opened from the Graph the Library shows (2.0.0-beta.2): Back with nothing before it, and the Library
    /// button, go back to that Graph (library_return_to_graph); kept across links into other notebooks
    from_graph: bool,
    requested_page: Option<u32>,
    previews_checked: u32,
    // whether a previews batch of the notebook whose previews show was queued or running at the last tick
    previews_pending: bool,
    nav: Nav,
    /// the links the document brought (doclinks.ini, 2.1): read at open, written only by Copy page and Copy to
    /// notebook; never part of `nav` (links.ini's caps, the lasso and Clear page never see them)
    dl: DocLinks,
    // Links (nav.h). Add a link (the Index, or the add-link shortcut) shows the strip, and the next pen
    // contact on the open page draws the dashed lasso around the link's area (lm), never ink; then Link
    // to... picks where it goes. A finger's tap on a link follows it; one held on it (HOLD_MS) shows
    // its border and the long-press card beside it (card), over the page. Held anywhere else on the page,
    // it arms the finger lasso (select.rs sel_finger_arm).
    lm: LinkMaking,
    /// a finger on the page (OWN_LINK)
    link_press: Press,
    link_down_at: u32,
    link_held: bool,
    /// that finger's down ended a selection (a tap outside it, design 6.1): its tap and its swipe do nothing
    /// more; its hold is a hold as anywhere
    link_quiet: bool,
    /// that finger's hold arms no lasso (no surprises: a palm or a grip thumb never selects or pastes): the
    /// pen was in range (or left under FHOLD_PEN_MS before), or did anything since the down, or the panel
    /// called the contact broad (TOUCH_F_BROAD)
    link_spoiled: bool,

    // One-finger capture (phase 3): the path is integer-only and lives beside, never on, page_canvas.
    fs: FsStroke,
    /// how many capture points have already been painted into a.screen
    fs_drawn: usize,
    /// canvas_dot_poly's distance into its dot pitch, carried across event frames
    fs_phase: i32,
    /// screen rect occupied by the visible dotted trail
    fs_trail: Rect,
    fs_last_frame: u32,
    fs_thinned: bool,
    fs_reject_until: u32,
    fs_reject_pending: bool,
    fs_tip_pending: bool,
    /// a canceled trail waits for the pinch's first refresh, or its end if it never commits
    fs_pinch_trail: Rect,
    fs_cancel_waiting: bool,
    /// the shape probes retain the last outcome after the capture itself is reset
    fs_last_kind: i32,
    fs_last_action: i32,
    fs_last_reason: FsReason,
    fs_last_d1: i32,
    fs_last_thr: i32,
    fs_last_d2: i32,
    fs_fired: u32,

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
fn with_pv<R>(a: &mut App, f: impl FnOnce(&mut PagesView, &mut App, &NotebookInfo, &Nav) -> R,
) -> R {
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

fn with_map<R>(a: &mut App, f: impl FnOnce(&mut MapView, &mut App, &NotebookInfo, &Nav) -> R) -> R {
    let s = ed(a);
    let mut map = std::mem::take(&mut s.map);
    let nb = std::mem::take(&mut s.nb);
    let nav = std::mem::take(&mut s.nav);
    let result = f(&mut map, a, &nb, &nav);
    let s = ed(a);
    s.map = map;
    s.nb = nb;
    s.nav = nav;
    result
}
fn map_draw(a: &mut App, r: Rect) {
    with_map(a, |map, a, _, nav| mapview_draw(map, a, nav, r));
    if ed_ref(a).map_kb.open {
        let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
        let mut kb = std::mem::take(&mut ed(a).map_kb);
        keyboard_draw(&mut kb, &mut a.screen, screen);
        a.toast_floor = kb.area.y;
        ed(a).map_kb = kb;
    }
}
// The Graph opens on every notebook (past the Graph of all notebooks' caps, on this one, and says so).
fn map_show(a: &mut App, around: bool) {
    app_toast(a,"",0);
    close_sheets(ed(a));
    pagesview_close(&mut ed(a).pv);
    link_off(a);
    let pid = ed_ref(a).open_pid();
    let ok = with_map(a, |map, a, nb, nav| mapview_open_scoped(map, a, nb, nav, pid, around, None));
    if !ok {
        mapview_close(&mut ed(a).map);
        pages_show(a);
    }
    touch_context(a);
}
fn map_hide(a: &mut App, draw: bool) {
    let s = ed(a);
    mapview_close(&mut s.map);
    keyboard_close(&mut s.map_kb);
    s.owner = Owner::None;
    if draw {
        compose_all(a, RefreshMode::Full);
    }
    touch_context(a);
}
// How often the tick looks for previews while a batch of the notebook shown is being made: each tile is drawn
// as its preview lands (the job tells of each one), so a group of them that land together shows in one refresh.
const PREVIEWS_POLL_MS: u32 = 100;
// ...and while none is: previews written otherwise (the open page's, at an idle save) show within this.
const PREVIEWS_IDLE_POLL_MS: u32 = 500;

/// A previews batch ended: the tiles it made are drawn (those that landed since the last look), and those it
/// could not make lose their loading mark.
pub fn editor_previews_done(a:&mut App,id:&str) {
    if a.scr==ScreenId::Editor && a.ed.as_deref().is_some_and(|s|(s.pv.open&&s.nb.id==id)
        ||(s.destination.open&&s.destination.selected.as_ref().is_some_and(|nb|nb.id==id))) {previews_repaint(a,true);}
}

// The notebook whose page previews show (the Pages view's, or the destination picker's pages), and the
// previews generation its tiles were drawn at.
fn previews_shown(s:&EditorState)->Option<(FixedStr<STORE_ID_LEN>,u64)> {
    if s.destination.open {return s.destination.selected.as_ref().map(|nb|(nb.id,s.destination.previews_generation()));}
    s.pv.open.then(||(s.nb.id,pagesview_previews_generation(&s.pv)))
}

// New previews were written (the tick saw them), or a batch ended (`ended`): only the tiles that show something
// else now are drawn again, in one GL16 of them, and never twice for the same previews.
fn previews_repaint(a:&mut App,ended:bool) {
    // (the Find sheet over the Pages view: its tiles are drawn as it closes)
    if !ed_ref(a).destination.open && ed_ref(a).find.open {return;}
    let Some((id,drawn))=previews_shown(ed_ref(a)) else{return};
    if !ended && store_page_previews_generation(id.as_str())==drawn {return;}
    let r=if ed_ref(a).destination.open {with_modal(a,|s|&mut s.destination,|p,a|p.repaint_pages(a))}
        else {with_pv(a,pagesview_repaint_tiles)};
    if !rect_is_empty(r) {app_invalidate(a,r,RefreshMode::Ui);}
}
pub fn editor_map_done(a: &mut App,id:&str, ok: bool) {
    if crate::library::library_map_done(a, id, ok) {
        return; // (the Library's Graph)
    }
    if let Some(s) = a.ed.as_deref_mut() {
        mapview_background_done(&mut s.map,id, ok);
    }
}
/// The Graph of all notebooks' layout child ended.
pub fn editor_map_all_done(a: &mut App, ok: bool) {
    if crate::library::library_map_all_done(a, ok) {
        return; // (the Library's Graph)
    }
    match a.ed.as_deref_mut() {
        Some(s) => mapview_all_background_done(&mut s.map, ok),
        // (no editor, no Graph: the graph-all.ini kept in memory is what the child wrote over)
        None if ok => mapview_forget_all_ini(),
        None => {}
    }
}
fn map_result(a: &mut App, result: MapResult) {
    match result {
        MapResult::Close => map_hide(a, true),
        MapResult::Open(pid) => {
            let to = ed_ref(a).nb.page_ids.iter().position(|&p| p == pid);
            map_hide(a, false);
            if let Some(i) = to {
                go_to_page_pushing(a, i as i32);
            } else {
                compose_all(a, RefreshMode::Full);
            }
        }
        MapResult::Target(target) => {map_hide(a,false);if !follow_target(a,target.as_str()){compose_all(a,RefreshMode::Full);}}
        MapResult::Repair(pid, seed) => {
            // A broken link's anchor: the page with the link, and its card (Change where it goes, Delete link).
            let to = ed_ref(a).nb.page_ids.iter().position(|&p| p == pid);
            map_hide(a, false);
            match to {
                Some(i) if i as i32 == ed_ref(a).index || go_to_page_pushing(a, i as i32) => {
                    if ed_ref(a).index != i as i32 {return;}
                    compose_all(a, RefreshMode::Full);
                    if ed_ref(a).nav.links_writable && nav_link_get(&ed_ref(a).nav, pid, seed).is_some() {card_show(a, &[seed]);}
                }
                _ => compose_all(a, RefreshMode::Full),
            }
        }
        MapResult::Pages => {map_hide(a,false);pages_show(a);}
        MapResult::List => {map_hide(a,false);map_show(a,false);with_map(a,|map,a,_,nav|mapview_set_list(map,a,nav));}
        MapResult::Rename(pid) => {
            let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
            let s = ed(a);
            if !s.nav.writable {
                return;
            }
            let name = nav_name(&s.nav, pid).as_bytes();
            keyboard_open(&mut s.map_kb, tr("Page name"), name);
            s.map_rename_pid = pid;
            s.map_kb.area = keyboard_modal_rect_docked(screen, 0);
            compose_all(a, RefreshMode::Ui);
        }
        MapResult::None => {}
    }
}
fn map_pointer(a: &mut App, phase: i32, ev: &InEvent) {
    if ed_ref(a).map_kb.open {
        let mut kb = std::mem::take(&mut ed(a).map_kb);
        let result = app_keyboard_pointer(a, &mut kb, phase, ev);
        let text = kb.text;
        ed(a).map_kb = kb;
        if result == KbResult::Done || result == KbResult::Cancel {
            keyboard_close(&mut ed(a).map_kb);
            if result == KbResult::Done {
                let s = ed(a);
                let pid = s.map_rename_pid;
                if nav_set_name(&mut s.nav, pid, text.as_str().trim()) == 0 && nav_save(s.nb.id.as_str(), &mut s.nav, &s.nb) == 0 {
                    s.changed = true;
                    let open = s.open_pid();
                    // (the node shows its new name: no toast, the owner, 2026-11-04; one left up from before goes)
                    with_map(a, |map, a, nb, nav| mapview_rebuild(map, a, nb, nav, open));
                    app_toast_dismiss(a);
                } else {
                    nav_load(s.nb.id.as_str(), &mut s.nav);
                    app_toast(a, tr("Couldn't save page name."), 3000);
                }
            }
            compose_all(a, RefreshMode::Ui);
        }
    } else {
        let result = with_map(a, |map, a, _, nav| mapview_pointer(map, a, nav, phase, ev));
        map_result(a, result);
    }
    if phase == 2 {
        ed(a).owner = Owner::None;
    }
}

// The same for the Links bar, with the notebook and its Nav that it reads.
fn with_lb<R>(a: &mut App, f: impl FnOnce(&mut LinkBar, &mut App, &NotebookInfo, &Nav) -> R) -> R {
    let s = ed(a);
    let mut lb = std::mem::take(&mut s.lb);
    let nb = std::mem::take(&mut s.nb);
    let nav = std::mem::take(&mut s.nav);
    let r = f(&mut lb, a, &nb, &nav);
    let s = ed(a);
    s.lb = lb;
    s.nb = nb;
    s.nav = nav;
    r
}

// The same for Link to... (the bar's frame in pick mode).
fn with_lt<R>(a: &mut App, f: impl FnOnce(&mut LinkBar, &mut App, &NotebookInfo, &Nav) -> R) -> R {
    let s = ed(a);
    let mut lt = std::mem::take(&mut s.lt);
    let nb = std::mem::take(&mut s.nb);
    let nav = std::mem::take(&mut s.nav);
    let r = f(&mut lt, a, &nb, &nav);
    let s = ed(a);
    s.lt = lt;
    s.nb = nb;
    s.nav = nav;
    r
}

// The same for the quick sizes card (it needs the App alone).
fn with_qs<R>(a: &mut App, f: impl FnOnce(&mut QuickSize, &mut App) -> R) -> R {
    let mut q = std::mem::take(&mut ed(a).qs);
    let r = f(&mut q, a);
    ed(a).qs = q;
    r
}

// The same for the long-press card.
fn with_card<R>(a: &mut App, f: impl FnOnce(&mut LinkCard, &mut App, &NotebookInfo, &Nav) -> R,
) -> R {
    let s = ed(a);
    let mut card = std::mem::take(&mut s.card);
    let nb = std::mem::take(&mut s.nb);
    let nav = std::mem::take(&mut s.nav);
    let r = f(&mut card, a, &nb, &nav);
    let s = ed(a);
    s.card = card;
    s.nb = nb;
    s.nav = nav;
    r
}

// The same for the Find sheet.
fn with_find<R>(a: &mut App, f: impl FnOnce(&mut FindSheet, &mut App, &NotebookInfo, &Nav) -> R,
) -> R {
    let s = ed(a);
    let mut find = std::mem::take(&mut s.find);
    let nb = std::mem::take(&mut s.nb);
    let nav = std::mem::take(&mut s.nav);
    let r = f(&mut find, a, &nb, &nav);
    let s = ed(a);
    s.find = find;
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
    // Anywhere on the screen, up to its very edges (fit_panel_to still sizes it with room to spare).
    let x = clampi(x, 0, a.w - w);
    let y = clampi(y, 0, a.h - h);
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
        self.hist.push(PageHistory { page_id: pid, h, page: None, to_list: false ,
        });
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
            if self.open {
                self.migrate_entry(e);
            }
        }
        Some(pid)
    }

    // The page of cache entry e, just read (its first visit this session): its links with no area get
    // the box of their ink, unless it was tried already, is read-only (a newer Grimoire's page may not
    // show all its strokes, so its ink box could be wrong) or links.ini cannot be written. Tried
    // whatever it had: a link drawn on it later this session is never migrated behind its back.
    fn migrate_entry(&mut self, e: usize) {
        let pid = self.hist[e].page_id;
        if self.migrate_done.binary_search(&pid).is_ok() || !self.nav.links_writable {
            return;
        }
        if let Some(p) = self.hist[e].page.as_deref() {
            if !p.readonly && nav_links_legacy(&self.nav, pid) > 0 && nav_link_migrate(&mut self.nav, p, pid) > 0 {
                self.links_migrated();
            }
        }
        self.migrate_tried(pid);
    }

    fn links_migrated(&mut self) {
        if !self.links_dirty {
            // (the flush waits for a pause: the idle ticks migrate a page each, and save once)
            self.links_try_at = now_ms().wrapping_add(LINKS_FLUSH_MS);
        }
        self.links_dirty = true;
        self.boxes.stale = true;
    }

    fn migrate_tried(&mut self, pid: u32) {
        // (room for every page was made at the open; a page added since may need more. Without it the
        // page is not remembered, and the idle ticks stop: see migrate_room)
        if self.migrate_done.len() < self.migrate_done.capacity() || self.migrate_done.try_reserve(16).is_ok() {
            if let Err(i) = self.migrate_done.binary_search(&pid) {
                self.migrate_done.insert(i, pid);
            }
        }
    }

    // Whether every page tried can be remembered: the idle ticks migrate only then, so a page that
    // could not be remembered is never read again and again.
    fn migrate_room(&mut self) -> bool {
        let needed = self.nb.page_ids.len().saturating_sub(self.migrate_done.len());
        self.migrate_done.try_reserve(needed).is_ok()
    }

    // The first page in notebook order with a link that has no area and that this session has not
    // tried, as an index; -1 for none.
    fn migrate_next(&self) -> i32 {
        for (i, &pid) in self.nb.page_ids.iter().enumerate() {
            if nav_links_legacy(&self.nav, pid) > 0 && self.migrate_done.binary_search(&pid).is_err() {
                return i as i32;
            }
        }
        -1
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
/// Select mode is on: the tool button shows the lasso, selected (no pen is the one in use while it is).
const TB_SELECT: i32 = -3;

// While the drawing tools draw white ink, the pen in use shows its icon hollow: white on white
// paper shows nothing, and without a mark the pen would seem broken.
fn draws_white(st: &Settings, t: ToolKind) -> bool {
    st.white && t == st.current_tool && (t.0 as u32) < TOOL_COUNT as u32 && t != TOOL_ERASER
}

// A pen's button on the panel (the tool button's, or a pinned one's), selected while it is the pen
// in use, hollow while that draws white.
fn draw_pen_button(c: &mut Canvas, st: &Settings, s: &mut EditorState, r: Rect, t: ToolKind) {
    if s.sel.state != SelState::Off {
        ui_button(c, r, ui_tool_icon(t.0), "", ButtonStyle::Plain);
    } else if draws_white(st, t) {
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
    s.panel_cells.clear();
    s.white_mark = false;
    s.tool_button_shows = TB_NONE;
    if s.collapsed {
        ui_frame(c, p, ui_px(22.0), 255, true, 0);
        ui_frame(c, p, ui_px(22.0), 0, false, ui_px(6.0));
        let mut ir = rect_inflate(p, -ui_px(20.0));
        ir.h -= ui_px(12.0);
        if s.sel.state != SelState::Off {
            ui_icon(c, IconKind::Lasso, ir, 0);
        } else if draws_white(st, st.current_tool) {
            ui_icon_hollow(c, ui_tool_icon(st.current_tool.0), ir, 0, 255);
            s.white_mark = true;
        } else {
            ui_icon(c, ui_tool_icon(st.current_tool.0), ir, 0);
        }
        // Expand hint in the corner.
        ui_icon(
            c,
            IconKind::Expand,
            Rect { x: p.x + p.w - ui_px(40.0), y: p.y + p.h - ui_px(40.0), w: ui_px(30.0), h: ui_px(30.0) ,
            },
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
    let grip = Rect { x: p.x, y: p.y, w: p.w - ui_px(62.0), h: head_h() ,
    };
    let col = Rect { x: p.x + p.w - ui_px(64.0), y: p.y + sc(12.0), w: ui_px(54.0), h: sc(56.0) ,
    };
    ui_icon(c, IconKind::Grip, Rect { x: p.x + ui_px(10.0), y: p.y + sc(14.0), w: ui_px(52.0), h: sc(52.0) ,
        }, 0,
    );
    ui_icon(c, IconKind::Collapse, col, 0);
    hits_add(&mut s.hits, grip, P_GRIP, 0);
    hits_add(&mut s.hits, rect_inflate(col, ui_px(10.0)), P_COLLAPSE, 0);
    s.panel_cells.extend([grip, rect_inflate(col, ui_px(10.0))]);
    let mut y = p.y + head_h();
    canvas_hline(c, p.x + ui_px(6.0), p.x + p.w - ui_px(6.0), y - ui_px(4.0), ui_px(3.0), 0,
    );
    y += gap();
    // The drawing tool (tap: the picker), the pens pinned under it, and the eraser. When the
    // drawing tool is one of the pins, its pin shows it: the tool button shows a grid instead.
    let dt = button_tool(st);
    let dt_pinned = settings_pinned(st, dt);
    let dt_sel = !dt_pinned && st.current_tool == dt;
    let rt = Rect { x, y, w, h: btn_h() ,
    };
    // (Select mode on: the tool button shows the lasso, selected, so the panel says the pen draws a lasso;
    // a tap still opens the picker, where a pen leaves Select mode)
    let selecting = s.sel.state != SelState::Off;
    let dt_sel = dt_sel && !selecting;
    if selecting {
        ui_button(c, rt, IconKind::Lasso, "", ButtonStyle::Selected);
        s.tool_button_shows = TB_SELECT;
    } else if dt_pinned {
        ui_button(c, rt, IconKind::Grid, "", ButtonStyle::Plain);
        s.tool_button_shows = TB_GRID;
    } else {
        draw_pen_button(c, st, s, rt, dt);
        s.tool_button_shows = dt.0;
    }
    // A small corner mark says "more tools here" (the grid says it by itself).
    if !dt_pinned || selecting {
        canvas_disc_aa(
            c,
            (rt.x + rt.w - ui_px(16.0)) as f32,
            (rt.y + rt.h - ui_px(16.0)) as f32,
            ui_px(7.0) as f32,
            if dt_sel || selecting { 255 } else { 0 },
            1.0,
        );
    }
    hits_add(&mut s.hits, rt, P_TOOLPICK, 0);
    s.panel_cells.push(rt);
    y += btn_h() + gap();
    for i in 0..npins(st) {
        let rp = Rect { x, y, w, h: btn_h() ,
        };
        let t = st.pins[i as usize];
        draw_pen_button(c, st, s, rp, t);
        hits_add(&mut s.hits, rp, P_TOOL, t.0);
        s.panel_cells.push(rp);
        y += btn_h() + gap();
    }
    let re = Rect { x, y, w, h: btn_h() ,
    };
    ui_button(c, re, IconKind::Eraser, "", if selecting { ButtonStyle::Plain } else { tool_style(st, TOOL_ERASER) });
    hits_add(&mut s.hits, re, P_TOOL, TOOL_ERASER.0);
    s.panel_cells.push(re);
    y += btn_h() + gap();
    let shapes = Rect { x, y, w, h:btn_h() };
    ui_button(c, shapes, IconKind::None, "", ButtonStyle::Plain);
    let icon = Rect { x:x+ui_px(24.0), y:y+ui_px(16.0), w:w-ui_px(48.0), h:btn_h()-ui_px(32.0) };
    font_draw_icon(c, FontFace::Regular, if st.shapes {0xF0831} else {0xF0832}, icon, 0);
    if !st.shapes { canvas_slash(c, icon, ui_px(4.0)); }
    hits_add(&mut s.hits, shapes, P_SHAPES, 0);
    s.panel_cells.push(shapes);
    y += btn_h() + gap();
    let pen = Rect { x, y, w, h: btn_h() };
    ui_button(c, pen, IconKind::Tune, "", ButtonStyle::Plain);
    hits_add(&mut s.hits, pen, P_PENSET, 0);
    s.panel_cells.push(pen);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6.0), ui_px(3.0), 160);
    y += div_h();
    let h = if s.open { s.cur_hist() } else { None };
    let cu = h.is_some_and(|i| history_can_undo(&s.hist[i].h));
    let cr = h.is_some_and(|i| history_can_redo(&s.hist[i].h));
    let ru = Rect { x, y, w, h: btn_h() ,
    };
    ui_button(c, ru, IconKind::Undo, "", if cu { ButtonStyle::Plain } else { ButtonStyle::Disabled },
    );
    if cu {
        hits_add(&mut s.hits, ru, P_UNDO, 0);
    }
    s.panel_cells.push(ru);
    y += btn_h() + gap();
    let rr = Rect { x, y, w, h: btn_h() ,
    };
    ui_button(c, rr, IconKind::Redo, "", if cr { ButtonStyle::Plain } else { ButtonStyle::Disabled },
    );
    if cr {
        hits_add(&mut s.hits, rr, P_REDO, 0);
    }
    s.panel_cells.push(rr);
    y += btn_h() + gap();
    let rc = Rect { x, y, w, h: btn_h() ,
    }; // clear the page right away (no confirmation)
    let can_clear = s.open_page().is_some_and(|p| !p.strokes.is_empty())
        || (s.open && nav_links_of(&s.nav,s.open_pid(),&mut 0)>0);
    ui_button(c, rc, IconKind::Broom, "", if can_clear { ButtonStyle::Plain } else { ButtonStyle::Disabled },
    );
    if can_clear {
        hits_add(&mut s.hits, rc, P_CLEAR, 0);
    }
    s.panel_cells.push(rc);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6.0), ui_px(3.0), 160);
    y += div_h();
    let rp = Rect { x, y, w, h: btn_h() ,
    };
    ui_button(c, rp, IconKind::Prev, "", if s.index > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled },
    );
    if s.index > 0 {
        hits_add(&mut s.hits, rp, P_PREV, 0);
    }
    s.panel_cells.push(rp);
    y += btn_h() + gap();
    let mut lab = String::new();
    // (a page the Links bar is looking at shows its own number)
    csnprintf!(&mut lab, 32, "%d / %d", if s.peek.active { s.peek.index } else { s.index } + 1, s.nb.npages());
    let lr = Rect { x: x - ui_px(6.0), y, w: w + ui_px(12.0), h: label_h() ,
    };
    ui_frame(c, lr, ui_px(10.0), 0, false, ui_px(2.0)); // a tappable chip: opens Pages
    let fit = g_fit();
    font_draw_centered(c, FontFace::Bold, ui_small_size() * (if fit.scale < 1.0 { fit.scale } else { 1.0 }), lr, &lab, 0,
    );
    hits_add(&mut s.hits, lr, P_PAGES, 0);
    s.panel_cells.push(lr);
    y += label_h() + gap();
    let last = s.index >= s.nb.npages() - 1;
    let rn = Rect { x, y, w, h: btn_h() ,
    };
    ui_button(c, rn, if last { IconKind::Plus } else { IconKind::Next }, "", ButtonStyle::Plain,
    );
    hits_add(&mut s.hits, rn, P_NEXT, 0);
    s.panel_cells.push(rn);
    y += btn_h() + gap();
    // Add a link directly; the upward two-finger swipe opens the Links bar.
    let rl = Rect { x, y, w, h: btn_h() };
    ui_button(c, rl, IconKind::LinkPlus, "", ButtonStyle::Plain);
    hits_add(&mut s.hits, rl, P_LINKS, 0);
    s.panel_cells.push(rl);
    y += btn_h() + gap();
    canvas_hline(c, x, x + w, y + div_h() / 2 - ui_px(6.0), ui_px(3.0), 160);
    y += div_h();
    // Paper, Display, Settings and Library (icons only, like the rest of the panel).
    let rest: [(IconKind, i32); 4] = [
        (IconKind::Paper, P_PAPER),
        (IconKind::Brightness, P_DISPLAY),
        (IconKind::Gear, P_SETTINGS),
        (IconKind::Library, P_LIBRARY),
    ];
    for (icon, id) in rest {
        let r = Rect { x, y, w, h: btn_h() ,
        };
        ui_button(c, r, icon, "", ButtonStyle::Plain);
        hits_add(&mut s.hits, r, id, 0);
        s.panel_cells.push(r);
        y += btn_h() + gap();
    }
}

/// The expanded panel's button at (x, y): the one there, or for a finger landing between buttons (a
/// gap, a divider, the frame's margin beside them) the nearest one, so the column has no dead strips
/// (the nearest by rows first: beside a button is that button). None on a greyed-out button, as ever.
fn panel_hit_at(s: &EditorState, x: f32, y: f32) -> Option<Hit> {
    if let Some(&h) = hits_find(&s.hits, x, y) {
        return Some(h);
    }
    let (xi, yi) = (x as i32, y as i32);
    if s.collapsed || !rect_contains(s.panel, xi, yi) || s.panel_cells.iter().any(|&r| rect_contains(r, xi, yi)) {
        return None;
    }
    let off = |lo: i32, len: i32, v: i32| (lo - v).max(v - (lo + len - 1)).max(0);
    let cell = s.panel_cells.iter().min_by_key(|r| (off(r.y, r.h, yi), off(r.x, r.w, xi)))?;
    hits_find(&s.hits, (cell.x + cell.w / 2) as f32, (cell.y + cell.h / 2) as f32).copied()
}

/// The runner's name for a panel button (`tap panel NAME`, `editor.panel_at.X.Y`): grip, collapse,
/// expand, tool-picker, eraser, tool:NAME (a pinned pen, NAME as `editor.tool` names tools), undo,
/// redo, clear, previous, pages (the page chip), next, links, paper, display, pen-feel, library. "" for a
/// hit that is not one of the panel's.
fn panel_name(h: &Hit) -> String {
    match h.id {
        P_GRIP => "grip".into(),
        P_COLLAPSE => "collapse".into(),
        P_EXPAND => "expand".into(),
        P_TOOLPICK => "tool-picker".into(),
        P_TOOL if h.arg == TOOL_ERASER.0 => "eraser".into(),
        P_TOOL => match app_probe_tool(ToolKind(h.arg)) {
            Some(n) => cformat!("tool:%s", n),
            None => cformat!("tool:%d", h.arg),
        },
        P_UNDO => "undo".into(),
        P_REDO => "redo".into(),
        P_CLEAR => "clear".into(),
        P_SHAPES => "shapes".into(),
        P_PREV => "previous".into(),
        P_PAGES => "pages".into(),
        P_NEXT => "next".into(),
        P_LINKS => "links".into(),
        P_PAPER => "paper".into(),
        P_DISPLAY => "display".into(),
        P_PENSET => "pen-feel".into(),
        P_SETTINGS => "settings".into(),
        P_LIBRARY => "library".into(),
        _ => String::new(),
    }
}

/// The centre of the panel's button NAME (panel_name's names), None for no such button now: no
/// editor open, or the button is not there (a disabled Undo has no hit, as a human tap finds
/// nothing). The hits go stale after a stroke until the settle, and a human tap settles first, so
/// this does too.
pub fn editor_panel_point(a: &mut App, name: &str) -> Option<(f32, f32)> {
    if crate::library::library_settings_active(a){return crate::library::library_control_point(a,name);}
    let s = a.ed.as_deref()?;
    if a.scr != ScreenId::Editor || !s.open {
        return None;
    }
    if name=="map.current" {return mapview_node_point(&s.map,&s.open_pid().to_string());}
    if let Some(n)=name.strip_prefix("map."){return mapview_control_point(&s.map,n);}
    if name.starts_with("music."){return musicbuild_control_point(&s.mb,name);}
    // The Paper sheet's tiles and buttons: paper.<kind number>, paper.custom, paper.more, paper.photos,
    // paper.cancel (only while it shows: its Cancel is among the hits).
    if let Some(n)=name.strip_prefix("paper.") {
        if !s.modal_hits.h.iter().any(|h|h.id==P_PICK_CANCEL) {return None;}
        let (id,arg)=match n {"custom"=>(P_PICK_CUSTOM,None),"more"=>(P_PICK_MORE,None),"photos"=>(P_PICK_PHOTOS,None),
            "cancel"=>(P_PICK_CANCEL,None),k=>(P_PICK_PAPER,Some(k.parse::<i32>().ok()?))};
        return s.modal_hits.h.iter().find(|h|h.id==id&&arg.is_none_or(|v|h.arg==v)).map(|h|((h.r.x+h.r.w/2) as f32,(h.r.y+h.r.h/2) as f32));
    }
    if let Some(n)=name.strip_prefix("pages."){return pagesview_control_point(&s.pv,n);}
    // The toolbar's count pad's keys while it is up: addpad.key.0 to addpad.key.9, .done, .del, .cancel, .dec, .inc.
    if let Some(k)=name.strip_prefix("addpad.key.") {
        if !s.add_kb.open {return None;}
        let code=match k {"done"=>KB_KEY_DONE,"del"=>KB_KEY_BKSP,"cancel"=>KB_KEY_CANCEL,"dec"=>KB_KEY_DEC,"inc"=>KB_KEY_INC,c if c.chars().count()==1=>c.chars().next()? as u32,_=>return None};
        let mut r=RECT_EMPTY;
        return keyboard_key_rect(&s.add_kb,code,&mut r).then(||((r.x+r.w/2) as f32,(r.y+r.h/2) as f32));
    }
    if let Some(n)=name.strip_prefix("quicksize."){return quicksize_control_point(&s.qs,n);}
    if let Some(n)=name.strip_prefix("display."){return dispmenu_control_point(&s.dm,n);}
    if name=="elsewhere.current" {return s.destination.point(&format!("book.{}",s.nb.id));}
    if let Some(n)=name.strip_prefix("elsewhere."){return s.destination.point(n);}
    if let Some(control)=name.strip_prefix("select."){return sel_control_point(s,control);}
    if let Some(control)=name.strip_prefix("bar."){return linkbar_control_point(&s.lb,control);}
    if let Some(control)=name.strip_prefix("linkto."){return linkbar_control_point(&s.lt,control);}
    if s.tools_open {
        let target=if name=="picker.pin-mode" {Some((P_PIN_MODE,0))}
            else if name=="picker.select" {Some((P_PICK_SELECT,0))}
            else if let Some(control)=name.strip_prefix("shapes.") {
                match control {"on"=>Some((P_SHAPES,0)),"hold"=>Some((P_SHAPE_HOLD,0)),"quads"=>Some((P_SHAPE_QUADS,0)),
                    "round"=>Some((P_SHAPE_ROUND,0)),"links"=>Some((P_SHAPE_LINKS,0)),"min"=>Some((P_SHAPE_MIN,0)),"corner"=>Some((P_SHAPE_CORNER,0)),"done"=>Some((P_SHAPE_DONE,0)),_=>None}
            }
            else if let Some(i)=name.strip_prefix("picker.ink.") {i.parse::<i32>().ok().map(|i|(P_PICK_INK,i))}
            else if let Some(tool)=name.strip_prefix("picker.tool:") {
                let action=shortcut_action(&format!("tool:{tool}"));
                (action>=SHORTCUT_TOOL_BASE && action<SHORTCUT_TOOL_BASE+TOOL_COUNT as i32).then_some((P_PICK_TOOL,action-SHORTCUT_TOOL_BASE))
            }else{None};
        if let Some((id,arg))=target {return s.tools_hits.h.iter().find(|h|h.id==id&&h.arg==arg).map(|h|((h.r.x+h.r.w/2) as f32,(h.r.y+h.r.h/2) as f32));}
    }
    if s.panel_stale {settle_now(a);}
    let s = ed_ref(a);
    let h = s.hits.h.iter().rev().find(|h| panel_name(h) == name)?;
    Some(((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32))
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
            bgimage_render(&mut full, s.nb.id.as_str(), k, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
                },
            );
            if bgimage_take_missing() {
                note_white(s, bgimage_generation());
            }
        } else {
            paper_render(&mut full, k, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
                }, dpi,
            );
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
        // The last tile: Custom music shows this page's layout, or the last one built (also for a
        // newer layout this build cannot draw, which is then not selected either).
        let custom = i == PICKER_PAPERS;
        let k = if custom {
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
        if let Some(pv) = paper_preview(s, dpi, k) {
            canvas_blit_fit(c, pv, tr_);
        }
        let sel = if custom { page_custom } else { page_k == Some(k) };
        ui_frame(
            c,
            rect_inflate(tr_, if sel { ui_px(10.0) } else { 0 }),
            ui_px(12.0),
            0,
            false,
            if sel { ui_px(9.0) } else { ui_px(3.0) },
        );
        // (Custom music... as in New notebook, which says its room.)
        let label = if custom { tr("Custom music...") } else { paper_label(k) };
        paper_tile_label(c, tr_, gap_, label, sel);
        hits_add(
            &mut s.modal_hits,
            Rect { x: tr_.x, y: tr_.y, w: g.tw, h: g.th + ui_px(90.0) ,
            },
            if custom { P_PICK_CUSTOM } else { P_PICK_PAPER },
            k.0 as i32,
        );
    }
    // Along the bottom, right to left: Cancel, Photos... (the Templates sheet on the photos) and More
    // layouts... (the Templates sheet), selected while the page is on a picture.
    let (bw, bh, bgap) = (ui_px(380.0), ui_px(140.0), ui_px(30.0));
    let by = in_.y + in_.h - bh;
    let cancel = Rect { x: in_.x + in_.w - bw, y: by, w: bw, h: bh };
    let photos = Rect { x: cancel.x - bgap - bw, y: by, w: bw, h: bh };
    let more = Rect { x: photos.x - bgap - bw, y: by, w: bw, h: bh };
    // TRANSLATORS: the paper sheets' button that opens the Templates sheet, of pictures copied into
    // grimoire/templates to write on as paper (a button: 348 px at 44 px, or two lines at 34 px).
    ui_button(c, more, IconKind::None, tr("More layouts..."), if page_picture { ButtonStyle::Selected } else { ButtonStyle::Plain });
    hits_add(&mut s.modal_hits, more, P_PICK_MORE, if page_picture { page_k.unwrap().0 as i32 } else { PAPER_BLANK.0 as i32 });
    // TRANSLATORS: the editor's Paper sheet's button that opens the Photos sheet (photos copied into
    // grimoire/photos), beside More layouts... (its room).
    ui_button(c, photos, IconKind::None, tr("Photos..."), ButtonStyle::Plain);
    hits_add(&mut s.modal_hits, photos, P_PICK_PHOTOS, 0);
    ui_button(c, cancel, IconKind::None, tr("Cancel"), ButtonStyle::Plain);
    hits_add(&mut s.modal_hits, cancel, P_PICK_CANCEL, 0);
}

/// Display data only: no mutable editor state escapes to the status renderer.
pub fn editor_status_text(a: &App) -> Option<(String, String, String)> {
    let s = a.ed.as_deref()?;
    if !s.open { return None; }
    let pid = *s.nb.page_ids.get(s.index as usize)?;
    let section = nav_section_of(&s.nav, &s.nb, s.index);
    let name = s.nb.page_ids.get(section.max(0) as usize).map(|p| nav_name(&s.nav, *p)).unwrap_or("");
    let page = format!("{} / {}  {}", s.index + 1, s.nb.page_ids.len(), name);
    let outgoing = s.nav.links.iter().filter(|l| l.pid == pid).count();
    let incoming = s.nav.links.iter().filter(|l| nav_link_page(l, &s.nb) == s.index).count();
    Some((s.nb.name.to_string(), page, format!("{} {}  {} {}", tr("From"), outgoing, tr("To"), incoming),
    ))
}

/// Refit after the status strip is enabled or removed, in either orientation.
pub fn editor_status_changed(a: &mut App) {
    if a.scr == ScreenId::Editor && a.ed.as_deref().is_some_and(|s| s.open) {
        view_reset(a);
        compose_all(a, RefreshMode::Full);
    } else if let Some(s) = a.ed.as_deref_mut() { s.view_w = 0; }
}

/// A read-only live preview of the corner while adjusting it in Display.
pub fn editor_status_preview(a:&App)->bool {
    if crate::library::library_status_preview(a) {return true;}
    a.scr==ScreenId::Editor && !crate::library::library_settings_active(a)
        && a.ed.as_deref().is_some_and(|s|s.open && s.dm.open)
}

// ---- view ----

fn view_identity(a: &App) -> bool {
    let s = ed_ref(a);
    s.zoom == 1.0 && s.ox == 0.0 && s.oy == 0.0 && s.pgw == a.w && s.pgh == a.h
}

// Whether the view (the zoom and its offsets) moved from `was`, by more than rounding: a zoom step at the most zoom
// or the least, or a scroll at the end of the first or the last page, leaves it where it was, and draws nothing (so
// a finger shape bound to it says nothing either, fs_fire).
fn view_moved(s: &EditorState, was: (f32, f32, f32)) -> bool {
    (s.zoom - was.0).abs() > was.0 * 1e-5 || (s.ox - was.1).abs() > 0.01 || (s.oy - was.2).abs() > 0.01
}
fn view_clamp(a: &mut App) {
    let top = status_reserved(a);
    let (aw, ah) = (a.w, a.h - top);
    let s = ed(a);
    s.oy += top as f32 / s.zoom;
    s.zoom = clampf(s.zoom, s.zoom_min, s.zoom_max);
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
    s.oy -= top as f32 / s.zoom;
    touch_context(a);
}

/// What touchgest needs of the editor to tell a pan from a swipe, and to let a pinch zoom: whether the
/// page scrolls on each axis, whether a pinch may zoom, whether gestures are on and whether a keyboard
/// is up, and where a finger may be the pen's writing hand (the page's ink, off its controls). Pushed
/// whenever any of these may have changed (the end of view_clamp, editor_event, editor_shortcut_at,
/// editor_tick and editor_enter), cheap: it stores a small struct. With a surface in
/// front that does not pan the page (the Pages view, the sheets), a two-finger drag is always a swipe
/// candidate: a pan there would move nothing.
fn touch_context(a: &App) {
    if crate::library::library_settings_active(a) {
        input_touch_context(crate::library::library_touch_view(a));return;
    }
    let Some(s) = a.ed.as_deref() else { return };
    let pans = s.open && !sheet_open(s);
    let keyboard_open = s.map_kb.open || s.pv.kb.open || s.find.open || s.lm.kb.open || s.card.kb.open || s.add_kb.open;
    // The pen's hand rests on the page alone: nothing over it, and never on its panel or the strip.
    let ink = pans && a.scr == ScreenId::Editor && !keyboard_open && matches!(s.lm.state, LmState::Off | LmState::Armed | LmState::Lasso);
    let strip = if matches!(s.lm.state, LmState::Armed | LmState::Lasso) { s.lm.strip } else { RECT_EMPTY };
    let keys = [&s.map_kb, &s.pv.kb, &s.find.kb, &s.lm.kb, &s.card.kb, &s.add_kb].into_iter().find(|k| k.open).map_or(RECT_EMPTY, |k| k.area);
    input_touch_context(TgView {
        scrolls_v: pans && (a.h - status_reserved(a)) as f32 / s.zoom < s.pgh as f32 - 0.5,
        scrolls_h: pans && a.w as f32 / s.zoom < s.pgw as f32 - 0.5,
        zoom_ok: a.settings.pinch_zoom
            && !s.map_kb.open
            && !s.add_kb.open
            && !(s.paper_open || s.mb.open || s.ts.open || s.lb.open || s.find.open || s.lt.open || s.card.open || s.qs.open),
        gestures_on: a.settings.gestures,
        record: false,
        keyboard_open,
        ink,
        controls: [s.panel, strip],
        keys,
    });
}

/// Where a page rests on a screen: (zoom, the least zoom, the most). The page fills the screen's width
/// (100% upright for a page of this screen's size, the whole page for a notebook from another Scribe,
/// theirs are 3:4 too); sideways with `fit` (settings.landscape_fit) the whole page shows instead,
/// centred, and two fingers swipe anywhere. The least is the whole page either way, and the most four
/// times the width's fit (the same expression and order it always was, so it is bit-identical).
fn view_rest(aw: i32, ah: i32, pgw: i32, pgh: i32, fit: bool) -> (f32, f32, f32) {
    let zoom_min = (aw as f32 / pgw as f32).min(ah as f32 / pgh as f32);
    let zoom = if fit && aw > ah { zoom_min } else { aw as f32 / pgw as f32 };
    let zoom_max = (aw as f32 / pgw as f32) * 4.0;
    (zoom, zoom_min, zoom_max)
}

// The page at rest (view_rest): the width across upright, sideways the width too (pan with two fingers;
// pinching out shows the whole page) or, with landscape_fit, the whole page.
fn view_reset(a: &mut App) {
    let top = status_reserved(a);
    let (aw, ah) = (a.w, a.h - top);
    let fit = a.settings.landscape_fit || top > 0;
    let s = ed(a);
    let (mut zoom, zoom_min, zoom_max) = view_rest(aw, ah, s.pgw, s.pgh, fit);
    if top > 0 { zoom = zoom_min; }
    s.zoom_min = zoom_min;
    s.zoom_max = zoom_max;
    s.zoom_default = zoom;
    s.zoom = zoom;
    s.ox = 0.0;
    s.oy = -(top as f32) / zoom;
    s.view_w = aw;
    s.view_h = ah + top;
    view_clamp(a);
    let s = ed(a);
    s.rest_ox = s.ox;
}

/// settings.landscape_fit was flipped (the Display sheet's row, or the action): the page rests as the
/// setting says from now on. In the editor, sideways, the view is laid out again at once, even pinched in
/// (a Full refresh, as a turn gives); upright it changes nothing, and in the Library the editor lays its
/// view out when it is next entered (view_w 0 is a view set up for no screen).
pub fn editor_fit_changed(a: &mut App, in_editor: bool) {
    if a.ed.is_none() {
        return;
    }
    if in_editor && a.scr == ScreenId::Editor && ed_ref(a).open {
        if a.w > a.h {
            view_reset(a);
            compose_all(a, RefreshMode::Full);
        }
    } else {
        ed(a).view_w = 0;
    }
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
    rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    )
}

fn zoomed(a: &App) -> bool {
    let s = ed_ref(a);
    // (The whole page sideways rests centred across, at a negative offset: rest_ox, 0 everywhere else.)
    (s.zoom - s.zoom_default).abs() > 0.01 || s.ox != s.rest_ox || s.oy + status_reserved(a) as f32 / s.zoom > 0.5
}

// Draws the visible page into a screen rect: bilinear sampling, a gray desk outside the page. The
// whole screen of it (a pinch, a zoomed page) is about 100 ms on the Kindle: the pens are read every
// 64 rows (work_pump, common.h).
fn blit_view(a: &mut App, r: Rect) {
    let top = status_reserved(a);
    let r = rect_intersect(r, Rect { x: 0, y: top, w: a.w, h: a.h - top ,
        },
    );
    let App { screen, ed, .. } = a;
    let s = ed.as_deref().unwrap();
    blit_canvas(screen, &s.page_canvas, s.zoom, s.ox, s.oy, r);
}

// The same for any page canvas at any view: the open page's, or the one the Links bar shows.
fn blit_canvas(screen: &mut Canvas, pc: &Canvas, zoom: f32, ox: f32, oy: f32, r: Rect) {
    // (the C's static ix[8192], wx[8192])
    let mut ix = [0i32; 8192];
    let mut wx = [0u8; 8192];
    let mut r = r;
    if r.w > 8192 {
        r.w = 8192;
    }
    for i in 0..r.w {
        let fx = ((r.x + i) as f32 + 0.5) / zoom + ox - 0.5;
        let x = fx.floor() as i32;
        ix[i as usize] = x;
        wx[i as usize] = ((fx - x as f32) * 255.0) as u8;
    }
    for y in r.y..r.y + r.h {
        let dst = y as usize * screen.stride as usize;
        let fy = (y as f32 + 0.5) / zoom + oy - 0.5;
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

/// Where a page of pgw x pgh rests on an aw x ah screen (view_rest), centred as view_clamp centres it: what
/// the view_reset of that page would give, for the page the bar shows without making it the open one.
fn peek_view(aw: i32, ah: i32, pgw: i32, pgh: i32, fit: bool) -> (f32, f32, f32) {
    let (zoom, zoom_min, zoom_max) = view_rest(aw, ah, pgw, pgh, fit);
    let z = clampf(zoom, zoom_min, zoom_max);
    let mut vw = aw as f32 / z;
    let mut vh = ah as f32 / z;
    if (vw - pgw as f32).abs() < 0.01 {
        vw = pgw as f32;
    }
    if (vh - pgh as f32).abs() < 0.01 {
        vh = pgh as f32;
    }
    // (a view that holds the whole page on an axis is centred on it; else it starts at the top or the left)
    let ox = if vw >= pgw as f32 { -(vw - pgw as f32) / 2.0 } else { clampf(0.0, 0.0, pgw as f32 - vw) };
    let oy = if vh >= pgh as f32 { -(vh - pgh as f32) / 2.0 } else { clampf(0.0, 0.0, pgh as f32 - vh) };
    if (z - 1.0).abs() < 0.01 && pgw == aw && pgh == ah {
        return (1.0, 0.0, 0.0);
    }
    (z, ox, oy)
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
    Ink { label: nc_("ink", "Black"), white: false ,
    },
    // TRANSLATORS: the other choice of the tool picker's ink row: the pens draw white ink, which
    // takes away the ink under it and keeps the paper's lines (426 px at 44 px).
    Ink { label: nc_("ink", "White"), white: true ,
    },
];
const N_INKS: i32 = K_INKS.len() as i32;

fn draw_tools(a: &mut App) {
    ed(a).tools_pressed = None;
    if ed_ref(a).shapes_menu { draw_shape_menu(a); return; }
    let (aw, ah) = (a.w, a.h);
    let App { screen: c, settings: st, ed, .. } = a;
    let s = ed.as_deref_mut().unwrap();
    hits_clear(&mut s.tools_hits);
    let bw = ui_px(300.0);
    let bh = ui_px(150.0);
    let g = ui_px(16.0);
    let cols = 3;
    let rows = (N_DRAW_TOOLS as i32 + cols - 1) / cols;
    let gridw = 2 * ui_px(520.0) + g;
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
    let hw = clampi(lx + (if lw > dw { lw } else { dw }) + pad, ui_px(210.0), w / 2,
    );
    let hb = Rect { x: x + w - ui_px(30.0) - hw, y: y + ui_px(14.0), w: hw, h: hh ,
    };
    ui_button(c, hb, IconKind::None, "", if s.pin_mode { ButtonStyle::Primary } else { ButtonStyle::Plain },
    );
    ui_icon(
        c,
        if s.pin_mode { IconKind::Check } else { IconKind::Pin },
        Rect { x: hb.x + ui_px(14.0), y: hb.y + (hh - hi) / 2, w: hi, h: hi ,
        },
        0,
    );
    ui_text(
        c,
        FontFace::Caps,
        hs,
        hs,
        Rect { x: hb.x + lx, y: hb.y, w: hb.w - lx - pad, h: hh ,
        },
        if s.pin_mode { done } else { pin },
        1,
        true,
        0,
    );
    hits_add(&mut s.tools_hits, hb, P_PIN_MODE, 0);
    // Select (Select mode: the lasso), left of Pin while the pens are not being pinned.
    let mut title_end = hb.x;
    if !s.pin_mode {
        // TRANSLATORS: the tool picker's button beside Pin that starts Select mode (the lasso that selects
        // ink to copy, cut, paste or move); 116 px at 34 px, the button widens for a longer word.
        let sel = c_("panel", "Select");
        let sw = clampi(lx + font_measure(FontFace::Caps, hs, sel) + pad, ui_px(210.0), w / 3);
        let sb = Rect { x: hb.x - ui_px(16.0) - sw, y: hb.y, w: sw, h: hh };
        ui_button(c, sb, IconKind::None, "", ButtonStyle::Plain);
        ui_icon(c, IconKind::Lasso, Rect { x: sb.x + ui_px(14.0), y: sb.y + (hh - hi) / 2, w: hi, h: hi }, 0);
        ui_text(c, FontFace::Caps, hs, hs, Rect { x: sb.x + lx, y: sb.y, w: sb.w - lx - pad, h: hh }, sel, 1, true, 0);
        hits_add(&mut s.tools_hits, sb, P_PICK_SELECT, 0);
        title_end = sb.x;
    }
    let mut title = String::new();
    if s.pin_mode {
        // TRANSLATORS: the tool picker's title while it chooses the pens pinned to the toolbar under the
        // tool button: a tap pins a pen or unpins it; %d is how many can be (3). About 700 px at 44 px
        // in bold, or two lines at 34 px. Normally the title is Tools.
        csnprintf!(&mut title, 160, tr("Pin to the toolbar (up to %d)"), PANEL_PINS_MAX as i32);
    } else {
        // TRANSLATORS: the title of the editor's tool picker.
        csnprintf!(&mut title, 160, "%s", tr("Tools"));
    }
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size(),
        ui_small_size(),
        Rect { x: x + ui_px(30.0), y, w: title_end - x - ui_px(50.0), h: ui_px(100.0) ,
        },
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
        ui_button(c, r, ui_tool_icon(t.0), name, if sel { ButtonStyle::Selected } else { ButtonStyle::Plain },
        );
        hits_add(&mut s.tools_hits, r, P_PICK_TOOL, t.0);
    }
    // The ink row, out of use in Pin mode (the pins are pens, whatever they draw).
    let iy = y + ui_px(100.0) + rows * bh + (rows - 1) * g + g;
    let iw = (gridw - (N_INKS - 1) * g) / N_INKS;
    s.tools_ink_drawn = None;
    for i in 0..N_INKS {
        let r = Rect { x: x + ui_px(30.0) + i * (iw + g), y: iy, w: iw, h: inkh ,
        };
        let ink = &K_INKS[i as usize];
        let stl = if s.pin_mode {
            ButtonStyle::Disabled
        } else if st.white == ink.white {
            s.tools_ink_drawn = Some(ink.white);
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

fn draw_shape_menu(a: &mut App) {
    let (aw,ah)=(a.w,a.h);
    let App {screen:c,settings:st,ed,..}=a;
    let s=ed.as_deref_mut().unwrap(); hits_clear(&mut s.tools_hits);
    let rows=[
        (P_SHAPES,tr(if st.shapes {n_("Smart shapes: On")}else{n_("Smart shapes: Off")})),
        (P_SHAPE_HOLD,tr(shape_hold_label(st.shape_hold_ms))),
        (P_SHAPE_QUADS,tr(match st.shape_quads {0=>n_("Four sides: Uneven allowed"),2=>n_("Four sides: Squares only"),_=>n_("Four sides: Rectangles")})),
        (P_SHAPE_ROUND,tr(match st.shape_round {0=>n_("Round shapes: As drawn"),2=>n_("Round shapes: Circles only"),_=>n_("Round shapes: Prefer circles")})),
        (P_SHAPE_CORNER,""),
        (P_SHAPE_MIN,""),
        (P_SHAPE_LINKS,tr(if st.shape_links {n_("Snap link areas: On")}else{n_("Snap link areas: Off")})),
        (P_SHAPE_DONE,tr("Done")),
    ];
    let n=rows.len() as i32;
    let (w,rh,g)=(ui_px(1116.0).min(aw-ui_px(20.0)),ui_px(120.0),ui_px(16.0));
    let h=ui_px(140.0)+n*rh+(n-1)*g;
    let mut x=s.panel.x-w-ui_px(20.0);
    if x<ui_px(10.0) {x=s.panel.x+s.panel.w+ui_px(20.0);}
    x=clampi(x,ui_px(10.0),aw-w-ui_px(10.0));
    let y=clampi(s.tools_y,ui_px(10.0),ah-h-ui_px(10.0));
    s.tools_rect=Rect{x,y,w,h};
    ui_frame(c,s.tools_rect,ui_px(22.0),255,true,0);
    ui_frame(c,s.tools_rect,ui_px(22.0),0,false,ui_px(6.0));
    font_draw_left(c,if ui_theme()==UiTheme::Gothic{FontFace::Title}else{FontFace::Bold},ui_title_size(),Rect{x:x+ui_px(30.0),y:y+ui_px(15.0),w:w-ui_px(60.0),h:ui_px(85.0)},tr("Smart shapes"),0);
    for (i,(id,label)) in rows.into_iter().enumerate() {
        let r=Rect{x:x+ui_px(30.0),y:y+ui_px(110.0)+i as i32*(rh+g),w:w-ui_px(60.0),h:rh};
        if shape_slider_id(id) {
            // (a slider's hit is its row and the gaps beside it, so a finger a little off still drags it)
            let band=Rect{y:r.y-g/2,h:r.h+g,..r};
            shape_slider_draw(c,id,band,st);hits_add(&mut s.tools_hits,band,id,0);continue;
        }
        ui_button(c,r,IconKind::None,label,ButtonStyle::Plain);hits_add(&mut s.tools_hits,r,id,0);
    }
}

// Whether a Smart shapes menu hit is one of its sliders.
fn shape_slider_id(id: i32) -> bool {
    id==P_SHAPE_CORNER || id==P_SHAPE_MIN
}
// The Smart shapes menu's sliders: Corner roundness (snapped polygons' corners, sharp at 0, as a
// percentage of the largest rounding that fits) and Smallest shape (the smallest stroke a shape
// snaps; 0 mm: any size). Both have the same text part, wide enough for either one's label and
// widest value, so their tracks line up; the track starts the knob's radius past the value, so
// the knob at 0 never covers it.
fn shape_slider(id: i32, band: Rect, st: &Settings) -> Slider {
    let mut sl=if id==P_SHAPE_CORNER {
        // TRANSLATORS: the Smart shapes menu's slider that rounds the corners of the rectangles,
        // triangles and other polygons a stroke snaps to (its value is "Sharp" or a percentage).
        // At 44 px left of its value; label and value share up to 64% of the 1032 px row.
        Slider{min:0.0,max:100.0,step:5.0,value:st.shape_corner as f32,label:tr("Corner roundness"),unit:"%",..Default::default()}
    } else {
        // TRANSLATORS: the Smart shapes menu's slider, the smallest stroke that becomes a shape (its
        // value is a size in mm, 3 to 40).
        Slider{min:SHAPE_MIN_LO_MM,max:SHAPE_MIN_HI_MM,step:1.0,value:st.shape_min_mm as f32,label:tr("Smallest shape"),unit:"mm",..Default::default()}
    };
    let row=rect_inflate(band,-ui_px(12.0));
    let ts=ui_text_size();
    let need=|label: &str, values: &[&str]| font_measure(FontFace::Regular,ts,label)+ui_px(16.0)
        +values.iter().map(|v| font_measure(FontFace::Bold,ts,v)).max().unwrap_or(0);
    let text_w=need(tr("Corner roundness"),&[c_("corners","Sharp"),&shape_corner_text(100)])
        .max(need(tr("Smallest shape"),&["40 mm"]));
    sl.text_w=clampi(text_w,row.w*42/100,row.w*64/100);
    ui_slider_layout(&mut sl,row);
    let clear=(ui_slider_knob_radius()+ui_px(10.0)-ui_px(26.0)).max(0);
    sl.track.x+=clear;sl.track.w-=clear;
    sl
}
fn shape_corner_text(pct: i32) -> String {
    let mut t=String::new();
    // TRANSLATORS: Corner roundness's value, a percentage of the most rounding a shape can take
    // ("25%"; in Spanish "25 %", the usual spacing), right of its slider's name in bold at 44 px.
    csnprintf!(&mut t,16,tr("%d%%"),pct);
    t
}
fn shape_slider_draw(c: &mut Canvas, id: i32, band: Rect, st: &Settings) {
    let sl=shape_slider(id,band,st);
    canvas_fill(c,band,255);
    let text=if id==P_SHAPE_CORNER {
        // TRANSLATORS: Corner roundness at 0: snapped polygons keep sharp corners.
        if st.shape_corner<=0 {c_("corners","Sharp").to_string()} else {shape_corner_text(st.shape_corner)}
    } else {
        format!("{} mm",st.shape_min_mm)
    };
    ui_slider_draw(c,&sl,Some(&text));
}
/// A press on a Smart shapes slider at x (screen px), its hit `h`: once the press has come over the
/// track (from the knob's radius before its start on) the knob follows it, set from x and clamped
/// to the ends; a press on the name and value alone moves nothing. The setting changes at once; it
/// is saved at the lift.
fn shape_slider_follow(a: &mut App, h: Hit, x: f32) {
    let mut sl=shape_slider(h.id,h.r,&a.settings);
    let s=ed(a);
    if !s.tools_slider && x<(sl.track.x-ui_slider_knob_radius()) as f32 {return;}
    s.tools_slider=true;
    if !ui_slider_set_from_x(&mut sl,x) {return;}
    if h.id==P_SHAPE_CORNER {a.settings.shape_corner=sl.value as i32;} else {a.settings.shape_min_mm=sl.value as i32;}
    shape_slider_draw(&mut a.screen,h.id,h.r,&a.settings);
    app_invalidate(a,h.r,RefreshMode::Fast);
}

// ---- the eye: where a finger tap follows a link ----

/// The open page's link boxes, made again if anything changed since they were made (LinkBoxes).
fn boxes_get(a: &mut App) {
    let dpi = a.dpi;
    let s = ed(a);
    if !s.boxes.stale {
        return;
    }
    s.boxes.stale = false;
    s.boxes.n = 0;
    let pid = s.open_pid();
    s.boxes.doc = match s.open_page() {
        Some(page) if pid != 0 => doclinks_boxes(&s.dl, page, pid),
        _ => Vec::new(),
    };
    let mut links = [NavLink::default(); LB_MAX];
    let mut areas = [RECT_EMPTY; LB_MAX];
    // (exactly link_under's minimum: the eye promises what nav_link_at hits)
    let n = match s.open_page() {
        Some(page) if pid != 0 => nav_link_areas(&s.nav, page, pid, NAV_LINK_MIN_MM * dpi / 25.4, &mut links, &mut areas,
        ) as usize,
        _ => 0,
    };
    let mut entries = [LbEntry::default(); LB_MAX];
    let mut numbers = [0u8; LB_MAX];
    linkbar_group(&s.nb, &links[..n], &areas[..n], dpi, &mut entries, &mut numbers,
    );
    s.boxes.link = links;
    s.boxes.area = areas;
    s.boxes.entry = numbers;
    s.boxes.n = n;
}

/// A page rect on the screen as the view maps it, to the pixel (page_to_screen pads it for repaints).
fn box_screen(s: &EditorState, p: Rect) -> Rect {
    let x0 = ((p.x as f32 - s.ox) * s.zoom).round() as i32;
    let y0 = ((p.y as f32 - s.oy) * s.zoom).round() as i32;
    let x1 = (((p.x + p.w) as f32 - s.ox) * s.zoom).round() as i32;
    let y1 = (((p.y + p.h) as f32 - s.oy) * s.zoom).round() as i32;
    Rect { x: x0, y: y0, w: (x1 - x0).max(1), h: (y1 - y0).max(1) ,
    }
}

/// The link outlines' band (the eye's boxes and snapped outlines, the area waiting for its target, the lasso):
/// 6 px of a 2 px checkerboard at 300 dpi (2.0.0-rc.12; they were 2 px dots 8 px apart, which e-ink showed faint).
fn link_band() -> i32 {
    ui_px(6.0) // (three dither cells: the grain shows)
}

/// Where an outline around sr draws: a snapped outline's band is centred on its path, so half the band
/// and the halo stand outside the area.
fn eye_reach(_a: &App, sr: Rect) -> Rect {
    rect_inflate(sr, link_band() / 2 + ui_px(1.0) + 1)
}

fn draw_outline(c:&mut Canvas,outline:LinkOutline,source:Rect,target:Rect,view:(f32,f32,f32)) {
    let (zoom,ox,oy)=view;
    let points:Vec<_>=outline.points(source,target).iter().map(|&(x,y)|((x-ox)*zoom,(y-oy)*zoom)).collect();
    canvas_dither_poly(c,&points,link_band(),ui_px(1.0));
}

/// Fine dotted outlines over the page and under the panel, without number badges.
/// Only on the screen (never in the page canvas, so never in a thumbnail, the PDF, the share page or the sleep
/// image), 1-bit, so DU and GL16 show it alike. At most LB_MAX boxes, each tested against r.
fn draw_areas(a: &mut App, r: Rect) {
    if !a.settings.link_areas {
        ed(a).boxes.drawn = 0;
        return;
    }
    // A peek is display-only: outline its links at its own viewport without committing it.
    let (links,areas,n,zoom,ox,oy,docs)=if ed_ref(a).peek.active {
        let s=ed_ref(a);let pk=s.peek;
        let mut links=[NavLink::default();LB_MAX];let mut areas=[RECT_EMPTY;LB_MAX];
        let page=s.hist.iter().find(|h|h.page_id==pk.pid).and_then(|h|h.page.as_deref()).or(s.ccache[pk.slot].page.as_deref());
        let n=page.map_or(0,|p|nav_link_areas(&s.nav,p,pk.pid,NAV_LINK_MIN_MM*a.dpi/25.4,&mut links,&mut areas)as usize);
        let docs=page.map_or_else(Vec::new,|p|doclinks_boxes(&s.dl,p,pk.pid));
        let top=status_reserved(a);let (zoom,ox,oy)=peek_view(a.w,a.h-top,s.pgw,s.pgh,a.settings.landscape_fit||top>0);
        (links,areas,n,zoom,ox,oy-top as f32/zoom,docs)
    } else {
        boxes_get(a);let s=ed_ref(a);(s.boxes.link,s.boxes.area,s.boxes.n,s.zoom,s.ox,s.oy,s.boxes.doc.clone())
    };
    ed(a).boxes.drawn = n;
    ed(a).boxes.doc_drawn = docs.len();
    let (thick, halo) = (link_band(), ui_px(1.0));
    let clip = a.screen.clip;
    canvas_set_clip(&mut a.screen, r);
    for i in 0..n {
        let p=areas[i];
        let x0=((p.x as f32-ox)*zoom).round()as i32;let y0=((p.y as f32-oy)*zoom).round()as i32;
        let x1=(((p.x+p.w)as f32-ox)*zoom).round()as i32;let y1=(((p.y+p.h)as f32-oy)*zoom).round()as i32;
        let sr=Rect::new(x0,y0,(x1-x0).max(1),(y1-y0).max(1));
        if !rect_overlaps(r, eye_reach(a, sr)) {
            continue;
        }
        if links[i].outline.n>0 {draw_outline(&mut a.screen,links[i].outline,links[i].area,p,(zoom,ox,oy));}
        else {canvas_dither_rect(&mut a.screen,sr,thick,halo);}
    }
    // The document links' boxes: dotted (a thinner band than the user's), unnumbered, only where r is.
    let thin=(thick/2).max(2);
    for p in docs {
        let x0=((p.x as f32-ox)*zoom).round()as i32;let y0=((p.y as f32-oy)*zoom).round()as i32;
        let x1=(((p.x+p.w)as f32-ox)*zoom).round()as i32;let y1=(((p.y+p.h)as f32-oy)*zoom).round()as i32;
        let sr=Rect::new(x0,y0,(x1-x0).max(1),(y1-y0).max(1));
        if !rect_overlaps(r,rect_inflate(sr,thin/2+ui_px(1.0)+1)) {continue;}
        canvas_dither_rect(&mut a.screen,sr,thin.min((sr.w.min(sr.h)/2).max(1)),halo);
    }
    a.screen.clip = clip;
}

/// Where the eye draws anything now (empty with none): the refresh of turning it on or off.
fn eye_bounds(a: &mut App) -> Rect {
    boxes_get(a);
    let mut u = RECT_EMPTY;
    for i in 0..ed_ref(a).boxes.n {
        let s = ed_ref(a);
        let sr = box_screen(s, s.boxes.area[i]);
        u = rect_union(u, eye_reach(a, sr));
    }
    for i in 0..ed_ref(a).boxes.doc.len() {
        let s = ed_ref(a);
        let sr = box_screen(s, s.boxes.doc[i]);
        u = rect_union(u, eye_reach(a, sr));
    }
    rect_intersect(u, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    )
}

/// The eye and its bar button update together, including any page being previewed. A single
/// composition avoids split refreshes and restores anything the previous overlay covered.
fn areas_toggle(a: &mut App, _button: Rect) {
    a.settings.link_areas = !a.settings.link_areas;
    settings_save(&a.settings);
    compose_all(a, RefreshMode::Ui);
}

// Rebuild a screen rect from the layers.
fn compose(a: &mut App, r: Rect) {
    if crate::library::library_settings_active(a) {crate::library::library_settings_redraw(a);return;}
    if ed_ref(a).destination.open {with_modal(a,|s|&mut s.destination,|p,a|p.draw(a));return;}
    if ed_ref(a).map.open {
        map_draw(a, r);
        return;
    }
    if ed(a).pv.open && !ed_ref(a).pv.overlay {
        // the Pages view covers everything (but the Find sheet opened over it)
        if ed_ref(a).find.open {
            // Only where r is, as over the page: the sheet's typing refreshes its parts one by one, and a screen
            // left holding the view where the keys are would show it there at the next key's refresh.
            let clip = a.screen.clip;
            canvas_set_clip(&mut a.screen, r);
            with_pv(a, pagesview_draw);
            with_find(a, |f, a, nb, nav| findsheet_draw(f, a, nb, nav, r));
            a.screen.clip = clip;
            return;
        }
        with_pv(a, pagesview_draw);
        return;
    }
    a.toast_floor = 0;
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    );
    if rect_is_empty(r) {
        return;
    }
    let top = status_reserved(a);
    canvas_fill(&mut a.screen, rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: top ,
            },
        ), 255,
    );
    let s = ed_ref(a);
    if s.peek.active {
        // The page the bar is looking at, at the view it rests in on this screen; the open page is not touched.
        let slot = s.peek.slot;
        let (zoom, ox, mut oy) = peek_view(a.w, a.h - top, s.pgw, s.pgh, a.settings.landscape_fit || top > 0,
        );
        oy -= top as f32 / zoom;
        let r = rect_intersect(r, Rect { x: 0, y: top, w: a.w, h: a.h - top ,
            },
        );
        let App { screen, ed, w: aw, .. } = a;
        let pc = &ed.as_deref().unwrap().ccache[slot].canvas;
        if zoom == 1.0 && ox == 0.0 && oy == 0.0 && pc.w == *aw && pc.h == screen.h {
            canvas_blit(screen, pc, r, r.x, r.y);
        } else {
            blit_canvas(screen, pc, zoom, ox, oy, r);
        }
    } else if view_identity(a) {
        let App { screen, ed, .. } = a;
        canvas_blit(screen, &ed.as_deref().unwrap().page_canvas, r, r.x, r.y);
    } else {
        blit_view(a, r);
    }
    draw_areas(a, r);
    draw_lasso(a, r);
    sel_draw_halo(a, r);
    draw_area_wait(a, r);
    draw_card_border(a, r);
    if rect_overlaps(r, ed(a).panel) && !ed(a).live_no_panel {
        // Only the part of the panel inside r: the page was blitted there alone, and the rest of the panel (and
        // whatever menu is over it) is on the screen as it was; drawn again, the panel would lay itself over
        // that menu, and over itself (its edges are blended with what is under them).
        let clip = a.screen.clip;
        canvas_set_clip(&mut a.screen, r);
        draw_panel(a);
        a.screen.clip = clip;
    }
    draw_menus(a, r);
    sel_draw(a, r);
}

/// Every menu, sheet and bar that is up, over the page and the panel, in the order they stack: Paper, Templates,
/// Display, Custom music, the tool picker, Find with its keyboard, and the Links bar over everything; each where
/// it meets r. The panel is under all of them, and only ever over the notebook page.
fn draw_menus(a: &mut App, r: Rect) {
    if ed_ref(a).pv.open && ed_ref(a).pv.overlay {
        with_pv(a,|pv,a,_,_|pagesview_draw_overlay(pv,a));
        return;
    }
    let s = ed_ref(a);
    if s.paper_open && rect_overlaps(r, s.paper_rect) {
        draw_paper_sheet(a);
    }
    let s = ed_ref(a);
    if s.ts.open && rect_overlaps(r, s.ts.rect) {
        with_modal(a, |s| &mut s.ts, tplsheet_draw);
    }
    let s = ed_ref(a);
    if s.dm.open && rect_overlaps(r, s.dm.rect) {
        with_modal(a, |s| &mut s.dm, dispmenu_draw);
    }
    let s = ed_ref(a);
    if s.mb.open && rect_overlaps(r, s.mb.rect) {
        with_modal(a, |s| &mut s.mb, musicbuild_draw);
    }
    let s = ed_ref(a);
    if s.tools_open && rect_overlaps(r, s.tools_rect) {
        draw_tools(a);
    }
    // The Find sheet's card of rows and keyboard, where r meets them.
    let s = ed_ref(a);
    if s.find.open {
        with_find(a, |f, a, nb, nav| findsheet_draw(f, a, nb, nav, r));
    }
    // The Links bar, over everything (laid out again as it is drawn: a turn changes the screen it is on).
    let s = ed_ref(a);
    if s.lb.open {
        let (aw, ah) = (a.w, a.h);
        let rect = {
            let lb = &mut ed(a).lb;
            linkbar_layout(lb, aw, ah);
            lb.rect
        };
        if rect_overlaps(r, rect) {
            let s = ed_ref(a);
            let peek = if s.peek.active { s.peek.index + 1 } else { 0 };
            with_lb(a, |lb, a, nb, nav| linkbar_draw(lb, a, nb, nav, peek));
        }
        a.toast_floor = rect.y;
    }
    // The link-making strip, where the bar draws (the bar is never up with it).
    let st = ed_ref(a).lm.state;
    if st == LmState::Armed || st == LmState::Lasso {
        draw_strip(a, r);
    }
    // Link to..., where the bar draws (never up with it), laid out again as it is drawn (a turn).
    let s = ed_ref(a);
    if s.lt.open && !s.lt.hidden {
        let (aw, ah) = (a.w, a.h);
        let (top_y, bottom_y) = (dock_top_y(a), dock_bottom_y(a));
        let (rect, top) = {
            let lt = &mut ed(a).lt;
            let top = lt.top;
            linkbar_layout_pick(lt, aw, ah, top, top_y, bottom_y);
            (lt.rect, top)
        };
        if rect_overlaps(r, rect) {
            with_lt(a, |lt, a, nb, nav| linkbar_draw(lt, a, nb, nav, 0));
        }
        if !top {
            a.toast_floor = rect.y;
        }
    }
    // New page...'s keyboard, over it.
    let s = ed_ref(a);
    if s.lm.state == LmState::Naming && s.lm.kb.open {
        let screen = Rect { x: 0, y: 0, w: a.w, h: a.h ,
        };
        let dock = s.lm.kb.dock_y;
        let area = keyboard_modal_rect_docked(screen, dock);
        if rect_overlaps(r, area) {
            let App { screen: c, ed, .. } = a;
            keyboard_draw(&mut ed.as_deref_mut().unwrap().lm.kb, c, screen);
        }
        if dock == 0 {
            a.toast_floor = area.y;
        }
    }
    // The toolbar's count pad, along the bottom.
    let s = ed_ref(a);
    if s.add_kb.open {
        let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
        let area = keyboard_area(&s.add_kb, screen);
        if rect_overlaps(r, area) {
            let App { screen: c, ed, .. } = a;
            keyboard_draw(&mut ed.as_deref_mut().unwrap().add_kb, c, screen);
        }
        a.toast_floor = area.y;
    }
    // The quick sizes card, over the page and the panel (nothing else is up with it).
    let s = ed_ref(a);
    if s.qs.open && rect_overlaps(r, s.qs.rect) {
        let clip = a.screen.clip;
        canvas_set_clip(&mut a.screen, r);
        with_qs(a, quicksize_draw);
        a.screen.clip = clip;
    }
    // The long-press card, above everything (the bar included), and Rename's keyboard over it; the card is
    // left out while that keyboard covers part of it.
    let s = ed_ref(a);
    if card_shows(s) && rect_overlaps(r, s.card.rect) {
        let clip = a.screen.clip;
        canvas_set_clip(&mut a.screen, r);
        with_card(a, linkcard_draw);
        a.screen.clip = clip;
    }
    let s = ed_ref(a);
    if s.card.open && s.card.kb.open {
        let screen = Rect { x: 0, y: 0, w: a.w, h: a.h ,
        };
        let dock = s.card.kb.dock_y;
        let area = keyboard_modal_rect_docked(screen, dock);
        if rect_overlaps(r, area) {
            let App { screen: c, ed, .. } = a;
            keyboard_draw(&mut ed.as_deref_mut().unwrap().card.kb, c, screen);
        }
        if dock == 0 {
            a.toast_floor = area.y;
        }
    }
}

/// The panel drawn again (a pen picked, the panel moved, folded or refit): its place composed from the layers, the
/// page, the panel and every menu that is up over it where they meet it. So never over a sheet, a keyboard or
/// the Links bar, and not at all over the Pages view, which covers the screen.
fn draw_panel_over(a: &mut App) {
    if ed_ref(a).pv.open {
        return;
    }
    let p = ed_ref(a).panel;
    compose(a, p);
}

/// Live view: where the panel is, for its layer (app.rs, live_push_screen), which holds the screen there as the
/// Kindle shows it: the panel, and whatever menu, sheet or bar is over a part of it or all of it, drawn over it
/// there too. So the browser shows the toolbar wherever the Kindle does, beside a menu as under none, and a menu
/// over it as the Kindle does (2.0.0, issue #16: the layer was empty while any menu was up, so the browser lost
/// the toolbar beside the Display sheet until it closed). None where the panel is not drawn (no notebook, the
/// Pages view, the Graph or the notebooks' browser over the screen) and while Settings is over the notebook,
/// whose screen, the panel's strip beside it included, the frame holds whole (editor_live_hole).
pub fn editor_live_panel(a: &App) -> Rect {
    if crate::library::library_settings_active(a) {return RECT_EMPTY;}
    match a.ed.as_deref() {
        Some(s) if s.open && !s.pv.open && !s.map.open && !s.destination.open => s.panel,
        _ => RECT_EMPTY,
    }
}

/// Live view: where the screen holds the panel, which the frame is pushed without (editor_live_under_panel): the
/// panel's rect, with a menu over it or not (the tiles hold the page and the menu there, never the panel, for a
/// browser that shows no toolbar), and none where the Pages view covers the screen.
pub fn editor_live_hole(a: &App) -> Rect {
    if crate::library::library_settings_active(a) {return RECT_EMPTY;}
    match a.ed.as_deref() {
        Some(s) if s.open && !s.pv.open => s.panel,
        _ => RECT_EMPTY,
    }
}

/// Live view's chip on a notebook page, `size` px square: beside the panel, level with its header (its collapsed box
/// as a whole), on its left, or on its right where the panel is too near the screen's left edge to leave room;
/// none (empty) with a menu, sheet, bar or the Pages view in front, which draw over the panel. It is an
/// overlay, so it moves with the panel at the next flush.
pub fn editor_live_chip(a: &App, size: i32) -> Rect {
    if crate::library::library_settings_active(a) {return RECT_EMPTY;}
    let Some(s) = a.ed.as_deref() else { return RECT_EMPTY ;
    };
    if !s.open || sheet_open(s) {
        return RECT_EMPTY;
    }
    let (p, gap) = (s.panel, ui_px(8.0));
    let level = if s.collapsed { box_() } else { head_h() };
    let y = clampi(p.y + (level - size) / 2, 0, (a.h - size).max(0));
    let left = p.x - gap - size;
    let x = if left >= 0 { left } else { clampi(p.x + p.w + gap, 0, (a.w - size).max(0)) };
    Rect { x, y, w: size, h: size ,
    }
}

/// Live view: push (live_push) of the screen with r (inside the panel and push) as it is without the
/// panel, in one push, a.screen left as it was: the browser has the page under the panel, and the
/// panel as its own layer (app.rs, live_push_screen). One push: a tile viewer woken between two
/// would send the panel in the page's tiles for a batch.
pub fn editor_live_under_panel(a: &mut App, r: Rect, push: Rect) {
    let mut keep = Canvas::default();
    if canvas_init(&mut keep, r.w, r.h) != 0 {
        live_push(&a.screen, push); // (the panel in the page's tiles, rather than none of push)
        return;
    }
    canvas_blit(&mut keep, &a.screen, r, 0, 0);
    let floor = a.toast_floor;
    ed(a).live_no_panel = true;
    compose(a, r);
    ed(a).live_no_panel = false;
    a.toast_floor = floor;
    live_push(&a.screen, push);
    canvas_blit(&mut a.screen, &keep, Rect { x: 0, y: 0, w: r.w, h: r.h ,
        }, r.x, r.y,
    );
    canvas_free(&mut keep);
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
    compose(a, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    );
    app_invalidate_all(a, m);
}

fn repaint_panel(a: &mut App, m: RefreshMode) {
    draw_panel_over(a);
    let p = ed(a).panel;
    app_invalidate(a, p, m);
}

// ---- pages ----

fn save_thumb_from_canvas(s: &mut EditorState) {
    let mut t = Canvas::default();
    if canvas_init(&mut t, s.pgw / 3, s.pgh / 3) != 0 {
        return;
    }
    canvas_downsample_keep_lines(&mut t, &s.page_canvas, 0.35);
    if store_save_thumb(s.nb.id.as_str(), &t) == 0 {
        store_note_thumb_source(s.nb.id.as_str(), &first_page_source(&s.nb));
        s.thumb_dirty = false;
    }
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
// The idle autosave: save_page, marked so that it makes the Pages preview at most every 30 s.
fn autosave(a: &mut App) -> bool {
    save_page_for(a, Some(CacheSave::Idle))
}

/// The page is being left (a page turn or jump, the notebook closing): save_page, and the kept drawing of a dense
/// page made now whatever the idle window says, so its first revisit reads it back.
fn save_page_leaving(a: &mut App) -> bool {
    save_page_for(a, Some(CacheSave::Leave))
}

/// Any other save (Ctrl+S, the Pages view, Settings, a move or a delete of pages, the power press, a sleep) writes
/// the page file and nothing else: its preview and kept drawing wait for the idle save or for leaving.
fn save_page(a: &mut App) -> bool {
    save_page_for(a, None)
}

fn save_page_for(a: &mut App, caches: Option<CacheSave>) -> bool {
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
            app_toast(a, tr("Couldn't save page. Is the storage full?"), 5000,
            );
            return false;
        }
        if s.index == 0 {
            s.thumb_dirty = true; // written when leaving the page (it is slow)
        }
        let pid = s.open_pid();
        pagesview_page_saved(&mut s.pv, pid);
        s.caches_pending = true;
        s.page_dirty = false;
        s.info_dirty = true;
    }
    if let Some(why) = caches {
        save_page_caches(s, now_ms(), why);
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

/// When a save may make the open page's preview and kept drawing (save_page_caches).
#[derive(Clone, Copy, PartialEq)]
enum CacheSave {
    /// The idle autosave.
    Idle,
    /// The page is left.
    Leave,
}

/// How long idle cache writes are spaced, and how long the page must have been quiet for its kept drawing to be
/// written at an idle save: 30 s. (The simulator takes GRIMOIRE_CACHE_WINDOW_MS, so scenarios need not wait.)
fn cache_window_ms() -> u32 {
    #[cfg(feature = "sim")]
    if let Some(ms) = std::env::var("GRIMOIRE_CACHE_WINDOW_MS").ok().and_then(|v| v.parse().ok()) {
        return ms;
    }
    30_000
}

// Whether the open page's canvas is the page as it is on disk, whole: what a preview or a kept drawing may be
// made from (not a page changed since its file was written: page_is_its_file, nor one with its picture white).
fn caches_writable(s: &EditorState) -> bool {
    let whole = s.canvas_page == s.open_page_ref() && s.page_canvas.w == s.pgw && s.page_canvas.h == s.pgh
        && !s.page_canvas.px.is_empty() && !s.picture_missing;
    page_is_its_file(s) && whole && s.open_page().is_some_and(|p| !p.readonly)
}

// Whether editor_tick should save for the caches alone: some are still to make, nothing can have changed them
// since, the last write of one is a window ago and the page has been quiet that long. (Not while they cannot be
// made: a pending flag that outlived a canvas that is not whole, or a read-only page, ran the autosave at every
// pass of the main loop.)
fn caches_due(s: &EditorState, now: u32) -> bool {
    let window = cache_window_ms();
    s.caches_pending && caches_writable(s) && now.wrapping_sub(s.preview_saved_ms) >= window
        && now.wrapping_sub(s.last_up) >= window
}

// Cache work happens after saving, never as the pen moves, and each file only when it is not current. The idle
// save makes the Pages preview, at most once a window (30 s, across page turns and reopens too), and the kept
// drawing of a dense page only once the page has been quiet that long as well: a page written in bursts would
// otherwise write its whole drawing (3.4 MB) at every pause. Leaving writes the drawing at once, window or not
// (without it the first revisit draws all the ink again), and no preview: that is the previews job's. Every
// other save writes neither; they stay pending for one of these.
fn save_page_caches(s: &mut EditorState, now: u32, why: CacheSave) {
    if !caches_writable(s) {
        // (a read-only page, never written, has nothing pending)
        if s.open_page().is_some_and(|p| p.readonly) {
            s.caches_pending = false;
        }
        return;
    }
    let pid = s.open_pid();
    let layered = s.open_page().is_some_and(page_layered);
    let keep = store_page_dense(&s.nb, pid)
        || (s.full_ms.0 == pid && s.full_ms.1 >= store_keep_render_ms());
    let window = cache_window_ms();
    match why {
        CacheSave::Leave => {
            if keep && !store_page_render_kept(&s.nb, pid, s.pgw, s.pgh, layered) {
                store_save_page_render(&s.nb, pid, &s.page_canvas, layered);
                s.preview_saved_ms = now;
            }
        }
        CacheSave::Idle => {
            if !s.caches_pending || now.wrapping_sub(s.preview_saved_ms) < window {
                return;
            }
            // (a try that fails, a full store, counts too: it is tried again a window later, not at every pass)
            let mut tried = false;
            if !store_page_preview_ok(&s.nb, pid) {
                store_save_page_preview(&s.nb, pid, &s.page_canvas);
                tried = true;
            }
            if keep && !store_page_render_kept(&s.nb, pid, s.pgw, s.pgh, layered) && now.wrapping_sub(s.last_up) >= window {
                store_save_page_render(&s.nb, pid, &s.page_canvas, layered);
                tried = true;
            }
            if tried {
                s.preview_saved_ms = now;
            }
            s.caches_pending = !store_page_preview_ok(&s.nb, pid)
                || (keep && !store_page_render_kept(&s.nb, pid, s.pgw, s.pgh, layered));
        }
    }
}

// What the Library card is drawn from: page 1's id and its file's identity (store_note_thumb_source).
fn first_page_source(nb: &NotebookInfo) -> String {
    let pid = nb.page_ids[0];
    format!("{pid}:{}", store_page_source_identity(nb, pid).unwrap_or_default())
}

// A change on another page leaves the Library card alone, when the card was drawn from page 1 as it is now (what
// it was drawn from is kept beside it, store_note_thumb_source: a card of before, or one drawn from another page 1
// or from a page 1 since written, is drawn again, whatever it held at the open). A changed or reordered first page
// uses its kept drawing, then its current small preview, before falling back to drawing the strokes. (A card of
// no known source, one a version before 2.1.0 made, may show a cleared page's old picture, and the preview beside
// it may too: that one is drawn from the strokes, once, as every close did before.)
fn refresh_library_thumb(s: &mut EditorState, dpi: f32) {
    let source = first_page_source(&s.nb);
    let pid = s.nb.page_ids[0];
    let was = store_thumb_source(s.nb.id.as_str());
    if !s.thumb_dirty && was.as_deref() == Some(source.as_str()) {
        return;
    }
    let mut p = None;
    if s.load_page(0, &mut p) != 0 { return; }
    let Some(p) = p else { return; };
    let mut t = Canvas::default();
    if canvas_init(&mut t, s.pgw / 3, s.pgh / 3) != 0 { return; }
    let mut full = Canvas::default();
    let kept = canvas_init(&mut full, s.pgw, s.pgh) == 0
        && store_load_page_render(&s.nb, pid, s.pgw, s.pgh, &mut full, page_layered(&p));
    if kept {
        canvas_downsample_keep_lines(&mut t, &full, 0.35);
    } else if was.is_some() && store_load_page_preview(&s.nb, pid, &mut full) {
        canvas_downsample_keep_lines(&mut t, &full, 0.35);
    } else {
        page_render_thumb(&mut t, &p, dpi);
    }
    if store_save_thumb(s.nb.id.as_str(), &t) == 0 {
        store_note_thumb_source(s.nb.id.as_str(), &source);
        s.thumb_dirty = false;
    }
}

// Draws r of the open page into the page canvas. An image paper's picture that is not there (a
// template's, made in the background) draws white: noted, to be drawn once it is (editor_tick).
// ---- the base canvas and the steps' patches ----

/// The steps' patches keep at most this many bytes, the open page's newest first (an older step's
/// undo draws its region again, as every undo did before 2.0.0-rc.10).
const PATCH_CAP: usize = 24 << 20;

fn base_valid(s: &EditorState) -> bool {
    s.base_page != 0 && s.base_page == s.canvas_page && !s.base.px.is_empty()
        && s.base.w == s.page_canvas.w && s.base.h == s.page_canvas.h && s.base.stride == s.page_canvas.stride
}
/// base becomes a copy of the page canvas (the page drawn whole, or another page come up).
fn base_reset(s: &mut EditorState) {
    s.base_page = 0;
    if s.page_canvas.px.is_empty() || s.canvas_page == 0 {
        return;
    }
    if s.base.w != s.page_canvas.w || s.base.h != s.page_canvas.h || s.base.px.len() != s.page_canvas.px.len() {
        canvas_free(&mut s.base);
        if canvas_init(&mut s.base, s.page_canvas.w, s.page_canvas.h) != 0 || s.base.px.len() != s.page_canvas.px.len() {
            return;
        }
    }
    s.base.px.copy_from_slice(&s.page_canvas.px);
    s.base_page = s.canvas_page;
}
/// The page canvas changed over r outside a step (the page drawn again there): base follows.
fn base_follow(s: &mut EditorState, r: Rect) {
    if s.base_hold || !base_valid(s) {
        return;
    }
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: s.page_canvas.w, h: s.page_canvas.h });
    let stride = s.page_canvas.stride;
    for y in r.y..r.y + r.h {
        let at = (y * stride + r.x) as usize;
        s.base.px[at..at + r.w as usize].copy_from_slice(&s.page_canvas.px[at..at + r.w as usize]);
    }
}
/// A stroke or an eraser sweep begins: base holds the page as it is now (copied again if it does not
/// mirror this page), and the history's count of steps is noted.
fn step_begin(s: &mut EditorState, hold: bool) {
    if !base_valid(s) {
        base_reset(s);
    }
    s.base_hold = hold;
    s.step_pushes = s.cur_hist().map_or(0, |h| s.hist[h].h.pushes);
}
/// It ended, having changed the page canvas inside page rect r: the step it pushed, if any, keeps the
/// tiles of r that changed (its patch); base mirrors the page canvas again.
fn step_end(s: &mut EditorState, r: Rect) {
    s.base_hold = false;
    if !base_valid(s) {
        return;
    }
    if let Some(h) = s.cur_hist() {
        if s.hist[h].h.pushes != s.step_pushes && !s.picture_missing {
            if let Some(p) = patch_capture(&s.base, &s.page_canvas, r) {
                history_attach_patch(&mut s.hist[h].h, p);
                let total: usize = s.hist.iter().map(|e| history_patch_bytes(&e.h)).sum();
                if total > PATCH_CAP {
                    for (i, e) in s.hist.iter_mut().enumerate() {
                        if i != h {
                            history_trim_patches(&mut e.h, 0);
                        }
                    }
                    history_trim_patches(&mut s.hist[h].h, PATCH_CAP);
                }
            }
        }
    }
    base_follow(s, r);
}

fn draw_page(a: &mut App, r: Rect) {
    draw_page_in(a, r, None);
}
/// The page drawn again where the strokes just taken off it (removed[first..], r their boxes) drew:
/// over r, in the tiles they drew into (page_render_tiles), or all of r without the memory for the
/// mask. The rect drawn.
fn draw_page_without(a: &mut App, r: Rect, first: usize) -> Rect {
    let dpi = a.dpi;
    let s = ed(a);
    let Some(mut m) = tile_mask_new(s.page_canvas.w, s.page_canvas.h) else {
        draw_page(a, r);
        return r;
    };
    for st in &s.removed[first..] {
        stroke_mark_tiles(&mut m, st, dpi);
    }
    tile_mask_seal(&mut m);
    let drawn = rect_intersect(r, tile_mask_bounds(&m));
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: m.w, h: m.h });
    let boxed = if rect_is_empty(r) { 0 } else { ((r.x + r.w - 1) / MASK_TILE - r.x / MASK_TILE + 1) * ((r.y + r.h - 1) / MASK_TILE - r.y / MASK_TILE + 1) };
    s.erase_tiles.0 = s.erase_tiles.0.wrapping_add(tile_mask_count(&m, drawn));
    s.erase_tiles.1 = s.erase_tiles.1.wrapping_add(boxed as u32);
    draw_page_in(a, drawn, Some(Rc::new(m)));
    drawn
}
fn draw_page_in(a: &mut App, r: Rect, mask: Option<Rc<TileMask>>) {
    let dpi = a.dpi;
    let s = ed(a);
    bgimage_take_missing(); // (what drew white before)
    let ip = s.page_ix().unwrap();
    let page = s.hist[ip].page.as_deref().unwrap();
    match &mask {
        Some(m) => page_render_tiles(&mut s.page_canvas, page, r, m, dpi),
        None => page_render(&mut s.page_canvas, page, r, dpi),
    }
    let whole = mask.is_none() && r.x <= 0 && r.y <= 0 && r.x + r.w >= s.pgw && r.y + r.h >= s.pgh;
    if bgimage_take_missing() {
        s.picture_missing = true;
        s.picture_gen = bgimage_generation();
    } else if whole {
        s.picture_missing = false;
    }
    base_follow(s, r);
}

// Whether the page in memory is the page file on disk: what a drawing kept of the file (store_load_page_render,
// which goes by the file's stamp alone) may stand for. Not while it has a change not yet written (page_dirty), so
// whatever changes the page marks it changed (mark_changed) before it draws it again. Clear page and a new paper
// drew it first, and read back the drawing kept of the file as it was before the change: the old ink came back on
// the cleared page, the old paper under a new one, and the idle save kept that picture under the new file's stamp,
// with the Pages preview and the Library card made from it (2.0.0 to 2.0.1).
fn page_is_its_file(s: &EditorState) -> bool {
    !s.page_dirty
}

// A drawing 2.0.0 or 2.0.1 kept of the open page, current for its file or not, goes as the page is drawn whole from
// its strokes with nothing unsaved (never read since 2.1.0, store.rs RENDER_TAG_VERSION: it may show the page as it
// was before a Clear or a new paper), and the Pages preview and the Library card made beside it are made again from
// this drawing: the preview now (a view decodes it again: bgjob.rs PreviewCache goes by the preview file too), the
// card as page 1 is left (write_thumb_if_needed). Not from a drawing with its picture white: later, then. For
// render_page_full and the Links bar's commit of a page it drew.
fn retire_kept_before(s: &mut EditorState) {
    let pid = s.open_pid();
    if page_is_its_file(s) && !s.picture_missing && s.page_ix().is_some() && store_retire_page_render(&s.nb, pid) {
        store_save_page_preview(&s.nb, pid, &s.page_canvas);
        if s.index == 0 {
            s.thumb_dirty = true;
        }
    }
}

fn render_page_full(a: &mut App) {
    let s = ed(a);
    s.boxes.stale = true; // (a page drawn whole is another page, or one that changed whole)
    let r = Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
    };
    // A dense page's drawing kept from before, when it is current (store_load_page_render) and the page in
    // memory is the file it was kept of (page_is_its_file): read back in a fraction of a second instead of
    // drawing every stroke again.
    let pid = s.open_pid();
    let layered = s.open_page().is_some_and(page_layered);
    let kept = page_is_its_file(s) && s.page_ix().is_some() && !s.page_canvas.px.is_empty()
        && store_load_page_render(&s.nb, pid, s.pgw, s.pgh, &mut s.page_canvas, layered);
    s.page_kept = kept;
    if kept {
        s.picture_missing = false;
        s.full_ms = (pid, u32::MAX); // (a page that keeps its drawing goes on keeping it)
    } else {
        let started = now_ms();
        draw_page(a, r);
        let s = ed(a);
        s.full_ms = (pid, now_ms().wrapping_sub(started));
        retire_kept_before(s);
        // A page that took this long, as it is on disk (nothing unsaved), keeps its drawing at once:
        // the next open is quick even if it is only read, never written.
        if s.full_ms.1 >= store_keep_render_ms() && !s.page_dirty && !s.picture_missing && s.page_ix().is_some() {
            let layered = s.open_page().is_some_and(page_layered);
            store_save_page_render(&s.nb, pid, &s.page_canvas, layered);
        }
    }
    let s = ed(a);
    s.canvas_page = s.open_page_ref();
    base_reset(s);
    s.preview_saved_ms = now_ms().wrapping_sub(store_page_cache_age_ms(&s.nb, pid).min(cache_window_ms()));
    s.caches_pending = false;
    // An image paper's picture can have come since its paper alone was drawn (S->paper: a picture
    // made in the background draws white until it is there), and the page now shows it.
    if paper_is_image(s.open_page().unwrap().paper) {
        s.paper_stale = true;
    }
}

// TRANSLATORS: a toast (about 1700 px at 44 px in bold) while a dense page is drawn as it opens (seconds on a
// Kindle); it goes as the page shows.
const K_LOADING_PAGE: &str = n_("Loading page...");

/// render_page_full for a page that opens (a page turn, a jump, a notebook opening, the page left after a
/// deletion): one that takes seconds to draw (store_page_slow: a page file of 256 KB or more, the owner's
/// decision of 2026-11-04; it went by store_page_dense's 512 KB before) and has no current kept drawing to read
/// back says "Loading page..." first, on the panel before the drawing starts, and the toast goes once the page
/// is drawn. Never for a page that draws quickly.
fn render_page_open(a: &mut App) {
    let toast = loading_toast_due(a);
    if toast {
        app_toast(a, tr(K_LOADING_PAGE), 120_000);
        app_flush(a); // (on the panel now: the drawing holds the main loop)
        ed(a).loading_toasts += 1;
    }
    render_page_full(a);
    // (another toast that took its place stays)
    if toast && a.toast_visible && a.toast.as_bytes() == tr(K_LOADING_PAGE).as_bytes() {
        app_toast_dismiss(a);
    }
}

// Whether the open page takes seconds to draw and has no current kept drawing to come up from (page_is_its_file;
// a pen held on an action holds the panel, so no toast could show before the drawing: none then).
fn loading_toast_due(a: &App) -> bool {
    let s = ed_ref(a);
    if a.pen_action_down || s.page_ix().is_none() || s.page_canvas.px.is_empty() {
        return false;
    }
    let pid = s.open_pid();
    let layered = s.open_page().is_some_and(page_layered);
    !(page_is_its_file(s) && store_page_render_kept(&s.nb, pid, s.pgw, s.pgh, layered)) && store_page_slow(&s.nb, pid)
}

// A page from a newer Grimoire (or one this build cannot fully read) is shown as far as it can
// be and never changed: writing, erasing, clearing and paper changes are refused with this.
// TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines).
const K_READONLY_MSG: &str = n_("This page was made by a newer version, and will be viewed as read-only.");

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

impl EditorState {
    /// The page the editor leaves, onto the back list.
    fn back_push(&mut self, pid: u32) {
        if self.back.len() == BACK_MAX {
            self.back.remove(0);
        }
        self.back.push(pid);
    }
    /// Pages deleted since they were pushed are skipped: dropped from the list.
    fn back_prune(&mut self) {
        let ids: Vec<u32> = (0..self.nb.npages().max(0) as usize).map(|i| self.nb.page_ids[i]).collect();
        self.back.retain(|p| ids.contains(p));
    }
    /// The page on show is not a page to go back to: entries at the top of the list that are it (a link
    /// followed, then the pages turned back by hand) are dropped, so "Back to" never offers where the
    /// owner already is and the go-back key never does nothing.
    fn back_trim(&mut self) {
        let here = self.open_pid();
        while here != 0 && self.back.last() == Some(&here) {
            self.back.pop();
        }
    }
    /// Where the newest page on the back list that is still in the notebook (and not the one on show)
    /// is: its index and what it is called (its Index name, or "Page 3"). None while the list is empty.
    fn back_top(&mut self) -> Option<(i32, String)> {
        self.back_prune();
        self.back_trim();
        let pid = *self.back.last()?;
        let i = (0..self.nb.npages()).find(|&i| self.nb.page_ids[i as usize] == pid)?;
        let name = nav_name(&self.nav, pid);
        Some((i, if name.is_empty() { cformat!(tr("Page %d"), i + 1) } else { name.to_string() },
        ))
    }
    /// The open page's id, 0 with none.
    fn open_pid(&self) -> u32 {
        if self.open && self.index >= 0 && self.index < self.nb.npages() {
            self.nb.page_ids[self.index as usize]
        } else {
            0
        }
    }
}

/// Where a link into another notebook was followed from (2.0.0-beta.2): the notebook, its page, the view
/// on it (zoom and offset, for the screen size they were for) and its own back list, so Back returns to the
/// same page at the same zoom, and that notebook's Back goes on where it left off.
#[derive(Clone, Debug, Default)]
struct NotebookBack {
    id: FixedStr<STORE_ID_LEN>,
    pid: u32,
    zoom: f32,
    ox: f32,
    oy: f32,
    view_w: i32,
    view_h: i32,
    back: Vec<u32>,
}

/// go_to_page, leaving the page it left on the back list (when it went anywhere else).
fn go_to_page_pushing(a: &mut App, index: i32) -> bool {
    let s = ed(a);
    let (from, at) = (s.open_pid(), s.index);
    let ok = go_to_page(a, index);
    let s = ed(a);
    if ok && s.index != at && from != 0 {
        s.back_push(from);
    }
    ok
}

// Migrated link areas (PHASE2 5.4) into links.ini, the whole file: returns at once when none wait. A
// failure keeps them in memory, says so in the log once, and is tried again LINKS_FLUSH_MS later.
fn links_flush(a: &mut App) {
    let s = ed(a);
    if !s.open || !s.links_dirty || !s.nav.links_writable {
        return;
    }
    if nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) == 0 {
        s.links_dirty = false;
        return;
    }
    if !s.links_fail_logged {
        s.links_fail_logged = true;
        qlog!("index: link areas left for a later save");
    }
    s.links_try_at = now_ms().wrapping_add(LINKS_FLUSH_MS);
}

fn go_to_page(a: &mut App, index: i32) -> bool {
    let s = ed(a);
    card_drop(s);
    lm_drop(s); // (a link's area is drawn on the page Add a link was asked on; the screen is drawn whole below)
    s.settle = RECT_EMPTY; // the whole screen is about to be redrawn anyway
    s.panel_stale = false;
    if !save_page_leaving(a) {
        compose_all(a, RefreshMode::Full);
        return false;
    }
    links_flush(a);
    write_thumb_if_needed(a);
    let s = ed(a);
    let index = clampi(index, 0, s.nb.npages() - 1);
    let Some(np) = s.page_at(index) else {
        compose_all(a, RefreshMode::Full);
        app_toast(a, tr("Couldn't open page"), 3000);
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
    render_page_open(a);
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
    canvas_free(&mut s.base);
    s.base_page = 0;
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
    page_render(&mut c, p.as_deref().unwrap(), Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
        }, dpi,
    );
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
    pb_reset(s);
    if !s.open {
        return;
    }
    card_drop(s);
    lm_drop(s);
    s.sel = SelMode::default(); // (Select mode ends with the notebook)
    s.pv.move_undo = None; // (a page move is undone only while its notebook stays open)
    s.pv.add_undo = None; // (and a batch of new pages)
    s.pv.redo = None; // (and their Redo)
    keyboard_close(&mut s.add_kb); // (the toolbar's count pad: nothing added)
    s.add_quiet_until = None;
    s.recent = Vec::new();
    s.back = Vec::new();
    s.boxes.stale = true;
    bar_drop(s);
    find_drop(s);
    // Pages an older version had saved on blank paper got their pictures back as they opened: the
    // thumbnail and the PDF it drew of them are drawn again.
    if store_take_restored_count() > 0 {
        s.changed = true;
        if s.index == 0 {
            s.thumb_dirty = true;
        }
    }
    save_page_leaving(a);
    links_flush(a); // (before the PDF, which reads the areas)
    backlinks_flush();
    libindex_flush();
    write_thumb_if_needed(a);
    let dpi = a.dpi;
    let s = ed(a);
    if s.index != 0 && s.changed {
        refresh_library_thumb(s, dpi);
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
    s.dl = DocLinks::default();
    s.links_dirty = false; // (what a failed flush kept is left for this notebook's next open)
    s.migrate_done = Vec::new();
    s.open = false;
    store_take_restored_count(); // (page 1, read for the thumbnail above, was drawn as it is now)
    bgimage_forget(None); // the notebook's pictures (up to 3 pages' worth) and its failures
}

/// loads, switches to SCR_EDITOR
pub fn editor_open_notebook(a: &mut App, id: &str) {
    let requested=ed(a).requested_page.take();
    ed(a).notebook_back.clear();
    ed(a).from_graph = false;
    backlinks_revalidate(); // (another notebook may have changed its links since: checked on idle ticks)
    libindex_revalidate(); // (and the Graph of all notebooks' index, for any notebook's files)
    close_notebook(a);
    // Every way into a notebook comes here (the Library's card and its menu, a new notebook): its
    // compaction, queued or running, is stopped before anything here reads or writes its pages (a
    // stopped compaction leaves each page file as it was; the list keeps the pages for later).
    bg_cancel_notebook(id);
    if store_load_info(id, &mut ed(a).nb) != 0 {
        app_toast(a, tr("Couldn't open notebook"), 3000);
        return;
    }
    let s = ed(a);
    s.thumb_dirty = false;
    store_recover_pages(&s.nb); // pages an older version set aside that open now
    // Pages an older version saved on blank paper get their pictures back (bg/pages.ini), all of
    // them now: the thumbnail and PDF it drew are drawn again when the notebook closes.
    store_restore_listed_pages(&s.nb);
    bgimage_retry_missing(); // (a picture the Library found missing may be there now)
    use_page_size(s);
    s.index = requested.and_then(|pid|s.nb.page_ids.iter().position(|&p|p==pid)).map_or_else(||clampi(s.nb.last_page,0,s.nb.npages()-1),|i|i as i32);
    s.page = s.page_at(s.index);
    if s.page.is_none() {
        s.free_histories();
        app_toast(a, tr("Couldn't open notebook"), 3000);
        return;
    }
    s.open = true;
    nav_load(s.nb.id.as_str(), &mut s.nav);
    s.dl = doclinks_load(s.nb.id.as_str());
    // The page it opens at was read before its links were: its beta.5 links migrate now, before the
    // first tap.
    s.links_dirty = false;
    s.links_fail_logged = false;
    s.migrate_done = Vec::new();
    let _ = s.migrate_done.try_reserve_exact(s.nb.page_ids.len()); // (else no idle migration: migrate_room)
    if let Some(e) = s.hist.iter().position(|h| h.page_id == s.nb.page_ids[s.index as usize]) {
        s.migrate_entry(e);
    }
    s.page_dirty = false;
    s.info_dirty = requested.is_some() && s.nb.last_page != s.index;
    if s.info_dirty {s.nb.last_page=s.index;}
    s.changed = false;
    if s.page_canvas.px.is_empty() && canvas_init(&mut s.page_canvas, s.pgw, s.pgh) != 0 {
        return;
    }
    render_page_open(a);
    ed(a).view_w = 0; // set the view up afresh on enter
    app_switch(a, ScreenId::Editor);
    arrive_readonly(a);
    first_open_tip(a);
}

// The owner who updated from the build before is told once, as the first notebook comes up, what two
// fingers up and down do now (hints bit 0). Not a fresh install, a settings file made by this build, or
// an owner who has seen it (Settings.upgraded, store.rs). The gesture map is not consulted: the tip says
// the two built-ins, and an owner who has only just updated has no gestures.ini to have changed them.
fn first_open_tip(a: &mut App) {
    let st = &a.settings;
    if !st.upgraded || !st.gestures || st.hints & HINT_FIRST_OPEN != 0 || !ed_ref(a).open || a.scr != ScreenId::Editor {
        return;
    }
    a.settings.hints |= HINT_FIRST_OPEN;
    settings_save(&a.settings);
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines), shown once after an update: what
    // two fingers dragged up and down on a page do now.
    app_toast(a, tr("New: two fingers down for the Graph, up for page links."), 6000,
    );
}

/// the same, at that page
// A page KOReader sent (annotate.h): notebook id open at its page page_id, or where
// editor_open_notebook opens it should it have no such page. added: the page was added to the
// notebook (or the notebook made) at this start, a change like any other: the notebook's PDF is
// made again as it closes, and the Library's card too when the page is its first.
pub fn editor_open_notebook_at(a: &mut App, id: &str, page_id: u32, added: bool) {
    ed(a).requested_page=Some(page_id);
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
    let Some(paper) = s.open_page().filter(|_| s.open).map(|p| p.paper) else { return ;
    };
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

// The open page changed: as it changes, before it is drawn again (page_is_its_file).
fn mark_changed(s: &mut EditorState) {
    s.page_dirty = true;
    s.changed = true;
    s.last_up = now_ms();
}

fn sample_of(s: &EditorState, ev: &InEvent) -> PenSample {
    let mut p = PenSample { x: 0.0, y: 0.0, pressure: ev.pressure, tilt_x: ev.tilt_x, tilt_y: ev.tilt_y, t_ms: in_sample_ms(ev) ,
    };
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
                c = Canvas { px, w: s.pgw, h: s.pgh, stride: s.pgw, clip: Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
                    } ,
                };
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
        let whole = Rect { x: 0, y: 0, w: c.w, h: c.h ,
        };
        page_paper_render(&mut c, s.open_page().unwrap(), whole, dpi); // (an image paper's picture too)
        if bgimage_take_missing() {
            note_white(s, bgimage_generation());
        }
        s.paper = SharedPaper { w: c.w, h: c.h, stride: c.stride, px: Some(Rc::new(c.px)) ,
        };
    }
    true
}

fn draw_begin(a: &mut App, ev: &InEvent) {
    if refuse_readonly(a) {
        // neither ink nor the eraser (tool, eraser end or side button)
        let s = ed(a);
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
        step_begin(s, true); // (the sweep draws the page again as it goes: base keeps it as it was)
        s.snap = s.open_page().unwrap().strokes.iter().map(|x| stroke_id(x)).collect();
        s.removed.clear();
        let (mut ex, mut ey) = (0.0, 0.0);
        to_page(s, ev.x, ev.y, &mut ex, &mut ey);
        s.ex = ex;
        s.ey = ey;
        s.erase_dirty = RECT_EMPTY;
        display_live_ink(true); // a pen stroke too: its frames never wait (erase_end repaints them)
        return;
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
        seed = nav_new_seed_with(&s.nav, Some(&s.dl), s.open_page().unwrap(), pid, seed);
    }
    // (stroke_new cannot fail here: the C's NULL check has nothing to catch)
    let mut cur = if erase { stroke_new(TOOL_ERASER, &prm, erase_seed(ps.tool, seed)) } else { stroke_new(ps.tool, &prm, seed) };
    stroke_mark_tip(&mut cur); // live and replayed, it keeps the tip it is drawn with now
    let (ppx, pstride) = if paper { (s.paper.px.clone(), s.paper.stride) } else { (None, 0) };
    stroke_brush_begin(&mut s.bs, &cur, dpi, ppx, 0, 0, pstride);
    let first = sample_of(s, ev);
    s.sh.burst = shape_in_burst(s.sh.last_lift.as_ref(), &first, dpi, s.zoom);
    s.sh.anchor = (ev.x, ev.y);
    s.sh.anchor_k = in_sample_ms(ev);
    s.sh.still_fired = false;
    s.sh.preview = None;
    s.sh.last_kind = ShapeKind::None;
    s.cur = Some(cur);
    step_begin(s, false); // (live ink goes straight onto the page canvas: base keeps the page under it)
    smoother_reset(&mut s.sm);
    // Ink asks for the pen mode (where it is on); an erase stroke is live ink without it, as the
    // stroke eraser is (the driver's pen path is for ink going down).
    if erase {
        display_live_ink(true);
    } else {
        display_pen_mode(true);
    }
}

fn shape_anchor_moved(anchor: (f32,f32), point: (f32,f32), dpi: f32, preview: bool) -> bool {
    (point.0-anchor.0).hypot(point.1-anchor.1) > mm_to_px(SHAPE_HOLD_MM * if preview {1.5} else {1.0},dpi)
}
fn shape_dwell_ms(ms: i32, burst: bool) -> u32 {
    (ms as f32 * if burst {SHAPE_BURST_HOLD} else {1.0}) as u32
}

fn draw_sample(a: &mut App, ev: &InEvent) {
    let shape_log = a.shape_log;
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
    sample_as_stored(&mut p); // (drawn as the page file will keep it: what a reopen draws)
    stroke_append(cur, &p, dpi);
    if s.sh.preview.is_some() {
        if !shape_anchor_moved(s.sh.anchor,(ev.x,ev.y),dpi,true) { return; }
        let preview = s.sh.preview.take().unwrap();
        canvas_blit(&mut s.page_canvas, &preview.saved, Rect { x:0,y:0,w:preview.region.w,h:preview.region.h }, preview.region.x, preview.region.y);
        let mut dirty = preview.region;
        for pt in &s.cur.as_ref().unwrap().pts[preview.raw_n..] {
            dirty = rect_union(dirty, brush_add(&mut s.bs, &mut s.page_canvas, pt));
        }
        s.sh.anchor = (ev.x,ev.y); s.sh.anchor_k = in_sample_ms(ev); s.sh.still_fired = false;
        s.sh.last_kind = ShapeKind::None;
        if shape_log {qlog!("shape: moved on");}
        show(a, dirty, RefreshMode::Fast);
        return;
    }
    if shape_anchor_moved(s.sh.anchor,(ev.x,ev.y),dpi,false) {
        s.sh.anchor = (ev.x,ev.y); s.sh.anchor_k = in_sample_ms(ev); s.sh.still_fired = false;
    }
    let d = brush_add(&mut s.bs, &mut s.page_canvas, &p);
    if !rect_is_empty(d) {
        show(a, d, RefreshMode::Fast);
    }
}

fn shape_report(enabled: bool, mode: &str, result: &ShapeResult, burst: bool, zoom: f32, elapsed: u32, render: u32) {
    if !enabled {return;}
    let kind = format!("{:?}",result.kind).to_lowercase();
    let reason = format!("{:?}",result.reason).to_lowercase();
    let f = result.features;
    qlog!("shape: %s %s %s diag %.1f straight %.3f gap %.3f wind %.2f turn %.2f corners %d rms %.3f max %.3f axis %.3f burst %d zoom %.2f ms %u render %u",
        mode,kind.as_str(),reason.as_str(),f.diag_mm,f.straight,f.gap,f.winding,f.turning,f.corners,f.rms,f.max_err,f.axis_ratio,burst as i32,zoom,elapsed,render);
}

// A still pen sends no samples on some hardware, so this is driven by the 30 ms wall-clock tick.
fn shape_hold_tick(a: &mut App, now: u32) {
    if !a.settings.shapes || !a.input_drained { return; }
    let (dpi, hold_ms, logging, min_mm) = (a.dpi, a.settings.shape_hold_ms, a.shape_log, a.settings.shape_min_mm);
    let (policy,corner)=((a.settings.shape_quads,a.settings.shape_round),a.settings.shape_corner);
    let s = ed(a);
    if s.owner != Owner::Draw || s.sh.still_fired || s.sh.preview.is_some() { return; }
    let Some(raw) = s.cur.as_ref().filter(|raw| !stroke_is_erase(raw)) else { return; };
    let hold = shape_dwell_ms(hold_ms,s.sh.burst);
    if now.wrapping_sub(s.sh.anchor_k) < hold { return; }
    s.sh.still_fired = true;
    // Smallest shape is the recognizer's own floor, in the screen mm its gates use (2.0.0-rc.12: it
    // was a second check in page mm, above a fixed 10 mm floor it could not lower).
    let mut prm = shape_params(dpi, s.zoom, s.open_page().unwrap().paper, s.pgw);
    prm.min_mm = min_mm as f32;
    prm.burst = s.sh.burst;
    prm.quad_mode=policy.0;prm.circle_mode=policy.1;prm.corner=corner as f32/100.0;
    work_pump();
    let started = now_ms();
    let r = shape_recognize(&raw.pts, &prm);
    let elapsed = now_ms().wrapping_sub(started);
    work_pump();
    let s = ed(a);
    s.sh.last_reason = r.reason;
    if r.kind == ShapeKind::None { shape_report(logging,"hold",&r,prm.burst,prm.zoom,elapsed,0); return; }
    let raw = s.cur.as_ref().unwrap();
    let Some(mut stroke) = shape_stroke(raw, &r, dpi) else { return; };
    stroke.pts.iter_mut().for_each(sample_as_stored);
    let region = rect_intersect(rect_union(raw.bbox, stroke.bbox), Rect {x:0,y:0,w:s.pgw,h:s.pgh});
    if rect_is_empty(region) { return; }
    let mut saved = Canvas::default();
    if canvas_init(&mut saved, region.w, region.h) != 0 { if logging {qlog!("shape: hold none nomem");} return; }
    canvas_blit(&mut saved, &s.page_canvas, region, 0, 0);
    let raw_n = raw.pts.len();
    let render_started = now_ms();
    draw_page(a, region);
    let s = ed(a);
    let mut bs = BrushState::default();
    stroke_brush_begin(&mut bs, &stroke, dpi, None, 0, 0, 0);
    for (i, pt) in stroke.pts.iter().enumerate() {
        brush_add(&mut bs, &mut s.page_canvas, pt);
        if i & 7 == 0 { work_pump(); }
    }
    s.sh.last_kind = r.kind;
    s.sh.preview = Some(ShapePreview { saved, region, raw_n, stroke });
    shape_report(logging,"hold",&r,prm.burst,prm.zoom,elapsed,now_ms().wrapping_sub(render_started));
    show(a, region, RefreshMode::Fast);
}

// A stroke ends: draw_end_stroke keeps it (an undo step), and that step keeps the tiles it changed.
fn draw_end(a: &mut App) {
    let s = ed(a);
    let mut r = s.cur.as_ref().map_or(RECT_EMPTY, |c| c.bbox);
    if let Some(p) = s.sh.preview.as_ref() {
        r = rect_union(r, rect_union(p.region, p.stroke.bbox));
    }
    draw_end_stroke(a);
    step_end(ed(a), r);
}

fn draw_end_stroke(a: &mut App) {
    let shape_log = a.shape_log;
    let dpi = a.dpi;
    let s = ed(a);
    let snapped = s.sh.preview.take();
    if let Some(preview) = snapped {
        s.cur = Some(preview.stroke);
        s.sh.kept=s.sh.kept.saturating_add(1);
        if shape_log {qlog!("shape: kept");}
        s.settle = rect_union(s.settle, preview.region);
        live_reset();
    }
    if let Some(cur) = s.cur.as_ref().filter(|s| !stroke_is_erase(s)) {
        s.sh.last_lift = cur.pts.last().copied();
    }
    let st = s.cur.take();
    s.owner = Owner::None;
    s.boxes.stale = true; // (a beta.5 link's ink may have been added to)
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
    // An erase stroke (the precise eraser, white ink) turned ink back to paper under DU: what it swept
    // counts toward a clean (display.rs, DuWear), added once the stroke is kept below. Pen-up work only.
    let wear = stroke_is_erase(&st).then(|| (st.bbox, stroke_wear_area(&st, dpi)));
    let h = s.cur_hist();
    let had_undo = h.is_some_and(|i| history_can_undo(&s.hist[i].h));
    let had_redo = h.is_some_and(|i| history_can_redo(&s.hist[i].h));
    let ip = s.page_ix().unwrap();
    let was_empty = s.hist[ip].page.as_deref().unwrap().strokes.is_empty();
    let erase = stroke_is_erase(&st);
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
    if let Some((bbox, area)) = wear {
        let (sr, z) = (page_to_screen(a, bbox), ed_ref(a).zoom as f64);
        du_wear_add(&mut a.du_wear, sr, (area * z * z) as i64, now_ms());
    }
}

/// The page px a stroke counts as toward DU wear (display.rs, DuWear) when the eraser takes it away, or
/// an erase stroke sweeps: the band its line lies in, its tip's width but at least WEAR_BAND_MM (the ink
/// and the paper right around it, what the eye takes for the erased place), never more than its box. So a
/// long thin line counts for its length, not for the page its box spans; a word for about its own box.
fn stroke_wear_area(st: &Stroke, dpi: f32) -> f64 {
    let tip = 2.0 * brush_max_radius(stroke_render_tool(st), &st.params, dpi);
    let band = tip.max(mm_to_px(WEAR_BAND_MM, dpi)) as f64;
    let len: f64 = st.pts.windows(2).map(|w| ((w[1].x - w[0].x) as f64).hypot((w[1].y - w[0].y) as f64)).sum();
    (len * band + band * band).min(st.bbox.w.max(0) as f64 * st.bbox.h.max(0) as f64)
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
fn dark_near(s_ed: &EditorState, dpi: f32, s: &Stroke, paper: &SharedPaper, rr: Rect, x0: f32, y0: f32, x1: f32, y1: f32, r: f32,
) -> bool {
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
    canvas_blit(pc, &keep, Rect { x: 0, y: 0, w: rr.w, h: rr.h ,
        }, rr.x, rr.y,
    );
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
        Rect { x: 0, y: 0, w: pc.w, h: pc.h ,
        },
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
        && !dark_near(s, dpi, &s.open_page().unwrap().strokes[i as usize], &s.paper, rr, x0, y0, x1, y1, rp,
        )
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
    let mut worn = 0.0f64; // (page px: stroke_wear_area)
    let first = s.removed.len(); // (this step's strokes go after)
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
        worn += stroke_wear_area(&st, dpi);
        s.removed.push(st);
    }
    let s = ed(a);
    s.ex = x;
    s.ey = y;
    if !rect_is_empty(d) {
        ed(a).boxes.stale = true; // (a link's ink may be gone)
        // Only where they drew: a marker stroke across the owner's 1,090-stroke page drew its box (the
        // whole page) again in 34 s on the Kindle (2.0.0-rc.11); its tiles take a thirteenth of that.
        let d = draw_page_without(a, d, first);
        show(a, d, RefreshMode::Fast);
        // (DU turned that ink back to paper: its traces count toward a clean, display.rs DuWear)
        let (sr, z) = (page_to_screen(a, d), ed_ref(a).zoom as f64);
        du_wear_add(&mut a.du_wear, sr, (worn * z * z) as i64, now_ms());
        let s = ed(a);
        s.erase_dirty = rect_union(s.erase_dirty, d);
    }
}

fn erase_end(a: &mut App) {
    let r = ed(a).erase_dirty;
    erase_end_sweep(a);
    step_end(ed(a), r);
}

fn erase_end_sweep(a: &mut App) {
    let s = ed(a);
    s.owner = Owner::None;
    s.boxes.stale = true;
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
    // (an empty page stays as it is: nothing to clear, no step, no toast)
    if ed(a).open_page().is_none_or(|p| p.strokes.is_empty())
        && nav_links_of(&ed_ref(a).nav,ed_ref(a).open_pid(),&mut 0)==0 {
        return;
    }
    let s = ed(a);
    let Some(h) = s.cur_hist() else { return };
    let ip = s.page_ix().unwrap();
    let pid=s.open_pid();
    if !s.nav.links_writable {
        app_toast(a,tr("Couldn't save links. Page unchanged."),3000);return;
    }
    let mut records=Vec::new();
    let count=nav_links_of(&s.nav,pid,&mut 0) as usize;
    if count>0 {
        if records.try_reserve_exact(count).is_err() {
            app_toast(a,tr("Couldn't save links. Page unchanged."),3000);return;
        }
        for link in s.nav.links.iter().filter(|link|link.pid==pid) {records.push((Some(link_rec(link)),None));}
        for (link,_) in &records {nav_link_remove(&mut s.nav,pid,link.as_ref().unwrap().seed);}
        if nav_save_links(s.nb.id.as_str(),&mut s.nav,&s.nb)!=0 {
            nav_load(s.nb.id.as_str(),&mut s.nav);migration_reset(s);
            app_toast(a,tr("Couldn't save links. Page unchanged."),3000);return;
        }
        s.links_dirty=false;
    }
    let links=(!records.is_empty()).then_some(LinkEdit{pid,records});
    let (hh, page) = hist_page(&mut s.hist, h, ip);
    history_push_clear_links(hh, page,links);
    mark_changed(s); // (before it is drawn: page_is_its_file)
    ed(a).boxes.stale = true;
    bar_rebuild(a);
    render_page_full(a);
    compose_all(a, RefreshMode::Full);
    repaint_panel(a, RefreshMode::Full);
    app_toast_dismiss(a); // (no "Page cleared": the owner, 2026-11-04; a toast left up from before goes)
}

// Whether a sheet is in front of the page: the Pages view, the tool picker, Custom music, Display
// or the paper sheet. The pen's button clears no page from under one.
fn sheet_open(s: &EditorState) -> bool {
    s.destination.open
        || s.map.open
        || s.pv.open
        || s.tools_open
        || s.mb.open
        || s.dm.open
        || s.paper_open
        || s.ts.open
        || s.lb.open
        || s.find.open
        || s.lt.open
        || s.card.open
        || s.qs.open
}

/// Whether the pen's next contact is no ink: the Links bar is up (a tap on it, or outside it, which closes or
/// stays), a link is being made (the strip, the lasso: 6.4), or a pen contact outside the bar is being
/// swallowed. Fast ink never previews such a contact.
pub fn editor_pen_not_ink(a: &App) -> bool {
    a.ed.as_deref().is_some_and(|s| {
        s.map.open
            || s.owner == Owner::PenDrag
            || s.pb_drag.is_some()
            || s.sh.preview.is_some()
            || s.lb.open
            || s.find.open
            || s.card.open
            || s.qs.open
            || s.lm.state != LmState::Off
            || s.sel.state != SelState::Off
            || (s.owner == Owner::Outside && s.owner_is_pen)
    })
}

/// A cheap snapshot for the early input hook. UI gestures never paint provisional ink. The
/// real brush remains authoritative: textures, nibs and paper restoration arrive in the tiles.
pub fn editor_live_style(a: &App, ev: &InEvent, out: &mut LiveStyle) {
    if crate::library::library_settings_active(a) {return;}
    let Some(s) = a.ed.as_deref() else { return };
    let Some(page) = s.open_page() else { return };
    if !s.open || pb_contact_bound(a, ev) || s.sh.preview.is_some() || page.readonly || sheet_open(s) || s.pinching || s.lm.state != LmState::Off || (s.owner != Owner::None && s.owner != Owner::Draw && s.owner != Owner::Erase) || editor_pen_not_ink(a) {
        return;
    }
    let st = &a.settings;
    let momentary = (ev.side_button && pb_momentary(a, PenBtn::Button))
        || (ev.side_button2 && pb_momentary(a, PenBtn::Button2));
    let mut ps = pen_stroke(st, ev.eraser || s.hold_eraser || momentary, ev.side_button);
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
    out.clip = rect_intersect(page_to_screen(a, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
            },
        ), Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    );
    out.clip = rect_intersect(out.clip, Rect { x: 0, y: status_reserved(a), w: a.w, h: a.h - status_reserved(a) ,
        },
    );
    out.exclude = s.panel;
}

/// A bound side-button contact. Its viewport anchor is fixed for the whole contact.
struct PenDrag {
    button: PenBtn,
    action: i32,
    x: f32,
    y: f32,
    px: f32,
    py: f32,
    ox: f32,
    oy: f32,
    zoom: f32,
    painted_at: u32,
    changed: bool,
}

fn pb_bit(button: PenBtn) -> u8 {
    match button { PenBtn::Button => 1, PenBtn::Button2 => 2, PenBtn::Eraser => 4 }
}

fn pb_key(button: PenBtn, kind: PenKind) -> GestureKey {
    GestureKey::Pen { button, kind }
}

fn pb_momentary(a: &App, button: PenBtn) -> bool {
    [PenKind::Hold, PenKind::Drag].iter().any(|&kind| {
        gesturemap_action(&a.gestures, pb_key(button, kind)) == SHORTCUT_HOLD_ERASER
    })
}

/// The early Live hook sees the button bits before the queued down reaches the editor.
fn pb_contact_bound(a: &App, ev: &InEvent) -> bool {
    [(PenBtn::Button, ev.side_button), (PenBtn::Button2, ev.side_button2)]
        .iter().any(|&(button, down)| down && !pb_momentary(a, button)
            && gesturemap_action(&a.gestures, pb_key(button, PenKind::Drag)) != 0)
}

fn pb_reset(s: &mut EditorState) {
    s.pb = PenButtons::default();
    s.pb_pen = PbPen::default();
    s.pb_buttons = 0;
    s.pb_erasers = 0;
    s.pb_drag = None;
    s.hold_eraser_key = false;
    s.hold_eraser = false;
    sel_hold_reset(s);
}

fn pb_dispatch(a: &mut App, action: i32) {
    if action <= 0 || action == SHORTCUT_HOLD_ERASER { return; }
    let (x, y) = ed_ref(a).pb_point.unwrap_or((a.w as f32 * 0.5, a.h as f32 * 0.5));
    editor_shortcut_at(a, action, x, y, ActionSrc::Pen);
}

fn pb_drag_end(a: &mut App, button: PenBtn) {
    if ed_ref(a).pb_drag.as_ref().is_none_or(|d| d.button != button) { return; }
    let drag = ed(a).pb_drag.take().unwrap();
    if drag.changed && a.scr == ScreenId::Editor && ed_ref(a).open {
        compose_all(a, RefreshMode::Ui);
    }
    if !ed_ref(a).pb_pen.touching && ed_ref(a).owner == Owner::PenDrag {
        ed(a).owner = Owner::None;
    }
}

fn pb_out(a: &mut App, out: PbOut, t: u32) {
    match out {
        PbOut::None => {}
        PbOut::Fire(button, kind) => {
            ed(a).pb_last = Some((button, kind));
            pb_dispatch(a, gesturemap_action(&a.gestures, pb_key(button, kind)));
        }
        PbOut::HoldBegin(button) => {
            ed(a).pb_last = Some((button, PenKind::Hold));
            let action = gesturemap_action(&a.gestures, pb_key(button, PenKind::Hold));
            // (`select` on a hold is held, not toggled: Select mode while the button is down)
            if action == SHORTCUT_SELECT { sel_hold(a, true); return; }
            pb_dispatch(a, action);
        }
        PbOut::HoldEnd(button) => {
            if gesturemap_action(&a.gestures, pb_key(button, PenKind::Hold)) == SHORTCUT_SELECT { sel_hold(a, false); }
        }
        PbOut::DragBegin(button) => {
            ed(a).pb_last = Some((button, PenKind::Drag));
            let action = gesturemap_action(&a.gestures, pb_key(button, PenKind::Drag));
            if action == 0 || pb_momentary(a, button) { return; }
            // `select` on a drag: Select mode held, and this contact (not claimed here) its lasso
            if action == SHORTCUT_SELECT { sel_hold(a, true); return; }
            // (what the pen held ends first: ink kept as drawn, a lasso back to the strip; a lasso the drag
            // took over stayed half drawn, and the next stroke was ink with the strip still up)
            owner_cancel(a);
            let (x, y) = ed_ref(a).pb_point.unwrap_or((a.w as f32 * 0.5, a.h as f32 * 0.5));
            let s = ed(a);
            let (mut px, mut py) = (0.0, 0.0);
            to_page(s, x, y, &mut px, &mut py);
            let drag = PenDrag { button, action, x, y, px, py, ox: s.ox, oy: s.oy,
                zoom: s.zoom, painted_at: t, changed: false };
            // A bound contact is never ink, even if its action is refused by a sheet.
            a.pen_action_down = true;
            if action != GESTURE_PAN && action != GESTURE_ZOOM { pb_dispatch(a, action); }
            if a.scr == ScreenId::Editor && ed_ref(a).open {
                let s = ed(a);
                s.pb_drag = Some(drag);
                s.owner = Owner::PenDrag;
                s.owner_is_pen = true;
            }
        }
        PbOut::DragEnd(button) => {
            if gesturemap_action(&a.gestures, pb_key(button, PenKind::Drag)) == SHORTCUT_SELECT { sel_hold(a, false); }
            pb_drag_end(a, button)
        }
    }
}

fn pb_drag_move(a: &mut App, ev: &InEvent) {
    if !ev.x.is_finite() || !ev.y.is_finite() || sheet_open(ed_ref(a)) { return; }
    let Some(d) = ed_ref(a).pb_drag.as_ref() else { return };
    if d.action != GESTURE_PAN && d.action != GESTURE_ZOOM { return; }
    let before = (ed_ref(a).zoom, ed_ref(a).ox, ed_ref(a).oy);
    let (action, x, y, px, py, ox, oy, zoom) = (d.action, d.x, d.y, d.px, d.py, d.ox, d.oy, d.zoom);
    let dpi = a.dpi;
    let s = ed(a);
    if action == GESTURE_PAN {
        s.ox = ox - (ev.x - x) / zoom;
        s.oy = oy - (ev.y - y) / zoom;
    } else {
        s.zoom = clampf(zoom * 1.25f32.powf((y - ev.y) / (dpi * 10.0 / 25.4)),
            s.zoom_min, s.zoom_max);
        s.ox = px - x / s.zoom;
        s.oy = py - y / s.zoom;
    }
    view_clamp(a);
    let s = ed(a);
    if before != (s.zoom, s.ox, s.oy) {
        let d = s.pb_drag.as_mut().unwrap();
        d.changed = true;
        if ev.t_ms.wrapping_sub(d.painted_at) >= 180 {
            d.painted_at = ev.t_ms;
            compose_all(a, RefreshMode::Fast);
        }
    }
}

/// Feed the shared timing model before any page/map pointer path can claim the nib.
fn pb_input(a: &mut App, ev: &InEvent) -> bool {
    if !matches!(ev.type_, InType::PenButton | InType::PenHover | InType::PenDown
        | InType::PenMove | InType::PenUp | InType::PenLeave) { return false; }
    if !ed_ref(a).open { return ev.type_ == InType::PenButton; }
    // The pen in range: a finger's hold arms no lasso now, and an armed one is dropped (select.rs)
    sel_finger_pen(a);
    if matches!(ev.type_, InType::PenDown | InType::PenHover) {
        fs_abort_view(a, FsAbortView::Pen);
    }
    let consumed = ed_ref(a).owner == Owner::PenDrag;
    let config = a.pb_config;
    let s = ed(a);
    match ev.type_ {
        InType::PenLeave => s.pb_pen = PbPen::default(),
        InType::PenButton => s.pb_pen.prox = true,
        _ => {
            s.pb_point = Some((ev.x, ev.y));
            s.pb_pen = PbPen { prox: true,
                touching: matches!(ev.type_, InType::PenDown | InType::PenMove) };
        }
    }
    let pen = s.pb_pen;
    s.pen_near = pen.prox;
    if !pen.prox { s.pen_left = ev.t_ms; }
    if ev.type_ != InType::PenLeave {
        for (button, down) in [(PenBtn::Button, ev.side_button), (PenBtn::Button2, ev.side_button2)] {
            if button == PenBtn::Button2 && !input_caps().pen_button2 { continue; }
            if ev.type_ == InType::PenButton
                && ((ev.value == 2) != (button == PenBtn::Button2)) { continue; }
            let bit = pb_bit(button);
            let was = ed_ref(a).pb_buttons & bit != 0;
            if was == down { continue; }
            let eraser = down && pb_momentary(a, button);
            let s = ed(a);
            if down { s.pb_buttons |= bit; } else { s.pb_buttons &= !bit; }
            if eraser { s.pb_erasers |= bit; } else { s.pb_erasers &= !bit; }
            s.hold_eraser = s.hold_eraser_key || s.pb_erasers != 0;
            let out = penbutton_event(&mut s.pb, &config, PbIn { t_ms: ev.t_ms, button, down }, pen);
            pb_out(a, out, ev.t_ms);
        }
    }
    for _ in 0..3 {
        let out = penbutton_pen(&mut ed(a).pb, &config, pen, ev.t_ms);
        if out == PbOut::None { break; }
        pb_out(a, out, ev.t_ms);
    }
    if ev.type_ == InType::PenLeave {
        let s = ed(a);
        s.pb_buttons = 0; s.pb_erasers = 0;
        s.hold_eraser = s.hold_eraser_key;
    }
    if ed_ref(a).owner == Owner::PenDrag || consumed {
        if ev.type_ == InType::PenMove { pb_drag_move(a, ev); }
        if matches!(ev.type_, InType::PenUp | InType::PenLeave) {
            ed(a).owner = Owner::None;
        }
        return true;
    }
    ev.type_ == InType::PenButton || a.scr != ScreenId::Editor
}

fn undo_redo(a: &mut App, undo: bool) {
    settle_now(a);
    sel_deselect(a);
    if refuse_readonly(a) {
        return;
    }
    let s = ed(a);
    let Some(h) = s.cur_hist() else { return };
    let Some(ip) = s.page_ix() else { return };
    let kind = if undo { history_undo_kind(&s.hist[h].h) } else { history_redo_kind(&s.hist[h].h) };
    let has_links=if undo {s.hist[h].h.undo.last()}else{s.hist[h].h.redo.last()}.is_some_and(|step|step.link.is_some());
    if kind == Some(ActKind::Link) || has_links {
        undo_redo_link(a, h, ip, undo);
        return;
    }
    let s = ed(a);
    let mut d = RECT_EMPTY;
    let (hh, page) = hist_page(&mut s.hist, h, ip);
    if !(if undo { history_undo(hh, page, &mut d) } else { history_redo(hh, page, &mut d) }) {
        return;
    }
    ed(a).boxes.stale = true;
    // The step's own pixels swapped back where it kept them (history.rs Patch): a copy of what it
    // touched, however dense the page. Else its region drawn again (back to a template whose picture
    // is still being made: white until it is).
    let s = ed(a);
    if !base_valid(s) {
        base_reset(s);
    }
    let swapped = if base_valid(s) && !s.picture_missing {
        history_swap_patch(&mut s.hist[h].h, undo, &mut s.page_canvas, &mut s.base).is_some()
    } else {
        false
    };
    if swapped {
        s.undo_patched += 1;
    } else {
        draw_page(a, d);
    }
    let s = ed(a);
    let full = d.w >= s.pgw && d.h >= s.pgh;
    show(a, d, if full { RefreshMode::Full } else { RefreshMode::Ui },
    );
    mark_changed(ed(a));
    repaint_panel(a, RefreshMode::Ui);
}

// ---- panel interaction ----

// ---- pages view: open, act, delete ----

fn pages_show(a: &mut App) {
    card_drop(ed(a));
    lm_drop(ed(a)); // (the view comes up whole)
    if !save_page(a){compose_all(a,RefreshMode::Full);return;}
    ed(a).pv.unified=true;ed(a).pv.index_tab=false;ed(a).pv.tab_initialized=true;
    let index = ed(a).index;
    if !with_pv(a, |pv, a, nb, nav| pagesview_open(pv, a, nb, nav, index)) {
        compose_all(a,RefreshMode::Full);return;
    }
    pages_seed_open(ed(a));
    compose_all(a, RefreshMode::Full);
}

// The open page's tile from its canvas, so the view shows it at once (its saved preview is made in the
// background). Not for a page that did not save, nor one drawn only in part.
fn pages_seed_open(s: &mut EditorState) {
    if s.page_dirty || s.page_ix().is_none() || s.page_canvas.w != s.pgw || s.page_canvas.h != s.pgh {
        return;
    }
    let pid = s.open_pid();
    pagesview_seed(&mut s.pv, &s.nb, pid, &s.page_canvas);
}

// Explicit Index entries stay on the list even on the first open with no named pages.
fn index_show(a:&mut App) {map_show(a,false);}


// name-page: the Pages view's naming keyboard for the open page, drawn over the notebook, as Name this page
// there opens it; its Done or Cancel goes back to the page (pages_event). Nothing while the names
// cannot be changed (that button is greyed out then).
fn pages_name(a: &mut App) {
    if !ed(a).nav.writable {
        return;
    }
    lm_drop(ed(a)); // (the view comes up whole)
    if !save_page(a){return;}
    ed(a).pv.unified=true;
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
    let Some(i) = s.hist.iter().position(|e| e.page_id == pid) else { return ;
    };
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
    // (no toast when done: the owner, 2026-11-04. One left up from before goes first, so one the delete raises,
    // a page that could not be saved, stays.)
    app_toast_dismiss(a);
    let s = ed(a);
    if k == n {
        // A notebook keeps at least one page: add a fresh blank one to stay behind.
        let paper = s.open_page().map_or(s.nb.paper, |p| p.paper);
        if store_add_page(&mut s.nb, n, paper) != 0 {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Delete in the Pages view
            // could not delete the ticked pages (the blank page a notebook keeps could not be added).
            app_toast(a, tr("Couldn't delete selection."), 2500);
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
        render_page_open(a);
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
    // (the pages' files went to grimoire/trash; the view shows them gone)
    compose_all(a, RefreshMode::Full);
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

// New page: inherit the current page's paper (document pictures fall back to notebook paper), at the index the
// Pages view gives, never the first, so the Library's card stays the same page. A page that holds
// no book page (in no line of source.ini) is a note page: a book page annotated later still goes
// before the first page that holds a later book page, after the notes on the one before.
fn pages_new(a: &mut App, at: i32) {
    if !save_page(a) {
        return;
    }
    let s = ed(a);
    let paper = s.open_page().map_or(s.nb.paper, |p| p.paper);
    if at < 1 || store_add_page(&mut s.nb, at, paper) != 0 {
        app_toast(a, tr("Couldn't add page"), 3000);
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
    // (no confirmation: the owner, 2026-11-04. A toast left up from before goes first, so one the copy raises
    // stays: the copy opening read-only says so.)
    app_toast_dismiss(a);
    // The source page read first (PHASE2 9.2): it is migrated as it is read, so the copy takes its areas. (One
    // that does not read keeps legacy links; they are copied anyway and find the copy's ink: same strokes,
    // same seeds.)
    let _ = ed(a).page_at(index);
    if store_duplicate_page(&mut ed(a).nb, index, index + 1) != 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Copy page in the
        // Pages view could not copy the page (the storage is full, or the notebook has 2000 pages).
        app_toast(a, tr("Couldn't copy page"), 3000);
        return;
    }
    // Copy page keeps links: the copy gets every link of its source, same seed, target and area. Not an undo
    // step; links to the copied page are not changed. Before the copy opens, so its boxes are there.
    let s = ed(a);
    let (src, copy) = (s.nb.page_ids[index as usize], s.nb.page_ids[index as usize + 1],
    );
    let mut first = 0;
    let mut links_failed = false;
    if nav_links_of(&s.nav, src, &mut first) > 0 {
        s.boxes.stale = true;
        if nav_link_copy(&mut s.nav, src, copy) != 0 || nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
            nav_load(s.nb.id.as_str(), &mut s.nav); // (back to the links as saved: the copy has none)
            migration_reset(s);
            links_failed = true;
        } else {
            s.links_dirty = false; // (the save wrote every migrated area too)
        }
    }
    // The document links come too (a planner's tabs are part of the page): Copy page's rewrite of
    // doclinks.ini. Past the water level, or with the file unwritable, the page is copied without them and
    // the same toast says so.
    if !doclinks_of(&ed_ref(a).dl, src).is_empty() {
        let s = ed(a);
        let mut dl = s.dl.clone();
        if doclinks_copy_page(&mut dl, src, copy) < 0 || doclinks_save(s.nb.id.as_str(), &mut dl, &s.nb) != 0 {
            links_failed = true;
        } else {
            s.dl = dl;
            s.boxes.stale = true;
        }
    }
    pages_added(a, index + 1);
    let s = ed(a);
    let erased = s.open_page().is_some_and(|p| p.strokes.iter().any(|x| stroke_is_erase(x)));
    if erased && s.index == index + 1 {
        let pid = s.nb.page_ids[s.index as usize];
        note_for_compaction(s, pid);
    }
    if links_failed {
        // TRANSLATORS: a toast as the copy Copy page made opens (about 1700 px at 44 px in bold, or two
        // lines): the page was copied, but the links drawn on it could not be saved for the copy.
        app_toast(a, tr("Copied without links: couldn't save them."), 3000,
        );
    }
}

// Move to... (and a held tile dragged to a gap): the ticked pages, together and in their order, before
// page `to` (npages: at the end), store_move_pages_undo. Only the page list changes (pages, links, names
// and history go by page id), so the open page stays open wherever it went, and the view stays, on the
// screenful where the moved pages are now. One undo step of the notebook's own (pv.move_undo, the bottom
// row's Undo): the editor's Undo button keeps to the open page's history and never undoes a move.
fn pages_move(a: &mut App, to: i32) {
    // (a drag's carried pages, else the ticked ones; a failure draws the view again, as the drag's outline
    // and gap bar are still on the screen and the ticks stay as they were)
    let carry = std::mem::take(&mut ed(a).pv.carry);
    if !save_page(a) {
        compose_all(a, RefreshMode::Ui);
        return;
    }
    let s = ed(a);
    let ticked: Vec<i32> = if carry.is_empty() { (0..s.nb.npages()).filter(|&i| pagesview_ticked(&s.pv, i)).collect() } else { carry };
    if ticked.is_empty() {
        return;
    }
    let moved: Vec<u32> = ticked.iter().map(|&i| s.nb.page_ids[i as usize]).collect();
    let open = s.open_pid();
    let first_was = s.nb.page_ids[0];
    s.nb.last_page = s.index;
    if store_move_pages_undo(&mut s.nb, &ticked, to, &mut s.pv.move_undo) != 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Move to... could not save
        // the notebook's new page order (the storage is full).
        app_toast(a, tr("Couldn't move pages."), 3000);
        compose_all(a, RefreshMode::Ui);
        return;
    }
    let s = ed(a);
    s.pv.redo = None; // (a new move: the Redo of what an Undo took back goes)
    s.index = s.nb.page_ids.iter().position(|&p| p == open).map_or(s.index, |i| i as i32);
    s.nb.last_page = s.index;
    s.pv.current = s.index;
    s.changed = true; // (the PDF mirror runs at close)
    if s.nb.page_ids[0] != first_was && s.index == 0 {
        s.thumb_dirty = true; // a new first page: the Library's card (written from its canvas as it is left)
    }
    pagesview_untick(&mut s.pv);
    let at = s.nb.page_ids.iter().position(|&p| p == moved[0]).unwrap_or(0) as i32;
    let (cols, rows) = pagesview_grid(&ed_ref(a).pv, a);
    let s = ed(a);
    s.pv.screen = at / (cols * rows).max(1);
    // (the view shows the pages where they went, with no toast: the owner, 2026-11-04; one left up from before,
    // a failed try's, goes)
    compose_all(a, RefreshMode::Full);
    app_toast_dismiss(a);
}

// The bottom row's Undo, right after a move (or the `undo` action over the Pages view): the pages back in the
// order they had (store_undo_move_pages, the same path as the move), the open page still open, the view on the
// screenful where the first page that moved back is; the move is then the Redo (pv.redo). A record that no
// longer applies (the pages changed since) is dropped, never used.
fn pages_undo_move(a: &mut App) {
    if !save_page(a) {
        return;
    }
    let s = ed(a);
    let Some(u) = s.pv.move_undo.take() else { return };
    let open = s.open_pid();
    let first_was = s.nb.page_ids[0];
    s.nb.last_page = s.index;
    if store_undo_move_pages(&mut s.nb, &u) != 0 {
        if u.applies(&s.nb) {
            s.pv.move_undo = Some(u); // (notebook.ini could not be written: it can be tried again)
        }
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the Pages view's Undo could
        // not put the moved pages back (the storage is full).
        app_toast(a, tr("Couldn't undo move."), 3000);
        compose_all(a, RefreshMode::Ui);
        return;
    }
    let s = ed(a);
    s.index = s.nb.page_ids.iter().position(|&p| p == open).map_or(s.index, |i| i as i32);
    s.nb.last_page = s.index;
    s.pv.current = s.index;
    s.changed = true;
    if s.nb.page_ids[0] != first_was && s.index == 0 {
        s.thumb_dirty = true;
    }
    pagesview_untick(&mut s.pv);
    let at = u.before.iter().zip(&u.after).position(|(b, f)| b != f).unwrap_or(0) as i32;
    s.pv.redo = Some(PageRedo::Move(u));
    let (cols, rows) = pagesview_grid(&ed_ref(a).pv, a);
    let s = ed(a);
    s.pv.screen = at / (cols * rows).max(1);
    compose_all(a, RefreshMode::Full);
    app_toast_dismiss(a); // (no confirmation; a toast left up from before, a failed try's, goes)
}

/// From how many pages New page's batch takes its pad down (one refresh) before it writes them (a batch under it
/// takes about a second at most on a Kindle, ADD_PAGE_MS, and keeps its one refresh).
const ADD_BUSY_PAGES: i32 = 25;
/// A batch says "Adding N pages..." while it is written when it should take longer than this on a Kindle, ms, and
/// always from ADD_NOTICE_PAGES pages; a smaller one is written with nothing said (the owner, 2026-11-04: "Show
/// "Adding N pages..." only for big batches (100 or more pages, or when adding takes longer than about 2
/// seconds), and take it down when done. Small batches stay silent.").
const ADD_NOTICE_MS: i32 = 2000;
const ADD_NOTICE_PAGES: i32 = 100;
/// What one page of a batch takes on a Kindle, ms: a batch's time is told from its count before it starts, as
/// nothing reaches the panel while it runs. Each page is one write_file_atomic of its blank file (store_add_pages),
/// two fsyncs, and an fsync takes 15 to 25 ms there (util.rs: a page's save, two files, took 65 to 104 ms): about
/// 40 ms a page, so a batch takes 2 s from about 50 pages. The same on every paper: a picture paper's pages are the
/// same blank files, and bg/pages.ini is written once for the batch (the simulator: 0.13 to 0.20 ms a page on Blank
/// and on a template's picture alike, batches of 200 and 1000 pages). grimoire.log's "pages: added=" line gives
/// each batch's time, to set it by.
const ADD_PAGE_MS: i32 = 40;
// Whether a batch of `n` pages says "Adding N pages..." as it is written: a big one, or one that should take a
// while. Not while a pen held on an action holds the panel: no toast could show before the writing then (it would
// come after, once the pen lifts).
fn add_notice_due(a: &App, n: i32) -> bool {
    !a.pen_action_down && (n >= ADD_NOTICE_PAGES || n.saturating_mul(ADD_PAGE_MS) > ADD_NOTICE_MS)
}
// The notice itself, for `n` pages.
fn add_notice_text(n: i32) -> String {
    // TRANSLATORS: a toast while New page's batch is being written (about 1700 px at 44 px in bold, or two
    // lines): %d blank pages are being added. Only for a large batch, which can take a minute; it goes when
    // they are added.
    cformat!(ng_("Adding %d page...", "Adding %d pages...", n as i64), n)
}
// The refresh that puts what is drawn on the panel before a batch's pages are written (nothing reaches it while
// they are), with "Adding N pages..." when `notice` (put up just before): counted (editor.add_notices) only when
// that refresh carried it, so the count says the notice was on the panel before the writing began.
fn add_notice_flush(a: &mut App, notice: bool) {
    let shown = a.toast_presents;
    app_flush(a);
    if notice && a.toast_presents != shown {
        ed(a).add_notices += 1;
    }
}
// The writing a batch of new pages does, from the Pages view's New page (a tap is a batch of one, its count
// pad more) or the toolbar's count pad: `n` blank pages at index `at` (never first), on the paper of the page
// they go after (the owner, 2026-11-03: the selected page's, else the open page's; in the Pages view the batch
// goes after the last page ticked, else after the open page, and from the toolbar after the open page; a
// document picture's falling back to the notebook's, store_new_page_paper), in one store operation
// (store_add_pages: one notebook.ini write; a failure adds none). An annotated notebook takes them as it
// takes New page's (note pages). The open page stays open (its index follows it) and the batch is the
// notebook's Undo record (pv.add_undo), which the Pages view's bottom row offers while its pages are still
// blank. `redraw` draws the screen as it will be without the pad (the batch's pad has closed): first, for a
// large batch, and on a failure. A big batch says "Adding N pages..." until it is written (add_notice_due),
// and the notice is down at the end on every path: replaced by the failure's toast, or taken down. False when
// nothing was added (a toast said so).
fn add_pages_write(a: &mut App, at: i32, n: i32, redraw: fn(&mut App)) -> bool {
    if store_room_for_pages(&ed_ref(a).nb) < n {
        // (New page tapped in a full notebook: its count pad, held, says the same)
        app_toast(a, &cformat!(tr("Notebook full with %d pages."), STORE_MAX_PAGES as i32), 3000);
        redraw(a);
        return false;
    }
    if !save_page(a) {
        redraw(a);
        return false;
    }
    let notice = add_notice_due(a, n);
    if n >= ADD_BUSY_PAGES || notice {
        // A batch this large takes a while (each page file is written and synced, ADD_PAGE_MS on a Kindle):
        // the pad goes first, in one refresh, so the screen never sits on the pad, with "Adding N pages..." in
        // the same refresh when the batch is big (the main loop is held until the pages are written).
        redraw(a);
        if notice {
            let msg = add_notice_text(n);
            app_toast(a, &msg, 120_000);
        }
        add_notice_flush(a, notice);
    }
    let started = now_ms();
    let s = ed(a);
    let paper = batch_paper(s, at - 1);
    let open = s.open_pid();
    s.nb.last_page = s.index;
    let mut undo = None;
    let failed = at < 1 || store_add_pages(&mut s.nb, at, n, paper, &mut undo) != 0;
    // (a large batch's time on this device, which ADD_PAGE_MS estimates; a picture paper's said apart)
    if n >= ADD_BUSY_PAGES || notice || failed {
        qlog!("pages: added=%d of %d ms=%d notice=%d paper=%s", if failed { 0 } else { n }, n,
            now_ms().wrapping_sub(started) as i32, notice as i32,
            if paper_is_image(store_new_page_paper(&ed_ref(a).nb, paper)) { "picture" } else { "drawn" });
    }
    if failed {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): New page's batch could not
        // add the pages (the storage is full); none was added.
        app_toast(a, tr("Couldn't add pages."), 3000); // (in the notice's place)
        redraw(a);
        return false;
    }
    // (another toast that took the notice's place stays)
    if notice && a.toast_visible && a.toast.as_bytes() == add_notice_text(n).as_bytes() {
        app_toast_dismiss(a);
    }
    let s = ed(a);
    s.index = s.nb.page_ids.iter().position(|&p| p == open).map_or(s.index, |i| i as i32);
    s.nb.last_page = s.index;
    s.pv.current = s.index;
    s.pv.add_undo = undo;
    s.pv.redo = None; // (a new batch: the Redo of what an Undo took back goes)
    s.changed = true; // (the PDF mirror runs at close)
    let npages = s.nb.npages() as usize;
    if s.pv.sel.len() < npages {
        s.pv.sel.resize(npages, false);
    }
    true
}

// The paper a batch takes from page `after`, the page it goes after: the open page's as it is in memory (saved
// just before), another page's from its file (store_page_paper, which reads no strokes).
fn batch_paper(s: &EditorState, after: i32) -> PaperKind {
    if after == s.index {
        if let Some(p) = s.open_page() {
            return p.paper;
        }
    }
    if after >= 0 && after < s.nb.npages() {
        return store_page_paper(&s.nb, after);
    }
    s.open_page().map_or(s.nb.paper, |p| p.paper)
}

// New page's batch in the Pages view (its count pad, from a hold or a modifier tap, or a tap: a batch of
// one): add_pages_write, and the view stays, on the screenful of the first new page, its tiles marked
// "(new)" and its bottom row offering Undo while those pages are still blank.
fn pages_add_batch(a: &mut App, at: i32, n: i32) {
    let ok = add_pages_write(a, at, n, |a| compose_all(a, RefreshMode::Ui));
    // Taps made while it was written (the pump read them) were aimed at the pad: none lands on the grid.
    ed(a).pv.quiet_until = Some(now_ms());
    if !ok {
        return;
    }
    let s = ed(a);
    pagesview_untick(&mut s.pv);
    let (cols, rows) = pagesview_grid(&ed_ref(a).pv, a);
    let s = ed(a);
    s.pv.screen = at / (cols * rows).max(1);
    // (its tiles show, marked new, with no toast: the owner, 2026-11-04; one left up from before goes)
    compose_all(a, RefreshMode::Ui);
    app_toast_dismiss(a);
}

// ---- the quick sizes card (2.1, lane L1) ----

// The pen a toolbar button stands for when it is held: a pinned pen, the eraser, or the tool button while it shows
// a pen (not the grid, whose pen is pinned and which opens the picker, nor the lasso).
fn qs_target(s: &EditorState, h: Hit) -> Option<ToolKind> {
    match h.id {
        P_TOOL if (h.arg as u32) < TOOL_COUNT as u32 => Some(ToolKind(h.arg)),
        P_TOOLPICK if s.tool_button_shows >= 0 => Some(ToolKind(s.tool_button_shows)),
        _ => None,
    }
}

// Whether a pen button held opens the card now: nothing in front of the page (a link being made, its strip up,
// does not stop it, as it does not stop a pen tap), and not over the toolbar's count pad.
fn qs_allowed(s: &EditorState) -> bool {
    s.open && !s.add_kb.open && !sheet_open(s) && matches!(s.lm.state, LmState::Off | LmState::Armed)
}

// The held button's pen becomes the pen in use, as its tap would make it (Select mode ends; the eraser in use
// stays, a hold never goes back to the pen before it), the button's press look is let go and the card opens
// beside it, in one Ui. The contact (a finger or the nib) stays the card's unless it was lifted already.
fn qs_hold(a: &mut App, h: Hit, tool: ToolKind, lifted: bool) {
    if h.id == P_TOOL {
        sel_off(a);
    }
    let mut save = false;
    let st = &mut a.settings;
    let changed = st.current_tool != tool;
    if changed {
        if h.id == P_TOOL && tool == TOOL_ERASER {
            st.before_eraser = st.current_tool.0;
        }
        st.current_tool = tool;
        save = true;
        if h.id != P_TOOL {
            sel_off(a);
        }
    }
    if a.settings.hints & HINT_QUICK_SIZE == 0 {
        // (a user who found the card needs no tip about it)
        a.settings.hints |= HINT_QUICK_SIZE;
        save = true;
    }
    if !lifted {
        canvas_invert(&mut a.screen, h.r); // (the press look is let go; the redraw below draws it right)
    }
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let s = ed_ref(a);
    let (panel, zoom) = (s.panel, s.zoom);
    let mut avoid = vec![status_zone(a)];
    if s.lm.state == LmState::Armed {
        avoid.push(s.lm.strip);
    }
    let mut q = QuickSize::default();
    quicksize_open(&mut q, tool, h.r, screen, panel, &avoid, zoom);
    let card = q.rect;
    let s = ed(a);
    s.qs = q;
    s.has_pressed = false;
    if save && s.qs_save == 0 {
        s.qs_save = 1;
    }
    if !lifted {
        s.owner = Owner::QuickSize;
        s.qs.opener = true;
        let p = s.press;
        press_begin(&mut s.qs.press, p.x, p.y, p.t0);
    }
    // (the card with the button, or with the whole panel when the pen changed and its marks moved)
    let u = rect_union(card, if changed { panel } else { h.r });
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

// The card closes with nothing picked (outside it, Esc, Close menu): the page and the panel where it was, one Ui.
fn qs_close(a: &mut App) {
    let s = ed(a);
    if !s.qs.open {
        return;
    }
    let r = s.qs.rect;
    s.qs = QuickSize::default();
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Ui);
}

// What a contact on the card asked for (quicksize.rs). A pick is a setting: the tool's two widths, nothing on the
// page, no history step; settings.ini is written a pass after the card's close was presented (qs_save).
fn qs_result(a: &mut App, r: QsResult, d: Rect) {
    match r {
        QsResult::None => {}
        QsResult::Changed => {
            if !rect_is_empty(d) {
                app_invalidate(a, d, RefreshMode::Ui);
            }
        }
        QsResult::Chip(i) => {
            let t = ed_ref(a).qs.tool;
            quicksize_apply(t, &mut a.settings.tools[t.0 as usize], i);
            ed(a).qs_save = 1;
            qs_close(a);
        }
        QsResult::Mode(m) => {
            a.settings.eraser_mode = m;
            let s = ed(a);
            s.qs_save = 1;
            let r = s.qs.rect;
            compose(a, r);
            app_invalidate(a, r, RefreshMode::Ui);
        }
    }
}

// ---- the toolbar's count pad (owner, 2026-11-03) ----

// Whether the toolbar's + (the next-page button on the last page) held opens the count pad now: nothing in
// front of the page that takes the pad's place (no link being made).
fn add_pad_allowed(s: &EditorState) -> bool {
    s.open && !s.add_kb.open && s.lm.state == LmState::Off && !sheet_open(s)
}

// The screen under the toolbar's count pad, as it is when the pad is not up (its area composed and refreshed).
fn add_pad_area(a: &App) -> Rect {
    keyboard_area(&ed_ref(a).add_kb, Rect { x: 0, y: 0, w: a.w, h: a.h })
}

// The toolbar's + held (HOLD_MS, as the Pages view's New page), clicked with a modifier key held, or the
// new-pages action: the count pad along the bottom, over the page, which stays. A full notebook says so.
fn add_pad_open(a: &mut App) {
    if !add_pad_allowed(ed_ref(a)) {
        return;
    }
    let room = store_room_for_pages(&ed_ref(a).nb);
    if room < 1 {
        // (the Pages view's toast for New page held in a full notebook)
        app_toast(a, &cformat!(tr("Notebook full with %d pages."), STORE_MAX_PAGES as i32), 3000);
        return;
    }
    // (the Pages view's title for its count pad: the same pad)
    keyboard_open_count(&mut ed(a).add_kb, tr("Pages to add"), room, add_label);
    let r = add_pad_area(a);
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Ui);
}

// The count pad goes with nothing added (its Cancel, Esc, Back, or a tap outside it): the page under it again.
fn add_pad_cancel(a: &mut App) {
    if !ed_ref(a).add_kb.open {
        return;
    }
    let r = add_pad_area(a);
    keyboard_close(&mut ed(a).add_kb);
    app_toast_dismiss(a);
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Ui);
}

// What a key of the toolbar's count pad did: Done adds the count after the open page (toolbar_add_batch),
// Cancel closes it. A count held at what the notebook can take shows as that count (no toast: the owner,
// 2026-11-04).
fn add_pad_result(a: &mut App, r: KbResult) {
    match r {
        KbResult::Done => {
            let n = keyboard_count(&ed_ref(a).add_kb);
            toolbar_add_batch(a, n);
        }
        KbResult::Cancel => add_pad_cancel(a),
        _ => {}
    }
}

// A finger or the pen on the toolbar's count pad.
fn add_pad_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    let mut kb = std::mem::take(&mut ed(a).add_kb);
    let r = app_keyboard_pointer(a, &mut kb, ph, ev);
    ed(a).add_kb = kb;
    add_pad_result(a, r);
}

// The toolbar's batch: `n` blank pages right after the open page (on the last page, where the toolbar's +
// adds one: the end), the open page staying open and on screen, the panel's page count showing them; the Pages
// view's bottom row has their Undo (the editor's Undo is the page's own history).
fn toolbar_add_batch(a: &mut App, n: i32) {
    let r = add_pad_area(a);
    keyboard_close(&mut ed(a).add_kb);
    app_toast_dismiss(a);
    let at = ed_ref(a).index + 1;
    let ok = add_pages_write(a, at, n, |a| {
        let r = add_pad_area(a);
        compose(a, r);
        app_invalidate(a, r, RefreshMode::Ui);
    });
    // Contacts made while it was written were aimed at the pad: none lands on the page.
    ed(a).add_quiet_until = Some(now_ms());
    if !ok {
        return;
    }
    // The panel drawn again (its page count, and its + is the next-page arrow now), then the page where the pad
    // was, in one refresh with the two buttons that changed (never the whole panel: the pad covers most of it).
    draw_panel_over(a);
    let u = ed_ref(a).hits.h.iter().filter(|h| h.id == P_PAGES || h.id == P_NEXT).fold(r, |u, h| rect_union(u, h.r));
    compose(a, r);
    app_invalidate(a, u, RefreshMode::Ui);
}

// The bottom row's Undo right after a batch: those pages taken back (store_undo_add_pages) while they are
// still the blank pages it wrote, the open page still open, the view on the screenful of the page before
// them. A record that no longer applies is dropped, never used.
fn pages_undo_add(a: &mut App) {
    if !save_page(a) {
        return;
    }
    let s = ed(a);
    let Some(u) = s.pv.add_undo.take() else { return };
    // A link to one of them, or a name on one, makes them pages in use: not taken back (nor offered).
    let used = pagesview_add_in_use(&u, &s.nav);
    let open = s.open_pid();
    let first = s.nb.page_ids.iter().position(|p| u.added.contains(p)).unwrap_or(0) as i32;
    s.nb.last_page = s.index;
    if used || u.added.contains(&open) || store_undo_add_pages(&mut s.nb, &u) != 0 {
        let s = ed(a);
        if !used && u.applies(&s.nb) && store_added_pages_untouched(&s.nb, &u) && !u.added.contains(&open) {
            s.pv.add_undo = Some(u); // (notebook.ini could not be written: it can be tried again)
        }
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the Pages view's Undo could
        // not take back the pages a batch added (one was written on, linked or named since, or the storage
        // is full).
        app_toast(a, tr("Couldn't undo: pages in use."), 3000);
        compose_all(a, RefreshMode::Ui);
        return;
    }
    let s = ed(a);
    for &pid in &u.added {
        forget_page(s, pid); // (one opened and left blank: its entry goes with it)
    }
    s.pv.redo = Some(PageRedo::Add(u)); // (the same pages again, by the bottom row's Redo or the redo action)
    s.index = s.nb.page_ids.iter().position(|&p| p == open).map_or(s.index, |i| i as i32);
    s.nb.last_page = s.index;
    s.pv.current = s.index;
    s.changed = true;
    pagesview_untick(&mut s.pv);
    let (cols, rows) = pagesview_grid(&ed_ref(a).pv, a);
    let s = ed(a);
    let to = (first - 1).max(0);
    s.pv.screen = to.min(s.nb.npages() - 1) / (cols * rows).max(1);
    compose_all(a, RefreshMode::Ui);
    app_toast_dismiss(a); // (no confirmation; a toast left up from before goes)
}

// The bottom row's Redo, right after an Undo (or the `redo` action over the Pages view): the move again, in the
// exact order it left (store_redo_move_pages), or the batch's blank pages again, with the same ids at the same
// places (store_redo_add_pages), the open page still open, the view on the screenful of the first page it
// changes. The record goes back to Undo (one step each). One that no longer applies is dropped, never used.
// A batch's Redo writes its pages as slowly as the batch did: a big one says "Adding N pages..." as the batch
// did (add_notice_due), on the panel before the writing, and the notice is down at the end, replaced by the
// failure's toast or taken down with the rest.
fn pages_redo(a: &mut App) {
    if !save_page(a) {
        return;
    }
    let s = ed(a);
    let Some(r) = s.pv.redo.take() else { return };
    if !r.applies(&s.nb) {
        return;
    }
    let batch = match &r {
        PageRedo::Add(u) => u.added.len() as i32,
        PageRedo::Move(_) => 0,
    };
    let notice = batch > 0 && add_notice_due(a, batch);
    if notice {
        app_toast(a, &add_notice_text(batch), 120_000);
        add_notice_flush(a, true);
    }
    let started = now_ms();
    let s = ed(a);
    let open = s.open_pid();
    let first_was = s.nb.page_ids[0];
    s.nb.last_page = s.index;
    let rc = match &r {
        PageRedo::Move(u) => store_redo_move_pages(&mut s.nb, u),
        PageRedo::Add(u) => store_redo_add_pages(&mut s.nb, u),
    };
    // (a large batch's time again, as add_pages_write logs the batch's)
    if batch >= ADD_BUSY_PAGES || notice || (batch > 0 && rc != 0) {
        qlog!("pages: redo added=%d of %d ms=%d notice=%d", if rc != 0 { 0 } else { batch }, batch,
            now_ms().wrapping_sub(started) as i32, notice as i32);
    }
    if rc != 0 {
        let s = ed(a);
        if r.applies(&s.nb) {
            s.pv.redo = Some(r); // (a file could not be written: it can be tried again)
        }
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the Pages view's Redo could not
        // do the page move or the new pages again (the storage is full).
        app_toast(a, tr("Couldn't redo."), 3000); // (in the notice's place)
        compose_all(a, RefreshMode::Ui);
        return;
    }
    let s = ed(a);
    s.index = s.nb.page_ids.iter().position(|&p| p == open).map_or(s.index, |i| i as i32);
    s.nb.last_page = s.index;
    s.pv.current = s.index;
    s.changed = true;
    if s.nb.page_ids[0] != first_was && s.index == 0 {
        s.thumb_dirty = true;
    }
    let npages = s.nb.npages() as usize;
    if s.pv.sel.len() < npages {
        s.pv.sel.resize(npages, false);
    }
    pagesview_untick(&mut s.pv);
    let at = match r {
        PageRedo::Move(u) => {
            let at = u.before.iter().zip(&u.after).position(|(b, f)| b != f).unwrap_or(0) as i32;
            s.pv.move_undo = Some(u);
            at
        }
        PageRedo::Add(u) => {
            let at = s.nb.page_ids.iter().position(|p| u.added.contains(p)).unwrap_or(0) as i32;
            s.pv.add_undo = Some(u);
            at
        }
    };
    let (cols, rows) = pagesview_grid(&ed_ref(a).pv, a);
    let s = ed(a);
    s.pv.screen = at / (cols * rows).max(1);
    compose_all(a, RefreshMode::Full);
    app_toast_dismiss(a); // (no confirmation; the notice, or a toast left up from before, a failed try's, goes)
}

// The `undo` and `redo` actions (keys, a controller, gestures) with the Pages view's grid in front: its own Undo
// and Redo, never the open page's history (and the editor's never these: the owner, 2026-11-03, "keep them
// seperate to each other"). Nothing to undo or redo does nothing. True when the view took the action.
fn pages_history_action(a: &mut App, action: i32) -> bool {
    let s = ed(a);
    let pv = &s.pv;
    if !pv.open || pv.kb.open || pv.index_tab || pv.picking || pv.overlay || pv.name_only || !pv.unified {
        return false;
    }
    if !pagesview_quiet(pv) {
        return true; // (a held tile's card, or a drag under way: the action is taken and does nothing)
    }
    if action == SHORTCUT_UNDO {
        match pagesview_undo_result(&s.pv, &s.nb, &s.nav) {
            Some(PvResult::UndoAdd) => pages_undo_add(a),
            Some(_) => pages_undo_move(a),
            None => {}
        }
    } else if pagesview_redo_offered(&s.pv, &s.nb) {
        pages_redo(a);
    }
    true
}

// Copy to notebook... (a held page's card, or the bottom row with pages ticked): the notebook picker
// over the Pages view, the open notebook left out; a page tile there or At the end says where.
fn pages_copy_to_pick(a: &mut App) {
    if !save_page(a) {
        return;
    }
    links_flush(a); // (the copies take the links as saved, migrated areas too)
    let s = ed(a);
    s.copy_pids = (0..s.nb.npages()).filter(|&i| pagesview_ticked(&s.pv, i)).map(|i| s.nb.page_ids[i as usize]).collect();
    if s.copy_pids.is_empty() {
        return;
    }
    let (folder, id) = (s.nb.folder.as_str().to_string(), s.nb.id.as_str().to_string());
    if with_modal(a, |s| &mut s.destination, |p, _| p.start_copy(&folder, &id)) {
        app_toast_dismiss(a);
        compose_all(a, RefreshMode::Full);
    } else {
        ed(a).copy_pids.clear();
        app_toast(a, tr("Couldn't read notebooks."), 3000);
    }
}

// The copy itself (store_copy_pages_to, then nav_copy_pages_across): pages `pids` into notebook `id`
// before its page `at`. That notebook's compaction stops first, as when a notebook opens; its card is
// drawn again when the copies come first, and its PDF mirror is made again.
fn pages_copy_to(a: &mut App, id: &str, at: i32, pids: &[u32]) {
    let dpi = a.dpi;
    compose_all(a, RefreshMode::Full);
    if pids.is_empty() {
        return;
    }
    bg_cancel_notebook(id);
    let mut dst = NotebookInfo::default();
    let mut out = Vec::new();
    let s = ed(a);
    if store_load_info(id, &mut dst) != 0 || dst.page_ids.len() + pids.len() > STORE_MAX_PAGES
        || store_copy_pages_to(&s.nb, pids, &mut dst, at, &mut out) != 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Copy to notebook... copied
        // nothing (the storage is full, a page does not read, or that notebook has 2000 pages).
        app_toast(a, tr("Couldn't copy pages."), 3000);
        return;
    }
    let (mut sw, mut sh, mut dw, mut dh) = (0, 0, 0, 0);
    store_notebook_page_size(&s.nb, &mut sw, &mut sh);
    store_notebook_page_size(&dst, &mut dw, &mut dh);
    let mut res = NavCopyResult::default();
    let nav_ok = nav_copy_pages_across(&s.nav, s.nb.id.as_str(), &out, &dst, dw as f32 / sw.max(1) as f32, dh as f32 / sh.max(1) as f32, &mut res) == 0;
    // The document links of the pages whose targets were copied too (a page out of a planner loses the tabs to
    // pages left behind); a notebook whose doclinks.ini does not write gets the pages without them.
    let nav_ok = doclinks_copy_across(&s.dl, &out, &dst, dw as f32 / sw.max(1) as f32, dh as f32 / sh.max(1) as f32) >= 0 && nav_ok;
    for c in out.iter().filter(|c| c.erased) {
        if compact_note_page(id, c.to) != 0 {
            qlog!("compact: %s: p%u.qpg (a copy) could not be listed", id, c.to);
        }
    }
    if at == 0 {
        // The copies come first: the Library's card of that notebook from its new first page.
        let mut p0: Option<Box<Page>> = None;
        if store_read_page(&dst, 0, &mut p0) == 0 {
            let mut t = Canvas::default();
            if canvas_init(&mut t, dw / 3, dh / 3) == 0 {
                page_render_thumb(&mut t, p0.as_deref().unwrap(), dpi);
                store_save_thumb(id, &t);
            }
        }
    }
    if a.settings.mirror_pdf {
        bg_mirror_pdf(id, dpi, &app_mirror_dir(), false);
    }
    if compact_listed(id) {
        bg_queue(BgKind::CompactPages, Some(id), None, dpi, false);
    }
    pagesview_untick(&mut ed(a).pv);
    compose_all(a, RefreshMode::Full);
    // (done, no toast: the owner, 2026-11-04; only names and links that could not be saved there say so, and a
    // toast left up from before goes)
    app_toast_dismiss(a);
    if !nav_ok {
        let n = out.len() as i32;
        // TRANSLATORS: a toast as Copy to notebook... is done (about 1700 px at 44 px in bold, or two lines): %d
        // pages were copied into the notebook named %s, but their page names and links could not be saved there.
        let msg = cformat!(ng_("Copied %d page to %s, without its names and links.", "Copied %d pages to %s, without their names and links.", n as i64), n, dst.name.as_str());
        app_toast(a, &msg, 3000);
    }
}

// ---- making a link without ink (PHASE2 6) ----

/// A link as an undo step keeps it (history.rs stays free of the Nav).
fn link_rec(l: &NavLink) -> LinkRec {
    LinkRec { seed: l.seed, target: l.target, area: l.area, has_area: l.has_area, outline:l.outline,
    }
}
/// The link a step's record puts back on page pid.
fn rec_link(pid: u32, r: &LinkRec) -> NavLink {
    NavLink { pid, seed: r.seed, target: r.target, area: r.area, has_area: r.has_area, outline:r.outline,
    }
}

/// After a nav_load rollback (a failed link save): the links in memory are the file's again, so migrated
/// areas no longer wait for a save, and every page with a legacy link may be tried again (idle ticks).
fn migration_reset(s: &mut EditorState) {
    s.links_dirty = false;
    s.migrate_done.clear(); // (its room stays: migrate_room)
    s.boxes.stale = true;
}

/// Link making ends with nothing drawn: for a caller that draws the whole screen next (a page change, the
/// Pages view coming up, closing). A pen still drawing the lasso has the rest of its stroke ignored; Link
/// to... and New page...'s keyboard close with it.
fn lm_drop(s: &mut EditorState) {
    sel_drop(s);
    if !s.destination.copying {
        s.destination=NotebookPicker::default(); // (Copy to notebook...'s picker is the Pages view's, not link making's)
    }
    if s.owner == Owner::Lasso || s.owner == Owner::Strip || s.owner == Owner::LinkTo {
        s.owner = Owner::None;
    }
    if s.find.open && s.find.mode == FindMode::Pick {
        find_drop(s); // (Link to...'s Search)
    }
    s.lt = LinkBar::default();
    // (a retarget or a redraw the card began: its border goes with it)
    if (!s.lm.retarget.is_empty() || s.lm.redraw.is_some()) && !s.card.open {
        s.card.border = false;
        s.card.boxes.clear();
    }
    s.lm = LinkMaking::default();
}

/// Link making ends silently with the strip, the lasso and the dotted rectangle taken off the screen in
/// one Ui refresh (a menu opening over the page: its own refresh joins it). Nothing when none is made.
fn link_off(a: &mut App) {
    if ed_ref(a).lm.state == LmState::Off {
        return;
    }
    let u = lm_bounds(a);
    lm_drop(ed(a));
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

/// Where link making draws now on the screen: the strip, the lasso so far, the dotted rectangle, Link to...
/// and New page...'s keyboard.
fn lm_bounds(a: &App) -> Rect {
    let s = ed_ref(a);
    if s.destination.open { return Rect::new(0, 0, a.w, a.h); }
    let mut u = RECT_EMPTY;
    if s.lm.state == LmState::Armed || s.lm.state == LmState::Lasso {
        u = rect_union(s.lm.strip, strip_rect(a));
    }
    u = rect_union(u, s.lm.lasso_rect);
    u = rect_union(u, area_band(a));
    if s.lt.open && !s.lt.hidden {
        u = rect_union(u, s.lt.rect);
    }
    if s.lm.state == LmState::Naming && s.lm.kb.open {
        u = rect_union(u, s.lm.kb.area);
    }
    if s.find.open && s.find.mode == FindMode::Pick {
        u = rect_union(u, rect_union(s.find.rect, s.find.kb.area));
    }
    if (!s.lm.retarget.is_empty() || s.lm.redraw.is_some()) && s.card.border && !s.card.open {
        u = rect_union(u, card_band(a)); // (the border the card left on the links)
    }
    rect_intersect(u, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    )
}

/// The strip while a link is being made (6.3): along the bottom, 20 px in from the sides and above the edge.
/// Above the clock corner when it is at the bottom (rule 12: it never meets the corner's zone).
fn strip_rect(a: &App) -> Rect {
    let h = ui_px(130.0);
    Rect { x: ui_px(20.0), y: dock_bottom_y(a) - h, w: a.w - ui_px(40.0), h ,
    }
}

/// The strip's Cancel: 260 x 100 at its right, 15 px in.
fn strip_cancel(strip: Rect) -> Rect {
    let (w, h, m) = (ui_px(260.0), ui_px(100.0), ui_px(15.0));
    Rect { x: strip.x + strip.w - m - w, y: strip.y + m, w, h ,
    }
}

/// The strip, over the page, the panel and the menus, where r meets it; the toasts keep above it.
fn draw_strip(a: &mut App, r: Rect) {
    let strip = strip_rect(a);
    let s = ed(a);
    s.lm.strip = strip;
    let pressed = s.lm.cancel_down;
    let redraw = s.lm.redraw.is_some();
    if rect_overlaps(r, strip) {
        let clip = a.screen.clip;
        canvas_set_clip(&mut a.screen, r);
        let c = &mut a.screen;
        ui_frame(c, strip, ui_px(24.0), 255, true, 0);
        ui_frame(c, strip, ui_px(24.0), 0, false, ui_px(6.0));
        let cancel = strip_cancel(strip);
        let tx = strip.x + ui_px(32.0);
        let text = Rect { x: tx, y: strip.y, w: cancel.x - ui_px(32.0) - tx, h: strip.h ,
        };
        // TRANSLATORS: the strip along the bottom while a link is made (about 1400 px at 44 px in bold):
        // the next pen stroke draws a ring around what the link is, and leaves no ink.
        let line = if redraw {
            // TRANSLATORS: the same strip for the long-press card's Redraw the area (about 1400 px at 44 px in
            // bold): the next pen stroke draws the link's new area, and leaves no ink.
            tr("Draw the link's new area.")
        } else {
            tr("Draw around what links.")
        };
        font_draw_left(c, FontFace::Bold, ui_text_size(), text, line, 0);
        ui_button(c, cancel, IconKind::None, tr("Cancel"), if pressed { ButtonStyle::Selected } else { ButtonStyle::Plain },
        );
        a.screen.clip = clip;
    }
    a.toast_floor = strip.y;
}

/// An area waits for where its link goes: Link to... (or what it opened) is up, and the dotted rectangle shows.
fn area_waits(s: &EditorState) -> bool {
    (s.lm.state == LmState::Pick || s.lm.state == LmState::Naming) && !rect_is_empty(s.lm.area)
}

/// The screen rect of the dotted rectangle around the area waiting for its target, and the band it paints
/// (its halo included).
fn area_screen(s: &EditorState) -> Rect {
    box_screen(s, s.lm.area)
}
fn area_band(a: &App) -> Rect {
    let s = ed_ref(a);
    if !area_waits(s) {
        return RECT_EMPTY;
    }
    rect_inflate(area_screen(s), link_band() / 2 + ui_px(1.0) + 1)
}

/// A page point on the screen (screen = (page - o) * zoom), unrounded: the lasso drawn as the pen moves and
/// the one compose draws whole go through this one conversion, so their pixels agree.
fn lasso_screen_pt(s: &EditorState, p: (f32, f32)) -> (f32, f32) {
    ((p.0 - s.ox) * s.zoom, (p.1 - s.oy) * s.zoom)
}

/// Where the lasso may draw on the screen: the page, below the status bar.
fn lasso_clip(a: &App) -> Rect {
    let s = ed_ref(a);
    let top = status_reserved(a);
    let page = page_to_screen(a, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
        },
    );
    rect_intersect(page, Rect { x: 0, y: top, w: a.w, h: a.h - top ,
        },
    )
}

/// The lasso's dashes (6.4): 4 px, 12 on and 8 off, black, no halo.
fn lasso_pen() -> (i32, i32, i32) {
    (link_band(), ui_px(12.0), ui_px(8.0)) // (its dash lengths, unused since the band is dithered whole: rc.12)
}

/// Whether point p (page px) joins the lasso after `last`: it must be `spacing` screen px from it. A full
/// lasso drops every other point first and doubles the spacing; a lasso with no room (its reservation
/// failed) keeps no points but still goes on from the new one. Returns whether p was taken.
fn lasso_keep(pts: &mut Vec<(f32, f32)>, spacing: &mut f32, last: &mut (f32, f32), p: (f32, f32), zoom: f32,
) -> bool {
    let (dx, dy) = ((p.0 - last.0) * zoom, (p.1 - last.1) * zoom);
    if dx * dx + dy * dy < *spacing * *spacing {
        return false;
    }
    if pts.len() >= LASSO_MAX {
        let mut k = 0;
        pts.retain(|_| {
            k += 1;
            k % 2 == 1
        });
        *spacing *= 2.0;
    }
    if pts.len() < pts.capacity() {
        pts.push(p);
    }
    *last = p;
    true
}

/// The area a lasso's box gives: whole page px around it (floor, ceil), on the page.
fn lasso_area(bx0: f32, by0: f32, bx1: f32, by1: f32, pgw: i32, pgh: i32) -> Rect {
    let x0 = clampi(bx0.floor() as i32, 0, pgw);
    let y0 = clampi(by0.floor() as i32, 0, pgh);
    let x1 = clampi(bx1.ceil() as i32, 0, pgw);
    let y1 = clampi(by1.ceil() as i32, 0, pgh);
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 ,
    }
}

/// A lasso smaller than LINK_BOX_MIN_MM both ways (at this dpi) makes no area.
fn lasso_too_small(r: Rect, dpi: f32) -> bool {
    let min_px = LINK_BOX_MIN_MM * dpi / 25.4;
    r.w <= 0 || r.h <= 0 || ((r.w as f32) < min_px && (r.h as f32) < min_px)
}

/// Add a link, from the Index, the bar's New link or the add-link shortcut, or the card's Redraw the area for
/// the link `redraw`: the strip comes up and the next pen contact on the open page draws the lasso. Refused,
/// with a toast, on a read-only page, while links.ini does not read, and (a new link) on a page that has all
/// the links it may.
fn link_arm(a: &mut App, from_bar: bool, redraw: Option<u32>) {
    if refuse_readonly(a) {
        return;
    }
    let s = ed(a);
    if !s.nav.links_writable {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Add a link was asked while
        // the notebook's links.ini could not be read, so no link can be made.
        app_toast(a, tr("Couldn't read links in this notebook."), 3000);
        return;
    }
    let pid = s.open_pid();
    if redraw.is_none() && nav_links_of(&s.nav, pid, &mut 0) >= NAV_LINKS_PER_PAGE {
        let mut msg = String::new();
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Add a link on a page that
        // has the most links a page may have; %d is that number (64).
        csnprintf!(&mut msg, 160, tr("Link limit (%d) reached: delete one first."), NAV_LINKS_PER_PAGE);
        app_toast(a, &msg, 3000);
        return;
    }
    let strip = strip_rect(a);
    let s = ed(a);
    s.lm = LinkMaking { state: LmState::Armed, strip, from_bar, redraw, ..Default::default() };
    compose(a, strip);
    app_invalidate(a, strip, RefreshMode::Ui);
}

/// A pen down on the page while armed: the lasso begins there (never pen mode, never the live ink style).
fn lasso_begin(a: &mut App, ev: &InEvent) {
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, ev.x, ev.y, &mut px, &mut py);
    s.owner = Owner::Lasso;
    let lm = &mut s.lm;
    lm.state = LmState::Lasso;
    lm.pts.clear();
    lm.shape_preview=None;lm.shape_kind=ShapeKind::None;lm.shape_tried=false;lm.outline=LinkOutline::default();
    lm.shape_anchor=(ev.x,ev.y);lm.shape_anchor_k=in_sample_ms(ev);
    if lm.pts.capacity() < LASSO_MAX {
        // (no room: the lasso is drawn as it goes and its box still makes the area; compose draws none of it)
        let _ = lm.pts.try_reserve_exact(LASSO_MAX);
    }
    if lm.pts.capacity() > 0 {
        lm.pts.push((px, py));
    }
    lm.last = (px, py);
    lm.spacing = ui_px(2.0) as f32;
    lm.phase = 0;
    lm.bx0 = px;
    lm.bx1 = px;
    lm.by0 = py;
    lm.by1 = py;
    lm.lasso_rect = RECT_EMPTY;
}

/// The pen moved while drawing the lasso: the new segment goes straight to the screen with a DU (one per
/// input batch, at the loop's flush), and never into the page. A segment under the panel or the strip is
/// kept but not drawn (compose draws the lasso under them).
fn lasso_move(a: &mut App, ev: &InEvent) {
    let dpi=a.dpi;
    if ed_ref(a).lm.shape_preview.is_some() {
        if !shape_anchor_moved(ed_ref(a).lm.shape_anchor,(ev.x,ev.y),dpi,true) {return;}
        let s=ed(a);s.lm.shape_preview=None;s.lm.shape_kind=ShapeKind::None;
        let r=s.lm.lasso_rect;compose(a,r);app_invalidate(a,r,RefreshMode::Fast);
    }
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, ev.x, ev.y, &mut px, &mut py);
    let lm = &mut s.lm;
    if shape_anchor_moved(lm.shape_anchor,(ev.x,ev.y),dpi,false) {
        lm.shape_anchor=(ev.x,ev.y);lm.shape_anchor_k=in_sample_ms(ev);lm.shape_tried=false;
    }
    lm.bx0 = lm.bx0.min(px);
    lm.bx1 = lm.bx1.max(px);
    lm.by0 = lm.by0.min(py);
    lm.by1 = lm.by1.max(py);
    let prev = lm.last;
    let zoom = s.zoom;
    if !lasso_keep(&mut lm.pts, &mut lm.spacing, &mut lm.last, (px, py), zoom) {
        return;
    }
    let (p0, p1) = (lasso_screen_pt(s, prev), lasso_screen_pt(s, (px, py)));
    let (thick, on, off) = lasso_pen();
    let seg = rect_from_points(p0.0, p0.1, p1.0, p1.1, thick as f32 + 1.0);
    let hidden = rect_overlaps(seg, s.panel) || rect_overlaps(seg, s.lm.strip);
    let phase = s.lm.phase;
    let clip = lasso_clip(a);
    let keep = a.screen.clip;
    canvas_set_clip(&mut a.screen, clip);
    let _ = (on, off, phase);
    canvas_dither_poly(&mut a.screen, &[p0, p1], if hidden { 0 } else { thick }, 0);
    let ph = 0; // (a continuous band: no dash phase to carry)
    a.screen.clip = keep;
    let d = rect_intersect(seg, clip);
    let s = ed(a);
    s.lm.phase = ph;
    s.lm.lasso_rect = rect_union(s.lm.lasso_rect, d);
    if !hidden && !rect_is_empty(d) {
        app_invalidate(a, d, RefreshMode::Fast);
    }
}

/// The whole lasso, as compose draws it (over the eye's boxes, under the panel), where r meets it.
fn draw_lasso(a: &mut App, r: Rect) {
    let s = ed_ref(a);
    if s.lm.state != LmState::Lasso || s.lm.pts.len() < 2 {
        return;
    }
    let pts: Vec<(f32, f32)> = s.lm.shape_preview.as_ref().unwrap_or(&s.lm.pts).iter().map(|&p| lasso_screen_pt(s, p)).collect();
    let (thick, on, off) = lasso_pen();
    let clip = a.screen.clip;
    let c = rect_intersect(r, lasso_clip(a));
    canvas_set_clip(&mut a.screen, c);
    let _ = (on, off);
    canvas_dither_poly(&mut a.screen, &pts, thick, 0);
    a.screen.clip = clip;
}

/// The dotted rectangle around the area waiting for its target (6.5), where r meets it.
fn draw_area_wait(a: &mut App, r: Rect) {
    let s = ed_ref(a);
    if !area_waits(s) {
        return;
    }
    let sr = area_screen(s);
    let (outline,area,view)=(s.lm.outline,s.lm.area,(s.zoom,s.ox,s.oy));
    let clip = a.screen.clip;
    canvas_set_clip(&mut a.screen, r);
    if outline.n>0 {draw_outline(&mut a.screen,outline,area,area,view);}
    else {canvas_dither_rect(&mut a.screen,sr,link_band(),ui_px(1.0));}
    a.screen.clip = clip;
}

/// Link selection uses the same shape policies as ink, but the snapped outline stays an overlay (and
/// keeps sharp corners: Corner roundness is the ink's).
fn link_shape_snap(a: &mut App,allow_open:bool) -> bool {
    if !a.settings.shapes || !a.settings.shape_links {return false;}
    let s=ed_ref(a);
    if s.lm.state!=LmState::Lasso {return false;}
    let mut pts=Vec::new();
    if pts.try_reserve_exact(s.lm.pts.len()).is_err() {return false;}
    for (i,&(x,y)) in s.lm.pts.iter().enumerate() {
        pts.push(PenSample{x,y,pressure:0.5,t_ms:(i as u32)*10,..Default::default()});
    }
    let mut prm=shape_params(a.dpi,s.zoom,s.open_page().unwrap().paper,s.pgw);
    // (every Smart shapes setting, as ink snaps with it: Four sides, Round shapes, Corner roundness, Smallest shape)
    prm.quad_mode=a.settings.shape_quads;prm.circle_mode=a.settings.shape_round;
    prm.corner=a.settings.shape_corner as f32/100.0;prm.min_mm=a.settings.shape_min_mm as f32;
    let result=shape_recognize(&pts,&prm);
    let closed=matches!(result.kind,ShapeKind::Rectangle|ShapeKind::Square|ShapeKind::Quad|ShapeKind::Triangle|ShapeKind::Circle|ShapeKind::Ellipse);
    if result.kind==ShapeKind::None || (!closed && !allow_open) {return false;}
    let clean:Vec<_>=result.pts.iter().map(|p|(p.x,p.y)).collect();
    let s=ed(a);s.lm.shape_preview=Some(clean);s.lm.shape_kind=result.kind;
    true
}

fn link_shape_tick(a: &mut App,now:u32) {
    let s=ed_ref(a);
    if s.owner!=Owner::Lasso || s.lm.shape_tried || s.lm.shape_preview.is_some() || !a.input_drained
        || now.wrapping_sub(s.lm.shape_anchor_k)<a.settings.shape_hold_ms as u32 {return;}
    ed(a).lm.shape_tried=true;
    if !link_shape_snap(a,true) {return;}
    let s=ed_ref(a);
    let mut r=s.lm.lasso_rect;
    for &p in s.lm.shape_preview.as_ref().unwrap() {
        let (x,y)=lasso_screen_pt(s,p);
        r=rect_union(r,Rect{x:x.floor() as i32-4,y:y.floor() as i32-4,w:9,h:9});
    }
    ed(a).lm.lasso_rect=r;
    compose(a,r);app_invalidate(a,r,RefreshMode::Fast);
}

/// The pen lifted (or left) while drawing the lasso (6.5): its box is the area. Too small, and the strip
/// stays for another try; otherwise the lasso and the strip go, the dotted rectangle shows the area (a GL16
/// flushed on its own), and Link to... opens to pick where the link goes (a GL16 of its card).
fn lasso_end(a: &mut App) {
    if ed_ref(a).lm.shape_preview.is_none() { link_shape_snap(a,false); }
    let dpi = a.dpi;
    let s = ed(a);
    s.owner = Owner::None;
    if s.lm.state != LmState::Lasso {
        return;
    }
    // (every attempt sets its own outline: one left by a snapped try that was too small never reaches the next)
    s.lm.outline=LinkOutline::default();
    if let Some(pts)=s.lm.shape_preview.take() {
        let closed=matches!(s.lm.shape_kind,ShapeKind::Rectangle|ShapeKind::Square|ShapeKind::Quad|ShapeKind::Triangle|ShapeKind::Circle|ShapeKind::Ellipse);
        s.lm.outline=LinkOutline::from_points(&pts,closed,s.pgw,s.pgh);
        if let Some(&(x,y))=pts.first() {
            s.lm.bx0=x;s.lm.bx1=x;s.lm.by0=y;s.lm.by1=y;
            for (x,y) in pts {s.lm.bx0=s.lm.bx0.min(x);s.lm.bx1=s.lm.bx1.max(x);s.lm.by0=s.lm.by0.min(y);s.lm.by1=s.lm.by1.max(y);}
        }
    }
    let lm = &s.lm;
    let mut r = lasso_area(lm.bx0, lm.by0, lm.bx1, lm.by1, s.pgw, s.pgh);
    if lm.outline.n>0 {r.w=r.w.max(1);r.h=r.h.max(1);}
    let drawn = lm.lasso_rect;
    if lasso_too_small(r, dpi) {
        lasso_cancel(a);
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): after Add a link, the
        // stroke drawn was too small to be the link's box.
        app_toast(a, tr("Link area too small."), 3000);
        return;
    }
    if let Some(seed) = s.lm.redraw {
        redraw_done(a, seed, r, drawn);
        return;
    }
    let strip = s.lm.strip;
    s.lm.state = LmState::Pick;
    s.lm.area = r;
    s.lm.pts = Vec::new();
    s.lm.lasso_rect = RECT_EMPTY;
    let u = rect_union(rect_union(drawn, strip), area_band(a));
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
    // (on its own: the rectangle's refresh and Link to...'s never merge into one union across the screen)
    app_flush(a);
    lt_open(a);
}

/// The lasso is given up and the strip waits for another (Armed), its dashes (and a snapped shape) taken off
/// the screen in a Ui refresh: a lasso too small for a link (lasso_end), and one cancelled (owner_cancel).
fn lasso_cancel(a: &mut App) {
    let s = ed(a);
    s.owner = Owner::None;
    if s.lm.state != LmState::Lasso {
        return;
    }
    let drawn = s.lm.lasso_rect;
    s.lm.state = LmState::Armed;
    s.lm.shape_preview = None;
    s.lm.shape_kind = ShapeKind::None;
    s.lm.outline = LinkOutline::default();
    s.lm.pts.clear();
    s.lm.lasso_rect = RECT_EMPTY;
    compose(a, drawn);
    app_invalidate(a, drawn, RefreshMode::Ui);
}

/// Where a card or keyboard docked at the top of the screen begins: 8 px below the clock corner when the
/// corner is at the top (its square and the indicator's box with the room a tap has around it, status_zone),
/// and below the status bar while it is up.
fn dock_top_y(a: &App) -> i32 {
    let z = status_zone(a);
    let mut y = status_reserved(a);
    if !rect_is_empty(z) && z.y < a.h / 2 {
        y = y.max(z.y + z.h);
    }
    y + ui_px(8.0)
}

/// Where a surface docked at the bottom of the screen (the strip, Link to...) ends: 20 px above the edge, and 8 px
/// above the clock corner's zone when the corner is at the bottom, so nothing this phase adds meets it (rule 12).
fn dock_bottom_y(a: &App) -> i32 {
    let z = status_zone(a);
    let mut y = a.h - ui_px(20.0);
    if !rect_is_empty(z) && z.y >= a.h / 2 {
        y = y.min(z.y - ui_px(8.0));
    }
    y
}

/// The Recent targets Link to... is offered, newest first: the targets links were made to this session,
/// then the back list's pages (linkbar_build_pick leaves out the open page, repeats and pages gone).
fn recent_targets(s: &EditorState) -> Vec<FixedStr<NAV_TARGET_LEN>> {
    let mut v: Vec<FixedStr<NAV_TARGET_LEN>> = Vec::new();
    // (a few dozen at most; with no room the chips are just not offered)
    if v.try_reserve_exact(s.recent.len() + s.back.len()).is_err() {
        return v;
    }
    v.extend(s.recent.iter().rev().copied());
    for &pid in s.back.iter().rev() {
        let mut t = String::new();
        csnprintf!(&mut t, NAV_TARGET_LEN, "p%u", pid);
        v.push(FixedStr::from_str(&t));
    }
    v
}

/// Link to... opens for the area that waits (6.6): docked at the top, below the clock corner, when the area
/// is in the lower 45% of the screen, else at the bottom; one GL16 of its card.
fn lt_open(a: &mut App) {
    let sr = area_screen(ed_ref(a));
    lt_open_for(a, sr.y + sr.h / 2, "", false);
}

/// Link to... for what is at screen line centre_y (an area, or the card's border): docked away from it.
/// `current` marks the target the links have now (a retarget), `no_pages` leaves Choose from the pages... out.
fn lt_open_for(a: &mut App, centre_y: i32, current: &str, no_pages: bool) {
    let (aw, ah) = (a.w, a.h);
    let (top_y, bottom_y) = (dock_top_y(a), dock_bottom_y(a));
    let s = ed(a);
    let top = linkbar_pick_docks_top(centre_y, ah);
    let recent = recent_targets(s);
    let mut lt = LinkBar::default();
    if linkbar_build_pick(&mut lt, &s.nb, &s.nav, &recent, s.open_pid(), current, no_pages,
    ) < 0 {
        qlog!("index: no memory to list the Index for a link");
    }
    linkbar_layout_pick(&mut lt, aw, ah, top, top_y, bottom_y);
    let rect = lt.rect;
    s.lt = lt;
    compose(a, rect);
    app_invalidate(a, rect, RefreshMode::Ui);
}

/// What a tap on Link to... asked (6.6): a chip picks (New page... names a page first), Choose from the
/// pages... opens the Pages view, Cancel makes no link, the pager shows another screenful.
fn lt_result(a: &mut App, r: LbResult, d: Rect) {
    if !rect_is_empty(d) {
        app_invalidate(a, d, RefreshMode::Ui);
    }
    match r {
        LbResult::Pick(i) => {
            let Some(p) = ed_ref(a).lt.picks.get(i as usize).copied() else { return ;
            };
            if p.kind == PickKind::Index {destination_open(a);}
            else if p.kind == PickKind::NewPage {
                newpage_start(a);
            } else {
                let target = p.target.as_str().to_string();
                link_pick(a, &target);
            }
        }
        LbResult::Search => find_pick_open(a),
        LbResult::Choose => link_choose(a),
        LbResult::Cancel => link_cancel(a),
        LbResult::Pager => {
            let g = ed(a).lt.grid;
            compose(a, g);
            app_invalidate(a, g, RefreshMode::Ui);
        }
        _ => {}
    }
}

/// Choose from the pages... (6.9): the Pages view picks, Link to... waiting hidden under it; its Back
/// comes back to Link to... (pages_event).
fn link_choose(a: &mut App) {
    if ed_ref(a).lt.no_pages {
        return;
    }
    // (no "Where does the link go?": the owner, 2026-11-04. A toast left up goes first, so a page that could not
    // be saved still says so.)
    app_toast_dismiss(a);
    ed(a).lt.hidden = true;
    save_page(a);
    let index = ed(a).index;
    ed(a).pv.unified=true;
    if with_pv(a, |pv, a, nb, nav| pagesview_pick(pv, a, nb, nav, index)) {
        pages_seed_open(ed(a));
    } else {
        let s = ed(a);
        s.lt.hidden = false;
        let r = s.lt.rect;
        compose(a, r);
        app_invalidate(a, r, RefreshMode::Ui);
        return;
    }
    compose_all(a, RefreshMode::Full);
}

/// New page... (6.8): a keyboard names the page to add, docked where Link to... is (at the top exactly when it
/// is), Link to... hidden under it. Without page names that can be saved the page is added unnamed at once.
fn newpage_start(a: &mut App) {
    if !ed_ref(a).nav.writable {
        newpage_done(a, "");
        return;
    }
    let dock = if ed_ref(a).lt.top { dock_top_y(a) } else { 0 };
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h ,
    };
    let s = ed(a);
    s.lm.state = LmState::Naming;
    s.lt.hidden = true;
    // TRANSLATORS: the title of the keyboard that names the page New page... adds for a link (about 1500 px at
    // 44 px).
    keyboard_open(&mut s.lm.kb, tr("Name the new page"), b"");
    keyboard_set_dock(&mut s.lm.kb, dock);
    s.lm.kb.area = keyboard_modal_rect_docked(screen, dock);
    let u = rect_union(s.lt.rect, s.lm.kb.area);
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

/// New page...'s Cancel, or a tap outside its keyboard: no page is added, and Link to... is back.
fn naming_back(a: &mut App) {
    let s = ed(a);
    if s.lm.state != LmState::Naming {
        return;
    }
    let u = rect_union(s.lm.kb.area, s.lt.rect);
    keyboard_close(&mut s.lm.kb);
    s.lm.state = LmState::Pick;
    s.lt.hidden = false;
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

/// New page...'s Done: a page at the end of the notebook, on its paper, named `name` (when there is one and
/// the names can be saved), and the link goes to it. The editor stays on its page. Undo takes the link away,
/// not the page.
fn newpage_done(a: &mut App, name: &str) {
    let s = ed(a);
    let kb_area = if s.lm.kb.open { s.lm.kb.area } else { RECT_EMPTY };
    keyboard_close(&mut s.lm.kb);
    if s.lm.state == LmState::Naming {
        s.lm.state = LmState::Pick;
    }
    s.lt.hidden = false;
    let n = s.nb.npages();
    let paper = s.open_page().map_or(s.nb.paper, |p| p.paper);
    if store_add_page(&mut s.nb, n, paper) != 0 {
        let u = rect_union(kb_area, s.lt.rect);
        compose(a, u);
        app_invalidate(a, u, RefreshMode::Ui);
        app_toast(a, tr("Couldn't add page"), 3000);
        return;
    }
    let pid = s.nb.page_ids[n as usize];
    s.changed = true;
    let mut name_failed = false;
    if !name.is_empty() && s.nav.writable && (nav_set_name(&mut s.nav, pid, name) != 0 || nav_save(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0) {
        nav_set_name(&mut s.nav, pid, ""); // (it stays unnamed)
        name_failed = true;
    }
    compose(a, kb_area);
    app_invalidate(a, kb_area, RefreshMode::Ui);
    let mut target = String::new();
    csnprintf!(&mut target, NAV_TARGET_LEN, "p%u", pid);
    link_pick_as(a, &target, n + 1);
    if name_failed {
        app_toast(a, tr("Couldn't save page name."), 3000);
    }
}

/// A finger or the pen on New page...'s keyboard (typing through the keyboard host).
fn naming_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    let mut kb = std::mem::take(&mut ed(a).lm.kb);
    let r = app_keyboard_pointer(a, &mut kb, ph, ev);
    let text = kb.text;
    ed(a).lm.kb = kb;
    match r {
        KbResult::Done => newpage_done(a, text.as_str().trim()),
        KbResult::Cancel => naming_back(a),
        _ => {}
    }
}

/// A press on the strip (6.3): Cancel inverts at the down and cancels at the up if it was a tap; the rest of
/// the strip does nothing (a pen there never starts the lasso).
fn strip_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    let s = ed(a);
    let cancel = strip_cancel(s.lm.strip);
    if ph == 0 {
        press_begin(&mut s.lm.press, ev.x, ev.y, ev.t_ms);
        if rect_contains(cancel, ev.x as i32, ev.y as i32) {
            s.lm.cancel_down = true;
            app_press_feedback(a, cancel);
        }
        return;
    }
    press_move(&mut s.lm.press, ev.x, ev.y);
    if ph != 2 {
        return;
    }
    s.owner = Owner::None;
    let was_down = s.lm.cancel_down;
    s.lm.cancel_down = false;
    if !was_down {
        return;
    }
    if !touch_cancelled(ev) && press_is_tap(&s.lm.press, cancel) {
        link_cancel(a);
    } else {
        compose(a, cancel);
        app_invalidate(a, cancel, RefreshMode::Fast);
    }
}

/// Link making ends having drawn `u` (lm_bounds before it ended): the Pages view, if it was up for it,
/// closes on the open page (its Full); else u is drawn again, a GL16.
fn lm_close(a: &mut App, u: Rect) {
    if ed_ref(a).pv.open {
        pages_hide(a, -1);
    } else {
        compose(a, u);
        app_invalidate(a, u, RefreshMode::Ui);
    }
}

/// A new link's area goes to target ("index" or "p<page id>"): the link is made with a seed of its own,
/// saved, and is one undo step (6.7); Link to... (or the Pages view) closes on the open page, and the bar
/// comes back when its New link began it.
fn link_pick(a: &mut App, target: &str) {
    link_pick_as(a, target, -1);
}

/// link_pick for a page New page... just added, `new_page` its number (its own toast), -1 otherwise.
fn link_pick_as(a: &mut App, target: &str, new_page: i32) {
    if ed_ref(a).lm.state == LmState::Pick && !ed_ref(a).lm.retarget.is_empty() {
        link_retarget(a, target, new_page);
        return;
    }
    let u = lm_bounds(a);
    let s = ed(a);
    let from_bar = s.lm.from_bar;
    if s.lm.state != LmState::Pick || s.page_ix().is_none() {
        lm_drop(s);
        lm_close(a, u);
        return;
    }
    let pid = s.open_pid();
    let area = s.lm.area;
    let outline=s.lm.outline;
    lm_drop(s);
    let page = s.open_page().unwrap();
    // unique among the page's ink and link seeds: a link never takes in ink, and new ink never joins it
    let want = now_ms().wrapping_mul(2654435761) ^ page.strokes.len() as u32;
    let seed = nav_new_seed_with(&s.nav, Some(&s.dl), page, pid, want);
    s.boxes.stale = true;
    let ok = nav_link_set(&mut s.nav, pid, seed, target) == 0
        && nav_link_set_area(&mut s.nav, pid, seed, Some(area)) == 0
        && nav_link_set_outline(&mut s.nav,pid,seed,outline)==0
        && nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) == 0;
    if !ok {
        nav_load(s.nb.id.as_str(), &mut s.nav); // (back to the links as saved)
        migration_reset(s);
        lm_close(a, u);
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines).
        app_toast(a, tr("Couldn't save link."), 3000);
        if from_bar {
            bar_show(a, false);
        }
        return;
    }
    recent_push(s, target);
    let made = nav_link_get(&s.nav, pid, seed);
    let mut records = Vec::new();
    // (no memory for the step's one record: the link stays, saved, but Undo does not take it away)
    if let (Some(l), Ok(())) = (made, records.try_reserve_exact(1)) {
        records.push((None, Some(link_rec(&l))));
    }
    if let Some(h) = s.cur_hist() {
        history_push_link(&mut s.hist[h].h, LinkEdit { pid, records });
    }
    s.links_dirty = false; // (the save wrote every migrated area too)
    s.changed = true;
    let to = made.map_or(-1, |l| nav_link_page(&l, &s.nb));
    let msg = nav_target_label(target).map_or_else(||linked_msg(s,to,new_page),|name|cformat!(tr("Linked to %s."),&name));
    lm_close(a, u); // (the rectangle goes; with the eye on, compose draws the new box)
    repaint_panel(a, RefreshMode::Ui); // (Undo is enabled now)
    if from_bar {
        bar_show(a, false); // (with the new entry, in the same pass)
    }
    // The first link ever made says, once, how a link is used (hints bit 1), instead of where it goes.
    if a.settings.hints & HINT_FIRST_LINK == 0 {
        a.settings.hints |= HINT_FIRST_LINK;
        settings_save(&a.settings);
        // TRANSLATORS: a toast, once, after the first link is made (about 1700 px at 44 px in bold, or two
        // lines): a finger's tap on the link goes where it goes; a finger held on it opens its card (rename,
        // change, redraw or delete it).
        app_toast(a, tr("Linked. Finger tap to go there; hold for more."), 4000,
        );
        return;
    }
    app_toast(a, &msg, 2500);
}

/// A target picked for a link goes first on the Recent targets of the next Link to... (pages only).
fn recent_push(s: &mut EditorState, target: &str) {
    if !target.starts_with('p') {
        return;
    }
    let t = FixedStr::<NAV_TARGET_LEN>::from_str(target);
    s.recent.retain(|r| r.as_bytes() != t.as_bytes());
    if s.recent.len() >= RECENT_MAX {
        s.recent.remove(0);
    }
    if s.recent.try_reserve(1).is_ok() {
        s.recent.push(t);
    }
}

/// What a pick says (6.7 step 5): where the link goes now (`to` a page index, or the Index), or the page New
/// page... added (`new_page` its number, else -1).
fn linked_msg(s: &EditorState, to: i32, new_page: i32) -> String {
    let mut msg = String::new();
    if new_page > 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): New page... added page %d at the
        // end of the notebook and the new link goes to it.
        csnprintf!(&mut msg, 200, tr("Linked to new page %d."), new_page);
    } else if to >= 0 {
        let name = nav_name(&s.nav, s.nb.page_ids[to as usize]);
        if !name.is_empty() {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a new link was made to
            // the page with this name in the Index (%s).
            csnprintf!(&mut msg, 200, tr("Linked to %s."), name);
        } else {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a new link was made to
            // page %d, which has no name in the Index.
            csnprintf!(&mut msg, 200, tr("Linked to page %d."), to + 1);
        }
    } else {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a new link was made to the
        // Index (the list of named pages).
        csnprintf!(&mut msg, 200, "%s", tr("Linked to the Index."));
    }
    msg
}

/// Change where it goes (8.7): Link to... picked `target` for the card's links (new_page as link_pick_as has
/// it). Every link gets it, saved, in one undo step; the border and Link to... go in one GL16. The target
/// they have already changes nothing (no step, no toast). From the bar, the bar comes back.
fn link_retarget(a: &mut App, target: &str, new_page: i32) {
    let u = lm_bounds(a);
    let s = ed(a);
    let from_bar = s.lm.from_bar;
    let seeds = std::mem::take(&mut s.lm.retarget);
    let pid = s.card.pid;
    let same = s.card.target.as_str() == target;
    lm_drop(s);
    s.card.border = false;
    s.card.boxes.clear();
    if same || pid == 0 {
        lm_close(a, u);
        if from_bar {
            bar_show(a, false);
        }
        return;
    }
    s.boxes.stale = true;
    let mut records = Vec::new();
    let room = records.try_reserve_exact(seeds.len()).is_ok();
    let mut ok = true;
    for &seed in &seeds {
        let Some(old) = nav_link_get(&s.nav, pid, seed) else { continue ;
        };
        ok &= nav_link_set(&mut s.nav, pid, seed, target) == 0; // (its area stays)
        if let (Some(new), true) = (nav_link_get(&s.nav, pid, seed), room) {
            records.push((Some(link_rec(&old)), Some(link_rec(&new))));
        }
    }
    if !ok || nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
        nav_load(s.nb.id.as_str(), &mut s.nav);
        migration_reset(s);
        lm_close(a, u);
        app_toast(a, tr("Couldn't save link."), 3000);
        if from_bar {
            bar_show(a, false);
        }
        return;
    }
    recent_push(s, target);
    if let Some(h) = s.cur_hist() {
        history_push_link(&mut s.hist[h].h, LinkEdit { pid, records });
    }
    s.links_dirty = false;
    s.changed = true;
    let to = nav_link_page(&NavLink { target: FixedStr::from_str(target), ..Default::default() }, &s.nb,
    );
    let msg = nav_target_label(target).map_or_else(||linked_msg(s,to,new_page),|name|cformat!(tr("Linked to %s."),&name));
    lm_close(a, u);
    repaint_panel(a, RefreshMode::Ui);
    if from_bar {
        bar_show(a, false);
    }
    app_toast(a, &msg, 2500);
}

/// Redraw the area (8.8): the lasso lifted with r (page px), not too small: the link `seed` has that area now,
/// saved, one undo step that brings the old one back; the border, the lasso and the strip go in one GL16.
fn redraw_done(a: &mut App, seed: u32, r: Rect, drawn: Rect) {
    let mut u = rect_union(drawn, lm_bounds(a));
    let s = ed(a);
    let pid = s.open_pid();
    let from_bar = s.lm.from_bar;
    let old = nav_link_get(&s.nav, pid, seed);
    let outline=s.lm.outline;
    lm_drop(s); // (the border goes with it)
    s.boxes.stale = true;
    let ok = old.is_some()
        && nav_link_set_area(&mut s.nav, pid, seed, Some(r)) == 0
        && nav_link_set_outline(&mut s.nav,pid,seed,outline)==0
        && nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) == 0;
    if !ok {
        nav_load(s.nb.id.as_str(), &mut s.nav);
        migration_reset(s);
        compose(a, u);
        app_invalidate(a, u, RefreshMode::Ui);
        app_toast(a, tr("Couldn't save link."), 3000);
        if from_bar {
            bar_show(a, false);
        }
        return;
    }
    let mut records = Vec::new();
    if let (Some(o), Some(n), Ok(())) = (old, nav_link_get(&s.nav, pid, seed), records.try_reserve_exact(1),
    ) {
        records.push((Some(link_rec(&o)), Some(link_rec(&n))));
    }
    if let Some(h) = s.cur_hist() {
        history_push_link(&mut s.hist[h].h, LinkEdit { pid, records });
    }
    s.links_dirty = false;
    s.changed = true;
    if a.settings.link_areas {
        u = rect_union(u, eye_bounds(a)); // (the eye's box moves with the area)
    }
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
    repaint_panel(a, RefreshMode::Ui);
    if from_bar {
        bar_show(a, false);
    }
    app_toast_dismiss(a); // (no confirmation: the owner, 2026-11-04; a toast left up from before goes)
}

/// No link is made (6.10): from the strip's or Link to...'s Cancel, a tap outside Link to..., the add-link
/// action again, Close menu, and Back while Link to... is up. The strip, the lasso, the rectangle and Link
/// to... go, with no toast (the owner, 2026-11-04), and a toast left up from the making goes too; the bar
/// comes back when its New link began it.
fn link_cancel(a: &mut App) {
    let s = ed(a);
    if s.lm.state == LmState::Off && !(s.pv.open && s.pv.picking) {
        return;
    }
    let u = lm_bounds(a);
    let s = ed(a);
    let from_bar = s.lm.from_bar;
    lm_drop(s);
    lm_close(a, u);
    if from_bar {
        bar_show(a, false);
    }
    app_toast_dismiss(a);
}

/// A menu came up while a link was being made (any menu's button, on the panel or as an action): the link is
/// not made, silently, and the strip or Link to... goes in the menu's refresh. Link to... and the Pages view
/// picking for the area are the making's own, and stay.
fn lm_check_menus(a: &mut App) {
    let Some(s) = a.ed.as_deref() else { return };
    let other = match s.lm.state {
        LmState::Off => false,
        LmState::Armed | LmState::Lasso => s.owner != Owner::Lasso && sheet_open(s),
        LmState::Pick | LmState::Naming => {
            s.tools_open
                || s.mb.open
                || s.dm.open
                || s.paper_open
                || s.ts.open
                || s.lb.open
                || (s.find.open && s.find.mode != FindMode::Pick)
                || (s.pv.open && !s.pv.picking)
        }
    };
    if other {
        link_off(a);
    }
    lm_say(a);
}

/// grimoire.log follows link making: each change of its state (after the event, shortcut or tick that made
/// it) with who has the contact then, so a log tells what was up when the pen or a finger stopped acting.
fn lm_say(a: &mut App) {
    let Some(s) = a.ed.as_deref_mut() else { return };
    if s.lm.state == s.lm_said {
        return;
    }
    // (armed: the strip is up; lasso: the pen draws it; pick: Link to... or what it opened; naming: New page...)
    let name = |st: LmState| match st {
        LmState::Off => "off",
        LmState::Armed => "armed",
        LmState::Lasso => "lasso",
        LmState::Pick => "pick",
        LmState::Naming => "naming",
    };
    let (from, to) = (name(s.lm_said), name(s.lm.state));
    s.lm_said = s.lm.state;
    if s.diag.admit(now_ms()) {
        if s.owner == Owner::None {
            qlog!("link: %s -> %s, no contact owned", from, to);
        } else {
            qlog!("link: %s -> %s, the %s owns %s", from, to, if s.owner_is_pen { "pen" } else { "finger" }, owner_name(s.owner));
        }
    }
}

/// Undo/redo link records and save links.ini. Clear also changes ink in the same step; a failed
/// save reverses that tentative ink change and keeps Clear available to retry. Other failed link
/// steps reload the saved links and are dropped.
fn undo_redo_link(a: &mut App, ih: usize, ip: usize, undo: bool) {
    let before = if a.settings.link_areas { eye_bounds(a) } else { RECT_EMPTY };
    let s = ed(a);
    let kind=if undo {history_undo_kind(&s.hist[ih].h)}else{history_redo_kind(&s.hist[ih].h)};
    let clearing=kind==Some(ActKind::Clear);
    // A lasso step (select.rs) with links: its ink goes back by its patch, as undo_redo's does.
    let lasso=kind==Some(ActKind::Lasso);
    let mut d = RECT_EMPTY;
    let (hh, page) = hist_page(&mut s.hist, ih, ip);
    if !(if undo { history_undo(hh, page, &mut d) } else { history_redo(hh, page, &mut d) }) {
        return;
    }
    let mut ok = true;
    if let Some(e) = history_moved_link(&s.hist[ih].h, undo) {
        let pid = e.pid;
        for (b, af) in &e.records {
            let want = if undo { b } else { af };
            match want {
                Some(r) => ok &= nav_link_put(&mut s.nav, &rec_link(pid, r)) == 0,
                None => {
                    if let Some(r) = b.as_ref().or(af.as_ref()) {
                        nav_link_remove(&mut s.nav, pid, r.seed);
                    }
                }
            }
        }
    }
    s.boxes.stale = true;
    ok = ok && nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) == 0;
    if !ok {
        if clearing || lasso {
            // A failed sidecar save must also reverse the tentative ink change, preserving
            // the original Clear action so a later retry remains possible.
            let (h,page)=hist_page(&mut s.hist,ih,ip);
            let mut dirty=RECT_EMPTY;
            if undo {history_redo(h,page,&mut dirty);}else{history_undo(h,page,&mut dirty);}
        }
        nav_load(s.nb.id.as_str(), &mut s.nav);
        migration_reset(s);
        let h = &mut s.hist[ih].h;
        if undo {
            if !clearing && !lasso {history_drop_last_redo(h);}
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Undo of a link change
            // could not save links.ini, so the change is still there.
            app_toast(a, tr("Couldn't save links: undo unsuccessful."), 3000,
            );
        } else {
            if !clearing && !lasso {history_drop_last_undo(h);}
            // TRANSLATORS: the same toast for Redo.
            app_toast(a, tr("Couldn't save links: redo unsuccessful."), 3000,
            );
        }
        repaint_panel(a, RefreshMode::Ui);
        return;
    }
    s.links_dirty = false;
    s.changed = true; // (the notebook changed: the PDF mirror runs at close)
    if lasso {
        if !base_valid(s) {
            base_reset(s);
        }
        let swapped = base_valid(s) && !s.picture_missing && history_swap_patch(&mut s.hist[ih].h, undo, &mut s.page_canvas, &mut s.base).is_some();
        if swapped {
            s.undo_patched += 1;
        } else {
            draw_page(a, d);
        }
        let u = if a.settings.link_areas { rect_union(page_to_screen(a, d), rect_union(before, eye_bounds(a))) } else { page_to_screen(a, d) };
        compose(a, u);
        app_invalidate(a, u, RefreshMode::Ui);
        mark_changed(ed(a));
        repaint_panel(a, RefreshMode::Ui);
        return;
    }
    if clearing {
        mark_changed(s);
        bar_rebuild(a);render_page_full(a);compose_all(a,RefreshMode::Full);repaint_panel(a,RefreshMode::Ui);
        return;
    }
    // (with the eye off a link change shows nowhere on the page: Undo and Redo say nothing, the owner,
    // 2026-11-04; a toast still up about the change, "Linked to ...", would now be wrong, so it goes)
    app_toast_dismiss(a);
    if a.settings.link_areas {
        let u = rect_union(before, eye_bounds(a));
        compose(a, u);
        app_invalidate(a, u, RefreshMode::Ui);
    }
    repaint_panel(a, RefreshMode::Ui);
}

fn pages_event(a: &mut App, ph: i32, ev: &InEvent) {
    let r = with_pv(a, |pv, a, nb, nav| pagesview_pointer(pv, a, nb, nav, ph, ev));
    pages_result(a, r);
}

/// A hardware keyboard's key while the editor has a modal that types (the Pages view's count pad, for a
/// batch of new pages): true when it took the key, which then goes to no shortcut.
pub fn editor_modal_key(a: &mut App, ev: &InEvent) -> bool {
    if a.scr != ScreenId::Editor {
        return false;
    }
    // The toolbar's count pad: its keys type, Enter is Done and Esc Cancel; no other key reaches the page.
    if ed_ref(a).open && ed_ref(a).add_kb.open {
        if let Some(key) = count_pad_key(ev.value) {
            let mut kb = std::mem::take(&mut ed(a).add_kb);
            let r = app_keyboard_press(a, &mut kb, key);
            ed(a).add_kb = kb;
            add_pad_result(a, r);
        }
        return true;
    }
    if !ed_ref(a).pv.open {
        return false;
    }
    match with_pv(a, |pv, a, nb, nav| pagesview_key(pv, a, nb, nav, ev.value)) {
        Some(r) => {
            pages_result(a, r);
            true
        }
        None => false,
    }
}

// What the Pages view asked for (PvResult), from a pointer event or a key.
fn pages_result(a: &mut App, r: PvResult) {
    match r {
        PvResult::Close => {
            let s = ed(a);
            if s.pv.picking && s.lm.state == LmState::Pick && s.lt.open {
                // Choose from the pages...'s Back: Link to... again (the page, the rectangle and it, composed whole)
                s.lt.hidden = false;
                pages_hide(a, -1);
            } else if s.pv.picking {
                link_cancel(a);
            } else {
                pages_hide(a, -1);
            }
        }
        PvResult::List => {pagesview_close(&mut ed(a).pv);map_show(a,false);with_map(a,|map,a,_,nav|mapview_set_list(map,a,nav));}
        PvResult::Graph => {
            pagesview_close(&mut ed(a).pv);
            map_show(a, false);
        }
        PvResult::Link => {
            let target=if ed_ref(a).pv.unified {ed_ref(a).pv.target}else{-1};
            pages_hide(a, target);
            if target<0 || ed_ref(a).index==target {link_arm(a, false, None);}
        }
        PvResult::Pick => {
            // (the view picks only for Choose from the pages..., a new link waiting in Link to...)
            let s = ed(a);
            let to = s.pv.target;
            let mut target = String::new();
            csnprintf!(&mut target, NAV_TARGET_LEN, "p%u", s.nb.page_ids[to as usize]);
            if s.lm.state == LmState::Pick {
                link_pick(a, &target);
            } else {
                pages_hide(a, -1);
            }
        }
        PvResult::Open => {
            let to = ed(a).pv.target;
            let s = ed(a);
            let (from, at) = (s.open_pid(), s.index);
            pages_hide(a, to);
            let s = ed(a);
            if s.index != at && from != 0 {
                s.back_push(from);
            }
        }
        PvResult::GoTo => {
            // Go to page...: the Find sheet over the view, which stays behind it: a row picked goes to that page,
            // Cancel leaves the view as it was.
            find_show(a);
        }
        PvResult::Delete => pages_delete(a),
        PvResult::Copy => {
            let to = ed(a).pv.target;
            pages_copy(a, to);
        }
        PvResult::Move => {
            let to = ed(a).pv.target;
            pages_move(a, to);
        }
        PvResult::CopyTo => pages_copy_to_pick(a),
        PvResult::UndoMove => pages_undo_move(a),
        PvResult::AddPages => {
            let (at, n) = (ed(a).pv.target, ed(a).pv.add_count);
            pages_add_batch(a, at, n);
        }
        PvResult::UndoAdd => pages_undo_add(a),
        PvResult::Redo => pages_redo(a),
        PvResult::Name => {
            // (the open page, or the held tile's after its card's Rename)
            let s = ed(a);
            let Some(&pid) = s.nb.page_ids.get(s.pv.name_page as usize) else { return };
            let mut old = String::new();
            i18n_copy(&mut old, nav_name(&s.nav, pid), NAV_NAME_LEN);
            let text = s.pv.kb.text;
            if nav_set_name(&mut s.nav, pid, text.as_str()) != 0 || nav_save(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
                nav_set_name(&mut s.nav, pid, &old);
                app_toast(a, tr("Couldn't save page name."), 3000);
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
    let Some(page) = s.open_page() else { return false ;
    };
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, x, y, &mut px, &mut py);
    nav_link_at(&s.nav, page, s.nb.page_ids[s.index as usize], px, py, NAV_LINK_MIN_MM * a.dpi / 25.4, out,
    )
}

// The document link under (x, y) on screen, on the open page (D9: its exact area first, then the finger's
// growth). Only asked where no link of the user's is: theirs come first.
fn doc_link_under(a: &App, x: f32, y: f32) -> Option<DocLink> {
    let s = ed_ref(a);
    let page = s.open_page()?;
    if s.dl.links.is_empty() || s.page_ix().is_none() {
        return None;
    }
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, x, y, &mut px, &mut py);
    doclinks_at(&s.dl, page, s.nb.page_ids[s.index as usize], px, py, NAV_LINK_MIN_MM * a.dpi / 25.4)
}

// A finger's tap at (x, y): a link there goes to its page, or opens the Index. The user's links first, then a
// document link (a planner's tab).
fn link_follow(a: &mut App, x: f32, y: f32) {
    let mut l = NavLink::default();
    if !link_under(a, x, y, &mut l) {
        if let Some(d) = doc_link_under(a, x, y) {
            follow_target(a, &format!("p{}", d.target));
        }
        return;
    }
    if ed_ref(a).nav.links_writable && nav_target_state(l.target.as_str()).is_some_and(|t| nav_target_broken(&t)) {
        // A link to a page or notebook that was deleted: its card (titled "... / Page gone" or "Notebook
        // gone"), to change where it goes or delete it, instead of a toast and nothing.
        link_off(a);
        card_show(a, &[l.seed]);
        return;
    }
    follow_target(a,l.target.as_str());
}

// ---- the long-press card (PHASE2 8) ----

/// The card is drawn now: up, and not under Rename's keyboard where they meet.
fn card_shows(s: &EditorState) -> bool {
    s.card.open && !(s.card.kb.open && rect_overlaps(s.card.rect, s.card.kb.area))
}

/// Where the heavy border goes around a box (page px) on the screen: 8 px outside its screen rect.
fn card_ring(s: &EditorState, b: Rect) -> Rect {
    rect_inflate(box_screen(s, b), ui_px(8.0))
}

/// Everything the card's borders cover on the screen now (the strips of each), empty for none.
fn card_band(a: &App) -> Rect {
    let s = ed_ref(a);
    let mut u = RECT_EMPTY;
    for &b in &s.card.boxes {
        u = rect_union(u, linkcard_border_reach(card_ring(s, b)));
    }
    rect_intersect(u, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    )
}

/// The heavy border around every box the card is for, while it is drawn (after the eye's boxes, under the
/// panel), where r meets it.
fn draw_card_border(a: &mut App, r: Rect) {
    let s = ed_ref(a);
    if !s.card.border || s.card.boxes.is_empty() || s.peek.active {
        return;
    }
    let rings: Vec<Rect> = s.card.boxes.iter().map(|&b| card_ring(s, b)).collect();
    let clip = a.screen.clip;
    let c = rect_intersect(r, lasso_clip(a));
    canvas_set_clip(&mut a.screen, c);
    for ring in rings {
        if rect_overlaps(c, linkcard_border_reach(ring)) {
            linkcard_border(&mut a.screen, ring);
        }
    }
    a.screen.clip = clip;
}

/// The open page's padded boxes of the links `seeds` (as the eye and nav_link_at have them), in their order.
fn card_boxes(a: &mut App, seeds: &[u32]) -> Vec<Rect> {
    boxes_get(a);
    let s = ed_ref(a);
    let mut v = Vec::new();
    if v.try_reserve_exact(seeds.len()).is_err() {
        return v; // (no border: the card still acts on the links)
    }
    for &seed in seeds {
        if let Some(k) = (0..s.boxes.n).find(|&k| s.boxes.link[k].seed == seed) {
            v.push(s.boxes.area[k]);
        }
    }
    v
}

/// The borders by DU (PLAN 9.5): each box's four strips refreshed on its own and flushed on its own, so no
/// refresh spans the gray ink inside the box (DU would posterize it); the strips hold only black and white.
fn card_border_fast(a: &mut App) {
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h ,
    };
    let rings: Vec<Rect> = {
        let s = ed_ref(a);
        s.card.boxes.iter().map(|&b| card_ring(s, b)).collect()
    };
    for ring in rings {
        for st in linkcard_strips(ring) {
            let st = rect_intersect(st, screen);
            if rect_is_empty(st) {
                continue;
            }
            compose(a, st);
            app_invalidate(a, st, RefreshMode::Fast);
            app_flush(a);
        }
    }
}

/// The card for the links `seeds` on the open page (one held on the page), its border shown first: the
/// strips by DU, then the card by GL16 beside it. The finger is still down: its lift does nothing more.
fn card_show(a: &mut App, seeds: &[u32]) {
    card_set(a, seeds, false);
    card_border_fast(a);
    let s = ed_ref(a);
    let (w, h) = linkcard_size(&s.card);
    let screen = Rect { x: 0, y: top_of_page(a), w: a.w, h: a.h - top_of_page(a) ,
    };
    let rect = popout_place(w, h, s.card.band, screen, &[s.panel, status_zone(a)]);
    ed(a).card.rect = rect;
    compose(a, rect);
    app_invalidate(a, rect, RefreshMode::Ui);
}

/// The card set up for the links `seeds` on the open page, with their boxes and the band their borders cover
/// (nothing drawn yet).
fn card_set(a: &mut App, seeds: &[u32], from_bar: bool) {
    let s = ed(a);
    let pid = s.open_pid();
    let readonly = s.open_page().is_some_and(|p| p.readonly);
    let boxes = card_boxes(a, seeds);
    with_card(a, |c, _a, nb, nav| {
        linkcard_set(c, pid, seeds, nb, nav, readonly, from_bar)});
    let s = ed(a);
    s.card.boxes = boxes;
    s.card.border = true;
    let band = card_band(a);
    ed(a).card.band = band;
}

/// The card of the document-link chip i (a group of the page's document links to one target): read only, its
/// heavy border around their exact boxes, one row, Rename the page. False for a chip with no page here.
fn card_set_doc(a: &mut App, i: i32) -> bool {
    let s = ed_ref(a);
    let Some(e) = linkbar_entry(&s.lb, i).copied() else { return false };
    let Some(&target) = (e.page >= 0).then(|| s.nb.page_ids.get(e.page as usize)).flatten() else { return false };
    let pid = s.open_pid();
    let Some(page) = s.open_page() else { return false };
    let mut boxes: Vec<Rect> = doclinks_targets(&s.dl, page, pid).into_iter().filter(|&(t, _)| t == target).map(|(_, r)| r).collect();
    boxes.sort_by_key(|r| (r.y, r.x));
    with_card(a, |c, _a, nb, nav| linkcard_set_doc(c, pid, target, nb, nav, true));
    let s = ed(a);
    s.card.boxes = boxes;
    s.card.border = true;
    let band = card_band(a);
    ed(a).card.band = band;
    true
}

/// A press held on chip `chip` of the Links bar (entry i, 8.10, D1): the long-press card for all the entry's
/// links on the page, above the bar in the chip's column, the bar staying under it. While the bar looks at
/// another page it looks at its own again first (the links are there), and the borders come with the page area's
/// one GL16 (no DU over a peek's ghosting); else they come by DU strips, or for more than three boxes by one GL16
/// of their bounds (4N DU presents would queue in the driver).
fn bar_hold(a: &mut App, i: i32, chip: Rect) {
    let d = with_lb(a, |lb, a, _nb, _nav| linkbar_hold(lb, a, i));
    let s = ed(a);
    let peeking = s.peek.active;
    if peeking {
        s.peek = Peek { count: s.peek.count, ..Peek::default() };
        s.lb.selected = -1;
    }
    // The entry's links, in reading order: those whose box carries its number. Inside the document links, the
    // chip's document links (read only: Rename the page).
    boxes_get(a);
    let is_doc = ed_ref(a).lb.in_docs;
    let s = ed_ref(a);
    let mut ks: Vec<usize> = (0..s.boxes.n).filter(|&k| s.boxes.entry[k] as i32 == i + 1).collect();
    ks.sort_by_key(|&k| (s.boxes.area[k].y, s.boxes.area[k].x, k));
    let seeds: Vec<u32> = if is_doc { Vec::new() } else { ks.iter().map(|&k| s.boxes.link[k].seed).collect() };
    let shown = if is_doc { card_set_doc(a, i) } else { !seeds.is_empty() };
    if !shown {
        if peeking {
            bar_back_drawn(a);
        }
        app_invalidate(a, d, RefreshMode::Ui);
        return;
    }
    if !is_doc {
        card_set(a, &seeds, true);
    }
    let s = ed_ref(a);
    let (aw, bar) = (a.w, s.lb.rect);
    if peeking {
        bar_back_drawn(a);
        app_flush(a);
    } else if s.card.boxes.len() <= 3 {
        card_border_fast(a);
    } else {
        let band = s.card.band;
        compose(a, band);
        app_invalidate(a, band, RefreshMode::Ui);
        app_flush(a);
    }
    let s = ed_ref(a);
    let (w, h) = linkcard_size(&s.card);
    let rect = linkcard_place_above(w, h, chip, bar, Rect { x: 0, y: 0, w: aw, h: a.h ,
        },
    );
    ed(a).card.rect = rect;
    compose(a, rect);
    let u = rect_union(rect, d);
    app_invalidate(a, u, RefreshMode::Ui);
}

/// The page area above the bar and the bar composed again after a peek ended (bar_back's drawing), one GL16.
fn bar_back_drawn(a: &mut App) {
    let s = ed_ref(a);
    let (area, bar) = (Rect { x: 0, y: 0, w: a.w, h: s.lb.rect.y ,
        }, s.lb.rect,
    );
    compose(a, area);
    compose(a, bar);
    app_invalidate(a, rect_union(area, bar), RefreshMode::Ui);
}

/// The screen's top line for a card: below the status bar while it is up.
fn top_of_page(a: &App) -> i32 {
    status_reserved(a)
}

/// A finger held still on a link (HOLD_MS, editor_tick): its border and the long-press card, over the
/// page (R2: no Pages view, no full refresh). A link being made is not made, silently.
fn card_open_link(a: &mut App) {
    ed(a).link_held = true;
    let mut l = NavLink::default();
    let (x0, y0) = (ed(a).link_press.x0, ed(a).link_press.y0);
    if !link_under(a, x0, y0, &mut l) {
        return;
    }
    if !ed(a).nav.links_writable {
        return;
    }
    link_off(a);
    card_show(a, &[l.seed]);
}

/// The card is gone without a word (a page change, a menu or screen drawn whole next, a turn, sleep, the
/// notebook closing): its border with it, Rename's keyboard closed, nothing saved.
fn card_drop(s: &mut EditorState) {
    if s.owner == Owner::Card {
        s.owner = Owner::None;
    }
    let kb = std::mem::take(&mut s.card.kb);
    s.card = LinkCard { kb, ..Default::default() };
    keyboard_close(&mut s.card.kb);
}

/// The card closes (8.12): it and its border go in one GL16 of the card and the strips together (the gray ink
/// under the strips comes back exactly). The bar under it, if it is up, stays.
fn card_close(a: &mut App) {
    let s = ed_ref(a);
    if !s.card.open && !s.card.border {
        return;
    }
    let mut u = rect_union(if s.card.open { s.card.rect } else { RECT_EMPTY }, card_band(a),
    );
    if s.card.kb.open {
        u = rect_union(u, s.card.kb.area);
    }
    card_drop(ed(a));
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

/// What a tap on the card asked.
fn card_result(a: &mut App, r: LcResult, d: Rect) {
    if !rect_is_empty(d) {
        app_invalidate(a, d, RefreshMode::Ui);
    }
    match r {
        LcResult::Row(CardRow::Rename) => card_rename(a),
        LcResult::Row(CardRow::Retarget) => card_retarget(a),
        LcResult::Row(CardRow::Redraw) => card_redraw(a),
        LcResult::Row(CardRow::Delete) => card_delete(a),
        _ => {}
    }
}

/// Where the card's border is, its centre's y on the screen (what docks a keyboard or Link to... away from it).
fn card_centre_y(a: &App) -> i32 {
    let b = ed_ref(a).card.band;
    b.y + b.h / 2
}

/// The page id a card's target names ("p<page id>"), 0 for none.
fn card_target_pid(s: &EditorState) -> u32 {
    if s.card.page < 0 || s.card.page as usize >= s.nb.page_ids.len() {
        return 0;
    }
    s.nb.page_ids[s.card.page as usize]
}

/// Rename (8.6, D3): a keyboard names the target page in the Index, docked away from the border (at the top
/// when the border is in the lower 45% of the screen); the card stays drawn where the keyboard leaves it.
fn card_rename(a: &mut App) {
    let pid = card_target_pid(ed_ref(a));
    if pid == 0 || !ed_ref(a).nav.writable {
        return;
    }
    let top = linkbar_pick_docks_top(card_centre_y(a), a.h);
    let dock = if top { dock_top_y(a) } else { 0 };
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h ,
    };
    let s = ed(a);
    let mut name = String::new();
    i18n_copy(&mut name, nav_name(&s.nav, pid), NAV_NAME_LEN);
    // TRANSLATORS: the title of the keyboard that names the page a held link goes to (about 1500 px at 44 px).
    keyboard_open(&mut s.card.kb, tr("Page name"), name.as_bytes());
    keyboard_set_dock(&mut s.card.kb, dock);
    s.card.kb.area = keyboard_modal_rect_docked(screen, dock);
    let u = rect_union(s.card.kb.area, s.card.rect);
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

/// Rename's keyboard closes (Done or Cancel, or a tap outside it): the card comes back with its title and
/// rows as they are now, in one GL16 of the keyboard and the card.
fn rename_back(a: &mut App) {
    rename_end(a, false);
}

/// rename_back, also when the keyboard closed itself (its Done: `done`).
fn rename_end(a: &mut App, done: bool) {
    let s = ed(a);
    if !s.card.open || (!s.card.kb.open && !done) {
        return;
    }
    let u = rect_union(s.card.kb.area, s.card.rect);
    keyboard_close(&mut s.card.kb);
    let (pid, readonly, from_bar) = (s.card.pid, s.card.readonly, s.card.from_bar);
    let (doc, doc_target) = (s.card.doc, s.card.doc_target);
    let seeds = s.card.seeds.clone();
    let (rect, boxes, band, border) = (s.card.rect, std::mem::take(&mut s.card.boxes), s.card.band, s.card.border,
    );
    with_card(a, |c, _a, nb, nav| {
        if doc {
            linkcard_set_doc(c, pid, doc_target, nb, nav, from_bar)
        } else {
            linkcard_set(c, pid, &seeds, nb, nav, readonly, from_bar)
        }});
    let c = &mut ed(a).card;
    c.rect = rect;
    c.boxes = boxes;
    c.band = band;
    c.border = border;
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
    bar_rebuild(a);
}

/// Rename's Done: the target page's Index name is `name` (empty takes it out of the Index), saved; not an undo
/// step (a notebook-wide name). Only a failure says so (the owner, 2026-11-04).
fn rename_done(a: &mut App, name: &str) {
    let s = ed(a);
    let pid = card_target_pid(s);
    let mut old = String::new();
    i18n_copy(&mut old, nav_name(&s.nav, pid), NAV_NAME_LEN);
    let failed = pid == 0 || nav_set_name(&mut s.nav, pid, name) != 0 || nav_save(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0;
    if failed && pid != 0 {
        nav_set_name(&mut s.nav, pid, &old);
    }
    if !failed {
        s.changed = true;
    }
    rename_end(a, true);
    if failed {
        app_toast(a, tr("Couldn't save page name."), 3000);
    } else {
        app_toast_dismiss(a); // (a toast left up from before, a failed try's, goes)
    }
}

/// A finger or the pen on Rename's keyboard (typing through the keyboard host).
fn card_kb_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    let mut kb = std::mem::take(&mut ed(a).card.kb);
    let r = app_keyboard_pointer(a, &mut kb, ph, ev);
    let text = kb.text;
    ed(a).card.kb = kb;
    match r {
        KbResult::Done => rename_done(a, text.as_str().trim()),
        KbResult::Cancel => rename_end(a, true),
        _ => {}
    }
}

/// Change where it goes (8.7): the card closes (its own GL16, flushed) and the border stays; Link to... picks
/// the new target for every link the card is for, the current one marked, without Choose from the pages...
/// (R2: the page stays in view). From the bar, the bar closes too and comes back after the pick or the cancel.
fn card_retarget(a: &mut App) {
    let s = ed(a);
    let r = s.card.rect;
    s.card.open = false;
    let from_bar = s.card.from_bar;
    let seeds = s.card.seeds.clone();
    let mut u = r;
    if from_bar && s.lb.open {
        u = rect_union(u, s.lb.rect);
        bar_drop(s);
    }
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
    app_flush(a);
    let s = ed(a);
    s.lm = LinkMaking { state: LmState::Pick, from_bar, retarget: seeds, ..Default::default() };
    let current = s.card.target.as_str().to_string();
    let cy = card_centre_y(a);
    lt_open_for(a, cy, &current, true);
}

/// Redraw the area (8.8): the card closes and the border stays on the old area; the strip asks for the new
/// one, and the next pen contact on the page draws it. From the bar, the bar folds into the strip.
fn card_redraw(a: &mut App) {
    let s = ed(a);
    let Some(&seed) = s.card.seeds.first() else { return ;
    };
    let r = s.card.rect;
    s.card.open = false;
    let from_bar = s.card.from_bar;
    let mut u = r;
    if from_bar && s.lb.open {
        u = rect_union(u, s.lb.rect);
        bar_drop(s);
    }
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
    link_arm(a, from_bar, Some(seed));
    if ed_ref(a).lm.state == LmState::Off {
        card_close(a); // (refused: the border goes, and the bar comes back)
        if from_bar {
            bar_show(a, false);
        }
    }
}

/// Delete link (8.9): the card's links go, one undo step that brings them all back; the ink drawn around them
/// (a visible border the owner drew) stays.
fn card_delete(a: &mut App) {
    let s = ed(a);
    let pid = s.card.pid;
    let seeds = s.card.seeds.clone();
    let mut records = Vec::new();
    let room = records.try_reserve_exact(seeds.len()).is_ok();
    for &seed in &seeds {
        if let Some(old) = nav_link_get(&s.nav, pid, seed) {
            if room {
                records.push((Some(link_rec(&old)), None));
            }
            nav_link_remove(&mut s.nav, pid, seed);
        }
    }
    s.boxes.stale = true;
    if nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
        nav_load(s.nb.id.as_str(), &mut s.nav);
        migration_reset(s);
        card_close(a);
        app_toast(a, tr("Couldn't save link."), 3000);
        return;
    }
    if let Some(h) = s.cur_hist() {
        history_push_link(&mut s.hist[h].h, LinkEdit { pid, records });
    }
    s.links_dirty = false; // (the save wrote every migrated area too)
    s.changed = true;
    s.panel_stale = true;
    card_close(a);
    repaint_panel(a, RefreshMode::Ui); // (Undo, which brings them back, is enabled now; no toast)
    bar_rebuild(a);
    app_toast_dismiss(a); // (a "Linked to ..." still up would be wrong now)
}

/// The bar under a card, built again after the card changed its links or their names (the same origin and
/// header), and drawn again where it was and where it is now. Nothing when the bar is not up.
fn bar_rebuild(a: &mut App) {
    let s = ed_ref(a);
    if !s.lb.open || s.peek.active {
        return;
    }
    let (dpi, aw, ah) = (a.dpi, a.w, a.h);
    boxes_get(a);
    let s = ed(a);
    let Some(page) = s.open_page() else { return };
    let old = s.lb.rect;
    let (hint, label, back_to, back_graph) = (s.lb.header_hint, s.lb.back_label, s.lb.back_to, s.lb.back_graph);
    let mut lb = LinkBar::default();
    linkbar_build(&mut lb, &s.nb, &s.nav, page, s.index, dpi);
    let docs = doclinks_targets(&s.dl, page, s.open_pid());
    linkbar_set_docs(&mut lb, &s.nb, &docs, dpi, s.lb.in_docs);
    lb.header_hint = hint;
    lb.back_label = label;
    lb.back_to = back_to;
    lb.back_graph = back_graph;
    lb.screen = s.lb.screen; // (the pager stays where it was: a row the owner was at stays in view)
    let from = bar_from_rows(s);
    linkbar_set_from(&mut lb, from, backlinks_generation());
    linkbar_layout(&mut lb, aw, ah);
    let u = rect_union(old, lb.rect);
    s.lb = lb;
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

fn move_panel_to(a: &mut App, x: i32, y: i32, m: RefreshMode) {
    let old = ed(a).panel;
    ed(a).panel = panel_rect_at(a, x, y);
    compose(a, old);
    draw_panel_over(a);
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
    draw_panel_over(a);
    let p = ed(a).panel;
    app_invalidate(a, rect_union(old, p), RefreshMode::Ui);
    persist_panel(a);
}

// ---- the tool picker's taps ----

// The tool picker closed (a pen picked, or a tap outside it), and its Pin mode with it.
fn close_tools(a: &mut App) {
    let s = ed(a);
    s.tools_open = false;
    s.shapes_menu = false;
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
    let old = rect_union(s.panel, if s.tools_open { s.tools_rect } else { RECT_EMPTY },
    );
    fit_panel_to(&a.settings, a.h);
    let p = ed(a).panel;
    ed(a).panel = panel_rect_at(a, p.x, p.y);
    compose(a, old); // (draws the panel and the picker whole, where they are now)
    let s = ed_ref(a);
    let now = rect_union(s.panel, if s.tools_open { s.tools_rect } else { RECT_EMPTY },
    );
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
        // toolbar holds: %d is how many it holds, 3 in this version. About 1700 px at 44 px in bold,
        // or two lines.
        csnprintf!(&mut msg, 200, ng_("Toolbar full (%d pen). Unpin one first.",
                                      "Toolbar full (%d pens). Unpin one first.", PANEL_PINS_MAX as i64),
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
        P_PICK_SELECT => {
            close_tools(a);
            if ed_ref(a).sel.state == SelState::Off {
                sel_toggle(a);
            }
        }
        P_PIN_MODE => {
            let s = ed(a);
            s.pin_mode = !s.pin_mode;
            draw_tools(a);
            let r = ed(a).tools_rect;
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_SHAPES | P_SHAPE_HOLD => {
            if h.id == P_SHAPES { editor_shapes_toggle(a); } else { editor_shape_hold_cycle(a); }
            draw_tools(a);
            let r = ed(a).tools_rect;
            app_invalidate(a,r,RefreshMode::Ui);
        }
        P_SHAPE_QUADS | P_SHAPE_ROUND | P_SHAPE_LINKS => {
            match h.id {
                P_SHAPE_QUADS => a.settings.shape_quads = (a.settings.shape_quads+1)%3,
                P_SHAPE_ROUND => a.settings.shape_round = (a.settings.shape_round+1)%3,
                _ => a.settings.shape_links = !a.settings.shape_links,
            }
            settings_save(&a.settings);draw_tools(a);
            let r=ed(a).tools_rect;app_invalidate(a,r,RefreshMode::Ui);
        }
        P_SHAPE_DONE => close_tools(a),
        P_PICK_INK => {
            let st = &mut a.settings;
            st.white = K_INKS[clampi(h.arg, 0, N_INKS - 1) as usize].white;
            settings_save(st);
            // The picker shows the ink now picked, and the panel's pen whether it draws white (the picker
            // beside the panel is outside the panel's rect: both are composed again, 2.0.0-rc.12).
            let s = ed(a);
            let r = rect_union(s.panel, s.tools_rect);
            if !s.pv.open {
                compose(a, r);
            }
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_PICK_TOOL => {
            let t = ToolKind(h.arg);
            if ed(a).pin_mode {
                toggle_pin(a, t); // (the panel and the picker drawn again, or a toast at the cap)
                return;
            }
            sel_off(a); // (choosing a pen leaves Select mode)
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
    s.shapes_menu = false;
    s.pin_mode = false;
    s.tools_y = s.panel.y;
    draw_tools(a);
    let r = ed(a).tools_rect;
    app_invalidate(a, r, RefreshMode::Ui);
}

fn open_shape_menu(a: &mut App) {
    let s=ed(a);
    s.tools_open=true;s.shapes_menu=true;s.pin_mode=false;s.tools_y=s.panel.y;
    draw_tools(a);
    let r=ed(a).tools_rect;app_invalidate(a,r,RefreshMode::Ui);
}

// ---- the Find sheet ----

// Go to page or name: the sheet opens over the page with an empty field (the sheet's own refresh: a Ui of the
// keyboard, there are no rows yet). The page it is asked on stays the open one until a row is gone to.
fn find_open(a: &mut App) {
    link_off(a);
    let mut f = std::mem::take(&mut ed(a).find);
    findsheet_open(&mut f, a);
    ed(a).find = f;
}

// Open and drawn at once: a Ui of what it covers.
fn find_show(a: &mut App) {
    find_open(a);
    let r = with_find(a, |f, a, nb, nav| {
        findsheet_draw(f, a, nb, nav, Rect { x: 0, y: 0, w: a.w, h: a.h ,
            },
        )});
    app_invalidate(a, r, RefreshMode::Ui);
}

// Link to...'s Search (7.2): the Find sheet in pick mode over the page, its keyboard docked where Link to... is (at
// the top exactly when it is), Link to... hidden under it; one Ui of both.
fn find_pick_open(a: &mut App) {
    let dock = if ed_ref(a).lt.top { dock_top_y(a) } else { 0 };
    let s = ed(a);
    s.lt.hidden = true;
    let lt_rect = s.lt.rect;
    let mut f = std::mem::take(&mut s.find);
    findsheet_open_pick(&mut f, a, dock);
    ed(a).find = f;
    compose(a, lt_rect);
    let r = with_find(a, |f, a, nb, nav| {
        findsheet_draw(f, a, nb, nav, Rect { x: 0, y: 0, w: a.w, h: a.h ,
            },
        )});
    app_invalidate(a, rect_union(r, lt_rect), RefreshMode::Ui);
}

// The sheet is gone without a word (leaving the editor, a sleep, the notebook closing, a page drawn whole).
fn find_drop(s: &mut EditorState) {
    s.find.open = false;
    s.find.has_pressed = false;
    keyboard_close(&mut s.find.kb);
    s.find.n = 0;
    s.find.rect = RECT_EMPTY;
}

// Cancel: the page again where the card and the keyboard were, a Ui. In pick mode Link to... comes back with it.
fn find_close(a: &mut App) {
    let s = ed(a);
    if !s.find.open {
        return;
    }
    let mut r = rect_union(s.find.rect, s.find.kb.area);
    let pick = s.find.mode == FindMode::Pick;
    let over_pv = s.pv.open && !s.pv.overlay;
    find_drop(s);
    if pick && s.lt.open {
        s.lt.hidden = false;
        r = rect_union(r, s.lt.rect);
    }
    // Over the Pages view: previews that came while the sheet was up (previews_repaint draws none under it) are
    // drawn in their tiles, which are refreshed in the same Ui. Before the compose below: it draws the view whole,
    // and the tiles to refresh are the ones that show something else now than the panel does.
    if over_pv {
        r = rect_union(r, with_pv(a, pagesview_repaint_tiles));
    }
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Ui);
}

// A row, or Done on the first one: the sheet closes, the page it was asked on goes on the back list and the page
// asked for opens (go_to_page's own Full).
fn find_go(a: &mut App, index: i32) {
    find_drop(ed(a));
    if ed_ref(a).pv.open {
        // (opened over the Pages view: it closes on that page, as its Go to page did)
        // (a page that does not open: go_to_page drew the screen whole without the view, in its own Full)
        pagesview_close(&mut ed(a).pv);
    }
    go_to_page_pushing(a, index);
}

// What the Find sheet was asked, after the refreshes of what was typed: the text line (the keyboard host's),
// then the card of rows, then the key, each its own fast refresh.
fn find_result(a: &mut App, r: FsResult, d: Rect) {
    match r {
        FsResult::Go(i) if ed_ref(a).find.mode == FindMode::Pick => {
            // Link to...'s Search: the link goes there, with no page change and nothing on the back list.
            let s = ed(a);
            let Some(&pid) = s.nb.page_ids.get(i.max(0) as usize) else { return ;
            };
            let mut target = String::new();
            csnprintf!(&mut target, NAV_TARGET_LEN, "p%u", pid);
            link_pick(a, &target);
        }
        FsResult::Go(i) => find_go(a, i),
        FsResult::Close => find_close(a),
        FsResult::Changed => {
            let rd = ed_ref(a).find.rows_dirty;
            if !rect_is_empty(rd) {
                // The rows changed: the page where the old card was, the new card on it, a refresh of its own.
                compose(a, rd);
                app_invalidate(a, rd, RefreshMode::Fast);
                app_flush(a);
            }
            app_invalidate(a, d, RefreshMode::Fast);
        }
        _ => app_invalidate(a, d, RefreshMode::Fast),
    }
}

fn shape_hold_label(ms: i32) -> &'static str {
    match shape_hold_nearest(ms) { 400 => n_("Hold: Quick"), 900 => n_("Hold: Slow"), _ => n_("Hold: Normal") }
}
fn shape_hold_nearest(ms: i32) -> i32 {
    [400,550,900].into_iter().min_by_key(|&v| (v-ms).abs()).unwrap()
}
// (no toast for either: the owner, 2026-11-04; the Shapes menu shows both, the panel's Shapes button the first)
pub fn editor_shapes_toggle(a: &mut App) {
    a.settings.shapes = !a.settings.shapes; settings_save(&a.settings);
    repaint_panel(a,RefreshMode::Ui);
    app_toast_dismiss(a); // (a toast left up from before goes)
}
pub fn editor_shape_hold_cycle(a: &mut App) {
    a.settings.shape_hold_ms = match shape_hold_nearest(a.settings.shape_hold_ms) {400=>550,550=>900,_=>400};
    settings_save(&a.settings);
    app_toast_dismiss(a);
}
pub fn editor_shape_last(a: &mut App) {
    let min_mm = a.settings.shape_min_mm;
    let (dpi,logging) = (a.dpi,a.shape_log);
    let (policy,corner)=((a.settings.shape_quads,a.settings.shape_round),a.settings.shape_corner);
    let s = ed(a);
    if s.owner != Owner::None || sheet_open(s) || s.open_page().is_none_or(|p| p.readonly) {return;}
    let (Some(ip),Some(hi)) = (s.page_ix(),s.cur_hist()) else {return;};
    let p = s.hist[ip].page.as_deref().unwrap();
    let valid = p.strokes.last().is_some_and(|raw| {
        !stroke_is_erase(raw)
            && s.hist[hi].h.undo.last().is_some_and(|step| step.kind == ActKind::Add && step.ids.last() == Some(&stroke_id(raw)))
    });
    if !valid {
        app_toast(a, tr("No shape in the last stroke."), 3000);
        return;
    }
    let raw = p.strokes.last().unwrap();
    let mut prm = shape_params(dpi,s.zoom,p.paper,s.pgw);
    prm.quad_mode=policy.0;prm.circle_mode=policy.1;prm.corner=corner as f32/100.0;prm.min_mm=min_mm as f32;
    let started = now_ms();
    let result = shape_recognize(&raw.pts,&prm);
    shape_report(logging,"last",&result,false,prm.zoom,now_ms().wrapping_sub(started),0);
    let Some(mut shape) = shape_stroke(raw,&result,dpi) else {app_toast(a,tr("No shape in the last stroke."),3000); return;};
    shape.pts.iter_mut().for_each(sample_as_stored);
    let index = p.strokes.len() as i32 - 1;
    let (history,page) = hist_page(&mut s.hist,hi,ip);
    let dirty = history_push_replace(history,page,index,shape);
    if rect_is_empty(dirty) {return;}
    s.sh.last_kind = result.kind; s.sh.kept=s.sh.kept.saturating_add(1); mark_changed(s);
    draw_page(a,dirty); show(a,dirty,RefreshMode::Ui); repaint_panel(a,RefreshMode::Ui); live_reset();
}

// ---- the Links bar ----

// The bar opens on the open page: its entries, the second header line for the first three opens, the way
// back to the page it was left from. One Ui refresh of its card, no flash. Opening it disarms link making,
// silently (the strip goes in the same refresh).
fn bar_open(a: &mut App) {
    bar_show(a, true);
}

/// The Links bar's Linked from rows for the open page: its notebook's pages whose links go to it (from the
/// open Nav: always current), then other notebooks' pages (the backlinks cache, as far as it has checked),
/// each labelled with its notebook's name and the page's Index name or number (backlinks_book: each other
/// notebook's notebook.ini and nav.ini, kept between builds and read again only when they change; never its
/// links.ini).
fn bar_from_rows(s: &EditorState) -> Vec<LbFrom> {
    let pid = s.open_pid();
    if pid == 0 || !s.nav.links_writable {
        return Vec::new();
    }
    let mut books: Vec<(FixedStr<STORE_ID_LEN>, Option<std::rc::Rc<BlBook>>)> = Vec::new();
    let mut out = Vec::new();
    for f in backlinks_for_page(&s.nb, &s.nav, pid) {
        if out.len() >= LB_FROM_MAX || out.try_reserve(1).is_err() {
            break;
        }
        if f.id == s.nb.id {
            let page = s.nb.page_ids.iter().position(|&p| p == f.pid).map_or(-1, |i| i as i32);
            let name = nav_name(&s.nav, f.pid);
            let (label, named) = if name.is_empty() { (cformat!(tr("Page %d"), page + 1), false) } else { (name.to_string(), true) };
            out.push(LbFrom { target: FixedStr::from_str(&format!("p{}", f.pid)), page, count: f.count, label, named });
            continue;
        }
        if !books.iter().any(|(id, _)| *id == f.id) {
            if books.try_reserve(1).is_err() {
                break;
            }
            books.push((f.id, backlinks_book(f.id.as_str())));
        }
        // (a notebook that does not read now, or a page it no longer lists: the cache is behind; left out)
        let Some((nb, nav)) = books.iter().find(|(id, _)| *id == f.id).and_then(|(_, b)| b.as_deref()).map(|b| (&b.info, &b.names)) else { continue };
        let Some(i) = nb.page_ids.iter().position(|&p| p == f.pid) else { continue };
        let Some(target) = nav_notebook_target(f.id.as_str(), f.pid) else { continue };
        let name = nav_name(nav, f.pid);
        let page = if name.is_empty() { cformat!(tr("Page %d"), i as i32 + 1) } else { name.to_string() };
        out.push(LbFrom { target: FixedStr::from_str(&target), page: -1, count: f.count, label: format!("{} / {}", nb.name.as_str(), page), named: true });
    }
    out
}

fn bar_show(a: &mut App, count_open: bool) {
    link_off(a);
    let (dpi, aw, ah) = (a.dpi, a.w, a.h);
    let s = ed(a);
    s.peek = Peek::default();
    cache_free(s);
    s.boxes.stale = true;
    boxes_get(a);
    let shown = (a.settings.hints >> HINT_BAR_SHIFT) & HINT_BAR_MASK;
    let s = ed(a);
    let Some(page) = s.open_page() else { return };
    let mut lb = LinkBar::default();
    linkbar_build(&mut lb, &s.nb, &s.nav, page, s.index, dpi);
    let docs = doclinks_targets(&s.dl, page, s.open_pid());
    linkbar_set_docs(&mut lb, &s.nb, &docs, dpi, false);
    lb.header_hint = shown < HINT_BAR_MASK;
    lb.back_label = s.index + 1;
    if let Some((_, label)) = s.back_top() {
        lb.back_to.set_cut(&label);
    } else if let Some(b)=s.notebook_back.last() {
        if let Some(label)=nav_target_label(&nav_notebook_target(b.id.as_str(),b.pid).unwrap_or_default()) {
            lb.back_to.set_cut(&label);
        }
    } else if s.from_graph {
        lb.back_graph = true; // ("Back to Graph": the Library's, which opened this notebook)
    }
    let from = bar_from_rows(s);
    linkbar_set_from(&mut lb, from, backlinks_generation());
    linkbar_layout(&mut lb, aw, ah);
    let rect = lb.rect;
    s.lb = lb;
    if count_open && shown < HINT_BAR_MASK {
        a.settings.hints = (a.settings.hints & !(HINT_BAR_MASK << HINT_BAR_SHIFT)) | ((shown + 1) << HINT_BAR_SHIFT);
        settings_save(&a.settings);
    }
    compose(a, rect);
    app_invalidate(a, rect, RefreshMode::Ui);
}

// The bar is gone without a word: leaving the editor, a sleep, the notebook closing, or something that
// draws everything itself next. A page it was looking at is only a display state, so nothing is undone:
// page_canvas and the view were never touched.
fn bar_drop(s: &mut EditorState) {
    s.lb.open = false;
    s.lb.has_pressed = false;
    s.peek = Peek::default();
    cache_free(s);
}

// The canvases of the peeks are freed with the bar.
fn cache_free(s: &mut EditorState) {
    for e in s.ccache.iter_mut() {
        *e = CacheEntry::default();
    }
    s.cache_hits = 0;
    s.cache_misses = 0;
}

// Cancel: back to the origin, and the bar closes: a GL16 of its card, or, with a page looked at, the whole
// screen again (the origin's canvas was never touched), a Full after three peeks (GL16 ghosting builds up).
fn bar_cancel(a: &mut App) {
    let s = ed(a);
    if !s.lb.open {
        return;
    }
    let (r, peeking, full) = (s.lb.rect, s.peek.active, s.peek.count >= 3);
    bar_drop(s);
    if peeking {
        compose_all(a, if full { RefreshMode::Full } else { RefreshMode::Ui },
        );
    } else {
        compose(a, r);
        app_invalidate(a, r, RefreshMode::Ui);
    }
}

// Stay where it is looking, or just close if it is looking at nothing: a tap on the page, a tap outside the
// bar, the page above, a pen down outside.
fn bar_stay(a: &mut App) {
    if ed_ref(a).peek.active {
        bar_commit(a);
    } else {
        bar_cancel(a);
    }
}

// The page above the bar is a page the notebook has now: what go_to_page does, without a render and without a
// second read. The origin is saved and goes on the back list (or, if this page was reached through Back, the
// list is popped, as a browser's Back does); the peeked page takes the origin's place, its canvas swapped into
// page_canvas and its Page into the session cache; one Full, which clears the peeks' ghosting.
fn bar_commit(a: &mut App) {
    let s = ed(a);
    if !s.lb.open || !s.peek.active {
        return;
    }
    let pk = s.peek;
    if !save_page(a) {
        bar_cancel(a);
        return;
    }
    links_flush(a);
    write_thumb_if_needed(a);
    let s = ed(a);
    let from = s.open_pid();
    // The peeked page in the session cache: the one it has, else the one the bar read, moved in.
    let Some(e) = s.entry_for(pk.pid) else {
        bar_cancel(a);
        return;
    };
    if s.hist[e].page.is_none() {
        s.hist[e].page = s.ccache[pk.slot].page.take();
        if s.hist[e].page.is_some() {
            s.migrate_entry(e); // (its first visit: the bar read it, not page_at)
        }
    }
    let ready = s.hist[e].page.is_some() && s.ccache[pk.slot].rows_done >= s.pgh && !s.ccache[pk.slot].white;
    if s.hist[e].page.is_none() {
        // (the page was lost somewhere: read it again, as go_to_page does)
        bar_drop(s);
        go_to_page_pushing(a, pk.index);
        return;
    }
    live_reset();
    a.live_processed = 0;
    a.live_processed_ms = 0;
    let s = ed(a);
    lm_drop(s);
    s.settle = RECT_EMPTY;
    s.panel_stale = false;
    s.page = Some(pk.pid);
    s.index = pk.index;
    s.nb.last_page = pk.index;
    s.info_dirty = true;
    if ready {
        // It was drawn from that very Page: they match by construction (canvas_page).
        let App { ed, .. } = a;
        let s = ed.as_deref_mut().unwrap();
        std::mem::swap(&mut s.page_canvas, &mut s.ccache[pk.slot].canvas);
        s.canvas_page = s.open_page_ref();
        base_reset(s);
        s.boxes.stale = true;
        if s.open_page().is_some_and(|p| paper_is_image(p.paper)) {
            s.paper_stale = true;
        }
        s.picture_missing = false; // (ready: the bar's canvas drew no picture white)
        retire_kept_before(s); // (drawn whole from its strokes, as render_page_full draws it)
    } else {
        render_page_open(a);
    }
    let s = ed(a);
    if pk.from_back {
        s.back.pop();
    } else if from != 0 {
        s.back_push(from);
    }
    bar_drop(s);
    view_reset(a);
    compose_all(a, RefreshMode::Full);
    arrive_readonly(a);
}

// ---- the peek's canvases ----

/// Rows of a page's canvas drawn in one go ahead of a tap.
const CACHE_BAND: i32 = 160;

// The slot that holds this page at this size and picture generation, if any.
fn cache_find(s: &EditorState, pid: u32) -> Option<usize> {
    let gen = bgimage_generation();
    (0..s.ccache.len()).find(|&i| {
        let e = &s.ccache[i];
        e.pid == pid && e.w == s.pgw && e.h == s.pgh && e.pic_gen == gen && !e.canvas.px.is_empty()
    })
}

// A slot for a page: a free one; else (for a tap, not for the prefetch) the least recently used that is not the
// one on show.
fn cache_victim(s: &EditorState, evict: bool) -> Option<usize> {
    if let Some(i) = (0..s.ccache.len()).find(|&i| s.ccache[i].pid == 0 || s.ccache[i].canvas.px.is_empty()) {
        return Some(i);
    }
    if !evict {
        return None;
    }
    // (the oldest by how many shows ago it was shown, across the tick's wrap)
    (0..s.ccache.len()).filter(|&i| !(s.peek.active && s.peek.slot == i)).max_by_key(|&i| s.cache_tick.wrapping_sub(s.ccache[i].used))
}

// A canvas and a page for page `index` in a slot (nothing drawn yet): the session cache's page if it has one,
// else read by the pumped reader. False when there is no memory for the canvas or the page does not read.
fn cache_begin(a: &mut App, slot: usize, index: i32) -> bool {
    let s = ed(a);
    let (pid, w, h) = (s.nb.page_ids[index as usize], s.pgw, s.pgh);
    s.ccache[slot] = CacheEntry::default();
    let e = &mut s.ccache[slot];
    if canvas_init(&mut e.canvas, w, h) != 0 {
        s.ccache[slot] = CacheEntry::default();
        return false;
    }
    e.w = w;
    e.h = h;
    e.pic_gen = bgimage_generation();
    e.from_hist = s.hist.iter().any(|x| x.page_id == pid && x.page.is_some());
    if !e.from_hist {
        let mut p = None;
        if store_load_page_pumped(&s.nb, index, w, h, &mut p) != 0 || p.is_none() {
            s.ccache[slot] = CacheEntry::default();
            return false;
        }
        e.page = p;
    }
    e.pid = pid;
    true
}

// Draws up to `bands` bands of 160 rows of a slot's canvas (all that are left with i32::MAX) from its page.
fn cache_render(a: &mut App, slot: usize, bands: i32) {
    let dpi = a.dpi;
    let s = ed(a);
    let (pgw, pgh) = (s.pgw, s.pgh);
    let App { ed, .. } = a;
    let s = ed.as_deref_mut().unwrap();
    let e = &mut s.ccache[slot];
    if e.pid == 0 || e.rows_done >= pgh {
        return;
    }
    let page: &Page = if e.from_hist {
        match s.hist.iter().find(|x| x.page_id == e.pid).and_then(|x| x.page.as_deref()) {
            Some(p) => p,
            None => return,
        }
    } else {
        match e.page.as_deref() {
            Some(p) => p,
            None => return,
        }
    };
    bgimage_take_missing(); // (what drew white before)
    let mut n = 0;
    while e.rows_done < pgh && n < bands {
        let r = Rect { x: 0, y: e.rows_done, w: pgw, h: CACHE_BAND.min(pgh - e.rows_done) ,
        };
        page_render(&mut e.canvas, page, r, dpi);
        e.rows_done += r.h;
        n += 1;
    }
    if bgimage_take_missing() {
        e.white = true; // (a picture that is not there yet drew white: a commit draws the page again)
    }
}

// The page the bar looks at above itself: from the cache (drawn whole now if the prefetch has not), the origin
// untouched. The page area above the bar and the bar go out as one GL16: no flash, never a GC16. `selected`
// is the entry (-1 for the back list's page).
fn peek_page(a: &mut App, t: i32, selected: i32, from_back: bool) {
    let aw = a.w;
    let s = ed(a);
    if t < 0 || t >= s.nb.npages() {
        return;
    }
    let pid = s.nb.page_ids[t as usize];
    let slot = match cache_find(s, pid) {
        Some(i) => {
            if s.ccache[i].rows_done >= s.pgh {
                s.cache_hits += 1;
            } else {
                s.cache_misses += 1;
            }
            i
        }
        None => {
            s.cache_misses += 1;
            let made = cache_victim(s, true).filter(|&i| cache_begin(a, i, t));
            match made {
                Some(i) => i,
                None => {
                    // No memory for another canvas, or the page does not read: what a link follow does, the same
                    // pixels, slower, no peek (the origin goes on the back list).
                    let s = ed(a);
                    let was = s.lb.open;
                    bar_drop(s);
                    if was {
                        go_to_page_pushing(a, t);
                    }
                    return;
                }
            }
        }
    };
    cache_render(a, slot, i32::MAX);
    let s = ed(a);
    s.cache_tick = s.cache_tick.wrapping_add(1);
    s.ccache[slot].used = s.cache_tick;
    let count = s.peek.count + 1;
    s.peek = Peek { active: true, index: t, pid, slot, count, from_back ,
    };
    s.lb.selected = selected;
    let area = Rect { x: 0, y: 0, w: aw, h: s.lb.rect.y ,
    };
    let bar = s.lb.rect;
    compose(a, area);
    compose(a, bar);
    app_invalidate(a, rect_union(area, bar), RefreshMode::Ui);
}

// The back chip. While a page is looked at it shows the origin again (the bar stays, its header the origin's);
// with none, "Back to ..." looks at the back list's newest page.
fn bar_back(a: &mut App) {
    let aw = a.w;
    let s = ed(a);
    if s.peek.active {
        s.peek = Peek { count: s.peek.count, ..Peek::default() };
        s.lb.selected = -1;
        let (area, bar) = (Rect { x: 0, y: 0, w: aw, h: s.lb.rect.y ,
            }, s.lb.rect,
        );
        compose(a, area);
        compose(a, bar);
        app_invalidate(a, rect_union(area, bar), RefreshMode::Ui);
    } else if let Some((t, _)) = s.back_top() {
        peek_page(a, t, -1, true);
    } else if !s.notebook_back.is_empty() {
        return_to_notebook(a);
    } else if s.from_graph {
        back_to_graph(a);
    }
}

// The next thing the bar can get ready for a tap while the pens and fingers are idle: the pages its entries on
// screen go to, then the back list's newest, each read (one page a tick) and drawn (one band of 160 rows a tick)
// into a free slot. It never evicts what is there, and it stops at the first contact (editor_tick asks).
fn prefetch_step(a: &mut App) {
    let s = ed(a);
    if !s.lb.open {
        return;
    }
    let mut want: Vec<i32> = Vec::new();
    for e in linkbar_visible(&s.lb) {
        if e.page >= 0 && e.page != s.index && !want.contains(&e.page) {
            want.push(e.page);
        }
    }
    for f in linkbar_visible_from(&s.lb) {
        if f.page >= 0 && f.page != s.index && !want.contains(&f.page) {
            want.push(f.page);
        }
    }
    if let Some((t, _)) = s.back_top() {
        if t != s.index && !want.contains(&t) {
            want.push(t);
        }
    }
    for t in want {
        let s = ed(a);
        let pid = s.nb.page_ids[t as usize];
        match cache_find(s, pid) {
            Some(i) => {
                if s.ccache[i].rows_done < s.pgh {
                    cache_render(a, i, 1);
                    return;
                }
            }
            None => {
                // (a free slot only: what is there is wanted by a tap as much)
                let Some(i) = cache_victim(s, false) else { return ;
                };
                if cache_begin(a, i, t) {
                    return; // (a page read is the tick's work)
                }
                return;
            }
        }
    }
}

// What the bar was asked (linkbar_pointer's answer to a tap), after the rect of the button let go is
// refreshed.
fn bar_result(a: &mut App, r: LbResult, d: Rect) {
    if !rect_is_empty(d) {
        app_invalidate(a, d, RefreshMode::Ui);
    }
    match r {
        LbResult::NamePage => {bar_stay(a);pages_name(a);}
        LbResult::Cancel => bar_cancel(a),
        LbResult::Peek(i) => {
            if let Some(f) = linkbar_from_row(&ed_ref(a).lb, i).cloned() {
                // Linked from: a page here is looked at as an entry's page is; another notebook's is followed
                // (Back returns).
                if f.page >= 0 {
                    if f.page != ed_ref(a).index {peek_page(a, f.page, i, false);}
                } else {bar_stay(a);follow_target(a, f.target.as_str());}
                return;
            }
            let Some(e) = linkbar_entry(&ed_ref(a).lb, i).copied() else { return ;
            };
            if ed(a).lb.links_ok && linkbar_entry_broken(&mut ed(a).lb, i) {
                // A link to a page or notebook that was deleted: its card at once, to change where it goes or
                // delete it (the card's title says what is gone).
                if let Some(chip) = linkbar_chip_rect(&ed_ref(a).lb, i) {bar_hold(a, i, chip);}
                return;
            }
            if nav_external_target(e.target.as_str()).is_some() {bar_stay(a);follow_target(a,e.target.as_str());}
            else if e.page == NAV_TO_INDEX {
                // The link target "index" keeps its list meaning; only the bar's Index button is the map.
                if ed_ref(a).peek.active {bar_commit(a);}
                bar_drop(ed(a));index_show(a);
            } else if e.page < 0 {
                // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a finger tapped a link
                // whose page was deleted (or that a newer Grimoire made, to a place this one cannot open).
                app_toast(a, tr("Linked page isn't in this notebook."), 3000);
            } else if e.page != ed_ref(a).index {
                peek_page(a, e.page, i, false);
            }
        }
        LbResult::Commit => bar_commit(a),
        LbResult::Back => bar_back(a),
        LbResult::GoTo => {
            // Find acts on the page that shows: stay on it, then the sheet (the go-to-page action's, which
            // takes a page number or a name).
            bar_stay(a);
            find_show(a);
        }
        LbResult::NewLink => {
            let s = ed(a);
            if s.lb.readonly {
                // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): New link on the Links bar
                // was tapped on a page from a newer Grimoire, which cannot be changed.
                app_toast(a, tr("This page is read-only."), 2500);
            } else if !s.lb.links_ok {
                // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): links.ini could not be
                // read, so the Links bar lists none and cannot make one.
                app_toast(a, tr("Couldn't read links."), 3000);
            } else {
                // The new link goes on the page that shows: commit to it first, then the strip; the bar comes
                // back once it is made or cancelled.
                bar_stay(a);
                link_arm(a, true, None);
            }
        }
        LbResult::Eye => {
            let s = ed(a);
            if !s.lb.links_ok {
                app_toast(a, tr("Couldn't read links."), 3000);
            } else {
                let button = s.lb.eye_button;
                areas_toggle(a, button);
            }
        }
        LbResult::Around => {
            bar_drop(ed(a));
            map_show(a, true);
        }
        LbResult::Index => {
            if ed_ref(a).peek.active {
                bar_commit(a);
            }
            bar_drop(ed(a));
            map_show(a, false);
        }
        LbResult::Pager => {
            let g = ed(a).lb.grid;
            compose(a, g);
            app_invalidate(a, g, RefreshMode::Ui);
        }
        LbResult::Docs | LbResult::DocsBack => bar_relayout(a),
        _ => {}
    }
}

// The bar's chips changed (into the document links or back out): its card is laid out again, which can make it
// taller or shorter, and drawn where it was and where it is.
fn bar_relayout(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let s = ed(a);
    let old = s.lb.rect;
    linkbar_layout(&mut s.lb, aw, ah);
    let u = rect_union(old, s.lb.rect);
    compose(a, u);
    app_invalidate(a, u, RefreshMode::Ui);
}

fn panel_act(a: &mut App, h: Hit) {
    // Choosing a pen leaves Select mode.
    if h.id == P_TOOL || (h.id == P_PICK_TOOL && !ed_ref(a).pin_mode) {
        sel_off(a);
    }
    match h.id {
        P_TOOLPICK => {
            // From any other tool, including the fixed eraser, the tool button's pen is one tap away; a tap
            // while that pen is in use opens the picker. So does every tap on the grid (its pen pinned).
            let cur = a.settings.current_tool;
            let dt = button_tool(&a.settings);
            if cur != dt && !settings_pinned(&a.settings, dt) {
                sel_off(a);
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
                    app_toast(a, tr("Couldn't add page"), 3000);
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
        P_SHAPES => open_shape_menu(a),
        P_PAGES => pages_show(a),
        // The dedicated toolbar button creates a link; the upward swipe opens the Links bar.
        P_LINKS => {
            if ed_ref(a).lm.state != LmState::Off { link_cancel(a); }
            else { link_arm(a,false,None); }
        }
        P_DISPLAY => {
            with_modal(a, |s| &mut s.dm, dispmenu_open);
            let r = with_modal(a, |s| &mut s.dm, dispmenu_draw);
            app_invalidate(a, r, RefreshMode::Ui);
        }
        P_PENSET => {
            let t = a.settings.current_tool;
            pensettings_open(a, t);
            // The first time Pen feel opens from the toolbar, once (hints bit 9): where a user looks for a width,
            // the way to a quicker one.
            if a.settings.hints & HINT_QUICK_SIZE == 0 {
                a.settings.hints |= HINT_QUICK_SIZE;
                settings_save(&a.settings);
                // TRANSLATORS: a toast, once, the first time Pen feel is opened from the toolbar (about 1300
                // px at 44 px in bold, or two lines): a pen button on the toolbar, held for a moment, opens a
                // card of that pen's widths.
                app_toast(a, tr("Hold a pen on the toolbar for its sizes."), 4000);
            }
        }
        P_SETTINGS => {
            fs_abort_view(a,FsAbortView::Screen);settle_now(a);
            if save_page(a){links_flush(a);pb_reset(ed(a));close_sheets(ed(a));crate::library::library_settings_from_notebook(a);}
        }
        P_LIBRARY => {
            // (no "Saving..." or "Saved" toast: the owner, 2026-11-04; a page that cannot be saved says so)
            app_flush(a);
            if !save_page(a) {
                return; // stay with the unsaved page rather than drop it
            }
            // (a toast left up from the page does not follow to the Library; one the Library raises as it opens,
            // its Graph's, stays)
            app_toast_dismiss(a);
            // (a notebook the Library's Graph opened goes back to that Graph: the Library as it was left)
            let to_graph = ed_ref(a).from_graph;
            close_notebook(a);
            ed(a).from_graph = false;
            if to_graph {
                crate::library::library_return_to_graph(a);
            }
            app_switch(a, ScreenId::Library);
        }
        P_COLLAPSE => set_collapsed(a, true),
        P_EXPAND => set_collapsed(a, false),
        _ => {}
    }
}

// The menu in front of the page, as the action of its button: the tool picker, Paper (the paper
// sheet, its Templates sheet and Custom music), Display, or the Index (index-map, whichever of its
// views shows; Pages for a picking Pages view); 0 with none.
fn front_menu(s: &EditorState) -> i32 {
    if s.map.open {
        return SHORTCUT_INDEX_MAP;
    }
    if s.find.open {
        return SHORTCUT_GO_TO_PAGE;
    }
    if s.lb.open {
        return SHORTCUT_LINKS;
    }
    if s.pv.open {
        return if (s.pv.unified&&!s.pv.picking)||s.pv.index_tab{SHORTCUT_INDEX_MAP}else{SHORTCUT_PAGES};
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
    action == SHORTCUT_INDEX_MAP
        || action == SHORTCUT_GO_TO_PAGE
        || action == SHORTCUT_LINKS
        || action == SHORTCUT_PAGES
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
    if ed_ref(a).map.open {
        map_hide(a, draw);
        return;
    }
    let s = ed(a);
    if s.find.open {
        if draw {
            find_close(a);
        } else {
            find_drop(s);
        }
        return;
    }
    if s.lb.open {
        if draw {
            bar_cancel(a);
        } else {
            bar_drop(s);
        }
        return;
    }
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
    if s.qs.open {
        r = rect_union(r, s.qs.rect);
    }
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
        SHORTCUT_INDEX_MAP => map_show(a, false),
        SHORTCUT_GO_TO_PAGE => find_show(a),
        SHORTCUT_LINKS => bar_open(a),
        SHORTCUT_PAGES => pages_show(a), // (the toolbar's page count)
        SHORTCUT_TOOL_PICKER => open_tools(a),
        SHORTCUT_PAPER => panel_act(a, Hit { id: P_PAPER, ..Default::default() },
        ),
        SHORTCUT_DISPLAY => panel_act(a, Hit { id: P_DISPLAY, ..Default::default() },
        ),
        SHORTCUT_PEN_FEEL => panel_act(a, Hit { id: P_PENSET, ..Default::default() },
        ),
        SHORTCUT_LIBRARY => panel_act(a, Hit { id: P_LIBRARY, ..Default::default() },
        ),
        // Nothing in front: a link being made is not made.
        SHORTCUT_CLOSE_MENU if ed(a).lm.state != LmState::Off => link_cancel(a),
        _ => {}
    }
}

/// Erase while held's button let go (false)
pub fn editor_hold_eraser(a: &mut App, held: bool) {
    if let Some(s) = a.ed.as_deref_mut() {
        s.hold_eraser_key = held;
        s.hold_eraser = held || s.pb_erasers != 0; // A stroke under way keeps what it does.
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
/// The open on-screen keyboard over the notebook (the Pages view's, Find's, the Graph's Rename, New
/// page..., the card's Rename), None for none: where a hardware key's input goes (app_type_text).
pub fn editor_keyboard_area(a: &App) -> Option<Rect> {
    if crate::library::library_settings_active(a) {
        return crate::library::library_keyboard_area(a);
    }
    let s = a.ed.as_deref()?;
    if !s.open {
        return None;
    }
    [&s.map_kb, &s.pv.kb, &s.find.kb, &s.lm.kb, &s.card.kb, &s.add_kb].into_iter().find(|k| k.open && !rect_is_empty(k.area)).map(|k| k.area)
}

/// Where a key pressed on the notebook acts (actions.rs app_place): its Graph, its Pages view, else the page.
pub fn editor_place(a: &App) -> u8 {
    match a.ed.as_deref() {
        Some(s) if s.open && s.map.open => PL_GRAPH,
        Some(s) if s.open && s.pv.open => PL_PAGES,
        _ => PL_PAGE,
    }
}

/// The notebook's Graph is up (its keys are the Graph's).
pub fn editor_graph_up(a: &App) -> bool {
    a.ed.as_deref().is_some_and(|s| s.open && s.map.open)
}

pub fn editor_shortcut(a: &mut App, action: i32) -> bool {
    let (x, y) = (a.w as f32 * 0.5, a.h as f32 * 0.5);
    editor_shortcut_at(a, action, x, y, ActionSrc::Key)
}

/// The same for an action that came from src, at the point (x, y) it came from (a key's is the screen's
/// centre). A page turn from a touch gesture never adds a page: on the last page it says so, as on the
/// first.
pub fn editor_shortcut_at(a: &mut App, action: i32, x: f32, y: f32, src: ActionSrc) -> bool {
    let r = editor_shortcut_in(a, action, x, y, src);
    lm_check_menus(a);
    touch_context(a);
    r
}

fn editor_shortcut_in(a: &mut App, action: i32, x: f32, y: f32, src: ActionSrc) -> bool {
    if crate::library::library_settings_active(a) {
        if action == SHORTCUT_BACK {
            return crate::library::library_back(a);
        }
        if action == SHORTCUT_SAVE {
            settings_save(&a.settings);
            app_toast_dismiss(a); // (no "Saved": the owner, 2026-11-04; a toast left up from before goes)
            return true;
        }
        if action==SHORTCUT_GO_BACK || action==SHORTCUT_CLOSE_MENU {crate::library::library_settings_back(a);return true;}
        return app_global_action(a,action,src);
    }
    if action == SHORTCUT_STATUS_MODE && !editor_busy(a) { status_cycle_mode(a); return true; }
    let asleep = a.asleep;
    let Some(s) = a.ed.as_deref_mut() else { return false ;
    };
    if !s.open || asleep || action == 0 {
        return false;
    }
    if s.destination.open {
        if action==SHORTCUT_GO_BACK||action==SHORTCUT_CLOSE_MENU||action==SHORTCUT_BACK {s.destination.open=false;s.lt.hidden=false;compose_all(a,RefreshMode::Full);return true;}
        return false;
    }
    if action == SHORTCUT_HOLD_ERASER {
        s.hold_eraser_key = true;
        s.hold_eraser = true;
        return true;
    }
    // A drag in the Pages view (a held tile moved): Esc cancels it, the pages where they were; nothing else is
    // done while it lasts.
    if (action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_BACK) && pagesview_dragging(&s.pv) {
        return with_pv(a, pagesview_cancel_drag);
    }
    if s.owner != Owner::None || s.pinching {
        return false;
    }
    // Ctrl+S: the page and its notebook written now, whatever is in front (nothing is lost by it). No toast
    // (the owner, 2026-11-04): a page that cannot be saved says so.
    if action == SHORTCUT_SAVE {
        settings_save(&a.settings);
        if save_page(a) {
            app_toast_dismiss(a); // (a toast left up from before, a failed save's, goes)
        }
        return true;
    }
    // Esc with a name, a number or Find being typed: Cancel, as the keyboard's own (5.2, layer 1).
    if action == SHORTCUT_BACK && editor_keyboard_area(a).is_some() {
        if !app_type_text(a, KbInput::Cancel) {
            close_front(a, true);
        }
        return true;
    }
    let Some(s) = a.ed.as_deref_mut() else { return false };
    // The toolbar's count pad: Close menu and Back close it with nothing added; nothing else is done under it.
    if s.add_kb.open {
        if action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK {
            add_pad_cancel(a);
            return true;
        }
        return false;
    }
    // An armed link selection leaves the page visible but is still an active operation.
    // A hovering pen-button hold must not clear its existing ink or link areas underneath it.
    if action == SHORTCUT_CLEAR_PAGE && s.lm.state != LmState::Off {
        return false;
    }
    // The Graph's own keys first: pans, zoom, Fit, its view, Select all, Enter, and Esc's
    // card-then-selection (not with its naming keyboard up).
    if s.map.open && !s.map_kb.open {
        if let Some(r) = with_map(a, |map, a, _, nav| mapview_action(map, a, nav, action)) {
            map_result(a, r);
            return true;
        }
    }
    let Some(s) = a.ed.as_deref_mut() else { return false };
    if s.map.open {
        if s.map_kb.open {
            if action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK {
                keyboard_close(&mut s.map_kb);
                compose_all(a, RefreshMode::Ui);
                return true;
            }
            return false;
        }
        if action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK {
            map_hide(a, true);
            return true;
        }
    }
    // The long-press card (8.11): with Rename's keyboard up nothing is done; Close menu and Back close the card
    // (back to the bar under it, if any); another menu's button closes it and goes on (with the bar under it,
    // the bar's own rules then apply); anything else is refused.
    if s.card.open {
        if s.card.kb.open {
            return false;
        }
        if action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK {
            card_close(a);
            return true;
        }
        if !menu_action(action) {
            return false;
        }
        card_close(a);
    }
    // The quick sizes card: Close menu and Back close it, one layer; another menu's button closes it and goes on;
    // anything else is refused.
    if ed_ref(a).qs.open {
        let action_closes = action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK;
        if action_closes {
            qs_close(a);
            return true;
        }
        if !menu_action(action) {
            return false;
        }
        qs_close(a);
    }
    let Some(s) = a.ed.as_deref_mut() else { return false ;
    };
    // New page...'s keyboard: Close menu and Back go back to Link to...; nothing else throws the typing away.
    if s.lm.state == LmState::Naming && s.lm.kb.open {
        if action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK {
            naming_back(a);
            return true;
        }
        return false;
    }
    // Link to... in front: Close menu, Back and add-link again make no link (6.10); another menu's button
    // ends the making silently and opens its menu (8.11).
    if s.lt.open && !s.lt.hidden {
        if action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_GO_BACK || action == SHORTCUT_BACK || action == SHORTCUT_ADD_LINK {
            link_cancel(a);
            return true;
        }
        if menu_action(action) {
            link_off(a);
            open_menu(a, action);
            return true;
        }
        return false;
    }
    // The Find sheet over the Pages view: Close menu, Back (Esc) and Go to page close it,
    // the view behind it as it was (its screen, its ticks); another menu's action closes it unseen and goes on with
    // the view in front.
    if s.find.open && s.find.mode == FindMode::Go && s.pv.open && !s.pv.picking {
        if action == SHORTCUT_GO_TO_PAGE || action == SHORTCUT_CLOSE_MENU || action == SHORTCUT_BACK {
            find_close(a);
            return true;
        }
        if !menu_action(action) {
            return false;
        }
        find_drop(s);
    }
    let Some(s) = a.ed.as_deref_mut() else { return false };
    if action == SHORTCUT_BACK {
        return editor_back(a);
    }
    if action == SHORTCUT_TWO_FINGER_UP {
        return editor_two_finger_up(a);
    }
    let mut action = action;
    if s.pv.open && !s.pv.picking && !s.pv.kb.open {
        // (Esc's `back` in the Pages view went to editor_back above: pagesview_back takes its drag, its card, then
        // its ticks, then the view closes.)
        if action == SHORTCUT_CLOSE_MENU && with_pv(a, pagesview_close_card) {
            // Close menu (Esc in a shortcuts.ini that keeps `key.Escape=close-menu`): the card alone, as its Cancel
            // (layer 3, which close-menu keeps; the ticks, layer 6, are back's alone).
            return true;
        }
        let s = ed(a);
        let grid = s.pv.unified && !s.pv.index_tab && !s.pv.overlay && !s.pv.name_only && s.pv.drag_free();
        // Go to page (Ctrl+F): the Find sheet over the view, which stays behind it, its card closed first.
        if action == SHORTCUT_GO_TO_PAGE && grid {
            with_pv(a, pagesview_close_card);
            find_show(a);
            return true;
        }
        if pagesview_grid_ready(&ed_ref(a).pv) {
            match action {
                // New page (Ctrl+N): the count pad at once, at 1
                SHORTCUT_NEW_PAGE | SHORTCUT_NEW_PAGES => return with_pv(a, pagesview_open_add),
                // Select all (Ctrl+A): every page ticked
                SHORTCUT_SELECT_ALL => return with_pv(a, pagesview_select_all),
                _ => {}
            }
        }
    }
    let s = ed(a);
    let mut front = front_menu(s);
    // The Index's Pages view is Pages' own menu as well: pages there closes it, as index-map does.
    if action == SHORTCUT_PAGES && front == SHORTCUT_INDEX_MAP && s.pv.open {
        front = SHORTCUT_PAGES;
    }
    if front != 0 && menu_action(action) {
        if s.pv.open && s.pv.kb.open {
            return false;
        }
        // Another menu's button or key while the bar looks at a page: that page is the one the menu is for.
        if front == SHORTCUT_LINKS && s.peek.active && action != SHORTCUT_LINKS && action != SHORTCUT_CLOSE_MENU {
            bar_commit(a);
            open_menu(a, action);
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
    // The Pages view in front: the page-turn actions turn its screen (keys, a controller, a gesture; its
    // arrows left the bottom row in 2.0.0-rc.12, where one finger flicked sideways turns it too).
    if (action == SHORTCUT_NEXT_PAGE || action == SHORTCUT_PREV_PAGE) && s.pv.open {
        if s.pv.kb.open {
            return false;
        }
        let next = action == SHORTCUT_NEXT_PAGE;
        with_pv(a, |pv, a, nb, nav| pagesview_turn(pv, a, nb, nav, next));
        return true;
    }
    // The Pages view in front: undo and redo are its own (a page move, a batch of new pages), never the page's.
    if (action == SHORTCUT_UNDO || action == SHORTCUT_REDO) && s.pv.open {
        return pages_history_action(a, action);
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
        draw_panel_over(a); // (the picker over the panel where it covers it)
        let s = ed(a);
        let r = rect_union(s.panel, s.tools_rect);
        app_invalidate(a, r, RefreshMode::Ui);
        return true;
    }
    // Back with Link to...'s Search up: Link to... again.
    if action == SHORTCUT_GO_BACK && s.find.open && s.find.mode == FindMode::Pick {
        find_close(a);
        return true;
    }
    // Back to the page before, with the bar up, is the bar's Cancel (the refusal below would swallow it).
    if action == SHORTCUT_GO_BACK && s.lb.open {
        bar_cancel(a);
        return true;
    }
    if sheet_open(s) {
        return false;
    }
    match action {
        SHORTCUT_SHAPES=>{editor_shapes_toggle(a);return true;},
        SHORTCUT_SHAPE_HOLD=>{editor_shape_hold_cycle(a);return true;},
        SHORTCUT_SHAPE_LAST=>{editor_shape_last(a);return true;},_=>{}
    }
    if (SHORTCUT_INVERT..=SHORTCUT_UPDATE_CHANNEL).contains(&action) {
        return app_global_action(a, action, src);
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
        sel_off(a);
        let t = ToolKind(action - SHORTCUT_TOOL_BASE);
        if t != TOOL_ERASER && !settings_pinned(&a.settings, t) {
            a.settings.draw_tool = t;
        }
        panel_act(a, Hit { id: P_TOOL, arg: t.0, ..Default::default() },
        );
    } else if action == SHORTCUT_UNDO || action == SHORTCUT_REDO {
        panel_act(a, Hit { id: if action == SHORTCUT_UNDO { P_UNDO } else { P_REDO }, ..Default::default() },
        );
    } else if action == SHORTCUT_PREV_PAGE || action == SHORTCUT_NEXT_PAGE {
        // The panel's arrows: next page on the last page adds one, as its + does. So does a gesture's turn
        // (settings.gesture_new_page, on by default); with it off, a gesture stops at the last page, as one
        // stops at the first: the page stays, with no toast (the owner, 2026-11-04).
        let (index, npages, may_add) = (ed(a).index, ed(a).nb.npages(), src == ActionSrc::Touch && a.settings.gesture_new_page,
        );
        if src != ActionSrc::Key && src != ActionSrc::Pad {
            if action == SHORTCUT_NEXT_PAGE && index >= npages - 1 && !may_add {
                return true;
            }
            if action == SHORTCUT_PREV_PAGE && index <= 0 {
                return true;
            }
        }
        panel_act(a, Hit { id: if action == SHORTCUT_NEXT_PAGE { P_NEXT } else { P_PREV }, ..Default::default() },
        );
    } else if action == SHORTCUT_FIRST_PAGE || action == SHORTCUT_LAST_PAGE {
        let s = ed(a);
        let to = if action == SHORTCUT_FIRST_PAGE { 0 } else { s.nb.npages() - 1 };
        if to != s.index {
            go_to_page(a, to);
        }
    } else if action == SHORTCUT_SELECT {
        sel_toggle(a);
    } else if action == SHORTCUT_SELECT_EXACT {
        sel_exact_action(a);
    } else if action == SHORTCUT_ADD_LINK {
        // The Index's Add a link from the page; pressed again while the strip is up, the link is not made,
        // as a menu's own button closes it.
        if ed(a).lm.state != LmState::Off {
            link_cancel(a);
        } else {
            link_arm(a, false, None);
        }
    } else if action == SHORTCUT_NAME_PAGE {
        pages_name(a);
    } else if action == SHORTCUT_GO_BACK {
        ed(a).back_prune();
        ed(a).back_trim();
        if ed_ref(a).back.is_empty() && !ed_ref(a).notebook_back.is_empty() {
            return return_to_notebook(a);
        }
        if ed_ref(a).back.is_empty() && ed_ref(a).from_graph {
            return back_to_graph(a);
        }
        // Back to the page the last link, Find or Pages view left: the newest on the list that is still
        // in the notebook and is not the page on show, which is not pushed again.
        let s = ed(a);
        s.back_prune();
        s.back_trim();
        match s.back.pop() {
            Some(pid) => {
                let at = (0..s.nb.npages()).find(|&i| s.nb.page_ids[i as usize] == pid);
                if let Some(i) = at {
                    go_to_page(a, i);
                }
            }
            None => {
                // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Back to the page
                // before was asked with no page to go back to.
                app_toast(a, tr("Nothing to go back to."), 2000);
            }
        }
    } else if action == SHORTCUT_GESTURE_NEW_PAGE {
        // (no toast: the owner, 2026-11-04; one left up from before goes)
        a.settings.gesture_new_page = !a.settings.gesture_new_page;
        settings_save(&a.settings);
        app_toast_dismiss(a);
    } else if action == SHORTCUT_LANDSCAPE_FIT {
        a.settings.landscape_fit = !a.settings.landscape_fit;
        settings_save(&a.settings);
        editor_fit_changed(a, true);
        app_toast_dismiss(a); // (no toast: the owner, 2026-11-04; one left up from before goes)
    } else if action == SHORTCUT_LINK_AREAS {
        areas_toggle(a, RECT_EMPTY);
    } else if action == SHORTCUT_NEW_PAGE {
        let i = ed(a).index;
        pages_new(a, i + 1); // the Pages view's New page, after the open one
    } else if action == SHORTCUT_NEW_PAGES {
        add_pad_open(a); // the toolbar's + held: the count pad, the pages after the open one, which stays
    } else if action == SHORTCUT_CLEAR_PAGE {
        clear_page(a); // the panel's broom: no question, one undo step
    } else if action == SHORTCUT_ZOOM_OUT || action == SHORTCUT_ZOOM_IN {
        let s = ed(a);
        let was = (s.zoom, s.ox, s.oy);
        let (mut px, mut py) = (0.0, 0.0);
        to_page(s, x, y, &mut px, &mut py);
        let factor = if action == SHORTCUT_ZOOM_IN { 1.25 } else { 0.8 };
        s.zoom = clampf(s.zoom * factor, s.zoom_min, s.zoom_max);
        s.ox = px - x / s.zoom;
        s.oy = py - y / s.zoom;
        view_clamp(a);
        // (at the most zoom, or the least, the view stays: nothing to draw)
        if view_moved(ed_ref(a), was) {
            compose_all(a, RefreshMode::Ui);
        }
    } else if action == SHORTCUT_SCROLL_UP || action == SHORTCUT_SCROLL_DOWN {
        let top = status_reserved(a) as f32;
        let step = a.h as f32 * 0.75;
        let height = (a.h as f32 - top).max(1.0);
        let down = action == SHORTCUT_SCROLL_DOWN;
        let s = ed(a);
        let (zoom, ox) = (s.zoom, s.ox);
        let edge = (s.pgh as f32 - height / zoom).max(0.0);
        let next = s.oy + top / zoom + if down { step } else { -step } / zoom;
        let to = s.index + if down { 1 } else { -1 };
        let crossing = (down && next > edge) || (!down && next < 0.0);
        if crossing && to >= 0 && to < s.nb.npages() {
            if go_to_page(a, to) {
                let s = ed(a);
                s.zoom = zoom;
                s.ox = ox;
                s.oy = if down { 0.0 } else { (s.pgh as f32 - height / zoom).max(0.0) } - top / zoom;
                view_clamp(a);
                // go_to_page's pending GC16 also carries this view at the preserved zoom.
                compose_all(a, RefreshMode::Ui);
            }
        } else {
            let was = (zoom, ox, ed_ref(a).oy);
            ed(a).oy = next - top / zoom;
            view_clamp(a);
            // (at the end of the first or the last page the view stays: nothing to draw)
            if view_moved(ed_ref(a), was) {
                compose_all(a, RefreshMode::Ui);
            }
        }
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
        // (an open picker's ink row follows)
        if ed_ref(a).tools_open && !ed_ref(a).shapes_menu && !ed_ref(a).pv.open {
            let r = ed_ref(a).tools_rect;
            compose(a, r);
            app_invalidate(a, r, RefreshMode::Ui);
        }
    } else {
        return false;
    }
    true
}

/// Esc (`back`) on a notebook, once the layers editor_shortcut_in answers first are passed (a keyboard, the
/// count pad, the Graph's Rename, the long-press card, New page...'s keyboard, Link to...): the first layer in
/// front eats it. The Pages view's card, then its ticks, then the view; the lasso's
/// selection, then Select mode; a menu or sheet, one level a press (the Templates sheet back to Paper); a link
/// being placed; and with nothing above the page, back to the Library as the toolbar's Library button goes
/// (saving first; to the Graph a notebook was opened from).
fn editor_back(a: &mut App) -> bool {
    let s = ed_ref(a);
    if s.pv.open {
        if !s.pv.picking && with_pv(a, |pv, a, nb, nav| pagesview_back(pv, a, nb, nav)) {
            return true;
        }
        close_front(a, true);
        return true;
    }
    match s.sel.state {
        SelState::Off => {}
        SelState::Selected | SelState::Dragging => {
            sel_deselect(a);
            return true;
        }
        _ => {
            sel_off(a);
            return true;
        }
    }
    if s.ts.open {
        // (as its own Back: the Paper sheet again, in the same place)
        let d = ed_ref(a).ts.rect;
        tplsheet_close(&mut ed(a).ts);
        compose(a, d);
        ed(a).paper_open = true;
        draw_paper_sheet(a);
        let p = ed(a).paper_rect;
        app_invalidate(a, rect_union(d, p), RefreshMode::Ui);
        return true;
    }
    if front_menu(s) != 0 {
        close_front(a, true);
        return true;
    }
    if s.lm.state != LmState::Off {
        link_cancel(a);
        return true;
    }
    if sheet_open(s) {
        close_front(a, true);
        return true;
    }
    panel_act(a, Hit { id: P_LIBRARY, ..Default::default() });
    true
}

/// Space (`two-finger-up`): what two fingers swiped up do on the page (gestures.ini touch.2.up; built in, and
/// with gestures off, the Links bar), as that swipe would do it there now.
fn editor_two_finger_up(a: &mut App) -> bool {
    // (refused where the swipe is, gestures on or off: with them off the Links bar would close Find)
    if gestures_refused(ed_ref(a)) {
        return false;
    }
    let key = GestureKey::Touch { fingers: 2, kind: TouchKind::Up };
    if !a.settings.gestures {
        return editor_shortcut_at(a, SHORTCUT_LINKS, a.w as f32 * 0.5, a.h as f32 * 0.5, ActionSrc::Touch);
    }
    match gesturemap_action(&a.gestures, key) {
        0 => return false,
        // (a file binding the swipe to Space's own action: the built-in, never Space again and again)
        SHORTCUT_TWO_FINGER_UP => {
            return editor_shortcut_at(a, SHORTCUT_LINKS, a.w as f32 * 0.5, a.h as f32 * 0.5, ActionSrc::Touch);
        }
        _ => {}
    }
    let ev = InEvent {
        type_: InType::Gesture,
        value: G_2_UP,
        x2: a.w as f32 * 0.5,
        y2: a.h as f32 * 0.5,
        ..Default::default()
    };
    editor_gesture(a, &ev);
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
    mark_changed(s); // (before it is drawn: page_is_its_file)
    render_page_full(a);
    compose_all(a, RefreshMode::Full);
    ed(a).last_up = now_ms(); // (the idle save 2 s after the drawing, seconds on a dense page, as before)
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
        if why == PREVIEW_NOT_MADE {
            app_toast(a, not_ready_yet(), 4000);
            return;
        }
        let mut msg = String::new();
        // TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines): a template picked
        // on the Templates sheet could not become the page's paper (or the new notebook's); %s is
        // the log file that says why.
        csnprintf!(&mut msg, 256, tr("Couldn't use template (see %s)."), "grimoire.log");
        app_toast(a, &msg, 4000);
        return;
    }
    if s.open_page().unwrap().paper != k {
        apply_paper(a, k);
    }
}

// A template or photo picked a moment after it was copied in: its preview (and the check that it is a picture
// Grimoire takes) is still being made in the background.
fn not_ready_yet() -> &'static str {
    // TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines): a template or photo was picked on
    // its sheet while Grimoire is still making its preview in the background (its tile shows the hourglass);
    // picking it again works once the tile shows its picture.
    tr("Not ready yet. Try again in a moment.")
}

// A photo picked on the Photos sheet becomes the open page's paper, as a template does (templates.h:
// a document page on its own picture, made upright in the background; Undo puts the old paper back).
fn apply_photo(a: &mut App, path: &str) {
    let mut k = PaperKind::default();
    let mut why: &'static str = "";
    let s = ed(a);
    if photo_install(&s.nb, path, &mut k, &mut why) != 0 {
        qlog!("editor: %s: a photo could not be used: %s", s.nb.id.as_str(), why);
        if why == PREVIEW_NOT_MADE {
            app_toast(a, not_ready_yet(), 4000);
            return;
        }
        let mut msg = String::new();
        // TRANSLATORS: in a toast (about 1700 px at 44 px in bold, or two lines): a photo picked on the
        // Photos sheet could not become the page's paper; %s is the log file that says why.
        csnprintf!(&mut msg, 256, tr("Couldn't use photo (see %s)."), "grimoire.log");
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
            with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_open_photos(ts, a, &nb, paper),
            );
        } else {
            with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_open(ts, a, Some(&nb), paper, None),
            );
        }
        let r = with_modal(a, |s| &mut s.ts, tplsheet_draw);
        app_invalidate(a, rect_union(sheet, r), RefreshMode::Ui);
        return;
    }
    if h.id == P_PICK_CUSTOM {
        compose(a, sheet);
        let (pgw, pgh) = (ed(a).pgw, ed(a).pgh);
        with_modal(a, |s| &mut s.mb, |mb, a| musicbuild_open_page(mb, a, paper, pgw, pgh),
        );
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
        s.panel_down_k = in_sample_ms(ev);
        let h = if modal { hits_find(&s.modal_hits, ev.x, ev.y).copied() } else { panel_hit_at(s, ev.x, ev.y) };
        s.has_pressed = h.is_some();
        s.dragging = false;
        if let Some(h) = h {
            s.pressed = h;
            s.grab_dx = ev.x - s.panel.x as f32;
            s.grab_dy = ev.y - s.panel.y as f32;
            s.drag_start_rect = s.panel;
            s.drag_trail = s.panel;
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
                let s = ed(a);
                s.drag_trail = rect_union(s.drag_trail, s.panel);
            }
        }
        return;
    }
    // Release.
    press_move(&mut s.press, ev.x, ev.y);
    s.has_pressed = false;
    if s.dragging {
        s.dragging = false;
        // (a cancel, the pen come near or a second finger, leaves it where it was last drawn: its
        // off-screen place would throw it into the corner)
        let (x, y) = if touch_cancelled(ev) {
            (s.panel.x, s.panel.y)
        } else {
            ((ev.x - s.grab_dx) as i32, (ev.y - s.grab_dy) as i32)
        };
        move_panel_to(a, x, y, RefreshMode::Ui);
        // Also clean up wherever the panel passed through: every place a fast frame drew it, not only
        // where it began and ended (issue #11: a drag that went round left the page between them
        // dithered). DU left a trace of the panel at each of those places that a grayscale refresh
        // does not reach (display.rs, DuWear), so they are cleaned with one flash of the trail alone;
        // with the pen coming (it cancelled the drag, or it is near) grayscale now, and the clean at
        // the next pause.
        let s = ed(a);
        let trail = rect_union(rect_union(s.drag_start_rect, s.drag_trail), s.panel);
        s.drag_trail = RECT_EMPTY;
        compose(a, trail);
        if touch_cancelled(ev) || ed_ref(a).pen_near {
            app_invalidate(a, trail, RefreshMode::Ui);
            du_wear_owe(&mut a.du_wear, trail, now_ms());
        } else {
            app_invalidate(a, trail, RefreshMode::Clean);
        }
        persist_panel(a);
        return;
    }
    let h = s.pressed;
    let p = s.press;
    if h.id != P_GRIP && h.id != P_EXPAND {
        canvas_invert(&mut a.screen, h.r);
        // (a press a second finger took over, the cancel, is let go with a Fast refresh as it was pressed)
        app_invalidate(a, h.r, if touch_cancelled(ev) { RefreshMode::Fast } else { RefreshMode::Ui },
        );
    }
    // (a lift on the button pressed, or for the panel on the strip around it that took the landing)
    let lifted_on = if modal { rect_contains(h.r, ev.x as i32, ev.y as i32) } else { panel_hit_at(ed_ref(a), ev.x, ev.y) == Some(h) };
    if !press_is_tap(&p, h.r) && !lifted_on {
        return;
    }
    if modal {
        modal_act(a, h);
    } else if h.id == P_NEXT && next_adds(ed_ref(a)) && add_pad_allowed(ed_ref(a))
        && (ev.key_mods & ADD_MODS != 0 || in_sample_ms(ev).wrapping_sub(ed_ref(a).panel_down_k) >= HOLD_MS) {
        // The toolbar's + held (the tick may not have seen it yet), or clicked with a modifier key held: the
        // count pad, as the Pages view's New page.
        add_pad_open(a);
    } else if let Some(tool) = qs_target(ed_ref(a), h).filter(|_| {
        qs_allowed(ed_ref(a)) && !p.moved && !touch_cancelled(ev) && in_sample_ms(ev).wrapping_sub(ed_ref(a).panel_down_k) >= HOLD_MS
    }) {
        // A pen button held (the tick may not have seen it yet): its pen picked and the sizes card up, and the
        // lift is never also the tap.
        qs_hold(a, h, tool, true);
    } else {
        panel_act(a, h);
    }
}

// Whether the panel's next-page button is its + now: on the last page, where a tap adds a page and goes to it.
fn next_adds(s: &EditorState) -> bool {
    s.index >= s.nb.npages() - 1
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
    s.zoom_max = 4.0;
    s.rest_ox = 0.0;
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
    canvas_free(&mut s.base);
    s.base_page = 0;
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
    if s.map.open {
        mapview_close(&mut s.map);
    }
    keyboard_close(&mut s.map_kb);
    card_drop(s);
    bar_drop(s);
    find_drop(s);
    s.qs = QuickSize::default();
    s.paper_open = false;
    s.dm.open = false;
    s.mb.open = false;
    s.tools_open = false;
    s.pin_mode = false;
    tplsheet_close(&mut s.ts);
}

// On a landscape full-page view the margins are useful space for the panel. Keep the writer's
// chosen side, and move only horizontally; custom page sizes with no room retain the old placement.
fn landscape_panel_slot(a: &App, left: bool) -> Option<i32> {
    if !a.settings.landscape_fit || a.w <= a.h {return None;}
    let s=ed_ref(a);
    let width=if s.collapsed {box_()}else{pw()};
    let gap=ui_px(24.0);
    let page_left=(-s.ox*s.zoom).floor() as i32;
    let page_right=((s.pgw as f32-s.ox)*s.zoom).ceil() as i32;
    let x=if left {page_left-gap-width}else{page_right+gap};
    (x>=0 && x+width<=a.w).then_some(x)
}

pub fn editor_enter(a: &mut App) {
    fs_abort_view(a, FsAbortView::Screen);
    let s = ed(a);
    if s.page_canvas.px.is_empty() && canvas_init(&mut s.page_canvas, s.pgw, s.pgh) != 0 {
        return;
    }
    if s.open && s.page.is_some() && s.canvas_page != s.open_page_ref() {
        render_page_full(a);
    }
    let s = ed_ref(a);
    let panel_left = if s.view_w > 0 && s.panel.w > 0 {
        s.panel.x + s.panel.w/2 < s.view_w/2
    } else {a.settings.panel_x >= 0 && a.settings.panel_x < a.w/2};
    let turned = s.view_w != a.w || s.view_h != a.h;
    if turned {
        view_reset(a); // new notebook or turned
        let s = ed(a);
        card_drop(s); // (silently: the screen is composed whole below)
        if s.lm.state != LmState::Off && !s.destination.open && !(s.pv.open && s.pv.picking) {
            lm_drop(s); // (silently: the screen is composed whole below; a picking Pages view stays)
        }
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
    if turned || (a.relayout && a.w>a.h && a.settings.landscape_fit) {
        if let Some(beside_page)=landscape_panel_slot(a,panel_left) {x=beside_page;}
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
    // (the quick sizes card was placed for the screen it was opened on: a turn or a wake closes it, silently, in
    // this redraw)
    ed(a).qs = QuickSize::default();
    compose(a, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    );
    touch_context(a);
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

// Two fingers: zoom around the midpoint and pan with it. A legacy pinch (the runner's `pinch`, value 0)
// decides itself: nothing until the fingers clearly spread or travel, then both. A session touchgest
// classified (value 1) says what it is: ZOOM (1, no dead zone: it committed already), PAN (2, the page
// moves with the midpoint, the zoom held) or SWIPE (3, nothing moves: the gesture comes at the lift).
fn pinch(a: &mut App, ev: &InEvent) {
    let mx = (ev.x + ev.x2) * 0.5;
    let my = (ev.y + ev.y2) * 0.5;
    let d = (ev.x2 - ev.x).hypot(ev.y2 - ev.y);
    let s = ed(a);
    if ev.type_ == InType::PinchBegin {
        if s.owner == Owner::Link {
            s.owner = Owner::None; // the first finger turns into the pinch
        }
        if s.owner == Owner::Find && !s.owner_is_pen {
            // The first finger was on a row of the Find sheet (a keyboard key typed at its down): it is let go.
            s.owner = Owner::None;
            let r = with_find(a, |f, a, _nb, _nav| findsheet_cancel_press(f, a));
            app_invalidate(a, r, RefreshMode::Fast);
        }
        let s = ed(a);
        if (s.owner == Owner::Bar || s.owner == Owner::Outside) && !s.owner_is_pen {
            // The first finger was on the bar, or outside it waiting to stay: a button it pressed is let go
            // (a Fast refresh) and nothing is decided.
            s.owner = Owner::None;
            let r = with_lb(a, |lb, a, _nb, _nav| linkbar_cancel_press(lb, a));
            app_invalidate(a, r, RefreshMode::Fast);
        }
        let s = ed(a);
        if s.owner == Owner::Card && !s.owner_is_pen {
            // The first finger was on a row of the card (a pressed row is let go) or Rename's keyboard.
            s.owner = Owner::None;
            if !s.card.kb.open {
                let r = with_card(a, |c, a, _nb, _nav| linkcard_cancel_press(c, a));
                app_invalidate(a, r, RefreshMode::Fast);
            }
        }
        let s = ed(a);
        if s.owner == Owner::LinkTo && !s.owner_is_pen {
            // The first finger was on Link to... (a chip it pressed is let go) or New page...'s keyboard.
            s.owner = Owner::None;
            if s.lm.state != LmState::Naming {
                let r = with_lt(a, |lt, a, _nb, _nav| linkbar_cancel_press(lt, a));
                app_invalidate(a, r, RefreshMode::Fast);
            }
        }
        let s = ed(a);
        if s.owner == Owner::Panel && !s.owner_is_pen {
            // The first finger had pressed a panel button: that press is cancelled.
            if s.has_pressed && s.pressed.id != P_GRIP && s.pressed.id != P_EXPAND {
                let r = s.pressed.r;
                canvas_invert(&mut a.screen, r);
                app_invalidate(a, r, RefreshMode::Fast);
            }
            let s = ed(a);
            s.has_pressed = false;
            s.dragging = false;
            s.owner = Owner::None;
        }
        let pinch_zoom = a.settings.pinch_zoom;
        let s = ed(a);
        // A classified session starts with pinch zoom off too: its pan pans (the gate below is the
        // legacy one's, with the zoom setting).
        let classified = ev.value == 1;
        // (The bar does not pan or zoom the page behind it.)
        if s.owner != Owner::None || s.paper_open || s.mb.open || s.ts.open || s.lb.open || s.find.open || s.lt.open || s.card.open || s.qs.open || (!classified && !pinch_zoom) {
            return;
        }
        let regrab = s.pinching;
        s.pinching = true;
        s.pinch_committed = regrab && s.pinch_committed;
        s.pinch_classified = classified;
        s.pinch_mx0 = mx;
        s.pinch_my0 = my;
        s.pinch_d0 = if d > 10.0 { d } else { 10.0 };
        s.pinch_z0 = s.zoom;
        let (mut px, mut py) = (0.0, 0.0);
        to_page(s, mx, my, &mut px, &mut py);
        s.pinch_px = px;
        s.pinch_py = py;
        if !regrab {
            s.last_pinch_paint = 0;
        }
        return;
    }
    if !s.pinching {
        return;
    }
    if ev.type_ == InType::PinchMove {
        if s.pinch_classified {
            if ev.value != 1 && ev.value != 2 {
                return; // a swipe candidate moves nothing
            }
            if !s.pinch_committed {
                // touchgest committed at this move: anchor here, so the view does not jump.
                s.pinch_committed = true;
                s.pinch_d0 = if d > 10.0 { d } else { 10.0 };
                s.pinch_z0 = s.zoom;
                let (mut px, mut py) = (0.0, 0.0);
                to_page(s, mx, my, &mut px, &mut py);
                s.pinch_px = px;
                s.pinch_py = py;
            }
            if ev.value == 1 {
                s.zoom = s.pinch_z0 * d / s.pinch_d0;
                s.zoom = clampf(s.zoom, s.zoom_min, s.zoom_max);
            } else {
                s.zoom = s.pinch_z0; // a pan holds the zoom: no drift
            }
        } else {
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
            s.zoom = clampf(s.zoom, s.zoom_min, s.zoom_max);
        }
        s.ox = s.pinch_px - mx / s.zoom; // keep the pinched spot under the fingers
        s.oy = s.pinch_py - my / s.zoom;
        view_clamp(a);
        let s = ed(a);
        if ev.t_ms.wrapping_sub(s.last_pinch_paint) > 180 {
            s.last_pinch_paint = ev.t_ms;
            compose_all(a, RefreshMode::Fast);
        fs_pending_cleanup(a, RefreshMode::Fast);
        }
        return;
    }
    s.pinching = false; // IN_PINCH_END
    // Settle without a flash, and only if the view actually changed.
    if s.pinch_committed {
        compose_all(a, RefreshMode::Ui);
    }
    fs_pending_cleanup(a, RefreshMode::Ui);
}

pub fn editor_leave(a: &mut App) {
    fs_abort_view(a, FsAbortView::Screen);
    // Leaving for pen settings / calibration keeps the notebook open; just persist.
    if ed(a).owner == Owner::Draw {
        draw_end(a);
    }
    if ed(a).owner == Owner::Erase {
        erase_end(a);
    }
    // (They go with the screen: Pen feel or calibration comes up whole. A turn or the redraw after waking lays
    // this screen out again as it is, and they stay.)
    if !a.relayout {
        pb_reset(ed(a));
        card_drop(ed(a));
        bar_drop(ed(a));
        find_drop(ed(a));
        lm_drop(ed(a)); // (a link being made is not made, silently)
    }
    save_page(a);
    links_flush(a);
    write_thumb_if_needed(a);
    // Out of the editor no page pans or zooms, and no gesture is its: only the master switch is told.
    input_touch_context(TgView { gestures_on: a.settings.gestures, ..TgView::default() });
}

/// flush pending page/info saves; false if they failed
pub fn editor_suspend(a: &mut App) -> bool {
    a.pen_action_down = false;
    a.pen_toast_pending = None;
    match a.ed.as_deref() {
        Some(s) if s.open => {}
        _ => return true,
    }
    fs_abort_view(a, FsAbortView::Screen);
    // Native unlock will consume the pen/finger releases. Commit live ink before yielding input.
    if ed(a).owner == Owner::Draw {
        draw_end(a);
    }
    if ed(a).owner == Owner::Erase {
        erase_end(a);
    }
    if ed(a).owner == Owner::Select {
        // (a drag of the selection ends as a cancel, as owner_cancel ends it: its lift is lost, and waking
        // to a box with no bar while the bar's buttons still answered where it was is how a tap on nothing
        // deleted it; a lasso being drawn is dropped)
        sel_cancel(a);
    }
    let s = ed(a);
    card_drop(s); // (the Kindle sleeps: it is not there on waking)
    bar_drop(s);
    find_drop(s);
    if s.lm.state != LmState::Off && !s.destination.open && !(s.pv.open && s.pv.picking) {
        lm_drop(s); // (a lasso being drawn ends with it, Link to... is not there on waking; a picking Pages view stays)
    }
    s.owner = Owner::None;
    s.has_pressed = false;
    s.dragging = false;
    s.pinching = false;
    pb_reset(s);
    s.hold_eraser = false; // (its release may never come: the Kindle sleeps, native PIN input takes the keys)
    let ok = save_page(a);
    links_flush(a); // (a failure keeps the areas for later: the page's save says whether to stay)
    ok
}

/// Whether the notebook's page itself is in front: a notebook is open and no sheet, menu, Pages view, Links bar
/// or keyboard is over it. The only place the clock and battery shows (status.rs), and where a tap in its
/// corner is its.
///
/// The card, Link to... and Find PICK (phase 2) are sheets for every other purpose, but they leave the page's corner in
/// front: the clock shows and keeps its tap with them up (rule 12), unless a keyboard covers the corner.
pub fn editor_page_in_front(a: &App) -> bool {
    !crate::library::library_settings_active(a) && a.ed.as_deref().is_some_and(|s| s.open && !corner_sheet_open(s) && !keyboard_over_corner(a, s))
}

/// The sheets that take the corner with them: every one sheet_open names but the card, Link to... and Find PICK.
fn corner_sheet_open(s: &EditorState) -> bool {
    s.destination.open
        || s.map.open
        || s.pv.open
        || s.tools_open
        || s.mb.open
        || s.dm.open
        || s.paper_open
        || s.ts.open
        || s.lb.open
        || (s.find.open && s.find.mode != FindMode::Pick)
}

/// Whether a keyboard the card, New page... or Find PICK opened, or the toolbar's count pad, lies over the clock
/// corner's square (one docked at the bottom, with the corner at the bottom).
fn keyboard_over_corner(a: &App, s: &EditorState) -> bool {
    let sq = status_square(a);
    [&s.card.kb, &s.lm.kb, &s.find.kb, &s.add_kb].iter().any(|kb| kb.open && rect_overlaps(kb.area, sq))
}

/// Whether (x, y) is on a surface of the card, Link to..., Find PICK or the strip: a control, never the corner's.
fn phase2_surface_at(s: &EditorState, x: i32, y: i32) -> bool {
    let kb_at = |kb: &Keyboard| kb.open && rect_contains(kb.area, x, y);
    (s.card.open && rect_contains(s.card.rect, x, y))
        || (s.qs.open && rect_contains(s.qs.rect, x, y))
        || (s.lt.open && !s.lt.hidden && rect_contains(s.lt.rect, x, y))
        || (s.find.open && rect_contains(s.find.rect, x, y))
        || ((s.lm.state == LmState::Armed || s.lm.state == LmState::Lasso) && rect_contains(s.lm.strip, x, y))
        || kb_at(&s.card.kb)
        || kb_at(&s.lm.kb)
        || kb_at(&s.find.kb)
        || kb_at(&s.add_kb)
}

/// Whether the editor has a control at (x, y) for a finger on the page: the panel, or a link there. (A sheet, a
/// menu or the Pages view that is up takes every tap: the clock and battery is not shown then, and asks no
/// more.) The clock and battery corner leaves a tap on a control to it, but for a link in its own corner,
/// which is not one there: the corner's tap is the clock's, over a link too.
pub fn editor_control_at(a: &App, x: f32, y: f32) -> bool {
    if crate::library::library_settings_active(a) {return true;}
    let s = ed_ref(a);
    if !s.open || corner_sheet_open(s) {
        return true;
    }
    if rect_contains(s.panel, x as i32, y as i32) || phase2_surface_at(s, x as i32, y as i32) {
        return true;
    }
    if status_hit(a, x, y) {
        return false;
    }
    let mut l = NavLink::default();
    link_under(a, x, y, &mut l) || doc_link_under(a, x, y).is_some()
}

/// Whether the finger lift being handled was a tap in the corner, which the clock took (status.rs
/// decides at the lift, before the screen sees it, and toggles itself): a real lift, not the cancel up (the
/// off-screen point that ends a finger a gesture or the pen took). Whatever tap a finger on the page would
/// make, the corner's comes first and nothing else happens.
fn corner_tap(cancelled: bool) -> bool {
    !cancelled && status_tapped()
}

const FS_OUT_NONE: i32 = 0;
const FS_OUT_FIRED: i32 = 1;
const FS_OUT_REJECT: i32 = 2;
const FS_OUT_REFUSED: i32 = 3;
const FS_OUT_CANCEL: i32 = 4;
const FS_TRAIL_MS: u32 = 40;
const FS_REJECT_MS: u32 = 250;

fn fs_time(ev: &InEvent) -> u32 {
    if ev.k_ms != 0 { ev.k_ms } else { ev.t_ms }
}

/// Capture is armed only on the page (or in the Links bar's page area), never over a sheet,
/// menu, keyboard, card or the Pages view. The clock corner stays part of the page surface.
fn fs_can_arm(a: &App, in_links_bar: bool) -> bool {
    let Some(s) = a.ed.as_deref() else { return false };
    if !s.open || a.scr != ScreenId::Editor || !a.settings.gestures {
        return false;
    }
    if gesturemap_shapes(&a.gestures).is_empty() && !gesturemap_has_onefinger(&a.gestures) {
        return false;
    }
    if s.pv.open || s.tools_open || s.mb.open || s.dm.open || s.paper_open || s.ts.open || s.find.open || s.lt.open || s.card.open || s.qs.open
        || s.lm.state != LmState::Off || s.pv.kb.open || s.find.kb.open || s.lm.kb.open || s.card.kb.open || s.add_kb.open
    {
        return false;
    }
    !s.lb.open || in_links_bar
}

fn fs_reset_capture(s: &mut EditorState) {
    s.fs = FsStroke::default();
    s.fs_drawn = 0;
    s.fs_phase = 0;
    s.fs_last_frame = 0;
    s.fs_thinned = false;
}

fn fs_begin_event(a: &mut App, ev: &InEvent, in_links_bar: bool) {
    if !fs_can_arm(a, in_links_bar) {
        return;
    }
    // A new attempt replaces any trail whose fade was interrupted by a fresh touch.
    if !rect_is_empty(ed_ref(a).fs_trail) {
        let r = ed_ref(a).fs_trail;
        compose(a, r);
        app_invalidate(a, r, RefreshMode::Ui);
        app_flush(a);
        ed(a).fs_trail = RECT_EMPTY;
        ed(a).fs_reject_pending = false;
        ed(a).fs_tip_pending = false;
    }
    let px100 = (a.dpi / 25.4 * 100.0).round() as i32;
    let t = fs_time(ev);
    let s = ed(a);
    fs_begin(&mut s.fs, ev.x.round_ties_even() as i32, ev.y.round_ties_even() as i32, t, px100);
    s.fs_drawn = 0;
    s.fs_phase = 0;
    s.fs_trail = RECT_EMPTY;
    s.fs_last_frame = 0;
    s.fs_thinned = false;
    s.fs_cancel_waiting = false;
}

fn fs_point_rect(points: &[(f32, f32)], dpi_pad: i32, a: &App) -> Rect {
    if points.is_empty() {
        return RECT_EMPTY;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (points[0].0, points[0].1, points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    rect_intersect(
        Rect {
            x: x0.floor() as i32 - dpi_pad,
            y: y0.floor() as i32 - dpi_pad,
            w: (x1.ceil() as i32 - x0.floor() as i32 + 1 + 2 * dpi_pad).max(0),
            h: (y1.ceil() as i32 - y0.floor() as i32 + 1 + 2 * dpi_pad).max(0),
        },
        Rect { x: 0, y: 0, w: a.w, h: a.h },
    )
}

/// Draw the newly captured points only into a.screen. The page canvas, live ink, stroke and saved
/// page are never involved. A bounded stack buffer bridges FsPt to the canvas polyline API.
fn fs_draw_frame(a: &mut App, t: u32) {
    let s = ed_ref(a);
    let interval = if display_last_fast_waiting() { FS_TRAIL_MS * 2 } else { FS_TRAIL_MS };
    if s.fs.state != FsState::Drawing || (s.fs_drawn != 0 && t.wrapping_sub(s.fs_last_frame) < interval) {
        return;
    }
    let n = fs_points(&s.fs).len();
    if n == 0 || n == s.fs_drawn {
        return;
    }
    let reset = n < s.fs_drawn;
    let old = s.fs_trail;
    if reset && !rect_is_empty(old) {
        compose(a, old);
        let s = ed(a);
        s.fs_trail = RECT_EMPTY;
        s.fs_drawn = 0;
        s.fs_phase = 0;
    }
    let (drawn, phase) = (ed_ref(a).fs_drawn, ed_ref(a).fs_phase);
    let from = if drawn > 0 { drawn.saturating_sub(1) } else { 0 };
    if from >= n {
        return;
    }
    let points = fs_points(&ed_ref(a).fs);
    let mut xy = [(0.0f32, 0.0f32); FS_RAW_MAX];
    let count = points.len() - from;
    for i in 0..count {
        xy[i] = (points[from + i].x as f32, points[from + i].y as f32);
    }
    let radius = ui_px(3.5).max(1);
    let pitch = ui_px(20.0).max(1);
    let halo = ui_px(2.0).max(1);
    let r = fs_point_rect(&xy[..count], radius + halo + 1, a);
    let ph = canvas_dot_poly(&mut a.screen, &xy[..count], radius, pitch, halo, phase);
    let s = ed(a);
    s.fs_trail = rect_union(s.fs_trail, r);
    s.fs_drawn = n;
    s.fs_phase = ph;
    s.fs_last_frame = t;
    app_invalidate(a, rect_union(if reset { old } else { RECT_EMPTY }, r), RefreshMode::Fast);
    app_flush(a);
}

/// Recompose the path's rect and redraw every other dot, then hold it for the 250 ms reject fade.
fn fs_thin_trail(a: &mut App) {
    let r = ed_ref(a).fs_trail;
    if rect_is_empty(r) || ed_ref(a).fs_thinned {
        return;
    }
    compose(a, r);
    let points = fs_points(&ed_ref(a).fs);
    let mut xy = [(0.0f32, 0.0f32); FS_RAW_MAX];
    for (i, p) in points.iter().enumerate() {
        xy[i] = (p.x as f32, p.y as f32);
    }
    let radius = ui_px(3.5).max(1);
    let pitch = (ui_px(20.0) * 2).max(2);
    let halo = ui_px(2.0).max(1);
    let n = points.len();
    canvas_dot_poly(&mut a.screen, &xy[..n], radius, pitch, halo, 0);
    ed(a).fs_thinned = true;
    app_invalidate(a, r, RefreshMode::Fast);
    app_flush(a);
}

fn fs_reason_name(r: FsReason) -> &'static str {
    match r {
        FsReason::None => "None",
        FsReason::Frames => "Frames",
        FsReason::TooSmall => "TooSmall",
        FsReason::TooSlow => "TooSlow",
        FsReason::LateStart => "LateStart",
        FsReason::Paused => "Paused",
        FsReason::Straight => "Straight",
        FsReason::Tangled => "Tangled",
        FsReason::Broad => "Broad",
        FsReason::Held => "Held",
        FsReason::Cancelled => "Cancelled",
        FsReason::NoMatch => "NoMatch",
        FsReason::Unclear => "Unclear",
        FsReason::Structure => "Structure",
        FsReason::Inconsistent => "Inconsistent",
        FsReason::SameAs => "SameAs",
        FsReason::TooClose => "TooClose",
        FsReason::Slip => "Slip",
        FsReason::Full => "Full",
        FsReason::TwoStrokes => "TwoStrokes",
    }
}

fn fs_set_probe(a: &mut App, kind: i32, action: i32, why: FsReason, d1: i32, thr: i32, d2: i32) {
    let s = ed(a);
    s.fs_last_kind = kind;
    s.fs_last_action = action;
    s.fs_last_reason = why;
    s.fs_last_d1 = d1;
    s.fs_last_thr = thr;
    s.fs_last_d2 = d2;
    if kind == FS_OUT_FIRED {
        s.fs_fired = s.fs_fired.saturating_add(1);
    }
}

fn fs_trace(_a: &App, outcome: &str, name: &str, d1: i32, thr: i32, d2: i32) {
    let line = if outcome == "reject" {
        cformat!("shape reject %s", name)
    } else {
        cformat!("shape %s %s %d %d %d", outcome, name, d1, thr, d2)
    };
    input_touch_trace_event(&line);
}

/// A shape reject's sparse Fast trail is allowed to sit until this deadline. The one-time tip is
/// shown only after the rect has had its non-flashing Ui cleanup.
fn fs_reject(a: &mut App, t: u32, why: FsReason, match_: Option<FsMatch>) {
    let (d1, thr, d2) = match_ .map_or((0, 0, 0), |m| (m.d1, m.thr, m.d2));
    let kind = if why == FsReason::NoMatch || why == FsReason::Unclear || why == FsReason::Structure { FS_OUT_REJECT } else { FS_OUT_REFUSED };
    fs_set_probe(a, kind, 0, why, d1, thr, d2);
    fs_trace(a, "reject", fs_reason_name(why), d1, thr, d2);
    if !rect_is_empty(ed_ref(a).fs_trail) {
        fs_thin_trail(a);
        let tip = a.settings.hints & HINT_FSHAPE == 0;
        let s = ed(a);
        s.fs_reject_pending = true;
        s.fs_reject_until = t.wrapping_add(FS_REJECT_MS);
        s.fs_tip_pending = why == FsReason::NoMatch && tip;
    } else {
        fs_reset_capture(ed(a));
    }
}

fn fs_fire_cleanup(a: &mut App, r: Rect) {
    if !rect_is_empty(r) {
        compose(a, r);
        app_invalidate(a, r, RefreshMode::Ui);
    }
}

fn fs_screen_to_page(a: &App, r: Rect) -> Rect {
    if rect_is_empty(r) || a.ed.is_none() {
        return RECT_EMPTY;
    }
    let s = ed_ref(a);
    let x0 = (r.x as f32 / s.zoom + s.ox).floor() as i32;
    let y0 = (r.y as f32 / s.zoom + s.oy).floor() as i32;
    let x1 = ((r.x + r.w) as f32 / s.zoom + s.ox).ceil() as i32;
    let y1 = ((r.y + r.h) as f32 / s.zoom + s.oy).ceil() as i32;
    rect_intersect(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh })
}

#[derive(Clone, Copy)]
enum FsAbortView {
    Pen,
    Pinch,
    Pending,
    Screen,
}

fn fs_abort_view(a: &mut App, view: FsAbortView) {
    let s = ed(a);
    if s.fs.state == FsState::Idle && rect_is_empty(s.fs_trail) && rect_is_empty(s.fs_pinch_trail) {
        return;
    }
    let r = rect_union(s.fs_trail, s.fs_pinch_trail);
    let active = s.fs.state != FsState::Idle;
    if active {
        fs_abort(&mut s.fs);
    }
    s.fs_cancel_waiting = false;
    s.fs_reject_pending = false;
    s.fs_tip_pending = false;
    if active {
        fs_set_probe(a, FS_OUT_CANCEL, 0, FsReason::Cancelled, 0, 0, 0);
    }
    match view {
        FsAbortView::Pen => {
            if !rect_is_empty(r) {
                compose(a, r);
                app_invalidate(a, r, RefreshMode::Fast);
                let page = fs_screen_to_page(a, r);
                let s = ed(a);
                s.settle = rect_union(s.settle, page);
                s.settle_at = now_ms().wrapping_add(SETTLE_NEAR_MS);
                app_flush(a);
            }
            ed(a).fs_trail = RECT_EMPTY;
            ed(a).fs_pinch_trail = RECT_EMPTY;
            fs_reset_capture(ed(a));
        }
        FsAbortView::Pinch => {
            let s = ed(a);
            s.fs_pinch_trail = rect_union(s.fs_pinch_trail, r);
            s.fs_trail = RECT_EMPTY;
            fs_reset_capture(s);
        }
        FsAbortView::Pending => {
            let s = ed(a);
            s.fs_pinch_trail = rect_union(s.fs_pinch_trail, r);
            s.fs_trail = RECT_EMPTY;
            s.fs_cancel_waiting = !rect_is_empty(r);
            fs_reset_capture(s);
        }
        FsAbortView::Screen => {
            let s = ed(a);
            s.fs_trail = RECT_EMPTY;
            s.fs_pinch_trail = RECT_EMPTY;
            s.fs_reject_pending = false;
            s.fs_tip_pending = false;
            fs_reset_capture(s);
        }
    }
    if active && matches!(view, FsAbortView::Pen | FsAbortView::Screen) && matches!(ed_ref(a).owner, Owner::Link | Owner::Outside) {
        ed(a).owner = Owner::None;
    }
}

fn fs_pending_cleanup(a: &mut App, mode: RefreshMode) {
    let r = ed_ref(a).fs_pinch_trail;
    if rect_is_empty(r) {
        return;
    }
    compose(a, r);
    app_invalidate(a, r, mode);
    ed(a).fs_pinch_trail = RECT_EMPTY;
    ed(a).fs_cancel_waiting = false;
    app_flush(a);
}

fn fs_fade_tick(a: &mut App, now: u32) {
    if !ed_ref(a).fs_reject_pending || (now.wrapping_sub(ed_ref(a).fs_reject_until) as i32) < 0 {
        return;
    }
    let (r, tip) = (ed_ref(a).fs_trail, ed_ref(a).fs_tip_pending);
    if !rect_is_empty(r) {
        compose(a, r);
        app_invalidate(a, r, RefreshMode::Ui);
    }
    let s = ed(a);
    s.fs_trail = RECT_EMPTY;
    s.fs_reject_pending = false;
    s.fs_tip_pending = false;
    fs_reset_capture(s);
    if tip && a.settings.hints & HINT_FSHAPE == 0 {
        a.settings.hints |= HINT_FSHAPE;
        settings_save(&a.settings);
        // TRANSLATORS: a one-time toast after a finger drawing does not match any recorded shape;
        // about 1700 px at 44 px bold or two lines. Settings > Gestures lists the recorded shapes.
        app_toast(a, tr("Unknown finger shape. See Settings > Gestures."), 4000);
    }
    app_flush(a);
}

fn fs_touch1_key(dir: FsSwipe) -> GestureKey {
    GestureKey::Touch {
        fingers: 1,
        kind: match dir {
            FsSwipe::Up => TouchKind::Up,
            FsSwipe::Down => TouchKind::Down,
            FsSwipe::Left => TouchKind::Left,
            FsSwipe::Right => TouchKind::Right,
        },
    }
}

fn fs_anchor(r: Rect) -> (f32, f32) {
    ((r.x as f32) + r.w as f32 * 0.5, (r.y as f32) + r.h as f32 * 0.5)
}

fn fs_shape_match(a: &App, sample: &FsSample) -> (FsMatch, i32) {
    let shapes = gesturemap_shapes(&a.gestures);
    let n = shapes.len().min(FS_SHAPES_MAX);
    let mut bank = [FsGesture::default(); FS_SHAPES_MAX];
    for i in 0..n {
        bank[i] = shapes[i].1;
    }
    let mut pump = work_pump;
    let m = fs_match(sample, &bank[..n], &mut pump);
    let action = m.fired.filter(|&i| i < n).map_or(0, |i| shapes[i].0);
    (m, action)
}

fn fs_action_allowed(a: &App, action: i32) -> bool {
    if action == SHORTCUT_NONE || action_sources(action) & SRC_SHAPE == 0 {
        return false;
    }
    let s = ed_ref(a);
    if s.lb.open {
        // In the Links bar a shape may cancel it, or switch it to another menu. A non-menu
        // action belongs to the page and the bar refuses it until the bar is dismissed.
        return menu_action(action);
    }
    true
}

fn fs_fire(a: &mut App, action: i32, anchor: (f32, f32), m: FsMatch, t: u32) -> bool {
    if !fs_action_allowed(a, action) {
        return false;
    }
    let trail = ed_ref(a).fs_trail;
    let toast_before = a.toast_until;
    let screen_before = a.scr;
    let pages_before = a.ed.as_deref().is_some_and(|s| s.pv.open);
    // Restoring before dispatch lets the action compose its new view and the trail cleanup in
    // the same pending refresh. If the action refuses, fs_reject paints its sparse trail again.
    let pending = (a.dirty, a.dirty_mode);
    fs_fire_cleanup(a, trail);
    // What the app has changed so far, and its settings: the action's name shows only if the action changes
    // either (2.0.0-beta.2, the owner: "Show the action's name only when the action actually did something").
    // Whatever an action does reaches one or the other (a page turned, ink drawn or taken back, a sheet or the
    // view changed, the panel drawn again, a setting), so Next page on the last page, Previous page on page 1,
    // Clear page on an empty page, and Undo or Redo with nothing to undo or redo fire with nothing said.
    let (changes, settings) = (a.changes, a.settings);
    let ran = editor_shortcut_at(a, action, anchor.0, anchor.1, ActionSrc::Shape);
    if !ran {
        (a.dirty, a.dirty_mode) = pending;
        return false;
    }
    fs_set_probe(a, FS_OUT_FIRED, action, FsReason::None, m.d1, m.thr, m.d2);
    let name = shortcut_action_name(action).unwrap_or("unknown");
    fs_trace(a, "fired", name, m.d1, m.thr, m.d2);
    let full_view = a.scr != screen_before || a.asleep || (!pages_before && a.ed.as_deref().is_some_and(|s| s.pv.open));
    let did = a.changes != changes || a.settings != settings;
    if did && !full_view && a.toast_until == toast_before {
        app_toast(a, action_label(action), 1200);
    }
    ed(a).fs_trail = RECT_EMPTY;
    ed(a).fs_reject_pending = false;
    ed(a).fs_tip_pending = false;
    fs_reset_capture(ed(a));
    app_flush(a);
    let _ = t;
    true
}

/// Finish the captured touch before the existing link tap path. True means the lift was a
/// shape/swipe attempt and is fully handled; false keeps today's tap and clock-corner behavior.
fn fs_finish_event(a: &mut App, ev: &InEvent) -> bool {
    if ed_ref(a).fs.state == FsState::Idle {
        return false;
    }
    let t = fs_time(ev);
    let was_dead = ed_ref(a).fs.state == FsState::Dead;
    let bounds = fs_box(&ed_ref(a).fs);
    let anchor = fs_anchor(bounds);
    let mut sample = FsSample::default();
    match fs_finish(&mut ed(a).fs, t, FsMode::Runtime, &mut sample) {
        FsLift::Nothing => {
            fs_reset_capture(ed(a));
            false
        }
        FsLift::Swipe(dir) => {
            let action = gesturemap_action(&a.gestures, fs_touch1_key(dir));
            if action == SHORTCUT_NONE {
                if !rect_is_empty(ed_ref(a).fs_trail) {
                    fs_reject(a, t, FsReason::NoMatch, None);
                    true
                } else {
                    fs_reset_capture(ed(a));
                    true
                }
            } else {
                let trail = ed_ref(a).fs_trail;
                let pending = (a.dirty, a.dirty_mode);
                fs_fire_cleanup(a, trail);
                let ran = editor_shortcut_at(a, action, anchor.0, anchor.1, ActionSrc::Touch);
                if ran {
                    fs_set_probe(a, FS_OUT_FIRED, action, FsReason::None, 0, 0, 0);
                    ed(a).fs_trail = RECT_EMPTY;
                    ed(a).fs_reject_pending = false;
                    ed(a).fs_tip_pending = false;
                    fs_reset_capture(ed(a));
                    fs_trace(a, "fired", shortcut_action_name(action).unwrap_or("unknown"), 0, 0, 0);
                    app_flush(a);
                } else if !rect_is_empty(ed_ref(a).fs_trail) {
                    (a.dirty, a.dirty_mode) = pending;
                    fs_reject(a, t, FsReason::NoMatch, None);
                } else {
                    (a.dirty, a.dirty_mode) = pending;
                    fs_reset_capture(ed(a));
                }
                true
            }
        }
        FsLift::Shape => {
            let (m, action) = fs_shape_match(a, &sample);
            if let Some(i) = m.fired {
                let action = gesturemap_shapes(&a.gestures).get(i).map_or(action, |p| p.0);
                if fs_fire(a, action, anchor, m, t) {
                    true
                } else {
                    fs_reject(a, t, FsReason::NoMatch, Some(m));
                    true
                }
            } else {
                fs_reject(a, t, m.why, Some(m));
                true
            }
        }
        FsLift::Refused(why) => {
            if !rect_is_empty(ed_ref(a).fs_trail) {
                fs_reject(a, t, why, None);
                true
            } else {
                fs_set_probe(a, FS_OUT_REFUSED, 0, why, 0, 0, 0);
                fs_trace(a, "reject", fs_reason_name(why), 0, 0, 0);
                fs_reset_capture(ed(a));
                was_dead
            }
        }
    }
}

pub fn editor_event(a: &mut App, ev: &InEvent) {
    if crate::library::library_settings_active(a) {
        crate::library::library_event(a,ev);touch_context(a);return;
    }
    if !pb_input(a, ev) { editor_event_in(a, ev); }
    lm_check_menus(a);
    touch_context(a);
}

// Two or three fingers made a gesture: what it does is gestures.ini's (the built-ins without one), but
// for the mirror rule. With a menu in front, a gesture bound to that menu's own action closes it, and so
// does the opposite gesture (two fingers down opened Pages, two fingers up close them: the same swipe
// the other way takes back what the first did). Any other gesture does its own action, which a menu in
// front refuses or swaps for its own (editor_shortcut_at).
/// Whether a gesture on the page (or Space, which does what two fingers up do) is refused now: no page open, or
/// a keyboard of the Pages view, Find, New page..., the long-press card or the count pad being typed on.
fn gestures_refused(s: &EditorState) -> bool {
    !s.open || s.pv.kb.open || s.find.open || s.lm.kb.open || s.card.kb.open || s.add_kb.open
}

fn editor_gesture(a: &mut App, ev: &InEvent) {
    let s = ed_ref(a);
    if gestures_refused(s) {
        return;
    }
    let Some(key) = gesturemap_key_of_id(ev.value) else { return ;
    };
    let front = front_menu(s);
    // With the bar up two fingers down always close it, back to where it began, whatever is bound (without this
    // 2-down would fall through to the Graph, which swaps the menu and keeps the page the bar was looking at).
    let action = if front == SHORTCUT_LINKS && key == (GestureKey::Touch { fingers: 2, kind: TouchKind::Down ,
            }) {
        SHORTCUT_LINKS
    } else if front != 0 {
        let m = &a.gestures;
        // (the Graph's gesture closes a picking Pages view too; a Pages gesture over the Index goes on to
        // the action, which closes the Index's Pages view or turns its Graph or List to Pages)
        let owned = |k: GestureKey| {
            let act = gesturemap_action(m, k);
            act == front || (front == SHORTCUT_PAGES && act == SHORTCUT_INDEX_MAP)
        };
        if owned(key) || owned(gesturemap_mirror(key)) {
            front
        } else {
            gesturemap_action(m, key)
        }
    } else {
        gesturemap_action(&a.gestures, key)
    };
    if action == 0 {
        return;
    }
    editor_shortcut_at(a, action, ev.x2, ev.y2, ActionSrc::Touch);
}

fn editor_event_in(a: &mut App, ev: &InEvent) {
    if ed_ref(a).destination.open {
        let ph=app_pointer_phase(ev);
        if ph>=0 {let r=with_modal(a,|s|&mut s.destination,|p,a|p.pointer(a,ph,ev));match r {
            NotebookPick::Cancel=>{ed(a).lt.hidden=false;compose_all(a,RefreshMode::Full);},
            NotebookPick::Place(id,at)=>{let pids=std::mem::take(&mut ed(a).copy_pids);pages_copy_to(a,&id,at,&pids);},
            NotebookPick::Notebook(_)=>compose_all(a,RefreshMode::Full),
            NotebookPick::Target(t)=>{ed(a).lt.hidden=false;let target=if let Some((id,pid))=nav_external_target(t.as_str()){if ed_ref(a).nb.id==id{format!("p{pid}")}else{t.as_str().to_string()}}else{t.as_str().to_string()};link_pick(a,&target);compose_all(a,RefreshMode::Full);},
            NotebookPick::None=>{}
        }}
        return;
    }
    if ed_ref(a).map.open {
        if matches!(ev.type_, InType::PinchBegin | InType::PinchMove | InType::PinchEnd) {
            ed(a).owner = Owner::None;
            ed(a).pinching = ev.type_ != InType::PinchEnd;
            if !ed_ref(a).map_kb.open {
                with_map(a, |map, a, _, _| mapview_pinch(map, a, ev));
            }
            return;
        }
        let phase = app_pointer_phase(ev);
        if phase >= 0 {
            if phase == 0 {
                ed(a).owner = Owner::Map;
                ed(a).owner_is_pen = matches!(ev.type_, InType::PenDown);
            }
            map_pointer(a, phase, ev);
            return;
        }
    }

    if status_reserved(a) > 0
        && ev.y < status_reserved(a) as f32
        && a.ed.as_deref().is_some_and(|s| s.open && !sheet_open(s))
        && app_pointer_phase(ev) == 0
        && !editor_busy(a)
    {
        return;
    }
    let s = ed(a);
    if !s.open {
        return;
    }
    if touch_cancelled(ev) {
        let view = if s.pen_near { FsAbortView::Pen } else { FsAbortView::Pending };
        fs_abort_view(a, view);
    } else if ev.type_ == InType::PenDown || ev.type_ == InType::PenHover {
        fs_abort_view(a, FsAbortView::Pen);
    } else if ev.type_ == InType::TouchMove
        && !s.owner_is_pen && matches!(s.owner, Owner::Link | Owner::Outside)
    {
        let t = fs_time(ev);
        let state = fs_add(&mut s.fs, ev.x.round_ties_even() as i32, ev.y.round_ties_even() as i32, t, ev.value & TOUCH_F_BROAD != 0);
        if state == FsState::Dead {
            fs_thin_trail(a);
            return;
        }
        fs_draw_frame(a, t);
    }
    let s = ed(a);
    if ev.type_ == InType::Gesture {
        editor_gesture(a, ev);
        return;
    }
    if ev.type_ == InType::PenLeave {
        s.pen_near = false;
        s.pen_left = ev.t_ms;
    } else if ev.type_ == InType::PenDown || ev.type_ == InType::PenMove || ev.type_ == InType::PenHover {
        s.pen_near = true;
    }
    if ev.type_ == InType::PinchBegin || ev.type_ == InType::PinchMove || ev.type_ == InType::PinchEnd {
        if ev.type_ == InType::PinchBegin {
            live_reset();
        fs_abort_view(a, FsAbortView::Pinch);
        }
        // (before pinch(): a session no pinch started for, the Pages view's or a refused one's, says it too)
        if ev.type_ == InType::PinchEnd && ev.value == 1 {
            // Pen proximity can refuse a gesture silently; no toast over the notebook.
        }
        if !ed(a).pv.open {
            pinch(a, ev);
        }
        if ev.type_ == InType::PinchEnd {
            fs_pending_cleanup(a, RefreshMode::Ui);
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
        // The pen wins over a finger on the page: its stroke starts, and the finger's session ends with
        // no follow. (A finger on anything else, or two fingers, still holds the gesture.)
        if pen && s.owner == Owner::Link {
            s.owner = Owner::None;
        }
        if s.owner != Owner::None && s.owner != Owner::PenDrag && pen == s.owner_is_pen {
            lift_lost(a, pen);
        }
        let s = ed(a);
        if s.owner != Owner::None || s.pinching {
            if s.diag.admit(now_ms()) {
                let (who, by) = (if pen { "pen" } else { "finger" }, if s.owner_is_pen { "pen" } else { "finger" });
                if s.owner == Owner::None {
                    qlog!("editor: a %s down ignored: a pinch is under way", who);
                } else {
                    qlog!("editor: a %s down ignored: the %s owns %s", who, by, owner_name(s.owner));
                }
            }
            return; // one gesture at a time
        }
        if s.lm.state == LmState::Lasso {
            // A lasso no pen draws any more (its pen was taken from it some other way): the strip again, so
            // this contact is never ink under the strip, nor the old lasso's dashes left up.
            if s.diag.admit(now_ms()) {
                qlog!("editor: a lasso was left with no pen drawing it: back to the strip");
            }
            lasso_cancel(a);
        }
        let s = ed(a);
        s.owner_is_pen = pen;
        // A contact made while the toolbar's batch was being written: let go, down to its lift. Then the
        // toolbar's count pad: a contact on it is its own; one outside closes it with nothing added (a finger
        // at the lift, a pen at its down, its stroke swallowed).
        let quiet = s.add_quiet_until.is_some_and(|until| until.wrapping_sub(ev.t_ms) as i32 >= 0);
        s.add_quiet_until = None;
        if quiet {
            s.owner = Owner::AddPad;
            return;
        }
        if s.add_kb.open {
            if rect_contains(s.add_kb.area, ev.x as i32, ev.y as i32) {
                s.owner = Owner::AddPad;
                add_pad_pointer(a, 0, ev);
            } else {
                s.owner = Owner::Outside;
                if pen {
                    add_pad_cancel(a);
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                }
            }
            return;
        }
        // The Find sheet: its rows and keyboard take the contact, a contact outside it is swallowed (a finger
        // decides at the lift, a pen at its down) and closes it.
        if s.find.open {
            if rect_contains(s.find.kb.area, ev.x as i32, ev.y as i32) || rect_contains(s.find.rect, ev.x as i32, ev.y as i32) {
                s.owner = Owner::Find;
                let mut d = RECT_EMPTY;
                let r = with_find(a, |f, a, nb, nav| {
                    findsheet_pointer(f, a, nb, nav, 0, ev, &mut d)});
                find_result(a, r, d);
            } else {
                s.owner = Owner::Outside;
                if pen {
                    find_close(a);
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                }
            }
            return;
        }
        // The long-press card (8.5): its rows (or Rename's keyboard) take a contact on them; one outside closes
        // the card (Rename's keyboard: cancels the rename) and is swallowed, a finger deciding at the lift (the
        // clock's corner keeps its tap), a pen at its down.
        if s.card.open {
            let on_kb = s.card.kb.open && rect_contains(s.card.kb.area, ev.x as i32, ev.y as i32);
            if on_kb {
                s.owner = Owner::Card;
                card_kb_pointer(a, 0, ev);
            } else if !s.card.kb.open && rect_contains(s.card.rect, ev.x as i32, ev.y as i32) {
                s.owner = Owner::Card;
                let mut d = RECT_EMPTY;
                with_card(a, |c, a, _nb, _nav| linkcard_pointer(c, a, 0, ev, &mut d));
            } else {
                s.owner = Owner::Outside;
                if pen && s.card.kb.open {
                    rename_back(a);
                } else if pen {
                    card_close(a);
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                }
            }
            return;
        }
        // The quick sizes card: a contact on it is its own; one outside it (the toolbar included) closes it with
        // nothing else done, a finger at the lift (the clock's corner keeps its tap), a pen at its down, its
        // stroke swallowed.
        if s.qs.open {
            if rect_contains(s.qs.rect, ev.x as i32, ev.y as i32) {
                s.owner = Owner::QuickSize;
                let mut d = RECT_EMPTY;
                let r = with_qs(a, |q, a| quicksize_pointer(q, a, 0, ev, &mut d));
                qs_result(a, r, d);
            } else {
                s.owner = Owner::Outside;
                if pen {
                    qs_close(a);
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                }
            }
            return;
        }
        // New page...'s keyboard takes a contact on it; one outside it goes back to Link to... (a finger at the
        // lift, a pen at its down, its stroke swallowed).
        if s.lm.state == LmState::Naming && s.lm.kb.open {
            if rect_contains(s.lm.kb.area, ev.x as i32, ev.y as i32) {
                s.owner = Owner::LinkTo;
                naming_pointer(a, 0, ev);
            } else {
                s.owner = Owner::Outside;
                if pen {
                    naming_back(a);
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                }
            }
            return;
        }
        // Link to... takes every contact: on it (its chips and tool row), or outside it, which makes no link (a
        // finger at the lift, the clock's corner excepted; a pen at its down, its stroke swallowed).
        if s.lt.open && !s.lt.hidden {
            if rect_contains(s.lt.rect, ev.x as i32, ev.y as i32) {
                s.owner = Owner::LinkTo;
                let mut d = RECT_EMPTY;
                with_lt(a, |lt, a, _nb, _nav| linkbar_pointer(lt, a, 0, ev, &mut d));
            } else {
                s.owner = Owner::Outside;
                if pen {
                    link_cancel(a);
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                }
            }
            return;
        }
        // The Links bar takes every contact first: on it (its buttons, chips, grip and header), or outside it,
        // which stays or closes at the lift (a pen outside acts at once and its stroke is swallowed).
        if s.lb.open {
            if rect_contains(s.lb.rect, ev.x as i32, ev.y as i32) {
                s.owner = Owner::Bar;
                s.lb.down_at = now_ms(); // (a hold on a chip opens the long-press card: editor_tick)
                let mut d = RECT_EMPTY;
                with_lb(a, |lb, a, _nb, _nav| linkbar_pointer(lb, a, 0, ev, &mut d));
            } else {
                s.owner = Owner::Outside;
                if pen {
                    bar_stay(a); // (the page above stays, or the bar just closes; the stroke is swallowed)
                } else {
                    press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
                fs_begin_event(a, ev, true);
                }
            }
            return;
        }
        if s.pv.open {
            s.owner = Owner::Pages;
            pages_event(a, 0, ev);
            return;
        }
        if s.tools_open {
            s.owner = Owner::Tools;
            s.tools_slider = false;
            press_begin(&mut s.tools_press, ev.x, ev.y, ev.t_ms);
            if let Some(h) = hits_find(&s.tools_hits, ev.x, ev.y).copied() {
                if shape_slider_id(h.id) {
                    // A slider answers the first contact (no pressed look: its knob goes there).
                    shape_slider_follow(a, h, ev.x);
                } else {
                    app_press_feedback(a, h.r);
                    ed(a).tools_pressed = Some(h.r);
                }
            }
            return;
        }
        if s.mb.open {
            s.owner = Owner::Build;
            let mut d = RECT_EMPTY;
            with_modal(a, |s| &mut s.mb, |mb, a| musicbuild_pointer(mb, a, 0, ev.x, ev.y, ev.t_ms, &mut d),
            );
            return;
        }
        if s.dm.open {
            s.owner = Owner::Display;
            let mut d = RECT_EMPTY;
            with_modal(a, |s| &mut s.dm, |dm, a| dispmenu_pointer(dm, a, 0, ev.x, ev.y, ev.t_ms, &mut d),
            );
            return;
        }
        if s.ts.open {
            s.owner = Owner::Templates;
            let mut d = RECT_EMPTY;
            with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_pointer(ts, a, 0, ev.x, ev.y, ev.t_ms, &mut d),
            );
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
        } else if (s.lm.state == LmState::Armed || s.lm.state == LmState::Lasso) && rect_contains(s.lm.strip, ev.x as i32, ev.y as i32) {
            // The strip takes a finger or the pen: its Cancel, or nothing (never the lasso).
            s.owner = Owner::Strip;
            strip_pointer(a, 0, ev);
        } else if rect_contains(s.panel, ev.x as i32, ev.y as i32) {
            s.owner = Owner::Panel;
            // A stroke just ended: Undo/Clear may have become available, but the panel (and its
            // hit list) is only rebuilt at the settle. Settle now so the tap finds the live buttons.
            if s.panel_stale {
                settle_now(a);
            }
            ui_pointer(a, 0, ev, false);
        } else if s.lm.state == LmState::Off && sel_takes(s, pen, ev.x, ev.y) {
            // Select mode: the bar, a drag of the selection, or the pen's lasso (never ink).
            sel_down(a, ev, pen);
        } else if pen && s.lm.state == LmState::Armed {
            // While a link is made the next pen contact on the page is its lasso, whatever the tool (the
            // eraser end, the side button and Erase while held too): never ink.
            lasso_begin(a, ev);
        } else if pen {
            draw_begin(a, ev);
            if ed(a).owner == Owner::Draw {
                draw_sample(a, ev);
            }
        } else {
            // A finger on the page: a tap follows a link there, a hold opens it, and a hold anywhere else
            // arms the finger lasso (editor_tick). Outside a selection it ends the selection first, and then
            // only its hold does anything.
            let quiet = sel_finger_outside(a);
            let s = ed(a);
            s.owner = Owner::Link;
            press_begin(&mut s.link_press, ev.x, ev.y, ev.t_ms);
            s.link_down_at = now_ms();
            s.link_held = false;
            s.link_quiet = quiet;
            let pen_recent = s.pen_left != 0 && ev.t_ms.wrapping_sub(s.pen_left) < FHOLD_PEN_MS;
            s.link_spoiled = s.pen_near || pen_recent || ev.value & TOUCH_F_BROAD != 0;
            if !quiet {
                fs_begin_event(a, ev, false);
            }
        }
        return;
    }
    if s.owner == Owner::None || pen != s.owner_is_pen {
        return;
    }
    owner_pointer(a, ph, ev, pen);
}

/// A down from the input that already holds a gesture (a pen down while the pen draws, a finger down while a
/// finger holds a button): that gesture's lift never came (lost in a full input queue, or a drop). It ends
/// as a cancel (owner_cancel), the log says so, and the new contact starts as any other. (Before, the lost
/// lift swallowed the next contact of that input: its down ignored, one gesture at a time, and its moves
/// and lift going on with the old gesture.)
fn lift_lost(a: &mut App, pen: bool) {
    let s = ed(a);
    if s.diag.admit(now_ms()) {
        let who = if pen { "pen" } else { "finger" };
        qlog!("editor: a %s down while the %s owned %s: its lift was lost", who, who, owner_name(s.owner));
    }
    owner_cancel(a);
}

/// The gesture the owner holds ends as a cancel: on a control, a sheet or a keyboard as the off-screen cancel
/// ends a finger's there (nothing it pressed acts), ink and the eraser kept as far as they went, a lasso taken
/// back to the strip (lasso_cancel). For a contact whose lift was lost (lift_lost), and one the pen's button
/// takes over for a drag (pb_out).
fn owner_cancel(a: &mut App) {
    let s = ed_ref(a);
    let (owner, pen, near) = (s.owner, s.owner_is_pen, s.pen_near);
    let cancel = InEvent { type_: InType::TouchUp, x: -10000.0, y: -10000.0, value: TOUCH_F_CANCEL, ..Default::default() };
    match owner {
        Owner::None | Owner::PenDrag => {}
        Owner::Draw => draw_end(a),
        Owner::Erase => erase_end(a),
        Owner::Lasso => lasso_cancel(a),
        Owner::Select => sel_cancel(a),
        Owner::Map if !ed_ref(a).map.open => {}
        _ => {
            if !pen {
                // (what editor_event_in does first with the cancel: a finger shape being drawn is dropped)
                fs_abort_view(a, if near { FsAbortView::Pen } else { FsAbortView::Pending });
            }
            owner_pointer(a, 2, &cancel, pen);
        }
    }
    ed(a).owner = Owner::None;
}

/// The rest of a contact (ph 1 a move, 2 its lift or the off-screen cancel) goes to what its down began: the
/// owner, from the input (pen) that owns it.
fn owner_pointer(a: &mut App, ph: i32, ev: &InEvent, pen: bool) {
    let s = ed(a);
    match s.owner {
        Owner::PenDrag => {}
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
        Owner::Lasso => {
            if ph == 1 {
                lasso_move(a, ev);
            } else {
                lasso_end(a);
            }
        }
        Owner::Strip => strip_pointer(a, ph, ev),
        Owner::AddPad => {
            if ed_ref(a).add_kb.open {
                add_pad_pointer(a, ph, ev);
            }
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
        }
        Owner::Select => sel_pointer(a, ph, ev),
        Owner::Map => map_pointer(a, ph, ev),
        Owner::QuickSize => {
            let mut d = RECT_EMPTY;
            let r = with_qs(a, |q, a| quicksize_pointer(q, a, ph, ev, &mut d));
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
            qs_result(a, r, d);
        }
        Owner::Card => {
            if ed_ref(a).card.kb.open {
                card_kb_pointer(a, ph, ev);
                if ph == 2 {
                    ed(a).owner = Owner::None;
                }
                return;
            }
            let mut d = RECT_EMPTY;
            let r = with_card(a, |c, a, _nb, _nav| linkcard_pointer(c, a, ph, ev, &mut d));
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
            if touch_cancelled(ev) {
                app_invalidate(a, d, RefreshMode::Fast);
            } else {
                card_result(a, r, d);
            }
        }
        Owner::LinkTo => {
            if ed_ref(a).lm.state == LmState::Naming {
                naming_pointer(a, ph, ev);
                if ph == 2 {
                    ed(a).owner = Owner::None;
                }
                return;
            }
            let mut d = RECT_EMPTY;
            let r = with_lt(a, |lt, a, _nb, _nav| linkbar_pointer(lt, a, ph, ev, &mut d));
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
            if touch_cancelled(ev) {
                // (a second finger took the touch over: the chip is let go as it was pressed, and nothing is done)
                app_invalidate(a, d, RefreshMode::Fast);
            } else {
                lt_result(a, r, d);
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
            if ev.value & TOUCH_F_BROAD != 0 {
                s.link_spoiled = true; // (palm-sized now: its hold arms nothing)
            }
            if ph != 2 {
                return;
            }
            s.owner = Owner::None;
            // Decided at the lift, in this order: a cancel (the touch became a gesture, or the pen came)
            // ends silently; the corner's tap is the clock's alone; an unheld tap that lifts within
            // TAP_SLOP_MM of where it landed (as the corner's) follows the link.
            let lp = s.link_press;
            let (x0, y0) = (lp.x0, lp.y0);
            let slop = mm_to_px(TAP_SLOP_MM, a.dpi);
            let rolled = (lp.x - x0).abs() > slop || (lp.y - y0).abs() > slop;
            if touch_cancelled(ev) || fs_finish_event(a, ev) || ed_ref(a).link_quiet || corner_tap(false) {
                return;
            }
            if !ed_ref(a).link_held && !rolled {
                link_follow(a, x0, y0);
            }
        }
        Owner::Find => {
            let mut d = RECT_EMPTY;
            let r = with_find(a, |f, a, nb, nav| {
                findsheet_pointer(f, a, nb, nav, ph, ev, &mut d)});
            if ph == 2 {
                ed(a).owner = Owner::None;
            }
            find_result(a, r, d);
        }
        Owner::Bar => {
            let mut d = RECT_EMPTY;
            let r = with_lb(a, |lb, a, _nb, _nav| linkbar_pointer(lb, a, ph, ev, &mut d));
            if ph == 2 || r == LbResult::Cancel {
                ed(a).owner = if ph == 2 { Owner::None } else { Owner::Modal }; // (a drag that closed it: the rest is swallowed)
            }
            if touch_cancelled(ev) {
                // A button the first finger pressed is let go with a Fast refresh as it was pressed: a second
                // finger took the touch over (the cancel does no tap, so there is no result to act on).
                app_invalidate(a, d, RefreshMode::Fast);
                bar_result(a, r, RECT_EMPTY);
            } else {
                bar_result(a, r, d);
            }
        }
        Owner::Outside => {
            // A pen's stroke here is swallowed to its end; a finger decides at the lift.
            if pen {
                if ph == 2 {
                    s.owner = Owner::None;
                }
                return;
            }
            press_move(&mut s.link_press, ev.x, ev.y);
            if ph != 2 {
                return;
            }
            s.owner = Owner::None;
            let (x0, y0, moved) = (s.link_press.x0, s.link_press.y0, s.link_press.moved);
            // In this order: a cancel (the touch became a gesture, or the pen came) does nothing; a lift on the
            // panel's Links button is the button again, which cancels; any other tap outside stays or closes,
            // and presses nothing. (The clock's corner is not shown with the bar or Find up: no tap is its.)
            if touch_cancelled(ev) || fs_finish_event(a, ev) || moved {
                return;
            }
            // The card closes (Rename's keyboard: the rename is cancelled, the card stays); the clock's corner
            // keeps its tap, and the card stays for it (rule 12).
            if ed_ref(a).card.open {
                if !corner_tap(false) {
                    if ed_ref(a).card.kb.open {
                        rename_back(a);
                    } else {
                        card_close(a);
                    }
                }
                return;
            }
            // The quick sizes card closes with nothing picked (the clock's corner keeps its tap).
            if ed_ref(a).qs.open {
                if !corner_tap(false) {
                    qs_close(a);
                }
                return;
            }
            // The toolbar's count pad goes with nothing added (the clock's corner keeps its tap).
            if ed_ref(a).add_kb.open {
                if !corner_tap(false) {
                    add_pad_cancel(a);
                }
                return;
            }
            // New page...'s keyboard and Link to... leave the clock's corner its tap (rule 12).
            if ed_ref(a).lm.state == LmState::Naming && ed_ref(a).lm.kb.open {
                if !corner_tap(false) {
                    naming_back(a);
                }
                return;
            }
            if ed_ref(a).lt.open && !ed_ref(a).lt.hidden {
                if !corner_tap(false) {
                    link_cancel(a);
                }
                return;
            }
            if ed_ref(a).find.open {
                // (a tap outside the Find sheet closes it, and is swallowed; in pick mode the corner keeps its tap)
                if !(ed_ref(a).find.mode == FindMode::Pick && corner_tap(false)) {
                    find_close(a);
                }
                return;
            }
            if !ed_ref(a).lb.open {
                return;
            }
            let links_button = ed_ref(a).hits.h.iter().any(|h| h.id == P_LINKS && rect_contains(h.r, x0 as i32, y0 as i32));
            if links_button {
                bar_stay(a);
                link_arm(a,false,None);
            } else {
                bar_stay(a);
            }
        }
        Owner::Tools => {
            press_move(&mut s.tools_press, ev.x, ev.y);
            let slider = hits_find(&s.tools_hits, s.tools_press.x0, s.tools_press.y0).filter(|h| shape_slider_id(h.id)).copied();
            if let Some(h) = slider {
                // A Smart shapes slider follows the press to its lift, where the value is saved; a
                // cancelled touch (the pen came, a second finger) keeps the value it had come to.
                if !touch_cancelled(ev) {
                    shape_slider_follow(a, h, ev.x);
                }
                if ph == 2 {
                    let s = ed(a);
                    s.owner = Owner::None;
                    s.tools_pressed = None;
                    // (a press that stayed on the name changed nothing, and gets no refresh)
                    if std::mem::take(&mut s.tools_slider) {
                        settings_save(&a.settings);
                        shape_slider_draw(&mut a.screen, h.id, h.r, &a.settings);
                        app_invalidate(a, h.r, RefreshMode::Ui);
                    }
                }
                return;
            }
            if ph != 2 {
                return;
            }
            s.owner = Owner::None;
            let mut pressed = s.tools_pressed.take();
            let back = app_press_release(a, &mut pressed);
            if !rect_is_empty(back) {
                app_invalidate(a, back, RefreshMode::Ui);
            }
            // A cancelled touch presses nothing and closes nothing: the palm a pen comes down beside
            // is cancelled when it does, and the pen's own contact, next, finds the menu still open.
            if touch_cancelled(ev) {
                return;
            }
            tools_tap(a);
        }
        Owner::Build => {
            let mut d = RECT_EMPTY;
            let r = with_modal(a, |s| &mut s.mb, |mb, a| musicbuild_pointer(mb, a, ph, ev.x, ev.y, ev.t_ms, &mut d),
            );
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
            let r = with_modal(a, |s| &mut s.ts, |ts, a| tplsheet_pointer(ts, a, ph, ev.x, ev.y, ev.t_ms, &mut d),
            );
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
            let r = if touch_cancelled(ev) {
                with_modal(a, |s| &mut s.dm, |dm, a| dispmenu_cancel(dm, a, &mut d))
            } else {
                with_modal(a, |s| &mut s.dm, |dm, a| dispmenu_pointer(dm, a, ph, ev.x, ev.y, ev.t_ms, &mut d))
            };
            if r == DmResult::Inverted {
                compose_all(a, RefreshMode::Full);
            } else if r == DmResult::Refit {
                editor_fit_changed(a, true);
                // The card was redrawn with its row flipped: presented, upright (sideways the Full of the
                // view laid out again takes it in). No toast (the owner, 2026-11-04); one left up goes.
                if !rect_is_empty(d) {
                    app_invalidate(a, d, RefreshMode::Ui);
                }
                app_toast_dismiss(a);
            } else if r == DmResult::Closed {
                compose(a, d);
                app_invalidate(a, d, RefreshMode::Ui);
            } else if !rect_is_empty(d) {
                app_invalidate(a, d, if ph == 2 { RefreshMode::Ui } else { RefreshMode::Fast },
                );
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

// DU's traces (display.rs, DuWear): past a quarter of the screen, or owed by a toolbar drag, one Clean
// refresh of where they are, at a pause as the settle's (the pen gone SETTLE_LEFT_MS, or nothing added and
// no stroke ended for SETTLE_NEAR_MS with it near), never under a stroke, a gesture or the Pages view. With a settle due
// in the same tick both go out as one refresh.
fn du_wear_tick(a: &mut App, now: u32) {
    // (never with input still queued: the pen coming near cancels a toolbar drag, and its hover may still be
    // waiting behind the cancel)
    if !du_wear_due(&a.du_wear, a.w, a.h) || a.pen_action_down || !a.input_drained {
        return;
    }
    let s = ed_ref(a);
    // (with the pen near: nothing added, and no stroke ended, for SETTLE_NEAR_MS; with it gone, SETTLE_LEFT_MS
    // since then and since the last was added, which gives a pen that is arriving the time to say so)
    let since_added = now.wrapping_sub(a.du_wear.at) as i32;
    let paused = (since_added >= SETTLE_NEAR_MS as i32 && (now.wrapping_sub(s.settle_at) as i32) >= 0)
        || (!s.pen_near && (now.wrapping_sub(s.pen_left) as i32) >= SETTLE_LEFT_MS && since_added >= SETTLE_LEFT_MS);
    if !s.open || s.owner != Owner::None || s.pinching || s.pv.open || s.map.open || s.destination.open || !paused {
        return;
    }
    // (on the screen as it is now: a flush clipped to it would never cover a rect reaching past it)
    let r = rect_intersect(a.du_wear.rect, Rect { x: 0, y: 0, w: a.w, h: a.h });
    if rect_is_empty(r) {
        a.du_wear = DuWear::default();
        return;
    }
    a.du_wear.rect = r;
    qlog!("display: cleaning fast refreshes' traces at %d,%d %dx%d (%lld px worn%s)", r.x, r.y, r.w, r.h, a.du_wear.area,
        if a.du_wear.owed { ", owed" } else { "" });
    compose(a, r);
    app_invalidate(a, r, RefreshMode::Clean);
}

// The deferred settle: one GL16 refresh for everything written since the last pause. It is no change of the
// app's (App.changes): what it draws is on the panel already, from the writing's fast refreshes, and an action
// that settles first and then finds nothing to do did nothing (Undo with nothing to undo, fs_fire).
fn settle_now(a: &mut App) {
    let changes = a.changes;
    settle_now_in(a);
    a.changes = changes;
}
fn settle_now_in(a: &mut App) {
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

// One idle tick's migration (PHASE2 5.3): the first page not tried with a link that has no area.
fn migrate_step(a: &mut App) {
    let s = ed(a);
    let i = s.migrate_next();
    if i < 0 {
        return;
    }
    let pid = s.nb.page_ids[i as usize];
    if let Some(e) = s.hist.iter().position(|h| h.page_id == pid && h.page.is_some()) {
        s.migrate_entry(e);
        return;
    }
    let mut p = None;
    if store_load_page_pumped(&s.nb, i, s.pgw, s.pgh, &mut p) == 0 {
        let s = ed(a);
        if let Some(p) = p.as_deref() {
            if !p.readonly && nav_link_migrate(&mut s.nav, p, pid) > 0 {
                s.links_migrated();
            }
        }
    }
    ed(a).migrate_tried(pid); // (a page that does not read now is not tried again this session)
}

pub fn editor_tick(a: &mut App, now: u32) {
    if crate::library::library_settings_active(a) {crate::library::library_tick(a,now);touch_context(a);return;}
    if !a.asleep && !ed_ref(a).pv.open && !ed_ref(a).destination.open && !editor_busy(a)
        && now.wrapping_sub(a.last_input)>1000 && now.wrapping_sub(ed_ref(a).previews_checked)>1000 {
        ed(a).previews_checked=now;
        let (current,dpi)=(ed_ref(a).index,a.dpi);
        with_pv(a,|pv,_,nb,_|pagesview_prefetch(pv,nb,current,dpi));
    }
    if ed_ref(a).pv.open||ed_ref(a).destination.open {
        // A batch for the notebook shown being made: its previews are looked for often, and each tile is drawn as
        // its preview lands (only the tiles that changed: previews_repaint).
        let pending=previews_shown(ed_ref(a)).is_some_and(|(id,_)|bg_pending(BgKind::PagePreviews,id.as_str()));
        // The last of them gone: the tiles still waiting are drawn again, even when no child's end said so (a
        // batch dropped unstarted: a fork that failed, a full queue), so no loading mark stays.
        let ended=std::mem::replace(&mut ed(a).previews_pending,pending) && !pending;
        if ended || now.wrapping_sub(ed_ref(a).previews_checked)>=if pending {PREVIEWS_POLL_MS} else {PREVIEWS_IDLE_POLL_MS} {
            ed(a).previews_checked=now;
            previews_repaint(a,ended);
        }
    }
    if ed_ref(a).destination.open {return;}
    if ed_ref(a).map.open {
        if ed_ref(a).map_kb.open {
            let mut kb = std::mem::take(&mut ed(a).map_kb);
            app_keyboard_tick(a, &mut kb, now);
            ed(a).map_kb = kb;
        } else {
            with_map(a, |map, a, nb, nav| {
                mapview_tick(map, a, now);
                // Other notebooks' links here found since it opened: its anchors made again (no dot moves).
                mapview_anchors_refresh(map, nb, nav);
                // The Graph of all notebooks: what the library index learned since, its new nodes placed.
                mapview_library_refresh(map, a, nb, nav, now);
            });
            if ed_ref(a).map.needs_redraw {
                compose_all(a, RefreshMode::Ui);
            }
        }
    }
    shape_hold_tick(a, now);
    link_shape_tick(a, now);
    if ed_ref(a).open && a.input_drained {
        let state = fs_tick(&mut ed(a).fs, now);
        if state == FsState::Dead {
            fs_thin_trail(a);
        } else {
            fs_draw_frame(a, now);
        }
        fs_fade_tick(a, now);
        if ed_ref(a).fs_cancel_waiting && !ed_ref(a).pinching {
            fs_pending_cleanup(a, RefreshMode::Ui);
        }
    }
    let s = ed(a);
    // The pen gone for SETTLE_LEFT_MS (not just lifted high for a moment), or near for too long.
    let stopped = (now.wrapping_sub(s.settle_at) as i32) >= 0 || (!s.pen_near && (now.wrapping_sub(s.pen_left) as i32) >= SETTLE_LEFT_MS);
    if s.open && s.owner == Owner::None && (!rect_is_empty(s.settle) || s.panel_stale) && stopped {
        settle_now(a);
    }
    du_wear_tick(a, now);
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
        with_pv(a, |pv, a, nb, nav| pagesview_tick(pv, a, nb, nav, now));
    }
    let s = ed(a);
    if s.open && s.owner == Owner::Link && !s.link_held && !s.link_press.moved && s.fs.state != FsState::Drawing
        && s.fs.state != FsState::Dead
        && now.wrapping_sub(s.link_down_at) >= HOLD_MS {
        fs_kill(&mut ed(a).fs, FsReason::Held);
        // On a link, its card; anywhere else on the page the finger lasso arms (owner ruling 2026-11-02):
        // drawn on it selects, lifted where it is it pastes (select.rs)
        // (a hold the pen or a palm spoiled arms nothing: it is only held)
        let (x0, y0) = (ed(a).link_press.x0, ed(a).link_press.y0);
        let mut l = NavLink::default();
        let spoiled = ed_ref(a).link_spoiled || ed_ref(a).pen_near;
        if link_under(a, x0, y0, &mut l) || spoiled || !sel_finger_arm(a, x0, y0) {
            card_open_link(a);
        }
    }
    // A pen held still in Select mode: a paste there (select.rs).
    if ed(a).open {
        sel_tick(a, now);
    }
    // A press held on a chip of the Links bar (a finger or the pen): the long-press card for its entry (8.10).
    let s = ed(a);
    if s.open && s.owner == Owner::Bar && !s.card.open && now.wrapping_sub(s.lb.down_at) >= HOLD_MS {
        if let Some((i, chip)) = linkbar_pressed_chip(&s.lb) {
            bar_hold(a, i, chip);
        }
    }
    // The toolbar's + held still (HOLD_MS): its press let go, and the count pad.
    let s = ed(a);
    if s.open && s.owner == Owner::Panel && s.has_pressed && s.pressed.id == P_NEXT && !s.press.moved
        && next_adds(s) && add_pad_allowed(s) && now.wrapping_sub(s.press.t0) >= HOLD_MS {
        let r = s.pressed.r;
        s.has_pressed = false;
        canvas_invert(&mut a.screen, r);
        app_invalidate(a, r, RefreshMode::Ui);
        add_pad_open(a);
    }
    // A pen button held still (HOLD_MS), by a finger or the nib: its pen is the pen in use, and the sizes card opens
    // beside it; the contact stays the card's (it may slide to a chip).
    let s = ed(a);
    if s.open && s.owner == Owner::Panel && s.has_pressed && !s.press.moved && now.wrapping_sub(s.press.t0) >= HOLD_MS {
        let h = s.pressed;
        if let Some(tool) = qs_target(s, h).filter(|_| qs_allowed(s)) {
            qs_hold(a, h, tool, false);
        }
    }
    // The sizes card's save, a pass of the loop after its close was presented.
    let drained = a.input_drained;
    let s = ed(a);
    if s.open && s.qs_save != 0 && s.owner == Owner::None {
        if s.qs_save == 1 {
            s.qs_save = 2;
        } else if drained {
            s.qs_save = 0;
            settings_save(&a.settings);
        }
    }
    // Typing on the toolbar's count pad pauses: its grayscale refresh.
    let s = ed(a);
    if s.open && s.add_kb.open {
        let mut kb = std::mem::take(&mut s.add_kb);
        app_keyboard_tick(a, &mut kb, now);
        ed(a).add_kb = kb;
    }
    // Typing on New page...'s keyboard pauses: its grayscale refresh.
    let s = ed(a);
    if s.open && s.lm.state == LmState::Naming && s.lm.kb.open {
        let mut kb = std::mem::take(&mut s.lm.kb);
        app_keyboard_tick(a, &mut kb, now);
        ed(a).lm.kb = kb;
    }
    // Typing on the Find sheet pauses: its keyboard's grayscale refresh, the card of rows with it. Or delete held
    // down deletes again: the rows the text finds now, after the text line's refresh, as for a key.
    let s = ed(a);
    if s.open && s.find.open && with_find(a, |f, a, nb, nav| findsheet_tick(f, a, nb, nav, now)) == FsResult::Changed {
        find_result(a, FsResult::Changed, RECT_EMPTY);
    }
    // Rename's keyboard on a link's card (its typing through the keyboard host, as every keyboard's): a held
    // delete's repeats, and its grayscale refresh once typing pauses.
    let s = ed(a);
    if s.open && s.card.open && s.card.kb.open {
        let mut kb = std::mem::take(&mut s.card.kb);
        app_keyboard_tick(a, &mut kb, now);
        ed(a).card.kb = kb;
    }
    // The Links bar gets the pages it goes to ready while nothing is touching: a page or a band a tick.
    let drained = a.input_drained;
    let s = ed(a);
    if s.open && s.lb.open && s.owner == Owner::None && drained {
        prefetch_step(a);
    }
    if ed_ref(a).open && a.input_drained {
        let config = a.pb_config;
        for _ in 0..3 {
            let out = penbutton_tick(&mut ed(a).pb, &config, now);
            if out == PbOut::None { break; }
            pb_out(a, out, now);
        }
    }
    // Autosave after a real pause (and never while the pen is down). One that fails (a full store)
    // tries again 2 s later, not at every tick: each try says so with a toast.
    let s = ed(a);
    if s.open && (s.page_dirty || caches_due(s, now))
        && s.owner == Owner::None && now.wrapping_sub(s.last_up) > 2000 && !autosave(a) {
        ed(a).last_up = now;
    }
    // beta.5's links on pages not visited yet: one page a tick, only while nothing is touching or open
    // over the page. A page not in the session cache is read for it and dropped (memory stays flat),
    // with the pens pumped as it is read. Each page is tried once, whatever happens.
    let s = ed(a);
    if s.open && drained && s.owner == Owner::None && !s.pinching && !sheet_open(s) && s.lm.state == LmState::Off && s.nav.links_writable && s.migrate_room() {
        migrate_step(a);
    }
    // The backlinks cache (other notebooks' links here): a few stamps a tick, at most one links.ini read, only
    // while nothing touches or is open over the page. A bar up when what it knows changed is built again.
    let s = ed(a);
    if s.open && drained && s.owner == Owner::None && !s.pinching && s.lm.state == LmState::Off && !s.find.open {
        // (one cache's work a tick: the Graph of all notebooks' index waits for backlinks.ini's check)
        if !backlinks_step() {
            libindex_step();
        }
    }
    let s = ed_ref(a);
    let gen = backlinks_generation();
    if s.open && s.lb.open && s.lb.mode == LbMode::List && !s.lb.hidden && !s.peek.active && s.owner == Owner::None
        && !s.card.open && s.lb.from_gen != gen {
        // (drawn again only when its rows changed: a check that found nothing new here redraws nothing)
        let rows = bar_from_rows(s);
        if rows == s.lb.from {
            ed(a).lb.from_gen = gen;
        } else {
            bar_rebuild(a);
        }
    }
    // Migrated areas are saved after a pause (and again 2 s after a failed save), apart from the
    // page's own save: a migrated page is not page_dirty.
    let s = ed(a);
    if s.open && s.links_dirty && s.owner == Owner::None && (now.wrapping_sub(s.links_try_at) as i32) >= 0 {
        links_flush(a);
    }
    // A page whose picture was made while a stroke or gesture was under way (editor_picture_made).
    let s = ed(a);
    if s.open && s.page.is_some() && s.canvas_page != s.open_page_ref() && !s.page_canvas.px.is_empty() && s.owner == Owner::None && !s.pinching {
        render_page_full(a);
        compose_all(a, RefreshMode::Ui);
    }
    lm_say(a);
    touch_context(a);
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

// For the probes editor.kept_matches_render, editor.preview_matches_render and editor.card_matches_render: the
// open page's kept drawing (.page-previews/pN.full) or its Pages preview (pN.pgm), when current for its file,
// against the open page drawn afresh (the preview as shrunk from it); or the Library card (thumb.pgm) against
// page 1 drawn afresh (the page in memory, else its file), shrunk as the editor shrinks its canvas for the card or
// as a closing notebook shrinks page 1's file. Some(true) the same, Some(false) not, None with no such cache.
// A cache is of the page file, the fresh drawing of the page in memory: asked with the page saved
// (editor.unsaved 0). Each draws a whole page.
fn cache_matches_render(s: &EditorState, key: &str, dpi: f32) -> Option<bool> {
    let fresh = |p: &Page| {
        let mut c = Canvas::default();
        (canvas_init(&mut c, p.w, p.h) == 0).then(|| {
            page_render(&mut c, p, Rect { x: 0, y: 0, w: p.w, h: p.h }, dpi);
            c
        })
    };
    let same = |x: &Canvas, y: &Canvas| x.w == y.w && x.h == y.h && x.px[..(x.w * x.h) as usize] == y.px[..(y.w * y.h) as usize];
    let pid = s.open_pid();
    match key {
        "kept_matches_render" => {
            let p = s.open_page()?;
            let mut kept = Canvas::default();
            if canvas_init(&mut kept, s.pgw, s.pgh) != 0 || !store_load_page_render(&s.nb, pid, s.pgw, s.pgh, &mut kept, page_layered(p)) {
                return None;
            }
            Some(same(&kept, &fresh(p)?))
        }
        "preview_matches_render" => {
            let p = s.open_page()?;
            let mut shown = Canvas::default();
            if !store_load_page_preview(&s.nb, pid, &mut shown) {
                return None;
            }
            let mut small = Canvas::default();
            Some(store_page_preview_image(&fresh(p)?, &mut small) && same(&shown, &small))
        }
        _ => {
            let mut card = Canvas::default();
            if store_load_thumb(s.nb.id.as_str(), &mut card) != 0 {
                return None;
            }
            let first = *s.nb.page_ids.first()?;
            let mut read = None;
            let p = match s.hist.iter().find(|e| e.page_id == first).and_then(|e| e.page.as_deref()) {
                Some(p) => p,
                None if store_read_page(&s.nb, 0, &mut read) == 0 => read.as_deref()?,
                None => return None,
            };
            let full = fresh(p)?;
            let (mut lines, mut plain) = (Canvas::default(), Canvas::default());
            if canvas_init(&mut lines, s.pgw / 3, s.pgh / 3) != 0 || canvas_init(&mut plain, s.pgw / 3, s.pgh / 3) != 0 {
                return None;
            }
            canvas_downsample_keep_lines(&mut lines, &full, 0.35); // (save_thumb_from_canvas)
            canvas_downsample(&mut plain, &full); // (close_notebook's page_render_thumb)
            Some(same(&card, &lines) || same(&card, &plain))
        }
    }
}

/// The headless runner's "editor.*" keys, asked without the prefix (app_probe's contract);
/// "templates.*" keys reach it whole (docs/SIM.md).
pub fn editor_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    if a.ed.is_some() {
        if let Some(v) = sel_probe(a, key) {
            csnprintf!(out, cap, "%s", v.as_str());
            return 0;
        }
    }
    if key=="pages.screen" {let pv=&ed_ref(a).pv;csnprintf!(out,cap,"%d",if !pv.open {0} else if pv.index_tab {pv.index_screen+1} else {pv.screen+1});return 0;}
    // Where `tap panel NAME` taps, "X Y" (empty for none): a script holds the toolbar's + there (editor.at.next).
    if let Some(name)=key.strip_prefix("at.") {
        out.clear();
        if let Some((x,y))=editor_panel_point(a,name) {csnprintf!(out,cap,"%d %d",x as i32,y as i32);}
        return 0;
    }
    // The toolbar's count pad: 1 while it is up; "addpad.done" its Done's label, "addpad.done.on" 1 when Done can
    // be pressed (a count of 1 or more); both empty while it is not up.
    if key=="addpad" {csnprintf!(out,cap,"%d",a.ed.as_deref().is_some_and(|s|s.add_kb.open) as i32);return 0;}
    if key=="addpad.done" || key=="addpad.done.on" {
        out.clear();
        if let Some(kb)=a.ed.as_deref().map(|s|&s.add_kb).filter(|kb|kb.open) {
            let n=keyboard_count(kb);
            if key=="addpad.done" {csnprintf!(out,cap,"%s",&add_label(n));} else {csnprintf!(out,cap,"%d",(n>=1) as i32);}
        }
        return 0;
    }
    if key=="pages.grid" {let (cols,rows)=crate::pagesview::pagesview_grid(&ed_ref(a).pv,a);csnprintf!(out,cap,"%d %d",cols,rows);return 0;}
    if let Some(k)=key.strip_prefix("pages.") {return with_pv(a,|pv,_,nb,nav|pagesview_probe(pv,nb,nav,k,out,cap));}
    if key=="screen_composed" {
        let before=a.screen.clone();compose(a,Rect::new(0,0,a.w,a.h));
        let same=before.px==a.screen.px;a.screen=before;
        csnprintf!(out,cap,"%d",same as i32);return 0;
    }
    if let Some(key)=key.strip_prefix("settings.") {return crate::library::library_probe(a,key,out,cap);}
    if let Some(probe) = key.strip_prefix("pen.") {
        let Some(s) = a.ed.as_deref() else {
            out.clear();
            if probe == "clicks" { out.push('0'); }
            return if probe == "gesture" || probe == "clicks" { 0 } else { -1 };
        };
        if probe == "gesture" {
            out.clear();
            if let Some((button, kind)) = s.pb_last {
                gesturemap_key_name(pb_key(button, kind), out);
                *out = out.trim_start_matches("pen.").to_string();
            }
            return 0;
        }
        return if penbutton_probe(&s.pb, probe, out) { 0 } else { -1 };
    }
    if let Some(probe) = key.strip_prefix("fshape.") {
        let Some(s) = a.ed.as_deref_mut() else { return -1 };
        match probe {
            "last" => {
                let kind = match s.fs_last_kind {
                    FS_OUT_FIRED => "fired",
                    FS_OUT_REJECT => "reject",
                    FS_OUT_REFUSED => "refused",
                    FS_OUT_CANCEL => "cancel",
                    FS_OUT_NONE => "",
                    _ => "",
                };
                if kind.is_empty() {
                    csnprintf!(out, cap, "%s", "");
                } else {
                    let name = if s.fs_last_kind == FS_OUT_FIRED {
                        shortcut_action_name(s.fs_last_action).unwrap_or("unknown")
                    } else {
                        fs_reason_name(s.fs_last_reason)
                    };
                    csnprintf!(out, cap, "%s %s %d %d %d", kind, name, s.fs_last_d1, s.fs_last_thr, s.fs_last_d2);
                }
            }
            "fired" => {
                csnprintf!(out, cap, "%u", s.fs_fired);
                s.fs_fired = 0;
            }
            "state" => {
                let name = match s.fs.state {
                    FsState::Idle => "idle",
                    FsState::Pending => "pending",
                    FsState::Drawing => "drawing",
                    FsState::Dead => "dead",
                };
                csnprintf!(out, cap, "%s", name);
            }
            "trail" => {
                let r = rect_union(s.fs_trail, s.fs_pinch_trail);
                if rect_is_empty(r) {
                    csnprintf!(out, cap, "%s", "");
                } else {
                    csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
                }
            }
            "armed" => { csnprintf!(out, cap, "%d", i32::from(fs_can_arm(a, true))); }
            _ => return -1,
        }
        return 0;
    }
    // panel_at.X.Y: the name of the panel's button at (X, Y), empty for none. A stale panel is settled
    // first, as a tap would (hence before s is borrowed).
    if let Some(xy) = key.strip_prefix("panel_at.") {
        let Some((xs, ys)) = xy.split_once('.') else { return -1 ;
        };
        let (x, y) = (atoi(xs), atoi(ys));
        if a.ed.is_none() {
            return -1;
        }
        let open = ed_ref(a).open && a.scr == ScreenId::Editor;
        if open && ed_ref(a).panel_stale {
            settle_now(a);
        }
        let s = ed_ref(a);
        let name = if open { panel_hit_at(s, x as f32, y as f32).as_ref().map(panel_name).unwrap_or_default() } else { String::new() };
        csnprintf!(out, cap, "%s", &name);
        return 0;
    }
    if key == "wear" {
        // DU's traces not cleaned yet (display.rs, DuWear): the area worn in thousandths of the screen,
        // with " owed" while a toolbar drag owes a clean; "0" once a clean or a Full wiped them.
        let w = a.du_wear;
        let screen = (a.w as i64 * a.h as i64).max(1);
        csnprintf!(out, cap, "%lld%s", w.area * 1000 / screen, if w.owed { " owed" } else { "" });
        return 0;
    }
    if key == "layers" {
        // Whether the screen is what its layers make of it: the page, the panel and then each menu that is up, in
        // their order. The screen is composed again whole and compared with what it held: "ok", or the first pixel
        // that differs, "X Y" (a panel drawn over a sheet, the Find sheet's keyboard or the Links bar shows here).
        // A pixel that differs by 64 at most is not one: a frame's slanted edge drawn again over itself is blended
        // with its own pixels (never by more than a quarter of the range), and a layer redrawn in place does that.
        // The screen is the fresh one after it, so a check says what was wrong once.
        if a.ed.is_none() || !(ed_ref(a).open && a.scr == ScreenId::Editor) {
            return -1;
        }
        let keep = a.screen.px.clone();
        compose(a, Rect { x: 0, y: 0, w: a.w, h: a.h ,
            },
        );
        match keep.iter().zip(a.screen.px.iter()).position(|(x, y)| x.abs_diff(*y) > 64) {
            None => csnprintf!(out, cap, "%s", "ok"),
            Some(i) => {
                let stride = a.screen.stride.max(1) as usize;
                csnprintf!(out, cap, "%d %d", (i % stride) as i32, (i / stride) as i32)
            }
        };
        return 0;
    }
    if key == "eye_drawn" {
        // How many link boxes the eye's last paint drew (not made again here: a stale paint shows).
        if a.ed.is_none() {
            return -1;
        }
        csnprintf!(out, cap, "%d", ed_ref(a).boxes.drawn as i32);
        return 0;
    }
    if key == "eye_docs" {
        // How many document-link boxes the eye's last paint drew.
        if a.ed.is_none() {
            return -1;
        }
        csnprintf!(out, cap, "%d", ed_ref(a).boxes.doc_drawn as i32);
        return 0;
    }
    if key == "doc_areas" {
        // How many document-link boxes the eye would draw now (0 while it is off).
        if a.ed.is_none() {
            return -1;
        }
        let n = if a.settings.link_areas && ed_ref(a).open && a.scr == ScreenId::Editor {
            boxes_get(a);
            ed_ref(a).boxes.doc.len() as i32
        } else {
            0
        };
        csnprintf!(out, cap, "%d", n);
        return 0;
    }
    if key == "areas" {
        // How many link boxes the eye draws now, 0 while it is off.
        if a.ed.is_none() {
            return -1;
        }
        let n = if a.settings.link_areas && ed_ref(a).open && a.scr == ScreenId::Editor {
            boxes_get(a);
            ed_ref(a).boxes.n as i32
        } else {
            0
        };
        csnprintf!(out, cap, "%d", n);
        return 0;
    }
    let dpi = a.dpi;
    let Some(s) = a.ed.as_deref() else { return -1 };
    let st = &a.settings;
    let has_page = s.open && s.page.is_some();
    if key=="notebook_id" {csnprintf!(out,cap,"%s",s.nb.id.as_str());}
    else if key=="destination" {csnprintf!(out,cap,"%d",s.destination.open as i32);}
    else if key=="destination.folder" {csnprintf!(out,cap,"%s",s.destination.folder.as_str());}
    else if key=="destination.notebook" {csnprintf!(out,cap,"%s",s.destination.selected.as_ref().map_or("",|b|b.id.as_str()));}
    else if key=="destination.cached" {csnprintf!(out,cap,"%d",s.destination.cached());}
    else if key=="destination.shows" {csnprintf!(out,cap,"%s",&s.destination.shows(a));}
    else if key=="destination.copying" {csnprintf!(out,cap,"%d",(s.destination.open&&s.destination.copying) as i32);}
    else if key=="page_ids" {
        // The open notebook's page order, its page ids comma-separated (Move to..., Copy to notebook...).
        let ids:Vec<String>=s.nb.page_ids.iter().map(|p|p.to_string()).collect();
        csnprintf!(out,cap,"%s",&ids.join(","));
    }
    else if key == "page" {
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
        // The strip is up: armed, or drawing the lasso.
        let on = s.open && (s.lm.state == LmState::Armed || s.lm.state == LmState::Lasso);
        csnprintf!(out, cap, "%d", if on { 1 } else { 0 });
    } else if key == "strip" {
        // The strip: 0, or new while it is up for a new link, redraw for the card's Redraw the area.
        let on = s.open && (s.lm.state == LmState::Armed || s.lm.state == LmState::Lasso);
        csnprintf!(out, cap, "%s", if !on { "0" } else if s.lm.redraw.is_some() { "redraw" } else { "new" });
    } else if key == "quicksize" {
        // The quick sizes card: the tool's name while it is up, 0 closed.
        if s.open && s.qs.open {
            csnprintf!(out, cap, "%s", app_probe_tool(s.qs.tool).unwrap_or("?"));
        } else {
            csnprintf!(out, cap, "%s", "0");
        }
    } else if let Some(k) = key.strip_prefix("quicksize.") {
        if !quicksize_probe(&s.qs, &a.settings, k, out, cap) {
            return -1;
        }
    } else if key == "link_card" {
        // The long-press card: 0, or 1 while it is up.
        csnprintf!(out, cap, "%d", if s.open && s.card.open { 1 } else { 0 });
    } else if let Some(k) = key.strip_prefix("link_card.") {
        if !linkcard_probe(&s.card, k, out, cap) {
            return -1;
        }
    } else if key == "lasso" {
        // How many points the lasso being drawn holds, 0 otherwise.
        csnprintf!(out, cap, "%d", if s.open && s.lm.state == LmState::Lasso { s.lm.pts.len() as i32 } else { 0 });
    } else if key == "link_area" {
        // The area a new link waits to be given a target for, x y w h page px, empty otherwise.
        if s.open && area_waits(s) {
            let r = s.lm.area;
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "map" || key.starts_with("map.") {
        let map_key = key.strip_prefix("map.").unwrap_or("");
        if !mapview_probe(&s.map, map_key, out) {
            return -1;
        }
    } else if key=="shape_snaps" {csnprintf!(out,cap,"%u",s.sh.kept);
    } else if key == "shape_preview" {
        let text=if s.sh.preview.is_some(){format!("{:?}",s.sh.last_kind).to_lowercase()}else{"0".to_string()};csnprintf!(out,cap,"%s",&text);
    } else if key == "stroke_shape" {
        let kind = format!("{:?}", s.sh.last_kind).to_lowercase();
        csnprintf!(out, cap, "%s", kind.as_str());
    } else if key == "undo_kind" || key == "redo_kind" {
        // The kind of step an undo (a redo) of the open page would apply, empty for none.
        let e = if has_page { s.page.and_then(|pid| s.hist.iter().position(|e| e.page_id == pid)) } else { None };
        let kind = e.and_then(|i| {
            if key == "undo_kind" { history_undo_kind(&s.hist[i].h) } else { history_redo_kind(&s.hist[i].h) }
        });
        let name = match kind {
            Some(ActKind::Add) => "add",
            Some(ActKind::Erase) => "erase",
            Some(ActKind::Paper) => "paper",
            Some(ActKind::Clear) => "clear",
            Some(ActKind::Link) => "link",
            Some(ActKind::Replace) => "replace",
            Some(ActKind::Lasso) => "lasso",
            None => "",
        };
        csnprintf!(out, cap, "%s", name);
    } else if let Some(xy) = key.strip_prefix("ui_at.") {
        // The name of what the card or link making shows at screen point (X, Y), empty for none: the card's
        // rows, Link to...'s chips and buttons, the strip's Cancel.
        let Some((xs, ys)) = xy.split_once('.') else { return -1 ;
        };
        let (x, y) = (atoi(xs), atoi(ys));
        let here = s.open && a.scr == ScreenId::Editor;
        let strip = here && (s.lm.state == LmState::Armed || s.lm.state == LmState::Lasso);
        let name = if here && s.card.open {
            linkcard_hit_name(&s.card, x, y)
        } else if here && s.lt.open {
            linkbar_hit_name(&s.lt, x, y)
        } else if strip && rect_contains(strip_cancel(s.lm.strip), x, y) {
            "strip.cancel".to_string()
        } else {
            String::new()
        };
        csnprintf!(out, cap, "%s", &name);
    } else if key == "linkto" {
        // Link to...: closed (0), or picking where a new link goes (up, or hidden under what it opened).
        csnprintf!(out, cap, "%s", if s.open && s.lt.open { "pick" } else { "0" });
    } else if let Some(k) = key.strip_prefix("linkto.") {
        if !linkbar_probe_pick(&s.lt, k, out, cap) {
            return -1;
        }
    } else if key == "recent" {
        // The Recent targets kept, as page numbers, newest first ("3 2"); empty for none.
        let mut v = String::new();
        for t in s.recent.iter().rev() {
            let p = if s.open { nav_link_page(&NavLink { target: *t, ..Default::default() }, &s.nb,
                ) } else { -1 };
            if p >= 0 {
                if !v.is_empty() {
                    v.push(' ');
                }
                v.push_str(&cformat!("%d", p + 1));
            }
        }
        csnprintf!(out, cap, "%s", &v);
    } else if key == "kb_dock" {
        // Where the keyboard in front is docked (Find, Rename, New page...): top or bottom; empty for none.
        let kb = if s.map_kb.open {
            Some(&s.map_kb)
        } else if s.find.open {
            Some(&s.find.kb)
        } else if s.card.kb.open {
            Some(&s.card.kb)
        } else if s.lm.kb.open {
            Some(&s.lm.kb)
        } else {
            None
        };
        csnprintf!(out, cap, "%s", kb.map_or("", |k| if k.dock_y > 0 { "top" } else { "bottom" }));
    } else if key == "links" {
        // The open page's links: how many it has, and how many of them have their box on it.
        let pid = if has_page { s.nb.page_ids[s.index as usize] } else { 0 };
        let boxed = if has_page {
            let mut links = [NavLink::default(); NAV_LINKS_PER_PAGE as usize];
            let mut areas = [RECT_EMPTY; NAV_LINKS_PER_PAGE as usize];
            nav_link_areas(&s.nav, s.open_page().unwrap(), pid, 1.0, &mut links, &mut areas,
            )
        } else {
            0
        };
        let mut first = 0;
        csnprintf!(out, cap, "%d %d", if has_page { nav_links_of(&s.nav, pid, &mut first) } else { 0 }, boxed);
    } else if let Some(n) = key.strip_prefix("link.").and_then(|n| n.strip_suffix(".area")) {
        // The open page's link N's area, x y w h in page px, empty for none (or no such link). Before
        // the link.N arm: its atoi would read "1.area" as 1.
        let mut first = 0;
        let n_links = if has_page { nav_links_of(&s.nav, s.nb.page_ids[s.index as usize], &mut first) } else { 0 };
        let i = atoi(n) - 1;
        match (i >= 0 && i < n_links).then(|| s.nav.links[(first + i) as usize]).filter(|l| l.has_area) {
            Some(l) => csnprintf!(out, cap, "%d %d %d %d", l.area.x, l.area.y, l.area.w, l.area.h),
            None => csnprintf!(out, cap, "%s", ""),
        };
    } else if key == "doclinks" {
        // The open page's document links, and all of the notebook's: `page notebook`.
        let pid = if has_page { s.nb.page_ids[s.index as usize] } else { 0 };
        csnprintf!(out, cap, "%d %d", if has_page { doclinks_of(&s.dl, pid).len() as i32 } else { 0 }, s.dl.links.len() as i32);
    } else if let Some(n) = key.strip_prefix("doclink.") {
        // The open page's document link N (from 1, annotation order): its target's page number from 1 and its
        // area, `p<number> x y w h`; empty past the last.
        let pid = if has_page { s.nb.page_ids[s.index as usize] } else { 0 };
        let mine = doclinks_of(&s.dl, pid);
        let i = atoi(n) - 1;
        match (i >= 0).then(|| mine.get(i as usize)).flatten() {
            Some(l) => {
                let r = l.area();
                let at = s.nb.page_ids.iter().position(|&p| p == l.target).map_or(0, |i| i + 1);
                csnprintf!(out, cap, "p%d %d %d %d %d", at as i32, r.x, r.y, r.w, r.h)
            }
            None => csnprintf!(out, cap, "%s", ""),
        };
    } else if key == "migrate_left" {
        // Pages of the open notebook with a link that has no area, not tried this session.
        let left = if s.open {
            s.nb.page_ids.iter().filter(|&&pid| !s.migrate_done.contains(&pid) && nav_links_legacy(&s.nav, pid) > 0).count()
        } else {
            0
        };
        csnprintf!(out, cap, "%d", left as i32);
    } else if key == "links_dirty" {
        csnprintf!(out, cap, "%d", if s.open && s.links_dirty { 1 } else { 0 });
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
    } else if key == "view" {
        // The view: its zoom (as editor.zoom) and the page point at the screen's top left, "1.00 0 0".
        csnprintf!(out, cap, "%.2f %.0f %.0f", s.zoom / s.zoom_default, s.ox, s.oy);
    } else if key == "notebook_back" {
        // How many notebooks Back returns through, and the newest one's id ("1 A"), "0" for none.
        match s.notebook_back.last() {
            Some(b) => csnprintf!(out, cap, "%d %s", s.notebook_back.len() as i32, b.id.as_str()),
            None => csnprintf!(out, cap, "%s", "0"),
        };
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
    } else if key == "back" {
        // The back list: its depth and its top as a page number ("2 3"), "0 0" when it is empty. (Pages
        // deleted since are not in it.)
        let ids: Vec<u32> = if s.open { (0..s.nb.npages().max(0) as usize).map(|i| s.nb.page_ids[i]).collect() } else { Vec::new() };
        let live: Vec<u32> = s.back.iter().copied().filter(|p| ids.contains(p)).collect();
        let top = live.last().and_then(|p| ids.iter().position(|q| q == p)).map_or(0, |i| i as i32 + 1);
        csnprintf!(out, cap, "%d %d", live.len() as i32, top);
    } else if key == "find" {
        // The Find sheet: closed (0), open (go: a page by its number or name), or picking where a new link goes (pick).
        csnprintf!(out, cap, "%s", if !s.find.open { "0" } else if s.find.mode == FindMode::Pick { "pick" } else { "go" });
    } else if let Some(k) = key.strip_prefix("find.") {
        if !findsheet_probe(&s.find, k, out, cap) {
            return -1;
        }
    } else if (key == "keyboard" || key == "text") && s.map_kb.open {
        csnprintf!(out, cap, "%s", if key == "keyboard" { s.map_kb.title.as_str() } else { s.map_kb.text.as_str() });
    } else if (key == "keyboard" || key == "text") && s.find.open {
        // Its keyboard's title and what the field holds (the Pages view's, otherwise).
        csnprintf!(out, cap, "%s", if key == "keyboard" { s.find.kb.title.as_str() } else { s.find.kb.text.as_str() });
    } else if (key == "keyboard" || key == "text") && s.card.kb.open {
        // Rename's (the card's).
        csnprintf!(out, cap, "%s", if key == "keyboard" { s.card.kb.title.as_str() } else { s.card.kb.text.as_str() });
    } else if (key == "keyboard" || key == "text") && s.add_kb.open {
        // The toolbar's count pad's.
        csnprintf!(out, cap, "%s", if key == "keyboard" { s.add_kb.title.as_str() } else { s.add_kb.text.as_str() });
    } else if (key == "keyboard" || key == "text") && s.lm.kb.open {
        // New page...'s.
        csnprintf!(out, cap, "%s", if key == "keyboard" { s.lm.kb.title.as_str() } else { s.lm.kb.text.as_str() });
    } else if key == "bar" {
        // The Links bar: closed (0), open, or looking at another page (peek).
        csnprintf!(out, cap, "%s", if !s.lb.open { "0" } else if s.peek.active { "peek" } else { "open" });
    } else if key == "bar.peek" {
        // The number of the page the bar is looking at, 0 when it is not.
        csnprintf!(out, cap, "%d", if s.lb.open && s.peek.active { s.peek.index + 1 } else { 0 });
    } else if key == "canvas_cache" {
        // What the bar's canvas cache did since the bar opened: taps that found their page ready, taps that did
        // not, and the pages it holds now ("1 0 2").
        let held = s.ccache.iter().filter(|e| e.pid != 0 && !e.canvas.px.is_empty()).count();
        csnprintf!(out, cap, "%d %d %d", s.cache_hits, s.cache_misses, held as i32);
    } else if let Some(k) = key.strip_prefix("bar.") {
        if !linkbar_probe(&s.lb, k, out, cap) {
            return -1;
        }
    } else if key == "scrolls" {
        // Whether the page scrolls on each axis now, "V H" (what a two-finger drag pans, touchgest.rs).
        let pans = s.open && !sheet_open(s);
        let v = pans && (a.h as f32 / s.zoom) < s.pgh as f32 - 0.5;
        let h = pans && (a.w as f32 / s.zoom) < s.pgw as f32 - 0.5;
        csnprintf!(out, cap, "%d %d", v as i32, h as i32);
    } else if key == "stroke_start" {
        // Where the page's newest stroke starts, in page px: ink lands under the pen through the view.
        let p = if has_page { s.open_page().and_then(|p| p.strokes.last()) } else { None };
        match p {
            Some(x) if !x.pts.is_empty() => {
                csnprintf!(out, cap, "%.0f %.0f", x.pts[0].x, x.pts[0].y)}
            _ => csnprintf!(out, cap, "%s", ""),
        };
    } else if key == "music_max" {
        // The most systems the Custom music sheet's layout fits on the page it is for.
        if s.mb.open {
            csnprintf!(out, cap, "%d", music_max_systems(&s.mb.m, s.mb.page_h, dpi));
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "tools.ink" {
        // The ink the open picker shows selected: "white", "black", or "" with no picker (or Pin mode).
        let v = if s.tools_open && !s.shapes_menu { s.tools_ink_drawn.map_or("", |w| if w { "white" } else { "black" }) } else { "" };
        csnprintf!(out, cap, "%s", v);
    } else if key == "hold_eraser" {
        csnprintf!(out, cap, "%d", if s.hold_eraser { 1 } else { 0 }); // Erase while held is down
    } else if key == "owner" {
        // Who has the contact now: "none", or the owner and which input ("lasso pen", "link finger").
        if s.owner == Owner::None {
            csnprintf!(out, cap, "%s", "none");
        } else {
            csnprintf!(out, cap, "%s %s", owner_name(s.owner), if s.owner_is_pen { "pen" } else { "finger" });
        }
    } else if key == "modal" {
        // What is in front (input goes to the first one open, in this order).
        let m = if crate::library::library_settings_active(a) {"settings"} else if s.map.open {
            "map"
        } else if s.add_kb.open {
            "add-pad"
        } else if s.find.open {
            "find"
        } else if s.card.open {
            "card"
        } else if s.qs.open {
            "quick-size"
        } else if s.lt.open && !s.pv.open {
            "linkto"
        } else if s.lb.open {
            "links"
        } else if s.pv.open {
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
    } else if key == "stroke_turn_deg" {
        // The newest stroke's sharpest turn from one piece to the next, in whole degrees, the turn
        // through its start included when it is closed: 90 at a snapped rectangle's corners, a few
        // where Corner roundness made them arcs. Empty with no stroke or fewer than three points.
        let x = if has_page { newest(s) } else { None };
        if let Some(x) = x.filter(|x| x.pts.len() >= 3) {
            let q = &x.pts;
            let closed = q[0].x == q[q.len() - 1].x && q[0].y == q[q.len() - 1].y;
            let v: Vec<(f32, f32)> = q[..q.len() - closed as usize].iter().map(|p| (p.x, p.y)).collect();
            let n = v.len();
            let mut most = 0.0f32;
            for k in if closed { 0..n } else { 1..n - 1 } {
                let (a, b, c) = (v[(k + n - 1) % n], v[k], v[(k + 1) % n]);
                let (u, w) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
                if (u.0 != 0.0 || u.1 != 0.0) && (w.0 != 0.0 || w.1 != 0.0) {
                    most = most.max(fmla(u.0, w.1, -(u.1 * w.0)).atan2(fmla(u.0, w.0, u.1 * w.1)).abs().to_degrees());
                }
            }
            csnprintf!(out, cap, "%d", most.round() as i32);
        }
    } else if key == "erase_strokes" {
        let n = if has_page { s.open_page().unwrap().strokes.iter().filter(|x| stroke_is_erase(x)).count() as i32 } else { 0 };
        csnprintf!(out, cap, "%d", n);
    } else if key == "page_kept" {
        // Whether the open page came up from its kept drawing instead of being drawn stroke by stroke.
        csnprintf!(out, cap, "%d", s.page_kept as i32);
    } else if key == "loading_toasts" {
        // How many times "Loading page..." went up before a slow page was drawn as it opened.
        csnprintf!(out, cap, "%d", s.loading_toasts as i32);
    } else if key == "add_notices" {
        // How many times "Adding N pages..." reached the panel before a batch of new pages was written, or a
        // batch's Redo wrote them again.
        csnprintf!(out, cap, "%d", s.add_notices as i32);
    } else if key == "erase_tiles" {
        // The stroke eraser's tiles drawn again since the editor began, and the tiles of the boxes of
        // what it took off (all it drew before 2.0.0-rc.12).
        csnprintf!(out, cap, "%u %u", s.erase_tiles.0, s.erase_tiles.1);
    } else if key == "undo_patched" {
        // Undos and redos that swapped a patch back instead of drawing the region again.
        csnprintf!(out, cap, "%u", s.undo_patched);
    } else if key == "live_matches_render" {
        // The open page drawn afresh, pixel for pixel what drawing, erasing and undoing left in the
        // page canvas: 1 when they are the same.
        let mut same = false;
        let mut c = Canvas::default();
        if has_page && s.canvas_page == s.open_page_ref() && !s.page_canvas.px.is_empty() && canvas_init(&mut c, s.pgw, s.pgh) == 0 {
            page_render(&mut c, s.open_page().unwrap(), Rect { x: 0, y: 0, w: s.pgw, h: s.pgh ,
                }, dpi,
            );
            let pc = &s.page_canvas;
            same = pc.w == c.w
                && pc.h == c.h
                && pc.stride == c.stride
                && c.px[..c.stride as usize * c.h as usize] == pc.px[..c.stride as usize * c.h as usize];
            if !same && pc.px.len() == c.px.len() {
                // (where they differ, in the log: the count, the box and the first pixel's two values)
                let (mut n, mut x0, mut y0, mut x1, mut y1, mut first) = (0, i32::MAX, i32::MAX, -1, -1, None);
                for (i, (&a, &b)) in pc.px.iter().zip(c.px.iter()).enumerate() {
                    if a != b {
                        let (x, y) = ((i as i32) % c.stride, (i as i32) / c.stride);
                        n += 1; x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x); y1 = y1.max(y);
                        first.get_or_insert((x, y, a, b));
                    }
                }
                let (fx, fy, fa, fb) = first.unwrap_or((0, 0, 0, 0));
                qlog!("editor: live differs from a fresh drawing at %d pixels, %d,%d-%d,%d, first at %d,%d: %d live, %d drawn",
                    n, x0, y0, x1, y1, fx, fy, fa as i32, fb as i32);
            }
        }
        csnprintf!(out, cap, "%d", if same { 1 } else { 0 });
    } else if key == "kept_matches_render" || key == "preview_matches_render" || key == "card_matches_render" {
        // What the caches beside the pages show, against the page drawn afresh: 1 the same, 0 not, none with no
        // such cache (cache_matches_render).
        let v = if has_page { cache_matches_render(s, key, dpi) } else { None };
        csnprintf!(out, cap, "%s", match v { None => "none", Some(true) => "1", Some(false) => "0" });
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
        // 1 while the panel is inside the screen, its buttons at full size (as without pins,
        // upright or sideways)
        let p = s.panel;
        let in_ = p.x >= 0 && p.x + p.w <= a.w && p.y >= 0 && p.y + p.h <= a.h;
        csnprintf!(out, cap, "%d", if in_ && btn_h() == ui_px(100.0) { 1 } else { 0 });
    } else if key == "pins" {
        settings_pins_text(st, out, cap); // the pinned pens in pin order, as settings.ini names them
    } else if key == "draw_tool" {
        let name = app_probe_tool(button_tool(st)); // the tool button's pen
        csnprintf!(out, cap, "%s", name.unwrap_or("?"));
    } else if key == "tool_button" {
        // What the tool button showed as the panel was last drawn: its pen, "grid" while that pen
        // is pinned, "select" while Select mode is on, empty while the panel is collapsed.
        let t = s.tool_button_shows;
        let name = if t >= 0 { app_probe_tool(ToolKind(t)) } else { None };
        csnprintf!(out, cap, "%s", if t == TB_GRID { "grid" } else if t == TB_SELECT { "select" } else if t == TB_NONE { "" } else { name.unwrap_or("?") });
    } else if key == "pin_mode" {
        csnprintf!(out, cap, "%d", if s.tools_open && s.pin_mode { 1 } else { 0 });
    } else if key == "shapes_menu" {
        csnprintf!(out,cap,"%d",(s.tools_open && s.shapes_menu) as i32);
    } else if key == "link_shape" {
        csnprintf!(out,cap,"%s",format!("{:?}",s.lm.shape_kind).to_lowercase().as_str());
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

    // Generated ink with STRESS TEST 3's counts and long stroke; no owner's drawing is stored here.
    fn stress_ink() -> Box<Page> {
        let mut p = page_new(1000, 1400, PAPER_BLANK);
        for k in 0..698 {
            let n = if k == 0 { 6648 } else { 225 + usize::from(k <= 477) };
            let tool = if k < 75 { TOOL_HIGHLIGHTER } else { TOOL_FOUNTAIN };
            let mut prm = ToolParams::default();
            tool_params_default(tool, &mut prm);
            let mut ink = stroke_new(tool, &prm, 19 + k as u32);
            for j in 0..n {
                stroke_append(&mut ink, &PenSample { x: 100.0 + (j % 200) as f32 * 3.0,
                    y: 200.0 + (k % 60) as f32 * 12.0, pressure: 0.6, t_ms: j as u32 * 3,
                    ..Default::default() }, 300.0);
            }
            page_add(&mut p, ink);
        }
        assert_eq!(p.strokes.iter().map(|s| s.pts.len()).sum::<usize>(), 163950);
        p
    }

    // A file's identity as the cache writes show it: its inode (a cache file is written aside and renamed over),
    // its size and its mtime.
    #[cfg(unix)]
    fn file_stamp(p: &std::path::Path) -> Option<(u64, u64, i64, i64)> {
        use std::os::unix::fs::MetadataExt;
        let m = p.metadata().ok()?;
        Some((m.ino(), m.len(), m.mtime(), m.mtime_nsec()))
    }

    // The window and the quiet time both over, as they are 30 s after the last save and the last change.
    fn caches_window_over(s: &mut EditorState) {
        s.preview_saved_ms = now_ms().wrapping_sub(cache_window_ms());
        s.last_up = now_ms().wrapping_sub(cache_window_ms());
    }

    // An open dense page written from blank: a quick open (full_ms 0), STRESS TEST 3's ink on it, changed.
    fn dense_app_page(root_name: &str) -> (std::path::PathBuf, App) {
        let root = fhold_root(root_name);
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Dense", "", PAPER_BLANK, &mut nb), 0);
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.nb = nb;
        s.hist[0].page = Some(stress_ink());
        s.page = Some(1);
        s.full_ms = (1, 0);
        draw_page(&mut a, Rect::new(0, 0, 1000, 1400));
        let s = ed(&mut a);
        s.canvas_page = s.open_page_ref();
        assert!(s.page_canvas.px.iter().any(|&v| v < 255));
        s.preview_saved_ms = now_ms().wrapping_sub(30_000);
        mark_changed(s);
        (root, a)
    }

    #[cfg(unix)]
    #[test]
    fn dense_written_from_blank_keeps_its_drawing_once_quiet_or_left_and_never_twice_in_a_window() {
        let (root, mut a) = dense_app_page("dense-saves");
        // The idle save: the page and its preview now; the drawing waits for a quiet page (a page written in
        // bursts would write 3.4 MB at every pause)
        assert!(autosave(&mut a));
        let s = ed_ref(&a);
        assert!(store_page_dense(&s.nb, 1));
        assert!(store_page_preview_ok(&s.nb, 1));
        assert!(!store_page_render_kept(&s.nb, 1, 1000, 1400, false), "no drawing at the first idle save");
        assert!(s.caches_pending);
        let dir = root.join("notebooks").join(s.nb.id.as_str());
        let files = [dir.join("p1.qpg"), dir.join(".page-previews/p1.pgm"), dir.join(".page-previews/p1.full")];
        let stamps = || files.iter().map(|p| file_stamp(p)).collect::<Vec<_>>();
        // Not due before the window and the quiet time are over (editor_tick would run autosave at each pass)
        let now = now_ms();
        assert!(!caches_due(ed_ref(&a), now.wrapping_add(10_000)));
        assert!(caches_due(ed_ref(&a), now.wrapping_add(31_000)));
        caches_window_over(ed(&mut a));
        assert!(autosave(&mut a));
        assert!(store_page_render_kept(&ed_ref(&a).nb, 1, 1000, 1400, false));
        assert!(!ed_ref(&a).caches_pending);
        let first = stamps();
        assert!(first.iter().all(|f| f.is_some()));
        // Nothing changed: no save of any kind writes anything again
        caches_window_over(ed(&mut a));
        assert!(autosave(&mut a));
        assert!(save_page(&mut a));
        assert!(save_page_leaving(&mut a));
        assert_eq!(first, stamps(), "unchanged saves write nothing again");
        // A change: Ctrl+S, the Pages view, Settings and a sleep (save_page) write the page file and no cache,
        // however often and even with the window over
        let s = ed(&mut a);
        s.open_page_mut().unwrap().strokes.pop();
        mark_changed(s);
        caches_window_over(s);
        assert!(save_page(&mut a));
        assert!(save_page(&mut a));
        let second = stamps();
        assert_ne!(first[0], second[0]);
        assert_eq!(first[1..], second[1..], "an explicit save makes no cache");
        assert!(ed_ref(&a).caches_pending);
        // An idle save inside the window makes none either (written a moment ago)
        let s = ed(&mut a);
        s.preview_saved_ms = now_ms();
        s.last_up = now_ms().wrapping_sub(cache_window_ms());
        assert!(autosave(&mut a));
        assert_eq!(second, stamps(), "no idle write inside the window");
        // Leaving writes the drawing at once, inside the window, and no preview; a second time, nothing
        assert!(save_page_leaving(&mut a));
        assert!(store_page_render_kept(&ed_ref(&a).nb, 1, 1000, 1400, false));
        let leaving = stamps();
        assert_ne!(leaving[2], second[2]);
        assert_eq!(leaving[..2], second[..2]);
        assert!(save_page_leaving(&mut a));
        assert_eq!(leaving, stamps());
        // The preview that waited finishes at a later idle save, without writing the page or the drawing again
        caches_window_over(ed(&mut a));
        assert!(autosave(&mut a));
        let last = stamps();
        assert_eq!(leaving[0], last[0]);
        assert_eq!(leaving[2], last[2]);
        assert_ne!(leaving[1], last[1]);
        assert!(!ed_ref(&a).caches_pending);
        // Neither a partial canvas nor one missing its picture can become a kept drawing
        for missing in [false, true] {
            let s = ed(&mut a);
            s.open_page_mut().unwrap().strokes.pop();
            mark_changed(s);
            s.picture_missing = missing;
            s.canvas_page = if missing { s.open_page_ref() } else { 0 };
            assert!(save_page_leaving(&mut a));
            assert!(!store_page_render_kept(&ed_ref(&a).nb, 1, 1000, 1400, false));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_caches_run_no_autosave_while_they_cannot_be_made() {
        // 2.1.0 review: the flag set by a save outlived a canvas that was not whole (an image paper's picture
        // still being made) or a read-only page, and editor_tick ran the autosave at every pass of the main loop.
        let (root, mut a) = dense_app_page("dense-pending");
        assert!(save_page(&mut a));
        let later = now_ms().wrapping_add(120_000);
        let s = ed(&mut a);
        s.last_up = now_ms().wrapping_sub(120_000);
        s.preview_saved_ms = now_ms().wrapping_sub(120_000);
        assert!(s.caches_pending);
        assert!(caches_due(s, later), "due when they can be made");
        s.picture_missing = true;
        assert!(!caches_due(s, later), "not while the picture is white");
        s.picture_missing = false;
        s.canvas_page = 0;
        assert!(!caches_due(s, later), "not while the canvas is another page's");
        s.canvas_page = s.open_page_ref();
        assert!(caches_due(s, later));
        s.page_dirty = true;
        assert!(!caches_due(s, later), "a changed page is saved first (page_dirty)");
        s.page_dirty = false;
        // A read-only page is never written: nothing is left pending for it
        s.open_page_mut().unwrap().readonly = true;
        assert!(autosave(&mut a));
        assert!(!ed_ref(&a).caches_pending);
        assert!(!caches_due(ed_ref(&a), later));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_keeps_an_unchanged_library_card_and_reuses_current_first_page_caches() {
        let root = fhold_root("dense-card");
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Card", "", PAPER_BLANK, &mut nb), 0);
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.nb = nb;
        let mut c = Canvas::default();
        canvas_init(&mut c, 1000, 1400);
        c.px.fill(23);
        store_save_thumb(s.nb.id.as_str(), &c);
        // A card nothing says what it was drawn from (one of an older version, a stale one at the open) is drawn
        // again: the open does not take it for page 1's as it is (2.1.0 review)
        let before = store_thumb_stamp(s.nb.id.as_str());
        let mut stale = Canvas::default();
        refresh_library_thumb(s, 300.0);
        assert_ne!(before, store_thumb_stamp(s.nb.id.as_str()), "a card of unknown source is drawn again");
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut stale), 0);
        assert!(stale.px.iter().all(|&v| v == 255), "a blank page 1: the card is blank, not the stale grey");
        store_save_thumb(s.nb.id.as_str(), &c);
        // (the card now is another file: a source noted for the one before does not hold for it)
        refresh_library_thumb(s, 300.0);
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut stale), 0);
        assert!(stale.px.iter().all(|&v| v == 255));
        c.px.fill(23);
        store_save_thumb(s.nb.id.as_str(), &c);
        store_note_thumb_source(s.nb.id.as_str(), &first_page_source(&s.nb));
        let before = store_thumb_stamp(s.nb.id.as_str());
        refresh_library_thumb(s, 300.0);
        assert_eq!(before, store_thumb_stamp(s.nb.id.as_str()), "other-page edits leave the card alone");
        // A changed first page: a sentinel kept drawing proves there is no stroke redraw.
        assert_eq!(store_save_page(&s.nb, 0, &stress_ink()), 0);
        assert!(store_save_page_render(&s.nb, 1, &c, false));
        refresh_library_thumb(s, 300.0);
        let mut card = Canvas::default();
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut card), 0);
        assert!(card.px.iter().all(|&v| v == 23));
        let current = store_thumb_stamp(s.nb.id.as_str());
        refresh_library_thumb(s, 300.0);
        assert_eq!(current, store_thumb_stamp(s.nb.id.as_str()));
        // With no kept drawing, a current preview also avoids a stroke redraw.
        let mut p = stress_ink(); p.strokes.pop();
        assert_eq!(store_save_page(&s.nb, 0, &p), 0);
        c.px.fill(71);
        store_save_page_preview(&s.nb, 1, &c);
        refresh_library_thumb(s, 300.0);
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut card), 0);
        assert!(card.px.iter().all(|&v| v == 71));
        // A new first page must replace the card, even if its file was never edited here.
        assert_eq!(store_add_page(&mut s.nb, 0, PAPER_BLANK), 0);
        refresh_library_thumb(s, 300.0);
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut card), 0);
        assert!(card.px.iter().all(|&v| v == 255));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The quick sizes card is a sheet in every check that asks whether something is in front, and Live view's fast
    /// ink never previews a pen contact on it (the card keeps the clock corner, as the long-press card does).
    #[test]
    fn the_quick_sizes_card_is_a_sheet_and_makes_no_fast_ink() {
        let mut a = app(1860, 2480, 1860, 2480);
        let s = ed(&mut a);
        s.open = true;
        assert!(!sheet_open(ed_ref(&a)) && !editor_pen_not_ink(&a));
        ed(&mut a).qs.open = true;
        assert!(sheet_open(ed_ref(&a)));
        assert!(editor_pen_not_ink(&a));
        // (so the browser's live style is off for any pen contact while it is up)
        let ev = InEvent { type_: InType::PenDown, x: 900.0, y: 900.0, pressure: 0.5, ..Default::default() };
        let mut out = LiveStyle::default();
        editor_live_style(&a, &ev, &mut out);
        assert_eq!(out.enabled, 0);
        assert!(!corner_sheet_open(ed_ref(&a)));
        assert!(!qs_allowed(ed_ref(&a)), "no second card over the first");
        let s = ed(&mut a);
        s.qs.open = false;
        s.qs = QuickSize::default();
        assert!(qs_allowed(ed_ref(&a)));
        // a hold targets a pinned pen, the eraser and the tool button while it shows a pen, nothing else
        let hit = |id: i32, arg: i32| Hit { r: RECT_EMPTY, id, arg };
        let s = ed(&mut a);
        s.tool_button_shows = TOOL_PENCIL.0;
        assert_eq!(qs_target(s, hit(P_TOOLPICK, 0)), Some(TOOL_PENCIL));
        assert_eq!(qs_target(s, hit(P_TOOL, TOOL_ERASER.0)), Some(TOOL_ERASER));
        s.tool_button_shows = TB_GRID;
        assert_eq!(qs_target(s, hit(P_TOOLPICK, 0)), None);
        s.tool_button_shows = TB_SELECT;
        assert_eq!(qs_target(s, hit(P_TOOLPICK, 0)), None);
        assert_eq!(qs_target(s, hit(P_UNDO, 0)), None);
        assert_eq!(qs_target(s, hit(P_EXPAND, 0)), None);
    }

    #[test]
    fn the_live_toolbar_layer_stays_where_the_panel_is_under_any_menu() {
        // 2.0.0, issue #16: the layer was empty while any menu was up, so the browser lost the toolbar beside the
        // Display sheet (and the Paper sheet, ...) while the Kindle still showed it. The layer is where the panel is
        // with a menu beside it or over it: it holds the screen there (live_push_screen), the menu's part over the
        // panel too, as the Kindle shows it.
        let mut a = app(1860, 2480, 1860, 2480);
        let s = ed(&mut a);
        s.open = true;
        s.panel = Rect { x: 1700, y: 300, w: 140, h: 900 };
        let panel = s.panel;
        assert_eq!(editor_live_panel(&a), panel);
        for open in [
            // the tool picker and its Pin mode beside the panel, and over it
            |s: &mut EditorState, on: bool| { s.tools_open = on; s.pin_mode = on; s.tools_rect = Rect { x: 500, y: 300, w: 1180, h: 900 }; },
            |s: &mut EditorState, on: bool| { s.tools_open = on; s.tools_rect = Rect { x: 600, y: 300, w: 1180, h: 900 }; },
            |s: &mut EditorState, on: bool| s.dm.open = on,
            |s: &mut EditorState, on: bool| s.paper_open = on,
            |s: &mut EditorState, on: bool| s.ts.open = on,
            |s: &mut EditorState, on: bool| s.mb.open = on,
            |s: &mut EditorState, on: bool| s.lb.open = on,
            |s: &mut EditorState, on: bool| s.lt.open = on,
            |s: &mut EditorState, on: bool| s.find.open = on,
            |s: &mut EditorState, on: bool| s.card.open = on,
            |s: &mut EditorState, on: bool| s.qs.open = on,
        ] {
            open(ed(&mut a), true);
            assert_eq!(editor_live_panel(&a), panel);
            assert_eq!(editor_live_hole(&a), panel); // (the tiles there hold the page and the menu, not the panel)
            open(ed(&mut a), false);
        }
        // A view over the whole screen draws no panel: no layer (the Pages view's frame has no hole either).
        for open in [
            |s: &mut EditorState, on: bool| s.pv.open = on,
            |s: &mut EditorState, on: bool| s.map.open = on,
            |s: &mut EditorState, on: bool| s.destination.open = on,
        ] {
            open(ed(&mut a), true);
            assert!(rect_is_empty(editor_live_panel(&a)));
            open(ed(&mut a), false);
            assert_eq!(editor_live_panel(&a), panel);
        }
        ed(&mut a).open = false;
        assert!(rect_is_empty(editor_live_panel(&a)));
    }
    fn fshape_app() -> App {
        #[cfg(feature = "sim")]
        {
            // Allocate the simulator's presented buffer without opening a window or script.
            let (mut w, mut h) = (0, 0);
            crate::sim_sdl::display_set_orientation(0, &mut w, &mut h);
        }
        let mut a = app(1000, 1400, 1000, 1400);
        a.scr = ScreenId::Editor;
        a.settings.status_on = false;
        a.settings.status_mode = 2;
        a.settings.gestures = true;
        a.settings.hints |= HINT_FSHAPE;
        a.settings.hints |= HINT_BAR_MASK << HINT_BAR_SHIFT;
        gesturemap_defaults(&mut a.gestures);
        assert!(gesturemap_set(&mut a.gestures, fs_touch1_key(FsSwipe::Right), SHORTCUT_ZOOM_IN) >= 0);
        let s = ed(&mut a);
        nav_init(&mut s.nav);
        s.open = true;
        s.nb.page_ids = vec![1, 2];
        for pid in [1, 2] {
            let i = s.entry_for(pid).unwrap();
            s.hist[i].page = Some(page_new(1000, 1400, PAPER_BLANK));
        }
        s.page = Some(1);
        assert_eq!(canvas_init(&mut s.page_canvas, 1000, 1400), 0);
        render_page_full(&mut a);
        view_reset(&mut a);
        compose_all(&mut a, RefreshMode::Ui);
        app_flush(&mut a);
        a
    }

    #[test]
    fn adding_pages_says_so_only_for_a_big_batch() {
        // About 40 ms a page on a Kindle (ADD_PAGE_MS), on any paper: over 2 s from 51 pages, and always from 100.
        let mut a = App::default();
        for (n, due) in [(1, false), (10, false), (25, false), (50, false), (51, true), (99, true), (100, true), (2000, true)] {
            assert_eq!(add_notice_due(&a, n), due, "{n} pages");
        }
        // A pen held on an action holds the panel: no notice could show before the writing.
        a.pen_action_down = true;
        assert!(!add_notice_due(&a, 2000));
    }

    #[test]
    fn space_does_nothing_under_find_with_gestures_on_or_off() {
        for gestures in [false, true] {
            let mut a = fshape_app();
            a.settings.gestures = gestures;
            find_open(&mut a);
            assert!(ed_ref(&a).find.open);
            assert!(!editor_shortcut(&mut a, SHORTCUT_TWO_FINGER_UP));
            assert!(ed_ref(&a).find.open && !ed_ref(&a).lb.open, "gestures {gestures}");
        }
        // With nothing in front, Space with gestures off opens the Links bar, as before.
        let mut a = fshape_app();
        a.settings.gestures = false;
        assert!(editor_shortcut(&mut a, SHORTCUT_TWO_FINGER_UP));
        assert!(ed_ref(&a).lb.open);
    }

    #[test]
    fn clear_removes_current_page_links_and_ink_as_one_undoable_saved_change() {
        let root=std::env::temp_dir().join(format!("grimoire-clear-links-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);store_init(root.to_str().unwrap());
        let mut nb=NotebookInfo::default();assert_eq!(store_create("Void","",PAPER_BLANK,&mut nb),0);
        assert_eq!(store_add_page(&mut nb,1,PAPER_BLANK),0);
        let mut a=fshape_app();
        let s=ed(&mut a);s.nb=nb;nav_init(&mut s.nav);
        assert_eq!(nav_link_set(&mut s.nav,1,77,"index"),0);
        assert_eq!(nav_link_set_area(&mut s.nav,1,77,Some(Rect{x:100,y:200,w:300,h:300})),0);
        assert_eq!(nav_link_set(&mut s.nav,2,78,"index"),0);
        assert_eq!(nav_save_links(s.nb.id.as_str(),&mut s.nav,&s.nb),0);
        let mut prm=ToolParams::default();tool_params_default(TOOL_PENCIL,&mut prm);
        let mut ink=stroke_new(TOOL_PENCIL,&prm,123);
        stroke_append(&mut ink,&PenSample{x:300.0,y:400.0,pressure:0.5,..Default::default()},300.0);
        page_add(s.open_page_mut().unwrap(),ink);
        clear_page(&mut a);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(),0);
        assert!(nav_link_get(&ed_ref(&a).nav,1,77).is_none());
        assert!(nav_link_get(&ed_ref(&a).nav,2,78).is_some());
        let mut saved=Nav::default();nav_load(ed_ref(&a).nb.id.as_str(),&mut saved);
        assert!(nav_link_get(&saved,1,77).is_none());
        let h=ed(&mut a).cur_hist().unwrap();
        assert_eq!(ed_ref(&a).hist[h].h.undo.last().unwrap().kind,ActKind::Clear);
        let blocked=root.join("notebooks").join(ed_ref(&a).nb.id.as_str()).join("links.ini.tmp");
        std::fs::create_dir(&blocked).unwrap();
        undo_redo(&mut a,true);
        assert!(ed_ref(&a).open_page().unwrap().strokes.is_empty());
        assert!(nav_link_get(&ed_ref(&a).nav,1,77).is_none());
        assert_eq!(history_undo_kind(&ed_ref(&a).hist[h].h),Some(ActKind::Clear));
        std::fs::remove_dir(&blocked).unwrap();
        undo_redo(&mut a,true);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(),1);
        assert!(nav_link_get(&ed_ref(&a).nav,1,77).unwrap().has_area);
        std::fs::create_dir(&blocked).unwrap();
        clear_page(&mut a);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(),1);
        assert!(nav_link_get(&ed_ref(&a).nav,1,77).is_some());
        undo_redo(&mut a,false);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(),1);
        assert!(nav_link_get(&ed_ref(&a).nav,1,77).is_some());
        assert_eq!(history_redo_kind(&ed_ref(&a).hist[h].h),Some(ActKind::Clear));
        std::fs::remove_dir(&blocked).unwrap();
        undo_redo(&mut a,false);
        assert!(ed_ref(&a).open_page().unwrap().strokes.is_empty());
        assert!(nav_link_get(&ed_ref(&a).nav,1,77).is_none());
        // A page with no ink still has something to clear when it has a link.
        let s=ed(&mut a);assert_eq!(nav_link_set(&mut s.nav,1,88,"index"),0);
        assert_eq!(nav_save_links(s.nb.id.as_str(),&mut s.nav,&s.nb),0);
        clear_page(&mut a);assert!(nav_link_get(&ed_ref(&a).nav,1,88).is_none());
        undo_redo(&mut a,true);assert!(nav_link_get(&ed_ref(&a).nav,1,88).is_some());
        assert!(ed_ref(&a).open_page().unwrap().strokes.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    // The open page drawn afresh from its strokes, pixel for pixel what the page canvas holds.
    fn canvas_is_the_page(a: &App) -> bool {
        let s = ed_ref(a);
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, s.pgw, s.pgh), 0);
        page_render(&mut c, s.open_page().unwrap(), Rect { x: 0, y: 0, w: s.pgw, h: s.pgh }, a.dpi);
        c.px == s.page_canvas.px
    }

    #[test]
    fn a_kept_drawing_stands_for_the_page_only_while_the_page_is_its_file() {
        // 2.0.0 to 2.0.1: Clear page and a new paper drew the page again before they marked it changed, and read
        // the drawing kept of its file back, the old ink and paper; a page with a change not yet written read it
        // back too wherever it was drawn whole.
        let root = std::env::temp_dir().join(format!("grimoire-kept-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Kept", "", PAPER_BLANK, &mut nb), 0);
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.nb = nb;
        // Page 1 with enough ink to say "Loading page..." as it opens (store_page_slow: a file of 256 KB or more)
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_BALLPOINT, &mut prm);
        for k in 0..160u32 {
            let mut ink = stroke_new(TOOL_BALLPOINT, &prm, 0x900 + k);
            for i in 0..100 {
                let t = i as f32 / 99.0;
                stroke_append(&mut ink, &PenSample { x: 60.0 + 880.0 * t, y: 40.0 + 8.0 * k as f32 + 6.0 * (t * 17.0).sin(), pressure: 0.6, t_ms: i * 5, ..Default::default() }, 300.0);
            }
            page_add(s.open_page_mut().unwrap(), ink);
        }
        let keep = |a: &mut App| {
            let s = ed(a);
            assert_eq!(store_save_page(&s.nb, 0, s.open_page().unwrap()), 0);
            s.page_dirty = false;
            render_page_full(a);
            let s = ed(a);
            assert!(store_save_page_render(&s.nb, 1, &s.page_canvas, false));
            assert!(store_page_slow(&s.nb, 1) && store_page_render_kept(&s.nb, 1, s.pgw, s.pgh, false));
        };
        keep(&mut a);
        // The page as its file is comes up from the drawing kept of it, with no "Loading page..."
        assert!(!loading_toast_due(&a));
        render_page_full(&mut a);
        assert!(ed_ref(&a).page_kept && canvas_is_the_page(&a));
        // A change not yet written: drawn from the strokes, and "Loading page..." would go up
        let s = ed(&mut a);
        let gone = s.open_page_mut().unwrap().strokes.pop().unwrap();
        stroke_free(gone);
        mark_changed(s);
        assert!(loading_toast_due(&a));
        render_page_full(&mut a);
        assert!(!ed_ref(&a).page_kept && canvas_is_the_page(&a));
        // A new paper
        keep(&mut a);
        apply_paper(&mut a, PAPER_NARROW);
        assert_eq!(ed_ref(&a).open_page().unwrap().paper, PAPER_NARROW);
        assert!(!ed_ref(&a).page_kept && canvas_is_the_page(&a));
        // Clear page
        keep(&mut a);
        clear_page(&mut a);
        assert!(ed_ref(&a).open_page().unwrap().strokes.is_empty());
        assert!(!ed_ref(&a).page_kept && canvas_is_the_page(&a));
        std::fs::remove_dir_all(root).unwrap();
    }

    // What 2.0.0 and 2.0.1 may have left of a cleared page pid: its old ink kept under the cleared file's stamp (their
    // tag, with no "|k2") and the Pages preview made from it. The old ink (the Library card was made from it) and the
    // drawing's stamp.
    fn ghost_of_2_0_1(s: &mut EditorState, pid: u32) -> (Canvas, String) {
        let mut ghost = Canvas::default();
        assert_eq!(canvas_init(&mut ghost, s.pgw, s.pgh), 0);
        ghost.px.iter_mut().enumerate().for_each(|(i, v)| *v = if i % 7 == 0 { 40 } else { 255 });
        assert!(store_save_page_render(&s.nb, pid, &ghost, false));
        store_save_page_preview(&s.nb, pid, &ghost);
        let mut dir = String::new();
        assert_eq!(store_notebook_dir(s.nb.id.as_str(), &mut dir, 600), 0);
        let stamp = format!("{dir}/.page-previews/p{pid}.full.stamp");
        let tag = std::fs::read_to_string(&stamp).unwrap();
        std::fs::write(&stamp, tag.replace("|k2", "")).unwrap(); // (2.0.1's tag)
        (ghost, stamp)
    }

    // The open page's Pages preview on disk is the one made from the page canvas.
    fn preview_is_the_canvas(s: &EditorState) -> bool {
        let mut shown = Canvas::default();
        let mut made = Canvas::default();
        store_load_page_preview(&s.nb, s.open_pid(), &mut shown) && store_page_preview_image(&s.page_canvas, &mut made) && shown.px == made.px
    }

    #[test]
    fn a_drawing_kept_by_2_0_1_goes_with_the_preview_and_card_made_beside_it() {
        // A cleared page's ghost from 2.0.1, current for the page file: the page is drawn from its strokes, the
        // drawing goes, and the preview and the card are made again from the page drawn. (The Pages view decodes the
        // preview again, its page file as it was: bgjob.rs a_preview_written_again_is_decoded_again.)
        let root = std::env::temp_dir().join(format!("grimoire-kept-before-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Before", "", PAPER_BLANK, &mut nb), 0);
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.nb = nb;
        assert_eq!(store_save_page(&s.nb, 0, s.open_page().unwrap()), 0); // (blank: cleared)
        let (ghost, stamp) = ghost_of_2_0_1(s, 1);
        let mut card = Canvas::default();
        assert_eq!(canvas_init(&mut card, s.pgw / 3, s.pgh / 3), 0);
        canvas_downsample_keep_lines(&mut card, &ghost, 0.35);
        assert_eq!(store_save_thumb(s.nb.id.as_str(), &card), 0);
        render_page_full(&mut a);
        assert!(!ed_ref(&a).page_kept && canvas_is_the_page(&a));
        let s = ed(&mut a);
        assert!(!std::path::Path::new(&stamp).exists() && !store_retire_page_render(&s.nb, 1));
        assert!(preview_is_the_canvas(s));
        assert!(s.thumb_dirty);
        write_thumb_if_needed(&mut a);
        let s = ed(&mut a);
        let mut shown = Canvas::default();
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut shown), 0);
        canvas_downsample_keep_lines(&mut card, &s.page_canvas, 0.35);
        assert!(shown.px == card.px);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_stale_drawing_kept_by_2_0_1_goes_with_the_preview_and_card_made_beside_it() {
        // The same ghost, then a word written: 2.0.1's idle save inside the previews' 30 s wrote the page file alone
        // (and so does a notebook closed before its idle save), so the drawing is of the file as it was. The card, made
        // from the canvas as page 1 was left, holds the old ink and the word, and so does the preview a later idle save
        // made from that canvas, current for the page file. The drawing goes all the same, and the preview and the card
        // are made again from the page drawn.
        let root = std::env::temp_dir().join(format!("grimoire-kept-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Stale", "", PAPER_BLANK, &mut nb), 0);
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.nb = nb;
        assert_eq!(store_save_page(&s.nb, 0, s.open_page().unwrap()), 0); // (blank: cleared)
        let (mut ghost, stamp) = ghost_of_2_0_1(s, 1);
        let mut prm = ToolParams::default();
        tool_params_default(TOOL_BALLPOINT, &mut prm);
        let mut word = stroke_new(TOOL_BALLPOINT, &prm, 0x77);
        for i in 0..30 {
            let t = i as f32 / 29.0;
            stroke_append(&mut word, &PenSample { x: 300.0 + 500.0 * t, y: 600.0 + 60.0 * t, pressure: 0.6, t_ms: i * 5, ..Default::default() }, 300.0);
        }
        page_add(s.open_page_mut().unwrap(), word);
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(store_save_page(&s.nb, 0, s.open_page().unwrap()), 0);
        assert!(!store_page_render_kept(&s.nb, 1, s.pgw, s.pgh, false) && store_page_preview_fresh(&s.nb, 1).is_none());
        // (the old ink and the word, as 2.0.1 showed the page)
        ghost.px[(600 * s.pgw + 300) as usize..][..500].fill(0);
        store_save_page_preview(&s.nb, 1, &ghost);
        assert!(store_page_preview_fresh(&s.nb, 1).is_some());
        let mut card = Canvas::default();
        assert_eq!(canvas_init(&mut card, s.pgw / 3, s.pgh / 3), 0);
        canvas_downsample_keep_lines(&mut card, &ghost, 0.35);
        assert_eq!(store_save_thumb(s.nb.id.as_str(), &card), 0);
        render_page_full(&mut a);
        assert!(!ed_ref(&a).page_kept && canvas_is_the_page(&a));
        let s = ed(&mut a);
        assert!(!std::path::Path::new(&stamp).exists());
        assert!(preview_is_the_canvas(s));
        assert!(s.thumb_dirty);
        write_thumb_if_needed(&mut a);
        let s = ed(&mut a);
        let mut shown = Canvas::default();
        assert_eq!(store_load_thumb(s.nb.id.as_str(), &mut shown), 0);
        canvas_downsample_keep_lines(&mut card, &s.page_canvas, 0.35);
        assert!(shown.px == card.px);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_drawing_kept_by_2_0_1_goes_as_the_links_bar_stays_on_its_page() {
        // A page reached from the Links bar comes from the bar's canvas, drawn from its strokes ahead of the tap,
        // not from render_page_full: staying there takes 2.0.1's drawing away as well, and makes its preview again,
        // even when the page left had its picture missing (a template's, still being made).
        let root = std::env::temp_dir().join(format!("grimoire-kept-bar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Bar", "", PAPER_BLANK, &mut nb), 0);
        assert_eq!(store_add_page(&mut nb, 1, PAPER_BLANK), 0);
        let mut a = fshape_app();
        let dpi = a.dpi;
        let s = ed(&mut a);
        assert_eq!(nb.page_ids, [1, 2]);
        s.nb = nb;
        assert_eq!(store_save_page(&s.nb, 0, s.open_page().unwrap()), 0);
        let e = s.entry_for(2).unwrap();
        assert_eq!(store_save_page(&s.nb, 1, s.hist[e].page.as_deref().unwrap()), 0); // (blank: cleared)
        let (_, stamp) = ghost_of_2_0_1(s, 2);
        // Page 2 peeked: the bar's canvas of it drawn whole
        let mut peeked = Canvas::default();
        assert_eq!(canvas_init(&mut peeked, s.pgw, s.pgh), 0);
        page_render(&mut peeked, s.hist[e].page.as_deref().unwrap(), Rect { x: 0, y: 0, w: s.pgw, h: s.pgh }, dpi);
        s.ccache[0] = CacheEntry { pid: 2, w: s.pgw, h: s.pgh, pic_gen: bgimage_generation(), canvas: peeked, rows_done: s.pgh, from_hist: true, ..Default::default() };
        s.lb.open = true;
        s.peek = Peek { active: true, index: 1, pid: 2, slot: 0, count: 1, from_back: false };
        s.picture_missing = true;
        bar_commit(&mut a);
        let s = ed(&mut a);
        assert!(!s.lb.open && s.index == 1 && s.open_pid() == 2);
        assert!(canvas_is_the_page(&a));
        let s = ed(&mut a);
        assert!(!s.picture_missing);
        assert!(!std::path::Path::new(&stamp).exists());
        assert!(preview_is_the_canvas(s));
        std::fs::remove_dir_all(root).unwrap();
    }

    fn fshape_event(a: &mut App, type_: InType, x: f32, y: f32, t_ms: u32) {
        editor_event(a, &InEvent { type_, x, y, t_ms, k_ms: t_ms, ..Default::default() });
    }

    fn pen_event(a: &mut App, type_: InType, side: bool, x: f32, y: f32, t_ms: u32) {
        if matches!(type_, InType::PenUp | InType::PenLeave) { app_pen_feedback_end(a); }
        editor_event(a, &InEvent { type_, side_button: side, x, y, t_ms, ..Default::default() });
    }

    // A word of two ink strokes on the open page around page point (x, y), its ids.
    fn sel_word(a: &mut App, x: f32, y: f32) {
        let s = ed(a);
        let ip = s.page_ix().unwrap();
        let page = s.hist[ip].page.as_deref_mut().unwrap();
        for k in 0..2 {
            let mut prm = ToolParams::default();
            tool_params_default(TOOL_BALLPOINT, &mut prm);
            let mut st = stroke_new(TOOL_BALLPOINT, &prm, 0x300 + k);
            for i in 0..20 {
                let t = i as f32 / 19.0;
                stroke_append(&mut st, &PenSample { x: x - 60.0 + 120.0 * t, y: y - 15.0 + 30.0 * k as f32, pressure: 0.6, ..Default::default() }, 300.0);
            }
            page_add(page, st);
        }
        render_page_full(a);
    }

    // A pen lasso (screen px) around (cx, cy), r wide, the button `side` held through it.
    fn sel_lasso(a: &mut App, cx: f32, cy: f32, r: f32, side: bool, t: u32) {
        pen_event(a, InType::PenDown, side, cx - r, cy - r, t);
        let ring = [(cx, cy - r - 5.0), (cx + r, cy - r), (cx + r + 5.0, cy), (cx + r, cy + r), (cx, cy + r + 5.0), (cx - r, cy + r), (cx - r - 5.0, cy), (cx - r, cy - r + 10.0)];
        for (k, &(x, y)) in ring.iter().enumerate() {
            pen_event(a, InType::PenMove, side, x, y, t + 10 * (k as u32 + 1));
        }
        pen_event(a, InType::PenUp, side, cx - r, cy - r + 10.0, t + 100);
    }

    // `select` on a pen button's drag is held, not toggled: the contact under the button is the lasso
    // (never early ink), its selection stays for the bar once the button is let go, and a tap outside it
    // then ends Select mode without a dot; a lasso around nothing ends it at once (beta.2 brief, Lane B 1).
    #[test]
    fn select_held_on_a_pen_drag() {
        let mut a = fshape_app();
        assert_eq!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Drag), SHORTCUT_SELECT), 0);
        app_gestures_sync(&mut a);
        let (mut px, mut py) = (0.0, 0.0);
        to_page(ed(&mut a), 400.0, 600.0, &mut px, &mut py);
        sel_word(&mut a, px, py);
        pen_event(&mut a, InType::PenHover, false, 300.0, 500.0, 1000);
        pen_event(&mut a, InType::PenButton, true, 300.0, 500.0, 1010);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        // the contact: Select mode held, the lasso drawn, no early ink
        let ev = InEvent { type_: InType::PenDown, side_button: true, x: 300.0, y: 500.0, ..Default::default() };
        let mut style = LiveStyle::default();
        editor_live_style(&a, &ev, &mut style);
        assert_eq!(style.enabled, 0);
        pen_event(&mut a, InType::PenDown, true, 300.0, 500.0, 1020);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        assert!(editor_pen_not_ink(&a));
        let ring = [(400.0, 495.0), (500.0, 500.0), (505.0, 600.0), (500.0, 700.0), (400.0, 705.0), (300.0, 700.0), (295.0, 600.0), (300.0, 510.0)];
        for (k, &(x, y)) in ring.iter().enumerate() {
            pen_event(&mut a, InType::PenMove, true, x, y, 1030 + 10 * k as u32);
        }
        pen_event(&mut a, InType::PenUp, true, 300.0, 510.0, 1200);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        assert_eq!(ed_ref(&a).sel.ids.len(), 2);
        // the button let go: the selection stays for the bar
        pen_event(&mut a, InType::PenButton, false, 300.0, 510.0, 1300);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        // a tap outside it: Select mode ends, and the tap leaves no ink
        sel_lasso_free_tap(&mut a, 800.0, 1100.0, 1400);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 2);
        // a held lasso around nothing: Select mode ends with the contact
        pen_event(&mut a, InType::PenButton, true, 800.0, 1100.0, 1500);
        sel_lasso(&mut a, 800.0, 1100.0, 60.0, true, 1510);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        pen_event(&mut a, InType::PenButton, false, 800.0, 1100.0, 1700);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 2);
        // a click still toggles: Select mode stays on after a click bound to `select`
        assert!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Tap), SHORTCUT_SELECT) >= 0);
        app_gestures_sync(&mut a);
        a.pb_config.waits = [false; 3];
        pen_event(&mut a, InType::PenButton, true, 800.0, 1100.0, 2000);
        pen_event(&mut a, InType::PenButton, false, 800.0, 1100.0, 2050);
        assert_eq!(ed_ref(&a).sel.state, SelState::Armed);
        assert!(!ed_ref(&a).sel.by_hold);
    }

    fn sel_lasso_free_tap(a: &mut App, x: f32, y: f32, t: u32) {
        pen_event(a, InType::PenDown, false, x, y, t);
        pen_event(a, InType::PenUp, false, x, y, t + 30);
    }

    // `select` on a pen button's hold: Select mode while it is held (the lasso drawn under it); let go with
    // nothing selected, the pen draws again.
    #[test]
    fn select_held_on_a_pen_hold() {
        let mut a = fshape_app();
        assert!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Hold), SHORTCUT_SELECT) >= 0);
        app_gestures_sync(&mut a);
        pb_out(&mut a, PbOut::HoldBegin(PenBtn::Button), 1000);
        assert_eq!(ed_ref(&a).sel.state, SelState::Armed);
        assert!(ed_ref(&a).sel.holding);
        pb_out(&mut a, PbOut::HoldEnd(PenBtn::Button), 1500);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        // with a selection made under it, Select mode stays after the release
        let (mut px, mut py) = (0.0, 0.0);
        to_page(ed(&mut a), 400.0, 600.0, &mut px, &mut py);
        sel_word(&mut a, px, py);
        pb_out(&mut a, PbOut::HoldBegin(PenBtn::Button), 2000);
        pen_event(&mut a, InType::PenHover, false, 300.0, 500.0, 2010);
        sel_lasso(&mut a, 400.0, 600.0, 100.0, false, 2020);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        pb_out(&mut a, PbOut::HoldEnd(PenBtn::Button), 2300);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        // pens reset (sleep): a held Select is let go
        pb_out(&mut a, PbOut::HoldBegin(PenBtn::Button), 2400);
        pb_reset(ed(&mut a));
        assert!(!ed_ref(&a).sel.holding);
    }

    #[test]
    fn select_held_leaves_the_copy_picker_and_sheets_alone() {
        let mut a = fshape_app();
        assert!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Hold), SHORTCUT_SELECT) >= 0);
        app_gestures_sync(&mut a);
        // Copy to notebook...'s picker up (the Pages view's): a held select neither starts nor closes it
        ed(&mut a).destination.open = true;
        ed(&mut a).destination.copying = true;
        pb_out(&mut a, PbOut::HoldBegin(PenBtn::Button), 1000);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        assert!(ed_ref(&a).destination.open && ed_ref(&a).destination.copying);
        pb_out(&mut a, PbOut::HoldEnd(PenBtn::Button), 1500);
        // and dropping link making keeps it too
        lm_drop(ed(&mut a));
        assert!(ed_ref(&a).destination.open && ed_ref(&a).destination.copying);
        ed(&mut a).destination = NotebookPicker::default();
        // a selection: its box and bar draw over the page, but not under a sheet, where they cannot be tapped
        let (mut px, mut py) = (0.0, 0.0);
        to_page(ed(&mut a), 400.0, 600.0, &mut px, &mut py);
        sel_word(&mut a, px, py);
        pb_out(&mut a, PbOut::HoldBegin(PenBtn::Button), 2000);
        pen_event(&mut a, InType::PenHover, false, 300.0, 500.0, 2010);
        sel_lasso(&mut a, 400.0, 600.0, 100.0, false, 2020);
        pb_out(&mut a, PbOut::HoldEnd(PenBtn::Button), 2300);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        let full = Rect::new(0, 0, a.screen.w, a.screen.h);
        let before = a.screen.px.clone();
        sel_draw(&mut a, full);
        assert!(a.screen.px != before, "the selection draws over the page");
        ed(&mut a).lb.open = true;
        let before = a.screen.px.clone();
        sel_draw(&mut a, full);
        assert!(a.screen.px == before, "nothing of the selection over the Links bar");
        // a held select under the open bar does nothing either
        pb_out(&mut a, PbOut::HoldBegin(PenBtn::Button), 2400);
        assert!(!ed_ref(&a).sel.holding);
    }

    #[test]
    fn pen_drag_blocks_early_live_ink_and_dispatches_once() {
        let mut a = fshape_app();
        assert_eq!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Drag), SHORTCUT_ZOOM_IN), 0);
        app_gestures_sync(&mut a);
        let ink = ed_ref(&a).page_canvas.px.clone();
        let ev = InEvent { type_: InType::PenDown, side_button: true, x: 500.0, y: 700.0, ..Default::default() };
        let mut style = LiveStyle::default();
        editor_live_style(&a, &ev, &mut style);
        assert_eq!(style.enabled, 0);
        pen_event(&mut a, InType::PenHover, false, 500.0, 700.0, 1000);
        pen_event(&mut a, InType::PenButton, true, 500.0, 700.0, 1010);
        pen_event(&mut a, InType::PenDown, true, 500.0, 700.0, 1020);
        assert_eq!(ed_ref(&a).owner, Owner::PenDrag);
        assert!(editor_pen_not_ink(&a));
        assert_eq!(ed_ref(&a).zoom, 1.25);
        pen_event(&mut a, InType::PenMove, true, 600.0, 800.0, 1220);
        assert_eq!(ed_ref(&a).zoom, 1.25);
        // Letting the button go swallows the rest of this contact too.
        pen_event(&mut a, InType::PenButton, false, 600.0, 800.0, 1230);
        assert_eq!(ed_ref(&a).owner, Owner::PenDrag);
        pen_event(&mut a, InType::PenMove, false, 700.0, 900.0, 1240);
        pen_event(&mut a, InType::PenUp, false, 700.0, 900.0, 1250);
        assert_eq!(ed_ref(&a).owner, Owner::None);
        assert_eq!(ed_ref(&a).page_canvas.px, ink);
        assert!(ed_ref(&a).open_page().unwrap().strokes.is_empty());
    }

    #[test]
    fn pen_momentary_eraser_starts_on_press_and_preserves_a_key_hold() {
        let mut a = fshape_app();
        assert_eq!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Hold), SHORTCUT_HOLD_ERASER), SHORTCUT_CLEAR_PAGE);
        app_gestures_sync(&mut a);
        pen_event(&mut a, InType::PenHover, false, 400.0, 600.0, 1000);
        pen_event(&mut a, InType::PenButton, true, 400.0, 600.0, 1010);
        assert!(ed_ref(&a).hold_eraser);
        editor_hold_eraser(&mut a, true);
        pen_event(&mut a, InType::PenButton, false, 400.0, 600.0, 1100);
        assert!(ed_ref(&a).hold_eraser);
        editor_hold_eraser(&mut a, false);
        assert!(!ed_ref(&a).hold_eraser);
        pen_event(&mut a, InType::PenButton, true, 400.0, 600.0, 1200);
        assert!(ed_ref(&a).hold_eraser);
        pen_event(&mut a, InType::PenLeave, false, 400.0, 600.0, 1210);
        assert!(!ed_ref(&a).hold_eraser);
    }

    #[test]
    fn pen_pan_and_zoom_keep_the_anchor_and_finish_with_ui() {
        for action in [GESTURE_PAN, GESTURE_ZOOM] {
            let mut a = fshape_app();
            assert_eq!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Drag), action), 0);
            app_gestures_sync(&mut a);
            let s = ed(&mut a);
            s.zoom = 2.0; s.ox = 100.0; s.oy = 100.0;
            pen_event(&mut a, InType::PenHover, false, 500.0, 700.0, 1000);
            pen_event(&mut a, InType::PenButton, true, 500.0, 700.0, 1010);
            pen_event(&mut a, InType::PenDown, true, 500.0, 700.0, 1020);
            a.dirty = RECT_EMPTY;
            pen_event(&mut a, InType::PenMove, true, 550.0, 650.0, 1100);
            assert!(rect_is_empty(a.dirty));
            pen_event(&mut a, InType::PenMove, true, 600.0, 600.0, 1220);
            assert_eq!(a.dirty_mode, RefreshMode::Fast);
            let s = ed_ref(&a);
            if action == GESTURE_PAN {
                assert_eq!((s.zoom, s.ox, s.oy), (2.0, 50.0, 150.0));
            } else {
                assert!(s.zoom > 2.0);
                assert!((s.ox + 500.0 / s.zoom - 350.0).abs() < 0.001);
                assert!((s.oy + 700.0 / s.zoom - 450.0).abs() < 0.001);
            }
            pen_event(&mut a, InType::PenUp, true, 600.0, 600.0, 1230);
            assert_eq!(a.dirty_mode, RefreshMode::Ui);
            assert_eq!(ed_ref(&a).nb.page_ids, [1, 2]);
            assert!(ed_ref(&a).open_page().unwrap().strokes.is_empty());
        }
    }

    #[test]
    fn pen_page_turn_never_adds_a_page_and_preserves_map_keyboard() {
        let mut a = fshape_app();
        assert_eq!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Tap), SHORTCUT_NEXT_PAGE), 0);
        ed(&mut a).index = 1; ed(&mut a).page = Some(2);
        a.settings.gesture_new_page = true;
        pb_out(&mut a, PbOut::Fire(PenBtn::Button, PenKind::Tap), 1000);
        assert_eq!(ed_ref(&a).nb.page_ids, [1, 2]);
        // (it stays on the last page, silently: no toast since the owner's toast pass)
        assert_eq!(ed_ref(&a).index, 1);
        assert!(!a.toast_visible);
        let s = ed(&mut a);
        s.map.open = true;
        keyboard_open(&mut s.map_kb, "Name", b"draft");
        assert_eq!(gesturemap_set(&mut a.gestures, pb_key(PenBtn::Button, PenKind::Tap), SHORTCUT_UNDO), SHORTCUT_NEXT_PAGE);
        pb_out(&mut a, PbOut::Fire(PenBtn::Button, PenKind::Tap), 1100);
        assert!(ed_ref(&a).map.open && ed_ref(&a).map_kb.open);
        assert_eq!(ed_ref(&a).map_kb.text.as_str(), "draft");
    }

    #[test]
    fn pen_bound_drag_defers_feedback_until_the_nib_lifts() {
        let mut a = fshape_app();
        a.pen_action_down = true;
        a.dirty = Rect::new(10, 10, 20, 20);
        a.dirty_mode = RefreshMode::Ui;
        let previous = a.toast;
        app_toast(&mut a, "Undo", 1200);
        assert_eq!(a.toast, previous);
        assert!(a.pen_toast_pending.is_some());
        app_flush(&mut a);
        assert_eq!(a.dirty, Rect::new(10, 10, 20, 20));
        app_pen_feedback_end(&mut a);
        assert!(!a.pen_action_down && a.pen_toast_pending.is_none());
        assert_eq!(a.toast.as_str(), "Undo");
    }

    #[test]
    fn fshape_swipe_releases_owner_and_cleans_only_the_screen() {
        let mut a = fshape_app();
        let ink = ed_ref(&a).page_canvas.px.clone();
        fshape_event(&mut a, InType::TouchDown, 200.0, 500.0, 1000);
        assert_eq!(ed_ref(&a).fs.state, FsState::Pending);
        for i in 1..=10 {
            fshape_event(&mut a, InType::TouchMove, 200.0 + i as f32 * 50.0, 500.0, 1000 + i * 30);
        }
        assert!(!rect_is_empty(ed_ref(&a).fs_trail));
        fshape_event(&mut a, InType::TouchUp, 700.0, 500.0, 1330);
        assert_eq!(ed_ref(&a).owner, Owner::None);
        assert_eq!(ed_ref(&a).fs.state, FsState::Idle);
        assert!(rect_is_empty(ed_ref(&a).fs_trail));
        assert_eq!(ed_ref(&a).zoom, 1.25);
        assert_eq!(ed_ref(&a).page_canvas.px, ink);
        let mut out = String::new();
        assert_eq!(editor_probe(&mut a, "fshape.last", &mut out, 128), 0);
        assert_eq!(out, "fired zoom-in 0 0 0");
        editor_probe(&mut a, "fshape.fired", &mut out, 128);
        assert_eq!(out, "1");
        editor_probe(&mut a, "fshape.fired", &mut out, 128);
        assert_eq!(out, "0");
    }

    #[test]
    fn fshape_reject_thins_then_fades_and_ticks_wait_for_input() {
        let mut a = fshape_app();
        fshape_event(&mut a, InType::TouchDown, 250.0, 400.0, 1000);
        let path = [(350,400), (450,400), (550,400), (650,400), (650,500), (650,600),
            (650,700), (550,700), (450,700), (350,700), (250,700), (250,600), (250,500), (250,400)];
        for (i, (x, y)) in path.into_iter().enumerate() {
            fshape_event(&mut a, InType::TouchMove, x as f32, y as f32, 1030 + i as u32 * 30);
        }
        a.input_drained = false;
        editor_tick(&mut a, 10000);
        assert_eq!(ed_ref(&a).fs.state, FsState::Drawing);
        fshape_event(&mut a, InType::TouchUp, 250.0, 400.0, 1450);
        assert_eq!(ed_ref(&a).fs_last_reason, FsReason::NoMatch);
        assert!(ed_ref(&a).fs_thinned && ed_ref(&a).fs_reject_pending);
        a.input_drained = true;
        editor_tick(&mut a, 1699);
        assert!(!rect_is_empty(ed_ref(&a).fs_trail));
        editor_tick(&mut a, 1700);
        assert!(rect_is_empty(ed_ref(&a).fs_trail));
        assert_eq!(ed_ref(&a).fs.state, FsState::Idle);
    }

    #[test]
    fn fshape_cancel_waits_for_pinch_but_pen_restores_into_settle() {
        let mut a = fshape_app();
        fshape_event(&mut a, InType::TouchDown, 200.0, 500.0, 1000);
        for i in 1..=6 {
            fshape_event(&mut a, InType::TouchMove, 200.0 + i as f32 * 50.0, 500.0, 1000 + i * 30);
        }
        let trail = ed_ref(&a).fs_trail;
        assert!(!rect_is_empty(trail));
        editor_event(&mut a, &InEvent { type_: InType::TouchUp, x: -10000.0, y: -10000.0,
            value: TOUCH_F_CANCEL, t_ms: 1200, ..Default::default() });
        assert_eq!(ed_ref(&a).fs_pinch_trail, trail);
        assert_eq!(ed_ref(&a).fs_last_kind, FS_OUT_CANCEL);
        fshape_event(&mut a, InType::PenHover, 700.0, 600.0, 1210);
        assert!(rect_is_empty(ed_ref(&a).fs_pinch_trail));
        assert!(!rect_is_empty(ed_ref(&a).settle));
        assert_eq!(ed_ref(&a).owner, Owner::None);
    }

    #[test]
    fn fshape_actions_preserve_anchor_and_scroll_zoom_without_adding_pages() {
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.zoom = 2.0;
        s.ox = 100.0;
        s.oy = 100.0;
        assert!(editor_shortcut_at(&mut a, SHORTCUT_ZOOM_IN, 500.0, 700.0, ActionSrc::Shape));
        let s = ed_ref(&a);
        assert_eq!(s.zoom, 2.5);
        assert!((s.ox + 500.0 / s.zoom - 350.0).abs() < 0.001);
        assert!((s.oy + 700.0 / s.zoom - 450.0).abs() < 0.001);
        ed(&mut a).oy = 800.0;
        assert!(editor_shortcut(&mut a, SHORTCUT_SCROLL_DOWN));
        assert_eq!((ed_ref(&a).index, ed_ref(&a).zoom, ed_ref(&a).oy), (1, 2.5, 0.0));
        // This in-memory notebook has no on-disk info file to persist between turns.
        ed(&mut a).info_dirty = false;
        assert!(editor_shortcut(&mut a, SHORTCUT_SCROLL_UP));
        assert_eq!((ed_ref(&a).index, ed_ref(&a).zoom, ed_ref(&a).oy), (0, 2.5, 840.0));
        ed(&mut a).index = 1;
        ed(&mut a).page = Some(2);
        a.settings.gesture_new_page = true;
        assert!(editor_shortcut_at(&mut a, SHORTCUT_NEXT_PAGE, 500.0, 700.0, ActionSrc::Shape));
        assert_eq!(ed_ref(&a).nb.page_ids, [1, 2]);
        ed(&mut a).dm.open = true;
        let invert = a.settings.invert;
        assert!(!editor_shortcut(&mut a, SHORTCUT_INVERT));
        assert_eq!(a.settings.invert, invert);
    }

    #[test]
    fn fshape_pinch_regrab_reanchors_the_committed_zoom() {
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.zoom = 2.0;
        s.ox = 100.0;
        s.oy = 100.0;
        s.pinching = true;
        s.pinch_committed = true;
        let mut ev = InEvent { type_: InType::PinchBegin, x: 200.0, y: 400.0,
            x2: 600.0, y2: 400.0, value: 1, t_ms: 1000, ..Default::default() };
        editor_event(&mut a, &ev);
        ev.type_ = InType::PinchMove;
        ev.x2 = 640.0;
        ev.t_ms = 1200;
        editor_event(&mut a, &ev);
        assert!((ed_ref(&a).zoom - 2.2).abs() < 0.001);
        assert!(ed_ref(&a).pinch_committed);
    }

    #[test]
    fn fshape_matches_on_the_page_and_obeys_the_links_bar_menu_rules() {
        let mut a = fshape_app();
        let path = [(250,400), (350,400), (450,400), (550,400), (650,400), (650,500),
            (650,600), (650,700), (550,700), (450,700), (350,700), (250,700), (250,600), (250,500), (250,400)];
        let pts: Vec<FsPt> = path.iter().map(|&(x, y)| FsPt { x, y }).collect();
        let mut sample = FsSample::default();
        fs_sample_of(&pts, &mut sample);
        assert_eq!(gesturemap_shape_set(&mut a.gestures, SHORTCUT_ZOOM_IN, &[sample; 3]), 0);
        let draw = |a: &mut App, base: u32| {
            fshape_event(a, InType::TouchDown, 250.0, 400.0, base);
            for (i, &(x, y)) in path.iter().enumerate().skip(1) {
                fshape_event(a, InType::TouchMove, x as f32, y as f32, base + i as u32 * 30);
            }
            fshape_event(a, InType::TouchUp, 250.0, 400.0, base + 450);
        };
        draw(&mut a, 1000);
        assert_eq!(ed_ref(&a).fs_last_kind, FS_OUT_FIRED);
        assert_eq!(ed_ref(&a).zoom, 1.25);
        bar_open(&mut a);
        draw(&mut a, 2000);
        assert!(ed_ref(&a).lb.open);
        assert_eq!(ed_ref(&a).fs_last_kind, FS_OUT_REJECT);
        assert!(ed_ref(&a).fs_reject_pending);
        assert_eq!(gesturemap_shape_set(&mut a.gestures, SHORTCUT_LINKS, &[sample; 3]), SHORTCUT_ZOOM_IN);
        draw(&mut a, 3000);
        assert_eq!(ed_ref(&a).fs_last_kind, FS_OUT_FIRED);
        assert_eq!(ed_ref(&a).fs_last_action, SHORTCUT_LINKS);
        assert!(!ed_ref(&a).lb.open);
        assert_eq!(ed_ref(&a).index, 0);
    }

    // A finger shape names its action only when the action did something (2.0.0-beta.2, the owner: "Show the
    // action's name only when the action actually did something"): Previous page on page 1, Next page on the last
    // page (a shape adds no page there, even with swiping past the last page adding one), Clear page on an empty
    // page, Undo or Redo with nothing to undo or redo, and Close menu with nothing open fire and say nothing; so
    // does Redo right after a stroke, whose settle is no change of the app's. Done, each names its action as
    // before, and so does a setting that changes with nothing drawn.
    #[test]
    fn fshape_names_its_action_only_when_it_did_something() {
        // (a data root of its own: the setting it changes is saved, settings.ini there)
        let root = std::env::temp_dir().join(format!("grimoire-fshape-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let mut a = fshape_app();
        let path = [(250,400), (350,400), (450,400), (550,400), (650,400), (650,500),
            (650,600), (650,700), (550,700), (450,700), (350,700), (250,700), (250,600), (250,500), (250,400)];
        let pts: Vec<FsPt> = path.iter().map(|&(x, y)| FsPt { x, y }).collect();
        let mut sample = FsSample::default();
        fs_sample_of(&pts, &mut sample);
        let clock = Cell::new(1000u32);
        // (the square drawn with a finger, bound to `action` alone; the toast it leaves, None for none)
        let fire = |a: &mut App, action: i32| -> Option<String> {
            assert!(gesturemap_shape_set(&mut a.gestures, action, &[sample; 3]) >= 0);
            app_toast_dismiss(a);
            let t = clock.get();
            fshape_event(a, InType::TouchDown, 250.0, 400.0, t);
            for (i, &(x, y)) in path.iter().enumerate().skip(1) {
                fshape_event(a, InType::TouchMove, x as f32, y as f32, t + i as u32 * 30);
            }
            fshape_event(a, InType::TouchUp, 250.0, 400.0, t + 450);
            clock.set(t + 2000);
            assert_eq!((ed_ref(a).fs_last_kind, ed_ref(a).fs_last_action), (FS_OUT_FIRED, action));
            ed(a).info_dirty = false; // (this in-memory notebook has no notebook.ini to write)
            a.toast_visible.then(|| a.toast.as_str().to_string())
        };
        let named = |action: i32| Some(action_label(action).to_string());
        a.settings.gesture_new_page = true;
        // Page 1 of 2, blank: nothing before it, nothing to clear, undo or redo, no menu to close
        assert_eq!(fire(&mut a, SHORTCUT_PREV_PAGE), None);
        assert_eq!(fire(&mut a, SHORTCUT_CLEAR_PAGE), None);
        assert_eq!(fire(&mut a, SHORTCUT_UNDO), None);
        assert_eq!(fire(&mut a, SHORTCUT_REDO), None);
        assert_eq!(fire(&mut a, SHORTCUT_CLOSE_MENU), None);
        assert_eq!(ed_ref(&a).index, 0);
        // Next page: page 2, named; there, the last page, nothing
        assert_eq!(fire(&mut a, SHORTCUT_NEXT_PAGE), named(SHORTCUT_NEXT_PAGE));
        assert_eq!(ed_ref(&a).index, 1);
        assert_eq!(fire(&mut a, SHORTCUT_NEXT_PAGE), None);
        assert_eq!((ed_ref(&a).index, ed_ref(&a).nb.npages()), (1, 2));
        // A stroke on page 2: Redo has nothing to redo (its settle draws what is there already), Undo takes the
        // stroke away and Redo puts it back, each named
        let t = clock.get();
        pen_event(&mut a, InType::PenDown, false, 300.0, 1000.0, t);
        for k in 1..10 {
            pen_event(&mut a, InType::PenMove, false, 300.0 + 30.0 * k as f32, 1000.0, t + 10 * k);
        }
        pen_event(&mut a, InType::PenUp, false, 570.0, 1000.0, t + 100);
        pen_event(&mut a, InType::PenLeave, false, 570.0, 1000.0, t + 110);
        clock.set(t + 2000);
        ed(&mut a).pen_left = now_ms().wrapping_sub(5000);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 1);
        assert!(!rect_is_empty(ed_ref(&a).settle) || ed_ref(&a).panel_stale);
        assert_eq!(fire(&mut a, SHORTCUT_REDO), None);
        assert_eq!(fire(&mut a, SHORTCUT_UNDO), named(SHORTCUT_UNDO));
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 0);
        assert_eq!(fire(&mut a, SHORTCUT_REDO), named(SHORTCUT_REDO));
        assert_eq!(fire(&mut a, SHORTCUT_REDO), None);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 1);
        // Clear page clears it, named; then the page is empty: nothing
        assert_eq!(fire(&mut a, SHORTCUT_CLEAR_PAGE), named(SHORTCUT_CLEAR_PAGE));
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 0);
        assert_eq!(fire(&mut a, SHORTCUT_CLEAR_PAGE), None);
        // Previous page: page 1, named (the cleared page as saved: this in-memory notebook has no file for it)
        ed(&mut a).page_dirty = false;
        assert_eq!(fire(&mut a, SHORTCUT_PREV_PAGE), named(SHORTCUT_PREV_PAGE));
        assert_eq!(ed_ref(&a).index, 0);
        // A setting with nothing drawn (swiping past the last page adds a page, no toast of its own): named
        assert_eq!(fire(&mut a, SHORTCUT_GESTURE_NEW_PAGE), named(SHORTCUT_GESTURE_NEW_PAGE));
        assert!(!a.settings.gesture_new_page);
        assert!(std::fs::read_to_string(root.join("settings.ini")).unwrap().contains("gesture_new_page=0"));
        let _ = std::fs::remove_dir_all(&root);
    }

    // Zoom in at the most zoom, Zoom out at the least, and a scroll past the end of the first or the last page
    // leave the view where it was: they draw nothing, so a finger shape bound to them says nothing either. A zoom
    // or a scroll that moves the view draws it.
    #[test]
    fn zoom_and_scroll_at_their_limits_draw_nothing() {
        let mut a = fshape_app();
        let s = ed(&mut a);
        s.zoom = s.zoom_max;
        view_clamp(&mut a);
        let changes = a.changes;
        assert!(editor_shortcut_at(&mut a, SHORTCUT_ZOOM_IN, 500.0, 700.0, ActionSrc::Shape));
        assert_eq!(a.changes, changes);
        assert!(editor_shortcut_at(&mut a, SHORTCUT_ZOOM_OUT, 500.0, 700.0, ActionSrc::Shape));
        assert_ne!(a.changes, changes);
        let s = ed(&mut a);
        s.zoom = s.zoom_min;
        view_clamp(&mut a);
        let changes = a.changes;
        assert!(editor_shortcut_at(&mut a, SHORTCUT_ZOOM_OUT, 500.0, 700.0, ActionSrc::Shape));
        assert_eq!(a.changes, changes);
        // Page 1 of 2, its top on show: up goes nowhere; zoomed in, down moves the view
        view_reset(&mut a);
        let changes = a.changes;
        assert!(editor_shortcut(&mut a, SHORTCUT_SCROLL_UP));
        assert_eq!((a.changes, ed_ref(&a).index), (changes, 0));
        ed(&mut a).zoom = 2.0;
        view_clamp(&mut a);
        let (changes, oy) = (a.changes, ed_ref(&a).oy);
        assert!(editor_shortcut(&mut a, SHORTCUT_SCROLL_DOWN));
        assert!(a.changes != changes && ed_ref(&a).oy > oy && ed_ref(&a).index == 0);
    }

    #[test]
    fn clock_corner_reserves_no_page_strip_at_any_zoom() {
        for (w, h) in [(1860, 2480), (2480, 1860)] {
            let mut a = app(w, h, 1860, 2480);
            a.scr = ScreenId::Editor;
            ed(&mut a).open = true;
            assert_eq!(canvas_init(&mut ed(&mut a).page_canvas, 1860, 2480), 0);
            a.settings.landscape_fit=true;
            let top = status_reserved(&a);
            assert_eq!(top, 0);
            view_reset(&mut a);
            assert!(!zoomed(&a));
            let s = ed_ref(&a);
            assert!((s.zoom - (w as f32 / 1860.0).min((h-top) as f32 / 2480.0)).abs() < 1e-6);
            let (mut x, mut y) = (0.0, 0.0);
            to_page(s, w as f32 / 2.0, top as f32, &mut x, &mut y);
            assert!(y.abs() < 0.001);
            ed(&mut a).zoom *= 2.0;
            view_clamp(&mut a);
            let s = ed_ref(&a);
            to_page(s, w as f32 / 2.0, top as f32, &mut x, &mut y);
            assert!(y >= -0.001);
            assert!(s.oy >= -(top as f32) / s.zoom - 0.001);
        }
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
    fn sideways_the_whole_page_rests_when_asked() {
        // Off (or upright, whatever it says) the page rests as it always did: its width across, the
        // ceiling four times that, bit for bit, on the three screens' shapes.
        for (aw, ah, pw, ph) in [(1860, 2480, 1860, 2480), (2480, 1860, 1860, 2480), (1986, 2648, 1986, 2648), (2648, 1986, 1986, 2648),
        ] {
            let width = aw as f32 / pw as f32;
            for fit in [false, true] {
                let (z, zmin, zmax) = view_rest(aw, ah, pw, ph, fit);
                assert_eq!(zmax.to_bits(), (width * 4.0).to_bits());
                assert_eq!(zmin.to_bits(), width.min(ah as f32 / ph as f32).to_bits());
                if !fit || aw < ah {
                    assert_eq!(z.to_bits(), width.to_bits());
                } else {
                    assert_eq!(z.to_bits(), zmin.to_bits());
                }
            }
        }
        // Off, sideways: the width across, from the top, at rest (the existing view).
        let mut a = app(2480, 1860, 1860, 2480);
        a.settings.landscape_fit = false;
        view_reset(&mut a);
        let s = ed_ref(&a);
        assert_eq!((s.zoom, s.zoom_default, s.zoom_max, s.rest_ox), (2480.0 / 1860.0, 2480.0 / 1860.0, 2480.0 / 1860.0 * 4.0, 0.0));
        assert!(!zoomed(&a));
        // On, sideways: the whole page, centred across, and that is rest: not zoomed until it moves.
        let mut a = app(2480, 1860, 1860, 2480);
        a.settings.landscape_fit = true;
        view_reset(&mut a);
        let s = ed_ref(&a);
        assert_eq!(s.zoom, s.zoom_min);
        assert_eq!(s.zoom, 0.75);
        assert_eq!(s.zoom_default, 0.75);
        assert_eq!(s.zoom_max, 2480.0 / 1860.0 * 4.0);
        assert!(s.ox < 0.0 && s.rest_ox == s.ox && s.oy == 0.0);
        assert!(!zoomed(&a));
        // Zoomed in, then out to the whole page again: still at rest.
        let s = ed(&mut a);
        s.zoom = 2.0;
        view_clamp(&mut a);
        assert!(zoomed(&a));
        let s = ed(&mut a);
        s.zoom = 0.1;
        view_clamp(&mut a);
        assert!(!zoomed(&a));
        // A pan away from rest is zoomed.
        let s = ed(&mut a);
        s.zoom = 1.5;
        s.oy = 200.0;
        view_clamp(&mut a);
        assert!(zoomed(&a));
        // Nothing may zoom past four times the width's fit.
        let s = ed(&mut a);
        s.zoom = 100.0;
        view_clamp(&mut a);
        assert_eq!(ed_ref(&a).zoom, 2480.0 / 1860.0 * 4.0);
        // Upright with it on is today's view.
        let mut a = app(1860, 2480, 1860, 2480);
        a.settings.landscape_fit = true;
        view_reset(&mut a);
        assert!(view_identity(&a) && !zoomed(&a));
    }

    #[test]
    fn a_peeked_page_rests_where_the_open_page_would() {
        // The view the bar shows a page at is the one view_reset would give it as the open page, on every
        // screen and page shape, with the whole page sideways on and off.
        for (aw, ah, pw, ph) in [(1860, 2480, 1860, 2480), (2480, 1860, 1860, 2480), (1986, 2648, 1986, 2648), (2648, 1986, 1986, 2648), (1860, 2480, 1986, 2648), (2480, 1860, 1986, 2648), (1986, 2648, 1860, 2480),
        ] {
            for fit in [false, true] {
                let mut a = app(aw, ah, pw, ph);
                a.settings.landscape_fit = fit;
                view_reset(&mut a);
                let s = ed_ref(&a);
                let (z, ox, oy) = peek_view(aw, ah, pw, ph, fit);
                assert_eq!((z.to_bits(), ox.to_bits(), oy.to_bits()), (s.zoom.to_bits(), s.ox.to_bits(), s.oy.to_bits()), "{aw}x{ah} page {pw}x{ph} fit {fit}");
            }
        }
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
        for (w, h, pins) in [(1860, 2480, 0), (2480, 1860, 0), (2480, 1860, 3), (1860, 2480, 3),
        ] {
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
            // Inside the screen, fitted with room for its margins (though it may go up to the edges).
            let m = ui_px(PANEL_MARGIN as f32);
            assert!(p.x >= 0 && p.x + p.w <= w && p.y >= 0 && p.y + p.h <= h && p.h + 2 * m <= h);
            assert_eq!(p.h, panel_height(g_fit(), pins as i32));
            // The buttons from the top: grip, collapse, the tool button, the pins, the eraser, (undo,
            // redo, clear and previous greyed out), the page chip, next, links, paper, display, pen feel,
            // the library, whose gap under it ends the panel but for its last 6 px.
            let ids: Vec<i32> = s.hits.h.iter().map(|h| h.id).collect();
            let mut want = vec![P_GRIP, P_COLLAPSE, P_TOOLPICK];
            want.extend(std::iter::repeat_n(P_TOOL, pins));
            want.extend([P_TOOL, P_SHAPES, P_PENSET, P_PAGES, P_NEXT, P_LINKS, P_PAPER, P_DISPLAY, P_SETTINGS, P_LIBRARY]);
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
        assert_eq!(p, Rect::new(1860 - panel_box(), 0, panel_box(), panel_box()));
        ed(&mut a).panel = p;
        draw_panel(&mut a);
        let s = ed_ref(&a);
        assert_eq!(s.hits.h.len(), 1);
        assert_eq!((s.hits.h[0].id, s.hits.h[0].r), (P_EXPAND, p));
        assert_eq!(s.tool_button_shows, TB_NONE);
    }

    #[test]
    fn live_views_chip_sits_beside_the_panel_and_goes_with_a_menu() {
        let mut a = app(1860, 2480, 1860, 2480);
        // No notebook open: no chip on the page.
        assert_eq!(editor_live_chip(&a, 42), RECT_EMPTY);
        let s = ed(&mut a);
        s.open = true;
        s.panel = Rect::new(1704, 220, 128, 1520);
        fit_panel_to(&a.settings, a.h);
        // Left of the panel, level with its header (80 tall: the chip's middle is the header's).
        let c = editor_live_chip(&a, 42);
        assert_eq!(c, Rect::new(1704 - 8 - 42, 220 + (head_h() - 42) / 2, 42, 42));
        assert!(!rect_overlaps(c, ed_ref(&a).panel));
        // It follows the panel.
        ed(&mut a).panel = Rect::new(900, 700, 128, 1520);
        assert_eq!(editor_live_chip(&a, 42).x, 900 - 8 - 42);
        assert_eq!(editor_live_chip(&a, 42).y, 700 + (head_h() - 42) / 2);
        // At the screen's left edge there is no room on its left: it is on its right.
        ed(&mut a).panel = Rect::new(10, 700, 128, 1520);
        let c = editor_live_chip(&a, 42);
        assert_eq!(c.x, 10 + 128 + 8);
        assert!(!rect_overlaps(c, ed_ref(&a).panel));
        // Folded, the box is one button: level with all of it.
        let s = ed(&mut a);
        s.collapsed = true;
        s.panel = Rect::new(1700, 220, box_(), box_());
        let c = editor_live_chip(&a, 42);
        assert_eq!((c.x, c.y), (1700 - 8 - 42, 220 + (box_() - 42) / 2));
        // Always on the screen.
        ed(&mut a).panel = Rect::new(0, 0, box_(), box_());
        let c = editor_live_chip(&a, 42);
        assert!(rect_contains_rect(Rect::new(0, 0, 1860, 2480), c));
        // A menu, sheet, bar or the Pages view in front takes it away (the panel's Live layer stays, holding the menu
        // over it as the screen does, but under the Pages view: the_live_toolbar_layer_stays_...).
        ed(&mut a).collapsed = false;
        ed(&mut a).panel = Rect::new(1704, 220, 128, 1520);
        assert_ne!(editor_live_panel(&a), RECT_EMPTY);
        assert_eq!(editor_live_hole(&a), Rect::new(1704, 220, 128, 1520));
        for open in [
            |s: &mut EditorState, on: bool| s.pv.open = on,
            // (the tool picker over the panel)
            |s: &mut EditorState, on: bool| { s.tools_open = on; s.tools_rect = if on { s.panel } else { RECT_EMPTY }; },
            |s: &mut EditorState, on: bool| s.mb.open = on,
            |s: &mut EditorState, on: bool| s.dm.open = on,
            |s: &mut EditorState, on: bool| s.ts.open = on,
            |s: &mut EditorState, on: bool| s.paper_open = on,
            |s: &mut EditorState, on: bool| s.lb.open = on,
            |s: &mut EditorState, on: bool| s.find.open = on,
        ] {
            assert!(editor_page_in_front(&a));
            open(ed(&mut a), true);
            assert!(!editor_page_in_front(&a));
            assert_eq!(editor_live_chip(&a, 42), RECT_EMPTY);
            assert_eq!(editor_live_panel(&a) == RECT_EMPTY, ed_ref(&a).pv.open);
            // (the frame is still pushed without the panel under a menu, but for the Pages view, which has none)
            assert_eq!(editor_live_hole(&a) == RECT_EMPTY, ed_ref(&a).pv.open);
            open(ed(&mut a), false);
            assert_ne!(editor_live_chip(&a, 42), RECT_EMPTY);
            assert_ne!(editor_live_panel(&a), RECT_EMPTY);
        }
    }

    #[test]
    fn the_panel_buttons_have_the_runners_names() {
        let h = |id: i32, arg: i32| Hit { r: RECT_EMPTY, id, arg ,
        };
        assert_eq!(panel_name(&h(P_TOOLPICK, 0)), "tool-picker");
        assert_eq!(panel_name(&h(P_TOOL, TOOL_ERASER.0)), "eraser");
        assert_eq!(panel_name(&h(P_TOOL, TOOL_HIGHLIGHTER.0)), "tool:highlighter");
        assert_eq!(panel_name(&h(P_TOOL, TOOL_FINELINER.0)), "tool:fineliner");
        for (id, name) in [
            (P_GRIP, "grip"),
            (P_COLLAPSE, "collapse"),
            (P_EXPAND, "expand"),
            (P_UNDO, "undo"),
            (P_REDO, "redo"),
            (P_CLEAR, "clear"),
            (P_SHAPES, "shapes"),
            (P_PREV, "previous"),
            (P_PAGES, "pages"),
            (P_NEXT, "next"),
            (P_LINKS, "links"),
            (P_PAPER, "paper"),
            (P_DISPLAY, "display"),
            (P_PENSET, "pen-feel"),
            (P_SETTINGS, "settings"),
            (P_LIBRARY, "library"),
        ] {
            assert_eq!(panel_name(&h(id, 0)), name);
        }
        // a sheet's hit is no panel button
        assert_eq!(panel_name(&h(P_PICK_PAPER, 0)), "");
    }

    #[test]
    fn landscape_panel_uses_the_blank_margin_on_either_side_only_with_full_page_on() {
        for collapsed in [false,true] {for bar in [0,1] {
            let mut a=app(2480,1860,1860,2480);
            a.settings.status_mode=bar;a.settings.landscape_fit=true;
            ed(&mut a).collapsed=collapsed;ed(&mut a).open=true;
            view_reset(&mut a);
            let width=if collapsed{box_()}else{pw()};
            let left=landscape_panel_slot(&a,true).unwrap();let right=landscape_panel_slot(&a,false).unwrap();
            let s=ed_ref(&a);
            let page_left=(-s.ox*s.zoom).floor()as i32;
            let page_right=((s.pgw as f32-s.ox)*s.zoom).ceil()as i32;
            assert_eq!(page_left-left-width,ui_px(24.0));
            assert_eq!(right-page_right,ui_px(24.0));
            assert!(left>=0 && right+width<=a.w);
            a.settings.landscape_fit=false;
            assert_eq!(landscape_panel_slot(&a,true),None);
            assert_eq!(landscape_panel_slot(&a,false),None);
            a.w=1860;a.h=2480;a.settings.landscape_fit=true;
            assert_eq!(landscape_panel_slot(&a,true),None);
        }}
    }

    #[test]
    fn a_snapped_link_try_that_was_too_small_leaves_no_outline_for_the_next_lasso() {
        let root=std::env::temp_dir().join(format!("grimoire-link-outline-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);store_init(root.to_str().unwrap());
        let mut nb=NotebookInfo::default();assert_eq!(store_create("Void","",PAPER_BLANK,&mut nb),0);
        assert_eq!(store_add_page(&mut nb,1,PAPER_BLANK),0);
        let mut a=fshape_app();
        a.settings.shapes=false;
        let s=ed(&mut a);s.nb=nb;nav_init(&mut s.nav);s.nav.links_writable=true;
        link_arm(&mut a,false,None);
        assert_eq!(ed_ref(&a).lm.state,LmState::Armed);
        // A small circle snapped (the hold), then lifted: too small for a link.
        lasso_begin(&mut a,&InEvent{type_:InType::PenDown,x:300.0,y:300.0,..Default::default()});
        let circle:Vec<(f32,f32)>=(0..24).map(|i|{let t=i as f32*std::f32::consts::TAU/24.0;
            (300.0+10.0*t.cos(),300.0+10.0*t.sin())}).collect();
        let s=ed(&mut a);s.lm.pts=circle.clone();s.lm.shape_preview=Some(circle);s.lm.shape_kind=ShapeKind::Circle;
        lasso_end(&mut a);
        assert_eq!(ed_ref(&a).lm.state,LmState::Armed);
        assert_eq!(ed_ref(&a).lm.outline.n,0);
        // A freehand lasso around something else: its link keeps a plain box, and saves.
        lasso_begin(&mut a,&InEvent{type_:InType::PenDown,x:500.0,y:600.0,..Default::default()});
        let s=ed(&mut a);s.lm.bx0=500.0;s.lm.by0=600.0;s.lm.bx1=800.0;s.lm.by1=900.0;
        s.lm.pts=vec![(500.0,600.0),(800.0,600.0),(800.0,900.0),(500.0,900.0)];
        lasso_end(&mut a);
        assert_eq!(ed_ref(&a).lm.state,LmState::Pick);
        assert_eq!(ed_ref(&a).lm.outline.n,0);
        link_pick_as(&mut a,"index",0);
        let s=ed_ref(&a);
        let mut seeds=0;assert_eq!(nav_links_of(&s.nav,1,&mut seeds),1);
        let l=s.nav.links.iter().find(|l|l.pid==1).unwrap();
        assert!(l.has_area);assert_eq!(l.outline.n,0);
        let _=std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_lasso_left_with_no_pen_goes_back_to_the_strip_never_ink() {
        let root=std::env::temp_dir().join(format!("grimoire-link-orphan-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);store_init(root.to_str().unwrap());
        let mut nb=NotebookInfo::default();assert_eq!(store_create("Void","",PAPER_BLANK,&mut nb),0);
        assert_eq!(store_add_page(&mut nb,1,PAPER_BLANK),0);
        let mut a=fshape_app();
        a.settings.shapes=false;
        let s=ed(&mut a);s.nb=nb;nav_init(&mut s.nav);s.nav.links_writable=true;
        link_arm(&mut a,false,None);
        let down=|x:f32,y:f32|InEvent{type_:InType::PenDown,x,y,pressure:0.6,..Default::default()};
        editor_event_in(&mut a,&down(300.0,300.0));
        editor_event_in(&mut a,&InEvent{type_:InType::PenMove,x:600.0,y:300.0,pressure:0.6,..Default::default()});
        assert_eq!((ed_ref(&a).owner,ed_ref(&a).lm.state),(Owner::Lasso,LmState::Lasso));
        // Something took the lasso's pen without ending it (a screen laid out again resets the owner): the
        // next pen down is a lasso of its own, from where it lands, and never ink under the strip.
        ed(&mut a).owner=Owner::None;
        editor_event_in(&mut a,&down(500.0,900.0));
        let s=ed_ref(&a);
        assert_eq!((s.owner,s.owner_is_pen,s.lm.state),(Owner::Lasso,true,LmState::Lasso));
        assert_eq!((s.lm.pts.len(),s.lm.bx0,s.lm.by0),(1,500.0,900.0));
        assert_eq!(s.open_page().unwrap().strokes.len(),0);
        // Its lift lost: the next down cancels it back to the strip and draws its own.
        editor_event_in(&mut a,&down(200.0,1000.0));
        let s=ed_ref(&a);
        assert_eq!((s.owner,s.lm.state,s.lm.pts.len(),s.lm.bx0),(Owner::Lasso,LmState::Lasso,1,200.0));
        assert_eq!(s.open_page().unwrap().strokes.len(),0);
        let _=std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_contact_lines_have_a_budget_a_minute() {
        let mut b=DiagBudget::default();
        for _ in 0..DIAG_LINES {assert!(b.admit(5));}
        assert!(!b.admit(6)); // (says so once)
        for _ in 0..1000 {assert!(!b.admit(59_000));}
        assert!(b.admit(60_005)); // a minute from the first line: a new window
        assert_eq!(b.lines,1);
        assert!(DiagBudget::default().admit(u32::MAX-10)); // (the clock wraps)
    }

    #[test]
    fn owner_names_cover_every_owner() {
        assert_eq!(owner_name(Owner::None),"none");
        assert_eq!(owner_name(Owner::Lasso),"lasso");
        assert_eq!(owner_name(Owner::LinkTo),"linkto");
        assert_eq!(owner_name(Owner::PenDrag),"pen-drag");
        assert_eq!(owner_name(Owner::Select),"select");
        assert_eq!(owner_name(Owner::AddPad),"add-pad");
        assert_eq!(Owner::QuickSize as i32, 22);
        assert_eq!(owner_name(Owner::QuickSize),"quick-size");
    }

    #[test]
    fn link_outline_keeps_paper_and_ink_outside_its_narrow_halo() {
        ui_init(300.0);
        let mut c=Canvas::default();assert_eq!(canvas_init(&mut c,600,600),0);
        canvas_fill(&mut c,Rect::new(0,0,600,600),123);
        let points:Vec<_>=(0..=72).map(|i|{let t=i as f32*std::f32::consts::TAU/72.0;
            (300.0+100.0*t.cos(),300.0+100.0*t.sin())}).collect();
        let outline=LinkOutline::from_points(&points,true,600,600);
        draw_outline(&mut c,outline,Rect::new(200,200,200,200),Rect::new(200,200,200,200),(1.0,0.0,0.0));
        let mut dots=0;
        for y in 0..600 {for x in 0..600 {
            let v=c.px[y*600+x];
            if v!=123 {
                let radius=((x as f32-300.0).powi(2)+(y as f32-300.0).powi(2)).sqrt();
                // (the band's square pen, halo included, reaches half its side times the diagonal, and a pixel more)
                let reach=(link_band()+2*ui_px(1.0)) as f32*0.5*std::f32::consts::SQRT_2+1.5;
                assert!((radius-100.0).abs()<reach,"overlay touched paper far from border at {x},{y}");
                assert!(v==0 || v==255);dots+=usize::from(v==0);
            }
        }}
        assert!(dots>40);
    }

    #[test]
    fn the_shape_sliders_line_up_and_their_knobs_clear_their_values() {
        let _a = app(1860, 2480, 1860, 2480);
        let mut st = Settings::default();
        settings_default(&mut st);
        let band = Rect { x: 598, y: 866, w: 1056, h: 136 };
        for th in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(th);
            let (c, m) = (shape_slider(P_SHAPE_CORNER, band, &st), shape_slider(P_SHAPE_MIN, band, &st));
            assert_eq!((c.track, c.text_w), (m.track, m.text_w));
            // the knob at 0 a little past the end of the value, and each label and value whole
            assert!(c.track.x - ui_slider_knob_radius() >= c.r.x + c.text_w + ui_px(8.0));
            let ts = ui_text_size();
            for (label, value) in [("Corner roundness", "Sharp"), ("Corner roundness", "100%"), ("Smallest shape", "40 mm")] {
                assert!(font_measure(FontFace::Regular, ts, label) + ui_px(16.0) + font_measure(FontFace::Bold, ts, value) <= c.text_w);
            }
            assert!(c.track.w >= ui_px(300.0));
        }
        ui_set_theme(UiTheme::Gothic);
    }

    #[test]
    fn shape_hold_uses_screen_mm_hysteresis_and_burst_time() {
        let mm = mm_to_px(1.0,300.0);
        assert!(!shape_anchor_moved((0.0,0.0),(0.9*mm,0.0),300.0,false));
        assert!(shape_anchor_moved((0.0,0.0),(1.1*mm,0.0),300.0,false));
        assert!(!shape_anchor_moved((0.0,0.0),(1.4*mm,0.0),300.0,true));
        assert!(shape_anchor_moved((0.0,0.0),(1.6*mm,0.0),300.0,true));
        assert_eq!(shape_dwell_ms(550,false),550);
        assert_eq!(shape_dwell_ms(550,true),825);
        assert_eq!(shape_hold_nearest(720),550);
        assert_eq!(shape_hold_nearest(800),900);
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

    // Two cache slots holding pages 11 and 12, shown at the ticks given (a canvas each, so neither is free).
    fn two_cached(shown: [u32; 2], tick: u32) -> EditorState {
        let mut s = EditorState::default();
        for (i, pid) in [11u32, 12].into_iter().enumerate() {
            let e = &mut s.ccache[i];
            assert_eq!(canvas_init(&mut e.canvas, 4, 4), 0);
            e.pid = pid;
            e.used = shown[i];
        }
        s.cache_tick = tick;
        s
    }

    #[test]
    fn the_cache_gives_up_the_page_shown_longest_ago() {
        // The page just shown stays; the older one goes.
        assert_eq!(cache_victim(&two_cached([5, 3], 5), true), Some(1));
        assert_eq!(cache_victim(&two_cached([2, 5], 5), true), Some(0));
        // Across the tick's wrap: slot 0 was shown at u32::MAX, two shows before slot 1's tick 1.
        assert_eq!(cache_victim(&two_cached([u32::MAX, 1], 1), true), Some(0));
        // The page on show is never the one: even when it is the oldest.
        let mut s = two_cached([1, 5], 5);
        s.peek = Peek { active: true, slot: 0, ..Default::default() };
        assert_eq!(cache_victim(&s, true), Some(1));
        // Without leave to evict (the prefetch) a full cache gives nothing; a free slot goes first.
        assert_eq!(cache_victim(&two_cached([1, 5], 5), false), None);
        let mut s = two_cached([5, 3], 5);
        s.ccache[0] = CacheEntry::default();
        assert_eq!(cache_victim(&s, false), Some(0));
    }

    #[test]
    fn the_back_list_never_offers_the_page_on_show() {
        let mut s = EditorState::default();
        s.nb.page_ids = vec![10, 20, 30, 40];
        s.open = true;
        s.index = 0; // page 10 is on show
        // A link followed from it, then the pages turned back by hand: its own entry is not a way back.
        s.back = vec![30, 10];
        assert_eq!(s.back_top(), Some((2, "Page 3".to_string())));
        assert_eq!(s.back, [30]);
        s.back = vec![10, 10];
        assert_eq!(s.back_top(), None);
        assert!(s.back.is_empty());
        // Where it is not at the top, it stays for later (a browser's Back goes through it).
        s.back = vec![10, 30];
        assert_eq!(s.back_top(), Some((2, "Page 3".to_string())));
        assert_eq!(s.back, [10, 30]);
        // Pages deleted since are skipped first, and may leave the page on show on top.
        s.back = vec![30, 10, 99];
        assert_eq!(s.back_top(), Some((2, "Page 3".to_string())));
        assert_eq!(s.back, [30]);
        // With no page open there is no page on show to leave out.
        s.open = false;
        s.back = vec![10];
        assert_eq!(s.back_top(), Some((0, "Page 1".to_string())));
    }

    #[test]
    fn probes_read_atoi_as_c_does() {
        assert_eq!(atoi("12"), 12);
        assert_eq!(atoi(" -3x"), -3);
        assert_eq!(atoi("x"), 0);
        assert_eq!(atoi(""), 0);
    }

    // The lasso (6.4, 6.5): a ring traced many times keeps at most LASSO_MAX points, the spacing doubling as
    // it fills, and the area is the box of every point the pen gave; a point closer than the spacing is not
    // kept; with no room for points the lasso still goes on.
    #[test]
    fn the_lasso_keeps_a_bounded_ring_and_its_whole_box() {
        let mut pts: Vec<(f32, f32)> = Vec::new();
        pts.try_reserve_exact(LASSO_MAX).unwrap();
        let mut spacing = 2.0f32;
        let start = (800.0f32, 1000.0f32);
        let mut last = start;
        pts.push(start);
        let (mut bx0, mut by0, mut bx1, mut by1) = (start.0, start.1, start.0, start.1);
        for i in 0..40_000 {
            let t = i as f32 * 0.01;
            let p = (800.0 + 300.0 * t.cos() - 300.0, 1000.0 + 200.0 * (t * 1.37).sin(),
            );
            bx0 = bx0.min(p.0);
            bx1 = bx1.max(p.0);
            by0 = by0.min(p.1);
            by1 = by1.max(p.1);
            lasso_keep(&mut pts, &mut spacing, &mut last, p, 1.0);
            assert!(pts.len() <= LASSO_MAX);
        }
        assert!(pts.capacity() == LASSO_MAX); // (never grown past its reservation)
        assert!(spacing > 2.0);
        let r = lasso_area(bx0, by0, bx1, by1, 1860, 2480);
        assert_eq!(r, Rect { x: bx0.floor() as i32, y: by0.floor() as i32, w: bx1.ceil() as i32 - bx0.floor() as i32, h: by1.ceil() as i32 - by0.floor() as i32 });
        // the kept points stay within the box, and reach its corners to within the spacing
        let (kx0, kx1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
        assert!(kx0 >= bx0 && kx1 <= bx1 && kx0 - bx0 <= spacing * 2.0 && bx1 - kx1 <= spacing * 2.0);
        // too close to the last point: not kept, and the last point stays
        let mut s2 = 2.0;
        let mut v = Vec::with_capacity(4);
        let mut l2 = (10.0, 10.0);
        assert!(!lasso_keep(&mut v, &mut s2, &mut l2, (11.0, 10.5), 1.0));
        assert!(lasso_keep(&mut v, &mut s2, &mut l2, (13.0, 10.0), 1.0));
        assert_eq!(l2, (13.0, 10.0));
        // zoomed in twice, the same page distance is twice the screen distance
        assert!(lasso_keep(&mut v, &mut s2, &mut l2, (14.1, 10.0), 2.0));
        // no room: nothing kept, the lasso still moves on
        let mut none: Vec<(f32, f32)> = Vec::new();
        let mut l3 = (0.0, 0.0);
        assert!(lasso_keep(&mut none, &mut s2, &mut l3, (50.0, 0.0), 1.0));
        assert!(none.is_empty() && l3 == (50.0, 0.0));
        // the area is clamped to the page
        assert_eq!(lasso_area(-5.5, -1.0, 2000.2, 30.1, 1860, 2480), Rect { x: 0, y: 0, w: 1860, h: 31 });
    }

    #[test]
    fn migration_tracks_new_pages_and_keeps_tried_ids_sorted() {
        let mut s = EditorState::default();
        s.nb.page_ids = vec![30, 10];
        assert!(s.migrate_room());
        s.migrate_tried(30);
        s.nb.page_ids.push(20);
        assert!(s.migrate_room());
        s.migrate_tried(10);
        s.migrate_tried(20);
        s.migrate_tried(10);
        assert_eq!(s.migrate_done, vec![10, 20, 30]);
    }

    // Too small for a link: under 5 mm both ways (59 px at 300 dpi); 5 mm either way is an area.
    #[test]
    fn a_lasso_under_five_mm_both_ways_is_too_small() {
        let min = (LINK_BOX_MIN_MM * 300.0 / 25.4).ceil() as i32; // 60
        assert!(lasso_too_small(Rect { x: 0, y: 0, w: 0, h: 0 }, 300.0));
        assert!(lasso_too_small(Rect { x: 0, y: 0, w: 0, h: 500 }, 300.0));
        assert!(lasso_too_small(Rect { x: 0, y: 0, w: 500, h: 0 }, 300.0));
        assert!(lasso_too_small(Rect { x: 0, y: 0, w: min - 1, h: min - 1 }, 300.0));
        assert!(!lasso_too_small(Rect { x: 0, y: 0, w: min, h: 3 }, 300.0));
        assert!(!lasso_too_small(Rect { x: 0, y: 0, w: 3, h: min }, 300.0));
        assert!(lasso_too_small(Rect { x: 0, y: 0, w: min, h: min }, 600.0));
    }

    // ---- the finger's way in to Select (owner ruling 2026-11-02): hold still, then draw ----

    // A data root of its own (the clipboard is root/clipboard.qpg), emptied first.
    fn fhold_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("grimoire-fhold-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        store_init(root.to_str().unwrap());
        assert!(!clipboard_has());
        root
    }

    fn finger(a: &mut App, type_: InType, x: f32, y: f32) {
        fshape_event(a, type_, x, y, now_ms());
    }

    // A finger down at (x, y), still for the hold time: editor_tick's hold fires.
    fn finger_hold(a: &mut App, x: f32, y: f32) {
        finger(a, InType::TouchDown, x, y);
        editor_tick(a, now_ms().wrapping_add(HOLD_MS + 10));
    }

    // The rest of a finger's lasso from its hold at (cx - r, cy - r): around (cx, cy), then the lift.
    fn finger_ring(a: &mut App, cx: f32, cy: f32, r: f32) {
        let ring = [(cx, cy - r - 5.0), (cx + r, cy - r), (cx + r + 5.0, cy), (cx + r, cy + r), (cx, cy + r + 5.0), (cx - r, cy + r), (cx - r - 5.0, cy), (cx - r, cy - r + 10.0)];
        for &(x, y) in &ring {
            finger(a, InType::TouchMove, x, y);
        }
        finger(a, InType::TouchUp, cx - r, cy - r + 10.0);
    }

    fn undo_depth(a: &mut App) -> usize {
        let h = ed(a).cur_hist().unwrap();
        ed_ref(a).hist[h].h.undo.len()
    }

    // Whether Select's bar is on the screen: laid out, and ink in its rect.
    fn bar_drawn(a: &App) -> bool {
        let b = rect_intersect(ed_ref(a).sel.bar, Rect { x: 0, y: 0, w: a.screen.w, h: a.screen.h });
        !rect_is_empty(b) && (b.y..b.y + b.h).any(|y| (b.x..b.x + b.w).any(|x| a.screen.px[(y * a.screen.stride + x) as usize] < 128))
    }

    // Held still off a link, a finger arms the lasso (a ring at it, Select mode entered quietly); drawn on, it
    // selects as the pen's lasso does, with no undo step, and a tap outside the selection ends Select mode.
    #[test]
    fn finger_hold_then_draw_selects() {
        let root = fhold_root("draw");
        let mut a = fshape_app();
        sel_word(&mut a, 400.0, 600.0);
        let depth = undo_depth(&mut a);
        finger_hold(&mut a, 280.0, 495.0);
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Lasso);
        assert_eq!(s.owner, Owner::Select);
        assert!(s.sel.finger && s.sel.by_finger);
        assert!(!rect_is_empty(s.sel.lasso_rect), "the ring is drawn, to be drawn away with the lasso");
        assert!(rect_is_empty(s.sel.bar), "no bar yet");
        // the pen's timed paste is not the finger's: held on, nothing happens
        editor_tick(&mut a, now_ms().wrapping_add(3 * HOLD_MS));
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger_ring(&mut a, 400.0, 615.0, 120.0);
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Selected);
        assert_eq!(s.sel.ids.len(), 2);
        assert_eq!(s.owner, Owner::None);
        assert!(s.sel.by_finger && !s.sel.finger);
        assert!(rect_is_empty(s.sel.lasso_rect));
        assert!(bar_drawn(&a), "the pen lasso's bar, drawn");
        assert_eq!(undo_depth(&mut a), depth, "selecting alone is no undo step");
        // a finger inside the box drags it, as before
        let b = ed_ref(&a).sel.box_;
        finger(&mut a, InType::TouchDown, (b.x + b.w / 2) as f32, (b.y + b.h / 2) as f32);
        assert_eq!(ed_ref(&a).sel.state, SelState::Dragging);
        finger(&mut a, InType::TouchUp, (b.x + b.w / 2) as f32, (b.y + b.h / 2) as f32);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        // a tap outside: the selection ends, and with it the Select mode the hold entered; nothing else
        finger(&mut a, InType::TouchDown, 700.0, 1100.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger(&mut a, InType::TouchUp, 700.0, 1100.0);
        assert_eq!(ed_ref(&a).owner, Owner::None);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 2);
        // a lasso around nothing ends Select mode at once
        finger_hold(&mut a, 580.0, 980.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger_ring(&mut a, 700.0, 1100.0, 120.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        assert_eq!(ed_ref(&a).owner, Owner::None);
        std::fs::remove_dir_all(root).unwrap();
    }

    // The screen is what its layers make of it (the `layers` probe's rule: a frame's blended edge drawn again over
    // itself may differ by 64 at most).
    fn layers_ok(a: &mut App) -> bool {
        let keep = a.screen.px.clone();
        compose(a, Rect { x: 0, y: 0, w: a.w, h: a.h });
        keep.iter().zip(a.screen.px.iter()).all(|(x, y)| x.abs_diff(*y) <= 64)
    }

    // A drag of the selection (2.1.0, select.rs's head): its pen samples draw nothing and refresh nothing; the
    // tick shows the newest place, the first move taking the bar away, and no other show comes before
    // DRAG_SHOW_MS while the pen moves; the screen is what compose makes of it at each step; Back in the middle
    // ends the selection with its copy of the screen and leaves the screen as compose makes it; a drag's lift
    // moves the ink by exactly its newest place, shown or not; a tap that wobbles a pixel is a tap; and a drag
    // whose lift was lost is ended by the next down, which acts on no button of the bar it had taken away.
    #[test]
    fn a_drag_draws_at_the_tick_never_at_a_sample() {
        let root = fhold_root("drag");
        let mut a = fshape_app();
        let (mut px, mut py) = (0.0, 0.0);
        to_page(ed(&mut a), 400.0, 600.0, &mut px, &mut py);
        sel_word(&mut a, px, py);
        sel_toggle(&mut a);
        sel_lasso(&mut a, 400.0, 615.0, 120.0, false, 1000);
        app_flush(&mut a);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        assert!(bar_drawn(&a));
        let probe = |a: &App, k: &str| sel_probe(a, k).unwrap();
        let first = |a: &App| ed_ref(a).open_page().unwrap().strokes.iter().map(|st| (st.pts[0].x, st.pts[0].y)).collect::<Vec<_>>();
        let at0 = first(&a);
        // the samples: nothing drawn, nothing to refresh
        let screen0 = a.screen.px.clone();
        pen_event(&mut a, InType::PenDown, false, 400.0, 610.0, 2000);
        assert_eq!(ed_ref(&a).sel.state, SelState::Dragging);
        for k in 1..=40 {
            pen_event(&mut a, InType::PenMove, false, 400.0 + 3.0 * k as f32, 610.0 - k as f32, 2000 + 3 * k as u32);
        }
        assert!(a.screen.px == screen0 && rect_is_empty(a.dirty), "a sample draws nothing");
        // (the first three within the drag's slop, 1 mm: the box stays for them)
        assert_eq!((probe(&a, "drag_samples"), probe(&a, "drag_shows"), probe(&a, "drag_shown")), ("37".into(), "0".into(), "-".into()));
        // the tick: the first move shown, the bar gone, the selection's ink walked only for the outline's strips
        // (the show's time a second back: every sample after it is newer than any tick below, so the pen never
        // seems to have paused and only DRAG_SHOW_MS can let a show through)
        let t0 = now_ms().wrapping_sub(1000);
        editor_tick(&mut a, t0);
        app_flush(&mut a);
        assert_eq!((probe(&a, "drag_shows"), probe(&a, "drag_copy")), ("1".into(), "1".into()));
        let (mut dx, mut dy) = (0.0, 0.0);
        to_page(ed(&mut a), 520.0, 570.0, &mut dx, &mut dy);
        let (mut ox, mut oy) = (0.0, 0.0);
        to_page(ed(&mut a), 400.0, 610.0, &mut ox, &mut oy);
        let want = ((dx - ox).round() as i32, (dy - oy).round() as i32);
        assert_eq!(probe(&a, "drag_shown"), format!("{} {}", want.0, want.1));
        assert!(!bar_drawn(&a), "the bar is away while the box moves");
        let inked: u32 = probe(&a, "drag_inked").parse().unwrap();
        assert!((1..=4).contains(&inked), "the halo walked only where the outline's strips were: {inked}");
        assert!(layers_ok(&mut a));
        // the pen moves on: a tick before DRAG_SHOW_MS went by since the show shows nothing
        for k in 41..=50 {
            pen_event(&mut a, InType::PenMove, false, 400.0 + 3.0 * k as f32, 610.0 - k as f32, 2200 + 3 * k as u32);
        }
        let screen1 = a.screen.px.clone();
        editor_tick(&mut a, t0.wrapping_add(DRAG_SHOW_MS - 1));
        assert!(a.screen.px == screen1 && rect_is_empty(a.dirty), "no show before DRAG_SHOW_MS");
        assert_eq!((probe(&a, "drag_shows"), probe(&a, "drag_shown")), ("1".into(), format!("{} {}", want.0, want.1)));
        // more samples, and DRAG_SHOW_MS went by since the show: the newest place shown, once
        for k in 51..=60 {
            pen_event(&mut a, InType::PenMove, false, 400.0 + 3.0 * k as f32, 610.0 - k as f32, 2200 + 3 * k as u32);
        }
        editor_tick(&mut a, t0.wrapping_add(DRAG_SHOW_MS));
        app_flush(&mut a);
        assert_eq!(probe(&a, "drag_shows"), "2");
        let newest = {
            let (mut x, mut y) = (0.0, 0.0);
            to_page(ed(&mut a), 580.0, 550.0, &mut x, &mut y);
            format!("{} {}", (x - ox).round() as i32, (y - oy).round() as i32)
        };
        assert_eq!(probe(&a, "drag_shown"), newest);
        editor_tick(&mut a, t0.wrapping_add(2 * DRAG_SHOW_MS)); // (nothing newer: nothing to show)
        assert_eq!(probe(&a, "drag_shows"), "2");
        assert!(layers_ok(&mut a));
        // Back in the middle: the selection ends, its copy of the screen with it, the screen as compose makes it
        assert!(editor_back(&mut a));
        app_flush(&mut a);
        assert_eq!(ed_ref(&a).sel.state, SelState::Armed);
        assert_eq!(probe(&a, "drag_copy"), "0");
        assert!(layers_ok(&mut a));
        pen_event(&mut a, InType::PenUp, false, 580.0, 550.0, 2600);
        assert_eq!(first(&a), at0, "nothing moved");
        // again, lifted with a place the tick never showed: the ink moved by it exactly, one step
        sel_lasso(&mut a, 400.0, 615.0, 120.0, false, 3000);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        let depth = undo_depth(&mut a);
        pen_event(&mut a, InType::PenDown, false, 400.0, 610.0, 4000);
        pen_event(&mut a, InType::PenMove, false, 450.0, 640.0, 4010);
        editor_tick(&mut a, now_ms());
        pen_event(&mut a, InType::PenMove, false, 470.0, 700.0, 4020);
        pen_event(&mut a, InType::PenUp, false, 470.0, 700.0, 4030);
        app_flush(&mut a);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        assert_eq!(undo_depth(&mut a), depth + 1);
        let (mut x, mut y) = (0.0, 0.0);
        to_page(ed(&mut a), 470.0, 700.0, &mut x, &mut y);
        let (mx, my) = ((x - ox).round(), (y - oy).round());
        let moved: Vec<(f32, f32)> = at0.iter().map(|&(x, y)| (x + mx, y + my)).collect();
        assert_eq!(first(&a), moved);
        assert_eq!(probe(&a, "drag_copy"), "0");
        assert!(layers_ok(&mut a));
        // a tap inside that wobbles a pixel (within the slop): no show, nothing moved, no undo step, the bar
        // left as it was
        let depth = undo_depth(&mut a);
        let screen2 = a.screen.px.clone();
        pen_event(&mut a, InType::PenDown, false, 470.0, 700.0, 5000);
        assert_eq!(ed_ref(&a).sel.state, SelState::Dragging);
        pen_event(&mut a, InType::PenMove, false, 471.0, 700.0, 5003);
        pen_event(&mut a, InType::PenMove, false, 471.0, 701.0, 5006);
        editor_tick(&mut a, now_ms().wrapping_add(1000));
        assert!(a.screen.px == screen2 && rect_is_empty(a.dirty), "a wobble shows nothing");
        pen_event(&mut a, InType::PenUp, false, 471.0, 701.0, 5009);
        app_flush(&mut a);
        assert_eq!((probe(&a, "drag_samples"), probe(&a, "drag_shows")), ("0".into(), "0".into()));
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        assert_eq!(undo_depth(&mut a), depth);
        assert_eq!(first(&a), moved);
        assert!(bar_drawn(&a));
        assert!(layers_ok(&mut a));
        // a drag whose lift was lost and that nothing ended (its owner let go): the next down ends it as a
        // cancel and is swallowed, even where the bar's Delete was before the drag (not on the screen then)
        let del = ed_ref(&a).sel.btns.iter().find(|(_, k)| *k == SelBtn::Delete).map(|(r, _)| *r).unwrap();
        let (cx, cy) = ((del.x + del.w / 2) as f32, (del.y + del.h / 2) as f32);
        pen_event(&mut a, InType::PenDown, false, 470.0, 700.0, 6000);
        pen_event(&mut a, InType::PenMove, false, 470.0, 640.0, 6010); // (up: the outline off the bar's rect)
        editor_tick(&mut a, now_ms());
        app_flush(&mut a);
        assert_eq!(probe(&a, "drag_shows"), "1");
        assert!(!bar_drawn(&a));
        ed(&mut a).owner = Owner::None;
        pen_event(&mut a, InType::PenDown, false, cx, cy, 7000);
        pen_event(&mut a, InType::PenUp, false, cx, cy, 7010);
        app_flush(&mut a);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        assert_eq!(ed_ref(&a).owner, Owner::None);
        assert_eq!(first(&a), moved, "nothing deleted, nothing moved");
        assert_eq!(undo_depth(&mut a), depth);
        assert_eq!(probe(&a, "drag_copy"), "0");
        assert!(bar_drawn(&a));
        assert!(layers_ok(&mut a));
        std::fs::remove_dir_all(root).unwrap();
    }

    // A finger held on a link opens the link's card, never the lasso; one that moved before the hold time is
    // what it always was (no lasso, no card).
    #[test]
    fn finger_hold_on_a_link_opens_its_card_and_a_moved_finger_arms_nothing() {
        let root = fhold_root("link");
        let mut a = fshape_app();
        let s = ed(&mut a);
        assert_eq!(nav_link_set(&mut s.nav, 1, 77, "p2"), 0);
        assert_eq!(nav_link_set_area(&mut s.nav, 1, 77, Some(Rect { x: 100, y: 200, w: 300, h: 300 })), 0);
        finger_hold(&mut a, 250.0, 350.0);
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Off);
        assert!(s.card.open, "the link's long-press card");
        assert!(s.link_held);
        finger(&mut a, InType::TouchUp, 250.0, 350.0);
        card_drop(ed(&mut a));
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        // moved past the slop before the hold time: no lasso, whatever the wait
        finger(&mut a, InType::TouchDown, 600.0, 900.0);
        finger(&mut a, InType::TouchMove, 600.0, 1000.0);
        editor_tick(&mut a, now_ms().wrapping_add(HOLD_MS + 10));
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        assert_eq!(ed_ref(&a).owner, Owner::Link);
        finger(&mut a, InType::TouchUp, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).owner, Owner::None);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        // a tremor within the slop still holds
        finger(&mut a, InType::TouchDown, 600.0, 900.0);
        finger(&mut a, InType::TouchMove, 605.0, 903.0);
        editor_tick(&mut a, now_ms().wrapping_add(HOLD_MS + 10));
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger(&mut a, InType::TouchUp, 605.0, 903.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        std::fs::remove_dir_all(root).unwrap();
    }

    // Lifted where it was held, the finger pastes the clipboard centred there (one undo step), or with no
    // clipboard does nothing at all; on a read-only page the lasso and Copy work and the paste is refused.
    #[test]
    fn finger_hold_lifted_still_pastes_and_read_only_refuses_it() {
        let root = fhold_root("paste");
        let mut a = fshape_app();
        sel_word(&mut a, 400.0, 600.0);
        let depth = undo_depth(&mut a);
        // no clipboard: nothing (Select mode left as quietly as it came)
        finger_hold(&mut a, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger(&mut a, InType::TouchUp, 602.0, 1001.0);
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Off);
        assert_eq!(s.owner, Owner::None);
        assert_eq!(s.open_page().unwrap().strokes.len(), 2);
        assert_eq!(undo_depth(&mut a), depth);
        assert!(!a.toast_visible);
        // a clipboard: the word lassoed with a finger and copied from the bar
        finger_hold(&mut a, 280.0, 495.0);
        finger_ring(&mut a, 400.0, 615.0, 120.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        let (cx, cy) = sel_control_point(ed_ref(&a), "copy").unwrap();
        finger(&mut a, InType::TouchDown, cx, cy);
        finger(&mut a, InType::TouchUp, cx, cy);
        assert!(clipboard_has());
        // held and lifted outside the selection: the old one goes, the paste lands centred there, selected
        finger_hold(&mut a, 500.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger(&mut a, InType::TouchUp, 500.0, 1000.0);
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Selected);
        assert_eq!(s.open_page().unwrap().strokes.len(), 4);
        assert!(bar_drawn(&a), "the pasted group's bar, drawn");
        let s = ed_ref(&a);
        let b = s.sel.box_;
        assert!((b.x + b.w / 2 - 500).abs() <= 1 && (b.y + b.h / 2 - 1000).abs() <= 1, "centred on the hold: {:?}", b);
        assert_eq!(undo_depth(&mut a), depth + 1, "the paste is one step");
        undo_redo(&mut a, true);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), 2);
        assert_eq!(undo_depth(&mut a), depth);
        // read-only: the lasso and Copy work, the paste is refused with nothing changed
        sel_off(&mut a);
        ed(&mut a).open_page_mut().unwrap().readonly = true;
        assert_eq!(clipboard_restore(None), 0); // (so the Copy below is what fills it)
        finger_hold(&mut a, 280.0, 495.0);
        finger_ring(&mut a, 400.0, 615.0, 120.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        let (cx, cy) = sel_control_point(ed_ref(&a), "copy").unwrap();
        finger(&mut a, InType::TouchDown, cx, cy);
        finger(&mut a, InType::TouchUp, cx, cy);
        assert!(clipboard_read().is_some_and(|g| g.ink == 2), "Copy on a read-only page fills the clipboard (the word's two strokes)");
        assert!(!a.toast_visible, "and says nothing");
        finger_hold(&mut a, 500.0, 1000.0);
        finger(&mut a, InType::TouchUp, 500.0, 1000.0);
        assert_eq!(a.toast.as_str(), tr(K_READONLY_MSG));
        let s = ed_ref(&a);
        assert_eq!(s.open_page().unwrap().strokes.len(), 2);
        assert_eq!(s.sel.state, SelState::Off);
        assert_eq!(undo_depth(&mut a), depth);
        std::fs::remove_dir_all(root).unwrap();
    }

    // In Select mode from the tool picker, a finger's hold lassos too, and the mode stays after it (it is
    // not the hold's to end); the pen's lasso there is unchanged.
    #[test]
    fn finger_hold_in_select_mode_keeps_the_mode() {
        let root = fhold_root("mode");
        let mut a = fshape_app();
        sel_word(&mut a, 400.0, 600.0);
        sel_toggle(&mut a);
        assert_eq!(ed_ref(&a).sel.state, SelState::Armed);
        finger_hold(&mut a, 280.0, 495.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        assert!(!ed_ref(&a).sel.by_finger);
        finger_ring(&mut a, 400.0, 615.0, 120.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        finger(&mut a, InType::TouchDown, 700.0, 1100.0);
        finger(&mut a, InType::TouchUp, 700.0, 1100.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Armed, "a tap outside deselects; the mode stays");
        // a still finger with no clipboard: nothing, and the mode stays
        finger_hold(&mut a, 600.0, 1000.0);
        finger(&mut a, InType::TouchUp, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Armed);
        sel_lasso(&mut a, 400.0, 615.0, 120.0, false, 5000);
        assert_eq!(ed_ref(&a).sel.state, SelState::Selected);
        std::fs::remove_dir_all(root).unwrap();
    }

    // ---- the finger hold's guards: a palm, a grip thumb or the writing hand never selects or pastes ----

    fn finger_v(a: &mut App, type_: InType, x: f32, y: f32, value: i32) {
        let t = now_ms();
        editor_event(a, &InEvent { type_, x, y, value, t_ms: t, k_ms: t, ..Default::default() });
    }

    // The word at (400, 600) on the page, lassoed with a finger and copied: the clipboard is full; Select
    // mode off again.
    fn fhold_clipboard(a: &mut App) {
        sel_word(a, 400.0, 600.0);
        finger_hold(a, 280.0, 495.0);
        finger_ring(a, 400.0, 615.0, 120.0);
        assert_eq!(ed_ref(a).sel.state, SelState::Selected);
        let (cx, cy) = sel_control_point(ed_ref(a), "copy").unwrap();
        finger(a, InType::TouchDown, cx, cy);
        finger(a, InType::TouchUp, cx, cy);
        assert!(clipboard_has());
        sel_off(a);
        assert_eq!(ed_ref(a).sel.state, SelState::Off);
    }

    // A contact the panel calls palm-sized (TOUCH_F_BROAD) never arms: at its down, or on a move before the
    // hold time; one that turns palm-sized after the arm drops the lasso, and its lift pastes nothing.
    #[test]
    fn finger_hold_guard_palm() {
        let root = fhold_root("palm");
        let mut a = fshape_app();
        fhold_clipboard(&mut a);
        let n = ed_ref(&a).open_page().unwrap().strokes.len();
        finger_v(&mut a, InType::TouchDown, 600.0, 1000.0, TOUCH_F_BROAD);
        editor_tick(&mut a, now_ms().wrapping_add(HOLD_MS + 10));
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger_v(&mut a, InType::TouchUp, 600.0, 1000.0, TOUCH_F_BROAD);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n);
        // broad on a move before the hold time
        finger(&mut a, InType::TouchDown, 600.0, 1000.0);
        finger_v(&mut a, InType::TouchMove, 601.0, 1000.0, TOUCH_F_BROAD);
        editor_tick(&mut a, now_ms().wrapping_add(HOLD_MS + 10));
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger(&mut a, InType::TouchUp, 601.0, 1000.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n);
        // broad after the arm: dropped, the ring drawn away, the lift ignored
        finger_hold(&mut a, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger_v(&mut a, InType::TouchMove, 601.0, 1000.0, TOUCH_F_BROAD);
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Off);
        assert_eq!(s.owner, Owner::None);
        assert!(rect_is_empty(s.sel.lasso_rect));
        finger(&mut a, InType::TouchUp, 601.0, 1000.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        std::fs::remove_dir_all(root).unwrap();
    }

    // The pen in range at the down (hovering, or gone under FHOLD_PEN_MS), or coming in range before the
    // hold time: nothing arms, and the lift pastes nothing. Gone longer ago: the hold arms as ever.
    #[test]
    fn finger_hold_guard_pen_before_the_arm() {
        let root = fhold_root("pen-before");
        let mut a = fshape_app();
        fhold_clipboard(&mut a);
        let n = ed_ref(&a).open_page().unwrap().strokes.len();
        pen_event(&mut a, InType::PenHover, false, 200.0, 200.0, now_ms());
        assert!(ed_ref(&a).pen_near);
        finger_hold(&mut a, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        assert_eq!(ed_ref(&a).owner, Owner::Link);
        finger(&mut a, InType::TouchUp, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n);
        // gone just now: still in range for this finger
        pen_event(&mut a, InType::PenLeave, false, 200.0, 200.0, now_ms());
        finger_hold(&mut a, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger(&mut a, InType::TouchUp, 600.0, 1000.0);
        // coming in range between the down and the hold time (fingers off the shape capture, so the
        // finger keeps its contact)
        ed(&mut a).pen_left = now_ms().wrapping_sub(5000);
        a.settings.gestures = false;
        finger(&mut a, InType::TouchDown, 600.0, 1000.0);
        pen_event(&mut a, InType::PenHover, false, 200.0, 200.0, now_ms());
        pen_event(&mut a, InType::PenLeave, false, 200.0, 200.0, now_ms());
        assert_eq!(ed_ref(&a).owner, Owner::Link);
        editor_tick(&mut a, now_ms().wrapping_add(HOLD_MS + 10));
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger(&mut a, InType::TouchUp, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n);
        // gone long ago: the hold arms
        ed(&mut a).pen_left = now_ms().wrapping_sub(5000);
        finger_hold(&mut a, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        finger(&mut a, InType::TouchUp, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n + 2, "lifted still: the paste");
        std::fs::remove_dir_all(root).unwrap();
    }

    // The pen coming after the arm (hovering, or writing elsewhere while a thumb rests): the armed lasso is
    // dropped at its first event, the ring drawn away, and the lift neither pastes nor selects; the pen's
    // stroke is ink as ever.
    #[test]
    fn finger_hold_guard_pen_after_the_arm() {
        let root = fhold_root("pen-after");
        let mut a = fshape_app();
        fhold_clipboard(&mut a);
        let n = ed_ref(&a).open_page().unwrap().strokes.len();
        finger_hold(&mut a, 600.0, 1000.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        pen_event(&mut a, InType::PenHover, false, 200.0, 200.0, now_ms());
        let s = ed_ref(&a);
        assert_eq!(s.sel.state, SelState::Off);
        assert_eq!(s.owner, Owner::None);
        assert!(rect_is_empty(s.sel.lasso_rect));
        finger(&mut a, InType::TouchMove, 900.0, 1000.0);
        finger(&mut a, InType::TouchUp, 900.0, 1000.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        pen_event(&mut a, InType::PenLeave, false, 200.0, 200.0, now_ms());
        // a thumb resting, armed, while the pen writes elsewhere: ink, and no paste at the thumb's lift
        ed(&mut a).pen_left = now_ms().wrapping_sub(5000);
        finger_hold(&mut a, 900.0, 1300.0);
        assert_eq!(ed_ref(&a).sel.state, SelState::Lasso);
        let t = now_ms();
        pen_event(&mut a, InType::PenDown, false, 200.0, 900.0, t);
        for k in 1..10 {
            pen_event(&mut a, InType::PenMove, false, 200.0 + 30.0 * k as f32, 900.0, t + 10 * k);
        }
        pen_event(&mut a, InType::PenUp, false, 470.0, 900.0, t + 100);
        pen_event(&mut a, InType::PenLeave, false, 470.0, 900.0, t + 110);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger(&mut a, InType::TouchUp, 900.0, 1300.0);
        let s = ed_ref(&a);
        assert_eq!(s.open_page().unwrap().strokes.len(), n + 1, "the pen's stroke, and no paste");
        assert_eq!(s.sel.state, SelState::Off);
        // and a thumb that lands while the pen writes: never armed, no paste
        let t = now_ms();
        pen_event(&mut a, InType::PenDown, false, 200.0, 1100.0, t);
        finger(&mut a, InType::TouchDown, 900.0, 1300.0);
        pen_event(&mut a, InType::PenMove, false, 400.0, 1100.0, t + 50);
        pen_event(&mut a, InType::PenUp, false, 400.0, 1100.0, t + 100);
        editor_tick(&mut a, now_ms().wrapping_add(HOLD_MS + 10));
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        finger(&mut a, InType::TouchUp, 900.0, 1300.0);
        assert_eq!(ed_ref(&a).open_page().unwrap().strokes.len(), n + 2);
        assert_eq!(ed_ref(&a).sel.state, SelState::Off);
        std::fs::remove_dir_all(root).unwrap();
    }
}

pub fn editor_map_point(a: &App, spec: &str) -> Option<(f32, f32)> {
    if a.scr == ScreenId::Library {
        return crate::library::library_map_point(a, spec);
    }
    a.ed.as_deref().and_then(|s| mapview_node_point(&s.map, spec))
}

/// Replay page samples through the current viewport, as real input reaches it.
pub fn editor_replay_point(a:&App,x:f32,y:f32)->(f32,f32) {
    let s=ed_ref(a);((x-s.ox)*s.zoom,(y-s.oy)*s.zoom)
}

fn destination_open(a:&mut App) {
    if !save_page(a){return;}
    let folder=ed_ref(a).nb.folder.as_str().to_string();
    if with_modal(a,|s|&mut s.destination,|p,_|p.start(&folder)) {ed(a).lt.hidden=true;compose_all(a,RefreshMode::Full);}
    else {app_toast(a,tr("Couldn't read notebooks."),3000);}
}
fn follow_target(a:&mut App,target:&str)->bool {
    if let Some((id,pid))=nav_external_target(target) {
        open_external_page(a,id,pid,true)
    } else if target=="index" {map_show(a,false);ed_ref(a).map.open}
    else {
        let l=NavLink{target:FixedStr::from_str(target),..Default::default()};let to=nav_link_page(&l,&ed_ref(a).nb);
        if to>=0 {go_to_page_pushing(a,to)}else{app_toast(a,tr("Linked page isn't in this notebook."),3000);false}
    }
}

// Both following a link and returning across notebooks must save the current links and validate
// the requested page before closing the editor that owns the current notebook.
fn return_to_notebook(a:&mut App)->bool {
    let Some(b)=ed_ref(a).notebook_back.last().cloned() else {return false;};
    let mut stack=ed_ref(a).notebook_back.clone();stack.pop();
    if !open_external_page(a,b.id.as_str(),b.pid,false) {return false;}
    ed(a).notebook_back=stack;
    // The page as it was left: its notebook's own back list, and the view (zoom and offset) when the screen
    // is the size it was for (a turn since rests the page as any open does).
    let s=ed(a);
    s.back=b.back;s.back_prune();
    if b.zoom>0.0 && b.view_w==s.view_w && b.view_h==s.view_h && (b.zoom,b.ox,b.oy)!=(s.zoom,s.ox,s.oy) {
        s.zoom=b.zoom.clamp(s.zoom_min,s.zoom_max);s.ox=b.ox;s.oy=b.oy;
        view_clamp(a);
        compose_all(a,RefreshMode::Full);
    }
    true
}
fn open_external_page(a:&mut App,id:&str,pid:u32,push_origin:bool)->bool {
        let mut nb=NotebookInfo::default();
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a link to another notebook's page
        // was followed, but that notebook or page is gone.
        if store_load_info(id,&mut nb)!=0 || !nb.page_ids.contains(&pid) {app_toast(a,tr("Notebook or page no longer exists."),3000);return false;}
        if ed_ref(a).nb.id==id {return nb.page_ids.iter().position(|&p|p==pid).is_some_and(|i|go_to_page_pushing(a,i as i32));}
        if !save_page_leaving(a){return false;}
        links_flush(a);if ed_ref(a).links_dirty {app_toast(a,tr("Couldn't save links."),3000);return false;}
        let i=nb.page_ids.iter().position(|&p|p==pid).unwrap();let mut checked=None;
        if store_read_page(&nb,i as i32,&mut checked)!=0 {app_toast(a,tr("Couldn't open page."),3000);return false;}drop(checked);
        let s=ed_ref(a);
        let origin=NotebookBack{id:s.nb.id,pid:s.open_pid(),zoom:s.zoom,ox:s.ox,oy:s.oy,view_w:s.view_w,view_h:s.view_h,back:s.back.clone()};
        let mut back=std::mem::take(&mut ed(a).notebook_back);
        if push_origin {if back.len()>=32 {back.remove(0);}back.push(origin);}
        let from_graph=ed_ref(a).from_graph;
        editor_open_notebook_at(a,id,pid,false);ed(a).notebook_back=back;ed(a).from_graph=from_graph;ed_ref(a).open&&ed_ref(a).nb.id==id&&ed_ref(a).open_pid()==pid
}

/// The Library's Graph opened a node (2.0.0-beta.2): notebook id at page pid, as editor_open_notebook_at opens
/// it, remembered as opened from the Graph, so Back with nothing before it goes back there.
pub fn editor_open_from_graph(a: &mut App, id: &str, pid: u32) {
    editor_open_notebook_at(a, id, pid, false);
    if a.scr == ScreenId::Editor && ed_ref(a).open && ed_ref(a).nb.id == id {
        ed(a).from_graph = true;
    }
}

// Back to the Library's Graph from a notebook it opened: the page saved and the notebook closed, as the
// Library button does, and the Library shows the Graph as it was left. False (staying) when the page could
// not be saved.
fn back_to_graph(a: &mut App) -> bool {
    app_flush(a);
    if !save_page(a) {
        return false;
    }
    close_notebook(a);
    ed(a).from_graph = false;
    crate::library::library_return_to_graph(a);
    app_switch(a, ScreenId::Library);
    app_toast(a, "", 0);
    true
}

pub fn editor_settings_background(a:&mut App) {
    compose(a,Rect::new(0,0,a.w,a.h));
}
