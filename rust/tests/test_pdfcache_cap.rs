//! The notebook PDF's cache at its cap (2.1.0-beta.2 fix round, M14): past it whole notebooks go, the one
//! used longest ago first, and a notebook found in the cache is a use (a close that finds every page there
//! keeps the notebook); a notebook that alone is over the cap keeps its first pages' records and writes no
//! more, so an unchanged notebook draws the same pages at each close and writes nothing; once a day any
//! background job prunes, the mirror on or off. One test function: the store root is process-wide.

use grimoire::prelude::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("pdfcache-cap-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn wave(seed: u32, y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut p);
    let mut s = stroke_new(TOOL_PENCIL, &p, seed);
    for i in 0..30 {
        let q = PenSample { x: 100.0 + 20.0 * i as f32, y: y + 6.0 * (i % 5) as f32, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 7 };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

fn book(name: &str, pages: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_NARROW_NM, &mut nb), 0);
    for i in 1..pages {
        assert_eq!(store_add_page(&mut nb, i as i32, PAPER_NARROW_NM), 0);
    }
    for i in 0..pages {
        let mut p = page_new(800, 1000, PAPER_NARROW_NM);
        page_add(&mut p, wave(i as u32 + 1, 200.0 + 40.0 * i as f32));
        assert_eq!(store_save_page(&nb, i as i32, &p), 0);
    }
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

fn full(nb: &NotebookInfo) -> Vec<u8> {
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf_to(nb, 300.0, &mut BufSink(&mut b), PdfCacheMode::Off, None), 0);
    b.data
}

/// A mirror (which prunes to the cap): the PDF and the pages it drew.
fn mirror(nb: &NotebookInfo) -> (Vec<u8>, u32) {
    let dir = format!("{}/PDFs", root().display());
    let before = pdf_pages_rendered();
    assert_eq!(export_mirror_pdf(nb, 300.0, &dir), 0);
    let drawn = pdf_pages_rendered() - before;
    let mut path = String::new();
    export_mirror_path(nb, &dir, &mut path, 1000);
    (std::fs::read(&path).unwrap(), drawn)
}

fn cache_dir(nb: &NotebookInfo) -> PathBuf {
    PathBuf::from(pdf_cache_root()).join(nb.id.as_str())
}

/// The notebook's records: (page id, its file's modification time).
fn records(nb: &NotebookInfo) -> Vec<(String, std::time::SystemTime)> {
    let mut v: Vec<_> = std::fs::read_dir(cache_dir(nb))
        .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".z")).map(|e| (e.file_name().to_string_lossy().into_owned(), e.metadata().unwrap().modified().unwrap())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn size(nb: &NotebookInfo) -> u64 {
    std::fs::read_dir(cache_dir(nb)).map(|d| d.flatten().map(|e| e.metadata().unwrap().len()).sum()).unwrap_or(0)
}

#[test]
fn the_cache_at_its_cap_never_thrashes() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, 800, 1000);
    a_notebook_found_in_the_cache_is_used();
    a_notebook_over_the_cap_writes_nothing_at_each_close();
    the_daily_sweep_prunes_with_the_mirror_off();
    pdf_cache_set_cap(PDF_CACHE_MAX);
    let _ = std::fs::remove_dir_all(&r);
}

/// The plan: oldest notebook first, by last use. B mirrored first, then A, then B closed again with every
/// page found: B is the one in use, A is the one to go. (0c26d912 took the oldest records of any
/// notebook, and a hit never made a record younger: B's went, and B drew every page at its next close.)
fn a_notebook_found_in_the_cache_is_used() {
    let b = book("Used B", 4);
    let a = book("Used A", 4);
    mirror(&b);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    mirror(&a);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let one = size(&b);
    assert_eq!(mirror(&b).1, 0, "B's every page is found");
    // the cap holds one notebook's records, not two
    pdf_cache_set_cap(one + one / 2);
    let (pdf, drawn) = mirror(&b);
    assert_eq!((pdf == full(&b), drawn), (true, 0));
    assert_eq!((records(&b).len(), records(&a).len()), (4, 0), "A, used longest ago, went whole; B, in use, stays whole");
    let (_, drawn) = mirror(&b);
    assert_eq!(drawn, 0, "B's next close draws nothing");
    pdf_cache_set_cap(PDF_CACHE_MAX);
    assert_eq!(store_delete(a.id.as_str()), 0);
    assert_eq!(store_delete(b.id.as_str()), 0);
}

/// A notebook whose records alone are past the cap: its first pages' records are kept, its last pages are
/// drawn at each close and not written (0c26d912 drew six of ten pages at every close of an unchanged
/// notebook, wrote them, and the prune took away the ones found the time before).
fn a_notebook_over_the_cap_writes_nothing_at_each_close() {
    let nb = book("Thrash", 10);
    let (_, drawn) = mirror(&nb);
    assert_eq!(drawn, 10);
    let all = size(&nb);
    pdf_cache_set_cap(all * 45 / 100);
    pdf_cache_prune(&nb);
    let kept = records(&nb);
    assert!(!kept.is_empty() && kept.len() < 10, "the prune kept part of the notebook: {}", kept.len());
    assert!(size(&nb) <= all * 45 / 100);
    // the records kept are the first pages'
    let first: Vec<String> = nb.page_ids[..kept.len()].iter().map(|p| format!("p{p}.z")).collect();
    let mut names: Vec<String> = kept.iter().map(|k| k.0.clone()).collect();
    names.sort();
    let mut first_sorted = first.clone();
    first_sorted.sort();
    assert_eq!(names, first_sorted, "the first pages' records stay");
    let want = full(&nb);
    let mut drawn = Vec::new();
    for _ in 0..4 {
        // nothing changes in the notebook between these closes
        let (pdf, d) = mirror(&nb);
        assert!(pdf == want, "the PDF is the full render at every close");
        drawn.push(d);
    }
    let expect = (10 - kept.len()) as u32;
    assert_eq!(drawn, vec![expect; 4], "each close draws the pages past the cap, the same ones, and no more");
    assert_eq!(records(&nb), kept, "no record was written, rewritten or taken away");
    pdf_cache_set_cap(PDF_CACHE_MAX);
    assert_eq!(store_delete(nb.id.as_str()), 0);
}

/// With the mirror off nothing else prunes: any background job's child runs the prune once a day (its
/// stamp is `.cache/pdf/.swept`), and the records of a deleted notebook go.
fn the_daily_sweep_prunes_with_the_mirror_off() {
    let keep = book("Swept keep", 2);
    let gone = book("Swept gone", 2);
    mirror(&keep);
    mirror(&gone);
    assert_eq!(store_delete(gone.id.as_str()), 0);
    let stamp = PathBuf::from(pdf_cache_root()).join(".swept");
    let _ = std::fs::remove_file(&stamp);
    pdf_cache_sweep_due();
    assert!(!cache_dir(&gone).exists(), "the deleted notebook's records are pruned");
    assert_eq!(records(&keep).len(), 2, "the others stay");
    assert!(stamp.exists());
    // within the day it does nothing (one stat a job)
    let again = book("Swept again", 2);
    mirror(&again);
    assert_eq!(store_delete(again.id.as_str()), 0);
    pdf_cache_sweep_due();
    assert!(cache_dir(&again).exists(), "not again within a day");
    let f = std::fs::OpenOptions::new().write(true).open(&stamp).unwrap();
    f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 86400)).unwrap();
    drop(f);
    pdf_cache_sweep_due();
    assert!(!cache_dir(&again).exists(), "a day later it prunes again");
}
