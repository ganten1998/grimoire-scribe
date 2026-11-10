//! The Index map. This view owns its raster and interaction; the editor owns navigation and names.
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MapResult {
    #[default]
    None,
    Open(u32),
    Target(FixedStr<NAV_TARGET_LEN>),
    /// a broken link (its page or notebook deleted): page id and seed, to open its card on its page
    Repair(u32, u32),
    Close,
    Rename(u32),
    Pages,
    List,
}
/// What the Graph shows (2.0.0-beta.2, docs/beta2/GRAPH-ALL.md): every notebook, what it always opens on
/// (mapview_open_scoped), or one notebook (MapView.id) only past the Graph of all notebooks' caps: the
/// editor's, or with no notebook open the one picked in the browser the Library opens then (the owner's
/// ruling of 2026-11-03 removed the Notebook choice). A blank view is one notebook's, as every view was before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapScope {
    All,
    #[default]
    One,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MapLayout {
    Seed,
    Running,
    #[default]
    Done,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapHitKind {
    Control(u8),
    Node(usize),
    Link(usize),
    /// an anchor: another notebook's page at the plot's edge (index into foreign)
    Anchor(usize),
}
#[derive(Clone, Copy, Debug)]
struct MapHit {
    rect: Rect,
    kind: MapHitKind,
    centre: (f32, f32),
    radius: f32,
}
/// An anchor (2.0.0-beta.2): a page of another notebook that this notebook links to, or that links here (the
/// backlinks cache), boxed at the plot's right edge, grouped by notebook, with a line to each page here it is
/// linked with. Anchors are never nodes of the layout: they move no dot, and the map keeps its shape.
#[derive(Clone, Debug, Default)]
struct ForeignPlace {
    /// the other page, n:<notebook>:p<page>
    target: FixedStr<NAV_TARGET_LEN>,
    /// its notebook's id (the group) and name
    key: FixedStr<STORE_ID_LEN>,
    notebook: String,
    label: String,
    /// its page's number there (0 when gone): the order in its group
    number: usize,
    /// pages here link to it / it links to pages here
    out: bool,
    inc: bool,
    /// a link to it finds its page or notebook deleted
    broken: bool,
    /// the pages here it is linked with, either way, and for each whether its links go out to it (else they
    /// only come in from it): each line's own style
    here: Vec<u32>,
    here_out: Vec<bool>,
    /// the first link here to it (page id, seed): what a tap on a broken anchor repairs
    first: (u32, u32),
}
struct MapDrag {
    x: f32,
    y: f32,
    ox: f32,
    oy: f32,
    zoom: f32,
    distance: f32,
    moved: bool,
    last_present: u32,
    saved: Canvas,
}
#[derive(Default)]
pub struct MapView {
    pub open: bool,
    pub needs_redraw: bool,
    /// the last open failed past the Graph of all notebooks' caps with no notebook open (the Library's Graph):
    /// the Library opens the notebooks' browser instead, to show one
    pub too_big: bool,
    pub list: bool,
    list_offset: usize,
    /// the List tab's: the editor's notebook's links, its pages and its name, whatever the Graph shows
    list_rows: Vec<NavLink>,
    list_pages: Vec<u32>,
    list_name: String,
    /// the notebook shown (one notebook's view): its links (what its anchors come from) and name
    link_rows: Vec<NavLink>,
    notebook_name: String,
    /// every notebook, or one (2.0.0-beta.2)
    pub scope: MapScope,
    /// the editor's notebook, and its page the Graph was opened from
    editor_id: FixedStr<STORE_ID_LEN>,
    open_pid: u32,
    /// one notebook's view of another notebook than the editor's: its notebook.ini and its Nav (names,
    /// and links as the Graph reads them), read from its files when it was picked
    other: Option<Box<(NotebookInfo, Nav)>>,
    /// All: graph-all.ini as last read or written
    all_ini: GraphAllIni,
    /// All: each node's page name ("" for none), and each notebook's name (by GraphNode.book)
    node_names: Vec<String>,
    book_names: Vec<String>,
    /// All: the library index's generation the model was built from, and when it was last built again
    index_gen: u32,
    index_built_ms: u32,
    /// All: notebooks the index left out, broken links in all, the islands' captions drawn
    left_out: usize,
    broken: usize,
    captions: usize,
    foreign: Vec<ForeignPlace>,
    /// the backlinks generation the anchors were made at
    anchors_gen: u32,
    link_count: usize,
    page_totals: Option<(usize,usize)>,
    canvas: Canvas,
    g: GraphModel,
    ini: GraphIni,
    id: FixedStr<STORE_ID_LEN>,
    pages: Vec<u32>,
    zoom: f32,
    ox: f32,
    oy: f32,
    mode: GraphMode,
    around: Vec<GraphRing>,
    centre: i32,
    /// the primary selected node (its card shows), -1 for none
    selected: i32,
    /// a selection of several (Select all), per node; empty when the
    /// selection is `selected` alone. When it is not empty it holds `selected` too (unless that is -1). Never
    /// saved, and no undo step.
    picked: Vec<bool>,
    /// when the selected node was selected (event ms): a pen's tap on it again sooner than PEN_REOPEN_MS is
    /// the stylus bouncing, not a second tap
    selected_k: u32,
    layout: MapLayout,
    hits: Vec<MapHit>,
    labels: Vec<Rect>,
    drag: Option<MapDrag>,
    last_input_ms: u32,
    job_done: bool,
    // the user panned or pinched since the view was last fitted: View fits again first
    view_touched: bool,
    w: i32,
    h: i32,
}
const BACK: u8 = 1;
const LIST: u8 = 2;
const OPEN: u8 = 10;
/// the selected node's card: Adjacent (View: Adjacent around it; "Around it" before 2.0.0-beta.2)
const ADJACENT: u8 = 11;
const RENAME: u8 = 12;
const VIEW: u8 = 16;
const GRAPH: u8 = 14;
const PAGES: u8 = 15;
/// Enter in the Graph (a fixed key, no action of the files'): opens the selected page when one is selected. An
/// id outside the registry's, so never bound.
pub const GRAPH_KEY_ENTER: i32 = 340;
// Keys zoom by this much a press, about the plot's centre.
const KEY_ZOOM: f32 = 1.25;
// The zoom a pinch stops at, out and in. Fitting the whole map may go further out (FIT_MIN), and a
// pinch from there goes no further out than where it started.
const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 6.0;
const FIT_MIN: f32 = 0.02;
fn u(px: f32) -> i32 {
    ui_px(px)
}
fn full(v: &MapView) -> Rect {
    Rect {
        x: 0,
        y: 0,
        w: v.w,
        h: v.h,
    }
}
fn plot(v: &MapView) -> Rect {
    let bottom = if v.selected >= 0 { u(370.0) } else { u(170.0) };
    Rect {
        x: u(16.0),
        y: u(290.0),
        w: v.w - u(32.0) - if v.foreign.is_empty(){0}else{u(470.0)},
        h: v.h - u(290.0) - bottom,
    }
}
// The graph-all.ini this process last read or wrote (it is read once, then kept: the Graph opens without
// reading it again), for the store's root it belongs to.
std::thread_local! { static ALL_INI: std::cell::RefCell<Option<(String, GraphAllIni)>> = const { std::cell::RefCell::new(None) }; }
fn all_ini_load() -> GraphAllIni {
    let root = store_root();
    if let Some(ini) = ALL_INI.with(|c| c.borrow().as_ref().filter(|k| k.0 == root).map(|k| k.1.clone())) {
        return ini;
    }
    let mut ini = GraphAllIni::default();
    graph_all_ini_read_path(&graph_all_ini_path(), &mut ini);
    all_ini_keep(&ini);
    ini
}
fn all_ini_keep(ini: &GraphAllIni) {
    let root = store_root();
    ALL_INI.with(|c| *c.borrow_mut() = Some((root, ini.clone())));
}
/// The kept graph-all.ini forgotten: read from the file at the next open (a child wrote it, or a test).
pub fn mapview_forget_all_ini() {
    ALL_INI.with(|c| *c.borrow_mut() = None);
}
// Opened from the Library (2.0.0-beta.2, the owner's ruling of 2026-11-03): the Graph as a place of its own,
// with no notebook open. It has no List or Pages tab; every node is another
// notebook's page (Open opens that notebook), and nothing here names, renames or repairs the editor's pages.
fn no_notebook(v: &MapView) -> bool {
    v.editor_id.is_empty()
}
/// The Graph was opened with no notebook open (from the Library): no List or Pages tab.
pub fn mapview_no_notebook(v: &MapView) -> bool {
    v.open && no_notebook(v)
}
// The editor's notebook's index in an All model's notebooks (u16::MAX when it has no node there).
fn editor_book(v: &MapView) -> u16 {
    if no_notebook(v) {
        return u16::MAX;
    }
    v.g.books.iter().position(|b| *b == v.editor_id).map_or(u16::MAX, |b| b as u16)
}
// Node i is a page of the editor's notebook (Open opens it there; Rename names it).
fn editor_page(v: &MapView, i: usize) -> bool {
    match v.scope {
        MapScope::All => v.g.nodes[i].book == editor_book(v),
        MapScope::One => v.other.is_none() && !no_notebook(v),
    }
}
// Node i's notebook id.
fn node_notebook(v: &MapView, i: usize) -> FixedStr<STORE_ID_LEN> {
    if v.scope == MapScope::All { v.g.books.get(v.g.nodes[i].book as usize).copied().unwrap_or_default() } else { v.id }
}
fn save(v: &mut MapView) {
    if v.scope == MapScope::All {
        if !v.all_ini.writable {return;}
        if v.layout!=MapLayout::Done {
            // (a child is laying All out: only View's choice is written, the child's positions kept; the copy
            // in memory is not kept again, as the child may have written the file already)
            if v.all_ini.mode != v.mode {v.all_ini.mode = v.mode;graph_all_ini_update_mode(v.mode);mapview_forget_all_ini();}
            return;
        }
        let mode_changed = v.all_ini.mode != v.mode;
        v.all_ini.mode = v.mode;
        // (the store keeps the positions of a notebook the model lacks now; once every notebook has been
        // checked, those of a notebook the index no longer has, gone to the trash, are forgotten)
        let forgot = libindex_complete() && libindex_with(|e| {
            let books = &v.g.books;
            graph_all_ini_forget(&mut v.all_ini, |id| books.iter().any(|b| b.as_str() == id)
                || e.binary_search_by(|x| x.book.id.as_bytes().cmp(id.as_bytes())).is_ok())
        });
        // (graph-all.ini is written only when it changed: one file on the Kindle's slow storage)
        let stored = graph_all_ini_store(&mut v.all_ini, &v.g);
        if (stored || forgot || mode_changed || !v.all_ini.file) && v.all_ini.writable {
            graph_all_ini_write(&mut v.all_ini);
        }
        all_ini_keep(&v.all_ini);
        return;
    }
    if !v.ini.writable || v.id.is_empty(){return;}
    if v.layout!=MapLayout::Done {
        // Seeds are display-only. A child may already have written its finished layout while
        // the UI waits to adopt it; only view preferences may be changed in that interval.
        graph_ini_update_preferences(v.id.as_str(),v.mode,v.ini.show_unlinked);
        return;
    }
    v.ini.mode = v.mode;
    graph_ini_store(&mut v.ini, &v.g);
    if v.ini.writable {graph_ini_write(v.id.as_str(), &mut v.ini);}
}

fn reserve<T>(v: &mut Vec<T>, n: usize) -> bool {
    v.try_reserve_exact(n.saturating_sub(v.len())).is_ok()
}
fn location(v: &MapView, i: usize) -> Option<(f32, f32)> {
    let n = &v.g.nodes[i];
    if !n.linked { return None; }
    if v.mode == GraphMode::Around {
        return v.around.iter().find(|r| r.node == i as i32).map(|r| (r.x, r.y));
    }
    if n.tier == GraphTier::Tray || !n.placed {
        return None;
    }
    if n.tier == GraphTier::Satellite && (!v.ini.show_unlinked || (n.sat_rank >= GRAPH_SAT_DRAWN && v.zoom < 2.0)) {
        return None;
    }
    Some((n.x, n.y))
}
fn screen_point(v: &MapView, p: (f32, f32)) -> (f32, f32) {
    ((p.0 - v.ox) * v.zoom, (p.1 - v.oy) * v.zoom)
}
fn page_number(v: &MapView, i: usize) -> usize {
    v.g.nodes[i].number as usize
}
// The page names of the notebook shown: the editor's (nav), or another's picked in the Library's browser past
// the caps.
fn names<'a>(v: &'a MapView, nav: &'a Nav) -> &'a Nav {
    v.other.as_ref().map_or(nav, |o| &o.1)
}
fn label(v: &MapView, nav: &Nav, i: usize) -> String {
    if v.scope == MapScope::All {
        let name = v.node_names.get(i).map_or("", |s| s.as_str());
        return if name.is_empty() { cformat!(tr("Page %d"), page_number(v, i) as i32) } else { name.to_string() };
    }
    let name = nav_name(names(v, nav), v.g.nodes[i].pid);
    if name.is_empty() {
        cformat!(tr("Page %d"), page_number(v, i) as i32)
    } else {
        name.to_string()
    }
}
// Fits every drawn dot: the whole linked map in All, the rings in Around.
fn fit_points(v: &mut MapView) {
    let r = plot(v);
    let mut bounds = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut count = 0;
    for i in 0..v.g.nodes.len() {
        if let Some((x, y)) = location(v, i) {
            let pad = v.g.nodes[i].r + 12.0;
            bounds.0 = bounds.0.min(x - pad);
            bounds.1 = bounds.1.min(y - pad);
            bounds.2 = bounds.2.max(x + pad);
            bounds.3 = bounds.3.max(y + pad);
            count += 1;
        }
    }
    if count == 0 {
        v.zoom = 1.0;
        v.ox = -(r.x + r.w / 2) as f32;
        v.oy = -(r.y + r.h / 2) as f32;
        return;
    }
    let z = ((r.w - u(80.0)) as f32 / (bounds.2 - bounds.0).max(1.0))
        .min((r.h - u(80.0)) as f32 / (bounds.3 - bounds.1).max(1.0))
        .clamp(FIT_MIN, ZOOM_MAX);
    v.zoom = z;
    v.ox = (bounds.0 + bounds.2) * 0.5 - (r.x + r.w / 2) as f32 / z;
    v.oy = (bounds.1 + bounds.3) * 0.5 - (r.y + r.h / 2) as f32 / z;
}
fn default_fit(v: &mut MapView) {
    fit_points(v);
    v.view_touched = false;
}
fn around_centre(v:&MapView)->i32 {
    if v.selected>=0 {return v.selected;}
    if v.g.open>=0 && v.g.nodes[v.g.open as usize].linked {return v.g.open;}
    v.g.nodes.iter().enumerate().filter(|(_,n)|n.linked).max_by_key(|(_,n)|n.degree).map_or(-1,|(i,_)|i as i32)
}
fn around(v: &mut MapView, centre: i32) -> bool {
    if centre<0 || !v.g.nodes.get(centre as usize).is_some_and(|n|n.linked) || graph_around(&v.g, centre, 2, &mut v.around) < 0 {
        return false;
    }
    v.mode = GraphMode::Around;
    v.centre = centre;
    true
}
// ---- the selection ----
fn is_selected(v: &MapView, i: usize) -> bool {
    if v.picked.is_empty() { v.selected == i as i32 } else { v.picked.get(i).copied().unwrap_or(false) }
}
fn selection_count(v: &MapView) -> usize {
    if v.picked.is_empty() { (v.selected >= 0) as usize } else { v.picked.iter().filter(|&&p| p).count() }
}
fn selection_clear(v: &mut MapView) {
    v.selected = -1;
    v.picked.clear();
}
fn select_only(v: &mut MapView, i: usize) {
    v.picked.clear();
    v.selected = i as i32;
}
// The selection as a bit per node (from `selected` alone when it was that); false when there is no memory for it.
fn picked_spread(v: &mut MapView) -> bool {
    if !v.picked.is_empty() {
        return true;
    }
    if !reserve(&mut v.picked, v.g.nodes.len()) {
        return false;
    }
    v.picked.resize(v.g.nodes.len(), false);
    if v.selected >= 0 {
        v.picked[v.selected as usize] = true;
    }
    true
}
// A selection of one or none is `selected` alone again: the one left is the primary (its card shows).
fn picked_settle(v: &mut MapView) {
    if v.picked.is_empty() {
        return;
    }
    let mut on = v.picked.iter().enumerate().filter(|(_, &p)| p).map(|(i, _)| i);
    let first = on.next();
    if on.next().is_none() {
        v.picked.clear();
        v.selected = first.map_or(-1, |i| i as i32);
    }
}
// Nodes added to the selection (Select all's); the primary stays what it was.
fn select_add(v: &mut MapView, nodes: &[usize]) {
    if nodes.is_empty() || !picked_spread(v) {
        return;
    }
    for &i in nodes {
        v.picked[i] = true;
    }
    picked_settle(v);
}
// Select all: every node drawn.
fn select_all(v: &mut MapView) {
    let drawn: Vec<usize> = (0..v.g.nodes.len()).filter(|&i| location(v, i).is_some()).collect();
    select_add(v, &drawn);
}
// The selection's pages as <notebook id>:<page id>, to find again in a model built anew.
fn picked_keys(v: &MapView) -> Vec<String> {
    let on = v.picked.iter().enumerate().filter(|(_, &p)| p).map(|(i, _)| i);
    on.map(|i| format!("{}:{}", node_notebook(v, i).as_str(), v.g.nodes[i].pid)).collect()
}
fn picked_find(v: &mut MapView, keys: &[String]) {
    v.picked.clear();
    let found: Vec<usize> = keys.iter().filter_map(|k| usize::try_from(node_spec(v, k)).ok()).collect();
    select_add(v, &found);
}
// The view button's and V's switch: All and Adjacent in turn, always the other one, fitted.
fn switch_view(v: &mut MapView) {
    if v.mode == GraphMode::Around {
        v.mode = GraphMode::All;
    } else {
        around(v, around_centre(v));
    }
    default_fit(v);
    save(v);
}
// The view's word: "All" or "Adjacent" (graph.ini still writes mode=around for Adjacent).
fn view_word(mode: GraphMode) -> &'static str {
    // TRANSLATORS: the Graph's view button says which view it shows, "View: All" or "View: Adjacent" (with
    // "View: %s"): every page, or the pages adjacent to one (linked to it, and theirs). Also the selected page's
    // card button that shows its adjacent pages, a third of the card (up to 360 px at 34 px).
    if mode == GraphMode::Around { tr("Adjacent") } else { tr("All") }
}
// The view moved by keys: one Ui refresh of the Graph, as a pan by finger.
fn moved(v: &mut MapView, a: &mut App, nav: &Nav) {
    v.view_touched = true;
    paint(v, a, nav, RefreshMode::Ui);
}

/// A key's or a controller button's action in the Graph, before its hosts' (editor.rs, library.rs): the arrows'
/// pans, zoom out and in about the plot's centre, Fit, View: All / Adjacent, Select all, Enter
/// (GRAPH_KEY_ENTER), and Esc's `back` (the selection; then MapResult::Close for the host to close the Graph).
/// None when the Graph does not take the action (the host goes on with it); one press, one step, each a Ui
/// refresh at most.
pub fn mapview_action(v: &mut MapView, a: &mut App, nav: &Nav, action: i32) -> Option<MapResult> {
    if !v.open {
        return None;
    }
    if !mapview_takes(action) && action != SHORTCUT_BACK && action != GRAPH_KEY_ENTER {
        return None;
    }
    if v.drag.is_some() {
        return Some(MapResult::None); // (keys do nothing while a drag or a pinch is under way)
    }
    if v.list {
        // (the List tab: Esc closes the Graph as its Back does; the plot's keys have nothing to move)
        return Some(if action == SHORTCUT_BACK { MapResult::Close } else { MapResult::None });
    }
    let pr = plot(v);
    match action {
        SHORTCUT_BACK => {
            if selection_count(v) == 0 {
                return Some(MapResult::Close);
            }
            selection_clear(v);
            paint(v, a, nav, RefreshMode::Ui);
        }
        SHORTCUT_PAN_LEFT | SHORTCUT_PAN_RIGHT => {
            let step = pr.w as f32 / 6.0 / v.zoom;
            v.ox += if action == SHORTCUT_PAN_LEFT { -step } else { step };
            moved(v, a, nav);
        }
        SHORTCUT_PAN_UP | SHORTCUT_PAN_DOWN => {
            let step = pr.h as f32 / 6.0 / v.zoom;
            v.oy += if action == SHORTCUT_PAN_UP { -step } else { step };
            moved(v, a, nav);
        }
        SHORTCUT_ZOOM_IN | SHORTCUT_ZOOM_OUT => {
            let f = if action == SHORTCUT_ZOOM_IN { KEY_ZOOM } else { 1.0 / KEY_ZOOM };
            // (as a pinch: no further out than ZOOM_MIN, or than a fit that was already further out)
            let z = (v.zoom * f).clamp(ZOOM_MIN.min(v.zoom), ZOOM_MAX);
            if z != v.zoom {
                let (cx, cy) = ((pr.x + pr.w / 2) as f32, (pr.y + pr.h / 2) as f32);
                v.ox += cx / v.zoom - cx / z;
                v.oy += cy / v.zoom - cy / z;
                v.zoom = z;
                moved(v, a, nav);
            }
        }
        SHORTCUT_FIT_PAGE => {
            default_fit(v);
            paint(v, a, nav, RefreshMode::Ui);
        }
        SHORTCUT_GRAPH_VIEW => {
            switch_view(v);
            paint(v, a, nav, RefreshMode::Ui);
        }
        SHORTCUT_SELECT_ALL => {
            select_all(v);
            paint(v, a, nav, RefreshMode::Ui);
        }
        GRAPH_KEY_ENTER => {
            if selection_count(v) == 1 && v.selected >= 0 {
                return Some(open_result(v, v.selected as usize));
            }
        }
        _ => return None,
    }
    Some(MapResult::None)
}
/// The actions the Graph takes from a key: in the Graph a key bound to one of them does it
/// (shortcuts_lookup_graph: and a file that binds no Graph action has the starter's arrows and V there, which
/// keep their meaning elsewhere). index-map, close-menu and go-back stay their hosts'.
pub fn mapview_takes(action: i32) -> bool {
    matches!(
        action,
        SHORTCUT_PAN_LEFT | SHORTCUT_PAN_RIGHT | SHORTCUT_PAN_UP | SHORTCUT_PAN_DOWN | SHORTCUT_GRAPH_VIEW
            | SHORTCUT_ZOOM_IN | SHORTCUT_ZOOM_OUT | SHORTCUT_FIT_PAGE | SHORTCUT_SELECT_ALL
    )
}

// Places the map from graph.ini: kept positions stay, a few new pages go beside their neighbours,
// a new link between far pages moves only its ends' neighbourhood, and a map that changed a lot is
// laid out again from where its dots were (graph_layout_warm), in a bgjob child when it is big.
fn layout(v: &mut MapView, a: &mut App) -> bool {
    let mut plan = graph_apply_ini(&mut v.g, &v.ini);
    if plan != GraphPlan::Full {
        if graph_place_new(&mut v.g, &mut work_pump) < 0 {
            return false;
        }
        if v.ini.has_gen && v.ini.gen != graph_gen(&v.g) {
            match graph_relax_stressed(&mut v.g, &mut work_pump) {
                -1 => return false,
                -2 => plan = GraphPlan::Full,
                _ => {}
            }
        }
    }
    let warm = v.g.force.iter().any(|&i| v.g.nodes[i as usize].placed);
    let ok = if plan == GraphPlan::Full {
        if graph_layout_in_process(&v.g) {
            graph_layout_warm(&mut v.g, &mut work_pump)
        } else {
            v.layout = MapLayout::Seed;
            if !(if warm { graph_seed_kept(&mut v.g, &mut work_pump) } else { graph_seed(&mut v.g, &mut work_pump) }) {
                return false;
            }
            let mut path = String::new();
            if graph_ini_path(v.id.as_str(), &mut path) != 0 {
                return false;
            }
            v.layout = MapLayout::Running;
            // Save the seed and view choices before the child reads the cache.
            save(v);
            if !bg_pending(BgKind::GraphLayout,v.id.as_str()) {
                bg_queue(BgKind::GraphLayout, Some(v.id.as_str()), Some(&path), a.dpi, false);
            }
            true
        }
    } else {
        true
    };
    if ok && v.layout != MapLayout::Running {
        // A cached map can be reopened while its layout job is still running or queued.
        v.layout = if bg_pending(BgKind::GraphLayout,v.id.as_str()){MapLayout::Running}else{MapLayout::Done};
    }
    ok
}
// The library's pages and notebooks for the panel, from the index (never a walk of every notebook.ini),
// the editor's notebook counted as it is now; None while the index knows no notebook yet.
fn library_totals(nb: &NotebookInfo) -> Option<(usize, usize)> {
    let (pages, books) = libindex_page_totals();
    if books == 0 {
        return None;
    }
    // (no notebook open, or the editor's left out of the Graph: the index's totals are the Graph's; they
    // count no notebook left out)
    if nb.id.is_empty() || libindex_omits(&graph_book_of(nb, &Nav::default())) {
        return Some((pages, books));
    }
    let indexed = libindex_with(|e| e.iter().find(|e| e.book.id == nb.id && e.readable()).map(|e| e.book.clone()));
    let indexed = indexed.filter(|b| !libindex_omits(b)).map(|b| b.pages.len());
    Some(match indexed {
        Some(n) => (pages - n + nb.page_ids.len(), books),
        None => (pages + nb.page_ids.len(), books + 1),
    })
}
// What every open does first: the view reset for the editor's notebook nb, its List tab's rows, the raster.
fn begin(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32) -> bool {
    *v = MapView::default();
    #[cfg(feature = "sim")]
    if std::env::var_os("GRIMOIRE_SIM_MAP_NOMEM").is_some_and(|s| s == "1") {
        app_toast(a, tr("Not enough memory for the Graph."), 3000);
        return false;
    }
    v.w = a.w;
    v.h = a.h;
    v.editor_id = nb.id;
    v.open_pid = open_pid;
    v.selected = -1;
    v.centre = -1;
    if canvas_init(&mut v.canvas, a.w, a.h) != 0 || !reserve(&mut v.list_pages, nb.page_ids.len()) {
        app_toast(a, tr("Not enough memory for the Graph."), 3000);
        return false;
    }
    v.list_pages.extend_from_slice(&nb.page_ids);
    v.list_rows = nav.links.iter().filter(|l| nb.page_ids.contains(&l.pid)).copied().collect();
    // (cached: the key parses the target and allocates its notebook id, so it is made once a row;
    // sort_by_key would make it at every comparison, O(n log n) parses and allocations)
    v.list_rows.sort_by_cached_key(|l| (nav_external_target(l.target.as_str()).map_or(String::new(), |(id, _)| id.to_string()), l.pid, l.seed));
    v.list_name = nb.name.as_str().to_string();
    libindex_load(); // (one file, if the first idle tick has not read it yet)
    true
}
// The view of one notebook, `shown` (the editor's nb, or another's) with its Nav: today's Graph, its
// graph.ini, View: All or Around, and lane C's anchors.
fn build_one(v: &mut MapView, a: &mut App, nb: &NotebookInfo, shown: &NotebookInfo, shown_nav: &Nav, open_pid: u32, around_open: bool) -> bool {
    v.scope = MapScope::One;
    v.id = shown.id;
    v.g = GraphModel::default();
    v.pages.clear();
    v.foreign.clear();
    v.ini = GraphIni::default();
    let n = shown.page_ids.len();
    let graph_nav = nav_for_graph(shown_nav);
    if !reserve(&mut v.pages, n) || !reserve(&mut v.hits, n + 32) || !reserve(&mut v.labels, n)
        || graph_build(&mut v.g, shown, &graph_nav, open_pid, true) != 0
    {
        return false;
    }
    v.pages.extend_from_slice(&shown.page_ids);
    graph_ini_read(shown.id.as_str(), &mut v.ini);
    v.mode = v.ini.mode;
    if !layout(v, a) {
        return false;
    }
    if (around_open || v.mode == GraphMode::Around) && !around(v, around_centre(v)) {v.mode=GraphMode::All;}
    save(v);
    v.link_rows=shown_nav.links.iter().filter(|l|shown.page_ids.contains(&l.pid)).copied().collect();
    // (the same cached key as list_rows's sort: one parse and allocation a row, not one a comparison)
    v.link_rows.sort_by_cached_key(|l| (nav_external_target(l.target.as_str()).map_or(String::new(),|(id,_)|id.to_string()),l.pid,l.seed));
    v.notebook_name=shown.name.as_str().to_string();
    v.link_count=graph_nav.links.iter().filter(|l|shown.page_ids.contains(&l.pid) &&
        (l.target=="index" || nav_external_target(l.target.as_str()).is_some() || nav_link_page(l,shown)>=0)).count();
    v.page_totals=library_totals(nb);
    v.index_gen = libindex_generation();
    anchors_build(v, shown);
    true
}
// What every open does last: shown, fitted, drawn whole.
fn finish(v: &mut MapView, a: &mut App, nav: &Nav) {
    v.open = true;
    default_fit(v);
    v.last_input_ms = now_ms();
    mapview_draw(v, a, nav, full(v));
    app_invalidate(a, full(v), RefreshMode::Full);
}
/// The Graph of one notebook, the editor's nb: what the editor's Graph falls back to past the Graph of all
/// notebooks' caps.
pub fn mapview_open(
    v: &mut MapView,
    a: &mut App,
    nb: &NotebookInfo,
    nav: &Nav,
    open_pid: u32,
    around_open: bool,
) -> bool {
    if nb.id.is_empty() {
        return false; // (no notebook open: no notebook's own Graph)
    }
    if !begin(v, a, nb, nav, open_pid) {
        return false;
    }
    if !build_one(v, a, nb, nb, nav, open_pid, around_open) {
        app_toast(a, tr("Not enough memory for the Graph."), 3000);
        return false;
    }
    finish(v, a, nav);
    true
}
// The Graph of all notebooks' model from the library index, the editor's notebook live (unsaved links
// included), with each node's page name and each notebook's name. graph_build_all's result.
fn build_all(v: &mut MapView, nb: &NotebookInfo, nav: &Nav, open_pid: u32) -> i32 {
    let live = graph_book_of(nb, nav);
    let mut g = GraphModel::default();
    // (no notebook open: every notebook from the index, no page "here")
    let rc = if nb.id.is_empty() { libindex_graph(&mut g, None, None) } else { libindex_graph(&mut g, Some(&live), Some((nb.id.as_str(), open_pid))) };
    if rc != 0 {
        return rc;
    }
    let n = g.nodes.len();
    let mut node_names = Vec::new();
    let mut book_names = Vec::new();
    if !reserve(&mut node_names, n) || !reserve(&mut book_names, g.books.len()) || !reserve(&mut v.hits, n + 32) || !reserve(&mut v.labels, n) {
        return -1;
    }
    libindex_with(|entries| {
        let book = |b: usize| -> Option<&GraphBook> {
            let id = g.books[b];
            if id == live.id && !live.id.is_empty() {
                return Some(&live);
            }
            entries.binary_search_by(|e| e.book.id.as_bytes().cmp(id.as_bytes())).ok().map(|k| &entries[k].book)
        };
        for b in 0..g.books.len() {
            book_names.push(book(b).map_or_else(String::new, |b| b.name.clone()));
        }
        for nd in &g.nodes {
            let name = if nd.named { book(nd.book as usize).and_then(|b| b.names.binary_search_by_key(&nd.pid, |e| e.0).ok().map(|k| b.names[k].1.clone())) } else { None };
            node_names.push(name.unwrap_or_default());
        }
    });
    v.broken = g.nodes.iter().map(|n| n.broken.max(0) as usize).sum();
    v.left_out = libindex_left_out();
    v.index_gen = libindex_generation();
    v.node_names = node_names;
    v.book_names = book_names;
    v.link_count = g.edges.iter().map(|e| e.mult.max(0) as usize).sum::<usize>()
        + g.nodes.iter().map(|n| (n.index_links + n.self_links).max(0) as usize).sum::<usize>();
    v.g = g;
    0
}
// The Graph of all notebooks for the editor's nb: built, placed from graph-all.ini, Around when asked.
// 0, -1 (memory) or GRAPH_ALL_TOO_BIG.
fn open_all_in(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32, around_open: bool) -> i32 {
    v.scope = MapScope::All;
    v.id = FixedStr::new();
    v.other = None;
    v.foreign.clear();
    v.pages.clear();
    v.link_rows.clear();
    v.notebook_name.clear();
    let rc = build_all(v, nb, nav, open_pid);
    if rc != 0 {
        return rc;
    }
    v.all_ini = all_ini_load();
    v.mode = v.all_ini.mode;
    if !layout_all(v, a) {
        return -1;
    }
    if (around_open || v.mode == GraphMode::Around) && !around(v, around_centre(v)) {v.mode=GraphMode::All;}
    save(v);
    v.page_totals = library_totals(nb);
    v.index_built_ms = now_ms();
    0
}
/// The Graph as it opens (2.0.0-beta.2): every notebook (`pick` None). Past the caps of the Graph of all
/// notebooks it says so and opens the editor's notebook; with no notebook open the Library then opens the
/// notebooks' browser, and the one picked there opens with `pick` (Back to the Graph comes back to it).
pub fn mapview_open_scoped(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32, around_open: bool, pick: Option<&str>) -> bool {
    match pick {
        Some(id) if id == nb.id.as_str() => return mapview_open(v, a, nb, nav, open_pid, around_open),
        Some(id) => {
            if !begin(v, a, nb, nav, open_pid) {
                return false;
            }
            if build_other(v, a, nb, id) {
                finish(v, a, nav);
                return true;
            }
            // (it does not read: the editor's notebook, or with none open every notebook)
            if !nb.id.is_empty() {
                return mapview_open(v, a, nb, nav, open_pid, around_open);
            }
        }
        None => {}
    }
    if !begin(v, a, nb, nav, open_pid) {
        return false;
    }
    let t0 = now_ms();
    match open_all_in(v, a, nb, nav, open_pid, around_open) {
        0 => {
            let built = now_ms().wrapping_sub(t0);
            finish(v, a, nav);
            // (what an open costs on the device: the model and kept positions, then the first draw)
            qlog!("graph: All notebooks open, %d notebooks, %d nodes, %d edges: built %u ms, drawn %u ms", v.g.books.len() as i32,
                v.g.nodes.len() as i32, v.g.edges.len() as i32, built, now_ms().wrapping_sub(t0).wrapping_sub(built));
            true
        }
        GRAPH_ALL_TOO_BIG if nb.id.is_empty() => {
            // TRANSLATORS: the Graph opened from the Library cannot draw every notebook: the library has more
            // linked pages or links than it can draw at once; the notebooks' browser opens under it to show
            // one (a toast, 2 lines at most).
            app_toast(a, tr("Too many linked pages to draw all notebooks."), 4000);
            v.too_big = true;
            false
        }
        GRAPH_ALL_TOO_BIG => {
            if !build_one(v, a, nb, nb, nav, open_pid, around_open) {
                app_toast(a, tr("Not enough memory for the Graph."), 3000);
                return false;
            }
            finish(v, a, nav);
            // TRANSLATORS: the Graph (Index, Graph tab) opens on the notebook instead of every notebook: the
            // library has more linked pages or links than it can draw at once (a toast, 2 lines at most).
            app_toast(a, tr("Too many linked pages: showing this notebook only."), 4000);
            true
        }
        _ => {
            app_toast(a, tr("Not enough memory for the Graph."), 3000);
            false
        }
    }
}
// One notebook's view of notebook id, another than the editor's: its notebook.ini and Nav read (three
// files, on the tap). False when it does not read.
fn build_other(v: &mut MapView, a: &mut App, nb: &NotebookInfo, id: &str) -> bool {
    let mut info = NotebookInfo::default();
    if store_load_info(id, &mut info) != 0 {
        return false;
    }
    let mut onav = Nav::default();
    nav_init(&mut onav);
    nav_load(id, &mut onav);
    let shown = info.clone();
    v.other = Some(Box::new((info, onav)));
    let other_nav = v.other.as_ref().map(|o| o.1.clone()).unwrap_or_default();
    if !build_one(v, a, nb, &shown, &other_nav, 0, false) {
        v.other = None;
        return false;
    }
    true
}
/// The notebook the Graph shows when it shows one (None for every notebook): what the Library's Graph keeps
/// while a notebook it opened is in front (MapKeep), so Back comes back to it.
pub fn mapview_pick(v: &MapView) -> Option<FixedStr<STORE_ID_LEN>> {
    (v.open && v.scope == MapScope::One).then_some(v.id)
}
// layout() for the Graph of all notebooks: graph-all.ini's positions, new pages and new components placed,
// a stretched new link relaxed, and a library that changed a lot laid out again from where its dots were,
// in a bgjob child (GraphLayoutAll) when it is big.
fn layout_all(v: &mut MapView, a: &mut App) -> bool {
    let mut plan = graph_all_apply_ini(&mut v.g, &v.all_ini);
    // (links changed between kept pages only when the kept part's gen is not the file's: new pages and new
    // notebooks alone stretch no kept edge, and move no kept dot)
    let kept_changed = v.all_ini.has_gen && v.all_ini.gen != graph_gen_of(&v.g, |n| n.placed);
    if plan != GraphPlan::Full {
        if graph_place_new(&mut v.g, &mut work_pump) < 0 {
            return false;
        }
        if kept_changed {
            match graph_relax_stressed(&mut v.g, &mut work_pump) {
                -1 => return false,
                -2 => plan = GraphPlan::Full,
                _ => {}
            }
        }
    }
    let warm = v.g.force.iter().any(|&i| v.g.nodes[i as usize].placed);
    let ok = if plan == GraphPlan::Full {
        if graph_layout_in_process(&v.g) {
            graph_layout_warm(&mut v.g, &mut work_pump)
        } else {
            v.layout = MapLayout::Seed;
            if !(if warm { graph_seed_kept(&mut v.g, &mut work_pump) } else { graph_seed(&mut v.g, &mut work_pump) }) {
                return false;
            }
            v.layout = MapLayout::Running;
            save(v);
            // (the child builds the model from the index as this process has it: written first)
            libindex_flush();
            if !bg_pending(BgKind::GraphLayoutAll, "") {
                bg_queue(BgKind::GraphLayoutAll, None, Some(&graph_all_ini_path()), a.dpi, false);
            }
            true
        }
    } else {
        true
    };
    if ok && v.layout != MapLayout::Running {
        v.layout = if bg_pending(BgKind::GraphLayoutAll, "") { MapLayout::Running } else { MapLayout::Done };
    }
    ok
}
// The anchors: other notebooks' pages linked from here (the links), and linking here (the backlinks cache),
// one box per page. Each other notebook is looked at once (backlinks_book: its notebook.ini and page names,
// kept between builds, never its links.ini), and where each target goes is worked out from that; a link
// whose page or notebook is gone is a broken anchor.
fn anchors_build(v: &mut MapView, nb: &NotebookInfo) {
    v.foreign.clear();
    v.anchors_gen = backlinks_generation();
    let mut books: Vec<(FixedStr<STORE_ID_LEN>, Option<std::rc::Rc<BlBook>>)> = Vec::new();
    fn book(books: &mut Vec<(FixedStr<STORE_ID_LEN>, Option<std::rc::Rc<BlBook>>)>, id: &str) -> Option<std::rc::Rc<BlBook>> {
        if let Some((_, b)) = books.iter().find(|(k, _)| *k == id) {
            return b.clone();
        }
        let b = backlinks_book(id);
        if books.try_reserve(1).is_ok() {
            books.push((FixedStr::from_str(id), b.clone()));
        }
        b
    }
    let add = |v: &mut MapView, books: &mut Vec<_>, target: &str, here: u32, out: bool, first: (u32, u32)| {
        let Some((id, pid)) = nav_external_target(target) else { return };
        if let Some(f) = v.foreign.iter_mut().find(|f| f.target == target) {
            f.out |= out;
            f.inc |= !out;
            match f.here.iter().position(|&p| p == here) {
                Some(k) => f.here_out[k] |= out,
                None => {
                    f.here.push(here);
                    f.here_out.push(out);
                }
            }
            return;
        }
        if v.foreign.try_reserve(1).is_err() {
            return;
        }
        let other = book(books, id);
        let state = nav_target_state_in(target, other.as_ref().map(|b| &b.info));
        let broken = state.as_ref().is_some_and(nav_target_broken);
        let (notebook, number, label) = match other.as_deref().map(|b| (&b.info, &b.names)) {
            Some((other, names)) => {
                let number = other.page_ids.iter().position(|&p| p == pid).map_or(0, |n| n + 1);
                let name = nav_name(names, pid);
                let label = if number == 0 { tr("Page gone").to_string() } else if name.is_empty() { cformat!(tr("Page %d"), number as i32) } else { format!("{number} · {name}") };
                (if other.name.is_empty() { tr("Notebook unavailable").to_string() } else { other.name.as_str().to_string() }, number, label)
            }
            None => (state.as_ref().map_or_else(|| tr("Notebook unavailable").to_string(), nav_target_state_label), 0, String::new()),
        };
        // (a page that links here and is gone since is the cache being behind: no anchor for it)
        if !out && number == 0 {
            return;
        }
        v.foreign.push(ForeignPlace { target: FixedStr::from_str(target), key: FixedStr::from_str(id), notebook, label, number, out, inc: !out, broken, here: vec![here], here_out: vec![out], first });
    };
    let links = std::mem::take(&mut v.link_rows);
    for l in &links {
        if nav_external_target(l.target.as_str()).is_some() {
            add(v, &mut books, l.target.as_str(), l.pid, true, (l.pid, l.seed));
        }
    }
    v.link_rows = links;
    for (here, id, src) in backlinks_into(nb.id.as_str()) {
        if !nb.page_ids.contains(&here) {
            continue;
        }
        if let Some(t) = nav_notebook_target(id.as_str(), src) {
            add(v, &mut books, &t, here, false, (0, 0));
        }
    }
    // Grouped by notebook (their ids' order, as the List groups them), pages in their order there, the
    // broken ones last in each group.
    v.foreign.sort_by(|a, b| (a.key, a.broken, a.number, a.target).cmp(&(b.key, b.broken, b.number, b.target)));
}

/// The backlinks cache learned more since the map opened (its idle check): the anchors made again and the
/// map drawn again. No dot moves. True when it changed.
pub fn mapview_anchors_refresh(v: &mut MapView, nb: &NotebookInfo, _nav: &Nav) -> bool {
    if !v.open || v.scope == MapScope::All || v.anchors_gen == backlinks_generation() {
        return false;
    }
    let before: Vec<(FixedStr<NAV_TARGET_LEN>, bool, bool)> = v.foreign.iter().map(|f| (f.target, f.out, f.inc)).collect();
    let shown = v.other.as_ref().map_or_else(|| nb.clone(), |o| o.0.clone());
    anchors_build(v, &shown);
    let changed = before != v.foreign.iter().map(|f| (f.target, f.out, f.inc)).collect::<Vec<_>>();
    if changed {
        v.needs_redraw = true;
    }
    changed
}

// A List row's link (the editor's notebook's) goes to another notebook's page that was deleted, or to a
// notebook that was: what anchors_build calls a broken anchor, worked out from the target alone, so the List
// repairs it in every scope (All notebooks has no anchors; another notebook's view has that notebook's).
// That notebook is looked at as the anchors look at it (backlinks_book: kept between taps).
fn link_broken(target: &str) -> bool {
    let Some((id, _)) = nav_external_target(target) else { return false };
    let other = backlinks_book(id);
    nav_target_state_in(target, other.as_ref().map(|b| &b.info)).as_ref().is_some_and(nav_target_broken)
}

fn live_anchors(v: &MapView) -> usize {
    v.foreign.iter().filter(|f| !f.broken).count()
}

/// What the Graph opened from the Library keeps while a notebook it opened is in front (2.0.0-beta.2): the
/// notebook picked in the browser past the caps (None: every notebook), the view where the user left it, and the page the
/// Open went to, selected again when Back comes back to the Graph.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MapKeep {
    pub pick: Option<FixedStr<STORE_ID_LEN>>,
    zoom: f32,
    ox: f32,
    oy: f32,
    touched: bool,
    w: i32,
    h: i32,
    selected: Option<(FixedStr<STORE_ID_LEN>, u32)>,
}
/// The Graph's MapKeep now (its notebook pick, view and selected page).
pub fn mapview_keep(v: &MapView) -> MapKeep {
    let selected = (v.open && v.selected >= 0 && (v.selected as usize) < v.g.nodes.len())
        .then(|| (node_notebook(v, v.selected as usize), v.g.nodes[v.selected as usize].pid));
    MapKeep { pick: mapview_pick(v), zoom: v.zoom, ox: v.ox, oy: v.oy, touched: v.view_touched, w: v.w, h: v.h, selected }
}
/// A Graph just opened again as `keep` had it: the page selected again when it is there, and a view the user
/// had moved put back when the screen is the size it was (else the open's fit stays). Drawn whole.
pub fn mapview_restore(v: &mut MapView, a: &mut App, nav: &Nav, keep: &MapKeep) {
    if !v.open {
        return;
    }
    if let Some((id, pid)) = keep.selected {
        let i = node_spec(v, &format!("{}:{}", id.as_str(), pid));
        if i >= 0 && location(v, i as usize).is_some() {
            v.selected = i;
        }
    }
    if keep.touched && keep.w == v.w && keep.h == v.h && keep.zoom > 0.0 {
        v.zoom = keep.zoom;
        v.ox = keep.ox;
        v.oy = keep.oy;
        v.view_touched = true;
    }
    paint(v, a, nav, RefreshMode::Full);
}

pub fn mapview_close(v: &mut MapView) {
    // Failed opens have not read the cache or established a complete model.
    if v.open {save(v);}
    *v = MapView::default();
}
fn button(v: &mut MapView, c: &mut Canvas, r: Rect, id: u8, text: &str, icon: IconKind, active: bool) {
    ui_button(
        c,
        r,
        icon,
        text,
        if active {
            ButtonStyle::Plain
        } else {
            ButtonStyle::Disabled
        },
    );
    if active {
        v.hits.push(MapHit {
            rect: r,
            kind: MapHitKind::Control(id),
            centre: (0.0, 0.0),
            radius: 0.0,
        });
    }
}
fn ring(c: &mut Canvas, x: f32, y: f32, r: f32, t: f32) {
    canvas_disc_aa(c, x, y, r, 0, 1.0);
    if r > t {
        canvas_disc_aa(c, x, y, r - t, 255, 1.0);
    }
}
// Node i's radius on screen. In the Graph of all notebooks a dot is never drawn under 3 px, so a whole
// library's fit reads as a shape and not as white speckle.
fn dot_r(v: &MapView, i: usize) -> f32 {
    let r = v.g.nodes[i].r * v.zoom;
    if v.scope == MapScope::All { r.max(3.0) } else { r }
}
// The white knockout around a dot: 4 px, and in the Graph of all notebooks no wider than the dot itself
// (it scales with the dot, so small dots keep their black).
fn knockout(v: &MapView, r: f32) -> f32 {
    if v.scope == MapScope::All { 4.0f32.min(r) } else { 4.0 }
}
fn visible_rect(v: &MapView, i: usize) -> Rect {
    location(v, i).map_or(RECT_EMPTY, |p| {
        let (x, y) = screen_point(v, p);
        let radius = dot_r(v, i) + 12.0;
        rect_from_points(x, y, x, y, radius)
    })
}
fn render(v: &mut MapView, c: &mut Canvas, nav: &Nav) {
    canvas_reset_clip(c);
    canvas_fill(c, full(v), 255);
    v.hits.clear();
    v.labels.clear();
    if v.list {render_list(v,c,nav);return;}
    let pr = plot(v);
    render_foreign(v,c);
    canvas_set_clip(c, pr);
    for k in 0..v.g.edges.len() {
        if k & 63 == 0 {
            work_pump();
        }
        let e = v.g.edges[k];
        let (Some(pa), Some(pb)) = (location(v, e.a as usize), location(v, e.b as usize)) else {
            continue;
        };
        let (a, b) = (screen_point(v, pa), screen_point(v, pb));
        let selected = is_selected(v, e.a as usize) || is_selected(v, e.b as usize);
        let width = if selected {
            5.0
        } else {
            (2.5 + (e.mult - 1) as f32).min(5.0)
        };
        let incoming = selected
            && if v.selected == e.a {
                e.dir & GRAPH_DIR_AB == 0
            } else {
                e.dir & GRAPH_DIR_BA == 0
            };
        if incoming {
            canvas_dash_poly(c, &[a, b], u(5.0), u(20.0), u(14.0), 0, 0);
        } else {
            canvas_line_aa(c, a.0, a.1, b.0, b.1, width, 0);
        }
    }
    // All knockouts precede all dots, so a neighbouring dot cannot erase another dot's ring.
    for i in 0..v.g.nodes.len() {
        if i & 63 == 0 {
            work_pump();
        }
        if let Some(p) = location(v, i) {
            let (x, y) = screen_point(v, p);
            let r = dot_r(v, i);
            canvas_disc_aa(c, x, y, r + knockout(v, r), 255, 1.0);
        }
    }
    // The selected node's notebook (in the Graph of all notebooks): each of its dots gets a thin outer ring.
    let same_book = (v.scope == MapScope::All && v.selected >= 0).then(|| v.g.nodes[v.selected as usize].book);
    for i in 0..v.g.nodes.len() {
        if i & 63 == 0 {
            work_pump();
        }
        let Some(p) = location(v, i) else {
            continue;
        };
        let (x, y) = screen_point(v, p);
        let n = v.g.nodes[i];
        let r = dot_r(v, i);
        if !rect_overlaps(visible_rect(v, i), pr) {
            continue;
        }
        // Outer rings first: their white interiors must not erase the dot's fill or inner ring. Every selected
        // node has the selected ring.
        if is_selected(v, i) {
            ring(c, x, y, r + 12.0, 8.0);
        } else if same_book == Some(n.book) {
            ring(c, x, y, r + 5.0, 1.5);
        }
        if n.open {
            // ("you are here": at least 8 px across even at a whole library's fit)
            let o = if v.scope == MapScope::All { (r + 6.0).max(8.0) } else { r + 6.0 };
            ring(c, x, y, o, 2.0);
        }
        let level = match n.degree {
            0 => 0,
            1 => 1,
            2 | 3 => 2,
            _ => 4,
        };
        canvas_disc_pattern(c, x, y, r, level);
        if level < 4 {
            ring(c, x, y, r, 3.0);
            canvas_disc_pattern(c, x, y, (r - 3.0).max(0.0), level);
        }
        if n.index_links > 0 {
            canvas_lozenge(c, x + r, y - r, u(8.0) as f32, 0);
        }
        if n.broken > 0 {
            // (a link of it goes to a page or notebook that is gone: lane C's dashed language)
            let side = u(16.0);
            canvas_dash_rect(c, Rect::new((x + r * 0.7) as i32, (y + r * 0.7) as i32, side, side), u(2.0), u(5.0), u(4.0), 0);
        }
        v.hits.push(MapHit {
            rect: rect_from_points(x, y, x, y, r.max(u(59.0) as f32)),
            kind: MapHitKind::Node(i),
            centre: (x, y),
            radius: r.max(u(59.0) as f32),
        });
    }
    // Priority order is selected, open, neighbours, then named pages by degree.
    let mut candidates = Vec::new();
    if reserve(&mut candidates, v.g.nodes.len()) {
        for i in 0..v.g.nodes.len() {
            if location(v, i).is_some()
                && (v.g.nodes[i].named
                    || v.zoom >= 2.0
                    || v.mode == GraphMode::Around
                    || is_selected(v, i)
                    || v.g.nodes[i].open)
            {
                candidates.push(i);
            }
        }
        candidates.sort_unstable_by_key(|&i| {
            (
                if i as i32 == v.selected {
                    0
                } else if is_selected(v, i) {
                    1
                } else if v.g.nodes[i].open {
                    2
                } else if v.selected >= 0 && graph_edge_between(&v.g, v.selected, i as i32) >= 0 {
                    3
                } else {
                    4
                },
                std::cmp::Reverse(v.g.nodes[i].degree),
                v.g.nodes[i].pid,
            )
        });
        // The Graph of all notebooks places at most LABELS_ALL labels at fit, and tests a label against the dots
        // near it (a coarse screen grid), never against every dot: 2000 names x 8000 dots is 16 million tests.
        let all = v.scope == MapScope::All;
        let grid = if all { dot_grid(v) } else { DotGrid::default() };
        let centre_book = (all && v.mode == GraphMode::Around && v.centre >= 0).then(|| v.g.nodes[v.centre as usize].book);
        for i in candidates {
            if all && v.zoom < 2.0 && v.labels.len() >= LABELS_ALL {
                break;
            }
            let (x, y) = screen_point(v, location(v, i).unwrap());
            let r = dot_r(v, i) + u(16.0) as f32;
            let mut name = if v.g.nodes[i].named {label(v,nav,i)} else {cformat!(tr("p. %d"),page_number(v,i) as i32)};
            if centre_book.is_some_and(|b| b != v.g.nodes[i].book) {
                // (in Around, a page of another notebook than the centre's says which)
                name = format!("{name} ({})", v.book_names.get(v.g.nodes[i].book as usize).map_or("", |s| s.as_str()));
            }
            let w = font_measure(FontFace::Bold, u(34.0) as f32, &name).min(u(320.0)) + u(16.0);
            let h = u(48.0);
            let spots = [
                Rect {
                    x: (x + r) as i32,
                    y: y as i32 - h / 2,
                    w,
                    h,
                },
                Rect {
                    x: (x - r) as i32 - w,
                    y: y as i32 - h / 2,
                    w,
                    h,
                },
                Rect {
                    x: x as i32 - w / 2,
                    y: (y - r) as i32 - h,
                    w,
                    h,
                },
                Rect {
                    x: x as i32 - w / 2,
                    y: (y + r) as i32,
                    w,
                    h,
                },
            ];
            if let Some(r) = spots.into_iter().find(|r| {
                rect_intersect(*r, pr) == *r
                    && !v.labels.iter().any(|&old| rect_overlaps(old, *r))
                    && if all { !grid.hits(*r, Some(i)) } else { !(0..v.g.nodes.len()).any(|other| {
                        other != i
                            && location(v, other).is_some_and(|p| {
                                let (cx, cy) = screen_point(v, p);
                                let radius = v.g.nodes[other].r * v.zoom + 4.0;
                                let nearest_x = cx.clamp(r.x as f32, (r.x + r.w) as f32);
                                let nearest_y = cy.clamp(r.y as f32, (r.y + r.h) as f32);
                                (cx - nearest_x).hypot(cy - nearest_y) < radius
                            })
                    }) }
            }) {
                v.labels.push(r);
                canvas_fill(c, r, if is_selected(v, i) { 0 } else { 255 });
                font_draw_left_name(
                    c,
                    FontFace::Bold,
                    u(34.0) as f32,
                    r,
                    &name,
                    if is_selected(v, i) { 255 } else { 0 },
                );
            }
        }
        if all && v.mode == GraphMode::All {
            render_captions(v, c, &grid, pr);
        }
    }
    canvas_reset_clip(c);
    if no_notebook(v) {
        let tabs=graph_navigation(c,v.w);
        for (i,id) in [BACK,GRAPH].into_iter().enumerate() {
            v.hits.push(MapHit{rect:tabs[i],kind:MapHitKind::Control(id),centre:(0.0,0.0),radius:0.0});
        }
    } else {
        let tabs=index_navigation(c,v.w,0);
        for (i,id) in [BACK,GRAPH,LIST,PAGES].into_iter().enumerate() {
            v.hits.push(MapHit{rect:tabs[i],kind:MapHitKind::Control(id),centre:(0.0,0.0),radius:0.0});
        }
    }
    let panel=Rect::new(u(20.0),v.h-u(150.0),v.w-u(40.0),u(130.0));
    ui_frame(c,panel,u(12.0),255,true,0);ui_frame(c,panel,u(12.0),0,false,u(2.0));
    // The view shown now, "View: All" or "View: Adjacent": a tap switches to the other, fitted, after a pan or
    // pinch too.
    let text=cformat!(tr("View: %s"),view_word(v.mode));
    let width=u(420.0).min(v.w/3);
    button(v,c,Rect::new(panel.x+u(12.0),panel.y+u(15.0),width,u(100.0)),VIEW,&text,IconKind::None,true);
    let nodes=v.g.nodes.iter().enumerate().filter(|(i,_)|location(v,*i).is_some()).count()+live_anchors(v);
    let orphans=orphan_count(v);
    let mut stats=cformat!(tr("Nodes: %d · Orphans: %d · Links: %d"),nodes as i32,orphans as i32,v.link_count as i32);
    if v.scope == MapScope::All && v.broken > 0 {
        // TRANSLATORS: after the Graph's counts (Nodes, Orphans, Links), the links in all notebooks that go to a
        // page or notebook that is gone; also the card of a page that has such links (small text).
        stats = format!("{stats} · {}", cformat!(ng_("%d broken link", "%d broken links", v.broken as i64), v.broken as i32));
    }
    if v.scope == MapScope::All {
        if let Some((at, of)) = libindex_building() {
            // TRANSLATORS: after the Graph's counts (Nodes, Orphans, Links) while the Graph of all notebooks reads
            // every notebook for the first time: how many it has read (small text, about 500 px).
            stats = format!("{stats} · {}", cformat!(tr("Reading notebooks: %d of %d"), at as i32, of as i32));
        }
    }
    let here = if v.scope == MapScope::All { v.list_pages.len() } else { v.pages.len() };
    let mut pages=match v.page_totals {
        // TRANSLATORS: the Graph opened from the Library (no notebook open): every page of the notebooks it
        // shows, and how many notebooks (small text, about 1000 px at 30 px).
        Some((total,books)) if no_notebook(v) => cformat!(tr("Pages: %d across %d notebooks"),total as i32,books as i32),
        Some((total,books)) => cformat!(tr("Pages: %d here · %d across %d notebooks"),here as i32,total as i32,books as i32),
        None if no_notebook(v) => String::new(),
        None => cformat!(tr("Pages in this notebook: %d"),here as i32),
    };
    if v.scope == MapScope::All && v.left_out > 0 {
        // TRANSLATORS: after the Graph's page counts: notebooks the Graph of all notebooks could not take in (its
        // limit of links), shown when there are some (small text).
        pages = format!("{pages} · {}", cformat!(ng_("%d notebook left out", "%d notebooks left out", v.left_out as i64), v.left_out as i32));
    }
    let sx=panel.x+width+u(40.0);let sw=panel.x+panel.w-sx-u(16.0);
    for (row,text) in [stats,pages].iter().enumerate() {
        ui_text(c,FontFace::Regular,u(30.0)as f32,u(24.0)as f32,Rect::new(sx,panel.y+u(12.0)+row as i32*u(52.0),sw,u(52.0)),text,1,true,0);
    }
    if v.selected>=0 {
        let i=v.selected as usize;let n=v.g.nodes[i];
        font_draw_left_name(c,FontFace::Bold,u(40.0)as f32,Rect::new(u(30.0),v.h-u(350.0),v.w-u(60.0),u(55.0)),&label(v,nav,i),0);
        let mine = editor_page(v, i);
        let mut details = if v.scope == MapScope::All && !mine {
            let book = v.book_names.get(n.book as usize).map_or("", |s| s.as_str());
            // TRANSLATORS: the Graph of all notebooks' card for a page of another notebook than the open one: its
            // notebook's name, its page number there, then its links (small text, the screen's width).
            cformat!(tr("%s, page %d. Links to %d, linked from %d."), book, page_number(v,i) as i32, n.links_to, n.linked_from)
        } else {
            cformat!(tr("Page %d. Links to %d, linked from %d."),page_number(v,i)as i32,n.links_to,n.linked_from)
        };
        if n.broken > 0 {
            details = format!("{details} {}", cformat!(ng_("%d broken link", "%d broken links", n.broken as i64), n.broken));
        }
        font_draw_left_name(c,FontFace::Regular,u(28.0)as f32,Rect::new(u(30.0),v.h-u(295.0),v.w-u(60.0),u(45.0)),&details,0);
        let gap=u(12.0);let bw=((v.w-u(64.0))/3).min(u(360.0));
        for (k,id,text) in [(0,OPEN,tr("Open")),(1,ADJACENT,view_word(GraphMode::Around)),(2,RENAME,tr("Rename"))] {
            button(v,c,Rect::new(u(20.0)+k*(bw+gap),v.h-u(240.0),bw,u(80.0)),id,text,IconKind::None,id!=RENAME||(nav.writable&&mine));
        }
    }
}

// The pages with no link: in one notebook's view its own, in the Graph of all notebooks every notebook's
// (the library's pages less the linked ones).
fn orphan_count(v: &MapView) -> usize {
    if v.scope == MapScope::All {
        return v.page_totals.map_or(0, |t| t.0.saturating_sub(v.g.nodes.len()));
    }
    v.g.nodes.iter().filter(|n|!n.linked).count()
}

/// The Graph of all notebooks' cap on the labels placed at fit (the highest priority first).
const LABELS_ALL: usize = 80;
const GRID_CELL: f32 = 96.0;
// The drawn dots by screen cell, and the largest of them: what a label or caption is tested against.
#[derive(Default)]
struct DotGrid {
    cells: std::collections::HashMap<(i32, i32), Vec<(usize, f32, f32, f32)>>,
    reach: f32,
}
impl DotGrid {
    // Whether rect r overlaps a dot (its knockout included) other than `skip`.
    fn hits(&self, r: Rect, skip: Option<usize>) -> bool {
        let pad = self.reach;
        let (x0, y0) = (((r.x as f32 - pad) / GRID_CELL).floor() as i32, ((r.y as f32 - pad) / GRID_CELL).floor() as i32);
        let (x1, y1) = ((((r.x + r.w) as f32 + pad) / GRID_CELL).floor() as i32, (((r.y + r.h) as f32 + pad) / GRID_CELL).floor() as i32);
        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let Some(list) = self.cells.get(&(gx, gy)) else { continue };
                for &(i, cx, cy, radius) in list {
                    if Some(i) == skip {
                        continue;
                    }
                    let nx = cx.clamp(r.x as f32, (r.x + r.w) as f32);
                    let ny = cy.clamp(r.y as f32, (r.y + r.h) as f32);
                    if (cx - nx).hypot(cy - ny) < radius {
                        return true;
                    }
                }
            }
        }
        false
    }
}
fn dot_grid(v: &MapView) -> DotGrid {
    let mut g = DotGrid::default();
    for i in 0..v.g.nodes.len() {
        if let Some(p) = location(v, i) {
            let (cx, cy) = screen_point(v, p);
            let radius = dot_r(v, i) + 4.0;
            g.reach = g.reach.max(radius);
            g.cells.entry(((cx / GRID_CELL).floor() as i32, (cy / GRID_CELL).floor() as i32)).or_default().push((i, cx, cy, radius));
        }
    }
    g
}

// Each notebook's name over its island (the Graph of all notebooks, View: All), in the label face, where the
// island is wider on screen than its caption and the caption overlaps no dot and no label.
fn render_captions(v: &mut MapView, c: &mut Canvas, grid: &DotGrid, pr: Rect) {
    v.captions = 0;
    let islands = graph_islands(&v.g);
    for d in islands {
        let Some(name) = v.book_names.get(d.book as usize).filter(|s| !s.is_empty()).cloned() else { continue };
        let w = font_measure(FontFace::Bold, u(34.0) as f32, &name).min(u(420.0)) + u(16.0);
        let width = 2.0 * d.r * v.zoom;
        if width < w as f32 {
            continue;
        }
        let (x, y) = screen_point(v, (d.x, d.y - d.r));
        let r = Rect::new(x as i32 - w / 2, y as i32 - u(52.0), w, u(48.0));
        if rect_intersect(r, pr) != r || v.labels.iter().any(|&l| rect_overlaps(l, r)) || grid.hits(r, None) {
            continue;
        }
        v.labels.push(r);
        canvas_fill(c, r, 255);
        font_draw_left_name(c, FontFace::Bold, u(34.0) as f32, r, &name, 0);
        canvas_hline(c, r.x + u(8.0), r.x + r.w - u(8.0), r.y + r.h - u(4.0), u(2.0), 0);
        v.captions += 1;
    }
}

pub fn mapview_draw(v: &mut MapView, a: &mut App, nav: &Nav, r: Rect) {
    if !v.open {
        return;
    }
    v.w = a.w;
    v.h = a.h;
    if v.canvas.w != a.w || v.canvas.h != a.h {
        v.drag = None;
        if canvas_init(&mut v.canvas, a.w, a.h) != 0 {
            return;
        }
        default_fit(v);
    }
    let mut canvas = std::mem::take(&mut v.canvas);
    render(v, &mut canvas, nav);
    v.canvas = canvas;
    canvas_blit(&mut a.screen, &v.canvas, r, r.x, r.y);
    v.needs_redraw = false;
}
fn hit(v: &MapView, x: f32, y: f32) -> Option<MapHitKind> {
    if let Some(h) = v
        .hits
        .iter()
        .rev()
        .find(|h| !matches!(h.kind, MapHitKind::Node(_)) && rect_contains(h.rect, x as i32, y as i32))
    {
        return Some(h.kind);
    }
    v.hits
        .iter()
        .filter_map(|h| {
            if matches!(h.kind, MapHitKind::Node(_)) {
                let d = (h.centre.0 - x).hypot(h.centre.1 - y);
                (d <= h.radius).then_some((d, h.kind))
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|p| p.1)
}
fn snapshot(v: &MapView) -> Canvas {
    let mut c = Canvas::default();
    if canvas_init(&mut c, v.w, v.h) == 0 {
        canvas_blit(&mut c, &v.canvas, full(v), 0, 0);
    }
    c
}
fn paint(v: &mut MapView, a: &mut App, nav: &Nav, mode: RefreshMode) {
    mapview_draw(v, a, nav, full(v));
    app_invalidate(a, full(v), mode);
}
/// A pen's second tap on the node it selected counts after this long (a stylus bounce is shorter).
const PEN_REOPEN_MS: u32 = 450;

pub fn mapview_pointer(v: &mut MapView, a: &mut App, nav: &Nav, phase: i32, e: &InEvent) -> MapResult {
    v.last_input_ms = e.t_ms;
    if phase == 0 {
        v.drag = Some(MapDrag {
            x: e.x,
            y: e.y,
            ox: v.ox,
            oy: v.oy,
            zoom: v.zoom,
            distance: 0.0,
            moved: false,
            last_present: e.t_ms,
            saved: snapshot(v),
        });
        return MapResult::None;
    }
    if phase == 1 && v.list {
        if let Some(d)=&mut v.drag {
            if (e.y-d.y).abs()>u(50.0) as f32 {d.moved=true;}
        }
        return MapResult::None;
    }
    if phase == 1 {
        let pr = plot(v);
        let Some(d) = v.drag.as_mut() else {
            return MapResult::None;
        };
        // A pen tip slides a millimetre or two on a tap, a fingertip rolls: a tap until TAP_SLOP_MM,
        // as on every other screen (at 1 mm the pen's taps on dots and Back turned into small pans).
        if (e.x - d.x).hypot(e.y - d.y) > mm_to_px(TAP_SLOP_MM, a.dpi) {
            d.moved = true;
        }
        if d.moved {
            v.ox = d.ox - (e.x - d.x) / v.zoom;
            v.oy = d.oy - (e.y - d.y) / v.zoom;
            if e.t_ms.wrapping_sub(d.last_present) >= 180 && !d.saved.px.is_empty() {
                d.last_present = e.t_ms;
                let (dx, dy) = ((e.x - d.x) as i32, (e.y - d.y) as i32);
                canvas_set_clip(&mut a.screen, pr);
                canvas_fill(&mut a.screen, pr, 255);
                canvas_blit(&mut a.screen, &d.saved, pr, pr.x + dx, pr.y + dy);
                canvas_reset_clip(&mut a.screen);
                app_invalidate(a, pr, RefreshMode::Fast);
            }
        }
        return MapResult::None;
    }
    let Some(d) = v.drag.take() else {
        return MapResult::None;
    };
    if touch_cancelled(e) {
        v.ox = d.ox;
        v.oy = d.oy;
        paint(v, a, nav, RefreshMode::Ui);
        return MapResult::None;
    }
    // A press that starts and ends on the same control is its tap, however it slid between.
    let same_control = match (hit(v, d.x, d.y), hit(v, e.x, e.y)) {
        (Some(MapHitKind::Control(p)), Some(MapHitKind::Control(q))) => p == q,
        _ => false,
    };
    if d.moved && same_control {
        v.ox = d.ox;
        v.oy = d.oy;
    } else if d.moved {
        if v.list {let rows=((d.y-e.y)/u(130.0) as f32) as i32;v.list_offset=(v.list_offset as i32+rows).clamp(0,v.list_rows.len().saturating_sub(1) as i32) as usize;}
        else {v.view_touched=true;}
        paint(v, a, nav, RefreshMode::Ui);
        return MapResult::None;
    }
    let selected = v.selected;
    let had_picked = !v.picked.is_empty();
    let mut label_dirty = v.labels.iter().fold(RECT_EMPTY, |r, &l| rect_union(r, l));
    let old_bounds = if selected >= 0 {
        visible_rect(v, selected as usize)
    } else {
        RECT_EMPTY
    };
    match hit(v, e.x, e.y) {
        Some(MapHitKind::Node(i)) => {
            // A tap selects, a tap on the selected node opens it, by pen as by finger (2.0.0-rc.12: a pen's
            // tap often reached the Kindle as two, which selected and opened at once).
            // A tap on the primary of a selection of several selects it alone first.
            let pen = e.type_ == InType::PenUp || e.type_ == InType::PenLeave;
            let alone = selected == i as i32 && !had_picked;
            if alone && !(pen && e.t_ms.wrapping_sub(v.selected_k) < PEN_REOPEN_MS) {
                return open_result(v, i);
            }
            if !alone {
                v.selected_k = e.t_ms;
            }
            select_only(v, i);
        }
        Some(MapHitKind::Link(i)) if !no_notebook(v) => {
            let l = v.list_rows[i];
            if link_broken(l.target.as_str()) {return MapResult::Repair(l.pid, l.seed);}
            return MapResult::Target(l.target);
        }
        Some(MapHitKind::Link(_)) => {}
        Some(MapHitKind::Anchor(i)) => {
            let f = &v.foreign[i];
            // (a broken link of another notebook than the editor's is repaired from that notebook)
            if f.broken && (v.other.is_some() || no_notebook(v)) {return MapResult::None;}
            if f.broken {return MapResult::Repair(f.first.0, f.first.1);}
            return MapResult::Target(f.target);
        }
        Some(MapHitKind::Control(id)) => match id {
            BACK => return MapResult::Close,
            LIST if !no_notebook(v) => {v.list=true;v.list_offset=0;},
            PAGES if !no_notebook(v) => return MapResult::Pages,
            GRAPH => {v.list=false;},
            ADJACENT => {
                if around(v, around_centre(v)) {save(v);default_fit(v);}
            }
            OPEN if v.selected >= 0 => return open_result(v, v.selected as usize),
            RENAME if v.selected >= 0 && editor_page(v, v.selected as usize) => return MapResult::Rename(v.g.nodes[v.selected as usize].pid),
            // All and Adjacent in turn, always (a moved view no longer fits first).
            VIEW => switch_view(v),
            _ => {}
        },
        None => selection_clear(v),
    }
    if had_picked {
        paint(v, a, nav, RefreshMode::Ui); // (a selection of several: its rings anywhere)
        return MapResult::None;
    }
    if selected != v.selected {
        let new_bounds = if v.selected >= 0 {
            visible_rect(v, v.selected as usize)
        } else {
            RECT_EMPTY
        };
        for edge in &v.g.edges {
            if edge.a == selected || edge.b == selected || edge.a == v.selected || edge.b == v.selected {
                if let (Some(a), Some(b)) = (location(v, edge.a as usize), location(v, edge.b as usize)) {
                    let (a, b) = (screen_point(v, a), screen_point(v, b));
                    label_dirty = rect_union(label_dirty, rect_from_points(a.0, a.1, b.0, b.1, 8.0));
                }
            }
        }
        // (anchors' lines, and in the Graph of all notebooks the selected page's notebook's rings, reach anywhere)
        if !v.foreign.is_empty() || v.scope == MapScope::All {paint(v,a,nav,RefreshMode::Ui);return MapResult::None;}
        mapview_draw(v, a, nav, full(v));
        label_dirty = v.labels.iter().fold(label_dirty, |r, &l| rect_union(r, l));
        let card = Rect {
            x: 0,
            y: v.h - u(370.0),
            w: v.w,
            h: u(370.0),
        };
        app_invalidate(
            a,
            rect_intersect(
                rect_union(rect_union(rect_union(old_bounds, new_bounds), card), label_dirty),
                full(v),
            ),
            RefreshMode::Ui,
        );
    } else {
        paint(v, a, nav, RefreshMode::Ui);
    }
    MapResult::None
}
// What opening node i is: a page of the editor's notebook opens there; another notebook's page is that
// notebook's target, so the editor opens it there and Back comes back here.
fn open_result(v: &MapView, i: usize) -> MapResult {
    let pid = v.g.nodes[i].pid;
    if editor_page(v, i) {
        return MapResult::Open(pid);
    }
    match nav_notebook_target(node_notebook(v, i).as_str(), pid) {
        Some(t) => MapResult::Target(FixedStr::from_str(&t)),
        None => MapResult::None,
    }
}
pub fn mapview_pinch(v: &mut MapView, a: &mut App, e: &InEvent) {
    if v.list{return;}
    v.last_input_ms = e.t_ms;
    let (x, y) = ((e.x + e.x2) * 0.5, (e.y + e.y2) * 0.5);
    let distance = (e.x2 - e.x).hypot(e.y2 - e.y).max(1.0);
    if e.type_ == InType::PinchBegin {
        v.drag = Some(MapDrag {
            x,
            y,
            ox: v.ox,
            oy: v.oy,
            zoom: v.zoom,
            distance,
            moved: true,
            last_present: e.t_ms,
            saved: snapshot(v),
        });
        return;
    }
    if e.type_ == InType::PinchEnd {
        v.drag = None;
        v.needs_redraw = true;
        return;
    }
    let pr = plot(v);
    let Some(d) = v.drag.as_mut() else {
        return;
    };
    v.view_touched = true;
    let z = (d.zoom * distance / d.distance).clamp(ZOOM_MIN.min(d.zoom), ZOOM_MAX);
    v.ox = d.ox + d.x / d.zoom - x / z;
    v.oy = d.oy + d.y / d.zoom - y / z;
    v.zoom = z;
    if e.t_ms.wrapping_sub(d.last_present) < 180 || d.saved.px.is_empty() {
        return;
    }
    d.last_present = e.t_ms;
    canvas_set_clip(&mut a.screen, pr);
    for yy in pr.y..pr.y + pr.h {
        for xx in pr.x..pr.x + pr.w {
            let sx = (((xx as f32 / z + v.ox - d.ox) * d.zoom).round()) as i32;
            let sy = (((yy as f32 / z + v.oy - d.oy) * d.zoom).round()) as i32;
            a.screen.px[(yy * a.screen.stride + xx) as usize] = if rect_contains(pr, sx, sy) {
                d.saved.px[(sy * d.saved.stride + sx) as usize]
            } else {
                255
            };
        }
    }
    canvas_reset_clip(&mut a.screen);
    app_invalidate(a, pr, RefreshMode::Fast);
}
pub fn mapview_background_done(v: &mut MapView,id:&str, ok: bool) {
    if v.open && v.scope == MapScope::One && v.id.as_str()==id && v.layout == MapLayout::Running {
        if ok {
            v.job_done = true;
        } else {
            v.layout = MapLayout::Seed;
            v.job_done = false;
            v.needs_redraw = true;
        }
    }
}
/// The Graph of all notebooks' layout child (BgKind::GraphLayoutAll) ended: its positions are adopted at the
/// next quiet tick (mapview_tick). A notebook's own layout child never ends All's. The graph-all.ini kept in
/// memory is forgotten whether All is shown or not (closed, or one notebook picked): the child wrote the file,
/// and the next open of All reads it.
pub fn mapview_all_background_done(v: &mut MapView, ok: bool) {
    if ok {
        mapview_forget_all_ini();
    }
    if v.open && v.scope == MapScope::All && v.layout == MapLayout::Running {
        if ok {
            v.job_done = true;
        } else {
            v.layout = MapLayout::Seed;
            v.job_done = false;
            v.needs_redraw = true;
        }
    }
}
// The dot a moved view is anchored on while the layout under it changes: the selected one when it is
// drawn, else the drawn dot nearest the plot's centre; with its world point.
fn view_anchor(v: &MapView) -> Option<(usize, f32, f32)> {
    if v.selected >= 0 {
        if let Some((x, y)) = location(v, v.selected as usize) {
            return Some((v.selected as usize, x, y));
        }
    }
    let r = plot(v);
    let (cx, cy) = ((r.x + r.w / 2) as f32 / v.zoom + v.ox, (r.y + r.h / 2) as f32 / v.zoom + v.oy);
    let mut best: Option<(f32, usize, f32, f32)> = None;
    for i in 0..v.g.nodes.len() {
        if let Some((x, y)) = location(v, i) {
            let d = (x - cx).hypot(y - cy);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, i, x, y));
            }
        }
    }
    best.map(|b| (b.1, b.2, b.3))
}
pub fn mapview_tick(v: &mut MapView, _a: &mut App, now: u32) -> MapResult {
    if v.open && v.job_done && v.drag.is_none() && now.wrapping_sub(v.last_input_ms) >= 500 && v.scope == MapScope::All {
        // The child's positions are kept positions: what changed here meanwhile (a notebook read, a link) is
        // placed beside them, never thrown away.
        let mut ini = GraphAllIni::default();
        if graph_all_ini_read_path(&graph_all_ini_path(), &mut ini) == 0 {
            let anchor = if v.view_touched { view_anchor(v) } else { None };
            v.all_ini = ini;
            all_ini_keep(&v.all_ini);
            graph_all_apply_ini(&mut v.g, &v.all_ini);
            graph_place_new(&mut v.g, &mut work_pump);
            if v.mode == GraphMode::Around {
                around(v, v.centre);
            }
            match anchor {
                None => default_fit(v),
                Some((i, x, y)) => {
                    if let Some((nx, ny)) = location(v, i) {
                        v.ox += nx - x;
                        v.oy += ny - y;
                    }
                }
            }
            v.layout = MapLayout::Done;
        } else {
            v.layout = MapLayout::Seed;
        }
        v.needs_redraw = true;
        v.job_done = false;
    }
    if v.open && v.job_done && v.drag.is_none() && now.wrapping_sub(v.last_input_ms) >= 500 {
        let mut ini = GraphIni::default();
        if graph_ini_read(v.id.as_str(), &mut ini) == 0 {
            // A view the user moved keeps the dot they were looking at under the same point;
            // otherwise the finished layout is fitted, as the seed was.
            let anchor = if v.view_touched { view_anchor(v) } else { None };
            v.ini = ini;
            graph_apply_ini(&mut v.g, &v.ini);
            graph_place_new(&mut v.g, &mut work_pump);
            if v.mode == GraphMode::Around {
                around(v, v.centre);
            }
            match anchor {
                None => default_fit(v),
                Some((i, x, y)) => {
                    if let Some((nx, ny)) = location(v, i) {
                        v.ox += nx - x;
                        v.oy += ny - y;
                    }
                }
            }
            v.layout = MapLayout::Done;
            v.needs_redraw = true;
        } else {
            v.layout = MapLayout::Seed;
            v.needs_redraw = true;
        }
        v.job_done = false;
    }
    MapResult::None
}
// The Graph of all notebooks built again (the index learned more, a page was named), every dot it had kept
// where it was: the new pages and notebooks placed beside them. False when it could not be built (the old
// one stays).
fn rebuild_all(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32) -> bool {
    save(v);
    let picked = picked_keys(v);
    let old = std::mem::take(&mut v.g);
    let keep = (std::mem::take(&mut v.node_names), std::mem::take(&mut v.book_names));
    let selected = (v.selected >= 0).then(|| (node_notebook_of(&old, v.selected as usize), old.nodes[v.selected as usize].pid));
    let centre = (v.centre >= 0 && (v.centre as usize) < old.nodes.len()).then(|| (node_notebook_of(&old, v.centre as usize), old.nodes[v.centre as usize].pid));
    v.g = old.clone();
    if build_all(v, nb, nav, open_pid) != 0 || !layout_all(v, a) {
        v.g = old;
        (v.node_names, v.book_names) = keep;
        return false;
    }
    let find = |g: &GraphModel, k: Option<(FixedStr<STORE_ID_LEN>, u32)>| k.map_or(-1, |(id, pid)| g.books.iter().position(|b| *b == id).map_or(-1, |b| graph_node_in(g, b as u16, pid)));
    v.selected = find(&v.g, selected);
    v.centre = find(&v.g, centre);
    picked_find(v, &picked);
    if v.mode == GraphMode::Around && !around(v, v.centre) {
        v.mode = GraphMode::All;
    }
    save(v);
    v.page_totals = library_totals(nb);
    v.index_built_ms = now_ms();
    v.needs_redraw = true;
    true
}
fn node_notebook_of(g: &GraphModel, i: usize) -> FixedStr<STORE_ID_LEN> {
    g.books.get(g.nodes[i].book as usize).copied().unwrap_or_default()
}
/// The Graph of all notebooks while it is open and untouched: when the library index has learned more (its
/// first build reading notebook after notebook, another version's changes), the model is built again and the
/// new nodes join beside the kept ones (at most every 2 s while the index is first being built, so the
/// screen is not refreshed for every notebook read). True when it was drawn again.
pub fn mapview_library_refresh(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, now: u32) -> bool {
    if !v.open || v.list || v.drag.is_some() || v.layout == MapLayout::Running
        || now.wrapping_sub(v.last_input_ms) < 500 || libindex_generation() == v.index_gen {
        return false;
    }
    if v.scope == MapScope::One {
        // (one notebook's view: only the panel's library totals come from the index)
        v.index_gen = libindex_generation();
        let totals = library_totals(nb);
        if totals != v.page_totals {
            v.page_totals = totals;
            v.needs_redraw = true;
        }
        return v.needs_redraw;
    }
    if libindex_building().is_some() && now.wrapping_sub(v.index_built_ms) < 2000 {
        return false;
    }
    let before = (graph_gen(&v.g), v.g.nodes.len(), v.left_out, v.broken);
    let open_pid = v.open_pid;
    if !rebuild_all(v, a, nb, nav, open_pid) {
        v.index_gen = libindex_generation(); // (not tried again until it learns more)
        return false;
    }
    if before == (graph_gen(&v.g), v.g.nodes.len(), v.left_out, v.broken) && libindex_building().is_some() {
        v.needs_redraw = false;
    }
    if !v.view_touched {
        default_fit(v);
    }
    v.needs_redraw
}
pub fn mapview_rebuild(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32) -> bool {
    if v.scope == MapScope::All {
        return rebuild_all(v, a, nb, nav, open_pid);
    }
    if v.other.is_some() {
        return false;
    }
    save(v);
    let mut g = GraphModel::default();
    if graph_build(&mut g, nb, &nav_for_graph(nav), open_pid, true) != 0 {
        return false;
    }
    let picked = picked_keys(v);
    let primary = (v.selected >= 0 && (v.selected as usize) < v.g.nodes.len()).then(|| v.g.nodes[v.selected as usize].pid);
    // save kept every dot the user has seen in v.ini, so layout starts from them: a rename moves
    // nothing but a dot that grew into its neighbour, and a new link only its far ends' neighbours.
    let old = std::mem::replace(&mut v.g, g);
    if !layout(v, a) {
        v.g = old;
        return false;
    }
    // (the selection found again by page: a page added before it moved its index)
    v.selected = primary.map_or(-1, |pid| graph_node_of(&v.g, pid));
    picked_find(v, &picked);
    if v.mode == GraphMode::Around {
        around(v, v.centre);
    }
    save(v);
    v.needs_redraw = true;
    true
}
pub fn mapview_probe(v: &MapView, key: &str, out: &mut String) -> bool {
    out.clear();
    match key {
        "" => out.push_str(if v.open { "graph" } else { "0" }),
        "stats"=>{let here=if v.scope==MapScope::All{v.list_pages.len()}else{v.pages.len()};
            out.push_str(&format!("{} {} {} {} {}",v.g.nodes.iter().filter(|n|n.linked).count()+live_anchors(v),orphan_count(v),v.link_count,here,v.page_totals.map_or(0,|p|p.0)));},
        // (the view button's word: all or adjacent; touched when the view was moved since it was fitted)
        "view"=>out.push_str(if v.mode==GraphMode::Around{"adjacent"}else{"all"}),
        "touched"=>out.push_str(if v.view_touched{"1"}else{"0"}),
        // (how many nodes are selected, and their page numbers in order)
        "selection"=>out.push_str(&selection_count(v).to_string()),
        "selection.pages"=>{let mut p:Vec<usize>=(0..v.g.nodes.len()).filter(|&i|is_selected(v,i)).map(|i|page_number(v,i)).collect();p.sort_unstable();out.push_str(&p.iter().map(|n|n.to_string()).collect::<Vec<_>>().join(" "));},
        // (the drawn dots as <notebook id>:<page id>, sorted: the Graph of all notebooks' pages repeat page ids)
        "visible.ids"=>{let mut ids:Vec<String>=v.g.nodes.iter().enumerate().filter(|(i,_)|location(v,*i).is_some()).map(|(i,n)|format!("{}:{}",node_notebook(v,i).as_str(),n.pid)).collect();ids.sort();out.push_str(&ids.join(" "));},
        "visible"=>{let mut ids:Vec<_>=v.g.nodes.iter().enumerate().filter(|(i,_)|location(v,*i).is_some()).map(|(_,n)|n.pid).collect();ids.sort_unstable();out.push_str(&ids.iter().map(|p|p.to_string()).collect::<Vec<_>>().join(" "));},
        "nodes" => out.push_str(&v.g.nodes.len().to_string()),
        "edges" => out.push_str(&v.g.edges.len().to_string()),
        "force" => out.push_str(&v.g.force.len().to_string()),
        "satellites" => out.push_str(
            &v.g.nodes
                .iter()
                .filter(|n| n.tier == GraphTier::Satellite)
                .count()
                .to_string(),
        ),
        "tray" => out.push_str(&v.g.tray.len().to_string()),
        // (the footer's: pages without a link)
        "orphans" => out.push_str(&v.g.nodes.iter().filter(|n| !n.linked).count().to_string()),
        "selected" => {
            if v.selected >= 0 {
                out.push_str(&page_number(v, v.selected as usize).to_string());
            }
        }
        // (2.0.0-beta.2) all or one (past the caps); the notebook shown; the Graph of all notebooks'
        // notebooks, captions drawn, broken links and whether the index is still first being built
        "scope" => out.push_str(if v.scope == MapScope::All { "all" } else { "one" }),
        "scope.id" => out.push_str(v.id.as_str()),
        "books" => out.push_str(&v.g.books.len().to_string()),
        "captions" => out.push_str(&v.captions.to_string()),
        "broken" => out.push_str(&v.broken.to_string()),
        "building" => out.push_str(if libindex_building().is_some() { "1" } else { "0" }),
        "selected.id" => {
            if v.selected >= 0 {
                out.push_str(node_notebook(v, v.selected as usize).as_str());
            }
        }
        "list" => out.push_str(if v.list{"1"}else{"0"}),
        // (the Index's tabs drawn: graph list pages, or graph alone when opened from the Library)
        "tabs" => out.push_str(if no_notebook(v) { "graph" } else { "graph list pages" }),
        "no_notebook" => out.push_str(if no_notebook(v) { "1" } else { "0" }),
        // (anchors: other notebooks' pages; foreign counts those whose page is there)
        "foreign"=>out.push_str(&live_anchors(v).to_string()),
        "anchors"=>out.push_str(&v.foreign.len().to_string()),
        "anchors.in"=>out.push_str(&v.foreign.iter().filter(|f|f.inc).count().to_string()),
        "anchors.broken"=>out.push_str(&v.foreign.iter().filter(|f|f.broken).count().to_string()),
        "mode" => out.push_str(if v.mode == GraphMode::All { "all" } else { "around" }),
        "layout" => out.push_str(match v.layout {
            MapLayout::Seed => "seed",
            MapLayout::Running => "running",
            MapLayout::Done => "done",
        }),
        "labels" => out.push_str(&v.labels.len().to_string()),
        "zoom" => out.push_str(&format!("{:.2}", v.zoom)),
        _ => {
            if let Some(k) = key.strip_prefix("anchor.").and_then(|s| s.parse::<usize>().ok()) {
                // Anchor K (from 1): `out`, `in` or `both`, then ` gone` for a broken one, then its target.
                if let Some(f) = k.checked_sub(1).and_then(|k| v.foreign.get(k)) {
                    out.push_str(match (f.out, f.inc) { (true, true) => "both", (true, false) => "out", _ => "in" });
                    if f.broken {out.push_str(" gone");}
                    out.push(' ');
                    out.push_str(f.target.as_str());
                }
            } else if let Some(spec) = key.strip_prefix("node.") {
                let i = node_spec(v, spec);
                if i >= 0 {
                    let n = v.g.nodes[i as usize];
                    let p = location(v, i as usize).map(|p| screen_point(v, p));
                    let (x, y) = p.unwrap_or((0.0, 0.0));
                    out.push_str(&format!(
                        "{} {:.0} {:.0} {:.0} {}",
                        match n.tier {
                            GraphTier::Force => "force",
                            GraphTier::Satellite => "satellite",
                            GraphTier::Tray => "tray",
                            GraphTier::Hidden => "hidden",
                        },
                        n.r * v.zoom,
                        x,
                        y,
                        p.is_some() as i32
                    ));
                }
            } else if let Some(xy) = key.strip_prefix("ui_at.") {
                let Some((x, y)) = xy
                    .split_once('.')
                    .and_then(|(x, y)| Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?)))
                else {
                    return false;
                };
                match hit(v, x, y) {
                    Some(MapHitKind::Node(i)) => out.push_str(&format!("node.{}", v.g.nodes[i].pid)),
                    Some(MapHitKind::Link(i)) => out.push_str(&format!("link.{}", i+1)),
                    Some(MapHitKind::Anchor(i)) => out.push_str(&format!("anchor.{}", i+1)),
                    Some(MapHitKind::Control(id)) => out.push_str(match id {
                        BACK => "back",
                        GRAPH => "graph",
                        LIST => "list",
                        PAGES => "pages",
                        VIEW => "view",
                        OPEN => "card.open",
                        ADJACENT => "card.adjacent",
                        RENAME => "card.rename",
                        _ => "tray.show",
                    }),
                    None => {}
                }
            } else {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    // The Notebook chip is gone (the owner's ruling of 2026-11-03, "remove it"): where it was, under the Index's
    // tabs at the left, there is no control, and a tap there selects nothing and changes no scope or view.
    fn assert_no_notebook_chip(v: &mut MapView, a: &mut App, nav: &Nav) {
        let (x, y) = (u(400.0) as f32, u(215.0) as f32);
        assert!(!matches!(hit(v, x, y), Some(MapHitKind::Control(_))), "a control where the Notebook chip was");
        let (scope, mode, id, list) = (v.scope, v.mode, v.id, v.list);
        let down = InEvent { type_: InType::TouchDown, x, y, t_ms: 100, ..Default::default() };
        let up = InEvent { type_: InType::TouchUp, x, y, t_ms: 150, ..Default::default() };
        mapview_pointer(v, a, nav, 0, &down);
        assert_eq!(mapview_pointer(v, a, nav, 2, &up), MapResult::None);
        assert_eq!((v.scope, v.mode, v.id, v.list), (scope, mode, id, list));
        let mut out = String::new();
        assert!(!mapview_probe(v, "menu", &mut out) && !mapview_probe(v, "menu.rows", &mut out));
        for name in ["scope", "scope.all", "scope.this", "scope.other"] {
            assert!(mapview_control_point(v, name).is_none(), "{name}");
        }
    }
    fn view(w: i32, h: i32, n: u32, linked: bool) -> (MapView, Nav) {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let mut nb = NotebookInfo::default();
        nb.page_ids = (1..=n).collect();
        let mut nav = Nav::default();
        nav_init(&mut nav);
        if linked {
            nav_set_name(&mut nav, 1, "Start");
            nav_set_name(&mut nav, 2, "Destination");
            nav_link_set(&mut nav, 1, 5, "p2");
        }
        let mut v = MapView {
            open: true,
            w,
            h,
            selected: -1,
            centre: -1,
            ..Default::default()
        };
        v.pages = nb.page_ids.clone();
        assert_eq!(graph_build(&mut v.g, &nb, &nav, if linked { 1 } else { 0 }, true), 0);
        assert!(graph_layout(&mut v.g, &mut || {}));
        assert!(reserve(&mut v.hits, n as usize + 32));
        assert!(reserve(&mut v.labels, n as usize));
        default_fit(&mut v);
        (v, nav)
    }
    #[test]
    fn fitted_nodes_and_labels_stay_in_the_viewport() {
        for (w, h) in [(1860, 2480), (2480, 1860)] {
            let (mut v, nav) = view(w, h, 4, true);
            v.selected = 1;
            default_fit(&mut v);
            let mut c = Canvas::default();
            canvas_init(&mut c, w, h);
            render(&mut v, &mut c, &nav);
            assert!((0.25..=6.0).contains(&v.zoom));
            assert!(v.labels.len() >= 2);
            for (i, &a) in v.labels.iter().enumerate() {
                assert_eq!(rect_intersect(a, plot(&v)), a);
                assert!(!v.labels[i + 1..].iter().any(|&b| rect_overlaps(a, b)));
            }
            for &i in &v.g.force {
                assert!(rect_overlaps(visible_rect(&v, i as usize), plot(&v)));
            }
        }
    }
    #[test]
    fn an_open_selected_dot_keeps_both_its_pattern_and_rings() {
        let (mut v, nav) = view(1860, 2480, 2, true);
        v.selected = 0;
        let mut c = Canvas::default();
        canvas_init(&mut c, v.w, v.h);
        render(&mut v, &mut c, &nav);
        let n = v.g.nodes[0];
        let (x, y) = screen_point(&v, (n.x, n.y));
        let r = n.r * v.zoom;
        let pixel = |xx: f32| c.px[y.round() as usize * c.stride as usize + xx.round() as usize];
        // Hollowing an outer ring used to erase the inner one and all of the dither.
        assert_eq!(pixel(x + r - 1.0), 0);
        assert_eq!(pixel(x + r + 5.0), 0);
        let inside = Rect { x: x.round() as i32 - 10, y: y.round() as i32 - 10, w: 20, h: 20 };
        let mut black = 0;
        for yy in inside.y..inside.y + inside.h {
            for xx in inside.x..inside.x + inside.w {
                black += (c.px[(yy * c.stride + xx) as usize] == 0) as i32;
            }
        }
        assert!(black > 0 && black < 400);
    }
    #[test]
    fn pan_and_pinch_preserve_the_world_anchor_and_settle_on_release() {
        let (mut v,nav)=view(1860,2480,4,true);
        let mut a=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut a.screen,a.w,a.h);
        mapview_draw(&mut v,&mut a,&nav,Rect::new(0,0,1860,2480));
        let (z,ox,oy)=(v.zoom,v.ox,v.oy);
        mapview_pointer(&mut v,&mut a,&nav,0,&InEvent{x:900.0,y:1000.0,t_ms:0,..Default::default()});
        mapview_pointer(&mut v,&mut a,&nav,1,&InEvent{x:1100.0,y:1100.0,t_ms:200,..Default::default()});
        assert!((v.ox-(ox-200.0/z)).abs()<0.001);assert!((v.oy-(oy-100.0/z)).abs()<0.001);
        mapview_pointer(&mut v,&mut a,&nav,2,&InEvent{x:1100.0,y:1100.0,t_ms:240,..Default::default()});
        assert!(v.drag.is_none());
        v.zoom=1.0;v.ox=0.0;v.oy=0.0;
        mapview_pinch(&mut v,&mut a,&InEvent{type_:InType::PinchBegin,x:800.0,y:1000.0,x2:1000.0,y2:1000.0,t_ms:300,..Default::default()});
        mapview_pinch(&mut v,&mut a,&InEvent{type_:InType::PinchMove,x:700.0,y:1000.0,x2:1100.0,y2:1000.0,t_ms:500,..Default::default()});
        assert_eq!(v.zoom,2.0);assert!((v.ox+900.0/v.zoom-900.0).abs()<0.001);assert!((v.oy+1000.0/v.zoom-1000.0).abs()<0.001);
        mapview_pinch(&mut v,&mut a,&InEvent{type_:InType::PinchEnd,t_ms:540,..Default::default()});
        assert!(v.drag.is_none()&&v.needs_redraw);
    }
    #[test]
    fn a_pen_tap_that_slides_a_little_still_selects_its_dot() {
        let (mut v,nav)=view(1860,2480,4,true);
        let mut a=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut a.screen,a.w,a.h);
        mapview_draw(&mut v,&mut a,&nav,Rect::new(0,0,1860,2480));
        let i=(0..v.g.nodes.len()).find(|&i| v.g.nodes[i].linked && i as i32!=v.selected).unwrap();
        let n=v.g.nodes[i];let (x,y)=screen_point(&v,(n.x,n.y));
        // ~2 mm of slide, as a pen tip does on a tap: still the dot's tap, not a pan.
        mapview_pointer(&mut v,&mut a,&nav,0,&InEvent{x,y,t_ms:0,..Default::default()});
        mapview_pointer(&mut v,&mut a,&nav,1,&InEvent{x:x+18.0,y:y+12.0,t_ms:60,..Default::default()});
        mapview_pointer(&mut v,&mut a,&nav,2,&InEvent{x:x+18.0,y:y+12.0,t_ms:90,..Default::default()});
        assert_eq!(v.selected,i as i32);
        assert!(!v.view_touched);
    }
    #[test]
    fn background_completion_belongs_only_to_its_notebook() {
        let mut v=MapView{open:true,layout:MapLayout::Running,..Default::default()};v.id.set("B");
        mapview_background_done(&mut v,"A",true);assert!(!v.job_done);assert_eq!(v.layout,MapLayout::Running);
        mapview_background_done(&mut v,"A",false);assert_eq!(v.layout,MapLayout::Running);
        mapview_background_done(&mut v,"B",true);assert!(v.job_done);
    }
    #[test]
    fn failed_open_and_pending_seed_never_replace_a_finished_cache() {
        let root=std::env::temp_dir().join(format!("grimoire-map-cache-{}",std::process::id()));
        store_init(root.to_str().unwrap());let mut nb=NotebookInfo::default();assert_eq!(store_create("Map","",PAPER_BLANK,&mut nb),0);
        let mut path=String::new();assert_eq!(graph_ini_path(nb.id.as_str(),&mut path),0);
        let original=b"# Grimoire graph\ngen=12345678\npos.1=1234,876\npin.1=1\nfuture=kept\n";
        assert_eq!(write_file_atomic(&path,original),0);
        let mut failed=MapView{id:nb.id,..Default::default()};mapview_close(&mut failed);
        assert_eq!(read_file(&path).unwrap(),original);
        let (mut v,_)=view(1860,2480,2,true);v.id=nb.id;v.layout=MapLayout::Running;
        let mut before=GraphIni::default();assert_eq!(graph_ini_read(nb.id.as_str(),&mut before),0);
        v.mode=GraphMode::Around;v.ini.show_unlinked=false;save(&mut v);
        let mut after=GraphIni::default();assert_eq!(graph_ini_read(nb.id.as_str(),&mut after),0);
        assert_eq!(after.pos,before.pos);assert_eq!(after.pins,before.pins);assert_eq!(after.unknown,before.unknown);
        assert_eq!(after.mode,GraphMode::Around);assert!(!after.show_unlinked);
        mapview_close(&mut v);graph_ini_read(nb.id.as_str(),&mut after);assert_eq!(after.pos,before.pos);
        std::fs::remove_file(&path).unwrap();
        let (mut seed,_)=view(1860,2480,2,true);seed.id=nb.id;seed.layout=MapLayout::Running;save(&mut seed);
        graph_ini_read(nb.id.as_str(),&mut after);assert!(after.pos.is_empty()&&!after.has_gen);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cached_map_layout_survives_close_reopen_completion_and_close() {
        let root=std::env::temp_dir().join(format!("grimoire-map-reopen-{}",std::process::id()));
        store_init(root.to_str().unwrap());let mut nb=NotebookInfo::default();assert_eq!(store_create("Map","",PAPER_BLANK,&mut nb),0);
        nb.page_ids=(1..=400).collect();nb.next_page_id=401;assert_eq!(store_save_info(&mut nb),0);
        let mut nav=Nav::default();nav_init(&mut nav);for pid in 1..400{assert_eq!(nav_link_set(&mut nav,pid,1,&format!("p{}",pid+1)),0);}
        assert_eq!(nav_save_links(nb.id.as_str(),&mut nav,&nb),0);
        let mut a=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut a.screen,a.w,a.h);
        let mut v=MapView{id:nb.id,open:true,w:a.w,h:a.h,..Default::default()};v.pages=nb.page_ids.clone();
        assert_eq!(graph_build(&mut v.g,&nb,&nav,1,true),0);assert_eq!(v.g.force.len(),400);
        assert!(layout(&mut v,&mut a));assert_eq!(v.layout,MapLayout::Running);assert_eq!(v.g.laid,GraphLaid::Seed);
        mapview_close(&mut v);
        assert!(mapview_open(&mut v,&mut a,&nb,&nav,1,false));assert_eq!(v.layout,MapLayout::Running);
        let until=now_ms().wrapping_add(10000);
        while bg_busy() && (now_ms().wrapping_sub(until) as i32)<0 {
            let mut kind=-1;let mut id=FixedStr::new();let result=bg_poll_job(Some(&mut kind),Some(&mut id));
            if kind==BgKind::GraphLayout as i32 {mapview_background_done(&mut v,id.as_str(),result>0);}
            sleep_ms(2);
        }
        assert!(!bg_busy());assert!(v.job_done);
        mapview_tick(&mut v,&mut a,now_ms().wrapping_add(600));assert_eq!(v.layout,MapLayout::Done);
        let mut before=GraphIni::default();graph_ini_read(nb.id.as_str(),&mut before);assert_eq!(before.pos.len(),400);
        mapview_close(&mut v);let mut after=GraphIni::default();graph_ini_read(nb.id.as_str(),&mut after);
        assert_eq!(after.pos,before.pos);assert_eq!(after.gen,before.gen);
        std::fs::remove_dir_all(root).unwrap();
    }
    // The Graph of all notebooks' layout child: its completion is All's (a notebook's own layout child, whatever
    // its id string, never ends it), and its positions are adopted as kept positions, so a model that changed
    // while it ran (a notebook read meanwhile) has the difference placed beside them.
    #[test]
    fn all_completion_is_alls_and_a_changed_model_is_placed_incrementally() {
        let mut v=MapView{open:true,scope:MapScope::All,layout:MapLayout::Running,..Default::default()};
        mapview_background_done(&mut v,"",true);assert!(!v.job_done);
        let same=v.id.as_str().to_string();mapview_background_done(&mut v,&same,true);assert!(!v.job_done);
        mapview_all_background_done(&mut v,true);assert!(v.job_done);
        let mut one=MapView{open:true,layout:MapLayout::Running,..Default::default()};
        mapview_all_background_done(&mut one,true);assert!(!one.job_done);
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-all-job-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        mapview_forget_all_ini();
        let mk=|name:&str,n:u32,chain:bool|{let mut nb=NotebookInfo::default();assert_eq!(store_create(name,"",PAPER_BLANK,&mut nb),0);
            nb.page_ids=(1..=n).collect();nb.next_page_id=n+1;assert_eq!(store_save_info(&mut nb),0);
            let mut nav=Nav::default();nav_init(&mut nav);
            if chain {for pid in 1..n{assert_eq!(nav_link_set(&mut nav,pid,1,&format!("p{}",1+(pid*7+3)%n)),0);assert_eq!(nav_link_set(&mut nav,pid,2,&format!("p{}",pid+1)),0);}assert_eq!(nav_save_links(nb.id.as_str(),&mut nav,&nb),0);}
            (nb,nav)};
        // Two notebooks of 200 linked pages: 400, past what lays out in the app's process.
        let (a,anav)=mk("A",200,true);let _b=mk("B",200,true);
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        let mut v=MapView::default();
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.scope,MapScope::All);assert_eq!(v.g.nodes.len(),400);assert_eq!(v.g.books.len(),2);
        assert_eq!(v.layout,MapLayout::Running);
        assert!(bg_pending(BgKind::GraphLayoutAll,""));
        // A third notebook comes while the child runs (its pages chained: a new component).
        let _c=mk("C",30,true);
        let until=now_ms().wrapping_add(20000);
        while bg_busy() && (now_ms().wrapping_sub(until) as i32)<0 {
            let mut kind=-1;let result=bg_poll_job(Some(&mut kind),None);
            if kind==BgKind::GraphLayoutAll as i32 {mapview_all_background_done(&mut v,result>0);}
            if kind==BgKind::GraphLayout as i32 {mapview_background_done(&mut v,"",result>0);}
            sleep_ms(2);
        }
        assert!(v.job_done);
        mapview_tick(&mut v,&mut app,now_ms().wrapping_add(600));
        assert_eq!(v.layout,MapLayout::Done);
        let kept: Vec<(FixedStr<STORE_ID_LEN>,u32,u32,u32)> = v.g.nodes.iter().map(|n|(v.g.books[n.book as usize],n.pid,n.x.to_bits(),n.y.to_bits())).collect();
        assert!(v.g.nodes.iter().all(|n|n.placed));
        // The index learns of C: the model is built again, C's pages placed as a block, no kept dot moving.
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        assert!(mapview_library_refresh(&mut v,&mut app,&a,&anav,now_ms().wrapping_add(5000)));
        assert_eq!(v.g.books.len(),3);assert_eq!(v.g.nodes.len(),430);
        assert!(v.g.nodes.iter().all(|n|n.placed));
        for (id,pid,x,y) in kept {
            let i=node_spec(&v,&format!("{}:{pid}",id.as_str()));assert!(i>=0);let i=i as usize;
            assert_eq!((v.g.nodes[i].x.to_bits(),v.g.nodes[i].y.to_bits()),(x,y));
        }
        assert_eq!(v.layout,MapLayout::Done,"a new notebook is placed as a block: the library is not laid out again");
        mapview_close(&mut v);
        let mut back=GraphAllIni::default();
        assert_eq!(graph_all_ini_read_path(&graph_all_ini_path(),&mut back),0);
        assert_eq!(back.pos.len(),430);
        mapview_forget_all_ini();
        std::fs::remove_dir_all(root).unwrap();
    }
    // All's layout child that ends while the Graph is closed, or shows one notebook, wrote graph-all.ini: the next
    // open of All reads it (not the copy kept from before the child ran), so it shows the child's layout and lays
    // nothing out again.
    #[test]
    fn an_all_child_that_ends_while_all_is_not_shown_is_read_at_the_next_open() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-all-closed-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        mapview_forget_all_ini();
        let mk=|name:&str,n:u32|{let mut nb=NotebookInfo::default();assert_eq!(store_create(name,"",PAPER_BLANK,&mut nb),0);
            nb.page_ids=(1..=n).collect();nb.next_page_id=n+1;assert_eq!(store_save_info(&mut nb),0);
            let mut nav=Nav::default();nav_init(&mut nav);
            for pid in 1..n{assert_eq!(nav_link_set(&mut nav,pid,1,&format!("p{}",1+(pid*7+3)%n)),0);assert_eq!(nav_link_set(&mut nav,pid,2,&format!("p{}",pid+1)),0);}
            assert_eq!(nav_save_links(nb.id.as_str(),&mut nav,&nb),0);
            (nb,nav)};
        // Two notebooks of 200 linked pages: 400, past what lays out in the app's process.
        let (a,anav)=mk("A",200);let _b=mk("B",200);
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        // (the child run to its end, its completion handed to the view as the app does)
        let finish=|v:&mut MapView|{
            let until=now_ms().wrapping_add(20000);
            while bg_busy() && (now_ms().wrapping_sub(until) as i32)<0 {
                let mut kind=-1;let result=bg_poll_job(Some(&mut kind),None);
                if kind==BgKind::GraphLayoutAll as i32 {mapview_all_background_done(v,result>0);}
                sleep_ms(2);
            }
            assert!(!bg_busy());
            let mut back=GraphAllIni::default();
            assert_eq!(graph_all_ini_read_path(&graph_all_ini_path(),&mut back),0);
            assert_eq!(back.pos.len(),400);
        };
        let mut v=MapView::default();
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.layout,MapLayout::Running);assert!(bg_pending(BgKind::GraphLayoutAll,""));
        // Closed while the child runs; it ends; All opens on its layout.
        mapview_close(&mut v);
        finish(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.all_ini.pos.len(),400);
        assert_eq!(v.layout,MapLayout::Done);assert!(!bg_pending(BgKind::GraphLayoutAll,""));
        mapview_close(&mut v);
        // The same with one notebook shown while the child runs (a notebook picked past the caps), then All again.
        std::fs::remove_file(graph_all_ini_path()).unwrap();
        mapview_forget_all_ini();
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.layout,MapLayout::Running);
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,Some(a.id.as_str())));
        assert_eq!(v.scope,MapScope::One);
        finish(&mut v);
        // (this notebook's own layout child, queued by its view, ended too)
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.scope,MapScope::All);assert_eq!(v.all_ini.pos.len(),400);
        assert_eq!(v.layout,MapLayout::Done);assert!(!bg_pending(BgKind::GraphLayoutAll,""));
        mapview_close(&mut v);
        while bg_busy() {let mut k=-1;bg_poll_job(Some(&mut k),None);sleep_ms(2);}
        mapview_forget_all_ini();
        std::fs::remove_dir_all(root).unwrap();
    }
    // A broken link's row in the List repairs it (its card) in every scope: the Graph opens on All notebooks, which
    // has no other-notebook boxes to say which links are broken, and in another notebook's view the boxes are
    // that notebook's. A live link's row follows it.
    #[test]
    fn a_broken_rows_tap_in_the_list_repairs_it_in_every_scope() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-all-list-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        mapview_forget_all_ini();
        let mk=|name:&str,n:u32|{let mut nb=NotebookInfo::default();assert_eq!(store_create(name,"",PAPER_BLANK,&mut nb),0);
            nb.page_ids=(1..=n).collect();nb.next_page_id=n+1;assert_eq!(store_save_info(&mut nb),0);nb};
        let (a,b)=(mk("A",3),mk("B",3));
        let mut gone=NotebookInfo::default();assert_eq!(store_create("Gone","",PAPER_BLANK,&mut gone),0);
        let mut anav=Nav::default();nav_init(&mut anav);
        let live=nav_notebook_target(b.id.as_str(),2).unwrap();
        assert_eq!(nav_link_set(&mut anav,1,7,&live),0);
        assert_eq!(nav_link_set(&mut anav,2,8,&nav_notebook_target(b.id.as_str(),99).unwrap()),0);
        assert_eq!(nav_link_set(&mut anav,3,9,&nav_notebook_target(gone.id.as_str(),1).unwrap()),0);
        assert_eq!(nav_save_links(a.id.as_str(),&mut anav,&a),0);
        // (the notebook the third link goes to is deleted)
        let mut dir=String::new();assert_eq!(store_notebook_dir(gone.id.as_str(),&mut dir,600),0);std::fs::remove_dir_all(&dir).unwrap();
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        let tap=|v:&mut MapView,app:&mut App,name:&str|{
            let (x,y)=mapview_control_point(v,name).unwrap_or_else(||panic!("no {name}"));
            let down=InEvent{type_:InType::TouchDown,x,y,t_ms:100,..Default::default()};
            let up=InEvent{type_:InType::TouchUp,x,y,t_ms:150,..Default::default()};
            mapview_pointer(v,app,&anav,0,&down);
            mapview_pointer(v,app,&anav,2,&up)
        };
        let mut v=MapView::default();
        for pick in [None,Some(b.id.as_str()),Some(a.id.as_str())] {
            assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,pick));
            assert_eq!(tap(&mut v,&mut app,"list"),MapResult::None);assert!(v.list);
            // (rows by the notebook they go to, then by page: B's page 2 from page 1, B's gone page from page 2,
            // then the deleted notebook's)
            let rows:Vec<u32>=v.list_rows.iter().map(|l|l.pid).collect();
            let want:Vec<(u32,MapResult)>=vec![(1,MapResult::Target(FixedStr::from_str(&live))),(2,MapResult::Repair(2,8)),(3,MapResult::Repair(3,9))];
            for (pid,result) in want {
                let k=rows.iter().position(|&p|p==pid).unwrap();
                assert_eq!(tap(&mut v,&mut app,&format!("link.{}",k+1)),result,"{pick:?} page {pid}");
            }
            mapview_close(&mut v);
        }
        mapview_forget_all_ini();
        std::fs::remove_dir_all(root).unwrap();
    }
    // Past the Graph of all notebooks' caps (here 10000 linked pages) the Graph says so and opens this notebook.
    #[test]
    fn past_the_caps_it_says_so_and_opens_this_notebook() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-all-caps-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        mapview_forget_all_ini();
        for k in 0..5 {
            let dir=root.join(format!("notebooks/cap{k}"));std::fs::create_dir_all(&dir).unwrap();
            let pages=(1..=2000).map(|p|p.to_string()).collect::<Vec<_>>().join(",");
            std::fs::write(dir.join("notebook.ini"),format!("name=Cap {k}\nfolder=\npaper=4\ncreated=1\nmodified=1\nlast_page=0\nnext_page_id=2001\npage_w=1860\npage_h=2480\npages={pages}\n")).unwrap();
            let links:String=(1..2000).map(|p|format!("link.{p}=00000001,p{}\n",p+1)).collect();
            std::fs::write(dir.join("links.ini"),links).unwrap();
        }
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        let mut nb=NotebookInfo::default();assert_eq!(store_load_info("cap0",&mut nb),0);
        let mut nav=Nav::default();nav_init(&mut nav);nav_load("cap0",&mut nav);
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        let mut v=MapView::default();
        assert!(mapview_open_scoped(&mut v,&mut app,&nb,&nav,1,false,None));
        assert_eq!(v.scope,MapScope::One);assert_eq!(v.id.as_str(),"cap0");
        assert_eq!(app.toast.as_str(),"Too many linked pages: showing this notebook only.");
        mapview_close(&mut v);
        while bg_busy() {let mut k=-1;bg_poll_job(Some(&mut k),None);sleep_ms(2);}
        mapview_forget_all_ini();
        std::fs::remove_dir_all(root).unwrap();
    }
    // A node of another notebook opens there (a target, so Back returns here); Rename is the editor's notebook's
    // alone; there is no Notebook choice (the owner's ruling of 2026-11-03: "remove it"), and a notebook shown
    // alone (past the caps) behaves as its own Graph.
    #[test]
    fn another_notebooks_node_opens_there_and_there_is_no_notebook_choice() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-all-taps-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        mapview_forget_all_ini();
        let mk=|name:&str,n:u32|{let mut nb=NotebookInfo::default();assert_eq!(store_create(name,"",PAPER_BLANK,&mut nb),0);
            nb.page_ids=(1..=n).collect();nb.next_page_id=n+1;assert_eq!(store_save_info(&mut nb),0);nb};
        let (a,b)=(mk("A",3),mk("B",3));
        let mut anav=Nav::default();nav_init(&mut anav);
        assert_eq!(nav_link_set(&mut anav,1,1,&nav_notebook_target(b.id.as_str(),2).unwrap()),0);
        assert_eq!(nav_save_links(a.id.as_str(),&mut anav,&a),0);
        let mut bnav=Nav::default();nav_init(&mut bnav);
        assert_eq!(nav_link_set(&mut bnav,2,1,"p3"),0);
        assert_eq!(nav_save_links(b.id.as_str(),&mut bnav,&b),0);
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        // An unsaved link of the editor's notebook is in All at once (its live Nav): A's page 2 to its page 3.
        assert_eq!(nav_link_set(&mut anav,2,2,"p3"),0);
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        let mut v=MapView::default();
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.g.nodes.len(),5);
        let a2=node_spec(&v,"2");let a3=node_spec(&v,"3");assert!(a2>=0&&a3>=0&&graph_edge_between(&v.g,a2,a3)>=0);
        let b2=node_spec(&v,&format!("{}:2",b.id.as_str()));assert!(b2>=0);
        assert_eq!(open_result(&v,b2 as usize),MapResult::Target(FixedStr::from_str(&nav_notebook_target(b.id.as_str(),2).unwrap())));
        assert_eq!(open_result(&v,a2 as usize),MapResult::Open(2));
        assert!(!editor_page(&v,b2 as usize)&&editor_page(&v,a2 as usize));
        // Where the Notebook chip was (under the tabs, at the left): no control, and a tap there changes nothing.
        assert_no_notebook_chip(&mut v,&mut app,&anav);
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,Some(a.id.as_str())));
        assert_eq!(v.scope,MapScope::One);assert_eq!(mapview_pick(&v),Some(a.id));
        // Another notebook's view: its dots open it there, Rename is off.
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,Some(b.id.as_str())));
        assert_eq!(v.scope,MapScope::One);assert!(v.other.is_some());
        let b2=node_spec(&v,"2");assert!(b2>=0);
        assert!(matches!(open_result(&v,b2 as usize),MapResult::Target(_)));
        assert!(!editor_page(&v,b2 as usize));
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&a,&anav,1,false,None));
        assert_eq!(v.scope,MapScope::All);assert_eq!(mapview_pick(&v),None);
        mapview_close(&mut v);
        mapview_forget_all_ini();
        std::fs::remove_dir_all(root).unwrap();
    }
    // The Graph opened from the Library (2.0.0-beta.2): no notebook open (an empty NotebookInfo and Nav). Every
    // node is another notebook's (Open is a target), nothing names, renames or repairs a page, there is no List,
    // Pages or Notebook choice to tap, a picked notebook that does not read leaves every notebook (never a view of
    // the empty notebook), the panel counts the library, and its view comes back as it was kept.
    #[test]
    fn with_no_notebook_open_the_graph_is_every_notebooks_and_names_none() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-no-notebook-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        mapview_forget_all_ini();
        let mk=|name:&str,n:u32|{let mut nb=NotebookInfo::default();assert_eq!(store_create(name,"",PAPER_BLANK,&mut nb),0);
            nb.page_ids=(1..=n).collect();nb.next_page_id=n+1;assert_eq!(store_save_info(&mut nb),0);nb};
        let (a,b,c)=(mk("A",3),mk("B",3),mk("C",2));
        let save=|nb:&NotebookInfo,links:&[(u32,String)]|{let mut nav=Nav::default();nav_init(&mut nav);
            for (k,(pid,t)) in links.iter().enumerate() {assert_eq!(nav_link_set(&mut nav,*pid,k as u32+1,t),0);}
            assert_eq!(nav_save_links(nb.id.as_str(),&mut nav,nb),0);};
        save(&a,&[(1,nav_notebook_target(b.id.as_str(),2).unwrap())]);
        save(&b,&[(2,"p3".into()),(1,nav_notebook_target(c.id.as_str(),9).unwrap())]);
        // C is left out of the Graph, and links to A's page 3: neither is drawn.
        save(&c,&[(1,nav_notebook_target(a.id.as_str(),3).unwrap())]);
        assert_eq!(store_set_graph_omit(c.id.as_str(),true),0);
        let mut n=0;while libindex_step() {n+=1;assert!(n<10_000);}
        let (none,nonav)=(NotebookInfo::default(),Nav::default());
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        let mut v=MapView::default();
        assert!(!mapview_open(&mut v,&mut app,&none,&nonav,0,false),"no notebook: no notebook's own Graph");
        assert!(mapview_open_scoped(&mut v,&mut app,&none,&nonav,0,false,None));
        assert!(mapview_no_notebook(&v));
        assert_eq!(v.scope,MapScope::All);
        let mut ids:Vec<String>=(0..v.g.nodes.len()).map(|i|format!("{}:{}",node_notebook(&v,i).as_str(),v.g.nodes[i].pid)).collect();ids.sort();
        let want={let mut w=vec![format!("{}:1",a.id.as_str()),format!("{}:2",b.id.as_str()),format!("{}:3",b.id.as_str())];w.sort();w};
        assert_eq!(ids,want,"C and A's page 3 (linked only from C) are not drawn");
        assert!(v.g.nodes.iter().all(|n|!n.open),"no page is here");
        for i in 0..v.g.nodes.len() {
            assert!(!editor_page(&v,i));
            assert!(matches!(open_result(&v,i),MapResult::Target(_)));
        }
        assert_eq!(v.page_totals,Some((6,2)),"the library's pages, C left out");
        let mut out=String::new();
        assert!(mapview_probe(&v,"tabs",&mut out));assert_eq!(out,"graph");
        let tap=|v:&mut MapView,app:&mut App,x:f32,y:f32|{
            let down=InEvent{type_:InType::TouchDown,x,y,t_ms:100,..Default::default()};
            let up=InEvent{type_:InType::TouchUp,x,y,t_ms:150,..Default::default()};
            mapview_pointer(v,app,&Nav::default(),0,&down);
            mapview_pointer(v,app,&Nav::default(),2,&up)
        };
        let at=|v:&MapView,name:&str|mapview_control_point(v,name);
        assert!(at(&v,"list").is_none()&&at(&v,"pages").is_none(),"no List or Pages tab");
        // Where List and Pages would be (index_navigation's second and third tabs): nothing.
        let mut c=Canvas::default();canvas_init(&mut c,app.w,app.h);
        let tabs=index_navigation(&mut c,app.w,0);
        for t in [tabs[2],tabs[3]] {
            assert_eq!(tap(&mut v,&mut app,(t.x+t.w/2) as f32,(t.y+t.h/2) as f32),MapResult::None);
            assert!(!v.list);
        }
        // No Notebook chip.
        assert_no_notebook_chip(&mut v,&mut app,&nonav);
        // A node selected: Rename is off (nothing returns Rename), Open is the target.
        let b2=node_spec(&v,&format!("{}:2",b.id.as_str()));assert!(b2>=0);
        let (x,y)=mapview_node_point(&v,&format!("{}:2",b.id.as_str())).unwrap();
        assert_eq!(tap(&mut v,&mut app,x,y),MapResult::None);assert_eq!(v.selected,b2);
        assert!(at(&v,"rename").is_none(),"Rename is greyed out: no hit");
        let (x,y)=at(&v,"open").unwrap();
        assert_eq!(tap(&mut v,&mut app,x,y),MapResult::Target(FixedStr::from_str(&nav_notebook_target(b.id.as_str(),2).unwrap())));
        // The view kept and given back: the page selected again.
        let keep=mapview_keep(&v);
        assert_eq!(keep.pick,None);
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&none,&nonav,0,false,keep.pick.as_ref().map(|p|p.as_str())));
        assert_eq!(v.selected,-1);
        mapview_restore(&mut v,&mut app,&nonav,&keep);
        assert_eq!(v.selected,node_spec(&v,&format!("{}:2",b.id.as_str())));
        // Ticks and the index's refresh, with no notebook: nothing assumes one.
        mapview_tick(&mut v,&mut app,now_ms().wrapping_add(1000));
        assert!(!mapview_anchors_refresh(&mut v,&none,&nonav),"All has no anchors");
        assert!(mapview_rebuild(&mut v,&mut app,&none,&nonav,0));
        assert_eq!(v.g.nodes.len(),3);
        // Another notebook picked (the browser past the caps): B's own Graph; its broken anchor (to C's missing
        // page 9) repairs nothing here.
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&none,&nonav,0,false,Some(b.id.as_str())));
        assert_eq!(v.scope,MapScope::One);assert!(v.other.is_some());
        assert!(v.g.nodes.iter().enumerate().all(|(i,_)|!editor_page(&v,i)));
        assert!(!mapview_rebuild(&mut v,&mut app,&none,&nonav,0),"another notebook's view is not rebuilt from the editor's");
        mapview_anchors_refresh(&mut v,&none,&nonav);
        if let Some(k)=v.foreign.iter().position(|f|f.broken) {
            let (x,y)=at(&v,&format!("anchor.{}",k+1)).unwrap();
            assert_eq!(tap(&mut v,&mut app,x,y),MapResult::None);
        }
        assert_eq!(mapview_pick(&v),Some(b.id));
        // A pick that does not read: every notebook, never the empty notebook's view.
        mapview_close(&mut v);
        assert!(mapview_open_scoped(&mut v,&mut app,&none,&nonav,0,false,Some("gone")));
        assert_eq!(v.scope,MapScope::All);assert!(v.id.is_empty());
        mapview_close(&mut v);
        while bg_busy() {let mut k=-1;bg_poll_job(Some(&mut k),None);sleep_ms(2);}
        mapview_forget_all_ini();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn anchors_never_move_a_dot_and_sit_beside_the_plot() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let root=std::env::temp_dir().join(format!("grimoire-map-anchors-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        let mk=|name:&str,n:u32|{let mut nb=NotebookInfo::default();assert_eq!(store_create(name,"",PAPER_BLANK,&mut nb),0);
            nb.page_ids=(1..=n).collect();nb.next_page_id=n+1;assert_eq!(store_save_info(&mut nb),0);nb};
        let (a,b)=(mk("A",6),mk("B",3));
        let mut nav=Nav::default();nav_init(&mut nav);
        for pid in 1..5 {assert_eq!(nav_link_set(&mut nav,pid,1,&format!("p{}",pid+1)),0);}
        assert_eq!(nav_save_links(a.id.as_str(),&mut nav,&a),0);
        let mut app=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut app.screen,app.w,app.h);
        let mut v=MapView::default();
        assert!(mapview_open(&mut v,&mut app,&a,&nav,1,false));
        let dots=|v:&MapView|v.g.nodes.iter().map(|n|(n.pid,n.x.to_bits(),n.y.to_bits(),n.tier)).collect::<Vec<_>>();
        let (before,gen)=(dots(&v),graph_gen(&v.g));
        assert!(v.foreign.is_empty());
        mapview_close(&mut v);
        // B links to A's pages 2 and 6 (6 is linked nowhere in A), A's page 3 links to B's page 1 and to a page
        // of B that is gone.
        let mut bn=Nav::default();nav_init(&mut bn);
        assert_eq!(nav_link_set(&mut bn,1,7,&nav_notebook_target(a.id.as_str(),2).unwrap()),0);
        assert_eq!(nav_link_set(&mut bn,2,8,&nav_notebook_target(a.id.as_str(),6).unwrap()),0);
        assert_eq!(nav_save_links(b.id.as_str(),&mut bn,&b),0);
        assert_eq!(nav_link_set(&mut nav,3,9,&nav_notebook_target(b.id.as_str(),1).unwrap()),0);
        assert_eq!(nav_link_set(&mut nav,3,10,&nav_notebook_target(b.id.as_str(),99).unwrap()),0);
        assert_eq!(nav_save_links(a.id.as_str(),&mut nav,&a),0);
        // (the cache read and its check done, as the idle ticks do)
        let mut n=0;while backlinks_step() {n+=1;assert!(n<1000);}
        assert!(mapview_open(&mut v,&mut app,&a,&nav,1,false));
        // The same dots, in the same places, with the same tiers (page 6 stays hidden) and graph.ini's gen.
        assert_eq!(dots(&v),before);assert_eq!(graph_gen(&v.g),gen);
        // (B's page 1 links to A's page 2 and A's page 3 links to it: one box, both ways)
        assert_eq!(v.foreign.len(),3,"{:?}",v.foreign);
        assert!(v.foreign[0].out && v.foreign[0].inc);
        // Each page's line has its own style: page 3 links out to B's page 1 (solid), page 2 only has a link
        // in from it (dashed).
        assert_eq!(v.foreign[0].here,vec![3,2]);
        assert_eq!(v.foreign[0].here_out,vec![true,false]);
        // B is looked at once for its two targets, and a build again reads nothing: no notebook.ini per target,
        // and B's names are kept between builds (never its links.ini).
        let (infos,books)=(store_info_reads(),backlinks_books_read());
        anchors_build(&mut v,&a);
        assert_eq!((store_info_reads()-infos,backlinks_books_read()-books),(0,0));
        assert_eq!(v.foreign.len(),3);
        let bb=backlinks_book(b.id.as_str()).unwrap();
        assert!(bb.names.links.is_empty() && !bb.names.links_writable);
        // A rename in B (its nav.ini written again) is read once, at the next build.
        let mut names=Nav::default();nav_init(&mut names);nav_load(b.id.as_str(),&mut names);
        nav_set_name(&mut names,1,"Renamed");assert_eq!(nav_save(b.id.as_str(),&mut names,&b),0);
        anchors_build(&mut v,&a);
        assert_eq!(backlinks_books_read()-books,1);
        assert!(v.foreign[0].label.ends_with("Renamed"),"{}",v.foreign[0].label);
        // Every arrow is one the fonts draw (Terminess has no two-way arrow: it would be a box).
        for (o,i) in [(true,true),(true,false),(false,true)] {
            assert!(anchor_arrow(o,i).chars().all(|ch|font_has_glyph(FontFace::Regular,ch as u32)),"{:?}",anchor_arrow(o,i));
        }
        assert_eq!(v.foreign.iter().filter(|f|f.inc).count(),2);
        assert_eq!(v.foreign.iter().filter(|f|f.broken).count(),1);
        assert!(v.foreign.last().unwrap().broken,"broken anchors last in their notebook's group");
        assert_eq!(v.foreign.last().unwrap().first,(3,10));
        // Each anchor's box is right of the plot, never over a dot.
        let pr=plot(&v);
        for h in v.hits.iter().filter(|h|matches!(h.kind,MapHitKind::Anchor(_))) {
            assert!(h.rect.x>=pr.x+pr.w,"{:?} {:?}",h.rect,pr);
        }
        assert_eq!(v.hits.iter().filter(|h|matches!(h.kind,MapHitKind::Anchor(_))).count(),3);
        // A broken anchor's tap repairs its link; a live one follows it.
        let at=|v:&MapView,k:usize|v.hits.iter().find(|h|h.kind==MapHitKind::Anchor(k)).map(|h|h.rect).unwrap();
        for (k,want) in [(2,MapResult::Repair(3,10)),(0,MapResult::Target(v.foreign[0].target))] {
            let r=at(&v,k);let (x,y)=((r.x+r.w/2) as f32,(r.y+r.h/2) as f32);
            let e=InEvent{x,y,type_:InType::TouchDown,..Default::default()};
            mapview_pointer(&mut v,&mut app,&nav,0,&e);
            let e=InEvent{x,y,type_:InType::TouchUp,..Default::default()};
            assert_eq!(mapview_pointer(&mut v,&mut app,&nav,2,&e),want);
        }
        mapview_close(&mut v);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_background_layout_preserves_retry_state() {
        let mut v=MapView{open:true,layout:MapLayout::Running,..Default::default()};
        mapview_background_done(&mut v,"",false);
        assert_eq!(v.layout,MapLayout::Seed);assert!(v.needs_redraw);assert!(!v.job_done);
    }
    #[test]
    fn the_nearest_dot_wins_and_controls_take_their_own_taps() {
        let mut v = MapView::default();
        for (i, x) in [(0, 100.0), (1, 160.0)] {
            v.hits.push(MapHit {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 220,
                    h: 220,
                },
                kind: MapHitKind::Node(i),
                centre: (x, 100.0),
                radius: 59.0,
            });
        }
        assert_eq!(hit(&v, 141.0, 100.0), Some(MapHitKind::Node(1)));
        assert_eq!(hit(&v, 110.0, 100.0), Some(MapHitKind::Node(0)));
        assert_eq!(hit(&v, 240.0, 100.0), None);
        v.hits.push(MapHit {
            rect: Rect {
                x: 100,
                y: 90,
                w: 50,
                h: 30,
            },
            kind: MapHitKind::Control(BACK),
            centre: (0.0, 0.0),
            radius: 0.0,
        });
        assert_eq!(hit(&v, 110.0, 100.0), Some(MapHitKind::Control(BACK)));
    }
    // A notebook of n pages, `linked` of them linked in a chain from page 1, laid out and fitted.
    fn chain_view(n: u32, linked: u32) -> (MapView, Nav, NotebookInfo) {
        let (mut v, mut nav) = view(1860, 2480, n, false);
        for pid in 1..linked {
            assert_eq!(nav_link_set(&mut nav, pid, 1, &format!("p{}", pid + 1)), 0);
        }
        let nb = NotebookInfo { page_ids: (1..=n).collect(), ..Default::default() };
        assert_eq!(graph_build(&mut v.g, &nb, &nav, 0, true), 0);
        assert!(graph_layout(&mut v.g, &mut || {}));
        default_fit(&mut v);
        (v, nav, nb)
    }
    #[test]
    fn view_cycles_all_and_adjacent_and_a_moved_view_still_switches() {
        let (mut v, nav, _) = chain_view(40, 12);
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        canvas_init(&mut a.screen, a.w, a.h);
        let r = full(&v);
        mapview_draw(&mut v, &mut a, &nav, r);
        let press = |v: &mut MapView, a: &mut App| {
            let (x, y) = mapview_control_point(v, "view").unwrap();
            mapview_pointer(v, a, &nav, 0, &InEvent { x, y, t_ms: 0, ..Default::default() });
            mapview_pointer(v, a, &nav, 2, &InEvent { x, y, t_ms: 40, ..Default::default() });
        };
        let mut probe = String::new();
        // All fits every linked dot: each one inside the plot
        let fitted = |v: &MapView| (0..v.g.nodes.len()).filter(|&i| location(v, i).is_some()).all(|i| rect_overlaps(visible_rect(v, i), plot(v)));
        assert!(fitted(&v));
        press(&mut v, &mut a);
        mapview_probe(&v, "view", &mut probe);
        assert_eq!(probe, "adjacent");
        press(&mut v, &mut a);
        mapview_probe(&v, "view", &mut probe);
        assert_eq!(probe, "all");
        assert!(fitted(&v));
        // a pan: the next press switches to Adjacent all the same (no "Fit:" first), fitted
        let (z, ox, oy) = (v.zoom, v.ox, v.oy);
        mapview_pointer(&mut v, &mut a, &nav, 0, &InEvent { x: 900.0, y: 1000.0, t_ms: 100, ..Default::default() });
        mapview_pointer(&mut v, &mut a, &nav, 1, &InEvent { x: 1500.0, y: 1600.0, t_ms: 300, ..Default::default() });
        mapview_pointer(&mut v, &mut a, &nav, 2, &InEvent { x: 1500.0, y: 1600.0, t_ms: 340, ..Default::default() });
        mapview_probe(&v, "view", &mut probe);
        assert_eq!(probe, "all");
        mapview_probe(&v, "touched", &mut probe);
        assert_eq!(probe, "1");
        press(&mut v, &mut a);
        mapview_probe(&v, "view", &mut probe);
        assert_eq!(probe, "adjacent");
        let around_view = (v.zoom, v.ox, v.oy);
        // a pinch in Adjacent: the next press goes to All, fitted as it was first
        mapview_pinch(&mut v, &mut a, &InEvent { type_: InType::PinchBegin, x: 800.0, y: 1000.0, x2: 1000.0, y2: 1000.0, t_ms: 400, ..Default::default() });
        mapview_pinch(&mut v, &mut a, &InEvent { type_: InType::PinchMove, x: 700.0, y: 1000.0, x2: 1100.0, y2: 1000.0, t_ms: 700, ..Default::default() });
        mapview_pinch(&mut v, &mut a, &InEvent { type_: InType::PinchEnd, t_ms: 740, ..Default::default() });
        assert!((v.zoom, v.ox, v.oy) != around_view);
        press(&mut v, &mut a);
        mapview_probe(&v, "view", &mut probe);
        assert_eq!(probe, "all");
        assert_eq!((v.zoom, v.ox, v.oy), (z, ox, oy));
        assert_eq!(v.mode, GraphMode::All);
    }
    #[test]
    fn all_fits_the_whole_map_when_opened_from_an_unlinked_page() {
        // 250 linked pages: the fit is far below the old tiny-dot limit, and the open page is plain
        let (mut v, _, nb) = chain_view(300, 250);
        let nav = { let mut n = Nav::default(); nav_init(&mut n); for pid in 1..250 { nav_link_set(&mut n, pid, 1, &format!("p{}", pid + 1)); } n };
        assert_eq!(graph_build(&mut v.g, &nb, &nav, 290, true), 0);
        assert!(graph_layout(&mut v.g, &mut || {}));
        assert_eq!(v.g.nodes[289].tier, GraphTier::Hidden);
        default_fit(&mut v);
        let drawn: Vec<usize> = (0..v.g.nodes.len()).filter(|&i| location(&v, i).is_some()).collect();
        assert_eq!(drawn.len(), 250);
        let pr = plot(&v);
        for &i in &drawn {
            let (x, y) = screen_point(&v, location(&v, i).unwrap());
            assert!(rect_contains(pr, x as i32, y as i32), "page {} off the plot", i + 1);
        }
    }
    #[test]
    fn a_finished_background_layout_is_fitted_unless_the_view_was_moved() {
        let (mut v, _, _) = chain_view(20, 20);
        let root=std::env::temp_dir().join(format!("grimoire-map-adopt-{}",std::process::id()));
        store_init(root.to_str().unwrap());let mut nb=NotebookInfo::default();assert_eq!(store_create("Map","",PAPER_BLANK,&mut nb),0);
        v.id = nb.id;
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        // the child's layout: the same map, twice as wide
        let mut wide = v.g.clone();
        for n in &mut wide.nodes { n.x *= 2.0; n.y *= 2.0; }
        let mut ini = GraphIni::default();
        graph_ini_store(&mut ini, &wide);
        assert_eq!(graph_ini_write(nb.id.as_str(), &mut ini), 0);
        v.layout = MapLayout::Running;
        v.job_done = true;
        mapview_tick(&mut v, &mut a, 10_000);
        assert_eq!(v.layout, MapLayout::Done);
        let adopted = (v.zoom, v.ox, v.oy);
        default_fit(&mut v);
        assert_eq!(adopted, (v.zoom, v.ox, v.oy));
        // moved: the selected dot stays under the same screen point
        for n in &mut wide.nodes { n.x *= 2.0; n.y *= 2.0; }
        graph_ini_store(&mut ini, &wide);
        assert_eq!(graph_ini_write(nb.id.as_str(), &mut ini), 0);
        v.selected = 7;
        v.view_touched = true;
        v.ox += 50.0;
        let before = screen_point(&v, location(&v, 7).unwrap());
        v.layout = MapLayout::Running;
        v.job_done = true;
        mapview_tick(&mut v, &mut a, 20_000);
        let after = screen_point(&v, location(&v, 7).unwrap());
        assert!((before.0 - after.0).abs() < 0.01 && (before.1 - after.1).abs() < 0.01);
        assert!(v.view_touched);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn unlinked_named_open_and_plain_pages_never_draw_or_hit() {
        for (w,h) in [(1860,2480),(2480,1860)] {
            let (mut v,mut nav)=view(w,h,250,false);
            nav_set_name(&mut nav,13,"Named but unlinked");
            let nb=NotebookInfo{page_ids:(1..=250).collect(),..Default::default()};
            graph_build(&mut v.g,&nb,&nav,14,true);
            graph_layout(&mut v.g,&mut ||{});
            for show in [true,false] {
                v.ini.show_unlinked=show;
                for mode in [GraphMode::All,GraphMode::Around] {
                    v.mode=mode;if mode==GraphMode::Around {around(&mut v,13);}
                    let mut c=Canvas::default();canvas_init(&mut c,w,h);render(&mut v,&mut c,&nav);
                    assert!(!v.hits.iter().any(|h|matches!(h.kind,MapHitKind::Node(_))));
                    assert!(!v.hits.iter().any(|h|matches!(h.kind,MapHitKind::Control(6..=9|13))));
                }
            }
        }
    }

    // ---- the Graph's keys, its selection, View: All / Adjacent ----

    // n pages, each linked to the next (every node drawn), the editor's notebook "nb", on a screen drawn once.
    fn chain(n: u32) -> (MapView, Nav, App) {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let mut nb = NotebookInfo::default();
        nb.page_ids = (1..=n).collect();
        nb.id = FixedStr::from_str("nb");
        let mut nav = Nav::default();
        nav_init(&mut nav);
        nav.writable = true;
        for p in 1..n {
            nav_link_set(&mut nav, p, p, &format!("p{}", p + 1));
        }
        let mut v = MapView { open: true, w: 1860, h: 2480, selected: -1, centre: -1, ..Default::default() };
        v.editor_id = nb.id;
        v.id = nb.id;
        v.pages = nb.page_ids.clone();
        v.list_pages = nb.page_ids.clone();
        v.list_rows = nav.links.clone();
        assert_eq!(graph_build(&mut v.g, &nb, &nav, 1, true), 0);
        assert!(graph_layout(&mut v.g, &mut || {}));
        default_fit(&mut v);
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        canvas_init(&mut a.screen, a.w, a.h);
        mapview_draw(&mut v, &mut a, &nav, Rect::new(0, 0, 1860, 2480));
        (v, nav, a)
    }
    fn node_xy(v: &MapView, i: usize) -> (f32, f32) {
        screen_point(v, location(v, i).unwrap())
    }
    fn probe(v: &MapView, key: &str) -> String {
        let mut out = String::new();
        assert!(mapview_probe(v, key, &mut out), "{key}");
        out
    }

    #[test]
    fn the_arrows_pan_a_sixth_of_the_plot_and_zoom_keys_step_about_its_centre() {
        let (mut v, nav, mut a) = chain(6);
        let pr = plot(&v);
        let (z, ox, oy) = (v.zoom, v.ox, v.oy);
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_LEFT), Some(MapResult::None));
        assert!((v.ox - (ox - pr.w as f32 / 6.0 / z)).abs() < 0.01 && v.oy == oy);
        assert_eq!(probe(&v, "touched"), "1");
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_RIGHT);
        assert!((v.ox - ox).abs() < 0.01);
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_DOWN);
        assert!((v.oy - (oy + pr.h as f32 / 6.0 / z)).abs() < 0.01);
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_UP);
        assert!((v.oy - oy).abs() < 0.01);
        // Zoom in and out by 1.25, the world point at the plot's centre staying there.
        v.zoom = 1.0;
        let (cx, cy) = ((pr.x + pr.w / 2) as f32, (pr.y + pr.h / 2) as f32);
        let world = (cx / v.zoom + v.ox, cy / v.zoom + v.oy);
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_ZOOM_IN);
        assert!((v.zoom - 1.25).abs() < 0.001);
        assert!((cx / v.zoom + v.ox - world.0).abs() < 0.01 && (cy / v.zoom + v.oy - world.1).abs() < 0.01);
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_ZOOM_OUT);
        assert!((v.zoom - 1.0).abs() < 0.001);
        // At the limit nothing changes (and nothing is drawn again).
        v.zoom = ZOOM_MAX;
        a.dirty = RECT_EMPTY;
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_ZOOM_IN), Some(MapResult::None));
        assert_eq!(v.zoom, ZOOM_MAX);
        assert!(rect_is_empty(a.dirty));
        // Fit: the view as it opened.
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_FIT_PAGE);
        assert!((v.zoom - z).abs() < 0.001 && (v.ox - ox).abs() < 0.01 && (v.oy - oy).abs() < 0.01);
        assert_eq!(probe(&v, "touched"), "0");
        // Not the Graph's: its hosts go on with it.
        for other in [SHORTCUT_INDEX_MAP, SHORTCUT_GO_BACK, SHORTCUT_PREV_PAGE, SHORTCUT_UNDO, SHORTCUT_TOOL_BASE] {
            assert_eq!(mapview_action(&mut v, &mut a, &nav, other), None, "{other}");
        }
        // Close menu with no card: the host closes the Graph.
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_CLOSE_MENU), None);
    }

    #[test]
    fn view_reads_all_or_adjacent_and_always_switches_fitted() {
        let (mut v, nav, mut a) = chain(6);
        assert_eq!(probe(&v, "view"), "all");
        // The view button switches even after a pan (no "Fit:" first), and fits.
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_LEFT);
        let (x, y) = mapview_control_point(&v, "view").unwrap();
        mapview_pointer(&mut v, &mut a, &nav, 0, &InEvent { type_: InType::TouchDown, x, y, t_ms: 10, ..Default::default() });
        mapview_pointer(&mut v, &mut a, &nav, 2, &InEvent { type_: InType::TouchUp, x, y, t_ms: 60, ..Default::default() });
        assert_eq!((probe(&v, "view").as_str(), probe(&v, "mode").as_str(), probe(&v, "touched").as_str()), ("adjacent", "around", "0"));
        // V: the other again, fitted.
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_UP);
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_GRAPH_VIEW);
        assert_eq!((probe(&v, "view").as_str(), probe(&v, "touched").as_str()), ("all", "0"));
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_GRAPH_VIEW);
        assert_eq!(probe(&v, "view"), "adjacent");
        // The words: the view's and the card's are the same msgid (Adjacent); Around and Fit: are gone.
        assert_eq!(view_word(GraphMode::Around), "Adjacent");
        assert_eq!(view_word(GraphMode::All), "All");
        let src = include_str!("mapview.rs");
        for gone in ["n_(\"Around\")", "tr(\"Around it\")", "tr(\"Fit: %s\")"] {
            assert!(!src.contains(gone), "{gone}");
        }
        // The docs say it too.
        let docs = [
            ("KINDLE.md", include_str!("../../docs/KINDLE.md")),
            ("DESIGN.md", include_str!("../../docs/DESIGN.md")),
            ("TESTING.md", include_str!("../../docs/TESTING.md")),
        ];
        for (name, text) in docs {
            let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
            for gone in ["choose Around", "opens Around", "and Around (", "reads Fit", "cycles All, Around and Fit;"] {
                assert!(!flat.contains(gone), "{name}: {gone}");
            }
        }
    }

    #[test]
    fn select_all_back_and_enter_in_the_graph() {
        let (mut v, nav, mut a) = chain(5);
        mapview_action(&mut v, &mut a, &nav, SHORTCUT_SELECT_ALL);
        assert_eq!(probe(&v, "selection"), "5");
        assert_eq!(probe(&v, "selection.pages"), "1 2 3 4 5");
        // Enter opens only one selected page.
        assert_eq!(mapview_action(&mut v, &mut a, &nav, GRAPH_KEY_ENTER), Some(MapResult::None));
        // Esc: the selection first, then the Graph.
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_BACK), Some(MapResult::None));
        assert_eq!(probe(&v, "selection"), "0");
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_BACK), Some(MapResult::Close));
        select_only(&mut v, 2);
        assert_eq!(mapview_action(&mut v, &mut a, &nav, GRAPH_KEY_ENTER), Some(MapResult::Open(v.g.nodes[2].pid)));
        // The List tab: Esc closes the Graph; the plot's keys move nothing.
        v.list = true;
        let ox = v.ox;
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_PAN_LEFT), Some(MapResult::None));
        assert_eq!(v.ox, ox);
        assert_eq!(mapview_action(&mut v, &mut a, &nav, SHORTCUT_BACK), Some(MapResult::Close));
    }

    #[test]
    fn a_selection_of_several_survives_a_rebuild_by_page() {
        let (mut v, _nav, _a) = chain(6);
        select_add(&mut v, &[1, 3, 4]);
        assert_eq!(selection_count(&v), 3);
        assert_eq!(v.selected, -1);
        let selected = |v: &MapView| (0..v.g.nodes.len()).filter(|&i| is_selected(v, i)).map(|i| v.g.nodes[i].pid).collect::<Vec<u32>>();
        let pids: Vec<u32> = [1, 3, 4].iter().map(|&i| v.g.nodes[i].pid).collect();
        assert_eq!(selected(&v), pids);
        let keys = picked_keys(&v);
        v.picked.clear();
        picked_find(&mut v, &keys);
        assert_eq!(selected(&v), pids);
    }

    // The eraser end's tap and a paired keyboard's Ctrl change nothing a tap does.
    #[test]
    fn a_pen_with_the_eraser_or_ctrl_taps_as_before() {
        let (mut v, nav, mut a) = chain(5);
        let (x, y) = node_xy(&v, 2);
        for (eraser, mods) in [(true, 0), (false, KEYMOD_CTRL as u32)] {
            selection_clear(&mut v);
            let ev = |t, ty| InEvent { type_: ty, x, y, t_ms: t, eraser, key_mods: mods, ..Default::default() };
            mapview_pointer(&mut v, &mut a, &nav, 0, &ev(1000, InType::PenDown));
            mapview_pointer(&mut v, &mut a, &nav, 2, &ev(1050, InType::PenUp));
            assert_eq!((v.selected, selection_count(&v)), (2, 1));
        }
    }

}

/// Runner taps resolve the node on this host instead of hard-coding layout coordinates.
pub fn mapview_node_point(v: &MapView, spec: &str) -> Option<(f32, f32)> {
    let i = node_spec(v, spec);
    if !v.open || i < 0 {
        return None;
    }
    let i = i as usize;
    v.hits.iter().find_map(|h| match h.kind {
        MapHitKind::Node(n) if n == i => Some(h.centre),
        _ => None,
    })
}

// A node named as the runner and the probes name one: "<page id>" (a page of the notebook shown, the editor's
// in the Graph of all notebooks) or "<notebook id>:<page id>". -1 for none.
fn node_spec(v: &MapView, spec: &str) -> i32 {
    match spec.split_once(':') {
        Some((id, pid)) => {
            let Ok(pid) = pid.parse::<u32>() else { return -1 };
            if v.scope == MapScope::One {
                return if v.id == id { graph_node_of(&v.g, pid) } else { -1 };
            }
            v.g.books.iter().position(|b| *b == id).map_or(-1, |b| graph_node_in(&v.g, b as u16, pid))
        }
        None => {
            let Ok(pid) = spec.parse::<u32>() else { return -1 };
            if v.scope == MapScope::All { graph_node_in(&v.g, editor_book(v), pid) } else { graph_node_of(&v.g, pid) }
        }
    }
}

/// One navigation bar for the three Index views. Back always returns to the notebook.
pub fn index_navigation(c:&mut Canvas,w:i32,selected:usize)->[Rect;4] {
    let u=ui_px;let back=Rect::new(u(20.0),u(20.0),u(150.0),u(150.0));
    let gap=u(20.0);let first=u(200.0);let width=(w-first-u(20.0)-2*gap)/3;
    let mut tabs=[back;4];ui_button(c,back,IconKind::Back,"",ButtonStyle::Plain);
    for (i,text) in [tr("Graph"),tr("List"),tr("Pages")].into_iter().enumerate(){
        tabs[i+1]=Rect::new(first+i as i32*(width+gap),u(20.0),width,u(150.0));
        ui_button(c,tabs[i+1],IconKind::None,text,if i==selected{ButtonStyle::Selected}else{ButtonStyle::Plain});
    }
    canvas_hline(c,0,w,u(190.0),u(3.0),0);tabs
}
/// The Graph's bar when it was opened from the Library (no notebook open): Back, and the Graph tab alone, as
/// wide as index_navigation's (there is no List or Pages to go to).
pub fn graph_navigation(c:&mut Canvas,w:i32)->[Rect;2] {
    let u=ui_px;let back=Rect::new(u(20.0),u(20.0),u(150.0),u(150.0));
    let gap=u(20.0);let first=u(200.0);let width=(w-first-u(20.0)-2*gap)/3;
    ui_button(c,back,IconKind::Back,"",ButtonStyle::Plain);
    let tab=Rect::new(first,u(20.0),width,u(150.0));
    ui_button(c,tab,IconKind::None,tr("Graph"),ButtonStyle::Selected);
    canvas_hline(c,0,w,u(190.0),u(3.0),0);[back,tab]
}
fn render_list(v:&mut MapView,c:&mut Canvas,nav:&Nav) {
    let tabs=index_navigation(c,v.w,1);
    for (i,id) in [BACK,GRAPH,LIST,PAGES].into_iter().enumerate(){v.hits.push(MapHit{rect:tabs[i],kind:MapHitKind::Control(id),centre:(0.0,0.0),radius:0.0});}
    let heading=cformat!(tr("Links from %s"),&v.list_name);
    font_draw_left_name(c,FontFace::Bold,ui_text_size(),Rect::new(u(40.0),u(205.0),v.w-u(80.0),u(65.0)),&heading,0);
    let mut y=u(290.0);let mut group=String::new();
    if v.list_rows.is_empty(){font_draw_left_name(c,FontFace::Regular,ui_text_size(),Rect::new(u(50.0),y,v.w-u(100.0),u(120.0)),tr("No links in this notebook yet."),0);return;}
    for i in v.list_offset..v.list_rows.len(){
        let l=v.list_rows[i];let key=nav_external_target(l.target.as_str()).map_or(String::new(),|(id,_)|id.to_string());
        let title=if key.is_empty(){v.list_name.clone()}else{let mut nb=NotebookInfo::default();if store_load_info(&key,&mut nb)==0{nb.name.as_str().to_string()}else{tr("Notebook unavailable").to_string()}};
        if i==v.list_offset||key!=group {if y+u(210.0)>v.h{break;}canvas_fill(c,Rect::new(u(30.0),y,v.w-u(60.0),u(70.0)),225);font_draw_left_name(c,FontFace::Bold,ui_text_size(),Rect::new(u(50.0),y,v.w-u(100.0),u(70.0)),&title,0);y+=u(85.0);group=key;}
        if y+u(120.0)>v.h{break;}
        let from=v.list_pages.iter().position(|&p|p==l.pid).map_or(0,|n|n+1);
        let destination=v.foreign.iter().find(|f|f.target==l.target).map(|f|if f.label.is_empty(){f.notebook.clone()}else{f.label.clone()}).or_else(||nav_target_label(l.target.as_str())).unwrap_or_else(||if l.target=="index"{tr("The Index").to_string()}else{let pid=l.target.as_str().strip_prefix('p').and_then(|p|p.parse().ok()).unwrap_or(0);let name=nav_name(nav,pid);if name.is_empty(){cformat!(tr("Page %d"),v.list_pages.iter().position(|&p|p==pid).map_or(0,|n|n+1)as i32)}else{name.to_string()}});
        let text=format!("{}  →  {}",cformat!(tr("Page %d"),from as i32),destination);
        let r=Rect::new(u(45.0),y,v.w-u(90.0),u(115.0));font_draw_left_name(c,FontFace::Regular,ui_text_size(),r,&text,0);canvas_hline(c,r.x,r.w,r.y+r.h,u(2.0),180);
        v.hits.push(MapHit{rect:r,kind:MapHitKind::Link(i),centre:(0.0,0.0),radius:0.0});y+=u(130.0);
    }
}
pub fn mapview_set_list(v:&mut MapView,a:&mut App,nav:&Nav) {v.list=true;paint(v,a,nav,RefreshMode::Full);}
pub fn mapview_control_point(v:&MapView,name:&str)->Option<(f32,f32)> {
    if let Some(n)=name.strip_prefix("anchor."){let i=n.parse::<usize>().ok()?.checked_sub(1)?;return v.hits.iter().find(|h|h.kind==MapHitKind::Anchor(i)).map(|h|((h.rect.x+h.rect.w/2)as f32,(h.rect.y+h.rect.h/2)as f32));}
    if let Some(n)=name.strip_prefix("link."){let i=n.parse::<usize>().ok()?.checked_sub(1)?;return v.hits.iter().find(|h|h.kind==MapHitKind::Link(i)).map(|h|((h.rect.x+h.rect.w/2)as f32,(h.rect.y+h.rect.h/2)as f32));}
    let id=match name{"view"=>VIEW,"back"=>BACK,"graph"=>GRAPH,"list"=>LIST,"pages"=>PAGES,"rename"=>RENAME,"open"=>OPEN,
        "around"|"adjacent"=>ADJACENT,_=>return None};
    v.hits.iter().find(|h|h.kind==MapHitKind::Control(id)).map(|h|((h.rect.x+h.rect.w/2) as f32,(h.rect.y+h.rect.h/2) as f32))
}

fn render_foreign(v:&mut MapView,c:&mut Canvas) {
    if v.foreign.is_empty(){return;}
    let pr=plot(v);let x=pr.x+pr.w+u(15.0);let width=u(440.0);let mut y=u(220.0);let mut group=None;
    for i in 0..v.foreign.len(){
        let key=v.foreign[i].key;
        if group!=Some(key) {
            if y+u(210.0)>pr.y+pr.h {break;}
            group=Some(key);
            let header=Rect::new(x,y,width,u(90.0));canvas_fill(c,header,225);ui_frame(c,header,u(8.0),0,false,u(3.0));
            font_draw_left_name(c,FontFace::Bold,u(34.0)as f32,Rect::new(x+u(12.0),y,width-u(24.0),header.h),&v.foreign[i].notebook,0);y+=u(100.0);
        }
        if y+u(95.0)>pr.y+pr.h {
            button(v,c,Rect::new(x,y,width,u(90.0)),LIST,tr("List"),IconKind::None,true);break;
        }
        let f=v.foreign[i].clone();
        let r=Rect::new(x,y,width,u(85.0));
        // An arrow says which way its links go: into this notebook, out of it, or both. A broken one is dashed.
        let arrow=anchor_arrow(f.out,f.inc);
        let mut label=if f.label.is_empty(){f.notebook.clone()}else{f.label.clone()};
        // A page here linked only from other notebooks has no dot (the map shows this notebook's links): the
        // box says which page it is, where no line can.
        let drawn=f.here.iter().any(|&pid|v.g.nodes.iter().position(|n|n.pid==pid).is_some_and(|at|location(v,at).is_some()));
        if !drawn {
            if let Some(n)=f.here.first().and_then(|&pid|v.pages.iter().position(|&p|p==pid)) {
                label=format!("{label}  →  {}",cformat!(c_("bar","p. %d"),n as i32+1));
            }
        }
        if f.broken {canvas_dash_rect(c,r,u(3.0),u(14.0),u(10.0),0);} else {ui_frame(c,r,u(8.0),0,false,u(2.0));}
        let text_r=Rect::new(x+u(10.0),y,width-u(20.0),r.h);
        font_draw_left_name(c,FontFace::Regular,u(32.0)as f32,text_r,&format!("{arrow}{label}"),0);
        if f.broken {dim_rect(c,text_r);}
        // Separate notebook boxes retain their own page identities; equal page IDs never merge. Incoming lines
        // are dashed, as the map draws a selected page's incoming edges: each page's line by its own links (a
        // box linked both ways has a solid line to a page that links out to it, a dashed one to a page it only
        // links in to).
        for (k,&pid) in f.here.iter().enumerate() {
            if let Some(at)=v.g.nodes.iter().position(|n|n.pid==pid){if let Some(p)=location(v,at){
                let (sx,sy)=screen_point(v,p);let end=(x as f32,(y+r.h/2)as f32);
                canvas_set_clip(c,pr);
                if f.here_out.get(k).copied().unwrap_or(f.out) && !f.broken {canvas_line_aa(c,sx,sy,end.0,end.1,2.0,110);}
                else {canvas_dash_poly(c,&[(sx,sy),end],u(3.0),u(14.0),u(10.0),0,0);}
                canvas_reset_clip(c);
            }}
        }
        v.hits.push(MapHit{rect:r,kind:MapHitKind::Anchor(i),centre:(0.0,0.0),radius:0.0});y+=u(100.0);
    }
}

// The arrow at the start of an anchor box: which way its links go, out of this notebook, into it, or both.
// (Terminess has the single arrows, not a two-way one: both is the pair.)
fn anchor_arrow(out:bool,inc:bool)->&'static str {
    match (out,inc) {(true,true)=>"←→ ",(true,false)=>"→ ",_=>"← "}
}

// Every other dark pixel of r cleared: drawn as off (the Links bar dims a gone page's chip the same way).
fn dim_rect(c:&mut Canvas,r:Rect) {
    let r=rect_intersect(r,c.clip);
    for y in r.y..r.y+r.h {for x in r.x..r.x+r.w {
        let p=&mut c.px[(y*c.stride+x) as usize];if *p<128 && ((x^y)&1)==1 {*p=255;}
    }}
}
