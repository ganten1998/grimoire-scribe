//! The Links bar (Rust only, the Index's phase 1): the links on the open page as one row per place they
//! go (Index entry, "x2" when two go to one), numbered in reading order; peek at one without leaving the
//! page, stay there with a tap on the page, or go back. An editor-hosted sheet like dispmenu.rs: it draws
//! itself and takes the taps, and says what was asked (LbResult); the editor does it. Its functions never
//! reach back into the editor's state (PORTING.md, Sheets and keyboards): the editor takes the bar out of
//! its state for each call that also needs the App, and hands it the notebook and its Nav.
//!
//! The part the eye needs too comes first: how the page's links group into entries and in what order
//! they are numbered, so the number on a box on the page and the number of its row in the bar are one
//! and the same.
//!
//! Layout (logical px at 300 dpi, scaled with ui_px). Upright (1860 wide): the card is 20 px in from each
//! side with its bottom 20 px above the screen's edge, 1820 wide. From the top: 16 px padding, a 36 px grip
//! strip with a 160 x 8 bar, a 100 px header (its second line, 60 px, the first three opens), R rows of
//! chips (R 1 to 3; two columns of 878 x 124, 24 px between and 16 px between rows), 20 px, a 140 px row of
//! four tool buttons and 20 px padding: 316 + 140 R tall. Sideways: three columns of 784, at most two rows
//! (six entries a screen either way), 2440 wide.
//!
//! Link to... (PHASE2 6.6) is the same frame in pick mode (LbMode::Pick): where a new link goes. Its chips are
//! New page..., The Index, up to four recent targets and the Index's entries; its header is "Where does the link
//! go?" with the pager and no X, and its tool row Search, Choose from the pages... (left out when the card opened
//! it) and Cancel. It docks at the top of the screen, below the clock corner, when the area it is for is low on
//! the page, so it never covers what is being linked.

use crate::prelude::*;

/// The most links a page has (nav.h), so the most rows and boxes.
pub const LB_MAX: usize = NAV_LINKS_PER_PAGE as usize;

/// Boxes whose tops are within this many mm are one line of the page, read left to right.
const LINE_MM: f32 = 10.0;

/// A drag down from the grip or the header this far closes the bar.
const DRAG_CLOSE_MM: f32 = 15.0;
/// A press held this long on a chip is neither a peek nor a stay: it is the long-press card's (the editor's tick
/// opens it, PHASE2 8.10).
const HOLD_MS: u32 = 600;

// Hit ids.
const LB_GRIP: i32 = 1;
const LB_X: i32 = 2;
const LB_PREV: i32 = 3;
const LB_NEXT: i32 = 4;
const LB_CHIP: i32 = 5;
const LB_BACK: i32 = 6;
const LB_GO: i32 = 7;
const LB_NEW: i32 = 8;
const LB_AREAS: i32 = 9;
const LB_INDEX: i32 = 10;
// (pick mode)
const LB_SEARCH: i32 = 11;
const LB_PAGES: i32 = 12;
const LB_CANCEL: i32 = 13;
const LB_AROUND: i32 = 14;

/// The bar lists the page's links (List), or it is Link to..., picking where a new link goes (Pick).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LbMode {
    #[default]
    List,
    Pick,
}

/// What a chip of Link to... is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PickKind {
    /// New page...: a page added at the end, named, and linked to
    #[default]
    NewPage,
    /// The Index
    Index,
    /// a page linked to, or left, a moment ago
    Recent,
    /// a named page, in the Index's order
    Entry,
}

/// A chip of Link to...: its kind, the target it gives ("index", "p<page id>"; empty for New page...) and
/// the page's index (NAV_TO_INDEX for The Index, -1 for New page...).
#[derive(Clone, Copy, Debug, Default)]
pub struct LbPick {
    pub kind: PickKind,
    pub target: FixedStr<NAV_TARGET_LEN>,
    pub page: i32,
}

/// The most recent targets Link to... offers.
pub const LB_RECENT_MAX: usize = 4;

/// What a tap on the bar asks the editor to do.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LbResult {
    #[default]
    None,
    /// entry i (an index into entries) tapped: look at its page above the bar
    Peek(i32),
    /// the entry already looked at tapped again: stay on its page
    Commit,
    /// X, or a drag down from the grip or header: back to the origin and close
    Cancel,
    /// the back chip: "‹ Page 3" while peeking (look at the origin again), "Back to ..." otherwise
    Back,
    NewLink,
    GoTo,
    /// the Areas button: the eye turned on or off (the editor redraws the button and the boxes)
    Eye,
    Index,
    Around,
    /// the pager: another screenful of chips (the grid is drawn again)
    Pager,
    /// Link to...: chip i of picks
    Pick(i32),
    /// Link to...: Search (Find in pick mode)
    Search,
    /// Link to...: Choose from the pages... (the Pages view picks)
    Choose,
    /// the pressed button was let go: its rect is to be refreshed
    Changed,
}

/// One row of the bar: a place the page's links go to.
#[derive(Clone, Copy, Debug, Default)]
pub struct LbEntry {
    /// "p<page id>" or "index" (or another version's)
    pub target: FixedStr<NAV_TARGET_LEN>,
    /// the page's index, NAV_TO_INDEX, or -1 for a page that is gone or a target this version does not know
    pub page: i32,
    /// links on the page to it (D1: shown once, x N)
    pub count: i32,
    /// the first of its boxes in reading order, page px
    pub first_box: Rect,
}

/// The links of one page (each with its box, page px) grouped by where they go: entries in the reading
/// order of each entry's first box, numbered from 1; `box_entry[i]` is the number of links[i]'s entry.
/// Boxes whose tops are within 10 mm count as one line, read left to right. Returns how many entries.
pub fn linkbar_group(
    nb: &NotebookInfo,
    links: &[NavLink],
    areas: &[Rect],
    dpi: f32,
    entries: &mut [LbEntry; LB_MAX],
    box_entry: &mut [u8; LB_MAX],
) -> usize {
    let n = links.len().min(areas.len()).min(LB_MAX);
    *box_entry = [0; LB_MAX];
    // Reading order: down the page, boxes within a line side by side.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (areas[i].y, areas[i].x, i));
    let line = mm_to_px(LINE_MM, dpi) as i32;
    let mut read: Vec<usize> = Vec::with_capacity(n);
    let mut at = 0;
    while at < order.len() {
        let top = areas[order[at]].y;
        let mut end = at;
        while end < order.len() && areas[order[end]].y - top <= line {
            end += 1;
        }
        let mut row: Vec<usize> = order[at..end].to_vec();
        row.sort_by_key(|&i| (areas[i].x, areas[i].y, i));
        read.extend(row);
        at = end;
    }
    let mut count = 0usize;
    for &i in &read {
        let target = links[i].target;
        let found = (0..count).find(|&e| entries[e].target.as_bytes() == target.as_bytes());
        let e = match found {
            Some(e) => e,
            None => {
                entries[count] = LbEntry { target, page: nav_link_page(&links[i], nb), count: 0, first_box: areas[i] };
                count += 1;
                count - 1
            }
        };
        entries[e].count += 1;
        box_entry[i] = (e + 1) as u8;
    }
    count
}

/// The entries the pager shows now: the screenful.
pub fn linkbar_visible(lb: &LinkBar) -> &[LbEntry] {
    let first = ((lb.screen * lb.per).max(0) as usize).min(lb.n);
    let end = (first + lb.per.max(0) as usize).min(lb.n);
    &lb.entries[first..end]
}

/// The bar's state while it is open (memory only).
#[derive(Clone, Debug)]
pub struct LinkBar {
    pub open: bool,
    /// the page (index) whose links it lists: kept while hopping (D2)
    pub origin: i32,
    pub entries: [LbEntry; LB_MAX],
    pub n: usize,
    /// links in all (the header's count): an entry with two is two
    pub links: i32,
    pub incoming: i32,
    /// the pager's screenful
    pub screen: i32,
    /// the entry being looked at, -1 for none
    pub selected: i32,
    /// the card
    pub rect: Rect,
    pub hits: HitList,
    pub press: Press,
    pub pressed: Hit,
    pub has_pressed: bool,
    /// the origin page is from a newer Grimoire: New link is refused
    pub readonly: bool,
    /// links.ini reads and writes (nav.links_writable): else the bar says it could not read them
    pub links_ok: bool,
    /// this open shows the second header line (the first three do)
    pub header_hint: bool,
    /// the origin page's number, for the back chip "‹ Page %d" while peeking
    pub back_label: i32,
    /// where the back list's newest page is, named ("Back to %s"), "" while it is empty
    pub back_to: FixedStr<NAV_NAME_LEN>,
    /// a press that began on the grip or the header: a drag down from it closes the bar
    drag: bool,
    // The layout (linkbar_layout)
    cols: i32,
    per: i32,
    screens: i32,
    rows: i32,
    /// the header and the chip grid together: what the pager draws again
    pub grid: Rect,
    /// the Areas button, which the eye's toggle draws again
    pub eye_button: Rect,
    // Link to... (pick mode)
    pub mode: LbMode,
    /// every chip, all screens
    pub picks: Vec<LbPick>,
    /// docked at the top of the screen (else at the bottom)
    pub top: bool,
    /// the target marked as the current one (a retarget), empty for a new link
    pub current: FixedStr<NAV_TARGET_LEN>,
    /// Choose from the pages... is not offered (the card's Change where it goes)
    pub no_pages: bool,
    /// it stays open under Find's pick mode, New page...'s keyboard or the Pages view, and is not drawn
    pub hidden: bool,
    /// the pager was turned by hand: a layout keeps the screen it is on
    paged: bool,
    /// the entry whose chip a press held on (the long-press card is up for it: its lift does nothing), -1 none
    pub held: i32,
    /// when the press on the bar began (now_ms, the editor's clock: its tick times the hold)
    pub down_at: u32,
}

// (Arrays derive Default only up to 32 elements: written by hand, as EditorState's LinkBoxes.)
impl Default for LinkBar {
    fn default() -> Self {
        LinkBar {
            open: false,
            origin: 0,
            entries: [LbEntry::default(); LB_MAX],
            n: 0,
            links: 0,
            incoming: 0,
            screen: 0,
            selected: -1,
            rect: RECT_EMPTY,
            hits: HitList::default(),
            press: Press::default(),
            pressed: Hit::default(),
            has_pressed: false,
            readonly: false,
            links_ok: true,
            header_hint: false,
            back_label: 0,
            back_to: FixedStr::default(),
            drag: false,
            cols: 2,
            per: 6,
            screens: 1,
            rows: 1,
            grid: RECT_EMPTY,
            eye_button: RECT_EMPTY,
            mode: LbMode::List,
            picks: Vec::new(),
            top: false,
            current: FixedStr::default(),
            no_pages: false,
            hidden: false,
            paged: false,
            held: -1,
            down_at: 0,
        }
    }
}

/// Opens the bar on page `origin` (page, its Page): its entries, one per place its links go, as the eye
/// numbers them. A read-only page lists its links as any (New link is refused); with links.ini unreadable it
/// lists none and says so. The caller then lays it out (linkbar_layout), after it has set header_hint,
/// back_label and back_to.
pub fn linkbar_build(lb: &mut LinkBar, nb: &NotebookInfo, nav: &Nav, page: &Page, origin: i32, dpi: f32) {
    *lb = LinkBar { open: true, origin, readonly: page.readonly, links_ok: nav.links_writable, ..Default::default() };
    if !lb.links_ok || origin < 0 || origin >= nb.npages() {
        return;
    }
    let pid = nb.page_ids[origin as usize];
    // Nav is sorted by source; repeated links from one page count once, as in the graph.
    let target = format!("p{}", pid);
    let mut last = 0;
    for l in &nav.links {
        if l.pid != pid && l.target == target.as_str() && l.pid != last && nb.page_ids.contains(&l.pid) {
            lb.incoming += 1;
            last = l.pid;
        }
    }
    let mut links = [NavLink::default(); LB_MAX];
    let mut areas = [RECT_EMPTY; LB_MAX];
    let n = nav_link_areas(nav, page, pid, NAV_LINK_MIN_MM * dpi / 25.4, &mut links, &mut areas) as usize;
    let mut numbers = [0u8; LB_MAX];
    lb.n = linkbar_group(nb, &links[..n], &areas[..n], dpi, &mut lb.entries, &mut numbers);
    lb.links = n as i32;
}

fn u(v: f32) -> i32 {
    ui_px(v)
}

/// (columns, rows at most): two by three upright, three by two sideways: six entries a screen either way.
fn grid_of(aw: i32, ah: i32) -> (i32, i32) {
    if aw > ah {
        (3, 2)
    } else {
        (2, 3)
    }
}

/// The card on an aw x ah screen: its width, its height from how many rows its entries need (the most
/// any screenful has, so it does not change size as the pager turns), 20 px above the screen's edge.
pub fn linkbar_layout(lb: &mut LinkBar, aw: i32, ah: i32) {
    let (cols, max_rows) = grid_of(aw, ah);
    lb.cols = cols;
    lb.per = cols * max_rows;
    lb.screens = ((lb.n as i32 + lb.per - 1) / lb.per).max(1);
    lb.screen = clampi(lb.screen, 0, lb.screens - 1);
    lb.rows = clampi((lb.n as i32 + cols - 1) / cols, 1, max_rows);
    let h = u(316.0 + 140.0 * lb.rows as f32) + if lb.header_hint { u(60.0) } else { 0 };
    lb.rect = Rect { x: u(20.0), y: ah - u(20.0) - h, w: aw - 2 * u(20.0), h };
    let top = lb.rect.y + u(16.0) + u(36.0);
    lb.grid = Rect { x: lb.rect.x, y: top, w: lb.rect.w, h: chips_y(lb) + lb.rows * u(124.0) + (lb.rows - 1) * u(16.0) - top };
}

// Where the chips begin: under the header (and its hint line).
fn chips_y(lb: &LinkBar) -> i32 {
    lb.rect.y + u(16.0) + u(36.0) + u(100.0) + if lb.header_hint { u(60.0) } else { 0 }
}

fn inner(lb: &LinkBar) -> Rect {
    rect_inflate(lb.rect, -u(20.0))
}

fn entry_name<'a>(nb: &NotebookInfo, nav: &'a Nav, e: &LbEntry) -> &'a str {
    if e.page >= 0 && (e.page as usize) < nb.page_ids.len() {
        nav_name(nav, nb.page_ids[e.page as usize])
    } else {
        ""
    }
}

/// What an entry is called: its Index name (and true), "Page %d" for a page with none (false), "The Index"
/// (false); "" for a page that is not here.
fn entry_label(nb: &NotebookInfo, nav: &Nav, e: &LbEntry) -> (String, bool) {
    if e.page == NAV_TO_INDEX {
        // TRANSLATORS: the notebook's Index (the list of named pages) as a place a link goes: a Links bar row, a
        // Link to... chip (about 700 px at 44 px in bold upright, 600 sideways) and the long-press card's title.
        return (tr("The Index").to_string(), false);
    }
    if e.page < 0 {
        return (String::new(), false);
    }
    let name = entry_name(nb, nav, e);
    if name.is_empty() {
        (cformat!(tr("Page %d"), e.page + 1), false)
    } else {
        (name.to_string(), true)
    }
}

// The icon above a button's label, drawn dimmer: every other pixel of its dark ones cleared (a 50% ordered
// dither, so it reads as off and not as the gray of a disabled button). `r` is the button.
fn dim_icon(c: &mut Canvas, r: Rect) {
    let inner = rect_inflate(r, -u(8.0));
    let ts = ui_small_size().min(inner.h as f32 * 0.28);
    let lh = (ts * 1.15) as i32;
    let ir = Rect { x: inner.x, y: inner.y, w: inner.w, h: inner.h - lh };
    dim(c, rect_inflate(ir, -(ir.w.min(ir.h) / 10)));
}

fn dim(c: &mut Canvas, r: Rect) {
    let r = rect_intersect(r, c.clip);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            let p = &mut c.px[(y * c.stride + x) as usize];
            if *p < 128 && ((x ^ y) & 1) == 1 {
                *p = 255;
            }
        }
    }
}

// A chip: the entry's number in a black disc, its name, and at the right where it goes and how many links.
fn draw_chip(c: &mut Canvas, r: Rect, num: i32, e: &LbEntry, label: &str, named: bool, selected: bool) {
    ui_frame(c, r, u(16.0), 255, true, 0);
    ui_frame(c, r, u(16.0), 0, false, if selected { u(8.0) } else { u(3.0) });
    let d = u(64.0);
    let disc = Rect { x: r.x + u(18.0), y: r.y + (r.h - d) / 2, w: d, h: d };
    canvas_disc_aa(c, disc.x as f32 + d as f32 / 2.0, disc.y as f32 + d as f32 / 2.0, d as f32 / 2.0, 0, 1.0);
    let mut nt = String::new();
    csnprintf!(&mut nt, 8, "%d", num);
    font_draw_centered(c, FontFace::Bold, ui_small_size(), disc, &nt, 255);
    let col_w = u(150.0);
    let name = Rect { x: r.x + u(96.0), y: r.y, w: r.w - u(96.0) - col_w - u(14.0), h: r.h };
    let ts = ui_text_size();
    if e.page == NAV_TO_INDEX {
        // The Notebook icon and "The Index".
        let ih = u(56.0);
        ui_icon(c, IconKind::Notebook, Rect { x: name.x, y: r.y + (r.h - ih) / 2, w: ih, h: ih }, 0);
        font_draw_left(c, FontFace::Regular, ts, Rect { x: name.x + ih + u(10.0), y: name.y, w: name.w - ih - u(10.0), h: name.h }, tr("The Index"), 0);
    } else if e.page < 0 {
        // A page that is not here: 50% dithered.
        // TRANSLATORS: a chip of the Links bar whose link goes to a page that is gone (about 700 px at 44 px).
        font_draw_left(c, FontFace::Regular, ts, name, tr("Page not here"), 0);
        dim(c, name);
    } else if !named {
        font_draw_left(c, FontFace::Regular, ts, name, label, 0); // (not named: its number, regular)
    } else {
        font_draw_left_name(c, FontFace::Bold, ts, name, label, 0);
    }
    // The right column: "p. 3", and under it "x2" when two links go there.
    let col = Rect { x: r.x + r.w - col_w - u(8.0), y: r.y, w: col_w, h: r.h };
    let mut lines: Vec<String> = Vec::new();
    if e.page >= 0 {
        // TRANSLATORS: a chip of the Links bar, in its right column at 34 px (about 150 px): the page a link goes
        // to, %d its number ("p. 12").
        lines.push(cformat!(c_("bar", "p. %d"), e.page + 1));
    }
    if e.count > 1 {
        // TRANSLATORS: a chip of the Links bar, under the page's number in its right column: how many links go
        // to that page (a multiplication sign and the count, "x2").
        lines.push(cformat!(tr("×%d"), e.count));
    }
    // (centred as a block: one line over the chip's middle, two close together about it)
    let lh = u(46.0);
    let top = col.y + (col.h - lh * lines.len() as i32) / 2;
    for (k, l) in lines.iter().enumerate() {
        font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect { x: col.x, y: top + k as i32 * lh, w: col.w, h: lh }, l, 0);
    }
}

/// Draws the bar into a.screen and registers its hits; returns its card (its rect). `peek` is the page number
/// being looked at above it (0 for none): then the header is the peek's, with the way back to the origin.
pub fn linkbar_draw(lb: &mut LinkBar, a: &mut App, nb: &NotebookInfo, nav: &Nav, peek: i32) -> Rect {
    if lb.mode == LbMode::Pick {
        return draw_pick(lb, a, nb, nav);
    }
    hits_clear(&mut lb.hits);
    let card = lb.rect;
    let inn = inner(lb);
    let c = &mut a.screen;
    ui_frame(c, card, u(24.0), 255, true, 0);
    ui_frame(c, card, u(24.0), 0, false, u(6.0));
    // The grip strip and the header: a press that drags down from them closes the bar.
    let gy = card.y + u(16.0);
    canvas_fill(c, Rect { x: card.x + (card.w - u(160.0)) / 2, y: gy + u(14.0), w: u(160.0), h: u(8.0) }, 160);
    let hy = gy + u(36.0);
    let head = Rect { x: inn.x, y: hy, w: inn.w, h: u(100.0) };
    hits_add(&mut lb.hits, Rect { x: card.x, y: card.y, w: card.w, h: hy + u(100.0) - card.y }, LB_GRIP, 0);
    // The X, and the pager left of it with more than a screenful.
    let bw = u(118.0);
    let bh = u(100.0);
    let x_btn = Rect { x: head.x + head.w - bw, y: hy, w: bw, h: bh };
    ui_button(c, x_btn, IconKind::Close, "", ButtonStyle::Plain);
    hits_add(&mut lb.hits, x_btn, LB_X, 0);
    let mut right = x_btn.x - u(16.0);
    if lb.screens > 1 {
        let next = Rect { x: right - bw, y: hy, w: bw, h: bh };
        let label_w = u(110.0);
        let lab = Rect { x: next.x - label_w, y: hy, w: label_w, h: bh };
        let prev = Rect { x: lab.x - bw, y: hy, w: bw, h: bh };
        ui_button(c, prev, IconKind::Prev, "", if lb.screen > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        ui_button(c, next, IconKind::Next, "", if lb.screen < lb.screens - 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        let mut pt = String::new();
        csnprintf!(&mut pt, 16, "%d/%d", lb.screen + 1, lb.screens);
        font_draw_centered(c, FontFace::Bold, ui_small_size(), lab, &pt, 0);
        if lb.screen > 0 {
            hits_add(&mut lb.hits, prev, LB_PREV, 0);
        }
        if lb.screen < lb.screens - 1 {
            hits_add(&mut lb.hits, next, LB_NEXT, 0);
        }
        right = prev.x - u(16.0);
    }
    let ts = ui_text_size();
    if peek > 0 {
        // The way back to where the bar began, then what is on show.
        // TRANSLATORS: the Links bar's chip, while a page is on show above it, that shows the page the bar began
        // on again: a single angle quote and the page's number (up to 380 px at 44 px in bold).
        let back_text = cformat!(tr("‹ Page %d"), lb.back_label);
        let bwid = ui_chip_w(FontFace::Bold, ts, &back_text, bh, u(200.0), u(380.0));
        let back = Rect { x: head.x, y: hy, w: bwid, h: bh };
        ui_button(c, back, IconKind::None, &back_text, ButtonStyle::Plain);
        hits_add(&mut lb.hits, back, LB_BACK, 0);
        // (the page's own name, or its number: an entry looked at and the back list's page both are pages)
        let name = if (peek as usize) <= nb.page_ids.len() { nav_name(nav, nb.page_ids[peek as usize - 1]) } else { "" };
        let shown = if name.is_empty() { cformat!(tr("Page %d"), peek) } else { name.to_string() };
        // TRANSLATORS: the Links bar's header while a page is on show above it: %s is its name (or "Page 3"), cut
        // to fit; about 1000 px at 44 px.
        let msg = cformat!(tr("Showing %s. Tap the page to stay."), &shown);
        // (a name is cut to fit, as names are anywhere)
        font_draw_left_name(c, FontFace::Regular, ts, Rect { x: back.x + back.w + u(20.0), y: hy, w: right - back.x - back.w - u(20.0), h: bh }, &msg, 0);
    } else {
        let mut tx = head.x;
        if !lb.back_to.is_empty() {
            // The back list has somewhere to go: a chip that looks there.
            // TRANSLATORS: the Links bar's chip, with no page on show, that looks at the page the last link, Find or
            // Pages view left: %s is its name (or "Page 3"); up to 560 px at 44 px in bold.
            let bt = cformat!(tr("Back to %s"), lb.back_to.as_str());
            let bwid = ui_chip_w(FontFace::Bold, ts, &bt, bh, u(260.0), u(560.0));
            let back = Rect { x: head.x, y: hy, w: bwid, h: bh };
            ui_button(c, back, IconKind::None, &bt, ButtonStyle::Plain);
            hits_add(&mut lb.hits, back, LB_BACK, 0);
            tx = back.x + back.w + u(20.0);
        }
        if lb.incoming > 0 {
            let text = cformat!(ng_("%d page links here", "%d pages link here", lb.incoming as i64), lb.incoming);
            let width = ui_chip_w(FontFace::Bold, ui_small_size(), &text, bh, u(360.0), u(620.0)).min(right - tx);
            let chip = Rect { x: right - width, y: hy, w: width, h: bh };
            ui_button(c, chip, IconKind::None, &text, ButtonStyle::Plain);
            hits_add(&mut lb.hits, chip, LB_AROUND, 0);
            right = chip.x - u(16.0);
        }
        // TRANSLATORS: the Links bar's header: %d is how many links the open page has (about 700 px at 44 px in bold).
        let title = cformat!(tr("Links on this page (%d)"), lb.links);
        ui_text(c, FontFace::Bold, ts, ui_small_size(), Rect { x: tx, y: hy, w: right - tx, h: bh }, &title, 1, false, 0);
    }
    if lb.header_hint && peek == 0 {
        // TRANSLATORS: a second line of the Links bar's header the first three times it opens, at 34 px (about 1300 px).
        font_draw_left(c, FontFace::Regular, ui_small_size(), Rect { x: head.x, y: hy + bh - u(4.0), w: head.w, h: u(60.0) }, tr("Tap a link to look. Tap the page to stay."), 0);
    }
    // The chips, or why there are none.
    let cy = chips_y(lb);
    let ch = u(124.0);
    let gap = u(24.0);
    let cw = (inn.w - (lb.cols - 1) * gap) / lb.cols;
    if lb.n == 0 {
        // TRANSLATORS: the Links bar's chip area when the open page has no links (about 1300 px at 44 px).
        let none = tr("No links on this page yet.");
        // TRANSLATORS: the same when links.ini cannot be read, so the bar lists none (about 1300 px at 44 px).
        let unreadable = tr("Links could not be read.");
        let msg = if lb.links_ok { none } else { unreadable };
        font_draw_left(c, FontFace::Regular, ts, Rect { x: inn.x + u(10.0), y: cy, w: inn.w - u(20.0), h: ch }, msg, 0);
    }
    let first = lb.screen * lb.per;
    for k in 0..lb.per {
        let i = first + k;
        if i as usize >= lb.n {
            break;
        }
        let r = Rect { x: inn.x + (k % lb.cols) * (cw + gap), y: cy + (k / lb.cols) * (ch + u(16.0)), w: cw, h: ch };
        let e = lb.entries[i as usize];
        let (label, named) = entry_label(nb, nav, &e);
        draw_chip(c, r, i + 1, &e, &label, named, i == lb.selected);
        hits_add(&mut lb.hits, r, LB_CHIP, i);
    }
    // The tool row: Go to, New link, Areas, Index.
    let tg = u(20.0);
    let tw = (inn.w - 3 * tg) / 4;
    let ty = card.y + card.h - u(20.0) - u(140.0);
    let tool = |k: i32| Rect { x: inn.x + k * (tw + tg), y: ty, w: tw, h: u(140.0) };
    let can_link = !lb.readonly && lb.links_ok;
    // TRANSLATORS: the Links bar's four tool buttons, each under its icon in caps at 34 px (about 400 px
    // wide): look for a page by its number or name; draw a new link; show the link areas on the page; the
    // Index.
    ui_button(c, tool(0), IconKind::Search, tr("Go to"), ButtonStyle::Plain);
    hits_add(&mut lb.hits, tool(0), LB_GO, 0);
    // TRANSLATORS: the Links bar's tool button that starts a new link (400 px at 34 px caps, under its icon).
    ui_button(c, tool(1), IconKind::LinkPlus, tr("New link"), if !can_link { ButtonStyle::Disabled } else if lb.n == 0 { ButtonStyle::Primary } else { ButtonStyle::Plain });
    hits_add(&mut lb.hits, tool(1), LB_NEW, 0);
    let eye_on = a.settings.link_areas;
    let eye = tool(2);
    // TRANSLATORS: the Links bar's tool button that shows or hides the dotted link areas on the page (400 px at
    // 34 px caps, under its eye; the eye is dimmed while they are hidden).
    ui_button(c, eye, IconKind::Eye, tr("Areas"), if !lb.links_ok { ButtonStyle::Disabled } else if eye_on { ButtonStyle::Selected } else { ButtonStyle::Plain });
    if !eye_on && lb.links_ok {
        dim_icon(c, eye);
    }
    hits_add(&mut lb.hits, eye, LB_AREAS, 0);
    lb.eye_button = eye;
    ui_button(c, tool(3), IconKind::Bubbles, tr("Index"), ButtonStyle::Plain);
    hits_add(&mut lb.hits, tool(3), LB_INDEX, 0);
    card
}

/// A pen or finger on the bar (phase 0 down, 1 move, 2 up). A chip or a button is inverted at once, on a DU
/// of its own (app_press_feedback), and acts at the lift if that is a tap; the grip and the header take a drag
/// down of 15 mm as Cancel. `dirty` gets the rect of the pressed button to refresh when it is let go.
pub fn linkbar_pointer(lb: &mut LinkBar, a: &mut App, ph: i32, ev: &InEvent, dirty: &mut Rect) -> LbResult {
    *dirty = RECT_EMPTY;
    if ph == 0 {
        press_begin(&mut lb.press, ev.x, ev.y, ev.t_ms);
        lb.held = -1;
        let h = hits_find(&lb.hits, ev.x, ev.y).copied();
        lb.has_pressed = h.is_some();
        lb.drag = false;
        if let Some(h) = h {
            lb.pressed = h;
            if h.id == LB_GRIP {
                lb.drag = true;
            } else {
                app_press_feedback(a, h.r);
            }
        }
        return LbResult::None;
    }
    if !lb.has_pressed {
        return LbResult::None;
    }
    press_move(&mut lb.press, ev.x, ev.y);
    if ph == 1 {
        if lb.drag && ev.y - lb.press.y0 >= mm_to_px(DRAG_CLOSE_MM, a.dpi) {
            lb.has_pressed = false;
            lb.drag = false;
            return LbResult::Cancel;
        }
        return LbResult::None;
    }
    lb.has_pressed = false;
    let h = lb.pressed;
    if h.id == LB_GRIP {
        lb.drag = false;
        return LbResult::None;
    }
    // The press is let go: drawn as it was, and refreshed whatever the tap did.
    canvas_invert(&mut a.screen, h.r);
    *dirty = h.r;
    let held = ev.t_ms.wrapping_sub(lb.press.t0) > HOLD_MS;
    if held || !press_is_tap(&lb.press, h.r) {
        return LbResult::None;
    }
    match h.id {
        LB_X => LbResult::Cancel,
        LB_PREV => {
            lb.screen = (lb.screen - 1).max(0);
            lb.paged = true;
            LbResult::Pager
        }
        LB_NEXT => {
            lb.screen = (lb.screen + 1).min(lb.screens - 1);
            lb.paged = true;
            LbResult::Pager
        }
        LB_CHIP if lb.mode == LbMode::Pick => LbResult::Pick(h.arg),
        LB_SEARCH => LbResult::Search,
        LB_PAGES => LbResult::Choose,
        LB_CANCEL => LbResult::Cancel,
        LB_CHIP => {
            if h.arg == lb.selected {
                LbResult::Commit
            } else {
                LbResult::Peek(h.arg)
            }
        }
        LB_BACK => LbResult::Back,
        LB_GO => LbResult::GoTo,
        LB_NEW => LbResult::NewLink,
        LB_AREAS => LbResult::Eye,
        LB_INDEX => LbResult::Index,
        LB_AROUND => LbResult::Around,
        _ => LbResult::None,
    }
}

/// The chip a press is on now in List mode, unmoved (entry index and the chip's rect): what a hold opens the
/// long-press card for. None for any other press.
pub fn linkbar_pressed_chip(lb: &LinkBar) -> Option<(i32, Rect)> {
    if !lb.open || lb.mode != LbMode::List || !lb.has_pressed || lb.pressed.id != LB_CHIP || lb.press.moved || lb.held >= 0 {
        return None;
    }
    Some((lb.pressed.arg, lb.pressed.r))
}

/// The press on chip `i` is a hold now (the long-press card opens for its entry): the chip is drawn as it was
/// (its rect returned, to refresh), and the lift does nothing.
pub fn linkbar_hold(lb: &mut LinkBar, a: &mut App, i: i32) -> Rect {
    lb.held = i;
    if !lb.has_pressed {
        return RECT_EMPTY;
    }
    lb.has_pressed = false;
    canvas_invert(&mut a.screen, lb.pressed.r);
    lb.pressed.r
}

/// The finger that pressed a button is a pinch now: the button is drawn as it was. The rect to refresh (empty
/// when no button was pressed).
pub fn linkbar_cancel_press(lb: &mut LinkBar, a: &mut App) -> Rect {
    let was = lb.has_pressed && lb.pressed.id != LB_GRIP;
    lb.has_pressed = false;
    lb.drag = false;
    if !was {
        return RECT_EMPTY;
    }
    canvas_invert(&mut a.screen, lb.pressed.r);
    lb.pressed.r
}

/// The runner's keys for the bar, without "editor.bar." (docs/SIM.md): "entries" (how many, 0 closed), "N"
/// (entry N from 1: `p3` then ` x2` when two links go there, `index` or `missing`; empty past the last),
/// "pager" (`2/3`, empty with one screenful) and "rect" (the card, `x y w h`, empty while closed). false for
/// another key.
pub fn linkbar_probe(lb: &LinkBar, key: &str, out: &mut String, cap: usize) -> bool {
    if lb.mode == LbMode::Pick {
        return linkbar_probe_pick(lb, key, out, cap);
    }
    if key == "entries" {
        csnprintf!(out, cap, "%d", if lb.open { lb.n as i32 } else { 0 });
    } else if key == "incoming" {
        csnprintf!(out, cap, "%d", lb.incoming);
    } else if key == "pager" {
        // The pager's screenful, "2/3"; empty with one screenful or closed.
        if lb.open && lb.screens > 1 {
            csnprintf!(out, cap, "%d/%d", lb.screen + 1, lb.screens);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "rect" {
        if lb.open {
            csnprintf!(out, cap, "%d %d %d %d", lb.rect.x, lb.rect.y, lb.rect.w, lb.rect.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key.bytes().all(|b| b.is_ascii_digit()) && !key.is_empty() {
        let i = key.parse::<i32>().unwrap_or(0) - 1;
        if lb.open && i >= 0 && (i as usize) < lb.n {
            let e = &lb.entries[i as usize];
            if e.page == NAV_TO_INDEX {
                csnprintf!(out, cap, "%s", "index");
            } else if e.page < 0 {
                csnprintf!(out, cap, "%s", "missing");
            } else if e.count > 1 {
                csnprintf!(out, cap, "p%d x%d", e.page + 1, e.count);
            } else {
                csnprintf!(out, cap, "p%d", e.page + 1);
            }
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else {
        return false;
    }
    true
}

// ---- Link to... (pick mode, PHASE2 6.6) ----

/// Link to... for a link on the page whose id is open_pid: the chips New page..., The Index, up to four of
/// `recent` (newest first: the targets picked a moment ago, then the back list), and every named page in the
/// Index's order, the open page included. A recent target is left out when it is the open page, the Index, a
/// page that is not here, or one already offered; `current` (a retarget's target, "" for a new link) marks
/// its chips, and the pager opens on the first of them; `no_pages` leaves Choose from the pages... out.
/// Returns -1 when there was no memory for the Index's entries: then only New page..., The Index and the
/// recent targets are offered (Search still reaches every page).
pub fn linkbar_build_pick(
    lb: &mut LinkBar,
    nb: &NotebookInfo,
    nav: &Nav,
    recent: &[FixedStr<NAV_TARGET_LEN>],
    open_pid: u32,
    current: &str,
    no_pages: bool,
) -> i32 {
    let origin = nb.page_ids.iter().position(|&p| p == open_pid).map_or(-1, |i| i as i32);
    *lb = LinkBar { open: true, mode: LbMode::Pick, origin, no_pages, ..Default::default() };
    lb.current.set_cut(current);
    // The Index's entries: page indices in its order.
    let mut idx: Vec<i32> = Vec::new();
    let mut named = 0usize;
    let mut rc = 0;
    if idx.try_reserve_exact(nb.page_ids.len()).is_ok() {
        idx.resize(nb.page_ids.len(), 0);
        named = (nav_index(nav, nb, &mut idx).max(0) as usize).min(idx.len());
    } else {
        rc = -1;
    }
    if lb.picks.try_reserve_exact(2 + LB_RECENT_MAX + named).is_err() {
        rc = -1;
        named = 0;
        if lb.picks.try_reserve_exact(2 + LB_RECENT_MAX).is_err() {
            return -1; // (no chips at all: the tool row alone)
        }
    }
    lb.picks.push(LbPick { kind: PickKind::NewPage, target: FixedStr::default(), page: -1 });
    lb.picks.push(LbPick { kind: PickKind::Index, target: FixedStr::from_str("index"), page: NAV_TO_INDEX });
    let mut taken = 0;
    for t in recent {
        if taken >= LB_RECENT_MAX {
            break;
        }
        let page = nav_link_page(&NavLink { target: *t, ..Default::default() }, nb);
        if page < 0 || nb.page_ids[page as usize] == open_pid {
            continue; // (the Index and pages that are not here are negative)
        }
        let again = lb.picks.iter().any(|p| p.kind == PickKind::Recent && p.target.as_bytes() == t.as_bytes());
        if again {
            continue;
        }
        lb.picks.push(LbPick { kind: PickKind::Recent, target: *t, page });
        taken += 1;
    }
    for &i in &idx[..named] {
        let mut target = FixedStr::<NAV_TARGET_LEN>::default();
        let mut t = String::new();
        csnprintf!(&mut t, NAV_TARGET_LEN, "p%u", nb.page_ids[i as usize]);
        target.set_cut(&t);
        lb.picks.push(LbPick { kind: PickKind::Entry, target, page: i });
    }
    lb.selected = if current.is_empty() { -1 } else { lb.picks.iter().position(|p| p.kind != PickKind::NewPage && p.target == current).map_or(-1, |i| i as i32) };
    rc
}

/// Whether Link to... docks at the top: the area it is for (its screen rect's centre, `centre_y`) is in the
/// lower 45% of an `ah` tall screen, where a card docked at the bottom would cover it.
pub fn linkbar_pick_docks_top(centre_y: i32, ah: i32) -> bool {
    centre_y as f32 >= 0.55 * ah as f32
}

/// Link to... on an aw x ah screen: the bar's frame, 316 + 140 R tall for R rows of chips (the most a screenful
/// has), at the top with its top at top_y (below the clock corner), or at the bottom ending at bottom_y (above a
/// clock corner at the bottom; 0: 20 px above the edge).
pub fn linkbar_layout_pick(lb: &mut LinkBar, aw: i32, ah: i32, top: bool, top_y: i32, bottom_y: i32) {
    let (cols, max_rows) = grid_of(aw, ah);
    let n = lb.picks.len() as i32;
    lb.cols = cols;
    lb.per = cols * max_rows;
    lb.screens = ((n + lb.per - 1) / lb.per).max(1);
    if !lb.paged && lb.selected >= 0 {
        lb.screen = lb.selected / lb.per; // (the current target's screen)
    }
    lb.screen = clampi(lb.screen, 0, lb.screens - 1);
    lb.rows = clampi((n + cols - 1) / cols, 1, max_rows);
    lb.header_hint = false;
    lb.top = top;
    let h = u(316.0 + 140.0 * lb.rows as f32);
    let y = if top { top_y } else if bottom_y > 0 { bottom_y - h } else { ah - u(20.0) - h };
    lb.rect = Rect { x: u(20.0), y, w: aw - 2 * u(20.0), h };
    let gtop = lb.rect.y + u(16.0) + u(36.0);
    lb.grid = Rect { x: lb.rect.x, y: gtop, w: lb.rect.w, h: chips_y(lb) + lb.rows * u(124.0) + (lb.rows - 1) * u(16.0) - gtop };
}

/// A chip's label in Link to...: the page's Index name (bold, and true), "Page %d" for a page with none, "New
/// page...", "The Index".
fn pick_label(nb: &NotebookInfo, nav: &Nav, p: &LbPick) -> (String, bool) {
    match p.kind {
        // TRANSLATORS: the first chip of Link to..., which adds a page at the end of the notebook and links to
        // it (about 700 px at 44 px in bold upright, 600 sideways).
        PickKind::NewPage => (tr("New page...").to_string(), false),
        PickKind::Index => (tr("The Index").to_string(), false),
        _ => {
            let e = LbEntry { target: p.target, page: p.page, count: 1, first_box: RECT_EMPTY };
            entry_label(nb, nav, &e)
        }
    }
}

// A chip of Link to...: an icon in the bar's number slot (the Check on the current target), its label, and the
// page's number at the right.
fn draw_pick_chip(c: &mut Canvas, r: Rect, p: &LbPick, label: &str, named: bool, current: bool) {
    ui_frame(c, r, u(16.0), 255, true, 0);
    ui_frame(c, r, u(16.0), 0, false, if current { u(8.0) } else { u(3.0) });
    let d = u(64.0);
    let slot = Rect { x: r.x + u(18.0), y: r.y + (r.h - d) / 2, w: d, h: d };
    let icon = if current {
        IconKind::Check
    } else {
        match p.kind {
            PickKind::NewPage => IconKind::Plus,
            PickKind::Index => IconKind::Notebook,
            PickKind::Recent => IconKind::Clock,
            PickKind::Entry => IconKind::None,
        }
    };
    if icon != IconKind::None {
        ui_icon(c, icon, slot, 0);
    }
    let col_w = u(150.0);
    let name = Rect { x: r.x + u(96.0), y: r.y, w: r.w - u(96.0) - col_w - u(14.0), h: r.h };
    let ts = ui_text_size();
    if named {
        font_draw_left_name(c, FontFace::Bold, ts, name, label, 0);
    } else {
        font_draw_left(c, FontFace::Regular, ts, name, label, 0);
    }
    if p.page >= 0 {
        let col = Rect { x: r.x + r.w - col_w - u(8.0), y: r.y, w: col_w, h: r.h };
        font_draw_centered(c, FontFace::Regular, ui_small_size(), col, &cformat!(c_("bar", "p. %d"), p.page + 1), 0);
    }
}

/// The tool row's buttons: Search, Choose from the pages... and Cancel, or Search and Cancel.
fn pick_tools(lb: &LinkBar) -> Vec<(Rect, i32)> {
    let inn = inner(lb);
    let tg = u(20.0);
    let ty = lb.rect.y + lb.rect.h - u(20.0) - u(140.0);
    let ids: &[i32] = if lb.no_pages { &[LB_SEARCH, LB_CANCEL] } else { &[LB_SEARCH, LB_PAGES, LB_CANCEL] };
    let k = ids.len() as i32;
    let tw = (inn.w - (k - 1) * tg) / k;
    ids.iter().enumerate().map(|(i, &id)| (Rect { x: inn.x + i as i32 * (tw + tg), y: ty, w: tw, h: u(140.0) }, id)).collect()
}

fn draw_pick(lb: &mut LinkBar, a: &mut App, nb: &NotebookInfo, nav: &Nav) -> Rect {
    hits_clear(&mut lb.hits);
    let card = lb.rect;
    let inn = inner(lb);
    let c = &mut a.screen;
    ui_frame(c, card, u(24.0), 255, true, 0);
    ui_frame(c, card, u(24.0), 0, false, u(6.0));
    let gy = card.y + u(16.0);
    canvas_fill(c, Rect { x: card.x + (card.w - u(160.0)) / 2, y: gy + u(14.0), w: u(160.0), h: u(8.0) }, 160);
    let hy = gy + u(36.0);
    let (bw, bh) = (u(118.0), u(100.0));
    let mut right = inn.x + inn.w;
    if lb.screens > 1 {
        let next = Rect { x: right - bw, y: hy, w: bw, h: bh };
        let label_w = u(110.0);
        let lab = Rect { x: next.x - label_w, y: hy, w: label_w, h: bh };
        let prev = Rect { x: lab.x - bw, y: hy, w: bw, h: bh };
        ui_button(c, prev, IconKind::Prev, "", if lb.screen > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        ui_button(c, next, IconKind::Next, "", if lb.screen < lb.screens - 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        let mut pt = String::new();
        csnprintf!(&mut pt, 16, "%d/%d", lb.screen + 1, lb.screens);
        font_draw_centered(c, FontFace::Bold, ui_small_size(), lab, &pt, 0);
        if lb.screen > 0 {
            hits_add(&mut lb.hits, prev, LB_PREV, 0);
        }
        if lb.screen < lb.screens - 1 {
            hits_add(&mut lb.hits, next, LB_NEXT, 0);
        }
        right = prev.x - u(16.0);
    }
    let ts = ui_text_size();
    // TRANSLATORS: the header of Link to..., the card that picks where a new link goes (about 1100 px at 44 px
    // in bold).
    ui_text(c, FontFace::Bold, ts, ui_small_size(), Rect { x: inn.x, y: hy, w: right - inn.x, h: bh }, tr("Where does the link go?"), 1, false, 0);
    let cy = chips_y(lb);
    let ch = u(124.0);
    let gap = u(24.0);
    let cw = (inn.w - (lb.cols - 1) * gap) / lb.cols;
    let first = lb.screen * lb.per;
    for k in 0..lb.per {
        let i = first + k;
        if i as usize >= lb.picks.len() {
            break;
        }
        let r = Rect { x: inn.x + (k % lb.cols) * (cw + gap), y: cy + (k / lb.cols) * (ch + u(16.0)), w: cw, h: ch };
        let p = lb.picks[i as usize];
        let (label, named) = pick_label(nb, nav, &p);
        let current = !lb.current.is_empty() && p.kind != PickKind::NewPage && p.target == lb.current.as_str();
        draw_pick_chip(c, r, &p, &label, named, current);
        hits_add(&mut lb.hits, r, LB_CHIP, i);
    }
    for (r, id) in pick_tools(lb) {
        let (icon, label) = match id {
            // TRANSLATORS: Link to...'s tool button that finds a page by its number or name for the link (in caps
            // at 34 px under its icon: about 540 px, or two lines; 860 px when there are two buttons).
            LB_SEARCH => (IconKind::Search, tr("Search")),
            // TRANSLATORS: Link to...'s tool button that opens the Pages view to tap the page the link goes to (in
            // caps at 34 px under its icon: about 540 px, or two lines).
            LB_PAGES => (IconKind::Grid, tr("Choose from the pages...")),
            _ => (IconKind::Close, tr("Cancel")),
        };
        ui_button(c, r, icon, label, ButtonStyle::Plain);
        hits_add(&mut lb.hits, r, id, 0);
    }
    card
}

/// The name of what Link to... has at screen point (x, y), for the runner's editor.ui_at (docs/SIM.md):
/// `linkto.chip.N` (N from 1 across the screens), `linkto.search`, `linkto.pages`, `linkto.cancel`,
/// `linkto.prev`, `linkto.next`; empty for none.
pub fn linkbar_hit_name(lb: &LinkBar, x: i32, y: i32) -> String {
    if !lb.open || lb.hidden || lb.mode != LbMode::Pick {
        return String::new();
    }
    match hits_find(&lb.hits, x as f32, y as f32) {
        Some(h) if h.id == LB_CHIP => cformat!("linkto.chip.%d", h.arg + 1),
        Some(h) => match h.id {
            LB_SEARCH => "linkto.search",
            LB_PAGES => "linkto.pages",
            LB_CANCEL => "linkto.cancel",
            LB_PREV => "linkto.prev",
            LB_NEXT => "linkto.next",
            _ => "",
        }
        .to_string(),
        None => String::new(),
    }
}

/// The runner's keys for Link to... (pick mode), without "editor.linkto.": "rect" (its card, empty closed),
/// "dock" (top or bottom), "chips" (how many, all screens), "N" (chip N from 1: new, index, recent p<page>,
/// p<page>, then " current" on the marked target; empty past the last), "screen" (S/T). false for another key.
pub fn linkbar_probe_pick(lb: &LinkBar, key: &str, out: &mut String, cap: usize) -> bool {
    if key == "rect" {
        if lb.open {
            csnprintf!(out, cap, "%d %d %d %d", lb.rect.x, lb.rect.y, lb.rect.w, lb.rect.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "dock" {
        csnprintf!(out, cap, "%s", if !lb.open { "" } else if lb.top { "top" } else { "bottom" });
    } else if key == "chips" {
        csnprintf!(out, cap, "%d", if lb.open { lb.picks.len() as i32 } else { 0 });
    } else if key == "screen" {
        if lb.open {
            csnprintf!(out, cap, "%d/%d", lb.screen + 1, lb.screens);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()) {
        let i = key.parse::<i32>().unwrap_or(0) - 1;
        match lb.picks.get(i.max(0) as usize).filter(|_| lb.open && i >= 0) {
            Some(p) => {
                let mut v = match p.kind {
                    PickKind::NewPage => "new".to_string(),
                    PickKind::Index => "index".to_string(),
                    PickKind::Recent => cformat!("recent p%d", p.page + 1),
                    PickKind::Entry => cformat!("p%d", p.page + 1),
                };
                if !lb.current.is_empty() && p.kind != PickKind::NewPage && p.target == lb.current.as_str() {
                    v.push_str(" current");
                }
                csnprintf!(out, cap, "%s", &v);
            }
            None => {
                csnprintf!(out, cap, "%s", "");
            }
        }
    } else {
        return false;
    }
    true
}

/// Named list controls for the headless runner; only currently active hit targets resolve.
pub fn linkbar_control_point(lb:&LinkBar,name:&str)->Option<(f32,f32)> {
    if !lb.open||lb.hidden{return None;}
    let (id,arg)=if lb.mode==LbMode::Pick {
        match name {
            "index"=>(LB_CHIP,lb.picks.iter().position(|p|p.kind==PickKind::Index)? as i32),
            "pages"=>(LB_PAGES,0),"search"=>(LB_SEARCH,0),"cancel"=>(LB_CANCEL,0),
            n if n.starts_with("chip.")=>(LB_CHIP,n[5..].parse::<i32>().ok()?-1),
            _=>return None,
        }
    } else {
        (match name {"index"=>LB_INDEX,"incoming"=>LB_AROUND,"new"=>LB_NEW,"areas"=>LB_AREAS,"close"=>LB_X,_=>return None},0)
    };
    lb.hits.h.iter().find(|h|h.id==id&&h.arg==arg).map(|h|((h.r.x+h.r.w/2) as f32,(h.r.y+h.r.h/2) as f32))
}
