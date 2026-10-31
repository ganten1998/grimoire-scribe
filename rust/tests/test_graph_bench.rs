// test_graph_bench: what a layout costs on 300 to 600 linked pages, and how much of a laid-out map
// one new link moves. It prints its numbers (run it with --release -- --nocapture to compare host
// builds) and checks only loose invariants, so it stays a test the gate can run.
#![allow(clippy::needless_range_loop)]
mod graph_corpus;
mod testlib;
use graph_corpus::*;
use grimoire::prelude::*;
use testlib::*;

fn build(b: &Book) -> GraphModel {
    let mut g = GraphModel::default();
    check!(graph_build(&mut g, &b.nb, &b.nav, 0, true) == 0);
    g
}

// The force nodes' bounding box and the mean edge length in rest lengths.
fn shape(g: &GraphModel) -> (f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &v in &g.force {
        let n = &g.nodes[v as usize];
        x0 = x0.min(n.x - n.r);
        y0 = y0.min(n.y - n.r);
        x1 = x1.max(n.x + n.r);
        y1 = y1.max(n.y + n.r);
    }
    let mut sum = 0.0f64;
    for e in &g.edges {
        let (a, b) = (&g.nodes[e.a as usize], &g.nodes[e.b as usize]);
        sum += ((a.x - b.x).hypot(a.y - b.y) / GRAPH_SPRING_L) as f64;
    }
    (x1 - x0, y1 - y0, (sum / g.edges.len().max(1) as f64) as f32)
}

// Sections: `chapters` named pages, each followed by `plain` unnamed pages without links, the
// chapters linked in a chain and by a few cross links.
fn sections(chapters: u32, plain: u32, seed: u64) -> Book {
    let mut pids = Vec::new();
    for c in 0..chapters {
        for k in 0..=plain {
            pids.push(c * 100 + k + 1);
        }
    }
    let mut b = book(&pids);
    let mut r = Lcg(seed);
    for c in 0..chapters {
        name(&mut b, c * 100 + 1, &format!("Chapter {c}"));
        if c > 0 {
            check!(link(&mut b, c * 100 + 1, 1, (c - 1) * 100 + 1) == 0);
        }
        if c % 3 == 0 {
            let to = r.below(chapters) * 100 + 1;
            if to != c * 100 + 1 {
                link(&mut b, c * 100 + 1, 2, to);
            }
        }
    }
    b
}

fn timed_layout(label: &str, b: &Book) {
    let reps = 3;
    let mut best = u128::MAX;
    let mut g = build(b);
    let mut pumps = 0;
    for _ in 0..reps {
        g = build(b);
        pumps = 0;
        let t = std::time::Instant::now();
        check!(graph_layout(&mut g, &mut || pumps += 1));
        best = best.min(t.elapsed().as_micros());
    }
    let (w, h, edge) = shape(&g);
    eprintln!("  {label}: {} force, {} edges: {:.1} ms ({} steps, {pumps} pumps), {w:.0} x {h:.0}, mean edge {edge:.2} L",
        g.force.len(), g.edges.len(), best as f64 / 1000.0, g.relaxed);
}

// One new link between two far-apart pages of a laid-out map: how many dots move more than half a
// rest length when the map is opened again.
fn one_new_link(label: &str, b: &Book) {
    let mut g = build(b);
    graph_layout(&mut g, &mut || {});
    let mut ini = GraphIni::default();
    graph_ini_store(&mut ini, &g);
    // the two linked pages furthest apart
    let (mut fa, mut fb, mut far) = (0usize, 0usize, 0.0f32);
    for &a in &g.force {
        for &c in &g.force {
            let (p, q) = (&g.nodes[a as usize], &g.nodes[c as usize]);
            let d = (p.x - q.x).hypot(p.y - q.y);
            if d > far && graph_edge_between(&g, a, c) < 0 {
                (fa, fb, far) = (a as usize, c as usize, d);
            }
        }
    }
    let mut more = b.clone();
    check!(link(&mut more, g.nodes[fa].pid, 99, g.nodes[fb].pid) == 0);
    let mut h = build(&more);
    let t = std::time::Instant::now();
    // the view's steps (mapview's layout)
    let mut plan = graph_apply_ini(&mut h, &ini);
    let mut moved_by = "kept";
    if plan != GraphPlan::Full {
        graph_place_new(&mut h, &mut || {});
        match graph_relax_stressed(&mut h, &mut || {}) {
            -2 => plan = GraphPlan::Full,
            n if n > 0 => moved_by = "stressed edge",
            _ => {}
        }
    }
    if plan == GraphPlan::Full {
        moved_by = "warm layout";
        graph_layout_warm(&mut h, &mut || {});
    }
    let us = t.elapsed().as_micros();
    let moved = g.force.iter().filter(|&&v| {
        let i = graph_node_of(&h, g.nodes[v as usize].pid) as usize;
        (h.nodes[i].x - g.nodes[v as usize].x).hypot(h.nodes[i].y - g.nodes[v as usize].y) > GRAPH_SPRING_L * 0.5
    }).count();
    eprintln!("  {label}: one new link ({moved_by}): {:.1} ms, {moved} of {} dots moved more than L/2", us as f64 / 1000.0, g.force.len());
    check!(moved * 4 < g.force.len());
}

fn bench() {
    let a = all_force_book(300, 450, 71);
    let b = all_force_book(450, 700, 72);
    let c = all_force_book(600, 900, 73);
    let s = sections(60, 9, 74);
    timed_layout("300 pages", &a);
    timed_layout("450 pages", &b);
    timed_layout("600 pages", &c);
    timed_layout("60 chapters of 10 pages", &s);
    timed_layout("random 800 pages", &random_book(800, 300, 40, 75));
    timed_layout("random 1200 pages", &random_book(1200, 500, 80, 76));
    timed_layout("200 chapters of 3 pages", &sections(200, 2, 77));
    one_new_link("300 pages", &a);
    one_new_link("60 chapters of 10 pages", &s);
}

#[test]
fn test_graph_bench() {
    run!(bench);
    done();
}
