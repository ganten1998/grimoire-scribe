// test_graph: the Index map's model, layout and graph.ini (src/graph.rs). The layout's floats differ
// between arm64 and armv7, so these check invariants (bands, degrees, no overlaps, rings, what moves
// and what does not), never coordinates. Bit-identical comparisons are within one run, on one machine.
#![allow(clippy::needless_range_loop, clippy::field_reassign_with_default)]
mod graph_corpus;
mod testlib;
use graph_corpus::*;
use grimoire::prelude::*;
use std::sync::OnceLock;
use testlib::*;

// The scratch folder: under GRIMOIRE_TEST_TMP when set, else the system's.
static TMP: OnceLock<String> = OnceLock::new();
fn tmp() -> &'static str {
    TMP.get_or_init(|| {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let d = base.join(format!("grimoire-test-graph-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch folder");
        d.to_string_lossy().into_owned()
    })
}
fn tmp_path(name: &str) -> String {
    format!("{}/{}", tmp(), name)
}

fn built(b: &Book, open: u32) -> GraphModel {
    let mut g = GraphModel::default();
    check!(graph_build(&mut g, &b.nb, &b.nav, open, false) == 0);
    g
}

// The view's model: linked pages only (mapview and the bgjob child build this one).
fn linked_built(b: &Book, open: u32) -> GraphModel {
    let mut g = GraphModel::default();
    check!(graph_build(&mut g, &b.nb, &b.nav, open, true) == 0);
    g
}

// What the view does on opening a map with a kept layout (mapview's layout, in-process): the kept
// positions, the new pages placed, a new link's stretched edge relaxed, and a full layout from the
// kept positions when either asks for one. Returns the plan it ended with.
fn reopen(g: &mut GraphModel, ini: &GraphIni) -> GraphPlan {
    let mut plan = graph_apply_ini(g, ini);
    if plan != GraphPlan::Full {
        check!(graph_place_new(g, &mut || {}) >= 0);
        if ini.has_gen && ini.gen != graph_gen(g) && graph_relax_stressed(g, &mut || {}) == -2 {
            plan = GraphPlan::Full;
        }
    }
    if plan == GraphPlan::Full {
        check!(graph_layout_warm(g, &mut || {}));
    }
    plan
}

fn moved(a: &GraphModel, b: &GraphModel, pid: u32) -> f32 {
    dist(node(a, pid), node(b, pid))
}

fn node(g: &GraphModel, pid: u32) -> &GraphNode {
    &g.nodes[graph_node_of(g, pid) as usize]
}

fn dist(a: &GraphNode, b: &GraphNode) -> f32 {
    ((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)).sqrt()
}

// The dots the view draws: the force nodes and the satellites (those past GRAPH_SAT_DRAWN only from
// 2x zoom, but they keep the rule too). The tray is lined up by the view, outside the world.
fn dots(g: &GraphModel) -> Vec<usize> {
    (0..g.nodes.len()).filter(|&i| g.nodes[i].placed && g.nodes[i].tier != GraphTier::Tray).collect()
}

// Pairs of dots that reach into each other's knockout (closer than r_a + r_b + 4).
fn overlaps(g: &GraphModel) -> usize {
    let d = dots(g);
    let mut bad = 0;
    for x in 0..d.len() {
        for y in x + 1..d.len() {
            let (a, b) = (&g.nodes[d[x]], &g.nodes[d[y]]);
            if dist(a, b) < a.r + b.r + GRAPH_KNOCKOUT - 1e-3 {
                bad += 1;
            }
        }
    }
    bad
}

fn all_finite(g: &GraphModel) -> bool {
    g.nodes.iter().all(|n| !n.placed || (n.x.is_finite() && n.y.is_finite())) && g.nodes.iter().all(|n| n.r.is_finite() && n.r > 0.0)
}

// The same nodes placed at the same bits (one run, one machine).
fn same_places(a: &GraphModel, b: &GraphModel) -> bool {
    a.nodes.len() == b.nodes.len()
        && a.nodes.iter().zip(&b.nodes).all(|(x, y)| x.placed == y.placed && x.x.to_bits() == y.x.to_bits() && x.y.to_bits() == y.y.to_bits())
}

fn force_placed(g: &GraphModel) -> bool {
    g.force.iter().all(|&v| g.nodes[v as usize].placed)
}

fn laid_out(b: &Book, open: u32) -> GraphModel {
    let mut g = built(b, open);
    graph_layout(&mut g, &mut || {});
    g
}

// FNV-1a 32, written out again: the gen's encoding is the contract, so the test spells it.
fn fnv(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in bytes {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

// ---- the model ----

// pages, notebook order:
//   13 12 | 20* 21 22 23 24 25 | 30* 31 32 | 40* 41 50 | 10 11      (* named)
// 21 -> 30 twice, 30 -> 21: one edge, both ways, 3 links. 22 -> the Index twice: a badge. 40 -> 20.
// 41 -> p999 (no such page) and 12 -> q7 (a target from a newer version): ignored. 50 -> 50: a loop.
// 23 -> 10, and 10 sits after the last named page. 11 is open.
fn fixture() -> Book {
    let mut b = book(&[13, 12, 20, 21, 22, 23, 24, 25, 30, 31, 32, 40, 41, 50, 10, 11]);
    name(&mut b, 20, "Alpha");
    name(&mut b, 30, "Beta");
    name(&mut b, 40, "Gamma");
    check!(link(&mut b, 21, 1, 30) == 0);
    check!(link(&mut b, 21, 2, 30) == 0);
    check!(link(&mut b, 30, 3, 21) == 0);
    check!(link_index(&mut b, 22, 4) == 0);
    check!(link_index(&mut b, 22, 5) == 0);
    check!(link(&mut b, 40, 6, 20) == 0);
    check!(link(&mut b, 41, 7, 999) == 0);
    link_raw(&mut b, 12, 8, "q7");
    check!(link(&mut b, 50, 9, 50) == 0);
    check!(link(&mut b, 23, 10, 10) == 0);
    b
}

fn bands_and_degrees() {
    let b = fixture();
    let g = built(&b, 11);
    check!(g.nodes.len() == 16);
    for (i, &pid) in b.nb.page_ids.iter().enumerate() {
        check!(graph_node_of(&g, pid) == i as i32);
        check!(g.nodes[i].pid == pid);
        // each node's section is nav_section_of's
        check!(g.nodes[i].section == nav_section_of(&b.nav, &b.nb, i as i32));
    }
    check!(graph_node_of(&g, 999) == -1 && graph_node_of(&g, 0) == -1);
    let tier = |pid| node(&g, pid).tier;
    for pid in [20, 21, 22, 23, 30, 40, 50, 10, 11] {
        check!(tier(pid) == GraphTier::Force);
    }
    for pid in [24, 25, 31, 32, 41] {
        check!(tier(pid) == GraphTier::Satellite);
    }
    check!(tier(13) == GraphTier::Tray && tier(12) == GraphTier::Tray);
    // the tray in notebook order (13 before 12), not page-id order
    check!(g.tray.len() == 2 && g.tray[0] == graph_node_of(&g, 13) && g.tray[1] == graph_node_of(&g, 12));
    // the force nodes in page-id order
    let force: Vec<u32> = g.force.iter().map(|&v| g.nodes[v as usize].pid).collect();
    check!(force == vec![10, 11, 20, 21, 22, 23, 30, 40, 50]);
    // satellites hang on their section's named page, in notebook order
    let n20 = graph_node_of(&g, 20);
    let n30 = graph_node_of(&g, 30);
    let n40 = graph_node_of(&g, 40);
    check!(node(&g, 24).section == n20 && node(&g, 24).sat_rank == 0);
    check!(node(&g, 25).section == n20 && node(&g, 25).sat_rank == 1);
    check!(node(&g, 31).section == n30 && node(&g, 31).sat_rank == 0);
    check!(node(&g, 32).section == n30 && node(&g, 32).sat_rank == 1);
    check!(node(&g, 41).section == n40 && node(&g, 41).sat_rank == 0);
    // (the open page 11, plain, keeps its slot on section 40's spiral, empty while it is open)
    check!(node(&g, 20).sats == 2 && node(&g, 30).sats == 2 && node(&g, 40).sats == 2);
    check!(node(&g, 11).section == n40 && node(&g, 11).sat_rank == 1 && node(&g, 11).tier == GraphTier::Force);
    check!(node(&g, 10).section == n40 && node(&g, 10).sats == 0 && node(&g, 10).sat_rank == -1);
    check!(graph_sat_more(&g, n40) == 0);
    check!(node(&g, 13).section == -1 && node(&g, 13).sat_rank == -1);
    // degrees: distinct linked pages, either way
    check!(node(&g, 21).degree == 1 && node(&g, 30).degree == 1);
    check!(node(&g, 20).degree == 1 && node(&g, 40).degree == 1);
    check!(node(&g, 23).degree == 1 && node(&g, 10).degree == 1);
    check!(node(&g, 22).degree == 0 && node(&g, 50).degree == 0 && node(&g, 11).degree == 0);
    check!(g.dmax == 1);
    check!(node(&g, 21).links_to == 1 && node(&g, 21).linked_from == 1);
    check!(node(&g, 40).links_to == 1 && node(&g, 40).linked_from == 0);
    check!(node(&g, 20).links_to == 0 && node(&g, 20).linked_from == 1);
    // the Index: a badge, never an edge
    check!(node(&g, 22).index_links == 2 && node(&g, 22).linked);
    check!(g.edges.iter().all(|e| e.a != graph_node_of(&g, 22) && e.b != graph_node_of(&g, 22)));
    // a loop: linked, no edge
    check!(node(&g, 50).self_links == 1 && node(&g, 50).linked);
    // links that go nowhere this notebook knows make nothing linked
    check!(!node(&g, 41).linked && !node(&g, 12).linked);
    // the open page
    check!(g.open == graph_node_of(&g, 11) && node(&g, 11).open && !node(&g, 11).linked);
    check!(!graph_persisted(node(&g, 11)) && graph_persisted(node(&g, 10)) && graph_persisted(node(&g, 20)));
    check!(!graph_persisted(node(&g, 24)) && !graph_persisted(node(&g, 13)));
    // edges: one per pair, sorted by page ids, with directions and multiplicity
    check!(g.edges.len() == 3);
    let e = |i: usize| (g.nodes[g.edges[i].a as usize].pid, g.nodes[g.edges[i].b as usize].pid, g.edges[i].dir, g.edges[i].mult);
    check!(e(0) == (10, 23, GRAPH_DIR_BA, 1));
    check!(e(1) == (20, 40, GRAPH_DIR_BA, 1));
    check!(e(2) == (21, 30, GRAPH_DIR_AB | GRAPH_DIR_BA, 3));
    check!(graph_edge_between(&g, graph_node_of(&g, 30), graph_node_of(&g, 21)) == 2);
    check!(graph_edge_between(&g, graph_node_of(&g, 21), graph_node_of(&g, 30)) == 2);
    check!(graph_edge_between(&g, n20, graph_node_of(&g, 21)) == -1);
    check!(graph_neighbours(&g, graph_node_of(&g, 21)) == [n30]);
    check!(graph_neighbours(&g, graph_node_of(&g, 22)).is_empty() && graph_neighbours(&g, -1).is_empty());
    // components: 10-23, 20-40, 21-30, and 11, 22, 50 alone
    check!(g.ncomp == 6);
    check!(node(&g, 10).comp == node(&g, 23).comp && node(&g, 21).comp == node(&g, 30).comp);
    check!(node(&g, 10).comp != node(&g, 21).comp && node(&g, 24).comp == -1);
    // radii: at fit, by degree; unnamed 0.75x; satellites and the tray 6 px
    check_near!(node(&g, 20).r, 30.0, 1e-4);
    check_near!(node(&g, 21).r, 22.5, 1e-4);
    check_near!(node(&g, 22).r, 10.5, 1e-4);
    check!(node(&g, 24).r == GRAPH_R_SAT && node(&g, 13).r == GRAPH_R_SAT);
    check!(g.laid == GraphLaid::Unplaced && g.nodes.iter().all(|n| !n.placed));
    // a notebook past STORE_MAX_PAGES is refused
    let big = book(&(1..=STORE_MAX_PAGES as u32 + 1).collect::<Vec<u32>>());
    let mut gb = GraphModel::default();
    check!(graph_build(&mut gb, &big.nb, &big.nav, 0, false) == -1 && gb.nodes.is_empty());
}

fn radius_formula() {
    check_near!(graph_radius(0, 0, true), 14.0, 1e-5);
    check_near!(graph_radius(0, 9, true), 14.0, 1e-5);
    check_near!(graph_radius(9, 9, true), 30.0, 1e-4);
    check_near!(graph_radius(9, 9, false), 22.5, 1e-4);
    check_near!(graph_radius(2, 7, true), 14.0 + 16.0 * (3.0f32).ln() / (8.0f32).ln(), 1e-4);
    check_near!(graph_radius(0, 7, false), 10.5, 1e-5);
    check!(graph_radius(-3, 7, true) == graph_radius(0, 7, true) && graph_radius(99, 7, true) == graph_radius(7, 7, true));
}

fn pid_table() {
    // a full notebook: every page found, through 4096 slots
    let pids: Vec<u32> = (0..STORE_MAX_PAGES as u32).map(|k| k.wrapping_mul(2654435761) | 1).collect();
    let b = book(&pids);
    let g = built(&b, 0);
    let mut ok = true;
    for (i, &pid) in pids.iter().enumerate() {
        ok &= graph_node_of(&g, pid) == i as i32;
    }
    check!(ok);
    check!(graph_node_of(&g, 2) == -1 && graph_node_of(&g, u32::MAX - 1) == -1);
    // consecutive ids, the usual case
    let seq: Vec<u32> = (1..=STORE_MAX_PAGES as u32).collect();
    let g = built(&book(&seq), 0);
    check!((1..=STORE_MAX_PAGES as u32).all(|p| graph_node_of(&g, p) == p as i32 - 1));
    check!(graph_node_of(&g, STORE_MAX_PAGES as u32 + 1) == -1);
    // a page listed twice answers with its first place
    let mut b = book(&[5, 6, 5]);
    check!(link(&mut b, 6, 1, 5) == 0);
    let g = built(&b, 0);
    check!(graph_node_of(&g, 5) == 0 && node(&g, 5).degree == 1 && g.edges.len() == 1);
}

// A page listed twice (a damaged notebook.ini): its links stay at its first place, but every place
// of a named page heads a section, as nav_section_of has it, and graph.ini keeps one position per
// page: the later place is laid out afresh each visit, the same way each time.
fn duplicate_pages() {
    let mut b = book(&[5, 6, 5, 7, 8]);
    name(&mut b, 5, "Five");
    check!(link(&mut b, 8, 1, 6) == 0);
    let g = built(&b, 0);
    for i in 0..5 {
        check!(g.nodes[i].section == nav_section_of(&b.nav, &b.nb, i as i32));
    }
    check!(g.nodes[2].named && g.nodes[2].dup && !g.nodes[0].dup && g.nodes[2].tier == GraphTier::Force);
    check!(g.nodes[3].section == 2 && g.nodes[3].tier == GraphTier::Satellite && g.nodes[3].sat_rank == 0);
    check!(g.nodes[0].sats == 0 && g.nodes[2].sats == 1);
    check!(graph_persisted(&g.nodes[0]) && !graph_persisted(&g.nodes[2]));
    // the gen holds page 5 once
    let mut bytes: Vec<u8> = Vec::new();
    for pid in [5u32, 6, 8] {
        bytes.push(b'n');
        bytes.extend_from_slice(&pid.to_le_bytes());
    }
    bytes.push(b'e');
    bytes.extend_from_slice(&6u32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    check!(graph_gen(&g) == fnv(&bytes));
    // laid out, kept, and visited twice: the same bits, nothing overlapping
    let mut g = built(&b, 0);
    graph_layout(&mut g, &mut || {});
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &g);
    check!(ini.pos.len() == 3 && overlaps(&g) == 0);
    let mut v1 = built(&b, 0);
    let mut v2 = built(&b, 0);
    check!(graph_apply_ini(&mut v1, &ini) == GraphPlan::Ready && graph_apply_ini(&mut v2, &ini) == GraphPlan::Ready);
    check!(graph_place_new(&mut v1, &mut || {}) == 1 && graph_place_new(&mut v2, &mut || {}) == 1);
    check!(same_places(&v1, &v2));
    check!(force_placed(&v1) && overlaps(&v1) == 0 && !graph_ini_store(&mut ini, &v1));
}

// ---- the layout ----

fn layout_deterministic() {
    // the same notebook twice: the same bits
    let b = random_book(120, 220, 30, 7);
    let g1 = laid_out(&b, 0);
    let g2 = laid_out(&b, 0);
    let same = g1.nodes.iter().zip(&g2.nodes).all(|(a, c)| a.placed == c.placed && a.x.to_bits() == c.x.to_bits() && a.y.to_bits() == c.y.to_bits());
    check!(same);
    check!(g1.laid == GraphLaid::Done && force_placed(&g1));
    // the same pages in other orders: every page lands on the same bits, so neither the jitter nor
    // anything else depends on a page's index (every page is a force node here: the satellites'
    // sections do follow the order)
    let b = all_force_book(90, 140, 11);
    let g1 = laid_out(&b, 0);
    for seed in [1u64, 2, 3] {
        let p = shuffled(&b, seed);
        check!(p.nb.page_ids != b.nb.page_ids);
        let g2 = laid_out(&p, 0);
        let mut same = true;
        for n in &g1.nodes {
            let m = node(&g2, n.pid);
            same &= n.x.to_bits() == m.x.to_bits() && n.y.to_bits() == m.y.to_bits() && n.r.to_bits() == m.r.to_bits();
        }
        check!(same);
        // and the seed alone
        let mut s1 = built(&b, 0);
        let mut s2 = built(&p, 0);
        graph_seed(&mut s1, &mut || {});
        graph_seed(&mut s2, &mut || {});
        check!(s1.nodes.iter().all(|n| node(&s2, n.pid).x.to_bits() == n.x.to_bits() && node(&s2, n.pid).y.to_bits() == n.y.to_bits()));
        check!(graph_gen(&s1) == graph_gen(&s2));
    }
}

fn components_apart() {
    // two stars, a triangle, a pair and three named pages alone
    let mut b = book(&(1..=24).collect::<Vec<u32>>());
    for leaf in 2..=8 {
        check!(link(&mut b, 1, leaf, leaf) == 0);
    }
    for leaf in 10..=15 {
        check!(link(&mut b, 9, leaf, leaf) == 0);
    }
    check!(link(&mut b, 16, 1, 17) == 0 && link(&mut b, 17, 1, 18) == 0 && link(&mut b, 18, 1, 16) == 0);
    check!(link(&mut b, 19, 1, 20) == 0);
    for pid in 21..=23 {
        name(&mut b, pid, "Alone");
    }
    let g = laid_out(&b, 0);
    check!(g.ncomp == 7 && force_placed(&g) && all_finite(&g));
    // each component's disc (centroid, reach) clear of every other's
    let mut discs: Vec<(f32, f32, f32)> = Vec::new();
    for c in 0..g.ncomp {
        let m: Vec<&GraphNode> = g.nodes.iter().filter(|n| n.comp == c).collect();
        let cx = m.iter().map(|n| n.x).sum::<f32>() / m.len() as f32;
        let cy = m.iter().map(|n| n.y).sum::<f32>() / m.len() as f32;
        let r = m.iter().map(|n| ((n.x - cx).powi(2) + (n.y - cy).powi(2)).sqrt() + n.r).fold(0.0f32, f32::max);
        discs.push((cx, cy, r));
    }
    let mut apart = true;
    for i in 0..discs.len() {
        for j in i + 1..discs.len() {
            let d = ((discs[i].0 - discs[j].0).powi(2) + (discs[i].1 - discs[j].1).powi(2)).sqrt();
            apart &= d >= discs[i].2 + discs[j].2;
        }
    }
    check!(apart);
    // and nothing of one component nearer to another than a knockout
    check!(overlaps(&g) == 0);
    // the springs hold: a star's leaves sit near a rest length from their hub
    let hub = node(&g, 1);
    let mut near = true;
    for leaf in 2..=8u32 {
        let d = dist(hub, node(&g, leaf));
        near &= d > 0.5 * GRAPH_SPRING_L && d < 3.0 * GRAPH_SPRING_L;
    }
    check!(near);
}

fn no_overlap_no_nan() {
    for (k, &(n, links, names)) in [(12usize, 10usize, 4usize), (40, 60, 10), (80, 130, 25), (150, 260, 40), (200, 400, 50), (200, 60, 12)].iter().enumerate() {
        let b = random_book(n, links, names, 100 + k as u64);
        let g = laid_out(&b, b.nb.page_ids[n / 2]);
        check!(force_placed(&g) && all_finite(&g));
        check!(overlaps(&g) == 0);
        // the seed alone keeps it too (it is what the view shows while a child lays the map out)
        let mut s = built(&b, 0);
        graph_seed(&mut s, &mut || {});
        check!(s.laid == GraphLaid::Seed && force_placed(&s) && all_finite(&s) && overlaps(&s) == 0);
        // and every satellite is placed beside a placed head; the tray never is
        check!(g.nodes.iter().all(|n| (n.tier == GraphTier::Satellite) == n.placed || n.tier == GraphTier::Force));
    }
    // no edge at all: a field of named orphans
    let mut b = book(&(1..=50).collect::<Vec<u32>>());
    for pid in 1..=50 {
        name(&mut b, pid, "Orphan");
    }
    let g = laid_out(&b, 0);
    check!(g.dmax == 0 && force_placed(&g) && all_finite(&g) && overlaps(&g) == 0);
    // an empty notebook, and one without a force node
    let g = laid_out(&book(&[]), 0);
    check!(g.nodes.is_empty() && g.laid == GraphLaid::Done);
    let g = laid_out(&book(&[1, 2, 3]), 0);
    check!(g.tray.len() == 3 && g.force.is_empty());
}

fn satellites_near_head() {
    // one long section (150 satellites: 60 drawn, 90 more from 2x), two short ones, a linked core
    let mut pids: Vec<u32> = (1..=200).collect();
    pids.rotate_left(3); // (pages 1-3 at the end: the order is not the ids')
    let mut b = book(&pids);
    name(&mut b, pids[0], "Long");
    name(&mut b, pids[151], "Short");
    name(&mut b, pids[160], "Shorter");
    for k in 170..199 {
        check!(link(&mut b, pids[k], 1, pids[k + 1]) == 0);
    }
    check!(link(&mut b, pids[0], 1, pids[170]) == 0);
    check!(link(&mut b, pids[151], 1, pids[171]) == 0);
    let g = laid_out(&b, 0);
    let long = graph_node_of(&g, pids[0]);
    check!(g.nodes[long as usize].sats == 150 && graph_sat_more(&g, long) == 90);
    check!(graph_sat_more(&g, graph_node_of(&g, pids[151])) == 0 && graph_sat_more(&g, -1) == 0);
    let mut within = true;
    let mut count = 0;
    for n in &g.nodes {
        if n.tier != GraphTier::Satellite {
            continue;
        }
        count += 1;
        let h = &g.nodes[n.section as usize];
        within &= n.placed && dist(n, h) <= 3.0 * GRAPH_SPRING_L && dist(n, h) >= h.r + n.r + GRAPH_KNOCKOUT;
    }
    check!(within && count == 150 + 8 + 9);
    // all of them clear of each other, of every other dot, and of every knockout, the 90 drawn only
    // from 2x zoom included (a section of more than about 1000 satellites squeezes its hidden ones
    // closer, to keep them within 3 rest lengths: past that they may touch at 2x)
    check!(overlaps(&g) == 0 && all_finite(&g));
    let hidden = g.nodes.iter().filter(|n| n.tier == GraphTier::Satellite && n.sat_rank >= GRAPH_SAT_DRAWN).count();
    check!(hidden == 90);
}

fn layout_pumps() {
    let b = random_book(60, 90, 12, 5);
    let mut g = built(&b, 0);
    check!(graph_layout_in_process(&g));
    let mut pumps = 0;
    graph_layout(&mut g, &mut || pumps += 1);
    check!(g.relaxed >= 60 && g.relaxed <= GRAPH_STEPS && pumps >= g.relaxed / GRAPH_PUMP_STEPS);
    // the biggest map laid out in the app's process: the longest stretch without a pump (the
    // Kindle is some 20x slower than the host, and its pen must be read every 100 ms)
    let most = all_force_book(GRAPH_INPROC_MAX as usize, 450, 13);
    let mut g = built(&most, 0);
    let start = std::time::Instant::now();
    let mut last = start;
    let mut longest = std::time::Duration::ZERO;
    graph_layout(&mut g, &mut || {
        longest = longest.max(last.elapsed());
        last = std::time::Instant::now();
    });
    longest = longest.max(last.elapsed());
    eprintln!(
        "  {} force nodes in-process: {} ms, at most {} us between pumps",
        GRAPH_INPROC_MAX,
        start.elapsed().as_millis(),
        longest.as_micros()
    );
    check!(longest.as_millis() < 50 && overlaps(&g) == 0);
    // a big map goes to a child
    let big = all_force_book(GRAPH_INPROC_MAX as usize + 1, 400, 3);
    check!(!graph_layout_in_process(&built(&big, 0)));
    check!(graph_layout_in_process(&built(&all_force_book(GRAPH_INPROC_MAX as usize, 400, 3), 0)));
}

// ---- incremental placement ----

// A notebook laid out, its layout kept, then one new page linked to an old one. Applying the kept
// layout leaves only the new page to place, and placing it moves nothing outside its 2-hop
// neighbourhood: those dots keep their bits.
fn incremental_moves_only_the_neighbourhood() {
    let a = random_book(90, 150, 25, 21);
    let ga = laid_out(&a, 0);
    let mut ini = GraphIni::default();
    check!(graph_ini_store(&mut ini, &ga));
    check!(!graph_ini_store(&mut ini, &ga)); // (nothing changed the second time)
    let kept = ga.force.iter().filter(|&&v| graph_persisted(&ga.nodes[v as usize])).count();
    check!(ini.pos.len() == kept);
    // the new page goes at the end of the notebook, linked to a well-linked old page (short of the
    // best linked: a new top degree would shrink every dot a little, and their satellites with them)
    let old = ga
        .force
        .iter()
        .map(|&v| &ga.nodes[v as usize])
        .filter(|n| n.degree < ga.dmax)
        .max_by_key(|n| (n.degree, std::cmp::Reverse(n.pid)))
        .unwrap()
        .pid;
    let new_pid = a.nb.next_page_id;
    let mut b = a.clone();
    b.nb.page_ids.push(new_pid);
    check!(link(&mut b, new_pid, 1, old) == 0);
    let mut g = built(&b, 0);
    check!(graph_gen(&g) != ini.gen);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Incremental);
    let fresh: Vec<u32> = g.force.iter().map(|&v| &g.nodes[v as usize]).filter(|n| !n.placed).map(|n| n.pid).collect();
    check!(fresh == vec![new_pid]);
    // the 2-hop set of the new page
    let nn = graph_node_of(&g, new_pid);
    let mut hop = vec![false; g.nodes.len()];
    hop[nn as usize] = true;
    for &u in graph_neighbours(&g, nn) {
        hop[u as usize] = true;
        for &w in graph_neighbours(&g, u) {
            hop[w as usize] = true;
        }
    }
    let mut pumps = 0;
    check!(graph_place_new(&mut g, &mut || pumps += 1) == 1);
    check!(pumps >= GRAPH_INC_STEPS / GRAPH_PUMP_STEPS);
    let mut still = true;
    let mut outside = 0;
    for &v in &ga.force {
        let was = &ga.nodes[v as usize];
        let i = graph_node_of(&g, was.pid) as usize;
        if hop[i] {
            continue;
        }
        outside += 1;
        still &= g.nodes[i].x.to_bits() == was.x.to_bits() && g.nodes[i].y.to_bits() == was.y.to_bits();
    }
    check!(still && outside > 40);
    // the satellites of heads outside it did not move either
    let mut sat_still = true;
    for n in &ga.nodes {
        if n.tier == GraphTier::Satellite && !hop[n.section as usize] {
            let m = node(&g, n.pid);
            sat_still &= m.x.to_bits() == n.x.to_bits() && m.y.to_bits() == n.y.to_bits();
        }
    }
    check!(sat_still);
    // and the new page sits sensibly: near its neighbour, clear of everything, in whole units
    let p = node(&g, new_pid);
    check!(p.placed && p.x.is_finite() && p.y.is_finite() && p.x == p.x.round() && p.y == p.y.round());
    let d = dist(p, node(&g, old));
    check!(d > p.r + node(&g, old).r && d < 2.5 * GRAPH_SPRING_L);
    check!(overlaps(&g) == 0 && force_placed(&g) && all_finite(&g));
    // what it stores now is the new graph, and a second visit changes nothing
    check!(graph_ini_store(&mut ini, &g));
    let mut g2 = built(&b, 0);
    check!(graph_apply_ini(&mut g2, &ini) == GraphPlan::Ready);
    check!(graph_place_new(&mut g2, &mut || {}) == 0);
    check!(g2.nodes.iter().zip(&g.nodes).all(|(x, y)| x.x.to_bits() == y.x.to_bits() && x.y.to_bits() == y.y.to_bits()));
    check!(!graph_ini_store(&mut ini, &g2));
}

fn large_incremental_moves_only_the_neighbourhood() {
    let a = random_book(700, 1100, 180, 21);
    let ga = laid_out(&a, 0);
    let mut ini = GraphIni::default();
    check!(graph_ini_store(&mut ini, &ga));
    check!(!graph_ini_store(&mut ini, &ga)); // (nothing changed the second time)
    let kept = ga.force.iter().filter(|&&v| graph_persisted(&ga.nodes[v as usize])).count();
    check!(ini.pos.len() == kept);
    // the new page goes at the end of the notebook, linked to a well-linked old page (short of the
    // best linked: a new top degree would shrink every dot a little, and their satellites with them)
    let old = ga
        .force
        .iter()
        .map(|&v| &ga.nodes[v as usize])
        .filter(|n| n.degree < ga.dmax)
        .max_by_key(|n| (n.degree, std::cmp::Reverse(n.pid)))
        .unwrap()
        .pid;
    let new_pid = a.nb.next_page_id;
    let mut b = a.clone();
    for extra in 0..3 {
        b.nb.page_ids.push(new_pid + extra);
        check!(link(&mut b, new_pid + extra, 1, old) == 0);
    }
    let mut g = built(&b, 0);
    check!(graph_gen(&g) != ini.gen);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Incremental);
    let fresh: Vec<u32> = g.force.iter().map(|&v| &g.nodes[v as usize]).filter(|n| !n.placed).map(|n| n.pid).collect();
    check!(fresh == vec![new_pid, new_pid + 1, new_pid + 2]);
    // the 2-hop set of the new page
    let nn = graph_node_of(&g, new_pid);
    let mut hop = vec![false; g.nodes.len()];
    for extra in 0..3 { hop[graph_node_of(&g, new_pid + extra) as usize] = true; }
    for &u in graph_neighbours(&g, nn) {
        hop[u as usize] = true;
        for &w in graph_neighbours(&g, u) {
            hop[w as usize] = true;
        }
    }
    let mut pumps = 0;
    check!(graph_place_new(&mut g, &mut || pumps += 1) == 3);
    check!(pumps >= GRAPH_INC_STEPS / GRAPH_PUMP_STEPS);
    let mut still = true;
    let mut outside = 0;
    for &v in &ga.force {
        let was = &ga.nodes[v as usize];
        let i = graph_node_of(&g, was.pid) as usize;
        if hop[i] {
            continue;
        }
        outside += 1;
        still &= g.nodes[i].x.to_bits() == was.x.to_bits() && g.nodes[i].y.to_bits() == was.y.to_bits();
    }
    check!(still && outside > 40);
    // the satellites of heads outside it did not move either
    let mut sat_still = true;
    for n in &ga.nodes {
        if n.tier == GraphTier::Satellite && !hop[n.section as usize] {
            let m = node(&g, n.pid);
            sat_still &= m.x.to_bits() == n.x.to_bits() && m.y.to_bits() == n.y.to_bits();
        }
    }
    check!(sat_still);
    // and the new page sits sensibly: near its neighbour, clear of everything, in whole units
    let p = node(&g, new_pid);
    check!(p.placed && p.x.is_finite() && p.y.is_finite() && p.x == p.x.round() && p.y == p.y.round());
    let d = dist(p, node(&g, old));
    check!(d > p.r + node(&g, old).r && d < 2.5 * GRAPH_SPRING_L);
    check!(overlaps(&g) == 0 && force_placed(&g) && all_finite(&g));
    // what it stores now is the new graph, and a second visit changes nothing
    check!(graph_ini_store(&mut ini, &g));
    let mut g2 = built(&b, 0);
    check!(graph_apply_ini(&mut g2, &ini) == GraphPlan::Ready);
    check!(graph_place_new(&mut g2, &mut || {}) == 0);
    check!(g2.nodes.iter().zip(&g.nodes).all(|(x, y)| x.x.to_bits() == y.x.to_bits() && x.y.to_bits() == y.y.to_bits()));
    check!(!graph_ini_store(&mut ini, &g2));
}

// Opening the map from a plain page makes it a force node for the visit only: it is placed beside
// its section's named page, every kept dot stays, and graph.ini does not change for it.
fn open_page_placed_beside_its_section() {
    let a = random_book(70, 90, 20, 31);
    let ga = laid_out(&a, 0);
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &ga);
    let plain = ga.nodes.iter().find(|n| n.tier == GraphTier::Satellite).unwrap();
    let mut g = built(&a, plain.pid);
    // A seed from a pending layout must not masquerade as a cached position for the open page.
    assert!(graph_seed(&mut g, &mut || {}));
    let o = node(&g, plain.pid);
    check!(o.tier == GraphTier::Force && o.open && !graph_persisted(o));
    check!(graph_gen(&g) == ini.gen);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Ready);
    check!(graph_place_new(&mut g, &mut || {}) == 1);
    let mut still = true;
    for &v in &ga.force {
        let was = &ga.nodes[v as usize];
        let m = node(&g, was.pid);
        still &= m.x.to_bits() == was.x.to_bits() && m.y.to_bits() == was.y.to_bits();
    }
    check!(still);
    let o = node(&g, plain.pid);
    let head = &g.nodes[o.section as usize];
    check!(o.placed && dist(o, head) < 3.0 * GRAPH_SPRING_L && overlaps(&g) == 0);
    check!(!graph_ini_store(&mut ini, &g));
}

// The satellites keep their places whichever page the map is opened from: an open plain page
// keeps its slot on its section's spiral, empty while it is open.
fn satellites_ignore_the_open_page() {
    let mut b = book(&(1..=40).collect::<Vec<u32>>());
    name(&mut b, 1, "Head");
    name(&mut b, 20, "Other");
    check!(link(&mut b, 1, 1, 20) == 0);
    let ga = laid_out(&b, 0);
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &ga);
    let mut views: Vec<GraphModel> = Vec::new();
    for open in [0u32, 3, 12, 25] {
        let mut g = built(&b, open);
        check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Ready);
        check!(graph_place_new(&mut g, &mut || {}) == (open != 0) as i32);
        views.push(g);
    }
    for v in &views[1..] {
        let mut same = true;
        for (k, n) in views[0].nodes.iter().enumerate() {
            let m = &v.nodes[k];
            if !m.open {
                same &= n.placed == m.placed && n.x.to_bits() == m.x.to_bits() && n.y.to_bits() == m.y.to_bits();
            }
        }
        check!(same);
        let o = &v.nodes[v.open as usize];
        check!(o.placed && o.sat_rank >= 0 && dist(o, &v.nodes[o.section as usize]) < 3.0 * GRAPH_SPRING_L && overlaps(v) == 0);
    }
}

fn relayout_rule() {
    check!(!graph_relayout_needed(0, 0) && !graph_relayout_needed(0, 50));
    check!(!graph_relayout_needed(1, 50) && !graph_relayout_needed(5, 50));
    check!(graph_relayout_needed(6, 50) && graph_relayout_needed(1, 1) && graph_relayout_needed(3, 3) && graph_relayout_needed(2, 0));
    check!(!graph_relayout_needed(10, 100) && graph_relayout_needed(11, 100));
    // nothing kept: a full layout
    let b = random_book(40, 60, 10, 3);
    let mut g = built(&b, 0);
    check!(graph_apply_ini(&mut g, &GraphIni::default()) == GraphPlan::Full);
    // more than a tenth new: a full layout
    let a = all_force_book(40, 60, 9);
    let ga = laid_out(&a, 0);
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &ga);
    let mut b = a.clone();
    let base = a.nb.next_page_id;
    for k in 0..5 {
        b.nb.page_ids.push(base + k);
        check!(link(&mut b, base + k, 1, a.nb.page_ids[k as usize]) == 0);
    }
    let mut g = built(&b, 0);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Full); // 5 of 45
    let mut b = a.clone();
    b.nb.page_ids.push(base);
    check!(link(&mut b, base, 1, a.nb.page_ids[0]) == 0);
    let mut g = built(&b, 0);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Incremental); // 1 of 41
    // removed pages count for nothing: their dots just go
    let mut b = a.clone();
    b.nb.page_ids.truncate(20);
    let mut g = built(&b, 0);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Ready);
}

// ---- Around ----

fn around_rings() {
    // centre 1; ring one: 2-6; ring two: 7-16 (two per ring-one page); 17 three links away
    let mut b = book(&(1..=18).collect::<Vec<u32>>());
    for k in 2..=6u32 {
        check!(link(&mut b, 1, k, k) == 0);
        check!(link(&mut b, k, 1, 2 * k + 3) == 0);
        check!(link(&mut b, 2 * k + 4, 1, k) == 0);
    }
    check!(link(&mut b, 7, 2, 17) == 0);
    name(&mut b, 18, "Alone");
    let g = laid_out(&b, 1);
    let c = graph_node_of(&g, 1);
    let mut out = Vec::new();
    check!(graph_around(&g, c, 2, &mut out) == 16 && out.len() == 16);
    check!(out[0].node == c && out[0].depth == 0 && out[0].x == 0.0 && out[0].y == 0.0 && out[0].parent == -1);
    let r = |m: &GraphRing| (m.x * m.x + m.y * m.y).sqrt();
    let ring1: Vec<&GraphRing> = out.iter().filter(|m| m.depth == 1).collect();
    let ring2: Vec<&GraphRing> = out.iter().filter(|m| m.depth == 2).collect();
    check!(ring1.len() == 5 && ring2.len() == 10);
    let r1 = r(ring1[0]);
    let r2 = r(ring2[0]);
    check!(ring1.iter().all(|m| (r(m) - r1).abs() < 0.01) && ring2.iter().all(|m| (r(m) - r2).abs() < 0.01));
    check!(r1 >= GRAPH_SPRING_L && r2 >= r1 + GRAPH_SPRING_L);
    // members only: 17 (three links away) and 18 (no link) are not in it
    check!(out.iter().all(|m| g.nodes[m.node as usize].pid != 17 && g.nodes[m.node as usize].pid != 18));
    // every outer page hangs on a ring-one page it is linked with, and its parents go round in order
    let k1 = |node: i32| ring1.iter().position(|m| m.node == node).unwrap_or(99);
    let mut linked = true;
    for m in &ring2 {
        linked &= k1(m.parent) < 5 && graph_edge_between(&g, m.node, m.parent) >= 0;
    }
    check!(linked);
    let mut order: Vec<(f32, usize)> = ring2.iter().map(|m| (m.y.atan2(m.x), k1(m.parent))).collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let drops = (0..order.len()).filter(|&i| order[(i + 1) % order.len()].1 < order[i].1).count();
    check!(drops <= 1); // one way round the circle, once
    // no two members overlap
    let mut clear = true;
    for i in 0..out.len() {
        for j in i + 1..out.len() {
            let (a, b) = (&out[i], &out[j]);
            let d = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
            clear &= d >= g.nodes[a.node as usize].r + g.nodes[b.node as usize].r + GRAPH_KNOCKOUT;
        }
    }
    check!(clear && out.iter().all(|m| m.x.is_finite() && m.y.is_finite()));
    // depth 1 only; a page with no link; a bad node
    check!(graph_around(&g, c, 1, &mut out) == 6 && out.iter().all(|m| m.depth <= 1));
    check!(graph_around(&g, graph_node_of(&g, 18), 2, &mut out) == 1 && out[0].node == graph_node_of(&g, 18));
    check!(graph_around(&g, -1, 2, &mut out) == 0 && out.is_empty());
    // the same twice
    let mut again = Vec::new();
    graph_around(&g, c, 2, &mut out);
    graph_around(&g, c, 2, &mut again);
    check!(out == again);
    // without a layout, it still answers (page-id angles)
    let raw = built(&b, 1);
    check!(graph_around(&raw, c, 2, &mut out) == 16);
}

// ---- graph.ini ----

fn read_bytes(p: &str) -> Vec<u8> {
    std::fs::read(p).unwrap_or_default()
}

fn ini_round_trip() {
    let path = tmp_path("round.ini");
    let mut ini = GraphIni::default();
    ini.gen = 0x5f3a9c21;
    ini.has_gen = true;
    ini.mode = GraphMode::Around;
    ini.show_unlinked = false;
    ini.pos = vec![GraphPos { pid: 7, x: -310, y: 442 }, GraphPos { pid: 12, x: 0, y: -1 }, GraphPos { pid: 4000000000, x: GRAPH_POS_MAX, y: -GRAPH_POS_MAX }];
    ini.pins = vec![7, 9];
    // carried lines: kept byte for byte, but those whose key this version writes (mode, gen,
    // unlinked, pos and pin of the pages it writes): its own line holds, and a reader taking the
    // last line must not find the stale one after it
    ini.unknown = b"future.key=1\npos.12=1,2,3\npos.99=1,2,3\nmode=local\ngen=xyz\nunlinked\npin.7=yes\npin.8=2\nk=a=b\n".to_vec();
    check!(graph_ini_write_path(&path, &mut ini) == 0 && ini.file);
    let text = read_bytes(&path);
    let want = b"# Grimoire: the Index map's layout (a cache: deleting it loses nothing)\n\
gen=5f3a9c21\nmode=around\nunlinked=hide\npos.7=-310,442\npos.12=0,-1\npos.4000000000=1000000,-1000000\n\
pin.7=1\npin.9=1\nfuture.key=1\npos.99=1,2,3\npin.8=2\nk=a=b\n";
    check!(text == want.to_vec());
    check!(ini.unknown == b"future.key=1\npos.99=1,2,3\npin.8=2\nk=a=b\n".to_vec());
    check!(!std::path::Path::new(&format!("{path}.tmp")).exists());
    let mut back = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut back) == 0);
    check!(back == ini);
    // written again: the same bytes (the carried lines byte for byte)
    check!(graph_ini_write_path(&path, &mut back) == 0 && read_bytes(&path) == want.to_vec());
    // a hand-edited file: CRLF, duplicates (the last wins), comments dropped, odd lines carried
    let hand = b"# a comment\r\ngen=0000ABCD\r\nmode=around\r\nmode=all\r\nunlinked=hide\r\npos.5=1,2\r\npos.3=-4,5\r\npos.5=7,8\r\n\
pin.3=1\r\npin.3=1\r\npin.4=2\r\npos.0=1,1\r\npos.6=1\r\npos.7=01,-0\r\npos.8=1000001,0\r\ngen=xyz\r\n\r\nlast";
    std::fs::write(&path, hand).unwrap();
    check!(graph_ini_read_path(&path, &mut back) == 0);
    check!(back.has_gen && back.gen == 0xabcd && back.mode == GraphMode::All && !back.show_unlinked);
    check!(back.pos == vec![GraphPos { pid: 3, x: -4, y: 5 }, GraphPos { pid: 5, x: 7, y: 8 }, GraphPos { pid: 7, x: 1, y: 0 }]);
    check!(back.pins == vec![3]);
    check!(back.unknown == b"pin.4=2\npos.0=1,1\npos.6=1\npos.8=1000001,0\ngen=xyz\nlast\n".to_vec());
    check!(back.writable && back.file);
    // written back: LF line ends (as nav.ini is), no comment, the stale gen=xyz left out
    check!(graph_ini_write_path(&path, &mut back) == 0);
    let want2 = b"# Grimoire: the Index map's layout (a cache: deleting it loses nothing)\n\
gen=0000abcd\nmode=all\nunlinked=hide\npos.3=-4,5\npos.5=7,8\npos.7=1,0\npin.3=1\n\
pin.4=2\npos.0=1,1\npos.6=1\npos.8=1000001,0\nlast\n";
    check!(read_bytes(&path) == want2.to_vec());
    let mut again = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut again) == 0 && again == back);
    // carried lines a read would refuse are never written: the file keeps its bytes
    let long = [vec![b'x'; GRAPH_LINE_MAX + 1], b"\n".to_vec()].concat();
    for bad in [long, b"no.newline=1".to_vec(), b"nul=\0\n".to_vec(), vec![b'y'; GRAPH_UNKNOWN_MAX + 1]] {
        let mut w = back.clone();
        w.unknown = bad;
        check!(graph_ini_write_path(&path, &mut w) == -1 && read_bytes(&path) == want2.to_vec());
    }
    let mut w = back.clone();
    w.unknown = [vec![b'x'; GRAPH_LINE_MAX], b"\r\n".to_vec()].concat();
    check!(graph_ini_write_path(&path, &mut w) == 0);
    check!(graph_ini_read_path(&path, &mut again) == 0 && again.writable && again.unknown.len() == GRAPH_LINE_MAX + 1);
    let mut many: Vec<u8> = Vec::new();
    while many.len() + 16 <= GRAPH_UNKNOWN_MAX {
        many.extend_from_slice(b"carried=0123456\n");
    }
    w.unknown = many.clone();
    check!(graph_ini_write_path(&path, &mut w) == 0 && graph_ini_read_path(&path, &mut again) == 0 && again.unknown == many);
    w.unknown.extend_from_slice(b"carried=0123456\n");
    check!(graph_ini_write_path(&path, &mut w) == -1);
    // a missing file: a fresh cache, writable
    let mut fresh = GraphIni::default();
    fresh.mode = GraphMode::Around;
    check!(graph_ini_read_path(&tmp_path("none.ini"), &mut fresh) == 0);
    check!(fresh == GraphIni::default() && fresh.writable && !fresh.file);
    // the id-based pair: root/notebooks/<id>/graph.ini
    let root = tmp_path("root");
    check!(store_init(&root) == 0);
    std::fs::create_dir_all(format!("{root}/notebooks/nb-graph")).unwrap();
    let mut p = String::new();
    check!(graph_ini_path("nb-graph", &mut p) == 0 && p == format!("{root}/notebooks/nb-graph/graph.ini"));
    check!(graph_ini_write("nb-graph", &mut ini) == 0 && read_bytes(&p) == want.to_vec());
    let mut by_id = GraphIni::default();
    check!(graph_ini_read("nb-graph", &mut by_id) == 0 && by_id == ini);
    check!(graph_ini_path("../escape", &mut p) == -1);
    check!(graph_ini_read("../escape", &mut by_id) == -1 && !by_id.writable);
    let mut w = GraphIni::default();
    check!(graph_ini_write("../escape", &mut w) == -1);
}

fn ini_limits() {
    let path = tmp_path("limits.ini");
    let mut ini = GraphIni::default();
    // a carried line of 255 bytes is kept; one of 256 refuses the file
    let ok_line = vec![b'x'; GRAPH_LINE_MAX];
    std::fs::write(&path, [b"gen=00000001\n".as_slice(), &ok_line, b"\n"].concat()).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == 0 && ini.unknown.len() == GRAPH_LINE_MAX + 1 && ini.writable);
    std::fs::write(&path, [b"gen=00000001\n".as_slice(), &vec![b'x'; GRAPH_LINE_MAX + 1], b"\n"].concat()).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == -1 && !ini.writable && ini.file && ini.unknown.is_empty());
    // carried lines up to 16 KiB in all (64 lines of 255 + newline), not one more
    let mut text: Vec<u8> = Vec::new();
    for k in 0..64 {
        let mut l = format!("k{k:03}=").into_bytes();
        l.resize(GRAPH_LINE_MAX, b'v');
        text.extend_from_slice(&l);
        text.push(b'\n');
    }
    std::fs::write(&path, &text).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == 0 && ini.unknown.len() == GRAPH_UNKNOWN_MAX && ini.unknown == text);
    text.extend_from_slice(b"one=more\n");
    std::fs::write(&path, &text).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == -1 && !ini.writable);
    // 256 KiB at most (comments fill it)
    let mut text = b"gen=00000002\n".to_vec();
    while text.len() < GRAPH_FILE_MAX - 100 {
        text.extend_from_slice(b"# padding padding padding padding padding padding padding padding padding\n");
    }
    text.resize(GRAPH_FILE_MAX - 1, b'#');
    text.push(b'\n');
    std::fs::write(&path, &text).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == 0 && ini.gen == 2 && ini.writable);
    text.insert(0, b'\n');
    std::fs::write(&path, &text).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == -1 && !ini.writable);
    // more positions than a notebook has pages
    let mut text = Vec::new();
    for pid in 1..=STORE_MAX_PAGES as u32 + 1 {
        text.extend_from_slice(format!("pos.{pid}=1,2\n").as_bytes());
    }
    std::fs::write(&path, &text).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == -1);
    // STORE_MAX_PAGES of them read, and a whole map's file stays far under the cap
    text.truncate(text.len() - format!("pos.{}=1,2\n", STORE_MAX_PAGES + 1).len());
    std::fs::write(&path, &text).unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == 0 && ini.pos.len() == STORE_MAX_PAGES);
    // a NUL anywhere, a folder, a symlink: refused
    std::fs::write(&path, b"gen=00000003\nmode=all\0\n").unwrap();
    check!(graph_ini_read_path(&path, &mut ini) == -1);
    let dir = tmp_path("adir.ini");
    std::fs::create_dir_all(&dir).unwrap();
    check!(graph_ini_read_path(&dir, &mut ini) == -1 && !ini.writable);
    let good = tmp_path("good.ini");
    std::fs::write(&good, b"gen=00000004\n").unwrap();
    let link = tmp_path("link.ini");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&good, &link).unwrap();
    check!(graph_ini_read_path(&link, &mut ini) == -1 && !ini.writable);
    let dangling = tmp_path("dangling.ini");
    let _ = std::fs::remove_file(&dangling);
    std::os::unix::fs::symlink(tmp_path("nowhere"), &dangling).unwrap();
    check!(graph_ini_read_path(&dangling, &mut ini) == -1);
    // a write that would pass the cap writes nothing
    let mut huge = GraphIni::default();
    huge.pos = (1..=10_000u32).map(|pid| GraphPos { pid, x: -GRAPH_POS_MAX, y: -GRAPH_POS_MAX }).collect();
    let before = read_bytes(&good);
    check!(graph_ini_write_path(&good, &mut huge) == -1 && read_bytes(&good) == before);
}

// A file that does not read is never overwritten: the write that follows the refusal fails, and
// the file keeps its bytes.
fn corrupt_file_refused() {
    let path = tmp_path("corrupt.ini");
    let bytes = b"gen=12345678\npos.5=1,2\n\0\x01binary tail".to_vec();
    std::fs::write(&path, &bytes).unwrap();
    let mut ini = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut ini) == -1);
    check!(!ini.writable && ini.file && ini.pos.is_empty() && !ini.has_gen);
    // the map lays itself out afresh, and the write that would save it is refused
    let b = random_book(30, 40, 8, 4);
    let mut g = built(&b, 0);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Full);
    check!(graph_layout_job(&mut g, &mut ini, &path) == -1);
    check!(ini.has_gen && !ini.pos.is_empty() && !ini.writable);
    check!(graph_ini_write_path(&path, &mut ini) == -1);
    check!(read_bytes(&path) == bytes && !std::path::Path::new(&format!("{path}.tmp")).exists());
    // an oversized one too
    let big = vec![b'#'; GRAPH_FILE_MAX + 1];
    std::fs::write(&path, &big).unwrap();
    let mut ini = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut ini) == -1 && graph_ini_write_path(&path, &mut ini) == -1 && read_bytes(&path) == big);
}

fn gen_encoding_and_mismatch() {
    // the encoding, spelled out
    check!(graph_gen(&built(&book(&[]), 0)) == 0x811c9dc5);
    let mut b = book(&[7, 5, 9]);
    check!(link(&mut b, 7, 1, 5) == 0);
    name(&mut b, 9, "Nine");
    let g = built(&b, 0);
    let mut bytes: Vec<u8> = Vec::new();
    for pid in [5u32, 7, 9] {
        bytes.push(b'n');
        bytes.extend_from_slice(&pid.to_le_bytes());
    }
    bytes.push(b'e');
    bytes.extend_from_slice(&5u32.to_le_bytes());
    bytes.extend_from_slice(&7u32.to_le_bytes());
    check!(graph_gen(&g) == fnv(&bytes));
    // what does not change the gen: the page order, another link on the same pair, an Index link
    // on a linked page, a name on a linked page, the open page
    let base = graph_gen(&g);
    let mut c = b.clone();
    c.nb.page_ids = vec![9, 7, 5];
    check!(link(&mut c, 5, 2, 7) == 0 && link(&mut c, 7, 3, 5) == 0 && link_index(&mut c, 5, 4) == 0);
    name(&mut c, 5, "Five");
    c.nb.page_ids.push(11);
    check!(graph_gen(&built(&c, 11)) == base);
    // what does: a new pair, a new linked or named page
    let mut d = b.clone();
    check!(link(&mut d, 9, 1, 5) == 0);
    check!(graph_gen(&built(&d, 0)) != base);
    let mut d = b.clone();
    d.nb.page_ids.push(11);
    name(&mut d, 11, "Eleven");
    check!(graph_gen(&built(&d, 0)) != base);
    // a gen mismatch with no new page: a pair of kept pages unlinked (both keep other links, and
    // neither is the best linked, so no dot grows): nothing moves
    let a = random_book(80, 120, 20, 41);
    let ga = laid_out(&a, 0);
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &ga);
    let path = tmp_path("gen.ini");
    check!(graph_ini_write_path(&path, &mut ini) == 0);
    let mut read = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut read) == 0 && read == ini);
    let quiet = |n: &GraphNode| n.degree >= 2 && n.degree < ga.dmax;
    let e = *ga.edges.iter().find(|e| quiet(&ga.nodes[e.a as usize]) && quiet(&ga.nodes[e.b as usize])).unwrap();
    let (p, q) = (ga.nodes[e.a as usize].pid, ga.nodes[e.b as usize].pid);
    let mut b = a.clone();
    let gone: Vec<(u32, u32)> = b.nav.links.iter().filter(|l| (l.pid == p && l.target == format!("p{q}").as_str()) || (l.pid == q && l.target == format!("p{p}").as_str())).map(|l| (l.pid, l.seed)).collect();
    check!(gone.len() as i32 == e.mult);
    for (pid, seed) in gone {
        nav_link_remove(&mut b.nav, pid, seed);
    }
    let mut g = built(&b, 0);
    check!(graph_gen(&g) != read.gen && g.edges.len() + 1 == ga.edges.len() && g.dmax == ga.dmax);
    check!(graph_apply_ini(&mut g, &read) == GraphPlan::Ready);
    check!(graph_place_new(&mut g, &mut || {}) == 0);
    check!(ga.nodes.iter().zip(&g.nodes).all(|(x, y)| x.x.to_bits() == y.x.to_bits() && x.y.to_bits() == y.y.to_bits()));
    check!(graph_ini_store(&mut read, &g) && read.gen == graph_gen(&g));
    // a pair linked: the two dots may grow, and then only what came to overlap moves
    let f = &ga.force;
    let (p, q) = (0..f.len())
        .flat_map(|i| (i + 1..f.len()).map(move |j| (f[i], f[j])))
        .find(|&(u, v)| graph_edge_between(&ga, u, v) < 0)
        .map(|(u, v)| (ga.nodes[u as usize].pid, ga.nodes[v as usize].pid))
        .unwrap();
    let mut b = a.clone();
    check!(link(&mut b, p, 77, q) == 0);
    let mut g = built(&b, 0);
    check!(graph_apply_ini(&mut g, &ini) == GraphPlan::Ready);
    check!(graph_place_new(&mut g, &mut || {}) == 0);
    check!(overlaps(&g) == 0 && force_placed(&g) && all_finite(&g));
}

// The bgjob child's work: the layout, then graph.ini written whole and atomically.
fn layout_job_writes_atomically() {
    let path = tmp_path("job.ini");
    let _ = std::fs::remove_file(&path);
    let b = random_book(60, 100, 15, 51);
    let mut g = built(&b, 0);
    let mut ini = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut ini) == 0 && !ini.file);
    ini.mode = GraphMode::Around;
    ini.unknown = b"carried=yes\n".to_vec();
    check!(graph_layout_job(&mut g, &mut ini, &path) == 0 && ini.file);
    check!(!std::path::Path::new(&format!("{path}.tmp")).exists());
    let mut back = GraphIni::default();
    check!(graph_ini_read_path(&path, &mut back) == 0);
    check!(back.gen == graph_gen(&g) && back.mode == GraphMode::Around && back.unknown == b"carried=yes\n".to_vec());
    let kept = g.force.iter().filter(|&&v| graph_persisted(&g.nodes[v as usize])).count();
    check!(back.pos.len() == kept && back == ini);
    // The view changes its choices after this job loaded its metadata: positions may change,
    // but finishing the job must keep those newer choices and carried lines.
    back.mode = GraphMode::All; back.show_unlinked = false; back.unknown = b"newer=kept\n".to_vec();
    check!(graph_ini_write_path(&path,&mut back)==0);
    check!(graph_layout_job(&mut g,&mut ini,&path)==0);
    check!(ini.mode==GraphMode::All && !ini.show_unlinked && ini.unknown==b"newer=kept\n");
    // (that second job started from the first one's positions: graph_layout_warm)
    check!(graph_ini_read_path(&path, &mut back) == 0 && back == ini);

    // the parent, reading it, finds the layout ready, and lays nothing out
    let mut view = built(&b, 0);
    check!(graph_apply_ini(&mut view, &back) == GraphPlan::Ready);
    graph_place_new(&mut view, &mut || {});
    check!(view.nodes.iter().zip(&g.nodes).all(|(x, y)| x.x.to_bits() == y.x.to_bits() && x.y.to_bits() == y.y.to_bits()));
}

// 2000 force nodes and 4000 edges: what a bgjob child lays out. The plan's figure is about 3.4 s on
// the Kindle (some 20x the host); the bound here is loose, for a debug build on a busy machine.
fn big_layout_within_bound() {
    let b = all_force_book(STORE_MAX_PAGES, NAV_LINKS_MAX as usize, 61);
    let mut g = built(&b, 0);
    check!(g.force.len() == STORE_MAX_PAGES && g.edges.len() == NAV_LINKS_MAX as usize);
    check!(!graph_layout_in_process(&g));
    let t = std::time::Instant::now();
    let mut pumps = 0;
    graph_layout(&mut g, &mut || pumps += 1);
    let ms = t.elapsed().as_millis();
    eprintln!("  2000 force nodes, 4000 edges: laid out in {ms} ms ({pumps} pumps)");
    check!(ms < 60_000);
    check!(pumps >= g.relaxed / GRAPH_PUMP_STEPS && force_placed(&g) && all_finite(&g));
    check!(overlaps(&g) == 0);
    let t = std::time::Instant::now();
    let mut s = built(&b, 0);
    graph_seed(&mut s, &mut || {});
    eprintln!("  the seed alone: {} ms", t.elapsed().as_millis());
    check!(force_placed(&s) && all_finite(&s) && overlaps(&s) == 0);
    // one new page on the big map: placed beside its neighbour, its 2-hop neighbourhood relaxed
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &g);
    let mut more = b.clone();
    let new_pid = more.nb.next_page_id;
    more.nb.page_ids.truncate(STORE_MAX_PAGES - 1);
    more.nb.page_ids.push(new_pid);
    let l = more.nav.links[more.nav.links.len() - 1]; // (room under NAV_LINKS_MAX)
    nav_link_remove(&mut more.nav, l.pid, l.seed);
    check!(link(&mut more, new_pid, 1, b.nb.page_ids[0]) == 0);
    let mut inc = built(&more, 0);
    check!(graph_apply_ini(&mut inc, &ini) == GraphPlan::Incremental);
    let t = std::time::Instant::now();
    check!(graph_place_new(&mut inc, &mut || {}) == 1);
    eprintln!("  one new page on it: placed in {} us", t.elapsed().as_micros());
    check!(force_placed(&inc) && overlaps(&inc) == 0);
    let t = std::time::Instant::now();
    let mut again = GraphModel::default();
    for _ in 0..10 {
        graph_build(&mut again, &b.nb, &b.nav, 0, false);
    }
    eprintln!("  the build: {} us each", t.elapsed().as_micros() / 10);
    check!(again.edges == g.edges);
}

// ---- the linked-only model (the view's) ----

// Only linked pages are force nodes; every other page, named or open, is hidden and has no
// satellites, so it takes no room. Adding plain pages and naming unlinked ones changes neither the
// gen nor any position.
fn linked_only_model() {
    let b = fixture();
    let g = linked_built(&b, 11);
    for pid in [21u32, 22, 23, 30, 40, 20, 50, 10] {
        check!(node(&g, pid).tier == GraphTier::Force);
    }
    // 31 32 24 25 41 13 12 are plain, 11 is open and plain
    for pid in [31u32, 32, 24, 25, 41, 13, 12, 11] {
        check!(node(&g, pid).tier == GraphTier::Hidden && node(&g, pid).sat_rank == -1);
    }
    check!(g.tray.is_empty() && g.nodes.iter().all(|n| n.sats == 0));
    check!(g.force.len() == 8);
    // 60 chapters, linked in a chain, each with 29 plain pages after it: the same layout as
    // without the plain pages
    let mut pids = Vec::new();
    let mut lean = Vec::new();
    for c in 0..60u32 {
        lean.push(c * 100 + 1);
        for k in 0..30 {
            pids.push(c * 100 + 1 + k);
        }
    }
    let mut full = book(&pids);
    let mut bare = book(&lean);
    for c in 0..60u32 {
        name(&mut full, c * 100 + 1, &format!("Chapter {c}"));
        name(&mut bare, c * 100 + 1, &format!("Chapter {c}"));
        if c > 0 {
            check!(link(&mut full, c * 100 + 1, 1, (c - 1) * 100 + 1) == 0);
            check!(link(&mut bare, c * 100 + 1, 1, (c - 1) * 100 + 1) == 0);
        }
    }
    let mut gf = linked_built(&full, 5);
    let mut gb = linked_built(&bare, 0);
    check!(graph_gen(&gf) == graph_gen(&gb) && gf.force.len() == 60 && !gf.nodes[4].placed);
    graph_layout(&mut gf, &mut || {});
    graph_layout(&mut gb, &mut || {});
    check!(lean.iter().all(|&pid| node(&gf, pid).x.to_bits() == node(&gb, pid).x.to_bits() && node(&gf, pid).y.to_bits() == node(&gb, pid).y.to_bits()));
    // the old model gave each chapter an invisible disc for its 29 satellites
    let (span, old_span) = {
        let width = |g: &GraphModel| {
            let xs = g.force.iter().filter(|&&v| g.nodes[v as usize].linked).map(|&v| g.nodes[v as usize].x);
            let (lo, hi) = xs.fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
            hi - lo
        };
        (width(&gf), width(&laid_out(&full, 0)))
    };
    eprintln!("  60 linked chapters of 30 pages: {span:.0} wide, {old_span:.0} with satellites");
    check!(span * 1.5 < old_span);
    // 50 more plain pages and 10 named unlinked ones, and an unlinked page opened: the gen and every
    // visible position stay as they were
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &gf);
    let mut more = full.clone();
    for k in 0..50 {
        more.nb.page_ids.push(9000 + k);
    }
    for c in 0..10u32 {
        name(&mut more, c * 100 + 7, "Named, no links");
    }
    let mut again = linked_built(&more, 9003);
    check!(graph_gen(&again) == ini.gen);
    check!(reopen(&mut again, &ini) == GraphPlan::Ready);
    check!(lean.iter().all(|&pid| node(&again, pid).x.to_bits() == node(&gf, pid).x.to_bits() && node(&again, pid).y.to_bits() == node(&gf, pid).y.to_bits()));
    check!(graph_persisted(node(&again, 101)) && !graph_persisted(node(&again, 107)));
}

// Links to the same page of another notebook count once, as links to one page of this one do.
fn external_links_count_distinct_pages() {
    let mut b = book(&[1, 2, 3]);
    link_raw(&mut b, 1, 1, "n:other-book:p5");
    link_raw(&mut b, 1, 2, "n:other-book:p5");
    link_raw(&mut b, 1, 3, "n:other-book:p6");
    link_raw(&mut b, 2, 4, "n:other-book:p5");
    check!(link(&mut b, 1, 5, 3) == 0 && link(&mut b, 1, 6, 3) == 0);
    let g = linked_built(&b, 0);
    check!(node(&g, 1).links_to == 3 && node(&g, 2).links_to == 1 && node(&g, 3).linked_from == 1);
}

// ---- what a changed map moves ----

// A laid-out chain of linked pages gets one new link between its two ends: only those two pages and
// their neighbours move, the new edge comes in from across the map, and reopening it again moves
// nothing at all.
fn new_link_moves_only_its_ends() {
    let pids = scattered_pids(40);
    let mut b = book(&pids);
    for k in 1..pids.len() {
        check!(link(&mut b, pids[k - 1], 1, pids[k]) == 0);
    }
    for k in (0..40).step_by(8) {
        name(&mut b, pids[k], &format!("Part {k}"));
    }
    let mut g = linked_built(&b, 0);
    graph_layout(&mut g, &mut || {});
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &g);
    // reopened unchanged: bit for bit
    let mut same = linked_built(&b, pids[3]);
    check!(reopen(&mut same, &ini) == GraphPlan::Ready && same_places(&same, &g));
    // the two linked pages furthest apart
    let (mut fa, mut fb, mut far) = (0u32, 0u32, 0.0f32);
    for &x in &pids {
        for &y in &pids {
            if dist(node(&g, x), node(&g, y)) > far {
                (fa, fb, far) = (x, y, dist(node(&g, x), node(&g, y)));
            }
        }
    }
    check!(far > 3.0 * GRAPH_SPRING_L);
    let mut more = b.clone();
    check!(link(&mut more, fa, 9, fb) == 0);
    let mut h = linked_built(&more, 0);
    check!(graph_gen(&h) != ini.gen);
    check!(reopen(&mut h, &ini) == GraphPlan::Ready);
    let near = |pid: u32| {
        let i = graph_node_of(&h, pid);
        pid == fa || pid == fb || graph_neighbours(&h, i).iter().any(|&j| h.nodes[j as usize].pid == fa || h.nodes[j as usize].pid == fb)
    };
    let shifted: Vec<u32> = pids.iter().copied().filter(|&pid| moved(&g, &h, pid) > 0.0).collect();
    eprintln!("  one new link across a 40-page chain: {} pages moved", shifted.len());
    check!(!shifted.is_empty() && shifted.iter().all(|&pid| near(pid)));
    check!(dist(node(&h, fa), node(&h, fb)) < far * 0.75 && overlaps(&h) == 0 && all_finite(&h));
    // stored and reopened: nothing moves again
    graph_ini_store(&mut ini, &h);
    let mut back = linked_built(&more, 0);
    check!(reopen(&mut back, &ini) == GraphPlan::Ready && same_places(&back, &h));
    // a map whose edges are all long has no stressed edge; too many stressed nodes ask for a warm
    // layout instead
    let wide = all_force_book(200, 300, 81);
    let mut w = linked_built(&wide, 0);
    graph_layout(&mut w, &mut || {});
    let mut scaled = g.clone();
    for n in &mut scaled.nodes {
        n.x *= 4.0;
        n.y *= 4.0;
    }
    check!(graph_relax_stressed(&mut scaled, &mut || {}) == 0);
    // a freshly laid-out map, dense or sparse, has none
    for m in [&w, &g] {
        let mut fresh = m.clone();
        check!(graph_relax_stressed(&mut fresh, &mut || {}) == 0);
    }
    let mut torn = w.clone();
    for &v in torn.force.iter().take(20) {
        torn.nodes[v as usize].x += 5000.0;
    }
    let before = torn.clone();
    check!(graph_relax_stressed(&mut torn, &mut || {}) == -2 && same_places(&torn, &before));
}

// More than GRAPH_RELAYOUT_PCT new pages: the map is laid out again from its kept positions, so the
// pages it kept stay about where they were (the median within half a rest length), with the usual
// invariants and the same result every time.
fn warm_layout_keeps_the_shape() {
    let base = random_book(500, 260, 30, 91);
    let mut g = linked_built(&base, 0);
    graph_layout(&mut g, &mut || {});
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &g);
    let kept: Vec<u32> = g.force.iter().map(|&v| g.nodes[v as usize].pid).collect();
    let mut more = base.clone();
    let mut r = Lcg(92);
    let add = kept.len() * 15 / 100 + 1;
    for k in 0..add as u32 {
        let pid = 20_000 + k;
        more.nb.page_ids.push(pid);
        check!(link(&mut more, pid, 1, kept[r.below(kept.len() as u32) as usize]) == 0);
    }
    let mut h = linked_built(&more, 0);
    check!(graph_apply_ini(&mut h, &ini) == GraphPlan::Full);
    let mut h2 = h.clone();
    check!(graph_layout_warm(&mut h, &mut || {}));
    check!(graph_layout_warm(&mut h2, &mut || {}) && same_places(&h, &h2));
    let mut moves: Vec<f32> = kept.iter().map(|&pid| moved(&g, &h, pid)).collect();
    moves.sort_unstable_by(|a, b| a.total_cmp(b));
    let median = moves[moves.len() / 2];
    // what the RC7 view did: a full layout from the seed
    let mut cold = linked_built(&more, 0);
    graph_layout(&mut cold, &mut || {});
    let mut cold_moves: Vec<f32> = kept.iter().map(|&pid| moved(&g, &cold, pid)).collect();
    cold_moves.sort_unstable_by(|a, b| a.total_cmp(b));
    eprintln!("  {add} new pages on {} linked: kept pages moved {median:.1} (median), {:.1} laid out afresh, in {} steps",
        kept.len(), cold_moves[cold_moves.len() / 2], h.relaxed);
    check!(median < GRAPH_SPRING_L * 0.5);
    check!(force_placed(&h) && all_finite(&h) && overlaps(&h) == 0 && h.laid == GraphLaid::Done);
    check!(h.relaxed <= GRAPH_WARM_STEPS);
    // the view's seed while a child runs it: kept positions, the new pages beside them
    let mut shown = linked_built(&more, 0);
    graph_apply_ini(&mut shown, &ini);
    check!(graph_seed_kept(&mut shown, &mut || {}) && shown.laid == GraphLaid::Seed && force_placed(&shown) && overlaps(&shown) == 0);
    check!(kept.iter().all(|&pid| moved(&g, &shown, pid) == 0.0));
}

// The relaxation stops once settled, well before GRAPH_STEPS on a typical notebook, and still lays
// it out the same way each time with no overlap.
fn relaxation_stops_when_settled() {
    for (pages, links, names, seed) in [(300usize, 200usize, 30usize, 95u64), (800, 300, 40, 75)] {
        let b = random_book(pages, links, names, seed);
        let mut g = linked_built(&b, 0);
        graph_layout(&mut g, &mut || {});
        let mut again = linked_built(&b, 0);
        graph_layout(&mut again, &mut || {});
        eprintln!("  {pages} pages, {} linked: settled in {} steps", g.force.len(), g.relaxed);
        check!(g.relaxed < GRAPH_STEPS && g.relaxed >= 60 && same_places(&g, &again) && overlaps(&g) == 0);
    }
}

// A page named on a laid-out map grows, while a new page is placed elsewhere: the repair still sets
// the grown dot apart from its neighbours.
fn repair_runs_with_new_pages() {
    let b = random_book(120, 160, 0, 97);
    let mut g = linked_built(&b, 0);
    graph_layout(&mut g, &mut || {});
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &g);
    // the unnamed linked page closest to another dot
    let mut best = (f32::MAX, 0u32);
    for &v in &g.force {
        for &w in &g.force {
            let (p, q) = (&g.nodes[v as usize], &g.nodes[w as usize]);
            if v != w && !p.named {
                let gap = dist(p, q) - p.r - q.r;
                if gap < best.0 {
                    best = (gap, p.pid);
                }
            }
        }
    }
    let mut more = b.clone();
    name(&mut more, best.1, "Grown");
    let new_pid = 50_000;
    more.nb.page_ids.push(new_pid);
    check!(link(&mut more, new_pid, 1, g.nodes[g.force[0] as usize].pid) == 0);
    let mut h = linked_built(&more, 0);
    check!(graph_apply_ini(&mut h, &ini) == GraphPlan::Incremental);
    check!(graph_place_new(&mut h, &mut || {}) == 1);
    check!(overlaps(&h) == 0 && force_placed(&h));
}

#[test]
fn test_graph() {
    run!(bands_and_degrees);
    run!(radius_formula);
    run!(pid_table);
    run!(duplicate_pages);
    run!(layout_deterministic);
    run!(components_apart);
    run!(no_overlap_no_nan);
    run!(satellites_near_head);
    run!(layout_pumps);
    run!(incremental_moves_only_the_neighbourhood);
    run!(large_incremental_moves_only_the_neighbourhood);
    run!(open_page_placed_beside_its_section);
    run!(satellites_ignore_the_open_page);
    run!(relayout_rule);
    run!(around_rings);
    run!(ini_round_trip);
    run!(ini_limits);
    run!(corrupt_file_refused);
    run!(gen_encoding_and_mismatch);
    run!(layout_job_writes_atomically);
    run!(big_layout_within_bound);
    run!(linked_only_model);
    run!(external_links_count_distinct_pages);
    run!(new_link_moves_only_its_ends);
    run!(warm_layout_keeps_the_shape);
    run!(relaxation_stops_when_settled);
    run!(repair_runs_with_new_pages);
    let _ = std::fs::remove_dir_all(tmp());
    done();
}
