// test_graph_keys: the Graph of all notebooks (docs/beta2/GRAPH-ALL.md, step 1) gave the layout a key per
// node (GraphNode.key) where it hashed and ordered by page id. A one-notebook model keys each node by its page
// id, so its layouts, gens and graph.ini bytes are what they were before keys came in: this test fingerprints
// them over the corpus and compares with the values recorded before the change. Floats differ between
// machines (fused multiply-adds, libm), so the recorded values are checked on the machine they were recorded
// on (an Apple arm64 host) and printed everywhere else.
#![allow(clippy::needless_range_loop)]
mod graph_corpus;
mod testlib;
use graph_corpus::*;
use grimoire::prelude::*;
use testlib::*;

struct Fp(u64);
impl Fp {
    fn eat(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x0100_0000_01B3);
        }
    }
    fn u32(&mut self, v: u32) {
        self.eat(&v.to_le_bytes());
    }
    fn f(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    fn model(&mut self, g: &GraphModel) {
        for n in &g.nodes {
            self.u32(n.pid);
            self.u32(n.placed as u32);
            self.f(n.x);
            self.f(n.y);
            self.f(n.r);
            self.u32(n.comp as u32);
        }
        for e in &g.edges {
            self.u32(e.a as u32);
            self.u32(e.b as u32);
        }
        for &v in &g.force {
            self.u32(v as u32);
        }
    }
}

fn scratch(name: &str) -> String {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join(format!("grimoire-test-graph-keys-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d.join(name).to_string_lossy().into_owned()
}

// One book through every one-notebook path the view takes: the build (both models), the seed, the full
// layout, graph.ini written and read back, a reopen with a few new pages and a new link, the warm layout and
// the Around rings.
fn fingerprint(b: &Book, tag: &str) -> u64 {
    let mut fp = Fp(0xCBF2_9CE4_8422_2325);
    for linked_only in [false, true] {
        let mut g = GraphModel::default();
        check!(graph_build(&mut g, &b.nb, &b.nav, b.nb.page_ids[b.nb.page_ids.len() / 2], linked_only) == 0);
        for n in &g.nodes {
            check!(n.key == n.pid);
        }
        let mut seed = g.clone();
        check!(graph_seed(&mut seed, &mut || {}));
        fp.model(&seed);
        check!(graph_layout(&mut g, &mut || {}));
        fp.model(&g);
        fp.u32(graph_gen(&g));
        let mut ini = GraphIni::default();
        graph_ini_store(&mut ini, &g);
        let path = scratch(&format!("{tag}-{linked_only}.ini"));
        check!(graph_ini_write_path(&path, &mut ini) == 0);
        fp.eat(&std::fs::read(&path).unwrap_or_default());
        // A few new pages linked in, and one new link between two kept pages.
        let mut more = b.clone();
        let next = more.nb.next_page_id;
        for k in 0..3 {
            more.nb.page_ids.push(next + k);
            link(&mut more, next + k, 900 + k, b.nb.page_ids[(k as usize * 7) % b.nb.page_ids.len()]);
        }
        link(&mut more, b.nb.page_ids[0], 990, b.nb.page_ids[b.nb.page_ids.len() - 1]);
        let mut h = GraphModel::default();
        check!(graph_build(&mut h, &more.nb, &more.nav, 0, linked_only) == 0);
        let mut back = GraphIni::default();
        check!(graph_ini_read_path(&path, &mut back) == 0);
        let plan = graph_apply_ini(&mut h, &back);
        fp.u32(plan as u32);
        check!(graph_place_new(&mut h, &mut || {}) >= 0);
        fp.u32(graph_relax_stressed(&mut h, &mut || {}) as u32);
        fp.model(&h);
        let mut warm = h.clone();
        check!(graph_layout_warm(&mut warm, &mut || {}));
        fp.model(&warm);
        let mut kept = GraphModel::default();
        check!(graph_build(&mut kept, &more.nb, &more.nav, 0, linked_only) == 0);
        graph_apply_ini(&mut kept, &back);
        check!(graph_seed_kept(&mut kept, &mut || {}));
        fp.model(&kept);
        let mut rings = Vec::new();
        let centre = g.force.first().copied().unwrap_or(-1);
        fp.u32(graph_around(&g, centre, 2, &mut rings) as u32);
        for r in &rings {
            fp.u32(r.node as u32);
            fp.f(r.x);
            fp.f(r.y);
        }
    }
    fp.0
}

fn corpus() -> Vec<(&'static str, Book)> {
    vec![
        ("random-120", random_book(120, 160, 30, 11)),
        ("random-400", random_book(400, 380, 90, 23)),
        ("force-150", all_force_book(150, 260, 5)),
        ("force-320", all_force_book(320, 300, 9)),
        ("shuffled-400", shuffled(&random_book(400, 380, 90, 23), 77)),
    ]
}

// Recorded at 26ef6c9 (before keys), on an Apple arm64 host, in the test profile.
const RECORDED: [(&str, u64); 5] = [
    ("random-120", 0x91a38adc782c83de),
    ("random-400", 0x66e1f8d587966c3b),
    ("force-150", 0x426328b8e40a1bf3),
    ("force-320", 0x6f039c26e1775736),
    ("shuffled-400", 0x7aee046b0f4d0296),
];

fn one_notebook_layouts_are_unchanged() {
    let recorded_here = cfg!(all(target_os = "macos", target_arch = "aarch64"));
    for (tag, b) in corpus() {
        let got = fingerprint(&b, tag);
        eprintln!("  {tag}: {got:#018x}");
        let want = RECORDED.iter().find(|r| r.0 == tag).map_or(0, |r| r.1);
        if recorded_here && want != 0 {
            check!(got == want);
        }
    }
}

#[test]
fn test_graph_keys() {
    run!(one_notebook_layouts_are_unchanged);
    done();
}
