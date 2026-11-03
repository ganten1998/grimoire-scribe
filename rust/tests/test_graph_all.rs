// test_graph_all: the Graph of all notebooks (docs/beta2/GRAPH-ALL.md, steps 3 and 4): its model
// (graph_build_all), graph-all.ini, the islands and the placement of new components. Invariants only,
// never coordinates; bit-identical comparisons are within one run.
#![allow(clippy::needless_range_loop)]
mod graph_corpus;
mod testlib;
use graph_corpus::library::*;
use graph_corpus::Lcg;
use grimoire::prelude::*;
use std::time::Instant;
use testlib::*;

fn scratch(name: &str) -> String {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join(format!("grimoire-test-graph-all-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d.join(name).to_string_lossy().into_owned()
}

fn page(id: &str, pages: &[u32]) -> GraphBook {
    GraphBook { id: FixedStr::from_str(id), name: id.to_string(), pages: pages.to_vec(), ..GraphBook::default() }
}
fn to_page(b: &mut GraphBook, src: u32, p: u32) {
    b.links.push(GraphLink { src, to: GraphTo::Page(p) });
    b.links.sort_unstable();
}
fn to_other(b: &mut GraphBook, src: u32, id: &str, p: u32) {
    b.links.push(GraphLink { src, to: GraphTo::Other(FixedStr::from_str(id), p) });
    b.links.sort_unstable();
}
fn build(books: &[&GraphBook]) -> GraphModel {
    let mut g = GraphModel::default();
    check!(graph_build_all(&mut g, books, &[], None) == 0);
    g
}
fn node(g: &GraphModel, id: &str, pid: u32) -> i32 {
    let b = g.books.iter().position(|b| b.as_str() == id).map_or(u16::MAX, |b| b as u16);
    graph_node_in(g, b, pid)
}
fn laid(books: &[&GraphBook]) -> GraphModel {
    let mut g = build(books);
    check!(graph_layout(&mut g, &mut || {}));
    g
}
// Every node's (notebook id, page id) and position.
fn places(g: &GraphModel) -> Vec<(String, u32, f32, f32)> {
    let mut v: Vec<_> = g.nodes.iter().map(|n| (g.books[n.book as usize].as_str().to_string(), n.pid, n.x, n.y)).collect();
    v.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    v
}
// How many kept nodes (in `before`) moved in `after`.
fn moved(before: &GraphModel, after: &GraphModel) -> usize {
    let mut n = 0;
    for b in &before.nodes {
        let id = before.books[b.book as usize];
        let i = node(after, id.as_str(), b.pid);
        if i >= 0 {
            let a = &after.nodes[i as usize];
            if a.x != b.x || a.y != b.y {
                n += 1;
            }
        }
    }
    n
}
fn overlaps(g: &GraphModel) -> usize {
    let mut n = 0;
    for i in 0..g.nodes.len() {
        for j in i + 1..g.nodes.len() {
            let (a, b) = (&g.nodes[i], &g.nodes[j]);
            if (a.x - b.x).hypot(a.y - b.y) < a.r + b.r {
                n += 1;
            }
        }
    }
    n
}
fn reopen(g: &mut GraphModel, ini: &GraphAllIni) -> GraphPlan {
    let mut plan = graph_all_apply_ini(g, ini);
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

fn identity_edges_badges_and_broken_links() {
    // Two notebooks, both starting at page 1: page 1 of A and page 1 of B are two nodes.
    let mut a = page("A", &[1, 2, 3, 4]);
    let mut b = page("B", &[1, 2, 3]);
    to_page(&mut a, 1, 2);
    to_other(&mut a, 2, "B", 1);
    to_other(&mut a, 2, "B", 1); // (twice: one link counted once in the pair's multiplicity? no, dedup in the book)
    a.links.dedup();
    a.links.push(GraphLink { src: 3, to: GraphTo::Index });
    to_other(&mut a, 3, "Gone", 1); // a notebook that is not there: broken
    to_other(&mut a, 4, "B", 99); // a page that is not there, on a page with no other link: no node
    to_other(&mut b, 1, "A", 1); // back to A's page 1: one edge both ways with A1? no: B1 to A1
    to_page(&mut b, 2, 2); // a loop: linked, no edge
    a.names.push((1, "Start".into()));
    let g = build(&[&a, &b]);
    check!(g.books.len() == 2 && graph_is_all(&g));
    let (a1, a2, a3, a4) = (node(&g, "A", 1), node(&g, "A", 2), node(&g, "A", 3), node(&g, "A", 4));
    let (b1, b2, b3) = (node(&g, "B", 1), node(&g, "B", 2), node(&g, "B", 3));
    check!(a1 >= 0 && a2 >= 0 && a3 >= 0 && b1 >= 0 && b2 >= 0);
    check!(a1 != b1);
    check!(a4 < 0 && b3 < 0);
    check!(g.nodes.len() == 5);
    check!(g.nodes[a1 as usize].named && !g.nodes[b1 as usize].named);
    check!(g.nodes[a1 as usize].number == 1 && g.nodes[b2 as usize].number == 2);
    check!(graph_edge_between(&g, a1, a2) >= 0);
    check!(graph_edge_between(&g, a2, b1) >= 0);
    check!(graph_edge_between(&g, b1, a1) >= 0);
    check!(g.edges.len() == 3);
    check!(g.nodes[a3 as usize].index_links == 1 && g.nodes[a3 as usize].degree == 0);
    check!(g.nodes[a3 as usize].broken == 1);
    check!(g.nodes[b2 as usize].self_links == 1 && g.nodes[b2 as usize].degree == 0);
    check!(g.nodes[a2 as usize].links_to == 1 && g.nodes[a2 as usize].linked_from == 1);
    check!(g.nodes[b1 as usize].linked_from == 1 && g.nodes[b1 as usize].links_to == 1);
    // A notebook that is there but not read (skip): links to it are neither edges nor broken.
    let mut g2 = GraphModel::default();
    check!(graph_build_all(&mut g2, &[&a, &b], &[FixedStr::from_str("Gone")], None) == 0);
    check!(g2.nodes[node(&g2, "A", 3) as usize].broken == 0);
    // The open page, by notebook and page id.
    let mut g3 = GraphModel::default();
    check!(graph_build_all(&mut g3, &[&b, &a], &[], Some(("B", 1))) == 0);
    check!(g3.open == node(&g3, "B", 1) && g3.nodes[g3.open as usize].open);
    // The order given does not matter.
    check!(g3.books == g.books);
    // Keys: unique, never 0.
    let mut keys: Vec<u32> = g.nodes.iter().map(|n| n.key).collect();
    keys.sort_unstable();
    keys.dedup();
    check!(keys.len() == g.nodes.len() && keys[0] != 0);
}

fn lookup_at_4096_and_16384_slots() {
    for (books, pages) in [(3usize, 300u32), (6, 1000)] {
        let lib = library(books, pages, 1.6, 0.05, 41);
        let g = build(&refs(&lib));
        check!(!g.nodes.is_empty());
        let slots = graph_slots_for(g.nodes.len());
        check!(slots == if g.nodes.len() * 2 <= 4096 { 4096 } else { (g.nodes.len() * 2).next_power_of_two() });
        for (i, n) in g.nodes.iter().enumerate() {
            check!(graph_node_in(&g, n.book, n.pid) == i as i32);
        }
        check!(graph_node_in(&g, 0, pages + 1) < 0);
        check!(graph_node_in(&g, books as u16, 1) < 0);
        if books == 6 {
            check!(slots == 16384);
        }
    }
}

fn caps_are_refused() {
    // Five notebooks of 2000 pages, every page linked: 10000 linked pages, past 8000.
    let mut lib = Vec::new();
    for k in 0..5 {
        let mut b = GraphBook { id: lib_id(k), pages: (1..=2000).collect(), ..GraphBook::default() };
        for p in 1..2000 {
            b.links.push(GraphLink { src: p, to: GraphTo::Page(p + 1) });
        }
        lib.push(b);
    }
    let mut g = GraphModel::default();
    check!(graph_build_all(&mut g, &refs(&lib), &[], None) == GRAPH_ALL_TOO_BIG);
    check!(g.nodes.is_empty() && g.open == -1);
    let four = refs(&lib[..4]);
    check!(graph_build_all(&mut g, &four, &[], None) == 0);
    check!(g.nodes.len() == 8000);
    // Past 40000 links between pages.
    let mut dense = GraphBook { id: lib_id(9), pages: (1..=1000).collect(), ..GraphBook::default() };
    for p in 1..=1000u32 {
        for k in 1..=41u32 {
            dense.links.push(GraphLink { src: p, to: GraphTo::Page(1 + (p + k * 13) % 1000) });
        }
    }
    dense.links.sort_unstable();
    dense.links.dedup();
    check!(dense.links.len() > GRAPH_ALL_EDGES_MAX);
    check!(graph_build_all(&mut g, &[&dense], &[], None) == GRAPH_ALL_TOO_BIG);
}

fn deterministic() {
    let lib = library(12, 60, 1.1, 0.12, 7);
    let a = laid(&refs(&lib));
    let mut rev = refs(&lib);
    rev.reverse();
    let b = laid(&rev);
    check!(places(&a) == places(&b));
    check!(graph_gen(&a) == graph_gen(&b));
    check!(overlaps(&a) == 0);
    check!(a.nodes.iter().all(|n| n.placed && n.x.is_finite() && n.y.is_finite()));
}

fn islands_never_overlap() {
    for seed in [3u64, 19] {
        let lib = library(15, 40, 1.2, 0.08, seed);
        let g = laid(&refs(&lib));
        let isl = graph_islands(&g);
        check!(isl.len() >= 10);
        for i in 0..isl.len() {
            for j in i + 1..isl.len() {
                let (p, q) = (isl[i], isl[j]);
                check!((p.x - q.x).hypot(p.y - q.y) >= p.r + q.r);
            }
        }
        // Each component sits whole in its home notebook's island (the notebook most of its pages are in).
        for c in 0..g.ncomp {
            let mut books: Vec<u16> = g.nodes.iter().filter(|n| n.comp == c).map(|n| n.book).collect();
            books.sort_unstable();
            let mut home = (0usize, 0u16);
            let mut k = 0;
            while k < books.len() {
                let j = books[k..].iter().take_while(|&&b| b == books[k]).count();
                if j > home.0 {
                    home = (j, books[k]);
                }
                k += j;
            }
            let d = isl.iter().find(|d| d.book == home.1).copied().unwrap_or_default();
            check!(g.nodes.iter().filter(|n| n.comp == c).all(|n| (n.x - d.x).hypot(n.y - d.y) <= d.r + 0.5));
        }
    }
}

// Left out of the Graph (2.0.0-beta.2, the owner's ruling of 2026-11-03): a notebook by its own setting, or
// by a folder it is in at any depth (a notebook moved into it with it); "School" does not cover "Schoolwork".
// Built without the left-out notebook (as libindex_graph builds, the notebook in skip), a link to it is no edge
// and nothing broken, and the page with it is linked only through its other links.
fn left_out_notebooks_and_folders() {
    let mut a = page("A", &[1, 2, 3]);
    let mut b = page("B", &[1, 2]);
    b.folder = "School/2025".into();
    let none: Vec<String> = Vec::new();
    let school = vec!["School".to_string()];
    check!(!graph_omits(&a, &none) && !graph_omits(&b, &none));
    a.omit = true;
    check!(graph_omits(&a, &none));
    a.omit = false;
    check!(graph_omits(&b, &school)); // (nested: School covers School/2025)
    check!(graph_omits(&b, &["School/2025".to_string()])); // (its own folder)
    check!(!graph_omits(&b, &["School/2025/Old".to_string()])); // (a folder inside it covers nothing above)
    check!(!graph_omits(&a, &school)); // (the top level is in no folder)
    a.folder = "Schoolwork".into();
    check!(!graph_omits(&a, &school)); // (a component boundary)
    a.folder = "School".into();
    check!(graph_omits(&a, &school)); // (moved in: left out with the folder)
    a.folder.clear();
    // A1 -> B2 and A2 -> A3; B left out: A1 is no node, A2 and A3 are.
    to_other(&mut a, 1, "B", 2);
    to_page(&mut a, 2, 3);
    let skip = [FixedStr::from_str("B")];
    let mut g = GraphModel::default();
    check!(graph_build_all(&mut g, &[&a], &skip, None) == 0);
    check!(node(&g, "A", 1) < 0 && node(&g, "A", 2) >= 0 && node(&g, "A", 3) >= 0);
    check!(g.edges.len() == 1 && g.nodes.iter().all(|n| n.broken <= 0));
    // Shown, B's page 2 and A1 are nodes, and their link an edge.
    check!(graph_build_all(&mut g, &[&a, &b], &[], None) == 0);
    check!(node(&g, "A", 1) >= 0 && node(&g, "B", 2) >= 0 && g.edges.len() == 2);
}

fn ini_round_trip_and_refusal() {
    let lib = library(5, 30, 1.0, 0.1, 5);
    let g = laid(&refs(&lib));
    let mut ini = GraphAllIni::default();
    check!(graph_all_ini_store(&mut ini, &g));
    check!(!graph_all_ini_store(&mut ini, &g));
    ini.mode = GraphMode::Around;
    ini.unknown = b"future=1\npos.X.1=later\n".to_vec();
    let path = scratch("all.ini");
    check!(graph_all_ini_write_path(&path, &mut ini) == 0);
    let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    check!(text.starts_with("# Grimoire: the Graph of all notebooks"));
    check!(text.contains("mode=around\n") && text.contains("future=1\n") && text.contains("pos.X.1=later\n"));
    check!(text.contains(&format!("pos.{}.", lib[0].id.as_str())));
    let mut back = GraphAllIni::default();
    check!(graph_all_ini_read_path(&path, &mut back) == 0);
    check!(back.pos == ini.pos && back.gen == ini.gen && back.mode == GraphMode::Around);
    // The last line for a page wins; a carried line for a page it writes is not written back.
    std::fs::write(&path, format!("pos.A.1=1,1\npos.A.1=5,6\n{}", "x=1\n")).unwrap();
    check!(graph_all_ini_read_path(&path, &mut back) == 0);
    check!(back.pos == vec![GraphAllPos { id: FixedStr::from_str("A"), pid: 1, x: 5, y: 6 }]);
    // The orphan build's lines (orphans=, a notebook's block at page id 0) are dropped, not carried:
    // one block line a notebook would pass GRAPH_UNKNOWN_MAX and get the file refused.
    let mut old = String::from("gen=00000001\norphans=0\npos.A.2=3,4\n");
    for k in 0..800 {
        old.push_str(&format!("pos.20261010-143205-{k:04}.0=-520,610\n"));
    }
    old.push_str("pos.B.10=7,7\nfuture=2\n");
    check!(old.len() > GRAPH_UNKNOWN_MAX);
    std::fs::write(&path, &old).unwrap();
    check!(graph_all_ini_read_path(&path, &mut back) == 0);
    check!(back.writable && back.unknown == b"future=2\n".to_vec());
    check!(back.pos.iter().map(|p| (p.id.as_str().to_string(), p.pid)).collect::<Vec<_>>() == vec![("A".to_string(), 2), ("B".to_string(), 10)]);
    check!(graph_all_ini_write_path(&path, &mut back) == 0);
    let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    check!(!text.contains("orphans") && !text.contains(".0=") && text.contains("future=2\n"));
    // A file that does not read is refused and never overwritten.
    std::fs::write(&path, b"gen=1\0").unwrap();
    check!(graph_all_ini_read_path(&path, &mut back) == -1);
    check!(!back.writable && graph_all_ini_write_path(&path, &mut back) == -1);
    let mut too_many = String::new();
    for k in 0..=GRAPH_ALL_NODES_MAX {
        too_many.push_str(&format!("pos.A.{}=0,0\n", k + 1));
    }
    std::fs::write(&path, too_many).unwrap();
    check!(graph_all_ini_read_path(&path, &mut back) == -1);
    // Applied: every position back, plan Ready, nothing moves.
    let mut h = build(&refs(&lib));
    check!(graph_all_apply_ini(&mut h, &ini) == GraphPlan::Ready);
    check!(places(&h) == places(&g));
}

fn stable_across_notebooks() {
    let lib = library(10, 50, 1.1, 0.1, 13);
    let g = laid(&refs(&lib));
    let mut ini = GraphAllIni::default();
    graph_all_ini_store(&mut ini, &g);
    // Pages moved within a notebook, a notebook renamed and moved to another folder: nothing moves.
    let mut shuffled = lib.clone();
    let mut r = Lcg(5);
    for k in (1..shuffled[2].pages.len()).rev() {
        let j = r.below(k as u32 + 1) as usize;
        shuffled[2].pages.swap(k, j);
    }
    shuffled[3].name = "Renamed".into();
    shuffled[3].folder = "Elsewhere/Deeper".into();
    let mut h = build(&refs(&shuffled));
    check!(reopen(&mut h, &ini) == GraphPlan::Ready);
    check!(moved(&g, &h) == 0);
    // A notebook deleted: its nodes vanish, every other keeps its position.
    let fewer: Vec<&GraphBook> = lib.iter().enumerate().filter(|(k, _)| *k != 4).map(|(_, b)| b).collect();
    let mut h = build(&fewer);
    reopen(&mut h, &ini);
    check!(moved(&g, &h) == 0);
    check!(h.nodes.iter().all(|n| h.books[n.book as usize] != lib[4].id));
    // A new notebook with 50 linked pages, linked among themselves: placed as a block, no kept node
    // moves, nothing overlaps, and the library is not laid out again.
    let mut more = lib.clone();
    let mut nb = GraphBook { id: FixedStr::from_str("2099-new"), name: "New".into(), pages: (1..=60).collect(), ..GraphBook::default() };
    for p in 1..50 {
        nb.links.push(GraphLink { src: p, to: GraphTo::Page(p + 1) });
    }
    nb.links.push(GraphLink { src: 50, to: GraphTo::Page(2) });
    nb.links.sort_unstable();
    more.push(nb);
    let mut h = build(&refs(&more));
    let plan = graph_all_apply_ini(&mut h, &ini);
    check!(plan == GraphPlan::Incremental);
    // Nothing changed between the kept pages: the kept part's gen is the file's (the view skips the stretched
    // edge pass); a new link between two kept pages changes it.
    check!(graph_gen(&h) != ini.gen);
    check!(graph_gen_of(&h, |n| n.placed) == ini.gen);
    let mut linked = lib.clone();
    let (x, y) = (g.nodes[0], g.nodes[g.nodes.len() - 1]);
    let id = g.books[x.book as usize];
    let to = if g.books[y.book as usize] == id { GraphTo::Page(y.pid) } else { GraphTo::Other(g.books[y.book as usize], y.pid) };
    let k = linked.iter().position(|b| b.id == id).unwrap();
    linked[k].links.push(GraphLink { src: x.pid, to });
    linked[k].links.sort_unstable();
    let mut l = build(&refs(&linked));
    graph_all_apply_ini(&mut l, &ini);
    check!(graph_gen_of(&l, |n| n.placed) != ini.gen);
    let fresh = h.nodes.iter().filter(|n| !n.placed).count();
    check!(fresh >= 50);
    check!(graph_place_new(&mut h, &mut || {}) >= 0);
    check!(h.nodes.iter().all(|n| n.placed));
    check!(moved(&g, &h) == 0);
    check!(overlaps(&h) == 0);
    // The kept layout in the child (graph_layout_warm's block placement) moves nothing kept either
    // before its relaxation: graph_seed_kept shows it.
    let mut s = build(&refs(&more));
    graph_all_apply_ini(&mut s, &ini);
    check!(graph_seed_kept(&mut s, &mut || {}));
    check!(moved(&g, &s) == 0);
    check!(s.nodes.iter().all(|n| n.placed));
    // Pages copied into a notebook with their links (a new component at home in lib[2]): placed beside
    // lib[2]'s island, the island nearest to it, and no kept node moves.
    let apart = library(10, 50, 1.1, 0.0, 13);
    let g = laid(&refs(&apart));
    let mut ini = GraphAllIni::default();
    graph_all_ini_store(&mut ini, &g);
    let lib = apart;
    let mut copied = lib.clone();
    let base = copied[2].pages.len() as u32;
    copied[2].pages.extend(base + 1..=base + 12);
    for p in base + 1..base + 12 {
        copied[2].links.push(GraphLink { src: p, to: GraphTo::Page(p + 1) });
    }
    copied[2].links.sort_unstable();
    let mut h = build(&refs(&copied));
    check!(graph_all_apply_ini(&mut h, &ini) == GraphPlan::Incremental);
    check!(graph_place_new(&mut h, &mut || {}) >= 0);
    check!(moved(&g, &h) == 0 && overlaps(&h) == 0);
    let new: Vec<&GraphNode> = h.nodes.iter().filter(|n| h.books[n.book as usize] == lib[2].id && n.pid > base).collect();
    check!(new.len() == 12);
    let (cx, cy) = (new.iter().map(|n| n.x).sum::<f32>() / 12.0, new.iter().map(|n| n.y).sum::<f32>() / 12.0);
    let gap = |d: &GraphIsland| (cx - d.x).hypot(cy - d.y) - d.r;
    let isl = graph_islands(&g);
    let home = isl.iter().find(|d| g.books[d.book as usize] == lib[2].id).copied().unwrap_or_default();
    check!(isl.iter().all(|d| gap(&home) <= gap(d)));
    // A new notebook whose pages link only among themselves is no relayout either: the 10% rule counts
    // only new pages of components that already have a placed page.
    let mut alone = lib.clone();
    let mut big = GraphBook { id: FixedStr::from_str("2099-alone"), pages: (1..=400).collect(), ..GraphBook::default() };
    for p in 1..400 {
        big.links.push(GraphLink { src: p, to: GraphTo::Page(p + 1) });
    }
    alone.push(big);
    let mut h = build(&refs(&alone));
    check!(graph_all_apply_ini(&mut h, &ini) == GraphPlan::Incremental);
    check!(h.nodes.iter().filter(|n| !n.placed).count() * 100 > g.nodes.len() * GRAPH_RELAYOUT_PCT as usize);
    // A few new pages linked into a kept component are placed beside their neighbours, as in one
    // notebook's Graph; many of them (past 10% of the kept pages) lay the map out again.
    let mut few = lib.clone();
    let mut nb = GraphBook { id: FixedStr::from_str("2099-few"), pages: (1..=3).collect(), ..GraphBook::default() };
    for p in 1..=3 {
        nb.links.push(GraphLink { src: p, to: GraphTo::Other(lib[1].id, 1 + p) });
    }
    few.push(nb);
    let mut h = build(&refs(&few));
    check!(graph_all_apply_ini(&mut h, &ini) == GraphPlan::Incremental);
    let mut many = lib.clone();
    let n = g.nodes.len() as u32 / 5;
    let mut nb = GraphBook { id: FixedStr::from_str("2099-many"), pages: (1..=n).collect(), ..GraphBook::default() };
    for p in 1..=n {
        nb.links.push(GraphLink { src: p, to: GraphTo::Other(lib[1].id, 1 + p % 50) });
    }
    many.push(nb);
    let mut h = build(&refs(&many));
    check!(graph_all_apply_ini(&mut h, &ini) == GraphPlan::Full);
}

fn job_writes_graph_all_ini() {
    let lib = library(6, 40, 1.1, 0.1, 21);
    let mut g = build(&refs(&lib));
    let path = scratch("job-all.ini");
    let _ = std::fs::remove_file(&path);
    let mut ini = GraphAllIni::default();
    check!(graph_all_layout_job(&mut g, &mut ini, &path) == 0);
    let mut back = GraphAllIni::default();
    check!(graph_all_ini_read_path(&path, &mut back) == 0);
    check!(back.pos.len() == g.nodes.len() && back.gen == graph_gen(&g));
    // The view's mode changed while the child ran: the child's write keeps it.
    check!(graph_all_ini_update_mode_at(&path, GraphMode::Around) == 0);
    let mut g2 = build(&refs(&lib));
    let mut ini2 = back.clone();
    check!(graph_all_layout_job(&mut g2, &mut ini2, &path) == 0);
    check!(graph_all_ini_read_path(&path, &mut back) == 0);
    check!(back.mode == GraphMode::Around);
}

// A notebook missing from the model when graph-all.ini is stored (the index being built again, a notebook.ini
// that did not read at a check, a notebook left out at the link cap) keeps its kept positions: it comes back
// where it was. The model's own notebooks are written as they are now (a page gone loses its position), and
// the carried positions never take the file past GRAPH_ALL_NODES_MAX.
fn store_keeps_the_notebooks_the_model_lacks() {
    // (no links across notebooks: each notebook's linked pages are the same with the others there or not)
    let lib = library(4, 60, 1.1, 0.0, 17);
    let g = laid(&refs(&lib));
    let mut ini = GraphAllIni::default();
    check!(graph_all_ini_store(&mut ini, &g));
    let all = ini.pos.clone();
    let of = |pos: &[GraphAllPos], id: FixedStr<STORE_ID_LEN>| pos.iter().filter(|p| p.id == id).copied().collect::<Vec<_>>();
    // Only notebooks 0 and 1 in the model: 2's and 3's positions stay as they were.
    let mut h = build(&refs(&lib[..2]));
    graph_all_apply_ini(&mut h, &ini);
    graph_all_ini_store(&mut ini, &h);
    for k in 2..4 {
        check!(!of(&all, lib[k].id).is_empty());
        check!(of(&ini.pos, lib[k].id) == of(&all, lib[k].id));
    }
    check!(ini.pos.windows(2).all(|w| (w[0].id.as_bytes(), w[0].pid) < (w[1].id.as_bytes(), w[1].pid)));
    // Back with every notebook: nothing to place, nothing moves.
    let mut back = build(&refs(&lib));
    check!(graph_all_apply_ini(&mut back, &ini) == GraphPlan::Ready);
    check!(moved(&g, &back) == 0);
    // A notebook of the model that lost its links: its positions go (it is in the model, as it is now).
    let mut fewer = lib.clone();
    fewer[1].links.clear();
    let f = build(&refs(&fewer));
    let in_model = f.books.contains(&lib[1].id);
    let mut gone = ini.clone();
    graph_all_ini_store(&mut gone, &f);
    if in_model {
        check!(of(&gone.pos, lib[1].id).is_empty());
    }
    // A notebook gone to the trash is forgotten (the view does it once every notebook has been checked).
    let mut trashed = ini.clone();
    check!(graph_all_ini_forget(&mut trashed, |id| id != lib[3].id.as_str()));
    check!(of(&trashed.pos, lib[3].id).is_empty() && of(&trashed.pos, lib[2].id) == of(&all, lib[2].id));
    check!(!graph_all_ini_forget(&mut trashed, |id| id != lib[3].id.as_str()));
    // The model's positions first, the carried ones only up to the cap.
    let mut full = GraphAllIni::default();
    for k in 0..GRAPH_ALL_NODES_MAX as u32 {
        full.pos.push(GraphAllPos { id: FixedStr::from_str("0000-elsewhere"), pid: k + 1, x: 0, y: 0 });
    }
    graph_all_ini_store(&mut full, &g);
    check!(full.pos.len() == GRAPH_ALL_NODES_MAX);
    check!(all.iter().all(|p| full.pos.contains(p)));
}

// Thousands of new components (a library whose layout was kept while only its first notebook had been read:
// the index's first fill) placed on the app's main thread in a time that grows with what is new, not with
// its square: 2826 components among 7025 pages took 2.3 s on the host (and some 20 times that on the Kindle).
fn many_new_components_are_placed_quickly() {
    let lib = library(100, 200, 0.22, 0.05, 31);
    let g = laid(&refs(&lib));
    let mut ini = GraphAllIni::default();
    graph_all_ini_store(&mut ini, &g);
    ini.pos.retain(|p| p.id == lib[0].id);
    let mut best = f64::MAX;
    let mut h = GraphModel::default();
    // (the faster of two runs: a machine busy with other work makes one slow)
    for _ in 0..2 {
        h = build(&refs(&lib));
        check!(graph_all_apply_ini(&mut h, &ini) == GraphPlan::Incremental);
        let t = Instant::now();
        check!(graph_place_new(&mut h, &mut || {}) >= 0);
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    eprintln!("  2826 new components placed in {best:.0} ms");
    check!(best < 800.0);
    check!(h.nodes.iter().all(|n| n.placed));
    // Notebook 0's components with no new page stay where they were (a new page's neighbours make room for
    // it), and nothing overlaps.
    let mut grew = vec![false; h.ncomp.max(0) as usize];
    for n in &h.nodes {
        if h.books[n.book as usize] != lib[0].id && n.comp >= 0 {
            grew[n.comp as usize] = true;
        }
    }
    let mut still = 0;
    for n in g.nodes.iter().filter(|n| g.books[n.book as usize] == lib[0].id) {
        let i = node(&h, lib[0].id.as_str(), n.pid);
        check!(i >= 0);
        let m = &h.nodes[i as usize];
        if m.comp >= 0 && !grew[m.comp as usize] {
            still += 1;
            check!(m.x == n.x && m.y == n.y);
        }
    }
    check!(still > 0);
    check!(overlaps(&h) == 0);
}

// graph_all_ini_update_mode at a path of the test's own (the real one is in the store's root).
fn graph_all_ini_update_mode_at(path: &str, mode: GraphMode) -> i32 {
    let mut latest = GraphAllIni::default();
    if graph_all_ini_read_path(path, &mut latest) != 0 {
        return -1;
    }
    latest.mode = mode;
    graph_all_ini_write_path(path, &mut latest)
}

#[test]
fn test_graph_all() {
    run!(identity_edges_badges_and_broken_links);
    run!(lookup_at_4096_and_16384_slots);
    run!(caps_are_refused);
    run!(deterministic);
    run!(islands_never_overlap);
    run!(left_out_notebooks_and_folders);
    run!(ini_round_trip_and_refusal);
    run!(stable_across_notebooks);
    run!(job_writes_graph_all_ini);
    run!(store_keeps_the_notebooks_the_model_lacks);
    run!(many_new_components_are_placed_quickly);
    done();
}
