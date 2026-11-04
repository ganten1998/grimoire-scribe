// The Pages view's previews job (store_build_page_previews, 2.0.0-beta.2): the light pages before a dense one,
// a current kept drawing read back instead of drawn, a stale one never used, a slow drawing kept by the job,
// a preview cut short made again, and what counts as a dense page (store_page_dense).
#[path = "testlib/mod.rs"]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const W: i32 = 600;
const H: i32 = 800;

fn fresh_root(tag: &str) -> String {
    let root = format!("{}/page-previews-{tag}-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    check!(store_init(&root) == 0);
    store_set_device(300.0, W, H);
    root
}

fn dir_of(nb: &NotebookInfo) -> String {
    let mut d = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut d, 700) == 0);
    d
}

fn notebook(name: &str, n: i32) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    check!(store_create(name, "", PAPER_NARROW, &mut nb) == 0);
    while nb.npages() < n {
        let at = nb.npages();
        check!(store_add_page(&mut nb, at, PAPER_NARROW) == 0);
    }
    nb
}

fn seeded(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 8) as f32 / (1 << 24) as f32
}

// A page of `strokes` ballpoint strokes of `pts` points each, wandering from seeded places.
fn page_of(strokes: usize, pts: usize, mut seed: u32) -> Box<Page> {
    let mut p = page_new(W, H, PAPER_NARROW);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut prm);
    for k in 0..strokes {
        let mut s = stroke_new(TOOL_BALLPOINT, &prm, 1000 + k as u32);
        let (mut x, mut y) = (seeded(&mut seed) * W as f32, seeded(&mut seed) * H as f32);
        for i in 0..pts {
            x = (x + seeded(&mut seed) * 8.0 - 4.0).clamp(0.0, W as f32);
            y = (y + seeded(&mut seed) * 8.0 - 4.0).clamp(0.0, H as f32);
            stroke_append(&mut s, &PenSample { x, y, pressure: 0.6, t_ms: i as u32 * 6, ..Default::default() }, 300.0);
        }
        page_add(&mut p, s);
    }
    p
}

fn preview_of(nb: &NotebookInfo, pid: u32) -> Option<Canvas> {
    let mut c = Canvas::default();
    store_load_page_preview(nb, pid, &mut c).then_some(c)
}

fn mean(c: &Canvas) -> f64 {
    c.px[..(c.w * c.h) as usize].iter().map(|&v| v as f64).sum::<f64>() / (c.w * c.h) as f64
}

fn modified(path: &str) -> std::time::SystemTime {
    std::fs::metadata(path).and_then(|m| m.modified()).unwrap()
}

// A dense page among light ones (first in the notebook's order) is made last: the light pages' previews are
// written before its own.
fn light_pages_first() {
    let _root = fresh_root("order");
    let nb = notebook("Order", 4);
    check!(store_save_page(&nb, 0, &page_of(1100, 60, 7)) == 0);
    for i in 1..4 {
        check!(store_save_page(&nb, i, &page_of(2, 40, 100 + i as u32)) == 0);
    }
    let dir = dir_of(&nb);
    let size = |i: usize| std::fs::metadata(format!("{dir}/p{}.qpg", nb.page_ids[i])).unwrap().len();
    check!(size(0) >= STORE_DENSE_PAGE_BYTES && size(1) < STORE_DENSE_PAGE_BYTES);
    check!(store_page_dense(&nb, nb.page_ids[0]) && !store_page_dense(&nb, nb.page_ids[1]));
    check!(store_build_page_previews(nb.id.as_str(), "0:4", 300.0) == 0);
    let pgm = |i: usize| format!("{dir}/.page-previews/p{}.pgm", nb.page_ids[i]);
    for i in 0..4 {
        check!(preview_of(&nb, nb.page_ids[i]).is_some());
    }
    let dense = modified(&pgm(0));
    for i in 1..4 {
        check!(modified(&pgm(i)) < dense);
    }
}

// A page with a current kept drawing gets its preview from that drawing, read back, not drawn: here a drawing
// all black, which no drawing of the page (a stroke on ruled paper) would give.
fn kept_drawing_read_back() {
    let _root = fresh_root("kept");
    let nb = notebook("Kept", 1);
    let pid = nb.page_ids[0];
    check!(store_save_page(&nb, 0, &page_of(2, 40, 3)) == 0);
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, pid, &black, false));
    check!(store_build_page_previews(nb.id.as_str(), "0:1", 300.0) == 0);
    let p = preview_of(&nb, pid);
    check!(p.as_ref().is_some_and(|p| mean(p) < 1.0));
}

// A kept drawing of the page file as it was before a later save is never used: the page is drawn again.
fn stale_kept_drawing_ignored() {
    let _root = fresh_root("stale");
    let nb = notebook("Stale", 1);
    let pid = nb.page_ids[0];
    check!(store_save_page(&nb, 0, &page_of(2, 40, 3)) == 0);
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, pid, &black, false));
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 0, &page_of(3, 40, 4)) == 0);
    check!(!store_page_render_kept(&nb, pid, W, H, false));
    check!(store_build_page_previews(nb.id.as_str(), "0:1", 300.0) == 0);
    let p = preview_of(&nb, pid);
    check!(p.as_ref().is_some_and(|p| mean(p) > 200.0));
}

// A page the job drew in store_keep_render_ms() or more (0 here, so every one) keeps its drawing, exactly what
// drawing it gives, so it opens from it and its next preview is read back. Under the threshold none is kept.
fn slow_drawing_kept_by_the_job() {
    let _root = fresh_root("keep");
    let nb = notebook("Keep", 2);
    for i in 0..2 {
        check!(store_save_page(&nb, i, &page_of(30, 50, 11 + i as u32)) == 0);
    }
    STORE_TEST_KEEP_RENDER_MS.store(u32::MAX - 1, std::sync::atomic::Ordering::Relaxed);
    check!(store_build_page_previews(nb.id.as_str(), "1:2", 300.0) == 0);
    check!(preview_of(&nb, nb.page_ids[1]).is_some() && !store_page_render_kept(&nb, nb.page_ids[1], W, H, false));
    STORE_TEST_KEEP_RENDER_MS.store(0, std::sync::atomic::Ordering::Relaxed);
    check!(store_build_page_previews(nb.id.as_str(), "0:1", 300.0) == 0);
    STORE_TEST_KEEP_RENDER_MS.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
    let pid = nb.page_ids[0];
    check!(store_page_render_kept(&nb, pid, W, H, false));
    let mut kept = Canvas::default();
    check!(canvas_init(&mut kept, W, H) == 0);
    check!(store_load_page_render(&nb, pid, W, H, &mut kept, false));
    let mut page = None;
    check!(store_read_page(&nb, 0, &mut page) == 0);
    let mut drawn = Canvas::default();
    check!(canvas_init(&mut drawn, W, H) == 0);
    page_render(&mut drawn, page.as_deref().unwrap(), Rect::new(0, 0, W, H), 300.0);
    check!(kept.px == drawn.px);
    // (and a page that kept a drawing once counts as dense from then on, current drawing or not)
    check!(store_page_dense(&nb, pid) && !store_page_dense(&nb, nb.page_ids[1]));
}

// A preview whose image was cut short (a cache write is never synced) under a current stamp is made again.
fn cut_short_preview_made_again() {
    let _root = fresh_root("cut");
    let nb = notebook("Cut", 2);
    for i in 0..2 {
        check!(store_save_page(&nb, i, &page_of(2, 40, 21 + i as u32)) == 0);
    }
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    let pid = nb.page_ids[1];
    check!(store_page_preview_ok(&nb, pid));
    let pgm = format!("{}/.page-previews/p{pid}.pgm", dir_of(&nb));
    let bytes = std::fs::read(&pgm).unwrap();
    std::fs::write(&pgm, &bytes[..bytes.len() / 2]).unwrap();
    check!(store_page_preview_fresh(&nb, pid).is_some() && !store_page_preview_ok(&nb, pid));
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    check!(store_page_preview_ok(&nb, pid) && preview_of(&nb, pid).is_some());
}

#[test]
fn test_page_previews() {
    run!(light_pages_first);
    run!(kept_drawing_read_back);
    run!(stale_kept_drawing_ignored);
    run!(slow_drawing_kept_by_the_job);
    run!(cut_short_preview_made_again);
    done();
}
