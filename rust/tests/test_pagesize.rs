// test_pagesize.c: a notebook keeps the page size it was made with (the Scribe 3 and the Scribe
// Colorsoft are 1986x2648, the 2022 and 2024 Scribes 1860x2480), wherever it is opened; a page an
// older version scaled to another screen goes back to its notebook's size; calibrations follow the
// panel; how many music systems fit is worked out for the page's own height; and which fast
// refreshes a colour panel waits after (all but live ink).
// Index loops and range comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains, clippy::neg_cmp_op_on_partial_ord)]
mod testlib;

use grimoire::prelude::*;
use testlib::*;

const DPI: f32 = 300.0;
// the Scribe (2022, 2024); the Scribe 3 / Colorsoft
const W2: i32 = 1860;
const H2: i32 = 2480;
const W3: i32 = 1986;
const H3: i32 = 2648;

fn root() -> String {
    format!("{}/pagesize-{}", tl_tmp(), std::process::id())
}

// ---- helpers ----

fn wave(t: ToolKind, y: f32, seed: u32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    let mut s = stroke_new(t, &p, seed);
    for i in 0..30 {
        let q = PenSample {
            x: fmla(i as f32, 55.0, 150.0),
            y: fmla((i % 6) as f32, 11.0, y),
            pressure: 0.15 + (i as f32) / 40.0,
            tilt_x: ((i % 9) as f32) - 4.0,
            tilt_y: 3.0 - ((i % 5) as f32),
            t_ms: (i as u32) * 7,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

fn page_file(nb: &NotebookInfo, index: usize) -> String {
    cformat!("%s/notebooks/%s/p%u.qpg", &root(), &nb.id, nb.page_ids[index])
}

fn info_file(nb: &NotebookInfo) -> String {
    cformat!("%s/notebooks/%s/notebook.ini", &root(), &nb.id)
}

// The size in a page file's first 16 bytes; None if it has none.
fn file_size(path: &str) -> Option<(i32, i32)> {
    use std::io::Read;
    let mut b = [0u8; 16];
    let mut f = std::fs::File::open(path).ok()?;
    f.read_exact(&mut b).ok()?;
    if &b[..4] != b"QPG1" {
        return None;
    }
    let w = (b[8] as u32 | (b[9] as u32) << 8 | (b[10] as u32) << 16 | (b[11] as u32) << 24) as i32;
    let h = (b[12] as u32 | (b[13] as u32) << 8 | (b[14] as u32) << 16 | (b[15] as u32) << 24) as i32;
    Some((w, h))
}

fn text_of(path: &str) -> Option<String> {
    read_file(path).map(|b| String::from_utf8_lossy(&b).into_owned())
}

// How many lines of text are exactly this line.
fn has_line(text: &str, line: &str) -> i32 {
    text.split('\n').filter(|l| *l == line).count() as i32
}

// notebook.ini without the lines that start with prefix1 or prefix2 (what 1.0.0-beta's save leaves
// of the keys it does not know).
fn drop_lines(nb: &NotebookInfo, prefix1: &str, prefix2: Option<&str>) {
    let path = info_file(nb);
    let t = text_of(&path);
    check!(t.is_some());
    let Some(t) = t else { return };
    let mut out = String::new();
    // (strtok(t, "\n"): the lines, none of them empty)
    for line in t.split('\n').filter(|l| !l.is_empty()) {
        if line.starts_with(prefix1) || prefix2.is_some_and(|p| line.starts_with(p)) {
            continue;
        }
        out.push_str(&cformat!("%s\n", line));
    }
    check!(write_file_atomic(&path, out.as_bytes()) == 0);
}

fn append_line(nb: &NotebookInfo, line: &str) {
    let path = info_file(nb);
    let t = text_of(&path);
    check!(t.is_some());
    let Some(t) = t else { return };
    let out = cformat!("%s%s\n", &t, line);
    check!(write_file_atomic(&path, out.as_bytes()) == 0);
}

fn render(p: &Page, c: &mut Canvas) -> bool {
    if canvas_init(c, p.w, p.h) != 0 {
        return false;
    }
    page_render(c, p, Rect { x: 0, y: 0, w: p.w, h: p.h }, DPI);
    true
}

fn same_pixels(a: &Canvas, b: &Canvas) -> bool {
    if a.w != b.w || a.h != b.h {
        return false;
    }
    for y in 0..a.h {
        let (ra, rb) = ((y * a.stride) as usize, (y * b.stride) as usize);
        if a.px[ra..ra + a.w as usize] != b.px[rb..rb + a.w as usize] {
            return false;
        }
    }
    true
}

// store_load_info(nb.id, &nb): the id is read out of the record it fills.
fn reload(nb: &mut NotebookInfo) -> i32 {
    let id = nb.id;
    store_load_info(id.as_str(), nb)
}

// Writes page `index` of nb as a version before 1.1 would after an edit on a screen of w x h: it
// loaded the page scaled to its screen and saved it at that size.
fn older_version_rescales(nb: &NotebookInfo, index: i32, w: i32, h: i32) {
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(nb, index, w, h, &mut p) == 0 && p.as_ref().is_some_and(|p| p.w == w && p.h == h));
    if let Some(p) = p {
        check!(store_save_page(nb, index, &p) == 0);
        page_free(p);
    }
}

// ---- notebooks keep their size ----

// A notebook made on an 1860x2480 Scribe, opened with the device at 1986x2648: it keeps its size,
// its pages render byte for byte as they did, and new pages get its size, not the screen's.
fn notebook_keeps_its_size() {
    store_set_device(DPI, W2, H2);
    let mut nb = NotebookInfo::default();
    check!(store_create("Made on a Scribe 2", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(nb.page_w == W2 && nb.page_h == H2);
    check!(store_add_page(&mut nb, 1, PAPER_MUSIC) == 0 && nb.npages() == 2);
    let tools = [[TOOL_PENCIL, TOOL_FOUNTAIN, TOOL_HIGHLIGHTER], [TOOL_CALLIGRAPHY, TOOL_CHARCOAL, TOOL_MARKER]];
    let mut refs = [Canvas::default(), Canvas::default()];
    for i in 0..2 {
        let mut p: Option<Box<Page>> = None;
        check!(store_load_page(&nb, i as i32, W2, H2, &mut p) == 0 && p.is_some());
        let Some(mut pg) = p else { return };
        for k in 0..3 {
            page_add(&mut pg, wave(tools[i][k], 400.0 + 700.0 * (k as f32), 11 + k as u32));
        }
        check!(store_save_page(&nb, i as i32, &pg) == 0);
        page_free(pg);
        // How the Scribe 2 shows it, read back (pressure and tilt are stored rounded).
        let mut p: Option<Box<Page>> = None;
        check!(store_load_page(&nb, i as i32, W2, H2, &mut p) == 0 && p.as_ref().is_some_and(|p| p.strokes.len() == 3));
        let Some(pg) = p else { return };
        if !render(&pg, &mut refs[i]) {
            return;
        }
        page_free(pg);
    }
    let path = info_file(&nb);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "page_w=1860") == 1 && has_line(t, "page_h=2480") == 1));

    // Now on a Scribe Colorsoft.
    store_set_device(DPI, W3, H3);
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(&back, &mut w, &mut h);
    check!(w == W2 && h == H2);
    for i in 0..2 {
        let mut p: Option<Box<Page>> = None;
        check!(store_load_page(&back, i as i32, w, h, &mut p) == 0 && p.as_ref().is_some_and(|p| p.w == W2 && p.h == H2 && p.strokes.len() == 3));
        let mut c = Canvas::default();
        if let Some(pg) = &p {
            if render(pg, &mut c) {
                check!(same_pixels(&c, &refs[i]));
                canvas_free(&mut c);
            }
        }
        if let Some(pg) = p.take() {
            page_free(pg);
        }
        // Exports read it the same way, with nothing changed on disk.
        check!(store_read_page(&back, i as i32, &mut p) == 0 && p.as_ref().is_some_and(|p| p.w == W2 && p.h == H2 && p.strokes.len() == 3));
        if let Some(pg) = &p {
            if render(pg, &mut c) {
                check!(same_pixels(&c, &refs[i]));
                canvas_free(&mut c);
            }
        }
        if let Some(pg) = p.take() {
            page_free(pg);
        }
        let mut png = Buf::default();
        let mut pr = PageRender::default();
        check!(export_page_png_ex(&back, i as i32, DPI, PAGE_LAYER_ALL, 1, Some(&mut pr), &mut png) == 0 && pr.w == W2 && pr.h == H2);
        buf_free(&mut png);
    }
    // A new page takes the notebook's size, not the screen's.
    let n = back.npages();
    check!(store_add_page(&mut back, n, PAPER_GRID) == 0 && back.npages() == 3);
    let path = page_file(&back, 2);
    check!(file_size(&path).is_some_and(|(fw, fh)| fw == W2 && fh == H2));
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&back, 2, W2, H2, &mut p) == 0 && p.as_ref().is_some_and(|p| p.w == W2 && p.paper == PAPER_GRID));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    // A page whose file is missing is blank paper of the notebook's size, for the editor and exports.
    check!(std::fs::remove_file(&path).is_ok());
    check!(store_read_page(&back, 2, &mut p) == 0 && p.as_ref().is_some_and(|p| p.w == W2 && p.h == H2 && p.paper == PAPER_MEDIUM));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let mut png = Buf::default();
    let mut pr = PageRender::default();
    check!(export_page_png_ex(&back, 2, DPI, PAGE_LAYER_ALL, 1, Some(&mut pr), &mut png) == 0 && pr.w == W2 && pr.h == H2);
    buf_free(&mut png);
    // The PDF's pages are the notebook's paper size: 1860 x 2480 px at 300 dpi.
    let mut pdf = Buf::default();
    check!(export_notebook_pdf(&back, DPI, &mut pdf) == 0 && !pdf.data.is_empty());
    let mut boxes = 0;
    for i in 0..pdf.data.len().saturating_sub(30) {
        boxes += (&pdf.data[i..i + 29] == b"/MediaBox [0 0 446.40 595.20]") as i32;
    }
    check!(boxes == 3);
    buf_free(&mut pdf);
    // A rename (a load and a save) keeps the size.
    check!(store_rename(back.id.as_str(), "Still a Scribe 2 notebook") == 0);
    check!(reload(&mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    // A notebook made here is the Colorsoft's size.
    let mut ksc = NotebookInfo::default();
    check!(store_create("Made on a Colorsoft", "", PAPER_WIDE, &mut ksc) == 0 && ksc.page_w == W3 && ksc.page_h == H3);
    let path = page_file(&ksc, 0);
    check!(file_size(&path).is_some_and(|(fw, fh)| fw == W3 && fh == H3));
    check!(store_add_page(&mut ksc, 1, PAPER_WIDE) == 0);
    let path = page_file(&ksc, 1);
    check!(file_size(&path).is_some_and(|(fw, fh)| fw == W3 && fh == H3));
    // And back on the Scribe 2, it stays 1986x2648.
    store_set_device(DPI, W2, H2);
    check!(reload(&mut ksc) == 0 && ksc.page_w == W3 && ksc.page_h == H3);
    check!(store_add_page(&mut ksc, 2, PAPER_WIDE) == 0);
    let path = page_file(&ksc, 2);
    check!(file_size(&path).is_some_and(|(fw, fh)| fw == W3 && fh == H3));
    canvas_free(&mut refs[0]);
    canvas_free(&mut refs[1]);
}

// A page a version before 1.1 scaled to another screen (and saved so after an edit) goes back to
// its notebook's size, its ink where it was written.
fn rescaled_pages_come_back() {
    store_set_device(DPI, W2, H2);
    let mut nb = NotebookInfo::default();
    check!(store_create("Moved about", "", PAPER_NARROW, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_NARROW) == 0);
    let mut orig: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 1, W2, H2, &mut orig) == 0 && orig.is_some());
    let Some(mut orig) = orig else { return };
    page_add(&mut orig, wave(TOOL_BALLPOINT, 900.0, 5));
    page_add(&mut orig, wave(TOOL_PENCIL, 1900.0, 6));
    check!(store_save_page(&nb, 1, &orig) == 0);
    // 1.0.0-beta.2 on a Colorsoft edits page 2: saved at 1986x2648, its points scaled up.
    older_version_rescales(&nb, 1, W3, H3);
    let path = page_file(&nb, 1);
    check!(file_size(&path).is_some_and(|(fw, fh)| fw == W3 && fh == H3));
    // Opened here (on either device), it is the notebook's size again.
    for dev in 0..2 {
        store_set_device(DPI, if dev != 0 { W3 } else { W2 }, if dev != 0 { H3 } else { H2 });
        let mut back = NotebookInfo::default();
        check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
        for via in 0..2 {
            let mut p: Option<Box<Page>> = None;
            if via == 0 {
                check!(store_load_page(&back, 1, W2, H2, &mut p) == 0);
            } else {
                check!(store_read_page(&back, 1, &mut p) == 0);
            }
            check!(p.as_ref().is_some_and(|p| p.w == W2 && p.h == H2 && p.strokes.len() == orig.strokes.len()));
            if let Some(pg) = &p {
                let mut s = 0;
                while s < pg.strokes.len() && s < orig.strokes.len() {
                    let mut k = 0;
                    while k < pg.strokes[s].pts.len() {
                        check_near!(pg.strokes[s].pts[k].x, orig.strokes[s].pts[k].x, 0.01);
                        check_near!(pg.strokes[s].pts[k].y, orig.strokes[s].pts[k].y, 0.01);
                        k += 7;
                    }
                    s += 1;
                }
            }
            if let Some(pg) = p.take() {
                page_free(pg);
            }
        }
        let mut png = Buf::default();
        let mut pr = PageRender::default();
        check!(export_page_png_ex(&back, 1, DPI, PAGE_LAYER_ALL, 1, Some(&mut pr), &mut png) == 0 && pr.w == W2 && pr.h == H2);
        buf_free(&mut png);
        // Exports changed nothing on disk.
        check!(file_size(&path).is_some_and(|(fw, fh)| fw == W3 && fh == H3));
    }
    // Written at the notebook's size again by the first save here.
    store_set_device(DPI, W3, H3);
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 1, W2, H2, &mut p) == 0 && p.is_some());
    if let Some(pg) = p {
        check!(store_save_page(&nb, 1, &pg) == 0);
        page_free(pg);
    }
    check!(file_size(&path).is_some_and(|(fw, fh)| fw == W2 && fh == H2));
    page_free(orig);
}

// notebook.ini without the size (from before 1.1, or saved by 1.0.0-beta, which drops the keys):
// the size most of its page files have gives it (a tie: the first page's), else the device does.
// The next save records it.
fn size_found_without_the_keys() {
    store_set_device(DPI, W2, H2);
    let mut nb = NotebookInfo::default();
    check!(store_create("No size recorded", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0 && store_add_page(&mut nb, 2, PAPER_MEDIUM) == 0);
    drop_lines(&nb, "page_w=", Some("page_h="));
    store_set_device(DPI, W3, H3); // opened on a Colorsoft
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    // Pages that do not read do not count: the others say.
    let p1 = page_file(&nb, 0);
    let p2 = page_file(&nb, 1);
    let p3 = page_file(&nb, 2);
    check!(write_file_atomic(&p1, b"QPG1") == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    check!(write_file_atomic(&p1, b"not a page at all, longer than sixteen bytes") == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    let _ = std::fs::remove_file(&p2);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    // A page from a newer version still has its size in the first 16 bytes. One page of each size
    // (a tie) gives the size of the first in page order; two of one size outnumber one.
    let header = |w: i32, h: i32| -> [u8; 24] {
        let mut v = [0u8; 24];
        v[..8].copy_from_slice(&[b'Q', b'P', b'G', b'1', 3, 0, 4, 0]);
        v[8] = (w & 255) as u8;
        v[9] = (w >> 8) as u8;
        v[12] = (h & 255) as u8;
        v[13] = (h >> 8) as u8;
        v
    };
    let v3 = header(W3, H3);
    let v2 = header(W2, H2);
    check!(write_file_atomic(&p1, &v3) == 0); // 1986x2648, none, 1860x2480
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W3 && back.page_h == H3);
    check!(write_file_atomic(&p1, &v2) == 0 && write_file_atomic(&p3, &v3) == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    check!(write_file_atomic(&p2, &v3) == 0); // 1860x2480, 1986x2648, 1986x2648
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W3 && back.page_h == H3);
    store_set_device(DPI, W2, H2);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W3 && back.page_h == H3);
    store_set_device(DPI, W3, H3);
    // No page file with a size at all: the device's.
    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
    let _ = std::fs::remove_file(&p3);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W3 && back.page_h == H3);
    store_set_device(DPI, W2, H2);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
    // Values no screen has are not taken.
    const BAD: [&str; 5] = ["page_w=0", "page_w=99", "page_w=7000", "page_w=-1986", "page_w=abc"];
    for bad in BAD {
        store_set_device(DPI, W3, H3);
        check!(store_save_info(&mut back) == 0); // writes page_w=1860, page_h=2480
        drop_lines(&back, "page_w=", None);
        append_line(&back, bad);
        let mut b2 = NotebookInfo::default();
        check!(store_load_info(nb.id.as_str(), &mut b2) == 0 && b2.page_w == W3 && b2.page_h == H3); // no page file: the device
    }
    // The next save records what was found, once (no copy among the lines kept for newer versions).
    store_set_device(DPI, W2, H2);
    check!(reload(&mut back) == 0 && store_save_info(&mut back) == 0);
    let path = info_file(&nb);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "page_w=1860") == 1 && has_line(t, "page_h=2480") == 1 && !t.contains("page_w=abc")));
    // Read by this version, a 1.0.0-beta.2 notebook.ini that carried the keys among its unknown
    // lines (after the ones it knows) gives the size too.
    drop_lines(&nb, "page_w=", Some("page_h="));
    append_line(&nb, "lang=es");
    append_line(&nb, "page_w=1986");
    append_line(&nb, "page_h=2648");
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W3 && back.page_h == H3);
    check!(back.extra.as_str().contains("lang=es") && !back.extra.as_str().contains("page_"));
}

// A ballpoint line across the page at height y.
fn line_at(y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut p);
    let mut s = stroke_new(TOOL_BALLPOINT, &p, 1);
    for i in 0..10 {
        let q = PenSample { x: fmla(100.0, i as f32, 200.0), y, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// A notebook made on an 1860x2480 Scribe, three pages with a line near the bottom of each. A
// version before 1.1 on a Colorsoft re-saves page 1 alone (1.0.0-beta does after a page delete)
// and drops the size from notebook.ini. Most of its pages say 1860x2480: on either Scribe every
// page opens at that size with its line where it was written, page 1's scaled back.
fn most_pages_give_the_size() {
    store_set_device(DPI, W2, H2);
    let mut nb = NotebookInfo::default();
    check!(store_create("Mixed by an older version", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0 && store_add_page(&mut nb, 2, PAPER_MEDIUM) == 0);
    for i in 0..3 {
        let mut p: Option<Box<Page>> = None;
        check!(store_load_page(&nb, i, W2, H2, &mut p) == 0 && p.is_some());
        let Some(mut pg) = p else { return };
        page_add(&mut pg, line_at(2300.0));
        check!(store_save_page(&nb, i, &pg) == 0);
        page_free(pg);
    }
    older_version_rescales(&nb, 0, W3, H3);
    drop_lines(&nb, "page_w=", Some("page_h="));
    for dev in 0..2 {
        store_set_device(DPI, if dev != 0 { W3 } else { W2 }, if dev != 0 { H3 } else { H2 });
        let mut back = NotebookInfo::default();
        check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W2 && back.page_h == H2);
        for i in 0..3 {
            let mut p: Option<Box<Page>> = None;
            check!(store_load_page(&back, i, W2, H2, &mut p) == 0 && p.as_ref().is_some_and(|p| p.strokes.len() == 1));
            if let Some(pg) = &p {
                if pg.strokes.len() == 1 {
                    check_near!(pg.strokes[0].pts[0].y, 2300.0, 0.01);
                }
            }
            if let Some(pg) = p.take() {
                page_free(pg);
            }
        }
    }
    // Page 2 re-saved there too: now most pages are 1986x2648, and the notebook takes that size.
    older_version_rescales(&nb, 1, W3, H3);
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.page_w == W3 && back.page_h == H3);
}

// ---- calibration follows the panel ----

// calib_to_panel with the record given as values.
fn to_panel(c: &mut InputCalib, from_w: i32, from_h: i32, w: i32, h: i32) -> CalibMove {
    let (mut fw, mut fh) = (from_w, from_h);
    calib_to_panel(c, &mut fw, &mut fh, w, h)
}

// A listing records the size it had to work out (notebooks from before 1.1), once: as written,
// with the notebook's modified time unchanged, so the Library's order stays as it was.
fn listing_records_a_found_size() {
    store_set_device(DPI, W2, H2);
    let mut nb = NotebookInfo::default();
    check!(store_create("Listed before 1.1", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0);
    drop_lines(&nb, "page_w=", Some("page_h="));
    let mut before = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut before) == 0);
    let path = info_file(&nb);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "page_w=1860") == 0));
    store_set_device(DPI, W3, H3); // listed on a Colorsoft: the pages still say 1860x2480
    let mut all: Vec<NotebookInfo> = Vec::new();
    check!(store_list(&mut all) == 0 && !all.is_empty());
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "page_w=1860") == 1 && has_line(t, "page_h=2480") == 1));
    let mut after = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut after) == 0 && after.page_w == W2 && after.page_h == H2);
    check!(after.modified == before.modified && after.name == before.name && after.npages() == before.npages());
    store_set_device(DPI, W2, H2);
}

fn calibration_scaling_math() {
    // The default mapping scales exactly.
    let mut d = InputCalib { valid: true, swap_xy: false, ax: W2 as f32, bx: 0.0, ay: H2 as f32, by: 0.0 };
    calib_scale(&mut d, W2, H2, W3, H3);
    check!(d.ax == W3 as f32 && d.ay == H3 as f32 && d.bx == 0.0 && d.by == 0.0);
    // A Scribe 2 calibration (not recorded: taken as 1860x2480) on a Colorsoft: proportional.
    let mut c = InputCalib { valid: true, swap_xy: false, ax: 1851.0, bx: 7.5, ay: 2493.0, by: -12.0 };
    let c0 = c;
    let (w2, h2, w3, h3) = (W2 as f64, H2 as f64, W3 as f64, H3 as f64);
    check!(to_panel(&mut c, 0, 0, W3, H3) == CalibMove::Scaled);
    check_near!(c.ax, 1851.0 * w3 / w2, 1e-3);
    check_near!(c.bx, 7.5 * w3 / w2, 1e-4);
    check_near!(c.ay, 2493.0 * h3 / h2, 1e-3);
    check_near!(c.by, -12.0 * h3 / h2, 1e-4);
    // Where it maps a point is that point scaled to the new panel.
    const PTS: [[f32; 2]; 3] = [[0.1, 0.2], [0.5, 0.5], [0.93, 0.81]];
    for i in 0..3 {
        let (mut x0, mut y0, mut x1, mut y1) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        calib_apply(&c0, PTS[i][0], PTS[i][1], &mut x0, &mut y0);
        calib_apply(&c, PTS[i][0], PTS[i][1], &mut x1, &mut y1);
        check_near!(x1, x0 * (W3 as f32) / (W2 as f32), 0.01);
        check_near!(y1, y0 * (H3 as f32) / (H2 as f32), 0.01);
    }
    // Back on the Scribe 2 (recorded as the Colorsoft's now): where it was.
    check!(to_panel(&mut c, W3, H3, W2, H2) == CalibMove::Scaled);
    check_near!(c.ax, c0.ax, 1e-3);
    check_near!(c.bx, c0.bx, 1e-4);
    check_near!(c.ay, c0.ay, 1e-3);
    check_near!(c.by, c0.by, 1e-4);
    // Swapped and mirrored axes: ax and bx still go with the panel's width.
    let mut s = InputCalib { valid: true, swap_xy: true, ax: -1860.0, bx: 1860.0, ay: 2480.0, by: 0.0 };
    check!(to_panel(&mut s, W2, H2, W3, H3) == CalibMove::Scaled);
    check!(s.swap_xy && s.ax == -1986.0 && s.bx == 1986.0 && s.ay == 2648.0 && s.by == 0.0);
    // The same panel, or no calibration: nothing changes.
    let mut same = c0;
    check!(to_panel(&mut same, W2, H2, W2, H2) == CalibMove::Same && same == c0);
    check!(to_panel(&mut same, 0, 0, W2, H2) == CalibMove::Same && same == c0);
    let mut off = InputCalib { valid: false, swap_xy: false, ax: 1860.0, bx: 0.0, ay: 2480.0, by: 0.0 };
    let off0 = off;
    check!(to_panel(&mut off, 0, 0, W3, H3) == CalibMove::Same && off == off0);
    // Not recorded, but its scale says this Colorsoft (a calibration 1.0.0-beta.2 made here): kept.
    let mut here = InputCalib { valid: true, swap_xy: false, ax: 1979.0, bx: 4.0, ay: 2655.0, by: -3.0 };
    let here0 = here;
    check!(to_panel(&mut here, 0, 0, W3, H3) == CalibMove::MadeHere && here == here0);
    // Recorded as a Colorsoft's, but redone on this Scribe 2 by a version that carried the line.
    let mut stale = InputCalib { valid: true, swap_xy: false, ax: 1858.0, bx: 2.0, ay: 2477.0, by: 1.0 };
    let stale0 = stale;
    check!(to_panel(&mut stale, W3, H3, W2, H2) == CalibMove::MadeHere && stale == stale0);
    // In between (3% off both): the record, or the 1860x2480 it is taken as, decides.
    let mut mid = InputCalib { valid: true, swap_xy: false, ax: 1926.0, bx: 0.0, ay: 2568.0, by: 0.0 };
    check!(to_panel(&mut mid, 0, 0, W3, H3) == CalibMove::Scaled);
    check_near!(mid.ax, 1926.0 * w3 / w2, 1e-3);
    // Panels whose sides change by different ratios (the Scribes' two do not: 1986 / 1860 is
    // 2648 / 2480): ax and bx go by the widths, ay and by by the heights, the axes swapped or not.
    for swap in 0..2 {
        let mut u = InputCalib { valid: true, swap_xy: swap != 0, ax: 1000.0, bx: 12.0, ay: 2000.0, by: -8.0 };
        let u0 = u;
        check!(to_panel(&mut u, 1000, 2000, 1500, 2200) == CalibMove::Scaled);
        check!(u.swap_xy == (swap != 0));
        check_near!(u.ax, 1500.0, 1e-3);
        check_near!(u.bx, 18.0, 1e-4);
        check_near!(u.ay, 2200.0, 1e-3);
        check_near!(u.by, -8.8, 1e-4);
        for i in 0..3 {
            let (mut x0, mut y0, mut x1, mut y1) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            calib_apply(&u0, PTS[i][0], PTS[i][1], &mut x0, &mut y0);
            calib_apply(&u, PTS[i][0], PTS[i][1], &mut x1, &mut y1);
            check_near!(x1, (x0 as f64) * 1.5, 0.01);
            check_near!(y1, (y0 as f64) * 1.1, 0.01);
        }
    }
    // A record gone stale the other way: recorded as this panel's, but spanning a Scribe's of the
    // other size and not this one (1.0.0-beta.2 carried the line through a calibration it made on
    // the other Scribe). It was made there, and is scaled from there, on either Scribe.
    let (mut fw, mut fh) = (W3, H3);
    let mut s2 = InputCalib { valid: true, swap_xy: false, ax: 1858.0, bx: 2.0, ay: 2477.0, by: 1.0 };
    check!(calib_to_panel(&mut s2, &mut fw, &mut fh, W3, H3) == CalibMove::Scaled && fw == W2 && fh == H2);
    check_near!(s2.ax, 1858.0 * w3 / w2, 1e-3);
    check_near!(s2.bx, 2.0 * w3 / w2, 1e-4);
    check_near!(s2.ay, 2477.0 * h3 / h2, 1e-3);
    check_near!(s2.by, 1.0 * h3 / h2, 1e-4);
    let mut s3 = InputCalib { valid: true, swap_xy: false, ax: 1979.0, bx: 4.0, ay: 2655.0, by: -3.0 };
    let s30 = s3;
    fw = W2;
    fh = H2;
    check!(calib_to_panel(&mut s3, &mut fw, &mut fh, W2, H2) == CalibMove::Scaled && fw == W3 && fh == H3);
    check_near!(s3.ax, 1979.0 * w2 / w3, 1e-3);
    check_near!(s3.ay, 2655.0 * h2 / h3, 1e-3);
    // Not recorded (so taken as an 1860x2480 Scribe's) on a Scribe 2, but a Colorsoft's: scaled too.
    let mut n3 = s30;
    fw = 0;
    fh = 0;
    check!(calib_to_panel(&mut n3, &mut fw, &mut fh, W2, H2) == CalibMove::Scaled && fw == W3 && fh == H3);
    check_near!(n3.ax, 1979.0 * w2 / w3, 1e-3);
    // Made here, recorded as the other's: the panel it was made on is this one.
    let mut h3c = s30;
    fw = W2;
    fh = H2;
    check!(calib_to_panel(&mut h3c, &mut fw, &mut fh, W3, H3) == CalibMove::MadeHere && fw == W3 && fh == H3);
    check!(h3c == s30);
    // Recorded as this panel's and spanning no Scribe's panel (a calibration far off): the record
    // stands, as it is.
    let mut loose = InputCalib { valid: true, swap_xy: false, ax: 1700.0, bx: 5.0, ay: 2300.0, by: 0.0 };
    let loose0 = loose;
    check!(to_panel(&mut loose, W2, H2, W2, H2) == CalibMove::Same && loose == loose0);
    check!(to_panel(&mut loose, 0, 0, W3, H3) == CalibMove::Scaled); // not recorded, not here: an 1860x2480 one's
    check_near!(loose.ax, 1700.0 * w3 / w2, 1e-3);
    // Spans: within 2.5% on both axes.
    let mut sp = InputCalib { valid: true, swap_xy: false, ax: 1860.0 * 1.024, bx: 0.0, ay: -2480.0 * 0.976, by: 0.0 };
    check!(calib_spans(&sp, W2, H2) && !calib_spans(&sp, W3, H3));
    sp.ax = 1860.0 * 1.026;
    check!(!calib_spans(&sp, W2, H2));
    check!(!calib_spans(&sp, 0, 0));
}

// settings.ini: pen.panel and touch.panel, the panel each calibration was made on.
fn calibration_panel_in_settings() {
    let path = cformat!("%s/settings.ini", &store_root());
    let _ = std::fs::remove_file(&path);
    let mut s = Settings::default();
    settings_load(&mut s);
    check!(s.pen_panel_w == 0 && s.pen_panel_h == 0 && s.touch_panel_w == 0 && s.touch_panel_h == 0);
    check!(settings_save(&s) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| !t.contains(".panel="))); // nothing recorded, nothing written
    s.pen_calib = InputCalib { valid: true, swap_xy: false, ax: 1986.0, bx: 0.0, ay: 2648.0, by: 0.0 };
    s.pen_panel_w = W3;
    s.pen_panel_h = H3;
    check!(settings_save(&s) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=1986x2648") == 1 && !t.contains("touch.panel=")));
    let mut b = Settings::default();
    check!(settings_load(&mut b) == 0 && b.pen_panel_w == W3 && b.pen_panel_h == H3 && b.touch_panel_w == 0);
    check!(b.pen_calib.valid && b.pen_calib.ax == 1986.0);
    b.touch_calib = InputCalib { valid: true, swap_xy: true, ax: 1860.0, bx: 0.0, ay: 2480.0, by: 0.0 };
    b.touch_panel_w = W2;
    b.touch_panel_h = H2;
    check!(settings_save(&b) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=1986x2648") == 1 && has_line(t, "touch.panel=1860x2480") == 1));
    // A round trip writes the same file.
    let mut b2 = Settings::default();
    check!(settings_load(&mut b2) == 0 && b2.touch_panel_w == W2 && b2.touch_panel_h == H2 && settings_save(&b2) == 0);
    let t2 = text_of(&path);
    check!(t.is_some() && t2.is_some() && t == t2);
    // A value written otherwise is read and goes back as it was while the panel stays; one that is
    // no panel size is a line this version does not understand: kept, the panel unrecorded.
    let odd = "pen.valid=1\npen.ax=1986\npen.ay=2648\npen.panel=1986x2648x2\ntouch.valid=1\ntouch.panel=wide\n";
    check!(write_file_atomic(&path, odd.as_bytes()) == 0);
    check!(settings_load(&mut b) == 0 && b.pen_panel_w == W3 && b.pen_panel_h == H3 && b.touch_panel_w == 0);
    check!(settings_save(&b) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=1986x2648x2") == 1 && has_line(t, "touch.panel=wide") == 1));
    b.pen_panel_w = W2; // scaled to another panel: written as it is now
    b.pen_panel_h = H2;
    check!(settings_save(&b) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=1860x2480") == 1 && !t.contains("1986x2648x2")));
    // A panel line this version cannot read stays while that calibration's panel is not recorded,
    // and goes once it is (app_init records this panel at startup): one line, the recorded one.
    let wide = "pen.valid=1\npen.ax=1986\npen.ay=2648\npen.panel=wide\ntouch.valid=1\ntouch.panel=narrow\n";
    check!(write_file_atomic(&path, wide.as_bytes()) == 0);
    check!(settings_load(&mut b) == 0 && b.pen_panel_w == 0 && b.touch_panel_w == 0);
    check!(settings_save(&b) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=wide") == 1 && has_line(t, "touch.panel=narrow") == 1));
    b.pen_panel_w = W3;
    b.pen_panel_h = H3;
    check!(settings_save(&b) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=1986x2648") == 1 && !t.contains("pen.panel=wide")));
    check!(t.as_deref().is_some_and(|t| has_line(t, "touch.panel=narrow") == 1));
    check!(settings_load(&mut b) == 0 && b.pen_panel_w == W3 && b.pen_panel_h == H3 && settings_save(&b) == 0);
    let t = text_of(&path);
    check!(t.as_deref().is_some_and(|t| has_line(t, "pen.panel=1986x2648") == 1 && !t.contains("pen.panel=wide")));
    let _ = std::fs::remove_file(&path);
}

// ---- music layouts at both heights ----

// music_max_systems as it was before 1.1, with the page taken as 2480 px at 300 dpi.
fn part_lines(p: MusicPart) -> i32 {
    if p >= PART_TAB4 && p <= PART_TAB7 {
        return 4 + (p.0 - PART_TAB4.0);
    }
    if p == PART_LYRICS {
        1
    } else {
        5
    }
}
fn old_max_systems(m: &MusicLayout) -> i32 {
    let mut su: f32 = 0.0;
    for i in 0..m.n as usize {
        let gap: f32 = if i == 0 {
            0.0
        } else if m.part[i] == PART_LYRICS || m.part[i - 1] == PART_LYRICS {
            3.5
        } else {
            4.5
        };
        su += gap + (part_lines(m.part[i]) - 1) as f32;
    }
    // (all literals: the compiler folds it, a step at a time, without fusing)
    let avail: f32 = 2480.0 / 300.0 * 25.4 - 14.0 - 12.0;
    let mut best = 1;
    for s in 1..=MUSIC_MAX_SYSTEMS {
        // two products added: the left one is fused
        if avail / fmla(s as f32, su, ((s - 1) as f32) * 6.0) >= 1.5 {
            best = s;
        }
    }
    best
}

fn music_layouts_at_both_heights() {
    // Every layout of up to five parts: on a 2480 px page the answer is what it always was, and a
    // 2648 px page fits as many or more.
    let nparts = PART_COUNT - PART_TREBLE.0; // treble staff to lyric line
    let (mut layouts, mut more, mut wrong) = (0, 0, 0);
    let mut m = MusicLayout { systems: 1, ..Default::default() };
    for n in 1..=5usize {
        let mut idx = [0i32; 5];
        loop {
            m.n = n as i32;
            for i in 0..n {
                m.part[i] = MusicPart(PART_TREBLE.0 + idx[i]);
            }
            let a = music_max_systems(&m, H2, DPI);
            let b = music_max_systems(&m, H3, DPI);
            if a != old_max_systems(&m) || b < a {
                wrong += 1;
            }
            more += (b > a) as i32;
            layouts += 1;
            let mut k = 0;
            while k < n {
                idx[k] += 1;
                if idx[k] != nparts {
                    break;
                }
                idx[k] = 0;
                k += 1;
            }
            if k == n {
                break;
            }
        }
    }
    check!(wrong == 0 && layouts > 100000 && more > 0);
    // The default (treble over 6-line tab): 6 systems on a Scribe 2 page, 7 on a Colorsoft page.
    let mut d = MusicLayout::default();
    check!(music_decode(music_default(), &mut d));
    check!(music_max_systems(&d, H2, DPI) == 6 && music_max_systems(&d, H3, DPI) == 7);
    // Rendering stays a pure function of the page: any region of a 1986x2648 or an 1860x2480 page
    // renders as the same region of the whole page does.
    let mut big = MusicLayout { systems: 7, n: 2, ..Default::default() };
    big.part[0] = PART_TREBLE;
    big.part[1] = PART_TAB6;
    let kinds = [PAPER_MUSIC, PAPER_GRAND, PAPER_TAB, PAPER_BASS5, music_encode(&big), music_default()];
    let sizes = [[W3, H3], [W2, H2]];
    for z in 0..2 {
        for k in 0..kinds.len() {
            let mut full = Canvas::default();
            let mut part = Canvas::default();
            if canvas_init(&mut full, sizes[z][0], sizes[z][1]) != 0 || canvas_init(&mut part, sizes[z][0], sizes[z][1]) != 0 {
                check!(false);
                return;
            }
            let (fw, fh) = (full.w, full.h);
            paper_render(&mut full, kinds[k], Rect { x: 0, y: 0, w: fw, h: fh }, DPI);
            let bands = [Rect { x: 0, y: 0, w: fw, h: 700 }, Rect { x: 0, y: 700, w: fw, h: 1300 }, Rect { x: 0, y: 2000, w: fw, h: fh - 2000 }];
            for i in 0..3 {
                paper_render(&mut part, kinds[k], bands[i], DPI);
            }
            check!(same_pixels(&full, &part));
            canvas_free(&mut full);
            canvas_free(&mut part);
        }
    }
}

// ---- the colour panel's wait ----

// Which fast refreshes are live ink (display.h): on a colour panel the Kindle waits for the
// driver's read after every other fast refresh, so pen reading stalls for none of a stroke's.
fn live_ink_frames() {
    let mut l = LiveInk::default();
    check!(!live_ink_frame(&mut l, RefreshMode::Fast, 1000)); // a pressed button, a dragged panel: waits
    live_ink_mark(&mut l, true, 1000); // a stroke starts
    check!(live_ink_frame(&mut l, RefreshMode::Fast, 1010) && live_ink_frame(&mut l, RefreshMode::Fast, 5000));
    check!(!live_ink_frame(&mut l, RefreshMode::Ui, 5010) && !live_ink_frame(&mut l, RefreshMode::Full, 5020)); // only fast ones
    live_ink_mark(&mut l, false, 6000); // the pen lifts
    check!(!live_ink_frame(&mut l, RefreshMode::Ui, 6010)); // (a grayscale refresh in between is not one)
    check!(live_ink_frame(&mut l, RefreshMode::Fast, 6050)); // the samples that came with the lifting
    check!(!live_ink_frame(&mut l, RefreshMode::Fast, 6060)); // and no more
    live_ink_mark(&mut l, true, 7000);
    live_ink_mark(&mut l, false, 7010);
    check!(!live_ink_frame(&mut l, RefreshMode::Fast, 7120)); // 100 ms or more after the end: not
    live_ink_mark(&mut l, false, 8000); // ended again (leaving a screen): nothing ends
    check!(!live_ink_frame(&mut l, RefreshMode::Fast, 8001));
    // An end at the clock's wrap, and at 0 ms (never taken for no end at all).
    live_ink_mark(&mut l, true, 0xFFFFFFF0);
    live_ink_mark(&mut l, false, 0xFFFFFFF0);
    check!(live_ink_frame(&mut l, RefreshMode::Fast, 0x20));
    live_ink_mark(&mut l, true, 0);
    live_ink_mark(&mut l, false, 0);
    check!(live_ink_frame(&mut l, RefreshMode::Fast, 5));
}

// The calibration, music and live ink tests touch no file: they run as they are.
#[test]
fn test_pagesize() {
    run!(calibration_scaling_math);
    run!(music_layouts_at_both_heights);
    run!(live_ink_frames);
    done();
}

// The C's other tests, in its order (calibration_scaling_math and the rest above are independent
// of them); they read and write notebooks, so they run once the store, pages and notebooks are
// translated (store.rs and page.rs, wave 2): cargo test --test test_pagesize -- --ignored
#[test]
#[ignore = "needs store.rs and page.rs (wave 2): notebooks, pages, settings and the page renders"]
fn test_pagesize_notebooks() {
    let r = root();
    let _ = std::fs::remove_dir_all(&r);
    check!(store_init(&r) == 0);
    run!(notebook_keeps_its_size);
    run!(rescaled_pages_come_back);
    run!(size_found_without_the_keys);
    run!(most_pages_give_the_size);
    run!(listing_records_a_found_size);
    run!(calibration_panel_in_settings);
    let _ = std::fs::remove_dir_all(&r);
    done();
}
