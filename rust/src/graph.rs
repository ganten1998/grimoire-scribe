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
