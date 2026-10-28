// graph_corpus: synthetic notebooks for test_graph. A page order, Index names and links are built
// straight into a NotebookInfo and a Nav (no files: the graph never loads a page), from a seeded
// generator, so every run builds the same notebooks.
#![allow(dead_code)]

use grimoire::prelude::*;
use std::collections::HashSet;

pub struct Book {
    pub nb: NotebookInfo,
    pub nav: Nav,
}

impl Clone for Book {
    fn clone(&self) -> Book {
        Book { nb: self.nb.clone(), nav: self.nav.clone() }
    }
}

/// A notebook with these pages, in this order, and no names or links yet.
pub fn book(pids: &[u32]) -> Book {
    let mut nb = NotebookInfo::default();
    nb.id.set("graph-test");
    nb.page_ids = pids.to_vec();
    nb.next_page_id = pids.iter().copied().max().unwrap_or(0) + 1;
    let mut nav = Nav::default();
    nav_init(&mut nav);
    Book { nb, nav }
}

pub fn name(b: &mut Book, pid: u32, text: &str) {
    assert_eq!(nav_set_name(&mut b.nav, pid, text), 0, "name page {pid}");
}

/// A link on page `from` (the strokes with `seed`) to page `to`; nav_link_set's code.
pub fn link(b: &mut Book, from: u32, seed: u32, to: u32) -> i32 {
    nav_link_set(&mut b.nav, from, seed, &format!("p{to}"))
}

pub fn link_index(b: &mut Book, from: u32, seed: u32) -> i32 {
    nav_link_set(&mut b.nav, from, seed, "index")
}

/// A link with any target, as a newer version may write one (nav_link_set takes only the two this
/// version knows).
pub fn link_raw(b: &mut Book, from: u32, seed: u32, target: &str) {
    b.nav.links.push(NavLink { pid: from, seed, target: FixedStr::from_str(target) });
}

/// A small fixed LCG (Knuth's MMIX constants): the corpus must not change between runs.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

/// n distinct page ids, scattered (not 1..n) so a test cannot lean on ids being indices.
pub fn scattered_pids(n: usize) -> Vec<u32> {
    (0..n as u32).map(|k| (k + 1) * 13 + k % 7).collect()
}

/// The same pages in another order (a seeded shuffle), names and links unchanged.
pub fn shuffled(b: &Book, seed: u64) -> Book {
    let mut out = b.clone();
    let mut r = Lcg(seed);
    let ids = &mut out.nb.page_ids;
    for i in (1..ids.len()).rev() {
        let j = r.below(i as u32 + 1) as usize;
        ids.swap(i, j);
    }
    out
}

/// n pages (in a lightly shuffled order), `names` of them named, and up to `links` links between
/// random pages (repeats allowed: they become multiplicities), a few to the Index.
pub fn random_book(n: usize, links: usize, names: usize, seed: u64) -> Book {
    let pids = scattered_pids(n);
    let mut b = book(&pids);
    let mut r = Lcg(seed);
    // A few pages swap places, as inserting pages does.
    for _ in 0..n / 8 {
        let i = r.below(n as u32) as usize;
        let j = r.below(n as u32) as usize;
        b.nb.page_ids.swap(i, j);
    }
    for k in 0..names.min(n) {
        let pid = b.nb.page_ids[r.below(n as u32) as usize];
        name(&mut b, pid, &format!("Entry {k}"));
    }
    for seed_no in (1u32..).take(links) {
        let from = b.nb.page_ids[r.below(n as u32) as usize];
        if r.below(16) == 0 {
            link_index(&mut b, from, seed_no);
        } else {
            let to = b.nb.page_ids[r.below(n as u32) as usize];
            if to != from {
                link(&mut b, from, seed_no, to);
            }
        }
    }
    b
}

/// n pages that are all force nodes: `links` links between distinct pairs (no repeats, at most 64
/// on a page, as nav_link_set allows), and every page left without a link named.
pub fn all_force_book(n: usize, links: usize, seed: u64) -> Book {
    let pids = scattered_pids(n);
    let mut b = book(&pids);
    let mut r = Lcg(seed);
    let mut pairs: HashSet<(u32, u32)> = HashSet::new();
    let mut linked: HashSet<u32> = HashSet::new();
    let mut seed_no = 1u32;
    let mut tries = 0;
    while pairs.len() < links && tries < links * 20 {
        tries += 1;
        let from = pids[r.below(n as u32) as usize];
        let to = pids[r.below(n as u32) as usize];
        let key = (from.min(to), from.max(to));
        if from == to || pairs.contains(&key) {
            continue;
        }
        if link(&mut b, from, seed_no, to) != 0 {
            continue; // (a page full of links, or the notebook's limit)
        }
        seed_no += 1;
        pairs.insert(key);
        linked.insert(from);
        linked.insert(to);
    }
    for (k, &pid) in pids.iter().enumerate() {
        if !linked.contains(&pid) {
            name(&mut b, pid, &format!("Alone {k}"));
        }
    }
    b
}
