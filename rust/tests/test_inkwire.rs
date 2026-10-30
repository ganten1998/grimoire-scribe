// test_inkwire.c: white ink and the precise eraser in the editor (src/inkmode.h: what a stroke of
// the pen does, the eraser's size at a zoom), the settings that choose them, and compaction
// (src/compact.h, src/bgwork_compact.c): a page file loses the strokes that change nothing, looks
// exactly as before, and only pages the list names are touched.
// Index loops and range comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains, clippy::neg_cmp_op_on_partial_ord, clippy::too_many_arguments)]
mod testlib;

use grimoire::prelude::*;
use std::cell::RefCell;
use testlib::*;

const DPI: f32 = 300.0;
const W: i32 = 1860;
const H: i32 = 2480;
const FULL: Rect = Rect { x: 0, y: 0, w: W, h: H };

fn root() -> String {
    format!("{}/inkwire-{}", tl_tmp(), std::process::id())
}

// ---- what a stroke of the pen does ----

fn pen_stroke_follows_the_settings() {
    let mut s = Settings::default();
    settings_default(&mut s);
    // Defaults: ink with the pen's tool, the eraser takes whole strokes, the button highlights.
    let mut p = pen_stroke(&s, false, false);
    check!(p.act == PenAct::Ink && p.tool == TOOL_PENCIL);
    check!(pen_stroke(&s, true, false).act == PenAct::Whole);
    p = pen_stroke(&s, false, true);
    check!(p.act == PenAct::Ink && p.tool == TOOL_HIGHLIGHTER);
    for mode in 0..2 {
        for white in 0..2 {
            for side in 0..4 {
                for cur in 0..TOOL_COUNT as i32 {
                    s.eraser_mode = mode;
                    s.white = white != 0;
                    s.side_button = side;
                    s.current_tool = ToolKind(cur);
                    s.draw_tool = TOOL_CHARCOAL;
                    let eraser = if mode != 0 { PenAct::Precise } else { PenAct::Whole };
                    let pen = if ToolKind(cur) == TOOL_ERASER { TOOL_CHARCOAL } else { ToolKind(cur) };
                    // The eraser end erases, whatever else.
                    for b in 0..2 {
                        p = pen_stroke(&s, true, b != 0);
                        check!(p.act == eraser && p.tool == TOOL_ERASER);
                    }
                    // The tip alone: the pen's tool, white with `white`; the eraser tool erases.
                    p = pen_stroke(&s, false, false);
                    if ToolKind(cur) == TOOL_ERASER {
                        check!(p.act == eraser && p.tool == TOOL_ERASER);
                    } else {
                        check!(p.act == (if white != 0 { PenAct::White } else { PenAct::Ink }) && p.tool == pen);
                    }
                    // With the side button held.
                    p = pen_stroke(&s, false, true);
                    if side == 1 {
                        check!(p.act == eraser && p.tool == TOOL_ERASER);
                    } else if side == 2 {
                        check!(p.act == PenAct::Ink && p.tool == TOOL_HIGHLIGHTER);
                    } else if side == 3 {
                        check!(p.act == PenAct::White && p.tool == pen);
                    } else if ToolKind(cur) == TOOL_ERASER {
                        check!(p.act == eraser && p.tool == TOOL_ERASER);
                    } else {
                        check!(p.act == (if white != 0 { PenAct::White } else { PenAct::Ink }) && p.tool == pen);
                    }
                }
            }
        }
    }
    // The eraser picked with the toolbar's tool still showing the eraser (a hand-edited file): the
    // pencil is the pen's drawing tool.
    s.current_tool = TOOL_ERASER;
    s.draw_tool = TOOL_ERASER;
    s.side_button = 3;
    check!(pen_drawing_tool(&s) == TOOL_PENCIL && pen_stroke(&s, false, true).tool == TOOL_PENCIL);
    s.draw_tool = ToolKind(99);
    check!(pen_drawing_tool(&s) == TOOL_PENCIL);
}

// The precise eraser keeps its size on screen: its widths divided by the zoom, within the range a
// page file keeps (so the stroke reads back as drawn).
fn eraser_size_follows_the_zoom() {
    let pi = tool_param_table();
    let mut ranges = 0;
    for i in 0..pi.len() {
        // (the C's offsetof(ToolParams, size_min_mm) and size_max_mm are fields 0 and 1)
        if pi[i].field == 0 || pi[i].field == 1 {
            check!(pi[i].min == ERASER_MM_MIN && pi[i].max == ERASER_MM_MAX);
            ranges += 1;
        }
    }
    check!(ranges == 2);
    let mut e = ToolParams::default();
    tool_params_default(TOOL_ERASER, &mut e);
    let mut z = eraser_at_zoom(&e, 1.0);
    check!(z == e);
    z = eraser_at_zoom(&e, 2.0);
    check_near!(z.size_min_mm, e.size_min_mm / 2.0, 1e-6);
    check_near!(z.size_max_mm, e.size_max_mm / 2.0, 1e-6);
    check!(z.opacity_max == e.opacity_max && z.pressure_gamma == e.pressure_gamma && z.smoothing == e.smoothing);
    z = eraser_at_zoom(&e, 4.0); // the 2.5 mm minimum at 4x: about 0.6 mm on the page
    check_near!(z.size_min_mm, 0.625f32, 1e-6);
    z = eraser_at_zoom(&e, 100.0); // never below the range
    check!(z.size_min_mm == ERASER_MM_MIN && z.size_max_mm == ERASER_MM_MIN);
    z = eraser_at_zoom(&e, 0.25); // zoomed out past the page: never above it
    check!(z.size_min_mm == 10.0 && z.size_max_mm == ERASER_MM_MAX);
    let nan = f32::NAN; // (strtof("nan", NULL))
    z = eraser_at_zoom(&e, nan);
    check!(z.size_min_mm == e.size_min_mm && z.size_max_mm == e.size_max_mm);
    z = eraser_at_zoom(&e, 0.0);
    check!(z.size_min_mm == e.size_min_mm);
    let mut bad = e;
    bad.size_min_mm = nan;
    z = eraser_at_zoom(&bad, 2.0);
    check!(z.size_min_mm == ERASER_MM_MIN);
    // An erase stroke with the zoomed settings reads back with exactly them.
    for k in 0..4 {
        let zoom = [1.0f32, 1.37, 3.9, 0.4][k];
        let prm = eraser_at_zoom(&e, zoom);
        let mut pg = page_new(W, H, PAPER_MEDIUM);
        let mut s = stroke_new(TOOL_ERASER, &prm, erase_seed(TOOL_ERASER, 77));
        let q = PenSample { x: 500.0, y: 600.0, pressure: 0.7, tilt_x: 0.0, tilt_y: 0.0, t_ms: 0 };
        stroke_append(&mut s, &q, DPI);
        page_add(&mut pg, s);
        let mut d: Vec<u8> = Vec::new();
        let mut back: Option<Box<Page>> = None;
        check!(page_serialize(&pg, &mut d) == 0 && page_deserialize(&d, &mut back) == 0 && back.as_ref().is_some_and(|b| b.strokes.len() == 1));
        if let Some(b) = &back {
            if b.strokes.len() == 1 {
                check!(b.strokes[0].params == prm);
                check!(b.strokes[0].seed == erase_seed(TOOL_ERASER, 77) && stroke_render_tool(&b.strokes[0]) == TOOL_ERASER);
            }
        }
        if let Some(b) = back {
            page_free(b);
        }
        page_free(pg);
    }
}

// The new keys and side_button's fourth value go through settings.ini as they are; a value past
// what this version has is used as the nearest one and written back as it was.
fn settings_round_trip() {
    let path = cformat!("%s/settings.ini", &store_root());
    let mut s = Settings::default();
    settings_default(&mut s);
    check!(s.eraser_mode == 0 && !s.white && s.side_button == 2);
    s.eraser_mode = 1;
    s.white = true;
    s.side_button = 3;
    check!(settings_save(&s) == 0);
    let mut b = Settings::default();
    check!(settings_load(&mut b) == 0 && b.eraser_mode == 1 && b.white && b.side_button == 3);
    let newer = "eraser_mode=2\nwhite=2\nside_button=5\n";
    check!(write_file_atomic(&path, newer.as_bytes()) == 0);
    check!(settings_load(&mut b) == 0 && b.eraser_mode == 1 && b.white && b.side_button == 3);
    check!(settings_save(&b) == 0);
    let t = read_file(&path).map(|t| String::from_utf8_lossy(&t).into_owned());
    check!(t.as_deref().is_some_and(|t| t.contains("\neraser_mode=2\n") && t.contains("\nwhite=2\n") && t.contains("\nside_button=5\n")));
    let _ = std::fs::remove_file(&path);
}

// ---- compaction ----

fn line(t: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, p: f32, seed: u32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(t, &mut prm);
    let mut s = stroke_new(t, &prm, seed);
    let steps = ((x1 - x0).hypot(y1 - y0) / 10.0) as i32 + 1;
    for i in 0..=steps {
        let f = (i as f32) / (steps as f32);
        let q = PenSample { x: fmla(x1 - x0, f, x0), y: fmla(y1 - y0, f, y0), pressure: p, tilt_x: 10.0, tilt_y: -5.0, t_ms: (i as u32) * 7 };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// The precise eraser's round tip along a line at pressure p (1: its full 7 mm).
fn rubber(x0: f32, y0: f32, x1: f32, y1: f32, p: f32, hash: u32) -> Box<Stroke> {
    let mut s = line(TOOL_ERASER, x0, y0, x1, y1, p, 0);
    s.seed = erase_seed(TOOL_ERASER, hash);
    s
}

// White ink with a pen's texture along a line.
fn white_line(texture: ToolKind, x0: f32, y0: f32, x1: f32, y1: f32, p: f32, hash: u32) -> Box<Stroke> {
    let mut s = line(texture, x0, y0, x1, y1, p, 0);
    s.tool = TOOL_ERASER;
    s.seed = erase_seed(texture, hash);
    s.bbox = RECT_EMPTY; // its reach is its texture's: built again
    let pts = std::mem::take(&mut s.pts);
    for q in &pts {
        stroke_append(&mut s, q, DPI);
    }
    s
}

fn page_path(nb: &NotebookInfo, index: usize) -> String {
    cformat!("%s/notebooks/%s/p%u.qpg", &root(), &nb.id, nb.page_ids[index])
}

fn list_path(nb: &NotebookInfo) -> String {
    cformat!("%s/notebooks/%s/erased.txt", &root(), &nb.id)
}

fn exists(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}

fn file_bytes(path: &str) -> Option<Vec<u8>> {
    read_file(path)
}

// The page in its file, drawn: with its paper, or its ink layer alone (fresh white canvas).
fn render_file(nb: &NotebookInfo, index: i32, layers: i32, c: &mut Canvas, strokes: Option<&mut i32>) {
    let mut p: Option<Box<Page>> = None;
    check!(store_read_page(nb, index, &mut p) == 0 && p.is_some());
    canvas_init(c, W, H);
    let Some(p) = p else { return };
    page_render_layers(c, &p, FULL, DPI, layers);
    if let Some(strokes) = strokes {
        *strokes = p.strokes.len() as i32;
    }
    page_free(p);
}

fn same_px(a: &Canvas, b: &Canvas) -> bool {
    let n = a.w as usize * a.h as usize;
    a.w == b.w && a.h == b.h && a.px[..n] == b.px[..n]
}

// The editor's live path: the page drawn, its paper alone beside it (the editor's cache), and each
// erase stroke started with stroke_brush_begin over them, sample by sample, with the editor's
// settings (the precise eraser at a zoom, white with a pen's own); one that changes no pixel is
// dropped. The page drawn afresh is the live canvas, pixel for pixel. True if the stroke was kept
// (the C returns it, or NULL).
fn live_erase(pg: &mut Page, live: &mut Canvas, paper: &Canvas, texture: ToolKind, prm: &ToolParams, x0: f32, y0: f32, x1: f32, y1: f32, hash: u32, restored: &mut bool) -> bool {
    let mut n = stroke_new(TOOL_ERASER, prm, erase_seed(texture, hash));
    let mut b = BrushState::default();
    // (the brush shares the paper's pixels: a copy made for it)
    stroke_brush_begin(&mut b, &n, DPI, Some(std::rc::Rc::new(paper.px.clone())), 0, 0, paper.stride);
    let steps = ((x1 - x0).hypot(y1 - y0) / 9.0) as i32 + 1;
    for i in 0..=steps {
        let f = (i as f32) / (steps as f32);
        // y0 + (y1 - y0) * f + 25.f * sinf(f * 7.f): both products are fused, the left one first
        let y = fmla(25.0, (f * 7.0).sin(), fmla(y1 - y0, f, y0));
        let q = PenSample { x: fmla(x1 - x0, f, x0), y, pressure: fmla(0.6, f, 0.4), tilt_x: 40.0, tilt_y: 20.0, t_ms: (i as u32) * 6 };
        stroke_append(&mut n, &q, DPI);
        brush_add(&mut b, live, &q);
    }
    *restored = b.restored;
    if !b.restored {
        stroke_free(n);
        return false;
    }
    page_add(pg, n);
    true
}

fn live_erase_equals_rerender() {
    let papers = [PAPER_MEDIUM, PAPER_MUSIC, PAPER_GRID, PAPER_CORNELL];
    let mut st = Settings::default();
    settings_default(&mut st);
    for pk in 0..papers.len() {
        let mut pg = page_new(W, H, papers[pk]);
        let mut live = Canvas::default();
        let mut paper = Canvas::default();
        let mut fresh = Canvas::default();
        canvas_init(&mut live, W, H);
        canvas_init(&mut paper, W, H);
        canvas_init(&mut fresh, W, H);
        paper_render(&mut paper, papers[pk], FULL, DPI);
        for t in 0..TOOL_COUNT as i32 {
            if ToolKind(t) != TOOL_ERASER {
                page_add(&mut pg, line(ToolKind(t), 200.0, (300 + t * 220) as f32, 1600.0, (360 + t * 220) as f32, 0.8, 40 + t as u32));
            }
        }
        page_render(&mut live, &pg, FULL, DPI);
        let mut r = false;
        let zooms = [1.0f32, 2.5, 4.0, 0.6];
        for z in 0..4i32 {
            let e = eraser_at_zoom(&st.tools[TOOL_ERASER.0 as usize], zooms[z as usize]);
            check!(live_erase(&mut pg, &mut live, &paper, TOOL_ERASER, &e, (300 + z * 350) as f32, 200.0, (380 + z * 350) as f32, 2300.0, 50 + z as u32, &mut r));
            check!(r);
        }
        check!(live_erase(&mut pg, &mut live, &paper, TOOL_PENCIL, &st.tools[TOOL_PENCIL.0 as usize], 150.0, 320.0, 1700.0, 345.0, 60, &mut r) && r);
        check!(live_erase(&mut pg, &mut live, &paper, TOOL_CHARCOAL, &st.tools[TOOL_CHARCOAL.0 as usize], 150.0, 1200.0, 1700.0, 1230.0, 61, &mut r) && r);
        // Over blank paper (its lines are the paper): nothing changes, nothing is kept. (Over ink
        // erased already it may: the eraser's soft rim lightens its edge a little more each time.)
        let e = eraser_at_zoom(&st.tools[TOOL_ERASER.0 as usize], 1.0);
        let n = pg.strokes.len();
        check!(!live_erase(&mut pg, &mut live, &paper, TOOL_ERASER, &e, 1750.0, 2350.0, 1800.0, 2400.0, 70, &mut r) && !r);
        check!(!live_erase(&mut pg, &mut live, &paper, TOOL_CALLIGRAPHY, &st.tools[TOOL_CALLIGRAPHY.0 as usize], 40.0, 40.0, 150.0, 200.0, 71, &mut r) && !r);
        check!(pg.strokes.len() == n);
        page_render(&mut fresh, &pg, FULL, DPI);
        check!(same_px(&live, &fresh));
        canvas_free(&mut live);
        canvas_free(&mut paper);
        canvas_free(&mut fresh);
        page_free(pg);
    }
}

// A page with ink the eraser takes away entirely (and the erase stroke over it then changes
// nothing), ink it takes away in part, white charcoal over a word and ink drawn after it all.
fn erased_page(k: PaperKind) -> Box<Page> {
    let mut p = page_new(W, H, k);
    page_add(&mut p, line(TOOL_BALLPOINT, 300.0, 500.0, 900.0, 520.0, 0.7, 11)); // 0: gone under 1
    page_add(&mut p, rubber(270.0, 505.0, 930.0, 515.0, 1.0, 12)); // 1: then changes nothing
    page_add(&mut p, line(TOOL_PENCIL, 300.0, 800.0, 900.0, 800.0, 0.8, 13)); // 2: erased in part by 3
    page_add(&mut p, rubber(600.0, 740.0, 610.0, 860.0, 1.0, 14)); // 3
    page_add(&mut p, line(TOOL_MARKER, 300.0, 1200.0, 1300.0, 1260.0, 0.9, 15)); // 4: whited out in part
    page_add(&mut p, white_line(TOOL_CHARCOAL, 700.0, 1150.0, 900.0, 1300.0, 1.0, 16));
    page_add(&mut p, line(TOOL_FINELINER, 300.0, 1600.0, 1500.0, 1650.0, 0.6, 17)); // 6: drawn after it all
    page_add(&mut p, rubber(1500.0, 2100.0, 1700.0, 2200.0, 1.0, 18)); // 7: over blank paper
    p
}

// Compaction takes out exactly what page_droppable says, the page looks the same (with its paper
// and its ink layer), the strokes it keeps are byte for byte what they were, and a page it is not
// asked about (or one without erase strokes) is not touched. The list goes once it is done.
fn compaction_takes_erased_ink_out() {
    let papers = [PAPER_MEDIUM, PAPER_GRAND, PAPER_DOTS, PAPER_BLANK];
    for pk in 0..papers.len() {
        let mut nb = NotebookInfo::default();
        check!(store_create("Compacted", "", papers[pk], &mut nb) == 0);
        check!(store_add_page(&mut nb, 1, papers[pk]) == 0 && store_add_page(&mut nb, 2, papers[pk]) == 0);
        let p = erased_page(papers[pk]);
        check!(store_save_page(&nb, 0, &p) == 0);
        let q = erased_page(papers[pk]); // the same, on a page nobody lists
        check!(store_save_page(&nb, 2, &q) == 0);
        let mut plain = page_new(W, H, papers[pk]);
        page_add(&mut plain, line(TOOL_PENCIL, 300.0, 300.0, 900.0, 900.0, 0.7, 21));
        check!(store_save_page(&nb, 1, &plain) == 0);
        // What the file should become: the page as read from it, without what page_droppable marks.
        let mut rd: Option<Box<Page>> = None;
        check!(store_read_page(&nb, 0, &mut rd) == 0 && rd.as_ref().is_some_and(|r| r.strokes.len() == 8));
        let mut drop = [false; 16];
        let expect = match &rd {
            Some(r) => page_droppable(r, DPI, &mut drop),
            None => -1,
        };
        check!(expect >= 3 && drop[0] && drop[1] && drop[7] && !drop[2] && !drop[3] && !drop[6]);
        if let Some(r) = rd.as_deref_mut() {
            for i in (0..r.strokes.len() as i32).rev() {
                if drop[i as usize] {
                    stroke_free(page_remove(r, i));
                }
            }
        }
        let mut want: Vec<u8> = Vec::new();
        check!(rd.as_ref().is_some_and(|r| page_serialize(r, &mut want) == 0));
        if let Some(r) = rd {
            page_free(r);
        }
        let mut before = Canvas::default();
        let mut before_ink = Canvas::default();
        let mut after = Canvas::default();
        let mut after_ink = Canvas::default();
        let (mut n_before, mut n_after) = (0, 0);
        render_file(&nb, 0, PAGE_LAYER_ALL, &mut before, Some(&mut n_before));
        render_file(&nb, 0, PAGE_LAYER_INK, &mut before_ink, None);
        let p1 = page_path(&nb, 0);
        let p2 = page_path(&nb, 1);
        let p3 = page_path(&nb, 2);
        let lp = list_path(&nb);
        let b2 = file_bytes(&p2);
        let b3 = file_bytes(&p3);
        // Listed: page 1 (twice: once is kept), page 2 (no erase strokes), a page no longer there.
        check!(!compact_listed(nb.id.as_str()));
        check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0 && compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
        check!(compact_note_page(nb.id.as_str(), nb.page_ids[1]) == 0 && compact_note_page(nb.id.as_str(), 999) == 0);
        check!(compact_listed(nb.id.as_str()) && exists(&lp));
        store_save_page_preview(&nb,nb.page_ids[0],&before);
        let mut cached_preview=Canvas::default();
        check!(store_load_page_preview(&nb,nb.page_ids[0],&mut cached_preview));
        let preview_pixels=cached_preview.px.clone();
        check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
        check!(store_load_page_preview(&nb,nb.page_ids[0],&mut cached_preview));
        check!(cached_preview.px==preview_pixels); // no re-render after equivalent storage compaction
        render_file(&nb, 0, PAGE_LAYER_ALL, &mut after, Some(&mut n_after));
        render_file(&nb, 0, PAGE_LAYER_INK, &mut after_ink, None);
        check!(n_before == 8 && n_after == 8 - expect);
        check!(same_px(&before, &after) && same_px(&before_ink, &after_ink));
        let b1 = file_bytes(&p1);
        check!(b1.as_deref() == Some(&want[..]));
        let c2 = file_bytes(&p2);
        let c3 = file_bytes(&p3);
        check!(b2.is_some() && c2.is_some() && b2 == c2); // listed, nothing to do: as it was
        check!(b3.is_some() && c3.is_some() && b3 == c3); // not listed: as it was
        check!(!compact_listed(nb.id.as_str()) && !exists(&lp));
        let tmp = cformat!("%s.compact.tmp", &p1);
        check!(!exists(&tmp));
        // Again: nothing listed, nothing done.
        check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
        let again = file_bytes(&p1);
        check!(again.is_some() && again == b1);
        canvas_free(&mut before);
        canvas_free(&mut before_ink);
        canvas_free(&mut after);
        canvas_free(&mut after_ink);
        page_free(p);
        page_free(q);
        page_free(plain);
        check!(store_delete(nb.id.as_str()) == 0);
    }
}

// Whether notebook nb's list names page pid.
fn lists(nb: &NotebookInfo, pid: u32) -> bool {
    let lp = list_path(nb);
    let Some(t) = read_file(&lp) else { return false };
    let t = String::from_utf8_lossy(&t).into_owned();
    // (strtoul(l, NULL, 10): blanks, an optional sign, digits; a line that is none gives 0)
    let strtoul = |l: &str| -> u64 {
        let l = l.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
        let l = l.strip_prefix('+').unwrap_or(l);
        let mut v: u64 = 0;
        for c in l.bytes().take_while(|c| c.is_ascii_digit()) {
            v = v.saturating_mul(10).saturating_add((c - b'0') as u64);
        }
        v
    };
    for l in t.split('\n') {
        if !l.starts_with('#') && strtoul(l) == pid as u64 {
            return true;
        }
    }
    false
}

// Pages compaction must leave as they are: from a newer version (read-only), at another size than
// the notebook's (an older version wrote it on another Scribe), damaged, or missing. Each file
// stays byte for byte. A missing page goes off the list (there is no ink in it); the others stay
// listed, and the one of another size is compacted once it is written at the notebook's size.
fn compaction_leaves_what_it_cannot_write() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Left", "", PAPER_MEDIUM, &mut nb) == 0);
    for i in 1..4 {
        check!(store_add_page(&mut nb, i, PAPER_MEDIUM) == 0);
    }
    let path: Vec<String> = (0..4).map(|i| page_path(&nb, i)).collect();
    // 0: version 3 (a newer Grimoire's); 1: a page of 1986x2648 with erased ink; 2: damage; 3: gone.
    let mut v3 = [0u8; 64];
    v3[..16].copy_from_slice(&[b'Q', b'P', b'G', b'1', 3, 0, 1, 0, 0x44, 0x07, 0, 0, 0xB0, 0x09, 0, 0]);
    check!(write_file_atomic(&path[0], &v3) == 0);
    let mut big = page_new(1986, 2648, PAPER_MEDIUM);
    page_add(&mut big, line(TOOL_BALLPOINT, 300.0, 500.0, 900.0, 520.0, 0.7, 31));
    page_add(&mut big, rubber(270.0, 505.0, 930.0, 515.0, 1.0, 32));
    let mut d: Vec<u8> = Vec::new();
    check!(page_serialize(&big, &mut d) == 0 && write_file_atomic(&path[1], &d) == 0);
    page_free(big);
    check!(write_file_atomic(&path[2], b"QPG1\x01\x00") == 0);
    let _ = std::fs::remove_file(&path[3]);
    let was: Vec<Option<Vec<u8>>> = (0..3).map(|i| file_bytes(&path[i])).collect();
    for i in 0..4 {
        check!(compact_note_page(nb.id.as_str(), nb.page_ids[i]) == 0);
    }
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
    for i in 0..3 {
        let now = file_bytes(&path[i]);
        check!(was[i].is_some() && now.is_some() && now == was[i]);
    }
    check!(!exists(&path[3]) && !lists(&nb, nb.page_ids[3]));
    for i in 0..3 {
        check!(lists(&nb, nb.page_ids[i]));
    }
    // The page of another size, written at the notebook's (as the editor writes it after an edit):
    // compacted now, and off the list.
    let mut back: Option<Box<Page>> = None;
    check!(store_read_page(&nb, 1, &mut back) == 0 && back.as_ref().is_some_and(|b| b.w == W && b.strokes.len() == 2));
    check!(back.as_ref().is_some_and(|b| store_save_page(&nb, 1, b) == 0));
    if let Some(b) = back.take() {
        page_free(b);
    }
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
    check!(store_read_page(&nb, 1, &mut back) == 0 && back.as_ref().is_some_and(|b| b.strokes.is_empty()));
    if let Some(b) = back.take() {
        page_free(b);
    }
    check!(!lists(&nb, nb.page_ids[1]) && lists(&nb, nb.page_ids[0]) && lists(&nb, nb.page_ids[2]));
    check!(store_delete(nb.id.as_str()) == 0);
}

// A tip variant this version knows (the vertical highlighter's, ext[2] = 1) is no newer data: such
// a page is compacted like any other, and its strokes keep their tip. A variant it does not know
// (2 on the highlighter) leaves the page as it is.
fn known_tips_are_compacted() {
    for unknown in 0..2 {
        let mut nb = NotebookInfo::default();
        check!(store_create("Tips", "", PAPER_MEDIUM, &mut nb) == 0);
        let mut p = erased_page(PAPER_MEDIUM);
        let mut hl = line(TOOL_HIGHLIGHTER, 300.0, 1900.0, 1400.0, 1920.0, 0.8, 19);
        stroke_mark_tip(&mut hl);
        check!(stroke_tip(&hl) == TIP_HIGHLIGHTER_VERTICAL);
        if unknown != 0 {
            hl.ext[2] = 2;
        }
        page_add(&mut p, hl);
        check!(store_save_page(&nb, 0, &p) == 0);
        let path = page_path(&nb, 0);
        let was = file_bytes(&path);
        check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
        check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
        let now = file_bytes(&path);
        let same = was.is_some() && now.is_some() && now == was;
        let l = now.as_ref().map_or(0, |n| n.len());
        let was_len = was.as_ref().map_or(0, |n| n.len());
        check!(if unknown != 0 { same } else { !same && l < was_len });
        let mut back: Option<Box<Page>> = None;
        check!(store_read_page(&nb, 0, &mut back) == 0 && back.as_ref().is_some_and(|b| !b.strokes.is_empty()));
        if let Some(b) = &back {
            if let Some(last) = b.strokes.last() {
                check!(last.tool == TOOL_HIGHLIGHTER && last.ext[2] == (if unknown != 0 { 2 } else { TIP_HIGHLIGHTER_VERTICAL }));
            }
        }
        if let Some(b) = back {
            page_free(b);
        }
        page_free(p);
        check!(store_delete(nb.id.as_str()) == 0);
    }
}

// Data this version reads past (docs/FORMAT.md, Registry): ext bytes on ink or on an erase stroke,
// an erase stroke's texture it lacks (a later tool's, or 4, the eraser's own), a chunk, a paper it
// does not know. The version that wrote it may draw the page otherwise (a colour, a layer, a
// texture that covers less than the round tip), so such a page is never judged by how this version
// draws it: left byte for byte, and listed for the version that wrote it. The same page without
// that data is compacted.
fn newer_data_is_left_as_it_is() {
    const CASES: usize = 7;
    let mut nb = NotebookInfo::default();
    check!(store_create("Newer", "", PAPER_MEDIUM, &mut nb) == 0);
    for i in 1..CASES as i32 {
        check!(store_add_page(&mut nb, i, PAPER_MEDIUM) == 0);
    }
    let mut pg: Vec<Box<Page>> = (0..CASES).map(|i| erased_page(if i == 5 { PaperKind(0x52000001) } else { PAPER_MEDIUM })).collect();
    pg[0].strokes[0].ext[0] = 5; // ink in a newer version's colour
    pg[1].strokes[1].ext[2] = 1; // a newer version's flag on the eraser
    pg[2].strokes[1].seed = 0x20u32 << 24 | 12; // a texture a later tool would have
    pg[3].strokes[1].seed = 4u32 << 24 | 12; // texture 4, the eraser's own
    const CHUNK: [u8; 11] = [b'n', b'o', b't', b'e', 3, 0, 0, 0, b'a', b'b', b'c']; // safe to copy
    pg[4].tail = CHUNK.to_vec();
    // 5: an image paper of a role this version lacks (2: a newer version's); 6: none of it.
    let mut was: Vec<Option<Vec<u8>>> = Vec::new();
    for i in 0..CASES {
        check!(store_save_page(&nb, i as i32, &pg[i]) == 0);
        let mut rd: Option<Box<Page>> = None;
        check!(store_read_page(&nb, i as i32, &mut rd) == 0 && rd.as_ref().is_some_and(|r| !r.readonly && r.strokes.len() == 8));
        if let Some(r) = rd {
            page_free(r);
        }
        let path = page_path(&nb, i);
        was.push(file_bytes(&path));
        check!(compact_note_page(nb.id.as_str(), nb.page_ids[i]) == 0);
    }
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
    for (i, p) in pg.into_iter().enumerate() {
        let path = page_path(&nb, i);
        let now = file_bytes(&path);
        let same = was[i].is_some() && now.is_some() && now == was[i];
        if i < CASES - 1 {
            check!(same && lists(&nb, nb.page_ids[i]));
        } else {
            check!(!same && now.as_ref().is_some_and(|n| n.len() < was[i].as_ref().map_or(0, |w| w.len())) && !lists(&nb, nb.page_ids[i]));
        }
        page_free(p);
    }
    check!(store_delete(nb.id.as_str()) == 0);
}

// The file is replaced only if it still holds what the pass read: a page written meanwhile (by the
// editor, an older version or a copy from a computer) stays as that writer left it, with no
// temporary file left behind, and listed; the next pass compacts what it holds then.
thread_local! {
    static G_MEANWHILE: RefCell<Option<Box<Page>>> = const { RefCell::new(None) };
}
fn write_meanwhile(path: &str) {
    let mut d: Vec<u8> = Vec::new();
    let ok = G_MEANWHILE.with(|g| g.borrow().as_ref().is_some_and(|p| page_serialize(p, &mut d) == 0));
    if !ok {
        return;
    }
    check!(write_file_atomic(path, &d) == 0);
}

fn a_page_written_meanwhile_stays() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Meanwhile", "", PAPER_MEDIUM, &mut nb) == 0);
    let p = erased_page(PAPER_MEDIUM);
    check!(store_save_page(&nb, 0, &p) == 0);
    let mut mw = erased_page(PAPER_MEDIUM);
    page_add(&mut mw, line(TOOL_FOUNTAIN, 300.0, 1900.0, 1200.0, 1950.0, 0.7, 41));
    let mut want: Vec<u8> = Vec::new();
    check!(page_serialize(&mw, &mut want) == 0);
    G_MEANWHILE.with(|g| *g.borrow_mut() = Some(mw));
    let path = page_path(&nb, 0);
    let tmp = cformat!("%s.compact.tmp", &path);
    check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
    *COMPACT_TEST_BEFORE_REPLACE.lock().unwrap() = Some(write_meanwhile);
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
    *COMPACT_TEST_BEFORE_REPLACE.lock().unwrap() = None;
    let now = file_bytes(&path);
    check!(now.as_deref() == Some(&want[..]));
    check!(!exists(&tmp) && lists(&nb, nb.page_ids[0]));
    // The next pass leaves out of it what the writer's page lets go (its new stroke stays).
    let mut rd: Option<Box<Page>> = None;
    check!(store_read_page(&nb, 0, &mut rd) == 0 && rd.as_ref().is_some_and(|r| r.strokes.len() == 9));
    let mut drop = [false; 16];
    let expect = match &rd {
        Some(r) => page_droppable(r, DPI, &mut drop),
        None => -1,
    };
    check!(expect >= 3 && !drop[8]);
    if let Some(r) = rd.take() {
        page_free(r);
    }
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0);
    check!(store_read_page(&nb, 0, &mut rd) == 0 && rd.as_ref().is_some_and(|r| r.strokes.len() as i32 == 9 - expect));
    check!(rd.as_ref().is_some_and(|r| !r.strokes.is_empty() && r.strokes[r.strokes.len() - 1].tool == TOOL_FOUNTAIN));
    if let Some(r) = rd.take() {
        page_free(r);
    }
    check!(!lists(&nb, nb.page_ids[0]));
    if let Some(m) = G_MEANWHILE.with(|g| g.borrow_mut().take()) {
        page_free(m);
    }
    page_free(p);
    check!(store_delete(nb.id.as_str()) == 0);
}

// A stop (bg_cancel_notebook's SIGTERM) that comes while a page file is being replaced waits for
// the rename: the page is its new file whole, no temporary file is left, and the list, not updated
// yet, still names it (the next pass finds nothing more to leave out and lets it go).
fn stop_now(_path: &str) {
    // SAFETY: kill to our own pid with SIGTERM, which the pass holds blocked here.
    unsafe { libc::kill(libc::getpid(), libc::SIGTERM) };
}

fn a_stop_waits_for_the_rename() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Stopped", "", PAPER_MEDIUM, &mut nb) == 0);
    let p = erased_page(PAPER_MEDIUM);
    check!(store_save_page(&nb, 0, &p) == 0);
    let mut rd: Option<Box<Page>> = None;
    check!(store_read_page(&nb, 0, &mut rd) == 0 && rd.as_ref().is_some_and(|r| r.strokes.len() == 8));
    let mut drop = [false; 16];
    check!(rd.as_ref().is_some_and(|r| page_droppable(r, DPI, &mut drop) >= 3));
    if let Some(r) = rd.as_deref_mut() {
        for i in (0..r.strokes.len() as i32).rev() {
            if drop[i as usize] {
                stroke_free(page_remove(r, i));
            }
        }
    }
    let mut want: Vec<u8> = Vec::new();
    check!(rd.as_ref().is_some_and(|r| page_serialize(r, &mut want) == 0));
    if let Some(r) = rd {
        page_free(r);
    }
    let path = page_path(&nb, 0);
    let tmp = cformat!("%s.compact.tmp", &path);
    check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
    // SAFETY: the child does only what the pass does, then _exit; it shares nothing with the test
    // harness's threads, which fork leaves behind.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        *COMPACT_TEST_BEFORE_REPLACE.lock().unwrap_or_else(|e| e.into_inner()) = Some(stop_now);
        bg_compact_pages(nb.id.as_str(), DPI);
        // SAFETY: _exit ends the forked child without running the parent's exit handlers.
        unsafe { libc::_exit(0) }; // (not reached: the stop lands as soon as the page is replaced)
    }
    let mut st: libc::c_int = 0;
    // SAFETY: waitpid writes the status into our local.
    let w = unsafe { libc::waitpid(pid, &mut st, 0) };
    check!(pid > 0 && w == pid && libc::WIFSIGNALED(st) && libc::WTERMSIG(st) == libc::SIGTERM);
    let now = file_bytes(&path);
    check!(now.is_some() && !want.is_empty() && now.as_deref() == Some(&want[..]));
    check!(!exists(&tmp) && lists(&nb, nb.page_ids[0]));
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0 && !compact_listed(nb.id.as_str()));
    let now = file_bytes(&path);
    check!(now.as_deref() == Some(&want[..]));
    page_free(p);
    check!(store_delete(nb.id.as_str()) == 0);
}

// A list that is there but does not read now (a read error, no memory) is never written over: a
// page noted then would have it list that page alone and forget the others. It is left as it is,
// the editor tries again at its next save, and once it reads the page is added to what it lists.
fn a_list_that_does_not_read_is_kept() {
    use std::os::unix::fs::PermissionsExt;
    let mut nb = NotebookInfo::default();
    check!(store_create("Unread", "", PAPER_MEDIUM, &mut nb) == 0);
    let lp = list_path(&nb);
    let t = "# a list\n3\n4\n";
    check!(write_file_atomic(&lp, t.as_bytes()) == 0);
    check!(std::fs::set_permissions(&lp, std::fs::Permissions::from_mode(0o000)).is_ok());
    if std::fs::File::open(&lp).is_ok() {
        // (run as root, which reads it anyway: nothing to see here)
    } else {
        check!(compact_note_page(nb.id.as_str(), 5) == -1);
        check!(compact_listed(nb.id.as_str())); // it may list pages: the pass is tried
        check!(bg_compact_pages(nb.id.as_str(), DPI) == -1);
    }
    check!(std::fs::set_permissions(&lp, std::fs::Permissions::from_mode(0o644)).is_ok());
    let back = read_file(&lp);
    check!(back.as_deref() == Some(t.as_bytes()));
    check!(compact_note_page(nb.id.as_str(), 5) == 0);
    let back = read_file(&lp);
    let ids = back.as_ref().and_then(|b| b.iter().position(|&c| c == b'\n').map(|i| &b[i..]));
    check!(ids == Some(&b"\n3\n4\n5\n"[..]));
    check!(store_delete(nb.id.as_str()) == 0);
}

// The list: one id per line, each once; comments and lines that are not page ids are skipped; a
// list with no ids lists nothing; and a notebook id that could not be one is never touched.
fn the_list() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Listed", "", PAPER_MEDIUM, &mut nb) == 0);
    let lp = list_path(&nb);
    let t = "# a comment\n\n7\n  9 \nx12\n7\n4294967295\n-3\n0\n12abc\n5\r\n";
    check!(write_file_atomic(&lp, t.as_bytes()) == 0);
    check!(compact_listed(nb.id.as_str()));
    check!(compact_note_page(nb.id.as_str(), 9) == 0 && compact_note_page(nb.id.as_str(), 8) == 0);
    let back = read_file(&lp);
    check!(back.as_ref().is_some_and(|b| b[0] == b'#'));
    // 7, 9, 5 kept in their order, 8 added at the end, nothing else.
    let ids = back.as_ref().and_then(|b| b.iter().position(|&c| c == b'\n').map(|i| &b[i..]));
    check!(ids == Some(&b"\n7\n9\n5\n8\n"[..]));
    let none = "# only a comment\nnothing\n";
    check!(write_file_atomic(&lp, none.as_bytes()) == 0);
    check!(!compact_listed(nb.id.as_str()));
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0 && !exists(&lp)); // nothing listed: the list goes
    check!(!compact_listed("../x") && compact_note_page("../x", 1) != 0 && !compact_listed(""));
    check!(store_delete(nb.id.as_str()) == 0);
}

// At startup every notebook with pages listed is compacted in the background (bgjob.c's child),
// and the page comes out as it would have in the foreground.
fn compaction_left_from_an_earlier_run() {
    let mut nb = NotebookInfo::default();
    let mut other = NotebookInfo::default();
    check!(store_create("Earlier", "", PAPER_WIDE, &mut nb) == 0 && store_create("Untouched", "", PAPER_WIDE, &mut other) == 0);
    let p = erased_page(PAPER_WIDE);
    check!(store_save_page(&nb, 0, &p) == 0 && store_save_page(&other, 0, &p) == 0);
    let mut rd: Option<Box<Page>> = None;
    check!(store_read_page(&nb, 0, &mut rd) == 0 && rd.is_some());
    let mut drop = [false; 16];
    let expect = match &rd {
        Some(r) => page_droppable(r, DPI, &mut drop),
        None => -1,
    };
    if let Some(r) = rd {
        page_free(r);
    }
    check!(expect >= 3 && compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
    let mut before = Canvas::default();
    let mut after = Canvas::default();
    let (mut n0, mut n1, mut n2) = (0, 0, 0);
    render_file(&nb, 0, PAGE_LAYER_ALL, &mut before, Some(&mut n0));
    check!(compact_queue_listed(DPI) == 1);
    let mut ms = 0;
    while bg_busy() && ms < 60000 {
        bg_poll();
        sleep_ms(10);
        ms += 10;
    }
    check!(!bg_busy());
    render_file(&nb, 0, PAGE_LAYER_ALL, &mut after, Some(&mut n1));
    check!(n0 == 8 && n1 == 8 - expect && same_px(&before, &after));
    canvas_free(&mut after);
    render_file(&other, 0, PAGE_LAYER_ALL, &mut after, Some(&mut n2));
    check!(n2 == 8 && same_px(&before, &after));
    check!(!compact_listed(nb.id.as_str()) && compact_queue_listed(DPI) == 0);
    canvas_free(&mut before);
    canvas_free(&mut after);
    page_free(p);
    check!(store_delete(nb.id.as_str()) == 0 && store_delete(other.id.as_str()) == 0);
}

// A sketch reworked under scrubs (page.h: page_droppable takes seconds on it), with strokes to
// leave out.
fn heavy_page() -> Box<Page> {
    let mut p = page_new(W, H, PAPER_MEDIUM);
    // (the coordinates are int expressions in the C, converted where the call takes them)
    for i in 0..360i32 {
        page_add(&mut p, line(TOOL_PENCIL, (100 + (i % 7) * 20) as f32, (100 + i * 6) as f32, (1700 - (i % 5) * 30) as f32, (140 + i * 6) as f32, 0.8, 100 + i as u32));
    }
    for k in 0..20i32 {
        page_add(&mut p, rubber(150.0, (150 + k * 60) as f32, 1650.0, (1300 + k * 50) as f32, 1.0, 7000 + k as u32));
    }
    p
}

extern "C" fn signal_noted(_sig: libc::c_int) {}

// Puts SIGTERM's disposition to signal_noted, returning the one it replaced.
fn note_sigterm() -> libc::sigaction {
    // SAFETY: sigaction is plain data, zeroed and then given a handler that does nothing and an
    // empty mask; the call installs it for SIGTERM and fills `old` with the disposition it replaces.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = signal_noted as extern "C" fn(libc::c_int) as usize;
        libc::sigemptyset(&mut sa.sa_mask);
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGTERM, &sa, &mut old);
        old
    }
}
fn restore_sigterm(old: &libc::sigaction) {
    // SAFETY: old is the disposition note_sigterm got back from sigaction.
    unsafe { libc::sigaction(libc::SIGTERM, old, std::ptr::null_mut()) };
}

// Opening a notebook stops its compaction at once, in the middle of a page, on the Kindle too:
// there Grimoire's SIGTERM handler (input_evdev.c) only notes the signal, and bgjob.c's child is a
// fork that keeps it, so a pass that kept it would run to its end while Grimoire waits in
// bg_cancel_notebook (and the launcher's watchdog kills it after 15 s). The page it was at is as it
// was, still listed, with no temporary file; the next pass compacts it.
fn opening_stops_a_running_pass() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Heavy", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0);
    let mut light = page_new(W, H, PAPER_MEDIUM);
    page_add(&mut light, line(TOOL_PENCIL, 300.0, 300.0, 900.0, 900.0, 0.7, 21));
    let heavy = heavy_page();
    check!(store_save_page(&nb, 0, &light) == 0 && store_save_page(&nb, 1, &heavy) == 0);
    let path = page_path(&nb, 1);
    let tmp = cformat!("%s.compact.tmp", &path);
    let was = file_bytes(&path);
    check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0 && compact_note_page(nb.id.as_str(), nb.page_ids[1]) == 0);
    check!(!bg_busy());
    let old = note_sigterm();
    bg_queue(BgKind::CompactPages, Some(nb.id.as_str()), None, DPI, false);
    // Once the light page is done (off the list), the pass is in the heavy one.
    let mut ms = 0;
    while lists(&nb, nb.page_ids[0]) && ms < 60000 {
        sleep_ms(2);
        ms += 2;
    }
    check!(!lists(&nb, nb.page_ids[0]) && lists(&nb, nb.page_ids[1]));
    let t0 = now_ms();
    bg_cancel_notebook(nb.id.as_str());
    let took = now_ms().wrapping_sub(t0);
    restore_sigterm(&old);
    check!(took < 1000);
    check!(!bg_busy());
    let now = file_bytes(&path);
    check!(was.is_some() && now.is_some() && now == was);
    check!(lists(&nb, nb.page_ids[1]) && !exists(&tmp));
    // The next pass gets through, and finds ink to leave out (so the stop above came in time).
    check!(bg_compact_pages(nb.id.as_str(), DPI) == 0 && !compact_listed(nb.id.as_str()));
    let mut rd: Option<Box<Page>> = None;
    check!(store_read_page(&nb, 1, &mut rd) == 0 && rd.as_ref().is_some_and(|r| r.strokes.len() < heavy.strokes.len()));
    if let Some(r) = rd {
        page_free(r);
    }
    page_free(light);
    page_free(heavy);
    check!(store_delete(nb.id.as_str()) == 0);
}

// The same stop sent right after the fork, before the child could set handlers of its own (a
// notebook opened in the very pass that started its compaction): bgjob.c holds the stops across the
// fork, so the child takes the stop with its default action and goes at once, with nothing written.
fn a_stop_right_after_the_fork() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Heavy too", "", PAPER_MEDIUM, &mut nb) == 0);
    let heavy = heavy_page();
    check!(store_save_page(&nb, 0, &heavy) == 0);
    let path = page_path(&nb, 0);
    let tmp = cformat!("%s.compact.tmp", &path);
    let was = file_bytes(&path);
    check!(compact_note_page(nb.id.as_str(), nb.page_ids[0]) == 0);
    check!(!bg_busy());
    let old = note_sigterm();
    bg_queue(BgKind::CompactPages, Some(nb.id.as_str()), None, DPI, false);
    let t0 = now_ms();
    bg_cancel_notebook(nb.id.as_str());
    let took = now_ms().wrapping_sub(t0);
    restore_sigterm(&old);
    check!(took < 500);
    check!(!bg_busy());
    let now = file_bytes(&path);
    check!(was.is_some() && now.is_some() && now == was);
    check!(lists(&nb, nb.page_ids[0]) && !exists(&tmp));
    page_free(heavy);
    check!(store_delete(nb.id.as_str()) == 0);
}

#[test]
fn test_inkwire() {
    let r = root();
    let _ = std::fs::remove_dir_all(&r);
    check!(store_init(&r) == 0);
    store_set_device(DPI, W, H);
    run!(pen_stroke_follows_the_settings);
    run!(eraser_size_follows_the_zoom);
    run!(settings_round_trip);
    run!(live_erase_equals_rerender);
    run!(compaction_takes_erased_ink_out);
    run!(compaction_leaves_what_it_cannot_write);
    run!(newer_data_is_left_as_it_is);
    run!(known_tips_are_compacted);
    run!(a_page_written_meanwhile_stays);
    run!(a_stop_waits_for_the_rename);
    run!(a_list_that_does_not_read_is_kept);
    run!(the_list);
    run!(compaction_left_from_an_earlier_run);
    run!(opening_stops_a_running_pass);
    run!(a_stop_right_after_the_fork);
    let _ = std::fs::remove_dir_all(&r);
    done();
}
