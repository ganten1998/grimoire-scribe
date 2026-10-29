// Not a port of a C test: the screens of musicbuild.rs, tplsheet.rs, pagesview.rs and pensettings.rs
// against the C build, as test_page_probe.rs checks page.rs. tests/fixtures/sheets_probe.c is a C
// program (it links ../src/musicbuild.c, tplsheet.c, pagesview.c, pensettings.c and what they need,
// built with clang -O1 -ffp-contract=on, the Makefile's test flags, app.c's calls stubbed) that draws
// each screen with fixed inputs and prints one hash of the pixels, with the layouts, results and
// probe answers beside them. fixtures/sheets_probe_c.txt is what it printed. This test does the same
// in the same order with integer arithmetic and exact float conversions, so the inputs are the same,
// and compares section by section. A mismatch of one pixel changes a hash: the layouts, the
// translations' fit, the fonts' placement, the pressure curve and readout, the brush on the test pad
// and the slider drags are held to the C. (How to record it again is at the top of sheets_probe.c.)
//
// The taps and the pen on the Pages view and Pen feel go through app.rs's real app_invalidate,
// app_toast, app_keyboard_pointer and app_pointer_phase (the C probe stubs them); those two sections
// run in test_sheets_probe_events, with the panel open as the app has it.
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)] // (the C probe's index loops and its pen helper, kept)
mod testlib;
use grimoire::prelude::*;
use std::ffi::CString;
use testlib::*;

const H0: u64 = 14695981039346656037;

fn fnv(mut h: u64, p: &[u8]) -> u64 {
    for &b in p {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}
fn fnv_u32(h: u64, v: u32) -> u64 {
    fnv(h, &v.to_le_bytes())
}
fn canvas_hash(c: &Canvas) -> u64 {
    let mut h = H0;
    h = fnv_u32(h, c.w as u32);
    h = fnv_u32(h, c.h as u32);
    for y in 0..c.h as usize {
        h = fnv(h, &c.px[y * c.stride as usize..y * c.stride as usize + c.w as usize]);
    }
    h
}
// (the C folds a hash into another as its two halves)
fn fold(h: u64, c: &Canvas) -> u64 {
    let x = canvas_hash(c);
    fnv_u32(fnv_u32(h, x as u32), (x >> 32) as u32)
}

fn system(cmd: &str) -> i32 {
    let c = CString::new(cmd).unwrap();
    // SAFETY: system reads the NUL-terminated command we own.
    unsafe { libc::system(c.as_ptr()) }
}
fn usleep(us: u64) {
    std::thread::sleep(std::time::Duration::from_micros(us));
}

struct Probe {
    root: String,
    app: App,
    out: String,
    nb: NotebookInfo,
    nav: Nav,
    many: Nav,
}

impl Probe {
    fn new(root: &str) -> Probe {
        Probe { root: root.to_string(), app: App { dpi: 300.0, ..Default::default() }, out: String::new(), nb: NotebookInfo::default(), nav: Nav::default(), many: Nav::default() }
    }
    fn emit(&mut self, label: &str) {
        let line = format!("{} {:016x}\n", label, canvas_hash(&self.app.screen));
        self.out.push_str(&line);
    }
    fn screen(&mut self, w: i32, h: i32) {
        canvas_free(&mut self.app.screen);
        self.app.w = w;
        self.app.h = h;
        assert_eq!(canvas_init(&mut self.app.screen, w, h), 0);
    }
    fn clear_screen(&mut self) {
        let r = Rect { x: 0, y: 0, w: self.app.w, h: self.app.h };
        canvas_fill(&mut self.app.screen, r, 255);
    }
}

fn tag(turn: i32, th: i32) -> &'static str {
    if turn != 0 {
        if th != 0 {
            "lc"
        } else {
            "lg"
        }
    } else if th != 0 {
        "pc"
    } else {
        "pg"
    }
}
fn look(th: i32) {
    ui_set_theme(if th != 0 { UiTheme::Classic } else { UiTheme::Gothic });
}
fn dims(turn: i32) -> (i32, i32) {
    if turn != 0 {
        (2480, 1860)
    } else {
        (1860, 2480)
    }
}

fn drain_jobs() -> i32 {
    let until = now_ms().wrapping_add(60000);
    while bg_busy() && (now_ms().wrapping_sub(until) as i32) < 0 {
        let mut kind = -1;
        bg_poll_kind(Some(&mut kind));
        if kind == BgKind::Template as i32 {
            bgimage_retry_missing();
        }
        usleep(5000);
    }
    let mut kind = -1;
    bg_poll_kind(Some(&mut kind));
    if kind == BgKind::Template as i32 {
        bgimage_retry_missing();
    }
    kind
}

fn put_template(root: &str, n: i32, name: &str) {
    let mut p = BgPicture { px: vec![255; 60 * 80], w: 60, h: 80, ch: 1 };
    let mut y = 5;
    while y < 80 {
        p.px[y as usize * 60..y as usize * 60 + 60].fill((100 + n % 50) as u8);
        y += 3 + n % 11;
    }
    p.px[(n % 60) as usize] = 0;
    let path = format!("{}/templates/{}", root, name);
    assert_eq!(bgimage_write_png(&path, &p), 0);
    bgpicture_free(&mut p);
}

// ---- Templates ----

fn labels(p: &mut Probe) {
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, 700, 600), 0);
    let mut g = [PaperGrid::default(); 6];
    let mut a = App { dpi: 300.0, ..Default::default() };
    a.w = 1860;
    a.h = 2480;
    paper_sheet_rect(&a, &mut g[0]);
    a.w = 2480;
    a.h = 1860;
    paper_sheet_rect(&a, &mut g[1]);
    const BOX: [[i32; 2]; 4] = [[1640, 1590], [1648, 1608], [1720, 1070], [1728, 1088]];
    for i in 0..4 {
        g[2 + i] = paper_grid(PAPER_SHEET_TILES, BOX[i][0], BOX[i][1], 30, 100, 300, PAPER_GRID_MIN_COLS);
    }
    for k in 0..6 {
        p.out.push_str(&cformat!("grid %d cols %d rows %d tw %d th %d cell %d\n", k as i32, g[k].cols, g[k].rows, g[k].tw, g[k].th, g[k].cell_h));
    }
    let gap = ui_px(30.0);
    for l in 0..i18n_language_count() {
        let code = i18n_language_code(l).unwrap();
        i18n_set(code);
        for th in 0..2 {
            look(th);
            let mut h = H0;
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
                        let x = canvas_hash(&c);
                        h = fnv_u32(h, x as u32);
                        h = fnv_u32(h, (x >> 32) as u32);
                    }
                }
                let all = Rect { x: 0, y: 0, w: c.w, h: c.h };
                canvas_fill(&mut c, all, 255);
                paper_tile_more(&mut c, tr_);
                h = fnv_u32(h, canvas_hash(&c) as u32);
            }
            p.out.push_str(&format!("labels {} {} {:016x}\n", code, if th != 0 { "classic" } else { "gothic" }, h));
        }
    }
    i18n_set("en");
    canvas_free(&mut c);
}

// ---- Custom music ----

const MB_UP: i32 = 1;
const MB_DOWN: i32 = 2;
const MB_REMOVE: i32 = 3;
const MB_ADD: i32 = 4;
const MB_LESS: i32 = 5;
const MB_MORE: i32 = 6;
const MB_CANCEL_ID: i32 = 7;
const MB_DONE_ID: i32 = 8;

fn mb_tap(a: &mut App, b: &mut MusicBuild, id: i32, arg: i32, d: &mut Rect) -> i32 {
    for i in 0..b.hits.h.len() {
        let h = b.hits.h[i];
        if h.id != id || h.arg != arg {
            continue;
        }
        let x = (h.r.x + h.r.w / 2) as f32;
        let y = (h.r.y + h.r.h / 2) as f32;
        musicbuild_pointer(b, a, 0, x, y, 1000, d);
        return musicbuild_pointer(b, a, 2, x, y, 1080, d) as i32;
    }
    -1
}

fn layout(systems: i32, parts: &[MusicPart]) -> MusicLayout {
    let mut m = MusicLayout { systems, n: parts.len() as i32, ..Default::default() };
    for (i, &pt) in parts.iter().enumerate() {
        m.part[i] = pt;
    }
    m
}

fn music(p: &mut Probe) {
    let l = [
        layout(5, &[PART_TREBLE, PART_TAB6]),
        layout(3, &[PART_TREBLE, PART_BASS, PART_ALTO, PART_TENOR, PART_PERC, PART_LYRICS]),
        layout(1, &[PART_STAFF]),
        layout(12, &[PART_TAB4, PART_TAB5, PART_TAB7]),
    ];
    for turn in 0..2 {
        let (w, h) = dims(turn);
        p.screen(w, h);
        let (pw, ph) = if turn != 0 { (1986, 2648) } else { (1860, 2480) };
        for th in 0..2 {
            look(th);
            for li in 0..4 {
                let mut b = MusicBuild::default();
                musicbuild_open_page(&mut b, &mut p.app, music_encode(&l[li]), pw, ph);
                p.clear_screen();
                let r = musicbuild_draw(&mut b, &mut p.app);
                p.out.push_str(&cformat!("music %s L%d rect %d %d %d %d systems %d ", tag(turn, th), li as i32, r.x, r.y, r.w, r.h, b.m.systems));
                p.emit("hash");
                musicbuild_free(&mut b);
            }
        }
    }
    // A sheet's taps: each prints what the pointer said, the layout it leaves and the screen.
    p.screen(1860, 2480);
    look(0);
    let mut b = MusicBuild::default();
    musicbuild_open(&mut b, &mut p.app, PAPER_BLANK);
    musicbuild_draw(&mut b, &mut p.app);
    let steps: [[i32; 2]; 20] = [
        [MB_ADD, PART_BASS.0],
        [MB_ADD, PART_LYRICS.0],
        [MB_DOWN, 0],
        [MB_UP, 3],
        [MB_REMOVE, 1],
        [MB_MORE, 0],
        [MB_MORE, 0],
        [MB_LESS, 0],
        [MB_ADD, PART_TAB6.0],
        [MB_ADD, PART_ALTO.0],
        [MB_ADD, PART_TENOR.0],
        [MB_ADD, PART_PERC.0],
        [MB_REMOVE, 0],
        [MB_UP, 0],
        [MB_LESS, 0],
        [MB_LESS, 0],
        [MB_LESS, 0],
        [MB_LESS, 0],
        [MB_LESS, 0],
        [MB_MORE, 0],
    ];
    for s in steps.iter() {
        let mut d = RECT_EMPTY;
        let r = mb_tap(&mut p.app, &mut b, s[0], s[1], &mut d);
        p.out.push_str(&cformat!(
            "music tap %d %d -> %d dirty %d %d %d %d systems %d n %d kind %u ",
            s[0],
            s[1],
            r,
            d.x,
            d.y,
            d.w,
            d.h,
            b.m.systems,
            b.m.n,
            musicbuild_result(&b).0
        ));
        p.emit("hash");
    }
    let mut d = RECT_EMPTY;
    let mut r = mb_tap(&mut p.app, &mut b, MB_DONE_ID, 0, &mut d);
    p.out.push_str(&cformat!("music done -> %d open %d kind %u saved %u\n", r, b.open, musicbuild_result(&b).0, p.app.settings.music_layout.0));
    // Opened again: from the layout saved. And a tap outside, and Cancel.
    musicbuild_open(&mut b, &mut p.app, PAPER_BLANK);
    p.out.push_str(&cformat!("music again systems %d n %d kind %u\n", b.m.systems, b.m.n, musicbuild_result(&b).0));
    musicbuild_draw(&mut b, &mut p.app);
    musicbuild_pointer(&mut b, &mut p.app, 0, 5.0, 5.0, 1000, &mut d);
    r = musicbuild_pointer(&mut b, &mut p.app, 2, 5.0, 5.0, 1080, &mut d) as i32;
    p.out.push_str(&cformat!("music outside -> %d open %d dirty %d %d %d %d\n", r, b.open, d.x, d.y, d.w, d.h));
    musicbuild_open(&mut b, &mut p.app, PAPER_BLANK);
    musicbuild_draw(&mut b, &mut p.app);
    r = mb_tap(&mut p.app, &mut b, MB_CANCEL_ID, 0, &mut d);
    p.out.push_str(&cformat!("music cancel -> %d open %d\n", r, b.open));
    musicbuild_free(&mut b);
}

// ---- Templates ----

fn ts_hit(s: &TplSheet, id: i32, arg: i32) -> Option<Hit> {
    for i in 0..s.hits.h.len() {
        if s.hits.h[i].id == id && (id != TS_HIT_TILE || s.hits.h[i].arg == arg) {
            return Some(s.hits.h[i]);
        }
    }
    None
}

fn templates(p: &mut Probe) {
    let root = p.root.clone();
    assert_eq!(system(&format!("rm -rf '{}/templates'", root)), 0);
    templates_init();
    // An empty folder, then with files that are no pictures, in every language (the empty state's
    // paragraphs wrap differently in each), then the Photos sheet's.
    for phase in 0..2 {
        if phase == 1 {
            assert_eq!(write_file_atomic(&format!("{}/templates/Scan.png", root), b"no picture"), 0);
            assert_eq!(write_file_atomic(&format!("{}/templates/Photo.jpg", root), b"no picture either"), 0);
            // (opened once, the previews job finds them bad: the sheets below say how many files it cannot use)
            let mut first = TplSheet::default();
            tplsheet_open(&mut first, &mut p.app, None, PAPER_BLANK, None);
            drain_jobs();
            tplsheet_close(&mut first);
        }
        for l in 0..i18n_language_count() {
            let code = i18n_language_code(l).unwrap();
            i18n_set(code);
            for turn in 0..2 {
                let (w, h) = dims(turn);
                p.screen(w, h);
                for th in 0..2 {
                    look(th);
                    for photos in 0..2 {
                        let mut s = TplSheet::default();
                        if photos != 0 {
                            tplsheet_open_photos(&mut s, &mut p.app, &NotebookInfo::default(), PAPER_BLANK);
                        } else {
                            tplsheet_open(&mut s, &mut p.app, None, PAPER_BLANK, None);
                        }
                        p.clear_screen();
                        tplsheet_draw(&mut s, &mut p.app);
                        let label = cformat!("tplsheet empty%d %s %s %s", phase, code, tag(turn, th), if photos != 0 { "photos" } else { "templates" });
                        p.emit(&label);
                        tplsheet_close(&mut s);
                    }
                }
            }
        }
        i18n_set("en");
        drain_jobs();
    }
    // Pictures: waiting tiles, then their previews, one of them the paper now, a broken one crossed out.
    assert_eq!(system(&format!("rm -rf '{}/templates'", root)), 0);
    templates_init();
    for n in 1..=30 {
        put_template(&root, n, &format!("page-{}.png", n));
    }
    assert_eq!(write_file_atomic(&format!("{}/templates/zz-broken.png", root), b"no picture"), 0);
    for turn in 0..2 {
        let (w, h) = dims(turn);
        p.screen(w, h);
        for th in 0..2 {
            look(th);
            let mut s = TplSheet::default();
            let current = format!("{}/templates/page-17.png", root);
            tplsheet_open(&mut s, &mut p.app, None, PAPER_BLANK, Some(&current));
            p.clear_screen();
            tplsheet_draw(&mut s, &mut p.app);
            p.out.push_str(&cformat!("tplsheet %s waiting page %d of %d selected %d ", tag(turn, th), s.page + 1, s.pages, s.selected));
            p.emit("hash");
            drain_jobs();
            let mut d = RECT_EMPTY;
            let ticked = tplsheet_tick(&mut s, &mut p.app, now_ms(), &mut d);
            p.out.push_str(&cformat!("tplsheet %s ready tick %d dirty %d %d %d %d page %d of %d ", tag(turn, th), ticked, d.x, d.y, d.w, d.h, s.page + 1, s.pages));
            p.emit("hash");
            // Every page of tiles, the broken one's last.
            let mut pg = 1;
            while pg < s.pages + 1 {
                let Some(hit) = ts_hit(&s, TS_HIT_NEXT, 0) else {
                    break;
                };
                let x = (hit.r.x + hit.r.w / 2) as f32;
                let y = (hit.r.y + hit.r.h / 2) as f32;
                tplsheet_pointer(&mut s, &mut p.app, 0, x, y, 1000, &mut d);
                let r = tplsheet_pointer(&mut s, &mut p.app, 2, x, y, 1080, &mut d);
                let label = cformat!("tplsheet %s turn %d -> %d page %d", tag(turn, th), pg, r as i32, s.page + 1);
                p.emit(&label);
                pg += 1;
            }
            tplsheet_close(&mut s);
        }
    }
    drain_jobs();
}

// ---- Pages ----

fn setup_notebook(p: &mut Probe) {
    assert_eq!(store_create("Probe", "", PAPER_MEDIUM, &mut p.nb), 0);
    for i in 1..30 {
        assert_eq!(store_add_page(&mut p.nb, i, PAPER_MEDIUM), 0);
    }
    nav_init(&mut p.nav);
    nav_init(&mut p.many);
    let names: [(usize, &str); 5] = [
        (1, "Intro"),
        (6, "Chapter two, a rather long name that must be cut short somewhere in the tile"),
        (12, "\u{65e5}\u{672c}\u{8a9e}"),
        (20, "\u{c9}t\u{e9}"),
        (25, "Notes"),
    ];
    for (page, name) in names {
        nav_set_name(&mut p.nav, p.nb.page_ids[page], name);
    }
    for i in 0..20usize {
        let nm = format!("Section {}", i + 1);
        nav_set_name(&mut p.many, p.nb.page_ids[i + if i > 9 { 3 } else { 0 }], &nm);
    }
    let mut dir = String::new();
    assert_eq!(store_notebook_dir(p.nb.id.as_str(), &mut dir, 700), 0);
    let file = format!("{}/source.ini", dir);
    let text = format!(
        "# probe\npage.12.300={}\npage.13.300={}\npage.5.300={}\npage.44.340={}\npage.7.300={}\n",
        p.nb.page_ids[2], p.nb.page_ids[6], p.nb.page_ids[9], p.nb.page_ids[17], p.nb.page_ids[26]
    );
    assert_eq!(write_file_atomic(&file, text.as_bytes()), 0);
}

// Hit ids, as pagesview.rs numbers them.
const PG_BACK: i32 = 1;
const PG_DELETE: i32 = 2;
const PG_ALL: i32 = 3;
const PG_TOGGLE: i32 = 4;
const PG_OPEN: i32 = 5;
const PG_PREV: i32 = 6;
const PG_NEXT: i32 = 7;
const PG_JUMP: i32 = 8;
const PG_NEW: i32 = 9;
const PG_COPY: i32 = 10;
const PG_TAB: i32 = 11;
const PG_NAME: i32 = 12;
const PG_LINK: i32 = 13;

fn pv_hit(v: &PagesView, id: i32, arg: i32) -> Option<Hit> {
    v.hits.h.iter().find(|h| h.id == id && h.arg == arg).copied()
}

fn pv_tap(p: &mut Probe, v: &mut PagesView, id: i32, arg: i32) -> i32 {
    let Some(h) = pv_hit(v, id, arg) else {
        return -1;
    };
    let mut ev = InEvent { type_: InType::TouchDown, x: (h.r.x + h.r.w / 2) as f32, y: (h.r.y + h.r.h / 2) as f32, t_ms: 1000, ..Default::default() };
    let (nb, nav) = (p.nb.clone(), p.nav.clone());
    pagesview_pointer(v, &mut p.app, &nb, &nav, 0, &ev);
    ev.type_ = InType::TouchUp;
    ev.t_ms = 1080;
    pagesview_pointer(v, &mut p.app, &nb, &nav, 2, &ev) as i32
}

fn pv_reset(v: &mut PagesView) {
    pagesview_free(v);
    *v = PagesView::default();
}

fn pv_probe_all(p: &mut Probe, v: &PagesView, nav: &Nav, tagname: &str) {
    const KEYS: [&str; 17] = [
        "pages_tab", "picking", "keyboard", "text", "index_rows", "tile.1", "tile.3", "tile.7", "tile.10", "tile.13", "tile.18", "tile.27", "tile.30", "tile.31",
        "tile.0", "tile.x", "bogus",
    ];
    for k in KEYS {
        let mut out = String::from("xxxx");
        let rc = pagesview_probe(v, &p.nb, nav, k, &mut out, 200);
        p.out.push_str(&cformat!("pages %s probe %s rc %d '%s'\n", tagname, k, rc, if rc == 0 { out.as_str() } else { "" }));
    }
}

fn pages_views(p: &mut Probe) {
    let (nb, nav, many) = (p.nb.clone(), p.nav.clone(), p.many.clone());
    for turn in 0..2 {
        let (w, h) = dims(turn);
        p.screen(w, h);
        for th in 0..2 {
            look(th);
            let t = tag(turn, th);
            let mut v = PagesView::default();
            // The Pages tab: the first screenful, then the one with the open page and some pages ticked.
            assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 0));
            v.index_tab = false;
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s tab0 screen %d", t, v.screen);
            p.emit(&label);
            pagesview_close(&mut v);
            assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 17));
            v.index_tab = false;
            v.sel[3] = true;
            v.sel[4] = true;
            v.sel[18] = true;
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s tab17 ticked %d screen %d", t, pagesview_ticked_count(&v), v.screen);
            p.emit(&label);
            for i in 0..30 {
                v.sel[i] = true;
            }
            v.screen = 99; // (clamped to the last)
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s all ticked %d screen %d", t, pagesview_ticked_count(&v), v.screen);
            p.emit(&label);
            pagesview_untick(&mut v);
            v.sel[7] = true;
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s one ticked %d", t, pagesview_ticked_count(&v));
            p.emit(&label);
            pv_probe_all(p, &v, &nav, t);
            // The Index tab, with five names, and with twenty (two screenfuls).
            v.index_tab = true;
            v.index_screen = 0;
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s index", t);
            p.emit(&label);
            pagesview_close(&mut v);
            pv_reset(&mut v);
            assert!(pagesview_open(&mut v, &mut p.app, &nb, &many, 4));
            for s in 0..3 {
                v.index_screen = s;
                pagesview_draw(&mut v, &mut p.app, &nb, &many);
                let label = cformat!("pages %s many index %d (screen %d)", t, s, v.index_screen);
                p.emit(&label);
            }
            pv_probe_all(p, &v, &many, t);
            pagesview_close(&mut v);
            // Picking where a link goes is checked on its own (pages_picking below): the picking header lost
            // The Index, Remove link and Go to page... in Index phase 2, so it no longer matches the C build.
            // The naming keyboard, over the Index; and Go to page's number pad.
            pv_reset(&mut v);
            assert!(pagesview_name(&mut v, &mut p.app, &nb, &nav, 6));
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s name only %d", t, v.name_only);
            p.emit(&label);
            pv_probe_all(p, &v, &nav, t);
            pagesview_close(&mut v);
            p.out.push_str(&cformat!("pages %s after name: tab %d init %d\n", t, v.index_tab, v.tab_initialized));
            assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 9));
            v.index_tab = false;
            v.naming = false;
            keyboard_open(&mut v.kb, "Go to page", b"");
            v.kb.page = 2;
            pagesview_draw(&mut v, &mut p.app, &nb, &nav);
            let label = cformat!("pages %s jump", t);
            p.emit(&label);
            pv_probe_all(p, &v, &nav, t);
            pagesview_close(&mut v);
            // Names that cannot be saved: Name this page greyed out, Add a link greyed out.
            let mut ro = nav.clone();
            ro.writable = false;
            ro.links_writable = false;
            assert!(pagesview_open(&mut v, &mut p.app, &nb, &ro, 6));
            v.index_tab = true;
            pagesview_draw(&mut v, &mut p.app, &nb, &ro);
            let label = cformat!("pages %s readonly index", t);
            p.emit(&label);
            pagesview_close(&mut v);
            let mut empty = Nav::default();
            nav_init(&mut empty);
            empty.writable = false;
            assert!(pagesview_open(&mut v, &mut p.app, &nb, &empty, 6));
            v.index_tab = true;
            pagesview_draw(&mut v, &mut p.app, &nb, &empty);
            let label = cformat!("pages %s unreadable index", t);
            p.emit(&label);
            pagesview_close(&mut v);
            pagesview_free(&mut v);
        }
    }
    // The translations: the Pages and Index tabs and the number pad in every language.
    for l in 0..i18n_language_count() {
        let code = i18n_language_code(l).unwrap();
        i18n_set(code);
        for turn in 0..2 {
            let (w, h) = dims(turn);
            p.screen(w, h);
            for th in 0..2 {
                look(th);
                let mut v = PagesView::default();
                assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 6));
                let mut h = H0;
                v.index_tab = false;
                v.sel[2] = true;
                pagesview_draw(&mut v, &mut p.app, &nb, &nav);
                h = fold(h, &p.app.screen);
                v.index_tab = true;
                pagesview_draw(&mut v, &mut p.app, &nb, &nav);
                h = fold(h, &p.app.screen);
                pagesview_close(&mut v);
                assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 6));
                v.index_tab = false;
                keyboard_open(&mut v.kb, "Go to page", b"");
                v.kb.page = 2;
                pagesview_draw(&mut v, &mut p.app, &nb, &nav);
                h = fold(h, &p.app.screen);
                pagesview_free(&mut v);
                p.out.push_str(&format!("pages lang {} {} {:016x}\n", code, tag(turn, th), h));
            }
        }
    }
    i18n_set("en");
}

// Rust only (Index phase 2): the Pages view picking where a new link goes (Link to...'s Choose from the
// pages...). Its header keeps Back and the two tabs and nothing beside them (The Index is a chip of Link to...
// and Remove link went with the hold path), and its bottom row has only the pagers: no Go to page..., New page
// or Copy page. A tile still picks its page; with no names it opens on the Pages tab.
fn pages_picking(p: &mut Probe) {
    let (nb, nav) = (p.nb.clone(), p.nav.clone());
    let mut none = Nav::default();
    nav_init(&mut none);
    for turn in 0..2 {
        let (w, h) = dims(turn);
        p.screen(w, h);
        for th in 0..2 {
            look(th);
            for named in 0..2 {
                let n = if named != 0 { &nav } else { &none };
                let mut v = PagesView::default();
                check!(pagesview_pick(&mut v, &mut p.app, &nb, n, 12));
                check!(v.picking && v.index_tab == (named != 0));
                pagesview_draw(&mut v, &mut p.app, &nb, n);
                let mut out = String::new();
                check!(pagesview_probe(&v, &nb, n, "picking", &mut out, 64) == 0 && out == "new");
                for retired in [PG_JUMP, PG_NEW, PG_COPY, PG_DELETE, PG_ALL, PG_NAME, PG_LINK, 14, 15] {
                    check!(!v.hits.h.iter().any(|h| h.id == retired));
                }
                // The header: white from the Index tab's right edge to the screen's.
                let tabs = v.hits.h.iter().filter(|h| h.id == PG_TAB).map(|h| h.r.x + h.r.w).max().unwrap_or(0);
                check!(tabs > 0);
                let back = v.hits.h.iter().find(|h| h.id == PG_BACK).map(|h| h.r).unwrap_or_default();
                let c = &p.app.screen;
                let mut white = true;
                for y in back.y..back.y + back.h {
                    for x in tabs + 1..c.w {
                        white &= c.px[(y * c.stride + x) as usize] == 255;
                    }
                }
                check!(white);
                // A tile picks its page (Pages tab), as before.
                if named == 0 {
                    check!(pv_tap(p, &mut v, PG_OPEN, 13) == PvResult::Pick as i32 && v.target == 13);
                }
                pagesview_free(&mut v);
            }
        }
    }
}

// A scripted sequence of taps, as the editor would see them: what each says, and the view's state.
fn pages_taps(p: &mut Probe) {
    let (nb, nav) = (p.nb.clone(), p.nav.clone());
    p.screen(1860, 2480);
    look(0);
    let mut v = PagesView::default();
    v.jump_pad = true; // (the C build's Go to page... is a number pad: this holds the view to what it recorded)
    assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 17));
    v.index_tab = false;
    pagesview_draw(&mut v, &mut p.app, &nb, &nav);
    let steps: [[i32; 2]; 25] = [
        [PG_TOGGLE, 3],
        [PG_TOGGLE, 5],
        [PG_TOGGLE, 3],
        [PG_ALL, 0],
        [PG_ALL, 0],
        [PG_NEXT, 0],
        [PG_TOGGLE, 29],
        [PG_COPY, 0],
        [PG_NEW, 0],
        [PG_DELETE, 0],
        [PG_PREV, 0],
        [PG_PREV, 0],
        [PG_TOGGLE, 12],
        [PG_COPY, 0],
        [PG_NEW, 0],
        [PG_TAB, 1],
        [PG_NAME, 0],
        [PG_TAB, 0],
        [PG_OPEN, 11],
        [PG_BACK, 0],
        [PG_TAB, 1],
        [PG_LINK, 0],
        [PG_NEXT, 0],
        [PG_PREV, 0],
        [PG_JUMP, 0],
    ];
    for s in steps.iter() {
        let r = pv_tap(p, &mut v, s[0], s[1]);
        let label = cformat!(
            "pages tap %d %d -> %d target %d tab %d screen %d idx %d ticked %d kb %d",
            s[0],
            s[1],
            r,
            v.target,
            v.index_tab,
            v.screen,
            v.index_screen,
            pagesview_ticked_count(&v),
            v.kb.open
        );
        p.emit(&label);
        if v.kb.open {
            break;
        }
    }
    // The number pad: a page in the notebook, one that is not, and Cancel.
    for typed in ["12", "31", "0", "7x"] {
        pagesview_close(&mut v);
        assert!(pagesview_open(&mut v, &mut p.app, &nb, &nav, 17));
        v.index_tab = false;
        pagesview_draw(&mut v, &mut p.app, &nb, &nav);
        pv_tap(p, &mut v, PG_JUMP, 0);
        let screen = Rect { x: 0, y: 0, w: p.app.w, h: p.app.h };
        keyboard_draw(&mut v.kb, &mut p.app.screen, screen);
        let mut ev = InEvent::default();
        for ch in typed.chars() {
            let mut kr = RECT_EMPTY;
            if !keyboard_key_rect(&v.kb, ch as u32, &mut kr) {
                p.out.push_str(&cformat!("pages key %c: none\n", ch));
                continue;
            }
            ev.type_ = InType::TouchDown;
            ev.x = (kr.x + kr.w / 2) as f32;
            ev.y = (kr.y + kr.h / 2) as f32;
            ev.t_ms = 2000;
            pagesview_pointer(&mut v, &mut p.app, &nb, &nav, 0, &ev);
            ev.type_ = InType::TouchUp;
            ev.t_ms = 2080;
            pagesview_pointer(&mut v, &mut p.app, &nb, &nav, 2, &ev);
        }
        let mut kr = RECT_EMPTY;
        keyboard_key_rect(&v.kb, KB_KEY_DONE, &mut kr);
        ev.type_ = InType::TouchDown;
        ev.x = (kr.x + kr.w / 2) as f32;
        ev.y = (kr.y + kr.h / 2) as f32;
        ev.t_ms = 3000;
        pagesview_pointer(&mut v, &mut p.app, &nb, &nav, 0, &ev);
        ev.type_ = InType::TouchUp;
        ev.t_ms = 3080;
        let r = pagesview_pointer(&mut v, &mut p.app, &nb, &nav, 2, &ev) as i32;
        let label = cformat!("pages typed '%s' -> %d target %d kb %d text '%s'", typed, r, v.target, v.kb.open, &v.kb.text);
        p.emit(&label);
    }
    pagesview_free(&mut v);
}

// ---- Pen feel ----

const PEN_KEYS: [&str; 19] = [
    "slider_pending",
    "param.size_min_mm",
    "param.size_max_mm",
    "param.opacity_max",
    "param.pressure_floor",
    "param.pressure_ceiling",
    "param.tilt_min_deg",
    "param.tilt_max_deg",
    "param.grain",
    "param.nonesuch",
    "pad_strokes",
    "pad_sample_rect",
    "pad_white",
    "pad_ink",
    "pad_lighter",
    "pad_ink_near.1000.1000",
    "pad_ink_near.-5.x",
    "pad_ink_near.700.900",
    "bogus",
];

fn pen_probe_all(p: &mut Probe, tagname: &str) {
    for k in PEN_KEYS {
        let mut out = String::from("xxxx");
        let rc = pensettings_probe(&mut p.app, k, &mut out, 200);
        p.out.push_str(&cformat!("penset %s probe %s rc %d '%s'\n", tagname, k, rc, if rc == 0 { out.as_str() } else { "" }));
    }
}

fn params_hash(prm: &ToolParams) -> u64 {
    let mut h = H0;
    for i in 0..ToolParams::COUNT {
        h = fnv_u32(h, prm.get(i).to_bits());
    }
    h
}

fn pen(p: &mut Probe, type_: InType, x: f32, y: f32, pressure: f32, tx: f32, ty: f32, ms: u32, eraser: bool) {
    let ev = InEvent { type_, x, y, pressure, tilt_x: tx, tilt_y: ty, t_ms: ms, eraser, ..Default::default() };
    pensettings_event(&mut p.app, &ev);
}

fn st(p: &mut Probe) -> &mut PenSetState {
    p.app.ps.as_deref_mut().unwrap()
}

// A stroke across the test pad, in 40 samples: a zigzag, the pressure ramping up and down, the pen
// leaning one way and the other. Made of integers, so the Rust twin makes the same.
fn pad_stroke(p: &mut Probe, eraser: bool, t0: u32, phase: i32) {
    let pad = st(p).pad;
    let x0 = (pad.x + pad.w / 10) as f32;
    let y0 = (pad.y + pad.h / 2 + phase * 40) as f32;
    pen(p, InType::PenDown, x0, y0, 0.15, 20.0, -15.0, t0, eraser);
    for i in 1..=40 {
        let x = x0 + (i * (pad.w * 8 / 10) / 40) as f32;
        let y = y0 + ((i * 37) % 160 - 80) as f32;
        let pr = (if i < 20 { 10 + i * 4 } else { 90 - (i - 20) * 3 }) as f32 / 100.0;
        let tx = ((i * 5) % 50) as f32 - 10.0;
        let ty = ((i * 3) % 60) as f32 - 30.0;
        pen(p, InType::PenMove, x, y, pr, tx, ty, t0 + i as u32 * 12, eraser);
    }
    pen(p, InType::PenUp, x0 + (pad.w * 8 / 10) as f32, y0, 0.0, 0.0, 0.0, t0 + 600, eraser);
}

fn pen_screen(p: &mut Probe, tab: ToolKind) {
    let s = st(p);
    s.tab = tab;
    s.ret = ScreenId::Editor;
    pensettings_enter(&mut p.app);
}

fn penset_draws(p: &mut Probe) {
    pensettings_init(&mut p.app);
    settings_default(&mut p.app.settings);
    p.app.scr = ScreenId::Penset;
    // Every tool's tab, upright and sideways, in both looks; the eraser's two modes; white ink.
    for turn in 0..2 {
        let (w, h) = dims(turn);
        p.screen(w, h);
        for th in 0..2 {
            look(th);
            for ti in 0..TOOL_COUNT {
                let tab = TOOL_ORDER[ti];
                for mode in 0..3 {
                    p.app.settings.eraser_mode = if mode == 1 { 1 } else { 0 };
                    p.app.settings.white = mode == 2;
                    pen_screen(p, tab);
                    let label = cformat!("penset %s tab %d mode %d", tag(turn, th), tab.0, mode);
                    p.emit(&label);
                }
            }
        }
    }
    p.app.settings.eraser_mode = 0;
    p.app.settings.white = false;
    // The languages: the calligraphy tab (its own names for two sliders) and the eraser tab.
    for l in 0..i18n_language_count() {
        let code = i18n_language_code(l).unwrap();
        i18n_set(code);
        p.screen(1860, 2480);
        let mut h = H0;
        for th in 0..2 {
            look(th);
            let tabs = [TOOL_CALLIGRAPHY, TOOL_ERASER, TOOL_PENCIL, TOOL_CHARCOAL];
            for k in 0..4 {
                for mode in 0..2 {
                    p.app.settings.eraser_mode = mode;
                    pen_screen(p, tabs[k]);
                    h = fold(h, &p.app.screen);
                }
            }
        }
        p.out.push_str(&format!("penset lang {} {:016x}\n", code, h));
    }
    i18n_set("en");
    pensettings_free(&mut p.app);
}

fn penset_events(p: &mut Probe) {
    pensettings_init(&mut p.app);
    settings_default(&mut p.app.settings);
    p.app.scr = ScreenId::Penset;
    // The pen on the pad, on each tool's tab: the stroke, the readout, the graph's live dot.
    p.screen(1860, 2480);
    look(0);
    for ti in 0..TOOL_COUNT {
        let tab = TOOL_ORDER[ti];
        pen_screen(p, tab);
        pad_stroke(p, false, 100000, 0);
        let label = cformat!("penset pad tool %d", tab.0);
        p.emit(&label);
        pad_stroke(p, false, 200000, 1);
        let label = cformat!("penset pad tool %d again", tab.0);
        p.emit(&label);
        pen_probe_all(p, &label[7..]);
    }
    // The same in the other look and sideways, for two tools.
    for turn in 0..2 {
        let (w, h) = dims(turn);
        p.screen(w, h);
        for th in 0..2 {
            look(th);
            let two = [TOOL_CALLIGRAPHY, TOOL_HIGHLIGHTER];
            for k in 0..2 {
                pen_screen(p, two[k]);
                pad_stroke(p, false, 300000, 0);
                let label = cformat!("penset pad %s tool %d", tag(turn, th), two[k].0);
                p.emit(&label);
            }
        }
    }
    p.screen(1860, 2480);
    look(0);
    // White ink on a drawing tab (the swatch), its eraser end, the precise eraser, the whole-stroke eraser.
    p.app.settings.white = true;
    pen_screen(p, TOOL_PENCIL);
    p.emit("penset white tab");
    pad_stroke(p, false, 400000, 0);
    p.emit("penset white stroke");
    pen_probe_all(p, "white");
    p.app.settings.white = false;
    p.app.settings.eraser_mode = 1;
    pen_screen(p, TOOL_BALLPOINT);
    pad_stroke(p, false, 500000, 0);
    p.emit("penset ballpoint ink");
    pad_stroke(p, true, 600000, 0); // the pen's eraser end: the precise eraser
    p.emit("penset ballpoint erased (end)");
    pen_probe_all(p, "end");
    p.app.settings.eraser_mode = 0;
    pen_screen(p, TOOL_FOUNTAIN);
    pad_stroke(p, false, 700000, 0);
    pad_stroke(p, true, 800000, 0); // the pen's eraser end, whole strokes: the pad is drawn bare
    p.emit("penset fountain wiped (end)");
    pen_probe_all(p, "wiped");
    // The eraser's tab: the practice ink, whole strokes taken away, then the precise eraser.
    for mode in 0..2 {
        p.app.settings.eraser_mode = mode;
        pen_screen(p, TOOL_ERASER);
        let label = cformat!("penset eraser mode %d", mode);
        p.emit(&label);
        pen_probe_all(p, &label[7..]);
        for k in 0..3 {
            pad_stroke(p, false, 900000 + k as u32 * 10000, k - 1);
            let label = cformat!("penset eraser mode %d stroke %d", mode, k);
            p.emit(&label);
            pen_probe_all(p, &label[7..]);
        }
    }
    // Sliders: a drag on each row, with a hold and a wide y for the fine mode, and the pairs' limits.
    p.app.settings.eraser_mode = 0;
    for tabi in 0..2 {
        let tab = if tabi != 0 { TOOL_CALLIGRAPHY } else { TOOL_PENCIL };
        pen_screen(p, tab);
        let n = st(p).nsliders;
        for row in 0..n {
            let r = st(p).slider_rows[row as usize];
            let xs = [
                (r.x + r.w * 45 / 100) as f32,
                (r.x + r.w * 60 / 100) as f32,
                (r.x + r.w * 99 / 100) as f32,
                (r.x + r.w * 50 / 100) as f32,
                (r.x + r.w * 41 / 100) as f32,
            ];
            let y = (r.y + r.h / 2) as f32;
            let t = 1000000 + row as u32 * 5000;
            pen(p, InType::PenDown, xs[0], y, 0.5, 0.0, 0.0, t, false);
            pen(p, InType::PenMove, xs[1], y, 0.5, 0.0, 0.0, t + 30, false);
            pen(p, InType::PenMove, xs[2], y, 0.5, 0.0, 0.0, t + 60, false);
            pen(p, InType::PenMove, xs[3], y + 130.0, 0.5, 0.0, 0.0, t + 500, false);
            pen(p, InType::PenMove, xs[4], y + 130.0, 0.5, 0.0, 0.0, t + 700, false);
            pen(p, InType::PenUp, xs[4], y + 130.0, 0.0, 0.0, 0.0, t + 800, false);
            let label = format!("penset slider tab {} row {} params {:016x}", tab.0, row, params_hash(&p.app.settings.tools[tab.0 as usize]));
            p.emit(&label);
            pen_probe_all(p, "slider");
        }
        // A drag held still (the tick repaints it), then released.
        {
            let r = st(p).slider_rows[1];
            let y = (r.y + r.h / 2) as f32;
            let x70 = (r.x + r.w * 70 / 100) as f32;
            let x72 = (r.x + r.w * 72 / 100) as f32;
            pen(p, InType::PenDown, x70, y, 0.5, 0.0, 0.0, 2000000, false);
            pen(p, InType::PenMove, x72, y, 0.5, 0.0, 0.0, 2000010, false);
            pensettings_tick(&mut p.app, 2000020);
            let d = st(p).slider_dirty;
            p.out.push_str(&cformat!("penset tick pending %d\n", d));
            pensettings_tick(&mut p.app, 2000100);
            let d = st(p).slider_dirty;
            p.out.push_str(&cformat!("penset tick pending %d\n", d));
            pen(p, InType::PenUp, x72, y, 0.0, 0.0, 0.0, 2000200, false);
            p.emit("penset slider held");
        }
    }
    // Pairs that keep each other in order (the widths, the pressure range, the tilt range): a slider
    // dragged past its partner takes the partner along.
    p.app.settings.eraser_mode = 0;
    pen_screen(p, TOOL_PENCIL);
    {
        // the row, and where the drag ends: its share of the row's width, in percent (0 and 100: the ends)
        const PAIRS: [(usize, i32); 11] = [(4, 50), (5, 0), (4, 100), (8, 0), (7, 100), (1, 0), (0, 100), (5, 100), (4, 0), (7, 0), (8, 100)];
        for i in 0..PAIRS.len() {
            let (row, to) = PAIRS[i];
            let r = st(p).slider_rows[row];
            let y = (r.y + r.h / 2) as f32;
            let x_to = if to == 0 {
                (r.x + r.w * 42 / 100) as f32
            } else if to == 100 {
                (r.x + r.w - 2) as f32
            } else {
                (r.x + r.w * to / 100) as f32
            };
            let t = 3000000 + i as u32 * 1000;
            pen(p, InType::PenDown, (r.x + r.w * 70 / 100) as f32, y, 0.5, 0.0, 0.0, t, false);
            pen(p, InType::PenMove, x_to, y, 0.5, 0.0, 0.0, t + 100, false);
            pen(p, InType::PenUp, x_to, y, 0.0, 0.0, 0.0, t + 200, false);
            let label = format!("penset pairs {} row {} to {} params {:016x}", i, row, to, params_hash(&p.app.settings.tools[TOOL_PENCIL.0 as usize]));
            p.emit(&label);
        }
    }
    pensettings_leave(&mut p.app);
    pensettings_free(&mut p.app);
}

// ---- the comparison ----

// The recorded text of section name (the lines after its "== name" marker, up to the next). The pen
// on the pad shows its tilt in Pen feel's readout only when input_caps says the pen reports it: the
// headless runner's does (the sim build), the host's platform does not, and the C probe was recorded
// both ways (sheets_probe_c_tilt.txt holds the one section that differs).
fn recorded(name: &str) -> String {
    let all = if cfg!(feature = "sim") && name == "penset events" {
        include_str!("fixtures/sheets_probe_c_tilt.txt")
    } else {
        include_str!("fixtures/sheets_probe_c.txt")
    };
    let mut out = String::new();
    let mut on = false;
    for line in all.lines() {
        if let Some(n) = line.strip_prefix("== ") {
            on = n == name;
            continue;
        }
        if on {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn same(name: &str, got: &str) {
    let want = recorded(name);
    let (mut g, mut w) = (got.lines(), want.lines());
    let mut n = 0;
    let mut bad = 0;
    loop {
        let (a, b) = (g.next(), w.next());
        if a.is_none() && b.is_none() {
            break;
        }
        n += 1;
        if a != b {
            bad += 1;
            if bad <= 8 {
                eprintln!("  {name} line {n}:\n    C:    {}\n    Rust: {}", b.unwrap_or("(none)"), a.unwrap_or("(none)"));
            }
        }
    }
    check!(bad == 0 && n > 0);
}

fn setup(root: &str) {
    let _ = std::fs::remove_dir_all(root);
    assert_eq!(store_init(root), 0);
    store_set_device(300.0, 1860, 2480);
    ui_init(300.0);
    assert_eq!(font_init(), 0);
}

fn section(p: &mut Probe, name: &str, f: fn(&mut Probe)) {
    p.out.clear();
    f(p);
    let got = std::mem::take(&mut p.out);
    same(name, &got);
}

#[test]
fn test_sheets_probe() {
    let root = format!("{}/sheets-{}", tl_tmp(), std::process::id());
    setup(&root);
    let mut p = Probe::new(&root);
    settings_default(&mut p.app.settings);
    section(&mut p, "labels", labels);
    section(&mut p, "music", music);
    section(&mut p, "templates", templates);
    setup_notebook(&mut p);
    section(&mut p, "pages views", pages_views);
    pages_picking(&mut p);
    section(&mut p, "penset draws", penset_draws);
    font_shutdown();
    done();
    let _ = std::fs::remove_dir_all(&root);
}

// The sections that tap and draw with the pen: they reach app.rs's app_invalidate, app_toast,
// app_keyboard_pointer and app_pointer_phase. app_keyboard_pointer flushes the text it typed
// (app_flush, which the C probe stubs), and app_flush presents it: the panel is opened first, as
// mainloop opens it before any screen runs. (The headless runner gives its panel only to a run
// that names a script; nothing reads this one.)
#[test]
fn test_sheets_probe_events() {
    let root = format!("{}/sheets-events-{}", tl_tmp(), std::process::id());
    setup(&root);
    #[cfg(feature = "sim")]
    std::env::set_var("GRIMOIRE_SCRIPT", "/dev/null");
    let (mut w, mut h, mut dpi) = (0, 0, 0.0f32);
    let _ = display_open(&mut w, &mut h, &mut dpi);
    let mut p = Probe::new(&root);
    settings_default(&mut p.app.settings);
    setup_notebook(&mut p);
    section(&mut p, "pages taps", pages_taps);
    section(&mut p, "penset events", penset_events);
    font_shutdown();
    done();
    let _ = std::fs::remove_dir_all(&root);
}
