// test_tplsheet.rs: the Templates sheet (src/tplsheet.rs): the paper pickers' grid, its pages of
// tiles and what stays in view through a turn, picking, Back and a tap outside, tiles that wait for
// their previews, and which tile is the page's paper. The port of tests/test_tplsheet.c: the same
// tests, in the C's order, with the same checks.
#![allow(clippy::needless_range_loop)] // (the C's index loops, kept)
mod testlib;
use grimoire::prelude::*;
use std::ffi::CString;
use std::sync::OnceLock;
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
fn root() -> String {
    ROOT.get().cloned().unwrap_or_default()
}
const DPI: f32 = 300.0;

// ---- the system, as the C test calls it ----

fn system(cmd: &str) -> i32 {
    let c = CString::new(cmd).unwrap();
    // SAFETY: system reads the NUL-terminated command we own.
    unsafe { libc::system(c.as_ptr()) }
}

fn usleep(us: u64) {
    std::thread::sleep(std::time::Duration::from_micros(us));
}

// ---- helpers ----

fn drain_jobs() -> i32 {
    let mut last = -1;
    let until = now_ms().wrapping_add(60000);
    while bg_busy() && (now_ms().wrapping_sub(until) as i32) < 0 {
        let mut kind = -1;
        bg_poll_kind(Some(&mut kind));
        if kind >= 0 {
            last = kind;
        }
        if kind == BgKind::Template as i32 {
            bgimage_retry_missing(); // (as app.c does when one ends)
        }
        usleep(5000);
    }
    let mut kind = -1;
    bg_poll_kind(Some(&mut kind));
    if kind == BgKind::Template as i32 {
        bgimage_retry_missing();
    }
    if kind >= 0 {
        kind
    } else {
        last
    }
}

// Template n: a gray picture of n lines, distinct for every n.
fn put_template(n: i32, name: &str) {
    let mut p = BgPicture { px: vec![0; 60 * 80], w: 60, h: 80, ch: 1 };
    p.px.fill(255);
    let mut y = 5;
    while y < 80 {
        p.px[y as usize * 60..y as usize * 60 + 60].fill((100 + n % 50) as u8);
        y += 3 + n % 11;
    }
    p.px[(n % 60) as usize] = 0;
    let path = format!("{}/templates/{}", root(), name);
    check!(bgimage_write_png(&path, &p) == 0);
    bgpicture_free(&mut p);
}

fn screen(a: &mut App, w: i32, h: i32) {
    canvas_free(&mut a.screen);
    a.w = w;
    a.h = h;
    check!(canvas_init(&mut a.screen, w, h) == 0);
}

fn new_app() -> App {
    App { dpi: DPI, ..Default::default() }
}

fn find_hit(s: &TplSheet, id: i32, arg: i32) -> Option<Hit> {
    for i in 0..s.hits.h.len() {
        if s.hits.h[i].id == id && (id != TS_HIT_TILE || s.hits.h[i].arg == arg) {
            return Some(s.hits.h[i]);
        }
    }
    None
}

// A tap at the middle of r (down, then up), as the hosts pass them on.
fn tap(s: &mut TplSheet, a: &mut App, r: Rect, dirty: &mut Rect) -> TsResult {
    let x = (r.x + r.w / 2) as f32;
    let y = (r.y + r.h / 2) as f32;
    tplsheet_pointer(s, a, 0, x, y, 1000, dirty);
    tplsheet_pointer(s, a, 2, x, y, 1080, dirty)
}

fn probe(s: &TplSheet, key: &str) -> String {
    let mut out = String::new();
    check!(tplsheet_probe(s, key, &mut out, 64) == 0);
    out
}

// ---- the grid ----

fn paper_grid_layouts() {
    let mut a = new_app();
    let mut g = PaperGrid::default();
    // The editor's Paper sheet (and the Templates sheet in its place): upright the papers keep their
    // 6 columns, More papers... a row of its own; sideways 7 columns keep them as big as before.
    a.w = 1860;
    a.h = 2480;
    let mut r = paper_sheet_rect(&a, &mut g);
    check!(g.cols == 6 && g.rows == 4 && g.tw == 249 && g.th == 332);
    check!(r.y >= 0 && r.y + r.h <= a.h && r.x >= 0 && r.x + r.w <= a.w);
    a.w = 2480;
    a.h = 1860;
    r = paper_sheet_rect(&a, &mut g);
    check!(g.cols == 7 && g.rows == 3 && g.tw == 274 && g.th == 365);
    check!(r.y >= 0 && r.y + r.h <= a.h);
    // New notebook's tiles in the room draw_new gives them (Gothic and Classic): upright 6 columns,
    // sideways 7, at the size 6 had before More papers... came (its labels' room stays 222 px).
    struct Nb {
        w: i32,
        h: i32,
        cols: i32,
        rows: i32,
        tw: i32,
    }
    let nb = [
        Nb { w: 1640, h: 1590, cols: 6, rows: 4, tw: 222 },
        Nb { w: 1648, h: 1608, cols: 6, rows: 4, tw: 226 },
        Nb { w: 1720, h: 1070, cols: 7, rows: 3, tw: 192 },
        Nb { w: 1728, h: 1088, cols: 7, rows: 3, tw: 196 },
    ];
    for i in 0..nb.len() {
        g = paper_grid(PAPER_SHEET_TILES, nb[i].w, nb[i].h, 30, 100, 300, PAPER_GRID_MIN_COLS);
        check!(g.cols == nb[i].cols && g.rows == nb[i].rows && g.tw == nb[i].tw);
    }
    // Any box: the tiles fit it, and no other number of columns (at least the minimum) makes them
    // bigger; of those as big, none leaves fewer places empty.
    let mut rng: u32 = 7;
    for t in 0..400 {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let n = 1 + ((rng >> 16) as i32) % 40;
        let aw = 300 + ((rng >> 8) as i32) % 2500;
        let ah = 300 + ((rng >> 4) as i32) % 2500;
        let min = 1 + t % 7;
        g = paper_grid(n, aw, ah, 30, 100, 300, min);
        check!(g.cols >= 1 && g.rows >= 1 && g.cols * g.rows >= n && g.tw >= 1 && g.tw <= 300);
        check!(g.cols >= (if min < n { min } else { n }));
        if g.tw > 1 {
            check!(g.cols * g.tw + (g.cols - 1) * 30 <= aw && g.rows * g.cell_h <= ah);
            let mut c = if min < n { min } else { n };
            while c <= n {
                let o = paper_grid(n, aw, ah, 30, 100, 300, c);
                if o.cols != c {
                    c += 1;
                    continue;
                }
                check!(o.tw <= g.tw);
                if o.tw == g.tw {
                    check!(o.rows * o.cols - n >= g.rows * g.cols - n);
                }
                c += 1;
            }
        }
    }
}

// ---- the sheet ----

fn pages_of_tiles() {
    templates_init();
    for n in 1..=50 {
        let name = format!("page-{}.png", n);
        put_template(n, &name);
    }
    let mut path = format!("{}/templates/zz-broken.png", root());
    check!(write_file_atomic(&path, b"no picture") == 0);
    let mut a = new_app();
    screen(&mut a, 1860, 2480);
    let mut s = TplSheet::default();
    // Opened before any preview is made: every tile waits (none can be picked), and none is read.
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    check!(s.open && s.t.len() == 51 && s.shown.len() == 51 && s.pending); // (the broken file is not known bad yet)
    tplsheet_draw(&mut s, &mut a);
    let mut v = probe(&s, "count");
    check!(v == "51");
    v = probe(&s, "ready");
    check!(v == "0");
    check!(find_hit(&s, TS_HIT_TILE, 0).is_none() && find_hit(&s, TS_HIT_BACK, 0).is_some() && find_hit(&s, TS_HIT_NEXT, 0).is_some());
    // The previews job ends: the tiles fill in, in natural order; the broken file is offered no more
    // but keeps its tile (the last) while the sheet is open.
    check!(drain_jobs() == BgKind::Template as i32);
    let mut d = Rect::default();
    check!(tplsheet_tick(&mut s, &mut a, now_ms(), &mut d) && !rect_is_empty(d));
    check!(
        s.shown.len() == 51
            && s.t[s.shown[50] as usize].bad
            && !s.pending
            && s.t[s.shown[1] as usize].name == "page-2"
            && s.t[s.shown[9] as usize].name == "page-10"
    );
    v = probe(&s, "count");
    check!(v == "50");
    v = probe(&s, "ready");
    check!(v == "50");
    v = probe(&s, "pages");
    check!(v == "3"); // 24 a page upright
    check!(find_hit(&s, TS_HIT_TILE, 0).is_some() && find_hit(&s, TS_HIT_TILE, 23).is_some() && find_hit(&s, TS_HIT_TILE, 24).is_none());
    check!(find_hit(&s, TS_HIT_PREV, 0).is_none());
    check!(!s.t[s.shown[0] as usize].thumb.px.is_empty() && s.t[s.shown[30] as usize].thumb.px.is_empty()); // only the tiles on screen are read
    // The next page; a turn keeps its first tile in view (sideways 21 a page: page 2 of 3 holds 24).
    let mut h = find_hit(&s, TS_HIT_NEXT, 0);
    check!(h.is_some() && tap(&mut s, &mut a, h.unwrap().r, &mut d) == TsResult::Changed);
    v = probe(&s, "page");
    check!(v == "2" && find_hit(&s, TS_HIT_TILE, 24).is_some() && find_hit(&s, TS_HIT_PREV, 0).is_some());
    check!(s.t[s.shown[0] as usize].thumb.px.is_empty() && !s.t[s.shown[24] as usize].thumb.px.is_empty()); // the page before let go of
    screen(&mut a, 2480, 1860);
    tplsheet_draw(&mut s, &mut a);
    v = probe(&s, "page");
    check!(v == "2" && s.per == 21 && find_hit(&s, TS_HIT_TILE, 21).is_some() && find_hit(&s, TS_HIT_TILE, 24).is_some());
    h = find_hit(&s, TS_HIT_NEXT, 0);
    check!(h.is_some() && tap(&mut s, &mut a, h.unwrap().r, &mut d) == TsResult::Changed);
    v = probe(&s, "page");
    check!(v == "3" && find_hit(&s, TS_HIT_TILE, 49).is_some() && find_hit(&s, TS_HIT_NEXT, 0).is_none());
    // Back upright: the page that holds the first tile shown sideways (42: page 2).
    screen(&mut a, 1860, 2480);
    tplsheet_draw(&mut s, &mut a);
    v = probe(&s, "page");
    check!(v == "2" && find_hit(&s, TS_HIT_TILE, 42).is_some());
    // Picking a tile: its file and preview, until tplsheet_close.
    h = find_hit(&s, TS_HIT_TILE, 30);
    check!(h.is_some() && tap(&mut s, &mut a, h.unwrap().r, &mut d) == TsResult::Picked && !s.open);
    path = format!("{}/templates/page-31.png", root());
    check!(!tplsheet_path(&s).is_empty() && tplsheet_path(&s) == path && tplsheet_preview(&mut s).is_some());
    v = probe(&s, "count");
    check!(v.is_empty()); // (closed)
    tplsheet_close(&mut s);
    check!(tplsheet_path(&s).is_empty() && s.t.is_empty() && s.shown.is_empty());
    // Back, and a tap outside the sheet.
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    check!(s.shown.len() == 50 && !s.pending && !bg_busy()); // (all made: no job queued again)
    h = find_hit(&s, TS_HIT_BACK, 0);
    check!(h.is_some() && tap(&mut s, &mut a, h.unwrap().r, &mut d) == TsResult::Back && !s.open && s.t.is_empty());
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    let out = Rect { x: s.rect.x + s.rect.w / 2, y: s.rect.y + s.rect.h + 20, w: 4, h: 4 };
    check!(tap(&mut s, &mut a, out, &mut d) == TsResult::Cancel && !s.open && rect_overlaps(d, s.rect));
    // In New notebook the template chosen before is selected, on its page.
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, Some(&path));
    tplsheet_draw(&mut s, &mut a);
    v = probe(&s, "selected");
    check!(v == "page-31");
    v = probe(&s, "page");
    check!(v == "2");
    tplsheet_close(&mut s);
    canvas_free(&mut a.screen);
}

// A template copied in after the previews were made waits for its own, then fills in; the previews
// job's other tiles stay as they are meanwhile.
fn tile_fills_in() {
    let mut a = new_app();
    screen(&mut a, 1860, 2480);
    put_template(77, "page-0.png"); // first in natural order
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    check!(s.pending && !s.t[s.shown[0] as usize].ready && s.t[s.shown[1] as usize].ready);
    check!(find_hit(&s, TS_HIT_TILE, 0).is_none() && find_hit(&s, TS_HIT_TILE, 1).is_some());
    let mut d = Rect::default();
    check!(!tplsheet_tick(&mut s, &mut a, now_ms(), &mut d) || s.pending); // (made only once the job has run)
    drain_jobs();
    check!(tplsheet_tick(&mut s, &mut a, now_ms(), &mut d) && !s.pending && find_hit(&s, TS_HIT_TILE, 0).is_some());
    check!(!tplsheet_tick(&mut s, &mut a, now_ms().wrapping_add(5000), &mut d)); // nothing more to wait for
    tplsheet_close(&mut s);
    canvas_free(&mut a.screen);
}

// The tile that is the open page's paper: the template it was installed from, by the notebook's
// index of its pictures, on its page of tiles. Never another notebook's, nor a document page.
fn page_paper_selected() {
    let mut a = new_app();
    screen(&mut a, 1860, 2480);
    let (mut nb, mut other) = (NotebookInfo::default(), NotebookInfo::default());
    check!(store_create("Planner", "", PAPER_MEDIUM, &mut nb) == 0 && store_create("Other", "", PAPER_MEDIUM, &mut other) == 0);
    let mut path = format!("{}/templates/page-40.png", root());
    let (mut k, mut k2) = (PaperKind::default(), PaperKind::default());
    let mut why: &'static str = "";
    check!(template_install(&nb, &path, &mut k, &mut why) == 0 && paper_is_image(k));
    path = format!("{}/templates/page-3.png", root());
    check!(template_install(&other, &path, &mut k2, &mut why) == 0);
    drain_jobs();
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, Some(&nb), k, None);
    tplsheet_draw(&mut s, &mut a);
    let mut v = probe(&s, "selected");
    check!(v == "page-40");
    v = probe(&s, "page");
    check!(v == "2"); // (page-0, page-1 ... page-40 is the 41st)
    tplsheet_close(&mut s);
    let not_here = [k2, paper_image(PAPER_ROLE_DOCUMENT, paper_image_id(k)), PAPER_MEDIUM];
    for i in 0..3 {
        tplsheet_open(&mut s, &mut a, Some(&nb), not_here[i], None);
        tplsheet_draw(&mut s, &mut a);
        v = probe(&s, "selected");
        check!(v.is_empty() && s.page == 0);
        tplsheet_close(&mut s);
    }
    canvas_free(&mut a.screen);
}

// No templates at all (and no folder): the sheet says how to add some, with only Back to tap.
fn empty_sheet() {
    let cmd = format!("rm -rf '{}/templates'", root());
    check!(system(&cmd) == 0);
    let mut a = new_app();
    screen(&mut a, 2480, 1860);
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    let mut v = probe(&s, "count");
    check!(s.open && v == "0" && s.hits.h.len() == 1 && s.hits.h[0].id == TS_HIT_BACK && !bg_busy());
    v = probe(&s, "pages");
    check!(v == "1");
    tplsheet_close(&mut s);
    templates_init(); // (the folder again, for whatever runs after)
    canvas_free(&mut a.screen);
}

// A file the previews job finds it cannot use, sorted before the page shown: the tiles on that page
// stay where they were (it keeps its tile, crossed out, until the sheet opens again).
fn bad_file_keeps_its_place() {
    let cmd = format!("rm -rf '{}/templates'", root());
    check!(system(&cmd) == 0);
    templates_init();
    let path = format!("{}/templates/0 broken.png", root());
    check!(write_file_atomic(&path, b"no picture") == 0);
    for n in 1..=40 {
        let name = format!("t{}.png", n);
        put_template(n, &name);
    }
    let mut a = new_app();
    screen(&mut a, 1860, 2480);
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    let mut d = Rect::default();
    let mut h = find_hit(&s, TS_HIT_NEXT, 0);
    check!(h.is_some() && tap(&mut s, &mut a, h.unwrap().r, &mut d) == TsResult::Changed);
    let before = s.t[s.shown[24] as usize].name.as_str().to_string(); // page 2's first tile
    check!(before == "t24");
    check!(drain_jobs() == BgKind::Template as i32);
    check!(tplsheet_tick(&mut s, &mut a, now_ms(), &mut d));
    let mut v = probe(&s, "page");
    check!(v == "2" && s.t[s.shown[24] as usize].name == before.as_str() && find_hit(&s, TS_HIT_TILE, 24).is_some());
    v = probe(&s, "count");
    check!(v == "40" && s.shown.len() == 41 && s.t[s.shown[0] as usize].bad && !s.pending);
    // Back on page 1: the file's tile, which cannot be picked; opened again, it is gone.
    h = find_hit(&s, TS_HIT_PREV, 0);
    check!(h.is_some() && tap(&mut s, &mut a, h.unwrap().r, &mut d) == TsResult::Changed);
    check!(find_hit(&s, TS_HIT_TILE, 0).is_none() && find_hit(&s, TS_HIT_TILE, 1).is_some());
    tplsheet_close(&mut s);
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    check!(s.shown.len() == 40 && s.t[s.shown[0] as usize].name == "t1" && find_hit(&s, TS_HIT_TILE, 0).is_some() && !bg_busy());
    tplsheet_close(&mut s);
    canvas_free(&mut a.screen);
}

fn count_previews() -> i32 {
    let dir = format!("{}/templates/.cache", root());
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            n += (name.len() > 4 && name.ends_with(".pgm")) as i32;
        }
    }
    n
}

// While the previews job runs (here: its child has not been reaped, so it still counts as running),
// the sheet looks for the previews made so far every 1.5 s, and fills their tiles in before the job
// is over; not sooner.
fn fills_in_while_the_job_runs() {
    let mut a = new_app();
    screen(&mut a, 1860, 2480);
    put_template(88, "t0.png"); // first in natural order, no preview yet
    let had = count_previews();
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    tplsheet_draw(&mut s, &mut a);
    check!(bg_busy() && s.pending && !s.t[s.shown[0] as usize].ready && find_hit(&s, TS_HIT_TILE, 0).is_none());
    let until = now_ms().wrapping_add(30000);
    while count_previews() == had && (now_ms().wrapping_sub(until) as i32) < 0 {
        usleep(5000);
    }
    check!(count_previews() == had + 1);
    let mut d = Rect::default();
    let gen = bgimage_generation();
    let next = s.next_check;
    check!(!tplsheet_tick(&mut s, &mut a, next.wrapping_sub(1), &mut d) && !s.t[s.shown[0] as usize].ready);
    check!(tplsheet_tick(&mut s, &mut a, next, &mut d) && s.t[s.shown[0] as usize].ready && !s.pending);
    check!(find_hit(&s, TS_HIT_TILE, 0).is_some() && bg_busy() && bgimage_generation() == gen);
    drain_jobs();
    tplsheet_close(&mut s);
    canvas_free(&mut a.screen);
}

// The columns that hold ink (a pixel under 128) in r of c: *x0 the first, *x1 the last (-1, -1
// for none).
fn ink_columns(c: &Canvas, r: Rect, x0: &mut i32, x1: &mut i32) {
    *x0 = -1;
    *x1 = -1;
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            if c.px[y as usize * c.stride as usize + x as usize] < 128 {
                if *x0 < 0 || x < *x0 {
                    *x0 = x;
                }
                if x > *x1 {
                    *x1 = x;
                }
            }
        }
    }
}

// UI-11: a paper tile's label never reaches past its tile (one wider than the tile goes on two
// lines under it), so labels side by side never run into each other; a selected one, in bold,
// stays within its column. In every language and style, in each picker's grid upright and
// sideways.
fn labels_keep_to_their_tiles() {
    let mut a = new_app();
    let mut g = [PaperGrid::default(); 6];
    a.w = 1860;
    a.h = 2480;
    paper_sheet_rect(&a, &mut g[0]);
    a.w = 2480;
    a.h = 1860;
    paper_sheet_rect(&a, &mut g[1]);
    const BOX: [[i32; 2]; 4] = [[1640, 1590], [1648, 1608], [1720, 1070], [1728, 1088]]; // New notebook's
    for i in 0..4 {
        g[2 + i] = paper_grid(PAPER_SHEET_TILES, BOX[i][0], BOX[i][1], 30, 100, 300, PAPER_GRID_MIN_COLS);
    }
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 700, 600) == 0);
    let gap = ui_px(30.0);
    let mut wrapped = 0;
    for l in 0..i18n_language_count() {
        check!(i18n_set(i18n_language_code(l).unwrap()));
        for th in 0..2 {
            ui_set_theme(if th != 0 { UiTheme::Classic } else { UiTheme::Gothic });
            for k in 0..6 {
                let tr_ = Rect { x: 200, y: 20, w: g[k].tw, h: g[k].th };
                for i in 0..PAPER_SHEET_TILES {
                    let label = if i < PAPER_COUNT {
                        paper_label(paper_at(i))
                    } else if i == PAPER_COUNT {
                        tr("Custom music...")
                    } else {
                        tr("More papers...")
                    };
                    for sel in 0..2 {
                        let all = Rect { x: 0, y: 0, w: c.w, h: c.h };
                        canvas_fill(&mut c, all, 255);
                        paper_tile_label(&mut c, tr_, gap, label, sel != 0);
                        let (mut x0, mut x1) = (0, 0);
                        let room = if sel != 0 { gap / 2 } else { ui_px(2.0) };
                        ink_columns(&c, Rect { x: 0, y: tr_.y + tr_.h, w: c.w, h: c.h - tr_.y - tr_.h }, &mut x0, &mut x1);
                        let in_ = x0 >= tr_.x - room && x1 < tr_.x + tr_.w + room;
                        if !in_ {
                            eprintln!(
                                "  ({}, {}, a {} px tile{}: \"{}\" inked from {} to {})",
                                i18n_language_code(l).unwrap(),
                                if th != 0 { "classic" } else { "gothic" },
                                tr_.w,
                                if sel != 0 { ", bold" } else { "" },
                                label,
                                x0 - tr_.x,
                                x1 - tr_.x
                            );
                        }
                        check!(x0 >= 0 && in_);
                        wrapped += (sel == 0 && font_measure(FontFace::Regular, ui_small_size() * 0.9, label) > tr_.w) as i32;
                    }
                }
            }
        }
    }
    check!(wrapped > 0); // (some labels are wider than their tiles: the test sees two lines)
    i18n_set("en");
    ui_set_theme(UiTheme::Gothic);
    canvas_free(&mut c);
}

// The empty state with files it cannot use: its heading, then three paragraphs (how to add
// templates, what works best, how many files it cannot use), each starting as far under the one
// before it whatever their numbers of lines, never as close as a line of it, nor far under the
// heading. In every language and style, upright and sideways.
fn empty_state_paragraphs() {
    let cmd = format!("rm -rf '{}/templates'", root());
    check!(system(&cmd) == 0);
    templates_init();
    let mut path = format!("{}/templates/Scan.png", root());
    check!(write_file_atomic(&path, b"no picture") == 0);
    path = format!("{}/templates/Photo.jpg", root());
    check!(write_file_atomic(&path, b"no picture either") == 0);
    let mut a = new_app();
    screen(&mut a, 1860, 2480);
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
    drain_jobs();
    tplsheet_close(&mut s);
    for l in 0..i18n_language_count() {
        check!(i18n_set(i18n_language_code(l).unwrap()));
        for th in 0..2 {
            ui_set_theme(if th != 0 { UiTheme::Classic } else { UiTheme::Gothic });
            for turn in 0..2 {
                screen(&mut a, if turn != 0 { 2480 } else { 1860 }, if turn != 0 { 1860 } else { 2480 });
                canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w: a.w, h: a.h }, 255);
                tplsheet_open(&mut s, &mut a, None, PAPER_BLANK, None);
                tplsheet_draw(&mut s, &mut a);
                let v = probe(&s, "count");
                check!(v == "0" && s.t.len() == 2 && s.hits.h.len() == 1);
                // The text's column: the sheet's inside (ui_sheet says where), above Back.
                let mut scratch = Canvas::default();
                check!(canvas_init(&mut scratch, a.w, a.h) == 0);
                let in_ = ui_sheet(&mut scratch, s.rect, tr("Templates"));
                canvas_free(&mut scratch);
                let col = Rect { x: in_.x + ui_px(40.0), y: in_.y, w: in_.w - 2 * ui_px(40.0), h: in_.h - ui_px(160.0) };
                // Its bands of rows with ink (a few blank rows inside one, as under an accent,
                // do not end it), and the blank between each and the next.
                let mut gaps = [0i32; 64];
                let (mut ng, mut last, mut bands) = (0usize, -1, 0);
                for y in col.y..col.y + col.h {
                    let (mut x0, mut x1) = (0, 0);
                    ink_columns(&a.screen, Rect { x: col.x, y, w: col.w, h: 1 }, &mut x0, &mut x1);
                    if x0 < 0 {
                        continue;
                    }
                    if last < 0 || y - last > ui_px(6.0) {
                        if last >= 0 && ng < 64 {
                            gaps[ng] = y - last - 1;
                            ng += 1;
                        }
                        bands += 1;
                    }
                    last = y;
                }
                let (mut wide, mut widest) = (0, 0);
                for i in 0..ng {
                    wide += (gaps[i] >= ui_px(45.0)) as i32; // (40 between the paragraphs, and the lines' own room)
                    if gaps[i] > widest {
                        widest = gaps[i];
                    }
                }
                if wide != 3 || widest > ui_px(120.0) {
                    let mut msg = format!(
                        "  ({}, {}, {}: {} bands, gaps",
                        i18n_language_code(l).unwrap(),
                        if th != 0 { "classic" } else { "gothic" },
                        if turn != 0 { "sideways" } else { "upright" },
                        bands
                    );
                    for i in 0..ng {
                        msg.push_str(&format!(" {}", gaps[i]));
                    }
                    eprintln!("{})", msg);
                }
                check!(wide == 3 && widest <= ui_px(120.0));
                tplsheet_close(&mut s);
            }
        }
    }
    i18n_set("en");
    ui_set_theme(UiTheme::Gothic);
    canvas_free(&mut a.screen);
}

// Everything, in the C's order (the tests share the store and the folders the ones before them made).
#[test]
fn test_tplsheet() {
    let top = format!("{}/tplsheet-{}", tl_tmp(), std::process::id());
    let _ = ROOT.set(top.clone());
    let _ = std::fs::remove_dir_all(&top);
    check!(store_init(&top) == 0);
    store_set_device(DPI, 1860, 2480);
    ui_init(DPI);
    check!(font_init() == 0);
    // (a press on a sheet now shows at once, app_press_feedback: the display is open)
    #[cfg(feature = "sim")]
    std::env::set_var("GRIMOIRE_SCRIPT", "/dev/null");
    let (mut w, mut h, mut dpi) = (0, 0, 0.0f32);
    let _ = display_open(&mut w, &mut h, &mut dpi);
    run!(paper_grid_layouts);
    run!(pages_of_tiles);
    run!(tile_fills_in);
    run!(page_paper_selected);
    run!(empty_sheet);
    run!(bad_file_keeps_its_place);
    run!(fills_in_while_the_job_runs);
    run!(labels_keep_to_their_tiles);
    run!(empty_state_paragraphs);
    font_shutdown();
    done();
    let _ = std::fs::remove_dir_all(&top);
}
