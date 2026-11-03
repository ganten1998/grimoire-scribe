// Synthetic libraries for the Graph of all notebooks (docs/beta2/GRAPH-ALL.md): GraphBooks built
// straight from a seeded generator, no files, so every run builds the same library.
#![allow(dead_code)]

use super::Lcg;
use grimoire::prelude::*;

pub fn lib_id(k: usize) -> FixedStr<STORE_ID_LEN> {
    FixedStr::from_str(&format!("2026{:04}-120000-{:04x}", k, k * 7919 % 65536))
}

/// `books` notebooks of `pages` pages (ids 1..=pages, as every notebook starts at 1), about
/// `per_page` links a page: most to pages of the same notebook, `cross` (0 to 1) of them to another
/// notebook's page, a few to the Index; a name on every tenth page.
pub fn library(books: usize, pages: u32, per_page: f32, cross: f32, seed: u64) -> Vec<GraphBook> {
    let mut r = Lcg(seed);
    let mut out = Vec::new();
    for k in 0..books {
        let mut b = GraphBook {
            id: lib_id(k),
            name: format!("Notebook {k}"),
            folder: if k % 3 == 0 { "School".into() } else { String::new() },
            pages: (1..=pages).collect(),
            ..GraphBook::default()
        };
        for p in (1..=pages).step_by(10) {
            b.names.push((p, format!("Topic {k}.{p}")));
        }
        let n = (pages as f32 * per_page) as u32;
        for _ in 0..n {
            let src = 1 + r.below(pages);
            let roll = r.below(1000) as f32 / 1000.0;
            let to = if roll < 0.04 {
                GraphTo::Index
            } else if roll < 0.04 + cross && books > 1 {
                let mut other = r.below(books as u32) as usize;
                if other == k {
                    other = (other + 1) % books;
                }
                GraphTo::Other(lib_id(other), 1 + r.below(pages))
            } else {
                GraphTo::Page(1 + r.below(pages))
            };
            b.links.push(GraphLink { src, to });
        }
        b.links.sort_unstable();
        b.links.dedup();
        out.push(b);
    }
    out
}

pub fn refs(books: &[GraphBook]) -> Vec<&GraphBook> {
    books.iter().collect()
}
