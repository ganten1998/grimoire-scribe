//! Select mode (docs/beta2/LASSO-DESIGN.md 6.1 and 7): the pen draws a dashed lasso, what it encloses
//! is selected (ink, its erasing, and its links), and a small bar acts on it: Copy, Cut, Delete, Paste.
//! A drag inside the selection's box moves only the box's outline; the strokes move at the release,
//! one undo step. The geometry and the data are lasso.rs and clipboard.rs; this is the screen.
//!
//! The drag on e-ink (2.0.0, the owner's report of 2026-11-05: "horrifically slow on drag with anything
//! containing colored in or multiple strokes"): a pen sample only records the newest place (`drag_step`),
//! and the editor's tick shows it (`drag_tick`) once the panel has finished the last fast refresh, at most
//! every DRAG_SHOW_MS or at once when the pen pauses (a burst of 60 samples is one show too). Its first move
//! takes
//! the bar away and keeps a copy of the screen without the outline (`DragView::base`); each show then puts
//! the outline's old strips back from that copy and draws the new ones, each strip a DU of its own. Nothing
//! is composed and no ink or halo is drawn while the box moves, so a selection of pencil shading costs what
//! one of three words does.
//!
//! The box's four corners carry resize handles (issue #37): dragged, they scale the selection about the
//! opposite corner, the strokes themselves scaled at the lift, one undo step (`resize_tick`,
//! `resize_done`, `sel_scale`); the outline and the handles follow the pen, its strips a DU each.
//!
//! Not a tool id: a mode beside link making, entered from the tool picker or the bindable `select`
//! action, left by choosing a pen, the action again, or the bar's Done.
//!
//! The finger's way in (owner ruling 2026-11-02: no panel button for Select): a finger held still on the
//! page off a link for HOLD_MS arms a lasso (a small dashed ring shows at the finger); drawn on
//! without lifting, it selects as the pen's does, and Select mode stays while the selection does; lifted
//! where it was held, it pastes the clipboard there (nothing without one). editor_tick arms it
//! (sel_finger_arm), from the same hold that opens a link's card.

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SelState {
    #[default]
    Off,
    /// waiting for the pen: the next pen contact on the page draws the lasso
    Armed,
    Lasso,
    Selected,
    Dragging,
}

/// A bar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelBtn {
    Copy,
    Cut,
    Delete,
    Paste,
    /// whole strokes or exact area (docs/beta2/PIXEL-LASSO.md 1.10): the mode the next lasso selects in,
    /// and with a selection, the same lasso judged again in the other
    Mode,
    Done,
}

/// The lasso's mode for this run of Grimoire (owner question 1: not remembered across restarts): false
/// whole strokes (the default), true exact area.
static EXACT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn exact_mode() -> bool {
    EXACT.load(std::sync::atomic::Ordering::Relaxed)
}

#[derive(Default)]
pub(super) struct SelMode {
    pub(super) state: SelState,
    /// the lasso, page px (LASSO_MAX points at most)
    pts: Vec<(f32, f32)>,
    last: (f32, f32),
    spacing: f32,
    /// screen px the lasso has drawn (a finger's hold ring with it)
    pub(super) lasso_rect: Rect,
    /// the page selected on (its id) and the selected ink by identity (re-found at each use: an edit
    /// elsewhere that took one away ends the selection)
    pid: u32,
    pub(super) ids: Vec<StrokeId>,
    /// the carried links, as they are now
    links: Vec<NavLink>,
    /// the box of the selected ink and its link areas, page px
    pub(super) box_: Rect,
    /// a drag: where it began (page px) and how far it has moved the box (whole page px)
    drag_from: (f32, f32),
    pub(super) drag: (i32, i32),
    /// every place a drag drew the outline and the bar by fast refreshes, screen px: its end draws all of
    /// it again in grays (issue #11: a drag that went round left the page between its ends dithered)
    outline_rect: Rect,
    /// the bar, screen px, and its buttons
    pub(super) bar: Rect,
    pub(super) btns: Vec<(Rect, SelBtn)>,
    /// where the bar was before it moved, still to be drawn over (the next redraw takes it)
    stale: Rect,
    /// Select mode was entered by a pen button held (its hold or drag bound to `select`), and whether it
    /// is still held: let go with nothing selected, or a tap outside the selection after, and the pen is
    /// a pen again (sel_settle_hold)
    pub(super) by_hold: bool,
    pub(super) holding: bool,
    /// the lasso's pen down: when (now_ms), and whether it has stayed put (a long press pastes there)
    down_at: u32,
    still: bool,
    /// the lasso is a finger's, armed by its hold (sel_finger_arm): it stays still within press_move's
    /// slop, and lifted still it pastes (sel_tick's timed paste is the pen's alone)
    pub(super) finger: bool,
    /// Select mode was entered by a finger's hold: it ends when nothing is selected any more (a lasso
    /// around nothing, a tap outside the selection, Delete, a page turn), as a held button's does let go
    pub(super) by_finger: bool,
    /// entered by a finger's hold and nothing of the mode drawn yet (the panel still shows the pen): leaving
    /// it so redraws nothing
    quiet: bool,
    /// the live preview of the lasso being drawn (docs/beta2/PIXEL-LASSO.md 2): what it would select now,
    /// a halo on that ink; None when the page could not be read, or an edit made it stale
    live: Option<Box<LassoLive>>,
    /// how many of `pts` the preview has folded in; a full lasso dropped every other point since (the
    /// preview answers again from the whole lasso)
    folded: usize,
    halved: bool,
    /// when the preview last folded, and when the last point was kept (input times, ms)
    fold_at: u32,
    point_at: u32,
    /// screen px the preview's fast refreshes covered: the lift draws all of it again in grays
    pub(super) preview_rect: Rect,
    /// the scripts' probes: the last lasso's batches and preview refreshes, whether its lift reused the
    /// preview's answer, and the rect the lift drew again
    preview_batches: u32,
    preview_refreshes: u32,
    preview_reused: bool,
    lift_rect: Rect,
    preview_seen: Rect,
    /// this lasso's (or selection's) mode: exact area (the mode, flipped by Shift or Alt held as the lasso began)
    pub(super) exact: bool,
    /// the lifted lasso's points, while its selection lasts: the mode button judges it again
    kept: Vec<(f32, f32)>,
    /// exact area: the strokes the lasso crosses (by identity) and their pieces, each with whether it is
    /// inside; nothing on the page changes until Move, Cut or Delete splices them in
    plan: Vec<(StrokeId, Vec<(Box<Stroke>, bool)>)>,
    /// a drag as the screen shows it (the module's head)
    dv: DragView,
    /// a resize of the selection's box: the corner dragged (0 none, 1 top-left, 2 top-right, 3 bottom-left,
    /// 4 bottom-right), the box it began with, and the box it last drew (page px)
    pub(super) resize_handle: i32,
    resize_box0: Rect,
    resize_shown: Rect,
}

/// A drag of the selection as the screen shows it, while `drag` is where the pen has taken it.
#[derive(Default)]
struct DragView {
    /// the drag has moved and its display began: the bar is gone, and the outline is drawn at `shown`
    /// (page px from the box's place; None for nowhere, while its first move takes it away)
    moving: bool,
    shown: Option<(i32, i32)>,
    /// the pen has strayed past the drag's slop from where it came down (DRAG_SLOP_MM): until then the box
    /// stays, so a tap that wobbles a pixel is a tap (no bar taken away, no move, no undo step)
    left: bool,
    /// the screen without the bar and the outline over `base_at` (the page's part of the screen, lasso_clip
    /// as the drag began), kept in step by every compose while it lasts (sel_draw); None without the memory
    /// for it, when each show composes its strips instead (the halo drawn again where they meet it)
    base: Option<Canvas>,
    base_at: Rect,
    /// when the outline was last shown, and when the place last changed (now_ms)
    shown_at: u32,
    moved_at: u32,
    /// the scripts' probes and the log line at the lift: the pen samples that changed the place, the shows
    /// (the first move's included), the px they presented, and how many times the selection's ink was
    /// walked while the drag lasted (its halo drawn: every point of every stroke it meets)
    samples: u32,
    shows: u32,
    samples_at_last_show: u32,
    px: i64,
    inked: u32,
}

/// How long the preview lets new lasso points wait before it folds them in (input time, ms, from the first
/// one waiting): at most about 8 refresh rounds a second however fast the pen moves.
const PREVIEW_MS: u32 = 120;
/// A pen that pauses this long with points not folded in yet gets them folded at once (editor_tick).
const PREVIEW_PAUSE_MS: u32 = 50;
/// The halo around previewed and selected ink, each side of it (ui px at 300 dpi).
const HALO_UI: f32 = 3.0;
/// A batch's refreshes: at most this many rects (the strokes that flipped, merged).
const PREVIEW_RECTS: usize = 6;
/// A show's DU refreshes: at most this many (its strips, two joined only when that covers no more).
const DRAG_RECTS: usize = 16;
/// A drag shows its newest place at most this often (ms), as the preview folds (on the Kindle the panel's
/// DU, about 188 ms at room temperature, paces it first: display_fast_busy)...
pub(super) const DRAG_SHOW_MS: u32 = PREVIEW_MS;
/// ...or as soon as the pen has kept still this long.
const DRAG_PAUSE_MS: u32 = PREVIEW_PAUSE_MS;
/// ...and after this many samples since the last show, however fast they arrive: the samples of one turn
/// are one show.
const DRAG_SHOW_SAMPLES: u32 = 60;
/// A resize handle's square (ui px at 300 dpi), and the square that takes a down on it (a finger's: wider
/// than it looks, half of it outside the box so the corner need not be hit exactly).
fn resize_handle_size() -> i32 {
    ui_px(28.0)
}

fn resize_handle_hit_size() -> i32 {
    ui_px(56.0)
}

/// How far a pen may stray from where it came down inside the selection (mm on the screen) before the box
/// moves: a tap's wobble, not a drag (a finger's: press_move's slop, finger_slop).
const DRAG_SLOP_MM: f32 = 1.0;

/// press_move's slop (ui.rs): how far a finger may stray and still be held, or lifted, still.
fn finger_slop() -> f32 {
    ui_px(30.0) as f32
}


/// The four resize handles' squares of box b (screen px), each at a corner, half of it outside.
fn resize_handles(b: Rect, sz: i32) -> [Rect; 4] {
    let h = sz / 2;
    [
        Rect { x: b.x - h, y: b.y - h, w: sz, h: sz },
        Rect { x: b.x + b.w - h, y: b.y - h, w: sz, h: sz },
        Rect { x: b.x - h, y: b.y + b.h - h, w: sz, h: sz },
        Rect { x: b.x + b.w - h, y: b.y + b.h - h, w: sz, h: sz },
    ]
}

/// Which of box b's resize handles screen point (x, y) is on: 0 none, 1 top-left, 2 top-right, 3 bottom-left,
/// 4 bottom-right.
fn resize_handle_hit(b: Rect, x: i32, y: i32) -> i32 {
    let v = resize_handles(b, resize_handle_hit_size());
    v.iter().position(|r| rect_contains(*r, x, y)).map_or(0, |i| i as i32 + 1)
}

/// The resize handle at screen (x, y) of the selection, 0 for none (no selection, or another page's).
fn resize_at(s: &EditorState, x: i32, y: i32) -> i32 {
    if s.sel.state != SelState::Selected || rect_is_empty(s.sel.box_) || s.sel.pid != s.open_pid() {
        return 0;
    }
    if s.open_page().is_some_and(|p| p.readonly) {
        return 0;
    }
    resize_handle_hit(box_at(s, s.sel.box_, s.sel.drag), x, y)
}

/// The corner a resize by `handle` keeps still of box b (page px): the one opposite it.
fn resize_anchor(b: Rect, handle: i32) -> (f32, f32) {
    match handle {
        1 => ((b.x + b.w) as f32, (b.y + b.h) as f32),
        2 => (b.x as f32, (b.y + b.h) as f32),
        3 => ((b.x + b.w) as f32, b.y as f32),
        _ => (b.x as f32, b.y as f32),
    }
}

/// The box a resize of `handle` to page point (px, py) makes of from: scaled uniformly about the opposite
/// corner, which stays, as large as the pointer asks; each side at least a handle's width, and on the page.
fn resize_box(s: &EditorState, from: Rect, handle: i32, px: f32, py: f32) -> Rect {
    if from.w <= 0 || from.h <= 0 {
        return from;
    }
    let (ax, ay) = resize_anchor(from, handle);
    let (cx, cy) = match handle {
        1 => (from.x as f32, from.y as f32),
        2 => ((from.x + from.w) as f32, from.y as f32),
        3 => (from.x as f32, (from.y + from.h) as f32),
        _ => ((from.x + from.w) as f32, (from.y + from.h) as f32),
    };
    let (d0x, d0y) = (cx - ax, cy - ay);
    let d0 = (d0x * d0x + d0y * d0y).sqrt();
    if d0 < 1e-3 {
        return from;
    }
    let k = ((px - ax) * (px - ax) + (py - ay) * (py - ay)).sqrt() / d0;
    let least = ui_px(20.0) as f32 / s.zoom;
    let kmin = (least / from.w as f32).max(least / from.h as f32).max(0.02);
    // (the side the dragged corner is on grows toward the page's edge: the anchor is the other one)
    let kw = if d0x > 0.0 { s.pgw as f32 - ax } else { ax } / from.w as f32;
    let kh = if d0y > 0.0 { s.pgh as f32 - ay } else { ay } / from.h as f32;
    let k = if k.is_finite() { k } else { 1.0 };
    let k = k.clamp(kmin, kw.min(kh).max(kmin));
    let w = (from.w as f32 * k).round().max(1.0) as i32;
    let h = (from.h as f32 * k).round().max(1.0) as i32;
    let x = if d0x > 0.0 { ax.round() as i32 } else { ax.round() as i32 - w };
    let y = if d0y > 0.0 { ay.round() as i32 } else { ay.round() as i32 - h };
    Rect { x: x.clamp(0, (s.pgw - w).max(0)), y: y.clamp(0, (s.pgh - h).max(0)), w, h }
}


fn sel_ref(a: &App) -> &SelMode {
    &ed_ref(a).sel
}

/// The selection's indices on the open page, ascending; None when there is none or it no longer holds
/// (another page, or one of its strokes is gone).
fn sel_indices(s: &EditorState) -> Option<Vec<usize>> {
    if (s.sel.ids.is_empty() && s.sel.plan.is_empty()) || s.sel.pid != s.open_pid() {
        return None;
    }
    let p = s.open_page()?;
    if !s.sel.plan.is_empty() && plan_indices(s).is_none() {
        return None;
    }
    let mut out = Vec::with_capacity(s.sel.ids.len());
    for (i, st) in p.strokes.iter().enumerate() {
        if s.sel.ids.contains(&stroke_id(st)) {
            out.push(i);
        }
    }
    (out.len() == s.sel.ids.len()).then_some(out)
}

/// Exact area: the indices on the open page of the strokes the plan cuts, in the plan's order (ascending
/// on the page when it was made); None when one is gone.
fn plan_indices(s: &EditorState) -> Option<Vec<usize>> {
    let p = s.open_page()?;
    s.sel.plan.iter().map(|(id, _)| p.strokes.iter().position(|st| stroke_id(st) == *id)).collect()
}

/// The page rect the open view shows.
fn view_page(a: &App) -> Rect {
    let s = ed_ref(a);
    let top = status_reserved(a);
    let (x0, y0) = (s.ox, s.oy + top as f32 / s.zoom);
    let (x1, y1) = (s.ox + a.w as f32 / s.zoom, s.oy + a.h as f32 / s.zoom);
    Rect { x: x0.ceil() as i32, y: y0.ceil() as i32, w: (x1 - x0).floor() as i32, h: (y1 - y0).floor() as i32 }
}

/// The screen rect the selection's overlay covers now: its box where the screen shows it, its bar (none while
/// a drag moves it).
fn overlay_rect(a: &App) -> Rect {
    let m = sel_ref(a);
    if !matches!(m.state, SelState::Selected | SelState::Dragging | SelState::Armed) {
        return RECT_EMPTY;
    }
    let mut r = if bar_hidden(m) { RECT_EMPTY } else { m.bar };
    if m.state != SelState::Armed && !rect_is_empty(m.box_) {
        if let Some(off) = outline_off(m) {
            let handles = if handles_shown(ed_ref(a)) { resize_handle_size() / 2 + ui_px(2.0) } else { 0 };
            r = rect_union(r, rect_inflate(box_at(ed_ref(a), m.box_, off), link_band() + ui_px(4.0) + handles));
        }
    }
    r
}

/// Whether the selection's box shows its four resize handles: a selection of ink on the open page, not
/// while a drag moves it (and not on a read-only page, which refuses the resize).
fn handles_shown(s: &EditorState) -> bool {
    if s.open_page().is_some_and(|p| p.readonly) {
        return false; // (a read-only page refuses the resize)
    }
    match s.sel.state {
        SelState::Selected => !rect_is_empty(s.sel.box_),
        SelState::Dragging => s.sel.resize_handle > 0,
        _ => false,
    }
}

/// Where the screen shows the selection's outline (page px from the box's place): while a drag moves it,
/// the place last shown (none while its first move takes it away); before that the box's own place.
fn outline_off(m: &SelMode) -> Option<(i32, i32)> {
    if m.resize_handle > 0 {
        // (a resize moves the box itself, so it is drawn where it is)
        return Some((0, 0));
    }
    match m.state {
        SelState::Dragging if m.dv.moving => m.dv.shown,
        SelState::Dragging => Some((0, 0)),
        _ => Some(m.drag),
    }
}

/// The bar is away while a drag moves the box (it comes back at the lift).
fn bar_hidden(m: &SelMode) -> bool {
    m.state == SelState::Dragging && (m.dv.moving || m.resize_handle > 0)
}

/// Page rect r moved by `off`, on the screen.
fn box_at(s: &EditorState, r: Rect, off: (i32, i32)) -> Rect {
    box_screen(s, Rect { x: r.x + off.0, y: r.y + off.1, ..r })
}

/// What the selection's outline draws with its box at `off`: the box's dithered band (as the link lasso's)
/// and its carried link areas' thinner ones, each (rect, band, halo) for canvas_dither_rect, screen px.
fn outline_shapes(s: &EditorState, off: (i32, i32)) -> Vec<(Rect, i32, i32)> {
    let mut v = Vec::with_capacity(1 + s.sel.links.len());
    v.push((rect_inflate(box_at(s, s.sel.box_, off), ui_px(6.0)), link_band(), ui_px(1.0)));
    for l in s.sel.links.iter().filter(|l| l.has_area) {
        v.push((box_at(s, l.area, off), ui_px(2.0), ui_px(1.0)));
    }
    v
}

/// The outline with its box at `off`, onto the screen where r meets the page's part of it (lasso_clip).
fn draw_outline(a: &mut App, off: (i32, i32), r: Rect) {
    let shapes = outline_shapes(ed_ref(a), off);
    let clip = a.screen.clip;
    let c = rect_intersect(r, lasso_clip(a));
    canvas_set_clip(&mut a.screen, c);
    for (b, t, h) in shapes {
        canvas_dither_rect(&mut a.screen, b, t, h);
    }
    a.screen.clip = clip;
}

/// The four resize handles over the selection's outline, at its box's corners, where r meets them.
fn draw_resize_handles(a: &mut App, off: (i32, i32), r: Rect) {
    let sz = resize_handle_size();
    let s = ed_ref(a);
    if rect_is_empty(s.sel.box_) {
        return;
    }
    let b = box_at(s, s.sel.box_, off);
    let clip = a.screen.clip;
    let c = rect_intersect(r, lasso_clip(a));
    canvas_set_clip(&mut a.screen, c);
    for h in resize_handles(b, sz) {
        let h = rect_intersect(h, c);
        if !rect_is_empty(h) {
            canvas_fill(&mut a.screen, h, 0);
        }
    }
    a.screen.clip = clip;
}

/// The four strips canvas_dither_rect(r, thick, halo) paints, apart: the top and the bottom the full width,
/// the left and the right between them (linkcard_strips' shape). Together exactly what it paints.
fn dither_strips(r: Rect, thick: i32, halo: i32) -> [Rect; 4] {
    let t = thick.min(r.w).min(r.h);
    let o = rect_inflate(r, halo);
    let s = t + 2 * halo;
    let top = Rect { x: o.x, y: o.y, w: o.w, h: s.min(o.h) };
    let bottom = Rect { x: o.x, y: (o.y + o.h - s).max(top.y + top.h), w: o.w, h: (o.y + o.h - (top.y + top.h)).clamp(0, s) };
    let mid_y = top.y + top.h;
    let mid_h = (bottom.y - mid_y).max(0);
    let left = Rect { x: o.x, y: mid_y, w: s.min(o.w), h: mid_h };
    let right = Rect { x: (o.x + o.w - s).max(left.x + left.w), y: mid_y, w: (o.x + o.w - (left.x + left.w)).clamp(0, s), h: mid_h };
    [top, bottom, left, right]
}

/// The screen strips the outline covers with its box at `off`, cut to where it draws (lasso_clip).
fn outline_strips(a: &App, off: (i32, i32)) -> Vec<Rect> {
    let clip = lasso_clip(a);
    let shapes = outline_shapes(ed_ref(a), off);
    let mut v = Vec::with_capacity(4 * shapes.len());
    for (b, t, h) in shapes {
        for st in dither_strips(b, t, h) {
            let st = rect_intersect(st, clip);
            if !rect_is_empty(st) {
                v.push(st);
            }
        }
    }
    v
}

fn rect_area(r: Rect) -> i64 {
    if rect_is_empty(r) { 0 } else { r.w as i64 * r.h as i64 }
}

/// The screen px of the outline's ink with its box at `off`, which a fast refresh turns back to the page as
/// it moves (its carried link areas' thinner ones left out).
fn outline_ink(a: &App, off: (i32, i32)) -> i64 {
    let s = ed_ref(a);
    if rect_is_empty(s.sel.box_) {
        return 0;
    }
    let b = rect_inflate(box_at(s, s.sel.box_, off), ui_px(6.0));
    let (w, h, t) = (b.w as i64, b.h as i64, link_band() as i64);
    (2 * (w + h) * t).min(w * h)
}

fn redraw(a: &mut App, r: Rect, m: RefreshMode) {
    let r = rect_union(r, std::mem::take(&mut ed(a).sel.stale));
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h });
    if rect_is_empty(r) {
        return;
    }
    compose(a, r);
    app_invalidate(a, r, m);
}

/// The bar's place and buttons for the state now (screen px).
fn layout_bar(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let top = status_reserved(a);
    let has_clip = clipboard_has();
    let s = ed(a);
    let st = s.sel.state;
    let mut kinds: Vec<SelBtn> = Vec::new();
    if st == SelState::Selected || st == SelState::Dragging {
        kinds.extend([SelBtn::Copy, SelBtn::Cut, SelBtn::Delete]);
    }
    if has_clip {
        kinds.push(SelBtn::Paste);
    }
    kinds.push(SelBtn::Mode);
    kinds.push(SelBtn::Done);
    let mut bw = ui_px(250.0);
    let mw = ui_px(360.0); // (the mode button: its longer names)
    let bh = ui_px(100.0);
    let m = ui_px(15.0);
    let h = bh + 2 * m;
    let n = kinds.len() as i32;
    // (under the box where the screen shows it)
    let off = outline_off(&s.sel).unwrap_or(s.sel.drag);
    let y = if st == SelState::Selected || st == SelState::Dragging {
        let b = box_at(s, s.sel.box_, off);
        let below = b.y + b.h + ui_px(30.0);
        if below + h <= ah - ui_px(20.0) { below } else { (b.y - ui_px(30.0) - h).max(top + ui_px(10.0)) }
    } else {
        ah - h - ui_px(40.0)
    };
    // the room beside the panel where it stands at a side over the bar's rows (the bar never covers it),
    // the plain buttons narrower when six do not fit there
    let (mut lo, mut hi) = (ui_px(10.0), aw - ui_px(10.0));
    let pn = s.panel;
    if !rect_is_empty(pn) && pn.y < y + h && pn.y + pn.h > y {
        if pn.x > aw / 2 {
            hi = hi.min(pn.x - ui_px(10.0));
        } else if pn.x + pn.w < aw / 2 {
            lo = lo.max(pn.x + pn.w + ui_px(10.0));
        }
    }
    if n * m + m + mw + (n - 1) * bw > hi - lo && n > 1 {
        bw = ((hi - lo - mw - (n + 1) * m) / (n - 1)).max(ui_px(190.0));
    }
    let wide = |b: SelBtn| if b == SelBtn::Mode { mw } else { bw };
    let w = kinds.iter().map(|&b| wide(b) + m).sum::<i32>() + m;
    let want = if st == SelState::Selected || st == SelState::Dragging {
        let b = box_at(s, s.sel.box_, off);
        b.x + b.w / 2 - w / 2
    } else {
        (lo + hi - w) / 2
    };
    let x = clampi(want, lo, (hi - w).max(lo));
    let bar = Rect { x, y, w, h };
    if s.sel.bar != bar && !rect_is_empty(s.sel.bar) {
        s.sel.stale = rect_union(s.sel.stale, s.sel.bar);
    }
    s.sel.bar = bar;
    let mut at = x + m;
    s.sel.btns = kinds
        .iter()
        .map(|&b| {
            let r = Rect { x: at, y: y + m, w: wide(b), h: bh };
            at += wide(b) + m;
            (r, b)
        })
        .collect();
}

fn btn_label(b: SelBtn) -> &'static str {
    match b {
        // TRANSLATORS: Select mode's bar under a selection (a button, about 220 px at 44 px): copies the
        // selected ink to the clipboard.
        SelBtn::Copy => c_("select", "Copy"),
        // TRANSLATORS: Select mode's bar: copies the selection to the clipboard and takes it off the page.
        SelBtn::Cut => c_("select", "Cut"),
        // TRANSLATORS: Select mode's bar: takes the selection off the page (not to the clipboard).
        SelBtn::Delete => c_("select", "Delete"),
        // TRANSLATORS: Select mode's bar: puts what was copied or cut on this page.
        SelBtn::Paste => c_("select", "Paste"),
        // TRANSLATORS: Select mode's bar: leaves Select mode.
        SelBtn::Done => c_("select", "Done"),
        SelBtn::Mode => mode_label(exact_mode()),
    }
}

fn mode_label(exact: bool) -> &'static str {
    if exact {
        // TRANSLATORS: Select mode's bar (a button, about 360 px at 44 px): the lasso cuts the strokes it
        // crosses along its line and takes only the ink inside it. Tapped, it goes back to whole strokes.
        c_("select", "Exact area")
    } else {
        // TRANSLATORS: Select mode's bar (a button, about 360 px at 44 px): the lasso takes whole strokes
        // (those mostly inside it). Tapped, it cuts strokes along the lasso instead (exact area).
        c_("select", "Whole strokes")
    }
}

/// The selection's box (dithered, as the link lasso is), its carried link areas dashed, and the bar,
/// where r meets them. compose calls it over everything.
pub(super) fn sel_draw(a: &mut App, r: Rect) {
    let st = sel_ref(a).state;
    // A drag's copy of the screen without its outline takes what compose made of r first: compose draws
    // this last, so r holds every layer under it now (never Live view's composing without the panel)
    if st == SelState::Dragging && !ed_ref(a).live_no_panel {
        let App { screen, ed, .. } = a;
        let dv = &mut ed.as_deref_mut().unwrap().sel.dv;
        let at = dv.base_at;
        if let Some(base) = dv.base.as_mut() {
            let rr = rect_intersect(r, at);
            if !rect_is_empty(rr) {
                canvas_blit(base, screen, rr, rr.x - at.x, rr.y - at.y);
            }
        }
    }
    // (under a sheet or the page peek the bar could not be tapped: it is drawn again as the sheet closes; a
    // finger's hold that entered Select mode shows no bar until it has selected or pasted something)
    if st == SelState::Off || st == SelState::Lasso || sel_ref(a).quiet || !ed_ref(a).open || sheet_open(ed_ref(a)) || ed_ref(a).peek.active {
        if st == SelState::Lasso {
            sel_draw_lasso(a, r);
        }
        return;
    }
    let hidden = bar_hidden(sel_ref(a));
    if !hidden {
        layout_bar(a);
    }
    let clip = a.screen.clip;
    let s = ed_ref(a);
    if (st == SelState::Selected || st == SelState::Dragging) && s.sel.pid == s.open_pid() {
        if let Some(off) = outline_off(&s.sel) {
            let handles = handles_shown(s);
            draw_outline(a, off, r);
            if handles {
                draw_resize_handles(a, off, r);
            }
        }
    }
    canvas_set_clip(&mut a.screen, r);
    let s = ed_ref(a);
    let (bar, btns, nlinks) = (s.sel.bar, s.sel.btns.clone(), s.sel.links.len());
    if !hidden && rect_overlaps(r, bar) {
        let c = &mut a.screen;
        ui_frame(c, bar, ui_px(24.0), 255, true, 0);
        ui_frame(c, bar, ui_px(24.0), 0, false, ui_px(6.0));
        for (br, b) in &btns {
            let mut label = btn_label(*b).to_string();
            if *b == SelBtn::Copy && nlinks > 0 {
                // TRANSLATORS: Select mode's Copy button when the selection carries links: %d is how many
                // (shown so the writer sees the links go along before acting).
                label = format!("{} ({})", label, cformat!(ng_("%d link", "%d links", nlinks as i64), nlinks as i32));
            }
            ui_button(c, *br, IconKind::None, &label, if *b == SelBtn::Done { ButtonStyle::Primary } else { ButtonStyle::Plain });
        }
    }
    a.screen.clip = clip;
}

fn sel_draw_lasso(a: &mut App, r: Rect) {
    let s = ed_ref(a);
    if s.sel.pts.len() < 2 {
        return;
    }
    let pts: Vec<(f32, f32)> = s.sel.pts.iter().map(|&p| lasso_screen_pt(s, p)).collect();
    let (thick, _, _) = lasso_pen();
    let clip = a.screen.clip;
    let c = rect_intersect(r, lasso_clip(a));
    canvas_set_clip(&mut a.screen, c);
    canvas_dither_poly(&mut a.screen, &pts, thick, 0);
    a.screen.clip = clip;
}

// ---- entering and leaving ----

/// The `select` action and the tool picker's Select: into Select mode, or out of it.
pub(super) fn sel_toggle(a: &mut App) {
    if sel_ref(a).state != SelState::Off {
        sel_off(a);
        return;
    }
    if !ed_ref(a).open {
        return;
    }
    sel_enter(a, false);
    app_toast_dismiss(a); // (no toast: the owner, 2026-11-04; one left up from before goes)
}

/// The `select-exact` action: Select mode in exact area, or with Select mode on, the bar's mode button.
pub(super) fn sel_exact_action(a: &mut App) {
    if sel_ref(a).state != SelState::Off {
        sel_mode_toggle(a);
        return;
    }
    if !ed_ref(a).open {
        return;
    }
    EXACT.store(true, std::sync::atomic::Ordering::Relaxed);
    sel_enter(a, false);
    ed(a).sel.exact = true;
    app_toast_dismiss(a); // (no toast: the owner, 2026-11-04; one left up from before goes)
}

fn sel_enter(a: &mut App, held: bool) {
    lm_drop(ed(a));
    let s = ed(a);
    s.sel = SelMode { state: SelState::Armed, by_hold: held, holding: held, ..Default::default() };
    layout_bar(a);
    let r = overlay_rect(a);
    redraw(a, r, RefreshMode::Ui);
    repaint_panel(a, RefreshMode::Ui); // (its tool button shows the lasso)
}

/// A pen button bound to `select` on its hold or its drag went down (held) or up: Select mode while it is
/// held, the contact under it the lasso. Let go with nothing selected and the pen is a pen again; with a
/// selection, Select mode stays for the bar and a drag, until a tap outside the selection, Done or a pen.
pub(super) fn sel_hold(a: &mut App, held: bool) {
    if held {
        // (a sheet or the page peek in front: the button's contact belongs to it, as the action's does)
        if sheet_open(ed_ref(a)) || ed_ref(a).peek.active {
            return;
        }
        if sel_ref(a).state == SelState::Off {
            if ed_ref(a).open {
                sel_enter(a, true);
            }
        } else if sel_ref(a).by_hold {
            ed(a).sel.holding = true;
        }
        return;
    }
    if !sel_ref(a).holding {
        return;
    }
    ed(a).sel.holding = false;
    sel_settle_hold(a);
}

/// The pen buttons were reset (sleep, the pen out of range): a held Select is let go.
pub(super) fn sel_hold_reset(s: &mut EditorState) {
    s.sel.holding = false;
}

/// Select mode a held button entered ends once it is let go and nothing is selected (a lasso still being
/// drawn finishes first); so does one a finger's hold entered, once nothing is selected.
fn sel_settle_hold(a: &mut App) {
    let m = sel_ref(a);
    if ((m.by_hold && !m.holding) || m.by_finger) && m.state == SelState::Armed {
        sel_off(a);
    }
}

/// Out of Select mode (a pen chosen, the action again, Done): the selection and the bar go.
pub(super) fn sel_off(a: &mut App) {
    if sel_ref(a).state == SelState::Off {
        return;
    }
    // (a drag's outline_rect: every place it showed the outline and the bar by DU, drawn again in grays;
    // its newest place counted as DU wear that the grays cannot wipe)
    let swept = take_outline_wear(a);
    let m = sel_ref(a);
    let r = rect_union(rect_union(rect_union(rect_union(overlay_rect(a), m.lasso_rect), m.stale), m.preview_rect), swept);
    let s = ed(a);
    if s.owner == Owner::Select {
        s.owner = Owner::None;
    }
    let quiet = s.sel.quiet;
    s.sel = SelMode::default();
    redraw(a, r, RefreshMode::Ui);
    if !quiet {
        repaint_panel(a, RefreshMode::Ui); // (its tool button shows the pen again)
    }
}

/// The selection ends (Select mode stays): an edit elsewhere, an undo, a page change.
pub(super) fn sel_deselect(a: &mut App) {
    let st = sel_ref(a).state;
    if !matches!(st, SelState::Selected | SelState::Dragging) {
        return;
    }
    let swept = take_outline_wear(a);
    let r = rect_union(overlay_rect(a), swept);
    let s = ed(a);
    s.sel.state = SelState::Armed;
    s.sel.ids.clear();
    s.sel.plan.clear();
    s.sel.kept.clear();
    s.sel.links.clear();
    s.sel.box_ = RECT_EMPTY;
    s.sel.drag = (0, 0);
    s.sel.dv = DragView::default(); // (a drag ended by Back, an undo...: its copy of the screen goes)
    layout_bar(a);
    let r = rect_union(r, overlay_rect(a));
    redraw(a, r, RefreshMode::Ui);
    sel_settle_hold(a);
}

/// For callers that draw the whole screen next (a page turn, the Pages view): the selection goes, the
/// mode stays (one a finger's hold entered goes with its selection).
pub(super) fn sel_drop(s: &mut EditorState) {
    if matches!(s.sel.state, SelState::Selected | SelState::Dragging | SelState::Lasso) {
        if s.owner == Owner::Select {
            s.owner = Owner::None;
        }
        let keep = !s.sel.by_finger;
        let (by_hold, holding) = (s.sel.by_hold, s.sel.holding);
        s.sel = SelMode { state: if keep { SelState::Armed } else { SelState::Off }, by_hold, holding, ..Default::default() };
    }
}

// ---- the pointer ----

/// Whether Select mode takes this down at (x, y): the bar, a drag of the selection, or a pen lasso. A
/// finger outside the selection and its bar is the page's (sel_finger_outside ends the selection, and its
/// hold may lasso again).
pub(super) fn sel_takes(s: &EditorState, pen: bool, x: f32, y: f32) -> bool {
    match s.sel.state {
        SelState::Off => false,
        SelState::Armed => pen || rect_contains(s.sel.bar, x as i32, y as i32),
        SelState::Selected if !pen && sel_indices(s).is_some() => {
            rect_contains(s.sel.bar, x as i32, y as i32) || in_grab(s, x, y) || resize_at(s, x as i32, y as i32) > 0
        }
        _ => true,
    }
}

/// Whether screen point (x, y) is where a down drags the selection: its box and a margin.
fn in_grab(s: &EditorState, x: f32, y: f32) -> bool {
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, x, y, &mut px, &mut py);
    let grab = rect_inflate(s.sel.box_, (ui_px(20.0) as f32 / s.zoom) as i32);
    rect_contains(grab, px as i32, py as i32)
}

/// A finger lands on the page outside the selection (sel_takes left it to the page): the selection ends,
/// as a tap outside it ends it (design 6.1). True when there was one: the caller lets that finger's tap
/// and swipe do nothing more, and its hold lasso again.
pub(super) fn sel_finger_outside(a: &mut App) -> bool {
    if sel_ref(a).state != SelState::Selected {
        return false;
    }
    sel_deselect(a);
    true
}

/// A down Select mode takes (sel_takes said so).
pub(super) fn sel_down(a: &mut App, ev: &InEvent, pen: bool) {
    if sel_ref(a).state == SelState::Dragging {
        // A drag whose lift never came and that nothing ended as a cancel (lift_lost and editor_suspend
        // do): ended now, the box and the bar back at their place, and this contact swallowed, since the
        // bar it may land on was not on the screen as it came down.
        qlog!("lasso: a drag was left with no lift: ended as a cancel");
        drag_done(a, true);
        ed(a).owner = Owner::Select;
        return;
    }
    let (x, y) = (ev.x as i32, ev.y as i32);
    let s = ed(a);
    s.owner = Owner::Select;
    if let Some(&(_, b)) = s.sel.btns.iter().find(|(r, _)| rect_contains(*r, x, y)) {
        s.owner = Owner::Select; // (the rest of the contact is swallowed)
        sel_button(a, b);
        return;
    }
    if rect_contains(s.sel.bar, x, y) {
        s.owner = Owner::Select;
        return;
    }
    if s.sel.state == SelState::Selected && sel_indices(s).is_none() {
        sel_deselect(a);
    }
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, ev.x, ev.y, &mut px, &mut py);
    if s.sel.state == SelState::Selected {
        let handle = resize_at(s, x, y);
        if handle > 0 {
            // a corner handle resizes the box about its opposite corner (one undo step at the lift)
            s.sel.state = SelState::Dragging;
            s.sel.resize_handle = handle;
            s.sel.resize_box0 = s.sel.box_;
            s.sel.resize_shown = s.sel.box_;
            s.sel.drag_from = (px, py);
            s.sel.drag = (0, 0);
            s.sel.outline_rect = RECT_EMPTY;
            s.sel.dv = DragView::default();
            return;
        }
        if in_grab(s, ev.x, ev.y) {
            // (nothing drawn yet: a tap here, no move, leaves the screen as it is; the first move begins the
            // drag's display at the next tick, drag_tick)
            s.sel.state = SelState::Dragging;
            s.sel.drag_from = (px, py);
            s.sel.drag = (0, 0);
            s.sel.outline_rect = RECT_EMPTY;
            s.sel.dv = DragView::default();
            return;
        }
        sel_deselect(a);
        if !pen || sel_ref(a).state == SelState::Off {
            // (a finger outside only deselects, and a tap that ended a held Select is no lasso: the rest
            // swallowed)
            ed(a).owner = Owner::Select;
            return;
        }
    }
    if !pen {
        ed(a).owner = Owner::Select;
        return;
    }
    // the lasso begins
    let dpi = a.dpi;
    lasso_begin_at(ed(a), px, py, false, ev.t_ms, dpi);
}

/// The lasso begins at page point (px, py): the pen's at its down, a finger's at its hold; `t` the input's
/// time. Its live preview starts with it (the open page's ink in a grid: well under a millisecond).
fn lasso_begin_at(s: &mut EditorState, px: f32, py: f32, finger: bool, t: u32, dpi: f32) {
    s.sel.state = SelState::Lasso;
    s.sel.pts.clear();
    if s.sel.pts.capacity() < LASSO_MAX {
        let _ = s.sel.pts.try_reserve_exact(LASSO_MAX);
    }
    if s.sel.pts.capacity() > 0 {
        s.sel.pts.push((px, py));
    }
    s.sel.last = (px, py);
    s.sel.spacing = ui_px(2.0) as f32;
    s.sel.lasso_rect = RECT_EMPTY;
    s.sel.down_at = now_ms();
    s.sel.still = true;
    s.sel.finger = finger;
    // Shift held on a paired keyboard as the lasso begins flips the mode for this lasso; Alt (Option on an
    // Apple keyboard) does too
    s.sel.exact = exact_mode() != (hidkeys_mods_held() & (KEYMOD_SHIFT | KEYMOD_ALT) != 0);
    s.sel.kept.clear();
    s.sel.plan.clear();
    let pid = s.open_pid();
    let links: Vec<NavLink> = s.nav.links.iter().filter(|l| l.pid == pid).copied().collect();
    s.sel.live = match s.open_page() {
        Some(p) if !s.sel.pts.is_empty() => {
            let mut lv = LassoLive::new(p, &links, dpi, (px, py));
            lv.set_exact(s.sel.exact);
            Some(Box::new(lv))
        }
        _ => None,
    };
    s.sel.folded = s.sel.pts.len();
    s.sel.halved = false;
    s.sel.fold_at = t;
    s.sel.point_at = t;
    s.sel.preview_rect = RECT_EMPTY;
    s.sel.preview_batches = 0;
    s.sel.preview_refreshes = 0;
}

/// The rest of a contact Select mode owns.
pub(super) fn sel_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    match sel_ref(a).state {
        SelState::Lasso => {
            if sel_ref(a).finger && ev.value & TOUCH_F_BROAD != 0 {
                // a finger's lasso the panel now calls palm-sized: dropped, the rest of the contact ignored
                lasso_drop(a);
                ed(a).owner = Owner::None;
                return;
            }
            if ph == 1 {
                lasso_step(a, ev);
            } else if touch_cancelled(ev) {
                lasso_drop(a);
            } else if sel_ref(a).finger && finger_still(ed(a), ev.x, ev.y) {
                finger_lift(a);
            } else {
                lasso_done(a);
            }
        }
        SelState::Dragging => {
            if sel_ref(a).resize_handle > 0 {
                if ph == 1 {
                    resize_step(a, ev);
                } else {
                    resize_done(a, touch_cancelled(ev));
                }
            } else if ph == 1 {
                drag_step(a, ev);
            } else {
                drag_done(a, touch_cancelled(ev));
            }
        }
        _ => {}
    }
    if ph == 2 {
        ed(a).owner = Owner::None;
    }
}

/// The lasso ends with nothing selected or pasted: its dashes (and a finger's ring) drawn away.
fn lasso_drop(a: &mut App) {
    let r = rect_union(sel_ref(a).lasso_rect, preview_end(ed(a)));
    let s = ed(a);
    s.sel.state = SelState::Armed;
    s.sel.pts.clear();
    s.sel.lasso_rect = RECT_EMPTY;
    s.sel.finger = false;
    redraw(a, r, RefreshMode::Ui);
    sel_settle_hold(a);
}

/// Any pen event (hover, contact, button, leave: the pen is or was just in range), from pb_input: a finger
/// still waiting for its hold will arm nothing, and a finger's armed lasso is dropped, its ring cleared,
/// the rest of that contact ignored (its lift neither pastes nor selects). No surprises: the writing hand
/// or a grip thumb never selects or pastes.
pub(super) fn sel_finger_pen(a: &mut App) {
    let s = ed(a);
    if s.owner_is_pen {
        return;
    }
    if s.owner == Owner::Link {
        s.link_spoiled = true;
    } else if s.owner == Owner::Select && s.sel.state == SelState::Lasso && s.sel.finger {
        qlog!("select: the pen came: the finger's lasso dropped");
        lasso_drop(a);
        ed(a).owner = Owner::None;
    }
}

pub(super) fn sel_cancel(a: &mut App) {
    let cancel = InEvent { type_: InType::TouchUp, x: -10000.0, y: -10000.0, value: TOUCH_F_CANCEL, ..Default::default() };
    sel_pointer(a, 2, &cancel);
}

/// Whether a finger's lasso is still where it was held, (x, y) its place now (screen px): within
/// press_move's slop of the hold, as the hold itself was. Once it has strayed it is a lasso to the lift.
fn finger_still(s: &mut EditorState, x: f32, y: f32) -> bool {
    if s.sel.still {
        let (mut px, mut py) = (0.0, 0.0);
        to_page(s, x, y, &mut px, &mut py);
        if let Some(&(x0, y0)) = s.sel.pts.first() {
            let r = finger_slop() / s.zoom;
            s.sel.still = (px - x0).abs() <= r && (py - y0).abs() <= r;
        }
    }
    s.sel.still
}

fn lasso_step(a: &mut App, ev: &InEvent) {
    let dpi = a.dpi;
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, ev.x, ev.y, &mut px, &mut py);
    if s.sel.finger {
        finger_still(s, ev.x, ev.y);
    } else if s.sel.still {
        if let Some(&(x0, y0)) = s.sel.pts.first() {
            // (2 mm on the screen: a hand's tremor, not a lasso)
            let r = mm_to_px(2.0, dpi) / s.zoom;
            s.sel.still = (px - x0) * (px - x0) + (py - y0) * (py - y0) <= r * r;
        }
    }
    let prev = s.sel.last;
    let zoom = s.zoom;
    let m = &mut s.sel;
    let before = m.pts.len();
    if !lasso_keep(&mut m.pts, &mut m.spacing, &mut m.last, (px, py), zoom) {
        return;
    }
    if m.pts.len() <= before && before > 0 {
        m.halved = true; // (a full lasso dropped every other point)
    }
    m.point_at = ev.t_ms;
    if m.folded + 1 >= m.pts.len() && !m.halved {
        // (the first point waiting: the preview's window starts with it)
        m.fold_at = ev.t_ms;
    }
    let (p0, p1) = (lasso_screen_pt(s, prev), lasso_screen_pt(s, (px, py)));
    let (thick, _, _) = lasso_pen();
    let seg = rect_from_points(p0.0, p0.1, p1.0, p1.1, thick as f32 + 1.0);
    let hidden = rect_overlaps(seg, s.panel) || rect_overlaps(seg, s.sel.bar);
    let clip = lasso_clip(a);
    let keep = a.screen.clip;
    canvas_set_clip(&mut a.screen, clip);
    canvas_dither_poly(&mut a.screen, &[p0, p1], if hidden { 0 } else { thick }, 0);
    a.screen.clip = keep;
    let d = rect_intersect(seg, clip);
    let s = ed(a);
    s.sel.lasso_rect = rect_union(s.sel.lasso_rect, d);
    if !hidden && !rect_is_empty(d) {
        app_invalidate(a, d, RefreshMode::Fast);
    }
    if (ev.t_ms.wrapping_sub(sel_ref(a).fold_at) as i32) >= PREVIEW_MS as i32 {
        preview_fold(a, ev.t_ms, true);
    }
}

// ---- the live preview (docs/beta2/PIXEL-LASSO.md 2) ----

/// The halo's width each side of the ink, screen px.
fn halo_px() -> i32 {
    ui_px(HALO_UI).max(1)
}

/// The screen rect stroke st's halo may cover.
fn halo_rect(s: &EditorState, st: &Stroke) -> Rect {
    rect_inflate(box_screen(s, st.bbox), halo_px() + 2)
}

/// The preview folds in the lasso's points not folded yet (the polygon closed from the newest one), and
/// the ink whose halo came or went is drawn again by fast refreshes (`show`: the lift redraws instead).
fn preview_fold(a: &mut App, t: u32, show: bool) {
    let dpi = a.dpi;
    let s = ed(a);
    let Some(mut live) = s.sel.live.take() else { return };
    s.sel.fold_at = t;
    let (folded, halved) = (s.sel.folded, s.sel.halved);
    let pts = std::mem::take(&mut s.sel.pts);
    let halo = halo_px();
    let zoom = s.zoom;
    // (the strokes that flipped: their halos' screen rects and about how many px those cover)
    let out: Option<(Vec<Rect>, i64)> = match s.open_page().filter(|p| live.holds(p)) {
        None => None,
        Some(_) if folded >= pts.len() && !halved => Some((Vec::new(), 0)),
        Some(p) => {
            let flips = if halved { live.rebuild(p, &pts) } else { live.extend(p, &pts, folded) };
            let mut rects = Vec::with_capacity(flips.len());
            let mut area: i64 = 0;
            for &i in &flips {
                let st = &p.strokes[i];
                rects.push(halo_rect(s, st));
                let w = (2.0 * mm_to_px(st.params.size_max_mm.max(0.1), dpi) * zoom) as i64 + 2 * halo as i64;
                area += ((stroke_length(st) * zoom) as i64 + w) * w;
            }
            Some((rects, area))
        }
    };
    s.sel.pts = pts;
    let Some((rects, area)) = out else {
        // (the page changed under the lasso: no preview; its halos go)
        let r = std::mem::take(&mut s.sel.preview_rect);
        if show {
            redraw(a, r, RefreshMode::Fast);
        }
        return;
    };
    s.sel.halved = false;
    s.sel.folded = s.sel.pts.len();
    s.sel.preview_batches = live.batches;
    s.sel.live = Some(live);
    if !show || rects.is_empty() {
        return;
    }
    let clip = lasso_clip(a);
    let rects = merge_rects(rects, PREVIEW_RECTS, ui_px(32.0));
    let mut all = RECT_EMPTY;
    for r in rects {
        let r = rect_intersect(r, clip);
        if rect_is_empty(r) {
            continue;
        }
        redraw(a, r, RefreshMode::Fast);
        all = rect_union(all, r);
        ed(a).sel.preview_refreshes += 1;
    }
    let s = ed(a);
    s.sel.preview_rect = rect_union(s.sel.preview_rect, all);
    du_wear_add(&mut a.du_wear, all, area, now_ms());
}

/// Rects merged greedily: any two within `near` px of each other join, then the closest pairs until at
/// most `max` are left.
fn merge_rects(mut v: Vec<Rect>, max: usize, near: i32) -> Vec<Rect> {
    v.retain(|r| !rect_is_empty(*r));
    loop {
        let mut best: Option<(i64, usize, usize)> = None;
        for i in 0..v.len() {
            for j in i + 1..v.len() {
                let (a, b) = (v[i], v[j]);
                let gx = (a.x.max(b.x) - (a.x + a.w).min(b.x + b.w)).max(0) as i64;
                let gy = (a.y.max(b.y) - (a.y + a.h).min(b.y + b.h)).max(0) as i64;
                let gap = gx.max(gy);
                if best.is_none_or(|bb| gap < bb.0) {
                    best = Some((gap, i, j));
                }
            }
        }
        match best {
            Some((gap, i, j)) if gap <= near as i64 || v.len() > max => {
                let u = rect_union(v[i], v[j]);
                v.swap_remove(j);
                v[i] = u;
            }
            _ => return v,
        }
    }
}

/// The preview ends with its lasso: its state goes; the screen rect its refreshes covered, which the
/// caller draws again.
fn preview_end(s: &mut EditorState) -> Rect {
    s.sel.preview_seen = s.sel.preview_rect;
    s.sel.live = None;
    s.sel.folded = 0;
    s.sel.halved = false;
    std::mem::take(&mut s.sel.preview_rect)
}

/// editor_tick: a pen that paused with lasso points not folded in yet, or one still moving past
/// PREVIEW_MS since the last fold, gets them folded now.
fn preview_tick(a: &mut App, now: u32) {
    let m = sel_ref(a);
    if m.state != SelState::Lasso || m.live.is_none() || (m.folded >= m.pts.len() && !m.halved) {
        return;
    }
    if (now.wrapping_sub(m.point_at) as i32) >= PREVIEW_PAUSE_MS as i32 || (now.wrapping_sub(m.fold_at) as i32) >= PREVIEW_MS as i32 {
        preview_fold(a, now, true);
    }
}

/// The halo on the ink the lasso being drawn would select (the preview's) or a selection holds: a dithered
/// band HALO_UI wider than the ink each side, the ink kept on top; in exact area, only along the runs of
/// a crossing stroke inside the lasso, with a short solid tick where it will be cut (the preview's; the
/// selection's inside pieces get the whole halo). compose calls it over the page, under the panel and the
/// menus.
pub(super) fn sel_draw_halo(a: &mut App, r: Rect) {
    let dpi = a.dpi;
    let clip = rect_intersect(r, lasso_clip(a));
    let s = ed_ref(a);
    if rect_is_empty(clip) || !s.open || sheet_open(s) || s.peek.active {
        return;
    }
    let Some(p) = s.open_page() else { return };
    // what has a halo: strokes whole, or exact area's inside runs
    let mut items: Vec<(&Stroke, Option<Vec<(f32, f32, bool)>>)> = Vec::new();
    match s.sel.state {
        SelState::Lasso => match s.sel.live.as_deref() {
            Some(l) if l.holds(p) => {
                for (i, runs) in l.halo_list() {
                    items.push((&p.strokes[i], runs));
                }
            }
            _ => return,
        },
        SelState::Selected | SelState::Dragging => {
            let Some(v) = sel_indices(s) else { return };
            for i in v {
                items.push((&p.strokes[i], None));
            }
            for (_, pieces) in &s.sel.plan {
                for (pc, inside) in pieces {
                    if *inside {
                        items.push((&**pc, None));
                    }
                }
            }
        }
        _ => return,
    }
    items.retain(|(st, _)| rect_overlaps(halo_rect(s, st), clip));
    if items.is_empty() {
        return;
    }
    let mut u = RECT_EMPTY;
    for (st, _) in &items {
        u = rect_union(u, halo_rect(s, st));
    }
    let u = rect_intersect(u, rect_intersect(clip, Rect { x: 0, y: 0, w: a.screen.w, h: a.screen.h }));
    if rect_is_empty(u) {
        return;
    }
    let halo = halo_px();
    let (zoom, ox, oy) = (s.zoom, s.ox, s.oy);
    let scr = |x: f32, y: f32| ((x - ox) * zoom, (y - oy) * zoom);
    let near = mm_to_px(1.6, dpi);
    // polylines (screen px, each point with its band's width) and ticks (screen segments)
    let mut paths: Vec<Vec<(f32, f32, i32)>> = Vec::new();
    let mut ticks: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for (st, runs) in &items {
        match runs {
            None => paths.push(
                st.pts
                    .iter()
                    .map(|q| {
                        let (x, y) = scr(q.x, q.y);
                        (x, y, (2.0 * dab_radius(st, q, dpi) * zoom).round() as i32 + 2 * halo)
                    })
                    .collect(),
            ),
            Some(v) => {
                let ink = 2.0 * mm_to_px(st.params.size_max_mm.max(0.1), dpi) * 0.5 * zoom;
                let t = ink.round() as i32 + 2 * halo;
                let mut cur: Vec<(f32, f32, i32)> = Vec::new();
                for k in 0..v.len() {
                    let (x, y, inside) = v[k];
                    let joined = k > 0 && ((x - v[k - 1].0).powi(2) + (y - v[k - 1].1).powi(2)).sqrt() <= near;
                    if k > 0 && joined && inside != v[k - 1].2 {
                        // a cut here: a tick across the ink halfway between the two samples
                        let (mx, my) = ((x + v[k - 1].0) / 2.0, (y + v[k - 1].1) / 2.0);
                        let (dx, dy) = (x - v[k - 1].0, y - v[k - 1].1);
                        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
                        let hl = (ink / 2.0 + ui_px(3.0) as f32) / zoom;
                        let (nx, ny) = (-dy / l * hl, dx / l * hl);
                        ticks.push((scr(mx + nx, my + ny), scr(mx - nx, my - ny)));
                    }
                    if !inside || !joined {
                        if cur.len() > 0 {
                            paths.push(std::mem::take(&mut cur));
                        }
                    }
                    if inside {
                        let (sx, sy) = scr(x, y);
                        cur.push((sx, sy, t));
                    }
                }
                if !cur.is_empty() {
                    paths.push(cur);
                }
            }
        }
    }
    // the page under the halos, to put the ink back over them
    let stride = a.screen.stride;
    let mut under = Vec::with_capacity((u.w * u.h) as usize);
    for y in u.y..u.y + u.h {
        let at = (y * stride + u.x) as usize;
        under.extend_from_slice(&a.screen.px[at..at + u.w as usize]);
    }
    let keep = a.screen.clip;
    canvas_set_clip(&mut a.screen, u);
    let c = &mut a.screen;
    let square = |x: f32, y: f32, t: i32| Rect { x: (x - t as f32 / 2.0 + 0.5).floor() as i32, y: (y - t as f32 / 2.0 + 0.5).floor() as i32, w: t, h: t };
    for path in &paths {
        if let Some(&(x, y, t)) = path.first() {
            canvas_fill_dither(c, square(x, y, t));
        }
        for w in path.windows(2) {
            let (a0, b0) = (w[0], w[1]);
            let t = a0.2.min(b0.2).max(1);
            let (dx, dy) = (b0.0 - a0.0, b0.1 - a0.1);
            let l = (dx * dx + dy * dy).sqrt();
            let step = (t as f32 / 4.0).max(1.0);
            let n = if l.is_finite() { (l / step).ceil().min(1.0e5) as i32 } else { 0 };
            for k in 1..=n {
                let f = k as f32 / n as f32;
                let tt = a0.2 + ((b0.2 - a0.2) as f32 * f) as i32;
                canvas_fill_dither(c, square(a0.0 + dx * f, a0.1 + dy * f, tt.max(1)));
            }
        }
    }
    for y in u.y..u.y + u.h {
        let at = (y * stride + u.x) as usize;
        let row = &mut c.px[at..at + u.w as usize];
        let src = &under[((y - u.y) * u.w) as usize..((y - u.y + 1) * u.w) as usize];
        for (d, &o) in row.iter_mut().zip(src) {
            *d = (*d).min(o);
        }
    }
    // the ticks, solid over the ink
    let tw = ui_px(3.0).max(1);
    for (p0, p1) in &ticks {
        let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
        let n = ((dx * dx + dy * dy).sqrt().ceil() as i32).clamp(1, 4096);
        for k in 0..=n {
            let f = k as f32 / n as f32;
            canvas_fill(c, square(p0.0 + dx * f, p0.1 + dy * f, tw), 0);
        }
    }
    a.screen.clip = keep;
    let m = &mut ed(a).sel;
    if m.state == SelState::Dragging {
        m.dv.inked += 1; // (the probe: a drag that moves draws none, but where its first move takes the outline away)
    }
}

/// The lasso lifted: what it encloses is selected (a toast when nothing is).
fn lasso_done(a: &mut App) {
    let dpi = a.dpi;
    // the preview folds in the last points (and the closing edge): its answer is the lift's
    let t = sel_ref(a).point_at;
    preview_fold(a, t, false);
    let live = ed(a).sel.live.take();
    let drawn = rect_union(sel_ref(a).lasso_rect, preview_end(ed(a)));
    let s = ed(a);
    s.sel.preview_reused = false;
    let pts = std::mem::take(&mut s.sel.pts);
    s.sel.state = SelState::Armed;
    s.sel.lasso_rect = RECT_EMPTY;
    s.sel.finger = false;
    let pid = s.open_pid();
    let lasso = Lasso::new(&pts);
    let page_links: Vec<NavLink> = s.nav.links.iter().filter(|l| l.pid == pid).copied().collect();
    let t0 = now_ms();
    let mut reused = false;
    let exact = s.sel.exact;
    let picked = match (&lasso, s.open_page()) {
        (Some(l), Some(p)) => {
            let on = match live.as_deref().filter(|lv| lv.holds(p)) {
                Some(lv) => {
                    reused = true;
                    let on = lv.enclosed();
                    debug_assert!(on == lasso_enclosed(p, l, dpi), "the preview's answer is the lift's");
                    on
                }
                None => lasso_enclosed(p, l, dpi),
            };
            Some((judge(p, &page_links, l, on, exact, dpi), p.strokes.iter().any(|st| stroke_is_erase(st) && rect_overlaps(st.bbox, l.bounds))))
        }
        _ => None,
    };
    let ms = now_ms().wrapping_sub(t0);
    s.sel.preview_reused = reused;
    if let Some(lv) = live.as_deref() {
        qlog!("lasso: preview batches=%d made=%d tests=%u reused=%d", lv.batches as i32, lv.made as i32, lv.tests.min(u32::MAX as u64) as u32, reused as i32);
    }
    let erasure_there = picked.as_ref().is_some_and(|p| p.1);
    let Some((plan, _)) = picked.filter(|(plan, _)| !plan.is_empty()) else {
        ed(a).sel.lift_rect = drawn;
        redraw(a, drawn, RefreshMode::Ui);
        if erasure_there {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a lasso in Select mode went
            // around only white ink or eraser marks, which are not objects: they move with the ink they cover.
            app_toast(a, tr("Nothing to select: white ink and eraser marks move with the ink under them."), 3500);
        } else {
            // TRANSLATORS: a toast: a lasso in Select mode went around no ink.
            app_toast(a, tr("Nothing to select."), 2000);
        }
        sel_settle_hold(a);
        return;
    };
    let s = ed(a);
    let (n, ncut) = (plan.taken(), plan.cut.len());
    sel_take(s, plan, &page_links, pts);
    qlog!("lasso: select n=%d cut=%d links=%d exact=%d ms=%u", n as i32, ncut as i32, s.sel.links.len() as i32, exact as i32, ms);
    let quiet = std::mem::take(&mut s.sel.quiet);
    layout_bar(a);
    let r = rect_union(drawn, overlay_rect(a));
    ed(a).sel.lift_rect = r;
    redraw(a, r, RefreshMode::Ui);
    if quiet {
        repaint_panel(a, RefreshMode::Ui); // (a finger's hold entered Select mode: its tool button shows the lasso now)
    }
}

/// What lasso l selects on page p in a mode, given the whole-strokes enclosure `on`: in whole strokes, a
/// plan with no cut (lasso_select's answer); in exact area, lasso_cut_plan's (timed and logged).
fn judge(p: &Page, links: &[NavLink], l: &Lasso, on: Vec<bool>, exact: bool, dpi: f32) -> CutPlan {
    if !exact {
        let sel = lasso_select_from(p, links, l, on);
        return CutPlan { whole: sel.strokes, cut: Vec::new(), links: sel.links };
    }
    let t0 = now_ms();
    let plan = lasso_cut_plan(p, links, l, dpi, on);
    let pieces: usize = plan.cut.iter().map(|c| c.1.len()).sum();
    qlog!("lasso: exact cut=%d pieces=%d walk_ms=%u", plan.cut.len() as i32, pieces as i32, now_ms().wrapping_sub(t0));
    plan
}

/// The selection becomes plan (made on the open page, whose links are `page_links`), its lasso `pts` kept
/// for the mode button.
fn sel_take(s: &mut EditorState, plan: CutPlan, page_links: &[NavLink], pts: Vec<(f32, f32)>) {
    let pid = s.open_pid();
    let p = s.open_page().unwrap();
    let ids: Vec<StrokeId> = plan.whole.iter().map(|&i| stroke_id(&p.strokes[i])).collect();
    let mut b = RECT_EMPTY;
    for &i in &plan.whole {
        b = rect_union(b, p.strokes[i].bbox);
    }
    for (_, v) in &plan.cut {
        for (pc, inside) in v {
            if *inside {
                b = rect_union(b, pc.bbox);
            }
        }
    }
    let cut: Vec<(StrokeId, Vec<(Box<Stroke>, bool)>)> = plan.cut.into_iter().map(|(i, v)| (stroke_id(&p.strokes[i]), v)).collect();
    let links: Vec<NavLink> = plan.links.iter().map(|&k| page_links[k]).collect();
    for l in &links {
        if l.has_area {
            b = rect_union(b, l.area);
        }
    }
    s.sel.state = SelState::Selected;
    s.sel.pid = pid;
    s.sel.ids = ids;
    s.sel.plan = cut;
    s.sel.links = links;
    s.sel.kept = pts;
    s.sel.box_ = rect_intersect(b, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh });
    s.sel.drag = (0, 0);
}

/// The bar's mode button: whole strokes or exact area for the lassos to come; with a selection, its lasso
/// judged again at once in the new mode (nothing on the page changes until Move, Cut or Delete).
fn sel_mode_toggle(a: &mut App) {
    let dpi = a.dpi;
    let now = !exact_mode();
    EXACT.store(now, std::sync::atomic::Ordering::Relaxed);
    let before = overlay_rect(a);
    let s = ed(a);
    s.sel.exact = now;
    if s.sel.state == SelState::Selected && !s.sel.kept.is_empty() && sel_indices(s).is_some() {
        let pts = std::mem::take(&mut s.sel.kept);
        let pid = s.open_pid();
        let page_links: Vec<NavLink> = s.nav.links.iter().filter(|l| l.pid == pid).copied().collect();
        let plan = match (Lasso::new(&pts), s.open_page()) {
            (Some(l), Some(p)) => Some(judge(p, &page_links, &l, lasso_enclosed(p, &l, dpi), now, dpi)),
            _ => None,
        };
        match plan.filter(|pl| !pl.is_empty()) {
            Some(plan) => {
                let (n, ncut) = (plan.taken(), plan.cut.len());
                sel_take(s, plan, &page_links, pts);
                qlog!("lasso: judged again n=%d cut=%d exact=%d", n as i32, ncut as i32, now as i32);
            }
            None => {
                // (nothing in the other mode: the selection goes, Select mode stays)
                s.sel.kept = pts;
                sel_deselect(a);
                app_toast(a, tr("Nothing to select."), 2000);
                return;
            }
        }
    }
    layout_bar(a);
    let r = rect_union(before, overlay_rect(a));
    redraw(a, r, RefreshMode::Ui);
}

/// A pen sample of the drag: the newest place (whole page px, clamped so the box stays on the page) and its
/// time, nothing drawn; drag_tick shows it. None while the pen is still within DRAG_SLOP_MM of where it came
/// down. A sample costs the same for three words as for a page of shading, so the main loop takes every
/// sample waiting in one turn and the outline is never behind the pen.
fn drag_step(a: &mut App, ev: &InEvent) {
    let dpi = a.dpi;
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, ev.x, ev.y, &mut px, &mut py);
    if !s.sel.dv.left {
        // (within the slop the box stays; past it, it follows the pen from where it came down)
        let r = if s.owner_is_pen { mm_to_px(DRAG_SLOP_MM, dpi) } else { finger_slop() } / s.zoom;
        let (ex, ey) = (px - s.sel.drag_from.0, py - s.sel.drag_from.1);
        if ex * ex + ey * ey <= r * r {
            return;
        }
        s.sel.dv.left = true;
    }
    let b = s.sel.box_;
    let dx = ((px - s.sel.drag_from.0).round() as i32).clamp(-b.x, (s.pgw - b.x - b.w).max(-b.x));
    let dy = ((py - s.sel.drag_from.1).round() as i32).clamp(-b.y, (s.pgh - b.y - b.h).max(-b.y));
    if (dx, dy) == s.sel.drag {
        return;
    }
    s.sel.drag = (dx, dy);
    s.sel.dv.moved_at = now_ms();
    s.sel.dv.samples += 1;
}

/// A pen sample of a resize: the box its newest place makes of the one it began with (resize_box), nothing
/// drawn; resize_tick shows it.
fn resize_step(a: &mut App, ev: &InEvent) {
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, ev.x, ev.y, &mut px, &mut py);
    let b = resize_box(s, s.sel.resize_box0, s.sel.resize_handle, px, py);
    if b == s.sel.box_ {
        return;
    }
    s.sel.box_ = b;
    s.sel.dv.moved_at = now_ms();
    s.sel.dv.samples += 1;
    s.sel.dv.left = true;
}

/// The screen rects a resize's frame covers at box b (page px): the outline's four strips and its four
/// handles, cut to where they draw (lasso_clip).
fn resize_frame(a: &App, b: Rect) -> Vec<Rect> {
    let s = ed_ref(a);
    if rect_is_empty(b) || s.sel.pid != s.open_pid() {
        return Vec::new();
    }
    let clip = lasso_clip(a);
    let at = box_at(s, b, (0, 0));
    let mut v = Vec::with_capacity(8);
    for st in dither_strips(rect_inflate(at, ui_px(6.0)), link_band(), ui_px(1.0)) {
        let st = rect_intersect(st, clip);
        if !rect_is_empty(st) {
            v.push(st);
        }
    }
    for h in resize_handles(at, resize_handle_size()) {
        let h = rect_intersect(rect_inflate(h, ui_px(1.0)), clip);
        if !rect_is_empty(h) {
            v.push(h);
        }
    }
    v
}

/// A frame's rects as one (screen px).
fn frame_rect(v: &[Rect]) -> Rect {
    v.iter().fold(RECT_EMPTY, |u, &r| rect_union(u, r))
}

/// The screen px a resize's frame at box b turns back to the page as it moves: the outline's ink (its
/// carried link areas' thinner ones left out) and its four handles.
fn resize_ink(a: &App, b: Rect) -> i64 {
    let s = ed_ref(a);
    if rect_is_empty(b) {
        return 0;
    }
    let ib = rect_inflate(box_at(s, b, (0, 0)), ui_px(6.0));
    let (w, h, t) = (ib.w as i64, ib.h as i64, link_band() as i64);
    let band = (2 * (w + h) * t).min(w * h);
    let sz = resize_handle_size() as i64;
    band + 4 * sz * sz
}

/// A resize's first show: the bar and the outline's frame where they are taken away composed again over
/// the page (one compose over both), the screen without them copied, the frame drawn at the box's newest
/// place, and one DU over both places at once. As a drag's first show (drag_begin): with the copy, every show
/// after it is a blit of that copy and a draw of the frame, whatever the page holds behind them (no page
/// composed, no ink walked).
fn resize_begin(a: &mut App, now: u32) {
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let bar = rect_intersect(rect_union(sel_ref(a).bar, std::mem::take(&mut ed(a).sel.stale)), screen);
    let box0 = sel_ref(a).resize_shown;
    let old = resize_frame(a, box0);
    let worn = resize_ink(a, box0);
    let gone = rect_union(bar, frame_rect(&old));
    if !rect_is_empty(gone) {
        // (one compose over both: composing a strip at a time walks the selection's ink again for each, the
        // halo drawn with it, so six composes cost six walks over every selected point)
        compose(a, gone);
    }
    let at = rect_intersect(lasso_clip(a), screen);
    let mut base = Canvas::default();
    let based = !rect_is_empty(at) && canvas_init(&mut base, at.w, at.h) == 0;
    if based {
        canvas_blit(&mut base, &a.screen, at, 0, 0);
    } else {
        qlog!("lasso: no memory for the resize's copy of the screen: each show composes its strips");
    }
    let s = ed(a);
    s.sel.dv.moving = true;
    s.sel.dv.base = based.then_some(base);
    s.sel.dv.base_at = at;
    s.sel.dv.samples_at_last_show = s.sel.dv.samples;
    let new = resize_frame(a, ed_ref(a).sel.box_);
    let r = rect_union(gone, frame_rect(&new));
    if rect_is_empty(r) {
        return;
    }
    draw_outline(a, (0, 0), r);
    draw_resize_handles(a, (0, 0), r);
    live_show(a, &[r]);
    app_invalidate(a, r, RefreshMode::Fast);
    app_flush(a);
    let m = &mut ed(a).sel;
    m.resize_shown = m.box_;
    m.outline_rect = rect_union(m.outline_rect, r);
    m.dv.shown_at = now;
    m.dv.shows += 1;
    m.dv.px += rect_area(r);
    du_wear_add(&mut a.du_wear, gone, worn, now_ms());
}

/// A resize's next show: the frame where it was put back to the page (from the copy of the screen, or
/// composed again where there is none), its new place drawn, one DU over both.
fn resize_show(a: &mut App, now: u32) {
    let was = sel_ref(a).resize_shown;
    let to = sel_ref(a).box_;
    let old = resize_frame(a, was);
    let new = resize_frame(a, to);
    let r = rect_union(frame_rect(&old), frame_rect(&new));
    if rect_is_empty(r) {
        return;
    }
    let worn = resize_ink(a, was);
    let mut based = false;
    {
        let App { screen, ed, .. } = a;
        let dv = &ed.as_deref().unwrap().sel.dv;
        if let Some(base) = dv.base.as_ref() {
            let at = dv.base_at;
            let rr = rect_intersect(r, at);
            if !rect_is_empty(rr) {
                canvas_blit(screen, base, Rect { x: rr.x - at.x, y: rr.y - at.y, ..rr }, rr.x, rr.y);
            }
            based = true;
        }
    }
    if !based {
        compose(a, r); // (one compose, one walk of the selection's ink: see resize_begin)
    }
    draw_outline(a, (0, 0), r);
    draw_resize_handles(a, (0, 0), r);
    live_show(a, &[r]);
    app_invalidate(a, r, RefreshMode::Fast);
    app_flush(a);
    let m = &mut ed(a).sel;
    m.resize_shown = m.box_;
    m.outline_rect = rect_union(m.outline_rect, r);
    m.dv.shown_at = now;
    m.dv.shows += 1;
    m.dv.px += rect_area(r);
    du_wear_add(&mut a.du_wear, frame_rect(&old), worn, now_ms());
}

/// editor_tick: a resize's newest box shown, once the panel has finished the last fast refresh, as a
/// drag's (drag_tick). One DU a show, over both places at once: the box follows the pen at the panel's own
/// rate (about 5 a second on the Kindle, its DU's length), whatever the page holds, and never one DU
/// queued behind another.
fn resize_tick(a: &mut App, now: u32) {
    let m = sel_ref(a);
    if m.state != SelState::Dragging || m.resize_handle == 0 || m.resize_shown == m.box_ {
        return;
    }
    if !drag_can_show(a) || display_fast_busy() {
        return;
    }
    if m.dv.shows == 0 {
        resize_begin(a, now);
    } else {
        resize_show(a, now);
    }
}

/// Whether the screen shows the page and the selection's overlay over it, so a drag's show may draw (not
/// under a sheet, the page peek or the Library's settings: what composes them does not draw the outline).
fn drag_can_show(a: &App) -> bool {
    let s = ed_ref(a);
    s.open && !sel_ref(a).quiet && !sheet_open(s) && !s.peek.active && !crate::library::library_settings_active(a)
}

/// editor_tick: a drag's newest place shown once the panel has finished the last fast refresh (on the Kindle,
/// display_fast_busy: never one queued behind a DU still on the panel) and DRAG_SHOW_MS went by since the last
/// show, or the pen kept still DRAG_PAUSE_MS, or DRAG_SHOW_SAMPLES samples came since the last show (a burst
/// of samples in one turn is one show). The first move begins the display (drag_begin). One show a turn at
/// most, after the turn's samples: the newest place.
fn drag_tick(a: &mut App, now: u32) {
    let m = sel_ref(a);
    if m.state != SelState::Dragging {
        return;
    }
    if m.resize_handle > 0 {
        resize_tick(a, now);
        return;
    }
    if (!m.dv.moving && m.drag == (0, 0)) || (m.dv.moving && m.dv.shown == Some(m.drag)) {
        return;
    }
    if !drag_can_show(a) || display_fast_busy() {
        return;
    }
    if !m.dv.moving {
        drag_begin(a, now);
        return;
    }
    let by_samples = m.dv.samples >= m.dv.samples_at_last_show + DRAG_SHOW_SAMPLES;
    let by_time = now.wrapping_sub(m.dv.shown_at) as i32 >= DRAG_SHOW_MS as i32;
    let by_pause = now.wrapping_sub(m.dv.moved_at) as i32 >= DRAG_PAUSE_MS as i32;
    if by_samples || by_time || by_pause {
        drag_show(a, now);
    }
}

/// The drag's first move shown: the bar and the outline taken away (composed again where they were, one
/// compose over both), the screen without them copied (over the page's part of it), the outline drawn
/// at the newest place; the bar and each strip a DU of its own. DU turned the bar and the outline back to
/// the page: their traces count toward a clean (display.rs, DuWear).
fn drag_begin(a: &mut App, now: u32) {
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let bar = rect_intersect(rect_union(sel_ref(a).bar, std::mem::take(&mut ed(a).sel.stale)), screen);
    let mut old = outline_strips(a, (0, 0));
    // (the selection's resize handles go with its outline, into the copy of the screen without them: a move
    // drag has them on the screen as it begins)
    let s = ed_ref(a);
    let handles: Vec<Rect> = if s.sel.resize_handle == 0 && !rect_is_empty(s.sel.box_) && s.open_page().is_some_and(|p| !p.readonly) {
        resize_handles(box_at(s, s.sel.box_, (0, 0)), resize_handle_size()).to_vec()
    } else {
        Vec::new()
    };
    let worn = rect_area(sel_ref(a).bar) + outline_ink(a, (0, 0)) + handles.iter().map(|&r| rect_area(r)).sum::<i64>();
    old.extend(handles);
    let s = ed(a);
    s.sel.dv.moving = true;
    s.sel.dv.shown = None;
    let gone = old.iter().fold(bar, |u, &r| rect_union(u, r));
    if !rect_is_empty(gone) {
        // (one compose over the bar and the outline's strips together: composing them a strip at a time walks
        // the selection's ink and its halo again for each, six walks where one does)
        compose(a, gone);
    }
    let at = rect_intersect(lasso_clip(a), screen);
    let mut base = Canvas::default();
    let based = !rect_is_empty(at) && canvas_init(&mut base, at.w, at.h) == 0;
    if based {
        canvas_blit(&mut base, &a.screen, at, 0, 0);
    } else {
        qlog!("lasso: no memory for the drag's copy of the screen: each show composes its strips");
    }
    let s = ed(a);
    let off = s.sel.drag;
    s.sel.dv.base = based.then_some(base);
    s.sel.dv.base_at = at;
    s.sel.dv.shown = Some(off);
    s.sel.dv.samples_at_last_show = s.sel.dv.samples;
    let new = outline_strips(a, off);
    if based {
        draw_outline(a, off, screen);
    } else if !rect_is_empty(gone) {
        compose(a, gone);
    }
    let mut v = Vec::with_capacity(1 + old.len() + new.len());
    v.push(bar);
    v.extend(old);
    v.extend(new);
    drag_present(a, v, now);
    du_wear_add(&mut a.du_wear, gone, worn, now_ms());
}

/// The drag's newest place shown: the outline's strips where it was put back (from the copy of the screen,
/// or composed again without it), the new ones drawn, each a DU of its own; the old outline counted as DU
/// wear. No ink and no halo drawn, nothing composed (with the copy): the same few hundred microseconds
/// whatever the selection holds.
fn drag_show(a: &mut App, now: u32) {
    let Some(was) = sel_ref(a).dv.shown else { return };
    let old = outline_strips(a, was);
    let worn = outline_ink(a, was);
    let off = sel_ref(a).drag;
    ed(a).sel.dv.shown = Some(off);
    let new = outline_strips(a, off);
    let based = {
        let App { screen, ed, .. } = a;
        let dv = &ed.as_deref().unwrap().sel.dv;
        match dv.base.as_ref() {
            Some(base) => {
                let at = dv.base_at;
                for &r in &old {
                    let r = rect_intersect(r, at);
                    if !rect_is_empty(r) {
                        canvas_blit(screen, base, Rect { x: r.x - at.x, y: r.y - at.y, ..r }, r.x, r.y);
                    }
                }
                true
            }
            None => false,
        }
    };
    if based {
        draw_outline(a, off, Rect { x: 0, y: 0, w: a.w, h: a.h });
    } else {
        let all = old.iter().chain(new.iter()).fold(RECT_EMPTY, |u, &r| rect_union(u, r));
        if !rect_is_empty(all) {
            compose(a, all);
        }
    }
    let gone = old.iter().fold(RECT_EMPTY, |u, &r| rect_union(u, r));
    let mut v = old;
    v.extend(new);
    drag_present(a, v, now);
    du_wear_add(&mut a.du_wear, gone, worn, now_ms());
}

/// A show's rects pushed to Live view in one frame before the panel's refreshes, cut to the page's part
/// of the screen (the panel has its own layer): a browser never draws the old outline back strip by strip
/// while the panel takes them one DU at a time (issue #50: Live view showed a drag's restored strips
/// before the new place). The screen already holds everything the show draws.
fn live_show(a: &App, rects: &[Rect]) {
    if !live_on() {
        return;
    }
    let all = rects.iter().fold(RECT_EMPTY, |u, &r| rect_union(u, r));
    let all = rect_intersect(all, lasso_clip(a));
    if !rect_is_empty(all) {
        live_push(&a.screen, all);
    }
}

/// A drag's rects refreshed by DU, each flushed on its own (the link card's border strips' way: one rect
/// around them all would span the gray ink inside the box), two joined only when one rect over both
/// covers no more than they do apart (a short move's strips overlap), at most DRAG_RECTS. They join the
/// drag's outline_rect, which its lift draws again in grays.
fn drag_present(a: &mut App, mut v: Vec<Rect>, now: u32) {
    v.retain(|r| !rect_is_empty(*r));
    loop {
        let mut best: Option<(i64, usize, usize)> = None;
        for i in 0..v.len() {
            for j in i + 1..v.len() {
                let waste = rect_area(rect_union(v[i], v[j])) - rect_area(v[i]) - rect_area(v[j]);
                if best.is_none_or(|b| waste < b.0) {
                    best = Some((waste, i, j));
                }
            }
        }
        match best {
            Some((waste, i, j)) if waste <= 0 || v.len() > DRAG_RECTS => {
                let u = rect_union(v[i], v[j]);
                v.swap_remove(j);
                v[i] = u;
            }
            _ => break,
        }
    }
    if !rect_is_empty(a.dirty) {
        app_flush(a); // (what was waiting goes in its own refresh, as it would have)
    }
    live_show(a, &v);
    let mut px = 0i64;
    let mut all = RECT_EMPTY;
    for r in v {
        app_invalidate(a, r, RefreshMode::Fast);
        app_flush(a);
        px += rect_area(r);
        all = rect_union(all, r);
    }
    let m = &mut ed(a).sel;
    m.outline_rect = rect_union(m.outline_rect, all);
    m.dv.shown_at = now;
    m.dv.shows += 1;
    m.dv.px += px;
}

/// The outline the drag (or resize) has shown by fast refreshes, taken for the lift: the screen rect it
/// swept, with its newest place counted as DU wear. The lift draws the outline away in grays, but DU leaves
/// its trace there, which only the wear's clean reaches; without this the newest outline stayed faintly on
/// the panel after the selection was dropped (issue #50).
fn take_outline_wear(a: &mut App) -> Rect {
    let (state, handle, shown, rshown) = {
        let m = sel_ref(a);
        (m.state, m.resize_handle, m.dv.shown, m.resize_shown)
    };
    let swept = std::mem::take(&mut ed(a).sel.outline_rect);
    if !rect_is_empty(swept) {
        let worn = if state == SelState::Dragging {
            if handle > 0 {
                resize_ink(a, rshown)
            } else {
                shown.map_or(0, |off| outline_ink(a, off))
            }
        } else {
            0
        };
        du_wear_add(&mut a.du_wear, swept, worn, now_ms());
    }
    swept
}

/// The drag's display ends (its lift, or a cancel): the log's line for it, and its copy of the screen goes.
fn drag_end_view(m: &mut SelMode) {
    let dv = &mut m.dv;
    if dv.moving {
        qlog!("lasso: drag samples=%u shows=%u px=%lld inked=%u copy=%d", dv.samples, dv.shows, dv.px, dv.inked, dv.base.is_some() as i32);
    }
    dv.moving = false;
    dv.shown = None;
    dv.base = None;
}

fn drag_done(a: &mut App, cancelled: bool) {
    if sel_ref(a).resize_handle > 0 {
        resize_done(a, cancelled);
        return;
    }
    let (dx, dy) = sel_ref(a).drag;
    // (where the drag drew the outline and the bar on its way, as well as where it ends; its newest place
    // counted as DU wear, so the clean reaches the trace the lift's grayscale refresh leaves)
    let swept = take_outline_wear(a);
    let before = rect_union(overlay_rect(a), swept);
    let s = ed(a);
    drag_end_view(&mut s.sel);
    s.sel.state = SelState::Selected;
    if cancelled || (dx, dy) == (0, 0) {
        s.sel.drag = (0, 0);
        layout_bar(a);
        let r = rect_union(before, overlay_rect(a));
        redraw(a, r, RefreshMode::Ui);
        return;
    }
    sel_move(a, dx, dy);
    let r = rect_union(before, overlay_rect(a));
    redraw(a, r, RefreshMode::Ui);
}

/// The resize released: the selection's box `to` as the screen last showed it, scaled about the corner
/// opposite the handle dragged (one undo step, sel_scale). A cancel, or a box the pen never changed, puts
/// the box back as it was.
fn resize_done(a: &mut App, cancelled: bool) {
    let (from, to, handle, shows) = {
        let m = sel_ref(a);
        (m.resize_box0, m.box_, m.resize_handle, m.dv.shows)
    };
    qlog!("lasso: resize handle=%d from=%dx%d to=%dx%d shows=%u", handle, from.w, from.h, to.w, to.h, shows);
    // (the outline where the resize last drew it, and where it ends: one refresh over both, as a drag's lift;
    // its newest frame counted as DU wear, as a drag's)
    let swept = take_outline_wear(a);
    let before = rect_union(overlay_rect(a), swept);
    let s = ed(a);
    drag_end_view(&mut s.sel);
    s.sel.state = SelState::Selected;
    s.sel.resize_handle = 0;
    s.sel.box_ = from;
    if !(cancelled || to == from) {
        sel_scale(a, from, to, handle);
    }
    layout_bar(a);
    let r = rect_union(before, overlay_rect(a));
    redraw(a, r, RefreshMode::Ui);
}

fn sel_button(a: &mut App, b: SelBtn) {
    match b {
        SelBtn::Copy => {
            sel_copy(a, false);
        }
        SelBtn::Cut => sel_cut(a),
        SelBtn::Delete => {
            sel_delete(a, false);
        }
        SelBtn::Paste => sel_paste(a, None),
        SelBtn::Mode => sel_mode_toggle(a),
        SelBtn::Done => sel_off(a),
    }
}

// ---- the gestures ----

/// The links of page pid that are not carried: their seeds (a stroke with one leaves that link).
fn staying_link_seeds(s: &EditorState, pid: u32, carried: &[NavLink]) -> HashSet<u32> {
    s.nav.links.iter().filter(|l| l.pid == pid && !carried.iter().any(|c| c.seed == l.seed)).map(|l| l.seed).collect()
}

/// The tiles the strokes given drew into, and their boxes' union (page px).
fn tiles_of<'x>(strokes: impl IntoIterator<Item = &'x Stroke>, w: i32, h: i32, dpi: f32) -> Option<(Rc<TileMask>, Rect)> {
    let mut m = tile_mask_new(w, h)?;
    let mut r = RECT_EMPTY;
    for st in strokes {
        stroke_mark_tiles(&mut m, st, dpi);
        r = rect_union(r, st.bbox);
    }
    tile_mask_seal(&mut m);
    Some((Rc::new(m), r))
}

/// Saves the links after records were applied to the Nav; false (the Nav reloaded from the file) when it
/// could not.
fn save_links(s: &mut EditorState) -> bool {
    if nav_save_links(s.nb.id.as_str(), &mut s.nav, &s.nb) != 0 {
        nav_load(s.nb.id.as_str(), &mut s.nav);
        migration_reset(s);
        return false;
    }
    s.links_dirty = false;
    s.boxes.stale = true;
    true
}

/// The free layers' table for the open page: those its strokes and its history use.
fn busy_layers(s: &mut EditorState, ih: usize, ip: usize) -> [bool; FLOOR_LAYERS] {
    let mut busy = [false; FLOOR_LAYERS];
    layers_in_use(s.hist[ip].page.as_deref().unwrap(), &mut busy);
    history_layers_in_use(&s.hist[ih].h, &mut busy);
    busy
}

/// The open page's paper for placing (None when its picture is missing: every floor scoped then).
fn paper_for_place(s: &mut EditorState, dpi: f32) -> Option<(Rc<Vec<u8>>, i32)> {
    if s.picture_missing || !page_paper(s, dpi) {
        return None;
    }
    s.paper.px.clone().map(|px| (px, s.paper.stride))
}

/// Appends a placed group's strokes to page ip; how many the page took (page_add drops a stroke it has no
/// memory for, and the step must own exactly the strokes added, never earlier ink).
fn add_placed(s: &mut EditorState, ip: usize, strokes: Vec<Box<Stroke>>) -> usize {
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    let before = page.strokes.len();
    for st in strokes {
        page_add(page, st);
    }
    page.strokes.len() - before
}

/// Exact area, inside a gesture's step: the plan's crossing strokes spliced into page ip, each replaced at
/// its place by its pieces (`whole`: the ink taken whole, its indices now). None with no plan.
fn splice_plan(s: &mut EditorState, ip: usize, whole: &[usize]) -> Option<Spliced> {
    if s.sel.plan.is_empty() {
        return None;
    }
    let at = plan_indices(s)?;
    let mut cut: Vec<(usize, Vec<(Box<Stroke>, bool)>)> = at.into_iter().zip(std::mem::take(&mut s.sel.plan).into_iter().map(|(_, v)| v)).collect();
    cut.sort_by_key(|c| c.0);
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    Some(plan_splice(page, whole, cut))
}

/// splice_plan undone (a Copy, or a gesture refused): the originals back, the pieces in the plan again.
fn unsplice_plan(s: &mut EditorState, ip: usize, sp: Spliced) {
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    let back = plan_unsplice(page, sp);
    let page = s.hist[ip].page.as_deref().unwrap();
    s.sel.plan = back.into_iter().map(|(i, v)| (stroke_id(&page.strokes[i]), v)).collect();
}

/// The outside pieces on page ip now (after the ink at `idx`, spliced indices, was taken off): their tiles
/// are drawn again with the old place, so the page canvas is the page drawn afresh to the pixel.
fn outside_now<'x>(s: &'x EditorState, ip: usize, sp: Option<&Spliced>, idx: &[usize]) -> Vec<&'x Stroke> {
    let Some(sp) = sp else { return Vec::new() };
    let page = s.hist[ip].page.as_deref().unwrap();
    sp.outside.iter().map(|&o| o - idx.iter().filter(|&&i| i < o).count()).filter_map(|o| page.strokes.get(o).map(|b| &**b)).collect()
}

/// After a spliced gesture took the ink at `idx` (spliced indices) off: the step's removed strokes (the
/// crossing originals and the ink taken whole, at their indices before; inside pieces dropped, they were
/// never on the page outside the gesture) and the outside pieces' indices now.
fn spliced_history(sp: Spliced, idx: &[usize], removed: Vec<Box<Stroke>>) -> (Vec<i32>, Vec<Box<Stroke>>, Vec<usize>) {
    let outside: Vec<usize> = sp.outside.iter().map(|&o| o - idx.iter().filter(|&&i| i < o).count()).collect();
    let mut out: Vec<(usize, Box<Stroke>)> = sp.originals;
    for (k, st) in removed.into_iter().enumerate() {
        if let Some(&(old, _)) = sp.whole.iter().find(|w| w.1 == idx[k]) {
            out.push((old, st));
        }
    }
    out.sort_by_key(|r| r.0);
    let ix = out.iter().map(|r| r.0 as i32).collect();
    (ix, out.into_iter().map(|r| r.1).collect(), outside)
}

// TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a move or a paste in Select mode was
// refused, as the page holds the most moved groups it can.
const TOO_MANY_LAYERS: &str = n_("Too many moved groups on this page: paste elsewhere.");

/// The drag released: the selection moves by (dx, dy), one undo step (Move: the ink off its place, the
/// group at the end of the page with its erasing, carried link areas moved).
fn sel_move(a: &mut App, dx: i32, dy: i32) {
    sel_place(a, Spot { dx: dx as f32, dy: dy as f32, scale: 1.0 });
}

/// The resize released: the selection scaled from box `from` to `to` about the corner opposite `handle`,
/// one undo step (the ink off its place, the group scaled and put at the end of the page with its erasing,
/// carried link areas scaled).
fn sel_scale(a: &mut App, from: Rect, to: Rect, handle: i32) {
    if from.w <= 0 || from.h <= 0 || to.w <= 0 || to.h <= 0 {
        return;
    }
    let k = (to.w as f32 / from.w as f32).min(to.h as f32 / from.h as f32);
    if !k.is_finite() || k <= 0.02 {
        return;
    }
    let (ax, ay) = resize_anchor(from, handle);
    sel_place(a, Spot { dx: ax - ax * k, dy: ay - ay * k, scale: k });
}

/// Page rect b put by spot (as the group's points are: x * scale + dx, y * scale + dy).
fn spot_box(b: Rect, spot: Spot) -> Rect {
    let mx = |v: i32| (v as f32 * spot.scale + spot.dx).round() as i32;
    let my = |v: i32| (v as f32 * spot.scale + spot.dy).round() as i32;
    let (x0, y0, x1, y1) = (mx(b.x), my(b.y), mx(b.x + b.w), my(b.y + b.h));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// The selection put by spot: moved (a drag released) or scaled (a resize released), one undo step: the ink
/// off its place, the group at the end of the page with its erasing, carried link areas moved or scaled.
fn sel_place(a: &mut App, spot: Spot) {
    let dpi = a.dpi;
    if refuse_readonly(a) {
        ed(a).sel.drag = (0, 0);
        return;
    }
    let s = ed(a);
    let (Some(ih), Some(ip), Some(idx)) = (s.cur_hist(), s.page_ix(), sel_indices(s)) else {
        sel_deselect(a);
        return;
    };
    let pid = s.open_pid();
    let t0 = now_ms();
    let carried = s.sel.links.clone();
    // the links first: if they cannot be saved, nothing changes
    let mut records: Vec<(Option<LinkRec>, Option<LinkRec>)> = Vec::new();
    let mut moved_links: Vec<NavLink> = Vec::new();
    for l in &carried {
        let Some(m) = link_placed(l, spot, pid, l.seed, l.target.as_str(), s.pgw, s.pgh) else { continue };
        records.push((Some(link_rec(l)), Some(link_rec(&m))));
        moved_links.push(m);
    }
    if !records.is_empty() {
        let ok = s.nav.links_writable && moved_links.iter().all(|m| nav_link_put(&mut s.nav, m) == 0);
        if !ok || !save_links(s) {
            if ok {
                // (save_links reloaded)
            } else {
                nav_load(s.nb.id.as_str(), &mut s.nav);
                migration_reset(s);
            }
            s.sel.drag = (0, 0);
            // TRANSLATORS: a toast: a move in Select mode could not save the links it carries.
            app_toast(a, tr("Couldn't save links: nothing moved."), 3000);
            return;
        }
    }
    let s = ed(a);
    step_begin(s, true);
    let sp = splice_plan(s, ip, &idx);
    let idx: Vec<usize> = sp.as_ref().map_or(idx, |sp| sp.sel.clone());
    let page = s.hist[ip].page.as_deref().unwrap();
    let g = lasso_lift(page, &idx, carried.clone(), dpi);
    let src_seeds = staying_link_seeds(s, pid, &carried);
    // take the selection off
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    let mut removed: Vec<Box<Stroke>> = Vec::with_capacity(idx.len());
    for &i in idx.iter().rev() {
        removed.push(page_remove(page, i as i32));
    }
    removed.reverse();
    // the old place drawn again in the tiles they drew into
    let (w, h) = (s.pgw, s.pgh);
    let mut dirty = RECT_EMPTY;
    let tm = tiles_of(removed.iter().map(|b| &**b).chain(outside_now(s, ip, sp.as_ref(), &idx)), w, h, dpi);
    if let Some((m, r)) = tm {
        let page = s.hist[ip].page.as_deref().unwrap();
        let r = rect_intersect(r, tile_mask_bounds(&m));
        page_render_tiles(&mut s.page_canvas, page, r, &m, dpi);
        dirty = r;
    }
    let src_ms = now_ms().wrapping_sub(t0);
    let t1 = now_ms();
    let paper = paper_for_place(s, dpi);
    let busy = busy_layers(s, ih, ip);
    let page = s.hist[ip].page.as_deref().unwrap();
    let dst_links: Vec<NavLink> = s.nav.links.iter().filter(|l| l.pid == pid).copied().collect();
    let (inkm, _) = lasso_seeds(&g, page, &dst_links, &[], &src_seeds, true);
    let placed = match lasso_place(&mut s.page_canvas, &g, spot, &inkm, paper.as_ref().map(|(b, st)| (b, *st)), &busy, dpi) {
        Ok(placed) => placed,
        Err(e) => {
            // no layer: everything back as it was (the old place, and the group's ink drawn before the refusal)
            let page = s.hist[ip].page.as_deref_mut().unwrap();
            for (k, st) in removed.into_iter().enumerate() {
                page_insert(page, idx[k] as i32, st);
            }
            if let Some(sp) = sp {
                unsplice_plan(s, ip, sp);
            }
            if !records.is_empty() {
                for (b, _) in &records {
                    if let Some(b) = b {
                        nav_link_put(&mut s.nav, &rec_link(pid, b));
                    }
                }
                save_links(s);
            }
            s.sel.drag = (0, 0);
            draw_page(a, rect_union(dirty, e.drawn));
            step_end(ed(a), RECT_EMPTY);
            app_toast(a, tr(TOO_MANY_LAYERS), 3000);
            return;
        }
    };
    let dst_ms = now_ms().wrapping_sub(t1);
    dirty = rect_union(dirty, placed.drawn);
    let want = placed.strokes.len();
    let n_added = add_placed(s, ip, placed.strokes);
    if n_added < want {
        draw_page(a, placed.drawn); // (a stroke the page could not take is not left on screen)
    }
    let s = ed(a);
    let link = (!records.is_empty()).then(|| LinkEdit { pid, records });
    let (hh, page) = hist_page(&mut s.hist, ih, ip);
    match sp {
        Some(sp) => {
            let (ix, gone, mut added_at) = spliced_history(sp, &idx, removed);
            let n = page.strokes.len();
            added_at.extend(n - n_added..n);
            history_push_lasso_at(hh, page, &ix, gone, &added_at, link);
        }
        None => {
            let idx32: Vec<i32> = idx.iter().map(|&i| i as i32).collect();
            history_push_lasso(hh, page, &idx32, removed, n_added, link);
        }
    }
    s.sel.kept.clear();
    if g.carried > 0 {
        s.hist[ip].to_list = true; // (the original erase strokes stay: compaction may drop those that now do nothing)
    }
    step_end(s, dirty);
    // the selection follows its ink
    let page = s.hist[ip].page.as_deref().unwrap();
    let n = page.strokes.len();
    s.sel.ids = page.strokes[n - n_added..].iter().filter(|st| stroke_is_ink(st)).map(|st| stroke_id(st)).collect();
    s.sel.links = moved_links;
    let b = spot_box(s.sel.box_, spot);
    s.sel.box_ = Rect { x: b.x.clamp(0, (s.pgw - b.w).max(0)), y: b.y.clamp(0, (s.pgh - b.h).max(0)), w: b.w, h: b.h };
    s.sel.drag = (0, 0);
    s.boxes.stale = true;
    qlog!("lasso: place n=%d carried=%d scoped=%d scale=%d src_ms=%u dst_ms=%u", g.ink as i32, g.carried as i32, placed.scoped as i32, (spot.scale * 1000.0) as i32, src_ms, dst_ms);
    s.last_lasso_ms = (src_ms, dst_ms);
    layout_bar(a);
    show(a, dirty, RefreshMode::Ui);
    mark_changed(ed(a));
    repaint_panel(a, RefreshMode::Ui);
}

/// Cut: Copy, then Delete. Refused whole on a read-only page or when the links it carries cannot be saved
/// (nothing copied either), and a Delete that fails after the copy puts the clipboard back as it was: a
/// Cut that cut nothing changed nothing.
fn sel_cut(a: &mut App) {
    if refuse_readonly(a) {
        return;
    }
    let s = ed(a);
    if !s.sel.links.is_empty() && !s.nav.links_writable {
        app_toast(a, tr(NOTHING_CUT), 3000);
        return;
    }
    let old = clipboard_snapshot();
    if !sel_copy(a, true) {
        return;
    }
    if !sel_delete(a, true) {
        if clipboard_restore(old.as_deref()) != 0 {
            qlog!("lasso: cut refused and the clipboard could not be put back");
        }
        layout_bar(a); // (Paste goes again when there was no clipboard before)
        let r = overlay_rect(a);
        redraw(a, r, RefreshMode::Ui);
        app_toast(a, tr(NOTHING_CUT), 3000);
    }
}

// TRANSLATORS: a toast: Cut in Select mode could not save the links the selection carries (links.ini), so
// nothing was cut and the clipboard is as it was.
const NOTHING_CUT: &str = n_("Couldn't save links: nothing cut.");

/// Copy: the selection (its visible ink, the erasing that goes with it, its links) to the clipboard. Not
/// an undo step; works on a read-only page too. False when nothing was copied. `cut`: for Cut, which takes
/// the ink off the page next. Neither says so with a toast (the owner, 2026-11-04): Paste appears.
fn sel_copy(a: &mut App, cut: bool) -> bool {
    let dpi = a.dpi;
    let s = ed(a);
    let Some(idx) = sel_indices(s) else {
        sel_deselect(a);
        return false;
    };
    let t0 = now_ms();
    // exact area: the pieces spliced in for the lift and out again at once (the page must not change)
    let ip = s.page_ix();
    let sp = ip.and_then(|ip| splice_plan(s, ip, &idx));
    let idx: Vec<usize> = sp.as_ref().map_or(idx, |sp| sp.sel.clone());
    let page = s.open_page().unwrap();
    // erased content does not travel: ink entirely erased is left out
    let visible: Vec<usize> = idx.iter().copied().filter(|&i| stroke_visible(page, i, dpi)).collect();
    let mut g = lasso_lift(page, &visible, s.sel.links.clone(), dpi);
    if let (Some(sp), Some(ip)) = (sp, ip) {
        #[cfg(debug_assertions)]
        let n_before = s.hist[ip].page.as_deref().map_or(0, |p| p.strokes.len()) - sp.outside.len() - sp.sel.len() + sp.originals.len() + sp.whole.len();
        unsplice_plan(s, ip, sp);
        #[cfg(debug_assertions)]
        debug_assert_eq!(s.hist[ip].page.as_deref().map_or(0, |p| p.strokes.len()), n_before, "Copy left the page as it was");
    }
    g.src_pid = s.open_pid();
    g.src_nb = s.nb.id.as_str().to_string();
    if clipboard_write(&g) != 0 {
        // TRANSLATORS: a toast: Copy or Cut in Select mode could not write the clipboard file.
        app_toast(a, tr("Couldn't write clipboard."), 3000);
        return false;
    }
    qlog!("lasso: copy n=%d carried=%d links=%d ms=%u", g.ink as i32, g.carried as i32, g.links.len() as i32, now_ms().wrapping_sub(t0));
    if cut {
        return true;
    }
    layout_bar(a); // (Paste appears)
    let r = overlay_rect(a);
    redraw(a, r, RefreshMode::Ui);
    app_toast_dismiss(a); // (a toast left up from before goes)
    true
}

/// Delete (and Cut, after its copy): the selected ink off the page with its carried links, one step. The
/// erase strokes stay (over the emptied place they change nothing). False when nothing was taken off.
fn sel_delete(a: &mut App, cut: bool) -> bool {
    let dpi = a.dpi;
    if refuse_readonly(a) {
        return false;
    }
    let s = ed(a);
    let (Some(ih), Some(ip), Some(idx)) = (s.cur_hist(), s.page_ix(), sel_indices(s)) else {
        sel_deselect(a);
        return false;
    };
    let pid = s.open_pid();
    let carried = s.sel.links.clone();
    let records: Vec<(Option<LinkRec>, Option<LinkRec>)> = carried.iter().map(|l| (Some(link_rec(l)), None)).collect();
    if !records.is_empty() {
        if !s.nav.links_writable {
            if !cut {
                app_toast(a, tr("Couldn't save links: nothing moved."), 3000);
            }
            return false;
        }
        for l in &carried {
            nav_link_remove(&mut s.nav, pid, l.seed);
        }
        if !save_links(s) {
            if !cut {
                app_toast(a, tr("Couldn't save links: nothing moved."), 3000);
            }
            return false;
        }
    }
    let s = ed(a);
    step_begin(s, true);
    let sp = splice_plan(s, ip, &idx);
    let idx: Vec<usize> = sp.as_ref().map_or(idx, |sp| sp.sel.clone());
    let page = s.hist[ip].page.as_deref_mut().unwrap();
    let mut removed: Vec<Box<Stroke>> = Vec::with_capacity(idx.len());
    for &i in idx.iter().rev() {
        removed.push(page_remove(page, i as i32));
    }
    removed.reverse();
    let (w, h) = (s.pgw, s.pgh);
    let mut dirty = RECT_EMPTY;
    let tm = tiles_of(removed.iter().map(|b| &**b).chain(outside_now(s, ip, sp.as_ref(), &idx)), w, h, dpi);
    if let Some((m, r)) = tm {
        let page = s.hist[ip].page.as_deref().unwrap();
        let r = rect_intersect(r, tile_mask_bounds(&m));
        page_render_tiles(&mut s.page_canvas, page, r, &m, dpi);
        dirty = r;
    }
    let n = removed.len();
    let link = (!records.is_empty()).then(|| LinkEdit { pid, records });
    let (hh, page) = hist_page(&mut s.hist, ih, ip);
    match sp {
        Some(sp) => {
            let (ix, gone, added_at) = spliced_history(sp, &idx, removed);
            history_push_lasso_at(hh, page, &ix, gone, &added_at, link);
        }
        None => {
            let idx32: Vec<i32> = idx.iter().map(|&i| i as i32).collect();
            history_push_lasso(hh, page, &idx32, removed, 0, link);
        }
    }
    step_end(s, dirty);
    let before = overlay_rect(a);
    let s = ed(a);
    s.sel.state = SelState::Armed;
    s.sel.ids.clear();
    s.sel.plan.clear();
    s.sel.kept.clear();
    s.sel.links.clear();
    s.sel.box_ = RECT_EMPTY;
    s.boxes.stale = true;
    qlog!("lasso: %s n=%d", if cut { "cut" } else { "delete" }, n as i32);
    layout_bar(a);
    let r = page_to_screen(a, dirty);
    redraw(a, rect_union(rect_union(before, overlay_rect(a)), r), RefreshMode::Ui);
    mark_changed(ed(a));
    repaint_panel(a, RefreshMode::Ui);
    if cut {
        app_toast_dismiss(a); // (no "Cut.": the owner, 2026-11-04; a toast left up from before goes)
    }
    sel_settle_hold(a);
    true
}

/// editor_tick: a pen held still on the page in Select mode (its lasso not begun) for HOLD_MS pastes
/// the clipboard centred there (design 6.1 and 6.7); the rest of the contact does nothing. Nothing without a
/// clipboard: the lasso goes on.
pub(super) fn sel_tick(a: &mut App, now: u32) {
    drag_tick(a, now);
    preview_tick(a, now);
    let m = sel_ref(a);
    if m.state != SelState::Lasso || m.finger || !m.still || now.wrapping_sub(m.down_at) < HOLD_MS || ed_ref(a).owner != Owner::Select {
        return;
    }
    if !clipboard_has() {
        ed(a).sel.still = false; // (asked once)
        return;
    }
    let s = ed(a);
    let at = s.sel.pts.first().map(|&(x, y)| (x.round() as i32, y.round() as i32));
    let r = rect_union(s.sel.lasso_rect, preview_end(s));
    s.sel.state = SelState::Armed;
    s.sel.pts.clear();
    s.sel.lasso_rect = RECT_EMPTY;
    s.sel.still = false;
    redraw(a, r, RefreshMode::Ui);
    let Some(at) = at else { return };
    sel_paste(a, Some(at));
}

/// editor_tick: a finger held still for HOLD_MS at screen point (x, y), not on a link: the finger lasso
/// arms there (Select mode entered for it, quietly, when it was off), and a small dashed ring at the
/// finger says so. False (nothing done) where it cannot: making a link, a sheet, the page peek or the
/// card in front, a selection being dragged, off the page, on the panel or Select's bar.
pub(super) fn sel_finger_arm(a: &mut App, x: f32, y: f32) -> bool {
    let clip = lasso_clip(a);
    let s = ed_ref(a);
    let (xi, yi) = (x as i32, y as i32);
    if !s.open || s.lm.state != LmState::Off || sheet_open(s) || s.peek.active || s.card.open
        || !matches!(s.sel.state, SelState::Off | SelState::Armed)
        || !rect_contains(clip, xi, yi) || rect_contains(s.panel, xi, yi) || rect_contains(s.sel.bar, xi, yi)
    {
        return false;
    }
    let s = ed(a);
    let (mut px, mut py) = (0.0, 0.0);
    to_page(s, x, y, &mut px, &mut py);
    if s.sel.state == SelState::Off {
        s.sel = SelMode { by_finger: true, quiet: true, ..Default::default() };
    }
    let dpi = a.dpi;
    let s = ed(a);
    lasso_begin_at(s, px, py, true, now_ms(), dpi);
    s.owner = Owner::Select;
    s.link_held = true;
    // (a still finger drew no trail: its capture, killed by the hold, starts clean next time)
    if rect_is_empty(s.fs_trail) {
        fs_reset_capture(s);
    }
    finger_cue(a, x, y);
    qlog!("select: finger hold armed the lasso");
    true
}

/// The cue that the finger's hold armed the lasso: a small dashed ring around it, one fast refresh. It
/// joins the lasso's drawn rect, so whatever ends the lasso draws it away with the lasso.
fn finger_cue(a: &mut App, x: f32, y: f32) {
    let rad = ui_px(40.0) as f32;
    let (thick, halo) = (ui_px(3.0).max(1), ui_px(2.0).max(1));
    let pts: Vec<(f32, f32)> = (0..=24).map(|k| {
        let t = k as f32 * std::f32::consts::TAU / 24.0;
        (x + rad * t.cos(), y + rad * t.sin())
    }).collect();
    let clip = lasso_clip(a);
    let keep = a.screen.clip;
    canvas_set_clip(&mut a.screen, clip);
    canvas_dash_poly(&mut a.screen, &pts, thick, ui_px(10.0), ui_px(8.0), halo, 0);
    a.screen.clip = keep;
    let m = (rad + (thick + 2 * halo) as f32).ceil() as i32 + 1;
    let r = rect_intersect(Rect { x: x as i32 - m, y: y as i32 - m, w: 2 * m + 1, h: 2 * m + 1 }, clip);
    let s = ed(a);
    s.sel.lasso_rect = rect_union(s.sel.lasso_rect, r);
    app_invalidate(a, r, RefreshMode::Fast);
}

/// A finger's lasso lifted where it was held: the ring goes, and the clipboard, if there is one, is
/// pasted centred there (refused on a read-only page, as the pen's is); a Select mode the hold entered
/// ends unless the paste left a selection.
fn finger_lift(a: &mut App) {
    let s = ed(a);
    let at = s.sel.pts.first().map(|&(x, y)| (x.round() as i32, y.round() as i32));
    let r = rect_union(s.sel.lasso_rect, preview_end(s));
    s.sel.state = SelState::Armed;
    s.sel.pts.clear();
    s.sel.lasso_rect = RECT_EMPTY;
    s.sel.finger = false;
    redraw(a, r, RefreshMode::Ui);
    if let (true, Some(at)) = (clipboard_has(), at) {
        sel_paste(a, Some(at));
    }
    sel_settle_hold(a);
}

/// Paste: the clipboard's group at the end of the open page (where it was if that spot is in view, else
/// in the middle of the view; centred on `at`, a long press), selected, one step with its links.
pub(super) fn sel_paste(a: &mut App, at: Option<(i32, i32)>) {
    let dpi = a.dpi;
    if refuse_readonly(a) {
        return;
    }
    let Some(g) = clipboard_read() else {
        // TRANSLATORS: a toast: Paste found no clipboard it could read.
        app_toast(a, tr("Couldn't read clipboard."), 3000);
        return;
    };
    let view = view_page(a);
    let before = overlay_rect(a);
    let s = ed(a);
    let (Some(ih), Some(ip)) = (s.cur_hist(), s.page_ix()) else { return };
    let pid = s.open_pid();
    let here = s.nb.id.as_str();
    // offset 3 mm only on its own page while the originals are there (design 6.7): after a Cut it lands
    // where it was
    let same_page = g.src_nb == here && g.src_pid == pid && s.hist[ip].page.as_deref().is_some_and(|p| group_originals_on(p, &g));
    let spot = paste_spot(g.box_, view, s.pgw, s.pgh, same_page, at, dpi);
    // the links: targets as they read from here, fresh seeds, limits checked before anything changes
    let page = s.hist[ip].page.as_deref().unwrap();
    let dst_links: Vec<NavLink> = s.nav.links.iter().filter(|l| l.pid == pid).copied().collect();
    let mut gl = g.clone();
    let mut index_left = false;
    if g.src_nb != here {
        gl.links.retain(|l| {
            let keep = nav_target_into(l.target.as_str(), &g.src_nb, &s.nb).is_some();
            index_left |= !keep;
            keep
        });
    }
    let (inkm, linkm) = lasso_seeds(&gl, page, &dst_links, &[], &HashSet::new(), false);
    let mut new_links: Vec<NavLink> = Vec::new();
    for l in &gl.links {
        let target = if g.src_nb != here { nav_target_into(l.target.as_str(), &g.src_nb, &s.nb).unwrap_or_default() } else { l.target.as_str().to_string() };
        if let Some(m) = link_placed(l, spot, pid, linkm[&l.seed], &target, s.pgw, s.pgh) {
            new_links.push(m);
        }
    }
    if !new_links.is_empty() {
        let on_page = nav_links_of(&s.nav, pid, &mut 0) as usize;
        if !s.nav.links_writable || on_page + new_links.len() > NAV_LINKS_PER_PAGE as usize || s.nav.links.len() + new_links.len() > NAV_LINKS_MAX as usize {
            // TRANSLATORS: a toast: Paste was refused whole because its links do not fit (64 on a page,
            // 4000 in a notebook) or links.ini cannot be written.
            app_toast(a, tr("Couldn't paste: its links don't fit here."), 3000);
            return;
        }
        let ok = new_links.iter().all(|m| nav_link_put(&mut s.nav, m) == 0);
        if !ok || !save_links(s) {
            if !ok {
                nav_load(s.nb.id.as_str(), &mut s.nav);
                migration_reset(s);
            }
            app_toast(a, tr("Couldn't paste: its links don't fit here."), 3000);
            return;
        }
    }
    let s = ed(a);
    step_begin(s, true);
    let t0 = now_ms();
    let paper = paper_for_place(s, dpi);
    let busy = busy_layers(s, ih, ip);
    let placed = match lasso_place(&mut s.page_canvas, &gl, spot, &inkm, paper.as_ref().map(|(b, st)| (b, *st)), &busy, dpi) {
        Ok(placed) => placed,
        Err(e) => {
            for m in &new_links {
                nav_link_remove(&mut s.nav, pid, m.seed);
            }
            if !new_links.is_empty() {
                save_links(s);
            }
            // (the group's ink drawn before the refusal is not on the page: drawn again from it)
            draw_page(a, e.drawn);
            step_end(ed(a), RECT_EMPTY);
            app_toast(a, tr(TOO_MANY_LAYERS), 3000);
            return;
        }
    };
    let ms = now_ms().wrapping_sub(t0);
    let dirty = placed.drawn;
    let want = placed.strokes.len();
    let n_added = add_placed(s, ip, placed.strokes);
    if n_added < want {
        draw_page(a, dirty); // (a stroke the page could not take is not left on screen)
    }
    let s = ed(a);
    let records: Vec<(Option<LinkRec>, Option<LinkRec>)> = new_links.iter().map(|m| (None, Some(link_rec(m)))).collect();
    let link = (!records.is_empty()).then(|| LinkEdit { pid, records });
    let (hh, page) = hist_page(&mut s.hist, ih, ip);
    history_push_lasso(hh, page, &[], Vec::new(), n_added, link);
    step_end(s, dirty);
    // the paste lands selected, ready to drag
    let page = s.hist[ip].page.as_deref().unwrap();
    let n = page.strokes.len();
    let b = g.box_;
    let x0 = (b.x as f32 * spot.scale + spot.dx).floor() as i32;
    let y0 = (b.y as f32 * spot.scale + spot.dy).floor() as i32;
    let x1 = ((b.x + b.w) as f32 * spot.scale + spot.dx).ceil() as i32;
    let y1 = ((b.y + b.h) as f32 * spot.scale + spot.dy).ceil() as i32;
    s.sel.state = SelState::Selected;
    s.sel.pid = pid;
    s.sel.ids = page.strokes[n - n_added..].iter().filter(|st| stroke_is_ink(st)).map(|st| stroke_id(st)).collect();
    s.sel.plan.clear(); // (a pasted group is whole strokes: no cut plan, no lasso to judge again)
    s.sel.kept.clear();
    s.sel.links = new_links;
    s.sel.box_ = rect_intersect(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }, Rect { x: 0, y: 0, w: s.pgw, h: s.pgh });
    s.sel.drag = (0, 0);
    s.boxes.stale = true;
    if g.carried > 0 {
        s.hist[ip].to_list = true;
    }
    qlog!("lasso: paste n=%d carried=%d scoped=%d src_ms=0 dst_ms=%u", g.ink as i32, g.carried as i32, placed.scoped as i32, ms);
    s.last_lasso_ms = (0, ms);
    if s.sel.ids.is_empty() {
        s.sel.state = SelState::Armed;
    }
    s.sel.quiet = false; // (a Select mode a finger's hold entered shows now: its bar, and the panel's lasso)
    layout_bar(a);
    let r = rect_union(before, overlay_rect(a));
    show(a, dirty, RefreshMode::Ui);
    redraw(a, r, RefreshMode::Ui);
    mark_changed(ed(a));
    repaint_panel(a, RefreshMode::Ui);
    if index_left {
        // TRANSLATORS: a toast: a pasted link to the Index was left out, as it cannot point to another
        // notebook's Index.
        app_toast(a, tr("Index links stay in their notebook."), 3000);
    }
}

// ---- the scripts' probes ----

/// editor.select, editor.selected, editor.sel_links, editor.sel_box, editor.sel_bar.<button>,
/// editor.clipboard, editor.layered, editor.lasso_ms, editor.drag_*: None for another key.
pub(super) fn sel_probe(a: &App, key: &str) -> Option<String> {
    let s = ed_ref(a);
    Some(match key {
        "select" => match s.sel.state {
            SelState::Off => "off",
            SelState::Armed => "armed",
            SelState::Lasso => "lasso",
            SelState::Selected => "selected",
            SelState::Dragging => "dragging",
        }
        .to_string(),
        "selected" => sel_indices(s).map_or(0, |v| v.len() + s.sel.plan.iter().map(|c| c.1.iter().filter(|q| q.1).count()).sum::<usize>()).to_string(),
        // exact area: the selection's (or the next lasso's) mode, and how many strokes its plan cuts
        "sel_exact" => (if matches!(s.sel.state, SelState::Off | SelState::Armed) { exact_mode() } else { s.sel.exact } as i32).to_string(),
        "sel_cut" => s.sel.plan.len().to_string(),
        // the open page's strokes, hashed (tool, seed, ext, every point): "page_sig_keep" keeps it (and says
        // "kept"), "page_sig_same" says 1 while it is the one kept (an exact-area Copy leaves the page so)
        "page_sig_keep" => {
            PAGE_SIG.with(|c| c.set(page_sig(s)));
            "kept".to_string()
        }
        "page_sig_same" => ((PAGE_SIG.with(|c| c.get()) == page_sig(s)) as i32).to_string(),
        "sel_links" => s.sel.links.len().to_string(),
        "sel_box" => {
            let b = s.sel.box_;
            format!("{} {} {} {}", b.x, b.y, b.w, b.h)
        }
        "clipboard" => (clipboard_has() as i32).to_string(),
        "layered" => (s.open_page().is_some_and(page_layered) as i32).to_string(),
        "lasso_ms" => format!("{} {}", s.last_lasso_ms.0, s.last_lasso_ms.1),
        // the live preview: the strokes with a halo now (the preview's while a lasso is drawn, the selection's
        // after), the last lasso's batches and fast refreshes, whether its lift reused the preview's answer,
        // and whether the lift's redraw covered every preview refresh
        "halo" => match s.sel.state {
            SelState::Lasso => s.sel.live.as_deref().map_or(0, |l| l.halo_list().len()),
            SelState::Selected | SelState::Dragging => sel_indices(s).map_or(0, |v| v.len()) + s.sel.plan.iter().map(|c| c.1.iter().filter(|q| q.1).count()).sum::<usize>(),
            _ => 0,
        }
        .to_string(),
        "preview_batches" => s.sel.preview_batches.to_string(),
        "preview_refreshes" => s.sel.preview_refreshes.to_string(),
        "preview_reused" => (s.sel.preview_reused as i32).to_string(),
        "preview_rect" => {
            let b = s.sel.preview_rect;
            format!("{} {} {} {}", b.x, b.y, b.w, b.h)
        }
        "lift_covers_preview" => ((rect_is_empty(s.sel.preview_seen) || rect_contains_rect(s.sel.lift_rect, s.sel.preview_seen)) as i32).to_string(),
        "lift_rect" => {
            let b = s.sel.lift_rect;
            format!("{} {} {} {}", b.x, b.y, b.w, b.h)
        }
        // a drag (2.0.0): the pen samples that changed its place, the shows of it (the first move's
        // included), how many times the selection's ink was walked while it lasted (its halo drawn), all
        // for the drag now or the last one; where the screen shows its outline ("dx dy" page px from the
        // box's place, "-" while no drag moves); and whether it holds its copy of the screen
        "drag_samples" => s.sel.dv.samples.to_string(),
        "drag_shows" => s.sel.dv.shows.to_string(),
        "drag_inked" => s.sel.dv.inked.to_string(),
        "drag_shown" => match (s.sel.state, s.sel.dv.moving, s.sel.dv.shown) {
            (SelState::Dragging, true, Some((x, y))) => format!("{} {}", x, y),
            _ => "-".to_string(),
        },
        "drag_copy" => (s.sel.dv.base.is_some() as i32).to_string(),
        // the open page's links: target@x,y,w,h each (area or "-"), in seed order
        "page_links" => {
            let pid = s.open_pid();
            let v: Vec<String> = s.nav.links.iter().filter(|l| l.pid == pid).map(|l| {
                if l.has_area { format!("{}@{},{},{},{}", l.target.as_str(), l.area.x, l.area.y, l.area.w, l.area.h) } else { format!("{}@-", l.target.as_str()) }
            }).collect();
            v.join(" ")
        }
        _ => return None,
    })
}

thread_local! {
    static PAGE_SIG: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn page_sig(s: &EditorState) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |v: u32| {
        h ^= v as u64;
        h = h.wrapping_mul(0x100000001b3);
    };
    if let Some(p) = s.open_page() {
        for st in &p.strokes {
            eat(st.tool.0 as u32);
            eat(st.seed);
            eat(u32::from_le_bytes([st.ext[0], st.ext[1], st.ext[2], 0]));
            for q in &st.pts {
                eat(q.x.to_bits());
                eat(q.y.to_bits());
                eat(q.pressure.to_bits());
            }
        }
    }
    h
}

/// The centre of a bar button for `tap panel select.<copy|cut|delete|paste|done>`.
pub(super) fn sel_control_point(s: &EditorState, name: &str) -> Option<(f32, f32)> {
    let want = match name {
        "copy" => SelBtn::Copy,
        "cut" => SelBtn::Cut,
        "delete" => SelBtn::Delete,
        "paste" => SelBtn::Paste,
        "mode" => SelBtn::Mode,
        "done" => SelBtn::Done,
        _ => return None,
    };
    s.sel.btns.iter().find(|(_, b)| *b == want).map(|(r, _)| ((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32))
}
