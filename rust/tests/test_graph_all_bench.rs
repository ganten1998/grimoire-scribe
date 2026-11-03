// test_graph_all_bench: what the Graph of all notebooks costs on synthetic libraries (docs/beta2/GRAPH-ALL.md,
// section 4's table): the model, the seed, a full layout, placing 1% new pages, and graph-index.ini's text
// read back. It prints its numbers (run it with --release -- --nocapture to compare host builds) and checks only
// loose invariants, so it stays a test the gate can run.
mod graph_corpus;
mod testlib;
use graph_corpus::library::*;
use grimoire::prelude::*;
use std::time::Instant;
use testlib::*;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn case(label: &str, books: usize, pages: u32, per_page: f32, cross: f32) {
    let lib = library(books, pages, per_page, cross, 31);
    let r = refs(&lib);
    let t = Instant::now();
    let mut g = GraphModel::default();
    let rc = graph_build_all(&mut g, &r, &[], None);
    let build = ms(t);
    if rc == GRAPH_ALL_TOO_BIG {
        eprintln!("  {label}: past the caps (build {build:.1} ms): opens on the editor's notebook");
        return;
    }
    check!(rc == 0);
    let mut seed = g.clone();
    let t = Instant::now();
    check!(graph_seed(&mut seed, &mut || {}));
    let seeded = ms(t);
    let t = Instant::now();
    check!(graph_layout(&mut g, &mut || {}));
    let full = ms(t);
    let mut ini = GraphAllIni::default();
    graph_all_ini_store(&mut ini, &g);
    // 1% of the linked pages forgotten: placed again beside their neighbours.
    let mut h = g.clone();
    let mut kept = ini.clone();
    let drop = (h.nodes.len() / 100).max(1);
    kept.pos.retain(|p| p.pid as usize % (h.nodes.len() / drop).max(2) != 0);
    graph_all_apply_ini(&mut h, &kept);
    let t = Instant::now();
    check!(graph_place_new(&mut h, &mut || {}) >= 0);
    let place = ms(t);
    let mut w = h.clone();
    let t = Instant::now();
    check!(graph_layout_warm(&mut w, &mut || {}));
    let warm = ms(t);
    let t = Instant::now();
    let mut back = GraphModel::default();
    check!(graph_build_all(&mut back, &r, &[], None) == 0);
    let plan = graph_all_apply_ini(&mut back, &ini);
    let reopen = ms(t);
    check!(plan == GraphPlan::Ready);
    let entries: Vec<LibEntry> = lib.iter().map(|b| LibEntry { stamps: [1, 1, 1], left_out: false, book: b.clone() }).collect();
    let text = libindex_text(&entries);
    let t = Instant::now();
    let parsed = libindex_parse(&text);
    let parse = ms(t);
    check!(parsed.as_ref().is_some_and(|p| p.len() == lib.len()));
    let links: usize = lib.iter().map(|b| b.links.len()).sum();
    eprintln!(
        "  {label}: {} linked pages / {} edges / {} components ({} links); build {build:.1} ms, seed {seeded:.1} ms, full layout {full:.1} ms, place 1% new {place:.1} ms, warm layout {warm:.1} ms, reopen (build + kept positions) {reopen:.1} ms; graph-index.ini {} KB, read back {parse:.1} ms",
        g.nodes.len(),
        g.edges.len(),
        g.ncomp,
        links,
        text.len() / 1024
    );
}

fn libraries() {
    case("20 x 15, dense", 20, 15, 1.4, 0.06);
    case("20 x 50, typical", 20, 50, 1.1, 0.05);
    case("20 x 100, typical", 20, 100, 1.1, 0.05);
    case("50 x 100, typical", 50, 100, 1.1, 0.05);
    case("20 x 100, every page linked", 20, 100, 2.5, 0.05);
    case("500 x 1", 500, 1, 1.0, 0.6);
    case("100 x 200, typical", 100, 200, 1.1, 0.05);
    case("100 x 200, sparse", 100, 200, 0.22, 0.05);
}

#[test]
fn test_graph_all_bench() {
    run!(libraries);
    done();
}
