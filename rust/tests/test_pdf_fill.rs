// test_pdf_fill.rs: the same PDF again fills its notebook (2.2, issue #123). A PDF whose notebook (by its file
// record, or by the book key where the notebook has none: one an earlier version made, or KOReader) lacks some of the
// book's pages gets those pages added, in book order, with the links between pages that both exist and names for
// pages that have none; the pages it had, their ink, names and links stay as they were. A notebook that has every
// page says "PDF already imported."; a PDF whose page count is not the one the notebook's pages name becomes a
// notebook of its own. The fill commits only while the notebook is closed, and a fill that stops, is cut off, or
// loses its PDF leaves the notebook as it was or filled, never half.
//
// The writer runs in this process (pdfimport::run_slice, every page through a real worker, a fresh exec of the
// headless runner), as in test_pdfimport.rs; the tests of the queue drive the jobs as the app does (bg_poll_job,
// then pdfimport::finished), as test_import_queue.rs does, and wait on no clock. Generated PDFs only (pdffix::tabs:
// four pages of 465 x 620 points, a tab on each, a link by a name on page 1, an outline naming pages 1 and 3).
#![cfg(feature = "sim")]
use grimoire::prelude::*;

const PW: i32 = 600;
const PH: i32 = 800;

// ---- the water level keeps a notebook's own links ----

// A fill over the water level trims only links it adds: the links the notebook had all stay, though their page is the
// densest.
#[test]
fn the_water_level_of_a_fill_trims_only_the_links_it_adds() {
    // 14,200 links the notebook has: 1,000 on page 1 (the densest), 33 on each of pages 2 to 401
    let mut had_links = DocLinks::default();
    for p in 1..=401u32 {
        for k in 0..(if p == 1 { 1000u32 } else { 33 }) {
            let area = Rect { x: (k % 40) as i32 * 12, y: (k / 40) as i32 * 12, w: 10, h: 10 };
            assert!(had_links.push(DocLink::new(p, 1000 + k, if p == 1 { 2 } else { 1 }, area).unwrap(), None));
        }
    }
    let had: std::collections::HashSet<(u32, u32)> = had_links.links.iter().map(|l| (l.pid, l.seed)).collect();
    // a fill adds one link on each of pages 2 to 401: 14,600, past the 14,400 of the water level
    let recs: Vec<grimoire::pdflinks::DocRecord> =
        (2..=401u32).map(|p| grimoire::pdflinks::DocRecord { src: p, dst: if p == 3 { 4 } else { 3 }, rect: Rect { x: 500, y: 500, w: 20, h: 20 }, quad: None, index: 100 }).collect();
    let mut dl = had_links.clone();
    let st = pdflinks_fill(&mut dl, &recs, PW, PH, &|p| (1..=401).contains(&p), &|_, _| false);
    assert_eq!(st.added, 400);
    let mut plain = dl.clone();
    let gone = pdflinks_water_keeping(&mut dl, &|l| had.contains(&(l.pid, l.seed)));
    assert_eq!((gone, dl.links.len()), (200, 14_400), "only links the fill added go");
    assert!(had.iter().all(|h| dl.links.iter().any(|l| (l.pid, l.seed) == *h)), "every link the notebook had stays");
    // the plain water level (an import's, a new notebook's) would trim the densest page: the notebook's own links
    assert!(pdflinks_water(&mut plain) > 0);
    assert!(!had.iter().all(|h| plain.links.iter().any(|l| (l.pid, l.seed) == *h)), "the plain level takes links page 1 had");
}
