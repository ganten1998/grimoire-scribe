// test_annotate.c: a page KOReader sent to annotate (src/annotate.h, docs/FORMAT.md, Annotated
// pages): reading requests and source.ini, the order a book's pages go in, names, the checks a
// page picture gets, and what a start does with a request, on a scratch data root.
//
// The C file is one binary that runs its tests in order. Here the ones that need no module of
// wave 2 (request_age, page_order, names, notices_fit) run now; the others call store.rs,
// bgimage.rs, bgjob.rs, page.rs or export.rs, which were still todo!() when this was ported, so
// they run as two #[ignore]d tests, in the C's order within each (cargo test -- --ignored
// annotate). Once those modules are translated, make it the one #[test] the C has: run! them all
// in the C's RUN order.
mod testlib;

use grimoire::ffi_stb;
use grimoire::prelude::*;
use std::cell::{Cell, RefCell};
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicI32, Ordering};
use testlib::*;

const DPI: f32 = 300.0;
/// the page size the tests' notebooks get (store_set_device in main)
const PW: i32 = 300;
const PH: i32 = 400;
const BOOK: &str = "0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70";
const TITLE: &str = "Le Petit Prince : édition illustrée";

thread_local! {
    static ROOT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The scratch data root of the test running (setup_store).
fn root() -> String {
    ROOT.with(|r| r.borrow().clone())
}

/// main()'s setup of the screens' side: the theme's sizes and the fonts. (The modules' state is
/// the thread's, and each #[test] runs on a thread of its own: every test sets up for itself.)
fn setup_ui() {
    thread_local! {
        static DONE: Cell<bool> = const { Cell::new(false) };
    }
    if !DONE.with(|d| d.replace(true)) {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
    }
}

/// main()'s setup of the store: a fresh root of its own for each of the tests below (the C has one
/// binary, one root), with the tests' device.
fn setup_store(tag: &str) {
    setup_store_device(tag, PW, PH);
}

fn setup_store_device(tag: &str, w: i32, h: i32) {
    let r = format!("{}/annotate-{}-{}", tl_tmp(), std::process::id(), tag);
    ROOT.with(|root| *root.borrow_mut() = r.clone());
    let _ = std::fs::remove_dir_all(&r);
    check!(store_init(&r) == 0);
    store_set_device(DPI, w, h);
    setup_ui();
}

// ---- files ----

fn exists(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}

fn put_file(path: &str, d: &[u8]) {
    let dir = match path.rfind('/') {
        Some(i) => &path[..i],
        None => path,
    };
    mkdir_p(dir);
    check!(write_file_atomic(path, d) == 0);
}

fn request_path(name: &str) -> String {
    format!("{}/{}/{}", root(), ANNOTATE_DIR, name)
}

fn files_in(dir: &str) -> i32 {
    std::fs::read_dir(dir).map_or(0, |d| d.flatten().count() as i32)
}

// ---- pictures ----

thread_local! {
    static G_RNG: Cell<u32> = const { Cell::new(4242) };
}

fn rnd8() -> u8 {
    G_RNG.with(|g| {
        g.set(g.get().wrapping_mul(1103515245).wrapping_add(12345));
        (g.get() >> 16) as u8
    })
}

/// A page of "text": dark runs on white, set by seed, w x h gray, as an encoded PNG.
fn text_png(w: i32, h: i32, seed: u32) -> Vec<u8> {
    let mut p = BgPicture { px: vec![255u8; (w * h) as usize], w, h, ch: 1 };
    G_RNG.with(|g| g.set(seed));
    let mut y = 30;
    while y + 12 < h - 30 {
        let mut x = 20;
        while x < w - 40 {
            let run = 8 + (rnd8() % 40) as i32;
            for yy in y..y + 12 {
                // (a run may reach past the row's end into the next row's start, as the C's memset)
                let at = (yy * w + x) as usize;
                p.px[at..at + run as usize].fill(30);
            }
            x += run + 8;
        }
        y += 24;
    }
    let path = format!("{}/scratch.png", root());
    check!(bgimage_write_png(&path, &p) == 0);
    let d = read_file(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    d
}

fn crc(p: &[u8]) -> u32 {
    let mut c = 0xFFFFFFFFu32;
    for &b in p {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320u32 ^ (c >> 1) } else { c >> 1 };
        }
    }
    c ^ 0xFFFFFFFFu32
}

/// Appends a chunk: length, type, data, CRC.
fn put_chunk(out: &mut Vec<u8>, type_: &str, d: &[u8]) {
    let at = out.len();
    out.extend_from_slice(&(d.len() as u32).to_be_bytes());
    out.extend_from_slice(type_.as_bytes());
    out.extend_from_slice(d);
    let c = crc(&out[at + 4..]);
    out.extend_from_slice(&c.to_be_bytes());
}

/// A 4-bit palette PNG of w x h, as lodepng writes a page of a few grays: stripes of 16 levels.
fn palette_png(w: i32, h: i32) -> Vec<u8> {
    let row = ((w + 1) / 2 + 1) as usize;
    let rawn = row * h as usize;
    let mut raw = vec![0u8; rawn];
    for y in 0..h {
        for x in 0..w {
            let v = ((x / 10 + y / 10) & 15) as u8;
            raw[y as usize * row + 1 + x as usize / 2] |= if x & 1 != 0 { v } else { v << 4 };
        }
    }
    let mut zn: i32 = 0;
    // SAFETY: stbi_zlib_compress reads rawn bytes of raw and returns a malloc block of zn bytes (or
    // null), which is copied out and freed here.
    let z = unsafe {
        let p = ffi_stb::stbi_zlib_compress(raw.as_mut_ptr(), rawn as i32, &mut zn, 8);
        let z = if p.is_null() { Vec::new() } else { std::slice::from_raw_parts(p, zn as usize).to_vec() };
        libc::free(p as *mut libc::c_void);
        z
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[4, 3, 0, 0, 0]);
    let mut plte = [0u8; 48];
    for i in 0..16 {
        plte[3 * i] = (i * 17) as u8;
        plte[3 * i + 1] = (i * 17) as u8;
        plte[3 * i + 2] = (i * 17) as u8;
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    put_chunk(&mut out, "IHDR", &ihdr);
    put_chunk(&mut out, "PLTE", &plte);
    put_chunk(&mut out, "IDAT", &z);
    put_chunk(&mut out, "IEND", &[]);
    out
}

// ---- requests ----

fn put_request(page: i32, pages: i32, kind: &str, png: Option<&[u8]>, extra: Option<&str>) {
    let text = format!(
        "# Grimoire: a page to annotate, from KOReader's Grimoire plugin (docs/FORMAT.md)\nversion=1\nbook={}\n\
         title={}\npage={}\npages={}\nkind={}\npicture=page.png\ntime={}\n{}",
        BOOK,
        TITLE,
        page,
        pages,
        kind,
        wall_time(),
        extra.unwrap_or("")
    );
    put_file(&request_path("request.ini"), text.as_bytes());
    if let Some(png) = png {
        put_file(&request_path("page.png"), png);
    }
}

// What the code under test logs (grimoire.log is its stderr), from log_capture to log_captured.
static G_LOG_SAVED: AtomicI32 = AtomicI32::new(-1);

fn g_log_path() -> String {
    format!("{}/captured.log", root())
}

fn log_capture() {
    // (stderr is not buffered here: the lines of earlier checks are out already)
    // SAFETY: dup, open and dup2 on descriptor 2 and one this function opens, which it closes again.
    unsafe {
        G_LOG_SAVED.store(libc::dup(2), Ordering::Relaxed);
        let path = std::ffi::CString::new(g_log_path()).unwrap();
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
}

fn log_captured() -> Option<String> {
    let saved = G_LOG_SAVED.swap(-1, Ordering::Relaxed);
    if saved >= 0 {
        // SAFETY: puts descriptor 2 back from the copy log_capture made, and closes the copy.
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
    let t = read_file(&g_log_path()).map(|t| String::from_utf8_lossy(&t).into_owned());
    if let Some(t) = &t {
        eprint!("{}", t);
    }
    t
}

/// Waits for the background jobs to finish (sanitized children on a busy machine: by the clock).
fn drain_jobs() {
    let until = now_ms().wrapping_add(60000);
    while bg_busy() && (now_ms().wrapping_sub(until) as i32) < 0 {
        bg_poll();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    bg_poll();
}

fn page_paper(nb: &NotebookInfo, index: i32, k: &mut PaperKind) -> bool {
    let mut p: Option<Box<Page>> = None;
    let ok = store_read_page(nb, index, &mut p) == 0;
    if ok {
        if let Some(p) = p.as_deref() {
            *k = p.paper;
        }
    }
    ok
}

// ---- reading a request ----

fn request_parsing() {
    let mut r = AnnotateRequest::default();
    let mut why: &'static str = "";
    let good = b"# a comment\r\nversion=1\r\nbook=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\r\ntitle=Le Petit Prince\r\n\
                 page=12\r\npages=96\r\nkind=paged\r\npicture=page.png\r\ntime=1792500000\r\nfrom=a newer plugin\r\n";
    check!(annotate_parse(good, &mut r, &mut why) == 0);
    check!(r.version == 1 && r.book == BOOK && r.title == "Le Petit Prince" && r.page == 12 && r.pages == 96);
    check!(r.kind == "paged" && r.picture == "page.png" && r.time == 1792500000);
    // No version line (a hand-made request): taken as version 1. A version it does not know: refused.
    check!(annotate_parse(b"book=ab12\npage=1\npages=1\npicture=a.png\n", &mut r, &mut why) == 0 && r.version == 1 && r.time == 0);
    check!(annotate_parse(b"version=2\nbook=ab12\npage=1\npages=1\npicture=a.png\n", &mut r, &mut why) != 0 && why.contains("version"));
    // What every request needs: a book key that is one, a page, a count at least the page, a picture.
    check!(annotate_parse(b"page=1\npages=1\npicture=a.png\n", &mut r, &mut why) != 0 && why.contains("book"));
    check!(annotate_parse(b"book=ab/12\npage=1\npages=1\npicture=a.png\n", &mut r, &mut why) != 0);
    check!(annotate_parse(b"book=ab12\npages=1\npicture=a.png\n", &mut r, &mut why) != 0 && why.contains("page number"));
    check!(annotate_parse(b"book=ab12\npage=0\npages=1\npicture=a.png\n", &mut r, &mut why) != 0);
    check!(annotate_parse(b"book=ab12\npage=5\npages=4\npicture=a.png\n", &mut r, &mut why) != 0 && why.contains("count"));
    check!(annotate_parse(b"book=ab12\npage=5\npicture=a.png\n", &mut r, &mut why) != 0);
    check!(annotate_parse(b"book=ab12\npage=-3\npages=4\npicture=a.png\n", &mut r, &mut why) != 0);
    check!(annotate_parse(b"book=ab12\npage=1\npages=1\n", &mut r, &mut why) != 0 && why.contains("picture"));
    // The picture is a plain .png name in the requests' folder: nothing that reaches elsewhere.
    let names = ["../x.png", "sub/x.png", ".x.png", "x.jpg", "x.png.tmp", "x y.png", ".png"];
    for name in names {
        let t = format!("book=ab12\npage=1\npages=1\npicture={}\n", name);
        check!(annotate_parse(t.as_bytes(), &mut r, &mut why) != 0);
    }
    check!(annotate_parse(b"book=ab12\npage=1\npages=1\npicture=page-12_v2.png\n", &mut r, &mut why) == 0);
    // A kind that is no word is left out; a title too long for a request is its start.
    check!(annotate_parse(b"book=ab12\npage=1\npages=1\npicture=a.png\nkind=Paged!\n", &mut r, &mut why) == 0 && r.kind.is_empty());
    let mut big = String::from("book=ab12\npage=1\npages=9\npicture=a.png\ntitle=");
    for _ in 0..700 {
        big.push('x');
    }
    big.push_str("\npage=7\n");
    check!(annotate_parse(big.as_bytes(), &mut r, &mut why) == 0 && r.title.len() == ANNOTATE_TITLE_LEN - 1 && r.page == 7);
    // (The C also passes NULL, a request that is not there: Rust has no NULL slice, so the nothing
    // that is there is the empty text, which has no book either.)
    check!(annotate_parse(b"", &mut r, &mut why) != 0);
}

fn request_age() {
    let mut r = AnnotateRequest { time: 1000000, ..AnnotateRequest::default() };
    check!(!annotate_too_old(&r, 1000000 + ANNOTATE_STALE_S, 0));
    check!(annotate_too_old(&r, 1000000 + ANNOTATE_STALE_S + 1, 0));
    check!(!annotate_too_old(&r, 1000000 - ANNOTATE_STALE_S, 0));
    check!(annotate_too_old(&r, 1000000 - ANNOTATE_STALE_S - 1, 0)); // from the future: the clock went back
    r.time = 0; // no time: the file's own
    check!(!annotate_too_old(&r, 5000, 4990) && annotate_too_old(&r, 5000, 4000) && annotate_too_old(&r, 5000, 0));
}

// ---- source.ini and the order of a book's pages ----

fn pages_of(ids: &[u32]) -> NotebookInfo {
    NotebookInfo { page_ids: ids.to_vec(), ..NotebookInfo::default() }
}

fn map_round_trip() {
    let text = "# Grimoire: the book this notebook annotates\nbook=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\nkind=paged\n\
                page.40.96=12\npage.12.96=10\nlater=a newer version's line\npage.x.96=3\npage.9.4=2\n\
                page.5.96=99\n";
    let mut m = AnnotateMap::default();
    check!(annotate_map_parse(Some(text.as_bytes()), &mut m) == 0);
    check!(m.book == BOOK && m.kind == "paged" && m.e.len() == 3);
    check!(
        m.extra.as_str().contains("later=a newer version's line\n")
            && m.extra.as_str().contains("page.x.96=3\n")
            && m.extra.as_str().contains("page.9.4=2\n")
    );
    let nb = pages_of(&[10, 11, 12, 13]);
    check!(annotate_map_find(&m, &nb, 12, 96) == 10 && annotate_map_find(&m, &nb, 40, 96) == 12);
    check!(annotate_map_find(&m, &nb, 5, 96) == 0); // its page (99) is gone from the notebook
    check!(annotate_map_find(&m, &nb, 12, 300) == 0); // another layout's page 12
    let out = annotate_map_text(&m, &nb);
    check!(out.is_some());
    let out = out.unwrap_or_default();
    check!(!out.contains(&0));
    let out = String::from_utf8_lossy(&out).into_owned();
    // In book order, the deleted page's line gone, the lines it does not know kept.
    let (p12, p40) = (out.find("page.12.96=10\n"), out.find("page.40.96=12\n"));
    check!(p12.is_some() && p40.is_some() && p12 < p40 && !out.contains("=99") && out.contains("later=a newer version's line\n"));
    check!(out.contains("book=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n") && out.contains("kind=paged\n"));
    let mut again = AnnotateMap::default();
    check!(annotate_map_parse(Some(out.as_bytes()), &mut again) == 0 && again.e.len() == 2 && again.extra == m.extra);
    annotate_map_free(&mut again);
    check!(annotate_map_add(&mut m, 12, 96, 11) == 0 && m.e.len() == 3 && annotate_map_find(&m, &nb, 12, 96) == 11); // (moved)
    annotate_map_free(&mut m);
    check!(annotate_map_parse(None, &mut m) == 0 && m.e.is_empty() && m.book.is_empty());
    annotate_map_free(&mut m);
}

fn page_order() {
    // Page 12 of 96 is the notebook's page 10, with a page of notes (11) after it; page 40 is 12,
    // with notes (13) after it too.
    let nb = pages_of(&[10, 11, 12, 13]);
    let mut m = AnnotateMap::default();
    check!(annotate_map_add(&mut m, 12, 96, 10) == 0 && annotate_map_add(&mut m, 40, 96, 12) == 0);
    check!(annotate_insert_at(&nb, &m, 5, 96) == 0); // before page 12
    check!(annotate_insert_at(&nb, &m, 20, 96) == 2); // after page 12 and its notes, before page 40
    check!(annotate_insert_at(&nb, &m, 50, 96) == 4); // at the end, after page 40's notes
                                                      // A reflowable book read at two font sizes: by the place in the book (page - 1 of pages).
    annotate_map_free(&mut m);
    m = AnnotateMap::default();
    check!(annotate_map_add(&mut m, 12, 300, 10) == 0); // 11/300 into the book
    check!(annotate_insert_at(&nb, &m, 16, 400) == 4); // 15/400, further: after
    check!(annotate_insert_at(&nb, &m, 15, 400) == 0); // 14/400, not as far: before
    check!(annotate_insert_at(&nb, &m, 12, 400) == 0);
    // No book page in the notebook yet (or its pages deleted): at the end.
    annotate_map_free(&mut m);
    m = AnnotateMap::default();
    check!(annotate_insert_at(&nb, &m, 3, 96) == 4);
    check!(annotate_map_add(&mut m, 50, 96, 77) == 0 && annotate_insert_at(&nb, &m, 3, 96) == 4);
    annotate_map_free(&mut m);
}

fn names() {
    let mut out = String::new();
    annotate_name(b"  Le Petit\tPrince\n ", &mut out, STORE_NAME_LEN);
    check!(out == "Le Petit Prince");
    annotate_name(b"\x01Caf\xc3\xa9 \xff\xfecr\xe2\x82\xacme\xc2\x85", &mut out, STORE_NAME_LEN); // C0, broken, C1
    check!(out == "Café cr€me");
    annotate_name(b"\xed\xa0\x80x\xc0\xafy\xf4\x90\x80\x80z", &mut out, STORE_NAME_LEN); // surrogate, overlong, too big
    check!(out == "xyz");
    annotate_name(b"", &mut out, STORE_NAME_LEN);
    check!(out.is_empty());
    // (The C also passes NULL: Rust has no NULL slice, the nothing that is there is the empty one.)
    annotate_name(b"", &mut out, STORE_NAME_LEN);
    check!(out.is_empty());
    // Cut to the name limit between two characters: 120 e acutes are 240 bytes; 47 fit in 95.
    let title = "é".repeat(120);
    annotate_name(title.as_bytes(), &mut out, STORE_NAME_LEN);
    check!(out.len() == 94);
    let mut ends = String::new();
    annotate_name(b"ab\xe2\x82\xac", &mut ends, 5); // "ab" + a 3-byte character: it does not fit whole
    check!(ends == "ab");
    // Every letter stays, also what Grimoire's fonts cannot draw (the name is the PDF's too; only the
    // screens leave that out, ui_name_label): Greek, Cyrillic, Japanese, Vietnamese.
    annotate_name("Война αβ 戦争".as_bytes(), &mut out, STORE_NAME_LEN);
    check!(out == "Война αβ 戦争");
    annotate_name("Pokémon 漫画 Vol. 3".as_bytes(), &mut out, STORE_NAME_LEN);
    check!(out == "Pokémon 漫画 Vol. 3");
    annotate_name("吾輩は猫である".as_bytes(), &mut out, STORE_NAME_LEN);
    check!(out == "吾輩は猫である");
    annotate_name("Tiếng Việt".as_bytes(), &mut out, STORE_NAME_LEN); // (Tieng Viet, its marks whole)
    check!(out == "Tiếng Việt");
    annotate_name(b"Le   Petit \t Prince", &mut out, STORE_NAME_LEN); // (one space between two words)
    check!(out == "Le Petit Prince");
}

// notebook.ini's source= line: read, written back; a value of another form kept as it was.
fn notebook_source_line() {
    check!(store_source_ok(BOOK) && store_source_ok("a-b_C9") && !store_source_ok("") && !store_source_ok("a:b"));
    let mut long_key = "a".repeat(63);
    check!(store_source_ok(&long_key));
    long_key.push('a');
    check!(!store_source_ok(&long_key));
    let mut nb = NotebookInfo::default();
    check!(store_create("Sourced", "", PAPER_BLANK, &mut nb) == 0);
    let id = nb.id.as_str().to_string();
    let mut dir = String::new();
    store_notebook_dir(&id, &mut dir, 600);
    let path = format!("{}/notebook.ini", dir);
    let t = read_file(&path);
    check!(t.as_deref().is_some_and(|t| !String::from_utf8_lossy(t).contains("source="))); // none: no line
    nb.source.set(BOOK);
    check!(store_save_info(&mut nb) == 0);
    let t = read_file(&path);
    check!(t.as_deref().is_some_and(|t| String::from_utf8_lossy(t).contains("\nsource=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n")));
    check!(store_load_info(&id, &mut nb) == 0 && nb.source == BOOK);
    // A newer version's form of the line: not this version's key, kept and written back as it was.
    let mut mixed = read_file(&path).unwrap_or_default();
    mixed.extend_from_slice(b"source=pdf:42\n");
    put_file(&path, &mixed);
    check!(store_load_info(&id, &mut nb) == 0 && nb.source == BOOK && nb.extra.as_str().contains("source=pdf:42\n"));
    check!(store_save_info(&mut nb) == 0);
    let t = read_file(&path);
    check!(t.as_deref().is_some_and(|t| {
        let t = String::from_utf8_lossy(t);
        t.contains("source=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n") && t.contains("source=pdf:42\n")
    }));
    check!(store_delete(&id) == 0);
}

// ---- page pictures ----

fn png_checks() {
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = "";
    let png = text_png(PW, PH, 7);
    let len = png.len();
    check!(!png.is_empty() && annotate_png_check(&png, &mut w, &mut h, &mut why) == 0 && w == PW && h == PH);
    check!(annotate_png_check(&png[..len * 3 / 5], &mut w, &mut h, &mut why) != 0 && why == "cut short");
    check!(annotate_png_check(&png[..len - 12], &mut w, &mut h, &mut why) != 0); // no IEND
    check!(annotate_png_check(&png[..20], &mut w, &mut h, &mut why) != 0);
    let mut bad = png.clone();
    bad[len / 2] ^= 0x10; // a bit flipped in its data: the CRC catches what the header cannot
    check!(annotate_png_check(&bad, &mut w, &mut h, &mut why) != 0 && why.contains("checksum"));
    check!(bgimage_check(&bad, len, &mut w, &mut h, &mut why) == 0);
    check!(annotate_png_check(b"not a picture at all\n", &mut w, &mut h, &mut why) != 0 && why == "not a PNG");
    // An ancillary chunk with a wrong CRC (decoders skip those) does not count.
    let mut anc = png[..33].to_vec();
    put_chunk(&mut anc, "tEXt", b"Grimoire");
    let k = anc.len();
    anc[k - 1] ^= 0xFF;
    anc.extend_from_slice(&png[33..]);
    check!(annotate_png_check(&anc, &mut w, &mut h, &mut why) == 0);
    // Bytes after its end (an older, longer file written over without being cut, say): refused.
    let mut tail = png.clone();
    tail.extend_from_slice(b"garbage");
    check!(annotate_png_check(&tail, &mut w, &mut h, &mut why) != 0 && why.contains("after its end"));
    check!(annotate_png_check(&tail[..len], &mut w, &mut h, &mut why) == 0);
    // A palette PNG, as KOReader's encoder writes a page of few grays.
    let pal = palette_png(PW, PH);
    check!(annotate_png_check(&pal, &mut w, &mut h, &mut why) == 0 && w == PW && h == PH);
    check!(bgimage_drawable(&pal, &mut w, &mut h, &mut why) == 0);
}

// ---- a start with a request ----

// What no start may log (grimoire.log goes into public bug reports): the requests' titles, book keys
// and picture names, as the tests below send them.
const K_PRIVATE: [&str; 10] =
    ["Petit", "illustr", "0f3c6e2a9b1d", "ffff0000", "Read Sideways", "abcd1234", "Full Book", "beef4242", "Tolstoy", "吾輩"];

// A start with whatever request is there: what it logged is in G_LOG (for the test to read further),
// held to the rule above.
thread_local! {
    static G_LOG: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn g_log() -> String {
    G_LOG.with(|l| l.borrow().clone().unwrap_or_default())
}

fn take(asked: bool) -> AnnotateResult {
    let mut r = AnnotateResult::default();
    log_capture();
    annotate_take(asked, &mut r);
    let log = log_captured();
    if let Some(log) = &log {
        for p in K_PRIVATE {
            if log.contains(p) {
                eprintln!("  a start logged \"{}\"", p);
            }
            check!(!log.contains(p));
        }
    }
    G_LOG.with(|l| *l.borrow_mut() = log);
    r
}

fn load(id: &str, nb: &mut NotebookInfo) -> bool {
    store_load_info(id, nb) == 0
}

fn starts_with_requests() {
    let dir = format!("{}/{}", root(), ANNOTATE_DIR);
    // No request: nothing, unless one was asked for.
    check!(take(false).outcome == AnnotateOutcome::None);
    let mut r = take(true);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Log);

    // Page 12 of a 96-page PDF: a notebook made for the book, its one page on the picture.
    let png12 = text_png(PW, PH, 12);
    let len12 = png12.len();
    put_request(12, 96, "paged", Some(&png12), None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && r.pid == 1 && r.page == 12 && r.pages == 96);
    check!(g_log().contains("annotate: notebook ")); // (what take checks the log of, it logged)
    check!(!exists(&dir)); // the request and its folder gone
    let mut nb = NotebookInfo::default();
    let id = r.nb.as_str().to_string();
    check!(load(&id, &mut nb) && nb.source == BOOK && nb.name == TITLE && nb.npages() == 1);
    check!(nb.page_w == PW && nb.page_h == PH && nb.last_page == 0);
    // Its page a document page on the picture; its own paper Blank, as an imported notebook's (what a
    // page added to it gets, and what a page that does not read shows: never a book page's picture).
    let mut k = PAPER_BLANK;
    check!(page_paper(&nb, 0, &mut k) && paper_is_document(k) && nb.paper == PAPER_BLANK);
    let (mut bgdir, mut file) = (String::new(), String::new());
    check!(bgimage_dir(&id, &mut bgdir, 700) == 0);
    bgimage_file(&bgdir, paper_image_id(k), &mut file, 900);
    let kept = read_file(&file);
    check!(kept.as_deref() == Some(&png12[..])); // KOReader's picture, as it is
    let mut nbdir = String::new();
    store_notebook_dir(&id, &mut nbdir, 600);
    let mut path = format!("{}/source.ini", nbdir);
    let map = read_file(&path).map(|m| String::from_utf8_lossy(&m).into_owned());
    check!(map.as_deref().is_some_and(|m| {
        m.contains("book=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\n") && m.contains("kind=paged\n") && m.contains("page.12.96=1\n")
    }));
    path = format!("{}/bg/pages.ini", nbdir);
    check!(exists(&path));
    let mut probe = String::new();
    check!(annotate_probe("result", 0, &mut probe, 900) == 0 && probe == "made");
    check!(annotate_probe("book_page", 0, &mut probe, 900) == 0 && probe == "12/96");

    // The same page again: opened as it is, nothing added.
    put_request(12, 96, "paged", Some(&png12), None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Opened && r.nb == id.as_str() && r.pid == 1);
    check!(load(&id, &mut nb) && nb.npages() == 1 && files_in(&bgdir) == 3); // the picture, index.ini, pages.ini

    // Page 5: before page 12. The card's thumbnail of the page that was first goes (the editor makes
    // the new one), and the notebook's paper stays Blank.
    path = format!("{}/thumb.pgm", nbdir);
    put_file(&path, b"P5\n1 1\n255\n\x80");
    let png5 = text_png(PW, PH, 5);
    put_request(5, 96, "paged", Some(&png5), None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && r.nb == id.as_str());
    check!(load(&id, &mut nb) && nb.npages() == 2 && nb.page_ids[0] == r.pid && nb.page_ids[1] == 1 && nb.last_page == 0);
    let mut k5 = PAPER_BLANK;
    check!(page_paper(&nb, 0, &mut k5) && paper_is_document(k5) && k5 != k && nb.paper == PAPER_BLANK && !exists(&path));
    let pid5 = r.pid;
    // Page 40: at the end, after a page of notes the owner added after page 12.
    check!(store_add_page(&mut nb, 2, k) == 0);
    let mut knote = PAPER_MEDIUM;
    check!(page_paper(&nb, 2, &mut knote) && knote == PAPER_BLANK); // (no document page's picture is copied)
    let png40 = text_png(PW, PH, 40);
    put_request(40, 96, "paged", Some(&png40), None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && load(&id, &mut nb) && nb.npages() == 4 && nb.page_ids[3] == r.pid && nb.last_page == 3);
    check!(nb.paper == PAPER_BLANK && nb.page_ids[0] == pid5); // (the first page stays page 5)
                                                               // Page 20: after page 12 and its notes, before page 40.
    let png20 = text_png(PW, PH, 20);
    put_request(20, 96, "paged", Some(&png20), None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && load(&id, &mut nb) && nb.npages() == 5 && nb.page_ids[3] == r.pid);
    check!(nb.page_ids[0] == pid5 && nb.page_ids[1] == 1);
    check!(annotate_probe("book_page", 3, &mut probe, 900) == 0 && probe == "20/96");
    check!(annotate_probe("book_page", 2, &mut probe, 900) == 0 && probe.is_empty()); // the notes

    // Not asked to open it (Grimoire started from the Library, or by Open Grimoire): deleted unopened.
    put_request(60, 96, "paged", Some(&png12), None);
    r = take(false);
    check!(r.outcome == AnnotateOutcome::Stale && !exists(&dir) && load(&id, &mut nb) && nb.npages() == 5);
    let lines = g_log().matches("annotate:").count();
    check!(lines == 1); // one log line
                        // Too old (or ahead of the clock), however asked.
    put_request(60, 96, "paged", Some(&png12), Some("time=978307200\n"));
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Stale && !exists(&dir) && load(&id, &mut nb) && nb.npages() == 5);
    // A picture cut short, or no picture at all: nothing made, the folder emptied.
    put_request(60, 96, "paged", Some(&png12[..len12 * 3 / 5]), None);
    r = take(true);
    check!(
        r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Picture && !exists(&dir) && load(&id, &mut nb) && nb.npages() == 5
    );
    put_request(60, 96, "paged", None, None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Picture && !exists(&dir));
    // A picture that is a link to a file elsewhere (the plugin writes a file of its own): not taken.
    put_request(60, 96, "paged", None, None);
    file = format!("{}/elsewhere.png", root());
    put_file(&file, &png12);
    path = request_path("page.png");
    check!(std::os::unix::fs::symlink(&file, &path).is_ok());
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Picture && !exists(&dir) && exists(&file));
    let _ = std::fs::remove_file(&file);
    // A request that does not read.
    path = request_path("request.ini");
    put_file(&path, b"book=\n");
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Log && !exists(&dir));

    // An older version saved the notebook and dropped its source= line: source.ini still names the
    // book, so its next page goes there, and the line comes back.
    nb.source.clear();
    check!(store_save_info(&mut nb) == 0 && load(&id, &mut nb) && nb.source.is_empty());
    let png70 = text_png(PW, PH, 70);
    put_request(70, 96, "paged", Some(&png70), None);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && r.nb == id.as_str() && load(&id, &mut nb) && nb.source == BOOK && nb.npages() == 6);

    // Another book (its picture under another name): a notebook of its own.
    let text =
        format!("version=1\nbook=ffff0000\ntitle={}\npage=1\npages=1\nkind=paged\npicture=Tolstoy-1.png\ntime={}\n", TITLE, wall_time());
    path = request_path("request.ini");
    put_file(&path, text.as_bytes());
    path = request_path("Tolstoy-1.png");
    put_file(&path, &png12);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && r.nb != id.as_str());
    let mut other = NotebookInfo::default();
    check!(load(r.nb.as_str(), &mut other) && other.source == "ffff0000");
    check!(other.name == "Le Petit Prince : édition illustrée 2"); // (its PDF's name apart)
                                                                   // Books whose titles Grimoire's fonts draw none of (Japanese): named after their titles all the
                                                                   // same, the second with " 2", as any two notebooks of one name would share a PDF; that PDF has
                                                                   // the title as its name. The screens show them as Untitled and Untitled 2 (ui_name_label).
    let ja = "吾輩は猫である";
    for i in 0..2 {
        let text = format!(
            "version=1\nbook=abcd1234{}\ntitle=吾輩は猫である\npage=1\npages=1\nkind=paged\npicture=page.png\ntime={}\n",
            i,
            wall_time()
        );
        path = request_path("request.ini");
        put_file(&path, text.as_bytes());
        path = request_path("page.png");
        put_file(&path, &png12);
        r = take(true);
        let mut want = format!("{}{}", ja, if i != 0 { " 2" } else { "" });
        check!(r.outcome == AnnotateOutcome::Made && load(r.nb.as_str(), &mut other) && other.name == want.as_str());
        let mut pdf = String::new();
        export_mirror_path(&other, "PDFs", &mut pdf, 700);
        want = format!("PDFs/{}{}.pdf", ja, if i != 0 { " 2" } else { "" });
        check!(pdf == want);
        let mut label = String::new();
        check!(ui_name_label(other.name.as_str(), &mut label, 64) == if i != 0 { "Untitled 2" } else { "Untitled" });
    }

    // A page whose file is gone (or does not read) shows the notebook's paper, Blank: never another
    // book page's picture, which an edit would then save into it.
    let mut at12: i32 = -1;
    for i in 0..nb.npages() {
        if nb.page_ids[i as usize] == 1 {
            at12 = i;
        }
    }
    path = format!("{}/p1.qpg", nbdir);
    check!(at12 >= 0 && std::fs::remove_file(&path).is_ok());
    let mut gone = PAPER_MEDIUM;
    check!(page_paper(&nb, at12, &mut gone) && gone == PAPER_BLANK);
}

fn pictures_of_another_size() {
    // A notebook from a Scribe of the other size, or a page sent sideways: fitted in the background.
    let png = text_png(PH, PW, 3); // sideways
    let text = format!(
        "version=1\nbook=abcd1234\ntitle=A Book Read Sideways\npage=3\npages=10\nkind=reflowable\npicture=page.png\ntime={}\n",
        wall_time()
    );
    let mut path = request_path("request.ini");
    put_file(&path, text.as_bytes());
    path = request_path("page.png");
    put_file(&path, &png);
    let mut r = take(true);
    check!(r.outcome == AnnotateOutcome::Made);
    let mut nb = NotebookInfo::default();
    let mut k = PAPER_BLANK;
    check!(load(r.nb.as_str(), &mut nb) && page_paper(&nb, 0, &mut k) && paper_is_document(k));
    let (mut bgdir, mut file) = (String::new(), String::new());
    bgimage_dir(r.nb.as_str(), &mut bgdir, 700);
    bgimage_file(&bgdir, paper_image_id(k), &mut file, 900);
    let source = format!("{}/{:06x}.source.png", bgdir, paper_image_id(k));
    check!(!exists(&file) || bg_busy()); // made in the background
    drain_jobs();
    check!(exists(&file) && !exists(&source));
    let made = read_file(&file);
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = "";
    check!(made.as_deref().is_some_and(|m| bgimage_drawable(m, &mut w, &mut h, &mut why) == 0) && w == PW && h == PH); // the page's size
                                                                                                                       // Its picture gone missing (its job never ran): the same page sent again makes it again. Drawn
                                                                                                                       // meanwhile, the page is white; once the picture is there it is "undrawn" (for the editor: draw
                                                                                                                       // that page again), and once drawn, no longer.
    let id = r.nb.as_str().to_string();
    let pic = paper_image_id(k);
    let _ = std::fs::remove_file(&file);
    bgimage_forget(None);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, PW, PH) == 0);
    bgimage_render(&mut c, &id, k, Rect { x: 0, y: 0, w: PW, h: PH });
    check!(c.px[(PH / 2) as usize * c.stride as usize + (PW / 2) as usize] == 255 && !bgimage_undrawn(&id, pic)); // (no file yet)
                                                                                                                  // The page left before its picture is made: the editor wrote the Library's card from the page
                                                                                                                  // drawn white. The job that makes the picture draws the card again, the page on its picture.
    let mut white = Canvas::default();
    check!(canvas_init(&mut white, PW / 3, PH / 3) == 0 && store_save_thumb(&id, &white) == 0);
    path = request_path("request.ini");
    put_file(&path, text.as_bytes());
    path = request_path("page.png");
    put_file(&path, &png);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Opened);
    drain_jobs();
    check!(exists(&file) && !exists(&source));
    bgimage_retry_missing(); // (as poll_jobs does once a picture job is done)
    let (mut card, mut want) = (Canvas::default(), Canvas::default());
    let mut first: Option<Box<Page>> = None;
    check!(store_load_thumb(&id, &mut card) == 0 && store_read_page(&nb, 0, &mut first) == 0 && first.is_some());
    check!(canvas_init(&mut want, PW / 3, PH / 3) == 0);
    if let Some(first) = first.as_deref() {
        page_render_thumb(&mut want, first, DPI);
    }
    let n = (want.w * want.h) as usize;
    check!(card.w == want.w && card.h == want.h && card.px[..n] == want.px[..n]);
    let nw = (white.w * white.h) as usize;
    check!(!card.px.is_empty() && card.px[..nw] != white.px[..nw]);
    canvas_free(&mut card);
    canvas_free(&mut want);
    canvas_free(&mut white);
    bgimage_forget(None);
    check!(bgimage_undrawn(&id, pic) && !bgimage_undrawn(&id, (pic + 1) & 0xFFFFFF));
    bgimage_render(&mut c, &id, k, Rect { x: 0, y: 0, w: PW, h: PH });
    check!(!bgimage_undrawn(&id, pic));
    canvas_free(&mut c);
}

// A notebook's raw zip carries its source.ini, so one put back from the zip keeps which of its pages
// is which book page: the same page sent again opens it, rather than adding a second one.
fn raw_zip_keeps_the_map() {
    let png = text_png(PW, PH, 7);
    let text =
        format!("version=1\nbook=feedc0de\ntitle=Zipped Book\npage=7\npages=20\nkind=paged\npicture=page.png\ntime={}\n", wall_time());
    let mut path = request_path("request.ini");
    put_file(&path, text.as_bytes());
    path = request_path("page.png");
    put_file(&path, &png);
    let r = take(true);
    check!(r.outcome == AnnotateOutcome::Made);
    let mut nb = NotebookInfo::default();
    check!(load(r.nb.as_str(), &mut nb));
    let mut b = Buf::default();
    {
        let mut z = zip_begin(&mut b);
        check!(export_notebook_zip(&nb, DPI, &mut z, "") == 0 && zip_end(&mut z) == 0);
    }
    let zp = format!("{}/zipped.zip", root());
    put_file(&zp, &b.data);
    buf_free(&mut b);
    let mut nbdir = String::new();
    store_notebook_dir(r.nb.as_str(), &mut nbdir, 600);
    let cmd = format!("unzip -p '{}' 'Zipped Book/raw/source.ini' | cmp -s - '{}/source.ini'", zp, nbdir);
    check!(std::process::Command::new("sh").arg("-c").arg(&cmd).status().is_ok_and(|s| s.success()));
    let _ = std::fs::remove_file(&zp);
}

fn failures_leave_nothing() {
    let png = text_png(PW, PH, 9);
    let mut text =
        format!("version=1\nbook=beef4242\ntitle=Full Book\npage=1\npages=9\nkind=paged\npicture=page.png\ntime={}\n", wall_time());
    let mut path = request_path("request.ini");
    put_file(&path, text.as_bytes());
    path = request_path("page.png");
    put_file(&path, &png);
    let mut r = take(true);
    check!(r.outcome == AnnotateOutcome::Made);
    let id = r.nb.as_str().to_string();
    let mut nbdir = String::new();
    store_notebook_dir(&id, &mut nbdir, 600);
    let mut nb = NotebookInfo::default();
    // The notebook's folder cannot be written (a full storage, as far as this can tell): the page
    // is not added, and the picture put in bg/ for it is taken back.
    let bgdir = format!("{}/bg", nbdir);
    let before = files_in(&bgdir);
    let png2 = text_png(PW, PH, 10);
    text = format!("version=1\nbook=beef4242\ntitle=Full Book\npage=2\npages=9\nkind=paged\npicture=page.png\ntime={}\n", wall_time());
    path = request_path("request.ini");
    put_file(&path, text.as_bytes());
    path = request_path("page.png");
    put_file(&path, &png2);
    let _ = std::fs::set_permissions(&nbdir, std::fs::Permissions::from_mode(0o555));
    r = take(true);
    let _ = std::fs::set_permissions(&nbdir, std::fs::Permissions::from_mode(0o755));
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Storage);
    check!(load(&id, &mut nb) && nb.npages() == 1 && files_in(&bgdir) == before);
    // A source.ini that does not read: left as it is, nothing added over it.
    path = format!("{}/source.ini", nbdir);
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000));
    text = format!("version=1\nbook=beef4242\ntitle=Full Book\npage=2\npages=9\nkind=paged\npicture=page.png\ntime={}\n", wall_time()); // (the same request again)
    let mut req = request_path("request.ini");
    put_file(&req, text.as_bytes());
    req = request_path("page.png");
    put_file(&req, &png2);
    r = take(true);
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    check!(r.outcome == AnnotateOutcome::Failed && load(&id, &mut nb) && nb.npages() == 1);

    // A page is listed last, its id taken first. notebook.ini cannot be written: nothing is (not its
    // file, nor its line in bg/pages.ini or in source.ini).
    let nbini = format!("{}/notebook.ini.tmp", nbdir);
    path = format!("{}/source.ini", nbdir);
    let side = format!("{}/pages.ini", bgdir);
    let map0 = read_file(&path);
    let side0 = read_file(&side);
    let next0 = nb.next_page_id;
    check!(map0.is_some() && side0.is_some() && std::fs::create_dir(&nbini).is_ok()); // (a folder where its new copy goes)
    req = request_path("request.ini");
    put_file(&req, text.as_bytes());
    req = request_path("page.png");
    put_file(&req, &png2);
    r = take(true);
    let _ = std::fs::remove_dir(&nbini);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Storage);
    check!(load(&id, &mut nb) && nb.npages() == 1 && nb.next_page_id == next0 && files_in(&bgdir) == before);
    let mut spare = format!("{}/p{}.qpg", nbdir, next0);
    let mut t = read_file(&path);
    check!(!exists(&spare) && t.is_some() && t == map0);
    t = read_file(&side);
    check!(t.is_some() && t == side0);
    // source.ini cannot be written, after the page's file was: taken back, its id given up (never
    // another page's), the picture too.
    spare = format!("{}/source.ini.tmp", nbdir);
    check!(std::fs::create_dir(&spare).is_ok());
    req = request_path("request.ini");
    put_file(&req, text.as_bytes());
    req = request_path("page.png");
    put_file(&req, &png2);
    r = take(true);
    let _ = std::fs::remove_dir(&spare);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Storage);
    check!(load(&id, &mut nb) && nb.npages() == 1 && nb.next_page_id == next0 + 1 && files_in(&bgdir) == before);
    spare = format!("{}/p{}.qpg", nbdir, next0);
    check!(!exists(&spare));
    // Sent again, the page goes in with the next id.
    req = request_path("request.ini");
    put_file(&req, text.as_bytes());
    req = request_path("page.png");
    put_file(&req, &png2);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && r.pid == next0 + 1 && load(&id, &mut nb) && nb.npages() == 2 && nb.page_ids[1] == r.pid);
    // A start cut off just before notebook.ini listed the page left its file and its lines in
    // bg/pages.ini and source.ini, its id taken: sent again, the page is a new one (the next id),
    // and the one left behind never becomes a page.
    let left = r.pid;
    nb.page_ids.truncate(1);
    check!(store_write_info_in(&nbdir, &nb) == 0 && load(&id, &mut nb) && nb.next_page_id == left + 1);
    spare = format!("{}/p{}.qpg", nbdir, left);
    check!(exists(&spare));
    req = request_path("request.ini");
    put_file(&req, text.as_bytes());
    req = request_path("page.png");
    put_file(&req, &png2);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Made && r.pid == left + 1 && load(&id, &mut nb) && nb.npages() == 2 && nb.page_ids[1] == r.pid);
    let t = read_file(&path).map(|t| String::from_utf8_lossy(&t).into_owned());
    let want = format!("page.2.9={}\n", r.pid);
    let gone = format!("={}\n", left);
    check!(t.as_deref().is_some_and(|t| t.contains(&want) && !t.contains(&gone)));

    // A notebook as full as one gets (2000 pages): nothing added.
    nb.page_ids = (1..=STORE_MAX_PAGES as u32).collect();
    check!(store_save_info(&mut nb) == 0);
    text = format!("version=1\nbook=beef4242\ntitle=Full Book\npage=3\npages=9\nkind=paged\npicture=page.png\ntime={}\n", wall_time());
    req = request_path("request.ini");
    put_file(&req, text.as_bytes());
    req = request_path("page.png");
    put_file(&req, &png2);
    r = take(true);
    check!(r.outcome == AnnotateOutcome::Failed && r.why == AnnotateWhy::Log);
    // What a start cut off while making a notebook left: removed at the next start.
    let mut work = format!("{}/notebooks/.annotate-20261020-000000-dead/bg", root());
    check!(mkdir_p(&work) == 0);
    path = format!("{}/000001.png", work);
    put_file(&path, &png[..10]);
    check!(take(false).outcome == AnnotateOutcome::None);
    work = format!("{}/notebooks/.annotate-20261020-000000-dead", root());
    check!(!exists(&work));
}

// A whole book's request: its pictures named page-<n>.png and nothing else, each page once, none past
// the last; sorted as they come.
fn book_request_parsing() {
    let mut r = BookRequest::default();
    let mut why: &'static str = "";
    let good = b"# a comment\r\nversion=1\r\nbook=0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70\r\ntitle=Hanon\r\n\
                 pages=96\r\npage=13\r\nkind=paged\r\ntime=1792500000\r\npicture.13=page-13.png\r\n\
                 picture.2=page-2.png\r\npicture.96=page-96.png\r\nfrom=a newer plugin\r\n";
    check!(book_parse(good, &mut r, &mut why) == 0);
    check!(r.version == 1 && r.book == BOOK && r.title == "Hanon" && r.page == 13 && r.pages == 96);
    check!(r.kind == "paged" && r.time == 1792500000 && r.sent.len() == 3);
    check!(r.sent.len() == 3 && r.sent[0] == 2 && r.sent[1] == 13 && r.sent[2] == 96);
    book_request_free(&mut r);
    // No pictures at all: every page there already, only the page to open.
    check!(book_parse(b"book=ab12\npages=3\npage=2\n", &mut r, &mut why) == 0 && r.sent.is_empty());
    book_request_free(&mut r);
    let bad: [&str; 10] = [
        "version=2\nbook=ab12\npages=3\npage=1\n",            // a version it does not know
        "pages=3\npage=1\n",                                  // no book
        "book=ab12\npage=1\n",                                // no page count
        "book=ab12\npages=3\npage=4\n",                       // the page to open past the last
        "book=ab12\npages=3\npage=1\npicture.4=page-4.png\n", // a picture past the last page
        "book=ab12\npages=3\npage=1\npicture.2=page-2.png\npicture.2=page-2.png\n", // twice
        "book=ab12\npages=3\npage=1\npicture.2=page-3.png\n", // the wrong name
        "book=ab12\npages=3\npage=1\npicture.2=../page-2.png\n", // reaching elsewhere
        "book=ab12\npages=3\npage=1\npicture.0=page-0.png\n", // no page 0
        "book=ab12\npages=3\npage=1\npicture.x=page-x.png\n", // no number
    ];
    for b in bad {
        let rc = book_parse(b.as_bytes(), &mut r, &mut why);
        if rc == 0 {
            eprintln!("  taken: {}", b);
        }
        check!(rc != 0 && !why.is_empty());
        book_request_free(&mut r);
    }
    // (The C also passes NULL: the nothing that is there is the empty text, which has no book.)
    check!(book_parse(b"", &mut r, &mut why) != 0);
}

// The notices a start with a request can end with (app.c), in every language and style, on the
// narrow screen: a toast has two lines. Keep this list in step with the messages.
fn notices_fit() {
    let screen = Rect { x: 0, y: 0, w: 1860, h: 2480 };
    for l in 0..i18n_language_count() {
        i18n_set(i18n_language_code(l).unwrap_or("en"));
        let log = cformat!(tr("Could not open the page KOReader sent (see %s)."), "grimoire.log");
        let notes = [
            tr("Could not open the page KOReader sent: its picture does not read."),
            tr("Could not add the page KOReader sent. Is the storage full?"),
            log.as_str(),
        ];
        for th in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(th);
            for note in notes {
                let lines = font_wrap_lines(FontFace::Bold, ui_text_size(), note, ui_toast_room(screen), None);
                if !(1..=2).contains(&lines) {
                    eprintln!("  {}: \"{}\" needs {} lines", i18n_current(), note, lines);
                }
                check!((1..=2).contains(&lines));
                check!(note.len() < 256); // whole in a toast's buffer
            }
        }
        // Translated (every language but English has its words for them).
        if i18n_language_code(l) != Some("en") {
            check!(notes[0] != "Could not open the page KOReader sent: its picture does not read.");
        }
    }
    ui_set_theme(UiTheme::Gothic);
    i18n_set("en");
}

// ---- a whole book (not in the C unit test: tests/scenarios/z_book_*.qs cover it there) ----

// What no start may log, for the book requests below.
const K_PRIVATE_BOOK: [&str; 6] = ["Hanon", "Virtuoso", "1a2b3c4d5e6f", "Petit", "0f3c6e2a9b1d", "illustr"];

/// A scenario fixture's request and pictures (tests/fixtures/book/<name>) into import/.book.
fn copy_fixture(name: &str) {
    let src = format!("{}/../tests/fixtures/book/{}", env!("CARGO_MANIFEST_DIR"), name);
    let dst = format!("{}/{}", root(), BOOK_DIR);
    mkdir_p(&dst);
    // (written anew: a request without a time is as old as its file, and a copy may keep the
    // original's time)
    for e in std::fs::read_dir(&src).unwrap().flatten() {
        let d = std::fs::read(e.path()).unwrap_or_default();
        put_file(&format!("{}/{}", dst, e.file_name().to_string_lossy()), &d);
    }
}

/// book_start, then book_step until it is DONE or FAILED, as app_tick calls them; the pictures to
/// fit in the background are queued one at a time (book_fits_tick) and waited for. Its log is held
/// to the rule above.
fn open_book(asked: bool) -> BookStatus {
    let mut st = BookStatus::default();
    log_capture();
    book_start(asked, &mut st);
    for _ in 0..10000 {
        if st.state != BookState::Running {
            break;
        }
        book_step(150, &mut st);
    }
    let log = log_captured();
    if let Some(log) = &log {
        for p in K_PRIVATE_BOOK {
            if log.contains(p) {
                eprintln!("  a book logged \"{}\"", p);
            }
            check!(!log.contains(p));
        }
    }
    G_LOG.with(|l| *l.borrow_mut() = log);
    for _ in 0..4 {
        book_fits_tick();
        drain_jobs();
    }
    st
}

fn probe(key: &str, page: i32) -> String {
    let mut out = String::new();
    check!(annotate_probe(key, page, &mut out, 64) == 0);
    out
}

// z_book_1_new.qs: no notebook is the book's yet: one is made, named after its title, with the three
// pages in book order, each a document page on its picture (the third, drawn sideways, fitted to its
// page in the background). It opens on page 2, and the request and its pictures are gone.
fn book_new() {
    let dir = format!("{}/{}", root(), BOOK_DIR);
    copy_fixture("three");
    let st = open_book(true);
    check!(st.state == BookState::Done && st.made && st.added == 3 && st.total == 3);
    check!(probe("book", 0) == "done" && probe("book_added", 0) == "3" && probe("book_leftover", 0) == "0");
    check!(!exists(&dir));
    check!(g_log().contains("book: ") && !g_log().contains("deleted"));
    let mut nb = NotebookInfo::default();
    check!(load(st.nb.as_str(), &mut nb) && nb.name == "Hanon: The Virtuoso Pianist" && nb.npages() == 3);
    check!(nb.source == "1a2b3c4d5e6f708192a3b4c5d6e7f809" && nb.paper == PAPER_BLANK && nb.last_page == 1);
    check!(nb.page_w == 1860 && nb.page_h == 2480);
    check!(nb.page_ids[1] == st.pid); // the reader's page, 2
    for i in 0..3 {
        let mut k = PAPER_BLANK;
        check!(page_paper(&nb, i, &mut k) && paper_is_document(k));
        check!(probe("book_page", i) == format!("{}/3", i + 1));
        // The picture is there (page 3's fitted by now), and its source is gone.
        let (mut bgdir, mut file) = (String::new(), String::new());
        bgimage_dir(st.nb.as_str(), &mut bgdir, 700);
        bgimage_file(&bgdir, paper_image_id(k), &mut file, 900);
        check!(exists(&file) && !exists(&format!("{}/{:06x}.source.png", bgdir, paper_image_id(k))));
    }
    // No book to ask for twice: a start with nothing there is NONE (and FAILED when asked).
    check!(open_book(false).state == BookState::None);
    check!(open_book(true).state == BookState::Failed);
}

// z_book_2_merge.qs: the book's notebook already holds book pages 5 and 12; the request names 11, 12
// and 13: 12 stays as it is, 11 goes before it and 13 after it. It opens on page 13, the fourth.
fn book_merge() {
    let dir = format!("{}/{}", root(), BOOK_DIR);
    let (png5, png12) = (text_png(1860, 2480, 5), text_png(1860, 2480, 12));
    put_request(5, 96, "paged", Some(&png5), None);
    let r = take(true);
    check!(r.outcome == AnnotateOutcome::Made);
    put_request(12, 96, "paged", Some(&png12), None);
    let r2 = take(true);
    check!(r2.outcome == AnnotateOutcome::Made && r2.nb == r.nb);
    let mut nb = NotebookInfo::default();
    check!(load(r.nb.as_str(), &mut nb) && nb.npages() == 2);
    let pid12 = r2.pid;
    copy_fixture("merge");
    let st = open_book(true);
    check!(st.state == BookState::Done && !st.made && st.added == 2 && st.total == 3);
    check!(st.nb == r.nb.as_str() && probe("book_leftover", 0) == "0" && !exists(&dir));
    check!(load(r.nb.as_str(), &mut nb) && nb.npages() == 4 && nb.last_page == 3 && nb.page_ids[3] == st.pid);
    check!(nb.page_ids[2] == pid12); // page 12 stays where it was
    for (i, want) in ["5/96", "11/96", "12/96", "13/96"].iter().enumerate() {
        check!(probe("book_page", i as i32) == *want);
    }
    check!(nb.page_ids[2] == pid12 && probe("book_added", 0) == "2");
}

// z_book_3_broken.qs: the second page's picture was cut short: no notebook is made, and the request
// and its pictures are gone. z_book_4_not_asked.qs: a request nobody asked Grimoire to open is
// deleted unopened.
fn book_broken_and_not_asked() {
    let dir = format!("{}/{}", root(), BOOK_DIR);
    copy_fixture("broken");
    let st = open_book(true);
    check!(st.state == BookState::Failed && st.why == AnnotateWhy::Picture && !st.too_many);
    check!(probe("book", 0) == "failed" && probe("book_leftover", 0) == "0" && !exists(&dir));
    let mut all: Vec<NotebookInfo> = Vec::new();
    check!(store_list(&mut all) == 0 && all.is_empty());
    check!(g_log().contains("its picture is cut short"));
    let mut work = String::new();
    csnprintf!(&mut work, 700, "%s/notebooks", &root());
    check!(files_in(&work) == 0); // nothing half made either
    copy_fixture("merge");
    let st = open_book(false);
    check!(st.state == BookState::Stale && probe("book", 0) == "stale" && probe("book_leftover", 0) == "0" && !exists(&dir));
    // A request that does not read, and one for more pages than a notebook holds.
    put_file(&format!("{}/request.ini", dir), b"book=\n");
    let st = open_book(true);
    check!(st.state == BookState::Failed && st.why == AnnotateWhy::Log && !exists(&dir));
    put_file(&format!("{}/request.ini", dir), format!("book=ab12\npages=2001\npage=1\ntime={}\n", wall_time()).as_bytes());
    let st = open_book(true);
    check!(st.state == BookState::Failed && st.too_many && !exists(&dir));
}

// The tests that need no module of wave 2.
#[test]
fn test_annotate() {
    setup_ui();
    run!(request_age);
    run!(page_order);
    run!(names);
    run!(notices_fit);
    done();
}

#[test]
#[ignore = "store.rs (store_source_ok, store_init, store_create, ...) is a wave-2 module, still todo!() here"]
fn test_annotate_with_store() {
    setup_store("store");
    run!(request_parsing);
    run!(map_round_trip);
    run!(notebook_source_line);
    run!(book_request_parsing);
    done();
    // (the C removes its scratch root when every check passed)
    let _ = std::fs::remove_dir_all(root());
}

#[test]
#[ignore = "store.rs, bgimage.rs, bgjob.rs, page.rs and export.rs are wave-2 modules, still todo!() here"]
fn test_annotate_starts() {
    setup_store("starts");
    run!(png_checks);
    run!(starts_with_requests);
    run!(pictures_of_another_size);
    run!(raw_zip_keeps_the_map);
    run!(failures_leave_nothing);
    done();
    // (the C removes its scratch root when every check passed)
    let _ = std::fs::remove_dir_all(root());
}

#[test]
#[ignore = "store.rs, bgimage.rs, bgjob.rs, page.rs and export.rs are wave-2 modules, still todo!() here"]
fn test_annotate_book() {
    // (the scenarios' fixtures are pictures at the Scribe's page size)
    setup_store_device("book", 1860, 2480);
    run!(book_new);
    setup_store_device("book2", 1860, 2480);
    run!(book_merge);
    setup_store_device("book3", 1860, 2480);
    run!(book_broken_and_not_asked);
    done();
    let _ = std::fs::remove_dir_all(root());
}
