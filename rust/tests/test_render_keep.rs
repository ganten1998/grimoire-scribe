// Dense pages keep their drawing (store_save_page_render): read back only while it is current.
#[path = "testlib/mod.rs"]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn page_with_ink(w: i32, h: i32) -> Box<Page> {
    let mut p = page_new(w, h, PAPER_NARROW);
    for t in [TOOL_MARKER, TOOL_CHARCOAL, TOOL_BALLPOINT] {
        let mut prm = ToolParams::default();
        tool_params_default(t, &mut prm);
        let mut s = stroke_new(t, &prm, 77 + t.0 as u32);
        for k in 0..60 {
            let f = k as f32 / 59.0;
            stroke_append(&mut s, &PenSample { x: 40.0 + f * (w as f32 - 80.0), y: 60.0 + 40.0 * t.0 as f32 + 30.0 * (f * 7.0).sin(), pressure: 0.6, t_ms: k * 5, ..Default::default() }, 300.0);
        }
        page_add(&mut p, s);
    }
    p
}

fn kept_drawings_are_exact_and_only_current() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Dense", "", PAPER_NARROW, &mut nb) == 0);
    let (w, h) = (600, 800);
    let p = page_with_ink(w, h);
    check!(store_save_page(&nb, 0, &p) == 0);
    let pid = nb.page_ids[0];
    let mut drawn = Canvas::default();
    check!(canvas_init(&mut drawn, w, h) == 0);
    page_render(&mut drawn, &p, Rect { x: 0, y: 0, w, h }, 300.0);
    check!(!store_page_render_kept(&nb, pid, w, h));
    check!(store_save_page_render(&nb, pid, &drawn));
    check!(store_page_render_kept(&nb, pid, w, h));
    // Read back byte for byte, into a canvas of the page's size only.
    let mut back = Canvas::default();
    check!(canvas_init(&mut back, w, h) == 0);
    check!(store_load_page_render(&nb, pid, w, h, &mut back) && back.px == drawn.px);
    let mut other = Canvas::default();
    check!(canvas_init(&mut other, w, h + 1) == 0);
    check!(!store_load_page_render(&nb, pid, w, h + 1, &mut other));
    // The page file written again (new ink): the kept drawing no longer counts.
    let mut p2 = page_with_ink(w, h);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut prm);
    let mut s = stroke_new(TOOL_PENCIL, &prm, 5);
    stroke_append(&mut s, &PenSample { x: 10.0, y: 10.0, pressure: 0.5, ..Default::default() }, 300.0);
    page_add(&mut p2, s);
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 0, &p2) == 0);
    check!(!store_page_render_kept(&nb, pid, w, h));
    let mut stale = Canvas::default();
    check!(canvas_init(&mut stale, w, h) == 0);
    check!(!store_load_page_render(&nb, pid, w, h, &mut stale));
    // The fingerprint is stable within a process.
    check!(page_render_fingerprint() == page_render_fingerprint() && page_render_fingerprint() != 0);
}

#[test]
fn test_render_keep() {
    let root = format!("{}/render-keep-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    run!(kept_drawings_are_exact_and_only_current);
    let _ = std::fs::remove_dir_all(&root);
}
