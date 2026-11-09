//! doclinks.ini and pdflinks.rs (lane L4, DESIGN.md 3.10): the file, its caps, the water level, fills, seeds, taps,
//! carrying through Copy page and Copy to notebook, the notebook PDF, links.txt from KOReader. One test function:
//! the store root is process-wide.

use grimoire::prelude::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("doclinks-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn book(name: &str, pages: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_NARROW_NM, &mut nb), 0);
    for i in 1..pages {
        assert_eq!(store_add_page(&mut nb, i as i32, PAPER_NARROW_NM), 0);
    }
    for i in 0..pages {
        let p = page_new(800, 1000, PAPER_NARROW_NM);
        assert_eq!(store_save_page(&nb, i as i32, &p), 0);
    }
    nb
}

fn rec(src: u32, dst: u32, x: i32, y: i32, w: i32, h: i32, index: u32) -> DocRecord {
    DocRecord { src, dst, rect: Rect::new(x, y, w, h), quad: None, index }
}

fn none(_: u32, _: u32) -> bool {
    false
}

fn file(nb: &NotebookInfo) -> PathBuf {
    root().join("notebooks").join(nb.id.as_str()).join("doclinks.ini")
}

fn save(nb: &NotebookInfo, dl: &mut DocLinks) {
    assert_eq!(doclinks_save(nb.id.as_str(), dl, nb), 0);
}

#[test]
fn document_links_file_fills_taps_carrying_and_the_pdf() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, 800, 1000);
    file_round_trip_caps_and_unknown_lines();
    two_oh_reader_ignores_it();
    water_level_by_count_and_by_bytes();
    fills_validation_seeds_and_tolerance();
    taps_exact_then_grown_later_wins();
    a_year_page_taps_boxes_folds_and_prints();
    copy_page_and_copy_to_notebook_carry_what_travels();
    book_links_text_and_mapping();
}

fn file_round_trip_caps_and_unknown_lines() {
    let nb = book("Round trip", 4);
    let ids = nb.page_ids.clone();
    let mut dl = DocLinks::default();
    let st = pdflinks_fill(
        &mut dl,
        &[rec(ids[0], ids[1], 10, 20, 100, 50, 0), rec(ids[0], ids[2], 200, 20, 100, 50, 1), rec(ids[1], ids[0], 5, 5, 40, 40, 0)],
        800,
        1000,
        &|p| ids.contains(&p),
        &none,
    );
    assert_eq!(st, FillStats { added: 3, ..Default::default() });
    // a quad outline rides along
    let mut q = rec(ids[2], ids[3], 100, 100, 200, 200, 0);
    q.quad = Some([(100.0, 100.0), (300.0, 120.0), (280.0, 300.0), (110.0, 280.0)]);
    pdflinks_fill(&mut dl, &[q], 800, 1000, &|p| ids.contains(&p), &none);
    assert!(dl.outline_of(&doclinks_of(&dl, ids[2])[0]).is_some());
    save(&nb, &mut dl);
    let back = doclinks_load(nb.id.as_str());
    assert!(back.writable && back.file);
    assert_eq!(back.links, dl.links, "a round trip is the same links in the same order");
    assert_eq!(doclinks_of(&back, ids[0]).len(), 2);
    assert_eq!(doclinks_of(&back, ids[0])[0].target, ids[1], "annotation order is kept");
    assert_eq!(back.outline_of(&doclinks_of(&back, ids[2])[0]), dl.outline_of(&doclinks_of(&dl, ids[2])[0]));
    // 24 bytes a link
    assert_eq!(std::mem::size_of::<DocLink>(), 24);
    // lines of a form this version does not know are carried, whatever their place
    let text = std::fs::read_to_string(file(&nb)).unwrap();
    let extra = format!("{text}future=thing\nlink.{}=000000aa,index\narea.{}=000000aa,1,1,5,5\n", ids[0], ids[0]);
    std::fs::write(file(&nb), &extra).unwrap();
    let mut with = doclinks_load(nb.id.as_str());
    assert!(with.writable);
    assert_eq!(with.links.len(), 4);
    save(&nb, &mut with);
    let again = std::fs::read_to_string(file(&nb)).unwrap();
    assert!(again.contains("future=thing\n") && again.contains(&format!("link.{}=000000aa,index\n", ids[0])));
    assert!(again.contains(&format!("area.{}=000000aa,1,1,5,5\n", ids[0])));
    // the lines of a page no longer in the notebook go at the next write, and the links to it too
    let mut fewer = nb.clone();
    fewer.page_ids.retain(|&p| p != ids[1]);
    let mut dl2 = doclinks_load(nb.id.as_str());
    assert_eq!(doclinks_save(nb.id.as_str(), &mut dl2, &fewer), 0);
    let text = std::fs::read_to_string(file(&nb)).unwrap();
    assert!(!text.contains(&format!("link.{}=", ids[1])), "a deleted page's own links go");
    assert!(!text.contains(&format!(",p{}\n", ids[1])), "and the links that went to it");
    assert!(text.contains(&format!("link.{}=", ids[2])));
    assert_eq!(dl2.links.len(), 2, "page 0's link to page 2 and page 2's to page 3 remain");
}

fn two_oh_reader_ignores_it() {
    let nb = book("Two oh", 3);
    let ids = nb.page_ids.clone();
    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_link_set(&mut nav, ids[0], 7, &format!("p{}", ids[1])), 0);
    assert_eq!(nav_link_set_area(&mut nav, ids[0], 7, Some(Rect::new(30, 40, 200, 100))), 0);
    assert_eq!(nav_save_links(nb.id.as_str(), &mut nav, &nb), 0);
    let mut dl = DocLinks::default();
    pdflinks_fill(&mut dl, &[rec(ids[0], ids[2], 400, 400, 100, 100, 0)], 800, 1000, &|p| ids.contains(&p), &none);
    save(&nb, &mut dl);
    // what a 2.0 build reads: links.ini alone. The user's link is there, editable, and the other is not in it.
    let mut old = Nav::default();
    assert_eq!(nav_load(nb.id.as_str(), &mut old), 0);
    assert_eq!(old.links.len(), 1);
    assert!(old.links_writable);
    assert_eq!(old.links[0].seed, 7);
    let ini = root().join("notebooks").join(nb.id.as_str()).join("links.ini");
    assert!(!std::fs::read_to_string(&ini).unwrap().contains(&format!("p{}\n", ids[2])), "links.ini never holds a document link");
    // ... and a save of the user's links never touches doclinks.ini
    let before = std::fs::read(file(&nb)).unwrap();
    assert_eq!(nav_link_set(&mut old, ids[1], 9, &format!("p{}", ids[0])), 0);
    assert_eq!(nav_link_set_area(&mut old, ids[1], 9, Some(Rect::new(1, 1, 50, 50))), 0);
    assert_eq!(nav_save_links(nb.id.as_str(), &mut old, &nb), 0);
    assert_eq!(std::fs::read(file(&nb)).unwrap(), before);
    // the user's links never count the document ones (the 4,000 and 64 caps are links.ini's alone)
    assert_eq!(nav_links_of(&old, ids[0], &mut 0), 1);
}

fn water_level_by_count_and_by_bytes() {
    // by count: 400 pages of 50 links is 20,000 links; the most under 90% of 16,000 is 14,400, 36 a page
    let mut dl = DocLinks::default();
    for pid in 1..=400u32 {
        for k in 0..50u32 {
            let l = DocLink::new(pid, 1000 + k, pid % 399 + 1, Rect::new(k as i32 * 10, 10, 8, 8)).unwrap();
            assert!(dl.push(l, None));
        }
    }
    let gone = pdflinks_water(&mut dl);
    assert_eq!(gone, 20_000 - 14_400);
    assert_eq!(dl.links.len(), 14_400);
    assert!(doclinks_bytes(&dl) <= DOCLINKS_FILE_MAX * 9 / 10);
    for pid in [1u32, 200, 400] {
        let mine = doclinks_of(&dl, pid);
        assert!(mine.len() == 36, "every page is at the level: {}", mine.len());
        assert_eq!(mine[0].seed, 1000, "each page keeps its first links, in annotation order");
    }
    // the densest pages first: a page of 1,000 links is cut before the pages of 10
    let mut dl = DocLinks::default();
    for pid in 1..=20u32 {
        let n = if pid == 1 { 1000 } else { 10 };
        for k in 0..n {
            assert!(dl.push(DocLink::new(pid, k, pid % 19 + 2, Rect::new(k as i32 % 500, 10, 8, 8)).unwrap(), None));
        }
    }
    for k in 0..15_000u32 {
        dl.push(DocLink::new(30 + k / 500, 100_000 + k, 2, Rect::new((k % 500) as i32, 20, 8, 8)).unwrap(), None);
    }
    let before_dense = doclinks_of(&dl, 1).len();
    pdflinks_water(&mut dl);
    assert!(dl.links.len() <= 14_400);
    assert!(doclinks_of(&dl, 1).len() < before_dense, "the densest page loses links");
    assert_eq!(doclinks_of(&dl, 5).len(), 10, "a thin page keeps all of its links");
    // by bytes: outlines make each link about 5x the size; 12,000 links are under 90% of the count cap
    // and well over 90% of the byte cap
    let mut dl = DocLinks::default();
    let outline = LinkOutline::from_points(&(0..=60).map(|i| {
        let a = i as f32 * std::f32::consts::TAU / 60.0;
        (60.0 + 50.0 * a.cos(), 60.0 + 50.0 * a.sin())
    }).collect::<Vec<_>>(), true, 800, 1000);
    assert!(outline.n >= 16);
    for k in 0..12_000u32 {
        let l = DocLink::new(1 + k / 60, k, 2 + k / 60, Rect::new(0, 0, 120, 120)).unwrap();
        assert!(dl.push(l, Some(outline)));
    }
    assert!(dl.links.len() <= 14_400 && doclinks_bytes(&dl) > DOCLINKS_FILE_MAX * 9 / 10);
    let gone = pdflinks_water(&mut dl);
    assert!(gone > 0);
    assert!(doclinks_bytes(&dl) <= DOCLINKS_FILE_MAX * 9 / 10, "bytes decide here: {}", doclinks_bytes(&dl));
    assert!(dl.links.len() < 12_000 && dl.links.len() > 6_000);
    // a file at the caps is refused whole
    let mut text = String::new();
    for k in 0..16_001u32 {
        text.push_str(&format!("link.1={:08x},p2\narea.1={:08x},0,0,5,5\n", k, k));
    }
    assert!(doclinks_parse(text.as_bytes()).is_none(), "16,001 links is past the cap");
    let mut dl = DocLinks::default();
    for k in 0..1025u32 {
        dl.push(DocLink::new(1, k, 2, Rect::new(0, 0, 5, 5)).unwrap(), None);
    }
    let nb = book("Caps", 3);
    let mut dl2 = DocLinks::default();
    for k in 0..1025u32 {
        dl2.push(DocLink::new(nb.page_ids[0], k, nb.page_ids[1], Rect::new(0, 0, 5, 5)).unwrap(), None);
    }
    assert_eq!(doclinks_save(nb.id.as_str(), &mut dl2, &nb), -1, "1,024 a page");
    // a file over 1.5 MiB does not read, and is not written over
    std::fs::write(file(&nb), vec![b'#'; DOCLINKS_FILE_MAX + 10]).unwrap();
    let big = doclinks_load(nb.id.as_str());
    assert!(!big.writable && big.links.is_empty());
    assert_eq!(doclinks_save(nb.id.as_str(), &mut dl2, &nb), -1);
    std::fs::remove_file(file(&nb)).unwrap();
    drop(dl);
}

fn fills_validation_seeds_and_tolerance() {
    let nb = book("Fills", 3);
    let ids = nb.page_ids.clone();
    let exists = |p: u32| ids.contains(&p);
    let mut dl = DocLinks::default();
    let recs = [
        rec(ids[0], ids[1], 10, 10, 100, 100, 0),
        rec(ids[0], ids[0], 10, 10, 100, 100, 1),         // its own page
        rec(ids[0], 9999, 10, 10, 100, 100, 2),           // no such page
        rec(ids[0], ids[1], 900, 10, 100, 100, 3),         // wholly off the page (800 wide)
        rec(ids[0], ids[2], 750, 10, 100, 100, 4),         // half off: clamped to the page
        rec(ids[0], ids[1], 11, 12, 99, 98, 5),            // the same target, within 2 px: the same link
    ];
    let st = pdflinks_fill(&mut dl, &recs, 800, 1000, &exists, &none);
    assert_eq!((st.added, st.same, st.invalid), (2, 1, 3), "{st:?}");
    let second = doclinks_of(&dl, ids[0])[1];
    assert_eq!(second.area(), Rect::new(750, 10, 50, 100), "clamped inside the page");
    // the same fill again changes nothing; the seeds are the same hash of page and annotation index
    let seeds: Vec<u32> = doclinks_of(&dl, ids[0]).iter().map(|l| l.seed).collect();
    assert_eq!(seeds[0], pdflinks_seed(ids[0], 0));
    let st = pdflinks_fill(&mut dl, &recs, 800, 1000, &exists, &none);
    assert_eq!((st.added, st.same), (0, 3), "{st:?}");
    assert_eq!(doclinks_of(&dl, ids[0]).iter().map(|l| l.seed).collect::<Vec<_>>(), seeds);
    // 3 px apart is another link
    let st = pdflinks_fill(&mut dl, &[rec(ids[0], ids[1], 14, 10, 100, 100, 9)], 800, 1000, &exists, &none);
    assert_eq!(st.added, 1);
    // seeds are never shared with the page's own ink or its links, nor with each other
    let want = pdflinks_seed(ids[1], 0);
    let mut dl3 = DocLinks::default();
    pdflinks_fill(&mut dl3, &[rec(ids[1], ids[0], 10, 10, 50, 50, 0)], 800, 1000, &exists, &|p, s| p == ids[1] && s == want);
    assert_ne!(doclinks_of(&dl3, ids[1])[0].seed, want, "a seed the page's ink has is skipped");
    // and the editor's new ink and hand-made links avoid a document link's seed in turn
    let nav = {
        let mut n = Nav::default();
        nav_init(&mut n);
        n
    };
    let p = page_new(800, 1000, PAPER_NARROW_NM);
    let mine = doclinks_of(&dl, ids[0])[0].seed;
    assert_ne!(nav_new_seed_with(&nav, Some(&dl), &p, ids[0], mine), mine);
    assert_eq!(nav_new_seed(&nav, &p, ids[0], mine), mine, "without them it would take it");
    assert_eq!(nav_new_seed_with(&nav, Some(&dl), &p, ids[0], mine ^ 0x55555555), mine ^ 0x55555555);
    // a page takes 1,024
    let mut many: Vec<DocRecord> = Vec::new();
    for k in 0..1100 {
        many.push(rec(ids[2], ids[0], (k % 70) as i32 * 11, (k / 70) as i32 * 11, 10, 10, k as u32));
    }
    let mut dl4 = DocLinks::default();
    let st = pdflinks_fill(&mut dl4, &many, 800, 1000, &exists, &none);
    assert_eq!((st.added, st.full), (1024, 76));
    // the planner's quad: an outline within the area, or none when it is not
    let mut dl5 = DocLinks::default();
    let mut q = rec(ids[0], ids[1], 100, 100, 100, 100, 0);
    q.quad = Some([(100.0, 100.0), (200.0, 100.0), (200.0, 200.0), (100.0, 200.0)]);
    pdflinks_fill(&mut dl5, &[q], 800, 1000, &exists, &none);
    assert!(doclinks_of(&dl5, ids[0])[0].has_outline());
    let mut dl6 = DocLinks::default();
    q.quad = Some([(0.0, 0.0), (500.0, 0.0), (500.0, 500.0), (0.0, 500.0)]);
    pdflinks_fill(&mut dl6, &[q], 800, 1000, &exists, &none);
    assert!(!doclinks_of(&dl6, ids[0])[0].has_outline(), "an outline outside its area is dropped, the box kept");
}

fn taps_exact_then_grown_later_wins() {
    let p = page_new(800, 1000, PAPER_NARROW_NM);
    let min_px = 84.0;
    let mut dl = DocLinks::default();
    // a big box, a small one inside it, and a small box 10 px to the right of the small one (reached only through
    // the 7 mm growth from the left)
    dl.push(DocLink::new(1, 1, 10, Rect::new(100, 100, 400, 400)).unwrap(), None);
    dl.push(DocLink::new(1, 2, 20, Rect::new(200, 200, 30, 30)).unwrap(), None);
    dl.push(DocLink::new(1, 3, 30, Rect::new(240, 200, 30, 30)).unwrap(), None);
    let at = |x: f32, y: f32| doclinks_at(&dl, &p, 1, x, y, min_px).map(|l| l.target);
    assert_eq!(at(210.0, 210.0), Some(20), "the smallest exact area");
    assert_eq!(at(150.0, 150.0), Some(10));
    assert_eq!(at(250.0, 210.0), Some(30));
    assert_eq!(at(235.0, 215.0), Some(10), "between two small boxes, inside the big one's exact area: the exact area wins over their growth");
    // reached only by the growth: 20 px right of the big box's edge is in no exact area; the growth of the big
    // box is 400 wide already; a lone small box takes it
    let mut lone = DocLinks::default();
    lone.push(DocLink::new(1, 1, 10, Rect::new(400, 400, 10, 10)).unwrap(), None);
    assert_eq!(doclinks_at(&lone, &p, 1, 430.0, 405.0, min_px).map(|l| l.target), Some(10), "within a finger's growth");
    assert_eq!(doclinks_at(&lone, &p, 1, 480.0, 405.0, min_px), None, "beyond it");
    // a tap inside an exact area beats a neighbour that only reaches it through the growth, even a smaller one
    let mut two = DocLinks::default();
    two.push(DocLink::new(1, 1, 10, Rect::new(100, 100, 200, 200)).unwrap(), None); // exact
    two.push(DocLink::new(1, 2, 20, Rect::new(305, 190, 10, 10)).unwrap(), None); // tiny, grown to 84 px
    assert_eq!(doclinks_at(&two, &p, 1, 298.0, 195.0, min_px).map(|l| l.target), Some(10));
    // equal boxes: the annotation drawn later wins
    let mut tie = DocLinks::default();
    tie.push(DocLink::new(1, 1, 10, Rect::new(100, 100, 50, 50)).unwrap(), None);
    tie.push(DocLink::new(1, 2, 20, Rect::new(100, 100, 50, 50)).unwrap(), None);
    assert_eq!(doclinks_at(&tie, &p, 1, 120.0, 120.0, min_px).map(|l| l.target), Some(20));
    // and a different page's links are not found
    assert_eq!(doclinks_at(&tie, &p, 2, 120.0, 120.0, min_px), None);
    // the user's links come first: nav_link_at finds theirs, and the caller asks doclinks_at only when it finds none
    let mut nav = Nav::default();
    nav_init(&mut nav);
    nav_link_set(&mut nav, 1, 77, "p9");
    nav_link_set_area(&mut nav, 1, 77, Some(Rect::new(110, 110, 30, 30)));
    let mut hit = NavLink::default();
    assert!(nav_link_at(&nav, &p, 1, 120.0, 120.0, min_px, &mut hit) && hit.seed == 77);
}

fn a_year_page_taps_boxes_folds_and_prints() {
    // a month page of a planner: 365 day cells, each a link to its own day page, and 12 month tabs
    let nb = book("Year", 40);
    let ids = nb.page_ids.clone();
    let exists = |p: u32| ids.contains(&p);
    let mut recs = Vec::new();
    for d in 0..365u32 {
        let (col, row) = ((d % 19) as i32, (d / 19) as i32);
        recs.push(rec(ids[0], ids[1 + (d % 39) as usize], 20 + col * 40, 100 + row * 40, 36, 36, d));
    }
    let mut dl = DocLinks::default();
    let st = pdflinks_fill(&mut dl, &recs, 800, 1000, &exists, &none);
    assert_eq!(st.added, 365);
    pdflinks_water(&mut dl);
    save(&nb, &mut dl);
    let dl = doclinks_load(nb.id.as_str());
    let p = page_new(800, 1000, PAPER_NARROW_NM);
    assert_eq!(doclinks_boxes(&dl, &p, ids[0]).len(), 365, "the eye draws 365 boxes");
    let mut found = 0;
    for r in &recs {
        let (cx, cy) = ((r.rect.x + r.rect.w / 2) as f32, (r.rect.y + r.rect.h / 2) as f32);
        if doclinks_at(&dl, &p, ids[0], cx, cy, 84.0).is_some_and(|l| l.target == r.dst) {
            found += 1;
        }
    }
    assert_eq!(found, 365, "every cell taps to its own page");
    // the bar: one row, from the PDF (365), opening as chips grouped by target (39 targets)
    let mut lb = LinkBar::default();
    let mut nav = Nav::default();
    nav_init(&mut nav);
    linkbar_build(&mut lb, &nb, &nav, &p, 0, 300.0);
    linkbar_set_docs(&mut lb, &nb, &doclinks_targets(&dl, &p, ids[0]), 300.0, false);
    assert_eq!((lb.n, lb.doc_links, lb.docs.len()), (0, 365, 39));
    assert!(linkbar_has_docs_row(&lb));
    assert_eq!(linkbar_chips(&lb), 1, "the page's rows: none, then the one row");
    assert!(linkbar_is_docs_row(&lb, 0));
    assert_eq!(lb.docs.iter().map(|e| e.count).sum::<i32>(), 365);
    linkbar_set_docs(&mut lb, &nb, &doclinks_targets(&dl, &p, ids[0]), 300.0, true);
    assert_eq!(linkbar_chips(&lb), 39);
    assert!(linkbar_entry(&lb, 0).is_some() && linkbar_entry(&lb, 39).is_none());
    // the notebook PDF: all 365 come out as link annotations on page 1, to the right pages
    let mut buf = Buf::default();
    assert_eq!(export_notebook_pdf(&nb, 72.0, &mut buf), 0);
    let pdf = String::from_utf8_lossy(&buf.data).into_owned();
    assert_eq!(pdf.matches("/Subtype /Link").count(), 365);
    let mut zout = Buf::default();
    {
        let mut z = zip_begin(&mut zout);
        assert_eq!(export_notebook_zip(&nb, 72.0, &mut z, ""), 0);
    }
    assert!(zout.data.windows(16).any(|w| w == b"raw/doclinks.ini"), "raw zips carry doclinks.ini");
}

fn copy_page_and_copy_to_notebook_carry_what_travels() {
    let nb = book("Carry src", 5);
    let ids = nb.page_ids.clone();
    let exists = |p: u32| ids.contains(&p);
    let mut dl = DocLinks::default();
    pdflinks_fill(
        &mut dl,
        &[
            rec(ids[0], ids[1], 10, 10, 50, 50, 0),
            rec(ids[0], ids[3], 100, 10, 50, 50, 1),
            rec(ids[1], ids[0], 10, 10, 50, 50, 0),
            rec(ids[1], ids[4], 100, 10, 50, 50, 1),
        ],
        800,
        1000,
        &exists,
        &none,
    );
    save(&nb, &mut dl);
    // Copy page, inside the notebook: every link of the page, seeds and areas as they were
    let mut work = doclinks_load(nb.id.as_str());
    let new_pid = 999;
    let mut nb2 = nb.clone();
    nb2.page_ids.insert(1, new_pid);
    assert_eq!(doclinks_copy_page(&mut work, ids[0], new_pid), 2);
    assert_eq!(doclinks_save(nb.id.as_str(), &mut work, &nb2), 0);
    let copied = doclinks_of(&work, new_pid);
    assert_eq!(copied.len(), 2);
    assert_eq!((copied[0].seed, copied[0].area(), copied[0].target), (doclinks_of(&dl, ids[0])[0].seed, Rect::new(10, 10, 50, 50), ids[1]));
    // past the water level the copy gets none, and says nothing is done (the editor's toast says it)
    let mut full = DocLinks::default();
    for k in 0..14_400u32 {
        full.push(DocLink::new(2000 + k / 100, k, 3, Rect::new(0, 0, 5, 5)).unwrap(), None);
    }
    full.push(DocLink::new(5, 1, 3, Rect::new(0, 0, 5, 5)).unwrap(), None);
    assert_eq!(doclinks_copy_page(&mut full, 5, 6), -1);
    assert!(doclinks_of(&full, 6).is_empty());
    // Copy to notebook...: pages 0 and 1 copied; the link 0 to 1 travels, 0 to 3 and 1 to 4 do not, 1 to 0 does
    let dst = book("Carry dst", 2);
    let (a, b) = (dst.page_ids[0], dst.page_ids[1]);
    let copies = [PageCopy { from: ids[0], to: a, erased: false }, PageCopy { from: ids[1], to: b, erased: false }];
    assert_eq!(doclinks_copy_across(&dl, &copies, &dst, 1.0, 1.0), 2);
    let got = doclinks_load(dst.id.as_str());
    assert_eq!(got.links.len(), 2);
    assert_eq!((doclinks_of(&got, a)[0].target, doclinks_of(&got, b)[0].target), (b, a), "rewritten to the copies");
    // a page taken alone loses its tabs to pages left behind, and the file is not even made
    let alone = book("Carry alone", 1);
    let one = [PageCopy { from: ids[0], to: alone.page_ids[0], erased: false }];
    assert_eq!(doclinks_copy_across(&dl, &one, &alone, 1.0, 1.0), 0);
    assert!(!file(&alone).exists());
    // scaled when the pages differ in size
    let big = book("Carry big", 2);
    let c2 = [PageCopy { from: ids[0], to: big.page_ids[0], erased: false }, PageCopy { from: ids[1], to: big.page_ids[1], erased: false }];
    assert_eq!(doclinks_copy_across(&dl, &c2, &big, 2.0, 2.0), 2);
    assert_eq!(doclinks_of(&doclinks_load(big.id.as_str()), big.page_ids[0])[0].area(), Rect::new(20, 20, 100, 100));
    // a destination whose doclinks.ini does not read gets the pages without them, and the file is left alone
    let bad = book("Carry bad", 2);
    std::fs::write(file(&bad), b"link.1=zz\n\0binary").unwrap();
    let c3 = [PageCopy { from: ids[0], to: bad.page_ids[0], erased: false }, PageCopy { from: ids[1], to: bad.page_ids[1], erased: false }];
    assert_eq!(doclinks_copy_across(&dl, &c3, &bad, 1.0, 1.0), -1);
    assert_eq!(std::fs::read(file(&bad)).unwrap(), b"link.1=zz\n\0binary");
}

fn book_links_text_and_mapping() {
    let t = b"# a comment\nP 1860 2480\nL 1 3 100 200 300 50\nL 3 1 0 0 10 10\n";
    let b = pdflinks_parse_book(t).unwrap();
    assert_eq!((b.pic_w, b.pic_h, b.links.len()), (1860, 2480, 2));
    assert_eq!(b.links[0], (1, 3, Rect::new(100, 200, 300, 50)));
    for bad in [
        &b"L 1 3 1 1 1 1\n"[..],            // no size first
        b"P 10 10\nP 10 10\n",              // two sizes
        b"P 10 10\nL 1 3 1 1 0 5\n",       // empty
        b"P 10 10\nL 0 3 1 1 5 5\n",       // page 0
        b"P 10 10\nL 1 3 1 1 5\n",         // short
        b"P 10 10\nL 1 3 1 1 5 5 5\n",     // long
        b"P 10 10\nX 1\n",                  // unknown
        b"P 10 10\nL 1 3 a 1 5 5\n",       // not a number
        b"P 0 10\n",
    ] {
        assert!(pdflinks_parse_book(bad).is_none(), "{:?}", String::from_utf8_lossy(bad));
    }
    // the same size: as they are; another size: fitted and centred as the picture is
    assert_eq!(pdflinks_to_page(Rect::new(10, 20, 30, 40), 800, 1000, 800, 1000), Rect::new(10, 20, 30, 40));
    let half = pdflinks_to_page(Rect::new(100, 100, 100, 100), 400, 500, 800, 1000);
    assert_eq!(half, Rect::new(200, 200, 200, 200));
    let wide = pdflinks_to_page(Rect::new(0, 0, 100, 100), 1000, 500, 800, 1000);
    assert_eq!((wide.x, wide.y, wide.w, wide.h), (0, 300, 80, 80), "a sideways page is fitted by width and centred");
}
