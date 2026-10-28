//! graph: the Index map's model and layout, and graph.ini, the file that keeps the layout between
//! visits (docs/index-hook, plan.graph_view and plan.data_and_formats).
//!
//! Every page of a notebook is a dot. The model is built from what the notebook already says about
//! its pages, so no page is loaded: the page order (NotebookInfo.page_ids), the Index names
//! (nav.ini), the links (links.ini) and the page that is open. The pages fall into three bands:
//!
//! - force nodes: named pages, pages with any link, and the open page. Only these are laid out by
//!   the forces, so a notebook of 2000 pages costs what its linked part costs;
//! - satellites: unnamed, unlinked pages with a named page before them. They sit on a sunflower
//!   spiral around that page (their section's head, as nav_section_of finds it), placed after the
//!   layout;
//! - the tray: unnamed, unlinked pages before the first named page, which belong to no section.
//!   The view lists them in notebook order beside the graph.
//!
//! Links between two pages make one edge per pair, with the directions it has and how many links
//! it stands for. Links to the Index are a badge on the page and never an edge: every page that
//! links to the Index would make it a hub pulling the whole map into a hairball.
//!
//! The layout is Navi's: a seed of BFS shells around each component's best linked page, then
//! about 300 Barnes-Hut steps of springs and repulsion, then the components packed side by side.
//! Floats come out differently on arm64 and armv7 (fused multiply-adds, libm), so the result is
//! kept in graph.ini and laid out again only when the graph changed a lot; a new page is placed
//! beside its neighbours and only its neighbourhood moves. The tests assert invariants, never
//! coordinates.
//!
//! Every loop of the layout runs in page-id order and every tie is broken by page id, never by a
//! page's place in the notebook: moving a page in the notebook moves no dot.
//!
//! The module is pure: no screen, no clock, no thread, and its only file is graph.ini. Long work
//! takes a pump callback (work_pump in the app) every GRAPH_PUMP_STEPS steps; a big layout runs in
//! a bgjob child (graph_layout_in_process says when, graph_layout_job is the child's work). Every
//! `a * b + c` in the float code is written `fmla(a, b, c)` (PORTING.md, Floating point).

use crate::prelude::*;
use std::f32::consts::{PI, TAU};

// ---- the model (plan.graph_view, DATA and NODES) ----

/// The page-id table's slots: a power of two, a little over twice STORE_MAX_PAGES, so a lookup
/// probes about one slot and always finds an empty one to stop at.
pub const GRAPH_SLOTS: usize = 4096;

/// GraphEdge.dir: the page with the lower id (a) links to the other (b), and b links to a.
pub const GRAPH_DIR_AB: u8 = 1;
pub const GRAPH_DIR_BA: u8 = 2;

// ---- the room the view's marks need (plan.graph_view, LOOK) ----

/// A force node's radius at fit, px: GRAPH_R_MIN + GRAPH_R_SPAN x ln(1 + d) / ln(1 + dmax).
pub const GRAPH_R_MIN: f32 = 14.0;
pub const GRAPH_R_SPAN: f32 = 16.0;
/// Unnamed force nodes are drawn this much smaller, so the Index's own pages stand out.
pub const GRAPH_R_UNNAMED: f32 = 0.75;
/// Satellites and tray pages (unnamed and unlinked) are small hollow dots.
pub const GRAPH_R_SAT: f32 = 6.0;
/// The white disc drawn under every dot is this much wider than the dot. No dot may reach into
/// another's disc, or the disc would cut it: the layout keeps r_a + r_b + GRAPH_KNOCKOUT between
/// the centres of any two dots it places (force nodes and satellites).
pub const GRAPH_KNOCKOUT: f32 = 4.0;
/// A section shows this many satellites at fit. The rest are counted (graph_sat_more, the view's
/// '+N') and drawn from 2x zoom; they have their room on the spiral all the same.
pub const GRAPH_SAT_DRAWN: i32 = 60;
/// The satellite spiral's pitch: each satellite gets pi x pitch^2 of the disc around its head, so
/// the closest two of 60 are about 20 px apart, more than two dots and a knockout (16 px).
pub const GRAPH_SAT_PITCH: f32 = 12.0;
/// The tray: its pitch in px at fit, and the count past which the view folds it into one line.
pub const GRAPH_TRAY_PITCH: i32 = 40;
pub const GRAPH_TRAY_FOLD: i32 = 200;

// ---- the layout (Navi's constants, plan.graph_view LAYOUT) ----

pub const GRAPH_REPULSION: f32 = 16000.0;
pub const GRAPH_SPRING_K: f32 = 0.055;
/// The springs' rest length: also the unit of every other distance in the layout.
pub const GRAPH_SPRING_L: f32 = 110.0;
pub const GRAPH_GRAVITY: f32 = 0.02;
pub const GRAPH_DAMPING: f32 = 0.9;
/// Barnes-Hut: a cell of side s at distance d acts as one body when s < theta x d.
pub const GRAPH_THETA: f32 = 0.9;
/// Steps of a full layout, and of an incremental one (over the new pages' 2-hop neighbourhood).
pub const GRAPH_STEPS: i32 = 300;
pub const GRAPH_INC_STEPS: i32 = 40;
/// The pump callback runs every this many steps, and once per collision round: for 300 nodes at
/// most 0.9 ms of work on the test host between two calls, some 18 ms on the Kindle (the plan's
/// host x20).
pub const GRAPH_PUMP_STEPS: i32 = 2;
/// Up to this many force nodes a full layout runs in the app's process, pumped: 300 take 75 ms on
/// the test host, about 1.5 s on the Kindle (the plan guessed 0.25 s). Past it, a bgjob child runs
/// it (graph_layout_job; 2000 nodes and 4000 edges take 0.9 s on the host) while the view shows
/// graph_seed's placement.
pub const GRAPH_INPROC_MAX: i32 = 300;
/// A full layout replaces the kept one when more than this percentage of the pages it keeps are
/// new (graph_relayout_needed); fewer are placed one by one (graph_place_new).
pub const GRAPH_RELAYOUT_PCT: i32 = 10;
/// Positions are whole world units within +-this (a map 2000 pages wide spans a few thousand).
pub const GRAPH_POS_MAX: i32 = 1_000_000;

// The space between packed components: a rest length, so two never read as one.
const COMP_GAP: f32 = GRAPH_SPRING_L;
// The collision pass leaves this much more than the knockout between two dots, so rounding both to
// whole units for graph.ini (at most 0.71 each) cannot bring them back into each other.
const SLACK: f32 = 2.0;
// What the collision pass forgives: a pair it has just set apart must not count as touching again
// because of a rounding error in the last bit.
const SLACK_EPS: f32 = 0.01;
// One step moves a node at most this far, so a close pair's repulsion cannot throw it across the map.
const MAX_MOVE: f32 = 40.0;
// Rounds of the collision pass, at most: a few are enough once spread has opened a dense component
// (2000 nodes take 3); the cap only bounds the work in a case nobody has met.
const COLLIDE_ROUNDS: i32 = 100;
// relax: a component with at most this many moving nodes keeps one tree of the others.
const FEW_MOVING: usize = 64;
// spread: a component is spread when it has a touching pair for every this many members or fewer,
// by at most this factor a pass, for at most this many passes.
const SPREAD_FEW: usize = 20;
const SPREAD_MAX: f32 = 1.5;
const SPREAD_PASSES: i32 = 4;
// The quadtree splits no deeper than this; bodies that still share a cell are summed one by one.
const TREE_DEPTH: i32 = 24;
// The golden angle, pi x (3 - sqrt 5): each satellite turns this far from the last, so none line
// up whatever their count (Vogel's sunflower).
const GOLDEN: f32 = 2.399_963;

// The salts of the page-id hash, one per use, so the jitters are not tied to each other.
const SALT_ANGLE: u32 = 1;
const SALT_RADIUS: u32 = 2;
const SALT_AROUND: u32 = 3;
const SALT_SAT: u32 = 4;
const SALT_APART: u32 = 5;
const SALT_SPOT: u32 = 6;
const SALT_TURN: u32 = 7;

// ---- graph.ini (plan.data_and_formats) ----

/// graph.ini is read up to this size; a bigger file is not one this version wrote.
pub const GRAPH_FILE_MAX: usize = 256 * 1024;
/// Lines this version does not read are carried up to this length each, and this much in all.
pub const GRAPH_LINE_MAX: usize = 255;
pub const GRAPH_UNKNOWN_MAX: usize = 16 * 1024;

const INI_HEADER: &[u8] = b"# Grimoire: the Index map's layout (a cache: deleting it loses nothing)\n";

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphTier {
    /// named, linked, or the open page: laid out by the forces
    #[default]
    Force = 0,
    /// unnamed and unlinked, after a named page: on a spiral around it
    Satellite = 1,
    /// unnamed and unlinked, before the first named page: in the tray
    Tray = 2,
}

/// What last placed the force nodes (the view's editor.map.layout probe reads 'seed' and 'done').
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphLaid {
    #[default]
    Unplaced = 0,
    /// graph_seed: the O(n + e) placement shown while a bgjob child lays the map out
    Seed = 1,
    /// graph_layout, or graph_place_new over kept positions
    Done = 2,
}

/// What graph_apply_ini found the kept layout good for.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphPlan {
    /// every kept page has its position: graph_place_new only adds the open page if it needs one
    #[default]
    Ready = 0,
    /// a few pages are new: graph_place_new places them beside their neighbours
    Incremental = 1,
    /// nothing kept, or more than GRAPH_RELAYOUT_PCT new: lay the map out again (graph_layout,
    /// or graph_seed and a bgjob child when graph_layout_in_process says no)
    Full = 2,
}

/// The map's mode, remembered in graph.ini: the whole notebook, or the rings around one page.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GraphMode {
    #[default]
    All = 0,
    Around = 1,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GraphNode {
    pub pid: u32,
    pub tier: GraphTier,
    /// has an Index name (nav.ini)
    pub named: bool,
    /// has a link: out (to a page of this notebook, to the Index or to itself) or in (from a page)
    pub linked: bool,
    /// the page open in the editor
    pub open: bool,
    /// distinct pages it is linked with, either way (not the Index, not itself)
    pub degree: i32,
    /// distinct pages it links to, and that link to it (the card's "Links to 3, linked from 2")
    pub links_to: i32,
    pub linked_from: i32,
    /// its links to The Index: the badge's count (the view may draw it as a flag)
    pub index_links: i32,
    /// its links to itself: never an edge and not in the degree, but the page counts as linked
    pub self_links: i32,
    /// its section's named page (nav_section_of: itself when it is named), -1 before the first name
    pub section: i32,
    /// a satellite's place among its section's satellites, in notebook order; -1 for the others
    pub sat_rank: i32,
    /// how many satellites a named page has
    pub sats: i32,
    /// a force node's component (numbered in the order of their lowest page id); -1 for the others
    pub comp: i32,
    /// radius at fit, px: graph_radius for force nodes, GRAPH_R_SAT for satellites and the tray
    pub r: f32,
    /// world position (1 unit = 1 px at zoom 1), valid when placed. The tray is never placed: the
    /// view lines it up in screen space.
    pub x: f32,
    pub y: f32,
    pub placed: bool,
}

/// One linked pair of pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphEdge {
    /// the two nodes, a with the lower page id
    pub a: i32,
    pub b: i32,
    /// GRAPH_DIR_AB and/or GRAPH_DIR_BA
    pub dir: u8,
    /// the links it stands for, both ways together (the view widens the line for more)
    pub mult: i32,
}

/// The map of one notebook: graph_build makes it, the layout functions place it.
#[derive(Clone, Debug, Default)]
pub struct GraphModel {
    /// every page, in notebook order: node i is page i
    pub nodes: Vec<GraphNode>,
    /// one per linked pair, sorted by the pair's page ids (a's, then b's)
    pub edges: Vec<GraphEdge>,
    /// the force nodes, in page-id order (the order every layout loop runs in)
    pub force: Vec<i32>,
    /// the tray's pages, in notebook order
    pub tray: Vec<i32>,
    /// the largest degree of a force node
    pub dmax: i32,
    /// the open page's node, -1 for none
    pub open: i32,
    /// how many components the force nodes make
    pub ncomp: i32,
    pub laid: GraphLaid,
    // The page-id table: node + 1 in each used slot, 0 in an empty one.
    slots: Vec<u16>,
    // Each node's place in page-id order: what ties are broken by.
    rank: Vec<u32>,
    // Node i's neighbours are adj[adj_at[i]..adj_at[i + 1]], in page-id order.
    adj_at: Vec<u32>,
    adj: Vec<i32>,
}

/// A member of the Around rings (graph_around), placed around the centre page at (0, 0).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GraphRing {
    pub node: i32,
    /// 0 for the centre, 1 or 2 links away
    pub depth: i32,
    /// the node it was reached from (-1 for the centre)
    pub parent: i32,
    pub x: f32,
    pub y: f32,
}

/// A kept position (pos.<pid>=<x>,<y>).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphPos {
    pub pid: u32,
    pub x: i32,
    pub y: i32,
}

/// graph.ini, the map's cache in the notebook's folder:
///
/// ```text
/// # Grimoire: the Index map's layout (a cache: deleting it loses nothing)
/// gen=5f3a9c21
/// mode=all
/// unlinked=show
/// pos.12=-310,442
/// pin.12=1
/// ```
///
/// pin lines are reserved for dragging dots (v2): read and written back, never used. Lines this
/// version does not read are carried. A file that does not read is refused: it is ignored, and
/// `writable` stays false so that session never overwrites it (graph_ini_read_path says what does
/// not read). GraphIni::default() is a fresh cache: mode all, unlinked shown, writable.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphIni {
    /// graph_gen of the model the positions belong to
    pub gen: u32,
    pub has_gen: bool,
    pub mode: GraphMode,
    /// unlinked=show|hide: the satellites and the tray
    pub show_unlinked: bool,
    /// sorted by page id, one each (the last line for a page wins)
    pub pos: Vec<GraphPos>,
    /// sorted, one each
    pub pins: Vec<u32>,
    /// the lines this version does not read, each with its newline, as read
    pub unknown: Vec<u8>,
    /// false after a file that does not read: never overwrite it
    pub writable: bool,
    /// graph.ini is there (or was refused)
    pub file: bool,
}

impl Default for GraphIni {
    fn default() -> Self {
        GraphIni {
            gen: 0,
            has_gen: false,
            mode: GraphMode::All,
            show_unlinked: true,
            pos: Vec::new(),
            pins: Vec::new(),
            unknown: Vec::new(),
            writable: true,
            file: false,
        }
    }
}

// ---- the page-id hash: jitter and tie-breaking ----

// A page id mixed with a salt (the murmur3 finalizer). The seed's jitter, the free spots and the
// spirals' turn come from it, so a page's place depends on its id and never on its index.
fn pid_hash(pid: u32, salt: u32) -> u32 {
    let mut h = pid ^ salt.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

// The hash as a fraction in [0, 1): its top 24 bits, which an f32 holds exactly.
fn pid_unit(pid: u32, salt: u32) -> f32 {
    (pid_hash(pid, salt) >> 8) as f32 / 16_777_216.0
}

// The hash as a jitter in [-0.5, 0.5).
fn pid_jitter(pid: u32, salt: u32) -> f32 {
    pid_unit(pid, salt) - 0.5
}

// ---- the page-id table ----

// Fibonacci hashing: the top 12 bits of the id times 2^32 / phi spread a notebook's consecutive
// ids evenly over the table.
fn slot_of(pid: u32) -> usize {
    (pid.wrapping_mul(0x9E37_79B1) >> 20) as usize & (GRAPH_SLOTS - 1)
}

fn slot_put(slots: &mut [u16], nodes: &[GraphNode], i: usize) {
    let pid = nodes[i].pid;
    let mut s = slot_of(pid);
    loop {
        let v = slots[s] as usize;
        if v == 0 {
            slots[s] = (i + 1) as u16;
            return;
        }
        // A page listed twice: its first place answers, as nav_link_page's search of the order does.
        if nodes[v - 1].pid == pid {
            return;
        }
        s = (s + 1) & (GRAPH_SLOTS - 1);
    }
}

/// The node of page pid, -1 when the notebook has no such page.
pub fn graph_node_of(g: &GraphModel, pid: u32) -> i32 {
    if pid == 0 || g.slots.len() != GRAPH_SLOTS {
        return -1;
    }
    let mut s = slot_of(pid);
    loop {
        let v = g.slots[s] as usize;
        if v == 0 {
            return -1;
        }
        if g.nodes[v - 1].pid == pid {
            return (v - 1) as i32;
        }
        s = (s + 1) & (GRAPH_SLOTS - 1);
    }
}

/// A node's linked pages, in page-id order (empty for a bad node).
pub fn graph_neighbours(g: &GraphModel, node: i32) -> &[i32] {
    if node < 0 || node as usize + 1 >= g.adj_at.len() {
        return &[];
    }
    let i = node as usize;
    &g.adj[g.adj_at[i] as usize..g.adj_at[i + 1] as usize]
}

/// The edge between two nodes, -1 when they are not linked.
pub fn graph_edge_between(g: &GraphModel, u: i32, v: i32) -> i32 {
    let n = g.nodes.len() as i32;
    if u < 0 || v < 0 || u >= n || v >= n || u == v || g.rank.len() != g.nodes.len() {
        return -1;
    }
    let (a, b) = if g.rank[u as usize] < g.rank[v as usize] { (u, v) } else { (v, u) };
    let key = (g.rank[a as usize], g.rank[b as usize]);
    match g.edges.binary_search_by(|e| (g.rank[e.a as usize], g.rank[e.b as usize]).cmp(&key)) {
        Ok(at) => at as i32,
        Err(_) => -1,
    }
}

// A page target's page id ("p<id>", the id from 1 to 4294967295 in decimal without leading zeros),
// 0 for any other target. It is the grammar of nav.rs's target_page; nav_link_page finds the page
// by searching the page order for each link, and the pid table does that here, so the build stays
// O(n + e).
fn target_pid(t: &[u8]) -> u32 {
    let c1 = t.get(1).copied().unwrap_or(0);
    if t.first() != Some(&b'p') || !(b'1'..=b'9').contains(&c1) {
        return 0;
    }
    let mut id: u64 = 0;
    for &c in &t[1..] {
        if !c.is_ascii_digit() {
            return 0;
        }
        id = id * 10 + (c - b'0') as u64;
        if id > u32::MAX as u64 {
            return 0;
        }
    }
    id as u32
}

// The nodes in page-id order: a stable radix sort, 8 bits a pass, O(n). It never looks at the
// notebook order, which is what keeps the layout independent of it.
fn pid_order(nodes: &[GraphNode]) -> Vec<u32> {
    let n = nodes.len();
    let mut a: Vec<u32> = (0..n as u32).collect();
    let mut b = vec![0u32; n];
    for shift in [0u32, 8, 16, 24] {
        let mut count = [0usize; 257];
        for &i in &a {
            count[((nodes[i as usize].pid >> shift) & 255) as usize + 1] += 1;
        }
        for k in 0..256 {
            count[k + 1] += count[k];
        }
        for &i in &a {
            let k = ((nodes[i as usize].pid >> shift) & 255) as usize;
            b[count[k]] = i;
            count[k] += 1;
        }
        std::mem::swap(&mut a, &mut b);
    }
    a
}

// Sorts directed links (low rank, high rank, direction) by the pair: two stable counting passes,
// the high rank and then the low, O(n + e).
fn sort_pairs(p: &mut Vec<(u32, u32, u8)>, n: usize) {
    let mut tmp = vec![(0u32, 0u32, 0u8); p.len()];
    for pass in 0..2 {
        let key = |e: &(u32, u32, u8)| if pass == 0 { e.1 as usize } else { e.0 as usize };
        let mut count = vec![0usize; n + 1];
        for e in p.iter() {
            count[key(e) + 1] += 1;
        }
        for k in 0..n {
            count[k + 1] += count[k];
        }
        for e in p.iter() {
            let k = key(e);
            tmp[count[k]] = *e;
            count[k] += 1;
        }
        std::mem::swap(p, &mut tmp);
    }
}

/// A force node's radius at fit: 14 + 16 x ln(1 + d) / ln(1 + dmax) px (14 to 30), 0.75 of it
/// for an unnamed page. With no edge anywhere every node has the smallest.
pub fn graph_radius(d: i32, dmax: i32, named: bool) -> f32 {
    let t = if dmax > 0 { ((1 + d.clamp(0, dmax)) as f32).ln() / ((1 + dmax) as f32).ln() } else { 0.0 };
    let r = fmla(GRAPH_R_SPAN, t, GRAPH_R_MIN);
    if named {
        r
    } else {
        r * GRAPH_R_UNNAMED
    }
}

/// The nodes graph.ini keeps and graph_gen covers: the force nodes that are named or linked. The
/// open page, when it is a force node only because it is open, is placed afresh beside its section
/// each time: it would otherwise make every visit from another plain page a different graph.
pub fn graph_persisted(n: &GraphNode) -> bool {
    n.tier == GraphTier::Force && (n.named || n.linked)
}

/// How many of a section head's satellites are not drawn at fit (the view's '+N'), 0 for a node
/// that is no head.
pub fn graph_sat_more(g: &GraphModel, head: i32) -> i32 {
    if head < 0 || head as usize >= g.nodes.len() {
        return 0;
    }
    (g.nodes[head as usize].sats - GRAPH_SAT_DRAWN).max(0)
}

/// Builds the map of notebook nb: its pages, Index names and links (nav), and the open page
/// (open_pid, 0 for none). Nothing is placed yet. -1 for a notebook with more pages than
/// STORE_MAX_PAGES (the pid table is sized for those), with g left empty.
///
/// One pass over the pages, the names and the links, each looked up in the pid table, so it is
/// O(n + e). Links from a page or to a page that is not in the notebook are ignored, as are targets
/// this version does not know; they make no page linked. A link from a page to itself counts as a
/// link (the page is a force node, so the link can be seen) but is no edge and no degree: a loop
/// says nothing about where the page belongs.
pub fn graph_build(g: &mut GraphModel, nb: &NotebookInfo, nav: &Nav, open_pid: u32) -> i32 {
    *g = GraphModel { open: -1, ..GraphModel::default() };
    let n = nb.page_ids.len();
    if n > STORE_MAX_PAGES {
        return -1;
    }
    let blank = GraphNode { section: -1, sat_rank: -1, comp: -1, ..GraphNode::default() };
    g.nodes = nb.page_ids.iter().map(|&pid| GraphNode { pid, ..blank }).collect();
    g.slots = vec![0u16; GRAPH_SLOTS];
    for i in 0..n {
        if g.nodes[i].pid != 0 {
            slot_put(&mut g.slots, &g.nodes, i);
        }
    }
    let order = pid_order(&g.nodes);
    g.rank = vec![0u32; n];
    for (k, &i) in order.iter().enumerate() {
        g.rank[i as usize] = k as u32;
    }
    for name in &nav.names {
        if name.text.is_empty() {
            continue;
        }
        let at = graph_node_of(g, name.pid);
        if at >= 0 {
            g.nodes[at as usize].named = true;
        }
    }
    if open_pid != 0 {
        g.open = graph_node_of(g, open_pid);
        if g.open >= 0 {
            g.nodes[g.open as usize].open = true;
        }
    }
    // Every link once: a badge, a loop, or a directed pair.
    let mut pairs: Vec<(u32, u32, u8)> = Vec::new();
    for l in &nav.links {
        let src = graph_node_of(g, l.pid);
        if src < 0 {
            continue;
        }
        let src = src as usize;
        if l.target == "index" {
            g.nodes[src].index_links += 1;
            g.nodes[src].linked = true;
            continue;
        }
        let dst = graph_node_of(g, target_pid(l.target.as_bytes()));
        if dst < 0 {
            continue;
        }
        let dst = dst as usize;
        g.nodes[src].linked = true;
        if dst == src {
            g.nodes[src].self_links += 1;
            continue;
        }
        g.nodes[dst].linked = true;
        if g.rank[src] < g.rank[dst] {
            pairs.push((g.rank[src], g.rank[dst], GRAPH_DIR_AB));
        } else {
            pairs.push((g.rank[dst], g.rank[src], GRAPH_DIR_BA));
        }
    }
    // One edge per pair: the pair's links side by side once sorted.
    sort_pairs(&mut pairs, n);
    for &(lo, hi, dir) in &pairs {
        let (a, b) = (order[lo as usize] as i32, order[hi as usize] as i32);
        match g.edges.last_mut() {
            Some(e) if e.a == a && e.b == b => {
                e.dir |= dir;
                e.mult += 1;
            }
            _ => g.edges.push(GraphEdge { a, b, dir, mult: 1 }),
        }
    }
    for k in 0..g.edges.len() {
        let e = g.edges[k];
        let (a, b) = (e.a as usize, e.b as usize);
        g.nodes[a].degree += 1;
        g.nodes[b].degree += 1;
        if e.dir & GRAPH_DIR_AB != 0 {
            g.nodes[a].links_to += 1;
            g.nodes[b].linked_from += 1;
        }
        if e.dir & GRAPH_DIR_BA != 0 {
            g.nodes[b].links_to += 1;
            g.nodes[a].linked_from += 1;
        }
    }
    // The neighbour lists, filled in edge order: each comes out in page-id order, the pages before
    // it from the edges where it is b, then those after it from the edges where it is a.
    g.adj_at = vec![0u32; n + 1];
    for i in 0..n {
        g.adj_at[i + 1] = g.adj_at[i] + g.nodes[i].degree as u32;
    }
    g.adj = vec![0i32; g.adj_at[n] as usize];
    let mut fill: Vec<u32> = g.adj_at[..n].to_vec();
    for e in &g.edges {
        let (a, b) = (e.a as usize, e.b as usize);
        g.adj[fill[a] as usize] = e.b;
        fill[a] += 1;
        g.adj[fill[b] as usize] = e.a;
        fill[b] += 1;
    }
    // The bands, in notebook order: a section runs from a named page to the next.
    let mut head: i32 = -1;
    for i in 0..n {
        if g.nodes[i].named {
            head = i as i32;
        }
        g.nodes[i].section = head;
        let nd = g.nodes[i];
        if nd.named || nd.linked || nd.open {
            g.nodes[i].tier = GraphTier::Force;
        } else if head >= 0 {
            let h = head as usize;
            g.nodes[i].tier = GraphTier::Satellite;
            g.nodes[i].sat_rank = g.nodes[h].sats;
            g.nodes[h].sats += 1;
        } else {
            g.nodes[i].tier = GraphTier::Tray;
            g.tray.push(i as i32);
        }
    }
    for &i in &order {
        if g.nodes[i as usize].tier == GraphTier::Force {
            g.force.push(i as i32);
        }
    }
    g.dmax = g.force.iter().map(|&i| g.nodes[i as usize].degree).max().unwrap_or(0);
    for i in 0..n {
        let nd = g.nodes[i];
        g.nodes[i].r = if nd.tier == GraphTier::Force { graph_radius(nd.degree, g.dmax, nd.named) } else { GRAPH_R_SAT };
    }
    // Components, numbered from their lowest page id, found breadth first.
    let mut queue: Vec<usize> = Vec::new();
    let mut nc = 0;
    for k in 0..g.force.len() {
        let s = g.force[k] as usize;
        if g.nodes[s].comp >= 0 {
            continue;
        }
        g.nodes[s].comp = nc;
        queue.clear();
        queue.push(s);
        let mut at = 0;
        while at < queue.len() {
            let u = queue[at];
            at += 1;
            for j in g.adj_at[u] as usize..g.adj_at[u + 1] as usize {
                let v = g.adj[j] as usize;
                if g.nodes[v].comp < 0 {
                    g.nodes[v].comp = nc;
                    queue.push(v);
                }
            }
        }
        nc += 1;
    }
    g.ncomp = nc;
    0
}

// ---- graph.ini's gen ----

fn fnv(mut h: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// FNV-1a 32 of the graph the layout belongs to, written to graph.ini as gen=<8 hex>. The bytes,
/// in this order:
///
/// - for each node graph_persisted keeps, in increasing page id: 'n' (0x6E), then its page id as 4
///   bytes, least significant first;
/// - for each edge, in increasing (lower page id, higher page id): 'e' (0x65), the lower page id,
///   then the higher, 4 bytes each, least significant first.
///
/// Names, directions, multiplicities and Index badges are left out: they change no position. An
/// empty graph's gen is FNV's offset basis, 811c9dc5.
pub fn graph_gen(g: &GraphModel) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for &v in &g.force {
        let nd = &g.nodes[v as usize];
        if graph_persisted(nd) {
            h = fnv(h, b"n");
            h = fnv(h, &nd.pid.to_le_bytes());
        }
    }
    for e in &g.edges {
        h = fnv(h, b"e");
        h = fnv(h, &g.nodes[e.a as usize].pid.to_le_bytes());
        h = fnv(h, &g.nodes[e.b as usize].pid.to_le_bytes());
    }
    h
}

/// The 10% rule: a full layout when more than GRAPH_RELAYOUT_PCT of the kept pages are new
/// (fresh of kept). A handful of new pages are placed beside their neighbours instead, so nothing
/// else moves.
pub fn graph_relayout_needed(fresh: i32, kept: i32) -> bool {
    fresh > 0 && (fresh >= kept || fresh as i64 * 100 > kept as i64 * GRAPH_RELAYOUT_PCT as i64)
}

/// Whether a full layout of g runs in the app's process (pumped) or in a bgjob child.
pub fn graph_layout_in_process(g: &GraphModel) -> bool {
    g.force.len() as i32 <= GRAPH_INPROC_MAX
}

// ---- the satellites' spiral ----

// The inner edge of a head's satellite spiral: clear of the head's dot, a satellite and both
// knockouts.
fn sat_r0(r_head: f32) -> f32 {
    r_head + GRAPH_R_SAT + GRAPH_KNOCKOUT + SLACK
}

// The squared distance of satellite `rank` (of `count`) from its head's centre. Satellite k lies
// at sqrt(r0^2 + pitch^2 (k + 0.5)): each takes the same area of the annulus, so the spiral is as
// even at its first dot as at its last. The ones past GRAPH_SAT_DRAWN (drawn from 2x zoom, where
// the dots grow with the zoom) go on at the same pitch, so they are as far apart; only a section
// so long (some 650 satellites) that they would leave 3 rest lengths of the head has that outer
// band squeezed to stay within them, and there the 2x view may show them touching.
fn sat_rho2(r_head: f32, count: i32, rank: i32) -> f32 {
    let r0 = sat_r0(r_head);
    let c2 = GRAPH_SAT_PITCH * GRAPH_SAT_PITCH;
    if rank < GRAPH_SAT_DRAWN {
        return fmla(c2, rank as f32 + 0.5, r0 * r0);
    }
    let base = fmla(c2, GRAPH_SAT_DRAWN as f32, r0 * r0);
    let hidden = (count - GRAPH_SAT_DRAWN).max(1) as f32;
    let lim = fmla(3.0, GRAPH_SPRING_L, -GRAPH_R_SAT);
    let mut h2 = c2;
    if fmla(h2, hidden, base) > lim * lim {
        h2 = (fmla(lim, lim, -base) / hidden).max(0.0);
    }
    fmla(h2, (rank - GRAPH_SAT_DRAWN) as f32 + 0.5, base)
}

// How far a head's satellites reach from its centre, their own radius included: the disc the
// collision pass keeps clear around the head, so no satellite ever lies on another dot, at any
// zoom.
fn sat_extent(r_head: f32, count: i32) -> f32 {
    if count <= 0 {
        return r_head;
    }
    sat_rho2(r_head, count, count - 1).sqrt() + GRAPH_R_SAT
}

// Satellite `rank` of a head with `count` of them, from the head's centre.
fn sat_offset(r_head: f32, count: i32, rank: i32, turn: f32) -> (f32, f32) {
    let rho = sat_rho2(r_head, count, rank).sqrt();
    let a = fmla(GOLDEN, rank as f32, turn);
    (rho * a.cos(), rho * a.sin())
}

// Puts every satellite on its head's spiral, turned by the head's page id so two neighbouring
// sections do not look alike. A head that is not placed leaves its satellites unplaced.
fn place_satellites(g: &mut GraphModel) {
    for i in 0..g.nodes.len() {
        if g.nodes[i].tier != GraphTier::Satellite {
            continue;
        }
        let s = g.nodes[i].section;
        if s < 0 || !g.nodes[s as usize].placed {
            g.nodes[i].placed = false;
            continue;
        }
        let h = g.nodes[s as usize];
        let (dx, dy) = sat_offset(h.r, h.sats, g.nodes[i].sat_rank, pid_unit(h.pid, SALT_SAT) * TAU);
        g.nodes[i].x = h.x + dx;
        g.nodes[i].y = h.y + dy;
        g.nodes[i].placed = true;
    }
}

// ---- the layout's working state ----

// The components' members, in page-id order: list[at[c]..at[c + 1]]; and each one's hub (its
// node with the highest degree, the lowest page id among equals).
struct Comps {
    at: Vec<u32>,
    list: Vec<u32>,
    hub: Vec<u32>,
}

impl Comps {
    fn members(&self, c: usize) -> &[u32] {
        &self.list[self.at[c] as usize..self.at[c + 1] as usize]
    }
}

fn comps_of(g: &GraphModel) -> Comps {
    let nc = g.ncomp.max(0) as usize;
    let mut at = vec![0u32; nc + 1];
    for &v in &g.force {
        at[g.nodes[v as usize].comp as usize + 1] += 1;
    }
    for c in 0..nc {
        at[c + 1] += at[c];
    }
    let mut fill: Vec<u32> = at[..nc].to_vec();
    let mut list = vec![0u32; g.force.len()];
    for &v in &g.force {
        let c = g.nodes[v as usize].comp as usize;
        list[fill[c] as usize] = v as u32;
        fill[c] += 1;
    }
    let mut hub = vec![0u32; nc];
    for c in 0..nc {
        let m = &list[at[c] as usize..at[c + 1] as usize];
        let mut best = m[0];
        for &v in m {
            if g.nodes[v as usize].degree > g.nodes[best as usize].degree {
                best = v;
            }
        }
        hub[c] = best;
    }
    Comps { at, list, hub }
}

// One cell of the Barnes-Hut quadtree.
#[derive(Clone, Copy, Default)]
struct Quad {
    // the square: its corner and side
    x0: f32,
    y0: f32,
    side: f32,
    // the bodies under it: how many, and the sum of their positions
    mass: f32,
    sx: f32,
    sy: f32,
    // the four quarters' cells (0: none, as the root is never a child) once it has split
    kids: [u32; 4],
    split: bool,
    // a leaf's body + 1 (0: none); bodies that share a leaf at the depth limit chain through next
    first: u32,
}

#[derive(Default)]
struct Tree {
    q: Vec<Quad>,
    next: Vec<u32>,
    px: Vec<f32>,
    py: Vec<f32>,
    stack: Vec<u32>,
}

impl Tree {
    fn build(&mut self, g: &GraphModel, members: &[u32]) {
        let n = g.nodes.len();
        if self.next.len() < n {
            self.next.resize(n, 0);
            self.px.resize(n, 0.0);
            self.py.resize(n, 0.0);
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &v in members {
            let nd = &g.nodes[v as usize];
            self.px[v as usize] = nd.x;
            self.py[v as usize] = nd.y;
            x0 = x0.min(nd.x);
            y0 = y0.min(nd.y);
            x1 = x1.max(nd.x);
            y1 = y1.max(nd.y);
        }
        self.q.clear();
        // (one unit wider than the bodies' box, so the last of them is inside the half-open square)
        self.q.push(Quad { x0, y0, side: (x1 - x0).max(y1 - y0) + 1.0, ..Quad::default() });
        for &v in members {
            self.insert(v);
        }
    }

    fn quarter(&self, at: usize, x: f32, y: f32) -> usize {
        let q = &self.q[at];
        let half = q.side * 0.5;
        (x >= q.x0 + half) as usize + 2 * (y >= q.y0 + half) as usize
    }

    fn kid(&mut self, at: usize, k: usize) -> usize {
        if self.q[at].kids[k] == 0 {
            let q = self.q[at];
            let half = q.side * 0.5;
            let x0 = if k & 1 != 0 { q.x0 + half } else { q.x0 };
            let y0 = if k & 2 != 0 { q.y0 + half } else { q.y0 };
            self.q.push(Quad { x0, y0, side: half, ..Quad::default() });
            let id = (self.q.len() - 1) as u32;
            self.q[at].kids[k] = id;
        }
        self.q[at].kids[k] as usize
    }

    fn insert(&mut self, b: u32) {
        let (x, y) = (self.px[b as usize], self.py[b as usize]);
        let mut at = 0usize;
        let mut depth = 0;
        loop {
            {
                let q = &mut self.q[at];
                q.mass += 1.0;
                q.sx += x;
                q.sy += y;
            }
            if self.q[at].split {
                let k = self.quarter(at, x, y);
                at = self.kid(at, k);
                depth += 1;
                continue;
            }
            if self.q[at].first == 0 {
                self.q[at].first = b + 1;
                self.next[b as usize] = 0;
                return;
            }
            if depth >= TREE_DEPTH {
                self.next[b as usize] = self.q[at].first;
                self.q[at].first = b + 1;
                return;
            }
            // A second body in a leaf: the first goes down a level, then b goes on from here.
            let c = self.q[at].first - 1;
            self.q[at].first = 0;
            self.q[at].split = true;
            let (cx, cy) = (self.px[c as usize], self.py[c as usize]);
            let k = self.quarter(at, cx, cy);
            let kid = self.kid(at, k);
            self.q[kid] = Quad { mass: 1.0, sx: cx, sy: cy, first: c + 1, ..self.q[kid] };
            self.next[c as usize] = 0;
            let k = self.quarter(at, x, y);
            at = self.kid(at, k);
            depth += 1;
        }
    }

    // The repulsion at (x, y) from the tree's bodies but `skip` (the body that is there itself, or
    // u32::MAX for a point outside the tree).
    fn push_at(&mut self, x: f32, y: f32, skip: u32) -> (f32, f32) {
        let i = skip;
        let (mut fx, mut fy) = (0.0f32, 0.0f32);
        self.stack.clear();
        self.stack.push(0);
        while let Some(at) = self.stack.pop() {
            let q = self.q[at as usize];
            if q.mass <= 0.0 {
                continue;
            }
            if !q.split {
                let mut b = q.first;
                while b != 0 {
                    let j = b - 1;
                    if j != i {
                        repel(x, y, self.px[j as usize], self.py[j as usize], 1.0, &mut fx, &mut fy);
                    }
                    b = self.next[j as usize];
                }
                continue;
            }
            let cx = q.sx / q.mass;
            let cy = q.sy / q.mass;
            let dx = x - cx;
            let dy = y - cy;
            let d2 = fmla(dx, dx, dy * dy);
            // A cell that holds the body itself is always opened, so it never pushes itself.
            let inside = x >= q.x0 && x < q.x0 + q.side && y >= q.y0 && y < q.y0 + q.side;
            if !inside && q.side * q.side < GRAPH_THETA * GRAPH_THETA * d2 {
                repel(x, y, cx, cy, q.mass, &mut fx, &mut fy);
            } else {
                for &k in &q.kids {
                    if k != 0 {
                        self.stack.push(k);
                    }
                }
            }
        }
        (fx, fy)
    }
}

// Coulomb's repulsion of a mass m at (px, py) on a body at (x, y): REPULSION x m / d^2, away from
// it. Closer than one unit counts as one unit, so two dots on top of each other cannot blow up.
fn repel(x: f32, y: f32, px: f32, py: f32, m: f32, fx: &mut f32, fy: &mut f32) {
    let dx = x - px;
    let dy = y - py;
    let d2 = fmla(dx, dx, dy * dy).max(1.0);
    let f = GRAPH_REPULSION * m / (d2 * d2.sqrt());
    *fx = fmla(dx, f, *fx);
    *fy = fmla(dy, f, *fy);
}

struct Lay {
    vx: Vec<f32>,
    vy: Vec<f32>,
    fx: Vec<f32>,
    fy: Vec<f32>,
    // What a dot keeps clear around its centre: its radius, or for a section head the reach of its
    // satellites.
    eff: Vec<f32>,
    movable: Vec<bool>,
    tree: Tree,
}

impl Lay {
    fn new(g: &GraphModel) -> Lay {
        let n = g.nodes.len();
        let eff = g.nodes.iter().map(|nd| if nd.sats > 0 { nd.r.max(sat_extent(nd.r, nd.sats)) } else { nd.r }).collect();
        Lay {
            vx: vec![0.0; n],
            vy: vec![0.0; n],
            fx: vec![0.0; n],
            fy: vec![0.0; n],
            eff,
            movable: vec![false; n],
            tree: Tree::default(),
        }
    }
}

// Evenly spaced angles for nodes in the order given, the ring turned as a whole to sit as close as
// it can to where each would like to be (`ideal`, in the same order): the circular mean of the
// differences. When the wishes cancel out (they all want the same angle) the ring starts at the
// first one's.
fn ring_angles(ideal: &[f32], out: &mut Vec<f32>) {
    out.clear();
    let m = ideal.len();
    if m == 0 {
        return;
    }
    let step = TAU / m as f32;
    let (mut s, mut c) = (0f64, 0f64);
    for (k, &a) in ideal.iter().enumerate() {
        let d = fmla(-step, k as f32, a);
        s += d.sin() as f64;
        c += d.cos() as f64;
    }
    let phase = if fmla(s, s, c * c) > 1e-6 * (m * m) as f64 { s.atan2(c) as f32 } else { ideal[0] };
    for k in 0..m {
        out.push(fmla(step, k as f32, phase));
    }
}

// Navi's place_disk for one component: its hub at the origin, then a ring for each BFS shell, the
// ring's nodes in the order of their parents' angles, so a branch stays together outward. A ring is
// a rest length beyond the last one, wider when its nodes need more room. Each node's angle and
// radius get a jitter from its page id, so no two start exactly symmetric (the forces could not
// tell them apart).
fn seed_component(g: &mut GraphModel, lay: &Lay, hub: usize, depth: &mut [i32], parent: &mut [u32], angle: &mut [f32]) {
    depth[hub] = 0;
    angle[hub] = 0.0;
    g.nodes[hub].x = 0.0;
    g.nodes[hub].y = 0.0;
    let mut shell: Vec<u32> = vec![hub as u32];
    let mut next: Vec<u32> = Vec::new();
    let mut ideal: Vec<f32> = Vec::new();
    let mut ang: Vec<f32> = Vec::new();
    let mut radius = 0.0f32;
    let mut prev_big = lay.eff[hub];
    loop {
        next.clear();
        for &u in &shell {
            let u = u as usize;
            for j in g.adj_at[u] as usize..g.adj_at[u + 1] as usize {
                let v = g.adj[j] as usize;
                if depth[v] < 0 {
                    depth[v] = depth[u] + 1;
                    parent[v] = u as u32;
                    next.push(v as u32);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(|&a, &b| {
            let (pa, pb) = (angle[parent[a as usize] as usize], angle[parent[b as usize] as usize]);
            pa.total_cmp(&pb).then(g.rank[a as usize].cmp(&g.rank[b as usize]))
        });
        ideal.clear();
        let mut around = 0.0f32;
        let mut big = 0.0f32;
        for &v in &next {
            ideal.push(angle[parent[v as usize] as usize]);
            around += fmla(2.0, lay.eff[v as usize], GRAPH_KNOCKOUT + SLACK);
            big = big.max(lay.eff[v as usize]);
        }
        ring_angles(&ideal, &mut ang);
        radius = (radius + GRAPH_SPRING_L).max(radius + prev_big + big + GRAPH_KNOCKOUT + SLACK).max(around / TAU);
        let slot = TAU / next.len() as f32;
        for (k, &v) in next.iter().enumerate() {
            let pid = g.nodes[v as usize].pid;
            let a = fmla(pid_jitter(pid, SALT_ANGLE), slot * 0.5, ang[k]);
            let rr = fmla(pid_jitter(pid, SALT_RADIUS), GRAPH_SPRING_L * 0.16, radius);
            angle[v as usize] = a;
            g.nodes[v as usize].x = rr * a.cos();
            g.nodes[v as usize].y = rr * a.sin();
        }
        prev_big = big;
        std::mem::swap(&mut shell, &mut next);
    }
}

// Seeds every component in its own frame, around the origin (pack puts them side by side).
fn seed_all(g: &mut GraphModel, lay: &Lay, cs: &Comps) {
    let n = g.nodes.len();
    let mut depth = vec![-1i32; n];
    let mut parent = vec![0u32; n];
    let mut angle = vec![0.0f32; n];
    for c in 0..cs.hub.len() {
        seed_component(g, lay, cs.hub[c] as usize, &mut depth, &mut parent, &mut angle);
    }
    for k in 0..g.force.len() {
        g.nodes[g.force[k] as usize].placed = true;
    }
}

// Relaxes the components that have a movable node: each step, the repulsion of the component's
// other nodes (Barnes-Hut), gravity toward the component's centre and the springs of the edges move
// the movable nodes; the others only push and pull. Components do not push each other: each is laid
// out in its own frame (or, placed already, is far enough from the rest for it not to matter).
//
// When only a few of a component's nodes move (an incremental placement), the tree of the ones that
// do not is built once for every step, and the moving ones push each other pair by pair: rebuilding
// a 2000-node tree 40 times was most of the cost of placing one page.
fn relax(g: &mut GraphModel, lay: &mut Lay, cs: &Comps, steps: i32, pump: &mut dyn FnMut()) {
    let nc = cs.hub.len();
    let mut active = vec![false; nc];
    for &v in &g.force {
        let v = v as usize;
        if lay.movable[v] {
            active[g.nodes[v].comp as usize] = true;
        }
        lay.vx[v] = 0.0;
        lay.vy[v] = 0.0;
    }
    let mut moving: Vec<Vec<u32>> = vec![Vec::new(); nc];
    let mut still: Vec<Option<Tree>> = (0..nc).map(|_| None).collect();
    for c in 0..nc {
        let m = cs.members(c);
        if !active[c] || m.len() < 2 {
            continue;
        }
        moving[c] = m.iter().copied().filter(|&v| lay.movable[v as usize]).collect();
        if moving[c].len() <= FEW_MOVING && moving[c].len() < m.len() {
            let fixed: Vec<u32> = m.iter().copied().filter(|&v| !lay.movable[v as usize]).collect();
            let mut t = Tree::default();
            t.build(g, &fixed);
            still[c] = Some(t);
        }
    }
    for step in 0..steps {
        for &v in &g.force {
            lay.fx[v as usize] = 0.0;
            lay.fy[v as usize] = 0.0;
        }
        for c in 0..nc {
            let m = cs.members(c);
            if !active[c] || m.len() < 2 {
                continue;
            }
            let (mut sx, mut sy) = (0f64, 0f64);
            for &v in m {
                sx += g.nodes[v as usize].x as f64;
                sy += g.nodes[v as usize].y as f64;
            }
            let cx = (sx / m.len() as f64) as f32;
            let cy = (sy / m.len() as f64) as f32;
            if still[c].is_none() {
                lay.tree.build(g, m);
            }
            for &v in &moving[c] {
                let (x, y) = (g.nodes[v as usize].x, g.nodes[v as usize].y);
                let (mut rx, mut ry) = match still[c].as_mut() {
                    Some(t) => t.push_at(x, y, u32::MAX),
                    None => lay.tree.push_at(x, y, v),
                };
                if still[c].is_some() {
                    for &u in &moving[c] {
                        if u != v {
                            repel(x, y, g.nodes[u as usize].x, g.nodes[u as usize].y, 1.0, &mut rx, &mut ry);
                        }
                    }
                }
                let v = v as usize;
                lay.fx[v] = fmla(-GRAPH_GRAVITY, x - cx, rx);
                lay.fy[v] = fmla(-GRAPH_GRAVITY, y - cy, ry);
            }
        }
        for e in &g.edges {
            let (a, b) = (e.a as usize, e.b as usize);
            if !lay.movable[a] && !lay.movable[b] {
                continue;
            }
            let dx = g.nodes[b].x - g.nodes[a].x;
            let dy = g.nodes[b].y - g.nodes[a].y;
            let d2 = fmla(dx, dx, dy * dy);
            if d2 < 1e-6 {
                continue; // (no direction to pull in; the repulsion parts them first)
            }
            let d = d2.sqrt();
            let f = GRAPH_SPRING_K * (d - GRAPH_SPRING_L) / d;
            lay.fx[a] = fmla(dx, f, lay.fx[a]);
            lay.fy[a] = fmla(dy, f, lay.fy[a]);
            lay.fx[b] = fmla(-dx, f, lay.fx[b]);
            lay.fy[b] = fmla(-dy, f, lay.fy[b]);
        }
        for &v in &g.force {
            let v = v as usize;
            if !lay.movable[v] {
                continue;
            }
            let mut vx = (lay.vx[v] + lay.fx[v]) * GRAPH_DAMPING;
            let mut vy = (lay.vy[v] + lay.fy[v]) * GRAPH_DAMPING;
            let s2 = fmla(vx, vx, vy * vy);
            // (a NaN or an infinity, which no input should make, stops the node instead of spreading)
            if !s2.is_finite() {
                vx = 0.0;
                vy = 0.0;
            } else if s2 > MAX_MOVE * MAX_MOVE {
                let k = MAX_MOVE / s2.sqrt();
                vx *= k;
                vy *= k;
            }
            lay.vx[v] = vx;
            lay.vy[v] = vy;
            g.nodes[v].x += vx;
            g.nodes[v].y += vy;
        }
        if (step + 1) % GRAPH_PUMP_STEPS == 0 {
            pump();
        }
    }
}

// Sets one pair apart when either is movable and their clear discs (lay.eff) come closer than the
// knockout plus `check`: to the knockout plus SLACK, the movable one going all the way, or each half
// when both are. True when it moved something.
fn push_apart(g: &mut GraphModel, lay: &Lay, i: usize, j: usize, check: f32) -> bool {
    let (mi, mj) = (lay.movable[i], lay.movable[j]);
    if !mi && !mj {
        return false;
    }
    let need = lay.eff[i] + lay.eff[j] + GRAPH_KNOCKOUT;
    let dx = g.nodes[j].x - g.nodes[i].x;
    let dy = g.nodes[j].y - g.nodes[i].y;
    let d2 = fmla(dx, dx, dy * dy);
    let lim = need + check - SLACK_EPS;
    if d2 >= lim * lim {
        return false;
    }
    let d = d2.sqrt();
    let (ux, uy) = if d > 1e-3 {
        (dx / d, dy / d)
    } else {
        // On top of each other: apart along an angle of the two page ids, the lower id one way and
        // the higher the other, whichever of them came first.
        let (pi, pj) = (g.nodes[i].pid, g.nodes[j].pid);
        let (lo, hi, sign) = if pi <= pj { (pi, pj, 1.0f32) } else { (pj, pi, -1.0f32) };
        let a = pid_unit(lo ^ hi.rotate_left(16), SALT_APART) * TAU;
        (sign * a.cos(), sign * a.sin())
    };
    let push = need + SLACK - d;
    let (si, sj) = if mi && mj {
        (0.5, 0.5)
    } else if mi {
        (1.0, 0.0)
    } else {
        (0.0, 1.0)
    };
    g.nodes[i].x = fmla(-ux, push * si, g.nodes[i].x);
    g.nodes[i].y = fmla(-uy, push * si, g.nodes[i].y);
    g.nodes[j].x = fmla(ux, push * sj, g.nodes[j].x);
    g.nodes[j].y = fmla(uy, push * sj, g.nodes[j].y);
    true
}

// Every pair of `nodes` that may lie within `cell` of each other, once each: the nodes are sorted
// into square cells of that side (by row, column, then page id, so the order never depends on the
// notebook's), and each is paired with the rest of its own cell, the cell to its right and the three
// below. `visit` may move the nodes; the cells are those of the positions at the start.
fn grid_pairs(g: &mut GraphModel, nodes: &[u32], cell: f32, cells: &mut Vec<(i32, i32, u32, u32)>, visit: &mut dyn FnMut(&mut GraphModel, usize, usize)) {
    cells.clear();
    for &v in nodes {
        let nd = &g.nodes[v as usize];
        cells.push(((nd.y / cell).floor() as i32, (nd.x / cell).floor() as i32, g.rank[v as usize], v));
    }
    cells.sort_unstable();
    for at in 0..cells.len() {
        let (cy, cx, _, i) = cells[at];
        let mut j = at + 1;
        while j < cells.len() && cells[j].0 == cy && cells[j].1 == cx {
            visit(g, i as usize, cells[j].3 as usize);
            j += 1;
        }
        for (oy, ox) in [(0, 1), (1, -1), (1, 0), (1, 1)] {
            let (ky, kx) = (cy.saturating_add(oy), cx.saturating_add(ox));
            let mut j = cells.partition_point(|c| (c.0, c.1) < (ky, kx));
            while j < cells.len() && cells[j].0 == ky && cells[j].1 == kx {
                visit(g, i as usize, cells[j].3 as usize);
                j += 1;
            }
        }
    }
}

// The grid's cell for these nodes: as wide as the widest pair of them can need.
fn pair_cell(lay: &Lay, nodes: &[u32], check: f32) -> f32 {
    let mut big = 0.0f32;
    for &v in nodes {
        big = big.max(lay.eff[v as usize]);
    }
    fmla(2.0, big, GRAPH_KNOCKOUT + check + 1.0)
}

// The collision pass: no two force nodes' clear discs closer than the knockout (plus `check`, which
// the layout sets to SLACK so the rounding for graph.ini keeps them apart, and a repair sets to 0 so
// a valid kept layout does not move). Forces alone do not promise it, and the view's knockouts need
// it. Pairs are set apart in grid order, round after round until a round moves nothing. True when it
// ends clean.
fn collide(g: &mut GraphModel, lay: &Lay, check: f32, pump: &mut dyn FnMut()) -> bool {
    let nodes: Vec<u32> = g.force.iter().map(|&v| v as u32).collect();
    let cell = pair_cell(lay, &nodes, check);
    let mut cells: Vec<(i32, i32, u32, u32)> = Vec::with_capacity(nodes.len());
    for _ in 0..COLLIDE_ROUNDS {
        let mut moved = false;
        grid_pairs(g, &nodes, cell, &mut cells, &mut |g, i, j| moved |= push_apart(g, lay, i, j, check));
        pump();
        if !moved {
            return true;
        }
    }
    false
}

// Navi's forces were made for its smaller dots: past a few hundred nodes their balance (gravity
// against the whole cloud's repulsion) packs a component tighter than dots of 14 to 30 px fit, and
// setting pairs apart one by one took the collision pass some 1500 rounds to open a 2000-node cloud
// (3 after this). So
// a component left with many touching pairs (one for every SPREAD_FEW members or more) is first
// spread about its centre, by what three quarters of those pairs need (at most SPREAD_MAX at a
// time), and measured again; the collision pass then has only the stragglers.
fn spread(g: &mut GraphModel, lay: &Lay, cs: &Comps, pump: &mut dyn FnMut()) {
    let mut cells: Vec<(i32, i32, u32, u32)> = Vec::new();
    let mut need: Vec<f32> = Vec::new();
    for c in 0..cs.hub.len() {
        let m = cs.members(c);
        if m.len() < 3 {
            continue;
        }
        let cell = pair_cell(lay, m, SLACK);
        for _ in 0..SPREAD_PASSES {
            need.clear();
            grid_pairs(g, m, cell, &mut cells, &mut |g, i, j| {
                let lim = lay.eff[i] + lay.eff[j] + GRAPH_KNOCKOUT + SLACK;
                let dx = g.nodes[j].x - g.nodes[i].x;
                let dy = g.nodes[j].y - g.nodes[i].y;
                let d = fmla(dx, dx, dy * dy).sqrt();
                if d < lim - SLACK_EPS {
                    need.push(lim / d.max(1.0));
                }
            });
            if need.len() * SPREAD_FEW < m.len() {
                break;
            }
            need.sort_by(|a, b| a.total_cmp(b));
            let s = need[need.len() * 3 / 4].clamp(1.0, SPREAD_MAX);
            let (mut sx, mut sy) = (0f64, 0f64);
            for &v in m {
                sx += g.nodes[v as usize].x as f64;
                sy += g.nodes[v as usize].y as f64;
            }
            let cx = (sx / m.len() as f64) as f32;
            let cy = (sy / m.len() as f64) as f32;
            for &v in m {
                let nd = &mut g.nodes[v as usize];
                nd.x = fmla(nd.x - cx, s, cx);
                nd.y = fmla(nd.y - cy, s, cy);
            }
            pump();
        }
    }
}

// Puts the components side by side: the widest in the middle, the others on rings around it, each
// ring as deep as its first (widest) disc, each disc in its own slice of the ring with half a gap on
// either side. Discs are each component's centroid and reach (lay.eff included), so no component
// touches another.
fn pack(g: &mut GraphModel, lay: &Lay, cs: &Comps) {
    let nc = cs.hub.len();
    let mut discs: Vec<(usize, f32, f32, f32)> = Vec::with_capacity(nc);
    for c in 0..nc {
        let m = cs.members(c);
        let (mut sx, mut sy) = (0f64, 0f64);
        for &v in m {
            sx += g.nodes[v as usize].x as f64;
            sy += g.nodes[v as usize].y as f64;
        }
        let cx = (sx / m.len() as f64) as f32;
        let cy = (sy / m.len() as f64) as f32;
        let mut r = 0.0f32;
        for &v in m {
            let nd = &g.nodes[v as usize];
            let (dx, dy) = (nd.x - cx, nd.y - cy);
            r = r.max(fmla(dx, dx, dy * dy).sqrt() + lay.eff[v as usize]);
        }
        discs.push((c, cx, cy, r));
    }
    // (components are numbered from their lowest page id, so equal discs keep a page-id order)
    discs.sort_by(|a, b| b.3.total_cmp(&a.3).then(a.0.cmp(&b.0)));
    let mut inner = 0.0f32;
    let mut band = 0.0f32;
    let mut ang = 0.0f32;
    let mut open = false;
    for (k, &(c, cx, cy, r)) in discs.iter().enumerate() {
        let (tx, ty) = if k == 0 {
            inner = r + COMP_GAP;
            (0.0, 0.0)
        } else {
            loop {
                let first = !open;
                if first {
                    band = r;
                    ang = 0.0;
                    open = true;
                }
                let mid = inner + band;
                let half = (fmla(COMP_GAP, 0.5, r) / mid).min(1.0).asin();
                if first || fmla(2.0, half, ang) <= TAU {
                    let a = ang + half;
                    ang = fmla(2.0, half, ang);
                    break (mid * a.cos(), mid * a.sin());
                }
                inner = fmla(2.0, band, inner + COMP_GAP);
                open = false;
            }
        };
        let (ox, oy) = (tx - cx, ty - cy);
        for &v in cs.members(c) {
            g.nodes[v as usize].x += ox;
            g.nodes[v as usize].y += oy;
        }
    }
}

// Whole world units, as graph.ini keeps them: what the view shows is what the next visit loads,
// bit for bit. (Adding 0.0 turns the -0.0 that rounding -0.3 gives into the 0.0 that reading "0"
// back gives.)
fn snap1(x: f32) -> f32 {
    if x.is_finite() {
        x.round().clamp(-GRAPH_POS_MAX as f32, GRAPH_POS_MAX as f32) + 0.0
    } else {
        0.0
    }
}

fn snap(g: &mut GraphModel, lay: &Lay) {
    for k in 0..g.force.len() {
        let v = g.force[k] as usize;
        if lay.movable[v] {
            g.nodes[v].x = snap1(g.nodes[v].x);
            g.nodes[v].y = snap1(g.nodes[v].y);
        }
    }
}

/// The seed alone: place_disk for each component, the components packed, overlaps set apart. It is
/// O(n + e) but for the collision pass, and is what the view shows while a bgjob child runs the
/// full layout (graph_layout_job), so it runs in the app's process: pump is called between its
/// stages and each collision round. Satellites follow their heads; the tray is the view's.
pub fn graph_seed(g: &mut GraphModel, pump: &mut dyn FnMut()) {
    let cs = comps_of(g);
    let mut lay = Lay::new(g);
    for &v in &g.force {
        lay.movable[v as usize] = true;
    }
    seed_all(g, &lay, &cs);
    pump();
    pack(g, &lay, &cs);
    collide(g, &lay, SLACK, pump);
    snap(g, &lay);
    place_satellites(g);
    g.laid = GraphLaid::Seed;
}

/// The full layout: the seed, GRAPH_STEPS relaxation steps (pump every GRAPH_PUMP_STEPS), the
/// components packed, overlaps set apart, positions rounded to whole units, then the satellites.
/// Every kept position is replaced. Run it in-process only when graph_layout_in_process says so.
pub fn graph_layout(g: &mut GraphModel, pump: &mut dyn FnMut()) {
    let cs = comps_of(g);
    let mut lay = Lay::new(g);
    for &v in &g.force {
        lay.movable[v as usize] = true;
    }
    seed_all(g, &lay, &cs);
    relax(g, &mut lay, &cs, GRAPH_STEPS, pump);
    spread(g, &lay, &cs, pump);
    pack(g, &lay, &cs);
    collide(g, &lay, SLACK, pump);
    snap(g, &lay);
    place_satellites(g);
    g.laid = GraphLaid::Done;
}

// The centre of the placed force nodes, and how far they reach from it (their clear discs
// included); the origin and 0 when none is placed.
fn placed_disc(g: &GraphModel, lay: &Lay) -> (f32, f32, f32) {
    let (mut sx, mut sy, mut k) = (0f64, 0f64, 0usize);
    for &v in &g.force {
        let nd = &g.nodes[v as usize];
        if nd.placed {
            sx += nd.x as f64;
            sy += nd.y as f64;
            k += 1;
        }
    }
    if k == 0 {
        return (0.0, 0.0, 0.0);
    }
    let (cx, cy) = ((sx / k as f64) as f32, (sy / k as f64) as f32);
    let mut r = 0.0f32;
    for &v in &g.force {
        let nd = &g.nodes[v as usize];
        if nd.placed {
            let (dx, dy) = (nd.x - cx, nd.y - cy);
            r = r.max(fmla(dx, dx, dy * dy).sqrt() + lay.eff[v as usize]);
        }
    }
    (cx, cy, r)
}

// Gives each unplaced force node a first position. A node with placed neighbours goes one rest
// length beyond their centroid, away from the map's centre (so a new leaf grows outward), turned a
// little by its page id; nodes are taken in page-id order, round after round, so a chain of new
// pages grows from the placed end. A node with no placed neighbour goes beside its section's named
// page (for a named page, the section it was cut from: the page before it), or failing that just
// outside everything placed, at an angle of its page id. Its own neighbours can then follow it.
fn place_fresh(g: &mut GraphModel, lay: &Lay, fresh: &[usize]) {
    loop {
        loop {
            let (cx, cy, _) = placed_disc(g, lay);
            let mut any = false;
            for &v in fresh {
                if g.nodes[v].placed {
                    continue;
                }
                let (mut sx, mut sy, mut k) = (0f64, 0f64, 0usize);
                for j in g.adj_at[v] as usize..g.adj_at[v + 1] as usize {
                    let u = &g.nodes[g.adj[j] as usize];
                    if u.placed {
                        sx += u.x as f64;
                        sy += u.y as f64;
                        k += 1;
                    }
                }
                if k == 0 {
                    continue;
                }
                let (nx, ny) = ((sx / k as f64) as f32, (sy / k as f64) as f32);
                let pid = g.nodes[v].pid;
                let (dx, dy) = (nx - cx, ny - cy);
                let base = if fmla(dx, dx, dy * dy) >= 1.0 { dy.atan2(dx) } else { fmla(pid_unit(pid, SALT_SPOT), TAU, -PI) };
                let a = fmla(pid_jitter(pid, SALT_TURN), 0.6, base);
                g.nodes[v].x = fmla(GRAPH_SPRING_L, a.cos(), nx);
                g.nodes[v].y = fmla(GRAPH_SPRING_L, a.sin(), ny);
                g.nodes[v].placed = true;
                any = true;
            }
            if !any {
                break;
            }
        }
        let Some(&v) = fresh.iter().find(|&&v| !g.nodes[v].placed) else { break };
        let s = g.nodes[v].section;
        let head = if s == v as i32 {
            if v > 0 {
                g.nodes[v - 1].section
            } else {
                -1
            }
        } else {
            s
        };
        let pid = g.nodes[v].pid;
        let a = pid_unit(pid, SALT_SPOT) * TAU;
        if head >= 0 && g.nodes[head as usize].placed {
            let h = head as usize;
            let dist = fmla(GRAPH_SPRING_L, 0.5, lay.eff[h] + lay.eff[v]);
            g.nodes[v].x = fmla(dist, a.cos(), g.nodes[h].x);
            g.nodes[v].y = fmla(dist, a.sin(), g.nodes[h].y);
        } else {
            let (cx, cy, r) = placed_disc(g, lay);
            let dist = if r > 0.0 { r + lay.eff[v] + COMP_GAP } else { 0.0 };
            g.nodes[v].x = fmla(dist, a.cos(), cx);
            g.nodes[v].y = fmla(dist, a.sin(), cy);
        }
        g.nodes[v].placed = true;
    }
}

/// Places the force nodes that have no position yet (new pages, and the open page), keeping every
/// other: each new one goes beside its placed neighbours, or beside its section's named page,
/// then GRAPH_INC_STEPS steps run over the new pages' 2-hop neighbourhood with everything else
/// fixed, and overlaps are set apart by moving only that neighbourhood. Last, a repair: dots that
/// came to overlap while nothing moved (a page that got a name is bigger, and its satellites need
/// room) are set apart, and only those move. A layout that was clean is left bit for bit as it was
/// outside the new pages' neighbourhood. Satellites follow their heads. Returns how many it placed.
pub fn graph_place_new(g: &mut GraphModel, pump: &mut dyn FnMut()) -> i32 {
    let cs = comps_of(g);
    let mut lay = Lay::new(g);
    let fresh: Vec<usize> = g.force.iter().map(|&v| v as usize).filter(|&v| !g.nodes[v].placed).collect();
    if !fresh.is_empty() {
        place_fresh(g, &lay, &fresh);
        for &v in &fresh {
            lay.movable[v] = true;
            for j in g.adj_at[v] as usize..g.adj_at[v + 1] as usize {
                let u = g.adj[j] as usize;
                lay.movable[u] = true;
                for k in g.adj_at[u] as usize..g.adj_at[u + 1] as usize {
                    lay.movable[g.adj[k] as usize] = true;
                }
            }
        }
        relax(g, &mut lay, &cs, GRAPH_INC_STEPS, pump);
        collide(g, &lay, SLACK, pump);
    }
    for &v in &g.force {
        lay.movable[v as usize] = true;
    }
    collide(g, &lay, 0.0, pump);
    snap(g, &lay);
    place_satellites(g);
    g.laid = GraphLaid::Done;
    fresh.len() as i32
}

/// Puts graph.ini's kept positions on the force nodes it keeps (graph_persisted), and says what
/// is left to do. The gen is not needed for that: the positions say which pages are new. A gen
/// that differs while no page is new means links changed between pages that kept their places,
/// and those stay put (nothing moves between visits); graph_ini_store then writes the new gen.
pub fn graph_apply_ini(g: &mut GraphModel, ini: &GraphIni) -> GraphPlan {
    let mut kept = 0;
    let mut fresh = 0;
    for k in 0..g.force.len() {
        let v = g.force[k] as usize;
        if !graph_persisted(&g.nodes[v]) {
            continue;
        }
        kept += 1;
        let pid = g.nodes[v].pid;
        match ini.pos.binary_search_by_key(&pid, |p| p.pid) {
            Ok(at) => {
                g.nodes[v].x = ini.pos[at].x as f32;
                g.nodes[v].y = ini.pos[at].y as f32;
                g.nodes[v].placed = true;
            }
            Err(_) => {
                g.nodes[v].placed = false;
                fresh += 1;
            }
        }
    }
    if fresh == 0 {
        GraphPlan::Ready
    } else if graph_relayout_needed(fresh, kept) {
        GraphPlan::Full
    } else {
        GraphPlan::Incremental
    }
}

/// The map's positions and gen into ini, to write: the placed nodes graph_persisted keeps, in
/// whole units. Mode, unlinked, pins and unknown lines stay as they are. True when anything
/// changed, so the caller writes only then.
pub fn graph_ini_store(ini: &mut GraphIni, g: &GraphModel) -> bool {
    let gen = graph_gen(g);
    let mut pos: Vec<GraphPos> = Vec::with_capacity(g.force.len());
    for &v in &g.force {
        let nd = &g.nodes[v as usize];
        if graph_persisted(nd) && nd.placed {
            pos.push(GraphPos { pid: nd.pid, x: snap1(nd.x) as i32, y: snap1(nd.y) as i32 });
        }
    }
    let changed = !ini.has_gen || ini.gen != gen || ini.pos != pos;
    ini.gen = gen;
    ini.has_gen = true;
    ini.pos = pos;
    changed
}

/// The work of a bgjob child for a big or Tidy-up layout, on a model the child built itself
/// (store_load_info, nav_load, graph_build; the open page does not matter, as it is not kept) and
/// the graph.ini it read (for its mode, unlinked choice, pins and carried lines, and to honour a
/// refusal): the full layout, unpumped, then graph.ini written whole and atomically at path. 0, or
/// -1 when the write failed or the file was refused.
pub fn graph_layout_job(g: &mut GraphModel, ini: &mut GraphIni, path: &str) -> i32 {
    graph_layout(g, &mut || {});
    graph_ini_store(ini, g);
    graph_ini_write_path(path, ini)
}

// ---- Around: the rings of one page (Navi's positions_radial) ----

// The radius of a ring of nodes as big as `big` at most, beyond a ring (or the centre) at `inner`
// whose nodes are as big as `inner_big`: a rest length out at least, clear of the inner nodes, and
// wide enough that neighbours on the ring keep a knockout between them.
fn ring_radius(inner: f32, inner_big: f32, big: f32, m: usize, min_gap: f32) -> f32 {
    let mut r = (inner + min_gap).max(inner + inner_big + big + GRAPH_KNOCKOUT + SLACK);
    if m >= 2 {
        let chord = fmla(2.0, big, GRAPH_KNOCKOUT + SLACK);
        r = r.max(chord / (2.0 * (PI / m as f32).sin()));
    }
    r
}

/// The Around layout of one page: `centre` in the middle, the pages linked with it on a ring, and
/// when depth is 2 the pages linked with those on an outer ring, each ring in the order of the
/// parents' angles (the first ring in the order its pages lie around the centre in the map, or by
/// page id when the map is not laid out). Only members are listed, the centre first, then each
/// ring in its order; returns how many. The view draws the edges between members. Instant and
/// deterministic: no forces.
pub fn graph_around(g: &GraphModel, centre: i32, depth: i32, out: &mut Vec<GraphRing>) -> i32 {
    out.clear();
    let n = g.nodes.len();
    if centre < 0 || centre as usize >= n || g.adj_at.len() != n + 1 {
        return 0;
    }
    let c = centre as usize;
    let mut seen = vec![false; n];
    seen[c] = true;
    out.push(GraphRing { node: centre, depth: 0, parent: -1, x: 0.0, y: 0.0 });
    let cn = g.nodes[c];
    let mut ring1: Vec<(f32, u32, usize)> = Vec::new();
    for &v in graph_neighbours(g, centre) {
        let v = v as usize;
        seen[v] = true;
        let nd = &g.nodes[v];
        let (dx, dy) = (nd.x - cn.x, nd.y - cn.y);
        let ideal = if cn.placed && nd.placed && fmla(dx, dx, dy * dy) > 0.0 {
            dy.atan2(dx)
        } else {
            fmla(pid_unit(nd.pid, SALT_AROUND), TAU, -PI)
        };
        ring1.push((ideal, g.rank[v], v));
    }
    if ring1.is_empty() {
        return 1;
    }
    ring1.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let ideal: Vec<f32> = ring1.iter().map(|e| e.0).collect();
    let mut ang1: Vec<f32> = Vec::new();
    ring_angles(&ideal, &mut ang1);
    let big1 = ring1.iter().map(|e| g.nodes[e.2].r).fold(0.0f32, f32::max);
    let r1 = ring_radius(0.0, cn.r, big1, ring1.len(), GRAPH_SPRING_L);
    for (k, e) in ring1.iter().enumerate() {
        out.push(GraphRing { node: e.2 as i32, depth: 1, parent: centre, x: r1 * ang1[k].cos(), y: r1 * ang1[k].sin() });
    }
    if depth < 2 {
        return out.len() as i32;
    }
    // The outer ring: met from the first ring in its order, each parent's pages by page id.
    let mut ring2: Vec<(usize, usize)> = Vec::new();
    for (k, e) in ring1.iter().enumerate() {
        for &v in graph_neighbours(g, e.2 as i32) {
            let v = v as usize;
            if !seen[v] {
                seen[v] = true;
                ring2.push((k, v));
            }
        }
    }
    if ring2.is_empty() {
        return out.len() as i32;
    }
    let ideal: Vec<f32> = ring2.iter().map(|e| ang1[e.0]).collect();
    let mut ang2: Vec<f32> = Vec::new();
    ring_angles(&ideal, &mut ang2);
    let big2 = ring2.iter().map(|e| g.nodes[e.1].r).fold(0.0f32, f32::max);
    let r2 = ring_radius(r1, big1, big2, ring2.len(), GRAPH_SPRING_L);
    for (k, e) in ring2.iter().enumerate() {
        out.push(GraphRing {
            node: e.1 as i32,
            depth: 2,
            parent: ring1[e.0].2 as i32,
            x: r2 * ang2[k].cos(),
            y: r2 * ang2[k].sin(),
        });
    }
    out.len() as i32
}

// ---- graph.ini: reading and writing ----

/// The path of notebook id's graph.ini (root/notebooks/<id>/graph.ini): 0, or -1 for an id that is
/// no notebook's.
pub fn graph_ini_path(id: &str, out: &mut String) -> i32 {
    let mut dir = String::new();
    if store_notebook_dir(id, &mut dir, 600) != 0 {
        return -1;
    }
    let len = csnprintf!(out, 700, "%s/graph.ini", &dir);
    if len < 0 || len as usize >= 700 {
        return -1;
    }
    0
}

// The file's bytes: Ok(None) when there is no file; Err when it is there and does not read (it
// cannot be opened or read, it is a symlink or not a regular file, it is over GRAPH_FILE_MAX, or it
// holds a NUL), as nav.rs reads its sidecars.
fn read_text(path: &str) -> Result<Option<Vec<u8>>, ()> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let f = std::fs::OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW).open(path);
    let f = match f {
        Ok(f) => f,
        Err(e) => {
            // Only a file that is not there is a missing cache (a dangling symlink is still a link).
            let absent = e.kind() == std::io::ErrorKind::NotFound
                && matches!(std::fs::symlink_metadata(path), Err(m) if m.kind() == std::io::ErrorKind::NotFound);
            return if absent { Ok(None) } else { Err(()) };
        }
    };
    let meta = f.metadata().map_err(|_| ())?;
    if !meta.file_type().is_file() || meta.len() > GRAPH_FILE_MAX as u64 {
        return Err(());
    }
    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve_exact(meta.len() as usize + 1).is_err() {
        return Err(());
    }
    f.take(GRAPH_FILE_MAX as u64 + 1).read_to_end(&mut buf).map_err(|_| ())?;
    if buf.len() > GRAPH_FILE_MAX || buf.contains(&0) {
        return Err(()); // never rewrite a file with a hidden binary tail
    }
    Ok(Some(buf))
}

// A decimal u32 at the start of s (at least one digit): its value and its length.
fn dec_u32(s: &[u8]) -> Option<(u32, usize)> {
    let mut v: u64 = 0;
    let mut k = 0;
    while k < s.len() && s[k].is_ascii_digit() {
        v = v * 10 + (s[k] - b'0') as u64;
        if v > u32::MAX as u64 {
            return None;
        }
        k += 1;
    }
    if k == 0 {
        None
    } else {
        Some((v as u32, k))
    }
}

// A coordinate at the start of s: an optional '-', then digits, within +-GRAPH_POS_MAX.
fn dec_coord(s: &[u8]) -> Option<(i32, usize)> {
    let neg = s.first() == Some(&b'-');
    let at = neg as usize;
    let mut v: i64 = 0;
    let mut k = at;
    while k < s.len() && s[k].is_ascii_digit() {
        v = v * 10 + (s[k] - b'0') as i64;
        if v > GRAPH_POS_MAX as i64 {
            return None;
        }
        k += 1;
    }
    if k == at {
        return None;
    }
    Some((if neg { -v } else { v } as i32, k))
}

// gen=<exactly 8 hex digits, either case>
fn hex8(v: &[u8]) -> Option<u32> {
    if v.len() != 8 {
        return None;
    }
    let mut h: u32 = 0;
    for &c in v {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => return None,
        };
        h = h << 4 | d as u32;
    }
    Some(h)
}

// pos.<pid>=<x>,<y>, nothing after
fn pos_parse(line: &[u8]) -> Option<GraphPos> {
    let rest = line.strip_prefix(b"pos.")?;
    let (pid, used) = dec_u32(rest)?;
    let rest = rest[used..].strip_prefix(b"=")?;
    let (x, used) = dec_coord(rest)?;
    let rest = rest[used..].strip_prefix(b",")?;
    let (y, used) = dec_coord(rest)?;
    if pid == 0 || used != rest.len() {
        return None;
    }
    Some(GraphPos { pid, x, y })
}

// pin.<pid>=1
fn pin_parse(line: &[u8]) -> Option<u32> {
    let rest = line.strip_prefix(b"pin.")?;
    let (pid, used) = dec_u32(rest)?;
    (pid != 0 && &rest[used..] == b"=1").then_some(pid)
}

// Keeps a line this version does not read, to write back: -1 past the limits (the file is then
// refused, as nav.ini is past its own).
fn carry(buf: &mut Vec<u8>, line: &[u8]) -> i32 {
    if line.len() > GRAPH_LINE_MAX || buf.len() + line.len() + 1 > GRAPH_UNKNOWN_MAX {
        return -1;
    }
    buf.extend_from_slice(line);
    buf.push(b'\n');
    0
}

fn ini_line(ini: &mut GraphIni, line: &[u8], pos: &mut Vec<GraphPos>, pins: &mut Vec<u32>) -> i32 {
    if line.is_empty() || line[0] == b'#' {
        return 0; // (the header is written afresh; comments are not kept)
    }
    if let Some(h) = line.strip_prefix(b"gen=").and_then(hex8) {
        ini.gen = h;
        ini.has_gen = true;
        return 0;
    }
    match line {
        b"mode=all" => ini.mode = GraphMode::All,
        b"mode=around" => ini.mode = GraphMode::Around,
        b"unlinked=show" => ini.show_unlinked = true,
        b"unlinked=hide" => ini.show_unlinked = false,
        _ => {
            if let Some(p) = pos_parse(line) {
                pos.push(p);
            } else if let Some(pid) = pin_parse(line) {
                pins.push(pid);
            } else {
                return carry(&mut ini.unknown, line);
            }
        }
    }
    0
}

fn ini_parse(text: &[u8], ini: &mut GraphIni) -> i32 {
    let mut pos: Vec<GraphPos> = Vec::new();
    let mut pins: Vec<u32> = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let end = text[at..].iter().position(|&c| c == b'\n').map(|k| at + k);
        let mut line = &text[at..end.unwrap_or(text.len())];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        if ini_line(ini, line, &mut pos, &mut pins) != 0 {
            return -1;
        }
        match end {
            Some(e) => at = e + 1,
            None => break,
        }
    }
    // The last line for a page wins: a stable sort keeps the lines' order among equals.
    pos.sort_by_key(|p| p.pid);
    let mut one: Vec<GraphPos> = Vec::with_capacity(pos.len());
    for p in pos {
        match one.last_mut() {
            Some(l) if l.pid == p.pid => *l = p,
            _ => one.push(p),
        }
    }
    pins.sort_unstable();
    pins.dedup();
    // More pages than a notebook can have: not a layout this version wrote.
    if one.len() > STORE_MAX_PAGES || pins.len() > STORE_MAX_PAGES {
        return -1;
    }
    ini.pos = one;
    ini.pins = pins;
    0
}

fn refuse(ini: &mut GraphIni) -> i32 {
    *ini = GraphIni { writable: false, file: true, ..GraphIni::default() };
    qlog!("index map: graph.ini could not be read; left unchanged");
    -1
}

/// Reads graph.ini at path into ini. A missing file is a fresh cache: the defaults, writable, 0.
/// A file that does not read is refused: ini gets the defaults with writable false (so this
/// session never overwrites it), -1. It does not read when it cannot be opened or read, is a
/// symlink or not a regular file, is over GRAPH_FILE_MAX or holds a NUL, when a line this version
/// does not read is over GRAPH_LINE_MAX or the carried lines would pass GRAPH_UNKNOWN_MAX, or when
/// it keeps more than STORE_MAX_PAGES positions or pins. A line that does not parse (a pos with
/// letters, mode=local) is not damage: it is carried as an unknown line, a newer version's perhaps.
/// The last line for a key wins. Lines may end in CRLF.
pub fn graph_ini_read_path(path: &str, ini: &mut GraphIni) -> i32 {
    match read_text(path) {
        Ok(None) => {
            *ini = GraphIni::default();
            0
        }
        Ok(Some(text)) => {
            let mut fresh = GraphIni { file: true, ..GraphIni::default() };
            if ini_parse(&text, &mut fresh) != 0 {
                return refuse(ini);
            }
            *ini = fresh;
            0
        }
        Err(()) => refuse(ini),
    }
}

// graph.ini's bytes: the header, gen (once there is one), mode, unlinked, the positions by page
// id, the pins, then the carried lines as read. None past GRAPH_FILE_MAX.
fn ini_text(ini: &GraphIni) -> Option<Vec<u8>> {
    let mut t: Vec<u8> = Vec::new();
    t.extend_from_slice(INI_HEADER);
    if ini.has_gen {
        t.extend_from_slice(cformat!("gen=%08x\n", ini.gen).as_bytes());
    }
    t.extend_from_slice(if ini.mode == GraphMode::Around { b"mode=around\n" } else { b"mode=all\n" });
    t.extend_from_slice(if ini.show_unlinked { b"unlinked=show\n" } else { b"unlinked=hide\n" });
    for p in &ini.pos {
        t.extend_from_slice(cformat!("pos.%u=%d,%d\n", p.pid, p.x, p.y).as_bytes());
    }
    for &pid in &ini.pins {
        t.extend_from_slice(cformat!("pin.%u=1\n", pid).as_bytes());
    }
    t.extend_from_slice(&ini.unknown);
    (t.len() <= GRAPH_FILE_MAX).then_some(t)
}

/// Writes ini to path, whole and atomically (write_file_atomic). -1, writing nothing, when ini is
/// not writable (its file was refused this session) or would pass GRAPH_FILE_MAX; -1 when the
/// write fails.
pub fn graph_ini_write_path(path: &str, ini: &mut GraphIni) -> i32 {
    if !ini.writable {
        return -1;
    }
    let Some(text) = ini_text(ini) else { return -1 };
    let rc = write_file_atomic(path, &text);
    if rc == 0 {
        ini.file = true;
    }
    rc
}

/// graph_ini_read_path on notebook id's graph.ini (an id that is no notebook's is refused).
pub fn graph_ini_read(id: &str, ini: &mut GraphIni) -> i32 {
    let mut path = String::new();
    if graph_ini_path(id, &mut path) != 0 {
        return refuse(ini);
    }
    graph_ini_read_path(&path, ini)
}

/// graph_ini_write_path on notebook id's graph.ini.
pub fn graph_ini_write(id: &str, ini: &mut GraphIni) -> i32 {
    let mut path = String::new();
    if graph_ini_path(id, &mut path) != 0 {
        return -1;
    }
    graph_ini_write_path(&path, ini)
}
